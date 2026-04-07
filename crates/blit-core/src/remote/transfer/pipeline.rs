//! Unified transfer pipeline: source → prepare → sink(s).
//!
//! All transfer paths (local→local, local→remote, remote→remote) use
//! [`execute_sink_pipeline`] to drive payloads from a [`TransferSource`]
//! through preparation and into one or more [`TransferSink`] instances.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use eyre::{Context, Result};
use futures::StreamExt;

use super::payload::{prepared_payload_stream, TransferPayload};
use super::progress::RemoteTransferProgress;
use super::sink::{SinkOutcome, TransferSink};
use super::source::TransferSource;

/// Execute a transfer pipeline: enumerate prepared payloads from `source`,
/// then write them into `sinks`.
///
/// - **Single sink** (local→local): payloads are written with up to
///   `sinks.len()` concurrency (i.e. 1 sink, sequential writes per sink
///   but the pipeline prefetches preparation).
/// - **Multiple sinks** (multi-stream TCP): payloads are distributed
///   round-robin across sinks, one concurrent write per sink.
///
/// Returns aggregated outcome across all sinks.
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

    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
    let sink_count = sinks.len();

    if sink_count == 1 {
        // Single sink: stream payloads sequentially through it.
        // Preparation is prefetched via prepared_payload_stream.
        let sink = &sinks[0];
        let mut stream = prepared_payload_stream(payloads, source, prefetch);

        while let Some(prepared) = stream.next().await {
            let payload = prepared.context("preparing payload")?;
            let file_count = match &payload {
                super::payload::PreparedPayload::File(h) => {
                    let name = h.relative_path.clone();
                    let size = h.size;
                    let outcome = sink
                        .write_payload(payload)
                        .await
                        .with_context(|| format!("writing {}", name))?;
                    if let Some(p) = progress {
                        p.report_file_complete(name, size);
                    }
                    let mut t = total.lock().unwrap();
                    t.merge(&outcome);
                    outcome.files_written
                }
                super::payload::PreparedPayload::TarShard { headers, .. } => {
                    let count = headers.len();
                    let names: Vec<_> = headers
                        .iter()
                        .map(|h| (h.relative_path.clone(), h.size))
                        .collect();
                    let outcome = sink
                        .write_payload(payload)
                        .await
                        .context("writing tar shard")?;
                    if let Some(p) = progress {
                        for (name, size) in &names {
                            p.report_file_complete(name.clone(), *size);
                        }
                    }
                    let mut t = total.lock().unwrap();
                    t.merge(&outcome);
                    count
                }
            };
            let _ = file_count;
        }

        sink.finish().await?;
    } else {
        // Multiple sinks: round-robin dispatch, one payload at a time per sink.
        // Each sink processes its payload concurrently with other sinks.
        let next_sink = AtomicUsize::new(0);
        let mut stream = prepared_payload_stream(payloads, source, prefetch);

        // Collect prepared payloads into per-sink queues, then execute.
        // This is simpler than true concurrent dispatch and avoids complex
        // stream splitting. Each sink gets ~1/N of the payloads.
        let mut queues: Vec<Vec<super::payload::PreparedPayload>> =
            (0..sink_count).map(|_| Vec::new()).collect();

        while let Some(prepared) = stream.next().await {
            let payload = prepared.context("preparing payload")?;
            let idx = next_sink.fetch_add(1, Ordering::Relaxed) % sink_count;
            queues[idx].push(payload);
        }

        // Execute each sink's queue concurrently.
        let mut handles = Vec::with_capacity(sink_count);
        for (idx, queue) in queues.into_iter().enumerate() {
            let sink = sinks[idx].clone();
            let total = total.clone();
            let progress = progress.cloned();
            handles.push(tokio::spawn(async move {
                for payload in queue {
                    let outcome = match &payload {
                        super::payload::PreparedPayload::File(h) => {
                            let name = h.relative_path.clone();
                            let size = h.size;
                            let o = sink
                                .write_payload(payload)
                                .await
                                .with_context(|| format!("writing {}", name))?;
                            if let Some(p) = &progress {
                                p.report_file_complete(name, size);
                            }
                            o
                        }
                        super::payload::PreparedPayload::TarShard { headers, .. } => {
                            let names: Vec<_> = headers
                                .iter()
                                .map(|h| (h.relative_path.clone(), h.size))
                                .collect();
                            let o = sink
                                .write_payload(payload)
                                .await
                                .context("writing tar shard")?;
                            if let Some(p) = &progress {
                                for (name, size) in &names {
                                    p.report_file_complete(name.clone(), *size);
                                }
                            }
                            o
                        }
                    };
                    let mut t = total.lock().unwrap();
                    t.merge(&outcome);
                }
                sink.finish().await?;
                Ok::<(), eyre::Report>(())
            }));
        }

        for handle in handles {
            handle.await.context("sink worker panicked")??;
        }
    }

    let result = total.lock().unwrap().clone();
    Ok(result)
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

        // Create test files
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

        // Scan source to get headers
        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut rx, handle) = source.scan(None, unreadable);
        let mut headers = Vec::new();
        while let Some(h) = rx.recv().await {
            headers.push(h);
        }
        let _total = handle.await.unwrap().unwrap();

        // Plan payloads
        let planned =
            crate::remote::transfer::payload::plan_transfer_payloads(headers, source.root(), Default::default())
                .unwrap();

        // Execute pipeline
        let outcome = execute_sink_pipeline(
            source,
            vec![sink],
            planned.payloads,
            4,
            None,
        )
        .await
        .unwrap();

        assert_eq!(outcome.files_written, 3);
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"alpha");
        assert_eq!(std::fs::read(dst.join("b.txt")).unwrap(), b"bravo");
        assert_eq!(std::fs::read(dst.join("sub/c.txt")).unwrap(), b"charlie");
    }
}
