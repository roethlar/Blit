#!/usr/bin/env python3
"""The Mac<->Mac decision rule (PREREGISTRATION.md rev 8, D-2026-07-14-3).

WHAT THIS IS FOR
    The harness COMPUTES the verdict, so no one can look at the numbers and then
    invent a favourable reading. That -- and only that -- is what the mechanization
    buys. The question, the statistic and the thresholds are all fixed before any
    data exists.

WHY IT IS THIS SMALL
    The previous rule had ~10 outcomes, five thresholds, a control certification tier
    and a precedence stack. Seven review rounds; FOUR of the last five BLOCKERs were
    in the RULE, not in the measurement -- every one a corner where the branches
    interacted to produce a confidently wrong verdict (a 1 ms effect reported as a
    reproduction; a control carrying 229 of 230 ms certified "clean"; a null printed
    while every control was dirty). Complexity was the defect. So:

THE STATISTIC (paired, because the design is paired)
    d_i = destinit_i - srcinit_i          per ABBA slot (positive = destination slower)
    D   = median(d_i)                     low median, even n
    CI  = exact distribution-free order-statistic interval on the population median,
          the narrowest whose coverage is >= 95%. At n=8 that is [min(d), max(d)]
          (99.22%); at n=16, [d_(4), d_(13)] (97.87%). No bootstrap, no approximation.

THE THRESHOLD (one)
    T = min(srcinit_median / 10, DELTA_REF)
        srcinit/10  -- the project's own 1.10 invariance bar
        DELTA_REF   -- 230 ms, the effect rig W actually measured
    The smaller of the two: an effect must matter by BOTH standards to count.

THE FOUR CELL STATES (mutually exclusive BY CONSTRUCTION -- there is no label for a
new case to walk past, because they partition the CI's position relative to +-T)
    EFFECT    CI_lo >= +T                 destination-initiated is slower, by >= T
    INVERTED  CI_hi <= -T                 source-initiated is slower, by >= T
    NONE      -T < CI_lo and CI_hi < +T   an effect of size T is EXCLUDED (equivalence)
    UNCLEAR   anything else               the CI spans the threshold: no answer

THE CONTROLS ARE A PRECONDITION
    Every control must be NONE at T/2 -- HALF the threshold. Half, because certifying a
    control with the very number that DEFINES the effect is incoherent: it would let the
    gRPC control carry all but 1 ms of P1 while we call the rig clean. If any control
    fails, NO verdict about the measurand is read: not a reproduction, and not a null.

WHAT IS DELIBERATELY ABSENT
    * The 1.10 bar takes NO part in inference. It is the project's ACCEPTANCE criterion:
      computed on the marginal medians, reported in every row, and never consulted --
      the marginal and paired statistics can disagree in direction AND magnitude, and
      every attempt to let one stand in for the other produced a false verdict.
    * The sign test is REPORTED, not decided on. At n=8 the CI already implies it
      (CI_lo >= T > 0 means every pair is >= T), so making it a second gate only added
      an interaction to get wrong.
    * No UNSTABLE / PARTIAL / BAR-FAIL-INCONSISTENT / UNDERPOWERED branches, and no
      precedence stack. A bimodal arm widens the CI, and a wide CI lands in UNCLEAR --
      which is exactly what those branches were hand-coding. Every run of every arm is
      still printed, so bimodality remains visible to the reader.
"""
import csv, os, sys
from math import comb

runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]

# ---- REGISTERED CONSTANTS: pinned in code, never taken from the environment --------
# They were once `${VAR:-default}`, and DELTA_REF_MS=240 turned a void into a null --
# i.e. the rule could be retuned from the command line, after the data existed, in the
# direction of the answer you want. That is the one thing pre-registration exists to
# make impossible.
DELTA_REF = 230          # ms; rig W's measured Delta_P1
REGISTERED_PAIRS = (8, 16)
MIN_COVERAGE = 0.95

_env = os.environ.get("DELTA_REF_MS")
if _env is not None and _env.strip() != str(DELTA_REF):
    sys.exit("REFUSING: DELTA_REF_MS=%r but the registered reference effect is %d ms. "
             "The rule is not tunable from the environment.\n" % (_env, DELTA_REF))


def cells_env(name):
    return [c for c in os.environ.get(name, "").split(",") if c]


MEASURANDS = cells_env("VERDICT_CELLS")
CONTROLS = cells_env("CONTROL_CELLS")
REGISTERED = cells_env("REGISTERED_CELLS") or (MEASURANDS + CONTROLS)
PAIRS = int(os.environ.get("REQUIRED_PAIRS", "8"))
# A harness-detected session void the engine cannot see for itself (end-load).
SESSION_VOID = os.environ.get("SESSION_VOID_REASON", "").strip()

if not MEASURANDS or not CONTROLS:
    sys.exit("REFUSING: VERDICT_CELLS and CONTROL_CELLS must both be set -- the controls "
             "are a precondition for any verdict, and an engine with none cannot certify "
             "the rig.\n")
if PAIRS not in REGISTERED_PAIRS:
    sys.exit("REFUSING: REQUIRED_PAIRS=%d is not registered %s.\n" % (PAIRS, REGISTERED_PAIRS))


def ms_of(r):
    """A corrupt row stops the grading, loudly. Soft-mapping it would hide it."""
    try:
        return int(r["ms"])
    except (TypeError, ValueError):
        sys.stderr.write("CORRUPT ROW: cell=%s arm=%s run=%s ms=%r. A benchmark whose "
                         "rows do not parse has no verdict.\n"
                         % (r.get("cell"), r.get("arm"), r.get("run"), r.get("ms")))
        raise SystemExit(2)


rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}

by, slots, voided = {}, {}, {}
for r in rows:
    key = (r["cell"], r["arm"])
    if r["valid"] == "yes":
        by.setdefault(key, []).append(ms_of(r))
        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = ms_of(r)
    else:
        voided[key] = voided.get(key, 0) + 1


def med(v):
    v = sorted(v)
    return v[(len(v) - 1) // 2]


def paired(c):
    return [v["destinit"] - v["srcinit"]
            for (cc, _run), v in sorted(slots.items())
            if cc == c and "srcinit" in v and "destinit" in v]


def median_ci(d):
    """Exact order-statistic interval: the NARROWEST [d_(k), d_(n+1-k)] whose coverage
    1 - 2*P(Bin(n,1/2) <= k-1) is still >= 95%. Returns (lo, hi, coverage)."""
    d = sorted(d)
    n = len(d)
    best = None
    for k in range(1, n // 2 + 1):
        cov = 1.0 - 2.0 * sum(comb(n, i) for i in range(k)) / (2.0 ** n)
        if cov >= MIN_COVERAGE:
            best = (d[k - 1], d[n - k], cov)      # larger k => narrower
    return best                                   # None if n is too small for 95%


def sign_p(d):
    """Reported, never decided on."""
    nz = [x for x in d if x]
    n = len(nz)
    if not n:
        return 1.0, 0, 0
    k = sum(1 for x in nz if x > 0)
    tail = sum(comb(n, i) for i in range(min(k, n - k) + 1))
    return min(1.0, 2.0 * tail / 2 ** n), k, n


def thresholds(s_med, scale=1.0):
    """T_pos and T_neg -- NOT symmetric in ms, because the 1.10 bar is symmetric in
    RATIO: +src/10 reaches ratio 1.10, but only -src/11 reaches the INVERSE 1.10.
    Both capped at DELTA_REF, so an effect must matter by the project's bar AND be the
    size of the one rig W measured. `scale` = 0.5 for controls."""
    return (min(s_med / 10.0, float(DELTA_REF)) * scale,
            -min(s_med / 11.0, float(DELTA_REF)) * scale)


def classify(ci_lo, ci_hi, t_pos, t_neg):
    """THE RULE. Four states partitioning the CI's position relative to the thresholds.
    They are mutually exclusive and exhaustive BY CONSTRUCTION -- there is no label here
    for a new case to walk past, which is what went wrong seven rounds in a row."""
    if ci_lo >= t_pos:
        return "EFFECT"
    if ci_hi <= t_neg:
        return "INVERTED"
    if t_neg < ci_lo and ci_hi < t_pos:
        return "NONE"
    return "UNCLEAR"


# ---- grade every registered cell ---------------------------------------------------
cell = {}
for c in sorted(set(REGISTERED) | set(meta)):
    d = paired(c)
    ci = median_ci(d) if d else None
    # COMPLETE is checked against the DATA, never against meta's say-so: a one-pair CSV
    # with a lying meta once graded as a full cell and emitted a null at 0% coverage.
    if (meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None):
        cell[c] = dict(state="INCOMPLETE", n=len(d))
        continue
    s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
    hi, lo = max(s_med, d_med), min(s_med, d_med)
    ci_lo, ci_hi, cov = ci
    t_pos, t_neg = thresholds(s_med)
    c_pos, c_neg = thresholds(s_med, 0.5)                      # controls: HALF
    p, k, n = sign_p(d)
    cell[c] = dict(
        state=classify(ci_lo, ci_hi, t_pos, t_neg),            # measurand rule
        ctrl_state=classify(ci_lo, ci_hi, c_pos, c_neg),       # control rule
        n=len(d), d=d, D=med(d), ci=(ci_lo, ci_hi), cov=cov, T=t_pos, Tneg=t_neg,
        src=s_med, dst=d_med, p=p, k=k,
        # The acceptance bar: integer-exact, `<= 1.10` PASSES. REPORTED, never used.
        bar="PASS" if 10 * hi <= 11 * lo else "FAIL",
        ratio=hi / lo if lo else 0.0)

# ---- outputs -----------------------------------------------------------------------
with open(sum_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,voided,runs\n")
    for (c, a) in sorted(by):
        v = by[(c, a)]
        f.write("%s,%s,%d,%d,%d,%d,%d,%s\n" % (c, a, med(v), sum(v) // len(v), min(v),
                                               max(v), voided.get((c, a), 0),
                                               " ".join(map(str, v))))

with open(pair_p, "w") as f:
    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,coverage,"
            "T_ms,sign_p,k_pos,state,control_state\n")
    for c in sorted(cell):
        x = cell[c]
        if x["state"] == "INCOMPLETE":
            f.write("%s,%d,,,,,,,,,,,,INCOMPLETE,\n" % (c, x["n"]))
            continue
        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%d,%.4f,%d/%d,%s,%s\n" % (
            c, x["n"], x["src"], x["dst"], x["ratio"], x["bar"], x["D"],
            x["ci"][0], x["ci"][1], x["cov"], round(x["T"]), x["p"], x["k"], x["n"],
            x["state"], x["ctrl_state"]))

with open(ver_p, "w") as f:
    f.write("comparison,kind,lhs_ms,rhs_ms,ratio,bar\n")
    for c in sorted(cell):
        x = cell[c]
        if x["state"] == "INCOMPLETE":
            f.write("%s,invariance,,,,INCOMPLETE\n" % c)
        else:
            f.write("%s,invariance,%d,%d,%.3f,%s\n"
                    % (c, x["src"], x["dst"], x["ratio"], x["bar"]))

# ---- THE SESSION VERDICT -----------------------------------------------------------
incomplete = [c for c in REGISTERED if cell.get(c, {}).get("state") == "INCOMPLETE"]
# A control is clean only at HALF the threshold.
dirty = [c for c in CONTROLS if not incomplete and cell[c]["ctrl_state"] != "NONE"]
m = {c: cell[c]["state"] for c in MEASURANDS if not incomplete}

if incomplete:
    verdict = "INCOMPLETE"
    why = ("cells short of their %d pairs, or with a CI below the registered %.0f%% "
           "coverage: %s. No verdict is read." % (PAIRS, 100 * MIN_COVERAGE,
                                                  ", ".join(incomplete)))
elif SESSION_VOID:
    verdict = "RIG-VOID"
    why = "the harness voided this session: %s. No verdict is read." % SESSION_VOID
elif dirty:
    verdict = "CONTROLS-NOT-CLEAN"
    why = ("control cell(s) are not free of an arm asymmetry at T/2: %s. P1 is claimed "
           "TCP-only and mixed-only; if the gRPC/large controls may be carrying the same "
           "asymmetry, then NEITHER a reproduction NOR a null is readable off this rig. "
           "Re-run at RUNS=16 to buy the power to certify them."
           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d], T/2=%d)"
                       % (c, cell[c]["ctrl_state"], cell[c]["D"], cell[c]["ci"][0],
                          cell[c]["ci"][1], round(cell[c]["T"] / 2))
                       for c in dirty))
elif "EFFECT" in m.values() and "INVERTED" in m.values():
    verdict = "MIXED"
    why = ("one direction shows the effect and the other INVERTS it -- a host x role "
           "interaction this rig cannot decompose. Inconclusive for the question.")
elif "EFFECT" in m.values():
    verdict = "REPRODUCES"
    why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: it shows "
           "P1 CAN occur macOS<->macOS, so it is not waivable as 'Windows residue'. It "
           "does NOT establish a platform-general cost, does NOT name the mechanism, "
           "does NOT kill H1 (the code H1 accuses runs here too), and leaves macOS/APFS "
           "and host x role explanations OPEN."
           % ", ".join(c for c, s in m.items() if s == "EFFECT"))
elif "INVERTED" in m.values():
    verdict = "INVERTED"
    why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank it as "
           "'P1 absent'." % ", ".join(c for c, s in m.items() if s == "INVERTED"))
elif all(s == "NONE" for s in m.values()):
    verdict = "DOES-NOT-REPRODUCE"
    why = ("both TCP-mixed cells EXCLUDE an effect of size T, and every control is clean "
           "at T/2 -- a genuine equivalence result. Scoped to THIS pair: P1 did not "
           "reproduce macOS<->macOS. That is CONSISTENT with 'the Windows peer is "
           "required' but does NOT prove it -- it could equally be a property of these "
           "two machines, their disks, or this macOS version.")
else:
    verdict = "UNCLEAR"
    why = ("the CI spans the threshold in: %s. The rig could not resolve an effect of "
           "size T either way -- this is NOT 'P1 vanishes'. Re-run at RUNS=16."
           % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))

out = ["SESSION VERDICT: %s" % verdict, "", why, "",
       "Per cell (T = min(srcinit_median/10, %d ms); controls must be NONE at T/2):" % DELTA_REF]
for c in sorted(cell):
    x = cell[c]
    if x["state"] == "INCOMPLETE":
        out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
        continue
    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] (%.1f%%) T=%3dms  "
               "ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
               % (c, x["state"], x["ctrl_state"], x["D"], x["ci"][0], x["ci"][1],
                  100 * x["cov"], round(x["T"]), x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
# A cell can be NONE (an effect of size T is excluded) and STILL carry a real, consistent
# effect BELOW T -- e.g. 99 ms on a 1000 ms arm, one millisecond under the threshold, on
# 7 of 8 pairs. That is not a contradiction and it does not change the verdict, but it
# must not hide inside the word "none". Reported, never decided on.
subthreshold = [c for c in sorted(cell)
                if cell[c]["state"] == "NONE" and cell[c]["p"] < 0.05 and cell[c]["D"]]
if subthreshold:
    out += ["",
            "NOTE -- a real but SUB-THRESHOLD effect is present in: %s."
            % ", ".join("%s (D=%+dms, T=%dms, sign_p=%.3f)"
                        % (c, cell[c]["D"], round(cell[c]["T"]), cell[c]["p"])
                        for c in subthreshold),
            "These cells are consistent in direction but smaller than the registered",
            "threshold, so they are not a reproduction of P1. They are NOT nothing."]

out += ["",
        "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
        "and take NO part in this verdict, which is decided only by the paired CI against",
        "T. sign_p is reported, not decided on. All runs are in summary.csv -- read them.",
        "",
        "Computed from the pre-registered rule. It declares nothing beyond it."]

open(sess_p, "w").write("\n".join(out) + "\n")
print("\n".join(out))
