# ldt-4-r3-f2 — validate canonical fixture shape before copying

**Severity**: LOW — a drifted or oversized canonical source could consume a
large transfer and breach the retained-space floor before the harness voided.
**Status**: Fixed and mutation-proved; accepted by clean final whole-change review.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `fdf7b3771c00c950ca40fb7e9904a91c89f8a72d`

## Evidence

`scripts/bench_ldt4_rigw.sh:775-796` wrote and hash-verified the Windows
canonical manifest before `scp`, but previously compared its shape with the
registered fixture only after the copy. The pre-copy free-space gate derived
`fixture_bytes` from the registered constant rather than the already-fetched
manifest.

## Predicted observable failure

If the canonical Windows tree drifts in file count or bytes, the harness can
copy up to the full wrong tree before rejecting it. An oversized tree can move
q below `MIN_FREE_BYTES` during that transfer and leave a large partial tree in
the retained incoming namespace. The session still fails closed, but it wastes
wire time, creates avoidable disk pressure, and adds operator cleanup burden in
an already-failing launch.

## What

Validate the fetched canonical manifest's exact registered shape before any
large copy. Derive the space reservation from that validated manifest, then
retain the post-copy q shape and exact manifest equality checks.

## Approach

`stage_fixtures` now computes `expected` and `win_shape` immediately after the
hash-verified manifest fetch and voids unless they are equal. `fixture_bytes`
comes from `win_shape`, so the capacity gate is tied to the source that will be
copied. After `scp`, the q manifest still has to match `win_shape` and compare
byte-for-byte with the fetched Windows manifest before exclusive promotion.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — pre-copy canonical shape check, manifest-sized
  capacity gate, explicit q shape failure, and ordered offline guard.

## Guard proof

- The 96-arm no-SSH self-test requires manifest write/fetch, registered source
  shape equality, manifest-derived bytes, the q free-space probe, and the actual
  fail-closed capacity enforcement to precede `scp`; q shape, exact manifest
  equality, and exclusive promotion must follow.
- Restoring the old behavior—registered-constant sizing before copy and Windows
  shape validation only afterward—makes the self-test fail before any SSH;
  exact restoration returns `PASS (96 arms, no SSH)`.
- Removing only the low-space enforcement also makes the self-test fail before
  any SSH; exact restoration again returns `PASS (96 arms, no SSH)`.

## Coder dispute

None.

## Known gaps

The live staging path has not run. A fresh exact harness launch remains before
hardware evidence.

## Reviewer comments

Claude Fable 5/max returned the candidate over exact `4e0fdc3..ef48920` with
`guard_confirmed=true`. Intake admitted the avoidable transfer/disk-pressure
failure because the manifest needed to reject it is already available before
copy. Final Fable 5/max review over exact `4e0fdc3..5a2265e` returned clean with
`guard_confirmed=true`; record:
`.review/results/ldt-4-canonical-r2.claude-verdict.md`.
