//! F2 Transfers screen. Two stacked tables: active rows on
//! top, recent rows on the bottom. Footer shows remote +
//! connection status + key hints.
//!
//! Renderer is pure — takes a [`TransfersState`] reference
//! and a [`ConnectionStatus`] string and emits widgets. The
//! event loop in `main.rs` owns the state and the Subscribe
//! stream; this module just paints.
//!
//! Layout (heights are constraints):
//!
//! ┌── header (1 line) ─────────────────────────┐
//! │ blit-tui · F2 Transfers · <remote>         │
//! ├── active table (Min 5) ────────────────────┤
//! │ id  kind  peer  module/path  bytes  bps    │
//! │ ...                                        │
//! ├── recent table (Min 5) ────────────────────┤
//! │ id  kind  peer  module/path  duration  ok  │
//! │ ...                                        │
//! ├── footer (1 line) ─────────────────────────┤
//! │ status · q/Esc quit · r refresh            │
//! └────────────────────────────────────────────┘

use crate::state::{ActiveRow, RecentRow, TransfersState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
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
    render_active_table(frame, chunks[1], state);
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

fn render_active_table(frame: &mut Frame, area: Rect, state: &TransfersState) {
    let rows: Vec<Row> = state
        .active_rows()
        .into_iter()
        .map(active_row_to_table_row)
        .collect();
    let widths = [
        Constraint::Length(20),
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Min(20),
        Constraint::Length(12),
        Constraint::Length(12),
    ];
    let header = Row::new(vec![
        Cell::from("transfer_id"),
        Cell::from("kind"),
        Cell::from("peer"),
        Cell::from("module/path"),
        Cell::from("bytes"),
        Cell::from("throughput"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Active "));
    frame.render_widget(table, area);
}

fn render_recent_table(frame: &mut Frame, area: Rect, state: &TransfersState) {
    let rows: Vec<Row> = state.recent_rows().map(recent_row_to_table_row).collect();
    let widths = [
        Constraint::Length(20),
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Min(20),
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

fn active_row_to_table_row(row: &ActiveRow) -> Row<'static> {
    Row::new(vec![
        Cell::from(row.transfer_id.clone()),
        Cell::from(kind_label(row.kind).to_string()),
        Cell::from(row.peer.clone()),
        Cell::from(module_path(&row.module, &row.path)),
        Cell::from(format_bytes(row.bytes_completed)),
        Cell::from(if row.throughput_bps == 0 {
            "-".to_string()
        } else {
            format!("{}/s", format_bytes(row.throughput_bps))
        }),
    ])
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
    ])
    .style(status_style)
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

    #[test]
    fn module_path_handles_each_empty_combination() {
        assert_eq!(module_path("", ""), "/");
        assert_eq!(module_path("", "p"), "p");
        assert_eq!(module_path("mod", ""), "mod");
        assert_eq!(module_path("mod", "sub/dir"), "mod/sub/dir");
    }
}
