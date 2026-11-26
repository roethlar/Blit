# cr-ecm43-1: an alias was not followed as the kernel follows it

**Severity**: HIGH — a copy and a resume write over a file the kernel would not open through that path. cr-ecm42-4's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm43-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r44` over `4e12d761..34c77a48` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r44.codex.md`.

## Evidence
"Preserve filesystem semantics through an alias. On Linux, ordinary copy and `--resume` both changed an existing file and then exited 2 when an inner symlink target ended in `/` or `/.`. The kernel returned `ENOTDIR` for that name. Separately, `create_regular_within` truncated a file through 43 links where the kernel returned `ELOOP`. Preserve the directory requirement, distinguish a missing leaf from other lookup errors, and count links across the entire walk."

The reviewer's probes (in the event stream): `dst/entry` → an outside `alias` → `dst/sub/leaf/` (or `/.`) — the system said `ENOTDIR` (`ENOENT` with no leaf), and blit wrote `leaf`; and `dst/entry` through 39, 40 or 42 outside aliases to `dst/leaf` — the kernel refused 41 and 43 links, and `create_regular_within` made or truncated `leaf` each time.

## Intake
Admitted as written.

## What
`real_path`, the real path of an alias's target, is resolved as the kernel resolves it — a name at a time from `/`:
- every link counted on the walk's own count (`alias_place` takes `&mut self` and passes `self.links`), refused past `MAX_FOLLOWED_LINKS` with `ELOOP`;
- a name a `/` or `/.` follows — as typed, or in a link's text — asked to be a directory (`ENOTDIR`);
- only a LAST name that is not there taken as missing; any other lookup error the kernel's own.

`alias_place` returns what the resolution refused as that error, and `None` only where the alias leads out. It had used `canonicalize`, with a count of its own begun at 0, and taken any error for a missing name.

## Guard proof
- `file_copy::…::an_alias_is_followed_as_the_kernel_follows_it` (Linux), with `openat2` and without, the kernel asked the same path for the answer (`fs::metadata`):
  - through an outside alias to `leaf/` and to `leaf/.`, `leaf` there or not: to make, to write and to resume are all refused, and `leaf` is as it was;
  - through 40 links in all, the file is made; through 41, and 43 to a file there, all three are refused and the file is untouched — where the kernel says `ELOOP`.
- Mutations, restored byte for byte, each → red:
  - a link's text asking for a directory taken for its name;
  - the alias's links counted apart from the walk's.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `real_path`, `Walk::alias_place`, `Walk::splice`.
- `crates/blit-core/src/copy/file_copy/mod.rs` — guard.

## Known gaps
- Off Linux the held-root walk is not taken (cr-ecm33-4, cr-ecm34-3's halves).
- A lookup error other than a missing name is now the kernel's, but no guard makes one (a directory on the way that cannot be searched is refused by the walk after it, either way).
