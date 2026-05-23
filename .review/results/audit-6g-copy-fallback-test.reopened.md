Reviewed sha: `e06d23bc13858f5bd90caa34c58097401347e1cd`

Reopened.

The macOS fallback test does not actually prove the claimed `clonefile` -> `fcopyfile` transition. `copy_file_falls_back_to_fcopyfile_when_clonefile_cannot_apply` pre-creates the destination, so `clonefile` should fail with `EEXIST`, but the assertions only check `bytes_copied` and byte identity:

- `crates/blit-core/src/copy/file_copy/mod.rs:276-282`

If `attempt_fcopyfile_macos` were broken and returned `false`, `copy_file` would fall through to the buffered streaming tail, still produce byte-identical output, and this test would pass. That leaves the intended fast-path fallback hop unpinned.

Expected fix: in the macOS-gated test, assert `outcome.clone_succeeded` is true after the pre-existing-destination copy. With `clonefile` forced to fail, `clone_succeeded == true` proves the next fast path (`fcopyfile`) handled the copy rather than the buffered tail.
