use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

/// Push with nested directory structure preserves hierarchy.
#[cfg(unix)]
#[test]
fn test_push_nested_directories() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(src_dir.join("a/b/c")).expect("nested dirs");
    fs::write(src_dir.join("root.txt"), b"root").expect("write root");
    fs::write(src_dir.join("a/level1.txt"), b"level1").expect("write level1");
    fs::write(src_dir.join("a/b/level2.txt"), b"level2").expect("write level2");
    fs::write(src_dir.join("a/b/c/level3.txt"), b"level3").expect("write level3");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_dir)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "push nested dirs failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(fs::read(ctx.module_dir.join("root.txt")).unwrap(), b"root");
    assert_eq!(
        fs::read(ctx.module_dir.join("a/level1.txt")).unwrap(),
        b"level1"
    );
    assert_eq!(
        fs::read(ctx.module_dir.join("a/b/level2.txt")).unwrap(),
        b"level2"
    );
    assert_eq!(
        fs::read(ctx.module_dir.join("a/b/c/level3.txt")).unwrap(),
        b"level3"
    );
}

/// Copy does NOT delete extraneous destination files (unlike mirror).
#[cfg(unix)]
#[test]
fn test_copy_does_not_delete_extraneous() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("new.txt"), b"new-content").expect("write new");

    // Pre-populate destination with an extra file
    fs::write(ctx.module_dir.join("existing.txt"), b"keep-me").expect("write existing");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_dir)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
    assert!(output.status.success(), "copy failed");

    // New file should be transferred
    assert_eq!(
        fs::read(ctx.module_dir.join("new.txt")).unwrap(),
        b"new-content"
    );
    // Existing file should NOT be deleted (copy != mirror)
    assert!(
        ctx.module_dir.join("existing.txt").exists(),
        "copy should not delete extraneous destination files"
    );
    assert_eq!(
        fs::read(ctx.module_dir.join("existing.txt")).unwrap(),
        b"keep-me"
    );
}

/// Pull with nested directories preserves structure locally.
#[cfg(unix)]
#[test]
fn test_pull_nested_directories() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");

    // Set up nested structure on daemon
    fs::create_dir_all(ctx.module_dir.join("x/y")).expect("nested dirs");
    fs::write(ctx.module_dir.join("top.bin"), b"top-data").expect("write top");
    fs::write(ctx.module_dir.join("x/mid.bin"), b"mid-data").expect("write mid");
    fs::write(ctx.module_dir.join("x/y/deep.bin"), b"deep-data").expect("write deep");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "pull nested dirs failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(fs::read(dest_dir.join("top.bin")).unwrap(), b"top-data");
    assert_eq!(fs::read(dest_dir.join("x/mid.bin")).unwrap(), b"mid-data");
    assert_eq!(
        fs::read(dest_dir.join("x/y/deep.bin")).unwrap(),
        b"deep-data"
    );
}

/// Push many small files exercises tar shard batching.
#[cfg(unix)]
#[test]
fn test_push_many_small_files() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");

    let file_count = 50;
    for i in 0..file_count {
        fs::write(
            src_dir.join(format!("f_{:04}.dat", i)),
            format!("content-{}", i),
        )
        .expect("write small file");
    }

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_dir)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
    assert!(output.status.success(), "push many small files failed");

    // Verify all files arrived with correct content
    for i in 0..file_count {
        let name = format!("f_{:04}.dat", i);
        let content = fs::read_to_string(ctx.module_dir.join(&name))
            .unwrap_or_else(|_| panic!("missing {}", name));
        assert_eq!(
            content,
            format!("content-{}", i),
            "wrong content in {}",
            name
        );
    }
}

/// Push empty source directory should succeed without errors.
#[cfg(unix)]
#[test]
fn test_push_empty_directory() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_dir)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "push empty dir failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Pull empty module should succeed without errors.
#[cfg(unix)]
#[test]
fn test_pull_empty_module() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "pull empty module failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
