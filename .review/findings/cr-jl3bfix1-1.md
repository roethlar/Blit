# cr-jl3bfix1-1: a run-ID-shaped job name is refused with guidance it already meets

**Severity**: LOW (reviewer: LOW)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `8d62c25e`
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3bfix1-r1 over `348be309..1acc17b0` (record `.review/results/jl3bfix1-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/cli.rs:169 — `parse_job_name` rejects a 32-character lowercase hexadecimal name through `valid_job_name`, but its error lists only conditions that such a name satisfies; crates/blit-core/src/job_record.rs:378 repeats the incomplete rule.

## Predicted observable failure
A user supplying such a name through `--save` or `jobs save` is told it must meet rules it already meets, with no indication that the run-ID namespace is reserved or how to correct the name.

## Reviewer's suggested approach
Have the shared validator return a specific failure reason and reuse it in both Clap parsing and `SavedJobs::path`, explicitly naming the reserved 32-lowercase-hex run-ID shape.

## Intake
Admitted. Both refusals list the name rules but not the reserved run-ID shape, so a refused name meets every rule they state. One check, giving its own reason, serves the command line and the saved-job store.

## What
One check, `job_record::job_name_problem`, gives the reason a name is refused — its length, a leading `.`, a character outside letters, digits, `-`, `_`, `.`, or the run-ID shape ("32 lowercase hex digits are a run ID's shape, kept for run IDs; choose another name"). `valid_job_name` is that check's yes/no, and both the command line (`--save`, `jobs save`) and the saved-job store word their refusals from it.

## Guard proof
`saved_jobs_keep_a_spec_by_name_until_deleted` (blit-core) checks each refusal's own reason (and that upper-case hex or another length is an ordinary name); `a_saved_jobs_name_means_the_saved_job_wherever_it_is_typed` (CLI) requires the run-ID refusal to say the shape is kept for run IDs. Mutation — the run-ID refusal giving the generic character rule — makes both fail; restored, green.
