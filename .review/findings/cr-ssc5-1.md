# cr-ssc5-1: Remote lossy-name collisions can transfer the second file under the first file's name

**Severity**: HIGH — on a lossy-name collision the source retains the second header (last-wins) while the destination granted the first; the second file's bytes can be written under the first file's name, or the session aborts instead of containing the duplicate
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range e14d1b72..69390931 (ssc-5), record .review/results/ssc-5-range.codex.json)

## Evidence
crates/blit-core/src/transfer_session/mod.rs:1936 — the source manifest map uses unconditional insert, so the second colliding header replaces the first, while lines 4332-4343 make the destination retain and grant the first header.

## Predicted observable failure
When two source names collapse to the same UTF-8 text and needs are emitted after both headers, the source resolves the need to the second file. If metadata matches, its bytes are silently written under the first file's raw name; if metadata differs, the session aborts instead of containing the duplicate.

## Reviewer's suggested approach
Make source-side retention first-wins as well, preferably with an explicit seen-path set, and add a test using colliding headers with different raw names and contents.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
