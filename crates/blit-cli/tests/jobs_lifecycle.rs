//! w9-5: jobs/detach lifecycle e2e (tests-jobs-lifecycle-no-e2e).
//!
//! The detached-job lifecycle — `--detach` output, `jobs list`,
//! `jobs watch` to a terminal state, and the `jobs cancel` exit-code
//! contract (`docs/plan/TUI_DESIGN.md` §6.5: 0 cancelled / 1 not
//! found / 2 unsupported) — previously ran in zero tests; coverage
//! stopped at formatting/exit-code unit tests in `jobs.rs`. This file
//! is the regression net W4 needed before changing cancellation
//! (that change has since landed: D-2026-07-04-3 / w4-5 flipped
//! CancelJob dispatch on for attached push/pull_sync, so exit 2 no
//! longer occurs for those kinds; the 0/1/2 mapping is unchanged).
//!
//! Watch exit codes (see `run_jobs_watch`): 0 finished-ok,
//! 1 finished-failed, 2 not-found, 3 timeout-while-active.
//!
//! The dual-daemon delegation harness builds on the shared `common`
//! spawn primitives (consolidated by w9-3).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, spawn_fake_blit_server, DaemonOptions, SpawnedDaemon, TestContext};

// ---------------------------------------------------------------
// Single-daemon cases: list shape, cancel/watch unknown-id codes.
// ---------------------------------------------------------------

#[test]
fn jobs_list_on_idle_daemon_exits_zero_with_empty_active() {
    let ctx = TestContext::new();
    let remote = format!("127.0.0.1:{}", ctx.daemon_port);

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("jobs").arg("list").arg(&remote).arg("--json");
    let output = run_with_timeout(cmd, Duration::from_secs(30));

    assert_eq!(
        output.status.code(),
        Some(0),
        "jobs list must exit 0 once the RPC returns cleanly\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "jobs list --json must emit valid JSON ({err})\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let active = json
        .get("active")
        .and_then(|a| a.as_array())
        .expect("JSON must carry an `active` array");
    assert!(
        active.is_empty(),
        "an idle daemon must report no active transfers, got: {active:?}"
    );
}

#[test]
fn jobs_cancel_unknown_id_exits_one() {
    let ctx = TestContext::new();
    let remote = format!("127.0.0.1:{}", ctx.daemon_port);

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("jobs")
        .arg("cancel")
        .arg(&remote)
        .arg("no-such-transfer-id");
    let output = run_with_timeout(cmd, Duration::from_secs(30));

    assert_eq!(
        output.status.code(),
        Some(1),
        "cancel of an unknown id must exit 1 (NotFound)\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn jobs_watch_unknown_id_exits_two() {
    let ctx = TestContext::new();
    let remote = format!("127.0.0.1:{}", ctx.daemon_port);

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("jobs")
        .arg("watch")
        .arg(&remote)
        .arg("no-such-transfer-id")
        .arg("--timeout-secs")
        .arg("15");
    let output = run_with_timeout(cmd, Duration::from_secs(30));

    assert_eq!(
        output.status.code(),
        Some(2),
        "watch of an unknown id must exit 2 (NotFound)\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------
// Delegation harness (dual daemon / fake source) — built on the
// shared `common` spawn primitives (w9-3).
// ---------------------------------------------------------------

struct DelegationContext {
    _ctx: TestContext,
    _second: Option<SpawnedDaemon>,
    src_port: u16,
    dst_port: u16,
    cli_bin: PathBuf,
    config_dir: PathBuf,
    module_src_dir: Option<PathBuf>,
    module_dst_dir: PathBuf,
}

impl DelegationContext {
    /// Real source daemon + delegation-enabled destination daemon.
    fn with_real_source() -> Self {
        // The context's own daemon is the plain source; the
        // destination (whose delegation gate + job table are under
        // test) is the second daemon.
        let ctx = TestContext::new();
        let dst = ctx.spawn_second_daemon(
            "daemon_dst",
            &DaemonOptions {
                delegation: true,
                ..Default::default()
            },
        );

        Self {
            src_port: ctx.daemon_port,
            dst_port: dst.port,
            cli_bin: ctx.cli_bin.clone(),
            config_dir: ctx.config_dir.clone(),
            module_src_dir: Some(ctx.module_dir.clone()),
            module_dst_dir: dst.module_dir.clone(),
            _ctx: ctx,
            _second: Some(dst),
        }
    }

    /// Fake stalling source + delegation-enabled destination daemon.
    /// The fake's port is owned by the caller's `FakeServerGuard`.
    fn with_stalling_source(fake_port: u16) -> Self {
        let ctx = TestContext::builder().delegation(true).build();

        Self {
            src_port: fake_port,
            dst_port: ctx.daemon_port,
            cli_bin: ctx.cli_bin.clone(),
            config_dir: ctx.config_dir.clone(),
            module_src_dir: None,
            module_dst_dir: ctx.module_dir.clone(),
            _ctx: ctx,
            _second: None,
        }
    }

    fn source_remote(&self) -> String {
        format!("127.0.0.1:{}:/test/", self.src_port)
    }

    fn dest_remote(&self) -> String {
        format!("127.0.0.1:{}:/test/", self.dst_port)
    }

    fn dest_host(&self) -> String {
        format!("127.0.0.1:{}", self.dst_port)
    }

    fn run_blit(&self, args: &[&str]) -> std::process::Output {
        let mut cmd = Command::new(&self.cli_bin);
        cmd.arg("--config-dir").arg(&self.config_dir);
        for arg in args {
            cmd.arg(arg);
        }
        run_with_timeout(cmd, Duration::from_secs(60))
    }
}

/// Run a detached delegated copy and return the transfer_id parsed
/// from the `--json` detach output ({"outcome":"detached", ...}).
fn detach_copy(ctx: &DelegationContext) -> String {
    let src = ctx.source_remote();
    let dst = ctx.dest_remote();
    let output = ctx.run_blit(&["copy", &src, &dst, "--detach", "--json"]);
    assert!(
        output.status.success(),
        "detached copy must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "--detach --json must emit valid JSON ({err})\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    assert_eq!(
        json.get("outcome").and_then(|o| o.as_str()),
        Some("detached"),
        "detach JSON must carry outcome=detached, got: {json}"
    );
    let transfer_id = json
        .get("transfer_id")
        .and_then(|t| t.as_str())
        .expect("detach JSON must carry transfer_id");
    assert!(!transfer_id.is_empty(), "transfer_id must not be empty");
    transfer_id.to_string()
}

#[test]
fn detached_copy_watch_to_terminal_then_cancel_is_not_found() {
    let ctx = DelegationContext::with_real_source();
    fs::write(
        ctx.module_src_dir
            .as_ref()
            .expect("real source")
            .join("payload.txt"),
        b"jobs-lifecycle-e2e",
    )
    .expect("write src file");

    let transfer_id = detach_copy(&ctx);

    // Watch the detached job to its terminal state. Whether the
    // tiny transfer is still active at subscribe time or already in
    // the recent ring, finished-ok exits 0 on both paths.
    let dest_host = ctx.dest_host();
    let watch = ctx.run_blit(&[
        "jobs",
        "watch",
        &dest_host,
        &transfer_id,
        "--timeout-secs",
        "30",
        "--json",
    ]);
    assert_eq!(
        watch.status.code(),
        Some(0),
        "watch must reach finished-ok\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&watch.stdout),
        String::from_utf8_lossy(&watch.stderr)
    );

    // The byte path was daemon-to-daemon; the payload must be on dst.
    let dest_file = ctx.module_dst_dir.join("payload.txt");
    assert_eq!(
        fs::read(&dest_file).expect("dst payload must exist"),
        b"jobs-lifecycle-e2e",
        "delegated copy must land the payload on the destination module"
    );

    // The finished job must be visible in `jobs list` (recent ring).
    let list = ctx.run_blit(&["jobs", "list", &dest_host, "--json"]);
    assert_eq!(list.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains(&transfer_id),
        "finished transfer must appear in jobs list, got:\n{stdout}"
    );

    // Cancelling a finished job is NotFound → exit 1 (§6.5).
    let cancel = ctx.run_blit(&["jobs", "cancel", &dest_host, &transfer_id]);
    assert_eq!(
        cancel.status.code(),
        Some(1),
        "cancel of a finished transfer must exit 1 (NotFound)\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&cancel.stdout),
        String::from_utf8_lossy(&cancel.stderr)
    );
}

#[test]
fn cancel_of_active_delegated_job_exits_zero() {
    // A fake source that accepts the gRPC connection but never
    // answers PullSync: the destination daemon emits Started (the
    // ActiveJobs row registers synchronously at dispatch, before the
    // handler runs), then stalls inside pull_sync_with_spec — a
    // deterministic window in which the job is active and cancelable.
    let fake = spawn_stalling_source();
    let ctx = DelegationContext::with_stalling_source(fake.port);

    let transfer_id = detach_copy(&ctx);

    let dest_host = ctx.dest_host();
    let cancel = ctx.run_blit(&["jobs", "cancel", &dest_host, &transfer_id]);
    assert_eq!(
        cancel.status.code(),
        Some(0),
        "cancel of an active delegated job must exit 0 (Cancelled)\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&cancel.stdout),
        String::from_utf8_lossy(&cancel.stderr)
    );
}

// ---------------------------------------------------------------
// Fake stalling source: a tonic server whose pull_sync never
// answers. Everything else is unimplemented. Served through the
// shared production-shaped scaffold (common::spawn_fake_blit_server).
// ---------------------------------------------------------------

fn spawn_stalling_source() -> common::FakeServerGuard {
    spawn_fake_blit_server(StallingPullSyncBlit, "fake stalling source")
}

struct StallingPullSyncBlit;

#[tonic::async_trait]
impl blit_core::generated::blit_server::Blit for StallingPullSyncBlit {
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
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    /// The point of this fake: accept the RPC and never answer.
    async fn pull_sync(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
    ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
        std::future::pending::<()>().await;
        unreachable!("pending() never resolves")
    }

    async fn subscribe(
        &self,
        _: tonic::Request<blit_core::generated::SubscribeRequest>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn list(
        &self,
        _: tonic::Request<blit_core::generated::ListRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn purge(
        &self,
        _: tonic::Request<blit_core::generated::PurgeRequest>,
    ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn complete_path(
        &self,
        _: tonic::Request<blit_core::generated::CompletionRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn list_modules(
        &self,
        _: tonic::Request<blit_core::generated::ListModulesRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn find(
        &self,
        _: tonic::Request<blit_core::generated::FindRequest>,
    ) -> Result<tonic::Response<Self::FindStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn disk_usage(
        &self,
        _: tonic::Request<blit_core::generated::DiskUsageRequest>,
    ) -> Result<tonic::Response<Self::DiskUsageStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn filesystem_stats(
        &self,
        _: tonic::Request<blit_core::generated::FilesystemStatsRequest>,
    ) -> Result<tonic::Response<blit_core::generated::FilesystemStatsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn delegated_pull(
        &self,
        _: tonic::Request<blit_core::generated::DelegatedPullRequest>,
    ) -> Result<tonic::Response<Self::DelegatedPullStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn get_state(
        &self,
        _: tonic::Request<blit_core::generated::GetStateRequest>,
    ) -> Result<tonic::Response<blit_core::generated::DaemonState>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn cancel_job(
        &self,
        _: tonic::Request<blit_core::generated::CancelJobRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }

    async fn clear_recent(
        &self,
        _: tonic::Request<blit_core::generated::ClearRecentRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ClearRecentResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stalling fake source"))
    }
}
