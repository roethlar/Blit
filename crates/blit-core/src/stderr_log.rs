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

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::OnceLock;

struct StderrLogger {
    binary: &'static str,
}

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "{}: {}: {}",
                self.binary,
                level_str(record.level()),
                record.args()
            );
        }
    }

    fn flush(&self) {}
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
