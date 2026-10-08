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
//! Harness: a real in-process `BlitService` on loopback plus a real session
//! client, with both semantic role orientations exercised here. Only in-crate tests can
//! build `ModuleConfig`/`BlitService::with_modules`, so this lives in
//! blit-daemon.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use blit_core::fs_enum::FileFilter;
use blit_core::generated::blit_server::BlitServer;
use blit_core::generated::{session_error, ComparisonMode};
use blit_core::remote::transfer::session_client::{
    connect_transfer_client_with_trace, run_pull_session, run_push_session, PullSessionOptions,
    PushSessionOptions,
};
use blit_core::remote::transfer::source::{FsTransferSource, OpenedSourceFile};
use blit_core::remote::transfer::{
    SessionPhaseRole, TransferLifecycleEvent, TransferLifecycleOutcome, TransferLifecycleTrace,
};
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
    /// ph-1c: this daemon's own isolated history store; the recording
    /// matrix asserts what the served end wrote here.
    perf_dir: tempfile::TempDir,
    /// jl-1b: this daemon's state dir; every served job logs here.
    _job_state: tempfile::TempDir,
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
        let perf_dir = tempfile::tempdir().expect("perf dir");
        let job_state = tempfile::tempdir().expect("job log state dir");
        let service = BlitService::from_runtime(
            modules,
            None,
            false,
            server_checksums_enabled,
            crate::metrics::TransferMetrics::disabled(),
            crate::delegation_gate::DelegationConfig::default(),
            Some(blit_core::perf_history::HistoryStore::at_dir(
                perf_dir.path().to_path_buf(),
            )),
        )
        .with_job_logs(Some(
            crate::job_logs::JobLogs::open(job_state.path(), blit_core::job_log::DEFAULT_KEEP)
                .expect("job logs"),
        ));
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
            perf_dir,
            _job_state: job_state,
        }
    }

    /// jl-1b: every log this daemon kept for its one finished job, read
    /// back over `GetJobLog` as `(role, finished, events)`. Waits for the
    /// dispatcher to close the log, which it does after the client already
    /// holds its summary.
    async fn job_logs(&self) -> Vec<(String, bool, Vec<blit_core::job_log::EventBody>)> {
        let mut transfer_id = None;
        for _ in 0..500 {
            if let Some(record) = self.active_jobs.recent().first() {
                transfer_id = Some(record.transfer_id.clone());
            }
            if let Some(id) = &transfer_id {
                let mut logs = Vec::new();
                let fetched = Arc::new(Mutex::new(Vec::new()));
                let sink = Arc::clone(&fetched);
                let read = blit_core::admin::jobs::read_job_logs(
                    &self.endpoint,
                    id,
                    None,
                    move |header, lines| {
                        let mut events = Vec::new();
                        for line in blit_core::job_log::LogLines::new(lines) {
                            match line? {
                                blit_core::job_log::LogLine::Event(event) => {
                                    events.push(event.body)
                                }
                                other => eyre::bail!("unreadable log line: {other:?}"),
                            }
                        }
                        sink.lock()
                            .unwrap()
                            .push((header.role, header.finished, events));
                        Ok(())
                    },
                )
                .await;
                if read.is_ok() {
                    logs.append(&mut fetched.lock().unwrap());
                    if !logs.is_empty() && logs.iter().all(|(_, finished, _)| *finished) {
                        return logs;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("the job's log never finished (job {transfer_id:?})");
    }

    /// The records this daemon's own store holds, oldest first.
    fn perf_records(&self) -> Vec<blit_core::perf_history::PerformanceRecord> {
        blit_core::perf_history::HistoryStore::at_dir(self.perf_dir.path().to_path_buf())
            .read_recent_records(0)
            .expect("read daemon perf store")
    }

    /// Wait for the daemon's own append to land: the responder sends
    /// the client its summary BEFORE `run_transfer_session` returns and
    /// records, so the client observing completion does not order the
    /// daemon-side write.
    async fn wait_for_perf_records(
        &self,
        n: usize,
    ) -> Vec<blit_core::perf_history::PerformanceRecord> {
        for _ in 0..200 {
            let records = self.perf_records();
            if records.len() >= n {
                return records;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!(
            "daemon store never reached {n} record(s): {:?}",
            self.perf_records()
        );
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

fn lifecycle_capture(
    run_id: &str,
) -> (
    TransferLifecycleTrace,
    Arc<Mutex<Vec<TransferLifecycleEvent>>>,
) {
    let events: Arc<Mutex<Vec<TransferLifecycleEvent>>> = Arc::default();
    let captured = Arc::clone(&events);
    (
        TransferLifecycleTrace::capture(run_id, move |event| {
            captured.lock().expect("lifecycle capture lock").push(event);
        }),
        events,
    )
}

fn assert_success_lifecycle(events: &[TransferLifecycleEvent], role: SessionPhaseRole) {
    let names = events.iter().map(|event| event.event).collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "control_connect_begin",
            "control_connect_end",
            "transfer_rpc_open_begin",
            "transfer_rpc_open_end",
            "session_establish_begin",
            "session_establish_end",
            "session_body_return",
        ]
    );
    assert_eq!(
        events
            .iter()
            .map(|event| event.producer_seq)
            .collect::<Vec<_>>(),
        (0..events.len() as u64).collect::<Vec<_>>()
    );
    assert!(events
        .iter()
        .all(|event| event.initiator_role == Some(role)));
    assert!(events[..5].iter().all(|event| event.session_id.is_none()));
    assert!(events[5..]
        .iter()
        .all(|event| event.session_id.as_deref().is_some_and(|id| !id.is_empty())));
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.outcome)
            .collect::<Vec<_>>(),
        vec![
            TransferLifecycleOutcome::Success,
            TransferLifecycleOutcome::Success,
            TransferLifecycleOutcome::Success,
            TransferLifecycleOutcome::Success,
        ]
    );
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
        blit_core::remote::transfer::source::SourceScan,
    ) {
        self.inner.scan(filter, unreadable)
    }

    async fn prepare_payload(
        &self,
        payload: blit_core::remote::transfer::payload::TransferPayload,
    ) -> eyre::Result<blit_core::remote::transfer::payload::PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn open_file(
        &self,
        header: &blit_core::generated::FileHeader,
    ) -> eyre::Result<OpenedSourceFile> {
        let mut inner = self.inner.open_file(header).await?;
        // Small duplex buffer (< one chunk) so `write_all` of the chunk
        // only completes once the data-plane send pipeline has DRAINED it
        // out to the TCP socket — i.e. `started` fires after payload bytes
        // have actually flowed over the data plane, not merely into a
        // local buffer (review otp-4b-3 F2).
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
        Ok(OpenedSourceFile::virtual_reader(Box::new(r), header.size))
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
async fn lifecycle_boundaries_are_role_symmetric_and_session_correlated() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &[("a.txt", b"alpha", 1_600_000_001)]);

    let (push_trace, push_events) = lifecycle_capture("push-lifecycle");
    run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions {
            lifecycle_trace: push_trace,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect("traced push succeeds");

    let dest = tempfile::tempdir().unwrap();
    let (pull_trace, pull_events) = lifecycle_capture("pull-lifecycle");
    run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions {
            lifecycle_trace: pull_trace,
            ..PullSessionOptions::default()
        },
    )
    .await
    .expect("traced pull succeeds");

    {
        let push_events = push_events.lock().expect("push lifecycle events");
        let pull_events = pull_events.lock().expect("pull lifecycle events");
        assert_success_lifecycle(&push_events, SessionPhaseRole::Source);
        assert_success_lifecycle(&pull_events, SessionPhaseRole::Destination);
        assert_ne!(
            push_events
                .last()
                .and_then(|event| event.session_id.as_ref()),
            pull_events
                .last()
                .and_then(|event| event.session_id.as_ref()),
            "independent sessions must not share a derived session id"
        );
    }
    daemon.stop().await;
}

/// ph-1c recording matrix, served legs (plan §Acceptance: "daemon side
/// proven, not assumed"): a served push and a served pull each append a
/// route-labeled record to the DAEMON's own store, tagged with the role
/// the daemon actually played — DESTINATION when serving a push, SOURCE
/// when serving a pull — and keyed per the ph-1b conventions
/// (`peer_host:local_root` / host-level).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn served_sessions_record_daemon_side_perf_history() {
    use blit_core::perf_history::{Initiator, LocalRole, Topology};

    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &[("a.txt", b"alpha", 1_600_000_001)]);

    run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("push succeeds");

    let records = daemon.wait_for_perf_records(1).await;
    assert_eq!(records.len(), 1, "served push appends exactly one record");
    let rec = &records[0];
    assert_eq!(rec.topology, Topology::Remote);
    assert_eq!(rec.local_role, LocalRole::Destination);
    assert_eq!(rec.initiator, Initiator::Cli);
    let key = rec.peer_key.as_deref().expect("served push keys peer+root");
    assert!(
        key.starts_with("127.0.0.1:"),
        "host-prefixed key, got {key}"
    );
    assert!(
        key.ends_with(&daemon.dest_root.display().to_string()),
        "key names the local destination root, got {key}"
    );
    assert!(
        rec.run_kind.is_real_transfer(),
        "served rows must be seed-eligible"
    );
    assert_eq!(rec.file_count, 1);
    assert_eq!(rec.total_bytes, 5);
    assert_eq!(rec.error_count, 0);

    let dest = tempfile::tempdir().unwrap();
    run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("pull succeeds");

    let records = daemon.wait_for_perf_records(2).await;
    assert_eq!(records.len(), 2, "served pull appends its own record");
    let rec = &records[1];
    assert_eq!(rec.topology, Topology::Remote);
    assert_eq!(rec.local_role, LocalRole::Source);
    assert_eq!(rec.initiator, Initiator::Cli);
    assert_eq!(
        rec.peer_key.as_deref(),
        Some("127.0.0.1"),
        "a pulled-from daemon knows only the peer host — no shared bucket"
    );
    assert_eq!(rec.total_bytes, 5);

    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_refusal_ends_establishment_without_a_session_body() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &[("a.txt", b"alpha", 1_600_000_001)]);
    let (trace, events) = lifecycle_capture("refused-lifecycle");

    let err = run_push_session(
        &daemon.endpoint_for_missing_module(),
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions {
            lifecycle_trace: trace,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect_err("unknown module refuses the traced session");
    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);

    {
        let events = events.lock().expect("refused lifecycle events");
        assert_eq!(
            events.iter().map(|event| event.event).collect::<Vec<_>>(),
            [
                "control_connect_begin",
                "control_connect_end",
                "transfer_rpc_open_begin",
                "transfer_rpc_open_end",
                "session_establish_begin",
                "session_establish_end",
            ]
        );
        assert_eq!(
            events.last().and_then(|event| event.outcome),
            Some(TransferLifecycleOutcome::Refused)
        );
        assert!(events.iter().all(|event| event.session_id.is_none()));
    }
    daemon.stop().await;
}

#[tokio::test]
async fn lifecycle_connect_error_ends_before_rpc_open() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("reserve loopback port");
    let port = listener.local_addr().expect("reserved address").port();
    drop(listener);
    let endpoint = RemoteEndpoint {
        host: "127.0.0.1".into(),
        port,
        path: RemotePath::Module {
            module: "test".into(),
            rel_path: PathBuf::new(),
        },
    };
    let (trace, events) = lifecycle_capture("connect-error-lifecycle");

    let _err = connect_transfer_client_with_trace(&endpoint, &trace)
        .await
        .expect_err("a closed loopback port must refuse the control connection");

    let events = events.lock().expect("connect-error lifecycle events");
    assert_eq!(
        events.iter().map(|event| event.event).collect::<Vec<_>>(),
        ["control_connect_begin", "control_connect_end"]
    );
    assert_eq!(
        events.last().and_then(|event| event.outcome),
        Some(TransferLifecycleOutcome::Error)
    );
}

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

/// otp-7b-2 (review otp-7a F4, deferred to 7b): a `CancelJob` fired while
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
        blit_core::remote::transfer::source::SourceScan,
    ) {
        self.inner.scan(filter, unreadable)
    }

    async fn prepare_payload(
        &self,
        payload: blit_core::remote::transfer::payload::TransferPayload,
    ) -> eyre::Result<blit_core::remote::transfer::payload::PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn open_file(
        &self,
        header: &blit_core::generated::FileHeader,
    ) -> eyre::Result<OpenedSourceFile> {
        use tokio::io::AsyncReadExt;
        let opened = self.inner.open_file(header).await?;
        if header.relative_path == self.fail_path {
            Ok(OpenedSourceFile::virtual_reader(
                Box::new(opened.into_reader().take(self.limit)),
                header.size,
            ))
        } else {
            Ok(opened)
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
async fn mid_resume_fault_is_contained_and_named_in_the_summary() {
    // SOURCE_SIDE_CONTAINMENT ssc-3 (A10): over the daemon-served
    // session, a source reader that dies mid-resume closes the record
    // FAILED — the push COMPLETES with a summary naming the file and the
    // source's reason, the partial is left unstamped for the next run.
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
    let summary = tokio::time::timeout(
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
    .expect("a truncated source is contained, never a session fault");

    assert_eq!(summary.files_failed, 1);
    assert_eq!(summary.files_resumed, 0);
    assert_eq!(summary.failures[0].relative_path, "big.bin");
    assert!(
        summary.failures[0]
            .reason
            .starts_with("source: changed size during transfer"),
        "the summary carries the source's reason: {}",
        summary.failures[0].reason
    );
    let stamped = std::fs::metadata(daemon.dest_root.join("big.bin"))
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_ne!(
        stamped, 1_600_000_100,
        "a failed resume must not stamp the partial as converged"
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

/// review otp-8 F1: the mid-transfer cancel guard on the IN-STREAM
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
    let final_streams = outcome
        .data_plane_streams
        .expect("TCP data plane reports final logical membership");
    assert!(
        (1..=blit_core::dial::DIAL_DEFAULT_STREAM_LIMIT).contains(&final_streams),
        "live data-plane membership stays within the receiver safety range"
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
    // review otp-10b-2 F4: post-cutover every verb rides `Transfer`, so
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn served_session_rows_record_progress_for_both_roles() {
    let daemon = Daemon::start(false).await;
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());

    let push_summary = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("push session");
    assert!(push_summary.bytes_transferred > 0);

    let dest = tempfile::tempdir().unwrap();
    let pull_outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("pull session");
    assert!(pull_outcome.summary.bytes_transferred > 0);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let recent = loop {
        let recent = daemon.active_jobs.recent();
        if recent.len() >= 2 || std::time::Instant::now() > deadline {
            break recent;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };

    let push = recent
        .iter()
        .find(|r| r.kind == crate::active_jobs::ActiveJobKind::Push)
        .expect("served push record");
    assert_eq!(push.bytes, push_summary.bytes_transferred);
    assert_eq!(push.files, push_summary.files_transferred);

    let pull = recent
        .iter()
        .find(|r| r.kind == crate::active_jobs::ActiveJobKind::PullSync)
        .expect("served pull record");
    assert_eq!(pull.bytes, pull_outcome.summary.bytes_transferred);
    assert_eq!(pull.files, pull_outcome.summary.files_transferred);
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

// ---------------------------------------------------------------------------
// jl-1b: each served job leaves a log naming every file, read back over
// GetJobLog through the client the CLI uses.
// ---------------------------------------------------------------------------

fn kinds_and_names(events: &[blit_core::job_log::EventBody]) -> Vec<String> {
    use blit_core::job_log::{EventBody, PhaseState};
    events
        .iter()
        .filter_map(|event| match event {
            EventBody::Phase { name, state } => Some(format!(
                "phase {name} {}",
                if *state == PhaseState::Start {
                    "start"
                } else {
                    "end"
                }
            )),
            EventBody::FileCopied { path, .. } => Some(format!("copied {path}")),
            EventBody::FileSent { path, .. } => Some(format!("sent {path}")),
            EventBody::FileDeleted { path, .. } => Some(format!("deleted {path}")),
            EventBody::FileFailed { path, .. } => Some(format!("failed {path}")),
            _ => None,
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_served_mirror_push_logs_every_copy_and_deletion() {
    use blit_core::job_log::{EventBody, Outcome, Role};
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());
    let daemon = Daemon::start(false).await;
    write_tree(
        &daemon.dest_root,
        &[("stale.txt", b"old", 1), ("gone/old.txt", b"old", 1)],
    );

    run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions {
            mirror_enabled: true,
            mirror_kind: blit_core::generated::MirrorMode::All,
            ..PushSessionOptions::default()
        },
    )
    .await
    .expect("mirror push succeeds");

    let logs = daemon.job_logs().await;
    assert_eq!(logs.len(), 1, "one log: {logs:?}");
    let (role, _, events) = &logs[0];
    assert_eq!(role, "destination");
    let EventBody::RunStart(start) = &events[0] else {
        panic!("first event: {:?}", events[0]);
    };
    assert_eq!(
        (
            start.role,
            start.run.verb.as_str(),
            start.run.destination.as_str()
        ),
        (Role::Destination, "push", "/test")
    );
    assert!(
        start.run.options.contains(&"mirror=all".to_string()),
        "{start:?}"
    );

    let mut named = kinds_and_names(events);
    // Files land in any order; the phases frame them.
    let copies: Vec<String> = {
        let mut copies: Vec<String> = named
            .iter()
            .filter(|line| line.starts_with("copied "))
            .cloned()
            .collect();
        copies.sort();
        copies
    };
    assert_eq!(
        copies,
        [
            "copied a.txt",
            "copied dir one/b.log",
            "copied dir one/deeper/c.dat",
            "copied empty.bin",
        ]
    );
    named.retain(|line| !line.starts_with("copied "));
    assert_eq!(
        named,
        [
            "phase transfer start",
            "phase transfer end",
            "phase delete start",
            "deleted gone/old.txt",
            "deleted stale.txt",
            "deleted gone/",
            "phase delete end",
        ]
    );
    let summary = events
        .iter()
        .find_map(|event| match event {
            EventBody::Summary(summary) => Some(*summary),
            _ => None,
        })
        .expect("a summary");
    assert_eq!(
        (
            summary.files_copied,
            summary.files_deleted,
            summary.files_failed
        ),
        (4, 3, 0)
    );
    assert!(matches!(
        events.last(),
        Some(EventBody::RunEnd {
            outcome: Outcome::Ok,
            detail: None
        })
    ));

    daemon.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_served_pull_logs_each_file_sent() {
    use blit_core::job_log::{EventBody, Outcome, Role};
    let daemon = Daemon::start(false).await;
    write_tree(&daemon.dest_root, &small_tree());
    let dest = tempfile::tempdir().unwrap();

    run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("pull succeeds");

    let logs = daemon.job_logs().await;
    assert_eq!(logs.len(), 1, "one log: {logs:?}");
    let (role, _, events) = &logs[0];
    assert_eq!(role, "source");
    let EventBody::RunStart(start) = &events[0] else {
        panic!("first event: {:?}", events[0]);
    };
    assert_eq!(
        (
            start.role,
            start.run.verb.as_str(),
            start.run.source.as_str()
        ),
        (Role::Source, "pull", "/test")
    );
    let mut sent: Vec<String> = kinds_and_names(events)
        .into_iter()
        .filter(|line| line.starts_with("sent "))
        .collect();
    sent.sort();
    assert_eq!(
        sent,
        [
            "sent a.txt",
            "sent dir one/b.log",
            "sent dir one/deeper/c.dat",
            "sent empty.bin",
        ]
    );
    assert!(matches!(
        events.last(),
        Some(EventBody::RunEnd {
            outcome: Outcome::Ok,
            ..
        })
    ));

    daemon.stop().await;
}

/// A destination directory the daemon cannot write into fails its one file
/// on its own; the log names it, with the reason, and the job ends failed.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_served_push_logs_a_file_that_failed_and_why() {
    use blit_core::job_log::{EventBody, Outcome};
    use std::os::unix::fs::PermissionsExt;
    let src = tempfile::tempdir().unwrap();
    write_tree(
        src.path(),
        &[
            ("ok.txt", b"fine", 1_600_000_001),
            ("locked/f.txt", b"no", 1_600_000_002),
        ],
    );
    let daemon = Daemon::start(false).await;
    let locked = daemon.dest_root.join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();

    let summary = run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("the push finishes, with one file failed");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(summary.files_failed, 1, "{summary:?}");

    let logs = daemon.job_logs().await;
    let (_, _, events) = &logs[0];
    let failed: Vec<(&str, &str)> = events
        .iter()
        .filter_map(|event| match event {
            EventBody::FileFailed { path, reason, .. } => Some((path.as_str(), reason.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(failed.len(), 1, "named once: {failed:?}");
    assert_eq!(failed[0].0, "locked/f.txt");
    assert!(
        failed[0].1.to_lowercase().contains("permission denied"),
        "{failed:?}"
    );
    assert!(kinds_and_names(events).contains(&"copied ok.txt".to_string()));
    assert!(matches!(
        events.last(),
        Some(EventBody::RunEnd { outcome: Outcome::Failed, detail: Some(detail) })
            if detail == "1 file(s) failed"
    ));

    daemon.stop().await;
}

/// More failures than the summary keeps reasons for: every one is still
/// named with its own reason, because the destination reports each as it
/// fails rather than leaving the log to the capped summary. Both carriers.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_served_push_logs_every_failure_past_the_summarys_cap() {
    use blit_core::job_log::EventBody;
    for in_stream_bytes in [false, true] {
        let src = tempfile::tempdir().unwrap();
        let daemon = Daemon::start(false).await;
        let names: Vec<String> = (0..70).map(|n| format!("f{n:02}.txt")).collect();
        for name in &names {
            std::fs::write(src.path().join(name), b"x").unwrap();
            // A folder in the way fails the file on its own.
            std::fs::create_dir(daemon.dest_root.join(name)).unwrap();
        }

        let summary = run_push_session(
            &daemon.endpoint,
            Arc::new(FsTransferSource::new(src.path().to_path_buf())),
            PushSessionOptions {
                in_stream_bytes,
                ..PushSessionOptions::default()
            },
        )
        .await
        .expect("the push finishes, every file failed");
        assert_eq!(summary.files_failed, 70);
        assert!(summary.failures.len() < 70, "the summary caps its reasons");

        let logs = daemon.job_logs().await;
        let (_, _, events) = &logs[0];
        let mut failed: Vec<(&str, &str)> = events
            .iter()
            .filter_map(|event| match event {
                EventBody::FileFailed { path, reason, .. } => {
                    Some((path.as_str(), reason.as_str()))
                }
                _ => None,
            })
            .collect();
        failed.sort();
        assert_eq!(
            failed.iter().map(|(path, _)| *path).collect::<Vec<_>>(),
            names.iter().map(String::as_str).collect::<Vec<_>>(),
            "in_stream_bytes={in_stream_bytes}"
        );
        assert!(
            failed
                .iter()
                .all(|(_, reason)| !reason.contains("the transfer's report kept no reason")),
            "in_stream_bytes={in_stream_bytes}: a failure fell back to the summary: {failed:?}"
        );

        daemon.stop().await;
    }
}

/// A file the pulling end could not write is named in the serving
/// source's log too — from the destination's summary, the only account a
/// source gets.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_served_pull_logs_the_destinations_failures() {
    use blit_core::job_log::EventBody;
    let daemon = Daemon::start(false).await;
    write_tree(&daemon.dest_root, &small_tree());
    let dest = tempfile::tempdir().unwrap();
    std::fs::create_dir(dest.path().join("a.txt")).unwrap();

    let outcome = run_pull_session(
        &daemon.endpoint,
        dest.path().to_path_buf(),
        PullSessionOptions::default(),
    )
    .await
    .expect("the pull finishes, one file failed");
    assert_eq!(outcome.summary.files_failed, 1);

    let logs = daemon.job_logs().await;
    // jl-1c: the job's record carries the count, though the job ran to
    // its end.
    let record = &daemon.active_jobs.recent()[0];
    assert!(record.ok);
    assert_eq!(record.files_failed, 1);
    let (role, _, events) = &logs[0];
    assert_eq!(role, "source");
    let failed: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            EventBody::FileFailed { path, .. } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(failed, ["a.txt"]);

    daemon.stop().await;
}

/// A source that cannot open any file: every granted file is skipped by
/// the sending side, the containment path a vanished or locked file takes.
struct CannotOpenSource {
    inner: FsTransferSource,
}

#[async_trait::async_trait]
impl blit_core::remote::transfer::source::TransferSource for CannotOpenSource {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
    ) -> (
        tokio::sync::mpsc::Receiver<blit_core::generated::FileHeader>,
        blit_core::remote::transfer::source::SourceScan,
    ) {
        self.inner.scan(filter, unreadable)
    }

    async fn prepare_payload(
        &self,
        payload: blit_core::remote::transfer::payload::TransferPayload,
    ) -> eyre::Result<blit_core::remote::transfer::payload::PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn open_file(
        &self,
        _header: &blit_core::generated::FileHeader,
    ) -> eyre::Result<OpenedSourceFile> {
        eyre::bail!("refused by the test")
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

/// Each kind of single-file record logs its failure as it happens — inside
/// the transfer phase — not only from the summary at the end: a file the
/// destination could not write, and a file the source skipped. Both
/// carriers.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_single_file_failure_is_logged_as_it_happens() {
    use blit_core::job_log::{EventBody, PhaseState};
    for in_stream_bytes in [false, true] {
        for source_skips in [false, true] {
            let case = format!("in_stream_bytes={in_stream_bytes} source_skips={source_skips}");
            let src = tempfile::tempdir().unwrap();
            // One small file is never batched: it travels as its own record.
            std::fs::write(src.path().join("one.txt"), b"x").unwrap();
            let daemon = Daemon::start(false).await;
            let source: Arc<dyn blit_core::remote::transfer::source::TransferSource> =
                if source_skips {
                    Arc::new(CannotOpenSource {
                        inner: FsTransferSource::new(src.path().to_path_buf()),
                    })
                } else {
                    std::fs::create_dir(daemon.dest_root.join("one.txt")).unwrap();
                    Arc::new(FsTransferSource::new(src.path().to_path_buf()))
                };

            let summary = run_push_session(
                &daemon.endpoint,
                source,
                PushSessionOptions {
                    in_stream_bytes,
                    ..PushSessionOptions::default()
                },
            )
            .await
            .unwrap_or_else(|err| panic!("{case}: the push finishes: {err:#}"));
            assert_eq!(summary.files_failed, 1, "{case}");

            let logs = daemon.job_logs().await;
            let (_, _, events) = &logs[0];
            let failed_at = events
                .iter()
                .position(|event| matches!(event, EventBody::FileFailed { path, .. } if path == "one.txt"))
                .unwrap_or_else(|| panic!("{case}: no failure logged: {events:?}"));
            let phase_end = events
                .iter()
                .position(|event| {
                    matches!(event, EventBody::Phase { name, state: PhaseState::End } if name == "transfer")
                })
                .unwrap_or_else(|| panic!("{case}: no transfer end: {events:?}"));
            assert!(
                failed_at < phase_end,
                "{case}: the failure was only learned from the summary: {events:?}"
            );
            if source_skips {
                let EventBody::FileFailed { reason, .. } = &events[failed_at] else {
                    unreachable!()
                };
                assert!(
                    reason.starts_with("source: cannot open"),
                    "{case}: {reason}"
                );
            }

            daemon.stop().await;
        }
    }
}

/// Review cr-jl1a-1, end to end: a pushed file whose name is not valid
/// UTF-8 is logged on the daemon with its exact bytes. Linux only — the
/// one platform here whose file system stores such a name; this guard
/// cannot fail on macOS or Windows.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_raw_named_file_is_logged_with_its_exact_bytes() {
    use blit_core::job_log::EventBody;
    use std::os::unix::ffi::OsStrExt as _;
    let src = tempfile::tempdir().unwrap();
    std::fs::write(
        src.path().join(std::ffi::OsStr::from_bytes(b"caf\xe9.txt")),
        b"x",
    )
    .unwrap();
    let daemon = Daemon::start(false).await;

    run_push_session(
        &daemon.endpoint,
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await
    .expect("the push lands the raw-named file");

    let logs = daemon.job_logs().await;
    let (_, _, events) = &logs[0];
    let copied: Vec<Option<&str>> = events
        .iter()
        .filter_map(|event| match event {
            EventBody::FileCopied { raw, .. } => Some(raw.as_deref()),
            _ => None,
        })
        .collect();
    assert_eq!(copied, [Some("caf\\xe9.txt")]);

    daemon.stop().await;
}

/// Review cr-jl1b-1: an open refused for an unknown module still leaves the
/// job's log — it names the job, its role and the refusal.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_served_job_refused_at_open_is_logged() {
    use blit_core::job_log::{EventBody, Outcome};
    let src = tempfile::tempdir().unwrap();
    write_tree(src.path(), &small_tree());
    let daemon = Daemon::start(false).await;

    let refused = run_push_session(
        &daemon.endpoint_for_missing_module(),
        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
        PushSessionOptions::default(),
    )
    .await;
    assert!(refused.is_err(), "an unknown module is refused");

    let logs = daemon.job_logs().await;
    let (role, _, events) = &logs[0];
    assert_eq!(role, "destination");
    let EventBody::RunStart(start) = &events[0] else {
        panic!("first event: {:?}", events[0]);
    };
    assert_eq!(start.run.destination, "/nope");
    assert!(
        matches!(
            events.last(),
            Some(EventBody::RunEnd { outcome: Outcome::Failed, detail: Some(detail) })
                if detail.contains("nope")
        ),
        "{events:?}"
    );

    daemon.stop().await;
}
