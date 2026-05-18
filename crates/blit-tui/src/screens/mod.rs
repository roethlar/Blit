//! Per-screen render modules. a1-2 shipped F2 (Transfers);
//! a1-3 added F1 (Daemons); a1-4 added F3 (Browse);
//! a1-5 added F4 (Profile); a1-6 adds the tab strip used
//! by all panes.

pub mod f1;
pub mod f2;
pub mod f3;
pub mod f4;

use crate::Screen;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Split the frame's area into a one-line tab strip on top
/// and the remaining area for the active pane. Returns
/// `(tab_area, body_area)` so the pane renderer can paint
/// the body region exactly as before.
///
/// Called by the router at the top of every draw; the
/// individual pane renderers paint into `body_area`.
pub fn split_for_tabs(frame_area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(frame_area);
    (chunks[0], chunks[1])
}

/// At-a-glance counts surfaced on the tab strip's right
/// edge. Mirrors TUI_DESIGN §5's header line
/// ("3 daemons │ 1 transfer active"). e-2 fills these
/// from `AppState`; pass `Default` to render zeroes.
#[derive(Debug, Clone, Copy, Default)]
pub struct TabStripCounts {
    pub daemons: usize,
    pub active_transfers: usize,
    pub recent_transfers: usize,
}

/// Paint the F1..F4 tab strip into `area`. The active
/// pane is rendered bold + cyan; inactive panes are dim.
/// Right-side counts column shows discovered daemons +
/// active/recent transfer counts + a `? help` reminder.
pub fn render_tab_strip(frame: &mut Frame, area: Rect, active: Screen, counts: TabStripCounts) {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(8);
    for (idx, (key, label, screen)) in [
        ("F1", "Daemons", Screen::F1),
        ("F2", "Transfers", Screen::F2),
        ("F3", "Browse", Screen::F3),
        ("F4", "Profile", Screen::F4),
    ]
    .iter()
    .enumerate()
    {
        let style = if *screen == active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if idx > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(format!(" {key} {label} "), style));
    }
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(28), Constraint::Length(48)])
        .split(area);
    frame.render_widget(Paragraph::new(Line::from(spans)), chunks[0]);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format_counts_line(counts),
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(ratatui::layout::Alignment::Right),
        chunks[1],
    );
}

fn format_counts_line(counts: TabStripCounts) -> String {
    format!(
        "{} daemons · {} active · {} recent · ? help",
        counts.daemons, counts.active_transfers, counts.recent_transfers,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_counts_line_includes_all_three_numbers() {
        let s = format_counts_line(TabStripCounts {
            daemons: 3,
            active_transfers: 1,
            recent_transfers: 47,
        });
        assert!(s.contains("3 daemons"));
        assert!(s.contains("1 active"));
        assert!(s.contains("47 recent"));
        assert!(s.contains("? help"));
    }

    #[test]
    fn format_counts_line_with_zeroes() {
        let s = format_counts_line(TabStripCounts::default());
        assert!(s.contains("0 daemons"));
    }
}
