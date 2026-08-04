//! Endpoint model: the things the console browses — the local
//! filesystem and remote daemons. In-memory only in this slice; mDNS
//! discovery and manual-add plumbing land in a later slice
//! (`docs/plan/BLIT_CONSOLE.md` §4, C1).

use serde::{Deserialize, Serialize};

/// In-memory identity for an endpoint, assigned by [`crate::Model`]
/// as endpoints are registered. Not persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointId(pub u64);

/// A remote blit daemon: the address the console dials plus the
/// human-facing name shown in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonEndpoint {
    pub address: String,
    pub name: String,
}

/// One browsable endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Endpoint {
    /// The local filesystem.
    Local,
    /// A remote daemon (browse stubbed as unsupported in this slice).
    Daemon(DaemonEndpoint),
}

impl Endpoint {
    /// The sidebar label for this endpoint.
    pub fn display_name(&self) -> &str {
        match self {
            Endpoint::Local => "Local",
            Endpoint::Daemon(daemon) => &daemon.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names() {
        assert_eq!(Endpoint::Local.display_name(), "Local");
        let daemon = Endpoint::Daemon(DaemonEndpoint {
            address: "magneto:9833".to_string(),
            name: "magneto".to_string(),
        });
        assert_eq!(daemon.display_name(), "magneto");
    }
}
