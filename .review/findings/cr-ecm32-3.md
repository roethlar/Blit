# cr-ecm32-3: the scan's probe, the local copier's and the tar packer's source opens could wait on a FIFO

**Severity**: HIGH — the scan's readability probe, the local copy's source open and the tar packer's member open each used a blocking open. A FIFO with no writer, put in a file's place, held the transfer indefinitely. The packer also took a device (`/dev/null`) for a file.
**Status**: Reviewed, found incomplete — `combined-r34` (Codex openreview, 2026-11-23) reviewed `70efef8a..0f912b06` — the `combined-r33` fixes — with `guard_confirmed: true`: "Checking before truncation, opening FIFOs without waiting, and recording the resolved parent and name for a Unix resume are sound changes." "A FIFO arriving after the writable probe can still block `fcopyfile`'s destination open indefinitely. Keep the clone fast path for an absent destination and pass a validated, nonblocking descriptor to the fallback." "`copy_file`, `resume_copy_file`, and `mmap_copy_file` all returned success for `/dev/null` in my Linux fixture. Their source opens can also wait on a FIFO. Route these public entry points through the regular-file opener." The remaining work is cr-ecm33-4 and -6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `6e52d386`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r33` over `714ed0b7..70efef8a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r33.codex.md`. It completes cr-ecm31-4 (`374ed558`).

## Evidence
"The scan probe, local file copier, and tar packer still use blocking source opens. A FIFO without a writer can stall them. The tar opener also accepted `/dev/null` as a file in my check. On macOS, the `fcopyfile` fallback still opens its destination for blocking writes, so a FIFO arriving after the writable probe can stall that path too. Apply the regular-file check at each actual open and carry the verified handle forward."

## Intake
Admitted, on Unix. The macOS `fcopyfile` fallback is macOS-only code that nothing here can run. It is left as a known gap.

## What
- `copy::open_to_read` opens to read with `O_NONBLOCK` on Unix.
- `copy::open_regular` keeps what `open_to_read` opened only if it is a regular file. It is the one opener for a source a transfer reads. Its users:
  - the scan's probe (`open_for_scan`);
  - the local copy's source open (`copy_resolved_file_payload`), whose handle is carried through the copy as before;
  - the checksum's opener (`checksum::open_regular_file`, which now wraps it).
- The tar packer's member opener (`open_member_from_fs`) opens with `open_to_read`. It reads the metadata as its Stat stage, and refuses anything not a regular file as its Open stage, so a device is no member.
- Each refusal is that one file's failure.

## Guard proof
Each guard runs its open in a thread, so a wait cannot hold the test:
- `file_copy::mode_tests::a_source_opened_to_be_read_is_a_regular_file_had_without_waiting` (Unix): a FIFO no one writes is refused within 20 seconds, and so is `/dev/null`.
- `source::fifo_scan_tests::the_scans_probe_does_not_wait_on_a_fifo` (Unix).
- `sink::tests::a_fifo_at_a_local_source_does_not_hold_the_copy` (Unix): the copy ends and the file fails.
- `payload::member_open_tests::the_packer_takes_only_a_regular_file_without_waiting` (Unix): a FIFO and `/dev/null` are each refused.

Mutations, each restored byte for byte:
- the opener waiting → red (all four);
- the scan's probe opening as any file → red;
- the local copier opening as any file → red;
- the packer taking a device → red.

All 1,157 `blit-core` tests pass. Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — `open_to_read`, `open_regular`; guard.
- `crates/blit-core/src/checksum.rs` — `open_regular_file`.
- `crates/blit-core/src/remote/transfer/source.rs` — `open_for_scan`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `copy_resolved_file_payload`; guard.
- `crates/blit-core/src/remote/transfer/payload.rs` — `open_member_from_fs`; guard.

## Known gaps
- macOS's `fcopyfile` fallback still opens its destination for a blocking write. That is macOS-only code nothing here can run, so it is not changed.
