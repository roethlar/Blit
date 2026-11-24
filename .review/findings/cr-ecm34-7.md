# cr-ecm34-7: the unretried count rode a failure named `(not retried)`

**Severity**: MEDIUM — a display string used as an identity (D-2026-11-24-1). When more files failed than the report could name, the retry loop carried the ones no pass was given as a synthesized failure entry whose path was `(not retried)`. `blit jobs retry` discarded any failure of that name as the placeholder, so a real file named `(not retried)` was discarded with it.
**Status**: Reviewed, found incomplete — `combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true`: "The retained Linux copy handle, contract-v30 disposition flag, explicit display phase and separate unretried count are sound choices. The following gaps still affect what gets written, reported or retried." "Given a legacy-format fixture whose sole entry was the synthetic `(not retried)`, `jobs retry` copied an unrelated real file with that name, omitted the original failure and exited **0**. Mark the new record format and require corroborating log evidence before retrying an ambiguous legacy entry." "With one requested name over the wire budget and one retry enabled, the CLI returned exit 2 with `files_failed = 1` and `files_not_retried = 0`. Account for the unnamed remainder before the empty-set exit." The remaining work is cr-ecm35-6 and -7
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `727df1e0`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r35` over `0f912b06..dc14afc4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r35.codex.md`.

## Evidence
"A real file named `(not retried)` was the sole failure in my fixture. With its log unavailable, `jobs retry` discarded the recorded filename and reported that it knew zero of one failed files. Generate the label when rendering the count."

## Intake
Admitted as the reviewer put it. It is the owner's rule: a display string is never an identity.

## What
The count stays a count:
- `PassFailures.unretried`, already exact, is the only carrier.
- `mark_retried` no longer adds a failure entry, and `UNRETRIED_PATH` is gone.

Rendering:
- The shared failure block takes the count. It writes "N file(s) were not retried: the retry set could not name them all" on a line of its own.
- The count is not elided again.
- Each route's `--json` document gains `files_not_retried`.
- The four print entry points take the count from the retry loop's outcome: local, push, pull and delegated. Paths without a retry pass give 0.

`blit jobs retry`:
- Every recorded failure is a file, whatever its name.
- A record is short when its list is truncated or names fewer files than it counts. The unretried ones are counted and not named, so such a record still reads its log, as before.

## Guard proof
- `jobs::retry_name_tests::a_failure_named_as_the_old_placeholder_is_a_file`: a record whose one failure is the file `(not retried)` is whole; its retry asks for that file, and no log is read.
- `transfers::failures::tests::the_unretried_count_is_its_own_line`: a failure of the file `(not retried)` is listed like any other, the count is its own line, and the elided remainder excludes it.
- The retry loop's tests now assert the count and no synthesized entry. The CLI test `retry_pass` still finds "2 file(s) were not retried" on stdout.
- Mutations, each restored byte for byte:
  - the placeholder's name discarded → red;
  - the count not said → red;
  - the count elided again → red.
- Checks:
  - native clippy is clean;
  - the CLI package passes: 508 tests over 44 binaries, Linux.

## Files changed
- `crates/blit-cli/src/transfers/retry.rs` — `mark_retried`; `UNRETRIED_PATH` removed; tests.
- `crates/blit-cli/src/transfers/failures.rs` — the block renders the count; guard.
- `crates/blit-cli/src/transfers/{local,remote,remote_remote_direct,mod}.rs` — the count to the renderers and the JSON documents.
- `crates/blit-cli/src/jobs.rs` — no name is discarded; guard.
- `CHANGELOG.md` — the count and its JSON key.

## Known gaps
- **Old records.** A run record written before this change, by a run whose retry set was cut short, lists the placeholder among its failures. Read now, that entry is a file named `(not retried)`. Its retry asks for that file too, and reports it not found. A whole-looking count could then skip the log that names the real remainder. Such a record's list was cut short, which normally marks it short, so its log is read anyway. Telling the entry apart would mean reading its name, which the owner's rule forbids. 1.0.0 is not tagged, so no released build wrote one.
