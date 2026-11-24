# cr-ecm35-6: an older run record's synthetic `(not retried)` entry was retried as a file

**Severity**: HIGH — a retry can copy an unrelated file and leave the run's own failure out, then exit 0. cr-ecm34-7 made every recorded failure a file. A record written before it may still hold the synthetic entry that counted the failures no retry pass was given, named `(not retried)`. Read now, that entry was taken for a file of that name.
**Status**: Reviewed, found incomplete — `combined-r37` (Codex openreview, 2026-11-24) reviewed `d2494965..2c55cb3a` — the `combined-r36` fixes — with `guard_confirmed: true`: "Exclusive creation, applying permissions through the open handle, and sharing failure accounting between the log and saved record are sound choices. The unretried count also works through the CLI. With one requested name over the wire budget, it reported one failure and one file not retried." "A complete daemon log naming a real file called `(not retried)` rebuilt the record with `failures_are_files = false`. A client without a local log then refuses a retry its daemon already identified. Set the flag when file events supply the replacement list." The remaining work is cr-ecm36-7
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `8735e2fa`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`. It completes cr-ecm34-7 (`727df1e0`) in part; the gap is the one its record named.

## Evidence
"Given a legacy-format fixture whose sole entry was the synthetic `(not retried)`, `jobs retry` copied an unrelated real file with that name, omitted the original failure and exited **0**. Mark the new record format and require corroborating log evidence before retrying an ambiguous legacy entry."

## Intake
Admitted as the reviewer put it. Per D-2026-11-24-1, the record says what it holds:
- A new record carries the marker that its failures are files.
- An older record has only the entry's name, which here is only a candidate. The run's log, which names files only (the synthetic entry was never logged), settles it.

## What
- **The marker.** `RunRecord.failures_are_files` is written `true` by every record this build starts. It is absent, so `false`, in a record from before.
- **The old entry is a candidate only.** In such a record an entry named `(not retried)` with no raw bytes is not taken for a file. `blit jobs retry` reads the run's log for it as for a short record:
  - A whole log that names a file by that name as failed makes it one.
  - A whole log that does not leaves it out, and the files the log names are retried.
  - Without a whole log, nothing is retried, and the refusal says why.
- **New records** are read as before: every failure is a file, whatever its name.

## Guard proof
- `jobs::retry_name_tests::an_old_records_count_entry_is_a_file_only_by_its_log` covers three cases for an old record:
  - its sole entry named `(not retried)`, with no log: refused;
  - a whole log naming `(not retried)` as failed: retried as that file;
  - a whole log naming only `real.txt`: `real.txt` retried, the entry left out.
- `a_failure_named_as_the_old_placeholder_is_a_file` (a new record) still holds.
- Mutations, each restored byte for byte:
  - the old entry taken for a file → red;
  - no refusal without a whole log → red;
  - an old record read as a new one → red;
  - a new record not marked → red (the new-record guard).
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,092 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/job_record.rs` — `RunRecord.failures_are_files`.
- `crates/blit-cli/src/jobs.rs` — the old entry settled by the log; guard.

## Known gaps
- A build older than this one cannot read a record this build writes: run records refuse unknown fields, as with every field added before.
