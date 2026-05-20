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
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

/// Help overlay visibility flag. Lives on `AppState` so
/// it survives across pane navigation (open the help on
/// F2, switch to F3, the help is still up).
///
/// d-31: the keymap has grown past what fits on an
/// 80×24 terminal (the modal is ~36 rows). `scroll_offset`
/// lets the operator page through the list with j/k
/// while the overlay is open.
#[derive(Debug, Default, Clone, Copy)]
pub struct HelpOverlay {
    visible: bool,
    /// d-31: top-line offset applied to the keymap
    /// Paragraph. 0 = top. Clamped by `scroll_down` so
    /// the operator can't scroll past the last line.
    scroll_offset: u16,
}

impl HelpOverlay {
    pub fn is_visible(self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        // d-31: re-opening always starts at the top.
        if !self.visible {
            self.scroll_offset = 0;
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
        // d-31: reset scroll so the next open starts at
        // the top.
        self.scroll_offset = 0;
    }

    /// d-31: current top-line offset for the renderer.
    pub fn scroll_offset(self) -> u16 {
        self.scroll_offset
    }

    /// d-31: scroll down one line, clamped so at least
    /// `MIN_VISIBLE_LINES` of content stays on screen
    /// (otherwise the operator could scroll into an
    /// all-blank modal).
    pub fn scroll_down(&mut self) {
        let max = help_line_count().saturating_sub(Self::MIN_VISIBLE_LINES);
        if self.scroll_offset < max {
            self.scroll_offset += 1;
        }
    }

    /// d-31: scroll up one line. No-op at the top.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Floor on how many keymap lines stay visible at the
    /// maximum scroll offset. Keeps the bottom of the
    /// list reachable without letting the operator scroll
    /// the content entirely off-screen.
    const MIN_VISIBLE_LINES: u16 = 3;
}

/// The full keymap as renderable lines. Extracted so
/// both the renderer and `help_line_count` (scroll
/// clamp) share one source of truth.
fn help_lines() -> Vec<Line<'static>> {
    vec![
        section_header("Navigation (global)"),
        kv("F1 / 1", "Daemons pane"),
        kv("F2 / 2", "Transfers pane"),
        kv("F3 / 3", "Browse pane"),
        kv("F4 / 4", "Profile / Verify / Diagnostics / Transfer"),
        kv("?", "toggle this help overlay"),
        kv("q / Esc", "quit (Ctrl-c emergency)"),
        // d-16 R2: `r` works on every pane — rescan
        // discovery on F1, re-open Subscribe / GetState
        // on F2, re-fetch browse on F3, re-read profile
        // on F4. Belongs in the global section, not
        // under any pane-specific block.
        kv("r", "refresh / rescan (active pane)"),
        // d-36: Ctrl+R hot-reloads tui.toml.
        kv("Ctrl-R", "reload tui.toml (theme / ticks / transfer knobs)"),
        // d-31: j/k scroll this overlay when it's open.
        kv("j / k", "scroll this help (when open)"),
        Line::from(""),
        section_header("F1 · F2 · F3 navigation"),
        kv("↑ ↓ / j k", "cursor (F1, F2 active, F3)"),
        kv("g / G", "jump to first / last row (F1, F2, F3)"),
        kv("Enter / → / l", "descend (F3) · browse daemon (F1)"),
        kv("← / h", "ascend (F3)"),
        kv(
            "t",
            "trigger transfer (F1) — remote↔local; ↑↓ copy/mirror/move",
        ),
        kv(
            "K",
            "cancel selected transfer (F2) — y/N prompt if [transfer] confirm_cancel",
        ),
        kv(
            "X",
            "cancel ALL active transfers (F2) — Shift+x; honors confirm_cancel",
        ),
        kv("/", "filter rows (F3) — Esc clears, Enter commits"),
        kv(
            "p",
            "pull selected → local dir (F3) — Enter runs, Esc cancels",
        ),
        kv("m", "mirror selected → local dir (F3) — y/N confirm"),
        kv("v", "move selected → local dir, delete source (F3) — y/N"),
        kv("P", "pull marked set → local dir (F3) — Shift+p"),
        kv("u", "disk usage of selected subtree (F3)"),
        kv("space", "multi-select rows (F3)"),
        kv("a", "select / clear all visible rows (F3)"),
        kv("D", "delete cursor row or marked set (F3) — y/N confirm"),
        Line::from(""),
        section_header("F4 · Profile lifecycle"),
        kv("c / d / e", "clear / disable / enable history"),
        kv("s", "diagnostics snapshot"),
        Line::from(""),
        section_header("F4 · Verify form"),
        kv("Tab", "enter / cycle Source → Destination"),
        kv("Enter", "run compare_trees"),
        kv("Ctrl-U", "clear focused field"),
        kv("H", "toggle hash mode (size+mtime ↔ checksum)"),
        kv("O", "toggle direction (two-way ↔ one-way)"),
        Line::from(""),
        section_header("F4 · Local transfer"),
        kv("C", "copy Source → Destination"),
        kv("M", "mirror (prompts before deleting at dest)"),
        kv("V", "move (prompts before deleting source)"),
        kv("y / N / Esc", "confirm / cancel destructive prompt"),
    ]
}

/// Total keymap line count. Drives the d-31 scroll clamp
/// in `HelpOverlay::scroll_down`.
fn help_line_count() -> u16 {
    help_lines().len() as u16
}

/// Render the overlay over the area provided (typically
/// the pane's body area; the caller chooses how much of
/// the frame to dim). Uses `Clear` to wipe the underlying
/// widgets so the modal isn't garbled by mid-render text.
///
/// d-31: takes the [`HelpOverlay`] so the renderer can
/// apply the operator's scroll offset.
pub fn render_overlay(frame: &mut Frame, area: Rect, overlay: HelpOverlay) {
    // Center a 70×36 box inside the given area. If the
    // area is smaller than the box, use the full area —
    // ratatui's diff renderer truncates rather than
    // crashing on overflow. d-26 bumped 34→35 to fit the
    // `/` filter row; d-30 bumped 35→36 for `X` batch
    // cancel; d-35 bumped 36→37 for `p` pull; d-36 bumped
    // 37→38 for `Ctrl-R` reload; d-41 bumped 38→39 for `u`
    // du; d-42 bumped 39→40 for `g / G` jump; d-45 bumped
    // 40→41 for `D` delete; d-49 bumped 41→42 for `space`
    // multi-select; d-51 bumped 42→43 for `a` select-all;
    // d-53 bumped 43→44 for `P` batch pull; d-55 bumped
    // 44→45 for `m` mirror; d-57 bumped 45→46 for `v` move;
    // d-58 bumped 46→47 for `t` trigger. d-31: when the area
    // is shorter than the modal, the operator scrolls with j/k.
    let modal = centered(area, 70, 47);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help · press ? or Esc to close ");
    // d-31: apply the vertical scroll offset. Paragraph
    // clips lines above the offset and below the modal
    // height — exactly the page-through behavior we want.
    let para = Paragraph::new(help_lines())
        .block(block)
        .scroll((overlay.scroll_offset(), 0));
    frame.render_widget(para, modal);

    // d-32: when the keymap overflows the modal's inner
    // height, draw a scrollbar on the right border so the
    // operator can see there's more above/below. Without
    // the indicator (d-31), the only cue that scrolling
    // does anything was the self-doc `j / k` row.
    let inner_height = modal.height.saturating_sub(2); // top + bottom border
    let total = help_line_count();
    if total > inner_height {
        let mut sb_state =
            ScrollbarState::new(total as usize).position(overlay.scroll_offset() as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        frame.render_stateful_widget(scrollbar, modal, &mut sb_state);
    }
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

    // d-31: help overlay scroll.

    #[test]
    fn new_overlay_starts_at_top() {
        let overlay = HelpOverlay::default();
        assert_eq!(overlay.scroll_offset(), 0);
    }

    #[test]
    fn scroll_down_advances_offset() {
        let mut overlay = HelpOverlay::default();
        overlay.scroll_down();
        assert_eq!(overlay.scroll_offset(), 1);
        overlay.scroll_down();
        assert_eq!(overlay.scroll_offset(), 2);
    }

    #[test]
    fn scroll_up_is_saturating_at_top() {
        let mut overlay = HelpOverlay::default();
        overlay.scroll_up();
        assert_eq!(overlay.scroll_offset(), 0, "can't scroll above the top");
    }

    #[test]
    fn scroll_down_then_up_returns_to_top() {
        let mut overlay = HelpOverlay::default();
        overlay.scroll_down();
        overlay.scroll_down();
        overlay.scroll_up();
        overlay.scroll_up();
        assert_eq!(overlay.scroll_offset(), 0);
    }

    #[test]
    fn scroll_down_clamps_so_content_stays_visible() {
        let mut overlay = HelpOverlay::default();
        // Scroll way past the end.
        for _ in 0..1000 {
            overlay.scroll_down();
        }
        let max = help_line_count().saturating_sub(HelpOverlay::MIN_VISIBLE_LINES);
        assert_eq!(
            overlay.scroll_offset(),
            max,
            "scroll clamps so MIN_VISIBLE_LINES of content stay on screen"
        );
        // At least some content is still in view.
        assert!(overlay.scroll_offset() < help_line_count());
    }

    #[test]
    fn close_resets_scroll() {
        let mut overlay = HelpOverlay::default();
        overlay.toggle(); // open
        overlay.scroll_down();
        overlay.scroll_down();
        assert!(overlay.scroll_offset() > 0);
        overlay.close();
        assert_eq!(overlay.scroll_offset(), 0, "close resets scroll to top");
    }

    #[test]
    fn toggle_closed_resets_scroll() {
        let mut overlay = HelpOverlay::default();
        overlay.toggle(); // open
        overlay.scroll_down();
        overlay.toggle(); // close via toggle
        assert_eq!(overlay.scroll_offset(), 0);
    }

    /// Helper: render the overlay into a TestBackend of
    /// the given size and flatten the buffer to a string.
    fn render_to_string(overlay: HelpOverlay, w: u16, h: u16) -> String {
        use ratatui::{backend::TestBackend, Terminal};
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                render_overlay(frame, frame.area(), overlay);
            })
            .expect("draw");
        let buf = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    /// d-32: when the modal area is shorter than the
    /// keymap, a scrollbar renders on the right edge. At
    /// offset 0 with content below, the end marker `▼`
    /// shows.
    #[test]
    fn scrollbar_renders_when_content_overflows() {
        // 80×12 → modal clamps to 12 rows, inner height
        // 10 < 34 keymap lines, so the scrollbar shows.
        let text = render_to_string(HelpOverlay::default(), 80, 12);
        assert!(
            text.contains('▼'),
            "overflowing modal must show a downward scroll marker; got:\n{text}"
        );
    }

    /// d-32: when the modal fits the keymap entirely, no
    /// scrollbar markers render — the indicator only
    /// appears when it's useful.
    #[test]
    fn scrollbar_absent_when_content_fits() {
        // d-58: the keymap grew; render in a 52-row area
        // (modal caps at 47, inner 45) so it fits and no
        // scrollbar is needed.
        let text = render_to_string(HelpOverlay::default(), 80, 52);
        assert!(
            !text.contains('▲') && !text.contains('▼'),
            "non-overflowing modal must not show scroll markers; got:\n{text}"
        );
    }

    #[test]
    fn centered_clamps_to_area_when_smaller() {
        let area = Rect::new(0, 0, 40, 10);
        let modal = centered(area, 70, 38);
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
        // d-45: tall enough that the full keymap renders
        // without clipping the bottom section the grep checks.
        // d-58: bumped to 52 as the keymap + modal grew.
        let backend = TestBackend::new(80, 52);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                // d-31: render at the top (offset 0) so
                // every section is visible for the grep.
                render_overlay(frame, frame.area(), HelpOverlay::default());
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
            "r",                               // refresh (global as of d-16 R2)
            "/",                               // d-26: F3 filter
            "X",                               // d-30: F2 batch cancel
            "p",                               // d-35: F3 pull
            "pull marked set",                 // d-53: F3 batch pull (`P`)
            "disk usage of selected subtree",  // d-41: F3 du (`u`)
            "jump to first / last row",        // d-42: g / G
            "delete cursor row or marked set", // d-45/d-50: F3 delete (`D`)
            "multi-select rows",               // d-49: F3 space
            "select / clear all visible rows", // d-51: F3 `a`
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
