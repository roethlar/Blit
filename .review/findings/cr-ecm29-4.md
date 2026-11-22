# cr-ecm29-4: the checksum comparison's open could wait on a FIFO

**Severity**: HIGH — with `--checksum`, a same-size destination is opened for its checksum (before any resume hashing). If a FIFO took its place after the diff's look, that open waited for a writer, and the transfer with it, indefinitely
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm29-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r30` over `e3ea9517..fd603571` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r30.codex.md`. It completes cr-ecm28-1 (`8c696dc9`).

## Evidence
"With `--checksum --resume`, a same-size destination is opened for checksum comparison before resume hashing. If it becomes a FIFO, that earlier open can still stall the transfer indefinitely." The reviewer's required change: "Use the nonblocking regular-file open for checksum comparison too."

## Intake
Admitted as the reviewer put it. The open is the destination diff's, so it applies to every `--checksum` session, not only one with `--resume`.

## What
`transfer_session::hash_file_abortable`, the diff's checksum hasher, opens the destination with `O_NONBLOCK` on Unix, as cr-ecm28-1's opens do. It hashes only what it opened if that is a regular file. Anything else is an error, which the diff already takes as no checksum, so the file is sent.

## Guard proof
- `transfer_session::tests::the_checksum_comparison_does_not_wait_on_a_fifo` (Unix): with a FIFO at the destination's path and no writer, the checksum's open returns an error within 20 seconds.
- Mutations, each restored byte for byte:
  - the open made without `O_NONBLOCK` → red (it waited, and the test let it go with a writer);
  - what was opened hashed whatever it is → red.
- The resume, sink, session and checksum tests (362) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `hash_file_abortable`; guard.

## Known gaps
- Off Unix, the open is as it was. Windows has no FIFO at a file's path.
