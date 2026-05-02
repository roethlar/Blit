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
/// Distributes payloads round-robin across `sinks` as they arrive. Each sink
/// runs as a separate tokio task: it reads payloads from its dedicated queue,
/// prepares them via `source.prepare_payload()`, writes them via
/// `sink.write_payload()`, and finally calls `sink.finish()`. Errors from any
/// worker propagate up.
///
/// `prefetch` controls the per-sink channel capacity — effectively the
/// preparation-in-flight limit per sink.
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
    let per_sink_capacity = prefetch.max(1);
    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));

    // Per-sink payload channels; dispatcher forwards round-robin to these.
    let mut sink_senders: Vec<mpsc::Sender<TransferPayload>> = Vec::with_capacity(sink_count);
    let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> =
        Vec::with_capacity(sink_count);

    for sink in sinks {
        let (tx, mut rx) = mpsc::channel::<TransferPayload>(per_sink_capacity);
        sink_senders.push(tx);
        let source_clone = source.clone();
        let progress_clone = progress.cloned();
        let total_clone = total.clone();
        sink_handles.push(tokio::spawn(async move {
            while let Some(payload) = rx.recv().await {
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
        }));
    }

    // Dispatcher: pull from the incoming channel, round-robin to sinks.
    // Uses async send (which applies backpressure) — if one sink is slower,
    // the dispatcher naturally blocks on that sink until it drains.
    let dispatcher = tokio::spawn(async move {
        let mut next = 0usize;
        while let Some(payload) = payload_rx.recv().await {
            let idx = next % sink_count;
            next = next.wrapping_add(1);
            if sink_senders[idx].send(payload).await.is_err() {
                // Sink worker dropped its receiver — treat as shutdown.
                return;
            }
        }
        // Drop senders so sink workers see end-of-stream and finish().
        drop(sink_senders);
    });

    // Wait for all sinks to finish and aggregate errors.
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
    let _ = dispatcher.await;

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
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

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
pub async fn execute_receive_pipeline(
    socket: &mut TcpStream,
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
                let outcome = sink.write_payload(payload).await.context("writing payload")?;
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
                let outcome = sink.write_payload(payload).await.context("writing payload")?;
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
                let outcome = sink.write_payload(payload).await.context("writing payload")?;
                total.merge(&outcome);
            }
            other => bail!("unknown data-plane record tag: 0x{:02X}", other),
        }
    }

    sink.finish().await.context("finalising sink")?;
    Ok(total)
}

async fn read_u32(socket: &mut TcpStream) -> Result<u32> {
    let mut buf = [0u8; 4];
    socket.read_exact(&mut buf).await.context("reading u32")?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_u64(socket: &mut TcpStream) -> Result<u64> {
    let mut buf = [0u8; 8];
    socket.read_exact(&mut buf).await.context("reading u64")?;
    Ok(u64::from_be_bytes(buf))
}

async fn read_i64(socket: &mut TcpStream) -> Result<i64> {
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
/// Maximum tar shard payload size (in bytes). The planner targets
/// 4–64 MiB; this is a generous upper bound that still forecloses
/// the "u64::MAX → terabyte allocation" attack.
const MAX_WIRE_TAR_SHARD_BYTES: usize = 1024 * 1024 * 1024;
/// Maximum single-block payload size on the resume protocol. Aligns
/// with `crate::copy::MAX_BLOCK_SIZE`.
const MAX_WIRE_BLOCK_BYTES: usize = 64 * 1024 * 1024;

async fn read_string(socket: &mut TcpStream) -> Result<String> {
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

async fn read_file_header(socket: &mut TcpStream) -> Result<FileHeader> {
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

async fn read_tar_shard(socket: &mut TcpStream) -> Result<(Vec<FileHeader>, Vec<u8>)> {
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
    use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
    use crate::remote::transfer::source::FsTransferSource;
    use std::sync::Mutex;
    use tempfile::tempdir;

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
            },
        ));

        // Each payload is a `(description, bytes)` pair fed to the parser.
        let payloads: Vec<(&str, Vec<u8>)> = vec![
            // ---- valid records ----
            ("empty stream (immediate END)",
             vec![DATA_PLANE_RECORD_END]),
            ("file record with zero-length path",
             encode_file(b"", &[], 0, 0)),
            ("file record with zero-length body",
             encode_file(b"hello.txt", &[], 0, 0o644)),
            ("file record with content",
             encode_file(b"a.txt", &b"payload"[..], 1_600_000_000, 0o755)),
            ("tar shard with zero entries",
             encode_tar_shard(&[], 0, &[])),
            ("tar shard with one entry",
             encode_tar_shard(
                 &[("f.txt", 5, 1_600_000_000, 0o644)],
                 5,
                 &[0u8; 5],
             )),
            ("block record",
             encode_block(b"f.txt", 0, b"hello")),
            ("block complete record",
             encode_block_complete(b"f.txt", 100)),
            // ---- truncated / malformed ----
            ("empty stream",
             vec![]),
            ("truncated tag byte only",
             vec![0x00]),
            ("tag then EOF (file header truncated)",
             {
                 let mut v = vec![DATA_PLANE_RECORD_FILE];
                 v.extend_from_slice(&42u32.to_be_bytes()); // path_len
                 // no path bytes, no size, no mtime, no perms
                 v
             }),
            ("file with path_len but no path bytes",
             {
                 let mut v = vec![DATA_PLANE_RECORD_FILE];
                 v.extend_from_slice(&5u32.to_be_bytes()); // claim 5 path bytes
                 v.extend_from_slice(b"ab"); // only 2 bytes provided
                 v
             }),
            ("file with path but no size/mtime/perms",
             {
                 let mut v = vec![DATA_PLANE_RECORD_FILE];
                 v.extend_from_slice(&3u32.to_be_bytes());
                 v.extend_from_slice(b"abc");
                 // size, mtime, perms all missing
                 v
             }),
            ("file with header but no content bytes",
             {
                 let mut v = vec![DATA_PLANE_RECORD_FILE];
                 v.extend_from_slice(&3u32.to_be_bytes());
                 v.extend_from_slice(b"abc");
                 v.extend_from_slice(&100u64.to_be_bytes()); // size = 100
                 v.extend_from_slice(&1i64.to_be_bytes()); // mtime
                 v.extend_from_slice(&0o644u32.to_be_bytes()); // perms
                 // no content
                 v
             }),
            ("file with oversized path_len (potential OOM guard)",
             {
                 let mut v = vec![DATA_PLANE_RECORD_FILE];
                 v.extend_from_slice(&(u32::MAX).to_be_bytes());
                 v
             }),
            ("tar shard with huge entry count",
             {
                 let mut v = vec![DATA_PLANE_RECORD_TAR_SHARD];
                 v.extend_from_slice(&(u32::MAX).to_be_bytes());
                 v
             }),
            ("tar shard truncated mid-entry header",
             {
                 let mut v = vec![DATA_PLANE_RECORD_TAR_SHARD];
                 v.extend_from_slice(&1u32.to_be_bytes()); // 1 entry
                 v.extend_from_slice(&3u32.to_be_bytes());
                 v.extend_from_slice(b"abc");
                 // missing size, mtime, perms for that entry
                 v
             }),
            ("tar shard with valid headers but truncated data_len",
             {
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
            ("tar shard with data_len but no tar bytes",
             {
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
            ("unknown record tag",
             vec![0xAB, DATA_PLANE_RECORD_END]),
            ("only unknown record tag (no END)",
             vec![0x42]),
            // ---- edge-case sizes ----
            ("file with declared size=MAX (no content)",
             {
                 let mut v = vec![DATA_PLANE_RECORD_FILE];
                 v.extend_from_slice(&7u32.to_be_bytes());
                 v.extend_from_slice(b"max.bin");
                 v.extend_from_slice(&u64::MAX.to_be_bytes()); // size = u64::MAX
                 v.extend_from_slice(&0i64.to_be_bytes()); // mtime
                 v.extend_from_slice(&0o644u32.to_be_bytes()); // perms
                 // no content — receiver should NOT panic / OOM trying to read u64::MAX bytes
                 v
             }),
            ("block with zero-length payload",
             encode_block(b"f.txt", 0, b"")),
            ("block with huge offset",
             encode_block(b"f.txt", u64::MAX, b"x")),
            ("block complete with zero total_size",
             encode_block_complete(b"f.txt", 0)),
        ];

        for (_desc, bytes) in &payloads {
            // execute_receive_pipeline takes &mut TcpStream. Use a real
            // loopback listener so we exercise the actual code path that
            // production hits.
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            let addr = listener.local_addr().expect("local addr");
            let (client_res, server_res) = tokio::join!(
                tokio::net::TcpStream::connect(addr),
                listener.accept(),
            );
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
}

