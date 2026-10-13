# cr-jl5-2: Preserve the command's start timestamp across deferred initialization

**Severity**: LOW (reviewer: LOW)
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `75864cb0`
**Reviewer**: codex / gpt-6-astra / xhigh / standard (inline, session-only — the owner's word `astra`), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-jl5-031e0352`; codereview defect hunt jl5-r1 over `df5fd1cf..031e0352` (record `.review/results/jl5-r1.codex.json` + `.events.jsonl`); 2026-10-13 18:03–18:18Z; capability_ok=true; guard_confirmed=true; SHAs pin-verified; the local ptk hook redirected the reviewer's shell calls (a note, not an invalidation)

## Evidence
crates/blit-cli/src/run_log.rs:173 — deferred begin_record calls RunRecord::starting at line 596, which assigns started_ms from the current time. With a delayed machine-id read, the unmodified binary exported a record whose started_ms was 1,206 ms after the copied file was already present; its log correctly retained the earlier run-start timestamp.

## Predicted observable failure
Slow initialization records the worker's initialization time as the command's start. jobs list displays incorrect start times and can reorder overlapping commands; exported timestamps can imply that a completed transfer began after its file operations.

## Reviewer's suggested approach
Capture the start timestamp in CommandRun::start and pass it to the background record initializer.

## Intake
Admitted. `RunRecord::starting` stamps `started_ms` with the current time on the record thread, which jl-5 moved off the command's clock, so the record's start is the start of its own writing (normally a millisecond later; under a slow start, seconds). The log's `run-start` is stamped at start already. Fix shape: take the start time in `CommandRun::start` and hand it to the record thread so the record and the log agree.

## What
The run's record now starts when its command did. `CommandRun::start` takes the start time once, before the run's log is made or started, and hands it to the `blit-job-record` thread; `begin_record` writes the record with that `started_ms` instead of stamping the moment its thread gets to writing it. A slow start (a slow machine-ID read, a cold disk) no longer records a late start time, so `jobs list` shows and orders runs by when their commands began, and the record agrees with the log's `run-start`.

## Approach
- `RunRecord::starting_at(run_id, spec, started_ms)` (new, `blit-core`): the record for a run that started at `started_ms`. `RunRecord::starting` stays for its existing callers and is now `starting_at(.., now_ms())`.
- `CommandRun::start`: `let started_ms = now_ms();` right after the run ID is made, before `RunLog::new`/`start_with`, so it precedes the log's own `run-start` stamp; the `u64` moves into the record thread's closure.
- `begin_record` gains a `started_ms: u64` parameter and builds the record with `RunRecord::starting_at`.
- The test helper `record_of` (added with cr-jl5-1) takes the start time to hand `begin_record`; the two cr-jl5-1 tests pass `now_ms()`.

## Files changed
- `crates/blit-core/src/job_record.rs:238-258` — `RunRecord::starting` delegates to the new `RunRecord::starting_at`.
- `crates/blit-cli/src/run_log.rs:147-149` — `CommandRun::start` takes `started_ms` before the log starts.
- `crates/blit-cli/src/run_log.rs:176-184` — the record thread passes it to `begin_record`.
- `crates/blit-cli/src/run_log.rs:587-595, 608` — `begin_record` takes `started_ms` and writes the record with `RunRecord::starting_at`.
- `crates/blit-cli/src/run_log.rs:1082-1088, 1102, 1129, 1152` — `record_of` takes the start time; its callers pass `now_ms()`.
- `crates/blit-cli/src/run_log.rs:1164-1174` — `the_record_starts_when_its_command_did` (the guard).

## Guard proof
- Guard: `run_log::tests::the_record_starts_when_its_command_did` (blit-cli unit test). It runs `begin_record` for a run started at 1_000 ms and loads the record back from the run store: `started_ms` must be exactly 1_000.
- Mutation: in `begin_record`, `RunRecord::starting_at(run_id, &spec, started_ms)` replaced with `RunRecord::starting(run_id, &spec)` (the current time, as before the fix; `let _ = started_ms;` to keep it compiling).
- Red: `cargo test -p blit-transfer --bin blit run_log::tests` → `the_record_starts_when_its_command_did ... FAILED`, "assertion `left == right` failed", `left: 1791917393018`, `right: 1000` — the record held the moment it was written, not the start it was handed.
- Restore: the fixed file copied back; the same command → 6 passed, the guard `ok`.

## Known gaps
- The guard proves `begin_record` keeps the start it is handed; that `CommandRun::start` takes it before the log starts is by reading (it is the first thing after the run ID), not by a test — a test of it would have to slow the record thread down, which nothing in the code offers short of a new hook.
- Verified on macOS (aarch64) only, plus Linux cross-clippy; the test sits in an ungated module but was not executed on Linux or Windows.

## Reviewer comments
Reviewer: codex / gpt-6-astra / xhigh / standard (inline, session-only — the owner's word `astra`); codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-jl5fix1-75864cb0`; re-review jl5fix1-r1 over `7cb47b4c..75864cb0`; 2026-10-13 20:11–20:18Z; reviewed_sha `75864cb04ba6b494e27661af26cd2c8a1fb9e5e9`, base_sha `7cb47b4cbc169bbd20d2d2c954bc7c7112926306`; capability_ok=true; guard_confirmed=true; verdict **clean**, both findings closed; record `.review/results/jl5fix1-r1.codex.json` + `.events.jsonl` (committed)

Closed: CommandRun::start captures started_ms before starting the workers and passes it through begin_record to RunRecord::starting_at. Delayed-machine-ID CLI probes confirmed that normal and exported records retain a timestamp preceding the completed copy; saved jobs and retries use the same initialization path.

Guard (reviewer's own): On macOS arm64, temporarily changed job_spec to reread the file: the_spec_keeps_the_list_the_run_started_with failed with ["b.txt"] instead of the captured lines. Restored with git checkout and confirmed it passed. Separately replaced starting_at with starting: the_record_starts_when_its_command_did failed with the current timestamp instead of 1000; restored with git checkout and confirmed it passed. Both restorations were checked against the original content, and the worktree is clean. Workspace fmt, native clippy and tests passed (1483 passed, 2 ignored), as did Linux cross-target clippy. Linux and Windows runtime tests were not run.
