//! The ONE brain of the Blit Console (`docs/plan/BLIT_CONSOLE.md`
//! §2): endpoint model, browse state, and an Elm-ish message/update
//! loop shared by the GUI and TUI faces. Zero UI dependencies;
//! browsing routes through the typed blit-app admin APIs — never
//! stdout parsing, never spawning the CLI.
//!
//! This slice (C1, first cut) covers the endpoint model, local
//! filesystem browse, and the update skeleton. Daemon browse is a
//! typed stub; mDNS discovery, the transfer composer, and the task
//! registry land in later slices.

pub mod browse;
pub mod endpoint;
pub mod model;

pub use browse::{browse, BrowseError, Listing};
pub use endpoint::{DaemonEndpoint, Endpoint, EndpointId};
pub use model::{update, Effect, Model, Msg};

pub use blit_app::admin::ls::DirEntry;
