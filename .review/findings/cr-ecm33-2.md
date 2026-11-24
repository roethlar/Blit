# cr-ecm33-2: a local copy's cleanup held no writable handle, and left a source-sized file it could neither remove nor mark

**Severity**: HIGH — the Linux local copy armed its cleanup with the file's identity but not its handle. The copy gave the file a read-only source's mode (0444) through the handle. Then, in a directory the cleanup could not remove from (0555), a later failure (the metadata tail) left the cleanup unable to unlink the file. Reopening it by name to mark it failed (the file was read-only by then). The file was left at exactly its source's size, which a retry comparing sizes skips.
**Status**: Reviewed, found incomplete — `combined-r35` (Codex openreview, 2026-11-24) reviewed `0f912b06..dc14afc4` — the `combined-r34` fixes and the three prose fixes — with `guard_confirmed: true`: "The ordinary Linux copy now succeeds through a valid destination symlink and retains its writable handle. A Unix resume refuses a write when it can no longer establish the destination directory. The typed endpoint and resume-error classifications also fit the goal. The remaining problems are at identifiable points where the implementation drops a handle, a file identity, or a known cleanup result." "With `--ignore-existing`, an eight-byte read-only source, a directory that became unwritable after creation, and an injected metadata failure, cleanup left the destination at eight bytes. A later size-only comparison can skip it. Give this branch's cleanup the writable handle as well, while preserving its exclusive-name check." The remaining work is cr-ecm34-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `aa23acb1`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r34.codex.md`. It completes cr-ecm32-2 (`02522882`).

## Evidence
"With a `0444` source, a writable existing destination in a `0555` directory, and an injected metadata failure, cleanup could neither unlink the file nor reopen it for writing. It left eight bytes, exactly the source size, so a size-based retry can skip it. Capture identity during validation and give cleanup the still-writable handle."

## Intake
Admitted as the reviewer put it.

## What
- `PartialTarget::arm_opened` takes the handle the copy opened and holds it: its identity is taken then, and the cleanup has it.
- The copy writes through that held handle (`held_file`). The time is stamped through it too.
- On failure, the settling (`settle_failed_target`, as for a resume) works through the handle:
  - it removes the file only while it is what the name holds, or what a link there leads to;
  - what it cannot remove, it marks unfinished through the handle (one byte longer than the source) and reports left in place.
- The separate `opened` binding is gone.

## Guard proof
- `sink::tests::a_local_copy_marks_what_it_cannot_remove_through_its_handle` (Linux): a 0444 source of 8 bytes copied over a writable file in a 0555 directory, with the metadata tail refused. The file fails, its reason carries `INCOMPLETE_LEFT_IN_PLACE`, and it is 9 bytes, marked. Skipped where the process is not barred (root); here it ran.
- Mutation, restored byte for byte: the cleanup marking by name, not through what it holds → red (8 bytes).
- cr-ecm33-1's and cr-ecm32-2's guards still hold.
- All 1,162 `blit-core` tests pass. Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `PartialTarget::arm_opened`, `held_file`, `copy_and_stamp_opened`; guard.

## Known gaps
- macOS and Windows keep the by-name copy and its by-name cleanup (cr-ecm32-2's known gap).
