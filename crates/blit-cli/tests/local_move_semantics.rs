//! R46-F1 regression: `blit move` between two local paths must NOT
//! purge unrelated entries from the destination.
//!
//! Pre-fix `crates/blit-cli/src/transfers/mod.rs:458` passed
//! `mirror=true` into the local-to-local move arm, so any file/dir
//! that existed at the destination but not at the source got
//! silently deleted along with the move. The other three move arms
//! (remote→local, local→remote, remote→remote) all correctly used
//! `mirror=false` — this was a bare-local outlier and a real
//! data-loss bug.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use tempfile::tempdir;
use wait_timeout::ChildExt;

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn command");
    match child.wait_timeout(timeout).expect("wait for process") {
        Some(_) => child.wait_with_output().expect("collect output"),
        None => {
            let _ = child.kill();
            let output = child.wait_with_output().expect("output after kill");
            panic!(
                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                timeout,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

fn cli_bin() -> PathBuf {
    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir
        .parent()
        .expect("deps parent directory")
        .to_path_buf();
    let name = if cfg!(windows) { "blit.exe" } else { "blit" };
    bin_dir.join(name)
}

#[test]
fn local_move_preserves_unrelated_destination_entries() {
    let tmp = tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    // Source has one file the move will transfer.
    fs::write(src.join("moved.txt"), b"moved-payload").unwrap();

    // Destination has a pre-existing unrelated file the user wants
    // kept. Pre-fix the move would delete this because the planner
    // saw it as "extra" relative to the source.
    fs::write(dst.join("keep_me.txt"), b"this must survive").unwrap();
    // Also a sibling subdirectory with a file inside, to prove
    // recursive deletion would have hit it too.
    fs::create_dir_all(dst.join("keep_dir")).unwrap();
    fs::write(dst.join("keep_dir/inner.txt"), b"inner survivor").unwrap();

    let mut cmd = Command::new(cli_bin());
    cmd.arg("move")
        .arg("--yes")
        .arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "move failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Source contents transferred and source removed.
    assert!(
        !src.join("moved.txt").exists(),
        "source file should be gone"
    );
    assert_eq!(
        fs::read(dst.join("moved.txt")).unwrap(),
        b"moved-payload",
        "moved file landed at destination"
    );

    // Pre-existing unrelated destination entries must survive — this
    // is the load-bearing assertion for R46-F1.
    assert!(
        dst.join("keep_me.txt").exists(),
        "unrelated destination file was deleted (R46-F1 mirror-on-move regression)"
    );
    assert_eq!(
        fs::read(dst.join("keep_me.txt")).unwrap(),
        b"this must survive"
    );
    assert!(
        dst.join("keep_dir/inner.txt").exists(),
        "unrelated destination subdirectory was deleted (R46-F1)"
    );
}

/// R47-F4 regression: `blit move SRC DST/` between two local paths
/// must refuse to delete the source if the scan was incomplete.
/// Pre-fix, the R46-F2 gate inside the orchestrator only fired
/// for `mirror=true`. Move passes `false`, so an unreadable
/// source subdirectory would be silently skipped during the copy,
/// then `fs::remove_dir_all(src)` would delete the source —
/// including the contents we couldn't read. Net effect: data
/// loss on files that never made it to dest.
///
/// unix-only because the test relies on `chmod 000` to make the
/// subdirectory unreadable to the scanner.
#[cfg(unix)]
#[test]
fn local_move_refuses_when_source_scan_incomplete() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    // Source has a readable file and an unreadable subdirectory.
    fs::write(src.join("readable.txt"), b"keep").unwrap();
    let blocked = src.join("blocked");
    fs::create_dir_all(&blocked).unwrap();
    fs::write(blocked.join("inner.txt"), b"unscannable").unwrap();

    // Make src/blocked unreadable so the walkdir can't enter it.
    let mut perms = fs::metadata(&blocked).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&blocked, perms).unwrap();
    struct PermGuard(std::path::PathBuf);
    impl Drop for PermGuard {
        fn drop(&mut self) {
            if let Ok(meta) = std::fs::metadata(&self.0) {
                let mut p = meta.permissions();
                p.set_mode(0o755);
                let _ = std::fs::set_permissions(&self.0, p);
            }
        }
    }
    let _guard = PermGuard(blocked.clone());

    let mut cmd = Command::new(cli_bin());
    cmd.arg("move")
        .arg("--yes")
        .arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(30));
    assert!(
        !output.status.success(),
        "move with unreadable source must fail; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("refusing to remove source") || stderr.contains("scan was"),
        "expected R47-F4 scan-incomplete refusal, got stderr: {}",
        stderr
    );

    // The unreadable subdir's contents must still be on disk —
    // either as the unreadable file inside src/blocked (if perms
    // permit verifying), or at minimum the src tree itself must
    // not have been removed.
    assert!(
        src.exists(),
        "src must not have been removed when scan was incomplete"
    );
    assert!(
        src.join("blocked").exists(),
        "src/blocked must still exist when move refused"
    );
}
