//! F2 Transfers screen. Two stacked tables: active rows on
//! top, recent rows on the bottom. Footer shows remote +
//! connection status + key hints.
//!
//! Renderer is pure — takes a [`TransfersState`] reference
//! and a [`ConnectionStatus`] string and emits widgets. The
//! event loop in `main.rs` owns the state and the Subscribe
//! stream; this module just paints.
//!
//! Layout (heights are constraints; columns reflect the
//! d-14 / d-15 / d-20 polish):
//!
//! ```text
//! ┌── header (1 line) ──────────────────────────────────────────────┐
//! │ blit-tui · F2 Transfers · <remote> · N active · N recent        │
//! ├── active table (Min 5) ─────────────────────────────────────────┤
//! │ id  kind  peer  module/path  bytes·NN%  throughput  age         │
//! │ ...                                                             │
//! ├── recent table (Min 5) ─────────────────────────────────────────┤
//! │ id  kind  peer  module/path  bytes  duration  throughput        │
//! │ ...                                                             │
//! ├── footer (1 line) ──────────────────────────────────────────────┤
//! │ status · [last event Xs ago] · q/Esc quit · r refresh           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use crate::state::{ActiveRow, RecentRow, TransfersState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::time::Instant;

/// Connection-status banner rendered in the footer.
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    /// No remote configured (no `--remote` flag).
    NoRemote,
    /// Initial GetState in flight.
    Connecting,
    /// Subscribe stream live.
    Live,
    /// Subscribe stream errored; falling back to periodic
    /// GetState reconcile.
    Degraded(String),
}

/// Render the F2 screen into `frame`. The renderer is a free
/// function so unit tests can call it against synthetic
/// state + a `TestBackend`-backed Terminal.
/// Render the F2 pane into a caller-supplied area. Used
/// by the router (a1-6) to leave room for the tab strip.
pub fn render_into(
    frame: &mut Frame,
    area: Rect,
    state: &TransfersState,
    remote_label: &str,
    status: &ConnectionStatus,
    now: Instant,
) {
    // d-14: the active-table "age" column compares
    // `start_unix_ms` (wall-clock at transfer start, sent
    // by the daemon) against the operator's wall-clock
    // now — Instant is monotonic so it can't be used here.
    // We capture the wall-clock once per frame so each
    // row's age computes against the same anchor.
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], remote_label, state);
    render_active_table(frame, chunks[1], state, now_unix_ms);
    render_recent_table(frame, chunks[2], state);
    render_footer(frame, chunks[3], status, state.last_event_at(), now);
}

fn render_header(frame: &mut Frame, area: Rect, remote_label: &str, state: &TransfersState) {
    let title = format!(
        " blit-tui · F2 Transfers · {} · {} active · {} recent ",
        remote_label,
        state.active_count(),
        state.recent_count(),
    );
    let para = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(para, area);
}

fn render_active_table(frame: &mut Frame, area: Rect, state: &TransfersState, now_unix_ms: u64) {
    let rows: Vec<Row> = state
        .active_rows()
        .into_iter()
        .map(|r| active_row_to_table_row(r, now_unix_ms))
        .collect();
    let widths = [
        Constraint::Length(20),
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Min(20),
        // d-15: bytes column carries "bytes · NN%" so we
        // need 18 chars for worst-case "1023.99 MiB · 100%".
        Constraint::Length(18),
        Constraint::Length(12),
        Constraint::Length(10),
    ];
    let header = Row::new(vec![
        Cell::from("transfer_id"),
        Cell::from("kind"),
        Cell::from("peer"),
        Cell::from("module/path"),
        Cell::from("bytes"),
        Cell::from("throughput"),
        Cell::from("age"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Active "))
        // d-21: highlight the row at the cursor index.
        // Black-on-cyan matches the tab-strip active-tab
        // visual (e-7 made that themable; the Active row
        // highlight stays Cyan for now — operator-visible
        // accent is the tab strip, the row highlight is
        // an internal selection marker).
        .row_highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let mut table_state = TableState::default().with_selected(state.selected_active_index());
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_recent_table(frame: &mut Frame, area: Rect, state: &TransfersState) {
    let rows: Vec<Row> = state.recent_rows().map(recent_row_to_table_row).collect();
    let widths = [
        Constraint::Length(20),
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(12),
    ];
    let header = Row::new(vec![
        Cell::from("transfer_id"),
        Cell::from("kind"),
        Cell::from("peer"),
        Cell::from("module/path"),
        Cell::from("bytes"),
        Cell::from("duration"),
        Cell::from("throughput"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Recent "));
    frame.render_widget(table, area);
}

fn render_footer(
    frame: &mut Frame,
    area: Rect,
    status: &ConnectionStatus,
    last_event_at: Option<Instant>,
    now: Instant,
) {
    let status_span = match status {
        ConnectionStatus::NoRemote => Span::styled(
            "no --remote — read-only splash",
            Style::default().fg(Color::Yellow),
        ),
        ConnectionStatus::Connecting => {
            Span::styled("connecting...", Style::default().fg(Color::Yellow))
        }
        ConnectionStatus::Live => Span::styled("live", Style::default().fg(Color::Green)),
        ConnectionStatus::Degraded(msg) => {
            Span::styled(format!("degraded: {msg}"), Style::default().fg(Color::Red))
        }
    };
    let mut spans = vec![status_span];
    // d-13: surface "last event Xs ago" when the
    // Subscribe stream / GetState snapshot has produced
    // anything. Hidden while NoRemote (nothing to fetch)
    // and pre-first-event (`last_event_at` is None).
    if let Some(at) = last_event_at {
        spans.push(Span::raw("  ·  "));
        spans.push(Span::styled(
            format!("last event {}", format_since(now, at)),
            Style::default().fg(Color::DarkGray),
        ));
    }
    spans.extend(vec![
        Span::raw("  ·  "),
        Span::styled("q/Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit  ·  "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" refresh"),
    ]);
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn format_since(now: Instant, then: Instant) -> String {
    let elapsed = now.saturating_duration_since(then);
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

fn active_row_to_table_row(row: &ActiveRow, now_unix_ms: u64) -> Row<'static> {
    Row::new(vec![
        Cell::from(row.transfer_id.clone()),
        Cell::from(kind_label(row.kind).to_string()),
        Cell::from(row.peer.clone()),
        Cell::from(module_path(&row.module, &row.path)),
        Cell::from(format_bytes_progress(row.bytes_completed, row.bytes_total)),
        Cell::from(if row.throughput_bps == 0 {
            "-".to_string()
        } else {
            format!("{}/s", format_bytes(row.throughput_bps))
        }),
        Cell::from(format_age_from_unix_ms(now_unix_ms, row.start_unix_ms)),
    ])
}

/// d-14: format the age of an active transfer for the
/// "age" column in F2's active table. `start_unix_ms` is
/// the wall-clock millisecond timestamp the daemon
/// stamped at transfer start (`ActiveTransfer::start_unix_ms`).
/// `now_unix_ms` is the operator's wall-clock now,
/// captured once per frame.
///
/// Returns "-" if `now < start` (clock skew between TUI
/// and daemon hosts) or if either value is zero. The
/// daemon stamps the field on every active row, so the
/// zero case shouldn't happen in practice — but the
/// renderer must not panic on garbage input from the
/// wire.
fn format_age_from_unix_ms(now_unix_ms: u64, start_unix_ms: u64) -> String {
    if now_unix_ms == 0 || start_unix_ms == 0 || now_unix_ms < start_unix_ms {
        return "-".to_string();
    }
    let age_ms = now_unix_ms - start_unix_ms;
    if age_ms < 1000 {
        return format!("{age_ms}ms");
    }
    let secs = age_ms / 1000;
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

fn recent_row_to_table_row(row: &RecentRow) -> Row<'static> {
    let status_style = if row.ok {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    };
    Row::new(vec![
        Cell::from(row.transfer_id.clone()),
        Cell::from(kind_label(row.kind).to_string()),
        Cell::from(row.peer.clone()),
        Cell::from(module_path(&row.module, &row.path)),
        Cell::from(format_bytes(row.bytes)),
        Cell::from(if row.ok {
            format_ms(row.duration_ms)
        } else {
            format!("FAIL: {}", row.error_message)
        }),
        Cell::from(format_recent_throughput(row)),
    ])
    .style(status_style)
}

/// d-20: average throughput for a completed F2 recent
/// row. Hidden ("-") when the rate would be misleading
/// (failed transfer, zero bytes, sub-millisecond
/// duration). Same shape as d-10's `format_rate` on F4
/// transfer Done — the operator gets a consistent
/// "X MiB/s" reading on both surfaces.
fn format_recent_throughput(row: &RecentRow) -> String {
    if !row.ok {
        return "-".to_string();
    }
    if row.bytes == 0 || row.duration_ms == 0 {
        return "-".to_string();
    }
    let bytes_per_sec = ((row.bytes as u128).saturating_mul(1000) / row.duration_ms as u128) as f64;
    if bytes_per_sec < 1.0 {
        return "-".to_string();
    }
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    if bytes_per_sec >= GIB {
        format!("{:.1} GiB/s", bytes_per_sec / GIB)
    } else if bytes_per_sec >= MIB {
        format!("{:.1} MiB/s", bytes_per_sec / MIB)
    } else if bytes_per_sec >= KIB {
        format!("{:.1} KiB/s", bytes_per_sec / KIB)
    } else {
        format!("{} B/s", bytes_per_sec.round() as u64)
    }
}

fn kind_label(kind: i32) -> &'static str {
    blit_app::admin::jobs::kind_label(kind)
}

fn module_path(module: &str, path: &str) -> String {
    match (module.is_empty(), path.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => path.to_string(),
        (false, true) => module.to_string(),
        (false, false) => format!("{module}/{path}"),
    }
}

fn format_bytes(n: u64) -> String {
    if n >= 1 << 30 {
        format!("{:.2} GiB", n as f64 / (1u64 << 30) as f64)
    } else if n >= 1 << 20 {
        format!("{:.2} MiB", n as f64 / (1u64 << 20) as f64)
    } else if n >= 1 << 10 {
        format!("{:.2} KiB", n as f64 / (1u64 << 10) as f64)
    } else {
        format!("{n} B")
    }
}

/// d-15: bytes-completed display for F2's active rows.
/// When `bytes_total > 0` (the daemon knows how much is
/// in flight) we append a percentage so the operator can
/// see fraction-complete at a glance:
///
///     500 KiB · 10%
///     1.20 GiB · 75%
///     0 B · 0%
///
/// When `bytes_total == 0` the daemon hasn't measured the
/// total yet (e.g. a remote pull whose ls hasn't returned),
/// so we just show the raw byte count to avoid lying about
/// a meaningless 0%.
///
/// `completed > total` (daemon counter drift) clamps the
/// percent to 100 — the percent is operator-facing UX, not
/// authoritative arithmetic.
fn format_bytes_progress(completed: u64, total: u64) -> String {
    let bytes_str = format_bytes(completed);
    if total == 0 {
        return bytes_str;
    }
    let percent = if completed >= total {
        100
    } else {
        // No overflow: completed < total ≤ u64::MAX, so
        // completed * 100 fits in u128.
        ((completed as u128 * 100) / total as u128) as u64
    };
    format!("{bytes_str} · {percent}%")
}

fn format_ms(n: u64) -> String {
    if n >= 1000 {
        format!("{:.1}s", n as f64 / 1000.0)
    } else {
        format!("{n}ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_picks_correct_unit() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
    }

    #[test]
    fn format_ms_picks_correct_unit() {
        assert_eq!(format_ms(0), "0ms");
        assert_eq!(format_ms(999), "999ms");
        assert_eq!(format_ms(1500), "1.5s");
    }

    // d-20: F2 recent-row throughput column.

    fn recent_row(bytes: u64, duration_ms: u64, ok: bool) -> RecentRow {
        RecentRow {
            transfer_id: "t".to_string(),
            kind: 0,
            peer: String::new(),
            module: String::new(),
            path: String::new(),
            duration_ms,
            bytes,
            ok,
            error_message: String::new(),
        }
    }

    #[test]
    fn recent_throughput_dash_for_failed_transfer() {
        let r = recent_row(1024 * 1024, 1000, false);
        assert_eq!(format_recent_throughput(&r), "-");
    }

    #[test]
    fn recent_throughput_dash_for_zero_bytes() {
        let r = recent_row(0, 1000, true);
        assert_eq!(format_recent_throughput(&r), "-");
    }

    #[test]
    fn recent_throughput_dash_for_zero_duration() {
        let r = recent_row(1024 * 1024, 0, true);
        assert_eq!(format_recent_throughput(&r), "-");
    }

    #[test]
    fn recent_throughput_kibibytes() {
        // 1 KiB in 1s = 1.0 KiB/s.
        let r = recent_row(1024, 1000, true);
        assert_eq!(format_recent_throughput(&r), "1.0 KiB/s");
    }

    #[test]
    fn recent_throughput_mebibytes() {
        // 100 MiB in 10s = 10 MiB/s.
        let r = recent_row(100 * 1024 * 1024, 10_000, true);
        assert_eq!(format_recent_throughput(&r), "10.0 MiB/s");
    }

    #[test]
    fn recent_throughput_bytes_tier_for_slow_transfers() {
        // 512 bytes in 1s = 512 B/s — below KiB cutoff.
        let r = recent_row(512, 1000, true);
        assert_eq!(format_recent_throughput(&r), "512 B/s");
    }

    #[test]
    fn module_path_handles_each_empty_combination() {
        assert_eq!(module_path("", ""), "/");
        assert_eq!(module_path("", "p"), "p");
        assert_eq!(module_path("mod", ""), "mod");
        assert_eq!(module_path("mod", "sub/dir"), "mod/sub/dir");
    }

    // d-14: F2 active-row age column.

    #[test]
    fn format_age_milliseconds() {
        assert_eq!(format_age_from_unix_ms(1_000_500, 1_000_000), "500ms");
    }

    #[test]
    fn format_age_seconds() {
        assert_eq!(format_age_from_unix_ms(1_005_000, 1_000_000), "5s");
        assert_eq!(format_age_from_unix_ms(1_060_000, 1_000_000), "1m");
    }

    #[test]
    fn format_age_minutes_and_hours() {
        // 2m → 120000ms past start
        assert_eq!(format_age_from_unix_ms(1_120_000, 1_000_000), "2m");
        // 1h → 3_600_000ms past start
        assert_eq!(format_age_from_unix_ms(4_600_000, 1_000_000), "1h");
    }

    // d-15: bytes-progress display.

    #[test]
    fn format_bytes_progress_omits_percent_when_total_unknown() {
        // bytes_total=0 means the daemon hasn't measured
        // the plan yet — show just the raw bytes.
        assert_eq!(format_bytes_progress(512, 0), "512 B");
        assert_eq!(format_bytes_progress(0, 0), "0 B");
    }

    #[test]
    fn format_bytes_progress_appends_percent_when_total_known() {
        assert_eq!(format_bytes_progress(0, 1000), "0 B · 0%");
        assert_eq!(format_bytes_progress(500, 1000), "500 B · 50%");
        assert_eq!(format_bytes_progress(1000, 1000), "1000 B · 100%");
    }

    #[test]
    fn format_bytes_progress_clamps_overflow_to_100() {
        // Daemon counter drift: completed > total.
        // Showing "120%" would confuse the operator more
        // than it'd inform.
        assert_eq!(format_bytes_progress(120, 100), "120 B · 100%");
    }

    #[test]
    fn format_bytes_progress_picks_correct_byte_unit() {
        assert_eq!(
            format_bytes_progress(1024 * 1024, 4 * 1024 * 1024),
            "1.00 MiB · 25%"
        );
    }

    #[test]
    fn format_age_returns_dash_for_garbage_inputs() {
        // Zero now → can't compute against an absolute
        // wall-clock anchor (renderer captures zero only
        // if SystemTime::now() fails, which is itself
        // already a panic-avoidance fallback).
        assert_eq!(format_age_from_unix_ms(0, 1_000_000), "-");
        // Zero start → daemon would only send this on a
        // garbage wire payload.
        assert_eq!(format_age_from_unix_ms(1_000_000, 0), "-");
        // Clock skew: TUI host clock is BEHIND the
        // daemon's. Showing a negative-age placeholder is
        // safer than wrapping around an unsigned subtract.
        assert_eq!(format_age_from_unix_ms(1_000_000, 1_000_500), "-");
    }
}
