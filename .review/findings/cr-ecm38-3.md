# cr-ecm38-3: the saved record dropped a failure's "not known" disposition

**Severity**: HIGH — a saved retry keeps a partial as a file someone put there. cr-ecm37-3 carries "not known" per file through the wire, the run log and the live retry. The CLI's record writer saved the run's leftover list as whole all the same. A failure whose disposition was not known went through the production writer, and `jobs retry` with `--ignore-existing` exited 0, leaving the nine-byte partial in place for an eight-byte source.
**Status**: Reviewed, found incomplete — `combined-r40` (Codex openreview, 2026-11-24) reviewed `86d69ee3..d7f5f5e9` — the `combined-r39` fixes and log-named-text — with `guard_confirmed: true`: "The held directory, explicit links from diagnostics to filenames, and preservation of named failures withheld from a retry are worth keeping. I would require the following corrections." "With a previous-format record that marked the leftover list complete and a log that recorded the disposition as unknown, `jobs retry` under `--ignore-existing` exited 0. It left a nine-byte partial for an eight-byte source. Distinguish records written with reliable disposition information. For older records, recover that information from sufficient evidence or decline the affected retry." The remaining work is cr-ecm39-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `929ce1a3`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r39` over `6ee7ed5e..86d69ee3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r39.codex.md`. It completes cr-ecm37-3 (`1d5be9a8`) in part.

## Evidence
"I passed a failure with unknown partial status through the production record writer. With `--ignore-existing`, `jobs retry` exited successfully and left the nine-byte partial in place for an eight-byte source. Persist the per-file unknown state, or mark the saved account incomplete so retry can decline it."

## Intake
Admitted, taking the reviewer's second branch: mark the saved account incomplete. A record from a daemon's answer already does this (cr-ecm35-5: `left_in_place_truncated` is set when any failure is not known). `jobs retry --ignore-existing` already declines an incomplete account, so it declines this one.

## What
`finish_record` (`blit-cli/src/run_log.rs`) takes the run's failed files from the one join (`failed_files`). When any of them is not known to have left its copy or not, it saves the leftover account as incomplete (`left_in_place_truncated`).

## Guard proof
- `run_log::tests::a_failure_not_known_to_have_left_its_copy_leaves_the_account_incomplete`:
  - a run whose one failure is not known is saved with its leftover account incomplete;
  - the same failure known either way is saved whole.
- Mutation, restored byte for byte: a "not known" disposition saved as whole → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,127 tests over 57 binaries.

## Files changed
- `crates/blit-cli/src/run_log.rs` — `finish_record`; guard.

## Known gaps
- Coarse: one failure that is not known declines the whole `--ignore-existing` retry of a saved job, not just its own file. The in-command retry passes decline per file (cr-ecm37-3).
