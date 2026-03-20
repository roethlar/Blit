# rclone Feature Integration Plan

## Goal

Adopt the most impactful operational features from rclone's mature ecosystem and integrate them natively into Blit's Rust architecture. This is **not** a port or fork -- it is a selective adoption of design patterns and user-facing capabilities that rclone has proven over 10+ years, implemented from scratch to leverage Blit's existing strengths (TCP data plane, tar shards, change journals, performance predictor).

**Guiding principle:** Steal the *ideas*, not the code. Every feature listed here is implementable in Rust without depending on rclone. Where rclone source is referenced, it is for design inspiration only.

## Baseline Assessment

### What Blit already does better than rclone

| Area | Blit advantage |
|------|----------------|
| Raw throughput | Custom TCP data plane with multi-stream, auto-tuned chunk sizes, zero-copy paths |
| Small file batching | Tar shard streaming over data plane (rclone sends files individually) |
| Change detection | OS-native journals (USN, FSEvents, Linux metadata snapshots) |
| Performance tuning | Adaptive predictor + local history with heuristic-driven scheduling |
| Platform copy paths | Deep per-OS optimization (CopyFileExW, clonefile, copy_file_range, ReFS block clone) |
| Daemon architecture | Persistent gRPC server with module exports, mDNS, admin RPCs |

### What rclone has that Blit lacks

| Area | Impact | rclone reference |
|------|--------|------------------|
| Filtering (include/exclude) | Very High | `fs/filter/` |
| Bandwidth limiting | High | `fs/accounting/token_bucket.go` |
| Integrity verification (`check`) | High | `fs/operations/check.go` |
| Error classification + retries | High | `fs/fserrors/`, `lib/pacer/` |
| Metrics export (Prometheus) | Medium | `fs/accounting/prometheus.go` |
| Deduplication | Medium | `fs/operations/dedupe.go` |
| Interactive TUI (ncdu) | Medium | `cmd/ncdu/` |
| Parallel march (lock-step walk) | Medium | `fs/march/march.go` |
| Bidirectional sync | Medium | `cmd/bisync/` |
| VFS / FUSE mount | Medium | `vfs/` |
| Serve protocols (HTTP/WebDAV) | Low | `cmd/serve/` |

## Relationship to Existing Plans

- **PERFORMANCE_ROADMAP.md** covers buffer pool, async I/O pipeline, TCP tuning, and chunked large-file transfers. Those items are **not duplicated** here. This plan covers *feature breadth*, not raw throughput.
- **REMOTE_TRANSFER_PARITY.md** is complete. The shared `remote::transfer` modules it produced are the foundation for several items below (retries, bandwidth limiting wrap the existing data plane).
- **TODO.md Phase 4** items (packaging, docs, CI) remain separate. This plan feeds new items into Phase 4 and adds a new **Phase 5: Operational Features**.

---

## Phase 5A: Core Operational Features (P0)

These are table-stakes for production use. Ship before v1.0.

### 5A.1 Filtering System

**rclone reference:** `fs/filter/filter.go`, `fs/filter/rules.go`, `fs/filter/glob.go`

**Scope:**
- `--exclude PATTERN` / `--include PATTERN` with multiple invocations and ordered precedence
- `--exclude-from FILE` / `--include-from FILE` for rule files
- `--min-size SIZE` / `--max-size SIZE`
- `--min-age DURATION` / `--max-age DURATION`
- `--files-from FILE` for explicit file lists (one path per line)

**Design:**
- New module: `blit-core/src/filter.rs` (~400 LOC)
- `FilterRules` struct holding ordered `Vec<FilterRule>` where each rule is Include/Exclude with a `GlobMatcher` (reuse existing `globset` dependency)
- Size/age filters as additional predicates applied after glob match
- `FilterRules::matches(&self, path: &Path, size: u64, mtime: SystemTime) -> FilterResult` returns `Include | Exclude | NoMatch`
- First-match-wins semantics (same as rclone and rsync)
- Wire into `enumeration.rs` so filtered files are never yielded to the planner
- Wire into `mirror_planner.rs` so filtered files are excluded from delete lists
- Pass filter rules through `CopyConfig` or a new `TransferOptions` struct
- CLI flags added to `blit-cli/src/cli.rs` for copy/mirror/move commands

**Integration points:**
- `FileEnumerator` gains a `filter: Option<FilterRules>` field
- `MirrorPlanner` respects filters for both copy and delete decisions
- Remote push manifest only includes files passing the filter
- Remote pull applies filters client-side after receiving manifest

**Tests:**
- Unit: glob matching, precedence, size/age predicates, files-from parsing
- Integration: `blit mirror --exclude '*.tmp' --exclude-from .blitignore src/ dst/`

**Effort:** 3-4 days

### 5A.2 Bandwidth Limiting

**rclone reference:** `fs/accounting/token_bucket.go`

**Scope:**
- `--bwlimit RATE` flag (e.g., `10M`, `1G`, `500K`)
- Applied globally across all streams/workers
- Optional time-of-day schedule: `--bwlimit "08:00,1M 18:00,off"` (stretch goal)

**Design:**
- New module: `blit-core/src/bandwidth.rs` (~200 LOC)
- Token-bucket rate limiter wrapping `tokio::time` (no new dependency needed, but `governor` crate is an option for more sophistication)
- `BandwidthLimiter` struct with `async fn acquire(&self, bytes: usize)` that sleeps until tokens are available
- Shared via `Arc<BandwidthLimiter>` across all transfer workers
- Wire into `DataPlaneSession::send_file` and `local_worker` copy loop
- Limiter is optional -- `None` means unlimited (zero overhead in the hot path)

**Tests:**
- Unit: token bucket accuracy, burst handling
- Integration: verify transfer completes within expected time bounds at limited rate

**Effort:** 1-2 days

### 5A.3 Transfer Retries with Error Classification

**rclone reference:** `fs/fserrors/error.go`, `lib/pacer/pacer.go`

**Scope:**
- Classify errors as `Retryable`, `Fatal`, or `NoRetry`
- Retry retryable errors with exponential backoff (default: 3 retries, 1s/2s/4s)
- `--retries N` and `--retries-sleep DURATION` flags
- Per-file retry (don't abort entire transfer on one file's transient failure)

**Design:**
- New module: `blit-core/src/retry.rs` (~200 LOC)
- `ErrorClass` enum: `Retryable(source)`, `Fatal(source)`, `NoRetry(source)`
- `classify_error(e: &eyre::Report) -> ErrorClass` using pattern matching on IO error kinds:
  - `ConnectionReset`, `TimedOut`, `BrokenPipe` -> Retryable
  - `PermissionDenied`, `NotFound` -> NoRetry
  - `OutOfMemory` -> Fatal
- `retry_with_backoff<F, T>(f: F, config: &RetryConfig) -> Result<T>` generic retry wrapper
- Integrate into `local_worker.rs` copy loop and `DataPlaneSession` send/receive
- Failed files after exhausting retries are collected and reported in the transfer summary (not silent)

**Tests:**
- Unit: error classification, backoff timing, retry exhaustion
- Integration: inject transient IO error, verify retry succeeds

**Effort:** 2 days

---

## Phase 5B: Observability & Verification (P1)

Ship with or shortly after v1.0.

### 5B.1 Integrity Verification Command (`blit check`)

**rclone reference:** `fs/operations/check.go`

**Scope:**
- `blit check SOURCE DEST` -- compare two trees by size+mtime (default) or hash
- `--checksum` flag to compare by Blake3 hash
- Output categories: matching, differing, missing-on-source, missing-on-dest, errors
- `--one-way` to only report files missing on dest (useful for verifying a copy)
- Exit code: 0 = identical, 1 = differences found, 2 = errors
- `--json` output for scripting

**Design:**
- New CLI verb in `blit-cli/src/check.rs` (~300 LOC)
- Reuse `enumeration.rs` to walk both trees
- Reuse `mirror_planner.rs` comparison logic (already computes to_copy/to_delete/unchanged)
- Add optional Blake3 hash comparison pass for files that match size+mtime
- Support remote checking: `blit check server:/module/ ./local/` using existing gRPC manifest exchange
- Filter integration: `blit check --exclude '*.log' src/ dst/`

**Tests:**
- Unit: comparison logic for each category
- Integration: create known differences, verify output and exit code

**Effort:** 2-3 days

### 5B.2 Metrics Export

**rclone reference:** `fs/accounting/prometheus.go`

**Scope:**
- Optional Prometheus metrics endpoint on the daemon (`--metrics-addr :9090`)
- Metrics: bytes_transferred_total, files_transferred_total, active_transfers, transfer_errors_total, transfer_duration_seconds (histogram), bytes_per_second (gauge)
- Per-module labels where applicable
- Compatible with existing `perf_history` data

**Design:**
- Add `metrics-exporter-prometheus` crate to `blit-daemon`
- New module: `blit-daemon/src/metrics.rs` (~200 LOC)
- `TransferMetrics` struct with atomic counters, updated from existing progress reporting
- HTTP endpoint served on a separate Tokio task
- Optional: expose via gRPC as a new `Metrics` RPC for non-HTTP consumers

**Dependencies:** `metrics = "0.22"`, `metrics-exporter-prometheus = "0.13"`

**Tests:**
- Unit: counter increment, histogram recording
- Integration: start daemon with `--metrics-addr`, scrape endpoint, verify output

**Effort:** 2 days

### 5B.3 Streaming Checksums During Transfer

**rclone reference:** `fs/operations/copy.go` (inline hashing)

**Scope:**
- `--verify` flag: compute Blake3 hash during write and compare with source hash after transfer
- Zero additional disk reads -- hash is computed inline as bytes are written
- Report mismatches immediately (per-file) with option to re-transfer

**Design:**
- `ChecksummedWriter<W: AsyncWrite>` wrapper in `blit-core/src/checksum.rs`
- Wraps the destination file writer, feeds bytes to `blake3::Hasher` as they pass through
- After write completes, compare final hash with source-provided hash
- Wire into both local copy path and remote data plane receiver
- Source hash can be sent in `FileHeader` proto (field already exists but unused for streaming verification)

**Effort:** 2 days

---

## Phase 5C: Admin & UX Features (P2)

Nice-to-have before v1.0, can follow shortly after.

### 5C.1 Deduplication Command (`blit-utils dedupe`)

**rclone reference:** `fs/operations/dedupe.go`

**Scope:**
- `blit-utils dedupe PATH` -- find duplicate files by hash
- Strategies: `--strategy report` (default, just list), `--strategy newest` (keep newest), `--strategy oldest`, `--strategy interactive`
- Group duplicates by Blake3 hash, display with sizes and paths
- `--min-size SIZE` to skip tiny files
- `--json` output

**Design:**
- New verb in `blit-utils/src/dedupe.rs` (~300 LOC)
- Walk tree with `FileEnumerator`, hash files, group by hash
- For remote: stream hashes via `Find` RPC + daemon-side hashing (new `Hash` RPC or extend `Find` response)
- Interactive mode uses simple stdin prompts

**Effort:** 2-3 days

### 5C.2 Interactive TUI Explorer

**rclone reference:** `cmd/ncdu/`

**Scope:**
- `blit-utils ncdu [PATH]` -- interactive space explorer
- Navigate directories, sort by size/name/count
- Delete files/directories from within the TUI
- Works for both local paths and remote `server:/module/` paths

**Design:**
- Add `ratatui` crate to `blit-utils`
- New module: `blit-utils/src/ncdu.rs` (~500 LOC)
- Tree data structure populated from `enumeration.rs` (local) or `DiskUsage` RPC (remote)
- Keyboard navigation: arrows/enter to navigate, `d` to delete, `s` to sort, `q` to quit
- Size formatting reuses existing `util.rs` byte formatting

**Dependencies:** `ratatui = "0.28"`, `crossterm = "0.28"`

**Effort:** 3-4 days

### 5C.3 Parallel March (Lock-Step Tree Walk)

**rclone reference:** `fs/march/march.go`

**Scope:**
- Replace sequential enumerate-then-compare with lock-step parallel walk of source and destination
- Emit `(Option<SrcEntry>, Option<DstEntry>)` pairs as they're discovered
- Enables streaming comparison without holding full manifests in memory

**Design:**
- New module: `blit-core/src/march.rs` (~400 LOC)
- Two parallel `FileEnumerator` instances, each yielding sorted entries
- Merge-join algorithm: advance whichever side is lexicographically behind
- Yield `MarchPair::Match(src, dst)`, `MarchPair::SrcOnly(src)`, `MarchPair::DstOnly(dst)`
- Feeds directly into `mirror_planner.rs` without intermediate `Vec` allocation
- Respects `FilterRules` on both sides
- Benefit: O(1) memory for arbitrarily large trees (currently O(n))

**Integration:**
- `MirrorPlanner` gains an alternative `plan_streaming()` method using march
- Orchestrator selects streaming vs batch based on estimated tree size (>100K entries -> streaming)
- Remote variant: client and daemon each walk their side, exchange sorted manifests in chunks

**Effort:** 4-5 days

---

## Phase 5D: Advanced Features (P3)

Post-v1.0. Tracked here for architectural awareness.

### 5D.1 Bidirectional Sync (`blit bisync`)

**rclone reference:** `cmd/bisync/`

**Scope:**
- Two-way sync between source and destination
- Detect changes on both sides since last sync
- Conflict resolution: newest-wins (default), skip, rename-both
- Persistent state file (`.blit-bisync-state`) tracking last-sync timestamps and hashes

**Design considerations:**
- Requires change tracking on both sides (change journals help enormously here)
- State file stores per-file hash+mtime at last sync point
- Three-way comparison: last-sync state vs current-source vs current-dest
- Conflicts (both sides changed) need user-configurable policy
- This is the most complex feature in this plan -- defer until core is battle-tested

**Effort:** 10-15 days

### 5D.2 VFS / FUSE Mount

**rclone reference:** `vfs/`, `cmd/mount/`

**Scope:**
- `blit mount server:/module/ /mnt/blit` -- mount a remote daemon as local filesystem
- Read-only initially, read-write as stretch goal
- Cache layer for performance (write-back, eviction, range tracking)

**Design considerations:**
- Use `fuser` crate (Rust FUSE bindings) on Linux/macOS
- VFS layer translates filesystem ops to gRPC calls against daemon
- Cache layer similar to rclone's `vfscache` -- local temp files for read-ahead
- Windows support via WinFsp or ProjectedFS (complex, defer)

**Dependencies:** `fuser = "0.14"`

**Effort:** 15-20 days

### 5D.3 Serve Protocols

**rclone reference:** `cmd/serve/`

**Scope:**
- `blit-daemon serve http` -- expose modules via HTTP with directory listing
- `blit-daemon serve webdav` -- WebDAV protocol for broader client compatibility
- Enables integration with tools that don't speak gRPC

**Design considerations:**
- Layer on top of existing daemon module/export system
- HTTP server via `axum` or `hyper` (already in tonic's dependency tree)
- WebDAV via `dav-server` crate or custom implementation
- Authentication reuses daemon token system

**Effort:** 5-7 days per protocol

---

## Implementation Order & Dependencies

```
Phase 5A (P0 - before v1.0)
  5A.1 Filtering ──────────┐
  5A.2 Bandwidth Limiting   ├── independent, can parallelize
  5A.3 Retries + Errors ───┘

Phase 5B (P1 - with v1.0)
  5B.1 Check command ───── depends on 5A.1 (filter integration)
  5B.2 Metrics export ──── independent
  5B.3 Streaming checksums  independent

Phase 5C (P2 - shortly after v1.0)
  5C.1 Dedupe ──────────── independent
  5C.2 ncdu TUI ────────── independent
  5C.3 Parallel march ──── depends on 5A.1 (filters in march)

Phase 5D (P3 - post v1.0)
  5D.1 Bisync ─────────── depends on 5A.1, 5C.3
  5D.2 VFS/FUSE ────────── depends on daemon stability
  5D.3 Serve protocols ─── depends on daemon stability
```

## New Dependencies Summary

| Crate | Used by | Purpose |
|-------|---------|---------|
| `governor` | blit-core (optional) | Token-bucket rate limiting |
| `metrics` + `metrics-exporter-prometheus` | blit-daemon | Prometheus metrics |
| `ratatui` + `crossterm` | blit-utils | Interactive TUI |
| `fuser` | blit-core (optional, P3) | FUSE mount |

All Phase 5A features require **zero new dependencies** -- they build on `globset`, `tokio::time`, and existing error types.

## Changes to Existing Modules

| Module | Changes |
|--------|---------|
| `lib.rs` | Add `pub mod filter`, `pub mod bandwidth`, `pub mod retry`, `pub mod march` |
| `CopyConfig` | Add `filter: Option<FilterRules>`, `bwlimit: Option<u64>`, `retry: RetryConfig`, `verify: bool` |
| `enumeration.rs` | Accept `FilterRules`, skip non-matching entries during walk |
| `mirror_planner.rs` | Respect filters for both copy and delete lists |
| `local_worker.rs` | Wire bandwidth limiter and retry wrapper around copy calls |
| `remote/transfer/data_plane.rs` | Wire bandwidth limiter into send/receive paths |
| `orchestrator/orchestrator.rs` | Pass new config fields through to workers |
| `blit-cli/src/cli.rs` | Add --exclude, --include, --bwlimit, --retries, --verify, check verb |
| `blit-utils/src/cli.rs` | Add dedupe and ncdu verbs |
| `proto/blit.proto` | Add `Hash` RPC (for remote dedupe), extend `FileHeader` with optional hash field |

## Risks & Mitigations

- **Filter precedence bugs:** Use rsync/rclone's well-documented first-match-wins semantics. Port their test cases as unit tests.
- **Bandwidth limiter accuracy:** Token bucket can burst. Accept ~10% overshoot; document that limits are approximate (same as rclone).
- **March complexity:** Lock-step walk with filters, symlinks, and Unicode normalization is subtle. Start with the simple sorted-merge case; add edge cases incrementally.
- **Scope creep:** Each section above is scoped to a single module with a line count estimate. If a module exceeds 500 LOC, split before continuing.
- **AI manageability:** All new modules target <400 LOC. Complex features (bisync, VFS) are deferred to Phase 5D where they can be broken into sub-plans like REMOTE_TRANSFER_PARITY.md.

## What This Plan Replaces

- **PERFORMANCE_ROADMAP.md Part 3** (Enterprise Features) is partially superseded:
  - Resumable transfers: already implemented (Phase 4, TODO.md)
  - Bandwidth limiting: moved here as 5A.2
  - Transfer retries: moved here as 5A.3
  - Server-side copy: already implemented as remote-to-remote transfers
  - Checksum verification: split between 5B.1 (check command) and 5B.3 (streaming checksums)
  - Progress webhooks: deferred; daemon metrics (5B.2) covers the monitoring use case
- **PERFORMANCE_ROADMAP.md Parts 1-2, 4** (buffer pool, async pipeline, TCP tuning, io_uring) remain the authoritative performance plan and are **not** affected by this document.

---

**Last Updated:** 2026-03-19
**Status:** Proposed
**Depends on:** Phase 4 completion (production hardening baseline)
