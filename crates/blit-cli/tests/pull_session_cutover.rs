//! otp-10b-2: the pull-shaped verb rides the unified transfer session.
//!
//! `blit_app::transfers::remote::run_remote_pull` — the one entry both
//! the CLI verbs and the TUI F3 trigger call — initiates a
//! DESTINATION-role `Transfer` session instead of driving the old
//! pull driver. These pins guard the verb-level option wiring that
//! cutover added (wire filter, in-session mirror, `--force-grpc`,
//! progress events, resume, the move-shaped compare mapping, single-
//! file layout, `--ignore-existing`) plus the absolute tree/count
//! pin that replaced the otp-10b-2 A/B parity run when the old
//! driver was deleted (otp-10c-2).
//!
//! Each test spawns a real daemon via the shared harness and calls the
//! library entry in-process — the same boundary the verbs use, minus
//! the clap/printer skin (which `remote_parity` and friends cover by
//! running the actual binary). Mirrors `push_session_cutover.rs`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod common;
use common::TestContext;

use blit_app::transfers::remote::{run_remote_pull, PullExecution};
use blit_core::generated::{ComparisonMode, FilterSpec, MirrorMode};
use blit_core::remote::transfer::{ProgressEvent, ProgressTotals, RemoteTransferProgress};
use blit_core::remote::{RemoteEndpoint, RemotePath};

fn module_endpoint(port: u16) -> RemoteEndpoint {
    RemoteEndpoint {
        host: "127.0.0.1".to_string(),
        port,
        path: RemotePath::Module {
            module: "test".to_string(),
            rel_path: PathBuf::new(),
        },
    }
}

/// A `PullExecution` with the verb defaults (`blit copy host:/test/ DEST`).
fn pull_execution(port: u16, dest_root: &Path) -> PullExecution {
    PullExecution {
        remote: module_endpoint(port),
        dest_root: dest_root.to_path_buf(),
        filter: None,
        mirror_mode: false,
        mirror_kind: MirrorMode::Off,
        force_grpc: false,
        trace_data_plane: false,
        require_complete_scan: false,
        drop_windows_metadata: false,
        resume: false,
        resume_block_size: 0,
        compare_mode: ComparisonMode::SizeMtime,
        ignore_existing: false,
        remote_label: format!("127.0.0.1:{port}:/test/"),
    }
}

/// Deterministic mixed fixture: nested dirs, an empty file, small
/// files (tar-shard candidates), and one multi-megabyte file (raw
/// candidate). Same shape as the push cutover fixture.
fn write_fixture(src: &Path) -> (usize, u64) {
    fs::create_dir_all(src.join("nested/deeper")).expect("fixture dirs");
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("empty.bin", Vec::new()),
        ("small_a.txt", b"alpha".to_vec()),
        ("nested/small_b.txt", b"beta-beta".to_vec()),
        ("nested/deeper/small_c.txt", vec![b'c'; 3000]),
        (
            "big.bin",
            (0..u32::try_from(3 * 1024 * 1024).unwrap())
                .map(|i| (i % 251) as u8)
                .collect(),
        ),
    ];
    let mut total = 0u64;
    for (rel, bytes) in &files {
        total += bytes.len() as u64;
        fs::write(src.join(rel), bytes).expect("write fixture file");
    }
    (files.len(), total)
}

/// Relative path → content for every file under `root`.
fn tree_contents(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.expect("walk tree");
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(root)
                .expect("relative path")
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, fs::read(entry.path()).expect("read file"));
        }
    }
    out
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

/// otp-10b-2's A/B parity pin, converted to an ABSOLUTE pin at
/// otp-10c-2 (the old-driver reference arm died with the driver):
/// the session-backed verb entry lands a byte-identical tree and
/// reports summary counts equal to the fixture's own totals — the
/// exact facts the A/B equality proved transitively. Performance
/// parity is carried by the committed otp-2/otp-2w baselines and
/// otp-12's interleaved old-binary acceptance runs.
#[test]
fn pull_verb_lands_identical_tree_with_exact_counts() {
    let ctx = TestContext::new();

    let (fixture_files, fixture_bytes) = write_fixture(&ctx.module_dir);
    let new_dest = ctx.workspace.join("new_dest");

    let new_summary = runtime()
        .block_on(run_remote_pull(
            pull_execution(ctx.daemon_port, &new_dest),
            None,
        ))
        .expect("session pull")
        .summary;

    let new_tree = tree_contents(&new_dest);
    assert_eq!(
        new_tree,
        tree_contents(&ctx.module_dir),
        "destination tree differs from source module"
    );

    assert_eq!(
        new_summary.files_transferred, fixture_files as u64,
        "files_transferred must equal the fixture count"
    );
    assert_eq!(
        new_summary.bytes_transferred, fixture_bytes,
        "bytes_transferred must equal the fixture byte total"
    );
    assert!(
        !new_summary.in_stream_carrier_used,
        "default carrier must be the TCP data plane"
    );
}

/// Mirror through the verb entry: the session DESTINATION (this end)
/// deletes the extraneous local entry at SourceDone (the one delete
/// rule — no post-pull purge step exists on this path) and scores it.
#[test]
fn pull_verb_mirror_purges_extraneous_and_scores_deletions() {
    let ctx = TestContext::new();
    write_fixture(&ctx.module_dir);
    let dest = ctx.workspace.join("dest");
    fs::create_dir_all(&dest).expect("dest dir");
    fs::write(dest.join("stale.txt"), b"delete me").expect("seed extraneous");

    let summary = runtime()
        .block_on(async {
            let execution = PullExecution {
                mirror_mode: true,
                mirror_kind: MirrorMode::All,
                ..pull_execution(ctx.daemon_port, &dest)
            };
            run_remote_pull(execution, None).await.expect("mirror pull")
        })
        .summary;

    assert!(
        !dest.join("stale.txt").exists(),
        "extraneous local file must be purged"
    );
    assert!(
        summary.entries_deleted >= 1,
        "summary must score the purge, got {}",
        summary.entries_deleted
    );
    assert_eq!(tree_contents(&dest), tree_contents(&ctx.module_dir));
}

/// The verb's filter rides `SessionOpen.filter` and scopes the daemon
/// SOURCE's scan (otp-6a chokepoint): excluded files never land.
#[test]
fn pull_verb_wire_filter_scopes_the_source_scan() {
    let ctx = TestContext::new();
    write_fixture(&ctx.module_dir);
    fs::write(ctx.module_dir.join("noise.log"), b"excluded").expect("write excluded file");
    let dest = ctx.workspace.join("dest");

    runtime().block_on(async {
        let execution = PullExecution {
            filter: Some(FilterSpec {
                exclude: vec!["*.log".to_string()],
                ..Default::default()
            }),
            ..pull_execution(ctx.daemon_port, &dest)
        };
        run_remote_pull(execution, None)
            .await
            .expect("filtered pull")
    });

    assert!(
        !dest.join("noise.log").exists(),
        "excluded file must not land"
    );
    assert!(dest.join("big.bin").exists(), "in-scope files must land");
}

/// `--force-grpc` maps to the session's in-stream byte carrier and the
/// summary attests to it (the printers' `[gRPC fallback]` marker).
#[test]
fn pull_verb_force_grpc_rides_the_in_stream_carrier() {
    let ctx = TestContext::new();
    write_fixture(&ctx.module_dir);
    let dest = ctx.workspace.join("dest");

    let summary = runtime()
        .block_on(async {
            let execution = PullExecution {
                force_grpc: true,
                ..pull_execution(ctx.daemon_port, &dest)
            };
            run_remote_pull(execution, None)
                .await
                .expect("forced in-stream pull")
        })
        .summary;

    assert!(
        summary.in_stream_carrier_used,
        "force_grpc must select the in-stream carrier"
    );
    assert_eq!(tree_contents(&dest), tree_contents(&ctx.module_dir));
}

/// The verb emits the w6-1 progress contract from the session
/// DESTINATION (otp-10b-2): need batches as the denominator, one
/// FileComplete per file, bytes riding Payload. On a fresh destination
/// every file is needed, so files and the denominator equal the
/// fixture exactly; bytes are bounded below by the content (the
/// data-plane tar-shard lane reports wire bytes, so a small archive
/// overhead above content size is legal).
#[test]
fn pull_verb_reports_w6_1_progress_events() {
    let ctx = TestContext::new();
    let (file_count, byte_count) = write_fixture(&ctx.module_dir);
    let dest = ctx.workspace.join("dest");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ProgressEvent>();
    runtime().block_on(async {
        let progress = RemoteTransferProgress::new(tx);
        run_remote_pull(pull_execution(ctx.daemon_port, &dest), Some(&progress))
            .await
            .expect("pull with progress");
    });

    let mut totals = ProgressTotals::default();
    let mut file_completes = 0usize;
    while let Ok(event) = rx.try_recv() {
        if matches!(event, ProgressEvent::FileComplete { .. }) {
            file_completes += 1;
        }
        totals.apply(&event);
    }
    assert_eq!(
        totals.manifest_files, file_count as u64,
        "need-batch denominator must count every needed file"
    );
    assert_eq!(totals.manifest_bytes, byte_count);
    assert_eq!(totals.files, file_count as u64, "files counted once each");
    assert_eq!(
        file_completes, file_count,
        "per-file lane, one event per file"
    );
    assert!(
        totals.bytes >= byte_count && totals.bytes <= byte_count + 1024 * 1024,
        "bytes ride Payload (content, plus bounded tar-shard framing), got {} for {} content",
        totals.bytes,
        byte_count
    );
}

/// codex otp-10a F1, mirrored on pull (otp-10b-2): a move-shaped pull
/// (`ComparisonMode::IgnoreTimes`) must transfer a same-size file even
/// when the local destination copy is NEWER — move deletes the REMOTE
/// source afterwards, so a compare-mode skip of content that silently
/// differs would destroy the only copy. The copy-shaped default
/// (SizeMtime) skips this cell by design (the standing owner
/// question); move must not — the control half proves the pin is not
/// vacuous.
#[test]
fn move_shaped_pull_transfers_same_size_newer_destination() {
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("data.bin"), b"source-bytes").expect("write source");

    let seed_dest = |dest: &Path| {
        fs::create_dir_all(dest).expect("dest dir");
        // Same size, different content, NEWER mtime at the destination.
        fs::write(dest.join("data.bin"), b"dest---bytes").expect("seed dest");
        let newer = filetime::FileTime::from_unix_time(
            filetime::FileTime::from_last_modification_time(
                &fs::metadata(ctx.module_dir.join("data.bin")).expect("meta"),
            )
            .unix_seconds()
                + 60,
            0,
        );
        filetime::set_file_mtime(dest.join("data.bin"), newer).expect("bump dest mtime");
    };

    let move_dest = ctx.workspace.join("move_dest");
    seed_dest(&move_dest);
    runtime().block_on(async {
        let execution = PullExecution {
            compare_mode: ComparisonMode::IgnoreTimes,
            require_complete_scan: true,
            ..pull_execution(ctx.daemon_port, &move_dest)
        };
        run_remote_pull(execution, None)
            .await
            .expect("move-shaped pull")
    });
    assert_eq!(
        fs::read(move_dest.join("data.bin")).expect("read dest"),
        b"source-bytes",
        "move-shaped pull must land the source bytes before the remote source is deleted"
    );

    // Control: the copy-shaped default skips the same cell — proving
    // the IgnoreTimes assertion above actually exercised the mapping.
    let copy_dest = ctx.workspace.join("copy_dest");
    seed_dest(&copy_dest);
    runtime().block_on(async {
        run_remote_pull(pull_execution(ctx.daemon_port, &copy_dest), None)
            .await
            .expect("copy-shaped pull")
    });
    assert_eq!(
        fs::read(copy_dest.join("data.bin")).expect("read dest"),
        b"dest---bytes",
        "SizeMtime keeps the newer destination (data-safe skip)"
    );
}

/// The verb-level `--checksum` mapping (the one compare mapping, both
/// verbs): a content-equal file skips regardless of mtime — the
/// SizeMtime control transfers the same cell, proving the Checksum leg
/// exercised the wire mode.
#[test]
fn checksum_pull_skips_content_equal_files_regardless_of_mtime() {
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("same.bin"), b"identical-content").expect("write source");

    let seed_dest = |dest: &Path| {
        fs::create_dir_all(dest).expect("dest dir");
        // Same content, mtime pushed far from the source's.
        fs::write(dest.join("same.bin"), b"identical-content").expect("seed dest");
        filetime::set_file_mtime(
            dest.join("same.bin"),
            filetime::FileTime::from_unix_time(1_000_000, 0),
        )
        .expect("skew dest mtime");
    };

    let checksum_dest = ctx.workspace.join("checksum_dest");
    seed_dest(&checksum_dest);
    let summary = runtime()
        .block_on(async {
            let execution = PullExecution {
                compare_mode: ComparisonMode::Checksum,
                ..pull_execution(ctx.daemon_port, &checksum_dest)
            };
            run_remote_pull(execution, None)
                .await
                .expect("checksum pull")
        })
        .summary;
    assert_eq!(
        summary.files_transferred, 0,
        "content-equal file must skip under Checksum compare"
    );

    // Control: SizeMtime sees the mtime skew and transfers.
    let control_dest = ctx.workspace.join("control_dest");
    seed_dest(&control_dest);
    let summary = runtime()
        .block_on(async {
            run_remote_pull(pull_execution(ctx.daemon_port, &control_dest), None)
                .await
                .expect("control pull")
        })
        .summary;
    assert_eq!(
        summary.files_transferred, 1,
        "SizeMtime control must transfer the mtime-skewed file"
    );
}

/// `--ignore-existing` rides `SessionOpen.ignore_existing` (otp-10b-2
/// wired it for both verbs): an existing destination file is skipped
/// whatever its content, and never overwritten.
#[test]
fn ignore_existing_pull_skips_existing_destination_files() {
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("keep.bin"), b"new-source-content").expect("write source");
    let dest = ctx.workspace.join("dest");
    fs::create_dir_all(&dest).expect("dest dir");
    fs::write(dest.join("keep.bin"), b"old").expect("seed dest");

    let summary = runtime()
        .block_on(async {
            let execution = PullExecution {
                ignore_existing: true,
                ..pull_execution(ctx.daemon_port, &dest)
            };
            run_remote_pull(execution, None)
                .await
                .expect("ignore-existing pull")
        })
        .summary;

    assert_eq!(summary.files_transferred, 0, "existing file must skip");
    assert_eq!(
        fs::read(dest.join("keep.bin")).expect("read dest"),
        b"old",
        "existing destination content must be preserved"
    );
}

/// Single-file pull layout (the old pull's convention, kept by the
/// session): the source manifests a file root with an empty relative
/// path, and the verb's `dest_root` is the target FILE path — the
/// bytes land exactly there, with the parent created on demand.
#[test]
fn single_file_pull_lands_at_the_target_file_path() {
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("payload.txt"), b"single-file-bytes").expect("write source");

    // Parent dir does not exist yet — the session sink creates each
    // write target's parent chain (the old pull pre-created it at the
    // verb layer; the session needs no such step).
    let target = ctx.workspace.join("fresh_dir").join("payload.txt");
    let endpoint = RemoteEndpoint {
        host: "127.0.0.1".to_string(),
        port: ctx.daemon_port,
        path: RemotePath::Module {
            module: "test".to_string(),
            rel_path: PathBuf::from("payload.txt"),
        },
    };

    runtime().block_on(async {
        let execution = PullExecution {
            remote: endpoint,
            dest_root: target.clone(),
            ..pull_execution(ctx.daemon_port, &target)
        };
        run_remote_pull(execution, None)
            .await
            .expect("single-file pull")
    });

    assert!(target.is_file(), "bytes must land AT the target path");
    assert_eq!(
        fs::read(&target).expect("read target"),
        b"single-file-bytes"
    );
}

/// A daemon started with `--force-grpc-data` never grants a TCP data
/// plane — a session pull against it rides the in-stream carrier even
/// though the client did not ask for it. Pins the SOURCE-responder
/// half of the otp-10a F3 policy (the push pin covers the DESTINATION
/// responder).
#[test]
fn daemon_force_grpc_data_forces_the_in_stream_carrier_on_pull() {
    let ctx = TestContext::builder()
        .extra_daemon_args(["--force-grpc-data"])
        .build();
    write_fixture(&ctx.module_dir);
    let dest = ctx.workspace.join("dest");

    let summary = runtime()
        .block_on(run_remote_pull(
            pull_execution(ctx.daemon_port, &dest), // client does NOT force
            None,
        ))
        .expect("pull against a fallback-forced daemon")
        .summary;

    assert!(
        summary.in_stream_carrier_used,
        "--force-grpc-data on the daemon must force the in-stream carrier"
    );
    assert_eq!(tree_contents(&dest), tree_contents(&ctx.module_dir));
}

/// `--resume` through the verb: a changed same-size destination file
/// is patched block-wise (`files_resumed` scored), lands byte-identical,
/// and reports w6-1 progress (counted once; bytes = the stale blocks
/// received, less than the whole file) — then the same contract on the
/// forced in-stream carrier.
#[test]
fn pull_verb_resume_patches_changed_partials_blockwise() {
    let ctx = TestContext::new();
    let big: Vec<u8> = (0..u32::try_from(4 * 1024 * 1024).unwrap())
        .map(|i| (i % 241) as u8)
        .collect();
    fs::write(ctx.module_dir.join("patch.bin"), &big).expect("write v1");
    let dest = ctx.workspace.join("dest");

    runtime().block_on(async {
        run_remote_pull(pull_execution(ctx.daemon_port, &dest), None)
            .await
            .expect("seed pull");
    });

    // Change the SOURCE: same size, changed slice, newer mtime — the
    // local copy becomes a resume candidate.
    let mut v2 = big.clone();
    for b in &mut v2[2 * 1024 * 1024..2 * 1024 * 1024 + 4096] {
        *b ^= 0xFF;
    }
    fs::write(ctx.module_dir.join("patch.bin"), &v2).expect("write v2");
    let bumped = filetime::FileTime::from_unix_time(
        filetime::FileTime::from_last_modification_time(
            &fs::metadata(ctx.module_dir.join("patch.bin")).expect("meta"),
        )
        .unix_seconds()
            + 5,
        0,
    );
    filetime::set_file_mtime(ctx.module_dir.join("patch.bin"), bumped).expect("bump mtime");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ProgressEvent>();
    let summary = runtime()
        .block_on(async {
            let progress = RemoteTransferProgress::new(tx);
            let execution = PullExecution {
                resume: true,
                ..pull_execution(ctx.daemon_port, &dest)
            };
            run_remote_pull(execution, Some(&progress))
                .await
                .expect("resume pull")
        })
        .summary;

    assert_eq!(summary.files_resumed, 1, "changed partial must resume");
    assert_eq!(
        fs::read(dest.join("patch.bin")).expect("read dest"),
        v2,
        "patched destination must match the new source"
    );

    let mut totals = ProgressTotals::default();
    while let Ok(event) = rx.try_recv() {
        totals.apply(&event);
    }
    assert_eq!(
        totals.manifest_files, 1,
        "the resumed need is the denominator"
    );
    assert_eq!(totals.manifest_bytes, v2.len() as u64);
    assert_eq!(totals.files, 1, "a resumed file completes exactly once");
    assert!(
        totals.bytes > 0 && totals.bytes < v2.len() as u64,
        "resume progress reports the stale blocks received, got {} of {}",
        totals.bytes,
        v2.len()
    );

    // Same contract on the in-stream carrier (the control-lane Block
    // arms report independently of the data-plane pipeline).
    let mut v3 = v2.clone();
    for b in &mut v3[..4096] {
        *b ^= 0xAA;
    }
    fs::write(ctx.module_dir.join("patch.bin"), &v3).expect("write v3");
    let bumped_again = filetime::FileTime::from_unix_time(bumped.unix_seconds() + 5, 0);
    filetime::set_file_mtime(ctx.module_dir.join("patch.bin"), bumped_again)
        .expect("bump mtime again");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ProgressEvent>();
    let summary = runtime()
        .block_on(async {
            let progress = RemoteTransferProgress::new(tx);
            let execution = PullExecution {
                resume: true,
                force_grpc: true,
                ..pull_execution(ctx.daemon_port, &dest)
            };
            run_remote_pull(execution, Some(&progress))
                .await
                .expect("in-stream resume pull")
        })
        .summary;

    assert!(
        summary.in_stream_carrier_used,
        "second round must ride the in-stream carrier"
    );
    assert_eq!(summary.files_resumed, 1, "in-stream resume must score");
    assert_eq!(
        fs::read(dest.join("patch.bin")).expect("read dest"),
        v3,
        "in-stream patched destination must match the new source"
    );
    let mut totals = ProgressTotals::default();
    while let Ok(event) = rx.try_recv() {
        totals.apply(&event);
    }
    assert_eq!(totals.files, 1, "in-stream resumed file completes once");
    assert!(
        totals.bytes > 0 && totals.bytes < v3.len() as u64,
        "in-stream resume reports the stale blocks received, got {} of {}",
        totals.bytes,
        v3.len()
    );
}
