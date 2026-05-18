//! Per-screen render modules. a1-2 shipped F2 (Transfers);
//! a1-3 added F1 (Daemons); a1-4 added F3 (Browse);
//! a1-5 added F4 (Profile); a1-6 adds the tab strip used
//! by all panes.

pub mod f1;
pub mod f2;
pub mod f3;
pub mod f4;

use crate::Screen;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
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

/// Paint the F1..F4 tab strip into `area`.
///
/// e-2 round 2 makes the layout responsive: tab labels
/// always come first (full or short variant) so the
/// primary navigation surface never gets clipped, and the
/// right-side counts shrink / disappear as area width
/// drops.
///
/// Width regimes (cumulative — falls through to the next
/// smaller as `area.width` shrinks):
///
/// 1. `area.width ≥ tab_full + counts_full`: full tab
///    labels (" F1 Daemons ", ...) + full counts (e.g.
///    "3 daemons · 1 active · 47 recent · ? help").
/// 2. `area.width ≥ tab_full + counts_short`: full tabs
///    + short counts ("3d · 1a · 47r").
/// 3. `area.width ≥ tab_short + counts_short`: short
///    tabs (" F1 ", ...) + short counts.
/// 4. Otherwise: short tabs only, no counts. The tabs are
///    always painted; on a terminal narrower than the
///    short-tab width, ratatui's Paragraph truncates the
///    span as a last resort.
pub fn render_tab_strip(
    frame: &mut Frame,
    area: Rect,
    active: Screen,
    counts: TabStripCounts,
    show_counts: bool,
    accent: Color,
) {
    let full_tab_spans = build_tab_spans(active, false, accent);
    let short_tab_spans = build_tab_spans(active, true, accent);
    let full_tab_width = total_span_width(&full_tab_spans);
    let short_tab_width = total_span_width(&short_tab_spans);

    // e-4: when `show_counts` is false (operator opted
    // out via `[tab_strip] show_counts = false`), the
    // right-edge column collapses to zero width and the
    // tabs always get full labels if they fit. Layout
    // logic stays identical; we just feed it zero widths
    // for the counts so it never gets selected.
    let full_counts = format_counts_full(counts);
    let short_counts = format_counts_short(counts);
    let (full_counts_width, short_counts_width) = if show_counts {
        (
            full_counts.chars().count() as u16,
            short_counts.chars().count() as u16,
        )
    } else {
        (0, 0)
    };

    let (tab_spans, tab_width, counts_str) =
        if area.width >= full_tab_width.saturating_add(full_counts_width) {
            (full_tab_spans, full_tab_width, full_counts)
        } else if area.width >= full_tab_width.saturating_add(short_counts_width) {
            (full_tab_spans, full_tab_width, short_counts)
        } else if area.width >= short_tab_width.saturating_add(short_counts_width) {
            (short_tab_spans, short_tab_width, short_counts)
        } else {
            (short_tab_spans, short_tab_width, String::new())
        };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(tab_width), Constraint::Min(0)])
        .split(area);

    frame.render_widget(Paragraph::new(Line::from(tab_spans)), chunks[0]);

    if show_counts && !counts_str.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                counts_str,
                Style::default().fg(Color::DarkGray),
            )))
            .alignment(Alignment::Right),
            chunks[1],
        );
    }
}

/// Build the tab spans. `short=true` uses just " F1 " etc.
/// (drops the "Daemons"/"Transfers"/... label).
fn build_tab_spans(active: Screen, short: bool, accent: Color) -> Vec<Span<'static>> {
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
                .bg(accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if idx > 0 {
            spans.push(Span::raw("  "));
        }
        let text = if short {
            format!(" {key} ")
        } else {
            format!(" {key} {label} ")
        };
        spans.push(Span::styled(text, style));
    }
    spans
}

fn total_span_width(spans: &[Span<'_>]) -> u16 {
    spans.iter().map(|s| s.content.chars().count() as u16).sum()
}

fn format_counts_full(counts: TabStripCounts) -> String {
    format!(
        "{} daemons · {} active · {} recent · ? help",
        counts.daemons, counts.active_transfers, counts.recent_transfers,
    )
}

fn format_counts_short(counts: TabStripCounts) -> String {
    format!(
        "{}d · {}a · {}r",
        counts.daemons, counts.active_transfers, counts.recent_transfers,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_counts_full_includes_all_three_numbers() {
        let s = format_counts_full(TabStripCounts {
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
    fn format_counts_full_with_zeroes() {
        let s = format_counts_full(TabStripCounts::default());
        assert!(s.contains("0 daemons"));
    }

    #[test]
    fn format_counts_short_keeps_numbers_drops_help_hint() {
        let s = format_counts_short(TabStripCounts {
            daemons: 3,
            active_transfers: 1,
            recent_transfers: 47,
        });
        assert!(s.contains("3d"));
        assert!(s.contains("1a"));
        assert!(s.contains("47r"));
        assert!(!s.contains("?"), "short form drops the ? help hint");
    }

    /// e-2 R2 finding 3: full tab labels render even at
    /// 80 cols. With width=80 we should fit full tabs +
    /// at least the short counts.
    #[test]
    fn render_at_80_cols_keeps_full_tabs() {
        let full = build_tab_spans(Screen::F1, false, Color::Cyan);
        let full_width = total_span_width(&full);
        assert!(
            full_width <= 60,
            "full-tab spans must fit within 60 cols so 80-col terminals \
             have room for counts; got {full_width}"
        );
        // 80 - full_width ≥ short_counts.len() (14ish) so
        // the responsive regime picks "full tabs + short
        // counts" not "short tabs + nothing".
        let short_counts = format_counts_short(TabStripCounts::default());
        let short_counts_w = short_counts.chars().count() as u16;
        assert!(
            full_width + short_counts_w <= 80,
            "full tabs + short counts must fit in 80 cols ({full_width} + {short_counts_w})",
        );
    }

    /// Short tabs alone fit even on a 30-col terminal.
    #[test]
    fn short_tabs_fit_narrow_terminal() {
        let short = build_tab_spans(Screen::F1, true, Color::Cyan);
        let short_width = total_span_width(&short);
        assert!(
            short_width <= 30,
            "short-tab spans must fit within 30 cols; got {short_width}"
        );
    }

    // e-4: tab-strip render honors the `show_counts`
    // flag from tui.toml.

    #[test]
    fn render_tab_strip_with_counts_shown_renders_counts() {
        use ratatui::{backend::TestBackend, Terminal};
        // 120 wide so the responsive layout picks the
        // full counts format ("3 daemons · ..."), not the
        // short fallback ("3d · 1a · 47r").
        let backend = TestBackend::new(120, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                render_tab_strip(
                    frame,
                    frame.area(),
                    Screen::F1,
                    TabStripCounts {
                        daemons: 3,
                        active_transfers: 1,
                        recent_transfers: 47,
                    },
                    true,
                    Color::Cyan,
                );
            })
            .expect("draw");
        let buf = terminal.backend().buffer();
        let mut text = String::new();
        for x in 0..buf.area.width {
            text.push_str(buf[(x, 0)].symbol());
        }
        assert!(text.contains("3 daemons"));
        assert!(text.contains("? help"));
    }

    #[test]
    fn render_tab_strip_with_counts_hidden_omits_counts() {
        use ratatui::{backend::TestBackend, Terminal};
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                render_tab_strip(
                    frame,
                    frame.area(),
                    Screen::F1,
                    TabStripCounts {
                        daemons: 3,
                        active_transfers: 1,
                        recent_transfers: 47,
                    },
                    false,
                    Color::Cyan,
                );
            })
            .expect("draw");
        let buf = terminal.backend().buffer();
        let mut text = String::new();
        for x in 0..buf.area.width {
            text.push_str(buf[(x, 0)].symbol());
        }
        // No counts strings present.
        assert!(!text.contains("daemons"));
        assert!(!text.contains("? help"));
        assert!(!text.contains("47"));
        // Tabs still render in full (no counts column
        // eating into the width budget).
        assert!(text.contains("Daemons"));
        assert!(text.contains("Profile"));
    }
}
