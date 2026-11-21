# cr-ecm13-3: A retry asked for a failed file whose name is not text by another file's name

**Severity**: HIGH — the file that failed is never retried, another is sent in its place, and the command exits 0
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm13-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r13` over `c99ad483..bd84649f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r13.codex.md`. Older than the plan (the retry passes, ssc-6); found through cr-ecm12-1's guard.

## Evidence
"An automatic retry for `dup` followed by byte `FF` overwrote the distinct UTF-8 file `dup` + U+FFFD and exited 0. I reproduced the same substitution with saved jobs after an intervening retry reported the raw name missing and discarded its bytes."

Cause. The exact set of failed paths goes from the end that writes to the command as TEXT (`TransferSummary.failed_paths`), and the text of a name that is not text is the nearest text — the name of another file, if the source has one. The retry passes of one command asked for that. (A saved job's retry already asks by bytes, from its log.) And a retry that did not find a requested name reported it missing by its text alone, so the bytes were gone from the next record.

## Intake
Admitted as the reviewer put it.

## What
- The end that writes keeps the exact bytes of each failed entry whose name is not text (`SinkOutcome.failed_raw`): recorded with the failure, or taken from the session's manifest when the failure was recorded by its text. The summary carries them at the places of `failed_paths` (`TransferSummary.failed_paths_raw`, **wire contract 26**; re-encoded for a transfer between two daemons).
- A retry pass is limited to a set of two kinds (`RetrySet`): paths by their text, and names that are not text by their bytes and by nothing else — from the exact set, and from the named report where the set did not fit. Under `--ignore-existing` the pass of the run's own incomplete copies takes its names that are not text by bytes as well, and what the passes leave goes back into the summary with its bytes.
- A requested name that was not found is reported with its bytes (`requested_but_unscanned`).

## Guard proof
- `retry_pass.rs::a_failed_name_that_is_not_text_is_retried_as_itself` (Unix, where the filesystem takes such a name): a source holding `dup\xff` and `dup` + U+FFFD, a folder in the way of the first. With `--retries 1` the command exits 2 — the file that failed has not landed — where it exited 0. Then, freed during the wait, it lands on the retry; the other file, changed at the destination during the same wait, stays as changed: the retry asked for one file.
- `transfers::retry::…::a_name_that_is_not_text_is_retried_by_its_bytes`: from the exact set, from the named report, and apart under `--ignore-existing`; and the summary after the passes holds the bytes.
- `sink::…::the_exact_failed_set_holds_the_bytes_of_a_name_that_is_not_text`; `transfer_session::…::a_requested_name_that_is_not_text_is_reported_missing_by_its_bytes`.
- Mutations, each restored byte for byte: a failure recorded by its text alone; one named from the manifest afterwards not given its bytes; the exact set going out as text alone; a requested name not found reported by its text alone; the local route dropping the bytes; a pass asking for the exact set, or the named report, by its text; a pass not given the bytes; a copy left under such a name not known by its bytes — nine, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/remote/transfer/{sink,delegated_summary}.rs`, `crates/blit-core/src/transfer_session/{mod,local}.rs` (`CONTRACT_VERSION = 26`), `crates/blit-core/src/transfers/remote.rs`, `crates/blit-core/proto/blit.proto`.
- `crates/blit-cli/src/transfers/retry.rs` — `RetrySet`; `crates/blit-cli/src/transfers/remote_remote_direct.rs` (a test's summary); `crates/blit-cli/tests/retry_pass.rs`.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- **Two failed entries whose names collapse to one text** are still one entry of the exact set, as they were: the end that writes holds that set by text. The second of two such names is refused when the manifest is read, so the case needs both to fail.
- What a pass REMOVED is listed by text alone; a copy left under a name that is not text is taken for removed when the text it is shown as is in that list.
- The guard through the binary runs on the local route; the bytes' way through a daemon's summary is by the unit guards and by reading.
- Run on Linux only.
