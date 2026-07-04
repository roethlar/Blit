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

/// Planner tuning options shared across engines.
#[derive(Clone, Copy, Debug)]
pub struct PlanOptions {
    pub force_tar: bool,
    pub small_target: Option<u64>,
    pub small_count_target: Option<usize>,
    pub medium_target: Option<u64>,
}

impl PlanOptions {
    pub fn new() -> Self {
        Self {
            force_tar: false,
            small_target: None,
            small_count_target: None,
            medium_target: None,
        }
    }
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an adaptive transfer task queue from enumerated file entries.
///
/// The heuristics mirror the original `net_async::client::build_plan` logic so that
/// every mode (push, pull, local) can share the same task ordering. Wire
/// chunk sizing is NOT planned here — it is owned by the live
/// [`crate::engine::TransferDial`] (w2-2: this module's static 16/32 MiB
/// chunk ladder was dead policy — every remote path overrode it from the
/// dial and no consumer read the planned value).
pub fn build_plan(
    files: &[crate::fs_enum::FileEntry],
    rootsrc: &Path,
    options: PlanOptions,
) -> Vec<TransferTask> {
    let mut size_map: HashMap<PathBuf, u64> = HashMap::new();
    let mut small: Vec<PathBuf> = Vec::new();
    let mut medium: Vec<(PathBuf, u64)> = Vec::new();
    let mut total_medium_bytes: u64 = 0;
    let mut large_files: Vec<TransferTask> = Vec::new();
    let mut total_bytes: u128 = 0;
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
        total_bytes += e.size as u128;
        if e.size < 1_048_576 {
            // <1MB
            small.push(rel);
        } else if e.size < 256 * 1_048_576 {
            // <256MB
            medium.push((rel, e.size));
            total_medium_bytes = total_medium_bytes.saturating_add(e.size);
        } else {
            // Large: schedule as single large-file task; range/delta decided when sending
            large_files.push(TransferTask::Large { path: rel.clone() });
        }
    }
    // Shard small files into larger tars for multi-GB workloads
    small.sort_by_key(|p| p.as_os_str().len());

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

    // Tar shards only make sense for 2+ files (batching) — a single file
    // gains nothing from tar wrapping and breaks the empty-path case
    // produced by enumerating a file root directly.
    let use_tar = if options.force_tar {
        small_count >= 1
    } else if small_count < 2 {
        false
    } else {
        small_count >= 32 || avg_small_size <= 128 * 1024
    };

    if use_tar {
        let mut target_shard = options.small_target.unwrap_or(8 * 1024 * 1024);
        if total_small_bytes >= 768 * 1024 * 1024 {
            target_shard = target_shard.max(64 * 1024 * 1024);
        } else if total_small_bytes >= 256 * 1024 * 1024 {
            target_shard = target_shard.max(32 * 1024 * 1024);
        } else {
            target_shard = target_shard.max(4 * 1024 * 1024);
        }
        let mut count_target = options
            .small_count_target
            .unwrap_or(if small_count >= 2048 {
                2048
            } else if small_count >= 1024 {
                1024
            } else {
                256
            });
        count_target = count_target.clamp(128, 4096);

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
    let mut target_bundle: u64 = options.medium_target.unwrap_or(128 * 1024 * 1024);
    if total_medium_bytes >= 512 * 1024 * 1024 {
        target_bundle = target_bundle.max(384 * 1024 * 1024);
    } else if total_bytes > 1_000_000_000 {
        target_bundle = target_bundle.max(256 * 1024 * 1024);
    }
    // Slight spread to avoid synchronized boundaries
    let max_bundle: u64 = (target_bundle as f64 * 1.25) as u64;
    let mut cur_b: Vec<PathBuf> = Vec::new();
    let mut cur_sz: u64 = 0;
    for (p, sz) in medium.into_iter() {
        if !cur_b.is_empty() && (cur_sz >= target_bundle || cur_sz + sz > max_bundle) {
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
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_enum::FileEntry;

    fn entry(rel: &str, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from("/src").join(rel),
            size,
            is_directory: false,
        }
    }

    /// w2-2: the planner classifies and batches tasks; it no longer
    /// mints a chunk size (the dial owns wire chunking). These pins
    /// cover the classification tiers the deletion left untouched.
    #[test]
    fn classifies_small_medium_large_and_interleaves() {
        // Two tiny files (avg ≤ 128 KiB → tar-eligible), one medium,
        // one large.
        let files = vec![
            entry("small-a", 4 * 1024),
            entry("small-b", 8 * 1024),
            entry("medium", 8 * 1_048_576),
            entry("large", 300 * 1_048_576),
        ];
        let tasks = build_plan(&files, Path::new("/src"), PlanOptions::default());
        let mut large = 0;
        let mut shards = 0;
        let mut bundles = 0;
        for task in &tasks {
            match task {
                TransferTask::Large { path } => {
                    assert_eq!(path, Path::new("large"));
                    large += 1;
                }
                TransferTask::TarShard(paths) => {
                    shards += 1;
                    assert_eq!(paths.len(), 2, "both small files share one shard");
                }
                TransferTask::RawBundle(paths) => {
                    bundles += 1;
                    assert_eq!(paths, &[PathBuf::from("medium")]);
                }
            }
        }
        assert_eq!((large, shards, bundles), (1, 1, 1));
        // Interleave starts with the large task so no stream builds
        // tars while another idles.
        assert!(matches!(tasks[0], TransferTask::Large { .. }));
    }

    #[test]
    fn single_small_file_is_never_tar_wrapped() {
        let files = vec![entry("only", 1024)];
        let tasks = build_plan(&files, Path::new("/src"), PlanOptions::default());
        assert_eq!(tasks.len(), 1);
        assert!(
            matches!(&tasks[0], TransferTask::RawBundle(paths) if paths.len() == 1),
            "a lone small file gains nothing from tar wrapping"
        );
    }

    #[test]
    fn force_tar_wraps_even_a_single_file() {
        let files = vec![entry("only", 1024)];
        let options = PlanOptions {
            force_tar: true,
            ..PlanOptions::default()
        };
        let tasks = build_plan(&files, Path::new("/src"), options);
        assert_eq!(tasks.len(), 1);
        assert!(matches!(&tasks[0], TransferTask::TarShard(paths) if paths.len() == 1));
    }

    #[test]
    fn small_count_target_splits_shards() {
        // 300 tiny files with a count target of 128 must split into
        // ceil(300/128) = 3 shards (the clamp floor).
        let files: Vec<FileEntry> = (0..300).map(|i| entry(&format!("f{i:03}"), 1024)).collect();
        let options = PlanOptions {
            small_count_target: Some(1), // clamped up to 128
            ..PlanOptions::default()
        };
        let tasks = build_plan(&files, Path::new("/src"), options);
        let shard_sizes: Vec<usize> = tasks
            .iter()
            .map(|t| match t {
                TransferTask::TarShard(paths) => paths.len(),
                other => panic!("expected only tar shards, got {other:?}"),
            })
            .collect();
        assert_eq!(shard_sizes, vec![128, 128, 44]);
    }
}
