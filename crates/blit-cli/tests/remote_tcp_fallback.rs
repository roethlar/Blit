use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

/// Daemon forced into gRPC data fallback (`--force-grpc-data`).
fn forced_grpc_ctx() -> TestContext {
    TestContext::builder()
        .extra_daemon_args(["--force-grpc-data"])
        .build()
}

#[test]
fn remote_push_falls_back_to_grpc_when_forced() {
    let mut ctx = forced_grpc_ctx();

    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("file.txt"), b"fallback-test").expect("write file");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    // Trailing slash on source: merge contents into module root.
    let src_arg = format!("{}/", src_dir.display());
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--force-grpc")
        .arg(&src_arg)
        .arg(&dest_remote);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(120));

    ctx.daemon.terminate();

    if !output.status.success() {
        panic!(
            "blit failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("gRPC data fallback") || stdout.contains("[gRPC fallback]"),
        "expected fallback message, got:\n{}",
        stdout
    );

    let dest_file = ctx.module_dir.join("file.txt");
    assert!(dest_file.exists(), "remote file missing");
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"fallback-test");
}

// ---------------------------------------------------------------
// w4-2 regression net: the daemon used to queue every needs-upload
// manifest entry into a 262,144-slot channel that nothing read in
// gRPC-fallback mode, so manifest entry #262,145 wedged daemon and
// client forever with no timeout in scope
// (async-push-upload-channel-fallback-wedge). The channel is deleted;
// these tests pin that many-file forced-gRPC pushes complete.
// ---------------------------------------------------------------

/// Spawn a daemon, mirror `file_count` generated files with
/// --force-grpc, assert success, and return how many landed.
fn forced_grpc_mirror_file_count(file_count: usize, timeout: Duration) -> usize {
    let mut ctx = forced_grpc_ctx();

    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    for idx in 0..file_count {
        // Shard into subdirs so no single directory holds 262k entries.
        let sub = src_dir.join(format!("d{}", idx / 1024));
        if idx % 1024 == 0 {
            fs::create_dir_all(&sub).expect("shard dir");
        }
        fs::write(sub.join(format!("f{idx}.txt")), b"x").expect("write src file");
    }

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let src_arg = format!("{}/", src_dir.display());
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--force-grpc")
        .arg(&src_arg)
        .arg(&dest_remote);
    let output = run_with_timeout(cli_cmd, timeout);
    ctx.daemon.terminate();

    assert!(
        output.status.success(),
        "forced-gRPC mirror of {file_count} files failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    walkdir_count_files(&ctx.module_dir)
}

fn walkdir_count_files(root: &std::path::Path) -> usize {
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read module dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                count += 1;
            }
        }
    }
    count
}

/// design-4 regression: 2000 files is well past the old failure cliff
/// (≥128 = FILE_LIST_EARLY_FLUSH_ENTRIES failed deterministically,
/// ~100 was timing-flaky). Pre-fix, both sides raced the daemon's
/// manifest loop: the daemon announced Negotiation(tcp_fallback) on
/// the mid-manifest early flush, and a force_grpc client started
/// streaming FileData on the first need-list batch without any
/// negotiation at all — either way the daemon's manifest loop
/// rejected the premature FileData and tore down the push. Fixed by
/// deferring the daemon's announcement to execute_grpc_fallback and
/// gating the client's fallback sends on fallback_negotiated.
/// 500 files / 300 s: tuned after the first Windows CI run of the
/// 2,000-file version timed out at 120 s (run 27429395227). Windows
/// runners are very slow on many-small-file I/O (Defender scans each
/// create; manifest enumeration alone took ~500 ms there) and the
/// daemon stats every entry (w4-4's queued hot spot). 500 is still
/// ~4x past the 128-entry cliff design-4 lived at; if THIS times out
/// on Windows, that's a real platform stall — file it, don't retune.
#[test]
fn forced_grpc_push_many_files_completes() {
    let landed = forced_grpc_mirror_file_count(500, Duration::from_secs(300));
    assert_eq!(landed, 500, "every file must land via the gRPC fallback");
}

/// The exact pre-w4-2 wedge: >262,144 needs-upload entries in
/// gRPC-fallback mode. With design-4 fixed this is expected to pass;
/// it stays ignored only for runtime (~270k files, multi-minute).
/// Joint acceptance test for design-4 + w4-2. Run manually:
///   cargo test -p blit-cli --test remote_tcp_fallback -- --ignored
#[test]
#[ignore = "~270k files / multi-minute runtime; run manually"]
fn forced_grpc_push_overflows_old_upload_channel_capacity() {
    let landed = forced_grpc_mirror_file_count(270_000, Duration::from_secs(1800));
    assert_eq!(
        landed, 270_000,
        "pre-w4-2 this hung forever at entry 262,145"
    );
}
