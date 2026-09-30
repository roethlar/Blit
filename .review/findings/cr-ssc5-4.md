# cr-ssc5-4: Mirror deletion does not shield descendants of a failed raw-name path

**Severity**: HIGH — the failed-path shield is built from lossy text only, so a failed raw-name source file whose destination path is a populated directory loses that subtree under mirror
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range e14d1b72..69390931 (ssc-5), record .review/results/ssc-5-range.codex.json)

## Evidence
crates/blit-core/src/mirror_planner.rs:266 — raw paths are added to the source keep set, but lines 274-277 build the failure shield exclusively from lossy text paths.

## Predicted observable failure
On a byte-keyed destination, if a raw-named source file fails to land while the destination path is an existing directory, the directory itself is kept but its children do not match the lossy shield and are deleted as extraneous. A contained per-file failure therefore causes destination data loss.

## Reviewer's suggested approach
Retain the raw identity associated with each manifest entry and add that raw path to the shield whenever the entry fails, including duplicate-collision failures.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
