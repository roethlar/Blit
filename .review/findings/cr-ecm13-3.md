# cr-ecm13-3: A retry asked for a failed file whose name is not text by another file's name

**Severity**: HIGH — the file that failed is never retried, another is sent in its place, and the command exits 0
**Status**: Reviewed, found incomplete — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`. The account of what a retry did not find drops the bytes: "A job recorded two failures, one named `dup` plus byte `FF`, the other `dup` plus U+FFFD. After the first disappeared, `jobs retry` copied the second and exited 0." And the answer two daemons give a CLI is not versioned for the new list: "a head CLI and two contract-25 daemons still agree on delegated spec version 12. Those daemons omit the raw failed-path list." The remaining work is cr-ecm14-1 and cr-ecm14-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `6f0479dc`, `b2b77b78`
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

## The first commit, and what it broke
`6f0479dc` gave the exact set the bytes of a failed entry in two ways: recorded with the failure, or taken afterwards "from the manifest, where a text names one granted entry". Two source names of one text — `dup` + byte `FF` and `dup` + U+FFFD — are two entries of ONE text: the end that writes takes the first and refuses the second. Where the second is the name that IS text, it was recorded by its text alone, and named afterwards by the bytes the manifest holds under that text — the OTHER file's. The retry asked for the file that had landed, found nothing to do, and the file that was refused was never sent: exit 0. Before `6f0479dc` that order was retried by text and went right, and the other order wrong; `6f0479dc` put the first right and this one wrong.

Which name a filesystem lists first differs. This session's gate keeps its tests' files on tmpfs, which lists the name that is text first; the fault shows on Btrfs, which lists the other first — found by the Btrfs run this round added, after `6f0479dc` had been committed on the tmpfs gate alone.

This commit: the entry refused because another has its text is recorded by ITS OWN bytes, whatever they are — also where its name is text (`transfer_session`, where the manifest is read) — so nothing names it again. The retry asks for it by those bytes.

## Guard proof
- `retry_pass.rs::a_failed_name_that_is_not_text_is_retried_as_itself` (Unix, where the filesystem takes such a name): a source holding `dup\xff` and `dup` + U+FFFD, a folder in the way of the first. With `--retries 1` the command exits 2 — the file that failed has not landed — where it exited 0. Then, freed during the wait, it lands on the retry; the other file, changed at the destination during the same wait, stays as changed: the retry asked for one file.
- `transfers::retry::…::a_name_that_is_not_text_is_retried_by_its_bytes`: from the exact set, from the named report, and apart under `--ignore-existing`; and the summary after the passes holds the bytes.
- `sink::…::the_exact_failed_set_holds_the_bytes_of_a_name_that_is_not_text`; `transfer_session::…::a_requested_name_that_is_not_text_is_reported_missing_by_its_bytes`.
- **`retry_pass.rs::two_names_of_one_text_both_arrive`** (Unix, where the filesystem takes such a name): a source holding both names, `--retries 1` — both files at the destination, exit 0, whichever is listed first. Run on tmpfs and on Btrfs.
- `…::a_failed_name_that_is_not_text_is_retried_as_itself` no longer depends on the order: the file of the other name is made during the wait.
- `sink::…::a_failure_recorded_by_its_bytes_is_not_named_again_from_the_manifest`.
- Mutations of this commit, each restored byte for byte, each run on the filesystem whose order shows it: the refused entry recorded by its text when its name is text → `two_names…` red on Btrfs; a failure recorded by its bytes named again from the manifest → the unit guard red; a pass asking for the exact set by its text → both end-to-end guards red on tmpfs.
- Mutations, each restored byte for byte: a failure recorded by its text alone; one named from the manifest afterwards not given its bytes; the exact set going out as text alone; a requested name not found reported by its text alone; the local route dropping the bytes; a pass asking for the exact set, or the named report, by its text; a pass not given the bytes; a copy left under such a name not known by its bytes — nine, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/remote/transfer/{sink,delegated_summary}.rs`, `crates/blit-core/src/transfer_session/{mod,local}.rs` (`CONTRACT_VERSION = 26`), `crates/blit-core/src/transfers/remote.rs`, `crates/blit-core/proto/blit.proto`.
- `crates/blit-cli/src/transfers/retry.rs` — `RetrySet`; `crates/blit-cli/src/transfers/remote_remote_direct.rs` (a test's summary); `crates/blit-cli/tests/retry_pass.rs`.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- **Two failed entries whose names collapse to one text** are still one entry of the exact set: the end that writes holds that set by text. Where both of two such names fail in one pass — one refused, the other failing where it is written — the set names one of them, the count says two, and the other is reported as not retried.
- **A failure of an entry whose name is text can now carry bytes** — its name's own — where another entry has its text; a report or a log then shows that name escaped, byte by byte.
- **`6f0479dc` was committed on a gate that could not show its fault** (tmpfs). The whole suite is run on Btrfs as well each round; it was run after that commit, not before.
- What a pass REMOVED is listed by text alone; a copy left under a name that is not text is taken for removed when the text it is shown as is in that list.
- The guard through the binary runs on the local route; the bytes' way through a daemon's summary is by the unit guards and by reading.
- Run on Linux only.
