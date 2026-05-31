# blit-prometheus-bridge

A standalone Prometheus exporter for a [blit](../../README.md) daemon.

The daemon itself never speaks HTTP or Prometheus (that's an explicit
non-goal — see `docs/plan/TUI_DESIGN.md` §9). This bridge is a separate
binary that scrapes the daemon's gRPC `GetState` and translates the
snapshot into the Prometheus text-exposition format.

## Build

```sh
cargo build --release -p blit-prometheus-bridge
# → target/release/blit-prometheus-bridge
```

## Usage

Two modes, selected by whether `--listen` is given.

### One-shot (print once, exit)

Scrape once and print metrics to stdout — handy for a smoke test or a
node_exporter `textfile` collector run from cron:

```sh
blit-prometheus-bridge --remote nas:9031
```

### Long-running HTTP exporter

Serve `GET /metrics`, scraping the daemon fresh on every request (pull
model — no cached staleness):

```sh
blit-prometheus-bridge --remote nas:9031 --listen 127.0.0.1:9119
```

Bind to `127.0.0.1` for a local-only exporter. There is no TLS or auth
(same operator-network trust model as the rest of blit); only expose it
on a trusted interface.

### Flags

| Flag             | Default | Meaning                                                        |
|------------------|---------|----------------------------------------------------------------|
| `--remote`       | (req'd) | Daemon to scrape — `host` or `host:port`.                      |
| `--listen`       | (none)  | `host:port` to serve `GET /metrics` on. Omit for one-shot.     |
| `--recent-limit` | `50`    | recent-runs ring depth to request; bounds `blit_recent_transfers`. |

## Prometheus scrape config

```yaml
scrape_configs:
  - job_name: blit
    static_configs:
      - targets: ["127.0.0.1:9119"]
```

The bridge's own scrape deadline (8s) sits below Prometheus's default
`scrape_timeout` (10s), so a hung daemon yields `blit_daemon_up 0`
rather than a Prometheus scrape error.

## Exposed metrics

| Metric                            | Type  | Meaning                                                  |
|-----------------------------------|-------|----------------------------------------------------------|
| `blit_daemon_up{version}`         | gauge | `1` when the scrape succeeded (label = daemon version); `0` (no label) when the scrape failed or timed out. |
| `blit_daemon_uptime_seconds`      | gauge | Seconds since the daemon started serving RPCs.           |
| `blit_daemon_modules`             | gauge | Number of modules the daemon exports.                    |
| `blit_daemon_delegation_enabled`  | gauge | `1` if the daemon accepts inbound delegated pulls, else `0`. |
| `blit_active_transfers`           | gauge | Transfers running on the daemon right now.               |
| `blit_recent_transfers`           | gauge | Completed transfers retained in the daemon's recent-runs ring. |

### Why no `*_operations_total` counters (yet)

The daemon's `GetState` always returns a `Counters` block, but when the
daemon runs **without `--metrics`** the counter atomics never
incremented, so the fields are *present but zero* — indistinguishable on
the wire from a daemon that genuinely served zero operations. Publishing
`blit_push_operations_total 0` for a busy-but-metrics-off daemon would be
false telemetry, so the bridge omits the operation counters until the
wire can signal whether metrics collection was enabled. The gauges above
all derive from fields that are reliable regardless of the `--metrics`
flag (`active`/`recent` come from the daemon's live tables, not the
metric atomics).
