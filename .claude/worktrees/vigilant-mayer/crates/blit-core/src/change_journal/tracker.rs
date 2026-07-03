use super::snapshot::{capture_snapshot, compare_snapshots};
use super::types::{ChangeState, ChangeTracker, ProbeToken, StoredRecord, StoredSnapshot};
use super::util::{canonical_to_key, canonicalize, journal_store_path, now_ms};
use eyre::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::Path;

impl ChangeTracker {
    pub fn load() -> Result<Self> {
        let path = journal_store_path()?;
        if !path.exists() {
            return Ok(Self {
                path,
                records: Default::default(),
            });
        }

        let file = File::open(&path)
            .with_context(|| format!("failed to open journal cache {}", path.display()))?;
        let reader = BufReader::new(file);

        let records: std::collections::HashMap<String, StoredRecord> =
            match serde_json::from_reader(reader) {
                Ok(records) => records,
                Err(err) => {
                    eprintln!(
                        "change_journal: failed to parse journal cache {} ({err}); starting fresh",
                        path.display()
                    );
                    Default::default()
                }
            };

        Ok(Self { path, records })
    }

    pub fn probe(&self, root: &Path) -> Result<ProbeToken> {
        let canonical = canonicalize(root)?;
        let key = canonical_to_key(&canonical);
        let new_snapshot = capture_snapshot(&canonical)?;
        let state = match (&new_snapshot, self.records.get(&key)) {
            (None, _) => ChangeState::Unsupported,
            (Some(_), None) => ChangeState::Unknown,
            (Some(new), Some(stored)) => compare_snapshots(&stored.snapshot, new),
        };

        Ok(ProbeToken {
            key,
            canonical_path: canonical,
            snapshot: new_snapshot,
            state,
        })
    }

    pub fn refresh_and_persist(&mut self, tokens: &[ProbeToken]) -> Result<()> {
        let mut changed = false;

        for token in tokens {
            match &token.snapshot {
                Some(snapshot) => {
                    let record = StoredRecord {
                        snapshot: snapshot.clone(),
                        recorded_at_epoch_ms: now_ms(),
                    };
                    self.records.insert(token.key.clone(), record);
                    changed = true;
                }
                None => {
                    if self.records.remove(&token.key).is_some() {
                        changed = true;
                    }
                }
            }
        }

        if changed {
            self.persist()?;
        }

        Ok(())
    }

    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create journal cache directory {}",
                    parent.display()
                )
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.path)
            .with_context(|| format!("failed to write journal cache {}", self.path.display()))?;

        serde_json::to_writer_pretty(&mut file, &self.records).with_context(|| {
            format!(
                "failed to serialise journal cache to {}",
                self.path.display()
            )
        })?;

        file.flush()?;
        Ok(())
    }

    pub fn reprobe_canonical(&self, canonical_path: &Path) -> Result<Option<StoredSnapshot>> {
        capture_snapshot(canonical_path)
    }
}
