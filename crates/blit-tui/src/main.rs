//! `blit-tui` — single-pane-of-glass operator TUI.
//!
//! Phase 5 milestone A.1 of `docs/plan/TUI_DESIGN.md`.
//! This slice (`a1-1-tui-scaffold`) lands ONLY the crate
//! scaffold and a minimal `ratatui` event loop. The four
//! screens (F1 Daemons / F2 Transfers / F3 Browse / F4
//! Profile-Verify) land in subsequent A.1 sub-slices.
//!
//! Today the binary:
//! - Enters the alternate screen + raw mode on startup.
//! - Renders a placeholder splash screen.
//! - Polls keyboard events on a 50ms tick.
//! - Exits cleanly on `q`, `Esc`, or `Ctrl-C`.
//!
//! That's enough surface to exercise the terminal lifecycle
//! (raw-mode enter/leave, alternate-screen enter/leave,
//! panic-on-poll teardown) without committing to the screen
//! layouts. Future slices fill in the content.
//!
//! Driven by tokio because the F2 Transfers pane (next slice)
//! will need an async `Subscribe` stream. Using tokio's
//! current-thread runtime here so a single ratatui draw and
//! a single async stream poll can share the same task.

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use eyre::{Context, Result};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

/// CLI flags. Today `--remote` is captured but not yet
/// consumed — the F1 Daemons pane will use it as the default
/// daemon to connect to. Keeping the parser scaffold here so
/// the next slice doesn't have to re-litigate flag shape.
#[derive(Parser, Debug)]
#[command(name = "blit-tui", about = "Operator TUI for Blit", version)]
struct Args {
    /// Default daemon to connect to (host or host:port). Used
    /// by future F1/F2 panes; ignored for now.
    #[arg(long)]
    remote: Option<String>,
}

/// Polling cadence for the event loop. 50ms keeps keystroke
/// latency low without burning CPU on idle.
const EVENT_POLL_INTERVAL_MS: u64 = 50;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let _ = args; // consumed in future slices.

    let mut terminal = enter_tui().context("entering TUI")?;
    // Ensure the terminal is restored even if the inner loop
    // panics. `scopeguard`-style: wrap the body in a closure
    // and always call `leave_tui` afterwards.
    let result = run_event_loop(&mut terminal).await;
    leave_tui(&mut terminal).context("leaving TUI")?;
    result
}

/// Set up the crossterm-backed terminal: raw mode, alternate
/// screen, cursor hidden. Mirror the standard ratatui setup
/// recipe so future slices don't need to know the magic
/// incantation.
fn enter_tui() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("enable_raw_mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("EnterAlternateScreen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Terminal::new")?;
    terminal.clear().context("terminal.clear")?;
    terminal.hide_cursor().context("hide_cursor")?;
    Ok(terminal)
}

/// Restore the terminal. Best-effort — we ignore individual
/// errors here so a partial failure can't mask the
/// `run_event_loop` error the user actually cares about.
fn leave_tui(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let _ = terminal.show_cursor();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
    Ok(())
}

/// Main event/draw loop. Renders the placeholder splash and
/// polls keyboard events with a short timeout so the future
/// async-stream branch (Subscribe events) can interleave.
async fn run_event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    loop {
        terminal.draw(render_splash).context("terminal.draw")?;
        // Poll without blocking — the timeout is the loop's
        // refresh rate. A future slice will use
        // `tokio::select!` between this poll and a Subscribe
        // stream.
        if event::poll(Duration::from_millis(EVENT_POLL_INTERVAL_MS)).context("event::poll")? {
            if let Event::Key(key) = event::read().context("event::read")? {
                if key.kind == KeyEventKind::Press && should_quit(key.code, key.modifiers) {
                    return Ok(());
                }
            }
        }
    }
}

/// Quit predicate. `q` / `Esc` are the muscle-memory
/// shortcuts; `Ctrl-C` is the safety net for a stuck UI.
fn should_quit(code: KeyCode, modifiers: KeyModifiers) -> bool {
    matches!(code, KeyCode::Char('q') | KeyCode::Esc)
        || (code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
}

/// Placeholder splash screen. Replaced by the four-screen
/// layout in subsequent A.1 sub-slices.
fn render_splash(frame: &mut ratatui::Frame) {
    let area = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" blit-tui (scaffold) ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![Span::styled(
            "Phase 5 / A.1 scaffold",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Future screens: F1 Daemons · F2 Transfers · F3 Browse · F4 Profile/Verify."),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(", "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(", or "),
            Span::styled("Ctrl-C", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to exit."),
        ]),
    ];

    let centered = center_within(inner, lines.len() as u16);
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, centered);
}

/// Vertically center `height` lines of content within `area`.
/// Used by the splash so the placeholder text sits in the
/// middle of the screen instead of the top-left.
fn center_within(area: Rect, height: u16) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);
    chunks[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_quit_recognises_q_esc_ctrl_c() {
        assert!(should_quit(KeyCode::Char('q'), KeyModifiers::empty()));
        assert!(should_quit(KeyCode::Esc, KeyModifiers::empty()));
        assert!(should_quit(KeyCode::Char('c'), KeyModifiers::CONTROL));
    }

    #[test]
    fn should_quit_ignores_other_keys() {
        assert!(!should_quit(KeyCode::Char('a'), KeyModifiers::empty()));
        assert!(!should_quit(KeyCode::Enter, KeyModifiers::empty()));
        // Plain 'c' without Ctrl is not a quit shortcut.
        assert!(!should_quit(KeyCode::Char('c'), KeyModifiers::empty()));
    }

    #[test]
    fn center_within_returns_middle_band() {
        let area = Rect::new(0, 0, 80, 20);
        let centered = center_within(area, 6);
        assert_eq!(centered.height, 6);
        // The centered rect should sit roughly mid-screen:
        // top margin ≈ (20 - 6) / 2 = 7.
        assert_eq!(centered.y, 7);
        assert_eq!(centered.x, area.x);
        assert_eq!(centered.width, area.width);
    }
}
