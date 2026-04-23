mod endpoints;
mod local;
mod remote;

pub use endpoints::{format_remote_endpoint, parse_transfer_endpoint, Endpoint};

use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{bail, Context, Result};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};

use crate::rm::delete_remote_path;
use blit_core::remote::RemotePath;
use endpoints::{
    ensure_remote_destination_supported, ensure_remote_source_supported,
    ensure_remote_transfer_supported,
};
use local::run_local_transfer;
use remote::{run_remote_pull_transfer, run_remote_push_transfer};

/// Returns true if the raw CLI source string specifies "copy contents" mode,
/// matching rsync's DOTDIR_NAME classification from `flist.c:send_file_list`.
///
/// Rsync treats these source forms as "copy the contents of this directory":
///   - ends with `/`     (e.g. `src/`)
///   - ends with `/.`    (e.g. `src/.`)
///   - is exactly `.`
///
/// On Windows, `\` and `\.` are also recognized as trailing separators.
/// On Unix, `\` is a literal filename character and is NOT recognized.
fn source_is_contents(raw_source: &str) -> bool {
    let s = raw_source.trim_end_matches([' ', '\t']);
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    let last = bytes[bytes.len() - 1];

    // Trailing directory separator
    if last == b'/' {
        return true;
    }
    #[cfg(windows)]
    if last == b'\\' {
        return true;
    }

    // Trailing "/." or just "." — rsync flist.c:2296
    if last == b'.' {
        if bytes.len() == 1 {
            return true; // source is exactly "."
        }
        let second_last = bytes[bytes.len() - 2];
        if second_last == b'/' {
            return true;
        }
        #[cfg(windows)]
        if second_last == b'\\' {
            return true;
        }
    }

    false
}

/// Extract the source's final path component (basename) as an OsString,
/// regardless of whether the source is local or remote. Returns None for
/// empty, root, or "." basenames (which would be meaningless to append).
fn source_basename(src: &Endpoint) -> Option<OsString> {
    let basename = match src {
        Endpoint::Local(p) => p.file_name().map(|s| s.to_os_string()),
        Endpoint::Remote(r) => match &r.path {
            RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
                rel_path.file_name().map(|s| s.to_os_string())
            }
            RemotePath::Discovery => None,
        },
    };
    match basename {
        Some(b) if !b.is_empty() && b != "." => Some(b),
        _ => None,
    }
}

/// Returns true if the destination should be treated as a container
/// (i.e. the source should be placed INSIDE it, not as it).
///
/// Matches rsync's `main.c:get_local_name`: a dest is a container if it
/// has a trailing slash, or if it already exists as a directory.
fn dest_is_container(raw_dest: &str, dst: &Endpoint) -> bool {
    let s = raw_dest.trim_end_matches([' ', '\t']);
    // Trailing "/" or "/." counts as container ("put into here").
    if s.ends_with('/') || s.ends_with("/.") {
        return true;
    }
    #[cfg(windows)]
    if s.ends_with('\\') || s.ends_with("\\.") {
        return true;
    }
    // Existing directory (local only — remote requires an RPC we don't want here).
    if let Endpoint::Local(p) = dst {
        if p.is_dir() {
            return true;
        }
    }
    false
}

/// Apply rsync-style destination semantics to compute the final target path.
///
/// Matches the union of rsync `flist.c:send_file_list` (source slash) and
/// `main.c:get_local_name` (dest container detection):
///
///   - If source is "contents" form (`src/`, `src/.`, `.`) → dest used as-is.
///   - Else if dest is a container (trailing slash, or existing local dir) →
///     append source's basename to dest.
///   - Else → dest used as-is (the user named an exact target path / rename).
///
/// Examples:
///   blit copy /a/src  /b/dst/    ->  /b/dst/src/...     (nest under dst)
///   blit copy /a/src/ /b/dst/    ->  /b/dst/<contents>  (merge)
///   blit copy /a/src  /b/newdst  ->  /b/newdst/...      (dst becomes src copy)
///   blit copy /a/f.txt /b/dst/   ->  /b/dst/f.txt       (into dir)
///   blit copy /a/f.txt /b/renamed.txt -> /b/renamed.txt (rename)
pub(crate) fn resolve_destination(
    raw_source: &str,
    raw_dest: &str,
    src: &Endpoint,
    dst: Endpoint,
) -> Endpoint {
    if source_is_contents(raw_source) {
        return dst;
    }
    if !dest_is_container(raw_dest, &dst) {
        return dst;
    }
    let Some(basename) = source_basename(src) else {
        return dst;
    };
    match dst {
        Endpoint::Local(p) => Endpoint::Local(p.join(&basename)),
        Endpoint::Remote(mut r) => {
            r.path = match r.path {
                RemotePath::Module { module, rel_path } => RemotePath::Module {
                    module,
                    rel_path: rel_path.join(&basename),
                },
                RemotePath::Root { rel_path } => RemotePath::Root {
                    rel_path: rel_path.join(&basename),
                },
                other => other,
            };
            Endpoint::Remote(r)
        }
    }
}

/// Prompt for confirmation of a destructive operation. Returns true if the user confirms.
/// Always returns true if `skip_prompt` is true.
fn confirm_destructive_operation(message: &str, skip_prompt: bool) -> Result<bool> {
    if skip_prompt {
        return Ok(true);
    }

    print!("{} [y/N]: ", message);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let decision = input.trim().to_ascii_lowercase();
    Ok(decision == "y" || decision == "yes")
}

#[derive(Copy, Clone)]
pub enum TransferKind {
    Copy,
    Mirror,
}

pub async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: TransferKind) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let raw_dst = parse_transfer_endpoint(&args.destination)?;
    let dst_endpoint = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);

    let operation = match mode {
        TransferKind::Copy => "copy",
        TransferKind::Mirror => "mirror",
    };
    let transfer_scope = match (&src_endpoint, &dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            format!("{} -> {}", src_path.display(), dst_path.display())
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            format!(
                "{} -> {}",
                src_path.display(),
                format_remote_endpoint(remote)
            )
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            format!(
                "{} -> {}",
                format_remote_endpoint(remote),
                dst_path.display()
            )
        }
        (Endpoint::Remote(a), Endpoint::Remote(b)) => {
            format!(
                "{} -> {}",
                format_remote_endpoint(a),
                format_remote_endpoint(b)
            )
        }
    };

    // For mirror operations, prompt unless --yes or --dry-run
    if matches!(mode, TransferKind::Mirror) && !args.dry_run {
        let dst_display = match &dst_endpoint {
            Endpoint::Local(p) => p.display().to_string(),
            Endpoint::Remote(r) => format_remote_endpoint(r),
        };
        let prompt = format!(
            "Mirror will delete extraneous files at destination '{}'. Continue?",
            dst_display
        );
        if !confirm_destructive_operation(&prompt, args.yes)? {
            println!("Aborted.");
            return Ok(());
        }
    }

    if !args.json {
        println!(
            "blit v{}: starting {} {}",
            env!("CARGO_PKG_VERSION"),
            operation,
            transfer_scope
        );
    }

    match (src_endpoint, dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            run_local_transfer(
                ctx,
                args,
                &src_path,
                &dst_path,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            ensure_remote_transfer_supported(args)?;
            ensure_remote_destination_supported(&remote)?;
            run_remote_push_transfer(
                args,
                Endpoint::Local(src_path),
                remote,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            ensure_remote_transfer_supported(args)?;
            ensure_remote_source_supported(&remote)?;
            run_remote_pull_transfer(
                args,
                remote,
                &dst_path,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Remote(src), Endpoint::Remote(dst)) => {
            ensure_remote_transfer_supported(args)?;
            ensure_remote_source_supported(&src)?;
            ensure_remote_destination_supported(&dst)?;
            run_remote_push_transfer(
                args,
                Endpoint::Remote(src),
                dst,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
    }
}

pub async fn run_move(ctx: &AppContext, args: &TransferArgs) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let raw_dst = parse_transfer_endpoint(&args.destination)?;
    let dst_endpoint = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);

    if args.dry_run {
        bail!("move does not support --dry-run");
    }

    // Prompt for confirmation before move (which deletes source)
    let src_display = match &src_endpoint {
        Endpoint::Local(p) => p.display().to_string(),
        Endpoint::Remote(r) => format_remote_endpoint(r),
    };
    let dst_display = match &dst_endpoint {
        Endpoint::Local(p) => p.display().to_string(),
        Endpoint::Remote(r) => format_remote_endpoint(r),
    };
    let prompt = format!(
        "Move will transfer '{}' to '{}' and delete the source. Continue?",
        src_display, dst_display
    );
    if !confirm_destructive_operation(&prompt, args.yes)? {
        println!("Aborted.");
        return Ok(());
    }

    if !args.json {
        println!(
            "blit v{}: starting move {} -> {}",
            env!("CARGO_PKG_VERSION"),
            src_display,
            dst_display
        );
    }

    match (src_endpoint, dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            run_local_transfer(ctx, args, &src_path, &dst_path, true).await?;

            if src_path.is_dir() {
                fs::remove_dir_all(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            } else if src_path.is_file() {
                fs::remove_file(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            }
            Ok(())
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            ensure_remote_transfer_supported(args)?;
            ensure_remote_source_supported(&remote)?;
            run_remote_pull_transfer(args, remote.clone(), &dst_path, false).await?;

            // Delete remote source
            let rel_path = match &remote.path {
                RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
                    rel_path.to_string_lossy().into_owned()
                }
                _ => bail!("unsupported remote source for move"),
            };
            delete_remote_path(&remote, &rel_path).await?;
            Ok(())
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            ensure_remote_transfer_supported(args)?;
            ensure_remote_destination_supported(&remote)?;
            run_remote_push_transfer(
                args,
                Endpoint::Local(src_path.clone()),
                remote.clone(),
                false,
            )
            .await?;

            // Delete local source
            if src_path.is_dir() {
                fs::remove_dir_all(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            } else if src_path.is_file() {
                fs::remove_file(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            }
            Ok(())
        }
        (Endpoint::Remote(src), Endpoint::Remote(dst)) => {
            ensure_remote_transfer_supported(args)?;
            ensure_remote_source_supported(&src)?;
            ensure_remote_destination_supported(&dst)?;
            run_remote_push_transfer(args, Endpoint::Remote(src.clone()), dst, false).await?;

            // Delete remote source
            let rel_path = match &src.path {
                RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
                    rel_path.to_string_lossy().into_owned()
                }
                _ => bail!("unsupported remote source for move"),
            };
            delete_remote_path(&src, &rel_path).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    #[test]
    fn copy_local_transfers_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;
        let ctx = AppContext {
            perf_history_enabled: false,
        };

        let args = TransferArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: false,
            checksum: false,
            size_only: false,
            ignore_times: false,
            ignore_existing: false,
            force: false,
            retries: 1,
            verbose: false,
            progress: false,
            yes: true, // Skip prompts in tests
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
            resume: false,
            null: false,
            json: false,
        };

        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
        let copied = std::fs::read(dest.join("hello.txt"))?;
        assert_eq!(copied, b"hello");
        Ok(())
    }

    #[test]
    fn copy_local_dry_run_creates_no_files() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;
        let ctx = AppContext {
            perf_history_enabled: false,
        };

        let args = TransferArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: true,
            checksum: false,
            size_only: false,
            ignore_times: false,
            ignore_existing: false,
            force: false,
            retries: 1,
            verbose: false,
            progress: false,
            yes: true, // Skip prompts in tests
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
            resume: false,
            null: false,
            json: false,
        };

        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
        assert!(!dest.join("hello.txt").exists());
        Ok(())
    }

    // rsync-compat: "copy contents" detection matches flist.c:send_file_list
    #[test]
    fn source_is_contents_trailing_slash() {
        assert!(source_is_contents("src/"));
        assert!(source_is_contents("/a/b/src/"));
        assert!(source_is_contents("src///"));
    }

    #[test]
    fn source_is_contents_trailing_dot_slash() {
        assert!(source_is_contents("src/."));
        assert!(source_is_contents("/a/b/src/."));
    }

    #[test]
    fn source_is_contents_just_dot() {
        assert!(source_is_contents("."));
    }

    #[test]
    fn source_is_contents_no_trailing() {
        assert!(!source_is_contents("src"));
        assert!(!source_is_contents("/a/b/src"));
        assert!(!source_is_contents(""));
        assert!(!source_is_contents("src.txt"));
        // second-to-last char is not a slash, so trailing '.' is part of filename
        assert!(!source_is_contents("foo.bar"));
    }

    #[test]
    fn source_is_contents_trims_whitespace() {
        assert!(source_is_contents("src/  "));
        assert!(source_is_contents("src/.\t"));
    }

    #[cfg(windows)]
    #[test]
    fn source_is_contents_windows_backslash() {
        assert!(source_is_contents("src\\"));
        assert!(source_is_contents("src\\."));
        assert!(source_is_contents("C:\\path\\"));
    }

    // resolve_destination: core rsync-style semantics
    //
    // Rules (applied identically to local→local, local→remote, remote→local,
    // remote→remote, since resolution happens at the top-level dispatch):
    //   source contents form (SRC/, SRC/., .) → dest as-is (merge)
    //   dest trailing slash OR existing local dir → append source basename (nest)
    //   else → dest as-is (rename / exact target)

    #[test]
    fn resolve_destination_existing_dir_appends_basename() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/GameDir", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, tmp.path().join("GameDir")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_trailing_slash_on_dest_appends_basename() {
        // Dest doesn't exist, but has trailing slash → still a container.
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(PathBuf::from("/b/new_dst"));
        let resolved = resolve_destination("/a/GameDir", "/b/new_dst/", &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, PathBuf::from("/b/new_dst/GameDir")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_nonexistent_no_slash_uses_dest_as_target() {
        // rsync: `rsync src newdst` where newdst doesn't exist → newdst becomes src copy
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(PathBuf::from("/definitely/does/not/exist/newdst"));
        let resolved = resolve_destination(
            "/a/GameDir",
            "/definitely/does/not/exist/newdst",
            &src,
            dst,
        );
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, PathBuf::from("/definitely/does/not/exist/newdst")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_source_trailing_slash_keeps_dest() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_before = tmp.path().to_path_buf();
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/GameDir/", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, dst_before),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_source_dot_slash_keeps_dest() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_before = tmp.path().to_path_buf();
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/GameDir/.", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, dst_before),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_file_to_existing_dir_appends_filename() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/file.txt"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/file.txt", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, tmp.path().join("file.txt")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_file_to_exact_path_is_rename() {
        // Dest doesn't exist, no trailing slash → exact rename.
        let src = Endpoint::Local(PathBuf::from("/a/file.txt"));
        let dst = Endpoint::Local(PathBuf::from("/b/renamed.txt"));
        let resolved =
            resolve_destination("/a/file.txt", "/b/renamed.txt", &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, PathBuf::from("/b/renamed.txt")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_remote_dest_with_trailing_slash_appends() {
        // Remote dest with trailing slash is a container (same rule as local).
        use blit_core::remote::{RemoteEndpoint, RemotePath};
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Remote(RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".into(),
                rel_path: PathBuf::from("common"),
            },
        });
        let resolved =
            resolve_destination("/a/GameDir", "h:/m/common/", &src, dst);
        match resolved {
            Endpoint::Remote(r) => match r.path {
                RemotePath::Module { rel_path, .. } => {
                    assert_eq!(rel_path, PathBuf::from("common/GameDir"))
                }
                _ => panic!("expected module path"),
            },
            _ => panic!("expected remote endpoint"),
        }
    }

    #[test]
    fn resolve_destination_remote_dest_no_slash_preserves_target() {
        // No trailing slash + remote dest (can't stat) → treat as exact target.
        use blit_core::remote::{RemoteEndpoint, RemotePath};
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Remote(RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".into(),
                rel_path: PathBuf::from("common/target"),
            },
        });
        let resolved =
            resolve_destination("/a/GameDir", "h:/m/common/target", &src, dst);
        match resolved {
            Endpoint::Remote(r) => match r.path {
                RemotePath::Module { rel_path, .. } => {
                    // No trailing slash on dest, can't stat remote → preserve target
                    assert_eq!(rel_path, PathBuf::from("common/target"))
                }
                _ => panic!("expected module path"),
            },
            _ => panic!("expected remote endpoint"),
        }
    }

    #[test]
    fn resolve_destination_remote_source_appends_basename_on_container() {
        use blit_core::remote::{RemoteEndpoint, RemotePath};
        let tmp = tempdir().unwrap();
        let src = Endpoint::Remote(RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".into(),
                rel_path: PathBuf::from("Games/DOOM"),
            },
        });
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved =
            resolve_destination("h:/m/Games/DOOM", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, tmp.path().join("DOOM")),
            _ => panic!("expected local endpoint"),
        }
    }
}
