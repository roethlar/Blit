//! `blit-tui` — single-pane-of-glass operator TUI.
//!
//! Phase 5 milestone A.1 of `docs/plan/TUI_DESIGN.md`.
//!
//! Slices so far:
//! - `a1-1-tui-scaffold`: crate + ratatui event loop +
//!   panic-safe terminal lifecycle.
//! - `a1-2-f2-transfers` (this slice): F2 Transfers pane.
//!   When `--remote <host>` is set, the binary fetches an
//!   initial `GetState` snapshot, opens a `Subscribe`
//!   stream against the daemon, and renders live active /
//!   recent rows. With no `--remote` the placeholder
//!   splash from a1-1 is replaced by an F2 frame in a
//!   "no remote configured" state so the layout is
//!   visible without a daemon.
//!
//! F1 (Daemons), F3 (Browse), F4 (Profile/Verify) land in
//! subsequent A.1 sub-slices.
//!
//! Driven by tokio current-thread. The event loop uses
//! `tokio::select!` between a keystroke poll and a
//! Subscribe stream message so a single task handles
//! both inputs.

mod browse;
mod daemons;
mod screens;
mod state;

use blit_app::admin::list_modules::Module;
use blit_app::admin::ls::DirEntry;
use blit_app::admin::{jobs, list_modules, ls};
use blit_app::scan;
use blit_core::generated::{DaemonEvent, DaemonState};
use blit_core::mdns::MdnsDiscoveredService;
use blit_core::remote::endpoint::RemoteEndpoint;
use browse::BrowseState;
use clap::Parser;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use daemons::{DaemonDetail, DaemonsState};
use eyre::{Context, Result};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use screens::f2::ConnectionStatus;
use state::TransfersState;
use std::io::{self, Stdout};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;

/// CLI flags. `--remote` is consumed by the F2 Transfers
/// pane (a1-2). `--screen` selects which pane the TUI
/// opens; a1-6 will replace this with in-app F-key routing.
#[derive(Parser, Debug)]
#[command(name = "blit-tui", about = "Operator TUI for Blit", version)]
struct Args {
    /// Default daemon to connect to (host or host:port).
    /// Consumed by F2; ignored by F1 (mDNS-only).
    #[arg(long)]
    remote: Option<String>,

    /// Which pane to render. Defaults to F2 to preserve
    /// the existing operator-facing behavior; F1 is opt-in
    /// until a1-6 lands the routing UI.
    #[arg(long, value_enum, default_value_t = ScreenArg::F2)]
    screen: ScreenArg,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum ScreenArg {
    F1,
    F2,
    F3,
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

    install_panic_hook();
    let mut guard = TuiGuard::new().context("entering TUI")?;
    let result = match args.screen {
        ScreenArg::F1 => run_f1_event_loop(guard.terminal_mut()).await,
        ScreenArg::F2 => run_f2_event_loop(guard.terminal_mut(), args.remote.as_deref()).await,
        ScreenArg::F3 => run_f3_event_loop(guard.terminal_mut(), args.remote.as_deref()).await,
    };
    drop(guard);
    result
}

/// Cadence for the background mDNS discovery loop. mDNS
/// answers settle inside ~1.5s on a quiet LAN, so 5s
/// between rescans is comfortably above the noise floor
/// while keeping the daemon list "fresh enough" for an
/// operator scanning the F1 pane.
const F1_DISCOVERY_INTERVAL: Duration = Duration::from_secs(5);

/// Per-scan timeout for the mDNS query. Each tick spends
/// up to this long collecting responses; smaller than the
/// interval to leave headroom for the next scan.
const F1_DISCOVERY_SCAN_TIMEOUT: Duration = Duration::from_millis(1500);

/// Sized to absorb a burst of progress events without
/// backing up the Subscribe forwarder. At the daemon's
/// 10 Hz progress cadence × N active rows + headroom for
/// transient draw pauses, 256 is plenty for one operator's
/// TUI.
const TUI_EVENT_BUFFER: usize = 256;

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

/// Main event/draw loop for the F2 Transfers pane. With a
/// `--remote`, runs against a live daemon (initial GetState
/// snapshot + Subscribe stream). Without `--remote`, renders
/// F2 in a "no remote configured" state so the layout is
/// visible.
async fn run_f2_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    remote_arg: Option<&str>,
) -> Result<()> {
    let mut state = TransfersState::new();
    let remote_label = remote_arg.unwrap_or("(no remote)").to_string();
    let mut status = if remote_arg.is_some() {
        ConnectionStatus::Connecting
    } else {
        ConnectionStatus::NoRemote
    };

    // a1-2 round 2: single-owner input task. crossterm
    // event::poll/read are sync and not cancellable, so racing
    // a fresh spawn_blocking on each loop iteration leaked
    // detached blocking tasks that could each consume a key
    // press. Now ONE blocking task loops on event::poll and
    // forwards every key press through an mpsc; the main
    // loop selects on it without touching crossterm directly.
    let (key_tx, mut key_rx) = mpsc::channel::<KeyEvent>(16);
    spawn_input_task(key_tx);

    // Optional Subscribe channel. None when no `--remote`.
    let mut event_rx: Option<mpsc::Receiver<EventOrError>> = None;
    let mut parsed_remote: Option<RemoteEndpoint> = None;

    if let Some(remote_str) = remote_arg {
        match RemoteEndpoint::parse(remote_str) {
            Ok(endpoint) => {
                // a1-2 round 4: subscribe-first ordering is
                // now causally enforced. `open_subscribe_stream`
                // awaits the subscribe RPC and only returns
                // OK after the daemon's broadcast receiver is
                // registered. Any transfer that Started after
                // this point but before the snapshot lands
                // buffers in `rx` and replays onto state
                // below.
                match open_subscribe_stream(&endpoint).await {
                    Ok(rx) => {
                        // Initial GetState. Events broadcast
                        // during this RPC's flight buffer in
                        // `rx`.
                        match jobs::query(&endpoint, 0).await {
                            Ok(snapshot) => state.replace_from_snapshot(snapshot),
                            Err(err) => {
                                status = ConnectionStatus::Degraded(format!(
                                    "initial GetState failed: {err}"
                                ));
                            }
                        }
                        event_rx = Some(rx);
                        parsed_remote = Some(endpoint);
                        // Drain buffered events onto the
                        // snapshot. TransferStarted is
                        // non-clobbering for ids already in
                        // active[]; `apply_event` also
                        // ignores any payload for an id
                        // already in recent[] so a transfer
                        // that completed in the race window
                        // and is captured in BOTH the
                        // snapshot.recent and the stream's
                        // buffered Started+Complete doesn't
                        // duplicate.
                        if let Some(rx) = event_rx.as_mut() {
                            drain_startup_events(rx, &mut state, &mut status);
                        }
                    }
                    Err(err) => {
                        status = ConnectionStatus::Degraded(err);
                        parsed_remote = Some(endpoint);
                    }
                }
            }
            Err(err) => {
                status = ConnectionStatus::Degraded(format!("parse '{remote_str}': {err}"));
            }
        }
    }

    loop {
        terminal
            .draw(|frame| {
                screens::f2::render(frame, &state, &remote_label, &status);
            })
            .context("terminal.draw")?;

        if let Some(rx) = event_rx.as_mut() {
            tokio::select! {
                // Keystroke path.
                key = key_rx.recv() => {
                    let Some(key) = key else {
                        // Input task dropped its sender —
                        // unexpected (it loops until tx fails),
                        // treat as a clean exit.
                        return Ok(());
                    };
                    if let Some(action) = key_action(&key) {
                        match action {
                            UserAction::Quit => return Ok(()),
                            UserAction::Refresh => {
                                if let Some(endpoint) = parsed_remote.as_ref() {
                                    refresh_via_get_state(endpoint, &mut state, &mut status).await;
                                }
                            }
                            // F2 has no cursor — ignore.
                            UserAction::SelectNext
                            | UserAction::SelectPrev
                            | UserAction::Descend
                            | UserAction::Ascend => {}
                        }
                    }
                }
                // Subscribe stream path.
                event = rx.recv() => {
                    match event {
                        Some(EventOrError::Connected) => {
                            // Stream open. Same rule as the
                            // startup drain: only transition
                            // Connecting → Live; preserve any
                            // existing Degraded that came from
                            // a failed initial snapshot.
                            if matches!(status, ConnectionStatus::Connecting) {
                                status = ConnectionStatus::Live;
                            }
                        }
                        Some(EventOrError::Event(daemon_event)) => {
                            state.apply_event(daemon_event);
                            // First event also confirms Live
                            // (defensive: if Connected was
                            // dropped due to mpsc backpressure
                            // we still flip out of Connecting).
                            if matches!(status, ConnectionStatus::Connecting) {
                                status = ConnectionStatus::Live;
                            }
                        }
                        Some(EventOrError::Error(msg)) => {
                            status = ConnectionStatus::Degraded(msg);
                            event_rx = None;
                        }
                        None => {
                            // Forwarder dropped its sender —
                            // stream task exited. Surface the
                            // degraded status and stop reading.
                            status = ConnectionStatus::Degraded(
                                "subscribe stream closed".to_string(),
                            );
                            event_rx = None;
                        }
                    }
                }
            }
        } else {
            // No live stream — only the keystroke path is
            // active.
            let Some(key) = key_rx.recv().await else {
                return Ok(());
            };
            if let Some(action) = key_action(&key) {
                match action {
                    UserAction::Quit => return Ok(()),
                    UserAction::Refresh => {
                        if let Some(endpoint) = parsed_remote.as_ref() {
                            refresh_via_get_state(endpoint, &mut state, &mut status).await;
                        }
                    }
                    // F2 has no cursor — ignore.
                    UserAction::SelectNext
                    | UserAction::SelectPrev
                    | UserAction::Descend
                    | UserAction::Ascend => {}
                }
            }
        }
    }
}

/// F1 Daemons event loop. Drives the mDNS discovery task
/// and renders [`DaemonsState`] on every loop tick.
///
/// No `--remote` parsing here — F1 lists every daemon the
/// network mDNS-advertises. The operator picks one and the
/// future browse / trigger panes wire that selection
/// through (later A.1 sub-slices + a1-6 routing).
async fn run_f1_event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let mut state = DaemonsState::new();

    let (key_tx, mut key_rx) = mpsc::channel::<KeyEvent>(16);
    spawn_input_task(key_tx);

    let (disco_tx, mut disco_rx) = mpsc::channel::<DiscoveryUpdate>(4);
    let (refresh_tx, refresh_rx) = mpsc::channel::<()>(1);
    spawn_discovery_task(
        F1_DISCOVERY_INTERVAL,
        F1_DISCOVERY_SCAN_TIMEOUT,
        refresh_rx,
        disco_tx,
    );

    // a1-3b: per-row GetState detail fetcher. Each task
    // tags its result with the `instance_name` it was
    // fetching so a stale response (selection changed while
    // in flight) can be silently dropped by the apply step
    // below.
    let (detail_tx, mut detail_rx) = mpsc::channel::<DetailUpdate>(F1_DETAIL_BUFFER);
    // Last `instance_name` we kicked a fetch for. Used to
    // dedup; the kick helper compares this against the
    // currently selected row.
    let mut last_fetched: Option<String> = None;

    loop {
        // Kick a detail fetch whenever the selection moved
        // since the last kick. Cheap: this is a pointer
        // compare against the cached Option<String>.
        maybe_kick_detail_fetch(&mut state, &mut last_fetched, &detail_tx);

        let now = Instant::now();
        terminal
            .draw(|frame| {
                screens::f1::render(frame, &state, now);
            })
            .context("terminal.draw")?;

        tokio::select! {
            key = key_rx.recv() => {
                let Some(key) = key else { return Ok(()); };
                if let Some(action) = key_action(&key) {
                    match action {
                        UserAction::Quit => return Ok(()),
                        UserAction::Refresh => {
                            // Non-blocking nudge to the discovery
                            // task. If the channel is full a scan
                            // is already pending — silently drop,
                            // the queued tick will satisfy us.
                            let _ = refresh_tx.try_send(());
                            // Invalidate the selected row's
                            // cached detail so the next kick
                            // fires fresh. invalidate_detail
                            // also bumps the row's request_id
                            // so any in-flight reply from
                            // before the invalidation is
                            // dropped on arrival.
                            if let Some(name) = state
                                .selected_row()
                                .map(|r| r.instance_name.clone())
                            {
                                state.invalidate_detail(&name);
                            }
                            last_fetched = None;
                        }
                        UserAction::SelectNext => state.select_next(),
                        UserAction::SelectPrev => state.select_prev(),
                        // F1 doesn't have tree navigation
                        // semantics today. Future a1-6 routing
                        // will repurpose Enter to switch panes;
                        // for now both Descend/Ascend are no-ops.
                        UserAction::Descend | UserAction::Ascend => {}
                    }
                }
            }
            update = disco_rx.recv() => {
                match update {
                    Some(DiscoveryUpdate::Result(services)) => {
                        state.replace_from_discovery(&services, Instant::now());
                    }
                    Some(DiscoveryUpdate::Error(msg)) => {
                        state.note_discovery_error(msg);
                    }
                    None => {
                        // Discovery task exited unexpectedly.
                        state.note_discovery_error(
                            "discovery task exited".to_string(),
                        );
                    }
                }
            }
            update = detail_rx.recv() => {
                if let Some(DetailUpdate { instance_name, request_id, result }) = update {
                    let detail = match result {
                        Ok(daemon_state) => DaemonDetail::Loaded {
                            state: Box::new(daemon_state),
                            fetched_at: Instant::now(),
                        },
                        Err(message) => DaemonDetail::Error { message },
                    };
                    // apply_detail_update returns false if
                    // the row's request_id has moved on since
                    // we spawned this fetch — in that case
                    // the stale result is dropped on the
                    // floor (a newer fetch is in flight or
                    // already returned).
                    state.apply_detail_update(&instance_name, request_id, detail);
                }
                // If the channel closed (None) we just
                // continue — the loop survives.
            }
        }
    }
}

/// If the selected row's name differs from `last_fetched`,
/// decide whether to spawn a fresh `GetState` fetch.
///
/// Cache contract (a1-3b round 2): an existing cached
/// detail entry — `Loaded`, `Pending`, OR `Error` — is
/// treated as "already covered for this row." Cursoring
/// off and back onto a row whose detail was previously
/// loaded must NOT replace the loaded data with `Pending`
/// or spawn another RPC. The `r` keystroke is the only
/// non-discovery path that invalidates a cached entry
/// (via [`DaemonsState::invalidate_detail`]).
fn maybe_kick_detail_fetch(
    state: &mut DaemonsState,
    last_fetched: &mut Option<String>,
    detail_tx: &mpsc::Sender<DetailUpdate>,
) {
    let Some(row) = state.selected_row() else {
        return;
    };
    let name = row.instance_name.clone();
    if last_fetched.as_deref() == Some(name.as_str()) {
        return;
    }
    // Already have a cached entry for this row — just track
    // the visit, don't refetch. Keeps `Loaded` data on
    // screen when the operator returns to a previously
    // viewed row; avoids redundant RPCs.
    if state.detail_for(&name).is_some() {
        *last_fetched = Some(name);
        return;
    }
    let Some(endpoint) = DaemonsState::endpoint_for_row(row) else {
        // No usable endpoint (remote row with no advertised
        // address). Mark as Error so the operator sees why.
        state.set_detail(
            name.clone(),
            DaemonDetail::Error {
                message: "no advertised address".to_string(),
            },
        );
        *last_fetched = Some(name);
        return;
    };
    // begin_fetch bumps the row's request_id, sets Pending,
    // and hands us the id to embed in the spawn so a stale
    // reply from a prior generation can be discarded.
    let request_id = state.begin_fetch(&name);
    spawn_detail_fetch(endpoint, name.clone(), request_id, detail_tx.clone());
    *last_fetched = Some(name);
}

/// One-shot GetState fetcher. The reply carries the
/// row name AND the request id from `begin_fetch`, so the
/// apply arm can drop a stale generation's result without
/// clobbering whatever a newer fetch already wrote.
fn spawn_detail_fetch(
    endpoint: RemoteEndpoint,
    instance_name: String,
    request_id: u64,
    tx: mpsc::Sender<DetailUpdate>,
) {
    tokio::spawn(async move {
        let result = jobs::query(&endpoint, 0)
            .await
            .map_err(|err| format!("{err:#}"));
        let _ = tx
            .send(DetailUpdate {
                instance_name,
                request_id,
                result,
            })
            .await;
    });
}

/// Reply envelope for the detail fetcher.
struct DetailUpdate {
    instance_name: String,
    request_id: u64,
    result: Result<DaemonState, String>,
}

/// Bounded channel depth for detail-fetch replies. 8 is
/// generous — at most one fetch per selection change, and
/// the loop consumes them faster than the operator can
/// cursor through rows.
const F1_DETAIL_BUFFER: usize = 8;

/// F3 Browse event loop. Lists modules at top level; cursor +
/// Enter descends into a module's root and then into its
/// directories via the `List` RPC.
///
/// `--remote` is required (operator-mode browsing requires a
/// daemon target). On missing or invalid endpoint the loop
/// renders the F3 frame with an error stats line and accepts
/// only `q` to quit.
async fn run_f3_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    remote_arg: Option<&str>,
) -> Result<()> {
    let mut state = BrowseState::new();
    let remote_label = remote_arg.unwrap_or("(no remote)").to_string();

    let (key_tx, mut key_rx) = mpsc::channel::<KeyEvent>(16);
    spawn_input_task(key_tx);

    // Parse the remote up-front so a malformed value lands
    // in the stats banner instead of bouncing back to a hard
    // crash. Missing flag is fatal for browsing (we have
    // nothing to list) — keep the loop alive so the operator
    // can read the message and quit.
    let endpoint = match remote_arg {
        Some(raw) => match RemoteEndpoint::parse(raw) {
            Ok(ep) => Some(ep),
            Err(err) => {
                state.note_fetch_error(format!("parse '{raw}': {err}"));
                None
            }
        },
        None => {
            state.note_fetch_error("--remote <host> is required for F3 Browse".to_string());
            None
        }
    };

    let (fetch_tx, mut fetch_rx) = mpsc::channel::<BrowseFetchReply>(8);
    // Tracks what fetch we last kicked: whichever view the
    // request was issued for. Comparing this against the
    // current `state.view()` lets us skip duplicate kicks
    // (operator pressing arrows without descending).
    let mut last_fetched_view: Option<browse::BrowseView> = None;

    loop {
        // Decide whether to kick a fetch. Only kicks when:
        // - We have an endpoint.
        // - The current view differs from the last-kicked.
        if endpoint.is_some()
            && views_differ(last_fetched_view.as_ref(), state.view())
            && matches!(
                state.status(),
                browse::BrowseFetchStatus::Idle | browse::BrowseFetchStatus::Error { .. }
            )
        {
            if let Some(ep) = endpoint.as_ref() {
                kick_browse_fetch(&mut state, ep, &fetch_tx);
                last_fetched_view = Some(state.view().clone());
            }
        }

        let now = Instant::now();
        let label = remote_label.clone();
        terminal
            .draw(|frame| {
                screens::f3::render(frame, &state, &label, now);
            })
            .context("terminal.draw")?;

        tokio::select! {
            key = key_rx.recv() => {
                let Some(key) = key else { return Ok(()); };
                if let Some(action) = key_action(&key) {
                    match action {
                        UserAction::Quit => return Ok(()),
                        UserAction::Refresh => {
                            // Drop last_fetched_view so the next
                            // iteration's kick fires.
                            last_fetched_view = None;
                            // Bump generation so any in-flight
                            // reply is dropped on arrival.
                            state.begin_fetch();
                            // begin_fetch sets Pending; the kick
                            // path uses Idle/Error as triggers,
                            // so reset to Idle here to re-fire.
                            // (Refresh is the only path that
                            // bumps generation without descending.)
                            state.note_fetch_error("refreshing".to_string());
                        }
                        UserAction::SelectNext => state.select_next(),
                        UserAction::SelectPrev => state.select_prev(),
                        UserAction::Descend => {
                            // descend mutates state.view; the
                            // next loop iteration's check
                            // detects the new view and kicks.
                            state.descend();
                        }
                        UserAction::Ascend => {
                            state.ascend();
                        }
                    }
                }
            }
            reply = fetch_rx.recv() => {
                if let Some(reply) = reply {
                    apply_browse_reply(&mut state, reply);
                }
            }
        }
    }
}

fn views_differ(prior: Option<&browse::BrowseView>, current: &browse::BrowseView) -> bool {
    match (prior, current) {
        (None, _) => true,
        (Some(browse::BrowseView::Modules), browse::BrowseView::Modules) => false,
        (
            Some(browse::BrowseView::Module { name: a, path: ap }),
            browse::BrowseView::Module { name: b, path: bp },
        ) => a != b || ap != bp,
        _ => true,
    }
}

/// Kick a fetch for the current view. Bumps the per-view
/// request id and spawns either a `list_modules` or
/// `list` RPC task depending on the view shape.
fn kick_browse_fetch(
    state: &mut BrowseState,
    endpoint: &RemoteEndpoint,
    fetch_tx: &mpsc::Sender<BrowseFetchReply>,
) {
    let request_id = state.begin_fetch();
    let endpoint = endpoint.clone();
    let view = state.view().clone();
    let tx = fetch_tx.clone();
    tokio::spawn(async move {
        let payload = match &view {
            browse::BrowseView::Modules => match list_modules::query(&endpoint).await {
                Ok(modules) => BrowseFetchPayload::Modules(modules),
                Err(err) => BrowseFetchPayload::Error(format!("{err:#}")),
            },
            browse::BrowseView::Module { name, path } => {
                let path_str = path.join("/");
                match ls::list_remote(&endpoint, name.clone(), path_str).await {
                    Ok(entries) => BrowseFetchPayload::Listing {
                        module: name.clone(),
                        path: path.clone(),
                        entries,
                    },
                    Err(err) => BrowseFetchPayload::Error(format!("{err:#}")),
                }
            }
        };
        let _ = tx
            .send(BrowseFetchReply {
                request_id,
                payload,
            })
            .await;
    });
}

fn apply_browse_reply(state: &mut BrowseState, reply: BrowseFetchReply) {
    if !state.is_current_request(reply.request_id) {
        // Stale generation — drop. A newer fetch is in
        // flight (or already returned).
        return;
    }
    let now = Instant::now();
    match reply.payload {
        BrowseFetchPayload::Modules(modules) => {
            state.apply_modules(modules, now);
        }
        BrowseFetchPayload::Listing {
            module,
            path,
            entries,
        } => {
            state.apply_listing(&module, &path, entries, now);
        }
        BrowseFetchPayload::Error(message) => {
            state.note_fetch_error(message);
        }
    }
}

struct BrowseFetchReply {
    request_id: u64,
    payload: BrowseFetchPayload,
}

enum BrowseFetchPayload {
    Modules(Vec<Module>),
    Listing {
        module: String,
        path: Vec<String>,
        entries: Vec<DirEntry>,
    },
    Error(String),
}

/// Messages from the discovery task back to the F1 loop.
enum DiscoveryUpdate {
    Result(Vec<MdnsDiscoveredService>),
    Error(String),
}

/// Spawn the F1 discovery task. Loops on a tokio interval,
/// running one-shot mDNS discovery each tick, forwarding
/// results (or errors) via `tx`. Accepts manual-refresh
/// pokes via `refresh_rx` — those simply break out of the
/// `interval.tick()` wait and re-scan immediately.
fn spawn_discovery_task(
    interval: Duration,
    scan_timeout: Duration,
    mut refresh_rx: mpsc::Receiver<()>,
    tx: mpsc::Sender<DiscoveryUpdate>,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // The first tick fires immediately, which is what we
        // want — operator gets a result on screen-open.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = ticker.tick() => {}
                signal = refresh_rx.recv() => {
                    if signal.is_none() {
                        // Caller dropped the refresh sender —
                        // F1 loop is exiting; we should too.
                        return;
                    }
                    // Reset the ticker so the next automatic
                    // scan is `interval` away from this manual
                    // one (avoids two back-to-back scans).
                    ticker.reset();
                }
            }
            match scan::discover(scan_timeout).await {
                Ok(services) => {
                    if tx.send(DiscoveryUpdate::Result(services)).await.is_err() {
                        // Receiver closed — F1 loop exited.
                        return;
                    }
                }
                Err(err) => {
                    let _ = tx.send(DiscoveryUpdate::Error(format!("{err:#}"))).await;
                }
            }
        }
    });
}

/// Single-owner crossterm input task. One spawn_blocking
/// task loops over `event::poll`/`event::read` and forwards
/// every key press through `tx`. Exits when the receiver
/// drops (TUI quitting) — observed via `blocking_send`
/// returning Err.
///
/// Solves the a1-2 round-1 leak: each loop iteration there
/// spawned a fresh spawn_blocking that could detach and
/// independently consume a keystroke when the select arm
/// preferred the Subscribe stream. With a single owner,
/// keystrokes always reach the main loop in order.
fn spawn_input_task(tx: mpsc::Sender<KeyEvent>) {
    tokio::task::spawn_blocking(move || loop {
        match event::poll(Duration::from_millis(EVENT_POLL_INTERVAL_MS)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    let local = KeyEvent {
                        code: key.code,
                        modifiers: key.modifiers,
                    };
                    if tx.blocking_send(local).is_err() {
                        // Receiver dropped — TUI exiting.
                        return;
                    }
                }
                Ok(_) => {
                    // Non-key event (resize, mouse, …) —
                    // ignored for now.
                }
                Err(_) => return,
            },
            Ok(false) => {
                // poll timeout; check whether the receiver
                // is still alive so we don't loop forever
                // after a TUI quit during quiet input.
                if tx.is_closed() {
                    return;
                }
            }
            Err(_) => return,
        }
    });
}

/// Action surfaced by `key_action` back to the loop.
/// `Quit` and `Refresh` are shared across screens; the
/// other variants are pane-specific (F2 ignores all
/// navigation today).
enum UserAction {
    Quit,
    Refresh,
    SelectNext,
    SelectPrev,
    /// F3: descend into the cursor row (enter / →).
    Descend,
    /// F3: pop back one level (←). Mapped only on the
    /// dedicated key; q/Esc remain Quit.
    Ascend,
}

/// Lightweight key-event copy. Avoids carrying a
/// `crossterm::event::KeyEvent` across `spawn_blocking`
/// boundaries (which would otherwise pull in lifetimes we
/// don't want).
struct KeyEvent {
    code: KeyCode,
    modifiers: KeyModifiers,
}

/// Classify a key press as a recognized user action, or
/// `None` if the key is one we ignore. Pure function so
/// tests can pin the keymap without spinning up an input
/// task.
fn key_action(key: &KeyEvent) -> Option<UserAction> {
    if should_quit(key.code, key.modifiers) {
        return Some(UserAction::Quit);
    }
    match key.code {
        KeyCode::Char('r') => Some(UserAction::Refresh),
        KeyCode::Down | KeyCode::Char('j') => Some(UserAction::SelectNext),
        KeyCode::Up | KeyCode::Char('k') => Some(UserAction::SelectPrev),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Some(UserAction::Descend),
        KeyCode::Left | KeyCode::Char('h') => Some(UserAction::Ascend),
        _ => None,
    }
}

/// Drain whatever events buffered in `rx` during the
/// subscribe→snapshot startup window and apply them onto
/// `state`. Returns once `try_recv` reports the channel
/// empty (or disconnected — surfaced via Degraded status).
///
/// Closes the a1-2 round-3 race: subscribe registered the
/// broadcast Receiver early, so a transfer that Started
/// after subscribe but before the snapshot was applied
/// lands in this buffer. Replaying onto the snapshot makes
/// the state consistent before the main select loop takes
/// over.
fn drain_startup_events(
    rx: &mut mpsc::Receiver<EventOrError>,
    state: &mut TransfersState,
    status: &mut ConnectionStatus,
) {
    use tokio::sync::mpsc::error::TryRecvError;
    loop {
        match rx.try_recv() {
            Ok(EventOrError::Connected) => {
                // Connected is a stream-health signal, not a
                // snapshot-health signal. Only let it
                // transition Connecting → Live. If the
                // caller already set Degraded (e.g. the
                // initial GetState failed) we must not
                // overwrite that — the live stream may be
                // healthy but the active/recent state is
                // incomplete and the user needs to know.
                if matches!(status, ConnectionStatus::Connecting) {
                    *status = ConnectionStatus::Live;
                }
            }
            Ok(EventOrError::Event(event)) => {
                state.apply_event(event);
                // Same rule as Connected: first event is a
                // stream-health signal. Don't paper over an
                // existing Degraded snapshot status.
                if matches!(status, ConnectionStatus::Connecting) {
                    *status = ConnectionStatus::Live;
                }
            }
            Ok(EventOrError::Error(msg)) => {
                *status = ConnectionStatus::Degraded(msg);
                // Continue draining — we don't expect more
                // events after Error in practice (forwarder
                // exits) but a benign extra Connected
                // before Error shouldn't trip us up.
            }
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return,
        }
    }
}

/// Re-issue a `GetState` query and replace local state.
/// Triggered by the 'r' keystroke; surfaces failures as
/// Degraded status instead of aborting the loop.
async fn refresh_via_get_state(
    endpoint: &RemoteEndpoint,
    state: &mut TransfersState,
    status: &mut ConnectionStatus,
) {
    match jobs::query(endpoint, 0).await {
        Ok(snapshot) => {
            state.replace_from_snapshot(snapshot);
            *status = ConnectionStatus::Live;
        }
        Err(err) => {
            *status = ConnectionStatus::Degraded(format!("refresh failed: {err}"));
        }
    }
}

/// Control / data messages from the Subscribe forwarder.
/// `Connected` is sent once after the subscribe RPC returns
/// successfully so the TUI can flip out of "Connecting"
/// without waiting for the first event.
enum EventOrError {
    Connected,
    Event(DaemonEvent),
    Error(String),
}

/// Open the Subscribe stream synchronously (awaited inline)
/// and spawn the forwarder task. Returns the mpsc Receiver
/// only AFTER the subscribe RPC has succeeded — i.e. the
/// daemon-side broadcast Receiver is already registered.
///
/// a1-2 round 4 fix: the previous shape called
/// `tokio::spawn(async move { subscribe(...).await ... })`
/// and returned immediately. The spawned task still had to
/// connect before the daemon would register its receiver.
/// During that gap a `GetState` could complete and a
/// transfer could start without ever being buffered.
///
/// By awaiting the subscribe RPC inline here, the caller
/// can be confident that when this function returns OK, the
/// daemon's broadcast is sending into the forwarder's
/// receiver.
async fn open_subscribe_stream(
    endpoint: &RemoteEndpoint,
) -> Result<mpsc::Receiver<EventOrError>, String> {
    let stream = jobs::subscribe(endpoint, "", false)
        .await
        .map_err(|err| format!("subscribe: {err}"))?;
    let (tx, rx) = mpsc::channel::<EventOrError>(TUI_EVENT_BUFFER);
    // Send Connected immediately — subscribe() has returned
    // OK so the daemon broadcast receiver is registered.
    let _ = tx.send(EventOrError::Connected).await;
    tokio::spawn(forward_subscribe_stream(stream, tx));
    Ok(rx)
}

/// Inner loop of the Subscribe forwarder task. Reads
/// `stream.message()` and forwards events into `tx` until
/// the stream ends, errors, or `tx` reports a closed
/// receiver. Factored out of `open_subscribe_stream` so the
/// spawn site is a single function call.
async fn forward_subscribe_stream(
    mut stream: tonic::Streaming<DaemonEvent>,
    tx: mpsc::Sender<EventOrError>,
) {
    loop {
        match stream.message().await {
            Ok(Some(event)) => {
                if tx.send(EventOrError::Event(event)).await.is_err() {
                    return;
                }
            }
            Ok(None) => {
                let _ = tx
                    .send(EventOrError::Error("stream ended".to_string()))
                    .await;
                return;
            }
            Err(status) => {
                let _ = tx
                    .send(EventOrError::Error(format!("stream: {}", status.message())))
                    .await;
                return;
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

    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }

    #[test]
    fn key_action_maps_quit_and_refresh() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('q'))),
            Some(UserAction::Quit)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Esc)),
            Some(UserAction::Quit)
        ));
        assert!(matches!(
            key_action(&KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }),
            Some(UserAction::Quit)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('r'))),
            Some(UserAction::Refresh)
        ));
    }

    /// a1-4: F3 navigation keys. Enter / → / 'l' descend
    /// into the cursor row; ← / 'h' ascend. Verifies the
    /// full set lands in the right variant.
    #[test]
    fn key_action_maps_f3_navigation() {
        assert!(matches!(
            key_action(&k(KeyCode::Enter)),
            Some(UserAction::Descend)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Right)),
            Some(UserAction::Descend)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('l'))),
            Some(UserAction::Descend)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Left)),
            Some(UserAction::Ascend)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('h'))),
            Some(UserAction::Ascend)
        ));
    }

    /// a1-3: F1 needs cursor navigation keys. Both arrows and
    /// vim-style hjkl bindings are accepted (operators who
    /// haven't broken the habit get both). 'j' / 'k' are
    /// case-sensitive — uppercase remains unmapped.
    #[test]
    fn key_action_maps_arrow_and_vim_navigation() {
        assert!(matches!(
            key_action(&k(KeyCode::Down)),
            Some(UserAction::SelectNext)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Up)),
            Some(UserAction::SelectPrev)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('j'))),
            Some(UserAction::SelectNext)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('k'))),
            Some(UserAction::SelectPrev)
        ));
    }

    #[test]
    fn key_action_returns_none_for_unmapped_keys() {
        assert!(key_action(&k(KeyCode::Char('a'))).is_none());
        assert!(key_action(&k(KeyCode::Char('R'))).is_none()); // case-sensitive
        assert!(key_action(&k(KeyCode::Char('J'))).is_none()); // case-sensitive
        assert!(key_action(&k(KeyCode::Char('K'))).is_none()); // case-sensitive
                                                               // Enter is now mapped (a1-4: F3 Descend) — it
                                                               // *isn't* in this "unmapped" list anymore.
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

    /// a1-2 round-5 regression: when the initial `GetState`
    /// fails but Subscribe is healthy, the buffered
    /// `Connected` message must NOT overwrite the
    /// `Degraded(...)` status set by the snapshot failure.
    /// Connected is a stream-health signal — it cannot tell
    /// the user the active/recent rows are complete.
    #[tokio::test]
    async fn drain_startup_events_connected_preserves_degraded() {
        let (tx, mut rx) = mpsc::channel::<EventOrError>(4);
        // Forwarder always pushes Connected first.
        tx.send(EventOrError::Connected).await.unwrap();
        drop(tx);

        let mut state = TransfersState::new();
        let mut status = ConnectionStatus::Degraded("initial GetState failed: timeout".to_string());
        drain_startup_events(&mut rx, &mut state, &mut status);
        match status {
            ConnectionStatus::Degraded(msg) => {
                assert!(msg.contains("initial GetState failed"));
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    /// Companion case: when the prior status is `Connecting`
    /// (normal happy path — snapshot succeeded, drain runs
    /// next), Connected SHOULD flip to Live.
    #[tokio::test]
    async fn drain_startup_events_connected_flips_connecting_to_live() {
        let (tx, mut rx) = mpsc::channel::<EventOrError>(4);
        tx.send(EventOrError::Connected).await.unwrap();
        drop(tx);

        let mut state = TransfersState::new();
        let mut status = ConnectionStatus::Connecting;
        drain_startup_events(&mut rx, &mut state, &mut status);
        assert!(matches!(status, ConnectionStatus::Live));
    }

    /// a1-3b round-2 regression: cursor flick onto a row,
    /// off, then back — must NOT re-spawn a fetch or
    /// overwrite the previously loaded detail with Pending.
    /// The cache contract (per the reviewer) is "an
    /// existing entry covers the row until the operator
    /// hits `r`."
    ///
    /// Driven directly through `maybe_kick_detail_fetch` —
    /// no real spawn happens because the detail_tx is just
    /// observed for emptiness afterwards.
    #[tokio::test]
    async fn maybe_kick_detail_fetch_preserves_loaded_on_revisit() {
        use crate::daemons::EndpointKind;
        use blit_core::generated::DaemonState as WireState;

        let mut state = DaemonsState::new();
        // Manually inject a remote row "alpha".
        let alpha = crate::daemons::DaemonRow {
            kind: EndpointKind::Remote,
            instance_name: "alpha".to_string(),
            addresses: vec![std::net::Ipv4Addr::new(10, 0, 0, 1)],
            port: 9031,
            module_count: None,
            delegation_enabled: None,
            version: None,
            modules: Vec::new(),
        };
        // Build a row vec by going through the public API:
        // discover one daemon, then patch its row to alpha.
        // Simpler: directly stuff via replace_from_discovery
        // with a synthetic service.
        use blit_core::mdns::MdnsDiscoveredService;
        use std::collections::HashMap;
        state.replace_from_discovery(
            &[MdnsDiscoveredService {
                fullname: "alpha._blit._tcp.local.".to_string(),
                instance_name: "alpha".to_string(),
                hostname: "alpha.local.".to_string(),
                port: 9031,
                addresses: vec![std::net::Ipv4Addr::new(10, 0, 0, 1)],
                properties: HashMap::new(),
            }],
            std::time::Instant::now(),
        );
        let _ = alpha; // shut unused-binding analysis up.

        // Move cursor onto alpha (Local @ 0, alpha @ 1).
        state.select_next();
        assert_eq!(state.selected_row().unwrap().instance_name, "alpha");

        // Pre-load a detail for alpha (simulating a prior fetch returning).
        let prior_state = WireState {
            version: "9.9.9".to_string(),
            ..WireState::default()
        };
        state.set_detail(
            "alpha".to_string(),
            DaemonDetail::Loaded {
                state: Box::new(prior_state),
                fetched_at: Instant::now(),
            },
        );

        let (detail_tx, mut detail_rx) = mpsc::channel::<DetailUpdate>(8);
        let mut last_fetched: Option<String> = None;

        // Cursor onto alpha → kick checks for cached entry,
        // finds Loaded, just bumps last_fetched.
        maybe_kick_detail_fetch(&mut state, &mut last_fetched, &detail_tx);
        assert_eq!(last_fetched.as_deref(), Some("alpha"));
        assert!(matches!(
            state.detail_for("alpha"),
            Some(DaemonDetail::Loaded { .. })
        ));
        // No spawn → no message on detail_tx.
        assert!(detail_rx.try_recv().is_err());

        // Cursor off (back to Local) and back onto alpha.
        state.select_prev();
        last_fetched = Some("local (this machine)".to_string());
        state.select_next();
        maybe_kick_detail_fetch(&mut state, &mut last_fetched, &detail_tx);
        assert_eq!(last_fetched.as_deref(), Some("alpha"));
        // STILL Loaded — not Pending, not overwritten.
        match state.detail_for("alpha") {
            Some(DaemonDetail::Loaded { state, .. }) => {
                assert_eq!(state.version, "9.9.9");
            }
            other => panic!("expected preserved Loaded, got {other:?}"),
        }
        assert!(detail_rx.try_recv().is_err());
    }

    /// a1-4: views_differ is the trigger predicate for
    /// the F3 fetcher. None / different views → true;
    /// equal views → false.
    #[test]
    fn views_differ_module_path_compare() {
        use crate::browse::BrowseView;

        let modules = BrowseView::Modules;
        let home_root = BrowseView::Module {
            name: "home".to_string(),
            path: Vec::new(),
        };
        let home_photos = BrowseView::Module {
            name: "home".to_string(),
            path: vec!["photos".to_string()],
        };

        // None → any non-None is "different."
        assert!(views_differ(None, &modules));
        assert!(views_differ(None, &home_root));

        // Same view → false.
        assert!(!views_differ(Some(&modules), &modules));
        assert!(!views_differ(Some(&home_root), &home_root));
        assert!(!views_differ(Some(&home_photos), &home_photos));

        // Different views.
        assert!(views_differ(Some(&modules), &home_root));
        assert!(views_differ(Some(&home_root), &home_photos));
    }

    /// Companion: when no cached detail exists, the kick
    /// DOES set Pending and (would) spawn — for the test
    /// we just verify Pending lands and the request_id was
    /// bumped (via begin_fetch's contract).
    #[tokio::test]
    async fn maybe_kick_detail_fetch_spawns_when_cache_empty() {
        use blit_core::mdns::MdnsDiscoveredService;
        use std::collections::HashMap;

        let mut state = DaemonsState::new();
        state.replace_from_discovery(
            &[MdnsDiscoveredService {
                fullname: "alpha._blit._tcp.local.".to_string(),
                instance_name: "alpha".to_string(),
                hostname: "alpha.local.".to_string(),
                port: 9031,
                addresses: vec![std::net::Ipv4Addr::new(10, 0, 0, 1)],
                properties: HashMap::new(),
            }],
            std::time::Instant::now(),
        );
        state.select_next();

        let (detail_tx, _detail_rx) = mpsc::channel::<DetailUpdate>(8);
        let mut last_fetched: Option<String> = None;
        maybe_kick_detail_fetch(&mut state, &mut last_fetched, &detail_tx);
        assert_eq!(last_fetched.as_deref(), Some("alpha"));
        assert!(matches!(
            state.detail_for("alpha"),
            Some(DaemonDetail::Pending)
        ));
    }
}
