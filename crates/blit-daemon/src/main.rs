mod delegation_gate;
mod metrics;
mod runtime;
mod service;

use crate::metrics::TransferMetrics;
use crate::runtime::{load_runtime, DaemonArgs, DaemonRuntime};
use crate::service::{BlitServer, BlitService};
use blit_core::mdns::{self, AdvertiseOptions, MdnsAdvertiser};
use clap::Parser;
use eyre::Result;
use std::net::SocketAddr;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let args = DaemonArgs::parse();
    let runtime = load_runtime(&args)?;
    let DaemonRuntime {
        bind_host,
        port,
        modules,
        default_root,
        mdns,
        motd,
        warnings,
        server_checksums_enabled,
        delegation,
    } = runtime;

    for warning in &warnings {
        eprintln!("[warn] {warning}");
    }

    let addr: SocketAddr = format!("{}:{}", bind_host, port).parse()?;
    if let Some(motd) = motd {
        println!("motd: {motd}");
    }
    if let Some(root) = &default_root {
        eprintln!(
            "[info] default root export: {}{}",
            root.path.display(),
            if root.read_only { " (read-only)" } else { "" }
        );
    }

    let module_names: Vec<String> = modules.keys().cloned().collect();
    let mdns_guard: Option<MdnsAdvertiser> = if mdns.disabled {
        if let Some(name) = &mdns.name {
            eprintln!(
                "[info] mDNS advertising disabled; instance name '{}' ignored",
                name
            );
        }
        None
    } else {
        match mdns::advertise(AdvertiseOptions {
            port,
            instance_name: mdns.name.as_deref(),
            module_names: &module_names,
            // §3.2: surface delegation availability so `blit scan`
            // can show which daemons can act as remote→remote
            // delegation destinations without operators having to
            // probe each one.
            delegation_enabled: delegation.allow_delegated_pull,
        }) {
            Ok(handle) => {
                eprintln!(
                    "[info] mDNS advertising '{}' on port {}",
                    handle.instance_name(),
                    port
                );
                Some(handle)
            }
            Err(err) => {
                eprintln!("[warn] failed to advertise mDNS service: {err:?}");
                None
            }
        }
    };

    // Counters are off by default; the `--metrics` flag turns collection
    // on AND wires the daemon to emit a one-line stderr summary at the
    // end of each push / pull / pull_sync / delegated_pull / purge RPC
    // — operator-facing visibility under systemd or foreground without
    // needing the (future) TUI. See `metrics::TransferMetrics::log_completion`.
    let metrics = if args.metrics {
        eprintln!("[info] internal RPC metrics enabled; per-RPC summary lines on stderr");
        TransferMetrics::enabled()
    } else {
        TransferMetrics::disabled()
    };

    if delegation.allow_delegated_pull {
        eprintln!(
            "[info] delegated pull enabled ({} allowlist entries)",
            delegation.allowed_source_hosts.len()
        );
    }
    let service = BlitService::from_runtime(
        modules,
        default_root,
        args.force_grpc_data,
        server_checksums_enabled,
        metrics,
        delegation,
    );

    println!("blitd v2 listening on {}", addr);

    Server::builder()
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;

    drop(mdns_guard);

    Ok(())
}
