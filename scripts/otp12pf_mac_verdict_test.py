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


def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=(),
            per_cell=None, void_reason="", pairs=8, env_extra=None):
    """Run the engine on a synthetic session and return its SESSION VERDICT.

    `src` (and `control_src`) may be an INT (a constant source arm) or a LIST giving
    the srcinit of each pair. The constant-only helper could not express the case
    that matters most -- the bar is computed on the MARGINAL medians while the CI is
    computed on the PAIRED differences, and those two can disagree in DIRECTION only
    when the source arm varies (round-5 codex, HIGH: "the helper hardcodes constant
    source times", which is why the n=16 blocker was unguardable).

    `per_cell` overrides individual cells with (d, src) so the two measurand
    directions can differ; `pairs` drives the n=16 escalation arm.
    """
    control_d = [5] * pairs if control_d is None else control_d
    per_cell = per_cell or {}
    tmp = tempfile.mkdtemp()
    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
    present = [c for c in REGISTERED if c not in drop_cells]
    with open(runs, "w") as f:
        w = csv.writer(f)
        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid".split(","))
        for cell in present:
            if cell in per_cell:
                d, s = per_cell[cell]
            elif cell in MEASURANDS:
                d, s = measurand_d, src
            else:
                d, s = control_d, control_src
            srcs = s if isinstance(s, list) else [s] * len(d)
            for i, (di, si) in enumerate(zip(d, srcs), 1):
                w.writerow([cell, "srcinit", "x", "h", i, si, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
                w.writerow([cell, "destinit", "x", "h", i, si + di, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
    with open(meta, "w") as f:
        f.write("cell,pairs_attempted,complete\n")
        for cell in present:
            # `complete=yes` is asserted even when the cell is SHORT: the engine must
            # not believe it (grok drove VANISHES out of a 1-pair session at 0% CI
            # coverage because the engine trusted this column).
            f.write("%s,%d,yes\n" % (cell, pairs))
    env = dict(os.environ,
               VERDICT_CELLS=",".join(MEASURANDS),
               CONTROL_CELLS=",".join(CONTROLS),
               REGISTERED_CELLS=",".join(REGISTERED),
               REQUIRED_PAIRS=str(pairs), SESSION_VOID_REASON=void_reason)
    env.pop("DELTA_REF_MS", None)          # it is PINNED in the engine now
    env.update(env_extra or {})
    out = subprocess.run([sys.executable, engine(), runs, meta,
                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
                         env=env, capture_output=True, text=True)
    if out.returncode == 2:
        # A DELIBERATE refusal (a pinned constant was tampered with, a row is corrupt).
        # Distinct from a crash: refusing is the engine working, not failing.
        return "ENGINE-REFUSED"
    if out.returncode != 0:
        return "ENGINE-CRASH: " + (out.stderr.strip().splitlines() or ["?"])[-1]
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

    # --- ROUND 3 (grok) -------------------------------------------------------
    # THE BLOCKER, reproduced live: the round-2 RIG-VOID fix was only half of the
    # hole. A control carrying a real, 8/8, rig-W-sized effect on a SLOW arm is
    # bar-PASS (ratio 1.092) and lands as PARTIAL -- and PARTIAL did not void. The
    # session printed a clean VANISHES while EVERY control showed the exact effect
    # size the power gate is built around.
    ("grok r3 (reproduced): a Delta_ref-sized control effect must VOID the rig",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[230] * 8, control_src=2500),
     "RIG-VOID", "VANISHES"),

    # ...but a TINY consistent control asymmetry is host x role (q is the faster
    # Mac), is excluded as smaller than the margin, and must NOT void the rig --
    # or every session dies.
    ("grok r3: a margin-excluded control asymmetry must NOT void the rig",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[5] * 8, control_src=1000),
     "VANISHES", "RIG-VOID"),

    ("grok r3 (reproduced): n=1 with complete=yes must not grade at 0% coverage",
     dict(measurand_d=[0], src=2000, control_d=[5], control_src=1000),
     "INCOMPLETE", "VANISHES"),

    ("grok r3 (reproduced): a clean one-direction REPRODUCES must not be masked",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
                    "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
     "REPRODUCES", "BAR-FAIL-INCONSISTENT"),

    ("grok r3: a harness-detected session void (end-load) refuses a verdict",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          void_reason="end-load on q is 9.1 (> 3.0)"),
     "RIG-VOID", "VANISHES"),

    # --- ROUND 5 -------------------------------------------------------------
    # codex BLOCKER: `bar == FAIL` carried no DIRECTION, so a bar failure in the
    # INVERSE direction made a positive effect "material". Thirteen +1ms pairs and
    # three pairs that fall below the whole distribution => the marginal medians fail
    # the bar the OTHER way (1200 vs 1001) while every surviving pair is +1ms.
    # The old rule called that REPRODUCES -- P1 reproducing off ONE MILLISECOND.
    ("codex r5: a bar FAIL in the INVERSE direction cannot make +1ms 'material'",
     dict(measurand_d=[1] * 13 + [-4500] * 3,
          src=[1000] * 7 + [1200] * 6 + [5000] * 3,
          control_d=[5] * 16, control_src=1000, pairs=16),
     "PARTIAL", "REPRODUCES"),

    # codex BLOCKER: the SAME materiality bug, third branch. ONE zero pair drags
    # ci_lo to 0, which demotes a control carrying the full rig-W effect from PARTIAL
    # to UNDERPOWERED -- and UNDERPOWERED was not on the void list. Session printed a
    # clean VANISHES with every control at D=+230.
    ("codex r5 (reproduced): an UNDERPOWERED control carrying D=+230 blocks the null",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[0] + [230] * 7, control_src=2500),
     "INCONCLUSIVE-UNDERPOWERED", "VANISHES"),

    # grok, same class, different shape: one NEGATIVE pair kills the sign test, so the
    # control is not even directional -- and it still must not support a null.
    ("grok r5 (reproduced): a non-directional but UNCERTIFIED control blocks the null",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[230] * 7 + [-10], control_src=2500),
     "INCONCLUSIVE-UNDERPOWERED", "VANISHES"),

    # grok BLOCKER: the zero-boundary null. A single zero pair vetoed `ci_lo > 0`, so
    # a 99ms effect on 7 of 8 pairs -- ONE MILLISECOND under the bar -- was "no effect"
    # AND null_excl (99 < 100), and reported VANISHES while the sign test REJECTED.
    # Direction is the sign test's job; the CI must not be able to veto it.
    ("grok r5 (reproduced): 7/8 pairs at 99ms is NOT equivalence, whatever the CI says",
     dict(measurand_d=[0] + [99] * 7, src=1000),
     "PARTIAL", "VANISHES"),

    # codex BLOCKER: the registered constants were env-tunable, so the operator could
    # retune the decision rule after seeing the data. The engine must REFUSE.
    ("codex r5: DELTA_REF_MS is PINNED -- the rule is not tunable from the environment",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[230] * 8, control_src=2500,
          env_extra={"DELTA_REF_MS": "240"}),
     "ENGINE-REFUSED", "VANISHES"),
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
    """The taxonomy must be EXHAUSTIVE: no input may land outside the registered set.

    The CONTROLS are fuzzed too. Round-3 grok: the old fuzz perturbed only the
    measurand and pinned the controls at [5]*8, so every dirty-control path -- the
    one that hid the BLOCKER -- went unexercised.
    """
    rng = random.Random(4242)
    bad = 0
    for _ in range(n):
        d = [rng.randint(-600, 600) for _ in range(8)]
        cd = [rng.randint(-300, 300) for _ in range(8)]
        src = rng.choice([600, 1000, 2000, 2500, 5000])
        csrc = rng.choice([600, 1000, 2500, 5000])
        got = session(measurand_d=d, src=src, control_d=cd, control_src=csrc)
        if got not in OUTCOMES:
            print("*** UNREGISTERED OUTCOME %r for d=%s src=%d ctrl=%s" % (got, d, src, cd))
            bad += 1
    print("fuzz: %d/%d inputs produced a registered outcome (measurand AND controls fuzzed)"
          % (n - bad, n))
    return bad


# --- MUTATION PROOF -----------------------------------------------------------
# A test that passes with its fix reverted is vacuous. Each mutation below REVERTS
# one fix in a COPY of the engine; the named case must then produce the forbidden
# verdict. `python3 scripts/otp12pf_mac_verdict_test.py --mutations`
#
# NOTE on the CI and the sign test. Rev 4 called them "mathematical duals". THAT WAS
# WRONG once a zero difference exists (round-5): the sign test DROPS zeros, so
# d = [0, 99x7] rejects at p = .0156 while the CI's lower bound is exactly 0. Rev 6
# therefore gives each the job it can actually do -- the SIGN TEST establishes
# DIRECTION, the CI establishes MAGNITUDE -- and the old conjunction (`ci_lo > 0 and
# p < .05`) is gone, because it silently required the CI to answer a question about
# direction that a single zero pair could veto. That veto is exactly how a 99 ms
# effect on 7 of 8 pairs came to be reported as VANISHES.
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
     ['        if dir_pos and material:\n'
      '            out = "REPRODUCES"',
      '        if bar == "PASS" and ci_lo > -breach_hi and ci_hi < breach_hi:\n'
      '            out = "VANISHES"\n'
      '        elif dir_pos and material:\n'
      '            out = "REPRODUCES"'],
     "rig-W-sized effect", "VANISHES"),

    ("negative margin uses -0.10*src instead of -src/11 (codex r2)",
     ["        breach_lo = -s_med / 11.0", "        breach_lo = -s_med / 10.0"],
     "negative bound", "VANISHES"),

    # The round-2 fail-open, faithfully: the void ignored the BAR entirely.
    ("RIG-VOID ignores the bar -> fails open (grok r2, reproduced live)",
     ['    if dt.get("bar") == "FAIL" or cell_outcome[c] == "UNSTABLE":\n'
      '        return True',
      '    if cell_outcome[c] == "UNSTABLE":\n'
      '        return True'],
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
     ['        material = bar_fail_pos or (ci_lo >= breach_hi)',
      '        material = bar_fail_pos'],
     "EXACT 1.10", "PARTIAL"),

    # The sign test no longer PARTICIPATES: direction is asserted regardless of it.
    # (Rev 6 moved direction OUT of the CI and INTO the sign test, so the faithful
    # revert is to stop consulting the sign test at all.) 7 positive + 1 negative
    # then reports a clean REPRODUCES at p = .0703.
    ("the sign test does not participate: direction asserted without it (codex r2)",
     ["        directional = p < 0.05",
      "        directional = True"],
     "sign test must PARTICIPATE", "REPRODUCES"),

    # --- ROUND 3 (grok) -------------------------------------------------------
    ("control void ignores absolute materiality -> a Delta_ref control escapes (grok r3)",
     ['    if dt.get("directional") and dt.get("ci_at_or_beyond_margin"):\n'
      '        return True',
      '    if False:\n'
      '        return True'],
     "Delta_ref-sized control effect", "VANISHES"),

    ("engine trusts meta.complete and never checks n (grok r3)",
     ["    return len(paired(c)) >= REQUIRED_PAIRS",
      "    return len(paired(c)) > 0",
      "        if cov < MIN_COVERAGE:",
      "        if False:"],
     "n=1 with complete=yes", "VANISHES"),

    ("UNSTABLE/BAR-FAIL-INCONSISTENT outrank REPRODUCES -> a clean repro is masked (grok r3)",
     ["    if repro and inv:\n"
      '        verdict = "MIXED-SIGN"',
      "    if barfi:\n"
      '        verdict = "BAR-FAIL-INCONSISTENT"\n'
      '        why = "masked"\n'
      "    elif repro and inv:\n"
      '        verdict = "MIXED-SIGN"'],
     "one-direction REPRODUCES", "BAR-FAIL-INCONSISTENT"),

    ("a harness-detected session void is ignored (grok r3)",
     ['elif SESSION_VOID_REASON:', 'elif False:'],
     "session void (end-load)", "VANISHES"),

    # --- ROUND 5 -------------------------------------------------------------
    ("`bar == FAIL` is direction-blind, so +1ms is 'material' (codex r5)",
     ['        bar_fail_pos = (bar == "FAIL") and d_med > s_med     # the bar failed the SAME way\n'
      '        bar_fail_neg = (bar == "FAIL") and d_med < s_med',
      '        bar_fail_pos = (bar == "FAIL")\n'
      '        bar_fail_neg = (bar == "FAIL")'],
     "INVERSE direction cannot make +1ms", "REPRODUCES"),

    ("an UNCERTIFIED control can still support a null (codex r5 + grok r5)",
     ["    elif van and len(van) == len(verd) and ctrl_uncertified:",
      "    elif False:"],
     "UNDERPOWERED control carrying D=+230", "VANISHES"),

    ("the CI vetoes DIRECTION, so one zero pair turns 99ms into equivalence (grok r5)",
     ["        directional = p < 0.05",
      "        directional = p < 0.05 and ci_lo > 0 and ci_hi > 0"],
     "7/8 pairs at 99ms", "VANISHES"),

    ("the registered DELTA_REF is taken from the environment again (codex r5)",
     ['DELTA_REF = REGISTERED_DELTA_REF_MS\n'
      '_env_delta = os.environ.get("DELTA_REF_MS")',
      'DELTA_REF = int(os.environ.get("DELTA_REF_MS", REGISTERED_DELTA_REF_MS))\n'
      '_env_delta = None'],
     "DELTA_REF_MS is PINNED", "VANISHES"),
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
        # KILLED means THE CASE NOW FAILS -- by its own assertion, not by matching a
        # verdict named here. Checking only for a specific forbidden verdict let a
        # mutant "survive" merely by failing a DIFFERENT way than predicted, which is
        # still a caught regression. The case's own contract is the arbiter.
        _, _, must_be, must_not_be = case
        killed = (must_be and got != must_be) or (must_not_be and got == must_not_be)
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
