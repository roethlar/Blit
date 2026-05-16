use crate::cli::ScanArgs;
use blit_app::scan;
use eyre::Result;
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
    /// §3.2: total module count as advertised by the daemon.
    /// Distinct from `modules.len()` because `modules` may be
    /// truncated for daemons exporting many modules.
    #[serde(skip_serializing_if = "Option::is_none")]
    module_count: Option<u32>,
    /// §3.2: whether the daemon accepts DelegatedPull requests
    /// (remote→remote initiator). Absent for pre-§3.2 daemons.
    #[serde(skip_serializing_if = "Option::is_none")]
    delegation_enabled: Option<bool>,
}

pub async fn run_scan(args: ScanArgs) -> Result<()> {
    let json = args.json;
    let wait_secs = args.wait;
    let services = scan::discover(Duration::from_secs(wait_secs)).await?;

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
                module_count: s.module_count(),
                delegation_enabled: s.delegation_enabled(),
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
        let module_count = service.module_count();
        match (modules.is_empty(), module_count) {
            (false, Some(count)) if (count as usize) > modules.len() => {
                // Module list truncated; show count + visible names.
                println!("  Modules ({}): {} ...", count, modules.join(", "));
            }
            (false, _) => {
                println!("  Modules: {}", modules.join(", "));
            }
            (true, Some(count)) if count > 0 => {
                // Daemon reports modules exist but TXT didn't carry
                // the list (compact daemon variant).
                println!("  Modules: {} (names not advertised)", count);
            }
            _ => {}
        }
        if let Some(true) = service.delegation_enabled() {
            println!("  Delegation: accepts DelegatedPull");
        }
    }

    Ok(())
}
