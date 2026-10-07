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

## Acceptance criteria

- [ ] (to be written once the open questions below are answered)

## Design

(to be written once the open questions below are answered)

## Slices

(to be cut once the design is agreed; the first slice stays small, per R1)

## Open questions

Asked one at a time, in this order:

- Q1. Which runs write a job record: every blit run (local copies, pushes,
  pulls, remote-to-remote) on the machine that ran it, or only jobs a daemon
  runs? — owner
- Q2. For a remote-to-remote job, which is "the original machine" (R4): the
  one where the command was typed, or the daemon host that ran the transfer?
  — owner
- Q3. What `blit retry` re-runs: only the files that failed, with the
  original options, or the whole job? — owner
- Q4. How long records are kept (count, age, or size), given that a full
  `-v` record of a large job lists every file. — owner
