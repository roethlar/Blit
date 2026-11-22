# cr-ecm29-7: what an exchange swapped out was looked at through /proc

**Severity**: MEDIUM — the entry the exchange swapped out was looked at through `/proc/self/fd`, so a Linux daemon with no procfs put back an otherwise valid replacement as a conflict, and the file failed
**Status**: Verified on Linux — `combined-r31` (Codex openreview, 2026-11-22) reviewed `fd603571..93ecf7b2` — the six `combined-r30` fixes — with `guard_confirmed: true`: "I would keep the descriptor-relative inspection after exchange, the nonblocking checksum open, and the per-file refusal when safe replacement is unavailable."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `80ec4afc`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r30` over `e3ea9517..fd603571` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r30.codex.md`. It completes cr-ecm28-2 (`3c25cd73`).

## Evidence
"The `/proc/self/fd` lookup makes a Linux daemon without procfs roll back an otherwise valid replacement as a conflict. The directory descriptor needed for the lookup is already open." The reviewer's required change: "Inspect the exchanged entry with `fstatat` on the staging descriptor."

## Intake
Admitted as the reviewer put it.

## What
After the exchange, `Resolved::publish_replacing` looks at what came out of it with `entry_at`, which calls `fstatat` on the staging directory's own descriptor without following a link. The `/proc`-based `stage_path` is gone.

## Guard proof
- `local_copy_mode::a_resume_without_proc_replaces_what_it_may_not_reuse` (`blit-cli`, Linux): `blit copy --resume` runs under `unshare --user --map-root-user --mount` with an empty tmpfs mounted over `/proc`. The source is 0600; the destination is 0644, too wide to reuse. The copy succeeds, and nothing is left beside the destination. Where the system grants no such namespace (`unshare` refused), the test prints `SKIPPED` and does nothing.
- Mutation, restored byte for byte: the look through `/proc` put back → red.
- The resume, sink and session tests (341) and the four local-copy-mode tests pass.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `publish_replacing`; `stage_path` removed.
- `crates/blit-cli/tests/local_copy_mode.rs` — guard.

## Known gaps
- The guard needs unprivileged user namespaces. Where they are restricted (as on some CI images), it skips and cannot go red there. Here it ran and went red under the mutation.
