use crate::cli::ScanArgs;
use blit_core::mdns;
use eyre::{Context, Result};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct ScanEntryJson {
    instance_name: String,
    host: String,
    port: u16,
    addresses: Vec<String>,
    version: Option<String>,
    modules: Vec<String>,
}

pub async fn run_scan(args: ScanArgs) -> Result<()> {
    let json = args.json;
    let wait_secs = args.wait;
    let wait = Duration::from_secs(wait_secs);
    let services = tokio::task::spawn_blocking(move || mdns::discover(wait))
        .await
        .context("mDNS discovery task panicked")??;

    if json {
        let entries: Vec<ScanEntryJson> = services
            .iter()
            .map(|s| ScanEntryJson {
                instance_name: s.instance_name.clone(),
                host: s.hostname.trim_end_matches('.').to_string(),
                port: s.port,
                addresses: s.addresses.iter().map(|a| a.to_string()).collect(),
                version: s.properties.get("version").cloned(),
                modules: s.modules(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if services.is_empty() {
        println!("No blit daemons discovered within {} second(s).", wait_secs);
        return Ok(());
    }

    println!("Discovered {} daemon(s):", services.len());
    for service in &services {
        println!("- {}", service.instance_name);

        let host = if let Some(addr) = service.addresses.first() {
            addr.to_string()
        } else {
            service.hostname.trim_end_matches('.').to_string()
        };
        let endpoint = if service.port == 9031 {
            format!("{}://", host)
        } else {
            format!("{}:{}://", host, service.port)
        };
        println!("  Endpoint: {}", endpoint);

        if service.addresses.len() > 1 {
            let addr_list = service
                .addresses
                .iter()
                .map(|addr| addr.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("  Addresses: {}", addr_list);
        }

        if let Some(version) = service.properties.get("version") {
            println!("  Version: {}", version);
        }
        let modules = service.modules();
        if !modules.is_empty() {
            println!("  Modules: {}", modules.join(", "));
        }
    }

    Ok(())
}
