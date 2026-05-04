//! Test instrumentation hooks for remote byte-path assertions.
//!
//! These hooks are inert unless `BLIT_TEST_COUNTER_FILE` is set. They are
//! intentionally env-gated instead of `cfg(test)` because CLI integration tests
//! execute the compiled `blit-cli` binary as a child process.

use std::fs::OpenOptions;
use std::io::Write;

const COUNTER_FILE_ENV: &str = "BLIT_TEST_COUNTER_FILE";

fn record(event: &str, value: u64) {
    let Ok(path) = std::env::var(COUNTER_FILE_ENV) else {
        return;
    };
    if path.is_empty() {
        return;
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{event} {value}");
    }
}

pub(crate) fn record_cli_data_plane_outbound_bytes(bytes: u64) {
    if bytes > 0 {
        record("cli_data_plane_outbound_bytes", bytes);
    }
}

pub(crate) fn record_remote_transfer_source_constructed() {
    record("remote_transfer_source_constructed", 1);
}
