mod endpoints;
mod local;
mod remote;
mod remote_remote_direct;

// Endpoint types come from `blit_app::endpoints` directly. The
// `transfers/endpoints.rs` shim now contains only the two
// clap-arg adapter wrappers (`ensure_remote_pull_supported` /
// `ensure_remote_push_supported`) — every other consumer
// imports from `blit_app::endpoints` directly.
use blit_app::endpoints::{format_remote_endpoint, parse_transfer_endpoint, Endpoint};

use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{bail, Context, Result};
use std::fs;
use std::io::{self, Write};

use crate::rm::delete_remote_path;
use blit_app::transfers::dispatch::{select_transfer_route, TransferKind, TransferRoute};
use blit_app::transfers::filter::{self, FilterInputs};
use blit_app::transfers::resolution::resolve_destination;
use blit_core::fs_enum::FileFilter;
use blit_core::remote::RemotePath;

/// Build a `FilterInputs` view over a `TransferArgs`. Lives here
/// because the orphan rule prevents `impl From<&TransferArgs>` on
/// `FilterInputs` (the struct moved to `blit-app::transfers::filter`,
/// `TransferArgs` stays in `blit-cli`). Inlined wrapper keeps the
/// `build_filter` / `build_filter_spec` call sites readable.
fn filter_inputs(args: &TransferArgs) -> FilterInputs<'_> {
    FilterInputs {
        include: &args.include,
        exclude: &args.exclude,
        files_from: args.files_from.as_ref(),
        min_size: args.min_size.as_deref(),
        max_size: args.max_size.as_deref(),
        min_age: args.min_age.as_deref(),
        max_age: args.max_age.as_deref(),
    }
}
use blit_app::endpoints::{ensure_remote_destination_supported, ensure_remote_source_supported};
use endpoints::{ensure_remote_pull_supported, ensure_remote_push_supported};
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

/// Build a `FileFilter` from a transfer command's args. Thin
/// clap-side wrapper around `blit_app::transfers::filter::build`.
pub(crate) fn build_filter(args: &TransferArgs) -> Result<FileFilter> {
    filter::build(&filter_inputs(args))
}

/// Build the wire-side `FilterSpec` proto from CLI args. Thin
/// wrapper around `blit_app::transfers::filter::build_spec`.
pub(crate) fn build_filter_spec(args: &TransferArgs) -> Result<blit_core::generated::FilterSpec> {
    filter::build_spec(&filter_inputs(args))
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
        if mode.is_mirror() {
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

    // `--detach` is only honored on daemon-to-daemon
    // delegated transfers. The CLI gates it up-front so a
    // misuse fails before any RPCs fire — clearer than
    // letting the daemon emit a phased error mid-stream.
    if args.detach {
        match (&src_endpoint, &dst_endpoint) {
            (Endpoint::Local(_), _) | (_, Endpoint::Local(_)) => bail!(
                "--detach is only supported for remote→remote transfers \
                 (the CLI is in the byte path for any local endpoint, so \
                 disconnecting would drop the bytes)"
            ),
            (Endpoint::Remote(_), Endpoint::Remote(_)) => {
                // Delegated remote→remote — detach is valid.
            }
        }
    }

    warn_if_dropping_windows_metadata(args);

    // For mirror operations, prompt unless --yes or --dry-run
    if mode.is_mirror() && !args.dry_run {
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

    match select_transfer_route(src_endpoint, dst_endpoint, mode) {
        TransferRoute::LocalToLocal { src, dst, mirror } => {
            if !src.exists() {
                bail!("source path does not exist: {}", src.display());
            }
            run_local_transfer(ctx, args, &src, &dst, mirror)
                .await
                .map(|_| ())
        }
        TransferRoute::LocalToRemote { src, dst, mirror } => {
            if !src.exists() {
                bail!("source path does not exist: {}", src.display());
            }
            ensure_remote_push_supported(args)?;
            ensure_remote_destination_supported(&dst)?;
            run_remote_push_transfer(args, src, dst, mirror).await
        }
        TransferRoute::RemoteToLocal { src, dst, mirror } => {
            ensure_remote_pull_supported(args)?;
            ensure_remote_source_supported(&src)?;
            run_remote_pull_transfer(
                args, src, &dst, mirror, false, // not a move — source survives
            )
            .await
        }
        TransferRoute::RemoteToRemoteDelegated { src, dst, mirror } => {
            ensure_remote_source_supported(&src)?;
            ensure_remote_destination_supported(&dst)?;
            ensure_remote_pull_supported(args)?;
            run_remote_to_remote_direct(args, src, dst, mirror, false /* not a move */).await
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

    if args.detach {
        // `blit move` runs a source-delete step after the
        // transfer completes. With --detach the CLI exits as
        // soon as the daemon's Started event arrives, so the
        // delete step would never fire — either leaving the
        // source around forever (silent move-becomes-copy) or
        // racing the still-running transfer with rm. Refuse
        // up front.
        bail!(
            "move does not support --detach: the source-delete step \
             needs the CLI to await transfer completion, so detaching \
             would silently turn a move into a copy. Use \
             `blit copy --detach SRC DST` and `blit rm SRC` once you've \
             confirmed the transfer completed via `blit jobs list`."
        );
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

    // R54-F2 (data-loss), refreshed at otp-10b-2 (and its codex F3):
    // reject `--force`, `--ignore-times`, and `--size-only` for move.
    //
    // Every move route — local, push, pull, delegated — now maps
    // through the move compare rule (transfer unconditionally, or
    // `--checksum` for the one skip that is content-proven safe;
    // codex otp-10a F1 and its otp-10b-2 mirrors). That makes
    // `--force` / `--ignore-times` pure no-ops on move: reject them
    // rather than silently absorb them.
    //
    // `--size-only` is worse than redundant: honoring it would mean a
    // size-only skip of a changed file followed by source-delete —
    // destroying the only copy, the exact hazard the move mapping
    // exists to prevent. The old pull driver honored it on move; the
    // gate closes that hole.
    //
    // R55 still applies: escape hatches must not have the same
    // data-loss class. `blit move --checksum` is honored on every
    // route (the session's Checksum compare is role-agnostic since
    // otp-10b-1/2; the local path honors it via R58-F7).
    if args.force {
        bail!(
            "move does not support --force: move already transfers \
             every file unconditionally (a compare-mode skip before \
             the source-delete could lose data), so the flag adds \
             nothing. Use `blit move --checksum` to skip only files \
             whose content is proven identical."
        );
    }
    if args.ignore_times {
        bail!(
            "move does not support --ignore-times: move already \
             transfers every file unconditionally (a compare-mode \
             skip before the source-delete could lose data), so the \
             flag adds nothing. Use `blit move --checksum` to skip \
             only files whose content is proven identical."
        );
    }
    if args.size_only {
        bail!(
            "move does not support --size-only: a same-size file \
             whose content changed would be skipped during the \
             transfer and then permanently removed by the \
             source-delete step. Use `blit move --checksum` for a \
             content-verified move, or plain `blit move` (which \
             transfers every file unconditionally)."
        );
    }

    warn_if_dropping_windows_metadata(args);

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
            // move_verb=true: the session refuses partial source
            // scans (R49-F2 / otp-9b F1) before we delete the remote
            // source via delete_remote_path below, and the move
            // compare mapping transfers unconditionally (codex
            // otp-10a F1, mirrored on pull at otp-10b-2).
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
                src_path.clone(),
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

const DROP_WINDOWS_METADATA_WARNING: &str =
    "warning: --drop-windows-metadata permanently discards Windows file attributes and named data streams";

fn warn_if_dropping_windows_metadata(args: &TransferArgs) {
    if args.drop_windows_metadata {
        eprintln!("{DROP_WINDOWS_METADATA_WARNING}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            verbose: false,
            progress: false,
            yes: true, // Skip prompts in tests
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
            detach: false,
            resume: false,
            drop_windows_metadata: false,
            retry: 0,
            wait: 5,
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
            verbose: false,
            progress: false,
            yes: true, // Skip prompts in tests
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
            detach: false,
            resume: false,
            drop_windows_metadata: false,
            retry: 0,
            wait: 5,
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

    // rsync-resolution unit tests moved alongside the
    // implementation to `blit_app::transfers::resolution`. The CLI
    // module's tests retain only the end-to-end dispatcher tests
    // above (`copy_local_transfers_file`,
    // `copy_local_dry_run_creates_no_files`).

    /// Build a minimal `TransferArgs` for the gate-rejection
    /// tests below. Source / destination are stringy and never
    /// touched by the path we're exercising — the bail happens
    /// before any RPC fires.
    ///
    /// `yes` controls the `args.yes` field. The audit-h1 round-2
    /// reviewer caught that the original helper hardcoded
    /// `yes: true`, which let the mirror destructive-confirm
    /// prompt mask a missing reject-gate; all gate-rejection
    /// tests explicitly opt into the `yes` value they want to
    /// exercise.
    fn gate_args(source: &str, destination: &str, detach: bool, yes: bool) -> TransferArgs {
        TransferArgs {
            source: source.to_string(),
            destination: destination.to_string(),
            dry_run: false,
            checksum: false,
            size_only: false,
            ignore_times: false,
            ignore_existing: false,
            force: false,
            verbose: false,
            progress: false,
            yes,
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
            detach,
            resume: false,
            drop_windows_metadata: false,
            retry: 0,
            wait: 5,
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
        }
    }

    fn ctx() -> AppContext {
        AppContext {
            perf_history_enabled: false,
        }
    }

    #[test]
    fn detach_rejected_for_local_to_local() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        let ctx = ctx();
        let args = gate_args(src.to_str().unwrap(), dst.to_str().unwrap(), true, true);
        let err = runtime()
            .block_on(run_transfer(&ctx, &args, TransferKind::Copy))
            .expect_err("local→local must reject --detach");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("--detach is only supported for remote→remote"),
            "got: {msg}"
        );
    }

    #[test]
    fn detach_rejected_on_blit_move() {
        let ctx = ctx();
        let args = gate_args("host-a:/m/", "host-b:/m/", true, true);
        let err = runtime()
            .block_on(run_move(&ctx, &args))
            .expect_err("move + --detach must bail");
        let msg = format!("{err:#}");
        assert!(msg.contains("move does not support --detach"), "got: {msg}");
    }
}
