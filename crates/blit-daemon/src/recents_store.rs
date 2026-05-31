//! Durable backing store for the `GetState.recent[]` ring.
//!
//! `rec-1` of the recent-persistence feature. The daemon's recent-runs
//! ring ([`crate::active_jobs::ActiveJobs`]) is otherwise in-memory only
//! (lost on restart). This module persists it to a dedicated JSONL file
//! so the TUI's F2 recent list survives daemon restarts.
//!
//! **Separation from planner telemetry.** The recents store lives in its
//! own file, `recents.jsonl`, alongside but entirely separate from
//! `perf_local.jsonl` (the predictor's training data in
//! [`blit_core::perf_history`]). A future `ClearRecent` (rec-2) wipes
//! `recents.jsonl` only — the planner's telemetry is never touched.
//!
//! **Format & durability.** One [`TransferRecord`] per line, oldest
//! first (the ring's own order). The file is rewritten in full on each
//! update — the ring is bounded ([`crate::active_jobs::DEFAULT_RECENT_LIMIT`]),
//! so the file stays small — via a temp-file + atomic rename so a crash
//! mid-write can never leave a torn file. Loading is tolerant: a missing
//! file yields an empty ring, and an unparseable line is skipped rather
//! than failing the daemon (a hand-edited or partially-migrated file
//! must never prevent startup).

use crate::active_jobs::TransferRecord;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Filename of the recents store, inside [`blit_core::config::config_dir`]
/// next to `perf_local.jsonl`.
const RECENTS_FILE: &str = "recents.jsonl";

/// Absolute path to the recents store — `config_dir()/recents.jsonl`.
/// Same directory as the planner's `perf_local.jsonl`, separate file.
pub fn recents_path() -> eyre::Result<PathBuf> {
    Ok(blit_core::config::config_dir()?.join(RECENTS_FILE))
}

/// Load up to `limit` most-recent records from `path`, oldest first.
///
/// Tolerant by design: a missing file (or empty file) yields an empty
/// `Vec`, and any line that fails to parse as a [`TransferRecord`] is
/// skipped. Never returns an error for a malformed store — recents are
/// informational and must not block daemon startup. Only the last
/// `limit` records are kept (matching the ring's eviction policy), so a
/// file that grew beyond `limit` from an older build is trimmed on load.
pub fn load(path: &Path, limit: usize) -> Vec<TransferRecord> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        // Missing file is the common first-run case; any other read
        // error (permissions, etc.) degrades to "no recents" rather
        // than failing startup.
        Err(_) => return Vec::new(),
    };
    let mut records: Vec<TransferRecord> = contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<TransferRecord>(line).ok())
        .collect();
    if limit == 0 {
        return Vec::new();
    }
    if records.len() > limit {
        // Keep the newest `limit` (records are oldest-first).
        records.drain(0..records.len() - limit);
    }
    records
}

/// Atomically rewrite `path` with `records` (oldest first), one JSON
/// object per line. Writes to a sibling temp file then renames over the
/// target, so a reader (or a crash) never observes a partially-written
/// store. Creates the parent directory if needed.
pub fn write_atomic(path: &Path, records: &[TransferRecord]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("jsonl.tmp");
    {
        let mut file = std::fs::File::create(&tmp)?;
        for record in records {
            // `to_vec` can only fail on a non-string map key or a
            // custom Serialize that errors; TransferRecord has neither,
            // so this is effectively infallible. Map it to io::Error
            // rather than unwrap so a future field change surfaces
            // cleanly instead of panicking the writer task.
            let line = serde_json::to_vec(record)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(&line)?;
            file.write_all(b"\n")?;
        }
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_jobs::{ActiveJobKind, TransferRecord};

    fn rec(id: &str) -> TransferRecord {
        TransferRecord {
            transfer_id: id.to_string(),
            kind: ActiveJobKind::Pull,
            peer: "peer".to_string(),
            module: "mod".to_string(),
            path: "path".to_string(),
            start_unix_ms: 1,
            duration_ms: 2,
            bytes: 3,
            ok: true,
            error_message: String::new(),
        }
    }

    #[test]
    fn load_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        assert!(load(&path, 50).is_empty());
    }

    #[test]
    fn write_then_load_round_trips_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        let records = vec![rec("a"), rec("b"), rec("c")];
        write_atomic(&path, &records).unwrap();
        let loaded = load(&path, 50);
        let ids: Vec<_> = loaded.iter().map(|r| r.transfer_id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
    }

    #[test]
    fn load_skips_malformed_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        let good = serde_json::to_string(&rec("good")).unwrap();
        std::fs::write(&path, format!("{good}\nnot json\n\n{good}\n")).unwrap();
        let loaded = load(&path, 50);
        assert_eq!(loaded.len(), 2);
        assert!(loaded.iter().all(|r| r.transfer_id == "good"));
    }

    #[test]
    fn load_trims_to_limit_keeping_newest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        let records = vec![rec("a"), rec("b"), rec("c"), rec("d")];
        write_atomic(&path, &records).unwrap();
        let loaded = load(&path, 2);
        let ids: Vec<_> = loaded.iter().map(|r| r.transfer_id.as_str()).collect();
        assert_eq!(ids, ["c", "d"], "keeps the newest `limit`, oldest-first");
    }

    #[test]
    fn load_zero_limit_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        write_atomic(&path, &[rec("a")]).unwrap();
        assert!(load(&path, 0).is_empty());
    }

    #[test]
    fn write_atomic_replaces_existing_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        write_atomic(&path, &[rec("old1"), rec("old2")]).unwrap();
        write_atomic(&path, &[rec("new")]).unwrap();
        let loaded = load(&path, 50);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].transfer_id, "new");
        assert!(
            !path.with_extension("jsonl.tmp").exists(),
            "temp file is renamed away, not left behind"
        );
    }
}
