# local-copy-mode: a file copied on its own between two local paths lost its mode

**Severity**: HIGH — a private file copied on this machine becomes readable by every user; a program stops being executable
**Status**: Reviewed, found incomplete — `combined-r17` (Codex openreview, 2026-11-21) reviewed `13faafd2..7912da16` — the nine `combined-r16` fixes and local-copy-mode — with `guard_confirmed: true`; the reviewer's own guard was this fix's (the old assumption restored: `0644` where `0600` was required; the source restored, green). "On Linux, copying a single `0600` file returned success and produced `0600` normally but `0644` with `--resume`. I observed this with both an absent destination and an existing partial." The remaining work is cr-ecm16-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `619aa215`
**Reviewer**: none — found by the working agent, 2026-11-21, while measuring the speed of cr-ecm15-1's fix: a guard written for that fix expected a copied file's mode and failed. No review has seen it. It is older than this session's work (the code is `copy_opened`'s since ssc-4, and `copy_file`'s before it); when it began was not traced.

## Evidence
Measured, Linux, umask 022, tmpfs and Btrfs alike — GNU cp 9.12 and rsync 3.5.2 beside `blit copy` (the build at `b2a9e435`):

```
cp -r                              small600:600 small640:640 big600:600 big750:750
rsync -r                           small600:600 small640:640 big600:600 big750:750
blit copy, this machine            small600:600 small640:640 big600:644 big750:644
blit copy, a tree of one file      alone600:644
blit copy, to a daemon             small600:600 small640:640 big600:600 big750:750
blit copy, from a daemon           small600:600 small640:640 big600:600 big750:750
```

Cause. A file that is copied on its own on this machine goes through `copy::copy_opened`, which gives the destination its source's time and mode `if !clone_succeeded` — on the reasoning that a clone carries them. On Linux the fast paths that copy most files, `copy_file_range` and `sendfile`, report themselves as clones and carry bytes and nothing else. The destination kept the mode it was created with. (macOS' `clonefile` and `fcopyfile` do carry them; small files are written from a shard, which sets the mode itself; the wire sets it from the manifest.)

## Intake
Admitted by the agent that found it. Not a review finding. Fixed without being asked, because the fault is a private file made readable by all, with the measurement beside `cp` and `rsync` to say what is right.

## What
`A_CLONE_CARRIES_METADATA` — true where a copy reported as a clone is one (macOS, Windows), false on Linux and the other Unixes — and the three places that give a destination its source's time and mode (`copy_file`, `copy_opened`, and cr-ecm15-1's `copy_opened_into`) now do so unless a clone that carries them was made. Nothing else changes: the mode given is the source's own, as it already was for every file not copied by a fast path.

## Guard proof
- `copy::file_copy::mode_tests::a_copied_file_has_its_sources_mode_however_it_was_copied`: modes 600, 640 and 750, a file of 256 KiB, through each of the three functions.
- `crates/blit-cli/tests/local_copy_mode.rs::a_local_copy_gives_every_file_its_sources_mode` (end to end, the measurement above as a test): small and large files of a tree and a tree of one file, with no option, with `--ignore-existing` and with `--checksum`.
- Mutations, each restored byte for byte: a fast copy taken to carry the mode, as it was (both guards red); the same for the copy through a held destination alone (both red); for the copy by its opened source alone (both red).

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — the three conditions; guard.
- `crates/blit-core/src/copy/file_copy/metadata.rs` — a comment put back on the function it describes (cr-ecm15-1 had inserted a function beneath it; no code).
- `crates/blit-cli/tests/local_copy_mode.rs` — guard (a new test binary: 56).
- `CHANGELOG.md`, `REVIEW.md`.

## Known gaps
- The mode given is the source's own, not the source's less the umask: a source of mode 666 lands 666, where `cp` and `rsync -r` give 644. That is what blit already did for every other file, and is not changed or judged here.
- Owner and group, and extended attributes, are not carried by a local copy; not measured here.
- Not run on macOS or Windows; nothing changes there by construction.
- Whether any release has the fault was not traced.
