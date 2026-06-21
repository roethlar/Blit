//! Unified transfer pipeline: source → prepare → sink(s).
//!
//! All transfer paths (local→local, local→remote push, remote→local pull,
//! remote→remote) route through the same executor. Payloads can be supplied
//! either upfront ([`execute_sink_pipeline`]) or incrementally as they are
//! produced ([`execute_sink_pipeline_streaming`]). The one-shot form is a
//! thin wrapper that sends every payload on a channel and delegates.

use std::sync::Arc;

use eyre::{Context, Result};
use tokio::sync::mpsc;

use super::payload::{PreparedPayload, TransferPayload};
use super::progress::RemoteTransferProgress;
use super::sink::{SinkOutcome, TransferSink};
use super::source::TransferSource;

/// Execute a transfer pipeline with all payloads known upfront.
///
/// This is a convenience wrapper around [`execute_sink_pipeline_streaming`]
/// that spawns a task to send every payload into the channel and then drops
/// the sender, signalling end-of-stream.
pub async fn execute_sink_pipeline(
    source: Arc<dyn TransferSource>,
    sinks: Vec<Arc<dyn TransferSink>>,
    payloads: Vec<TransferPayload>,
    prefetch: usize,
    progress: Option<&RemoteTransferProgress>,
) -> Result<SinkOutcome> {
    if sinks.is_empty() {
        return Ok(SinkOutcome::default());
    }
    if payloads.is_empty() {
        for sink in &sinks {
            sink.finish().await?;
        }
        return Ok(SinkOutcome::default());
    }

    let capacity = prefetch.max(1);
    let (tx, rx) = mpsc::channel::<TransferPayload>(capacity);

    // Feed payloads in a background task so the pipeline can start writing
    // before the whole vec is queued (the channel provides back-pressure).
    let feeder = tokio::spawn(async move {
        for payload in payloads {
            if tx.send(payload).await.is_err() {
                break;
            }
        }
        // Dropping tx closes the channel and signals end-of-stream.
    });

    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
    let _ = feeder.await;
    result
}

/// Execute a transfer pipeline with payloads arriving on a channel.
///
/// Payloads are distributed across `sinks` through a single shared
/// **work-stealing** queue (a bounded `flume` MPMC channel): each sink
/// runs as a tokio task that pulls the next available payload via
/// `recv_async().await`, so a slow sink can never head-of-line-block the
/// others (the failure mode of the previous round-robin per-sink
/// channels). A forwarder task moves payloads from the incoming
/// `payload_rx` onto the shared queue; dropping its sender on
/// end-of-stream lets every worker observe `Disconnected` once the queue
/// drains, at which point it calls `sink.finish()`. Errors from any
/// worker propagate up (first error wins).
///
/// `prefetch` controls the per-sink preparation-in-flight limit; the
/// shared queue is bounded at `prefetch * sinks.len()` so total
/// in-flight capacity matches the previous per-sink-channel design
/// (back-pressure preserved).
pub async fn execute_sink_pipeline_streaming(
    source: Arc<dyn TransferSource>,
    sinks: Vec<Arc<dyn TransferSink>>,
    mut payload_rx: mpsc::Receiver<TransferPayload>,
    prefetch: usize,
    progress: Option<&RemoteTransferProgress>,
) -> Result<SinkOutcome> {
    if sinks.is_empty() {
        // Drain incoming channel so the producer isn't left dangling.
        while payload_rx.recv().await.is_some() {}
        return Ok(SinkOutcome::default());
    }

    let sink_count = sinks.len();
    let capacity = prefetch.max(1) * sink_count;
    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));

    // Single shared work queue. Each worker owns exactly one sink but
    // pulls payloads from the common queue, so work is stolen by
    // whichever sink is free rather than pre-assigned round-robin.
    let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
    let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> = Vec::with_capacity(sink_count);

    // Cancellation flag set by the first worker that errors. Without it,
    // one sink failing only drops that worker's `work_rx` clone; as long
    // as any other worker is alive `send_async` keeps succeeding, so the
    // forwarder would keep draining `payload_rx` and queueing payloads
    // that can never complete — delaying first-error-wins propagation
    // (Codex review, PR2). With it, the forwarder stops at the next
    // payload boundary and closes the queue so the survivors drain and
    // finish promptly.
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));

    for sink in sinks {
        let work_rx = work_rx.clone();
        let source_clone = source.clone();
        let progress_clone = progress.cloned();
        let total_clone = total.clone();
        let cancelled_worker = cancelled.clone();
        sink_handles.push(tokio::spawn(async move {
            // Wrap the body so any early-return error trips the shared
            // cancel flag before the `?` unwinds the task.
            let run = async {
                while let Ok(payload) = work_rx.recv_async().await {
                    let prepared = source_clone
                        .prepare_payload(payload)
                        .await
                        .context("preparing payload")?;
                    let files: Vec<(String, u64)> = match &prepared {
                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
                        PreparedPayload::TarShard { headers, .. } => headers
                            .iter()
                            .map(|h| (h.relative_path.clone(), h.size))
                            .collect(),
                        // Resume-block payloads patch existing files; no
                        // file-completion event from one-block-at-a-time.
                        PreparedPayload::FileBlock { .. }
                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
                    };
                    let outcome = sink
                        .write_payload(prepared)
                        .await
                        .context("writing payload")?;
                    if let Some(p) = &progress_clone {
                        for (name, size) in &files {
                            p.report_file_complete(name.clone(), *size);
                        }
                    }
                    let mut t = total_clone.lock().unwrap();
                    t.merge(&outcome);
                }
                sink.finish().await?;
                Ok::<(), eyre::Report>(())
            }
            .await;
            if run.is_err() {
                // Signal the forwarder (and implicitly the other workers,
                // once the queue closes) to stop feeding new work.
                cancelled_worker.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            run
        }));
    }

    // Drop our own receiver handle so the channel closes once the
    // forwarder drops its sender and the workers' clones drain — without
    // this, `recv_async` would never see `Disconnected`.
    drop(work_rx);

    // Forwarder: move payloads from the incoming channel onto the shared
    // work queue. `send_async` applies back-pressure (bounded queue); if
    // every worker has gone away (e.g. all sinks errored) the send fails
    // and we stop. It also bails as soon as a worker sets `cancelled`, so
    // a single sink error halts intake promptly instead of waiting for
    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
    // cancel) signals the workers.
    let cancelled_fwd = cancelled.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(payload) = payload_rx.recv().await {
            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
                // A worker errored — stop draining the producer and let
                // the queue close so survivors finish and the error
                // surfaces without delay.
                return;
            }
            if work_tx.send_async(payload).await.is_err() {
                // All workers dropped their receivers — nothing left to
                // feed; treat as shutdown.
                return;
            }
        }
        // Dropping work_tx closes the queue → workers see Disconnected
        // after draining and run finish().
        drop(work_tx);
    });

    // Wait for all sinks to finish and aggregate errors (first wins).
    let mut first_err: Option<eyre::Report> = None;
    for h in sink_handles {
        match h.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) if first_err.is_none() => first_err = Some(e),
            Ok(Err(_)) => {}
            Err(join) if first_err.is_none() => {
                first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
            }
            Err(_) => {}
        }
    }
    let _ = forwarder.await;

    if let Some(err) = first_err {
        return Err(err);
    }

    let result = total.lock().unwrap().clone();
    Ok(result)
}

// =====================================================================
// Receive pipeline — symmetric counterpart of execute_sink_pipeline.
// =====================================================================

use crate::generated::FileHeader;
use eyre::bail;
use tokio::io::{AsyncRead, AsyncReadExt};

use super::data_plane::{
    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
    DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
};

/// Drive a `TransferSink` from a TCP wire stream.
///
/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
/// where the outbound executor takes a [`TransferSource`] and dispatches
/// payloads round-robin across N sinks, this one consumes a single
/// inbound wire (parsing record headers and producing
/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
/// sequentially. Multi-stream parallelism comes from spawning N invocations,
/// one per inbound TCP connection.
///
/// Both directions converge on `TransferSink::write_payload`: file data
/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
/// which uses the same `receive_stream_double_buffered` helper as the
/// daemon's push receiver and the client's pull receiver — one path,
/// one optimization surface.
pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
    socket: &mut R,
    sink: Arc<dyn TransferSink>,
    progress: Option<&RemoteTransferProgress>,
) -> Result<SinkOutcome> {
    let mut total = SinkOutcome::default();

    loop {
        let mut tag = [0u8; 1];
        socket
            .read_exact(&mut tag)
            .await
            .context("reading data-plane record tag")?;

        match tag[0] {
            DATA_PLANE_RECORD_END => break,
            DATA_PLANE_RECORD_FILE => {
                let mut header = read_file_header(socket).await?;
                let file_size = read_u64(socket).await?;
                let mtime = read_i64(socket).await?;
                let perms = read_u32(socket).await?;
                header.size = file_size;
                header.mtime_seconds = mtime;
                header.permissions = perms;
                // Use AsyncReadExt::take to give the sink exactly
                // file_size bytes of the wire. tokio's Take is the
                // canonical way to limit a borrowed AsyncRead.
                use tokio::io::AsyncReadExt;
                let mut reader = (&mut *socket).take(file_size);
                let outcome = sink
                    .write_file_stream(&header, &mut reader)
                    .await
                    .with_context(|| format!("receiving {}", header.relative_path))?;
                if let Some(p) = progress {
                    p.report_payload(0, outcome.bytes_written);
                    p.report_file_complete(header.relative_path.clone(), outcome.bytes_written);
                }
                total.merge(&outcome);
            }
            DATA_PLANE_RECORD_TAR_SHARD => {
                let (headers, data) = read_tar_shard(socket).await?;
                let bytes = data.len() as u64;
                let payload = PreparedPayload::TarShard { headers, data };
                let outcome = sink
                    .write_payload(payload)
                    .await
                    .context("writing payload")?;
                if let Some(p) = progress {
                    p.report_payload(0, bytes);
                }
                total.merge(&outcome);
            }
            DATA_PLANE_RECORD_BLOCK => {
                let path = read_string(socket).await?;
                let offset = read_u64(socket).await?;
                let len = read_u32(socket).await? as usize;
                if len > MAX_WIRE_BLOCK_BYTES {
                    bail!(
                        "wire block payload {} bytes exceeds max {} (rejecting to avoid OOM)",
                        len,
                        MAX_WIRE_BLOCK_BYTES
                    );
                }
                let mut bytes = vec![0u8; len];
                socket
                    .read_exact(&mut bytes)
                    .await
                    .context("reading block bytes")?;
                let payload = PreparedPayload::FileBlock {
                    relative_path: path,
                    offset,
                    bytes,
                };
                let outcome = sink
                    .write_payload(payload)
                    .await
                    .context("writing payload")?;
                total.merge(&outcome);
            }
            DATA_PLANE_RECORD_BLOCK_COMPLETE => {
                let path = read_string(socket).await?;
                let total_size = read_u64(socket).await?;
                let mtime = read_i64(socket).await?;
                let perms = read_u32(socket).await?;
                let payload = PreparedPayload::FileBlockComplete {
                    relative_path: path,
                    total_size,
                    mtime_seconds: mtime,
                    permissions: perms,
                };
                let outcome = sink
                    .write_payload(payload)
                    .await
                    .context("writing payload")?;
                total.merge(&outcome);
            }
            other => bail!("unknown data-plane record tag: 0x{:02X}", other),
        }
    }

    sink.finish().await.context("finalising sink")?;
    Ok(total)
}

async fn read_u32<R: AsyncRead + Unpin>(socket: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    socket.read_exact(&mut buf).await.context("reading u32")?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_u64<R: AsyncRead + Unpin>(socket: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    socket.read_exact(&mut buf).await.context("reading u64")?;
    Ok(u64::from_be_bytes(buf))
}

async fn read_i64<R: AsyncRead + Unpin>(socket: &mut R) -> Result<i64> {
    let mut buf = [0u8; 8];
    socket.read_exact(&mut buf).await.context("reading i64")?;
    Ok(i64::from_be_bytes(buf))
}

/// Maximum path length accepted from the wire. Higher than POSIX
/// PATH_MAX (4096) but bounded so a hostile peer can't trigger a
/// many-GB allocation by sending u32::MAX as a path length.
const MAX_WIRE_PATH_LEN: usize = 64 * 1024;
/// Maximum file count per tar shard. The planner targets up to a few
/// thousand entries per shard; this bound prevents a wire-driven
/// `Vec::with_capacity(u32::MAX)` allocation.
const MAX_WIRE_TAR_SHARD_FILES: usize = 1_048_576;
/// Maximum tar shard payload size (in bytes). Single source of truth
/// is `tar_safety::MAX_TAR_SHARD_BYTES` so the wire-side reader
/// rejects shards the receive-side helper would reject anyway.
/// Previously inconsistent: wire was 1 GiB, helper was 256 MiB —
/// closing F8 of `docs/reviews/codebase_review_2026-05-01.md`.
const MAX_WIRE_TAR_SHARD_BYTES: usize =
    crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES as usize;
/// Maximum single-block payload size on the resume protocol. Aligns
/// with `crate::copy::MAX_BLOCK_SIZE`.
const MAX_WIRE_BLOCK_BYTES: usize = 64 * 1024 * 1024;

async fn read_string<R: AsyncRead + Unpin>(socket: &mut R) -> Result<String> {
    let len = read_u32(socket).await? as usize;
    if len > MAX_WIRE_PATH_LEN {
        bail!(
            "wire path length {} exceeds max {} (rejecting to avoid OOM)",
            len,
            MAX_WIRE_PATH_LEN
        );
    }
    let mut buf = vec![0u8; len];
    socket
        .read_exact(&mut buf)
        .await
        .context("reading string bytes")?;
    String::from_utf8(buf).context("invalid UTF-8 in data-plane string")
}

async fn read_file_header<R: AsyncRead + Unpin>(socket: &mut R) -> Result<FileHeader> {
    let path = read_string(socket).await?;
    // Validate at the wire boundary — rejects ../, absolute paths,
    // Windows drive prefixes, UNC, NUL bytes. Sinks re-validate via
    // `safe_join` (defense in depth), but failing here keeps unsafe
    // headers out of the FileHeader stream entirely.
    crate::path_safety::validate_wire_path(&path)
        .with_context(|| format!("rejecting wire file header path {:?}", path))?;
    Ok(FileHeader {
        relative_path: path,
        size: 0, // populated by caller from the file_size field on the wire
        mtime_seconds: 0,
        permissions: 0,
        checksum: vec![],
    })
}

async fn read_tar_shard<R: AsyncRead + Unpin>(
    socket: &mut R,
) -> Result<(Vec<FileHeader>, Vec<u8>)> {
    let count = read_u32(socket).await? as usize;
    if count > MAX_WIRE_TAR_SHARD_FILES {
        bail!(
            "wire tar shard file count {} exceeds max {} (rejecting to avoid OOM)",
            count,
            MAX_WIRE_TAR_SHARD_FILES
        );
    }
    let mut headers = Vec::with_capacity(count);
    for _ in 0..count {
        let path = read_string(socket).await?;
        crate::path_safety::validate_wire_path(&path)
            .with_context(|| format!("rejecting wire tar shard header path {:?}", path))?;
        let size = read_u64(socket).await?;
        let mtime = read_i64(socket).await?;
        let permissions = read_u32(socket).await?;
        headers.push(FileHeader {
            relative_path: path,
            size,
            mtime_seconds: mtime,
            permissions,
            checksum: vec![],
        });
    }
    let tar_size = read_u64(socket).await?;
    if tar_size > MAX_WIRE_TAR_SHARD_BYTES as u64 {
        bail!(
            "wire tar shard payload {} bytes exceeds max {} (rejecting to avoid OOM)",
            tar_size,
            MAX_WIRE_TAR_SHARD_BYTES
        );
    }
    let mut data = vec![0u8; tar_size as usize];
    socket
        .read_exact(&mut data)
        .await
        .context("reading tar shard bytes")?;
    Ok((headers, data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::ComparisonMode;
    use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
    use crate::remote::transfer::source::FsTransferSource;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use tempfile::tempdir;

    /// Sink that fails the first `write_payload` with a recognisable
    /// message. Used by the POST_REVIEW_FIXES §1.1b regression test
    /// to confirm `execute_sink_pipeline_streaming` returns the
    /// underlying error verbatim — which is what
    /// `MultiStreamSender::queue` then surfaces to the user instead
    /// of the previous generic "data plane pipeline closed
    /// unexpectedly" string.
    struct FailingSink {
        marker: &'static str,
        dst_root: PathBuf,
    }

    #[async_trait::async_trait]
    impl TransferSink for FailingSink {
        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
            eyre::bail!("{}", self.marker)
        }
        fn root(&self) -> &Path {
            &self.dst_root
        }
    }

    #[tokio::test]
    async fn pipeline_copies_files_end_to_end() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();

        std::fs::write(src.join("a.txt"), b"alpha").unwrap();
        std::fs::write(src.join("b.txt"), b"bravo").unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub/c.txt"), b"charlie").unwrap();

        let source = Arc::new(FsTransferSource::new(src.clone()));
        let sink = Arc::new(FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        ));

        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut rx, handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = rx.recv().await {
            headers.push(h);
        }
        let _total = handle.await.unwrap().unwrap();

        let planned = crate::remote::transfer::payload::plan_transfer_payloads(
            headers,
            source.root(),
            Default::default(),
        )
        .unwrap();

        let outcome = execute_sink_pipeline(source, vec![sink], planned.payloads, 4, None)
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 3);
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"alpha");
        assert_eq!(std::fs::read(dst.join("b.txt")).unwrap(), b"bravo");
        assert_eq!(std::fs::read(dst.join("sub/c.txt")).unwrap(), b"charlie");
    }

    #[tokio::test]
    async fn streaming_pipeline_writes_as_payloads_arrive() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        for i in 0..5 {
            std::fs::write(src.join(format!("f{i}.txt")), format!("content-{i}")).unwrap();
        }

        let source = Arc::new(FsTransferSource::new(src.clone()));
        let sink = Arc::new(FsTransferSink::new(
            src.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        ));

        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut rx, scan_handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = rx.recv().await {
            headers.push(h);
        }
        let _ = scan_handle.await.unwrap().unwrap();

        let planned = crate::remote::transfer::payload::plan_transfer_payloads(
            headers,
            source.root(),
            Default::default(),
        )
        .unwrap();

        let (tx, rx) = mpsc::channel::<TransferPayload>(2);

        // Feed payloads one-at-a-time asynchronously.
        let feeder = tokio::spawn(async move {
            for p in planned.payloads {
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                let _ = tx.send(p).await;
            }
        });

        let outcome = execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None)
            .await
            .unwrap();

        let _ = feeder.await;

        assert_eq!(outcome.files_written, 5);
        for i in 0..5 {
            let content = std::fs::read_to_string(dst.join(format!("f{i}.txt"))).unwrap();
            assert_eq!(content, format!("content-{i}"));
        }
    }

    #[tokio::test]
    async fn streaming_pipeline_multi_sink_distributes_work() {
        // With 2 local sinks pointing at the SAME dst, all payloads land at
        // dst (round-robin determines which sink writes which file).
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        for i in 0..8 {
            std::fs::write(src.join(format!("f{i}.txt")), format!("n{i}")).unwrap();
        }

        let source = Arc::new(FsTransferSource::new(src.clone()));
        let mk_sink = || {
            Arc::new(FsTransferSink::new(
                src.clone(),
                dst.clone(),
                FsSinkConfig {
                    preserve_times: false,
                    dry_run: false,
                    checksum: None,
                    resume: false,
                    compare_mode: ComparisonMode::SizeMtime,
                },
            )) as Arc<dyn TransferSink>
        };

        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut rx, scan_handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = rx.recv().await {
            headers.push(h);
        }
        let _ = scan_handle.await.unwrap().unwrap();

        let planned = crate::remote::transfer::payload::plan_transfer_payloads(
            headers,
            source.root(),
            Default::default(),
        )
        .unwrap();

        let outcome = execute_sink_pipeline(
            source,
            vec![mk_sink(), mk_sink()],
            planned.payloads,
            4,
            None,
        )
        .await
        .unwrap();

        assert_eq!(outcome.files_written, 8);
        for i in 0..8 {
            let content = std::fs::read_to_string(dst.join(format!("f{i}.txt"))).unwrap();
            assert_eq!(content, format!("n{i}"));
        }
    }

    // -- wire-format fuzz harness -------------------------------------------------

    /// Feed a sequence of bytes through the receive-pipeline parser via a
    /// TCP socket pair and assert it never panics. The bytes are crafted to
    /// hit every record-type branch, boundary conditions, and common
    /// malformed inputs.
    #[tokio::test]
    async fn fuzz_wire_format_parser_does_not_panic() {
        use std::path::PathBuf;
        // Build a minimal FsTransferSink that writes to a temp dir.
        let tmp = tempdir().unwrap();
        let dst = tmp.path().to_path_buf();
        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
            PathBuf::from("/nonexistent-src"),
            dst,
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        ));

        // Each payload is a `(description, bytes)` pair fed to the parser.
        let payloads: Vec<(&str, Vec<u8>)> = vec![
            // ---- valid records ----
            ("empty stream (immediate END)", vec![DATA_PLANE_RECORD_END]),
            (
                "file record with zero-length path",
                encode_file(b"", &[], 0, 0),
            ),
            (
                "file record with zero-length body",
                encode_file(b"hello.txt", &[], 0, 0o644),
            ),
            (
                "file record with content",
                encode_file(b"a.txt", &b"payload"[..], 1_600_000_000, 0o755),
            ),
            ("tar shard with zero entries", encode_tar_shard(&[], 0, &[])),
            (
                "tar shard with one entry",
                encode_tar_shard(&[("f.txt", 5, 1_600_000_000, 0o644)], 5, &[0u8; 5]),
            ),
            ("block record", encode_block(b"f.txt", 0, b"hello")),
            (
                "block complete record",
                encode_block_complete(b"f.txt", 100),
            ),
            // ---- truncated / malformed ----
            ("empty stream", vec![]),
            ("truncated tag byte only", vec![0x00]),
            ("tag then EOF (file header truncated)", {
                let mut v = vec![DATA_PLANE_RECORD_FILE];
                v.extend_from_slice(&42u32.to_be_bytes()); // path_len
                                                           // no path bytes, no size, no mtime, no perms
                v
            }),
            ("file with path_len but no path bytes", {
                let mut v = vec![DATA_PLANE_RECORD_FILE];
                v.extend_from_slice(&5u32.to_be_bytes()); // claim 5 path bytes
                v.extend_from_slice(b"ab"); // only 2 bytes provided
                v
            }),
            ("file with path but no size/mtime/perms", {
                let mut v = vec![DATA_PLANE_RECORD_FILE];
                v.extend_from_slice(&3u32.to_be_bytes());
                v.extend_from_slice(b"abc");
                // size, mtime, perms all missing
                v
            }),
            ("file with header but no content bytes", {
                let mut v = vec![DATA_PLANE_RECORD_FILE];
                v.extend_from_slice(&3u32.to_be_bytes());
                v.extend_from_slice(b"abc");
                v.extend_from_slice(&100u64.to_be_bytes()); // size = 100
                v.extend_from_slice(&1i64.to_be_bytes()); // mtime
                v.extend_from_slice(&0o644u32.to_be_bytes()); // perms
                                                              // no content
                v
            }),
            ("file with oversized path_len (potential OOM guard)", {
                let mut v = vec![DATA_PLANE_RECORD_FILE];
                v.extend_from_slice(&(u32::MAX).to_be_bytes());
                v
            }),
            ("tar shard with huge entry count", {
                let mut v = vec![DATA_PLANE_RECORD_TAR_SHARD];
                v.extend_from_slice(&(u32::MAX).to_be_bytes());
                v
            }),
            ("tar shard truncated mid-entry header", {
                let mut v = vec![DATA_PLANE_RECORD_TAR_SHARD];
                v.extend_from_slice(&1u32.to_be_bytes()); // 1 entry
                v.extend_from_slice(&3u32.to_be_bytes());
                v.extend_from_slice(b"abc");
                // missing size, mtime, perms for that entry
                v
            }),
            ("tar shard with valid headers but truncated data_len", {
                let mut v = vec![DATA_PLANE_RECORD_TAR_SHARD];
                v.extend_from_slice(&1u32.to_be_bytes());
                let path = b"f.txt";
                v.extend_from_slice(&(path.len() as u32).to_be_bytes());
                v.extend_from_slice(path);
                v.extend_from_slice(&100u64.to_be_bytes()); // size
                v.extend_from_slice(&1i64.to_be_bytes()); // mtime
                v.extend_from_slice(&0o644u32.to_be_bytes()); // perms
                                                              // tar_size missing
                v
            }),
            ("tar shard with data_len but no tar bytes", {
                let mut v = vec![DATA_PLANE_RECORD_TAR_SHARD];
                v.extend_from_slice(&1u32.to_be_bytes());
                let path = b"f.txt";
                v.extend_from_slice(&(path.len() as u32).to_be_bytes());
                v.extend_from_slice(path);
                v.extend_from_slice(&100u64.to_be_bytes());
                v.extend_from_slice(&1i64.to_be_bytes());
                v.extend_from_slice(&0o644u32.to_be_bytes());
                v.extend_from_slice(&50u64.to_be_bytes()); // tar_size = 50
                                                           // no tar bytes
                v
            }),
            ("unknown record tag", vec![0xAB, DATA_PLANE_RECORD_END]),
            ("only unknown record tag (no END)", vec![0x42]),
            // ---- edge-case sizes ----
            ("file with declared size=MAX (no content)", {
                let mut v = vec![DATA_PLANE_RECORD_FILE];
                v.extend_from_slice(&7u32.to_be_bytes());
                v.extend_from_slice(b"max.bin");
                v.extend_from_slice(&u64::MAX.to_be_bytes()); // size = u64::MAX
                v.extend_from_slice(&0i64.to_be_bytes()); // mtime
                v.extend_from_slice(&0o644u32.to_be_bytes()); // perms
                                                              // no content — receiver should NOT panic / OOM trying to read u64::MAX bytes
                v
            }),
            (
                "block with zero-length payload",
                encode_block(b"f.txt", 0, b""),
            ),
            (
                "block with huge offset",
                encode_block(b"f.txt", u64::MAX, b"x"),
            ),
            (
                "block complete with zero total_size",
                encode_block_complete(b"f.txt", 0),
            ),
        ];

        for (_desc, bytes) in &payloads {
            // execute_receive_pipeline takes &mut TcpStream. Use a real
            // loopback listener so we exercise the actual code path that
            // production hits.
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            let addr = listener.local_addr().expect("local addr");
            let (client_res, server_res) =
                tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept(),);
            let mut writer = client_res.expect("connect");
            let (mut reader, _) = server_res.expect("accept");
            let sink = Arc::clone(&sink);

            // Writer task: push the crafted bytes.
            let bytes = bytes.clone();
            let writer_handle = tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                let _ = writer.write_all(&bytes).await;
                let _ = writer.shutdown().await;
            });

            // Reader task: try to parse. Must not panic.
            let result = execute_receive_pipeline(&mut reader, sink, None).await;

            let _ = writer_handle.await;

            // Success is fine (valid input). Error is fine (malformed input).
            // The ONLY failure mode we're checking for is a panic.
            let _ = result;
        }
    }

    // Fuzz-test helpers: encode wire-format records into byte vectors.

    fn encode_file(path: &[u8], content: &[u8], mtime: i64, perms: u32) -> Vec<u8> {
        let mut v = vec![DATA_PLANE_RECORD_FILE];
        v.extend_from_slice(&(path.len() as u32).to_be_bytes());
        v.extend_from_slice(path);
        v.extend_from_slice(&(content.len() as u64).to_be_bytes());
        v.extend_from_slice(&mtime.to_be_bytes());
        v.extend_from_slice(&perms.to_be_bytes());
        v.extend_from_slice(content);
        v
    }

    fn encode_tar_shard(
        entries: &[(&str, u64, i64, u32)],
        tar_size: u64,
        tar_data: &[u8],
    ) -> Vec<u8> {
        let mut v = vec![DATA_PLANE_RECORD_TAR_SHARD];
        v.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for &(path, size, mtime, perms) in entries {
            let p = path.as_bytes();
            v.extend_from_slice(&(p.len() as u32).to_be_bytes());
            v.extend_from_slice(p);
            v.extend_from_slice(&size.to_be_bytes());
            v.extend_from_slice(&mtime.to_be_bytes());
            v.extend_from_slice(&perms.to_be_bytes());
        }
        v.extend_from_slice(&tar_size.to_be_bytes());
        v.extend_from_slice(tar_data);
        v
    }

    fn encode_block(path: &[u8], offset: u64, content: &[u8]) -> Vec<u8> {
        let mut v = vec![DATA_PLANE_RECORD_BLOCK];
        v.extend_from_slice(&(path.len() as u32).to_be_bytes());
        v.extend_from_slice(path);
        v.extend_from_slice(&offset.to_be_bytes());
        v.extend_from_slice(&(content.len() as u32).to_be_bytes());
        v.extend_from_slice(content);
        v
    }

    fn encode_block_complete(path: &[u8], total_size: u64) -> Vec<u8> {
        let mut v = vec![DATA_PLANE_RECORD_BLOCK_COMPLETE];
        v.extend_from_slice(&(path.len() as u32).to_be_bytes());
        v.extend_from_slice(path);
        v.extend_from_slice(&total_size.to_be_bytes());
        v
    }

    /// POST_REVIEW_FIXES §1.1b regression. When a sink errors mid-
    /// pipeline, `execute_sink_pipeline_streaming` must return the
    /// underlying error message — not the previous generic "data
    /// plane pipeline closed unexpectedly" produced by
    /// `MultiStreamSender::queue` when its `tx.send` saw the receiver
    /// drop. The fix in `MultiStreamSender::queue` only works if this
    /// invariant holds at the pipeline layer.
    #[tokio::test]
    async fn pipeline_streaming_surfaces_underlying_sink_error() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"alpha").unwrap();

        let source = Arc::new(FsTransferSource::new(src.clone()));
        let dst = tmp.path().join("dst");
        let failing: Arc<dyn TransferSink> = Arc::new(FailingSink {
            marker: "synthetic sink failure: disk full",
            dst_root: dst,
        });

        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = header_rx.recv().await {
            headers.push(h);
        }
        let _scanned = scan_handle.await.unwrap().unwrap();

        let planned = crate::remote::transfer::payload::plan_transfer_payloads(
            headers,
            source.root(),
            Default::default(),
        )
        .unwrap();

        // Feed the planned payloads through the streaming variant
        // exactly as MultiStreamSender::connect does it: spawn the
        // pipeline in a task, push payloads via mpsc, then drop the
        // sender to signal end-of-stream.
        let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(4);
        let source_clone = Arc::clone(&source);
        let pipeline = tokio::spawn(async move {
            execute_sink_pipeline_streaming(source_clone, vec![failing], payload_rx, 4, None).await
        });

        for payload in planned.payloads {
            // Sink errors after the first write; later sends may
            // race the channel close. We only care that the
            // pipeline future resolves with the real error.
            let _ = payload_tx.send(payload).await;
        }
        drop(payload_tx);

        let result = pipeline.await.expect("pipeline task did not panic");
        let err = result.expect_err("pipeline must surface the sink error");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("synthetic sink failure: disk full"),
            "expected pipeline error to include underlying sink message; got:\n{}",
            msg
        );
    }

    /// audit-1c2: a receive that stalls (no bytes) must abort with the
    /// StallGuard's TimedOut rather than blocking forever. A duplex whose
    /// writer half is held open but never written keeps the first record-
    /// tag read perpetually Pending; the StallGuard wrapping it trips
    /// after the (short, test) idle window and the pipeline surfaces it.
    #[tokio::test]
    async fn receive_pipeline_aborts_on_stall() {
        use crate::remote::transfer::stall_guard::StallGuard;
        use std::path::PathBuf;

        let tmp = tempdir().unwrap();
        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
            PathBuf::from("/nonexistent-src"),
            tmp.path().to_path_buf(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        ));

        // Writer half held open (bound to a name) but never written → the
        // read side is perpetually Pending.
        let (rx, _tx) = tokio::io::duplex(64);
        let mut guarded = StallGuard::new(rx, std::time::Duration::from_millis(20));

        let err = execute_receive_pipeline(&mut guarded, sink, None)
            .await
            .expect_err("a stalled receive must abort, not hang");
        assert!(
            format!("{err:#}").contains("stalled"),
            "expected a StallGuard timeout in the error chain; got: {err:#}"
        );
    }
}

#[cfg(test)]
mod workqueue_tests {
    //! PR2: the shared work-queue must let a fast sink steal work a slow
    //! sink would otherwise have been assigned under the old round-robin
    //! dispatcher. Without work-stealing, N payloads split evenly across
    //! sinks and one slow sink bottlenecks the whole transfer; with it,
    //! the fast sink absorbs the bulk.
    use super::*;
    use crate::remote::transfer::sink::{SinkOutcome, TransferSink};
    use crate::remote::transfer::source::FsTransferSource;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tempfile::tempdir;

    /// Counts payloads it writes; optionally sleeps per payload to model
    /// a slow stream. Ignores the payload bytes — timing is governed
    /// purely by the configured delay, isolating the dispatch behaviour.
    struct CountingSink {
        delay: Duration,
        count: Arc<AtomicU64>,
        root: PathBuf,
    }

    #[async_trait::async_trait]
    impl TransferSink for CountingSink {
        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(SinkOutcome {
                files_written: 1,
                bytes_written: 0,
            })
        }
        fn root(&self) -> &Path {
            &self.root
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn fast_sink_steals_work_from_slow_sink() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let n = 40usize;
        for i in 0..n {
            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
        }

        let source = Arc::new(FsTransferSource::new(src.clone()));
        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = header_rx.recv().await {
            headers.push(h);
        }
        let _ = scan_handle.await.unwrap().unwrap();
        // Feed each file as its OWN payload (not via plan_transfer_payloads,
        // which bundles tiny files into a single tar shard — that would
        // leave only one payload and nothing to steal).
        assert_eq!(headers.len(), n, "expected one header per file");

        let fast_count = Arc::new(AtomicU64::new(0));
        let slow_count = Arc::new(AtomicU64::new(0));
        let fast: Arc<dyn TransferSink> = Arc::new(CountingSink {
            delay: Duration::ZERO,
            count: Arc::clone(&fast_count),
            root: PathBuf::from("/fast"),
        });
        let slow: Arc<dyn TransferSink> = Arc::new(CountingSink {
            delay: Duration::from_millis(20),
            count: Arc::clone(&slow_count),
            root: PathBuf::from("/slow"),
        });

        let (tx, rx) = mpsc::channel::<TransferPayload>(4);
        let feeder = tokio::spawn(async move {
            for h in headers {
                if tx.send(TransferPayload::File(h)).await.is_err() {
                    break;
                }
            }
        });
        let outcome = execute_sink_pipeline_streaming(source, vec![fast, slow], rx, 2, None)
            .await
            .expect("pipeline ok");
        let _ = feeder.await;

        let fast_n = fast_count.load(Ordering::Relaxed);
        let slow_n = slow_count.load(Ordering::Relaxed);
        assert_eq!(outcome.files_written, n, "every payload written once");
        assert_eq!(
            fast_n + slow_n,
            n as u64,
            "every payload accounted to exactly one sink"
        );
        // Round-robin would force ~20/20 and the slow sink would gate the
        // whole transfer. Work-stealing lets the zero-delay sink take the
        // overwhelming majority while the slow sink sits in its sleeps.
        assert!(
            fast_n > slow_n * 3,
            "fast sink should steal the bulk of the work: fast={fast_n} slow={slow_n}"
        );
    }

    /// Codex-review (PR2) regression: when the only sink errors, the
    /// forwarder must stop draining the producer promptly rather than
    /// continuing to pull every remaining payload. We feed a large
    /// payload set through a single always-failing sink and assert that
    /// (a) the pipeline surfaces the error, and (b) the forwarder
    /// consumed far fewer than all payloads before halting — proving the
    /// cancel flag short-circuits intake instead of draining to the end.
    struct ErrSink {
        root: PathBuf,
    }

    #[async_trait::async_trait]
    impl TransferSink for ErrSink {
        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
            eyre::bail!("synthetic immediate failure")
        }
        fn root(&self) -> &Path {
            &self.root
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn forwarder_stops_promptly_on_worker_error() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let n = 200usize;
        for i in 0..n {
            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
        }
        let source = Arc::new(FsTransferSource::new(src.clone()));
        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = header_rx.recv().await {
            headers.push(h);
        }
        let _ = scan_handle.await.unwrap().unwrap();
        assert_eq!(headers.len(), n);

        let sink: Arc<dyn TransferSink> = Arc::new(ErrSink {
            root: PathBuf::from("/err"),
        });

        // Count how many payloads the forwarder actually pulled from the
        // producer. With prefetch=1 and a single sink, the bounded queue
        // holds 1; once the sink errors and trips `cancelled`, the
        // forwarder must stop, so `sent` stays a tiny constant rather
        // than reaching n.
        let sent = Arc::new(AtomicU64::new(0));
        let (tx, rx) = mpsc::channel::<TransferPayload>(1);
        let sent_feeder = sent.clone();
        let feeder = tokio::spawn(async move {
            for h in headers {
                if tx.send(TransferPayload::File(h)).await.is_err() {
                    break;
                }
                sent_feeder.fetch_add(1, Ordering::Relaxed);
            }
        });

        let result = execute_sink_pipeline_streaming(source, vec![sink], rx, 1, None).await;
        let _ = feeder.await;

        assert!(result.is_err(), "pipeline must surface the sink error");
        let pulled = sent.load(Ordering::Relaxed);
        assert!(
            pulled < (n as u64) / 2,
            "forwarder should halt soon after the error, not drain all {n} payloads; pulled={pulled}"
        );
    }

    /// Reports each payload's real byte size so the executor's byte and
    /// file aggregation can be checked end to end without touching disk.
    struct ByteSink {
        bytes: Arc<AtomicU64>,
        root: PathBuf,
    }

    #[async_trait::async_trait]
    impl TransferSink for ByteSink {
        async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
            let (files, bytes): (usize, u64) = match &payload {
                PreparedPayload::File(h) => (1, h.size),
                PreparedPayload::TarShard { headers, .. } => {
                    (headers.len(), headers.iter().map(|h| h.size).sum())
                }
                _ => (0, 0),
            };
            self.bytes.fetch_add(bytes, Ordering::Relaxed);
            Ok(SinkOutcome {
                files_written: files,
                bytes_written: bytes,
            })
        }
        fn root(&self) -> &Path {
            &self.root
        }
    }

    /// REV4 ue-r2-1a (work-stealing as behaviour): byte and file totals
    /// stay correct when two sinks pull from the shared queue. Distinct
    /// per-file sizes mean any double-count or dropped payload shifts the
    /// totals, and the per-sink sum pins that every byte lands on exactly
    /// one sink.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn byte_and_file_totals_correct_under_work_stealing() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let n = 30usize;
        let mut expected_bytes = 0u64;
        for i in 0..n {
            // Distinct sizes so a miscount (double-add / drop) is visible.
            let body = vec![b'x'; i + 1];
            expected_bytes += body.len() as u64;
            std::fs::write(src.join(format!("f{i}.dat")), &body).unwrap();
        }
        let source = Arc::new(FsTransferSource::new(src.clone()));
        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = header_rx.recv().await {
            headers.push(h);
        }
        let _ = scan_handle.await.unwrap().unwrap();
        assert_eq!(headers.len(), n, "one header per file");

        let bytes_a = Arc::new(AtomicU64::new(0));
        let bytes_b = Arc::new(AtomicU64::new(0));
        let a: Arc<dyn TransferSink> = Arc::new(ByteSink {
            bytes: Arc::clone(&bytes_a),
            root: PathBuf::from("/a"),
        });
        let b: Arc<dyn TransferSink> = Arc::new(ByteSink {
            bytes: Arc::clone(&bytes_b),
            root: PathBuf::from("/b"),
        });

        let (tx, rx) = mpsc::channel::<TransferPayload>(4);
        let feeder = tokio::spawn(async move {
            for h in headers {
                if tx.send(TransferPayload::File(h)).await.is_err() {
                    break;
                }
            }
        });
        let outcome = execute_sink_pipeline_streaming(source, vec![a, b], rx, 2, None)
            .await
            .expect("pipeline ok");
        let _ = feeder.await;

        assert_eq!(outcome.files_written, n, "file total");
        assert_eq!(outcome.bytes_written, expected_bytes, "byte total");
        assert_eq!(
            bytes_a.load(Ordering::Relaxed) + bytes_b.load(Ordering::Relaxed),
            expected_bytes,
            "every byte accounted to exactly one sink, none double-counted"
        );
    }

    /// REV4 ue-r2-1a (cancellation): when the producer stops feeding and
    /// drops the channel mid-stream, the work-stealing executor winds
    /// down promptly — workers drain what was queued, run `finish`, and
    /// the call returns without hanging (the timeout is the no-hang
    /// assertion). Only the fed payloads complete; nothing past the
    /// cancellation point is invented.
    ///
    /// Hard-abort of in-flight workers on dropping the pipeline future
    /// itself is out of scope here: the workers are bare `tokio::spawn`
    /// (a `JoinHandle` drop does not abort the task), which is the
    /// AbortOnDrop family tracked under w4-1.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn producer_cancel_winds_down_pipeline_promptly() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let n = 50usize;
        for i in 0..n {
            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
        }
        let source = Arc::new(FsTransferSource::new(src.clone()));
        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = header_rx.recv().await {
            headers.push(h);
        }
        let _ = scan_handle.await.unwrap().unwrap();
        assert_eq!(headers.len(), n);

        let count = Arc::new(AtomicU64::new(0));
        let sink: Arc<dyn TransferSink> = Arc::new(CountingSink {
            delay: Duration::ZERO,
            count: Arc::clone(&count),
            root: PathBuf::from("/c"),
        });

        // Feed only the first 5 headers, then drop the sender to model a
        // cancelled / aborted producer.
        let (tx, rx) = mpsc::channel::<TransferPayload>(2);
        let feeder = tokio::spawn(async move {
            for h in headers.into_iter().take(5) {
                if tx.send(TransferPayload::File(h)).await.is_err() {
                    break;
                }
            }
            // `tx` dropped here → channel closes → pipeline must wind down.
        });

        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None),
        )
        .await
        .expect("pipeline must wind down promptly after producer cancels, not hang")
        .expect("graceful shutdown is not an error");
        let _ = feeder.await;

        assert_eq!(
            outcome.files_written, 5,
            "only the fed payloads are written"
        );
        assert_eq!(count.load(Ordering::Relaxed), 5);
    }
}
