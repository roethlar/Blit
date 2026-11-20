# cr-ecm4-4: A command refused for what it says had already replaced the saved job

**Severity**: MEDIUM — a working saved job was replaced by one that can never run; nothing was transferred or deleted
**Status**: Verified — `exclude-case-r5` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..585d3599`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding (it found one refusal still made after the save, `--workers` on a remote route — cr-ecm5-2)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `a34af5e4`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r4` over `3bc3ee47..0669279f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r4.codex.md`.

## Evidence
Both `move --ignore-case --save nightly` and a mirror whose pattern file contained `[aé].txt` replaced a working `nightly` before reporting the error; `jobs run nightly` then failed.

Cause: `run_command` (`crates/blit-cli/src/main.rs`) saves the job before the verb runs — on purpose, so that a command which fails on the day (a host down, a source not mounted) can be run again — and every refusal sat in the verb's dispatch, after the save. cr-ecm3-2 moved one of them forward (`move`'s filters) and recorded the rest as a known gap, on the reasoning that the job "is refused again when run, so nothing is moved". That overlooked the job the name held before.

## Predicted observable failure
Any copy, mirror or move that is refused for its own words, typed with `--save NAME` where NAME is a job that works: the job is replaced, and `blit jobs run NAME` fails from then on. With `--export FILE`, a job file that can never run is written.

## Intake
Admitted. The fix covers every refusal a command's words decide, not only the two the reviewer ran.

## What
`refuse_unrunnable` (`crates/blit-cli/src/transfers/mod.rs`) asks, before a run is recorded, a job saved or a job exported, everything the dispatch asks before its first banner, warning or question:
- the route the two endpoints select, with the checks on the pair (`select_route`: the same path, one inside the other);
- for `move`, its options (`reject_move_options`: `--dry-run`, `--detach`, a filter, a case mode, `--ignore-existing`, `--null`, `--force`, `--ignore-times`, `--size-only`);
- for `copy` and `mirror`, the options the verb or route cannot honour and the filter (`reject_transfer_options`: `--null`, `--detach`, the diagnostic checker pin, a pattern that is no glob or holds a class no name can match, a size or age that cannot be read).

The dispatch calls the same three functions where its inline checks were, so there is one statement of each refusal. A command refused this way is not a run: it leaves no run record, as `move`'s filter refusal has left none since cr-ecm3-2.

What still saves first, by design: a command whose words are sound and which fails on the day — a source that does not exist, a daemon that does not answer.

## Guard proof
- `saved_jobs::a_command_refused_for_what_it_says_leaves_the_saved_job_as_it_was`: a working `nightly`, then nine refused commands each with `--save nightly --export FILE` — the reviewer's two, and one of each other kind (two more of `move`'s options, a pattern that is no glob, an unreadable size, `mirror --null`, `--detach` on a local route, source and destination the same). After each: the refusal's words, nothing saved, no file exported, nothing transferred, the saved-job listing unchanged, and `jobs run nightly` copies as before.
- `saved_jobs::a_move_refused_for_a_filter_saves_no_job` (cr-ecm3-2) unchanged and green.
- Mutations, each restored byte for byte: nothing refused before the save → red; `move`'s options not asked early → red; `copy`'s and `mirror`'s not asked early → red.

## Files changed
- `crates/blit-cli/src/transfers/mod.rs` — `select_route`, `refuse_unrunnable`, `reject_transfer_options`, `reject_move_options` (the dispatch's own checks, moved into functions without change).
- `crates/blit-cli/src/main.rs` — asked before the run starts.
- `crates/blit-cli/tests/saved_jobs.rs` — the guard.

## Known gaps
- A command refused for its words is no longer listed as a failed run in `blit jobs list`. Before this fix the ones refused at dispatch were.
- Refusals that need the daemon's answer (a module that does not exist, a version mismatch) come after the save, as any failure on the day does.
- `--files-from` naming a file that cannot be read was already refused by the save itself ("this command cannot be saved as a job"); unchanged.
- Run on Linux only.
