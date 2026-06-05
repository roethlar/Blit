//! Phase 6 dual-pane shell renderer.
//!
//! This is the M1 render shell: two browsable panes, path bars, mark
//! column, contextual help, and visible actions. Listing providers and
//! transfer execution land in later slices.

use crate::dual_pane::{BrowserEntry, DualPaneState, PaneFetchStatus, PaneId, PaneState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

pub fn render_into(frame: &mut Frame, area: Rect, state: &DualPaneState, accent: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    render_pane(
        frame,
        panes[0],
        state.pane(PaneId::Left),
        state.active() == PaneId::Left,
        accent,
    );
    render_pane(
        frame,
        panes[1],
        state.pane(PaneId::Right),
        state.active() == PaneId::Right,
        accent,
    );
    render_help(frame, chunks[1]);
    render_actions(frame, chunks[2], state, accent);
}

fn render_pane(frame: &mut Frame, area: Rect, pane: &PaneState, active: bool, accent: Color) {
    let title = format!(" {}: {} ", pane.id().label(), pane.location().display());
    let border_style = if active {
        Style::default().fg(accent)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    if pane.entries().is_empty() {
        let suffix = match pane.status() {
            PaneFetchStatus::Idle => "  (waiting to load)",
            PaneFetchStatus::Pending { .. } => "  (loading...)",
            PaneFetchStatus::Loaded => "  (no entries)",
            PaneFetchStatus::Error { message } => {
                let line = Line::from(vec![
                    Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(pane.path_editor().to_string()),
                    Span::styled(
                        format!("  error: {message}"),
                        Style::default().fg(Color::Red),
                    ),
                ]);
                frame.render_widget(Paragraph::new(line).block(block), area);
                return;
            }
        };
        let line = Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
            Span::raw(pane.path_editor().to_string()),
            Span::styled(suffix, Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(line).block(block), area);
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
            Span::raw(pane.path_editor().to_string()),
            filter_span(pane),
        ])),
        chunks[0],
    );

    let rows: Vec<Row> = pane
        .entries()
        .iter()
        .map(|entry| entry_to_row(entry, pane.is_marked(&entry.id)))
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Min(18),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from(""),
            Cell::from("name"),
            Cell::from("kind"),
            Cell::from("size"),
            Cell::from("mtime"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .row_highlight_style(
        Style::default()
            .fg(super::contrasting_fg(accent))
            .bg(accent)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">");
    let mut table_state = TableState::default().with_selected(Some(pane.cursor()));
    frame.render_stateful_widget(table, chunks[1], &mut table_state);
}

fn entry_to_row(entry: &BrowserEntry, marked: bool) -> Row<'static> {
    let mark = if marked { "[x]" } else { "[ ]" };
    let name = if entry.read_only {
        format!("{} (ro)", entry.name)
    } else {
        entry.name.clone()
    };
    Row::new(vec![
        Cell::from(mark.to_string()),
        Cell::from(name),
        Cell::from(entry.kind.label()),
        Cell::from(format_size(entry.size)),
        Cell::from(format_mtime(entry.mtime_seconds)),
    ])
}

fn render_help(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" switch pane  "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" open  "),
            Span::styled("Space", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" mark  "),
            Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" search  "),
            Span::styled("?", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" help"),
        ])),
        area,
    );
}

fn filter_span(pane: &PaneState) -> Span<'static> {
    if pane.filter().is_empty() {
        Span::raw("")
    } else {
        Span::styled(
            format!("  filter: {}", pane.filter()),
            Style::default().fg(Color::Green),
        )
    }
}

fn render_actions(frame: &mut Frame, area: Rect, state: &DualPaneState, accent: Color) {
    let mut spans = Vec::new();
    spans.push(Span::styled(
        format!(
            "{} marked -> {}  ",
            state.active_pane().marked_count(),
            state.inactive_pane().path_editor()
        ),
        Style::default().fg(Color::DarkGray),
    ));
    for (idx, label) in state.action_labels().into_iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            format!("[{label}]"),
            Style::default()
                .fg(super::contrasting_fg(accent))
                .bg(accent)
                .add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn format_size(size: Option<u64>) -> String {
    let Some(size) = size else {
        return String::new();
    };
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KiB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GiB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_mtime(mtime_seconds: Option<i64>) -> String {
    mtime_seconds
        .map(|value| value.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_pane::{BrowserEntry, EntryKind, Location};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn rendered_text(state: &DualPaneState) -> String {
        let backend = TestBackend::new(100, 18);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_into(frame, frame.area(), state, Color::Cyan))
            .expect("draw");
        format!("{:?}", terminal.backend().buffer())
    }

    #[test]
    fn renders_two_panes_and_visible_actions() {
        let mut state = DualPaneState::new(Location::local("/src"), Location::local("/dst"));
        state.pane_mut(PaneId::Left).set_entries(vec![
            BrowserEntry::new("photos", "photos", EntryKind::Directory),
            BrowserEntry::new("notes", "notes.txt", EntryKind::File).with_size(4096),
        ]);

        let text = rendered_text(&state);

        assert!(text.contains("Left: Local /src"));
        assert!(text.contains("Right: Local /dst"));
        assert!(text.contains("[Copy -> Right]"));
        assert!(text.contains("photos"));
        assert!(text.contains("4.0 KiB"));
    }

    #[test]
    fn action_direction_flips_with_active_pane() {
        let mut state = DualPaneState::new(Location::local("/src"), Location::local("/dst"));
        state.switch_active();

        let text = rendered_text(&state);

        assert!(text.contains("[Copy -> Left]"));
        assert!(text.contains("[Mirror -> Left]"));
        assert!(text.contains("[Move -> Left]"));
    }
}
