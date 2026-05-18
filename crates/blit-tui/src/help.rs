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
    // Center a 70×32 box inside the given area. If the
    // area is smaller than the box, use the full area —
    // ratatui's diff renderer truncates rather than
    // crashing on overflow.
    let modal = centered(area, 70, 32);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help · press ? or Esc to close ");
    let lines: Vec<Line<'static>> = vec![
        section_header("Navigation (global)"),
        kv("F1", "Daemons pane"),
        kv("F2", "Transfers pane"),
        kv("F3", "Browse pane"),
        kv("F4", "Profile / Verify / Diagnostics / Transfer"),
        kv("?", "toggle this help overlay"),
        kv("q / Esc", "quit (Ctrl-c emergency)"),
        // d-16 R2: `r` works on every pane — rescan
        // discovery on F1, re-open Subscribe / GetState
        // on F2, re-fetch browse on F3, re-read profile
        // on F4. Belongs in the global section, not
        // under any pane-specific block.
        kv("r", "refresh / rescan (active pane)"),
        Line::from(""),
        section_header("F1 · F3 navigation"),
        kv("↑ ↓ / j k", "cursor (F1, F3)"),
        kv("Enter / → / l", "descend (F3)"),
        kv("← / h", "ascend (F3)"),
        Line::from(""),
        section_header("F4 · Profile lifecycle"),
        kv("c / d / e", "clear / disable / enable history"),
        kv("s", "diagnostics snapshot"),
        Line::from(""),
        section_header("F4 · Verify form"),
        kv("Tab", "enter / cycle Source → Destination"),
        kv("Enter", "run compare_trees"),
        kv("H", "toggle hash mode (size+mtime ↔ checksum)"),
        kv("O", "toggle direction (two-way ↔ one-way)"),
        Line::from(""),
        section_header("F4 · Local transfer"),
        kv("C", "copy Source → Destination"),
        kv("M", "mirror (prompts before deleting at dest)"),
        kv("V", "move (prompts before deleting source)"),
        kv("y / N / Esc", "confirm / cancel destructive prompt"),
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
        let modal = centered(area, 70, 32);
        // Width / height are capped to the area's dims.
        assert!(modal.width <= 40);
        assert!(modal.height <= 10);
        assert!(modal.width > 0 && modal.height > 0);
    }

    /// d-16: regression test — the help overlay's keymap
    /// must surface every public keystroke. Renders the
    /// modal into a TestBackend and asserts each key
    /// appears in the right section.
    ///
    /// d-16 R2: tightened to check section attribution,
    /// not just bare substring presence — `r` was listed
    /// under "F1 · F3 navigation" while still active on
    /// F2/F4 in the original landing, and the old loose
    /// grep didn't catch it.
    #[test]
    fn help_modal_documents_all_public_keys() {
        use ratatui::{backend::TestBackend, Terminal};
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                render_overlay(frame, frame.area());
            })
            .expect("draw");
        // Flatten the buffer to a single string for grep.
        let buf = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        // First-pass: every key surfaces somewhere.
        for needle in [
            "F1",
            "F2",
            "F3",
            "F4",
            "Tab",
            "Enter",
            "C",
            "M",
            "V",
            "H",
            "O",
            "y / N / Esc",
            "c / d / e",
            "s",
            "?",
            "r", // refresh (global as of d-16 R2)
        ] {
            assert!(
                text.contains(needle),
                "help overlay missing key reference {needle:?}; rendered text:\n{text}",
            );
        }
        // Section attribution: each key must appear in
        // the right section. We slice the rendered text
        // between section headers and grep each slice.
        let global_section = section_contents(&text, "Navigation (global)");
        assert!(
            global_section.contains("r"),
            "`r` is a global refresh key; must live in Navigation (global) — got section:\n{global_section}",
        );
        let f4_profile = section_contents(&text, "F4 · Profile lifecycle");
        assert!(
            f4_profile.contains("c / d / e"),
            "F4 profile lifecycle section must list `c / d / e`; got:\n{f4_profile}",
        );
        assert!(
            f4_profile.contains('s'),
            "F4 profile lifecycle section must list `s` (snapshot); got:\n{f4_profile}",
        );
        let f4_verify = section_contents(&text, "F4 · Verify form");
        for needle in ["Tab", "Enter", "H", "O"] {
            assert!(
                f4_verify.contains(needle),
                "F4 Verify section missing {needle:?}; got:\n{f4_verify}",
            );
        }
        let f4_transfer = section_contents(&text, "F4 · Local transfer");
        for needle in ["C", "M", "V", "y / N / Esc"] {
            assert!(
                f4_transfer.contains(needle),
                "F4 transfer section missing {needle:?}; got:\n{f4_transfer}",
            );
        }
    }

    /// Slice the rendered modal text into the lines
    /// belonging to `header`'s section — between this
    /// header and the next one in display order. Used by
    /// the keymap test to assert section attribution.
    fn section_contents<'a>(text: &'a str, header: &str) -> &'a str {
        // Display order of section headers. Each entry is
        // (this_header, next_header). The final entry's
        // next_header is empty, meaning "to end of text".
        let next_header: &str = match header {
            "Navigation (global)" => "F1 · F3 navigation",
            "F1 · F3 navigation" => "F4 · Profile lifecycle",
            "F4 · Profile lifecycle" => "F4 · Verify form",
            "F4 · Verify form" => "F4 · Local transfer",
            "F4 · Local transfer" => "",
            other => panic!("unknown section header in help test: {other:?}"),
        };
        let start = text.find(header).unwrap_or_else(|| {
            panic!("section header {header:?} not found in rendered help; got:\n{text}")
        });
        let rest = &text[start..];
        if next_header.is_empty() {
            return rest;
        }
        let end = rest.find(next_header).unwrap_or(rest.len());
        &rest[..end]
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
