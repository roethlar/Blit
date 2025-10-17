pub mod auto_tune;
pub mod buffer;
pub mod checksum;
pub mod copy;
pub mod delete;
pub mod enumeration;
pub mod fs_capability;
pub mod fs_enum;
pub mod local_worker;
pub mod logger;
pub mod mirror_planner;
pub mod orchestrator;
pub mod perf_history;
pub mod perf_predictor;
pub mod tar_stream;
pub mod transfer_engine;
pub mod transfer_facade;
pub mod transfer_plan;
#[cfg(windows)]
pub mod win_fs;
pub mod zero_copy;

#[derive(Clone)]
pub struct CopyConfig {
    pub workers: usize,
    pub preserve_times: bool,
    pub dry_run: bool,
    pub checksum: Option<crate::checksum::ChecksumType>,
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            workers: num_cpus::get().max(1),
            preserve_times: true,
            dry_run: false,
            checksum: None,
        }
    }
}

pub mod generated {
    tonic::include_proto!("blit.v2");
}
