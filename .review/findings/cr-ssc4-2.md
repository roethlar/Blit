# cr-ssc4-2: Local source read errors leave a truncated or partial destination behind

**Severity**: MEDIUM — a contained local source read error leaves a truncated or partial destination file that a later size/mtime compare may treat as converged (silent corruption of a previously valid copy)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b70ef03a..261912bb, record .review/results/ssc-4-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/sink.rs:1732 — `copy_opened` errors are propagated immediately after the destination may have been created or truncated; cleanup at lines 1744-1746 runs only for a successful copy followed by a size mismatch. On Linux, crates/blit-core/src/copy/file_copy/mod.rs:328 creates/truncates the destination before reading.

## Predicted observable failure
A mid-copy source I/O error is reported as contained, but the destination remains truncated or partially written. This can destroy a previously valid destination file; on paths that pre-size the destination, a later size/mtime comparison can even treat the corrupt file as converged.

## Reviewer's suggested approach
Guard non-resume destination mutation and remove the destination on every source-side copy or post-copy-stat error, committing the guard only after validation succeeds. Preserve the intentional in-place partial behavior only for resume copies.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
