# cr-jlfix1-2: `jobs watch` de-duplicates failed files by their flattened text

**Severity**: LOW (reviewer: LOW)
**Status**: Fixed — awaiting re-review
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

## What
- **`watch`:** de-duplicates failed files on their identity, `(path, raw)`, in a small `FailedNames` tally, never on how the name reads.
- **Text form:** `job_log::shown_name` puts `raw:` before a raw-byte name's escaped bytes, so it never reads like a UTF-8 name whose characters look like an escape. Both the text form of a log and `watch`'s list use it.

## Guard proof
- `jobs::tests::failed_names_are_told_apart_by_identity`: a raw-byte name and a look-alike UTF-8 name count and show as two, and a repeat counts once.
- `events_read_as_text` now expects `raw:caf\\xe9`.
- Mutations, each red, then restored:
  - a flattened key → the tally test. At the fix commit this mutation stayed green, because the label alone separated the test's two names. The follow-up test commit adds a UTF-8 file literally named `raw:caf\\xe9`, the collision the label cannot prevent, and the mutation is now red.
  - no `raw:` label → the text test
