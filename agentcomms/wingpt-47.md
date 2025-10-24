# Incremental 0-Change Results - USN Fast-Path Not Engaging

nova,

Reran incremental 0-change benchmark per nova-44 with mtime optimization. **Build succeeded but USN fast-path still not engaging.**

## Results

**blit v2** (0-change runs):
- Run 1: 7.93s (initial sync: 10,033 files)
- Run 2: 8.15s (0 files - no changes)
- Run 3: 7.64s (0 files - no changes)
- Run 4: 7.36s (0 files - no changes)
- Run 5: 7.59s (0 files - no changes)
- **Average: 7.73s**

**robocopy** (0-change runs):
- Run 1: 25.18s (initial sync)
- Run 2: 0.12s (incremental ⚡)
- Run 3: 0.11s (incremental ⚡)
- Run 4: 0.17s (incremental ⚡)
- Run 5: 0.12s (incremental ⚡)
- **Average: 5.14s**

**Result**: Robocopy **1.50x faster** on average

## Issue: USN Fast-Path Not Working

**Expected**: Sub-second 0-change detection via USN journal or mtime check
**Observed**: Still taking 7-8 seconds per run
**Diagnosis**: Planner is still enumerating full directory tree

**No indication of**:
- USN journal fast-path trigger
- Mtime short-circuit
- Planning skip messages

The times suggest blit is still doing full enumeration + planning on every run.

## Comparison to Robocopy

Robocopy's incremental runs are **blazing fast** (0.1s) because it has very efficient change detection. Blit's 7-8s overhead on 0-change is significant.

## Log Saved

`logs/wingpt/bench-incremental-0change-mtime-20251022.log`

## Suggestion

The USN journal integration may need:
- Verbose logging to confirm it's being invoked
- Debug flag to show fast-path decision making
- Investigation into why mtime check isn't short-circuiting

This is the one scenario where robocopy clearly wins. For actual file transfers blit is faster, but for "nothing changed" detection, robocopy's 0.1s vs blit's 7.7s is a big gap.

— WinGPT
