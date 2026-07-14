#!/usr/bin/env python3
"""Mechanize the Mac<->Mac pre-registered decision rule (PREREGISTRATION.md rev 4).

The harness must COMPUTE the verdict, not leave it to be applied by hand after the
numbers are visible -- that is what pre-registration exists to prevent.

WHAT ROUND 2 BROKE, AND WHAT REV 4 FIXES (codex r2 + grok, 15 findings)
----------------------------------------------------------------------
Every one of these let a real effect read as absent, or a dirty rig read as clean:

  * The equivalence margin was tied to the BAR alone. On a slow arm the bar is
    WIDER than the effect we are trying to exclude: srcinit=2500 with all eight
    d_i = 230 (a rig-W-sized effect in EVERY pair) gave ratio 1.092 (bar PASS),
    CI [230,230], margin 0.10*2500 = 250 -> "VANISHES". Both reviewers reproduced
    it. The margin is now min(BAR_BREACH, DELTA_REF) -- a null must exclude an
    effect the size of the one rig W actually measured, not merely one the bar
    would forgive.
  * The negative margin was wrong for a symmetric RATIO bar. The bar is symmetric
    in ratio, so the inverting boundary is -src/11 (-9.09%), NOT -0.10*src: a CI
    of [-190,0] on src=2000 was called VANISHES though -190 IS an inversion ratio
    of 1.105 -- over the bar.
  * The bootstrap CI was not 95% at n=8 (it resolved to ~[d2,d7], true coverage
    92.97%) and the 10k seeded resamples added no information. Replaced with the
    EXACT distribution-free order-statistic interval, and its true coverage is
    printed, not assumed.
  * The sign test was computed and never read, so 7/8 positive pairs could report
    REPRODUCES while the registered two-sided sign test said p = .0703.
  * RIG-VOID FAILED OPEN (grok, reproduced live): the code demanded a control both
    fail the bar AND land outside a set of outcomes, so a control with bar FAIL and
    a CI crossing zero (-> INCONCLUSIVE) ESCAPED the void -- grok drove a session
    that emitted VANISHES with its gRPC controls sitting at ratio 1.200, bar FAIL.
    A control that fails the bar now voids the rig, unconditionally.
  * A partial CELLS set was FILTERED rather than marked INCOMPLETE, so a one-cell
    run could emit VANISHES while claiming "both" cells vanished. The full
    REGISTERED set must be present and complete.
  * An exact 1.10 ratio could never REPRODUCE (grok): the bar is `<= 1.10 PASSES`
    (the project's acceptance semantics, kept), and REPRODUCES required a bar FAIL,
    so a precise 10% effect was unreportable by construction. Materiality is now
    "bar FAILS **or** the CI's near bound reaches the 10% threshold".

THE STATISTIC
-------------
  d_i  = destinit_i - srcinit_i     (positive = destination-initiated is slower)
  D    = median(d_i)                (LOW median for even n, applied everywhere)
  CI   = EXACT distribution-free order-statistic interval on the population median:
         the narrowest [d_(k), d_(n+1-k)] whose coverage 1 - 2*P(Bin(n,1/2) <= k-1)
         is >= 95%. At n=8 that is k=1 -> [min(d), max(d)], coverage 99.22%.
         n=8 admits NO exact 95% interval; the conservative side is chosen
         deliberately, and the true coverage is reported in every row.
  sign = exact two-sided binomial test on the count of positive d_i (zeros dropped).
         At n=8, p < .05 requires ALL EIGHT pairs to share a sign (k=8 -> p=.0078;
         k=7 -> p=.0703, NOT significant).

  BAR        : integer-exact, 10*hi <= 11*lo. `<= 1.10` PASSES (project semantics).
  BREACH_HI  = +src/10   (the effect that reaches ratio 1.10)
  BREACH_LO  = -src/11   (the effect that reaches INVERSE ratio 1.10 -- NOT -src/10)
  MARGIN_HI  = min(BREACH_HI, DELTA_REF)    <- the equivalence margin. DELTA_REF is
  MARGIN_LO  = max(BREACH_LO, -DELTA_REF)      an ABSOLUTE floor (rig W's measured
                                               230 ms): a null must exclude a
                                               rig-W-sized effect however slow this
                                               rig's arms are.

PER-CELL OUTCOMES (exhaustive; no unreportable region)
  pos_effect  = CI_lo > 0 and sign_p < .05         (a real destination-slower effect)
  neg_effect  = CI_hi < 0 and sign_p < .05         (a real source-slower effect)
  material    = bar FAILS or CI_lo >= BREACH_HI    (it reaches the 10% threshold)
  material_neg= bar FAILS or CI_hi <= BREACH_LO
  null_excl   = CI lies STRICTLY inside (MARGIN_LO, MARGIN_HI)

  REPRODUCES            pos_effect and material
  INVERSION             neg_effect and material_neg
  PARTIAL               a real effect (either sign) that is NOT material
  VANISHES              no effect AND null_excl -- a genuine EQUIVALENCE result
  UNDERPOWERED          no effect and the CI cannot exclude the margin -> a PASS
                        here is NOT "P1 vanishes"; the rig could not have seen it
  BAR-FAIL-INCONSISTENT bar FAILS but the pairs do not agree in sign. The medians
                        breach 1.10 while the paired evidence contradicts itself
                        (pf-0's bistability, in a new dress). NEVER a null.
  UNSTABLE              (override) an arm is bimodal AND the bar flips on pooled runs
  INCOMPLETE            the cell did not finish its registered pairs
"""
import csv, os, sys
from math import comb

runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]
DELTA_REF = int(os.environ.get("DELTA_REF_MS", "230"))


def cells_env(name):
    return [c for c in os.environ.get(name, "").split(",") if c]


VERDICT_CELLS = cells_env("VERDICT_CELLS")
CONTROL_CELLS = cells_env("CONTROL_CELLS")
# The full registered set must be PRESENT and COMPLETE. A partial CELLS set that is
# merely filtered lets a one-cell run emit VANISHES while claiming "both" cells
# vanished (codex r2 BLOCKER 1).
REGISTERED_CELLS = cells_env("REGISTERED_CELLS") or (VERDICT_CELLS + CONTROL_CELLS)

rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}

by, slots, void = {}, {}, {}
for r in rows:
    key = (r["cell"], r["arm"])
    if r["valid"] == "yes":
        by.setdefault(key, []).append(int(r["ms"]))
        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
    else:
        void[key] = void.get(key, 0) + 1


def med(v):
    """Low median for even n, stated once and applied consistently."""
    v = sorted(v)
    return v[(len(v) - 1) // 2]


def complete(c):
    if c not in meta or meta[c].get("complete") != "yes":
        return False
    arms = [a for (cc, a) in by if cc == c]
    if "srcinit" not in arms or "destinit" not in arms:
        return False
    return len(paired(c)) > 0


def paired(c):
    return [v["destinit"] - v["srcinit"]
            for (cc, _run), v in sorted(slots.items())
            if cc == c and "srcinit" in v and "destinit" in v]


def median_ci(d):
    """EXACT distribution-free CI on the population median.

    [d_(k), d_(n+1-k)] covers the median with probability
    1 - 2*P(Bin(n,1/2) <= k-1). Pick the LARGEST k (narrowest interval) whose
    coverage is still >= 95%. Returns (lo, hi, coverage). No bootstrap: at n=8 the
    bootstrap median CI resolves to ~[d2,d7] (92.97%) while claiming 95%.
    """
    d = sorted(d)
    n = len(d)
    if n == 0:
        return 0, 0, 0.0
    if n == 1:
        return d[0], d[0], 0.0
    best = None
    for k in range(1, n // 2 + 1):
        tail = sum(comb(n, i) for i in range(0, k)) / (2.0 ** n)
        cov = 1.0 - 2.0 * tail
        if cov >= 0.95:
            best = (d[k - 1], d[n - k], cov)      # larger k => narrower
    if best is None:                              # n too small for 95% at any k
        return d[0], d[-1], 1.0 - 2.0 / (2.0 ** n)
    return best


def sign_p(d):
    """Exact two-sided binomial test on the count of positive differences."""
    nz = [x for x in d if x != 0]
    n = len(nz)
    if n == 0:
        return 1.0, 0, 0
    k = sum(1 for x in nz if x > 0)
    tail = sum(comb(n, i) for i in range(0, min(k, n - k) + 1))
    return min(1.0, 2.0 * tail / (2 ** n)), k, n


def bar_of(hi, lo):
    """Integer-exact. `<= 1.10` PASSES -- the project's acceptance semantics."""
    return "PASS" if 10 * hi <= 11 * lo else "FAIL"


# ---- summary: every run printed (pf-0's bistability lesson) ------------------
with open(sum_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,runs\n")
    for (c, a) in sorted(by):
        v = by[(c, a)]
        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s\n" % (
            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
            void.get((c, a), 0), " ".join(str(x) for x in v)))

# ---- paired stats + per-cell outcome ----------------------------------------
cell_outcome, cell_detail = {}, {}
all_cells = sorted(set(REGISTERED_CELLS) | set(meta))
with open(pair_p, "w") as f:
    f.write("cell,n_pairs,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,ci_coverage,"
            "sign_p,k_pos_of_n,breach_hi_ms,breach_lo_ms,margin_hi_ms,margin_lo_ms,"
            "delta_ref_ms,null_excluded,unstable,outcome\n")
    for c in all_cells:
        if not complete(c):
            cell_outcome[c] = "INCOMPLETE"
            f.write("%s,0,,,,,,,,,,,,,,,%d,,,INCOMPLETE\n" % (c, DELTA_REF))
            continue
        d = paired(c)
        s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
        hi, lo = max(s_med, d_med), min(s_med, d_med)
        bar = bar_of(hi, lo)
        D = med(d)
        ci_lo, ci_hi, cov = median_ci(d)
        p, k, n = sign_p(d)

        # The bar is symmetric in RATIO, so the two boundaries are NOT symmetric in
        # ms: +src/10 reaches 1.10, but only -src/11 reaches the INVERSE 1.10.
        breach_hi = s_med / 10.0
        breach_lo = -s_med / 11.0
        # A null must exclude an effect the size of the one rig W measured (230 ms),
        # not merely one the bar would forgive -- on a slow arm the bar is WIDER.
        margin_hi = min(breach_hi, float(DELTA_REF))
        margin_lo = max(breach_lo, -float(DELTA_REF))

        pos_effect = ci_lo > 0 and p < 0.05
        neg_effect = ci_hi < 0 and p < 0.05
        material = (bar == "FAIL") or (ci_lo >= breach_hi)
        material_neg = (bar == "FAIL") or (ci_hi <= breach_lo)
        null_excl = (ci_lo > margin_lo) and (ci_hi < margin_hi)

        # UNSTABLE, as a STATISTIC not a vibe: an arm splits into two clusters
        # separated by more than the paired spread, AND the bar verdict flips when
        # graded on pooled runs instead of medians.
        unstable = "no"
        for arm in ("srcinit", "destinit"):
            v = sorted(by[(c, arm)])
            gaps = [(v[i + 1] - v[i], i) for i in range(len(v) - 1)]
            gmax = max(gaps)[0] if gaps else 0
            if gmax > (max(d) - min(d)) and gmax > 0:
                a_src = sum(by[(c, "srcinit")]) / float(len(by[(c, "srcinit")]))
                a_dst = sum(by[(c, "destinit")]) / float(len(by[(c, "destinit")]))
                if bar_of(max(a_src, a_dst), min(a_src, a_dst)) != bar:
                    unstable = "yes"

        if pos_effect and material:
            out = "REPRODUCES"
        elif neg_effect and material_neg:
            out = "INVERSION"
        elif pos_effect or neg_effect:
            out = "PARTIAL"
        elif bar == "FAIL":
            # The medians breach 1.10 but the pairs do not agree in sign. Never a
            # null, never a clean reproduction -- report it as its own thing.
            out = "BAR-FAIL-INCONSISTENT"
        elif null_excl:
            out = "VANISHES"
        else:
            out = "UNDERPOWERED"
        if unstable == "yes":
            out = "UNSTABLE"

        cell_outcome[c] = out
        cell_detail[c] = dict(D=D, ci=(ci_lo, ci_hi), cov=cov, p=p, k=k, n=n, bar=bar,
                              ratio=hi / lo if lo else 0.0,
                              breach=(breach_hi, breach_lo),
                              margin=(margin_hi, margin_lo), null_excl=null_excl)
        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%.4f,%d/%d,%d,%d,%d,%d,%d,%s,%s,%s\n" % (
            c, len(d), s_med, d_med, (hi / lo if lo else 0.0), bar, D, ci_lo, ci_hi, cov,
            p, k, n, round(breach_hi), round(breach_lo), round(margin_hi), round(margin_lo),
            DELTA_REF, "yes" if null_excl else "no", unstable, out))

# ---- per-cell invariance rows (unchanged shape) ------------------------------
with open(ver_p, "w") as f:
    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,delta_ms,bar,outcome\n")
    for c in all_cells:
        if not complete(c):
            f.write("%s,invariance,srcinit,destinit,,,,,1.10,INCOMPLETE\n" % c)
            continue
        s, dd = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
        hi, lo = max(s, dd), min(s, dd)
        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,1.10,%s\n" % (
            c, s, dd, hi / lo if lo else 0.0, dd - s, bar_of(hi, lo)))

# ---- SESSION VERDICT: strict precedence, exhaustive --------------------------
lines = []
# Every REGISTERED cell must be present and complete. Absent cells are INCOMPLETE,
# never filtered away (codex r2 BLOCKER 1).
missing = [c for c in REGISTERED_CELLS if c not in cell_outcome]
for c in missing:
    cell_outcome[c] = "INCOMPLETE"
incomplete = [c for c in REGISTERED_CELLS if cell_outcome.get(c) == "INCOMPLETE"]

ctrl = [c for c in CONTROL_CELLS if c in cell_outcome]
verd = [c for c in VERDICT_CELLS if c in cell_outcome]

# RIG-VOID: a control that FAILS THE BAR voids the rig, unconditionally -- no
# secondary outcome test may let it escape (grok reproduced the fail-open: gRPC
# controls at ratio 1.200 / bar FAIL, session still emitted VANISHES). An UNSTABLE
# control, or a control showing an effect it must not show, voids it too.
ctrl_void = [c for c in ctrl
             if cell_detail.get(c, {}).get("bar") == "FAIL"
             or cell_outcome[c] in ("UNSTABLE", "REPRODUCES", "INVERSION",
                                    "BAR-FAIL-INCONSISTENT")]
# Controls that are merely noisy or sub-bar do not void, but they are NEVER silent.
ctrl_caveat = [c for c in ctrl if cell_outcome[c] in ("PARTIAL", "UNDERPOWERED")]

if incomplete:
    verdict = "INCOMPLETE"
    why = ("registered cells missing or short of their pairs: %s. The full "
           "registered set must complete before any verdict is read."
           % ", ".join(incomplete))
elif ctrl_void:
    verdict = "RIG-VOID"
    why = ("control cell(s) are not clean: %s. A rig whose gRPC/large control "
           "misbehaves cannot adjudicate a TCP-only claim. NO verdict may be read."
           % ", ".join("%s(%s,bar=%s)" % (c, cell_outcome[c],
                                          cell_detail.get(c, {}).get("bar", "?"))
                       for c in ctrl_void))
else:
    outs = {c: cell_outcome[c] for c in verd}
    repro = [c for c, o in outs.items() if o == "REPRODUCES"]
    inv = [c for c, o in outs.items() if o == "INVERSION"]
    unst = [c for c, o in outs.items() if o == "UNSTABLE"]
    barfi = [c for c, o in outs.items() if o == "BAR-FAIL-INCONSISTENT"]
    van = [c for c, o in outs.items() if o == "VANISHES"]
    part = [c for c, o in outs.items() if o == "PARTIAL"]
    under = [c for c, o in outs.items() if o == "UNDERPOWERED"]

    if unst:
        verdict = "UNSTABLE"
        why = ("bimodal arm(s) whose bar verdict flips on pooled runs: %s. Report as "
               "unstable, NOT resolved." % ", ".join(unst))
    elif barfi:
        verdict = "BAR-FAIL-INCONSISTENT"
        why = ("the medians breach the 1.10 bar in %s, but the paired differences do "
               "not agree in sign. This is NOT a null and NOT a clean reproduction: "
               "the cell is self-contradictory (pf-0's bistability shape). Report the "
               "runs verbatim." % ", ".join(barfi))
    elif repro and inv:
        verdict = "MIXED-SIGN"
        why = ("reproduces in %s but INVERTS in %s -- a host x role interaction "
               "this rig cannot decompose. INCONCLUSIVE for the pairing question."
               % (", ".join(repro), ", ".join(inv)))
    elif repro:
        verdict = "REPRODUCES"
        why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: it "
               "shows P1 CAN occur macOS<->macOS, so P1 is not waivable as 'Windows "
               "residue'. It does NOT establish a platform-general layout cost, it "
               "does NOT name the mechanism, it does NOT kill H1 (H1 accuses code, and "
               "that code runs here too), and it leaves macOS/APFS and host x role "
               "explanations OPEN." % ", ".join(repro))
    elif inv:
        verdict = "INVERSION"
        why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank "
               "this as 'P1 absent'." % ", ".join(inv))
    elif under:
        verdict = "INCONCLUSIVE-UNDERPOWERED"
        why = ("cells cannot exclude an effect of size min(bar_breach, %d ms): %s. A "
               "PASS here is NOT 'P1 vanishes' -- the instrument could not have seen "
               "it (pf-0's error, pre-empted)." % (DELTA_REF, ", ".join(under)))
    elif van and len(van) == len(verd):
        verdict = "VANISHES"
        why = ("both TCP-mixed cells EXCLUDE an effect of size min(bar_breach, %d ms) "
               "(a genuine equivalence result). Scoped to THIS pair: P1 did not "
               "reproduce macOS<->macOS. That is CONSISTENT with 'the Windows peer is "
               "required' but does NOT prove it -- it could equally be a property of "
               "these two machines, their disks, or this macOS version." % DELTA_REF)
    elif part:
        verdict = "PARTIAL"
        why = ("a real but sub-bar asymmetry in: %s. Neither a reproduction nor a "
               "vanish; pf-1 owns it." % ", ".join(part))
    else:
        verdict = "INCONCLUSIVE"
        why = "no registered case matched cleanly; report the cells verbatim."

    if ctrl_caveat:
        why += ("\n\nCONTROL CAVEAT (does not void the rig, and is not silent): %s "
                "show a real sub-bar asymmetry or cannot exclude one. P1 is claimed "
                "TCP-only and mixed-only; weigh this against that claim."
                % ", ".join("%s(%s)" % (c, cell_outcome[c]) for c in ctrl_caveat))

lines.append("SESSION VERDICT: %s" % verdict)
lines.append("")
lines.append(why)
lines.append("")
lines.append("Per-cell outcomes (the rule is graded on paired.csv):")
for c in sorted(cell_outcome):
    dt = cell_detail.get(c)
    if dt:
        lines.append(
            "  %-14s %-22s ratio=%.3f bar=%s  D=%+dms CI=[%+d,%+d] (%.1f%%) "
            "margin=[%+d,%+d] sign_p=%.4f (%d/%d pos)"
            % (c, cell_outcome[c], dt["ratio"], dt["bar"], dt["D"],
               dt["ci"][0], dt["ci"][1], 100 * dt["cov"],
               round(dt["margin"][1]), round(dt["margin"][0]),
               dt["p"], dt["k"], dt["n"]))
    else:
        lines.append("  %-14s %s" % (c, cell_outcome[c]))
lines.append("")
lines.append("CI = exact order-statistic interval on the median; its true coverage is")
lines.append("printed per cell (n=8 admits no exact 95% interval -- the conservative")
lines.append("side is taken deliberately). A null requires the CI to lie strictly")
lines.append("inside the margin, which is min(bar_breach, DELTA_REF=%dms)." % DELTA_REF)
lines.append("")
lines.append("This file is COMPUTED from the pre-registered rule. It declares nothing")
lines.append("beyond it, and the owner walks the numbers.")

open(sess_p, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
