//! ONE_TRANSFER_PATH otp-4a loopback e2e: the daemon serves the unified
//! `Transfer` session and a real client initiates it as SOURCE over
//! gRPC (in-stream carrier). These tests replace the otp-1 UNIMPLEMENTED
//! pin — the RPC now serves — and pin the push-equivalent behavior:
//!
//! - a session lands bytes byte-identically and scores them correctly;
//! - **A/B parity**: the same fixture through OLD push and the NEW
//!   session yields byte-identical destination trees + equal shared
//!   summary counters (the converge-up bar, in-stream);
//! - responder refusals (read-only module, unknown module) arrive as
//!   `SessionError` frames, surfaced to the client as faults;
//! - the unified SizeMtime semantic: a same-size destination file that
//!   is NEWER than the source is SKIPPED (the data-safe, pull-style
//!   converged behavior — see the finding doc's compare decision).
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
use blit_core::remote::transfer::session_client::{run_push_session, PushSessionOptions};
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_lands_bytes_and_scores_them() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());

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
        summary.in_stream_carrier_used,
        "otp-4a rides the in-stream carrier"
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
