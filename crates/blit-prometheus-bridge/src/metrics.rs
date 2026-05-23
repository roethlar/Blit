//! Render a daemon `GetState` snapshot as Prometheus
//! text-exposition format. Pure (no I/O) so it unit-tests
//! without a live daemon.
//!
//! Metric set (bridge slice 1):
//! - `blit_daemon_up{version}` gauge — 1 whenever a scrape
//!   produced a snapshot (the print-once CLI only emits on a
//!   successful query; a future HTTP-server slice will emit 0 on a
//!   failed scrape).
//! - `blit_daemon_uptime_seconds`, `blit_daemon_modules`,
//!   `blit_daemon_delegation_enabled`, `blit_active_transfers`,
//!   `blit_recent_transfers` gauges.
//!
//! **Operation counters are deliberately NOT emitted yet.** The
//! daemon always returns `counters: Some(..)` (see
//! `crates/blit-daemon/src/service/core.rs` + `proto/blit.proto`):
//! when `--metrics` is disabled the atomics never incremented, so the
//! fields are *present but zero* — indistinguishable on the wire from
//! a daemon that genuinely served zero operations. Publishing
//! `blit_push_operations_total 0` for a busy-but-metrics-off daemon
//! would be false telemetry once scraped. Until the wire grows a
//! `metrics_enabled` signal (or omits `Counters` when disabled), this
//! bridge exposes only the always-reliable gauges above (uptime /
//! module count / delegation flag / live active+recent counts all come
//! from fields independent of the `--metrics` flag).

use blit_core::generated::DaemonState;
use std::fmt::Write;

/// Format one scrape's worth of metrics. Always ends with a
/// trailing newline so concatenation stays line-oriented.
pub(crate) fn format_metrics(state: &DaemonState) -> String {
    let mut out = String::new();

    metric(
        &mut out,
        "blit_daemon_up",
        "Whether the daemon GetState scrape succeeded (1 = this snapshot is live).",
        "gauge",
        &format!("{{version=\"{}\"}}", escape_label(&state.version)),
        1,
    );
    metric(
        &mut out,
        "blit_daemon_uptime_seconds",
        "Seconds since the daemon started serving RPCs.",
        "gauge",
        "",
        state.uptime_seconds,
    );
    metric(
        &mut out,
        "blit_daemon_modules",
        "Number of modules the daemon exports.",
        "gauge",
        "",
        state.modules.len() as u64,
    );
    metric(
        &mut out,
        "blit_daemon_delegation_enabled",
        "1 if the daemon accepts inbound delegated pulls, else 0.",
        "gauge",
        "",
        u64::from(state.delegation_enabled),
    );
    metric(
        &mut out,
        "blit_active_transfers",
        "Transfers running on the daemon right now.",
        "gauge",
        "",
        state.active.len() as u64,
    );
    metric(
        &mut out,
        "blit_recent_transfers",
        "Completed transfers retained in the daemon's recent-runs ring.",
        "gauge",
        "",
        state.recent.len() as u64,
    );

    // NOTE: operation counters (push/pull/purge/errors) are NOT emitted
    // here — see the module docs. `state.counters` is always `Some`, so
    // we cannot tell a real zero from a metrics-disabled zero, and
    // publishing false zeros would corrupt a Prometheus counter series.

    out
}

/// Emit one metric family: `# HELP`, `# TYPE`, then the sample
/// line (`name<labels> value`). `labels` is either empty or a
/// pre-rendered `{k="v"}` block.
fn metric(out: &mut String, name: &str, help: &str, kind: &str, labels: &str, value: u64) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} {kind}");
    let _ = writeln!(out, "{name}{labels} {value}");
}

/// Escape a Prometheus label value per the exposition format:
/// backslash, double-quote, and newline are escaped.
fn escape_label(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::generated::{ActiveTransfer, Counters, ModuleInfo, TransferRecord};

    /// Mirrors the DEFAULT daemon's `GetState` shape: `counters` is
    /// `Some(..)` even though this daemon ran without `--metrics`, so
    /// the operation totals are present-but-zero. The bridge must NOT
    /// publish those zeros (see `omits_operation_counters_*`).
    fn sample_state() -> DaemonState {
        DaemonState {
            version: "0.1.0".to_string(),
            uptime_seconds: 3600,
            modules: vec![ModuleInfo::default(), ModuleInfo::default()],
            active: vec![ActiveTransfer::default()],
            recent: vec![TransferRecord::default(), TransferRecord::default()],
            // Default/metrics-disabled daemon: present-but-zero counters.
            counters: Some(Counters {
                push_operations_total: 0,
                pull_operations_total: 0,
                purge_operations_total: 0,
                active_transfers: 0,
                transfer_errors_total: 0,
            }),
            delegation_enabled: true,
        }
    }

    #[test]
    fn formats_gauges_from_reliable_fields() {
        let out = format_metrics(&sample_state());
        assert!(out.contains("blit_daemon_uptime_seconds 3600"), "{out}");
        assert!(out.contains("blit_daemon_modules 2"), "{out}");
        assert!(out.contains("blit_daemon_delegation_enabled 1"), "{out}");
        // active/recent come from the live tables, not the metrics
        // atomics, so they're reliable regardless of `--metrics`.
        assert!(out.contains("blit_active_transfers 1"), "{out}");
        assert!(out.contains("blit_recent_transfers 2"), "{out}");
    }

    /// Regression guard for the round-1 reopen: even given the real
    /// daemon shape (`counters: Some(present-but-zero)`), the bridge
    /// must omit the operation-counter families — a metrics-disabled
    /// daemon's zeros are not genuine and would be false telemetry.
    #[test]
    fn omits_operation_counters_to_avoid_false_zeros() {
        let out = format_metrics(&sample_state());
        assert!(!out.contains("blit_push_operations_total"), "{out}");
        assert!(!out.contains("blit_pull_operations_total"), "{out}");
        assert!(!out.contains("blit_purge_operations_total"), "{out}");
        assert!(!out.contains("blit_transfer_errors_total"), "{out}");
        // ...and no counter TYPE lines leak either.
        assert!(!out.contains("counter"), "{out}");
    }

    #[test]
    fn emits_help_and_type_lines_for_gauges() {
        let out = format_metrics(&sample_state());
        assert!(
            out.contains("# TYPE blit_daemon_uptime_seconds gauge"),
            "{out}"
        );
        assert!(out.contains("# HELP blit_active_transfers "), "{out}");
        // up carries the version label.
        assert!(out.contains("blit_daemon_up{version=\"0.1.0\"} 1"), "{out}");
    }

    #[test]
    fn delegation_disabled_is_zero() {
        let mut state = sample_state();
        state.delegation_enabled = false;
        let out = format_metrics(&state);
        assert!(out.contains("blit_daemon_delegation_enabled 0"), "{out}");
    }

    #[test]
    fn version_label_is_escaped() {
        let mut state = sample_state();
        state.version = "v\"weird\\".to_string();
        let out = format_metrics(&state);
        assert!(
            out.contains("blit_daemon_up{version=\"v\\\"weird\\\\\"} 1"),
            "{out}"
        );
    }
}
