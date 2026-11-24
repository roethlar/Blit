# cr-ecm39-2: a short record was completed from a log marked incomplete

**Severity**: HIGH — a saved retry leaves a failed file out and writes over one that landed.
- **Completion.** A record whose failure list was cut short is completed from the run's log on this machine. The completion was made from whatever log there was, whole or not.
- **The check.** Only the count was checked.

The reviewer's run had a log marked incomplete. It still named a stale failure, a file a later pass had landed, which made the number of names match the recorded count. `jobs retry` exited 0, left out a file that had really failed, and overwrote the one that had landed.
**Status**: Reviewed, found incomplete — `combined-r41` (Codex openreview, 2026-11-24) reviewed `d7f5f5e9..b6e6fab9` — the `combined-r40` fixes — with `guard_confirmed: true`: "The per-failure identity and Linux `openat2` fast path are worth keeping. A 72-directory-deep resume completed with a 64-descriptor limit and produced the correct bytes. I would require the following changes." "`jobs retry` accepted a crash-recovered log, overwrote an already-landed file, omitted a file that still needed copying, and exited 0. A log with a sequence-number gap produced the same result. Check sequence continuity and terminal summary coverage before completing a short record. Recovery can append `run-end` to a partial log." The remaining work is cr-ecm40-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `3098728b`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r40` over `86d69ee3..d7f5f5e9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r40.codex.md`. It completes cr-ecm38-7 (`949f1c89`).

## Evidence
"I gave `jobs retry` a truncated record and a log marked incomplete. A stale failure made the number of names match the recorded count. The retry exited 0, omitted an actual failed file, and overwrote a third file that had already landed. Require complete log evidence before adding identities to a short record. Matching counts alone cannot establish which files failed."

## Intake
Admitted as the reviewer put it.

## What
In `failed_identities` (`blit-cli/src/jobs.rs`), a record whose list is short is completed from the log only when the log is read and whole: every log of the run ends with its `run-end` and lost nothing. Otherwise nothing is retried, and the refusal says why and how to run the whole job again. The count checks, too few and too many, still apply to a list a whole log completes.

## Guard proof
- `jobs::retry_name_tests::a_cut_list_is_completed_only_from_a_whole_log`: a cut record counting two failures that names one.
  - With a log that lost events, naming the other, it is refused.
  - With a whole log naming it, it is retried as both.
- Mutation, restored byte for byte: a cut list completed from a log not whole → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,140 tests over 57 binaries.

## Files changed
- `crates/blit-cli/src/jobs.rs` — `failed_identities`; guard.

## Known gaps
- A short record with only a partial log can no longer be retried; the job must be run again whole.
