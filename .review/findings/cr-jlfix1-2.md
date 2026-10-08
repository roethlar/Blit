# cr-jlfix1-2: `jobs watch` de-duplicates failed files by their flattened text

**Severity**: LOW (reviewer: LOW)
**Status**: Admitted — open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jlfix1-r1c over `7d2bd7d3..efe53362` (record `.review/results/jlfix1-r1c.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:600 — raw.unwrap_or(path) erases whether the displayed string came from exact escaped bytes; line 601 deduplicates that flattened value in HashSet<String>.

## Predicted observable failure
On Unix, an invalid-byte filename whose escaped raw value is, for example, caf\\xe9.txt and a valid UTF-8 filename literally containing those characters receive the same key; if both fail, jobs watch names only one and reports fewer distinct failures than the authoritative count.

## Reviewer's suggested approach
Deduplicate by a tagged key such as (path, raw), and label or otherwise render raw-byte names distinctly so the two files remain identifiable.

## Intake
Admitted. `watch` keyed its de-duplication on `raw.unwrap_or(path)`, so a raw-byte name whose escape reads like a literal UTF-8 name merges with it. Fix: de-duplicate on (path, raw); the text form labels a raw-byte name (`raw:` before its escaped bytes) so the two read differently.
