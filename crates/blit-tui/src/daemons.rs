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

use blit_core::generated::DaemonState;
use blit_core::mdns::MdnsDiscoveredService;
use blit_core::remote::endpoint::RemoteEndpoint;
use std::collections::HashMap;
use std::time::Instant;

/// Endpoint kind discriminator. F1 treats the local host as
/// a first-class endpoint alongside discovered remote daemons
/// (TUI_DESIGN §10 owner-signoff: `blit-tui` must work without
/// any daemon on the LAN; `Local` appears in F1 so downstream
/// F2/F3 routing can address it symmetrically with remote
/// daemons).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointKind {
    /// Synthetic row for the host running the TUI itself.
    /// Always present, regardless of mDNS results.
    Local,
    /// A daemon discovered over mDNS.
    Remote,
}

/// Synthetic instance name reserved for the Local row.
/// Carved out so a remote daemon that advertised the same
/// name doesn't accidentally collapse the synthetic row.
pub const LOCAL_INSTANCE_NAME: &str = "local (this machine)";

/// One row of the F1 daemons table. Eagerly materialised
/// from `MdnsDiscoveredService` so the renderer doesn't have
/// to re-parse TXT records on each frame.
#[derive(Debug, Clone)]
pub struct DaemonRow {
    pub kind: EndpointKind,
    pub instance_name: String,
    pub addresses: Vec<std::net::Ipv4Addr>,
    pub port: u16,
    /// `Some` once the daemon advertises the TXT key
    /// (post-§3.2 daemons); `None` for older daemons where
    /// the renderer should show `?` instead of `0`. Always
    /// `None` for `EndpointKind::Local`.
    pub module_count: Option<u32>,
    /// `Some(true|false)` for §3.2+ daemons; `None` shows `?`.
    /// Always `None` for `EndpointKind::Local`.
    pub delegation_enabled: Option<bool>,
    /// `Some` when the daemon advertised the `version` TXT
    /// key. Always populated by our own advertise path so
    /// `None` here means a pre-version or non-conforming
    /// daemon. Always `None` for `EndpointKind::Local`.
    pub version: Option<String>,
    /// Module names from the `modules` TXT key. May be
    /// truncated past ~180 bytes of TXT — use
    /// `module_count` for the authoritative total. Always
    /// empty for `EndpointKind::Local` (a follow-up slice
    /// will populate this from `GetState` against the local
    /// daemon if one is running on the loopback port).
    pub modules: Vec<String>,
}

impl DaemonRow {
    fn from_service(service: &MdnsDiscoveredService) -> Self {
        Self {
            kind: EndpointKind::Remote,
            instance_name: service.instance_name.clone(),
            addresses: service.addresses.clone(),
            port: service.port,
            module_count: service.module_count(),
            delegation_enabled: service.delegation_enabled(),
            version: service.properties.get("version").cloned(),
            modules: service.modules(),
        }
    }

    /// Construct the synthetic Local row. Always pinned at
    /// index 0 of the table.
    fn local() -> Self {
        Self {
            kind: EndpointKind::Local,
            instance_name: LOCAL_INSTANCE_NAME.to_string(),
            addresses: Vec::new(),
            port: 0,
            module_count: None,
            delegation_enabled: None,
            version: None,
            modules: Vec::new(),
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self.kind, EndpointKind::Local)
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

/// Per-daemon `GetState` snapshot, keyed by `instance_name`
/// in [`DaemonsState::details`]. Drives the detail block's
/// rendering — mDNS gives us the row's identity, `GetState`
/// gives us the live counters / uptime / version.
#[derive(Debug, Clone)]
pub enum DaemonDetail {
    /// `GetState` fetch is in flight for this row.
    Pending,
    /// Last fetch returned `Ok`. `fetched_at` lets the
    /// renderer show "as of Xs ago" if we want.
    Loaded {
        state: Box<DaemonState>,
        fetched_at: Instant,
    },
    /// Last fetch failed. Carry the message so the operator
    /// can diagnose (e.g. "no local daemon detected" for
    /// Local row when nothing's listening on the loopback).
    Error { message: String },
}

/// Pane state for F1. Holds the most recent discovery
/// result, the cursor position, and a per-row `GetState`
/// cache. Replacement of `rows` is whole-snapshot — there's
/// no incremental update because mDNS doesn't give us
/// per-service events of the shape we'd want (departures
/// aren't reliably signalled).
#[derive(Debug, Clone)]
pub struct DaemonsState {
    rows: Vec<DaemonRow>,
    /// Selected row index, clamped to `rows.len()` on every
    /// replacement. Stays `0` when `rows` is empty.
    selected: usize,
    status: DiscoveryStatus,
    /// `GetState` snapshots keyed by `instance_name`. Lives
    /// outside `rows` so a rescan doesn't blow away
    /// previously-fetched detail data.
    details: HashMap<String, DaemonDetail>,
    /// Per-row monotonically increasing request id. Each
    /// kick of a GetState fetch bumps this and embeds the
    /// new value in the spawn's reply envelope. The reply
    /// arm only applies the result if the id still matches
    /// — so an older fetch returning after a newer one (or
    /// after the cursor moved away and back) is dropped on
    /// the floor instead of clobbering the fresh data.
    request_ids: HashMap<String, u64>,
}

impl Default for DaemonsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonsState {
    /// Initial state holds only the synthetic Local row.
    /// Discovery hasn't run yet — `status` reflects that
    /// with `Scanning`. Operator can immediately interact
    /// with the Local endpoint even before mDNS returns.
    pub fn new() -> Self {
        Self {
            rows: vec![DaemonRow::local()],
            selected: 0,
            status: DiscoveryStatus::Scanning,
            details: HashMap::new(),
            request_ids: HashMap::new(),
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

    /// Lookup the cached `GetState` detail for a daemon by
    /// `instance_name`. `None` when the renderer has never
    /// requested a fetch for that row.
    pub fn detail_for(&self, instance_name: &str) -> Option<&DaemonDetail> {
        self.details.get(instance_name)
    }

    /// Insert or replace the cached detail for a daemon.
    /// Bypasses request-id matching — use this when the
    /// caller is the synchronous originator of a state
    /// change (e.g. the kick path setting `Pending`,
    /// invalidations, tests). The async reply arm should
    /// use [`Self::apply_detail_update`] instead so a stale
    /// in-flight fetch can't clobber a newer one.
    pub fn set_detail(&mut self, instance_name: String, detail: DaemonDetail) {
        self.details.insert(instance_name, detail);
    }

    /// Bump the per-row request id, store `Pending`, and
    /// return the new id. Called by the event loop's
    /// `maybe_kick_detail_fetch` to mark the start of a
    /// fresh fetch — the spawned task embeds the returned
    /// id in its reply, and the apply arm only writes the
    /// result if the row's id still matches.
    pub fn begin_fetch(&mut self, instance_name: &str) -> u64 {
        let next = self.request_ids.get(instance_name).copied().unwrap_or(0) + 1;
        self.request_ids.insert(instance_name.to_string(), next);
        self.details
            .insert(instance_name.to_string(), DaemonDetail::Pending);
        next
    }

    /// Apply a result from a previously-spawned fetch.
    /// Returns `true` if the result was current (and got
    /// written into the cache); `false` if it was stale and
    /// dropped. Stale results occur when:
    ///
    /// - The operator pressed `r` and a second fetch was
    ///   kicked for the same row before the first returned.
    /// - The operator moved off the row and back, kicking
    ///   another fetch before the first returned.
    /// - Any other path that increments the row's request id
    ///   between begin_fetch and the reply.
    pub fn apply_detail_update(
        &mut self,
        instance_name: &str,
        request_id: u64,
        detail: DaemonDetail,
    ) -> bool {
        let latest = self.request_ids.get(instance_name).copied().unwrap_or(0);
        if request_id != latest {
            return false;
        }
        self.details.insert(instance_name.to_string(), detail);
        true
    }

    /// Drop the cached detail (and bump the row's request
    /// id) so the next kick fires fresh. Called by the `r`
    /// keystroke. Bumping the id ensures any in-flight
    /// reply from before the invalidation is dropped by
    /// [`Self::apply_detail_update`] when it arrives.
    pub fn invalidate_detail(&mut self, instance_name: &str) {
        self.details.remove(instance_name);
        let next = self.request_ids.get(instance_name).copied().unwrap_or(0) + 1;
        self.request_ids.insert(instance_name.to_string(), next);
    }

    /// Build the [`RemoteEndpoint`] to fetch this row's
    /// `GetState` from. Returns `None` for a remote row that
    /// didn't advertise any address (defensive — discovery
    /// shouldn't surface such rows in practice).
    ///
    /// Local rows resolve to `127.0.0.1:9031` — the operator
    /// can opt into a different loopback port via future
    /// config but the default port matches the daemon's
    /// canonical bind.
    pub fn endpoint_for_row(row: &DaemonRow) -> Option<RemoteEndpoint> {
        if row.is_local() {
            // Default daemon port; if no local daemon is
            // running the GetState fetch will fail and the
            // detail block surfaces "no local daemon".
            return RemoteEndpoint::parse("127.0.0.1:9031").ok();
        }
        let addr = row.addresses.first()?;
        RemoteEndpoint::parse(&format!("{}:{}", addr, row.port)).ok()
    }

    /// Replace the row set with a fresh discovery result.
    /// Preserves the cursor on the same `instance_name` if
    /// it's still present; otherwise falls back to
    /// `min(prior_index, rows.len()-1)` so the operator
    /// doesn't get teleported back to row 0 every time a
    /// daemon in the middle of the list goes away (mDNS
    /// rescans every 5s; transient drops are common).
    ///
    /// The synthetic Local row is always re-injected at
    /// index 0 so it survives across rescans.
    pub fn replace_from_discovery(
        &mut self,
        services: &[MdnsDiscoveredService],
        scanned_at: Instant,
    ) {
        let prior_index = self.selected;
        let prior_selected_name = self
            .rows
            .get(self.selected)
            .map(|r| r.instance_name.clone());

        let mut discovered: Vec<DaemonRow> = services.iter().map(DaemonRow::from_service).collect();
        // Stable display order by instance_name so a rescan
        // doesn't re-shuffle the remote rows on the operator.
        discovered.sort_by(|a, b| a.instance_name.cmp(&b.instance_name));

        // Local always anchors index 0.
        let mut rows = Vec::with_capacity(discovered.len() + 1);
        rows.push(DaemonRow::local());
        rows.extend(discovered);
        self.rows = rows;

        self.selected = match prior_selected_name {
            // Same name still here → stay on it.
            Some(name) => match self.rows.iter().position(|r| r.instance_name == name) {
                Some(idx) => idx,
                // Name gone — keep the operator near where
                // they were. Saturating to the last valid
                // row means a deletion in the middle moves
                // the cursor up by one rather than back to
                // the top.
                None => prior_index.min(self.rows.len().saturating_sub(1)),
            },
            None => 0,
        };
        // Belt-and-braces clamp (rows always contains Local
        // so this should never trigger, but cheap to keep).
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
    fn new_state_has_local_row() {
        let state = DaemonsState::new();
        // Local row is always present, even before discovery.
        assert_eq!(state.rows().len(), 1);
        let local = &state.rows()[0];
        assert_eq!(local.kind, EndpointKind::Local);
        assert!(local.is_local());
        assert_eq!(local.instance_name, LOCAL_INSTANCE_NAME);
    }

    #[test]
    fn replace_from_discovery_keeps_local_at_index_zero() {
        let mut state = DaemonsState::new();
        let services = vec![svc("zebra", &[]), svc("alpha", &[]), svc("mike", &[])];
        state.replace_from_discovery(&services, Instant::now());
        // Local + 3 remotes.
        assert_eq!(state.rows().len(), 4);
        assert!(state.rows()[0].is_local());
        // Remotes sorted alphabetically after Local.
        assert_eq!(state.rows()[1].instance_name, "alpha");
        assert_eq!(state.rows()[2].instance_name, "mike");
        assert_eq!(state.rows()[3].instance_name, "zebra");
    }

    #[test]
    fn replace_from_discovery_preserves_local_selection_across_rescan() {
        let mut state = DaemonsState::new();
        // Cursor starts on Local (index 0).
        assert!(state.selected_row().unwrap().is_local());
        // Rescan with some daemons — Local row stays selected.
        state.replace_from_discovery(&[svc("alpha", &[])], Instant::now());
        assert!(state.selected_row().unwrap().is_local());
    }

    #[test]
    fn replace_from_discovery_preserves_selection_on_rescan() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(
            &[svc("alpha", &[]), svc("bravo", &[]), svc("charlie", &[])],
            Instant::now(),
        );
        // After replace: Local @ 0, alpha @ 1, bravo @ 2, charlie @ 3.
        state.select_next(); // → alpha
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

    /// a1-3 round 2: when the selected name disappears from
    /// the new snapshot, the cursor should fall back to
    /// `min(prior_index, rows.len()-1)` rather than jumping
    /// to row 0. This keeps the operator near where they
    /// were in the list.
    #[test]
    fn replace_from_discovery_keeps_index_near_prior_when_name_lost() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(
            &[
                svc("alpha", &[]),
                svc("bravo", &[]),
                svc("charlie", &[]),
                svc("delta", &[]),
            ],
            Instant::now(),
        );
        // After replace: Local, alpha, bravo, charlie, delta (indices 0..4).
        // Move cursor to "charlie" (index 3).
        state.select_next(); // alpha
        state.select_next(); // bravo
        state.select_next(); // charlie
        assert_eq!(state.selected_row().unwrap().instance_name, "charlie");
        // Rescan, charlie gone — alpha + bravo + delta remain.
        // New row indices: Local @ 0, alpha @ 1, bravo @ 2, delta @ 3.
        // Prior index was 3; new len 4 → min(3, 3) = 3, which
        // is delta. NOT row 0.
        state.replace_from_discovery(
            &[svc("alpha", &[]), svc("bravo", &[]), svc("delta", &[])],
            Instant::now(),
        );
        assert_eq!(state.selected_index(), 3);
        assert_eq!(state.selected_row().unwrap().instance_name, "delta");
    }

    /// When the selected row was the last in the list and it
    /// disappears, the cursor clamps to the new last row —
    /// not row 0.
    #[test]
    fn replace_from_discovery_clamps_to_last_row_when_tail_disappears() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[]), svc("bravo", &[])], Instant::now());
        // Move to last (bravo @ index 2).
        state.select_next();
        state.select_next();
        assert_eq!(state.selected_row().unwrap().instance_name, "bravo");
        // bravo gone — rows become Local, alpha. prior_index=2,
        // new len=2, clamp to index 1 (alpha).
        state.replace_from_discovery(&[svc("alpha", &[])], Instant::now());
        assert_eq!(state.selected_index(), 1);
        assert_eq!(state.selected_row().unwrap().instance_name, "alpha");
    }

    #[test]
    fn select_next_prev_bounded() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[]), svc("bravo", &[])], Instant::now());
        // Indices: Local @ 0, alpha @ 1, bravo @ 2.
        state.select_prev(); // already at 0
        assert_eq!(state.selected_index(), 0);
        state.select_next(); // → 1 (alpha)
        state.select_next(); // → 2 (bravo)
        assert_eq!(state.selected_index(), 2);
        state.select_next(); // already at last
        assert_eq!(state.selected_index(), 2);
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
        // Local + alpha.
        assert_eq!(state.rows().len(), 2);
        state.note_discovery_error("network unreachable".to_string());
        // Rows still there.
        assert_eq!(state.rows().len(), 2);
        match state.status() {
            DiscoveryStatus::Degraded { message } => {
                assert_eq!(message, "network unreachable");
            }
            _ => panic!("expected Degraded"),
        }
    }

    #[test]
    fn empty_discovery_still_has_local_row() {
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[])], Instant::now());
        state.replace_from_discovery(&[], Instant::now());
        // Local survives an empty rescan.
        assert_eq!(state.rows().len(), 1);
        assert!(state.rows()[0].is_local());
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn from_service_tags_row_as_remote() {
        let s = svc("mycroft", &[("version", "0.1.0")]);
        let row = DaemonRow::from_service(&s);
        assert_eq!(row.kind, EndpointKind::Remote);
        assert!(!row.is_local());
    }

    #[test]
    fn endpoint_for_row_returns_loopback_for_local() {
        let local = DaemonRow::local();
        let endpoint = DaemonsState::endpoint_for_row(&local).unwrap();
        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 9031);
    }

    #[test]
    fn endpoint_for_row_uses_first_advertised_address() {
        let s = svc("mycroft", &[]);
        let row = DaemonRow::from_service(&s);
        let endpoint = DaemonsState::endpoint_for_row(&row).unwrap();
        assert_eq!(endpoint.host, "192.168.1.10");
        assert_eq!(endpoint.port, 9031);
    }

    #[test]
    fn endpoint_for_row_returns_none_when_remote_has_no_address() {
        let mut row = DaemonRow::from_service(&svc("addressless", &[]));
        row.addresses.clear();
        assert!(DaemonsState::endpoint_for_row(&row).is_none());
    }

    #[test]
    fn detail_for_returns_set_value() {
        let mut state = DaemonsState::new();
        state.set_detail(
            "mycroft".to_string(),
            DaemonDetail::Error {
                message: "boom".to_string(),
            },
        );
        match state.detail_for("mycroft") {
            Some(DaemonDetail::Error { message }) => assert_eq!(message, "boom"),
            other => panic!("expected Error, got {other:?}"),
        }
        // Lookup for a name we never set returns None.
        assert!(state.detail_for("unknown").is_none());
    }

    #[test]
    fn set_detail_replaces_prior_value_for_same_name() {
        let mut state = DaemonsState::new();
        state.set_detail("mycroft".to_string(), DaemonDetail::Pending);
        state.set_detail(
            "mycroft".to_string(),
            DaemonDetail::Error {
                message: "later failure".to_string(),
            },
        );
        match state.detail_for("mycroft") {
            Some(DaemonDetail::Error { message }) => assert_eq!(message, "later failure"),
            other => panic!("expected later Error, got {other:?}"),
        }
    }

    /// `begin_fetch` increments the row's request_id and
    /// stores Pending. Subsequent calls bump the id.
    #[test]
    fn begin_fetch_increments_request_id_per_row() {
        let mut state = DaemonsState::new();
        let id1 = state.begin_fetch("mycroft");
        assert_eq!(id1, 1);
        assert!(matches!(
            state.detail_for("mycroft"),
            Some(DaemonDetail::Pending)
        ));
        let id2 = state.begin_fetch("mycroft");
        assert_eq!(id2, 2);
        // Independent per row.
        let other = state.begin_fetch("skippy");
        assert_eq!(other, 1);
    }

    /// `apply_detail_update` applies a result tagged with
    /// the current request_id and returns true.
    #[test]
    fn apply_detail_update_writes_current_generation() {
        let mut state = DaemonsState::new();
        let id = state.begin_fetch("mycroft");
        let applied = state.apply_detail_update(
            "mycroft",
            id,
            DaemonDetail::Error {
                message: "boom".to_string(),
            },
        );
        assert!(applied);
        match state.detail_for("mycroft") {
            Some(DaemonDetail::Error { message }) => assert_eq!(message, "boom"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    /// `apply_detail_update` drops a result tagged with a
    /// stale (older) request_id and returns false. The
    /// existing Pending/Loaded state is preserved.
    #[test]
    fn apply_detail_update_drops_stale_generation() {
        let mut state = DaemonsState::new();
        let id1 = state.begin_fetch("mycroft");
        // Caller kicks again before the first reply arrives.
        let _id2 = state.begin_fetch("mycroft");
        // Stale reply from fetch #1 arrives:
        let applied = state.apply_detail_update(
            "mycroft",
            id1,
            DaemonDetail::Error {
                message: "from stale fetch #1".to_string(),
            },
        );
        assert!(!applied);
        // Detail should still be Pending (set by fetch #2),
        // not the stale Error.
        assert!(matches!(
            state.detail_for("mycroft"),
            Some(DaemonDetail::Pending)
        ));
    }

    /// `invalidate_detail` removes the cache entry and
    /// bumps the request_id so an in-flight reply from
    /// before the invalidation is dropped.
    #[test]
    fn invalidate_detail_drops_in_flight_reply() {
        let mut state = DaemonsState::new();
        let id = state.begin_fetch("mycroft");
        state.invalidate_detail("mycroft");
        // No cached entry.
        assert!(state.detail_for("mycroft").is_none());
        // Reply from the now-invalidated fetch arrives:
        let applied = state.apply_detail_update(
            "mycroft",
            id,
            DaemonDetail::Loaded {
                state: Box::new(DaemonState::default()),
                fetched_at: Instant::now(),
            },
        );
        assert!(!applied);
        assert!(state.detail_for("mycroft").is_none());
    }

    #[test]
    fn details_survive_discovery_rescan() {
        // Once we've cached a detail, a rescan of mDNS
        // (which replaces `rows`) must NOT blow away the
        // cached entries — flicking the cursor onto a row
        // that we just fetched shouldn't trigger an extra
        // round trip.
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&[svc("alpha", &[])], Instant::now());
        state.set_detail("alpha".to_string(), DaemonDetail::Pending);
        state.replace_from_discovery(&[svc("alpha", &[]), svc("bravo", &[])], Instant::now());
        assert!(matches!(
            state.detail_for("alpha"),
            Some(DaemonDetail::Pending)
        ));
    }
}
