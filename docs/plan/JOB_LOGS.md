# Job logs — every job's full record, retrievable, and retryable from it

**Status**: Draft
**Created**: 2026-10-07
**Supersedes**: nothing
**Decision ref**: pending (set when the owner flips this to Active)

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
for its own settings, and its job records beside its existing recents.

- R10 (settings format): "toml if it makes sense. that is up to you. if json
  is simpler, fine". Agent's call: TOML for the settings file — a person
  edits it, TOML allows comments, and the daemon's settings are already TOML
  (one format for both); JSON for run records and saved jobs — machine-
  written, and the format the owner asked records to be stored in (R1).

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

- [ ] (to be written once the open questions below are answered)

## Design

(to be written once the open questions below are answered)

## Slices

(to be cut once the design is agreed; the first slice stays small, per R1)

## Open questions

Asked one at a time, in this order:

- Q1. Which runs write a job record? — RULED by R5/R6: every run (a
  successful run is likely reused); every machine involved in a job keeps it
  — for remote-to-remote, both daemons (R6).
- Q2. For a remote-to-remote job, which is "the original machine" (R4)?
  — open; R6 puts the record on both daemons, so retry needs a rule for which
  end re-runs it. — owner
- Q3. What `blit retry` re-runs: only the files that failed, with the
  original options, or the whole job? — owner
- Q4. How long records are kept — RULED by R7: the last 50 by default,
  configurable in `config.toml` (R9: TOML, per-user folder; the daemon's in
  its own `config.toml`). Whether a saved job (R5) counts toward the
  50 is open (proposed: no — a saved job is kept until deleted). — owner
- Q5. The cleaned-up command surface for R5 (`--save <name>`, export by job
  ID, running a saved job, listing and deleting saved jobs) — proposed to the
  owner 2026-10-07, awaiting a ruling. — owner
