# cr-ecm34-2: an exclusive local copy's cleanup held no writable handle

**Severity**: HIGH — a retry can skip an incomplete file. A local copy that keeps what exists (`--ignore-existing`) makes its destination exclusively. Its cleanup knew which file it made, but the handle stayed with the copy only. With a read-only source, a directory barred once the file was made, and a failure in the metadata tail, the cleanup could neither remove the file nor open it again to mark it. It left it at its source's size, which a size-only comparison skips. cr-ecm33-2 gave the ordinary copy's cleanup its handle; this branch was left out.
**Status**: Verified — `combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true`: "The retained Linux copy handle, contract-v30 disposition flag, explicit display phase and separate unretried count are sound choices. The following gaps still affect what gets written, reported or retried."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `de291f55`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r35` over `0f912b06..dc14afc4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r35.codex.md`. It completes cr-ecm33-2 (`aa23acb1`).

## Evidence
"With `--ignore-existing`, an eight-byte read-only source, a directory that became unwritable after creation, and an injected metadata failure, cleanup left the destination at eight bytes. A later size-only comparison can skip it. Give this branch's cleanup the writable handle as well, while preserving its exclusive-name check."

## Intake
Admitted as the reviewer put it.

## What
On Linux, the exclusive branch of `copy_and_stamp_opened` hands what it made to the cleanup: `PartialTarget::arm_made_held`.
- The cleanup knows which file it made, as before. The name must be that very file: no link is followed, so the exclusive check is kept.
- The cleanup now also holds the handle, and the copy and the time stamp go through that held handle.
- What the cleanup cannot remove it marks unfinished through the handle, one byte longer than the source, and reports as left in place.

macOS and Windows keep the copy's own handle, as before.

## Guard proof
- `sink::tests::an_exclusive_copy_marks_what_it_cannot_remove_through_its_handle` (Linux): under `--ignore-existing`, a read-only source's copy fails in its metadata tail after its directory was barred, and is left nine bytes long, flagged left in place.
- The exclusive-copy tests (`a_name_made_by_a_run_that_keeps_what_exists_is_written_through_what_made_it`, `a_run_that_keeps_what_exists_writes_through_no_link`) still hold.
- Mutation, restored byte for byte: the copy writes through its own handle and the cleanup holds none (the fix undone) → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,082 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the exclusive branch; `arm_made_held`; guard.

## Known gaps
- macOS and Windows unchanged: their copies keep their own paths (cr-ecm32-2), and no host is reachable to prove a change there.
