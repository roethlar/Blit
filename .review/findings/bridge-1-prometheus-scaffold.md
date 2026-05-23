# bridge-1-prometheus-scaffold: GetState → Prometheus text + print-once CLI

**Severity**: Feature (Milestone E — optional Prometheus bridge, step 1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `c8291f6`

## What

First slice of the optional Prometheus bridge named in `TUI_DESIGN.md`
§9 ("a separate bridge program can scrape `GetState`") and Milestone E
("optional Prometheus bridge as a separate binary scraping GetState").
Keeping it a **separate binary** is the whole point — the daemon never
speaks HTTP/Prometheus itself (§9 non-goal). This slice delivers the
foundation: scrape a daemon's `GetState` once and print the metrics to
stdout. That one-shot output is already usable by a node_exporter
`textfile` collector or a `curl`-free smoke test. A later slice layers a
long-running HTTP `/metrics` server + scrape loop on the same formatter.

## Approach

- New workspace crate `crates/blit-prometheus-bridge` (binary). Deps:
  `blit-app` (reuses `admin::jobs::query` — the same GetState client
  `blit jobs list` uses), `blit-core` (`RemoteEndpoint`, `DaemonState`),
  `clap`, `tokio`, `eyre`. No new gRPC plumbing; no changes to existing
  crates.
- `metrics::format_metrics(&DaemonState) -> String` — **pure**, so it
  unit-tests without a live daemon. Emits `# HELP` / `# TYPE` / sample
  lines in Prometheus text-exposition format:
  - Gauges: `blit_daemon_up{version}` (always 1 on a successful scrape),
    `blit_daemon_uptime_seconds`, `blit_daemon_modules`,
    `blit_daemon_delegation_enabled`, `blit_active_transfers`,
    `blit_recent_transfers`.
  - Counters (`_total`): `blit_push_operations_total`,
    `blit_pull_operations_total`, `blit_purge_operations_total`,
    `blit_transfer_errors_total` — from the daemon's `Counters` snapshot.
  - When `counters` is `None` (daemon ran without `--metrics`, so the
    atomics never incremented) the counter families are omitted; the
    gauges are always present (active/recent are independent of the
    metrics flag, per the proto comment).
  - Label values are escaped (`\`, `"`, `\n`) per the exposition format.
- `main`: `--remote` (+ `--recent-limit`, default 50) →
  `RemoteEndpoint::parse` → `jobs::query` → `format_metrics` → stdout.

## Scope decisions

- **One-shot, not yet an HTTP server.** Slice 1 prints once and exits —
  the smallest useful unit and the testable formatter foundation. The
  `/metrics` HTTP endpoint + periodic scrape (and `blit_daemon_up 0` on
  a failed scrape) are the next slice, built on this `format_metrics`.
- **Crate name** `blit-prometheus-bridge` follows the design's wording;
  open to a rename (`blit-metrics`?) if preferred.
- Per-transfer metrics (labels per `transfer_id`) are intentionally
  excluded — high cardinality; daemon-level aggregates only for now.

## Files changed

- `crates/blit-prometheus-bridge/Cargo.toml`, `src/main.rs`,
  `src/metrics.rs` (new crate).
- `Cargo.toml`: workspace member added.

## Tests

Workspace 28 test binaries (new crate adds 5 unit tests in `metrics`):
- `formats_gauges_and_counters` — gauge + counter values render.
- `emits_help_and_type_lines_with_correct_kinds` — HELP/TYPE present;
  gauge vs counter; `up` carries the version label.
- `delegation_disabled_is_zero`.
- `missing_counters_omits_counter_families_but_keeps_gauges`.
- `version_label_is_escaped`.

The live `jobs::query` path is integration (needs a daemon).

## Round 2 (commit `9411754`)

**Reopen finding:** the daemon **always** returns `counters: Some(..)`
(`crates/blit-daemon/src/service/core.rs` + `proto/blit.proto:739`) —
present-but-zero when `--metrics` is off, since the atomics never
incremented. That's indistinguishable on the wire from a genuine zero.
So the round-1 formatter's `if let Some(c) = &state.counters` always
fired, publishing `blit_push_operations_total 0` etc. for a busy daemon
running without `--metrics` — false telemetry once scraped into a
Prometheus counter series. The round-1 test only covered `counters =
None`, a state the real daemon never produces.

**Fix (reviewer option 3 — omit until the wire can distinguish):**
- `format_metrics` no longer emits the operation-counter families
  (`push`/`pull`/`purge`/`transfer_errors_total`). It exposes only the
  gauges that derive from `--metrics`-independent fields: `up`,
  `uptime_seconds`, `modules`, `delegation_enabled`, `active_transfers`
  (from the live `active[]` table), `recent_transfers` (from the live
  `recent[]` ring). Module docs explain why.
- Counters will return in a later slice once the wire grows a
  `metrics_enabled` signal (or omits `Counters` when disabled) so a real
  zero is distinguishable from a metrics-off zero — that's a
  daemon/proto change, out of scope for this client-only bridge slice.

**Tests:** 5 (unchanged count; reworked). `sample_state` now mirrors the
real default-daemon shape (`counters: Some(all-zero)`).
`omits_operation_counters_to_avoid_false_zeros` is the regression guard:
given that shape, the formatter emits no `*_total` family and no
`counter` TYPE line. Gauge tests unchanged.

## Reviewer comments

(empty — pending round-2 grade)
