//! otp-10a: the push-shaped verb rides the unified transfer session.
//!
//! `blit_app::transfers::remote::run_remote_push` — the one entry both
//! the CLI verbs and the TUI F1 trigger call — now initiates a
//! SOURCE-role `Transfer` session instead of driving the old
//! per-direction push client. These pins guard the verb-level option
//! wiring that cutover added (wire filter, mirror, `--force-grpc`,
//! progress events, resume flags, the unreadable-scan error `blit
//! move`'s source-delete gate relies on) plus an A/B parity run
//! against the old driver, which stays in-tree until otp-10c deletes
//! it.
//!
//! Each test spawns a real daemon via the shared harness and calls the
//! library entry in-process — the same boundary the verbs use, minus
//! the clap/printer skin (which `remote_parity` and friends cover by
//! running the actual binary).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod common;
use common::TestContext;

use blit_app::endpoints::Endpoint;
use blit_app::transfers::remote::{run_remote_push, PushExecution};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::{ComparisonMode, FilterSpec, MirrorMode};
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::remote::transfer::{ProgressEvent, ProgressTotals, RemoteTransferProgress};
use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};

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

/// A `PushExecution` with the verb defaults (`blit copy SRC host:/test/`).
fn push_execution(src: &Path, port: u16) -> PushExecution {
    PushExecution {
        source: Endpoint::Local(src.to_path_buf()),
        remote: module_endpoint(port),
        filter: None,
        mirror_mode: false,
        mirror_kind: MirrorMode::Off,
        force_grpc: false,
        trace_data_plane: false,
        require_complete_scan: false,
        resume: false,
        resume_block_size: 0,
        compare_mode: ComparisonMode::SizeMtime,
        ignore_existing: false,
        remote_label: format!("127.0.0.1:{port}:/test/"),
    }
}

/// Deterministic mixed fixture: nested dirs, an empty file, small
/// files (tar-shard candidates), and one multi-megabyte file (raw
/// candidate).
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

/// A/B parity (the otp-10a converge guard): the session-backed verb
/// entry and the old push driver land byte-identical trees with equal
/// summary counts, from the same source, against real daemons. The old
/// driver is deleted at otp-10c; until then it is the reference.
#[test]
fn push_verb_and_old_push_produce_identical_trees_and_counts() {
    let ctx = TestContext::new();
    let old_daemon = ctx.spawn_second_daemon("old", &Default::default());

    let src = ctx.workspace.join("src");
    write_fixture(&src);

    let (new_summary, old_report) = runtime().block_on(async {
        let new_summary = run_remote_push(push_execution(&src, ctx.daemon_port), None)
            .await
            .expect("session push")
            .summary;

        let mut client = RemotePushClient::connect(module_endpoint(old_daemon.port))
            .await
            .expect("old push connect");
        let old_report = client
            .push(
                Arc::new(FsTransferSource::new(src.clone())),
                &FileFilter::default(),
                false,
                MirrorMode::Off,
                false,
                false,
                None,
                false,
            )
            .await
            .expect("old push");
        (new_summary, old_report)
    });

    let new_tree = tree_contents(&ctx.module_dir);
    let old_tree = tree_contents(&old_daemon.module_dir);
    assert_eq!(new_tree, old_tree, "session vs old push trees differ");
    assert_eq!(
        new_tree,
        tree_contents(&src),
        "destination tree differs from source"
    );

    assert_eq!(
        new_summary.files_transferred, old_report.summary.files_transferred,
        "files_transferred parity"
    );
    assert_eq!(
        new_summary.bytes_transferred, old_report.summary.bytes_transferred,
        "bytes_transferred parity"
    );
    assert!(
        !new_summary.in_stream_carrier_used,
        "default carrier must be the TCP data plane"
    );
}

/// Mirror through the verb entry: the wire `mirror_enabled` +
/// `MirrorMode::All` reach the daemon DESTINATION, which purges the
/// extraneous entry (the one delete rule) and scores it.
#[test]
fn push_verb_mirror_purges_extraneous_and_scores_deletions() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    write_fixture(&src);
    fs::write(ctx.module_dir.join("stale.txt"), b"delete me").expect("seed extraneous");

    let summary = runtime()
        .block_on(async {
            let execution = PushExecution {
                mirror_mode: true,
                mirror_kind: MirrorMode::All,
                require_complete_scan: true,
                ..push_execution(&src, ctx.daemon_port)
            };
            run_remote_push(execution, None).await.expect("mirror push")
        })
        .summary;

    assert!(
        !ctx.module_dir.join("stale.txt").exists(),
        "extraneous file must be purged"
    );
    assert!(
        summary.entries_deleted >= 1,
        "summary must score the purge, got {}",
        summary.entries_deleted
    );
    assert_eq!(tree_contents(&ctx.module_dir), tree_contents(&src));
}

/// The verb's filter rides `SessionOpen.filter` and scopes the
/// SOURCE-side scan (otp-6a chokepoint): excluded files never land.
#[test]
fn push_verb_wire_filter_scopes_the_source_scan() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    write_fixture(&src);
    fs::write(src.join("noise.log"), b"excluded").expect("write excluded file");

    runtime().block_on(async {
        let execution = PushExecution {
            filter: Some(FilterSpec {
                exclude: vec!["*.log".to_string()],
                ..Default::default()
            }),
            ..push_execution(&src, ctx.daemon_port)
        };
        run_remote_push(execution, None)
            .await
            .expect("filtered push")
    });

    assert!(
        !ctx.module_dir.join("noise.log").exists(),
        "excluded file must not land"
    );
    assert!(
        ctx.module_dir.join("big.bin").exists(),
        "in-scope files must land"
    );
}

/// `--force-grpc` maps to the session's in-stream byte carrier and the
/// summary attests to it (the printers' `[gRPC fallback]` marker).
#[test]
fn push_verb_force_grpc_rides_the_in_stream_carrier() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    write_fixture(&src);

    let summary = runtime()
        .block_on(async {
            let execution = PushExecution {
                force_grpc: true,
                ..push_execution(&src, ctx.daemon_port)
            };
            run_remote_push(execution, None)
                .await
                .expect("forced in-stream push")
        })
        .summary;

    assert!(
        summary.in_stream_carrier_used,
        "force_grpc must select the in-stream carrier"
    );
    assert_eq!(tree_contents(&ctx.module_dir), tree_contents(&src));
}

/// The verb emits the w6-1 progress contract from the session SOURCE:
/// need batches as the denominator, one FileComplete per file, bytes
/// riding Payload — folded through the shared `ProgressTotals`, they
/// must equal the fixture exactly (fresh destination ⇒ every file is
/// needed).
#[test]
fn push_verb_reports_w6_1_progress_events() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    let (file_count, byte_count) = write_fixture(&src);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ProgressEvent>();
    runtime().block_on(async {
        let progress = RemoteTransferProgress::new(tx);
        run_remote_push(push_execution(&src, ctx.daemon_port), Some(&progress))
            .await
            .expect("push with progress");
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
    assert_eq!(totals.files, file_count as u64, "files counted once each");
    assert_eq!(
        file_completes, file_count,
        "per-file lane, one event per file"
    );
    assert_eq!(
        totals.bytes, byte_count,
        "bytes ride Payload, planned sizes"
    );
}

/// Old-push posture on unreadable source entries (the gate `blit move`
/// relies on): the readable subset still transfers, then the call
/// errors naming the skip — success must never mask silently-missing
/// files, because move deletes the source on success.
#[cfg(unix)]
#[test]
fn push_verb_fails_when_source_has_unreadable_entries() {
    use std::os::unix::fs::PermissionsExt;

    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(src.join("locked")).expect("locked dir");
    fs::write(src.join("readable.txt"), b"fine").expect("readable file");
    fs::write(src.join("locked/secret.txt"), b"unreachable").expect("locked file");
    fs::set_permissions(src.join("locked"), fs::Permissions::from_mode(0o000)).expect("lock dir");

    let result = runtime().block_on(run_remote_push(push_execution(&src, ctx.daemon_port), None));

    // Restore before asserting so the tempdir always cleans up.
    fs::set_permissions(src.join("locked"), fs::Permissions::from_mode(0o755)).expect("unlock dir");

    let err = match result {
        Ok(_) => panic!("unreadable source entries must fail the push"),
        Err(err) => err,
    };
    assert!(
        format!("{err:#}").contains("skipped due to permission or access errors"),
        "error must name the skip, got: {err:#}"
    );
    assert!(
        ctx.module_dir.join("readable.txt").exists(),
        "readable subset still lands (send-what's-readable, then error)"
    );
}

/// codex otp-10a F1: a move-shaped push (`ComparisonMode::IgnoreTimes`)
/// must transfer a same-size file even when the destination copy is
/// NEWER — move deletes the source afterwards, so a compare-mode skip
/// of content that silently differs would destroy the only copy of the
/// source bytes. The copy-shaped default (SizeMtime) skips this cell by
/// design (the standing owner question); move must not.
#[test]
fn move_shaped_push_transfers_same_size_newer_destination() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).expect("src dir");
    fs::write(src.join("data.bin"), b"source-bytes").expect("write source");

    // Same size, different content, NEWER mtime at the destination.
    fs::write(ctx.module_dir.join("data.bin"), b"dest---bytes").expect("seed dest");
    let newer = filetime::FileTime::from_unix_time(
        filetime::FileTime::from_last_modification_time(
            &fs::metadata(src.join("data.bin")).expect("meta"),
        )
        .unix_seconds()
            + 60,
        0,
    );
    filetime::set_file_mtime(ctx.module_dir.join("data.bin"), newer).expect("bump dest mtime");

    runtime().block_on(async {
        let execution = PushExecution {
            compare_mode: ComparisonMode::IgnoreTimes,
            ..push_execution(&src, ctx.daemon_port)
        };
        run_remote_push(execution, None)
            .await
            .expect("move-shaped push")
    });

    assert_eq!(
        fs::read(ctx.module_dir.join("data.bin")).expect("read dest"),
        b"source-bytes",
        "move-shaped push must land the source bytes before the source is deleted"
    );
}

/// codex otp-10a F3: a daemon started with `--force-grpc-data` never
/// grants a TCP data plane — a session push against it rides the
/// in-stream carrier even though the client did not ask for it, the
/// same server-side force the old push handler honored.
#[test]
fn daemon_force_grpc_data_forces_the_in_stream_carrier() {
    let ctx = TestContext::builder()
        .extra_daemon_args(["--force-grpc-data"])
        .build();
    let src = ctx.workspace.join("src");
    write_fixture(&src);

    let summary = runtime()
        .block_on(run_remote_push(
            push_execution(&src, ctx.daemon_port), // client does NOT force
            None,
        ))
        .expect("push against a fallback-forced daemon")
        .summary;

    assert!(
        summary.in_stream_carrier_used,
        "--force-grpc-data on the daemon must force the in-stream carrier"
    );
    assert_eq!(tree_contents(&ctx.module_dir), tree_contents(&src));
}

/// codex otp-10a F4: `--relay-via-cli --resume` is refused up front —
/// the relay's remote source cannot serve resume block reads on the
/// TCP data plane, so the combination would fault mid-transfer on the
/// default carrier while succeeding on the forced in-stream one. The
/// refusal needs no daemon: it fires before any connection.
#[test]
fn relay_source_with_resume_is_refused_before_any_connection() {
    let execution = PushExecution {
        source: Endpoint::Remote(module_endpoint(9)), // nothing listens on port 9
        resume: true,
        ..push_execution(Path::new("unused"), 9)
    };
    let err = match runtime().block_on(run_remote_push(execution, None)) {
        Ok(_) => panic!("relay + resume must be refused"),
        Err(err) => err,
    };
    assert!(
        format!("{err:#}").contains("--resume is not supported with --relay-via-cli"),
        "got: {err:#}"
    );
}

/// `--resume` through the verb: a changed same-size destination file is
/// patched block-wise (`files_resumed` scored) and lands byte-identical.
#[test]
fn push_verb_resume_patches_changed_partials_blockwise() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).expect("src dir");
    let big: Vec<u8> = (0..u32::try_from(4 * 1024 * 1024).unwrap())
        .map(|i| (i % 241) as u8)
        .collect();
    fs::write(src.join("patch.bin"), &big).expect("write v1");

    runtime().block_on(async {
        run_remote_push(push_execution(&src, ctx.daemon_port), None)
            .await
            .expect("seed push");
    });

    // Same size, changed content, newer mtime — a resume candidate.
    let mut v2 = big.clone();
    for b in &mut v2[2 * 1024 * 1024..2 * 1024 * 1024 + 4096] {
        *b ^= 0xFF;
    }
    fs::write(src.join("patch.bin"), &v2).expect("write v2");
    let bumped = filetime::FileTime::from_unix_time(
        filetime::FileTime::from_last_modification_time(
            &fs::metadata(src.join("patch.bin")).expect("meta"),
        )
        .unix_seconds()
            + 5,
        0,
    );
    filetime::set_file_mtime(src.join("patch.bin"), bumped).expect("bump mtime");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ProgressEvent>();
    let summary = runtime()
        .block_on(async {
            let progress = RemoteTransferProgress::new(tx);
            let execution = PushExecution {
                resume: true,
                ..push_execution(&src, ctx.daemon_port)
            };
            run_remote_push(execution, Some(&progress))
                .await
                .expect("resume push")
        })
        .summary;

    assert_eq!(summary.files_resumed, 1, "changed partial must resume");
    assert_eq!(
        fs::read(ctx.module_dir.join("patch.bin")).expect("read dest"),
        v2,
        "patched destination must match the new source"
    );

    // codex otp-10a F6: a resumed file reports w6-1 progress like any
    // other — counted once on the per-file lane, bytes = the stale
    // blocks actually sent (less than the whole file).
    let mut totals = ProgressTotals::default();
    while let Ok(event) = rx.try_recv() {
        totals.apply(&event);
    }
    assert_eq!(
        totals.manifest_files, 1,
        "the resumed need is the denominator"
    );
    assert_eq!(totals.files, 1, "a resumed file completes exactly once");
    assert!(
        totals.bytes > 0 && totals.bytes < v2.len() as u64,
        "resume progress reports the stale blocks sent, got {} of {}",
        totals.bytes,
        v2.len()
    );

    // Same contract on the in-stream carrier (`send_resume_block_records`
    // reports independently of the data-plane pipeline): change the file
    // again and resume with the forced fallback.
    let mut v3 = v2.clone();
    for b in &mut v3[..4096] {
        *b ^= 0xAA;
    }
    fs::write(src.join("patch.bin"), &v3).expect("write v3");
    let bumped_again = filetime::FileTime::from_unix_time(bumped.unix_seconds() + 5, 0);
    filetime::set_file_mtime(src.join("patch.bin"), bumped_again).expect("bump mtime again");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ProgressEvent>();
    let summary = runtime()
        .block_on(async {
            let progress = RemoteTransferProgress::new(tx);
            let execution = PushExecution {
                resume: true,
                force_grpc: true,
                ..push_execution(&src, ctx.daemon_port)
            };
            run_remote_push(execution, Some(&progress))
                .await
                .expect("in-stream resume push")
        })
        .summary;
    assert_eq!(summary.files_resumed, 1, "in-stream resume still resumes");
    let mut totals = ProgressTotals::default();
    while let Ok(event) = rx.try_recv() {
        totals.apply(&event);
    }
    assert_eq!(totals.files, 1, "in-stream resumed file completes once");
    assert!(
        totals.bytes > 0 && totals.bytes < v3.len() as u64,
        "in-stream resume reports stale blocks, got {} of {}",
        totals.bytes,
        v3.len()
    );
    assert_eq!(
        fs::read(ctx.module_dir.join("patch.bin")).expect("read dest"),
        v3,
        "in-stream patched destination must match the new source"
    );
}
