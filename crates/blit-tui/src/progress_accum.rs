//! audit-7d: pure progress-folding + throughput helpers extracted from
//! `main.rs` (behavior-preserving — verbatim move, no logic change). These
//! fold a stream of `ProgressEvent`s into running `(files, bytes)` totals
//! using the per-direction semantics each transfer path emits, plus the
//! du-total reducer and the throughput formula. All pure functions; the
//! event-loop and spawn tasks in `main.rs` call them.

/// d-37 round 2: fold one pull `ProgressEvent` into the running
/// `(files, bytes)` totals using pull-receive semantics. Bytes come from
/// `Payload` only; file count from `FileComplete` only.
///
/// The TCP data-plane path emits BOTH `Payload { files: 0, bytes: N }`
/// and `FileComplete { bytes: N }` for the same completed file
/// (`pipeline.rs` `execute_receive_pipeline`), so adding bytes from both
/// would double-count and the footer would snap backward when the
/// authoritative reply total lands. The direct-gRPC path emits
/// `FileComplete { bytes: 0 }` (`pull.rs` `finalize_active_file`) with
/// bytes carried by `Payload` — so counting bytes from `Payload` alone is
/// correct on both paths, and counting one file per `FileComplete` is
/// correct on both paths.
pub(crate) fn accumulate_pull_progress(
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

/// d-63: fold one push `ProgressEvent` into the running `(files, bytes)`
/// totals using push-SEND semantics.
///
/// Unlike the pull (receive) path, the push send path reports bytes on
/// `FileComplete` (`data_plane.rs` `send_payloads`:
/// `report_file_complete(path, header.size)`) and emits NO `Payload`
/// events — so bytes AND files both come from `FileComplete` here
/// (whereas `accumulate_pull_progress` takes bytes from `Payload` to
/// avoid the receive path's `Payload`+`FileComplete` double-count).
/// Counting bytes from `Payload` here would report 0; counting
/// `FileComplete` bytes is correct and never double-counts because push
/// emits no `Payload`.
pub(crate) fn accumulate_push_progress(
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
/// `report_payload(file_delta, byte_delta)` — so a `Payload` carries BOTH
/// the file and byte deltas (unlike the receive path, where
/// `Payload.files` is unused and files come from `FileComplete`, and
/// unlike push, where bytes ride `FileComplete`). It emits no
/// `FileComplete`, so take both fields from `Payload` here.
pub(crate) fn accumulate_delegated_progress(
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
/// Suppressed (returns 0) until at least one second has elapsed —
/// `bytes / tiny_elapsed` produces meaningless multi-GiB/s spikes in the
/// first moments of a transfer, and the footer reads better with no rate
/// than a wrong one. After the warm-up it's a simple cumulative average
/// (`bytes / elapsed`), matching the "is it moving" intent of the footer
/// rather than an instantaneous rate.
pub(crate) fn pull_throughput(bytes: u64, elapsed_secs: f64) -> u64 {
    if elapsed_secs >= 1.0 {
        (bytes as f64 / elapsed_secs) as u64
    } else {
        0
    }
}

/// Fold one `DiskUsageEntry`'s `(bytes, files)` into the running best
/// total, keeping the entry with the largest byte count (the F3 du
/// summary reports the deepest/total measurement).
pub(crate) fn du_total_from_entries(
    acc: Option<(u64, u64)>,
    bytes: u64,
    files: u64,
) -> Option<(u64, u64)> {
    match acc {
        Some((best_bytes, _)) if best_bytes >= bytes => acc,
        _ => Some((bytes, files)),
    }
}
