# AI-Powered Telemetry Analysis: Scope and Guardrails

## Overview

Blit captures per-run performance records in `perf_local.jsonl` (capped at
~1 MiB). The adaptive predictor already uses this data for planner heuristics.
This document explores optional AI-powered analysis of that same local data
for anomaly detection, tuning recommendations, and diagnostics.

## Design Principles

1. **All data stays on-device.** No telemetry is sent to external services.
   Analysis runs locally using the existing JSONL file.
2. **Opt-in only.** Analysis is triggered explicitly by the user
   (`blit diagnostics analyze` or similar), never automatically.
3. **No model dependencies.** Analysis uses statistical methods, not ML models.
   No additional binary dependencies, no GPU requirements.
4. **Actionable output.** Every recommendation must include a concrete CLI
   flag or config change the user can apply.

## Proposed Features

### 1. Anomaly Detection

Detect runs that deviate significantly from historical baselines.

**What to flag:**
- Throughput drops >50% compared to the rolling median for similar workloads
  (same mode, similar file count/size range)
- Stall event spikes (>2x the historical average)
- Error rate increases
- Planner duration outliers (>3x the p95 for the workload class)

**Workload classification:**
- Group records by `(mode, file_count_bucket, total_bytes_bucket)`
- Buckets: small (<100 files), medium (100-10K), large (>10K)
- Size: small (<100 MiB), medium (100 MiB - 10 GiB), large (>10 GiB)

**Output example:**
```
Recent anomalies (last 7 days):
  2026-04-05 14:23  mirror  5,200 files / 8.3 GiB  throughput: 245 MB/s
    WARNING: 62% below median (645 MB/s) for similar workloads
    Possible causes: disk contention, network degradation
    Suggestion: check destination I/O with `iostat` or retry with --resume
```

### 2. Tuning Recommendations

Analyze historical patterns to suggest optimal settings.

**Recommendations to generate:**
- Worker count: If records show diminishing returns above N workers, suggest
  `--workers N`
- Checksum mode: If `--checksum` runs show <5% catch rate, suggest metadata
  comparison is sufficient
- Resume mode: If repeated transfers to the same destination show >30%
  unchanged files, suggest `--resume`
- Tar shard efficiency: If tar shard ratio (shard_files/total_files) is low
  for small-file workloads, investigate threshold tuning

**Output example:**
```
Tuning suggestions based on 47 recent runs:
  - Your mirror operations average 8.2 workers. Records show throughput
    plateaus at 6 workers. Consider: --workers 6
  - 12 of 15 checksum runs found 0 mismatches. Metadata comparison
    (default) may be sufficient for this workload.
```

### 3. Diagnostics Summary

Provide a human-readable summary of transfer health.

**Metrics:**
- Average throughput by mode (copy/mirror) over last N runs
- Throughput trend (improving, stable, degrading)
- Error rate trend
- Most common fast-path selections
- Tar shard utilization statistics

**Output example:**
```
Performance summary (last 30 days, 23 runs):
  Copy:   avg 1.2 GiB/s  (stable)    errors: 0.4%
  Mirror: avg 890 MiB/s   (improving) errors: 0.1%

  Fast path usage: tiny_manifest 48%, standard 39%, huge_manifest 13%
  Tar shards: 72% of small files batched (avg 34 files/shard)
```

## Implementation Plan

### Phase 1: Statistics Engine

Add a `perf_analysis` module to `blit-core` with:

```rust
pub struct WorkloadClass {
    pub mode: TransferMode,
    pub file_count_bucket: Bucket,
    pub size_bucket: Bucket,
}

pub struct WorkloadStats {
    pub throughput_median: f64,
    pub throughput_p95: f64,
    pub planner_median_ms: f64,
    pub stall_rate: f64,
    pub error_rate: f64,
    pub sample_count: usize,
}

pub fn classify_records(records: &[PerformanceRecord]) -> HashMap<WorkloadClass, WorkloadStats>;
pub fn detect_anomalies(records: &[PerformanceRecord]) -> Vec<Anomaly>;
pub fn generate_recommendations(records: &[PerformanceRecord]) -> Vec<Recommendation>;
```

### Phase 2: CLI Integration

Add subcommand to `blit-cli`:

```
blit diagnostics analyze [--days <N>] [--json]
```

Reads local `perf_local.jsonl`, runs analysis, prints results. JSON output
for scripting.

### Phase 3: blit-utils Integration

Add `blit-utils analyze` as a standalone equivalent for operators who only
have the utils binary installed.

## Guardrails

- **No network access.** The analysis module must not make any network calls.
  All data comes from the local JSONL file.
- **No PII.** Performance records contain only numeric metrics and mode names.
  No file paths, hostnames, or user data is stored or analyzed.
- **Bounded computation.** Analysis must complete in <1 second for the maximum
  JSONL file size (~1 MiB, ~1000 records). No unbounded loops.
- **Graceful degradation.** If <10 records exist, skip anomaly detection and
  print a message suggesting more data is needed. Recommendations require at
  least 5 records in a workload class.
- **No auto-apply.** Recommendations are suggestions only. The tool never
  modifies config files or CLI defaults automatically.
- **Versioned output.** JSON output includes a `schema_version` field so
  downstream tools can handle format changes.

## Not In Scope

- Machine learning or model training
- Cloud-based analysis or data upload
- Real-time monitoring (daemon-side)
- Comparison across different machines
- Integration with external observability platforms (Prometheus, Grafana)

These could be explored in future iterations but are explicitly excluded from
the initial implementation to keep scope manageable.

## Dependencies

None beyond what `blit-core` already provides. The implementation uses only
standard library collections and arithmetic.
