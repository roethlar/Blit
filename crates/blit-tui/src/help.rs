//! `?` help overlay. Toggled by the `?` key; absorbs
//! keystrokes (except `?` itself + Esc) while visible so
//! the operator can study the keymap without accidentally
//! triggering pane actions.
//!
//! Atomic scope: a single static keymap reference rendered
//! as a centered modal. Per-pane contextual help (e.g.
//! "what does Enter do here?") is future polish.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// Help overlay visibility flag. Lives on `AppState` so
/// it survives across pane navigation (open the help on
/// F2, switch to F3, the help is still up).
#[derive(Debug, Default, Clone, Copy)]
pub struct HelpOverlay {
    visible: bool,
}

impl HelpOverlay {
    pub fn is_visible(self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

/// Render the overlay over the area provided (typically
/// the pane's body area; the caller chooses how much of
/// the frame to dim). Uses `Clear` to wipe the underlying
/// widgets so the modal isn't garbled by mid-render text.
pub fn render_overlay(frame: &mut Frame, area: Rect) {
    // Center a 60×16 box inside the given area. If the
    // area is smaller than the box, use the full area.
    let modal = centered(area, 64, 18);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help · press ? or Esc to close ");
    let lines: Vec<Line<'static>> = vec![
        section_header("Navigation (global)"),
        kv("F1", "Daemons pane"),
        kv("F2", "Transfers pane"),
        kv("F3", "Browse pane"),
        kv("F4", "Profile / Verify / Diagnostics"),
        kv("?", "toggle this help overlay"),
        kv("q / Esc", "quit (Ctrl-c emergency)"),
        Line::from(""),
        section_header("Per-pane"),
        kv("r", "refresh / rescan"),
        kv("↑ ↓ / j k", "cursor (F1, F3)"),
        kv("Enter / → / l", "descend (F3)"),
        kv("← / h", "ascend (F3)"),
        kv("Tab", "enter / cycle Verify form (F4)"),
        kv("c / d / e", "profile clear / disable / enable (F4)"),
        kv("s", "diagnostics snapshot (F4)"),
    ];
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, modal);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(h)) / 2),
            Constraint::Length(h),
            Constraint::Min(0),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(w)) / 2),
            Constraint::Length(w),
            Constraint::Min(0),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn section_header(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn kv(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{key:>14}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::raw(desc.to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_visibility() {
        let mut overlay = HelpOverlay::default();
        assert!(!overlay.is_visible());
        overlay.toggle();
        assert!(overlay.is_visible());
        overlay.toggle();
        assert!(!overlay.is_visible());
    }

    #[test]
    fn close_sets_invisible_regardless_of_prior() {
        let mut overlay = HelpOverlay::default();
        overlay.close();
        assert!(!overlay.is_visible());
        overlay.toggle();
        overlay.close();
        assert!(!overlay.is_visible());
    }

    #[test]
    fn centered_clamps_to_area_when_smaller() {
        let area = Rect::new(0, 0, 40, 10);
        let modal = centered(area, 64, 18);
        // Width / height are capped to the area's dims.
        assert!(modal.width <= 40);
        assert!(modal.height <= 10);
        assert!(modal.width > 0 && modal.height > 0);
    }

    #[test]
    fn centered_returns_centered_rect_inside_area() {
        let area = Rect::new(0, 0, 100, 40);
        let modal = centered(area, 60, 20);
        assert_eq!(modal.width, 60);
        assert_eq!(modal.height, 20);
        // Roughly centered: ~20-padded left, ~10-padded top.
        assert_eq!(modal.x, 20);
        assert_eq!(modal.y, 10);
    }
}
