//! F1 Daemons state — purely a snapshot of the most recent
//! mDNS discovery result plus the operator's row selection.
//!
//! The discovery loop in `main.rs` calls
//! [`blit_app::scan::discover`] on a timer and pushes
//! `DiscoveryUpdate`s through an mpsc. This module owns the
//! reducer that maps those updates onto a stable [`DaemonsState`]
//! the renderer consumes.
//!
//! Render-side helpers live in `screens/f1.rs`. State-shape
//! decisions belong here.

use blit_core::mdns::MdnsDiscoveredService;
use std::time::Instant;

/// One row of the F1 daemons table. Eagerly materialised
/// from `MdnsDiscoveredService` so the renderer doesn't have
/// to re-parse TXT records on each frame.
#[derive(Debug, Clone)]
pub struct DaemonRow {
    pub instance_name: String,
    pub addresses: Vec<std::net::Ipv4Addr>,
    pub port: u16,
    /// `Some` once the daemon advertises the TXT key
    /// (post-§3.2 daemons); `None` for older daemons where
    /// the renderer should show `?` instead of `0`.
    pub module_count: Option<u32>,
    /// `Some(true|false)` for §3.2+ daemons; `None` shows `?`.
    pub delegation_enabled: Option<bool>,
    /// `Some` when the daemon advertised the `version` TXT
    /// key. Always populated by our own advertise path so
    /// `None` here means a pre-version or non-conforming
    /// daemon.
    pub version: Option<String>,
    /// Module names from the `modules` TXT key. May be
    /// truncated past ~180 bytes of TXT — use
    /// `module_count` for the authoritative total.
    pub modules: Vec<String>,
}

impl DaemonRow {
    fn from_service(service: &MdnsDiscoveredService) -> Self {
        Self {
            instance_name: service.instance_name.clone(),
            addresses: service.addresses.clone(),
            port: service.port,
            module_count: service.module_count(),
            delegation_enabled: service.delegation_enabled(),
            version: service.properties.get("version").cloned(),
            modules: service.modules(),
        }
    }
}

/// Connection / discovery status banner for the F1 footer.
/// Mirrors `f2::ConnectionStatus` shape but is F1-specific
/// because the failure modes differ — there's no Subscribe
/// stream here, just periodic mDNS scans.
#[derive(Debug, Clone)]
pub enum DiscoveryStatus {
    /// No scan has completed yet (first tick still in flight).
    Scanning,
    /// At least one scan returned; `last_scan_at` records
    /// when. Renderer can format "Xs ago" off that.
    Live { last_scan_at: Instant },
    /// Last scan failed; carry the message for diagnostics.
    /// Operator sees the previous row set unchanged plus an
    /// error banner.
    Degraded { message: String },
}

/// Pane state for F1. Holds the most recent discovery
/// result and the cursor position. Replacement is
/// whole-snapshot — there's no incremental update because
/// mDNS doesn't give us per-service events of the shape
/// we'd want (departures aren't reliably signalled).
#[derive(Debug, Clone)]
pub struct DaemonsState {
    rows: Vec<DaemonRow>,
    /// Selected row index, clamped to `rows.len()` on every
    /// replacement. Stays `0` when `rows` is empty.
    selected: usize,
    status: DiscoveryStatus,
}

impl Default for DaemonsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonsState {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            selected: 0,
            status: DiscoveryStatus::Scanning,
        }
    }

    pub fn rows(&self) -> &[DaemonRow] {
        &self.rows
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_row(&self) -> Option<&DaemonRow> {
        self.rows.get(self.selected)
    }

    pub fn status(&self) -> &DiscoveryStatus {
        &self.status
    }

    /// Replace the row set with a fresh discovery result.
    /// Preserves the cursor on the same `instance_name` if
    /// it's still present (operator's selection survives a
    /// rescan); otherwise clamps to the new length.
    pub fn replace_from_discovery(
        &mut self,
        services: &[MdnsDiscoveredService],
        scanned_at: Instant,
    ) {
        let prior_selected_name = self
            .rows
            .get(self.selected)
            .map(|r| r.instance_name.clone());
        self.rows = services.iter().map(DaemonRow::from_service).collect();
        // Stable display order by instance_name so a rescan
        // doesn't re-shuffle rows on the operator.
        self.rows
            .sort_by(|a, b| a.instance_name.cmp(&b.instance_name));
        self.selected = match prior_selected_name {
            Some(name) => self
                .rows
                .iter()
                .position(|r| r.instance_name == name)
                .unwrap_or(0),
            None => 0,
        };
        // Belt-and-braces clamp in case rows is now empty.
        if self.rows.is_empty() {
            self.selected = 0;
        }
        self.status = DiscoveryStatus::Live {
            last_scan_at: scanned_at,
        };
    }

    /// Surface a discovery failure WITHOUT clearing the
    /// previous row set — the operator still sees what
    /// worked before plus a banner saying the latest scan
    /// failed.
    pub fn note_discovery_error(&mut self, message: String) {
        self.status = DiscoveryStatus::Degraded { message };
    }

    /// Cursor down. No-op when the cursor is already at the
    /// last row OR the list is empty.
    pub fn select_next(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
        }
    }

    /// Cursor up. No-op at row 0.
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::mdns::MdnsDiscoveredService;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    fn svc(name: &str, props: &[(&str, &str)]) -> MdnsDiscoveredService {
        let mut properties = HashMap::new();
        for (k, v) in props {
            properties.insert((*k).to_string(), (*v).to_string());
        }
        MdnsDiscoveredService {
            fullname: format!("{name}._blit._tcp.local."),
            instance_name: name.to_string(),
            hostname: format!("{name}.local."),
            port: 9031,
            addresses: vec![Ipv4Addr::new(192, 168, 1, 10)],
            properties,
        }
    }

    #[test]
    fn replace_from_discovery_sorts_by_instance_name() {
        let mut state = DaemonsState::new();
        let services = vec![svc("zebra", &[]), svc("alpha", &[]), svc("mike", &[])];
        state.replace_from_discovery(&services, Instant::now());
        assert_eq!(state.rows().len(), 3);
        assert_eq!(state.rows()[0].instance_name, "alpha");
        assert_eq!(state.rows()[1].instance_name, "mike");
        assert_eq!(state.rows()[2].instance_name, "zebra");
    }

    #[test]
    fn replace_from_discovery_preserves_selection_on_rescan() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(
            &[svc("alpha", &[]), svc("bravo", &[]), svc("charlie", &[])],
            Instant::now(),
        );
        state.select_next(); // → bravo
        assert_eq!(state.selected_row().unwrap().instance_name, "bravo");
        // Bravo still present after rescan with a new daemon
        // mixed in — cursor stays on bravo.
        state.replace_from_discovery(
            &[
                svc("alpha", &[]),
                svc("aardvark", &[]),
                svc("bravo", &[]),
                svc("charlie", &[]),
            ],
            Instant::now(),
        );
        assert_eq!(state.selected_row().unwrap().instance_name, "bravo");
    }

    #[test]
    fn replace_from_discovery_clamps_selection_when_prior_row_disappears() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[]), svc("bravo", &[])], Instant::now());
        state.select_next(); // → bravo
                             // Rescan, bravo gone.
        state.replace_from_discovery(&[svc("alpha", &[])], Instant::now());
        assert_eq!(state.selected_row().unwrap().instance_name, "alpha");
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn select_next_prev_bounded() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[]), svc("bravo", &[])], Instant::now());
        state.select_prev(); // already at 0
        assert_eq!(state.selected_index(), 0);
        state.select_next(); // → 1
        assert_eq!(state.selected_index(), 1);
        state.select_next(); // already at last
        assert_eq!(state.selected_index(), 1);
    }

    #[test]
    fn from_service_pulls_txt_keys() {
        let s = svc(
            "mycroft",
            &[
                ("version", "0.1.0"),
                ("module_count", "3"),
                ("delegation_enabled", "1"),
                ("modules", "home,media,backups"),
            ],
        );
        let row = DaemonRow::from_service(&s);
        assert_eq!(row.version.as_deref(), Some("0.1.0"));
        assert_eq!(row.module_count, Some(3));
        assert_eq!(row.delegation_enabled, Some(true));
        assert_eq!(row.modules, vec!["home", "media", "backups"]);
    }

    #[test]
    fn from_service_handles_pre_3_2_daemon() {
        // No module_count / delegation_enabled / version TXT.
        let s = svc("legacy", &[]);
        let row = DaemonRow::from_service(&s);
        assert_eq!(row.version, None);
        assert_eq!(row.module_count, None);
        assert_eq!(row.delegation_enabled, None);
        assert!(row.modules.is_empty());
    }

    #[test]
    fn note_discovery_error_preserves_rows_and_sets_status() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[])], Instant::now());
        assert_eq!(state.rows().len(), 1);
        state.note_discovery_error("network unreachable".to_string());
        // Rows still there.
        assert_eq!(state.rows().len(), 1);
        match state.status() {
            DiscoveryStatus::Degraded { message } => {
                assert_eq!(message, "network unreachable");
            }
            _ => panic!("expected Degraded"),
        }
    }

    #[test]
    fn empty_discovery_clamps_selected_to_zero() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[])], Instant::now());
        state.replace_from_discovery(&[], Instant::now());
        assert_eq!(state.rows().len(), 0);
        assert_eq!(state.selected_index(), 0);
        assert!(state.selected_row().is_none());
    }
}
