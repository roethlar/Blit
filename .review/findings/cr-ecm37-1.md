# cr-ecm37-1: the destination's root was held by its name for each thread, not by the transfer

**Severity**: HIGH — a later transfer can write into a directory that is no longer its destination. Brought in by cr-ecm36-1. The Linux opener kept one held root descriptor per thread, found by the root's path, and a cache of directory places per thread, also by path. A worker reused for a later job, after the destination directory was renamed away and another made at its name, wrote into the retired directory.

The reviewer also named two places that went by the name:
- the directories a write needs were made by `create_dir_all`, which follows a directory on the way that has become a link out of the destination;
- where the kernel has no `openat2`, the opens went by the name.
**Status**: Reviewed, found incomplete — `combined-r39` (Codex openreview, 2026-11-24) reviewed `6ee7ed5e..86d69ee3` — the `combined-r38` fixes — with `guard_confirmed: true`: "The session-held directory, explicit failure index, structured unknown disposition and hash-based summary lookups are sound choices. I would keep them and require the following corrections." "If the destination is renamed and another directory takes its old name, the fallback used for internal absolute links or unavailable `openat2` can open the replacement. Directory creation and resume also use the pathname. Resolve, create and reopen relative to the session's held directory, including on fallback." The remaining work is cr-ecm38-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `6e49eb4a`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r38` over `2c55cb3a..6ee7ed5e` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r38.codex.md`. It completes cr-ecm36-1 (`d5378b09`).

## Evidence
"The thread-local root descriptor is reused by pathname. If a daemon processes another job after a destination directory is rotated and recreated, a reused worker can write into the retired directory. Hold the root per session, validate cached directories by identity, and use held directories for parent creation and fallback opens."

## Intake
Admitted as the reviewer put it, with one choice of means. "Validate cached directories by identity" is met by caching none. A file's place beneath the root is found from its path alone: beneath the name the transfer gave its root, or beneath the canonical root. The kernel walks that place from the root held on every open. Nothing is remembered that can go stale.

## What
- **`copy::Within`**: a transfer's destination root as the transfer holds it. It keeps:
  - the canonical root, for every check made by path;
  - the name the transfer gave it, and where that name lies beneath the canonical root;
  - on Linux, the directory itself, opened `O_PATH` when the transfer starts.

  `FsTransferSink` holds one for its life, and every write path takes it: streamed, single-file and tar-member writes, and the opener. The per-thread root descriptor and place cache are gone.
- **Opens** (`open_to_write`, Linux) go beneath the transfer's held root, by `openat2(RESOLVE_BENEATH)` from it. Where the kernel has no `openat2` (`ENOSYS`, `EPERM`), they no longer go by the name. They go by the destination's own resolution:
  - the chain followed;
  - its directory found within the root and held;
  - the file opened by its name there.

  A creation through that resolution makes nothing where something stands at the name. The same resolution serves cr-ecm37-4's `EXDEV`.
- **Directories** (`copy::create_dirs_within`, Linux):
  - each missing directory a write needs is found from the held root by `openat2` and made by `mkdirat` in the one found above it;
  - where the kernel cannot walk so (no `openat2`, or an absolute link on the way), the part that stands is first found within the root, and the rest is made by the name.

  The sink's three places that made directories by `create_dir_all` use it: the stream's parent readiness, a file payload's parent, and a tar member's parent.

  A directory that stands above the root is on the way to it, not a way out. The directory holding a destination that is one file is such a directory. The first draft refused it, and `remote_move`'s single-file pull failed: it is that case's guard.

## Guard proof
- `file_copy::mode_tests::each_transfer_writes_into_the_root_it_holds` (Linux): a root renamed away and another made at its name, on one thread.
  - A transfer that holds the new root writes into it, not the retired one.
  - The transfer that held the old root keeps to it.
- `file_copy::mode_tests::directories_are_made_beneath_the_held_root_only` (Linux):
  - a directory on the way that leads out has nothing made outside;
  - missing directories are made beneath the root;
  - through an absolute link within, they are made where it leads.
- `file_copy::mode_tests::without_openat2_the_opens_keep_within_the_root` (Linux, the kernel told it has no `openat2` through a test seam): a file and a directory under a directory that leads out are refused with nothing made outside; within the root both are made.
- `sink::cr_win_1_tests::a_member_under_a_directory_that_leads_out_makes_nothing_outside` (Linux): a tar member under a directory that leads out fails, and none of the directories it needs is made outside.
- Mutations, each restored byte for byte:
  - the root held by its name for each thread, as before → red;
  - directories made by the name → red;
  - without `openat2`, opens by the name → red;
  - the standing part of a directory not checked within the root → red;
  - a tar member's directories made by the name → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,115 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs`:
  - `Within`, `create_dirs_within`;
  - the opener beneath the held root;
  - the no-`openat2` resolution and its test seam;
  - the per-thread caches removed; guards.
- `crates/blit-core/src/copy/mod.rs` — exports.
- `crates/blit-core/src/remote/transfer/sink.rs`:
  - `FsTransferSink.within`;
  - the write paths take it;
  - the three places directories are made; guard.

## Known gaps
- Off Linux the opens and the directories made still go by the name, its checks made before (cr-ecm33-4's line). No Mac or BSD host is reachable here.
- A transfer whose root is renamed away while it runs keeps writing into the root it holds, by design.
