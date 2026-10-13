# cr-jl5-1: Snapshot --files-from before the transfer can overwrite it

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `7e188159`
**Reviewer**: codex / gpt-6-astra / xhigh / standard (inline, session-only — the owner's word `astra`), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-jl5-031e0352`; codereview defect hunt jl5-r1 over `df5fd1cf..031e0352` (record `.review/results/jl5-r1.codex.json` + `.events.jsonl`); 2026-10-13 18:03–18:18Z; capability_ok=true; guard_confirmed=true; SHAs pin-verified; the local ptk hook redirected the reviewer's shell calls (a note, not an invalidation)

## Evidence
crates/blit-cli/src/run_log.rs:173 — begin_record now runs concurrently with the transfer, but job_spec still reads --files-from from disk at line 733. Reproduced with the unmodified binary by holding the machine-id read until a copy overwrote its destination-side list: the transfer copied a.txt and list.txt, but --export recorded files_from as ["b.txt"], the replacement list's contents.

## Predicted observable failure
Without --save, a transfer that overwrites its --files-from file before deferred initialization reaches job_spec silently stores a different job. Running the exported job subsequently selects b.txt instead of the original a.txt and list.txt.

## Reviewer's suggested approach
Capture the list contents before transfer dispatch, preferably sharing the parsed selection with the transfer, and defer only persistence of that captured specification.

## Intake
Admitted. Before jl-5, `job_spec` ran before the transfer started, so the list it recorded was the list the run used. jl-5 moved `begin_record` to the `blit-job-record` thread, which runs beside the transfer; `job_spec` still reads the `--files-from` file from disk there, so a transfer that writes that file (a list kept inside its own destination tree, a mirror's deletion of it) can be recorded with another list than it ran. The plan's acceptance says a saved job reproduces the run after the list file changed; this breaks that. The reviewer reproduced it on the landed binary. Fix shape: capture the list's contents once, before the transfer can touch the file — sharing the transfer's own read where it has one — and defer only the writing of the spec; nothing else `job_spec` reads races with the transfer (the current folder does not change).

## What
The run's job record now keeps the `--files-from` list as the command read it before the transfer started. `run_command` reads the list once, just before `CommandRun::start`, and hands the text to the record's thread on `TransferArgs`; `job_spec` builds the spec's `files_from` from that captured text and never reads the file itself, so a transfer that overwrites its own list (one kept in its destination and listed in itself) can no longer make its record name another job than it ran.

## Approach
- `run_log::capture_files_from(&mut args)` (new, async): when `args.files_from` is set, reads the file's text through `tokio::task::spawn_blocking` (awaited) and stores it, or the read's error in the words `job_spec` used (`"<list>: <error>"`), in the new internal field `TransferArgs::files_from_text: Option<Arc<Result<String, String>>>` (`#[arg(skip)]`, never a flag). A command with no list returns at once — no task, no read, nothing added to its clock. A command with a list waits for one read, as it did before jl-5 (when `job_spec` read the list on the clock). The `Arc` keeps the per-pass and record-thread clones of `TransferArgs` from copying the list's text.
- `run_command` (`main.rs`) calls it immediately before `CommandRun::start`, so every route — copy, mirror, move, `jobs run`, `jobs retry` — captures before the record thread is spawned and before dispatch.
- `job_spec`: `files_from` comes from the captured text with the same line semantics as before (`text.lines()`, comments and blank lines kept). A captured read error returns that same message, so the job is "not kept" with the same warning text and the command goes on (the transfer reports the unreadable list itself). A list that is set but was not captured (no caller does this) is refused in words (`"<list>: the list was not read before the run"`) rather than read late. `job_spec`'s signature is unchanged.
- The transfer's own read of the list (`build_filter` → `FileFilter::load_files_from`, at dispatch, before any write) is unchanged; the reviewer's "share the parsed selection" suggestion was not taken (the filter wants a parsed path set, the spec the raw lines).
- The four exhaustive `TransferArgs` literals in unit-test fixtures gain `files_from_text: None`; the run_log module doc names the one read left on the clock.

## Files changed
- `crates/blit-cli/src/cli.rs:423-429` — `TransferArgs::files_from_text` (`#[arg(skip)]`).
- `crates/blit-cli/src/main.rs:261-262` — `run_command` captures the list before `CommandRun::start`.
- `crates/blit-cli/src/run_log.rs:13-14` — module doc: the list's read is the one left on the command's clock.
- `crates/blit-cli/src/run_log.rs:697-715` — `capture_files_from`.
- `crates/blit-cli/src/run_log.rs:719-720, 753-765` — `job_spec` builds `files_from` from the captured text, never the file.
- `crates/blit-cli/src/run_log.rs:1070-1145` — tests: `record_of` helper (runs `begin_record` into a temp store and loads the spec/record back), `the_spec_keeps_the_list_the_run_started_with` (the guard), `an_unreadable_list_keeps_no_job_and_says_why`.
- `crates/blit-cli/src/transfers/mod.rs:1375, 1444, 1519`, `crates/blit-cli/src/transfers/remote_remote_direct.rs:373` — test fixtures gain `files_from_text: None`.
- `crates/blit-cli/tests/saved_jobs.rs:293-336` — `a_job_keeps_the_list_its_run_overwrote`, the reviewer's end-to-end scenario.

## Guard proof
- Guard: `run_log::tests::the_spec_keeps_the_list_the_run_started_with` (blit-cli unit test). It writes a list (`a.txt`, `# kept`, a blank line, `list.txt`), captures it with `capture_files_from`, overwrites the file with `b.txt`, runs `begin_record`, and loads the spec back from the run store: it must hold the captured four lines.
- Mutation: `job_spec`'s captured-text arm changed back to reading the file — `(Some(list), Some(_)) => { let text = std::fs::read_to_string(cwd.join(list)).map_err(|error| format!("{}: {error}", list.display()))?; ... }`.
- Red: `cargo test -p blit-transfer --bin blit run_log::tests` → `the_spec_keeps_the_list_the_run_started_with ... FAILED`, "assertion `left == right` failed", `left: Some(["b.txt"])`, `right: Some(["a.txt", "# kept", "", "list.txt"])` — the spec held the overwritten file's line, not the captured list.
- Restore: the fixed file copied back; the same command → 5 passed, the guard `ok`.
- The end-to-end test `saved_jobs::a_job_keeps_the_list_its_run_overwrote` (a copy with `--files-from dst/list.txt`, `list.txt` listed in itself, so the copy overwrites it; `--export` must show the original `["a.txt", "list.txt"]`) passes with the fix but is **not** a guard: it cannot be made to fail deterministically without the fix, because the record's thread usually reads the list before the copy writes it. Under the same mutation it passed 5 of 5 runs on macOS. The unit test above is the proof.

## Known gaps
- The capture and the transfer's filter are still two reads of the list, as they were before jl-5 (when `job_spec` and the filter each read it). The transfer writes nothing before its filter is built, so the transfer itself can no longer make them differ; something else rewriting the list in the moment between the two reads still could, as it could before jl-5.
- Verified on macOS (aarch64) only, plus Linux cross-clippy; the new tests sit in ungated modules and use relative endpoints, but were not executed on Linux or Windows.

## Reviewer comments
(the re-review's verdict, when dispatched)
