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
//! │ modules: home, media, backups                    │
//! │ delegation: yes                                  │
//! ├── footer (1 line) ───────────────────────────────┤
//! │ status · q quit · r refresh · ↑↓ select          │
//! └──────────────────────────────────────────────────┘

use crate::daemons::{DaemonRow, DaemonsState, DiscoveryStatus};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::time::Instant;

/// Top-level F1 render entry. Same shape as f2::render.
pub fn render(frame: &mut Frame, state: &DaemonsState, now: Instant) {
    let area = frame.area();
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
    render_detail(frame, chunks[2], state);
    render_footer(frame, chunks[3], state.status(), now);
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

fn render_detail(frame: &mut Frame, area: Rect, state: &DaemonsState) {
    let block = Block::default().borders(Borders::ALL).title(" Selected ");
    let lines: Vec<Line> = match state.selected_row() {
        Some(row) => detail_lines(row),
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

fn detail_lines(row: &DaemonRow) -> Vec<Line<'static>> {
    if row.is_local() {
        return local_detail_lines(row);
    }
    let header = format!(
        "{} · {}:{} · {}",
        row.instance_name,
        format_address(row),
        row.port,
        row.version.clone().unwrap_or_else(|| "?".to_string()),
    );
    let modules_line = if row.modules.is_empty() && row.module_count.is_none() {
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
    };
    let delegation_line = format!(
        "delegation: {}",
        match row.delegation_enabled {
            Some(true) => "enabled",
            Some(false) => "disabled",
            None => "unknown (pre-§3.2 daemon)",
        }
    );
    vec![
        Line::from(Span::styled(
            header,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(modules_line),
        Line::from(delegation_line),
    ]
}

/// Detail block for the synthetic Local row. Distinct copy
/// from `detail_lines` because the Local endpoint doesn't
/// have an advertised address/port/version (it's "this
/// machine," not a daemon advertising itself over mDNS).
///
/// A follow-up slice (`a1-3b-f1-getstate-detail`) will
/// upgrade this block with `GetState`-driven counters when
/// a daemon is running on the loopback interface.
fn local_detail_lines(row: &DaemonRow) -> Vec<Line<'static>> {
    let header = format!("{} · this machine (no mDNS advertise)", row.instance_name);
    vec![
        Line::from(Span::styled(
            header,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("modules: (pending GetState integration — a1-3b)"),
        Line::from(Span::styled(
            "Local endpoint — F2/F3 routing slices will treat this symmetrically with remote daemons.",
            Style::default().fg(Color::DarkGray),
        )),
    ]
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

    #[test]
    fn detail_lines_label_unknown_delegation_for_pre_3_2_daemon() {
        let mut r = row("legacy");
        r.delegation_enabled = None;
        r.module_count = None;
        r.modules.clear();
        r.version = None;
        let lines = detail_lines(&r);
        // Three lines: header, modules, delegation.
        assert_eq!(lines.len(), 3);
        // Joining all spans of each line to a string for the
        // assertion. Span styling doesn't matter here.
        let line_text: Vec<String> = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        assert!(line_text[0].contains("legacy"));
        assert!(line_text[0].contains("?"));
        assert!(line_text[1].contains("does not advertise"));
        assert!(line_text[2].contains("pre-§3.2"));
    }

    #[test]
    fn detail_lines_shows_advertised_module_names() {
        let r = row("mycroft");
        let lines = detail_lines(&r);
        let modules_line: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(modules_line.contains("home"));
        assert!(modules_line.contains("media"));
    }

    #[test]
    fn detail_lines_falls_back_to_module_count_when_names_truncated() {
        let mut r = row("dense");
        r.modules.clear();
        r.module_count = Some(40);
        let lines = detail_lines(&r);
        let modules_line: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
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
        let lines = detail_lines(&r);
        let header: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(header.contains("this machine"));
        let modules: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(modules.contains("a1-3b"));
        let footer: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(footer.contains("Local endpoint"));
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
        terminal.draw(|frame| render(frame, &state, now)).unwrap();
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
}
