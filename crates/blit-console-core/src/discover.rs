//! Discovery: turn mDNS results into registered daemon endpoints.
//!
//! The scan itself is blit-app's [`blit_app::scan::discover`] — the
//! same typed wrapper `blit scan` and the TUI's F1 pane use, which
//! runs `blit_core::mdns::discover` on a blocking thread. This module
//! owns the mapping from [`MdnsDiscoveredService`] to the console's
//! [`DaemonEndpoint`]; the merge of a fresh snapshot into the model
//! lives in [`crate::model`] (upsert-by-address, see there).

use crate::endpoint::DaemonEndpoint;
use blit_core::mdns::MdnsDiscoveredService;
use std::fmt;
use std::time::Duration;

/// How long one discovery scan listens for mDNS announcements.
/// Matches the `blit scan --wait` default (`crates/blit-cli/src/cli.rs`).
pub const DEFAULT_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(2);

/// Why a discovery scan failed.
#[derive(Debug)]
pub struct DiscoveryError {
    pub reason: String,
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "daemon discovery failed: {}", self.reason)
    }
}

impl std::error::Error for DiscoveryError {}

/// Map one discovered service to a dialable daemon endpoint:
/// `address` is the first advertised IPv4 address plus the daemon
/// port (the form [`blit_core::remote::endpoint::RemoteEndpoint::parse`]
/// accepts), `name` is the mDNS instance name shown in the sidebar.
///
/// Returns `None` when the service advertised no IPv4 address — the
/// same skip the TUI's `DaemonsState::endpoint_for_row` makes for
/// address-less rows (`crates/blit-tui/src/daemons.rs`).
pub fn endpoint_from_service(service: &MdnsDiscoveredService) -> Option<DaemonEndpoint> {
    let address = service.addresses.first()?;
    Some(DaemonEndpoint {
        address: format!("{}:{}", address, service.port),
        name: service.instance_name.clone(),
    })
}

/// Map a whole discovery snapshot, skipping services without an
/// address. Input order is preserved — `blit_core::mdns::discover`
/// already sorts by instance name.
pub fn endpoints_from_services(services: &[MdnsDiscoveredService]) -> Vec<DaemonEndpoint> {
    services.iter().filter_map(endpoint_from_service).collect()
}

/// Scan the LAN for blit daemons and return them as daemon endpoints.
/// Async through blit-app's spawn-blocking wrapper — no blocking call
/// runs on the caller's runtime. This is what a host executes for
/// [`crate::Effect::Discover`], feeding the result back as
/// [`crate::Msg::DiscoveryLoaded`] / [`crate::Msg::DiscoveryFailed`].
pub async fn discover_daemons(timeout: Duration) -> Result<Vec<DaemonEndpoint>, DiscoveryError> {
    let services = blit_app::scan::discover(timeout)
        .await
        .map_err(|err| DiscoveryError {
            reason: err.to_string(),
        })?;
    Ok(endpoints_from_services(&services))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    fn service(name: &str, addresses: &[Ipv4Addr], port: u16) -> MdnsDiscoveredService {
        MdnsDiscoveredService {
            fullname: format!("{name}._blit._tcp.local."),
            instance_name: name.to_string(),
            hostname: format!("{name}.local."),
            port,
            addresses: addresses.to_vec(),
            properties: HashMap::new(),
        }
    }

    #[test]
    fn service_maps_to_first_address_and_port() {
        let svc = service(
            "magneto",
            &[Ipv4Addr::new(192, 168, 1, 10), Ipv4Addr::new(10, 0, 0, 2)],
            9031,
        );
        let endpoint = endpoint_from_service(&svc).unwrap();
        assert_eq!(endpoint.address, "192.168.1.10:9031");
        assert_eq!(endpoint.name, "magneto");
    }

    #[test]
    fn service_without_addresses_is_skipped() {
        let svc = service("ghost", &[], 9031);
        assert!(endpoint_from_service(&svc).is_none());
    }

    #[test]
    fn snapshot_skips_addressless_and_preserves_order() {
        let services = vec![
            service("alpha", &[Ipv4Addr::new(192, 168, 1, 10)], 9031),
            service("ghost", &[], 9031),
            service("bravo", &[Ipv4Addr::new(192, 168, 1, 20)], 9833),
        ];
        let endpoints = endpoints_from_services(&services);
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].name, "alpha");
        assert_eq!(endpoints[0].address, "192.168.1.10:9031");
        assert_eq!(endpoints[1].name, "bravo");
        assert_eq!(endpoints[1].address, "192.168.1.20:9833");
    }
}
