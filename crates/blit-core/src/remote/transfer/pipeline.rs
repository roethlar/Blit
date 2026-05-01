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
                let payload = PreparedPayload::FileBlockComplete {
                    relative_path: path,
                    total_size,
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

async fn read_string(socket: &mut TcpStream) -> Result<String> {
    let len = read_u32(socket).await? as usize;
    let mut buf = vec![0u8; len];
    socket
        .read_exact(&mut buf)
        .await
        .context("reading string bytes")?;
    String::from_utf8(buf).context("invalid UTF-8 in data-plane string")
}

async fn read_file_header(socket: &mut TcpStream) -> Result<FileHeader> {
    let path = read_string(socket).await?;
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
    let mut headers = Vec::with_capacity(count);
    for _ in 0..count {
        let path = read_string(socket).await?;
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
    let tar_size = read_u64(socket).await? as usize;
    let mut data = vec![0u8; tar_size];
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
}

