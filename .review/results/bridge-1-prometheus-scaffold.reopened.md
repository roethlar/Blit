Reviewed sha: `c8291f698ef8cff7a979cf129d870711fad5b2f1`

Verdict: reopened

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.
- Extra targeted check: `cargo test -p blit-prometheus-bridge` passed: 5 tests.

Findings:

1. `crates/blit-prometheus-bridge/src/metrics.rs:75` emits the `_total` counter families whenever `state.counters` is `Some`, with HELP text like "Cumulative push operations served." That is not safe for the actual daemon default: `crates/blit-daemon/src/service/core.rs:1097` always returns `counters: Some(counters)`, and the existing test at `crates/blit-daemon/src/service/core.rs:1303` documents that a metrics-disabled service still returns present-but-zero counters. `proto/blit.proto:739` also says `--metrics` off means the fields are zero, not absent. The bridge therefore reports `blit_push_operations_total 0`, `blit_pull_operations_total 0`, etc. for a default daemon even after transfers occurred, which is false telemetry once scraped by Prometheus or node_exporter's textfile collector. The formatter test at `crates/blit-prometheus-bridge/src/metrics.rs:201` only covers `counters = None`, a state the current daemon does not use for the disabled-metrics path. Fix options: make `GetState` omit `Counters` when metrics collection is disabled, expose a `metrics_enabled` signal and suppress/label the counters accordingly, or deliberately omit operation counters from this first bridge slice until the wire can distinguish disabled from real zero. Add a test against the actual disabled/default daemon `GetState` shape so the bridge cannot regress back to publishing false zeros.
