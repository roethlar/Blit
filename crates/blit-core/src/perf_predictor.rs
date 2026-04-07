//! Adaptive planning predictor fed by local performance history.
//!
//! The predictor maintains lightweight coefficients per workload profile and
//! persists them alongside the performance history log. A simple online
//! gradient-descent update keeps the model responsive while remaining stable.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::perf_history::{config_dir, PerformanceRecord, TransferMode};

const STATE_VERSION: u32 = 1;
const STATE_FILENAME: &str = "perf_predictor.json";

// Default coefficients (ms contributions).
const DEFAULT_ALPHA_MS_PER_FILE: f64 = 0.05;
const DEFAULT_BETA_MS_PER_MB: f64 = 0.01;
const DEFAULT_GAMMA_MS: f64 = 50.0;

const LEARNING_RATE: f64 = 0.0005;
const MIN_COEFFICIENT: f64 = 0.000001;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PredictorCoefficients {
    alpha_ms_per_file: f64,
    beta_ms_per_mb: f64,
    gamma_ms: f64,
}

impl PredictorCoefficients {
    fn default() -> Self {
        Self {
            alpha_ms_per_file: DEFAULT_ALPHA_MS_PER_FILE,
            beta_ms_per_mb: DEFAULT_BETA_MS_PER_MB,
            gamma_ms: DEFAULT_GAMMA_MS,
        }
    }

    fn clamp(&mut self) {
        self.alpha_ms_per_file = self.alpha_ms_per_file.max(MIN_COEFFICIENT);
        self.beta_ms_per_mb = self.beta_ms_per_mb.max(MIN_COEFFICIENT);
        self.gamma_ms = self.gamma_ms.max(MIN_COEFFICIENT);
    }

    fn predict_ms(&self, file_count: usize, total_bytes: u64) -> f64 {
        let files = file_count as f64;
        let mb = bytes_to_mb(total_bytes);
        self.alpha_ms_per_file * files + self.beta_ms_per_mb * mb + self.gamma_ms
    }

    fn apply_observation(&mut self, file_count: usize, total_bytes: u64, observed_ms: f64) {
        let files = file_count as f64;
        let mb = bytes_to_mb(total_bytes);
        let predicted = self.predict_ms(file_count, total_bytes);
        let error = observed_ms - predicted;

        self.alpha_ms_per_file += LEARNING_RATE * error * files;
        self.beta_ms_per_mb += LEARNING_RATE * error * mb;
        self.gamma_ms += LEARNING_RATE * error;
        self.clamp();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PredictorProfile {
    coefficients: PredictorCoefficients,
    observations: u64,
}

impl PredictorProfile {
    fn new() -> Self {
        Self {
            coefficients: PredictorCoefficients::default(),
            observations: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
struct ProfileKey {
    source_fs: Option<String>,
    dest_fs: Option<String>,
    mode: TransferMode,
    fast_path: Option<String>,
    skip_unchanged: bool,
    checksum: bool,
}

impl ProfileKey {
    fn new(record: &PerformanceRecord) -> Self {
        Self {
            source_fs: record.source_fs.clone(),
            dest_fs: record.dest_fs.clone(),
            mode: record.mode.clone(),
            fast_path: record.fast_path.clone(),
            skip_unchanged: record.options.skip_unchanged,
            checksum: record.options.checksum,
        }
    }

    fn from_components(
        source_fs: Option<String>,
        dest_fs: Option<String>,
        mode: TransferMode,
        fast_path: Option<&str>,
        skip_unchanged: bool,
        checksum: bool,
    ) -> Self {
        Self {
            source_fs,
            dest_fs,
            mode,
            fast_path: fast_path.map(|s| s.to_string()),
            skip_unchanged,
            checksum,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PredictorState {
    version: u32,
    #[serde(with = "profile_map")]
    profiles: HashMap<ProfileKey, PredictorProfile>,
}

impl PredictorState {
    fn new() -> Self {
        Self {
            version: STATE_VERSION,
            profiles: HashMap::new(),
        }
    }
}

pub struct PerformancePredictor {
    state: PredictorState,
    path: PathBuf,
}

impl PerformancePredictor {
    pub fn load() -> Result<Self> {
        let path = config_dir()?.join(STATE_FILENAME);
        if let Ok(mut file) = File::open(&path) {
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            let mut state: PredictorState =
                serde_json::from_str(&buf).context("parse predictor state")?;
            if state.version != STATE_VERSION {
                state = PredictorState::new();
            }
            Ok(Self { state, path })
        } else {
            Ok(Self {
                state: PredictorState::new(),
                path,
            })
        }
    }

    pub fn predict_ms(&mut self, record: &PerformanceRecord) -> f64 {
        let key = ProfileKey::new(record);
        let profile = self
            .state
            .profiles
            .entry(key)
            .or_insert_with(PredictorProfile::new);
        profile
            .coefficients
            .predict_ms(record.file_count, record.total_bytes)
    }

    pub fn predict_planner_ms(
        &self,
        mode: TransferMode,
        fast_path: Option<&str>,
        skip_unchanged: bool,
        checksum: bool,
        file_count: usize,
        total_bytes: u64,
    ) -> Option<(f64, u64)> {
        let key =
            ProfileKey::from_components(None, None, mode, fast_path, skip_unchanged, checksum);
        self.state.profiles.get(&key).map(|profile| {
            (
                profile.coefficients.predict_ms(file_count, total_bytes),
                profile.observations,
            )
        })
    }

    pub fn observe(&mut self, record: &PerformanceRecord) {
        let key = ProfileKey::new(record);
        let profile = self
            .state
            .profiles
            .entry(key)
            .or_insert_with(PredictorProfile::new);
        profile.coefficients.apply_observation(
            record.file_count,
            record.total_bytes,
            record.planner_duration_ms as f64,
        );
        profile.observations = profile.observations.saturating_add(1);
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&self.path)?;
        let data = serde_json::to_string_pretty(&self.state)?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_recent_records(
        &self,
        history_path: &Path,
        limit: usize,
    ) -> Result<Vec<PerformanceRecord>> {
        use crate::perf_history::migrate_record;
        let file = File::open(history_path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
                records.push(migrate_record(record));
            }
        }
        let len = records.len();
        if len > limit {
            records = records[len - limit..].to_vec();
        }
        Ok(records)
    }
}

#[derive(Serialize, Deserialize)]
struct ProfileEntry {
    key: ProfileKey,
    value: PredictorProfile,
}

mod profile_map {
    use super::{PredictorProfile, ProfileEntry, ProfileKey};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(
        map: &HashMap<ProfileKey, PredictorProfile>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<ProfileEntry> = map
            .iter()
            .map(|(key, value)| ProfileEntry {
                key: key.clone(),
                value: value.clone(),
            })
            .collect();
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<ProfileKey, PredictorProfile>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries: Vec<ProfileEntry> = Vec::deserialize(deserializer)?;
        Ok(entries
            .into_iter()
            .map(|entry| (entry.key, entry.value))
            .collect())
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

#[cfg(test)]
impl PerformancePredictor {
    pub fn for_tests(dir: &Path) -> Self {
        Self {
            state: PredictorState::new(),
            path: dir.join(STATE_FILENAME),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf_history::OptionSnapshot;

    fn make_record(
        mode: TransferMode,
        file_count: usize,
        total_bytes: u64,
        planner_ms: u128,
    ) -> PerformanceRecord {
        PerformanceRecord {
            schema_version: 1,
            timestamp_epoch_ms: 0,
            mode,
            source_fs: None,
            dest_fs: None,
            file_count,
            total_bytes,
            options: OptionSnapshot {
                dry_run: false,
                preserve_symlinks: true,
                include_symlinks: false,
                skip_unchanged: true,
                checksum: false,
                workers: 4,
            },
            fast_path: None,
            planner_duration_ms: planner_ms,
            transfer_duration_ms: 0,
            stall_events: 0,
            error_count: 0,
            tar_shard_tasks: 0,
            tar_shard_files: 0,
            tar_shard_bytes: 0,
            raw_bundle_tasks: 0,
            raw_bundle_files: 0,
            raw_bundle_bytes: 0,
            large_tasks: 0,
            large_bytes: 0,
        }
    }

    #[test]
    fn default_coefficients_produce_positive_prediction() {
        let coeffs = PredictorCoefficients::default();
        let prediction = coeffs.predict_ms(100, 10 * 1024 * 1024);
        assert!(prediction > 0.0, "prediction should be positive");
    }

    #[test]
    fn coefficients_never_go_negative() {
        let mut coeffs = PredictorCoefficients::default();
        // Observe 0 ms many times — drives coefficients toward zero
        for _ in 0..10_000 {
            coeffs.apply_observation(100, 10 * 1024 * 1024, 0.0);
        }
        assert!(coeffs.alpha_ms_per_file >= MIN_COEFFICIENT);
        assert!(coeffs.beta_ms_per_mb >= MIN_COEFFICIENT);
        assert!(coeffs.gamma_ms >= MIN_COEFFICIENT);
    }

    #[test]
    fn predictions_converge_toward_observations() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut predictor = PerformancePredictor::for_tests(dir.path());

        // Use small feature values to keep gradient updates stable
        // (lr=0.0005, files=10 → effective lr per alpha step = 0.005)
        let target_ms = 200.0;
        let file_count = 10;
        let total_bytes = 1024 * 1024; // 1 MiB

        for _ in 0..500 {
            let record = make_record(
                TransferMode::Copy,
                file_count,
                total_bytes,
                target_ms as u128,
            );
            predictor.observe(&record);
        }

        let prediction =
            predictor.predict_ms(&make_record(TransferMode::Copy, file_count, total_bytes, 0));
        let error_pct = ((prediction - target_ms) / target_ms).abs() * 100.0;
        assert!(
            error_pct < 15.0,
            "prediction {:.1} ms should be within 15% of target {:.1} ms (error: {:.1}%)",
            prediction,
            target_ms,
            error_pct
        );
    }

    #[test]
    fn prediction_improves_with_observations() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut predictor = PerformancePredictor::for_tests(dir.path());

        // Small features to avoid gradient explosion
        let target_ms = 100.0;
        let file_count = 5;
        let total_bytes = 512 * 1024; // 0.5 MiB

        // Initial prediction (default coefficients)
        let initial =
            predictor.predict_ms(&make_record(TransferMode::Copy, file_count, total_bytes, 0));
        let initial_error = (initial - target_ms).abs();

        // Train
        for _ in 0..200 {
            let record = make_record(
                TransferMode::Copy,
                file_count,
                total_bytes,
                target_ms as u128,
            );
            predictor.observe(&record);
        }

        let trained =
            predictor.predict_ms(&make_record(TransferMode::Copy, file_count, total_bytes, 0));
        let trained_error = (trained - target_ms).abs();

        assert!(
            trained_error < initial_error,
            "trained error ({:.1}) should be less than initial error ({:.1})",
            trained_error,
            initial_error
        );
    }

    #[test]
    fn profiles_are_isolated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut predictor = PerformancePredictor::for_tests(dir.path());

        // Small features for gradient stability
        let file_count = 5;
        let total_bytes = 256 * 1024;

        // Train copy profile with 50ms
        for _ in 0..200 {
            predictor.observe(&make_record(
                TransferMode::Copy,
                file_count,
                total_bytes,
                50,
            ));
        }

        // Train mirror profile with 150ms
        for _ in 0..200 {
            predictor.observe(&make_record(
                TransferMode::Mirror,
                file_count,
                total_bytes,
                150,
            ));
        }

        let copy_pred =
            predictor.predict_ms(&make_record(TransferMode::Copy, file_count, total_bytes, 0));
        let mirror_pred = predictor.predict_ms(&make_record(
            TransferMode::Mirror,
            file_count,
            total_bytes,
            0,
        ));

        // Profiles should be independent — mirror trained on higher values
        assert!(
            (copy_pred - mirror_pred).abs() > 10.0,
            "profiles should diverge: copy={:.1}, mirror={:.1}",
            copy_pred,
            mirror_pred
        );
    }

    #[test]
    fn save_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut predictor = PerformancePredictor::for_tests(dir.path());

        // Train it
        for _ in 0..50 {
            predictor.observe(&make_record(TransferMode::Copy, 200, 50 * 1024 * 1024, 250));
        }

        let prediction_before =
            predictor.predict_ms(&make_record(TransferMode::Copy, 200, 50 * 1024 * 1024, 0));

        // Save and reload
        predictor.save().expect("save");
        let mut loaded = PerformancePredictor::for_tests(dir.path());
        let path = dir.path().join(STATE_FILENAME);
        let mut file = File::open(&path).expect("open");
        let mut buf = String::new();
        file.read_to_string(&mut buf).expect("read");
        let state: PredictorState = serde_json::from_str(&buf).expect("parse");
        loaded.state = state;

        let prediction_after =
            loaded.predict_ms(&make_record(TransferMode::Copy, 200, 50 * 1024 * 1024, 0));

        assert!(
            (prediction_before - prediction_after).abs() < 0.001,
            "predictions should match after round-trip: before={:.3}, after={:.3}",
            prediction_before,
            prediction_after
        );
    }

    #[test]
    fn predict_planner_ms_returns_none_for_unseen_profile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let predictor = PerformancePredictor::for_tests(dir.path());

        let result = predictor.predict_planner_ms(TransferMode::Copy, None, true, false, 100, 1024);
        assert!(result.is_none(), "unseen profile should return None");
    }

    #[test]
    fn predict_planner_ms_returns_value_after_observation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut predictor = PerformancePredictor::for_tests(dir.path());

        predictor.observe(&make_record(TransferMode::Copy, 100, 1024, 50));

        let result = predictor.predict_planner_ms(TransferMode::Copy, None, true, false, 100, 1024);
        assert!(
            result.is_some(),
            "should return prediction after observation"
        );
        let (ms, obs) = result.unwrap();
        assert!(ms > 0.0);
        assert_eq!(obs, 1);
    }

    #[test]
    fn prediction_scales_with_file_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut predictor = PerformancePredictor::for_tests(dir.path());

        // Train: more files → more time
        for _ in 0..100 {
            predictor.observe(&make_record(TransferMode::Copy, 100, 0, 100));
            predictor.observe(&make_record(TransferMode::Copy, 1000, 0, 1000));
        }

        let pred_100 = predictor.predict_ms(&make_record(TransferMode::Copy, 100, 0, 0));
        let pred_1000 = predictor.predict_ms(&make_record(TransferMode::Copy, 1000, 0, 0));

        assert!(
            pred_1000 > pred_100,
            "1000 files ({:.1} ms) should predict higher than 100 files ({:.1} ms)",
            pred_1000,
            pred_100
        );
    }
}
