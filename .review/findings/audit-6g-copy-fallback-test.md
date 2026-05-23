# audit-6g-copy-fallback-test: fast-path → fallback coverage for copy_file

**Severity**: Test Gap
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `e06d23b`
**Parent finding**: `audit-6-test-gaps` (item 7).

## What

`crates/blit-core/src/copy/file_copy/mod.rs` `copy_file` chains
platform fast paths (macOS: clonefile → fcopyfile → buffered; Linux:
copy_file_range → sendfile → sparse → buffered) but had no test forcing a
fast-path failure to verify the chain advances and still copies
correctly.

## Approach (no production change)

Added a `fallback_tests` module:

- `copy_file_produces_byte_identical_copy` (all platforms): whatever fast
  path applies, the copy is byte-identical and `bytes_copied` matches.
- `copy_file_falls_back_to_fcopyfile_when_clonefile_cannot_apply`
  (macOS): a pre-existing destination makes `clonefile(2)` return
  `EEXIST`, deterministically forcing the first fast-path hop to fail.
  `fcopyfile` (opened with truncate, not `COPYFILE_EXCL`) then overwrites
  and the copy must still be byte-identical — a genuine fallback
  transition exercised with no production seam.

## Verification gap (flagged)

Forcing the chain all the way to the **buffered streaming tail** requires
`fcopyfile` to *also* fail, which has no benign deterministic trigger.
The chain is inlined and cfg-gated per OS, so testing the buffered tail
directly would need a production injection seam (e.g. an internal
`force_streaming` variant). That restructures a hot, multi-OS copy path
purely for a test, so it was not added. **Follow-up option** (audit-6g2,
if the reviewer wants full-chain coverage): add a `pub(crate)`
`copy_file` variant that takes a `force_streaming` flag short-circuiting
the fast-path booleans, and assert `clone_succeeded == false` + byte
identity. Note the Linux/Windows branches can't be compile-verified on
the darwin dev host (CARGO_FEATURE_PURE=1 only helps blake3, not the
cfg-gated copy branches).

## Files changed

- `crates/blit-core/src/copy/file_copy/mod.rs`: `fallback_tests` module
  (2 tests; one macOS-gated). No production change.

## Scope

One sub-item of audit-6. Remaining: 6a (blit-app), 6b (TUI render), 6c
(bridge HTTP integration), 6e (pull-move/push-move). 6d + 6f verified.

## Reviewer comments

(empty — pending review)
