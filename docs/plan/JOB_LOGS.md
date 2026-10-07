# Job logs — every job's full record, retrievable, and retryable from it

**Status**: Active — owner: "go" (2026-10-07, D-2026-10-07-5), after the r1 review's changes were adopted (R17)
**Created**: 2026-10-07
**Supersedes**: nothing
**Decision ref**: D-2026-10-07-5

## Goal

Every job leaves a durable record of everything it did, so an operator can
read it after the fact and a failed job can be finished later. Each job has an
ID. Its record is stored as JSON on the machine that ran it, can be pulled as
JSON or as readable text, holds everything a `-p -v` run would have shown plus
the internal detail forensics needs, and holds enough to run the job again:
`blit retry <job-id>` or `blit retry <path-to-json>` re-runs a failed job on
the machine where it originally ran.

Requirements as the owner stated them (2026-10-07, verbatim):

- R1: "daemon logs need to be retrievable. stored locally in json, with a json
  to txt converter option so the user can pull either format. that means job
  IDs. this could spiral, so start small."
- R2: "saved logs should contain everything a -p -v run would show, plus
  whatever internal stuff is needed, and anything else useful for forensics."
- R3: "jobs should be retryable if failed via another run, so blit retry
  <jobid> or <path_to_json>. finished jobs' json files therefore need to store
  what's needed for a retry."
- R4: "this should probably be limited to the original machine to avoid the
  footgun."

Shaping, same day (the owner thinking aloud, then ruling):

- Brainstorm: save jobs, local or daemon, "in the same .json format as the
  retry jobs, then we can do a blit job <job.json> or something like that",
  with every daemon run stored as "a job_run-<timestamp>.log or similar for
  retries"; then "we need a way to manage jobs, prune old ones, etc. saving
  every local run by default seems like a burden. simplify it for me."
- R5 (rejecting "only failed local runs leave a record"): "no. successful
  runs are pretty likely to be reused. backup tasks, stuff like that. I think
  we need a --save <jobname> and a --export that takes a job ID and works
  in-line with --save to save it to the .blit folder internally and export to
  the filename specified. that needs to be cleaned up, but you get the idea."
- R6: "daemon logs need to live on both daemons, or log hunting will be
  annoying."
- R7 (retention): "50 default, blit.conf option for others".
- R8 (rejecting a two-command surface and dropping saved jobs): "no, result
  of above." — saved, reusable jobs are in scope.
- R9 (where settings and job files live): "config can't be in /etc because
  this is also a windows app. toml is fine, details are up to you. ~/.blit is
  universal, or we can use the userprofile to keep it in AppData\Local,
  ~/Library/Application Support, etc."

Agent's design call under R9's delegation (2026-10-07): one per-user folder
per machine, the platform's own — macOS `~/Library/Application
Support/com.Blit.Blit` and Linux `~/.config/blit` (the folders
`config_dir()` already resolves, holding `recents.jsonl` and the perf
history), and Windows `%LOCALAPPDATA%\Blit` (moved from the roaming
`%APPDATA%\Blit\Blit\config` `config_dir()` returns today, because job
records can be large and a roaming profile copies them at every sign-in; the
existing small files move with it once). Inside it: `config.toml` (settings,
e.g. how many run records to keep), `jobs/runs/` (run records) and
`jobs/saved/` (saved jobs). The daemon keeps its existing machine-wide
`config.toml` (`/etc/blit/config.toml` on Linux/macOS,
`C:\ProgramData\Blit\config.toml` on Windows — Windows is already covered)
for its own settings, and its job logs in `jobs/logs/` under its state
directory — systemd's `$STATE_DIRECTORY` when the unit sets
`StateDirectory=`, else its config folder, the rule its performance
history already follows (decided at jl-1b, 2026-10-07: under
`ProtectSystem=strict` only the state directory is writable).

- R10 (settings format): "toml if it makes sense. that is up to you. if json
  is simpler, fine". Agent's call: TOML for the settings file — a person
  edits it, TOML allows comments, and the daemon's settings are already TOML
  (one format for both); JSON for run records and saved jobs — machine-
  written, and the format the owner asked records to be stored in (R1).
- R11 (on the proposed `blit jobs export`): "also needs an in-line --export
  that works with --save to save it locally and exported." Owner's question
  on the proposed `blit retry`: "why not jobs retry?" (answered in Design).
- R12 (confirming the command surface): "right, then we can have a blit jobs
  --help for specific help. we should have that same syntax for all verbs.
  Yes." Verified 2026-10-07: every verb and sub-verb already answers
  `--help` with its own help (clap); the new `jobs` sub-verbs inherit it.
  `blit jobs`'s one-line help ("Inspect transfer jobs on a remote daemon")
  must change to cover local jobs.
- R13 (correcting the proposal that a daemon job could be retried from
  anywhere): "no, JOBS are local. LOGS are both. sorry that was unclear."
- R14 (what a log holds, after seeing that blit's `-v` lists no files, only a
  few diagnostic lines): "okay, so keep -v. drop -p unless that is relevant
  forensically".
- R15 (Q6, naming files): "B" — a log names every file copied, deleted, and
  failed.
- R16 (Q7): "store logs compressed."
- R17 (2026-10-07, on the r1 review's four material changes, presented with
  "logging never fails a transfer" inside the fourth): "yes" — all four
  adopted (Design: "Three artifacts", "Detached jobs", "Log keys and event
  schema", "Crash safety, pruning and backpressure"; slices re-cut).

## Non-goals

- Spiralling scope (R1: "start small"): the first slices deliver the record,
  its retrieval and `blit retry`; anything beyond that waits for its own
  ruling.
- A new user-visible option the owner did not ask for (repo rule): the owner
  asked for `blit retry` and for pulling a log as JSON or text; any other
  knob is hidden or worked out at runtime.

## Constraints

- FAST, SIMPLE, RELIABLE (D-2026-07-09-1): writing the record must not slow a
  transfer measurably; the user-facing surface stays the commands the owner
  asked for.
- Retry runs only on the machine where the job originally ran (R4).
- Wire changes land under contract 7 (unreleased; v0.1.3 is contract 6), per
  SOURCE_SIDE_CONTAINMENT's Constraints, and nothing ships until the open work
  is done (D-2026-10-07-3).

## Facts this builds on (verified 2026-10-07)

- Every daemon job already has an ID (`transfer_id`), shown by `--detach` and
  `blit jobs list`; `blit jobs watch` and `blit jobs cancel` take it.
- The daemon already persists its last 50 finished jobs as JSON lines
  (`recents.jsonl` in its config dir, `crates/blit-daemon/src/recents_store.rs`),
  but each record (`TransferRecord`) holds only ok/failed, bytes, files and a
  whole-job error message — no per-file failures and no log.
- Defect (c) of 2026-10-07 (STATE): a `--detach` job whose files failed one by
  one is recorded ok, and `blit jobs watch` reports success. A job record that
  carries the failures closes it.
- There is no `blit.conf` and no `.blit` folder today. The daemon reads a TOML
  config (`/etc/blit/config.toml`, `C:\ProgramData\Blit\config.toml`;
  `docs/DAEMON_CONFIG.md`); the CLI has no config file, only its per-user
  data folder (`blit_core::config::config_dir()`: `~/Library/Application
  Support/com.Blit.Blit` on macOS, `~/.config/blit` on Linux), which already
  holds `recents.jsonl` and the perf history. R7's "blit.conf" and R5's
  ".blit folder" map onto these unless the owner rules otherwise.

## Acceptance criteria

- [ ] A `--detach` remote-to-remote job with one file that fails on its own:
      `blit jobs watch` exits non-zero and names the file; `blit jobs list`
      shows a failed-file count; `blit jobs log <host> <job-id>` shows the
      file and its reason (closes defect (c) of 2026-10-07).
- [ ] Every run — local copy, push, pull, remote-to-remote — leaves a log on
      each machine involved, all under one job ID; `blit jobs log` reads each
      as text and, with `--json`, as the raw events.
- [ ] A log names every file copied, deleted and failed (with reasons), holds
      what `-v` adds, phase start/end times and stall notices; it is written
      as the run goes and stored compressed.
- [ ] Retention: with the default, the 51st run's log replaces the oldest;
      the `config.toml` value is honored; saved jobs are never removed by it.
- [ ] `--save <name>`, `--export <file>`, `blit jobs save|export|run|delete|list`
      behave as the Design's command surface says; `blit jobs run <name>`
      re-runs the same transfer.
- [ ] `blit jobs retry <job-id|file>` re-sends only the files that failed,
      with the original options; on another machine it refuses with a plain
      message naming the machine the job belongs to.
- [ ] Windows keeps settings, jobs and logs under `%LOCALAPPDATA%\Blit`; files
      found in the old roaming folder are moved once.
- [ ] FAST: logging costs no measurable slowdown on the existing small-file
      and large-file benches (within run-to-run noise).
- [ ] Every new sub-verb answers `--help` with its own help (R12).
- [ ] Crash: a daemon killed mid-run leaves a log that startup finalizes as
      interrupted and `blit jobs log` reads.
- [ ] Logging failure: with the log directory unwritable the transfer still
      completes, and its report says the log is incomplete.
- [ ] Detached: after a `--detach` job finishes, `blit jobs list` on the
      initiating machine shows its outcome (fetched from the daemon) and
      `blit jobs retry` re-runs exactly its failed files.
- [ ] Saved jobs reproduce: `blit jobs run <name>` from another working
      directory, or after the `--files-from` file changed, transfers exactly
      what the original run did.
- [ ] Self-delegation: a daemon that is both ends of one run keeps two
      distinct logs, labeled by role.

## Design

### Command surface (confirmed by the owner 2026-10-07, R12)

Two kinds of saved file, one JSON format: **run records** (written by every
run on every machine involved, newest 50 kept, R5/R6/R7) and **saved jobs**
(written only on request, kept until deleted).

1. `blit copy|mirror|move … --save <name>` — run it and keep it as a saved
   job `<name>` in the per-user folder.
2. `blit copy|mirror|move … --export <file>` — run it and also write its job
   to `<file>`; with `--save`, both (R11).
3. `blit jobs save <job-id> <name>` — keep a past run as a saved job.
4. `blit jobs export <job-id|name> <file>` — write a run record or saved job
   to a file.
5. `blit jobs run <name|file>` — run a saved job again.
6. `blit jobs retry <job-id|file>` — re-send only what failed. Moved under
   `jobs` from R3's `blit retry` on the owner's question ("why not jobs
   retry?"): every job action then lives under `jobs`, and it cannot be
   confused with the transfer flags `--retry`/`--retries`.
7. `blit jobs log [<host>] <job-id>` — show a record as text; `--json` for
   the raw file.
8. `blit jobs list [<host>]`, `blit jobs delete <name>`.

### Jobs and logs (R13)

- A **job** — what to run again, and how its last run ended (outcome and the
  exact failed paths) — lives only on the machine where the command was
  typed. `jobs save`, `jobs export`, `jobs run` and `jobs retry` act there and
  nowhere else (R4). A remote-to-remote job's failed paths already come back
  to the CLI in the delegated summary (`failed_paths`, exact), so its local
  job holds them. A `--detach` job's outcome is reconciled from the
  receiving daemon (see "Detached jobs").
- A **log** — the forensic record of everything a `-p -v` run would show plus
  internal detail (R2) — is kept by every machine that took part, each for
  its own part, under the same job ID: the local machine for a local copy;
  the CLI machine and the daemon for a push or pull; both daemons for a
  remote-to-remote job (R6), and the CLI machine for what it saw.
  `blit jobs log [<host>] <job-id>` reads any of them.

### What a log holds (R2, R14)

- The run's identity: job ID, machine, verb, source, destination, options,
  start and end time, and the outcome.
- Everything `-v` adds today, whether or not `-v` was given: scan time,
  average rate, workers used, the planned count and bytes, and how files
  were batched (verified 2026-10-07: blit's `-v` lists no file names).
- The final summary: counts of files copied, deleted, failed; bytes.
- Every failed file with its reason.
- From `-p`, only what answers after the fact where time went (agent's call
  under R14): when each phase (scan, transfer, delete) started and ended, and
  any stall notice. The redrawn progress line and periodic progress
  snapshots are not kept.
- Every file copied and every file deleted, by name (R15).
- Size (agent's design, follows from R15): a million-file backup's log is
  on the order of 100 MB, and 50 are kept per machine. The log is written to
  disk as the run goes — one JSON event per line — never held in memory, so
  a large job costs disk, not RAM, and is compressed when the run finishes
  (R16). `blit jobs log` reads it as text or JSON transparently; `blit jobs
  export` writes it uncompressed.

### Three artifacts (R17, review r1 MC1/F2)

One family of versioned JSON documents, each with a `format` and `version`
field and explicit migrations from every earlier version:

- **JobSpec** — what to run. Written at submission, never changed. Holds the
  verb, every transfer-affecting option (and only those), the working
  directory the command was typed in, local endpoints resolved to absolute
  paths, remote endpoints as typed locators (host, port, module, path), the
  *contents* of any `--files-from` list (not its path), and the machine ID.
  Validation on load rejects a spec this build cannot run faithfully. A
  saved job (`--save`, `jobs save`) is a JobSpec under a name.
- **RunRecord** — one run of a JobSpec: run ID, attempt number, parent run
  (for a retry), start and end, outcome, counts, and the exact failed paths
  with the left-in-place and removed sets. States: running, finished,
  interrupted, or waiting on a named daemon (detached). Lives on the
  initiating machine only (R13).
- **EventLog** — one participant's forensic record of one run (R2/R14/R15),
  on every machine involved (R6); see "Log keys and event schema".

### Detached jobs (R17, review r1 MC2/F1)

A `--detach` run writes its JobSpec and a RunRecord in the state "waiting on
<daemon>" before the CLI exits. Every local `jobs` operation that reads a run
— `list`, `log`, `save`, `export`, `retry` — first asks that daemon for the
run's outcome by run ID; once the daemon reports it finished, the local
RunRecord is updated atomically and the daemon is not asked again. If the
daemon is unreachable, the operation says so and shows the record as still
waiting — `retry` refuses until the outcome is known.

### Log keys and event schema (R17, review r1 MC3/F4)

- A log is keyed by **run ID + participant + role + attempt**: the participant
  is the machine ID, the role is `initiator`, `source` or `destination`, and
  the attempt counts retries of the run. One daemon that is both the
  delegated destination and the served source of the same run therefore
  writes two logs, never one interleaved file. `blit jobs log` shows the
  participant logs it finds, labeled, or one with `--role`.
- The event schema is defined and versioned before any storage or retrieval
  is built (slice jl-1a): one event per line, each with a timestamp, a
  sequence number and a kind (`run-start`, `phase`, `file-copied`,
  `file-sent`, `file-deleted`, `file-failed`, `stall`, `diagnostic`,
  `summary`, `run-end`, `log-incomplete`). Settled at jl-1b, before any
  release: `file-sent` names what a source sent (whether it landed is the
  destination's to say), and `file-copied` carries the size only when the
  recorder knows it.

### Crash safety, pruning and backpressure (R17, review r1 MC4/F3)

- **While running** a log is written to `<key>.partial.jsonl`, appending whole
  lines and syncing at phase changes and every few seconds; a torn last line
  is tolerated on read. **Finishing** compresses it to `<key>.jsonl.gz` and
  renames it into place in one step, then removes the partial.
- **After a crash**, startup finds every `.partial.jsonl` without a live
  owner, appends a `run-end` marked interrupted, and finalizes it; the
  RunRecord, where it lives on this machine, becomes interrupted.
- **Retrieval** serves an active partial, a recovered log and a finished log
  alike.
- **Pruning** runs when a run finishes, holds a lock, and only ever removes
  finished logs and records beyond the newest 50 — never a partial, never a
  saved JobSpec.
- **Backpressure:** events go through a bounded queue to one writer per log;
  when the writer falls behind, the transfer waits for it rather than drop
  events, because a log with holes misleads.
- **Logging never fails a transfer** (R17): if a log cannot be written (disk
  full, permissions), logging stops, the transfer continues, a
  `log-incomplete` event is attempted, and the run's report says the log is
  incomplete.

### Identity, protocol and retention (agent's design within the rulings)

- **Job ID.** The machine where the command is typed creates the ID (a
  random 128-bit ID shown in short form, like `--detach` shows today) and
  sends it with the transfer, so every machine involved logs under the same
  ID. A daemon still creates one for a job no CLI started.
- **Machine ID.** Created once per machine and kept in the per-user folder;
  every job records it, and `blit jobs retry`/`run` refuse a job whose ID is
  not this machine's (R4). Hostnames change, so they are not used.
- **Protocol (contract 7, unreleased).** The job ID rides the session-open
  and delegated-pull requests; a new `GetJobLog` call returns one log by ID;
  the finished-job record (`TransferRecord`) gains the failed-file count.
- **Retention.** Per machine, the newest 50 logs and the newest 50 run
  records (R7) — `[jobs] keep = 50` in the CLI's new per-user `config.toml`
  and in the daemon's existing `config.toml`. Saved jobs are kept until
  deleted. Pruning happens when a run finishes; nothing to manage.
- **Exposure.** A daemon's logs name files; `blit jobs log <host>` reads them
  with the same reach `blit ls`/`find` already have on that daemon — no new
  access.

## Slices

Small first, per R1; each slice is one coherent, testable change.

1. **jl-1a — the event schema and a crash-safe log writer (the small start).**
   The versioned event schema; the per-log writer (bounded queue, partial
   file, sync points, compress-and-rename on finish, torn-line tolerance);
   startup recovery of orphaned partials; locked pruning to the newest
   `keep` (default 50); "logging never fails a transfer". Library-level,
   tested without a network.
   **Landed `c4440c54` 2026-10-07** as `blit_core::job_log` (21 tests, each guard
   mutation-proven red), adding `flate2` (its default pure-Rust backend) to
   blit-core for the `.jsonl.gz` files. Reading the `[jobs] keep` setting
   moved to jl-1b, where the daemon first writes logs; jl-1a has nothing that
   reads it.
2. **jl-1b — daemon logs and retrieval.** The daemon writes each job's log
   (keyed run + participant + role + attempt) through jl-1a, runs startup
   recovery, and reads `[jobs] keep` from its `config.toml`; `GetJobLog`;
   `blit jobs log <host> <job-id> [--json] [--role]`.
   **Landed `51da3977` 2026-10-07.** Served pushes and pulls log from the open on,
   delegated pulls from dispatch; the log is owned by the job's dispatcher
   task and closed after the job's record. The engine names each file as
   it lands, fails (new `FileFailed` progress event, at every destination
   record site) or is deleted by the mirror pass (new `Deleted`); the
   summary's exact failed list fills in anything not seen live. The daemon
   makes its machine ID here (planned for jl-3; the CLI reuses it there).
   Known gaps, each for a later slice or its own ruling: no run ID crosses
   the wire yet (jl-2), so a delegated job's two daemon logs carry
   different job IDs; a source's log learns failures only from the
   destination's summary (reasons for the first 64, names exact unless
   that list was cut short); a daemon source records no scan timing; a
   move's source-side delete (`Purge`) is not a job and leaves no log; the
   resume-record failure sites are reported live but not test-pinned
   (the summary still names those files).
3. **jl-1c — failures in the job list.** `jobs list`/`watch` show the
   failed-file count and `watch` exits non-zero when files failed (closes
   defect (c)).
4. **jl-2 — one run ID everywhere, local logs.** The CLI creates the job ID and
   sends it, so both daemons of a remote-to-remote job and the CLI machine log
   under it; local runs write logs too; the per-user folder (Windows moved to
   `%LOCALAPPDATA%\Blit`, old files moved once), the CLI's `config.toml`;
   `blit jobs log <job-id|file>` and `blit jobs list` locally.
5. **jl-3 — jobs.** Every run writes its JobSpec and RunRecord locally
   (versioned, with migrations); machine ID; detached reconciliation for
   every local `jobs` operation; `--save`, `--export`, `blit jobs
   save|export|run|delete`.
6. **jl-4 — retry.** `blit jobs retry <job-id|file>`: a child run (next
   attempt, parent recorded) re-sending only the failed paths with the
   original JobSpec, through the existing retry-pass machinery; refuse on
   another machine; refuse a detached run until its outcome is known.

## Review history

- **r1** (2026-10-07; openreview codex, gpt-5.6-sol @ xhigh, frontier/fallback,
  codex-cli 0.159.3; over `13bff7d8..92192a33`; capability_ok, guard_confirmed,
  SHAs pinned; record `.review/results/joblogs-plan-r1.codex.json`):
  **acceptable_with_changes**. The reviewer's goal statement matches the
  owner's; it endorses the commands, logs on every machine, retention,
  compression, one run identity and the incremental slices. A Claude dispatch
  of the same review was stopped by the owner (D-2026-10-07-4) and is excluded.
  - Material changes — all four ADOPTED by the owner (R17, 2026-10-07):
    MC1 separate and version a JobSpec, a RunRecord and a per-participant
    EventLog instead of one loosely defined JSON format;
    MC2 define detached-job reconciliation for every local jobs operation,
    not only retry;
    MC3 key logs by run ID plus participant, role and attempt, and define the
    event schema before storage and retrieval;
    MC4 specify partial-log recovery, atomic finalization, pruning
    concurrency, writer backpressure and logging-failure behavior, and split
    jl-1 so these are independently testable.
  - Findings, intake 2026-10-07 — all ADMITTED as plan revisions (no code
    exists yet): F1 HIGH a detached run cannot finalize the initiating
    machine's record (the CLI has exited; only retry was given a fetch);
    F2 HIGH saved jobs are not reproducible across working directories,
    changed `--files-from` contents or format changes (raw path strings);
    F3 HIGH post-run compression leaves a crashed run's partial log
    undiscoverable; F4 MEDIUM a job ID alone cannot key logs when one daemon
    is both delegated destination and served source. Each lands in the plan
    with the material change it belongs to (F1→MC2, F2→MC1, F3→MC4, F4→MC3).

## Open questions

Asked one at a time, in this order:

- Q1. Which runs write a job record? — RULED by R5/R6: every run (a
  successful run is likely reused); every machine involved in a job keeps it
  — for remote-to-remote, both daemons (R6).
- Q2. Which machine re-runs a job — RULED (R13): jobs are local to the
  machine where the command was typed; logs live on every machine involved.
- Q3. What a retry re-runs — RULED with the command surface (R12):
  `blit jobs retry` re-sends only what failed, with the original options;
  `blit jobs run` re-runs a whole saved job.
- Q4. How long records are kept — RULED by R7: the last 50 by default,
  configurable in `config.toml` (R9: TOML, per-user folder; the daemon's in
  its own `config.toml`). A saved job does not count toward the 50 — it is
  kept until deleted (proposed with the command surface, confirmed R12).
- Q5. The cleaned-up command surface — RULED (R12): see Design.
- Q6. Whether a log names files — RULED (R15): every file copied, deleted
  and failed.
- Q7. Compress finished logs — RULED (R16): yes.
