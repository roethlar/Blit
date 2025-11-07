mod helpers;
mod types;

pub use types::{ProgressEvent, RemotePushProgress, RemotePushReport, TransferMode};

use self::helpers::{
    decode_token, destination_path, drain_pending_headers, map_status, module_and_path,
    send_manifest_complete, send_payload, spawn_manifest_task, spawn_response_task,
};
use crate::fs_enum::FileFilter;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::server_push_response::Payload as ServerPayload;
use crate::generated::ClientPushRequest;
use crate::generated::{FileHeader, PushSummary};
use crate::remote::endpoint::RemoteEndpoint;
use eyre::{bail, Result};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::data_plane::DataPlaneSession;
use super::payload::{
    payload_file_count, plan_transfer_payloads, transfer_payloads_via_control_plane,
};

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
        source_root: &Path,
        filter: &FileFilter,
        mirror_mode: bool,
        force_grpc: bool,
        progress: Option<&RemotePushProgress>,
        trace_data_plane: bool,
    ) -> Result<RemotePushReport> {
        if !source_root.exists() {
            bail!("source path does not exist: {}", source_root.display());
        }

        let start = Instant::now();
        let mut first_payload_elapsed: Option<Duration> = None;

        let mut manifest_lookup: HashMap<String, FileHeader> = HashMap::new();

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

        let (manifest_rx, manifest_task) =
            spawn_manifest_task(source_root.to_path_buf(), filter.clone_without_cache());

        let mut manifest_rx = manifest_rx;

        let mut files_requested: Vec<String> = Vec::new();
        let mut pending_queue: VecDeque<String> = VecDeque::new();
        let mut fallback_upload_complete_sent = false;
        let mut fallback_files_sent: usize = 0;
        let mut need_list_received = false;
        let mut data_plane_session: Option<DataPlaneSession> = None;
        let mut data_plane_outstanding: usize = 0;
        let mut data_plane_finished = false;
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
                                    pending_queue.extend(rels.drain(..));
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
                                                let result = stream_fallback_from_queue(
                                                    source_root,
                                                    &mut pending_queue,
                                                    &manifest_lookup,
                                                    &tx,
                                                    progress,
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
                                            if let Some(session) = data_plane_session.as_mut() {
                                                let headers =
                                                    drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                                if !headers.is_empty() {
                                                    let payloads =
                                                        plan_transfer_payloads(headers, source_root)?;
                                                    if !payloads.is_empty() {
                                                        let sent = payload_file_count(&payloads);
                                                        session
                                                            .send_payloads(source_root, payloads)
                                                            .await?;
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
                                        let result = stream_fallback_from_queue(
                                            source_root,
                                            &mut pending_queue,
                                            &manifest_lookup,
                                            &tx,
                                            progress,
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
                                        if let Some(session) = data_plane_session.as_mut() {
                                            if !data_plane_finished {
                                                session.finish().await?;
                                                data_plane_finished = true;
                                            }
                                        }
                                        data_plane_session = None;
                                    } else {
                                        if neg.tcp_port == 0 {
                                            eyre::bail!("server reported zero data port for negotiated transfer");
                                        }

                                        let token_bytes = decode_token(&neg.one_time_token)?;
                                        if data_plane_session.is_none() {
                                            let mut session = DataPlaneSession::connect(
                                                &self.endpoint.host,
                                                neg.tcp_port,
                                                &token_bytes,
                                                trace_data_plane,
                                            )
                                            .await?;
                                            let headers =
                                                drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                            if !headers.is_empty() {
                                                let payloads =
                                                    plan_transfer_payloads(headers, source_root)?;
                                                if !payloads.is_empty() {
                                                    let sent = payload_file_count(&payloads);
                                                    session
                                                        .send_payloads(source_root, payloads)
                                                        .await?;
                                                    if sent > 0 && first_payload_elapsed.is_none() {
                                                        first_payload_elapsed = Some(start.elapsed());
                                                    }
                                                    data_plane_outstanding =
                                                        data_plane_outstanding.saturating_sub(sent);
                                                }
                                            }
                                            data_plane_session = Some(session);
                                            data_port = Some(neg.tcp_port);
                                            transfer_mode = TransferMode::DataPlane;
                                        } else if let Some(session) = data_plane_session.as_mut() {
                                            let headers =
                                                drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                        if !headers.is_empty() {
                                            let payloads =
                                                plan_transfer_payloads(headers, source_root)?;
                                            if !payloads.is_empty() {
                                                let sent = payload_file_count(&payloads);
                                                session
                                                    .send_payloads(source_root, payloads)
                                                    .await?;
                                                if sent > 0 && first_payload_elapsed.is_none() {
                                                    first_payload_elapsed = Some(start.elapsed());
                                                }
                                                data_plane_outstanding =
                                                    data_plane_outstanding.saturating_sub(sent);
                                            }
                                        }
                                        }
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
                            let rel = header.relative_path.clone();
                            send_payload(&tx, ClientPayload::FileManifest(header.clone())).await?;
                            manifest_lookup.insert(rel.clone(), header);

                            match transfer_mode {
                                TransferMode::Fallback => {
                                    if need_list_received {
                                        let result = stream_fallback_from_queue(
                                            source_root,
                                            &mut pending_queue,
                                            &manifest_lookup,
                                            &tx,
                                            progress,
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
                                    if let Some(session) = data_plane_session.as_mut() {
                                        let headers =
                                            drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                        if !headers.is_empty() {
                                            let payloads =
                                                plan_transfer_payloads(headers, source_root)?;
                                            if !payloads.is_empty() {
                                                let sent = payload_file_count(&payloads);
                                                session
                                                    .send_payloads(source_root, payloads)
                                                    .await?;
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
                        source_root,
                        Vec::new(),
                        &tx,
                        true,
                        progress,
                    )
                    .await?;
                    fallback_upload_complete_sent = true;
                }
            }

            if matches!(transfer_mode, TransferMode::DataPlane) {
                if let Some(session) = data_plane_session.as_mut() {
                    if manifest_done
                        && pending_queue.is_empty()
                        && data_plane_outstanding == 0
                        && !data_plane_finished
                    {
                        session.finish().await?;
                        data_plane_finished = true;
                    }
                }
            }
        }

        manifest_task
            .await
            .map_err(|err| eyre::eyre!("manifest enumeration task failed: {}", err))??;

        if let Some(mut session) = data_plane_session.take() {
            if !data_plane_finished {
                session.finish().await?;
            }
        }

        if let Err(join_err) = response_task.await {
            return Err(eyre::eyre!("response stream task failed: {}", join_err));
        }

        let summary = summary.ok_or_else(|| eyre::eyre!("push stream ended without summary"))?;

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
    source_root: &Path,
    pending_queue: &mut VecDeque<String>,
    manifest_lookup: &HashMap<String, FileHeader>,
    tx: &mpsc::Sender<ClientPushRequest>,
    progress: Option<&RemotePushProgress>,
) -> Result<FallbackStreamResult> {
    let headers = drain_pending_headers(pending_queue, manifest_lookup);
    if headers.is_empty() {
        return Ok(FallbackStreamResult::empty());
    }

    let payloads = plan_transfer_payloads(headers, source_root)?;
    if payloads.is_empty() {
        return Ok(FallbackStreamResult::empty());
    }

    let sent = payload_file_count(&payloads);
    transfer_payloads_via_control_plane(source_root, payloads, tx, false, progress).await?;

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
