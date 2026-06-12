use super::super::admin::purge_extraneous_entries;
use super::super::util::{
    metadata_mtime_seconds, resolve_manifest_relative_path, resolve_module, resolve_relative_path,
};
use super::super::PushSender;
use super::data_plane::{
    accept_data_connection_stream, bind_data_plane_listener, execute_grpc_fallback, generate_token,
    TransferStats,
};
use crate::runtime::{ModuleConfig, RootExport};
use base64::{engine::general_purpose, Engine as _};
use blit_core::generated::{
    client_push_request, server_push_response, Ack, ClientPushRequest, DataTransferNegotiation,
    FileHeader, FileList, PushSummary, ServerPushResponse,
};
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
    let mut data_plane_handle: Option<tokio::task::JoinHandle<Result<TransferStats, Status>>> =
        None;
    let mut force_grpc_effective = force_grpc_data;
    let mut fallback_used = false;

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
                let sanitized = rel.to_string_lossy().to_string();

                if file_requires_upload(module_ref, &rel, &file)? {
                    file.relative_path = sanitized.clone();
                    // w4-2: the 262,144-slot upload channel that used to sit
                    // here is gone. Headers travel on the wire post-Phase-5;
                    // the TCP receiver drained it into the void, and in gRPC
                    // fallback nothing read it at all — so manifest entry
                    // #262,145 wedged daemon and client forever with no
                    // timeout in scope.
                    // w5-1: was an unconditional per-file eprintln — stderr
                    // spam proportional to file count. Debug-level now;
                    // visible with BLIT_LOG=debug.
                    log::debug!("push server queued {}", sanitized);
                    let flushed = need_list_sender.push(sanitized).await?;
                    files_to_upload.push(file);
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

                            let stream_target = desired_streams(&files_to_upload);
                            let transfer_task = tokio::spawn(accept_data_connection_stream(
                                listener,
                                token.clone(),
                                module_for_transfer,
                                stream_target,
                            ));

                            send_control_message(
                                &tx,
                                server_push_response::Payload::Negotiation(
                                    DataTransferNegotiation {
                                        tcp_port: port as u32,
                                        one_time_token: token_string,
                                        tcp_fallback: false,
                                        stream_count: stream_target,
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
            let stream_target = desired_streams(&files_to_upload);
            let transfer_task = tokio::spawn(accept_data_connection_stream(
                listener,
                token.clone(),
                module_for_transfer,
                stream_target,
            ));
            send_control_message(
                &tx,
                server_push_response::Payload::Negotiation(DataTransferNegotiation {
                    tcp_port: port as u32,
                    one_time_token: token_string,
                    tcp_fallback: false,
                    stream_count: stream_target,
                }),
            )
            .await?;
            data_plane_handle = Some(transfer_task);
        }

        if let Some(handle) = data_plane_handle.take() {
            handle
                .await
                .map_err(|_| Status::internal("data plane task cancelled"))??
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

fn desired_streams(files: &[FileHeader]) -> u32 {
    if files.is_empty() {
        return 1;
    }
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    let file_count = files.len();
    if total_bytes >= 32 * 1024 * 1024 * 1024 || file_count >= 200_000 {
        16
    } else if total_bytes >= 8 * 1024 * 1024 * 1024 || file_count >= 80_000 {
        12
    } else if total_bytes >= 2 * 1024 * 1024 * 1024 || file_count >= 50_000 {
        10
    } else if total_bytes >= 512 * 1024 * 1024 || file_count >= 10_000 {
        8
    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
        4
    } else if total_bytes >= 32 * 1024 * 1024 || file_count >= 256 {
        2
    } else {
        1
    }
}
