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

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blit_core::dial::DIAL_DEFAULT_STREAM_LIMIT;
use blit_core::generated::transfer_frame::Frame;
use blit_core::generated::{
    session_error, BlockHashList, CapacityProfile, ComparisonMode, FileData, FileHeader,
    FilterSpec, ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, ResumeSettings,
    SessionError, SessionHello, SessionOpen, SourceDone, TransferFrame, TransferRole,
    TransferSummary,
};
use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
use blit_core::remote::transfer::{
    PreparedPayload, ProgressEvent, RemoteTransferProgress, SessionPhaseEvent, SessionPhaseRole,
    SessionPhaseTrace, SmallFileCarrier, SmallFileProbe, SmallFileProbeReport, TimingAggregate,
    TransferPayload,
};
use blit_core::transfer_plan::PlanOptions;
use blit_core::transfer_session::transport::{in_process_pair, FrameTransport};
use blit_core::transfer_session::{
    run_destination, run_source, DestinationInstruments, DestinationOutcome,
    DestinationSessionConfig, DestinationTarget, HelloConfig, SessionEndpoint, SessionFault,
    SourceInstruments, SourceSessionConfig, CONTRACT_VERSION,
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
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: source_endpoint,
        plan_options,
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: dest_endpoint,
        data_plane_host: None,
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
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

// ---------------------------------------------------------------------------
// Resume block phase (otp-7a, docs/plan/OTP7_RESUME.md)
// ---------------------------------------------------------------------------

/// Block size for the resume fixtures: the session's floor (64 KiB,
/// `MIN_RESUME_BLOCK_SIZE`) so the open's value equals the effective
/// clamped value and byte-count expectations stay exact. Fixtures add
/// partial tail blocks deliberately.
const RESUME_BS: u32 = 64 * 1024;

fn resume_open(initiator_role: TransferRole, block_size: u32) -> SessionOpen {
    SessionOpen {
        resume: Some(ResumeSettings {
            enabled: true,
            block_size,
        }),
        ..basic_open(initiator_role)
    }
}

/// Run a resume-enabled fixture under both role assignments (fresh
/// trees per run) and pin the invariance property, exactly as
/// [`assert_invariant_across_roles`] does for plain sessions (plan D6:
/// resume runs identically whichever end initiated).
async fn assert_resume_invariant_across_roles(
    src_files: &[FileSpec],
    dst_files: &[FileSpec],
    block_size: u32,
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

        let (source_result, dest_result) = run_session_with_open(
            resume_open(initiator_role, block_size),
            &src_root,
            &dst_root,
            PlanOptions::default(),
        )
        .await;
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
            "these fixtures request the in-stream carrier (otp-7a); the \
             data-plane variants are pinned separately (otp-7b)"
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

/// Run a resume-enabled fixture over the TCP DATA PLANE under both role
/// assignments (otp-7b) and pin the invariance property. Mirrors
/// [`assert_resume_invariant_across_roles`] (in-stream) with the data
/// plane wired per connection role, exactly as the plain data-plane
/// tests below wire it: the RESPONDER binds+accepts on loopback, the
/// INITIATOR dials 127.0.0.1.
async fn assert_resume_data_plane_invariant_across_roles(
    src_files: &[FileSpec],
    dst_files: &[FileSpec],
    block_size: u32,
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

        let open = SessionOpen {
            in_stream_bytes: false,
            ..resume_open(initiator_role, block_size)
        };
        // The initiator dials the responder's loopback grant; the
        // responder never dials.
        let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
            TransferRole::Source => (
                SessionEndpoint::initiator(open),
                SessionEndpoint::Responder,
                Some("127.0.0.1".to_string()),
                None,
            ),
            TransferRole::Destination => (
                SessionEndpoint::Responder,
                SessionEndpoint::initiator(open),
                None,
                Some("127.0.0.1".to_string()),
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
        let source = Arc::new(FsTransferSource::new(src_root.clone()));
        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
            tokio::join!(
                run_source(source_cfg, a, source),
                run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
            )
        })
        .await
        .expect("session run timed out");

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
            !source_summary.in_stream_carrier_used,
            "otp-7b resume rides the TCP data plane (initiator {initiator_role:?})"
        );
        assert!(
            dest_outcome.data_plane_streams.is_some(),
            "the data plane must have run (initiator {initiator_role:?})"
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resume_over_the_data_plane_moves_only_the_changed_blocks() {
    // otp-7b guard-proof 1 over the TCP data plane: the same partial
    // fixture as the in-stream pin, but block records ride the sockets
    // as binary BLOCK/BLOCK_COMPLETE records. Only the 2 missing blocks
    // may move (pinned through `bytes_transferred`, which on the data
    // plane counts exactly the block bytes the sink applied); a plain
    // absent file rides along so file records and block records coexist
    // on the sockets. Guard: neuter the shared block-diff (send every
    // block) and this fails at 6 blocks ≠ 2; revert the grant
    // un-suppression and the in_stream_carrier_used assertion fails.
    let bs = RESUME_BS as usize;
    let content = make_patterned(6 * bs);
    let partial = content[..4 * bs].to_vec();
    let src: Vec<FileSpec> = vec![
        ("big.bin", content, 1_600_000_700),
        ("fresh.txt", b"fresh".to_vec(), 1_600_000_701),
    ];
    let dst: Vec<FileSpec> = vec![("big.bin", partial, 1_600_000_600)];

    let (summary, needed) =
        assert_resume_data_plane_invariant_across_roles(&src, &dst, RESUME_BS).await;
    assert_eq!(needed, vec!["big.bin".to_string(), "fresh.txt".to_string()]);
    assert_eq!(summary.files_transferred, 2);
    assert_eq!(summary.files_resumed, 1);
    assert_eq!(
        summary.bytes_transferred,
        (2 * bs + 5) as u64,
        "only the 2 stale blocks plus the plain file may move"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resume_data_plane_honors_block_sizes_above_the_in_stream_ceiling() {
    // codex otp-7b-1 F6 / D-2026-07-10-2 pin: the data-plane carrier's
    // block-size ceiling (64 MiB) exceeds the in-stream carrier's
    // (2 MiB). Request 4 MiB blocks over a 4 MiB file whose LAST byte
    // is stale at the dest: honored, the single 4 MiB block covers the
    // whole file and the whole file moves; an implementation wrongly
    // clamping to the in-stream ceiling would hash two 2 MiB blocks,
    // find only the second stale, and move 2 MiB instead.
    const BS: usize = 4 * 1024 * 1024;
    let content = make_patterned(BS);
    let mut stale_tail = content.clone();
    *stale_tail.last_mut().unwrap() ^= 0xFF;
    let src: Vec<FileSpec> = vec![("one-block.bin", content, 1_600_001_600)];
    let dst: Vec<FileSpec> = vec![("one-block.bin", stale_tail, 1_600_001_500)];

    let (summary, _) = assert_resume_data_plane_invariant_across_roles(&src, &dst, BS as u32).await;
    assert_eq!(summary.files_resumed, 1);
    assert_eq!(
        summary.bytes_transferred, BS as u64,
        "a 4 MiB block size (over the in-stream ceiling) must be honored \
         on the data plane: one block covers the file, so the whole file moves"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resume_over_the_data_plane_stale_partial_falls_back_to_full_content() {
    // otp-7b: the D1 stale-partial fallback holds on the data-plane
    // carrier too — a dest partial sharing NO blocks with the source
    // degrades to full content of that file, never an abort, never
    // trusted hashes. A shrunk-to-empty source rides along: zero
    // blocks, truncate-to-0 at complete (a zero-block BLOCK_COMPLETE
    // record over the socket).
    let bs = RESUME_BS as usize;
    let content = make_patterned(3 * bs + 200);
    let stale = vec![0xFFu8; content.len()];
    let src: Vec<FileSpec> = vec![
        ("swapped.bin", content.clone(), 1_600_000_900),
        ("shrunk.bin", Vec::new(), 1_600_000_901),
    ];
    let dst: Vec<FileSpec> = vec![
        ("swapped.bin", stale, 1_600_000_810),
        ("shrunk.bin", vec![0xEE; 100], 1_600_000_811),
    ];

    let (summary, needed) =
        assert_resume_data_plane_invariant_across_roles(&src, &dst, RESUME_BS).await;
    assert_eq!(
        needed,
        vec!["shrunk.bin".to_string(), "swapped.bin".to_string()]
    );
    assert_eq!(summary.files_resumed, 2);
    assert_eq!(summary.files_transferred, 2);
    assert_eq!(
        summary.bytes_transferred,
        content.len() as u64,
        "every block of the swapped file moves; the shrunk file moves none"
    );
}

#[tokio::test]
async fn resume_moves_only_the_changed_blocks() {
    // Plan guard-proof 1 (partial resume): a 6-block file whose first 4
    // blocks already landed at the dest. Only the 2 missing blocks may
    // move — pinned through `bytes_transferred`, which under the
    // in-stream carrier counts exactly the payload bytes written.
    // Guard: neuter the source block-diff (send every block) and this
    // fails at 6 blocks ≠ 2 blocks. A plain absent file rides along, so
    // block records and file records coexist in one session.
    let bs = RESUME_BS as usize;
    let content = make_patterned(6 * bs);
    let partial = content[..4 * bs].to_vec();
    let src: Vec<FileSpec> = vec![
        ("big.bin", content, 1_600_000_700),
        ("fresh.txt", b"fresh".to_vec(), 1_600_000_701),
    ];
    let dst: Vec<FileSpec> = vec![("big.bin", partial, 1_600_000_600)];

    let (summary, needed) = assert_resume_invariant_across_roles(&src, &dst, RESUME_BS).await;
    assert_eq!(needed, vec!["big.bin".to_string(), "fresh.txt".to_string()]);
    assert_eq!(summary.files_transferred, 2);
    assert_eq!(summary.files_resumed, 1);
    assert_eq!(
        summary.bytes_transferred,
        (2 * bs + 5) as u64,
        "only the 2 stale blocks plus the plain file may move"
    );
}

#[tokio::test]
async fn resume_identical_content_moves_zero_blocks_and_stamps_mtime() {
    // Plan guard-proof 2 (identical file): same bytes, dest mtime older
    // (an mtime-only touch). SizeMtime says transfer; every block hash
    // matches, so ZERO payload bytes move — yet the file still counts
    // done and BlockTransferComplete stamps the source mtime, which is
    // what makes the next run skip it. Run per role so the mtime stamp
    // can be asserted on the live dest tree.
    let bs = RESUME_BS as usize;
    let content = make_patterned(2 * bs + 123);
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(
            &src_root,
            &[("touched.bin", content.clone(), 1_600_000_800)],
        );
        write_tree(
            &dst_root,
            &[("touched.bin", content.clone(), 1_600_000_700)],
        );

        let (source_result, dest_result) = run_session_with_open(
            resume_open(initiator_role, RESUME_BS),
            &src_root,
            &dst_root,
            PlanOptions::default(),
        )
        .await;
        let summary = source_result
            .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
        let outcome = dest_result.unwrap_or_else(|e| {
            panic!("destination failed under initiator {initiator_role:?}: {e:#}")
        });
        assert_eq!(summary, outcome.summary);
        assert_eq!(summary.files_resumed, 1);
        assert_eq!(summary.files_transferred, 1);
        assert_eq!(
            summary.bytes_transferred, 0,
            "identical content must move zero block bytes (initiator {initiator_role:?})"
        );
        assert_trees_identical(&src_root, &dst_root);
        let stamped = std::fs::metadata(dst_root.join("touched.bin"))
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            stamped, 1_600_000_800,
            "BlockTransferComplete must stamp the source mtime (initiator {initiator_role:?})"
        );
    }
}

#[tokio::test]
async fn resume_stale_partial_falls_back_to_full_content() {
    // Plan guard-proof 3 (stale-partial fallback, D1/Q1): a dest partial
    // sharing NO blocks with the source degrades to a full-content
    // transfer of that file — never an abort, never trusted hashes.
    // Guard: force the source to trust the stale hashes and the
    // byte-identical assertion fails on corrupt output. A shrunk-to-empty
    // source rides along: zero blocks, truncate-to-0 at complete.
    let bs = RESUME_BS as usize;
    let content = make_patterned(3 * bs + 200);
    let stale = vec![0xFFu8; content.len()];
    let src: Vec<FileSpec> = vec![
        ("swapped.bin", content.clone(), 1_600_000_900),
        ("shrunk.bin", Vec::new(), 1_600_000_901),
    ];
    let dst: Vec<FileSpec> = vec![
        ("swapped.bin", stale, 1_600_000_810),
        ("shrunk.bin", vec![0xEE; 100], 1_600_000_811),
    ];

    let (summary, needed) = assert_resume_invariant_across_roles(&src, &dst, RESUME_BS).await;
    assert_eq!(
        needed,
        vec!["shrunk.bin".to_string(), "swapped.bin".to_string()]
    );
    assert_eq!(summary.files_resumed, 2);
    assert_eq!(summary.files_transferred, 2);
    assert_eq!(
        summary.bytes_transferred,
        content.len() as u64,
        "every block of the swapped file moves; the shrunk file moves none"
    );
}

#[tokio::test]
async fn resume_ineligible_targets_are_plain_full_transfers() {
    // Plan D2: absent and empty dest files have no partial to hash — they
    // transfer as plain full records with no resume flag, and count in
    // files_transferred but never files_resumed.
    let bs = RESUME_BS as usize;
    let src: Vec<FileSpec> = vec![
        ("absent.bin", make_patterned(2 * bs), 1_600_001_000),
        ("empty-dest.bin", make_patterned(bs + 5), 1_600_001_001),
    ];
    let dst: Vec<FileSpec> = vec![("empty-dest.bin", Vec::new(), 1_600_000_910)];

    let (summary, needed) = assert_resume_invariant_across_roles(&src, &dst, RESUME_BS).await;
    assert_eq!(
        needed,
        vec!["absent.bin".to_string(), "empty-dest.bin".to_string()]
    );
    assert_eq!(summary.files_resumed, 0);
    assert_eq!(summary.files_transferred, 2);
    assert_eq!(summary.bytes_transferred, (3 * bs + 5) as u64);
}

#[tokio::test]
async fn resume_block_size_floor_clamps_tiny_requests() {
    // codex otp-7a F1: a block_size=1 open must not hash at 1-byte
    // granularity (a 32× hash-list amplification) — the destination
    // clamps to the 64 KiB floor. Behavioral pin: a 2-block file whose
    // second block is stale moves exactly one floor-sized block; an
    // unclamped run would move a different byte count (either the tiny
    // byte-granular diff, or the whole file via the cap-overflow
    // fallback a 1-byte list triggers).
    let bs = RESUME_BS as usize; // == MIN_RESUME_BLOCK_SIZE
    let content = make_patterned(2 * bs);
    let mut stale_tail = content.clone();
    for b in &mut stale_tail[bs..] {
        *b = 0xFE;
    }
    let src: Vec<FileSpec> = vec![("floor.bin", content, 1_600_001_300)];
    let dst: Vec<FileSpec> = vec![("floor.bin", stale_tail, 1_600_001_200)];

    let (summary, _) = assert_resume_invariant_across_roles(&src, &dst, 1).await;
    assert_eq!(summary.files_resumed, 1);
    assert_eq!(
        summary.bytes_transferred, bs as u64,
        "the floor clamp must yield exactly one 64 KiB stale block"
    );
}

#[tokio::test]
async fn resume_block_size_ceiling_clamps_oversized_requests() {
    // codex otp-7a F1: a 64 MiB block_size would put a single
    // BlockTransfer frame far past tonic's 4 MiB decode limit on the
    // gRPC-served in-stream carrier — the destination clamps to the
    // 2 MiB in-stream ceiling. Behavioral pin: a 4 MiB file whose
    // second half is stale moves exactly one 2 MiB block; unclamped,
    // the whole file is one block and 4 MiB moves.
    const CEIL: usize = 2 * 1024 * 1024; // == MAX_IN_STREAM_RESUME_BLOCK_SIZE
    let content = make_patterned(2 * CEIL);
    let mut stale_tail = content.clone();
    for b in &mut stale_tail[CEIL..] {
        *b = 0xFD;
    }
    let src: Vec<FileSpec> = vec![("ceiling.bin", content, 1_600_001_400)];
    let dst: Vec<FileSpec> = vec![("ceiling.bin", stale_tail, 1_600_001_310)];

    let (summary, _) = assert_resume_invariant_across_roles(&src, &dst, 64 * 1024 * 1024).await;
    assert_eq!(summary.files_resumed, 1);
    assert_eq!(
        summary.bytes_transferred, CEIL as u64,
        "the ceiling clamp must yield exactly one 2 MiB stale block"
    );
}

#[tokio::test]
async fn file_record_for_resume_flagged_path_is_protocol_violation() {
    // codex otp-7a F3: a resume-flagged grant may be satisfied ONLY by
    // its block record. A scripted source that answers the grant with a
    // whole-file record must fail the session fast — accepting it would
    // bypass the hash choreography and report a clean summary. Also
    // pins the wire BlockHashList.block_size == the open's (in-range)
    // value.
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    let bs = RESUME_BS as usize;
    let content = make_patterned(2 * bs);
    write_tree(
        &dst_root,
        &[("partial.bin", content[..bs].to_vec(), 1_600_001_500)],
    );

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
    peer.send(wire(Frame::Open(resume_open(
        TransferRole::Source,
        RESUME_BS,
    ))))
    .await
    .unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));

    let header = FileHeader {
        relative_path: "partial.bin".into(),
        size: (2 * bs) as u64,
        mtime_seconds: 1_600_001_600,
        permissions: 0o644,
        checksum: vec![],
    };
    peer.send(wire(Frame::ManifestEntry(header.clone())))
        .await
        .unwrap();
    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
        scan_complete: true,
    })))
    .await
    .unwrap();

    // The grant must come back resume-flagged, with its hash list.
    let mut saw_resume_need = false;
    let mut saw_hashes = false;
    while !(saw_resume_need && saw_hashes) {
        match recv_or_panic(&mut peer).await {
            Frame::NeedBatch(batch) => {
                assert!(
                    batch
                        .entries
                        .iter()
                        .any(|e| e.relative_path == "partial.bin" && e.resume),
                    "the partial must be granted with resume=true"
                );
                saw_resume_need = true;
            }
            Frame::BlockHashes(list) => {
                assert_eq!(list.relative_path, "partial.bin");
                assert_eq!(
                    list.block_size, RESUME_BS,
                    "an in-range open block_size must ride the wire unclamped"
                );
                saw_hashes = true;
            }
            Frame::NeedComplete(_) => continue,
            other => panic!("expected need choreography, got {other:?}"),
        }
    }

    // The violation: a whole-file record for the resume-flagged path.
    peer.send(wire(Frame::FileBegin(header))).await.unwrap();

    // Bounded wait: a regression here (accepting the record) leaves the
    // destination blocked on FileData frames this peer never sends —
    // the pin must fail on the clock, not hang the suite.
    let refusal = tokio::time::timeout(SUITE_TIMEOUT, async {
        loop {
            match recv_or_panic(&mut peer).await {
                Frame::Error(e) => break e,
                Frame::NeedComplete(_) => continue,
                other => panic!("expected SessionError, got {other:?}"),
            }
        }
    })
    .await
    .expect("the violation must be answered promptly, not absorbed");
    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
    let dest_err = dest.await.unwrap().unwrap_err();
    let fault = fault_of(&dest_err);
    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
    assert!(
        fault.message.contains("resume-flagged"),
        "got: {}",
        fault.message
    );
}

/// otp-7a fault injection: a source whose reader for one path yields
/// only the first `limit` bytes and then EOF, provably short of the
/// manifested size — the session's mid-record fault (the same EOF-short
/// abort a whole-file record has).
struct TruncatedReadSource {
    inner: FsTransferSource,
    fail_path: &'static str,
    limit: u64,
}

#[async_trait::async_trait]
impl TransferSource for TruncatedReadSource {
    fn scan(
        &self,
        filter: Option<blit_core::fs_enum::FileFilter>,
        unreadable_paths: Arc<std::sync::Mutex<Vec<String>>>,
    ) -> (
        tokio::sync::mpsc::Receiver<FileHeader>,
        tokio::task::JoinHandle<eyre::Result<u64>>,
    ) {
        self.inner.scan(filter, unreadable_paths)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> eyre::Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<std::sync::Mutex<Vec<String>>>,
    ) -> eyre::Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
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

#[tokio::test]
async fn mid_resume_source_fault_surfaces_cleanly_to_both_ends() {
    // Plan guard-proof 4 (mid-resume-failure, D4): the source faults
    // mid-block-phase — after at least one BlockTransfer landed — and a
    // clean `SessionFault` surfaces at BOTH ends within the suite
    // timeout (no deadlock), with no summary and so no false
    // `files_resumed`. The partial is left partially patched by design
    // (in-place model); the next run re-syncs it.
    let bs = RESUME_BS as usize;
    let content = make_patterned(3 * bs);
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(
            &src_root,
            &[("partial.bin", content.clone(), 1_600_001_100)],
        );
        // Every dest block is stale, so the source starts sending block
        // records immediately; its reader dies mid-block-2.
        write_tree(
            &dst_root,
            &[("partial.bin", vec![0x11; content.len()], 1_600_001_000)],
        );

        let open = resume_open(initiator_role, RESUME_BS);
        let (source_endpoint, dest_endpoint) = match initiator_role {
            TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
            TransferRole::Destination => {
                (SessionEndpoint::Responder, SessionEndpoint::initiator(open))
            }
            TransferRole::Unspecified => unreachable!(),
        };
        let source_cfg = SourceSessionConfig {
            instruments: Default::default(),
            hello: HelloConfig::default(),
            endpoint: source_endpoint,
            plan_options: PlanOptions::default(),
            data_plane_host: None,
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: dest_endpoint,
            data_plane_host: None,
            receiver_capacity: None,
            instruments: Default::default(),
            local_apply: None,
        };
        let (a, b) = in_process_pair();
        let source: Arc<dyn TransferSource> = Arc::new(TruncatedReadSource {
            inner: FsTransferSource::new(src_root.clone()),
            fail_path: "partial.bin",
            limit: (bs + bs / 2) as u64, // dies halfway through block 2
        });
        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
            tokio::join!(
                run_source(source_cfg, a, source),
                run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
            )
        })
        .await
        .expect("mid-resume fault must not deadlock");

        let source_err = source_result.expect_err("source must fault");
        let source_fault = fault_of(&source_err);
        assert_eq!(source_fault.code, session_error::Code::Internal);
        assert!(
            source_fault.message.contains("partial.bin"),
            "source fault must name the file: {}",
            source_fault.message
        );
        // otp-7b-2 (D-2026-07-09-1 Q2 rider): STRUCTURED file identity on
        // the fault — locally lifted from the FaultedPath marker — and an
        // end-of-operation summary naming it with a re-run suggestion.
        assert_eq!(
            source_fault.relative_path.as_deref(),
            Some("partial.bin"),
            "source fault carries the structured path (initiator {initiator_role:?})"
        );
        let summary = source_fault
            .end_of_operation_summary()
            .expect("a file-naming fault yields the end-of-operation summary");
        assert!(summary.contains("partial.bin") && summary.contains("re-run"));
        let dest_err = dest_result.expect_err("destination must fault");
        let dest_fault = fault_of(&dest_err);
        assert_eq!(
            dest_fault.code,
            session_error::Code::Internal,
            "the destination must surface the source's framed fault, got: {}",
            dest_fault.message
        );
        assert!(
            dest_fault.message.contains("partial.bin"),
            "destination fault must name the file: {}",
            dest_fault.message
        );
        // The identity crossed the wire (SessionError.relative_path), so
        // the OTHER end can name the file in its summary too.
        assert_eq!(
            dest_fault.relative_path.as_deref(),
            Some("partial.bin"),
            "destination fault carries the structured path over the wire \
             (initiator {initiator_role:?})"
        );
        // The fault was genuinely MID-record (codex F6): block 0 landed
        // in place before the reader died in block 1, so the partial is
        // partially patched — the in-place model D4 documents — and the
        // never-sent tail is untouched.
        let patched = std::fs::read(dst_root.join("partial.bin")).unwrap();
        assert_eq!(
            &patched[..bs],
            &content[..bs],
            "the first stale block must have landed before the fault \
             (initiator {initiator_role:?})"
        );
        assert_eq!(
            patched[bs], 0x11,
            "no byte past the faulted block may land (initiator {initiator_role:?})"
        );
    }
}

#[tokio::test]
async fn block_hashes_without_a_held_resume_need_fault_the_source() {
    // Choreography strictness: a BlockHashList must correlate with a
    // resume-flagged need the destination previously granted; an
    // uncorrelated list is a protocol violation, not a silent ignore.
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    std::fs::create_dir_all(&src_root).unwrap();
    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_001_200)]);

    let source_cfg = SourceSessionConfig {
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(resume_open(TransferRole::Source, RESUME_BS)),
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
    peer.send(wire(Frame::BlockHashes(BlockHashList {
        relative_path: "real.txt".into(),
        block_size: RESUME_BS,
        hashes: Vec::new(),
    })))
    .await
    .unwrap();

    let source_err = source_task.await.unwrap().unwrap_err();
    let fault = fault_of(&source_err);
    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
    assert!(
        fault.message.contains("without a held resume need"),
        "got: {}",
        fault.message
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_tiny_files_transfer_with_live_workers_when_source_initiates() {
    // Workload shape no longer selects a worker target. Production telemetry
    // may adapt while this 10k-file transfer runs, so deterministic worker
    // sequences are pinned by the in-crate ldt-2 role guard instead.
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
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: Some("127.0.0.1".into()),
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
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
        "the adaptive guard must ride the TCP data plane"
    );
    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
    let streams = outcome
        .data_plane_streams
        .expect("data plane ran, stream count recorded");
    assert!((1..=DIAL_DEFAULT_STREAM_LIMIT).contains(&streams));
    assert_trees_identical(&src_root, &dst_root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn payload_completion_does_not_wait_for_a_tuner_decision_under_either_initiator() {
    // The retired shape ramp delayed SourceDone until a predetermined target
    // was reached. With live-only authority, useful work and completion need
    // no predetermined resize target at all.
    const FILE_COUNT: usize = 2_000;

    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        for i in 0..FILE_COUNT {
            std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
        }

        let open = SessionOpen {
            initiator_role: initiator_role as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: false,
            ..Default::default()
        };
        let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
            TransferRole::Source => (
                SessionEndpoint::initiator(open),
                SessionEndpoint::Responder,
                Some("127.0.0.1".into()),
                None,
            ),
            TransferRole::Destination => (
                SessionEndpoint::Responder,
                SessionEndpoint::initiator(open),
                None,
                Some("127.0.0.1".into()),
            ),
            TransferRole::Unspecified => unreachable!(),
        };

        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
        let source_cfg = SourceSessionConfig {
            instruments: SourceInstruments {
                progress: Some(RemoteTransferProgress::new(progress_tx)),
                ..Default::default()
            },
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

        let (source_transport, dest_transport) = in_process_pair();
        let source = Arc::new(FsTransferSource::new(src_root.clone()));
        let session = tokio::spawn({
            let dst_root = dst_root.clone();
            async move {
                tokio::join!(
                    run_source(source_cfg, source_transport, source),
                    run_destination(dest_cfg, dest_transport, DestinationTarget::Fixed(dst_root)),
                )
            }
        });

        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, session)
            .await
            .expect("session run timed out")
            .expect("session task panicked");
        let summary = source_result.expect("source succeeds");
        let outcome = dest_result.expect("destination succeeds");
        assert_eq!(summary, outcome.summary);
        assert_eq!(summary.files_transferred, FILE_COUNT as u64);
        let streams = outcome
            .data_plane_streams
            .expect("data plane ran, final logical count recorded");
        assert!((1..=DIAL_DEFAULT_STREAM_LIMIT).contains(&streams));
        let mut completed = 0usize;
        while let Ok(event) = progress_rx.try_recv() {
            if matches!(event, ProgressEvent::FileComplete { .. }) {
                completed += 1;
            }
        }
        assert_eq!(completed, FILE_COUNT, "every payload reported completion");
        assert_trees_identical(&src_root, &dst_root);
    }
}

struct PhaseTraceCase {
    summary: TransferSummary,
    needed_paths: Vec<String>,
    data_plane_streams: Option<usize>,
    tree: BTreeMap<String, Vec<u8>>,
    events: Vec<SessionPhaseEvent>,
}

async fn run_phase_trace_case(initiator_role: TransferRole, trace_enabled: bool) -> PhaseTraceCase {
    const FILE_COUNT: usize = 256;
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    for i in 0..FILE_COUNT {
        std::fs::write(src_root.join(format!("f{i:04}.bin")), b"x").unwrap();
    }

    let captured: Arc<Mutex<Vec<SessionPhaseEvent>>> = Arc::default();
    let phase_trace = if trace_enabled {
        let sink = Arc::clone(&captured);
        SessionPhaseTrace::capture("phase-guard", move |event| {
            sink.lock()
                .expect("phase capture lock poisoned")
                .push(event);
        })
    } else {
        SessionPhaseTrace::disabled()
    };

    let open = SessionOpen {
        initiator_role: initiator_role as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: false,
        ..Default::default()
    };
    let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
        TransferRole::Source => (
            SessionEndpoint::initiator(open),
            SessionEndpoint::Responder,
            Some("127.0.0.1".into()),
            None,
        ),
        TransferRole::Destination => (
            SessionEndpoint::Responder,
            SessionEndpoint::initiator(open),
            None,
            Some("127.0.0.1".into()),
        ),
        TransferRole::Unspecified => unreachable!(),
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: source_endpoint,
        plan_options: PlanOptions::default(),
        data_plane_host: source_host,
        instruments: SourceInstruments {
            session_phase_trace: phase_trace.clone(),
            ..Default::default()
        },
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: dest_endpoint,
        data_plane_host: dest_host,
        receiver_capacity: None,
        instruments: DestinationInstruments {
            session_phase_trace: phase_trace,
            ..Default::default()
        },
        local_apply: None,
    };
    let (source_transport, dest_transport) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.clone()));
    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, source_transport, source),
            run_destination(
                dest_cfg,
                dest_transport,
                DestinationTarget::Fixed(dst_root.clone()),
            ),
        )
    })
    .await
    .expect("phase trace session timed out");

    let summary = source_result.expect("source succeeds");
    let mut outcome = dest_result.expect("destination succeeds");
    assert_eq!(summary, outcome.summary);
    assert_trees_identical(&src_root, &dst_root);
    outcome.needed_paths.sort();
    let events = captured
        .lock()
        .expect("phase capture lock poisoned")
        .clone();
    PhaseTraceCase {
        summary,
        needed_paths: outcome.needed_paths,
        data_plane_streams: outcome.data_plane_streams,
        tree: collect_tree(&dst_root),
        events,
    }
}

fn one_phase_event<'a>(
    events: &'a [SessionPhaseEvent],
    role: SessionPhaseRole,
    name: &str,
    epoch: Option<u32>,
) -> &'a SessionPhaseEvent {
    let found: Vec<_> = events
        .iter()
        .filter(|event| event.endpoint_role == role && event.event == name && event.epoch == epoch)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected one {role:?}/{name}/epoch={epoch:?}, got {}",
        found.len()
    );
    found[0]
}

fn one_phase_batch<'a>(
    events: &'a [SessionPhaseEvent],
    role: SessionPhaseRole,
    name: &str,
    batch: u64,
) -> &'a SessionPhaseEvent {
    let found: Vec<_> = events
        .iter()
        .filter(|event| {
            event.endpoint_role == role && event.event == name && event.batch == Some(batch)
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected one {role:?}/{name}/batch={batch}, got {}",
        found.len()
    );
    found[0]
}

fn phase_position(
    events: &[SessionPhaseEvent],
    role: SessionPhaseRole,
    name: &str,
    epoch: Option<u32>,
    batch: Option<u64>,
) -> usize {
    events
        .iter()
        .position(|event| {
            event.endpoint_role == role
                && event.event == name
                && event.epoch == epoch
                && event.batch == batch
        })
        .unwrap_or_else(|| panic!("missing {role:?}/{name}/epoch={epoch:?}/batch={batch:?}"))
}

fn phase_socket_position(
    events: &[SessionPhaseEvent],
    role: SessionPhaseRole,
    name: &str,
    epoch: Option<u32>,
    socket: Option<u32>,
) -> usize {
    events
        .iter()
        .position(|event| {
            event.endpoint_role == role
                && event.event == name
                && event.epoch == epoch
                && event.socket == socket
        })
        .unwrap_or_else(|| panic!("missing {role:?}/{name}/epoch={epoch:?}/socket={socket:?}"))
}

fn assert_phase_trace_partial_order(events: &[SessionPhaseEvent], initiator: TransferRole) {
    let source = SessionPhaseRole::Source;
    let destination = SessionPhaseRole::Destination;

    let manifest_begin = one_phase_event(events, source, "manifest_complete_send_begin", None);
    let manifest = one_phase_event(events, source, "manifest_complete_sent", None);
    one_phase_event(events, destination, "manifest_complete_received", None);
    let queued = one_phase_event(events, source, "first_payload_queued", None);
    let first_write = events
        .iter()
        .filter(|event| event.endpoint_role == source && event.event == "first_socket_write")
        .min_by_key(|event| event.elapsed_ns)
        .expect("at least one source socket writes payload");
    assert!(manifest_begin.elapsed_ns <= manifest.elapsed_ns);
    assert!(manifest.elapsed_ns < queued.elapsed_ns);
    assert!(queued.elapsed_ns <= first_write.elapsed_ns);
    assert!(
        phase_position(events, source, "manifest_complete_send_begin", None, None,)
            < phase_position(
                events,
                destination,
                "manifest_complete_received",
                None,
                None,
            )
    );

    let write_keys: Vec<_> = events
        .iter()
        .filter(|event| event.endpoint_role == source && event.event == "first_socket_write")
        .map(|event| (event.epoch, event.socket))
        .collect();
    let write_begin_keys: Vec<_> = events
        .iter()
        .filter(|event| event.endpoint_role == source && event.event == "socket_write_begin")
        .map(|event| (event.epoch, event.socket))
        .collect();
    let receive_keys: Vec<_> = events
        .iter()
        .filter(|event| {
            event.endpoint_role == destination && event.event == "first_payload_received"
        })
        .map(|event| (event.epoch, event.socket))
        .collect();
    for keys in [&write_keys, &write_begin_keys, &receive_keys] {
        assert!(
            keys.iter()
                .all(|(epoch, socket)| epoch.is_some() && socket.is_some()),
            "per-socket phase marker lost epoch/socket correlation"
        );
        assert_eq!(
            keys.len(),
            keys.iter().copied().collect::<BTreeSet<_>>().len(),
            "duplicate per-socket phase marker"
        );
    }
    let writes: BTreeSet<_> = write_keys.into_iter().collect();
    let write_begins: BTreeSet<_> = write_begin_keys.into_iter().collect();
    let receives: BTreeSet<_> = receive_keys.into_iter().collect();
    assert_eq!(write_begins, writes);
    assert_eq!(writes, receives);
    for (epoch, socket) in writes {
        assert!(
            phase_socket_position(events, source, "socket_write_begin", epoch, socket,)
                < phase_socket_position(
                    events,
                    destination,
                    "first_payload_received",
                    epoch,
                    socket,
                )
        );
    }

    let expected_attached = BTreeSet::from([
        (Some(0), Some(0)),
        (Some(0), Some(1)),
        (Some(0), Some(2)),
        (Some(0), Some(3)),
    ]);
    for role in [source, destination] {
        let attached_keys: Vec<_> = events
            .iter()
            .filter(|event| event.endpoint_role == role && event.event == "socket_trace_attached")
            .map(|event| (event.epoch, event.socket))
            .collect();
        assert!(attached_keys
            .iter()
            .all(|(epoch, socket)| epoch.is_some() && socket.is_some()));
        assert_eq!(
            attached_keys.len(),
            attached_keys.iter().copied().collect::<BTreeSet<_>>().len(),
            "duplicate {role:?} socket trace attachment"
        );
        assert_eq!(
            attached_keys.into_iter().collect::<BTreeSet<_>>(),
            expected_attached,
            "every acquired {role:?} socket must carry the phase trace"
        );
    }

    let need_begin = one_phase_batch(events, destination, "need_batch_send_begin", 0);
    let need_sent = one_phase_batch(events, destination, "need_batch_sent", 0);
    one_phase_batch(events, source, "need_batch_received", 0);
    assert!(need_begin.elapsed_ns <= need_sent.elapsed_ns);
    assert!(
        phase_position(events, destination, "need_batch_send_begin", None, Some(0),)
            < phase_position(events, source, "need_batch_received", None, Some(0),)
    );
    let planner_begin = one_phase_batch(events, source, "planner_begin", 0);
    let planner_end = one_phase_batch(events, source, "planner_end", 0);
    assert!(planner_begin.elapsed_ns <= planner_end.elapsed_ns);

    let (source_action, destination_action) = match initiator {
        TransferRole::Source => ("dial", "accept"),
        TransferRole::Destination => ("accept", "dial"),
        TransferRole::Unspecified => unreachable!(),
    };
    for (role, action) in [(source, source_action), (destination, destination_action)] {
        for socket in 0..4 {
            let begin = phase_socket_position(
                events,
                role,
                &format!("socket_{action}_begin"),
                Some(0),
                Some(socket),
            );
            let end = phase_socket_position(
                events,
                role,
                &format!("socket_{action}_end"),
                Some(0),
                Some(socket),
            );
            let attached =
                phase_socket_position(events, role, "socket_trace_attached", Some(0), Some(socket));
            assert!(begin < end && end < attached);
        }
    }
    let source_complete = one_phase_event(events, source, "data_plane_complete", None);
    let destination_complete = one_phase_event(events, destination, "data_plane_complete", None);
    let summary_begin = one_phase_event(events, destination, "summary_send_begin", None);
    let summary_sent = one_phase_event(events, destination, "summary_sent", None);
    one_phase_event(events, source, "summary_received", None);
    assert!(
        source_complete.elapsed_ns
            <= one_phase_event(events, source, "summary_received", None).elapsed_ns
    );
    assert!(destination_complete.elapsed_ns <= summary_begin.elapsed_ns);
    assert!(summary_begin.elapsed_ns <= summary_sent.elapsed_ns);
    assert!(
        phase_position(events, destination, "summary_send_begin", None, None)
            < phase_position(events, source, "summary_received", None, None)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_phase_trace_is_complete_and_inert_under_both_initiators() {
    let source_off = run_phase_trace_case(TransferRole::Source, false).await;
    let source_on = run_phase_trace_case(TransferRole::Source, true).await;
    let destination_off = run_phase_trace_case(TransferRole::Destination, false).await;
    let destination_on = run_phase_trace_case(TransferRole::Destination, true).await;

    assert!(source_off.events.is_empty());
    assert!(destination_off.events.is_empty());
    for (off, on) in [
        (&source_off, &source_on),
        (&destination_off, &destination_on),
    ] {
        assert_eq!(off.summary, on.summary);
        assert_eq!(off.needed_paths, on.needed_paths);
        assert!(off.data_plane_streams.is_some());
        assert_eq!(off.tree, on.tree);
        assert!(on.data_plane_streams.is_some());
    }
    assert_eq!(source_on.summary, destination_on.summary);
    assert_eq!(source_on.needed_paths, destination_on.needed_paths);
    assert_eq!(source_on.tree, destination_on.tree);

    let source_session_id = source_on.events[0].session_id.clone();
    let destination_session_id = destination_on.events[0].session_id.clone();
    assert_ne!(
        source_session_id, destination_session_id,
        "independent sessions need distinct correlation fingerprints"
    );

    for (case, initiator, initiator_phase_role) in [
        (&source_on, TransferRole::Source, SessionPhaseRole::Source),
        (
            &destination_on,
            TransferRole::Destination,
            SessionPhaseRole::Destination,
        ),
    ] {
        let session_ids: BTreeSet<_> = case
            .events
            .iter()
            .map(|event| event.session_id.as_str())
            .collect();
        assert_eq!(session_ids.len(), 1);
        let session_id = *session_ids.first().unwrap();
        assert_eq!(session_id.len(), 16);
        assert!(session_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert!(case
            .events
            .iter()
            .all(|event| event.schema == 1 && event.run_id == "phase-guard"));
        assert!(case
            .events
            .iter()
            .all(|event| event.initiator_role == initiator_phase_role));
        assert_eq!(
            case.events
                .iter()
                .map(|event| event.endpoint_role)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([SessionPhaseRole::Source, SessionPhaseRole::Destination])
        );
        for role in [SessionPhaseRole::Source, SessionPhaseRole::Destination] {
            let mut sequences: Vec<_> = case
                .events
                .iter()
                .filter(|event| event.endpoint_role == role)
                .map(|event| event.producer_seq)
                .collect();
            let expected: Vec<_> = (0..sequences.len() as u64).collect();
            sequences.sort_unstable();
            assert_eq!(sequences, expected, "{role:?} sequence has gaps/duplicates");
        }
        assert_phase_trace_partial_order(&case.events, initiator);
    }
}

struct SmallFileProbeCase {
    summary: TransferSummary,
    needed_paths: Vec<String>,
    data_plane_streams: Option<usize>,
    tree: BTreeMap<String, Vec<u8>>,
    metadata: BTreeMap<String, FileMetadataSnapshot>,
    reports: Vec<SmallFileProbeReport>,
}

#[derive(Debug, Eq, PartialEq)]
struct FileMetadataSnapshot {
    len: u64,
    mtime_seconds: i64,
    readonly: bool,
    #[cfg(unix)]
    mode: u32,
}

fn collect_file_metadata(root: &Path) -> BTreeMap<String, FileMetadataSnapshot> {
    collect_tree(root)
        .into_keys()
        .map(|relative_path| {
            let metadata = std::fs::metadata(root.join(&relative_path)).unwrap();
            let snapshot = FileMetadataSnapshot {
                len: metadata.len(),
                mtime_seconds: filetime::FileTime::from_last_modification_time(&metadata)
                    .unix_seconds(),
                readonly: metadata.permissions().readonly(),
                #[cfg(unix)]
                mode: {
                    use std::os::unix::fs::PermissionsExt;
                    metadata.permissions().mode() & 0o777
                },
            };
            (relative_path, snapshot)
        })
        .collect()
}

async fn run_small_file_probe_case(
    initiator_role: TransferRole,
    carrier: SmallFileCarrier,
    probe_enabled: bool,
) -> SmallFileProbeCase {
    const FILE_COUNT: usize = 256;
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    for i in 0..FILE_COUNT {
        let path = src_root.join(format!("f{i:04}.bin"));
        std::fs::write(&path, vec![i as u8; 4 * 1024]).unwrap();
        filetime::set_file_mtime(
            &path,
            filetime::FileTime::from_unix_time(1_650_000_000 + i as i64, 0),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
    }

    let captured: Arc<Mutex<Vec<SmallFileProbeReport>>> = Arc::default();
    let probe = if probe_enabled {
        let sink = Arc::clone(&captured);
        SmallFileProbe::capture(
            format!("p2-guard-{initiator_role:?}-{carrier:?}"),
            move |report| sink.lock().unwrap().push(report),
        )
    } else {
        SmallFileProbe::disabled()
    };

    let tcp = carrier == SmallFileCarrier::Tcp;
    let open = SessionOpen {
        initiator_role: initiator_role as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: !tcp,
        ..Default::default()
    };
    let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
        TransferRole::Source => (
            SessionEndpoint::initiator(open),
            SessionEndpoint::Responder,
            tcp.then(|| "127.0.0.1".into()),
            None,
        ),
        TransferRole::Destination => (
            SessionEndpoint::Responder,
            SessionEndpoint::initiator(open),
            None,
            tcp.then(|| "127.0.0.1".into()),
        ),
        TransferRole::Unspecified => unreachable!(),
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: source_endpoint,
        plan_options: PlanOptions {
            force_tar: true,
            small_count_target: Some(128),
            ..Default::default()
        },
        data_plane_host: source_host,
        instruments: SourceInstruments {
            small_file_probe: probe.clone(),
            ..Default::default()
        },
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: dest_endpoint,
        data_plane_host: dest_host,
        receiver_capacity: None,
        instruments: DestinationInstruments {
            small_file_probe: probe,
            ..Default::default()
        },
        local_apply: None,
    };
    let (source_transport, dest_transport) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.clone()));
    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, source_transport, source),
            run_destination(
                dest_cfg,
                dest_transport,
                DestinationTarget::Fixed(dst_root.clone()),
            ),
        )
    })
    .await
    .expect("small-file probe session timed out");

    let summary = source_result.expect("source succeeds");
    let mut outcome = dest_result.expect("destination succeeds");
    assert_eq!(summary, outcome.summary);
    assert_trees_identical(&src_root, &dst_root);
    outcome.needed_paths.sort();
    let reports = captured.lock().unwrap().clone();
    SmallFileProbeCase {
        summary,
        needed_paths: outcome.needed_paths,
        data_plane_streams: outcome.data_plane_streams,
        tree: collect_tree(&dst_root),
        metadata: collect_file_metadata(&dst_root),
        reports,
    }
}

fn assert_timing_observed(timing: &TimingAggregate, samples: u64) {
    assert_eq!(timing.samples, samples);
    assert!(timing.total_ns > 0, "timing aggregate is vacuously zero");
    assert!(timing.max_ns > 0, "timing max is vacuously zero");
    assert!(timing.max_ns <= timing.total_ns);
}

fn assert_small_file_probe_inventory(
    case: &SmallFileProbeCase,
    initiator_role: TransferRole,
    carrier: SmallFileCarrier,
) {
    const FILE_COUNT: u64 = 256;
    assert_eq!(case.reports.len(), 2, "one summary per semantic endpoint");
    let source = case
        .reports
        .iter()
        .find(|report| report.endpoint_role == SessionPhaseRole::Source)
        .expect("source report");
    let destination = case
        .reports
        .iter()
        .find(|report| report.endpoint_role == SessionPhaseRole::Destination)
        .expect("destination report");
    let expected_initiator = match initiator_role {
        TransferRole::Source => SessionPhaseRole::Source,
        TransferRole::Destination => SessionPhaseRole::Destination,
        TransferRole::Unspecified => unreachable!(),
    };
    let expected_run_id = format!("p2-guard-{initiator_role:?}-{carrier:?}");
    for report in [&source, &destination] {
        assert_eq!(report.schema, 1);
        assert_eq!(report.run_id, expected_run_id);
        assert!(report.success);
        assert_eq!(report.event, "summary");
        assert_eq!(report.initiator_role, expected_initiator);
        assert_eq!(report.carrier, carrier);
        assert_eq!(report.shard_receive_dropped, 0);
        assert_eq!(report.shard_sink_dropped, 0);
        assert_eq!(report.correlation_id.len(), 16);
        assert!(report
            .correlation_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    }
    assert_eq!(source.correlation_id, destination.correlation_id);
    assert_eq!(destination.source_bookkeeping, Default::default());
    assert_eq!(source.tcp_claims, Default::default());
    assert_eq!(source.in_stream_claims, Default::default());
    assert!(source.shard_receive.is_empty());
    assert!(source.shard_sink.is_empty());
    assert_eq!(source.shard_receive_dropped, 0);
    assert_eq!(source.shard_sink_dropped, 0);

    let bookkeeping = &source.source_bookkeeping;
    assert_eq!(bookkeeping.manifest_entries_inserted, FILE_COUNT);
    assert_eq!(bookkeeping.manifest_insert_sync_wait.samples, FILE_COUNT);
    assert_eq!(bookkeeping.manifest_insert_map_op.samples, FILE_COUNT);
    assert_eq!(bookkeeping.need_entries_resolved, FILE_COUNT);
    assert_eq!(bookkeeping.need_resolve_sync_wait.samples, FILE_COUNT);
    assert_eq!(bookkeeping.need_resolve_map_op.samples, FILE_COUNT);
    assert_eq!(bookkeeping.need_event_send.samples, FILE_COUNT);
    assert_eq!(bookkeeping.need_event_hop.samples, FILE_COUNT);
    assert_eq!(bookkeeping.need_handler_work.samples, FILE_COUNT);
    for timing in [
        &bookkeeping.manifest_insert_sync_wait,
        &bookkeeping.manifest_insert_map_op,
        &bookkeeping.need_resolve_sync_wait,
        &bookkeeping.need_resolve_map_op,
        &bookkeeping.need_event_send,
        &bookkeeping.need_event_hop,
        &bookkeeping.need_handler_work,
    ] {
        assert_timing_observed(timing, FILE_COUNT);
    }
    assert_eq!(bookkeeping.planner_input_entries, FILE_COUNT);
    assert!(bookkeeping.planned_tar_shards > 0);
    assert_eq!(bookkeeping.planned_tar_members, FILE_COUNT);
    assert!(bookkeeping.planner.samples > 0);
    assert_timing_observed(&bookkeeping.planner, bookkeeping.planner.samples);
    let shard_count = destination.shard_receive.len() as u64;
    assert!(shard_count > 0);

    let (claims, absent_claims) = match carrier {
        SmallFileCarrier::Tcp => (&destination.tcp_claims, &destination.in_stream_claims),
        SmallFileCarrier::InStream => (&destination.in_stream_claims, &destination.tcp_claims),
    };
    assert_eq!(claims.members, FILE_COUNT);
    assert_eq!(claims.successful_removes, FILE_COUNT);
    assert_eq!(
        claims.lock_acquisitions,
        if carrier == SmallFileCarrier::Tcp {
            FILE_COUNT
        } else {
            shard_count
        }
    );
    assert_eq!(claims.lock_wait.samples, claims.lock_acquisitions);
    assert_eq!(claims.lock_hold.samples, claims.lock_acquisitions);
    assert_timing_observed(&claims.lock_wait, claims.lock_acquisitions);
    assert_timing_observed(&claims.lock_hold, claims.lock_acquisitions);
    assert_eq!(absent_claims, &Default::default());

    assert_eq!(destination.shard_receive.len() as u64, shard_count);
    assert_eq!(
        destination
            .shard_receive
            .iter()
            .map(|shard| shard.members)
            .sum::<u64>(),
        FILE_COUNT
    );
    assert_eq!(destination.shard_sink.len() as u64, shard_count);
    assert_eq!(
        destination
            .shard_sink
            .iter()
            .map(|shard| shard.members)
            .sum::<u64>(),
        FILE_COUNT
    );
    let receive_shards: BTreeMap<_, _> = destination
        .shard_receive
        .iter()
        .map(|shard| {
            (
                shard.shard_id.as_str(),
                (shard.members, shard.archive_bytes),
            )
        })
        .collect();
    let sink_shards: BTreeMap<_, _> = destination
        .shard_sink
        .iter()
        .map(|shard| {
            (
                shard.shard_id.as_str(),
                (shard.members, shard.archive_bytes),
            )
        })
        .collect();
    assert_eq!(receive_shards.len() as u64, shard_count);
    assert_eq!(receive_shards, sink_shards);
    for (seq, shard) in destination.shard_receive.iter().enumerate() {
        assert_eq!(shard.seq, seq as u64);
        assert_eq!(shard.carrier, carrier);
        assert!(shard.start_elapsed_ns > 0);
        assert_eq!(shard.total_ns, shard.record_receive_ns + shard.sink_ns);
        assert!(shard.record_receive_ns > 0);
        assert!(shard.correlation_ns > 0);
        assert!(shard.sink_ns > 0);
        assert!(shard.total_ns > 0);
    }
    let mut blocking_pool_wait_total = 0u64;
    for (seq, shard) in destination.shard_sink.iter().enumerate() {
        assert_eq!(shard.seq, seq as u64);
        assert_eq!(shard.carrier, carrier);
        assert!(shard.start_elapsed_ns > 0);
        assert!(shard.parse_validate_ns > 0);
        assert!(shard.member_parallel_wall_ns > 0);
        assert!(shard.total_ns > 0);
        assert!(
            shard.total_ns
                >= shard
                    .blocking_pool_wait_ns
                    .saturating_add(shard.parse_validate_ns)
                    .saturating_add(shard.member_parallel_wall_ns)
        );
        blocking_pool_wait_total =
            blocking_pool_wait_total.saturating_add(shard.blocking_pool_wait_ns);
        for timing in [
            &shard.member.mkdir,
            &shard.member.open,
            &shard.member.write,
            &shard.member.close,
            &shard.member.metadata,
            &shard.member.total,
        ] {
            assert_timing_observed(timing, shard.members);
        }
        assert!(
            shard.member.total.total_ns
                >= shard
                    .member
                    .mkdir
                    .total_ns
                    .saturating_add(shard.member.open.total_ns)
                    .saturating_add(shard.member.write.total_ns)
                    .saturating_add(shard.member.close.total_ns)
                    .saturating_add(shard.member.metadata.total_ns)
        );
    }
    assert!(blocking_pool_wait_total > 0);
    let member_sample_totals =
        destination
            .shard_sink
            .iter()
            .fold([0u64; 6], |mut totals, shard| {
                totals[0] += shard.member.mkdir.samples;
                totals[1] += shard.member.open.samples;
                totals[2] += shard.member.write.samples;
                totals[3] += shard.member.close.samples;
                totals[4] += shard.member.metadata.samples;
                totals[5] += shard.member.total.samples;
                totals
            });
    for samples in member_sample_totals {
        assert_eq!(samples, FILE_COUNT);
    }

    match carrier {
        SmallFileCarrier::Tcp => {
            assert_eq!(bookkeeping.tar_shards_queued, shard_count);
            assert_eq!(bookkeeping.tar_members_queued, FILE_COUNT);
            assert_timing_observed(&bookkeeping.tar_queue, shard_count);
        }
        SmallFileCarrier::InStream => {
            assert_eq!(bookkeeping.tar_shards_queued, 0);
            assert_eq!(bookkeeping.tar_members_queued, 0);
            assert_eq!(bookkeeping.tar_queue, Default::default());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn small_file_probe_is_complete_and_inert_across_roles_and_carriers() {
    for carrier in [SmallFileCarrier::Tcp, SmallFileCarrier::InStream] {
        let source_off = run_small_file_probe_case(TransferRole::Source, carrier, false).await;
        let source_on = run_small_file_probe_case(TransferRole::Source, carrier, true).await;
        let destination_off =
            run_small_file_probe_case(TransferRole::Destination, carrier, false).await;
        let destination_on =
            run_small_file_probe_case(TransferRole::Destination, carrier, true).await;

        assert!(source_off.reports.is_empty());
        assert!(destination_off.reports.is_empty());
        for (off, on) in [
            (&source_off, &source_on),
            (&destination_off, &destination_on),
        ] {
            assert_eq!(off.summary, on.summary);
            assert_eq!(off.needed_paths, on.needed_paths);
            assert_eq!(
                off.data_plane_streams.is_some(),
                carrier == SmallFileCarrier::Tcp
            );
            assert_eq!(
                on.data_plane_streams.is_some(),
                carrier == SmallFileCarrier::Tcp
            );
            assert_eq!(off.tree, on.tree);
            assert_eq!(off.metadata, on.metadata);
        }
        assert_eq!(source_on.summary, destination_on.summary);
        assert_eq!(source_on.needed_paths, destination_on.needed_paths);
        assert_eq!(
            source_on.data_plane_streams.is_some(),
            destination_on.data_plane_streams.is_some()
        );
        assert_eq!(source_on.tree, destination_on.tree);
        assert_eq!(source_on.metadata, destination_on.metadata);
        assert_eq!(
            source_on.summary.in_stream_carrier_used,
            carrier == SmallFileCarrier::InStream
        );
        assert_small_file_probe_inventory(&source_on, TransferRole::Source, carrier);
        assert_small_file_probe_inventory(&destination_on, TransferRole::Destination, carrier);

        let source_report = source_on
            .reports
            .iter()
            .find(|report| report.endpoint_role == SessionPhaseRole::Destination)
            .unwrap();
        let destination_report = destination_on
            .reports
            .iter()
            .find(|report| report.endpoint_role == SessionPhaseRole::Destination)
            .unwrap();
        assert_ne!(
            source_report.correlation_id,
            destination_report.correlation_id
        );
        let source_ids: BTreeSet<_> = source_report
            .shard_receive
            .iter()
            .map(|shard| shard.shard_id.as_str())
            .collect();
        let destination_ids: BTreeSet<_> = destination_report
            .shard_receive
            .iter()
            .map(|shard| shard.shard_id.as_str())
            .collect();
        assert!(source_ids.is_disjoint(&destination_ids));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pull_data_plane_lands_bytes() {
    // otp-5b-1: the transport/role decoupling in the PULL direction — the
    // mirror of the push data-plane test above. Here the DESTINATION is the
    // *initiator* (dials + receives) and the SOURCE is the *responder*
    // (binds + accepts + sends). Control frames ride the in-process
    // transport; the data-plane socket rides loopback TCP (the SOURCE
    // responder binds 0.0.0.0:0, the DESTINATION initiator dials
    // 127.0.0.1). The deterministic ldt-2 role guard separately pins the
    // receiver-bounded epoch-0 floor.
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
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
        plan_options: PlanOptions::default(),
        data_plane_host: None, // a responder never dials
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open), // dials + receives
        data_plane_host: Some("127.0.0.1".into()),
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
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
    assert!(outcome.data_plane_streams.is_some());
    assert_trees_identical(&src_root, &dst_root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_tiny_files_transfer_with_live_workers_when_destination_initiates() {
    // The mirror of the SOURCE-initiator guard: connection topology changes
    // which end opens sockets, never the worker policy. File count alone is
    // not a static worker target; live telemetry may still resize while the
    // fixture runs.
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
    // data-plane test, the data plane following connection role.
    let open = SessionOpen {
        initiator_role: TransferRole::Destination as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: false,
        // Wire contract: zero means unknown, not a one-stream cap. Pin it
        // on the destination-initiator orientation, where this end both
        // advertises and enforces the receiver ceiling.
        receiver_capacity: Some(CapacityProfile {
            max_streams: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let source_cfg = SourceSessionConfig {
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
        plan_options: PlanOptions::default(),
        data_plane_host: None, // a responder never dials
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open), // dials + receives
        data_plane_host: Some("127.0.0.1".into()),
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
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
        "the adaptive guard must ride the TCP data plane"
    );
    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
    let streams = outcome
        .data_plane_streams
        .expect("data plane ran, stream count recorded");
    assert!((1..=DIAL_DEFAULT_STREAM_LIMIT).contains(&streams));
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
            instruments: Default::default(),
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
            receiver_capacity: None,
            instruments: Default::default(),
            local_apply: None,
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
        instruments: Default::default(),
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
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
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
async fn mirror_enabled_without_scope_is_refused() {
    // otp-6b: a mirror-enabled open with no concrete scope (kind defaults to
    // UNSPECIFIED) is a contradiction — refuse it at OPEN with a protocol
    // violation, from the destination (the end that executes deletions).
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();

    let mut open = basic_open(TransferRole::Source);
    open.mirror_enabled = true; // no mirror_kind set → UNSPECIFIED
    let source_cfg = SourceSessionConfig {
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root));
    let (source_result, dest_result) = tokio::join!(
        run_source(source_cfg, a, source),
        run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
    );
    assert_eq!(
        fault_of(&source_result.unwrap_err()).code,
        session_error::Code::ProtocolViolation
    );
    assert!(dest_result.is_err());
}

/// Drive one mirror session (fixed direction src→dst) with the given
/// initiator role, mirror kind, and optional include filter, over
/// pre-populated trees. Returns the source summary and dest outcome.
async fn run_mirror_session(
    initiator_role: TransferRole,
    src_root: &Path,
    dst_root: &Path,
    mirror_kind: MirrorMode,
    include: Option<&str>,
) -> (
    eyre::Result<TransferSummary>,
    eyre::Result<DestinationOutcome>,
) {
    let mut open = basic_open(initiator_role);
    open.mirror_enabled = true;
    open.mirror_kind = mirror_kind as i32;
    if let Some(pat) = include {
        open.filter = Some(FilterSpec {
            include: vec![pat.to_string()],
            ..Default::default()
        });
    }
    run_session_with_open(open, src_root, dst_root, PlanOptions::default()).await
}

#[tokio::test]
async fn mirror_all_purges_extraneous_under_both_initiators() {
    // otp-6b: MirrorMode::All deletes every dest entry absent from the
    // source set — files and the now-empty dirs that held them — whichever
    // end initiates. The delete count includes the pruned directory.
    let src = vec![
        ("keep.txt", b"new".to_vec(), 1_600_000_001),
        ("sub/keep2.txt", b"new".to_vec(), 1_600_000_002),
    ];
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, &src);
        write_tree(
            &dst_root,
            &[
                ("keep.txt", b"old".to_vec(), 1_500_000_000),
                ("sub/keep2.txt", b"old".to_vec(), 1_500_000_000),
                ("stale.txt", b"gone".to_vec(), 1_500_000_000),
                ("dead/old.bin", b"gone".to_vec(), 1_500_000_000),
            ],
        );

        let (sr, dr) =
            run_mirror_session(initiator_role, &src_root, &dst_root, MirrorMode::All, None).await;
        let summary =
            sr.unwrap_or_else(|e| panic!("source failed (init {initiator_role:?}): {e:#}"));
        let dest =
            dr.unwrap_or_else(|e| panic!("destination failed (init {initiator_role:?}): {e:#}"));

        assert_eq!(
            summary, dest.summary,
            "both ends agree (init {initiator_role:?})"
        );
        // stale.txt + dead/old.bin + the dead/ dir = 3.
        assert_eq!(
            summary.entries_deleted, 3,
            "extraneous file + nested file + pruned dir (init {initiator_role:?})"
        );
        assert!(!dst_root.join("stale.txt").exists());
        assert!(!dst_root.join("dead").exists());
        assert!(dst_root.join("keep.txt").exists());
        // After an All mirror the dest tree equals the source tree exactly.
        assert_trees_identical(&src_root, &dst_root);
    }
}

#[tokio::test]
async fn mirror_filtered_subset_preserves_out_of_scope() {
    // otp-6b: FilteredSubset deletes only extraneous entries WITHIN the
    // filter's scope. An out-of-scope dest file (not matching the include
    // filter) is left alone — the scope contract.
    let src = vec![("keep.txt", b"new".to_vec(), 1_600_000_001)];
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(&src_root, &src);
    write_tree(
        &dst_root,
        &[
            ("keep.txt", b"old".to_vec(), 1_500_000_000),
            ("stale.txt", b"gone".to_vec(), 1_500_000_000), // in scope, extraneous → deleted
            ("keep.log", b"safe".to_vec(), 1_500_000_000),  // out of scope → kept
        ],
    );

    let (sr, dr) = run_mirror_session(
        TransferRole::Source,
        &src_root,
        &dst_root,
        MirrorMode::FilteredSubset,
        Some("*.txt"),
    )
    .await;
    let summary = sr.expect("source session");
    let _ = dr.expect("destination session");

    assert_eq!(summary.entries_deleted, 1, "only the in-scope stale.txt");
    assert!(!dst_root.join("stale.txt").exists());
    assert!(
        dst_root.join("keep.log").exists(),
        "out-of-scope file must be preserved"
    );
    assert!(dst_root.join("keep.txt").exists());
}

#[tokio::test]
async fn mirror_all_purges_out_of_scope_even_when_filtered() {
    // otp-6b: MirrorMode::All ignores the filter for the deletion scope
    // (the filter only shapes the source set). An out-of-scope dest file
    // absent from the filtered source set IS deleted.
    let src = vec![("keep.txt", b"new".to_vec(), 1_600_000_001)];
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(&src_root, &src);
    write_tree(
        &dst_root,
        &[
            ("keep.txt", b"old".to_vec(), 1_500_000_000),
            ("stale.txt", b"gone".to_vec(), 1_500_000_000),
            ("keep.log", b"gone".to_vec(), 1_500_000_000), // out of scope but All → deleted
        ],
    );

    let (sr, dr) = run_mirror_session(
        TransferRole::Source,
        &src_root,
        &dst_root,
        MirrorMode::All,
        Some("*.txt"),
    )
    .await;
    let summary = sr.expect("source session");
    let _ = dr.expect("destination session");

    assert_eq!(
        summary.entries_deleted, 2,
        "stale.txt and out-of-scope keep.log"
    );
    assert!(!dst_root.join("stale.txt").exists());
    assert!(!dst_root.join("keep.log").exists());
    assert!(dst_root.join("keep.txt").exists());
}

#[tokio::test]
async fn mirror_refused_when_source_scan_incomplete() {
    // otp-6b: mirroring on an incomplete source scan could delete files the
    // source still has (they were merely unreadable mid-scan). The
    // destination must refuse at ManifestComplete{scan_complete=false} and
    // delete nothing. Scripted source peer so we control the flag.
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(
        &dst_root,
        &[("victim.txt", b"keep".to_vec(), 1_500_000_000)],
    );

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
        DestinationTarget::Fixed(dst_root.clone()),
    ));

    let mut open = basic_open(TransferRole::Source);
    open.mirror_enabled = true;
    open.mirror_kind = MirrorMode::All as i32;
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(wire(Frame::Open(open))).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));

    // A manifest entry, then declare the scan INCOMPLETE.
    peer.send(wire(Frame::ManifestEntry(FileHeader {
        relative_path: "present.txt".into(),
        size: 1,
        mtime_seconds: 1_600_000_000,
        permissions: 0o644,
        checksum: vec![],
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
        scan_complete: false,
    })))
    .await
    .unwrap();

    let refusal = loop {
        match recv_or_panic(&mut peer).await {
            Frame::Error(e) => break e,
            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
            other => panic!("expected SessionError, got {other:?}"),
        }
    };
    assert_eq!(refusal.code, session_error::Code::Internal as i32);
    assert!(
        refusal.message.contains("scan"),
        "refusal must cite the incomplete scan, got: {}",
        refusal.message
    );
    let dest_err = dest.await.unwrap().unwrap_err();
    assert_eq!(fault_of(&dest_err).code, session_error::Code::Internal);
    assert!(
        dst_root.join("victim.txt").exists(),
        "nothing may be deleted on a refused mirror"
    );
}

#[tokio::test]
async fn cancel_frame_during_mirror_purge_aborts_the_deletions() {
    // codex otp-10b-2 F1: a peer fault (CancelJob on the serving
    // source) arriving while the DESTINATION runs its mirror delete
    // pass must abort the pass and surface the fault — not sit unread
    // on the control lane while deletions run to completion behind a
    // cancelled session. Scripted source peer: an EMPTY manifest makes
    // every destination file extraneous, and the CANCELLED frame is
    // queued right behind SourceDone — the purge race reads it (biased
    // frame-first) and flips the abort flag before the pass's next
    // filesystem op.
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();
    for i in 0..2000 {
        let path = dst_root.join(format!("victim_{i:04}.txt"));
        std::fs::write(&path, b"extraneous").unwrap();
    }

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
        DestinationTarget::Fixed(dst_root.clone()),
    ));

    let mut open = basic_open(TransferRole::Source);
    open.mirror_enabled = true;
    open.mirror_kind = MirrorMode::All as i32;
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(wire(Frame::Open(open))).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));

    // Empty complete scan → empty need list → the purge would delete
    // all 2000 files. Queue the cancel right behind SourceDone.
    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
        scan_complete: true,
    })))
    .await
    .unwrap();
    assert!(matches!(
        recv_or_panic(&mut peer).await,
        Frame::NeedComplete(_)
    ));
    peer.send(wire(Frame::SourceDone(SourceDone {})))
        .await
        .unwrap();
    peer.send(wire(Frame::Error(SessionError {
        code: session_error::Code::Cancelled as i32,
        message: "job cancelled by operator".into(),
        ..Default::default()
    })))
    .await
    .unwrap();

    let dest_err = tokio::time::timeout(SUITE_TIMEOUT, dest)
        .await
        .expect("destination must not hang mid-purge")
        .unwrap()
        .expect_err("a cancelled session must not report success");
    assert_eq!(
        fault_of(&dest_err).code,
        session_error::Code::Cancelled,
        "the peer's CANCELLED must own the outcome, got: {dest_err:#}"
    );
    let survivors = std::fs::read_dir(&dst_root).unwrap().count();
    assert!(
        survivors > 0,
        "the purge must abort instead of deleting all 2000 entries \
         behind a cancelled session"
    );
}

#[tokio::test]
async fn cancel_mid_file_record_surfaces_the_peers_fault() {
    // otp-5b-3 (re-scoped, direction-agnostic): cancel while file DATA
    // is in flight. The scripted source opens a 3 MiB record, delivers
    // one partial FileData frame, then sends CANCELLED mid-record —
    // bytes still owed. The destination must surface the peer's fault
    // (code CANCELLED, message preserved), NOT a ProtocolViolation
    // about frame position, and must not finalize the partial file.
    // Direction-invariance (D-2026-07-05-1) makes this one scripted
    // shape stand for both user-facing directions: the in-stream
    // record receive is the same code under either initiator.
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();

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
        DestinationTarget::Fixed(dst_root.clone()),
    ));

    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
        .await
        .unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));

    let size = 3 * 1024 * 1024 + 17;
    let header = FileHeader {
        relative_path: "big.bin".into(),
        size: size as u64,
        mtime_seconds: 1_600_000_000,
        permissions: 0o644,
        checksum: vec![],
    };
    peer.send(wire(Frame::ManifestEntry(header.clone())))
        .await
        .unwrap();
    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
        scan_complete: true,
    })))
    .await
    .unwrap();

    // Missing at the destination → must be granted.
    let mut granted = false;
    loop {
        match recv_or_panic(&mut peer).await {
            Frame::NeedBatch(batch) => {
                granted |= batch.entries.iter().any(|e| e.relative_path == "big.bin");
            }
            Frame::NeedComplete(_) => break,
            other => panic!("expected need choreography, got {other:?}"),
        }
    }
    assert!(granted, "'big.bin' must be on the need list");

    // Open the record, land ONE partial frame, cancel mid-record.
    peer.send(wire(Frame::FileBegin(header))).await.unwrap();
    peer.send(wire(Frame::FileData(FileData {
        content: make_patterned(64 * 1024),
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::Error(SessionError {
        code: session_error::Code::Cancelled as i32,
        message: "job cancelled by operator".into(),
        ..Default::default()
    })))
    .await
    .unwrap();

    let dest_err = tokio::time::timeout(SUITE_TIMEOUT, dest)
        .await
        .expect("destination must not hang on a cancelled record")
        .unwrap()
        .expect_err("a session cancelled mid-record must not report success");
    let fault = fault_of(&dest_err);
    assert_eq!(
        fault.code,
        session_error::Code::Cancelled,
        "the peer's CANCELLED must own the outcome — not a violation \
         about frame position; got: {dest_err:#}"
    );
    assert!(
        fault.message.contains("cancelled by operator"),
        "the peer's message must survive the wire, got: {}",
        fault.message
    );
    // The 64 KiB partial must not be finalized as 'big.bin'.
    let final_path = dst_root.join("big.bin");
    if let Ok(meta) = std::fs::metadata(&final_path) {
        assert!(
            meta.len() < size as u64,
            "a cancelled record must never finalize at full size"
        );
    }
}

#[tokio::test]
async fn incomplete_scan_refused_when_completeness_required() {
    // codex otp-9b F1 (R49-F2 on the session): an initiator that
    // declared require_complete_scan (`blit move` — the source is
    // deleted after success) must NOT get a success out of an
    // incomplete source scan; files the scan could not read would be
    // silently lost with the source. The destination refuses at
    // ManifestComplete{scan_complete=false} with SCAN_INCOMPLETE.
    // Scripted source peer so we control the flag.
    let tmp = tempfile::tempdir().unwrap();
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&dst_root).unwrap();

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
        DestinationTarget::Fixed(dst_root.clone()),
    ));

    let mut open = basic_open(TransferRole::Source);
    open.require_complete_scan = true;
    peer.send(hello_frame()).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
    peer.send(wire(Frame::Open(open))).await.unwrap();
    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));

    peer.send(wire(Frame::ManifestEntry(FileHeader {
        relative_path: "present.txt".into(),
        size: 1,
        mtime_seconds: 1_600_000_000,
        permissions: 0o644,
        checksum: vec![],
    })))
    .await
    .unwrap();
    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
        scan_complete: false,
    })))
    .await
    .unwrap();

    // Bounded wait: an implementation that fails to refuse proceeds to
    // the payload phase and would otherwise hang this scripted peer.
    let refusal = tokio::time::timeout(SUITE_TIMEOUT, async {
        loop {
            match recv_or_panic(&mut peer).await {
                Frame::Error(e) => break e,
                Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
                other => panic!("expected SessionError, got {other:?}"),
            }
        }
    })
    .await
    .expect("destination must refuse the incomplete scan, not proceed");
    assert_eq!(refusal.code, session_error::Code::ScanIncomplete as i32);
    let dest_err = dest.await.unwrap().unwrap_err();
    assert_eq!(
        fault_of(&dest_err).code,
        session_error::Code::ScanIncomplete
    );
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
/// — the `TransferSource` contract permits that, and the since-deleted relay
/// source (`RemoteTransferSource`, removed at otp-10c-1) really did it. Used
/// to prove the session applies filters via the universal `FilteredSource`
/// chokepoint, not via the per-impl `scan(filter)` arg (codex otp-6a F1). If
/// the session ever reverts to threading the filter through `scan`, this
/// source drops it and every file transfers.
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
        // The filter arg is discarded on purpose (models a source impl
        // that ignores it, as the deleted relay source did).
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
    // honoring the scan(filter) argument — the contract lets an impl
    // ignore it (the deleted relay source did). Drive a push session
    // whose source ignores the scan arg; the filter must still apply
    // because the session wraps it in FilteredSource.
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
        instruments: Default::default(),
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
        receiver_capacity: None,
        instruments: Default::default(),
        local_apply: None,
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
        instruments: Default::default(),
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
        instruments: Default::default(),
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
        instruments: Default::default(),
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

/// otp-10b-1: a Checksum session is a CONTENT compare — a file whose
/// bytes are identical but whose mtime differs must SKIP (the old
/// pull's `--checksum` behavior, now role-agnostic), under both
/// initiator layouts. Control: the same fixture under SizeMtime
/// transfers (source mtime is newer), proving the skip is the
/// checksum's doing and the pin is not vacuous.
#[tokio::test]
async fn checksum_compare_skips_content_equal_files_regardless_of_mtime() {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        for (mode, expected) in [
            (ComparisonMode::SizeMtime, 1u64), // control: mtime differs
            (ComparisonMode::Checksum, 0u64),  // content-equal skips
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let src_root = tmp.path().join("src");
            let dst_root = tmp.path().join("dst");
            std::fs::create_dir_all(&src_root).unwrap();
            std::fs::create_dir_all(&dst_root).unwrap();
            write_tree(&src_root, &[("same.bin", vec![7u8; 4096], 2_000)]);
            write_tree(&dst_root, &[("same.bin", vec![7u8; 4096], 1_000)]);

            let open = SessionOpen {
                compare_mode: mode as i32,
                ..basic_open(initiator_role)
            };
            let (source_result, dest_result) =
                run_session_with_open(open, &src_root, &dst_root, PlanOptions::default()).await;
            let summary = source_result.unwrap_or_else(|e| {
                panic!("source failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
            });
            dest_result.unwrap_or_else(|e| {
                panic!("destination failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
            });
            assert_eq!(
                summary.files_transferred, expected,
                "{mode:?} under initiator {initiator_role:?}"
            );
        }
    }
}

/// otp-10b-1: the cell SizeMtime provably misses — same size, same
/// mtime, DIFFERENT content — must transfer under Checksum, both
/// initiator layouts. Control first: SizeMtime skips it (that is the
/// documented weakness `--checksum` exists for).
#[tokio::test]
async fn checksum_compare_transfers_content_change_size_mtime_misses() {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        for (mode, expected) in [
            (ComparisonMode::SizeMtime, 0u64), // control: looks up to date
            (ComparisonMode::Checksum, 1u64),  // content differs
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let src_root = tmp.path().join("src");
            let dst_root = tmp.path().join("dst");
            std::fs::create_dir_all(&src_root).unwrap();
            std::fs::create_dir_all(&dst_root).unwrap();
            write_tree(&src_root, &[("stealth.bin", vec![1u8; 4096], 1_000)]);
            write_tree(&dst_root, &[("stealth.bin", vec![2u8; 4096], 1_000)]);

            let open = SessionOpen {
                compare_mode: mode as i32,
                ..basic_open(initiator_role)
            };
            let (source_result, dest_result) =
                run_session_with_open(open, &src_root, &dst_root, PlanOptions::default()).await;
            let summary = source_result.unwrap_or_else(|e| {
                panic!("source failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
            });
            dest_result.unwrap_or_else(|e| {
                panic!("destination failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
            });
            assert_eq!(
                summary.files_transferred, expected,
                "{mode:?} under initiator {initiator_role:?}"
            );
            if mode == ComparisonMode::Checksum {
                assert_trees_identical(&src_root, &dst_root);
            }
        }
    }
}
