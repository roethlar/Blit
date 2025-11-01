use super::types::{PlanTaskStats, PlannerEvent};
use crate::transfer_plan::{PlanOptions, TransferTask};
use eyre::{eyre, Result};
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

pub(super) struct TaskAggregator {
    small_paths: Vec<PathBuf>,
    small_bytes: u64,
    small_count: u64,
    small_target: u64,
    small_count_target: usize,
    small_profile: bool,
    total_small_bytes: u64,
    medium_paths: Vec<PathBuf>,
    medium_bytes: u64,
    medium_target: u64,
    total_medium_bytes: u64,
    medium_max: u64,
    pub(super) chunk_bytes: usize,
    options: PlanOptions,
    pub(super) stats: PlanTaskStats,
    block_clone_same_volume: bool,
}

impl TaskAggregator {
    pub fn new(options: PlanOptions, block_clone_same_volume: bool) -> Self {
        let small_target = options.small_target.unwrap_or(8 * 1024 * 1024);
        let medium_target = options.medium_target.unwrap_or(128 * 1024 * 1024);
        let medium_max = (medium_target as f64 * 1.25) as u64;
        let chunk_bytes = 16 * 1024 * 1024;

        Self {
            small_paths: Vec::new(),
            small_bytes: 0,
            small_count: 0,
            small_target,
            small_count_target: options.small_count_target.unwrap_or(2048),
            small_profile: false,
            total_small_bytes: 0,
            medium_paths: Vec::new(),
            medium_bytes: 0,
            medium_target,
            total_medium_bytes: 0,
            medium_max,
            chunk_bytes,
            options,
            stats: PlanTaskStats::default(),
            block_clone_same_volume,
        }
    }

    pub fn push(
        &mut self,
        rel: PathBuf,
        size: u64,
        tx: &UnboundedSender<PlannerEvent>,
    ) -> Result<()> {
        if self.block_clone_same_volume {
            self.chunk_bytes = self.chunk_bytes.max(8 * 1024 * 1024);
            self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
            self.stats.raw_bundle_files = self.stats.raw_bundle_files.saturating_add(1);
            self.stats.raw_bundle_bytes = self.stats.raw_bundle_bytes.saturating_add(size);
            self.emit_task(tx, TransferTask::RawBundle(vec![rel]))?;
            return Ok(());
        }

        const LARGE_THRESHOLD: u64 = 256 * 1024 * 1024;
        if size >= LARGE_THRESHOLD {
            self.chunk_bytes = 32 * 1024 * 1024;
            self.stats.large_tasks = self.stats.large_tasks.saturating_add(1);
            self.stats.large_bytes = self.stats.large_bytes.saturating_add(size);
            self.emit_task(tx, TransferTask::Large { path: rel })?;
            return Ok(());
        }

        if size < 1_048_576 {
            self.small_paths.push(rel);
            self.small_bytes += size;
            self.small_count = self.small_count.saturating_add(1);
            self.total_small_bytes = self.total_small_bytes.saturating_add(size);
            self.promote_small_strategy();
            self.update_small_profile();
            self.chunk_bytes = self.chunk_bytes.max(self.small_target as usize);

            let reached_bytes = self.small_bytes >= self.small_target;
            let reached_count = self.small_paths.len() >= self.small_count_target;

            if (reached_bytes || reached_count) && !self.small_paths.is_empty() {
                let shard_bytes = self.small_bytes;
                let paths = std::mem::take(&mut self.small_paths);
                self.small_bytes = 0;
                self.stats.tar_shard_tasks = self.stats.tar_shard_tasks.saturating_add(1);
                self.stats.tar_shard_files = self.stats.tar_shard_files.saturating_add(paths.len());
                self.stats.tar_shard_bytes = self.stats.tar_shard_bytes.saturating_add(shard_bytes);
                self.emit_task(tx, TransferTask::TarShard(paths))?;
            }
            return Ok(());
        }

        self.medium_paths.push(rel);
        self.medium_bytes += size;
        self.total_medium_bytes = self.total_medium_bytes.saturating_add(size);
        self.promote_medium_strategy();
        if (self.medium_bytes >= self.medium_target && !self.medium_paths.is_empty())
            || self.medium_bytes > self.medium_max
        {
            let bundle_bytes = self.medium_bytes;
            let bundle = std::mem::take(&mut self.medium_paths);
            self.medium_bytes = 0;
            self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
            self.stats.raw_bundle_files = self.stats.raw_bundle_files.saturating_add(bundle.len());
            self.stats.raw_bundle_bytes = self.stats.raw_bundle_bytes.saturating_add(bundle_bytes);
            self.emit_task(tx, TransferTask::RawBundle(bundle))?;
        }

        Ok(())
    }

    pub fn flush_remaining(&mut self, tx: &UnboundedSender<PlannerEvent>) -> Result<()> {
        if !self.small_paths.is_empty() {
            let leftover_bytes = self.small_bytes;
            let paths = std::mem::take(&mut self.small_paths);
            self.small_bytes = 0;
            let should_tar = self.options.force_tar
                || self.small_profile
                || paths.len() >= self.small_count_target
                || leftover_bytes >= self.small_target;
            if should_tar {
                self.chunk_bytes = self.chunk_bytes.max(self.small_target as usize);
                self.stats.tar_shard_tasks = self.stats.tar_shard_tasks.saturating_add(1);
                self.stats.tar_shard_files = self.stats.tar_shard_files.saturating_add(paths.len());
                self.stats.tar_shard_bytes =
                    self.stats.tar_shard_bytes.saturating_add(leftover_bytes);
                self.emit_task(tx, TransferTask::TarShard(paths))?;
            } else {
                self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
                self.stats.raw_bundle_files =
                    self.stats.raw_bundle_files.saturating_add(paths.len());
                self.stats.raw_bundle_bytes =
                    self.stats.raw_bundle_bytes.saturating_add(leftover_bytes);
                self.emit_task(tx, TransferTask::RawBundle(paths))?;
            }
        }
        if !self.medium_paths.is_empty() {
            let bundle_bytes = self.medium_bytes;
            let bundle = std::mem::take(&mut self.medium_paths);
            self.medium_bytes = 0;
            self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
            self.stats.raw_bundle_files = self.stats.raw_bundle_files.saturating_add(bundle.len());
            self.stats.raw_bundle_bytes = self.stats.raw_bundle_bytes.saturating_add(bundle_bytes);
            self.emit_task(tx, TransferTask::RawBundle(bundle))?;
        }
        Ok(())
    }

    fn promote_small_strategy(&mut self) {
        if self.total_small_bytes >= 768 * 1024 * 1024 && self.small_target < 64 * 1024 * 1024 {
            self.small_target = 64 * 1024 * 1024;
        } else if self.total_small_bytes >= 256 * 1024 * 1024
            && self.small_target < 32 * 1024 * 1024
        {
            self.small_target = 32 * 1024 * 1024;
        }
        if self.total_small_bytes >= 1_000_000_000 {
            self.chunk_bytes = self.chunk_bytes.max(32 * 1024 * 1024);
        }
    }

    fn promote_medium_strategy(&mut self) {
        const PROMOTE_MEDIUM_THRESHOLD: u64 = 512 * 1024 * 1024;
        if self.total_medium_bytes >= PROMOTE_MEDIUM_THRESHOLD
            && self.medium_target < 384 * 1024 * 1024
        {
            self.medium_target = 384 * 1024 * 1024;
            self.medium_max = (self.medium_target as f64 * 1.25) as u64;
            self.chunk_bytes = self.chunk_bytes.max(32 * 1024 * 1024);
        }
    }

    fn update_small_profile(&mut self) {
        if self.small_profile {
            return;
        }
        if self.small_count >= 64 {
            let avg = if self.small_count == 0 {
                0
            } else {
                self.total_small_bytes / self.small_count
            };
            if avg <= 64 * 1024 {
                self.small_profile = true;
                self.small_count_target = 1024;
                self.chunk_bytes = self.chunk_bytes.max(self.small_target as usize);
            }
        }
    }

    fn emit_task(&self, tx: &UnboundedSender<PlannerEvent>, task: TransferTask) -> Result<()> {
        tx.send(PlannerEvent::Task(task))
            .map_err(|_| eyre!("planner consumer dropped"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer_plan::PlanOptions;

    fn drain_tasks(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlannerEvent>,
    ) -> Vec<TransferTask> {
        let mut out = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            if let PlannerEvent::Task(task) = evt {
                out.push(task);
            }
        }
        out
    }

    #[test]
    fn tiny_files_emit_tar_shards() {
        let options = PlanOptions::default();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut agg = TaskAggregator::new(options, false);
        for i in 0..1024 {
            let path = PathBuf::from(format!("file_{i}.bin"));
            agg.push(path, 16 * 1024, &tx).unwrap();
        }
        agg.flush_remaining(&tx).unwrap();
        let tasks = drain_tasks(&mut rx);
        assert!(!tasks.is_empty());
        for task in tasks {
            match task {
                TransferTask::TarShard(_) => {}
                other => panic!("expected tar shard, got {:?}", other),
            }
        }
    }
}
