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

THE THREE QUESTIONS (rev 7) -- kept apart, because tangling them produced the SAME
class of defect in rounds 3, 4, 5 AND 6. ALL INFERENCE IS PAIRED; the bar (marginal
medians) is the project's ACCEPTANCE criterion and takes no part in inference.

  DIRECTION   = the SIGN TEST      directional = sign_p < .05  (zeros dropped)
  MAGNITUDE   = the paired CI      material     = CI_lo >= BREACH_HI
                                   material_neg = CI_hi <= BREACH_LO
  EQUIVALENCE = the CI vs MARGIN   null_excl    = CI strictly inside the margin

PER-CELL OUTCOMES (exhaustive; no unreportable region)
  REPRODUCES            dir_pos and material
  INVERSION             dir_neg and material_neg
  PARTIAL               a real direction whose magnitude is NOT material
  VANISHES              no direction AND null_excl -- a genuine EQUIVALENCE result
  UNDERPOWERED          no direction and the CI cannot exclude the margin -> a PASS
                        here is NOT "P1 vanishes"; the rig could not have seen it
  BAR-FAIL-INCONSISTENT the bar FAILS but the pairs establish NO consistent direction
  UNSTABLE              (override) an arm is bimodal AND the bar flips on pooled runs
  INCOMPLETE            the cell did not finish its registered pairs

THE CONTROLS ARE A PRECONDITION, NOT A FOOTNOTE
  CONTAMINATING  a directional effect whose CI sits at/beyond the margin, or bimodal
                 -> RIG-VOID. The rig is carrying the effect we came to measure.
  CERTIFIED      bar PASSES and the paired CI lies strictly inside HALF the margin.
                 Half, because certifying a control with the very threshold that
                 DEFINES the effect is incoherent -- it would let a control carry all
                 but 1 ms of P1 and still call the rig clean (round-6, grok).
  otherwise      NOT CERTIFIED -> CONTROLS-UNCERTIFIED, and NO measurand verdict may
                 be read: not a null, and NOT a reproduction either. Uncertainty about
                 a rig-wide confound is not evidence that the confound is absent
                 (round-6, codex).
"""
import csv, os, sys
from math import comb

runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]

# ---- THE REGISTERED CONSTANTS ARE PINNED IN CODE, NOT TAKEN FROM THE ENVIRONMENT --
# Round-5 (codex, BLOCKER): they were env-overridable, so `DELTA_REF_MS=240` turned a
# RIG-VOID into a VANISHES -- i.e. the pre-registered decision rule could be edited
# from the command line, by the same person who wants a particular answer, AFTER the
# data existed. The whole point of pre-registration is that this is impossible.
#
# A deviation is not silently accepted and not silently ignored: it REFUSES.
REGISTERED_DELTA_REF_MS = 230        # rig W's measured Delta_P1 (the reference effect)
REGISTERED_PAIRS = (8, 16)           # 8 registered; 16 the UNDERPOWERED escalation
MIN_COVERAGE = 0.95

DELTA_REF = REGISTERED_DELTA_REF_MS
_env_delta = os.environ.get("DELTA_REF_MS")
if _env_delta is not None and _env_delta.strip() != str(REGISTERED_DELTA_REF_MS):
    sys.stderr.write(
        "REFUSING: DELTA_REF_MS=%r but the PRE-REGISTERED reference effect is %d ms. "
        "The decision rule is not tunable from the environment -- that is what "
        "pre-registration exists to prevent.\n" % (_env_delta, REGISTERED_DELTA_REF_MS))
    raise SystemExit(2)


def cells_env(name):
    return [c for c in os.environ.get(name, "").split(",") if c]


VERDICT_CELLS = cells_env("VERDICT_CELLS")
CONTROL_CELLS = cells_env("CONTROL_CELLS")
# The controls are a PRECONDITION for reading any verdict, so an engine invoked
# WITHOUT them cannot grade anything (round-6 grok, LOW: called standalone with no
# controls it happily emitted VANISHES -- a footgun aimed at exactly the person who
# would re-grade a CSV by hand).
if not VERDICT_CELLS or not CONTROL_CELLS:
    sys.stderr.write(
        "REFUSING: VERDICT_CELLS and CONTROL_CELLS must both be set. The controls are "
        "a precondition for any verdict -- an engine with no controls cannot certify "
        "the rig, and must not pretend to.\n")
    raise SystemExit(2)
# The full registered set must be PRESENT and COMPLETE. A partial CELLS set that is
# merely filtered lets a one-cell run emit VANISHES while claiming "both" cells
# vanished (codex r2 BLOCKER 1).
REGISTERED_CELLS = cells_env("REGISTERED_CELLS") or (VERDICT_CELLS + CONTROL_CELLS)
# The engine is separately executable and is hashed into the manifest, so it must
# not depend on the harness telling it the truth. Round-3 grok (HIGH): it trusted
# `meta.complete == yes` and never checked n, so a CSV with ONE pair and a lying
# meta produced VANISHES at 0% CI coverage -- a confident false equivalence claim.
REQUIRED_PAIRS = int(os.environ.get("REQUIRED_PAIRS", "8"))
if REQUIRED_PAIRS not in REGISTERED_PAIRS:
    sys.stderr.write(
        "REFUSING: REQUIRED_PAIRS=%d is not a registered pair count %s.\n"
        % (REQUIRED_PAIRS, REGISTERED_PAIRS))
    raise SystemExit(2)
# A session-level void the HARNESS detected (e.g. end-load above the bar). The
# engine must be able to refuse a verdict on evidence it cannot see itself.
SESSION_VOID_REASON = os.environ.get("SESSION_VOID_REASON", "").strip()

rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}


def ms_of(r):
    """A corrupt row must stop the grading, LOUDLY. Mapping it to a soft outcome
    would hide the corruption; a traceback would obscure it (round-3 grok, LOW)."""
    try:
        return int(r["ms"])
    except (TypeError, ValueError):
        sys.stderr.write(
            "CORRUPT ROW: cell=%s arm=%s run=%s has non-numeric ms=%r. Refusing to "
            "grade -- a benchmark whose rows do not parse has no verdict.\n"
            % (r.get("cell"), r.get("arm"), r.get("run"), r.get("ms")))
        raise SystemExit(2)


by, slots, void = {}, {}, {}
for r in rows:
    key = (r["cell"], r["arm"])
    if r["valid"] == "yes":
        by.setdefault(key, []).append(ms_of(r))
        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = ms_of(r)
    else:
        void[key] = void.get(key, 0) + 1


def med(v):
    """Low median for even n, stated once and applied consistently."""
    v = sorted(v)
    return v[(len(v) - 1) // 2]


def complete(c):
    """COMPLETE is checked against the DATA, not against meta's say-so.

    Round-3 (grok, HIGH): this trusted `meta.complete == yes` and required only
    >= 1 pair, so a one-pair CSV with a lying meta graded as a full cell and
    emitted VANISHES at 0% CI coverage. The pair count is now enforced here, and
    the CI's coverage is enforced at the grading site.
    """
    if c not in meta or meta[c].get("complete") != "yes":
        return False
    arms = [a for (cc, a) in by if cc == c]
    if "srcinit" not in arms or "destinit" not in arms:
        return False
    return len(paired(c)) >= REQUIRED_PAIRS


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

        # A CI that does not reach the registered confidence level cannot ground
        # ANY outcome -- least of all a null. Grading on it is how the n=1 session
        # emitted VANISHES at 0% coverage.
        if cov < MIN_COVERAGE:
            cell_outcome[c] = "INCOMPLETE"
            f.write("%s,%d,,,,,,,,%.4f,,,,,,,%d,,,INCOMPLETE\n" % (c, len(d), cov, DELTA_REF))
            continue

        # The bar is symmetric in RATIO, so the two boundaries are NOT symmetric in
        # ms: +src/10 reaches 1.10, but only -src/11 reaches the INVERSE 1.10.
        breach_hi = s_med / 10.0
        breach_lo = -s_med / 11.0
        # A null must exclude an effect the size of the one rig W measured (230 ms),
        # not merely one the bar would forgive -- on a slow arm the bar is WIDER.
        margin_hi = min(breach_hi, float(DELTA_REF))
        margin_lo = max(breach_lo, -float(DELTA_REF))

        # THE THREE QUESTIONS, KEPT SEPARATE. Rounds 3, 4 and 5 all produced the same
        # class of defect by tangling them together, so they are now disentangled and
        # each is answered by the statistic that can actually answer it:
        #
        #   DIRECTION  -- the SIGN TEST. Is there a consistent direction at all?
        #   MAGNITUDE  -- the CI. Is the effect big enough to matter, IN THAT DIRECTION?
        #   EQUIVALENCE-- the CI vs the MARGIN. Is a material effect EXCLUDED?
        #
        # Round-5 (codex, BLOCKER): `bar == "FAIL"` carried NO DIRECTION, yet made an
        # effect of EITHER sign "material" -- so at n=16, thirteen +1 ms pairs and three
        # -110 ms pairs (marginal medians failing the bar in the INVERSE direction) gave
        # a clean `REPRODUCES` for a ONE MILLISECOND effect. A bar failure is only
        # material to a claim that points the SAME WAY as the bar failure.
        #
        # Round-5 (grok, BLOCKER): a single ZERO pair dragged `ci_lo` to 0, which killed
        # the old `pos_effect` (it demanded `ci_lo > 0`) -- so `d = [0, 99x7]` at
        # src=1000 was "no effect" AND null_excl (99 < margin 100) and reported
        # `VANISHES`, while the sign test REJECTED at p = .0156. Seven of eight pairs
        # carried a 99 ms effect, one millisecond under the bar, and it was called
        # equivalence. DIRECTION is the sign test's job, not the CI's.
        # ALL INFERENCE IS PAIRED. The bar is computed on the MARGINAL medians; the CI
        # on the PAIRED differences. They are different statistics and they can point
        # OPPOSITE WAYS (round-5), or agree in direction while disagreeing wildly in
        # magnitude (round-6). Rev 6 tried to fix that by making the bar failure
        # direction-aware -- and codex promptly drove `material` again: at n=16 a
        # paired D of ONE MILLISECOND (CI [1,1], 16/16 positive) still reported
        # REPRODUCES, because three outliers moved the MARGINAL median enough to fail
        # the bar in the matching direction, and `material` accepted a bar failure as
        # a substitute for paired magnitude.
        #
        # So the bar no longer participates in INFERENCE AT ALL. It is the project's
        # ACCEPTANCE criterion: it is computed, reported in every row, and used to
        # judge a CELL against the 1.10 invariance bar -- but direction and magnitude
        # are decided by the paired statistics, and by nothing else.
        directional = p < 0.05                       # DIRECTION  -- the sign test
        dir_pos = directional and k > (n - k)
        dir_neg = directional and k < (n - k)
        material = ci_lo >= breach_hi                # MAGNITUDE  -- the paired CI, only
        material_neg = ci_hi <= breach_lo
        null_excl = (ci_lo > margin_lo) and (ci_hi < margin_hi)   # EQUIVALENCE

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

        if dir_pos and material:
            out = "REPRODUCES"
        elif dir_neg and material_neg:
            out = "INVERSION"
        elif directional:
            # A real, consistent direction that is NOT material. NEVER a null -- this
            # is where grok's [0, 99x7] belongs, not in VANISHES.
            out = "PARTIAL"
        elif bar == "FAIL":
            # The medians breach 1.10 but the pairs establish no consistent direction.
            out = "BAR-FAIL-INCONSISTENT"
        elif null_excl:
            out = "VANISHES"
        else:
            out = "UNDERPOWERED"
        if unstable == "yes":
            out = "UNSTABLE"

        cell_outcome[c] = out
        cell_detail[c] = dict(
            D=D, ci=(ci_lo, ci_hi), cov=cov, p=p, k=k, n=n, bar=bar,
            ratio=hi / lo if lo else 0.0,
            breach=(breach_hi, breach_lo),
            margin=(margin_hi, margin_lo), null_excl=null_excl,
            directional=directional,
            # The whole CI sits at or beyond the margin, in the direction of the
            # effect: the cell is CARRYING a material asymmetry, not merely failing to
            # exclude one.
            ci_at_or_beyond_margin=(dir_pos and ci_lo >= margin_hi)
            or (dir_neg and ci_hi <= margin_lo))
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

# RIG-VOID. A control must be CLEAN, and "clean" is measured by the SAME absolute
# materiality the power gate uses -- not by the bar alone.
#
# Round 2 (grok): a control with bar FAIL whose CI crossed zero escaped the void,
# and a session emitted VANISHES with its controls at ratio 1.200. Fixed.
# Round 3 (grok, BLOCKER -- REPRODUCED): the SAME structural hole survived one level
# down. A control with a real, 8/8, rig-W-sized effect (d_i = 230 in every pair) on
# a SLOW arm (src=2500 -> ratio 1.092) is bar-PASS, lands as PARTIAL, and escaped
# the void -- so the session printed a clean VANISHES while every control carried
# the exact effect size the power gate is built around. On a slow arm the bar is
# WIDER than DELTA_REF; that is the very thing the margin exists to fix, and the
# control rule was still using the bar.
#
# A control therefore voids the rig unless its own effect is EXCLUDED as smaller
# than the margin (null_excl) -- i.e. unless the control itself passes the
# equivalence test. A tiny consistent asymmetry (host x role: q is the faster Mac)
# is immaterial and does NOT void; a margin-sized one does.
# WHAT A CONTROL MUST PROVE -- expressed as the question, not as a list of labels.
#
# Three rounds running, this rule was written as "void if the outcome is one of
# {...}", and three times an effect walked through a label that was not on the list:
#   r3: a bar-FAIL control whose CI crossed zero was INCONCLUSIVE -> escaped.
#   r4: a Delta_ref-sized control effect on a slow arm was PARTIAL -> escaped.
#   r5: ONE zero pair made a 7/8 Delta_ref control UNDERPOWERED -> escaped, and the
#       session printed VANISHES with every control carrying D=+230.
# So it is no longer written that way. There are exactly two questions:
#
#   1. Is the control CONTAMINATING? -- it carries a directional effect whose whole
#      CI sits at or beyond the margin, or it fails the bar, or it is bimodal.
#      Nothing in this rig can be trusted; the session is RIG-VOID.
#   2. Is the control CERTIFIED CLEAN? -- its effect is EXCLUDED as smaller than the
#      margin (null_excl). If it is not, we cannot say the rig is free of a
#      material arm asymmetry, so A NULL IS NOT AVAILABLE. (It does not void a
#      REPRODUCTION: a merely NOISY control does not manufacture a consistent 8/8
#      one-directional effect in the measurand, and voiding real evidence on that
#      basis would be its own false negative -- grok, round-5 NEW-5, which is why
#      an unproven control blocks the null rather than killing the session.)
# CONTAMINATING: the rig is CARRYING the effect we came to measure. Nothing here can
# be trusted -> RIG-VOID. Paired evidence only (a marginal-median bar failure with
# clean pairs is not contamination -- it made a control simultaneously "certified" and
# "contaminating", a contradiction codex drove to a FALSE RIG-VOID).
def _ctrl_contaminating(c):
    dt = cell_detail.get(c, {})
    if cell_outcome[c] == "UNSTABLE":
        return True
    return bool(dt.get("directional") and dt.get("ci_at_or_beyond_margin"))


# CERTIFIED CLEAN: and the threshold for a CONTROL must be STRICTLY TIGHTER than the
# effect we claim to detect in the MEASURAND. Round-6 (grok, BLOCKER): certification
# used the SAME margin as materiality, so a control carrying D = +229 ms -- ONE
# MILLISECOND under the reference effect -- certified as "clean", and the session
# printed VANISHES with the prose "every control is CERTIFIED clean". Certifying a
# control with the very threshold that defines the effect is incoherent: it would let
# us claim P1 is TCP-only while the gRPC control carries all but 1 ms of it.
#
# So a control must carry LESS THAN HALF the material effect. That is not an invented
# number: it is the specificity claim itself, made checkable. P1 is asserted to be
# TCP-only and mixed-only; if a control carries half the effect, that assertion is not
# readable off this rig. (At src=2500 -> 115 ms; at src=1000 -> 50 ms; i.e. ~5% of the
# arm, which is the rig noise measured on the q-baseline, 2-4%.)
def _ctrl_certified(c):
    dt = cell_detail.get(c, {})
    if not dt:
        return False
    if dt.get("bar") == "FAIL":
        return False            # a control breaching the acceptance bar certifies nothing
    lo, hi = dt["ci"]
    m_hi, m_lo = dt["margin"]
    return (lo > m_lo / 2.0) and (hi < m_hi / 2.0)


ctrl_void = [c for c in ctrl if _ctrl_contaminating(c)]
# NOT CERTIFIED => NO VERDICT MAY BE READ ABOUT THE MEASURAND -- not a null, and NOT A
# REPRODUCTION EITHER (round-6 codex, BLOCKER: uncertified controls blocked only
# VANISHES, so with every control at D=+230 the engine still confidently declared P1
# REPRODUCED). "Uncertainty about a rig-wide confound is not evidence that the confound
# is absent" -- and P1's whole claim is that the effect is specific to TCP x mixed.
ctrl_uncertified = [c for c in ctrl if c not in ctrl_void and not _ctrl_certified(c)]
# Controls that certify clean but still carry a real, tiny asymmetry (host x role -- q
# is the faster Mac) do not block anything, and are NEVER silent.
ctrl_caveat = [c for c in ctrl
               if c not in ctrl_void and c not in ctrl_uncertified
               and cell_outcome[c] == "PARTIAL"]

if incomplete:
    verdict = "INCOMPLETE"
    why = ("registered cells missing, short of their %d pairs, or graded on a CI "
           "below the registered %.0f%% coverage: %s. The full registered set must "
           "complete before any verdict is read."
           % (REQUIRED_PAIRS, 100 * MIN_COVERAGE, ", ".join(incomplete)))
elif SESSION_VOID_REASON:
    # Evidence the engine cannot see for itself (end-load, an operator abort).
    verdict = "RIG-VOID"
    why = ("the harness voided the session: %s. NO verdict may be read."
           % SESSION_VOID_REASON)
elif ctrl_void:
    verdict = "RIG-VOID"
    why = ("control cell(s) are CONTAMINATING -- the rig is carrying the very effect "
           "this experiment measures: %s. NO verdict may be read."
           % ", ".join("%s(%s,bar=%s)" % (c, cell_outcome[c],
                                          cell_detail.get(c, {}).get("bar", "?"))
                       for c in ctrl_void))
elif ctrl_uncertified:
    # BEFORE any measurand branch. A control that cannot be certified clean blocks
    # EVERY verdict -- the null AND the reproduction. P1 is claimed TCP-only and
    # mixed-only; if the gRPC/large controls might be carrying the same arm asymmetry,
    # then neither "it reproduced" nor "it vanished" is readable off this rig.
    verdict = "CONTROLS-UNCERTIFIED"
    why = ("control cell(s) could NOT be certified free of an arm asymmetry: %s. A "
           "control must carry LESS THAN HALF the material effect for P1's TCP-only / "
           "mixed-only claim to be readable here. Until they do, NO measurand verdict "
           "may be read -- not a null, and NOT a reproduction: uncertainty about a "
           "rig-wide confound is not evidence that the confound is absent. Re-run with "
           "the registered RUNS=16 escalation to buy the power to certify them."
           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d])"
                       % (c, cell_outcome[c], cell_detail.get(c, {}).get("D", 0),
                          cell_detail.get(c, {}).get("ci", (0, 0))[0],
                          cell_detail.get(c, {}).get("ci", (0, 0))[1])
                       for c in ctrl_uncertified))
else:
    outs = {c: cell_outcome[c] for c in verd}
    repro = [c for c, o in outs.items() if o == "REPRODUCES"]
    inv = [c for c, o in outs.items() if o == "INVERSION"]
    unst = [c for c, o in outs.items() if o == "UNSTABLE"]
    barfi = [c for c, o in outs.items() if o == "BAR-FAIL-INCONSISTENT"]
    van = [c for c, o in outs.items() if o == "VANISHES"]
    part = [c for c, o in outs.items() if o == "PARTIAL"]
    under = [c for c, o in outs.items() if o == "UNDERPOWERED"]

    # PRECEDENCE. A clean reproduction in EITHER direction answers the registered
    # question, and a messy SIBLING cell does not retract it (round-3 grok, HIGH:
    # UNSTABLE and BAR-FAIL-INCONSISTENT outranked REPRODUCES, so a clean 8/8
    # reproduction in nq was reported as BAR-FAIL-INCONSISTENT because qn was noisy
    # -- a FALSE NON-REPRODUCTION against the pre-registration's "either direction"
    # rule). MIXED-SIGN still outranks it: a reproduction in one direction and an
    # INVERSION in the other is evidence of the host x role artifact itself.
    #
    # Demoting UNSTABLE below REPRODUCES cannot leak a null: VANISHES requires ALL
    # measurand cells to VANISH, so any unstable sibling still blocks it.
    if repro and inv:
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
        messy = [c for c in (unst + barfi)]
        if messy:
            why += ("\n\nSIBLING CAVEAT: the other direction is not clean (%s). The "
                    "pre-registration answers the question on EITHER direction, so "
                    "the reproduction stands -- but the sibling is reported, not "
                    "buried, and it is NOT evidence of an inversion."
                    % ", ".join("%s(%s)" % (c, cell_outcome[c]) for c in messy))
    elif inv:
        verdict = "INVERSION"
        why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank "
               "this as 'P1 absent'." % ", ".join(inv))
    elif unst:
        verdict = "UNSTABLE"
        why = ("bimodal arm(s) whose bar verdict flips on pooled runs: %s. Report as "
               "unstable, NOT resolved." % ", ".join(unst))
    elif barfi:
        verdict = "BAR-FAIL-INCONSISTENT"
        why = ("the medians breach the 1.10 bar in %s, but the paired evidence does "
               "NOT establish a consistent effect (the CI includes 0, or the sign "
               "test does not reject). This is NOT a null and NOT a clean "
               "reproduction: the cell contradicts itself (pf-0's bistability shape). "
               "Report the runs verbatim." % ", ".join(barfi))
    elif under:
        verdict = "INCONCLUSIVE-UNDERPOWERED"
        why = ("cells cannot exclude an effect of size min(bar_breach, %d ms): %s. A "
               "PASS here is NOT 'P1 vanishes' -- the instrument could not have seen "
               "it (pf-0's error, pre-empted)." % (DELTA_REF, ", ".join(under)))
    elif van and len(van) == len(verd):
        verdict = "VANISHES"
        why = ("both TCP-mixed cells EXCLUDE an effect of size min(bar_breach, %d ms), "
               "and every control is CERTIFIED clean (a genuine equivalence result). "
               "Scoped to THIS pair: P1 did not reproduce macOS<->macOS. That is "
               "CONSISTENT with 'the Windows peer is required' but does NOT prove it -- "
               "it could equally be a property of these two machines, their disks, or "
               "this macOS version." % DELTA_REF)
    elif part:
        verdict = "PARTIAL"
        why = ("a real but sub-bar asymmetry in: %s. Neither a reproduction nor a "
               "vanish; pf-1 owns it." % ", ".join(part))
    else:
        verdict = "INCONCLUSIVE"
        why = "no registered case matched cleanly; report the cells verbatim."

    if ctrl_caveat:
        # NOT "sub-bar": a Delta_ref-sized control effect is bar-sub only because the
        # arm is slow, and those now VOID. What survives here is either excluded as
        # smaller than the MARGIN, or undetectable. Say that, precisely.
        why += ("\n\nCONTROL CAVEAT (does not void the rig, and is not silent): %s. A "
                "PARTIAL control carries a real asymmetry that is EXCLUDED as smaller "
                "than the margin (min(bar_breach, %d ms)); an UNDERPOWERED control "
                "could not resolve one either way. P1 is claimed TCP-only and "
                "mixed-only; weigh this against that claim."
                % (", ".join("%s(%s)" % (c, cell_outcome[c]) for c in ctrl_caveat),
                   DELTA_REF))

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
