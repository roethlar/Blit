#!/usr/bin/env python3
"""Mechanize the Mac<->Mac pre-registered decision rule.

docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md is the spec. The harness
must COMPUTE the verdict, not leave it to be applied by hand after the numbers
are visible -- that is what pre-registration exists to prevent (codex BLOCKER 1).

The noise statistic is a PAIRED inference, not a range. A range (max-min) grows
with n and is dominated by outliers, so a large consistent effect can hide under
it: with srcinit=2000 and d=[0,180,180,190,190,200,200,200] a range rule reports
"VANISHES" despite 7/8 positive pairs and an effect 83% the size of rig W's P1
(codex BLOCKER 2). Instead:

  d_i  = destinit_i - srcinit_i     (positive = destination-initiated is slower)
  D    = median(d_i)
  CI   = 95% bootstrap CI on the median (seeded => the verdict is deterministic)
  sign = exact two-sided binomial test on the count of positive d_i

  BAR_BREACH = the effect that would push this cell's ratio to the 1.10 bar
             = 0.10 * srcinit_median

  REPRODUCES : bar FAILS and CI_lo > 0            (a real, bar-breaking slowdown)
  INVERSION  : bar FAILS and CI_hi < 0            (source-initiated is the slow arm)
  VANISHES   : bar PASSES and |CI| lies strictly inside +/-BAR_BREACH
               -> a genuine EQUIVALENCE result: an effect big enough to matter is
                  EXCLUDED, not merely unobserved.
  PARTIAL    : bar PASSES, CI excludes 0, but the effect is not excluded as small
  UNDERPOWERED: bar PASSES and the CI is too wide to exclude a bar-breaching
               effect -> a null here is INCONCLUSIVE, never "P1 vanishes".
"""
import csv, os, random, sys
from math import comb

runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]
DELTA_REF = int(os.environ.get("DELTA_REF_MS", "230"))
VERDICT_CELLS = os.environ.get("VERDICT_CELLS", "").split(",")
CONTROL_CELLS = os.environ.get("CONTROL_CELLS", "").split(",")

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
    """Low median for even n, stated and applied consistently (codex LOW)."""
    v = sorted(v)
    return v[(len(v) - 1) // 2]


def complete(c):
    if c not in meta or meta[c]["complete"] != "yes":
        return False
    arms = [a for (cc, a) in by if cc == c]
    return "srcinit" in arms and "destinit" in arms


def boot_ci(d, iters=10000, seed=12345):
    """95% bootstrap CI on the median. Seeded: the verdict must be reproducible."""
    rng = random.Random(seed)
    n = len(d)
    meds = sorted(med([d[rng.randrange(n)] for _ in range(n)]) for _ in range(iters))
    return meds[int(0.025 * iters)], meds[int(0.975 * iters) - 1]


def sign_p(d):
    """Exact two-sided binomial test on the count of positive differences."""
    nz = [x for x in d if x != 0]
    n = len(nz)
    if n == 0:
        return 1.0, 0, 0
    k = sum(1 for x in nz if x > 0)
    tail = sum(comb(n, i) for i in range(0, min(k, n - k) + 1))
    return min(1.0, 2.0 * tail / (2 ** n)), k, n


# ---- summary: every run printed (pf-0's bistability lesson) ------------------
with open(sum_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,runs\n")
    for (c, a) in sorted(by):
        if not complete(c):
            continue
        v = by[(c, a)]
        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s\n" % (
            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
            void.get((c, a), 0), " ".join(str(x) for x in v)))

# ---- paired stats + per-cell outcome ----------------------------------------
cell_outcome, cell_detail = {}, {}
with open(pair_p, "w") as f:
    f.write("cell,n_pairs,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,"
            "sign_p,k_pos_of_n,bar_breach_ms,delta_ref_ms,powered_for_null,unstable,outcome\n")
    for c in sorted(meta):
        if not complete(c):
            cell_outcome[c] = "INCOMPLETE"
            f.write("%s,,,,,,,,,,,,,,,INCOMPLETE\n" % c)
            continue
        d = [v["destinit"] - v["srcinit"]
             for (cc, _run), v in sorted(slots.items())
             if cc == c and "srcinit" in v and "destinit" in v]
        s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
        hi, lo = max(s_med, d_med), min(s_med, d_med)
        bar = "PASS" if 10 * hi <= 11 * lo else "FAIL"      # integer-exact
        D = med(d)
        ci_lo, ci_hi = boot_ci(d)
        p, k, n = sign_p(d)
        breach = 0.10 * s_med                                # effect that reaches 1.10
        powered = (ci_hi - ci_lo) < breach                   # can we exclude a breaching effect?

        # UNSTABLE, as a STATISTIC not a vibe: an arm splits into two clusters
        # separated by more than the paired spread, AND the bar verdict flips when
        # graded on pooled runs instead of medians.
        unstable = "no"
        for arm in ("srcinit", "destinit"):
            v = sorted(by[(c, arm)])
            gaps = [(v[i + 1] - v[i], i) for i in range(len(v) - 1)]
            gmax, gi = max(gaps) if gaps else (0, 0)
            if gmax > (max(d) - min(d)) and gmax > 0:
                pooled_hi = max(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
                pooled_lo = min(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
                pooled_bar = "PASS" if 10 * pooled_hi <= 11 * pooled_lo else "FAIL"
                if pooled_bar != bar:
                    unstable = "yes"

        if bar == "FAIL" and ci_lo > 0:
            out = "REPRODUCES"
        elif bar == "FAIL" and ci_hi < 0:
            out = "INVERSION"
        elif bar == "PASS" and ci_lo > -breach and ci_hi < breach:
            out = "VANISHES"
        elif bar == "PASS" and not powered:
            out = "UNDERPOWERED"
        elif bar == "PASS" and (ci_lo > 0 or ci_hi < 0):
            out = "PARTIAL"
        else:
            out = "INCONCLUSIVE"
        if unstable == "yes":
            out = "UNSTABLE"

        cell_outcome[c] = out
        cell_detail[c] = dict(D=D, ci=(ci_lo, ci_hi), p=p, k=k, n=n, bar=bar,
                              ratio=hi / lo if lo else 0.0, breach=breach)
        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%d/%d,%d,%d,%s,%s,%s\n" % (
            c, len(d), s_med, d_med, (hi / lo if lo else 0.0), bar, D, ci_lo, ci_hi,
            p, k, n, breach, DELTA_REF, "yes" if powered else "no", unstable, out))

# ---- per-cell invariance rows (unchanged shape) ------------------------------
with open(ver_p, "w") as f:
    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,delta_ms,bar,outcome\n")
    for c in sorted(meta):
        if not complete(c):
            f.write("%s,invariance,srcinit,destinit,,,,,1.10,INCOMPLETE\n" % c)
            continue
        s, dd = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
        hi, lo = max(s, dd), min(s, dd)
        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,1.10,%s\n" % (
            c, s, dd, hi / lo if lo else 0.0, dd - s,
            "PASS" if 10 * hi <= 11 * lo else "FAIL"))

# ---- SESSION VERDICT: the six registered outcomes, in strict precedence ------
lines = []
ctrl = [c for c in CONTROL_CELLS if c in cell_outcome]
verd = [c for c in VERDICT_CELLS if c in cell_outcome]

ctrl_fail = [c for c in ctrl
             if cell_outcome[c] not in ("VANISHES", "INCONCLUSIVE", "UNDERPOWERED")
             and cell_detail.get(c, {}).get("bar") == "FAIL"]
incomplete = [c for c in (ctrl + verd) if cell_outcome[c] == "INCOMPLETE"]

if incomplete:
    verdict = "INCOMPLETE"
    why = "cells did not complete: %s" % ", ".join(incomplete)
elif ctrl_fail:
    # 1. RIG-VOID -- a rig whose control fails cannot adjudicate a TCP-only claim.
    verdict = "RIG-VOID"
    why = ("control cell(s) FAILED the 1.10 bar: %s. The rig is not measuring "
           "cleanly; NO verdict may be read." % ", ".join(ctrl_fail))
else:
    outs = {c: cell_outcome[c] for c in verd}
    repro = [c for c, o in outs.items() if o == "REPRODUCES"]
    inv = [c for c, o in outs.items() if o == "INVERSION"]
    unst = [c for c, o in outs.items() if o == "UNSTABLE"]
    van = [c for c, o in outs.items() if o == "VANISHES"]
    part = [c for c, o in outs.items() if o == "PARTIAL"]
    under = [c for c, o in outs.items() if o in ("UNDERPOWERED", "INCONCLUSIVE")]

    if unst:
        verdict = "UNSTABLE"
        why = ("bimodal arm(s) whose verdict flips on pooled runs: %s. Report as "
               "unstable, NOT resolved." % ", ".join(unst))
    elif repro and inv:
        verdict = "MIXED-SIGN"
        why = ("reproduces in %s but INVERTS in %s -- a host x role interaction "
               "this rig cannot decompose. INCONCLUSIVE for the pairing question."
               % (", ".join(repro), ", ".join(inv)))
    elif repro:
        verdict = "REPRODUCES"
        why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: "
               "it shows P1 CAN occur macOS<->macOS -- it does NOT establish a "
               "platform-general layout cost, and it does NOT kill H1 (H1 accuses "
               "code, and that code runs here too)." % ", ".join(repro))
    elif inv:
        verdict = "INVERSION"
        why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank "
               "this as 'P1 absent'." % ", ".join(inv))
    elif under:
        verdict = "INCONCLUSIVE-UNDERPOWERED"
        why = ("cells cannot exclude a bar-breaching effect: %s. A PASS here is NOT "
               "'P1 vanishes' -- the instrument could not have seen it (pf-0's "
               "error, pre-empted)." % ", ".join(under))
    elif van and len(van) == len(verd):
        verdict = "VANISHES"
        why = ("both TCP-mixed cells EXCLUDE a bar-breaching effect (equivalence). "
               "Scoped to THIS pair: P1 did not reproduce macOS<->macOS. That is "
               "CONSISTENT with 'Windows is required' but does NOT prove it -- it "
               "could be a property of these two machines/disks/OS version.")
    elif part:
        verdict = "PARTIAL"
        why = ("a real but sub-bar asymmetry in: %s. Neither a reproduction nor a "
               "vanish; pf-1 owns it." % ", ".join(part))
    else:
        verdict = "INCONCLUSIVE"
        why = "no registered case matched cleanly; report the cells verbatim."

lines.append("SESSION VERDICT: %s" % verdict)
lines.append("")
lines.append(why)
lines.append("")
lines.append("Per-cell outcomes (the rule is graded on paired.csv):")
for c in sorted(cell_outcome):
    d = cell_detail.get(c)
    if d:
        lines.append("  %-14s %-12s ratio=%.3f bar=%s  D=%+dms CI=[%+d,%+d] sign_p=%.3f (%d/%d pos)"
                     % (c, cell_outcome[c], d["ratio"], d["bar"], d["D"],
                        d["ci"][0], d["ci"][1], d["p"], d["k"], d["n"]))
    else:
        lines.append("  %-14s %s" % (c, cell_outcome[c]))
lines.append("")
lines.append("This file is COMPUTED from the pre-registered rule. It declares nothing")
lines.append("beyond it, and the owner walks the numbers.")

open(sess_p, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
