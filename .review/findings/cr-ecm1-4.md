# cr-ecm1-4: A job file holding two options no command takes together is run

**Severity**: MEDIUM — a job file edited to hold both case modes ran under `--ignore-case` and a mirror deleted a file the job as made never named; it takes a hand-edited file to get there
**Status**: Verified — `exclude-slice1-r2` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..a1fd6bae`, which holds this fix, with `guard_confirmed: true`, and required nothing further on it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `8ca5a474`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, review `exclude-slice1-r1` over `3bc3ee47..66d1338a`. That run FAILED before a verdict (provider 503); this finding is read from its event stream (`.review/results/exclude-slice1-r1.codex.events.jsonl`, event 136, first probe).

## Evidence
The reviewer's probe against the built head: `mirror --yes --match-case --include '*.cfg' --export job.json` over a source holding `keep.cfg` and a destination holding `ONLY.CFG` — exit 0, `ONLY.CFG` kept (the pattern as typed does not name it). The reviewer then set `ignore_case` to true in the job file beside `match_case` and ran `jobs run job.json`: exit 0, `ONLY.CFG` deleted.

Cause: `--match-case` and `--ignore-case` exclude each other on the command line, where the parser refuses them. A job's arguments are built from its spec (`crates/blit-cli/src/jobs.rs` `spec_args`) and never pass the parser, and the filter reads `ignore_case` alone.

Measured afterwards on this machine (2026-11-20, Linux), the same edit for every pair the command line declares: `match_case`+`ignore_case`, `checksum`+`size_only`, `checksum`+`ignore_times`, `size_only`+`ignore_times`, `ignore_existing`+`force` — all five ran, exit 0. The four older pairs have run this way since job files existed; slice 1 added a fifth of the same kind, and the first one that changes what a mirror deletes.

## Predicted observable failure
A job file edited to hold two options that contradict each other is run under whichever the code happens to read, with no word that the file says two things.

## Intake
Admitted, for all five pairs: the rule slice 1 broke is the same rule the four older pairs already broke, and fixing one of five would leave a known imperfection (D-2026-10-19-2).

## What
A job's spec that holds two options no transfer takes together is refused before anything is written or run: `this job asks for both --match-case and --ignore-case; a transfer takes only one of them`. The rule lives with the spec (`SpecOptions::conflict`, `blit-core`), so every front end that reads job files has it; `jobs run` and `jobs retry` both go through `spec_args`, which asks it first.

## Guard proof
- `job_record::tests::a_spec_names_two_options_no_transfer_takes_together` (core): each of the five pairs is named in the words typed; either option alone, and one option from each of three pairs, is no conflict.
- `jobs::retry_name_tests::a_spec_holding_two_options_the_command_line_refuses_together_is_refused` (CLI): reads the pairs from the command line's own declaration (`clap`'s conflicts on `TransferArgs`), and requires each to be refused by `spec_args` naming both options, before the run's `--files-from` list is written; either option alone still builds. A pair added to the command line and not to the rule turns it red.
- `saved_jobs::a_job_file_holding_two_options_no_command_takes_together_is_refused`: the reviewer's scenario through the binary, and the same for `checksum`+`size_only`: non-zero exit naming both options, nothing copied, `ONLY.CFG` still there; the job as exported still runs.
- Mutations, each restored byte for byte: the refusal removed from `spec_args` → the CLI and end-to-end guards red; the case pair dropped from the rule → all three red; `--ignore-existing`/`--force` dropped from the rule → the core guard and the CLI guard red (the drift check works).

## Files changed
- `crates/blit-core/src/job_record.rs` — `SpecOptions::conflict`; the core guard.
- `crates/blit-cli/src/jobs.rs` — `spec_args` refuses; the CLI guard.
- `crates/blit-cli/tests/saved_jobs.rs` — the end-to-end guard.

## Known gaps
- Run on Linux only.
- The rule's five pairs are written out in core and held to the command line's declaration by the CLI guard, not derived from it: core does not depend on the command line.
