//! Stderr backend for the `log` facade, shared by every workspace binary.
//!
//! The `log` crate's default logger is a no-op: until a binary installs a
//! backend, every `log::warn!` / `log::error!` in the workspace — including
//! security-degradation warnings and best-effort metadata failures whose
//! *only* surface is the warn — is formatted and discarded (w5-1,
//! errors-log-facade-has-no-backend). Each binary calls [`init`] first
//! thing in `main`.
//!
//! Output follows the one workspace stderr convention (w5-1,
//! errors-stderr-prefix-babel): `<binary>: <level>: <message>`.
//!
//! While a caller owns a live terminal row on stderr (the CLI progress
//! row), a raw `eprintln!` here scrolls that row off screen. Such a
//! caller installs its own line sink with [`redirect_lines`] for the
//! lifetime of the row; with no sink installed — every daemon, every
//! non-row run — the backend writes exactly the same bytes to stderr as
//! before.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, OnceLock, RwLock};

struct StderrLogger {
    binary: &'static str,
}

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            route_line(&format!(
                "{}: {}: {}",
                self.binary,
                level_str(record.level()),
                record.args()
            ));
        }
    }

    fn flush(&self) {}
}

/// Destination for one formatted log line while a live terminal row
/// owns stderr. Called from any thread the `log` facade is used on.
pub type LineSink = Arc<dyn Fn(&str) + Send + Sync>;

/// The installed sink, or `None` when log lines go straight to stderr.
/// A `RwLock` rather than a swap-free atomic because the sink is set
/// once per transfer and read only when something actually logs.
static LINE_SINK: RwLock<Option<LineSink>> = RwLock::new(None);

/// Route every log line through `sink` until the returned guard drops.
///
/// The single-writer rule for transfer-time stderr: while a progress row
/// is live, every line print goes through the row's handle so it scrolls
/// cleanly above the intact row. Restoring on `Drop` covers the error
/// path too — a row torn down by an early return or an unwind restores
/// the previous backend just as an orderly finish does.
///
/// Nesting is LIFO: the guard restores whatever was installed before it.
#[must_use = "the redirect lasts only as long as the returned guard"]
pub fn redirect_lines(sink: LineSink) -> LineRedirect {
    let previous = LINE_SINK
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .replace(sink);
    LineRedirect { previous }
}

/// Restores the previous log-line destination when dropped. See
/// [`redirect_lines`].
pub struct LineRedirect {
    previous: Option<LineSink>,
}

impl Drop for LineRedirect {
    fn drop(&mut self) {
        *LINE_SINK
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = self.previous.take();
    }
}

/// Send one already-formatted line to the installed sink, or to stderr
/// when there is none.
/// Emit one already-formatted line through the active line sink, or to
/// stderr when none is installed.
///
/// Public for diagnostics that must honour the live progress row's
/// sole-writer contract without going through `log` — the log path prefixes
/// `binary: LEVEL:` and is subject to `BLIT_LOG` filtering, neither of which
/// a machine-read artifact line can tolerate (cr-ls1-3).
pub fn route_line(line: &str) {
    // Clone the handle out and release the lock before calling it: the
    // sink writes to a terminal, and one that logged while holding the
    // read lock would deadlock against a concurrent redirect.
    let sink = LINE_SINK
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    match sink {
        Some(sink) => sink(line),
        None => eprintln!("{line}"),
    }
}

fn level_str(level: Level) -> &'static str {
    match level {
        Level::Error => "error",
        Level::Warn => "warn",
        Level::Info => "info",
        Level::Debug => "debug",
        Level::Trace => "trace",
    }
}

/// Resolve the max level from a `BLIT_LOG` value (`off|error|warn|info|
/// debug|trace`, case-insensitive). Unset or unparseable → warn.
fn level_from_env(value: Option<&str>) -> LevelFilter {
    value
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(LevelFilter::Warn)
}

static LOGGER: OnceLock<StderrLogger> = OnceLock::new();

/// Install the stderr logger for `binary` (e.g. `"blit"`, `"blitd"`).
/// Default max level is warn; `BLIT_LOG` overrides. Idempotent: only the
/// first successful install in a process takes effect.
pub fn init(binary: &'static str) {
    let logger = LOGGER.get_or_init(|| StderrLogger { binary });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(level_from_env(std::env::var("BLIT_LOG").ok().as_deref()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The line sink is process-global, so the tests that install one
    /// take turns. (No logger is installed in a test binary, so nothing
    /// else in the suite can reach the sink.)
    static SINK_TURN: Mutex<()> = Mutex::new(());

    /// Drive the backend exactly as the `log` facade does, without
    /// installing it process-wide (the install is a one-shot).
    fn emit(logger: &StderrLogger, level: Level, message: &str) {
        log::set_max_level(LevelFilter::Trace);
        logger.log(
            &Record::builder()
                .level(level)
                .args(format_args!("{message}"))
                .build(),
        );
    }

    /// clp-2 residue (a): while a live progress row owns stderr, a
    /// library `log::warn!` must reach the row's handle, not raw
    /// stderr — a raw line scrolls the row off screen. Red when
    /// `Log::log` writes stderr directly instead of routing.
    #[test]
    fn a_log_line_routes_through_the_installed_sink() {
        let _turn = SINK_TURN.lock().unwrap_or_else(|p| p.into_inner());
        let captured: Arc<Mutex<Vec<String>>> = Arc::default();
        let sink = {
            let captured = Arc::clone(&captured);
            Arc::new(move |line: &str| {
                captured
                    .lock()
                    .expect("capture poisoned")
                    .push(line.to_string())
            })
        };
        let logger = StderrLogger { binary: "blit" };
        let redirect = redirect_lines(sink);
        emit(&logger, Level::Warn, "permission denied");
        // The guard restores the raw backend, so this second line must
        // NOT reach the sink (it goes to the harness-captured stderr).
        drop(redirect);
        emit(&logger, Level::Warn, "after the row");

        assert_eq!(
            captured.lock().expect("capture poisoned").as_slice(),
            ["blit: warn: permission denied"],
            "one line routed while the row was live, none after it finished"
        );
    }

    /// Restoration is LIFO, so a redirect installed inside another one
    /// hands the lane back rather than clearing it.
    #[test]
    fn nested_redirects_restore_the_outer_sink() {
        let _turn = SINK_TURN.lock().unwrap_or_else(|p| p.into_inner());
        let outer: Arc<Mutex<Vec<String>>> = Arc::default();
        let inner: Arc<Mutex<Vec<String>>> = Arc::default();
        let sink_for = |captured: &Arc<Mutex<Vec<String>>>| {
            let captured = Arc::clone(captured);
            Arc::new(move |line: &str| {
                captured
                    .lock()
                    .expect("capture poisoned")
                    .push(line.to_string())
            }) as LineSink
        };
        let logger = StderrLogger { binary: "blit" };

        let outer_guard = redirect_lines(sink_for(&outer));
        let inner_guard = redirect_lines(sink_for(&inner));
        emit(&logger, Level::Error, "inner");
        drop(inner_guard);
        emit(&logger, Level::Error, "outer");
        drop(outer_guard);

        assert_eq!(
            inner.lock().expect("capture poisoned").as_slice(),
            ["blit: error: inner"]
        );
        assert_eq!(
            outer.lock().expect("capture poisoned").as_slice(),
            ["blit: error: outer"]
        );
    }

    #[test]
    fn levels_render_lowercase_for_the_prefix_convention() {
        assert_eq!(level_str(Level::Error), "error");
        assert_eq!(level_str(Level::Warn), "warn");
        assert_eq!(level_str(Level::Info), "info");
        assert_eq!(level_str(Level::Debug), "debug");
        assert_eq!(level_str(Level::Trace), "trace");
    }

    #[test]
    fn unset_or_garbage_blit_log_defaults_to_warn() {
        assert_eq!(level_from_env(None), LevelFilter::Warn);
        assert_eq!(level_from_env(Some("")), LevelFilter::Warn);
        assert_eq!(level_from_env(Some("loud")), LevelFilter::Warn);
    }

    #[test]
    fn blit_log_overrides_are_parsed_case_insensitively() {
        assert_eq!(level_from_env(Some("debug")), LevelFilter::Debug);
        assert_eq!(level_from_env(Some("ERROR")), LevelFilter::Error);
        assert_eq!(level_from_env(Some(" off ")), LevelFilter::Off);
        assert_eq!(level_from_env(Some("Trace")), LevelFilter::Trace);
    }
}
