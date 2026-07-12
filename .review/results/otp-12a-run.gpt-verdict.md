# otp-12a recorded-run round — adjudication

**Reviewed range**: `775b6b5..b0ebf73` (otp-2 provenance correction,
live-earned harness fixes, CELLS escalation filter, the zoey evidence).
**Raw review**: `.review/results/otp-12a-run.codex.md` (gpt-5.6-sol,
144,573 tokens). **Verdict**: FAIL — 6 findings (1 High, 2 Medium,
3 Low), with the numerics explicitly confirmed ("medians, ratios,
requested diffs, executable bit, and sweep methodology check out"; the
recomputed table is "numerically exact"). All six ACCEPTED.
reviewer: gpt-5.6-sol

## F1 (High) — provenance grep false-positives; old client unestablished

Confirmed, and it explains an earlier session mystery: the e757dcc
client's only bare-sha match is the cargo-embedded build-directory
path (`…/blit-e757dcc/…`), not a build id — pre-cutover CLIENT binaries
embed none (daemons do). Also a bare-sha grep cannot distinguish clean
from `<sha>.dirty.…`. Fixed: greps now require the `+<sha>` build-id
form (paths can't match); where no id exists the operator must pass
`OLD_CLIENT_PROVENANCE_BY_BUILD=1`, acknowledging provenance rests on
the documented clean-worktree rebuild + the manifest sha256 — and the
evidence README states that basis explicitly. The recorded sessions'
old client IS that in-session worktree build (manifest-pinned), so the
data stands; the finding is about the check's rigor, which now matches
reality. A re-run remains cheap if the owner wants belt-and-braces.

## F2 (Medium) — D2 never pre-registered escalation supersession

Confirmed — the trigger was pre-registered, the governing session was
not; rolling push_tcp_large up as PASS was post-run adjudication, and
the README's "its best run beat the old median" claim was wrong (2597
beat the committed 2702, not the same-session 2418). Fixed: D2 amended
with a dated supersession rule (RUNS=8 governs; RUNS=4 rows stay
committed and visible), the README row now cites the amendment's
provenance honestly, and the wrong claim is corrected. The otp-13 walk
sees both sessions either way.

## F3 (Medium) — "provably rig-side" / dirt-content overreach

Confirmed. Fixed: the drift analysis is reframed as correlation plus
same-session parity (arithmetic unchanged — codex verified 1.248 /
0.995), with the dirty-reference confound named; the otp-2 correction
note no longer asserts what the dirt was.

## F4 (Low) — marginal-gap context misstated

Confirmed. Fixed: restated per the CSVs — the OLD arm ran 15.4% faster
than its own committed baseline; the unified path beats the committed
baseline by 6.5% and sits 10.5% behind the faster same-session old arm;
neighboring ratios listed plainly (0.909, 1.001, 1.005, 1.043) with no
"at or ahead" spin.

## F5 (Low) — mistyped CELLS exits 0 with empty evidence

Confirmed. Fixed: unmatched CELLS entries die after the matrix loop.

## F6 (Low) — stale header prerequisite

Confirmed. Fixed: the header now names the sha-named old-daemon default
and why the unqualified 2026-07-10 staging is never used.

## Fix commit

fix sha: `fa18787` (`bash -n` clean; check-docs green; docs + harness
script only — no crates/proto anywhere in otp-12; suite stands at the
recorded 1484).
