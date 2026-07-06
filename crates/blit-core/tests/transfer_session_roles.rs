//! Role-parameterized session suite (otp-3).
//!
//! Every fixture runs under BOTH role assignments — the initiator as
//! SOURCE (push-shaped) and the initiator as DESTINATION (pull-shaped)
//! — over the in-process transport, and the outcomes must be
//! IDENTICAL: same need-list set, same summary counts, same bytes on
//! disk. This is the owner's invariance requirement
//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1) in its first
//! executable form: there is no per-direction code to diverge, and
//! this suite pins that the one code path really is
//! initiator-indifferent.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use blit_core::generated::transfer_frame::Frame;
use blit_core::generated::{
    session_error, ComparisonMode, FileHeader, FilterSpec, ManifestComplete, NeedBatch,
    NeedComplete, NeedEntry, SessionHello, SessionOpen, TransferFrame, TransferRole,
    TransferSummary,
};
use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
use blit_core::remote::transfer::{PreparedPayload, TransferPayload};
use blit_core::transfer_plan::PlanOptions;
use blit_core::transfer_session::transport::{in_process_pair, FrameTransport};
use blit_core::transfer_session::{
    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
    HelloConfig, SessionEndpoint, SessionFault, SourceSessionConfig, CONTRACT_VERSION,
};

const SUITE_TIMEOUT: Duration = Duration::from_secs(120);

/// (relative path, content, mtime seconds). Fixture mtimes are fixed
/// epochs so both role-assignment runs see byte-for-byte identical
/// trees.
type FileSpec = (&'static str, Vec<u8>, i64);

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

/// Every regular file under `root` as rel-path → bytes.
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

fn assert_trees_identical(src: &Path, dst: &Path) {
    let src_tree = collect_tree(src);
    let dst_tree = collect_tree(dst);
    assert_eq!(
        src_tree.keys().collect::<Vec<_>>(),
        dst_tree.keys().collect::<Vec<_>>(),
        "path sets differ between {src:?} and {dst:?}"
    );
    for (rel, bytes) in &src_tree {
        assert_eq!(
            bytes, &dst_tree[rel],
            "content differs for '{rel}' between {src:?} and {dst:?}"
        );
    }
}

fn basic_open(initiator_role: TransferRole) -> SessionOpen {
    SessionOpen {
        initiator_role: initiator_role as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: true,
        ..Default::default()
    }
}

/// Drive one full session between `src_root` and `dst_root` with the
/// given end acting as initiator. Data direction is FIXED
/// (src_root → dst_root); the parameter only swaps which end opens
/// the session — the thing the owner's invariant says must not
/// matter.
async fn run_session(
    initiator_role: TransferRole,
    src_root: &Path,
    dst_root: &Path,
    plan_options: PlanOptions,
) -> (
    eyre::Result<TransferSummary>,
    eyre::Result<DestinationOutcome>,
) {
    run_session_with_open(basic_open(initiator_role), src_root, dst_root, plan_options).await
}

/// Like [`run_session`] but with a caller-supplied open, so a fixture
/// can exercise filter/mirror fields. The initiator role is read from
/// the open itself.
async fn run_session_with_open(
    open: SessionOpen,
    src_root: &Path,
    dst_root: &Path,
    plan_options: PlanOptions,
) -> (
    eyre::Result<TransferSummary>,
    eyre::Result<DestinationOutcome>,
) {
    let initiator_role = TransferRole::try_from(open.initiator_role)
        .unwrap_or_else(|_| panic!("open carries a valid initiator role"));
    let (source_endpoint, dest_endpoint) = match initiator_role {
        TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
        TransferRole::Destination => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
        TransferRole::Unspecified => panic!("fixture must pick a role"),
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: source_endpoint,
        plan_options,
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: dest_endpoint,
        data_plane_host: None,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.to_path_buf()));
    tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(
                dest_cfg,
                b,
                DestinationTarget::Fixed(dst_root.to_path_buf())
            ),
        )
    })
    .await
    .expect("session run timed out")
}

/// Run the same fixture under both role assignments (fresh trees per
/// run) and pin the invariance property: identical need sets,
/// identical summaries, byte-identical destinations.
async fn assert_invariant_across_roles(
    src_files: &[FileSpec],
    dst_files: &[FileSpec],
    plan_options: PlanOptions,
) -> (TransferSummary, Vec<String>) {
    let mut per_role: Vec<(TransferSummary, Vec<String>)> = Vec::new();
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, src_files);
        write_tree(&dst_root, dst_files);

        let (source_result, dest_result) =
            run_session(initiator_role, &src_root, &dst_root, plan_options).await;
        let source_summary = source_result
            .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
        let dest_outcome = dest_result.unwrap_or_else(|e| {
            panic!("destination failed under initiator {initiator_role:?}: {e:#}")
        });

        assert_eq!(
            source_summary, dest_outcome.summary,
            "both ends must hold the same summary (initiator {initiator_role:?})"
        );
        assert!(
            source_summary.in_stream_carrier_used,
            "otp-3 sessions ride the in-stream carrier"
        );
        assert_trees_identical(&src_root, &dst_root);

        let mut needed = dest_outcome.needed_paths.clone();
        needed.sort();
        per_role.push((dest_outcome.summary, needed));
    }

    let (summary_a, needed_a) = per_role.remove(0);
    let (summary_b, needed_b) = per_role.remove(0);
    assert_eq!(
        needed_a, needed_b,
        "need-list set must be identical whichever end initiates"
    );
    assert_eq!(
        summary_a, summary_b,
        "summary must be identical whichever end initiates"
    );
    (summary_a, needed_a)
}

fn fault_of(err: &eyre::Report) -> &SessionFault {
    err.downcast_ref::<SessionFault>()
        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Mixed small tree: nested dirs, an empty file, a name with spaces,
/// and a file larger than the in-stream chunk so file records span
/// multiple FileData frames.
fn small_tree() -> Vec<FileSpec> {
    vec![
        ("a.txt", b"alpha".to_vec(), 1_600_000_001),
        ("empty.bin", Vec::new(), 1_600_000_002),
        ("dir one/b.log", vec![0xAB; 4096], 1_600_000_003),
        (
            "dir one/deeper/c.dat",
            b"gamma-content".to_vec(),
            1_600_000_004,
        ),
        // 3 MiB + 17 so the record needs 4 FileData frames and ends
        // on a partial chunk.
        (
            "big/blob.bin",
            make_patterned(3 * 1024 * 1024 + 17),
            1_600_000_005,
        ),
    ]
}

fn make_patterned(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

#[tokio::test]
async fn small_tree_byte_identical_under_both_initiators() {
    let src = small_tree();
    let (summary, needed) = assert_invariant_across_roles(&src, &[], PlanOptions::default()).await;
    assert_eq!(summary.files_transferred, src.len() as u64);
    assert_eq!(
        summary.bytes_transferred,
        src.iter().map(|(_, c, _)| c.len() as u64).sum::<u64>()
    );
    assert_eq!(summary.entries_deleted, 0);
    assert_eq!(summary.files_resumed, 0);
    assert_eq!(
        needed.len(),
        src.len(),
        "empty destination needs everything"
    );
}

#[tokio::test]
async fn tiny_file_tree_tar_shard_records_under_both_initiators() {
    // 200 tiny files under nested dirs; force_tar makes the planner's
    // tar-shard choice deterministic so the tar record grammar
    // (header + chunks + complete → tar-safety unpack) is exercised
    // under both role assignments.
    let mut src: Vec<FileSpec> = Vec::new();
    let names: Vec<String> = (0..200)
        .map(|i| format!("shards/d{}/f{:03}.txt", i % 7, i))
        .collect();
    let leaked: Vec<&'static str> = names
        .into_iter()
        .map(|n| Box::leak(n.into_boxed_str()) as &'static str)
        .collect();
    for (i, name) in leaked.iter().enumerate() {
        src.push((
            name,
            format!("tiny-{i}").into_bytes(),
            1_600_100_000 + i as i64,
        ));
    }
    let plan = PlanOptions {
        force_tar: true,
        ..PlanOptions::default()
    };
    let (summary, needed) = assert_invariant_across_roles(&src, &[], plan).await;
    assert_eq!(summary.files_transferred, 200);
    assert_eq!(needed.len(), 200);
}

#[tokio::test]
async fn incremental_transfer_needs_only_missing_and_changed() {
    let src: Vec<FileSpec> = vec![
        // Identical on both sides (same size, same mtime) → skipped.
        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
        // Same size, source newer → transferred.
        ("newer.txt", b"NEW-eight".to_vec(), 1_600_000_200),
        // Absent on destination → transferred.
        ("sub/missing.txt", b"fresh".to_vec(), 1_600_000_300),
    ];
    let dst: Vec<FileSpec> = vec![
        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
        ("newer.txt", b"old-eight".to_vec(), 1_600_000_100),
    ];
    let (summary, needed) = assert_invariant_across_roles(&src, &dst, PlanOptions::default()).await;
    assert_eq!(
        needed,
        vec!["newer.txt".to_string(), "sub/missing.txt".to_string()],
        "need list must be exactly the changed + missing files"
    );
    assert_eq!(summary.files_transferred, 2);
    assert_eq!(summary.bytes_transferred, 9 + 5);
}

#[tokio::test]
async fn preexisting_identical_tree_yields_empty_need_list() {
    let files: Vec<FileSpec> = vec![
        ("one.txt", b"matching".to_vec(), 1_600_000_400),
        ("nested/two.txt", b"also matching".to_vec(), 1_600_000_500),
    ];
    let (summary, needed) =
        assert_invariant_across_roles(&files, &files, PlanOptions::default()).await;
    assert!(needed.is_empty(), "identical trees must need nothing");
    assert_eq!(summary.files_transferred, 0);
    assert_eq!(summary.bytes_transferred, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
    // sf-2 pin ported onto the unified session (otp-4b-2). The responder
    // grants the zero-knowledge single stream (no manifest seen at
    // SessionAccept); a 10k-tiny-file transfer over the TCP data plane
    // must re-run the shape table over the accumulated need list and grow
    // the stream count past 1 via `DataPlaneResize{ADD}`. Mirrors the old
    // push sf-2 pin (`shape_resize_e2e.rs`), now on the session: the
    // settled count is read from the destination's `data_plane_streams`.
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    const FILE_COUNT: usize = 10_000;
    for i in 0..FILE_COUNT {
        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
    }

    // SOURCE initiator over the TCP data plane: the control lane rides the
    // in-process transport; the data-plane sockets ride loopback TCP (the
    // responder binds 0.0.0.0:0 and the source dials 127.0.0.1).
    let open = SessionOpen {
        initiator_role: TransferRole::Source as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: false,
        ..Default::default()
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: Some("127.0.0.1".into()),
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.clone()));
    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
        )
    })
    .await
    .expect("session run timed out");

    let summary = source_result.expect("source succeeds");
    let outcome = dest_result.expect("destination succeeds");
    assert!(
        !summary.in_stream_carrier_used,
        "the sf-2 pin must ride the TCP data plane"
    );
    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
    let streams = outcome
        .data_plane_streams
        .expect("data plane ran, stream count recorded");
    assert!(
        streams > 1,
        "a {FILE_COUNT}-file transfer must correct the single-stream grant \
         upward via shape resize; settled at {streams}"
    );
    assert_trees_identical(&src_root, &dst_root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_data_plane_single_stream_lands_bytes() {
    // otp-5b-1: the transport/role decoupling in the PULL direction — the
    // mirror of the push data-plane test above. Here the DESTINATION is the
    // *initiator* (dials + receives) and the SOURCE is the *responder*
    // (binds + accepts + sends). Control frames ride the in-process
    // transport; the data-plane socket rides loopback TCP (the SOURCE
    // responder binds 0.0.0.0:0, the DESTINATION initiator dials
    // 127.0.0.1). Single-stream because this 4-file tree's shape wants only
    // one stream — the pull data plane CAN resize (otp-5b-2), but a small
    // need list never crosses the shape threshold; the resize itself is
    // pinned by `pull_data_plane_shape_corrects_to_more_than_one_stream`.
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(
        &src_root,
        &[
            ("a.txt", b"alpha".to_vec(), 1_600_000_001),
            ("empty.bin", b"".to_vec(), 1_600_000_002),
            ("dir/b.log", b"beta beta beta".to_vec(), 1_600_000_003),
            ("dir/deep/c.dat", b"gamma-content".to_vec(), 1_600_000_004),
        ],
    );

    // DESTINATION initiator; SOURCE responder — the roles flipped from the
    // push data-plane test, the data plane following connection role.
    let open = SessionOpen {
        initiator_role: TransferRole::Destination as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: false,
        ..Default::default()
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
        plan_options: PlanOptions::default(),
        data_plane_host: None, // a responder never dials
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open), // dials + receives
        data_plane_host: Some("127.0.0.1".into()),
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.clone()));
    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
        )
    })
    .await
    .expect("session run timed out");

    let summary = source_result.expect("source responder succeeds");
    let outcome = dest_result.expect("destination initiator succeeds");
    assert!(
        !summary.in_stream_carrier_used,
        "the pull data plane must ride TCP, not the in-stream carrier"
    );
    assert_eq!(
        summary, outcome.summary,
        "both ends must hold the same summary"
    );
    assert_eq!(outcome.summary.files_transferred, 4);
    assert_eq!(
        outcome.data_plane_streams,
        Some(1),
        "a 4-file need list stays single-stream (below the shape threshold)"
    );
    assert_trees_identical(&src_root, &dst_root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_data_plane_shape_corrects_to_more_than_one_stream() {
    // otp-5b-2: the sf-2 shape correction in the PULL direction — the
    // mirror of `many_tiny_files_shape_correct_to_more_than_one_stream`
    // (push). Here the DESTINATION is the *initiator* (dials the epoch-N
    // sockets it grows to) and the SOURCE is the *responder* (accepts them
    // off its bound listener). The control-lane `DataPlaneResize{ADD}` /
    // `DataPlaneResizeAck` frames are identical to push; only the transport
    // action flips. A 10k-tiny-file transfer must re-run the shape table
    // over the accumulated need list and grow the stream count past 1.
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    const FILE_COUNT: usize = 10_000;
    for i in 0..FILE_COUNT {
        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
    }

    // DESTINATION initiator; SOURCE responder — roles flipped from the push
    // shape test, the data plane following connection role.
    let open = SessionOpen {
        initiator_role: TransferRole::Destination as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: false,
        ..Default::default()
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
        plan_options: PlanOptions::default(),
        data_plane_host: None, // a responder never dials
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open), // dials + receives
        data_plane_host: Some("127.0.0.1".into()),
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.clone()));
    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
        )
    })
    .await
    .expect("session run timed out");

    let summary = source_result.expect("source responder succeeds");
    let outcome = dest_result.expect("destination initiator succeeds");
    assert!(
        !summary.in_stream_carrier_used,
        "the pull sf-2 pin must ride the TCP data plane"
    );
    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
    let streams = outcome
        .data_plane_streams
        .expect("data plane ran, stream count recorded");
    assert!(
        streams > 1,
        "a {FILE_COUNT}-file PULL transfer must correct the single-stream \
         grant upward via shape resize; settled at {streams}"
    );
    assert_trees_identical(&src_root, &dst_root);
}

#[tokio::test]
async fn preserves_mtime_on_streamed_files() {
    // Not part of the role matrix — pins that the file-record write
    // path applies the manifest mtime (parity with today's receive
    // paths, which the byte-identical asserts alone wouldn't catch).
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(
        &src_root,
        &[("stamped.txt", b"stamp me".to_vec(), 1_555_555_555)],
    );

    let (source_result, dest_result) = run_session(
        TransferRole::Source,
        &src_root,
        &dst_root,
        PlanOptions::default(),
    )
    .await;
    source_result.unwrap();
    dest_result.unwrap();

    let meta = std::fs::metadata(dst_root.join("stamped.txt")).unwrap();
    let mtime = filetime::FileTime::from_last_modification_time(&meta);
    assert_eq!(mtime.unix_seconds(), 1_555_555_555);
}

// ---------------------------------------------------------------------------
// Handshake refusals
// ---------------------------------------------------------------------------

#[tokio::test]
async fn build_mismatch_refused_under_both_initiators() {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();

        let open = basic_open(initiator_role);
        let (source_endpoint, dest_endpoint) = match initiator_role {
            TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
            _ => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
        };
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig {
                build_id: "0.1.0+aaaaaaaaaaaa".into(),
                contract_version: CONTRACT_VERSION,
            },
            endpoint: source_endpoint,
            plan_options: PlanOptions::default(),
            data_plane_host: None,
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig {
                build_id: "0.1.0+bbbbbbbbbbbb".into(),
                contract_version: CONTRACT_VERSION,
            },
            endpoint: dest_endpoint,
            data_plane_host: None,
        };
        let (a, b) = in_process_pair();
        let source = Arc::new(FsTransferSource::new(src_root.clone()));
        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
            tokio::join!(
                run_source(source_cfg, a, source),
                run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
            )
        })
        .await
        .unwrap();

        for (end, err) in [
            ("source", source_result.unwrap_err()),
            ("destination", dest_result.err().unwrap()),
        ] {
            let fault = fault_of(&err);
            assert_eq!(
                fault.code,
                session_error::Code::BuildMismatch,
                "{end} must refuse with BUILD_MISMATCH (initiator {initiator_role:?})"
            );
            assert!(
                fault.message.contains("aaaaaaaaaaaa") && fault.message.contains("bbbbbbbbbbbb"),
                "{end} must name both build ids, got: {}",
                fault.message
            );
        }
        assert!(
            collect_tree(&dst_root).is_empty(),
            "no bytes may move on a refused handshake"
        );
    }
}

#[tokio::test]
async fn contract_version_mismatch_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();

    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig {
            build_id: HelloConfig::default().build_id,
            contract_version: CONTRACT_VERSION + 1,
        },
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root));
    let (source_result, dest_result) = tokio::join!(
        run_source(source_cfg, a, source),
        run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
    );
    assert_eq!(
        fault_of(&source_result.unwrap_err()).code,
        session_error::Code::BuildMismatch
    );
    assert_eq!(
        fault_of(&dest_result.err().unwrap()).code,
        session_error::Code::BuildMismatch
    );
}

#[tokio::test]
async fn mirror_request_is_refused_until_its_slice_lands() {
    // otp-3 refuses what it does not implement rather than silently
    // ignoring it: a mirror-enabled open must fail the session at the
    // OPEN phase, from the destination (the end that would execute
    // deletions).
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();

    let mut open = basic_open(TransferRole::Source);
    open.mirror_enabled = true;
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root));
    let (source_result, dest_result) = tokio::join!(
        run_source(source_cfg, a, source),
        run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
    );
    let source_fault = fault_of(&source_result.unwrap_err()).clone();
    assert_eq!(source_fault.code, session_error::Code::Internal);
    assert!(
        source_fault.message.contains("otp-6"),
        "refusal must say when mirror lands, got: {}",
        source_fault.message
    );
    assert!(dest_result.is_err());
}

#[tokio::test]
async fn source_filter_limits_manifest_under_both_initiators() {
    // otp-6a: an include filter on the open restricts the source scan to
    // matching files; non-matching files are neither manifested nor
    // transferred, whichever end initiates. `*.txt` matches by basename,
    // so the nested keep2.txt is included and the .log / .bin are not.
    let src = vec![
        ("keep.txt", b"a".to_vec(), 1_600_000_001),
        ("drop.log", b"b".to_vec(), 1_600_000_002),
        ("dir/keep2.txt", b"c".to_vec(), 1_600_000_003),
        ("dir/skip.bin", b"d".to_vec(), 1_600_000_004),
    ];
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &src);

        let mut open = basic_open(initiator_role);
        open.filter = Some(FilterSpec {
            include: vec!["*.txt".to_string()],
            ..Default::default()
        });
        let (source_result, dest_result) =
            run_session_with_open(open, &src_root, &dst_root, PlanOptions::default()).await;
        let summary = source_result
            .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
        let dest = dest_result.unwrap_or_else(|e| {
            panic!("destination failed under initiator {initiator_role:?}: {e:#}")
        });

        assert_eq!(
            summary, dest.summary,
            "both ends agree (init {initiator_role:?})"
        );
        assert_eq!(
            summary.files_transferred, 2,
            "only the two .txt files (init {initiator_role:?})"
        );
        let mut needed = dest.needed_paths.clone();
        needed.sort();
        assert_eq!(
            needed,
            vec!["dir/keep2.txt".to_string(), "keep.txt".to_string()],
            "need list is the filtered set (init {initiator_role:?})"
        );
        assert!(dst_root.join("keep.txt").exists());
        assert!(dst_root.join("dir/keep2.txt").exists());
        assert!(
            !dst_root.join("drop.log").exists(),
            "filtered-out file must not transfer (init {initiator_role:?})"
        );
        assert!(!dst_root.join("dir/skip.bin").exists());
    }
}

/// A source that delegates everything to an inner `FsTransferSource` but
/// deliberately IGNORES the `scan` filter argument (calls `inner.scan(None)`)
/// — exactly how the real `RemoteTransferSource` behaves. Used to prove the
/// session applies filters via the universal `FilteredSource` chokepoint, not
/// via the per-impl `scan(filter)` arg (codex otp-6a F1). If the session ever
/// reverts to threading the filter through `scan`, this source drops it and
/// every file transfers.
struct FilterIgnoringSource {
    inner: FsTransferSource,
}

#[async_trait::async_trait]
impl TransferSource for FilterIgnoringSource {
    fn scan(
        &self,
        _filter: Option<blit_core::fs_enum::FileFilter>,
        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
    ) -> (
        tokio::sync::mpsc::Receiver<FileHeader>,
        tokio::task::JoinHandle<eyre::Result<u64>>,
    ) {
        // The filter arg is discarded on purpose (models RemoteTransferSource).
        self.inner.scan(None, unreadable)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> eyre::Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
    ) -> eyre::Result<Vec<FileHeader>> {
        self.inner.check_availability(headers, unreadable).await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

#[tokio::test]
async fn session_filters_via_chokepoint_not_scan_arg() {
    // otp-6a F1 (codex): filtering must not depend on the inner source
    // honoring the scan(filter) argument — RemoteTransferSource ignores it.
    // Drive a push session whose source ignores the scan arg; the filter
    // must still apply because the session wraps it in FilteredSource.
    let src = vec![
        ("keep.txt", b"a".to_vec(), 1_600_000_001),
        ("drop.log", b"b".to_vec(), 1_600_000_002),
    ];
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(&src_root, &src);

    let mut open = basic_open(TransferRole::Source);
    open.filter = Some(FilterSpec {
        include: vec!["*.txt".to_string()],
        ..Default::default()
    });
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FilterIgnoringSource {
        inner: FsTransferSource::new(src_root.clone()),
    });
    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
        )
    })
    .await
    .expect("session run timed out");

    let summary = source_result.expect("source session");
    let _ = dest_result.expect("destination session");
    assert_eq!(
        summary.files_transferred, 1,
        "filter must apply via the FilteredSource chokepoint even when the \
         inner source ignores the scan arg"
    );
    assert!(dst_root.join("keep.txt").exists());
    assert!(!dst_root.join("drop.log").exists());
}

// ---------------------------------------------------------------------------
// Protocol-violation fail-fast (scripted peer)
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

#[tokio::test]
async fn payload_record_before_manifest_complete_is_protocol_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();

    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
    };
    let (mut peer, dest_transport) = in_process_pair();
    let dest = tokio::spawn(run_destination(
        dest_cfg,
        dest_transport,
        DestinationTarget::Fixed(dst_root),
    ));

    // Scripted source peer: valid handshake, then a payload record
    // while its manifest is still open — the contract's example
    // violation ("payload records may begin only AFTER the source's
    // ManifestComplete").
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
        .await
        .unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));

    let header = FileHeader {
        relative_path: "early.bin".into(),
        size: 4,
        mtime_seconds: 1_600_000_000,
        permissions: 0o644,
        checksum: vec![],
    };
    peer.send(wire(Frame::ManifestEntry(header.clone())))
        .await
        .unwrap();
    peer.send(wire(Frame::FileBegin(header))).await.unwrap();

    // The destination must answer with a SessionError frame naming
    // the violation...
    let refusal = loop {
        match recv_or_panic(&mut peer).await {
            Frame::Error(e) => break e,
            // need batches may legitimately arrive first
            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
            other => panic!("expected SessionError, got {other:?}"),
        }
    };
    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);

    // ...and its driver must fail with the same fault.
    let dest_err = dest.await.unwrap().unwrap_err();
    assert_eq!(
        fault_of(&dest_err).code,
        session_error::Code::ProtocolViolation
    );
    assert!(
        collect_tree(tmp.path()).is_empty(),
        "no bytes may land from a violating record"
    );
}

#[tokio::test]
async fn need_for_unknown_path_faults_the_source() {
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    std::fs::create_dir_all(&src_root).unwrap();
    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);

    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let (source_transport, mut peer) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root));
    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));

    // Scripted destination peer: valid handshake, then a need for a
    // path that was never manifested.
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
    peer.send(wire(Frame::Accept(Default::default())))
        .await
        .unwrap();
    loop {
        match recv_or_panic(&mut peer).await {
            Frame::ManifestEntry(_) => continue,
            Frame::ManifestComplete(_) => break,
            other => panic!("expected manifest stream, got {other:?}"),
        }
    }
    peer.send(wire(Frame::NeedBatch(NeedBatch {
        entries: vec![NeedEntry {
            relative_path: "never-manifested.txt".into(),
            resume: false,
        }],
    })))
    .await
    .unwrap();

    let source_err = source_task.await.unwrap().unwrap_err();
    let fault = fault_of(&source_err);
    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
    assert!(fault.message.contains("never-manifested.txt"));

    // The source must have told the peer why before aborting.
    let refusal = match recv_or_panic(&mut peer).await {
        Frame::Error(e) => e,
        other => panic!("expected SessionError, got {other:?}"),
    };
    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
}

#[tokio::test]
async fn resume_flagged_need_is_refused_in_non_resume_session() {
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    std::fs::create_dir_all(&src_root).unwrap();
    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);

    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let (source_transport, mut peer) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root));
    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));

    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
    peer.send(wire(Frame::Accept(Default::default())))
        .await
        .unwrap();
    loop {
        match recv_or_panic(&mut peer).await {
            Frame::ManifestEntry(_) => continue,
            Frame::ManifestComplete(_) => break,
            other => panic!("expected manifest stream, got {other:?}"),
        }
    }
    peer.send(wire(Frame::NeedBatch(NeedBatch {
        entries: vec![NeedEntry {
            relative_path: "real.txt".into(),
            resume: true,
        }],
    })))
    .await
    .unwrap();

    let source_err = source_task.await.unwrap().unwrap_err();
    assert_eq!(
        fault_of(&source_err).code,
        session_error::Code::ProtocolViolation
    );
}

#[tokio::test]
async fn need_complete_before_manifest_complete_faults_the_source() {
    // codex otp-3 F2: NeedComplete is only legal after the source's
    // ManifestComplete has been received (contract §Phase state
    // machine). A peer promising "nothing further needed" before it
    // could have seen the full manifest must fail the session fast,
    // not end it as an empty transfer. The 500-entry manifest plus a
    // peer that reads nothing until after its early NeedComplete
    // keeps the source provably mid-manifest (64-frame transport
    // cap) when the violation is processed.
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    std::fs::create_dir_all(&src_root).unwrap();
    let mut files: Vec<FileSpec> = Vec::new();
    for i in 0..500 {
        let name: &'static str = Box::leak(format!("f{i:03}.txt").into_boxed_str());
        files.push((name, b"x".to_vec(), 1_600_000_000 + i as i64));
    }
    write_tree(&src_root, &files);

    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let (source_transport, mut peer) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root));
    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));

    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
    peer.send(wire(Frame::Accept(Default::default())))
        .await
        .unwrap();
    // The violation: promise need-completion before reading a single
    // manifest frame.
    peer.send(wire(Frame::NeedComplete(NeedComplete {})))
        .await
        .unwrap();

    // The source must abort with a SessionError before its manifest
    // completes — never treat the early promise as a clean empty
    // transfer.
    let refusal = loop {
        match recv_or_panic(&mut peer).await {
            Frame::ManifestEntry(_) => continue,
            Frame::Error(e) => break e,
            Frame::ManifestComplete(_) => {
                panic!("source completed its manifest instead of failing fast")
            }
            Frame::SourceDone(_) => panic!("source treated early NeedComplete as legitimate"),
            other => panic!("expected SessionError, got {other:?}"),
        }
    };
    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);

    let source_err = source_task.await.unwrap().unwrap_err();
    let fault = fault_of(&source_err);
    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
    assert!(
        fault.message.contains("ManifestComplete"),
        "fault must name the ordering rule, got: {}",
        fault.message
    );
}

#[tokio::test]
async fn manifest_entry_after_manifest_complete_is_protocol_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();

    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
    };
    let (mut peer, dest_transport) = in_process_pair();
    let dest = tokio::spawn(run_destination(
        dest_cfg,
        dest_transport,
        DestinationTarget::Fixed(dst_root),
    ));

    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
        .await
        .unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));

    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
        scan_complete: true,
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::ManifestEntry(FileHeader {
        relative_path: "late.txt".into(),
        size: 1,
        mtime_seconds: 1,
        permissions: 0o644,
        checksum: vec![],
    })))
    .await
    .unwrap();

    let dest_err = dest.await.unwrap().unwrap_err();
    assert_eq!(
        fault_of(&dest_err).code,
        session_error::Code::ProtocolViolation
    );
}
