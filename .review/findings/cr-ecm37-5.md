# cr-ecm37-5: a shortened diagnostic was not tied to its exact entry

**Severity**: MEDIUM — a retry gives up names it has; the JSON and the saved record lose what the report said. cr-ecm36-3 made a failure whose path the wire shortened say so, and kept anything that names files from taking the rendering for a name. It did not say which exact name the rendering stood for, and the reviewer found three consequences.
- **The retry.** With 260 missing long names the failed-path list was cut for size, and the retry fell back to the report's names alone. Every one of those was shortened, so `--retries 1` retried none, though the list carried 225 exact names. The saved record held all 225.
- **JSON** showed a 4,096-byte rendering of a 4,583-byte name with no flag.
- **The saved record** named the long file without its reason.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm37-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r38` over `2c55cb3a..6ee7ed5e` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r38.codex.md`. It completes cr-ecm36-3 (`f29d3dc7`) in part.

## Evidence
"With 260 missing long names, `--retries 1` retried zero even though the saved record contained 225 exact names. For one 4,583-byte name, JSON exposed a 4,096-byte `relative_path` without a shortening flag, and the saved record lost the reason. Give the diagnostic a reference to the exact entry and use it in retry, persistence and JSON."

## Intake
Admitted as the reviewer put it.

## What
- **The reference.** The report's `FileFailure` gains `uint32 failed_path_at = 7` (contract v33): where the failure's exact name stands in the same summary's `failed_paths`, counting from 1, or 0 where that list does not carry it.
  - The destination sets it as it builds the summary, by each failure's exact identity (`SinkOutcome::wire_failures_linked`).
  - The CLI sets it again when it writes the run's final report back onto the summary (`failures_to_report`).
- **Reading it.** Every reader of a summary's report takes it through one function, `failures_from_report`, by way of `failures_of`, or `delegated_summary_failures` for a delegated pull. That covers:
  - the CLI's routes and its retry;
  - the run log's totals, from which the saved record is made;
  - the local surface.

  A linked entry is read back as its exact name, no longer shortened, so the retry, the saved record and JSON have the name and its reason.
- **JSON** gives each failure a `path_shortened` field, true only for a path the report could not link.
- **The retry** (`PassFailures::retry_set`): a list cut for size still names exactly every file it carries. The pass is given all of those with the report's names, and is marked short.

## Guard proof
- `sink::cr_win_1_tests::a_shortened_failure_is_read_back_as_its_exact_name`:
  - a long failure is shortened on the wire, linked to place 1, and read back as its exact name;
  - unlinked, it stays shortened and says so.
- `run_log::tests::a_shortened_failure_keeps_its_reason`: a summary with that entry joins the reason to the long name.
- `transfers::retry::tests::a_cut_list_gives_the_pass_every_name_it_carries`: a cut list of two names with one shortened report entry gives the pass both names, marked short.
- `transfers::retry::tests::the_final_summary_links_its_report`: the report written back onto the summary is linked.
- `transfers::failures::tests::the_json_says_when_a_path_is_shortened`.
- Mutations, each restored byte for byte:
  - the report not linked → red;
  - the link not read back → red;
  - a cut list given the report's names alone → red;
  - the final summary not linked → red;
  - the JSON silent on a shortened path → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,121 tests over 57 binaries.

## Files changed
- `crates/blit-core/proto/blit.proto` — `FileFailure.failed_path_at`.
- `crates/blit-core/src/transfer_session/mod.rs` — contract v33; the summary links its report; wire literals.
- `crates/blit-core/src/remote/transfer/sink.rs` — `wire_failures_linked`, `failures_to_report`, `failures_from_report`; guard.
- `crates/blit-core/src/transfers/failures.rs` — `failures_of`.
- `crates/blit-core/src/remote/transfer/delegated_summary.rs` — the delegated report read back linked.
- `crates/blit-core/src/run_log.rs`, `crates/blit-core/src/transfer_session/local.rs` — readers; guard.
- `crates/blit-cli/src/run_log.rs`, `transfers/{mod,remote,remote_remote_direct}.rs` — readers.
- `crates/blit-cli/src/transfers/retry.rs` — readers; `retry_set`; `apply_to_summary`; guards.
- `crates/blit-cli/src/transfers/failures.rs` — `path_shortened` in JSON; guard.
- `crates/blit-core/tests/source_side_containment.rs` — literals.

## Known gaps
- A failure whose name the cut list does not carry stays shortened, with no name. The retry counts it as not retried, as before.
- The human-facing report now prints a linked long name whole.
