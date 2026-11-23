# cr-ecm31-4: the source's open and the local destination's writable probe could wait on a FIFO

**Severity**: HIGH — the source opened each file to be sent with a blocking open, and a local copy probed its destination with a blocking open for writing, then made it with another (`File::create`). A FIFO at either place — put in a source file's place after the scan, or standing at the destination — held the transfer indefinitely.
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm31-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r32` over `93ecf7b2..714ed0b7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r32.codex.md`. It completes cr-ecm30-5 (`b0c50a2c`).

## Evidence
"The manifest checksum still opens its filesystem source through a blocking open. A local copy can also reach a blocking writable probe when its destination is a FIFO. Either can stall the transfer indefinitely. Check for a regular file at the actual nonblocking opens and carry the verified handle forward." (`crates/blit-core/src/remote/transfer/source.rs:542`, `crates/blit-core/src/remote/transfer/sink.rs:3542`)

## Intake
Admitted. Each open that meets the FIFO is made without waiting and goes on only with a regular file.

The verified handle is carried as far as each open's own use: the source's handle is what is sent, and the made destination is what is written. The probe still precedes `copy_opened`'s own making of the file, and that making is now guarded the same way.

## What
- `FsTransferSource::open_file` (every source file sent, the manifest's checksum among them): opened with `O_NONBLOCK` on Unix, and refused unless it is a regular file.
- `probe_destination_writable` (the local copy):
  - opened for writing with `O_NONBLOCK` on Unix;
  - a FIFO no one reads (`ENXIO`) or anything that is not a regular file fails the file, left as it is;
  - nothing there is still fine.
- `copy::create_regular`: one maker of a file to write from its start. It opens with `O_NONBLOCK` on Unix and keeps the file only if it is regular. It replaces `File::create` in `copy_file` and `copy_opened` (all four places) and in the sink's `create_to_write`.
- `CHANGELOG.md`: a FIFO or a device where a copy expects a file fails that file.

## Guard proof
- `source::tests::a_fifo_at_a_sources_name_does_not_hold_its_open` (Unix): a FIFO at a source's name is refused within 20 seconds.
- `sink::tests::a_fifo_at_a_destination_does_not_hold_the_copy` (Unix):
  - a local copy onto a FIFO with no reader ends within 20 seconds, the file failed and the FIFO left;
  - `create_to_write` and `create_regular` each refuse a FIFO at once.
  - Each runs in its own thread, so a wait cannot hold the test.
- Mutations, each restored byte for byte:
  - the source's open waiting → red;
  - the destination's probe waiting → red;
  - the maker of the file to write waiting → red.
- All 1,150 `blit-core` tests pass. Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/remote/transfer/source.rs` — `open_file`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `probe_destination_writable`, `create_to_write`; guard.
- `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — `create_regular`, its uses.
- `CHANGELOG.md`.

## Known gaps
- macOS's clone paths (`fclonefileat`, `fcopyfile`) still go by the destination's name. Nothing here can run them.
- A FIFO with a reader at a destination is opened for writing an instant before it is refused. The reader sees an empty write end close.
