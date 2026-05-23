//! Render a daemon `GetState` snapshot as Prometheus
//! text-exposition format. Pure (no I/O) so it unit-tests
//! without a live daemon.
//!
//! Metric set (bridge slice 1):
//! - `blit_daemon_up{version}` gauge — 1 whenever a scrape
//!   produced a snapshot (the print-once CLI only emits on a
//!   successful query; the future HTTP server will emit 0 on a
//!   failed scrape).
//! - `blit_daemon_uptime_seconds`, `blit_daemon_modules`,
//!   `blit_daemon_delegation_enabled`, `blit_active_transfers`,
//!   `blit_recent_transfers` gauges.
//! - `blit_{push,pull,purge}_operations_total`,
//!   `blit_transfer_errors_total` counters (from the daemon's
//!   `Counters` snapshot — zero when the daemon ran without
//!   `--metrics`, in which case the atomics never incremented).

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

    if let Some(c) = &state.counters {
        metric(
            &mut out,
            "blit_push_operations_total",
            "Cumulative push operations served.",
            "counter",
            "",
            c.push_operations_total,
        );
        metric(
            &mut out,
            "blit_pull_operations_total",
            "Cumulative pull operations served.",
            "counter",
            "",
            c.pull_operations_total,
        );
        metric(
            &mut out,
            "blit_purge_operations_total",
            "Cumulative purge operations served.",
            "counter",
            "",
            c.purge_operations_total,
        );
        metric(
            &mut out,
            "blit_transfer_errors_total",
            "Cumulative transfer errors.",
            "counter",
            "",
            c.transfer_errors_total,
        );
    }

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

    fn sample_state() -> DaemonState {
        DaemonState {
            version: "0.1.0".to_string(),
            uptime_seconds: 3600,
            modules: vec![ModuleInfo::default(), ModuleInfo::default()],
            active: vec![ActiveTransfer::default()],
            recent: vec![TransferRecord::default(), TransferRecord::default()],
            counters: Some(Counters {
                push_operations_total: 10,
                pull_operations_total: 20,
                purge_operations_total: 3,
                active_transfers: 1,
                transfer_errors_total: 2,
            }),
            delegation_enabled: true,
        }
    }

    #[test]
    fn formats_gauges_and_counters() {
        let out = format_metrics(&sample_state());
        // Gauges.
        assert!(out.contains("blit_daemon_uptime_seconds 3600"), "{out}");
        assert!(out.contains("blit_daemon_modules 2"), "{out}");
        assert!(out.contains("blit_daemon_delegation_enabled 1"), "{out}");
        assert!(out.contains("blit_active_transfers 1"), "{out}");
        assert!(out.contains("blit_recent_transfers 2"), "{out}");
        // Counters carry the cumulative totals.
        assert!(out.contains("blit_push_operations_total 10"), "{out}");
        assert!(out.contains("blit_pull_operations_total 20"), "{out}");
        assert!(out.contains("blit_purge_operations_total 3"), "{out}");
        assert!(out.contains("blit_transfer_errors_total 2"), "{out}");
    }

    #[test]
    fn emits_help_and_type_lines_with_correct_kinds() {
        let out = format_metrics(&sample_state());
        assert!(
            out.contains("# TYPE blit_daemon_uptime_seconds gauge"),
            "{out}"
        );
        assert!(
            out.contains("# TYPE blit_push_operations_total counter"),
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
    fn missing_counters_omits_counter_families_but_keeps_gauges() {
        let mut state = sample_state();
        state.counters = None;
        let out = format_metrics(&state);
        assert!(!out.contains("blit_push_operations_total"), "{out}");
        // Gauges still present.
        assert!(out.contains("blit_active_transfers 1"), "{out}");
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
