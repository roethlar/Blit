//! The ONE brain of the Blit Console (`docs/plan/BLIT_CONSOLE.md`
//! §2): endpoint model, browse state, and an Elm-ish message/update
//! loop shared by the GUI and TUI faces. Zero UI dependencies;
//! browsing routes through the typed blit-app admin APIs — never
//! stdout parsing, never spawning the CLI.
//!
//! This slice (C1, second cut) covers daemon discovery (mDNS via
//! blit-app's scan wrapper) and daemon browse (module list at the
//! root, `admin::ls::list_remote` below it) alongside slice 1's
//! endpoint model, local browse, and update loop. The transfer
//! composer and the task registry land in later slices.

pub mod browse;
pub mod discover;
pub mod endpoint;
pub mod model;

pub use browse::{browse, BrowseError, Listing};
pub use discover::{
    discover_daemons, endpoint_from_service, endpoints_from_services, DiscoveryError,
    DEFAULT_DISCOVERY_TIMEOUT,
};
pub use endpoint::{DaemonEndpoint, Endpoint, EndpointId};
pub use model::{update, Effect, Model, Msg};

pub use blit_app::admin::ls::DirEntry;
