use protoc_bin_vendored::protoc_bin_path;
use std::path::PathBuf;
use std::process::Command;

/// Best-effort git identity for the same-build session handshake
/// (D-2026-07-05-2, docs/TRANSFER_SESSION.md §Invariants 2). Returns
/// "<short sha>[.dirty]" or "unknown" when git/repo is unavailable
/// (e.g. building from a source tarball).
fn git_build_suffix(manifest_dir: &std::path::Path) -> String {
    let run = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(manifest_dir)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    let Some(sha) = run(&["rev-parse", "--short=12", "HEAD"]).filter(|s| !s.is_empty()) else {
        return "unknown".to_string();
    };

    // Track HEAD so the sha refreshes on commit/branch switch without
    // rebuilding on every unrelated file change. Dirty state is
    // best-effort: it is sampled when the build script runs, so a
    // tree that becomes dirty without touching HEAD can keep a stale
    // clean flag until the next rebuild — the sha component is the
    // load-bearing part of the handshake.
    if let Some(git_dir) = run(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/refs");
    }

    let dirty = run(&["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    if dirty {
        format!("{sha}.dirty")
    } else {
        sha
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc_path);

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proto_dir = manifest_dir.join("..").join("..").join("proto");
    let proto_file = proto_dir.join("blit.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());
    println!(
        "cargo:rustc-env=BLIT_GIT_SHA={}",
        git_build_suffix(&manifest_dir)
    );

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[proto_file.as_path()], &[proto_dir.as_path()])?;
    Ok(())
}
