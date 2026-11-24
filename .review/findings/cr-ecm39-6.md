# cr-ecm39-6: the held-root walk kept a descriptor per directory, and every resume took it

**Severity**: MEDIUM — a deep resume fails. Brought in by cr-ecm38-2.
- **Every resume took the walk.** cr-ecm38-2's walk from the held root served every resume, ordinary paths among them.
- **One descriptor per directory.** It held a descriptor for each directory it had entered, to walk back a `..`.

With one worker, a 64-descriptor limit and a file 72 directories deep, the base completed the resume; the head failed with `Too many open files`.
**Status**: Reviewed, found incomplete — `combined-r41` (Codex openreview, 2026-11-24) reviewed `d7f5f5e9..b6e6fab9` — the `combined-r40` fixes — with `guard_confirmed: true`: "The per-failure identity and Linux `openat2` fast path are worth keeping. A 72-directory-deep resume completed with a 64-descriptor limit and produced the correct bytes. I would require the following changes." "After an in-root parent was renamed and its old name reused, `Walk::up` returned the replacement directory's inode. A subsequent write can land in the wrong directory. Verify the identity of the reacquired parent before continuing through `..`." "With `openat2` unavailable through the repository's test seam, path resolution succeeded but the new fast-path test failed because the fallback held two descriptors. Condition the fast-path assertion on syscall availability and retain the bounded-descriptor test for the fallback." The remaining work is cr-ecm40-3 and -7
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0f09ab1e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r40` over `86d69ee3..d7f5f5e9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r40.codex.md`. It completes cr-ecm38-2 (`fbb002d5`) in part.

## Evidence
"With one worker, a 64-descriptor limit, and a file 72 directories deep, the base completed the resume. The head failed with `Too many open files`. Use the kernel walk for ordinary paths and bound the descriptors retained by the fallback. It currently holds one per ancestor."

## Intake
Admitted as the reviewer put it.

## What
- **The kernel walks an ordinary path.** `resume::resolve_beneath` first asks the kernel for the directory, with one `openat2(RESOLVE_BENEATH)` from the held root, and looks at the name there. It takes the walk only for what that cannot serve:
  - a link at the name, to be followed;
  - an absolute link on the way;
  - no `openat2`.
- **The walk holds one directory at a time.** It keeps where it stands as the names of the directories entered, and the one directory it is in. A `..` is walked back by finding the parent again from the root: by the kernel's walk beneath it, or one name at a time, one held at once.

## Guard proof
Both use a path 72 directories deep (Linux).
- `file_copy::mode_tests::a_deep_walk_holds_one_directory_at_a_time`: with the kernel told it has no `openat2`, the walk resolves the path holding at most two directories at once. The count comes from a test-only counter of the directories a walk holds.
- `file_copy::mode_tests::an_ordinary_path_is_walked_by_the_kernel`: with `openat2`, the walk is not taken at all.
- Mutations, each restored byte for byte:
  - every directory on the way held (the old directory kept, never released) → red;
  - every path walked by the held-root walk → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,135 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `resolve_beneath`'s kernel path; the walk (`Walk`, `HeldDir`, `up`); the counter.
- `crates/blit-core/src/copy/file_copy/mod.rs` — guards.

## Known gaps
- The guards count the walk's own descriptors. They do not run the reviewer's 64-descriptor limit, which would need the test process's limit lowered for every test running beside it.
