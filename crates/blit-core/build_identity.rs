use std::hash::{BuildHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Inputs that can change either peer's executable behavior. Documentation and
/// other repository bookkeeping deliberately do not churn the wire build ID.
const BUILD_INPUTS: &[&str] = &["Cargo.toml", "Cargo.lock", "crates", "proto"];

fn run_git(manifest_dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn run_git_string(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    run_git(manifest_dir, args).map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
}

fn run_scoped_git(manifest_dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let mut command = Command::new("git");
    command.args(args).arg("--");
    for input in BUILD_INPUTS {
        command.arg(format!(":(top){input}"));
    }
    let output = command.current_dir(manifest_dir).output().ok()?;
    output.status.success().then_some(output.stdout)
}

fn workspace_root(manifest_dir: &Path, emit_rerun: bool) -> PathBuf {
    if emit_rerun {
        if let Some(git_dir) = run_git_string(manifest_dir, &["rev-parse", "--absolute-git-dir"]) {
            println!("cargo:rerun-if-changed={git_dir}/HEAD");
            println!("cargo:rerun-if-changed={git_dir}/refs");
            println!("cargo:rerun-if-changed={git_dir}/index");
        }
    }
    let root = run_git_string(manifest_dir, &["rev-parse", "--show-toplevel"])
        .map(PathBuf::from)
        .or_else(|| {
            let candidate = manifest_dir.parent()?.parent()?;
            (candidate.join("Cargo.toml").is_file() && candidate.join("crates").is_dir())
                .then(|| candidate.to_path_buf())
        })
        .unwrap_or_else(|| manifest_dir.to_path_buf());
    if emit_rerun {
        for input in BUILD_INPUTS {
            let path = root.join(input);
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    root
}

fn hash_untracked_contents(root: &Path, porcelain: &[u8], hasher: &mut impl Hasher) {
    for entry in porcelain.split(|byte| *byte == 0) {
        let Some(path_bytes) = entry.strip_prefix(b"?? ") else {
            continue;
        };
        path_bytes.hash(hasher);
        let path = root.join(String::from_utf8_lossy(path_bytes).as_ref());
        match std::fs::read(path) {
            Ok(contents) => contents.hash(hasher),
            Err(error) => {
                // A concurrent disappearance or unreadable build input must
                // not collapse onto the identity of a readable file.
                error.kind().hash(hasher);
            }
        }
    }
}

fn unknown_build_suffix() -> String {
    // Independent compilations must never false-match when their source
    // identity cannot be established. One built binary still matches itself.
    let nonce = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    format!("unknown.{nonce:016x}")
}

/// Git identity for the same-build session handshake.
///
/// Clean build inputs use the commit SHA. Dirty build inputs add a stable hash
/// of their scoped status, tracked diff, and untracked file contents. Every
/// watched workspace input is declared on every run, so separate Cargo target
/// profiles cannot retain different subsets of an older dirty tree.
fn git_build_suffix_inner(manifest_dir: &Path, emit_rerun: bool) -> String {
    let root = workspace_root(manifest_dir, emit_rerun);
    let Some(sha) = run_git_string(manifest_dir, &["rev-parse", "--short=12", "HEAD"])
        .filter(|sha| !sha.is_empty())
    else {
        return unknown_build_suffix();
    };

    let Some(porcelain) = run_scoped_git(
        manifest_dir,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    ) else {
        return unknown_build_suffix();
    };
    if porcelain.is_empty() {
        return sha;
    }

    let Some(diff) = run_scoped_git(manifest_dir, &["diff", "--binary", "--no-ext-diff", "HEAD"])
    else {
        return unknown_build_suffix();
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    porcelain.hash(&mut hasher);
    diff.hash(&mut hasher);
    hash_untracked_contents(&root, &porcelain, &mut hasher);
    format!("{sha}.dirty.{:016x}", hasher.finish())
}

#[cfg(not(test))]
pub(crate) fn git_build_suffix(manifest_dir: &Path) -> String {
    git_build_suffix_inner(manifest_dir, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn docs_do_not_churn_identity_and_untracked_source_content_does() {
        let repo = tempfile::tempdir().expect("temp repo");
        let root = repo.path();
        let manifest_dir = root.join("crates/blit-core");
        fs::create_dir_all(manifest_dir.join("src")).expect("core src");
        fs::create_dir_all(root.join("docs")).expect("docs");
        fs::write(root.join("Cargo.toml"), b"[workspace]\n").expect("workspace manifest");
        fs::write(manifest_dir.join("src/lib.rs"), b"pub fn value() {}\n").expect("source");
        fs::write(root.join("docs/readme.md"), b"initial\n").expect("docs file");
        git(root, &["init", "-q"]);
        git(root, &["add", "."]);
        git(
            root,
            &[
                "-c",
                "user.name=Blit Test",
                "-c",
                "user.email=blit@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        );

        let identity = || git_build_suffix_inner(&manifest_dir, false);
        let clean = identity();
        assert!(!clean.contains(".dirty."));

        fs::write(root.join("docs/readme.md"), b"docs only\n").expect("edit docs");
        assert_eq!(
            identity(),
            clean,
            "non-build documentation must not change the peer identity"
        );

        let untracked = manifest_dir.join("src/new_module.rs");
        fs::write(&untracked, b"pub const VALUE: u8 = 1;\n").expect("untracked source");
        let first = identity();
        assert!(first.contains(".dirty."));

        fs::write(&untracked, b"pub const VALUE: u8 = 2;\n").expect("edit untracked source");
        let second = identity();
        assert_ne!(
            first, second,
            "untracked source bytes must participate even when porcelain is unchanged"
        );
        assert_eq!(second, identity());
    }

    #[test]
    fn missing_git_identity_never_false_matches_independent_builds() {
        let repo = tempfile::tempdir().expect("temp source tree");
        let root = repo.path();
        let manifest_dir = root.join("crates/blit-core");
        fs::create_dir_all(manifest_dir.join("src")).expect("core src");
        fs::write(root.join("Cargo.toml"), b"[workspace]\n").expect("workspace manifest");

        let first = git_build_suffix_inner(&manifest_dir, false);
        let second = git_build_suffix_inner(&manifest_dir, false);
        assert!(first.starts_with("unknown."));
        assert!(second.starts_with("unknown."));
        assert_ne!(first, second);
    }
}
