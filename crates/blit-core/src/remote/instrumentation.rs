//! Diagnostics instrumentation hooks for remote byte-path assertions.
//!
//! These hooks are inert unless a counter file path has been installed
//! via [`set_counter_path`]. The path is installed by the `blit` CLI
//! from the global `--diagnostics-counter-file <PATH>` flag at startup,
//! before any RPC fires.
//!
//! audit-l39 (2026-06-04): pre-0.1.1 this module read the
//! `BLIT_TEST_COUNTER_FILE` env var instead of the CLI flag. Env vars
//! are out for app + diagnostic config per owner directive; the
//! integration tests (`crates/blit-cli/tests/remote_remote.rs`) and
//! the operator bench script (`scripts/bench_remote_remote.sh`) now
//! pass `--diagnostics-counter-file PATH` explicitly. The CLI flag
//! is marked `hide_short_help = true` so it's hidden from the short
//! `-h` summary (de-emphasized for operators who don't need it) but
//! still appears in the full `--help` output where troubleshooting
//! flags belong — diagnostics-discoverable, not advertised.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

/// CLI-installed diagnostics counter file path. Set once at startup
/// by `blit-cli`'s main when `--diagnostics-counter-file` is supplied;
/// `None` otherwise (and `record` is a no-op).
static COUNTER_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Install the diagnostics counter file path. Called by the CLI from
/// `main` after parsing `Cli`. A second call is silently ignored — the
/// path is installed once per process.
pub fn set_counter_path(path: PathBuf) {
    let _ = COUNTER_PATH.set(path);
}

fn record(event: &str, value: u64) {
    let Some(path) = COUNTER_PATH.get() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{event} {value}");
    }
}

/// ssc-6: one line per end-of-run retry pass (`retry_pass <k>`), so a
/// test can count passes without timing the run.
pub fn record_retry_pass(pass: u64) {
    record("retry_pass", pass);
}

/// ssc-6: the wait the retry loop was asked to honour before a pass
/// (`retry_wait_seconds <n>`), recorded whether or not the diagnostics
/// switch replaced the real sleep.
pub fn record_retry_wait_seconds(seconds: u64) {
    record("retry_wait_seconds", seconds);
}

pub(crate) fn record_cli_data_plane_outbound_bytes(bytes: u64) {
    if bytes > 0 {
        record("cli_data_plane_outbound_bytes", bytes);
    }
}
