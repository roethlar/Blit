use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use tempfile::tempdir;

mod common;
use common::{run_with_timeout, spawn_fake_blit_server, DaemonOptions, SpawnedDaemon, TestContext};

/// Dual real daemons in one workspace: daemon A (the context's own
/// daemon) is the plain source; daemon B is the destination whose
/// delegation gate is under test. Harness lives in `common` (w9-3).
struct DualDaemonContext {
    _ctx: TestContext,
    _daemon_b: SpawnedDaemon,
    workspace: PathBuf,
    daemon_a_port: u16,
    daemon_b_port: u16,
    cli_bin: PathBuf,
    config_dir: PathBuf,
    module_a_dir: PathBuf,
    module_b_dir: PathBuf,
}

impl DualDaemonContext {
    fn new(dest_delegation: bool) -> Self {
        let ctx = TestContext::new();
        let daemon_b = ctx.spawn_second_daemon(
            "daemon_b",
            &DaemonOptions {
                delegation: dest_delegation,
                ..Default::default()
            },
        );

        Self {
            workspace: ctx.workspace.clone(),
            daemon_a_port: ctx.daemon_port,
            daemon_b_port: daemon_b.port,
            cli_bin: ctx.cli_bin.clone(),
            config_dir: ctx.config_dir.clone(),
            module_a_dir: ctx.module_dir.clone(),
            module_b_dir: daemon_b.module_dir.clone(),
            _ctx: ctx,
            _daemon_b: daemon_b,
        }
    }

    fn source_remote(&self) -> String {
        format!("127.0.0.1:{}:/test/", self.daemon_a_port)
    }

    fn dest_remote(&self) -> String {
        format!("127.0.0.1:{}:/test/", self.daemon_b_port)
    }

    fn counter_path(&self, name: &str) -> PathBuf {
        self.workspace.join(format!("{name}.counter"))
    }
}

#[test]
fn remote_to_remote_copy_delegates_directly_without_cli_byte_path() {
    let ctx = DualDaemonContext::new(true);
    let payload = vec![b'x'; 2 * 1024 * 1024];
    fs::write(ctx.module_a_dir.join("payload.bin"), &payload).expect("write src file");

    let counter = ctx.counter_path("direct");
    let output = run_blit(
        &ctx,
        &["copy", &ctx.source_remote(), &ctx.dest_remote()],
        Some(&counter),
    );
    assert_success(&output);

    assert_eq!(
        fs::read(ctx.module_b_dir.join("payload.bin")).unwrap(),
        payload
    );
    let counters = read_counters(&counter);
    assert_eq!(
        counters.remote_transfer_source_constructed, 0,
        "direct path must not construct RemoteTransferSource"
    );
    assert_eq!(
        counters.cli_data_plane_outbound_bytes, 0,
        "direct path must not send payload bytes from the CLI data plane"
    );
}

#[test]
fn remote_to_remote_gate_reject_does_not_fall_back_to_relay() {
    let ctx = DualDaemonContext::new(false);
    fs::write(ctx.module_a_dir.join("payload.bin"), b"payload").expect("write src file");

    let counter = ctx.counter_path("gate_reject");
    let output = run_blit(
        &ctx,
        &["copy", &ctx.source_remote(), &ctx.dest_remote()],
        Some(&counter),
    );
    assert!(
        !output.status.success(),
        "delegation-disabled destination should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("delegated pull is disabled"),
        "expected gate reason in stderr, got:\n{stderr}"
    );
    assert!(!ctx.module_b_dir.join("payload.bin").exists());

    let counters = read_counters(&counter);
    assert_eq!(counters.remote_transfer_source_constructed, 0);
    assert_eq!(counters.cli_data_plane_outbound_bytes, 0);
}

#[test]
fn remote_to_remote_explicit_relay_uses_legacy_cli_byte_path() {
    let ctx = DualDaemonContext::new(false);
    let payload = vec![b'r'; 1024 * 1024];
    fs::write(ctx.module_a_dir.join("relay.bin"), &payload).expect("write src file");

    let counter = ctx.counter_path("relay");
    let output = run_blit(
        &ctx,
        &[
            "copy",
            "--relay-via-cli",
            &ctx.source_remote(),
            &ctx.dest_remote(),
        ],
        Some(&counter),
    );
    assert_success(&output);

    assert_eq!(
        fs::read(ctx.module_b_dir.join("relay.bin")).unwrap(),
        payload
    );
    let counters = read_counters(&counter);
    assert!(
        counters.remote_transfer_source_constructed > 0,
        "--relay-via-cli must construct the relay source"
    );
    assert!(
        counters.cli_data_plane_outbound_bytes >= payload.len() as u64,
        "relay path should send payload-sized bytes through the CLI data plane; counters={counters:?}"
    );
}

#[test]
fn remote_to_remote_relay_transfers_nested_tree() {
    // ue-r2-1h: the relay's enumeration (`scan_remote_files`) and
    // per-file byte streaming (`open_remote_file`) moved from the
    // deleted Pull RPC onto PullSync sessions (metadata-only scan +
    // single-file force_grpc reads). A nested multi-file tree
    // exercises both against the REAL daemon: recursive header
    // enumeration with correct relative paths, then byte-identical
    // per-file relay.
    let ctx = DualDaemonContext::new(false);
    let big = vec![b'B'; 512 * 1024];
    fs::create_dir_all(ctx.module_a_dir.join("nested/deep")).expect("mkdirs");
    fs::write(ctx.module_a_dir.join("top.bin"), &big).expect("write top");
    fs::write(ctx.module_a_dir.join("nested/mid.txt"), b"middle file").expect("write mid");
    fs::write(ctx.module_a_dir.join("nested/deep/leaf.txt"), b"leaf").expect("write leaf");

    let counter = ctx.counter_path("relay_tree");
    let output = run_blit(
        &ctx,
        &[
            "copy",
            "--relay-via-cli",
            &ctx.source_remote(),
            &ctx.dest_remote(),
        ],
        Some(&counter),
    );
    assert_success(&output);

    assert_eq!(fs::read(ctx.module_b_dir.join("top.bin")).unwrap(), big);
    assert_eq!(
        fs::read(ctx.module_b_dir.join("nested/mid.txt")).unwrap(),
        b"middle file"
    );
    assert_eq!(
        fs::read(ctx.module_b_dir.join("nested/deep/leaf.txt")).unwrap(),
        b"leaf"
    );

    let counters = read_counters(&counter);
    assert!(
        counters.remote_transfer_source_constructed > 0,
        "--relay-via-cli must construct the relay source"
    );
}

#[test]
fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
    let work = tempdir().expect("tempdir");
    let config_dir = work.path().join("cli-config");
    fs::create_dir_all(&config_dir).expect("cli config");
    let cli_bin = common::cli_bin();
    let stale = spawn_fake_blit_server(UnimplementedBlit, "fake unimplemented destination");
    let counter = work.path().join("stale.counter");

    let src_remote = "127.0.0.1:9:/test/";
    let dst_remote = format!("127.0.0.1:{}:/test/", stale.port);
    let mut cmd = Command::new(cli_bin);
    cmd.arg("--config-dir")
        .arg(&config_dir)
        .arg("--diagnostics-counter-file")
        .arg(&counter)
        .arg("copy")
        .arg(src_remote)
        .arg(dst_remote);

    let output = run_with_timeout(cmd, Duration::from_secs(20));
    assert!(
        !output.status.success(),
        "stale destination should fail without fallback"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not implement DelegatedPull"),
        "expected explicit upgrade/relay message, got:\n{stderr}"
    );

    let counters = read_counters(&counter);
    assert_eq!(counters.remote_transfer_source_constructed, 0);
    assert_eq!(counters.cli_data_plane_outbound_bytes, 0);
}

#[test]
fn source_refuses_destination_negotiation_does_not_fall_back_to_relay() {
    let ctx = DualDaemonContext::new(true);
    let rejecting_source = spawn_fake_blit_server(RejectingPullSyncBlit, "fake rejecting source");
    let counter = ctx.counter_path("source_refuses");
    let src_remote = format!("127.0.0.1:{}:/test/", rejecting_source.port);

    let output = run_blit(
        &ctx,
        &["copy", &src_remote, &ctx.dest_remote()],
        Some(&counter),
    );
    assert!(
        !output.status.success(),
        "source negotiation refusal should fail without fallback"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("source refused delegated pull"),
        "expected NEGOTIATE wording in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("source ACL rejected delegated peer"),
        "expected source refusal reason in stderr, got:\n{stderr}"
    );
    assert!(
        fs::read_dir(&ctx.module_b_dir).unwrap().next().is_none(),
        "destination should remain empty after source negotiation refusal"
    );

    let counters = read_counters(&counter);
    assert_eq!(counters.remote_transfer_source_constructed, 0);
    assert_eq!(counters.cli_data_plane_outbound_bytes, 0);
}

fn run_blit(
    ctx: &DualDaemonContext,
    args: &[&str],
    counter: Option<&Path>,
) -> std::process::Output {
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir").arg(&ctx.config_dir);
    // audit-l39: --diagnostics-counter-file replaced the pre-0.1.1
    // BLIT_TEST_COUNTER_FILE env var. Both flags are global, so they
    // must appear before the subcommand.
    if let Some(path) = counter {
        cmd.arg("--diagnostics-counter-file").arg(path);
    }
    for arg in args {
        cmd.arg(arg);
    }
    run_with_timeout(cmd, Duration::from_secs(60))
}

fn assert_success(output: &std::process::Output) {
    if !output.status.success() {
        panic!(
            "blit failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[derive(Debug, Default)]
struct CounterValues {
    cli_data_plane_outbound_bytes: u64,
    remote_transfer_source_constructed: u64,
}

fn read_counters(path: &Path) -> CounterValues {
    let mut out = CounterValues::default();
    let Ok(contents) = fs::read_to_string(path) else {
        return out;
    };
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let value = parts
            .next()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        match name {
            "cli_data_plane_outbound_bytes" => {
                out.cli_data_plane_outbound_bytes =
                    out.cli_data_plane_outbound_bytes.saturating_add(value);
            }
            "remote_transfer_source_constructed" => {
                out.remote_transfer_source_constructed =
                    out.remote_transfer_source_constructed.saturating_add(value);
            }
            _ => {}
        }
    }
    out
}

struct UnimplementedBlit;

#[tonic::async_trait]
impl blit_core::generated::blit_server::Blit for UnimplementedBlit {
    type PushStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::ServerPushResponse, tonic::Status>,
    >;
    type PullSyncStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::ServerPullMessage, tonic::Status>,
    >;
    type FindStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::FindEntry, tonic::Status>,
    >;
    type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
    >;
    type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
    >;
    type SubscribeStream = std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<
                    Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
                > + Send,
        >,
    >;

    type TransferStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::TransferFrame, tonic::Status>,
    >;

    // otp-1: unified-session wire surface; fakes refuse like the
    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
    async fn transfer(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
    ) -> Result<tonic::Response<Self::TransferStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("otp-1 stub"))
    }

    async fn push(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
    ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn pull_sync(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
    ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn subscribe(
        &self,
        _: tonic::Request<blit_core::generated::SubscribeRequest>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn list(
        &self,
        _: tonic::Request<blit_core::generated::ListRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn purge(
        &self,
        _: tonic::Request<blit_core::generated::PurgeRequest>,
    ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn complete_path(
        &self,
        _: tonic::Request<blit_core::generated::CompletionRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn list_modules(
        &self,
        _: tonic::Request<blit_core::generated::ListModulesRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn find(
        &self,
        _: tonic::Request<blit_core::generated::FindRequest>,
    ) -> Result<tonic::Response<Self::FindStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn disk_usage(
        &self,
        _: tonic::Request<blit_core::generated::DiskUsageRequest>,
    ) -> Result<tonic::Response<Self::DiskUsageStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn filesystem_stats(
        &self,
        _: tonic::Request<blit_core::generated::FilesystemStatsRequest>,
    ) -> Result<tonic::Response<blit_core::generated::FilesystemStatsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn delegated_pull(
        &self,
        _: tonic::Request<blit_core::generated::DelegatedPullRequest>,
    ) -> Result<tonic::Response<Self::DelegatedPullStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn get_state(
        &self,
        _: tonic::Request<blit_core::generated::GetStateRequest>,
    ) -> Result<tonic::Response<blit_core::generated::DaemonState>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn cancel_job(
        &self,
        _: tonic::Request<blit_core::generated::CancelJobRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn clear_recent(
        &self,
        _: tonic::Request<blit_core::generated::ClearRecentRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ClearRecentResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }
}

struct RejectingPullSyncBlit;

#[tonic::async_trait]
impl blit_core::generated::blit_server::Blit for RejectingPullSyncBlit {
    type PushStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::ServerPushResponse, tonic::Status>,
    >;
    type PullSyncStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::ServerPullMessage, tonic::Status>,
    >;
    type FindStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::FindEntry, tonic::Status>,
    >;
    type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
    >;
    type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
    >;
    type SubscribeStream = std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<
                    Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
                > + Send,
        >,
    >;

    type TransferStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::TransferFrame, tonic::Status>,
    >;

    // otp-1: unified-session wire surface; fakes refuse like the
    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
    async fn transfer(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
    ) -> Result<tonic::Response<Self::TransferStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("otp-1 stub"))
    }

    async fn push(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
    ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn pull_sync(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
    ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
        Err(tonic::Status::permission_denied(
            "source ACL rejected delegated peer",
        ))
    }

    async fn subscribe(
        &self,
        _: tonic::Request<blit_core::generated::SubscribeRequest>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn list(
        &self,
        _: tonic::Request<blit_core::generated::ListRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn purge(
        &self,
        _: tonic::Request<blit_core::generated::PurgeRequest>,
    ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn complete_path(
        &self,
        _: tonic::Request<blit_core::generated::CompletionRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn list_modules(
        &self,
        _: tonic::Request<blit_core::generated::ListModulesRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn find(
        &self,
        _: tonic::Request<blit_core::generated::FindRequest>,
    ) -> Result<tonic::Response<Self::FindStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn disk_usage(
        &self,
        _: tonic::Request<blit_core::generated::DiskUsageRequest>,
    ) -> Result<tonic::Response<Self::DiskUsageStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn filesystem_stats(
        &self,
        _: tonic::Request<blit_core::generated::FilesystemStatsRequest>,
    ) -> Result<tonic::Response<blit_core::generated::FilesystemStatsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn delegated_pull(
        &self,
        _: tonic::Request<blit_core::generated::DelegatedPullRequest>,
    ) -> Result<tonic::Response<Self::DelegatedPullStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn get_state(
        &self,
        _: tonic::Request<blit_core::generated::GetStateRequest>,
    ) -> Result<tonic::Response<blit_core::generated::DaemonState>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn cancel_job(
        &self,
        _: tonic::Request<blit_core::generated::CancelJobRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn clear_recent(
        &self,
        _: tonic::Request<blit_core::generated::ClearRecentRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ClearRecentResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }
}
