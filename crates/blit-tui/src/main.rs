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
mod f1push;
mod f1trigger;
mod f3del;
mod f3du;
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
    /// Shared remote endpoint (parsed once at startup). This is
    /// the F2 transfers target — its Subscribe stream stays bound
    /// to the launch remote for the session.
    parsed_remote: Option<RemoteEndpoint>,
    /// d-47: the F3 BROWSE target. Initialized to the launch
    /// remote, but `enter` on an F1 daemon row retargets it so
    /// the operator can browse any discovered daemon. F3
    /// browse/pull/du/delete all key off this, not
    /// `parsed_remote`. (F2 deliberately stays on the launch
    /// remote — see the d-47 finding's known gaps.)
    browse_target: Option<RemoteEndpoint>,
    /// Display label for the remote (raw user input or
    /// "(no remote)").
    remote_label: String,

    // F1
    daemons: DaemonsState,
    daemons_last_fetched: Option<String>,
    /// d-58: `t` trigger-transfer modal (source/dest entry). On
    /// commit it hands off to the F3 pull machine + jumps to F3.
    f1_trigger: f1trigger::F1TriggerState,
    /// d-61: local→remote push lifecycle (the trigger's push
    /// direction). Status shown in the F1 footer.
    f1_push: f1push::F1PushState,
    f1_push_reply_tx: mpsc::Sender<F1PushReply>,
    /// d-63: live push progress snapshots (try_send, lossy).
    f1_push_progress_tx: mpsc::Sender<F1PushProgress>,
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
    /// m2f-9 R2: set when an mDNS discovery update changes the watched
    /// set *while a setup is in flight* (`transfers_setup_pending`).
    /// `refan_f2_setup` no-ops while pending, so the change would
    /// otherwise be lost — the in-flight setup completes on the stale
    /// set and later steady updates compare equal. The setup-reply arm
    /// consults this flag and re-fans once the pending setup lands.
    transfers_refan_after_setup: bool,
    /// m2f-10: identities (`host_port_display`) of watched daemons whose
    /// Subscribe stream has errored. With the m2f-5 fan-out, one daemon's
    /// stream ending must NOT blank the whole F2 pane to "degraded" — the
    /// other daemons are still live. The connection banner is derived
    /// from this set vs. the watched total (see `f2_status_from_health`):
    /// empty → Live, some → partial, all → fully degraded. A daemon is
    /// removed when it sends a healthy signal (recovered) or on re-fan.
    f2_degraded_daemons: std::collections::BTreeSet<String>,

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
    /// d-53: sequential batch-pull (`P`) over the F3 marked
    /// set. Drives the single-source `f3_pull` machine one
    /// source at a time; `None` when no batch is running.
    f3_batch_pull: Option<BatchPull>,
    /// d-37: live pull-progress snapshots from the running
    /// pull task.
    f3_pull_progress_tx: mpsc::Sender<F3PullProgress>,
    /// d-41: F3 disk-usage (`u`) lifecycle — the subtree
    /// total for the cursor row, shown in the Stats block.
    f3_du: f3du::F3DuState,
    f3_du_reply_tx: mpsc::Sender<F3DuReply>,
    /// d-45: F3 delete (`D`) lifecycle — confirm prompt +
    /// remote Purge for the cursor row.
    f3_del: f3del::F3DelState,
    f3_del_reply_tx: mpsc::Sender<F3DelReply>,
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
        /// m2f-7: the daemon owning `transfer_id`, captured at `K`
        /// press (the cursor may move before `y`). CancelJob targets
        /// this daemon, not `parsed_remote`.
        daemon: String,
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
        /// m2f-8: `(daemon, transfer_id)` per active row, frozen at
        /// prompt creation — each CancelJob targets its own daemon.
        targets: Vec<(String, String)>,
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
    // dark-1: a non-empty but unrecognized base bg/fg color name falls
    // back to the terminal default. (Empty is the valid "unset" case.)
    if tui_config.theme.background_is_invalid() {
        config_warnings.push(format!(
            "tui.toml [theme] background = {:?} is not a recognized color; \
             using the terminal default",
            tui_config.theme.background,
        ));
    }
    if tui_config.theme.foreground_is_invalid() {
        config_warnings.push(format!(
            "tui.toml [theme] foreground = {:?} is not a recognized color; \
             using the terminal default",
            tui_config.theme.foreground,
        ));
    }
    // dark-2: a non-empty mode that isn't a known preset is ignored.
    if tui_config.theme.mode_is_invalid() {
        config_warnings.push(format!(
            "tui.toml [theme] mode = {:?} is not a recognized preset \
             (use \"dark\" or \"light\"); ignoring it",
            tui_config.theme.mode,
        ));
    }

    // keys-1: a [keys] quit value that isn't a single character falls
    // back to the default. Same buffer-then-flush contract.
    if tui_config.keys.quit_char().is_none() {
        config_warnings.push(format!(
            "tui.toml [keys] quit = {:?} is not a single character; \
             using default {:?} (Esc / Ctrl+C always quit)",
            tui_config.keys.quit,
            config::KeysDefaults::DEFAULT_QUIT,
        ));
    }
    // keys-2: same for the refresh key.
    if tui_config.keys.refresh_char().is_none() {
        config_warnings.push(format!(
            "tui.toml [keys] refresh = {:?} is not a single character; \
             using default {:?}",
            tui_config.keys.refresh,
            config::KeysDefaults::DEFAULT_REFRESH,
        ));
    }
    // keys-3: invalid (non-single-char) pane-switch aliases fall back to
    // their default digit.
    let pane_raw = [
        &tui_config.keys.pane_f1,
        &tui_config.keys.pane_f2,
        &tui_config.keys.pane_f3,
        &tui_config.keys.pane_f4,
    ];
    let pane_chars = tui_config.keys.pane_chars();
    for (i, raw) in pane_raw.iter().enumerate() {
        if pane_chars[i].is_none() {
            config_warnings.push(format!(
                "tui.toml [keys] pane_f{} = {:?} is not a single character; \
                 using default {:?}",
                i + 1,
                raw,
                config::KeysDefaults::DEFAULT_PANE[i],
            ));
        }
    }
    // keys-2 R2 / keys-3: collision policy. A binding that resolves to a
    // character already claimed by a higher-precedence binding (dispatch
    // order: quit > pane aliases > refresh) is disabled — flag each so
    // the operator can pick distinct keys rather than silently lose one.
    let resolved = tui_config.keys.resolved();
    for (i, nav) in resolved.nav.iter().enumerate() {
        if nav.is_none() {
            config_warnings.push(format!(
                "tui.toml [keys] pane_f{} = {:?} collides with a \
                 higher-precedence key (quit / an earlier pane alias) and \
                 is disabled — pick a distinct key (F{} still navigates)",
                i + 1,
                pane_raw[i],
                i + 1,
            ));
        }
    }
    if resolved.refresh.is_none() {
        config_warnings.push(format!(
            "tui.toml [keys] refresh = {:?} collides with a \
             higher-precedence key (quit or a pane alias) and is disabled \
             — pick a distinct [keys] refresh",
            tui_config.keys.refresh,
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
/// e-8: resolve the launch remote. An explicit `--remote` flag always
/// wins (returned verbatim, including a degenerate empty string, so the
/// existing parse-error path is preserved). Absent a flag, fall back to
/// `[daemon] default_remote` when it's non-blank; a blank/whitespace
/// config value is treated as unset so the TUI launches mDNS-only.
fn resolve_launch_remote(cli_remote: Option<&str>, config_default: &str) -> Option<String> {
    if let Some(raw) = cli_remote {
        return Some(raw.to_string());
    }
    let trimmed = config_default.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

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
    // e-8: an explicit `--remote` wins; otherwise fall back to
    // `[daemon] default_remote` from tui.toml. A config-sourced remote
    // flows through the same parse path, so a bad value surfaces the
    // same F2/F3 banner as a bad CLI flag.
    let launch_remote =
        resolve_launch_remote(args.remote.as_deref(), &tui_config.daemon.default_remote);
    let (parsed_remote, parse_error_message): (Option<RemoteEndpoint>, Option<String>) =
        match launch_remote.as_deref() {
            Some(raw) => match RemoteEndpoint::parse(raw) {
                Ok(ep) => (Some(ep), None),
                Err(err) => (None, Some(format!("parse '{raw}': {err}"))),
            },
            None => (None, None),
        };
    let remote_label = launch_remote
        .as_deref()
        .unwrap_or("(no remote)")
        .to_string();

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
    // d-61: F1 push reply channel (terminal outcome only).
    let (f1_push_reply_tx, mut f1_push_reply_rx) = mpsc::channel::<F1PushReply>(2);
    // d-63: F1 push live-progress channel (small, lossy).
    let (f1_push_progress_tx, mut f1_push_progress_rx) = mpsc::channel::<F1PushProgress>(8);

    // d-37: F3 pull live-progress channel. Small bounded
    // buffer — the forwarder `try_send`s, dropping
    // intermediate snapshots when full.
    let (f3_pull_progress_tx, mut f3_pull_progress_rx) = mpsc::channel::<F3PullProgress>(8);

    // d-41: F3 du reply channel.
    let (f3_du_reply_tx, mut f3_du_reply_rx) = mpsc::channel::<F3DuReply>(2);

    // d-45: F3 delete reply channel.
    let (f3_del_reply_tx, mut f3_del_reply_rx) = mpsc::channel::<F3DelReply>(2);

    let mut app = AppState {
        current_screen: args.screen.into(),
        parsed_remote: parsed_remote.clone(),
        browse_target: parsed_remote.clone(),
        remote_label,
        daemons: DaemonsState::new(),
        f1_trigger: f1trigger::F1TriggerState::new(),
        f1_push: f1push::F1PushState::new(),
        f1_push_reply_tx,
        f1_push_progress_tx,
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
        transfers_refan_after_setup: false,
        f2_degraded_daemons: std::collections::BTreeSet::new(),
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
        f3_batch_pull: None,
        f3_pull_reply_tx: f3_pull_reply_tx.clone(),
        f3_pull_progress_tx: f3_pull_progress_tx.clone(),
        f3_du: f3du::F3DuState::new(),
        f3_du_reply_tx: f3_du_reply_tx.clone(),
        f3_del: f3del::F3DelState::new(),
        f3_del_reply_tx: f3_del_reply_tx.clone(),
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
    // m2f-5: F2 fans out to every watched daemon (launch parsed_remote
    // + discovered). At startup discovery is still empty, so this is
    // just parsed_remote when set.
    let watched = f2_watched_endpoints(&app);
    if !watched.is_empty() {
        app.transfers_setup_gen += 1;
        app.transfers_setup_pending = true;
        spawn_f2_setup_task(watched, app.transfers_setup_gen, f2_setup_tx.clone());
    }

    // F4 initial profile fetch — kicked once so the operator
    // sees data the first time they hit F4.
    let initial_profile_id = app.profile.begin_fetch();
    spawn_profile_fetch(initial_profile_id, profile_reply_tx.clone());

    // Optional Subscribe receiver. Populated once F2 setup
    // completes (either at startup or after `r` re-opens
    // the stream in the future).
    let mut transfers_event_rx: Option<mpsc::Receiver<F2Event>> = None;

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
        // d-47: keyed off `browse_target` (the F3 daemon), not
        // `parsed_remote` (the F2 daemon).
        if matches!(app.current_screen, Screen::F3)
            && app.browse_target.is_some()
            && views_differ(app.browse_last_fetched_view.as_ref(), app.browse.view())
            && matches!(
                app.browse.status(),
                browse::BrowseFetchStatus::Idle | browse::BrowseFetchStatus::Error { .. }
            )
        {
            if let Some(ep) = app.browse_target.as_ref() {
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
        // d-50 R2: auto-hide a batch delete outcome (single-row
        // deletes self-hide on cursor move; batch has no such
        // event, so it expires on a TTL like the d-38 pull TTL).
        // d-52: TTL is operator-tunable via
        // `[transfer] delete_status_ttl_ms`, read each frame.
        let delete_ttl = Duration::from_millis(tui_config.transfer.delete_status_ttl_ms_clamped());
        app.f3_del.clear_terminal_if_expired(now, delete_ttl);
        // d-64: auto-hide the F1 push outcome (mirrors the pull TTL).
        // Tunable via `[transfer] push_status_ttl_ms`, read each frame.
        let push_ttl = Duration::from_millis(tui_config.transfer.push_status_ttl_ms_clamped());
        app.f1_push.clear_terminal_if_expired(now, push_ttl);
        // d-36: accent + reload banner are recomputed each
        // frame from the (possibly hot-reloaded) config, so
        // a `Ctrl+R` theme change takes effect immediately.
        let accent_color = tui_config
            .theme
            .parse_accent()
            .map(raw_color_to_ratatui)
            .unwrap_or(ratatui::style::Color::Cyan);
        // dark-1/dark-2: optional base bg/fg painted under the whole TUI.
        // resolved_base_colors applies the `mode` preset with explicit
        // background/foreground overriding it. Both `None` (the default)
        // means "leave the terminal's own colors" — no base layer.
        // Recomputed each frame so a `Ctrl+R` theme reload re-colors live.
        let (base_bg, base_fg) = tui_config.theme.resolved_base_colors();
        let base_style = base_theme_style(
            base_bg.map(raw_color_to_ratatui),
            base_fg.map(raw_color_to_ratatui),
        );
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
                // dark-1: paint the base bg/fg over the whole frame FIRST,
                // so every fg-only widget drawn on top inherits it (a
                // `Style` with `bg: None` leaves the painted bg intact).
                // `None` → no base layer → terminal default, as before.
                if let Some(style) = base_style {
                    frame.render_widget(
                        ratatui::widgets::Block::default().style(style),
                        frame.area(),
                    );
                }
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
                // d-47: cursor specs derive from the F3 browse
                // target, not the F2 remote.
                let f3_pull_spec: Option<String> = app.browse_target.as_ref().and_then(|base| {
                    browse::pull_source_endpoint(app.browse.view(), app.browse.selected_row(), base)
                        .map(|e| e.display())
                });
                // d-47: F3 header shows the browsed daemon (which
                // `enter` on F1 can change), falling back to the
                // launch label when no browse target is set.
                let f3_label: String = app
                    .browse_target
                    .as_ref()
                    .map(|e| e.host_port_display())
                    .unwrap_or_else(|| app.remote_label.clone());
                match app.current_screen {
                    Screen::F1 => screens::f1::render_into(
                        frame,
                        body_area,
                        &app.daemons,
                        now,
                        f1_trigger_prompt(&app.f1_trigger),
                        f1_push_status(&app.f1_push),
                        accent_color,
                    ),
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
                        accent_color,
                    ),
                    Screen::F3 => screens::f3::render_into(
                        frame,
                        body_area,
                        &app.browse,
                        &f3_label,
                        f3_pull_spec.as_deref(),
                        &f3_pull_to_display(app.f3_pull.status()),
                        &f3_du_to_display(app.f3_du.status(), f3_pull_spec.as_deref()),
                        &f3_del_to_display(app.f3_del.status(), f3_pull_spec.as_deref()),
                        // d-53: batch-pull progress (current/total).
                        app.f3_batch_pull.as_ref().map(|b| (b.done + 1, b.total)),
                        now,
                        accent_color,
                    ),
                    Screen::F4 => screens::f4::render_into(
                        frame,
                        body_area,
                        &app.profile,
                        &app.verify,
                        &app.diagnostics,
                        &app.transfer,
                        now,
                        accent_color,
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
        // d-40 R2: the F3 pull outcome fragment has its own
        // auto-hide deadline. A short `pull_status_ttl_ms`
        // must collapse the sleep budget exactly like the
        // d-24 cancel TTL does, or a long `live_tick`
        // interval would delay it (reviewer reopen).
        let pull_remaining = if matches!(app.current_screen, Screen::F3) {
            app.f3_pull.terminal_remaining(Instant::now(), pull_ttl)
        } else {
            None
        };
        // d-64: the F1 push outcome fragment collapses the sleep
        // budget the same way, so a short `push_status_ttl_ms`
        // isn't delayed by a long live-tick interval.
        let push_remaining = if matches!(app.current_screen, Screen::F1) {
            app.f1_push.terminal_remaining(Instant::now(), push_ttl)
        } else {
            None
        };
        let tick_budget = compute_tick_budget(
            needs_live_tick,
            live_tick_interval,
            min_opt(min_opt(cancel_remaining, pull_remaining), push_remaining),
        );
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
                // d-66: F4's destructive clear-history confirm is
                // modal — route y/n/Esc to the confirm handler
                // before the verify-edit handler and before
                // key_action (where bare Esc would map to Quit).
                if app.current_screen == Screen::F4
                    && app.profile.is_confirming_clear()
                    && handle_profile_clear_confirm_keystroke(&key, &mut app)
                {
                    continue;
                }
                if app.current_screen == Screen::F4
                    && app.verify.focus().is_editing()
                    && handle_verify_keystroke(&key, &mut app, &verify_run_tx)
                {
                    continue;
                }
                // d-65: F1's destructive-push confirm is modal —
                // route y/n/Esc to the confirm handler BEFORE the
                // edit handler (the modal is still in Editing state,
                // so is_editing is also true; this guard wins).
                if app.current_screen == Screen::F1
                    && app.f1_trigger.is_confirming()
                    && handle_f1_trigger_confirm_keystroke(&key, &mut app)
                {
                    continue;
                }
                // d-58: F1's `t` trigger-transfer modal is an
                // input mode — while open, chars / Backspace / Tab
                // / Esc / Enter route to the modal instead of the
                // F1 dispatcher (so `t` etc. are text, not actions).
                if app.current_screen == Screen::F1
                    && app.f1_trigger.is_editing()
                    && handle_f1_trigger_keystroke(&key, &mut app)
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
                // d-55/d-57: F3's `m` mirror / `v` move confirm is
                // modal while open — route y/n/Esc to the destructive
                // handler (like the delete confirm) before the normal
                // F3 dispatcher.
                if app.current_screen == Screen::F3
                    && app.f3_pull.is_confirming_destructive()
                    && handle_f3_destructive_confirm_keystroke(&key, &mut app)
                {
                    continue;
                }
                // d-45: F3's `D` delete confirm is modal while
                // open — route keystrokes to the delete handler
                // (y/n/Esc) before the normal F3 dispatcher.
                if app.current_screen == Screen::F3
                    && app.f3_del.is_confirming()
                    && handle_f3_delete_keystroke(&key, &mut app)
                {
                    continue;
                }
                // If handle_verify_keystroke returned false
                // (F-keys, Ctrl-c) or we're not in editing
                // mode, fall through to the action
                // dispatcher.
                if let Some(action) = key_action(&key, &KeyMap::from_config(&tui_config)) {
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
                        // m2f-9: auto re-fan the merged Subscribe streams
                        // when a discovery update changes the watched-daemon
                        // set, so a newly discovered daemon's transfers
                        // appear (and a vanished one's streams drop) without
                        // an explicit `r`. The change is deferred if a setup
                        // is already in flight (see handle_discovery_watch_change).
                        let before = f2_watched_identities(&app);
                        app.daemons.replace_from_discovery(&services, Instant::now());
                        handle_discovery_watch_change(
                            &mut app,
                            &before,
                            &mut transfers_event_rx,
                            &f2_setup_tx,
                        );
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
                        Ok((daemon_state, capacities)) => DaemonDetail::Loaded {
                            state: Box::new(daemon_state),
                            capacities,
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
                // m2f-5 R2: `None` (all merged senders closed) drops the
                // receiver; a single daemon's Error keeps it alive so the
                // other daemons' forwarders keep feeding F2.
                if !apply_f2_event(&mut app, event) {
                    transfers_event_rx = None;
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
                            F2SetupPayload::Ready { event_rx, snapshots } => {
                                let mut rx = event_rx;
                                // m2f-5: hydrate each watched daemon additively, so
                                // every daemon's transfers coexist in the view. A
                                // per-daemon GetState failure degrades only if NONE
                                // succeeded (the streams may still be live).
                                let now = Instant::now();
                                let mut any_snapshot_ok = false;
                                let mut last_snapshot_err: Option<String> = None;
                                for (daemon, snap) in snapshots {
                                    match snap {
                                        Ok(snapshot) => {
                                            app.transfers.merge_snapshot(&daemon, snapshot, now);
                                            any_snapshot_ok = true;
                                        }
                                        Err(err) => last_snapshot_err = Some(err),
                                    }
                                }
                                if !any_snapshot_ok {
                                    if let Some(err) = last_snapshot_err {
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
                        // m2f-9 R2: a discovery update changed the watch set
                        // while this setup was in flight. Now that it has
                        // landed and `pending` is cleared, re-fan against the
                        // current set so the daemon discovered mid-flight is
                        // actually watched (a no-op if the set is now empty).
                        // Applies to both Ready and Failed — a failed setup
                        // followed by a daemon appearing should retry.
                        apply_deferred_refan(
                            &mut app,
                            &mut transfers_event_rx,
                            &f2_setup_tx,
                        );
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
                        Ok((files, bytes, deleted)) => {
                            let applied =
                                app.f3_pull.apply_done(request_id, files, bytes, deleted, at);
                            // d-53: advance the batch — start the
                            // next queued source with the same
                            // dest, or clear the batch when done.
                            // Only advance on an applied (current-
                            // generation) reply.
                            if applied {
                                advance_batch_pull(&mut app);
                            }
                        }
                        Err(message) => {
                            let applied =
                                app.f3_pull.apply_error(request_id, message, at);
                            // d-53: abort the rest of the batch on
                            // a failed pull — don't silently keep
                            // pulling after an error the operator
                            // hasn't seen.
                            if applied {
                                app.f3_batch_pull = None;
                            }
                        }
                    }
                }
            }
            // d-63: F1 push live-progress snapshots.
            snapshot = f1_push_progress_rx.recv() => {
                if let Some(F1PushProgress {
                    request_id,
                    files,
                    bytes,
                    bytes_per_sec,
                }) = snapshot
                {
                    app.f1_push
                        .apply_progress(request_id, files, bytes, bytes_per_sec);
                }
            }
            // d-61: F1 push replies. Generation-guarded in
            // `F1PushState::apply_*` (compares `request_id`).
            reply = f1_push_reply_rx.recv() => {
                if let Some(F1PushReply { request_id, result }) = reply {
                    let at = Instant::now();
                    match result {
                        Ok((files, bytes)) => {
                            app.f1_push.apply_done(request_id, files, bytes, at);
                        }
                        Err(message) => {
                            app.f1_push.apply_error(request_id, message, at);
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
            // d-41: F3 du replies. Same generation guard as the
            // pull reply — `apply_*` drop a superseded query.
            reply = f3_du_reply_rx.recv() => {
                if let Some(F3DuReply { request_id, result }) = reply {
                    match result {
                        Ok((bytes, files)) => {
                            app.f3_du.apply_done(request_id, bytes, files);
                        }
                        Err(message) => {
                            app.f3_du.apply_error(request_id, message);
                        }
                    }
                }
            }
            // d-45: F3 delete (Purge) replies. Same generation
            // guard as the du/pull replies.
            reply = f3_del_reply_rx.recv() => {
                if let Some(F3DelReply { request_id, result }) = reply {
                    match result {
                        Ok(files_deleted) => {
                            // d-45 R2: a stale delete reply (superseded
                            // run) returns false — only refresh the
                            // listing when THIS delete actually applied,
                            // so the deleted row leaves the F3 table
                            // instead of lingering as an apparently-live
                            // entry. Reuses the `r`-key refresh path; the
                            // loop auto-kicks the re-fetch.
                            if app.f3_del.apply_done(request_id, files_deleted, Instant::now()) {
                                handle_f3_refresh(
                                    &mut app.browse,
                                    app.browse_target.is_some(),
                                    &mut app.browse_last_fetched_view,
                                );
                            }
                        }
                        Err(message) => {
                            app.f3_del.apply_error(request_id, message, Instant::now());
                        }
                    }
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
    transfers_event_rx: &mut Option<mpsc::Receiver<F2Event>>,
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
            UserAction::SelectFirst => app.daemons.select_first(),
            UserAction::SelectLast => app.daemons.select_last(),
            // d-47: `enter` / `l` / `→` on a daemon row switches
            // the F3 browse target to that daemon and jumps to
            // F3. Resolve the endpoint first so the immutable
            // `app.daemons` borrow ends before we mutate. Local
            // (and any row without a resolvable endpoint) is a
            // no-op — F3 is a remote browser.
            UserAction::Descend => {
                if let Some(endpoint) = f1_browse_target(&app.daemons) {
                    retarget_browse(app, endpoint.clone());
                    // d-48: F2 follows — restart the Subscribe
                    // stream against the daemon we just switched
                    // to, so its transfers track what we browse.
                    let gen = reset_f2_for_resubscribe(app, &endpoint, transfers_event_rx);
                    // m2f-5: re-fan to all watched daemons (the reset
                    // repointed parsed_remote to the selected one, so
                    // it's included).
                    spawn_f2_setup_task(f2_watched_endpoints(app), gen, f2_setup_tx.clone());
                }
            }
            // d-58: `t` opens the trigger-transfer modal for the
            // selected daemon (TUI_DESIGN §5.1 `[t] trigger
            // transfer`). Source is prefilled to the daemon's
            // `host:port:/` so the operator just appends a module
            // path; dest starts empty. Local / no-endpoint rows are
            // a no-op — a trigger needs a remote source. Commit runs
            // a remote→local pull (see the trigger keystroke handler).
            UserAction::F1TriggerBegin => {
                if let Some(endpoint) = f1_browse_target(&app.daemons) {
                    let prefill = format!("{}:/", endpoint.host_port_display());
                    app.f1_trigger.begin(prefill);
                }
            }
            _ => {}
        },
        Screen::F2 => match action {
            UserAction::Refresh => {
                // m2f-5 R2: `r` re-fans the merged Subscribe setup to the
                // CURRENT watched set — so it picks up daemons discovered
                // since the last setup, even while a stream is live (a
                // plain GetState refresh of parsed_remote would miss
                // them). A no-op while a setup is already pending.
                refan_f2_setup(app, transfers_event_rx, f2_setup_tx);
            }
            // d-21: cursor selection in the active table.
            // First press selects the newest row (index 0);
            // subsequent presses walk through.
            UserAction::SelectNext => app.transfers.select_next_active(),
            UserAction::SelectPrev => app.transfers.select_prev_active(),
            UserAction::SelectFirst => app.transfers.select_first_active(),
            UserAction::SelectLast => app.transfers.select_last_active(),
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
                } else if let (Some(id), Some(daemon)) = (
                    app.transfers.selected_active_id().map(|s| s.to_string()),
                    // m2f-7: cancel targets the SELECTED row's daemon, not
                    // parsed_remote — F2 shows rows from every daemon.
                    app.transfers
                        .selected_active_daemon()
                        .map(|s| s.to_string()),
                ) {
                    if tui_config.transfer.confirm_cancel {
                        app.cancel_status = F2CancelStatus::Confirming {
                            transfer_id: id,
                            daemon,
                        };
                    } else if let Some(endpoint) = cancel_endpoint(&daemon) {
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
                // d-30 R2: clone-out the variant payload
                // before mutating `app.cancel_status` so
                // the borrow doesn't outlive the match.
                let confirmed = match &app.cancel_status {
                    F2CancelStatus::Confirming {
                        transfer_id,
                        daemon,
                    } => ConfirmedCancel::Single {
                        id: transfer_id.clone(),
                        daemon: daemon.clone(),
                    },
                    F2CancelStatus::ConfirmingBatch { targets } => {
                        ConfirmedCancel::Batch(targets.clone())
                    }
                    _ => return,
                };
                match confirmed {
                    // m2f-7: single cancel targets the captured row's daemon.
                    ConfirmedCancel::Single { id, daemon } => {
                        let Some(endpoint) = cancel_endpoint(&daemon) else {
                            app.cancel_status = F2CancelStatus::Idle;
                            return;
                        };
                        app.cancel_request_seq += 1;
                        let rid = app.cancel_request_seq;
                        app.cancel_status = F2CancelStatus::Sending {
                            transfer_id: id.clone(),
                            request_id: rid,
                        };
                        spawn_cancel_transfer(rid, endpoint, id, app.cancel_reply_tx.clone());
                    }
                    // m2f-8: batch cancel sends each CancelJob to the
                    // daemon that owns the transfer.
                    ConfirmedCancel::Batch(targets) => {
                        let count = spawn_cancels_for_targets(
                            targets,
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
                } else {
                    // m2f-8: batch cancels every active row across ALL
                    // watched daemons (each target carries its own
                    // daemon) — no longer gated on parsed_remote.
                    let targets = snapshot_active_targets(&app.transfers);
                    if targets.is_empty() {
                        // No active transfers — silent no-op.
                    } else if tui_config.transfer.confirm_cancel {
                        app.cancel_status = F2CancelStatus::ConfirmingBatch { targets };
                    } else {
                        let count = spawn_cancels_for_targets(
                            targets,
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
                    app.browse_target.is_some(),
                    &mut app.browse_last_fetched_view,
                );
            }
            UserAction::SelectNext => app.browse.select_next(),
            UserAction::SelectPrev => app.browse.select_prev(),
            UserAction::SelectFirst => app.browse.select_first(),
            UserAction::SelectLast => app.browse.select_last(),
            // d-49: `space` toggles the cursor row's mark.
            UserAction::F3ToggleMark => app.browse.toggle_mark(),
            // d-51: `a` marks/clears all visible rows.
            UserAction::F3ToggleMarkAll => app.browse.toggle_mark_all_visible(),
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
                let source = app.browse_target.as_ref().and_then(|base| {
                    browse::pull_source_endpoint(app.browse.view(), app.browse.selected_row(), base)
                });
                if !busy {
                    if let Some(source) = source {
                        app.f3_pull.begin(source);
                    }
                }
            }
            // d-55: `m` opens the mirror destination prompt for the
            // cursor row. Same source resolution as `p`; the kind
            // rides the pull state so commit routes through the
            // destructive confirm. The mirror reads FROM the remote
            // and deletes at the LOCAL dest, so no read-only gate is
            // needed (read-only is about writing to the module).
            UserAction::F3MirrorBegin => {
                let busy = app.f3_pull.is_entering_dest()
                    || app.f3_pull.is_running()
                    || app.f3_pull.is_confirming_destructive();
                let source = app.browse_target.as_ref().and_then(|base| {
                    browse::pull_source_endpoint(app.browse.view(), app.browse.selected_row(), base)
                });
                if !busy {
                    if let Some(source) = source {
                        app.f3_pull.begin_mirror(source);
                    }
                }
            }
            // d-57: `v` opens the move destination prompt for the
            // cursor row. Like mirror, but the destructive phase
            // deletes the REMOTE source after a complete receive.
            // That deletes from the module, so it IS gated on
            // read-only — a read-only source can't be moved out of.
            UserAction::F3MoveBegin => {
                let busy = app.f3_pull.is_entering_dest()
                    || app.f3_pull.is_running()
                    || app.f3_pull.is_confirming_destructive();
                let source = if app.browse.current_module_read_only() {
                    None
                } else {
                    app.browse_target.as_ref().and_then(|base| {
                        browse::pull_source_endpoint(
                            app.browse.view(),
                            app.browse.selected_row(),
                            base,
                        )
                    })
                };
                // d-57 R2 (reviewer reopen): a move deletes the
                // remote source after the receive, so the source
                // must be a deletable PATH — never a module root.
                // In the top-level modules view `pull_source_endpoint`
                // maps a module row to `Module { rel_path: "" }`; the
                // daemon rejects empty/root purge paths, so without
                // this gate `v` on a module row would copy the whole
                // module locally and then fail the source delete. Same
                // `is_deletable_remote_path` gate F3 delete (`D`) uses
                // to refuse module-root purges. (This also covers the
                // top-level read-only module row, which is a module
                // root and so rejected here regardless of read-only —
                // `current_module_read_only()` only tracks a descended
                // module, not the selected top-level row.)
                let source = source.filter(is_deletable_remote_path);
                if !busy {
                    if let Some(source) = source {
                        app.f3_pull.begin_move(source);
                    }
                }
            }
            // d-53: `P` batch-pulls the marked set. Opens the
            // dest prompt for the first source and queues the
            // rest; on each pull's completion the reply arm
            // starts the next with the same dest. No-op without
            // marks or with a pull already in flight.
            UserAction::F3BatchPullBegin => {
                let busy = app.f3_pull.is_entering_dest()
                    || app.f3_pull.is_running()
                    || app.f3_batch_pull.is_some();
                if !busy && app.browse.marked_count() > 0 {
                    if let Some(base) = app.browse_target.clone() {
                        let mut sources: std::collections::VecDeque<RemoteEndpoint> =
                            app.browse.marked_endpoints(&base).into_iter().collect();
                        if let Some(first) = sources.pop_front() {
                            let total = sources.len() + 1;
                            app.f3_batch_pull = Some(BatchPull {
                                remaining: sources,
                                raw_dest: String::new(),
                                done: 0,
                                total,
                            });
                            app.f3_pull.begin(first);
                        }
                    }
                }
            }
            // d-41: `u` runs du for the cursor row. Resolves
            // the same cursor endpoint the pull preview uses;
            // the canonical `.display()` spec is both the du
            // target and the path the renderer gates on.
            UserAction::F3DuBegin => {
                let target = app.browse_target.as_ref().and_then(|base| {
                    browse::pull_source_endpoint(app.browse.view(), app.browse.selected_row(), base)
                });
                if let Some(target) = target {
                    // d-43: a cache hit serves the total instantly
                    // (status already Done); only a miss spawns the
                    // RPC.
                    if let f3du::DuBegin::Fetch(id) = app.f3_du.begin(target.display()) {
                        spawn_f3_du(id, target, app.f3_du_reply_tx.clone());
                    }
                }
            }
            // d-45: `D` opens the delete confirm prompt. Gated
            // on a resolvable cursor endpoint that is NOT a
            // module root — refusing to delete a whole module
            // from the TUI (mirrors `blit rm`). Read-only
            // modules are enforced server-side: the Purge fails
            // and the error surfaces in the footer.
            UserAction::F3DeleteBegin => {
                // d-46: read-only modules disable `D` up front
                // (TUI_DESIGN §5.3). The daemon also rejects the
                // Purge, but gating here avoids opening a confirm
                // prompt for an operation that can't succeed.
                let base = if app.browse.current_module_read_only() {
                    None
                } else {
                    app.browse_target.clone()
                };
                if let Some(base) = base {
                    // d-50: a non-empty multi-select is the batch
                    // target; otherwise the cursor row.
                    let batch = app.browse.marked_count() > 0;
                    let endpoints: Vec<RemoteEndpoint> = if batch {
                        app.browse.marked_endpoints(&base)
                    } else {
                        browse::pull_source_endpoint(
                            app.browse.view(),
                            app.browse.selected_row(),
                            &base,
                        )
                        .into_iter()
                        .collect()
                    };
                    if let Some((module_ep, rel_paths, label, gate)) =
                        build_delete_request(endpoints, batch)
                    {
                        app.f3_del.begin(module_ep, rel_paths, label, gate);
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
                // d-66: clearing the history log is permanent, so
                // arm a y/N confirm instead of wiping on this
                // single keystroke. The actual clear runs from
                // `handle_profile_clear_confirm_keystroke` on `y`.
                app.profile.begin_clear_confirm();
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

/// m2f-5 R2: (re)start the merged F2 Subscribe setup against the
/// CURRENT watched daemon set. The F2 `r` refresh and the initial
/// open both go through here. Drops any live merged receiver (its
/// forwarders then exit) and spawns a fresh fan-out, so a refresh
/// picks up daemons discovered since the last setup. Returns whether
/// a setup was spawned.
///
/// No-op (returns `false`) while a setup is already pending — the
/// round-3 overlap-race guard: pressing `r` mid-setup must NOT spawn
/// a duplicate.
///
/// m2f-9 R3: reconciles the view to the watch set on every call. A
/// daemon that left discovery can no longer send a Complete/Error
/// event, so its in-flight active rows are pruned here
/// ([`TransfersState::retain_active_daemons`]) rather than lingering
/// forever. When the watch set is now **empty** (last daemon vanished /
/// mDNS-only with nothing found), the live receiver is still dropped and
/// F2 returns to the no-daemon state — the pre-R3 early-return left the
/// stale stream live.
fn refan_f2_setup(
    app: &mut AppState,
    transfers_event_rx: &mut Option<mpsc::Receiver<F2Event>>,
    f2_setup_tx: &mpsc::Sender<F2SetupReply>,
) -> bool {
    if app.transfers_setup_pending {
        return false;
    }
    let watched = f2_watched_endpoints(app);
    // Reconcile first: drop active rows for daemons no longer watched
    // (recent history is kept), so a shrink (`A+B → A`) doesn't strand
    // `B`'s rows, and an empty set clears the table.
    let watched_ids: std::collections::BTreeSet<String> =
        watched.iter().map(|ep| ep.host_port_display()).collect();
    app.transfers.retain_active_daemons(&watched_ids);
    // m2f-10: a re-fan opens fresh streams for the new watch set, so
    // per-daemon stream health resets — any prior degraded marks belong
    // to the streams we're dropping.
    app.f2_degraded_daemons.clear();
    // Drop the old merged stream — its per-daemon forwarders exit when
    // the receiver is gone.
    *transfers_event_rx = None;
    if watched.is_empty() {
        // Nothing left to watch. No stream to open; reflect the
        // no-daemon state rather than leaving the vanished daemon's
        // receiver live (the F2 pane shows no remote / mDNS-only).
        app.transfers_status = ConnectionStatus::NoRemote;
        return false;
    }
    app.transfers_status = ConnectionStatus::Connecting;
    app.transfers_setup_gen += 1;
    app.transfers_setup_pending = true;
    spawn_f2_setup_task(watched, app.transfers_setup_gen, f2_setup_tx.clone());
    true
}

/// m2f-9: react to an mDNS discovery update by comparing the watch set
/// against `before` (captured before `replace_from_discovery`). On a
/// genuine change, re-fan the merged Subscribe streams so the view
/// tracks the current daemon set without a manual `r`.
///
/// R2: if a setup is already in flight (`transfers_setup_pending` — e.g.
/// the startup fan-out, whose receiver isn't live yet), `refan_f2_setup`
/// would no-op and silently drop the change. So in that case record a
/// deferred re-fan instead; [`apply_deferred_refan`] runs it once the
/// pending setup's reply lands. Returns whether a re-fan was spawned
/// *now* (false when deferred or when nothing changed).
fn handle_discovery_watch_change(
    app: &mut AppState,
    before: &std::collections::BTreeSet<String>,
    transfers_event_rx: &mut Option<mpsc::Receiver<F2Event>>,
    f2_setup_tx: &mpsc::Sender<F2SetupReply>,
) -> bool {
    if &f2_watched_identities(app) == before {
        return false;
    }
    if app.transfers_setup_pending {
        app.transfers_refan_after_setup = true;
        false
    } else {
        refan_f2_setup(app, transfers_event_rx, f2_setup_tx)
    }
}

/// m2f-9 R2: run a deferred re-fan recorded by
/// [`handle_discovery_watch_change`] while a setup was pending. Called
/// from the setup-reply arm once that setup has landed and `pending` is
/// cleared, so the daemon discovered mid-flight ends up watched. Returns
/// whether a re-fan was spawned.
fn apply_deferred_refan(
    app: &mut AppState,
    transfers_event_rx: &mut Option<mpsc::Receiver<F2Event>>,
    f2_setup_tx: &mpsc::Sender<F2SetupReply>,
) -> bool {
    if std::mem::take(&mut app.transfers_refan_after_setup) {
        refan_f2_setup(app, transfers_event_rx, f2_setup_tx)
    } else {
        false
    }
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
        event_rx: mpsc::Receiver<F2Event>,
        /// m2f-5: one entry per daemon whose Subscribe stream opened —
        /// `(daemon identity, its initial GetState result)`. Each Ok
        /// is merged additively so all watched daemons coexist in the
        /// view.
        snapshots: Vec<(String, Result<DaemonState, String>)>,
    },
    /// No watched daemon could be subscribed (none reachable / none
    /// to watch).
    Failed(String),
}

/// Background task for F2 setup. Opens the Subscribe
/// stream and fires the initial `GetState`. Either result
/// becomes a single `F2SetupReply` message into `tx`,
/// tagged with the generation the caller bumped before
/// spawning. Running this off the router's await means a
/// slow / unreachable remote does NOT block the TUI's
/// first draw.
/// m2f-5: fan out F2's Subscribe to every watched daemon. Opens one
/// stream per daemon into a SINGLE merged channel and fetches each
/// daemon's initial `GetState`. A daemon whose subscribe fails is
/// skipped (the others still show); the reply is `Failed` only when
/// none could be reached. Per-daemon reconnect / degraded UI is a
/// follow-up (m2f-6).
fn spawn_f2_setup_task(daemons: Vec<RemoteEndpoint>, gen: u64, tx: mpsc::Sender<F2SetupReply>) {
    tokio::spawn(async move {
        let (merged_tx, merged_rx) = mpsc::channel::<F2Event>(TUI_EVENT_BUFFER);
        let mut snapshots = Vec::new();
        let mut any_subscribed = false;
        for endpoint in &daemons {
            // Each daemon forwards into a clone of the shared sender.
            if open_subscribe_stream(endpoint, merged_tx.clone())
                .await
                .is_ok()
            {
                any_subscribed = true;
                let snap = jobs::query(endpoint, 0)
                    .await
                    .map_err(|err| format!("{err:#}"));
                snapshots.push((endpoint.host_port_display(), snap));
            }
        }
        // Drop our handle so the forwarders' clones are the only
        // senders — the merged receiver then closes once every
        // watched stream ends.
        drop(merged_tx);

        let payload = if any_subscribed {
            F2SetupPayload::Ready {
                event_rx: merged_rx,
                snapshots,
            }
        } else if daemons.is_empty() {
            F2SetupPayload::Failed("no daemons discovered yet".to_string())
        } else {
            F2SetupPayload::Failed("no reachable daemons".to_string())
        };
        let _ = tx.send(F2SetupReply { gen, payload }).await;
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
        let result = match jobs::query(&endpoint, 0).await {
            Ok(state) => {
                // d-54: fan out a `df` per advertised module to
                // enrich the detail with capacity. Best-effort —
                // a module whose FilesystemStats fails is simply
                // omitted (the GetState succeeded, which is the
                // primary payload). Sequential keeps it simple;
                // module counts are small.
                let mut capacities = Vec::new();
                for module in &state.modules {
                    if let Ok(stats) =
                        blit_app::admin::df::query(&endpoint, module.name.clone()).await
                    {
                        capacities.push((module.name.clone(), stats.used_bytes, stats.total_bytes));
                    }
                }
                Ok((state, capacities))
            }
            Err(err) => Err(format!("{err:#}")),
        };
        let _ = tx
            .send(DetailUpdate {
                instance_name,
                request_id,
                result,
            })
            .await;
    });
}

/// Reply envelope for the detail fetcher. d-54: the `Ok`
/// payload bundles the GetState snapshot with the per-module
/// `(name, used, total)` capacity fan-out.
struct DetailUpdate {
    instance_name: String,
    request_id: u64,
    result: Result<(DaemonState, Vec<(String, u64, u64)>), String>,
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
/// d-47 R2: the daemon endpoint `enter` on F1 should browse, or
/// `None` when the action is a no-op.
///
/// Gates out the **Local** row: `DaemonsState::endpoint_for_row`
/// returns the loopback `127.0.0.1:9031` for Local (so the
/// daemon's own RPCs work), but F3 is a *remote* browser — Enter
/// on Local must do nothing, not browse loopback. (Round 1
/// retargeted on any `Some`, which silently browsed loopback.)
fn f1_browse_target(daemons: &daemons::DaemonsState) -> Option<RemoteEndpoint> {
    daemons
        .selected_row()
        .filter(|row| !row.is_local())
        .and_then(daemons::DaemonsState::endpoint_for_row)
}

/// d-47: point the F3 browser at `endpoint` and jump to F3.
/// Resets the browse state to a fresh Modules view and clears
/// `browse_last_fetched_view` so the loop's fetch driver kicks a
/// fresh listing for the new daemon.
fn retarget_browse(app: &mut AppState, endpoint: RemoteEndpoint) {
    app.browse_target = Some(endpoint);
    app.browse = browse::BrowseState::new();
    app.browse_last_fetched_view = None;
    app.current_screen = Screen::F3;
}

/// d-48: reset F2 state so its Subscribe stream re-opens against
/// `endpoint` (F2 follows the daemon the operator switched to on
/// F1). Drops the old stream (`event_rx = None`), clears the rows,
/// repoints `parsed_remote` and the label, marks a setup pending,
/// and bumps the generation so a stale in-flight setup reply from
/// the previous daemon is ignored by the loop's gen gate. Returns
/// the new generation for the caller's `spawn_f2_setup_task`.
///
/// Mirrors the startup / `r`-refresh setup path exactly — same
/// machinery, just retargeted — so the generation-guarded
/// lifecycle (a1-6b round 3) stays intact.
fn reset_f2_for_resubscribe(
    app: &mut AppState,
    endpoint: &RemoteEndpoint,
    transfers_event_rx: &mut Option<mpsc::Receiver<F2Event>>,
) -> u64 {
    app.parsed_remote = Some(endpoint.clone());
    app.remote_label = endpoint.host_port_display();
    app.transfers = state::TransfersState::new();
    app.transfers_status = ConnectionStatus::Connecting;
    *transfers_event_rx = None;
    // d-48 R2: drop any pending/in-flight cancel from the OLD
    // daemon. A `Confirming`/`ConfirmingBatch` left open would,
    // on `y`, fire CancelJob with the old daemon's transfer
    // id(s) against the new daemon's endpoint (the confirm path
    // re-reads `parsed_remote`, now repointed). Clearing to Idle
    // also drops a late `Sending` reply — the reply arm only
    // applies when status is `Sending` with a matching id, so a
    // non-`Sending` status makes the stale reply inert.
    app.cancel_status = F2CancelStatus::Idle;
    app.transfers_setup_pending = true;
    app.transfers_setup_gen += 1;
    app.transfers_setup_gen
}

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
        F2CancelStatus::Confirming { transfer_id, .. } => F2CancelDisplay::ConfirmingCancel {
            transfer_id: transfer_id.clone(),
        },
        F2CancelStatus::ConfirmingBatch { targets } => F2CancelDisplay::ConfirmingBatch {
            count: targets.len(),
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

/// d-58/d-59: bridge the F1 trigger modal to the renderer-facing
/// `TriggerPrompt`. `None` when the modal is closed.
fn f1_trigger_prompt(state: &f1trigger::F1TriggerState) -> Option<screens::f1::TriggerPrompt> {
    use f1trigger::{F1TriggerStatus, TriggerField};
    match state.status() {
        F1TriggerStatus::Idle => None,
        F1TriggerStatus::Editing {
            source,
            dest,
            focus,
            kind,
            error,
            confirming,
        } => Some(screens::f1::TriggerPrompt {
            source: source.clone(),
            dest: dest.clone(),
            source_focused: *focus == TriggerField::Source,
            // imperative verb ("copy"/"mirror"/"move"); pull's
            // verb triple uses "pull" for Copy, so spell "copy"
            // here to match the design's launcher vocabulary.
            mode: match kind {
                f3pull::PullKind::Copy => "copy",
                f3pull::PullKind::Mirror => "mirror",
                f3pull::PullKind::Move => "move",
            },
            // Mirror + move delete something → flag red.
            destructive: kind.is_destructive(),
            // d-62: inline validation error from the last commit.
            error: error.clone(),
            // d-65/d-71: a destructive transfer awaiting y/N confirm.
            // The detail spells out what gets deleted. Move's victim
            // depends on direction: a local→remote push move deletes
            // the LOCAL source; a remote→remote delegated move deletes
            // the REMOTE source — classify the source string to say
            // which.
            confirm_detail: confirming.then(|| match kind {
                f3pull::PullKind::Mirror => "deletes extraneous at dest",
                f3pull::PullKind::Move => {
                    use blit_app::endpoints::{parse_transfer_endpoint, Endpoint};
                    match parse_transfer_endpoint(source) {
                        Ok(Endpoint::Remote(_)) => "deletes the remote source",
                        _ => "deletes the local source",
                    }
                }
                f3pull::PullKind::Copy => "",
            }),
        }),
    }
}

/// d-61: bridge the F1 push state to the renderer-facing
/// `PushStatusDisplay`. `None` when Idle (the discovery footer
/// shows instead).
fn f1_push_status(state: &f1push::F1PushState) -> Option<screens::f1::PushStatusDisplay> {
    use f1push::F1PushStatus;
    use screens::f1::PushStatusDisplay;
    match state.status() {
        F1PushStatus::Idle => None,
        F1PushStatus::Running {
            label,
            files,
            bytes,
            bytes_per_sec,
            kind,
            delegated,
            ..
        } => Some(PushStatusDisplay::Running {
            label: label.clone(),
            files: *files,
            bytes: *bytes,
            bytes_per_sec: *bytes_per_sec,
            // d-65/d-68: present-participle verb for the kind
            // (or "delegating" for a remote→remote delegated copy).
            verb: push_present_verb(*kind, *delegated),
        }),
        F1PushStatus::Done {
            files,
            bytes,
            label,
            kind,
            delegated,
            ..
        } => Some(PushStatusDisplay::Done {
            files: *files,
            bytes: *bytes,
            label: label.clone(),
            // d-65/d-68: past-tense verb for the kind.
            verb: push_past_verb(*kind, *delegated),
        }),
        F1PushStatus::Error {
            message,
            kind,
            delegated,
            ..
        } => Some(PushStatusDisplay::Error {
            message: message.clone(),
            verb: push_past_verb(*kind, *delegated),
        }),
    }
}

/// d-65: push footer verbs by kind. Copy reads as "push" (not
/// "pull") since this is the local→remote direction. d-68/d-70: a
/// remote→remote delegated copy reads "delegating/delegated" (the
/// CLI host isn't in the byte path, so neither push nor pull fits);
/// a delegated mirror reads "mirroring/mirrored" — the destructive
/// dest-purge is the salient thing and the footer label shows the
/// remote dest, so the delegated context stays clear.
fn push_present_verb(kind: f3pull::PullKind, delegated: bool) -> &'static str {
    match (delegated, kind) {
        (true, f3pull::PullKind::Copy) => "delegating",
        (_, f3pull::PullKind::Mirror) => "mirroring",
        (_, f3pull::PullKind::Move) => "moving",
        (false, f3pull::PullKind::Copy) => "pushing",
    }
}

fn push_past_verb(kind: f3pull::PullKind, delegated: bool) -> &'static str {
    match (delegated, kind) {
        (true, f3pull::PullKind::Copy) => "delegated",
        (_, f3pull::PullKind::Mirror) => "mirrored",
        (_, f3pull::PullKind::Move) => "moved",
        (false, f3pull::PullKind::Copy) => "pushed",
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
        F3PullStatus::EnteringDest { dest, kind, .. } => F3PullDisplay::EnteringDest {
            dest: dest.clone(),
            // imperative verb ("pull"/"mirror"/"move")
            verb: kind.verbs().0,
        },
        // d-55/d-57: destructive confirm (y/N). The detail spells
        // out what gets removed so the operator knows the stakes.
        F3PullStatus::Confirm { dest, kind, .. } => F3PullDisplay::Confirm {
            dest: dest.clone(),
            verb: kind.verbs().0,
            detail: confirm_detail(*kind),
        },
        F3PullStatus::Running {
            dest,
            files,
            bytes,
            bytes_per_sec,
            kind,
            ..
        } => F3PullDisplay::Running {
            dest: dest.clone(),
            files: *files,
            bytes: *bytes,
            bytes_per_sec: *bytes_per_sec,
            // present participle ("pulling"/"mirroring"/"moving")
            verb: kind.verbs().1,
        },
        F3PullStatus::Done {
            files,
            bytes,
            dest,
            kind,
            deleted,
            ..
        } => F3PullDisplay::Done {
            files: *files,
            bytes: *bytes,
            dest: dest.clone(),
            // past tense ("pulled"/"mirrored"/"moved")
            verb: kind.verbs().2,
            deleted: *deleted,
        },
        F3PullStatus::Error { message, .. } => F3PullDisplay::Error {
            message: message.clone(),
        },
    }
}

/// d-55/d-57: the destructive-confirm detail line for each kind —
/// what the operator is about to lose. `Copy` is non-destructive
/// and never reaches the confirm gate.
fn confirm_detail(kind: f3pull::PullKind) -> &'static str {
    use f3pull::PullKind;
    match kind {
        PullKind::Mirror => "deletes extraneous",
        PullKind::Move => "deletes the remote source",
        PullKind::Copy => "",
    }
}

/// d-41: bridge the F3 du state machine to the renderer-facing
/// `F3DuDisplay`. This is where the path-match gating lives:
/// the result only shows while the cursor is still on the path
/// the du was computed for (`current_path`, the cursor's
/// canonical spec). A `Running`/`Done`/`Error` for any other
/// path renders as `Hidden`, so an outdated subtree total never
/// appears against the wrong row.
fn f3_du_to_display(
    status: &f3du::F3DuStatus,
    current_path: Option<&str>,
) -> screens::f3::F3DuDisplay {
    use f3du::F3DuStatus;
    use screens::f3::F3DuDisplay;
    let matches = |path: &str| current_path == Some(path);
    match status {
        F3DuStatus::Idle => F3DuDisplay::Hidden,
        F3DuStatus::Running { path, .. } if matches(path) => F3DuDisplay::Running,
        F3DuStatus::Done {
            path, bytes, files, ..
        } if matches(path) => F3DuDisplay::Done {
            bytes: *bytes,
            files: *files,
        },
        F3DuStatus::Error { path, message } if matches(path) => F3DuDisplay::Error {
            message: message.clone(),
        },
        _ => F3DuDisplay::Hidden,
    }
}

/// d-45: bridge the F3 delete state to the renderer-facing
/// `F3DelDisplay`. `Confirming` / `Deleting` always show (an
/// active operation); `Done` / `Error` are path-gated like the
/// du display — a stale outcome hides once the cursor leaves the
/// deleted path.
fn f3_del_to_display(
    status: &f3del::F3DelStatus,
    current_path: Option<&str>,
) -> screens::f3::F3DelDisplay {
    use f3del::F3DelStatus;
    use screens::f3::F3DelDisplay;
    // d-50: a single-row delete carries `gate_path = Some(spec)`
    // and hides its outcome once the cursor leaves that path
    // (the d-45 behavior). A batch carries `None` and shows the
    // outcome until the next action (its rows are gone after the
    // post-delete refresh anyway).
    let gated = |gate: &Option<String>| match gate {
        Some(p) => current_path == Some(p.as_str()),
        None => true,
    };
    match status {
        F3DelStatus::Idle => F3DelDisplay::Hidden,
        F3DelStatus::Confirming { label, .. } => F3DelDisplay::Confirming {
            label: label.clone(),
        },
        F3DelStatus::Deleting { .. } => F3DelDisplay::Deleting,
        F3DelStatus::Done {
            label,
            files_deleted,
            gate_path,
            ..
        } if gated(gate_path) => F3DelDisplay::Done {
            label: label.clone(),
            files_deleted: *files_deleted,
        },
        F3DelStatus::Error {
            label,
            message,
            gate_path,
            ..
        } if gated(gate_path) => F3DelDisplay::Error {
            message: message.clone(),
        },
        _ => F3DelDisplay::Hidden,
    }
}

/// d-53: sequential batch-pull state. `P` on F3 pulls every
/// marked source into one destination, one at a time, reusing
/// the single-source `f3_pull` machine. `raw_dest` is captured
/// once (when the operator confirms the first pull's prompt) and
/// reused for each queued source; `remaining` holds the sources
/// not yet started. `done`/`total` drive the footer counter.
struct BatchPull {
    remaining: std::collections::VecDeque<RemoteEndpoint>,
    raw_dest: String,
    done: usize,
    total: usize,
}

/// d-35: reply envelope from a spawned F3 pull task.
struct F3PullReply {
    request_id: u64,
    /// Ok((files_transferred, bytes_transferred, files_deleted))
    /// or a flattened error string. d-56: `files_deleted` is the
    /// mirror purge count (0 for a plain pull).
    result: Result<(usize, u64, u64), String>,
}

/// d-61: reply envelope from a spawned F1 push task.
struct F1PushReply {
    request_id: u64,
    /// Ok((files_transferred, bytes_transferred)) or a flattened
    /// error string.
    result: Result<(u64, u64), String>,
}

/// d-41: reply envelope from a spawned F3 du task.
struct F3DuReply {
    request_id: u64,
    /// Ok((bytes, files)) for the subtree root, or a
    /// flattened error string.
    result: Result<(u64, u64), String>,
}

/// d-45: reply envelope from a spawned F3 delete task.
struct F3DelReply {
    request_id: u64,
    /// Ok(files_deleted) or a flattened error string (e.g. the
    /// daemon's read-only-module rejection).
    result: Result<u64, String>,
}

/// d-45: spawn a `Purge` for an F3 cursor row and post the
/// outcome back on `tx`. The module-root guard runs in the
/// dispatcher (before the prompt opens), so by here the path is
/// known-deletable; the daemon still enforces read-only modules
/// and containment, surfacing as `Err`.
fn spawn_f3_del(
    request_id: u64,
    module_endpoint: RemoteEndpoint,
    rel_paths: Vec<String>,
    tx: mpsc::Sender<F3DelReply>,
) {
    tokio::spawn(async move {
        let result = run_f3_del(&module_endpoint, rel_paths)
            .await
            .map_err(|err| format!("{err:#}"));
        let _ = tx.send(F3DelReply { request_id, result }).await;
    });
}

/// d-45 / d-50: issue one `Purge` deleting `rel_paths` under
/// `module_endpoint`'s module. `rel_paths` are already canonical
/// forward-slash wire paths (built by `del_wire_path` at the
/// dispatch boundary). Returns the daemon's total files-deleted.
async fn run_f3_del(module_endpoint: &RemoteEndpoint, rel_paths: Vec<String>) -> eyre::Result<u64> {
    use blit_app::admin::rm;
    let (module, _) = rm::extract_module_and_path(module_endpoint)?;
    rm::purge(module_endpoint, module, rel_paths).await
}

/// d-45 R2: the module-relative Purge wire path for a cursor
/// endpoint — forward-slash joined regardless of client OS.
/// Thin wrapper over `rel_path_to_string` so the conversion
/// boundary is named + unit-testable.
fn del_wire_path(rel_path: &std::path::Path) -> String {
    blit_app::endpoints::rel_path_to_string(rel_path)
}

/// d-50: assemble a delete request from resolved cursor/marked
/// endpoints. Filters out non-deletable targets (module roots),
/// converts each to a canonical wire rel-path, and returns
/// `(module_endpoint, rel_paths, label, gate_path)` or `None`
/// when nothing is deletable.
///
/// - `batch` (a multi-select was active) → label "N item(s)",
///   `gate_path = None` (outcome shows until the next action).
/// - single cursor row → label is the path spec, `gate_path =
///   Some(spec)` (outcome hides once the cursor leaves it, the
///   d-45 behavior).
///
/// All targets share one module (they come from one F3 view), so
/// the first endpoint carries the module for the single `Purge`.
fn build_delete_request(
    endpoints: Vec<RemoteEndpoint>,
    batch: bool,
) -> Option<(RemoteEndpoint, Vec<String>, String, Option<String>)> {
    use blit_app::admin::rm;
    let deletable: Vec<RemoteEndpoint> = endpoints
        .into_iter()
        .filter(is_deletable_remote_path)
        .collect();
    let module_endpoint = deletable.first()?.clone();
    let mut rel_paths = Vec::with_capacity(deletable.len());
    for ep in &deletable {
        if let Ok((_module, rel_path)) = rm::extract_module_and_path(ep) {
            rel_paths.push(del_wire_path(&rel_path));
        }
    }
    if rel_paths.is_empty() {
        return None;
    }
    let (label, gate_path) = if batch {
        (format!("{} item(s)", rel_paths.len()), None)
    } else {
        let spec = module_endpoint.display();
        (spec.clone(), Some(spec))
    };
    Some((module_endpoint, rel_paths, label, gate_path))
}

/// d-45: may this cursor endpoint be deleted from the TUI?
///
/// Refuses a module root or empty rel-path — you can't nuke a
/// whole module via `D` (mirrors `blit rm`'s guard). Also refuses
/// `Discovery` (bare-host) endpoints, which carry no path.
/// Pure — the dispatcher gates the confirm prompt on this.
fn is_deletable_remote_path(endpoint: &RemoteEndpoint) -> bool {
    use blit_app::admin::rm;
    match rm::extract_module_and_path(endpoint) {
        Ok((_module, rel_path)) => {
            let rel = rel_path.to_string_lossy();
            !rel.is_empty() && rel != "."
        }
        Err(_) => false,
    }
}

/// d-41: spawn a `DiskUsage` query for an F3 cursor row and
/// post the subtree total back on `tx`. `max_depth = 0` makes
/// the daemon stream a single aggregate entry for `remote`'s
/// path; we keep the largest-byte entry defensively (the root
/// dominates any child the daemon might also emit). The reply
/// is generation-stamped with `request_id` so the loop drops it
/// if a newer `u` superseded this one.
fn spawn_f3_du(request_id: u64, remote: RemoteEndpoint, tx: mpsc::Sender<F3DuReply>) {
    tokio::spawn(async move {
        let result = run_f3_du_total(&remote)
            .await
            .map_err(|err| format!("{err:#}"));
        let _ = tx.send(F3DuReply { request_id, result }).await;
    });
}

/// d-41 round 2: depth requested for the F3 du query.
///
/// **Must be ≥ 1.** The daemon maps a request `max_depth == 0`
/// to `None` = *unbounded* — it would then stream one row per
/// descendant path (the full `blit du` response shape) for a
/// hotkey that renders a single Stats line. `1` bounds the
/// stream to the root + its immediate children. The root entry
/// (depth 0) always accumulates every descendant's bytes
/// regardless of the depth cap, so the subtree total is still
/// complete; the cap only limits how many *rows* are streamed
/// back. Round 1 used `0` and leaned on client-side max-byte
/// folding to discard the flood — correct totals, but it pulled
/// the entire descendant stream over gRPC (reviewer reopen).
const F3_DU_MAX_DEPTH: u32 = 1;

// d-41 R2 guard (compile-time): depth 0 means UNBOUNDED in the
// daemon — F3 du would stream the full descendant tree for a
// one-line aggregate. This fails the build if anyone reverts it.
const _: () = assert!(
    F3_DU_MAX_DEPTH >= 1,
    "F3_DU_MAX_DEPTH must be >= 1; 0 is unbounded in the daemon"
);

/// d-41: stream the du aggregate for `remote` and return its
/// `(bytes, files)` subtree total. Split out from the spawn so
/// the accumulation (keep the max-byte entry) is unit-testable
/// without a live daemon — see `du_total_from_entries`.
async fn run_f3_du_total(remote: &RemoteEndpoint) -> eyre::Result<(u64, u64)> {
    use blit_app::admin::du;
    use blit_app::endpoints::{module_and_rel_path, rel_path_to_string};

    let (module, rel_path) = module_and_rel_path(remote)?;
    let start_path = rel_path_to_string(&rel_path);
    let mut acc: Option<(u64, u64)> = None;
    du::stream(remote, module, start_path, F3_DU_MAX_DEPTH, |entry| {
        acc = du_total_from_entries(acc, entry.bytes, entry.files);
        Ok(())
    })
    .await?;
    acc.ok_or_else(|| eyre::eyre!("no disk-usage data returned"))
}

/// d-41: fold one du entry into the running aggregate, keeping
/// the entry with the most bytes. At [`F3_DU_MAX_DEPTH`] = 1 the
/// daemon emits the root plus immediate children; the root
/// subtree contains every child, so the max-byte entry is always
/// the root total we want.
fn du_total_from_entries(acc: Option<(u64, u64)>, bytes: u64, files: u64) -> Option<(u64, u64)> {
    match acc {
        Some((best_bytes, _)) if best_bytes >= bytes => acc,
        _ => Some((bytes, files)),
    }
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

/// d-63: live progress snapshot from a running F1 push, forwarded
/// to the event loop. `bytes_per_sec` is the average throughput
/// (0 until ~1s elapsed), reusing [`pull_throughput`].
struct F1PushProgress {
    request_id: u64,
    files: u64,
    bytes: u64,
    bytes_per_sec: u64,
}

/// d-63: fold one push `ProgressEvent` into the running
/// `(files, bytes)` totals using push-SEND semantics.
///
/// Unlike the pull (receive) path, the push send path reports
/// bytes on `FileComplete` (`data_plane.rs` `send_payloads`:
/// `report_file_complete(path, header.size)`) and emits NO
/// `Payload` events — so bytes AND files both come from
/// `FileComplete` here (whereas `accumulate_pull_progress` takes
/// bytes from `Payload` to avoid the receive path's
/// `Payload`+`FileComplete` double-count). Counting bytes from
/// `Payload` here would report 0; counting `FileComplete` bytes is
/// correct and never double-counts because push emits no
/// `Payload`.
fn accumulate_push_progress(
    files: &mut u64,
    bytes: &mut u64,
    event: &blit_core::remote::transfer::ProgressEvent,
) {
    use blit_core::remote::transfer::ProgressEvent;
    match event {
        ProgressEvent::FileComplete { bytes: b, .. } => {
            *files = files.saturating_add(1);
            *bytes = bytes.saturating_add(*b);
        }
        // Push send emits no Payload events; ignore defensively so
        // a future emitter change can't double-count.
        ProgressEvent::Payload { .. } | ProgressEvent::ManifestBatch { .. } => {}
    }
}

/// d-69: fold one delegated-pull `ProgressEvent` into the running
/// `(files, bytes)` totals. The delegated path
/// (`remote::report_bytes_progress`) reports cumulative deltas via
/// `report_payload(file_delta, byte_delta)` — so a `Payload` carries
/// BOTH the file and byte deltas (unlike the receive path, where
/// `Payload.files` is unused and files come from `FileComplete`, and
/// unlike push, where bytes ride `FileComplete`). It emits no
/// `FileComplete`, so take both fields from `Payload` here.
fn accumulate_delegated_progress(
    files: &mut u64,
    bytes: &mut u64,
    event: &blit_core::remote::transfer::ProgressEvent,
) {
    use blit_core::remote::transfer::ProgressEvent;
    match event {
        ProgressEvent::Payload { files: f, bytes: b } => {
            *files = files.saturating_add(*f as u64);
            *bytes = bytes.saturating_add(*b);
        }
        ProgressEvent::FileComplete { .. } | ProgressEvent::ManifestBatch { .. } => {}
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

/// d-55 R2 / d-57: build the `PullSyncOptions` for an F3 pull.
///
/// `mirror_mode` MUST live here, on the options — the wire
/// `TransferOperationSpec` is built from `options`
/// (`RemotePullClient::build_spec_from_options`), so it's
/// `options.mirror_mode` that tells the daemon to compute the
/// delete list. The execution-level `PullSyncExecution.mirror_mode`
/// is only the receive-side `track_paths` flag; setting it alone
/// (d-55 round 1) left the daemon emitting `MirrorMode::Off`, so
/// `apply_pull_mirror_purge` had no paths to delete and the
/// "mirror" silently behaved like a plain pull. The CLI sets the
/// options field (`blit-cli/src/transfers/remote.rs`); we match it.
///
/// d-57: a `Move` sets `require_complete_scan` so the daemon
/// refuses a partial source scan — mirroring the CLI's move guard
/// (`run_remote_pull_transfer_deferred(.., true)`). Deleting the
/// remote source after an incomplete copy would lose the files
/// that were skipped.
fn f3_pull_options(kind: f3pull::PullKind) -> blit_core::remote::pull::PullSyncOptions {
    use f3pull::PullKind;
    blit_core::remote::pull::PullSyncOptions {
        mirror_mode: kind == PullKind::Mirror,
        require_complete_scan: kind == PullKind::Move,
        ..blit_core::remote::pull::PullSyncOptions::default()
    }
}

fn spawn_f3_pull(
    request_id: u64,
    source: RemoteEndpoint,
    dest_root: std::path::PathBuf,
    // d-55/d-57: copy / mirror / move. Mirror runs the post-pull
    // `apply_pull_mirror_purge`; move deletes the remote source.
    kind: f3pull::PullKind,
    tx: mpsc::Sender<F3PullReply>,
    // d-37: live byte/file progress snapshots. `try_send`
    // means a full channel just drops an intermediate
    // update — the authoritative final count rides the
    // `F3PullReply`.
    progress_tx: mpsc::Sender<F3PullProgress>,
) {
    use blit_app::admin::rm::delete_remote_path;
    use blit_app::transfers::remote::{apply_pull_mirror_purge, run_pull_sync, PullSyncExecution};
    use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
    use f3pull::PullKind;
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

        // d-57: a move needs the source endpoint again after the
        // receive (to delete it), but `source` moves into the
        // execution — clone it up front for the move path only.
        let move_source = (kind == PullKind::Move).then(|| source.clone());
        let remote_label = source.display();
        let execution = PullSyncExecution {
            remote: source,
            dest_root,
            options: f3_pull_options(kind),
            compute_checksums: false,
            // receive-side track_paths — only meaningful for mirror.
            mirror_mode: kind == PullKind::Mirror,
            remote_label,
        };
        // d-55: run_pull_sync does the receive half; the
        // destructive step (mirror purge / move source-delete) is
        // run AFTER the progress monitor is torn down. So:
        // pull → drop progress → drain forwarder → destructive phase.
        let sync = run_pull_sync(execution, Some(&progress)).await;
        // Close the progress channel → the forwarder drains and
        // exits before we run the destructive phase / send the reply.
        drop(progress);
        let _ = forwarder.await;
        let result = match sync {
            Ok(outcome) => {
                let transferred = (
                    outcome.report.files_transferred,
                    outcome.report.bytes_transferred,
                );
                match kind {
                    // Plain pull — nothing to remove.
                    PullKind::Copy => Ok((transferred.0, transferred.1, 0)),
                    // d-55/d-56: delete local files absent from the
                    // source; surface the purge count.
                    PullKind::Mirror => match apply_pull_mirror_purge(&outcome, true).await {
                        Ok(stats) => {
                            let deleted = stats.map(|s| s.files_deleted).unwrap_or(0);
                            Ok((transferred.0, transferred.1, deleted))
                        }
                        Err(err) => Err(format!("{err:#}")),
                    },
                    // d-57: delete the remote source only after a
                    // successful receive. The `require_complete_scan`
                    // option made the daemon refuse a partial scan, so
                    // a successful pull means the whole source was
                    // copied — safe to remove it now. A delete failure
                    // surfaces as the op's error (the copy already
                    // succeeded, but the operator must know the source
                    // wasn't removed).
                    PullKind::Move => {
                        let source = move_source.expect("move_source set for Move");
                        match blit_app::admin::rm::extract_module_and_path(&source) {
                            Ok((_, rel_path)) => {
                                let wire = del_wire_path(&rel_path);
                                match delete_remote_path(&source, &wire).await {
                                    Ok(removed) => Ok((transferred.0, transferred.1, removed)),
                                    Err(err) => Err(format!(
                                        "received but failed to delete remote source: {err:#}"
                                    )),
                                }
                            }
                            Err(err) => Err(format!(
                                "received but cannot resolve remote source to delete: {err:#}"
                            )),
                        }
                    }
                }
            }
            Err(err) => Err(format!("{err:#}")),
        };
        let _ = tx.send(F3PullReply { request_id, result }).await;
    });
}

/// d-61: spawn a local→remote COPY push for an F1 trigger.
/// d-65 R2: build the `PushExecution` for an F1 trigger push.
/// Extracted from `spawn_f1_push` so the mirror-safety options are
/// unit-pinnable (the reviewer flagged the inline construction as
/// untested). Mirror sets `mirror_mode` + `MirrorMode::All` — the
/// daemon deletes destination entries absent from the source — AND
/// `require_complete_scan`, so a partial local enumeration can never
/// drive that purge (an under-scanned source would otherwise make
/// valid remote files look extraneous). This matches the CLI's
/// `require_complete_scan: mirror_mode` in
/// `crates/blit-cli/src/transfers/remote.rs`. Copy/move push never
/// delete at the dest, so they leave both off — an incomplete scan
/// there only under-copies, which is safe and retryable.
fn build_f1_push_execution(
    local_source: std::path::PathBuf,
    remote: RemoteEndpoint,
    kind: f3pull::PullKind,
) -> blit_app::transfers::remote::PushExecution {
    use blit_app::endpoints::Endpoint;
    use blit_app::transfers::remote::PushExecution;
    use blit_core::fs_enum::FileFilter;
    use blit_core::generated::MirrorMode;
    let mirror = kind == f3pull::PullKind::Mirror;
    let remote_label = remote.display();
    PushExecution {
        source: Endpoint::Local(local_source),
        remote,
        filter: FileFilter::default(),
        mirror_mode: mirror,
        mirror_kind: if mirror {
            MirrorMode::All
        } else {
            MirrorMode::Off
        },
        force_grpc: false,
        trace_data_plane: false,
        require_complete_scan: mirror,
        remote_label,
    }
}

/// Runs `run_remote_push` on a task and flattens the outcome into
/// an [`F1PushReply`] (generation-guarded by `request_id`). d-63:
/// a progress forwarder accumulates push-send `ProgressEvent`s
/// into live `(files, bytes)` snapshots on `progress_tx` (lossy
/// via `try_send`); the authoritative totals ride the reply.
fn spawn_f1_push(
    request_id: u64,
    local_source: std::path::PathBuf,
    remote: RemoteEndpoint,
    // d-65: copy / mirror / move. Mirror sets `mirror_mode` (the
    // daemon deletes extraneous files at the dest); move deletes
    // the LOCAL source after a successful push.
    kind: f3pull::PullKind,
    tx: mpsc::Sender<F1PushReply>,
    progress_tx: mpsc::Sender<F1PushProgress>,
) {
    use blit_app::transfers::remote::run_remote_push;
    use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
    use f3pull::PullKind;
    tokio::spawn(async move {
        // d-63: progress monitor — run_remote_push reports
        // ProgressEvents into `pe_rx`; the forwarder accumulates
        // cumulative (files, bytes) via `accumulate_push_progress`
        // and ships snapshots to the UI.
        let (pe_tx, mut pe_rx) = mpsc::unbounded_channel::<ProgressEvent>();
        let progress = RemoteTransferProgress::new(pe_tx);
        let forwarder = tokio::spawn(async move {
            let started = Instant::now();
            let mut files = 0u64;
            let mut bytes = 0u64;
            while let Some(event) = pe_rx.recv().await {
                accumulate_push_progress(&mut files, &mut bytes, &event);
                let bytes_per_sec = pull_throughput(bytes, started.elapsed().as_secs_f64());
                let _ = progress_tx.try_send(F1PushProgress {
                    request_id,
                    files,
                    bytes,
                    bytes_per_sec,
                });
            }
        });

        // d-65: a move deletes the local source after a successful
        // push — keep a copy of the path before it moves into the
        // execution (only the Move path needs it).
        let move_source = (kind == PullKind::Move).then(|| local_source.clone());
        let execution = build_f1_push_execution(local_source, remote, kind);
        let sent = run_remote_push(execution, Some(&progress)).await;
        // Close the progress channel → the forwarder drains and
        // exits before the (possibly long) move source-delete and
        // the terminal reply.
        drop(progress);
        let _ = forwarder.await;
        let result = match sent {
            Ok(outcome) => {
                let transferred = (
                    outcome.report.summary.files_transferred,
                    outcome.report.summary.bytes_transferred,
                );
                match move_source {
                    // d-65: move — delete the local source only after
                    // a successful push. A delete failure surfaces as
                    // the op's error (the push already landed, but the
                    // operator must know the source wasn't removed).
                    Some(src) => match remove_local_source(&src) {
                        Ok(()) => Ok(transferred),
                        Err(err) => {
                            Err(format!("pushed but failed to delete local source: {err:#}"))
                        }
                    },
                    None => Ok(transferred),
                }
            }
            Err(err) => Err(format!("{err:#}")),
        };
        let _ = tx.send(F1PushReply { request_id, result }).await;
    });
}

/// d-65: remove a local move source after its push landed. A dir
/// is removed recursively, a file directly. Mirrors the CLI's
/// local-source-delete step in `run_move`.
fn remove_local_source(path: &std::path::Path) -> std::io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// d-70: build the `DelegatedPullExecution` for an F1 remote→remote
/// transfer. Extracted from `spawn_f1_delegated_pull` so the
/// mirror option is unit-pinnable (cf. the d-65 push builder). The
/// options come from `f3_pull_options(kind)`: copy → no flags;
/// mirror → `mirror_mode` on, `require_complete_scan` OFF. The OFF is
/// deliberate and matches the CLI's delegated path
/// (`crates/blit-cli/src/transfers/mod.rs` passes
/// `require_complete_scan = false` for delegated copy/mirror) — in a
/// delegated transfer the *daemons* enumerate, not this client, so
/// the d-65 client-side partial-scan guard doesn't apply. (Move,
/// which the CLI scans-completely for, is rejected upstream.) Always
/// attached (`detach: false`); detached/F2-visible delegation is a
/// follow-up.
fn build_delegated_execution(
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    kind: f3pull::PullKind,
) -> blit_app::transfers::remote::DelegatedPullExecution {
    let dst_label = dst.display();
    blit_app::transfers::remote::DelegatedPullExecution {
        src,
        dst,
        options: f3_pull_options(kind),
        trace_data_plane: false,
        // The TUI doesn't surface a `--relay-via-cli` toggle yet, so
        // don't suggest it in transport-error hints.
        relay_fallback_suggestable: false,
        dst_label,
        detach: false,
    }
}

/// d-68: run a remote→remote *delegated* transfer on a task — the
/// destination daemon pulls from the source daemon (the CLI host is
/// not in the byte path) — and flatten the outcome into an
/// [`F1PushReply`] (generation-guarded by `request_id`, reusing the
/// F1 push footer). Attached (`detach: false`, matching the CLI
/// default). d-69: a progress forwarder turns the daemon's cumulative
/// `BytesProgress` (delivered as `Payload` deltas via
/// `report_bytes_progress`) into live `(files, bytes)` snapshots on
/// `progress_tx`; the authoritative totals still ride the terminal
/// reply. d-70: `kind` selects copy vs mirror via
/// [`build_delegated_execution`]. d-71: move runs a delegated copy
/// then deletes the remote SOURCE — `require_complete_scan` (set for
/// move by `f3_pull_options`) makes the daemon refuse a partial scan,
/// so a successful copy means the whole source was transferred before
/// the delete (mirrors the F3 remote→local move).
fn spawn_f1_delegated_pull(
    request_id: u64,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    kind: f3pull::PullKind,
    tx: mpsc::Sender<F1PushReply>,
    progress_tx: mpsc::Sender<F1PushProgress>,
) {
    use blit_app::admin::rm::{delete_remote_path, extract_module_and_path};
    use blit_app::transfers::remote::run_delegated_pull;
    use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
    tokio::spawn(async move {
        // d-69: progress monitor — run_delegated_pull reports Payload
        // deltas into `pe_rx`; the forwarder accumulates cumulative
        // (files, bytes) via `accumulate_delegated_progress` and ships
        // snapshots to the UI (lossy via try_send).
        let (pe_tx, mut pe_rx) = mpsc::unbounded_channel::<ProgressEvent>();
        let progress = RemoteTransferProgress::new(pe_tx);
        let forwarder = tokio::spawn(async move {
            let started = Instant::now();
            let mut files = 0u64;
            let mut bytes = 0u64;
            while let Some(event) = pe_rx.recv().await {
                accumulate_delegated_progress(&mut files, &mut bytes, &event);
                let bytes_per_sec = pull_throughput(bytes, started.elapsed().as_secs_f64());
                let _ = progress_tx.try_send(F1PushProgress {
                    request_id,
                    files,
                    bytes,
                    bytes_per_sec,
                });
            }
        });

        // d-71: a move needs the source endpoint again after the copy
        // (to delete it), but it moves into the execution — clone it
        // up front for the move path only.
        let move_source = (kind == f3pull::PullKind::Move).then(|| src.clone());
        let execution = build_delegated_execution(src, dst, kind);
        let sent = run_delegated_pull(execution, Some(&progress), |_| {}).await;
        // Close the progress channel → the forwarder drains and exits
        // before the destructive phase / terminal reply.
        drop(progress);
        let _ = forwarder.await;
        let result = match sent {
            Ok(outcome) => {
                let transferred = (
                    outcome.summary.files_transferred,
                    outcome.summary.bytes_transferred,
                );
                match move_source {
                    // d-71: delete the remote source only after a
                    // successful delegated copy. A delete failure
                    // surfaces as the op's error (the copy already
                    // landed, but the operator must know the source
                    // wasn't removed).
                    Some(source) => match extract_module_and_path(&source) {
                        Ok((_, rel_path)) => {
                            let wire = del_wire_path(&rel_path);
                            match delete_remote_path(&source, &wire).await {
                                Ok(_) => Ok(transferred),
                                Err(err) => Err(format!(
                                    "delegated but failed to delete remote source: {err:#}"
                                )),
                            }
                        }
                        Err(err) => Err(format!(
                            "delegated but cannot resolve remote source to delete: {err:#}"
                        )),
                    },
                    None => Ok(transferred),
                }
            }
            Err(err) => Err(format!("{err:#}")),
        };
        let _ = tx.send(F1PushReply { request_id, result }).await;
    });
}

/// d-62/d-65: outcome of classifying an F1 trigger commit.
#[derive(Debug)]
enum TriggerOutcome {
    /// A transfer started — the caller closes the modal.
    Launched,
    /// A destructive PUSH (mirror/move local→remote) needs a y/N
    /// confirm before launching — the caller opens the confirm
    /// gate. (Pull-family mirror/move confirm on F3 instead, so
    /// they go straight to `Launched`.)
    NeedsConfirm,
    /// Invalid input — the caller keeps the modal open + shows the
    /// message.
    Rejected(String),
}

/// d-62: classify + launch an F1 trigger commit (or signal that a
/// destructive push needs confirmation).
///
/// Direction is decided by the strict transfer parser (d-61):
/// remote source → remote→local (pull family, d-58…d-60); local
/// source → local→remote push (copy/mirror/move, d-61/d-65). Both
/// endpoints are validated up front so a typo'd or unsupported
/// endpoint never silently starts — or silently drops — a
/// transfer. d-65: a mirror/move PUSH is destructive (deletes at
/// the remote dest / deletes the local source), so it returns
/// `NeedsConfirm` unless `confirmed`; the caller's confirm handler
/// re-calls with `confirmed = true`.
fn plan_f1_trigger(
    app: &mut AppState,
    src: &str,
    dest: &str,
    kind: f3pull::PullKind,
    confirmed: bool,
) -> TriggerOutcome {
    use blit_app::endpoints::{
        ensure_remote_destination_supported, parse_transfer_endpoint, Endpoint,
    };
    use blit_app::transfers::resolution::resolve_destination;
    let source = match parse_transfer_endpoint(src) {
        Ok(s) => s,
        Err(_) => return TriggerOutcome::Rejected(format!("invalid source: {src}")),
    };
    // d-68: with a remote source, classify the destination up front.
    // It's a remote→remote delegated transfer ONLY when the dest is a
    // genuine remote module/root (`host:/module/…` or `host://…`);
    // everything else stays on the remote→local pull path, where the
    // dest is a local directory. Three buckets:
    //   - Remote module/root  → delegated.
    //   - Remote *discovery*  → bare `host` / `host:port` parses as a
    //     discovery endpoint, but here it's an ordinary relative local
    //     dest (e.g. `backup`) for the pull — fall through.
    //   - genuine local dest  → fall through.
    //   - parse Err           → a remote-*shaped* typo (e.g.
    //     `host:/module` missing its trailing slash); reject rather
    //     than mis-route into the pull as a literal local path.
    // d-68 R2 added the Err→reject arm; R3 narrows delegation to
    // supported remote dests so discovery doesn't steal relative
    // local pull destinations.
    if let Endpoint::Remote(ref source_ep) = source {
        match parse_transfer_endpoint(dest) {
            Ok(Endpoint::Remote(dst_ep))
                if ensure_remote_destination_supported(&dst_ep).is_ok() =>
            {
                // d-71 R2: resolve the destination exactly like the CLI
                // (`run_copy`/`run_move` call `resolve_destination`
                // before dispatch) BEFORE delegating. Without it, a
                // non-trailing-slash source + a container dest writes
                // into the dest root instead of `<dest>/<basename>` —
                // and for a move the source-delete then removes data
                // that was copied to the wrong place. `resolve_destination`
                // is a no-op for trailing-slash ("copy contents")
                // sources, so it doesn't change copy/mirror behavior for
                // those. It preserves the Remote variant for a remote
                // dst, so the rebind is infallible.
                let resolved = resolve_destination(src, dest, &source, Endpoint::Remote(dst_ep));
                let Endpoint::Remote(resolved_dst) = resolved else {
                    return TriggerOutcome::Rejected(format!("invalid destination: {dest}"));
                };
                return plan_f1_delegated(app, source_ep.clone(), resolved_dst, kind, confirmed);
            }
            // Discovery (bare host) or genuine local path → pull below.
            Ok(_) => {}
            Err(_) => return TriggerOutcome::Rejected(format!("invalid destination: {dest}")),
        }
    }
    match source {
        // Remote source → remote→local (pull family).
        Endpoint::Remote(source) => match kind {
            f3pull::PullKind::Copy => {
                let Some(launch) = app.f3_pull.start_pull(source, dest.to_string()) else {
                    return TriggerOutcome::Rejected("a transfer is already in flight".into());
                };
                spawn_f3_pull(
                    launch.request_id,
                    launch.source,
                    launch.dest_root,
                    launch.kind,
                    app.f3_pull_reply_tx.clone(),
                    app.f3_pull_progress_tx.clone(),
                );
                app.current_screen = Screen::F3;
                TriggerOutcome::Launched
            }
            // Mirror / move route through the F3 destructive confirm
            // gate (so they don't need the trigger's own confirm).
            // Move deletes the remote source, so refuse a module-root
            // source up front (d-60).
            f3pull::PullKind::Mirror | f3pull::PullKind::Move => {
                if kind == f3pull::PullKind::Move && !is_deletable_remote_path(&source) {
                    return TriggerOutcome::Rejected("cannot move a module root".into());
                }
                if kind == f3pull::PullKind::Mirror {
                    app.f3_pull.begin_mirror(source);
                } else {
                    app.f3_pull.begin_move(source);
                }
                for c in dest.chars() {
                    app.f3_pull.push_char(c);
                }
                let _ = app.f3_pull.begin_run();
                if app.f3_pull.is_confirming_destructive() {
                    app.current_screen = Screen::F3;
                    TriggerOutcome::Launched
                } else {
                    TriggerOutcome::Rejected("a transfer is already in flight".into())
                }
            }
        },
        // Local source → local→remote push (copy / mirror / move).
        Endpoint::Local(local_src) => {
            let remote = match parse_transfer_endpoint(dest) {
                Ok(Endpoint::Remote(r)) => r,
                Ok(Endpoint::Local(_)) => {
                    return TriggerOutcome::Rejected(
                        "push destination must be remote (host:/module/)".into(),
                    )
                }
                Err(_) => return TriggerOutcome::Rejected(format!("invalid destination: {dest}")),
            };
            if ensure_remote_destination_supported(&remote).is_err() {
                return TriggerOutcome::Rejected(
                    "destination needs a module (host:/module/)".into(),
                );
            }
            // d-72: resolve the destination like the CLI (`run_copy` /
            // `run_move` call `resolve_destination` before dispatch),
            // matching the delegated path (d-71 R2) and the F4
            // local-transfer path. A non-trailing-slash local source +
            // a container remote dest must nest under
            // `<dest>/<basename>`, not write into the dest root — for a
            // push MOVE (deletes the local source) writing to the wrong
            // remote target and then removing the source is data loss.
            // No-op for trailing-slash ("copy contents") sources;
            // preserves the Remote variant, so the rebind is infallible.
            let remote = match resolve_destination(
                src,
                dest,
                &Endpoint::Local(local_src.clone()),
                Endpoint::Remote(remote),
            ) {
                Endpoint::Remote(r) => r,
                Endpoint::Local(_) => {
                    return TriggerOutcome::Rejected(format!("invalid destination: {dest}"))
                }
            };
            // d-65: mirror (delete-extraneous at the remote) and move
            // (delete the local source) are destructive — gate them
            // behind a confirm. Copy launches immediately.
            if kind.is_destructive() && !confirmed {
                return TriggerOutcome::NeedsConfirm;
            }
            let label = remote.display();
            let Some(request_id) = app.f1_push.begin(label, kind) else {
                return TriggerOutcome::Rejected("a push is already running".into());
            };
            spawn_f1_push(
                request_id,
                local_src,
                remote,
                kind,
                app.f1_push_reply_tx.clone(),
                app.f1_push_progress_tx.clone(),
            );
            TriggerOutcome::Launched
        }
    }
}

/// d-68/d-70/d-71: classify + launch a remote→remote delegated
/// transfer from the F1 trigger. Copy delegates immediately; mirror
/// (purges the dest) and move (deletes the remote source after the
/// copy) are destructive, so they route through the trigger's y/N
/// confirm — the same gate the local→remote push mirror/move uses.
/// Move also refuses a module-root source up front. The destination
/// must resolve to a module (`host:/module/`).
fn plan_f1_delegated(
    app: &mut AppState,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    kind: f3pull::PullKind,
    confirmed: bool,
) -> TriggerOutcome {
    use blit_app::endpoints::ensure_remote_destination_supported;
    if ensure_remote_destination_supported(&dst).is_err() {
        return TriggerOutcome::Rejected("destination needs a module (host:/module/)".into());
    }
    // d-71: move deletes the remote SOURCE after the copy, so refuse a
    // module-root source up front — there's no single path to remove
    // (same guard as the F3 remote→local move, d-60).
    if kind == f3pull::PullKind::Move && !is_deletable_remote_path(&src) {
        return TriggerOutcome::Rejected("cannot move a module root".into());
    }
    // d-70/d-71: mirror purges the destination, move deletes the
    // remote source — both destructive, so gate behind the trigger's
    // y/N confirm (copy launches straight away).
    if kind.is_destructive() && !confirmed {
        return TriggerOutcome::NeedsConfirm;
    }
    let label = dst.display();
    let Some(request_id) = app.f1_push.begin_delegated(label, kind) else {
        return TriggerOutcome::Rejected("a push is already running".into());
    };
    spawn_f1_delegated_pull(
        request_id,
        src,
        dst,
        kind,
        app.f1_push_reply_tx.clone(),
        app.f1_push_progress_tx.clone(),
    );
    TriggerOutcome::Launched
}

/// d-53 R2: does this batch destination need a trailing slash to
/// force container semantics? A non-empty dest that doesn't
/// already end in `/` does — without it, `resolve_destination`
/// treats a non-existing path as an exact target, so multiple
/// batch sources would collide on the same path instead of
/// nesting under it. Blank / already-slashed dests need nothing.
fn needs_container_slash(dest: &str) -> bool {
    let trimmed = dest.trim_end();
    !trimmed.is_empty() && !trimmed.ends_with('/')
}

/// d-53: drive the sequential batch pull forward after one
/// source's pull completed. Bumps the done count, then starts
/// the next queued source (with the captured dest) and spawns
/// its task — or clears the batch when the queue is empty.
fn advance_batch_pull(app: &mut AppState) {
    // Scope the `&mut app.f3_batch_pull` borrow so it ends before
    // we touch `app.f3_pull` / spawn below.
    let next = {
        let Some(batch) = app.f3_batch_pull.as_mut() else {
            return;
        };
        batch.done += 1;
        batch
            .remaining
            .pop_front()
            .map(|src| (src, batch.raw_dest.clone()))
    };
    match next {
        Some((source, raw_dest)) => {
            if let Some(launch) = app.f3_pull.start_pull(source, raw_dest) {
                spawn_f3_pull(
                    launch.request_id,
                    launch.source,
                    launch.dest_root,
                    launch.kind,
                    app.f3_pull_reply_tx.clone(),
                    app.f3_pull_progress_tx.clone(),
                );
            } else {
                // Blank dest or unexpectedly busy — stop the batch
                // rather than spin.
                app.f3_batch_pull = None;
            }
        }
        None => app.f3_batch_pull = None,
    }
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

/// d-40 round 2: the shorter of two optional deadlines.
///
/// The loop has more than one auto-hide fragment that can
/// pull the sleep budget below `live_tick.interval_ms`: the
/// F2 cancel status (d-24) and the F3 pull outcome (d-40).
/// They live on different screens so at most one is `Some`
/// in practice, but merging generically keeps the budget
/// correct if that ever changes — the loop must wake for
/// whichever deadline is nearer.
fn min_opt(
    a: Option<std::time::Duration>,
    b: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, b) => b,
    }
}

/// d-30 R2: local container for "what the operator
/// confirmed" — moved out of `cancel_status` so the
/// mutable borrow on `app.cancel_status` doesn't live
/// across the spawn. Single carries one id; Batch
/// carries the frozen list captured at prompt creation.
enum ConfirmedCancel {
    /// m2f-7: single cancel carries the id + the daemon that owns it
    /// (CancelJob targets that daemon).
    Single { id: String, daemon: String },
    /// m2f-8: `(daemon, transfer_id)` per active row.
    Batch(Vec<(String, String)>),
}

/// m2f-7: turn an F2 row's source-daemon identity (`host_port_display`)
/// back into a connectable endpoint for CancelJob. The identity has no
/// module path — only the control plane (host:port) matters for
/// CancelJob — so `RemoteEndpoint::parse` of `host` / `host:port`
/// yields a usable Discovery endpoint. `None` on a malformed identity.
fn cancel_endpoint(daemon: &str) -> Option<RemoteEndpoint> {
    RemoteEndpoint::parse(daemon).ok()
}

/// d-30 / d-30 R2: snapshot the current active-transfer
/// ids in a stable order. Called once when the operator
/// presses `Shift+X` — the result is then either stored
/// on `ConfirmingBatch` (confirm path) or fed directly
/// into `spawn_cancels_for_ids` (non-confirm path).
/// m2f-8: snapshot `(daemon, transfer_id)` for every active row, so a
/// batch cancel sends each `CancelJob` to the daemon that owns the
/// transfer (F2 shows rows from many daemons). Frozen at prompt
/// creation (d-30 R2 race fix).
fn snapshot_active_targets(transfers: &state::TransfersState) -> Vec<(String, String)> {
    transfers
        .active_rows()
        .into_iter()
        .map(|row| (row.source_daemon.clone(), row.transfer_id.clone()))
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
/// m2f-8: spawn one CancelJob per `(daemon, id)` target, each against
/// the daemon that owns it (`cancel_endpoint`). A target whose daemon
/// identity is malformed is skipped. Returns the number spawned.
fn spawn_cancels_for_targets(
    targets: Vec<(String, String)>,
    cancel_request_seq: &mut u64,
    tx: &mpsc::Sender<CancelReply>,
) -> usize {
    let mut count = 0;
    for (daemon, id) in targets {
        let Some(endpoint) = cancel_endpoint(&daemon) else {
            continue;
        };
        *cancel_request_seq += 1;
        let rid = *cancel_request_seq;
        spawn_cancel_transfer(rid, endpoint, id, tx.clone());
        count += 1;
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

/// dark-1: build the base frame style from the optional `[theme]`
/// background / foreground colors. Returns `None` when BOTH are unset —
/// so the caller skips painting a base layer and the terminal's own
/// colors show through (the historical default). Pure, so the
/// bg/fg → style mapping is unit-testable.
fn base_theme_style(
    bg: Option<ratatui::style::Color>,
    fg: Option<ratatui::style::Color>,
) -> Option<ratatui::style::Style> {
    if bg.is_none() && fg.is_none() {
        return None;
    }
    let mut style = ratatui::style::Style::default();
    if let Some(bg) = bg {
        style = style.bg(bg);
    }
    if let Some(fg) = fg {
        style = style.fg(fg);
    }
    Some(style)
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
    // d-50 R2: tick while a batch delete outcome is showing so
    // it auto-hides on its TTL (single-row outcomes are
    // event-cleared, so they don't need ticking).
    if app.f3_del.is_batch_terminal() {
        return true;
    }
    // d-64: tick while an F1 push Done/Error fragment is showing
    // so it auto-hides on its TTL.
    if app.f1_push.is_terminal() {
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

/// d-65: handle a keystroke while the F1 destructive-push confirm
/// is open. Modal like the F3 destructive confirm: `y`/`Y`
/// launches (re-runs `plan_f1_trigger` with `confirmed = true`),
/// `n`/`N`/`Esc` aborts back to editing, and every other key is
/// swallowed. `?` / Ctrl-c / F-keys bubble.
fn handle_f1_trigger_confirm_keystroke(key: &KeyEvent, app: &mut AppState) -> bool {
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
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some((src, dest, kind)) = app.f1_trigger.peek() {
                match plan_f1_trigger(app, &src, &dest, kind, true) {
                    TriggerOutcome::Launched => app.f1_trigger.close(),
                    // A push that became busy between confirm and y:
                    // drop back to editing with the reason.
                    TriggerOutcome::Rejected(msg) => app.f1_trigger.set_error(msg),
                    // Already past the confirm gate; can't recur.
                    TriggerOutcome::NeedsConfirm => {}
                }
            }
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.f1_trigger.cancel_confirm();
            true
        }
        // Modal: swallow everything else.
        _ => true,
    }
}

/// d-66: handle a keystroke while F4's destructive
/// clear-history confirm is armed. Returns `true` when the
/// key was consumed. `y` runs the clear (and re-fetches on
/// success, mirroring the pre-d-66 inline handler); `n`/`Esc`
/// cancels. Ctrl-c (quit), F-keys (pane nav), and `?` (help)
/// fall through so the operator is never trapped in the modal.
fn handle_profile_clear_confirm_keystroke(key: &KeyEvent, app: &mut AppState) -> bool {
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
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.profile.cancel_clear_confirm();
            // Only re-fetch on success — otherwise begin_fetch's
            // Pending → Loaded sequence would wipe the error banner.
            let outcome = apply_profile_clear();
            if apply_lifecycle_outcome(&mut app.profile, outcome) {
                let id = app.profile.begin_fetch();
                spawn_profile_fetch(id, app.profile_reply_tx.clone());
            }
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.profile.cancel_clear_confirm();
            true
        }
        // Modal: swallow everything else.
        _ => true,
    }
}

/// d-26: F3 filter-mode keystroke router. Mirrors the
/// `handle_verify_keystroke` shape — returns `true` when
/// the key was absorbed (so the outer dispatcher skips
/// d-58: handle a keystroke while the F1 trigger-transfer modal
/// is open. Returns `true` if consumed. `Tab` toggles the focused
/// field, chars / Backspace edit it, `Esc` cancels. `Enter`
/// commits: the source string is parsed to a `RemoteEndpoint` and
/// — when valid — handed to the verified F3 pull machine
/// (`start_pull`, a remote→local copy) and the view jumps to F3 so
/// the operator watches the pull in its existing footer. `?`
/// (help), Ctrl-c (quit), and F-keys (pane nav) fall through.
fn handle_f1_trigger_keystroke(key: &KeyEvent, app: &mut AppState) -> bool {
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
            app.f1_trigger.cancel();
            true
        }
        KeyCode::Tab => {
            app.f1_trigger.toggle_focus();
            true
        }
        KeyCode::Up => {
            // d-59/d-60: cycle copy → mirror → move.
            app.f1_trigger.cycle_kind(true);
            true
        }
        KeyCode::Down => {
            app.f1_trigger.cycle_kind(false);
            true
        }
        KeyCode::Enter => {
            // d-62: `peek` reads the trimmed fields without closing.
            // `None` = a blank field → stay open silently. Otherwise
            // validate + launch; on a validation failure record an
            // inline error and keep the modal open. d-65: a
            // destructive push (mirror/move) opens a confirm gate
            // instead of launching.
            if let Some((src, dest, kind)) = app.f1_trigger.peek() {
                match plan_f1_trigger(app, &src, &dest, kind, false) {
                    TriggerOutcome::Launched => app.f1_trigger.close(),
                    TriggerOutcome::NeedsConfirm => app.f1_trigger.begin_confirm(),
                    TriggerOutcome::Rejected(msg) => app.f1_trigger.set_error(msg),
                }
            }
            true
        }
        KeyCode::Backspace => {
            app.f1_trigger.pop_char();
            true
        }
        KeyCode::Char(c) => {
            if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                return false;
            }
            app.f1_trigger.push_char(c);
            true
        }
        _ => false,
    }
}

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
            // d-53: Esc on the prompt aborts the whole batch
            // (nothing has run yet — the dest was never
            // confirmed).
            app.f3_batch_pull = None;
            true
        }
        KeyCode::Enter => {
            // d-53 R2: for a batch, the destination MUST be a
            // container — multiple sources resolved against a
            // non-existing slash-less path would each become the
            // same exact target (overwrite/collision). Force a
            // trailing slash so `resolve_destination` nests every
            // source under `<dest>/<basename>`. (Single `p` is
            // untouched — only the batch path normalizes.) The
            // basenames are unique within one F3 view, so the
            // nested targets never collide.
            if app.f3_batch_pull.is_some()
                && needs_container_slash(app.f3_pull.entering_dest().unwrap_or(""))
            {
                app.f3_pull.push_char('/');
            }
            // d-53: capture the (now container-normalized) dest
            // once for the batch before begin_run consumes it, so
            // queued sources reuse it.
            if let Some(dest) = app.f3_pull.entering_dest().map(str::to_string) {
                if let Some(batch) = app.f3_batch_pull.as_mut() {
                    batch.raw_dest = dest;
                }
            }
            if let Some(launch) = app.f3_pull.begin_run() {
                spawn_f3_pull(
                    launch.request_id,
                    launch.source,
                    launch.dest_root,
                    launch.kind,
                    app.f3_pull_reply_tx.clone(),
                    app.f3_pull_progress_tx.clone(),
                );
            } else {
                // Empty dest → begin_run is a no-op and the
                // prompt stays open; the batch dest we just
                // captured is blank, which is fine (re-captured
                // on the next Enter).
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

/// d-45: handle a keystroke while the F3 delete confirm prompt is
/// open. Returns `true` if consumed. This is a **modal** confirm
/// (delete is destructive): `y`/`Y` fires the Purge, `n`/`N`/`Esc`
/// aborts, and every other key is swallowed so a stray `p`/`u`/`/`
/// can't stack another prompt or move the cursor mid-confirm. The
/// only escapes are `?` (help), Ctrl-c (emergency quit), and
/// F-keys (pane navigation), which fall through to the dispatcher.
fn handle_f3_delete_keystroke(key: &KeyEvent, app: &mut AppState) -> bool {
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
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(launch) = app.f3_del.confirm() {
                spawn_f3_del(
                    launch.request_id,
                    launch.module_endpoint,
                    launch.rel_paths,
                    app.f3_del_reply_tx.clone(),
                );
            }
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.f3_del.cancel();
            true
        }
        // Modal: swallow everything else (no accidental nav or
        // prompt-stacking during a destructive confirm).
        _ => true,
    }
}

/// d-55/d-57: handle a keystroke while the F3 destructive confirm
/// (mirror `m` / move `v`) prompt is open. Returns `true` if
/// consumed. Modal like the delete confirm: `y`/`Y` launches the
/// op, `n`/`N`/`Esc` aborts, and every other key is swallowed so a
/// stray `p`/`m`/`v`/`/` can't stack another prompt or move the
/// cursor mid-confirm. `?` (help), Ctrl-c (emergency quit), and
/// F-keys (pane nav) fall through to the dispatcher.
fn handle_f3_destructive_confirm_keystroke(key: &KeyEvent, app: &mut AppState) -> bool {
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
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(launch) = app.f3_pull.confirm_destructive() {
                spawn_f3_pull(
                    launch.request_id,
                    launch.source,
                    launch.dest_root,
                    launch.kind,
                    app.f3_pull_reply_tx.clone(),
                    app.f3_pull_progress_tx.clone(),
                );
            }
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.f3_pull.cancel_destructive();
            true
        }
        // Modal: swallow everything else (no accidental nav or
        // prompt-stacking during a destructive confirm).
        _ => true,
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
    /// d-42: jump the cursor to the first row (`g`).
    /// Honored by F1 and F3 list panes.
    SelectFirst,
    /// d-42: jump the cursor to the last row (`G`).
    /// Honored by F1 and F3 list panes.
    SelectLast,
    /// F3: descend into the cursor row (enter / →).
    Descend,
    /// d-58: F1 only. `t` opens the trigger-transfer modal for
    /// the selected daemon (source/dest entry → remote→local
    /// pull). No-op off F1 / on a local row.
    F1TriggerBegin,
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
    /// d-55: F3 only. `m` opens the mirror destination prompt
    /// for the cursor-selected remote path. Same prompt as `p`,
    /// but commit routes through a destructive confirm (a mirror
    /// deletes local files absent from the source). No-op if no
    /// remote / nothing selectable / an op already in flight.
    F3MirrorBegin,
    /// d-57: F3 only. `v` opens the move destination prompt for
    /// the cursor-selected remote path. Like mirror, but the
    /// destructive phase deletes the REMOTE source after a
    /// complete receive. Gated on read-only (deletes from the
    /// module). No-op if no remote / nothing selectable / an op
    /// already in flight.
    F3MoveBegin,
    /// d-41: F3 only. `u` (usage) runs a DiskUsage query
    /// for the cursor-selected remote path and shows the
    /// subtree byte/file total in the Stats block. No-op if
    /// no remote is configured or nothing is selectable.
    F3DuBegin,
    /// d-53: F3 only. `P` (Shift+p) pulls the marked set into a
    /// local destination, sequentially — the batch pair to `p`
    /// (pull cursor). No-op without marks or with a pull already
    /// in flight. Other panes ignore it.
    F3BatchPullBegin,
    /// d-49: F3 only. `space` toggles the multi-select mark on
    /// the cursor row (TUI_DESIGN §5.3). Foundation for batch
    /// transfer / delete; other panes ignore it.
    F3ToggleMark,
    /// d-51: F3 only. `a` toggle-marks every visible row (mark
    /// all, or clear all if already all-marked). Other panes
    /// ignore it.
    F3ToggleMarkAll,
    /// d-45: F3 only. `D` (Shift+d) opens a delete confirm
    /// prompt for the cursor-selected remote path. No-op for
    /// a module root / non-deletable cursor, or when a delete
    /// is already confirming or in flight.
    F3DeleteBegin,
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
fn key_action(key: &KeyEvent, keymap: &KeyMap) -> Option<UserAction> {
    if is_quit(key.code, key.modifiers, keymap.quit) {
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
    // keys-3: the pane-switch digit aliases are configurable
    // (`[keys] pane_fN`, default `1`-`4`). Plain press only (no
    // Ctrl/Alt). A `None` slot means that alias collided with a
    // higher-precedence binding and is disabled (the F-keys above still
    // navigate). `nav[i]` maps to F1..F4 in order.
    if !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        const PANES: [Screen; 4] = [Screen::F1, Screen::F2, Screen::F3, Screen::F4];
        for (i, alias) in keymap.nav.iter().enumerate() {
            if *alias == Some(key.code) {
                return Some(UserAction::Navigate(PANES[i]));
            }
        }
    }
    // keys-2: the configurable refresh key. Plain press only (no
    // Ctrl/Alt) so it never shadows `Ctrl+R` reload — which is already
    // handled above, but the guard keeps the contract explicit. Default
    // `r`. `None` when the configured refresh collided with quit (quit
    // wins; keys-2 R2 collision policy) — then there's no refresh key.
    if let Some(refresh) = keymap.refresh {
        if key.code == refresh
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return Some(UserAction::Refresh);
        }
    }
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => Some(UserAction::SelectNext),
        KeyCode::Up | KeyCode::Char('k') => Some(UserAction::SelectPrev),
        // d-42: vim-style jump to first / last row. `g`
        // (first) and `G` (last) extend the j/k cursor on
        // the F1 and F3 list panes; other panes ignore the
        // variants in their dispatch arms. Home/End alias
        // for operators who don't think in vim. Single `g`
        // (not the vim `gg` double-tap) — no chord state to
        // track, and `G` is already the natural pair.
        KeyCode::Char('g') | KeyCode::Home => Some(UserAction::SelectFirst),
        KeyCode::Char('G') | KeyCode::End => Some(UserAction::SelectLast),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Some(UserAction::Descend),
        KeyCode::Left | KeyCode::Char('h') => Some(UserAction::Ascend),
        // d-58: `t` (trigger transfer) on F1 — TUI_DESIGN §5.1.
        // Only the F1 dispatcher acts on it; other panes ignore.
        // While the trigger modal is open the F1 trigger keystroke
        // handler absorbs input first, so `t` is a text char then.
        KeyCode::Char('t') => Some(UserAction::F1TriggerBegin),
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
        // d-55: `m` (mirror) on F3 opens the destination prompt
        // in mirror mode for the cursor-selected remote path.
        // TUI_DESIGN §5.3 lists `m: mirror`; lowercase `m` is
        // free in the global map (no divergence needed, unlike
        // du/dump/batch-pull). While the dest prompt or mirror
        // confirm is open the F3 keystroke handlers absorb input
        // before this dispatcher runs, so `m` is a text char then.
        KeyCode::Char('m') => Some(UserAction::F3MirrorBegin),
        // d-57: `v` (move) on F3 — TUI_DESIGN §5.3 lists
        // `v: move`. Free in the global map (the F4 local move is
        // `V`, case-distinct). Pulls the cursor source then deletes
        // the remote source. While the dest prompt or destructive
        // confirm is open the F3 keystroke handlers absorb input
        // first, so `v` is a text char then.
        KeyCode::Char('v') => Some(UserAction::F3MoveBegin),
        // d-53: `P` (Shift+p) batch-pulls the F3 marked set —
        // the natural pair to `p` (the design lists `c` for
        // copy, but `c` is ProfileClear in the global map; `P`
        // pairs with `p` and is free, same divergence approach
        // as `u` for du / `s` for dump). While the dest prompt
        // is open the pull keystroke handler absorbs input first.
        KeyCode::Char('P') => Some(UserAction::F3BatchPullBegin),
        // d-41: `u` (usage) runs du for the F3 cursor row.
        // Other panes ignore the variant. TUI_DESIGN §5.3
        // lists `d` for du, but `d` is already
        // ProfileDisable on F4 (key_action is a global
        // map); we rebind to the `u`(sage) mnemonic, the
        // same divergence resolution used for `s`(napshot)
        // dump. While the F3 filter or pull-dest prompt is
        // open the text-input handler absorbs `u` first, so
        // it's only a du trigger in normal F3 nav mode.
        KeyCode::Char('u') => Some(UserAction::F3DuBegin),
        // d-49: `space` toggles the F3 multi-select mark on the
        // cursor row. Other panes ignore it. While the F3
        // filter / pull-dest prompt is open the text handlers
        // absorb space as a character first.
        KeyCode::Char(' ') => Some(UserAction::F3ToggleMark),
        // d-51: `a` marks/clears all visible F3 rows at once.
        KeyCode::Char('a') => Some(UserAction::F3ToggleMarkAll),
        // d-45: `D` (Shift+d) opens the F3 delete confirm
        // prompt for the cursor row. Capital chosen because
        // lowercase `d` is ProfileDisable (F4) and delete is
        // destructive — a deliberate Shift guards it. Other
        // panes ignore the variant. While the confirm prompt
        // is open the F3 delete keystroke handler absorbs
        // y/n/Esc before this dispatcher runs.
        KeyCode::Char('D') => Some(UserAction::F3DeleteBegin),
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
/// m2f-5: the set of daemons F2 watches — the launch `parsed_remote`
/// (if any) plus every discovered remote daemon, deduped by
/// `host_port_display()` identity (the same identity used as the
/// `state::row_key` daemon component and the row label — host, plus
/// `:port` when non-default, so same-host/different-port daemons stay
/// distinct). The launch daemon comes first so it's watched
/// immediately, before mDNS discovery settles.
///
/// Known edge: a daemon given as `parsed_remote` by hostname and also
/// discovered by IP has two distinct identities, so it'd be watched
/// twice (its rows appear under both labels). Identity reconciliation
/// across mDNS-IP and user-hostname forms is a follow-up (m2f-6).
fn f2_watched_endpoints(app: &AppState) -> Vec<RemoteEndpoint> {
    let mut seen: Vec<String> = Vec::new();
    let mut endpoints: Vec<RemoteEndpoint> = Vec::new();
    let candidates = app
        .parsed_remote
        .clone()
        .into_iter()
        .chain(app.daemons.remote_endpoints());
    for ep in candidates {
        let id = ep.host_port_display();
        if !seen.contains(&id) {
            seen.push(id);
            endpoints.push(ep);
        }
    }
    endpoints
}

/// m2f-9: the identity set F2 currently watches, keyed by the same
/// `host_port_display` `f2_watched_endpoints` dedups on. Comparing this
/// across an mDNS discovery update tells the loop whether the watch set
/// actually changed — so F2 can auto re-fan to pick up a newly
/// discovered daemon (or drop a vanished one) without an explicit `r`.
fn f2_watched_identities(app: &AppState) -> std::collections::BTreeSet<String> {
    f2_watched_endpoints(app)
        .iter()
        .map(|ep| ep.host_port_display())
        .collect()
}

/// m2f-5 R2: apply one F2 merged-stream signal. Returns `false` only
/// when the merged receiver should be dropped — i.e. `None`, meaning
/// every watched daemon's forwarder has closed (all senders gone). A
/// single daemon's `Error` returns `true` (keep the receiver) so the
/// other daemons' streams keep feeding F2; the whole-view status goes
/// Degraded for now (per-daemon status is m2f-6). `None` arm reads
/// `recv() == None`, the all-senders-closed condition.
/// m2f-10: fold per-daemon stream health into the single F2 connection
/// banner. `degraded` is the set of watched daemons whose stream has
/// errored; `watched_total` is how many daemons F2 is fanning out to.
///
/// - none degraded → `Live`.
/// - some (but not all) → `Degraded` with a "M/N streams down: ..."
///   message — the pane keeps showing the live daemons' transfers, but
///   the operator sees which ones dropped.
/// - all degraded → `Degraded` "all N daemon stream(s) down".
///
/// `watched_total` is floored at the degraded count so a stale/zero
/// total can't make "all-down" read as "partial".
fn f2_status_from_health(
    degraded: &std::collections::BTreeSet<String>,
    watched_total: usize,
) -> ConnectionStatus {
    if degraded.is_empty() {
        return ConnectionStatus::Live;
    }
    let total = watched_total.max(degraded.len());
    if degraded.len() >= total {
        ConnectionStatus::Degraded(format!("all {} daemon stream(s) down", degraded.len()))
    } else {
        let names: Vec<&str> = degraded.iter().map(String::as_str).collect();
        ConnectionStatus::Degraded(format!(
            "{}/{total} daemon streams down: {}",
            degraded.len(),
            names.join(", ")
        ))
    }
}

fn apply_f2_event(app: &mut AppState, event: Option<F2Event>) -> bool {
    match event {
        // m2f-4: the event carries its source daemon, so the row is
        // tagged with the stream's daemon.
        Some(F2Event {
            daemon,
            kind: EventOrError::Connected,
        }) => {
            // m2f-10: a healthy signal clears this daemon from the
            // degraded set (it reconnected) and the banner is re-derived.
            mark_daemon_healthy(app, &daemon);
            true
        }
        Some(F2Event {
            daemon,
            kind: EventOrError::Event(daemon_event),
        }) => {
            app.transfers
                .apply_event(&daemon, daemon_event, Instant::now());
            // An event is itself proof the stream is live.
            mark_daemon_healthy(app, &daemon);
            true
        }
        Some(F2Event {
            daemon,
            // The per-daemon error text is intentionally not surfaced in
            // the single-line banner — with the fan-out it could list
            // many daemons, so the banner names the affected daemon
            // identities (the actionable handle) and the count.
            kind: EventOrError::Error(_),
        }) => {
            // m2f-10: record THIS daemon as degraded and re-derive the
            // banner from the set vs. the watched total — one stream
            // ending must not blank the pane when others are still live.
            // Keep the merged receiver: only this daemon's forwarder
            // ended; the others keep sending.
            app.f2_degraded_daemons.insert(daemon);
            let total = f2_watched_endpoints(app).len();
            app.transfers_status = f2_status_from_health(&app.f2_degraded_daemons, total);
            true
        }
        None => {
            app.transfers_status =
                ConnectionStatus::Degraded("all subscribe streams closed".to_string());
            false
        }
    }
}

/// m2f-10: a watched daemon produced a healthy signal (Connected or any
/// event).
///
/// - If the daemon was in the degraded set, its stream just recovered →
///   re-derive the banner from the (now smaller) set.
/// - Otherwise only lift an initial `Connecting` to `Live`. Crucially we
///   must NOT overwrite a `Degraded` set by a failed initial `GetState`
///   (snapshot health, set in the setup-reply arm) just because the
///   stream is live — that distinction predates m2f-10 and the
///   `drain_startup_events` path relies on it.
fn mark_daemon_healthy(app: &mut AppState, daemon: &str) {
    let recovered = app.f2_degraded_daemons.remove(daemon);
    if recovered {
        let total = f2_watched_endpoints(app).len();
        app.transfers_status = f2_status_from_health(&app.f2_degraded_daemons, total);
    } else if matches!(app.transfers_status, ConnectionStatus::Connecting) {
        app.transfers_status = ConnectionStatus::Live;
    }
}

fn drain_startup_events(
    rx: &mut mpsc::Receiver<F2Event>,
    state: &mut TransfersState,
    status: &mut ConnectionStatus,
) {
    use tokio::sync::mpsc::error::TryRecvError;
    loop {
        match rx.try_recv() {
            // m2f-4: each event carries its source daemon, so the
            // applied row is tagged with the stream's daemon.
            Ok(F2Event { daemon, kind }) => match kind {
                EventOrError::Connected => {
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
                EventOrError::Event(event) => {
                    state.apply_event(&daemon, event, Instant::now());
                    // Same rule as Connected: first event is a
                    // stream-health signal. Don't paper over an
                    // existing Degraded snapshot status.
                    if matches!(status, ConnectionStatus::Connecting) {
                        *status = ConnectionStatus::Live;
                    }
                }
                EventOrError::Error(msg) => {
                    *status = ConnectionStatus::Degraded(msg);
                    // Continue draining — we don't expect more
                    // events after Error in practice (forwarder
                    // exits) but a benign extra Connected
                    // before Error shouldn't trip us up.
                }
            },
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return,
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

/// m2f-4: a Subscribe-stream signal tagged with the daemon it came
/// from. The forwarder stamps each event with its daemon identity
/// (`host_port_display()`) so the F2 event loop can route it to the
/// right `(daemon, transfer_id)` rows — the per-event identity the
/// m2f-5 fan-out (one forwarder per discovered daemon, all merged
/// into one channel) relies on. Single daemon today.
struct F2Event {
    daemon: String,
    kind: EventOrError,
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
/// m2f-5: open one daemon's Subscribe stream and forward its events
/// into the SHARED merged channel (`merged_tx`), tagged with the
/// daemon's stable identity. Returns `Ok` once the daemon's broadcast
/// receiver is registered (so the caller knows this daemon is being
/// watched). The fan-out calls this once per watched daemon, each
/// pushing into the same merged receiver the event loop drains.
async fn open_subscribe_stream(
    endpoint: &RemoteEndpoint,
    merged_tx: mpsc::Sender<F2Event>,
) -> Result<(), String> {
    let stream = jobs::subscribe(endpoint, "", false)
        .await
        .map_err(|err| format!("subscribe: {err}"))?;
    // m2f-4: tag every signal from this stream with the daemon's
    // stable identity (host:port — matches the row_key daemon
    // component and the F2 header label).
    let daemon = endpoint.host_port_display();
    // Send Connected immediately — subscribe() has returned
    // OK so the daemon broadcast receiver is registered.
    let _ = merged_tx
        .send(F2Event {
            daemon: daemon.clone(),
            kind: EventOrError::Connected,
        })
        .await;
    tokio::spawn(forward_subscribe_stream(stream, daemon, merged_tx));
    Ok(())
}

/// Inner loop of the Subscribe forwarder task. Reads
/// `stream.message()` and forwards events into `tx` until
/// the stream ends, errors, or `tx` reports a closed
/// receiver. Factored out of `open_subscribe_stream` so the
/// spawn site is a single function call.
async fn forward_subscribe_stream(
    mut stream: tonic::Streaming<DaemonEvent>,
    daemon: String,
    tx: mpsc::Sender<F2Event>,
) {
    // m2f-4: every forwarded signal carries its source daemon.
    let tag = |kind| F2Event {
        daemon: daemon.clone(),
        kind,
    };
    loop {
        match stream.message().await {
            Ok(Some(event)) => {
                if tx.send(tag(EventOrError::Event(event))).await.is_err() {
                    return;
                }
            }
            Ok(None) => {
                let _ = tx
                    .send(tag(EventOrError::Error("stream ended".to_string())))
                    .await;
                return;
            }
            Err(status) => {
                let _ = tx
                    .send(tag(EventOrError::Error(format!(
                        "stream: {}",
                        status.message()
                    ))))
                    .await;
                return;
            }
        }
    }
}

/// keys-1/2/3: the operator-remappable global key bindings, resolved
/// from `[keys]` config (with the collision policy already applied).
/// Quit, refresh, and the pane-switch digit aliases today; later slices
/// add per-screen keys. Built once per keystroke from the
/// (hot-reloadable) config, so a `Ctrl+R` remap takes effect live.
struct KeyMap {
    /// The configurable quit character. `Esc` / `Ctrl+C` quit regardless.
    quit: KeyCode,
    /// The configurable refresh character (plain press; `Ctrl+R` reload
    /// is separate). `None` when the configured refresh collided with a
    /// higher-precedence binding (see `KeysDefaults::resolved`).
    refresh: Option<KeyCode>,
    /// keys-3: pane-switch digit aliases, F1..F4 order. Each is `None`
    /// when its configured char collided with a higher-precedence
    /// binding. The function keys F1-F4 navigate regardless.
    nav: [Option<KeyCode>; 4],
}

impl KeyMap {
    fn from_config(config: &config::TuiConfig) -> Self {
        let resolved = config.keys.resolved();
        Self {
            quit: KeyCode::Char(resolved.quit),
            refresh: resolved.refresh.map(KeyCode::Char),
            nav: resolved.nav.map(|c| c.map(KeyCode::Char)),
        }
    }
}

/// Quit predicate. The configured quit key (`q` by default) is the
/// muscle-memory shortcut; `Esc` is the secondary, and `Ctrl-C` is the
/// safety net for a stuck UI — the latter two always quit so a bad
/// `[keys] quit` value can never lock the operator in.
fn is_quit(code: KeyCode, modifiers: KeyModifiers, quit: KeyCode) -> bool {
    // keys-1 R2: the configured quit char claims only a PLAIN press —
    // never a Ctrl/Alt chord for that character. Otherwise `quit = "r"`
    // would steal `Ctrl+R` (config reload), and any future Ctrl/Alt
    // chord for the chosen char. Shift is allowed (capitals are distinct
    // KeyCodes anyway). `Esc` and `Ctrl+C` stay as modifier-aware
    // failsafes regardless of the configured char.
    (code == quit && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT))
        || matches!(code, KeyCode::Esc)
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

    /// keys-1: the default keymap (quit = `q`) recognises q / Esc /
    /// Ctrl+C, and ignores unrelated keys.
    #[test]
    fn is_quit_recognises_default_quit_esc_ctrl_c() {
        let q = KeyCode::Char('q');
        assert!(is_quit(KeyCode::Char('q'), KeyModifiers::empty(), q));
        assert!(is_quit(KeyCode::Esc, KeyModifiers::empty(), q));
        assert!(is_quit(KeyCode::Char('c'), KeyModifiers::CONTROL, q));
        assert!(!is_quit(KeyCode::Char('a'), KeyModifiers::empty(), q));
        assert!(!is_quit(KeyCode::Enter, KeyModifiers::empty(), q));
        // Plain 'c' without Ctrl is not a quit shortcut.
        assert!(!is_quit(KeyCode::Char('c'), KeyModifiers::empty(), q));
    }

    /// keys-1: a remapped quit key is honoured, and Esc / Ctrl+C stay
    /// as failsafes (the old `q` no longer quits once remapped).
    #[test]
    fn is_quit_honours_remapped_key_and_failsafes() {
        let x = KeyCode::Char('x');
        assert!(is_quit(KeyCode::Char('x'), KeyModifiers::empty(), x));
        // Failsafes unaffected by the remap.
        assert!(is_quit(KeyCode::Esc, KeyModifiers::empty(), x));
        assert!(is_quit(KeyCode::Char('c'), KeyModifiers::CONTROL, x));
        // The old default no longer quits.
        assert!(!is_quit(KeyCode::Char('q'), KeyModifiers::empty(), x));
    }

    /// keys-1: a remapped quit key flows from config through the
    /// keymap into key_action. Uses inline `KeyEvent` (not the `k`
    /// helper) and an explicit custom keymap.
    #[test]
    fn key_action_honours_remapped_quit() {
        let mut cfg = config::TuiConfig::default();
        cfg.keys.quit = "x".to_string();
        let custom = KeyMap::from_config(&cfg);
        let ev = |code| KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
        };
        assert!(matches!(
            key_action(&ev(KeyCode::Char('x')), &custom),
            Some(UserAction::Quit)
        ));
        // Default 'q' is no longer quit under this keymap.
        assert!(!matches!(
            key_action(&ev(KeyCode::Char('q')), &custom),
            Some(UserAction::Quit)
        ));
    }

    /// keys-1 R2 regression: a remapped quit char must NOT hijack the
    /// Ctrl/Alt chord for that character. `quit = "r"` claims a PLAIN
    /// `r` (quit) but leaves `Ctrl+R` as config reload.
    #[test]
    fn remapped_quit_does_not_steal_ctrl_chord() {
        let mut cfg = config::TuiConfig::default();
        cfg.keys.quit = "r".to_string();
        let custom = KeyMap::from_config(&cfg);
        // Plain 'r' → quit (the operator's chosen quit key).
        assert!(matches!(
            key_action(
                &KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: KeyModifiers::empty(),
                },
                &custom
            ),
            Some(UserAction::Quit)
        ));
        // Ctrl+R → still config reload, NOT quit.
        assert!(matches!(
            key_action(
                &KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: KeyModifiers::CONTROL,
                },
                &custom
            ),
            Some(UserAction::ReloadConfig)
        ));
        // is_quit itself: the configured char with Ctrl/Alt is not quit.
        let r = KeyCode::Char('r');
        assert!(is_quit(r, KeyModifiers::empty(), r));
        assert!(!is_quit(r, KeyModifiers::CONTROL, r));
        assert!(!is_quit(r, KeyModifiers::ALT, r));
        // Failsafes still fire under the remap.
        assert!(is_quit(KeyCode::Esc, KeyModifiers::empty(), r));
        assert!(is_quit(KeyCode::Char('c'), KeyModifiers::CONTROL, r));
    }

    /// keys-2: a remapped refresh key is honoured, the old default `r`
    /// stops refreshing, and `Ctrl+R` config reload is unaffected.
    #[test]
    fn key_action_honours_remapped_refresh() {
        let mut cfg = config::TuiConfig::default();
        cfg.keys.refresh = "R".to_string();
        let custom = KeyMap::from_config(&cfg);
        let ev = |code, modifiers| KeyEvent { code, modifiers };
        // Remapped 'R' → Refresh.
        assert!(matches!(
            key_action(&ev(KeyCode::Char('R'), KeyModifiers::empty()), &custom),
            Some(UserAction::Refresh)
        ));
        // Old default 'r' no longer refreshes under this map.
        assert!(!matches!(
            key_action(&ev(KeyCode::Char('r'), KeyModifiers::empty()), &custom),
            Some(UserAction::Refresh)
        ));
        // Ctrl+R is still config reload (refresh remap doesn't touch it).
        assert!(matches!(
            key_action(&ev(KeyCode::Char('r'), KeyModifiers::CONTROL), &custom),
            Some(UserAction::ReloadConfig)
        ));
    }

    /// keys-2 R2: when the configured refresh collides with quit, quit
    /// wins and there is no usable refresh key (KeyMap.refresh = None).
    #[test]
    fn quit_refresh_collision_disables_refresh() {
        let mut cfg = config::TuiConfig::default();
        cfg.keys.quit = "r".to_string(); // collides with default refresh "r"
        let custom = KeyMap::from_config(&cfg);
        assert!(custom.refresh.is_none(), "refresh disabled on collision");
        // Plain 'r' → Quit (the precedence winner), never Refresh.
        assert!(matches!(
            key_action(
                &KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: KeyModifiers::empty(),
                },
                &custom
            ),
            Some(UserAction::Quit)
        ));
    }

    /// keys-3: a remapped pane-switch alias is honoured, the old default
    /// digit stops navigating, other panes are unaffected, and the
    /// conventional F-keys always navigate.
    #[test]
    fn key_action_honours_remapped_pane() {
        let mut cfg = config::TuiConfig::default();
        cfg.keys.pane_f2 = "t".to_string();
        let custom = KeyMap::from_config(&cfg);
        let ev = |code| KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
        };
        // Remapped 't' → Navigate F2.
        assert!(matches!(
            key_action(&ev(KeyCode::Char('t')), &custom),
            Some(UserAction::Navigate(Screen::F2))
        ));
        // Old default '2' no longer navigates.
        assert!(!matches!(
            key_action(&ev(KeyCode::Char('2')), &custom),
            Some(UserAction::Navigate(Screen::F2))
        ));
        // Other panes unaffected: '1' still → F1.
        assert!(matches!(
            key_action(&ev(KeyCode::Char('1')), &custom),
            Some(UserAction::Navigate(Screen::F1))
        ));
        // The function key F2 navigates regardless of the digit remap.
        assert!(matches!(
            key_action(&ev(KeyCode::F(2)), &custom),
            Some(UserAction::Navigate(Screen::F2))
        ));
    }

    /// dark-1: the base style is `None` only when both bg and fg are
    /// unset (→ no base layer, terminal default); otherwise it carries
    /// whichever colors are set.
    #[test]
    fn base_theme_style_built_from_set_colors() {
        use ratatui::style::Color;
        assert!(base_theme_style(None, None).is_none());
        let s = base_theme_style(Some(Color::Black), None).unwrap();
        assert_eq!(s.bg, Some(Color::Black));
        assert_eq!(s.fg, None);
        let s = base_theme_style(None, Some(Color::White)).unwrap();
        assert_eq!(s.fg, Some(Color::White));
        assert_eq!(s.bg, None);
        let s = base_theme_style(Some(Color::Black), Some(Color::White)).unwrap();
        assert_eq!(s.bg, Some(Color::Black));
        assert_eq!(s.fg, Some(Color::White));
    }

    /// dark-1: validates the mechanism the whole feature relies on — a
    /// base layer's background shows through a fg-only widget rendered on
    /// top (ratatui leaves a cell's bg unchanged when the widget's style
    /// has `bg: None`).
    #[test]
    fn base_layer_bg_shows_through_fg_only_widget() {
        use ratatui::style::{Color, Style};
        use ratatui::text::Span;
        use ratatui::widgets::{Block, Paragraph};
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(8, 2);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                // Base layer: bg = Blue across the whole frame.
                frame.render_widget(
                    Block::default().style(Style::default().bg(Color::Blue)),
                    frame.area(),
                );
                // A fg-only widget on top (its style sets fg, not bg).
                frame.render_widget(
                    Paragraph::new(Span::styled("hi", Style::default().fg(Color::White))),
                    frame.area(),
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let cell = &buf[(0, 0)]; // the 'h'
        assert_eq!(cell.fg, Color::White, "widget fg applied");
        assert_eq!(
            cell.bg,
            Color::Blue,
            "fg-only widget inherits the base layer's bg"
        );
    }

    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }

    /// Default keymap (quit = `q`) for the key_action tests below.
    fn km() -> KeyMap {
        KeyMap::from_config(&config::TuiConfig::default())
    }

    /// Test wrapper: classify a key under the DEFAULT keymap. Most
    /// key_action tests don't care about remapping, so this keeps them
    /// terse (and let keys-1 thread the keymap without touching them).
    fn ka(key: &KeyEvent) -> Option<UserAction> {
        key_action(key, &km())
    }

    #[test]
    fn key_action_maps_quit_and_refresh() {
        assert!(matches!(ka(&k(KeyCode::Char('q'))), Some(UserAction::Quit)));
        assert!(matches!(ka(&k(KeyCode::Esc)), Some(UserAction::Quit)));
        assert!(matches!(
            ka(&KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }),
            Some(UserAction::Quit)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('r'))),
            Some(UserAction::Refresh)
        ));
    }

    /// a1-6: F-keys F1..F4 map to Navigate(...) for the
    /// corresponding pane. Verified across all four keys.
    #[test]
    fn key_action_maps_f_keys_to_navigate() {
        let f = |n| ka(&k(KeyCode::F(n)));
        assert!(matches!(f(1), Some(UserAction::Navigate(Screen::F1))));
        assert!(matches!(f(2), Some(UserAction::Navigate(Screen::F2))));
        assert!(matches!(f(3), Some(UserAction::Navigate(Screen::F3))));
        // d-19: digit aliases for tab nav. F1-F4 still
        // map but so do 1-4 — terminals that drop F-keys
        // (mosh / certain SSH proxies / CI muxers) can
        // still navigate. Helper closure pins each.
        let d = |c| ka(&k(KeyCode::Char(c)));
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
                ka(&KeyEvent {
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
        assert!(matches!(ka(&k(KeyCode::Enter)), Some(UserAction::Descend)));
        assert!(matches!(ka(&k(KeyCode::Right)), Some(UserAction::Descend)));
        assert!(matches!(
            ka(&k(KeyCode::Char('l'))),
            Some(UserAction::Descend)
        ));
        assert!(matches!(ka(&k(KeyCode::Left)), Some(UserAction::Ascend)));
        assert!(matches!(
            ka(&k(KeyCode::Char('h'))),
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
            ka(&k(KeyCode::Down)),
            Some(UserAction::SelectNext)
        ));
        assert!(matches!(ka(&k(KeyCode::Up)), Some(UserAction::SelectPrev)));
        assert!(matches!(
            ka(&k(KeyCode::Char('j'))),
            Some(UserAction::SelectNext)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('k'))),
            Some(UserAction::SelectPrev)
        ));
    }

    #[test]
    fn key_action_returns_none_for_unmapped_keys() {
        // `z` is unmapped (`a` became F3ToggleMarkAll in d-51).
        assert!(ka(&k(KeyCode::Char('z'))).is_none());
        assert!(ka(&k(KeyCode::Char('R'))).is_none()); // case-sensitive
        assert!(ka(&k(KeyCode::Char('J'))).is_none()); // case-sensitive
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
        assert!(matches!(ka(&ctrl_r), Some(UserAction::ReloadConfig)));
        assert!(matches!(
            ka(&k(KeyCode::Char('r'))),
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
            ka(&k(KeyCode::Char('K'))),
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
            daemon: "nas".to_string(),
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
            daemon: "nas".to_string(),
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
            daemon: "nas".to_string(),
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
            daemon: "nas".to_string(),
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
            ka(&k(KeyCode::Char('X'))),
            Some(UserAction::CancelAllActiveTransfers)
        ));
    }

    /// d-30: ConfirmingBatch reports `is_confirming` true
    /// — Esc routing extends to the batch prompt.
    #[test]
    fn f2_cancel_status_confirming_batch_predicates() {
        let s = F2CancelStatus::ConfirmingBatch {
            targets: vec![("nas".to_string(), "a".to_string())],
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
            targets: vec![
                ("nas".into(), "a".into()),
                ("nas".into(), "b".into()),
                ("nas".into(), "c".into()),
                ("nas".into(), "d".into()),
                ("nas".into(), "e".into()),
            ],
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
            targets: vec![("nas".into(), "a".into())],
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
            targets: vec![("nas".into(), "a".into())],
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
        // m2f-8: targets are (daemon, transfer_id) pairs.
        let frozen = vec![
            ("nas".to_string(), "t-A".to_string()),
            ("skippy:9001".to_string(), "t-B".to_string()),
        ];
        let status = F2CancelStatus::ConfirmingBatch {
            targets: frozen.clone(),
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
        // Pattern-match round-trip: the targets the dispatcher
        // would read on `y` are exactly the ones the
        // operator confirmed.
        match status {
            F2CancelStatus::ConfirmingBatch { targets } => {
                assert_eq!(targets, frozen);
            }
            other => panic!("expected ConfirmingBatch, got {other:?}"),
        }
    }

    /// m2f-8: snapshot_active_targets captures (daemon, id) for every
    /// active row across daemons at the snapshot moment.
    #[test]
    fn snapshot_active_targets_captures_all_active_rows() {
        use blit_core::generated::{daemon_event, DaemonEvent, TransferStarted};
        let mut transfers = state::TransfersState::new();
        let started = |id: &str| DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: id.to_string(),
                kind: 0,
                peer: String::new(),
                module: String::new(),
                path: String::new(),
                start_unix_ms: 1_000_000,
            })),
        };
        transfers.apply_event("nas", started("t-A"), Instant::now());
        transfers.apply_event("skippy:9001", started("t-B"), Instant::now());
        let mut targets = snapshot_active_targets(&transfers);
        targets.sort();
        assert_eq!(
            targets,
            vec![
                ("nas".to_string(), "t-A".to_string()),
                ("skippy:9001".to_string(), "t-B".to_string()),
            ]
        );
    }

    /// m2f-8: snapshot_active_targets returns empty for a fresh state.
    /// Pairs with the dispatcher's `if targets.is_empty()` no-op guard.
    #[test]
    fn snapshot_active_targets_empty_state() {
        let transfers = state::TransfersState::new();
        assert!(snapshot_active_targets(&transfers).is_empty());
    }

    /// m2f-8: batch cancel spawns one RPC per (daemon, id) target and
    /// skips any whose daemon identity won't parse — returning the
    /// count actually dispatched.
    #[tokio::test]
    async fn spawn_cancels_for_targets_skips_malformed_and_counts_valid() {
        let (tx, _rx) = mpsc::channel::<CancelReply>(4);
        let mut seq = 0u64;
        let count = spawn_cancels_for_targets(
            vec![
                ("nas".to_string(), "t1".to_string()),
                ("skippy:9001".to_string(), "t2".to_string()),
                // Empty daemon identity → cancel_endpoint None → skipped.
                (String::new(), "t3".to_string()),
            ],
            &mut seq,
            &tx,
        );
        assert_eq!(count, 2, "two valid targets dispatched, malformed skipped");
        assert_eq!(seq, 2, "request seq advanced once per dispatched cancel");
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

    // d-40 R2 (reviewer reopen): the F3 pull outcome TTL
    // must collapse the sleep budget just like the d-24
    // cancel TTL, or a long live tick silently delays a
    // short `pull_status_ttl_ms`.

    #[test]
    fn min_opt_picks_the_shorter_deadline() {
        use std::time::Duration;
        assert_eq!(
            min_opt(
                Some(Duration::from_millis(250)),
                Some(Duration::from_millis(60_000))
            ),
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            min_opt(Some(Duration::from_millis(900)), None),
            Some(Duration::from_millis(900))
        );
        assert_eq!(
            min_opt(None, Some(Duration::from_millis(120))),
            Some(Duration::from_millis(120))
        );
        assert_eq!(min_opt(None, None), None);
    }

    /// The reviewer's exact scenario: 5s live tick, 250ms
    /// pull TTL, an F3 terminal fragment showing (so
    /// `pull_remaining = Some(~250ms)`). The sleep budget
    /// must be no greater than the remaining pull TTL — i.e.
    /// the loop wakes to hide the fragment on time, not ~20x
    /// late. (`terminal_remaining`'s own behavior is covered
    /// in `f3pull::tests`; here we assert the budget chain.)
    #[test]
    fn short_pull_ttl_overrides_long_live_tick() {
        use std::time::Duration;
        let live_tick = Duration::from_millis(5_000);
        let pull_remaining = Some(Duration::from_millis(250));
        let budget = compute_tick_budget(true, live_tick, min_opt(None, pull_remaining));
        assert_eq!(
            budget,
            Some(Duration::from_millis(250)),
            "budget must collapse to the 250ms pull deadline, not the 5s tick"
        );
    }

    /// Both a cancel fragment and a pull fragment pending
    /// (defensive — they're screen-exclusive in practice):
    /// the loop wakes for whichever is nearer.
    #[test]
    fn budget_picks_nearer_of_cancel_and_pull_deadlines() {
        use std::time::Duration;
        let budget = compute_tick_budget(
            true,
            Duration::from_millis(5_000),
            min_opt(
                Some(Duration::from_millis(800)),
                Some(Duration::from_millis(250)),
            ),
        );
        assert_eq!(budget, Some(Duration::from_millis(250)));
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
        let (tx, mut rx) = mpsc::channel::<F2Event>(4);
        // Forwarder always pushes Connected first.
        tx.send(F2Event {
            daemon: "nas".to_string(),
            kind: EventOrError::Connected,
        })
        .await
        .unwrap();
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
        let (tx, mut rx) = mpsc::channel::<F2Event>(4);
        tx.send(F2Event {
            daemon: "nas".to_string(),
            kind: EventOrError::Connected,
        })
        .await
        .unwrap();
        drop(tx);

        let mut state = TransfersState::new();
        let mut status = ConnectionStatus::Connecting;
        drain_startup_events(&mut rx, &mut state, &mut status);
        assert!(matches!(status, ConnectionStatus::Live));
    }

    /// m2f-4: a drained Event tags its row with the EVENT's source
    /// daemon (carried per-event from the stream), not a single
    /// global label — the per-stream identity the m2f-5 fan-out needs.
    #[tokio::test]
    async fn drain_startup_events_tags_row_with_event_daemon() {
        let (tx, mut rx) = mpsc::channel::<F2Event>(4);
        tx.send(F2Event {
            daemon: "skippy:9001".to_string(),
            kind: EventOrError::Event(DaemonEvent {
                payload: Some(
                    blit_core::generated::daemon_event::Payload::TransferStarted(
                        blit_core::generated::TransferStarted {
                            transfer_id: "t1".to_string(),
                            kind: 0,
                            peer: String::new(),
                            module: String::new(),
                            path: String::new(),
                            start_unix_ms: 1,
                        },
                    ),
                ),
            }),
        })
        .await
        .unwrap();
        drop(tx);

        let mut state = TransfersState::new();
        let mut status = ConnectionStatus::Connecting;
        drain_startup_events(&mut rx, &mut state, &mut status);
        assert_eq!(
            state.active_rows()[0].source_daemon,
            "skippy:9001",
            "row tagged with the event's daemon"
        );
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
                capacities: Vec::new(),
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
            ka(&k(KeyCode::Char('?'))),
            Some(UserAction::ToggleHelp)
        ));
    }

    /// d-26: `/` is mapped to F3FilterBegin. F1/F2/F4
    /// dispatch arms ignore the variant.
    #[test]
    fn key_action_maps_slash_to_f3_filter_begin() {
        assert!(matches!(
            ka(&k(KeyCode::Char('/'))),
            Some(UserAction::F3FilterBegin)
        ));
    }

    /// d-53 R2: `needs_container_slash` — a batch dest gets a
    /// trailing slash forced unless it's blank or already
    /// slash-terminated.
    #[test]
    fn needs_container_slash_cases() {
        assert!(needs_container_slash("/tmp/out"));
        assert!(needs_container_slash("relative/dir"));
        assert!(!needs_container_slash("/tmp/out/"), "already a container");
        assert!(!needs_container_slash(""), "blank → nothing to do");
        assert!(!needs_container_slash("   "), "whitespace → nothing to do");
    }

    /// d-53: when the last queued source finishes (`remaining`
    /// empty), `advance_batch_pull` clears the batch. (The
    /// start-next path spawns a task and is exercised manually.)
    #[test]
    fn advance_batch_pull_clears_when_queue_empty() {
        let mut app = make_test_app_state(Screen::F3);
        app.f3_batch_pull = Some(BatchPull {
            remaining: std::collections::VecDeque::new(),
            raw_dest: "/tmp/out".to_string(),
            done: 2,
            total: 3,
        });
        advance_batch_pull(&mut app);
        assert!(app.f3_batch_pull.is_none(), "an empty queue ends the batch");
    }

    /// d-53: `P` maps to F3BatchPullBegin (and `p` stays the
    /// single-cursor pull).
    #[test]
    fn key_action_maps_shift_p_to_batch_pull() {
        assert!(matches!(
            ka(&k(KeyCode::Char('P'))),
            Some(UserAction::F3BatchPullBegin)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('p'))),
            Some(UserAction::F3PullBegin)
        ));
    }

    /// d-49: `space` maps to F3ToggleMark.
    #[test]
    fn key_action_maps_space_to_f3_toggle_mark() {
        assert!(matches!(
            ka(&k(KeyCode::Char(' '))),
            Some(UserAction::F3ToggleMark)
        ));
    }

    /// d-51: `a` maps to F3ToggleMarkAll.
    #[test]
    fn key_action_maps_a_to_f3_toggle_mark_all() {
        assert!(matches!(
            ka(&k(KeyCode::Char('a'))),
            Some(UserAction::F3ToggleMarkAll)
        ));
    }

    /// d-41: `u` maps to F3DuBegin. Other panes ignore the
    /// variant in their dispatch arms.
    #[test]
    fn key_action_maps_u_to_f3_du_begin() {
        assert!(matches!(
            ka(&k(KeyCode::Char('u'))),
            Some(UserAction::F3DuBegin)
        ));
    }

    /// d-45: `D` maps to F3DeleteBegin. Other panes ignore it.
    #[test]
    fn key_action_maps_shift_d_to_f3_delete_begin() {
        assert!(matches!(
            ka(&k(KeyCode::Char('D'))),
            Some(UserAction::F3DeleteBegin)
        ));
        // Lowercase `d` stays ProfileDisable (F4), NOT delete.
        assert!(matches!(
            ka(&k(KeyCode::Char('d'))),
            Some(UserAction::ProfileDisable)
        ));
    }

    // d-45: only non-root remote paths are deletable from the
    // TUI (mirrors `blit rm`'s module-root guard).

    #[test]
    fn is_deletable_rejects_module_root_and_discovery() {
        // A module root (no rel-path) must NOT be deletable.
        // Trailing slash = the module-root form.
        let root = RemoteEndpoint::parse("nas:/home/").expect("module root");
        assert!(
            !is_deletable_remote_path(&root),
            "deleting a whole module from the TUI must be refused"
        );
        // A bare-host discovery endpoint carries no path.
        if let Ok(disco) = RemoteEndpoint::parse("nas") {
            assert!(!is_deletable_remote_path(&disco));
        }
    }

    #[test]
    fn is_deletable_accepts_a_sub_path() {
        let path = RemoteEndpoint::parse("nas:/home/photos/old.jpg").expect("sub path");
        assert!(is_deletable_remote_path(&path));
    }

    /// d-45 R2 (reviewer reopen): the Purge wire path must be a
    /// forward-slash relative path regardless of client OS — NOT
    /// a `to_string_lossy` of a platform-shaped `PathBuf`. Pins
    /// the conversion boundary at `(module, rel_path)` → wire.
    #[test]
    fn del_wire_path_is_forward_slash_joined() {
        use blit_app::admin::rm;
        // Resolve a multi-component cursor endpoint the same way
        // the dispatcher does, then run the conversion boundary.
        let ep = RemoteEndpoint::parse("nas:/home/photos/old.jpg").expect("ep");
        let (module, rel_path) = rm::extract_module_and_path(&ep).expect("extract");
        assert_eq!(module, "home");
        assert_eq!(
            del_wire_path(&rel_path),
            "photos/old.jpg",
            "wire path must be forward-slash joined, not a platform PathBuf render"
        );
        // Also pin it directly against a component-pushed path
        // (how a Windows client assembles the cursor path).
        let mut built = std::path::PathBuf::new();
        built.push("photos");
        built.push("old.jpg");
        assert_eq!(del_wire_path(&built), "photos/old.jpg");
    }

    /// d-45 R2: a successful delete reply must refresh the F3
    /// listing so the deleted row can't linger as an
    /// apparently-live entry. Pins that an applied delete +
    /// the refresh path invalidate `browse_last_fetched_view`.
    #[test]
    fn successful_delete_invalidates_browse_view() {
        let mut app = make_test_app_state(Screen::F3);
        // A remote must be configured for the refresh to run.
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:/home/").expect("remote"));
        // Pretend a fetch already happened for the current view.
        app.browse_last_fetched_view = Some(app.browse.view().clone());
        // Drive a delete to the Deleting state and capture its id.
        let ep = RemoteEndpoint::parse("nas:/home/old.txt").expect("ep");
        app.f3_del.begin(
            ep,
            vec!["old.txt".to_string()],
            "nas:/home/old.txt".to_string(),
            Some("nas:/home/old.txt".to_string()),
        );
        let launch = app.f3_del.confirm().expect("confirm");
        // Apply the success reply exactly as the select arm does.
        if app.f3_del.apply_done(launch.request_id, 1, Instant::now()) {
            handle_f3_refresh(
                &mut app.browse,
                app.parsed_remote.is_some(),
                &mut app.browse_last_fetched_view,
            );
        }
        assert!(
            app.browse_last_fetched_view.is_none(),
            "a successful delete must invalidate the fetched view so the listing refreshes"
        );
        assert!(matches!(
            app.f3_del.status(),
            f3del::F3DelStatus::Done { .. }
        ));
    }

    /// d-47 R2 (reviewer reopen): `enter` on the Local row must
    /// be a no-op — F3 is a remote browser, and Local resolves to
    /// loopback which we must NOT browse. A fresh DaemonsState has
    /// only the Local row with the cursor on it.
    #[test]
    fn f1_browse_target_is_none_for_local_row() {
        let daemons = daemons::DaemonsState::new();
        assert!(
            f1_browse_target(&daemons).is_none(),
            "Enter on the Local row must not retarget the browser"
        );
    }

    /// d-47 R2: `enter` on a discovered REMOTE daemon resolves to
    /// that daemon's endpoint (the path that should retarget).
    #[test]
    fn f1_browse_target_is_some_for_remote_row() {
        use blit_core::mdns::MdnsDiscoveredService;
        use std::collections::HashMap;
        use std::net::Ipv4Addr;

        let mut daemons = daemons::DaemonsState::new();
        let svc = MdnsDiscoveredService {
            fullname: "skippy._blit._tcp.local.".to_string(),
            instance_name: "skippy".to_string(),
            hostname: "skippy.local.".to_string(),
            port: 9031,
            addresses: vec![Ipv4Addr::new(192, 168, 1, 20)],
            properties: HashMap::new(),
        };
        daemons.replace_from_discovery(&[svc], std::time::Instant::now());
        // Move the cursor off the Local row (index 0) onto skippy.
        daemons.select_next();
        assert!(
            !daemons.selected_row().unwrap().is_local(),
            "cursor should be on the remote row now"
        );
        assert!(
            f1_browse_target(&daemons).is_some(),
            "Enter on a remote daemon resolves to its endpoint"
        );
    }

    /// d-48: `reset_f2_for_resubscribe` repoints F2 at the new
    /// daemon — sets parsed_remote/label, clears the stream + rows,
    /// marks a setup pending, and bumps the generation (so a stale
    /// in-flight reply from the old daemon is dropped).
    #[test]
    fn reset_f2_for_resubscribe_repoints_and_bumps_generation() {
        let mut app = make_test_app_state(Screen::F1);
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:/home/").expect("launch"));
        let gen_before = app.transfers_setup_gen;
        // Simulate a live F2 stream + a pending flag cleared.
        let (_tx, rx) = mpsc::channel::<F2Event>(1);
        let mut event_rx = Some(rx);
        app.transfers_setup_pending = false;

        // d-48 R2: a cancel confirm open against the OLD daemon
        // must not survive the switch.
        app.cancel_status = F2CancelStatus::Confirming {
            transfer_id: "old-daemon-job".to_string(),
            daemon: "nas".to_string(),
        };

        let other = RemoteEndpoint::parse("skippy:/media/").expect("other");
        let gen = reset_f2_for_resubscribe(&mut app, &other, &mut event_rx);

        assert_eq!(
            app.parsed_remote.as_ref().map(|e| e.host_port_display()),
            Some(other.host_port_display()),
            "F2 now targets the new daemon"
        );
        assert!(event_rx.is_none(), "old Subscribe stream is dropped");
        assert!(app.transfers_setup_pending, "a fresh setup is pending");
        assert_eq!(gen, gen_before + 1, "generation bumped");
        assert_eq!(app.transfers_setup_gen, gen_before + 1);
        assert!(matches!(app.transfers_status, ConnectionStatus::Connecting));
        assert_eq!(app.transfers.active_count(), 0, "old rows cleared");
        assert!(
            matches!(app.cancel_status, F2CancelStatus::Idle),
            "a stale cancel confirm from the old daemon is cleared"
        );
    }

    /// d-47: `retarget_browse` points F3 at a new daemon — sets
    /// browse_target, resets the browse view to Modules, clears
    /// the last-fetched marker (so the loop re-fetches), and
    /// navigates to F3. `parsed_remote` (F2's target) is left
    /// untouched.
    #[test]
    fn retarget_browse_switches_f3_target_and_navigates() {
        let mut app = make_test_app_state(Screen::F1);
        let launch = RemoteEndpoint::parse("nas:/home/").expect("launch");
        app.parsed_remote = Some(launch.clone());
        app.browse_target = Some(launch.clone());
        // Pretend F3 had already fetched a module view.
        app.browse.descend(); // (no rows → no-op, view stays Modules)
        app.browse_last_fetched_view = Some(app.browse.view().clone());

        let other = RemoteEndpoint::parse("skippy:/media/").expect("other daemon");
        retarget_browse(&mut app, other.clone());

        assert_eq!(app.current_screen, Screen::F3, "jumps to F3");
        assert_eq!(
            app.browse_target.as_ref().map(|e| e.host_port_display()),
            Some(other.host_port_display()),
            "F3 now targets the selected daemon"
        );
        assert!(
            app.browse_last_fetched_view.is_none(),
            "cleared so the loop re-fetches the new daemon's modules"
        );
        assert!(
            matches!(app.browse.view(), browse::BrowseView::Modules),
            "browse resets to the Modules list"
        );
        // F2's target is unchanged — it stays on the launch remote.
        assert_eq!(
            app.parsed_remote.as_ref().map(|e| e.host_port_display()),
            Some(launch.host_port_display()),
            "parsed_remote (F2) is not retargeted"
        );
    }

    /// d-45 R2: a *stale* delete reply (superseded run) must NOT
    /// trigger a refresh — `apply_done` returns false and the
    /// view stays as-is.
    #[test]
    fn stale_delete_reply_does_not_refresh() {
        let mut app = make_test_app_state(Screen::F3);
        app.browse_last_fetched_view = Some(app.browse.view().clone());
        let ep = RemoteEndpoint::parse("nas:/home/old.txt").expect("ep");
        app.f3_del.begin(
            ep,
            vec!["old.txt".to_string()],
            "nas:/home/old.txt".to_string(),
            Some("nas:/home/old.txt".to_string()),
        );
        let launch = app.f3_del.confirm().expect("confirm");
        let stale = launch.request_id + 99;
        if app.f3_del.apply_done(stale, 1, Instant::now()) {
            handle_f3_refresh(
                &mut app.browse,
                app.parsed_remote.is_some(),
                &mut app.browse_last_fetched_view,
            );
        }
        assert!(
            app.browse_last_fetched_view.is_some(),
            "a stale (superseded) delete reply must not invalidate the view"
        );
    }

    // d-45: delete display bridge. Confirming/Deleting always
    // show; Done/Error are path-gated like du.

    /// d-50: `build_delete_request` — single vs batch shaping +
    /// module-root filtering.
    #[test]
    fn build_delete_request_single_is_gated_with_path_label() {
        let ep = RemoteEndpoint::parse("nas:/home/old.txt").expect("ep");
        let (module_ep, rels, label, gate) =
            build_delete_request(vec![ep], false).expect("deletable");
        assert_eq!(rels, vec!["old.txt".to_string()]);
        assert_eq!(label, module_ep.display());
        assert_eq!(gate.as_deref(), Some(module_ep.display().as_str()));
    }

    #[test]
    fn build_delete_request_batch_counts_and_is_ungated() {
        let a = RemoteEndpoint::parse("nas:/home/a.txt").expect("a");
        let b = RemoteEndpoint::parse("nas:/home/b.txt").expect("b");
        let (_module_ep, rels, label, gate) =
            build_delete_request(vec![a, b], true).expect("deletable");
        assert_eq!(rels.len(), 2);
        assert!(rels.contains(&"a.txt".to_string()));
        assert!(rels.contains(&"b.txt".to_string()));
        assert_eq!(label, "2 item(s)");
        assert!(gate.is_none(), "batch outcome is not path-gated");
    }

    #[test]
    fn build_delete_request_filters_module_roots() {
        // A module root is not deletable → nothing to delete.
        let root = RemoteEndpoint::parse("nas:/home/").expect("module root");
        assert!(build_delete_request(vec![root], false).is_none());
    }

    /// d-57 R2 (reviewer reopen): the F3 move gate is the same
    /// `is_deletable_remote_path` check delete uses — a move whose
    /// source is a module root must be refused up front, because the
    /// post-receive source delete (an empty/root purge path) would
    /// fail at the daemon after the whole module was already copied.
    #[test]
    fn move_gate_rejects_module_root_accepts_paths() {
        // Module root → rejected (the bug: would copy then fail).
        let root = RemoteEndpoint::parse("nas:/home/").expect("module root");
        assert!(
            !is_deletable_remote_path(&root),
            "a module root is not a moveable (deletable) source"
        );
        // A file / dir under a module → moveable.
        let file = RemoteEndpoint::parse("nas:/home/docs/a.txt").expect("file");
        assert!(is_deletable_remote_path(&file));
        let dir = RemoteEndpoint::parse("nas:/home/docs/").expect("dir");
        assert!(is_deletable_remote_path(&dir));
    }

    #[test]
    fn f3_del_display_confirming_always_shows() {
        use screens::f3::F3DelDisplay;
        let ep = RemoteEndpoint::parse("nas:/home/old.txt").expect("ep");
        let status = f3del::F3DelStatus::Confirming {
            module_endpoint: ep,
            rel_paths: vec!["old.txt".to_string()],
            label: "nas:/home/old.txt".to_string(),
            gate_path: Some("nas:/home/old.txt".to_string()),
        };
        // Shows even when the cursor is on a different path —
        // it's an active prompt, not a stale outcome.
        match f3_del_to_display(&status, Some("nas:/home/other")) {
            F3DelDisplay::Confirming { label } => assert_eq!(label, "nas:/home/old.txt"),
            other => panic!("expected Confirming, got {other:?}"),
        }
    }

    #[test]
    fn f3_del_display_single_is_path_gated_batch_is_not() {
        use screens::f3::F3DelDisplay;
        // Single delete: gate_path Some → hides when cursor moves.
        let single = f3del::F3DelStatus::Done {
            label: "nas:/home/old.txt".to_string(),
            files_deleted: 3,
            gate_path: Some("nas:/home/old.txt".to_string()),
            finished_at: Instant::now(),
        };
        assert!(matches!(
            f3_del_to_display(&single, Some("nas:/home/old.txt")),
            F3DelDisplay::Done {
                files_deleted: 3,
                ..
            }
        ));
        assert!(
            matches!(
                f3_del_to_display(&single, Some("nas:/home/other")),
                F3DelDisplay::Hidden
            ),
            "single-delete outcome hides once the cursor leaves the path"
        );

        // Batch delete: gate_path None → shows regardless of cursor.
        let batch = f3del::F3DelStatus::Done {
            label: "3 item(s)".to_string(),
            files_deleted: 9,
            gate_path: None,
            finished_at: Instant::now(),
        };
        assert!(matches!(
            f3_del_to_display(&batch, Some("nas:/home/anywhere")),
            F3DelDisplay::Done {
                files_deleted: 9,
                ..
            }
        ));
    }

    /// d-42: `g`/Home → SelectFirst, `G`/End → SelectLast.
    #[test]
    fn key_action_maps_jump_keys() {
        assert!(matches!(
            ka(&k(KeyCode::Char('g'))),
            Some(UserAction::SelectFirst)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Home)),
            Some(UserAction::SelectFirst)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('G'))),
            Some(UserAction::SelectLast)
        ));
        assert!(matches!(ka(&k(KeyCode::End)), Some(UserAction::SelectLast)));
    }

    // d-41: pure du-aggregate accumulator. With max_depth 0
    // the daemon emits a single root row, but folding by
    // max-bytes is robust if it ever emits children too.

    #[test]
    fn du_total_keeps_first_entry_when_alone() {
        assert_eq!(
            du_total_from_entries(None, 14_680_064, 8_442),
            Some((14_680_064, 8_442))
        );
    }

    #[test]
    fn du_total_keeps_largest_byte_entry() {
        // A later, smaller child entry must NOT replace the
        // larger root aggregate.
        let acc = du_total_from_entries(None, 1_000, 10);
        let acc = du_total_from_entries(acc, 9_000, 90); // bigger → replaces
        let acc = du_total_from_entries(acc, 500, 5); // smaller → ignored
        assert_eq!(acc, Some((9_000, 90)));
    }

    #[test]
    fn du_total_equal_bytes_keeps_existing() {
        let acc = du_total_from_entries(None, 1_000, 10);
        // Equal bytes → keep the first (>= guard).
        let acc = du_total_from_entries(acc, 1_000, 99);
        assert_eq!(acc, Some((1_000, 10)));
    }

    // d-41: the du display bridge gates on the cursor still
    // being on the queried path, so a stale total never
    // shows against the wrong row.

    #[test]
    fn f3_du_display_shows_done_only_for_matching_path() {
        use screens::f3::F3DuDisplay;
        let status = f3du::F3DuStatus::Done {
            path: "nas:/home/photos".to_string(),
            bytes: 2048,
            files: 7,
        };
        // Cursor still on the queried path → shown.
        match f3_du_to_display(&status, Some("nas:/home/photos")) {
            F3DuDisplay::Done { bytes, files } => {
                assert_eq!(bytes, 2048);
                assert_eq!(files, 7);
            }
            other => panic!("expected Done, got {other:?}"),
        }
        // Cursor moved elsewhere → hidden (stale).
        assert!(matches!(
            f3_du_to_display(&status, Some("nas:/home/docs")),
            F3DuDisplay::Hidden
        ));
        // No cursor spec at all → hidden.
        assert!(matches!(
            f3_du_to_display(&status, None),
            F3DuDisplay::Hidden
        ));
    }

    #[test]
    fn f3_du_display_idle_is_always_hidden() {
        use screens::f3::F3DuDisplay;
        assert!(matches!(
            f3_du_to_display(&f3du::F3DuStatus::Idle, Some("nas:/x")),
            F3DuDisplay::Hidden
        ));
    }

    #[test]
    fn f3_du_display_running_and_error_gate_on_path() {
        use screens::f3::F3DuDisplay;
        let running = f3du::F3DuStatus::Running {
            request_id: 1,
            path: "nas:/a".to_string(),
        };
        assert!(matches!(
            f3_du_to_display(&running, Some("nas:/a")),
            F3DuDisplay::Running
        ));
        assert!(matches!(
            f3_du_to_display(&running, Some("nas:/b")),
            F3DuDisplay::Hidden
        ));
        let error = f3du::F3DuStatus::Error {
            path: "nas:/a".to_string(),
            message: "boom".to_string(),
        };
        match f3_du_to_display(&error, Some("nas:/a")) {
            F3DuDisplay::Error { message } => assert_eq!(message, "boom"),
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(matches!(
            f3_du_to_display(&error, Some("nas:/b")),
            F3DuDisplay::Hidden
        ));
    }

    /// d-26 helper: build a fresh `AppState` for keystroke
    /// tests. Mirrors the boilerplate of
    /// `handle_verify_keystroke_returns_false_for_question_mark`.
    /// m2f-5: the F2 watch set dedups daemons by their host:port
    /// identity — parsed_remote first, then discovered — and includes
    /// the port (so same-host/different-port daemons stay distinct).
    #[test]
    fn f2_watched_endpoints_dedups_by_identity() {
        let mut app = make_test_app_state(Screen::F2);
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:9444:/m/").expect("parse"));
        let watched = f2_watched_endpoints(&app);
        // No discovery yet → just the launch daemon, port preserved.
        assert_eq!(watched.len(), 1);
        assert_eq!(watched[0].host_port_display(), "nas:9444");
    }

    /// m2f-9: the watched-identity set tracks discovery — a daemon
    /// appearing changes the set (so the loop knows to auto re-fan), and
    /// a re-report of the same daemons leaves it unchanged (so a steady
    /// discovery feed doesn't churn live streams). Identities are keyed
    /// by host:port, matching f2_watched_endpoints' dedup.
    #[test]
    fn f2_watched_identities_changes_when_a_daemon_appears() {
        let mut app = make_test_app_state(Screen::F2);
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:/home/").expect("launch"));
        let before = f2_watched_identities(&app);
        assert_eq!(
            before,
            ["nas".to_string()].into_iter().collect(),
            "only the launch daemon before discovery"
        );

        let skippy = blit_core::mdns::MdnsDiscoveredService {
            fullname: "skippy._blit._tcp.local.".to_string(),
            instance_name: "skippy".to_string(),
            hostname: "skippy.local.".to_string(),
            // Non-default port so the identity visibly carries it.
            port: 9050,
            addresses: vec![std::net::Ipv4Addr::new(192, 168, 1, 50)],
            properties: std::collections::HashMap::new(),
        };
        app.daemons
            .replace_from_discovery(std::slice::from_ref(&skippy), Instant::now());
        let after = f2_watched_identities(&app);
        assert!(after.contains("nas"), "launch daemon retained");
        // Discovered daemons resolve to their advertised <ip>:<port>.
        assert!(
            after.contains("192.168.1.50:9050"),
            "discovered daemon added"
        );
        assert_ne!(
            before, after,
            "appearance changes the set → triggers re-fan"
        );

        // Re-reporting the same daemon leaves the set unchanged → no
        // needless re-fan on a steady discovery feed.
        let steady = f2_watched_identities(&app);
        app.daemons
            .replace_from_discovery(std::slice::from_ref(&skippy), Instant::now());
        assert_eq!(
            steady,
            f2_watched_identities(&app),
            "stable feed → no change"
        );
    }

    /// e-8: the launch remote resolves CLI-first, then the
    /// `[daemon] default_remote` config, then nothing. An explicit flag
    /// always wins (even an empty one, preserving the existing
    /// parse-error path); a blank/whitespace config is treated as unset.
    #[test]
    fn resolve_launch_remote_prefers_cli_then_config() {
        // CLI flag present → used verbatim, config ignored.
        assert_eq!(
            resolve_launch_remote(Some("nas:/m/"), "skippy:/x/"),
            Some("nas:/m/".to_string())
        );
        // An explicit empty flag still counts as "the operator said so".
        assert_eq!(
            resolve_launch_remote(Some(""), "skippy:/x/"),
            Some(String::new())
        );
        // No flag → fall back to a non-blank config default.
        assert_eq!(
            resolve_launch_remote(None, "skippy:/x/"),
            Some("skippy:/x/".to_string())
        );
        // No flag + blank/whitespace config → mDNS-only (None).
        assert_eq!(resolve_launch_remote(None, ""), None);
        assert_eq!(resolve_launch_remote(None, "   "), None);
        // Config default is trimmed.
        assert_eq!(
            resolve_launch_remote(None, "  nas:/m/  "),
            Some("nas:/m/".to_string())
        );
    }

    /// m2f-7: a row's source-daemon identity round-trips to a
    /// connectable cancel endpoint — host:port preserved (so CancelJob
    /// reaches the right daemon), default port handled.
    #[test]
    fn cancel_endpoint_round_trips_daemon_identity() {
        let ep = cancel_endpoint("skippy:9001").expect("non-default port");
        assert_eq!(ep.host_port_display(), "skippy:9001");
        let ep = cancel_endpoint("nas").expect("default port");
        assert_eq!(ep.host, "nas");
        assert_eq!(ep.host_port_display(), "nas");
    }

    fn make_test_app_state(screen: Screen) -> AppState {
        AppState {
            current_screen: screen,
            parsed_remote: None,
            browse_target: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            f1_trigger: f1trigger::F1TriggerState::new(),
            f1_push: f1push::F1PushState::new(),
            f1_push_reply_tx: mpsc::channel::<F1PushReply>(1).0,
            f1_push_progress_tx: mpsc::channel::<F1PushProgress>(1).0,
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            transfers_refan_after_setup: false,
            f2_degraded_daemons: std::collections::BTreeSet::new(),
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
            f3_batch_pull: None,
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            f3_du: f3du::F3DuState::new(),
            f3_du_reply_tx: mpsc::channel::<F3DuReply>(1).0,
            f3_del: f3del::F3DelState::new(),
            f3_del_reply_tx: mpsc::channel::<F3DelReply>(1).0,
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
            ka(&k(KeyCode::Char('p'))),
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

    // d-55: F3 `m` mirror — key mapping + the destructive
    // confirm keystroke handler.

    /// `m` maps to F3MirrorBegin (distinct from `M`, the F4
    /// local mirror). Only the F3 dispatcher acts on it.
    #[test]
    fn key_action_maps_m_to_f3_mirror_begin() {
        assert!(matches!(
            ka(&k(KeyCode::Char('m'))),
            Some(UserAction::F3MirrorBegin)
        ));
        // `M` stays the F4 local mirror — case-distinct.
        assert!(matches!(
            ka(&k(KeyCode::Char('M'))),
            Some(UserAction::TransferMirror)
        ));
    }

    // d-58: F1 `t` trigger-transfer modal.

    #[test]
    fn key_action_maps_t_to_f1_trigger_begin() {
        assert!(matches!(
            ka(&k(KeyCode::Char('t'))),
            Some(UserAction::F1TriggerBegin)
        ));
    }

    /// Helper: an F1 app with the trigger modal open (source
    /// prefilled, dest typed).
    fn app_with_trigger_modal() -> AppState {
        let mut app = make_test_app_state(Screen::F1);
        app.f1_trigger.begin("nas:9031:/".to_string());
        // append a module path to the source
        app.f1_trigger.toggle_focus();
        for c in "home/docs".chars() {
            app.f1_trigger.push_char(c);
        }
        // back to dest, type a local path
        app.f1_trigger.toggle_focus();
        for c in "/tmp/out".chars() {
            app.f1_trigger.push_char(c);
        }
        assert!(app.f1_trigger.is_editing());
        app
    }

    #[test]
    fn handle_f1_trigger_keystroke_esc_cancels() {
        let mut app = app_with_trigger_modal();
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Esc), &mut app));
        assert!(!app.f1_trigger.is_editing(), "Esc closes the modal");
    }

    /// d-61: a local source + remote dest + Copy kind commits a
    /// push (local→remote). The push status shows on F1 — no jump
    /// to F3 (that's the pull direction). Needs a tokio reactor for
    /// the detached push task (races to a non-existent daemon).
    #[tokio::test]
    async fn handle_f1_trigger_keystroke_enter_starts_push_for_local_source() {
        let mut app = make_test_app_state(Screen::F1);
        // Local-path source (does NOT parse as a remote endpoint),
        // remote dest → push direction.
        app.f1_trigger.begin("/tmp/src".to_string());
        for c in "nas:9031:/home/".chars() {
            app.f1_trigger.push_char(c);
        }
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(!app.f1_trigger.is_editing(), "commit closes the modal");
        assert!(app.f1_push.is_running(), "a push launched");
        assert_eq!(
            app.current_screen,
            Screen::F1,
            "push status shows on F1, no jump to F3"
        );
        assert!(!app.f3_pull.is_running(), "push is not a pull");
    }

    /// d-61 R2 (reviewer reopen): a malformed remote-shaped source
    /// (`nas:9031:/home` — missing the module trailing slash) fails
    /// `RemoteEndpoint::parse`, but it must NOT be misclassified as
    /// a local push source. `parse_transfer_endpoint` returns Err
    /// for `:/`-shaped inputs, so the commit drops: neither a push
    /// nor a pull starts.
    #[test]
    fn handle_f1_trigger_keystroke_malformed_remote_source_does_not_push() {
        let mut app = make_test_app_state(Screen::F1);
        // Looks remote but is invalid (module root needs trailing
        // slash); dest is a valid remote.
        app.f1_trigger.begin("nas:9031:/home".to_string());
        for c in "other:9031:/backup/".chars() {
            app.f1_trigger.push_char(c);
        }
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(
            !app.f1_push.is_running(),
            "a malformed remote source must not become a local push"
        );
        assert!(!app.f3_pull.is_running(), "and it isn't a pull either");
        // d-62: the modal stays open with an inline error (instead
        // of silently closing) so the operator can fix the typo.
        match app.f1_trigger.status() {
            f1trigger::F1TriggerStatus::Editing { error, .. } => {
                assert!(error.is_some(), "a validation error is shown");
            }
            other => panic!("modal should stay open with an error, got {other:?}"),
        }
    }

    /// d-61 R3 (reviewer reopen): a bare-host push destination
    /// (`nas:9031`) parses as a remote endpoint but is
    /// `RemotePath::Discovery` — no module/root — which the push
    /// client rejects. The dest-shape gate must refuse it BEFORE
    /// starting a push, so no Running footer appears for a transfer
    /// that can't succeed.
    #[test]
    fn handle_f1_trigger_keystroke_bare_host_dest_does_not_push() {
        let mut app = make_test_app_state(Screen::F1);
        app.f1_trigger.begin("/tmp/src".to_string());
        for c in "nas:9031".chars() {
            app.f1_trigger.push_char(c);
        }
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(
            !app.f1_push.is_running(),
            "a bare-host (module-less) dest must not start a push"
        );
        assert!(!app.f3_pull.is_running());
    }

    /// d-65: a destructive push (mirror/move local→remote) opens
    /// the confirm gate on Enter — it does NOT launch immediately,
    /// and a `y` then launches it.
    #[tokio::test]
    async fn handle_f1_trigger_mirror_push_confirms_then_launches() {
        let mut app = make_test_app_state(Screen::F1);
        app.f1_trigger.begin("/tmp/src".to_string());
        for c in "nas:9031:/home/".chars() {
            app.f1_trigger.push_char(c);
        }
        app.f1_trigger.cycle_kind(true); // Copy → Mirror
                                         // Enter opens the confirm gate — nothing launches yet.
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(app.f1_trigger.is_confirming(), "destructive push confirms");
        assert!(!app.f1_push.is_running(), "no push until y");
        // y launches the mirror push.
        assert!(handle_f1_trigger_confirm_keystroke(
            &k(KeyCode::Char('y')),
            &mut app
        ));
        assert!(app.f1_push.is_running(), "y launches the mirror push");
        assert!(!app.f1_trigger.is_editing(), "modal closed on launch");
    }

    /// d-65: `n`/`Esc` at the confirm aborts back to editing — no
    /// push, modal stays open.
    #[test]
    fn handle_f1_trigger_confirm_cancel_returns_to_editing() {
        for cancel in [KeyCode::Char('n'), KeyCode::Esc] {
            let mut app = make_test_app_state(Screen::F1);
            app.f1_trigger.begin("/tmp/src".to_string());
            for c in "nas:9031:/home/".chars() {
                app.f1_trigger.push_char(c);
            }
            app.f1_trigger.cycle_kind(true); // Mirror
            handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app);
            assert!(app.f1_trigger.is_confirming());
            assert!(handle_f1_trigger_confirm_keystroke(&k(cancel), &mut app));
            assert!(!app.f1_push.is_running(), "{cancel:?}: no push");
            assert!(app.f1_trigger.is_editing(), "back to editing");
            assert!(!app.f1_trigger.is_confirming(), "confirm cleared");
        }
    }

    /// d-65: a move push confirms then launches (the local-source
    /// delete happens in the spawned task after the push lands).
    #[tokio::test]
    async fn handle_f1_trigger_move_push_confirms_then_launches() {
        let mut app = make_test_app_state(Screen::F1);
        app.f1_trigger.begin("/tmp/src".to_string());
        for c in "nas:9031:/home/".chars() {
            app.f1_trigger.push_char(c);
        }
        app.f1_trigger.cycle_kind(true); // Mirror
        app.f1_trigger.cycle_kind(true); // Move
        handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app);
        assert!(app.f1_trigger.is_confirming());
        handle_f1_trigger_confirm_keystroke(&k(KeyCode::Char('y')), &mut app);
        assert!(app.f1_push.is_running(), "y launches the move push");
    }

    // d-66: F4 destructive clear-history confirm gate.

    #[test]
    fn handle_profile_clear_confirm_cancel_keeps_history() {
        for cancel in [KeyCode::Char('n'), KeyCode::Char('N'), KeyCode::Esc] {
            let mut app = make_test_app_state(Screen::F4);
            app.profile.begin_clear_confirm();
            assert!(app.profile.is_confirming_clear());
            assert!(
                handle_profile_clear_confirm_keystroke(&k(cancel), &mut app),
                "{cancel:?}: consumed"
            );
            assert!(
                !app.profile.is_confirming_clear(),
                "{cancel:?}: confirm dropped without clearing"
            );
        }
    }

    #[test]
    fn handle_profile_clear_confirm_swallows_unrelated_keys() {
        // A stray letter inside the modal must NOT leak through to
        // the dispatcher AND must leave the confirm armed (the
        // operator hasn't answered yet).
        let mut app = make_test_app_state(Screen::F4);
        app.profile.begin_clear_confirm();
        assert!(handle_profile_clear_confirm_keystroke(
            &k(KeyCode::Char('x')),
            &mut app
        ));
        assert!(app.profile.is_confirming_clear(), "still awaiting y/N");
    }

    #[test]
    fn handle_profile_clear_confirm_lets_escape_hatches_through() {
        // Ctrl-c (quit), F-keys (pane nav), and `?` (help) must
        // fall through so the operator is never trapped mid-confirm.
        let ctrl_c = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        for (key, what) in [
            (ctrl_c, "ctrl-c"),
            (k(KeyCode::F(2)), "F2"),
            (k(KeyCode::Char('?')), "?"),
        ] {
            let mut app = make_test_app_state(Screen::F4);
            app.profile.begin_clear_confirm();
            assert!(
                !handle_profile_clear_confirm_keystroke(&key, &mut app),
                "{what}: falls through"
            );
            assert!(
                app.profile.is_confirming_clear(),
                "{what}: confirm untouched"
            );
        }
    }

    /// `y` runs the clear and drops the confirm. The clear targets
    /// a tempdir config override so it never touches the operator's
    /// real `perf_local.jsonl`; the spawned re-fetch needs a runtime.
    #[tokio::test]
    async fn handle_profile_clear_confirm_y_clears_and_disarms() {
        let tmp = tempfile::tempdir().expect("tmp");
        blit_core::config::set_config_dir(tmp.path());
        let mut app = make_test_app_state(Screen::F4);
        app.profile.begin_clear_confirm();
        assert!(handle_profile_clear_confirm_keystroke(
            &k(KeyCode::Char('y')),
            &mut app
        ));
        assert!(!app.profile.is_confirming_clear(), "y disarms the confirm");
        blit_core::config::clear_config_dir_override();
    }

    /// d-65 R2: the mirror push must require a complete source
    /// scan — a partial local enumeration could otherwise make
    /// valid remote files look extraneous and get purged. Copy/move
    /// never delete at the dest, so they leave the gate off.
    #[test]
    fn build_f1_push_execution_gates_mirror_purge_on_complete_scan() {
        use blit_core::generated::MirrorMode;
        let remote = RemoteEndpoint::parse("nas:9031:/home/").expect("remote");

        let mirror = build_f1_push_execution(
            std::path::PathBuf::from("/tmp/src"),
            remote.clone(),
            f3pull::PullKind::Mirror,
        );
        assert!(mirror.mirror_mode, "mirror enables the dest purge");
        assert_eq!(mirror.mirror_kind, MirrorMode::All);
        assert!(
            mirror.require_complete_scan,
            "mirror purge must be gated on a complete source scan",
        );

        for kind in [f3pull::PullKind::Copy, f3pull::PullKind::Move] {
            let ex =
                build_f1_push_execution(std::path::PathBuf::from("/tmp/src"), remote.clone(), kind);
            assert!(!ex.mirror_mode, "{kind:?}: never purges the dest");
            assert_eq!(ex.mirror_kind, MirrorMode::Off, "{kind:?}");
            assert!(
                !ex.require_complete_scan,
                "{kind:?}: no dest delete, so no scan gate needed",
            );
        }
    }

    /// d-68: a remote source + remote dest copy routes to the
    /// delegated path (reusing the F1 push footer, flagged
    /// `delegated`) — NOT the remote→local pull machine.
    #[tokio::test]
    async fn plan_f1_trigger_remote_to_remote_copy_delegates() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/sub/",
            "skippy:/backup/",
            f3pull::PullKind::Copy,
            false,
        );
        assert!(
            matches!(out, TriggerOutcome::Launched),
            "delegated copy launches"
        );
        match app.f1_push.status() {
            f1push::F1PushStatus::Running { delegated, .. } => {
                assert!(*delegated, "remote→remote uses the delegated lifecycle")
            }
            other => panic!("expected delegated Running, got {other:?}"),
        }
        // The pull machine must NOT have been engaged (no mis-route
        // of the remote dest as a local path).
        assert!(
            !app.f3_pull.is_running(),
            "remote dest must not start a remote→local pull"
        );
    }

    /// d-71: remote→remote MOVE confirms (destructive — deletes the
    /// remote source after the copy) then launches a delegated move.
    #[tokio::test]
    async fn plan_f1_trigger_remote_to_remote_move_confirms_then_launches() {
        let mut app = make_test_app_state(Screen::F1);
        // Source has a subpath so it's a deletable (non-module-root)
        // remote path.
        let unconfirmed = plan_f1_trigger(
            &mut app,
            "nas:/photos/2024/",
            "skippy:/backup/",
            f3pull::PullKind::Move,
            false,
        );
        assert!(
            matches!(unconfirmed, TriggerOutcome::NeedsConfirm),
            "move needs confirm"
        );
        assert!(!app.f1_push.is_running(), "no launch before confirm");

        let confirmed = plan_f1_trigger(
            &mut app,
            "nas:/photos/2024/",
            "skippy:/backup/",
            f3pull::PullKind::Move,
            true,
        );
        assert!(matches!(confirmed, TriggerOutcome::Launched));
        match app.f1_push.status() {
            f1push::F1PushStatus::Running {
                delegated, kind, ..
            } => {
                assert!(*delegated);
                assert_eq!(*kind, f3pull::PullKind::Move);
            }
            other => panic!("expected delegated move Running, got {other:?}"),
        }
    }

    /// d-71: the move confirm detail names the right victim by
    /// direction — a remote source (delegated move) deletes the
    /// REMOTE source; a local source (push move) deletes the LOCAL
    /// source.
    #[test]
    fn f1_trigger_prompt_move_detail_follows_source_direction() {
        let to_move = |source: &str| {
            let mut t = f1trigger::F1TriggerState::new();
            t.begin(source.to_string());
            for c in "skippy:/backup/".chars() {
                t.push_char(c);
            }
            t.cycle_kind(true); // copy → mirror
            t.cycle_kind(true); // mirror → move
            t.begin_confirm();
            f1_trigger_prompt(&t).expect("prompt").confirm_detail
        };
        assert_eq!(
            to_move("nas:/photos/2024/"),
            Some("deletes the remote source")
        );
        assert_eq!(to_move("/tmp/src"), Some("deletes the local source"));
    }

    /// d-71 R2: a delegated transfer must resolve the destination like
    /// the CLI BEFORE launch — a non-trailing-slash source + a
    /// container dest appends the source basename
    /// (`nas:/photos/2024` → `skippy:/backup/` ⇒ `skippy:/backup/2024`).
    /// Without it a move would copy into the dest root and then delete
    /// the wrong source (data loss). We assert via the launched run's
    /// label, which is the resolved destination.
    #[tokio::test]
    async fn plan_f1_trigger_delegated_move_resolves_container_dest() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/2024", // no trailing slash → basename appends
            "skippy:/backup/",  // container
            f3pull::PullKind::Move,
            true, // pre-confirmed so it launches
        );
        assert!(matches!(out, TriggerOutcome::Launched));
        match app.f1_push.status() {
            f1push::F1PushStatus::Running { label, .. } => {
                assert!(
                    label.contains("backup/2024"),
                    "dest resolved to <container>/<basename>, got {label:?}"
                );
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    /// d-72: the local→remote PUSH branch must resolve the dest like
    /// the CLI too — a non-trailing-slash local source + a container
    /// remote dest nests under `<dest>/<basename>`. (Push move deletes
    /// the local source, so writing to the wrong remote target would
    /// be data loss.) Asserted via the launched push's label.
    #[tokio::test]
    async fn plan_f1_trigger_push_resolves_container_dest() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "/home/me/work", // local source, no trailing slash
            "skippy:/backup/",
            f3pull::PullKind::Copy,
            false,
        );
        assert!(matches!(out, TriggerOutcome::Launched));
        match app.f1_push.status() {
            f1push::F1PushStatus::Running { label, .. } => assert!(
                label.contains("backup/work"),
                "push dest resolved to <container>/<basename>, got {label:?}"
            ),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    /// d-71 R3: the destructive push MOVE case the reviewer called out
    /// — `/tmp/src -> nas:/home/` must resolve to `nas:/home/src`
    /// BEFORE launch, or the post-push local-source delete loses data
    /// to the wrong remote target.
    #[tokio::test]
    async fn plan_f1_trigger_push_move_resolves_container_dest() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "/tmp/src",
            "nas:/home/",
            f3pull::PullKind::Move,
            true, // pre-confirmed (move is destructive)
        );
        assert!(matches!(out, TriggerOutcome::Launched));
        match app.f1_push.status() {
            f1push::F1PushStatus::Running { label, .. } => assert!(
                label.contains("home/src"),
                "push move resolved to nas:/home/src, got {label:?}"
            ),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    /// d-71 R3: a trailing-slash ("copy contents") local source on the
    /// PUSH branch must NOT append the basename — dest stays the root.
    #[tokio::test]
    async fn plan_f1_trigger_push_trailing_source_keeps_dest_root() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "/tmp/src/",
            "nas:/home/",
            f3pull::PullKind::Copy,
            false,
        );
        assert!(matches!(out, TriggerOutcome::Launched));
        match app.f1_push.status() {
            f1push::F1PushStatus::Running { label, .. } => assert!(
                !label.contains("src"),
                "copy-contents keeps the dest root, got {label:?}"
            ),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    /// d-71 R2: a trailing-slash ("copy contents") source must NOT
    /// append the basename — the dest stays the container root.
    #[tokio::test]
    async fn plan_f1_trigger_delegated_trailing_source_keeps_dest_root() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/2024/", // trailing slash → contents
            "skippy:/backup/",
            f3pull::PullKind::Copy,
            false,
        );
        assert!(matches!(out, TriggerOutcome::Launched));
        match app.f1_push.status() {
            f1push::F1PushStatus::Running { label, .. } => {
                assert!(
                    !label.contains("2024"),
                    "copy-contents keeps the dest root, got {label:?}"
                );
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    /// d-71: a delegated move whose SOURCE is a module root is refused
    /// up front — there's no single path to delete (mirrors the F3
    /// remote→local move guard, d-60).
    #[test]
    fn plan_f1_trigger_remote_to_remote_move_module_root_source_rejected() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/", // module root — no subpath
            "skippy:/backup/",
            f3pull::PullKind::Move,
            false,
        );
        match out {
            TriggerOutcome::Rejected(msg) => assert!(msg.contains("module root"), "{msg}"),
            other => panic!("expected Rejected, got {other:?}"),
        }
        assert!(!app.f1_push.is_running());
    }

    /// d-70: remote→remote MIRROR is destructive (purges the dest), so
    /// it routes through the trigger's y/N confirm: unconfirmed →
    /// NeedsConfirm (no launch); confirmed → delegated launch with
    /// kind=Mirror.
    #[tokio::test]
    async fn plan_f1_trigger_remote_to_remote_mirror_confirms_then_launches() {
        let mut app = make_test_app_state(Screen::F1);
        let unconfirmed = plan_f1_trigger(
            &mut app,
            "nas:/photos/",
            "skippy:/backup/",
            f3pull::PullKind::Mirror,
            false,
        );
        assert!(
            matches!(unconfirmed, TriggerOutcome::NeedsConfirm),
            "mirror needs confirm"
        );
        assert!(!app.f1_push.is_running(), "no launch before confirm");

        let confirmed = plan_f1_trigger(
            &mut app,
            "nas:/photos/",
            "skippy:/backup/",
            f3pull::PullKind::Mirror,
            true,
        );
        assert!(matches!(confirmed, TriggerOutcome::Launched));
        match app.f1_push.status() {
            f1push::F1PushStatus::Running {
                delegated, kind, ..
            } => {
                assert!(*delegated, "delegated lifecycle");
                assert_eq!(*kind, f3pull::PullKind::Mirror, "carries the mirror kind");
            }
            other => panic!("expected delegated mirror Running, got {other:?}"),
        }
    }

    /// d-70: the delegated execution builder pins the mirror option.
    /// Mirror sets `mirror_mode` but leaves `require_complete_scan`
    /// OFF — matching the CLI's delegated path (the daemons scan, not
    /// this client). Copy sets neither.
    #[test]
    fn build_delegated_execution_mirror_options() {
        let src = RemoteEndpoint::parse("nas:/photos/").expect("src");
        let dst = RemoteEndpoint::parse("skippy:/backup/").expect("dst");

        let mirror = build_delegated_execution(src.clone(), dst.clone(), f3pull::PullKind::Mirror);
        assert!(mirror.options.mirror_mode, "mirror enables the dest purge");
        assert!(
            !mirror.options.require_complete_scan,
            "delegated mirror leaves scan-gate off (daemons scan, not the client)"
        );

        let copy = build_delegated_execution(src, dst, f3pull::PullKind::Copy);
        assert!(!copy.options.mirror_mode, "copy never purges");
        assert!(!copy.options.require_complete_scan);
    }

    /// d-68 R2: with a remote source, a remote-*shaped* but invalid
    /// destination (module path missing its trailing slash) must be
    /// rejected — NOT fall through into a remote→local pull that
    /// treats `skippy:/backup` as a literal local directory.
    #[test]
    fn plan_f1_trigger_remote_source_rejects_malformed_remote_dest() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/",
            "skippy:/backup", // no trailing slash → remote-shaped parse error
            f3pull::PullKind::Copy,
            false,
        );
        match out {
            TriggerOutcome::Rejected(msg) => assert!(msg.contains("invalid destination"), "{msg}"),
            other => panic!("expected Rejected, got {other:?}"),
        }
        assert!(
            !app.f3_pull.is_running(),
            "must not start a local-path pull"
        );
        assert!(!app.f1_push.is_running(), "must not delegate");
    }

    /// d-68 R2: a remote source with a genuine *local* destination
    /// still routes to the remote→local pull (the Ok(Local) arm
    /// falls through, unchanged by the malformed-dest guard). Tokio:
    /// the pull path spawns a task.
    #[tokio::test]
    async fn plan_f1_trigger_remote_source_local_dest_still_pulls() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/",
            "/tmp/pulled",
            f3pull::PullKind::Copy,
            false,
        );
        assert!(
            matches!(out, TriggerOutcome::Launched),
            "remote→local copy launches"
        );
        assert!(
            app.f3_pull.is_running(),
            "engages the remote→local pull machine"
        );
    }

    /// d-68 R3: a bare relative destination (`backup`) parses as a
    /// remote *discovery* endpoint, but for a remote source it's an
    /// ordinary local pull destination — it must fall through to the
    /// remote→local pull, NOT be rejected as a non-module delegated
    /// dest (the R2 over-correction the reviewer flagged).
    #[tokio::test]
    async fn plan_f1_trigger_remote_source_bare_dest_pulls_not_delegates() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/",
            "backup",
            f3pull::PullKind::Copy,
            false,
        );
        assert!(
            matches!(out, TriggerOutcome::Launched),
            "bare local dest pulls, not rejected"
        );
        assert!(app.f3_pull.is_running(), "remote→local pull engaged");
        assert!(!app.f1_push.is_running(), "must not delegate a bare dest");
    }

    /// d-68 R4: a Windows drive local destination (`C:/tmp/out`)
    /// contains `:/` but is a local path — it must reach the
    /// remote→local pull, not be rejected as a remote-shaped typo.
    #[tokio::test]
    async fn plan_f1_trigger_remote_source_windows_local_dest_pulls() {
        let mut app = make_test_app_state(Screen::F1);
        let out = plan_f1_trigger(
            &mut app,
            "nas:/photos/",
            "C:/tmp/out",
            f3pull::PullKind::Copy,
            false,
        );
        assert!(
            matches!(out, TriggerOutcome::Launched),
            "Windows local dest pulls"
        );
        assert!(app.f3_pull.is_running(), "remote→local pull engaged");
        assert!(!app.f1_push.is_running(), "must not delegate");
    }

    #[test]
    fn handle_f1_trigger_keystroke_tab_toggles_focus() {
        let mut app = app_with_trigger_modal();
        // Focus is on dest after the helper; Tab → source.
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Tab), &mut app));
        match app.f1_trigger.status() {
            f1trigger::F1TriggerStatus::Editing { focus, .. } => {
                assert_eq!(*focus, f1trigger::TriggerField::Source);
            }
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    #[test]
    fn handle_f1_trigger_keystroke_chars_edit_dest() {
        let mut app = make_test_app_state(Screen::F1);
        app.f1_trigger.begin("nas:9031:/home".to_string());
        for c in "/x".chars() {
            assert!(handle_f1_trigger_keystroke(&k(KeyCode::Char(c)), &mut app));
        }
        match app.f1_trigger.status() {
            f1trigger::F1TriggerStatus::Editing { dest, .. } => assert_eq!(dest, "/x"),
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    /// `Enter` with a valid remote source + dest launches a pull
    /// on the F3 machine and jumps to F3. Needs a tokio reactor
    /// for the detached spawn (races to a non-existent daemon,
    /// ignored).
    #[tokio::test]
    async fn handle_f1_trigger_keystroke_enter_launches_pull_and_jumps_to_f3() {
        let mut app = app_with_trigger_modal();
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(!app.f1_trigger.is_editing(), "commit closes the modal");
        assert!(app.f3_pull.is_running(), "the pull launched");
        assert_eq!(app.current_screen, Screen::F3, "jumped to F3 to watch");
    }

    /// d-59: Up/Down flips the mode; committing a mirror routes
    /// through the F3 destructive confirm gate (NOT a direct
    /// launch) and jumps to F3 so the operator confirms y/N there.
    #[test]
    fn handle_f1_trigger_keystroke_mirror_routes_to_f3_confirm() {
        let mut app = app_with_trigger_modal();
        // Flip to mirror, then commit.
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Up), &mut app));
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(!app.f1_trigger.is_editing(), "commit closes the modal");
        assert!(
            app.f3_pull.is_confirming_destructive(),
            "a mirror waits at the F3 confirm gate, not a direct launch"
        );
        assert!(
            !app.f3_pull.is_running(),
            "no PullSync until the operator confirms"
        );
        assert_eq!(app.current_screen, Screen::F3, "jumped to F3 to confirm");
    }

    /// d-60: cycling to move (Up×2) and committing routes through
    /// the F3 confirm gate as a Move-kind op.
    #[test]
    fn handle_f1_trigger_keystroke_move_routes_to_f3_confirm() {
        let mut app = app_with_trigger_modal();
        // Copy → Mirror → Move.
        handle_f1_trigger_keystroke(&k(KeyCode::Up), &mut app);
        handle_f1_trigger_keystroke(&k(KeyCode::Up), &mut app);
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(
            app.f3_pull.is_confirming_destructive(),
            "move waits at confirm"
        );
        assert_eq!(app.current_screen, Screen::F3);
    }

    /// d-60 (data-loss guard): a move whose source is a module
    /// root is refused — like F3 `v`, the daemon rejects empty/root
    /// purge paths, so it must not even open the confirm.
    ///
    /// d-60 R2 (reviewer reopen): the source MUST be a *valid*
    /// module-root endpoint (`nas:9031:/home/` — module syntax
    /// requires the trailing slash) so the test exercises the
    /// `is_deletable_remote_path` gate rather than tripping a parse
    /// failure (the round-1 `nas:9031:/home` didn't parse, so the
    /// test passed for the wrong reason). The paired subpath test
    /// below proves a normal move source still reaches the confirm.
    #[test]
    fn handle_f1_trigger_keystroke_move_rejects_module_root_source() {
        // Sanity: this is a parseable endpoint (a module root), so
        // we actually reach + exercise the d-60 gate.
        assert!(
            RemoteEndpoint::parse("nas:9031:/home/").is_ok(),
            "test source must parse, else it gates for the wrong reason"
        );

        let mut app = make_test_app_state(Screen::F1);
        // Source = a module root (rel path empty), dest = local.
        app.f1_trigger.begin("nas:9031:/home/".to_string());
        for c in "/tmp/out".chars() {
            app.f1_trigger.push_char(c);
        }
        // Cycle to move.
        app.f1_trigger.cycle_kind(true);
        app.f1_trigger.cycle_kind(true);
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        // Module root → the d-60 gate refuses it: no confirm opens.
        assert!(
            !app.f3_pull.is_confirming_destructive(),
            "module-root move source is refused by the gate"
        );
        assert_eq!(app.current_screen, Screen::F1, "stays on F1 (no jump)");
    }

    /// d-60 R2: the paired positive case — a move whose source is a
    /// real subpath (not a module root) DOES reach the F3
    /// destructive confirm, proving the gate refuses only roots.
    #[test]
    fn handle_f1_trigger_keystroke_move_subpath_reaches_confirm() {
        let mut app = make_test_app_state(Screen::F1);
        app.f1_trigger.begin("nas:9031:/home/docs".to_string());
        for c in "/tmp/out".chars() {
            app.f1_trigger.push_char(c);
        }
        app.f1_trigger.cycle_kind(true);
        app.f1_trigger.cycle_kind(true);
        assert!(handle_f1_trigger_keystroke(&k(KeyCode::Enter), &mut app));
        assert!(
            app.f3_pull.is_confirming_destructive(),
            "a real subpath move source reaches the confirm gate"
        );
        assert_eq!(app.current_screen, Screen::F3, "jumped to F3 to confirm");
    }

    #[test]
    fn handle_f1_trigger_keystroke_lets_escapes_bubble() {
        let mut app = app_with_trigger_modal();
        assert!(!handle_f1_trigger_keystroke(
            &k(KeyCode::Char('?')),
            &mut app
        ));
        let ctrl_c = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert!(!handle_f1_trigger_keystroke(&ctrl_c, &mut app));
        assert!(!handle_f1_trigger_keystroke(&k(KeyCode::F(2)), &mut app));
    }

    /// Helper: an F3 app sitting on the mirror confirm gate
    /// (mirror prompt committed with a dest).
    fn app_with_mirror_confirm() -> AppState {
        let mut app = make_test_app_state(Screen::F3);
        let source = RemoteEndpoint::parse("nas:/photos/2024").expect("endpoint");
        app.f3_pull.begin_mirror(source);
        for c in "/tmp/out".chars() {
            app.f3_pull.push_char(c);
        }
        app.f3_pull.begin_run();
        assert!(app.f3_pull.is_confirming_destructive());
        app
    }

    // `y` calls `spawn_f3_pull`, which needs a tokio reactor
    // for the detached task — run under a runtime. The task
    // races off to a non-existent daemon and is ignored; we
    // only assert the synchronous state transition to Running.
    /// d-57: a move sits on the same destructive confirm gate; `y`
    /// launches it as a `Move`-kind run. Needs a tokio reactor for
    /// the detached spawn (the task races to a non-existent daemon
    /// and is ignored).
    #[tokio::test]
    async fn handle_f3_destructive_confirm_keystroke_y_launches_move() {
        let mut app = make_test_app_state(Screen::F3);
        let source = RemoteEndpoint::parse("nas:/photos/2024").expect("endpoint");
        app.f3_pull.begin_move(source);
        for c in "/tmp/out".chars() {
            app.f3_pull.push_char(c);
        }
        app.f3_pull.begin_run();
        assert!(app.f3_pull.is_confirming_destructive());
        assert!(handle_f3_destructive_confirm_keystroke(
            &k(KeyCode::Char('y')),
            &mut app
        ));
        match app.f3_pull.status() {
            f3pull::F3PullStatus::Running { kind, .. } => {
                assert_eq!(*kind, f3pull::PullKind::Move, "y launches a move run");
            }
            other => panic!("expected Running move, got {other:?}"),
        }
    }

    // `y` calls `spawn_f3_pull`, which needs a tokio reactor
    // for the detached task — run under a runtime. The task
    // races off to a non-existent daemon and is ignored; we
    // only assert the synchronous state transition to Running.
    #[tokio::test]
    async fn handle_f3_destructive_confirm_keystroke_y_launches() {
        let mut app = app_with_mirror_confirm();
        assert!(handle_f3_destructive_confirm_keystroke(
            &k(KeyCode::Char('y')),
            &mut app
        ));
        assert!(app.f3_pull.is_running(), "y confirms and launches");
    }

    #[test]
    fn handle_f3_destructive_confirm_keystroke_n_and_esc_cancel() {
        for cancel in [KeyCode::Char('n'), KeyCode::Char('N'), KeyCode::Esc] {
            let mut app = app_with_mirror_confirm();
            assert!(handle_f3_destructive_confirm_keystroke(
                &k(cancel),
                &mut app
            ));
            assert!(
                matches!(app.f3_pull.status(), f3pull::F3PullStatus::Idle),
                "{cancel:?} aborts the mirror"
            );
        }
    }

    /// Modal: a stray key during the destructive confirm is
    /// swallowed (returns true) but changes nothing — no
    /// prompt-stacking or cursor moves mid-confirm.
    #[test]
    fn handle_f3_destructive_confirm_keystroke_swallows_other_keys() {
        let mut app = app_with_mirror_confirm();
        assert!(handle_f3_destructive_confirm_keystroke(
            &k(KeyCode::Char('p')),
            &mut app
        ));
        assert!(app.f3_pull.is_confirming_destructive(), "still confirming");
    }

    /// d-55 R2 regression (reviewer reopen): the F3 mirror must
    /// build a mirror-ENABLED wire spec, not just carry the
    /// post-pull purge flag. The daemon computes the delete list
    /// from `TransferOperationSpec.mirror_mode`, which
    /// `build_spec_from_options` derives from `options.mirror_mode`
    /// — so a mirror whose options say `mirror_mode = false` would
    /// get `MirrorMode::Off` and silently behave like a plain pull
    /// (no deletions). Assert the spec is non-Off for a mirror and
    /// Off for a copy.
    #[test]
    fn f3_mirror_options_build_mirror_enabled_spec() {
        use blit_core::generated::MirrorMode;
        use blit_core::remote::pull::RemotePullClient;
        let endpoint = RemoteEndpoint::parse("nas:/photos/2024").expect("endpoint");

        let mirror_spec = RemotePullClient::build_spec_from_options(
            &endpoint,
            &f3_pull_options(f3pull::PullKind::Mirror),
        )
        .expect("mirror spec");
        assert_ne!(
            mirror_spec.mirror_mode,
            MirrorMode::Off as i32,
            "a mirror must ask the daemon to compute deletions"
        );
        // No filter / no delete-all scope → FilteredSubset (the
        // build_spec default for mirror_mode without delete_all).
        assert_eq!(mirror_spec.mirror_mode, MirrorMode::FilteredSubset as i32);

        let copy_spec = RemotePullClient::build_spec_from_options(
            &endpoint,
            &f3_pull_options(f3pull::PullKind::Copy),
        )
        .expect("copy spec");
        assert_eq!(
            copy_spec.mirror_mode,
            MirrorMode::Off as i32,
            "a plain pull must never request deletions"
        );
    }

    /// d-57 (data-loss guard): a move MUST set
    /// `require_complete_scan` so the daemon refuses a partial
    /// source scan — otherwise files skipped by an incomplete
    /// scan would survive the copy but be deleted with the rest
    /// of the source. A move is NOT a mirror, so its spec stays
    /// `MirrorMode::Off` (the source delete is a separate purge,
    /// not a destination-purge mirror).
    #[test]
    fn f3_move_options_require_complete_scan_and_not_mirror() {
        use blit_core::generated::MirrorMode;
        use blit_core::remote::pull::RemotePullClient;
        let move_opts = f3_pull_options(f3pull::PullKind::Move);
        assert!(
            move_opts.require_complete_scan,
            "move must refuse a partial scan before deleting the source"
        );
        assert!(!move_opts.mirror_mode, "move is not a destination mirror");

        let endpoint = RemoteEndpoint::parse("nas:/photos/2024").expect("endpoint");
        let spec =
            RemotePullClient::build_spec_from_options(&endpoint, &move_opts).expect("move spec");
        assert_eq!(
            spec.mirror_mode,
            MirrorMode::Off as i32,
            "move's source delete is a separate purge, not a mirror"
        );
        assert!(
            spec.require_complete_scan,
            "the scan-complete guard must reach the wire spec"
        );
        // Copy/mirror never set the scan guard.
        assert!(!f3_pull_options(f3pull::PullKind::Copy).require_complete_scan);
        assert!(!f3_pull_options(f3pull::PullKind::Mirror).require_complete_scan);
    }

    /// d-56/d-57: the bridge carries the past-tense verb + the
    /// destructive-phase delete count from the pull state into the
    /// renderer-facing Done display, so the footer can report
    /// "mirrored … · N deleted".
    #[test]
    fn f3_pull_to_display_done_carries_verb_and_deleted() {
        use screens::f3::F3PullDisplay;
        let mut s = f3pull::F3PullState::new();
        let source = RemoteEndpoint::parse("nas:/photos/2024").expect("endpoint");
        s.begin_mirror(source);
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        s.begin_run();
        let launch = s.confirm_destructive().expect("confirm");
        s.apply_done(launch.request_id, 7, 700, 3, Instant::now());
        match f3_pull_to_display(s.status()) {
            F3PullDisplay::Done { verb, deleted, .. } => {
                assert_eq!(verb, "mirrored", "bridge maps the kind to the past verb");
                assert_eq!(deleted, 3, "bridge preserves the purge delete count");
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn handle_f3_destructive_confirm_keystroke_lets_escapes_bubble() {
        // `?` (help), Ctrl-c (quit), F-keys (pane nav) must
        // fall through to the dispatcher.
        let mut app = app_with_mirror_confirm();
        assert!(!handle_f3_destructive_confirm_keystroke(
            &k(KeyCode::Char('?')),
            &mut app
        ));
        let ctrl_c = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert!(!handle_f3_destructive_confirm_keystroke(&ctrl_c, &mut app));
        for n in 1..=4 {
            assert!(!handle_f3_destructive_confirm_keystroke(
                &k(KeyCode::F(n)),
                &mut app
            ));
        }
    }

    // d-63: push-progress accumulator semantics. The push SEND
    // path (data_plane.rs `send_payloads`) reports bytes on
    // `FileComplete` and emits NO `Payload` — the opposite of the
    // pull receive path. So the push accumulator must take bytes
    // from FileComplete; taking them from Payload (like the pull
    // accumulator) would report 0 bytes.

    #[test]
    fn accumulate_push_progress_counts_files_and_bytes_from_file_complete() {
        use blit_core::remote::transfer::ProgressEvent;
        let mut files = 0u64;
        let mut bytes = 0u64;
        for (i, size) in [100u64, 200, 300].iter().enumerate() {
            accumulate_push_progress(
                &mut files,
                &mut bytes,
                &ProgressEvent::FileComplete {
                    path: format!("f{i}"),
                    bytes: *size,
                },
            );
        }
        assert_eq!(files, 3);
        assert_eq!(bytes, 600, "push counts bytes from FileComplete");
    }

    /// d-69: the delegated path reports cumulative deltas via
    /// `report_payload(file_delta, byte_delta)`, so each `Payload`
    /// carries BOTH counts and there are no `FileComplete` events.
    #[test]
    fn accumulate_delegated_progress_sums_files_and_bytes_from_payload() {
        use blit_core::remote::transfer::ProgressEvent;
        let mut files = 0u64;
        let mut bytes = 0u64;
        accumulate_delegated_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::Payload {
                files: 2,
                bytes: 500,
            },
        );
        accumulate_delegated_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::Payload {
                files: 1,
                bytes: 250,
            },
        );
        assert_eq!(
            files, 3,
            "delegated takes files from Payload, not FileComplete"
        );
        assert_eq!(bytes, 750);
        // A stray FileComplete must NOT double-count files.
        accumulate_delegated_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::FileComplete {
                path: "x".into(),
                bytes: 99,
            },
        );
        assert_eq!(files, 3, "FileComplete ignored on the delegated path");
        assert_eq!(bytes, 750);
    }

    /// Push emits no Payload, but if one appeared it must NOT add
    /// bytes (the FileComplete is authoritative for push) — guards
    /// against a future double-count.
    #[test]
    fn accumulate_push_progress_ignores_payload_and_manifest() {
        use blit_core::remote::transfer::ProgressEvent;
        let mut files = 0u64;
        let mut bytes = 0u64;
        accumulate_push_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::Payload {
                files: 0,
                bytes: 999,
            },
        );
        accumulate_push_progress(
            &mut files,
            &mut bytes,
            &ProgressEvent::ManifestBatch { files: 5 },
        );
        assert_eq!(files, 0);
        assert_eq!(bytes, 0, "Payload/ManifestBatch don't move push totals");
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
            browse_target: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            f1_trigger: f1trigger::F1TriggerState::new(),
            f1_push: f1push::F1PushState::new(),
            f1_push_reply_tx: mpsc::channel::<F1PushReply>(1).0,
            f1_push_progress_tx: mpsc::channel::<F1PushProgress>(1).0,
            daemons_last_fetched: None,
            // Senders aren't called on the false branch
            // but the struct demands them.
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            transfers_refan_after_setup: false,
            f2_degraded_daemons: std::collections::BTreeSet::new(),
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
            f3_batch_pull: None,
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            f3_du: f3du::F3DuState::new(),
            f3_du_reply_tx: mpsc::channel::<F3DuReply>(1).0,
            f3_del: f3del::F3DelState::new(),
            f3_del_reply_tx: mpsc::channel::<F3DelReply>(1).0,
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
            browse_target: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            f1_trigger: f1trigger::F1TriggerState::new(),
            f1_push: f1push::F1PushState::new(),
            f1_push_reply_tx: mpsc::channel::<F1PushReply>(1).0,
            f1_push_progress_tx: mpsc::channel::<F1PushProgress>(1).0,
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            transfers_refan_after_setup: false,
            f2_degraded_daemons: std::collections::BTreeSet::new(),
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
            f3_batch_pull: None,
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            f3_du: f3du::F3DuState::new(),
            f3_du_reply_tx: mpsc::channel::<F3DuReply>(1).0,
            f3_del: f3del::F3DelState::new(),
            f3_del_reply_tx: mpsc::channel::<F3DelReply>(1).0,
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
            browse_target: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            f1_trigger: f1trigger::F1TriggerState::new(),
            f1_push: f1push::F1PushState::new(),
            f1_push_reply_tx: mpsc::channel::<F1PushReply>(1).0,
            f1_push_progress_tx: mpsc::channel::<F1PushProgress>(1).0,
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            transfers_refan_after_setup: false,
            f2_degraded_daemons: std::collections::BTreeSet::new(),
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
            f3_batch_pull: None,
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            f3_du: f3du::F3DuState::new(),
            f3_du_reply_tx: mpsc::channel::<F3DuReply>(1).0,
            f3_del: f3del::F3DelState::new(),
            f3_del_reply_tx: mpsc::channel::<F3DelReply>(1).0,
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
            browse_target: None,
            remote_label: String::new(),
            daemons: DaemonsState::new(),
            f1_trigger: f1trigger::F1TriggerState::new(),
            f1_push: f1push::F1PushState::new(),
            f1_push_reply_tx: mpsc::channel::<F1PushReply>(1).0,
            f1_push_progress_tx: mpsc::channel::<F1PushProgress>(1).0,
            daemons_last_fetched: None,
            detail_tx: mpsc::channel::<DetailUpdate>(1).0,
            discovery_refresh_tx: mpsc::channel::<()>(1).0,
            transfers: TransfersState::new(),
            transfers_status: ConnectionStatus::NoRemote,
            transfers_setup_gen: 0,
            transfers_setup_pending: false,
            transfers_refan_after_setup: false,
            f2_degraded_daemons: std::collections::BTreeSet::new(),
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
            f3_batch_pull: None,
            f3_pull_reply_tx: mpsc::channel::<F3PullReply>(1).0,
            f3_pull_progress_tx: mpsc::channel::<F3PullProgress>(1).0,
            f3_du: f3du::F3DuState::new(),
            f3_du_reply_tx: mpsc::channel::<F3DuReply>(1).0,
            f3_del: f3del::F3DelState::new(),
            f3_del_reply_tx: mpsc::channel::<F3DelReply>(1).0,
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
        app.transfers.merge_snapshot(
            "",
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
            ka(&k(KeyCode::Char('C'))),
            Some(UserAction::TransferCopy)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('M'))),
            Some(UserAction::TransferMirror)
        ));
        // d-5: capital V triggers the F4 local move. d-57:
        // lowercase `v` is the F3 move (case-distinct, like
        // `m`/`M`).
        assert!(matches!(
            ka(&k(KeyCode::Char('V'))),
            Some(UserAction::TransferMove)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('v'))),
            Some(UserAction::F3MoveBegin)
        ));
    }

    /// d-6: `H` maps to the Verify-mode toggle. Lowercase
    /// `h` stays bound to Ascend (F3 navigation), so only
    /// uppercase claims the toggle.
    #[test]
    fn key_action_maps_verify_checksum_toggle() {
        assert!(matches!(
            ka(&k(KeyCode::Char('H'))),
            Some(UserAction::ToggleVerifyChecksum)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('h'))),
            Some(UserAction::Ascend)
        ));
    }

    /// d-7: `O` maps to the Verify-direction toggle.
    /// Lowercase `o` stays unmapped (reserved for future
    /// polish).
    #[test]
    fn key_action_maps_verify_one_way_toggle() {
        assert!(matches!(
            ka(&k(KeyCode::Char('O'))),
            Some(UserAction::ToggleVerifyOneWay)
        ));
        assert!(ka(&k(KeyCode::Char('o'))).is_none());
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
                matches!(ka(&k(code)), Some(UserAction::TransferMirrorConfirm)),
                "expected TransferMirrorConfirm for {code:?}",
            );
        }
        for code in [KeyCode::Char('n'), KeyCode::Char('N')] {
            assert!(
                matches!(ka(&k(code)), Some(UserAction::TransferCancel)),
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
            ka(&k(KeyCode::Char('c'))),
            Some(UserAction::ProfileClear)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('d'))),
            Some(UserAction::ProfileDisable)
        ));
        assert!(matches!(
            ka(&k(KeyCode::Char('e'))),
            Some(UserAction::ProfileEnable)
        ));
        // Uppercase E remains unmapped. Uppercase C is
        // TransferCopy (d-4); uppercase D is F3DeleteBegin
        // (d-45) — both covered in their own tests. The
        // Profile keys themselves are lowercase-only.
        assert!(ka(&k(KeyCode::Char('E'))).is_none());
        // Ctrl-c remains Quit (not ProfileClear).
        assert!(matches!(
            ka(&KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }),
            Some(UserAction::Quit)
        ));
    }

    /// m2f-5 R2: F2 `r` re-fans even while a stream is LIVE — so it
    /// picks up daemons discovered since the last setup. With a live
    /// receiver + a discovered daemon, refresh schedules a new merged
    /// setup (drops the old receiver, bumps the generation, sets
    /// pending) rather than only querying parsed_remote. The a1-6b
    /// round-3 overlap guard still holds: no respawn while pending.
    #[tokio::test]
    async fn refan_f2_setup_respawns_on_live_refresh_and_guards_pending() {
        let (tx, _rx) = mpsc::channel::<F2SetupReply>(1);
        let mut app = make_test_app_state(Screen::F2);
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:/home/").expect("launch"));
        app.daemons.replace_from_discovery(
            &[blit_core::mdns::MdnsDiscoveredService {
                fullname: "skippy._blit._tcp.local.".to_string(),
                instance_name: "skippy".to_string(),
                hostname: "skippy.local.".to_string(),
                port: 9031,
                addresses: vec![std::net::Ipv4Addr::new(192, 168, 1, 50)],
                properties: std::collections::HashMap::new(),
            }],
            Instant::now(),
        );

        // A live merged receiver is present.
        let (_etx, erx) = mpsc::channel::<F2Event>(1);
        let mut event_rx = Some(erx);
        let gen_before = app.transfers_setup_gen;

        assert!(
            refan_f2_setup(&mut app, &mut event_rx, &tx),
            "live stream + discovered daemon → re-fan"
        );
        assert!(event_rx.is_none(), "old merged receiver dropped");
        assert!(app.transfers_setup_pending, "a new setup is pending");
        assert_eq!(app.transfers_setup_gen, gen_before + 1, "generation bumped");

        // Overlap guard: a second refresh while pending is a no-op.
        assert!(
            !refan_f2_setup(&mut app, &mut event_rx, &tx),
            "no duplicate setup while one is pending"
        );
    }

    /// m2f-9 R2 regression: a daemon discovered *while the initial setup
    /// is still pending* must not be lost. The startup fan-out is in
    /// flight (pending, no live receiver), discovery adds a daemon — the
    /// change can't re-fan immediately (refan no-ops while pending), so
    /// it's deferred; once the stale setup's reply lands and pending
    /// clears, the deferred re-fan runs and F2 ends up watching the
    /// discovered daemon without a manual `r`.
    #[tokio::test]
    async fn discovery_during_pending_setup_refans_after_it_lands() {
        let (tx, _rx) = mpsc::channel::<F2SetupReply>(1);
        let mut app = make_test_app_state(Screen::F2);
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:/home/").expect("launch"));

        // Startup fan-out in flight: pending, receiver not yet live.
        app.transfers_setup_gen += 1;
        app.transfers_setup_pending = true;
        let gen_at_spawn = app.transfers_setup_gen;
        let mut event_rx: Option<mpsc::Receiver<F2Event>> = None;

        // Discovery adds a daemon while the setup is pending.
        let before = f2_watched_identities(&app);
        app.daemons.replace_from_discovery(
            &[blit_core::mdns::MdnsDiscoveredService {
                fullname: "skippy._blit._tcp.local.".to_string(),
                instance_name: "skippy".to_string(),
                hostname: "skippy.local.".to_string(),
                port: 9050,
                addresses: vec![std::net::Ipv4Addr::new(192, 168, 1, 50)],
                properties: std::collections::HashMap::new(),
            }],
            Instant::now(),
        );
        let spawned_now = handle_discovery_watch_change(&mut app, &before, &mut event_rx, &tx);
        assert!(!spawned_now, "can't re-fan while pending → deferred");
        assert!(app.transfers_refan_after_setup, "deferred re-fan recorded");
        assert_eq!(
            app.transfers_setup_gen, gen_at_spawn,
            "no new setup spawned while the first is still pending"
        );

        // The stale setup completes: clear pending, then run the deferred
        // re-fan (mirrors the setup-reply arm).
        app.transfers_setup_pending = false;
        let did_refan = apply_deferred_refan(&mut app, &mut event_rx, &tx);
        assert!(did_refan, "deferred re-fan spawns a fresh fan-out");
        assert!(app.transfers_setup_pending, "the fresh setup is pending");
        assert_eq!(
            app.transfers_setup_gen,
            gen_at_spawn + 1,
            "generation bumped for the re-fan"
        );
        assert!(
            !app.transfers_refan_after_setup,
            "deferred flag cleared once consumed"
        );
        // The fresh fan-out's watch set includes the mid-flight daemon.
        assert!(
            f2_watched_identities(&app).contains("192.168.1.50:9050"),
            "discovered daemon is now watched"
        );
    }

    /// m2f-9 R3: when the last watched daemon vanishes (mDNS-only, no
    /// launch remote), the auto re-fan must drop the live receiver and
    /// reconcile the view to empty. The pre-R3 early-return on an empty
    /// watch set left the vanished daemon's stream live.
    #[tokio::test]
    async fn discovery_emptying_drops_receiver_and_clears_rows() {
        let (tx, _rx) = mpsc::channel::<F2SetupReply>(1);
        let mut app = make_test_app_state(Screen::F2); // no parsed_remote
        let svc = |name: &str, ip: [u8; 4], port: u16| blit_core::mdns::MdnsDiscoveredService {
            fullname: format!("{name}._blit._tcp.local."),
            instance_name: name.to_string(),
            hostname: format!("{name}.local."),
            port,
            addresses: vec![std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])],
            properties: std::collections::HashMap::new(),
        };
        let started = |id: &str| {
            use blit_core::generated::{daemon_event, DaemonEvent, TransferStarted};
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: id.to_string(),
                    kind: 0,
                    peer: String::new(),
                    module: String::new(),
                    path: String::new(),
                    start_unix_ms: 1_000_000,
                })),
            }
        };
        app.daemons
            .replace_from_discovery(&[svc("skippy", [192, 168, 1, 50], 9050)], Instant::now());
        // A live receiver + an active row tagged with skippy's identity.
        let (_etx, erx) = mpsc::channel::<F2Event>(1);
        let mut event_rx = Some(erx);
        app.transfers
            .apply_event("192.168.1.50:9050", started("tA"), Instant::now());
        assert_eq!(app.transfers.active_count(), 1);

        // skippy vanishes from discovery.
        let before = f2_watched_identities(&app);
        app.daemons.replace_from_discovery(&[], Instant::now());
        let spawned = handle_discovery_watch_change(&mut app, &before, &mut event_rx, &tx);

        assert!(!spawned, "empty watch set → no new setup spawned");
        assert!(event_rx.is_none(), "stale receiver dropped");
        assert!(
            app.transfers.active_rows().is_empty(),
            "vanished daemon's active rows pruned"
        );
        assert!(
            f2_watched_identities(&app).is_empty(),
            "nothing left to watch"
        );
        assert!(matches!(app.transfers_status, ConnectionStatus::NoRemote));
    }

    /// m2f-9 R3: when the watch set shrinks (`A+B → A`), the re-fan
    /// prunes the removed daemon's active rows — they can never complete
    /// (its stream is gone), so they must not linger in the table while
    /// the fresh setup hydrates only the remaining daemon.
    #[tokio::test]
    async fn discovery_shrink_prunes_removed_daemon_active_rows() {
        let (tx, _rx) = mpsc::channel::<F2SetupReply>(1);
        let mut app = make_test_app_state(Screen::F2);
        let svc = |name: &str, ip: [u8; 4], port: u16| blit_core::mdns::MdnsDiscoveredService {
            fullname: format!("{name}._blit._tcp.local."),
            instance_name: name.to_string(),
            hostname: format!("{name}.local."),
            port,
            addresses: vec![std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])],
            properties: std::collections::HashMap::new(),
        };
        let started = |id: &str| {
            use blit_core::generated::{daemon_event, DaemonEvent, TransferStarted};
            DaemonEvent {
                payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                    transfer_id: id.to_string(),
                    kind: 0,
                    peer: String::new(),
                    module: String::new(),
                    path: String::new(),
                    start_unix_ms: 1_000_000,
                })),
            }
        };
        app.daemons.replace_from_discovery(
            &[
                svc("nas", [192, 168, 1, 50], 9050),
                svc("skippy", [192, 168, 1, 51], 9051),
            ],
            Instant::now(),
        );
        let (_etx, erx) = mpsc::channel::<F2Event>(1);
        let mut event_rx = Some(erx);
        app.transfers
            .apply_event("192.168.1.50:9050", started("tA"), Instant::now());
        app.transfers
            .apply_event("192.168.1.51:9051", started("tB"), Instant::now());
        assert_eq!(app.transfers.active_count(), 2);

        // skippy vanishes; nas remains.
        app.daemons
            .replace_from_discovery(&[svc("nas", [192, 168, 1, 50], 9050)], Instant::now());
        // Not pending → re-fan prunes the removed daemon and respawns
        // for the remaining one.
        assert!(refan_f2_setup(&mut app, &mut event_rx, &tx));
        let daemons: Vec<&str> = app
            .transfers
            .active_rows()
            .iter()
            .map(|r| r.source_daemon.as_str())
            .collect();
        assert_eq!(
            daemons,
            vec!["192.168.1.50:9050"],
            "removed daemon's active row pruned, remaining daemon kept"
        );
        assert!(
            app.transfers_setup_pending,
            "fresh setup pending for the remaining daemon"
        );
    }

    /// m2f-5 R2: in the fan-out, one daemon's stream Error must NOT
    /// drop the merged receiver — the other daemons keep feeding F2.
    /// Only `None` (all senders closed) tears it down.
    #[test]
    fn apply_f2_event_error_keeps_receiver_none_drops_it() {
        let mut app = make_test_app_state(Screen::F2);

        // One daemon's forwarder ends with an Error → keep the receiver.
        assert!(apply_f2_event(
            &mut app,
            Some(F2Event {
                daemon: "nas".to_string(),
                kind: EventOrError::Error("nas stream: boom".to_string()),
            })
        ));

        // A healthy event from ANOTHER daemon is still applied.
        assert!(apply_f2_event(
            &mut app,
            Some(F2Event {
                daemon: "skippy:9001".to_string(),
                kind: EventOrError::Event(DaemonEvent {
                    payload: Some(
                        blit_core::generated::daemon_event::Payload::TransferStarted(
                            blit_core::generated::TransferStarted {
                                transfer_id: "t1".to_string(),
                                kind: 0,
                                peer: String::new(),
                                module: String::new(),
                                path: String::new(),
                                start_unix_ms: 1,
                            },
                        ),
                    ),
                }),
            })
        ));
        assert_eq!(
            app.transfers.active_rows()[0].source_daemon,
            "skippy:9001",
            "other daemon's event applied despite the prior error"
        );

        // All senders closed (None) → drop the merged receiver.
        assert!(!apply_f2_event(&mut app, None));
    }

    /// m2f-10: the health→banner fold. None degraded is Live; a subset
    /// is a partial Degraded that names the affected daemons; all (or a
    /// count exceeding a stale total) is a full Degraded.
    #[test]
    fn f2_status_from_health_partial_vs_full() {
        use std::collections::BTreeSet;
        assert!(matches!(
            f2_status_from_health(&BTreeSet::new(), 3),
            ConnectionStatus::Live
        ));

        let one: BTreeSet<String> = ["skippy:9050".to_string()].into_iter().collect();
        match f2_status_from_health(&one, 3) {
            ConnectionStatus::Degraded(msg) => {
                assert!(msg.contains("1/3"), "partial count in: {msg}");
                assert!(msg.contains("skippy:9050"), "names the daemon: {msg}");
            }
            other => panic!("expected partial Degraded, got {other:?}"),
        }

        let all: BTreeSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        match f2_status_from_health(&all, 3) {
            ConnectionStatus::Degraded(msg) => assert!(msg.contains("all 3"), "{msg}"),
            other => panic!("expected full Degraded, got {other:?}"),
        }
        // A degraded count exceeding a stale/zero total reads as all-down,
        // never as a nonsensical partial.
        match f2_status_from_health(&all, 0) {
            ConnectionStatus::Degraded(msg) => assert!(msg.contains("all 3"), "{msg}"),
            other => panic!("expected full Degraded, got {other:?}"),
        }
    }

    /// m2f-10: with two watched daemons, one stream erroring yields a
    /// PARTIAL degrade (the pane stays usable for the live daemon), and
    /// that daemon recovering returns the banner to Live — rather than
    /// the pre-m2f-10 behavior of blanking the whole pane to Degraded.
    #[test]
    fn apply_f2_event_partial_degrade_then_recover() {
        let mut app = make_test_app_state(Screen::F2);
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:/home/").expect("launch"));
        app.daemons.replace_from_discovery(
            &[blit_core::mdns::MdnsDiscoveredService {
                fullname: "skippy._blit._tcp.local.".to_string(),
                instance_name: "skippy".to_string(),
                hostname: "skippy.local.".to_string(),
                port: 9050,
                addresses: vec![std::net::Ipv4Addr::new(192, 168, 1, 50)],
                properties: std::collections::HashMap::new(),
            }],
            Instant::now(),
        );
        assert_eq!(
            f2_watched_endpoints(&app).len(),
            2,
            "nas + discovered skippy"
        );

        // skippy's stream errors → partial, not a full blank.
        apply_f2_event(
            &mut app,
            Some(F2Event {
                daemon: "192.168.1.50:9050".to_string(),
                kind: EventOrError::Error("connection reset".to_string()),
            }),
        );
        match &app.transfers_status {
            ConnectionStatus::Degraded(msg) => {
                assert!(msg.contains("1/2"), "one of two down: {msg}");
                assert!(msg.contains("192.168.1.50:9050"), "names it: {msg}");
            }
            other => panic!("expected partial Degraded, got {other:?}"),
        }
        assert_eq!(app.f2_degraded_daemons.len(), 1);

        // skippy recovers → back to Live, set cleared.
        apply_f2_event(
            &mut app,
            Some(F2Event {
                daemon: "192.168.1.50:9050".to_string(),
                kind: EventOrError::Connected,
            }),
        );
        assert!(
            matches!(app.transfers_status, ConnectionStatus::Live),
            "recovered → Live"
        );
        assert!(app.f2_degraded_daemons.is_empty());
    }

    /// m2f-10 regression guard: a healthy stream signal from a daemon
    /// that was never degraded must NOT overwrite a `Degraded` set by a
    /// failed initial `GetState` (snapshot health). The live stream may
    /// be fine while the active/recent view is incomplete — that
    /// distinction predates m2f-10 and must survive it.
    #[test]
    fn healthy_event_does_not_clobber_snapshot_degraded() {
        let mut app = make_test_app_state(Screen::F2);
        app.parsed_remote = Some(RemoteEndpoint::parse("nas:/home/").expect("launch"));
        app.transfers_status =
            ConnectionStatus::Degraded("initial GetState failed: boom".to_string());
        apply_f2_event(
            &mut app,
            Some(F2Event {
                daemon: "nas".to_string(),
                kind: EventOrError::Connected,
            }),
        );
        match &app.transfers_status {
            ConnectionStatus::Degraded(msg) => {
                assert!(
                    msg.contains("initial GetState failed"),
                    "snapshot status kept: {msg}"
                );
            }
            other => panic!("snapshot Degraded must persist, got {other:?}"),
        }
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
