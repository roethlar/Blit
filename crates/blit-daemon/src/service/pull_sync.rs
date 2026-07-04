//! Bidirectional pull with manifest comparison for selective transfers.
//!
//! This module implements the PullSync RPC which allows clients to send their
//! local manifest so the server can compare and only send files that need updating.

use super::push::{bind_data_plane_listener, generate_token, TransferStats};
use super::util::{
    metadata_mtime_seconds, normalize_relative_path, permissions_mode, resolve_module,
    resolve_relative_path,
};
use super::PullSyncSender;
use crate::runtime::{ModuleConfig, RootExport};

use base64::{engine::general_purpose, Engine as _};
use blit_core::buffer::BufferPool;
use blit_core::engine::{initial_stream_proposal, TransferDial};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::{
    client_pull_message, server_pull_message, BlockHashRequest, BlockTransfer,
    BlockTransferComplete, CapacityProfile, ClientPullMessage, ComparisonMode, DataPlaneResize,
    DataPlaneResizeOp, DataTransferNegotiation, FileHeader, FileList, ManifestBatch, MirrorMode,
    PullSummary, PullSyncAck, ServerPullMessage, TransferOperationSpec,
};
use blit_core::manifest::{
    compare_manifests, files_needing_transfer, CompareMode, CompareOptions, FileStatus,
};
use blit_core::remote::transfer::data_plane::DataPlaneSession;
use blit_core::remote::transfer::operation_spec::NormalizedTransferOperation;
use blit_core::remote::transfer::plan_transfer_payloads;
use blit_core::remote::transfer::sink::{DataPlaneSink, TransferSink};
use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
use blit_core::transfer_plan::PlanOptions;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tonic::{Status, Streaming};

/// Handle a bidirectional PullSync stream.
pub(crate) async fn handle_pull_sync_stream(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    mut stream: Streaming<ClientPullMessage>,
    tx: PullSyncSender,
    force_grpc_override: bool,
    server_checksums_enabled: bool,
    active_job: &crate::active_jobs::ActiveJobGuard,
) -> Result<(), Status> {
    // Phase 1: Receive the unified TransferOperationSpec and normalize
    // it at the wire boundary. NormalizedTransferOperation::from_spec
    // is the single chokepoint that validates spec_version, rejects
    // unknown enum values, and folds Unspecified into concrete defaults.
    let raw_spec = match receive_spec(&mut stream).await? {
        Some(s) => s,
        None => {
            return Err(Status::invalid_argument(
                "expected TransferOperationSpec as first message",
            ))
        }
    };
    let spec = NormalizedTransferOperation::from_spec(raw_spec)
        .map_err(|e| Status::invalid_argument(format!("invalid TransferOperationSpec: {e:#}")))?;

    // Populate the ActiveJobs row now that we know the
    // endpoint (b-2-set-endpoint). Recorded verbatim from
    // the wire; containment is verified below when joining
    // onto the module root.
    active_job.set_endpoint(spec.module.clone(), spec.source_path.clone());

    // Resolve module from spec
    let module = resolve_module(&modules, default_root.as_ref(), &spec.module).await?;

    let force_grpc = spec.force_grpc || force_grpc_override;
    let mirror_mode = spec.mirror_enabled();
    let mirror_kind = spec.mirror_mode;
    let require_complete_scan = spec.require_complete_scan;
    let compare_mode_kind = spec.compare_mode;
    let client_wants_checksum = matches!(compare_mode_kind, ComparisonMode::Checksum);
    // Filter parity (F10): the source-side filter from the spec is
    // applied during enumeration via FileEnumerator and post-applied
    // to the deletion candidate list. None means "no filter".
    let source_filter = spec.filter.clone();
    let resume_settings = spec.resume;
    let resume_mode = resume_settings.enabled;
    // Clamp block size to safe limit to prevent server OOM
    use blit_core::copy::MAX_BLOCK_SIZE;
    let block_size = resume_settings.block_size.min(MAX_BLOCK_SIZE as u32);

    // Acknowledge header with server capabilities
    send_pull_sync_ack(&tx, server_checksums_enabled).await?;

    // Resolve path
    let requested = if spec.source_path.trim().is_empty() {
        PathBuf::from(".")
    } else {
        resolve_relative_path(&spec.source_path)?
    };

    // F2: resolve and verify containment at the entry point.
    let root = super::util::resolve_contained_path(&module, &requested)?;
    if !root.exists() {
        return Err(Status::not_found(format!(
            "path not found in module '{}': {}",
            module.name, spec.source_path
        )));
    }

    // Phase 2: Receive client manifest
    let client_manifest = receive_client_manifest(&mut stream).await?;

    // Phase 3: Enumerate server files and compare with client manifest.
    // Compute checksums if client requests checksum mode and server has checksums enabled.
    let compute_checksums = client_wants_checksum && server_checksums_enabled;
    let (server_entries, scan_outcome) = collect_pull_entries_with_checksums(
        &module.path,
        &root,
        &requested,
        compute_checksums,
        source_filter.clone().unwrap_or_default(),
    )
    .await?;

    // R47-F3 / R49-F2: refuse when the source scan was incomplete
    // and either:
    //   - mirror mode is on (pull_sync's delete-list builder
    //     would translate "absent at source" into "delete from
    //     destination," so a missing subtree silently deletes from
    //     the client); or
    //   - the initiator set `require_complete_scan` (the `blit
    //     move` case — initiator will delete the source after the
    //     pull, so partial scans silently lose files we couldn't
    //     read).
    if (mirror_mode || require_complete_scan) && !scan_outcome.is_complete() {
        let reason = if mirror_mode {
            "mirror pull"
        } else {
            "pull-then-delete-source (move)"
        };
        let preview = scan_outcome
            .suppressed_errors
            .iter()
            .take(5)
            .map(|e| format!("{} ({})", e.path, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Status::failed_precondition(format!(
            "refusing {} from module '{}': source scan was \
             incomplete ({} unreadable entr{}); the first {} reported: \
             {}. Resolve the scan errors (typically permissions) on \
             the daemon side.",
            reason,
            module.name,
            scan_outcome.suppressed_errors.len(),
            if scan_outcome.suppressed_errors.len() == 1 {
                "y"
            } else {
                "ies"
            },
            scan_outcome.suppressed_errors.len().min(5),
            preview
        )));
    }

    // Convert to FileHeader for comparison
    let server_manifest: Vec<FileHeader> =
        server_entries.iter().map(|e| e.header.clone()).collect();

    // Send manifest batch for progress reporting
    let total_bytes: u64 = server_manifest.iter().map(|h| h.size).sum();
    send_manifest_batch(&tx, server_manifest.len() as u64, total_bytes).await?;

    // ue-r2-1h: metadata-only session — the relay's manifest scan,
    // ported from the deprecated Pull RPC's `metadata_only` flag. One
    // file_header frame per enumerated entry, then a summary: no
    // comparison, no need-list, no delete-list, no data plane, no
    // bytes moved. Placed after the incomplete-scan refusal so a
    // metadata_only+mirror/move combination still fails closed rather
    // than reporting a manifest a destructive follow-up can't trust.
    // Summary counts mirror the deprecated path: files/bytes describe
    // the enumerated manifest (callers treat them as workload size,
    // not bytes moved).
    if spec.metadata_only {
        for entry in &server_entries {
            send_file_header(&tx, entry.header.clone()).await?;
        }
        send_summary(
            &tx,
            TransferStats {
                files_transferred: server_manifest.len() as u64,
                bytes_transferred: total_bytes,
                bytes_zero_copy: 0,
            },
            true,
            0,
        )
        .await?;
        return Ok(());
    }

    // Map protocol ComparisonMode onto the internal CompareMode used
    // by manifest comparison primitives. ignore_existing is now its
    // own orthogonal axis on the wire (R4-F2) so we read it directly
    // from the normalized spec rather than re-expanding an enum.
    let compare_mode = compare_mode_to_internal(compare_mode_kind);
    let ignore_existing = spec.ignore_existing;

    // Compare manifests: server files are source, client files are target
    let compare_opts = CompareOptions {
        mode: compare_mode,
        ignore_existing,
        include_deletions: mirror_mode,
    };
    let diff = compare_manifests(&server_manifest, &client_manifest, &compare_opts);

    // Scope the deletion candidate list per MirrorMode (closes F4).
    // - Off: no deletions ever (compare_opts already enforces this).
    // - FilteredSubset: only candidates that the source filter would
    //   have allowed are real deletions; out-of-scope client files
    //   are left alone so users don't lose files the source pretends
    //   not to know about (e.g. user excludes `*.log` on source —
    //   their `important.log` on dest is none of mirror's business).
    // - All: every absent-on-source client file is a real deletion.
    let scoped_deletions = scope_deletions(
        &diff.files_to_delete,
        &client_manifest,
        mirror_kind,
        &source_filter,
    );

    // Tell the client which files to delete (replaces the prior
    // dest-tree walking inference that mis-purged unchanged files).
    if mirror_mode {
        send_delete_list(&tx, &scoped_deletions).await?;
    }

    if diff.files_to_transfer.is_empty() && scoped_deletions.is_empty() {
        // Nothing to transfer
        send_summary(&tx, TransferStats::default(), false, 0).await?;
        return Ok(());
    }

    // Send list of files that will be downloaded (the NeedList)
    let files_to_send = files_needing_transfer(&diff);
    send_need_list(&tx, &files_to_send).await?;

    // Build client-size lookup so we can distinguish "mtime-only change"
    // (same size, different mtime) from "actual content change" (different
    // size). The former benefits from block-hash comparison — the server
    // compares Blake3 hashes block-by-block and sends only differing blocks.
    // When only mtime was touched, that sends zero bytes.
    let client_size_map: std::collections::HashMap<&str, u64> = client_manifest
        .iter()
        .map(|h| (h.relative_path.as_str(), h.size))
        .collect();

    // Effective resume set: always includes Modified files where the client
    // has the same size (mtime-only change → block-hash compare avoids full
    // re-transfer). When --resume is set, expand to ALL Modified files
    // including size-changed ones (partial-block resume).
    let effective_resume: std::collections::HashSet<String> = diff
        .files_to_transfer
        .iter()
        .filter(|f| f.status == FileStatus::Modified)
        .filter(|f| resume_mode || client_size_map.get(f.relative_path.as_str()) == Some(&f.size))
        .map(|f| f.relative_path.clone())
        .collect();

    // Filter server entries to only those needing transfer
    let entries_to_send: Vec<PullEntry> = server_entries
        .into_iter()
        .filter(|e| files_to_send.contains(&e.header.relative_path))
        .collect();

    if entries_to_send.is_empty() {
        send_summary(
            &tx,
            TransferStats::default(),
            false,
            scoped_deletions.len() as u64,
        )
        .await?;
        return Ok(());
    }

    // Phase 4: Transfer files
    let bytes_to_send: u64 = entries_to_send.iter().map(|e| e.header.size).sum();

    // ue-r2-1e: the daemon is the byte SENDER on pull_sync — one dial
    // per transfer, conservative start, bounded by the client's
    // advertised receiver profile from the spec (None = old client →
    // conservative defaults). Arc'd since ue-r2-2: the tuner and the
    // resize controller share it.
    let dial = TransferDial::conservative_within(spec.receiver_capacity.as_ref()).shared();

    if force_grpc {
        // gRPC fallback: stream via control plane (full files or blocks)
        if !effective_resume.is_empty() {
            // Block-level resume via gRPC bidirectional stream
            let stats = stream_via_block_resume_grpc(
                &module,
                &entries_to_send,
                block_size,
                &tx,
                &mut stream,
                &effective_resume,
            )
            .await?;
            send_summary(&tx, stats, true, scoped_deletions.len() as u64).await?;
        } else {
            // gRPC pull fallback uses the unified planner + sink so
            // tar-shard batching applies to the same workloads that
            // benefit from it on push (Step 4C).
            let stats =
                stream_via_grpc(&module, &root, entries_to_send, bytes_to_send, &tx, &dial).await?;
            send_summary(&tx, stats, true, scoped_deletions.len() as u64).await?;
        }
    } else if !effective_resume.is_empty() {
        // Data plane with block-level resume
        // Use gRPC for block hash exchange, data plane for block transfer
        let stats = stream_via_data_plane_resume(
            &dial,
            &module,
            entries_to_send,
            bytes_to_send,
            block_size,
            &tx,
            &mut stream,
            &effective_resume,
        )
        .await?;
        send_summary(&tx, stats, false, scoped_deletions.len() as u64).await?;
    } else {
        // Data plane transfer (full files). Pass the enumeration `root`,
        // not module.path — header.relative_path is relative to `root`.
        // ue-r2-1g: the daemon (byte sender, shape-knower) proposes the
        // stream count from the engine's shape table, bounded by the
        // client's advertised receiver profile.
        let stream_count = negotiated_pull_streams(
            bytes_to_send,
            entries_to_send.len(),
            spec.receiver_capacity.as_ref(),
            &dial,
        );
        // ue-r2-2: resize gate, pull direction — the client (byte
        // receiver AND dialer) advertised the capability bit in its
        // spec, and this arm is by construction the live-TCP,
        // non-resume path. The receiver profile is already folded
        // into the dial's ceiling.
        let resize_on = spec.capabilities.supports_stream_resize;
        let stats = stream_via_data_plane(
            &module,
            &root,
            entries_to_send,
            bytes_to_send,
            &tx,
            &mut stream,
            &dial,
            stream_count,
            resize_on,
        )
        .await?;
        send_summary(&tx, stats, false, scoped_deletions.len() as u64).await?;
    }

    Ok(())
}

async fn receive_spec(
    stream: &mut Streaming<ClientPullMessage>,
) -> Result<Option<TransferOperationSpec>, Status> {
    match stream.message().await {
        Ok(Some(msg)) => {
            if let Some(client_pull_message::Payload::Spec(spec)) = msg.payload {
                Ok(Some(spec))
            } else {
                Ok(None)
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(Status::internal(format!("failed to receive spec: {}", e))),
    }
}

/// Translate the protocol-level `ComparisonMode` enum onto the
/// internal `CompareMode`. `ignore_existing` is no longer carried via
/// the enum — it's an orthogonal field on `TransferOperationSpec`
/// (R4-F2) and the caller reads it from the normalized spec directly.
fn compare_mode_to_internal(mode: ComparisonMode) -> CompareMode {
    match mode {
        ComparisonMode::Checksum => CompareMode::Checksum,
        ComparisonMode::SizeOnly => CompareMode::SizeOnly,
        ComparisonMode::IgnoreTimes => CompareMode::IgnoreTimes,
        ComparisonMode::Force => CompareMode::Force,
        // Unspecified | SizeMtime — both fall back to the historical default.
        _ => CompareMode::Default,
    }
}

/// ue-r2-1g: stream count for the pull_sync full-file data plane. The
/// daemon is the byte SENDER and the workload-shape-knower; it
/// proposes from the engine-owned shape table, bounded by the client's
/// advertised receiver ceiling, and records the result on the dial
/// (the `ue-r2-2` resize target).
///
/// A client that advertised no capacity profile — or an unknown
/// (`0`) `max_streams` — gets today's single-stream behavior: REV4
/// Design §5 ("no capacity profile means use today's
/// static/conservative behavior") and the proto contract on
/// `CapacityProfile.max_streams` ("0 = unknown → sender stays at
/// today's negotiated stream_count"). The resume path does not call
/// this — its interleaved block-hash protocol is strictly ordered on
/// the control stream, so it stays single-stream by design (an
/// explicit RELIABLE exception, see `.review/findings/ue-r2-1g.md`).
fn negotiated_pull_streams(
    total_bytes: u64,
    file_count: usize,
    receiver_capacity: Option<&CapacityProfile>,
    dial: &TransferDial,
) -> u32 {
    let client_multistream_capable = receiver_capacity.is_some_and(|p| p.max_streams > 0);
    if !client_multistream_capable {
        // Record the settled single stream on the dial too (codex
        // ue-r2-1g F2): ue-r2-2's resize reads the dial as the
        // negotiation baseline, so leaving the constructor's floor (4)
        // there would misstate what this transfer actually runs.
        return dial.set_negotiated_streams(1) as u32;
    }
    // The dial's ceiling already folds in the client's profile (it was
    // built from the same spec field), so the proposal is
    // receiver-bounded; set_negotiated_streams re-clamps and records.
    let proposal = initial_stream_proposal(total_bytes, file_count, dial.ceiling_max_streams());
    dial.set_negotiated_streams(proposal as usize) as u32
}

async fn receive_client_manifest(
    stream: &mut Streaming<ClientPullMessage>,
) -> Result<Vec<FileHeader>, Status> {
    let mut manifest = Vec::new();

    loop {
        match stream.message().await {
            Ok(Some(msg)) => match msg.payload {
                Some(client_pull_message::Payload::LocalFile(header)) => {
                    manifest.push(header);
                }
                Some(client_pull_message::Payload::ManifestDone(_)) => {
                    break;
                }
                _ => {
                    return Err(Status::invalid_argument(
                        "unexpected message during manifest phase",
                    ));
                }
            },
            Ok(None) => break,
            Err(e) => {
                return Err(Status::internal(format!(
                    "failed to receive manifest: {}",
                    e
                )))
            }
        }
    }

    Ok(manifest)
}

async fn send_pull_sync_ack(
    tx: &PullSyncSender,
    server_checksums_enabled: bool,
) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::PullSyncAck(PullSyncAck {
            server_checksums_enabled,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send ack"))
}

async fn send_manifest_batch(
    tx: &PullSyncSender,
    file_count: u64,
    total_bytes: u64,
) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::ManifestBatch(ManifestBatch {
            file_count,
            total_bytes,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send manifest batch"))
}

/// ue-r2-1h: metadata-only sessions stream the enumerated manifest as
/// bare `file_header` frames (no `file_data` follows — the same frame
/// the gRPC fallback uses, minus the bytes).
async fn send_file_header(tx: &PullSyncSender, header: FileHeader) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::FileHeader(header)),
    }))
    .await
    .map_err(|_| Status::internal("failed to send file header"))
}

async fn send_need_list(tx: &PullSyncSender, files: &[String]) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::FilesToDownload(FileList {
            relative_paths: files.to_vec(),
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send need list"))
}

async fn send_delete_list(tx: &PullSyncSender, paths: &[String]) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::DeleteList(FileList {
            relative_paths: paths.to_vec(),
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send delete list"))
}

/// Apply MirrorMode + source filter scoping to the candidate deletion
/// list returned by `compare_manifests`. Closes F4 from
/// `docs/reviews/codebase_review_2026-05-01.md`: previously the
/// client walked its own dest tree and inferred "delete anything not
/// transferred", which mis-purged unchanged files and ignored filter
/// scope. Now the daemon — which has the filtered server manifest
/// and the unfiltered client manifest — computes the authoritative
/// list and sends it to the client.
fn scope_deletions(
    candidates: &[String],
    client_manifest: &[FileHeader],
    mirror: MirrorMode,
    filter: &Option<FileFilter>,
) -> Vec<String> {
    use std::time::{Duration, SystemTime};
    match mirror {
        MirrorMode::Off | MirrorMode::Unspecified => Vec::new(),
        MirrorMode::All => candidates.to_vec(),
        MirrorMode::FilteredSubset => {
            let Some(filter) = filter else {
                // No filter set, so "filtered subset" is the same as "all".
                return candidates.to_vec();
            };
            if filter.is_empty() {
                return candidates.to_vec();
            }
            let by_path: std::collections::HashMap<&str, &FileHeader> = client_manifest
                .iter()
                .map(|h| (h.relative_path.as_str(), h))
                .collect();
            candidates
                .iter()
                .filter(|path| {
                    let Some(h) = by_path.get(path.as_str()) else {
                        return false;
                    };
                    let mtime = if h.mtime_seconds > 0 {
                        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(h.mtime_seconds as u64))
                    } else {
                        None
                    };
                    filter.allows_relative(std::path::Path::new(path.as_str()), h.size, mtime)
                })
                .cloned()
                .collect()
        }
    }
}

async fn send_summary(
    tx: &PullSyncSender,
    stats: TransferStats,
    tcp_fallback: bool,
    entries_deleted: u64,
) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::Summary(PullSummary {
            files_transferred: stats.files_transferred,
            bytes_transferred: stats.bytes_transferred,
            bytes_zero_copy: stats.bytes_zero_copy,
            tcp_fallback_used: tcp_fallback,
            entries_deleted,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send summary"))
}

async fn stream_via_grpc(
    _module: &ModuleConfig,
    source_root: &Path,
    entries: Vec<PullEntry>,
    _total_bytes: u64,
    tx: &PullSyncSender,
    dial: &TransferDial,
) -> Result<TransferStats, Status> {
    use blit_core::remote::transfer::pipeline::execute_sink_pipeline;
    use blit_core::remote::transfer::sink::GrpcServerStreamingSink;
    use blit_core::remote::transfer::source::FsTransferSource;
    use blit_core::remote::transfer::DEFAULT_PAYLOAD_PREFETCH;
    use std::sync::Arc;

    // Reuse the unified planner so gRPC fallback emits the same
    // payload mix (single files / tar shards) as the TCP data plane —
    // closes Step 4C, no artificial single-file cripple.
    // ue-r2-1e: chunking comes from the live dial (conservative start,
    // receiver-profile-bounded), not the size-keyed ladder.
    let plan_options = PlanOptions {
        chunk_bytes_override: Some(dial.chunk_bytes()),
        ..Default::default()
    };
    let headers: Vec<FileHeader> = entries.iter().map(|e| e.header.clone()).collect();
    let planned = plan_transfer_payloads(headers, source_root, plan_options)
        .map_err(|err| Status::internal(format!("planning gRPC payloads: {err}")))?;

    let source: Arc<dyn TransferSource> =
        Arc::new(FsTransferSource::new(source_root.to_path_buf()));
    let sink: Arc<dyn blit_core::remote::transfer::sink::TransferSink> =
        Arc::new(GrpcServerStreamingSink::new(
            source.clone(),
            tx.clone(),
            dial.chunk_bytes(),
            source_root.to_path_buf(),
        ));

    let outcome = execute_sink_pipeline(
        source,
        vec![sink],
        planned.payloads,
        DEFAULT_PAYLOAD_PREFETCH,
        None,
    )
    .await
    .map_err(|err| Status::internal(format!("gRPC pull pipeline failed: {err}")))?;

    Ok(TransferStats {
        files_transferred: outcome.files_written as u64,
        bytes_transferred: outcome.bytes_written,
        bytes_zero_copy: 0,
    })
}

#[allow(clippy::too_many_arguments)]
async fn stream_via_data_plane(
    _module: &ModuleConfig,
    source_root: &Path,
    entries: Vec<PullEntry>,
    total_bytes: u64,
    tx: &PullSyncSender,
    stream: &mut Streaming<ClientPullMessage>,
    dial: &Arc<TransferDial>,
    stream_count: u32,
    resize_on: bool,
) -> Result<TransferStats, Status> {
    use blit_core::engine::{spawn_dial_tuner_with_resize, SharedStreamProbes};
    use blit_core::remote::transfer::generate_sub_token;
    use blit_core::remote::transfer::payload_file_count;
    use blit_core::remote::transfer::pipeline::{
        execute_sink_pipeline, execute_sink_pipeline_elastic, SinkControl,
    };

    // ue-r2-1e: dial-driven chunking (conservative start,
    // receiver-profile-bounded).
    let plan_options = PlanOptions {
        chunk_bytes_override: Some(dial.chunk_bytes()),
        ..Default::default()
    };

    // Set up data plane listener
    let listener = bind_data_plane_listener()
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
    let port = listener
        .local_addr()
        .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
        .port();
    let token = generate_token()?;
    let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
    // ue-r2-2: minted only when the client advertised resize — every
    // epoch-0 socket must echo it after the one-time token.
    let epoch0_sub = if resize_on {
        generate_sub_token().map_err(|err| Status::internal(format!("{err:#}")))?
    } else {
        Vec::new()
    };

    // Send negotiation. ue-r2-1g: stream_count is the engine's
    // shape-keyed, receiver-bounded proposal (negotiated_pull_streams);
    // the client fans out to exactly this many receive workers.
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::Negotiation(
            DataTransferNegotiation {
                tcp_port: port as u32,
                one_time_token: token_string,
                tcp_fallback: false,
                stream_count,
                // ue-r2-1b: on pull the CLIENT is the byte receiver, so
                // the profile travels client→daemon in
                // TransferOperationSpec.receiver_capacity — this field
                // stays unset on pull negotiations.
                receiver_capacity: None,
                // ue-r2-2: the full fold — client capability bit
                // (resize_on) AND own support AND this literal only
                // exists on the live-TCP, non-resume path.
                resize_enabled: resize_on,
                epoch0_sub_token: epoch0_sub.clone(),
            },
        )),
    }))
    .await
    .map_err(|_| Status::internal("failed to send negotiation"))?;

    // Plan transfer payloads against the enumeration root — header.relative_path
    // is relative to source_root (NOT module.path).
    let headers: Vec<FileHeader> = entries.iter().map(|e| e.header.clone()).collect();
    let planned = plan_transfer_payloads(headers, source_root, plan_options)
        .map_err(|err| Status::internal(format!("failed to plan payloads: {}", err)))?;

    let file_count = payload_file_count(&planned.payloads);

    // Shared buffer pool across all streams (hoisted out of
    // accept_and_wrap_sinks at ue-r2-2 so an ADDed stream's session
    // shares it; sizing unchanged — epoch-0 count, FIFO-fair
    // semaphore, growth is the W3.1 memory-aware-pool row).
    let streams = stream_count.max(1) as usize;
    let buffer_size = dial.chunk_bytes().max(64 * 1024);
    let pool_size = streams * 2 + 4;
    let memory_budget = buffer_size * pool_size * 2;
    let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));

    // ue-r2-1g: accept N token-authenticated connections and wrap each
    // as a DataPlaneSink (the multistream pattern harvested from the
    // Pull RPC deleted at ue-r2-1h). The shared pipeline's elastic
    // work-stealing queue distributes payloads across all streams.
    let probes: Option<SharedStreamProbes> = if resize_on {
        Some(Arc::new(std::sync::Mutex::new(Vec::new())))
    } else {
        None
    };
    let sinks = accept_and_wrap_sinks(
        &listener,
        &token,
        if resize_on {
            Some(epoch0_sub.as_slice())
        } else {
            None
        },
        streams,
        dial.chunk_bytes(),
        dial.prefetch_count(),
        source_root,
        Arc::clone(&pool),
        probes.as_ref(),
    )
    .await?;

    let source: Arc<dyn TransferSource> =
        Arc::new(FsTransferSource::new(source_root.to_path_buf()));

    if !resize_on {
        execute_sink_pipeline(source, sinks, planned.payloads, dial.prefetch_count(), None)
            .await
            .map_err(|err| Status::internal(format!("pull sync data plane pipeline: {err:#}")))?;
        return Ok(TransferStats {
            files_transferred: file_count as u64,
            bytes_transferred: total_bytes,
            bytes_zero_copy: 0,
        });
    }

    // ── ue-r2-2: resize controller (the daemon is sender/controller
    // on pull; the client dials). Owns the tuner's proposal stream,
    // the client's acks on the request stream, and the listener for
    // epoch-N sockets — armed for exactly one pending epoch at a time
    // (the dial's one-in-flight rule), with a TTL so an abandoned dial
    // lapses non-fatally. ──────────────────────────────────────────
    let probes = probes.expect("probe registry exists when resize is on");
    let (proposal_tx, mut proposal_rx) = tokio::sync::mpsc::unbounded_channel();
    let tuner = spawn_dial_tuner_with_resize(dial, Arc::clone(&probes), Some(proposal_tx));
    let (ctl_tx, ctl_rx) = tokio::sync::mpsc::unbounded_channel::<SinkControl>();
    let prefetch = dial.prefetch_count().max(1);
    let (payload_tx, payload_rx) = tokio::sync::mpsc::channel(prefetch);
    let payloads = planned.payloads;
    let feeder = tokio::spawn(async move {
        for payload in payloads {
            if payload_tx.send(payload).await.is_err() {
                break;
            }
        }
    });
    let pipeline = execute_sink_pipeline_elastic(
        Arc::clone(&source),
        sinks,
        payload_rx,
        prefetch,
        None,
        Some(ctl_rx),
    );
    tokio::pin!(pipeline);

    enum Pending {
        /// Command sent; waiting for the client's ack.
        AwaitingAck {
            epoch: u32,
            target: usize,
            add: bool,
            sub_token: Vec<u8>,
        },
        /// ADD acked; the accept is armed until `expires`.
        AwaitingDial {
            epoch: u32,
            target: usize,
            sub_token: Vec<u8>,
            expires: tokio::time::Instant,
        },
    }
    let mut pending: Option<Pending> = None;
    let mut next_stream_id = streams as u32;
    let mut client_gone = false;
    let mut proposals_done = false;
    // Spawned epoch-N validation tasks (each settles its own epoch);
    // aborted at teardown so none outlives the handler.
    let mut validations: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    let outcome = loop {
        let dial_deadline = match &pending {
            Some(Pending::AwaitingDial { expires, .. }) => Some(*expires),
            _ => None,
        };
        tokio::select! {
            res = &mut pipeline => break res,
            proposal = proposal_rx.recv(), if pending.is_none() && !client_gone && !proposals_done => {
                let Some(p) = proposal else {
                    proposals_done = true;
                    continue;
                };
                let (op, sub_token) = if p.add {
                    match generate_sub_token() {
                        Ok(sub) => (DataPlaneResizeOp::Add, sub),
                        Err(err) => {
                            log::warn!("pull resize ADD skipped (no credential source): {err:#}");
                            dial.resize_settled(p.epoch, dial.live_streams(), false);
                            continue;
                        }
                    }
                } else {
                    (DataPlaneResizeOp::Remove, Vec::new())
                };
                let sent = tx
                    .send(Ok(ServerPullMessage {
                        payload: Some(server_pull_message::Payload::DataPlaneResize(
                            DataPlaneResize {
                                op: op as i32,
                                epoch: p.epoch,
                                target_stream_count: p.target_streams as u32,
                                sub_token: sub_token.clone(),
                            },
                        )),
                    }))
                    .await
                    .is_ok();
                if sent {
                    pending = Some(Pending::AwaitingAck {
                        epoch: p.epoch,
                        target: p.target_streams,
                        add: p.add,
                        sub_token,
                    });
                } else {
                    // Control stream gone — the transfer is ending.
                    dial.resize_settled(p.epoch, dial.live_streams(), false);
                }
            }
            msg = stream.message(), if !client_gone => match msg {
                Ok(Some(frame)) => {
                    if let Some(client_pull_message::Payload::DataPlaneResizeAck(ack)) =
                        frame.payload
                    {
                        match pending.take() {
                            Some(Pending::AwaitingAck { epoch, target, add, sub_token })
                                if ack.epoch == epoch =>
                            {
                                if !ack.accepted {
                                    dial.resize_settled(epoch, dial.live_streams(), false);
                                } else if add {
                                    // Client dials next; arm the accept.
                                    pending = Some(Pending::AwaitingDial {
                                        epoch,
                                        target,
                                        sub_token,
                                        expires: tokio::time::Instant::now()
                                            + PULL_ACCEPT_TIMEOUT,
                                    });
                                } else {
                                    // REMOVE: retire one worker — its END
                                    // record ends the client's worker.
                                    {
                                        let mut ps =
                                            probes.lock().expect("probe registry poisoned");
                                        if ps.len() > 1 {
                                            ps.pop();
                                        }
                                    }
                                    let _ = ctl_tx.send(SinkControl::RetireOne);
                                    dial.resize_settled(epoch, target, true);
                                }
                            }
                            other => {
                                pending = other;
                                log::debug!(
                                    "pull: ignoring unsolicited/stale resize ack (epoch {})",
                                    ack.epoch
                                );
                            }
                        }
                    }
                    // Anything else on the request stream mid-transfer
                    // was previously unread on this path; keep ignoring.
                }
                Ok(None) | Err(_) => client_gone = true,
            },
            accepted = listener.accept(), if dial_deadline.is_some() => match accepted {
                Ok((socket, addr)) => {
                    let Some(Pending::AwaitingDial { epoch, target, sub_token, .. }) =
                        pending.take()
                    else {
                        unreachable!("accept arm gated on AwaitingDial");
                    };
                    // ue-r2-2 review (codex): validate in a spawned
                    // task — an inline 15s token-read would freeze the
                    // controller (pipeline results, acks, expiry).
                    // The accept consumes the armed slot: a stray dial
                    // that beats the real one costs the epoch (settled
                    // refused inside the task) and the real socket
                    // degrades non-fatally client-side — bounded harm,
                    // no controller stall. The task settles the dial
                    // itself; on a finished pipeline it closes the
                    // socket with a clean END (codex C3) so the
                    // client's authorized worker exits normally.
                    let stream_id = next_stream_id;
                    next_stream_id += 1;
                    let token = token.clone();
                    let dial = Arc::clone(dial);
                    let source = Arc::clone(&source);
                    let source_root = source_root.to_path_buf();
                    let pool = Arc::clone(&pool);
                    let probes = Arc::clone(&probes);
                    let ctl_tx = ctl_tx.clone();
                    validations.push(tokio::spawn(async move {
                        match accept_one_resize_socket(
                            socket,
                            &token,
                            &sub_token,
                            &dial,
                            &source,
                            &source_root,
                            pool,
                            &probes,
                            stream_id,
                        )
                        .await
                        {
                            Ok(sink) => {
                                eprintln!(
                                    "blitd: pull data plane: resize epoch {} socket \
                                     accepted from {}",
                                    epoch, addr
                                );
                                if ctl_tx.send(SinkControl::Add(Arc::clone(&sink))).is_ok() {
                                    dial.resize_settled(epoch, target, true);
                                } else {
                                    let _ = sink.finish().await;
                                    dial.resize_settled(epoch, dial.live_streams(), false);
                                }
                            }
                            Err(status) => {
                                log::warn!(
                                    "pull data plane: dropping resize socket from {addr}: \
                                     {status}"
                                );
                                dial.resize_settled(epoch, dial.live_streams(), false);
                            }
                        }
                    }));
                }
                Err(err) => {
                    log::warn!("pull data plane: resize accept failed: {err}");
                }
            },
            _ = async {
                tokio::time::sleep_until(dial_deadline.expect("gated")).await
            }, if dial_deadline.is_some() => {
                if let Some(Pending::AwaitingDial { epoch, .. }) = pending.take() {
                    log::warn!("pull resize ADD epoch {epoch} expired unclaimed");
                    dial.resize_settled(epoch, dial.live_streams(), false);
                }
            }
        }
    };
    tuner.abort();
    for validation in validations {
        // A mid-validation task at teardown holds only a socket the
        // client already treats as optional (zero-received leniency);
        // aborting bounds the handler's lifetime.
        validation.abort();
    }
    let _ = feeder.await;
    outcome.map_err(|err| Status::internal(format!("pull sync data plane pipeline: {err:#}")))?;

    Ok(TransferStats {
        files_transferred: file_count as u64,
        bytes_transferred: total_bytes,
        bytes_zero_copy: 0,
    })
}

/// ue-r2-2: validate and wrap ONE epoch-N pull socket: 48-byte
/// handshake (one-time token ‖ this epoch's sub-token), then a
/// LiveProbe session + sink registered with the tuner. Refusals are
/// the caller's to treat as non-fatal (the armed slot stays).
#[allow(clippy::too_many_arguments)]
async fn accept_one_resize_socket(
    mut socket: tokio::net::TcpStream,
    expected_token: &[u8],
    expected_sub: &[u8],
    dial: &TransferDial,
    source: &Arc<dyn TransferSource>,
    source_root: &Path,
    pool: Arc<BufferPool>,
    probes: &blit_core::engine::SharedStreamProbes,
    stream_id: u32,
) -> Result<Arc<dyn TransferSink>, Status> {
    use blit_core::remote::transfer::progress::{LiveProbe, StreamId, StreamProbe};

    let mut buf = vec![0u8; expected_token.len() + expected_sub.len()];
    match tokio::time::timeout(PULL_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => {
            return Err(Status::internal(format!(
                "failed to read pull resize token: {err}"
            )));
        }
        Err(_elapsed) => {
            return Err(Status::deadline_exceeded(format!(
                "pull resize token read timed out after {:?}",
                PULL_TOKEN_TIMEOUT
            )));
        }
    }
    let (token, sub) = buf.split_at(expected_token.len());
    if token != expected_token || sub != expected_sub {
        return Err(Status::unauthenticated("invalid data plane token"));
    }

    let probe = StreamProbe::new(StreamId(stream_id));
    let tuner_view = StreamProbe::from_telemetry(probe.id(), probe.telemetry());
    let session = DataPlaneSession::from_stream_with_probe(
        socket,
        false,
        dial.chunk_bytes(),
        dial.prefetch_count(),
        pool,
        LiveProbe(probe),
    )
    .await;
    probes
        .lock()
        .expect("probe registry poisoned")
        .push(tuner_view);
    Ok(Arc::new(DataPlaneSink::new(
        session,
        Arc::clone(source),
        source_root.to_path_buf(),
    )))
}

// R47-F5 / R46-F7: bound the accept + token-read so a peer that
// opened the control RPC but never opened the data socket(s)
// can't pin this task (and the listener) indefinitely. Hoisted to
// module scope at ue-r2-2 — the resize controller shares them.
const PULL_ACCEPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const PULL_TOKEN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// Accept N TCP connections, validate each token, wrap each in a
/// `DataPlaneSink`. `source_root` is the enumeration root (module.path +
/// requested subpath) — files are read relative to this via header.relative_path.
///
/// ue-r2-1g: harvested verbatim from the deprecated Pull RPC — REV4
/// required the multistream pattern to live in PullSync before
/// `ue-r2-1h` deleted that RPC. ue-r2-2: when `epoch0_sub` is set the
/// handshake is token ‖ sub-token (48 bytes) and each stream carries a
/// LiveProbe registered in `probes` — the resize substrate. The pool
/// is caller-owned so epoch-N sessions share it.
#[allow(clippy::too_many_arguments)]
async fn accept_and_wrap_sinks(
    listener: &TcpListener,
    expected_token: &[u8],
    epoch0_sub: Option<&[u8]>,
    streams: usize,
    chunk_bytes: usize,
    payload_prefetch: usize,
    source_root: &Path,
    pool: Arc<BufferPool>,
    probes: Option<&blit_core::engine::SharedStreamProbes>,
) -> Result<Vec<Arc<dyn TransferSink>>, Status> {
    use blit_core::remote::transfer::progress::{LiveProbe, StreamId, StreamProbe};

    let source: Arc<dyn blit_core::remote::transfer::source::TransferSource> =
        Arc::new(FsTransferSource::new(source_root.to_path_buf()));

    let dst_root = source_root.to_path_buf();
    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
    let suffix_len = epoch0_sub.map(<[u8]>::len).unwrap_or(0);
    for idx in 0..streams {
        let (mut socket, addr) =
            match tokio::time::timeout(PULL_ACCEPT_TIMEOUT, listener.accept()).await {
                Ok(Ok(pair)) => pair,
                Ok(Err(err)) => {
                    return Err(Status::internal(format!("data plane accept failed: {err}")));
                }
                Err(_elapsed) => {
                    return Err(Status::deadline_exceeded(format!(
                        "pull data plane accept timed out after {:?} \
                         waiting for stream {}/{}",
                        PULL_ACCEPT_TIMEOUT,
                        idx + 1,
                        streams
                    )));
                }
            };
        eprintln!(
            "blitd: pull data plane: accepted connection {} from {}",
            idx, addr
        );

        // Validate token (and, under resize, the epoch-0 sub-token)
        // before handing the socket to a sink.
        let mut token_buf = vec![0u8; expected_token.len() + suffix_len];
        match tokio::time::timeout(PULL_TOKEN_TIMEOUT, socket.read_exact(&mut token_buf)).await {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => {
                return Err(Status::internal(format!(
                    "failed to read pull token: {err}"
                )));
            }
            Err(_elapsed) => {
                return Err(Status::deadline_exceeded(format!(
                    "pull token read timed out after {:?}",
                    PULL_TOKEN_TIMEOUT
                )));
            }
        }
        let (token_part, sub_part) = token_buf.split_at(expected_token.len());
        if token_part != expected_token || epoch0_sub.is_some_and(|sub| sub_part != sub) {
            log::warn!("pull data plane: invalid token");
            // ue-r2-1g self-review F3: a bad token is a credentials
            // failure — UNAUTHENTICATED, matching what the pull_sync
            // full-file path returned before the harvest and what the
            // resume path in this file still returns. (The deprecated
            // Pull RPC said PERMISSION_DENIED here; it died at
            // ue-r2-1h with no consumer keying on the code.)
            return Err(Status::unauthenticated("invalid data plane token"));
        }

        let sink: Arc<dyn TransferSink> = if let Some(probes) = probes {
            let probe = StreamProbe::new(StreamId(idx as u32));
            let tuner_view = StreamProbe::from_telemetry(probe.id(), probe.telemetry());
            let session = DataPlaneSession::from_stream_with_probe(
                socket,
                false,
                chunk_bytes,
                payload_prefetch,
                Arc::clone(&pool),
                LiveProbe(probe),
            )
            .await;
            probes
                .lock()
                .expect("probe registry poisoned")
                .push(tuner_view);
            Arc::new(DataPlaneSink::new(
                session,
                source.clone(),
                dst_root.clone(),
            ))
        } else {
            let session = DataPlaneSession::from_stream(
                socket,
                false,
                chunk_bytes,
                payload_prefetch,
                Arc::clone(&pool),
            )
            .await;
            Arc::new(DataPlaneSink::new(
                session,
                source.clone(),
                dst_root.clone(),
            ))
        };
        sinks.push(sink);
    }

    Ok(sinks)
}

/// Stream files using block-level resume via data plane (primary path).
///
/// Uses gRPC for block hash exchange, then sends blocks via TCP data plane.
/// Pipelines block hash requests to avoid per-file RTT penalty.
async fn stream_via_data_plane_resume(
    dial: &TransferDial,
    module: &ModuleConfig,
    entries: Vec<PullEntry>,
    _total_bytes: u64,
    block_size_param: u32,
    tx: &PullSyncSender,
    stream: &mut Streaming<ClientPullMessage>,
    effective_resume: &std::collections::HashSet<String>,
) -> Result<TransferStats, Status> {
    use blit_core::buffer::BufferPool;
    use blit_core::copy::DEFAULT_BLOCK_SIZE;
    use blit_core::remote::transfer::data_plane::DataPlaneSession;
    use tokio::io::AsyncReadExt;

    let block_size = if block_size_param == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        block_size_param as usize
    };

    // Set up data plane listener
    let listener = bind_data_plane_listener()
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
    let port = listener
        .local_addr()
        .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
        .port();
    let token = generate_token()?;
    let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);

    // Send negotiation
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::Negotiation(
            DataTransferNegotiation {
                tcp_port: port as u32,
                one_time_token: token_string,
                tcp_fallback: false,
                stream_count: 1, // Single stream for resume mode
                // ue-r2-1b: see the streaming-path negotiation above.
                receiver_capacity: None,
                resize_enabled: false,
                epoch0_sub_token: Vec::new(),
            },
        )),
    }))
    .await
    .map_err(|_| Status::internal("failed to send negotiation"))?;

    // R46-F7: bounded waits on accept + token-read. Same rationale
    // and values as `accept_and_wrap_sinks` (the full-file path) —
    // a stalled peer mustn't hold the daemon's listener indefinitely.
    // This path stays single-connection (resume is single-stream by
    // design), so it keeps its own inline accept.
    use std::time::Duration as StdDuration2;
    const ACCEPT_TIMEOUT: StdDuration2 = StdDuration2::from_secs(30);
    const TOKEN_TIMEOUT: StdDuration2 = StdDuration2::from_secs(15);
    let (socket, _) = match tokio::time::timeout(ACCEPT_TIMEOUT, listener.accept()).await {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => {
            return Err(Status::internal(format!(
                "failed to accept data plane connection: {}",
                e
            )));
        }
        Err(_elapsed) => {
            return Err(Status::deadline_exceeded(format!(
                "pull-sync data plane accept timed out after {:?}",
                ACCEPT_TIMEOUT
            )));
        }
    };

    // Verify token
    let expected_token = token;
    let mut token_buf = vec![0u8; expected_token.len()];
    let mut socket = socket;
    match tokio::time::timeout(TOKEN_TIMEOUT, socket.read_exact(&mut token_buf)).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            return Err(Status::internal(format!("failed to read token: {}", e)));
        }
        Err(_elapsed) => {
            return Err(Status::deadline_exceeded(format!(
                "pull-sync token read timed out after {:?}",
                TOKEN_TIMEOUT
            )));
        }
    }
    if token_buf != expected_token {
        return Err(Status::unauthenticated("invalid data plane token"));
    }

    // Create buffer pool
    let buffer_size = dial.chunk_bytes().max(64 * 1024);
    let pool_size = 4;
    let memory_budget = buffer_size * pool_size * 2;
    let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));

    // Create data plane session
    let mut session =
        DataPlaneSession::from_stream(socket, false, dial.chunk_bytes(), 8, pool).await;

    let mut stats = TransferStats::default();

    // Phase 1: Send all block hash requests upfront for resume-eligible files
    // This fills the pipeline so the client can compute hashes while we transfer data.
    for entry in entries.iter() {
        if effective_resume.contains(&entry.header.relative_path) {
            tx.send(Ok(ServerPullMessage {
                payload: Some(server_pull_message::Payload::BlockHashRequest(
                    BlockHashRequest {
                        relative_path: entry.header.relative_path.clone(),
                        block_size: block_size as u32,
                    },
                )),
            }))
            .await
            .map_err(|_| Status::internal("failed to send block hash request"))?;
        }
    }

    // Phase 2: Process each file, consuming block hash responses Just-In-Time.
    // This avoids buffering hashes for all files in memory (O(N) memory usage).
    // Because requests were sent in order, responses are guaranteed to arrive in order.
    let mut buffer = vec![0u8; block_size];

    for entry in entries {
        let abs_path = module.path.join(&entry.relative_path);
        let relative_path = &entry.header.relative_path;
        let is_effective_resume = effective_resume.contains(relative_path);

        // Get client hashes if resume-eligible (JIT from stream)
        let file_client_hashes = if is_effective_resume {
            match stream.message().await {
                Ok(Some(msg)) => {
                    if let Some(client_pull_message::Payload::BlockHashes(hash_list)) = msg.payload
                    {
                        if hash_list.relative_path == *relative_path {
                            Some(hash_list.hashes)
                        } else {
                            return Err(Status::internal(format!(
                                "protocol mismatch: expected hashes for '{}', got '{}'",
                                relative_path, hash_list.relative_path
                            )));
                        }
                    } else {
                        return Err(Status::invalid_argument("expected BlockHashes response"));
                    }
                }
                Ok(None) => {
                    return Err(Status::internal(
                        "stream closed before receiving all hash responses",
                    ))
                }
                Err(e) => return Err(Status::internal(format!("receiving block hashes: {}", e))),
            }
        } else {
            None
        };

        // Open file for streaming read
        let mut file = tokio::fs::File::open(&abs_path).await.map_err(|e| {
            Status::internal(format!("failed to open {}: {}", abs_path.display(), e))
        })?;

        let file_size = file
            .metadata()
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get metadata for {}: {}",
                    abs_path.display(),
                    e
                ))
            })?
            .len() as usize;

        // Process blocks by streaming
        let mut block_idx = 0usize;
        let mut offset = 0usize;

        loop {
            let bytes_read = file.read(&mut buffer).await.map_err(|e| {
                Status::internal(format!("reading block from {}: {}", abs_path.display(), e))
            })?;

            if bytes_read == 0 {
                break;
            }

            let block_content = &buffer[..bytes_read];
            let server_hash = blake3::hash(block_content);

            // Check if this block needs transfer
            let needs_transfer = match &file_client_hashes {
                Some(hashes) if block_idx < hashes.len() => {
                    server_hash.as_bytes() != hashes[block_idx].as_slice()
                }
                _ => true,
            };

            if needs_transfer {
                session
                    .send_block(relative_path, offset as u64, block_content)
                    .await
                    .map_err(|err| Status::internal(format!("sending block: {}", err)))?;

                stats.bytes_transferred += block_content.len() as u64;
            }

            offset += bytes_read;
            block_idx += 1;
        }

        // Signal file complete via data plane. Send mtime + perms with
        // the terminator so the receiver can stamp metadata even when
        // zero blocks transferred (mtime-only touch + auto-promote case).
        session
            .send_block_complete(
                relative_path,
                file_size as u64,
                entry.header.mtime_seconds,
                entry.header.permissions,
            )
            .await
            .map_err(|err| Status::internal(format!("sending block complete: {}", err)))?;

        stats.files_transferred += 1;
    }

    // Finish data plane session
    session
        .finish()
        .await
        .map_err(|err| Status::internal(format!("finishing data plane: {}", err)))?;

    Ok(stats)
}

/// Stream files using block-level resume via gRPC (fallback path).
///
/// For each file:
/// 1. If resume-eligible: request block hashes from client, compare, send only differing blocks
/// 2. If not resume-eligible (new file): send full file via block transfers
///
/// Note: This fallback uses simpler stop-and-wait protocol since it's for diagnostic use.
async fn stream_via_block_resume_grpc(
    module: &ModuleConfig,
    entries: &[PullEntry],
    block_size: u32,
    tx: &PullSyncSender,
    stream: &mut Streaming<ClientPullMessage>,
    effective_resume: &std::collections::HashSet<String>,
) -> Result<TransferStats, Status> {
    use blit_core::copy::DEFAULT_BLOCK_SIZE;
    use tokio::io::AsyncReadExt;

    let block_size = if block_size == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        block_size as usize
    };
    let mut stats = TransferStats::default();

    for entry in entries {
        let abs_path = module.path.join(&entry.relative_path);
        let relative_path = &entry.header.relative_path;
        let is_effective_resume = effective_resume.contains(relative_path);

        // Open file for streaming
        let mut file = tokio::fs::File::open(&abs_path).await.map_err(|e| {
            Status::internal(format!("failed to open {}: {}", abs_path.display(), e))
        })?;

        let file_size = file
            .metadata()
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get metadata for {}: {}",
                    abs_path.display(),
                    e
                ))
            })?
            .len() as usize;

        // Get client block hashes if resume-eligible
        let client_hashes: Option<Vec<Vec<u8>>> = if is_effective_resume {
            // Request block hashes from client
            tx.send(Ok(ServerPullMessage {
                payload: Some(server_pull_message::Payload::BlockHashRequest(
                    BlockHashRequest {
                        relative_path: relative_path.clone(),
                        block_size: block_size as u32,
                    },
                )),
            }))
            .await
            .map_err(|_| Status::internal("failed to send block hash request"))?;

            // Wait for client's block hash response
            match stream.message().await {
                Ok(Some(msg)) => {
                    if let Some(client_pull_message::Payload::BlockHashes(hash_list)) = msg.payload
                    {
                        if hash_list.relative_path == *relative_path {
                            Some(hash_list.hashes)
                        } else {
                            None // Path mismatch, send full file
                        }
                    } else {
                        None // Unexpected message, send full file
                    }
                }
                Ok(None) => None, // Stream closed, send full file
                Err(_) => None,   // Error, send full file
            }
        } else {
            None // New file, no client hashes
        };

        // Stream through file blocks
        let mut buffer = vec![0u8; block_size];
        let mut block_idx = 0usize;
        let mut offset = 0usize;

        loop {
            let bytes_read = file.read(&mut buffer).await.map_err(|e| {
                Status::internal(format!("reading block from {}: {}", abs_path.display(), e))
            })?;

            if bytes_read == 0 {
                break;
            }

            let block_content = &buffer[..bytes_read];
            let server_hash = blake3::hash(block_content);

            // Check if this block differs from client's
            let needs_transfer = match &client_hashes {
                Some(hashes) if block_idx < hashes.len() => {
                    // Compare Blake3 hashes (32 bytes)
                    server_hash.as_bytes() != hashes[block_idx].as_slice()
                }
                _ => true, // No client hash or index out of bounds, need to transfer
            };

            if needs_transfer {
                tx.send(Ok(ServerPullMessage {
                    payload: Some(server_pull_message::Payload::BlockTransfer(BlockTransfer {
                        relative_path: relative_path.clone(),
                        offset: offset as u64,
                        content: block_content.to_vec(),
                    })),
                }))
                .await
                .map_err(|_| Status::internal("failed to send block transfer"))?;

                stats.bytes_transferred += block_content.len() as u64;
            }

            offset += bytes_read;
            block_idx += 1;
        }

        // Signal file complete
        tx.send(Ok(ServerPullMessage {
            payload: Some(server_pull_message::Payload::BlockComplete(
                BlockTransferComplete {
                    relative_path: relative_path.clone(),
                    total_bytes: file_size as u64,
                },
            )),
        }))
        .await
        .map_err(|_| Status::internal("failed to send block complete"))?;

        stats.files_transferred += 1;
    }

    Ok(stats)
}

// ── Enumeration entries (relocated from the deleted service/pull.rs
// at ue-r2-1h — pull_sync is their only consumer) ────────────────────

pub(crate) struct PullEntry {
    pub(crate) header: FileHeader,
    pub(crate) relative_path: PathBuf,
}

/// R47-F3: returns the enumeration outcome alongside the entry
/// list so destructive callers (pull_sync's delete-list builder)
/// can detect an incomplete source scan and refuse to translate
/// "absent at source" into "delete from destination." On a clean
/// scan `outcome.suppressed_errors` is empty and behavior matches
/// the historical contract.
async fn collect_pull_entries_with_checksums(
    module_root: &Path,
    root: &Path,
    requested: &Path,
    compute_checksums: bool,
    filter: blit_core::fs_enum::FileFilter,
) -> Result<(Vec<PullEntry>, blit_core::enumeration::EnumerationOutcome), Status> {
    if root.is_file() {
        // Single-file root: physical path (for reads) is the requested path
        // from the module; wire path (in the header, for the client's
        // dest_root.join) must be empty so the client writes to its
        // already-resolved dest target — not nested under a basename it
        // already appended.
        let physical = if requested == Path::new(".") {
            root.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            requested.to_path_buf()
        };
        // R59 finding #4: apply the user filter to single-file roots
        // too. Pre-fix the daemon returned the entry unconditionally,
        // so `blit pull host:module/file.txt --exclude '*.txt'` still
        // pulled the file even though the local single-file path
        // (engine/single_file.rs) correctly skipped on the same flag.
        // Filter against the basename (matches what allows_entry
        // does for directory enumeration of leaf files).
        let name = physical.file_name().map(PathBuf::from);
        let size = std::fs::metadata(root).map(|m| m.len()).unwrap_or(0);
        let mtime = std::fs::metadata(root).and_then(|m| m.modified()).ok();
        let allows = match name.as_deref() {
            Some(name_path) => filter.allows_entry(Some(name_path), root, size, mtime),
            None => true,
        };
        if !allows {
            return Ok((
                Vec::new(),
                blit_core::enumeration::EnumerationOutcome::default(),
            ));
        }
        let mut header = build_file_header(module_root, &physical, compute_checksums)?;
        header.relative_path = String::new();
        return Ok((
            vec![PullEntry {
                header,
                relative_path: physical,
            }],
            blit_core::enumeration::EnumerationOutcome::default(),
        ));
    }

    if !root.is_dir() {
        return Err(Status::invalid_argument("unsupported path type for pull"));
    }

    let root_clone = root.to_path_buf();
    let requested_clone = requested.to_path_buf();
    let module_root = module_root.to_path_buf();
    tokio::task::spawn_blocking(
        move || -> Result<(Vec<PullEntry>, blit_core::enumeration::EnumerationOutcome), Status> {
            use rayon::prelude::*;

            let enumerator = blit_core::enumeration::FileEnumerator::new(filter);
            let (entries, outcome) = enumerator
                .enumerate_local_capturing(&root_clone)
                .map_err(|err| Status::internal(format!("enumeration error: {}", err)))?;

            // Filter to files only
            let file_entries: Vec<_> = entries
                .into_iter()
                .filter(|e| matches!(e.kind, blit_core::enumeration::EntryKind::File { .. }))
                .collect();

            // Physical path (relative to module_root, used for reading) = requested + entry
            // Wire path (in header.relative_path, used by client for dest_root.join) = entry only
            // Previously both were set to the joined form, causing the client to double-nest
            // when the CLI resolver had already appended the basename to dest_root.
            let files: Result<Vec<PullEntry>, Status> = if compute_checksums {
                file_entries
                    .into_par_iter()
                    .map(|entry| {
                        let physical = requested_clone.join(&entry.relative_path);
                        let wire = entry.relative_path.clone();
                        let mut header = build_file_header(&module_root, &physical, true)?;
                        header.relative_path = blit_core::path_posix::relative_path_to_posix(&wire);
                        Ok(PullEntry {
                            header,
                            relative_path: physical,
                        })
                    })
                    .collect()
            } else {
                file_entries
                    .into_iter()
                    .map(|entry| {
                        let physical = requested_clone.join(&entry.relative_path);
                        let wire = entry.relative_path.clone();
                        let mut header = build_file_header(&module_root, &physical, false)?;
                        header.relative_path = blit_core::path_posix::relative_path_to_posix(&wire);
                        Ok(PullEntry {
                            header,
                            relative_path: physical,
                        })
                    })
                    .collect()
            };

            files.map(|f| (f, outcome))
        },
    )
    .await
    .map_err(|err| Status::internal(format!("enumeration task failed: {}", err)))?
}

fn build_file_header(
    base: &Path,
    relative: &Path,
    compute_checksum: bool,
) -> Result<FileHeader, Status> {
    let abs_path = base.join(relative);

    if compute_checksum {
        // Open file once for both metadata and hashing
        let mut file = std::fs::File::open(&abs_path)
            .map_err(|err| Status::internal(format!("open {}: {}", abs_path.display(), err)))?;
        let metadata = file
            .metadata()
            .map_err(|err| Status::internal(format!("stat {}: {}", abs_path.display(), err)))?;

        // w7-4: hash via the shared read loop in blit-core (this was
        // the fifth hand-rolled copy, with a 256 KiB stack array).
        let checksum =
            blit_core::checksum::hash_reader(&mut file, blit_core::checksum::ChecksumType::Blake3)
                .map_err(|err| Status::internal(format!("hash {}: {err:#}", abs_path.display())))?;

        Ok(FileHeader {
            relative_path: normalize_relative_path(relative),
            size: metadata.len(),
            mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            permissions: permissions_mode(&metadata),
            checksum,
        })
    } else {
        // Just get metadata, no checksum needed
        let metadata = std::fs::metadata(&abs_path)
            .map_err(|err| Status::internal(format!("stat {}: {}", abs_path.display(), err)))?;

        Ok(FileHeader {
            relative_path: normalize_relative_path(relative),
            size: metadata.len(),
            mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            permissions: permissions_mode(&metadata),
            checksum: vec![],
        })
    }
}

#[cfg(test)]
mod single_file_filter_tests {
    //! R59 finding #4: the daemon pull single-file fast path
    //! returned the entry unconditionally, ignoring the user-supplied
    //! filter. Local single-file copy (engine/single_file.rs) already
    //! honored the filter, so the two paths drifted apart.
    //! (Relocated from service/pull.rs at ue-r2-1h with
    //! `collect_pull_entries_with_checksums`, the function they pin.)

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn single_file_root_excluded_by_filter_returns_empty() {
        let tmp = tempdir().unwrap();
        let module = tmp.path();
        let file = module.join("payload.txt");
        fs::write(&file, b"hello").unwrap();

        let mut filter = blit_core::fs_enum::FileFilter::default();
        filter.exclude_files = vec!["*.txt".to_string()];

        let (entries, outcome) = collect_pull_entries_with_checksums(
            module,
            &file,
            Path::new("payload.txt"),
            false,
            filter,
        )
        .await
        .unwrap();
        assert!(
            entries.is_empty(),
            "single-file root matching --exclude must yield no entries"
        );
        assert!(outcome.suppressed_errors.is_empty());
    }

    #[tokio::test]
    async fn single_file_root_included_by_filter_passes() {
        let tmp = tempdir().unwrap();
        let module = tmp.path();
        let file = module.join("payload.txt");
        fs::write(&file, b"hello").unwrap();

        let mut filter = blit_core::fs_enum::FileFilter::default();
        filter.include_files = vec!["*.txt".to_string()];

        let (entries, _) = collect_pull_entries_with_checksums(
            module,
            &file,
            Path::new("payload.txt"),
            false,
            filter,
        )
        .await
        .unwrap();
        assert_eq!(entries.len(), 1);
        // Wire path stays empty (preserves the single-file dest target
        // contract — client appends nothing).
        assert!(entries[0].header.relative_path.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.into(),
            size,
            mtime_seconds: 0,
            permissions: 0,
            checksum: vec![],
        }
    }

    fn manifest(paths: &[(&str, u64)]) -> Vec<FileHeader> {
        paths.iter().map(|(p, s)| header(p, *s)).collect()
    }

    #[test]
    fn scope_off_returns_empty() {
        let candidates = vec!["a.txt".into(), "b.tmp".into()];
        let client = manifest(&[("a.txt", 1), ("b.tmp", 1)]);
        let out = scope_deletions(&candidates, &client, MirrorMode::Off, &None);
        assert!(out.is_empty());
    }

    #[test]
    fn scope_all_returns_everything() {
        let candidates: Vec<String> = vec!["a.txt".into(), "b.tmp".into()];
        let client = manifest(&[("a.txt", 1), ("b.tmp", 1)]);
        let out = scope_deletions(&candidates, &client, MirrorMode::All, &None);
        assert_eq!(out, candidates);
    }

    #[test]
    fn filtered_subset_drops_excluded_paths() {
        // Source filter excludes *.tmp. A client file `b.tmp` shouldn't
        // be deleted even if absent from the filtered source manifest —
        // the filter excluded it on purpose, so it's none of mirror's
        // business.
        let mut filter = FileFilter::default();
        filter.exclude_files = vec!["*.tmp".into()];
        let candidates: Vec<String> = vec!["a.txt".into(), "b.tmp".into()];
        let client = manifest(&[("a.txt", 1), ("b.tmp", 1)]);
        let out = scope_deletions(
            &candidates,
            &client,
            MirrorMode::FilteredSubset,
            &Some(filter),
        );
        assert_eq!(out, vec!["a.txt".to_string()]);
    }

    #[test]
    fn filtered_subset_no_filter_acts_like_all() {
        // `FilteredSubset` with no filter is equivalent to `All` —
        // there's no scope to scope to, so every absent client file
        // is a real deletion.
        let candidates: Vec<String> = vec!["a.txt".into()];
        let client = manifest(&[("a.txt", 1)]);
        let out = scope_deletions(&candidates, &client, MirrorMode::FilteredSubset, &None);
        assert_eq!(out, candidates);
    }

    #[test]
    fn filtered_subset_respects_min_size() {
        // Source filter requires min_size=100. A small file would
        // never be transferred, so its absence on the source isn't a
        // signal to delete it on dest.
        let mut filter = FileFilter::default();
        filter.min_size = Some(100);
        let candidates: Vec<String> = vec!["small.txt".into(), "big.txt".into()];
        let client = manifest(&[("small.txt", 5), ("big.txt", 500)]);
        let out = scope_deletions(
            &candidates,
            &client,
            MirrorMode::FilteredSubset,
            &Some(filter),
        );
        assert_eq!(out, vec!["big.txt".to_string()]);
    }

    // ── ue-r2-1g: stream-count decision (engine proposal, gated on
    //    the client's advertised receiver profile) ──────────────────

    fn capacity(max_streams: u32) -> CapacityProfile {
        CapacityProfile {
            cpu_cores: 0,
            drain_class: 0,
            load_percent: 0,
            max_streams,
            drain_rate_bytes_per_sec: 0,
            max_chunk_bytes: 0,
            max_inflight_bytes: 0,
        }
    }

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn no_receiver_profile_stays_single_stream() {
        // Old client (pre-1e spec, no receiver_capacity): today's
        // behavior byte-for-byte, even for a huge workload — REV4
        // Design §5's "no profile → static/conservative behavior".
        let dial = TransferDial::conservative_within(None);
        assert_eq!(negotiated_pull_streams(40 * GIB, 500_000, None, &dial), 1);
        // codex F2: the settled count is recorded on the dial even on
        // the conservative arm (ue-r2-2 reads it as the baseline).
        assert_eq!(dial.initial_streams(), 1);
    }

    #[test]
    fn unknown_max_streams_stays_single_stream() {
        // Profile present but max_streams = 0 (unknown): the proto
        // contract says "sender stays at today's negotiated
        // stream_count" — 1 on pull_sync.
        let profile = capacity(0);
        let dial = TransferDial::conservative_within(Some(&profile));
        assert_eq!(
            negotiated_pull_streams(40 * GIB, 500_000, Some(&profile), &dial),
            1
        );
        assert_eq!(dial.initial_streams(), 1, "codex F2: recorded on the dial");
    }

    #[test]
    fn capable_profile_gets_the_engine_shape_table() {
        let profile = capacity(32);
        let dial = TransferDial::conservative_within(Some(&profile));
        // Table cap for a huge workload.
        assert_eq!(
            negotiated_pull_streams(40 * GIB, 10, Some(&profile), &dial),
            16
        );
        // File-count key fires independently of bytes (300 files → 2).
        let dial = TransferDial::conservative_within(Some(&profile));
        assert_eq!(negotiated_pull_streams(1, 300, Some(&profile), &dial), 2);
        // Tiny workload stays single-stream even for a capable client.
        let dial = TransferDial::conservative_within(Some(&profile));
        assert_eq!(negotiated_pull_streams(1, 10, Some(&profile), &dial), 1);
    }

    #[test]
    fn receiver_ceiling_clamps_the_proposal() {
        // Client advertises max_streams = 6: the 16-stream proposal
        // must clamp to the receiver's authority.
        let profile = capacity(6);
        let dial = TransferDial::conservative_within(Some(&profile));
        assert_eq!(
            negotiated_pull_streams(40 * GIB, 10, Some(&profile), &dial),
            6
        );
    }

    #[test]
    fn negotiated_count_is_recorded_on_the_dial() {
        // The dial carries the settled count forward (the ue-r2-2
        // resize target reads it).
        let profile = capacity(32);
        let dial = TransferDial::conservative_within(Some(&profile));
        let n = negotiated_pull_streams(40 * GIB, 10, Some(&profile), &dial);
        assert_eq!(dial.initial_streams(), n as usize);
    }

    #[test]
    fn filtered_subset_drops_unknown_paths() {
        // A path in the candidate list that isn't in the client manifest
        // (shouldn't happen in practice, but defensively) is dropped —
        // we can't size/mtime-check what we don't have metadata for.
        let mut filter = FileFilter::default();
        filter.exclude_files = vec!["*.tmp".into()];
        let candidates: Vec<String> = vec!["mystery.txt".into()];
        let client: Vec<FileHeader> = Vec::new();
        let out = scope_deletions(
            &candidates,
            &client,
            MirrorMode::FilteredSubset,
            &Some(filter),
        );
        assert!(out.is_empty());
    }
}
