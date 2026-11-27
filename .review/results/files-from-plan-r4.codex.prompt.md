You are the reviewer in an independent review of a pinned change in this repository.

The question, which is the whole of the substantive brief:

From your own reading of the repository, state the goal this change serves and how you would achieve it. Then judge: is the change as made the best way to achieve that goal?

Mechanical coordinates:

- Repository: /home/michael/dev/gitea_repos/blit_v2 (your working directory; it is the shared working tree, checked out at the head SHA below; its only untracked files are this review's own records under `.review/results/`).
- The change is exactly `git diff d8689340813c82725efbee4b56813077bc3e963f..f044ed526eddac74636c7adda0b5a490e6191534` (base..head). Evaluate that fixed snapshot.
- You may read anything in the repository and its Git history to discover the goal and to judge the change.
- Boundaries: do not modify, create or delete anything in the shared working tree outside the one directory named next. No commits, no pushes, no branch, tag or ref changes, no network mutations. Anything you run that writes, and any revert/restore proof, happens in a disposable copy of the head SHA that you create yourself under /home/michael/dev/gitea_repos/blit_v2/target/review-files-from-plan-r4 (inside the ignored build directory, on disk) — with `git worktree add --detach <dir> <head>`, or an export of that SHA if you prefer — and that you remove when you are done.
- Capability proof: read one file from the repository and run `git rev-parse f044ed526eddac74636c7adda0b5a490e6191534`; report whether both succeeded as `capability_ok: true` or `capability_ok: false`.
- Guard: this repository's policy requires the reviewer to run a red/green guard independently in that disposable copy and to report the literal field `guard_confirmed: true` or `guard_confirmed: false`. The range changes documentation only; the repository's verification commands for it are `git diff --check d8689340813c82725efbee4b56813077bc3e963f..f044ed526eddac74636c7adda0b5a490e6191534` and `bash scripts/agent/check-docs.sh`.
- Heartbeats: at major phase changes, around long-running checks, and during prolonged tool-heavy work, state briefly what you are currently doing. Activity only: no provisional verdict or finding in a heartbeat.

Result: lead with the approach, not a defect list. Report the goal you discovered, the approach you would take, how the change compares, and any material changes you would require. Give one verdict: `best_approach` (no material changes are needed), `acceptable_with_changes` (the approach stands; name the required changes) or `to_be_replaced` (name the replacement and the required changes). Concrete findings are optional. Include the base and head SHAs you reviewed, `capability_ok` and `guard_confirmed`.
