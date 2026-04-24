//! Smoke tests for `blit diagnostics dump`. The command is intentionally
//! stable-surfaced (bug reporters paste its JSON output), so we pin the
//! outer shape here — not every field — so a future refactor can add
//! fields without breaking the test but cannot silently drop one of the
//! load-bearing ones.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

fn cli_bin() -> PathBuf {
    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir
        .parent()
        .expect("deps parent directory")
        .to_path_buf();
    let name = if cfg!(windows) {
        "blit-cli.exe"
    } else {
        "blit-cli"
    };
    bin_dir.join(name)
}

fn run_dump(args: &[&str]) -> std::process::Output {
    let bin = cli_bin();
    let mut cmd = Command::new(&bin);
    cmd.arg("diagnostics").arg("dump");
    cmd.args(args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn");
    match child
        .wait_timeout(Duration::from_secs(10))
        .expect("wait_timeout")
    {
        Some(_) => child.wait_with_output().expect("wait_with_output"),
        None => panic!("diagnostics dump timed out"),
    }
}

#[test]
fn dump_local_to_local_json_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    // Use `src/` so rsync treats it as contents-merge and the resolved
    // destination is unchanged — simpler to assert on.
    let src_arg = format!("{}/", src.display());
    let dst_arg = format!("{}/", dst.display());
    let out = run_dump(&[&src_arg, &dst_arg, "--json"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout:\n{stdout}"));

    assert!(v["blit_version"].is_string(), "blit_version present");
    assert_eq!(v["source"]["kind"], "local");
    assert_eq!(v["destination"]["kind"], "local");
    assert_eq!(v["source"]["exists"], true);
    assert_eq!(v["destination"]["exists"], true);
    assert_eq!(v["rsync_resolution"]["source_is_contents"], true);
    assert_eq!(v["rsync_resolution"]["destination_is_container"], true);
    assert_eq!(v["rsync_resolution"]["resolution_changed"], false);
    assert!(v["source"]["free_bytes"].is_u64());
    assert!(v["destination"]["free_bytes"].is_u64());
}

#[test]
fn dump_detects_rsync_basename_append() {
    // src WITHOUT trailing slash + dst WITH trailing slash → rsync
    // appends src's basename to dst. Diagnostics dump must mark
    // resolution_changed=true so the user can see why the transfer
    // landed where it did.
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let src_arg = src.to_string_lossy().to_string();
    let dst_arg = format!("{}/", dst.display());
    let out = run_dump(&[&src_arg, &dst_arg, "--json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    assert_eq!(v["rsync_resolution"]["resolution_changed"], true);
    let resolved = v["rsync_resolution"]["resolved_destination"]
        .as_str()
        .unwrap();
    assert!(resolved.ends_with("/dst/src"), "expected dst/src, got {resolved}");
}

#[test]
fn dump_remote_destination_captures_endpoint_fields() {
    // No network call — we just parse.
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();

    let out = run_dump(&[&src.to_string_lossy(), "server:/mod/path", "--json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    assert_eq!(v["destination"]["kind"], "remote");
    assert_eq!(v["destination"]["path_kind"], "module");
    assert_eq!(v["destination"]["module"], "mod");
    assert_eq!(v["destination"]["host"], "server");
    // Remote → not local → same_device must be false.
    assert_eq!(v["same_device"], false);
}
