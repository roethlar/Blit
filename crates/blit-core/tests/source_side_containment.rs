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
//!
//! ssc-3 (D2, D-2026-09-28-2): a file that fails AFTER its record was
//! announced — a read error, a short read, or a size that no longer
//! matches when re-checked after the body — is RETRACTED by its own
//! terminator: the destination discards the partial in place, the file
//! is reported, the session continues. The same for a resume record
//! whose source read fails mid-diff (A10): the partial stays unstamped.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blit_core::generated::transfer_frame::Frame;
use blit_core::generated::{
    session_error, FileData, FileFailure, FileHeader, FilterSpec, ManifestComplete, MirrorMode,
    RecordEnd, ResumeSettings, SessionHello, SessionOpen, SourceDone, TransferFrame, TransferRole,
    TransferSummary,
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
    /// ssc-3: the body reads `n` bytes and then the reader fails (an
    /// I/O error mid-copy). The handle's length is the manifest's, so
    /// the record is announced first and retracted mid-body.
    ReadErrorAfter(u64),
    /// ssc-3: the body ends after `n` bytes though the manifest promised
    /// more (the file shrank while being read).
    TruncateAt(u64),
    /// ssc-3: the body is intact but the handle reports `n` bytes when
    /// re-checked after it (the file was rewritten while being read).
    DriftsAfterBody(u64),
    /// cr-ssc1-5: the handle opens but its metadata cannot be read.
    StatFails,
}

/// A reader that yields its inner bytes and then fails instead of
/// reporting EOF — a source whose disk errors mid-file.
struct FailAfter {
    inner: tokio::io::Take<Box<dyn tokio::io::AsyncRead + Unpin + Send>>,
}

impl tokio::io::AsyncRead for FailAfter {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let this = self.get_mut();
        match std::pin::Pin::new(&mut this.inner).poll_read(cx, buf) {
            std::task::Poll::Ready(Ok(())) if buf.filled().len() == before => {
                std::task::Poll::Ready(Err(std::io::Error::other(
                    "Input/output error (os error 5)",
                )))
            }
            other => other,
        }
    }
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

    async fn open_file(&self, header: &FileHeader) -> eyre::Result<OpenedSourceFile> {
        match self.faults.get(header.relative_path.as_str()) {
            Some(Fault::OpenFails) => Err(eyre::eyre!(
                "The process cannot access the file because it is being used by another process. (os error 32)"
            )),
            Some(Fault::DeclaresLen(len)) => {
                let opened = self.inner.open_file(header).await?;
                Ok(OpenedSourceFile::virtual_reader(opened.into_reader(), *len))
            }
            Some(Fault::ReadErrorAfter(n)) => {
                use tokio::io::AsyncReadExt as _;
                let opened = self.inner.open_file(header).await?;
                Ok(OpenedSourceFile::virtual_reader(
                    Box::new(FailAfter {
                        inner: opened.into_reader().take(*n),
                    }),
                    header.size,
                ))
            }
            Some(Fault::TruncateAt(n)) => {
                use tokio::io::AsyncReadExt as _;
                let opened = self.inner.open_file(header).await?;
                Ok(OpenedSourceFile::virtual_reader(
                    Box::new(opened.into_reader().take(*n)),
                    header.size,
                ))
            }
            Some(Fault::DriftsAfterBody(after)) => {
                let opened = self.inner.open_file(header).await?;
                Ok(OpenedSourceFile::virtual_reader_drifting(
                    opened.into_reader(),
                    header.size,
                    *after,
                ))
            }
            Some(Fault::StatFails) => {
                let opened = self.inner.open_file(header).await?;
                Ok(OpenedSourceFile::virtual_reader_stat_fails(
                    opened.into_reader(),
                    header.size,
                ))
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

/// ssc-4 A11 (D-E): a file whose Windows-metadata hydration fails at
/// payload preparation is skipped before announcement — on both
/// carriers, under both initiators — never a pipeline/session failure.
/// The hydrator seam (`FsTransferSource::with_hydrator`) stands in for
/// the Windows read of a named stream that vanished, is denied, or
/// changed size; the reason class follows the error text.
async fn assert_hydration_skip_contained(carrier: Carrier, error_text: &str, reason_prefix: &str) {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &three_big_files());
        let text = error_text.to_string();
        let failing: blit_core::remote::transfer::payload::Hydrator =
            Arc::new(move |path: &Path, _header: &mut FileHeader| {
                if path.ends_with("locked.bin") {
                    eyre::bail!("{text}")
                }
                Ok(())
            });
        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()).with_hydrator(failing),
            faults: HashMap::new(),
        });
        let (sr, dr) = run_with(
            open_for(initiator_role, carrier),
            carrier,
            source,
            dst_root.clone(),
        )
        .await;
        let summary = sr.unwrap_or_else(|e| panic!("source must complete ({carrier:?}): {e:#}"));
        let dest = dr.unwrap_or_else(|e| panic!("destination must complete ({carrier:?}): {e:#}"));
        assert_eq!(summary, dest.summary);
        assert_eq!(summary.files_failed, 1, "{:?}", summary.failures);
        assert_eq!(summary.files_transferred, 2);
        assert_eq!(summary.failures[0].relative_path, "locked.bin");
        assert!(
            summary.failures[0].reason.starts_with(reason_prefix),
            "({carrier:?}) {}",
            summary.failures[0].reason
        );
        let landed = collect_tree(&dst_root);
        assert_eq!(
            landed.keys().collect::<Vec<_>>(),
            vec!["ok1.bin", "sub/ok2.bin"]
        );
    }
}

/// cr-ssc2-1: a SHARD member whose Windows-metadata hydration fails at
/// preparation — the member vanished after the scan, on a platform whose
/// scan populated `windows_metadata` — is a per-file skip on both
/// carriers; its shard-mates land. (Reviewer's prediction for ssc-2 on
/// Windows CI: the whole transfer aborted at `hydrate_payload_header(..)?`
/// before the packer's containment ran; ssc-4 made hydration per member.)
async fn assert_shard_member_hydration_failure_skipped(carrier: Carrier) {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        // Three small files plan as ONE tar shard.
        write_tree(
            &src_root,
            &[
                ("a.txt", patterned(4096, 1), 1_600_000_001),
                ("vanished.txt", patterned(4096, 2), 1_600_000_002),
                ("sub/c.txt", patterned(4096, 3), 1_600_000_003),
            ],
        );
        let failing: blit_core::remote::transfer::payload::Hydrator = Arc::new(
            move |path: &Path, _header: &mut FileHeader| {
                if path.ends_with("vanished.txt") {
                    // What the Windows named-stream read reports for a
                    // member deleted after the scan.
                    let _ = std::fs::remove_file(path);
                    eyre::bail!(
                        "reading Windows named streams: The system cannot find the file specified. (os error 2)"
                    )
                }
                Ok(())
            },
        );
        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()).with_hydrator(failing),
            faults: HashMap::new(),
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
        assert_eq!(summary, dest.summary);
        assert_eq!(summary.files_failed, 1, "{:?}", summary.failures);
        assert_eq!(summary.files_transferred, 2, "the shard-mates land");
        assert_eq!(summary.failures[0].relative_path, "vanished.txt");
        assert!(
            summary.failures[0].reason.starts_with("source:"),
            "({carrier:?}) {}",
            summary.failures[0].reason
        );
        let landed = collect_tree(&dst_root);
        assert_eq!(
            landed.keys().collect::<Vec<_>>(),
            vec!["a.txt", "sub/c.txt"]
        );
        assert_eq!(landed["a.txt"], patterned(4096, 1));
        assert_eq!(landed["sub/c.txt"], patterned(4096, 3));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_shard_member_hydration_failure_is_skipped_and_reported() {
    assert_shard_member_hydration_failure_skipped(Carrier::InStream).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_shard_member_hydration_failure_is_skipped_and_reported() {
    // Mutation proof: make the shard hydration loop in
    // `prepare_payload_with` propagate the error (`hydrate(..)?`, the
    // pre-ssc-4 shape) and the source's pipeline faults instead of
    // completing.
    assert_shard_member_hydration_failure_skipped(Carrier::DataPlane).await;
}

#[tokio::test]
async fn in_stream_hydration_failure_is_skipped_and_reported() {
    assert_hydration_skip_contained(
        Carrier::InStream,
        "reading Windows named stream \"meta\": Access is denied. (os error 5)",
        "source: cannot read metadata:",
    )
    .await;
}

#[tokio::test]
async fn data_plane_hydration_failure_is_skipped_and_reported() {
    assert_hydration_skip_contained(
        Carrier::DataPlane,
        "reading Windows named stream \"meta\": Access is denied. (os error 5)",
        "source: cannot read metadata:",
    )
    .await;
}

#[tokio::test]
async fn named_stream_size_drift_at_hydration_is_the_drift_class() {
    assert_hydration_skip_contained(
        Carrier::InStream,
        "Windows named stream \"meta\" on x changed size while reading: expected 4, got 9",
        "source: changed size during transfer (Windows metadata:",
    )
    .await;
}

/// ssc-4 A11 on a REAL Windows named stream (runs on Windows CI only):
/// the scan records the stream at 4 bytes; the stream is rewritten to 9
/// bytes before the payload is prepared; hydration sees the drift and
/// the file is skipped with the drift reason — the run completes.
#[cfg(windows)]
#[tokio::test]
async fn windows_named_stream_that_changed_size_after_the_scan_is_skipped() {
    struct StreamGrowSource {
        inner: FsTransferSource,
        grow: PathBuf,
        applied: Mutex<bool>,
    }
    #[async_trait::async_trait]
    impl TransferSource for StreamGrowSource {
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
                    std::fs::write(&self.grow, b"nine byte").unwrap();
                    *applied = true;
                }
            }
            self.inner.prepare_payload(payload).await
        }
        async fn open_file(&self, header: &FileHeader) -> eyre::Result<OpenedSourceFile> {
            self.inner.open_file(header).await
        }
        fn root(&self) -> &Path {
            self.inner.root()
        }
    }
    for carrier in [Carrier::InStream, Carrier::DataPlane] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &three_big_files());
        let stream_path = PathBuf::from(format!("{}:meta", src_root.join("locked.bin").display()));
        std::fs::write(&stream_path, b"four").unwrap();
        let source: Arc<dyn TransferSource> = Arc::new(StreamGrowSource {
            inner: FsTransferSource::new(src_root.clone()),
            grow: stream_path,
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
        assert_eq!(summary.files_failed, 1, "{:?}", summary.failures);
        assert_eq!(summary.failures[0].relative_path, "locked.bin");
        assert!(
            summary.failures[0]
                .reason
                .starts_with("source: changed size during transfer (Windows metadata:"),
            "{}",
            summary.failures[0].reason
        );
        assert_eq!(summary.files_transferred, 2);
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
// cr-ssc1-5: a metadata failure on the opened handle, before
// announcement, is a per-file skip on both carriers — never fatal
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_opened_handle_stat_failure_is_skipped_and_reported() {
    // Mutation proof: restore `.map_err(tag_path)?` on the pre-announce
    // `reader.len()` in `send_payload_records` and the source faults
    // instead of completing.
    assert_skip_contained(
        Carrier::InStream,
        Fault::StatFails,
        "source: cannot read metadata:",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_opened_handle_stat_failure_is_skipped_and_reported() {
    // Mutation proof: restore `.with_context(..)?` on the pre-announce
    // `file.len()` in `DataPlaneSession::send_file`.
    assert_skip_contained(
        Carrier::DataPlane,
        Fault::StatFails,
        "source: cannot read metadata:",
    )
    .await;
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
// cr-ssc1-1 — A19: a skipped source FILE whose destination is a populated
// DIRECTORY shields that directory and its contents from mirror deletion;
// unrelated extraneous entries still go
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mirror_shields_the_destination_subtree_of_a_skipped_source_file() {
    for carrier in [Carrier::InStream, Carrier::DataPlane] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        // Source: `node` is a FILE (it will be skipped); ok1.bin lands.
        write_tree(
            &src_root,
            &[
                ("node", patterned(BIG, 2), 1_600_000_002),
                ("ok1.bin", patterned(BIG, 1), 1_600_000_001),
            ],
        );
        // Destination: `node/` is a populated DIRECTORY, plus one
        // genuinely extraneous file elsewhere.
        write_tree(
            &dst_root,
            &[
                ("node/keep.txt", b"precious".to_vec(), 1_500_000_000),
                (
                    "node/deeper/also.txt",
                    b"precious too".to_vec(),
                    1_500_000_000,
                ),
                ("stale.txt", b"gone".to_vec(), 1_500_000_000),
            ],
        );
        let mut open = open_for(TransferRole::Source, carrier);
        open.mirror_enabled = true;
        open.mirror_kind = MirrorMode::All as i32;
        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()),
            faults: HashMap::from([("node", Fault::OpenFails)]),
        });
        let (sr, dr) = run_with(open, carrier, source, dst_root.clone()).await;
        let summary = sr.unwrap_or_else(|e| panic!("source must complete ({carrier:?}): {e:#}"));
        let dest = dr.unwrap_or_else(|e| panic!("destination must complete ({carrier:?}): {e:#}"));
        assert_eq!(summary, dest.summary);
        assert_eq!(summary.files_failed, 1, "node is reported ({carrier:?})");
        assert_eq!(summary.failures[0].relative_path, "node");
        assert!(
            !dst_root.join("stale.txt").exists(),
            "the unrelated extraneous entry still goes ({carrier:?})"
        );
        assert_eq!(
            std::fs::read(dst_root.join("node/keep.txt")).unwrap(),
            b"precious",
            "a skipped file's destination subtree is shielded ({carrier:?})"
        );
        assert_eq!(
            std::fs::read(dst_root.join("node/deeper/also.txt")).unwrap(),
            b"precious too",
            "every descendant is shielded ({carrier:?})"
        );
        assert_eq!(
            summary.entries_deleted, 1,
            "exactly stale.txt was deleted ({carrier:?})"
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
        ..Default::default()
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

/// cr-ssc2-3: the one-byte probe past `size` is the check that catches a
/// member that grew AFTER the stat and the bounded read — the only
/// window the stat cannot see. A member opener that reports the
/// manifest size but yields one byte more makes that window
/// deterministic: the stat passes, the `take(size)` read fills exactly,
/// and only the probe can tell.
#[test]
fn packer_probe_catches_growth_after_the_stat() {
    use blit_core::remote::transfer::{
        build_tar_shard_with, MemberOpenError, MemberOpenStage, OpenedMember,
    };

    let header = FileHeader {
        relative_path: "grows-late.txt".into(),
        size: SMALL as u64,
        mtime_seconds: 1,
        permissions: 0o644,
        ..Default::default()
    };
    let opener = |_: &Path| -> Result<OpenedMember, MemberOpenError> {
        Ok(OpenedMember {
            len: SMALL as u64,
            reader: Box::new(std::io::Cursor::new(patterned(SMALL + 1, 13))),
        })
    };
    let built = build_tar_shard_with(Path::new("/unused"), &[header], &opener).unwrap();
    assert!(
        built.headers.is_empty(),
        "a member that grew after the stat is not packed (mutation: remove the probe and it is)"
    );
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

// ---------------------------------------------------------------------------
// ssc-3 — A9: a record announced and then failed is RETRACTED by its own
// terminator (D2, D-2026-09-28-2): the partial is discarded in place
// (D5, D-2026-09-29-2), the file is reported, the session continues.
// ---------------------------------------------------------------------------

fn mtime_seconds(path: &Path) -> i64 {
    std::fs::metadata(path)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// The A9 property, one mid-body fault at a time, on `carrier` under
/// both initiator roles: the faulted file is reported with a `source:`
/// reason, its destination path is ABSENT afterwards even though a
/// stale decoy was there before the run (in-place model: overwritten,
/// then removed on abort), a decoy outside the destination root is
/// untouched, the other two files land, both ends agree.
async fn assert_retraction_contained(carrier: Carrier, fault: Fault, reason_prefix: &str) {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        write_tree(&src_root, &three_big_files());
        // A stale decoy at the faulted path (different size and mtime,
        // so the diff wants the file) and one outside the root.
        write_tree(&dst_root, &[("locked.bin", vec![0xEE; 10], 1_500_000_000)]);
        write_tree(&outside, &[("decoy.bin", vec![0xDD; 10], 1_500_000_000)]);

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
            "the retracted record leaves no file at its path ({carrier:?}, init {initiator_role:?})"
        );
        assert_eq!(landed["ok1.bin"], patterned(BIG, 1));
        assert_eq!(landed["sub/ok2.bin"], patterned(BIG, 3));
        assert_eq!(
            std::fs::read(outside.join("decoy.bin")).unwrap(),
            vec![0xDD; 10],
            "nothing outside the destination root is touched"
        );
        let gate = refuse_source_delete_on_failures(
            "src",
            summary.files_failed,
            &failures_from_wire(&summary.failures),
        );
        assert!(
            gate.is_err(),
            "move must refuse source deletion on a retraction"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_read_error_mid_body_is_retracted_and_reported() {
    assert_retraction_contained(
        Carrier::InStream,
        Fault::ReadErrorAfter((BIG / 2) as u64),
        "source: read error",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_read_error_mid_body_is_retracted_and_reported() {
    assert_retraction_contained(
        Carrier::DataPlane,
        Fault::ReadErrorAfter((BIG / 2) as u64),
        "source: read error",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_short_read_mid_body_is_retracted_and_reported() {
    assert_retraction_contained(
        Carrier::InStream,
        Fault::TruncateAt((BIG / 2) as u64),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_short_read_mid_body_is_retracted_and_reported() {
    assert_retraction_contained(
        Carrier::DataPlane,
        Fault::TruncateAt((BIG / 2) as u64),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_size_drift_after_body_is_retracted_and_reported() {
    assert_retraction_contained(
        Carrier::InStream,
        Fault::DriftsAfterBody(BIG as u64 + 7),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_size_drift_after_body_is_retracted_and_reported() {
    assert_retraction_contained(
        Carrier::DataPlane,
        Fault::DriftsAfterBody(BIG as u64 + 7),
        "source: changed size during transfer",
    )
    .await;
}

// ---------------------------------------------------------------------------
// ssc-3 — A10: a resume record whose source read fails mid-diff is closed
// FAILED — the partial stays in place and UNSTAMPED, the file is
// reported, the session continues — on both carriers.
// ---------------------------------------------------------------------------

const RESUME_BS: u32 = 64 * 1024;

fn resume_open_for(initiator_role: TransferRole, carrier: Carrier) -> SessionOpen {
    SessionOpen {
        resume: Some(ResumeSettings {
            enabled: true,
            block_size: RESUME_BS,
        }),
        ..open_for(initiator_role, carrier)
    }
}

async fn assert_resume_fault_contained(carrier: Carrier, fault: Fault, reason_prefix: &str) {
    let bs = RESUME_BS as usize;
    let content = patterned(3 * bs, 9);
    const DST_MTIME: i64 = 1_600_001_000;
    const SRC_MTIME: i64 = 1_600_001_100;
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(
            &src_root,
            &[
                ("partial.bin", content.clone(), SRC_MTIME),
                ("ok1.bin", patterned(BIG, 1), 1_600_000_001),
            ],
        );
        // Every dest block is stale, so the source sends block records
        // immediately; its reader fails inside block 2.
        write_tree(
            &dst_root,
            &[("partial.bin", vec![0x11; content.len()], DST_MTIME)],
        );
        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()),
            faults: HashMap::from([("partial.bin", fault)]),
        });
        let (sr, dr) = run_with(
            resume_open_for(initiator_role, carrier),
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
        assert_eq!(summary.files_failed, 1, "the resumed file fails once");
        assert_eq!(summary.files_resumed, 0, "a failed resume is not a resume");
        assert_eq!(summary.files_transferred, 1, "the other file lands");
        assert_eq!(summary.failures[0].relative_path, "partial.bin");
        assert!(
            summary.failures[0].reason.starts_with(reason_prefix),
            "reason must be the source's ({carrier:?}): {}",
            summary.failures[0].reason
        );
        // In-place model: block 0 landed before the fault, nothing past
        // it, and the partial is NOT stamped as converged.
        let partial = dst_root.join("partial.bin");
        let patched = std::fs::read(&partial).unwrap();
        assert_eq!(
            &patched[..bs],
            &content[..bs],
            "block 0 landed ({carrier:?})"
        );
        assert_eq!(
            patched[bs], 0x11,
            "nothing past the faulted block lands ({carrier:?})"
        );
        // The in-place block write itself bumps the OS mtime; what must
        // NOT happen is the finalisation stamp that would make the next
        // compare call this partial converged (the source's mtime).
        assert_ne!(
            mtime_seconds(&partial),
            SRC_MTIME,
            "a failed resume must not stamp the partial as converged ({carrier:?}, init {initiator_role:?})"
        );
        assert_eq!(collect_tree(&dst_root)["ok1.bin"], patterned(BIG, 1));
    }
}

/// cr-ssc1-4 / cr-ssc3-1: a resume-granted file the source cannot open,
/// or whose size no longer matches the manifest before the diff, is
/// skipped before any block record — the destination partial is left
/// exactly as it was, the file is reported, the other file lands, the
/// move gate refuses — on both carriers.
async fn assert_resume_open_failure_skipped(carrier: Carrier) {
    assert_resume_pre_diff_skip(carrier, Fault::OpenFails, "source: cannot open:").await;
}

async fn assert_resume_pre_diff_skip(carrier: Carrier, fault: Fault, reason_prefix: &str) {
    let bs = RESUME_BS as usize;
    let content = patterned(3 * bs, 9);
    const DST_MTIME: i64 = 1_600_001_000;
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(
            &src_root,
            &[
                ("partial.bin", content.clone(), 1_600_001_100),
                ("ok1.bin", patterned(BIG, 1), 1_600_000_001),
            ],
        );
        write_tree(
            &dst_root,
            &[("partial.bin", vec![0x11; content.len()], DST_MTIME)],
        );
        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()),
            faults: HashMap::from([("partial.bin", fault)]),
        });
        let (sr, dr) = run_with(
            resume_open_for(initiator_role, carrier),
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
        assert_eq!(summary.files_failed, 1, "the skipped file fails once");
        assert_eq!(summary.files_resumed, 0);
        assert_eq!(summary.files_transferred, 1, "the other file lands");
        assert_eq!(summary.failures[0].relative_path, "partial.bin");
        assert!(
            summary.failures[0].reason.starts_with(reason_prefix),
            "reason must be the source's ({carrier:?}): {}",
            summary.failures[0].reason
        );
        // `move --resume`-shaped: the source-delete gate reads this
        // summary and must refuse while the file did not land.
        assert!(
            refuse_source_delete_on_failures(
                "src",
                summary.files_failed,
                &failures_from_wire(&summary.failures),
            )
            .is_err(),
            "move must refuse source deletion ({carrier:?})"
        );
        let partial = dst_root.join("partial.bin");
        assert_eq!(
            std::fs::read(&partial).unwrap(),
            vec![0x11; content.len()],
            "no block touched the partial ({carrier:?})"
        );
        assert_eq!(
            mtime_seconds(&partial),
            DST_MTIME,
            "the partial is untouched, not stamped ({carrier:?})"
        );
        assert_eq!(collect_tree(&dst_root)["ok1.bin"], patterned(BIG, 1));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_resume_source_open_failure_is_skipped_and_reported() {
    assert_resume_open_failure_skipped(Carrier::InStream).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_resume_source_open_failure_is_skipped_and_reported() {
    // Mutation proof: restore `.await?` on `ResumeBlockDiff::open` in
    // `DataPlaneSink::write_payload`'s ResumeFile arm and the source's
    // pipeline faults instead of completing.
    assert_resume_open_failure_skipped(Carrier::DataPlane).await;
}

// ---------------------------------------------------------------------------
// cr-ssc3-1: a resumed file whose size changed — before the diff (skip)
// or while being diffed (failed record) — is never finalised short
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_resume_growth_before_the_diff_is_skipped() {
    assert_resume_pre_diff_skip(
        Carrier::InStream,
        Fault::DeclaresLen(3 * RESUME_BS as u64 + 4096),
        "source: changed size during transfer (manifest",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_resume_growth_before_the_diff_is_skipped() {
    // Mutation proof: remove the pre-diff `len()` check in
    // `ResumeBlockDiff::open` and the grown file is resumed at the
    // manifest size and stamped — this guard then fails on
    // `files_failed`.
    assert_resume_pre_diff_skip(
        Carrier::DataPlane,
        Fault::DeclaresLen(3 * RESUME_BS as u64 - 1),
        "source: changed size during transfer (manifest",
    )
    .await;
}

/// The file grew while its blocks were being diffed: every block lands
/// (they were read from the manifest-sized prefix), but the record closes
/// FAILED with the changed-size reason, the partial is NOT stamped as
/// converged, and the move gate refuses — on both carriers.
async fn assert_resume_growth_during_diff_contained(carrier: Carrier) {
    let bs = RESUME_BS as usize;
    let content = patterned(3 * bs, 9);
    const DST_MTIME: i64 = 1_600_001_000;
    const SRC_MTIME: i64 = 1_600_001_100;
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(
            &src_root,
            &[
                ("partial.bin", content.clone(), SRC_MTIME),
                ("ok1.bin", patterned(BIG, 1), 1_600_000_001),
            ],
        );
        write_tree(
            &dst_root,
            &[("partial.bin", vec![0x11; content.len()], DST_MTIME)],
        );
        let source: Arc<dyn TransferSource> = Arc::new(FaultySource {
            inner: FsTransferSource::new(src_root.clone()),
            faults: HashMap::from([(
                "partial.bin",
                Fault::DriftsAfterBody(3 * RESUME_BS as u64 + 4096),
            )]),
        });
        let (sr, dr) = run_with(
            resume_open_for(initiator_role, carrier),
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
        assert_eq!(summary.files_failed, 1, "the grown file fails once");
        assert_eq!(summary.files_resumed, 0, "a failed resume is not a resume");
        assert_eq!(summary.files_transferred, 1, "the other file lands");
        assert_eq!(summary.failures[0].relative_path, "partial.bin");
        assert!(
            summary.failures[0]
                .reason
                .starts_with("source: changed size during transfer (manifest"),
            "reason must be the drift class ({carrier:?}): {}",
            summary.failures[0].reason
        );
        let partial = dst_root.join("partial.bin");
        assert_ne!(
            mtime_seconds(&partial),
            SRC_MTIME,
            "a file that grew during the diff must not be stamped as converged ({carrier:?}, init {initiator_role:?})"
        );
        assert!(
            refuse_source_delete_on_failures(
                "src",
                summary.files_failed,
                &failures_from_wire(&summary.failures),
            )
            .is_err(),
            "move --resume must refuse source deletion ({carrier:?})"
        );
        assert_eq!(collect_tree(&dst_root)["ok1.bin"], patterned(BIG, 1));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_resume_growth_during_the_diff_is_reported_and_unstamped() {
    assert_resume_growth_during_diff_contained(Carrier::InStream).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_resume_growth_during_the_diff_is_reported_and_unstamped() {
    // Mutation proof: remove the post-diff `len()` check in
    // `ResumeBlockDiff::next_event` and the grown file is resumed at the
    // manifest size and stamped — this guard then fails on `files_failed`.
    assert_resume_growth_during_diff_contained(Carrier::DataPlane).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_resume_short_read_mid_diff_is_reported_and_unstamped() {
    assert_resume_fault_contained(
        Carrier::InStream,
        Fault::TruncateAt((RESUME_BS + RESUME_BS / 2) as u64),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_resume_short_read_mid_diff_is_reported_and_unstamped() {
    assert_resume_fault_contained(
        Carrier::DataPlane,
        Fault::TruncateAt((RESUME_BS + RESUME_BS / 2) as u64),
        "source: changed size during transfer",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_resume_read_error_mid_diff_is_reported_and_unstamped() {
    assert_resume_fault_contained(
        Carrier::InStream,
        Fault::ReadErrorAfter((RESUME_BS + RESUME_BS / 2) as u64),
        "source: read error",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_resume_read_error_mid_diff_is_reported_and_unstamped() {
    assert_resume_fault_contained(
        Carrier::DataPlane,
        Fault::ReadErrorAfter((RESUME_BS + RESUME_BS / 2) as u64),
        "source: read error",
    )
    .await;
}

// ---------------------------------------------------------------------------
// ssc-5 (D-F, D-2026-09-29-3, A13): names that are not valid UTF-8 travel as
// raw bytes beside the lossy text. A destination that can hold the bytes
// creates the real name and the file converges on re-run; one that cannot
// reports the file at manifest intake — whatever the diff would have said —
// and never creates a path by the lossy text. Two entries that collapse to
// one text path are reported, never silently merged.
// ---------------------------------------------------------------------------

/// A source whose scan rewrites chosen manifest headers on their way out
/// (duplicate them, decorate them with raw name bytes); payload paths
/// delegate to the real filesystem source.
struct ManifestRewritingSource {
    inner: FsTransferSource,
    rewrite: Arc<dyn Fn(FileHeader) -> Vec<FileHeader> + Send + Sync>,
}

#[async_trait::async_trait]
impl TransferSource for ManifestRewritingSource {
    fn scan(
        &self,
        filter: Option<blit_core::fs_enum::FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (tokio::sync::mpsc::Receiver<FileHeader>, SourceScan) {
        let (mut rx, scan) = self.inner.scan(filter, unreadable_paths);
        let (tx, out) = tokio::sync::mpsc::channel(64);
        let rewrite = Arc::clone(&self.rewrite);
        tokio::spawn(async move {
            while let Some(header) = rx.recv().await {
                for rewritten in rewrite(header) {
                    if tx.send(rewritten).await.is_err() {
                        return;
                    }
                }
            }
        });
        (out, scan)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> eyre::Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn open_file(&self, header: &FileHeader) -> eyre::Result<OpenedSourceFile> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

fn rewriting_source(
    src_root: PathBuf,
    rewrite: impl Fn(FileHeader) -> Vec<FileHeader> + Send + Sync + 'static,
) -> Arc<dyn TransferSource> {
    Arc::new(ManifestRewritingSource {
        inner: FsTransferSource::new(src_root),
        rewrite: Arc::new(rewrite),
    })
}

/// A13: a second manifest entry for a text path already seen is reported
/// as a lossy-name collision — the first wins and lands, nothing is
/// overwritten, the session completes.
async fn assert_duplicate_manifest_path_is_reported_once(carrier: Carrier) {
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(
        &src_root,
        &[
            ("dup.bin", patterned(BIG, 7), 1_600_000_011),
            ("ok.bin", patterned(BIG, 8), 1_600_000_012),
        ],
    );
    let source = rewriting_source(src_root.clone(), |header| {
        if header.relative_path == "dup.bin" {
            vec![header.clone(), header]
        } else {
            vec![header]
        }
    });
    let (summary, outcome) = run_with(
        open_for(TransferRole::Source, carrier),
        carrier,
        source,
        dst_root.clone(),
    )
    .await;
    let summary = summary.expect("session completes");
    outcome.expect("destination completes");
    assert_eq!(summary.files_failed, 1, "the duplicate is reported once");
    let failures = failures_from_wire(&summary.failures);
    assert_eq!(failures[0].relative_path, "dup.bin");
    assert!(
        failures[0]
            .reason
            .starts_with(blit_core::raw_name::DUPLICATE_MANIFEST_PATH_REASON),
        "reason: {}",
        failures[0].reason
    );
    assert_eq!(
        summary.files_transferred, 2,
        "the first header lands, so does ok.bin"
    );
    let landed = collect_tree(&dst_root);
    assert_eq!(landed.get("dup.bin").map(Vec::len), Some(BIG));
    assert_eq!(landed.get("ok.bin").map(Vec::len), Some(BIG));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_duplicate_manifest_path_is_reported_once_first_wins() {
    assert_duplicate_manifest_path_is_reported_once(Carrier::InStream).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_duplicate_manifest_path_is_reported_once_first_wins() {
    assert_duplicate_manifest_path_is_reported_once(Carrier::DataPlane).await;
}

/// The lossy text of `caf\xe9.txt` as the scan renders it.
#[cfg(not(all(unix, not(target_os = "macos"))))]
const LOSSY_NAME: &str = "caf\u{fffd}.txt";
const RAW_NAME: &[u8] = b"caf\xe9.txt";

/// A13 on a destination that cannot store raw bytes (macOS, Windows —
/// `raw_name::destination_can_store_raw_names()` is false there): the
/// entry is reported at intake with the exact reason, never granted,
/// nothing is created by the lossy text, and a pre-existing destination
/// counterpart under the lossy text survives a mirror. The destination
/// already holds an identical copy under the lossy text, so a diff would
/// have called it converged — it is reported regardless.
#[cfg(not(all(unix, not(target_os = "macos"))))]
async fn assert_unstorable_raw_name_is_reported_at_intake(carrier: Carrier) {
    assert!(!blit_core::raw_name::destination_can_store_raw_names());
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    let lossy_content = patterned(BIG, 9);
    write_tree(
        &src_root,
        &[
            (LOSSY_NAME, lossy_content.clone(), 1_600_000_021),
            ("ok.bin", patterned(BIG, 10), 1_600_000_022),
        ],
    );
    // An identical converged copy under the lossy text, plus an extraneous
    // entry the mirror must still delete.
    write_tree(
        &dst_root,
        &[
            (LOSSY_NAME, lossy_content.clone(), 1_600_000_021),
            ("extraneous.bin", patterned(1024, 11), 1_600_000_023),
        ],
    );
    let source = rewriting_source(src_root.clone(), |mut header| {
        if header.relative_path == LOSSY_NAME {
            header.raw_relative_path = Some(RAW_NAME.to_vec());
        }
        vec![header]
    });
    let mut open = open_for(TransferRole::Source, carrier);
    open.mirror_enabled = true;
    open.mirror_kind = MirrorMode::All as i32;
    let (summary, outcome) = run_with(open, carrier, source, dst_root.clone()).await;
    let summary = summary.expect("session completes");
    outcome.expect("destination completes");
    assert_eq!(summary.files_failed, 1);
    let failures = failures_from_wire(&summary.failures);
    assert_eq!(failures[0].relative_path, LOSSY_NAME);
    assert_eq!(
        failures[0].reason,
        blit_core::raw_name::DESTINATION_CANNOT_STORE_REASON,
        "exact intake reason"
    );
    assert_eq!(summary.files_transferred, 1, "only ok.bin lands");
    let landed = collect_tree(&dst_root);
    assert_eq!(landed.get("ok.bin").map(Vec::len), Some(BIG));
    assert_eq!(
        landed.get(LOSSY_NAME).cloned(),
        Some(lossy_content),
        "the counterpart under the lossy text is kept by the mirror, untouched"
    );
    assert!(
        !landed.contains_key("extraneous.bin"),
        "the mirror still deletes genuinely extraneous entries"
    );
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_stream_unstorable_raw_name_is_reported_at_intake_and_never_created() {
    assert_unstorable_raw_name_is_reported_at_intake(Carrier::InStream).await;
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn data_plane_unstorable_raw_name_is_reported_at_intake_and_never_created() {
    assert_unstorable_raw_name_is_reported_at_intake(Carrier::DataPlane).await;
}

/// Linux (and the other byte-keyed Unix filesystems): the real name
/// round-trips byte-exact on both carriers and converges on re-run; a
/// mirror keeps the raw-named counterpart.
#[cfg(all(unix, not(target_os = "macos")))]
mod raw_round_trip {
    use super::*;
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;

    fn raw_path(root: &Path) -> PathBuf {
        root.join(OsStr::from_bytes(RAW_NAME))
    }

    fn dest_names(root: &Path) -> Vec<Vec<u8>> {
        let mut names: Vec<Vec<u8>> = std::fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name().as_bytes().to_vec())
            .collect();
        names.sort();
        names
    }

    async fn assert_raw_name_round_trips(carrier: Carrier) {
        assert!(blit_core::raw_name::destination_can_store_raw_names());
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        let content = patterned(BIG, 12);
        std::fs::write(raw_path(&src_root), &content).unwrap();
        filetime::set_file_mtime(
            raw_path(&src_root),
            filetime::FileTime::from_unix_time(1_600_000_031, 0),
        )
        .unwrap();
        write_tree(&src_root, &[("ok.bin", patterned(BIG, 13), 1_600_000_032)]);
        std::fs::write(dst_root.join("extraneous.bin"), b"gone").unwrap();

        let mut open = open_for(TransferRole::Source, carrier);
        open.mirror_enabled = true;
        open.mirror_kind = MirrorMode::All as i32;
        let (summary, outcome) = run_with(
            open.clone(),
            carrier,
            Arc::new(FsTransferSource::new(src_root.clone())),
            dst_root.clone(),
        )
        .await;
        let summary = summary.expect("session completes");
        outcome.expect("destination completes");
        assert_eq!(summary.files_failed, 0, "{:?}", summary.failures);
        assert_eq!(summary.files_transferred, 2);
        assert_eq!(
            dest_names(&dst_root),
            vec![RAW_NAME.to_vec(), b"ok.bin".to_vec()],
            "the exact bytes name the destination file; nothing by the lossy text"
        );
        assert_eq!(std::fs::read(raw_path(&dst_root)).unwrap(), content);

        // Re-run: converged (nothing copied), and the mirror keeps the
        // raw-named counterpart.
        let (summary, outcome) = run_with(
            open,
            carrier,
            Arc::new(FsTransferSource::new(src_root.clone())),
            dst_root.clone(),
        )
        .await;
        let summary = summary.expect("second session completes");
        outcome.expect("destination completes");
        assert_eq!(summary.files_failed, 0);
        assert_eq!(summary.files_transferred, 0, "converged on re-run");
        assert_eq!(
            dest_names(&dst_root),
            vec![RAW_NAME.to_vec(), b"ok.bin".to_vec()]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn in_stream_raw_name_round_trips_byte_exact_and_converges() {
        assert_raw_name_round_trips(Carrier::InStream).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn data_plane_raw_name_round_trips_byte_exact_and_converges() {
        assert_raw_name_round_trips(Carrier::DataPlane).await;
    }

    /// The local route: same bytes, same convergence, through
    /// `run_local_session`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn local_raw_name_round_trips_byte_exact_and_converges() {
        use blit_core::transfer_session::{run_local_session, LocalMirrorOptions};
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        let content = patterned(BIG, 14);
        std::fs::write(raw_path(&src_root), &content).unwrap();
        std::fs::write(src_root.join("small.txt"), b"small").unwrap();
        let options = || LocalMirrorOptions {
            progress: false,
            perf_history: false,
            mirror: true,
            ..Default::default()
        };
        let first = run_local_session(&src_root, &dst_root, options())
            .await
            .expect("first local session");
        assert_eq!(first.files_failed, 0, "{:?}", first.failures);
        assert_eq!(
            dest_names(&dst_root),
            vec![RAW_NAME.to_vec(), b"small.txt".to_vec()]
        );
        assert_eq!(std::fs::read(raw_path(&dst_root)).unwrap(), content);
        let second = run_local_session(&src_root, &dst_root, options())
            .await
            .expect("second local session");
        assert_eq!(second.files_failed, 0);
        assert_eq!(second.copied_files, 0, "converged on re-run");
        assert_eq!(
            dest_names(&dst_root),
            vec![RAW_NAME.to_vec(), b"small.txt".to_vec()]
        );
    }
}

/// cr-ssc6-1 (D-I): a `files_from`-scoped scan — what a retry pass runs
/// — accounts for every requested path. One the walk cannot enumerate
/// (gone from the source, or unreadable at scan time) is the
/// destination's recorded failure with a `source:` reason, so a retry
/// pass can never silently drop a file it was asked to re-land. The
/// present requested file still lands; nothing outside the set is
/// touched.
async fn assert_scoped_scan_reports_unscanned_requests(carrier: Carrier) {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(
            &src_root,
            &[
                ("present.bin", patterned(BIG, 1), 1_600_000_001),
                ("outside.bin", patterned(BIG, 2), 1_600_000_002),
                ("sub/unreadable.bin", patterned(BIG, 3), 1_600_000_003),
            ],
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                src_root.join("sub/unreadable.bin"),
                std::fs::Permissions::from_mode(0o000),
            )
            .unwrap();
        }
        let mut open = open_for(initiator_role, carrier);
        let mut files_from = vec!["present.bin".to_string(), "gone.bin".to_string()];
        if cfg!(unix) {
            files_from.push("sub/unreadable.bin".to_string());
        }
        open.filter = Some(FilterSpec {
            files_from,
            ..Default::default()
        });
        let source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root.clone()));
        let (sr, dr) = run_with(open, carrier, source, dst_root.clone()).await;
        let summary = sr.unwrap_or_else(|e| panic!("source must complete ({carrier:?}): {e:#}"));
        let dest = dr.unwrap_or_else(|e| panic!("destination must complete ({carrier:?}): {e:#}"));
        assert_eq!(summary, dest.summary, "both ends agree ({carrier:?})");
        let expected_failed = if cfg!(unix) { 2 } else { 1 };
        assert_eq!(
            summary.files_failed, expected_failed,
            "every requested path the scan could not enumerate is a failure ({carrier:?}): {:?}",
            summary.failures
        );
        assert_eq!(
            summary.files_transferred, 1,
            "the present requested file lands"
        );
        let by_path: HashMap<_, _> = summary
            .failures
            .iter()
            .map(|f| (f.relative_path.as_str(), f.reason.as_str()))
            .collect();
        assert!(
            by_path["gone.bin"].starts_with("source: missing at retry"),
            "a path the scan never saw is missing: {:?}",
            by_path
        );
        #[cfg(unix)]
        assert!(
            by_path["sub/unreadable.bin"]
                .starts_with("source: unreadable at retry: permission denied"),
            "a path the scan could not open keeps its reason: {:?}",
            by_path
        );
        let landed = collect_tree(&dst_root);
        assert_eq!(
            landed.keys().collect::<Vec<_>>(),
            vec!["present.bin"],
            "only the requested, present file lands ({carrier:?})"
        );
        let gate = refuse_source_delete_on_failures(
            "src",
            summary.files_failed,
            &failures_from_wire(&summary.failures),
        );
        assert!(
            gate.is_err(),
            "move must refuse while a requested file did not land"
        );
    }
}

#[tokio::test]
async fn in_stream_scoped_scan_reports_requested_paths_it_cannot_enumerate() {
    assert_scoped_scan_reports_unscanned_requests(Carrier::InStream).await;
}

#[tokio::test]
async fn data_plane_scoped_scan_reports_requested_paths_it_cannot_enumerate() {
    assert_scoped_scan_reports_unscanned_requests(Carrier::DataPlane).await;
}

/// cr-fix1-2: a shard member that opened but whose handle metadata could
/// not be read is reported as a metadata failure, not as "cannot open" —
/// the reason names the actual cause.
#[test]
fn a_shard_member_stat_failure_is_reported_as_a_metadata_failure() {
    use blit_core::remote::transfer::{build_tar_shard_with, MemberOpenError, MemberOpenStage};
    let header = FileHeader {
        relative_path: "m.bin".to_string(),
        size: 16,
        ..Default::default()
    };
    let opener = |_: &Path| -> Result<OpenedMember, MemberOpenError> {
        Err(MemberOpenError {
            stage: MemberOpenStage::Stat,
            error: std::io::Error::other("fstat: input/output error"),
        })
    };
    let built = build_tar_shard_with(Path::new("/unused"), &[header], &opener).unwrap();
    assert_eq!(built.skipped.len(), 1, "{:?}", built.skipped);
    assert!(
        built.skipped[0]
            .reason
            .starts_with("source: cannot read metadata: fstat"),
        "the stat stage keeps its identity: {}",
        built.skipped[0].reason
    );
    let opener = |_: &Path| -> Result<OpenedMember, MemberOpenError> {
        Err(MemberOpenError {
            stage: MemberOpenStage::Open,
            error: std::io::Error::other("EACCES"),
        })
    };
    let built = build_tar_shard_with(
        Path::new("/unused"),
        &[FileHeader {
            relative_path: "m.bin".to_string(),
            size: 16,
            ..Default::default()
        }],
        &opener,
    )
    .unwrap();
    assert!(built.skipped[0]
        .reason
        .starts_with("source: cannot open: EACCES"));
}
