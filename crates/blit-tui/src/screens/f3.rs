//! F3 Browse screen — header / tree-or-list table /
//! stats block / footer. Mirrors F1's layout shape so the
//! operator's eye finds the same regions.
//!
//! Renderer is pure: takes a [`BrowseState`] reference and
//! emits widgets. Navigation + RPC fetching live in
//! `main::run_f3_event_loop`.
//!
//! Layout (d-26 / d-28 / d-33 polish — `/` filter
//! fragment in footer, visible-only rows in the table,
//! differentiated empty-state message + pull-source
//! preview in Stats):
//!
//! ```text
//! ┌── header (1 line) ───────────────────────────────┐
//! │ blit-tui · F3 Browse · <remote> · <breadcrumb>   │
//! ├── entries table (Min 5) ─────────────────────────┤
//! │ name  kind  size  mtime                          │
//! │ ...   (rows filtered by d-26 filter when set)    │
//! ├── stats block (Length 5) ────────────────────────┤
//! │ Selected: photos/ · <kind> · <size>              │
//! │ View: <breadcrumb> · <V>/<N> entries (when       │
//! │       filtered) or <N> entries (no filter)       │
//! │ Pull: <host>:/<module>/<rel-path>   [d-33]       │
//! │ — or, when nothing is selectable —               │
//! │ (no entries) / (no rows match filter) [d-28]     │
//! ├── footer (1 line) ───────────────────────────────┤
//! │ status · [filter: foo │ filter: foo_] · q quit … │
//! └──────────────────────────────────────────────────┘
//! ```
//!
//! d-33 / d-34: the `Pull:` line surfaces the canonical
//! remote source spec for the selected row (derived via
//! `browse::pull_source_endpoint(...).display()`).
//!
//! d-35: `p` opens a destination prompt and runs a
//! remote→local PullSync owned by the TUI process. The
//! footer shows one of:
//! - `pull → <dest>_` (cyan, EnteringDest — typing)
//! - `pulling → <dest>... (N file(s) · X)` (yellow,
//!   Running — d-37 live byte counter once data flows)
//! - `pulled N file(s) · X → <dest>` (green, Done)
//! - `pull failed: <msg>` (red, Error)
//!
//! d-26's filter fragment renders one of:
//! - hidden (no filter, not editing)
//! - `filter: foo_` (cyan, while editing — trailing `_`
//!   marks the cursor position)
//! - `filter: foo` (green, applied + not editing)
//!
//! d-28: when the Stats block has nothing to highlight
//! (cursor sits on a hidden row or rows are empty), the
//! message distinguishes the two reasons:
//! - `(no rows match filter)` — rows loaded, filter
//!   excludes everything. Hint to relax the filter.
//! - `(no entries)` — empty rowset (pre-fetch or genuinely
//!   empty module/directory).

use crate::browse::{BrowseFetchStatus, BrowseRow, BrowseRowKind, BrowseState, BrowseView};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::time::Instant;

/// d-35: renderer-facing snapshot of the F3 pull
/// lifecycle. Bridged from `f3pull::F3PullStatus` by
/// `main.rs` so the screens layer doesn't reach into the
/// pull state machine's internals.
#[derive(Debug, Clone)]
pub enum F3PullDisplay {
    /// No pull fragment (Idle).
    Hidden,
    /// Destination prompt open; `dest` is what the
    /// operator has typed so far.
    EnteringDest { dest: String },
    /// PullSync in flight. d-37: live cumulative
    /// counters (0 until the first progress event).
    /// d-39: `bytes_per_sec` is average throughput
    /// (0 until ~1s elapsed).
    Running {
        dest: String,
        files: usize,
        bytes: u64,
        bytes_per_sec: u64,
    },
    /// Pull finished — files + bytes pulled, dest path.
    Done {
        files: usize,
        bytes: u64,
        dest: String,
    },
    /// Pull failed.
    Error { message: String },
}

/// Render the F3 pane into a caller-supplied area (router-aware).
///
/// d-33 / d-34: `pull_spec` is the pre-rendered canonical
/// pull-source spec for the cursor (the caller derives it
/// from `browse::pull_source_endpoint(...).display()` so
/// the screens layer stays free of `blit_core` types).
/// `None` when no remote is configured or nothing is
/// selectable.
pub fn render_into(
    frame: &mut Frame,
    area: Rect,
    state: &BrowseState,
    remote_label: &str,
    pull_spec: Option<&str>,
    pull: &F3PullDisplay,
    now: Instant,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            // d-33: Stats grew from 4→5 rows for the
            // "Pull:" preview line.
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], state, remote_label);
    render_table(frame, chunks[1], state);
    render_stats(frame, chunks[2], state, pull_spec);
    render_footer(frame, chunks[3], state, pull, now);
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
    // d-26: filter-aware — only rows that match the
    // current filter make it into the table. With an
    // empty filter `visible_indices()` returns all rows
    // so this is a no-op vs. pre-d-26 behavior.
    let visible_indices = state.visible_indices();
    let rows: Vec<Row> = visible_indices
        .iter()
        .map(|&i| row_to_table_row(&state.rows()[i]))
        .collect();
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
    // d-26: TableState's index addresses the visible-row
    // ordinal, not the underlying `state.rows()` index.
    // `visible_selected_position` maps the model cursor
    // back into the visible list.
    let mut table_state = TableState::default().with_selected(state.visible_selected_position());
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_stats(frame: &mut Frame, area: Rect, state: &BrowseState, pull_spec: Option<&str>) {
    let block = Block::default().borders(Borders::ALL).title(" Stats ");
    // d-26: when a filter is active, show "<V>/<N>
    // entries" so the operator can see how many rows the
    // filter is hiding.
    let total = state.rows().len();
    let visible = state.visible_indices().len();
    let count_fragment = if state.filter().is_empty() {
        format!("{total} entries")
    } else {
        format!("{visible}/{total} entries")
    };
    let lines = match state.selected_row() {
        Some(row) => {
            let mut lines = vec![
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
                Line::from(format!("View: {} · {}", state.breadcrumb(), count_fragment,)),
            ];
            // d-33 / d-34: canonical remote pull-source
            // spec for the cursor, pre-rendered by the
            // caller via `pull_source_endpoint().display()`.
            // Foundation for F3 transfer-from-cursor — the
            // destination prompt + pull execution land in
            // follow-on slices.
            if let Some(spec) = pull_spec {
                lines.push(Line::from(vec![
                    Span::styled("Pull: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(spec.to_string()),
                ]));
            }
            lines
        }
        None => vec![Line::from(Span::styled(
            // d-28: differentiated empty-state message —
            // `(no rows match filter)` when a non-empty
            // filter excludes every loaded row, `(no
            // entries)` otherwise.
            state.empty_state_message(),
            Style::default().fg(Color::DarkGray),
        ))],
    };
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_footer(
    frame: &mut Frame,
    area: Rect,
    state: &BrowseState,
    pull: &F3PullDisplay,
    now: Instant,
) {
    let status_span = match state.status() {
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
    // d-26: filter fragment sits between status and the
    // key hints. Hidden when the filter is empty AND not
    // editing — so just pressing `/` produces a visible
    // "filter: _" cursor even before the operator types.
    let mut spans: Vec<Span> = vec![status_span];
    if !state.filter().is_empty() || state.is_editing_filter() {
        spans.push(Span::raw("  ·  "));
        let fragment = if state.is_editing_filter() {
            format!("filter: {}_", state.filter())
        } else {
            format!("filter: {}", state.filter())
        };
        let color = if state.is_editing_filter() {
            Color::Cyan
        } else {
            Color::Green
        };
        spans.push(Span::styled(fragment, Style::default().fg(color)));
    }
    // d-35: pull fragment — prompt / progress / outcome.
    match pull {
        F3PullDisplay::Hidden => {}
        F3PullDisplay::EnteringDest { dest } => {
            spans.push(Span::raw("  ·  "));
            spans.push(Span::styled(
                format!("pull → {dest}_"),
                Style::default().fg(Color::Cyan),
            ));
        }
        F3PullDisplay::Running {
            dest,
            files,
            bytes,
            bytes_per_sec,
        } => {
            spans.push(Span::raw("  ·  "));
            // d-37: show the live count once bytes start
            // flowing; before that just "pulling →".
            // d-39: append throughput once the rate
            // settles (suppressed for the first ~1s).
            let frag = if *bytes > 0 || *files > 0 {
                let rate = if *bytes_per_sec > 0 {
                    format!(" · {}/s", format_bytes(*bytes_per_sec))
                } else {
                    String::new()
                };
                format!(
                    "pulling → {dest}... ({files} file(s) · {}{rate})",
                    format_bytes(*bytes)
                )
            } else {
                format!("pulling → {dest}...")
            };
            spans.push(Span::styled(frag, Style::default().fg(Color::Yellow)));
        }
        F3PullDisplay::Done { files, bytes, dest } => {
            spans.push(Span::raw("  ·  "));
            spans.push(Span::styled(
                format!("pulled {files} file(s) · {} → {dest}", format_bytes(*bytes)),
                Style::default().fg(Color::Green),
            ));
        }
        F3PullDisplay::Error { message } => {
            spans.push(Span::raw("  ·  "));
            spans.push(Span::styled(
                format!("pull failed: {message}"),
                Style::default().fg(Color::Red),
            ));
        }
    }
    // Tail: shared key hints. `/` joins the keymap since
    // d-26 made it bindable on F3; `p` since d-35.
    spans.extend([
        Span::raw("  ·  "),
        Span::styled("q/Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit  ·  "),
        Span::styled("enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" into  ·  "),
        Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" filter  ·  "),
        Span::styled("p", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" pull"),
    ]);
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
