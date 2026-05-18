//! F3 Browse screen — header / tree-or-list table /
//! stats block / footer. Mirrors F1's layout shape so the
//! operator's eye finds the same regions.
//!
//! Renderer is pure: takes a [`BrowseState`] reference and
//! emits widgets. Navigation + RPC fetching live in
//! `main::run_f3_event_loop`.
//!
//! Layout:
//!
//! ```text
//! ┌── header (1 line) ───────────────────────────────┐
//! │ blit-tui · F3 Browse · <remote> · <breadcrumb>   │
//! ├── entries table (Min 5) ─────────────────────────┤
//! │ name  kind  size  mtime                          │
//! │ ...                                              │
//! ├── stats block (Length 3) ────────────────────────┤
//! │ Selected: photos/ · <kind> · <size>              │
//! │ View: <breadcrumb> · <N> entries                 │
//! ├── footer (1 line) ───────────────────────────────┤
//! │ status · q quit · r refresh · enter into · esc up│
//! └──────────────────────────────────────────────────┘
//! ```

use crate::browse::{BrowseFetchStatus, BrowseRow, BrowseRowKind, BrowseState, BrowseView};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::time::Instant;

/// Render the F3 pane into a caller-supplied area (router-aware).
pub fn render_into(
    frame: &mut Frame,
    area: Rect,
    state: &BrowseState,
    remote_label: &str,
    now: Instant,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], state, remote_label);
    render_table(frame, chunks[1], state);
    render_stats(frame, chunks[2], state);
    render_footer(frame, chunks[3], state.status(), now);
}

fn render_header(frame: &mut Frame, area: Rect, state: &BrowseState, remote_label: &str) {
    let title = format!(
        " blit-tui · F3 Browse · {} · {} ",
        remote_label,
        state.breadcrumb(),
    );
    let para = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(para, area);
}

fn render_table(frame: &mut Frame, area: Rect, state: &BrowseState) {
    let rows: Vec<Row> = state.rows().iter().map(row_to_table_row).collect();
    let widths = [
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(12),
    ];
    let header = Row::new(vec![
        Cell::from("name"),
        Cell::from("kind"),
        Cell::from("size"),
        Cell::from("mtime"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));
    let block_title = match state.view() {
        BrowseView::Modules => " Modules ".to_string(),
        BrowseView::Module { name, path } => {
            if path.is_empty() {
                format!(" {name} ")
            } else {
                format!(" {name}/{} ", path.join("/"))
            }
        }
    };
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(block_title))
        .row_highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let mut table_state = TableState::default().with_selected(Some(state.selected_index()));
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_stats(frame: &mut Frame, area: Rect, state: &BrowseState) {
    let block = Block::default().borders(Borders::ALL).title(" Stats ");
    let lines = match state.selected_row() {
        Some(row) => vec![
            Line::from(format!(
                "Selected: {} · {} · {}",
                row.name,
                kind_label(&row.kind),
                if matches!(row.kind, BrowseRowKind::File) {
                    format_bytes(row.size_bytes)
                } else {
                    "—".to_string()
                },
            )),
            Line::from(format!(
                "View: {} · {} entries",
                state.breadcrumb(),
                state.rows().len(),
            )),
        ],
        None => vec![Line::from(Span::styled(
            "(no entries)",
            Style::default().fg(Color::DarkGray),
        ))],
    };
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_footer(frame: &mut Frame, area: Rect, status: &BrowseFetchStatus, now: Instant) {
    let status_span = match status {
        BrowseFetchStatus::Idle => Span::styled("idle", Style::default().fg(Color::DarkGray)),
        BrowseFetchStatus::Pending => {
            Span::styled("fetching...", Style::default().fg(Color::Yellow))
        }
        BrowseFetchStatus::Loaded { fetched_at } => Span::styled(
            format!("loaded · {}", format_since(now, *fetched_at)),
            Style::default().fg(Color::Green),
        ),
        BrowseFetchStatus::Error { message } => {
            Span::styled(format!("error: {message}"), Style::default().fg(Color::Red))
        }
    };
    let line = Line::from(vec![
        status_span,
        Span::raw("  ·  "),
        Span::styled("q/Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit  ·  "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" refresh  ·  "),
        Span::styled("enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" into  ·  "),
        Span::styled("←", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" up"),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn row_to_table_row(row: &BrowseRow) -> Row<'static> {
    let kind = kind_label(&row.kind);
    let size = match &row.kind {
        BrowseRowKind::File => format_bytes(row.size_bytes),
        _ => "—".to_string(),
    };
    let mtime = if row.mtime_seconds > 0 {
        format_mtime(row.mtime_seconds)
    } else {
        "—".to_string()
    };
    Row::new(vec![
        Cell::from(row.name.clone()),
        Cell::from(kind.to_string()),
        Cell::from(size),
        Cell::from(mtime),
    ])
}

fn kind_label(kind: &BrowseRowKind) -> &'static str {
    match kind {
        BrowseRowKind::Module { read_only: true } => "module (ro)",
        BrowseRowKind::Module { read_only: false } => "module",
        BrowseRowKind::Directory => "dir",
        BrowseRowKind::File => "file",
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

fn format_mtime(secs: i64) -> String {
    // Mtime is wire seconds-since-epoch. We render it as a
    // short YYYY-MM-DD string without pulling in a date
    // crate: chrono isn't a workspace dep here. Approximate
    // — accurate enough for an at-a-glance browse column;
    // the operator who needs exact timestamps can `ls`
    // directly.
    if secs <= 0 {
        return "—".to_string();
    }
    // Days since epoch.
    let days = secs / 86_400;
    // Naive Gregorian calculation from days-since-1970-01-01.
    // Good enough for a browse column.
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn days_to_ymd(days_since_epoch: i64) -> (i32, u32, u32) {
    // Algorithm: Howard Hinnant's days_from_civil inverse,
    // adapted for i64 day count.
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (year, m as u32, d as u32)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_label_covers_each_variant() {
        assert_eq!(
            kind_label(&BrowseRowKind::Module { read_only: false }),
            "module"
        );
        assert_eq!(
            kind_label(&BrowseRowKind::Module { read_only: true }),
            "module (ro)"
        );
        assert_eq!(kind_label(&BrowseRowKind::Directory), "dir");
        assert_eq!(kind_label(&BrowseRowKind::File), "file");
    }

    #[test]
    fn format_bytes_picks_correct_unit() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
    }

    #[test]
    fn format_mtime_handles_zero_and_negative() {
        assert_eq!(format_mtime(0), "—");
        assert_eq!(format_mtime(-1), "—");
    }

    /// Sanity-check `days_to_ymd` against a known epoch
    /// date. 1970-01-01 is day 0; 2024-01-01 is 19723.
    #[test]
    fn days_to_ymd_matches_known_dates() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        assert_eq!(days_to_ymd(19_723), (2024, 1, 1));
    }
}
