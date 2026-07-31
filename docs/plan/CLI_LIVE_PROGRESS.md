# CLI Live Progress

**Status**: Active (D-2026-07-31-2, owner "flip clp")
**Created**: 2026-07-31
**Supersedes**: nothing (the queued "CLI transfer output redesign"
TODO — persistent stats block, "probably a TUI", re-confirmed in
D-2026-07-09-1's Why note — remains a separate, later effort; this plan
only makes the existing `-p`/`-v` flags truthful on one stable row)
**Decision ref**: D-2026-07-31-2

## Goal

`blit mirror -p` (and every local transfer) shows ONE stable, in-place
updating status row — phase, file counts, bytes, current activity — that
survives the whole run without scrolling artifacts; `-v` adds clean
per-file completion lines that scroll above the status row. No raw
`eprintln!` from library code ever fights the row.

Motivating failure (2026-07-31, owner session): a local mirror with `-p`
renders a static spinner ("Mirroring D:\Apps\ → H:\apps\") that is
scrolled off-row once per second by blit-core's unconditional
`eprintln!("Enumerated {} entries… (streaming manifest)")`, leaving a
wrapped trail of dead spinner remnants; `-v` changes nothing until the
end summary. The owner cannot distinguish enumerating from comparing
from copying.

## Non-goals

- The persistent-stats-block / TUI redesign (separate queued effort).
- Remote-transfer display overhaul: remote paths keep their current
  rendering; they benefit only from the shared enumeration-event change.
- JSON/NDJSON output modes: byte-identical behavior (R46-F4: `--json`
  owns stdout; progress stays on stderr).
- No new CLI flags; `-p`/`-v`/`--progress` semantics per `cli.rs`
  (`effective_progress`, cli.rs:374-380) are unchanged in *when* they
  engage — only in *what* they show.
- Daemon/serving behavior: a SOURCE daemon with no attached progress sink
  keeps the current once-per-second stderr line (it lands in daemon logs,
  not a TTY).

## Constraints

- Single-writer rule for transfer-time stderr: while a progress row is
  live, every line print goes through the indicatif handle
  (`ProgressBar::println` / `suspend`) — nothing writes raw stderr.
- No blocking calls in async contexts; the render loop consumes the
  existing unbounded event channel.
- Status row must be width-safe: truncate the current-file segment to the
  terminal width (indicatif template `{wide_msg}` or explicit truncation)
  so the row never wraps.
- Test count never drops; rendering logic lands as pure functions with
  unit tests (format-a-snapshot → expected string).

## Acceptance criteria

- [ ] `blit mirror -p <src> <dst>`: one status row, updated in place,
      never duplicated or scrolled during enumeration, diff, copy, and
      mirror-delete phases (manual check on a real run; automated: the
      renderer emits carriage-return updates through one ProgressBar and
      no raw eprintln fires while a sink is attached — unit-testable via
      the sink-attached gate).
- [ ] The row carries: phase label, files completed / enumerated-so-far,
      bytes written, and (during copy) the current file name, truncated
      to width.
- [ ] `-v` with `-p`: per-file completion lines print via the indicatif
      handle and scroll cleanly above the intact status row.
- [ ] Without `-p`: behavior unchanged (quiet run, end summary, the
      existing once-per-second enumeration line).
- [ ] Source-side enumeration reports through the progress lane when a
      sink is attached; the raw `eprintln!` fires only when no sink
      exists. Guard: with a sink attached, zero "streaming manifest"
      lines reach raw stderr (test asserts the gate; red when the gate is
      reverted).
- [ ] `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`
      green; docs gate passes.

## Design

### Current state (verified 2026-07-31)

- CLI flags: `cli.rs:218-230` (`verbose`, `progress`,
  `effective_progress` at :374-380).
- Local spinner: `blit-cli/src/transfers/local.rs:118-135` — static
  `ProgressBar::new_spinner()` with fixed message; finished at :140-142.
- The dead flag: `LocalMirrorOptions.progress: bool`
  (`transfer_session/local.rs:146`) is carried but never consulted; the
  local apply starts with `progress: None`
  (`transfer_session/local.rs:643`), even though
  `LocalApply::start(&self, progress: Option<RemoteTransferProgress>)`
  (`local.rs:330`) already threads a sink into the pipeline's
  `report_payload`/`report_file_complete` calls
  (`remote/transfer/pipeline.rs:498-518`).
- Event vocabulary already sufficient: `ProgressEvent`
  (`remote/transfer/progress.rs:38`), `RemoteTransferProgress::new(
  UnboundedSender<ProgressEvent>)` with `report_manifest_batch`,
  `report_payload`, `report_file_complete` (progress.rs:599-626).
- The colliding print: `remote/transfer/source.rs:336-346` — the
  enumeration loop `eprintln!`s once per second, unconditionally, from
  library code (R46-F4 comment: stderr-not-stdout is deliberate; the
  collision with a live row is not).

### Approach

1. **Wire the sink (clp-1).** `run_local_session` constructs an
   unbounded `ProgressEvent` channel when `LocalMirrorOptions.progress`
   is set, passes `RemoteTransferProgress` into `LocalApply::start`, and
   exposes the receiver to the caller (surface it on the options/return
   path the way the remote route does). Source-side enumeration gains an
   optional progress handle: when attached, the once-per-second count
   reports via `report_manifest_batch` instead of `eprintln!`; the
   eprintln remains for sink-less callers (daemon logs). The destination
   diff already knows manifest batches; enumeration counts come from the
   SOURCE side of the local session (both ends in-process for local).
2. **Render (clp-1 minimal, clp-2 polish).** `blit-cli` replaces the
   static spinner with a consumer task draining the receiver into one
   `ProgressBar`: phase transitions (enumerating → comparing/copying →
   deleting), counters, bytes; `-v` routes per-file completions through
   `pb.println`. Rendering state → string is a pure function with unit
   tests; the task ends when the channel closes, then the CLI prints the
   existing summary.
3. **Row hygiene (clp-2).** Width-safe truncation of the current-file
   segment; steady tick retained; a final `finish_and_clear` before the
   summary (as today, local.rs:140-142).

### Risks

- Local sessions run source and destination in one process; the
  enumeration handle must not introduce a second channel consumer that
  races the CLI's renderer — one channel, one consumer, events from both
  roles.
- Progress events are unbounded-channel sends from hot paths; the
  renderer must drain promptly (indicatif tick + drain loop), never
  block the pipeline.
- `report_file_complete` currently fires per payload completion in the
  pipeline; per-file failure containment (pfc-2, in flight) changes which
  files report complete — clp work rebases on whatever pfc-2 lands.

## Slices

One coherent, testable change per slice — sized for the review loop.

1. **clp-1** — sink wiring end to end: enumeration-event gate in
   `source.rs` (sink → `report_manifest_batch`, no raw eprintln),
   `run_local_session` channel construction behind `options.progress`,
   CLI consumer task with the minimal live row (phase + counts + bytes),
   guard tests for the gate and the wiring.
2. **clp-2** — render polish: current-file segment with width-safe
   truncation, `-v` per-file lines via the indicatif handle, phase
   labels incl. mirror-delete, pure-function format tests.

## Open questions

- None. Flipped Active by the owner ("flip clp", D-2026-07-31-2).
  Execution order: clp-1/clp-2 run after pfc-2 lands (clp rebases on
  pfc-2's file-complete semantics), before pfc-3..6.
