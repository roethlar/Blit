# cr-ssc5-5: A successful raw-name mirror preserves an extraneous lossy-text counterpart

**Severity**: MEDIUM — a distinct lossy-text counterpart at the destination survives a successful raw-name mirror as an unreported extraneous file
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0322a708`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range e14d1b72..69390931 (ssc-5), record .review/results/ssc-5-range.codex.json)

## Evidence
crates/blit-core/src/mirror_planner.rs:260 — every lossy relative path is added to the keep set, and lines 263-267 additionally add its raw-byte path, treating both distinct filesystem names as present at the source.

## Predicted observable failure
On Linux, mirroring a sole source file caf\xe9.txt into a destination that also contains the distinct UTF-8 name caf�.txt leaves the latter untouched, so the mirror reports success while retaining an extraneous file.

## Reviewer's suggested approach
Model each manifest entry as one physical destination identity: use its raw path on byte-capable destinations and its text path otherwise. Preserve the text path separately only when an actual representable source entry has that name.

## What
Manifest intake sends raw-named entries to the planner as (text, bytes) and puts only representable names in the text keep set; the planner keeps a raw-named entry by its bytes where storable, by its text otherwise — never both.

## Guard proof
Planner unit pin in both capability branches; Linux mirror pin (proven on magneto): a distinct caf\u{FFFD}.txt beside the raw-named source file is deleted. Mutation: both identities kept → red.

## Known gaps
none
