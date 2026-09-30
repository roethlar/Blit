//! Source-side per-file containment (contract v7,
//! `docs/plan/SOURCE_SIDE_CONTAINMENT.md` ssc-1).
//!
//! A file the SOURCE cannot open, or whose size no longer matches the
//! manifest when the source goes to read it, is SKIPPED — nothing of it
//! is announced on the wire — and reported through the destination's
//! per-file failure report, on both byte carriers and under both
//! initiator roles. The rest of the manifest lands, the session
//! completes, and the run reports exit-2 semantics (`files_failed`).
//! The destination's need ledger refuses every off-contract shape: a
//! need never delivered nor skipped, a skip for an un-granted path, a
//! record terminator with no open record, and an `ok` terminator short
//! of the header's size.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blit_core::generated::transfer_frame::Frame;
use blit_core::generated::{
    session_error, FileData, FileFailure, FileHeader, ManifestComplete, MirrorMode, RecordEnd,
    SessionHello, SessionOpen, SourceDone, TransferFrame, TransferRole, TransferSummary,
};
use blit_core::remote::transfer::source::{
    FsTransferSource, OpenedSourceFile, SourceScan, TransferSource,
};
use blit_core::remote::transfer::{PreparedPayload, TransferPayload};
use blit_core::transfer_plan::PlanOptions;
use blit_core::transfer_session::transport::{in_process_pair, FrameTransport};
use blit_core::transfer_session::{
    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
    HelloConfig, SessionEndpoint, SessionFault, SourceSessionConfig,
};
use blit_core::transfers::failures::{failures_from_wire, refuse_source_delete_on_failures};

const SUITE_TIMEOUT: Duration = Duration::from_secs(120);

/// Above the planner's shard threshold, so every fixture file rides as
/// its own single-file record (tar shards are ssc-2's slice).
const BIG: usize = 1_500_000;

fn patterned(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

fn write_tree(root: &Path, files: &[(&str, Vec<u8>, i64)]) {
    for (rel, content, mtime) in files {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
    }
}

fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
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

/// How the fault-injecting source misbehaves for one path.
#[derive(Clone, Copy)]
enum Fault {
    /// `open_file` fails (a locked, denied, or vanished file).
    OpenFails,
    /// `open_file` succeeds but the handle reports this length — a file
    /// whose size drifted since the scan.
    DeclaresLen(u64),
}

struct FaultySource {
    inner: FsTransferSource,
    faults: HashMap<&'static str, Fault>,
}

#[async_trait::async_trait]
impl TransferSource for FaultySource {
    fn scan(
        &self,
        filter: Option<blit_core::fs_enum::FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (tokio::sync::mpsc::Receiver<FileHeader>, SourceScan) {
        self.inner.scan(filter, unreadable_paths)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> eyre::Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> eyre::Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(&self, header: &FileHeader) -> eyre::Result<OpenedSourceFile> {
        match self.faults.get(header.relative_path.as_str()) {
            Some(Fault::OpenFails) => Err(eyre::eyre!(
                "The process cannot access the file because it is being used by another process. (os error 32)"
            )),
            Some(Fault::DeclaresLen(len)) => {
                let opened = self.inner.open_file(header).await?;
                Ok(OpenedSourceFile::virtual_reader(opened.into_reader(), *len))
            }
            None => self.inner.open_file(header).await,
        }
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Carrier {
    InStream,
    DataPlane,
}

fn open_for(initiator_role: TransferRole, carrier: Carrier) -> SessionOpen {
    SessionOpen {
        initiator_role: initiator_role as i32,
        compare_mode: blit_core::generated::ComparisonMode::SizeMtime as i32,
        in_stream_bytes: carrier == Carrier::InStream,
        ..Default::default()
    }
}

/// Drive one session src→dst with `source` (the fault-injecting one)
/// under the given initiator role and carrier.
async fn run_with(
    open: SessionOpen,
    carrier: Carrier,
    source: Arc<dyn TransferSource>,
    dst_root: PathBuf,
) -> (
    eyre::Result<TransferSummary>,
    eyre::Result<DestinationOutcome>,
) {
    let initiator_role = TransferRole::try_from(open.initiator_role).unwrap();
    let loopback = || (carrier == Carrier::DataPlane).then(|| "127.0.0.1".to_string());
    let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
        TransferRole::Source => (
            SessionEndpoint::initiator(open),
            SessionEndpoint::Responder,
            loopback(),
            None,
        ),
        TransferRole::Destination => (
            SessionEndpoint::Responder,
            SessionEndpoint::initiator(open),
            None,
            loopback(),
        ),
        TransferRole::Unspecified => unreachable!(),
    };
    let source_cfg = SourceSessionConfig {
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: source_endpoint,
        plan_options: PlanOptions::default(),
        data_plane_host: source_host,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: dest_endpoint,
        data_plane_host: dest_host,
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
    };
    let (a, b) = in_process_pair();
    tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
        )
    })
    .await
    .expect("session run timed out")
}

fn three_big_files() -> Vec<(&'static str, Vec<u8>, i64)> {
    vec![
        ("ok1.bin", patterned(BIG, 1), 1_600_000_001),
        ("locked.bin", patterned(BIG, 2), 1_600_000_002),
        ("sub/ok2.bin", patterned(BIG, 3), 1_600_000_003),
    ]
}

/// The A3/A4 property, one fault at a time: the faulted file is
/// reported with a `source:` reason, the other two land, both ends hold
/// the same summary, and the run is exit-2 material (`files_failed`).
async fn assert_skip_contained(carrier: Carrier, fault: Fault, reason_prefix: &str) {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &three_big_files());

        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()),
            faults: HashMap::from([("locked.bin", fault)]),
        });
        let (sr, dr) = run_with(
            open_for(initiator_role, carrier),
            carrier,
            source,
            dst_root.clone(),
        )
        .await;
        let summary = sr.unwrap_or_else(|e| {
            panic!("source must complete ({carrier:?}, init {initiator_role:?}): {e:#}")
        });
        let dest = dr.unwrap_or_else(|e| {
            panic!("destination must complete ({carrier:?}, init {initiator_role:?}): {e:#}")
        });
        assert_eq!(summary, dest.summary, "both ends agree ({carrier:?})");
        assert_eq!(
            summary.in_stream_carrier_used,
            carrier == Carrier::InStream,
            "the fixture must ride the carrier under test"
        );
        assert_eq!(summary.files_failed, 1, "exactly the faulted file fails");
        assert_eq!(summary.files_transferred, 2, "the other two land");
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].relative_path, "locked.bin");
        assert!(
            summary.failures[0].reason.starts_with(reason_prefix),
            "reason must be the source's ({carrier:?}): {}",
            summary.failures[0].reason
        );
        let landed = collect_tree(&dst_root);
        assert_eq!(
            landed.keys().collect::<Vec<_>>(),
            vec!["ok1.bin", "sub/ok2.bin"],
            "nothing of the skipped file lands ({carrier:?})"
        );
        assert_eq!(landed["ok1.bin"], patterned(BIG, 1));
        assert_eq!(landed["sub/ok2.bin"], patterned(BIG, 3));
        // A6: the move gate reads this summary and refuses to delete the
        // source while a file did not land.
        let gate = refuse_source_delete_on_failures(
            "src",
            summary.files_failed,
            &failures_from_wire(&summary.failures),
        );
        assert!(gate.is_err(), "move must refuse source deletion on a skip");
    }
}

// ---------------------------------------------------------------------------
// A3: open failure is skipped before announcement, both carriers
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_source_open_failure_is_skipped_and_reported() {
    assert_skip_contained(Carrier::InStream, Fault::OpenFails, "source: cannot open:").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_source_open_failure_is_skipped_and_reported() {
    // Mutation proof: restore the `?` on `open_file` in
    // `DataPlaneSession::send_file` and this test's source faults with
    // "opening locked.bin" instead of completing.
    assert_skip_contained(Carrier::DataPlane, Fault::OpenFails, "source: cannot open:").await;
}

// ---------------------------------------------------------------------------
// A4: size drift at open is skipped before announcement, both carriers
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_size_drift_at_open_is_skipped_and_reported() {
    assert_skip_contained(
        Carrier::InStream,
        Fault::DeclaresLen(BIG as u64 + 4096),
        "source: changed size during transfer (manifest",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_size_drift_at_open_is_skipped_and_reported() {
    assert_skip_contained(
        Carrier::DataPlane,
        Fault::DeclaresLen(BIG as u64 - 1),
        "source: changed size during transfer (manifest",
    )
    .await;
}

// ---------------------------------------------------------------------------
// A7: a mirror under a source-side skip still deletes extraneous entries
// and keeps the skipped file's destination counterpart
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mirror_under_a_source_side_skip_deletes_extraneous_and_keeps_the_counterpart() {
    for carrier in [Carrier::InStream, Carrier::DataPlane] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &three_big_files());
        let old_copy = patterned(BIG, 9);
        write_tree(
            &dst_root,
            &[
                ("locked.bin", old_copy.clone(), 1_500_000_000),
                ("stale.txt", b"gone".to_vec(), 1_500_000_000),
            ],
        );
        let mut open = open_for(TransferRole::Source, carrier);
        open.mirror_enabled = true;
        open.mirror_kind = MirrorMode::All as i32;
        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()),
            faults: HashMap::from([("locked.bin", Fault::OpenFails)]),
        });
        let (sr, dr) = run_with(open, carrier, source, dst_root.clone()).await;
        let summary = sr.unwrap_or_else(|e| panic!("source must complete ({carrier:?}): {e:#}"));
        let dest = dr.unwrap_or_else(|e| panic!("destination must complete ({carrier:?}): {e:#}"));
        assert_eq!(summary, dest.summary);
        assert_eq!(summary.files_failed, 1);
        assert_eq!(
            summary.entries_deleted, 1,
            "stale.txt is extraneous and goes ({carrier:?})"
        );
        assert!(!dst_root.join("stale.txt").exists());
        assert_eq!(
            std::fs::read(dst_root.join("locked.bin")).unwrap(),
            old_copy,
            "the skipped file's counterpart is in the manifest, never extraneous ({carrier:?})"
        );
        assert_eq!(
            std::fs::read(dst_root.join("ok1.bin")).unwrap(),
            patterned(BIG, 1)
        );
    }
}

// ---------------------------------------------------------------------------
// The real thing on Windows: a file held open with no sharing
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn windows_sharing_violation_is_skipped_and_reported() {
    use std::os::windows::fs::OpenOptionsExt;
    for carrier in [Carrier::InStream, Carrier::DataPlane] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &three_big_files());
        // Hold the file with share_mode(0): every other open fails with
        // ERROR_SHARING_VIOLATION — what NTUSER.DAT and a live SQLite WAL
        // look like to a backup.
        let _held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(src_root.join("locked.bin"))
            .unwrap();
        let source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root.clone()));
        let (sr, dr) = run_with(
            open_for(TransferRole::Source, carrier),
            carrier,
            source,
            dst_root.clone(),
        )
        .await;
        let summary = sr.unwrap_or_else(|e| panic!("source must complete ({carrier:?}): {e:#}"));
        let dest = dr.unwrap_or_else(|e| panic!("destination must complete ({carrier:?}): {e:#}"));
        assert_eq!(summary, dest.summary);
        assert_eq!(
            summary.files_failed, 1,
            "{carrier:?}: {:?}",
            summary.failures
        );
        assert_eq!(summary.failures[0].relative_path, "locked.bin");
        assert!(summary.failures[0]
            .reason
            .starts_with("source: cannot open:"));
        assert_eq!(summary.files_transferred, 2);
    }
}

// ---------------------------------------------------------------------------
// A5: the ledger's violations, scripted peer on the in-stream carrier
// ---------------------------------------------------------------------------

fn wire(frame: Frame) -> TransferFrame {
    TransferFrame { frame: Some(frame) }
}

async fn recv_or_panic(t: &mut FrameTransport) -> Frame {
    t.recv()
        .await
        .unwrap()
        .expect("peer closed unexpectedly")
        .frame
        .expect("empty frame")
}

fn hello_frame() -> TransferFrame {
    let hello = HelloConfig::default();
    wire(Frame::Hello(SessionHello {
        build_id: hello.build_id,
        contract_version: hello.contract_version,
    }))
}

fn fault_of(err: &eyre::Report) -> &SessionFault {
    err.downcast_ref::<SessionFault>()
        .unwrap_or_else(|| panic!("expected a SessionFault, got {err:#}"))
}

/// Script a SOURCE against a real destination: hello, open (in-stream,
/// SOURCE initiator), one manifest entry of `size` bytes, complete —
/// and consume the need choreography. Returns the peer transport, the
/// destination join handle, and the granted header.
async fn scripted_source_with_one_grant(
    dst_root: PathBuf,
    size: u64,
) -> (
    FrameTransport,
    tokio::task::JoinHandle<eyre::Result<DestinationOutcome>>,
    FileHeader,
) {
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
    };
    let (mut peer, dest_transport) = in_process_pair();
    let dest = tokio::spawn(run_destination(
        dest_cfg,
        dest_transport,
        DestinationTarget::Fixed(dst_root),
    ));
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(wire(Frame::Open(open_for(
        TransferRole::Source,
        Carrier::InStream,
    ))))
    .await
    .unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
    let header = FileHeader {
        relative_path: "granted.bin".into(),
        size,
        mtime_seconds: 1_600_000_100,
        permissions: 0o644,
        ..Default::default()
    };
    peer.send(wire(Frame::ManifestEntry(header.clone())))
        .await
        .unwrap();
    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
        scan_complete: true,
    })))
    .await
    .unwrap();
    let mut granted = false;
    let mut complete = false;
    while !(granted && complete) {
        match recv_or_panic(&mut peer).await {
            Frame::NeedBatch(batch) => {
                assert!(batch
                    .entries
                    .iter()
                    .any(|e| e.relative_path == "granted.bin"));
                granted = true;
            }
            Frame::NeedComplete(_) => complete = true,
            other => panic!("expected need choreography, got {other:?}"),
        }
    }
    (peer, dest, header)
}

async fn expect_violation(
    peer: &mut FrameTransport,
    dest: tokio::task::JoinHandle<eyre::Result<DestinationOutcome>>,
    needle: &str,
) {
    let refusal = tokio::time::timeout(SUITE_TIMEOUT, async {
        match recv_or_panic(peer).await {
            Frame::Error(e) => e,
            other => panic!("expected SessionError, got {other:?}"),
        }
    })
    .await
    .expect("the violation must be answered promptly, not absorbed");
    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
    let dest_err = dest.await.unwrap().unwrap_err();
    let fault = fault_of(&dest_err);
    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
    assert!(
        fault.message.contains(needle),
        "expected {needle:?} in: {}",
        fault.message
    );
}

#[tokio::test]
async fn source_done_with_a_need_never_delivered_nor_skipped_is_a_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    let (mut peer, dest, _header) = scripted_source_with_one_grant(dst_root, 8).await;
    peer.send(wire(Frame::SourceDone(SourceDone {})))
        .await
        .unwrap();
    expect_violation(&mut peer, dest, "never delivered").await;
}

#[tokio::test]
async fn skip_for_an_ungranted_path_is_a_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    let (mut peer, dest, _header) = scripted_source_with_one_grant(dst_root, 8).await;
    peer.send(wire(Frame::FileSkipped(FileFailure {
        relative_path: "evil.bin".into(),
        reason: "source: cannot open".into(),
    })))
    .await
    .unwrap();
    expect_violation(&mut peer, dest, "never granted").await;
}

#[tokio::test]
async fn file_end_with_no_open_record_is_a_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    let (mut peer, dest, _header) = scripted_source_with_one_grant(dst_root, 8).await;
    peer.send(wire(Frame::FileEnd(RecordEnd {
        ok: true,
        reason: String::new(),
    })))
    .await
    .unwrap();
    expect_violation(&mut peer, dest, "FileEnd").await;
}

#[tokio::test]
async fn ok_terminator_short_of_the_header_size_is_a_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    let (mut peer, dest, header) = scripted_source_with_one_grant(dst_root, 8).await;
    peer.send(wire(Frame::FileBegin(header))).await.unwrap();
    peer.send(wire(Frame::FileData(FileData {
        content: b"1234".to_vec(),
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::FileEnd(RecordEnd {
        ok: true,
        reason: String::new(),
    })))
    .await
    .unwrap();
    expect_violation(&mut peer, dest, "still promised").await;
}

#[tokio::test]
async fn skip_for_a_granted_need_then_source_done_completes_with_the_failure_reported() {
    // The destination half of a skip, end to end on the scripted lane:
    // Granted → Failed, the summary carries it, SourceDone is accepted.
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    let (mut peer, dest, _header) = scripted_source_with_one_grant(dst_root.clone(), 8).await;
    peer.send(wire(Frame::FileSkipped(FileFailure {
        relative_path: "granted.bin".into(),
        reason: "source: cannot open: locked".into(),
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::SourceDone(SourceDone {})))
        .await
        .unwrap();
    let summary = tokio::time::timeout(SUITE_TIMEOUT, async {
        match recv_or_panic(&mut peer).await {
            Frame::Summary(s) => s,
            other => panic!("expected TransferSummary, got {other:?}"),
        }
    })
    .await
    .expect("summary must follow SourceDone");
    assert_eq!(summary.files_failed, 1);
    assert_eq!(summary.failures[0].relative_path, "granted.bin");
    assert_eq!(summary.failures[0].reason, "source: cannot open: locked");
    assert_eq!(summary.files_transferred, 0);
    assert!(!dst_root.join("granted.bin").exists());
    let outcome = dest.await.unwrap().expect("destination completes");
    assert_eq!(outcome.summary, summary);
    // A second skip for the same (now Failed) need would be a violation:
    // pinned by the ledger's own unit tests; here the session already
    // closed cleanly.
}

#[tokio::test]
async fn failed_terminator_discards_the_partial_and_reports_the_file() {
    // The destination half of a retraction (the source half is ssc-3):
    // a record that ends `ok = false` at any byte count leaves no file
    // and is reported with the source's reason.
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    let (mut peer, dest, header) = scripted_source_with_one_grant(dst_root.clone(), 8).await;
    peer.send(wire(Frame::FileBegin(header))).await.unwrap();
    peer.send(wire(Frame::FileData(FileData {
        content: b"1234".to_vec(),
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::FileEnd(RecordEnd {
        ok: false,
        reason: "source: read error: Input/output error".into(),
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::SourceDone(SourceDone {})))
        .await
        .unwrap();
    let summary = tokio::time::timeout(SUITE_TIMEOUT, async {
        match recv_or_panic(&mut peer).await {
            Frame::Summary(s) => s,
            other => panic!("expected TransferSummary, got {other:?}"),
        }
    })
    .await
    .expect("summary must follow SourceDone");
    assert_eq!(summary.files_failed, 1);
    assert_eq!(summary.files_transferred, 0);
    assert_eq!(
        summary.failures[0].reason,
        "source: read error: Input/output error"
    );
    assert!(
        !dst_root.join("granted.bin").exists(),
        "the partial must not be left looking like a finished file"
    );
    dest.await.unwrap().expect("destination completes");
}

// ---------------------------------------------------------------------------
// ssc-2 — A1/A2: tar shard fidelity. A member whose bytes are not exactly
// what its manifest header promised is skipped and reported; its
// shard-mates land intact.
// ---------------------------------------------------------------------------

/// Small enough that three of them plan as ONE tar shard (the planner's
/// small-file rule: count ≥ 32 or average ≤ 128 KiB).
const SMALL: usize = 4_096;

fn three_small_files() -> Vec<(&'static str, Vec<u8>, i64)> {
    vec![
        ("a.txt", patterned(SMALL, 11), 1_600_000_011),
        ("drift.txt", patterned(SMALL, 12), 1_600_000_012),
        ("sub/c.txt", patterned(SMALL, 13), 1_600_000_013),
    ]
}

#[derive(Clone, Copy, Debug)]
enum Drift {
    /// Grew after the scan (the 2026-09-25 field failure: a SQLite WAL).
    Grow(usize),
    /// Shrank after the scan.
    Truncate(usize),
    /// Deleted after the scan (a rollback journal that went away).
    Vanish,
}

fn apply_drift(path: &Path, drift: Drift) {
    match drift {
        Drift::Grow(n) => {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
            f.write_all(&patterned(n, 99)).unwrap();
        }
        Drift::Truncate(len) => {
            let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
            f.set_len(len as u64).unwrap();
        }
        Drift::Vanish => std::fs::remove_file(path).unwrap(),
    }
}

/// Mutates named members ON DISK between the scan and the packer's
/// read — exactly the window in which a live file changes — then
/// delegates to the real filesystem source so the real packer sees it.
struct ShardDriftSource {
    inner: FsTransferSource,
    drifts: HashMap<&'static str, Drift>,
    applied: Mutex<bool>,
}

#[async_trait::async_trait]
impl TransferSource for ShardDriftSource {
    fn scan(
        &self,
        filter: Option<blit_core::fs_enum::FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (tokio::sync::mpsc::Receiver<FileHeader>, SourceScan) {
        self.inner.scan(filter, unreadable_paths)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> eyre::Result<PreparedPayload> {
        {
            let mut applied = self.applied.lock().unwrap();
            if !*applied {
                for (rel, drift) in &self.drifts {
                    apply_drift(&self.inner.root().join(rel), *drift);
                }
                *applied = true;
            }
        }
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> eyre::Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(&self, header: &FileHeader) -> eyre::Result<OpenedSourceFile> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

/// The packer alone, against the extractor alone: the pre-ssc-2 packer
/// streamed a grown member to EOF under its stale header size, so the
/// extractor parsed the overflow as the next tar header and died with
/// `tar shard entry: numeric field was not a number …` (the field
/// message). Now the grown member is skipped and its shard-mate extracts.
///
/// Mutation proof (ssc-2 (i)): restore `append_data(&mut tar_header,
/// rel, &mut file)` with `set_size(header.size)` in `build_tar_shard`
/// and this test fails at the extractor with that message.
#[test]
fn packer_never_lets_a_grown_member_corrupt_its_shard_mate() {
    use blit_core::remote::transfer::tar_safety::{safe_extract_tar_shard, TarShardExtractOptions};
    use blit_core::remote::transfer::{build_tar_shard, changed_size_reason};

    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    write_tree(&src, &three_small_files());
    let headers: Vec<FileHeader> = three_small_files()
        .iter()
        .map(|(rel, bytes, mtime)| FileHeader {
            relative_path: rel.to_string(),
            size: bytes.len() as u64,
            mtime_seconds: *mtime,
            permissions: 0o644,
            ..Default::default()
        })
        .collect();
    // The scan captured 4096 bytes; the file grows before the pack.
    apply_drift(&src.join("drift.txt"), Drift::Grow(1_000));

    let built = build_tar_shard(&src, &headers).expect("packing must not fail");
    assert_eq!(
        built
            .headers
            .iter()
            .map(|h| h.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["a.txt", "sub/c.txt"],
        "only the members that matched their headers are packed"
    );
    assert_eq!(built.skipped.len(), 1);
    assert_eq!(built.skipped[0].relative_path, "drift.txt");
    assert_eq!(
        built.skipped[0].reason,
        changed_size_reason(SMALL as u64, SMALL as u64 + 1_000)
    );

    let extracted = safe_extract_tar_shard(
        &built.data,
        built.headers.clone(),
        &dst,
        &TarShardExtractOptions::default(),
    )
    .expect("the packed members extract cleanly");
    // The extractor decodes into memory (the sink writes afterwards);
    // both shard-mates come back byte-exact under their own names.
    let mut got: Vec<(String, Vec<u8>)> =
        extracted.into_iter().map(|e| (e.rel, e.contents)).collect();
    got.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        got,
        vec![
            ("a.txt".to_string(), patterned(SMALL, 11)),
            ("sub/c.txt".to_string(), patterned(SMALL, 13)),
        ]
    );
}

/// The stat pre-check and the read-side checks (bounded read + one-byte
/// probe) are independent defences against the same drift. A static
/// fixture cannot grow BETWEEN the stat and the read, so the probe's
/// own value is proven by mutation pairs, not by one red test:
/// (ii-a) stat check disabled → this test stays green (the probe
/// catches the growth); (ii-b) stat check AND probe disabled → red
/// (the stale 4096-byte prefix is packed under the manifest header).
#[test]
fn packer_skips_a_member_that_grows_between_stat_and_read() {
    use blit_core::remote::transfer::build_tar_shard;

    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    // The file on disk is already grown; the header promises the old
    // size but the stat check is bypassed by lying about neither — so
    // this pins the READ-side checks: the manifest says 4096, the file
    // holds 4096+1000, the stat catches it. To isolate the probe, the
    // second header promises exactly the on-disk length minus nothing
    // while the file is longer than the `take` window can see.
    write_tree(&src, &[("grown.txt", patterned(SMALL + 1_000, 12), 1)]);
    let header = FileHeader {
        relative_path: "grown.txt".into(),
        size: SMALL as u64,
        mtime_seconds: 1,
        permissions: 0o644,
        ..Default::default()
    };
    let built = build_tar_shard(&src, &[header]).unwrap();
    assert!(built.headers.is_empty(), "the grown member is not packed");
    assert!(built.data.is_empty(), "a fully-skipped shard has no bytes");
    assert_eq!(built.skipped.len(), 1);
    assert!(
        built.skipped[0]
            .reason
            .starts_with("source: changed size during transfer"),
        "{}",
        built.skipped[0].reason
    );
}

/// The A1/A2 property end to end: the drifted member is reported with a
/// `source:` reason, its two shard-mates land byte-exact, both ends hold
/// the same summary, both carriers, both initiator roles.
async fn assert_shard_drift_contained(carrier: Carrier, drift: Drift, reason_prefix: &str) {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &three_small_files());

        let source: Arc<dyn TransferSource> = Arc::new(ShardDriftSource {
            inner: FsTransferSource::new(src_root.clone()),
            drifts: HashMap::from([("drift.txt", drift)]),
            applied: Mutex::new(false),
        });
        let (sr, dr) = run_with(
            open_for(initiator_role, carrier),
            carrier,
            source,
            dst_root.clone(),
        )
        .await;
        let summary = sr.unwrap_or_else(|e| {
            panic!("source must complete ({carrier:?}, {drift:?}, init {initiator_role:?}): {e:#}")
        });
        let dest = dr.unwrap_or_else(|e| {
            panic!("destination must complete ({carrier:?}, {drift:?}, init {initiator_role:?}): {e:#}")
        });
        assert_eq!(
            summary, dest.summary,
            "both ends agree ({carrier:?}, {drift:?})"
        );
        assert_eq!(
            summary.in_stream_carrier_used,
            carrier == Carrier::InStream,
            "the fixture must ride the carrier under test"
        );
        assert_eq!(
            summary.files_failed, 1,
            "exactly the drifted member fails ({drift:?})"
        );
        assert_eq!(
            summary.files_transferred, 2,
            "its shard-mates land ({drift:?})"
        );
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].relative_path, "drift.txt");
        assert!(
            summary.failures[0].reason.starts_with(reason_prefix),
            "reason must be the source's ({carrier:?}, {drift:?}): {}",
            summary.failures[0].reason
        );
        let landed = collect_tree(&dst_root);
        assert_eq!(
            landed.keys().collect::<Vec<_>>(),
            vec!["a.txt", "sub/c.txt"],
            "nothing of the drifted member lands ({carrier:?}, {drift:?})"
        );
        assert_eq!(landed["a.txt"], patterned(SMALL, 11));
        assert_eq!(landed["sub/c.txt"], patterned(SMALL, 13));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_shard_member_that_grew_is_skipped_and_its_mates_land() {
    assert_shard_drift_contained(
        Carrier::InStream,
        Drift::Grow(1_000),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_shard_member_that_grew_is_skipped_and_its_mates_land() {
    assert_shard_drift_contained(
        Carrier::DataPlane,
        Drift::Grow(1_000),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_shard_member_that_shrank_is_skipped_and_its_mates_land() {
    assert_shard_drift_contained(
        Carrier::InStream,
        Drift::Truncate(100),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_shard_member_that_shrank_is_skipped_and_its_mates_land() {
    assert_shard_drift_contained(
        Carrier::DataPlane,
        Drift::Truncate(100),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_shard_member_that_vanished_is_skipped_and_its_mates_land() {
    assert_shard_drift_contained(Carrier::InStream, Drift::Vanish, "source: cannot open:").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_shard_member_that_vanished_is_skipped_and_its_mates_land() {
    assert_shard_drift_contained(Carrier::DataPlane, Drift::Vanish, "source: cannot open:").await;
}

/// A shard whose every member drifted sends only skips — no shard record
/// at all — and the session still completes with every member reported.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fully_skipped_shard_sends_only_skips_and_the_session_completes() {
    for carrier in [Carrier::InStream, Carrier::DataPlane] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &three_small_files());
        let source: Arc<dyn TransferSource> = Arc::new(ShardDriftSource {
            inner: FsTransferSource::new(src_root.clone()),
            drifts: HashMap::from([
                ("a.txt", Drift::Vanish),
                ("drift.txt", Drift::Grow(7)),
                ("sub/c.txt", Drift::Truncate(1)),
            ]),
            applied: Mutex::new(false),
        });
        let (sr, dr) = run_with(
            open_for(TransferRole::Source, carrier),
            carrier,
            source,
            dst_root.clone(),
        )
        .await;
        let summary = sr.unwrap_or_else(|e| panic!("source must complete ({carrier:?}): {e:#}"));
        let dest = dr.unwrap_or_else(|e| panic!("destination must complete ({carrier:?}): {e:#}"));
        assert_eq!(summary, dest.summary);
        assert_eq!(
            summary.files_failed, 3,
            "every member is reported ({carrier:?})"
        );
        assert_eq!(summary.files_transferred, 0);
        let mut failed: Vec<&str> = summary
            .failures
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        failed.sort();
        assert_eq!(failed, vec!["a.txt", "drift.txt", "sub/c.txt"]);
        assert!(
            collect_tree(&dst_root).is_empty(),
            "nothing lands from a fully-skipped shard ({carrier:?})"
        );
    }
}
