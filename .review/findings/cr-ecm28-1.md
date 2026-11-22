# cr-ecm28-1: the wire's opens of a destination could wait on a FIFO

**Severity**: HIGH — a regular destination that became a FIFO after the look, or after the hashing, could hold the hash reader or the sink's hold indefinitely before either checked the descriptor; the writable reopen could wait too
**Status**: Reviewed, found incomplete — `combined-r30` (Codex openreview, 2026-11-22) reviewed `e3ea9517..fd603571` — the five `combined-r29` fixes — with `guard_confirmed: true`: "I would keep the nonblocking opens for resume hashing, the Unix hard-link check, private staging, and permission rollback through the unfinished-length update." "With `--checksum --resume`, a same-size destination is opened for checksum comparison before resume hashing. If it becomes a FIFO, that earlier open can still stall the transfer indefinitely." The remaining work is cr-ecm29-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `8c696dc9`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r29` over `10145127..e3ea9517` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r29.codex.md`. It completes cr-ecm27-3 (`fa9e1dcf`) for the wire.

## Evidence
"If a regular destination becomes a FIFO after lookup or after hashing, the hash reader or the sink can wait indefinitely before either validates the descriptor. The writable reopen can also wait. Open nonblocking and verify that the descriptor is regular at each acquisition point."

## Intake
Admitted as the reviewer put it.

## What
- The destination's hashing (`compute_resume_block_hashes`) opens the file without waiting (`O_NONBLOCK`). What was opened must be a regular file, or the file comes whole. A seam (`AFTER_THE_HASHED_LOOK_HOOKS`) runs between the path's look and the open.
- The hashed hold (`hold_hashed`) opens for reading and reopens for writing without waiting. What it opened must be a regular file, or the file fails.

## Guard proof
- `own_incomplete_copy_tests::a_fifo_put_in_a_hashed_files_place_does_not_hold_the_hashing` (Unix): a FIFO put in the file's place after the look — the hashing ends within its bound, `Whole`.
- `sink::tests::a_fifo_put_in_a_hashed_files_place_does_not_hold_the_hold` (Unix): a FIFO in a hashed file's place before its hold — the hold ends within its bound, the file fails, the FIFO left.
- Mutations, each restored byte for byte: the hashing's open waiting (red); the hashed hold's open waiting (red).
- The resume, sink and session tests (337) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`, the seam; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_hashed`; guard.

## Known gaps
- The writable reopen's not waiting has no guard of its own. A FIFO there is a different file from the one read, which the identity check refuses.
