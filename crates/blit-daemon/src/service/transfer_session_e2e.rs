//! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
//! unified `Transfer` session and a real client initiates it as SOURCE
//! over gRPC. otp-4b makes the default carrier the **TCP data plane**
//! (the responder grants it in `SessionAccept`, the client dials +
//! authenticates + sends payloads over sockets); the in-stream carrier
//! stays live as the requested fallback. These tests pin the
//! push-equivalent behavior over both carriers:
//!
//! - a session lands bytes byte-identically and scores them correctly,
//!   over the data plane and over the in-stream fallback — with exact
//!   summary counts (the absolute form of the old A/B parity pins;
//!   the old-driver reference arms died at otp-10c-2);
//! - responder refusals (read-only module, unknown module) arrive as
//!   `SessionError` frames, surfaced to the client as faults;
//! - the unified SizeMtime semantic: a same-size destination file that
//!   is NEWER than the source is SKIPPED (the data-safe, pull-style
//!   converged behavior — see the finding doc's compare decision).
//!
//! otp-5a/5b add the pull-equivalent (roles flipped): the client initiates
//! as DESTINATION and the daemon streams its module tree as the SOURCE
//! Responder. otp-5b makes the default carrier the TCP data plane too — the
//! daemon (SOURCE responder) binds+grants+accepts sockets while sending and
//! the client (DESTINATION initiator) dials + receives — with the in-stream
//! carrier as the requested fallback. Those tests pin a byte-identical
//! landing over both carriers with exact summary counts (the absolute
//! form of the old A/B parity pins — the old drivers died at
//! otp-10c-2), proving the one served RPC handles both directions by
//! the declared role, not a second code path.
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
use blit_core::generated::{session_error, ComparisonMode};
use blit_core::remote::transfer::session_client::{
    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
};
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::remote::{RemoteEndpoint, RemotePath};
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
        Self::start_with(read_only, true).await
    }

    /// otp-10b-1: variant for a daemon whose operator disabled
    /// server-side checksum hashing (`--no-server-checksums`).
    async fn start_with_checksums_disabled() -> Self {
        Self::start_with(false, false).await
    }

    async fn start_with(read_only: bool, server_checksums_enabled: bool) -> Self {
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
        let service = BlitService::from_runtime(
            modules,
            None,
            false,
            server_checksums_enabled,
            crate::metrics::TransferMetrics::disabled(),
            crate::delegation_gate::DelegationConfig::default(),
        );
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

pub(crate) type FileSpec = (&'static str, &'static [u8], i64);

pub(crate) fn write_tree(root: &Path, files: &[FileSpec]) {
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

pub(crate) fn assert_trees_identical(a: &Path, b: &Path) {
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
async fn session_push_lands_identical_tree_with_exact_counts() {
    // otp-10c-2: this was the otp-4 A/B parity pin against the old
    // push driver. The reference arm died with the driver, so the pin
    // is now ABSOLUTE — byte-identical tree AND summary counts equal
    // to the fixture's own totals (exactly what the A/B equality
    // proved transitively; the committed otp-2/otp-2w baselines +
    // otp-12's interleaved old-binary runs carry the performance
    // half).
    let src = tempfile::tempdir().unwrap();
    let fixture = small_tree();
    write_tree(src.path(), &fixture);
    let expected_files = fixture.len() as u64;
    let expected_bytes: u64 = fixture.iter().map(|(_, data, _)| data.len() as u64).sum();

    let daemon = Daemon::start(false).await;
    let summary = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("session push succeeds");

    assert_trees_identical(src.path(), &daemon.dest_root);
    assert_eq!(summary.files_transferred, expected_files);
    assert_eq!(summary.bytes_transferred, expected_bytes);
    assert_eq!(summary.entries_deleted, 0);

    daemon.stop().await;
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

/// otp-10b-1: a served Checksum session content-compares — a
/// content-equal destination file skips despite a newer source mtime
/// (SizeMtime would transfer it), and the daemon DESTINATION hashes
/// its own candidates to decide.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn checksum_push_skips_content_equal_dest_over_served_session() {
    let daemon = Daemon::start(false).await;
    write_tree(
        &daemon.dest_root,
        &[("same.bin", b"identical-bytes", 1_000)],
    );
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &[("same.bin", b"identical-bytes", 2_000)]);

    let summary = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions {
            compare_mode: ComparisonMode::Checksum,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect("checksum session push succeeds");

    assert_eq!(
        summary.files_transferred, 0,
        "content-equal file must skip under Checksum despite the newer source mtime"
    );
    daemon.stop().await;
}

/// otp-10b-1: a daemon whose operator disabled server-side checksums
/// refuses a `COMPARISON_MODE_CHECKSUM` open with `CHECKSUM_DISABLED`
/// — never a silent degrade to a weaker compare — in BOTH roles (the
/// refusal is responder policy, not role logic).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn checksum_open_refused_when_daemon_disables_checksums() {
    let daemon = Daemon::start_with_checksums_disabled().await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &[("f.txt", b"x", 1_000)]);

    // Push-shaped (daemon = DESTINATION responder).
    let push_err = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions {
            compare_mode: ComparisonMode::Checksum,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect_err("checksum push against a no-checksum daemon must refuse");
    let fault = push_err
        .downcast_ref::<SessionFault>()
        .expect("refusal surfaces as a SessionFault");
    assert_eq!(fault.code, session_error::Code::ChecksumDisabled);
    assert!(
        fault.message.contains("checksum") && fault.message.contains("disabled"),
        "operator-facing reason names the knob, got: {}",
        fault.message
    );

    // Pull-shaped (daemon = SOURCE responder): same policy, same code.
    let dest = tempfile::tempdir().unwrap();
    let pull_err = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            compare_mode: ComparisonMode::Checksum,
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect_err("checksum pull against a no-checksum daemon must refuse");
    let fault = pull_err
        .downcast_ref::<SessionFault>()
        .expect("refusal surfaces as a SessionFault");
    assert_eq!(fault.code, session_error::Code::ChecksumDisabled);

    daemon.stop().await;
}

// ---------------------------------------------------------------------------
// otp-7b-2: cancel + fault identity during a data-plane resume
// ---------------------------------------------------------------------------

/// otp-7b-2 (codex otp-7a F4, deferred to 7b): a `CancelJob` fired while
/// the resume block phase is provably in progress over the TCP data
/// plane tears down cleanly — the client surfaces the peer's framed
/// CANCELLED (not the transport break), nothing hangs, and the daemon
/// drains the job row — exactly as otp-4b-3 pinned for file records.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mid_resume_cancel_surfaces_cancelled_over_the_data_plane() {
    const BS: usize = 64 * 1024;
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    let content = vec![0xABu8; 4 * 1024 * 1024];
    std::fs::write(src.path().join("big.bin"), &content).unwrap();
    filetime::set_file_mtime(
        src.path().join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_100, 0),
    )
    .unwrap();
    // An all-stale dest partial, so the file is resume-flagged and the
    // block phase starts sending immediately.
    std::fs::write(
        daemon.dest_root.join("big.bin"),
        vec![0x11u8; content.len()],
    )
    .unwrap();
    filetime::set_file_mtime(
        daemon.dest_root.join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_000, 0),
    )
    .unwrap();

    let started = Arc::new(tokio::sync::Notify::new());
    let source = Arc::new(StuckAfterFirstChunkSource {
        inner: FsTransferSource::new(src.path().to_path_buf()),
        started: Arc::clone(&started),
    });

    let ep = daemon.endpoint.clone();
    let client = tokio::spawn(async move {
        run_push_session(
            &ep,
            source,
            PushSessionOptions {
                resume: true,
                resume_block_size: BS as u32,
                ..PushSessionOptions::default()
            },
        )
        .await
    });

    // The resume block phase is provably in progress: the block-diff has
    // consumed the stuck reader's first chunk and can never finish.
    tokio::time::timeout(std::time::Duration::from_secs(10), started.notified())
        .await
        .expect("the resume block phase should start before cancel");

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
        "the served resume session's row honors cancellation"
    );

    let result = tokio::time::timeout(std::time::Duration::from_secs(10), client)
        .await
        .expect("client must not hang on a mid-resume cancel")
        .expect("client task joins");
    let err = result.expect_err("a cancelled resume transfer fails");
    assert_eq!(
        fault_of(&err).code,
        session_error::Code::Cancelled,
        "the client surfaces the peer's framed CANCELLED: {err:#}"
    );

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
        "the daemon must drain the cancelled resume job from active[]"
    );

    daemon.stop().await;
}

/// otp-7b-2 fault-injection source: the reader for one path yields only
/// the first `limit` bytes then EOF, provably short of the manifested
/// size — the mid-record fault D4 documents. Everything else delegates
/// to the real filesystem source.
struct TruncatedReadSource {
    inner: FsTransferSource,
    fail_path: &'static str,
    limit: u64,
}

#[async_trait::async_trait]
impl blit_core::remote::transfer::source::TransferSource for TruncatedReadSource {
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
        use tokio::io::AsyncReadExt;
        let reader = self.inner.open_file(header).await?;
        if header.relative_path == self.fail_path {
            Ok(Box::new(reader.take(self.limit)))
        } else {
            Ok(reader)
        }
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

/// otp-7b-2 (D-2026-07-09-1 Q2 rider): a source fault mid-resume over
/// the daemon-served data plane surfaces with STRUCTURED file identity,
/// and the end-of-operation summary the CLI will print (otp-10) names
/// the affected file and suggests a re-run to converge.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mid_resume_fault_names_the_file_in_the_end_of_operation_summary() {
    const BS: usize = 64 * 1024;
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    let content: Vec<u8> = (0..3 * BS).map(|i| (i % 251) as u8).collect();
    std::fs::write(src.path().join("big.bin"), &content).unwrap();
    filetime::set_file_mtime(
        src.path().join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_100, 0),
    )
    .unwrap();
    // All-stale partial: the source starts sending blocks immediately,
    // and its reader dies halfway through block 2.
    std::fs::write(
        daemon.dest_root.join("big.bin"),
        vec![0x11u8; content.len()],
    )
    .unwrap();
    filetime::set_file_mtime(
        daemon.dest_root.join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_000, 0),
    )
    .unwrap();

    let source = Arc::new(TruncatedReadSource {
        inner: FsTransferSource::new(src.path().to_path_buf()),
        fail_path: "big.bin",
        limit: (BS + BS / 2) as u64,
    });
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        run_push_session(
            &daemon.endpoint,
            source,
            PushSessionOptions {
                resume: true,
                resume_block_size: BS as u32,
                ..PushSessionOptions::default()
            },
        ),
    )
    .await
    .expect("a mid-resume fault must not hang")
    .expect_err("a truncated source must fault the session");

    let fault = fault_of(&err);
    assert_eq!(
        fault.relative_path.as_deref(),
        Some("big.bin"),
        "the fault carries structured file identity: {fault:?}"
    );
    let summary = fault
        .end_of_operation_summary()
        .expect("a file-naming fault yields the end-of-operation summary");
    assert!(
        summary.contains("big.bin") && summary.contains("re-run"),
        "the summary names the file and suggests a re-run: {summary}"
    );

    daemon.stop().await;
}

// ---------------------------------------------------------------------------
// otp-7b: resume over the TCP data plane, daemon-served both directions
// ---------------------------------------------------------------------------

/// otp-7b: a resume push over the daemon-served session rides the TCP
/// data plane — the destination partial is patched block-wise (binary
/// BLOCK/BLOCK_COMPLETE records on the sockets), only the stale blocks
/// move, and the summary counts the file resumed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn push_session_resumes_partial_over_the_data_plane() {
    const BS: usize = 64 * 1024; // == the session's block-size floor
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    let content: Vec<u8> = (0..6 * BS).map(|i| (i % 251) as u8).collect();
    std::fs::write(src.path().join("big.bin"), &content).unwrap();
    filetime::set_file_mtime(
        src.path().join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_100, 0),
    )
    .unwrap();
    // Dest partial: the first 4 blocks already landed, older mtime.
    std::fs::write(daemon.dest_root.join("big.bin"), &content[..4 * BS]).unwrap();
    filetime::set_file_mtime(
        daemon.dest_root.join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_000, 0),
    )
    .unwrap();

    let summary = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions {
            resume: true,
            resume_block_size: BS as u32,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect("resume session push succeeds");

    assert!(
        !summary.in_stream_carrier_used,
        "otp-7b resume rides the TCP data plane"
    );
    assert_eq!(summary.files_resumed, 1);
    assert_eq!(summary.files_transferred, 1);
    assert_eq!(
        summary.bytes_transferred,
        (2 * BS) as u64,
        "only the 2 missing blocks may move"
    );
    assert_trees_identical(src.path(), &daemon.dest_root);
    daemon.stop().await;
}

/// otp-7b, roles flipped: a resume pull — the daemon is the SOURCE
/// responder running the block-diff and sending block records over the
/// sockets it accepted; the client DESTINATION initiator hashes its
/// partial, dials, and applies the blocks.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_resumes_partial_over_the_data_plane() {
    const BS: usize = 64 * 1024;
    let daemon = Daemon::start(false).await;
    let content: Vec<u8> = (0..6 * BS).map(|i| (i % 251) as u8).collect();
    // The daemon module tree is the source.
    std::fs::write(daemon.dest_root.join("big.bin"), &content).unwrap();
    filetime::set_file_mtime(
        daemon.dest_root.join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_100, 0),
    )
    .unwrap();
    let dest = tempfile::tempdir().unwrap();
    std::fs::write(dest.path().join("big.bin"), &content[..4 * BS]).unwrap();
    filetime::set_file_mtime(
        dest.path().join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_000, 0),
    )
    .unwrap();

    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            resume: true,
            resume_block_size: BS as u32,
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect("resume session pull succeeds");

    assert!(
        !outcome.summary.in_stream_carrier_used,
        "otp-7b resume pull rides the TCP data plane"
    );
    assert_eq!(outcome.summary.files_resumed, 1);
    assert_eq!(outcome.summary.files_transferred, 1);
    assert_eq!(
        outcome.summary.bytes_transferred,
        (2 * BS) as u64,
        "only the 2 missing blocks may move"
    );
    assert_trees_identical(&daemon.dest_root, dest.path());
    daemon.stop().await;
}

// ---------------------------------------------------------------------------
// otp-8: the fallback byte-carrier's residue — resume over the REAL wire
// on the in-stream carrier. The in-process role suite exercises the same
// record grammar, but only a real tonic stream enforces the 4 MiB frame
// decode limit the in-stream block-size ceiling exists for
// (D-2026-07-10-1) — these pins put that ceiling where it can fail.
// ---------------------------------------------------------------------------

/// otp-8: a resume push forced onto the in-stream carrier still patches
/// the destination partial block-wise over the daemon-served RPC — the
/// same fixture as the data-plane twin above, so the two carriers are
/// pinned equivalent over the wire (same blocks move, same summary).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn push_session_resumes_partial_over_in_stream_carrier() {
    const BS: usize = 64 * 1024; // == the session's block-size floor
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    let content: Vec<u8> = (0..6 * BS).map(|i| (i % 251) as u8).collect();
    std::fs::write(src.path().join("big.bin"), &content).unwrap();
    filetime::set_file_mtime(
        src.path().join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_100, 0),
    )
    .unwrap();
    // Dest partial: the first 4 blocks already landed, older mtime.
    std::fs::write(daemon.dest_root.join("big.bin"), &content[..4 * BS]).unwrap();
    filetime::set_file_mtime(
        daemon.dest_root.join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_000, 0),
    )
    .unwrap();

    let summary = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions {
            in_stream_bytes: true,
            resume: true,
            resume_block_size: BS as u32,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect("in-stream resume session push succeeds");

    assert!(
        summary.in_stream_carrier_used,
        "an in_stream_bytes resume request rides the in-stream carrier"
    );
    assert_eq!(summary.files_resumed, 1);
    assert_eq!(summary.files_transferred, 1);
    assert_eq!(
        summary.bytes_transferred,
        (2 * BS) as u64,
        "only the 2 missing blocks may move"
    );
    assert_trees_identical(src.path(), &daemon.dest_root);
    daemon.stop().await;
}

/// otp-8, roles flipped, and the D-2026-07-10-1 clamp pinned over real
/// tonic: an OVERSIZED block-size request (8 MiB) on the in-stream
/// carrier must clamp to the carrier's 2 MiB ceiling. The fixture makes
/// the effective block size observable: a 6 MiB source, a same-size
/// dest copy with ONE corrupt byte at offset 3 MiB (older mtime). With
/// 2 MiB blocks exactly the middle block moves — `bytes_transferred`
/// == 2 MiB. An unclamped 8 MiB block would cover the whole file and
/// ship a single 6 MiB `BlockTransfer` frame, which tonic's default
/// 4 MiB decode limit rejects (the failure the ceiling exists to
/// prevent — unobservable on the in-process transport, which has no
/// frame limit); any other effective block size moves a different byte
/// count.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_resume_clamps_oversized_blocks_to_in_stream_ceiling() {
    const MIB: usize = 1024 * 1024;
    let daemon = Daemon::start(false).await;
    let content: Vec<u8> = (0..6 * MIB).map(|i| (i % 251) as u8).collect();
    // The daemon module tree is the source.
    std::fs::write(daemon.dest_root.join("big.bin"), &content).unwrap();
    filetime::set_file_mtime(
        daemon.dest_root.join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_100, 0),
    )
    .unwrap();
    let dest = tempfile::tempdir().unwrap();
    let mut stale = content.clone();
    stale[3 * MIB] ^= 0xFF;
    std::fs::write(dest.path().join("big.bin"), &stale).unwrap();
    filetime::set_file_mtime(
        dest.path().join("big.bin"),
        filetime::FileTime::from_unix_time(1_600_000_000, 0),
    )
    .unwrap();

    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            in_stream_bytes: true,
            resume: true,
            resume_block_size: (8 * MIB) as u32,
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect("in-stream resume session pull succeeds");

    assert!(
        outcome.summary.in_stream_carrier_used,
        "an in_stream_bytes resume request rides the in-stream carrier"
    );
    assert_eq!(outcome.summary.files_resumed, 1);
    assert_eq!(outcome.summary.files_transferred, 1);
    assert_eq!(
        outcome.summary.bytes_transferred,
        (2 * MIB) as u64,
        "the 8 MiB request must clamp to the 2 MiB in-stream ceiling: \
         exactly the one 2 MiB block holding the corrupt byte moves"
    );
    assert_trees_identical(&daemon.dest_root, dest.path());
    daemon.stop().await;
}

/// codex otp-8 F1: the mid-transfer cancel guard on the IN-STREAM
/// carrier. The data-plane twin above relies on the drain's
/// `recv_peer_fault` select arm; in-stream, the send half runs the
/// record sends inline, so without the fault race a cancel leaves the
/// client stuck in `reader.read()` forever (this test then fails its
/// no-hang timeout) — and a send that errors on the RPC teardown would
/// surface INTERNAL instead of the peer's framed CANCELLED.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mid_transfer_cancel_surfaces_cancelled_over_in_stream_carrier() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("big.bin"), vec![0xABu8; 4 * 1024 * 1024]).unwrap();

    let started = Arc::new(tokio::sync::Notify::new());
    let source = Arc::new(StuckAfterFirstChunkSource {
        inner: FsTransferSource::new(src.path().to_path_buf()),
        started: Arc::clone(&started),
    });

    let ep = daemon.endpoint.clone();
    let client = tokio::spawn(async move {
        run_push_session(
            &ep,
            source,
            PushSessionOptions {
                in_stream_bytes: true,
                ..PushSessionOptions::default()
            },
        )
        .await
    });

    tokio::time::timeout(std::time::Duration::from_secs(10), started.notified())
        .await
        .expect("payload bytes should flow in-stream before cancel");

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

    let result = tokio::time::timeout(std::time::Duration::from_secs(10), client)
        .await
        .expect("client must not hang on a mid-transfer cancel (in-stream)")
        .expect("client task joins");
    let err = result.expect_err("a cancelled transfer fails");
    assert_eq!(
        fault_of(&err).code,
        session_error::Code::Cancelled,
        "the client surfaces the peer's framed CANCELLED on the in-stream carrier: {err:#}"
    );

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
// otp-5a: pull-equivalent (client initiates as DESTINATION, daemon is SOURCE)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_lands_bytes_over_the_data_plane() {
    // Roles flipped: the daemon's MODULE tree is the SOURCE; the client
    // initiates as DESTINATION and the daemon streams its module tree. With
    // otp-5b the default carrier is the TCP data plane — the daemon (SOURCE
    // responder) binds+grants+accepts sockets while sending, and the client
    // (DESTINATION initiator) dials + receives over them. `dest_root` here
    // is the module (source) root — the harness field name is push-oriented.
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
        !outcome.summary.in_stream_carrier_used,
        "otp-5b pull default rides the TCP data plane, not the in-stream carrier"
    );
    assert_eq!(
        outcome.data_plane_streams,
        Some(1),
        "otp-5b-1 pull is single-stream (no resize until otp-5b-2)"
    );
    assert_trees_identical(&daemon.dest_root, dest.path());
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_lands_bytes_over_in_stream_carrier() {
    // The in-stream carrier is the pull fallback (diagnostics / unreachable
    // data plane). Requesting it must still land bytes byte-identically and
    // score them — the otp-5a path stays live under otp-5b.
    let daemon = Daemon::start(false).await;
    write_tree(&daemon.dest_root, &small_tree());

    let dest = tempfile::tempdir().unwrap();
    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            in_stream_bytes: true,
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect("in-stream session pull succeeds");

    assert_eq!(outcome.summary.files_transferred, small_tree().len() as u64);
    assert!(
        outcome.summary.in_stream_carrier_used,
        "an in_stream_bytes request rides the in-stream carrier"
    );
    assert_trees_identical(&daemon.dest_root, dest.path());
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn served_sessions_record_their_kind_and_endpoint() {
    // codex otp-10b-2 F4: post-cutover every verb rides `Transfer`, so
    // the jobs taxonomy must come from the open — a pull-shaped session
    // records PullSync (the old pull verbs' kind — CancelJob-capable,
    // wire TransferKind::PullSync) and a push-shaped one records Push,
    // both with the open's module, instead of the dispatch-time
    // Push/empty placeholders the pre-fix handler left in place.
    let daemon = Daemon::start(false).await;
    write_tree(&daemon.dest_root, &small_tree());

    let dest = tempfile::tempdir().unwrap();
    run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("pull session");

    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());
    run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("push session");

    // The rows drain (and their TransferRecords land on the recents
    // ring) when the daemon's spawned task drops its guard — bounded
    // wait, the client RPCs have already returned.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let recent = loop {
        let recent = daemon.active_jobs.recent();
        if recent.len() >= 2 || std::time::Instant::now() > deadline {
            break recent;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };

    let pull = recent
        .iter()
        .find(|r| r.kind == crate::active_jobs::ActiveJobKind::PullSync)
        .expect("the served pull must record kind PullSync");
    assert_eq!(pull.module, "test", "pull row carries the open's module");
    let push = recent
        .iter()
        .find(|r| r.kind == crate::active_jobs::ActiveJobKind::Push)
        .expect("the served push must record kind Push");
    assert_eq!(push.module, "test", "push row carries the open's module");
    daemon.stop().await;
}

// ---------------------------------------------------------------------------
// otp-9a: the pull session-client surface the delegated reroute (otp-9b)
// consumes — mirror + filter through PullSessionOptions, and the caller's
// live byte counter. The session has honored mirror/filter since otp-6;
// these pin the CLIENT wiring over a daemon-served RPC.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_mirror_purges_extraneous_via_client_options() {
    let daemon = Daemon::start(false).await;
    write_tree(&daemon.dest_root, &small_tree());

    let dest = tempfile::tempdir().unwrap();
    std::fs::write(dest.path().join("stale.bin"), b"extraneous").unwrap();

    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            mirror_enabled: true,
            mirror_kind: blit_core::generated::MirrorMode::All,
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect("mirror session pull succeeds");

    assert!(
        !dest.path().join("stale.bin").exists(),
        "mirror ALL purges the extraneous destination file (one delete rule)"
    );
    assert_eq!(
        outcome.summary.entries_deleted, 1,
        "the purge is scored on the summary"
    );
    assert_trees_identical(&daemon.dest_root, dest.path());
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_filter_limits_manifest_via_client_options() {
    let daemon = Daemon::start(false).await;
    write_tree(
        &daemon.dest_root,
        &[
            ("keep.txt", b"a" as &[u8], 1_600_000_001),
            ("drop.log", b"b", 1_600_000_002),
        ],
    );

    let dest = tempfile::tempdir().unwrap();
    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            filter: Some(blit_core::generated::FilterSpec {
                include: vec!["*.txt".to_string()],
                ..Default::default()
            }),
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect("filtered session pull succeeds");

    assert_eq!(outcome.summary.files_transferred, 1);
    assert!(dest.path().join("keep.txt").exists());
    assert!(
        !dest.path().join("drop.log").exists(),
        "the include filter rides the open and scopes the remote scan"
    );
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_session_reports_bytes_against_the_callers_counter() {
    use std::sync::atomic::{AtomicU64, Ordering};

    let daemon = Daemon::start(false).await;
    write_tree(&daemon.dest_root, &small_tree());

    let counter = Arc::new(AtomicU64::new(0));
    let dest = tempfile::tempdir().unwrap();
    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            byte_progress: Some(blit_core::remote::transfer::ByteProgressSink::from_counter(
                Arc::clone(&counter),
            )),
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect("session pull succeeds");

    let counted = counter.load(Ordering::Relaxed);
    assert!(counted > 0, "the caller's live counter saw bytes land");
    assert_eq!(
        counted, outcome.summary.bytes_transferred,
        "the counter and the summary agree on applied payload bytes"
    );
    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_pull_lands_identical_tree_with_exact_counts() {
    // otp-10c-2: this was the otp-5 A/B parity pin against the old
    // pull driver — converted to an ABSOLUTE pin the same way as the
    // push twin above (the reference arm died with the driver).
    let daemon = Daemon::start(false).await;
    let fixture = small_tree();
    write_tree(&daemon.dest_root, &fixture);
    let expected_files = fixture.len() as u64;
    let expected_bytes: u64 = fixture.iter().map(|(_, data, _)| data.len() as u64).sum();

    let dest = tempfile::tempdir().unwrap();
    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("session pull succeeds");

    assert_trees_identical(&daemon.dest_root, dest.path());
    assert_eq!(outcome.summary.files_transferred, expected_files);
    assert_eq!(outcome.summary.bytes_transferred, expected_bytes);

    daemon.stop().await;
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
