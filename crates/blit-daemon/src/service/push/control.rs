use super::super::admin::purge_extraneous_entries;
use super::super::util::{metadata_mtime_seconds, resolve_module, resolve_relative_path};
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
use tokio::sync::{mpsc, Mutex};
use tonic::{Status, Streaming};

const FILE_LIST_BATCH_MAX_ENTRIES: usize = 16 * 1024;
const FILE_LIST_BATCH_MAX_BYTES: usize = 512 * 1024;
const FILE_LIST_BATCH_MAX_DELAY: Duration = Duration::from_millis(25);
const FILE_LIST_EARLY_FLUSH_ENTRIES: usize = 128;
const FILE_LIST_EARLY_FLUSH_BYTES: usize = 64 * 1024;
const FILE_LIST_EARLY_FLUSH_DELAY: Duration = Duration::from_millis(5);
const FILE_UPLOAD_CHANNEL_CAPACITY: usize = FILE_LIST_BATCH_MAX_ENTRIES * 16;

pub(crate) async fn handle_push_stream(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    mut stream: Streaming<ClientPushRequest>,
    tx: PushSender,
    force_grpc_data: bool,
) -> Result<(), Status> {
    let mut module: Option<ModuleConfig> = None;
    let mut files_to_upload: Vec<FileHeader> = Vec::new();
    let mut manifest_complete = false;
    let mut mirror_mode = false;
    let mut expected_rel_files: Vec<PathBuf> = Vec::new();
    let mut force_grpc_client = false;
    let mut need_list_sender = FileListBatcher::new(tx.clone());
    let (upload_tx, upload_rx) = mpsc::channel::<FileHeader>(FILE_UPLOAD_CHANNEL_CAPACITY);
    let mut upload_rx_opt = Some(upload_rx);
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
                let dest_path = header.destination_path.trim();
                if !dest_path.is_empty() {
                    let rel = resolve_relative_path(dest_path)?;
                    config.path = config.path.join(rel);
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
                let rel = resolve_relative_path(&file.relative_path)?;
                expected_rel_files.push(rel.clone());
                let sanitized = rel.to_string_lossy().to_string();

                if file_requires_upload(module_ref, &rel, &file)? {
                    file.relative_path = sanitized.clone();
                    upload_tx.send(file.clone()).await.map_err(|_| {
                        eprintln!(
                            "upload_tx send failed for {} (mirror_mode={})",
                            sanitized, mirror_mode
                        );
                        Status::internal("failed to enqueue upload header")
                    })?;
                    let flushed = need_list_sender.push(sanitized).await?;
                    files_to_upload.push(file);
                    if flushed && data_plane_handle.is_none() {
                        if force_grpc_effective {
                            fallback_used = true;
                            send_control_message(
                                &tx,
                                server_push_response::Payload::Negotiation(
                                    DataTransferNegotiation {
                                        tcp_port: 0,
                                        one_time_token: String::new(),
                                        tcp_fallback: true,
                                        stream_count: 0,
                                    },
                                ),
                            )
                            .await?;
                        } else {
                            let listener = match bind_data_plane_listener().await {
                                Ok(l) => l,
                                Err(_) => {
                                    fallback_used = true;
                                    force_grpc_effective = true;
                                    send_control_message(
                                        &tx,
                                        server_push_response::Payload::Negotiation(
                                            DataTransferNegotiation {
                                                tcp_port: 0,
                                                one_time_token: String::new(),
                                                tcp_fallback: true,
                                                stream_count: 0,
                                            },
                                        ),
                                    )
                                    .await?;
                                    continue;
                                }
                            };

                            let port = listener
                                .local_addr()
                                .map_err(|err| {
                                    Status::internal(format!("querying listener addr: {}", err))
                                })?
                                .port();

                            let token = generate_token();
                            let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);

                            let module_for_transfer = module_ref.clone();

                            let upload_rx =
                                upload_rx_opt.take().expect("upload receiver already taken");

                            let stream_target = desired_streams(&files_to_upload);
                            let transfer_task = tokio::spawn(accept_data_connection_stream(
                                listener,
                                token.clone(),
                                module_for_transfer,
                                upload_rx,
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
            Some(client_push_request::Payload::ManifestComplete(_)) => {
                manifest_complete = true;
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

    drop(upload_tx);
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
            let token = generate_token();
            let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
            let upload_rx = upload_rx_opt.take().expect("upload receiver already taken");
            let module_for_transfer = module.clone();
            let stream_target = desired_streams(&files_to_upload);
            let transfer_task = tokio::spawn(accept_data_connection_stream(
                listener,
                token.clone(),
                module_for_transfer,
                upload_rx,
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
        let purge_stats = purge_extraneous_entries(module.path.clone(), expected_rel_files).await?;
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
        let entry_bytes = path.as_bytes().len();
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
        } else if !self.sent_any {
            send_control_message(
                &self.tx,
                server_push_response::Payload::FilesToUpload(FileList {
                    relative_paths: Vec::new(),
                }),
            )
            .await?;
        }
        Ok(())
    }

    fn should_flush(&self) -> bool {
        if self.batch.is_empty() {
            return false;
        }

        if !self.sent_any {
            if self.batch.len() >= FILE_LIST_EARLY_FLUSH_ENTRIES
                || self.batch_bytes >= FILE_LIST_EARLY_FLUSH_BYTES
                || self.last_flush.elapsed() >= FILE_LIST_EARLY_FLUSH_DELAY
            {
                return true;
            }
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
    let full_path = module.path.join(rel);
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
