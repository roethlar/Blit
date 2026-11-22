# cr-ecm16-1: a resumed copy on this machine left a file 644

**Severity**: HIGH — with `--resume`, a private file copied on its own becomes readable by all; a program stops being one
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm16-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It completes local-copy-mode (`619aa215`).

## Evidence
"On Linux, copying a single `0600` file returned success and produced `0600` normally but `0644` with `--resume`. I observed this with both an absent destination and an existing partial. Apply the source mode through the resume writer's handle before reporting success."

local-copy-mode put right the three copy functions that give a destination its source's time and mode. A file copied with `--resume` goes by a fourth, `copy::resume_copy_from`, which patches the destination in place block by block and never gave it a mode at all: it kept the one it was created with, or the one a partial left before had.

## Intake
Admitted as the reviewer put it.

## What
`resume_copy_from`, once the patch is complete and the destination cut to the source's length, gives it the source's mode through its own handle, taken from the source's own opened handle (Unix). Nothing else in the resume changes.

## Guard proof
- `copy::file_copy::resume::tests::a_resumed_copy_has_its_sources_mode`: modes 600, 640 and 750, into a file the resume makes and into a partial of mode 644 left before.
- `crates/blit-cli/tests/local_copy_mode.rs::a_local_copy_gives_every_file_its_sources_mode` (extended): the copy with `--resume` too — small and large files of a tree and a tree of one file.
- Mutation, restored byte for byte: the resumed copy given no mode, as it was (both guards red).

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — the mode; guard.
- `crates/blit-cli/tests/local_copy_mode.rs` — guard (each run lands in a directory of its own; two runs shared one before).

## Known gaps
- The mode is set best-effort, as the other three copy functions set it: a failure is logged at debug level and the copy succeeds.
- A resume interrupted part-way leaves its partial with whatever mode it had; the mode is given when the copy completes.
- Unix only; on Windows the resume is as it was.
