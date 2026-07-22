# windows-move-tree-hang: blit move (push-tree) hangs on Windows CI

**Severity**: High — nested local-to-remote moves stalled on Windows
**Status**: Fixed; stale Windows ignore removed by the rel-3 reconciliation
**Branch**: `master`
**Fix commits**: `48c5a11` (original path correction), this rel-3 commit

## Symptom

`test_remote_move_local_to_remote_directory_tree` in
`crates/blit-cli/tests/remote_move.rs` consistently times out (>60s) on
the GitHub Actions Windows runner; the other three tests in
`remote_move.rs` (single-file push-move, single-file pull-move, and
the symmetric remote→local **directory-tree** pull-move) pass on
Windows. Linux + macOS pass all four. The failure has been observed
across at least 14 consecutive CI runs on `phase5/a1`, going back to
the original audit-6e directory-coverage commits (no run on the branch
has ever passed Windows CI).

## Root cause

The retained Windows log from GitHub run `26706097142`, job `78707535631`,
ended after the old daemon returned three needs: `a.txt`, `b.txt`, and
`nested\c.txt`. There was no source-delete output or failure. The old push
client keyed its manifest by canonical POSIX `nested/c.txt`, so the native
Windows echo missed the lookup. Top-level payloads could run, but the nested
file remained outstanding and both ends waited until the test's 60-second
process timeout. This is why flat/single-file moves passed and only the nested
push-tree case hung.

The handle-lifetime theory was wrong. The hang happened during transfer, not
in `remove_dir_all`.

## Fix and current path

`48c5a11` changed the old daemon need-list echo from
`PathBuf::to_string_lossy()` to `path_posix::relative_path_to_posix()`. The
owner's Windows host reproduced nested-push failures before that commit and
passed the nested push guards after it; the 10k forced-gRPC guard fell from a
300-second timeout to 0.77 seconds.

The later unified-session cutover deleted the old push controller entirely.
Current enumeration creates POSIX wire paths through `relative_path_to_posix`,
and destination diff returns `FileHeader.relative_path` directly in
`NeedEntry`; it never converts the wire identity back through a native
`PathBuf`. The exact old mismatch is therefore absent by construction.

Rel-3 removes the stale Windows ignore from
`test_remote_move_local_to_remote_directory_tree`. Its three-file nested tree
is the direct end-to-end guard. Local focused and workspace tests pass; the
exact current commit still needs the owner-gated hosted Windows run shared
with rel-1 confirmation.

## Cross-ref

The test itself was added by `audit-6e-move-directory-coverage`
(commit `5f92b66`) and `audit-6e round 2` (`6d410ac`). Both were
reviewer-verified on macOS; Windows CI was not gating the verdicts at
the time.
