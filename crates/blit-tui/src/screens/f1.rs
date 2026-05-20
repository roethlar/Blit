//! F1 Daemons screen. Header / daemons table / detail
//! block / footer. Renderer is pure — takes a
//! [`DaemonsState`] reference and emits widgets.
//!
//! The discovery loop in `main.rs` owns the mDNS rescan
//! cadence; this module just paints whatever the state says
//! is current. Detail pane is rendered ONLY when a row is
//! selected (matches the design's "appears on row select"
//! behavior — the layout reserves space so the screen
//! doesn't jitter when an operator scrolls onto / off the
//! list).
//!
//! Layout (heights are constraints):
//!
//! ┌── header (1 line) ───────────────────────────────┐
//! │ blit-tui · F1 Daemons · N daemons                │
//! ├── daemons table (Min 5) ─────────────────────────┤
//! │ name  addr  port  ver  modules  deleg            │
//! │ ...                                              │
//! ├── selected detail (Length 5) ────────────────────┤
//! │ mycroft · 192.168.1.10:9031 · v0.1.0             │
//! │ modules: home (12 GiB / 16 GiB), media, backups  │
//! │ delegation: yes               [d-54: df capacity]│
//! ├── footer (1 line) ───────────────────────────────┤
//! │ status · q quit · r refresh · ↑↓ select          │
//! └──────────────────────────────────────────────────┘

use crate::daemons::{DaemonDetail, DaemonRow, DaemonsState, DiscoveryStatus};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::time::Instant;

/// Render the F1 pane into a caller-supplied area. The
/// router (a1-6) calls this with `body_area` from
/// `screens::split_for_tabs` so the tab strip lives above.
/// d-58: renderer-facing snapshot of the F1 trigger-transfer
/// modal. Bridged from `f1trigger::F1TriggerState` by `main.rs`
/// so the screens layer doesn't reach into the modal's internals.
pub struct TriggerPrompt {
    pub source: String,
    pub dest: String,
    /// `true` when the source field has focus (else dest).
    pub source_focused: bool,
}

pub fn render_into(
    frame: &mut Frame,
    area: Rect,
    state: &DaemonsState,
    now: Instant,
    // d-58: `Some` while the `t` trigger modal is open.
    trigger: Option<TriggerPrompt>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], state);
    render_table(frame, chunks[1], state);
    render_detail(frame, chunks[2], state, now);
    // d-58: the trigger modal, when open, replaces the discovery
    // footer line with the source/dest entry prompt.
    match trigger {
        Some(prompt) => render_trigger(frame, chunks[3], &prompt),
        None => render_footer(frame, chunks[3], state.status(), now),
    }
}

/// d-58: render the trigger-transfer entry prompt on the footer
/// line. The focused field's value is shown with a trailing
/// cursor (`_`) and bold; the hint reminds Tab/Enter/Esc.
fn render_trigger(frame: &mut Frame, area: Rect, prompt: &TriggerPrompt) {
    let focused = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let unfocused = Style::default().fg(Color::DarkGray);
    let (src_style, dst_style) = if prompt.source_focused {
        (focused, unfocused)
    } else {
        (unfocused, focused)
    };
    let src = if prompt.source_focused {
        format!("{}_", prompt.source)
    } else {
        prompt.source.clone()
    };
    let dst = if prompt.source_focused {
        prompt.dest.clone()
    } else {
        format!("{}_", prompt.dest)
    };
    let line = Line::from(vec![
        Span::styled("trigger ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("src: ", Style::default().fg(Color::DarkGray)),
        Span::styled(src, src_style),
        Span::styled("  dst: ", Style::default().fg(Color::DarkGray)),
        Span::styled(dst, dst_style),
        Span::styled(
            "   (Tab switch · Enter pull · Esc cancel)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_header(frame: &mut Frame, area: Rect, state: &DaemonsState) {
    let title = format!(
        " blit-tui · F1 Daemons · {} discovered ",
        state.rows().len()
    );
    let para = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(para, area);
}

fn render_table(frame: &mut Frame, area: Rect, state: &DaemonsState) {
    let rows: Vec<Row> = state.rows().iter().map(daemon_to_row).collect();
    let widths = [
        Constraint::Length(20),
        Constraint::Length(18),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
    ];
    let header = Row::new(vec![
        Cell::from("name"),
        Cell::from("address"),
        Cell::from("port"),
        Cell::from("version"),
        Cell::from("modules"),
        Cell::from("delegation"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Daemons "))
        // Stateful render: TableState carries the selected
        // index AND maintains an offset for auto-scrolling
        // so the highlighted row stays in view even when
        // the daemon count exceeds the viewport height.
        .row_highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let mut table_state = TableState::default().with_selected(Some(state.selected_index()));
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_detail(frame: &mut Frame, area: Rect, state: &DaemonsState, now: Instant) {
    let block = Block::default().borders(Borders::ALL).title(" Selected ");
    let lines: Vec<Line> = match state.selected_row() {
        Some(row) => {
            let detail = state.detail_for(&row.instance_name);
            detail_lines(row, detail, now)
        }
        None => vec![Line::from(Span::styled(
            "(no daemon selected — waiting for discovery)",
            Style::default().fg(Color::DarkGray),
        ))],
    };
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_footer(frame: &mut Frame, area: Rect, status: &DiscoveryStatus, now: Instant) {
    let status_span = match status {
        DiscoveryStatus::Scanning => {
            Span::styled("scanning...", Style::default().fg(Color::Yellow))
        }
        DiscoveryStatus::Live { last_scan_at } => Span::styled(
            format!("live · last scan {}", format_since(now, *last_scan_at)),
            Style::default().fg(Color::Green),
        ),
        DiscoveryStatus::Degraded { message } => Span::styled(
            format!("degraded: {message}"),
            Style::default().fg(Color::Red),
        ),
    };
    let line = Line::from(vec![
        status_span,
        Span::raw("  ·  "),
        Span::styled("q/Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit  ·  "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" refresh  ·  "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" select"),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn daemon_to_row(row: &DaemonRow) -> Row<'static> {
    if row.is_local() {
        // Local endpoint columns hold placeholder values
        // because there's no advertised address/port/version
        // for "this machine." A follow-up slice
        // (a1-3b-f1-getstate-detail) will populate the
        // modules cell from a loopback `GetState` query
        // when a local daemon is running.
        Row::new(vec![
            Cell::from(row.instance_name.clone()),
            Cell::from("(this machine)"),
            Cell::from("—"),
            Cell::from("—"),
            Cell::from("—"),
            Cell::from("—"),
        ])
    } else {
        Row::new(vec![
            Cell::from(row.instance_name.clone()),
            Cell::from(format_address(row)),
            Cell::from(row.port.to_string()),
            Cell::from(row.version.clone().unwrap_or_else(|| "?".to_string())),
            Cell::from(format_module_count(row)),
            Cell::from(format_delegation(row.delegation_enabled)),
        ])
    }
}

fn detail_lines(
    row: &DaemonRow,
    detail: Option<&DaemonDetail>,
    now: Instant,
) -> Vec<Line<'static>> {
    if row.is_local() {
        return local_detail_lines(row, detail, now);
    }
    // Remote header line stays mDNS-driven (it's the row's
    // identity); GetState data populates the body lines.
    let header = format!(
        "{} · {}:{} · {}",
        row.instance_name,
        format_address(row),
        row.port,
        row.version.clone().unwrap_or_else(|| "?".to_string()),
    );
    let mut lines = vec![Line::from(Span::styled(
        header,
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    lines.extend(detail_body_for_remote(row, detail, now));
    lines
}

/// Body lines for a remote row's detail block, populated
/// from `GetState`. Falls back to the mDNS-only modules /
/// delegation lines when the fetch hasn't returned yet.
fn detail_body_for_remote(
    row: &DaemonRow,
    detail: Option<&DaemonDetail>,
    now: Instant,
) -> Vec<Line<'static>> {
    match detail {
        Some(DaemonDetail::Loaded {
            state,
            capacities,
            fetched_at,
        }) => {
            let counters_line = format!(
                "active: {} · recent: {} · push: {} · pull: {} · errors: {}",
                state.active.len(),
                state.recent.len(),
                state
                    .counters
                    .as_ref()
                    .map(|c| c.push_operations_total)
                    .unwrap_or(0),
                state
                    .counters
                    .as_ref()
                    .map(|c| c.pull_operations_total)
                    .unwrap_or(0),
                state
                    .counters
                    .as_ref()
                    .map(|c| c.transfer_errors_total)
                    .unwrap_or(0),
            );
            let modules_line = if state.modules.is_empty() && row.modules.is_empty() {
                "modules: (none)".to_string()
            } else if state.modules.is_empty() {
                // Some daemons might return empty modules in
                // GetState (e.g., perms) — fall back to the
                // mDNS-advertised names.
                format!("modules: {} (from mDNS)", row.modules.join(", "))
            } else {
                // d-54: annotate each module with its capacity
                // (used / total) when the df fan-out got it;
                // bare name otherwise.
                let parts: Vec<String> = state
                    .modules
                    .iter()
                    .map(
                        |m| match capacities.iter().find(|(name, _, _)| name == &m.name) {
                            Some((_, used, total)) if *total > 0 => format!(
                                "{} ({} / {})",
                                m.name,
                                format_bytes(*used),
                                format_bytes(*total)
                            ),
                            _ => m.name.clone(),
                        },
                    )
                    .collect();
                format!("modules: {}", parts.join(", "))
            };
            let uptime_line = format!(
                "uptime: {} · as of {}",
                format_uptime(state.uptime_seconds),
                format_since(now, *fetched_at),
            );
            vec![
                Line::from(counters_line),
                Line::from(modules_line),
                Line::from(uptime_line),
            ]
        }
        Some(DaemonDetail::Pending) => vec![
            Line::from(Span::styled(
                "fetching GetState...",
                Style::default().fg(Color::Yellow),
            )),
            // Keep the mDNS-only lines visible while the
            // fetch is in flight — the operator still gets
            // useful info on a slow daemon.
            Line::from(mdns_modules_line(row)),
            Line::from(mdns_delegation_line(row)),
        ],
        Some(DaemonDetail::Error { message }) => vec![
            Line::from(Span::styled(
                format!("GetState failed: {message}"),
                Style::default().fg(Color::Red),
            )),
            Line::from(mdns_modules_line(row)),
            Line::from(mdns_delegation_line(row)),
        ],
        None => vec![
            // Before any fetch has been attempted, show the
            // mDNS-only body (matches the a1-3 round-1
            // contract, just without the live counters).
            Line::from(mdns_modules_line(row)),
            Line::from(mdns_delegation_line(row)),
            Line::from(Span::styled(
                "GetState not fetched yet",
                Style::default().fg(Color::DarkGray),
            )),
        ],
    }
}

fn mdns_modules_line(row: &DaemonRow) -> String {
    if row.modules.is_empty() && row.module_count.is_none() {
        "modules: (daemon does not advertise)".to_string()
    } else if row.modules.is_empty() {
        format!(
            "modules: {} (names not advertised)",
            row.module_count
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".to_string())
        )
    } else {
        format!("modules: {}", row.modules.join(", "))
    }
}

fn mdns_delegation_line(row: &DaemonRow) -> String {
    format!(
        "delegation: {}",
        match row.delegation_enabled {
            Some(true) => "enabled",
            Some(false) => "disabled",
            None => "unknown (pre-§3.2 daemon)",
        }
    )
}

/// Detail block for the synthetic Local row. The mDNS row
/// doesn't carry useful identity (no advertised address /
/// version), so the rendering is GetState-or-bust:
///
/// - `Loaded`: show "live · vX · uptime · counters"
/// - `Pending`: "fetching GetState from loopback..."
/// - `Error` / `None`: "no local daemon detected (start
///   `blit-daemon` on this host to enable transfers)".
fn local_detail_lines(
    row: &DaemonRow,
    detail: Option<&DaemonDetail>,
    now: Instant,
) -> Vec<Line<'static>> {
    let header = format!("{} · this machine", row.instance_name);
    let body = match detail {
        Some(DaemonDetail::Loaded {
            state, fetched_at, ..
        }) => {
            let live = format!(
                "local daemon detected · v{} · uptime {}",
                state.version,
                format_uptime(state.uptime_seconds),
            );
            let counters = format!(
                "active: {} · recent: {} · push: {} · pull: {} · errors: {}",
                state.active.len(),
                state.recent.len(),
                state
                    .counters
                    .as_ref()
                    .map(|c| c.push_operations_total)
                    .unwrap_or(0),
                state
                    .counters
                    .as_ref()
                    .map(|c| c.pull_operations_total)
                    .unwrap_or(0),
                state
                    .counters
                    .as_ref()
                    .map(|c| c.transfer_errors_total)
                    .unwrap_or(0),
            );
            vec![
                Line::from(Span::styled(live, Style::default().fg(Color::Green))),
                Line::from(counters),
                Line::from(format!("as of {}", format_since(now, *fetched_at))),
            ]
        }
        Some(DaemonDetail::Pending) => vec![
            Line::from(Span::styled(
                "fetching GetState from 127.0.0.1:9031...",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(""),
        ],
        Some(DaemonDetail::Error { message }) => vec![
            Line::from(Span::styled(
                "no local daemon detected",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(format!("(127.0.0.1:9031 — {message})")),
            Line::from(Span::styled(
                "start `blit-daemon` on this host to enable Local-endpoint transfers.",
                Style::default().fg(Color::DarkGray),
            )),
        ],
        None => vec![
            Line::from(Span::styled(
                "GetState not yet attempted",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(""),
        ],
    };
    let mut lines = vec![Line::from(Span::styled(
        header,
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    lines.extend(body);
    lines
}

/// Format an uptime in seconds as e.g. "3d 4h 12m" or
/// "1m 32s" — readable at glance, doesn't try to be
/// millisecond-exact.
/// d-54: byte formatter for the per-module capacity line. Same
/// IEC tiers as the F2/F4 formatters (d-25); duplicated per the
/// existing per-screen convention.
fn format_bytes(n: u64) -> String {
    if n >= 1 << 40 {
        format!("{:.2} TiB", n as f64 / (1u64 << 40) as f64)
    } else if n >= 1 << 30 {
        format!("{:.2} GiB", n as f64 / (1u64 << 30) as f64)
    } else if n >= 1 << 20 {
        format!("{:.2} MiB", n as f64 / (1u64 << 20) as f64)
    } else if n >= 1 << 10 {
        format!("{:.2} KiB", n as f64 / (1u64 << 10) as f64)
    } else {
        format!("{n} B")
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else if mins > 0 {
        format!("{mins}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// Renders the first address as `a.b.c.d`, or `n.n.n.n+N`
/// when the daemon advertises multiple. Trades precision
/// for column width — the detail pane shows the full list
/// if we ever need it (we don't today).
fn format_address(row: &DaemonRow) -> String {
    match row.addresses.as_slice() {
        [] => "(no addr)".to_string(),
        [one] => one.to_string(),
        [first, rest @ ..] => format!("{}+{}", first, rest.len()),
    }
}

fn format_module_count(row: &DaemonRow) -> String {
    match row.module_count {
        Some(n) => n.to_string(),
        None => "?".to_string(),
    }
}

fn format_delegation(enabled: Option<bool>) -> String {
    match enabled {
        Some(true) => "yes".to_string(),
        Some(false) => "no".to_string(),
        None => "?".to_string(),
    }
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
    use crate::daemons::DaemonRow;
    use std::net::Ipv4Addr;

    fn row(name: &str) -> DaemonRow {
        DaemonRow {
            kind: crate::daemons::EndpointKind::Remote,
            instance_name: name.to_string(),
            addresses: vec![Ipv4Addr::new(192, 168, 1, 10)],
            port: 9031,
            module_count: Some(3),
            delegation_enabled: Some(true),
            version: Some("0.1.0".to_string()),
            modules: vec!["home".to_string(), "media".to_string()],
        }
    }

    fn local_row() -> DaemonRow {
        // Mirror the daemons::DaemonRow::local() shape for
        // test consumption. Tests here don't reach into
        // DaemonRow::local directly because it's private to
        // the daemons module.
        DaemonRow {
            kind: crate::daemons::EndpointKind::Local,
            instance_name: crate::daemons::LOCAL_INSTANCE_NAME.to_string(),
            addresses: Vec::new(),
            port: 0,
            module_count: None,
            delegation_enabled: None,
            version: None,
            modules: Vec::new(),
        }
    }

    #[test]
    fn format_address_handles_zero_one_many() {
        let mut r = row("a");
        r.addresses.clear();
        assert_eq!(format_address(&r), "(no addr)");

        r.addresses = vec![Ipv4Addr::new(10, 0, 0, 1)];
        assert_eq!(format_address(&r), "10.0.0.1");

        r.addresses = vec![
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(10, 0, 0, 2),
            Ipv4Addr::new(10, 0, 0, 3),
        ];
        assert_eq!(format_address(&r), "10.0.0.1+2");
    }

    #[test]
    fn format_module_count_distinguishes_zero_from_unknown() {
        let mut r = row("a");
        r.module_count = Some(0);
        assert_eq!(format_module_count(&r), "0");
        r.module_count = None;
        assert_eq!(format_module_count(&r), "?");
    }

    #[test]
    fn format_delegation_renders_three_states() {
        assert_eq!(format_delegation(Some(true)), "yes");
        assert_eq!(format_delegation(Some(false)), "no");
        assert_eq!(format_delegation(None), "?");
    }

    #[test]
    fn format_since_picks_correct_unit() {
        let then = Instant::now();
        // Same instant → 0s.
        assert_eq!(format_since(then, then), "0s ago");
        // Saturating sub when "now" precedes "then" (shouldn't
        // happen in practice but the helper must not panic).
        assert_eq!(
            format_since(then, then + std::time::Duration::from_secs(5)),
            "0s ago"
        );
    }

    fn now_for_tests() -> Instant {
        Instant::now()
    }

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn detail_lines_label_unknown_delegation_for_pre_3_2_daemon() {
        let mut r = row("legacy");
        r.delegation_enabled = None;
        r.module_count = None;
        r.modules.clear();
        r.version = None;
        let lines = detail_lines(&r, None, now_for_tests());
        // 4 lines now: header, modules, delegation,
        // GetState-not-fetched-yet hint.
        assert_eq!(lines.len(), 4);
        let line_text: Vec<String> = lines.iter().map(line_text).collect();
        assert!(line_text[0].contains("legacy"));
        assert!(line_text[0].contains("?"));
        assert!(line_text[1].contains("does not advertise"));
        assert!(line_text[2].contains("pre-§3.2"));
        assert!(line_text[3].contains("GetState"));
    }

    #[test]
    fn detail_lines_shows_advertised_module_names() {
        let r = row("mycroft");
        let lines = detail_lines(&r, None, now_for_tests());
        // mDNS-only path: header, modules, delegation, hint.
        let modules_line = line_text(&lines[1]);
        assert!(modules_line.contains("home"));
        assert!(modules_line.contains("media"));
    }

    #[test]
    fn detail_lines_falls_back_to_module_count_when_names_truncated() {
        let mut r = row("dense");
        r.modules.clear();
        r.module_count = Some(40);
        let lines = detail_lines(&r, None, now_for_tests());
        let modules_line = line_text(&lines[1]);
        assert!(modules_line.contains("40"));
        assert!(modules_line.contains("not advertised"));
    }

    /// a1-3 round 2: the Local row renders distinctly from a
    /// remote daemon — placeholder columns, custom detail
    /// block. This pins the contract that downstream slices
    /// (F2/F3 routing) can identify the Local endpoint
    /// visually.
    #[test]
    fn detail_lines_for_local_row_uses_local_specific_copy() {
        let r = local_row();
        let lines = detail_lines(&r, None, now_for_tests());
        let header = line_text(&lines[0]);
        assert!(header.contains("this machine"));
        // First body line surfaces the "GetState not yet
        // attempted" hint (a1-3b shape).
        let body = line_text(&lines[1]);
        assert!(body.contains("GetState"));
    }

    /// Local row in the table uses placeholders for the
    /// columns that don't apply ("this machine" address,
    /// "—" elsewhere) so the column layout doesn't shift.
    #[test]
    fn daemon_to_row_for_local_uses_placeholders() {
        let row = daemon_to_row(&local_row());
        // Pull each cell's content (column 0..5) by inspecting
        // ratatui's Row debug repr; since we can't introspect
        // cell text directly, render to a TestBackend instead.
        // Wide terminal so the placeholder isn't truncated.
        // Real F1 layout uses fixed-width columns; this test
        // is only asserting that the placeholder copy reaches
        // the rendered buffer for the Local row.
        let backend = ratatui::backend::TestBackend::new(160, 4);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = ratatui::layout::Rect::new(0, 0, 160, 3);
                let widths = [ratatui::layout::Constraint::Length(24); 6];
                let table = ratatui::widgets::Table::new(vec![row], widths);
                frame.render_widget(table, area);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(
            rendered.contains("(this machine)"),
            "expected '(this machine)' in:\n{rendered}"
        );
    }

    /// a1-3 round 2: when the daemon list is taller than
    /// the table viewport, the highlighted row must stay
    /// visible. TableState auto-scrolls so that even row 15
    /// of 20 appears in a 6-line viewport.
    #[test]
    fn selected_row_stays_visible_when_list_exceeds_viewport() {
        use crate::daemons::DaemonsState;
        use blit_core::mdns::MdnsDiscoveredService;
        use std::collections::HashMap;

        let services: Vec<MdnsDiscoveredService> = (0..20)
            .map(|i| MdnsDiscoveredService {
                fullname: format!("daemon{i:02}._blit._tcp.local."),
                instance_name: format!("daemon{i:02}"),
                hostname: format!("daemon{i:02}.local."),
                port: 9031,
                addresses: vec![Ipv4Addr::new(10, 0, 0, i as u8 + 1)],
                properties: HashMap::new(),
            })
            .collect();
        let mut state = DaemonsState::new();
        state.replace_from_discovery(&services, Instant::now());
        // Move cursor to a row that would be off the first
        // page (Local + daemon00..daemon14 → index 15 is
        // daemon14).
        for _ in 0..15 {
            state.select_next();
        }
        let target = state.selected_row().unwrap().instance_name.clone();
        assert_eq!(target, "daemon14");

        // Render into a small TestBackend and verify the
        // selected daemon's name is somewhere in the buffer.
        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let now = Instant::now();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_into(frame, area, &state, now, None);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(
            rendered.contains(&target),
            "expected selected row '{target}' to be visible in viewport, got:\n{rendered}"
        );
    }

    /// a1-3b: Remote row with a `Loaded` GetState shows
    /// live counters in the detail block.
    #[test]
    fn detail_lines_for_remote_loaded_shows_counters() {
        use blit_core::generated::{Counters, DaemonState};
        let r = row("mycroft");
        let state = DaemonState {
            version: "0.2.0".to_string(),
            uptime_seconds: 90_000, // 1d 1h
            counters: Some(Counters {
                push_operations_total: 12,
                pull_operations_total: 4,
                purge_operations_total: 1,
                active_transfers: 1,
                transfer_errors_total: 0,
            }),
            ..DaemonState::default()
        };
        let detail = DaemonDetail::Loaded {
            state: Box::new(state),
            capacities: Vec::new(),
            fetched_at: Instant::now(),
        };
        let lines = detail_lines(&r, Some(&detail), Instant::now());
        // Header + counters + modules + uptime.
        assert_eq!(lines.len(), 4);
        let counters = line_text(&lines[1]);
        assert!(counters.contains("push: 12"));
        assert!(counters.contains("pull: 4"));
        assert!(counters.contains("errors: 0"));
        let uptime = line_text(&lines[3]);
        assert!(uptime.contains("uptime"));
        assert!(uptime.contains("1d"));
    }

    /// d-54: the modules line annotates each module with its
    /// capacity (used / total) when the df fan-out provided it,
    /// and falls back to the bare name when it didn't.
    #[test]
    fn detail_lines_annotate_module_capacity_with_fallback() {
        use blit_core::generated::{DaemonState, ModuleInfo};
        let r = row("mycroft");
        let module = |n: &str| ModuleInfo {
            name: n.to_string(),
            path: format!("/srv/{n}"),
            read_only: false,
        };
        let state = DaemonState {
            modules: vec![module("home"), module("media")],
            ..DaemonState::default()
        };
        let detail = DaemonDetail::Loaded {
            state: Box::new(state),
            // `home` has capacity; `media` does not (df failed).
            capacities: vec![("home".to_string(), 12u64 << 30, 16u64 << 30)],
            fetched_at: Instant::now(),
        };
        let lines = detail_lines(&r, Some(&detail), Instant::now());
        let modules = line_text(&lines[2]);
        assert!(
            modules.contains("home (12.00 GiB / 16.00 GiB)"),
            "home shows capacity: {modules}"
        );
        assert!(
            modules.contains("media") && !modules.contains("media ("),
            "media falls back to bare name: {modules}"
        );
    }

    /// a1-3b: Pending state shows a "fetching..." spinner
    /// line and keeps the mDNS-only body lines visible
    /// (operator isn't dropped into a blank pane while a
    /// fetch is in flight).
    #[test]
    fn detail_lines_for_remote_pending_shows_spinner_and_mdns_fallback() {
        let r = row("mycroft");
        let detail = DaemonDetail::Pending;
        let lines = detail_lines(&r, Some(&detail), Instant::now());
        assert_eq!(lines.len(), 4);
        let spinner = line_text(&lines[1]);
        assert!(spinner.contains("fetching"));
        let modules = line_text(&lines[2]);
        assert!(modules.contains("home")); // mDNS fallback
    }

    /// a1-3b: Error state surfaces the failure message in
    /// red and keeps the mDNS fallback body lines.
    #[test]
    fn detail_lines_for_remote_error_shows_message_and_fallback() {
        let r = row("mycroft");
        let detail = DaemonDetail::Error {
            message: "connect refused".to_string(),
        };
        let lines = detail_lines(&r, Some(&detail), Instant::now());
        let err = line_text(&lines[1]);
        assert!(err.contains("GetState failed"));
        assert!(err.contains("connect refused"));
    }

    /// a1-3b: Local row with Loaded shows the "local daemon
    /// detected" header + counters.
    #[test]
    fn detail_lines_for_local_loaded_shows_live() {
        use blit_core::generated::{Counters, DaemonState};
        let r = local_row();
        let state = DaemonState {
            version: "0.2.0".to_string(),
            uptime_seconds: 60,
            counters: Some(Counters {
                push_operations_total: 0,
                pull_operations_total: 1,
                purge_operations_total: 0,
                active_transfers: 0,
                transfer_errors_total: 0,
            }),
            ..DaemonState::default()
        };
        let detail = DaemonDetail::Loaded {
            state: Box::new(state),
            capacities: Vec::new(),
            fetched_at: Instant::now(),
        };
        let lines = detail_lines(&r, Some(&detail), Instant::now());
        let live = line_text(&lines[1]);
        assert!(live.contains("local daemon detected"));
        assert!(live.contains("0.2.0"));
        let counters = line_text(&lines[2]);
        assert!(counters.contains("pull: 1"));
    }

    /// a1-3b: Local row with Error shows the "no local
    /// daemon detected" message with a hint to start
    /// blit-daemon.
    #[test]
    fn detail_lines_for_local_error_shows_no_daemon_hint() {
        let r = local_row();
        let detail = DaemonDetail::Error {
            message: "connection refused".to_string(),
        };
        let lines = detail_lines(&r, Some(&detail), Instant::now());
        let hint = line_text(&lines[1]);
        assert!(hint.contains("no local daemon"));
        let footer = line_text(&lines[3]);
        assert!(footer.contains("blit-daemon"));
    }

    /// `format_uptime` covers each unit threshold.
    #[test]
    fn format_uptime_picks_correct_unit() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(42), "42s");
        assert_eq!(format_uptime(60), "1m 0s");
        assert_eq!(format_uptime(3700), "1h 1m");
        assert_eq!(format_uptime(90_061), "1d 1h 1m");
    }
}
