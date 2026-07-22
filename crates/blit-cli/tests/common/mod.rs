//! Shared integration-test harness (w9-3 consolidation).
//!
//! The single daemon-spawn implementation for every blit-cli test
//! binary: config serialization, port picking, the once-per-binary
//! `cargo build`, spawn + readiness poll, plus the shared `cli_bin()`
//! / `run_with_timeout` helpers and the production-shaped fake-server
//! scaffold. The per-file clones of this logic (remote_remote,
//! remote_pull_mirror, remote_checksum_negotiation,
//! remote_tcp_fallback, and the newer jobs_lifecycle /
//! readonly_enforcement mini-harnesses) were deleted in its favor —
//! extend this file instead of re-cloning it
//! (tests-five-daemon-harness-clones).

// Every test binary compiles this module and each uses a different
// subset of it, so per-binary dead_code lints would fire on whatever
// that binary happens not to touch. The blanket allow is the honest
// setting for a shared harness, not a mask for genuinely dead code.
#![allow(dead_code)]

use std::collections::HashSet;
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use blit_core::generated::{blit_client::BlitClient, ListModulesRequest};
use serde::Serialize;
use tempfile::tempdir;
use tonic::transport::Endpoint;
use wait_timeout::ChildExt;

// ---------------------------------------------------------------
// blitd.toml serialization — superset of every knob the deleted
// clones expressed.
// ---------------------------------------------------------------

#[derive(Serialize)]
pub struct DaemonConfig {
    pub daemon: DaemonSection,
    #[serde(rename = "module")]
    pub modules: Vec<ModuleSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation: Option<DelegationSection>,
}

#[derive(Serialize)]
pub struct DaemonSection {
    pub bind: String,
    pub port: u16,
    pub no_mdns: bool,
}

#[derive(Serialize)]
pub struct ModuleSection {
    pub name: String,
    pub path: PathBuf,
    pub comment: Option<String>,
    pub read_only: bool,
    /// The daemon defaults this to `true` when the key is absent
    /// (runtime.rs `default_true`), so serializing an explicit `true`
    /// is behavior-identical to the pre-w9-3 configs that omitted it.
    pub delegation_allowed: bool,
}

#[derive(Serialize)]
pub struct DelegationSection {
    pub allow_delegated_pull: bool,
    pub allowed_source_hosts: Vec<String>,
}

// ---------------------------------------------------------------
// Binary discovery + the once-per-binary daemon build.
// ---------------------------------------------------------------

fn bin_dir() -> PathBuf {
    let exe_path = std::env::current_exe().expect("current_exe");
    exe_path
        .parent()
        .expect("test binary directory")
        .parent()
        .expect("deps parent directory")
        .to_path_buf()
}

pub fn cli_bin() -> PathBuf {
    bin_dir().join(if cfg!(windows) { "blit.exe" } else { "blit" })
}

pub fn daemon_bin() -> PathBuf {
    bin_dir().join(if cfg!(windows) {
        "blit-daemon.exe"
    } else {
        "blit-daemon"
    })
}

/// Build `blit-daemon` at most once per test binary.
///
/// The build exists because `cargo test -p blit-cli` does not build
/// another package's binary; each test binary triggers it itself so
/// no test depends on suite ordering for the daemon to exist (R16-F1,
/// `docs/reviews/followup_review_2026-05-02.md`). Pre-w9-3 every
/// `TestContext::new()` ran its own nested `cargo build` (~75 per
/// full-suite run), all contending for cargo's build-dir lock — the
/// OnceLock keeps the per-process independence guarantee while paying
/// the subprocess cost once per binary
/// (tests-per-test-cargo-build-subprocess).
pub fn ensure_daemon_built() {
    static DAEMON_BUILT: OnceLock<()> = OnceLock::new();
    DAEMON_BUILT.get_or_init(|| {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root");
        let mut build = Command::new("cargo");
        build.current_dir(workspace_root);
        build
            .arg("build")
            .arg("-p")
            .arg("blit-daemon")
            .arg("--bin")
            .arg("blit-daemon");
        // Cross-target layout (target/<triple>/debug): the triple must
        // be passed through or the daemon lands in the wrong directory.
        let maybe_target = bin_dir()
            .parent()
            .and_then(|p| p.file_name())
            .map(|component| component.to_string_lossy().to_string());
        if let Some(triple) = maybe_target {
            if triple != "target" {
                build.arg("--target").arg(triple);
            }
        }
        let output = build.output().expect("invoke cargo build for blit-daemon");
        assert!(
            output.status.success(),
            "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

// ---------------------------------------------------------------
// Daemon spawn primitive + TestContext builder.
// ---------------------------------------------------------------

/// Pick a port no other test in THIS process has been handed.
///
/// The probe listener is dropped before the daemon binds, so the OS
/// can hand the same port to two parallel tests probing in the same
/// window — the losing daemon exits on "address in use" and its test
/// then talks to the *winner's* daemon (empty/wrong module, spurious
/// failures). Pre-w9-3 the per-test `cargo build` accidentally
/// serialized bring-ups and mostly hid this; the OnceLock build makes
/// parallel probes routine, so uniqueness is enforced with a
/// process-global claimed-set. Cargo runs test binaries sequentially,
/// so per-process uniqueness is exactly the needed scope; collisions
/// with unrelated system processes remain possible but are caught by
/// the child-death check in `spawn_daemon`.
/// Every port handed out in this process — by `pick_unused_port` AND
/// by the fake-server scaffold — goes through this one set, so a fake
/// server can never be assigned a port a daemon was promised (codex
/// review of f6e592e caught the fake-server path bypassing the set).
fn claim_port(port: u16) -> bool {
    static CLAIMED: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
    CLAIMED
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .expect("claimed-port set")
        .insert(port)
}

pub fn pick_unused_port() -> u16 {
    loop {
        let port = TcpListener::bind(("127.0.0.1", 0))
            .expect("bind probe listener")
            .local_addr()
            .expect("listener addr")
            .port();
        if claim_port(port) {
            return port;
        }
    }
}

/// Poll until something listens on `127.0.0.1:port` (50 × 100 ms).
///
/// Fake servers own their bound listener before this runs, so a TCP-level
/// readiness check is sufficient for them. Real daemon processes must use
/// [`wait_for_owned_readiness`], which proves application identity as well as a
/// listening socket.
pub fn wait_for_port(port: u16, label: &str) {
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("{label} failed to listen on {port}");
}

pub(crate) fn exported_modules_include_path<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    expected_path: &Path,
) -> bool {
    paths
        .into_iter()
        .any(|path| Path::new(path) == expected_path)
}

/// Wait for the spawned process to answer a positive identity probe.
///
/// A bare TCP connect can hit a foreign daemon during the probe-to-bind port
/// gap. The process can then lose its bind and exit after the harness has
/// already returned. Keeping the liveness check and the identity probe in one
/// loop prevents that false-ready state.
pub(crate) fn wait_for_owned_readiness(
    attempts: usize,
    pause: Duration,
    mut exited: impl FnMut() -> Result<Option<String>, String>,
    mut identity_matches: impl FnMut() -> bool,
) -> Result<(), String> {
    for attempt in 0..attempts {
        if let Some(status) = exited()? {
            return Err(format!("spawned daemon exited during startup ({status})"));
        }
        if identity_matches() {
            return Ok(());
        }
        if attempt + 1 < attempts {
            thread::sleep(pause);
        }
    }
    Err("spawned daemon did not answer its identity probe".to_string())
}

fn daemon_identity_matches(
    runtime: &tokio::runtime::Runtime,
    port: u16,
    expected_module_path: &Path,
) -> bool {
    // Avoid constructing a tonic runtime/probe until a listener exists. The
    // TCP check is only an optimization; the module-path match below is the
    // readiness proof.
    if TcpStream::connect(("127.0.0.1", port)).is_err() {
        return false;
    }

    let uri = format!("http://127.0.0.1:{port}");
    runtime.block_on(async {
        let endpoint = match Endpoint::from_shared(uri) {
            Ok(endpoint) => endpoint
                .connect_timeout(Duration::from_millis(250))
                .timeout(Duration::from_millis(250)),
            Err(_) => return false,
        };
        let channel =
            match tokio::time::timeout(Duration::from_millis(250), endpoint.connect()).await {
                Ok(Ok(channel)) => channel,
                _ => return false,
            };
        let response = match tokio::time::timeout(
            Duration::from_millis(250),
            BlitClient::new(channel).list_modules(ListModulesRequest {}),
        )
        .await
        {
            Ok(Ok(response)) => response.into_inner(),
            _ => return false,
        };

        exported_modules_include_path(
            response.modules.iter().map(|module| module.path.as_str()),
            expected_module_path,
        )
    })
}

/// Per-daemon knobs — everything the deleted harness clones existed
/// to express. (`delegation_allowed` on the module stays hardcoded
/// `true`, the daemon's own default; no test exercises `false` today
/// — add a knob here when one does.)
#[derive(Clone, Default)]
pub struct DaemonOptions {
    /// Export the module `read_only = true` (w9-4's three write gates).
    pub read_only: bool,
    /// Write a `[delegation]` table enabling delegated pull from
    /// loopback. Sources must be authorized by IP/CIDR form, not
    /// hostname — mirrors the production SSRF rule.
    pub delegation: bool,
    /// Extra daemon CLI flags (e.g. `--no-server-checksums`,
    /// `--force-grpc-data`).
    pub extra_args: Vec<String>,
}

/// One spawned daemon: its port, its module directory, and the child
/// guard that kills it on drop.
pub struct SpawnedDaemon {
    pub port: u16,
    pub module_dir: PathBuf,
    pub daemon: ChildGuard,
}

/// Spawn one daemon under `workspace`: writes `<name>.toml`, creates
/// `module_dir` if missing, picks a fresh port, waits for readiness.
/// `TestContext` routes through this; dual-daemon tests call it (via
/// `TestContext::spawn_second_daemon`) for their second daemon.
pub fn spawn_daemon(
    workspace: &Path,
    name: &str,
    module_dir: &Path,
    opts: &DaemonOptions,
) -> SpawnedDaemon {
    ensure_daemon_built();

    fs::create_dir_all(module_dir).expect("module dir");
    let port = pick_unused_port();

    let config = DaemonConfig {
        daemon: DaemonSection {
            bind: "127.0.0.1".into(),
            port,
            no_mdns: true,
        },
        modules: vec![ModuleSection {
            name: "test".into(),
            path: module_dir.to_path_buf(),
            comment: None,
            read_only: opts.read_only,
            delegation_allowed: true,
        }],
        delegation: opts.delegation.then(|| DelegationSection {
            allow_delegated_pull: true,
            // Loopback sources must be authorized by IP/CIDR form, not
            // hostname form. This mirrors the production SSRF rule.
            allowed_source_hosts: vec!["127.0.0.1".to_string()],
        }),
    };
    let config_path = workspace.join(format!("{name}.toml"));
    let toml = toml::to_string(&config).expect("serialize config");
    fs::write(&config_path, toml).expect("write config");

    let mut cmd = Command::new(daemon_bin());
    cmd.arg("--config")
        .arg(&config_path)
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string());
    for arg in &opts.extra_args {
        cmd.arg(arg);
    }
    // stderr policy: discard. The pre-w9-3 shared harness piped stderr
    // "for debugging" but nothing ever read it — zero diagnostics in
    // practice plus a latent pipe-buffer deadlock once a chatty daemon
    // wrote 64 KiB. Real capture (drain thread, dump on readiness
    // failure) is w9-6 (tests-harness-stderr-blackhole).
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    // The daemon canonicalizes exported roots before serving them. Matching
    // the unique temporary module path proves the RPC response came from this
    // child, rather than from any process that happened to own the port first.
    let expected_module_path = module_dir.canonicalize().expect("canonical module dir");
    let readiness_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("daemon readiness runtime");
    wait_for_owned_readiness(
        50,
        Duration::from_millis(100),
        || {
            child
                .try_wait()
                .map(|status| status.map(|status| status.to_string()))
                .map_err(|err| format!("poll spawned daemon: {err}"))
        },
        || daemon_identity_matches(&readiness_runtime, port, &expected_module_path),
    )
    .unwrap_or_else(|reason| {
        panic!(
            "daemon {name} was not ready on port {port}: {reason}; \
             port taken or config rejected"
        )
    });
    let daemon = ChildGuard::new(child);

    SpawnedDaemon {
        port,
        module_dir: module_dir.to_path_buf(),
        daemon,
    }
}

pub struct TestContext {
    pub _work: tempfile::TempDir,
    pub workspace: PathBuf,
    pub daemon_port: u16,
    pub daemon: ChildGuard,
    pub cli_bin: PathBuf,
    pub config_dir: PathBuf,
    pub module_dir: PathBuf,
}

/// Knob surface for the primary daemon; `build()` yields the context.
pub struct TestContextBuilder {
    opts: DaemonOptions,
}

impl TestContextBuilder {
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.opts.read_only = read_only;
        self
    }

    pub fn delegation(mut self, delegation: bool) -> Self {
        self.opts.delegation = delegation;
        self
    }

    pub fn extra_daemon_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.opts.extra_args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn build(self) -> TestContext {
        let work = tempdir().expect("tempdir");
        let workspace = work.path().to_path_buf();

        let config_dir = workspace.join("cli-config");
        fs::create_dir_all(&config_dir).expect("cli config");

        let spawned = spawn_daemon(&workspace, "blitd", &workspace.join("module"), &self.opts);

        TestContext {
            _work: work,
            workspace,
            daemon_port: spawned.port,
            daemon: spawned.daemon,
            cli_bin: cli_bin(),
            config_dir,
            module_dir: spawned.module_dir,
        }
    }
}

impl TestContext {
    pub fn builder() -> TestContextBuilder {
        TestContextBuilder {
            opts: DaemonOptions::default(),
        }
    }

    pub fn new() -> Self {
        Self::builder().build()
    }

    /// w9-4: same daemon + module, but the module is exported
    /// `read_only: true` so tests can exercise the three write gates
    /// (push, purge, delegated pull). Before this knob existed no
    /// test config in the workspace could express a read-only module.
    pub fn new_read_only() -> Self {
        Self::builder().read_only(true).build()
    }

    /// Spawn an additional daemon in this context's workspace with its
    /// own module dir (`module_<name>`) and config (`<name>.toml`).
    /// The dual-daemon delegation tests build on this.
    pub fn spawn_second_daemon(&self, name: &str, opts: &DaemonOptions) -> SpawnedDaemon {
        spawn_daemon(
            &self.workspace,
            name,
            &self.workspace.join(format!("module_{name}")),
            opts,
        )
    }
}

// ---------------------------------------------------------------
// Child-process plumbing shared across binaries.
// ---------------------------------------------------------------

pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
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

pub struct ChildGuard {
    pub child: Option<std::process::Child>,
}

impl ChildGuard {
    pub fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    /// Kill + reap now instead of at scope end — for tests that must
    /// assert on filesystem state after the daemon is gone.
    pub fn terminate(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

// ---------------------------------------------------------------
// In-process fake Blit gRPC servers (wire-shape tests).
// ---------------------------------------------------------------

/// A fake server running on its own thread + current_thread runtime;
/// Drop signals shutdown and joins the thread.
pub struct FakeServerGuard {
    pub port: u16,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for FakeServerGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Serve `svc` on an ephemeral loopback port, production-shaped: the
/// builder comes from `blit_core::remote::grpc_server`, so fakes carry
/// the same HTTP/2 keepalive config as a real daemon
/// (tests-fake-server-config-skew).
pub fn spawn_fake_blit_server<S>(svc: S, label: &str) -> FakeServerGuard
where
    S: blit_core::generated::blit_server::Blit,
{
    // Unlike the daemon path there is no probe-to-bind gap here — the
    // listener is kept and handed to tonic — but the OS-assigned port
    // must still go through the shared claimed-set: it could otherwise
    // be a port `pick_unused_port` already promised to a daemon whose
    // own bind is still pending (that daemon would then die on
    // "address in use" or, worse, its test would ready-check against
    // this fake). Loop until the OS hands us an unclaimed port.
    let listener = loop {
        let candidate = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake server");
        let port = candidate.local_addr().expect("fake server addr").port();
        if claim_port(port) {
            break candidate;
        }
    };
    let port = listener.local_addr().expect("fake server addr").port();
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

            let listener =
                tokio::net::TcpListener::from_std(listener).expect("tokio fake listener");
            blit_core::remote::grpc_server::production_server_builder()
                .add_service(BlitServer::new(svc))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("fake server");
        });
    });

    wait_for_port(port, label);
    FakeServerGuard {
        port,
        shutdown: Some(shutdown_tx),
        join: Some(join),
    }
}
