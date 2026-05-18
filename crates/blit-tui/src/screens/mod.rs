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

/// Paint the F1..F4 tab strip into `area`. The active
/// pane is rendered bold + cyan; inactive panes are dim.
pub fn render_tab_strip(frame: &mut Frame, area: Rect, active: Screen) {
    let mut spans: Vec<Span> = Vec::with_capacity(8);
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
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
