# audit-6e-move-directory-coverage: directory-tree pull-move + push-move tests

**Severity**: Test Gap
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `6d410ac`
**Parent finding**: `audit-6-test-gaps` (item 5).

## What

audit-6 item 5 flagged missing remote→local / local→remote move tests.
That premise is **stale**: `remote_move.rs` already covers both
single-file remote directions (`test_remote_move_local_to_remote`,
`test_remote_move_remote_to_local`), `local_move_semantics.rs` covers
local→local, and `remote_remote.rs` covers remote→remote — all four
cardinal directions. The genuine remaining gap is **multi-file/directory
moves**: the recursive copy-then-delete-source path, where
partial-deletion data loss actually lives (the existing move tests only
move single files).

## Approach (no production change)

Added two integration tests using the existing `TestContext` daemon
harness:

- `test_remote_move_local_to_remote_directory_tree` — push-move a nested
  tree (`a.txt`, `b.txt`, `nested/c.txt`); asserts all files land at the
  module root AND the entire source tree is removed (source deleted only
  after the verified copy — the data-loss guard).
- `test_remote_move_remote_to_local_directory_tree` — pull-move a nested
  remote subtree; asserts all files arrive locally AND the remote source
  files are removed.

## Files changed

- `crates/blit-cli/tests/remote_move.rs`: 2 directory-tree move tests. No
  production change.

## Scope

Closes the last audit-6 sub-item. audit-6 (6a/6b/6c/6d/6e/6f/6g) is now
fully covered.

## Round 2 (commit `6d410ac`)

**Reopen finding:** the remote→local directory-move test asserted the
source files were unlinked but not that the nested `inner/` dir and the
`tree/` root were removed — asymmetric with the push direction (which
asserts the whole source tree is gone).

**Fix:** added the symmetric remote-side directory assertions
(`!remote_sub.join("inner").exists()`, `!remote_sub.exists()`). Verified
the pull-move does remove the empty source dirs, so it pins recursive
source-tree removal, not just file unlinking.

## Reviewer comments

(empty — pending round-2 grade)
