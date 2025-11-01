use crate::cli::ScanArgs;
use blit_core::mdns;
use eyre::{Context, Result};
use std::time::Duration;

pub async fn run_scan(args: &ScanArgs) -> Result<()> {
    let wait = Duration::from_secs(args.wait);
    let services = tokio::task::spawn_blocking(move || mdns::discover(wait))
        .await
        .context("mDNS discovery task panicked")??;

    if services.is_empty() {
        println!("No blit daemons discovered within {} second(s).", args.wait);
        return Ok(());
    }

    println!("Discovered {} daemon(s):", services.len());
    for service in &services {
        println!("- {}", service.instance_name);
        println!("  Host: {}:{}", service.hostname, service.port);
        if !service.addresses.is_empty() {
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
