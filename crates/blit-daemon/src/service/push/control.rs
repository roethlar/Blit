use super::super::admin::purge_extraneous_entries;
use super::super::util::{
    metadata_mtime_seconds, resolve_manifest_relative_path, resolve_module, resolve_relative_path,
};
use super::super::PushSender;
use super::data_plane::{
    accept_data_connection_stream, accept_data_connection_stream_resizable,
    bind_data_plane_listener, execute_grpc_fallback, generate_token, ResizeArm, TransferStats,
};
use crate::runtime::{ModuleConfig, RootExport};
use base64::{engine::general_purpose, Engine as _};
use blit_core::generated::{
    client_push_request, server_push_response, Ack, ClientPushRequest, DataPlaneResize,
    DataPlaneResizeAck, DataPlaneResizeOp, DataTransferNegotiation, FileHeader, FileList,
    PushSummary, ServerPushResponse,
};
use blit_core::remote::transfer::AbortOnDrop;
use std::collections::HashMap;
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tonic::{Status, Streaming};

const FILE_LIST_BATCH_MAX_ENTRIES: usize = 16 * 1024;
const FILE_LIST_BATCH_MAX_BYTES: usize = 512 * 1024;
const FILE_LIST_BATCH_MAX_DELAY: Duration = Duration::from_millis(25);
const FILE_LIST_EARLY_FLUSH_ENTRIES: usize = 128;
const FILE_LIST_EARLY_FLUSH_BYTES: usize = 64 * 1024;
const FILE_LIST_EARLY_FLUSH_DELAY: Duration = Duration::from_millis(5);
/// w4-4: manifest entries are buffered and their requires-upload
/// checks (canonical containment + stat — 3+ blocking syscalls each)
/// run in chunked `spawn_blocking` batches instead of inline on the
/// runtime per entry. Sized to the need-list early-flush threshold so
/// the reply cadence a fast-streaming push sees is unchanged; a
/// trickling manifest (client still scanning) is covered by the
/// delay trigger in [`manifest_drain_due`] instead — without it the
/// batcher's own 64 KiB/5 ms early-flush triggers could never fire
/// between chunk boundaries (codex w4-4 review, 1 Medium).
const MANIFEST_CHECK_CHUNK: usize = FILE_LIST_EARLY_FLUSH_ENTRIES;

/// w4-4 (codex review): when a buffered manifest entry has waited
/// this long, drain the chunk even if it is not full — mirrors the
/// batcher's `FILE_LIST_EARLY_FLUSH_DELAY` so a slowly-enumerating
/// client still gets its first need-list (and mid-manifest TCP
/// spin-up) within milliseconds, not after 128 entries trickle in.
/// Under a fast manifest stream 128 entries arrive well inside this
/// window, so the chunk cap dominates and syscall batching is kept.
const MANIFEST_CHECK_MAX_DELAY: Duration = FILE_LIST_EARLY_FLUSH_DELAY;

/// The two drain triggers for the buffered manifest checks: chunk
/// full, or the oldest buffered entry has waited past the delay
/// bound. Pure so the trigger contract is unit-testable.
fn manifest_drain_due(pending_len: usize, oldest_buffered: Option<Instant>) -> bool {
    pending_len >= MANIFEST_CHECK_CHUNK
        || matches!(oldest_buffered, Some(t) if t.elapsed() >= MANIFEST_CHECK_MAX_DELAY)
}

pub(crate) async fn handle_push_stream(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    mut stream: Streaming<ClientPushRequest>,
    tx: PushSender,
    force_grpc_data: bool,
    active_job: &crate::active_jobs::ActiveJobGuard,
) -> Result<(), Status> {
    let mut module: Option<ModuleConfig> = None;
    let mut files_to_upload: Vec<FileHeader> = Vec::new();
    let mut manifest_complete = false;
    let mut mirror_mode = false;
    let mut expected_rel_files: Vec<PathBuf> = Vec::new();
    let mut force_grpc_client = false;
    // R59 #1 F1/F2: state captured from PushHeader + ManifestComplete
    // so the purge phase can refuse on a partial scan (F1) and
    // honor the user's filter scope (F2).
    let mut require_complete_scan = false;
    let mut mirror_kind = blit_core::generated::MirrorMode::Unspecified;
    let mut purge_filter = blit_core::fs_enum::FileFilter::default();
    let mut scan_complete = false;
    let mut need_list_sender = FileListBatcher::new(tx.clone());
    // w4-4: manifest entries awaiting their chunked requires-upload
    // check (see MANIFEST_CHECK_CHUNK / drain_manifest_checks), and
    // when the oldest of them was buffered (drives the delay trigger;
    // evaluated on the next arrival, matching the batcher's own
    // push-time flush semantics).
    let mut pending_manifest: Vec<PendingManifestEntry> = Vec::new();
    let mut manifest_buffered_at: Option<Instant> = None;
    // design-2 / w4-1: `AbortOnDrop`, not a bare `JoinHandle` — an
    // early `?` return anywhere in this handler while a data-plane
    // task is running (or the `stream.message()` race below erroring)
    // must abort the accept/receive task instead of detaching it.
    let mut data_plane_handle: Option<AbortOnDrop<Result<TransferStats, Status>>> = None;
    let mut force_grpc_effective = force_grpc_data;
    let mut fallback_used = false;
    // ue-r2-2: the client's advertised resize capability (PushHeader
    // bit) and, once a resize-enabled TCP negotiation is out, the
    // channel that arms the acceptor for each ADD epoch.
    let mut client_supports_resize = false;
    let mut resize_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<ResizeArm>> = None;
    // ue-r2-2 review (codex): cumulative armed-stream count, seeded at
    // negotiation - the ADD refusal bound.
    let mut resize_live: u32 = 0;

    while let Some(request) = stream.message().await? {
        match request.payload {
            Some(client_push_request::Payload::Header(header)) => {
                if module.is_some() {
                    return Err(Status::invalid_argument("duplicate push header received"));
                }
                // Populate the ActiveJobs row now that we know
                // the endpoint (b-2-set-endpoint). The wire
                // `destination_path` is what the user supplied;
                // we record it verbatim — containment is
                // verified below when joining onto the module
                // root.
                active_job.set_endpoint(header.module.clone(), header.destination_path.clone());
                let mut config =
                    resolve_module(&modules, default_root.as_ref(), &header.module).await?;
                if config.read_only {
                    return Err(Status::permission_denied(format!(
                        "module '{}' is read-only",
                        config.name
                    )));
                }
                mirror_mode = header.mirror_mode;
                force_grpc_client = header.force_grpc;
                force_grpc_effective = force_grpc_data || force_grpc_client;
                // ue-r2-2: fold input (a) of the resize gate — the
                // peer's capability bit. (b) own support and (c)/(d)
                // the live-TCP conditions fold in at the negotiation
                // literals, which only exist on the TCP path.
                client_supports_resize = header.supports_stream_resize;
                // R59 #1: capture F1 / F2 fields from the new wire shape.
                require_complete_scan = header.require_complete_scan;
                mirror_kind = blit_core::generated::MirrorMode::try_from(header.mirror_kind)
                    .unwrap_or(blit_core::generated::MirrorMode::Unspecified);
                if let Some(wire_filter) = header.filter.as_ref() {
                    let mut f = blit_core::fs_enum::FileFilter::default();
                    f.include_files = wire_filter.include.clone();
                    f.exclude_files = wire_filter.exclude.clone();
                    f.min_size = wire_filter.min_size;
                    f.max_size = wire_filter.max_size;
                    f.min_age = wire_filter.min_age_secs.map(std::time::Duration::from_secs);
                    f.max_age = wire_filter.max_age_secs.map(std::time::Duration::from_secs);
                    f.reference_time = Some(std::time::SystemTime::now());
                    f.files_from = if wire_filter.files_from.is_empty() {
                        None
                    } else {
                        Some(wire_filter.files_from.iter().map(PathBuf::from).collect())
                    };
                    purge_filter = f;
                }
                let dest_path = header.destination_path.trim();
                if !dest_path.is_empty() {
                    let rel = resolve_relative_path(dest_path)?;
                    let new_path = config.path.join(rel);
                    // F2 / R13-F1: verify the rewritten module path
                    // stays inside the canonical module root before
                    // any downstream operation runs against it. Without
                    // this, a destination_path traversing an in-module
                    // symlink to outside would have all subsequent
                    // ops (file writes, mirror-purge enumeration)
                    // operate outside the module. Per-file write paths
                    // are already individually checked, but mirror
                    // purge enumerates module.path before any per-file
                    // check can fire.
                    blit_core::path_safety::verify_contained(&config.canonical_root, &new_path)
                        .map_err(|e| {
                            Status::permission_denied(format!(
                                "destination path containment: {e:#}"
                            ))
                        })?;
                    config.path = new_path;
                }
                module = Some(config);
                send_control_message(&tx, server_push_response::Payload::Ack(Ack {})).await?;
            }
            Some(
                client_push_request::Payload::TarShardHeader(_)
                | client_push_request::Payload::TarShardChunk(_)
                | client_push_request::Payload::TarShardComplete(_),
            ) => {
                return Err(Status::failed_precondition(
                    "tar shard payload received before manifest enumeration completed",
                ));
            }
            Some(client_push_request::Payload::FileManifest(mut file)) => {
                let module_ref = module.as_ref().ok_or_else(|| {
                    Status::failed_precondition("push manifest received before header")
                })?;
                let rel = resolve_manifest_relative_path(&file.relative_path)?;
                expected_rel_files.push(rel.clone());
                // Wire paths are canonically POSIX (`path_posix`). On
                // Windows, `PathBuf::to_string_lossy` re-joins the
                // validated components with backslashes, so the
                // need-list echoed paths the client's manifest lookup
                // (keyed by its own POSIX strings) could never match —
                // every nested-path push to a Windows daemon planned
                // zero payloads for those files and both ends stalled.
                let sanitized = blit_core::path_posix::relative_path_to_posix(&rel);
                file.relative_path = sanitized.clone();

                // w4-4: buffer the entry; the requires-upload check
                // (canonical containment + stat, 3+ blocking syscalls)
                // runs in chunked spawn_blocking batches instead of
                // inline on the runtime — a 1M-file push used to run
                // ~3M+ blocking syscalls on an executor worker.
                if manifest_buffered_at.is_none() {
                    manifest_buffered_at = Some(Instant::now());
                }
                pending_manifest.push(PendingManifestEntry {
                    rel,
                    sanitized,
                    file,
                });
                if manifest_drain_due(pending_manifest.len(), manifest_buffered_at) {
                    let flushed = drain_manifest_checks(
                        module_ref,
                        &mut pending_manifest,
                        &mut need_list_sender,
                        &mut files_to_upload,
                    )
                    .await?;
                    manifest_buffered_at = None;
                    // design-4: in forced-gRPC mode the early-flush branch
                    // must NOT announce the fallback negotiation here. The
                    // client reacts to Negotiation(tcp_fallback) by
                    // immediately streaming FileData on this same request
                    // stream — but this loop is still reading the manifest,
                    // and its FileData arm is a hard failed_precondition.
                    // That broke every forced-gRPC push of ≥128 files
                    // (FILE_LIST_EARLY_FLUSH_ENTRIES) and was timing-flaky
                    // near ~100. The post-manifest execute_grpc_fallback
                    // sends the one canonical fallback negotiation — the
                    // path every working small push already takes. Early
                    // negotiation only ever helped the TCP path (it starts
                    // the data plane for pipelining), so it is now TCP-only.
                    // (w4-4 moved this from per-entry to post-chunk-drain:
                    // the data plane still spins up mid-manifest on the
                    // first flush, at chunk granularity.)
                    if flushed && data_plane_handle.is_none() && !force_grpc_effective {
                        {
                            let listener = match bind_data_plane_listener().await {
                                Ok(l) => l,
                                Err(_) => {
                                    // Bind failed: flip to fallback mode but
                                    // stay quiet — announcing mid-manifest
                                    // would trip the same design-4 wedge.
                                    fallback_used = true;
                                    force_grpc_effective = true;
                                    continue;
                                }
                            };

                            let port = listener
                                .local_addr()
                                .map_err(|err| {
                                    Status::internal(format!("querying listener addr: {}", err))
                                })?
                                .port();

                            let token = generate_token()?;
                            let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);

                            let module_for_transfer = module_ref.clone();

                            let stream_target = engine_stream_proposal(&files_to_upload);
                            // ue-r2-2: full resize fold — peer bit AND
                            // own support AND a live TCP data plane
                            // (this literal only exists on that path;
                            // the fallback literal stays false).
                            let resize_on = client_supports_resize;
                            let epoch0_sub = if resize_on {
                                generate_resize_sub_token()?
                            } else {
                                Vec::new()
                            };
                            let transfer_task = if resize_on {
                                let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
                                resize_cmd_tx = Some(cmd_tx);
                                resize_live = stream_target.max(1);
                                AbortOnDrop::new(tokio::spawn(
                                    accept_data_connection_stream_resizable(
                                        listener,
                                        token.clone(),
                                        epoch0_sub.clone(),
                                        module_for_transfer,
                                        stream_target,
                                        cmd_rx,
                                    ),
                                ))
                            } else {
                                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream(
                                    listener,
                                    token.clone(),
                                    module_for_transfer,
                                    stream_target,
                                )))
                            };

                            send_control_message(
                                &tx,
                                server_push_response::Payload::Negotiation(
                                    DataTransferNegotiation {
                                        tcp_port: port as u32,
                                        one_time_token: token_string,
                                        tcp_fallback: false,
                                        stream_count: stream_target,
                                        // ue-r2-1e: the daemon is the
                                        // byte receiver on push — it
                                        // advertises its capacity so
                                        // the client's dial can ramp
                                        // within it.
                                        receiver_capacity: Some(
                                            blit_core::engine::local_receiver_capacity(),
                                        ),
                                        resize_enabled: resize_on,
                                        epoch0_sub_token: epoch0_sub,
                                    },
                                ),
                            )
                            .await?;

                            data_plane_handle = Some(transfer_task);
                        }
                    }
                }
            }
            Some(client_push_request::Payload::ManifestComplete(mc)) => {
                // w4-4: drain the sub-chunk remainder before leaving the
                // manifest phase — `need_list_sender.finish()` below and
                // the post-manifest negotiation both need the complete
                // need list / files_to_upload. No mid-manifest data-plane
                // spin-up here: the post-manifest path owns negotiation
                // once the manifest is done.
                if !pending_manifest.is_empty() {
                    let module_ref = module.as_ref().ok_or_else(|| {
                        Status::failed_precondition("push manifest received before header")
                    })?;
                    drain_manifest_checks(
                        module_ref,
                        &mut pending_manifest,
                        &mut need_list_sender,
                        &mut files_to_upload,
                    )
                    .await?;
                }
                manifest_complete = true;
                scan_complete = mc.scan_complete;
                break;
            }
            Some(client_push_request::Payload::FileData(_)) => {
                return Err(Status::failed_precondition(
                    "data payload received before negotiation",
                ));
            }
            Some(client_push_request::Payload::UploadComplete(_)) => {}
            Some(client_push_request::Payload::DataPlaneResize(req)) => {
                // ue-r2-2: an ADD can land while the manifest loop is
                // still running (the data plane starts at the early
                // flush) — same handling as the transfer phase.
                handle_resize_request(&tx, &resize_cmd_tx, &mut resize_live, req).await?;
            }
            None => {}
        }
    }

    let module = module.ok_or_else(|| Status::invalid_argument("push stream missing header"))?;
    if !manifest_complete {
        return Err(Status::invalid_argument(
            "push stream ended before manifest completion",
        ));
    }

    need_list_sender.finish().await?;

    let force_grpc_effective = force_grpc_effective || force_grpc_client;

    let transfer_stats = if files_to_upload.is_empty() {
        TransferStats::default()
    } else if force_grpc_effective {
        fallback_used = true;
        execute_grpc_fallback(&tx, &mut stream, &module, files_to_upload.clone()).await?
    } else {
        if data_plane_handle.is_none() {
            let listener = bind_data_plane_listener()
                .await
                .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
            let port = listener
                .local_addr()
                .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
                .port();
            let token = generate_token()?;
            let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
            let module_for_transfer = module.clone();
            let stream_target = engine_stream_proposal(&files_to_upload);
            // ue-r2-2: same fold as the early-flush site.
            let resize_on = client_supports_resize;
            let epoch0_sub = if resize_on {
                generate_resize_sub_token()?
            } else {
                Vec::new()
            };
            let transfer_task = if resize_on {
                let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
                resize_cmd_tx = Some(cmd_tx);
                resize_live = stream_target.max(1);
                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream_resizable(
                    listener,
                    token.clone(),
                    epoch0_sub.clone(),
                    module_for_transfer,
                    stream_target,
                    cmd_rx,
                )))
            } else {
                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream(
                    listener,
                    token.clone(),
                    module_for_transfer,
                    stream_target,
                )))
            };
            send_control_message(
                &tx,
                server_push_response::Payload::Negotiation(DataTransferNegotiation {
                    tcp_port: port as u32,
                    one_time_token: token_string,
                    tcp_fallback: false,
                    stream_count: stream_target,
                    // ue-r2-1e: see the early-flush negotiation above.
                    receiver_capacity: Some(blit_core::engine::local_receiver_capacity()),
                    resize_enabled: resize_on,
                    epoch0_sub_token: epoch0_sub,
                }),
            )
            .await?;
            data_plane_handle = Some(transfer_task);
        }

        if let Some(handle) = data_plane_handle.take() {
            // ue-r2-2: keep servicing the request stream while the data
            // plane runs — the client's DataPlaneResize frames arrive
            // mid-transfer. Everything else on the stream during this
            // phase was previously unread; ignore it the same way.
            //
            // design-2 / w4-1: `handle.join()` is pinned across loop
            // iterations rather than polling a bare `JoinHandle`
            // directly — `AbortOnDrop::join` holds `self` across its
            // internal await, so if `msg?` below errors and this
            // function returns, dropping `join_fut` mid-poll drops the
            // still-owned `AbortOnDrop`, which aborts the data-plane
            // task instead of detaching it.
            let mut client_stream_done = false;
            let join_fut = handle.join();
            tokio::pin!(join_fut);
            loop {
                tokio::select! {
                    res = &mut join_fut => {
                        break res.map_err(|_| Status::internal("data plane task cancelled"))??;
                    }
                    msg = stream.message(), if !client_stream_done => match msg? {
                        Some(request) => {
                            if let Some(client_push_request::Payload::DataPlaneResize(req)) =
                                request.payload
                            {
                                handle_resize_request(&tx, &resize_cmd_tx, &mut resize_live, req).await?;
                            }
                        }
                        None => client_stream_done = true,
                    },
                }
            }
        } else {
            TransferStats::default()
        }
    };

    let mut entries_deleted = 0u64;
    if mirror_mode {
        // R59 #1 F1: if the client demanded a complete source scan
        // (mandatory for mirror), refuse to purge when the actual
        // scan was incomplete. Pre-fix the daemon purged
        // unconditionally, so a permission error mid-scan caused
        // silent dest-side data loss when files absent from the
        // (incomplete) manifest were deleted from destination.
        if require_complete_scan && !scan_complete {
            return Err(Status::failed_precondition(
                "source scan was incomplete (unreadable paths); \
                 refusing to purge destination to prevent data loss. \
                 Resolve the unreadable source path(s) and retry.",
            ));
        }
        // R59 #1 F2: choose the purge filter based on mirror_kind.
        // ALL = full destination tree (no filter, historical
        // behavior). FILTERED_SUBSET (default) = honor user's filter
        // so out-of-scope destination entries aren't deleted.
        let scoped_filter = match mirror_kind {
            blit_core::generated::MirrorMode::All => blit_core::fs_enum::FileFilter::default(),
            // FilteredSubset is the default for mirror_mode=true with
            // an unspecified mirror_kind (back-compat: older clients
            // that don't send the field still get the safe scope).
            blit_core::generated::MirrorMode::Unspecified
            | blit_core::generated::MirrorMode::FilteredSubset
            | blit_core::generated::MirrorMode::Off => purge_filter.clone_without_cache(),
        };
        let purge_stats = purge_extraneous_entries(
            module.path.clone(),
            module.canonical_root.clone(),
            expected_rel_files,
            scoped_filter,
        )
        .await?;
        entries_deleted = purge_stats.total();
    }

    send_control_message(
        &tx,
        server_push_response::Payload::Summary(PushSummary {
            files_transferred: transfer_stats.files_transferred,
            bytes_transferred: transfer_stats.bytes_transferred,
            bytes_zero_copy: transfer_stats.bytes_zero_copy,
            tcp_fallback_used: fallback_used,
            entries_deleted,
        }),
    )
    .await?;

    Ok(())
}

/// ue-r2-2: 16 random bytes for the resize handshake suffix, minted
/// beside the one-time token (`Status`-mapped like `generate_token`).
fn generate_resize_sub_token() -> Result<Vec<u8>, Status> {
    blit_core::remote::transfer::generate_sub_token()
        .map_err(|err| Status::internal(format!("{err:#}")))
}

/// ue-r2-2: answer a client `DataPlaneResize`. ADD registers the
/// epoch's credential with the acceptor (which arms exactly one
/// accept, TTL-bounded) BEFORE the ack goes out, so the client's dial
/// can never race an unarmed listener. REMOVE is accounting-only —
/// the client retires a worker and that worker's END record tears the
/// daemon-side stream down through the normal path. Refusals
/// (`accepted: false`) cover: resize never negotiated, a malformed
/// credential, a target beyond this daemon's advertised ceiling, a
/// CUMULATIVE count at the ceiling (codex review: per-request target
/// checks alone would let replayed ADDs with fresh credentials grow
/// the worker set unboundedly — `resize_live` counts every armed ADD,
/// conservatively including ones whose dial later lapses), or an
/// acceptor that already finished.
async fn handle_resize_request(
    tx: &PushSender,
    resize_cmd_tx: &Option<tokio::sync::mpsc::UnboundedSender<ResizeArm>>,
    resize_live: &mut u32,
    req: DataPlaneResize,
) -> Result<(), Status> {
    let op = DataPlaneResizeOp::try_from(req.op).unwrap_or(DataPlaneResizeOp::Unspecified);
    let ceiling = blit_core::engine::local_receiver_capacity()
        .max_streams
        .max(1);
    let within_ceiling = req.target_stream_count <= ceiling && *resize_live < ceiling;
    let accepted = match (op, resize_cmd_tx) {
        (DataPlaneResizeOp::Add, Some(cmd_tx)) => {
            req.sub_token.len() == blit_core::remote::transfer::SUB_TOKEN_LEN
                && within_ceiling
                && cmd_tx
                    .send(ResizeArm {
                        epoch: req.epoch,
                        sub_token: req.sub_token.clone(),
                    })
                    .is_ok()
        }
        (DataPlaneResizeOp::Remove, Some(_)) => true,
        _ => false,
    };
    if accepted {
        match op {
            DataPlaneResizeOp::Add => *resize_live = resize_live.saturating_add(1),
            DataPlaneResizeOp::Remove => *resize_live = resize_live.saturating_sub(1).max(1),
            _ => {}
        }
    }
    if !accepted {
        log::warn!(
            "push: refusing DataPlaneResize (op {:?}, epoch {}, target {})",
            op,
            req.epoch,
            req.target_stream_count
        );
    }
    send_control_message(
        tx,
        server_push_response::Payload::DataPlaneResizeAck(DataPlaneResizeAck {
            epoch: req.epoch,
            effective_stream_count: req.target_stream_count,
            accepted,
        }),
    )
    .await
}

struct FileListBatcher {
    tx: PushSender,
    batch: Vec<String>,
    batch_bytes: usize,
    sent_any: bool,
    last_flush: Instant,
}

impl FileListBatcher {
    fn new(tx: PushSender) -> Self {
        Self {
            tx,
            batch: Vec::new(),
            batch_bytes: 0,
            sent_any: false,
            last_flush: Instant::now(),
        }
    }

    async fn push(&mut self, path: String) -> Result<bool, Status> {
        let entry_bytes = path.len();
        if self.batch.is_empty() {
            self.last_flush = Instant::now();
        }

        self.batch_bytes = self.batch_bytes.saturating_add(entry_bytes + 1);
        self.batch.push(path);

        if self.should_flush() {
            self.flush().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn flush(&mut self) -> Result<(), Status> {
        if self.batch.is_empty() {
            return Ok(());
        }

        self.sent_any = true;
        let payload = server_push_response::Payload::FilesToUpload(FileList {
            relative_paths: mem::take(&mut self.batch),
        });
        self.batch_bytes = 0;
        self.last_flush = Instant::now();
        send_control_message(&self.tx, payload).await
    }

    async fn finish(mut self) -> Result<(), Status> {
        if !self.batch.is_empty() {
            self.flush().await?;
        }
        // Always emit an empty FilesToUpload terminator so the client
        // can distinguish "more need_lists may arrive" from "no more
        // coming". Without this, the client races between its early-
        // finish condition (looks complete) and the daemon still
        // streaming batches — closes the data plane prematurely and
        // late manifest entries can't be queued.
        send_control_message(
            &self.tx,
            server_push_response::Payload::FilesToUpload(FileList {
                relative_paths: Vec::new(),
            }),
        )
        .await?;
        Ok(())
    }

    fn should_flush(&self) -> bool {
        if self.batch.is_empty() {
            return false;
        }

        if !self.sent_any
            && (self.batch.len() >= FILE_LIST_EARLY_FLUSH_ENTRIES
                || self.batch_bytes >= FILE_LIST_EARLY_FLUSH_BYTES
                || self.last_flush.elapsed() >= FILE_LIST_EARLY_FLUSH_DELAY)
        {
            return true;
        }

        self.batch.len() >= FILE_LIST_BATCH_MAX_ENTRIES
            || self.batch_bytes >= FILE_LIST_BATCH_MAX_BYTES
            || self.last_flush.elapsed() >= FILE_LIST_BATCH_MAX_DELAY
    }
}

pub(super) async fn send_control_message(
    tx: &PushSender,
    payload: server_push_response::Payload,
) -> Result<(), Status> {
    tx.send(Ok(ServerPushResponse {
        payload: Some(payload),
    }))
    .await
    .map_err(|_| Status::internal("failed to send push response"))
}

/// w4-4: one manifest entry buffered for the chunked requires-upload
/// check. `rel` is the validated relative path (containment input),
/// `sanitized` its canonical POSIX wire form (need-list echo), `file`
/// the header (already rewritten to the sanitized path) queued for
/// upload if the check says so.
struct PendingManifestEntry {
    rel: PathBuf,
    sanitized: String,
    file: FileHeader,
}

/// w4-4: run the buffered entries' requires-upload checks in ONE
/// `spawn_blocking` call (each check is a canonical-containment
/// ancestor walk plus a stat — blocking syscalls that used to run
/// per-entry on the runtime), then feed the need list in the original
/// manifest order. Returns true if any need-list push flushed a batch
/// to the client (the caller's cue to spin up the data plane
/// mid-manifest on the TCP path).
async fn drain_manifest_checks(
    module: &ModuleConfig,
    pending: &mut Vec<PendingManifestEntry>,
    need_list: &mut FileListBatcher,
    files_to_upload: &mut Vec<FileHeader>,
) -> Result<bool, Status> {
    if pending.is_empty() {
        return Ok(false);
    }
    let batch = mem::take(pending);
    let module_for_check = module.clone();
    let (batch, decisions) = tokio::task::spawn_blocking(move || {
        let decisions: Result<Vec<bool>, Status> = batch
            .iter()
            .map(|entry| file_requires_upload(&module_for_check, &entry.rel, &entry.file))
            .collect();
        (batch, decisions)
    })
    .await
    .map_err(|err| Status::internal(format!("manifest check task failed: {err}")))?;
    let decisions = decisions?;

    let mut any_flushed = false;
    for (entry, requires_upload) in batch.into_iter().zip(decisions) {
        if requires_upload {
            // w4-2: the 262,144-slot upload channel that used to sit
            // here is gone. Headers travel on the wire post-Phase-5;
            // the TCP receiver drained it into the void, and in gRPC
            // fallback nothing read it at all — so manifest entry
            // #262,145 wedged daemon and client forever with no
            // timeout in scope.
            // w5-1: was an unconditional per-file eprintln — stderr
            // spam proportional to file count. Debug-level now;
            // visible with BLIT_LOG=debug.
            log::debug!("push server queued {}", entry.sanitized);
            let flushed = need_list.push(entry.sanitized).await?;
            any_flushed = any_flushed || flushed;
            files_to_upload.push(entry.file);
        }
    }
    Ok(any_flushed)
}

fn file_requires_upload(
    module: &ModuleConfig,
    rel: &Path,
    header: &FileHeader,
) -> Result<bool, Status> {
    use super::super::util::resolve_contained_path;
    // F2: canonical containment check before stat. Same protection
    // as the actual write path — a symlink in the parent could
    // otherwise have us stat outside the module.
    let full_path = resolve_contained_path(module, rel)?;
    let requires_upload = match fs::metadata(&full_path) {
        Ok(meta) => {
            if !meta.is_file() {
                true
            } else {
                let same_size = meta.len() == header.size;
                let same_mtime = metadata_mtime_seconds(&meta)
                    .map(|seconds| seconds == header.mtime_seconds)
                    .unwrap_or(false);
                !(same_size && same_mtime)
            }
        }
        Err(_) => true,
    };
    Ok(requires_upload)
}

/// ue-r2-1f: the daemon's private `desired_streams` ladder retired
/// into the engine's shared shape-aware proposal (same table), clamped
/// to the receiver ceiling this daemon advertises in its
/// CapacityProfile. Single owner for the push stream-count start; the
/// client's dial clamps again on its side.
fn engine_stream_proposal(files: &[FileHeader]) -> u32 {
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    blit_core::engine::initial_stream_proposal(
        total_bytes,
        files.len(),
        blit_core::engine::local_receiver_capacity().max_streams as usize,
    )
}

#[cfg(test)]
mod data_plane_handle_abort_tests {
    //! design-2 / w4-1: `handle_push_stream`'s `data_plane_handle` was
    //! a bare `Option<JoinHandle<...>>`. Any early `?` return while a
    //! data-plane accept/receive task was running (the manifest
    //! loop's several fallible `send_control_message` calls, or the
    //! `stream.message()?` race in the post-manifest select loop)
    //! dropped the handle without aborting it, leaving the task
    //! running with no owner — unreachable by `CancelJob`. This pins
    //! the fix at the field-type level: wrapping the same
    //! `tokio::spawn` result in `AbortOnDrop` and dropping the
    //! `Option` (simulating the early-return path) must abort the
    //! task instead of detaching it. The full handler is exercised
    //! end-to-end elsewhere; reproducing a real gRPC push stream just
    //! to trigger this drop path would be disproportionate to the
    //! fix, which is purely "the field is wrapped now".

    use super::AbortOnDrop;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tonic::Status;

    #[tokio::test]
    async fn dropping_data_plane_handle_aborts_task() {
        let completed = Arc::new(AtomicBool::new(false));
        let completed_in_task = Arc::clone(&completed);
        let handle: Option<AbortOnDrop<Result<(), Status>>> =
            Some(AbortOnDrop::new(tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                completed_in_task.store(true, Ordering::SeqCst);
                Ok(())
            })));

        // Simulate an early `?` return out of `handle_push_stream`
        // while the field is still `Some`.
        drop(handle);

        // Wait well past the task's own 500ms delay — the margin has
        // to exceed the task's natural runtime, not just be "soon
        // after drop", or the assertion would pass regardless of
        // whether abort actually fired.
        tokio::time::sleep(Duration::from_millis(700)).await;
        assert!(
            !completed.load(Ordering::SeqCst),
            "data plane task ran to completion despite its handle being dropped — \
             data_plane_handle detached instead of aborting"
        );
    }
}

#[cfg(test)]
mod manifest_check_batch_tests {
    //! w4-4: the per-entry requires-upload check moved into chunked
    //! spawn_blocking batches (`drain_manifest_checks`). These pin the
    //! drained batch's decision parity with the old inline loop: an
    //! up-to-date file is skipped, a stale/missing file is queued with
    //! its sanitized POSIX wire path, manifest order is preserved, and
    //! the buffer comes back empty.

    use super::super::super::util::metadata_mtime_seconds;
    use super::*;

    fn test_module(root: &Path) -> ModuleConfig {
        ModuleConfig {
            name: "test".to_string(),
            path: root.to_path_buf(),
            canonical_root: root.canonicalize().expect("canonicalize test root"),
            read_only: false,
            _comment: None,
            delegation_allowed: true,
        }
    }

    fn pending(rel: &str, size: u64, mtime_seconds: i64) -> PendingManifestEntry {
        let rel_path = PathBuf::from(rel);
        let sanitized = blit_core::path_posix::relative_path_to_posix(&rel_path);
        PendingManifestEntry {
            rel: rel_path,
            sanitized: sanitized.clone(),
            file: FileHeader {
                relative_path: sanitized,
                size,
                mtime_seconds,
                permissions: 0o644,
                checksum: vec![],
            },
        }
    }

    #[tokio::test]
    async fn drain_skips_up_to_date_and_queues_stale_and_missing_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("same.txt"), b"12345").unwrap();
        std::fs::write(root.join("sub/stale.txt"), b"old-content").unwrap();

        let same_meta = std::fs::metadata(root.join("same.txt")).unwrap();
        let module = test_module(root);

        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        let mut need_list = FileListBatcher::new(tx);
        let mut files_to_upload: Vec<FileHeader> = Vec::new();
        let mut batch = vec![
            // Same size + mtime as on disk → up to date → skipped.
            pending(
                "same.txt",
                same_meta.len(),
                metadata_mtime_seconds(&same_meta).unwrap_or(0),
            ),
            // Exists but wrong size → stale → queued.
            pending("sub/stale.txt", 999, 0),
            // Not on disk → queued.
            pending("sub/missing.txt", 42, 0),
        ];

        let flushed =
            drain_manifest_checks(&module, &mut batch, &mut need_list, &mut files_to_upload)
                .await
                .expect("drain succeeds");

        assert!(batch.is_empty(), "drain consumes the pending buffer");
        assert!(
            !flushed,
            "3 entries stay under the early-flush threshold — no batch flush"
        );
        let queued: Vec<&str> = files_to_upload
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        assert_eq!(
            queued,
            vec!["sub/stale.txt", "sub/missing.txt"],
            "stale + missing queued with POSIX wire paths, manifest order kept, \
             up-to-date file skipped"
        );
    }

    #[test]
    fn drain_trigger_fires_on_chunk_or_delay() {
        // Chunk trigger: full chunk drains regardless of age.
        assert!(manifest_drain_due(
            MANIFEST_CHECK_CHUNK,
            Some(Instant::now())
        ));
        // Neither trigger: young, sub-chunk buffer waits.
        assert!(!manifest_drain_due(1, Some(Instant::now())));
        assert!(!manifest_drain_due(0, None));
        // Delay trigger (codex w4-4 review): a sub-chunk buffer whose
        // oldest entry has aged past the bound drains — a trickling
        // manifest must not wait for 128 entries to see its first
        // need-list flush.
        let stale = Instant::now() - (MANIFEST_CHECK_MAX_DELAY * 2);
        assert!(manifest_drain_due(1, Some(stale)));
    }

    #[tokio::test]
    async fn drain_on_empty_buffer_is_a_no_op() {
        let tmp = tempfile::tempdir().unwrap();
        let module = test_module(tmp.path());
        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        let mut need_list = FileListBatcher::new(tx);
        let mut files_to_upload: Vec<FileHeader> = Vec::new();
        let mut batch: Vec<PendingManifestEntry> = Vec::new();

        let flushed =
            drain_manifest_checks(&module, &mut batch, &mut need_list, &mut files_to_upload)
                .await
                .expect("empty drain succeeds");
        assert!(!flushed);
        assert!(files_to_upload.is_empty());
    }

    #[tokio::test]
    async fn drain_rejects_containment_escape() {
        // A traversal that survives lexical validation upstream must
        // still die at the canonical containment check inside the
        // batched path, exactly as the inline check did. Symlink the
        // escape (symlinks are the reason the check is canonical, not
        // lexical).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("module");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        #[cfg(not(unix))]
        {
            // Windows symlink creation needs privileges; skip the
            // escape arm there — the containment helper itself is
            // platform-shared and pinned by path_safety's own suite.
            return;
        }

        #[cfg(unix)]
        {
            let module = test_module(&root);
            let (tx, _rx) = tokio::sync::mpsc::channel(64);
            let mut need_list = FileListBatcher::new(tx);
            let mut files_to_upload: Vec<FileHeader> = Vec::new();
            let mut batch = vec![pending("link/escape.txt", 1, 0)];

            let err =
                drain_manifest_checks(&module, &mut batch, &mut need_list, &mut files_to_upload)
                    .await
                    .expect_err("containment escape must fail the drain");
            assert_eq!(err.code(), tonic::Code::PermissionDenied);
            assert!(files_to_upload.is_empty());
        }
    }
}
