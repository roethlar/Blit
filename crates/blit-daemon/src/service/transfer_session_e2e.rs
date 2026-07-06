//! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
//! unified `Transfer` session and a real client initiates it as SOURCE
//! over gRPC. otp-4b makes the default carrier the **TCP data plane**
//! (the responder grants it in `SessionAccept`, the client dials +
//! authenticates + sends payloads over sockets); the in-stream carrier
//! stays live as the requested fallback. These tests pin the
//! push-equivalent behavior over both carriers:
//!
//! - a session lands bytes byte-identically and scores them correctly,
//!   over the data plane and over the in-stream fallback;
//! - **A/B parity**: the same fixture through OLD push and the NEW
//!   session (data plane) yields byte-identical destination trees +
//!   equal shared summary counters (the converge-up bar);
//! - responder refusals (read-only module, unknown module) arrive as
//!   `SessionError` frames, surfaced to the client as faults;
//! - the unified SizeMtime semantic: a same-size destination file that
//!   is NEWER than the source is SKIPPED (the data-safe, pull-style
//!   converged behavior — see the finding doc's compare decision).
//!
//! otp-5a adds the pull-equivalent (roles flipped): the client initiates
//! as DESTINATION and the daemon streams its module tree as the SOURCE
//! Responder over the in-stream carrier. Those tests pin a byte-identical
//! landing + A/B parity vs old `pull_sync`, proving the one served RPC
//! handles both directions by the declared role, not a second code path.
//!
//! Harness mirrors `push/shape_resize_e2e.rs`: a real in-process
//! `BlitService` on loopback + a real client. Only in-crate tests can
//! build `ModuleConfig`/`BlitService::with_modules`, so this lives in
//! blit-daemon.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use blit_core::fs_enum::FileFilter;
use blit_core::generated::blit_server::BlitServer;
use blit_core::generated::{session_error, MirrorMode};
use blit_core::remote::pull::PullSyncOptions;
use blit_core::remote::transfer::session_client::{
    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
};
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePullClient, RemotePushClient};
use blit_core::transfer_session::SessionFault;
use tokio::sync::oneshot;

use crate::runtime::ModuleConfig;
use crate::service::BlitService;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// A running in-process daemon exposing module "test" over a writable
/// (or read-only) temp dir, and the loopback endpoint targeting it.
struct Daemon {
    endpoint: RemoteEndpoint,
    shutdown: Option<oneshot::Sender<()>>,
    server: Option<tokio::task::JoinHandle<()>>,
    _dest: tempfile::TempDir,
    dest_root: PathBuf,
    active_jobs: crate::active_jobs::ActiveJobs,
}

impl Daemon {
    async fn start(read_only: bool) -> Self {
        let dest = tempfile::tempdir().expect("dest dir");
        let canonical = dest.path().canonicalize().expect("canonical dest");
        let mut modules = HashMap::new();
        modules.insert(
            "test".to_string(),
            ModuleConfig {
                name: "test".into(),
                path: canonical.clone(),
                canonical_root: canonical.clone(),
                read_only,
                _comment: None,
                delegation_allowed: true,
            },
        );
        let service = BlitService::with_modules(modules, false);
        let active_jobs = service.active_jobs.clone();
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind loopback listener");
        let port = listener.local_addr().expect("listener addr").port();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            blit_core::remote::grpc_server::production_server_builder()
                .add_service(BlitServer::new(service))
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = shutdown_rx.await;
                    },
                )
                .await
                .expect("in-process daemon serves");
        });
        let endpoint = RemoteEndpoint {
            host: "127.0.0.1".into(),
            port,
            path: RemotePath::Module {
                module: "test".into(),
                rel_path: PathBuf::new(),
            },
        };
        Daemon {
            endpoint,
            shutdown: Some(shutdown_tx),
            server: Some(server),
            _dest: dest,
            dest_root: canonical,
            active_jobs,
        }
    }

    /// Endpoint pointing at a module name that isn't configured.
    fn endpoint_for_missing_module(&self) -> RemoteEndpoint {
        RemoteEndpoint {
            host: self.endpoint.host.clone(),
            port: self.endpoint.port,
            path: RemotePath::Module {
                module: "nope".into(),
                rel_path: PathBuf::new(),
            },
        }
    }

    async fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(server) = self.server.take() {
            server.await.expect("server task joins");
        }
    }
}

type FileSpec = (&'static str, &'static [u8], i64);

fn write_tree(root: &Path, files: &[FileSpec]) {
    for (rel, content, mtime) in files {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
    }
}

/// rel-path → bytes for every regular file under `root`. Content only
/// (byte-identical), copied from the role suite — no shared test util
/// exists across crates yet.
fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        walk(root, root, &mut out);
    }
    out
}

fn assert_trees_identical(a: &Path, b: &Path) {
    let ta = collect_tree(a);
    let tb = collect_tree(b);
    assert_eq!(
        ta.keys().collect::<Vec<_>>(),
        tb.keys().collect::<Vec<_>>(),
        "path sets differ between {a:?} and {b:?}"
    );
    for (rel, bytes) in &ta {
        assert_eq!(bytes, &tb[rel], "content differs for '{rel}'");
    }
}

fn small_tree() -> Vec<FileSpec> {
    vec![
        ("a.txt", b"alpha", 1_600_000_001),
        ("empty.bin", b"", 1_600_000_002),
        ("dir one/b.log", b"beta beta beta", 1_600_000_003),
        ("dir one/deeper/c.dat", b"gamma-content", 1_600_000_004),
    ]
}

fn fault_of(err: &eyre::Report) -> &SessionFault {
    err.downcast_ref::<SessionFault>()
        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
}

// --- otp-4b-3: deterministic mid-transfer cancel over the data plane ---

/// A `TransferSource` that puts a transfer into a provably-stuck
/// mid-payload state: `open_file` writes exactly one 64 KiB chunk over
/// the data plane (so bytes have demonstrably flowed), signals `started`,
/// then blocks forever without emitting the rest of the file. The
/// transfer therefore cannot complete on its own — the only exits are the
/// cancel under test or the reader being dropped when the session aborts.
/// Everything else delegates to the real filesystem source.
struct StuckAfterFirstChunkSource {
    inner: FsTransferSource,
    started: Arc<tokio::sync::Notify>,
}

#[async_trait::async_trait]
impl blit_core::remote::transfer::source::TransferSource for StuckAfterFirstChunkSource {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
    ) -> (
        tokio::sync::mpsc::Receiver<blit_core::generated::FileHeader>,
        tokio::task::JoinHandle<eyre::Result<u64>>,
    ) {
        self.inner.scan(filter, unreadable)
    }

    async fn prepare_payload(
        &self,
        payload: blit_core::remote::transfer::payload::TransferPayload,
    ) -> eyre::Result<blit_core::remote::transfer::payload::PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<blit_core::generated::FileHeader>,
        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
    ) -> eyre::Result<Vec<blit_core::generated::FileHeader>> {
        self.inner.check_availability(headers, unreadable).await
    }

    async fn open_file(
        &self,
        header: &blit_core::generated::FileHeader,
    ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        let mut inner = self.inner.open_file(header).await?;
        // Small duplex buffer (< one chunk) so `write_all` of the chunk
        // only completes once the data-plane send pipeline has DRAINED it
        // out to the TCP socket — i.e. `started` fires after payload bytes
        // have actually flowed over the data plane, not merely into a
        // local buffer (codex otp-4b-3 F2).
        let (mut w, r) = tokio::io::duplex(4 * 1024);
        let started = Arc::clone(&self.started);
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0u8; 64 * 1024];
            if let Ok(n) = inner.read(&mut buf).await {
                if n > 0 && w.write_all(&buf[..n]).await.is_ok() {
                    started.notify_one();
                }
            }
            // Hold the write half open (no EOF) and never write again:
            // the transfer is now stuck mid-payload until the session is
            // aborted (which drops this task) or cancelled.
            std::future::pending::<()>().await;
            drop(w);
        });
        Ok(Box::new(r))
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
/// exactly what the RPC handler fires) while a payload is stuck mid-flight
/// over the TCP data plane. The client must surface
/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
/// than the data-plane transport break it also causes, and it must not
/// hang. The daemon must then tear the job down cleanly (the active row
/// drains).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mid_transfer_cancel_surfaces_cancelled_over_the_data_plane() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    // One file larger than a single chunk, so the stuck reader keeps the
    // transfer provably incomplete after its first 64 KiB.
    std::fs::write(src.path().join("big.bin"), vec![0xABu8; 4 * 1024 * 1024]).unwrap();

    let started = Arc::new(tokio::sync::Notify::new());
    let source = Arc::new(StuckAfterFirstChunkSource {
        inner: FsTransferSource::new(src.path().to_path_buf()),
        started: Arc::clone(&started),
    });

    let ep = daemon.endpoint.clone();
    let client =
        tokio::spawn(
            async move { run_push_session(&ep, source, PushSessionOptions::default()).await },
        );

    // Bytes have flowed over the data plane and the transfer is now stuck
    // mid-payload — a deterministic mid-transfer point.
    tokio::time::timeout(std::time::Duration::from_secs(10), started.notified())
        .await
        .expect("payload bytes should flow over the data plane before cancel");

    // Fire the row's cancellation token — exactly what the `CancelJob` RPC
    // handler does via `cancel_authorized` (audit-9). The RPC-level
    // mapping (auth, outcome codes) is unit-tested separately; this pins
    // the end-to-end propagation through the served session.
    let transfer_id = daemon
        .active_jobs
        .snapshot()
        .into_iter()
        .next()
        .expect("an active transfer row")
        .transfer_id;
    assert_eq!(
        daemon.active_jobs.cancel(&transfer_id),
        crate::active_jobs::CancelOutcome::Cancelled,
        "the served session's row honors cancellation"
    );

    // The client must surface CANCELLED promptly (no hang).
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), client)
        .await
        .expect("client must not hang on a mid-transfer cancel")
        .expect("client task joins");
    let err = result.expect_err("a cancelled transfer fails");
    assert_eq!(
        fault_of(&err).code,
        session_error::Code::Cancelled,
        "the client surfaces the peer's framed CANCELLED, not the data-plane break: {err:#}"
    );

    // Daemon tears down cleanly: the active row drains.
    let mut drained = false;
    for _ in 0..200 {
        if daemon.active_jobs.snapshot().is_empty() {
            drained = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        drained,
        "the daemon must drain the cancelled job from active[]"
    );

    daemon.stop().await;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_lands_bytes_over_the_data_plane() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());

    // Default options ⇒ TCP data plane: the responder grants it and the
    // client dials + sends payloads over sockets (otp-4b).
    let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
    let summary = run_push_session(&daemon.endpoint, source, PushSessionOptions::default())
        .await
        .expect("session push succeeds");

    assert_eq!(summary.files_transferred, small_tree().len() as u64);
    assert_eq!(
        summary.bytes_transferred,
        small_tree()
            .iter()
            .map(|(_, c, _)| c.len() as u64)
            .sum::<u64>()
    );
    assert!(
        !summary.in_stream_carrier_used,
        "otp-4b default rides the TCP data plane, not the in-stream carrier"
    );
    assert_trees_identical(src.path(), &daemon.dest_root);
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_lands_bytes_over_in_stream_carrier() {
    // The in-stream carrier is the fallback (diagnostics / unreachable
    // data plane). Requesting it must still land bytes byte-identically
    // and score them — the otp-4a path stays live under otp-4b.
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());

    let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
    let summary = run_push_session(
        &daemon.endpoint,
        source,
        PushSessionOptions {
            in_stream_bytes: true,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect("in-stream session push succeeds");

    assert_eq!(summary.files_transferred, small_tree().len() as u64);
    assert!(
        summary.in_stream_carrier_used,
        "an in_stream_bytes request rides the in-stream carrier"
    );
    assert_trees_identical(src.path(), &daemon.dest_root);
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn old_push_and_session_produce_identical_trees_and_counts() {
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());

    // Arm A: OLD push.
    let daemon_a = Daemon::start(false).await;
    let mut push_client = RemotePushClient::connect(daemon_a.endpoint.clone())
        .await
        .expect("old push client connects");
    let report = push_client
        .push(
            Arc::new(FsTransferSource::new(src.path().to_path_buf())),
            &FileFilter::default(),
            false,
            MirrorMode::FilteredSubset,
            false,
            false,
            None,
            false,
        )
        .await
        .expect("old push succeeds");

    // Arm B: NEW session.
    let daemon_b = Daemon::start(false).await;
    let summary = run_push_session(
        &daemon_b.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("session push succeeds");

    // Both destinations equal the source and each other.
    assert_trees_identical(src.path(), &daemon_a.dest_root);
    assert_trees_identical(src.path(), &daemon_b.dest_root);
    assert_trees_identical(&daemon_a.dest_root, &daemon_b.dest_root);

    // Shared summary counters agree (transport-specific fields —
    // tcp_fallback_used/bytes_zero_copy vs in_stream_carrier_used — have
    // no cross analog and are not compared).
    assert_eq!(report.summary.files_transferred, summary.files_transferred);
    assert_eq!(report.summary.bytes_transferred, summary.bytes_transferred);
    assert_eq!(report.summary.entries_deleted, summary.entries_deleted);

    daemon_a.stop().await;
    daemon_b.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_only_module_refuses_the_session() {
    let daemon = Daemon::start(true).await; // read-only
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &[("a.txt", b"alpha", 1_600_000_001)]);

    let err = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect_err("read-only module must refuse the session");
    assert_eq!(fault_of(&err).code, session_error::Code::ReadOnly);
    assert!(
        collect_tree(&daemon.dest_root).is_empty(),
        "no bytes may land on a refused session"
    );
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unknown_module_refuses_the_session() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &[("a.txt", b"alpha", 1_600_000_001)]);

    let err = run_push_session(
        &daemon.endpoint_for_missing_module(),
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect_err("unknown module must refuse the session");
    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn same_size_newer_destination_is_skipped_not_clobbered() {
    // The unified SizeMtime decision (finding doc compare section): the
    // sole push/pull divergence is same-size + dest-NEWER. The session
    // adopts the data-safe, converge-up behavior — SKIP, never clobber
    // a newer destination file with older source content. (--force
    // overrides; not exercised here.)
    let daemon = Daemon::start(false).await;

    // Seed the destination with a NEWER, same-size, different-content
    // file plus a file that genuinely needs updating.
    write_tree(
        &daemon.dest_root,
        &[
            ("keep.txt", b"NEWER-destination", 1_600_100_000),
            ("stale.txt", b"old-destination--", 1_600_000_000),
        ],
    );
    let src = tempfile::tempdir().unwrap();
    write_tree(
        src.path(),
        &[
            // same size (17) as dest keep.txt, but OLDER → must be skipped.
            ("keep.txt", b"older-source-here", 1_600_000_000),
            // same size (17) as dest stale.txt, but NEWER → must transfer.
            ("stale.txt", b"new-source-here--", 1_600_200_000),
        ],
    );

    let summary = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("session push succeeds");

    // Only stale.txt transfers; keep.txt (newer on dest) is left intact.
    assert_eq!(
        summary.files_transferred, 1,
        "only the stale file transfers"
    );
    assert_eq!(
        std::fs::read(daemon.dest_root.join("keep.txt")).unwrap(),
        b"NEWER-destination",
        "a newer same-size destination file must NOT be clobbered"
    );
    assert_eq!(
        std::fs::read(daemon.dest_root.join("stale.txt")).unwrap(),
        b"new-source-here--",
        "a stale destination file must be updated"
    );
    daemon.stop().await;
}

// ---------------------------------------------------------------------------
// otp-5a: pull-equivalent (client initiates as DESTINATION, daemon is SOURCE)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_lands_bytes_and_scores_them() {
    // Roles flipped: the daemon's MODULE tree is the SOURCE; the client
    // initiates as DESTINATION and the daemon streams its module tree
    // (otp-5a). The SOURCE responder grants no data plane, so the carrier
    // is the in-stream fallback. `dest_root` here is the module (source)
    // root — the harness field name is push-oriented.
    let daemon = Daemon::start(false).await;
    write_tree(&daemon.dest_root, &small_tree());

    let dest = tempfile::tempdir().unwrap();
    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("session pull succeeds");

    assert_eq!(outcome.summary.files_transferred, small_tree().len() as u64);
    assert_eq!(
        outcome.summary.bytes_transferred,
        small_tree()
            .iter()
            .map(|(_, c, _)| c.len() as u64)
            .sum::<u64>()
    );
    assert!(
        outcome.summary.in_stream_carrier_used,
        "otp-5a pull rides the in-stream carrier (no SOURCE-responder data plane yet)"
    );
    assert_trees_identical(&daemon.dest_root, dest.path());
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn old_pull_and_session_produce_identical_trees_and_counts() {
    // Arm A: OLD pull_sync into a client-local dest.
    let daemon_a = Daemon::start(false).await;
    write_tree(&daemon_a.dest_root, &small_tree());
    let dest_a = tempfile::tempdir().unwrap();
    let mut pull_client = RemotePullClient::connect(daemon_a.endpoint.clone())
        .await
        .expect("old pull client connects");
    let report = pull_client
        .pull_sync(
            dest_a.path(),
            Vec::new(),
            &PullSyncOptions::default(),
            false,
            None,
        )
        .await
        .expect("old pull succeeds");

    // Arm B: NEW session (client DESTINATION initiator).
    let daemon_b = Daemon::start(false).await;
    write_tree(&daemon_b.dest_root, &small_tree());
    let dest_b = tempfile::tempdir().unwrap();
    let outcome = run_pull_session(
        &daemon_b.endpoint,
        dest_b.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("session pull succeeds");

    // Both dests equal their source module and each other.
    assert_trees_identical(&daemon_a.dest_root, dest_a.path());
    assert_trees_identical(&daemon_b.dest_root, dest_b.path());
    assert_trees_identical(dest_a.path(), dest_b.path());

    // Shared counters agree (transport-specific fields have no cross
    // analog and are not compared). Old pull already SKIPs the same-size
    // dest-NEWER cell, so this A/B is byte-identical with no caveat —
    // unlike the push A/B where old push clobbers.
    assert_eq!(
        report.files_transferred as u64,
        outcome.summary.files_transferred
    );
    assert_eq!(report.bytes_transferred, outcome.summary.bytes_transferred);

    daemon_a.stop().await;
    daemon_b.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unknown_module_refuses_the_pull_session() {
    let daemon = Daemon::start(false).await;
    let dest = tempfile::tempdir().unwrap();
    let err = run_pull_session(
        &daemon.endpoint_for_missing_module(),
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect_err("unknown module must refuse the pull session");
    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);
    daemon.stop().await;
}
