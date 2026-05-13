use crate::cli::TransferArgs;
use eyre::{bail, eyre, Context, Result};
use std::path::Path;

#[cfg(test)]
use std::path::PathBuf;

use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::delegated_pull_error::Phase as DelegatedPullPhase;
use blit_core::generated::delegated_pull_progress::Payload as DelegatedPayload;
use blit_core::generated::{
    BytesProgress, DelegatedPullRequest, DelegatedPullSummary, RemoteSourceLocator,
};
use blit_core::remote::pull::PullSyncOptions;
use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePullClient};
use tonic::Code;

use super::endpoints::format_remote_endpoint;
use super::remote::spawn_progress_monitor_with_options;

pub async fn run_remote_to_remote_direct(
    args: &TransferArgs,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    mirror_mode: bool,
    require_complete_scan: bool,
) -> Result<()> {
    // Copy/mirror callers: `--relay-via-cli` is a valid escape
    // hatch, so error messages mention it.
    run_remote_to_remote_direct_inner(
        args,
        src,
        dst,
        mirror_mode,
        require_complete_scan,
        false, // defer_output
        true,  // relay_fallback_suggestable
    )
    .await
    .map(|_| ())
}

/// R51-F4: move's variant of [`run_remote_to_remote_direct`].
/// Returns the delegated summary instead of printing inline so
/// the caller can defer output until after source-delete.
///
/// R53-F2: move refuses `--relay-via-cli` (R50-F1), so error
/// messages must not point users at it — they'd be sent to a
/// flag the same command rejects.
pub async fn run_remote_to_remote_direct_deferred(
    args: &TransferArgs,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    mirror_mode: bool,
    require_complete_scan: bool,
) -> Result<DeferredDelegatedState> {
    run_remote_to_remote_direct_inner(
        args,
        src,
        dst,
        mirror_mode,
        require_complete_scan,
        true,  // defer_output
        false, // relay_fallback_suggestable — move refuses --relay-via-cli
    )
    .await
}

pub struct DeferredDelegatedState {
    pub summary: blit_core::generated::DelegatedPullSummary,
    pub src: RemoteEndpoint,
    pub dst: RemoteEndpoint,
}

pub fn print_deferred_delegated_result(args: &TransferArgs, state: &DeferredDelegatedState) {
    if args.json {
        print_delegated_json(&state.summary, &state.src, &state.dst);
    } else {
        describe_delegated_result(&state.summary, &state.src, &state.dst);
    }
}

async fn run_remote_to_remote_direct_inner(
    args: &TransferArgs,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    mirror_mode: bool,
    require_complete_scan: bool,
    defer_output: bool,
    relay_fallback_suggestable: bool,
) -> Result<DeferredDelegatedState> {
    let filter_spec = super::build_filter_spec(args)?;
    let pull_opts = PullSyncOptions {
        force_grpc: args.force_grpc,
        mirror_mode,
        delete_all_scope: args.delete_scope_all(),
        filter: Some(filter_spec),
        size_only: args.size_only,
        ignore_times: args.ignore_times,
        ignore_existing: args.ignore_existing,
        force: args.force,
        checksum: args.checksum,
        resume: args.resume,
        block_size: 0,
        // R49-F2: move arms set this true so the daemon refuses
        // partial source scans before we delete the source.
        require_complete_scan,
    };
    let spec = RemotePullClient::build_spec_from_options(&src, &pull_opts)?;
    let (dst_module, dst_destination_path) = destination_spec_fields(&dst)?;

    let request = DelegatedPullRequest {
        dst_module,
        dst_destination_path,
        src: Some(RemoteSourceLocator {
            host: src.host.clone(),
            port: src.port as u32,
        }),
        spec: Some(spec),
        trace_data_plane: args.trace_data_plane,
    };

    let uri = dst.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to destination {}", format_remote_endpoint(&dst)))?;

    let response = client.delegated_pull(request).await.map_err(|status| {
        let relay_hint = if relay_fallback_suggestable {
            " or pass --relay-via-cli"
        } else {
            ""
        };
        let relay_clause = if relay_fallback_suggestable {
            "; pass --relay-via-cli to route through the CLI host"
        } else {
            ""
        };
        if status.code() == Code::Unimplemented {
            eyre!(
                "destination daemon does not implement DelegatedPull; upgrade the destination \
                 daemon{relay_hint}"
            )
        } else if status.code() == Code::Unavailable {
            eyre!(
                "destination daemon is unavailable for delegated pull ({}){}",
                status.message(),
                relay_clause
            )
        } else {
            eyre!(
                "delegated remote-to-remote transfer failed: {}",
                status.message()
            )
        }
    })?;
    let mut stream = response.into_inner();

    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress final progress line on move
    );
    let mut summary: Option<DelegatedPullSummary> = None;
    let mut failure: Option<eyre::Report> = None;
    let mut bytes_progress_state = DelegatedBytesProgressState::default();

    loop {
        let message = match stream.message().await {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(status) => {
                failure = Some(if status.code() == Code::Unavailable {
                    let relay_clause = if relay_fallback_suggestable {
                        "; pass --relay-via-cli to route through the CLI host"
                    } else {
                        ""
                    };
                    eyre!(
                        "delegation stream lost ({}){}",
                        status.message(),
                        relay_clause
                    )
                } else {
                    eyre!("delegation stream failed: {}", status.message())
                });
                break;
            }
        };
        // clippy::collapsible_match wants the verbose-gated branch as a
        // match guard. We keep the inner `if` because the alternative
        // requires a second `Some(DelegatedPayload::Started(_)) => {}`
        // fallthrough arm, which is uglier than the explicit gate.
        #[allow(clippy::collapsible_match)]
        match message.payload {
            Some(DelegatedPayload::Started(started)) => {
                if args.verbose && !args.json {
                    eprintln!(
                        "[delegation] destination pulling from {} ({} stream(s))",
                        started.source_data_plane_endpoint, started.stream_count
                    );
                }
            }
            Some(DelegatedPayload::ManifestBatch(batch)) => {
                if let Some(progress) = progress_handle.as_ref() {
                    progress.report_manifest_batch(batch.file_count as usize);
                }
            }
            Some(DelegatedPayload::BytesProgress(bytes)) => {
                report_bytes_progress(progress_handle.as_ref(), &mut bytes_progress_state, &bytes);
            }
            Some(DelegatedPayload::Summary(done)) => {
                summary = Some(done);
                break;
            }
            Some(DelegatedPayload::Error(error)) => {
                failure = Some(map_delegated_error(
                    error.phase,
                    &error.upstream_message,
                    relay_fallback_suggestable,
                ));
                break;
            }
            None => {}
        }
    }

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }
    if let Some(error) = failure {
        return Err(error);
    }

    let summary = summary.ok_or_else(|| eyre!("delegation ended before summary"))?;
    let state = DeferredDelegatedState { summary, src, dst };
    if !defer_output {
        print_deferred_delegated_result(args, &state);
    }
    Ok(state)
}

fn report_bytes_progress(
    progress: Option<&blit_core::remote::transfer::RemoteTransferProgress>,
    state: &mut DelegatedBytesProgressState,
    bytes: &BytesProgress,
) {
    if let Some(progress) = progress {
        let file_delta = bytes
            .files_completed
            .saturating_sub(state.files_completed)
            .try_into()
            .unwrap_or(usize::MAX);
        let byte_delta = bytes.bytes_completed.saturating_sub(state.bytes_completed);
        state.files_completed = state.files_completed.max(bytes.files_completed);
        state.bytes_completed = state.bytes_completed.max(bytes.bytes_completed);
        if file_delta > 0 || byte_delta > 0 {
            progress.report_payload(file_delta, byte_delta);
        }
    }
}

#[derive(Default)]
struct DelegatedBytesProgressState {
    files_completed: u64,
    bytes_completed: u64,
}

fn map_delegated_error(
    phase: i32,
    message: &str,
    relay_fallback_suggestable: bool,
) -> eyre::Report {
    let phase = DelegatedPullPhase::try_from(phase).unwrap_or(DelegatedPullPhase::Unknown);
    let relay_clause = if relay_fallback_suggestable {
        ". Pass --relay-via-cli to route through the CLI host"
    } else {
        ""
    };
    let relay_clause_semi = if relay_fallback_suggestable {
        "; pass --relay-via-cli to route through the CLI host"
    } else {
        ""
    };
    match phase {
        DelegatedPullPhase::DelegationRejected => {
            eyre!("delegation rejected by destination daemon: {message}{relay_clause}")
        }
        DelegatedPullPhase::ConnectSource => {
            eyre!("destination daemon cannot reach source ({message}){relay_clause_semi}")
        }
        DelegatedPullPhase::Negotiate => eyre!("source refused delegated pull: {message}"),
        DelegatedPullPhase::Transfer => eyre!("delegated transfer failed: {message}"),
        DelegatedPullPhase::Apply => {
            eyre!("destination failed to apply delegated transfer: {message}")
        }
        DelegatedPullPhase::Unknown => eyre!("delegated transfer failed: {message}"),
    }
}

fn destination_spec_fields(dst: &RemoteEndpoint) -> Result<(String, String)> {
    match &dst.path {
        RemotePath::Module { module, rel_path } => {
            Ok((module.clone(), normalize_for_request(rel_path)))
        }
        RemotePath::Root { rel_path } => Ok((String::new(), normalize_for_request(rel_path))),
        RemotePath::Discovery => bail!(
            "remote destination must include a module or root (e.g., server:/module/ or server://path)"
        ),
    }
}

fn normalize_for_request(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

fn print_delegated_json(
    summary: &DelegatedPullSummary,
    src: &RemoteEndpoint,
    dst: &RemoteEndpoint,
) {
    use serde_json::json;
    let body = json!({
        "operation": "delegated_pull",
        "source": format_remote_endpoint(src),
        "destination": format_remote_endpoint(dst),
        "files_transferred": summary.files_transferred,
        "bytes_transferred": summary.bytes_transferred,
        "bytes_zero_copy": summary.bytes_zero_copy,
        "entries_deleted": summary.entries_deleted,
        "tcp_fallback": summary.tcp_fallback_used,
        "source_peer_observed": summary.source_peer_observed,
    });
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
}

fn describe_delegated_result(
    summary: &DelegatedPullSummary,
    src: &RemoteEndpoint,
    dst: &RemoteEndpoint,
) {
    println!(
        "Delegated remote-to-remote transfer complete: {} file(s), {} bytes (zero-copy {} bytes){} from {} to {}.",
        summary.files_transferred,
        summary.bytes_transferred,
        summary.bytes_zero_copy,
        if summary.tcp_fallback_used {
            " [gRPC fallback]"
        } else {
            ""
        },
        format_remote_endpoint(src),
        format_remote_endpoint(dst)
    );
    if summary.entries_deleted > 0 {
        println!(
            "Mirror purge removed {} entrie(s) on destination.",
            summary.entries_deleted
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
    use tokio::sync::mpsc;

    fn endpoint(path: RemotePath) -> RemoteEndpoint {
        RemoteEndpoint {
            host: "localhost".to_string(),
            port: 9031,
            path,
        }
    }

    #[test]
    fn destination_fields_for_module_root_use_dot_path() {
        let dst = endpoint(RemotePath::Module {
            module: "mod".to_string(),
            rel_path: PathBuf::new(),
        });
        let (module, path) = destination_spec_fields(&dst).unwrap();
        assert_eq!(module, "mod");
        assert_eq!(path, ".");
    }

    #[test]
    fn destination_fields_for_subpath_normalize_forward_slashes() {
        let dst = endpoint(RemotePath::Module {
            module: "mod".to_string(),
            rel_path: PathBuf::from("a").join("b"),
        });
        let (module, path) = destination_spec_fields(&dst).unwrap();
        assert_eq!(module, "mod");
        assert_eq!(path, "a/b");
    }

    #[test]
    fn bytes_progress_reports_cumulative_values_as_deltas() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let progress = RemoteTransferProgress::new(tx);
        let mut state = DelegatedBytesProgressState::default();

        report_bytes_progress(
            Some(&progress),
            &mut state,
            &BytesProgress {
                files_completed: 1,
                files_total: 3,
                bytes_completed: 1024,
                bytes_total: 4096,
            },
        );
        report_bytes_progress(
            Some(&progress),
            &mut state,
            &BytesProgress {
                files_completed: 2,
                files_total: 3,
                bytes_completed: 4096,
                bytes_total: 4096,
            },
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 1024
            }
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 3072
            }
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn bytes_progress_duplicate_cumulative_update_is_not_counted_twice() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let progress = RemoteTransferProgress::new(tx);
        let mut state = DelegatedBytesProgressState::default();
        let update = BytesProgress {
            files_completed: 1,
            files_total: 1,
            bytes_completed: 2048,
            bytes_total: 2048,
        };

        report_bytes_progress(Some(&progress), &mut state, &update);
        report_bytes_progress(Some(&progress), &mut state, &update);

        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 2048
            }
        ));
        assert!(rx.try_recv().is_err());
    }
}
