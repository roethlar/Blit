use crate::cli::{JobsCancelArgs, JobsCommand, JobsListArgs, JobsWatchArgs};
use blit_app::admin::jobs;
use blit_app::admin::jobs::{CancelJobOutcome, WatchSnapshot};
use blit_core::generated::{daemon_event, DaemonState};
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Return shape from [`run_jobs`]. `list` always exits with
/// success once the RPC returned cleanly; `cancel` carries
/// the per-outcome exit code mandated by the CLI contract
/// (`docs/plan/TUI_DESIGN.md` §6.5):
///
///   Cancelled  → 0
///   NotFound   → 1
///   Unsupported → 2
///
/// Same pattern as `run_check`: the verb owns the
/// `ExitCode`, `main` returns it.
pub async fn run_jobs(command: JobsCommand) -> Result<ExitCode> {
    match command {
        JobsCommand::List(args) => {
            run_jobs_list(args).await?;
            Ok(ExitCode::SUCCESS)
        }
        JobsCommand::Cancel(args) => run_jobs_cancel(args).await,
        JobsCommand::Watch(args) => run_jobs_watch(args).await,
    }
}

async fn run_jobs_list(args: JobsListArgs) -> Result<()> {
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    let state = jobs::query(&remote, args.recent_limit).await?;

    if args.json {
        print_json(&state)?;
    } else {
        print_human(&remote, &state);
    }
    Ok(())
}

async fn run_jobs_cancel(args: JobsCancelArgs) -> Result<ExitCode> {
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    let outcome = jobs::cancel(&remote, &args.transfer_id).await?;
    if args.json {
        print_cancel_json(&outcome);
    } else {
        print_cancel_human(&remote, &outcome);
    }
    Ok(cancel_exit_code(&outcome))
}

/// Map [`CancelJobOutcome`] to the contract's exit codes.
/// Pulled out as a sync helper so unit tests can pin the
/// mapping without spinning up a tonic server.
pub(crate) fn cancel_exit_code(outcome: &CancelJobOutcome) -> ExitCode {
    match outcome {
        CancelJobOutcome::Cancelled { .. } => ExitCode::SUCCESS,
        CancelJobOutcome::NotFound { .. } => ExitCode::from(1),
        CancelJobOutcome::Unsupported { .. } => ExitCode::from(2),
    }
}

/// Snapshot of the active row's metadata, captured by the
/// initial `GetState` before the streaming loop. Used to merge
/// the wire `TransferComplete` / `TransferError` event's
/// (sparse) fields back into the pre-existing
/// `WatchSnapshot::Finished` JSON schema so JSON-Lines
/// consumers see a stable terminal shape on both the
/// snapshot-finished and stream-finished paths.
struct ActiveSnapshot {
    kind: i32,
    peer: String,
    module: String,
    path: String,
    start_unix_ms: u64,
}

impl ActiveSnapshot {
    /// Merge with a `TransferComplete` to produce a
    /// `TransferRecord`-shaped value that matches the JSON
    /// schema emitted by `print_watch_json(Finished(...))`.
    fn to_finished_complete(
        &self,
        c: &blit_core::generated::TransferComplete,
    ) -> blit_core::generated::TransferRecord {
        blit_core::generated::TransferRecord {
            transfer_id: c.transfer_id.clone(),
            kind: self.kind,
            peer: self.peer.clone(),
            module: self.module.clone(),
            path: self.path.clone(),
            start_unix_ms: self.start_unix_ms,
            duration_ms: c.duration_ms,
            bytes: c.bytes,
            files: c.files,
            ok: true,
            error_message: String::new(),
        }
    }

    /// Merge with a `TransferError` to produce the same shape.
    /// `duration_ms` derives from `start_unix_ms` since the
    /// event itself doesn't carry it; `bytes` / `files` stay
    /// at zero (unknown on the error path).
    fn to_finished_error(
        &self,
        e: &blit_core::generated::TransferError,
    ) -> blit_core::generated::TransferRecord {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let duration_ms = now_ms.saturating_sub(self.start_unix_ms);
        blit_core::generated::TransferRecord {
            transfer_id: e.transfer_id.clone(),
            kind: self.kind,
            peer: self.peer.clone(),
            module: self.module.clone(),
            path: self.path.clone(),
            start_unix_ms: self.start_unix_ms,
            duration_ms,
            bytes: 0,
            files: 0,
            ok: false,
            error_message: e.message.clone(),
        }
    }
}

/// Stream live progress for a single transfer until it
/// terminates or the optional timeout fires. Uses the c-2
/// `Subscribe` RPC scoped by c-5a's `transfer_id_filter` so
/// the CLI only receives events for the watched transfer.
///
/// Exit codes:
///
///   Finished + ok=true   → 0
///   Finished + ok=false  → 1
///   NotFound             → 2 (id never seen, or completed
///                              before subscribe + rotated out
///                              of the recent ring)
///   Timeout while active → 3 (deadline fired before any
///                              terminal event arrived)
///
/// Flow:
/// 1. Open the Subscribe stream FIRST — registers our
///    per-subscriber forwarder with the daemon so any terminal
///    event fired after this point lands in the mpsc and is
///    observable on the loop's first `message().await`.
/// 2. Query GetState. Three branches:
///    - Already in recent[] → drop stream, emit terminal,
///      return appropriate exit code.
///    - In active[]        → emit initial line, cache active
///      metadata, fall through to stream loop.
///    - NotFound           → emit not-found, return 2.
/// 3. Consume Subscribe stream events for the transfer:
///    - TransferProgress → update progress line / JSON.
///    - TransferComplete → emit terminal line, return 0.
///    - TransferError    → emit failed line, return 1.
///    - TransferStarted  → ignored (initial GetState already
///      reported state).
/// 4. Stream errors fall back to a final GetState query so a
///    Subscribe Lagged or daemon disconnect doesn't leave the
///    operator without a terminal answer.
///
/// `args.interval_ms` is preserved on the CLI for backward
/// compatibility but has no effect under the streaming model
/// — Subscribe pushes; no polling cadence to configure.
async fn run_jobs_watch(args: JobsWatchArgs) -> Result<ExitCode> {
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    if args.transfer_id.trim().is_empty() {
        eyre::bail!("transfer_id must not be empty");
    }
    let deadline = if args.timeout_secs > 0 {
        Some(Instant::now() + Duration::from_secs(args.timeout_secs))
    } else {
        None
    };

    if !args.json {
        eprintln!(
            "Watching transfer {} on {} (streaming)...",
            args.transfer_id,
            remote.display(),
        );
    }

    // c-6 round 2: subscribe FIRST so terminal events that
    // fire between the snapshot and our stream registration
    // land in the per-subscriber mpsc and are observable on
    // the loop's first `message().await`. The original
    // ordering (GetState first, Subscribe second) allowed a
    // race: transfer was Active at snapshot time, then drained
    // before Subscribe registered, terminal events broadcast
    // before our receiver existed, no replay (c-5b deferred),
    // and the stream hung forever waiting for a transfer_id
    // that's never going to fire again.
    // c-7: ask for replay_recent so any TransferProgress
    // events that fired between our snapshot and the next
    // tick land in the stream immediately instead of waiting
    // up to ~100ms. The replayed TransferStarted that comes
    // through is harmless — the loop's TransferStarted arm is
    // a no-op since the initial GetState already rendered the
    // active line.
    let mut stream = jobs::subscribe(&remote, &args.transfer_id, true).await?;

    // Step 1: GetState snapshot so we handle the already-
    // completed and never-existed cases (and the in-flight
    // case where we want to render an initial line + cache
    // metadata for terminal JSON merge).
    let state = jobs::query(&remote, 0).await?;
    let snap = jobs::watch_snapshot(&state, &args.transfer_id);
    let active_snapshot = match &snap {
        WatchSnapshot::Finished(r) => {
            if args.json {
                print_watch_json(&snap);
            } else {
                emit_human_finished(r);
            }
            return Ok(if r.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            });
        }
        WatchSnapshot::NotFound => {
            if args.json {
                print_watch_json(&snap);
            } else {
                eprintln!(
                    "[not-found] transfer '{}' is not on {} (already completed \
                     and rotated out of the recent ring, or never existed)",
                    args.transfer_id,
                    remote.display()
                );
            }
            return Ok(ExitCode::from(2));
        }
        WatchSnapshot::Active(a) => {
            if args.json {
                print_watch_json(&snap);
            } else {
                emit_human_active(a, None);
            }
            // c-6 round 2: cache the active row's metadata so
            // when a terminal event arrives over the stream we
            // can synthesize a `WatchSnapshot::Finished`-shaped
            // JSON object — same schema (kind, peer, module,
            // path, start_unix_ms, duration_ms, ok,
            // error_message) that the snapshot-finished path
            // emits. Subscribers iterating JSON-Lines see one
            // stable terminal shape regardless of which path
            // produced it.
            ActiveSnapshot {
                kind: a.kind,
                peer: a.peer.clone(),
                module: a.module.clone(),
                path: a.path.clone(),
                start_unix_ms: a.start_unix_ms,
            }
        }
    };

    loop {
        // tonic's `Streaming::message()` returns
        // `Result<Option<T>, Status>`:
        //   Ok(Some(msg))  → forward frame
        //   Ok(None)        → stream ended cleanly
        //   Err(status)     → stream error (Aborted = Lagged)
        let next_message = match deadline {
            Some(d) => {
                let remaining = d.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    if args.json {
                        print_watch_timeout_json(&args.transfer_id, args.timeout_secs);
                    } else {
                        eprintln!(
                            "[timeout] transfer '{}' still active after {}s",
                            args.transfer_id, args.timeout_secs
                        );
                    }
                    return Ok(ExitCode::from(3));
                }
                match tokio::time::timeout(remaining, stream.message()).await {
                    Ok(item) => item,
                    Err(_) => {
                        if args.json {
                            print_watch_timeout_json(&args.transfer_id, args.timeout_secs);
                        } else {
                            eprintln!(
                                "[timeout] transfer '{}' still active after {}s",
                                args.transfer_id, args.timeout_secs
                            );
                        }
                        return Ok(ExitCode::from(3));
                    }
                }
            }
            None => stream.message().await,
        };
        match next_message {
            Ok(Some(event)) => match event.payload {
                Some(daemon_event::Payload::TransferProgress(p)) => {
                    if args.json {
                        print_watch_progress_json(&p);
                    } else {
                        emit_human_progress(&args.transfer_id, &p);
                    }
                }
                Some(daemon_event::Payload::TransferComplete(c)) => {
                    if args.json {
                        // Synthesize a Finished-shaped JSON
                        // object by merging the event's fields
                        // with the cached active snapshot —
                        // schema matches the GetState-finished
                        // path so JSON-Lines consumers see one
                        // stable terminal shape.
                        let merged = active_snapshot.to_finished_complete(&c);
                        print_watch_json(&WatchSnapshot::Finished(merged));
                    } else {
                        emit_human_complete(&c);
                    }
                    return Ok(ExitCode::SUCCESS);
                }
                Some(daemon_event::Payload::TransferError(e)) => {
                    if args.json {
                        let merged = active_snapshot.to_finished_error(&e);
                        print_watch_json(&WatchSnapshot::Finished(merged));
                    } else {
                        eprintln!("blit: transfer '{}' failed: {}", e.transfer_id, e.message);
                    }
                    return Ok(ExitCode::from(1));
                }
                Some(daemon_event::Payload::TransferStarted(_)) | None => {
                    // Started already covered by the initial
                    // GetState. None happens for a future
                    // wire variant we don't recognize — drop.
                }
            },
            Err(status) => {
                // Stream error (typically Lagged → Aborted).
                // Fall back to a final GetState so the operator
                // gets a terminal answer rather than a stream
                // failure.
                eprintln!(
                    "blit: subscribe stream failed ({}); reconciling via GetState...",
                    status.message()
                );
                return reconcile_via_get_state(&args, &remote).await;
            }
            Ok(None) => {
                // Daemon closed the stream — likely shutting
                // down. Same fallback.
                eprintln!(
                    "blit: daemon closed the subscribe stream; \
                     reconciling via GetState..."
                );
                return reconcile_via_get_state(&args, &remote).await;
            }
        }
    }
}

/// On Subscribe stream error / end, query GetState once more
/// to decide the terminal exit. Mirrors the initial-snapshot
/// branches so the operator always gets a coherent answer.
async fn reconcile_via_get_state(
    args: &JobsWatchArgs,
    remote: &RemoteEndpoint,
) -> Result<ExitCode> {
    let state = jobs::query(remote, 0).await?;
    let snap = jobs::watch_snapshot(&state, &args.transfer_id);
    if args.json {
        print_watch_json(&snap);
    }
    match snap {
        WatchSnapshot::Finished(r) => {
            if !args.json {
                emit_human_finished(&r);
            }
            Ok(if r.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        WatchSnapshot::Active(a) => {
            if !args.json {
                emit_human_active(&a, Some("still active after stream loss"));
            }
            // Stream is gone and the transfer is still active.
            // Without polling we can't follow it further; exit
            // 3 (timeout-equivalent: "we gave up watching").
            Ok(ExitCode::from(3))
        }
        WatchSnapshot::NotFound => {
            if !args.json {
                eprintln!(
                    "[not-found] transfer '{}' is no longer on {}",
                    args.transfer_id,
                    remote.display()
                );
            }
            Ok(ExitCode::from(2))
        }
    }
}

fn emit_human_active(a: &blit_core::generated::ActiveTransfer, note: Option<&str>) {
    let age_ms = age_ms_since(a.start_unix_ms);
    if let Some(note) = note {
        eprintln!(
            "[active] {} {} peer={} age={} ({})",
            jobs::kind_label(a.kind),
            module_path(&a.module, &a.path),
            a.peer,
            format_ms(age_ms),
            note,
        );
    } else {
        eprintln!(
            "[active] {} {} peer={} age={}",
            jobs::kind_label(a.kind),
            module_path(&a.module, &a.path),
            a.peer,
            format_ms(age_ms),
        );
    }
}

fn emit_human_finished(r: &blit_core::generated::TransferRecord) {
    let status = if r.ok {
        "ok".to_string()
    } else {
        format!("FAILED: {}", r.error_message)
    };
    eprintln!(
        "[done] {} {} duration={} {}",
        jobs::kind_label(r.kind),
        module_path(&r.module, &r.path),
        format_ms(r.duration_ms),
        status,
    );
}

fn emit_human_progress(transfer_id: &str, p: &blit_core::generated::TransferProgress) {
    let bps = p.throughput_bps;
    eprintln!(
        "[progress] {} bytes={} throughput={}/s",
        transfer_id,
        p.bytes_completed,
        format_bps(bps),
    );
}

fn emit_human_complete(c: &blit_core::generated::TransferComplete) {
    eprintln!(
        "[done] transfer {} bytes={} duration={} ok",
        c.transfer_id,
        c.bytes,
        format_ms(c.duration_ms),
    );
}

fn format_bps(bps: u64) -> String {
    if bps >= 1_000_000_000 {
        format!("{:.2} GB", bps as f64 / 1_000_000_000.0)
    } else if bps >= 1_000_000 {
        format!("{:.2} MB", bps as f64 / 1_000_000.0)
    } else if bps >= 1_000 {
        format!("{:.2} KB", bps as f64 / 1_000.0)
    } else {
        format!("{} B", bps)
    }
}

fn print_watch_progress_json(p: &blit_core::generated::TransferProgress) {
    use serde_json::json;
    let body = json!({
        "state": "progress",
        "transfer_id": p.transfer_id,
        "bytes_completed": p.bytes_completed,
        "bytes_total": p.bytes_total,
        "files_completed": p.files_completed,
        "files_total": p.files_total,
        "throughput_bps": p.throughput_bps,
    });
    if let Ok(line) = serde_json::to_string(&body) {
        println!("{}", line);
    }
}

// c-6 round 2: the standalone `print_watch_complete_json` /
// `print_watch_error_json` emitters were replaced by merging
// the event into a `WatchSnapshot::Finished` via
// `ActiveSnapshot::into_finished_*`, then routing through the
// existing `print_watch_json`. That keeps the terminal JSON
// schema identical regardless of which path produced it.

fn print_watch_json(snap: &WatchSnapshot) {
    use serde_json::json;
    let body = match snap {
        WatchSnapshot::Active(a) => json!({
            "state": "active",
            "transfer_id": a.transfer_id,
            "kind": jobs::kind_label(a.kind),
            "peer": a.peer,
            "module": a.module,
            "path": a.path,
            "start_unix_ms": a.start_unix_ms,
            "bytes_completed": a.bytes_completed,
            "bytes_total": a.bytes_total,
        }),
        WatchSnapshot::Finished(r) => json!({
            "state": "finished",
            "transfer_id": r.transfer_id,
            "kind": jobs::kind_label(r.kind),
            "peer": r.peer,
            "module": r.module,
            "path": r.path,
            "start_unix_ms": r.start_unix_ms,
            "duration_ms": r.duration_ms,
            "ok": r.ok,
            "error_message": r.error_message,
        }),
        WatchSnapshot::NotFound => json!({
            "state": "not_found",
        }),
    };
    // JSON-Lines: one object per poll, no trailing newline
    // from to_string (println! adds it).
    if let Ok(line) = serde_json::to_string(&body) {
        println!("{}", line);
    }
}

/// Emit the terminal `state: "timeout"` line when --timeout-secs
/// fires while the transfer is still in active[]. JSON consumers
/// rely on the stream having a terminal state line — exit code 3
/// is for shells; the JSON object is for the same stream that's
/// been seeing `state: "active"` rows.
fn print_watch_timeout_json(transfer_id: &str, timeout_secs: u64) {
    use serde_json::json;
    let body = json!({
        "state": "timeout",
        "transfer_id": transfer_id,
        "timeout_secs": timeout_secs,
    });
    if let Ok(line) = serde_json::to_string(&body) {
        println!("{}", line);
    }
}

fn print_cancel_json(outcome: &CancelJobOutcome) {
    use serde_json::json;
    let body = match outcome {
        CancelJobOutcome::Cancelled { transfer_id } => json!({
            "outcome": "cancelled",
            "transfer_id": transfer_id,
        }),
        CancelJobOutcome::NotFound { transfer_id } => json!({
            "outcome": "not_found",
            "transfer_id": transfer_id,
        }),
        CancelJobOutcome::Unsupported {
            transfer_id,
            message,
        } => json!({
            "outcome": "unsupported",
            "transfer_id": transfer_id,
            "message": message,
        }),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );
}

fn print_cancel_human(remote: &RemoteEndpoint, outcome: &CancelJobOutcome) {
    match outcome {
        CancelJobOutcome::Cancelled { transfer_id } => {
            println!("Cancelled transfer {transfer_id} on {}", remote.display());
        }
        CancelJobOutcome::NotFound { transfer_id } => {
            eprintln!(
                "No active transfer with id '{transfer_id}' on {}",
                remote.display()
            );
        }
        CancelJobOutcome::Unsupported {
            transfer_id,
            message,
        } => {
            eprintln!("blit: cannot cancel transfer '{transfer_id}': {message}");
        }
    }
}

fn print_json(state: &DaemonState) -> Result<()> {
    use serde_json::json;
    let active: Vec<_> = state
        .active
        .iter()
        .map(|a| {
            json!({
                "transfer_id": a.transfer_id,
                "kind": jobs::kind_label(a.kind),
                "peer": a.peer,
                "module": a.module,
                "path": a.path,
                "start_unix_ms": a.start_unix_ms,
                "bytes_completed": a.bytes_completed,
                "bytes_total": a.bytes_total,
            })
        })
        .collect();
    let recent: Vec<_> = state
        .recent
        .iter()
        .map(|r| {
            json!({
                "transfer_id": r.transfer_id,
                "kind": jobs::kind_label(r.kind),
                "peer": r.peer,
                "module": r.module,
                "path": r.path,
                "start_unix_ms": r.start_unix_ms,
                "duration_ms": r.duration_ms,
                "bytes": r.bytes,
                "files": r.files,
                "ok": r.ok,
                "error_message": r.error_message,
            })
        })
        .collect();
    let counters = state.counters.as_ref().map(|c| {
        json!({
            "push_operations_total": c.push_operations_total,
            "pull_operations_total": c.pull_operations_total,
            "purge_operations_total": c.purge_operations_total,
            "active_transfers": c.active_transfers,
            "transfer_errors_total": c.transfer_errors_total,
        })
    });
    let modules: Vec<_> = state
        .modules
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "path": m.path,
                "read_only": m.read_only,
            })
        })
        .collect();
    let body = json!({
        "version": state.version,
        "uptime_seconds": state.uptime_seconds,
        "delegation_enabled": state.delegation_enabled,
        "modules": modules,
        "active": active,
        "recent": recent,
        "counters": counters,
    });
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn print_human(remote: &RemoteEndpoint, state: &DaemonState) {
    println!(
        "Daemon: blit {} on {} — uptime {}",
        state.version,
        remote.display(),
        format_uptime(state.uptime_seconds),
    );
    println!(
        "Delegation: {}",
        if state.delegation_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    if state.modules.is_empty() {
        println!("Modules: (none)");
    } else {
        let names: Vec<&str> = state.modules.iter().map(|m| m.name.as_str()).collect();
        println!("Modules: {}", names.join(", "));
    }

    println!();
    if state.active.is_empty() {
        println!("Active: (none)");
    } else {
        println!("Active ({}):", state.active.len());
        for a in &state.active {
            // `<id>  <kind>  <module>/<path>  peer=<peer>  age=<ms>`
            let age_ms = age_ms_since(a.start_unix_ms);
            println!(
                "  {}  {}  {}  peer={}  age={}",
                a.transfer_id,
                jobs::kind_label(a.kind),
                module_path(&a.module, &a.path),
                a.peer,
                format_ms(age_ms),
            );
        }
    }

    println!();
    if state.recent.is_empty() {
        println!("Recent: (none)");
    } else {
        // Display newest-first for human eyes — the wire is
        // oldest-first, so iterate in reverse.
        println!("Recent ({}):", state.recent.len());
        for r in state.recent.iter().rev() {
            let status = if r.ok {
                "ok".to_string()
            } else {
                format!("FAILED: {}", r.error_message)
            };
            println!(
                "  {}  {}  {}  peer={}  duration={}  {}",
                r.transfer_id,
                jobs::kind_label(r.kind),
                module_path(&r.module, &r.path),
                r.peer,
                format_ms(r.duration_ms),
                status,
            );
        }
    }

    if let Some(c) = &state.counters {
        println!();
        println!(
            "Counters: push={} pull={} purge={} active={} errors={}",
            c.push_operations_total,
            c.pull_operations_total,
            c.purge_operations_total,
            c.active_transfers,
            c.transfer_errors_total,
        );
    }
}

fn module_path(module: &str, path: &str) -> String {
    match (module.is_empty(), path.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => path.to_string(),
        (false, true) => module.to_string(),
        (false, false) => format!("{module}/{path}"),
    }
}

fn format_uptime(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

fn format_ms(ms: u64) -> String {
    if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{ms}ms")
    }
}

fn age_ms_since(start_unix_ms: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    now_ms.saturating_sub(start_unix_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_uptime_renders_hours_minutes_seconds() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(45), "45s");
        assert_eq!(format_uptime(125), "2m 5s");
        assert_eq!(format_uptime(3661), "1h 1m");
    }

    #[test]
    fn format_ms_switches_to_seconds_above_1k() {
        assert_eq!(format_ms(0), "0ms");
        assert_eq!(format_ms(999), "999ms");
        assert_eq!(format_ms(1000), "1.0s");
        assert_eq!(format_ms(3500), "3.5s");
    }

    #[test]
    fn module_path_handles_each_empty_combination() {
        assert_eq!(module_path("", ""), "/");
        assert_eq!(module_path("", "p"), "p");
        assert_eq!(module_path("mod", ""), "mod");
        assert_eq!(module_path("mod", "sub/dir"), "mod/sub/dir");
    }

    /// `ExitCode` doesn't implement `PartialEq`, so we compare
    /// via the `Debug` repr — stable across releases of std
    /// and good enough to pin the contract.
    fn exit_code_repr(c: ExitCode) -> String {
        format!("{:?}", c)
    }

    fn sample_active_snapshot() -> ActiveSnapshot {
        ActiveSnapshot {
            kind: blit_core::generated::TransferKind::DelegatedPull as i32,
            peer: "10.0.0.5:443".to_string(),
            module: "mod-a".to_string(),
            path: "sub/dir".to_string(),
            start_unix_ms: 1_700_000_000_000,
        }
    }

    /// c-6 round 2 regression: terminal JSON shape on the
    /// stream-complete path must match the GetState-finished
    /// path. Pre-fix the stream path emitted only
    /// (state, transfer_id, bytes, files, duration_ms,
    /// tcp_fallback_used, ok), missing kind/peer/module/path/
    /// start_unix_ms that the snapshot path provides. Merging
    /// with the cached ActiveSnapshot restores parity.
    #[test]
    fn active_snapshot_to_finished_complete_carries_all_finished_fields() {
        let snap = sample_active_snapshot();
        let complete = blit_core::generated::TransferComplete {
            transfer_id: "t1-7".to_string(),
            bytes: 1024,
            files: 4,
            duration_ms: 1200,
            tcp_fallback_used: false,
        };
        let merged = snap.to_finished_complete(&complete);
        assert_eq!(merged.transfer_id, "t1-7");
        assert_eq!(merged.kind, snap.kind);
        assert_eq!(merged.peer, snap.peer);
        assert_eq!(merged.module, snap.module);
        assert_eq!(merged.path, snap.path);
        assert_eq!(merged.start_unix_ms, snap.start_unix_ms);
        assert_eq!(merged.duration_ms, 1200);
        assert_eq!(merged.bytes, 1024);
        assert_eq!(merged.files, 4);
        assert!(merged.ok);
        assert!(merged.error_message.is_empty());
    }

    /// Same parity check on the stream-error path.
    #[test]
    fn active_snapshot_to_finished_error_carries_all_finished_fields() {
        let snap = sample_active_snapshot();
        let err = blit_core::generated::TransferError {
            transfer_id: "t1-7".to_string(),
            message: "module not found".to_string(),
        };
        let merged = snap.to_finished_error(&err);
        assert_eq!(merged.transfer_id, "t1-7");
        assert_eq!(merged.kind, snap.kind);
        assert_eq!(merged.peer, snap.peer);
        assert_eq!(merged.module, snap.module);
        assert_eq!(merged.path, snap.path);
        assert_eq!(merged.start_unix_ms, snap.start_unix_ms);
        // duration_ms is computed from now - start; just
        // sanity-check it's non-negative (saturating sub).
        let _ = merged.duration_ms;
        assert_eq!(merged.bytes, 0);
        assert_eq!(merged.files, 0);
        assert!(!merged.ok);
        assert_eq!(merged.error_message, "module not found");
    }

    #[test]
    fn cancel_exit_code_maps_each_outcome_to_the_contract_code() {
        let cancelled = CancelJobOutcome::Cancelled {
            transfer_id: "t1".to_string(),
        };
        let not_found = CancelJobOutcome::NotFound {
            transfer_id: "t2".to_string(),
        };
        let unsupported = CancelJobOutcome::Unsupported {
            transfer_id: "t3".to_string(),
            message: "kind not cancellable".to_string(),
        };

        assert_eq!(
            exit_code_repr(cancel_exit_code(&cancelled)),
            exit_code_repr(ExitCode::SUCCESS),
            "Cancelled must exit 0",
        );
        assert_eq!(
            exit_code_repr(cancel_exit_code(&not_found)),
            exit_code_repr(ExitCode::from(1)),
            "NotFound must exit 1",
        );
        assert_eq!(
            exit_code_repr(cancel_exit_code(&unsupported)),
            exit_code_repr(ExitCode::from(2)),
            "Unsupported must exit 2",
        );
    }
}
