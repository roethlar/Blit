use protoc_bin_vendored::protoc_bin_path;
use std::hash::{BuildHasher, Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

/// Git identity for the same-build session handshake
/// (D-2026-07-05-2, docs/TRANSFER_SESSION.md §Invariants 2). The
/// handshake is the ONLY compatibility gate — no negotiation exists —
/// so an imprecise identity must never let two different builds
/// exact-match (otp-3 codex F1):
///
/// - clean tree  → `<short sha>` — two ends built from the same
///   commit match, per the contract's definition.
/// - dirty tree  → `<short sha>.dirty.<content hash>` — the nonce is
///   a deterministic hash of the porcelain status + diff, so
///   byte-identical dirty trees still match (and no-op rebuilds
///   don't churn the id) while any content difference refuses.
/// - no git      → `unknown.<per-compilation entropy>` — independent
///   compilations can never false-match; one binary deployed to both
///   ends still matches itself.
///
/// Residual window (accepted, reviewed): a first edit to a
/// previously-clean file OUTSIDE blit-core/proto, with no git
/// operation in between, keeps the last sampled identity until the
/// next script trigger. Closing it means watching every workspace
/// source and recompiling the world on any edit anywhere —
/// deliberately not done; see the otp-3 verdict record.
fn git_build_suffix(manifest_dir: &std::path::Path) -> String {
    let run = |args: &[&str]| -> Option<Vec<u8>> {
        let out = Command::new("git")
            .args(args)
            .current_dir(manifest_dir)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(out.stdout)
    };
    let run_str = |args: &[&str]| -> Option<String> {
        run(args).map(|b| String::from_utf8_lossy(&b).trim().to_string())
    };

    let Some(sha) = run_str(&["rev-parse", "--short=12", "HEAD"]).filter(|s| !s.is_empty()) else {
        // No git identity: entropy from a randomly-keyed hasher (no
        // extra deps) so separate compilations get distinct ids.
        let nonce = std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish();
        return format!("unknown.{nonce:016x}");
    };

    // Re-sample identity when git state moves (HEAD/refs/index) or
    // the wire-owning sources change. src/ + proto/ make blit-core
    // edits re-run this script; index catches add/commit/checkout.
    if let Some(git_dir) = run_str(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/refs");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src").display()
    );

    let porcelain = run(&["status", "--porcelain", "-z"]).unwrap_or_default();
    if porcelain.is_empty() {
        return sha;
    }

    // Watch each currently-dirty path so continued edits to it
    // re-run this script and refresh the content nonce.
    if let Some(root) = run_str(&["rev-parse", "--show-toplevel"]) {
        let root = PathBuf::from(root);
        for entry in porcelain.split(|b| *b == 0) {
            // porcelain -z entries: "XY <path>"; renames add a second
            // NUL-separated path record, which parses the same way.
            if entry.len() > 3 {
                let path = String::from_utf8_lossy(&entry[3..]);
                println!(
                    "cargo:rerun-if-changed={}",
                    root.join(path.as_ref()).display()
                );
            }
        }
    }

    // Deterministic content nonce: same dirty content → same id
    // (std's zero-keyed DefaultHasher; same-rustc is implied by
    // same-build, so cross-version instability is fine — different
    // toolchains SHOULD refuse each other).
    let diff = run(&["diff", "HEAD"]).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    porcelain.hash(&mut hasher);
    diff.hash(&mut hasher);
    format!("{sha}.dirty.{:016x}", hasher.finish())
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
