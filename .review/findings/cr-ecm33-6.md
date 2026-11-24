# cr-ecm33-6: the exported path-based copies took a device for a file and could wait on a FIFO source

**Severity**: MEDIUM — `copy_file`, `resume_copy_file` and `mmap_copy_file`, public entry points of the platform crate, opened their source as any file. Each returned success for `/dev/null`, copying an empty file, and each could wait on a FIFO with no writer.
**Status**: Reviewed, found incomplete — `combined-r35` (Codex openreview, 2026-11-24) reviewed `0f912b06..dc14afc4` — the `combined-r34` fixes and the three prose fixes — with `guard_confirmed: true`: "The ordinary Linux copy now succeeds through a valid destination symlink and retains its writable handle. A Unix resume refuses a write when it can no longer establish the destination directory. The typed endpoint and resume-error classifications also fit the goal. The remaining problems are at identifiable points where the implementation drops a handle, a file identity, or a known cleanup result." "`mmap_copy_file` falls back to `std::fs::copy(src, dst)`, which resolves both names again. The macOS path copier also reopens the source and uses a blocking destination open for `fcopyfile`. A changed name can select a different file, and a FIFO can stall the copy. Use descriptor-based fallbacks and derive the length from the opened source." The remaining work is cr-ecm34-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0c98acb3`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r34.codex.md`. It completes cr-ecm32-3 (`6e52d386`).

## Evidence
"`copy_file`, `resume_copy_file`, and `mmap_copy_file` all returned success for `/dev/null` in my Linux fixture. Their source opens can also wait on a FIFO. Route these public entry points through the regular-file opener."

## Intake
Admitted as the reviewer put it.

## What
- `copy_file` (off Windows), `resume_copy_file` and `mmap_copy_file` open their source through `copy::open_regular`: without waiting, and only a regular file.
- `mmap_copy_file` also makes its destination through `copy::create_regular`.
- Anything else fails the copy.

## Guard proof
- `file_copy::mode_tests::the_path_based_copies_take_only_a_regular_source` (Unix):
  - `/dev/null` is refused by each of the three;
  - a FIFO no one writes is refused by `copy_file` within 20 seconds, in a thread of its own.
- Mutations, each restored byte for byte, one site at a time opening as any file → red (`copy_file`, `resume_copy_file`, `mmap_copy_file`).
- All 1,164 `blit-core` tests pass. Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — `copy_file`; guard.
- `crates/blit-core/src/copy/file_copy/resume.rs` — `resume_copy_file`.
- `crates/blit-core/src/copy/file_copy/mmap.rs` — `mmap_copy_file`.

## Known gaps
- On Windows `copy_file` opens its source as before (no FIFO at a file's path there).
