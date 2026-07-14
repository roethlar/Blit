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
    CI  = exact distribution-free order-statistic interval on the population median, the
          narrowest whose coverage is >= 95%. AT THE REGISTERED n=8 THAT IS [min(d), max(d)]
          (99.22%) -- i.e. it CANNOT trim. No bootstrap, no approximation.
    RANGE = [min(d), max(d)], and a NULL is judged on the RANGE, never on a trimmed CI.

    n IS EXACTLY 8. Not "at least": at any larger n the >=95% interval starts TRIMMING
    outliers, and a bimodal arm then yields a narrow median CI and a FALSE verdict. grok
    drove exactly that with a 16-pair CSV (3 pairs at -500 trimmed away, 13 at +200 left)
    -> REPRODUCES. The cell must carry EXACTLY the registered pair count or it is INCOMPLETE.

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
REGISTERED_PAIRS = (8,)
MIN_COVERAGE = 0.95

_env = os.environ.get("DELTA_REF_MS")
if _env is not None and _env.strip() != str(DELTA_REF):
    sys.exit("REFUSING: DELTA_REF_MS=%r but the registered reference effect is %d ms. "
             "The rule is not tunable from the environment.\n" % (_env, DELTA_REF))


def cells_env(name):
    return [c for c in os.environ.get(name, "").split(",") if c]


REGISTERED_MEASURANDS = ("nq_tcp_mixed", "qn_tcp_mixed")
REGISTERED_CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")

MEASURANDS = list(REGISTERED_MEASURANDS)
CONTROLS = list(REGISTERED_CONTROLS)
REGISTERED = MEASURANDS + CONTROLS
for _name, _want in (("VERDICT_CELLS", MEASURANDS), ("CONTROL_CELLS", CONTROLS),
                     ("REGISTERED_CELLS", REGISTERED)):
    _got = cells_env(_name)
    if _got and _got != _want:
        sys.exit("REFUSING: %s=%s but the registered set is %s. Which cells are CONTROLS is "
                 "part of the pre-registration -- omitting one is how a dirty control gets "
                 "dropped and the session grades anyway.\n" % (_name, _got, _want))
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
        v = int(r["ms"])
        if v <= 0:
            # A transfer cannot take zero time. With src_median = 0 the thresholds collapse
            # to 0 and classify(0,0,0,0,0,0) returns EFFECT -- a session of zeros would
            # report a REPRODUCTION (codex r10).
            raise ValueError("non-positive")
        return v
    except (TypeError, ValueError):
        sys.stderr.write("CORRUPT ROW: cell=%s arm=%s run=%s ms=%r (must be a POSITIVE "
                         "integer). A benchmark whose rows do not parse has no verdict.\n"
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


def classify(ci_lo, ci_hi, rng_lo, rng_hi, t_pos, t_neg):
    """THE RULE. Four states, mutually exclusive and exhaustive BY CONSTRUCTION.

    EFFECT/INVERTED use the >=95% CI on the median; NONE uses the FULL RANGE. At the
    registered n=8 these coincide (the CI IS the range), so nothing can be trimmed either
    way -- the distinction is the SEMANTICS that keeps the rule sound if a larger n is ever
    registered, and the engine REFUSES any n but 8.

    NONE uses the FULL RANGE -- EVERY pair must lie inside +-T. Round 8 (codex, BLOCKER):
    a >=95% CI at n>8 TRIMS outliers, so a BIMODAL arm produces a NARROW median CI and a
    FALSE NULL (driven: CI = [1,1] from modes at +-110). An equivalence claim must never be
    reachable by trimming away the very pairs that contradict it. This is also why
    bimodality needs no special branch: it cannot hide from the range.
    """
    if ci_lo >= t_pos:
        return "EFFECT"
    if ci_hi <= t_neg:
        return "INVERTED"
    if t_neg < rng_lo and rng_hi < t_pos:
        return "NONE"
    return "UNCLEAR"


# ---- pass 1: measure every cell -----------------------------------------------------
cell = {}
for c in sorted(set(REGISTERED) | set(meta)):
    d = paired(c)
    ci = median_ci(d) if d else None
    # COMPLETE is checked against the DATA, never against meta's say-so: a one-pair CSV
    # with a lying meta once graded as a full cell and emitted a null at 0% coverage.
    if (meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None
            or len(by.get((c, "srcinit"), [])) != PAIRS
            or len(by.get((c, "destinit"), [])) != PAIRS):
        # EVERY arm must carry exactly the registered count too, not just the paired slots:
        # a duplicate or unpaired valid row would sit in the arm's list and skew its MEDIAN
        # (and therefore T, B and the bar) while the pair count still looked right.
        cell[c] = dict(state="INCOMPLETE", n=len(d))
        continue
    s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
    hi, lo = max(s_med, d_med), min(s_med, d_med)
    ci_lo, ci_hi, cov = ci
    p, k, n = sign_p(d)
    cell[c] = dict(n=len(d), d=d, D=med(d), ci=(ci_lo, ci_hi), rng=(min(d), max(d)),
                   cov=cov, src=s_med, dst=d_med, p=p, k=k,
                   # The acceptance bar: integer-exact, `<= 1.10` PASSES. REPORTED, never used.
                   bar="PASS" if 10 * hi <= 11 * lo else "FAIL",
                   ratio=hi / lo if lo else 0.0)

# ---- pass 2: the controls certify the rig, and BOUND its residual bias ---------------
# A control certifies clean at T/2 -- but "clean" is not "zero". A control sitting at +49
# with T/2 = 50 is accepted, and THAT 49 ms OF ARM BIAS MAY BE RIDING IN THE MEASURAND
# TOO, so a measurand "EFFECT" at exactly T could be half real and half rig (round-8
# codex, BLOCKER). The bias the controls FAIL TO EXCLUDE is therefore carried into the
# measurand's thresholds:
#
#     B = the arm asymmetry the controls could not rule out, as a FRACTION OF THE ARM,
#         scaled to the cell it is applied to. Taken from each control's full RANGE (not its
#         CI: the CI is an interval for the MEDIAN and it TRIMS, and a bound on what the rig
#         might be carrying must never be computed by trimming). Relative, not raw ms:
#         the controls run different fixtures at different speeds.
#     an EFFECT must clear  T + B     (bias could be INFLATING it)
#     a NULL   must fit in  T - B     (bias could be MASKING an effect)
#
# If the controls are genuinely clean, B is a few ms and this barely moves. If they are
# marginal, it bites -- which is the point.
dirty = []
B_frac = 0.0          # RELATIVE, not raw milliseconds
for c in CONTROLS:
    x = cell.get(c, {})
    if x.get("state") == "INCOMPLETE":
        continue
    c_pos, c_neg = thresholds(x["src"], 0.5)
    x["ctrl_state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], c_pos, c_neg)
    x["ctrl_T"] = c_pos
    if x["ctrl_state"] != "NONE":
        dirty.append(c)
    elif x["src"]:
        # B IS A FRACTION OF THE ARM, NOT A MILLISECOND COUNT (round-9 codex, BLOCKER).
        # The controls run on DIFFERENT fixtures and therefore different arm speeds: the
        # same 4.9% arm bias is 122 ms on a 2500 ms large-file control and 24 ms on a fast
        # one. Carrying raw ms across them OVER-penalises a measurand slower than the
        # control and UNDER-penalises one that is faster -- and the second direction is the
        # dangerous one: a 4.9% bias measured on a fast control would license a measurand
        # effect that is mostly rig. Take the bias as a FRACTION and scale it to whatever
        # arm it is being applied to.
        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])

# ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
for c in MEASURANDS:
    x = cell.get(c, {})
    if x.get("state") == "INCOMPLETE":
        continue
    t_pos, t_neg = thresholds(x["src"])
    B = B_frac * x["src"]                    # the control bias, on THIS cell's arm
    x["T"] = t_pos
    x["B"] = B
    x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1],
                          t_pos + B, t_neg - B)          # an EFFECT must clear T + B
    if x["state"] == "NONE":
        # ...and a NULL must survive the TIGHTER bound: bias could be masking an effect.
        if not (t_neg + B < x["rng"][0] and x["rng"][1] < t_pos - B):
            x["state"] = "UNCLEAR"

# Controls also carry a state for the report; measurands carry a ctrl_state for symmetry.
for c in cell:
    x = cell[c]
    if x.get("state") == "INCOMPLETE":
        continue
    if "state" not in x:                                  # a control: report its own state
        t_pos, t_neg = thresholds(x["src"])
        x["T"] = t_pos
        x["B"] = 0.0
        x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], t_pos, t_neg)
    x.setdefault("ctrl_state", "-")

# ---- outputs -----------------------------------------------------------------------
with open(sum_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,voided,runs\n")
    for (c, a) in sorted(by):
        v = by[(c, a)]
        f.write("%s,%s,%d,%d,%d,%d,%d,%s\n" % (c, a, med(v), sum(v) // len(v), min(v),
                                               max(v), voided.get((c, a), 0),
                                               " ".join(map(str, v))))

with open(pair_p, "w") as f:
    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,range_lo,range_hi,"
            "coverage,T_ms,B_ms,sign_p,k_pos,state,control_state\n")
    for c in sorted(cell):
        x = cell[c]
        if x["state"] == "INCOMPLETE":
            f.write("%s,%d,,,,,,,,,,,,,,,INCOMPLETE,\n" % (c, x["n"]))
            continue
        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%d,%d,%.4f,%d,%d,%.4f,%d/%d,%s,%s\n" % (
            c, x["n"], x["src"], x["dst"], x["ratio"], x["bar"], x["D"],
            x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], x["cov"],
            round(x["T"]), round(x.get("B", 0)), x["p"], x["k"], x["n"],
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
           "There is no escalation: a noisy rig is fixed by a QUIETER RIG, not more pairs."
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
           "size T either way -- this is NOT 'P1 vanishes'. Fix the rig, do not add pairs."
           % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))

out = ["SESSION VERDICT: %s" % verdict, "", why, "",
       "Per cell. T = min(srcinit_median/10, %d ms). Controls must be NONE at T/2, and B is"
       % DELTA_REF,
       "the arm bias they could NOT exclude: an EFFECT must clear T+B, a NULL must fit in T-B."]
for c in sorted(cell):
    x = cell[c]
    if x["state"] == "INCOMPLETE":
        out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
        continue
    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] range=[%+5d,%+5d] "
               "T=%3dms B=%3dms  ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
               % (c, x["state"], x["ctrl_state"], x["D"], x["ci"][0], x["ci"][1],
                  x["rng"][0], x["rng"][1], round(x["T"]), round(x.get("B", 0)),
                  x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
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
        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
        "uses the CI. That is why bimodality needs no special branch: it cannot hide from",
        "the range.",
        "",
        "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
        "and take NO part in this verdict, which is decided only by the paired CI against",
        "T. sign_p is reported, not decided on. All runs are in summary.csv -- read them.",
        "",
        "Computed from the pre-registered rule. It declares nothing beyond it."]

open(sess_p, "w").write("\n".join(out) + "\n")
print("\n".join(out))
