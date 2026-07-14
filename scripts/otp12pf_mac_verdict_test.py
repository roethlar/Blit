#!/usr/bin/env python3
"""Guard test for otp12pf_mac_verdict.py (rev 8, D-2026-07-14-3).

    python3 scripts/otp12pf_mac_verdict_test.py             # the cases
    python3 scripts/otp12pf_mac_verdict_test.py --mutations # prove they are not vacuous

NEARLY every case is a defect a reviewer actually drove out of a previous revision of this
engine, across ten review rounds. The rule was REWRITTEN and simplified; these cases are the
price of that rewrite -- each asserts that the SIMPLER rule still refuses the wrong answer
the COMPLEX rule once gave.

A mutation reverts one fix in a copy of the engine; the named case must then FAIL. NOT EVERY
CASE HAS ONE: 16 of the 37 do. The rest are behavioural (the rig must be able to SAY each
thing it can say) and have no single line to revert. Two more guards are asserted DIRECTLY
rather than by mutation, because at n=8 no synthetic session can tell the CI from the RANGE
-- they are the same two numbers -- and a mutation that cannot be killed is not a proof.
(Round-10, codex: the previous docstring claimed every case was mutation-proven. It was not.)
"""
import csv, os, random, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_VERDICT = os.path.join(HERE, "otp12pf_mac_verdict.py")
CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")
MEASURANDS = ("nq_tcp_mixed", "qn_tcp_mixed")
REGISTERED = MEASURANDS + CONTROLS
OUTCOMES = {"INCOMPLETE", "RIG-VOID", "CONTROLS-NOT-CLEAN", "MIXED", "REPRODUCES",
            "INVERTED", "DOES-NOT-REPRODUCE", "UNCLEAR",
            # A deliberate REFUSAL is the engine working, not failing -- the fuzz can draw
            # an impossible session (a destination time of zero when src=600 and d=-600),
            # and refusing it is the correct answer.
            "ENGINE-REFUSED"}


def engine():
    """Resolved per call: the mutation harness repoints it, and a cached path would
    silently test the UNMUTATED engine and report a kill it never made."""
    return os.environ.get("VERDICT_PY", DEFAULT_VERDICT)


def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=(),
            per_cell=None, void_reason="", pairs=8, env_extra=None, extra_rows=()):
    """`src` may be an int OR a per-pair list. The bar is computed on the MARGINAL
    medians and the CI on the PAIRED differences, and the two only disagree when the
    source arm varies -- a constant-only helper made that whole class of bug
    unguardable by construction.

    `extra_rows` = [(cell, arm, ms), ...]: valid rows in ONE arm with no partner in the
    other. Same reason (round-11, codex + grok, independently): every row this helper could
    write was PAIRED, so a CSV with a duplicate or unpaired valid row -- which skews that
    arm's MEDIAN, and therefore T, B and the bar, while the PAIR count still looks right --
    was unrepresentable, and the engine's arm-count check was consequently unguarded. A fix
    no test can express is a fix no test can PROVE."""
    control_d = [5] * pairs if control_d is None else control_d
    per_cell = per_cell or {}
    tmp = tempfile.mkdtemp()
    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
    present = [c for c in REGISTERED if c not in drop_cells]
    with open(runs, "w") as f:
        w = csv.writer(f)
        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,settled_ms,rtt_ms,files,bytes,"
                   "exit,drain,cold,valid".split(","))
        for cell in present:
            if cell in per_cell:
                d, s = per_cell[cell]
            elif cell in MEASURANDS:
                d, s = measurand_d, src
            else:
                d, s = control_d, control_src
            srcs = s if isinstance(s, list) else [s] * len(d)
            for i, (di, si) in enumerate(zip(d, srcs), 1):
                w.writerow([cell, "srcinit", "x", "h", i, si, 0, 250, 5, 1, 1, 0,
                            "drained_1x2s", "cold", "yes"])
                w.writerow([cell, "destinit", "x", "h", i, si + di, 0, 250, 5, 1, 1, 0,
                            "drained_1x2s", "cold", "yes"])
        # UNPAIRED valid rows: a run id no other arm carries, so the PAIR count is untouched
        # and only the arm's own median moves.
        for j, (cell, arm, ms) in enumerate(extra_rows, 1):
            w.writerow([cell, arm, "x", "h", "x%d" % j, ms, 0, 250, 5, 1, 1, 0,
                        "drained_1x2s", "cold", "yes"])
    with open(meta, "w") as f:
        f.write("cell,pairs_attempted,complete\n")
        for cell in present:
            # `complete=yes` is asserted even when a cell is SHORT: the engine must not
            # believe it (a 1-pair CSV once graded as a full cell at 0% CI coverage).
            f.write("%s,%d,yes\n" % (cell, pairs))
    env = dict(os.environ, VERDICT_CELLS=",".join(MEASURANDS),
               CONTROL_CELLS=",".join(CONTROLS), REGISTERED_CELLS=",".join(REGISTERED),
               REQUIRED_PAIRS="8", SESSION_VOID_REASON=void_reason)
    env.pop("DELTA_REF_MS", None)                      # PINNED in the engine
    env.update(env_extra or {})
    out = subprocess.run([sys.executable, engine(), runs, meta,
                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
                         env=env, capture_output=True, text=True)
    # A DELIBERATE refusal is the engine WORKING, not failing: exit 2 (a corrupt or
    # impossible row) or an explicit REFUSING (a pinned constant or cell role tampered with).
    if out.returncode == 2 or (out.returncode != 0 and "REFUSING" in (out.stderr or "")):
        return "ENGINE-REFUSED"
    if out.returncode != 0:
        return "ENGINE-CRASH: " + (out.stderr.strip().splitlines() or ["?"])[-1]
    return out.stdout.splitlines()[0].split(":", 1)[1].strip()


# (name, kwargs, must_be, must_not_be)
CASES = [
    # --- a real effect must never read as nothing --------------------------------
    ("codex r1: a 190ms effect on 7/8 pairs is not a null",
     dict(measurand_d=[0, 180, 180, 190, 190, 200, 200, 200], src=2000),
     "UNCLEAR", "DOES-NOT-REPRODUCE"),

    ("codex r2: a rig-W-sized effect (230ms) in EVERY pair, on a slow 2500ms arm",
     dict(measurand_d=[230] * 8, src=2500, control_d=[0] * 8),
     "REPRODUCES", "DOES-NOT-REPRODUCE"),

    ("codex r2: an effect the 10% bar alone would forgive (240ms @ 2500)",
     dict(measurand_d=[-100, -50, 0, 50, 100, 200, 220, 240], src=2500, control_d=[0] * 8),
     "UNCLEAR", "DOES-NOT-REPRODUCE"),

    ("codex r2: the inverting threshold is -src/11, not -src/10 (CI [-190,0] @ 2000)",
     dict(measurand_d=[-190, -190, 0, 0, 0, 0, 0, 0], src=2000, control_d=[0] * 8),
     "UNCLEAR", "DOES-NOT-REPRODUCE"),

    # --- an artifact must never read as an effect --------------------------------
    ("codex r2: 7 positive + 1 negative is not a reproduction",
     dict(measurand_d=[-20, 300, 310, 320, 330, 340, 350, 360], src=1000),
     "UNCLEAR", "REPRODUCES"),

    ("codex r5: a 1ms paired effect is not a reproduction, whatever the medians do",
     dict(measurand_d=[1] * 13 + [-4500] * 3,
          src=[1000] * 7 + [1200] * 6 + [5000] * 3,
          control_d=[5] * 16, control_src=1000, pairs=16),
     None, "REPRODUCES"),

    ("codex r6: nor when the marginal bar fails in the MATCHING direction",
     dict(measurand_d=[400] * 3 + [1] * 13, src=[1000] * 8 + [1200] * 8,
          control_d=[5] * 16, control_src=1000, pairs=16),
     None, "REPRODUCES"),

    ("one huge outlier must not manufacture a reproduction (the CI's LOWER bound decides)",
     dict(measurand_d=[10, 10, 10, 10, 10, 10, 10, 800], src=1000),
     "UNCLEAR", "REPRODUCES"),

    ("grok r9: a LONG cell (16 pairs) is INCOMPLETE — a CI at n>8 TRIMS the pairs that contradict it",
     dict(measurand_d=[-500] * 3 + [200] * 13, src=1000, control_d=[0] * 16),
     "INCOMPLETE", "REPRODUCES"),

    ("a SHORT cell (6 of 8 pairs) claiming complete=yes is INCOMPLETE",
     dict(measurand_d=[-4, -2, -1, 0, 1, 2], src=2000),
     "INCOMPLETE", "DOES-NOT-REPRODUCE"),

    # codex r11 (MEDIUM) + grok r11 (HIGH), found INDEPENDENTLY: the arm-count check was the
    # engine's only defence against a valid-but-UNPAIRED row, and NOTHING guarded it -- delete
    # the two conjuncts and all 34 cases still passed. The 8 pairs are intact here, so the pair
    # count looks right; 12 extra valid srcinit rows at 100ms drag that ARM's median from
    # 1000ms to 100ms, T = src/10 collapses from 100 to 10, and a +50ms difference -- a NULL at
    # the true arm speed -- becomes an EFFECT. The CSV is the harness's, but the engine must not
    # be the thing that trusts it: this is the row-integrity check, and it now has a proof.
    ("codex+grok r11: unpaired valid rows skew the ARM median -- exactly 8 rows per arm, or INCOMPLETE",
     dict(measurand_d=[50] * 8, src=1000, control_d=[0] * 8,
          extra_rows=[("nq_tcp_mixed", "srcinit", 100)] * 12),
     "INCOMPLETE", "REPRODUCES"),

    # --- the controls are a precondition -----------------------------------------
    ("grok r2: a bar-FAIL control whose CI crosses zero blocks every verdict",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[-100, -50, 300, 320, 340, 350, 360, 380], control_src=1000),
     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),

    ("grok r4: a Delta_ref-sized control effect blocks every verdict",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[230] * 8, control_src=2500),
     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),

    ("codex r5: ...and so does one with a single zero pair (CI [0,230])",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[0] + [230] * 7, control_src=2500),
     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),

    ("grok r5: ...and a non-directional one (CI [-10,230])",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[230] * 7 + [-10], control_src=2500),
     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),

    ("grok r6: ...and one at D=+229, ONE MS under the reference effect",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[229] * 8, control_src=2500),
     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),

    # THE CASE THAT ISOLATES THE T/2 CONTROL BAR. A SLOW control (5000ms arm) carrying 120ms
    # is dirty at T/2 = 115 but would pass at the full T = 230, and its bias fraction (2.4%)
    # is small enough on a 1000ms measurand (B = 24 vs T/2 = 50) that the B >= T/2 gate does
    # NOT fire. So this is the ONLY case where certifying controls at T instead of T/2 changes
    # the verdict -- every faster control that fails at T/2 is now ALSO caught by the bias
    # gate, which silently made the old mutation for this fix VACUOUS (it survived).
    ("grok r6: a control clean at T but DIRTY at T/2 blocks every verdict (T/2 is load-bearing)",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=1000,
          control_d=[120] * 8, control_src=5000),
     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),

    ("codex r6: a dirty control blocks a REPRODUCTION too, not just a null",
     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000,
          control_d=[0] + [230] * 7, control_src=2500),
     "CONTROLS-NOT-CLEAN", "REPRODUCES"),

    # ...but a GOOD rig must still be able to ANSWER. An instrument that can never
    # conclude is also broken (grok r6: the "dead zone").
    ("a clean rig with a tiny host x role control asymmetry still answers",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[5] * 8, control_src=1000),
     "DOES-NOT-REPRODUCE", "CONTROLS-NOT-CLEAN"),

    # --- the rig must be able to say each of the things it can say ----------------
    ("a real, bar-breaking slowdown reproduces",
     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000),
     "REPRODUCES", None),

    ("an exact 10% effect is reportable on a bias-free rig (it was once unreachable)",
     dict(measurand_d=[100] * 8, src=1000, control_d=[0] * 8),
     "REPRODUCES", None),

    # codex r8, BLOCKER: a control at +5 is "clean", but that 5ms of arm bias may be
    # riding in the measurand too -- so an effect of EXACTLY T could be (T-5) real plus
    # 5 rig. It must not be banked as a reproduction. B carries the bias the controls
    # could not exclude into the measurand's threshold.
    ("codex r8: an effect of exactly T is NOT a reproduction when the controls carry bias",
     dict(measurand_d=[100] * 8, src=1000, control_d=[5] * 8),
     "UNCLEAR", "REPRODUCES"),

    ("codex r9: B is RELATIVE — a 4.9% bias on a FAST control must not under-penalise a slower measurand",
     dict(measurand_d=[130] * 8, src=1000,
          control_d=[24] * 8, control_src=500),
     "UNCLEAR", "REPRODUCES"),

    ("grok r9: a null must also survive the TIGHTER bound (bias could be MASKING an effect)",
     dict(measurand_d=[60] * 8, src=1000, control_d=[49] * 8),
     "UNCLEAR", "DOES-NOT-REPRODUCE"),

    # codex r11, HIGH (grok found the same dead-zone): the controls PASS at T/2 and the rig is
    # still unreadable. T is capped at 230; the permitted bias is 4.9% OF THE ARM. On a 10000ms
    # measurand that is B=490 against T=230 -- a null is impossible (T-B < 0) and the "effect"
    # at 720ms is up to 68% permitted rig bias, at a ratio of only 1.072. Owner: refuse to grade.
    ("codex r11: B >= T/2 is NOT a clean rig, even when every control passes at T/2",
     dict(measurand_d=[720] * 8, src=10000, control_d=[49] * 8, control_src=1000),
     "CONTROLS-NOT-CLEAN", "REPRODUCES"),

    # ...and the SAME control bias on a measurand whose arm it can actually bound still grades.
    # The gate must bite on the DIVERGENCE (capped T vs fractional B), not on any bias at all.
    ("codex r11: ...but a rig whose bias is small against T still answers",
     dict(measurand_d=[300] * 8, src=1000, control_d=[10] * 8, control_src=1000),
     "REPRODUCES", "CONTROLS-NOT-CLEAN"),

    ("codex r8: ...and the same effect IS one once the rig is bias-free",
     dict(measurand_d=[105] * 8, src=1000, control_d=[5] * 8),
     "REPRODUCES", "UNCLEAR"),

    ("source-initiated slower is INVERTED, never 'P1 absent'",
     dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
     "INVERTED", None),

    ("one direction reproduces, the other inverts -> MIXED",
     dict(measurand_d=[0] * 8, src=1000,
          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
                    "qn_tcp_mixed": ([-300, -310, -320, -330, -340, -350, -360, -370], 1000)}),
     "MIXED", "REPRODUCES"),

    # codex r11, HIGH: B hardens each CELL but could make the SESSION verdict EASIER. At
    # +110 / -94 on a 1000ms arm, controls AT ZERO give MIXED; clean controls at +5 push the
    # -94 cell out of INVERTED (it needs <= -95.9), the MIXED branch stops firing, and the
    # session upgrades itself to REPRODUCES. A NOISIER RIG PRODUCED A STRONGER CLAIM.
    ("codex r11: a NOISIER rig must not upgrade MIXED to REPRODUCES",
     dict(measurand_d=[0] * 8, src=1000, control_d=[5] * 8, control_src=1000,
          per_cell={"nq_tcp_mixed": ([110] * 8, 1000),
                    "qn_tcp_mixed": ([-94] * 8, 1000)}),
     "MIXED", "REPRODUCES"),

    ("a clean one-direction reproduction is NOT masked by a noisy sibling",
     dict(measurand_d=[0] * 8, src=1000,
          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
                    "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
     "REPRODUCES", "UNCLEAR"),

    # codex r11, MEDIUM: the ARM median controls T, B and the bar -- and the LOW median (which
    # is registered only for the paired D) is anti-conservative on a BIMODAL arm. Here the arm
    # is 4x1000 + 4x5000: the low median calls it 1000ms, so +100 is "a 10% effect" -> EFFECT.
    # The conventional median calls it 3000ms, where the same +100 is 3.3% -- below both bars.
    # Rig W's fast arm is ALREADY bimodal (~730/~840), so this is not a synthetic worry.
    ("codex r11: a BIMODAL arm must not shrink T (the arm median is conventional, not low)",
     dict(measurand_d=[100] * 8, src=[1000] * 4 + [5000] * 4, control_d=[0] * 8),
     "DOES-NOT-REPRODUCE", "REPRODUCES"),

    ("codex r8: a bimodal arm cannot hide from the RANGE (a null is judged on every pair)",
     dict(measurand_d=[-110, 0, -110, 110, 110, 0, -110, 0], src=730,
          control_d=[0] * 8),
     "UNCLEAR", "DOES-NOT-REPRODUCE"),

    ("a null the rig could not have SEEN is UNCLEAR, not a null",
     dict(measurand_d=[-400, -300, -100, 0, 0, 100, 300, 400], src=2000),
     "UNCLEAR", "DOES-NOT-REPRODUCE"),

    # --- integrity ---------------------------------------------------------------
    ("a missing registered cell is INCOMPLETE, never filtered away",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          drop_cells=("qn_tcp_mixed",)),
     "INCOMPLETE", "DOES-NOT-REPRODUCE"),

    ("grok r3: n=1 with complete=yes must not grade at 0% CI coverage",
     dict(measurand_d=[0], src=2000, control_d=[5], control_src=1000),
     "INCOMPLETE", "DOES-NOT-REPRODUCE"),

    ("grok r3: a harness-detected session void (end-load) refuses a verdict",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          void_reason="end-load on q is 9.1 (> 3.0)"),
     "RIG-VOID", "DOES-NOT-REPRODUCE"),

    ("codex r10: a session of ZERO timings must not report an EFFECT",
     dict(measurand_d=[0] * 8, src=0, control_d=[0] * 8, control_src=0),
     "ENGINE-REFUSED", "REPRODUCES"),

    ("codex r10: the CELL ROLES are pinned -- a dirty control cannot be dropped from the set",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          control_d=[230] * 8, control_src=2500,
          env_extra={"CONTROL_CELLS": "nq_grpc_mixed"}),
     "ENGINE-REFUSED", "DOES-NOT-REPRODUCE"),

    ("codex r5: DELTA_REF_MS is PINNED -- the rule is not tunable from the environment",
     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
          env_extra={"DELTA_REF_MS": "240"}),
     "ENGINE-REFUSED", "DOES-NOT-REPRODUCE"),
]

MUTATIONS = [
    ("the control threshold is the SAME as the measurand's, not half (grok r6)",
     ['    c_pos, c_neg = thresholds(x["src"], 0.5)',
      '    c_pos, c_neg = thresholds(x["src"], 1.0)'],
     "clean at T but DIRTY at T/2"),

    ("dirty controls block only the null, not a reproduction (codex r6)",
     ["elif dirty or bias_over:",
      "elif (dirty or bias_over) and not any(s == 'EFFECT' for s in m.values()):"],
     "blocks a REPRODUCTION too"),

    ("a permitted bias of HALF the threshold still grades -- B > T licenses a rig effect (codex r11)",
     ["    if t_pos > 0 and B >= t_pos / 2.0:", "    if False:"],
     "B >= T/2 is NOT a clean rig"),

    ("the ARM median is the LOW median again, so a bimodal arm shrinks T (codex r11)",
     ["    v = sorted(v)\n    n = len(v)\n    if n % 2:\n        return float(v[n // 2])\n"
      "    return (v[n // 2 - 1] + v[n // 2]) / 2.0",
      "    v = sorted(v)\n    return float(v[(len(v) - 1) // 2])"],
     "BIMODAL arm must not shrink T"),

    ("MIXED is decided on the HARDENED states, so control noise upgrades it to REPRODUCES (codex r11)",
     ['elif "EFFECT" in m0.values() and "INVERTED" in m0.values():',
      'elif "EFFECT" in m.values() and "INVERTED" in m.values():'],
     "NOISIER rig must not upgrade MIXED"),

    ("the inverting threshold is -src/10, not -src/11 (codex r2)",
     ["            -min(s_med / 11.0, float(DELTA_REF)) * scale)",
      "            -min(s_med / 10.0, float(DELTA_REF)) * scale)"],
     "inverting threshold is -src/11"),

    ("the threshold ignores DELTA_REF, so the bar alone forgives 240ms (codex r2)",
     ["    return (min(s_med / 10.0, float(DELTA_REF)) * scale,",
      "    return ((s_med / 10.0) * scale,"],
     "bar alone would forgive"),

    ("EFFECT is decided on the CI's MIDPOINT, not its lower bound (an outlier reproduces)",
     ["    if ci_lo >= t_pos + B:", "    if (ci_lo + ci_hi) / 2.0 >= t_pos + B:"],
     "one huge outlier"),

    ("the NULL is not tightened by the control bias -- a masked effect reads as a null (grok r9)",
     ["    if t_neg + B < rng_lo and rng_hi < t_pos - B:",
      "    if t_neg < rng_lo and rng_hi < t_pos:"],
     "null must also survive the TIGHTER bound"),

    ("the EFFECT is not hardened by the control bias -- an effect of exactly T reproduces (codex r8)",
     ["    if ci_lo >= t_pos + B:", "    if ci_lo >= t_pos:"],
     "exactly T is NOT a reproduction"),

    ("B is carried as RAW MILLISECONDS across controls of different arm speeds (codex r9)",
     ['        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])',
      '        B_frac = max(B_frac, abs(x["rng"][0]), abs(x["rng"][1]))',
      '    B = B_frac * x["src"]                    # the control bias, on THIS cell\'s arm',
      "    B = B_frac"],
     "B is RELATIVE"),

    ("the control's residual bias is not carried into the measurand (codex r8)",
     ['        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])',
      "        B_frac = max(B_frac, 0.0)"],
     "exactly T is NOT a reproduction"),

    ("the engine trusts meta.complete and never counts the pairs (grok r3)",
     ['    if (meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None\n'
      '            or len(by.get((c, "srcinit"), [])) != PAIRS\n'
      '            or len(by.get((c, "destinit"), [])) != PAIRS):',
      '    if meta.get(c, {}).get("complete") != "yes" or ci is None:'],
     "SHORT cell (6 of 8 pairs)"),

    # SELECTIVELY: only the two ARM-count conjuncts, leaving meta.complete and the PAIR count
    # in place. The combined mutation above dies on the short-pair case and so proved nothing
    # about these two lines (round-11, codex + grok).
    ("the engine counts PAIRS but not the rows in each ARM (codex+grok r11)",
     ['    if (meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None\n'
      '            or len(by.get((c, "srcinit"), [])) != PAIRS\n'
      '            or len(by.get((c, "destinit"), [])) != PAIRS):',
      '    if (meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None):'],
     "unpaired valid rows skew the ARM median"),

    ("a missing registered cell is filtered away (codex r2)",
     ["for c in sorted(set(REGISTERED) | set(meta)):", "for c in sorted(meta):"],
     "missing registered cell"),

    ("a harness-detected session void is ignored (grok r3)",
     ["elif SESSION_VOID:", "elif False:"],
     "session void (end-load)"),

    ("a non-positive timing is accepted, and zeros then report an EFFECT (codex r10)",
     ["        if v <= 0:", "        if False:"],
     "ZERO timings"),

    ("the cell ROLES are taken from the environment again (codex r10)",
     ["    if _got and _got != _want:", "    if False:"],
     "CELL ROLES are pinned"),

    ("the registered DELTA_REF is taken from the environment again (codex r5)",
     ['_env = os.environ.get("DELTA_REF_MS")', "_env = None"],
     "DELTA_REF_MS is PINNED"),
]


def rule_unit_tests():
    """The RULE itself, called directly -- because a session at n=8 cannot distinguish the
    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
    it closed if a larger n is ever registered again, and it can only be tested here."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("eng", DEFAULT_VERDICT)
    # the engine runs on import (it is a script), so exercise classify() via a subprocess-free
    # re-implementation guard: read the function out of the source and exec it in isolation.
    src = open(DEFAULT_VERDICT).read()
    start = src.index("def classify(")
    end = src.index("\n\n", src.index("return \"UNCLEAR\"", start))
    ns = {}
    exec(src[start:end], ns)
    classify = ns["classify"]
    bad = 0
    checks = [
        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
        ("a real effect clears T",                       (80, 90, 75, 95, 73, -66),   "EFFECT"),
        ("an inverted effect clears -T",                 (-90, -80, -95, -75, 73, -66), "INVERTED"),
    ]
    for name, args, want in checks:
        got = classify(*args)
        ok = got == want
        print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
        if not ok:
            bad += 1

    # THE IDENTITY THE WHOLE RULE LEANS ON: at the registered n=8, the >=95% order-statistic
    # interval IS [min, max]. That is why nothing can be trimmed -- not a null, not B. If
    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
    # must refuse that n (it does).
    ns2 = {}
    src2 = open(DEFAULT_VERDICT).read()
    st = src2.index("def median_ci(")
    exec(src2[st:src2.index("\n\n", src2.index("return best", st))],
         {"comb": __import__("math").comb, "MIN_COVERAGE": 0.95}, ns2)
    import random as _r
    rr = _r.Random(9)
    for _ in range(200):
        d = [rr.randint(-500, 500) for _ in range(8)]
        lo, hi, cov = ns2["median_ci"](d)
        if (lo, hi) != (min(d), max(d)) or cov < 0.95:
            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
            bad += 1
            break
    else:
        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
    return bad


def run_cases():
    bad = []
    for name, kw, must_be, must_not in CASES:
        got = session(**kw)
        ok = not (must_be and got != must_be) and not (must_not and got == must_not)
        print("%-66s -> %-20s %s" % (name[:66], got, "ok" if ok else "*** FAIL ***"))
        if not ok:
            bad.append(name)
            print("      expected %s / must not be %s" % (must_be, must_not))
    return bad


def fuzz(n=300):
    """No input may land outside the registered outcomes. The CONTROLS are fuzzed too --
    pinning them clean once left every dirty-control path unexercised, and that is
    exactly where a BLOCKER was hiding."""
    rng = random.Random(4242)
    bad = 0
    for _ in range(n):
        got = session(measurand_d=[rng.randint(-600, 600) for _ in range(8)],
                      src=rng.choice([600, 1000, 2000, 2500, 5000]),
                      control_d=[rng.randint(-300, 300) for _ in range(8)],
                      control_src=rng.choice([600, 1000, 2500, 5000]))
        if got not in OUTCOMES:
            print("*** UNREGISTERED OUTCOME %r" % got)
            bad += 1
    print("fuzz: %d/%d inputs produced a registered outcome (measurand AND controls)"
          % (n - bad, n))
    return bad


def mutate():
    src = open(DEFAULT_VERDICT).read()
    bad = 0
    for name, subs, key in MUTATIONS:
        body = src
        for i in range(0, len(subs), 2):
            old, new = subs[i], subs[i + 1]
            if old not in body:     # the engine drifted: the proof is STALE, not passing
                print("*** STALE MUTATION (target not found): %s" % name)
                bad += 1
                body = None
                break
            body = body.replace(old, new, 1)
        if body is None:
            continue
        tmp = tempfile.mkdtemp()
        path = os.path.join(tmp, "mutant.py")
        open(path, "w").write(body)
        case = next(c for c in CASES if key in c[0])
        os.environ["VERDICT_PY"] = path
        got = session(**case[1])
        del os.environ["VERDICT_PY"]
        # KILLED == the case now FAILS, by its OWN contract. Checking instead for a
        # verdict named here let a mutant "survive" by failing a different way.
        _, _, must_be, must_not = case
        killed = (must_be and got != must_be) or (must_not and got == must_not)
        print("%-66s -> %-20s %s" % (name[:66], got,
                                     "KILLED" if killed else "*** SURVIVED — VACUOUS ***"))
        if not killed:
            bad += 1
    return bad


if __name__ == "__main__":
    if "--mutations" in sys.argv:
        print("Reverting each fix in a copy of the engine; the named case must then FAIL.\n")
        n = mutate()
        print("\n%d/%d mutations killed" % (len(MUTATIONS) - n, len(MUTATIONS)))
        sys.exit(1 if n else 0)
    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")
    unit = rule_unit_tests()
    print()
    fails = run_cases()
    print()
    z = fuzz()
    print("\n%d/%d cases passed" % (len(CASES) - len(fails), len(CASES)))
    sys.exit(1 if (fails or z or unit) else 0)
