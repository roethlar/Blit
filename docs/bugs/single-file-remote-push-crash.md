# Bug: `blit copy <file> server:/mod/` crashes during payload planning

**Status:** **RESOLVED.** Fixed across client and daemon. Regression tests in `crates/blit-cli/tests/remote_push_single_file.rs` pin the container-dir and rename cases. The fix touches both ends because `PathBuf::join("")` preserves a trailing `/` on Unix in multiple places:

1. Client side (`filter_readable_headers`, `build_tar_shard`): handle `rel.is_empty()` by using `source_root` directly instead of joining.
2. Daemon side: new `resolve_manifest_relative_path()` variant preserves empty-as-empty (the generic `resolve_relative_path` folds empty to ".", which breaks single-file pushes); new `resolve_dest_path()` helper does the same empty-handling when joining destination components.

**Component:** `blit-core` — `crates/blit-core/src/remote/transfer/source.rs` (`FsTransferSource`) and `crates/blit-core/src/remote/push/client/helpers.rs::filter_readable_headers`
**Severity:** Medium — transfer fails fast with a clear error and non-zero exit, so no silent data loss. But the user-facing form is `blit copy FILE REMOTE/` — a natural command that looks like it should "just work" — and the error message leaks internal terminology ("payload planning") that doesn't help the user recover.
**Reported against:** `blit-cli` v0.1.0 (confirmed present through commit `fe63b9a` — the local-side single-file fix in `execute_single_file_copy` does not reach the remote-push code path).

## Summary

When the source of a remote push is a **regular file** (not a directory), payload planning fails with:

```
opening /tmp/file.txt/ during payload planning: Not a directory
```

The local→local single-file path was fixed separately (see
`single-file-source-silent-noop.md` and the `execute_single_file_copy`
branch in `orchestrator/orchestrator.rs`). That fix short-circuits local
copies before they reach the enumeration/planner pipeline. The remote
push path (`run_remote_push_transfer` in `crates/blit-cli/src/transfers/remote.rs`)
has no equivalent short-circuit: it wraps the file path in
`FsTransferSource::new(path)` and calls `client.push(...)`, which
eventually reaches `filter_readable_headers` in
`crates/blit-core/src/remote/push/client/helpers.rs`.

## Reproducer

```console
$ rm -rf /tmp/repro && mkdir -p /tmp/repro
$ echo "hello world" > /tmp/repro/file.txt
$ blit-daemon --config /path/to/test-config.toml &  # module "test" → /tmp/dst/
$ blit-cli copy /tmp/repro/file.txt server:/test/ --yes
Error: negotiating push manifest for /tmp/repro/file.txt -> server:/test/

Caused by:
    opening /tmp/repro/file.txt/ during payload planning: Not a directory (os error 20)
```

Note the trailing `/` on the path in the error — a symptom of the root cause below.

Compare to the working directory case:

```console
$ blit-cli copy /tmp/repro/ server:/test/ --yes   # succeeds
```

And the (now-fixed) local case:

```console
$ blit-cli copy /tmp/repro/file.txt /tmp/dst/ --yes  # works after commit 6cf07bf
```

## Root cause (hypothesis)

`FsTransferSource::new(path)` stores `path` as its `root` without
distinguishing "file source" from "directory source". When the push
pipeline subsequently:

1. enumerates `root` and produces `FileHeader{ relative_path: "file.txt", ... }`, or
2. produces `FileHeader{ relative_path: "", ... }` (if the fix from
   the pull side's `sanitize_relative_path` has been extended) and
   `filter_readable_headers` computes `source_root.join(&rel)`,

…the resulting absolute path is either `/tmp/repro/file.txt/file.txt`
(double-nest, → "Not a directory") or `/tmp/repro/file.txt/` with a
trailing separator preserved by `PathBuf::join("")` on Unix (→
"Not a directory" on open).

The error line quotes `"/tmp/repro/file.txt/"` with a single trailing
slash, suggesting path (2) — the empty-`relative_path` variant.

## Suggested fix

Mirror the local-side strategy: detect file vs. directory at the
boundary of `run_remote_push_transfer` and take a dedicated file-source
path that emits exactly one `FileHeader{ relative_path: "", ... }` and
routes through the same pipeline, with `FsTransferSource::open_file`
handling the empty-path case by returning the file at `root` directly
(this pattern is already in `FsTransferSource::open_file` for the pull
side per commit `e70b21e`).

Concretely:

1. In `run_remote_push_transfer`, if `source` is `Endpoint::Local(p)`
   and `p.is_file()`:
   - Set `FsTransferSource` root to `p.parent()` (the containing directory)
   - Arrange for the pushed file list to contain only `p.file_name()` as
     the relative path
2. Or: extend `FsTransferSource` with a `Mode::SingleFile` variant that
   makes enumeration yield exactly one header with `relative_path = ""`,
   and update `filter_readable_headers` (and any peer in the push
   pipeline) to treat empty `relative_path` as "use root directly" —
   matching the pull-side convention already in place.

Option 1 is probably cleaner because it doesn't require the peer fix in
`filter_readable_headers`, just a one-time normalization at the CLI boundary.

## Regression test plan

Once fixed, add to `crates/blit-cli/tests/remote_transfer_edges.rs` (or
wherever remote push tests live):

```rust
#[test]
fn push_single_file_source_to_remote_directory() {
    // starts a daemon, copies a single file to a container destination,
    // asserts the destination receives the file under its basename
}
```

Also pin the idempotent case — re-running after a successful single-file
push should report `Up to date` (see the strengthened
`single_file_copy_idempotent` test for the local-side pattern).

## Related work

- `docs/bugs/single-file-source-silent-noop.md` — the local-side
  companion bug, fixed in commit `6cf07bf`.
- `docs/bugs/HANDOFF-remote-pull-single-file.md` — earlier handoff doc
  for the remote-pull single-file path. The pull side is now fixed
  (commit `e70b21e`); the push side is not.
- `crates/blit-core/src/remote/pull.rs::sanitize_relative_path` — the
  pull side's empty-path handling that the push side should mirror.
