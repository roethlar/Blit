# cr-ecm5-2: A command refused for its route had already replaced the saved job

**Severity**: HIGH — a working saved copy job was replaced by a mirror; run again, it deleted a destination-only file
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm5-2: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r5` over `3bc3ee47..585d3599` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r5.codex.md`.

## Evidence
A remote `mirror -y --workers 1 --save nightly` replaced a working saved copy job before rejecting `--workers`. The saved spec omitted that diagnostic flag. `jobs run nightly` executed the mirror, deleting a destination-only file.

Cause: cr-ecm4-4 gathered every refusal a command's words decide into `refuse_unrunnable` — except the ones each route's own arm made (`crates/blit-cli/src/transfers/mod.rs`: `ensure_remote_push_supported`, `ensure_remote_pull_supported`, `ensure_remote_destination_supported`, `ensure_remote_source_supported`, fourteen calls in the seven remote arms of `copy`/`mirror` and `move`). Those ran after the question and after the save. This one is worse than the cases cr-ecm4-4 closed: the option refused is a hidden diagnostic a job does not record, so the job saved was one that RUNS.

## Predicted observable failure
`blit mirror --workers N --save NAME SRC HOST:/module/` (or `--dry-run`, or a destination that names no module) where NAME is a job that works: the command is refused and NAME is now a mirror to that daemon, which `blit jobs run NAME` carries out.

## Intake
Admitted. cr-ecm4-4's record said it covered "every refusal a command's own words decide"; it did not.

## What
`reject_unsupported_for_route` asks, by route, what a transfer through a daemon refuses — `--dry-run`, `--workers`, an endpoint that names no module — and is called where every other refusal is: by `refuse_unrunnable` before anything is kept of the command, and at dispatch before the banner, the warnings and the question. The fourteen calls in the route arms are gone; there is one statement of each refusal.

A second effect: these refusals now come BEFORE `mirror` and `move` ask their question, where they used to come after it.

## Guard proof
- `saved_jobs::a_command_refused_for_its_route_leaves_the_saved_job_as_it_was`: a working saved copy and a file only its destination holds; then five refused commands through a real daemon, each with `--save nightly --export FILE` — the reviewer's `mirror -y --workers 1` pushed, `--workers` pulled, `--workers` on a `move`, `--dry-run` to a daemon, a destination with no module. After each: the refusal's words, nothing saved, no file exported, nothing transferred, the saved-job listing unchanged; and `jobs run nightly` is still the copy — it copies, and the destination-only file is still there.
- Mutations, each restored byte for byte: `copy`/`mirror` not asking the route's refusals → red; `move` not asking them before the save → red.
- `saved_jobs::a_command_refused_for_what_it_says_leaves_the_saved_job_as_it_was` (cr-ecm4-4) unchanged and green.

## Files changed
- `crates/blit-cli/src/transfers/mod.rs` — `reject_unsupported_for_route`; the in-arm calls removed.
- `crates/blit-cli/tests/saved_jobs.rs` — the guard.

## Known gaps
- Refusals that need the daemon's answer (a module that does not exist, a read-only module, a version mismatch) still come after the save, as any failure on the day does; such a job records everything typed and fails the same way when run.
- A source path that does not exist is still found after the save, by design (it may exist on the day).
- The daemon-to-daemon route is covered by the function's one arm and not by a run of its own.
- Run on Linux only.
