# cr-ecm37-6: a record an earlier blit saved with a shortened rendering was retried as two files

**Severity**: MEDIUM — a retry asks for a file that does not exist and reports a failure that never happened. Before cr-ecm36-3, a run whose failure had a long name saved two entries for it in its record:
- the full name, from the failed-path list;
- the report's shortened rendering of that name.

The record counted one failure. The head retried both entries and reported two failures.
**Status**: Reviewed, found incomplete — `combined-r39` (Codex openreview, 2026-11-24) reviewed `6ee7ed5e..86d69ee3` — the `combined-r38` fixes — with `guard_confirmed: true`: "The session-held directory, explicit failure index, structured unknown disposition and hash-based summary lookups are sound choices. I would keep them and require the following corrections." "A truncated record counting one failure yielded two retry names when its log supplied both the full and shortened spellings. Require the recovered identity count to equal the recorded failure count after merging the log." The remaining work is cr-ecm38-7
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `1bc7aaa9`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r38` over `2c55cb3a..6ee7ed5e` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r38.codex.md`. It completes cr-ecm36-3 (`f29d3dc7`) in part.

## Evidence
"I had the base binary save one failed long name. It stored both the full name and its shortened rendering. The head retried both and reported two failures. When the identity count exceeds the recorded failure count, require corroborating exact identities or report that the retry is ambiguous."

## Intake
Admitted, taking the reviewer's second branch: the retry is refused as ambiguous.
- **Why not corroborate.** No exact identity is there to be had. The build that saved such a record wrote its log's end of run by the same join, so the log names the rendering as a file too.
- **Why not filter.** Telling a rendering from a file by its words is barred by D-2026-11-24-1.

A current record names each failed file once, so it never names more than it counts.

## What
`failed_identities` (`blit-cli/src/jobs.rs`) checks the record's own entries before anything else. When it names more files than its count of failures, nothing is retried, and the refusal says why:
- a record from an earlier blit can name a long file twice, once by a shortened rendering of its name;
- which entry is the file cannot be told;
- the whole job should be run again, with how.

## Guard proof
- `jobs::retry_name_tests::a_record_naming_more_files_than_it_failed_is_not_retried`:
  - a record counting one failure that names a long file and its rendering is refused;
  - the same record with one entry is retried as that file.
- Mutation, restored byte for byte: such a record retried → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,122 tests over 57 binaries.

## Files changed
- `crates/blit-cli/src/jobs.rs` — the check in `failed_identities`; guard.

## Known gaps
- Such a record's retry can only be had by running the whole job again.
