pub mod admin;
pub mod browse;
pub mod buffer;
pub mod check;
pub mod checksum;
pub mod client;
pub mod config;
pub mod copy;
pub mod deletion;
pub mod diagnostics;
pub mod dial;
pub mod discover;
pub mod display;
pub mod endpoint;
pub mod endpoints;
pub mod enumeration;
pub mod fs_capability;
pub mod fs_enum;
pub mod manifest;
pub mod mdns;
pub mod mirror_planner;
pub mod model;
pub mod path_posix;
pub mod path_safety;
pub mod perf_history;
pub mod profile;
pub mod remote;
pub mod scan;
pub mod seed_store;
pub mod stderr_log;
pub mod transfer_plan;
pub mod transfer_session;
pub mod transfers;
#[cfg(windows)]
pub mod win_fs;
pub(crate) mod windows_metadata;
pub mod wire_metadata;
pub mod zero_copy;

#[cfg(test)]
#[path = "../build_identity.rs"]
mod build_identity_test_support;

pub mod generated {
    tonic::include_proto!("blit.v2");
}
