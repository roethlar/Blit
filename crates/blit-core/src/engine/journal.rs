//! Change-journal checkpoint persistence + probe logging for the
//! engine's `journal_no_work` strategy. Moved verbatim from
//! `orchestrator/orchestrator.rs` at ue-r2-1c.

use crate::change_journal::{ChangeTracker, ProbeToken, StoredSnapshot};

pub(super) fn persist_journal_checkpoints(
    tracker: &mut ChangeTracker,
    tokens: &mut [ProbeToken],
    verbose: bool,
) {
    if tokens.is_empty() {
        return;
    }

    for token in tokens.iter_mut() {
        match tracker.reprobe_canonical(&token.canonical_path) {
            Ok(snapshot) => token.snapshot = snapshot,
            Err(err) => {
                token.snapshot = None;
                if verbose {
                    eprintln!(
                        "Failed to refresh journal snapshot for {}: {err:?}",
                        token.canonical_path.display()
                    );
                }
            }
        }
    }

    if let Err(err) = tracker.refresh_and_persist(tokens) {
        if verbose {
            eprintln!("Failed to update journal checkpoint: {err:?}");
        }
    }
}

pub(super) fn log_probe(label: &str, probe: &ProbeToken) {
    eprintln!(
        "Journal probe {label} state={:?} snapshot={} path={}",
        probe.state,
        probe.snapshot.is_some(),
        probe.canonical_path.display()
    );

    if let Some(snapshot) = &probe.snapshot {
        match snapshot {
            StoredSnapshot::Windows(snap) => {
                eprintln!(
                    "  {label} windows: volume={} journal_id={} next_usn={} mtime={:?}",
                    snap.volume, snap.journal_id, snap.next_usn, snap.root_mtime_epoch_ms
                );
            }
            StoredSnapshot::MacOs(snap) => {
                eprintln!(
                    "  {label} macOS: fsid={} event_id={}",
                    snap.fsid, snap.event_id
                );
            }
            StoredSnapshot::Linux(snap) => {
                eprintln!(
                    "  {label} linux: device={} inode={} ctime={}s+{}ns mtime={:?}",
                    snap.device,
                    snap.inode,
                    snap.ctime_sec,
                    snap.ctime_nsec,
                    snap.root_mtime_epoch_ms
                );
            }
        }
    }
}
