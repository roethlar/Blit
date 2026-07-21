# ldt-4-live-f6 — gate q on stable machine identity, not resolver hostname

**Severity**: MEDIUM — the registered run cannot pass its start environment
gate after a network-service transition changes the resolver-derived hostname
of the same pinned Mac.
**Status**: Accepted; fix and guard pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

Exact staged and tactically reviewed harness `322a161` authenticated current
Windows `.173`, verified all three cross-endpoint fixture manifests, and then
failed closed in retained session
`ldt4-20260721T202216Z-322a1611230e` at the start environment gate:
`harness is not executing on q.lan`.

The same q machine now reports resolver-derived `hostname=Q.local` after its
10 GbE service returned. Its stable macOS identity remains
`scutil --get LocalHostName = Q` and `scutil --get ComputerName = Q`; registered
`en8` MAC/IP/MTU/media/topology gates remain independent. `runs.csv` has zero
data rows, q port 9031 is closed, and Windows runtime preparation, daemon swap,
and transfer never began.

## Predicted observable failure

Every live launch reserves additive evidence and endpoint namespaces, stages
and verifies fixtures, then voids before daemon preparation whenever DHCP or
network-service priority makes `hostname` render a different domain suffix or
case for the same Mac.

## What

Replace the resolver-derived `hostname == q.lan` gate with stable exact macOS
LocalHostName and ComputerName gates, record both values in start/end evidence,
and make the analyzer validate them.

## Approach

- Read `scutil --get LocalHostName` and `scutil --get ComputerName`, fail closed
  unless each is exact `Q`, and retain the existing MAC/IP/NIC/link gates.
- Record both stable identities in every environment boundary file.
- Extend analyzer parsing and synthetic tests to require both exact fields.
- Exercise the production identity helper through the Bash 3.2 self-test with
  a local `scutil` stub; mutation-prove both producer and analyzer guards.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — pending stable q identity helper/gate/evidence.
- `scripts/ldt4_rigw_analyze.py` — pending stable identity validation.
- `scripts/ldt4_rigw_analyze_test.py` — pending exact evidence guard.

## Guard proof

Pending.

## Coder dispute

None.

## Known gaps

Fix, focused/full gates, tactical review, additive staging, and a completed
live arm/run remain.

## Reviewer comments

This finding came from the attached live launch, not a reviewer candidate.
