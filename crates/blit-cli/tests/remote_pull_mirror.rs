use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[test]
fn remote_pull_mirror_purges_extraneous_local_files() {
    let mut ctx = TestContext::new();
    fs::write(ctx.module_dir.join("server.txt"), b"from-server").expect("write server file");

    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");
    fs::write(dest_dir.join("extra.txt"), b"stale").expect("write extra file");

    let remote_src = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(120));

    ctx.daemon.terminate();

    if !output.status.success() {
        panic!(
            "blit-cli failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let server_file = dest_dir.join("server.txt");
    assert!(
        server_file.exists(),
        "expected server file copied to {}",
        server_file.display()
    );
    let bytes = fs::read(&server_file).expect("read server file");
    assert_eq!(bytes, b"from-server");

    assert!(
        !dest_dir.join("extra.txt").exists(),
        "extraneous local file should be purged"
    );
}

#[test]
fn remote_pull_mirror_filtered_subset_preserves_out_of_scope_files() {
    // Closes F4: the daemon's filtered source manifest doesn't
    // include `*.tmp` files, but the client's dest has one. Under
    // FilteredSubset (default), out-of-scope files survive — only
    // in-scope client files absent from the source set are purged.
    let mut ctx = TestContext::new();
    fs::write(ctx.module_dir.join("keep.txt"), b"from-server").unwrap();
    fs::write(ctx.module_dir.join("server.tmp"), b"server tmp").unwrap();

    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).unwrap();
    fs::write(dest_dir.join("dont-touch.tmp"), b"local tmp").unwrap();
    fs::write(dest_dir.join("extra.txt"), b"local extra").unwrap();

    let remote_src = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--exclude")
        .arg("*.tmp")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cmd, Duration::from_secs(120));
    ctx.daemon.terminate();
    if !output.status.success() {
        panic!(
            "mirror failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // In-scope file copied
    assert_eq!(fs::read(dest_dir.join("keep.txt")).unwrap(), b"from-server");
    // In-scope local file absent on filtered source: deleted
    assert!(
        !dest_dir.join("extra.txt").exists(),
        "in-scope extraneous file should be purged"
    );
    // Out-of-scope local file: preserved (the source filter excluded
    // *.tmp, so blit pretends to have no opinion about local *.tmp files)
    assert!(
        dest_dir.join("dont-touch.tmp").exists(),
        "out-of-scope local file must survive FilteredSubset mirror"
    );
    // Out-of-scope server file: never transferred
    assert!(!dest_dir.join("server.tmp").exists());
}

#[test]
fn remote_pull_mirror_delete_scope_all_purges_out_of_scope() {
    // `--delete-scope all` switches to MirrorMode::All — every dest
    // file absent from the (filtered) source set is deleted, even
    // those outside the filter scope. Sharp tool, opt-in.
    let mut ctx = TestContext::new();
    fs::write(ctx.module_dir.join("keep.txt"), b"from-server").unwrap();

    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).unwrap();
    fs::write(dest_dir.join("dont-touch.tmp"), b"local tmp").unwrap();
    fs::write(dest_dir.join("extra.txt"), b"local extra").unwrap();

    let remote_src = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--exclude")
        .arg("*.tmp")
        .arg("--delete-scope")
        .arg("all")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cmd, Duration::from_secs(120));
    ctx.daemon.terminate();
    if !output.status.success() {
        panic!(
            "mirror failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert_eq!(fs::read(dest_dir.join("keep.txt")).unwrap(), b"from-server");
    // Both extra and out-of-scope tmp deleted under MirrorMode::All
    assert!(!dest_dir.join("extra.txt").exists());
    assert!(!dest_dir.join("dont-touch.tmp").exists());
}
