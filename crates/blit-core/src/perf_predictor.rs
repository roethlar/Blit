//! Adaptive planning predictor fed by local performance history.
//!
//! The predictor maintains lightweight coefficients per workload profile and
//! persists them alongside the performance history log. A simple online
//! gradient-descent update keeps the model responsive while remaining stable.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};
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

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn load_recent_records(
        &self,
        history_path: &PathBuf,
        limit: usize,
    ) -> Result<Vec<PerformanceRecord>> {
        let file = File::open(history_path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines().flatten() {
            if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
                records.push(record);
            }
        }
        let len = records.len();
        if len > limit {
            records = records[len - limit..].to_vec();
        }
        Ok(records)
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
