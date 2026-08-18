use crate::cli::ScanArgs;
use blit_core::scan::{self, MdnsDiscoveredService};
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
    /// cv-2: transfer-session protocol version the daemon advertised —
    /// the number that decides whether a transfer with it can open
    /// (D-2026-08-18-2). Emitted as null, never omitted, when the
    /// daemon advertised none: unknown, not a mismatch.
    contract_version: Option<u32>,
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
                contract_version: s.contract_version(),
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
        for line in service_lines(service) {
            println!("{line}");
        }
    }

    Ok(())
}

/// The text-mode block for one discovered daemon, one line per entry.
/// Pure so the rendering — notably the unknown-protocol case — is
/// pinned by unit tests without an mDNS network.
fn service_lines(service: &MdnsDiscoveredService) -> Vec<String> {
    let mut lines = vec![format!("- {}", service.instance_name)];

    let host = if let Some(addr) = service.addresses.first() {
        addr.to_string()
    } else {
        service.hostname.trim_end_matches('.').to_string()
    };
    let endpoint = if service.port == blit_core::remote::endpoint::RemoteEndpoint::DEFAULT_PORT {
        format!("{}://", host)
    } else {
        format!("{}:{}://", host, service.port)
    };
    lines.push(format!("  Endpoint: {}", endpoint));

    if service.addresses.len() > 1 {
        let addr_list = service
            .addresses
            .iter()
            .map(|addr| addr.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  Addresses: {}", addr_list));
    }

    if let Some(version) = service.properties.get("version") {
        lines.push(format!("  Version: {}", version));
    }
    // cv-2: the protocol version is what decides whether a transfer
    // with this daemon can open (D-2026-08-18-2). A daemon predating
    // cv-2 advertises none — that is UNKNOWN, never a mismatch, and
    // never an error.
    lines.push(match service.contract_version() {
        Some(contract) => format!("  Protocol: v{}", contract),
        None => "  Protocol: unknown (not advertised)".to_string(),
    });
    let modules = service.modules();
    let module_count = service.module_count();
    match (modules.is_empty(), module_count) {
        (false, Some(count)) if (count as usize) > modules.len() => {
            // Module list truncated; show count + visible names.
            lines.push(format!("  Modules ({}): {} ...", count, modules.join(", ")));
        }
        (false, _) => {
            lines.push(format!("  Modules: {}", modules.join(", ")));
        }
        (true, Some(count)) if count > 0 => {
            // Daemon reports modules exist but TXT didn't carry
            // the list (compact daemon variant).
            lines.push(format!("  Modules: {} (names not advertised)", count));
        }
        _ => {}
    }
    if let Some(true) = service.delegation_enabled() {
        lines.push("  Delegation: accepts DelegatedPull".to_string());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    fn service(props: &[(&str, &str)]) -> MdnsDiscoveredService {
        MdnsDiscoveredService {
            fullname: "magneto._blit._tcp.local.".into(),
            instance_name: "magneto".into(),
            hostname: "magneto.local.".into(),
            port: 9031,
            addresses: vec![Ipv4Addr::new(192, 168, 1, 10)],
            properties: props
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<HashMap<_, _>>(),
        }
    }

    #[test]
    fn advertised_protocol_version_is_shown() {
        let lines = service_lines(&service(&[("version", "0.1.2"), ("contract", "6")]));
        assert!(
            lines.contains(&"  Protocol: v6".to_string()),
            "protocol row missing from {lines:?}"
        );
    }

    #[test]
    fn absent_protocol_version_renders_as_unknown_not_an_error() {
        let lines = service_lines(&service(&[("version", "0.1.2")]));
        assert!(
            lines.contains(&"  Protocol: unknown (not advertised)".to_string()),
            "pre-cv-2 daemon must render as unknown, got {lines:?}"
        );
        assert!(
            !lines.iter().any(|l| {
                let l = l.to_ascii_lowercase();
                l.contains("error") || l.contains("mismatch")
            }),
            "unknown must never read as a fault: {lines:?}"
        );
    }

    #[test]
    fn unparseable_protocol_version_renders_as_unknown() {
        let lines = service_lines(&service(&[("contract", "not-a-number")]));
        assert!(lines.contains(&"  Protocol: unknown (not advertised)".to_string()));
    }
}
