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
use crossterm::cursor::Show;
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
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Tracks whether raw mode has been entered. Used by the
/// panic hook to decide whether `restore_terminal` is safe
/// to call (avoids spurious teardown if the panic fires
/// before `enter_raw_mode` succeeded). Atomic for the panic
/// hook's `Send + Sync` requirement.
static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let _ = args; // consumed in future slices.

    // Install the panic hook BEFORE touching the terminal so
    // a panic inside `TuiGuard::new()` (or anywhere
    // downstream) still restores. The hook chains the original
    // handler so panic output still appears after restore.
    install_panic_hook();

    // `TuiGuard::new` is transactional: if any setup step
    // fails it unwinds the partial state before returning Err.
    // `Drop` covers every other exit path (normal return,
    // `?`-propagated error, panic unwinding through main).
    let mut guard = TuiGuard::new().context("entering TUI")?;
    let result = run_event_loop(guard.terminal_mut()).await;
    // `guard` drops here (end of scope). Drop runs
    // `restore_terminal` regardless of `result`. Returning
    // `result` after drop preserves the loop's exit status.
    drop(guard);
    result
}

/// RAII wrapper for the crossterm/ratatui terminal lifecycle.
/// `new()` is transactional — partial setup failures unwind
/// before the Result is returned. `Drop` restores raw mode,
/// leaves the alternate screen, and shows the cursor on
/// every exit path (normal, ?-propagated error, panic
/// unwinding).
struct TuiGuard {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
}

impl TuiGuard {
    /// Sets up raw mode → alternate screen → terminal → clear
    /// → hide cursor in order. Each step on success advances
    /// a local `progress` marker so failures rewind only the
    /// steps that actually succeeded.
    fn new() -> Result<Self> {
        enable_raw_mode().context("enable_raw_mode")?;
        // From this point on `TUI_ACTIVE` reflects state we
        // need to roll back if anything below fails.
        TUI_ACTIVE.store(true, Ordering::SeqCst);

        // Stage 1: alternate screen.
        let mut stdout = io::stdout();
        if let Err(err) = execute!(stdout, EnterAlternateScreen) {
            // Roll back stage 0.
            let _ = disable_raw_mode();
            TUI_ACTIVE.store(false, Ordering::SeqCst);
            return Err(eyre::eyre!("EnterAlternateScreen: {err}"));
        }

        // Stage 2: ratatui Terminal handle. From here we have
        // a real Terminal we can call `clear` / `hide_cursor`
        // / `show_cursor` on. Failures rewind via the
        // already-stored TUI_ACTIVE flag through `restore_terminal()`.
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend) {
            Ok(t) => t,
            Err(err) => {
                restore_terminal();
                return Err(eyre::eyre!("Terminal::new: {err}"));
            }
        };

        // Stage 3+: terminal-API calls. Same rollback shape.
        if let Err(err) = terminal.clear() {
            restore_terminal();
            return Err(eyre::eyre!("terminal.clear: {err}"));
        }
        if let Err(err) = terminal.hide_cursor() {
            restore_terminal();
            return Err(eyre::eyre!("terminal.hide_cursor: {err}"));
        }

        Ok(Self {
            terminal: Some(terminal),
        })
    }

    /// Mutable borrow of the contained `Terminal` for the
    /// event loop's draw cycles. Stays inside the guard so
    /// Drop owns the lifecycle — the loop can't outlive the
    /// guard's restoration.
    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        self.terminal
            .as_mut()
            .expect("TuiGuard::terminal_mut after Drop is impossible")
    }
}

impl Drop for TuiGuard {
    fn drop(&mut self) {
        // Idempotent — restore_terminal checks TUI_ACTIVE.
        restore_terminal();
    }
}

/// Pure state transition: swap the provided flag to false
/// and return whether this call was the one that observed
/// it `true`. Parameterised on the flag so unit tests can
/// pass local `AtomicBool` instances and run in parallel
/// without racing on the process-global `TUI_ACTIVE`.
fn take_active_for_restore(flag: &AtomicBool) -> bool {
    flag.swap(false, Ordering::SeqCst)
}

/// Best-effort terminal restore: show cursor, leave
/// alternate screen, disable raw mode. Idempotent —
/// the first caller observes `TUI_ACTIVE = true` via
/// `take_active_for_restore`; subsequent callers see it
/// `false` and early-return. The panic hook and Drop can
/// both call this without double-teardown.
fn restore_terminal() {
    if !take_active_for_restore(&TUI_ACTIVE) {
        return;
    }
    let mut stdout = io::stdout();
    let _ = execute!(stdout, Show);
    let _ = execute!(stdout, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// Install a panic hook that restores the terminal before
/// chaining to the previous hook. Without this a panic
/// during the event loop leaves the terminal in raw mode +
/// alternate screen + cursor hidden until the user types
/// `reset` or restarts their shell.
fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original(info);
    }));
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

    /// `take_active_for_restore` is the pure state-transition
    /// helper that `restore_terminal` uses to decide whether
    /// to fire any crossterm calls. Testing it directly
    /// validates the idempotency contract WITHOUT writing
    /// real terminal escape sequences to stderr.
    ///
    /// Tests use local `AtomicBool` instances (round 3
    /// review) so parallel test execution doesn't race on
    /// the process-global `TUI_ACTIVE`.
    ///
    /// Inactive → false (and stays false).
    #[test]
    fn take_active_for_restore_inactive_returns_false() {
        let flag = AtomicBool::new(false);
        assert!(!take_active_for_restore(&flag));
        assert!(!flag.load(Ordering::SeqCst));
    }

    /// Active → true on first call, false on subsequent
    /// calls. Validates the "panic hook AND Drop both call
    /// this" contract: only the winner does the teardown.
    #[test]
    fn take_active_for_restore_active_then_inactive() {
        let flag = AtomicBool::new(true);
        assert!(take_active_for_restore(&flag));
        assert!(!flag.load(Ordering::SeqCst));
        // Second caller sees inactive — no double teardown.
        assert!(!take_active_for_restore(&flag));
        assert!(!flag.load(Ordering::SeqCst));
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
