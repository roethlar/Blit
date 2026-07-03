use crate::enumeration::EnumeratedEntry;
use crate::fs_enum::{CopyJob, FileEntry};
use crate::transfer_plan::TransferTask;
use eyre::eyre;
use std::path::PathBuf;
use std::thread;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct TransferFacade;

#[derive(Debug)]
pub struct LocalTransferPlan {
    pub entries: Vec<EnumeratedEntry>,
    pub copy_jobs: Vec<CopyJob>,
}

#[derive(Debug)]
pub struct PullTransferPlan {
    pub files_to_fetch: Vec<FileEntry>,
    pub skipped_rel_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub enum PlannerEvent {
    Task(TransferTask),
    Progress {
        enumerated_files: usize,
        total_bytes: u64,
    },
}

#[derive(Debug)]
pub struct LocalPlanFinal {
    pub entries: Vec<EnumeratedEntry>,
    pub copy_jobs: Vec<CopyJob>,
    pub chunk_bytes: usize,
    pub total_bytes: u64,
    pub task_stats: PlanTaskStats,
}

#[derive(Debug, Clone, Default)]
pub struct PlanTaskStats {
    pub tar_shard_tasks: usize,
    pub tar_shard_files: usize,
    pub tar_shard_bytes: u64,
    pub raw_bundle_tasks: usize,
    pub raw_bundle_files: usize,
    pub raw_bundle_bytes: u64,
    pub large_tasks: usize,
    pub large_bytes: u64,
}

pub struct LocalPlanStream {
    pub events: UnboundedReceiver<PlannerEvent>,
    pub(crate) join_handle: thread::JoinHandle<Result<LocalPlanFinal, eyre::Report>>,
}

impl LocalPlanStream {
    pub fn into_parts(self) -> (UnboundedReceiver<PlannerEvent>, PlanJoinHandle) {
        (
            self.events,
            PlanJoinHandle {
                handle: self.join_handle,
            },
        )
    }
}

pub struct PlanJoinHandle {
    handle: thread::JoinHandle<Result<LocalPlanFinal, eyre::Report>>,
}

impl PlanJoinHandle {
    pub fn wait(self) -> Result<LocalPlanFinal, eyre::Report> {
        match self.handle.join() {
            Ok(res) => res,
            Err(err) => Err(eyre!("planner thread panicked: {:?}", err)),
        }
    }
}
