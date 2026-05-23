# bridge-3-prometheus-readme: operator usage doc for the bridge

**Severity**: Docs (Milestone E — optional Prometheus bridge, step 3)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `9561fb2`

## What

The Prometheus bridge is functionally complete (bridge-1 formatter +
print-once CLI, bridge-2 HTTP exporter — both verified). bridge-3 adds
the operator-facing README the new crate was missing: how to build and
run it, the two modes, the flags, how to wire Prometheus, what metrics
it exposes, and why the operation counters are omitted.

## Contents

`crates/blit-prometheus-bridge/README.md`:
- **Build** (`cargo build --release -p blit-prometheus-bridge`).
- **Usage** — one-shot (print once) vs. `--listen` (HTTP exporter,
  pull-model), with the local-only / no-TLS trust note.
- **Flag table** — `--remote` (required), `--listen`, `--recent-limit`
  (default 50).
- **Prometheus scrape config** snippet, noting the bridge's 8s scrape
  deadline sits below Prometheus's 10s default `scrape_timeout` so a
  hung daemon yields `blit_daemon_up 0`, not a scrape error.
- **Exposed-metric reference table** — the six gauges
  (`blit_daemon_up{version}`, `_uptime_seconds`, `_modules`,
  `_delegation_enabled`, `blit_active_transfers`, `blit_recent_transfers`).
- **Why no `*_operations_total` counters yet** — the metrics-disabled
  present-but-zero `Counters` caveat (see
  [[feedback-getstate-counters-zero]]).

## Verification

All metric names and flags were read from the current source
(`src/main.rs` `Args`, `src/metrics.rs`) rather than written from
memory. Docs-only change: no code touched. `cargo fmt --check`, `clippy
-p blit-prometheus-bridge -D warnings`, and the crate's 12 tests still
pass (unaffected).

## Scope

Docs only. With the bridge now built (CLI + HTTP exporter, both
deadline-safe) and documented, the optional Prometheus bridge feature is
complete. Possible future slices (not started): labelled per-module
metrics, and the operation counters once the wire distinguishes
metrics-disabled from genuine-zero.

## Reviewer comments

(empty — pending grade)
