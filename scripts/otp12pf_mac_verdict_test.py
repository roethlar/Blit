#!/usr/bin/env python3
"""Guard test for otp12pf_mac_verdict.py — run it before trusting a Mac<->Mac run.

    python3 scripts/otp12pf_mac_verdict_test.py

Every case below is a DEFECT A REVIEWER ACTUALLY FOUND in a previous revision of
this engine, encoded so it cannot come back. Each is mutation-proven: reverting the
fix in the engine makes exactly that case fail (see MUTATIONS at the bottom, run
with `--mutations`).

    VERDICT_PY=<path> python3 scripts/otp12pf_mac_verdict_test.py   # test a copy

The headline defects:

  * codex r1 — a RANGE noise rule let a real 190 ms effect on 7/8 pairs report
    "VANISHES" (83% of rig W's Delta_P1).
  * codex r2 + grok — the equivalence margin was tied to the BAR, which on a slow
    arm is WIDER than the effect being excluded: all eight d_i = 230 on a 2500 ms
    arm still reported "VANISHES".
  * codex r2 — the negative margin used -0.10*src, but the bar is symmetric in
    RATIO, so the inverting bound is -src/11: a CI of [-190,0] on src=2000 reported
    "VANISHES" though -190 IS an inversion ratio of 1.105.
  * codex r2 — the sign test was computed and never read: 7/8 positive pairs could
    report REPRODUCES while the registered test said p = .0703 (not significant).
  * grok (REPRODUCED LIVE) — RIG-VOID failed open: a control with bar FAIL whose CI
    crossed zero escaped the void, and a session emitted VANISHES with its gRPC
    controls sitting at ratio 1.200 / bar FAIL.
  * codex r2 — a partial cell set was FILTERED, not marked INCOMPLETE, so a one-cell
    run could emit VANISHES while claiming "both" cells vanished.
  * grok — an exact 1.10 ratio could never REPRODUCE, because the bar is `<=1.10
    PASSES` and REPRODUCES demanded a bar FAIL: a precise 10% effect was
    unreportable by construction.
"""
import csv, os, random, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_VERDICT = os.path.join(HERE, "otp12pf_mac_verdict.py")


def engine():
    """Resolved per call, never cached: the mutation harness repoints it at runtime,
    and a cached path would silently test the UNMUTATED engine and report a kill it
    never made."""
    return os.environ.get("VERDICT_PY", DEFAULT_VERDICT)
CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")
MEASURANDS = ("nq_tcp_mixed", "qn_tcp_mixed")
REGISTERED = MEASURANDS + CONTROLS
OUTCOMES = {"REPRODUCES", "INVERSION", "PARTIAL", "VANISHES", "UNDERPOWERED",
            "BAR-FAIL-INCONSISTENT", "UNSTABLE", "INCOMPLETE", "MIXED-SIGN",
            "RIG-VOID", "INCONCLUSIVE", "INCONCLUSIVE-UNDERPOWERED"}


def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=()):
    """Run the engine on a synthetic session and return its SESSION VERDICT."""
    control_d = [5] * 8 if control_d is None else control_d
    tmp = tempfile.mkdtemp()
    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
    present = [c for c in REGISTERED if c not in drop_cells]
    with open(runs, "w") as f:
        w = csv.writer(f)
        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid".split(","))
        for cell in present:
            d, s = (measurand_d, src) if cell in MEASURANDS else (control_d, control_src)
            for i, di in enumerate(d, 1):
                w.writerow([cell, "srcinit", "x", "h", i, s, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
                w.writerow([cell, "destinit", "x", "h", i, s + di, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
    with open(meta, "w") as f:
        f.write("cell,pairs_attempted,complete\n")
        for cell in present:
            f.write("%s,8,yes\n" % cell)
    env = dict(os.environ, DELTA_REF_MS="230",
               VERDICT_CELLS=",".join(MEASURANDS),
               CONTROL_CELLS=",".join(CONTROLS),
               REGISTERED_CELLS=",".join(REGISTERED))
    out = subprocess.run([sys.executable, engine(), runs, meta,
                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
                         env=env, capture_output=True, text=True)
    if out.returncode != 0:
        return "ENGINE-CRASH: " + out.stderr.strip().splitlines()[-1]
    return out.stdout.splitlines()[0].split(":", 1)[1].strip()


# (name, kwargs, must_be, must_not_be)
CASES = [
    ("codex r1: real 190ms effect on 7/8 pairs (83% of rig W's Delta_P1)",
     dict(measurand_d=[0, 180, 180, 190, 190, 200, 200, 200], src=2000),
     None, "VANISHES"),

    ("codex r2: a rig-W-sized effect (230ms) in EVERY pair, on a slow 2500ms arm",
     dict(measurand_d=[230] * 8, src=2500),
     "PARTIAL", "VANISHES"),

    # THE MARGIN'S OWN CASE. No consistent effect (so the effect-detection branch
    # does NOT fire), but the CI still reaches +240 -- ABOVE rig W's 230 ms and
    # BELOW the bar's 250 ms on this slow arm. A margin tied to the bar calls this
    # equivalence; the true margin, min(bar_breach, DELTA_REF), cannot exclude a
    # rig-W-sized effect and must say so.
    ("codex r2: the margin is min(bar_breach, DELTA_REF) -- the bar alone forgives 240ms",
     dict(measurand_d=[-100, -50, 0, 50, 100, 200, 220, 240], src=2500),
     "INCONCLUSIVE-UNDERPOWERED", "VANISHES"),

    # The medians must PASS the bar here, or the cell never reaches the margin logic
    # and the case would not exercise the bound it claims to guard.
    ("codex r2: the negative bound is -src/11, not -0.10*src (CI [-190,0] @ 2000)",
     dict(measurand_d=[-190, -190, 0, 0, 0, 0, 0, 0], src=2000),
     "INCONCLUSIVE-UNDERPOWERED", "VANISHES"),

    ("codex r2: the sign test must PARTICIPATE (7 pos + 1 neg -> p=.0703, n.s.)",
     dict(measurand_d=[-20, 300, 310, 320, 330, 340, 350, 360], src=1000),
     "BAR-FAIL-INCONSISTENT", "REPRODUCES"),

    ("grok (reproduced live): a bar-FAIL control whose CI crosses zero must VOID",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[-100, -50, 300, 320, 340, 350, 360, 380], control_src=1000),
     "RIG-VOID", "VANISHES"),

    ("codex r2: a missing registered cell is INCOMPLETE, never filtered away",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          drop_cells=("qn_tcp_mixed",)),
     "INCOMPLETE", "VANISHES"),

    ("grok: an EXACT 1.10 ratio must be reportable (it was unreachable)",
     dict(measurand_d=[100] * 8, src=1000),
     "REPRODUCES", None),

    ("a genuinely absent effect is a real EQUIVALENCE result",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000),
     "VANISHES", None),

    ("a bar-breaking, consistent slowdown reproduces",
     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000),
     "REPRODUCES", None),

    ("source-initiated is the slow arm -> INVERSION, never 'P1 absent'",
     dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
     "INVERSION", None),

    ("a null the rig could not have SEEN is UNDERPOWERED, not VANISHES",
     dict(measurand_d=[-400, -300, -100, 0, 0, 100, 300, 400], src=2000),
     "INCONCLUSIVE-UNDERPOWERED", "VANISHES"),
]


def run_cases():
    failures = []
    for name, kw, must_be, must_not_be in CASES:
        got = session(**kw)
        ok = not (must_be and got != must_be) and not (must_not_be and got == must_not_be)
        print("%-64s -> %-26s %s" % (name[:64], got, "ok" if ok else "*** FAIL ***"))
        if not ok:
            failures.append(name)
            if must_be:
                print("      expected %s" % must_be)
            if must_not_be:
                print("      must NOT be %s" % must_not_be)
    return failures


def fuzz(n=300):
    """The taxonomy must be EXHAUSTIVE: no input may land outside the registered set."""
    rng = random.Random(4242)
    bad = 0
    for _ in range(n):
        d = [rng.randint(-600, 600) for _ in range(8)]
        src = rng.choice([600, 1000, 2000, 2500, 5000])
        got = session(measurand_d=d, src=src)
        if got not in OUTCOMES:
            print("*** UNREGISTERED OUTCOME %r for d=%s src=%d" % (got, d, src))
            bad += 1
    print("fuzz: %d/%d inputs produced a registered outcome" % (n - bad, n))
    return bad


# --- MUTATION PROOF -----------------------------------------------------------
# A test that passes with its fix reverted is vacuous. Each mutation below REVERTS
# one fix in a COPY of the engine; the named case must then produce the forbidden
# verdict. `python3 scripts/otp12pf_mac_verdict_test.py --mutations`
#
# NOTE on the CI/sign-test pair: the exact order-statistic CI and the sign test are
# mathematical DUALS -- at the >=95% level, "CI_lo > 0" and "the sign test rejects"
# are the same statement. Either one alone blocks the 7/8 defect, so neither
# mutation alone flips the verdict. The COMBINED mutation (bootstrap CI *and* no
# sign test) is what proves the pair is load-bearing, and it is listed as such.
MUTATIONS = [
    ("equivalence margin tied to the BAR alone (codex r2 + grok)",
     ["        margin_hi = min(breach_hi, float(DELTA_REF))\n"
      "        margin_lo = max(breach_lo, -float(DELTA_REF))",
      "        margin_hi = breach_hi\n"
      "        margin_lo = breach_lo"],
     "the bar alone forgives 240ms", "VANISHES"),

    # The OLD engine tested equivalence BEFORE it tested for an effect, so a cell
    # with a real effect that the bar forgave was called VANISHES rather than
    # PARTIAL. Restoring that order must resurrect codex's counterexample.
    ("equivalence tested BEFORE effect detection, with the bar-tied margin (codex r2)",
     ['        if pos_effect and material:\n'
      '            out = "REPRODUCES"\n'
      '        elif neg_effect and material_neg:\n'
      '            out = "INVERSION"\n'
      '        elif pos_effect or neg_effect:\n'
      '            out = "PARTIAL"',
      '        if bar == "PASS" and ci_lo > -breach_hi and ci_hi < breach_hi:\n'
      '            out = "VANISHES"\n'
      '        elif pos_effect and material:\n'
      '            out = "REPRODUCES"\n'
      '        elif neg_effect and material_neg:\n'
      '            out = "INVERSION"\n'
      '        elif pos_effect or neg_effect:\n'
      '            out = "PARTIAL"'],
     "rig-W-sized effect", "VANISHES"),

    ("negative margin uses -0.10*src instead of -src/11 (codex r2)",
     ["        breach_lo = -s_med / 11.0", "        breach_lo = -s_med / 10.0"],
     "negative bound", "VANISHES"),

    ("RIG-VOID ignores the bar -> fails open (grok, reproduced live)",
     ['ctrl_void = [c for c in ctrl\n'
      '             if cell_detail.get(c, {}).get("bar") == "FAIL"\n'
      '             or cell_outcome[c] in ("UNSTABLE", "REPRODUCES", "INVERSION",\n'
      '                                    "BAR-FAIL-INCONSISTENT")]',
      'ctrl_void = [c for c in ctrl\n'
      '             if cell_outcome[c] in ("UNSTABLE", "REPRODUCES", "INVERSION")]'],
     "bar-FAIL control", "VANISHES"),

    # The fix is BOTH halves: the cell loop must walk the REGISTERED set (not merely
    # what turned up in the CSV), and absent cells must be marked INCOMPLETE rather
    # than filtered. Reverting only one leaves the other still catching it, so the
    # faithful revert of the old fail-open is both.
    ("a missing registered cell is filtered away, not INCOMPLETE (codex r2)",
     ["all_cells = sorted(set(REGISTERED_CELLS) | set(meta))",
      "all_cells = sorted(meta)",
      "missing = [c for c in REGISTERED_CELLS if c not in cell_outcome]",
      "missing = []"],
     "missing registered cell", "VANISHES"),

    ("materiality requires a bar FAIL, so exact 1.10 is unreachable (grok)",
     ['        material = (bar == "FAIL") or (ci_lo >= breach_hi)',
      '        material = (bar == "FAIL")'],
     "EXACT 1.10", "PARTIAL"),

    ("COMBINED: bootstrap-style CI [d2,d7] *and* no sign test (codex r2)",
     ["    best = None\n"
      "    for k in range(1, n // 2 + 1):",
      "    best = (d[1], d[n - 2], 0.9297) if n >= 4 else None\n"
      "    for k in range(1, 1):",
      "    pos_effect = ci_lo > 0 and p < 0.05",
      "    pos_effect = ci_lo > 0"],
     "sign test must PARTICIPATE", "REPRODUCES"),
]


def mutate():
    src = open(os.path.join(HERE, "otp12pf_mac_verdict.py")).read()
    bad = 0
    for name, subs, case_key, forbidden in MUTATIONS:
        body = src
        for i in range(0, len(subs), 2):
            old, new = subs[i], subs[i + 1]
            # A mutation whose target text no longer exists is a SILENT PASS -- the
            # engine drifted and the proof is stale. Fail loudly.
            if old not in body:
                print("*** STALE MUTATION (target text not found): %s" % name)
                bad += 1
                body = None
                break
            body = body.replace(old, new, 1)
        if body is None:
            continue
        tmp = tempfile.mkdtemp()
        path = os.path.join(tmp, "mutant.py")
        open(path, "w").write(body)
        case = next(c for c in CASES if case_key in c[0])
        os.environ["VERDICT_PY"] = path
        got = session(**case[1])
        del os.environ["VERDICT_PY"]
        killed = (got == forbidden)
        print("%-62s -> %-22s %s" % (name[:62], got,
                                     "KILLED (guard is real)" if killed
                                     else "*** SURVIVED — GUARD IS VACUOUS ***"))
        if not killed:
            bad += 1
    return bad


if __name__ == "__main__":
    if "--mutations" in sys.argv:
        print("Reverting each fix in a copy of the engine; the guard must then FAIL.\n")
        nbad = mutate()
        print()
        print("%d/%d mutations killed" % (len(MUTATIONS) - nbad, len(MUTATIONS)))
        sys.exit(1 if nbad else 0)
    fails = run_cases()
    print()
    bad = fuzz()
    print()
    print("%d/%d cases passed" % (len(CASES) - len(fails), len(CASES)))
    sys.exit(1 if (fails or bad) else 0)
