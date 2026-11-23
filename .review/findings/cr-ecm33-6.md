# cr-ecm33-6: the exported path-based copies took a device for a file and could wait on a FIFO source

**Severity**: MEDIUM — `copy_file`, `resume_copy_file` and `mmap_copy_file`, public entry points of the platform crate, opened their source as any file. Each returned success for `/dev/null`, copying an empty file, and each could wait on a FIFO with no writer.
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm33-6`)
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
