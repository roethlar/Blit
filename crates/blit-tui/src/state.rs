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
use std::time::Instant;

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
    /// m2f-1: the daemon whose Subscribe stream reported this
    /// transfer (distinct from `peer`, the transfer's other
    /// endpoint). Single-valued today; once F2 fans out across
    /// every discovered daemon (m2f-2) this is what tells the
    /// operator which daemon a row belongs to.
    pub source_daemon: String,
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
            // Set by the caller from the snapshot's source daemon
            // (`From` can't see it — `ActiveTransfer` has no daemon).
            source_daemon: String::new(),
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
    /// m2f-1: source daemon (see [`ActiveRow::source_daemon`]).
    pub source_daemon: String,
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
            // Set by the caller from the snapshot's source daemon.
            source_daemon: String::new(),
        }
    }
}

/// The TUI's view-side state. Replicates the daemon's
/// `active[]` + `recent[]` sections of `GetState` and lets
/// the Subscribe stream mutate them incrementally.
#[derive(Debug, Default)]
pub struct TransfersState {
    /// Live transfers, keyed by the composite
    /// `row_key(source_daemon, transfer_id)` (m2f-2 — `transfer_id`
    /// isn't unique across daemons). HashMap so Progress events can
    /// update in place by key without scanning.
    active: HashMap<String, ActiveRow>,
    /// Recently completed, newest-first. Bounded by
    /// [`TUI_RECENT_CAP`] — oldest entries drop on
    /// overflow.
    recent: VecDeque<RecentRow>,
    /// d-13: monotonic timestamp of the last event that
    /// mutated this view — Subscribe stream event OR
    /// GetState snapshot reconcile. F2's footer renders
    /// "last event Xs ago" against this, and the live
    /// tick gate uses it to decide whether the F2 footer
    /// needs refreshing.
    last_event_at: Option<Instant>,
    /// d-21 R2: cursor anchored on a transfer_id rather
    /// than a display index. An index-based cursor would
    /// silently retarget after row removal — same index,
    /// different transfer underneath. The id-based cursor
    /// "falls off" naturally when the underlying transfer
    /// terminates (id no longer present in `active`) and
    /// doesn't come back when an unrelated transfer
    /// starts later.
    ///
    /// m2f-2: holds the composite `row_key(daemon, transfer_id)`, not
    /// the bare id — `transfer_id` isn't unique across daemons once
    /// F2 fans out, so the cursor anchors on the (daemon, id) pair.
    selected_active_key: Option<String>,
}

impl TransfersState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Timestamp of the most recent mutating event /
    /// snapshot. `None` until first wire activity.
    pub fn last_event_at(&self) -> Option<Instant> {
        self.last_event_at
    }

    /// m2f-3/m2f-5: hydrate from ONE daemon's `GetState` snapshot
    /// WITHOUT disturbing other daemons' rows — F2 fans out across
    /// every watched daemon (m2f-5), each hydrating independently here
    /// then mutating incrementally via `apply_event`. (Replaced the
    /// old `replace_from_snapshot`, which cleared the whole view — no
    /// good once more than one daemon is watched.)
    ///
    /// d-13: records `fetched_at` as `last_event_at` so F2's footer
    /// can show "last event Xs ago" against the snapshot too.
    ///
    /// Replaces only `source_daemon`'s rows: drops its existing
    /// active + recent entries, then inserts the snapshot's. Rows from
    /// every other daemon are untouched. Recent rows are grouped by
    /// this merge (the wire types carry no completion timestamp, so a
    /// precise cross-daemon time interleave isn't possible); the
    /// global ring stays bounded by [`TUI_RECENT_CAP`].
    pub fn merge_snapshot(&mut self, source_daemon: &str, state: DaemonState, fetched_at: Instant) {
        self.active.retain(|_, r| r.source_daemon != source_daemon);
        self.recent.retain(|r| r.source_daemon != source_daemon);
        for a in state.active {
            let mut row: ActiveRow = a.into();
            row.source_daemon = source_daemon.to_string();
            let key = row_key(source_daemon, &row.transfer_id);
            self.active.insert(key, row);
        }
        for r in state.recent.into_iter().rev() {
            let mut row: RecentRow = r.into();
            row.source_daemon = source_daemon.to_string();
            self.push_recent(row);
        }
        self.last_event_at = Some(fetched_at);
    }

    /// m2f-9 R3: reconcile the active table with the current watched
    /// daemon set after a re-fan. Drops active rows whose `source_daemon`
    /// is no longer watched — a daemon that left mDNS discovery can't
    /// send a Complete/Error event, so its in-flight rows would otherwise
    /// linger in the active table forever. **Recent rows are kept**: a
    /// finished transfer is history regardless of whether its daemon is
    /// still on the network. Clears the active cursor if the row it was
    /// anchored to was just removed (same fall-off contract as a
    /// terminated transfer).
    pub fn retain_active_daemons(&mut self, watched: &std::collections::BTreeSet<String>) {
        self.active
            .retain(|_, r| watched.contains(&r.source_daemon));
        if let Some(key) = self.selected_active_key.as_ref() {
            if !self.active.contains_key(key) {
                self.selected_active_key = None;
            }
        }
    }

    /// Apply one Subscribe stream event. Returns true when
    /// the event mutated state (useful for triggering a
    /// redraw); false when the event was for an unknown id
    /// or a no-op variant.
    ///
    /// Terminal-id dedup: once a transfer_id lands in the
    /// recent ring, subsequent events for that id are
    /// ignored. Closes the a1-2 round-3 startup race where
    /// the snapshot's `recent[]` already contained a
    /// transfer that ALSO had buffered Started+Complete in
    /// the Subscribe stream — without dedup the buffered
    /// Started would re-insert it as active and the
    /// buffered Complete would push a duplicate recent row.
    pub fn apply_event(&mut self, source_daemon: &str, event: DaemonEvent, now: Instant) -> bool {
        let mutated = self.apply_event_inner(source_daemon, event);
        if mutated {
            self.last_event_at = Some(now);
        }
        mutated
    }

    fn apply_event_inner(&mut self, source_daemon: &str, event: DaemonEvent) -> bool {
        // Look up the event's transfer_id and short-circuit
        // if the id is already terminal. We check this
        // BEFORE the variant match because every
        // transfer-scoped variant carries `transfer_id`.
        let event_id = match event.payload.as_ref() {
            Some(daemon_event::Payload::TransferStarted(s)) => Some(s.transfer_id.as_str()),
            Some(daemon_event::Payload::TransferProgress(p)) => Some(p.transfer_id.as_str()),
            Some(daemon_event::Payload::TransferComplete(c)) => Some(c.transfer_id.as_str()),
            Some(daemon_event::Payload::TransferError(e)) => Some(e.transfer_id.as_str()),
            None => None,
        };
        if let Some(id) = event_id {
            // m2f-2: dedup is per-(daemon, id) — a recent transfer on
            // daemon A must not suppress a same-id transfer on daemon
            // B (`transfer_id` is `t<ms>-<n>`, unique only within a
            // daemon, so cross-daemon collisions are possible).
            if self
                .recent
                .iter()
                .any(|r| r.transfer_id == id && r.source_daemon == source_daemon)
            {
                // Id is terminal — ignore further events
                // for it. Returning false signals no state
                // change (caller can avoid a redraw).
                return false;
            }
        }
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
                let key = row_key(source_daemon, &s.transfer_id);
                let inserted = !self.active.contains_key(&key);
                self.active.entry(key).or_insert_with(|| ActiveRow {
                    transfer_id: s.transfer_id,
                    kind: s.kind,
                    peer: s.peer,
                    module: s.module,
                    path: s.path,
                    start_unix_ms: s.start_unix_ms,
                    bytes_completed: 0,
                    bytes_total: 0,
                    throughput_bps: 0,
                    source_daemon: source_daemon.to_string(),
                });
                inserted
            }
            Some(daemon_event::Payload::TransferProgress(p)) => {
                if let Some(row) = self.active.get_mut(&row_key(source_daemon, &p.transfer_id)) {
                    row.bytes_completed = p.bytes_completed;
                    row.bytes_total = p.bytes_total;
                    row.throughput_bps = p.throughput_bps;
                    true
                } else {
                    false
                }
            }
            Some(daemon_event::Payload::TransferComplete(c)) => {
                let removed = self.active.remove(&row_key(source_daemon, &c.transfer_id));
                let kind = removed.as_ref().map(|r| r.kind).unwrap_or(0);
                let peer = removed.as_ref().map(|r| r.peer.clone()).unwrap_or_default();
                let module = removed
                    .as_ref()
                    .map(|r| r.module.clone())
                    .unwrap_or_default();
                let path = removed.as_ref().map(|r| r.path.clone()).unwrap_or_default();
                let row_daemon = removed
                    .as_ref()
                    .map(|r| r.source_daemon.clone())
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| source_daemon.to_string());
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
                    source_daemon: row_daemon,
                });
                true
            }
            Some(daemon_event::Payload::TransferError(e)) => {
                let removed = self.active.remove(&row_key(source_daemon, &e.transfer_id));
                let kind = removed.as_ref().map(|r| r.kind).unwrap_or(0);
                let peer = removed.as_ref().map(|r| r.peer.clone()).unwrap_or_default();
                let module = removed
                    .as_ref()
                    .map(|r| r.module.clone())
                    .unwrap_or_default();
                let path = removed.as_ref().map(|r| r.path.clone()).unwrap_or_default();
                let row_daemon = removed
                    .as_ref()
                    .map(|r| r.source_daemon.clone())
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| source_daemon.to_string());
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
                    source_daemon: row_daemon,
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

    /// d-21 R2: derive the display index from the
    /// id-anchored cursor. Returns `None` when the
    /// selected transfer is no longer present (terminated
    /// or never seen) — the operator has to press j/k to
    /// re-anchor.
    pub fn selected_active_index(&self) -> Option<usize> {
        let key = self.selected_active_key.as_ref()?;
        self.active_rows()
            .iter()
            .position(|r| row_key(&r.source_daemon, &r.transfer_id) == *key)
    }

    /// d-22: the transfer_id at the cursor — the target
    /// for `K` (cancel-selected). `None` when the cursor
    /// isn't anchored (no navigation yet) OR when the
    /// anchored id has terminated (cursor fell off).
    /// Callers MUST check `Some` before firing CancelJob;
    /// the d-21 R2 fall-off contract means we never lie
    /// about which transfer is selected.
    pub fn selected_active_id(&self) -> Option<&str> {
        let key = self.selected_active_key.as_ref()?;
        // The cursor anchors on the composite key; callers want the
        // bare transfer_id (CancelJob targets it). Look the row up and
        // return its id — `None` when the cursor fell off (terminated).
        self.active.get(key).map(|r| r.transfer_id.as_str())
    }

    /// m2f-7: the source daemon of the cursor's transfer — CancelJob
    /// must target THIS daemon, not the launch `parsed_remote`, now
    /// that F2 shows rows from every daemon. `None` when the cursor
    /// isn't anchored or fell off (same contract as
    /// [`Self::selected_active_id`]).
    pub fn selected_active_daemon(&self) -> Option<&str> {
        let key = self.selected_active_key.as_ref()?;
        self.active.get(key).map(|r| r.source_daemon.as_str())
    }

    /// d-21 R2: advance the cursor. If the previously
    /// selected id is no longer present (transfer
    /// terminated), the next press re-anchors at index 0
    /// rather than walking forward from a stale index.
    /// First call from no-cursor lands on index 0.
    pub fn select_next_active(&mut self) {
        let rows = self.active_rows();
        if rows.is_empty() {
            self.selected_active_key = None;
            return;
        }
        let next_idx = match self.selected_active_index() {
            None => 0,
            Some(idx) => (idx + 1).min(rows.len() - 1),
        };
        self.selected_active_key = Some(row_key(
            &rows[next_idx].source_daemon,
            &rows[next_idx].transfer_id,
        ));
    }

    /// d-21 R2: walk the cursor up. Same re-anchor
    /// semantics as `select_next_active` — a stale id
    /// resets to index 0 instead of "saturate at 0 from
    /// nowhere."
    pub fn select_prev_active(&mut self) {
        let rows = self.active_rows();
        if rows.is_empty() {
            self.selected_active_key = None;
            return;
        }
        let prev_idx = match self.selected_active_index() {
            None => 0,
            Some(idx) => idx.saturating_sub(1),
        };
        self.selected_active_key = Some(row_key(
            &rows[prev_idx].source_daemon,
            &rows[prev_idx].transfer_id,
        ));
    }

    /// d-44: anchor the cursor on the first active row (`g`).
    /// No-op when there are no active transfers — leaves the
    /// cursor unanchored rather than inventing a selection.
    pub fn select_first_active(&mut self) {
        let rows = self.active_rows();
        self.selected_active_key = rows
            .first()
            .map(|r| row_key(&r.source_daemon, &r.transfer_id));
    }

    /// d-44: anchor the cursor on the last active row (`G`).
    /// No-op when there are no active transfers.
    pub fn select_last_active(&mut self) {
        let rows = self.active_rows();
        self.selected_active_key = rows
            .last()
            .map(|r| row_key(&r.source_daemon, &r.transfer_id));
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

/// m2f-2: the `active` map's key. `transfer_id` is `t<ms>-<n>`,
/// unique only within a daemon, so once F2 fans out across daemons
/// (m2f-2/m2f-3) two daemons can mint the same id. Keying by
/// `(daemon, id)` keeps their rows distinct. The unit-separator can't
/// appear in a host or id, so the join is unambiguous.
fn row_key(source_daemon: &str, transfer_id: &str) -> String {
    format!("{source_daemon}\u{1f}{transfer_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::generated::{
        ActiveTransfer, DaemonEvent, DaemonState, TransferComplete, TransferError, TransferKind,
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

    // d-13: last_event_at timestamps mutate-on-apply.

    #[test]
    fn last_event_at_none_until_first_mutation() {
        let state = TransfersState::new();
        assert_eq!(state.last_event_at(), None);
    }

    #[test]
    fn merge_snapshot_stamps_last_event_at() {
        let mut state = TransfersState::new();
        let stamp = Instant::now();
        state.merge_snapshot("", DaemonState::default(), stamp);
        assert_eq!(state.last_event_at(), Some(stamp));
    }

    // d-21: F2 active-row cursor selection.

    fn started_event(id: &str) -> DaemonEvent {
        DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: id.to_string(),
                kind: TransferKind::DelegatedPull as i32,
                peer: String::new(),
                module: String::new(),
                path: String::new(),
                start_unix_ms: 0,
            })),
        }
    }

    /// m2f-1: active rows record the source daemon they were
    /// reported from, and a Complete carries it to the recent row —
    /// the foundation the multi-daemon fan-out (m2f-2) relies on to
    /// label rows by daemon.
    #[test]
    fn rows_record_source_daemon() {
        let mut state = TransfersState::new();
        state.apply_event("nas", started_event("t1"), Instant::now());
        assert_eq!(state.active_rows()[0].source_daemon, "nas");

        let complete = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                transfer_id: "t1".to_string(),
                duration_ms: 10,
                bytes: 100,
                files: 1,
                tcp_fallback_used: false,
            })),
        };
        state.apply_event("nas", complete, Instant::now());
        assert_eq!(
            state
                .recent_rows()
                .next()
                .expect("recent row")
                .source_daemon,
            "nas",
            "Complete carries the source daemon to the recent row"
        );
    }

    /// m2f-2: two daemons can mint the same `transfer_id`
    /// (`t<ms>-<n>` is unique only within a daemon), so the F2 view
    /// must keep their rows distinct (composite `(daemon, id)` key) —
    /// not collapse them — and completing one must not evict the
    /// other.
    #[test]
    fn same_id_from_two_daemons_stays_distinct() {
        let mut state = TransfersState::new();
        state.apply_event("nas", started_event("t1"), Instant::now());
        state.apply_event("skippy", started_event("t1"), Instant::now());
        assert_eq!(state.active_count(), 2, "same id on two daemons → two rows");

        let complete = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                transfer_id: "t1".to_string(),
                duration_ms: 1,
                bytes: 0,
                files: 0,
                tcp_fallback_used: false,
            })),
        };
        // Completing t1 on nas must leave skippy's t1 untouched.
        state.apply_event("nas", complete, Instant::now());
        assert_eq!(state.active_count(), 1, "only nas's row left");
        assert_eq!(state.active_rows()[0].source_daemon, "skippy");
        // And nas's t1 in recent doesn't dedup-suppress skippy's t1.
        assert_eq!(state.recent_count(), 1);
    }

    /// m2f-9 R3: when a daemon leaves the watch set, its in-flight
    /// active rows are pruned (it can no longer send a Complete/Error),
    /// but its recent rows survive as history. The active cursor clears
    /// if it was anchored to a pruned row.
    #[test]
    fn retain_active_daemons_drops_unwatched_active_keeps_recent() {
        let mut state = TransfersState::new();
        state.apply_event("nas", started_event("a1"), Instant::now());
        state.apply_event("skippy", started_event("b1"), Instant::now());
        // skippy also has a completed (recent) transfer.
        state.apply_event("skippy", started_event("b0"), Instant::now());
        let complete = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                transfer_id: "b0".to_string(),
                duration_ms: 1,
                bytes: 0,
                files: 0,
                tcp_fallback_used: false,
            })),
        };
        state.apply_event("skippy", complete, Instant::now());
        assert_eq!(state.active_count(), 2, "nas:a1 + skippy:b1 active");
        assert_eq!(state.recent_count(), 1, "skippy:b0 recent");

        // Anchor the cursor on skippy's active row, then drop skippy
        // from the watch set.
        state.select_first_active();
        let watched: std::collections::BTreeSet<String> = ["nas".to_string()].into_iter().collect();
        state.retain_active_daemons(&watched);

        let daemons: Vec<&str> = state
            .active_rows()
            .iter()
            .map(|r| r.source_daemon.as_str())
            .collect();
        assert_eq!(
            daemons,
            vec!["nas"],
            "only the watched daemon's active row remains"
        );
        assert_eq!(
            state.recent_count(),
            1,
            "skippy's recent (history) row is kept"
        );
        // The cursor either fell off (anchored to skippy) or re-anchored
        // to a surviving row — never points at a pruned daemon.
        if let Some(daemon) = state.selected_active_daemon() {
            assert_eq!(daemon, "nas", "cursor never points at a pruned daemon");
        }
    }

    /// m2f-2 round 2: two daemon instances on the SAME host but
    /// different ports are valid, and the daemon identity is
    /// `host_port_display()` (e.g. `nas:9001` vs `nas:9002`). With the
    /// same `transfer_id` they must stay distinct — a host-only
    /// identity would have collided them like the pre-m2f-2 bare-id
    /// map.
    #[test]
    fn same_host_different_port_daemons_stay_distinct() {
        let mut state = TransfersState::new();
        state.apply_event("nas:9001", started_event("t1"), Instant::now());
        state.apply_event("nas:9002", started_event("t1"), Instant::now());
        assert_eq!(
            state.active_count(),
            2,
            "distinct port identities → two rows"
        );

        let complete = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                transfer_id: "t1".to_string(),
                duration_ms: 1,
                bytes: 0,
                files: 0,
                tcp_fallback_used: false,
            })),
        };
        state.apply_event("nas:9001", complete, Instant::now());
        assert_eq!(state.active_count(), 1, "only :9002 left");
        assert_eq!(state.active_rows()[0].source_daemon, "nas:9002");
        // The :9001 recent row must not dedup-suppress :9002's t1.
        assert_eq!(state.recent_count(), 1);
    }

    /// m2f-3: `merge_snapshot` hydrates one daemon without disturbing
    /// others — the additive primitive the fan-out (m2f-4) needs.
    #[test]
    fn merge_snapshot_is_additive_per_daemon() {
        let active = |id: &str| ActiveTransfer {
            transfer_id: id.to_string(),
            ..Default::default()
        };
        let mut state = TransfersState::new();
        // Daemon A: two active.
        state.merge_snapshot(
            "nas",
            DaemonState {
                active: vec![active("a1"), active("a2")],
                ..Default::default()
            },
            Instant::now(),
        );
        // Daemon B: one active — must NOT drop A's rows.
        state.merge_snapshot(
            "skippy",
            DaemonState {
                active: vec![active("b1")],
                ..Default::default()
            },
            Instant::now(),
        );
        assert_eq!(state.active_count(), 3, "A's 2 + B's 1 coexist");

        // Re-merging A replaces only A's rows; B's stay.
        state.merge_snapshot(
            "nas",
            DaemonState {
                active: vec![active("a3")],
                ..Default::default()
            },
            Instant::now(),
        );
        assert_eq!(state.active_count(), 2, "A now has 1 (a3), B still has b1");
        let daemons: std::collections::HashSet<&str> = state
            .active_rows()
            .iter()
            .map(|r| r.source_daemon.as_str())
            .collect();
        assert!(daemons.contains("nas") && daemons.contains("skippy"));
    }

    /// m2f-7: the cursor exposes its row's source daemon (CancelJob's
    /// target) consistently with its transfer_id — both from the SAME
    /// selected row, even with rows from different daemons.
    #[test]
    fn selected_active_daemon_matches_cursor_row() {
        let mut state = TransfersState::new();
        state.apply_event("nas", started_event("t1"), Instant::now());
        state.apply_event("skippy:9001", started_event("t2"), Instant::now());
        assert!(state.selected_active_daemon().is_none(), "no cursor yet");
        state.select_first_active();
        let daemon = state.selected_active_daemon().expect("daemon");
        let id = state.selected_active_id().expect("id");
        // The (daemon, id) pair belongs to one row, not a cross of two.
        assert!(
            (daemon == "nas" && id == "t1") || (daemon == "skippy:9001" && id == "t2"),
            "selected daemon+id come from the same row: {daemon} / {id}"
        );
    }

    /// m2f-1: a snapshot tags every row with the daemon it came from.
    #[test]
    fn snapshot_tags_rows_with_source_daemon() {
        let mut state = TransfersState::new();
        let snap = DaemonState {
            active: vec![ActiveTransfer {
                transfer_id: "a1".to_string(),
                ..Default::default()
            }],
            recent: vec![TransferRecord {
                transfer_id: "r1".to_string(),
                ok: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        state.merge_snapshot("skippy", snap, Instant::now());
        assert_eq!(state.active_rows()[0].source_daemon, "skippy");
        assert_eq!(
            state.recent_rows().next().expect("recent").source_daemon,
            "skippy"
        );
    }

    #[test]
    fn selected_active_index_is_none_until_first_navigation() {
        let state = TransfersState::new();
        assert!(state.selected_active_index().is_none());
    }

    #[test]
    fn select_next_active_lands_on_index_zero_first_time() {
        let mut state = TransfersState::new();
        state.apply_event("", started_event("t-1"), Instant::now());
        state.apply_event("", started_event("t-2"), Instant::now());
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(0));
    }

    #[test]
    fn select_next_active_walks_through_rows() {
        let mut state = TransfersState::new();
        state.apply_event("", started_event("t-1"), Instant::now());
        state.apply_event("", started_event("t-2"), Instant::now());
        state.apply_event("", started_event("t-3"), Instant::now());
        state.select_next_active();
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(1));
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(2));
        // Clamps at end.
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(2));
    }

    #[test]
    fn select_prev_active_saturates_at_zero() {
        let mut state = TransfersState::new();
        state.apply_event("", started_event("t-1"), Instant::now());
        state.apply_event("", started_event("t-2"), Instant::now());
        state.select_next_active();
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(1));
        state.select_prev_active();
        assert_eq!(state.selected_active_index(), Some(0));
        state.select_prev_active();
        assert_eq!(state.selected_active_index(), Some(0));
    }

    #[test]
    fn select_next_active_no_op_on_empty_list() {
        let mut state = TransfersState::new();
        state.select_next_active();
        assert!(state.selected_active_index().is_none());
    }

    /// d-44: `g` / `G` anchor the F2 cursor on the first /
    /// last active row.
    #[test]
    fn select_first_and_last_active_anchor_the_cursor() {
        let mut state = TransfersState::new();
        state.apply_event("", started_event("t-1"), Instant::now());
        state.apply_event("", started_event("t-2"), Instant::now());
        state.apply_event("", started_event("t-3"), Instant::now());
        state.select_last_active();
        assert_eq!(
            state.selected_active_index(),
            Some(2),
            "G anchors on the last active row"
        );
        state.select_first_active();
        assert_eq!(
            state.selected_active_index(),
            Some(0),
            "g anchors on the first active row"
        );
    }

    #[test]
    fn select_first_last_active_noop_on_empty_list() {
        let mut state = TransfersState::new();
        state.select_first_active();
        assert!(state.selected_active_index().is_none());
        state.select_last_active();
        assert!(state.selected_active_index().is_none());
    }

    // d-21 R2: id-anchored cursor doesn't silently
    // retarget when the selected row is removed.

    /// Reopen scenario A: 3 active rows, cursor on the
    /// middle one. The middle row Completes. The cursor
    /// must "fall off" (selected_active_index → None)
    /// rather than shift to the row that *was* third —
    /// pre-fix the index stayed at 1 and silently pointed
    /// at a different transfer.
    #[test]
    fn middle_row_complete_does_not_retarget_cursor() {
        let mut state = TransfersState::new();
        // Distinct start_unix_ms so sort order is stable.
        let mut now = Instant::now();
        for (id, start) in [("t-1", 100), ("t-2", 200), ("t-3", 300)] {
            let ev = DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: id.to_string(),
                    kind: TransferKind::DelegatedPull as i32,
                    peer: String::new(),
                    module: String::new(),
                    path: String::new(),
                    start_unix_ms: start,
                })),
            };
            state.apply_event("", ev, now);
            now += std::time::Duration::from_millis(1);
        }
        // Sort order is newest-first by start_unix_ms:
        // t-3 (300), t-2 (200), t-1 (100). Walk to index 1
        // = t-2 (the middle row).
        state.select_next_active();
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(1));
        // Complete t-2.
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferComplete(
                    blit_core::generated::TransferComplete {
                        transfer_id: "t-2".to_string(),
                        duration_ms: 100,
                        bytes: 0,
                        files: 0,
                        tcp_fallback_used: false,
                    },
                )),
            },
            now,
        );
        // Cursor falls off — operator must press j/k to
        // re-anchor on either remaining row.
        assert!(
            state.selected_active_index().is_none(),
            "middle row completion must not retarget the cursor at the next transfer"
        );
    }

    /// Reopen scenario B: single active row, cursor on
    /// it. Row completes (list empty). A new unrelated
    /// transfer starts later. The cursor must still be
    /// off-list — pre-fix the index stayed Some(0) and
    /// the new row got an unintended selection.
    #[test]
    fn solo_row_complete_then_new_start_keeps_cursor_off() {
        let mut state = TransfersState::new();
        let now = Instant::now();
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: "t-1".to_string(),
                    kind: TransferKind::DelegatedPull as i32,
                    peer: String::new(),
                    module: String::new(),
                    path: String::new(),
                    start_unix_ms: 1,
                })),
            },
            now,
        );
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(0));
        // Complete the solo row → list empty.
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferComplete(
                    blit_core::generated::TransferComplete {
                        transfer_id: "t-1".to_string(),
                        duration_ms: 100,
                        bytes: 0,
                        files: 0,
                        tcp_fallback_used: false,
                    },
                )),
            },
            now,
        );
        assert!(state.selected_active_index().is_none());
        // Unrelated new transfer starts.
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: "t-2".to_string(),
                    kind: TransferKind::DelegatedPull as i32,
                    peer: String::new(),
                    module: String::new(),
                    path: String::new(),
                    start_unix_ms: 2,
                })),
            },
            now,
        );
        // Cursor MUST still be off-list — the operator
        // never selected t-2.
        assert!(
            state.selected_active_index().is_none(),
            "new unrelated transfer must not auto-select; operator must press j/k"
        );
    }

    /// Cursor "falls off" when the underlying row is
    /// removed (Complete/Error). `selected_active_index`
    /// returns None so the caller knows the cursor isn't
    /// pointing at a real row anymore. Operator can press
    /// up/down to re-anchor.
    #[test]
    fn selected_active_index_falls_off_when_row_terminates() {
        let mut state = TransfersState::new();
        state.apply_event("", started_event("t-1"), Instant::now());
        state.select_next_active();
        assert_eq!(state.selected_active_index(), Some(0));
        // Complete the only row → list empty → cursor off-list.
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferComplete(
                    blit_core::generated::TransferComplete {
                        transfer_id: "t-1".to_string(),
                        duration_ms: 100,
                        bytes: 0,
                        files: 0,
                        tcp_fallback_used: false,
                    },
                )),
            },
            Instant::now(),
        );
        assert!(state.selected_active_index().is_none());
    }

    #[test]
    fn apply_event_stamps_only_on_mutation() {
        let mut state = TransfersState::new();
        // Started for a new id: mutates, stamps.
        let started_stamp = Instant::now();
        let mutated = state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: "t-1".to_string(),
                    kind: TransferKind::DelegatedPull as i32,
                    peer: String::new(),
                    module: String::new(),
                    path: String::new(),
                    start_unix_ms: 0,
                })),
            },
            started_stamp,
        );
        assert!(mutated);
        assert_eq!(state.last_event_at(), Some(started_stamp));

        // Progress for unknown id: no-op, last_event_at
        // must NOT advance to the no-op stamp.
        let noop_stamp = started_stamp + std::time::Duration::from_secs(1);
        let mutated = state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
                    transfer_id: "unknown".to_string(),
                    bytes_completed: 0,
                    bytes_total: 0,
                    throughput_bps: 0,
                    files_completed: 0,
                    files_total: 0,
                })),
            },
            noop_stamp,
        );
        assert!(!mutated);
        assert_eq!(
            state.last_event_at(),
            Some(started_stamp),
            "no-op events must not refresh last_event_at",
        );
    }

    #[test]
    fn merge_snapshot_populates_active_and_recent() {
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
        state.merge_snapshot("", snapshot, Instant::now());
        assert_eq!(state.active_count(), 2);
        assert_eq!(state.recent_count(), 2);
    }

    #[test]
    fn apply_event_progress_updates_row_in_place() {
        let mut state = TransfersState::new();
        state.merge_snapshot(
            "",
            DaemonState {
                active: vec![make_active("t-1", 0)],
                ..DaemonState::default()
            },
            Instant::now(),
        );
        let mutated = state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
                    transfer_id: "t-1".to_string(),
                    bytes_completed: 4096,
                    bytes_total: 0,
                    files_completed: 0,
                    files_total: 0,
                    throughput_bps: 1_000_000,
                })),
            },
            Instant::now(),
        );
        assert!(mutated);
        let row = state.active_rows()[0];
        assert_eq!(row.bytes_completed, 4096);
        assert_eq!(row.throughput_bps, 1_000_000);
    }

    #[test]
    fn apply_event_progress_for_unknown_id_returns_false() {
        let mut state = TransfersState::new();
        let mutated = state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
                    transfer_id: "unknown".to_string(),
                    bytes_completed: 0,
                    bytes_total: 0,
                    files_completed: 0,
                    files_total: 0,
                    throughput_bps: 0,
                })),
            },
            Instant::now(),
        );
        assert!(!mutated);
        assert_eq!(state.active_count(), 0);
    }

    #[test]
    fn apply_event_complete_moves_row_to_recent() {
        let mut state = TransfersState::new();
        state.merge_snapshot(
            "",
            DaemonState {
                active: vec![make_active("t-1", 0)],
                ..DaemonState::default()
            },
            Instant::now(),
        );
        let mutated = state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                    transfer_id: "t-1".to_string(),
                    bytes: 1_000_000,
                    files: 0,
                    duration_ms: 5_000,
                    tcp_fallback_used: false,
                })),
            },
            Instant::now(),
        );
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
        state.merge_snapshot(
            "",
            DaemonState {
                active: vec![make_active("t-1", 0)],
                ..DaemonState::default()
            },
            Instant::now(),
        );
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferError(TransferError {
                    transfer_id: "t-1".to_string(),
                    message: "module not found".to_string(),
                })),
            },
            Instant::now(),
        );
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
        assert!(state.apply_event("", ev.clone(), Instant::now()));
        assert_eq!(state.active_count(), 1);
        // Second apply for the same id: returns false
        // (state didn't change). Counter stays at 1.
        assert!(!state.apply_event("", ev, Instant::now()));
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
        state.merge_snapshot(
            "",
            DaemonState {
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
            },
            Instant::now(),
        );
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
        assert!(!state.apply_event("", started, Instant::now()));
        // Snapshot's bytes_completed preserved.
        let row = &state.active_rows()[0];
        assert_eq!(row.bytes_completed, 500_000);
    }

    /// a1-2 round-4 regression: the buffered Started +
    /// Complete pair for a transfer that's ALREADY in the
    /// snapshot's `recent[]` (because it completed before
    /// the GetState response arrived) MUST NOT produce a
    /// duplicate recent row. Terminal-id dedup at the start
    /// of `apply_event` short-circuits subsequent events
    /// once an id is in recent.
    #[test]
    fn buffered_events_dedupe_against_snapshot_recent() {
        let mut state = TransfersState::new();
        // Snapshot already has the transfer in recent[].
        state.merge_snapshot(
            "",
            DaemonState {
                recent: vec![make_record("race-id", true)],
                ..DaemonState::default()
            },
            Instant::now(),
        );
        assert_eq!(state.recent_count(), 1);

        // Buffered Started — should be ignored.
        let started_mutated = state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: "race-id".to_string(),
                    kind: TransferKind::Pull as i32,
                    peer: "p".to_string(),
                    module: "m".to_string(),
                    path: "/".to_string(),
                    start_unix_ms: 1,
                })),
            },
            Instant::now(),
        );
        assert!(!started_mutated);
        assert_eq!(state.active_count(), 0);
        assert_eq!(state.recent_count(), 1);

        // Buffered Complete — should also be ignored.
        let complete_mutated = state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferComplete(
                    blit_core::generated::TransferComplete {
                        transfer_id: "race-id".to_string(),
                        bytes: 0,
                        files: 0,
                        duration_ms: 0,
                        tcp_fallback_used: false,
                    },
                )),
            },
            Instant::now(),
        );
        assert!(!complete_mutated);
        // Still exactly one recent row.
        assert_eq!(state.recent_count(), 1);
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
        state.merge_snapshot("", DaemonState::default(), Instant::now());

        // Apply buffered Started first.
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: "race-id".to_string(),
                    kind: TransferKind::DelegatedPull as i32,
                    peer: "race-peer".to_string(),
                    module: "race-mod".to_string(),
                    path: "race/path".to_string(),
                    start_unix_ms: 1,
                })),
            },
            Instant::now(),
        );
        // Then buffered Complete.
        state.apply_event(
            "",
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferComplete(
                    blit_core::generated::TransferComplete {
                        transfer_id: "race-id".to_string(),
                        bytes: 999,
                        files: 0,
                        duration_ms: 50,
                        tcp_fallback_used: false,
                    },
                )),
            },
            Instant::now(),
        );

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
            state.apply_event(
                "",
                DaemonEvent {
                    payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                        transfer_id: format!("t-{i}"),
                        bytes: 0,
                        files: 0,
                        duration_ms: 0,
                        tcp_fallback_used: false,
                    })),
                },
                Instant::now(),
            );
        }
        assert_eq!(state.recent_count(), TUI_RECENT_CAP);
        // Newest-first: the most recent id should be on top.
        let newest = state.recent_rows().next().unwrap();
        assert_eq!(newest.transfer_id, format!("t-{}", TUI_RECENT_CAP + 4));
    }
}
