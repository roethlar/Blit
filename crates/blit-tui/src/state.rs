//! In-memory model of the daemon's transfer state the TUI
//! renders. Hydrated by the initial `GetState` snapshot,
//! updated incrementally by `Subscribe` stream events.
//!
//! The state lives entirely in the TUI process and is
//! refreshed from the daemon — there's no source of truth
//! here, just a view-side cache that lets the renderer
//! avoid hammering `GetState` on every keystroke.

use blit_core::generated::{
    daemon_event, ActiveTransfer, DaemonEvent, DaemonState, TransferRecord,
};
use std::collections::HashMap;
use std::collections::VecDeque;

/// Maximum number of finished transfers retained client-
/// side. Matches the daemon's default recent ring depth
/// (50) so the TUI can render the same window without
/// re-querying.
pub const TUI_RECENT_CAP: usize = 50;

/// Single in-flight transfer row. Mirrors the wire shape of
/// `ActiveTransfer` but as an owned struct so the renderer
/// doesn't need lifetimes against the original snapshot.
#[derive(Debug, Clone)]
pub struct ActiveRow {
    pub transfer_id: String,
    pub kind: i32,
    pub peer: String,
    pub module: String,
    pub path: String,
    pub start_unix_ms: u64,
    pub bytes_completed: u64,
    pub bytes_total: u64,
    pub throughput_bps: u64,
}

impl From<ActiveTransfer> for ActiveRow {
    fn from(value: ActiveTransfer) -> Self {
        Self {
            transfer_id: value.transfer_id,
            kind: value.kind,
            peer: value.peer,
            module: value.module,
            path: value.path,
            start_unix_ms: value.start_unix_ms,
            bytes_completed: value.bytes_completed,
            bytes_total: value.bytes_total,
            // Snapshot doesn't carry throughput — only the
            // Subscribe stream's TransferProgress events do.
            // The first progress tick after subscribe fills
            // this in; until then we render the bps column
            // as a dash.
            throughput_bps: 0,
        }
    }
}

/// Single completed transfer row. Mirrors `TransferRecord`.
#[derive(Debug, Clone)]
pub struct RecentRow {
    pub transfer_id: String,
    pub kind: i32,
    pub peer: String,
    pub module: String,
    pub path: String,
    pub duration_ms: u64,
    pub bytes: u64,
    pub ok: bool,
    pub error_message: String,
}

impl From<TransferRecord> for RecentRow {
    fn from(value: TransferRecord) -> Self {
        Self {
            transfer_id: value.transfer_id,
            kind: value.kind,
            peer: value.peer,
            module: value.module,
            path: value.path,
            duration_ms: value.duration_ms,
            bytes: value.bytes,
            ok: value.ok,
            error_message: value.error_message,
        }
    }
}

/// The TUI's view-side state. Replicates the daemon's
/// `active[]` + `recent[]` sections of `GetState` and lets
/// the Subscribe stream mutate them incrementally.
#[derive(Debug, Default)]
pub struct TransfersState {
    /// Live transfers, keyed by transfer_id. HashMap so
    /// Progress events can update in place by id without
    /// scanning.
    active: HashMap<String, ActiveRow>,
    /// Recently completed, newest-first. Bounded by
    /// [`TUI_RECENT_CAP`] — oldest entries drop on
    /// overflow.
    recent: VecDeque<RecentRow>,
}

impl TransfersState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace `active[]` and `recent[]` from a fresh
    /// `GetState` snapshot. Called on initial connect and
    /// whenever the stream errors and falls back to a
    /// reconcile pass.
    pub fn replace_from_snapshot(&mut self, state: DaemonState) {
        self.active.clear();
        for a in state.active {
            self.active.insert(a.transfer_id.clone(), a.into());
        }
        self.recent.clear();
        // Wire ordering is oldest-first; the TUI renders
        // newest-first, so insert in reverse.
        for r in state.recent.into_iter().rev() {
            self.push_recent(r.into());
        }
    }

    /// Apply one Subscribe stream event. Returns true when
    /// the event mutated state (useful for triggering a
    /// redraw); false when the event was for an unknown id
    /// or a no-op variant.
    pub fn apply_event(&mut self, event: DaemonEvent) -> bool {
        match event.payload {
            Some(daemon_event::Payload::TransferStarted(s)) => {
                // Subscribe filter is empty (we watch every
                // transfer), so the initial buffered drain
                // can deliver Started events for transfers
                // already in the snapshot. Non-clobbering
                // insert (or_insert_with) preserves the
                // snapshot's bytes_completed / throughput
                // when both sources agree on a transfer.
                //
                // Returns true when the entry was newly
                // inserted (state actually changed); false
                // when the row was already present (no-op).
                let id = s.transfer_id.clone();
                let inserted = !self.active.contains_key(&id);
                self.active.entry(id).or_insert_with(|| ActiveRow {
                    transfer_id: s.transfer_id,
                    kind: s.kind,
                    peer: s.peer,
                    module: s.module,
                    path: s.path,
                    start_unix_ms: s.start_unix_ms,
                    bytes_completed: 0,
                    bytes_total: 0,
                    throughput_bps: 0,
                });
                inserted
            }
            Some(daemon_event::Payload::TransferProgress(p)) => {
                if let Some(row) = self.active.get_mut(&p.transfer_id) {
                    row.bytes_completed = p.bytes_completed;
                    row.bytes_total = p.bytes_total;
                    row.throughput_bps = p.throughput_bps;
                    true
                } else {
                    false
                }
            }
            Some(daemon_event::Payload::TransferComplete(c)) => {
                let removed = self.active.remove(&c.transfer_id);
                let kind = removed.as_ref().map(|r| r.kind).unwrap_or(0);
                let peer = removed.as_ref().map(|r| r.peer.clone()).unwrap_or_default();
                let module = removed
                    .as_ref()
                    .map(|r| r.module.clone())
                    .unwrap_or_default();
                let path = removed.as_ref().map(|r| r.path.clone()).unwrap_or_default();
                self.push_recent(RecentRow {
                    transfer_id: c.transfer_id,
                    kind,
                    peer,
                    module,
                    path,
                    duration_ms: c.duration_ms,
                    bytes: c.bytes,
                    ok: true,
                    error_message: String::new(),
                });
                true
            }
            Some(daemon_event::Payload::TransferError(e)) => {
                let removed = self.active.remove(&e.transfer_id);
                let kind = removed.as_ref().map(|r| r.kind).unwrap_or(0);
                let peer = removed.as_ref().map(|r| r.peer.clone()).unwrap_or_default();
                let module = removed
                    .as_ref()
                    .map(|r| r.module.clone())
                    .unwrap_or_default();
                let path = removed.as_ref().map(|r| r.path.clone()).unwrap_or_default();
                self.push_recent(RecentRow {
                    transfer_id: e.transfer_id,
                    kind,
                    peer,
                    module,
                    path,
                    duration_ms: 0,
                    bytes: 0,
                    ok: false,
                    error_message: e.message,
                });
                true
            }
            None => false,
        }
    }

    /// Active rows, newest-first.
    pub fn active_rows(&self) -> Vec<&ActiveRow> {
        let mut rows: Vec<&ActiveRow> = self.active.values().collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.start_unix_ms));
        rows
    }

    /// Recent rows, newest-first.
    pub fn recent_rows(&self) -> impl Iterator<Item = &RecentRow> {
        self.recent.iter()
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    pub fn recent_count(&self) -> usize {
        self.recent.len()
    }

    fn push_recent(&mut self, row: RecentRow) {
        self.recent.push_front(row);
        while self.recent.len() > TUI_RECENT_CAP {
            self.recent.pop_back();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::generated::{
        ActiveTransfer, DaemonEvent, TransferComplete, TransferError, TransferKind,
        TransferProgress, TransferRecord, TransferStarted,
    };

    fn make_active(id: &str, bytes: u64) -> ActiveTransfer {
        ActiveTransfer {
            transfer_id: id.to_string(),
            kind: TransferKind::DelegatedPull as i32,
            peer: "p".to_string(),
            module: "m".to_string(),
            path: "/".to_string(),
            start_unix_ms: 1,
            bytes_completed: bytes,
            bytes_total: 0,
        }
    }

    fn make_record(id: &str, ok: bool) -> TransferRecord {
        TransferRecord {
            transfer_id: id.to_string(),
            kind: TransferKind::DelegatedPull as i32,
            peer: "p".to_string(),
            module: "m".to_string(),
            path: "/".to_string(),
            start_unix_ms: 1,
            duration_ms: 100,
            bytes: 0,
            files: 0,
            ok,
            error_message: String::new(),
        }
    }

    #[test]
    fn replace_from_snapshot_populates_active_and_recent() {
        let mut state = TransfersState::new();
        let snapshot = DaemonState {
            version: String::new(),
            uptime_seconds: 0,
            modules: vec![],
            active: vec![make_active("t-1", 100), make_active("t-2", 200)],
            recent: vec![make_record("t-3", true), make_record("t-4", false)],
            counters: None,
            delegation_enabled: false,
        };
        state.replace_from_snapshot(snapshot);
        assert_eq!(state.active_count(), 2);
        assert_eq!(state.recent_count(), 2);
    }

    #[test]
    fn apply_event_progress_updates_row_in_place() {
        let mut state = TransfersState::new();
        state.replace_from_snapshot(DaemonState {
            active: vec![make_active("t-1", 0)],
            ..DaemonState::default()
        });
        let mutated = state.apply_event(DaemonEvent {
            payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
                transfer_id: "t-1".to_string(),
                bytes_completed: 4096,
                bytes_total: 0,
                files_completed: 0,
                files_total: 0,
                throughput_bps: 1_000_000,
            })),
        });
        assert!(mutated);
        let row = state.active_rows()[0];
        assert_eq!(row.bytes_completed, 4096);
        assert_eq!(row.throughput_bps, 1_000_000);
    }

    #[test]
    fn apply_event_progress_for_unknown_id_returns_false() {
        let mut state = TransfersState::new();
        let mutated = state.apply_event(DaemonEvent {
            payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
                transfer_id: "unknown".to_string(),
                bytes_completed: 0,
                bytes_total: 0,
                files_completed: 0,
                files_total: 0,
                throughput_bps: 0,
            })),
        });
        assert!(!mutated);
        assert_eq!(state.active_count(), 0);
    }

    #[test]
    fn apply_event_complete_moves_row_to_recent() {
        let mut state = TransfersState::new();
        state.replace_from_snapshot(DaemonState {
            active: vec![make_active("t-1", 0)],
            ..DaemonState::default()
        });
        let mutated = state.apply_event(DaemonEvent {
            payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                transfer_id: "t-1".to_string(),
                bytes: 1_000_000,
                files: 0,
                duration_ms: 5_000,
                tcp_fallback_used: false,
            })),
        });
        assert!(mutated);
        assert_eq!(state.active_count(), 0);
        assert_eq!(state.recent_count(), 1);
        let r = state.recent_rows().next().unwrap();
        assert_eq!(r.transfer_id, "t-1");
        assert!(r.ok);
        assert_eq!(r.bytes, 1_000_000);
    }

    #[test]
    fn apply_event_error_moves_row_to_recent_with_message() {
        let mut state = TransfersState::new();
        state.replace_from_snapshot(DaemonState {
            active: vec![make_active("t-1", 0)],
            ..DaemonState::default()
        });
        state.apply_event(DaemonEvent {
            payload: Some(daemon_event::Payload::TransferError(TransferError {
                transfer_id: "t-1".to_string(),
                message: "module not found".to_string(),
            })),
        });
        let r = state.recent_rows().next().unwrap();
        assert!(!r.ok);
        assert_eq!(r.error_message, "module not found");
    }

    #[test]
    fn apply_event_started_inserts_idempotently() {
        let mut state = TransfersState::new();
        let ev = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: "t-new".to_string(),
                kind: TransferKind::Pull as i32,
                peer: "p".to_string(),
                module: "m".to_string(),
                path: "/".to_string(),
                start_unix_ms: 1,
            })),
        };
        // First apply: row inserted, returns true.
        assert!(state.apply_event(ev.clone()));
        assert_eq!(state.active_count(), 1);
        // Second apply for the same id: returns false
        // (state didn't change). Counter stays at 1.
        assert!(!state.apply_event(ev));
        assert_eq!(state.active_count(), 1);
    }

    /// a1-2 round-3 regression: startup race between
    /// `subscribe()` (registers receiver early) and
    /// `GetState` (snapshot taken later). Buffered
    /// `TransferStarted` events for transfers already in the
    /// snapshot MUST NOT clobber the snapshot's bytes /
    /// throughput. Idempotent insert via `or_insert_with`
    /// preserves the existing row.
    #[test]
    fn apply_event_started_does_not_clobber_snapshot_progress() {
        let mut state = TransfersState::new();
        // Snapshot has the transfer with 500 KB of progress.
        state.replace_from_snapshot(DaemonState {
            active: vec![ActiveTransfer {
                transfer_id: "t-1".to_string(),
                kind: TransferKind::DelegatedPull as i32,
                peer: "peer-A".to_string(),
                module: "mod-X".to_string(),
                path: "sub/file".to_string(),
                start_unix_ms: 1,
                bytes_completed: 500_000,
                bytes_total: 0,
            }],
            ..DaemonState::default()
        });
        // Buffered Started event arrives — same id, but the
        // Started shape only carries metadata, no progress.
        let started = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: "t-1".to_string(),
                kind: TransferKind::DelegatedPull as i32,
                peer: "peer-A".to_string(),
                module: "mod-X".to_string(),
                path: "sub/file".to_string(),
                start_unix_ms: 1,
            })),
        };
        assert!(!state.apply_event(started));
        // Snapshot's bytes_completed preserved.
        let row = &state.active_rows()[0];
        assert_eq!(row.bytes_completed, 500_000);
    }

    /// a1-2 round-3 regression: a transfer that starts AND
    /// completes within the startup race window must arrive
    /// in the recent ring with full metadata. The buffered
    /// Started inserts the row; the buffered Complete moves
    /// it to recent with the Started's kind / peer /
    /// module / path intact.
    #[test]
    fn buffered_started_then_complete_preserves_metadata() {
        let mut state = TransfersState::new();
        // Empty initial snapshot (the transfer wasn't yet
        // visible when GetState fired).
        state.replace_from_snapshot(DaemonState::default());

        // Apply buffered Started first.
        state.apply_event(DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: "race-id".to_string(),
                kind: TransferKind::DelegatedPull as i32,
                peer: "race-peer".to_string(),
                module: "race-mod".to_string(),
                path: "race/path".to_string(),
                start_unix_ms: 1,
            })),
        });
        // Then buffered Complete.
        state.apply_event(DaemonEvent {
            payload: Some(daemon_event::Payload::TransferComplete(
                blit_core::generated::TransferComplete {
                    transfer_id: "race-id".to_string(),
                    bytes: 999,
                    files: 0,
                    duration_ms: 50,
                    tcp_fallback_used: false,
                },
            )),
        });

        assert_eq!(state.active_count(), 0);
        let r = state.recent_rows().next().unwrap();
        assert_eq!(r.transfer_id, "race-id");
        // Metadata copied from the Started, not blank.
        assert_eq!(r.peer, "race-peer");
        assert_eq!(r.module, "race-mod");
        assert_eq!(r.path, "race/path");
        assert_eq!(r.kind, TransferKind::DelegatedPull as i32);
        assert_eq!(r.bytes, 999);
        assert!(r.ok);
    }

    #[test]
    fn recent_ring_drops_oldest_on_overflow() {
        let mut state = TransfersState::new();
        for i in 0..(TUI_RECENT_CAP + 5) {
            state.apply_event(DaemonEvent {
                payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                    transfer_id: format!("t-{i}"),
                    bytes: 0,
                    files: 0,
                    duration_ms: 0,
                    tcp_fallback_used: false,
                })),
            });
        }
        assert_eq!(state.recent_count(), TUI_RECENT_CAP);
        // Newest-first: the most recent id should be on top.
        let newest = state.recent_rows().next().unwrap();
        assert_eq!(newest.transfer_id, format!("t-{}", TUI_RECENT_CAP + 4));
    }
}
