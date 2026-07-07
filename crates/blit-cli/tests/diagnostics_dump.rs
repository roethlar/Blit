//! Smoke tests for `blit diagnostics dump`. The command is intentionally
//! stable-surfaced (bug reporters paste its JSON output), so we pin the
//! outer shape here — not every field — so a future refactor can add
//! fields without breaking the test but cannot silently drop one of the
//! load-bearing ones.

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout};

fn run_dump(args: &[&str]) -> std::process::Output {
    let bin = cli_bin();
    let mut cmd = Command::new(&bin);
    cmd.arg("diagnostics").arg("dump");
    cmd.args(args);
    run_with_timeout(cmd, Duration::from_secs(10))
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
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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
    // Normalize separators so the suffix check works on Windows where
    // the path prefix uses backslashes.
    let normalized = resolved.replace('\\', "/");
    assert!(
        normalized.ends_with("/dst/src"),
        "expected dst/src, got {resolved}"
    );
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
