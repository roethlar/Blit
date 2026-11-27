# alias-path-too-long: an alias to a place deeper than `PATH_MAX` was refused where the kernel opens it

**Severity**: MEDIUM — a copy and a resume fail for a file the kernel reaches. cr-ecm43-1's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix alias-path-too-long`)
**Reviewer**: none — met by the reviewer's own probes in the event stream of `combined-r45` (Codex openreview over `34c77a48..ebc9d81c`, 2026-11-26; `.review/results/combined-r45.codex.events.jsonl`), which ended with no verdict when its service flagged the content; reproduced here by a guard; for the next openreview.

## Evidence
The reviewer's probe: beneath the root, eleven names of 200 bytes, then through an outside alias to that folder eleven more, and a file — 4,400 bytes below the root; an entry linked to it by the alias's path. `long_alias kernel_error=None blit_error=Some(Os { code: 36, kind: InvalidFilename, message: "File name too long" })`. cr-ecm43-1's resolution built the real path from `/` as a string, and asked the system about each step by that string.

## Intake
Admitted.

## What
**`real_place`** (in place of `real_path`): where an alias's target lies below the root, resolved as before a name at a time — but by descriptors, one directory held at a time, and the root known among the directories entered by its identity. No path is built from `/`, so none is too long; the place it gives is walked from the held root as any other.

## Guard proof
- `file_copy::…::an_alias_to_a_place_deeper_than_a_path_can_say_is_followed` (Linux), with `openat2` and without: a file there is written through the alias, and one is made there, where the kernel's own open succeeds.
- Against the resolution before this fix: red, `File name too long (os error 36)` — the reviewer's failure.
- cr-ecm43-1's and cr-ecm42-4's guards, on the new resolution: green.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `real_place`, `Walk::alias_place`.
- `crates/blit-core/src/copy/file_copy/mod.rs` — guard.

## Known gaps
- Off Linux the held-root walk is not taken (cr-ecm33-4, cr-ecm34-3's halves).
