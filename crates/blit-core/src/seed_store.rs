//! Settled-dial seed store (ph-3 of `docs/plan/PERF_HISTORY_PLANNING.md`).
//!
//! At session close, alongside the perf-history record, the closing end
//! persists the dial values its controllers actually **settled** on,
//! keyed by `(route, peer_key, workload class)`. The next session that
//! opens with a matching key reads the seed to warm-start its dials
//! (ph-4 checkers, ph-5 session workers). This replaces the retired
//! gradient-descent predictor (R1, D-2026-08-20-2): settled values are
//! ground truth from a real run; coefficients were a model of one.
//!
//! ## Persistence gate
//!
//! A seed is written only when ALL of these hold (plan §Design, review
//! gate: "never from a run that was still probing when it ended"):
//!
//! - the dial's own controller reports **settled** — the caller passes
//!   `None` for a dial that was still probing (or pinned by a
//!   diagnostic flag, which discovered nothing);
//! - the run moved at least [`SEED_MIN_FILES`] files — a run too small
//!   to exercise the dial cannot teach it;
//! - the run was a real transfer — dry runs and measurement lanes never
//!   reach the write (the caller gates on its `RunKind`).
//!
//! ## Keying
//!
//! `topology|local_role|initiator|peer_key|workload_class`, the same
//! axes as the v3 [`crate::perf_history::PerformanceRecord`] plus a
//! coarse workload class ([`WorkloadClass`]): two buckets split at
//! 1 MiB mean file size, because the dials this store seeds react to
//! file-count-vs-byte balance, not exact sizes, and over-fine keys
//! would make seeds never hit (plan §Risks). A route with no
//! `peer_key` is **never** written: a shared `unknown` bucket would
//! blend unrelated destinations into one seed.
//!
//! ## File
//!
//! `perf_seeds.json` in the same store directory as
//! `perf_local.jsonl` (operator config dir, or the daemon's
//! `$STATE_DIRECTORY`), schema-versioned, capped at
//! [`MAX_SEED_KEYS`] entries (oldest evicted), written via
//! temp-file + atomic rename so a torn write can never corrupt the
//! map. Plain text on purpose — on-device and debuggable, like the
//! history file.

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::perf_history::{HistoryStore, RouteTag};

/// Minimum files a run must have moved before it may teach a seed.
pub const SEED_MIN_FILES: usize = 100;

/// Entry cap; the oldest `updated_ms` is evicted past this. Keys are
/// coarse, so a normal operator sits far below it.
pub const MAX_SEED_KEYS: usize = 256;

/// Current on-disk schema version.
pub const SEED_SCHEMA_VERSION: u32 = 1;

const SEEDS_FILE: &str = "perf_seeds.json";

/// Coarse workload class — the third key axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadClass {
    /// Mean file size under 1 MiB: file-count-dominated runs.
    SmallFiles,
    /// Mean file size at or above 1 MiB: byte-dominated runs.
    LargeFiles,
}

impl WorkloadClass {
    /// Classify a finished run. `files` must be non-zero (the
    /// min-files gate already guarantees it at the only call sites).
    pub fn classify(files: usize, bytes: u64) -> Self {
        const MIB: u64 = 1024 * 1024;
        if files > 0 && bytes / files as u64 >= MIB {
            Self::LargeFiles
        } else {
            Self::SmallFiles
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::SmallFiles => "small",
            Self::LargeFiles => "large",
        }
    }
}

/// The dial values one run settled on. `None` slots are "this run
/// discovered nothing for that dial" and never overwrite a stored
/// value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettledDials {
    /// Destination-comparison concurrency the checker ladder settled
    /// on (`AdaptiveCheckers`); `None` when still probing or pinned.
    pub checkers: Option<u32>,
    /// Session worker / stream count the transfer dial converged to;
    /// `None` when it never settled. Written by ph-5's wiring.
    pub workers: Option<u32>,
}

impl SettledDials {
    pub fn is_empty(&self) -> bool {
        self.checkers.is_none() && self.workers.is_none()
    }
}

/// One persisted seed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeedEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkers: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workers: Option<u32>,
    /// Settled runs folded into this entry.
    pub runs: u32,
    /// Unix millis of the last update; the eviction ordering.
    pub updated_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SeedFile {
    schema_version: u32,
    /// Key → seed. `BTreeMap` for deterministic serialization.
    seeds: BTreeMap<String, SeedEntry>,
}

impl Default for SeedFile {
    fn default() -> Self {
        Self {
            schema_version: SEED_SCHEMA_VERSION,
            seeds: BTreeMap::new(),
        }
    }
}

/// The seed store for one state directory. Construction mirrors
/// [`HistoryStore`]: `user()`, `daemon()`, or `at_dir()` for tests.
#[derive(Debug, Clone)]
pub struct SeedStore {
    dir: PathBuf,
}

impl SeedStore {
    pub fn user() -> Result<Self> {
        Ok(Self::for_history(&HistoryStore::user()?))
    }

    pub fn daemon() -> Result<Self> {
        Ok(Self::for_history(&HistoryStore::daemon()?))
    }

    /// Beside the given history store's file — the seed always lives
    /// with the records that taught it.
    pub fn for_history(store: &HistoryStore) -> Self {
        Self {
            dir: store.dir().to_path_buf(),
        }
    }

    pub fn at_dir(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(SEEDS_FILE)
    }

    /// The seed key for a route. `None` when the route has no
    /// `peer_key` — such runs are recorded in history but must never
    /// teach a seed (no shared `unknown` bucket).
    pub fn key(route: &RouteTag, class: WorkloadClass) -> Option<String> {
        let peer = route.peer_key.as_deref()?;
        Some(format!(
            "{}|{}|{}|{}|{}",
            route.topology.label(),
            route.local_role.label(),
            route.initiator.label(),
            peer,
            class.label(),
        ))
    }

    /// Fold one settled run into the store. Applies the persistence
    /// gate; a call that fails any gate is a silent no-op (recording
    /// must never fail a transfer, matching the history writer).
    ///
    /// Returns `true` when a seed was written (for tests and logging).
    pub fn record_settled(
        &self,
        route: &RouteTag,
        dials: SettledDials,
        files: usize,
        bytes: u64,
    ) -> Result<bool> {
        if dials.is_empty() || files < SEED_MIN_FILES {
            return Ok(false);
        }
        let class = WorkloadClass::classify(files, bytes);
        let Some(key) = Self::key(route, class) else {
            return Ok(false);
        };

        let mut file = self.load()?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let entry = file.seeds.entry(key).or_insert(SeedEntry {
            checkers: None,
            workers: None,
            runs: 0,
            updated_ms: now_ms,
        });
        if let Some(v) = dials.checkers {
            entry.checkers = Some(v);
        }
        if let Some(v) = dials.workers {
            entry.workers = Some(v);
        }
        entry.runs = entry.runs.saturating_add(1);
        entry.updated_ms = now_ms;

        while file.seeds.len() > MAX_SEED_KEYS {
            let oldest = file
                .seeds
                .iter()
                .min_by_key(|(_, e)| e.updated_ms)
                .map(|(k, _)| k.clone())
                .expect("non-empty map has a minimum");
            file.seeds.remove(&oldest);
        }

        self.store(&file)?;
        Ok(true)
    }

    /// The seed for a route + workload class, if one was learned.
    pub fn lookup(&self, route: &RouteTag, class: WorkloadClass) -> Result<Option<SeedEntry>> {
        let Some(key) = Self::key(route, class) else {
            return Ok(None);
        };
        Ok(self.load()?.seeds.remove(&key))
    }

    /// The most recently updated seed for a route, across workload
    /// classes (ph-4). At session OPEN the run's own class is not yet
    /// knowable — the scan has not streamed — so warm-start reads take
    /// the route's newest entry. The asymmetry with the class-keyed
    /// WRITE side is deliberate and safe: the dial MEASURES a seed
    /// before adopting it, so a cross-class seed costs at most one
    /// probing chunk before the walk resumes cold behavior.
    pub fn lookup_route_latest(&self, route: &RouteTag) -> Result<Option<SeedEntry>> {
        let peer = match route.peer_key.as_deref() {
            Some(peer) => peer,
            None => return Ok(None),
        };
        let prefix = format!(
            "{}|{}|{}|{}|",
            route.topology.label(),
            route.local_role.label(),
            route.initiator.label(),
            peer,
        );
        Ok(self
            .load()?
            .seeds
            .into_iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(_, entry)| entry)
            .max_by_key(|entry| entry.updated_ms))
    }

    fn load(&self) -> Result<SeedFile> {
        let path = self.path();
        match std::fs::read(&path) {
            Ok(bytes) => {
                let parsed: SeedFile = serde_json::from_slice(&bytes)
                    .with_context(|| format!("parsing seed store {}", path.display()))?;
                // Future schema bumps migrate here, keyed on
                // `parsed.schema_version` like `migrate_record`.
                Ok(parsed)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(SeedFile::default()),
            Err(err) => Err(err).with_context(|| format!("reading seed store {}", path.display())),
        }
    }

    fn store(&self, file: &SeedFile) -> Result<()> {
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating seed store dir {}", self.dir.display()))?;
        let path = self.path();
        let tmp = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(file)?;
        std::fs::write(&tmp, &bytes)
            .with_context(|| format!("writing seed store temp {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("replacing seed store {}", path.display()))?;
        Ok(())
    }
}

/// Convenience used by the session close paths: fold a settled run
/// into the operator store, swallowing (but logging) failures the same
/// way the history append does.
pub fn record_settled_user(route: &RouteTag, dials: SettledDials, files: usize, bytes: u64) {
    let result =
        SeedStore::user().and_then(|store| store.record_settled(route, dials, files, bytes));
    if let Err(err) = result {
        log::warn!("failed to update dial seed store: {err:?}");
    }
}

/// Convenience used by the session open paths (ph-4): the route's
/// newest seed from the operator store. Any failure — missing file,
/// corrupt JSON, unreadable state dir — is a cold start, never an
/// error: a seed is an optimization hint, and a session must open
/// identically with or without one.
pub fn route_seed_user(route: &RouteTag) -> Option<SeedEntry> {
    match SeedStore::user().and_then(|store| store.lookup_route_latest(route)) {
        Ok(entry) => entry,
        Err(err) => {
            log::debug!("dial seed lookup failed (cold start): {err:?}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf_history::{Initiator as I, LocalRole as R, Topology as T};

    fn route(peer: Option<&str>) -> RouteTag {
        RouteTag {
            topology: T::Local,
            local_role: R::Source,
            initiator: I::Cli,
            peer_key: peer.map(str::to_string),
        }
    }

    fn store() -> (tempfile::TempDir, SeedStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SeedStore::at_dir(dir.path().to_path_buf());
        (dir, store)
    }

    /// ph-4 read side: the route's NEWEST class entry wins, other
    /// routes' entries never bleed in, and a keyless route reads
    /// nothing.
    #[test]
    fn route_latest_takes_the_newest_class_for_that_route_only() {
        let (_dir, store) = store();
        // Absent file: cold start, not an error.
        assert!(store
            .lookup_route_latest(&route(Some("/dst")))
            .expect("absent store reads clean")
            .is_none());
        let json = serde_json::json!({
            "schema_version": SEED_SCHEMA_VERSION,
            "seeds": {
                "local|source|cli|/dst|small": {"checkers": 4, "runs": 3, "updated_ms": 100},
                "local|source|cli|/dst|large": {"checkers": 16, "runs": 1, "updated_ms": 200},
                "local|source|cli|/other|small": {"checkers": 64, "runs": 9, "updated_ms": 999},
            },
        });
        std::fs::create_dir_all(_dir.path()).unwrap();
        std::fs::write(store.path(), serde_json::to_vec(&json).unwrap()).unwrap();
        let seed = store
            .lookup_route_latest(&route(Some("/dst")))
            .expect("read")
            .expect("seed present");
        assert_eq!(seed.checkers, Some(16), "newest entry for the route wins");
        assert!(store
            .lookup_route_latest(&route(None))
            .expect("keyless route reads clean")
            .is_none());
    }

    /// ph-4 poison recovery, store half: a corrupt file is an error at
    /// the store level (`route_seed_user` degrades it to a cold start).
    #[test]
    fn corrupt_store_errors_instead_of_inventing_seeds() {
        let (_dir, store) = store();
        std::fs::create_dir_all(_dir.path()).unwrap();
        std::fs::write(store.path(), b"not json").unwrap();
        assert!(store.lookup_route_latest(&route(Some("/dst"))).is_err());
        assert!(store
            .lookup(&route(Some("/dst")), WorkloadClass::SmallFiles)
            .is_err());
    }

    #[test]
    fn settled_run_seeds_and_round_trips() {
        let (_dir, store) = store();
        let dials = SettledDials {
            checkers: Some(8),
            workers: None,
        };
        assert!(store
            .record_settled(&route(Some("/dst")), dials, 5_000, 10 * 1024 * 1024)
            .expect("record"));
        let seed = store
            .lookup(&route(Some("/dst")), WorkloadClass::SmallFiles)
            .expect("lookup")
            .expect("seed present");
        assert_eq!(seed.checkers, Some(8));
        assert_eq!(seed.workers, None);
        assert_eq!(seed.runs, 1);
    }

    #[test]
    fn probing_run_teaches_nothing() {
        // The review gate: a run whose dial never settled (both slots
        // None) writes NO file at all.
        let (_dir, store) = store();
        assert!(!store
            .record_settled(&route(Some("/dst")), SettledDials::default(), 5_000, 1024,)
            .expect("record"));
        assert!(!store.path().exists(), "no seed file may appear");
    }

    #[test]
    fn tiny_run_teaches_nothing() {
        let (_dir, store) = store();
        let dials = SettledDials {
            checkers: Some(8),
            workers: None,
        };
        assert!(!store
            .record_settled(&route(Some("/dst")), dials, SEED_MIN_FILES - 1, 1024)
            .expect("record"));
        assert!(!store.path().exists());
    }

    #[test]
    fn missing_peer_key_teaches_nothing() {
        let (_dir, store) = store();
        let dials = SettledDials {
            checkers: Some(8),
            workers: None,
        };
        assert!(!store
            .record_settled(&route(None), dials, 5_000, 1024)
            .expect("record"));
        assert!(!store.path().exists());
    }

    #[test]
    fn merge_preserves_the_other_slot_and_counts_runs() {
        let (_dir, store) = store();
        let r = route(Some("/dst"));
        store
            .record_settled(
                &r,
                SettledDials {
                    checkers: Some(8),
                    workers: None,
                },
                5_000,
                1024,
            )
            .expect("first");
        store
            .record_settled(
                &r,
                SettledDials {
                    checkers: None,
                    workers: Some(12),
                },
                5_000,
                1024,
            )
            .expect("second");
        let seed = store
            .lookup(&r, WorkloadClass::SmallFiles)
            .expect("lookup")
            .expect("seed");
        assert_eq!(seed.checkers, Some(8), "earlier slot survives");
        assert_eq!(seed.workers, Some(12));
        assert_eq!(seed.runs, 2);
    }

    #[test]
    fn workload_classes_do_not_blend() {
        let (_dir, store) = store();
        let r = route(Some("/dst"));
        store
            .record_settled(
                &r,
                SettledDials {
                    checkers: Some(4),
                    workers: None,
                },
                200,
                200 * 2 * 1024 * 1024, // 2 MiB mean → large
            )
            .expect("large run");
        assert!(store
            .lookup(&r, WorkloadClass::SmallFiles)
            .expect("lookup")
            .is_none());
        assert_eq!(
            store
                .lookup(&r, WorkloadClass::LargeFiles)
                .expect("lookup")
                .expect("seed")
                .checkers,
            Some(4)
        );
    }

    #[test]
    fn eviction_caps_the_map_at_the_oldest_key() {
        let (_dir, store) = store();
        let dials = SettledDials {
            checkers: Some(2),
            workers: None,
        };
        for i in 0..(MAX_SEED_KEYS + 3) {
            let r = route(Some(&format!("/dst-{i}")));
            store
                .record_settled(&r, dials, 5_000, 1024)
                .expect("record");
        }
        let parsed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(store.path()).expect("read")).expect("json");
        let seeds = parsed["seeds"].as_object().expect("seeds object");
        assert!(seeds.len() <= MAX_SEED_KEYS);
    }

    #[test]
    fn corrupt_file_reports_instead_of_silently_resetting() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), b"not json").unwrap();
        assert!(store
            .lookup(&route(Some("/dst")), WorkloadClass::SmallFiles)
            .is_err());
    }
}
