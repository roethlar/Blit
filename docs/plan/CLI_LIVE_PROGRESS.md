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
- [ ] Without `-p` **and without a TTY**: behavior unchanged (quiet run,
      end summary, the existing once-per-second enumeration line).
      Clarified at clp-1 landing: `effective_progress()` already engages
      on an interactive TTY without `-p` (existing semantics, unchanged
      in *when* — see Non-goals), so a TTY run gets the row and no raw
      lines; the unchanged-behavior guarantee is for sink-less runs.
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

1. **Wire the sink (clp-1) — landed; as-built notes.** The CLI owns both
   channel ends and passes the sender handle in via
   `LocalMirrorOptions.progress_events: Option<RemoteTransferProgress>`
   (a return-side receiver cannot work: `run_local_session` returns only
   after the transfer ends). Source-side enumeration reports through a
   **new** `ProgressEvent::Enumerated { files }` lane (folded into
   `ProgressTotals.enumerated_files`), NOT `report_manifest_batch` as
   this plan originally said: on a local session the destination diff
   already emits `ManifestBatch` for the need list, so reusing that lane
   would double-count the copy denominator. The once-per-second line AND
   the completion line both gate on sink attachment
   (`EnumerationHeartbeat` in source.rs); sink-less callers print
   byte-identically. Remote routes deliberately do not attach the lane
   yet (their monitor does not render it — see clp-2 candidates).
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
   labels incl. mirror-delete (today an up-to-date tree can show
   "enumerating" for the whole run and mirror-delete shows as
   "copying"), pure-function format tests. Plus the clp-1 review
   residue: **(a)** route the `stderr_log` backend through the live
   row's handle while a row is live — `record_unreadable_entry`'s
   `log::warn!` (source.rs) and pfc-2's `record_failure` warn
   (sink.rs) still scroll the row (reproduced with a deny-ACL file);
   **(b)** LiveProgressRow drain-loop test coverage (extract the
   event→repaint decision so a test can feed a channel);
   **(c)** decide the stderr-redirected `-p` posture (sink attaches
   even when indicatif hides the bar, so redirected stderr gets no
   enumeration liveness at all — either attach only when the bar can
   draw, or accept and record);
   **(d)** consider attaching the enumeration lane on the remote push
   route (`blit-app/src/transfers/remote.rs` builds
   `FsTransferSource::new` with a sink in hand) together with remote
   render support.
3. **clp-3 — colour on the live row and the failure block** (owner asked
   2026-08-01: scope "both", palette "dracula"). **`[x]` LANDED** — see
   §clp-3 landing notes.

   Not a new option and not decoration — see §clp-3 scope below. The row
   currently gives equal visual weight to the phase, the counts and a
   truncated path, and a partial failure (pfc-5) reads at the same weight
   as a success. Colour is used to separate those, and nowhere else.

## clp-3 scope (DRAFT — no code until flipped)

### Palette

Dracula, exact hex. `console` 0.16.4 exposes `Color::TrueColor(u8,u8,u8)`,
so the real palette is available rather than an approximation, and it still
routes through console's capability detection.

| element | colour | hex |
|---|---|---|
| phase `enumerating` | Purple | `#BD93F9` |
| phase `comparing` | Cyan | `#8BE9FD` |
| phase `copying` | Green | `#50FA7B` |
| phase `deleting` | Orange | `#FFB86C` |
| current file path | Comment | `#6272A4` |
| separators (`•`) | Comment | `#6272A4` |
| counts / bytes | default fg | — |
| failure block header | Red | `#FF5555` |
| failed path | default fg | — |
| failure reason | Red | `#FF5555` |
| re-run hint | Comment | `#6272A4` |

`deleting` is Orange rather than Red deliberately: red is reserved for
"something went wrong", and a mirror delete pass is expected work. Orange
still marks it as the destructive phase.

Counts stay default-foreground because they are the numbers the operator
reads; colouring them competes with the two things colour is here to
surface.

### Truecolor fallback

Emit exact Dracula when the terminal advertises 24-bit
(`COLORTERM=truecolor|24bit`), otherwise the standard Dracula 256-colour
approximations (purple 141, cyan 117, green 84, orange 215, comment 61,
red 203). Both paths go through `console::Style`, never hand-written escape
sequences — hand-rolled escapes would bypass the capability detection that
makes the no-colour cases below work.

### Where colour must NOT appear

Each of these is an existing seam, not new logic:

- `NO_COLOR` set (any value) — the de-facto standard.
- `TERM=dumb`.
- stderr is not a TTY. The row already gates on "can the bar draw"
  (clp-2 residue (c)), so piped and redirected output must stay
  **byte-identical to today**.
- `--json`, whose stdout is a machine contract.

### Guards

Given this slice's review history, the guards are specified up front and
each names the property it proves, not a consequence of it:

- **Styling is a pure function of (element, capability)** — extract it so
  the mapping is unit-testable without a terminal, and assert the exact
  escape bytes for truecolor, 256 and none. A test asserting "output is
  non-empty" would prove nothing.
- **No-colour paths are byte-identical to the current output.** Capture the
  rendered row and failure block with colour disabled and compare against
  the existing expected strings — this is the regression that actually
  matters, because it is what every piped consumer and every existing test
  depends on.
- **The failure block is red where it claims to be.** Drive the real binary
  with a forced-colour env and a deny-ACL file, assert the failure header
  carries the red SGR sequence — at the seam, not on the helper.
- Red/green proof required for each, per the standing note in
  `.review/findings/cr-ls1-15-16.md`.

### Non-goals for clp-3

- No colour on the `ls-0` summary block (copied/repaired/average/workers).
  Scope is the row and the failure block, as asked.
- No theming, no `--color` flag, no config. `NO_COLOR` and TTY detection are
  the entire control surface — anything more is user-facing surface for a
  decision nobody needs to make (D-2026-08-01-1).
- No change to what the row SAYS, only how it is rendered.

## clp-3 landing notes

Landed as drafted, with one design constraint the draft had not caught and
which turned out to be the whole shape of the change.

- **Colour is applied AFTER layout, never before.** Escape sequences are
  zero-width on screen but not zero-length in a `String`, so styling before
  the row's width arithmetic corrupts every truncation. `lay_out_row` now
  returns the truncated pieces and `RowLayout::render` styles them. The draft
  said "template change"; it was not.
- `render_live_row` and `failure_block` became `#[cfg(test)]` reference
  forms: production always calls the styled path with a possibly-disabled
  palette, so the plain output cannot drift from the coloured one by
  construction rather than by a parallel implementation.
- `console` is now a direct dependency of `blit-cli` (it was already in the
  tree via indicatif). `Color::TrueColor` exists in 0.16, so the palette is
  exact Dracula rather than an approximation.
- The failure block resolves its palette against **stdout** and the row
  against **stderr**, because that is where each actually writes.

Guards, each proven by revert:
- `styling_never_changes_the_visible_row` — strip the SGR back off and the
  bytes must equal the plain row, across every phase and widths 0..200.
  This is the load-bearing one: it catches any mis-split of the head, any
  lost text, and the layout hazard above. **Proven** by making `render`
  truncate the styled string (i.e. counting escapes as width): red at
  width 40, byte-identical restore.
- `the_header_and_reason_are_red_and_the_path_is_not` — **proven** by
  repointing the header at `Muted`: red.
- `colour_never_changes_the_block_text` — same strip-and-compare for the
  block, including the elided-count case, and asserts the styled form
  DIFFERS from the plain one so it cannot pass by colouring nothing.
- `deleting_and_failure_never_share_a_colour` — the one semantic invariant,
  asserted at both depths.
- Exact-byte assertions per capability tier; `NO_COLOR` (including empty),
  `TERM=dumb`, and a caller-denied gate all verified to disable.

Verified by hand: piped output contains no escape bytes at all.

## clp-2 landing notes

- Landed with post-review hardenings: DeleteBegin gated on `execute` (a
  dry run must not announce a deletion it will not perform;
  guard-tested), the perf-history failure print moved to the log facade
  (the last raw stderr writer on the local path), control bytes
  sanitized out of row text and `-v` lines (guard-tested), `biased;`
  dropped from the drain select (unbounded lane — fairness keeps the
  tick arm from starving), the log-redirect CLI half pinned via the
  `row_line_sink` seam, and the tick-arm repaint pinned under a paused
  clock (tokio `test-util` added to blit-cli dev-deps).
- Residue (d) — remote-route enumeration attachment — is DEFERRED to
  the remote render work by design; the remote monitor ignores the new
  phase signals via if-let, and `ProgressTotals`/daemon folds carry
  explicit no-op arms.
- Known gaps (recorded, not owed by clp): the `{wide_msg}` no-wrap
  mechanism itself has no automated guard (needs a fake `TermLike`);
  the renderer lays out for a fixed 80 columns (true width adaptation
  needs a terminal-size dependency decision); `LineRedirect` restoration
  is LIFO-only — a future concurrent-row design must revisit the seam;
  column counting is char-based, not display-width (CJK under-fills,
  never wraps).

## Open questions

- None. Flipped Active by the owner ("flip clp", D-2026-07-31-2).
  Execution order: clp-1/clp-2 run after pfc-2 lands (clp rebases on
  pfc-2's file-complete semantics), before pfc-3..6.
