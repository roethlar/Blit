# ldt-4-live-f6 — gate q on stable machine identity, not resolver hostname

**Severity**: MEDIUM — the registered run cannot pass its start environment
gate after a network-service transition changes the resolver-derived hostname
of the same pinned Mac.
**Status**: Fixed, mutation-proved, and full-gate green; tactical review pending.
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

- `scripts/bench_ldt4_rigw.sh` — stable q identity helper/gate/evidence.
- `scripts/ldt4_rigw_analyze.py` — stable identity validation.
- `scripts/ldt4_rigw_analyze_test.py` — exact evidence guard.

## Guard proof

- Focused restored green: `bash -n`; 96-arm Bash self-test with no SSH; 77
  analyzer tests.
- Producer mutation: changing the production LocalHostName expectation from
  `Q` to `Q.local` made the Bash self-test fail at the stable identity guard;
  restoring exact `Q` returned it green.
- Consumer mutation: allowing `Q.local` in the analyzer's production identity
  regex made `test_environment_gate_requires_stable_q_identity` fail because
  the invalid evidence was accepted; restoring exact `Q` returned all 77 tests
  green.
- Full repository gates pass: rustfmt check, strict workspace clippy, and the
  complete workspace test suite.

## Coder dispute

None.

## Known gaps

Tactical review, additive staging, and a completed live arm/run remain.

## Reviewer comments

This finding came from the attached live launch, not a reviewer candidate.
