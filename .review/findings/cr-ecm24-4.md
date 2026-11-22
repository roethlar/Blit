# cr-ecm24-4: a destination this run could not read aborted the transfer

**Severity**: HIGH — an unreadable destination (`0004`) in a writable directory ended the whole pull while its hashes were taken, though its mode already said a `0600` source needed a fresh copy; older than this plan
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm24-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It was there at both of the reviewer's revisions.

## Evidence
"An unreadable `0004` destination in a writable directory aborted the entire pull during hash acquisition at both revisions. Its metadata already established that a `0600` source required a fresh copy. Select that fallback before opening for hashing, and revalidate the handle when reuse is possible."

## Intake
Admitted as the reviewer put it. A file that would be reused but cannot be opened to be read (its owner's own, `0200`) is sent whole too: its hashes cannot be had, and per D-2026-07-09-1 it is one file's matter, not the transfer's.

## What
`compute_resume_block_hashes` asks whether the file may be reused — owned by this run's user, granting others nothing its source does not — of what was seen of it before it is opened. One that may not be is sent whole without being opened. One that cannot be opened to be read is sent whole. The question is asked again of the file opened, which the hashes are of.

## Guard proof
- `own_incomplete_copy_tests::a_destination_this_run_cannot_read_is_sent_whole` (Unix, not as root): a `0004` destination and a `0200` one, each for a `0600` source — no hashes, `Whole`, no error.
- Mutation, restored byte for byte: a file that cannot be opened to be read ending the transfer (red).
- The resume, sink and session tests (310) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard.

## Known gaps
- The decision made before the open has no guard of its own. Once a file that cannot be opened is sent whole anyway, it saves only the open of a file that would be sent whole.
- A read that fails partway through the hashing still ends the transfer, as before.
