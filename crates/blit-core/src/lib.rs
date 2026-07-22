pub mod buffer;
pub mod checksum;
pub mod config;
pub mod copy;
pub mod delete;
pub mod deletion;
pub mod dial;
pub mod enumeration;
pub mod fs_capability;
pub mod fs_enum;
pub mod logger;
pub mod manifest;
pub mod mdns;
pub mod mirror_planner;
pub mod path_posix;
pub mod path_safety;
pub mod perf_history;
pub mod perf_predictor;
pub mod remote;
pub mod stderr_log;
pub mod tar_stream;
pub mod transfer_plan;
pub mod transfer_session;
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
