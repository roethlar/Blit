//! w9-5: jobs/detach lifecycle e2e (tests-jobs-lifecycle-no-e2e).
//!
//! The detached-job lifecycle — `--detach` output, `jobs list`,
//! `jobs watch` to a terminal state, and the `jobs cancel` exit-code
//! contract (`docs/plan/TUI_DESIGN.md` §6.5: 0 cancelled / 1 not
//! found / 2 unsupported) — previously ran in zero tests; coverage
//! stopped at formatting/exit-code unit tests in `jobs.rs`. This file
//! is the regression net W4 needs before changing cancellation.
//!
//! Watch exit codes (see `run_jobs_watch`): 0 finished-ok,
//! 1 finished-failed, 2 not-found, 3 timeout-while-active.
//!
//! The dual-daemon delegation harness mirrors `remote_remote.rs`
//! (consolidation of the harness clones is w9-3's job).

use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tempfile::tempdir;

mod common;
use common::{run_with_timeout, ChildGuard, TestContext};

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
// Delegation harness (dual daemon / fake source), mirroring
// remote_remote.rs.
// ---------------------------------------------------------------

#[derive(Serialize)]
struct DaemonConfig {
    daemon: DaemonSection,
    #[serde(rename = "module")]
    modules: Vec<ModuleSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delegation: Option<DelegationSection>,
}

#[derive(Serialize)]
struct DaemonSection {
    bind: String,
    port: u16,
    no_mdns: bool,
}

#[derive(Serialize)]
struct ModuleSection {
    name: String,
    path: PathBuf,
    comment: Option<String>,
    read_only: bool,
    delegation_allowed: bool,
}

#[derive(Serialize)]
struct DelegationSection {
    allow_delegated_pull: bool,
    allowed_source_hosts: Vec<String>,
}

fn binary_paths() -> (PathBuf, PathBuf) {
    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir
        .parent()
        .expect("deps parent directory")
        .to_path_buf();
    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
    let daemon_bin = bin_dir.join(if cfg!(windows) {
        "blit-daemon.exe"
    } else {
        "blit-daemon"
    });
    (cli_bin, daemon_bin)
}

fn wait_for_port(port: u16, label: &str) {
    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "{label} failed to listen on {port}");
}

fn spawn_daemon(
    workspace: &Path,
    bin: &Path,
    port: u16,
    name: &str,
    module_path: &Path,
    delegation_enabled: bool,
) -> ChildGuard {
    let config = DaemonConfig {
        daemon: DaemonSection {
            bind: "127.0.0.1".into(),
            port,
            no_mdns: true,
        },
        modules: vec![ModuleSection {
            name: "test".into(),
            path: module_path.to_path_buf(),
            comment: None,
            read_only: false,
            delegation_allowed: true,
        }],
        delegation: delegation_enabled.then(|| DelegationSection {
            allow_delegated_pull: true,
            // Loopback sources must be authorized by IP/CIDR form,
            // mirroring the production SSRF rule (see remote_remote.rs).
            allowed_source_hosts: vec!["127.0.0.1".to_string()],
        }),
    };

    let config_path = workspace.join(format!("{name}.toml"));
    let toml = toml::to_string(&config).expect("serialize config");
    fs::write(&config_path, toml).expect("write config");

    let child = Command::new(bin)
        .arg("--config")
        .arg(&config_path)
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn daemon");

    wait_for_port(port, &format!("daemon {name}"));
    ChildGuard::new(child)
}

struct DelegationContext {
    _work: tempfile::TempDir,
    src_port: u16,
    dst_port: u16,
    _src_daemon: Option<ChildGuard>,
    _dst_daemon: ChildGuard,
    cli_bin: PathBuf,
    config_dir: PathBuf,
    module_src_dir: Option<PathBuf>,
    module_dst_dir: PathBuf,
}

impl DelegationContext {
    /// Real source daemon + delegation-enabled destination daemon.
    fn with_real_source() -> Self {
        let work = tempdir().expect("tempdir");
        let workspace = work.path().to_path_buf();

        let module_src_dir = workspace.join("module_src");
        fs::create_dir_all(&module_src_dir).expect("module src dir");
        let module_dst_dir = workspace.join("module_dst");
        fs::create_dir_all(&module_dst_dir).expect("module dst dir");
        let config_dir = workspace.join("cli-config");
        fs::create_dir_all(&config_dir).expect("cli config");

        let (cli_bin, daemon_bin) = binary_paths();

        let src_port = common::pick_unused_port();
        let dst_port = common::pick_unused_port();
        assert_ne!(src_port, dst_port, "ports must be different");

        let src_daemon = spawn_daemon(
            &workspace,
            &daemon_bin,
            src_port,
            "daemon_src",
            &module_src_dir,
            false,
        );
        let dst_daemon = spawn_daemon(
            &workspace,
            &daemon_bin,
            dst_port,
            "daemon_dst",
            &module_dst_dir,
            true,
        );

        Self {
            _work: work,
            src_port,
            dst_port,
            _src_daemon: Some(src_daemon),
            _dst_daemon: dst_daemon,
            cli_bin,
            config_dir,
            module_src_dir: Some(module_src_dir),
            module_dst_dir,
        }
    }

    /// Fake stalling source + delegation-enabled destination daemon.
    /// The fake's port is owned by the caller's `StallingSourceGuard`.
    fn with_stalling_source(fake_port: u16) -> Self {
        let work = tempdir().expect("tempdir");
        let workspace = work.path().to_path_buf();

        let module_dst_dir = workspace.join("module_dst");
        fs::create_dir_all(&module_dst_dir).expect("module dst dir");
        let config_dir = workspace.join("cli-config");
        fs::create_dir_all(&config_dir).expect("cli config");

        let (cli_bin, daemon_bin) = binary_paths();
        let dst_port = common::pick_unused_port();

        let dst_daemon = spawn_daemon(
            &workspace,
            &daemon_bin,
            dst_port,
            "daemon_dst",
            &module_dst_dir,
            true,
        );

        Self {
            _work: work,
            src_port: fake_port,
            dst_port,
            _src_daemon: None,
            _dst_daemon: dst_daemon,
            cli_bin,
            config_dir,
            module_src_dir: None,
            module_dst_dir,
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
// answers. Everything else is unimplemented (same shape as
// remote_remote.rs's fake daemons).
// ---------------------------------------------------------------

struct StallingSourceGuard {
    port: u16,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for StallingSourceGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn spawn_stalling_source() -> StallingSourceGuard {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake source");
    let port = listener.local_addr().expect("fake source addr").port();
    listener
        .set_nonblocking(true)
        .expect("set fake source nonblocking");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let join = thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("fake source runtime");
        runtime.block_on(async move {
            use blit_core::generated::blit_server::BlitServer;
            use tokio_stream::wrappers::TcpListenerStream;
            use tonic::transport::Server;

            let listener =
                tokio::net::TcpListener::from_std(listener).expect("tokio fake source listener");
            Server::builder()
                .add_service(BlitServer::new(StallingPullSyncBlit))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("fake source server");
        });
    });

    wait_for_port(port, "fake stalling source");
    StallingSourceGuard {
        port,
        shutdown: Some(shutdown_tx),
        join: Some(join),
    }
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
