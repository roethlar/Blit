use super::admin::{
    delete_rel_paths, filesystem_stats_for_path, list_completions, sanitize_request_paths,
    split_completion_prefix, stream_disk_usage, stream_find_entries,
};
use super::pull::stream_pull;
use super::pull_sync::handle_pull_sync_stream;
use super::push::handle_push_stream;
use super::util::{
    metadata_mtime_seconds, resolve_contained_path, resolve_module, resolve_relative_path,
};
use super::{DiskUsageSender, FindSender};
use crate::metrics::TransferMetrics;
use crate::runtime::{ModuleConfig, RootExport};
use blit_core::generated::blit_server::Blit;
pub use blit_core::generated::blit_server::BlitServer;
use blit_core::generated::{
    ClientPullMessage, ClientPushRequest, CompletionRequest, CompletionResponse,
    DelegatedPullProgress, DelegatedPullRequest, DiskUsageEntry, DiskUsageRequest, FileInfo,
    FilesystemStatsRequest, FilesystemStatsResponse, FindEntry, FindRequest, ListModulesRequest,
    ListModulesResponse, ListRequest, ListResponse, ModuleInfo, PullChunk, PullRequest,
    PurgeRequest, PurgeResponse, ServerPullMessage, ServerPushResponse,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

pub struct BlitService {
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    force_grpc_data: bool,
    server_checksums_enabled: bool,
    metrics: Arc<TransferMetrics>,
    /// Delegation gate config. The handler reads it on every
    /// `DelegatedPull` request; default-disabled means no caller can
    /// make this daemon initiate outbound connects until the operator
    /// flips `[delegation] allow_delegated_pull = true`.
    pub(crate) delegation: Arc<crate::delegation_gate::DelegationConfig>,
}

impl BlitService {
    pub(crate) fn from_runtime(
        modules: HashMap<String, ModuleConfig>,
        default_root: Option<RootExport>,
        force_grpc_data: bool,
        server_checksums_enabled: bool,
        metrics: Arc<TransferMetrics>,
        delegation: crate::delegation_gate::DelegationConfig,
    ) -> Self {
        Self {
            modules: Arc::new(Mutex::new(modules)),
            default_root,
            force_grpc_data,
            server_checksums_enabled,
            metrics,
            delegation: Arc::new(delegation),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn with_modules(
        modules: HashMap<String, ModuleConfig>,
        force_grpc_data: bool,
    ) -> Self {
        Self::from_runtime(
            modules,
            None,
            force_grpc_data,
            true,
            TransferMetrics::disabled(),
            crate::delegation_gate::DelegationConfig::default(),
        )
    }

    /// Inner purge body. Extracted from the trait method so the
    /// `--metrics` completion log can wrap a single call site and
    /// branch on Result without duplicating the response shape.
    /// §3.1 followup.
    async fn purge_inner(&self, req: PurgeRequest) -> Result<Response<PurgeResponse>, Status> {
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        if module.read_only {
            return Err(Status::permission_denied(format!(
                "module '{}' is read-only",
                module.name
            )));
        }
        let sanitized = sanitize_request_paths(req.paths_to_delete)?;
        if sanitized.is_empty() {
            return Ok(Response::new(PurgeResponse { files_deleted: 0 }));
        }
        let stats = delete_rel_paths(
            module.path.clone(),
            module.canonical_root.clone(),
            sanitized,
        )
        .await?;
        Ok(Response::new(PurgeResponse {
            files_deleted: stats.total(),
        }))
    }
}

#[tonic::async_trait]
impl Blit for BlitService {
    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullStream = ReceiverStream<Result<PullChunk, Status>>;
    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
    type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;

    async fn push(
        &self,
        request: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        let modules = Arc::clone(&self.modules);
        let (tx, rx) = mpsc::channel(32);
        let stream = request.into_inner();
        let force_grpc_data = self.force_grpc_data;
        let default_root = self.default_root.clone();
        // Counter increments at the dispatch boundary — single chokepoint
        // per RPC, no reach-in to the transfer pipeline. No-op when
        // metrics are disabled (default). The active-transfers gauge
        // uses an RAII guard so panic/cancellation can't leak it
        // (F5 of docs/reviews/codebase_review_2026-05-01.md).
        let metrics = Arc::clone(&self.metrics);
        metrics.inc_push();
        let guard = Arc::clone(&metrics).enter_transfer();
        // §3.1 / D5: capture start time so `--metrics` can emit a
        // per-RPC duration line at completion.
        let started = std::time::Instant::now();

        tokio::spawn(async move {
            // `guard` is moved into the task; its Drop fires no
            // matter how the task ends.
            let guard = guard;
            let result =
                handle_push_stream(modules, default_root, stream, tx.clone(), force_grpc_data)
                    .await;
            let ok = result.is_ok();
            if let Err(status) = result {
                metrics.inc_error();
                let _ = tx.send(Err(status)).await;
            }
            // §3.1 followup: drop the active-transfer guard BEFORE the
            // completion log so `active=N` reflects state AFTER the
            // just-finished RPC is removed from the gauge. Pre-fix
            // a single-transfer log showed `active=1`, which is
            // misleading for an end-of-RPC summary.
            drop(guard);
            metrics.log_completion("push", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn pull(
        &self,
        request: Request<PullRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;

        let force_grpc = req.force_grpc || self.force_grpc_data;
        let metadata_only = req.metadata_only;
        let (tx, rx) = mpsc::channel(32);
        let metrics = Arc::clone(&self.metrics);
        metrics.inc_pull();
        let guard = Arc::clone(&metrics).enter_transfer();
        let started = std::time::Instant::now();

        tokio::spawn(async move {
            let guard = guard;
            let result = stream_pull(module, req.path, force_grpc, metadata_only, tx.clone()).await;
            let ok = result.is_ok();
            if let Err(status) = result {
                metrics.inc_error();
                let _ = tx.send(Err(status)).await;
            }
            drop(guard);
            metrics.log_completion("pull", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn pull_sync(
        &self,
        request: Request<Streaming<ClientPullMessage>>,
    ) -> Result<Response<Self::PullSyncStream>, Status> {
        let modules = Arc::clone(&self.modules);
        let (tx, rx) = mpsc::channel(32);
        let stream = request.into_inner();
        let force_grpc_data = self.force_grpc_data;
        let default_root = self.default_root.clone();
        let server_checksums_enabled = self.server_checksums_enabled;
        let metrics = Arc::clone(&self.metrics);
        metrics.inc_pull();
        let guard = Arc::clone(&metrics).enter_transfer();
        let started = std::time::Instant::now();

        tokio::spawn(async move {
            let guard = guard;
            let result = handle_pull_sync_stream(
                modules,
                default_root,
                stream,
                tx.clone(),
                force_grpc_data,
                server_checksums_enabled,
            )
            .await;
            let ok = result.is_ok();
            if let Err(status) = result {
                metrics.inc_error();
                let _ = tx.send(Err(status)).await;
            }
            drop(guard);
            metrics.log_completion("pull_sync", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn delegated_pull(
        &self,
        request: Request<DelegatedPullRequest>,
    ) -> Result<Response<Self::DelegatedPullStream>, Status> {
        let req = request.into_inner();
        let modules = Arc::clone(&self.modules);
        let default_root = self.default_root.clone();
        let delegation = Arc::clone(&self.delegation);
        let metrics = Arc::clone(&self.metrics);
        let metrics_for_log = Arc::clone(&self.metrics);
        let (tx, rx) = mpsc::channel(32);
        // §3.1 followup: cover delegated_pull in the per-RPC summary
        // log too. The handler increments `pull_ops` + the active
        // gauge inside `run_delegated_pull` (delegated_pull.rs:227),
        // so without this site `delegated_pull` would count toward
        // `pull_ops` but never emit its own completion line.
        let started = std::time::Instant::now();

        // R30-F2: race the handler against tx.closed() so a CLI
        // disconnect drops the inner pull future. tonic's response
        // stream drops the mpsc Receiver when the client cancels;
        // that closes the Sender, and tx.closed() resolves. The
        // handler's pull_sync_with_spec future is then dropped,
        // which propagates cancellation through the existing pull
        // cancellation path (data plane connection drop, manifest
        // task cleanup). Without this race the spawned task would
        // continue to write — and post-R30-F1 to delete — on dst
        // after the operator has Ctrl-C'd.
        //
        // Cloning tx for the handler so the original tx survives
        // long enough for tx.closed() to be the racing future.
        let handler_tx = tx.clone();
        tokio::spawn(async move {
            // None        = cancelled by client hangup (handler future dropped)
            // Some(true)  = handler returned success
            // Some(false) = handler returned failure (phased error already
            //               sent to client over handler_tx)
            let outcome: Option<bool> = tokio::select! {
                biased;
                _ = tx.closed() => {
                    // Caller hung up. Dropping handler_tx (which
                    // happens at end of the select branch) and
                    // dropping the outer task drops the handler
                    // future implicitly via select cancellation.
                    None
                }
                handler_ok = super::delegated_pull::handle_delegated_pull(
                    req,
                    modules,
                    default_root,
                    delegation,
                    metrics,
                    handler_tx,
                ) => {
                    Some(handler_ok)
                }
            };
            // The handler's RAII guard releases the active gauge as
            // its scope ends with the spawn task above, so by the
            // time we log here `active` already excludes this RPC.
            //
            // R-followup: distinguish handler-failure from client-
            // cancellation. Both log `ok=false` (the transfer didn't
            // complete), but only the former is a daemon error and
            // increments `errors`. Pre-fix every non-cancelled run
            // logged `ok=true` regardless of whether
            // `run_delegated_pull` returned Err — a real failure
            // could log `delegated_pull ok … errors=N` with N
            // unchanged.
            let ok = matches!(outcome, Some(true));
            if matches!(outcome, Some(false)) {
                metrics_for_log.inc_error();
            }
            metrics_for_log.log_completion("delegated_pull", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;

        let requested = if req.path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(&req.path)?
        };

        let target = resolve_contained_path(&module, &requested)?;
        let response_entries =
            tokio::task::spawn_blocking(move || -> Result<Vec<FileInfo>, Status> {
                let metadata = fs::metadata(&target).map_err(|err| {
                    Status::internal(format!("stat {}: {}", target.display(), err))
                })?;

                if metadata.is_file() {
                    let name = requested
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| ".".to_string());
                    let info = FileInfo {
                        name,
                        is_dir: false,
                        size: metadata.len(),
                        mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
                    };
                    Ok(vec![info])
                } else if metadata.is_dir() {
                    let mut infos = Vec::new();
                    let entries = fs::read_dir(&target).map_err(|err| {
                        Status::internal(format!("read_dir {}: {}", target.display(), err))
                    })?;
                    for entry in entries {
                        let entry = entry.map_err(|err| {
                            Status::internal(format!(
                                "read_dir entry {}: {}",
                                target.display(),
                                err
                            ))
                        })?;
                        let path = entry.path();
                        let meta = entry.metadata().map_err(|err| {
                            Status::internal(format!("metadata {}: {}", path.display(), err))
                        })?;
                        let name = entry.file_name().to_string_lossy().into_owned();
                        infos.push(FileInfo {
                            name,
                            is_dir: meta.is_dir(),
                            size: meta.len(),
                            mtime_seconds: metadata_mtime_seconds(&meta).unwrap_or(0),
                        });
                    }
                    infos.sort_by(|a, b| a.name.cmp(&b.name));
                    Ok(infos)
                } else {
                    Err(Status::invalid_argument(format!(
                        "unsupported path type for list: {}",
                        target.display()
                    )))
                }
            })
            .await
            .map_err(|err| Status::internal(format!("list task failed: {}", err)))??;

        Ok(Response::new(ListResponse {
            entries: response_entries,
        }))
    }

    async fn purge(
        &self,
        request: Request<PurgeRequest>,
    ) -> Result<Response<PurgeResponse>, Status> {
        let req = request.into_inner();
        // F5: counters mark dispatch attempts (matching push/pull
        // semantics). Previously inc_purge fired only after a
        // successful delete, contradicting the metrics-module doc.
        self.metrics.inc_purge();
        // §3.1 followup: purge needs its own completion line.
        // Pre-fix `purge_ops` was visible only on later push/pull
        // logs, never on the purge RPC itself.
        let started = std::time::Instant::now();
        let result = self.purge_inner(req).await;
        let ok = result.is_ok();
        if result.is_err() {
            self.metrics.inc_error();
        }
        self.metrics.log_completion("purge", started.elapsed(), ok);
        result
    }

    async fn complete_path(
        &self,
        request: Request<CompletionRequest>,
    ) -> Result<Response<CompletionResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        if !req.include_files && !req.include_directories {
            return Err(Status::invalid_argument(
                "at least one of include_files or include_directories must be true",
            ));
        }

        let (dir_rel, display_prefix, leaf_prefix) =
            split_completion_prefix(req.path_prefix.as_str())?;
        let search_root = resolve_contained_path(&module, &dir_rel)?;
        let include_files = req.include_files;
        let include_dirs = req.include_directories;

        let entries = tokio::task::spawn_blocking(move || {
            list_completions(
                &search_root,
                &display_prefix,
                &leaf_prefix,
                include_files,
                include_dirs,
            )
        })
        .await
        .map_err(|err| Status::internal(format!("completion task failed: {}", err)))??;

        Ok(Response::new(CompletionResponse {
            completions: entries,
        }))
    }

    async fn list_modules(
        &self,
        _request: Request<ListModulesRequest>,
    ) -> Result<Response<ListModulesResponse>, Status> {
        let guard = self.modules.lock().await;
        let mut modules: Vec<ModuleInfo> = guard
            .values()
            .map(|module| ModuleInfo {
                name: module.name.clone(),
                path: module.path.to_string_lossy().into_owned(),
                read_only: module.read_only,
            })
            .collect();
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Response::new(ListModulesResponse { modules }))
    }

    async fn disk_usage(
        &self,
        request: Request<DiskUsageRequest>,
    ) -> Result<Response<Self::DiskUsageStream>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let start_rel = if req.start_path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(req.start_path.trim())?
        };

        let max_depth = if req.max_depth == 0 {
            None
        } else {
            Some(req.max_depth as usize)
        };

        let (tx, rx): (
            DiskUsageSender,
            mpsc::Receiver<Result<DiskUsageEntry, Status>>,
        ) = mpsc::channel(32);

        let module_root = module.path.clone();
        tokio::spawn(async move {
            let err_sender = tx.clone();
            let result = tokio::task::spawn_blocking(move || {
                stream_disk_usage(module_root, start_rel, max_depth, &tx)
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(status)) => {
                    let _ = err_sender.send(Err(status)).await;
                }
                Err(join_err) => {
                    let _ = err_sender
                        .send(Err(Status::internal(format!(
                            "disk usage worker failed: {}",
                            join_err
                        ))))
                        .await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn find(
        &self,
        request: Request<FindRequest>,
    ) -> Result<Response<Self::FindStream>, Status> {
        let req = request.into_inner();
        if !req.include_files && !req.include_directories {
            return Err(Status::invalid_argument(
                "at least one of include_files or include_directories must be true",
            ));
        }
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let (tx, rx): (FindSender, mpsc::Receiver<Result<FindEntry, Status>>) = mpsc::channel(32);

        let start_rel = if req.start_path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(&req.start_path)?
        };

        let module_root = module.path.clone();
        let pattern = req.pattern.clone();
        let case_sensitive = req.case_sensitive;
        let include_files = req.include_files;
        let include_dirs = req.include_directories;
        let max_results = if req.max_results == 0 {
            None
        } else {
            Some(req.max_results as usize)
        };

        tokio::spawn(async move {
            let err_sender = tx.clone();
            let result = tokio::task::spawn_blocking(move || {
                stream_find_entries(
                    module_root,
                    start_rel,
                    pattern,
                    case_sensitive,
                    include_files,
                    include_dirs,
                    max_results,
                    &tx,
                )
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(status)) => {
                    let _ = err_sender.send(Err(status)).await;
                }
                Err(join_err) => {
                    let _ = err_sender
                        .send(Err(Status::internal(format!(
                            "find worker failed: {}",
                            join_err
                        ))))
                        .await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn filesystem_stats(
        &self,
        request: Request<FilesystemStatsRequest>,
    ) -> Result<Response<FilesystemStatsResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let stats = filesystem_stats_for_path(&module.path)?;

        Ok(Response::new(stats))
    }
}
