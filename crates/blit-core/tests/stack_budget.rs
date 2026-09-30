//! win-1: a transfer session must fit a small stack.
//!
//! The CLI polls its whole command future on the process's MAIN thread
//! (`#[tokio::main]`), and Windows gives that thread 1 MiB. A debug
//! build's async poll frames hold every nested future inline, so when
//! the SOURCE_SIDE_CONTAINMENT work grew the destination routes, every
//! `blit` run that was a DESTINATION died on Windows with `thread 'main'
//! has overflowed its stack` — while macOS and Linux, whose main threads
//! get 8 MiB, passed. The fix boxes the session bodies where one large
//! future nests another (`transfers::local::run`, the local session's
//! role join, `run_source`/`run_destination`, the pull/push clients, the
//! retry loop), so a caller holds a pointer instead of the session.
//!
//! This guard runs the same shapes on every platform, identically: each
//! session is driven on a thread whose stack is exactly [`BUDGET`], with
//! a current-thread runtime (the future is polled on that thread, as
//! `block_on` polls the CLI's). Measured minimums of this thread's stack
//! (DEVLOG 2026-09-30, "win-1"), macOS aarch64 debug: before the boxing
//! the local mirror needed 368 KiB, the pull-shaped destination 208 KiB
//! in-stream and 192 KiB on the data plane; after it 160 / 160 / 144 KiB
//! (x86_64 debug: 176 / 176 / 160 KiB). So at this layer only the local
//! mirror goes red before the boxing: 1 MiB reproduces nothing here, and
//! the pull regression lived in the CLI's own frames —
//! `crates/blit-cli/tests/main_thread_stack_budget.rs` holds the real
//! binary to its budget and is red before the fix on every shape,
//! pull included. This file keeps the sessions themselves small on every
//! platform.
//!
//! The pull shape's SOURCE runs on its own thread and runtime: in a real
//! pull it is the daemon, and only the DESTINATION is on the caller's
//! stack.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use blit_core::generated::{ComparisonMode, MirrorMode, SessionOpen, TransferRole};
use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
use blit_core::transfer_plan::PlanOptions;
use blit_core::transfer_session::transport::in_process_pair;
use blit_core::transfer_session::{
    run_destination, run_source, DestinationSessionConfig, DestinationTarget, HelloConfig,
    LocalMirrorOptions, SessionEndpoint, SourceSessionConfig,
};

/// The stack each session gets: above every post-fix minimum with at
/// least a 1.45x margin on both architectures measured, below the local
/// mirror's pre-fix 368 KiB.
const BUDGET: usize = 256 * 1024;

/// Run the future `make` builds to completion on a fresh thread whose
/// stack is exactly `stack` bytes, polled by a current-thread runtime on
/// that thread. An overflow aborts the test process, which fails the
/// test — there is no way to catch it, and none is wanted.
fn on_stack_of<F, Fut>(stack: usize, make: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()>,
{
    std::thread::Builder::new()
        .name("stack-budget".into())
        .stack_size(stack)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("current-thread runtime");
            runtime.block_on(make());
        })
        .expect("spawn the budgeted thread")
        .join()
        .expect("the session must complete within the stack budget");
}

/// A tree with small files in subdirectories and one file large enough
/// to ride its own record, so both the shard and the single-file paths
/// run.
fn source_tree(root: &Path) {
    for i in 0..24u8 {
        let path = root.join(format!("d{}/f{i}.txt", i % 3));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, vec![i; 1000 + usize::from(i) * 100]).unwrap();
    }
    std::fs::write(root.join("big.bin"), vec![7u8; 3_000_000]).unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    source_tree(&src);
    // Something for the mirror to delete, and a DIRECTORY where a source
    // file must land, so the per-file failure path runs too.
    std::fs::create_dir_all(dst.join("extraneous")).unwrap();
    std::fs::write(dst.join("extraneous/stale.txt"), b"stale").unwrap();
    std::fs::create_dir_all(dst.join("d0/f0.txt")).unwrap();
    (tmp, src, dst)
}

#[test]
fn a_local_mirror_session_fits_the_stack_budget() {
    let (_tmp, src, dst) = fixture();
    let (s, d) = (src.clone(), dst.clone());
    on_stack_of(BUDGET, move || async move {
        let options = LocalMirrorOptions {
            mirror: true,
            perf_history: false,
            ..Default::default()
        };
        let summary = blit_core::transfers::local::run(&s, &d, options)
            .await
            .expect("the local mirror completes");
        assert_eq!(summary.files_failed, 1, "{:?}", summary.failures);
        assert!(summary.copied_files > 0);
    });
    assert!(!dst.join("extraneous").exists(), "the mirror deleted");
    assert_eq!(
        std::fs::read(dst.join("big.bin")).unwrap(),
        std::fs::read(src.join("big.bin")).unwrap()
    );
}

/// A DESTINATION-initiated session (the pull shape) with the SOURCE
/// served from another thread, on both byte carriers.
fn pull_shaped(in_stream: bool) {
    let (_tmp, src, dst) = fixture();
    let (source_transport, dest_transport) = in_process_pair();
    let source_root = src.clone();
    let source = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("source runtime");
        runtime.block_on(async move {
            let source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(source_root));
            let cfg = SourceSessionConfig {
                instruments: Default::default(),
                hello: HelloConfig::default(),
                endpoint: SessionEndpoint::Responder,
                plan_options: PlanOptions::default(),
                data_plane_host: None,
            };
            run_source(cfg, source_transport, source)
                .await
                .expect("the source completes")
        })
    });
    let d = dst.clone();
    on_stack_of(BUDGET, move || async move {
        let open = SessionOpen {
            initiator_role: TransferRole::Destination as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: in_stream,
            mirror_enabled: true,
            mirror_kind: MirrorMode::All as i32,
            ..Default::default()
        };
        let cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::initiator(open),
            data_plane_host: (!in_stream).then(|| "127.0.0.1".to_string()),
            receiver_capacity: None,
            instruments: Default::default(),
            local_apply: None,
        };
        let outcome = run_destination(cfg, dest_transport, DestinationTarget::Fixed(d))
            .await
            .expect("the destination completes");
        assert_eq!(outcome.summary.files_failed, 1);
        assert!(outcome.summary.files_transferred > 0);
    });
    let summary = source.join().expect("source thread");
    assert_eq!(summary.files_failed, 1);
    assert!(!dst.join("extraneous").exists(), "the mirror deleted");
}

#[test]
fn a_pull_shaped_destination_session_fits_the_stack_budget_in_stream() {
    pull_shaped(true);
}

#[test]
fn a_pull_shaped_destination_session_fits_the_stack_budget_on_the_data_plane() {
    pull_shaped(false);
}
