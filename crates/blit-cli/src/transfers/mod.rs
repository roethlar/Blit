mod endpoints;
mod local;
mod remote;
mod remote_remote_direct;

pub use endpoints::{format_remote_endpoint, parse_transfer_endpoint, Endpoint};

use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{bail, Context, Result};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::rm::delete_remote_path;
use blit_core::fs_enum::{parse_duration, parse_size, FileFilter};
use blit_core::remote::RemotePath;

/// Common shape of the filter inputs across commands. Both `TransferArgs`
/// (copy/mirror/move) and `CheckArgs` (check) populate this. The single
/// `build_filter_from_inputs` helper consumes it so all commands route
/// through identical filter semantics.
pub(crate) struct FilterInputs<'a> {
    pub include: &'a [String],
    pub exclude: &'a [String],
    pub files_from: Option<&'a PathBuf>,
    pub min_size: Option<&'a str>,
    pub max_size: Option<&'a str>,
    pub min_age: Option<&'a str>,
    pub max_age: Option<&'a str>,
}

impl<'a> FilterInputs<'a> {
    pub fn from_transfer(args: &'a TransferArgs) -> Self {
        Self {
            include: &args.include,
            exclude: &args.exclude,
            files_from: args.files_from.as_ref(),
            min_size: args.min_size.as_deref(),
            max_size: args.max_size.as_deref(),
            min_age: args.min_age.as_deref(),
            max_age: args.max_age.as_deref(),
        }
    }
}
use endpoints::{
    ensure_remote_destination_supported, ensure_remote_pull_supported,
    ensure_remote_push_supported, ensure_remote_source_supported,
};
use local::run_local_transfer;
use remote::{run_remote_pull_transfer, run_remote_push_transfer};
use remote_remote_direct::run_remote_to_remote_direct;

/// Render an endpoint for human-facing log lines, collapsing any runs of
/// `/` into a single `/` in the local-path portion. Filesystems already
/// ignore `//+`, but users stare at it — our own banner printed
/// `src//foo` when a script appended `/` to an already-trailing-slash
/// `$SRC`. This is display-only; the actual path handling is unchanged
/// so rsync trailing-slash semantics still apply.
fn display_endpoint(e: &Endpoint) -> String {
    match e {
        Endpoint::Local(p) => collapse_slashes(&p.display().to_string()),
        Endpoint::Remote(r) => format_remote_endpoint(r),
    }
}

fn collapse_slashes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_slash = false;
    for c in s.chars() {
        let is_slash = c == '/';
        if !(is_slash && prev_slash) {
            out.push(c);
        }
        prev_slash = is_slash;
    }
    out
}

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
pub(crate) fn source_is_contents(raw_source: &str) -> bool {
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
pub(crate) fn dest_is_container(raw_dest: &str, dst: &Endpoint) -> bool {
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

/// Build a `FileFilter` from a transfer command's args. Convenience wrapper
/// around `build_filter_from_inputs`.
pub(crate) fn build_filter(args: &TransferArgs) -> Result<FileFilter> {
    build_filter_from_inputs(&FilterInputs::from_transfer(args))
}

/// Build the wire-side `FilterSpec` proto message from CLI args.
/// Used by the remote pull path so the daemon enforces the same
/// filter the CLI would have applied locally. `--files-from` is
/// read here and shipped expanded so the daemon doesn't have to
/// reach back into the client's filesystem.
pub(crate) fn build_filter_spec(args: &TransferArgs) -> Result<blit_core::generated::FilterSpec> {
    use blit_core::generated::FilterSpec;
    let mut spec = FilterSpec {
        include: args.include.clone(),
        exclude: args.exclude.clone(),
        min_size: None,
        max_size: None,
        min_age_secs: None,
        max_age_secs: None,
        files_from: Vec::new(),
    };
    if let Some(s) = args.min_size.as_deref() {
        spec.min_size = Some(parse_size(s).with_context(|| format!("--min-size {s}"))?);
    }
    if let Some(s) = args.max_size.as_deref() {
        spec.max_size = Some(parse_size(s).with_context(|| format!("--max-size {s}"))?);
    }
    if let Some(s) = args.min_age.as_deref() {
        spec.min_age_secs = Some(
            parse_duration(s)
                .with_context(|| format!("--min-age {s}"))?
                .as_secs(),
        );
    }
    if let Some(s) = args.max_age.as_deref() {
        spec.max_age_secs = Some(
            parse_duration(s)
                .with_context(|| format!("--max-age {s}"))?
                .as_secs(),
        );
    }
    if let Some(path) = args.files_from.as_ref() {
        let entries = FileFilter::load_files_from(path)?;
        spec.files_from = entries
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
    }
    Ok(spec)
}

/// Build a `FileFilter` from filter inputs. Single helper used by all
/// commands (copy/mirror/move/check) so filter behavior is identical
/// regardless of which CLI verb invoked it. The orchestrator-side
/// helper — not the leaf code — is what calculates the filter.
pub(crate) fn build_filter_from_inputs(inputs: &FilterInputs<'_>) -> Result<FileFilter> {
    let mut filter = FileFilter::default();
    filter.include_files = inputs.include.to_vec();
    filter.exclude_files = inputs.exclude.to_vec();
    if let Some(s) = inputs.min_size {
        filter.min_size = Some(parse_size(s).with_context(|| format!("--min-size {s}"))?);
    }
    if let Some(s) = inputs.max_size {
        filter.max_size = Some(parse_size(s).with_context(|| format!("--max-size {s}"))?);
    }
    if let Some(s) = inputs.min_age {
        filter.min_age = Some(parse_duration(s).with_context(|| format!("--min-age {s}"))?);
    }
    if let Some(s) = inputs.max_age {
        filter.max_age = Some(parse_duration(s).with_context(|| format!("--max-age {s}"))?);
    }
    if filter.min_age.is_some() || filter.max_age.is_some() {
        // Captured once per command invocation — calculated by orchestrator-side
        // helper, not by leaf code each time `allows_entry` is called.
        filter.reference_time = Some(SystemTime::now());
    }
    if let Some(path) = inputs.files_from {
        filter.files_from = Some(FileFilter::load_files_from(path)?);
    }
    Ok(filter)
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
    let pre_resolve_display = display_endpoint(&raw_dst);
    let dst_endpoint = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);

    let operation = match mode {
        TransferKind::Copy => "copy",
        TransferKind::Mirror => "mirror",
    };
    let src_display = display_endpoint(&src_endpoint);
    let dst_display = display_endpoint(&dst_endpoint);

    // R54-F1 (data-loss / silent bug): `--null` only works
    // correctly for LOCAL COPY. Outside that envelope it's
    // either destructive or silently ignored:
    //   - `blit mirror --null`: the null sink discards writes,
    //     but `apply_mirror_deletions` still runs (only
    //     `options.dry_run` gates the actual remove_* calls)
    //     and deletes destination-only files — turning a
    //     supposedly read-only benchmark into a destructive op.
    //   - `blit copy --null` to/from a remote endpoint: the
    //     remote push/pull paths don't implement null
    //     semantics, so the flag is silently ignored and a
    //     normal write happens.
    //
    // The narrowest safe contract for 0.1.0: --null is local
    // copy only. Reject the other combinations at the CLI;
    // proper plumbing of null semantics through mirror-delete
    // and the remote paths is a post-release item.
    if args.null {
        if matches!(mode, TransferKind::Mirror) {
            bail!(
                "--null is not supported with `blit mirror`: the \
                 null sink discards writes, but mirror's \
                 destination-purge step would still delete \
                 destination-only files, turning what's supposed \
                 to be a read-only benchmark into a destructive \
                 operation. Use `blit copy --null SRC DST` (local \
                 only) for read-path benchmarking."
            );
        }
        if matches!(src_endpoint, Endpoint::Remote(_))
            || matches!(dst_endpoint, Endpoint::Remote(_))
        {
            bail!(
                "--null is not supported with remote endpoints: \
                 the remote push/pull paths don't implement null \
                 semantics, so the flag would be silently \
                 ignored and a real write would happen. Use \
                 `blit copy --null SRC DST` between two local \
                 paths for read-path benchmarking."
            );
        }
    }

    // For mirror operations, prompt unless --yes or --dry-run
    if matches!(mode, TransferKind::Mirror) && !args.dry_run {
        let prompt = format!(
            "Mirror will delete extraneous files at destination '{}'. Continue?",
            dst_display
        );
        if !confirm_destructive_operation(&prompt, args.yes)? {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Banner goes to stderr so stdout stays reserved for the summary /
    // JSON output. Version dropped — `blit --version` is the right place
    // for that, not every invocation.
    if !args.json {
        eprintln!("starting {} {} -> {}", operation, src_display, dst_display);
        if args.verbose && dst_display != pre_resolve_display {
            eprintln!(
                "  (destination resolved by rsync trailing-slash rule: {} -> {})",
                pre_resolve_display, dst_display
            );
        }
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
            .map(|_| ())
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            ensure_remote_push_supported(args)?;
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
            ensure_remote_pull_supported(args)?;
            ensure_remote_source_supported(&remote)?;
            run_remote_pull_transfer(
                args,
                remote,
                &dst_path,
                matches!(mode, TransferKind::Mirror),
                false, // not a move — source survives
            )
            .await
        }
        (Endpoint::Remote(src), Endpoint::Remote(dst)) => {
            ensure_remote_source_supported(&src)?;
            ensure_remote_destination_supported(&dst)?;
            if args.relay_via_cli {
                ensure_remote_push_supported(args)?;
                run_remote_push_transfer(
                    args,
                    Endpoint::Remote(src),
                    dst,
                    matches!(mode, TransferKind::Mirror),
                )
                .await
            } else {
                ensure_remote_pull_supported(args)?;
                run_remote_to_remote_direct(
                    args,
                    src,
                    dst,
                    matches!(mode, TransferKind::Mirror),
                    false, // not a move
                )
                .await
            }
        }
    }
}

pub async fn run_move(ctx: &AppContext, args: &TransferArgs) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let raw_dst = parse_transfer_endpoint(&args.destination)?;
    let pre_resolve_display = display_endpoint(&raw_dst);
    let dst_endpoint = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);

    if args.dry_run {
        bail!("move does not support --dry-run");
    }

    // R49-F1 (data-loss): reject `--exclude` / `--include` /
    // `--min-size` / `--max-size` / `--min-age` / `--max-age` /
    // `--files-from` on `blit move`. Move means "transfer the
    // source, then delete it." With a filter, files that match
    // the exclude rule (or that fail the include rule) are
    // skipped during the transfer — but the source-delete step
    // would still remove them, losing data the user explicitly
    // didn't want copied.
    let filters_set = !args.exclude.is_empty()
        || !args.include.is_empty()
        || args.min_size.is_some()
        || args.max_size.is_some()
        || args.min_age.is_some()
        || args.max_age.is_some()
        || args.files_from.is_some();
    if filters_set {
        bail!(
            "move does not support filters (--exclude / --include / \
             --min-size / --max-size / --min-age / --max-age / \
             --files-from): the source-delete step would silently \
             remove files that were filtered out of the transfer. \
             Run `blit copy` with filters first, then `blit rm` the \
             remaining source manually if needed."
        );
    }

    // R51-F1 (data-loss): reject `--ignore-existing` for the same
    // reason as filters. The planner drops any source file whose
    // destination already exists (diff_planner.rs:135), so
    // `blit move --ignore-existing` would skip `src/foo` whenever
    // `dst/foo` was already present and then delete `src/foo`
    // along with the rest of the source tree — silent data loss
    // for files that look pre-existing on the destination but
    // diverged from the source side.
    if args.ignore_existing {
        bail!(
            "move does not support --ignore-existing: the source \
             file would be skipped during the transfer and then \
             permanently removed by the source-delete step. Run \
             `blit copy --ignore-existing` first, then `blit rm` \
             the source manually if you really want that semantic."
        );
    }

    // R52-F1 (data-loss): reject `--null`. The flag routes the
    // local transfer into `null_sink`, which deliberately writes
    // nothing — then move's source-delete step removes the
    // original. Net effect: `blit move --null --yes src/ dst/`
    // erases src without ever creating dst contents. --null is
    // a benchmarking/diagnostics primitive; it has no meaningful
    // semantic combined with move.
    if args.null {
        bail!(
            "move does not support --null: --null writes nothing \
             to the destination, but move would still delete the \
             source afterward, which would erase data with no \
             copy. Use --null only with `blit copy SRC DST` \
             between two local paths for read-path benchmarking."
        );
    }

    // R54-F2 (data-loss): reject `--force` and `--ignore-times`
    // for move. Both flags are documented as "unconditionally
    // transfer regardless of size/mtime match," but neither is
    // currently plumbed through `LocalMirrorOptions` /
    // `PushControl`'s comparison-mode selection — the local and
    // local→remote paths fall through to size+mtime regardless.
    // For move that means a stale destination with matching
    // size+mtime is treated as up-to-date, the source isn't
    // copied, and then the source-delete step removes it.
    //
    // R55: the error messages must NOT point users at a workaround
    // that has the same data-loss class. Specifically:
    //   - `blit copy --force` / `--ignore-times`: not plumbed for
    //     local-source paths either — same skip-then-delete bug
    //     hits in the recommended copy step.
    //   - `blit copy --checksum`: works end-to-end for local→local
    //     and for remote-source pull (PullSyncOptions.checksum is
    //     honored), but local→remote push at
    //     `daemon/src/service/push/control.rs:419` decides need-list
    //     by size+mtime only regardless. So --checksum is safe to
    //     recommend for local-to-local, NOT for local-to-remote.
    // The remediation in each branch below is tailored to what
    // the user actually has available as a safe escape hatch.
    if args.force {
        bail!(
            "move does not support --force: the local and \
             local→remote transfer paths don't currently honor \
             this flag in their comparison mode, so a stale \
             destination with matching size+mtime would be \
             treated as up-to-date — the source would be \
             skipped during the transfer and then deleted by \
             move.\n\
             \n\
             Safe escape hatches by direction:\n\
               local→local: `blit copy --checksum SRC DST` \
             (content comparison) then `blit rm SRC` once you've \
             verified the result.\n\
               remote-source: `blit copy --checksum REMOTE DST` \
             (--checksum is honored end-to-end on the pull path) \
             then delete the remote source manually.\n\
               local→remote: NO automatic safe replacement — the \
             daemon's push receive compares by size+mtime only \
             regardless of --checksum. `touch` source files to \
             bump mtime before transfer, or compare contents \
             out-of-band, then move.\n\
             \n\
             Proper plumbing of --force/--ignore-times through \
             the local and push comparison paths is a post-0.1.0 \
             item."
        );
    }
    if args.ignore_times {
        bail!(
            "move does not support --ignore-times: same reason \
             as --force — the local and local→remote paths fall \
             through to size+mtime comparison regardless of \
             this flag, so a stale destination with matching \
             size+mtime would be treated as up-to-date and the \
             source-delete step would lose data.\n\
             \n\
             Safe escape hatches by direction:\n\
               local→local: `blit copy --checksum SRC DST` then \
             `blit rm SRC` once verified.\n\
               remote-source: `blit copy --checksum REMOTE DST` \
             then delete the remote source manually.\n\
               local→remote: NO automatic safe replacement — \
             daemon push compares by size+mtime only. `touch` \
             source files to bump mtime first, or verify \
             contents out-of-band, then move.\n\
             \n\
             Proper plumbing of --ignore-times through the local \
             and push paths is a post-0.1.0 item."
        );
    }

    // Prompt for confirmation before move (which deletes source)
    let src_display = display_endpoint(&src_endpoint);
    let dst_display = display_endpoint(&dst_endpoint);
    let prompt = format!(
        "Move will transfer '{}' to '{}' and delete the source. Continue?",
        src_display, dst_display
    );
    if !confirm_destructive_operation(&prompt, args.yes)? {
        println!("Aborted.");
        return Ok(());
    }

    if !args.json {
        eprintln!("starting move {} -> {}", src_display, dst_display);
        if args.verbose && dst_display != pre_resolve_display {
            eprintln!(
                "  (destination resolved by rsync trailing-slash rule: {} -> {})",
                pre_resolve_display, dst_display
            );
        }
    }

    match (src_endpoint, dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            // R46-F1 (data-loss): pass `mirror=false` here. `move`
            // means "copy + delete source," NOT "purge unrelated
            // destination entries."
            //
            // R49-F3: use the deferred-output variant so the
            // "success" summary doesn't hit stdout until AFTER the
            // source-delete decision. Pre-fix, run_local_transfer
            // emitted the JSON/human summary before returning, so
            // a subsequent unreadable-paths refusal would exit
            // non-zero while stdout already contained a
            // "successful copy" document.
            let summary =
                local::run_local_transfer_deferred(ctx, args, &src_path, &dst_path, false).await?;

            // R47-F4 (data-loss): refuse to delete the source if
            // the scan was incomplete. The R46-F2 mirror gate only
            // fires when `mirror=true`, but move uses mirror=false,
            // so unreadable source files would be silently skipped
            // during the copy and then permanently removed from
            // the source by the `remove_dir_all` below.
            if !summary.unreadable_paths.is_empty() {
                let preview = summary
                    .unreadable_paths
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ");
                bail!(
                    "refusing to remove source {}: scan was \
                     incomplete ({} unreadable entr{}); the first \
                     {} reported: {}. Files we couldn't read were \
                     skipped during the copy — deleting the source \
                     now would lose them. Resolve the scan errors \
                     (typically permissions) and re-run.",
                    src_path.display(),
                    summary.unreadable_paths.len(),
                    if summary.unreadable_paths.len() == 1 {
                        "y"
                    } else {
                        "ies"
                    },
                    summary.unreadable_paths.len().min(5),
                    preview
                );
            }

            if src_path.is_dir() {
                fs::remove_dir_all(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            } else if src_path.is_file() {
                fs::remove_file(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            }

            // R49-F3: source-delete succeeded, emit the deferred
            // summary now so JSON consumers see one self-contained
            // success document (or no output at all on the failure
            // path above).
            local::print_local_transfer_summary(
                ctx,
                args,
                false,
                &summary,
                summary.duration,
                &src_path,
                &dst_path,
            )?;
            Ok(())
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            ensure_remote_pull_supported(args)?;
            ensure_remote_source_supported(&remote)?;
            // R49-F2: require_complete_scan=true so the source
            // daemon refuses partial scans before we delete the
            // remote source via delete_remote_path below.
            // R51-F4: defer output so a failure during the
            // remote-source delete doesn't leave a success-looking
            // transfer summary on stdout.
            let state = remote::run_remote_pull_transfer_deferred(
                args,
                remote.clone(),
                &dst_path,
                false,
                true,
            )
            .await?;

            // Delete remote source
            let rel_path = match &remote.path {
                RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
                    rel_path.to_string_lossy().into_owned()
                }
                _ => bail!("unsupported remote source for move"),
            };
            delete_remote_path(&remote, &rel_path).await?;
            remote::print_deferred_pull_result(args, &state);
            Ok(())
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            ensure_remote_push_supported(args)?;
            ensure_remote_destination_supported(&remote)?;
            // Push uses local-source scanning on the CLI side,
            // which routes through spawn_manifest_task's unreadable
            // accumulator and surfaces failures pre-transfer.
            // R51-F4: defer output so a failure during the
            // local-source delete doesn't leave a success-looking
            // transfer summary on stdout.
            let state = remote::run_remote_push_transfer_deferred(
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
            remote::print_deferred_push_result(args, &state);
            Ok(())
        }
        (Endpoint::Remote(src), Endpoint::Remote(dst)) => {
            ensure_remote_source_supported(&src)?;
            ensure_remote_destination_supported(&dst)?;
            // R50-F1 / R51-F2 (data-loss): reject `--relay-via-cli`
            // for remote-source move. The relay path goes through
            // `run_remote_push_transfer` → `RemoteTransferSource::
            // scan`, which uses the legacy metadata-only Pull RPC
            // (`collect_pull_entries` discards
            // EnumerationOutcome). There's no scan-complete signal
            // to thread through that path without restructuring
            // the legacy RPC, so for 0.1.0 we close the data-loss
            // window by refusing the combination entirely. The
            // direct delegated path (default) carries the
            // require_complete_scan signal end-to-end.
            if args.relay_via_cli {
                bail!(
                    "move does not support --relay-via-cli with a \
                     remote source: the legacy relay path does not \
                     verify that the source-side scan was complete, \
                     so an unreadable subtree on the source daemon \
                     would be silently lost when the source is \
                     deleted. Drop --relay-via-cli to use the \
                     direct delegated path, which enforces the \
                     complete-scan gate."
                );
            }
            ensure_remote_pull_supported(args)?;
            // R49-F2: require_complete_scan=true so the source
            // daemon refuses partial scans before delete_remote_path
            // below.
            // R51-F4: defer output so a remote-source delete
            // failure doesn't leave a success-looking delegated
            // summary on stdout.
            let state = remote_remote_direct::run_remote_to_remote_direct_deferred(
                args,
                src.clone(),
                dst,
                false,
                true,
            )
            .await?;

            // Delete remote source
            let rel_path = match &src.path {
                RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
                    rel_path.to_string_lossy().into_owned()
                }
                _ => bail!("unsupported remote source for move"),
            };
            delete_remote_path(&src, &rel_path).await?;
            remote_remote_direct::print_deferred_delegated_result(args, &state);
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
            relay_via_cli: false,
            resume: false,
            null: false,
            json: false,
            exclude: vec![],
            include: vec![],
            files_from: None,
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
            delete_scope: "subset".into(),
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
            relay_via_cli: false,
            resume: false,
            null: false,
            json: false,
            exclude: vec![],
            include: vec![],
            files_from: None,
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
            delete_scope: "subset".into(),
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
        let resolved =
            resolve_destination("/a/GameDir", "/definitely/does/not/exist/newdst", &src, dst);
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
        let resolved = resolve_destination("/a/file.txt", "/b/renamed.txt", &src, dst);
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
        let resolved = resolve_destination("/a/GameDir", "h:/m/common/", &src, dst);
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
        let resolved = resolve_destination("/a/GameDir", "h:/m/common/target", &src, dst);
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
        let resolved = resolve_destination("h:/m/Games/DOOM", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, tmp.path().join("DOOM")),
            _ => panic!("expected local endpoint"),
        }
    }
}
