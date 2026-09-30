# cr-ssc4-2: Local source read errors leave a truncated or partial destination behind

**Severity**: MEDIUM — a contained local source read error leaves a truncated or partial destination file that a later size/mtime compare may treat as converged (silent corruption of a previously valid copy)
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `2322acb6`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b70ef03a..261912bb, record .review/results/ssc-4-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/sink.rs:1732 — `copy_opened` errors are propagated immediately after the destination may have been created or truncated; cleanup at lines 1744-1746 runs only for a successful copy followed by a size mismatch. On Linux, crates/blit-core/src/copy/file_copy/mod.rs:328 creates/truncates the destination before reading.

## Predicted observable failure
A mid-copy source I/O error is reported as contained, but the destination remains truncated or partially written. This can destroy a previously valid destination file; on paths that pre-size the destination, a later size/mtime comparison can even treat the corrupt file as converged.

## Reviewer's suggested approach
Guard non-resume destination mutation and remove the destination on every source-side copy or post-copy-stat error, committing the guard only after validation succeeds. Preserve the intentional in-place partial behavior only for resume copies.

## What
`copy_resolved_file_payload` holds an RAII `PartialTarget` guard from the moment a non-resume copy starts until validation and the metadata tail succeed; any exit in between removes the destination file. Resume copies keep their in-place partial (D-2026-07-09-1 Q2). A test-only after-copy fault seam stands in for a mid-copy source I/O error.

## Guard proof
Sink unit test: the injected fault leaves no destination file and is reported as `source: read error`. Mutation: guard never armed → the partial survives → red.

## Known gaps
The mid-copy fault is injected through a seam; a real mid-copy I/O error cannot be produced deterministically on a local filesystem.
