pub mod helpers;
mod types;

pub use crate::remote::transfer::progress::ProgressEvent;
pub use types::{RemotePushProgress, RemotePushReport, TransferMode};

use self::helpers::{
    decode_token, destination_path, drain_pending_headers, map_status, module_and_path,
    send_manifest_complete, send_payload, spawn_response_task,
};
use crate::auto_tune::TuningParams;
use crate::buffer::BufferPool;
use crate::fs_enum::FileFilter;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::server_push_response::Payload as ServerPayload;
use crate::generated::ClientPushRequest;
use crate::generated::{FileHeader, PushSummary};
use crate::remote::endpoint::RemoteEndpoint;
use crate::remote::transfer::CONTROL_PLANE_CHUNK_SIZE;
use crate::remote::tuning::determine_remote_tuning;
use crate::transfer_plan::PlanOptions;
use eyre::{eyre, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;

use super::data_plane::DataPlaneSession;
use super::payload::{payload_file_count, TransferPayload};
// Push planning routes through the unified diff_planner module so the
// canonical entry point is the same regardless of origin type. Push's
// "diff" itself lives on the daemon side (NeedList) — see plan_push_payloads.
use crate::remote::transfer::diff_planner::plan_push_payloads as plan_transfer_payloads;
use crate::remote::transfer::pipeline::{execute_sink_pipeline, execute_sink_pipeline_streaming};
use crate::remote::transfer::progress::RemoteTransferProgress;
use crate::remote::transfer::sink::{DataPlaneSink, GrpcFallbackSink, SinkOutcome, TransferSink};
use crate::remote::transfer::source::TransferSource;

/// Await a pipeline JoinHandle and return the outcome with
/// consistent error wrapping. Used by both `MultiStreamSender::queue`
/// (via `drain_pipeline_error`) and `MultiStreamSender::finish` so
/// the failure-path messages are identical regardless of which side
/// noticed the pipeline died first.
///
/// Terminal states:
///
/// - `Ok(Ok(o))` → `Ok(o)` — pipeline returned cleanly with the
///   accumulated `SinkOutcome`.
/// - `Ok(Err(e))` → `Err(e.wrap_err("data plane pipeline failed"))` —
///   the eyre cause chain reads "data plane pipeline failed: <inner>"
///   so the underlying disk-full / channel-closed / etc. surfaces in
///   the user-visible message.
/// - `Err(join)` → `Err(eyre!("data plane pipeline panicked: {join}"))`
///   — the panic message surfaces rather than being hidden.
///
/// Closes R43 follow-up to R42-F2: previously `finish()` duplicated
/// these match arms while a comment claimed they routed through the
/// helper. Now there's actually one helper.
async fn drain_pipeline_outcome(handle: JoinHandle<Result<SinkOutcome>>) -> Result<SinkOutcome> {
    match handle.await {
        Ok(Ok(o)) => Ok(o),
        Ok(Err(e)) => Err(e.wrap_err("data plane pipeline failed")),
        Err(join) => Err(eyre!("data plane pipeline panicked: {join}")),
    }
}

/// Drain a pipeline JoinHandle into a clear `eyre::Report` for the
/// producer-side path where we already know the channel closed.
/// Wraps `drain_pipeline_outcome` so the failure formatting is
/// shared, then converts the `Ok` case (channel closed but pipeline
/// returned cleanly) into a diagnostic message — that combination is
/// the rare race in pipeline shutdown that we surface rather than
/// hide behind silence.
///
/// Extracted to a free function so the join-error-drain logic is
/// directly testable without spinning up a full
/// `MultiStreamSender::connect` (which requires real TCP streams).
/// Closes R42-F2.
async fn drain_pipeline_error(handle: JoinHandle<Result<SinkOutcome>>) -> eyre::Report {
    match drain_pipeline_outcome(handle).await {
        Ok(_) => eyre!(
            "data plane pipeline closed cleanly but the producer \
             channel was already closed — likely a race in \
             pipeline shutdown"
        ),
        Err(report) => report,
    }
}

/// Feeds payloads into N TCP data-plane sinks via the unified streaming
/// pipeline. The event loop pushes payloads as need-list batches arrive;
/// round-robin distribution across sinks is handled by the pipeline.
struct MultiStreamSender {
    payload_tx: Option<mpsc::Sender<TransferPayload>>,
    /// Pipeline JoinHandle. `Option` so `queue()` can `take()` it on
    /// the unhappy path: if `tx.send().await` fails the receiver has
    /// been dropped, which means the pipeline died with an error
    /// inside the spawned task. We surface that real error instead
    /// of the previous generic "data plane pipeline closed
    /// unexpectedly" string. POST_REVIEW_FIXES §1.1b.
    pipeline_handle: Option<JoinHandle<Result<SinkOutcome>>>,
    started: Instant,
}

impl MultiStreamSender {
    #[allow(clippy::too_many_arguments)]
    async fn connect(
        host: &str,
        port: u32,
        token: &[u8],
        chunk_bytes: usize,
        payload_prefetch: usize,
        stream_count: usize,
        trace: bool,
        source: Arc<dyn TransferSource>,
        tcp_buffer_size: Option<usize>,
        progress: Option<RemoteTransferProgress>,
    ) -> Result<Self> {
        let streams = stream_count.max(1);

        // Shared buffer pool across all sinks.
        let pool_size = streams * 2 + 4;
        let buffer_size = chunk_bytes.max(64 * 1024);
        let memory_budget = buffer_size * pool_size * 2;
        let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));

        let dst_root = PathBuf::from(format!("{}:{}", host, port));

        let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
        for _ in 0..streams {
            let session = DataPlaneSession::connect(
                host,
                port,
                token,
                chunk_bytes,
                payload_prefetch,
                trace,
                tcp_buffer_size,
                Arc::clone(&pool),
            )
            .await?;
            sinks.push(Arc::new(DataPlaneSink::new(
                session,
                source.clone(),
                dst_root.clone(),
            )));
        }

        let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));

        let source_clone = source.clone();
        let prefetch = payload_prefetch.max(1);
        let pipeline_handle = tokio::spawn(async move {
            execute_sink_pipeline_streaming(
                source_clone,
                sinks,
                payload_rx,
                prefetch,
                progress.as_ref(),
            )
            .await
        });

        Ok(Self {
            payload_tx: Some(payload_tx),
            pipeline_handle: Some(pipeline_handle),
            started: Instant::now(),
        })
    }

    /// Feed one or more payloads to the streaming pipeline.
    async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
        let tx = self
            .payload_tx
            .as_ref()
            .ok_or_else(|| eyre!("data plane sender already finished"))?;
        for payload in payloads {
            if tx.send(payload).await.is_err() {
                // Receiver dropped → pipeline task already exited.
                // Drain `pipeline_handle` to surface the underlying
                // error (sink worker errored, remote daemon closed,
                // disk full on dest…) instead of the previous
                // generic "data plane pipeline closed unexpectedly".
                // POST_REVIEW_FIXES §1.1b.
                drop(self.payload_tx.take());
                let handle = self
                    .pipeline_handle
                    .take()
                    .ok_or_else(|| eyre!("data plane pipeline handle missing"))?;
                return Err(drain_pipeline_error(handle).await);
            }
        }
        Ok(())
    }

    /// Close the payload channel and wait for the pipeline to drain.
    async fn finish(mut self) -> Result<()> {
        // Drop the sender so the pipeline sees end-of-stream.
        drop(self.payload_tx.take());
        let handle = self
            .pipeline_handle
            .take()
            .ok_or_else(|| eyre!("data plane pipeline handle missing"))?;
        // Route both Ok and Err through the shared drain helper so
        // the failure-path wrapping ("data plane pipeline failed:
        // <cause>" / "data plane pipeline panicked: <join>") matches
        // exactly what `queue()` would produce. R43 follow-up to
        // R42-F2 — earlier this was a hand-rolled match that
        // duplicated the helper's arms.
        let outcome = drain_pipeline_outcome(handle).await?;
        let elapsed = self.started.elapsed().as_secs_f64().max(1e-6);
        let throughput = (outcome.bytes_written as f64 * 8.0) / elapsed / 1e9;
        eprintln!(
            "[data-plane-client] aggregate {:.2} Gbps ({:.2} MiB in {:.2}s)",
            throughput.max(0.0),
            outcome.bytes_written as f64 / 1024.0 / 1024.0,
            elapsed
        );
        Ok(())
    }
}

fn ensure_remote_tuning(
    remote_tuning: &mut Option<TuningParams>,
    plan_options: &mut PlanOptions,
    size_hint: u64,
) -> TuningParams {
    if remote_tuning.is_none() {
        let tuning = determine_remote_tuning(size_hint);
        plan_options.chunk_bytes_override = Some(tuning.chunk_bytes);
        *remote_tuning = Some(tuning);
    }
    remote_tuning
        .as_ref()
        .cloned()
        .expect("remote_tuning set by preceding assignment")
}

fn effective_size_hint(requested: u64, manifest_bytes: u64) -> u64 {
    if requested > 0 {
        requested
    } else {
        manifest_bytes.max(1)
    }
}

fn prune_unrequested_payloads(
    payloads: &mut Vec<TransferPayload>,
    requested: &mut HashSet<String>,
) -> usize {
    let mut filtered: Vec<TransferPayload> = Vec::with_capacity(payloads.len());
    let mut skipped = 0usize;

    for payload in payloads.drain(..) {
        match payload {
            TransferPayload::File(header) => {
                if requested.remove(header.relative_path.as_str()) {
                    filtered.push(TransferPayload::File(header));
                } else {
                    skipped += 1;
                }
            }
            // Resume payloads originate on the receive side; the
            // outbound prune path never sees them.
            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
                skipped += 1;
            }
            TransferPayload::TarShard { headers } => {
                let mut kept_headers = Vec::with_capacity(headers.len());
                for header in headers {
                    if requested.remove(header.relative_path.as_str()) {
                        kept_headers.push(header);
                    } else {
                        skipped += 1;
                    }
                }
                if !kept_headers.is_empty() {
                    filtered.push(TransferPayload::TarShard {
                        headers: kept_headers,
                    });
                }
            }
        }
    }

    payloads.extend(filtered);
    skipped
}

pub struct RemotePushClient {
    endpoint: RemoteEndpoint,
    client: crate::generated::blit_client::BlitClient<tonic::transport::Channel>,
}

impl RemotePushClient {
    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
        let uri = endpoint.control_plane_uri();
        // audit-2: bound the connect (30s). Plain `BlitClient::connect`
        // has no deadline, so an unreachable destination daemon would
        // hang a remote push for the OS TCP timeout (60-127s). The outer
        // `tokio::time::timeout` is what bounds slow DNS too —
        // `connect_timeout` alone only bounds the post-resolution TCP
        // attempt (tonic/hyper-util resolve the name first).
        let conn = tonic::transport::Endpoint::from_shared(uri.clone())
            .map_err(|err| eyre::eyre!("invalid endpoint {}: {}", uri, err))?
            .connect_timeout(std::time::Duration::from_secs(30));
        let channel = tokio::time::timeout(std::time::Duration::from_secs(30), conn.connect())
            .await
            .map_err(|_| eyre::eyre!("connecting to {} timed out", uri))?
            .map_err(|err| eyre::eyre!("failed to connect to {}: {}", uri, err))?;
        let client = crate::generated::blit_client::BlitClient::new(channel);

        Ok(Self { endpoint, client })
    }

    pub async fn push(
        &mut self,
        source: Arc<dyn TransferSource>,
        filter: &FileFilter,
        mirror_mode: bool,
        mirror_kind: crate::generated::MirrorMode,
        force_grpc: bool,
        require_complete_scan: bool,
        progress: Option<&RemotePushProgress>,
        trace_data_plane: bool,
    ) -> Result<RemotePushReport> {
        let source_root = source.root();
        // We don't check source_root.exists() here because source might be remote/virtual.
        // If it's FsTransferSource, it should have been checked before creation or we trust it.

        let start = Instant::now();
        let mut first_payload_elapsed: Option<Duration> = None;

        let mut manifest_lookup: HashMap<String, FileHeader> = HashMap::new();
        let mut requested_files: HashSet<String> = HashSet::new();
        let mut plan_options = PlanOptions::default();
        let mut remote_tuning: Option<TuningParams> = None;
        let mut manifest_total_bytes: u64 = 0;
        let mut transfer_size_hint: u64 = 0;

        let (tx, rx) = mpsc::channel(32);
        let outbound = ReceiverStream::new(rx);

        let response_stream = self
            .client
            .push(outbound)
            .await
            .map_err(map_status)?
            .into_inner();
        let (mut response_rx, response_task) = spawn_response_task(response_stream);

        let (module, rel_path) = module_and_path(&self.endpoint)?;
        let destination_path = destination_path(&rel_path);

        // R59 #1 F2: translate the client's FileFilter to wire FilterSpec
        // so the daemon's purge enumerator can honor scope. Pre-fix the
        // daemon used FileFilter::default() and would delete user-excluded
        // destination entries it considered "extraneous".
        let wire_filter = crate::generated::FilterSpec {
            include: filter.include_files.clone(),
            exclude: filter.exclude_files.clone(),
            min_size: filter.min_size,
            max_size: filter.max_size,
            min_age_secs: filter.min_age.map(|d| d.as_secs()),
            max_age_secs: filter.max_age.map(|d| d.as_secs()),
            files_from: filter
                .files_from
                .as_ref()
                .map(|set| {
                    set.iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default(),
        };
        send_payload(
            &tx,
            ClientPayload::Header(crate::generated::PushHeader {
                module,
                mirror_mode,
                destination_path,
                force_grpc,
                filter: Some(wire_filter),
                mirror_kind: mirror_kind as i32,
                require_complete_scan,
            }),
        )
        .await?;

        let unreadable_paths: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let (manifest_rx, manifest_task) = source.scan(
            Some(filter.clone_without_cache()),
            Arc::clone(&unreadable_paths),
        );

        let mut manifest_rx = manifest_rx;

        let mut files_requested: Vec<String> = Vec::new();
        let mut pending_queue: VecDeque<String> = VecDeque::new();
        let mut fallback_upload_complete_sent = false;
        let mut fallback_files_sent: usize = 0;
        let mut need_list_received = false;
        let mut data_plane_sender: Option<MultiStreamSender> = None;
        let mut data_plane_outstanding: usize = 0;
        let mut data_plane_files_sent: usize = 0;
        let mut data_port: Option<u32> = None;
        let mut fallback_used = force_grpc;
        let mut summary: Option<PushSummary> = None;

        let mut transfer_mode = if force_grpc {
            TransferMode::Fallback
        } else {
            TransferMode::Undecided
        };
        // design-4: the daemon's wire contract rejects FileData while its
        // manifest loop is still running ("data payload received before
        // negotiation"). Even in forced-gRPC mode the client must therefore
        // hold its fallback payloads until the daemon announces
        // Negotiation(tcp_fallback) — which the daemon only sends after it
        // has seen ManifestComplete. Pre-fix, force_grpc initialized
        // Fallback mode and the first mid-manifest need-list batch
        // triggered FileData sends that raced the daemon's manifest loop:
        // every forced-gRPC push of ≥128 files (one early need-list flush)
        // died, and ~100 files was a coin flip.
        let mut fallback_negotiated = false;

        let mut manifest_done = false;
        // Track whether we received new need-list entries this iteration.
        // Don't finish the data plane until a full iteration passes with
        // no new entries — this ensures all in-flight gRPC batches arrive.
        let mut need_list_fresh: bool;
        // Set when the daemon signals "no more need_lists coming" by
        // sending an empty FilesToUpload terminator. Gates the early
        // finish() so we don't close the data plane while the daemon
        // is still streaming need_list batches.
        let mut need_lists_done = false;
        loop {
            if manifest_done && summary.is_some() {
                break;
            }
            need_list_fresh = false;

            tokio::select! {
                biased;

                maybe_message = response_rx.recv() => {
                    match maybe_message {
                        Some(Ok(message)) => {
                            match message.payload {
                                Some(ServerPayload::Ack(_)) => {}
                                Some(ServerPayload::FilesToUpload(list)) => {
                                    if list.relative_paths.is_empty() {
                                        // Empty terminator — no more need_lists coming.
                                        // Fall through to the bottom of the loop so the
                                        // early-finish check can fire on this iteration;
                                        // don't `continue` (that would skip the check
                                        // and require another response message to wake
                                        // the select, which never arrives).
                                        need_lists_done = true;
                                    } else {
                                    need_list_fresh = true;
                                    let mut rels = list.relative_paths;
                                    files_requested.extend(rels.iter().cloned());
                                    let newly_requested = rels.len();
                                    let mut batch_bytes = 0u64;
                                    for rel in &rels {
                                        requested_files.insert(rel.clone());
                                        if let Some(header) = manifest_lookup.get(rel) {
                                            batch_bytes =
                                                batch_bytes.saturating_add(header.size);
                                        }
                                        // w5-1: was an unconditional per-file
                                        // eprintln — stderr spam proportional
                                        // to file count. Debug-level now;
                                        // visible with BLIT_LOG=debug.
                                        log::debug!("push need-list includes {}", rel);
                                    }
                                    pending_queue.extend(rels.drain(..));
                                    transfer_size_hint =
                                        transfer_size_hint.saturating_add(batch_bytes);
                                    need_list_received = true;

                                    if !matches!(transfer_mode, TransferMode::Fallback) {
                                        data_plane_outstanding =
                                            data_plane_outstanding.saturating_add(newly_requested);
                                    }

                                    if let Some(progress) = progress {
                                        if newly_requested > 0 {
                                            progress.report_manifest_batch(newly_requested);
                                        }
                                    }

                                    match transfer_mode {
                                        TransferMode::Fallback => {
                                            // design-4: hold payloads until the
                                            // daemon's fallback negotiation;
                                            // until then entries just accumulate
                                            // in pending_queue (drained by the
                                            // Negotiation arm).
                                            if fallback_negotiated && need_list_received {
                                                let size_hint = effective_size_hint(
                                                    transfer_size_hint,
                                                    manifest_total_bytes,
                                                );
                                                let tuning = ensure_remote_tuning(
                                                    &mut remote_tuning,
                                                    &mut plan_options,
                                                    size_hint,
                                                );
                                                let result = stream_fallback_from_queue(
                                                    source.clone(),
                                                    &mut pending_queue,
                                                    &manifest_lookup,
                                                    &tx,
                                                    progress,
                                                    plan_options,
                                                    tuning.chunk_bytes,
                                                    tuning.initial_streams,
                                                    &unreadable_paths,
                                                ).await?;
                                                if result.files_sent > 0 {
                                                    fallback_files_sent =
                                                        fallback_files_sent.saturating_add(result.files_sent);
                                                }
                                                if result.payloads_dispatched
                                                    && first_payload_elapsed.is_none()
                                                {
                                                    first_payload_elapsed = Some(start.elapsed());
                                                }
                                            }
                                        }
                                        TransferMode::DataPlane => {
                                            if let Some(sender) = data_plane_sender.as_mut() {
                                                let headers =
                                                    drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                                if !headers.is_empty() {
                                                    let headers = source.check_availability(
                                                        headers,
                                                        Arc::clone(&unreadable_paths),
                                                    )
                                                    .await?;
                                                    if headers.is_empty() {
                                                        continue;
                                                    }
                                                    let size_hint = effective_size_hint(
                                                        transfer_size_hint,
                                                        manifest_total_bytes,
                                                    );
                                                    let _ = ensure_remote_tuning(
                                                        &mut remote_tuning,
                                                        &mut plan_options,
                                                        size_hint,
                                                    );
                                            let planned =
                                                plan_transfer_payloads(headers, source_root, plan_options)?;
                                            for payload in &planned.payloads {
                                                match payload {
                                                    TransferPayload::File(header) => {
                                                        // w5-1: was unconditional per-file
                                                        // eprintln; BLIT_LOG=debug shows it.
                                                        log::debug!(
                                                            "push enqueue {} for TCP stream",
                                                            header.relative_path
                                                        );
                                                    }
                                                    TransferPayload::TarShard { headers } => {
                                                        for header in headers {
                                                            log::debug!(
                                                                "push enqueue {} via tar shard",
                                                                header.relative_path
                                                            );
                                                        }
                                                    }
                                                    TransferPayload::FileBlock { .. }
                                                    | TransferPayload::FileBlockComplete { .. } => {
                                                        // Receive-only — never produced by the outbound planner.
                                                    }
                                                }
                                            }
                                            if !planned.payloads.is_empty() {
                                                        let sent = payload_file_count(&planned.payloads);
                                                        sender.queue(planned.payloads).await?;
                                                        if sent > 0 && first_payload_elapsed.is_none() {
                                                            first_payload_elapsed = Some(start.elapsed());
                                                        }
                                                        data_plane_files_sent += sent;
                                                        data_plane_outstanding =
                                                            data_plane_outstanding.saturating_sub(sent);
                                                    }
                                                }
                                            }
                                        }
                                        TransferMode::Undecided => {}
                                    }
                                    } // end else (non-empty need_list)
                                }
                                Some(ServerPayload::Negotiation(neg)) => {
                                    if neg.tcp_fallback {
                                        fallback_used = true;
                                        transfer_mode = TransferMode::Fallback;
                                        // design-4: only now may fallback
                                        // payloads flow — the daemon is past
                                        // its manifest loop and ready to
                                        // receive FileData.
                                        fallback_negotiated = true;

                                            if need_list_received {
                                                let size_hint = effective_size_hint(
                                                    transfer_size_hint,
                                                    manifest_total_bytes,
                                                );
                                            let tuning = ensure_remote_tuning(
                                                &mut remote_tuning,
                                                &mut plan_options,
                                                size_hint,
                                            );
                                            let result = stream_fallback_from_queue(
                                                source.clone(),
                                                &mut pending_queue,
                                                &manifest_lookup,
                                                &tx,
                                                progress,
                                                plan_options,
                                                tuning.chunk_bytes,
                                                tuning.prefetch_count.unwrap_or_else(|| tuning.initial_streams.max(1)),
                                                &unreadable_paths,
                                            ).await?;
                                            if result.files_sent > 0 {
                                                fallback_files_sent =
                                                    fallback_files_sent.saturating_add(result.files_sent);
                                            }
                                            if result.payloads_dispatched
                                                && first_payload_elapsed.is_none()
                                            {
                                                first_payload_elapsed = Some(start.elapsed());
                                            }
                                        }

                                        data_plane_outstanding = 0;
                                        if let Some(sender) = data_plane_sender.take() {
                                            sender.finish().await?;
                                        }
                                    } else {
                                        if neg.tcp_port == 0 {
                                            eyre::bail!("server reported zero data port for negotiated transfer");
                                        }

                                        let token_bytes = decode_token(&neg.one_time_token)?;
                                        let size_hint = effective_size_hint(
                                            transfer_size_hint,
                                            manifest_total_bytes,
                                        );
                                        let tuning = ensure_remote_tuning(
                                            &mut remote_tuning,
                                            &mut plan_options,
                                            size_hint,
                                        );
                                        if data_plane_sender.is_none() {
                                            let stream_target = neg
                                                .stream_count
                                                .max(1)
                                                .min(tuning.max_streams as u32) as usize;
                                            let payload_prefetch = tuning
                                                .prefetch_count
                                                .unwrap_or_else(|| tuning.initial_streams.max(1));
                                            let sender = MultiStreamSender::connect(
                                                &self.endpoint.host,
                                                neg.tcp_port,
                                                &token_bytes,
                                                tuning.chunk_bytes,
                                                payload_prefetch,
                                                stream_target,
                                                trace_data_plane,
                                                source.clone(),
                                                tuning.tcp_buffer_size,
                                                progress.cloned(),
                                            )
                                            .await?;
                                            data_plane_sender = Some(sender);
                                            data_port = Some(neg.tcp_port);
                                        }

                                        if let Some(sender) = data_plane_sender.as_mut() {
                                            let headers =
                                                drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                            if !headers.is_empty() {
                                                let headers = source
                                                    .check_availability(headers, unreadable_paths.clone())
                                                    .await?;
                                                if headers.is_empty() {
                                                    continue;
                                                }
                                            let mut planned = plan_transfer_payloads(
                                                headers,
                                                source_root,
                                                plan_options,
                                            )?;
                                            let skipped = prune_unrequested_payloads(
                                                &mut planned.payloads,
                                                &mut requested_files,
                                            );
                                            if skipped > 0 {
                                                log::debug!(
                                                    "push: daemon did not request {} payload file(s); skipping",
                                                    skipped
                                                );
                                            }
                                            if !planned.payloads.is_empty() {
                                                let sent = payload_file_count(&planned.payloads);
                                                sender.queue(planned.payloads).await?;
                                                if sent > 0 && first_payload_elapsed.is_none() {
                                                    first_payload_elapsed = Some(start.elapsed());
                                                }
                                                data_plane_files_sent += sent;
                                                data_plane_outstanding =
                                                    data_plane_outstanding.saturating_sub(sent);
                                                }
                                            }
                                        }
                                        transfer_mode = TransferMode::DataPlane;
                                    }
                                }
                                Some(ServerPayload::Summary(push_summary)) => {
                                    summary = Some(push_summary);
                                }
                                None => {}
                            }
                        }
                        Some(Err(err)) => return Err(err),
                        None => break,
                    }
                }
                maybe_header = manifest_rx.recv(), if !manifest_done => {
                    match maybe_header {
                        Some(header) => {
                            // Normalize path to ensure consistency with server requests
                            let rel = if header.relative_path.starts_with("./") {
                                header.relative_path[2..].to_string()
                            } else {
                                header.relative_path.clone()
                            };
                            let mut header = header;
                            header.relative_path = rel.clone();

                            // Check availability via the source abstraction
                            let available = source.check_availability(vec![header.clone()], Arc::clone(&unreadable_paths)).await?;
                            if available.is_empty() {
                                continue;
                            }

                            manifest_total_bytes =
                                manifest_total_bytes.saturating_add(header.size);
                            send_payload(&tx, ClientPayload::FileManifest(header.clone())).await?;
                            manifest_lookup.insert(rel.clone(), header);

                            match transfer_mode {
                                TransferMode::Fallback => {
                                    // design-4: never interleave FileData
                                    // between our own manifest sends — wait
                                    // for the daemon's fallback negotiation.
                                    if fallback_negotiated && need_list_received {
                                        let size_hint = effective_size_hint(
                                            transfer_size_hint,
                                            manifest_total_bytes,
                                        );
                                        let tuning = ensure_remote_tuning(
                                            &mut remote_tuning,
                                            &mut plan_options,
                                            size_hint,
                                        );
                                        let result = stream_fallback_from_queue(
                                            source.clone(),
                                            &mut pending_queue,
                                            &manifest_lookup,
                                            &tx,
                                            progress,
                                            plan_options,
                                            tuning.chunk_bytes,
                                            tuning.initial_streams,
                                            &unreadable_paths,
                                        ).await?;
                                        if result.files_sent > 0 {
                                            fallback_files_sent =
                                                fallback_files_sent.saturating_add(result.files_sent);
                                        }
                                        if result.payloads_dispatched
                                            && first_payload_elapsed.is_none()
                                        {
                                            first_payload_elapsed = Some(start.elapsed());
                                        }
                                    }
                                }
                                TransferMode::DataPlane => {
                                    if let Some(sender) = data_plane_sender.as_mut() {
                                        let headers =
                                            drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                        if !headers.is_empty() {
                                            let headers = source.check_availability(
                                                headers,
                                                Arc::clone(&unreadable_paths),
                                            )
                                            .await?;
                                            if headers.is_empty() {
                                                continue;
                                            }
                                            let size_hint = effective_size_hint(
                                                transfer_size_hint,
                                                manifest_total_bytes,
                                            );
                                            let _ = ensure_remote_tuning(
                                                &mut remote_tuning,
                                                &mut plan_options,
                                                size_hint,
                                            );
                                            let mut planned =
                                                plan_transfer_payloads(headers, source_root, plan_options)?;
                                            let skipped = prune_unrequested_payloads(
                                                &mut planned.payloads,
                                                &mut requested_files,
                                            );
                                            if skipped > 0 {
                                                log::debug!(
                                                    "push: daemon did not request {} payload file(s); skipping",
                                                    skipped
                                                );
                                            }
                                            for payload in &planned.payloads {
                                                match payload {
                                                    TransferPayload::File(header) => {
                                                        // w5-1: was unconditional per-file
                                                        // eprintln; BLIT_LOG=debug shows it.
                                                        log::debug!(
                                                            "push enqueue {} for TCP stream",
                                                            header.relative_path
                                                        );
                                                    }
                                                    TransferPayload::TarShard { headers } => {
                                                        for header in headers {
                                                            log::debug!(
                                                                "push enqueue {} via tar shard",
                                                                header.relative_path
                                                            );
                                                        }
                                                    }
                                                    TransferPayload::FileBlock { .. }
                                                    | TransferPayload::FileBlockComplete { .. } => {
                                                        // Receive-only — never produced by the outbound planner.
                                                    }
                                                }
                                            }
                                            if !planned.payloads.is_empty() {
                                                let sent = payload_file_count(&planned.payloads);
                                                sender.queue(planned.payloads).await?;
                                                if sent > 0 && first_payload_elapsed.is_none() {
                                                    first_payload_elapsed = Some(start.elapsed());
                                                }
                                                data_plane_files_sent += sent;
                                                data_plane_outstanding =
                                                    data_plane_outstanding.saturating_sub(sent);
                                            }
                                        }
                                    }
                                }
                                TransferMode::Undecided => {}
                            }
                        }
                        None => {
                            manifest_done = true;
                            // R59 #1 F1: report scan completeness to the
                            // daemon at ManifestComplete time. Walkdir
                            // errors land in `unreadable_paths` synchronously
                            // during the scan; the channel closing (None)
                            // guarantees the manifest task has finished
                            // pushing them, so reading here is race-free.
                            let scan_complete = unreadable_paths
                                .lock()
                                .map(|g| g.is_empty())
                                .unwrap_or(false);
                            send_manifest_complete(&tx, scan_complete).await?;
                        }
                    }
                }
            }

            if matches!(transfer_mode, TransferMode::Fallback)
                && !fallback_upload_complete_sent
                && !need_list_fresh
                && need_list_received
                && manifest_done
                && pending_queue.is_empty()
                && (files_requested.is_empty() || fallback_files_sent >= files_requested.len())
            {
                // Send UploadComplete via a temporary GrpcFallbackSink.
                let finish_sink = GrpcFallbackSink::new(
                    source.clone(),
                    tx.clone(),
                    CONTROL_PLANE_CHUNK_SIZE,
                    PathBuf::from("grpc-fallback"),
                );
                finish_sink.finish().await?;
                fallback_upload_complete_sent = true;
            }

            if matches!(transfer_mode, TransferMode::DataPlane)
                && !need_list_fresh
                && need_lists_done
                && pending_queue.is_empty()
                && manifest_done
                && data_plane_outstanding == 0
                && data_plane_files_sent >= files_requested.len()
            {
                if let Some(sender) = data_plane_sender.take() {
                    sender.finish().await?;
                }
            }
        }

        manifest_task
            .await
            .map_err(|err| eyre::eyre!("manifest enumeration task failed: {}", err))??;

        if let Some(sender) = data_plane_sender.take() {
            sender.finish().await?;
        }

        if let Err(join_err) = response_task.await {
            return Err(eyre::eyre!("response stream task failed: {}", join_err));
        }

        let summary = summary.ok_or_else(|| eyre::eyre!("push stream ended without summary"))?;

        let unreadable = unreadable_paths
            .lock()
            .map_err(|err| eyre!("manifest warnings poisoned: {}", err))?;
        if !unreadable.is_empty() {
            let preview: Vec<_> = unreadable.iter().take(5).cloned().collect();
            let mut message = format!(
                "{} file(s) were skipped due to permission or access errors: {}",
                unreadable.len(),
                preview.join(", ")
            );
            if unreadable.len() > preview.len() {
                let remaining = unreadable.len() - preview.len();
                message.push_str(&format!(" (and {} more)", remaining));
            }
            return Err(eyre!(message));
        }

        Ok(RemotePushReport {
            files_requested,
            fallback_used,
            data_port,
            summary,
            first_payload_elapsed,
        })
    }
}

async fn stream_fallback_from_queue(
    source: Arc<dyn TransferSource>,
    pending_queue: &mut VecDeque<String>,
    manifest_lookup: &HashMap<String, FileHeader>,
    tx: &mpsc::Sender<ClientPushRequest>,
    progress: Option<&RemotePushProgress>,
    plan_options: PlanOptions,
    chunk_bytes: usize,
    payload_prefetch: usize,
    unreadable: &Arc<Mutex<Vec<String>>>,
) -> Result<FallbackStreamResult> {
    let headers = drain_pending_headers(pending_queue, manifest_lookup);
    if headers.is_empty() {
        return Ok(FallbackStreamResult::empty());
    }

    let headers = source
        .check_availability(headers, Arc::clone(unreadable))
        .await?;
    if headers.is_empty() {
        return Ok(FallbackStreamResult::empty());
    }

    let planned = plan_transfer_payloads(headers, source.root(), plan_options)?;
    if planned.payloads.is_empty() {
        return Ok(FallbackStreamResult::empty());
    }

    let sent = payload_file_count(&planned.payloads);
    let control_chunk = if chunk_bytes == 0 {
        planned.chunk_bytes
    } else {
        chunk_bytes
    };
    let sink: Arc<dyn TransferSink> = Arc::new(GrpcFallbackSink::new(
        source.clone(),
        tx.clone(),
        control_chunk,
        PathBuf::from("grpc-fallback"),
    ));
    execute_sink_pipeline(
        source,
        vec![sink],
        planned.payloads,
        payload_prefetch,
        progress.map(|p| p as &RemoteTransferProgress),
    )
    .await?;

    Ok(FallbackStreamResult {
        files_sent: sent,
        payloads_dispatched: true,
    })
}

#[derive(Debug, Clone, Copy)]
struct FallbackStreamResult {
    files_sent: usize,
    payloads_dispatched: bool,
}

impl FallbackStreamResult {
    fn empty() -> Self {
        Self {
            files_sent: 0,
            payloads_dispatched: false,
        }
    }
}

#[cfg(test)]
mod drain_pipeline_error_tests {
    //! R42-F2: pin the producer-side join-error-drain logic that
    //! `MultiStreamSender::queue` and `MultiStreamSender::finish`
    //! depend on. The previous regression test only exercised
    //! `execute_sink_pipeline_streaming` — these tests cover the
    //! drain helper directly so a future change that replaces the
    //! match arms with a generic message gets caught here.
    //!
    //! `MultiStreamSender::queue` cannot easily be tested in
    //! isolation because `connect()` requires real TCP streams. The
    //! drain helper is the one piece of `queue`'s unhappy path that
    //! has interesting logic to verify.

    use super::*;
    use eyre::eyre;

    #[tokio::test]
    async fn drain_returns_pipeline_error_with_failed_prefix() {
        // Pipeline task returned `Err(...)`. drain_pipeline_error
        // must wrap with "data plane pipeline failed" so the eyre
        // cause chain reads `data plane pipeline failed: <inner>`.
        let handle: JoinHandle<Result<SinkOutcome>> =
            tokio::spawn(async { Err(eyre!("synthetic sink failure: disk full on dest")) });

        let report = drain_pipeline_error(handle).await;
        let msg = format!("{:#}", report);
        assert!(
            msg.contains("data plane pipeline failed"),
            "missing wrapping prefix in:\n{}",
            msg
        );
        assert!(
            msg.contains("synthetic sink failure: disk full on dest"),
            "underlying cause not preserved in:\n{}",
            msg
        );
    }

    #[tokio::test]
    async fn drain_returns_panic_message_for_aborted_task() {
        // Pipeline task panicked. drain_pipeline_error must produce
        // a "data plane pipeline panicked" message rather than
        // hiding the panic.
        let handle: JoinHandle<Result<SinkOutcome>> = tokio::spawn(async {
            panic!("synthetic pipeline panic");
        });

        let report = drain_pipeline_error(handle).await;
        let msg = format!("{:#}", report);
        assert!(
            msg.contains("data plane pipeline panicked"),
            "missing panic-prefix in:\n{}",
            msg
        );
    }

    // ── drain_pipeline_outcome (the underlying helper) ───────────────

    #[tokio::test]
    async fn drain_outcome_returns_value_on_clean_finish() {
        // Happy path: pipeline returned `Ok(SinkOutcome)`; the
        // helper passes it through. `finish()` relies on this to
        // get the per-run throughput numbers.
        let outcome = SinkOutcome {
            files_written: 7,
            bytes_written: 1024,
        };
        let cloned = outcome.clone();
        let handle: JoinHandle<Result<SinkOutcome>> = tokio::spawn(async move { Ok(cloned) });
        let got = drain_pipeline_outcome(handle).await.expect("clean finish");
        assert_eq!(got.files_written, outcome.files_written);
        assert_eq!(got.bytes_written, outcome.bytes_written);
    }

    #[tokio::test]
    async fn drain_outcome_wraps_pipeline_error() {
        // `finish()` failure path: pipeline returned Err. The helper
        // must wrap the same way `queue()`'s drain does so the user-
        // visible message is "data plane pipeline failed: <cause>"
        // regardless of which call site reported the error.
        let handle: JoinHandle<Result<SinkOutcome>> =
            tokio::spawn(async { Err(eyre!("synthetic apply error: ENOSPC")) });
        let err = drain_pipeline_outcome(handle).await.expect_err("must err");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("data plane pipeline failed"),
            "missing wrapping prefix in:\n{}",
            msg
        );
        assert!(
            msg.contains("synthetic apply error: ENOSPC"),
            "underlying cause not preserved in:\n{}",
            msg
        );
    }

    #[tokio::test]
    async fn drain_outcome_surfaces_panic_message() {
        let handle: JoinHandle<Result<SinkOutcome>> = tokio::spawn(async {
            panic!("synthetic finish-time panic");
        });
        let err = drain_pipeline_outcome(handle).await.expect_err("must err");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("data plane pipeline panicked"),
            "missing panic-prefix in:\n{}",
            msg
        );
    }

    #[tokio::test]
    async fn drain_diagnoses_clean_close_race() {
        // Pipeline task returned `Ok(SinkOutcome::default())` but
        // the drain helper was called anyway — meaning the
        // producer's tx.send saw the receiver dropped while the
        // pipeline was about to (or had just) finished cleanly. We
        // emit a diagnostic message rather than swallowing this as
        // success, so a future regression where this race becomes
        // common surfaces in logs.
        let handle: JoinHandle<Result<SinkOutcome>> =
            tokio::spawn(async { Ok(SinkOutcome::default()) });

        let report = drain_pipeline_error(handle).await;
        let msg = format!("{:#}", report);
        assert!(
            msg.contains("closed cleanly") && msg.contains("race"),
            "expected race diagnostic in:\n{}",
            msg
        );
    }
}
