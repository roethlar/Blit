# m2f-10-f2-per-daemon-health: partial-degrade banner

**Severity**: Feature / correctness (multi-daemon F2 — connection health)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `365be9a`

## What

The m2f-5 fan-out has F2 watching every daemon via merged Subscribe
streams. But `apply_f2_event` set the **global** `transfers_status` to
`Degraded(msg)` whenever *any* one daemon's stream ended — blanking the
whole F2 pane's health banner even though the other daemons were still
live. The arm's own comment acknowledged "only this one daemon's
forwarder ended; the others are still sending," yet the banner said
otherwise. m2f-10 tracks per-daemon stream health so the banner shows
**partial vs. full** failure.

## Approach

- `AppState.f2_degraded_daemons: BTreeSet<String>` — watched daemon
  identities (`host_port_display`) whose Subscribe stream has errored.
- `f2_status_from_health(degraded, watched_total) -> ConnectionStatus`:
  - none degraded → `Live`.
  - subset → `Degraded("M/N daemon streams down: <names>")` — the pane
    keeps rendering the live daemons' transfers; the operator sees which
    dropped.
  - all (or a count exceeding a stale/zero total) → `Degraded("all N
    daemon stream(s) down")`.
- `apply_f2_event`:
  - `Error` records the daemon in the set and re-derives the banner.
  - `Connected` / `Event` call `mark_daemon_healthy`: if the daemon was
    degraded, its stream recovered → re-derive (back toward `Live`);
    otherwise only the pre-existing `Connecting → Live` transition
    fires. Crucially this still does **not** overwrite a `Degraded` set
    by a failed initial `GetState` (snapshot health) — that distinction
    predates m2f-10 (`drain_startup_events`) and is preserved.
- `refan_f2_setup` clears the set: a re-fan opens fresh streams for the
  new watch set, so prior degraded marks belong to the dropped streams.

The single-daemon case is unchanged in spirit: 1-of-1 down → "all 1
daemon stream(s) down", same blank-pane semantics operators see today.

## Scope decisions

- **Banner names daemons, not per-daemon error text.** The status is a
  single footer line; with the fan-out, concatenating each daemon's
  error string could overflow. The daemon identity is the actionable
  handle. (The full-failure / single-daemon path still effectively
  conveys the failure.)
- **`drain_startup_events` keeps its existing single-status behavior.**
  That path drains events buffered before the loop starts, while status
  is freshly `Connecting` / snapshot-`Degraded`. A buffered `Error`
  there is the initial-connection failure, for which the full error
  message is the right banner; the partial-health model is for the
  steady state where independent daemons drop over time. A startup
  error is reconciled on the next re-fan.

## Files changed

- `crates/blit-tui/src/main.rs`: `f2_degraded_daemons` field (+ all
  initializers); `f2_status_from_health`; `mark_daemon_healthy`;
  `apply_f2_event` Error/Connected/Event arms; `refan_f2_setup` clears
  the set; three unit tests.

## Tests

599 total (+3):
- `f2_status_from_health_partial_vs_full` — none→Live; subset→partial
  (count + names); all / stale-total→full.
- `apply_f2_event_partial_degrade_then_recover` — two watched daemons;
  one Error → partial ("1/2 ... <id>"); that daemon's Connected →
  back to `Live`, set cleared.
- `healthy_event_does_not_clobber_snapshot_degraded` — a healthy signal
  from a never-degraded daemon does not overwrite a snapshot-failure
  `Degraded`.

The merged-stream wiring is integration (live daemons).

## Reviewer comments

(empty — pending grade)
