# P1 offline evidence reconciliation (2026-07-22)

**Status:** Evidence — resolved from retained data, exact code history, and
mutation-sensitive deterministic guards. No transfer or endpoint command was
run for this reconciliation.

## Conclusion

P1 exposed a real historical product defect, and that defect is fixed. The
exact failing product code deterministically realized different TCP worker
counts depending on which endpoint initiated the session. The retained rig
rows did not log worker count, so they cannot prove that those physical runs
landed at exactly 3 and 2; their slow/fast ordering and the post-fix target-cell
result both match the direct code guard.

The stale project record called P1 "unexplained" because a later hardware run
could not assign a formal millisecond recovery to the fix. That limitation does
not erase the direct code proof: the exact pre-fix production path failed the
two role guards at different worker counts, the fix made both roles reach the
same target, and removing either half of the fix makes the guards fail again.

## Evidence chain

1. The two failing sessions used builds `e21cf84` and `f35702a`. Their
   Windows→Mac `wm_tcp_mixed` cells measured destination initiation at
   1127/1221 ms and source initiation at 911/939 ms: ratios 1.237 and 1.300.
2. The relevant production files are unchanged from `f35702a` through
   `a76b785^` (`6b0f01c`); only role-test text changed. The pre-fix guard
   therefore exercises the same worker-control code as the second failing
   session. The only relevant difference between `e21cf84` and `f35702a` is
   mid-record error handling, not worker selection or resize.
3. Before `a76b785`, the shared shape calculation wanted eight workers but the
   session stopped extending the one-worker-per-epoch ramp when
   `NeedComplete` arrived. The source-initiated guard stopped at 3 workers and
   the destination-initiated guard at 2. The destination-initiated admission
   path also read the legal wire value `max_streams = 0` as a one-worker cap,
   while the source-owned path correctly read it as unknown/default.
4. In the failing Windows→Mac cell, `win_init` is the SOURCE-initiated arm and
   `mac_init` is the DESTINATION-initiated arm. The code defect's ordering is
   therefore the measured ordering: 3-worker source initiation was faster;
   2-worker destination initiation was slower.
5. `a76b785..42b9b38` fixed both causes: one SOURCE-owned ramp continues to the
   shared target, both role layouts use the same receiver-ceiling resolver,
   refusal is terminal, and resize arbitration is serialized. The exact role
   guards changed from 3-versus-2 red (and a separate destination-only 1) to
   8-versus-8 green. Recorded mutations restoring the stopped ramp or the
   destination-only zero-capacity interpretation fail those guards.
6. The retained post-fix build `8e019ef` contains the complete fix range. Its
   static-target traces reached eight workers in both layouts. Its P1 target
   cell reversed direction and passed the original point bar: 1.0738 with
   tracing off and 1.0933 with tracing on. That run's noisy controls prevent a
   broader hardware-causality claim; they do not contradict the direct
   old-red/new-green code proof.
7. `65a0f9f` later replaced the static target with the current adaptive
   controller. Its deterministic real-session guards run both socket layouts
   through identical ADD `4→17`, REMOVE `4→1`, idle/hysteresis, and receiver-
   bound traces using the same SOURCE-owned policy.

## Result

- The historical P1 initiator discrepancy is closed as a fixed code defect.
- The current code retains deterministic role parity under the adaptive
  controller.
- No new physical transfer is required to close this discrepancy.
- This does not close the separate P2 old-versus-new small-file performance
  finding.

Canonical implementation and review records:

- `.review/findings/otp-12-worker-parity.md`
- `.review/findings/ldt-2.md`
- `docs/bench/otp12-pf1-rigw-2026-07-15/README.md`
