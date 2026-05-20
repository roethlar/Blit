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
mod config;
mod daemons;
mod diagnostics;
mod f3pull;
mod help;
mod profile;
mod screens;
mod state;
mod transfer;
mod verify;

use blit_app::admin::list_modules::Module;
use blit_app::admin::ls::DirEntry;
use blit_app::admin::{jobs, list_modules, ls};
use blit_app::profile as app_profile;
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

    /// Initial pane to open. With a1-6 routing in place,
    /// the operator can switch panes via F1..F4 keys at
    /// any time — this flag just picks the starting pane.
    /// Defaults to F1 (Daemons) since that's the natural
    /// entry point: scan the LAN, pick a daemon, then
    /// drill into F2/F3/F4 from there.
    #[arg(long, value_enum, default_value_t = ScreenArg::F1)]
    screen: ScreenArg,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum ScreenArg {
    F1,
    F2,
    F3,
    F4,
}

/// In-app pane identifier. Distinct from `ScreenArg`
/// because (a) the CLI value-enum maps to lowercase
/// `f1`/`f2`/etc. for clap, while we want PascalCase
/// values in code, and (b) `ScreenArg` may grow CLI-only
/// variants in the future (e.g. a Help screen) that don't
/// have an F-key.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    F1,
    F2,
    F3,
    F4,
}

impl From<ScreenArg> for Screen {
    fn from(arg: ScreenArg) -> Self {
        match arg {
            ScreenArg::F1 => Screen::F1,
            ScreenArg::F2 => Screen::F2,
            ScreenArg::F3 => Screen::F3,
            ScreenArg::F4 => Screen::F4,
        }
    }
}

/// Cross-pane state aggregator (a1-6b). Holds every pane's
/// state, the per-pane bookkeeping bits (cursor-tracking
/// names, view paths, statuses), and the senders for
/// background-task replies. Receivers + the senders the
/// background tasks themselves use stay in the router's
/// local scope and get borrowed into each pane loop.
///
/// State preservation: `AppState` lives for the whole TUI
/// session, so navigating away from a pane and back keeps
/// the existing data — F1's discovered rows + cursor, F2's
/// active/recent tables, F3's tree position, F4's loaded
/// profile.
///
/// Background-task preservation: the discovery task,
/// Subscribe stream, and per-pane reply channels are
/// spawned once by `run_router` and stay alive across
/// navigations. Pane loops just borrow the receivers.
struct AppState {
    /// Active pane. Changes via F-key navigation.
    current_screen: Screen,
    /// Shared remote endpoint (parsed once at startup).
    parsed_remote: Option<RemoteEndpoint>,
    /// Display label for the remote (raw user input or
    /// "(no remote)").
    remote_label: String,

    // F1
    daemons: DaemonsState,
    daemons_last_fetched: Option<String>,
    /// Sender into the F1 detail fetcher mpsc. Cloned by
    /// `maybe_kick_detail_fetch` into each spawned task.
    detail_tx: mpsc::Sender<DetailUpdate>,
    /// Sender for the F1 discovery refresh-trigger channel
    /// (bounded(1)). Operator's `r` keystroke nudges this.
    discovery_refresh_tx: mpsc::Sender<()>,

    // F2
    transfers: TransfersState,
    transfers_status: ConnectionStatus,
    /// Generation counter for F2 setup tasks. Bumped each
    /// time `spawn_f2_setup_task` is called; the reply
    /// envelope carries the same value, and the apply arm
    /// drops the reply if the generation has moved on.
    /// Same pattern as `DaemonsState::request_ids` for F1
    /// detail fetches.
    transfers_setup_gen: u64,
    /// True from the moment we spawn an F2 setup task until
    /// its reply lands. Refresh keystrokes consult this so
    /// pressing `r` while a setup is in flight doesn't
    /// spawn a duplicate.
    transfers_setup_pending: bool,

    // F3
    browse: BrowseState,
    browse_last_fetched_view: Option<browse::BrowseView>,
    /// Sender into the F3 browse fetcher mpsc. Cloned by
    /// `kick_browse_fetch` into each spawned task.
    browse_fetch_tx: mpsc::Sender<BrowseFetchReply>,

    // F4
    profile: profile::ProfileState,
    /// Sender into the F4 profile fetcher mpsc. Cloned by
    /// `spawn_profile_fetch` into each spawned task.
    profile_reply_tx: mpsc::Sender<ProfileReply>,
    /// F4 Verify form. Holds source/destination text, the
    /// current focus, and the most recent run's result.
    verify: verify::VerifyState,
    /// F4 Diagnostics dump state. Tracks the most recent
    /// snapshot's status (idle / running / done(path) /
    /// error). Operator triggers via `s`.
    diagnostics: diagnostics::DiagnosticsState,
    /// Sender into the F4 Diagnostics dump task mpsc.
    /// Cloned by `spawn_diagnostics_dump` into each
    /// spawned task.
    diagnostics_reply_tx: mpsc::Sender<DiagnosticsReply>,
    /// `?` help overlay. When visible, the overlay paints
    /// on top of the active pane and absorbs keystrokes
    /// (except `?`/Esc which close it). Visibility persists
    /// across F-key navigation.
    help: help::HelpOverlay,
    /// F4 local-transfer state. Operator triggers a
    /// copy / mirror via `C` / `M` once the Verify form's
    /// Source and Destination are filled in.
    transfer: transfer::TransferState,
    /// Sender into the F4 transfer reply mpsc.
    transfer_reply_tx: mpsc::Sender<TransferReply>,
    /// d-22: lifecycle of an F2 `K` cancel-selected
    /// request. `Idle` until the operator presses K with
    /// an anchored cursor; `Sending` while the CancelJob
    /// RPC is in flight; `Done`/`Error` once it lands.
    /// Status renders into the F2 footer.
    cancel_status: F2CancelStatus,
    cancel_reply_tx: mpsc::Sender<CancelReply>,
    cancel_request_seq: u64,
    /// d-35: F3 transfer-from-cursor pull lifecycle —
    /// destination prompt + remote→local PullSync owned
    /// by the TUI process. Renders into the F3 footer.
    f3_pull: f3pull::F3PullState,
    f3_pull_reply_tx: mpsc::Sender<F3PullReply>,
    /// d-37: live pull-progress snapshots from the running
    /// pull task.
    f3_pull_progress_tx: mpsc::Sender<F3PullProgress>,
    /// d-36: transient banner shown after a `Ctrl+R`
    /// config reload (`config reloaded` on success, the
    /// parse error on failure). Auto-hides via a
    /// renderer-side TTL — `None` once expired or never
    /// reloaded.
    reload_banner: Option<ReloadBanner>,
}

/// d-36: outcome of a `Ctrl+R` `tui.toml` reload, shown
/// briefly in the tab-strip line.
#[derive(Debug, Clone)]
struct ReloadBanner {
    message: String,
    /// `true` = success (green), `false` = parse error
    /// (red, current config kept).
    ok: bool,
    shown_at: Instant,
}

impl ReloadBanner {
    /// How long the banner stays on screen.
    const TTL: std::time::Duration = std::time::Duration::from_secs(4);

    /// `true` while the banner should still render.
    fn is_visible(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.shown_at) < Self::TTL
    }
}

/// d-22: F2 cancel-selected lifecycle. Lives on AppState
/// so the F2 footer can render whichever variant is
/// current. Same generation-guard pattern as the F4
/// transfer machinery — `Sending`'s `transfer_id` plus a
/// monotonic `request_id` let the reply arm drop a stale
/// reply if the operator fires a second cancel before
/// the first lands.
#[derive(Debug, Clone)]
enum F2CancelStatus {
    Idle,
    /// d-29: operator pressed `K` while
    /// `[transfer] confirm_cancel = true`. Waiting for
    /// `y` (kick the RPC) or `n` / `Esc` (revert to
    /// Idle). No TTL — the prompt stays until the
    /// operator answers.
    Confirming {
        transfer_id: String,
    },
    /// d-30: operator pressed `Shift+X` (batch cancel)
    /// while `[transfer] confirm_cancel = true` AND
    /// at least one active row was present. Waiting
    /// for `y` (fire cancel RPCs against the frozen
    /// ids) or `n` / `Esc` (revert to Idle).
    ///
    /// d-30 round 2: `transfer_ids` is captured at
    /// prompt creation, not on confirm. The Subscribe
    /// stream keeps mutating `transfers.active` while
    /// the prompt is up, so re-snapshotting on `y`
    /// would race — the operator could confirm
    /// "cancel 2 transfers" against A/B, then A/B
    /// complete and C/D start before they press `y`,
    /// and the pre-fix code would have cancelled C/D
    /// instead. Freezing the ids at prompt creation
    /// closes the race.
    ConfirmingBatch {
        transfer_ids: Vec<String>,
    },
    Sending {
        transfer_id: String,
        request_id: u64,
    },
    /// d-30: N parallel cancel RPCs spawned. The
    /// individual outcomes don't surface in the cancel
    /// fragment (operator sees them on the Subscribe
    /// stream as TransferComplete/Error events). After
    /// the configured TTL, the fragment auto-hides like
    /// the single-cancel Done variant.
    BatchInitiated {
        count: usize,
        finished_at: Instant,
    },
    Done {
        // The transfer_id is carried by `outcome` (every
        // CancelJobOutcome variant has its own
        // `transfer_id` field), so we don't double up
        // here.
        outcome: blit_app::admin::jobs::CancelJobOutcome,
        /// d-23: terminal-state timestamp. The footer
        /// converter hides the fragment after the
        /// configured cancel-status TTL (d-24:
        /// `tui.toml [transfer] cancel_status_ttl_ms`,
        /// default 5s) has elapsed so the operator gets
        /// a few seconds to read the outcome and then
        /// the footer self-cleans.
        finished_at: Instant,
    },
    Error {
        transfer_id: String,
        message: String,
        finished_at: Instant,
    },
}

impl F2CancelStatus {
    fn is_sending(&self) -> bool {
        matches!(self, F2CancelStatus::Sending { .. })
    }

    /// d-29 / d-30: `true` while waiting on the operator's
    /// `y`/`N` answer for either a single-cancel confirm
    /// or a batch-cancel confirm.
    fn is_confirming(&self) -> bool {
        matches!(
            self,
            F2CancelStatus::Confirming { .. } | F2CancelStatus::ConfirmingBatch { .. }
        )
    }
}

/// Reply envelope from the spawned CancelJob task.
struct CancelReply {
    request_id: u64,
    transfer_id: String,
    result: Result<blit_app::admin::jobs::CancelJobOutcome, String>,
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

    // e-3 R2: load `tui.toml` BEFORE entering the
    // alternate screen, otherwise parse warnings written
    // via eprintln corrupt the rendered UI (or get
    // swallowed by the alternate screen and never reach
    // the operator). The loader's `on_warn` callback
    // pushes any warning into a Vec; we flush it AFTER
    // the TuiGuard drops so the message lands cleanly on
    // the post-exit terminal.
    let mut config_warnings: Vec<String> = Vec::new();
    let tui_config = config::load(|msg| config_warnings.push(msg));

    // e-7: validate the [theme] accent color. An unknown
    // name (typo, terminal-specific color not in our
    // palette) buffers a warning + falls back to the
    // default. Same buffer-then-flush contract as parse
    // errors.
    if tui_config.theme.parse_accent().is_none() {
        config_warnings.push(format!(
            "tui.toml [theme] accent_color = {:?} is not a recognized color; \
             using default {:?}",
            tui_config.theme.accent_color,
            config::ThemeDefaults::DEFAULT_ACCENT,
        ));
    }

    let mut guard = TuiGuard::new().context("entering TUI")?;
    let result = run_router(guard.terminal_mut(), &args, tui_config).await;
    drop(guard);

    // Drain accumulated warnings now that the terminal
    // is back to its normal state.
    for warning in config_warnings {
        eprintln!("[blit-tui] {warning}");
    }
    result
}

/// a1-6b round 2: single unified event loop. The router
/// spawns every background task once at startup and runs a
/// single `tokio::select!` that drains EVERY pane's reply
/// channel each iteration. This way a hidden pane's
/// producer (mDNS discovery while the operator is on F2,
/// Subscribe events while they're on F1, etc.) can't back
/// up and stall the producer.
///
/// F2 setup is also fully backgrounded. `open_subscribe_stream`
/// plus the initial `GetState` run in a spawned task whose
/// completion arrives through `f2_setup_rx`. The TUI's first
/// draw therefore runs immediately, regardless of how slow
/// or unreachable the remote is.
async fn run_router(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    args: &Args,
    // d-36: `mut` so a `Ctrl+R` reload can swap in a
    // freshly-parsed config without restarting the TUI.
    mut tui_config: config::TuiConfig,
) -> Result<()> {
    // a1-6 round 2: the input task is owned by the router
    // for the whole TUI lifetime.
    let (key_tx, mut key_rx) = mpsc::channel::<KeyEvent>(16);
    spawn_input_task(key_tx);

    // a1-6b: parse remote up-front so every pane sees the
    // same endpoint (or None) without re-parsing. Round 2:
    // keep the parse error string so F2/F3 banners can
    // surface the specific message (backslash guidance,
    // missing module-path syntax, etc.) instead of a
    // generic "invalid endpoint."
    let (parsed_remote, parse_error_message): (Option<RemoteEndpoint>, Option<String>) =
        match args.remote.as_deref() {
            Some(raw) => match RemoteEndpoint::parse(raw) {
                Ok(ep) => (Some(ep), None),
                Err(err) => (None, Some(format!("parse '{raw}': {err}"))),
            },
            None => (None, None),
        };
    let remote_label = args.remote.as_deref().unwrap_or("(no remote)").to_string();

    // a1-6b: background tasks spawned ONCE here. Receivers
    // live in this function's scope; pane loops borrow them
    // by `&mut` reference.

    // F1 mDNS discovery — spawned once, runs for the whole
    // TUI session. Pane visits to F1 just drain `disco_rx`.
    let (discovery_refresh_tx, refresh_rx) = mpsc::channel::<()>(1);
    let (disco_tx, mut disco_rx) = mpsc::channel::<DiscoveryUpdate>(4);
    spawn_discovery_task(
        F1_DISCOVERY_INTERVAL,
        F1_DISCOVERY_SCAN_TIMEOUT,
        refresh_rx,
        disco_tx,
    );

    // F1 detail fetcher reply channel.
    let (detail_tx, mut detail_rx) = mpsc::channel::<DetailUpdate>(F1_DETAIL_BUFFER);

    // F3 browse fetcher reply channel.
    let (browse_fetch_tx, mut browse_fetch_rx) = mpsc::channel::<BrowseFetchReply>(8);

    // F4 profile fetcher reply channel.
    let (profile_reply_tx, mut profile_reply_rx) = mpsc::channel::<ProfileReply>(4);

    // F4 Verify run reply channel.
    let (verify_run_tx, mut verify_run_rx) = mpsc::channel::<VerifyReply>(2);

    // F4 Diagnostics dump reply channel.
    let (diagnostics_reply_tx, mut diagnostics_reply_rx) = mpsc::channel::<DiagnosticsReply>(2);

    // F4 local-transfer reply channel.
    let (transfer_reply_tx, mut transfer_reply_rx) = mpsc::channel::<TransferReply>(2);

    // d-22: F2 cancel-selected reply channel. Same shape
    // as the F4 transfer reply machinery.
    let (cancel_reply_tx, mut cancel_reply_rx) = mpsc::channel::<CancelReply>(2);

    // d-35: F3 pull reply channel.
    let (f3_pull_reply_tx, mut f3_pull_reply_rx) = mpsc::channel::<F3PullReply>(2);

    // d-37: F3 pull live-progress channel. Small bounded
    // buffer — the forwarder `try_send`s, dropping
    // intermediate snapshots when full.
    let (f3_pull_progress_tx, mut f3_pull_progress_rx) = mpsc::channel::<F3PullProgress>(8);

    let mut app = AppState {
        current_screen: args.screen.into(),
        parsed_remote: parsed_remote.clone(),
        remote_label,
        daemons: DaemonsState::new(),
        daemons_last_fetched: None,
        detail_tx: detail_tx.clone(),
        discovery_refresh_tx,
        transfers: TransfersState::new(),
        transfers_status: if parsed_remote.is_some() {
            ConnectionStatus::Connecting
        } else if parse_error_message.is_some() {
            ConnectionStatus::Degraded(parse_error_message.clone().unwrap_or_default())
        } else {
            ConnectionStatus::NoRemote
        },
        transfers_setup_gen: 0,
        transfers_setup_pending: false,
        browse: BrowseState::new(),
        browse_last_fetched_view: None,
        browse_fetch_tx: browse_fetch_tx.clone(),
        profile: profile::ProfileState::new(),
        profile_reply_tx: profile_reply_tx.clone(),
        verify: verify::VerifyState::with_defaults_and_paths(
            tui_config.verify.default_use_checksum,
            tui_config.verify.default_one_way,
            tui_config.verify.default_source.clone(),
            tui_config.verify.default_destination.clone(),
        ),
        diagnostics: diagnostics::DiagnosticsState::new(),
        diagnostics_reply_tx: diagnostics_reply_tx.clone(),
        help: help::HelpOverlay::default(),
        transfer: transfer::TransferState::new(),
        transfer_reply_tx: transfer_reply_tx.clone(),
        cancel_status: F2CancelStatus::Idle,
        cancel_reply_tx: cancel_reply_tx.clone(),
        cancel_request_seq: 0,
        f3_pull: f3pull::F3PullState::new(),
        f3_pull_reply_tx: f3_pull_reply_tx.clone(),
        f3_pull_progress_tx: f3_pull_progress_tx.clone(),
        reload_banner: None,
    };

    // F3 banner for missing/malformed remote. Surfaces the
    // specific parse error when present (round 2 of a1-6b).
    match (args.remote.as_deref(), parse_error_message.as_deref()) {
        (None, _) => app
            .browse
            .note_fetch_error("--remote <host> is required for F3 Browse".to_string()),
        (Some(_), Some(msg)) => {
            app.browse.note_fetch_error(msg.to_string());
        }
        (Some(_), None) => {}
    }

    // F2 Subscribe + initial GetState — backgrounded.
    // Round 2 of a1-6b: a slow / unreachable remote no
    // longer blocks the TUI's first draw. The spawned task
    // posts its outcome through `f2_setup_rx` and the
    // unified loop's select! arm wires the resulting
    // event_rx + snapshot into `app`.
    let (f2_setup_tx, mut f2_setup_rx) = mpsc::channel::<F2SetupReply>(1);
    if let Some(endpoint) = app.parsed_remote.clone() {
        app.transfers_setup_gen += 1;
        app.transfers_setup_pending = true;
        spawn_f2_setup_task(endpoint, app.transfers_setup_gen, f2_setup_tx.clone());
    }

    // F4 initial profile fetch — kicked once so the operator
    // sees data the first time they hit F4.
    let initial_profile_id = app.profile.begin_fetch();
    spawn_profile_fetch(initial_profile_id, profile_reply_tx.clone());

    // Optional Subscribe receiver. Populated once F2 setup
    // completes (either at startup or after `r` re-opens
    // the stream in the future).
    let mut transfers_event_rx: Option<mpsc::Receiver<EventOrError>> = None;

    // ───────────────────────────────────────────────────
    // Unified event loop. Drains every pane's channels on
    // every iteration regardless of which pane is active,
    // so a hidden producer can never back up and stall.
    // ───────────────────────────────────────────────────
    loop {
        // Per-pane "before draw" work: F1 kicks the GetState
        // detail fetch if the cursor moved.
        if matches!(app.current_screen, Screen::F1) {
            maybe_kick_detail_fetch(
                &mut app.daemons,
                &mut app.daemons_last_fetched,
                &app.detail_tx,
            );
        }
        // F3 kicks a browse fetch when its view changed.
        if matches!(app.current_screen, Screen::F3)
            && app.parsed_remote.is_some()
            && views_differ(app.browse_last_fetched_view.as_ref(), app.browse.view())
            && matches!(
                app.browse.status(),
                browse::BrowseFetchStatus::Idle | browse::BrowseFetchStatus::Error { .. }
            )
        {
            if let Some(ep) = app.parsed_remote.as_ref() {
                kick_browse_fetch(&mut app.browse, ep, &app.browse_fetch_tx);
                app.browse_last_fetched_view = Some(app.browse.view().clone());
            }
        }

        let now = Instant::now();
        // d-36: drop an expired reload banner so
        // `needs_live_tick` stops ticking for it.
        if app
            .reload_banner
            .as_ref()
            .is_some_and(|b| !b.is_visible(now))
        {
            app.reload_banner = None;
        }
        // d-38: auto-hide a finished F3 pull fragment once
        // its TTL elapses (same state-level expiry as the
        // reload banner — `needs_live_tick` ticks while a
        // terminal fragment shows, then stops once cleared).
        // d-40: TTL is operator-tunable via
        // `[transfer] pull_status_ttl_ms`, read each frame so
        // a Ctrl+R reload retunes it live.
        let pull_ttl = Duration::from_millis(tui_config.transfer.pull_status_ttl_ms_clamped());
        app.f3_pull.clear_terminal_if_expired(now, pull_ttl);
        // d-36: accent + reload banner are recomputed each
        // frame from the (possibly hot-reloaded) config, so
        // a `Ctrl+R` theme change takes effect immediately.
        let accent_color = tui_config
            .theme
            .parse_accent()
            .map(raw_color_to_ratatui)
            .unwrap_or(ratatui::style::Color::Cyan);
        let reload_banner = app
            .reload_banner
            .as_ref()
            .filter(|b| b.is_visible(now))
            .map(|b| {
                (
                    b.message.clone(),
                    if b.ok {
                        ratatui::style::Color::Green
                    } else {
                        ratatui::style::Color::Red
                    },
                )
            });
        terminal
            .draw(|frame| {
                let (tab_area, body_area) = screens::split_for_tabs(frame.area());
                // e-2 R2: daemons = discovered remotes
                // (excludes the synthetic Local row), and
                // active/recent fold the F4 local transfer
                // state into the daemon-stream counts.
                let counts = screens::TabStripCounts {
                    daemons: app.daemons.discovered_count(),
                    active_transfers: app.transfers.active_count() + app.transfer.count_active(),
                    recent_transfers: app.transfers.recent_count() + app.transfer.count_recent(),
                };
                screens::render_tab_strip(
                    frame,
                    tab_area,
                    app.current_screen,
                    counts,
                    tui_config.tab_strip.show_counts,
                    accent_color,
                    reload_banner.as_ref().map(|(m, c)| (m.as_str(), *c)),
                );
                // d-34: derive the F3 pull-source spec
                // through the real `RemoteEndpoint` so the
                // preview round-trips (bracketed IPv6,
                // port-aware) via the endpoint's own
                // `display()` rather than a hand-built
                // string. Bound here so the owned String
                // outlives the render call.
                let f3_pull_spec: Option<String> = app.parsed_remote.as_ref().and_then(|base| {
                    browse::pull_source_endpoint(app.browse.view(), app.browse.selected_row(), base)
                        .map(|e| e.display())
                });
                match app.current_screen {
                    Screen::F1 => screens::f1::render_into(frame, body_area, &app.daemons, now),
                    Screen::F2 => screens::f2::render_into(
                        frame,
                        body_area,
                        &app.transfers,
                        &app.remote_label,
                        &app.transfers_status,
                        &cancel_status_to_display(
                            &app.cancel_status,
                            now,
                            std::time::Duration::from_millis(
                                tui_config.transfer.cancel_status_ttl_ms_clamped(),
                            ),
                        ),
                        now,
                    ),
                    Screen::F3 => screens::f3::render_into(
                        frame,
                        body_area,
                        &app.browse,
                        &app.remote_label,
                        f3_pull_spec.as_deref(),
                        &f3_pull_to_display(app.f3_pull.status()),
                        now,
                    ),
                    Screen::F4 => screens::f4::render_into(
                        frame,
                        body_area,
                        &app.profile,
                        &app.verify,
                        &app.diagnostics,
                        &app.transfer,
                        now,
                    ),
                }
                if app.help.is_visible() {
                    // Overlay paints on top of the pane.
                    // Uses `Clear` internally so widgets
                    // beneath aren't visible through it.
                    help::render_overlay(frame, body_area, app.help);
                }
            })
            .context("terminal.draw")?;

        // d-9: a conditional ticker keeps the F4 elapsed
        // counters live while a Verify run or local
        // transfer is in flight. When idle, the tick
        // future is `pending()` so the loop sleeps
        // indefinitely waiting on real events — no idle
        // CPU burn, no terminal flicker.
        // e-5: cadence is now operator-tunable via
        // `[live_tick] interval_ms` in tui.toml (default
        // 500ms; clamped to [50, 5000]).
        // d-24 R2: when F2 is visible and a Done/Error
        // cancel fragment is pending auto-hide, the sleep
        // budget collapses to min(live_tick_interval,
        // remaining_cancel_ttl). Otherwise a long
        // `live_tick.interval_ms` would silently delay a
        // short `cancel_status_ttl_ms`.
        let needs_live_tick = needs_live_tick(&app);
        let live_tick_interval =
            std::time::Duration::from_millis(tui_config.live_tick.interval_ms_clamped());
        let cancel_ttl =
            std::time::Duration::from_millis(tui_config.transfer.cancel_status_ttl_ms_clamped());
        let cancel_remaining = if matches!(app.current_screen, Screen::F2) {
            cancel_status_remaining_ttl(&app.cancel_status, Instant::now(), cancel_ttl)
        } else {
            None
        };
        let tick_budget =
            compute_tick_budget(needs_live_tick, live_tick_interval, cancel_remaining);
        let live_tick = async {
            if let Some(dur) = tick_budget {
                tokio::time::sleep(dur).await;
            } else {
                std::future::pending::<()>().await;
            }
        };
        tokio::pin!(live_tick);

        // Build the optional Subscribe future. `select!`'s
        // `if` guard prevents polling when we have no
        // receiver yet (F2 setup still in flight, or no
        // remote configured).
        tokio::select! {
            // d-9 / e-5: live-tick wakeup for F4 elapsed
            // counters + freshness footers. Cadence is
            // `tui_config.live_tick.interval_ms_clamped()`
            // (default 500ms; bounded to [50, 5000]).
            // Body is empty — the next loop iteration's
            // terminal.draw call computes a fresh `now`
            // and re-renders with the updated duration
            // string.
            _ = &mut live_tick => {}

            // Keystrokes — dispatched to the active pane.
            key = key_rx.recv() => {
                let Some(key) = key else { return Ok(()); };
                // d-2 round 1: when F4's Verify form has
                // focus, char keys must go through as
                // text input, not as profile-lifecycle
                // actions (`c`/`d`/`e`). Esc clears focus
                // instead of quitting; F-keys still
                // navigate (intercepted in
                // handle_verify_keystroke when not
                // editable).
                // e-1: when the `?` help overlay is open,
                // it absorbs every keystroke except `?`
                // (toggle), Esc (close), and Ctrl-c
                // (emergency quit). F-keys are absorbed
                // too — the operator can't accidentally
                // pane-switch while reading the help.
                if app.help.is_visible() {
                    if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
                        app.help.close();
                        continue;
                    }
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        return Ok(());
                    }
                    // d-31: j/k (and arrow / PageUp-Down)
                    // scroll the keymap while the overlay
                    // is open. Everything else is absorbed
                    // so the operator can't pane-switch or
                    // trigger actions mid-read.
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.help.scroll_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.help.scroll_up(),
                        _ => {}
                    }
                    continue;
                }
                // d-12 R2: Esc cancels a pending mirror/move
                // confirm prompt. Has to happen BEFORE the
                // Verify keystroke handler so the operator
                // can still escape a confirm even after
                // Tab-ing into the Verify form (which
                // `handle_verify_keystroke` would otherwise
                // absorb Esc for clear-focus). Also has to
                // happen before key_action runs because
                // `should_quit` maps bare Esc to Quit, which
                // would tear down the TUI on what the
                // operator intended as a "no, don't do that"
                // gesture.
                if esc_cancels_confirm(&key, &app) {
                    // Reset whichever state machine had
                    // the pending confirm. Both calls are
                    // no-ops when their own state is Idle,
                    // so it's safe to fire both (in
                    // practice only one is in Confirming
                    // at a time, but no harm in being
                    // defensive).
                    app.transfer.cancel_confirm();
                    if app.cancel_status.is_confirming() {
                        app.cancel_status = F2CancelStatus::Idle;
                    }
                    continue;
                }
                if app.current_screen == Screen::F4
                    && app.verify.focus().is_editing()
                    && handle_verify_keystroke(&key, &mut app, &verify_run_tx)
                {
                    continue;
                }
                // d-26: F3's `/` filter edit mode mirrors
                // the F4 Verify pattern — while editing,
                // chars / Backspace / Esc / Enter route
                // to the filter API instead of the normal
                // F3 dispatcher.
                if app.current_screen == Screen::F3
                    && app.browse.is_editing_filter()
                    && handle_f3_filter_keystroke(&key, &mut app)
                {
                    continue;
                }
                // d-35: F3's `p` pull destination prompt
                // uses the same input-mode pattern — while
                // entering the dest, keystrokes route to the
                // pull state instead of the F3 dispatcher.
                if app.current_screen == Screen::F3
                    && app.f3_pull.is_entering_dest()
                    && handle_f3_pull_keystroke(&key, &mut app)
                {
                    continue;
                }
                // If handle_verify_keystroke returned false
                // (F-keys, Ctrl-c) or we're not in editing
                // mode, fall through to the action
                // dispatcher.
                if let Some(action) = key_action(&key) {
                    match action {
                        UserAction::Quit => return Ok(()),
                        UserAction::ToggleHelp => {
                            app.help.toggle();
                        }
                        // d-36: Ctrl+R hot-reloads tui.toml.
                        // Global (handled here, not in a
                        // per-pane dispatcher) since it owns
                        // the run_router-scoped `tui_config`.
                        UserAction::ReloadConfig => {
                            let (next, banner) = reload_tui_config(&tui_config, Instant::now());
                            tui_config = next;
                            app.reload_banner = Some(banner);
                        }
                        UserAction::Navigate(target) => {
                            app.current_screen = target;
                            // Leaving F4 drops the editing
                            // focus so the next visit
                            // starts in action-key mode.
                            if target != Screen::F4 {
                                app.verify.clear_focus();
                            }
                        }
                        action => {
                            // F4-specific: Tab toggles the
                            // Verify focus. Otherwise route
                            // to the normal pane action.
                            if app.current_screen == Screen::F4
                                && matches!(key.code, KeyCode::Tab)
                            {
                                app.verify.cycle_focus();
                            } else {
                                handle_pane_action(
                                    action,
                                    &mut app,
                                    &mut transfers_event_rx,
                                    &f2_setup_tx,
                                    &tui_config,
                                )
                                .await;
                            }
                        }
                    }
                } else if app.current_screen == Screen::F4
                    && matches!(key.code, KeyCode::Tab)
                {
                    // Tab from action-mode enters
                    // editing mode.
                    app.verify.cycle_focus();
                }
            }
            // F1 mDNS discovery feed — drained continuously.
            update = disco_rx.recv() => {
                match update {
                    Some(DiscoveryUpdate::Result(services)) => {
                        app.daemons.replace_from_discovery(&services, Instant::now());
                    }
                    Some(DiscoveryUpdate::Error(msg)) => {
                        app.daemons.note_discovery_error(msg);
                    }
                    None => {
                        app.daemons.note_discovery_error(
                            "discovery task exited".to_string(),
                        );
                    }
                }
            }
            // F1 GetState-detail replies.
            update = detail_rx.recv() => {
                if let Some(DetailUpdate { instance_name, request_id, result }) = update {
                    let detail = match result {
                        Ok(daemon_state) => DaemonDetail::Loaded {
                            state: Box::new(daemon_state),
                            fetched_at: Instant::now(),
                        },
                        Err(message) => DaemonDetail::Error { message },
                    };
                    app.daemons.apply_detail_update(&instance_name, request_id, detail);
                }
            }
            // F2 Subscribe events — only polled when we
            // have a receiver. The guard means the arm is
            // disabled until f2_setup completes.
            event = async {
                match transfers_event_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            }, if transfers_event_rx.is_some() => {
                match event {
                    Some(EventOrError::Connected) => {
                        if matches!(app.transfers_status, ConnectionStatus::Connecting) {
                            app.transfers_status = ConnectionStatus::Live;
                        }
                    }
                    Some(EventOrError::Event(daemon_event)) => {
                        app.transfers.apply_event(daemon_event, Instant::now());
                        if matches!(app.transfers_status, ConnectionStatus::Connecting) {
                            app.transfers_status = ConnectionStatus::Live;
                        }
                    }
                    Some(EventOrError::Error(msg)) => {
                        app.transfers_status = ConnectionStatus::Degraded(msg);
                        transfers_event_rx = None;
                    }
                    None => {
                        app.transfers_status = ConnectionStatus::Degraded(
                            "subscribe stream closed".to_string(),
                        );
                        transfers_event_rx = None;
                    }
                }
            }
            // F2 setup task completion.
            setup = f2_setup_rx.recv() => {
                if let Some(reply) = setup {
                    // Round-3 generation gate: drop the
                    // reply if a newer setup was spawned
                    // before this one returned. The pending
                    // flag is cleared either way so a
                    // future `r` can spawn afresh.
                    if reply.gen != app.transfers_setup_gen {
                        // Stale generation — drop silently.
                    } else {
                        app.transfers_setup_pending = false;
                        match reply.payload {
                            F2SetupPayload::Ready { event_rx, snapshot_result } => {
                                let mut rx = event_rx;
                                match snapshot_result {
                                    Ok(snapshot) => app.transfers.replace_from_snapshot(snapshot, Instant::now()),
                                    Err(err) => {
                                        app.transfers_status = ConnectionStatus::Degraded(
                                            format!("initial GetState failed: {err}"),
                                        );
                                    }
                                }
                                drain_startup_events(
                                    &mut rx,
                                    &mut app.transfers,
                                    &mut app.transfers_status,
                                );
                                transfers_event_rx = Some(rx);
                            }
                            F2SetupPayload::Failed(err) => {
                                app.transfers_status = ConnectionStatus::Degraded(err);
                            }
                        }
                    }
                }
            }
            // F3 browse-fetch replies.
            reply = browse_fetch_rx.recv() => {
                if let Some(reply) = reply {
                    apply_browse_reply(&mut app.browse, reply);
                }
            }
            // F4 profile-fetch replies.
            reply = profile_reply_rx.recv() => {
                if let Some(ProfileReply { request_id, result }) = reply {
                    if app.profile.is_current_request(request_id) {
                        match result {
                            Ok(report) => app.profile.apply_report(report, Instant::now()),
                            Err(message) => app.profile.note_fetch_error(message),
                        }
                    }
                }
            }
            // F4 Verify run replies. Generation gating is
            // inside `apply_result`/`apply_error` —
            // stale replies are silently dropped.
            reply = verify_run_rx.recv() => {
                if let Some(VerifyReply { request_id, result }) = reply {
                    match result {
                        Ok(r) => { app.verify.apply_result(request_id, r); }
                        Err(msg) => { app.verify.apply_error(request_id, msg); }
                    }
                }
            }
            // F4 Diagnostics dump replies.
            reply = diagnostics_reply_rx.recv() => {
                if let Some(DiagnosticsReply { request_id, result }) = reply {
                    match result {
                        Ok(path) => { app.diagnostics.apply_done(request_id, path); }
                        Err(msg) => { app.diagnostics.apply_error(request_id, msg); }
                    }
                }
            }
            // F4 local-transfer replies.
            reply = transfer_reply_rx.recv() => {
                if let Some(TransferReply { request_id, kind, result }) = reply {
                    match result {
                        Ok(summary) => { app.transfer.apply_done(request_id, kind, summary); }
                        Err(msg) => { app.transfer.apply_error(request_id, kind, msg); }
                    }
                }
            }
            // d-22: F2 cancel-selected replies.
            reply = cancel_reply_rx.recv() => {
                if let Some(CancelReply { request_id, transfer_id, result }) = reply {
                    // Generation guard: a second cancel
                    // fired while the first was in flight
                    // would have bumped `cancel_request_seq`.
                    // The stale reply still arrives — drop
                    // it so the operator sees the latest
                    // attempt's outcome, not the previous.
                    let current_request_id = match &app.cancel_status {
                        F2CancelStatus::Sending { request_id: rid, .. } => Some(*rid),
                        _ => None,
                    };
                    if current_request_id != Some(request_id) {
                        // Stale — drop.
                    } else {
                        let finished_at = Instant::now();
                        app.cancel_status = match result {
                            Ok(outcome) => {
                                let _ = transfer_id; // carried by outcome
                                F2CancelStatus::Done {
                                    outcome,
                                    finished_at,
                                }
                            }
                            Err(message) => F2CancelStatus::Error {
                                transfer_id,
                                message,
                                finished_at,
                            },
                        };
                    }
                }
            }
            // d-35: F3 pull replies. The generation guard
            // lives in `F3PullState::apply_*` (compares
            // `request_id` to the current `Running` run).
            reply = f3_pull_reply_rx.recv() => {
                if let Some(F3PullReply { request_id, result }) = reply {
                    let at = Instant::now();
                    match result {
                        Ok((files, bytes)) => {
                            app.f3_pull.apply_done(request_id, files, bytes, at);
                        }
                        Err(message) => {
                            app.f3_pull.apply_error(request_id, message, at);
                        }
                    }
                }
            }
            // d-37: F3 pull live-progress snapshots.
            snapshot = f3_pull_progress_rx.recv() => {
                if let Some(F3PullProgress {
                    request_id,
                    files,
                    bytes,
                    bytes_per_sec,
                }) = snapshot
                {
                    app.f3_pull
                        .apply_progress(request_id, files, bytes, bytes_per_sec);
                }
            }
        }
    }
}

/// Dispatch a non-Quit, non-Navigate action to the pane
/// that's currently active. The unified loop intercepts
/// Quit/Navigate before calling this; everything else is
/// pane-specific.
async fn handle_pane_action(
    action: UserAction,
    app: &mut AppState,
    transfers_event_rx: &mut Option<mpsc::Receiver<EventOrError>>,
    f2_setup_tx: &mpsc::Sender<F2SetupReply>,
    tui_config: &config::TuiConfig,
) {
    match app.current_screen {
        Screen::F1 => match action {
            UserAction::Refresh => {
                let _ = app.discovery_refresh_tx.try_send(());
                if let Some(name) = app.daemons.selected_row().map(|r| r.instance_name.clone()) {
                    app.daemons.invalidate_detail(&name);
                }
                app.daemons_last_fetched = None;
            }
            UserAction::SelectNext => app.daemons.select_next(),
            UserAction::SelectPrev => app.daemons.select_prev(),
            _ => {}
        },
        Screen::F2 => match action {
            UserAction::Refresh => {
                if should_spawn_f2_setup(transfers_event_rx.is_some(), app.transfers_setup_pending)
                {
                    // No live stream and no setup in flight
                    // — try to (re)open. Round-3 guard
                    // closes the duplicate-setup race.
                    if let Some(endpoint) = app.parsed_remote.clone() {
                        app.transfers_status = ConnectionStatus::Connecting;
                        app.transfers_setup_gen += 1;
                        app.transfers_setup_pending = true;
                        spawn_f2_setup_task(endpoint, app.transfers_setup_gen, f2_setup_tx.clone());
                    }
                } else if transfers_event_rx.is_some() {
                    if let Some(endpoint) = app.parsed_remote.as_ref() {
                        refresh_via_get_state(
                            endpoint,
                            &mut app.transfers,
                            &mut app.transfers_status,
                        )
                        .await;
                    }
                }
                // else: setup is pending; refresh is a no-op
                // until the in-flight task lands.
            }
            // d-21: cursor selection in the active table.
            // First press selects the newest row (index 0);
            // subsequent presses walk through.
            UserAction::SelectNext => app.transfers.select_next_active(),
            UserAction::SelectPrev => app.transfers.select_prev_active(),
            // d-22: cancel the cursor-selected transfer.
            // Gated on a confirmed live selection AND a
            // remote being configured AND no cancel
            // already in flight (Sending) AND no confirm
            // prompt already up. Without all four the
            // keystroke is silently ignored.
            //
            // d-29: when `[transfer] confirm_cancel = true`,
            // K transitions to Confirming instead of
            // firing the RPC immediately. `y` then
            // promotes Confirming → Sending; `n` / `Esc`
            // revert to Idle.
            UserAction::CancelSelectedTransfer => {
                if app.cancel_status.is_sending() || app.cancel_status.is_confirming() {
                    // Already mid-cycle — ignore.
                } else if let (Some(id), Some(endpoint)) = (
                    app.transfers.selected_active_id().map(|s| s.to_string()),
                    app.parsed_remote.clone(),
                ) {
                    if tui_config.transfer.confirm_cancel {
                        app.cancel_status = F2CancelStatus::Confirming { transfer_id: id };
                    } else {
                        app.cancel_request_seq += 1;
                        let rid = app.cancel_request_seq;
                        app.cancel_status = F2CancelStatus::Sending {
                            transfer_id: id.clone(),
                            request_id: rid,
                        };
                        spawn_cancel_transfer(rid, endpoint, id, app.cancel_reply_tx.clone());
                    }
                }
            }
            // d-29: `y` confirms a pending cancel — promote
            // Confirming → Sending and fire the RPC.
            // d-30: `y` also promotes ConfirmingBatch →
            // BatchInitiated and spawns N RPCs.
            UserAction::TransferMirrorConfirm if app.cancel_status.is_confirming() => {
                let Some(endpoint) = app.parsed_remote.clone() else {
                    // No remote — abort the prompt rather
                    // than silently sit on it.
                    app.cancel_status = F2CancelStatus::Idle;
                    return;
                };
                // d-30 R2: clone-out the variant payload
                // before mutating `app.cancel_status` so
                // the borrow doesn't outlive the match.
                let confirmed = match &app.cancel_status {
                    F2CancelStatus::Confirming { transfer_id } => {
                        ConfirmedCancel::Single(transfer_id.clone())
                    }
                    F2CancelStatus::ConfirmingBatch { transfer_ids } => {
                        ConfirmedCancel::Batch(transfer_ids.clone())
                    }
                    _ => return,
                };
                match confirmed {
                    ConfirmedCancel::Single(id) => {
                        app.cancel_request_seq += 1;
                        let rid = app.cancel_request_seq;
                        app.cancel_status = F2CancelStatus::Sending {
                            transfer_id: id.clone(),
                            request_id: rid,
                        };
                        spawn_cancel_transfer(rid, endpoint, id, app.cancel_reply_tx.clone());
                    }
                    ConfirmedCancel::Batch(ids) => {
                        let count = spawn_cancels_for_ids(
                            ids,
                            &endpoint,
                            &mut app.cancel_request_seq,
                            &app.cancel_reply_tx,
                        );
                        app.cancel_status = F2CancelStatus::BatchInitiated {
                            count,
                            finished_at: Instant::now(),
                        };
                    }
                }
            }
            // d-29 / d-30: `n` aborts whichever confirm
            // prompt is open.
            UserAction::TransferCancel if app.cancel_status.is_confirming() => {
                app.cancel_status = F2CancelStatus::Idle;
            }
            // d-30: `Shift+X` batch-cancels every active
            // transfer. Same gates as the single-cancel
            // K path — no remote, no rows, or mid-cycle
            // → silent no-op.
            //
            // d-30 R2: snapshot the active ids ONCE here.
            // The confirm path stores the ids on the
            // ConfirmingBatch variant so the `y` arm
            // doesn't re-read `transfers.active` (which
            // the Subscribe stream keeps mutating in the
            // background).
            UserAction::CancelAllActiveTransfers => {
                if app.cancel_status.is_sending() || app.cancel_status.is_confirming() {
                    // Already mid-cycle — ignore.
                } else if let Some(endpoint) = app.parsed_remote.clone() {
                    let ids = snapshot_active_ids(&app.transfers);
                    if ids.is_empty() {
                        // No active transfers — silent no-op.
                    } else if tui_config.transfer.confirm_cancel {
                        app.cancel_status = F2CancelStatus::ConfirmingBatch { transfer_ids: ids };
                    } else {
                        let count = spawn_cancels_for_ids(
                            ids,
                            &endpoint,
                            &mut app.cancel_request_seq,
                            &app.cancel_reply_tx,
                        );
                        app.cancel_status = F2CancelStatus::BatchInitiated {
                            count,
                            finished_at: Instant::now(),
                        };
                    }
                }
            }
            _ => {}
        },
        Screen::F3 => match action {
            UserAction::Refresh => {
                handle_f3_refresh(
                    &mut app.browse,
                    app.parsed_remote.is_some(),
                    &mut app.browse_last_fetched_view,
                );
            }
            UserAction::SelectNext => app.browse.select_next(),
            UserAction::SelectPrev => app.browse.select_prev(),
            UserAction::Descend => {
                app.browse.descend();
            }
            UserAction::Ascend => {
                app.browse.ascend();
            }
            UserAction::F3FilterBegin => {
                app.browse.begin_edit_filter();
            }
            // d-35: `p` opens the pull destination prompt.
            // Gated on: a remote configured, a derivable
            // pull source under the cursor, and no pull
            // already entering-dest or running.
            UserAction::F3PullBegin => {
                let busy = app.f3_pull.is_entering_dest() || app.f3_pull.is_running();
                let source = app.parsed_remote.as_ref().and_then(|base| {
                    browse::pull_source_endpoint(app.browse.view(), app.browse.selected_row(), base)
                });
                if !busy {
                    if let Some(source) = source {
                        app.f3_pull.begin(source);
                    }
                }
            }
            _ => {}
        },
        Screen::F4 => match action {
            UserAction::Refresh => {
                let id = app.profile.begin_fetch();
                spawn_profile_fetch(id, app.profile_reply_tx.clone());
            }
            UserAction::ProfileClear => {
                // Only re-fetch on success — otherwise
                // begin_fetch's Pending → Loaded sequence
                // would wipe the error banner.
                let outcome = apply_profile_clear();
                if apply_lifecycle_outcome(&mut app.profile, outcome) {
                    let id = app.profile.begin_fetch();
                    spawn_profile_fetch(id, app.profile_reply_tx.clone());
                }
            }
            UserAction::ProfileDisable => {
                let outcome = apply_profile_set_enabled(false);
                if apply_lifecycle_outcome(&mut app.profile, outcome) {
                    let id = app.profile.begin_fetch();
                    spawn_profile_fetch(id, app.profile_reply_tx.clone());
                }
            }
            UserAction::ProfileEnable => {
                let outcome = apply_profile_set_enabled(true);
                if apply_lifecycle_outcome(&mut app.profile, outcome) {
                    let id = app.profile.begin_fetch();
                    spawn_profile_fetch(id, app.profile_reply_tx.clone());
                }
            }
            UserAction::DiagnosticsDump
                if !app.verify.source.trim().is_empty()
                    && !app.verify.destination.trim().is_empty() =>
            {
                // No-op when either field is empty —
                // there's nothing meaningful to snapshot.
                let id = app.diagnostics.begin_dump();
                spawn_diagnostics_dump(
                    id,
                    app.verify.source.clone(),
                    app.verify.destination.clone(),
                    app.diagnostics_reply_tx.clone(),
                );
            }
            UserAction::TransferCopy if can_start_transfer(app) => {
                match prepare_local_transfer(&app.verify.source, &app.verify.destination) {
                    Ok((src, dst)) => {
                        let id = app.transfer.begin(transfer::TransferKind::Copy);
                        spawn_local_transfer(
                            id,
                            transfer::TransferKind::Copy,
                            src,
                            dst,
                            app.transfer_reply_tx.clone(),
                        );
                    }
                    Err(msg) => {
                        app.transfer
                            .note_validation_error(transfer::TransferKind::Copy, msg);
                    }
                }
            }
            UserAction::TransferMirror if can_start_transfer(app) => {
                // d-4 R2: prompt before destructive mirror.
                // Validate the paths up-front so a parse
                // error surfaces before the operator
                // confirms (avoids "confirmed, then immediately
                // got a parse error" UX).
                match prepare_local_transfer(&app.verify.source, &app.verify.destination) {
                    Ok(_) => {
                        app.transfer.begin_confirm_mirror();
                    }
                    Err(msg) => {
                        app.transfer
                            .note_validation_error(transfer::TransferKind::Mirror, msg);
                    }
                }
            }
            UserAction::TransferMove if can_start_transfer(app) => {
                // Same gate as mirror — paths must parse
                // first so the operator doesn't confirm a
                // delete-source flow against an invalid
                // source.
                match prepare_local_transfer(&app.verify.source, &app.verify.destination) {
                    Ok(_) => {
                        app.transfer.begin_confirm_move();
                    }
                    Err(msg) => {
                        app.transfer
                            .note_validation_error(transfer::TransferKind::Move, msg);
                    }
                }
            }
            UserAction::TransferMirrorConfirm if app.transfer.is_confirming_mirror() => {
                // Re-validate at fire time. The Verify
                // fields are also invalidated on edit via
                // handle_verify_keystroke, but be defensive
                // — a stale confirm-pending must never run
                // a different set of paths than were shown
                // in the prompt.
                match prepare_local_transfer(&app.verify.source, &app.verify.destination) {
                    Ok((src, dst)) => {
                        let id = app.transfer.begin(transfer::TransferKind::Mirror);
                        spawn_local_transfer(
                            id,
                            transfer::TransferKind::Mirror,
                            src,
                            dst,
                            app.transfer_reply_tx.clone(),
                        );
                    }
                    Err(msg) => {
                        app.transfer
                            .note_validation_error(transfer::TransferKind::Mirror, msg);
                    }
                }
            }
            UserAction::TransferMirrorConfirm if app.transfer.is_confirming_move() => {
                match prepare_local_transfer(&app.verify.source, &app.verify.destination) {
                    Ok((src, dst)) => {
                        let id = app.transfer.begin(transfer::TransferKind::Move);
                        spawn_local_move(id, src, dst, app.transfer_reply_tx.clone());
                    }
                    Err(msg) => {
                        app.transfer
                            .note_validation_error(transfer::TransferKind::Move, msg);
                    }
                }
            }
            UserAction::TransferCancel if app.transfer.is_confirming() => {
                app.transfer.cancel_confirm();
            }
            UserAction::ToggleVerifyChecksum => {
                app.verify.toggle_checksum();
            }
            UserAction::ToggleVerifyOneWay => {
                app.verify.toggle_one_way();
            }
            _ => {}
        },
    }
}

/// Wipe the local perf-history file. Returns `Ok(())` on
/// success or `Err(message)` on failure. Caller is
/// expected to surface the error via
/// [`apply_lifecycle_outcome`] before kicking any
/// follow-up read.
fn apply_profile_clear() -> Result<(), String> {
    blit_core::perf_history::clear_history()
        .map(|_| ())
        .map_err(|err| format!("clear failed: {err:#}"))
}

/// Toggle the perf-history-enabled flag. Same error shape
/// as [`apply_profile_clear`].
fn apply_profile_set_enabled(enabled: bool) -> Result<(), String> {
    blit_core::perf_history::set_perf_history_enabled(enabled).map_err(|err| {
        let verb = if enabled { "enable" } else { "disable" };
        format!("{verb} failed: {err:#}")
    })
}

/// Apply the outcome of an F4 lifecycle mutation
/// (`c`/`d`/`e`). Returns `true` if the caller should
/// kick a profile re-fetch (the action succeeded). On
/// failure, writes the message into the profile state's
/// Error banner and returns `false` — the caller MUST NOT
/// kick a re-fetch in that case because `begin_fetch` would
/// immediately flip the status to `Pending`, hiding the
/// failure from the operator (d-1 round-2 fix).
fn apply_lifecycle_outcome(
    profile_state: &mut profile::ProfileState,
    result: Result<(), String>,
) -> bool {
    match result {
        Ok(()) => true,
        Err(msg) => {
            profile_state.note_fetch_error(msg);
            false
        }
    }
}

/// Pure helper: decide whether the F2 refresh keystroke
/// should spawn a fresh setup task. Returns `true` only
/// when we lack a live Subscribe receiver AND no setup is
/// already in flight. This is the round-3 fix for the F2
/// overlap race: pressing `r` while the initial setup is
/// still running must NOT spawn a duplicate.
fn should_spawn_f2_setup(event_rx_present: bool, setup_pending: bool) -> bool {
    !event_rx_present && !setup_pending
}

/// Reply envelope from the F2 setup task. Carries a
/// generation so the loop can drop stale results when a
/// second setup was kicked before the first returned.
struct F2SetupReply {
    gen: u64,
    payload: F2SetupPayload,
}

enum F2SetupPayload {
    Ready {
        event_rx: mpsc::Receiver<EventOrError>,
        snapshot_result: Result<DaemonState, String>,
    },
    Failed(String),
}

/// Background task for F2 setup. Opens the Subscribe
/// stream and fires the initial `GetState`. Either result
/// becomes a single `F2SetupReply` message into `tx`,
/// tagged with the generation the caller bumped before
/// spawning. Running this off the router's await means a
/// slow / unreachable remote does NOT block the TUI's
/// first draw.
fn spawn_f2_setup_task(endpoint: RemoteEndpoint, gen: u64, tx: mpsc::Sender<F2SetupReply>) {
    tokio::spawn(async move {
        let event_rx = match open_subscribe_stream(&endpoint).await {
            Ok(rx) => rx,
            Err(err) => {
                let _ = tx
                    .send(F2SetupReply {
                        gen,
                        payload: F2SetupPayload::Failed(err),
                    })
                    .await;
                return;
            }
        };
        let snapshot_result = jobs::query(&endpoint, 0)
            .await
            .map_err(|err| format!("{err:#}"));
        let _ = tx
            .send(F2SetupReply {
                gen,
                payload: F2SetupPayload::Ready {
                    event_rx,
                    snapshot_result,
                },
            })
            .await;
    });
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

/// Pure helper for F3's `r` keystroke. When `has_endpoint`
/// is false the refresh is a no-op — there's no daemon to
/// query and the existing status banner is already showing
/// the actionable error (missing or malformed `--remote`).
/// Overwriting that with `"refreshing"` would hide the
/// operator's actual problem AND leave the UI stuck on
/// "refreshing" because the kick path can never fire
/// without an endpoint.
///
/// When `has_endpoint` is true, bumps the request id (to
/// discard any in-flight reply), resets `last_fetched_view`
/// (so the next loop iteration's kick fires), and sets the
/// status to `Error("refreshing")` which the kick path
/// treats as a refresh trigger.
fn handle_f3_refresh(
    state: &mut BrowseState,
    has_endpoint: bool,
    last_fetched_view: &mut Option<browse::BrowseView>,
) {
    if !has_endpoint {
        return;
    }
    *last_fetched_view = None;
    state.begin_fetch();
    state.note_fetch_error("refreshing".to_string());
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

/// One-shot profile reader. `profile::query` is sync; wrap
/// it in `spawn_blocking` so a slow filesystem doesn't
/// stall the event loop.
fn spawn_profile_fetch(request_id: u64, tx: mpsc::Sender<ProfileReply>) {
    tokio::spawn(async move {
        // `profile::query(0)` matches the CLI's "no limit"
        // contract: read all records.
        let result = tokio::task::spawn_blocking(|| app_profile::query(0)).await;
        let envelope = match result {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(err)) => Err(format!("{err:#}")),
            Err(join_err) => Err(format!("profile read task panicked: {join_err}")),
        };
        let _ = tx
            .send(ProfileReply {
                request_id,
                result: envelope,
            })
            .await;
    });
}

struct ProfileReply {
    request_id: u64,
    result: Result<blit_app::profile::ProfileReport, String>,
}

/// Reply envelope from the F4 Verify run task. Generation
/// is tagged so a stale reply (operator edited the form
/// between kicks) gets dropped on arrival.
struct VerifyReply {
    request_id: u64,
    result: Result<blit_app::check::CheckResult, String>,
}

/// Reply envelope from a local-transfer task.
struct TransferReply {
    request_id: u64,
    kind: transfer::TransferKind,
    result: Result<blit_core::orchestrator::LocalMirrorSummary, String>,
}

/// d-22: convert the internal `F2CancelStatus` to the
/// renderer-facing `F2CancelDisplay` (which lives in
/// `screens/f2.rs` to avoid the screens layer reaching
/// into main.rs's types).
fn cancel_status_to_display(
    status: &F2CancelStatus,
    now: Instant,
    ttl: std::time::Duration,
) -> screens::f2::F2CancelDisplay {
    use blit_app::admin::jobs::CancelJobOutcome;
    use screens::f2::F2CancelDisplay;
    match status {
        F2CancelStatus::Idle => F2CancelDisplay::Hidden,
        F2CancelStatus::Confirming { transfer_id } => F2CancelDisplay::ConfirmingCancel {
            transfer_id: transfer_id.clone(),
        },
        F2CancelStatus::ConfirmingBatch { transfer_ids } => F2CancelDisplay::ConfirmingBatch {
            count: transfer_ids.len(),
        },
        F2CancelStatus::BatchInitiated { count, finished_at } => {
            if now.saturating_duration_since(*finished_at) >= ttl {
                return F2CancelDisplay::Hidden;
            }
            F2CancelDisplay::BatchInitiated { count: *count }
        }
        F2CancelStatus::Sending { transfer_id, .. } => F2CancelDisplay::Sending {
            transfer_id: transfer_id.clone(),
        },
        F2CancelStatus::Done {
            outcome,
            finished_at,
        } => {
            // d-23: hide the terminal fragment after the
            // TTL. The state itself stays — we don't mutate
            // it from the renderer — but the operator sees
            // the footer self-clean.
            if now.saturating_duration_since(*finished_at) >= ttl {
                return F2CancelDisplay::Hidden;
            }
            match outcome {
                CancelJobOutcome::Cancelled { transfer_id: id } => F2CancelDisplay::Cancelled {
                    transfer_id: id.clone(),
                },
                CancelJobOutcome::NotFound { transfer_id: id } => F2CancelDisplay::NotFound {
                    transfer_id: id.clone(),
                },
                CancelJobOutcome::Unsupported {
                    transfer_id: id,
                    message,
                } => F2CancelDisplay::Unsupported {
                    transfer_id: id.clone(),
                    message: message.clone(),
                },
            }
        }
        F2CancelStatus::Error {
            transfer_id,
            message,
            finished_at,
        } => {
            if now.saturating_duration_since(*finished_at) >= ttl {
                return F2CancelDisplay::Hidden;
            }
            F2CancelDisplay::Failed {
                transfer_id: transfer_id.clone(),
                message: message.clone(),
            }
        }
    }
}

/// d-36: re-read `tui.toml` for a `Ctrl+R` hot-reload.
/// Returns the config to use plus the banner to show.
///
/// On a parse error, the CURRENT config is kept (the
/// loader returns defaults on failure, which would
/// silently wipe the operator's settings) and the banner
/// carries the error. On success — including a missing
/// file, which legitimately means "use defaults" — the
/// freshly-loaded config is adopted.
fn reload_tui_config(
    current: &config::TuiConfig,
    now: Instant,
) -> (config::TuiConfig, ReloadBanner) {
    let mut warning: Option<String> = None;
    let loaded = config::load(|msg| warning = Some(msg));
    classify_reload(loaded, warning, current, now)
}

/// Pure core of [`reload_tui_config`] — splits the I/O
/// (`config::load`) from the keep-vs-adopt decision so
/// the decision is unit-testable without touching the
/// process-global config dir (which would race under
/// parallel tests).
fn classify_reload(
    loaded: config::TuiConfig,
    warning: Option<String>,
    current: &config::TuiConfig,
    now: Instant,
) -> (config::TuiConfig, ReloadBanner) {
    match warning {
        Some(message) => (
            current.clone(),
            ReloadBanner {
                message: format!("reload failed: {message} — kept previous"),
                ok: false,
                shown_at: now,
            },
        ),
        None => (
            loaded,
            ReloadBanner {
                message: "config reloaded".to_string(),
                ok: true,
                shown_at: now,
            },
        ),
    }
}

/// d-35: bridge the F3 pull state machine to the
/// renderer-facing `F3PullDisplay` (lives in
/// `screens/f3.rs` so the screens layer doesn't reach
/// into the `f3pull` module's internals).
fn f3_pull_to_display(status: &f3pull::F3PullStatus) -> screens::f3::F3PullDisplay {
    use f3pull::F3PullStatus;
    use screens::f3::F3PullDisplay;
    match status {
        F3PullStatus::Idle => F3PullDisplay::Hidden,
        F3PullStatus::EnteringDest { dest, .. } => {
            F3PullDisplay::EnteringDest { dest: dest.clone() }
        }
        F3PullStatus::Running {
            dest,
            files,
            bytes,
            bytes_per_sec,
            ..
        } => F3PullDisplay::Running {
            dest: dest.clone(),
            files: *files,
            bytes: *bytes,
            bytes_per_sec: *bytes_per_sec,
        },
        F3PullStatus::Done {
            files, bytes, dest, ..
        } => F3PullDisplay::Done {
            files: *files,
            bytes: *bytes,
            dest: dest.clone(),
        },
        F3PullStatus::Error { message, .. } => F3PullDisplay::Error {
            message: message.clone(),
        },
    }
}

/// d-35: reply envelope from a spawned F3 pull task.
struct F3PullReply {
    request_id: u64,
    /// Ok((files_transferred, bytes_transferred)) or a
    /// flattened error string.
    result: Result<(usize, u64), String>,
}

/// d-35: spawn a remote→local PullSync for an F3
/// transfer-from-cursor. Mirrors the F4 local-transfer
/// spawn shape: run the operation on a tokio task, flatten
/// the outcome into a [`F3PullReply`], and send it back on
/// `tx` for the event loop to apply (generation-guarded by
/// `request_id`).
///
/// This is the TUI's own pull (the daemon streams bytes to
/// this process), so it uses default `PullSyncOptions` —
/// no mirror, no filter, no progress monitor. A non-mirror
/// pull needs only `run_pull_sync`; the mirror-purge half
/// (`apply_pull_mirror_purge`) is a no-op when
/// `mirror_mode = false`, so it's skipped.
/// d-37: live progress snapshot forwarded from a running
/// pull to the event loop. d-39: `bytes_per_sec` is the
/// average throughput (0 until ~1s elapsed).
struct F3PullProgress {
    request_id: u64,
    files: usize,
    bytes: u64,
    bytes_per_sec: u64,
}

/// d-37 round 2: fold one pull `ProgressEvent` into the
/// running `(files, bytes)` totals using pull-receive
/// semantics. Bytes come from `Payload` only; file count
/// from `FileComplete` only.
///
/// The TCP data-plane path emits BOTH
/// `Payload { files: 0, bytes: N }` and
/// `FileComplete { bytes: N }` for the same completed
/// file (`pipeline.rs` `execute_receive_pipeline`), so
/// adding bytes from both would double-count and the
/// footer would snap backward when the authoritative
/// reply total lands. The direct-gRPC path emits
/// `FileComplete { bytes: 0 }` (`pull.rs`
/// `finalize_active_file`) with bytes carried by
/// `Payload` — so counting bytes from `Payload` alone is
/// correct on both paths, and counting one file per
/// `FileComplete` is correct on both paths.
fn accumulate_pull_progress(
    files: &mut usize,
    bytes: &mut u64,
    event: &blit_core::remote::transfer::ProgressEvent,
) {
    use blit_core::remote::transfer::ProgressEvent;
    match event {
        ProgressEvent::Payload { bytes: b, .. } => {
            *bytes = bytes.saturating_add(*b);
        }
        ProgressEvent::FileComplete { .. } => {
            *files = files.saturating_add(1);
        }
        ProgressEvent::ManifestBatch { .. } => {}
    }
}

/// d-39: average pull throughput in bytes/sec.
///
/// Suppressed (returns 0) until at least one second has
/// elapsed — `bytes / tiny_elapsed` produces meaningless
/// multi-GiB/s spikes in the first moments of a transfer,
/// and the footer reads better with no rate than a wrong
/// one. After the warm-up it's a simple cumulative average
/// (`bytes / elapsed`), matching the "is it moving" intent
/// of the footer rather than an instantaneous rate.
fn pull_throughput(bytes: u64, elapsed_secs: f64) -> u64 {
    if elapsed_secs >= 1.0 {
        (bytes as f64 / elapsed_secs) as u64
    } else {
        0
    }
}

fn spawn_f3_pull(
    request_id: u64,
    source: RemoteEndpoint,
    dest_root: std::path::PathBuf,
    tx: mpsc::Sender<F3PullReply>,
    // d-37: live byte/file progress snapshots. `try_send`
    // means a full channel just drops an intermediate
    // update — the authoritative final count rides the
    // `F3PullReply`.
    progress_tx: mpsc::Sender<F3PullProgress>,
) {
    use blit_app::transfers::remote::{run_pull_sync, PullSyncExecution};
    use blit_core::remote::pull::PullSyncOptions;
    use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
    tokio::spawn(async move {
        // d-37: progress monitor. run_pull_sync reports
        // ProgressEvents into `pe_rx`; the forwarder
        // accumulates cumulative (files, bytes) via
        // `accumulate_pull_progress` and ships snapshots
        // to the UI.
        let (pe_tx, mut pe_rx) = mpsc::unbounded_channel::<ProgressEvent>();
        let progress = RemoteTransferProgress::new(pe_tx);
        let forwarder = tokio::spawn(async move {
            let started = Instant::now();
            let mut files = 0usize;
            let mut bytes = 0u64;
            while let Some(event) = pe_rx.recv().await {
                accumulate_pull_progress(&mut files, &mut bytes, &event);
                let bytes_per_sec = pull_throughput(bytes, started.elapsed().as_secs_f64());
                // Lossy on a full channel — progress is
                // approximate; the reply carries the truth.
                let _ = progress_tx.try_send(F3PullProgress {
                    request_id,
                    files,
                    bytes,
                    bytes_per_sec,
                });
            }
        });

        let remote_label = source.display();
        let execution = PullSyncExecution {
            remote: source,
            dest_root,
            options: PullSyncOptions::default(),
            compute_checksums: false,
            mirror_mode: false,
            remote_label,
        };
        let result = run_pull_sync(execution, Some(&progress))
            .await
            .map(|outcome| {
                (
                    outcome.report.files_transferred,
                    outcome.report.bytes_transferred,
                )
            })
            .map_err(|err| format!("{err:#}"));
        // Close the progress channel → the forwarder drains
        // and exits before we send the terminal reply.
        drop(progress);
        let _ = forwarder.await;
        let _ = tx.send(F3PullReply { request_id, result }).await;
    });
}

/// d-24 round 2: how much wall-clock time remains before the
/// d-23 auto-hide kicks in on a Done/Error cancel fragment.
///
/// Returns `Some(remaining)` only while the fragment is still
/// visible. `None` for:
/// - `Idle` / `Sending` — no deadline (Sending waits for the
///   RPC reply, not a timer).
/// - Already-expired Done/Error — the renderer already returns
///   `Hidden`, so no further wakeup is needed.
///
/// The event loop reads this to ensure a short
/// `cancel_status_ttl_ms` isn't silently bounded by a longer
/// `live_tick.interval_ms` (round-1 R2 reopen). The fix is
/// `min(live_tick_interval, remaining)` while F2 is visible.
fn cancel_status_remaining_ttl(
    status: &F2CancelStatus,
    now: Instant,
    ttl: std::time::Duration,
) -> Option<std::time::Duration> {
    let finished_at = match status {
        F2CancelStatus::Done { finished_at, .. } => *finished_at,
        F2CancelStatus::Error { finished_at, .. } => *finished_at,
        // d-30: BatchInitiated has a finished_at like
        // Done/Error — the loop must wake to hide it on
        // the same TTL boundary as the single-cancel
        // variants.
        F2CancelStatus::BatchInitiated { finished_at, .. } => *finished_at,
        _ => return None,
    };
    let elapsed = now.saturating_duration_since(finished_at);
    if elapsed >= ttl {
        None
    } else {
        Some(ttl - elapsed)
    }
}

/// d-24 round 2: pick the actual sleep budget for the loop's
/// optional `live_tick` future.
///
/// - When the live tick is needed AND a cancel fragment is
///   pending, sleep the shorter of the two (cancel deadline
///   wins for short TTLs).
/// - When only the live tick is needed, use its interval.
/// - When only a cancel fragment is pending (no other
///   freshness-driven ticks), wake just for the deadline.
/// - When neither applies, return `None` — the loop sleeps
///   indefinitely waiting on real events.
fn compute_tick_budget(
    needs_live_tick: bool,
    live_tick_interval: std::time::Duration,
    cancel_remaining: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    match (needs_live_tick, cancel_remaining) {
        (true, Some(rem)) => Some(live_tick_interval.min(rem)),
        (true, None) => Some(live_tick_interval),
        (false, Some(rem)) => Some(rem),
        (false, None) => None,
    }
}

/// d-30 R2: local container for "what the operator
/// confirmed" — moved out of `cancel_status` so the
/// mutable borrow on `app.cancel_status` doesn't live
/// across the spawn. Single carries one id; Batch
/// carries the frozen list captured at prompt creation.
enum ConfirmedCancel {
    Single(String),
    Batch(Vec<String>),
}

/// d-30 / d-30 R2: snapshot the current active-transfer
/// ids in a stable order. Called once when the operator
/// presses `Shift+X` — the result is then either stored
/// on `ConfirmingBatch` (confirm path) or fed directly
/// into `spawn_cancels_for_ids` (non-confirm path).
fn snapshot_active_ids(transfers: &state::TransfersState) -> Vec<String> {
    transfers
        .active_rows()
        .into_iter()
        .map(|row| row.transfer_id.clone())
        .collect()
}

/// d-30 / d-30 R2: spawn one CancelJob RPC per id.
/// Returns the count of RPCs dispatched.
///
/// Each RPC uses the same `cancel_reply_tx` channel as
/// the single-cancel path; the reply arm in the event
/// loop discards replies whose request_id doesn't
/// match the *current* `Sending.request_id`, so batch
/// replies are dropped harmlessly. Operators see the
/// per-transfer outcomes via the Subscribe stream's
/// `TransferComplete` / `TransferError` events, which
/// is the same channel that displays normal transfer
/// completions.
fn spawn_cancels_for_ids(
    ids: Vec<String>,
    endpoint: &blit_core::remote::RemoteEndpoint,
    cancel_request_seq: &mut u64,
    tx: &mpsc::Sender<CancelReply>,
) -> usize {
    let count = ids.len();
    for id in ids {
        *cancel_request_seq += 1;
        let rid = *cancel_request_seq;
        spawn_cancel_transfer(rid, endpoint.clone(), id, tx.clone());
    }
    count
}

/// d-22: spawn a CancelJob RPC against `endpoint` for
/// `transfer_id`. Reply lands on `tx` as a [`CancelReply`].
/// The async machinery exists in `blit_app::admin::jobs::cancel`;
/// this is a thin wrapper that flattens the Result into the
/// reply envelope.
fn spawn_cancel_transfer(
    request_id: u64,
    endpoint: blit_core::remote::RemoteEndpoint,
    transfer_id: String,
    tx: mpsc::Sender<CancelReply>,
) {
    tokio::spawn(async move {
        let result = blit_app::admin::jobs::cancel(&endpoint, &transfer_id)
            .await
            .map_err(|err| format!("{err:#}"));
        let _ = tx
            .send(CancelReply {
                request_id,
                transfer_id,
                result,
            })
            .await;
    });
}

/// e-7: bridge from the config's `RawColor` (which lives
/// in `config` to avoid leaking ratatui types into the
/// schema layer) to the ratatui color used by the
/// renderer.
fn raw_color_to_ratatui(c: config::RawColor) -> ratatui::style::Color {
    use ratatui::style::Color;
    match c {
        config::RawColor::Black => Color::Black,
        config::RawColor::Red => Color::Red,
        config::RawColor::Green => Color::Green,
        config::RawColor::Yellow => Color::Yellow,
        config::RawColor::Blue => Color::Blue,
        config::RawColor::Magenta => Color::Magenta,
        config::RawColor::Cyan => Color::Cyan,
        config::RawColor::Gray => Color::Gray,
        config::RawColor::DarkGray => Color::DarkGray,
        config::RawColor::LightRed => Color::LightRed,
        config::RawColor::LightGreen => Color::LightGreen,
        config::RawColor::LightYellow => Color::LightYellow,
        config::RawColor::LightBlue => Color::LightBlue,
        config::RawColor::LightMagenta => Color::LightMagenta,
        config::RawColor::LightCyan => Color::LightCyan,
        config::RawColor::White => Color::White,
    }
}

/// `true` when the event loop should arm the 500ms
/// live-tick wakeup. The render path uses `now: Instant`
/// in several places that visibly tick — d-9 added the
/// initial F4 transfer/verify gate; d-11 extends to the
/// per-pane "fetched Xs ago" freshness footers on F1, F3,
/// and F4.
///
/// Pane-specific conditions:
/// - F1: `DaemonsState::has_live_timestamp()` covers
///   either the `Live` footer or a cached `Loaded`
///   detail block on the selected row. Round 2 fix: the
///   detail line keeps showing "as of Xs ago" even when
///   discovery drops to Degraded, so the gate must too.
/// - F2: `TransfersState::last_event_at()` is `Some` once
///   any Subscribe event or GetState snapshot has landed
///   (d-13). Pre-d-13 F2 didn't render anything against
///   `now` so this was false.
/// - F3: `BrowseFetchStatus::Loaded` shows "loaded · Xs ago".
/// - F4: `ProfileFetchStatus::Loaded` ticks the footer
///   (even when no transfer/verify run is active).
///
/// Confirm prompts and pure-Idle states deliberately
/// don't tick — there's nothing visible that depends on
/// the current time.
fn needs_live_tick(app: &AppState) -> bool {
    if app.transfer.is_running() || app.verify.is_running() {
        return true;
    }
    // d-36: tick while a reload banner is showing so it
    // auto-expires (the loop clears it once past TTL).
    if app.reload_banner.is_some() {
        return true;
    }
    // d-38: tick while an F3 pull Done/Error fragment is
    // showing so it auto-hides on its TTL.
    if app.f3_pull.is_terminal() {
        return true;
    }
    match app.current_screen {
        Screen::F1 => app.daemons.has_live_timestamp(),
        Screen::F2 => app.transfers.last_event_at().is_some(),
        Screen::F3 => matches!(
            app.browse.status(),
            browse::BrowseFetchStatus::Loaded { .. }
        ),
        Screen::F4 => matches!(
            app.profile.status(),
            profile::ProfileFetchStatus::Loaded { .. }
        ),
    }
}

/// d-12: predicate for the router's Esc-cancels-confirm
/// intercept. Returns true ONLY for bare Esc (no Ctrl /
/// Alt modifiers) while a confirmation prompt is open.
/// The router calls this BEFORE `handle_verify_keystroke`
/// and `key_action` so the confirm-cancel branch absorbs
/// the keystroke even if the operator has Tab-ed into the
/// Verify form's edit mode mid-confirm (d-12 round-2 fix
/// — pre-fix the Verify keystroke handler ate the Esc and
/// the confirm stayed visible with no way out).
///
/// d-29: extended to cover F2's cancel-confirm prompt
/// (`[transfer] confirm_cancel`). Whichever state
/// machine has a pending confirm, Esc reverts it.
fn esc_cancels_confirm(key: &KeyEvent, app: &AppState) -> bool {
    key.code == KeyCode::Esc
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && (app.transfer.is_confirming() || app.cancel_status.is_confirming())
}

/// `true` when the operator can kick a local transfer:
/// both Verify fields are non-empty AND no transfer is
/// running or awaiting a destructive-confirm prompt.
fn can_start_transfer(app: &AppState) -> bool {
    !app.verify.source.trim().is_empty()
        && !app.verify.destination.trim().is_empty()
        && !app.transfer.is_busy()
}

/// Synchronously validate + resolve the Verify form's raw
/// strings into a `(src, dst)` pair of local filesystem
/// paths. Mirrors `crates/blit-cli/src/transfers/mod.rs:101`:
///
/// 1. Parse both endpoints (`parse_transfer_endpoint`).
/// 2. Reject remote endpoints — F4 only kicks local
///    transfers (the CLI dispatches remote routes through
///    daemon RPCs; the TUI's Verify form is local-only).
/// 3. Resolve the destination through
///    `resolve_destination` so "copy /tmp/src /tmp/dst/"
///    nests into `/tmp/dst/src`, matching `blit copy`
///    semantics including the rsync trailing-slash rule.
///
/// Returns the resolved `(src, dst)` paths, or an
/// `Err(message)` formatted for the F4 transfer block.
fn prepare_local_transfer(
    raw_source: &str,
    raw_destination: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    use blit_app::endpoints::{parse_transfer_endpoint, Endpoint};
    use blit_app::transfers::resolution::resolve_destination;

    let src_endpoint =
        parse_transfer_endpoint(raw_source).map_err(|e| format!("parse source: {e:#}"))?;
    let raw_dst = parse_transfer_endpoint(raw_destination)
        .map_err(|e| format!("parse destination: {e:#}"))?;
    let resolved_dst = resolve_destination(raw_source, raw_destination, &src_endpoint, raw_dst);

    match (src_endpoint, resolved_dst) {
        (Endpoint::Local(src), Endpoint::Local(dst)) => Ok((src, dst)),
        (Endpoint::Remote(_), _) | (_, Endpoint::Remote(_)) => {
            Err("F4 transfers only support local→local paths; \
             use the CLI for remote endpoints"
                .to_string())
        }
    }
}

/// Spawn a local copy / mirror via
/// `blit_app::transfers::local::run`. The caller has
/// already validated + resolved both paths through
/// [`prepare_local_transfer`], so this just forwards them
/// to the orchestrator and ferries the reply back.
///
/// `perf_history` is read from on-disk config at call time
/// — matches the CLI's `ctx.perf_history_enabled` snapshot
/// (`crates/blit-cli/src/transfers/local.rs:184`). The F4
/// `d` / `e` lifecycle keys can flip this setting, and a
/// transfer launched immediately after must honor the new
/// value.
fn spawn_local_transfer(
    request_id: u64,
    kind: transfer::TransferKind,
    source: std::path::PathBuf,
    destination: std::path::PathBuf,
    tx: mpsc::Sender<TransferReply>,
) {
    tokio::spawn(async move {
        let perf_history_enabled = blit_core::perf_history::perf_history_enabled().unwrap_or(true);
        let options = blit_core::orchestrator::LocalMirrorOptions {
            mirror: matches!(kind, transfer::TransferKind::Mirror),
            perf_history: perf_history_enabled,
            ..Default::default()
        };
        let result = blit_app::transfers::local::run(&source, &destination, options)
            .await
            .map_err(|err| format!("{err:#}"));
        let _ = tx
            .send(TransferReply {
                request_id,
                kind,
                result,
            })
            .await;
    });
}

/// Spawn a local move = copy + source-purge. Mirrors the
/// CLI's `blit move` shape
/// (`crates/blit-cli/src/transfers/mod.rs:430-503`):
///
/// 1. Run `transfers::local::run` with `mirror=false`.
/// 2. If `summary.unreadable_paths` is non-empty, refuse
///    to delete the source — files we couldn't read were
///    skipped during the copy, so removing them from the
///    source side would lose data. This is the R47-F4
///    data-loss gate; the TUI must enforce it too.
/// 3. Otherwise delete the source (`remove_dir_all` for
///    directories, `remove_file` for files).
///
/// Surfaces the `LocalMirrorSummary` on success (so the
/// Done banner shows the same planned/copied/bytes numbers
/// as copy/mirror), or a flat error string on either the
/// copy failure or the post-copy purge failure / safety
/// refusal.
fn spawn_local_move(
    request_id: u64,
    source: std::path::PathBuf,
    destination: std::path::PathBuf,
    tx: mpsc::Sender<TransferReply>,
) {
    tokio::spawn(async move {
        let result = perform_local_move(&source, &destination).await;
        let _ = tx
            .send(TransferReply {
                request_id,
                kind: transfer::TransferKind::Move,
                result,
            })
            .await;
    });
}

/// Async core of [`spawn_local_move`], split out so it can
/// be exercised by `#[tokio::test]` without going through
/// the spawn/channel plumbing.
async fn perform_local_move(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<blit_core::orchestrator::LocalMirrorSummary, String> {
    let perf_history_enabled = blit_core::perf_history::perf_history_enabled().unwrap_or(true);
    let options = blit_core::orchestrator::LocalMirrorOptions {
        mirror: false,
        perf_history: perf_history_enabled,
        ..Default::default()
    };
    let summary = blit_app::transfers::local::run(source, destination, options)
        .await
        .map_err(|err| format!("{err:#}"))?;

    if !summary.unreadable_paths.is_empty() {
        // R47-F4 (data-loss): refuse purge on incomplete
        // scan. Quote the first few unreadable paths so
        // the operator can act on the message.
        let preview = summary
            .unreadable_paths
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "refusing to remove source: scan was incomplete ({} unreadable entr{}); \
             first {} reported: {}. Resolve the scan errors (typically permissions) \
             and re-run.",
            summary.unreadable_paths.len(),
            if summary.unreadable_paths.len() == 1 {
                "y"
            } else {
                "ies"
            },
            summary.unreadable_paths.len().min(3),
            preview,
        ));
    }

    if source.is_dir() {
        tokio::fs::remove_dir_all(source)
            .await
            .map_err(|e| format!("removing {}: {e}", source.display()))?;
    } else if source.is_file() {
        tokio::fs::remove_file(source)
            .await
            .map_err(|e| format!("removing {}: {e}", source.display()))?;
    }
    // If the source was already gone (e.g. concurrent
    // delete), treat as success — the post-condition
    // "source no longer exists" holds.

    Ok(summary)
}

/// Reply envelope from the F4 Diagnostics dump task.
/// Same generation pattern as VerifyReply.
struct DiagnosticsReply {
    request_id: u64,
    result: Result<std::path::PathBuf, String>,
}

/// Spawn a diagnostics-dump task on a blocking worker.
/// Builds the JSON snapshot via
/// `blit_app::diagnostics::dump::endpoint_snapshot` for
/// both source and destination, writes it to
/// `~/.config/blit/diagnostics-<unix-ms>.json`, and posts
/// the resulting path through `tx`.
fn spawn_diagnostics_dump(
    request_id: u64,
    source: String,
    destination: String,
    tx: mpsc::Sender<DiagnosticsReply>,
) {
    tokio::spawn(async move {
        let result =
            tokio::task::spawn_blocking(move || run_diagnostics_dump(&source, &destination)).await;
        let envelope = match result {
            Ok(Ok(path)) => Ok(path),
            Ok(Err(msg)) => Err(msg),
            Err(join_err) => Err(format!("diagnostics task panicked: {join_err}")),
        };
        let _ = tx
            .send(DiagnosticsReply {
                request_id,
                result: envelope,
            })
            .await;
    });
}

/// Synchronous core of the diagnostics dump. Mirrors the
/// CLI's `run_diagnostics_dump` shape (see
/// `crates/blit-cli/src/diagnostics.rs`) so a TUI-generated
/// file is interchangeable with the CLI's `blit diagnostics
/// dump --json` output. Specifically:
///
/// - `destination` and `same_device` are computed against
///   the RESOLVED destination (post-`resolve_destination`),
///   so a source directory copied into a container reports
///   the effective target.
/// - `rsync_resolution` block carries the four flags the
///   CLI emits + a `resolution_changed` boolean.
/// - `invocation` lists the TUI process's argv so bug
///   reports can correlate the dump with how `blit-tui`
///   was launched.
fn run_diagnostics_dump(source: &str, destination: &str) -> Result<std::path::PathBuf, String> {
    let snapshot = build_diagnostics_snapshot(source, destination)?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("clock: {e}"))?
        .as_millis();
    let dir = blit_core::perf_history::config_dir().map_err(|e| format!("config dir: {e:#}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    let path = dir.join(format!("diagnostics-{now_ms}.json"));
    let pretty = serde_json::to_string_pretty(&snapshot).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, pretty).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Build the diagnostics JSON snapshot. Exposed as a pure
/// helper so tests can compare the TUI's JSON shape
/// against the CLI's without writing to disk.
fn build_diagnostics_snapshot(
    source: &str,
    destination: &str,
) -> Result<serde_json::Value, String> {
    use blit_app::diagnostics::dump::{endpoint_display, endpoint_snapshot, same_device};
    use blit_app::endpoints::parse_transfer_endpoint;
    use blit_app::transfers::resolution::{
        dest_is_container, resolve_destination, source_is_contents,
    };

    let src_endpoint =
        parse_transfer_endpoint(source).map_err(|e| format!("parse source: {e:#}"))?;
    let raw_dst =
        parse_transfer_endpoint(destination).map_err(|e| format!("parse destination: {e:#}"))?;
    let pre_resolve_dst = raw_dst.clone();
    let resolved_dst = resolve_destination(source, destination, &src_endpoint, raw_dst);

    let source_contents_mode = source_is_contents(source);
    let dest_is_container_flag = dest_is_container(destination, &pre_resolve_dst);

    let pre_resolve_json = endpoint_display(&pre_resolve_dst);
    let resolved_display = endpoint_display(&resolved_dst);

    Ok(serde_json::json!({
        "blit_version": env!("CARGO_PKG_VERSION"),
        "invocation": std::env::args().collect::<Vec<_>>(),
        "source": endpoint_snapshot(source, &src_endpoint),
        // Destination & same_device are evaluated against
        // the RESOLVED endpoint (matches CLI behavior).
        "destination": endpoint_snapshot(destination, &resolved_dst),
        "rsync_resolution": {
            "source_is_contents": source_contents_mode,
            "destination_is_container": dest_is_container_flag,
            "pre_resolve_destination": pre_resolve_json,
            "resolved_destination": resolved_display,
            "resolution_changed": pre_resolve_json != resolved_display,
        },
        "same_device": same_device(&src_endpoint, &resolved_dst),
    }))
}

/// Handle a keystroke when F4's Verify form has focus.
/// Returns `true` if the keystroke was consumed; `false`
/// when the dispatcher should fall through to the normal
/// action handler (F-keys for navigation, Ctrl-c for
/// emergency quit).
fn handle_verify_keystroke(
    key: &KeyEvent,
    app: &mut AppState,
    verify_run_tx: &mpsc::Sender<VerifyReply>,
) -> bool {
    // Ctrl-c → emergency quit; let dispatcher handle.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    // d-18: Ctrl-U clears the focused field (terminal
    // "kill-line" convention). Same invalidation contract
    // as character edits — handled inside
    // `clear_focused_field`.
    if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.verify.clear_focused_field();
        app.transfer.cancel_confirm();
        return true;
    }
    // F-keys → navigate; let dispatcher handle.
    if let KeyCode::F(_) = key.code {
        return false;
    }
    // `?` is a global help shortcut. Even while the
    // Verify form has focus, the operator should be able
    // to open the keymap overlay — that's the one
    // pane state where they're MOST likely to need it.
    // Return false so the dispatcher's ToggleHelp arm
    // runs. (e-1 round-2 fix.)
    if key.code == KeyCode::Char('?')
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return false;
    }
    match key.code {
        KeyCode::Esc => {
            // Esc drops focus without quitting the TUI.
            app.verify.clear_focus();
            true
        }
        KeyCode::Tab => {
            app.verify.cycle_focus();
            true
        }
        KeyCode::Enter => {
            if app.verify.can_run() {
                let gen = app.verify.begin_run();
                spawn_verify_run(
                    gen,
                    app.verify.source.clone(),
                    app.verify.destination.clone(),
                    app.verify.use_checksum(),
                    app.verify.one_way(),
                    verify_run_tx.clone(),
                );
            }
            true
        }
        KeyCode::Backspace => {
            app.verify.backspace();
            // d-4 R2: editing the Verify form pulls the rug
            // out from under any pending mirror-confirm
            // prompt — the path the operator confirmed is
            // no longer what the form holds. Drop the
            // confirmation back to Idle so the operator
            // re-presses `M` on the new paths.
            app.transfer.cancel_confirm();
            true
        }
        KeyCode::Char(c) => {
            // Skip modifier combos (Alt-x etc.) so they
            // don't sneak in as garbled text.
            if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                return false;
            }
            app.verify.insert_char(c);
            app.transfer.cancel_confirm();
            true
        }
        _ => false,
    }
}

/// d-26: F3 filter-mode keystroke router. Mirrors the
/// `handle_verify_keystroke` shape — returns `true` when
/// the key was absorbed (so the outer dispatcher skips
/// `key_action`), or `false` when the key should pass
/// through (Ctrl-c emergency quit, F-keys, `?` help).
///
/// `/` while NOT editing is handled in the action
/// dispatcher (`UserAction::F3FilterBegin`); this
/// function only runs when `is_editing_filter` is true.
fn handle_f3_filter_keystroke(key: &KeyEvent, app: &mut AppState) -> bool {
    // Emergency quit always falls through to key_action.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    // F-keys navigate panes; let the dispatcher handle.
    if let KeyCode::F(_) = key.code {
        return false;
    }
    // `?` is the global help toggle — even mid-filter.
    if key.code == KeyCode::Char('?')
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return false;
    }
    match key.code {
        KeyCode::Esc => {
            app.browse.cancel_filter();
            true
        }
        KeyCode::Enter => {
            app.browse.commit_filter();
            true
        }
        KeyCode::Backspace => {
            app.browse.pop_filter_char();
            true
        }
        KeyCode::Char(c) => {
            // Skip Ctrl-/Alt-modified chars so terminal
            // shortcuts don't get appended as garbled
            // filter text.
            if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                return false;
            }
            app.browse.push_filter_char(c);
            true
        }
        _ => false,
    }
}

/// d-35: F3 pull destination-prompt keystroke router.
/// Same shape as `handle_f3_filter_keystroke` — returns
/// `true` when the key was absorbed into the prompt,
/// `false` when it should bubble to the dispatcher
/// (Ctrl-c quit, F-key nav, `?` help). Only runs while
/// `f3_pull.is_entering_dest()` is true.
///
/// On `Enter` with a non-empty dest, fires the pull RPC
/// (via `begin_run` → `spawn_f3_pull`) and transitions to
/// `Running`. On `Esc`, aborts the prompt.
fn handle_f3_pull_keystroke(key: &KeyEvent, app: &mut AppState) -> bool {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    if let KeyCode::F(_) = key.code {
        return false;
    }
    if key.code == KeyCode::Char('?')
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return false;
    }
    match key.code {
        KeyCode::Esc => {
            app.f3_pull.cancel();
            true
        }
        KeyCode::Enter => {
            if let Some(launch) = app.f3_pull.begin_run() {
                spawn_f3_pull(
                    launch.request_id,
                    launch.source,
                    launch.dest_root,
                    app.f3_pull_reply_tx.clone(),
                    app.f3_pull_progress_tx.clone(),
                );
            }
            // Absorb Enter even on an empty dest (begin_run
            // is a no-op there and the prompt stays open).
            true
        }
        KeyCode::Backspace => {
            app.f3_pull.pop_char();
            true
        }
        KeyCode::Char(c) => {
            if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                return false;
            }
            app.f3_pull.push_char(c);
            true
        }
        _ => false,
    }
}

/// Spawn a `compare_trees` run on a blocking task. Both
/// inputs are local path strings; the task parses them to
/// `PathBuf` and runs the comparison with a default
/// `FileFilter`. `use_checksum` follows the
/// `VerifyState`'s mode toggle (d-6) — `false` is the
/// default size+mtime compare matching rsync; `true` is
/// per-file content checksum. `one_way` follows the d-7
/// direction toggle — `false` reports both
/// `missing-on-src` and `missing-on-dst`; `true` skips
/// the dst-walk and matches `blit check --one-way`.
fn spawn_verify_run(
    request_id: u64,
    source: String,
    destination: String,
    use_checksum: bool,
    one_way: bool,
    tx: mpsc::Sender<VerifyReply>,
) {
    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            let src = std::path::PathBuf::from(&source);
            let dst = std::path::PathBuf::from(&destination);
            blit_app::check::compare_trees(
                &src,
                &dst,
                use_checksum,
                one_way,
                blit_core::fs_enum::FileFilter::default(),
            )
        })
        .await;
        let envelope = match result {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(err)) => Err(format!("{err:#}")),
            Err(join_err) => Err(format!("verify task panicked: {join_err}")),
        };
        let _ = tx
            .send(VerifyReply {
                request_id,
                result: envelope,
            })
            .await;
    });
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
/// navigation today). `Navigate` bubbles up to the
/// router so the top-level can switch which pane is
/// active.
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
    /// Switch to a different pane. Bubbles back to the
    /// router so the top-level can switch which pane is
    /// active.
    Navigate(Screen),
    /// F4: `c` clears the perf-history file.
    ProfileClear,
    /// F4: `d` disables history recording (new transfers
    /// stop adding records).
    ProfileDisable,
    /// F4: `e` re-enables history recording.
    ProfileEnable,
    /// F4: `s` dumps a diagnostics snapshot to disk for
    /// the current Source/Destination form pair.
    DiagnosticsDump,
    /// Toggle the `?` help overlay. Works from every pane.
    ToggleHelp,
    /// F4: `C` triggers a local copy from the Verify
    /// form's Source → Destination.
    TransferCopy,
    /// F4: `M` opens the destructive-mirror confirmation
    /// prompt. Actual mirror only fires after `Y`.
    TransferMirror,
    /// F4: `V` opens the source-deleting move
    /// confirmation prompt. Actual move only fires after
    /// `Y`. Move = copy + delete-source, so it's the most
    /// destructive of the three triggers.
    TransferMove,
    /// F4: `y` confirms a pending mirror-or-move prompt
    /// and kicks the actual transfer.
    TransferMirrorConfirm,
    /// F4: `n` cancels a pending mirror-or-move prompt.
    TransferCancel,
    /// F4: `H` toggles the Verify form between size+mtime
    /// (default, rsync-style) and per-file checksum.
    /// Invalidates any prior result so the displayed
    /// counts always match the current mode.
    ToggleVerifyChecksum,
    /// F4: `O` toggles the Verify form between two-way
    /// (default — reports `missing-on-src` and
    /// `missing-on-dst`) and one-way (matches
    /// `blit check --one-way`: skips the dst walk). Same
    /// invalidation contract as ToggleVerifyChecksum.
    ToggleVerifyOneWay,
    /// d-22: F2 only. `K` cancels the cursor-selected
    /// active transfer via the daemon's CancelJob RPC.
    /// No-op if the cursor isn't anchored on a live row
    /// (operator presses j/k first to select).
    CancelSelectedTransfer,
    /// d-26: F3 only. `/` enters filter-edit mode for the
    /// current view's row list. Subsequent chars route
    /// through `handle_f3_filter_keystroke` (separate
    /// from the action dispatcher).
    F3FilterBegin,
    /// d-30: F2 only. `Shift+X` cancels every currently
    /// active transfer in one keystroke. When
    /// `[transfer] confirm_cancel = true` the dispatcher
    /// prompts for `y/N` first; otherwise the cancels
    /// fire immediately. Outcomes propagate via the
    /// Subscribe stream rather than the per-reply path.
    CancelAllActiveTransfers,
    /// d-35: F3 only. `p` opens the pull destination
    /// prompt for the cursor-selected remote path. No-op
    /// if no remote / nothing selectable / a pull is
    /// already in flight.
    F3PullBegin,
    /// d-36: `Ctrl+R` re-reads `tui.toml` and swaps the
    /// live config (theme, tick interval, transfer knobs)
    /// without restarting the TUI. Global — works from
    /// every pane. A parse error keeps the current config
    /// and surfaces the error in the tab-strip banner.
    ReloadConfig,
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
    // d-36: `Ctrl+R` reloads tui.toml. Checked before the
    // plain `Char('r') => Refresh` arm below so the Ctrl
    // modifier disambiguates the two.
    if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(UserAction::ReloadConfig);
    }
    // F1-F4 navigate to the named pane. Available from
    // every pane — that's the whole point of the router.
    if let KeyCode::F(n) = key.code {
        match n {
            1 => return Some(UserAction::Navigate(Screen::F1)),
            2 => return Some(UserAction::Navigate(Screen::F2)),
            3 => return Some(UserAction::Navigate(Screen::F3)),
            4 => return Some(UserAction::Navigate(Screen::F4)),
            _ => {}
        }
    }
    // d-19: digit aliases for tab nav. Some terminals
    // (mosh, certain SSH proxies, screen-multiplexers
    // running inside CI environments) drop F-keys
    // entirely, mapping them to escape sequences the
    // operator's terminal doesn't translate back. Bare
    // `1`-`4` always survive. When the Verify form has
    // edit focus, handle_verify_keystroke captures the
    // digit as text input before this dispatcher runs,
    // so typing a path with "config/1/data" still works.
    if let KeyCode::Char(c) = key.code {
        if !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            match c {
                '1' => return Some(UserAction::Navigate(Screen::F1)),
                '2' => return Some(UserAction::Navigate(Screen::F2)),
                '3' => return Some(UserAction::Navigate(Screen::F3)),
                '4' => return Some(UserAction::Navigate(Screen::F4)),
                _ => {}
            }
        }
    }
    match key.code {
        KeyCode::Char('r') => Some(UserAction::Refresh),
        KeyCode::Down | KeyCode::Char('j') => Some(UserAction::SelectNext),
        KeyCode::Up | KeyCode::Char('k') => Some(UserAction::SelectPrev),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Some(UserAction::Descend),
        KeyCode::Left | KeyCode::Char('h') => Some(UserAction::Ascend),
        // F4 profile lifecycle keys. The Ctrl-c quit
        // shortcut is intercepted by `should_quit` above
        // — only bare lowercase `c` reaches this arm.
        // Panes other than F4 ignore these variants in
        // `handle_pane_action`.
        KeyCode::Char('c') => Some(UserAction::ProfileClear),
        KeyCode::Char('d') => Some(UserAction::ProfileDisable),
        KeyCode::Char('e') => Some(UserAction::ProfileEnable),
        // `s` dumps a diagnostics snapshot. `d` is taken
        // by ProfileDisable; the TUI_DESIGN listing `[d]
        // dump` conflicts with `[d] disable` on the same
        // screen — we resolve the conflict by binding
        // dump on the mnemonic `s` (snapshot) key.
        KeyCode::Char('s') => Some(UserAction::DiagnosticsDump),
        // `?` toggles the global help overlay. The bare
        // `?` glyph on most layouts requires Shift, which
        // crossterm hands us as just `Char('?')`.
        KeyCode::Char('?') => Some(UserAction::ToggleHelp),
        // Capital C/M trigger local transfers from F4's
        // Verify form (Source → Destination). Capitals
        // chosen because lowercase c/d/e are taken by
        // ProfileClear/Disable/Enable on F4. F1/F2/F3
        // wildcard-ignore the variants below.
        KeyCode::Char('C') => Some(UserAction::TransferCopy),
        KeyCode::Char('M') => Some(UserAction::TransferMirror),
        // Capital `V` triggers the source-deleting move
        // confirm flow. Lowercase `v` is unmapped — kept
        // free for potential vim-style "visual mode" /
        // multi-select on a future F3 polish slice.
        KeyCode::Char('V') => Some(UserAction::TransferMove),
        // d-22: `K` (kill) cancels the F2-selected
        // active transfer. F1/F3/F4 ignore in their
        // dispatch arms.
        KeyCode::Char('K') => Some(UserAction::CancelSelectedTransfer),
        // d-26: `/` opens F3's filter input. F1/F2/F4
        // ignore the variant. While editing, the F3
        // filter keystroke handler absorbs all input
        // before this dispatcher runs.
        KeyCode::Char('/') => Some(UserAction::F3FilterBegin),
        // d-35: `p` (pull) on F3 opens the destination
        // prompt for the cursor-selected remote path.
        // Other panes ignore. While the prompt is open the
        // F3 pull keystroke handler absorbs input before
        // this dispatcher runs, so `p` is a text char then.
        KeyCode::Char('p') => Some(UserAction::F3PullBegin),
        // d-30: `X` (Shift+x) on F2 cancels every active
        // transfer in one keystroke. Other panes ignore.
        // Mnemonic: cross out everything.
        KeyCode::Char('X') => Some(UserAction::CancelAllActiveTransfers),
        // `H` toggles Verify mode (size+mtime ↔ checksum).
        // Capital chosen because lowercase `h` is the
        // Ascend / left-arrow alias used by F3 navigation.
        KeyCode::Char('H') => Some(UserAction::ToggleVerifyChecksum),
        // `O` (One-way) toggles Verify direction. The
        // mnemonic mirrors `--one-way` on `blit check`.
        // Lowercase `o` stays unmapped — reserved for
        // potential "open in editor" / "open module" in
        // a future polish.
        KeyCode::Char('O') => Some(UserAction::ToggleVerifyOneWay),
        // `y` / `n` confirm or cancel a pending mirror
        // prompt. The F4 dispatcher only acts on these
        // while `transfer.is_confirming_mirror()` is true —
        // otherwise they're no-ops. Both cases are
        // accepted to match the rsync-style `[y/N]` prompt
        // the CLI uses (`crates/blit-cli/src/transfers/mod.rs:182`).
        KeyCode::Char('Y') | KeyCode::Char('y') => Some(UserAction::TransferMirrorConfirm),
        KeyCode::Char('N') | KeyCode::Char('n') => Some(UserAction::TransferCancel),
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
                state.apply_event(event, Instant::now());
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
            state.replace_from_snapshot(snapshot, Instant::now());
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

    /// d-3 round-2 regression: the TUI diagnostics dump
    /// JSON must carry the same top-level shape as the
    /// CLI's `blit diagnostics dump --json` (modulo the
    /// timestamp, which is determined by the writer).
    /// Specifically, the snapshot must include the
    /// `rsync_resolution` block and compute
    /// `destination` / `same_device` against the RESOLVED
    /// destination.
    #[test]
    fn tui_dump_shape_matches_cli_contract() {
        // Use /tmp paths so resolve_destination has
        // something to chew on. With a non-trailing-slash
        // source and a non-existent dest path,
        // resolve_destination is a passthrough.
        let snapshot = build_diagnostics_snapshot("/tmp/a", "/tmp/b").expect("build snapshot");
        let obj = snapshot.as_object().expect("top-level is object");
        // Top-level keys that match the CLI's run_diagnostics_dump.
        assert!(obj.contains_key("blit_version"));
        assert!(obj.contains_key("invocation"));
        assert!(obj.contains_key("source"));
        assert!(obj.contains_key("destination"));
        assert!(obj.contains_key("rsync_resolution"));
        assert!(obj.contains_key("same_device"));
        // The rsync_resolution sub-object must have all
        // five fields the CLI emits.
        let resolution = obj
            .get("rsync_resolution")
            .and_then(|v| v.as_object())
            .expect("rsync_resolution is object");
        assert!(resolution.contains_key("source_is_contents"));
        assert!(resolution.contains_key("destination_is_container"));
        assert!(resolution.contains_key("pre_resolve_destination"));
        assert!(resolution.contains_key("resolved_destination"));
        assert!(resolution.contains_key("resolution_changed"));
    }

    /// d-3 round-2: when the source is a directory and
    /// the destination is a container (trailing slash),
    /// the resolved destination differs from the
    /// pre-resolved destination and the
    /// `resolution_changed` flag flips to true. This is
    /// the bug-bait case the reviewer called out — the
    /// TUI must NOT report the un-resolved destination.
    #[test]
    fn tui_dump_resolves_destination_for_container_targets() {
        // Make a temp src directory the resolver sees.
        let tmp = std::env::temp_dir().join(format!("blit-d-3-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("mkdir tmp");
        let src = tmp.join("payload");
        std::fs::create_dir_all(&src).expect("mkdir payload");
        let dst = tmp.join("container");
        std::fs::create_dir_all(&dst).expect("mkdir container");

        // Container destination = trailing-slash.
        let src_str = src.display().to_string();
        let dst_str = format!("{}/", dst.display());
        let snapshot = build_diagnostics_snapshot(&src_str, &dst_str).expect("build snapshot");

        let resolution = snapshot
            .get("rsync_resolution")
            .and_then(|v| v.as_object())
            .expect("rsync_resolution");
        // The source is NOT a /-suffixed contents-mode
        // path, so this is "nest under dst" semantics.
        // resolve_destination should append the basename.
        assert_eq!(
            resolution.get("source_is_contents"),
            Some(&serde_json::Value::Bool(false))
        );
        assert_eq!(
            resolution.get("destination_is_container"),
            Some(&serde_json::Value::Bool(true))
        );
        // pre_resolve != resolved means we'd nest.
        let pre = resolution
            .get("pre_resolve_destination")
            .and_then(|v| v.as_str())
            .unwrap();
        let post = resolution
            .get("resolved_destination")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_ne!(pre, post, "expected resolve_destination to change the path");
        assert_eq!(
            resolution.get("resolution_changed"),
            Some(&serde_json::Value::Bool(true))
        );

        // Clean up.
        let _ = std::fs::remove_dir_all(&tmp);
    }

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

    /// a1-6: F-keys F1..F4 map to Navigate(...) for the
    /// corresponding pane. Verified across all four keys.
    #[test]
    fn key_action_maps_f_keys_to_navigate() {
        let f = |n| key_action(&k(KeyCode::F(n)));
        assert!(matches!(f(1), Some(UserAction::Navigate(Screen::F1))));
        assert!(matches!(f(2), Some(UserAction::Navigate(Screen::F2))));
        assert!(matches!(f(3), Some(UserAction::Navigate(Screen::F3))));
        // d-19: digit aliases for tab nav. F1-F4 still
        // map but so do 1-4 — terminals that drop F-keys
        // (mosh / certain SSH proxies / CI muxers) can
        // still navigate. Helper closure pins each.
        let d = |c| key_action(&k(KeyCode::Char(c)));
        assert!(
            matches!(d('1'), Some(UserAction::Navigate(Screen::F1))),
            "`1` must map to F1 navigation",
        );
        assert!(matches!(d('2'), Some(UserAction::Navigate(Screen::F2))));
        assert!(matches!(d('3'), Some(UserAction::Navigate(Screen::F3))));
        assert!(matches!(d('4'), Some(UserAction::Navigate(Screen::F4))));
        // Out-of-range digits stay unmapped.
        assert!(d('5').is_none());
        assert!(d('0').is_none());
        // Ctrl-1 / Alt-1 fall through (don't claim
        // modifier combos the operator might use for
        // terminal escape sequences).
        for mods in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
            assert!(
                key_action(&KeyEvent {
                    code: KeyCode::Char('1'),
                    modifiers: mods,
                })
                .is_none(),
                "modifier+1 must not navigate (modifiers: {mods:?})",
            );
        }
        assert!(matches!(f(4), Some(UserAction::Navigate(Screen::F4))));
        // Out-of-range F-keys are not mapped (F5+ unused
        // today; the design reserves them for future help /
        // settings / etc.).
        assert!(f(5).is_none());
        assert!(f(12).is_none());
    }

    /// a1-6: ScreenArg → Screen mapping covers all four
    /// variants. Pins the CLI-to-router translation so a
    /// future ScreenArg variant can't silently default.
    #[test]
    fn screen_arg_to_screen_mapping_is_total() {
        assert_eq!(Screen::from(ScreenArg::F1), Screen::F1);
        assert_eq!(Screen::from(ScreenArg::F2), Screen::F2);
        assert_eq!(Screen::from(ScreenArg::F3), Screen::F3);
        assert_eq!(Screen::from(ScreenArg::F4), Screen::F4);
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
                                                               // `K` was unmapped before d-22; it now maps
                                                               // to CancelSelectedTransfer for the F2 cancel
                                                               // flow. Other capitals (C/M/V/H/O/Y/N) are
                                                               // also mapped now via earlier slices.
                                                               // Enter is now mapped (a1-4: F3 Descend) — it
                                                               // *isn't* in this "unmapped" list anymore.
    }

    // d-36: Ctrl+R config hot-reload.

    /// `Ctrl+R` maps to ReloadConfig; bare `r` stays
    /// Refresh (the Ctrl modifier disambiguates).
    #[test]
    fn key_action_maps_ctrl_r_to_reload_config() {
        let ctrl_r = KeyEvent {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert!(matches!(
            key_action(&ctrl_r),
            Some(UserAction::ReloadConfig)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('r'))),
            Some(UserAction::Refresh)
        ));
    }

    /// Reload success (no parse warning): the freshly
    /// loaded config is adopted and the banner is a green
    /// "config reloaded".
    #[test]
    fn classify_reload_success_adopts_new() {
        let mut loaded = config::TuiConfig::default();
        loaded.transfer.confirm_cancel = true;
        let current = config::TuiConfig::default();
        let (next, banner) = classify_reload(loaded, None, &current, Instant::now());
        assert!(next.transfer.confirm_cancel, "reloaded config adopted");
        assert!(banner.ok);
        assert_eq!(banner.message, "config reloaded");
    }

    /// Reload parse error: the CURRENT config is kept (not
    /// the defaults the loader returns on failure) and the
    /// banner carries the error.
    #[test]
    fn classify_reload_parse_error_keeps_current() {
        // The loader returns defaults on a parse error...
        let loaded = config::TuiConfig::default();
        // ...but `current` has a non-default value we must
        // NOT lose.
        let mut current = config::TuiConfig::default();
        current.transfer.confirm_cancel = true;
        let (next, banner) = classify_reload(
            loaded,
            Some("failed to parse tui.toml: …".to_string()),
            &current,
            Instant::now(),
        );
        assert!(
            next.transfer.confirm_cancel,
            "parse error must keep the current config, not reset to defaults"
        );
        assert!(!banner.ok);
        assert!(banner.message.contains("reload failed"));
    }

    /// The reload banner auto-hides after its TTL.
    #[test]
    fn reload_banner_visibility_expires() {
        let now = Instant::now();
        let banner = ReloadBanner {
            message: "config reloaded".to_string(),
            ok: true,
            shown_at: now,
        };
        assert!(banner.is_visible(now));
        assert!(banner.is_visible(now + std::time::Duration::from_secs(3)));
        assert!(!banner.is_visible(now + std::time::Duration::from_secs(5)));
    }

    /// d-36: needs_live_tick is true while a reload banner
    /// is set, so the loop wakes to expire it.
    #[test]
    fn needs_live_tick_true_while_reload_banner_set() {
        let mut app = make_test_app_state(Screen::F1);
        // F1 with no live timestamp → normally false.
        assert!(!needs_live_tick(&app));
        app.reload_banner = Some(ReloadBanner {
            message: "config reloaded".to_string(),
            ok: true,
            shown_at: Instant::now(),
        });
        assert!(needs_live_tick(&app));
    }

    /// d-22: `K` maps to CancelSelectedTransfer. F2's
    /// dispatcher honors it only when the cursor is
    /// anchored on a live row and there's a remote +
    /// no cancel in flight; other panes silently ignore.
    #[test]
    fn key_action_maps_cancel_selected_transfer() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('K'))),
            Some(UserAction::CancelSelectedTransfer)
        ));
    }

    // d-29: F2 cancel-confirm state machine pure tests.
    // The dispatch path is hard to drive end-to-end without
    // a fake daemon; these tests pin the state machine
    // transitions and the predicates the router consults.

    /// d-29: a Confirming variant reports `is_confirming`
    /// true and `is_sending` false.
    #[test]
    fn f2_cancel_status_confirming_predicates() {
        let s = F2CancelStatus::Confirming {
            transfer_id: "t-1".to_string(),
        };
        assert!(s.is_confirming());
        assert!(!s.is_sending());
    }

    /// d-29: Sending stays `is_sending` only.
    #[test]
    fn f2_cancel_status_sending_predicates() {
        let s = F2CancelStatus::Sending {
            transfer_id: "t-1".to_string(),
            request_id: 1,
        };
        assert!(s.is_sending());
        assert!(!s.is_confirming());
    }

    /// d-29: Idle / Done / Error all report false for
    /// both predicates.
    #[test]
    fn f2_cancel_status_idle_done_error_predicates() {
        let idle = F2CancelStatus::Idle;
        assert!(!idle.is_confirming());
        assert!(!idle.is_sending());

        let done = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at: Instant::now(),
        };
        assert!(!done.is_confirming());
        assert!(!done.is_sending());

        let err = F2CancelStatus::Error {
            transfer_id: "t-1".to_string(),
            message: "boom".to_string(),
            finished_at: Instant::now(),
        };
        assert!(!err.is_confirming());
        assert!(!err.is_sending());
    }

    /// d-29: the renderer-side bridge maps a Confirming
    /// state to the new `ConfirmingCancel` display variant.
    /// No TTL applies — bridge ignores `now`/`ttl`.
    #[test]
    fn cancel_status_to_display_renders_confirming() {
        let status = F2CancelStatus::Confirming {
            transfer_id: "t-1".to_string(),
        };
        let display =
            cancel_status_to_display(&status, Instant::now(), std::time::Duration::from_secs(60));
        match display {
            screens::f2::F2CancelDisplay::ConfirmingCancel { transfer_id } => {
                assert_eq!(transfer_id, "t-1");
            }
            other => panic!("expected ConfirmingCancel, got {other:?}"),
        }
    }

    /// d-29: `cancel_status_remaining_ttl` returns None
    /// for Confirming — the prompt has no deadline; the
    /// loop has nothing to wake up for.
    #[test]
    fn cancel_status_remaining_ttl_confirming_returns_none() {
        let status = F2CancelStatus::Confirming {
            transfer_id: "t-1".to_string(),
        };
        let remaining = cancel_status_remaining_ttl(
            &status,
            Instant::now(),
            std::time::Duration::from_secs(60),
        );
        assert!(remaining.is_none());
    }

    /// d-29: `esc_cancels_confirm` predicates returns true
    /// for either F4 (verify-transfer) OR F2 (cancel)
    /// confirm states.
    #[test]
    fn esc_cancels_confirm_routes_f2_cancel_confirm() {
        let mut app = make_test_app_state(Screen::F2);
        app.cancel_status = F2CancelStatus::Confirming {
            transfer_id: "t-1".to_string(),
        };
        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
        };
        assert!(
            esc_cancels_confirm(&esc, &app),
            "Esc must route to the F2 cancel-confirm reset path"
        );
    }

    /// d-29: predicate stays false when neither state
    /// machine is confirming — Esc bubbles to the normal
    /// dispatcher (which maps it to Quit on F2).
    #[test]
    fn esc_cancels_confirm_returns_false_when_neither_confirming() {
        let app = make_test_app_state(Screen::F2);
        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
        };
        assert!(!esc_cancels_confirm(&esc, &app));
    }

    // d-30: batch cancel state machine.

    /// d-30: `X` (Shift+x) maps to CancelAllActiveTransfers.
    #[test]
    fn key_action_maps_shift_x_to_cancel_all() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('X'))),
            Some(UserAction::CancelAllActiveTransfers)
        ));
    }

    /// d-30: ConfirmingBatch reports `is_confirming` true
    /// — Esc routing extends to the batch prompt.
    #[test]
    fn f2_cancel_status_confirming_batch_predicates() {
        let s = F2CancelStatus::ConfirmingBatch {
            transfer_ids: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        assert!(s.is_confirming());
        assert!(!s.is_sending());
    }

    /// d-30: BatchInitiated is a terminal-ish state that
    /// neither sends nor confirms. Its TTL drives
    /// auto-hide just like Done/Error.
    #[test]
    fn f2_cancel_status_batch_initiated_predicates() {
        let s = F2CancelStatus::BatchInitiated {
            count: 4,
            finished_at: Instant::now(),
        };
        assert!(!s.is_confirming());
        assert!(!s.is_sending());
    }

    /// d-30: bridge maps ConfirmingBatch → ConfirmingBatch
    /// display variant with the count (= len of frozen
    /// ids) preserved.
    #[test]
    fn cancel_status_to_display_renders_confirming_batch() {
        let status = F2CancelStatus::ConfirmingBatch {
            transfer_ids: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
        };
        let display =
            cancel_status_to_display(&status, Instant::now(), std::time::Duration::from_secs(60));
        match display {
            screens::f2::F2CancelDisplay::ConfirmingBatch { count } => {
                assert_eq!(count, 5);
            }
            other => panic!("expected ConfirmingBatch, got {other:?}"),
        }
    }

    /// d-30: bridge maps BatchInitiated within TTL →
    /// BatchInitiated display variant.
    #[test]
    fn cancel_status_to_display_renders_batch_initiated_within_ttl() {
        let now = Instant::now();
        let status = F2CancelStatus::BatchInitiated {
            count: 7,
            finished_at: now,
        };
        let display = cancel_status_to_display(&status, now, std::time::Duration::from_secs(5));
        match display {
            screens::f2::F2CancelDisplay::BatchInitiated { count } => {
                assert_eq!(count, 7);
            }
            other => panic!("expected BatchInitiated, got {other:?}"),
        }
    }

    /// d-30: past TTL the BatchInitiated fragment hides
    /// just like the single-cancel Done variant.
    #[test]
    fn cancel_status_to_display_hides_batch_initiated_past_ttl() {
        let finished_at = Instant::now();
        let later = finished_at + std::time::Duration::from_secs(6);
        let status = F2CancelStatus::BatchInitiated {
            count: 3,
            finished_at,
        };
        let display = cancel_status_to_display(&status, later, std::time::Duration::from_secs(5));
        assert!(matches!(display, screens::f2::F2CancelDisplay::Hidden));
    }

    /// d-30: ConfirmingBatch has no TTL — the prompt
    /// stays until the operator answers.
    #[test]
    fn cancel_status_remaining_ttl_confirming_batch_returns_none() {
        let status = F2CancelStatus::ConfirmingBatch {
            transfer_ids: vec!["a".into(), "b".into(), "c".into(), "d".into()],
        };
        let remaining =
            cancel_status_remaining_ttl(&status, Instant::now(), std::time::Duration::from_secs(5));
        assert!(remaining.is_none());
    }

    /// d-30: BatchInitiated drives the loop's sleep
    /// budget — the loop must wake to hide the fragment.
    #[test]
    fn cancel_status_remaining_ttl_batch_initiated_returns_positive() {
        let finished_at = Instant::now();
        let now = finished_at + std::time::Duration::from_millis(500);
        let status = F2CancelStatus::BatchInitiated {
            count: 2,
            finished_at,
        };
        let remaining =
            cancel_status_remaining_ttl(&status, now, std::time::Duration::from_secs(5));
        // 5s TTL − 500ms elapsed = 4500ms remaining.
        assert_eq!(remaining, Some(std::time::Duration::from_millis(4500)));
    }

    /// d-30: Esc routing covers ConfirmingBatch the same
    /// way as Confirming (single-cancel).
    #[test]
    fn esc_cancels_confirm_routes_f2_confirming_batch() {
        let mut app = make_test_app_state(Screen::F2);
        app.cancel_status = F2CancelStatus::ConfirmingBatch {
            transfer_ids: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
        };
        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
        };
        assert!(esc_cancels_confirm(&esc, &app));
    }

    /// d-30 round 2 REGRESSION: the reviewer-described
    /// TOCTOU race. Pre-fix, `ConfirmingBatch` stored
    /// only the count and the `y` arm re-snapshotted
    /// `transfers.active_rows()` at confirm time. The
    /// Subscribe stream keeps mutating that set, so an
    /// operator could confirm "cancel A, B" and have the
    /// `y` press actually cancel C, D.
    ///
    /// Post-fix the ids are frozen at prompt creation.
    /// This test pins the contract: build a state with
    /// ConfirmingBatch containing a specific id list,
    /// verify the bridge + display reflect THAT list's
    /// length, and verify reading the variant out via
    /// pattern-match returns the same Vec we put in.
    #[test]
    fn confirming_batch_freezes_ids_at_prompt_creation() {
        let frozen = vec!["t-A".to_string(), "t-B".to_string()];
        let status = F2CancelStatus::ConfirmingBatch {
            transfer_ids: frozen.clone(),
        };
        // Display reflects the frozen count.
        let display =
            cancel_status_to_display(&status, Instant::now(), std::time::Duration::from_secs(60));
        match display {
            screens::f2::F2CancelDisplay::ConfirmingBatch { count } => {
                assert_eq!(count, 2);
            }
            other => panic!("expected ConfirmingBatch, got {other:?}"),
        }
        // Pattern-match round-trip: the ids the dispatcher
        // would read on `y` are exactly the ones the
        // operator confirmed.
        match status {
            F2CancelStatus::ConfirmingBatch { transfer_ids } => {
                assert_eq!(transfer_ids, frozen);
            }
            other => panic!("expected ConfirmingBatch, got {other:?}"),
        }
    }

    /// d-30 R2: snapshot_active_ids returns the ids of
    /// every active row at the snapshot moment.
    #[test]
    fn snapshot_active_ids_captures_all_active_rows() {
        use blit_core::generated::{daemon_event, DaemonEvent, TransferStarted};
        let mut transfers = state::TransfersState::new();
        for id in ["t-A", "t-B", "t-C"] {
            let event = DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: id.to_string(),
                    kind: 0,
                    peer: String::new(),
                    module: String::new(),
                    path: String::new(),
                    start_unix_ms: 1_000_000,
                })),
            };
            transfers.apply_event(event, Instant::now());
        }
        let mut ids = snapshot_active_ids(&transfers);
        ids.sort();
        assert_eq!(
            ids,
            vec!["t-A".to_string(), "t-B".to_string(), "t-C".to_string()]
        );
    }

    /// d-30 R2: snapshot_active_ids returns empty for a
    /// fresh state. Pairs with the dispatcher's
    /// `if ids.is_empty()` no-op guard.
    #[test]
    fn snapshot_active_ids_empty_state() {
        let transfers = state::TransfersState::new();
        assert!(snapshot_active_ids(&transfers).is_empty());
    }

    // d-23: cancel-status TTL auto-clear. d-24 made the
    // TTL config-tunable; the tests use the default value
    // (5s) via `TransferDefaults::DEFAULT_CANCEL_TTL_MS`.

    /// Default TTL used by the d-23 cancel-fragment tests.
    /// Mirrors the production default before any operator
    /// override; d-24 moved the literal out of main.rs and
    /// into `config::TransferDefaults`.
    const TEST_CANCEL_TTL: std::time::Duration =
        std::time::Duration::from_millis(config::TransferDefaults::DEFAULT_CANCEL_TTL_MS);

    #[test]
    fn cancel_status_idle_renders_hidden() {
        let display =
            cancel_status_to_display(&F2CancelStatus::Idle, Instant::now(), TEST_CANCEL_TTL);
        assert!(matches!(display, screens::f2::F2CancelDisplay::Hidden));
    }

    #[test]
    fn cancel_status_sending_renders_sending_regardless_of_time() {
        let status = F2CancelStatus::Sending {
            transfer_id: "t-1".to_string(),
            request_id: 1,
        };
        // Sending has no TTL — it stays on screen until
        // the RPC reply lands (which transitions to
        // Done/Error).
        let display = cancel_status_to_display(&status, Instant::now(), TEST_CANCEL_TTL);
        match display {
            screens::f2::F2CancelDisplay::Sending { transfer_id } => {
                assert_eq!(transfer_id, "t-1");
            }
            other => panic!("expected Sending, got {other:?}"),
        }
    }

    #[test]
    fn cancel_status_done_within_ttl_renders_terminal_variant() {
        let now = Instant::now();
        let status = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at: now,
        };
        // Same Instant → within TTL.
        let display = cancel_status_to_display(&status, now, TEST_CANCEL_TTL);
        match display {
            screens::f2::F2CancelDisplay::Cancelled { transfer_id } => {
                assert_eq!(transfer_id, "t-1");
            }
            other => panic!("expected Cancelled, got {other:?}"),
        }
    }

    #[test]
    fn cancel_status_done_past_ttl_renders_hidden() {
        let finished_at = Instant::now();
        let later = finished_at + TEST_CANCEL_TTL + std::time::Duration::from_millis(1);
        let status = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at,
        };
        let display = cancel_status_to_display(&status, later, TEST_CANCEL_TTL);
        assert!(
            matches!(display, screens::f2::F2CancelDisplay::Hidden),
            "past-TTL Done must hide the fragment"
        );
    }

    #[test]
    fn cancel_status_error_past_ttl_renders_hidden() {
        let finished_at = Instant::now();
        let later = finished_at + TEST_CANCEL_TTL + std::time::Duration::from_millis(1);
        let status = F2CancelStatus::Error {
            transfer_id: "t-1".to_string(),
            message: "boom".to_string(),
            finished_at,
        };
        let display = cancel_status_to_display(&status, later, TEST_CANCEL_TTL);
        assert!(matches!(display, screens::f2::F2CancelDisplay::Hidden));
    }

    #[test]
    fn cancel_status_done_exactly_at_ttl_renders_hidden() {
        // The `>=` boundary: at exactly TTL elapsed, the
        // fragment is gone. Picks the safer side (less
        // clutter) when the operator's clock lands on
        // the exact boundary.
        let finished_at = Instant::now();
        let at_boundary = finished_at + TEST_CANCEL_TTL;
        let status = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at,
        };
        let display = cancel_status_to_display(&status, at_boundary, TEST_CANCEL_TTL);
        assert!(matches!(display, screens::f2::F2CancelDisplay::Hidden));
    }

    /// d-24: an operator-overridden TTL governs the
    /// fragment lifetime, not the default. Verifies the
    /// production code path picks up the clamped value
    /// from `cancel_status_ttl_ms_clamped()` rather than
    /// the old hardcoded 5s.
    #[test]
    fn cancel_status_respects_caller_supplied_ttl() {
        let finished_at = Instant::now();
        let custom_ttl = std::time::Duration::from_millis(1_000);
        let just_past = finished_at + custom_ttl + std::time::Duration::from_millis(1);
        let status = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at,
        };
        // Past the custom TTL → hidden, even though the
        // default 5s TTL would still be showing.
        let display = cancel_status_to_display(&status, just_past, custom_ttl);
        assert!(
            matches!(display, screens::f2::F2CancelDisplay::Hidden),
            "1s custom TTL must hide a 1.001s-old Done fragment"
        );
        // And same finished_at + a smaller `now` delta is
        // still showing under the same custom TTL.
        let within = finished_at + std::time::Duration::from_millis(500);
        let display = cancel_status_to_display(&status, within, custom_ttl);
        match display {
            screens::f2::F2CancelDisplay::Cancelled { transfer_id } => {
                assert_eq!(transfer_id, "t-1");
            }
            other => panic!("expected Cancelled within custom TTL, got {other:?}"),
        }
    }

    // d-24 round 2: the loop's sleep budget must respect
    // the cancel-TTL deadline when F2 is visible, so a
    // short cancel TTL isn't silently bounded by a long
    // live_tick interval.

    #[test]
    fn cancel_status_remaining_ttl_idle_returns_none() {
        let now = Instant::now();
        let ttl = std::time::Duration::from_millis(5_000);
        assert!(cancel_status_remaining_ttl(&F2CancelStatus::Idle, now, ttl).is_none());
    }

    #[test]
    fn cancel_status_remaining_ttl_sending_returns_none() {
        let now = Instant::now();
        let ttl = std::time::Duration::from_millis(5_000);
        let status = F2CancelStatus::Sending {
            transfer_id: "t-1".to_string(),
            request_id: 1,
        };
        // Sending waits on the RPC reply, not a timer —
        // the loop has no cancel-driven deadline.
        assert!(cancel_status_remaining_ttl(&status, now, ttl).is_none());
    }

    #[test]
    fn cancel_status_remaining_ttl_done_within_returns_positive() {
        let finished_at = Instant::now();
        let ttl = std::time::Duration::from_millis(1_000);
        let now = finished_at + std::time::Duration::from_millis(250);
        let status = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at,
        };
        // Elapsed 250ms of a 1000ms TTL → 750ms remain.
        let remaining = cancel_status_remaining_ttl(&status, now, ttl);
        assert_eq!(remaining, Some(std::time::Duration::from_millis(750)));
    }

    #[test]
    fn cancel_status_remaining_ttl_error_within_returns_positive() {
        let finished_at = Instant::now();
        let ttl = std::time::Duration::from_millis(2_000);
        let now = finished_at + std::time::Duration::from_millis(1_500);
        let status = F2CancelStatus::Error {
            transfer_id: "t-1".to_string(),
            message: "boom".to_string(),
            finished_at,
        };
        let remaining = cancel_status_remaining_ttl(&status, now, ttl);
        assert_eq!(remaining, Some(std::time::Duration::from_millis(500)));
    }

    #[test]
    fn cancel_status_remaining_ttl_past_returns_none() {
        // Already past TTL — the renderer returns Hidden,
        // so the loop has nothing left to wake for.
        let finished_at = Instant::now();
        let ttl = std::time::Duration::from_millis(500);
        let now = finished_at + std::time::Duration::from_millis(501);
        let status = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at,
        };
        assert!(cancel_status_remaining_ttl(&status, now, ttl).is_none());
    }

    #[test]
    fn cancel_status_remaining_ttl_at_boundary_returns_none() {
        // Boundary matches the d-23 `>=` convention used
        // by `cancel_status_to_display` — exact-tick lands
        // on Hidden, so the loop has no remaining wakeup.
        let finished_at = Instant::now();
        let ttl = std::time::Duration::from_millis(500);
        let now = finished_at + ttl;
        let status = F2CancelStatus::Done {
            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
                transfer_id: "t-1".to_string(),
            },
            finished_at,
        };
        assert!(cancel_status_remaining_ttl(&status, now, ttl).is_none());
    }

    /// d-24 R2 REGRESSION: this is the scenario the
    /// reviewer flagged. Operator sets a 250ms cancel TTL
    /// but a 5000ms live-tick. Pre-fix, the loop slept
    /// 5000ms after the cancel reply and the fragment
    /// stayed on screen ~20x longer than configured.
    /// Post-fix, the tick budget collapses to the
    /// shorter of the two.
    #[test]
    fn short_cancel_ttl_overrides_long_live_tick() {
        let budget = compute_tick_budget(
            true,
            std::time::Duration::from_millis(5_000),
            Some(std::time::Duration::from_millis(250)),
        );
        assert_eq!(budget, Some(std::time::Duration::from_millis(250)));
    }

    #[test]
    fn long_cancel_ttl_keeps_live_tick_unchanged() {
        // 60s cancel TTL + 500ms live tick → live tick
        // wins (freshness footer cadence drives the loop).
        let budget = compute_tick_budget(
            true,
            std::time::Duration::from_millis(500),
            Some(std::time::Duration::from_millis(60_000)),
        );
        assert_eq!(budget, Some(std::time::Duration::from_millis(500)));
    }

    #[test]
    fn tick_budget_no_live_tick_no_cancel_returns_none() {
        // Pure-idle: no freshness ticks, no cancel
        // fragment → the loop sleeps indefinitely.
        let budget = compute_tick_budget(false, std::time::Duration::from_millis(500), None);
        assert!(budget.is_none());
    }

    #[test]
    fn tick_budget_cancel_only_wakes_for_deadline() {
        // Edge case: needs_live_tick is false (e.g. the
        // freshness gate didn't fire) but a cancel
        // fragment is still pending. The loop must still
        // wake for the deadline — otherwise a stale
        // fragment would persist until the next real
        // event.
        let budget = compute_tick_budget(
            false,
            std::time::Duration::from_millis(500),
            Some(std::time::Duration::from_millis(120)),
        );
        assert_eq!(budget, Some(std::time::Duration::from_millis(120)));
    }

    #[test]
    fn tick_budget_live_tick_only_returns_interval() {
        let budget = compute_tick_budget(true, std::time::Duration::from_millis(500), None);
        assert_eq!(budget, Some(std::time::Duration::from_millis(500)));
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

    /// d-1 round-2 regression: a failed lifecycle action
    /// (clear / disable / enable) must NOT be hidden by a
    /// follow-up profile re-fetch. `apply_lifecycle_outcome`
    /// returns false on Err so the caller skips the fetch,
    /// and the Error banner survives.
    #[test]
    fn apply_lifecycle_outcome_preserves_error_and_skips_fetch() {
        let mut state = profile::ProfileState::new();
        let should_refetch =
            apply_lifecycle_outcome(&mut state, Err("clear failed: boom".to_string()));
        assert!(!should_refetch, "Err path must signal 'no re-fetch'");
        match state.status() {
            profile::ProfileFetchStatus::Error { message } => {
                assert_eq!(message, "clear failed: boom");
            }
            other => panic!("expected Error banner, got {other:?}"),
        }
    }

    /// Companion: on Ok, returns true (caller refetches)
    /// and the status is left as-is (caller's begin_fetch
    /// drives the next transition).
    #[test]
    fn apply_lifecycle_outcome_ok_signals_refetch_without_banner_change() {
        let mut state = profile::ProfileState::new();
        let before = matches!(state.status(), profile::ProfileFetchStatus::Idle);
        assert!(before);
        let should_refetch = apply_lifecycle_outcome(&mut state, Ok(()));
        assert!(should_refetch, "Ok path must signal 're-fetch'");
        // Status unchanged by the helper — the caller's
        // begin_fetch flips it to Pending.
        assert!(matches!(state.status(), profile::ProfileFetchStatus::Idle));
    }

    /// e-1: `?` toggles the global help overlay.
    #[test]
    fn key_action_maps_question_mark_to_toggle_help() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('?'))),
            Some(UserAction::ToggleHelp)
        ));
    }

    /// d-26: `/` is mapped to F3FilterBegin. F1/F2/F4
    /// dispatch arms ignore the variant.
    #[test]
    fn key_action_maps_slash_to_f3_filter_begin() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('/'))),
            Some(UserAction::F3FilterBegin)
        ));
    }

    /// d-26 helper: build a fresh `AppState` for keystroke
    /// tests. Mirrors the boilerplate of
    /// `handle_verify_keystroke_returns_false_for_question_mark`.
    fn make_test_app_state(screen: Screen) -> AppState {
        AppState {
            current_screen: screen,
            parsed_remote: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            browse: BrowseState::new(),
            browse_last_fetched_view: None,
            browse_fetch_tx: mpsc::channel::<BrowseFetchReply>(1).0,
            profile: profile::ProfileState::new(),
            profile_reply_tx: mpsc::channel::<ProfileReply>(1).0,
            verify: verify::VerifyState::new(),
            diagnostics: diagnostics::DiagnosticsState::new(),
            diagnostics_reply_tx: mpsc::channel::<DiagnosticsReply>(1).0,
            help: help::HelpOverlay::default(),
            transfer: transfer::TransferState::new(),
            transfer_reply_tx: mpsc::channel::<TransferReply>(1).0,
            cancel_status: F2CancelStatus::Idle,
            cancel_reply_tx: mpsc::channel::<CancelReply>(1).0,
            cancel_request_seq: 0,
            f3_pull: f3pull::F3PullState::new(),
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            reload_banner: None,
        }
    }

    /// d-26: chars append to the filter while editing.
    #[test]
    fn handle_f3_filter_keystroke_routes_chars_to_filter() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        let consumed = handle_f3_filter_keystroke(&k(KeyCode::Char('p')), &mut app);
        assert!(consumed);
        assert_eq!(app.browse.filter(), "p");
    }

    /// d-26: Backspace pops one char.
    #[test]
    fn handle_f3_filter_keystroke_routes_backspace_to_pop() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        app.browse.push_filter_char('p');
        app.browse.push_filter_char('h');
        let consumed = handle_f3_filter_keystroke(&k(KeyCode::Backspace), &mut app);
        assert!(consumed);
        assert_eq!(app.browse.filter(), "p");
    }

    /// d-26: Enter commits — filter persists, edit mode exits.
    #[test]
    fn handle_f3_filter_keystroke_routes_enter_to_commit() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        app.browse.push_filter_char('p');
        let consumed = handle_f3_filter_keystroke(&k(KeyCode::Enter), &mut app);
        assert!(consumed);
        assert_eq!(app.browse.filter(), "p");
        assert!(!app.browse.is_editing_filter());
    }

    /// d-26: Esc cancels — filter clears, edit mode exits.
    #[test]
    fn handle_f3_filter_keystroke_routes_esc_to_cancel() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        app.browse.push_filter_char('p');
        let consumed = handle_f3_filter_keystroke(&k(KeyCode::Esc), &mut app);
        assert!(consumed);
        assert_eq!(app.browse.filter(), "");
        assert!(!app.browse.is_editing_filter());
    }

    /// d-26: `?` is still global from filter-edit mode —
    /// returns false so the dispatcher's ToggleHelp runs.
    #[test]
    fn handle_f3_filter_keystroke_returns_false_for_question_mark() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        let consumed = handle_f3_filter_keystroke(&k(KeyCode::Char('?')), &mut app);
        assert!(!consumed);
        // Filter unchanged.
        assert_eq!(app.browse.filter(), "");
    }

    /// d-26: F-keys still navigate panes from filter-edit
    /// mode — returns false so the dispatcher routes to
    /// Navigate(Screen::Fx).
    #[test]
    fn handle_f3_filter_keystroke_returns_false_for_f_keys() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        for n in 1..=4 {
            let consumed = handle_f3_filter_keystroke(&k(KeyCode::F(n)), &mut app);
            assert!(
                !consumed,
                "F{n} must bubble back to the dispatcher for pane nav"
            );
        }
    }

    /// d-26: Ctrl-c is the emergency quit shortcut — always
    /// falls through to `should_quit`.
    #[test]
    fn handle_f3_filter_keystroke_returns_false_for_ctrl_c() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        let key = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        let consumed = handle_f3_filter_keystroke(&key, &mut app);
        assert!(!consumed);
    }

    /// d-26: Ctrl-modified chars don't get inserted as
    /// garbled filter text — returns false so the
    /// dispatcher can route them (or ignore them).
    #[test]
    fn handle_f3_filter_keystroke_returns_false_for_ctrl_chars() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse.begin_edit_filter();
        let key = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
        };
        let consumed = handle_f3_filter_keystroke(&key, &mut app);
        assert!(!consumed);
        assert_eq!(app.browse.filter(), "");
    }

    // d-35: F3 pull keystroke routing.

    /// `p` maps to F3PullBegin (only acted on by the F3
    /// dispatch arm).
    #[test]
    fn key_action_maps_p_to_f3_pull_begin() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('p'))),
            Some(UserAction::F3PullBegin)
        ));
    }

    /// Helper: an F3 app with the pull prompt already
    /// open (source = a parsed endpoint).
    fn app_with_pull_prompt() -> AppState {
        let mut app = make_test_app_state(Screen::F3);
        let source = RemoteEndpoint::parse("nas:/photos/2024").expect("endpoint");
        app.f3_pull.begin(source);
        app
    }

    #[test]
    fn handle_f3_pull_keystroke_routes_chars_to_dest() {
        let mut app = app_with_pull_prompt();
        for c in "/tmp".chars() {
            assert!(handle_f3_pull_keystroke(&k(KeyCode::Char(c)), &mut app));
        }
        match app.f3_pull.status() {
            f3pull::F3PullStatus::EnteringDest { dest, .. } => assert_eq!(dest, "/tmp"),
            other => panic!("expected EnteringDest, got {other:?}"),
        }
    }

    #[test]
    fn handle_f3_pull_keystroke_backspace_pops_dest() {
        let mut app = app_with_pull_prompt();
        for c in "/tmpx".chars() {
            handle_f3_pull_keystroke(&k(KeyCode::Char(c)), &mut app);
        }
        assert!(handle_f3_pull_keystroke(&k(KeyCode::Backspace), &mut app));
        match app.f3_pull.status() {
            f3pull::F3PullStatus::EnteringDest { dest, .. } => assert_eq!(dest, "/tmp"),
            other => panic!("expected EnteringDest, got {other:?}"),
        }
    }

    #[test]
    fn handle_f3_pull_keystroke_esc_cancels() {
        let mut app = app_with_pull_prompt();
        handle_f3_pull_keystroke(&k(KeyCode::Char('x')), &mut app);
        assert!(handle_f3_pull_keystroke(&k(KeyCode::Esc), &mut app));
        assert!(matches!(app.f3_pull.status(), f3pull::F3PullStatus::Idle));
    }

    #[test]
    fn handle_f3_pull_keystroke_enter_on_empty_dest_keeps_prompt() {
        let mut app = app_with_pull_prompt();
        // No dest typed → Enter is absorbed but the prompt
        // stays open (begin_run is a no-op on empty dest).
        assert!(handle_f3_pull_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(app.f3_pull.is_entering_dest());
    }

    #[test]
    fn handle_f3_pull_keystroke_returns_false_for_f_keys() {
        let mut app = app_with_pull_prompt();
        for n in 1..=4 {
            assert!(
                !handle_f3_pull_keystroke(&k(KeyCode::F(n)), &mut app),
                "F{n} must bubble to the dispatcher for pane nav"
            );
        }
    }

    #[test]
    fn handle_f3_pull_keystroke_returns_false_for_question_mark() {
        let mut app = app_with_pull_prompt();
        assert!(!handle_f3_pull_keystroke(&k(KeyCode::Char('?')), &mut app));
    }

    #[test]
    fn handle_f3_pull_keystroke_returns_false_for_ctrl_c() {
        let mut app = app_with_pull_prompt();
        let key = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert!(!handle_f3_pull_keystroke(&key, &mut app));
    }

    // d-37 round 2: pull-progress accumulator semantics.

    /// The reviewer-flagged regression: the TCP data-plane
    /// path emits `Payload { bytes: N }` AND
    /// `FileComplete { bytes: N }` for the SAME file.
    /// Bytes must come from Payload only — the pair must
    /// total N bytes / 1 file, not 2N.
    #[test]
    fn accumulate_pull_progress_data_plane_pair_no_double_count() {
        use blit_core::remote::transfer::ProgressEvent;
        let mut files = 0usize;
        let mut bytes = 0u64;
        accumulate_pull_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::Payload {
                files: 0,
                bytes: 1024,
            },
        );
        accumulate_pull_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::FileComplete {
                path: "f.txt".to_string(),
                bytes: 1024,
            },
        );
        assert_eq!(bytes, 1024, "bytes from Payload only — not doubled");
        assert_eq!(files, 1, "one file from the FileComplete");
    }

    /// Direct-gRPC path: bytes arrive via `Payload` chunks,
    /// `FileComplete` carries `bytes: 0`. Bytes accumulate
    /// from the chunks; the file is counted once.
    #[test]
    fn accumulate_pull_progress_grpc_chunks_then_zero_byte_complete() {
        use blit_core::remote::transfer::ProgressEvent;
        let mut files = 0usize;
        let mut bytes = 0u64;
        for chunk in [4096u64, 4096, 2000] {
            accumulate_pull_progress(
                &mut files,
                &mut bytes,
                &ProgressEvent::Payload {
                    files: 0,
                    bytes: chunk,
                },
            );
        }
        accumulate_pull_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::FileComplete {
                path: "big.bin".to_string(),
                bytes: 0,
            },
        );
        assert_eq!(bytes, 10192);
        assert_eq!(files, 1);
    }

    /// ManifestBatch events don't touch the byte/file
    /// totals (they're a discovery-phase signal).
    #[test]
    fn accumulate_pull_progress_manifest_batch_is_inert() {
        use blit_core::remote::transfer::ProgressEvent;
        let mut files = 0usize;
        let mut bytes = 0u64;
        accumulate_pull_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::ManifestBatch { files: 12 },
        );
        assert_eq!(files, 0);
        assert_eq!(bytes, 0);
    }

    /// Multi-file data-plane transfer: each file emits the
    /// Payload+FileComplete pair; totals stay honest.
    #[test]
    fn accumulate_pull_progress_multi_file_data_plane() {
        use blit_core::remote::transfer::ProgressEvent;
        let mut files = 0usize;
        let mut bytes = 0u64;
        for (i, size) in [100u64, 200, 300].iter().enumerate() {
            accumulate_pull_progress(
                &mut files,
                &mut bytes,
                &ProgressEvent::Payload {
                    files: 0,
                    bytes: *size,
                },
            );
            accumulate_pull_progress(
                &mut files,
                &mut bytes,
                &ProgressEvent::FileComplete {
                    path: format!("f{i}"),
                    bytes: *size,
                },
            );
        }
        assert_eq!(bytes, 600);
        assert_eq!(files, 3);
    }

    #[test]
    fn pull_throughput_suppressed_in_first_second() {
        // d-39: a tiny elapsed window would otherwise
        // report `bytes / 0.01s` = a bogus multi-GiB/s
        // spike. Below the 1s warm-up it's pinned to 0.
        assert_eq!(pull_throughput(1_000_000, 0.0), 0);
        assert_eq!(pull_throughput(1_000_000, 0.5), 0);
        assert_eq!(pull_throughput(1_000_000, 0.999), 0);
    }

    #[test]
    fn pull_throughput_is_cumulative_average_after_warmup() {
        // At exactly the 1s boundary and beyond, it's
        // bytes / elapsed.
        assert_eq!(pull_throughput(1_048_576, 1.0), 1_048_576);
        assert_eq!(pull_throughput(1_048_576, 2.0), 524_288);
        assert_eq!(pull_throughput(10_000_000, 4.0), 2_500_000);
    }

    #[test]
    fn pull_throughput_zero_bytes_is_zero() {
        assert_eq!(pull_throughput(0, 5.0), 0);
    }

    /// e-1 round-2 regression: `?` is GLOBAL, including
    /// from inside the Verify form's edit mode. The
    /// verify handler must return false for `Char('?')`
    /// so the dispatcher's `ToggleHelp` runs instead of
    /// inserting the character into the focused field.
    #[test]
    fn handle_verify_keystroke_returns_false_for_question_mark() {
        // Build a state with Verify focused on Source.
        // Then send `?`. Expect handler to NOT consume it
        // (returns false), and the source field stays empty.
        let mut app = AppState {
            current_screen: Screen::F4,
            parsed_remote: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            daemons_last_fetched: None,
            // Senders aren't called on the false branch
            // but the struct demands them.
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            browse: BrowseState::new(),
            browse_last_fetched_view: None,
            browse_fetch_tx: mpsc::channel::<BrowseFetchReply>(1).0,
            profile: profile::ProfileState::new(),
            profile_reply_tx: mpsc::channel::<ProfileReply>(1).0,
            verify: verify::VerifyState::new(),
            diagnostics: diagnostics::DiagnosticsState::new(),
            diagnostics_reply_tx: mpsc::channel::<DiagnosticsReply>(1).0,
            help: help::HelpOverlay::default(),
            transfer: transfer::TransferState::new(),
            transfer_reply_tx: mpsc::channel::<TransferReply>(1).0,
            cancel_status: F2CancelStatus::Idle,
            cancel_reply_tx: mpsc::channel::<CancelReply>(1).0,
            cancel_request_seq: 0,
            f3_pull: f3pull::F3PullState::new(),
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            reload_banner: None,
        };
        app.verify.cycle_focus(); // Source
        let (verify_run_tx, _verify_run_rx) = mpsc::channel::<VerifyReply>(1);

        let consumed = handle_verify_keystroke(&k(KeyCode::Char('?')), &mut app, &verify_run_tx);
        assert!(
            !consumed,
            "`?` must bubble back to the global dispatcher, not be consumed as text"
        );
        assert!(
            app.verify.source.is_empty(),
            "`?` must NOT insert into the focused field, got: {:?}",
            app.verify.source
        );
    }

    /// d-12 R2: predicate that gates the router's
    /// Esc-cancels-confirm intercept. Pins the matrix the
    /// reviewer asked for, in particular the "confirm
    /// pending + Verify focus editing" combination — the
    /// gate returns true regardless of focus state so the
    /// confirm-cancel branch wins over `handle_verify_keystroke`.
    #[test]
    fn esc_cancels_confirm_priority_matrix() {
        let mut app = AppState {
            current_screen: Screen::F4,
            parsed_remote: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            browse: BrowseState::new(),
            browse_last_fetched_view: None,
            browse_fetch_tx: mpsc::channel::<BrowseFetchReply>(1).0,
            profile: profile::ProfileState::new(),
            profile_reply_tx: mpsc::channel::<ProfileReply>(1).0,
            verify: verify::VerifyState::new(),
            diagnostics: diagnostics::DiagnosticsState::new(),
            diagnostics_reply_tx: mpsc::channel::<DiagnosticsReply>(1).0,
            help: help::HelpOverlay::default(),
            transfer: transfer::TransferState::new(),
            transfer_reply_tx: mpsc::channel::<TransferReply>(1).0,
            cancel_status: F2CancelStatus::Idle,
            cancel_reply_tx: mpsc::channel::<CancelReply>(1).0,
            cancel_request_seq: 0,
            f3_pull: f3pull::F3PullState::new(),
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            reload_banner: None,
        };

        let esc = k(KeyCode::Esc);
        // No confirm pending → false even on Esc.
        assert!(!esc_cancels_confirm(&esc, &app));

        // Confirm pending → true.
        app.transfer.begin_confirm_mirror();
        assert!(esc_cancels_confirm(&esc, &app));

        // *** THE REGRESSION ***: even with Verify form
        // focused into edit mode, Esc must still cancel
        // the confirm (the intercept runs BEFORE
        // handle_verify_keystroke). Pre-fix this returned
        // true correctly here, but the router consulted
        // handle_verify_keystroke first and ate the Esc.
        app.verify.cycle_focus();
        assert!(app.verify.focus().is_editing());
        assert!(
            esc_cancels_confirm(&esc, &app),
            "Esc must cancel confirm even when Verify form has edit focus"
        );

        // Ctrl-Esc / Alt-Esc still fall through to the
        // regular dispatcher.
        let ctrl_esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::CONTROL,
        };
        assert!(!esc_cancels_confirm(&ctrl_esc, &app));
        let alt_esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::ALT,
        };
        assert!(!esc_cancels_confirm(&alt_esc, &app));

        // Move confirm also gets cancelled (the gate is
        // confirm-kind-agnostic via is_confirming()).
        app.transfer.cancel_confirm();
        app.transfer.begin_confirm_move();
        assert!(esc_cancels_confirm(&esc, &app));

        // Non-Esc keys with confirm pending don't trigger.
        let y = k(KeyCode::Char('y'));
        assert!(!esc_cancels_confirm(&y, &app));
    }

    /// d-12: cancel_confirm dismisses both ConfirmingMirror
    /// and ConfirmingMove back to Idle. The router's Esc
    /// intercept calls cancel_confirm directly, so this
    /// test pins the state-transition contract that the
    /// intercept relies on.
    #[test]
    fn cancel_confirm_dismisses_either_confirm_kind() {
        let mut state = transfer::TransferState::new();

        // Mirror confirm → cancel → Idle.
        state.begin_confirm_mirror();
        assert!(state.is_confirming_mirror());
        assert!(state.cancel_confirm());
        assert!(matches!(state.status(), transfer::TransferStatus::Idle));

        // Move confirm → cancel → Idle.
        state.begin_confirm_move();
        assert!(state.is_confirming_move());
        assert!(state.cancel_confirm());
        assert!(matches!(state.status(), transfer::TransferStatus::Idle));
    }

    /// d-9: `needs_live_tick` is true ONLY while a Verify
    /// run or a local transfer is in flight — that's when
    /// the F4 elapsed counter needs a 500ms wakeup to
    /// re-render. Idle, confirm-pending, Done, and Error
    /// states all return false so the loop sleeps on real
    /// events.
    #[test]
    fn needs_live_tick_only_during_active_runs() {
        let mut app = AppState {
            current_screen: Screen::F4,
            parsed_remote: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            browse: BrowseState::new(),
            browse_last_fetched_view: None,
            browse_fetch_tx: mpsc::channel::<BrowseFetchReply>(1).0,
            profile: profile::ProfileState::new(),
            profile_reply_tx: mpsc::channel::<ProfileReply>(1).0,
            verify: verify::VerifyState::new(),
            diagnostics: diagnostics::DiagnosticsState::new(),
            diagnostics_reply_tx: mpsc::channel::<DiagnosticsReply>(1).0,
            help: help::HelpOverlay::default(),
            transfer: transfer::TransferState::new(),
            transfer_reply_tx: mpsc::channel::<TransferReply>(1).0,
            cancel_status: F2CancelStatus::Idle,
            cancel_reply_tx: mpsc::channel::<CancelReply>(1).0,
            cancel_request_seq: 0,
            f3_pull: f3pull::F3PullState::new(),
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            reload_banner: None,
        };

        // All-idle → no tick.
        assert!(!needs_live_tick(&app));

        // Mirror confirmation pending → no tick (the
        // banner is static, nothing to refresh).
        app.transfer.begin_confirm_mirror();
        assert!(!needs_live_tick(&app));
        app.transfer.cancel_confirm();

        // Transfer Running → tick.
        let _id = app.transfer.begin(transfer::TransferKind::Copy);
        assert!(needs_live_tick(&app));

        // Drop back to Idle, then start a Verify run.
        let id = _id;
        app.transfer
            .apply_done(id, transfer::TransferKind::Copy, Default::default());
        assert!(!needs_live_tick(&app));

        app.verify.source = "/tmp/a".to_string();
        app.verify.destination = "/tmp/b".to_string();
        let _ = app.verify.begin_run();
        assert!(needs_live_tick(&app));
    }

    /// d-11: extend the live-tick gate to per-pane
    /// freshness footers. F1's "live · last scan Xs ago"
    /// (when DiscoveryStatus is Live), F3's "loaded · Xs
    /// ago" (when BrowseFetchStatus is Loaded), and F4's
    /// "loaded · Xs ago" (when ProfileFetchStatus is
    /// Loaded) all tick — F2 doesn't use `now` so it
    /// stays gated off.
    #[test]
    fn needs_live_tick_covers_per_pane_freshness_footers() {
        let mut app = AppState {
            current_screen: Screen::F1,
            parsed_remote: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            browse: BrowseState::new(),
            browse_last_fetched_view: None,
            browse_fetch_tx: mpsc::channel::<BrowseFetchReply>(1).0,
            profile: profile::ProfileState::new(),
            profile_reply_tx: mpsc::channel::<ProfileReply>(1).0,
            verify: verify::VerifyState::new(),
            diagnostics: diagnostics::DiagnosticsState::new(),
            diagnostics_reply_tx: mpsc::channel::<DiagnosticsReply>(1).0,
            help: help::HelpOverlay::default(),
            transfer: transfer::TransferState::new(),
            transfer_reply_tx: mpsc::channel::<TransferReply>(1).0,
            cancel_status: F2CancelStatus::Idle,
            cancel_reply_tx: mpsc::channel::<CancelReply>(1).0,
            cancel_request_seq: 0,
            f3_pull: f3pull::F3PullState::new(),
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            reload_banner: None,
        };

        // F1, pre-discovery (Scanning) → no tick.
        assert!(!needs_live_tick(&app), "F1 Scanning has no time component");

        // F1 with Live status → tick.
        app.daemons
            .replace_from_discovery(&[], std::time::Instant::now());
        assert!(needs_live_tick(&app), "F1 Live ticks the last-scan footer");

        // Switch to F2 — no `now` use, no tick even with
        // a live remote.
        app.current_screen = Screen::F2;
        // d-13: F2 doesn't tick until it has seen at
        // least one event. `last_event_at` is None on a
        // fresh TransfersState.
        assert!(
            !needs_live_tick(&app),
            "F2 doesn't tick until last_event_at is Some"
        );
        // After a GetState snapshot lands, F2's footer
        // shows "last event Xs ago" → tick.
        app.transfers.replace_from_snapshot(
            blit_core::generated::DaemonState::default(),
            std::time::Instant::now(),
        );
        assert!(
            needs_live_tick(&app),
            "F2 with last_event_at Some ticks the footer"
        );

        // F3 with browse status Idle → no tick.
        app.current_screen = Screen::F3;
        assert!(!needs_live_tick(&app));
        // F3 after a successful fetch (Loaded) → tick.
        app.browse.apply_modules(vec![], std::time::Instant::now());
        assert!(
            needs_live_tick(&app),
            "F3 Loaded ticks the loaded-since footer"
        );

        // F4 with profile status Idle → no tick (no
        // running transfer either).
        app.current_screen = Screen::F4;
        assert!(!needs_live_tick(&app));
        // F4 after a successful profile fetch → tick.
        let id = app.profile.begin_fetch();
        app.profile.apply_report(
            blit_app::profile::ProfileReport {
                enabled: true,
                records: vec![],
                predictor_path: None,
                predictor: None,
            },
            std::time::Instant::now(),
        );
        let _ = id;
        assert!(
            needs_live_tick(&app),
            "F4 Loaded ticks the profile-as-of footer"
        );
    }

    /// d-4: capital C / M trigger local copy / mirror.
    /// Lowercase stays mapped to the existing actions
    /// (`c` → ProfileClear, `m` would be unmapped today).
    #[test]
    fn key_action_maps_transfer_triggers() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('C'))),
            Some(UserAction::TransferCopy)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('M'))),
            Some(UserAction::TransferMirror)
        ));
        // d-5: capital V triggers move. Lowercase v stays
        // unmapped (reserved for a future visual / multi-
        // select polish on F3).
        assert!(matches!(
            key_action(&k(KeyCode::Char('V'))),
            Some(UserAction::TransferMove)
        ));
        assert!(key_action(&k(KeyCode::Char('v'))).is_none());
    }

    /// d-6: `H` maps to the Verify-mode toggle. Lowercase
    /// `h` stays bound to Ascend (F3 navigation), so only
    /// uppercase claims the toggle.
    #[test]
    fn key_action_maps_verify_checksum_toggle() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('H'))),
            Some(UserAction::ToggleVerifyChecksum)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('h'))),
            Some(UserAction::Ascend)
        ));
    }

    /// d-7: `O` maps to the Verify-direction toggle.
    /// Lowercase `o` stays unmapped (reserved for future
    /// polish).
    #[test]
    fn key_action_maps_verify_one_way_toggle() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('O'))),
            Some(UserAction::ToggleVerifyOneWay)
        ));
        assert!(key_action(&k(KeyCode::Char('o'))).is_none());
    }

    /// d-5: V triggers the move confirm flow — a copy
    /// followed by source delete. State must transition
    /// `Idle → ConfirmingMove`, and `is_busy()` must
    /// gate further triggers.
    #[test]
    fn transfer_state_move_confirm_lifecycle() {
        let mut state = transfer::TransferState::new();
        state.begin_confirm_move();
        assert!(state.is_confirming_move());
        assert!(state.is_confirming());
        assert!(state.is_busy());
        // `is_confirming_mirror` MUST stay false — the
        // dispatcher routes `y` to mirror-confirm only
        // when that specific state is set.
        assert!(!state.is_confirming_mirror());
        assert!(state.cancel_confirm());
        assert!(!state.is_busy());
    }

    /// d-5: end-to-end `perform_local_move`. Writes a
    /// source file, runs the move, asserts the destination
    /// has the file and the source is gone.
    #[tokio::test]
    async fn perform_local_move_deletes_source_after_copy() {
        let tmp = tempfile::tempdir().expect("tmp");
        let src = tmp.path().join("src.txt");
        std::fs::write(&src, b"hello").expect("write src");
        let dst = tmp.path().join("dst.txt");

        let summary = perform_local_move(&src, &dst).await.expect("move succeeds");
        assert!(summary.copied_files >= 1, "summary records the copy");
        assert!(!src.exists(), "source must be removed after move");
        assert_eq!(
            std::fs::read(&dst).expect("dst readable"),
            b"hello",
            "destination must have the source's bytes"
        );
    }

    /// d-4 R2: `y` / `n` confirm or cancel the mirror
    /// destructive-op prompt. Both cases map (rsync-style
    /// `[y/N]`). The F4 dispatcher only acts on these while
    /// `transfer.is_confirming_mirror()` is true.
    #[test]
    fn key_action_maps_transfer_confirm_keys() {
        for code in [KeyCode::Char('y'), KeyCode::Char('Y')] {
            assert!(
                matches!(
                    key_action(&k(code)),
                    Some(UserAction::TransferMirrorConfirm)
                ),
                "expected TransferMirrorConfirm for {code:?}",
            );
        }
        for code in [KeyCode::Char('n'), KeyCode::Char('N')] {
            assert!(
                matches!(key_action(&k(code)), Some(UserAction::TransferCancel)),
                "expected TransferCancel for {code:?}",
            );
        }
    }

    /// d-4 R2 destination resolution: a file source copied
    /// into an existing destination directory must resolve
    /// to `<dest>/<source-basename>`, matching rsync /
    /// `blit copy` behavior. The CLI does this through
    /// `resolve_destination` (`crates/blit-cli/src/transfers/mod.rs:105`);
    /// the TUI must do the same before calling
    /// `blit_app::transfers::local::run`.
    #[test]
    fn prepare_local_transfer_appends_basename_for_container_dest() {
        let tmp = tempfile::tempdir().expect("tmp");
        let src_file = tmp.path().join("file.txt");
        std::fs::write(&src_file, "hello").expect("write src");
        let dst_dir = tmp.path().join("out");
        std::fs::create_dir(&dst_dir).expect("mkdir dst");

        let (src, dst) = prepare_local_transfer(
            src_file.to_str().unwrap(),
            // Trailing slash signals "destination is a
            // container" per the rsync trailing-slash rule.
            &format!("{}/", dst_dir.display()),
        )
        .expect("prepare ok");

        assert_eq!(src, src_file);
        assert_eq!(
            dst,
            dst_dir.join("file.txt"),
            "destination must nest under the container"
        );
    }

    /// d-4 R2: rsync's "copy contents" rule. A trailing
    /// slash on the SOURCE means "copy the contents of",
    /// so the destination stays as-is (no basename append).
    #[test]
    fn prepare_local_transfer_source_contents_keeps_dest() {
        let tmp = tempfile::tempdir().expect("tmp");
        let src_dir = tmp.path().join("src");
        std::fs::create_dir(&src_dir).expect("mkdir src");
        let dst_dir = tmp.path().join("dst");
        std::fs::create_dir(&dst_dir).expect("mkdir dst");

        let src_input = format!("{}/", src_dir.display());
        let dst_input = format!("{}/", dst_dir.display());
        let (src, dst) = prepare_local_transfer(&src_input, &dst_input).expect("prepare ok");

        assert_eq!(src, src_dir);
        assert_eq!(
            dst, dst_dir,
            "source-contents → dest stays as the container"
        );
    }

    /// d-4 R2: remote endpoints are rejected — the F4
    /// Verify form is local-only. The CLI dispatches
    /// remote routes through daemon RPCs; the TUI would
    /// need additional plumbing.
    #[test]
    fn prepare_local_transfer_rejects_remote_source() {
        let err = prepare_local_transfer("host:/module/path", "/tmp/dst").expect_err("rejected");
        assert!(
            err.contains("local"),
            "error must mention local-only restriction, got: {err}",
        );
    }

    /// d-4 R2: confirm-pending guards. M alone doesn't
    /// fire a mirror; it transitions to ConfirmingMirror
    /// and the dispatcher reads `is_confirming_mirror()`
    /// before honoring `y`/`n`.
    #[test]
    fn transfer_state_mirror_confirm_lifecycle() {
        let mut state = transfer::TransferState::new();
        assert!(!state.is_confirming_mirror());
        state.begin_confirm_mirror();
        assert!(state.is_confirming_mirror());
        assert!(state.is_busy(), "busy gates can_start_transfer");
        // Cancel resets without firing the transfer.
        assert!(state.cancel_confirm());
        assert!(!state.is_busy());
        assert!(matches!(state.status(), transfer::TransferStatus::Idle));
    }

    /// d-1 (F4 profile lifecycle keys): `c` / `d` / `e`
    /// land on the right UserAction variants. Uppercase
    /// variants stay unmapped — these are case-sensitive
    /// per the design.
    #[test]
    fn key_action_maps_profile_lifecycle_keys() {
        assert!(matches!(
            key_action(&k(KeyCode::Char('c'))),
            Some(UserAction::ProfileClear)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('d'))),
            Some(UserAction::ProfileDisable)
        ));
        assert!(matches!(
            key_action(&k(KeyCode::Char('e'))),
            Some(UserAction::ProfileEnable)
        ));
        // Uppercase D / E remain unmapped (Profile keys
        // are lowercase-only). Uppercase C is now mapped
        // to TransferCopy as of d-4, so it's covered in
        // `key_action_maps_transfer_triggers`.
        assert!(key_action(&k(KeyCode::Char('D'))).is_none());
        assert!(key_action(&k(KeyCode::Char('E'))).is_none());
        // Ctrl-c remains Quit (not ProfileClear).
        assert!(matches!(
            key_action(&KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }),
            Some(UserAction::Quit)
        ));
    }

    /// a1-6b round 3: F2 refresh keystroke must not
    /// spawn a duplicate setup task. The contract is "only
    /// spawn when there's no live stream AND no setup
    /// already in flight."
    #[test]
    fn should_spawn_f2_setup_only_when_no_stream_and_no_pending() {
        // Initial state — no stream, no pending — spawn.
        assert!(should_spawn_f2_setup(false, false));
        // Setup already in flight — don't spawn duplicate.
        assert!(!should_spawn_f2_setup(false, true));
        // Live stream — refresh path goes through
        // refresh_via_get_state, not setup spawn.
        assert!(!should_spawn_f2_setup(true, false));
        // Both flags set: still don't spawn (defensive —
        // shouldn't happen in practice, but a stale pending
        // flag shouldn't override a live stream).
        assert!(!should_spawn_f2_setup(true, true));
    }

    /// a1-4 round-2 regression: refresh while F3 has no
    /// usable endpoint (missing or malformed `--remote`)
    /// MUST be a no-op — the actionable error banner must
    /// survive. Round-1 unconditionally wiped the banner
    /// with "refreshing" and stranded the operator.
    #[test]
    fn handle_f3_refresh_without_endpoint_preserves_error() {
        let mut state = BrowseState::new();
        state.note_fetch_error("--remote <host> is required for F3 Browse".to_string());
        let mut last_fetched: Option<browse::BrowseView> = None;

        handle_f3_refresh(&mut state, false, &mut last_fetched);

        match state.status() {
            browse::BrowseFetchStatus::Error { message } => {
                assert!(message.contains("--remote"));
                assert!(!message.contains("refreshing"));
            }
            other => panic!("expected preserved Error banner, got {other:?}"),
        }
        // last_fetched_view unchanged.
        assert!(last_fetched.is_none());
    }

    /// Companion: with an endpoint, refresh does the
    /// expected dance (bumps generation, resets
    /// last_fetched_view, flips to Error("refreshing") so
    /// the kick path re-fires next iteration).
    #[test]
    fn handle_f3_refresh_with_endpoint_arms_next_kick() {
        let mut state = BrowseState::new();
        // Simulate the "after a successful list_modules"
        // state.
        state.apply_modules(Vec::new(), Instant::now());
        let mut last_fetched: Option<browse::BrowseView> = Some(browse::BrowseView::Modules);

        handle_f3_refresh(&mut state, true, &mut last_fetched);

        // Generation bumped (status is Error("refreshing")
        // now, but the request_id under the hood was bumped
        // by begin_fetch).
        match state.status() {
            browse::BrowseFetchStatus::Error { message } => {
                assert_eq!(message, "refreshing");
            }
            other => panic!("expected Error(refreshing), got {other:?}"),
        }
        assert!(last_fetched.is_none());
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
