# cr-uv1-3: One `rm` operand's failure was charged to all, and what had been done was not reported

**Severity**: HIGH — a file was deleted and reported as not removed; another, removable, was left
**Status**: Verified — `utility-slices-r2` (Codex openreview, 2026-11-21) reviewed `4841b2d4..3cd49e40`, the seven `utility-slices-r1` fixes, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `3b2d6b1b`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r1` over `b55efff7..49350572` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r1.codex.md`.

## Evidence
The reviewer supplied `first`, `blocked/entry`, and `last`, with `blocked` unreadable. Blit deleted `first`, reported all three as failures, and left `last` in place. GNU removed both accessible files. A containment-check error currently discards the accumulated reply. Record that refusal against its operand, retain earlier successes, and process the remaining operands.

Cause. `remove_as_rm` checked the operand's place by its path before reading anything and returned an error for the REQUEST when the check failed — and the check fails not only for a path that leaves the module but for one it cannot resolve, below a directory that may not be searched. The daemon turned that into a failed request; the reply built so far, with `first` already removed, was dropped, and the CLI charged the request's error to every operand of it. The daemon's reading of the operands did the same for an operand that cannot be one (the module's root, a `..`): the whole request refused.

## Predicted observable failure
Any `blit rm` of several paths where one lies below an unsearchable directory, or leaves the module through a link: the paths before it are deleted and reported as failures, the paths after it are not attempted.

## Intake
Admitted as the reviewer put it.

## What
- **Where an operand lies is that operand's question.** `remove_as_rm` no longer fails the request for it. On Unix the operand's directory is resolved and opened from the root down (cr-uv1-2); what that meets — `Permission denied` for a directory that may not be searched, `the path leaves the module` for one outside — is the operand's failure, in the system's words where the system gave them. Elsewhere the check by path is kept and its refusal recorded the same way. `Err` is left for a cancelled operation.
- **An operand that cannot be one is its own failure** (`read_operands`): the module's root, a path with `..` in it. The request goes on to the others. (The CLI still refuses the whole command for the module's root before anything is sent, AC-8.)
- The reply is built operand by operand and nothing in it is dropped.

Against GNU `rm`, through the binary: `rm t/file.txt t/sealed/entry t/other.txt` with `t/sealed` unsearchable prints `cannot remove 't/sealed/entry': Permission denied`, exits 1 and removes both files, as `rm` does; `rm -rf t/sealed/entry t/dir t/sealed` likewise equals it.

## Guard proof
- `blit-daemon` `service::admin::rm_tests::an_operand_that_cannot_be_reached_does_not_end_the_request` (Unix): the reviewer's three operands — one failure, for operand 1, `Permission denied`; two entries removed; `first` and `last` gone.
- `service::admin::rm_tests::an_operand_that_cannot_be_one_is_its_own_failure`: `first`, `../victim`, the empty path, `.`, `last` — three failures, each with its operand's place, and both files gone; and (Unix) an operand through a link out of the module: `the path leaves the module`, the next operand removed, the file outside untouched.
- `deletion::rm_tests::a_target_outside_the_root_is_a_failure_of_that_operand`: by its own path and through a link, with and without `-f` — an answer, one failure, nothing touched; and (Unix) below an unsearchable directory, `Permission denied`.
- `rm.rs::rm_equals_the_system_rm` (Linux): the two commands above, lines, exit status and what is left compared with the system `rm`'s.
- `rm.rs::a_daemon_refuses_rm_from_another_contract`: the module's root asked for directly is an answer with that operand's failure, nothing removed.
- Mutations, each restored byte for byte: an operand that cannot be placed ends the request, as before → the first three guards and `rm_equals_the_system_rm` red; an operand that cannot be one ends the request → the second and the last.

## Files changed
- `crates/blit-core/src/deletion.rs` — `remove_as_rm`, `leaves_the_root`; the guard.
- `crates/blit-daemon/src/service/admin.rs` — `read_operands` (in place of `sanitize_request_paths`), `RmOperand`, `remove_operands_sync`; the guards.
- `crates/blit-daemon/src/service/core.rs` — the call.
- `crates/blit-cli/tests/rm.rs` — the cases; the root's refusal as an operand's failure.
- `CHANGELOG.md`.

## Known gaps
- A request the daemon refuses whole — a read-only module, another build, no answer — is still charged to each of its operands by the CLI: none of them went.
- `DeletionError::Containment` is no longer produced by `remove_as_rm`; the mirror's delete pass still uses it.
- On Windows the place of an operand is checked by its path and any refusal reads `the path leaves the module`, including one the system would have worded otherwise. Not run on Windows.
- Run on Linux only.
