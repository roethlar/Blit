# cr-ecm30-5: the local checksum comparison's open could wait on a FIFO

**Severity**: HIGH — a local `--checksum` copy compares a same-size destination again in the sink through `checksum::hash_file`, which opened it with a blocking `File::open`; a FIFO put there after the metadata check held the copy indefinitely
**Status**: Reviewed, found incomplete — `combined-r32` (Codex openreview, 2026-11-23) reviewed `93ecf7b2..714ed0b7` — the `combined-r31` round, the in-place rework of D-2026-11-23-1 among it — with `guard_confirmed: true`: "The Unix implementation removes the staging directory and the dependency on atomic exchange. Creating the partial directly also preserves the destination directory's group inheritance. I would keep that design and the shared nonblocking checksum opener." "The manifest checksum still opens its filesystem source through a blocking open. A local copy can also reach a blocking writable probe when its destination is a FIFO. Either can stall the transfer indefinitely. Check for a regular file at the actual nonblocking opens and carry the verified handle forward." The remaining work is cr-ecm31-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm30-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` over `fd603571..93ecf7b2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`. It completes cr-ecm29-4 (`0971c1f9`).

## Evidence
"A local `--checksum` copy without `--resume` can compare the destination again in the sink. That path still calls blocking `File::open`, so a FIFO appearing after its metadata check can stall the command. Share the opener across both comparisons." (`crates/blit-core/src/copy/compare.rs:168`, `crates/blit-core/src/checksum.rs:192`)

## Intake
Admitted as the reviewer put it: one opener, shared.

## What
- `checksum::open_regular_file` is the one opener for a file to be hashed. It opens with `O_NONBLOCK` on Unix and refuses what it opened unless that is a regular file.
- Its users:
  - `checksum::hash_file` — the local comparison in `copy::compare`, `check`'s, and the mirror planner's;
  - `checksum::partial_hash_first_last`;
  - the wire's `transfer_session::hash_file_abortable` (cr-ecm29-4's own opener is replaced by it).
- A FIFO, or anything else that is not a regular file, has no hash: the comparison says the file differs, and it is copied.

## Guard proof
- `checksum::tests::hashing_a_file_does_not_wait_on_a_fifo` (Unix): with a FIFO at the path and no writer, `hash_file` returns an error within 20 seconds.
- `transfer_session::tests::the_checksum_comparison_does_not_wait_on_a_fifo` (cr-ecm29-4's guard) now runs through the shared opener too.
- Mutations, each restored byte for byte:
  - the shared opener without `O_NONBLOCK` → red (both guards);
  - the shared opener hashing whatever it opened → red (both guards).
- The checksum, compare, check, session and mirror tests (226) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/checksum.rs` — `open_regular_file`, `hash_file`, `partial_hash_first_last`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `hash_file_abortable`.

## Known gaps
- Off Unix the open is as it was, and only the regular-file check is new. Windows has no FIFO at a file's path.
