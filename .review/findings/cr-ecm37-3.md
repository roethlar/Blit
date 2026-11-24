# cr-ecm37-3: a resume record that closed where the destination could not be looked at reported nothing left

**Severity**: HIGH — a retry can keep a partial as a file the user put there.

When the source failed a resume record, the close looked up what stood at the name. Any failure of that lookup was taken for "gone". With the search permission of the file's directory taken before the close, the receiver reported `left_in_place = false` while the nine-byte partial for an eight-byte source stood. An `--ignore-existing` retry would then keep that partial.

And there was no way to say "not known" for one file: a failure's disposition was a bool on the progress lane and on the wire.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm37-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r38` over `2c55cb3a..6ee7ed5e` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r38.codex.md`. It completes cr-ecm36-5 (`e07ef48a`).

## Evidence
"Removing directory search permission before a resume record closed produced `left_in_place=false`. The nine-byte partial for an eight-byte source was still there. Carry lookup errors as an unknown disposition so an `--ignore-existing` retry cannot mistake that partial for a file to keep."

## Intake
Admitted as the reviewer put it. Per D-2026-11-24-1, "not known" travels as a structured field wherever a disposition does. It is never written into a reason for anything to read back.

## What
- **The close** (`FsTransferSink::fail_resume_record`) looks the name up once, resolving it and then reading its metadata. It reads the result in three ways:
  - the held file stands;
  - it is gone: the error's kind is `NotFound`, or another file is in its place;
  - not known: any other error.

  A record this run made, or whose held failure said it left its copy, takes that answer. A record that could not have left anything stays "nothing left". A failure that is not known is recorded as such (`SinkOutcome::failed_unknown`), and its reason says why for people.
- **The sink's outcome** keeps the paths whose disposition is not known, and how many failures said so (`left_unknown_holds`).
- **The progress lane.** `ProgressEvent::FileFailed.left_in_place` is an `Option<bool>`, so the job log's `FileFailed` event says `None` for such a file.
- **The wire.**
  - The report's `FileFailure` gains `bool left_unknown = 6`, so the contract goes to v32.
  - The summary takes its leftover account from one method, `SinkOutcome::wire_left_account`. It marks the account inexact when the report cannot carry a "not known" failure with its flag: past the report's cap, or with the path shortened.
- **The join.** In `RunTotals::failed_files`, a reported failure that is not known is joined as not known, unless the leftover lists name it.
- **The retry** (`PassFailures::pass_set`, the CLI). Under `--ignore-existing`, a failure that is not known is not given to the pass:
  - kept, its partial would stand as a file the user put there;
  - retried as a leftover, a file put there since would be overwritten.

  It is counted as not retried. Without the flag it is retried like any other.

## Guard proof
- `sink::cr_win_1_tests::a_resume_record_closing_unseen_says_not_known` (Unix, not as the superuser): a partial this run made, its directory's search permission taken before the close. The failure says not known, not left; the outcome holds it so, and the partial stands at nine bytes.
- `sink::cr_win_1_tests::an_unknown_past_the_report_makes_the_left_account_inexact`:
  - a "not known" failure in the report leaves the account exact;
  - one past the report's cap marks it inexact.
- `progress::progress_totals_tests::an_unknown_leftover_is_reported_not_known`: the per-file lane says `None`.
- `run_log::tests::a_kept_unknown_is_not_known`: the summary's join says `None`.
- `transfers::retry::tests::an_unknown_leftover_is_not_retried_under_ignore_existing`:
  - under the flag, the pass is given only the other failure and is marked short;
  - without it, both are given.
- Mutations, each restored byte for byte:
  - a lookup that fails taken for gone → red;
  - a "not known" past the report not marked → red;
  - the lane says known → red;
  - the join says known → red;
  - a "not known" retried under the flag → red.
- Checks: native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/proto/blit.proto` — `FileFailure.left_unknown`.
- `crates/blit-core/src/transfer_session/mod.rs` — contract v32; the summary takes `wire_left_account`; wire literals.
- `crates/blit-core/src/remote/transfer/sink.rs`:
  - `FileFailure.left_unknown`;
  - `SinkOutcome`'s "not known" set, `failed_unknown`, `left_unknown_holds` and `wire_left_account`;
  - the close; guards.
- `crates/blit-core/src/remote/transfer/progress.rs` — `ProgressEvent::FileFailed.left_in_place: Option<bool>`; guard.
- `crates/blit-core/src/run_log.rs` — the join; the progress event's disposition as given; guard.
- `crates/blit-cli/src/transfers/retry.rs` — `left_unknown`, `pass_set`; guard.
- Literals elsewhere: `payload.rs`, `transfers/failures.rs`, the CLI's `run_log.rs` and `failures.rs`, `tests/source_side_containment.rs`.

## Known gaps
- A "not known" failure the report cannot carry (past its cap) marks the whole leftover account inexact. Under `--ignore-existing`, every failure not known to be a leftover is then not retried in that run. This is safe, but coarser than one file.
- The guard on the close cannot run as the superuser, whom a directory's permissions do not bind. It says so and passes.
