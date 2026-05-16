use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tempfile::tempdir;
use wait_timeout::ChildExt;

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
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    read_only: bool,
    delegation_allowed: bool,
}

#[derive(Serialize)]
struct DelegationSection {
    allow_delegated_pull: bool,
    allowed_source_hosts: Vec<String>,
}

fn pick_unused_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind probe listener")
        .local_addr()
        .expect("listener addr")
        .port()
}

#[allow(dead_code)]
struct DualDaemonContext {
    _work: tempfile::TempDir,
    workspace: PathBuf,
    daemon_a_port: u16,
    daemon_b_port: u16,
    _daemon_a: ChildGuard,
    _daemon_b: ChildGuard,
    cli_bin: PathBuf,
    config_dir: PathBuf,
    module_a_dir: PathBuf,
    module_b_dir: PathBuf,
}

impl DualDaemonContext {
    fn new(dest_delegation: bool) -> Self {
        let work = tempdir().expect("tempdir");
        let workspace = work.path().to_path_buf();

        let module_a_dir = workspace.join("module_a");
        fs::create_dir_all(&module_a_dir).expect("module a dir");

        let module_b_dir = workspace.join("module_b");
        fs::create_dir_all(&module_b_dir).expect("module b dir");

        let config_dir = workspace.join("cli-config");
        fs::create_dir_all(&config_dir).expect("cli config");

        let port_a = pick_unused_port();
        let port_b = pick_unused_port();
        assert_ne!(port_a, port_b, "ports must be different");

        let (cli_bin, daemon_bin) = binary_paths();
        build_daemon();

        let daemon_a = Self::spawn_daemon(
            &workspace,
            &daemon_bin,
            port_a,
            "daemon_a",
            &module_a_dir,
            false,
        );
        let daemon_b = Self::spawn_daemon(
            &workspace,
            &daemon_bin,
            port_b,
            "daemon_b",
            &module_b_dir,
            dest_delegation,
        );

        Self {
            _work: work,
            workspace,
            daemon_a_port: port_a,
            daemon_b_port: port_b,
            _daemon_a: daemon_a,
            _daemon_b: daemon_b,
            cli_bin,
            config_dir,
            module_a_dir,
            module_b_dir,
        }
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
                // Loopback sources must be authorized by IP/CIDR form, not
                // hostname form. This mirrors the production SSRF rule.
                allowed_source_hosts: vec!["127.0.0.1".to_string()],
            }),
        };

        let config_path = workspace.join(format!("{}.toml", name));
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
    let work = tempdir().expect("tempdir");
    let config_dir = work.path().join("cli-config");
    fs::create_dir_all(&config_dir).expect("cli config");
    let (cli_bin, _daemon_bin) = binary_paths();
    let stale = spawn_unimplemented_blit_server();
    let counter = work.path().join("stale.counter");

    let src_remote = "127.0.0.1:9:/test/";
    let dst_remote = format!("127.0.0.1:{}:/test/", stale.port);
    let mut cmd = Command::new(cli_bin);
    cmd.arg("--config-dir")
        .arg(&config_dir)
        .arg("copy")
        .arg(src_remote)
        .arg(dst_remote)
        .env("BLIT_TEST_COUNTER_FILE", &counter);

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

#[cfg(unix)]
#[test]
fn source_refuses_destination_negotiation_does_not_fall_back_to_relay() {
    let ctx = DualDaemonContext::new(true);
    let rejecting_source = spawn_rejecting_pull_sync_server();
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
    for arg in args {
        cmd.arg(arg);
    }
    if let Some(path) = counter {
        cmd.env("BLIT_TEST_COUNTER_FILE", path);
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

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn command");

    match child.wait_timeout(timeout).expect("wait for process") {
        Some(_status) => child
            .wait_with_output()
            .expect("collect command output after completion"),
        None => {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .expect("collect output after killing command");
            panic!(
                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                timeout,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
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

fn build_daemon() {
    let mut build = Command::new("cargo");
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    build.current_dir(workspace_root);
    build
        .arg("build")
        .arg("-p")
        .arg("blit-daemon")
        .arg("--bin")
        .arg("blit-daemon");
    let output = build.output().expect("invoke cargo build for blit-daemon");
    assert!(
        output.status.success(),
        "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

struct ChildGuard {
    child: Option<std::process::Child>,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct UnimplementedServerGuard {
    port: u16,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for UnimplementedServerGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn spawn_unimplemented_blit_server() -> UnimplementedServerGuard {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake server");
    let port = listener.local_addr().expect("fake addr").port();
    listener
        .set_nonblocking(true)
        .expect("set fake server nonblocking");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let join = thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("fake server runtime");
        runtime.block_on(async move {
            use blit_core::generated::blit_server::BlitServer;
            use tokio_stream::wrappers::TcpListenerStream;
            use tonic::transport::Server;

            let listener =
                tokio::net::TcpListener::from_std(listener).expect("tokio fake listener");
            Server::builder()
                .add_service(BlitServer::new(UnimplementedBlit))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("fake server");
        });
    });

    wait_for_port(port, "fake unimplemented destination");
    UnimplementedServerGuard {
        port,
        shutdown: Some(shutdown_tx),
        join: Some(join),
    }
}

fn spawn_rejecting_pull_sync_server() -> UnimplementedServerGuard {
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
                .add_service(BlitServer::new(RejectingPullSyncBlit))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("fake source server");
        });
    });

    wait_for_port(port, "fake rejecting source");
    UnimplementedServerGuard {
        port,
        shutdown: Some(shutdown_tx),
        join: Some(join),
    }
}

struct UnimplementedBlit;

#[tonic::async_trait]
impl blit_core::generated::blit_server::Blit for UnimplementedBlit {
    type PushStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::ServerPushResponse, tonic::Status>,
    >;
    type PullStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::PullChunk, tonic::Status>,
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

    async fn push(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
    ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn pull(
        &self,
        _: tonic::Request<blit_core::generated::PullRequest>,
    ) -> Result<tonic::Response<Self::PullStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn pull_sync(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
    ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
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
}

struct RejectingPullSyncBlit;

#[tonic::async_trait]
impl blit_core::generated::blit_server::Blit for RejectingPullSyncBlit {
    type PushStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::ServerPushResponse, tonic::Status>,
    >;
    type PullStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::PullChunk, tonic::Status>,
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

    async fn push(
        &self,
        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
    ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "test only exercises pull_sync",
        ))
    }

    async fn pull(
        &self,
        _: tonic::Request<blit_core::generated::PullRequest>,
    ) -> Result<tonic::Response<Self::PullStream>, tonic::Status> {
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
}
