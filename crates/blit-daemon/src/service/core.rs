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
use crate::active_jobs::{ActiveJobKind, ActiveJobs, CancelOutcome};
use crate::metrics::TransferMetrics;
use crate::runtime::{ModuleConfig, RootExport};
use blit_core::generated::blit_server::Blit;
pub use blit_core::generated::blit_server::BlitServer;
use blit_core::generated::{
    ActiveTransfer, CancelJobRequest, CancelJobResponse, ClientPullMessage, ClientPushRequest,
    CompletionRequest, CompletionResponse, Counters, DaemonState, DelegatedPullProgress,
    DelegatedPullRequest, DiskUsageEntry, DiskUsageRequest, FileInfo, FilesystemStatsRequest,
    FilesystemStatsResponse, FindEntry, FindRequest, GetStateRequest, ListModulesRequest,
    ListModulesResponse, ListRequest, ListResponse, ModuleInfo, PullChunk, PullRequest,
    PurgeRequest, PurgeResponse, ServerPullMessage, ServerPushResponse, TransferRecord,
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
    /// Always-on registry of in-flight transfers. Populated
    /// from the dispatch boundary in this file; read by
    /// `GetState.active[]` once that RPC lands (milestone B
    /// sub-slice). See `crate::active_jobs`.
    pub(crate) active_jobs: ActiveJobs,
    /// Wall-clock at construction. `GetState.uptime_seconds`
    /// reports `Instant::now().duration_since(started_at)`.
    /// Captured once so a clock jump between construction and
    /// the GetState call doesn't show up as negative uptime.
    started_at: std::time::Instant,
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
            active_jobs: ActiveJobs::new(),
            started_at: std::time::Instant::now(),
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
        let peer = peer_addr_string(&request);
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
        // ActiveJobs row registered with empty module/path —
        // those arrive in the first stream frame; the handler
        // calls `job.set_endpoint(...)` once the header is
        // parsed (b-2-set-endpoint).
        let job =
            self.active_jobs
                .register(ActiveJobKind::Push, peer, String::new(), String::new());
        // §3.1 / D5: capture start time so `--metrics` can emit a
        // per-RPC duration line at completion.
        let started = std::time::Instant::now();

        tokio::spawn(async move {
            // `guard` and `job` are moved into the task; their
            // Drop fires no matter how the task ends.
            let guard = guard;
            let job = job;
            let result = handle_push_stream(
                modules,
                default_root,
                stream,
                tx.clone(),
                force_grpc_data,
                &job,
            )
            .await;
            let (ok, err_msg) = outcome_from_status(&result);
            if let Err(status) = result {
                metrics.inc_error();
                let _ = tx.send(Err(status)).await;
            }
            // Record the outcome before dropping the
            // ActiveJob guard — Drop builds the recent-runs
            // TransferRecord and reads this cell. If we
            // dropped the guard first the record would say
            // "cancelled before outcome recorded."
            job.record_outcome(ok, err_msg);
            drop(job);
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
        let peer = peer_addr_string(&request);
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;

        let force_grpc = req.force_grpc || self.force_grpc_data;
        let metadata_only = req.metadata_only;
        let (tx, rx) = mpsc::channel(32);
        let metrics = Arc::clone(&self.metrics);
        metrics.inc_pull();
        let guard = Arc::clone(&metrics).enter_transfer();
        // ActiveJobs row registered alongside the metrics gauge:
        // both are RAII-scoped to the spawned task so they
        // drain together on every termination path (success,
        // error, panic, client cancellation).
        let job = self.active_jobs.register(
            ActiveJobKind::Pull,
            peer,
            req.module.clone(),
            req.path.clone(),
        );
        let started = std::time::Instant::now();

        tokio::spawn(async move {
            let guard = guard;
            let job = job;
            let result = stream_pull(module, req.path, force_grpc, metadata_only, tx.clone()).await;
            let (ok, err_msg) = outcome_from_status(&result);
            if let Err(status) = result {
                metrics.inc_error();
                let _ = tx.send(Err(status)).await;
            }
            job.record_outcome(ok, err_msg);
            drop(guard);
            drop(job);
            metrics.log_completion("pull", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn pull_sync(
        &self,
        request: Request<Streaming<ClientPullMessage>>,
    ) -> Result<Response<Self::PullSyncStream>, Status> {
        let peer = peer_addr_string(&request);
        let modules = Arc::clone(&self.modules);
        let (tx, rx) = mpsc::channel(32);
        let stream = request.into_inner();
        let force_grpc_data = self.force_grpc_data;
        let default_root = self.default_root.clone();
        let server_checksums_enabled = self.server_checksums_enabled;
        let metrics = Arc::clone(&self.metrics);
        metrics.inc_pull();
        let guard = Arc::clone(&metrics).enter_transfer();
        // Same shape as `push` above: module + path arrive in
        // the first stream frame; handler calls
        // `job.set_endpoint(...)` after parsing the spec.
        let job =
            self.active_jobs
                .register(ActiveJobKind::PullSync, peer, String::new(), String::new());
        let started = std::time::Instant::now();

        tokio::spawn(async move {
            let guard = guard;
            let job = job;
            let result = handle_pull_sync_stream(
                modules,
                default_root,
                stream,
                tx.clone(),
                force_grpc_data,
                server_checksums_enabled,
                &job,
            )
            .await;
            let (ok, err_msg) = outcome_from_status(&result);
            if let Err(status) = result {
                metrics.inc_error();
                let _ = tx.send(Err(status)).await;
            }
            job.record_outcome(ok, err_msg);
            drop(guard);
            drop(job);
            metrics.log_completion("pull_sync", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn delegated_pull(
        &self,
        request: Request<DelegatedPullRequest>,
    ) -> Result<Response<Self::DelegatedPullStream>, Status> {
        let peer = peer_addr_string(&request);
        let req = request.into_inner();
        // ActiveJobs row mirrors the metrics gauge — both are
        // owned by the spawned task so the row drains on every
        // termination path (success, handler failure, client
        // hangup). Module + dst path come straight off the
        // request; they're synchronously available here unlike
        // the streaming RPCs (push, pull_sync), which register
        // with empty endpoint strings and have their handlers
        // fill them in via `ActiveJobGuard::set_endpoint` once
        // the first stream frame parses.
        let job = self.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            peer,
            req.dst_module.clone(),
            req.dst_destination_path.clone(),
        );
        // Captured before `req` moves into the handler call.
        // Drives the conditional select arm below: when
        // `detach=true` the `tx.closed()` race is disabled, so
        // a CLI disconnect no longer drops the transfer
        // future. The transfer then runs to completion,
        // failure, or `CancelJob(transfer_id)` regardless of
        // client connection state.
        let detach = req.detach;
        let transfer_id_for_started = job.transfer_id().to_string();
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
        // Clone the cancellation token off the guard before
        // moving the guard into the spawn task. The future's
        // select needs a `.cancelled()` future; cloning the
        // token (cheap, internal Arc) lets us hold the
        // cancelled-future on its own line.
        let cancel_token = job.cancellation_token().clone();
        tokio::spawn(async move {
            // `job` moves into the spawned task alongside the
            // metrics guard; its Drop runs on every exit path
            // from the select below.
            let job = job;
            // Three-way race (the tx.closed arm is gated by
            // `!detach` — see m-jobs-3):
            //   tx.closed()             → client hung up (R30-F2);
            //                              disabled when detach=true
            //   cancel_token.cancelled() → `CancelJob` RPC fired the
            //                              token from another task
            //                              (m-jobs-1)
            //   handle_delegated_pull → handler ran to completion or
            //                              failure
            //
            // Outcome encoding:
            //   None         → cancelled (client OR CancelJob)
            //   Some(true)   → handler returned success
            //   Some(false)  → handler returned failure (phased
            //                  error already sent to client over
            //                  handler_tx)
            let outcome: Option<bool> = tokio::select! {
                biased;
                _ = tx.closed(), if !detach => {
                    // Caller hung up and we're NOT detached.
                    // Dropping handler_tx (which happens at end
                    // of the select branch) and dropping the
                    // outer task drops the handler future
                    // implicitly via select cancellation.
                    None
                }
                _ = cancel_token.cancelled() => {
                    // `CancelJob` fired the token. Same
                    // teardown path as a client hangup — the
                    // handler future is dropped and the data
                    // plane cleans up via the existing
                    // cancellation chain.
                    None
                }
                handler_ok = super::delegated_pull::handle_delegated_pull(
                    req,
                    modules,
                    default_root,
                    delegation,
                    metrics,
                    handler_tx,
                    transfer_id_for_started,
                ) => {
                    Some(handler_ok)
                }
            };
            // Map the select outcome onto the ActiveJobs ring
            // shape:
            //   Some(true)  → ok, no error
            //   Some(false) → handler-failure; the handler
            //                  already sent the phased error to
            //                  the client and surfaced it via
            //                  `metrics.inc_error` below. We
            //                  don't have the message string at
            //                  this level — the C milestone
            //                  routes structured errors. Use a
            //                  short marker.
            //   None        → client hangup or CancelJob.
            //                  Distinguish by inspecting the
            //                  cancellation token: if it was
            //                  cancelled, the cause was
            //                  CancelJob; otherwise it was the
            //                  client.
            let (job_ok, job_err) = match outcome {
                Some(true) => (true, None),
                Some(false) => (false, Some("delegated_pull handler failed".to_string())),
                None if cancel_token.is_cancelled() => {
                    (false, Some("cancelled via CancelJob".to_string()))
                }
                None => (false, Some("client cancelled".to_string())),
            };
            job.record_outcome(job_ok, job_err);
            drop(job);
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

    async fn get_state(
        &self,
        request: Request<GetStateRequest>,
    ) -> Result<Response<DaemonState>, Status> {
        use std::sync::atomic::Ordering;
        let req = request.into_inner();
        // `recent_limit` semantics (`proto/blit.proto`): 0 means
        // "use the daemon's default" — return the full ring as
        // sized by `with_recent_limit`. A non-zero value caps
        // the response to the most-recent N entries; the daemon
        // doesn't grow the ring for one request, just truncates
        // for this response.
        let modules = {
            let guard = self.modules.lock().await;
            let mut ms: Vec<ModuleInfo> = guard
                .values()
                .map(|module| ModuleInfo {
                    name: module.name.clone(),
                    path: module.path.to_string_lossy().into_owned(),
                    read_only: module.read_only,
                })
                .collect();
            ms.sort_by(|a, b| a.name.cmp(&b.name));
            ms
        };

        let active: Vec<ActiveTransfer> = self
            .active_jobs
            .snapshot()
            .into_iter()
            .map(|j| ActiveTransfer {
                transfer_id: j.transfer_id,
                kind: j.kind.to_wire() as i32,
                peer: j.peer,
                module: j.module,
                path: j.path,
                start_unix_ms: j.start_unix_ms,
                // `bytes_completed` reads from the per-row atomic
                // (c-1a-byte-counter-api). Stays at zero until
                // c-1b wires the data-plane receive loop to call
                // `ByteProgressSink::report`; the wire shape is
                // already correct.
                bytes_completed: j.bytes_completed,
                // `bytes_total` lands in a subsequent C slice
                // from the manifest stage.
                bytes_total: 0,
            })
            .collect();

        // `recent()` returns oldest-first; truncate from the
        // front when capping to the most-recent N. `as usize`
        // is well-defined: even on 32-bit targets the proto u32
        // can't exceed `usize::MAX`.
        let mut recent_rows = self.active_jobs.recent();
        if req.recent_limit > 0 {
            let limit = req.recent_limit as usize;
            if recent_rows.len() > limit {
                let drop_n = recent_rows.len() - limit;
                recent_rows.drain(0..drop_n);
            }
        }
        let recent: Vec<TransferRecord> = recent_rows
            .into_iter()
            .map(|r| TransferRecord {
                transfer_id: r.transfer_id,
                kind: r.kind.to_wire() as i32,
                peer: r.peer,
                module: r.module,
                path: r.path,
                start_unix_ms: r.start_unix_ms,
                duration_ms: r.duration_ms,
                // Final byte count from the per-row atomic
                // (c-1a-byte-counter-api). Zero until c-1b
                // wires the data-plane receive loop.
                bytes: r.bytes,
                // `files` lands in a subsequent C slice.
                files: 0,
                ok: r.ok,
                error_message: r.error_message,
            })
            .collect();

        let counters = Counters {
            push_operations_total: self.metrics.push_operations.load(Ordering::Relaxed),
            pull_operations_total: self.metrics.pull_operations.load(Ordering::Relaxed),
            purge_operations_total: self.metrics.purge_operations.load(Ordering::Relaxed),
            active_transfers: self.metrics.active_transfers.load(Ordering::Relaxed),
            transfer_errors_total: self.metrics.transfer_errors.load(Ordering::Relaxed),
        };

        let uptime_seconds = self.started_at.elapsed().as_secs();

        Ok(Response::new(DaemonState {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds,
            modules,
            active,
            recent,
            counters: Some(counters),
            delegation_enabled: self.delegation.allow_delegated_pull,
        }))
    }

    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        let req = request.into_inner();
        if req.transfer_id.trim().is_empty() {
            return Err(Status::invalid_argument(
                "CancelJobRequest.transfer_id must not be empty",
            ));
        }
        // `ActiveJobs::cancel` is synchronous and short — the
        // critical section is one `HashMap::get` + (when the
        // kind supports it) one `CancellationToken::cancel()`.
        // No async work to do.
        match self.active_jobs.cancel(&req.transfer_id) {
            CancelOutcome::Cancelled => Ok(Response::new(CancelJobResponse {
                transfer_id: req.transfer_id,
            })),
            CancelOutcome::Unsupported => Err(Status::failed_precondition(format!(
                "transfer '{}' is not cancellable from another client (CLI is in the byte path; \
                 cancel from the originating client instead)",
                req.transfer_id
            ))),
            CancelOutcome::NotFound => Err(Status::not_found(format!(
                "no active transfer matches transfer_id '{}'",
                req.transfer_id
            ))),
        }
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

/// Format the remote peer of a tonic request as `<ip>:<port>`,
/// or `"unknown"` when the transport didn't surface one (eg.
/// in-process tests that bypass the network).
fn peer_addr_string<T>(request: &Request<T>) -> String {
    request
        .remote_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Translate a handler's `Result<_, Status>` into the
/// `(ok, error_message)` pair the ActiveJobs guard expects.
/// Used by `push`, `pull`, and `pull_sync` dispatchers.
/// `delegated_pull` has its own shape (handler returns `bool`
/// inside a select) and inlines the equivalent mapping there.
fn outcome_from_status<T>(result: &Result<T, Status>) -> (bool, Option<String>) {
    match result {
        Ok(_) => (true, None),
        Err(status) => (false, Some(status.message().to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_jobs::ActiveJobKind;
    use blit_core::generated::TransferKind as WireKind;

    fn empty_service() -> BlitService {
        BlitService::with_modules(HashMap::new(), false)
    }

    #[tokio::test]
    async fn get_state_empty_daemon_returns_zero_active_and_recent() {
        let svc = empty_service();
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.version, env!("CARGO_PKG_VERSION"));
        assert!(state.active.is_empty());
        assert!(state.recent.is_empty());
        assert!(state.modules.is_empty());
        // Counters present but zero because `with_modules` builds
        // a metrics-disabled service.
        let counters = state.counters.expect("counters present");
        assert_eq!(counters.push_operations_total, 0);
        assert_eq!(counters.pull_operations_total, 0);
        assert_eq!(counters.transfer_errors_total, 0);
    }

    #[tokio::test]
    async fn get_state_surfaces_live_active_row_and_recent_row() {
        let svc = empty_service();
        // Live row.
        let guard = svc.active_jobs.register(
            ActiveJobKind::Pull,
            "10.0.0.5:443".to_string(),
            "mod-a".to_string(),
            "sub/dir".to_string(),
        );

        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.active.len(), 1);
        let row = &state.active[0];
        assert_eq!(row.kind, WireKind::Pull as i32);
        assert_eq!(row.peer, "10.0.0.5:443");
        assert_eq!(row.module, "mod-a");
        assert_eq!(row.path, "sub/dir");
        // Byte-level fields are zero with no reports against
        // the per-row counter — c-1a wired the atomic but no
        // call site reports against it yet (c-1b lands the
        // data-plane wiring); `bytes_total` lands in a later C
        // slice from the manifest stage.
        assert_eq!(row.bytes_completed, 0);
        assert_eq!(row.bytes_total, 0);

        // Confirm the wire field tracks the atomic by
        // reporting through the sink and re-snapshotting.
        guard.bytes_counter().report(4096);
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state2 = resp.into_inner();
        assert_eq!(state2.active.len(), 1);
        assert_eq!(state2.active[0].bytes_completed, 4096);

        // Drop the active row + record outcome → it should now
        // appear in `recent[]`.
        guard.record_outcome(true, None);
        drop(guard);

        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert!(state.active.is_empty());
        assert_eq!(state.recent.len(), 1);
        let rec = &state.recent[0];
        assert_eq!(rec.kind, WireKind::Pull as i32);
        assert!(rec.ok);
        assert_eq!(rec.error_message, "");
        // `bytes` is the final value of the per-row atomic
        // captured at Drop. The earlier `bytes_counter().report(4096)`
        // is what lands here; `files` is a later C slice.
        assert_eq!(rec.bytes, 4096);
        assert_eq!(rec.files, 0);
    }

    #[tokio::test]
    async fn get_state_recent_limit_truncates_to_most_recent_n() {
        // Push 5 records into the ring, request recent_limit=3,
        // expect the 3 most recent in oldest-first order.
        let svc = empty_service();
        for i in 0..5u32 {
            let guard = svc.active_jobs.register(
                ActiveJobKind::Pull,
                format!("peer{i}"),
                "mod".to_string(),
                "/".to_string(),
            );
            guard.record_outcome(true, None);
        }

        // recent_limit=3 → most-recent 3 only.
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 3 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 3);
        // Oldest-first within the truncated window: peer2, peer3, peer4.
        assert_eq!(state.recent[0].peer, "peer2");
        assert_eq!(state.recent[1].peer, "peer3");
        assert_eq!(state.recent[2].peer, "peer4");

        // recent_limit=0 → full ring (the daemon's default).
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 5);
        assert_eq!(state.recent[0].peer, "peer0");
        assert_eq!(state.recent[4].peer, "peer4");

        // recent_limit larger than what the daemon has → returns
        // everything (no error).
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 999 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 5);
    }

    #[tokio::test]
    async fn cancel_job_ok_for_delegated_pull() {
        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "mod".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        let token = guard.cancellation_token().clone();

        let resp = svc
            .cancel_job(Request::new(CancelJobRequest {
                transfer_id: id.clone(),
            }))
            .await
            .expect("cancel_job ok");
        assert_eq!(resp.into_inner().transfer_id, id);
        assert!(token.is_cancelled(), "delegated_pull token must be fired");

        // The active row stays in the table until Drop runs;
        // letting the guard fall out of scope here.
        drop(guard);
    }

    #[tokio::test]
    async fn cancel_job_failed_precondition_for_non_delegated_kind() {
        let svc = empty_service();
        for kind in [
            ActiveJobKind::Push,
            ActiveJobKind::Pull,
            ActiveJobKind::PullSync,
        ] {
            let guard =
                svc.active_jobs
                    .register(kind, "p".to_string(), "mod".to_string(), "/".to_string());
            let id = guard.transfer_id().to_string();
            let token = guard.cancellation_token().clone();

            let err = svc
                .cancel_job(Request::new(CancelJobRequest {
                    transfer_id: id.clone(),
                }))
                .await
                .expect_err("non-delegated kind must reject CancelJob");
            assert_eq!(err.code(), tonic::Code::FailedPrecondition);
            assert!(
                !token.is_cancelled(),
                "{}: token must NOT be fired when CancelJob is unsupported",
                kind.as_str()
            );
            drop(guard);
        }
    }

    #[tokio::test]
    async fn cancel_job_not_found_for_unknown_transfer_id() {
        let svc = empty_service();
        let err = svc
            .cancel_job(Request::new(CancelJobRequest {
                transfer_id: "t-nope".to_string(),
            }))
            .await
            .expect_err("unknown id must NotFound");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn cancel_job_invalid_argument_for_empty_id() {
        let svc = empty_service();
        let err = svc
            .cancel_job(Request::new(CancelJobRequest {
                transfer_id: String::new(),
            }))
            .await
            .expect_err("empty id must InvalidArgument");
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn get_state_failure_record_carries_error_message() {
        let svc = empty_service();
        {
            let guard = svc.active_jobs.register(
                ActiveJobKind::Push,
                "p".to_string(),
                String::new(),
                String::new(),
            );
            guard.record_outcome(false, Some("module not found".to_string()));
        }
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 1);
        assert!(!state.recent[0].ok);
        assert_eq!(state.recent[0].error_message, "module not found");
    }
}
