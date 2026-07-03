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

        // Build a usable endpoint URL, preferring IP address over hostname
        let host = if let Some(addr) = service.addresses.first() {
            addr.to_string()
        } else {
            // Strip trailing dot from FQDN if present
            service.hostname.trim_end_matches('.').to_string()
        };
        let endpoint = if service.port == 9031 {
            format!("{}://", host)
        } else {
            format!("{}:{}://", host, service.port)
        };
        println!("  Endpoint: {}", endpoint);

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
