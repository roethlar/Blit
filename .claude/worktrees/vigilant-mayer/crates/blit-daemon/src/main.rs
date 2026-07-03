mod runtime;
mod service;

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
            "[info] default root export: {}{}{}",
            root.path.display(),
            if root.read_only { " (read-only)" } else { "" },
            if root.use_chroot { " [chroot]" } else { "" }
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

    let service = BlitService::from_runtime(modules, default_root, args.force_grpc_data, server_checksums_enabled);

    println!("blitd v2 listening on {}", addr);

    Server::builder()
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;

    drop(mdns_guard);

    Ok(())
}
