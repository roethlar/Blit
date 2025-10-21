use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Adaptive transfer task classification shared across push, pull, and local engines.
#[derive(Clone, Debug)]
pub enum TransferTask {
    TarShard(Vec<PathBuf>),
    /// Bundle of medium files to send back-to-back in a single worker turn.
    RawBundle(Vec<PathBuf>),
    /// Large single file; delta/range logic decides stripes internally.
    Large {
        path: PathBuf,
    },
}

/// Planned work queue along with the preferred chunk size for streaming.
#[derive(Clone, Debug)]
pub struct Plan {
    pub tasks: Vec<TransferTask>,
    pub chunk_bytes: usize,
}

/// Planner tuning options shared across engines.
#[derive(Clone, Copy, Debug)]
pub struct PlanOptions {
    pub force_tar: bool,
}

impl PlanOptions {
    pub fn new() -> Self {
        Self { force_tar: false }
    }
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an adaptive transfer plan from enumerated file entries.
///
/// The heuristics mirror the original `net_async::client::build_plan` logic so that
/// every mode (push, pull, local) can share the same task ordering and chunk sizing.
pub fn build_plan(
    files: &[crate::fs_enum::FileEntry],
    rootsrc: &Path,
    options: PlanOptions,
) -> Plan {
    let mut size_map: HashMap<PathBuf, u64> = HashMap::new();
    let mut small: Vec<PathBuf> = Vec::new();
    let mut medium: Vec<(PathBuf, u64)> = Vec::new();
    let mut large_files: Vec<TransferTask> = Vec::new();
    // Kickoff histogram (bytes per bin)
    let mut bins_bytes = [0u128; 6];
    let mut bins_count = [0u64; 6];
    for e in files {
        if e.is_directory {
            continue;
        }
        let rel = e
            .path
            .strip_prefix(rootsrc)
            .unwrap_or(&e.path)
            .to_path_buf();
        size_map.insert(rel.clone(), e.size);
        if e.size < 64 * 1024 {
            // <64KiB
            small.push(rel);
            bins_bytes[0] += e.size as u128;
            bins_count[0] += 1;
        } else if e.size < 1_048_576 {
            // 64KiB–1MB
            small.push(rel);
            bins_bytes[1] += e.size as u128;
            bins_count[1] += 1;
        } else if e.size < 256 * 1_048_576 {
            // <256MB
            medium.push((rel, e.size));
            if e.size < 32 * 1_048_576 {
                bins_bytes[2] += e.size as u128;
                bins_count[2] += 1;
            } else {
                bins_bytes[3] += e.size as u128;
                bins_count[3] += 1;
            }
        } else {
            // Large: schedule as single large-file task; range/delta decided when sending
            large_files.push(TransferTask::Large { path: rel.clone() });
            if e.size < 2 * 1024 * 1024 * 1024 {
                bins_bytes[4] += e.size as u128;
                bins_count[4] += 1;
            } else {
                bins_bytes[5] += e.size as u128;
                bins_count[5] += 1;
            }
        }
    }
    // Shard small files into larger tars for multi-GB workloads
    small.sort_by_key(|p| p.as_os_str().len());
    let total_bytes: u128 = bins_bytes.iter().copied().sum();

    let mut small_tasks: Vec<TransferTask> = Vec::new();
    let small_count = small.len();
    let total_small_bytes: u64 = small.iter().fold(0u64, |acc, p| {
        acc.saturating_add(*size_map.get(p).unwrap_or(&(64 * 1024)))
    });
    let avg_small_size = if small_count == 0 {
        0
    } else {
        total_small_bytes / small_count as u64
    };

    let use_tar = if options.force_tar {
        true
    } else if small_count == 0 {
        false
    } else {
        small_count >= 32 || avg_small_size <= 128 * 1024
    };

    if use_tar {
        let target_shard = if total_small_bytes >= 768 * 1024 * 1024 {
            64 * 1024 * 1024
        } else if total_small_bytes >= 256 * 1024 * 1024 {
            32 * 1024 * 1024
        } else {
            8 * 1024 * 1024
        };
        let count_target = if small_count >= 2048 {
            2048
        } else if small_count >= 1024 {
            1024
        } else {
            256
        };

        let mut cur: Vec<PathBuf> = Vec::new();
        let mut cur_bytes: u64 = 0;
        for p in small.iter() {
            let size = *size_map.get(p).unwrap_or(&(64 * 1024));
            let would_exceed = cur_bytes + size > target_shard;
            let reached_count = cur.len() >= count_target;
            if !cur.is_empty() && (would_exceed || reached_count) {
                small_tasks.push(TransferTask::TarShard(std::mem::take(&mut cur)));
                cur_bytes = 0;
            }
            cur.push(p.clone());
            cur_bytes += size;
        }
        if !cur.is_empty() {
            small_tasks.push(TransferTask::TarShard(cur));
        }
    } else {
        for p in small.iter() {
            small_tasks.push(TransferTask::RawBundle(vec![p.clone()]));
        }
    }

    let mut medium_tasks: Vec<TransferTask> = Vec::new();
    let target_bundle: u64 = if total_bytes > 1_000_000_000 {
        256 * 1024 * 1024 // 256 MiB for large manifests
    } else {
        128 * 1024 * 1024 // 128 MiB default
    };
    // Slight spread to avoid synchronized boundaries
    let max_bundle: u64 = (target_bundle as f64 * 1.25) as u64;
    let mut cur_b: Vec<PathBuf> = Vec::new();
    let mut cur_sz: u64 = 0;
    for (p, sz) in medium.into_iter() {
        if cur_sz >= target_bundle && !cur_b.is_empty() {
            medium_tasks.push(TransferTask::RawBundle(std::mem::take(&mut cur_b)));
            cur_sz = 0;
        }
        if cur_sz + sz > max_bundle && !cur_b.is_empty() {
            medium_tasks.push(TransferTask::RawBundle(std::mem::take(&mut cur_b)));
            cur_sz = 0;
        }
        cur_b.push(p);
        cur_sz += sz;
    }
    if !cur_b.is_empty() {
        medium_tasks.push(TransferTask::RawBundle(cur_b));
    }

    // Interleave tasks from large, shard, bundle to avoid all streams building tars at once
    let mut tasks = Vec::new();
    let mut i_l = 0usize;
    let mut i_s = 0usize;
    let mut i_m = 0usize;
    while i_l < large_files.len() || i_s < small_tasks.len() || i_m < medium_tasks.len() {
        if i_l < large_files.len() {
            tasks.push(large_files[i_l].clone());
            i_l += 1;
        }
        if i_s < small_tasks.len() {
            tasks.push(small_tasks[i_s].clone());
            i_s += 1;
        }
        if i_m < medium_tasks.len() {
            tasks.push(medium_tasks[i_m].clone());
            i_m += 1;
        }
    }
    // Choose chunk size: larger for big transfers dominated by large files
    let large_bytes = bins_bytes[4] + bins_bytes[5];
    let chunk_bytes = if total_bytes > 1_000_000_000 {
        32 * 1024 * 1024 // 32 MiB for transfers >1GB
    } else if large_bytes * 100 / total_bytes.max(1) >= 50 {
        32 * 1024 * 1024 // 32 MiB when dominated by large files
    } else {
        16 * 1024 * 1024 // 16 MiB default
    };
    Plan { tasks, chunk_bytes }
}

/// Convert Plan to daemon format (u8 type code, paths)
/// Used by server pull mode for backward compatibility
pub fn plan_to_daemon_format(plan: &Plan) -> Vec<(u8, Vec<PathBuf>)> {
    plan.tasks
        .iter()
        .map(|task| match task {
            TransferTask::TarShard(paths) => (1u8, paths.clone()),
            TransferTask::RawBundle(paths) => (2u8, paths.clone()),
            TransferTask::Large { path } => (3u8, vec![path.clone()]),
        })
        .collect()
}
