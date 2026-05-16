# m-jobs-6-watch: `blit jobs watch` polls GetState until transfer drains

**Severity**: Feature (new CLI verb, no protocol changes)
**Status**: In progress / pending review
**Branch**: `phase5/m-jobs`
**Commit**: filled by the sentinel commit

## What

Final M-Jobs CLI sub-slice — the polling-via-GetState
stopgap the design doc (§6.5) specifies until C's
`Subscribe` RPC lands. With this slice the operator-facing
surface for daemon-owned transfers is complete:

```
blit copy --detach src:/m/ dst:/m/      # fire and forget
blit jobs watch dst <transfer_id>       # follow until done
blit jobs cancel dst <transfer_id>      # change mind midway
```

## Deferred items

- **m-jobs-4 (per-job event ring inside ActiveJob)** —
  defers to milestone C. The ring exists solely as a
  producer for C's `Subscribe` "catch up to recent events"
  semantic; nothing consumes it without Subscribe in flight.
  Building it now would create dead infrastructure with no
  consumer, which the reviewer has consistently flagged.
  C lands the ring alongside its producer and subscriber.
- **m-jobs-5 (`SubscribeRequest.transfer_id_filter` proto
  field)** — defers for the same reason. `SubscribeRequest`
  itself doesn't exist until C; the field can't be added to
  a message that isn't defined. C lands both.

Captured as "Known gaps" rather than M-Jobs todos that need
their own slices — the design doc's M-Jobs scope is "the
things M-Jobs ships on its own merits," and these two are
solely C-supporting infrastructure.

## Approach

`WatchSnapshot` enum (`Active` / `Finished` / `NotFound`) +
pure `watch_snapshot(state, transfer_id)` helper in
`blit_app::admin::jobs`. CLI runner polls `GetState` at the
configured interval and matches on the snapshot.

`recent[]` is consulted before `active[]` for id lookup
because the recent record is the terminal authoritative one
— if a Drop-then-push race somehow left the id in both, the
terminal outcome wins.

Exit codes carry semantic meaning:

|  Code  | Meaning                                     |
|--------|---------------------------------------------|
| 0      | Finished + ok                               |
| 1      | Finished + failed                           |
| 2      | NotFound (never seen, or rotated out)       |
| 3      | Timeout while still active                  |

NotFound on first poll is the documented behavior for
transfers that completed before the watch started and
rotated out of the daemon's recent ring (default depth 50).
Subscribe (milestone C) will eliminate this race by giving
the daemon push semantics; the polling approach inherits the
race by construction.

## Output

Human (stderr — keeps stdout reserved for any future tabular
or JSON output the verb may grow):

```
Watching transfer t1779-42 on host-b (poll 1000ms)...
[active] delegated_pull mod/path peer=10.0.0.5:443 age=2.4s
[active] delegated_pull mod/path peer=10.0.0.5:443 age=3.4s
[done] delegated_pull mod/path duration=4.2s ok
```

JSON (stdout, JSON-Lines so consumers can `jq -c .` over the
stream):

```
{"state":"active","transfer_id":"t1779-42",...,"bytes_completed":0}
{"state":"active","transfer_id":"t1779-42",...}
{"state":"finished","transfer_id":"t1779-42",...,"ok":true}
```

## Files changed

- `crates/blit-app/src/admin/jobs.rs`:
  - `+WatchSnapshot` enum.
  - `+watch_snapshot(state, transfer_id) -> WatchSnapshot`
    pure helper.
  - +4 unit tests:
    - `watch_snapshot_finds_active_row`
    - `watch_snapshot_finds_finished_row`
    - `watch_snapshot_not_found_when_absent_from_both`
    - `watch_snapshot_prefers_finished_when_both_present`
- `crates/blit-cli/src/cli.rs`:
  - `+JobsCommand::Watch(JobsWatchArgs)`.
  - `+JobsWatchArgs { remote, transfer_id, interval_ms,
    timeout_secs, json }`.
- `crates/blit-cli/src/jobs.rs`:
  - `+run_jobs_watch` dispatcher entry.
  - `+print_watch_json` formatter (JSON-Lines).
  - `run_jobs` dispatch grew the new variant.

## Tests added

4 in `blit_app::admin::jobs::tests` (listed above). The CLI
runner itself is uncovered by unit tests — it polls an RPC,
which requires an in-process tonic server to exercise.
Same posture as `run_jobs_list` and the cancel path, which
defer that to integration testing.

Workspace: 544 passed (was 540; +4).

## Known gaps

1. **NotFound-on-first-poll race against the recent ring.**
   If a transfer completes faster than the operator can
   type `blit jobs watch`, and the recent ring rotates the
   row out before the watch query lands, the watch returns
   NotFound. Documented behavior under polling; Subscribe
   (milestone C) replaces polling with daemon-push and
   eliminates the race.

2. **No `--from-start` flag.** A future iteration could
   accept an "I expect this id to exist; wait up to T
   seconds for it to appear" mode for scripted callers
   that spawn detach + watch back-to-back. Out of scope —
   detach output already prints the id before `Started`
   returns, so a back-to-back invocation is fine.

3. **Bytes/files columns are zero in human output.**
   They're stuck at zero in the wire shape until
   milestone C wires byte-level instrumentation; the JSON
   surfaces them as zeros for consumer stability. Human
   output omits them entirely — adding them when they're
   always zero just clutters the line.

4. **No CLI unit-test of `run_jobs_watch`.** Loop driven
   off `jobs::query` (a network call); covering it
   end-to-end requires an in-process tonic server with a
   scripted GetState response sequence. Out of scope for
   this slice; same posture as the rest of the `blit jobs`
   surface.

## Round 2 (sha `5ab9eef`)

Addressed all three reviewer findings:

1. **Planning docs aligned with M-Jobs vs C scope.**
   - `docs/plan/TUI_DESIGN.md` §6.5 CLI surface: `blit jobs watch`
     now documented as a GetState polling loop in M-Jobs (with
     `--interval-ms` / `--timeout-secs` / `--json`), with the
     `Subscribe`-stream upgrade explicitly deferred to milestone C.
   - `TODO.md` Phase 5 M-Jobs row: detach + cancel + watch-as-
     polling now described; per-job event ring +
     `SubscribeRequest.transfer_id_filter` explicitly listed as
     deferred to C.
   - `TODO.md` Phase 5 C row: absorbs both deferred items (event
     ring and `transfer_id_filter`) plus the streaming upgrade.

2. **JSON timeout now produces a terminal state line.**
   - New `print_watch_timeout_json(transfer_id, timeout_secs)` in
     `crates/blit-cli/src/jobs.rs` emits
     `{"state":"timeout","transfer_id":"...","timeout_secs":N}`
     before the timeout branch returns exit code 3.
   - Stream contract is now consistent: every terminal exit (0, 1,
     2, 3) emits one final JSON object describing the outcome.
   - The `--json` flag docstring in `JobsWatchArgs` already said
     "one object per poll, plus a final outcome line" — the
     implementation now matches that contract for all four exits.

3. **`WatchSnapshot` rustdoc no longer inherits `kind_label`'s.**
   - Split the doc comments cleanly: `WatchSnapshot` carries only
     its own docstring; `kind_label`'s "Human-readable label..."
     doc-block lives directly above the function again.

## Round 3 (sha `6ff5480`)

Reviewer's round-2 verdict left one Medium open: round 2's doc
sweep had fixed the §6.5 CLI-surface paragraph and the TODO rows
but missed three later sections in `TUI_DESIGN.md` that still
claimed M-Jobs ownership of `Subscribe`-scoped pieces. Round 3
finishes the sweep:

- **§6.5 "M-Jobs introduces" list.** Removed the bullets for
  the per-job event ring and `SubscribeRequest.transfer_id_filter`.
  Replaced them with a bullet for the `blit jobs watch` verb
  (the thing M-Jobs actually ships) and added a new "Deferred
  from M-Jobs to milestone C" subsection that explicitly owns
  the event ring and the filter field, with the reasoning
  (dead infrastructure without `Subscribe`).
- **§11 Phasing summary table.** M-Jobs row's wire-changes
  column no longer claims `transfer_id_filter`; C row now lists
  `Subscribe` + `transfer_id_filter` + per-job event ring +
  jobs-watch streaming upgrade.
- **§12 Structural commitments.** Spelled out which milestone
  lands which field in the RPC contract list: M-Jobs ships
  `GetState` / `CancelJob` / `detach` / `DelegatedPullStarted.transfer_id`;
  milestone C ships `Subscribe` and `SubscribeRequest.transfer_id_filter`.

No code changes (per the reviewer's note "the code does not
need changes for this finding").

## Reviewer comments

(empty — pending grade)
