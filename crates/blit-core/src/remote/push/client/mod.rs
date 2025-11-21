pub mod helpers;
mod types;

pub use crate::remote::transfer::progress::ProgressEvent;
pub use types::{RemotePushProgress, RemotePushReport, TransferMode};

use self::helpers::{
    decode_token, destination_path, drain_pending_headers, filter_readable_headers, map_status,
    module_and_path, record_unreadable_entry, send_manifest_complete, send_payload,
    spawn_manifest_task, spawn_response_task,
};
use crate::auto_tune::TuningParams;
use crate::fs_enum::FileFilter;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::server_push_response::Payload as ServerPayload;
use crate::generated::ClientPushRequest;
use crate::generated::{FileHeader, PushSummary};
use crate::remote::endpoint::RemoteEndpoint;
use crate::remote::transfer::CONTROL_PLANE_CHUNK_SIZE;
use crate::remote::tuning::determine_remote_tuning;
use crate::transfer_plan::PlanOptions;
use eyre::{bail, eyre, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;

use super::data_plane::DataPlaneSession;
use super::payload::{
    payload_file_count, plan_transfer_payloads, transfer_payloads_via_control_plane,
    TransferPayload, DEFAULT_PAYLOAD_PREFETCH,
};
use crate::remote::transfer::source::TransferSource;

const MIN_STREAM_BATCH_BYTES: u64 = 32 * 1024 * 1024;
const MAX_STREAM_BATCH_BYTES: u64 = 512 * 1024 * 1024;

fn calculate_batch_target(chunk_bytes: usize) -> u64 {
    let chunk_bytes = chunk_bytes.max(1) as u64;
    (chunk_bytes.saturating_mul(4)).clamp(MIN_STREAM_BATCH_BYTES, MAX_STREAM_BATCH_BYTES)
}

fn estimated_payload_bytes(payload: &TransferPayload) -> u64 {
    match payload {
        TransferPayload::File(header) => header.size,
        TransferPayload::TarShard { headers } => headers.iter().map(|h| h.size).sum(),
    }
}

struct MultiStreamSender {
    workers: Vec<mpsc::Sender<Option<Vec<TransferPayload>>>>,
    handles: Vec<JoinHandle<Result<StreamStats>>>,
    next_worker: usize,
    cancelled: Arc<AtomicBool>,
    batch_bytes_target: u64,
}

impl MultiStreamSender {
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
    ) -> Result<Self> {
        let streams = stream_count.max(1);
        let mut workers = Vec::with_capacity(streams);
        let mut handles = Vec::with_capacity(streams);

        for _ in 0..streams {
            let session =
                DataPlaneSession::connect(host, port, token, chunk_bytes, payload_prefetch, trace, tcp_buffer_size)
                    .await?;
            let (tx, rx) = mpsc::channel::<Option<Vec<TransferPayload>>>(4);
            let source_clone = Arc::clone(&source);
            let handle =
                tokio::spawn(async move { data_plane_worker(session, rx, source_clone).await });
            workers.push(tx);
            handles.push(handle);
        }

        Ok(Self {
            workers,
            handles,
            next_worker: 0,
            cancelled: Arc::new(AtomicBool::new(false)),
            batch_bytes_target: calculate_batch_target(chunk_bytes),
        })
    }

    async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
        if payloads.is_empty() {
            return Ok(());
        }

        if self.workers.len() == 1 {
            self.dispatch_batch(payloads).await?;
            return Ok(());
        }

        let mut batch = Vec::new();
        let mut batch_bytes = 0u64;
        let target = self.batch_bytes_target;
        for payload in payloads {
            batch_bytes = batch_bytes.saturating_add(estimated_payload_bytes(&payload));
            batch.push(payload);
            if batch_bytes >= target && !batch.is_empty() {
                let to_send = std::mem::take(&mut batch);
                batch_bytes = 0;
                self.dispatch_batch(to_send).await?;
            }
        }

        if !batch.is_empty() {
            self.dispatch_batch(batch).await?;
        }

        Ok(())
    }

    async fn dispatch_batch(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
        if payloads.is_empty() {
            return Ok(());
        }
        let idx = self.next_worker;
        self.next_worker = (self.next_worker + 1) % self.workers.len();
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(eyre!("data plane transfer cancelled"));
        }
        self.workers[idx]
            .send(Some(payloads))
            .await
            .map_err(|_| eyre!("data plane worker channel closed"))
    }

    async fn finish(mut self) -> Result<()> {
        for tx in &self.workers {
            tx.send(None)
                .await
                .map_err(|_| eyre!("data plane worker channel closed"))?;
        }
        let mut total_bytes = 0u64;
        for handle in self.handles.drain(..) {
            let stats = handle
                .await
                .map_err(|err| eyre!(format!("data plane worker panicked: {}", err)))??;
            let elapsed = stats.start.elapsed().as_secs_f64().max(1e-6);
            let throughput = (stats.bytes as f64 * 8.0) / elapsed / 1e9;
            eprintln!(
                "[data-plane-client] stream {:.2} Gbps ({:.2} MiB in {:.2}s)",
                throughput.max(0.0),
                stats.bytes as f64 / 1024.0 / 1024.0,
                elapsed
            );
            total_bytes = total_bytes.saturating_add(stats.bytes);
        }
        if total_bytes > 0 {
            eprintln!("[data-plane-client] total bytes sent {}", total_bytes);
        }
        Ok(())
    }
}

struct StreamStats {
    start: Instant,
    bytes: u64,
}

async fn data_plane_worker(
    mut session: DataPlaneSession,
    mut rx: mpsc::Receiver<Option<Vec<TransferPayload>>>,
    source: Arc<dyn TransferSource>,
) -> Result<StreamStats> {
    let start = Instant::now();
    while let Some(batch) = rx.recv().await {
        match batch {
            Some(payloads) => {
                session
                    .send_payloads(source.clone(), payloads)
                    .await?;
            }
            None => break,
        }
    }
    session.finish().await?;
    Ok(StreamStats {
        start,
        bytes: session.bytes_sent(),
    })
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
    remote_tuning.as_ref().cloned().unwrap()
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

    payloads.extend(filtered.into_iter());
    skipped
}

pub struct RemotePushClient {
    endpoint: RemoteEndpoint,
    client: crate::generated::blit_client::BlitClient<tonic::transport::Channel>,
}

impl RemotePushClient {
    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
        let uri = endpoint.control_plane_uri();
        let client = crate::generated::blit_client::BlitClient::connect(uri.clone())
            .await
            .map_err(|err| eyre::eyre!("failed to connect to {}: {}", uri, err))?;

        Ok(Self { endpoint, client })
    }

    pub async fn push(
        &mut self,
        source: Arc<dyn TransferSource>,
        filter: &FileFilter,
        mirror_mode: bool,
        force_grpc: bool,
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

        send_payload(
            &tx,
            ClientPayload::Header(crate::generated::PushHeader {
                module,
                mirror_mode,
                destination_path,
                force_grpc,
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
        let mut data_port: Option<u32> = None;
        let mut fallback_used = force_grpc;
        let mut summary: Option<PushSummary> = None;

        let mut transfer_mode = if force_grpc {
            TransferMode::Fallback
        } else {
            TransferMode::Undecided
        };

        let mut manifest_done = false;
        loop {
            if manifest_done && summary.is_some() {
                break;
            }

            tokio::select! {
                biased;

                maybe_message = response_rx.recv() => {
                    match maybe_message {
                        Some(Ok(message)) => {
                            match message.payload {
                                Some(ServerPayload::Ack(_)) => {}
                                Some(ServerPayload::FilesToUpload(list)) => {
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
                                        eprintln!("[push] need-list includes {}", rel);
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
                                            for payload in &planned.payloads {
                                                match payload {
                                                    TransferPayload::File(header) => {
                                                        eprintln!(
                                                            "[push] enqueue {} for TCP stream",
                                                            header.relative_path
                                                        );
                                                    }
                                                    TransferPayload::TarShard { headers } => {
                                                        for header in headers {
                                                            eprintln!(
                                                                "[push] enqueue {} via tar shard",
                                                                header.relative_path
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            if !planned.payloads.is_empty() {
                                                        let sent = payload_file_count(&planned.payloads);
                                                        sender.queue(planned.payloads).await?;
                                                        if sent > 0 && first_payload_elapsed.is_none() {
                                                            first_payload_elapsed = Some(start.elapsed());
                                                        }
                                                        data_plane_outstanding =
                                                            data_plane_outstanding.saturating_sub(sent);
                                                    }
                                                }
                                            }
                                        }
                                        TransferMode::Undecided => {}
                                    }
                                }
                                Some(ServerPayload::Negotiation(neg)) => {
                                    if neg.tcp_fallback {
                                        fallback_used = true;
                                        transfer_mode = TransferMode::Fallback;

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
                                                eprintln!(
                                                    "[push] daemon did not request {} payload file(s); skipping",
                                                    skipped
                                                );
                                            }
                                            if !planned.payloads.is_empty() {
                                                let sent = payload_file_count(&planned.payloads);
                                                sender.queue(planned.payloads).await?;
                                                if sent > 0 && first_payload_elapsed.is_none() {
                                                    first_payload_elapsed = Some(start.elapsed());
                                                }
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
                                                eprintln!(
                                                    "[push] daemon did not request {} payload file(s); skipping",
                                                    skipped
                                                );
                                            }
                                            for payload in &planned.payloads {
                                                match payload {
                                                    TransferPayload::File(header) => {
                                                        eprintln!(
                                                            "[push] enqueue {} for TCP stream",
                                                            header.relative_path
                                                        );
                                                    }
                                                    TransferPayload::TarShard { headers } => {
                                                        for header in headers {
                                                            eprintln!(
                                                                "[push] enqueue {} via tar shard",
                                                                header.relative_path
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            if !planned.payloads.is_empty() {
                                                let sent = payload_file_count(&planned.payloads);
                                                sender.queue(planned.payloads).await?;
                                                if sent > 0 && first_payload_elapsed.is_none() {
                                                    first_payload_elapsed = Some(start.elapsed());
                                                }
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
                            send_manifest_complete(&tx).await?;
                        }
                    }
                }
            }

            if matches!(transfer_mode, TransferMode::Fallback) {
                if !fallback_upload_complete_sent
                    && need_list_received
                    && manifest_done
                    && pending_queue.is_empty()
                    && (files_requested.is_empty() || fallback_files_sent >= files_requested.len())
                {
                    transfer_payloads_via_control_plane(
                        source.clone(),
                        Vec::new(),
                        &tx,
                        true,
                        progress,
                        remote_tuning
                            .as_ref()
                            .map(|t| t.chunk_bytes)
                            .unwrap_or(CONTROL_PLANE_CHUNK_SIZE),
                        remote_tuning
                            .as_ref()
                            .map(|t| t.initial_streams)
                            .unwrap_or(DEFAULT_PAYLOAD_PREFETCH),
                    )
                    .await?;
                    fallback_upload_complete_sent = true;
                }
            }

            if matches!(transfer_mode, TransferMode::DataPlane) {
                if pending_queue.is_empty() && manifest_done && data_plane_outstanding == 0 {
                    if let Some(sender) = data_plane_sender.take() {
                        sender.finish().await?;
                    }
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

    let headers = source.check_availability(headers, Arc::clone(unreadable)).await?;
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
    transfer_payloads_via_control_plane(
        source,
        planned.payloads,
        tx,
        false,
        progress,
        control_chunk,
        payload_prefetch,
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
