# cr-ecm3-1: A daemon-to-daemon request with `--ignore-case` was not refused by daemons of the earlier contract

**Severity**: HIGH — a `mirror --ignore-case` between two older daemons copied what it was told to leave out and deleted what it was told to keep, exit 0
**Status**: Verified — `exclude-case-r4` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..0669279f`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f45ea7b1`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r3` over `3bc3ee47..467773b4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r3.codex.md`.

## Evidence
The reviewer built two daemons from the base commit (contract 8) and ran the head CLI against them. A direct transfer was refused for the version mismatch. A delegated `mirror --ignore-case --exclude '*cache'` exited 0, copied `aCache/source.txt` and deleted `bCACHE/saved.txt`: the two daemons negotiated contract 8 with each other, and the CLI's request — whose own `spec_version` was still 3 — was accepted with the new field unread.

Cause: the contract version gates the two ends of a SESSION. In a daemon-to-daemon transfer the session is between the daemons; the machine that asks only hands the destination daemon a request (`DelegatedPullRequest.spec`), and what gates that is the spec's own version (`SUPPORTED_SPEC_VERSION`, `crates/blit-core/src/remote/transfer/operation_spec.rs`, compared exactly). Slice 3 added `FilterSpec.ignore_case` inside that spec and bumped the contract, not the spec version. The constant's own history says why that matters (version 2, R51-F3).

Slice 3's record said of AC-6's last clause that "no build of contract 8 was run against this one". The reviewer ran one.

## Predicted observable failure
Until every daemon is rebuilt, `copy` or `mirror --ignore-case` between two daemons of the earlier build reads every pattern exactly at both of them, silently. Under `mirror` that deletes at the destination what the command was told to keep.

## Intake
Admitted.

## What
`SUPPORTED_SPEC_VERSION` is 4. A daemon of the earlier build refuses this build's request (`unsupported TransferOperationSpec spec_version 4 (expected 3)`), and this build's daemon refuses an earlier client's.

## Guard proof
- `remote::transfer::operation_spec::tests::a_spec_of_the_version_before_the_case_mode_is_refused`: a spec carrying the mode is accepted at this build's version and refused at version 3, the last that did not know the field; and the version, read at run time, is 4 or later.
- Mutation (the constant back to 3): the guard red; restored byte for byte.
- The reviewer's own run is the evidence for the failure; it has not been repeated here against base-built daemons after the fix.

## Files changed
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — the constant, its history, the guard.

## Known gaps
- Not run against daemons of the earlier build after the fix (the refusal is by exact comparison of a number those daemons already compare).
- The utility requests (`rm`, `find`, `du`, `df`, `ls`) have no such gate at all yet; that is `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` AC-11.
