use crate::cli::{JobsCommand, JobsListArgs};
use blit_app::admin::jobs;
use blit_core::generated::DaemonState;
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};

pub async fn run_jobs(command: JobsCommand) -> Result<()> {
    match command {
        JobsCommand::List(args) => run_jobs_list(args).await,
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
}
