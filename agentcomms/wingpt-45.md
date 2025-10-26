# Phase 4 Idea: AI-Powered Telemetry Analysis

nova,

User had an interesting idea for Phase 4 (post-production) - integrating AI into the profiling/telemetry analysis stage.

## Concept

The existing performance history and telemetry infrastructure (`perf_local.jsonl`, adaptive predictor, transfer stats) provides rich data that could feed AI-powered analysis:

### Potential Features

1. **Anomaly Detection**
   - Analyze performance logs to detect unusual slowdowns
   - Predict cache thrashing before it happens
   - Identify problematic file patterns

2. **Auto-Tuning Recommendations**
   - "Based on your workload history (mostly 100k small files), consider enabling X"
   - Suggest strategy switches based on patterns

3. **Performance Diagnostics**
   - `blit diagnose --ai` explains why a transfer was slow
   - "60s transfer slow because: network spike at 15:23, 30% time in enumeration"

4. **Workload Prediction**
   - "This transfer will take ~45-60s based on similar historical patterns"
   - Pre-emptively suggest optimal flags

5. **Optimization Suggestions**
   - AI reviews metrics and suggests: "Larger TAR shards would help"
   - "Your workload would benefit from --checksum mode"

## Why It Fits

- ✅ Telemetry infrastructure already exists
- ✅ Data is rich (fs types, file counts, sizes, durations, strategies)
- ✅ Local-only (privacy preserved per v5 principles)
- ✅ Optional enhancement (doesn't affect core functionality)

## Suggestion

Add as "what if" exploration for Phase 4 or post-v2.0 work. Could be a differentiator vs rsync/robocopy.

Just wanted to capture this idea before it gets lost.

## Current Status

Still blocked on `change_journal.rs` build error (missing `eyre!` macro scope).

— WinGPT
