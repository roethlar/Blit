# log-named-text: the run log and the saved record named a text failure by the bytes heard for its spelling

**Severity**: HIGH — a saved retry asks for a file that landed and reports success while the one that failed is still missing. Found by the working agent 2026-11-24 while fixing cr-ecm38-3; no review has it. It is the same misnaming as cr-ecm37-2, in a later place.

Two places gave a failure that the account names by its text alone the bytes the run's log had heard for that spelling (`RawName` events):
- the CLI's record writer, through `failed_raw_names`;
- the run log's end pass, through `active.raw_names`.

Those bytes belong to another file, a sibling of that spelling the run sent. In `combined-r38`'s case, a vanished `dup` + U+FFFD was still saved as `dup` + byte `FF` after cr-ecm37-2 and cr-ecm38-1. `jobs retry` would ask for the sibling, which had landed, and exit 0.
**Status**: Reviewed, found incomplete — `combined-r40` (Codex openreview, 2026-11-24) reviewed `86d69ee3..d7f5f5e9` — the `combined-r39` fixes and log-named-text — with `guard_confirmed: true`: "The held directory, explicit links from diagnostics to filenames, and preservation of named failures withheld from a retry are worth keeping. I would require the following corrections." "The live log recorded a raw-named file as copied, then attributed its text-named sibling's write failure to that same raw file. The closing summary added the correct failure, leaving both in the log. Carry the exact identity on live progress events. The run-wide lookup by display text can return an earlier pass's filename." The remaining work is cr-ecm39-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f62df807`
**Reviewer**: none yet — found outside a review; for the next openreview.

## Evidence
By reading `finish_record` and `RunLog::close` against cr-ecm38-1's account.
- **What goes in.** The final summary names the vanished UTF-8 file by its text alone, its identity.
- **What the log heard.** The run's log heard `RawName` (`dup` + U+FFFD → `dup` + byte `FF`) when the retry pass sent the sibling.
- **What comes out.** `failed_raw_names` and the end pass took those bytes for the failure.

## Intake
Admitted. The lookup served peers that sent a failure's text without its bytes. Transfer sessions require an exact contract match (`HELLO`), and every peer of this contract carries the bytes of every name that is not text:
- the report's `raw_relative_path`;
- `failed_paths_raw`, exact per identity since cr-ecm36-4;
- the delegated summary's `failed_paths_raw`.

So a failure the account names by its text alone is a text name, and the lookup could only misname. Per D-2026-11-24-1, an identity travels as given.

## What
- **The run log's end pass** (`blit-core/src/run_log.rs`) logs each failure by the identity the account gives it. The live-heard bytes are no longer looked up, and neither is the left-in-place decision those bytes drove; the account's own decision stands.
- **The CLI's record writer** (`finish_record`) saves each failure by the identity the account gives it. `failed_raw_names` and its `raws` parameter are removed.
- **Tests that built old peers' totals.** Three built a byte-named failure with no bytes, and the daemon's job-log test filled in a summary from the log:
  - `a_live_failure_hides_no_other_failure_of_its_text`;
  - `an_unnamed_failure_is_left_in_place_by_its_own_bytes`;
  - `a_record_keeps_a_failed_names_exact_bytes`;
  - the daemon's `a_raw_name_is_logged_with_its_exact_bytes`.

  Each now gives its totals the bytes, as a current peer sends them, and asserts what it asserted before.
- **Test count.** `a_failed_names_bytes_come_from_the_runs_log`, the test of the removed helper, is removed with it, and the new guard below takes its place.

## Guard proof
- `run_log::tests::a_text_failure_is_not_named_by_another_files_bytes`: a log that heard `dup` + byte `FF` for the spelling `dup` + U+FFFD logs a failure the account names by that text as the text, with no bytes.
- Mutation, restored byte for byte: the end pass names a text failure by the bytes heard for its spelling → red.
- The record writer has no source of other bytes left, so it holds by construction; no mutation can put the lookup back short of the removed parameter.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,127 tests over 57 binaries, the same count as before.

## Files changed
- `crates/blit-core/src/run_log.rs` — the end pass; guard; two tests' totals.
- `crates/blit-cli/src/run_log.rs` — `finish_record`; `failed_raw_names` and its test removed; one test's totals.
- `crates/blit-daemon/src/job_logs.rs` — one test's summary.

## Known gaps
- The live lane still names a failure from the bytes heard for its spelling. Within one session the manifest grants one entry per text, so it names the right file there.
