#!/usr/bin/env python3
"""Guard test for otp12pf_mac_verdict.py — run it before trusting a Mac<->Mac run.

    python3 scripts/otp12pf_mac_verdict_test.py

The defect it guards (codex round-2 BLOCKER on the harness): the first revision
graded "did the effect vanish?" against S = max(d) - min(d), a RANGE. A range
grows with n and is dominated by outliers, so a large CONSISTENT effect hides
under it:

    srcinit = 2000 ms;  d = [0,180,180,190,190,200,200,200]
    -> D = 190, S = 200, bar PASSES, |D| <= S  =>  "VANISHES"

...on 7/8 positive pairs, with an effect 83% the size of rig W's Delta_P1. It
would have reported "P1 requires the Windows peer" off an effect nearly as large
as P1 itself. The rule now uses a bootstrap CI + an equivalence bound against the
bar-breaching effect, and this test pins that.
"""
import csv, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
VERDICT = os.path.join(HERE, "otp12pf_mac_verdict.py")
CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")
VERDICT_CELLS = ("nq_tcp_mixed", "qn_tcp_mixed")


def verdict_for(d, src=2000):
    tmp = tempfile.mkdtemp()
    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
    with open(runs, "w") as f:
        w = csv.writer(f)
        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid".split(","))
        for cell in VERDICT_CELLS:
            for i, di in enumerate(d, 1):
                w.writerow([cell, "srcinit", "x", "h", i, src, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
                w.writerow([cell, "destinit", "x", "h", i, src + di, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
        for cell in CONTROLS:            # clean controls, so the rig is not VOID
            for i in range(1, 9):
                w.writerow([cell, "srcinit", "x", "h", i, 1000, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
                w.writerow([cell, "destinit", "x", "h", i, 1005, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
    with open(meta, "w") as f:
        f.write("cell,pairs_attempted,complete\n")
        for cell in VERDICT_CELLS + CONTROLS:
            f.write("%s,8,yes\n" % cell)
    env = dict(os.environ, DELTA_REF_MS="230",
               VERDICT_CELLS=",".join(VERDICT_CELLS),
               CONTROL_CELLS=",".join(CONTROLS))
    out = subprocess.run([sys.executable, VERDICT, runs, meta,
                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
                         env=env, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("verdict engine failed:\n" + out.stderr)
    return out.stdout.splitlines()[0].split(":", 1)[1].strip()


CASES = [
    # (name, d, src, must_be, must_not_be)
    ("codex counterexample: real 190ms effect, 7/8 positive",
     [0, 180, 180, 190, 190, 200, 200, 200], 2000, None, "VANISHES"),
    ("a genuinely absent effect",
     [-4, -2, -1, 0, 0, 1, 2, 3], 2000, "VANISHES", None),
    ("a bar-breaking slowdown (destination-initiated)",
     [300, 310, 320, 330, 340, 350, 360, 370], 1000, "REPRODUCES", None),
    ("source-initiated is the slow arm",
     [-300, -310, -320, -330, -340, -350, -360, -370], 1000, "INVERSION", None),
]

failures = 0
for name, d, src, must_be, must_not_be in CASES:
    got = verdict_for(d, src)
    ok = True
    if must_be and got != must_be:
        ok = False
    if must_not_be and got == must_not_be:
        ok = False
    print("%-52s -> %-26s %s" % (name, got, "ok" if ok else "*** FAIL ***"))
    if not ok:
        failures += 1
        if must_be:
            print("      expected %s" % must_be)
        if must_not_be:
            print("      must NOT be %s (the range-rule bug is back)" % must_not_be)

print()
print("%d/%d cases passed" % (len(CASES) - failures, len(CASES)))
sys.exit(1 if failures else 0)
