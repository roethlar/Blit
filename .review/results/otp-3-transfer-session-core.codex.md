Reading additional input from stdin...
OpenAI Codex v0.142.5
--------
workdir: /home/michael/dev/Blit
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f3363-daad-7d42-b2d6-9610bf7b36e2
--------
user
Review the diff of commit ef9ffa1 (run: git show ef9ffa1). It implements otp-3 'TransferSession core (role-parameterized, in-process transport)' — slice 3 of docs/plan/ONE_TRANSFER_PATH.md, against the wire/session contract docs/TRANSFER_SESSION.md and the implementation record .review/findings/otp-3-transfer-session-core.md (read all three). Check: correctness regressions; the slice's acceptance criteria (role-parameterized state machine over the existing engine, in-process transport, both role assignments over the same fixtures with identical need lists and summaries, build-mismatch refusal per D-2026-07-05-2, protocol-violation fail-fast); FAST/SIMPLE/RELIABLE; the invariants relevant to transfer code (byte-identical results, bounded buffering / no deadlock on the bidirectional bounded transport, byte-accounting, cancellation N/A until otp-4); no contradiction with docs/DECISIONS.md; and that the test count did not drop (1484 -> 1500). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the target commit and the referenced contract/plan/record docs first, then cross-check tests and decisions against the diff. No files will be changed.
exec
/usr/bin/zsh -lc 'git show ef9ffa1' in /home/michael/dev/Blit
 succeeded in 0ms:
commit ef9ffa14f57a3e0e64904d6a43564d8b3c7f9f49
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 13:46:33 2026 -0400

    otp-3: TransferSession core, role-parameterized, in-process transport
    
    One session state machine (run_source/run_destination) over a
    FrameTransport per docs/TRANSFER_SESSION.md: shared hello/open/accept
    phase with the strict same-build handshake (D-2026-07-05-2),
    streaming manifest, destination-owned incremental diff
    (manifest::header_transfer_status extraction), in-stream record
    grammar with fail-fast violations, destination-computed summary.
    Role suite runs every fixture under both initiator layouts and pins
    identical need sets, summaries, and byte-identical trees — the
    owner's invariance property (D-2026-07-05-1) as tests.
    
    Suite 1484 -> 1500. Gate: fmt/clippy/test clean.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

diff --git a/.review/findings/otp-3-transfer-session-core.md b/.review/findings/otp-3-transfer-session-core.md
index 83f0015..c414a31 100644
--- a/.review/findings/otp-3-transfer-session-core.md
+++ b/.review/findings/otp-3-transfer-session-core.md
@@ -1,76 +1,170 @@
 # otp-3 — TransferSession core (role-parameterized, in-process)
 
 **Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-3.
-**Status**: scoped — approach recorded 2026-07-05, implementation next.
+**Status**: implemented — awaiting codex review.
 **Contract**: `docs/TRANSFER_SESSION.md` (post-review, `f861579`).
 
-## Scope (what otp-3 proves)
+## What
 
-The role-parameterized session state machine exists in blit-core and
-moves real bytes in-process with the roles swappable over the same
-fixtures — the owner's invariance property enters the test suite
-here. In otp-3 the byte carrier is the in-stream frame grammar only
-(the TCP data plane + daemon serving land at otp-4); mirror, filters
-beyond passthrough, resume, StallGuard/jobs wiring are later slices
-per the plan.
+The unified session state machine exists in blit-core and moves real
+bytes in-process, with the roles swappable over the same fixtures.
+`run_source` / `run_destination` implement the contract's HELLO →
+OPEN/ACCEPT → role-lane phases over a `FrameTransport`; the byte
+carrier is the in-stream frame grammar (file records + tar-shard
+records, strictly serialized, fail-fast). The owner's invariance
+property is now in the test suite: every fixture runs under both
+initiator layouts and must produce the identical need-list set,
+identical summary, and byte-identical destination tree.
 
-## Approach (surveyed 2026-07-05)
+## Approach (as implemented)
 
-- New module `crates/blit-core/src/transfer_session/`:
-  - `transport.rs` — `FrameTransport` trait (`send(TransferFrame)`,
-    `recv() -> Option<TransferFrame>`) + `in_process_pair()` built on
-    bounded mpsc channels. otp-4 adds the gRPC-backed transport;
-    otp-11 reuses the in-process one for local transfers.
-  - `mod.rs` — `run_source(cfg, transport, Arc<dyn TransferSource>)`
-    and `run_destination(cfg, transport, sink)` drivers, plus the
-    shared hello/open/accept phase code (one implementation, both
-    roles call it).
-  - Hello: `session_build_id()` composed compile-time
-    (`CARGO_PKG_VERSION` + `BLIT_GIT_SHA` emitted by blit-core's
-    existing build script, fallback "unknown") + `CONTRACT_VERSION`
-    const; exact-match check per contract, mismatch →
-    `SessionError{BUILD_MISMATCH}` naming both ids.
-- SOURCE driver: `TransferSource::scan` streams headers →
-  `manifest_entry` frames (immediate start); need batches consumed
-  incrementally; payloads planned via the existing
-  `diff_planner::plan_push_payloads` on needed headers; in-stream
-  records emitted per the contract grammar (file records:
-  `file_begin` + `file_data`×N, completion at exactly `header.size`;
-  tar records via the existing tar planner; payload records only
-  after `ManifestComplete` per the carrier rule); `SourceDone`; await
-  `TransferSummary`.
-- DESTINATION driver: manifest entries diffed incrementally against
-  the destination root using the `diff_planner::filter_unchanged`
-  predicate (the existing single owner of compare_mode semantics —
-  reused, not duplicated); `NeedBatch` emission with the engine's
-  existing batching; `NeedComplete` only after ManifestComplete +
-  all entries diffed (contract); in-stream records reassembled and
-  written through `FsTransferSink::write_file_stream` (file records)
-  and the existing tar-safety unpack path (tar records); summary
-  computed destination-side.
-- Tests (all in-process, both role assignments over the same
-  fixtures — the suite runs each case twice via a role parameter):
-  build-id mismatch refusal; small tree byte-identical; tiny-file
-  tree (tar-shard records) byte-identical; incremental (pre-seeded
-  destination) transfers only the need list; empty need list
-  completes clean; protocol-violation fail-fast (payload record
-  before ManifestComplete). Role-swap equality: for each fixture,
-  the need-list set and summary counts must be IDENTICAL under both
-  role assignments — the first executable form of the owner's
-  invariant.
+- `crates/blit-core/src/transfer_session/transport.rs` —
+  `FrameTx`/`FrameRx` halves + `FrameTransport` (splittable) +
+  `in_process_pair()` on bounded mpsc (64 frames/direction).
+- `crates/blit-core/src/transfer_session/mod.rs` —
+  - `session_build_id()` = `CARGO_PKG_VERSION+BLIT_GIT_SHA[.dirty]`
+    (build.rs emits the sha; rerun-if-changed on `.git/HEAD` +
+    `.git/refs`; dirty flag sampled at build-script time, best-effort
+    by nature). `CONTRACT_VERSION = 1`. Exact-match hello both ways;
+    mismatch → `SessionError{BUILD_MISMATCH}` naming both ids
+    (D-2026-07-05-2).
+  - `establish()` — ONE hello/open/accept implementation both role
+    drivers call. Responder-side capability validation refuses what
+    later slices implement (mirror → otp-6, filters → otp-6, resume →
+    otp-7) with a `SessionError` instead of accepting — fail-fast, no
+    silent ignoring. Receiver capacity travels DEST→SOURCE at setup
+    (open when initiator is DEST, accept when responder is), consumed
+    from otp-4 when the dial attaches.
+  - SOURCE driver: split into a send half and a dedicated receive
+    half (deadlock-freedom: the transport is bounded both ways, so a
+    single loop that blocks on send while the peer blocks on its own
+    send would deadlock — the recv half always drains). Needs are
+    validated against the sent-manifest map (unknown / duplicate /
+    resume-flagged → PROTOCOL_VIOLATION), which bounds the internal
+    event queue by the source's own manifest size. Payloads plan per
+    accumulated need batch via `diff_planner::plan_push_payloads`,
+    emit as file records (`file_begin` + `file_data`×N, completion at
+    exactly `header.size`) and tar records (existing tar builder via
+    `prepare_payload`), only after `ManifestComplete` (in-stream
+    carrier rule). `SourceDone` only after `NeedComplete` + queue
+    drained; then awaits the destination's summary.
+  - DESTINATION driver: sequential frame loop (its sends can't
+    deadlock because the source's recv half always drains). Manifest
+    entries buffer into 128-entry chunks (w4-4 rationale) and
+    stat+compare on the blocking pool; need batches stream back
+    mid-manifest; `NeedComplete` only after ManifestComplete + all
+    entries diffed. File records write through
+    `FsTransferSink::write_file_stream` fed by a bounded
+    `tokio::io::duplex` pipe (256 KiB); tar records buffer to exactly
+    `archive_size` (≤ `MAX_TAR_SHARD_BYTES`, `try_reserve_exact`) and
+    unpack through the existing tar-safety path
+    (`write_payload(TarShard)`). Grammar violations (payload before
+    ManifestComplete, record interleave/overrun/short-complete,
+    payload not on the need list, `SourceDone` with outstanding
+    needs, resume/resize frames in an otp-3 session) →
+    `SessionError{PROTOCOL_VIOLATION}` + abort. Diff stats go through
+    the same canonical-containment chokepoint as sink writes
+    (R46-F3): an escaping manifest path is a violation, not a stat.
+  - Faults are `SessionFault` (wire code + message + both build ids +
+    peer_notified), carried in `eyre::Report` — tests downcast and
+    assert codes. An end that aborts sends the error frame first
+    unless the peer already knows.
 
-## Files (planned)
+### Deviations from the scoped approach (2026-07-05 survey)
+
+1. **Destination diff predicate**: the scoping note named
+   `diff_planner::filter_unchanged`, but that predicate stats BOTH
+   sides locally — impossible for a wire destination and a role-
+   separation leak in-process (otp-4 must be transport substitution,
+   not new choreography). The mode-aware header-vs-target owner that
+   already exists is `manifest::compare_manifests`; its per-entry
+   body is now extracted as `manifest::header_transfer_status`
+   (public), `compare_manifests` is refactored onto it, and the
+   session destination feeds it from a live stat. Single-owner intent
+   preserved; `From<ComparisonMode> for CompareMode` added alongside.
+2. **`DestinationOutcome`**: `run_destination` returns
+   `{summary, needed_paths}` rather than bare summary — the role
+   suite pins need-set equality across role assignments, which the
+   scoping called for but the driver didn't expose.
+3. `SessionEndpoint::Initiator` boxes its `SessionOpen`
+   (clippy large-enum-variant); `SessionEndpoint::initiator()`
+   constructor provided.
+
+## Files
 
 - `crates/blit-core/src/transfer_session/{mod.rs,transport.rs}` (new)
 - `crates/blit-core/src/lib.rs` (module export)
 - `crates/blit-core/build.rs` (BLIT_GIT_SHA emission)
+- `crates/blit-core/src/manifest.rs` (`header_transfer_status`
+  extraction + `From<ComparisonMode>`; `compare_manifests` behavior
+  unchanged)
+- `crates/blit-core/Cargo.toml` (filetime added to dev-deps for the
+  fixture suite)
 - `crates/blit-core/tests/transfer_session_roles.rs` (new, the
-  role-parameterized fixture suite)
+  role-parameterized suite)
+
+## Tests
+
+Suite 1484 → 1500 (+16; count never dropped). New:
+
+- `transfer_session_roles.rs` (12): small mixed tree (multi-chunk
+  3 MiB file, empty file, spaced/nested names) byte-identical under
+  both initiators with identical need sets + summaries; 200-file
+  force-tar tree likewise (tar record grammar both layouts);
+  incremental pre-seeded destination needs exactly {changed,
+  missing}; identical pre-seeded tree yields empty need list and
+  0/0 summary; mtime preservation on streamed files; build-id
+  mismatch refused both ends under both initiator layouts (message
+  names both ids, no bytes move); contract-version mismatch refused;
+  mirror-enabled open refused with the otp-6 pointer; scripted-peer
+  violations fail fast (payload record before ManifestComplete, need
+  for never-manifested path, resume-flagged need, manifest entry
+  after ManifestComplete) with the error frame observed on the wire.
+- `transport.rs` (2): pair delivery both directions; closed-peer
+  semantics.
+- `mod.rs` (2): build-id shape; fault wire round-trip (perspective
+  swap included).
+
+Gate: `cargo fmt --check` ✓, `clippy --workspace --all-targets
+-D warnings` ✓, `cargo test --workspace` 1500/0 ✓.
 
-## Known gaps (carried into implementation)
+## Known gaps (carried forward)
 
-- Data plane, daemon serving, ActiveJobs/cancel, progress events:
-  otp-4. Mirror: otp-6. Resume: otp-7. Delegated: otp-9.
-- The in-process transport intentionally exercises the same frame
-  grammar the wire will carry, so otp-4 is transport substitution,
-  not new choreography.
+- **SizeMtime semantic divergence, decided at otp-4/5 parity**: the
+  session inherits `manifest::compare_file`'s Default arm (transfer
+  when src NEWER; skip when target same-age-or-newer) — today's
+  pull_sync semantic. Today's push daemon uses exact size+mtime
+  equality instead, so a destination file with newer mtime but
+  different content re-transfers under old push and is skipped by
+  the session. The otp-4 A/B parity pins against old push will
+  surface this; picking the unified semantic (and whether
+  `compare_file`'s Default arm changes) is that slice's recorded
+  decision. otp-3 deliberately did not change live pull_sync
+  behavior by editing the shared arm.
+- Checksum compare mode transfers everything when headers carry no
+  checksum (manifest enumeration never populates it today) — the
+  conservative arm of `compare_file`, parity with today's push.
+  Whether the session grows source-side checksum population is a
+  parity-slice call (otp-4/5).
+- Strict `SourceDone`: a needed file that vanishes source-side
+  mid-transfer faults the session (`INTERNAL` on read failure /
+  EOF-short). Old push tolerates and skips (`check_availability`,
+  unreadable list). The contract has no "source skipped these"
+  notification yet; if parity requires tolerance, that's a contract
+  addendum at otp-4/5, not silent skipping.
+- Need-batch cadence knobs (the FileListBatcher's 5 ms/64 KiB early
+  flush + 25 ms max delay) are not replicated in-process; batches
+  flush per 128-entry diff chunk and at ManifestComplete. Cadence
+  matters when a real wire + mid-manifest data-plane spin-up exist —
+  otp-4.
+- Single-file source roots (`relative_path = ""` wire form) are
+  untested against the session; parity slices own that edge.
+- `require_complete_scan` + `ManifestComplete.scan_complete` travel
+  the wire but gate nothing until mirror (otp-6).
+- Resize frames on the in-stream carrier are treated as
+  PROTOCOL_VIOLATION (no data plane exists to resize in otp-3); the
+  frame table marks them any-phase for sessions WITH a plane —
+  otp-4 wires the real semantics.
+- In-process transport caps frames, not bytes (64 × ≤1 MiB payload
+  frames ≈ 64 MiB/direction worst case). Fine for tests and local
+  use; the wire carrier has HTTP/2 byte-level flow control.
diff --git a/crates/blit-core/Cargo.toml b/crates/blit-core/Cargo.toml
index 38f37ee..c0772f7 100644
--- a/crates/blit-core/Cargo.toml
+++ b/crates/blit-core/Cargo.toml
@@ -69,6 +69,9 @@ protoc-bin-vendored = "3"
 tempfile = "3"
 # ue-r2-1e: paused-clock testing for the dial tuner (dev-only).
 tokio = { version = "1", features = ["full", "test-util"] }
+# otp-3: deterministic fixture mtimes in the role-parameterized
+# session suite (regular dep already; repeated here for tests/).
+filetime = "0.2"
 
 [target.'cfg(windows)'.dependencies]
 windows = { version = "0.62", features = [
diff --git a/crates/blit-core/build.rs b/crates/blit-core/build.rs
index 623f64d..f1a5435 100644
--- a/crates/blit-core/build.rs
+++ b/crates/blit-core/build.rs
@@ -1,5 +1,48 @@
 use protoc_bin_vendored::protoc_bin_path;
 use std::path::PathBuf;
+use std::process::Command;
+
+/// Best-effort git identity for the same-build session handshake
+/// (D-2026-07-05-2, docs/TRANSFER_SESSION.md §Invariants 2). Returns
+/// "<short sha>[.dirty]" or "unknown" when git/repo is unavailable
+/// (e.g. building from a source tarball).
+fn git_build_suffix(manifest_dir: &std::path::Path) -> String {
+    let run = |args: &[&str]| -> Option<String> {
+        let out = Command::new("git")
+            .args(args)
+            .current_dir(manifest_dir)
+            .output()
+            .ok()?;
+        if !out.status.success() {
+            return None;
+        }
+        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
+    };
+
+    let Some(sha) = run(&["rev-parse", "--short=12", "HEAD"]).filter(|s| !s.is_empty()) else {
+        return "unknown".to_string();
+    };
+
+    // Track HEAD so the sha refreshes on commit/branch switch without
+    // rebuilding on every unrelated file change. Dirty state is
+    // best-effort: it is sampled when the build script runs, so a
+    // tree that becomes dirty without touching HEAD can keep a stale
+    // clean flag until the next rebuild — the sha component is the
+    // load-bearing part of the handshake.
+    if let Some(git_dir) = run(&["rev-parse", "--absolute-git-dir"]) {
+        println!("cargo:rerun-if-changed={git_dir}/HEAD");
+        println!("cargo:rerun-if-changed={git_dir}/refs");
+    }
+
+    let dirty = run(&["status", "--porcelain"])
+        .map(|s| !s.is_empty())
+        .unwrap_or(false);
+    if dirty {
+        format!("{sha}.dirty")
+    } else {
+        sha
+    }
+}
 
 fn main() -> Result<(), Box<dyn std::error::Error>> {
     let protoc_path = protoc_bin_path()?;
@@ -10,6 +53,10 @@ fn main() -> Result<(), Box<dyn std::error::Error>> {
     let proto_file = proto_dir.join("blit.proto");
 
     println!("cargo:rerun-if-changed={}", proto_file.display());
+    println!(
+        "cargo:rustc-env=BLIT_GIT_SHA={}",
+        git_build_suffix(&manifest_dir)
+    );
 
     tonic_prost_build::configure()
         .build_server(true)
diff --git a/crates/blit-core/src/lib.rs b/crates/blit-core/src/lib.rs
index 828a43b..0d2e631 100644
--- a/crates/blit-core/src/lib.rs
+++ b/crates/blit-core/src/lib.rs
@@ -23,6 +23,7 @@ pub mod remote;
 pub mod stderr_log;
 pub mod tar_stream;
 pub mod transfer_plan;
+pub mod transfer_session;
 #[cfg(windows)]
 pub mod win_fs;
 pub mod zero_copy;
diff --git a/crates/blit-core/src/manifest.rs b/crates/blit-core/src/manifest.rs
index a71690d..cae239c 100644
--- a/crates/blit-core/src/manifest.rs
+++ b/crates/blit-core/src/manifest.rs
@@ -3,7 +3,7 @@
 //! This module provides manifest comparison logic used by both push and pull
 //! operations to determine which files need to be transferred.
 
-use crate::generated::FileHeader;
+use crate::generated::{ComparisonMode, FileHeader};
 use std::collections::HashMap;
 
 /// How to compare files between source and target.
@@ -23,6 +23,21 @@ pub enum CompareMode {
     Checksum,
 }
 
+/// Canonical mapping from the wire enum. `Unspecified` folds to the
+/// historical default, matching `NormalizedTransferOperation` and the
+/// diff planner's defensive handling.
+impl From<ComparisonMode> for CompareMode {
+    fn from(mode: ComparisonMode) -> Self {
+        match mode {
+            ComparisonMode::Checksum => CompareMode::Checksum,
+            ComparisonMode::SizeOnly => CompareMode::SizeOnly,
+            ComparisonMode::IgnoreTimes => CompareMode::IgnoreTimes,
+            ComparisonMode::Force => CompareMode::Force,
+            ComparisonMode::Unspecified | ComparisonMode::SizeMtime => CompareMode::Default,
+        }
+    }
+}
+
 /// Status of a file after manifest comparison.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum FileStatus {
@@ -104,24 +119,13 @@ pub fn compare_manifests(
 
     // Compare each source file against target
     for src in source {
-        let status = match target_map.get(src.relative_path.as_str()) {
-            None => FileStatus::New,
-            Some(&(target_size, target_mtime, target_checksum)) => {
-                // File exists on target
-                if options.ignore_existing {
-                    // Skip all existing files regardless of differences
-                    FileStatus::SkippedExisting
-                } else {
-                    compare_file(
-                        src,
-                        target_size,
-                        target_mtime,
-                        target_checksum,
-                        options.mode,
-                    )
-                }
-            }
-        };
+        let status = header_transfer_status(
+            src,
+            target_map
+                .get(src.relative_path.as_str())
+                .map(|&(size, mtime, checksum)| (size, mtime, checksum)),
+            options,
+        );
 
         if status == FileStatus::New || status == FileStatus::Modified {
             diff.bytes_to_transfer += src.size;
@@ -148,6 +152,39 @@ pub fn compare_manifests(
     diff
 }
 
+/// Per-entry form of [`compare_manifests`]: status of one source
+/// header against the target's view of the same path —
+/// `Some((size, mtime_seconds, checksum))` when the target has the
+/// path, `None` when it doesn't. This is the single owner of the
+/// mode-aware header-vs-target decision; `compare_manifests` and the
+/// unified `transfer_session` destination diff (which stats its own
+/// filesystem per entry instead of materializing a full target
+/// manifest) both call it.
+pub fn header_transfer_status(
+    src: &FileHeader,
+    target: Option<(u64, i64, &[u8])>,
+    options: &CompareOptions,
+) -> FileStatus {
+    match target {
+        None => FileStatus::New,
+        Some((target_size, target_mtime, target_checksum)) => {
+            // File exists on target
+            if options.ignore_existing {
+                // Skip all existing files regardless of differences
+                FileStatus::SkippedExisting
+            } else {
+                compare_file(
+                    src,
+                    target_size,
+                    target_mtime,
+                    target_checksum,
+                    options.mode,
+                )
+            }
+        }
+    }
+}
+
 /// Compare a single file using the specified comparison mode.
 fn compare_file(
     src: &FileHeader,
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
new file mode 100644
index 0000000..7345b6a
--- /dev/null
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -0,0 +1,1262 @@
+//! Unified transfer session — the ONE block of transfer code
+//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
+//!
+//! A transfer has a SOURCE role and a DESTINATION role; which end
+//! initiated and which CLI verb was used select roles, never code.
+//! Both roles run the drivers below over a [`transport::FrameTransport`];
+//! the wire contract they implement — phases, frame table, record
+//! grammar, error semantics — is `docs/TRANSFER_SESSION.md` (otp-1).
+//!
+//! otp-3 scope: the role-parameterized state machine over the existing
+//! engine with the in-process transport and the in-stream byte
+//! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
+//! progress wiring land at otp-4; mirror otp-6; resume otp-7;
+//! delegated otp-9 (see the slice list in the plan).
+
+pub mod transport;
+
+use std::collections::{HashMap, HashSet};
+use std::fmt;
+use std::path::{Path, PathBuf};
+use std::sync::{Arc, Mutex as StdMutex};
+
+use eyre::Result;
+use tokio::io::{AsyncReadExt, AsyncWriteExt};
+use tokio::sync::mpsc;
+
+use crate::generated::transfer_frame::Frame;
+use crate::generated::{
+    session_error, ComparisonMode, FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch,
+    NeedComplete, NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
+    TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
+};
+use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
+use crate::remote::transfer::diff_planner;
+use crate::remote::transfer::payload::PreparedPayload;
+use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
+use crate::remote::transfer::source::TransferSource;
+use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
+use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
+use crate::transfer_plan::PlanOptions;
+use transport::{FrameRx, FrameTransport, FrameTx};
+
+/// Belt-and-braces wire-shape version, bumped on any change to the
+/// frame set or grammar. Exchanged (and exact-matched) in
+/// `SessionHello` alongside the build id (D-2026-07-05-2).
+pub const CONTRACT_VERSION: u32 = 1;
+
+/// Payload chunk size on the in-stream carrier. Same unit the gRPC
+/// control plane uses today; the data plane (otp-4) has its own.
+const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;
+
+/// Manifest entries buffered per destination diff batch. Mirrors the
+/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
+/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
+/// blocking pool instead of inline per entry.
+const DEST_DIFF_CHUNK: usize = 128;
+
+/// Buffer of the in-memory pipe that feeds wire file-record bytes
+/// into `FsTransferSink::write_file_stream`. Bounds destination-side
+/// buffering per file record.
+const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
+
+/// This build's session identity: `<crate version>+<git sha>[.dirty]`
+/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
+/// "unknown" when git was unavailable at compile time.
+pub fn session_build_id() -> &'static str {
+    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
+}
+
+/// The identity this end presents in `SessionHello`. Defaults to the
+/// real compile-time identity; tests inject mismatches.
+#[derive(Debug, Clone)]
+pub struct HelloConfig {
+    pub build_id: String,
+    pub contract_version: u32,
+}
+
+impl Default for HelloConfig {
+    fn default() -> Self {
+        Self {
+            build_id: session_build_id().to_string(),
+            contract_version: CONTRACT_VERSION,
+        }
+    }
+}
+
+/// Which handshake part this end plays. Orthogonal to role: all four
+/// initiator/role combinations run the same state machine (contract
+/// §Invariants 3).
+pub enum SessionEndpoint {
+    /// This end opened the transport; it sends `SessionOpen`.
+    /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
+    Initiator { open: Box<SessionOpen> },
+    /// This end answers `SessionOpen` with `SessionAccept`. Daemon
+    /// module/path/read-only validation attaches here at otp-4.
+    Responder,
+}
+
+impl SessionEndpoint {
+    /// Convenience constructor so callers don't spell the `Box`.
+    pub fn initiator(open: SessionOpen) -> Self {
+        SessionEndpoint::Initiator {
+            open: Box::new(open),
+        }
+    }
+}
+
+pub struct SourceSessionConfig {
+    pub hello: HelloConfig,
+    pub endpoint: SessionEndpoint,
+    /// Engine planner knobs (tar/large/raw thresholds). Local to the
+    /// source end — strategy selection is planner-owned and never
+    /// crosses the wire (contract §Transport selection).
+    pub plan_options: PlanOptions,
+}
+
+pub struct DestinationSessionConfig {
+    pub hello: HelloConfig,
+    pub endpoint: SessionEndpoint,
+}
+
+/// A session-terminating fault: either end refusing, aborting, or
+/// catching the peer in a protocol violation. Carried as the error
+/// payload of the drivers' `eyre::Report`s — downcast to inspect the
+/// wire code.
+#[derive(Debug, Clone)]
+pub struct SessionFault {
+    pub code: session_error::Code,
+    pub message: String,
+    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
+    /// which end is stale (contract §Errors).
+    pub local_build_id: String,
+    pub peer_build_id: String,
+    /// True when the peer already knows about this fault — it sent
+    /// the `SessionError` frame itself, or this end already emitted
+    /// one. Drivers must not send another.
+    pub peer_notified: bool,
+}
+
+impl SessionFault {
+    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
+        Self {
+            code,
+            message: message.into(),
+            local_build_id: String::new(),
+            peer_build_id: String::new(),
+            peer_notified: false,
+        }
+    }
+
+    fn protocol_violation(message: impl Into<String>) -> Self {
+        Self::new(session_error::Code::ProtocolViolation, message)
+    }
+
+    fn internal(message: impl Into<String>) -> Self {
+        Self::new(session_error::Code::Internal, message)
+    }
+
+    fn from_wire(err: SessionError) -> Self {
+        Self {
+            code: session_error::Code::try_from(err.code)
+                .unwrap_or(session_error::Code::SessionErrorUnspecified),
+            message: err.message,
+            // The peer reports its view: its "local" is our peer.
+            local_build_id: err.peer_build_id,
+            peer_build_id: err.local_build_id,
+            peer_notified: true,
+        }
+    }
+
+    fn to_wire(&self) -> SessionError {
+        SessionError {
+            code: self.code as i32,
+            message: self.message.clone(),
+            local_build_id: self.local_build_id.clone(),
+            peer_build_id: self.peer_build_id.clone(),
+        }
+    }
+}
+
+impl fmt::Display for SessionFault {
+    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
+        write!(f, "session {}: {}", self.code.as_str_name(), self.message)
+    }
+}
+
+impl std::error::Error for SessionFault {}
+
+/// Downcast a driver-internal error back to its fault, wrapping
+/// non-fault failures (fs errors, planner errors, transport failures)
+/// as INTERNAL — an end that aborts says why before closing.
+fn fault_from_report(report: eyre::Report) -> SessionFault {
+    match report.downcast::<SessionFault>() {
+        Ok(fault) => fault,
+        Err(other) => SessionFault::internal(format!("{other:#}")),
+    }
+}
+
+fn frame(f: Frame) -> TransferFrame {
+    TransferFrame { frame: Some(f) }
+}
+
+fn error_frame(fault: &SessionFault) -> TransferFrame {
+    frame(Frame::Error(fault.to_wire()))
+}
+
+/// Short frame identifier for protocol-violation messages.
+fn frame_name(f: &Option<Frame>) -> &'static str {
+    match f {
+        Some(Frame::Hello(_)) => "SessionHello",
+        Some(Frame::Open(_)) => "SessionOpen",
+        Some(Frame::Accept(_)) => "SessionAccept",
+        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
+        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
+        Some(Frame::NeedBatch(_)) => "NeedBatch",
+        Some(Frame::NeedComplete(_)) => "NeedComplete",
+        Some(Frame::BlockHashes(_)) => "BlockHashList",
+        Some(Frame::FileBegin(_)) => "FileBegin",
+        Some(Frame::FileData(_)) => "FileData",
+        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
+        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
+        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
+        Some(Frame::Block(_)) => "BlockTransfer",
+        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
+        Some(Frame::Resize(_)) => "DataPlaneResize",
+        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
+        Some(Frame::SourceDone(_)) => "SourceDone",
+        Some(Frame::Summary(_)) => "TransferSummary",
+        Some(Frame::Error(_)) => "SessionError",
+        None => "empty frame",
+    }
+}
+
+fn complement(role: TransferRole) -> TransferRole {
+    match role {
+        TransferRole::Source => TransferRole::Destination,
+        TransferRole::Destination => TransferRole::Source,
+        TransferRole::Unspecified => TransferRole::Unspecified,
+    }
+}
+
+/// Per-role capability check of the operation a `SessionOpen`
+/// describes. otp-3 refuses what later slices implement rather than
+/// silently ignoring it (fail-fast; contract §Errors).
+type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
+
+fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
+    if open.resume.as_ref().is_some_and(|r| r.enabled) {
+        return Err(SessionFault::internal(
+            "resume is not implemented on the unified session yet (otp-7)",
+        ));
+    }
+    if open
+        .filter
+        .as_ref()
+        .is_some_and(|f| *f != FilterSpec::default())
+    {
+        return Err(SessionFault::internal(
+            "filters are not implemented on the unified session yet (otp-6)",
+        ));
+    }
+    Ok(())
+}
+
+fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
+    if open.mirror_enabled {
+        return Err(SessionFault::internal(
+            "mirror is not implemented on the unified session yet (otp-6)",
+        ));
+    }
+    if open.resume.as_ref().is_some_and(|r| r.enabled) {
+        return Err(SessionFault::internal(
+            "resume is not implemented on the unified session yet (otp-7)",
+        ));
+    }
+    Ok(())
+}
+
+/// Outcome of the HELLO + OPEN phases.
+struct Negotiated {
+    open: SessionOpen,
+    #[allow(dead_code)] // capacity/grant consumed from otp-4 on
+    accept: SessionAccept,
+}
+
+/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
+/// scoping requirement). Sends the refusal `SessionError` itself when
+/// it detects the fault locally; returned faults are `peer_notified`.
+async fn establish(
+    transport: &mut FrameTransport,
+    hello: &HelloConfig,
+    endpoint: &SessionEndpoint,
+    local_role: TransferRole,
+    validate_open: &OpenValidator,
+) -> Result<Negotiated> {
+    // HELLO both ways, exact match (D-2026-07-05-2). First frame each
+    // direction; no ordering between the two directions.
+    transport
+        .send(frame(Frame::Hello(SessionHello {
+            build_id: hello.build_id.clone(),
+            contract_version: hello.contract_version,
+        })))
+        .await?;
+
+    let peer_hello = match expect_frame(transport).await? {
+        Frame::Hello(h) => h,
+        other => {
+            return Err(notify_and_wrap(
+                transport,
+                SessionFault::protocol_violation(format!(
+                    "expected SessionHello, got {}",
+                    frame_name(&Some(other))
+                )),
+            )
+            .await)
+        }
+    };
+
+    if peer_hello.build_id != hello.build_id
+        || peer_hello.contract_version != hello.contract_version
+    {
+        let fault = SessionFault {
+            code: session_error::Code::BuildMismatch,
+            message: format!(
+                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
+                hello.build_id, hello.contract_version,
+                peer_hello.build_id, peer_hello.contract_version,
+            ),
+            local_build_id: hello.build_id.clone(),
+            peer_build_id: peer_hello.build_id.clone(),
+            peer_notified: false,
+        };
+        return Err(notify_and_wrap(transport, fault).await);
+    }
+
+    match endpoint {
+        SessionEndpoint::Initiator { open } => {
+            let open = open.as_ref().clone();
+            transport.send(frame(Frame::Open(open.clone()))).await?;
+            let accept = match expect_frame(transport).await? {
+                Frame::Accept(a) => a,
+                other => {
+                    return Err(notify_and_wrap(
+                        transport,
+                        SessionFault::protocol_violation(format!(
+                            "expected SessionAccept, got {}",
+                            frame_name(&Some(other))
+                        )),
+                    )
+                    .await)
+                }
+            };
+            Ok(Negotiated { open, accept })
+        }
+        SessionEndpoint::Responder => {
+            let open = match expect_frame(transport).await? {
+                Frame::Open(o) => o,
+                other => {
+                    return Err(notify_and_wrap(
+                        transport,
+                        SessionFault::protocol_violation(format!(
+                            "expected SessionOpen, got {}",
+                            frame_name(&Some(other))
+                        )),
+                    )
+                    .await)
+                }
+            };
+            // The initiator declares ITS role; this responder end must
+            // hold the complement.
+            let declared =
+                TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
+            if declared != complement(local_role) {
+                return Err(notify_and_wrap(
+                    transport,
+                    SessionFault::protocol_violation(format!(
+                        "initiator declared role {} but this responder is {}",
+                        declared.as_str_name(),
+                        local_role.as_str_name()
+                    )),
+                )
+                .await);
+            }
+            if let Err(fault) = validate_open(&open) {
+                // Refusal is a SessionError instead of SessionAccept,
+                // never a silent close (contract §Phase state machine).
+                return Err(notify_and_wrap(transport, fault).await);
+            }
+            let accept = SessionAccept {
+                // The byte RECEIVER advertises capacity at session
+                // open (D-2026-06-20-1/-2); consumed by the dial when
+                // the data plane lands (otp-4).
+                receiver_capacity: if local_role == TransferRole::Destination {
+                    Some(crate::engine::local_receiver_capacity())
+                } else {
+                    None
+                },
+                // No grant = in-stream byte carrier, otp-3's only one.
+                data_plane: None,
+            };
+            transport.send(frame(Frame::Accept(accept.clone()))).await?;
+            Ok(Negotiated { open, accept })
+        }
+    }
+}
+
+/// Receive one frame during establish; peer errors and closes become
+/// terminal faults.
+async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
+    match transport.recv().await? {
+        Some(TransferFrame {
+            frame: Some(Frame::Error(err)),
+        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
+        Some(TransferFrame { frame: Some(f) }) => Ok(f),
+        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
+            SessionFault::protocol_violation("frame with empty oneof"),
+        )),
+        None => Err(eyre::Report::new(SessionFault::internal(
+            "peer closed during session establish",
+        ))),
+    }
+}
+
+/// Send the fault to the peer (best effort), mark it notified, and
+/// wrap it for return.
+async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
+    let _ = transport.send(error_frame(&fault)).await;
+    fault.peer_notified = true;
+    eyre::Report::new(fault)
+}
+
+// ---------------------------------------------------------------------------
+// SOURCE driver
+// ---------------------------------------------------------------------------
+
+/// Events the source's receive half forwards to its send half. The
+/// channel is unbounded but bounded by construction: every `Need`
+/// consumes a distinct sent-manifest entry (unknown or repeated paths
+/// fault the session), so the queue never exceeds the source's own
+/// manifest size — the contract's bounded-buffering rule holds.
+enum SourceEvent {
+    Need(FileHeader),
+    NeedComplete,
+    Summary(TransferSummary),
+    Fault(SessionFault),
+}
+
+/// Run the SOURCE role of one transfer session over `transport`.
+/// Returns the destination-computed `TransferSummary` (contract: the
+/// end that wrote the bytes is the end that attests to them).
+pub async fn run_source(
+    cfg: SourceSessionConfig,
+    transport: FrameTransport,
+    source: Arc<dyn TransferSource>,
+) -> Result<TransferSummary> {
+    let mut transport = transport;
+    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
+        // Own-config coherence: a source initiator declares SOURCE.
+        let declared = TransferRole::try_from(open.initiator_role);
+        if declared != Ok(TransferRole::Source) {
+            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
+        }
+        if let Err(fault) = source_open_validator(open) {
+            eyre::bail!("run_source initiator config unsupported: {fault}");
+        }
+    }
+
+    let negotiated = establish(
+        &mut transport,
+        &cfg.hello,
+        &cfg.endpoint,
+        TransferRole::Source,
+        &source_open_validator,
+    )
+    .await?;
+
+    let (mut tx, rx) = transport.split();
+    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
+    let (event_tx, event_rx) = mpsc::unbounded_channel();
+    // AbortOnDrop: an early error return below must abort the receive
+    // half instead of leaking it (same rationale as design-2 / w4-1).
+    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
+        rx,
+        Arc::clone(&sent),
+        event_tx,
+    )));
+
+    match source_send_half(&cfg, &negotiated, &mut tx, source, sent, event_rx).await {
+        Ok(summary) => Ok(summary),
+        Err(report) => {
+            let mut fault = fault_from_report(report);
+            if !fault.peer_notified {
+                let _ = tx.send(error_frame(&fault)).await;
+                fault.peer_notified = true;
+            }
+            Err(eyre::Report::new(fault))
+        }
+    }
+}
+
+/// Receive half of the source driver: drains the transport for the
+/// whole session so destination sends can never deadlock against a
+/// blocked source send, and routes the destination lane to the send
+/// half. Terminates on summary, error, close, or violation.
+async fn source_recv_half(
+    mut rx: Box<dyn FrameRx>,
+    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
+    events: mpsc::UnboundedSender<SourceEvent>,
+) {
+    loop {
+        let received = match rx.recv().await {
+            Ok(Some(f)) => f,
+            Ok(None) => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
+                    "peer closed before TransferSummary",
+                )));
+                return;
+            }
+            Err(err) => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
+                    "transport receive failed: {err:#}"
+                ))));
+                return;
+            }
+        };
+        match received.frame {
+            Some(Frame::NeedBatch(batch)) => {
+                for entry in batch.entries {
+                    if entry.resume {
+                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
+                            format!(
+                                "resume-flagged need for '{}' in a session opened without resume",
+                                entry.relative_path
+                            ),
+                        )));
+                        return;
+                    }
+                    let header = sent
+                        .lock()
+                        .expect("sent-manifest lock poisoned")
+                        .remove(&entry.relative_path);
+                    match header {
+                        Some(h) => {
+                            let _ = events.send(SourceEvent::Need(h));
+                        }
+                        None => {
+                            let _ = events.send(SourceEvent::Fault(
+                                SessionFault::protocol_violation(format!(
+                                    "need for unknown or already-needed path '{}'",
+                                    entry.relative_path
+                                )),
+                            ));
+                            return;
+                        }
+                    }
+                }
+            }
+            Some(Frame::NeedComplete(_)) => {
+                let _ = events.send(SourceEvent::NeedComplete);
+            }
+            Some(Frame::Summary(summary)) => {
+                let _ = events.send(SourceEvent::Summary(summary));
+                return;
+            }
+            Some(Frame::Error(err)) => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
+                return;
+            }
+            other => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
+                    format!("{} on the source's receive lane", frame_name(&other)),
+                )));
+                return;
+            }
+        }
+    }
+}
+
+async fn source_send_half(
+    cfg: &SourceSessionConfig,
+    negotiated: &Negotiated,
+    tx: &mut Box<dyn FrameTx>,
+    source: Arc<dyn TransferSource>,
+    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
+    mut events: mpsc::UnboundedReceiver<SourceEvent>,
+) -> Result<TransferSummary> {
+    let mut pending: Vec<FileHeader> = Vec::new();
+    let mut need_complete = false;
+
+    // Streaming manifest: entries go out as enumeration produces them
+    // (immediate start in every direction — plan §Design 2). The open
+    // carries no source path: the source end owns its local endpoint.
+    let _ = &negotiated.open;
+    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
+    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
+    while let Some(header) = header_rx.recv().await {
+        sent.lock()
+            .expect("sent-manifest lock poisoned")
+            .insert(header.relative_path.clone(), header.clone());
+        tx.send(frame(Frame::ManifestEntry(header))).await?;
+        // Faults detected by the receive half abort the stream now,
+        // not after the full scan; needs just accumulate.
+        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
+    }
+    let scanned = scan_handle
+        .await
+        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
+    let scan_complete = unreadable
+        .lock()
+        .expect("unreadable list lock poisoned")
+        .is_empty();
+    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
+    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
+        scan_complete,
+    })))
+    .await?;
+
+    // Payload phase. In-stream record grammar: payload records only
+    // after ManifestComplete, strictly serialized per record
+    // (contract §Transport selection). Needs accumulated while a
+    // record batch was being sent become the next planner batch.
+    let mut read_buf = vec![0u8; IN_STREAM_CHUNK];
+    loop {
+        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
+        if !pending.is_empty() {
+            let batch = std::mem::take(&mut pending);
+            send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf).await?;
+            continue;
+        }
+        if need_complete {
+            break;
+        }
+        match events.recv().await {
+            Some(event) => {
+                handle_source_event(event, &mut pending, &mut need_complete)?;
+            }
+            None => {
+                return Err(eyre::Report::new(SessionFault::internal(
+                    "source receive half ended before NeedComplete",
+                )))
+            }
+        }
+    }
+
+    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
+
+    // CLOSING: the destination is the scorer; the next event must be
+    // its summary (the receive half ends after forwarding it).
+    match events.recv().await {
+        Some(SourceEvent::Summary(summary)) => Ok(summary),
+        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
+        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
+            format!("need for '{}' after NeedComplete", h.relative_path),
+        ))),
+        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
+            SessionFault::protocol_violation("duplicate NeedComplete"),
+        )),
+        None => Err(eyre::Report::new(SessionFault::internal(
+            "source receive half ended before TransferSummary",
+        ))),
+    }
+}
+
+fn drain_source_events(
+    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
+    pending: &mut Vec<FileHeader>,
+    need_complete: &mut bool,
+) -> Result<()> {
+    while let Ok(event) = events.try_recv() {
+        handle_source_event(event, pending, need_complete)?;
+    }
+    Ok(())
+}
+
+fn handle_source_event(
+    event: SourceEvent,
+    pending: &mut Vec<FileHeader>,
+    need_complete: &mut bool,
+) -> Result<()> {
+    match event {
+        SourceEvent::Need(header) => {
+            if *need_complete {
+                return Err(eyre::Report::new(SessionFault::protocol_violation(
+                    format!("need for '{}' after NeedComplete", header.relative_path),
+                )));
+            }
+            pending.push(header);
+            Ok(())
+        }
+        SourceEvent::NeedComplete => {
+            if *need_complete {
+                return Err(eyre::Report::new(SessionFault::protocol_violation(
+                    "duplicate NeedComplete",
+                )));
+            }
+            *need_complete = true;
+            Ok(())
+        }
+        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
+            "TransferSummary before SourceDone",
+        ))),
+        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
+    }
+}
+
+/// Plan one batch of needed headers with the engine planner and emit
+/// the resulting payload records per the in-stream grammar.
+async fn send_payload_records(
+    tx: &mut Box<dyn FrameTx>,
+    source: &Arc<dyn TransferSource>,
+    plan_options: PlanOptions,
+    batch: Vec<FileHeader>,
+    read_buf: &mut [u8],
+) -> Result<()> {
+    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
+    for payload in payloads {
+        match source.prepare_payload(payload).await? {
+            PreparedPayload::File(header) => {
+                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
+                if header.size == 0 {
+                    continue; // record complete at 0 cumulative bytes
+                }
+                let mut reader = source.open_file(&header).await?;
+                let mut remaining = header.size;
+                while remaining > 0 {
+                    let want = read_buf.len().min(remaining as usize);
+                    let got = reader.read(&mut read_buf[..want]).await?;
+                    if got == 0 {
+                        // Shorter on disk than the manifest promised —
+                        // the record can no longer complete at
+                        // header.size; abort rather than pad.
+                        eyre::bail!(
+                            "'{}' hit EOF with {} bytes still promised",
+                            header.relative_path,
+                            remaining
+                        );
+                    }
+                    tx.send(frame(Frame::FileData(FileData {
+                        content: read_buf[..got].to_vec(),
+                    })))
+                    .await?;
+                    remaining -= got as u64;
+                }
+            }
+            PreparedPayload::TarShard { headers, data } => {
+                tx.send(frame(Frame::TarShardHeader(TarShardHeader {
+                    files: headers,
+                    archive_size: data.len() as u64,
+                })))
+                .await?;
+                for chunk in data.chunks(IN_STREAM_CHUNK) {
+                    tx.send(frame(Frame::TarShardChunk(
+                        crate::generated::TarShardChunk {
+                            content: chunk.to_vec(),
+                        },
+                    )))
+                    .await?;
+                }
+                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
+                    .await?;
+            }
+            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
+                // The outbound planner never emits these (resume is
+                // receive-originated and lands at otp-7).
+                eyre::bail!("resume payload planned in a non-resume session");
+            }
+        }
+    }
+    Ok(())
+}
+
+// ---------------------------------------------------------------------------
+// DESTINATION driver
+// ---------------------------------------------------------------------------
+
+/// What the destination end can report after a completed session.
+#[derive(Debug, Clone)]
+pub struct DestinationOutcome {
+    /// The summary this end computed and sent (contract: DESTINATION
+    /// is the scorer).
+    pub summary: TransferSummary,
+    /// Paths this end put on the need list, in emission order. The
+    /// role suite pins these identical across role assignments — the
+    /// executable form of the owner's invariance requirement.
+    pub needed_paths: Vec<String>,
+}
+
+/// Run the DESTINATION role of one transfer session over `transport`,
+/// writing under `dst_root`. Diffs the streamed manifest against its
+/// own filesystem (the destination is the one diff owner — plan
+/// §Design 3), returns the summary it computed and sent.
+pub async fn run_destination(
+    cfg: DestinationSessionConfig,
+    transport: FrameTransport,
+    dst_root: PathBuf,
+) -> Result<DestinationOutcome> {
+    let mut transport = transport;
+    let endpoint = match cfg.endpoint {
+        SessionEndpoint::Initiator { mut open } => {
+            let declared = TransferRole::try_from(open.initiator_role);
+            if declared != Ok(TransferRole::Destination) {
+                eyre::bail!(
+                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
+                );
+            }
+            if let Err(fault) = destination_open_validator(&open) {
+                eyre::bail!("run_destination initiator config unsupported: {fault}");
+            }
+            // Dial contract: the byte receiver advertises capacity in
+            // its open when it is the initiator (contract §Invariants 5).
+            if open.receiver_capacity.is_none() {
+                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
+            }
+            SessionEndpoint::Initiator { open }
+        }
+        SessionEndpoint::Responder => SessionEndpoint::Responder,
+    };
+
+    let negotiated = establish(
+        &mut transport,
+        &cfg.hello,
+        &endpoint,
+        TransferRole::Destination,
+        &destination_open_validator,
+    )
+    .await?;
+
+    match destination_session(&mut transport, &negotiated, &dst_root).await {
+        Ok(outcome) => Ok(outcome),
+        Err(report) => {
+            let mut fault = fault_from_report(report);
+            if !fault.peer_notified {
+                let _ = transport.send(error_frame(&fault)).await;
+                fault.peer_notified = true;
+            }
+            Err(eyre::Report::new(fault))
+        }
+    }
+}
+
+fn violation(message: String) -> eyre::Report {
+    eyre::Report::new(SessionFault::protocol_violation(message))
+}
+
+async fn destination_session(
+    transport: &mut FrameTransport,
+    negotiated: &Negotiated,
+    dst_root: &Path,
+) -> Result<DestinationOutcome> {
+    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
+        .unwrap_or(ComparisonMode::Unspecified);
+    let compare_opts = CompareOptions {
+        mode: compare_mode.into(),
+        ignore_existing: negotiated.open.ignore_existing,
+        include_deletions: false, // mirror lands at otp-6
+    };
+    // src_root is only consumed by local File payloads, which never
+    // occur on a session destination (payload bytes arrive as records
+    // and go through the stream/tar write paths).
+    let sink = FsTransferSink::new(
+        PathBuf::new(),
+        dst_root.to_path_buf(),
+        FsSinkConfig {
+            preserve_times: true,
+            dry_run: false,
+            checksum: None,
+            resume: false,
+            compare_mode,
+        },
+    );
+    // Same canonical-containment chokepoint the sink write paths use
+    // (R46-F3), applied to diff stats so a hostile manifest path can't
+    // make the destination stat outside its root.
+    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
+
+    let mut pending: Vec<FileHeader> = Vec::new();
+    let mut outstanding: HashSet<String> = HashSet::new();
+    let mut needed_paths: Vec<String> = Vec::new();
+    let mut manifest_complete = false;
+    let mut files_written: u64 = 0;
+    let mut bytes_written: u64 = 0;
+
+    loop {
+        let received = match transport.recv().await? {
+            Some(f) => f,
+            None => {
+                return Err(eyre::Report::new(SessionFault::internal(
+                    "peer closed mid-session",
+                )))
+            }
+        };
+        match received.frame {
+            Some(Frame::ManifestEntry(header)) => {
+                if manifest_complete {
+                    return Err(violation(format!(
+                        "manifest entry '{}' after ManifestComplete",
+                        header.relative_path
+                    )));
+                }
+                pending.push(header);
+                if pending.len() >= DEST_DIFF_CHUNK {
+                    let chunk = std::mem::take(&mut pending);
+                    diff_chunk_and_send_needs(
+                        transport,
+                        chunk,
+                        dst_root,
+                        canonical_dst_root.as_deref(),
+                        &compare_opts,
+                        &mut outstanding,
+                        &mut needed_paths,
+                    )
+                    .await?;
+                }
+            }
+            Some(Frame::ManifestComplete(_complete)) => {
+                if manifest_complete {
+                    return Err(violation("duplicate ManifestComplete".into()));
+                }
+                // (scan_complete gates mirror purges from otp-6 on;
+                // nothing consumes it in otp-3.)
+                let chunk = std::mem::take(&mut pending);
+                diff_chunk_and_send_needs(
+                    transport,
+                    chunk,
+                    dst_root,
+                    canonical_dst_root.as_deref(),
+                    &compare_opts,
+                    &mut outstanding,
+                    &mut needed_paths,
+                )
+                .await?;
+                // NeedComplete only after ManifestComplete received
+                // AND every entry diffed — both true here.
+                transport
+                    .send(frame(Frame::NeedComplete(NeedComplete {})))
+                    .await?;
+                manifest_complete = true;
+            }
+            Some(Frame::FileBegin(header)) => {
+                if !manifest_complete {
+                    return Err(violation(format!(
+                        "payload record for '{}' before ManifestComplete",
+                        header.relative_path
+                    )));
+                }
+                if !outstanding.remove(&header.relative_path) {
+                    return Err(violation(format!(
+                        "payload for '{}' which is not on the need list",
+                        header.relative_path
+                    )));
+                }
+                let outcome = receive_file_record(transport, &sink, &header).await?;
+                files_written += outcome.files_written as u64;
+                bytes_written += outcome.bytes_written;
+            }
+            Some(Frame::TarShardHeader(shard)) => {
+                if !manifest_complete {
+                    return Err(violation("tar shard record before ManifestComplete".into()));
+                }
+                for h in &shard.files {
+                    if !outstanding.remove(&h.relative_path) {
+                        return Err(violation(format!(
+                            "tar shard entry '{}' which is not on the need list",
+                            h.relative_path
+                        )));
+                    }
+                }
+                let outcome = receive_tar_record(transport, &sink, shard).await?;
+                files_written += outcome.files_written as u64;
+                bytes_written += outcome.bytes_written;
+            }
+            Some(Frame::SourceDone(_)) => {
+                if !manifest_complete {
+                    return Err(violation("SourceDone before ManifestComplete".into()));
+                }
+                if !outstanding.is_empty() {
+                    return Err(violation(format!(
+                        "SourceDone with {} needed file(s) never sent",
+                        outstanding.len()
+                    )));
+                }
+                let summary = TransferSummary {
+                    files_transferred: files_written,
+                    bytes_transferred: bytes_written,
+                    entries_deleted: 0, // mirror lands at otp-6
+                    in_stream_carrier_used: true,
+                    files_resumed: 0, // resume lands at otp-7
+                };
+                transport.send(frame(Frame::Summary(summary))).await?;
+                return Ok(DestinationOutcome {
+                    summary,
+                    needed_paths,
+                });
+            }
+            Some(Frame::Error(err)) => {
+                return Err(eyre::Report::new(SessionFault::from_wire(err)));
+            }
+            other => {
+                // Everything else is off-lane or off-phase here:
+                // destination-lane frames echoed back, resume frames
+                // in a non-resume session (otp-7), resize with no
+                // data plane to resize (otp-4), stray handshake
+                // frames, bare FileData/TarShardChunk outside a
+                // record. Fail fast, no tolerant parsing.
+                return Err(violation(format!(
+                    "{} not valid on the destination's receive lane in this phase",
+                    frame_name(&other)
+                )));
+            }
+        }
+    }
+}
+
+/// Stat-and-compare one chunk of manifest entries on the blocking
+/// pool (2+ syscalls per entry — same rationale as the daemon's
+/// w4-4 chunked checks), then stream the resulting need batch.
+async fn diff_chunk_and_send_needs(
+    transport: &mut FrameTransport,
+    chunk: Vec<FileHeader>,
+    dst_root: &Path,
+    canonical_dst_root: Option<&Path>,
+    compare_opts: &CompareOptions,
+    outstanding: &mut HashSet<String>,
+    needed_paths: &mut Vec<String>,
+) -> Result<()> {
+    if chunk.is_empty() {
+        return Ok(());
+    }
+    let dst_root = dst_root.to_path_buf();
+    let canonical = canonical_dst_root.map(Path::to_path_buf);
+    let opts = compare_opts.clone();
+    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
+        let mut needed = Vec::new();
+        for header in &chunk {
+            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
+                needed.push(header.relative_path.clone());
+            }
+        }
+        Ok(needed)
+    })
+    .await
+    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
+
+    let entries: Vec<NeedEntry> = needed
+        .into_iter()
+        // A path the source manifests twice is diffed twice but
+        // needed at most once.
+        .filter(|path| outstanding.insert(path.clone()))
+        .map(|relative_path| {
+            needed_paths.push(relative_path.clone());
+            NeedEntry {
+                relative_path,
+                resume: false, // resume lands at otp-7
+            }
+        })
+        .collect();
+    if entries.is_empty() {
+        return Ok(());
+    }
+    transport
+        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
+        .await?;
+    Ok(())
+}
+
+/// Does the destination need this manifest entry? Stats its own file
+/// and delegates the verdict to `manifest::header_transfer_status` —
+/// the same mode-aware owner `compare_manifests` uses, fed from a
+/// live stat instead of a materialized target manifest.
+fn destination_needs(
+    header: &FileHeader,
+    dst_root: &Path,
+    canonical_dst_root: Option<&Path>,
+    opts: &CompareOptions,
+) -> Result<bool> {
+    let dst = match canonical_dst_root {
+        Some(canonical) => {
+            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
+        }
+        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
+    }
+    .map_err(|err| {
+        SessionFault::protocol_violation(format!(
+            "manifest path '{}' escapes the destination root: {err:#}",
+            header.relative_path
+        ))
+    })?;
+
+    let target = match std::fs::metadata(&dst) {
+        Ok(meta) if meta.is_file() => {
+            let mtime = match meta.modified() {
+                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
+                    Ok(d) => d.as_secs() as i64,
+                    Err(e) => -(e.duration().as_secs() as i64),
+                },
+                Err(_) => 0,
+            };
+            Some((meta.len(), mtime))
+        }
+        // Absent — or present as a directory/other, which a file
+        // write must replace: both diff as "target does not have it"
+        // (matches the push daemon's file_requires_upload).
+        _ => None,
+    };
+    let status = header_transfer_status(
+        header,
+        // Destination-side checksums are never precomputed; Checksum
+        // mode therefore transfers (the conservative arm of
+        // compare_file), matching what push does today.
+        target.map(|(size, mtime)| (size, mtime, &[] as &[u8])),
+        opts,
+    );
+    Ok(matches!(status, FileStatus::New | FileStatus::Modified))
+}
+
+/// Receive one strictly-serialized file record (`file_begin` already
+/// consumed) and stream its bytes into the sink through a bounded
+/// in-memory pipe — record completion is exactly `header.size`
+/// cumulative bytes (contract §Transport selection).
+async fn receive_file_record(
+    transport: &mut FrameTransport,
+    sink: &FsTransferSink,
+    header: &FileHeader,
+) -> Result<crate::remote::transfer::SinkOutcome> {
+    let (mut pipe_wr, mut pipe_rd) = tokio::io::duplex(FILE_RECORD_PIPE_BYTES);
+    let write = sink.write_file_stream(header, &mut pipe_rd);
+    let feed = async {
+        let mut remaining = header.size;
+        while remaining > 0 {
+            let received = match transport.recv().await? {
+                Some(f) => f,
+                None => {
+                    return Err(eyre::Report::new(SessionFault::internal(format!(
+                        "peer closed inside file record '{}'",
+                        header.relative_path
+                    ))))
+                }
+            };
+            match received.frame {
+                Some(Frame::FileData(data)) => {
+                    let len = data.content.len() as u64;
+                    if len > remaining {
+                        return Err(violation(format!(
+                            "file record '{}' overran its size by {} byte(s)",
+                            header.relative_path,
+                            len - remaining
+                        )));
+                    }
+                    pipe_wr.write_all(&data.content).await?;
+                    remaining -= len;
+                }
+                other => {
+                    // Strict serialization: nothing may interleave
+                    // with an open record on the source lane.
+                    return Err(violation(format!(
+                        "{} inside file record '{}' ({} byte(s) short)",
+                        frame_name(&other),
+                        header.relative_path,
+                        remaining
+                    )));
+                }
+            }
+        }
+        pipe_wr.shutdown().await?;
+        Ok(())
+    };
+    let (outcome, ()) = tokio::try_join!(write, feed)?;
+    Ok(outcome)
+}
+
+/// Receive one tar-shard record (`tar_shard_header` already consumed):
+/// buffer to exactly `archive_size` (bounded by the shared tar cap)
+/// and hand the archive to the sink's tar-safety unpack path.
+async fn receive_tar_record(
+    transport: &mut FrameTransport,
+    sink: &FsTransferSink,
+    shard: TarShardHeader,
+) -> Result<crate::remote::transfer::SinkOutcome> {
+    if shard.archive_size > MAX_TAR_SHARD_BYTES {
+        return Err(violation(format!(
+            "tar shard of {} bytes exceeds the {} byte cap",
+            shard.archive_size, MAX_TAR_SHARD_BYTES
+        )));
+    }
+    let mut data: Vec<u8> = Vec::new();
+    data.try_reserve_exact(shard.archive_size as usize)
+        .map_err(|err| eyre::eyre!("allocating {} byte tar shard: {err}", shard.archive_size))?;
+    loop {
+        let received = match transport.recv().await? {
+            Some(f) => f,
+            None => {
+                return Err(eyre::Report::new(SessionFault::internal(
+                    "peer closed inside tar shard record",
+                )))
+            }
+        };
+        match received.frame {
+            Some(Frame::TarShardChunk(chunk)) => {
+                if data.len() as u64 + chunk.content.len() as u64 > shard.archive_size {
+                    return Err(violation(format!(
+                        "tar shard record overran its declared {} bytes",
+                        shard.archive_size
+                    )));
+                }
+                data.extend_from_slice(&chunk.content);
+            }
+            Some(Frame::TarShardComplete(_)) => {
+                if data.len() as u64 != shard.archive_size {
+                    return Err(violation(format!(
+                        "tar shard record completed at {} of {} declared bytes",
+                        data.len(),
+                        shard.archive_size
+                    )));
+                }
+                return sink
+                    .write_payload(PreparedPayload::TarShard {
+                        headers: shard.files,
+                        data,
+                    })
+                    .await;
+            }
+            other => {
+                return Err(violation(format!(
+                    "{} inside tar shard record",
+                    frame_name(&other)
+                )));
+            }
+        }
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn build_id_has_version_and_git_components() {
+        let id = session_build_id();
+        let (version, git) = id.split_once('+').expect("build id must be version+git");
+        assert_eq!(version, env!("CARGO_PKG_VERSION"));
+        assert!(!git.is_empty(), "git component must be non-empty");
+    }
+
+    #[test]
+    fn fault_round_trips_the_wire_shape() {
+        let fault = SessionFault {
+            code: session_error::Code::BuildMismatch,
+            message: "boom".into(),
+            local_build_id: "1.0+aaa".into(),
+            peer_build_id: "1.0+bbb".into(),
+            peer_notified: false,
+        };
+        let wire = fault.to_wire();
+        let back = SessionFault::from_wire(wire);
+        assert_eq!(back.code, session_error::Code::BuildMismatch);
+        assert_eq!(back.message, "boom");
+        // from_wire swaps perspective: the sender's local is our peer.
+        assert_eq!(back.peer_build_id, "1.0+aaa");
+        assert_eq!(back.local_build_id, "1.0+bbb");
+        assert!(back.peer_notified);
+    }
+}
diff --git a/crates/blit-core/src/transfer_session/transport.rs b/crates/blit-core/src/transfer_session/transport.rs
new file mode 100644
index 0000000..224bd3f
--- /dev/null
+++ b/crates/blit-core/src/transfer_session/transport.rs
@@ -0,0 +1,142 @@
+//! Frame transports for the unified transfer session.
+//!
+//! The session drivers in this module's parent speak
+//! [`TransferFrame`]s through the `FrameTx`/`FrameRx` halves and never
+//! know what carries them. otp-3 ships the in-process pair below;
+//! otp-4 adds a gRPC-backed implementation over the `Transfer` RPC
+//! (transport substitution, not new choreography —
+//! docs/TRANSFER_SESSION.md); otp-11 reuses the in-process pair for
+//! local transfers.
+
+use async_trait::async_trait;
+use eyre::{eyre, Result};
+use tokio::sync::mpsc;
+
+use crate::generated::TransferFrame;
+
+/// Sending half of a frame transport. `send` applies the transport's
+/// own backpressure (bounded channel here, HTTP/2 flow control on the
+/// wire) — the session contract deliberately leans on it instead of
+/// buffering (docs/TRANSFER_SESSION.md §Phase state machine).
+#[async_trait]
+pub trait FrameTx: Send {
+    async fn send(&mut self, frame: TransferFrame) -> Result<()>;
+}
+
+/// Receiving half of a frame transport. `Ok(None)` means the peer
+/// closed the stream cleanly; transport-level failures are `Err`.
+#[async_trait]
+pub trait FrameRx: Send {
+    async fn recv(&mut self) -> Result<Option<TransferFrame>>;
+}
+
+/// One endpoint's bidirectional frame stream, splittable so a driver
+/// can run its send and receive halves concurrently (the source
+/// driver must keep draining need batches while it streams manifest
+/// entries, or a full channel in each direction deadlocks the pair).
+pub struct FrameTransport {
+    tx: Box<dyn FrameTx>,
+    rx: Box<dyn FrameRx>,
+}
+
+impl FrameTransport {
+    pub fn new(tx: Box<dyn FrameTx>, rx: Box<dyn FrameRx>) -> Self {
+        Self { tx, rx }
+    }
+
+    pub async fn send(&mut self, frame: TransferFrame) -> Result<()> {
+        self.tx.send(frame).await
+    }
+
+    pub async fn recv(&mut self) -> Result<Option<TransferFrame>> {
+        self.rx.recv().await
+    }
+
+    pub fn split(self) -> (Box<dyn FrameTx>, Box<dyn FrameRx>) {
+        (self.tx, self.rx)
+    }
+}
+
+/// Bounded per-direction capacity of the in-process pair. Small on
+/// purpose: the session must stay live under transport backpressure
+/// (both drivers are exercised against it in the role suite), and a
+/// deep channel would only hide ordering bugs the wire will expose.
+pub const IN_PROCESS_CHANNEL_FRAMES: usize = 64;
+
+struct MpscFrameTx {
+    tx: mpsc::Sender<TransferFrame>,
+}
+
+#[async_trait]
+impl FrameTx for MpscFrameTx {
+    async fn send(&mut self, frame: TransferFrame) -> Result<()> {
+        self.tx
+            .send(frame)
+            .await
+            .map_err(|_| eyre!("in-process transport peer closed"))
+    }
+}
+
+struct MpscFrameRx {
+    rx: mpsc::Receiver<TransferFrame>,
+}
+
+#[async_trait]
+impl FrameRx for MpscFrameRx {
+    async fn recv(&mut self) -> Result<Option<TransferFrame>> {
+        Ok(self.rx.recv().await)
+    }
+}
+
+/// Two connected in-process endpoints: what one sends, the other
+/// receives. Both roles of a local transfer (otp-11) — and every
+/// otp-3 test — run over this pair.
+pub fn in_process_pair() -> (FrameTransport, FrameTransport) {
+    let (a_tx, b_rx) = mpsc::channel(IN_PROCESS_CHANNEL_FRAMES);
+    let (b_tx, a_rx) = mpsc::channel(IN_PROCESS_CHANNEL_FRAMES);
+    (
+        FrameTransport::new(
+            Box::new(MpscFrameTx { tx: a_tx }),
+            Box::new(MpscFrameRx { rx: a_rx }),
+        ),
+        FrameTransport::new(
+            Box::new(MpscFrameTx { tx: b_tx }),
+            Box::new(MpscFrameRx { rx: b_rx }),
+        ),
+    )
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::generated::{transfer_frame, SourceDone};
+
+    fn source_done_frame() -> TransferFrame {
+        TransferFrame {
+            frame: Some(transfer_frame::Frame::SourceDone(SourceDone {})),
+        }
+    }
+
+    #[tokio::test]
+    async fn pair_delivers_frames_both_directions() {
+        let (mut a, mut b) = in_process_pair();
+        a.send(source_done_frame()).await.unwrap();
+        b.send(source_done_frame()).await.unwrap();
+        assert!(matches!(
+            b.recv().await.unwrap().unwrap().frame,
+            Some(transfer_frame::Frame::SourceDone(_))
+        ));
+        assert!(matches!(
+            a.recv().await.unwrap().unwrap().frame,
+            Some(transfer_frame::Frame::SourceDone(_))
+        ));
+    }
+
+    #[tokio::test]
+    async fn dropped_peer_reads_as_clean_close_and_send_error() {
+        let (mut a, b) = in_process_pair();
+        drop(b);
+        assert!(a.recv().await.unwrap().is_none(), "closed peer = Ok(None)");
+        assert!(a.send(source_done_frame()).await.is_err());
+    }
+}
diff --git a/crates/blit-core/tests/transfer_session_roles.rs b/crates/blit-core/tests/transfer_session_roles.rs
new file mode 100644
index 0000000..63138b7
--- /dev/null
+++ b/crates/blit-core/tests/transfer_session_roles.rs
@@ -0,0 +1,710 @@
+//! Role-parameterized session suite (otp-3).
+//!
+//! Every fixture runs under BOTH role assignments — the initiator as
+//! SOURCE (push-shaped) and the initiator as DESTINATION (pull-shaped)
+//! — over the in-process transport, and the outcomes must be
+//! IDENTICAL: same need-list set, same summary counts, same bytes on
+//! disk. This is the owner's invariance requirement
+//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1) in its first
+//! executable form: there is no per-direction code to diverge, and
+//! this suite pins that the one code path really is
+//! initiator-indifferent.
+
+use std::collections::BTreeMap;
+use std::path::Path;
+use std::sync::Arc;
+use std::time::Duration;
+
+use blit_core::generated::transfer_frame::Frame;
+use blit_core::generated::{
+    session_error, ComparisonMode, FileHeader, ManifestComplete, NeedBatch, NeedEntry,
+    SessionHello, SessionOpen, TransferFrame, TransferRole, TransferSummary,
+};
+use blit_core::remote::transfer::source::FsTransferSource;
+use blit_core::transfer_plan::PlanOptions;
+use blit_core::transfer_session::transport::{in_process_pair, FrameTransport};
+use blit_core::transfer_session::{
+    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, HelloConfig,
+    SessionEndpoint, SessionFault, SourceSessionConfig, CONTRACT_VERSION,
+};
+
+const SUITE_TIMEOUT: Duration = Duration::from_secs(120);
+
+/// (relative path, content, mtime seconds). Fixture mtimes are fixed
+/// epochs so both role-assignment runs see byte-for-byte identical
+/// trees.
+type FileSpec = (&'static str, Vec<u8>, i64);
+
+fn write_tree(root: &Path, files: &[FileSpec]) {
+    for (rel, content, mtime) in files {
+        let path = root.join(rel);
+        if let Some(parent) = path.parent() {
+            std::fs::create_dir_all(parent).unwrap();
+        }
+        std::fs::write(&path, content).unwrap();
+        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
+    }
+}
+
+/// Every regular file under `root` as rel-path → bytes.
+fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
+    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
+        for entry in std::fs::read_dir(dir).unwrap() {
+            let entry = entry.unwrap();
+            let path = entry.path();
+            if path.is_dir() {
+                walk(root, &path, out);
+            } else {
+                let rel = path
+                    .strip_prefix(root)
+                    .unwrap()
+                    .to_string_lossy()
+                    .replace('\\', "/");
+                out.insert(rel, std::fs::read(&path).unwrap());
+            }
+        }
+    }
+    let mut out = BTreeMap::new();
+    if root.exists() {
+        walk(root, root, &mut out);
+    }
+    out
+}
+
+fn assert_trees_identical(src: &Path, dst: &Path) {
+    let src_tree = collect_tree(src);
+    let dst_tree = collect_tree(dst);
+    assert_eq!(
+        src_tree.keys().collect::<Vec<_>>(),
+        dst_tree.keys().collect::<Vec<_>>(),
+        "path sets differ between {src:?} and {dst:?}"
+    );
+    for (rel, bytes) in &src_tree {
+        assert_eq!(
+            bytes, &dst_tree[rel],
+            "content differs for '{rel}' between {src:?} and {dst:?}"
+        );
+    }
+}
+
+fn basic_open(initiator_role: TransferRole) -> SessionOpen {
+    SessionOpen {
+        initiator_role: initiator_role as i32,
+        compare_mode: ComparisonMode::SizeMtime as i32,
+        in_stream_bytes: true,
+        ..Default::default()
+    }
+}
+
+/// Drive one full session between `src_root` and `dst_root` with the
+/// given end acting as initiator. Data direction is FIXED
+/// (src_root → dst_root); the parameter only swaps which end opens
+/// the session — the thing the owner's invariant says must not
+/// matter.
+async fn run_session(
+    initiator_role: TransferRole,
+    src_root: &Path,
+    dst_root: &Path,
+    plan_options: PlanOptions,
+) -> (
+    eyre::Result<TransferSummary>,
+    eyre::Result<DestinationOutcome>,
+) {
+    let open = basic_open(initiator_role);
+    let (source_endpoint, dest_endpoint) = match initiator_role {
+        TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
+        TransferRole::Destination => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
+        TransferRole::Unspecified => panic!("fixture must pick a role"),
+    };
+    let source_cfg = SourceSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: source_endpoint,
+        plan_options,
+    };
+    let dest_cfg = DestinationSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: dest_endpoint,
+    };
+    let (a, b) = in_process_pair();
+    let source = Arc::new(FsTransferSource::new(src_root.to_path_buf()));
+    tokio::time::timeout(SUITE_TIMEOUT, async {
+        tokio::join!(
+            run_source(source_cfg, a, source),
+            run_destination(dest_cfg, b, dst_root.to_path_buf()),
+        )
+    })
+    .await
+    .expect("session run timed out")
+}
+
+/// Run the same fixture under both role assignments (fresh trees per
+/// run) and pin the invariance property: identical need sets,
+/// identical summaries, byte-identical destinations.
+async fn assert_invariant_across_roles(
+    src_files: &[FileSpec],
+    dst_files: &[FileSpec],
+    plan_options: PlanOptions,
+) -> (TransferSummary, Vec<String>) {
+    let mut per_role: Vec<(TransferSummary, Vec<String>)> = Vec::new();
+    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
+        let tmp = tempfile::tempdir().unwrap();
+        let src_root = tmp.path().join("src");
+        let dst_root = tmp.path().join("dst");
+        std::fs::create_dir_all(&src_root).unwrap();
+        std::fs::create_dir_all(&dst_root).unwrap();
+        write_tree(&src_root, src_files);
+        write_tree(&dst_root, dst_files);
+
+        let (source_result, dest_result) =
+            run_session(initiator_role, &src_root, &dst_root, plan_options).await;
+        let source_summary = source_result
+            .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
+        let dest_outcome = dest_result.unwrap_or_else(|e| {
+            panic!("destination failed under initiator {initiator_role:?}: {e:#}")
+        });
+
+        assert_eq!(
+            source_summary, dest_outcome.summary,
+            "both ends must hold the same summary (initiator {initiator_role:?})"
+        );
+        assert!(
+            source_summary.in_stream_carrier_used,
+            "otp-3 sessions ride the in-stream carrier"
+        );
+        assert_trees_identical(&src_root, &dst_root);
+
+        let mut needed = dest_outcome.needed_paths.clone();
+        needed.sort();
+        per_role.push((dest_outcome.summary, needed));
+    }
+
+    let (summary_a, needed_a) = per_role.remove(0);
+    let (summary_b, needed_b) = per_role.remove(0);
+    assert_eq!(
+        needed_a, needed_b,
+        "need-list set must be identical whichever end initiates"
+    );
+    assert_eq!(
+        summary_a, summary_b,
+        "summary must be identical whichever end initiates"
+    );
+    (summary_a, needed_a)
+}
+
+fn fault_of(err: &eyre::Report) -> &SessionFault {
+    err.downcast_ref::<SessionFault>()
+        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
+}
+
+// ---------------------------------------------------------------------------
+// Fixtures
+// ---------------------------------------------------------------------------
+
+/// Mixed small tree: nested dirs, an empty file, a name with spaces,
+/// and a file larger than the in-stream chunk so file records span
+/// multiple FileData frames.
+fn small_tree() -> Vec<FileSpec> {
+    vec![
+        ("a.txt", b"alpha".to_vec(), 1_600_000_001),
+        ("empty.bin", Vec::new(), 1_600_000_002),
+        ("dir one/b.log", vec![0xAB; 4096], 1_600_000_003),
+        (
+            "dir one/deeper/c.dat",
+            b"gamma-content".to_vec(),
+            1_600_000_004,
+        ),
+        // 3 MiB + 17 so the record needs 4 FileData frames and ends
+        // on a partial chunk.
+        (
+            "big/blob.bin",
+            make_patterned(3 * 1024 * 1024 + 17),
+            1_600_000_005,
+        ),
+    ]
+}
+
+fn make_patterned(len: usize) -> Vec<u8> {
+    (0..len).map(|i| (i % 251) as u8).collect()
+}
+
+#[tokio::test]
+async fn small_tree_byte_identical_under_both_initiators() {
+    let src = small_tree();
+    let (summary, needed) = assert_invariant_across_roles(&src, &[], PlanOptions::default()).await;
+    assert_eq!(summary.files_transferred, src.len() as u64);
+    assert_eq!(
+        summary.bytes_transferred,
+        src.iter().map(|(_, c, _)| c.len() as u64).sum::<u64>()
+    );
+    assert_eq!(summary.entries_deleted, 0);
+    assert_eq!(summary.files_resumed, 0);
+    assert_eq!(
+        needed.len(),
+        src.len(),
+        "empty destination needs everything"
+    );
+}
+
+#[tokio::test]
+async fn tiny_file_tree_tar_shard_records_under_both_initiators() {
+    // 200 tiny files under nested dirs; force_tar makes the planner's
+    // tar-shard choice deterministic so the tar record grammar
+    // (header + chunks + complete → tar-safety unpack) is exercised
+    // under both role assignments.
+    let mut src: Vec<FileSpec> = Vec::new();
+    let names: Vec<String> = (0..200)
+        .map(|i| format!("shards/d{}/f{:03}.txt", i % 7, i))
+        .collect();
+    let leaked: Vec<&'static str> = names
+        .into_iter()
+        .map(|n| Box::leak(n.into_boxed_str()) as &'static str)
+        .collect();
+    for (i, name) in leaked.iter().enumerate() {
+        src.push((
+            name,
+            format!("tiny-{i}").into_bytes(),
+            1_600_100_000 + i as i64,
+        ));
+    }
+    let plan = PlanOptions {
+        force_tar: true,
+        ..PlanOptions::default()
+    };
+    let (summary, needed) = assert_invariant_across_roles(&src, &[], plan).await;
+    assert_eq!(summary.files_transferred, 200);
+    assert_eq!(needed.len(), 200);
+}
+
+#[tokio::test]
+async fn incremental_transfer_needs_only_missing_and_changed() {
+    let src: Vec<FileSpec> = vec![
+        // Identical on both sides (same size, same mtime) → skipped.
+        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
+        // Same size, source newer → transferred.
+        ("newer.txt", b"NEW-eight".to_vec(), 1_600_000_200),
+        // Absent on destination → transferred.
+        ("sub/missing.txt", b"fresh".to_vec(), 1_600_000_300),
+    ];
+    let dst: Vec<FileSpec> = vec![
+        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
+        ("newer.txt", b"old-eight".to_vec(), 1_600_000_100),
+    ];
+    let (summary, needed) = assert_invariant_across_roles(&src, &dst, PlanOptions::default()).await;
+    assert_eq!(
+        needed,
+        vec!["newer.txt".to_string(), "sub/missing.txt".to_string()],
+        "need list must be exactly the changed + missing files"
+    );
+    assert_eq!(summary.files_transferred, 2);
+    assert_eq!(summary.bytes_transferred, 9 + 5);
+}
+
+#[tokio::test]
+async fn preexisting_identical_tree_yields_empty_need_list() {
+    let files: Vec<FileSpec> = vec![
+        ("one.txt", b"matching".to_vec(), 1_600_000_400),
+        ("nested/two.txt", b"also matching".to_vec(), 1_600_000_500),
+    ];
+    let (summary, needed) =
+        assert_invariant_across_roles(&files, &files, PlanOptions::default()).await;
+    assert!(needed.is_empty(), "identical trees must need nothing");
+    assert_eq!(summary.files_transferred, 0);
+    assert_eq!(summary.bytes_transferred, 0);
+}
+
+#[tokio::test]
+async fn preserves_mtime_on_streamed_files() {
+    // Not part of the role matrix — pins that the file-record write
+    // path applies the manifest mtime (parity with today's receive
+    // paths, which the byte-identical asserts alone wouldn't catch).
+    let tmp = tempfile::tempdir().unwrap();
+    let src_root = tmp.path().join("src");
+    let dst_root = tmp.path().join("dst");
+    std::fs::create_dir_all(&src_root).unwrap();
+    std::fs::create_dir_all(&dst_root).unwrap();
+    write_tree(
+        &src_root,
+        &[("stamped.txt", b"stamp me".to_vec(), 1_555_555_555)],
+    );
+
+    let (source_result, dest_result) = run_session(
+        TransferRole::Source,
+        &src_root,
+        &dst_root,
+        PlanOptions::default(),
+    )
+    .await;
+    source_result.unwrap();
+    dest_result.unwrap();
+
+    let meta = std::fs::metadata(dst_root.join("stamped.txt")).unwrap();
+    let mtime = filetime::FileTime::from_last_modification_time(&meta);
+    assert_eq!(mtime.unix_seconds(), 1_555_555_555);
+}
+
+// ---------------------------------------------------------------------------
+// Handshake refusals
+// ---------------------------------------------------------------------------
+
+#[tokio::test]
+async fn build_mismatch_refused_under_both_initiators() {
+    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
+        let tmp = tempfile::tempdir().unwrap();
+        let src_root = tmp.path().join("src");
+        let dst_root = tmp.path().join("dst");
+        std::fs::create_dir_all(&src_root).unwrap();
+        std::fs::create_dir_all(&dst_root).unwrap();
+
+        let open = basic_open(initiator_role);
+        let (source_endpoint, dest_endpoint) = match initiator_role {
+            TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
+            _ => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
+        };
+        let source_cfg = SourceSessionConfig {
+            hello: HelloConfig {
+                build_id: "0.1.0+aaaaaaaaaaaa".into(),
+                contract_version: CONTRACT_VERSION,
+            },
+            endpoint: source_endpoint,
+            plan_options: PlanOptions::default(),
+        };
+        let dest_cfg = DestinationSessionConfig {
+            hello: HelloConfig {
+                build_id: "0.1.0+bbbbbbbbbbbb".into(),
+                contract_version: CONTRACT_VERSION,
+            },
+            endpoint: dest_endpoint,
+        };
+        let (a, b) = in_process_pair();
+        let source = Arc::new(FsTransferSource::new(src_root.clone()));
+        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
+            tokio::join!(
+                run_source(source_cfg, a, source),
+                run_destination(dest_cfg, b, dst_root.clone()),
+            )
+        })
+        .await
+        .unwrap();
+
+        for (end, err) in [
+            ("source", source_result.unwrap_err()),
+            ("destination", dest_result.err().unwrap()),
+        ] {
+            let fault = fault_of(&err);
+            assert_eq!(
+                fault.code,
+                session_error::Code::BuildMismatch,
+                "{end} must refuse with BUILD_MISMATCH (initiator {initiator_role:?})"
+            );
+            assert!(
+                fault.message.contains("aaaaaaaaaaaa") && fault.message.contains("bbbbbbbbbbbb"),
+                "{end} must name both build ids, got: {}",
+                fault.message
+            );
+        }
+        assert!(
+            collect_tree(&dst_root).is_empty(),
+            "no bytes may move on a refused handshake"
+        );
+    }
+}
+
+#[tokio::test]
+async fn contract_version_mismatch_is_refused() {
+    let tmp = tempfile::tempdir().unwrap();
+    let src_root = tmp.path().join("src");
+    let dst_root = tmp.path().join("dst");
+    std::fs::create_dir_all(&src_root).unwrap();
+    std::fs::create_dir_all(&dst_root).unwrap();
+
+    let source_cfg = SourceSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
+        plan_options: PlanOptions::default(),
+    };
+    let dest_cfg = DestinationSessionConfig {
+        hello: HelloConfig {
+            build_id: HelloConfig::default().build_id,
+            contract_version: CONTRACT_VERSION + 1,
+        },
+        endpoint: SessionEndpoint::Responder,
+    };
+    let (a, b) = in_process_pair();
+    let source = Arc::new(FsTransferSource::new(src_root));
+    let (source_result, dest_result) = tokio::join!(
+        run_source(source_cfg, a, source),
+        run_destination(dest_cfg, b, dst_root),
+    );
+    assert_eq!(
+        fault_of(&source_result.unwrap_err()).code,
+        session_error::Code::BuildMismatch
+    );
+    assert_eq!(
+        fault_of(&dest_result.err().unwrap()).code,
+        session_error::Code::BuildMismatch
+    );
+}
+
+#[tokio::test]
+async fn mirror_request_is_refused_until_its_slice_lands() {
+    // otp-3 refuses what it does not implement rather than silently
+    // ignoring it: a mirror-enabled open must fail the session at the
+    // OPEN phase, from the destination (the end that would execute
+    // deletions).
+    let tmp = tempfile::tempdir().unwrap();
+    let src_root = tmp.path().join("src");
+    let dst_root = tmp.path().join("dst");
+    std::fs::create_dir_all(&src_root).unwrap();
+    std::fs::create_dir_all(&dst_root).unwrap();
+
+    let mut open = basic_open(TransferRole::Source);
+    open.mirror_enabled = true;
+    let source_cfg = SourceSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::initiator(open),
+        plan_options: PlanOptions::default(),
+    };
+    let dest_cfg = DestinationSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::Responder,
+    };
+    let (a, b) = in_process_pair();
+    let source = Arc::new(FsTransferSource::new(src_root));
+    let (source_result, dest_result) = tokio::join!(
+        run_source(source_cfg, a, source),
+        run_destination(dest_cfg, b, dst_root),
+    );
+    let source_fault = fault_of(&source_result.unwrap_err()).clone();
+    assert_eq!(source_fault.code, session_error::Code::Internal);
+    assert!(
+        source_fault.message.contains("otp-6"),
+        "refusal must say when mirror lands, got: {}",
+        source_fault.message
+    );
+    assert!(dest_result.is_err());
+}
+
+// ---------------------------------------------------------------------------
+// Protocol-violation fail-fast (scripted peer)
+// ---------------------------------------------------------------------------
+
+fn wire(frame: Frame) -> TransferFrame {
+    TransferFrame { frame: Some(frame) }
+}
+
+async fn recv_or_panic(t: &mut FrameTransport) -> Frame {
+    t.recv()
+        .await
+        .unwrap()
+        .expect("peer closed unexpectedly")
+        .frame
+        .expect("empty frame")
+}
+
+fn hello_frame() -> TransferFrame {
+    let hello = HelloConfig::default();
+    wire(Frame::Hello(SessionHello {
+        build_id: hello.build_id,
+        contract_version: hello.contract_version,
+    }))
+}
+
+#[tokio::test]
+async fn payload_record_before_manifest_complete_is_protocol_violation() {
+    let tmp = tempfile::tempdir().unwrap();
+    let dst_root = tmp.path().join("dst");
+    std::fs::create_dir_all(&dst_root).unwrap();
+
+    let dest_cfg = DestinationSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::Responder,
+    };
+    let (mut peer, dest_transport) = in_process_pair();
+    let dest = tokio::spawn(run_destination(dest_cfg, dest_transport, dst_root));
+
+    // Scripted source peer: valid handshake, then a payload record
+    // while its manifest is still open — the contract's example
+    // violation ("payload records may begin only AFTER the source's
+    // ManifestComplete").
+    peer.send(hello_frame()).await.unwrap();
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
+    peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
+        .await
+        .unwrap();
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
+
+    let header = FileHeader {
+        relative_path: "early.bin".into(),
+        size: 4,
+        mtime_seconds: 1_600_000_000,
+        permissions: 0o644,
+        checksum: vec![],
+    };
+    peer.send(wire(Frame::ManifestEntry(header.clone())))
+        .await
+        .unwrap();
+    peer.send(wire(Frame::FileBegin(header))).await.unwrap();
+
+    // The destination must answer with a SessionError frame naming
+    // the violation...
+    let refusal = loop {
+        match recv_or_panic(&mut peer).await {
+            Frame::Error(e) => break e,
+            // need batches may legitimately arrive first
+            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
+            other => panic!("expected SessionError, got {other:?}"),
+        }
+    };
+    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
+
+    // ...and its driver must fail with the same fault.
+    let dest_err = dest.await.unwrap().unwrap_err();
+    assert_eq!(
+        fault_of(&dest_err).code,
+        session_error::Code::ProtocolViolation
+    );
+    assert!(
+        collect_tree(tmp.path()).is_empty(),
+        "no bytes may land from a violating record"
+    );
+}
+
+#[tokio::test]
+async fn need_for_unknown_path_faults_the_source() {
+    let tmp = tempfile::tempdir().unwrap();
+    let src_root = tmp.path().join("src");
+    std::fs::create_dir_all(&src_root).unwrap();
+    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);
+
+    let source_cfg = SourceSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
+        plan_options: PlanOptions::default(),
+    };
+    let (source_transport, mut peer) = in_process_pair();
+    let source = Arc::new(FsTransferSource::new(src_root));
+    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
+
+    // Scripted destination peer: valid handshake, then a need for a
+    // path that was never manifested.
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
+    peer.send(hello_frame()).await.unwrap();
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
+    peer.send(wire(Frame::Accept(Default::default())))
+        .await
+        .unwrap();
+    loop {
+        match recv_or_panic(&mut peer).await {
+            Frame::ManifestEntry(_) => continue,
+            Frame::ManifestComplete(_) => break,
+            other => panic!("expected manifest stream, got {other:?}"),
+        }
+    }
+    peer.send(wire(Frame::NeedBatch(NeedBatch {
+        entries: vec![NeedEntry {
+            relative_path: "never-manifested.txt".into(),
+            resume: false,
+        }],
+    })))
+    .await
+    .unwrap();
+
+    let source_err = source_task.await.unwrap().unwrap_err();
+    let fault = fault_of(&source_err);
+    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
+    assert!(fault.message.contains("never-manifested.txt"));
+
+    // The source must have told the peer why before aborting.
+    let refusal = match recv_or_panic(&mut peer).await {
+        Frame::Error(e) => e,
+        other => panic!("expected SessionError, got {other:?}"),
+    };
+    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
+}
+
+#[tokio::test]
+async fn resume_flagged_need_is_refused_in_non_resume_session() {
+    let tmp = tempfile::tempdir().unwrap();
+    let src_root = tmp.path().join("src");
+    std::fs::create_dir_all(&src_root).unwrap();
+    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);
+
+    let source_cfg = SourceSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
+        plan_options: PlanOptions::default(),
+    };
+    let (source_transport, mut peer) = in_process_pair();
+    let source = Arc::new(FsTransferSource::new(src_root));
+    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
+
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
+    peer.send(hello_frame()).await.unwrap();
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
+    peer.send(wire(Frame::Accept(Default::default())))
+        .await
+        .unwrap();
+    loop {
+        match recv_or_panic(&mut peer).await {
+            Frame::ManifestEntry(_) => continue,
+            Frame::ManifestComplete(_) => break,
+            other => panic!("expected manifest stream, got {other:?}"),
+        }
+    }
+    peer.send(wire(Frame::NeedBatch(NeedBatch {
+        entries: vec![NeedEntry {
+            relative_path: "real.txt".into(),
+            resume: true,
+        }],
+    })))
+    .await
+    .unwrap();
+
+    let source_err = source_task.await.unwrap().unwrap_err();
+    assert_eq!(
+        fault_of(&source_err).code,
+        session_error::Code::ProtocolViolation
+    );
+}
+
+#[tokio::test]
+async fn manifest_entry_after_manifest_complete_is_protocol_violation() {
+    let tmp = tempfile::tempdir().unwrap();
+    let dst_root = tmp.path().join("dst");
+    std::fs::create_dir_all(&dst_root).unwrap();
+
+    let dest_cfg = DestinationSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::Responder,
+    };
+    let (mut peer, dest_transport) = in_process_pair();
+    let dest = tokio::spawn(run_destination(dest_cfg, dest_transport, dst_root));
+
+    peer.send(hello_frame()).await.unwrap();
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
+    peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
+        .await
+        .unwrap();
+    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
+
+    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
+        scan_complete: true,
+    })))
+    .await
+    .unwrap();
+    peer.send(wire(Frame::ManifestEntry(FileHeader {
+        relative_path: "late.txt".into(),
+        size: 1,
+        mtime_seconds: 1,
+        permissions: 0o644,
+        checksum: vec![],
+    })))
+    .await
+    .unwrap();
+
+    let dest_err = dest.await.unwrap().unwrap_err();
+    assert_eq!(
+        fault_of(&dest_err).code,
+        session_error::Code::ProtocolViolation
+    );
+}

exec
/usr/bin/zsh -lc "sed -n '1,220p' docs/DECISIONS.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# DECISIONS — settled choices

**Status**: Active

Append-only ledger of decisions that future sessions must not relitigate or miss.
Add entries via the `decision` procedure in `docs/agent/PROTOCOL.md`. Newest last.
When a decision supersedes plan text, the plan text gets edited in the same
session — this file is the index, not a substitute for fixing the doc.

Format:

```
## D-<YYYY-MM-DD>-<n> — <short title>
- Decision: <one line>
- Why: <one line>
- Supersedes: <doc §/decision ID, or "nothing">
```

---

## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
- Decision: `RELEASE_PLAN_v2_2026-05-04.md` is a frozen reference, no longer the active source of truth.
- Why: 0.1.0 tagged 2026-05-31; the plan served its purpose.
- Supersedes: RELEASE_PLAN_v2_2026-05-04.md as active plan.

## D-2026-05-31-2 — Pick-not-Type TUI direction
- Decision: `TUI_REWORK.md` (dual-pane, M1–M6) supersedes `TUI_DESIGN.md` §6 trigger-modal text inputs and the F3 free-text destination prompt.
- Why: any field requiring the operator to recall and type an off-screen path is an interface failure.
- Supersedes: TUI_DESIGN.md §6 (portions).

## D-2026-06-04-1 — R3 overrides R2 in the audit chain
- Decision: where R2 and R3 disagree on a finding's severity or content, R3 wins; see the ID-override table in `AUDIT_REPORT_2026-06-04_INDEX.md`.
- Why: R3 incorporates the GPT R2 critique and severity rebalance.
- Supersedes: conflicting R2 entries.

## D-2026-06-04-2 — Env vars are out for app + diagnostic config
- Decision: no environment-variable configuration carve-out (R3-L39); purge completed via `audit-l39-m27-env-var-purge`.
- Why: owner policy — config surfaces stay explicit.
- Supersedes: nothing (clarifies prior ambiguity).

## D-2026-06-04-3 — Streaming planner ratified, build deferred
- Decision: `greenfield_plan_v6.md` §1.1 (streaming planner + 1 s heartbeat + 10 s stall detector) is canonical but not yet built; multi-slice implementation queued after audit Round 1 (H10b).
- Why: data-loss/DoS hardening takes priority; the plan claim is ratified rather than retired.
- Supersedes: nothing.

## D-2026-06-06-1 — STATE.md precedence model adopted
- Decision: `docs/STATE.md` is the single entry point for current state, with the precedence order in `AGENTS.md` §1; DEVLOG.md is write-only history, TODO.md is backlog-only, tool-local memories are scratch.
- Why: state smeared across TODO/DEVLOG/plan-README/Serena was the drift mechanism the 2026-06-04 audit documented (drift-* findings, M28).
- Supersedes: "Agent-Specific Expectations" in the previous AGENTS.md (Serena memories as session persistence).

## D-2026-06-07-1 — Keep the `c793df2` octopus on master; no history rewrite
- Decision: `c793df2` (a `git merge -s ours` octopus whose parents are `600023a` + `eafb187` + `d9d4ec7`) stays on `origin/master`; we do **not** rewrite history or force-push to remove it.
- Why: its tree is byte-identical to `600023a` (`git diff 600023a c793df2` is empty) and the workspace builds, so it is cosmetically ugly but harmless; rewriting already-pushed shared history is riskier than the wart. The merge was pushed without owner approval — the corrective is the new AGENTS.md §8 Git-safety contract, not a second unsafe operation.
- Consequence (the trap): because `eafb187` and `d9d4ec7` are now *ancestors* of master, `git branch --merged` falsely reports them merged and a plain `git merge` of either no-ops without landing code. `d9d4ec7` (adaptive-streams-pr3-resizable) does **not** build and its files are not in master's tree. Branch cleanup in this repo is by explicit name only, never `--merged`.
- Supersedes: nothing.

## D-2026-06-07-2 — Adaptive-streams lands via cherry-pick/rebase, excluding the WIP
- Decision: the adaptive-streams stack (live-progress → PR1 telemetry → PR2 work-queue → PR2 review fix, up to `eafb187`) lands later as a planned `docs/plan/` slice via cherry-pick or rebase onto fresh commits — never via `git merge` of the branch (see D-2026-06-07-1 trap). `d9d4ec7` (PR3 WIP, "DOES NOT BUILD") is explicitly excluded until it is finished and compiles.
- Why: the `-s ours` octopus recorded those tips as parents without landing their code, so the feature is not actually in master; a real merge would no-op. The one real conflict (`data_plane.rs`: `StallGuardWriter` vs the `Probe` generic) must be resolved by hand, which only a cherry-pick/rebase surfaces.
- Supersedes: nothing.

## D-2026-06-11-1 — Design-coherence review plan Active; ratification covers Phase A only
- Decision: `docs/plan/DESIGN_COHERENCE_REVIEW.md` flipped Draft → Active. Owner approval authorizes **Phase A only** (concept-ownership map + per-crate stratum inventory); Phases B and C each need a fresh go/no-go at the preceding checkpoint. Interview decisions bound into the plan: blit-tui light pass, owner ratifies each Phase C finding, wire-breaking recommendations in scope (proto not frozen).
- Why: the repo was built by many models across several greenfield restarts and the owner judges it too inconsistently designed to trust as-is; mapping concept ownership precedes any re-scope (audit-h3c slice 2) or feature landing (adaptive-streams) so the fixes get designed once.
- Supersedes: nothing.

## D-2026-06-11-2 — Design-review queue ratified in full; Pull-RPC delete; zero_copy gets a FAST evaluation
- Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
- Why: review program (D-2026-06-11-1) delivered all three phases; owner is the gate for queue entry and exercised it in full.
- Supersedes: nothing (completes D-2026-06-11-1; `DESIGN_COHERENCE_REVIEW.md` flips Active → Shipped).

## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
- Decision: `zero_copy.rs` is deleted rather than wired in. The w8-1b evaluation (`docs/plan/ZERO_COPY_RECEIVE_EVAL.md`) recommended deletion and the owner agreed (2026-06-12 session). The deletion executes inside w8-1 once the w5-1 sentinel (lib.rs) is graded — it is no longer excluded from that sweep.
- Why: the dead draft busy-waits on EAGAIN (would be rewritten, not revived); wiring needs a raw-fd special case beside a permanent buffered fallback; the CPU saving is a fraction of one core, Linux-only, and unmeasured. Revisit gate: 10 GbE benchmarks showing receive-side CPU saturation — design notes preserved in the eval doc.
- Supersedes: D-2026-06-11-2 item (b) (zero_copy exclusion from W8.1 was pending this evaluation; the evaluation is done).

## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
- Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
- Scope consequence: this **moots the standalone premise** of the queued incremental work and absorbs the goals — w2-2 (three ladders → one dial) is `ue-1b`; w2-3 multi-stream pull (`MULTISTREAM_PULL.md`) is `ue-1d` via the unified sequencer; w2-4 (delete deprecated Pull RPC) is `ue-1e`; adaptive-streams cherry-pick is `ue-1a`. `MULTISTREAM_PULL.md` is superseded as a standalone plan (kept as reference); its goal survives inside this plan. The design-review queue's correctness findings (w4-1 etc.) are independent and unaffected.
- Supersedes: the "ground-up redesign" framing of the 2026-06-14 open question recorded in STATE.md (that open question is now closed); `MULTISTREAM_PULL.md` as a standalone plan (goal absorbed into `UNIFIED_TRANSFER_ENGINE.md` slice `ue-1d`).

## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` is **Active**. Owner approved with four parameters that bind the design: (q1) **no probe-then-go phase** — the engine starts moving within ~1s at conservative defaults bounded by the receiver ceiling and the tuner adjusts dials live from the first byte; the "small-transfer threshold" is obviated (no probe to skip), and the **planner** carries the workload-shape judgment (file count vs bytes) that the old size gate proxied. (q2) the receiver advertises a **rich capacity profile** (CPU cores, disk class, load, max streams, drain estimate) — "more data serves the ubergoal"; do not minimize the negotiation payload. (q3) engine type **deferred to the agent**, who recommends a new src/dst-agnostic `TransferEngine` + a local adapter over renaming `TransferOrchestrator` in place — ratified at `ue-1c`. (q4) `ue-2` (mid-transfer stream add/drop via PR3's resize proto) is **in scope at Active**, sequenced last; 11 months of owner benchmarking is the justification, the 10 GbE rig is sign-off not a gate.
- Why: owner answered the four gating questions (the stated Draft→Active condition) and said "active now." q1 materially improved the design — live-from-first-byte removes the fragile size threshold and collapses the A/B/C probe staging into "adjust what is cheap in `ue-1b`, add stream resize in `ue-2`."
- Inference flagged for owner (now vetoed — see D-2026-06-20-3): the agent had proposed folding the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b) in as the planner half and superseding its "after audit Round 1" timing. **Owner vetoed 2026-06-20.** The absorption is dropped; D-2026-06-04-3 stands unchanged. The engine's workload-shape-awareness + first-byte-within-~1s requirements remain, stated on their own merits, not as the H10b concept.
- Supersedes: the "A-first warmup probe" and "size-gated skip-probe" framings in the Draft version of `UNIFIED_TRANSFER_ENGINE.md` (already edited in-place). *(The proposed supersession of D-2026-06-04-3's streaming-planner timing is withdrawn per the owner veto — see D-2026-06-20-3.)*

## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).

## D-2026-06-20-4 — Unified transfer engine plan review freeze
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
- Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.

## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
- Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*

## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
- Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.

## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
- Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
- Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
- Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).

## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
- Supersedes: nothing (closes the STATE.md "commit erratum" blocked item).

## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
- Why: owner call 2026-07-04 ("flip it"). The original "disconnect is the cancel" rationale predates w4-3's race wiring; the flip is now policy-only, and cancel-from-anywhere (second terminal, TUI) is strictly more operable than find-and-kill-the-client.
- Supersedes: the DelegatedPull-only cancellation policy recorded in `active_jobs.rs`'s `supports_cancellation` rustdoc (edited when the slice lands) and the corresponding "policy deliberately unchanged" scope note in the w4-3 finding doc (which anticipated exactly this flip).

## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).

## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.

## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).

## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
- Why: the 10 GbE session showed skippy's 32-core receiver at 1.43 cores — gate not met on that rig — but the gate was always about CPU-bound receivers, and the owner now operates one. On a CPU-bound receiver, cutting the userspace copy is exactly the FAST lever the eval preserved design notes for.
- Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.

## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
- Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
- Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).

exec
/usr/bin/zsh -lc "sed -n '1,240p' .review/findings/otp-3-transfer-session-core.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# otp-3 — TransferSession core (role-parameterized, in-process)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-3.
**Status**: implemented — awaiting codex review.
**Contract**: `docs/TRANSFER_SESSION.md` (post-review, `f861579`).

## What

The unified session state machine exists in blit-core and moves real
bytes in-process, with the roles swappable over the same fixtures.
`run_source` / `run_destination` implement the contract's HELLO →
OPEN/ACCEPT → role-lane phases over a `FrameTransport`; the byte
carrier is the in-stream frame grammar (file records + tar-shard
records, strictly serialized, fail-fast). The owner's invariance
property is now in the test suite: every fixture runs under both
initiator layouts and must produce the identical need-list set,
identical summary, and byte-identical destination tree.

## Approach (as implemented)

- `crates/blit-core/src/transfer_session/transport.rs` —
  `FrameTx`/`FrameRx` halves + `FrameTransport` (splittable) +
  `in_process_pair()` on bounded mpsc (64 frames/direction).
- `crates/blit-core/src/transfer_session/mod.rs` —
  - `session_build_id()` = `CARGO_PKG_VERSION+BLIT_GIT_SHA[.dirty]`
    (build.rs emits the sha; rerun-if-changed on `.git/HEAD` +
    `.git/refs`; dirty flag sampled at build-script time, best-effort
    by nature). `CONTRACT_VERSION = 1`. Exact-match hello both ways;
    mismatch → `SessionError{BUILD_MISMATCH}` naming both ids
    (D-2026-07-05-2).
  - `establish()` — ONE hello/open/accept implementation both role
    drivers call. Responder-side capability validation refuses what
    later slices implement (mirror → otp-6, filters → otp-6, resume →
    otp-7) with a `SessionError` instead of accepting — fail-fast, no
    silent ignoring. Receiver capacity travels DEST→SOURCE at setup
    (open when initiator is DEST, accept when responder is), consumed
    from otp-4 when the dial attaches.
  - SOURCE driver: split into a send half and a dedicated receive
    half (deadlock-freedom: the transport is bounded both ways, so a
    single loop that blocks on send while the peer blocks on its own
    send would deadlock — the recv half always drains). Needs are
    validated against the sent-manifest map (unknown / duplicate /
    resume-flagged → PROTOCOL_VIOLATION), which bounds the internal
    event queue by the source's own manifest size. Payloads plan per
    accumulated need batch via `diff_planner::plan_push_payloads`,
    emit as file records (`file_begin` + `file_data`×N, completion at
    exactly `header.size`) and tar records (existing tar builder via
    `prepare_payload`), only after `ManifestComplete` (in-stream
    carrier rule). `SourceDone` only after `NeedComplete` + queue
    drained; then awaits the destination's summary.
  - DESTINATION driver: sequential frame loop (its sends can't
    deadlock because the source's recv half always drains). Manifest
    entries buffer into 128-entry chunks (w4-4 rationale) and
    stat+compare on the blocking pool; need batches stream back
    mid-manifest; `NeedComplete` only after ManifestComplete + all
    entries diffed. File records write through
    `FsTransferSink::write_file_stream` fed by a bounded
    `tokio::io::duplex` pipe (256 KiB); tar records buffer to exactly
    `archive_size` (≤ `MAX_TAR_SHARD_BYTES`, `try_reserve_exact`) and
    unpack through the existing tar-safety path
    (`write_payload(TarShard)`). Grammar violations (payload before
    ManifestComplete, record interleave/overrun/short-complete,
    payload not on the need list, `SourceDone` with outstanding
    needs, resume/resize frames in an otp-3 session) →
    `SessionError{PROTOCOL_VIOLATION}` + abort. Diff stats go through
    the same canonical-containment chokepoint as sink writes
    (R46-F3): an escaping manifest path is a violation, not a stat.
  - Faults are `SessionFault` (wire code + message + both build ids +
    peer_notified), carried in `eyre::Report` — tests downcast and
    assert codes. An end that aborts sends the error frame first
    unless the peer already knows.

### Deviations from the scoped approach (2026-07-05 survey)

1. **Destination diff predicate**: the scoping note named
   `diff_planner::filter_unchanged`, but that predicate stats BOTH
   sides locally — impossible for a wire destination and a role-
   separation leak in-process (otp-4 must be transport substitution,
   not new choreography). The mode-aware header-vs-target owner that
   already exists is `manifest::compare_manifests`; its per-entry
   body is now extracted as `manifest::header_transfer_status`
   (public), `compare_manifests` is refactored onto it, and the
   session destination feeds it from a live stat. Single-owner intent
   preserved; `From<ComparisonMode> for CompareMode` added alongside.
2. **`DestinationOutcome`**: `run_destination` returns
   `{summary, needed_paths}` rather than bare summary — the role
   suite pins need-set equality across role assignments, which the
   scoping called for but the driver didn't expose.
3. `SessionEndpoint::Initiator` boxes its `SessionOpen`
   (clippy large-enum-variant); `SessionEndpoint::initiator()`
   constructor provided.

## Files

- `crates/blit-core/src/transfer_session/{mod.rs,transport.rs}` (new)
- `crates/blit-core/src/lib.rs` (module export)
- `crates/blit-core/build.rs` (BLIT_GIT_SHA emission)
- `crates/blit-core/src/manifest.rs` (`header_transfer_status`
  extraction + `From<ComparisonMode>`; `compare_manifests` behavior
  unchanged)
- `crates/blit-core/Cargo.toml` (filetime added to dev-deps for the
  fixture suite)
- `crates/blit-core/tests/transfer_session_roles.rs` (new, the
  role-parameterized suite)

## Tests

Suite 1484 → 1500 (+16; count never dropped). New:

- `transfer_session_roles.rs` (12): small mixed tree (multi-chunk
  3 MiB file, empty file, spaced/nested names) byte-identical under
  both initiators with identical need sets + summaries; 200-file
  force-tar tree likewise (tar record grammar both layouts);
  incremental pre-seeded destination needs exactly {changed,
  missing}; identical pre-seeded tree yields empty need list and
  0/0 summary; mtime preservation on streamed files; build-id
  mismatch refused both ends under both initiator layouts (message
  names both ids, no bytes move); contract-version mismatch refused;
  mirror-enabled open refused with the otp-6 pointer; scripted-peer
  violations fail fast (payload record before ManifestComplete, need
  for never-manifested path, resume-flagged need, manifest entry
  after ManifestComplete) with the error frame observed on the wire.
- `transport.rs` (2): pair delivery both directions; closed-peer
  semantics.
- `mod.rs` (2): build-id shape; fault wire round-trip (perspective
  swap included).

Gate: `cargo fmt --check` ✓, `clippy --workspace --all-targets
-D warnings` ✓, `cargo test --workspace` 1500/0 ✓.

## Known gaps (carried forward)

- **SizeMtime semantic divergence, decided at otp-4/5 parity**: the
  session inherits `manifest::compare_file`'s Default arm (transfer
  when src NEWER; skip when target same-age-or-newer) — today's
  pull_sync semantic. Today's push daemon uses exact size+mtime
  equality instead, so a destination file with newer mtime but
  different content re-transfers under old push and is skipped by
  the session. The otp-4 A/B parity pins against old push will
  surface this; picking the unified semantic (and whether
  `compare_file`'s Default arm changes) is that slice's recorded
  decision. otp-3 deliberately did not change live pull_sync
  behavior by editing the shared arm.
- Checksum compare mode transfers everything when headers carry no
  checksum (manifest enumeration never populates it today) — the
  conservative arm of `compare_file`, parity with today's push.
  Whether the session grows source-side checksum population is a
  parity-slice call (otp-4/5).
- Strict `SourceDone`: a needed file that vanishes source-side
  mid-transfer faults the session (`INTERNAL` on read failure /
  EOF-short). Old push tolerates and skips (`check_availability`,
  unreadable list). The contract has no "source skipped these"
  notification yet; if parity requires tolerance, that's a contract
  addendum at otp-4/5, not silent skipping.
- Need-batch cadence knobs (the FileListBatcher's 5 ms/64 KiB early
  flush + 25 ms max delay) are not replicated in-process; batches
  flush per 128-entry diff chunk and at ManifestComplete. Cadence
  matters when a real wire + mid-manifest data-plane spin-up exist —
  otp-4.
- Single-file source roots (`relative_path = ""` wire form) are
  untested against the session; parity slices own that edge.
- `require_complete_scan` + `ManifestComplete.scan_complete` travel
  the wire but gate nothing until mirror (otp-6).
- Resize frames on the in-stream carrier are treated as
  PROTOCOL_VIOLATION (no data plane exists to resize in otp-3); the
  frame table marks them any-phase for sessions WITH a plane —
  otp-4 wires the real semantics.
- In-process transport caps frames, not bytes (64 × ≤1 MiB payload
  frames ≈ 64 MiB/direction worst case). Fine for tests and local
  use; the wire carrier has HTTP/2 byte-level flow control.

exec
/usr/bin/zsh -lc "sed -n '1,220p' .agents/state.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/TRANSFER_SESSION.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Agent State (pointer)

Current state for this repo lives canonically in **`docs/STATE.md`** — read that
file, not this one. This stub exists only so tools expecting the standard
`.agents/` layout find their way.

Do not record state here. The path `docs/STATE.md` is load-bearing: the CI docs
gate (`.github/workflows/docs-gate.yml`), the doc lint
(`scripts/agent/check-docs.sh`), the session hooks (`scripts/agent/context.sh`),
and the `handoff` procedure in `docs/agent/PROTOCOL.md` all read or enforce it.

- Now / Next / Blockers: `docs/STATE.md`
- Verification commands: `.agents/repo-map.json` (mirrors `AGENTS.md` §5)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`

 succeeded in 0ms:
# TransferSession wire + session contract (otp-1)

**Status**: Active (contract; implementation lands otp-3..otp-10)
**Created**: 2026-07-05
**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)

This document is the authoritative contract for the single `Transfer`
RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
session"; this doc explains the state machine the proto cannot.

## Invariants

1. **One vocabulary, role-tagged.** Both wire directions carry the
   same frame type (`TransferFrame`). Which frames an end may send is
   determined by its ROLE (SOURCE or DESTINATION), never by whether
   it is the gRPC client or server. This is the structural form of
   the owner's invariant: there is no push-shaped or pull-shaped
   message set to diverge.
2. **Same build only (D-2026-07-05-2).** The first frame each way is
   `SessionHello{build_id, contract_version}`. Both ends compare for
   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
   naming both ids, then stream close. No negotiate-down, no advisory
   fields, no feature-capability bits — same build implies same
   features. `build_id` = `<crate version>+<git commit hash>[.dirty]`
   composed at compile time; `contract_version` is a belt-and-braces
   integer bumped on any wire-shape change (exact match required).
3. **Roles.** The initiator (the end that opened the RPC — a CLI
   client, or a daemon acting as delegated initiator) declares in
   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
   (always a daemon) takes the other role. All four
   initiator/role combinations run the identical state machine.
4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
   from live enumeration (immediate start — no buffered-enumeration
   phase in any direction). DESTINATION diffs incrementally against
   its own filesystem and streams need batches back. DESTINATION is
   authoritative for what it has; SOURCE is authoritative for what
   exists to send.
5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
   (whichever end holds DESTINATION) advertises its
   `CapacityProfile` at session open — in `SessionOpen` when the
   initiator is DESTINATION, in `SessionAccept` when the responder
   is. The byte SENDER (SOURCE) owns the live dial bounded by that
   profile. Absent/0 profile fields mean "unknown hardware value" —
   conservative defaults, never unlimited, and NEVER "old peer"
   (there are no old peers).
6. **One stream policy.** The data plane opens at the dial floor
   immediately; SOURCE shape-corrects the stream count upward via
   resize as the need list accumulates (the sf-2 mechanism —
   `TransferDial::propose_shape_resize` — now the only policy).
   SOURCE is the resize controller in every session.

## Phase state machine

```
INITIATOR                                RESPONDER
  |-- SessionHello ----------------------->|   (phase: HELLO)
  |<------------------------ SessionHello--|
  |     both verify build_id exact match; mismatch => SessionError + close
  |-- SessionOpen ------------------------>|   (phase: OPEN)
  |<---------------------- SessionAccept --|
  |     responder validates module/path/read-only/gate here;
  |     refusal is a SessionError, never a silent close
  |                                        |
  |==== from here the lanes are ROLES, not initiator/responder ====|
  |  (whichever end holds SOURCE sends source-lane frames,          |
  |   regardless of which end opened the RPC)                       |
  |                                                                 |
  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
  |  DEST streams:    NeedBatch* ... NeedComplete                  |
  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
  |                   frames when the in-stream carrier is chosen) |
  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
  |                                                                 |
  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
  |  `resume=true` is followed by DEST's BlockHashList for that     |
  |  file BEFORE SOURCE may send any byte of that file; stale or    |
  |  mismatched partials fall back to full-file transfer.           |
  |                                                                 |
  |  mirror: DEST computes deletions LOCALLY from the completed     |
  |  source manifest (filter-scoped, scan-complete-guarded) and     |
  |  executes them itself. No delete list crosses the wire.         |
  |                                                                 |
  |  CLOSING (role-directed, both initiator layouts):               |
  |    SOURCE -> DEST:  SourceDone (all requested payloads flushed) |
  |    DEST -> SOURCE:  TransferSummary (DEST is the scorer)        |
  |  then the INITIATOR closes the RPC stream.                      |
```

- Phase violations (a frame arriving in a phase where its role may
  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
  fail-fast, no tolerant parsing.
- `NeedComplete` is DESTINATION's promise that no further need
  batches follow (SOURCE may finish after flushing what was asked).
  It may be sent only after BOTH: the source's `ManifestComplete`
  has been received AND the destination has finished diffing every
  received manifest entry. Mirror deletions additionally require the
  scan-complete guard, as above.
- **Flow control is the transport's, deliberately:** manifest, need,
  and in-stream payload frames ride gRPC/HTTP-2 stream flow control;
  each end holds only bounded internal queues (the engine's existing
  batching — 128-entry manifest check chunks, need-list batcher).
  Nothing in the contract requires unbounded buffering of the peer's
  stream, and implementations must not introduce it.
- `TransferSummary` always travels DESTINATION → SOURCE (the end
  that wrote bytes and executed deletes is the end that can attest
  to them), then the initiator surfaces it to the operator.

## Frame set and field numbers

`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`

`TransferFrame.frame` oneof (field numbers frozen by this doc):

| # | frame | sender | phase |
|---|-------|--------|-------|
| 1 | `SessionHello` | both, first frame | HELLO |
| 2 | `SessionOpen` | initiator | OPEN |
| 3 | `SessionAccept` | responder | OPEN |
| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
| 10 | `FileData file_data` | SOURCE | in-stream carrier |
| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
| 14 | `BlockTransfer block` | SOURCE | resume |
| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
| 18 | `SourceDone source_done` | SOURCE | closing |
| 19 | `TransferSummary summary` | DESTINATION | closing |
| 20 | `SessionError error` | both | any |

Reused messages (`FileHeader`, `FileData`, `TarShard*`,
`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
existing shapes — the session reuses the engine's payload vocabulary
verbatim. New messages (`SessionHello`, `SessionOpen`,
`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
defined in the proto with their field numbers.

Deliberately absent: `PeerCapabilities` (same build = same
features), `spec_version` negotiation (the hello's exact match
replaces it), any delete list (mirror is destination-local), any
push/pull-specific message.

## Transport selection

- **TCP data plane (default):** the RESPONDER binds the listener and
  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
  dials (NAT/firewall reality — connection topology, not
  choreography). Byte direction on the sockets is set by role:
  SOURCE writes, DESTINATION reads.
  **`initial_streams` is an ACCEPT ceiling, not a dial order**
  (D-2026-06-20-1/-2 preserved): it is the number of epoch-0 accept
  slots the responder arms, computed as min(engine dial floor,
  DESTINATION's capacity ceiling). SOURCE — wherever it sits — owns
  the dial and may use fewer epoch-0 sockets than armed; unclaimed
  slots expire harmlessly. Growth beyond epoch 0 happens only via
  SOURCE-initiated resize (sf-2 shape correction / tuner), one armed
  accept per ADD epoch, exactly as ue-r2-2 built.
  **Socket auth, exact:** every epoch-0 socket opens with
  `session_token` (16 bytes) immediately followed by
  `epoch0_sub_token` (16 bytes); every resize-ADD socket opens with
  `session_token` followed by that epoch's `sub_token` from the
  `DataPlaneResize` frame. Tokens are single-session; each armed
  accept slot admits exactly one socket (no replay within a
  session); armed slots that go unclaimed expire, as today's resize
  wiring already does. A socket presenting anything else is closed
  without response.
- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
  (operator `--force-grpc` diagnostics) or granted by the responder
  when it cannot bind a data plane (`SessionAccept` with no grant).
  Payload frames 9-15 ride the RPC itself. Same choreography, same
  planner decisions, different byte carrier.
  **Record grammar (fail-fast):** payload records on the
  source-lane are STRICTLY SERIALIZED — after `file_begin(header)`,
  only `file_data` frames for that file may follow on the lane until
  the record completes; completion is inferred at exactly
  `header.size` cumulative bytes (a `file_begin`/`tar_shard_header`/
  `block` arriving early, or bytes overrunning `size`, is
  `PROTOCOL_VIOLATION`). Tar-shard records run
  `tar_shard_header … tar_shard_chunk* … tar_shard_complete`; block
  records complete with `block_complete`. Payload records may begin
  only AFTER the source's `ManifestComplete` — this per-transport
  ordering rule applies identically to both roles and mirrors the
  design-4-proven fallback ordering, so manifest frames and payload
  records never interleave. DESTINATION-lane frames (need batches,
  acks, summary) are unaffected — they travel the other direction.
- **Local (in-process):** the identical session state machine runs
  with both roles in one process over an in-process frame channel —
  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
  file vs block) is planner-owned and reads workload shape +
  capability, never role/initiator/transport.

## Errors, cancel, stall

- `SessionError{code, message}` codes (plus both build ids on
  BUILD_MISMATCH):
  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
  or aborts says why before closing; operators never diagnose from a
  bare stream reset.
- `CancelJob` interop: the responder registers the session in
  `ActiveJobs` at OPEN (same transfer_id contract as today); the
  cancel token races the session exactly as w4-3 wired, and the
  peer receives `SessionError{CANCELLED}`.
- StallGuard, byte-accounting, and progress events (w6-1 contract)
  attach at the same boundaries they do today; the session emits the
  existing `DaemonEvent` payloads.

## What this replaces

At cutover (otp-10): `Push`, `PullSync`, and their message
choreographies are deleted from the proto and the tree; the four
per-direction drivers die with them; `DelegatedPull` shrinks to
trigger + progress relay (no payload bytes). Until then this
contract's surface exists compiled-but-refusing
(`Transfer` returns `UNIMPLEMENTED`; pinned by test).

exec
/usr/bin/zsh -lc "sed -n '1,220p' .agents/repo-guidance.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Repo-Specific Guidance
<!-- Extends AGENTS.md; never overrides it. Rules and pointers only — state
     lives in .agents/state.md. -->

## Mission Detail

Blit is a high-performance, extensible file enumeration, planning, transfer,
and orchestration platform for local and remote backups, migration, and
cross-platform syncing, with CLI and daemon interfaces (`crates/blit-cli`,
`crates/blit-daemon`), async-aware planning, and Windows/Linux/macOS support.

## Reading Order

This repo predates the toolkit's `.agents/state.md` / `.agents/decisions.md`
convention and keeps its own canonical files at different paths; the
`.agents/` files below are pointer stubs, not duplicates. Read in this order:

1. `docs/STATE.md` — single entry point for current active work, queue, and
   blockers (the canonical equivalent of `.agents/state.md`; see
   `.agents/state.md` for why the path differs).
2. The active plan doc(s) `docs/STATE.md` names (under `docs/plan/`).
3. `REVIEW.md` + `.review/` — review-loop status for in-flight findings.
4. `docs/DECISIONS.md` — settled decisions and supersessions (the canonical
   equivalent of `.agents/decisions.md`).
5. `docs/agent/PROTOCOL.md` — the executable procedures behind the trigger
   vocabulary (`catchup`, `plan`, `decision`, `handoff`, `drift`, plus the
   repo-specific `slice` operator below).
6. Everything else in `docs/` — reference or historical; check its
   `**Status**:` header.
7. Code and tests are ground truth for behavior; plans are ground truth for
   intent. A mismatch is a drift finding, not permission to pick whichever is
   convenient.

`DEVLOG.md` is append-only history — write to it, never read it for current
state. `TODO.md` is the long-horizon backlog; the actionable queue lives in
`docs/STATE.md` and `REVIEW.md`. `.serena/memories/` and any tool-local
memory are scratch, never authoritative.

## Operator Vocabulary (repo-specific extension)

`AGENTS.md`'s Operator Requests section defines the toolkit's generic
vocabulary (`catchup`, `handoff`, `drift`, `decision`, `plan`, `playbook`).
In this repo every one of those words resolves to a procedure in
`docs/agent/PROTOCOL.md`, not to the generic `.agents/state.md`/
`.agents/decisions.md` files directly — read the matching section there and
execute it exactly:

- `catchup` → re-ground from `docs/STATE.md` + active docs; summarize
  now/next/blockers.
- `plan <topic>` → interview the owner, write `docs/plan/<NAME>.md`; no code
  until `**Status**: Active`.
- `decision <topic>` → record in `docs/DECISIONS.md`, propagate
  supersessions.
- `handoff` → update `docs/STATE.md` for the next session; prune to caps.
- `drift [scope]` → audit a doc against code; fix docs, file findings, raise
  questions.
- `slice` (repo-specific, no generic-template equivalent) → pick up the next
  review finding and run it through the codex review loop
  (`docs/agent/GPT_REVIEW_LOOP.md`).

**Review policy (D-2026-07-04-1): every code change and every plan change
goes through the codex review loop in `docs/agent/GPT_REVIEW_LOOP.md` — no
exceptions.** The `.review/README.md` async sentinel hand-off is retired;
its `findings/`/`results/` records and `REVIEW.md` remain the record store.

Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
Antigravity exposes `catchup`/`handoff` as workspace skills in
`.agents/skills/`. This repo does not currently use `.agents/playbooks/` —
the codex review loop and `docs/agent/PROTOCOL.md` already cover that role
for review-loop work.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Test count may grow but never drop versus the prior baseline unless the
  removal is called out in the finding doc's Known gaps.
- Windows parity: after touching platform-specific code (`win_fs`, planners),
  run `scripts/windows/run-blit-tests.ps1`.
- Docs gate (CI): a push touching `crates/**` or `proto/**` must also touch
  `docs/STATE.md`, unless the commit message contains `[state: skip]`
  (reserved for mechanical changes). `scripts/agent/check-docs.sh` must pass;
  run it locally before pushing docs changes.
- Full command list and policy also live in `.agents/repo-map.json`.

## Remotes & Sync

- `origin` — `https://github.com/roethlar/Blit.git` (GitHub, canonical).
- `gitea` — `http://q:3000/michael/blit_v2.git` (LAN gitea mirror; pushed
  manually alongside or after `origin`, not auto-synced by any hook or CI
  job — it can lag GitHub by a commit or more at any given time).
- (Names verified against `git remote -v` 2026-07-04; an earlier revision
  of this doc called GitHub `github` and the mirror `origin` — that never
  matched the actual config and misread `origin/master` references.)
- Push policy: `.agents/push-policy.md` (ask). This repo's git-safety rules
  go well beyond a simple push policy — see Earned Practices below.

## Earned Practices

These are absolute; they exist because an unapproved `git merge -s ours`
octopus (commit `c793df2`) was pushed to `origin/master` without the owner's
consent (`docs/DECISIONS.md` D-2026-06-07-1).

- **No agent-created branches.** Agents never create git branches on their
  own decision. All work happens on `master` or the branch the owner already
  checked out.
- **Owner is the sole gate for git operations that publish, rewrite, or
  destroy.** No `push`, `push --force`/`--force-with-lease`,
  `reset --hard`, rebase or other history rewrite, `commit --amend` on
  pushed commits, or deletion of any branch/tag/ref (local or remote)
  without the owner approving that exact action in the current session.
  Working-tree edits, local commits, and read-only inspection
  (`status`/`log`/`diff`/`show`) need no special approval.
- **Branch deletion is by explicit name only** — the owner names the branch,
  the agent deletes that branch.
- **Before any push:** list the exact local refs, remote refs, and
  destination remotes, then stop and wait for approval.
- **`--merged`/`--no-merged` are unreliable in this repo.** The `-s ours`
  octopus made two now-abandoned branch tips ancestors of `master`, so
  `git branch --merged master` falsely lists them as merged and a plain
  `git merge` of those branches no-ops without landing any code
  (`docs/DECISIONS.md` D-2026-06-07-2). Verify content actually arrived
  (`git diff <branch> master`) before treating anything as landed or
  deleting it.
- **Checkpoints are owner-only.** Only an explicit owner message satisfies a
  checkpoint or verification step. Agents report observations; the owner
  declares pass/fail. Never self-certify a gate or continue a plan past one
  because the condition appears met. Approvals are single-use, step-specific,
  never carried across sessions. When the owner asks a question or thinks out
  loud, answer in plain English and stop — act only on an explicit decision.

## Style

- Rust edition 2021; format with rustfmt. Modules snake_case, types
  PascalCase, constants SHOUT_CASE; match existing names (`transfer_engine`,
  `TransferOrchestrator`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, transfer engine,
  orchestrator); most logic and unit tests live here. New modules get
  re-exported in `crates/blit-core/src/lib.rs`.
- `crates/blit-cli/`, `crates/blit-daemon/` — CLI and daemon binaries; admin
  verbs (scan, ls, find, du, df, rm, completions, profile, list-modules) live
  in `blit-cli` alongside transfer commands.
- `crates/blit-app/`, `crates/blit-tui/` — TUI application layers.
- `crates/blit-prometheus-bridge/` — metrics bridge.
- `proto/blit.proto` — gRPC definitions; `blit-core`'s build script vendors
  protoc.
- Integration tests live per-crate (`crates/blit-cli/tests/`,
  `crates/blit-core/tests/`); the root `Cargo.toml` is a virtual workspace,
  so a root-level `tests/` dir would never be compiled. `scripts/` — helper
  tooling.
- `blit-utils` was intentionally removed; its admin verbs now live in
  `blit-cli`. Do not re-add it as a workspace member.

## Commit and Docs Hygiene

- Commit subject: short imperative ("Add streaming planner heartbeat").
  Review-loop commits: `Fix <id>: <one-line summary>` per `.review/README.md`.
- After meaningful work: append a `DEVLOG.md` entry (newest-first, ISO
  timestamp) and update `docs/STATE.md` — the `handoff` procedure does both.
- Every doc in `docs/plan/` carries a `**Status**:` header, one of:
  `Draft | Active | Shipped | Superseded | Historical`. Superseding a doc
  requires a `docs/DECISIONS.md` entry naming winner and loser, and an edit
  to the superseded text.
- `docs/STATE.md` stays ≤ 200 lines with ≤ 3 handoff entries; prune the
  overflow into `DEVLOG.md`.

exec
/usr/bin/zsh -lc "sed -n '1,240p' docs/plan/ONE_TRANSFER_PATH.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# One transfer path — direction-invariant by construction

**Status**: Active
**Created**: 2026-07-05
**Supersedes**: post-REV4 residue item "pull 1s-start restructuring"
(absorbed here); pauses `docs/plan/SMALL_FILE_CEILING.md` after sf-2
(D-2026-07-05-1). REV4's mixed-version-peers constraint is superseded
outright by **D-2026-07-05-2 (no version compatibility, ever — same
build only)** — annotated in REV4 §Constraints
**Decision ref**: D-2026-07-05-1 (directive + pause);
**D-2026-07-05-4 (Draft → Active, owner "flip the plan and go",
2026-07-05)**

## Directive (owner, 2026-07-05, verbatim)

> "make ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF
> ANYTHING EVER using anything else because anything else does not
> exist."

> "just make it so that I NEVER see a situation where pull is faster
> than push or vice versa. that CAN NEVER be possible because of
> something blit did. it should be identical if I start the transfer
> from skippy and push to this machine or if I start the transfer on
> this machine and pull from skippy."

> On benchmark methodology: "tmp on one side, spinning rust on the
> other is not a valid test."

Scope, wire, and process were explicitly delegated to the agent
("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
owner's requirement is the invariant; everything below is the
architecture that makes the invariant impossible to violate rather
than merely maintained by discipline.

## Goal

One `TransferSession` implementation owns every byte transfer blit
performs. A transfer has a SOURCE role and a DESTINATION role; which
end initiated, and which CLI verb was used, select roles — they do not
select code. When this plan ships, the per-direction drivers (client
push driver, daemon push-receive, client pull driver, daemon
pull-send, delegated-pull driver, local orchestration) **do not
exist**: for fixed endpoints and dataset, direction/initiator/verb
cannot affect behavior or wall time by blit's doing, because there is
no second code path to differ.

## Non-goals

- Version compatibility of ANY kind (D-2026-07-05-2, owner standing
  rule: "backward compatibility is NOT a consideration... same build
  only. do not engineer tech debt into an unshipped product"). A blit
  client talks only to a blit-daemon from the same build; the session
  handshake REFUSES a mismatched peer outright. No negotiate-down, no
  advisory fields, no feature-capability bits for version skew.
  `Push`/`PullSync` are deleted at cutover with no bridge. (Old-path
  code coexists in-tree during the migration slices solely so each
  slice lands green — that is migration scaffolding, not wire
  compatibility.)
- Making different hardware perform identically. If src and dst sit
  on different disks, the two *data directions* still differ by
  physics; the invariant is that the same data direction between the
  same endpoints is identical regardless of who initiates and which
  verb is used.
- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
- New features. This is a consolidation; capability parity with
  today (mirror, filters, resume, fallback, delegation, progress,
  jobs, cancellation) is the bar. Zero-copy receive is **unparked**
  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
  after cutover, not one of this plan's slices — see the Design note
  on the write-strategy seam.

## Constraints

- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
  violated at the choreography layer.
- **Converge up, not down**: per benchmark cell, the unified session
  must match the better of today's two directions (within ±10% run
  noise), not their average. Unification that slows the fast
  direction fails review.
- REV4 invariants carry: byte-identical results, StallGuard,
  cancellation, byte-accounting. Existing pins are ported (not
  dropped) as tests become role-parameterized; test count never
  drops.
- The sf-2 shape-correction behavior (stream count corrects as the
  need list accumulates) becomes the one and only stream policy —
  both directions inherit it by construction; its pins carry over.
- **The bounded-unilateral dial contract carries unchanged**
  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
  dial, bounded by the byte RECEIVER's advertised capacity profile
  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
  unlimited). The session's role model must express this — profile
  travels DESTINATION→SOURCE at setup regardless of who initiated —
  and otp-1's contract names it explicitly.
- Wire contract discipline (REV4 rule): the unified session's proto —
  messages, field numbers, capability negotiation, transport
  selection — is a reviewed doc+proto slice **before** any behavior
  depends on it.
- Every slice through the codex loop (D-2026-07-04-1); tree green
  after every slice; transitional coexistence of old+new paths is
  scaffolding only — the plan is not Shipped until the deletion slice
  lands and the deletion proof is recorded.
- Windows parity: suite green on the owner's machine + windows-latest
  CI before Shipped.

## Acceptance criteria

- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
      on a symmetric rig (same filesystem class both ends, cold
      caches, disk-to-disk), for each data direction and workload
      (large / 10k-small / mixed): wall time initiating from end A vs
      end B, and via push-verb vs pull-verb, differs only within
      run-to-run noise (±10%). Matrix committed as evidence.
- [ ] **Converge up, measured (codex F4)**: before cutover, the
      corrected symmetric-fs harness records a per-cell baseline of
      the OLD paths, both directions; after cutover, every unified
      cell must be ≤ the better of that cell's two old directions
      + run noise (±10%). A symmetric-but-slower result fails.
- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
      (driver), daemon `push/control.rs` choreography, daemon
      `pull_sync.rs` choreography, the delegated-pull driver, the
      separate local orchestration path, and the `Push`/`PullSync`
      RPCs no longer exist in the tree; one `TransferSession` and one
      `Transfer` RPC remain. The `DelegatedPull` RPC may survive only
      as trigger + progress relay — the proof must show it carries no
      payload bytes (codex F3). Recorded file-by-file in the final
      slice's finding doc.
- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
      guard), filters, block-resume, gRPC fallback carrier, delegated
      transfer, progress events, jobs/cancel, read-only enforcement —
      each demonstrated by ported tests on the session.
- [ ] Suite green throughout; final test count ≥ pre-plan baseline
      (1483); all REV4 invariant pins and the sf-2 pin pass
      role-parameterized.
- [ ] Benchmark methodology corrected and recorded: symmetric-fs
      cells are the verdict cells; tmpfs cells remain only as
      explicitly-labeled wire-reference rows (never compared across
      directions with asymmetric endpoints).
- [ ] Windows: full suite green (owner machine) + windows-latest CI.

## Design

**What already is one code** (kept, becomes the session's engine):
`remote/transfer/` — pipeline, sink/source abstractions, data plane,
diff planner, tar-shard, stall guard, progress, `operation_spec` (the
REV4 unified contract), and the engine dial (stream policy incl. sf-2
shape correction). The defect layer is above it: four driver loops
choreograph these pieces differently per direction.

**The one choreography** (roles, not directions):

1. Initiator opens the single bidi `Transfer` RPC and sends the
   operation spec: which end is SOURCE, which is DESTINATION, path/
   module, filters, mirror/resume flags, capabilities.
2. SOURCE enumerates and **streams** its manifest immediately (no
   buffered-enumeration phase — this generalizes push's fast start;
   pull's full-enumeration-then-negotiate slow start is deleted, which
   absorbs the "pull 1s-start" residue item).
3. DESTINATION diffs incrementally against its own filesystem and
   returns need-list batches (one diff owner, always the end that
   owns the target fs — push's proven model; pull_sync's
   source-side diff is deleted).
4. The data plane opens at the dial floor immediately; stream count
   shape-corrects as the need list accumulates (sf-2 mechanism, now
   the only policy, both roles).
5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
   the one pipeline into the data plane; DESTINATION writes through
   the one receive path. The receive sink is built with a
   **runtime-selected write-strategy seam**: buffered relay is the
   universal strategy; capability-gated alternatives slot in behind
   it without new paths — the first is zero-copy/splice
   (D-2026-07-05-3, unparked for CPU-bound receivers like the
   owner's UNAS 8 Pro; design input:
   `ZERO_COPY_RECEIVE_EVAL.md` §If-FAST-evidence), landing as a
   follow-on slice set after cutover. Strategy selection reads
   capability and payload type, never role or initiator.
6. Mirror: DESTINATION computes deletions from the completed source
   manifest it received (filter-scoped, scan-complete-guarded) and
   executes them locally. One rule, no per-direction delete
   choreography.
7. Resume: optional block-hash phase inside the same session, same
   messages regardless of roles.
8. Summary/byte-accounting: one record shape.

**Transport facts vs choreography**: the connection-initiating end
dials TCP data-plane sockets (NAT reality) — byte direction within a
socket is set by role, not by who dialed. The gRPC-fallback lane
becomes a *byte-carrier option* inside the same session (control-
stream frames instead of TCP sockets), selected at negotiation — not
a separate transfer path. Resize keeps its controller-at-sender rule.

**Delegated transfer**: a daemon receiving a delegated request simply
becomes an initiator of the same session against the other daemon
(destination role on its module fs). The bespoke delegated-pull
driver is deleted; the delegation *gate* (authorization) stays. The
`DelegatedPull` RPC itself is client↔daemon trigger + progress relay
(`DelegatedPullProgress` stream) — it never carries payload bytes;
its handler shrinks to "authorize, spawn the session, relay the
session's progress events." It stays wire-compatible or is folded at
cutover — either way the deletion proof asserts no bytes flow
through it (codex F3).

**Resume ordering (RELIABLE exception, codex F5)**: resumed files use
a strictly-ordered block-hash exchange — the DESTINATION's block map
for a file must complete before the SOURCE sends any block of that
file, and stale/mismatched partials fall back to full-file transfer.
This is an explicit exception to the immediate-start rule, exactly as
today's resume path is an explicit single-stream RELIABLE exception
(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
contract; otp-7 pins the stale-partial and mid-resume-failure cases
in tests.

**Local transfers**: the same session driver over an in-process
transport (both roles in one process, no wire). The engine underneath
is already shared; the separate local orchestration path is deleted
in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
guard the migration.

**Affected crates**: `blit-core` (new `transfer_session` module;
`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
`blit-tui` (progress/jobs consume the same events).

**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
mitigated by strangler slices with the tree green throughout and a
non-optional deletion slice. Per-cell regression risk on today's
faster direction — mitigated by the converge-up constraint and
baseline parity pins per slice. Wire break — lockstep upgrade,
owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
Progress/jobs/TUI integration churn — the session emits the existing
event contract (w6-1) at the same boundaries.

## Slices

One coherent, testable change per slice — sized for the `.review/`
loop. Tree green after every slice; old paths keep working until
otp-9 deletes them.


codex
The commit diff is large, so I’m switching from the truncated full show to file-level reads at the committed tree. That will let me cite exact line numbers in any findings instead of relying on the truncated patch.
exec
/usr/bin/zsh -lc 'git show --name-status --oneline ef9ffa1' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "sed -n '1,220p' docs/STATE.md" in /home/michael/dev/Blit
 succeeded in 0ms:
ef9ffa1 otp-3: TransferSession core, role-parameterized, in-process transport
M	.review/findings/otp-3-transfer-session-core.md
M	crates/blit-core/Cargo.toml
M	crates/blit-core/build.rs
M	crates/blit-core/src/lib.rs
M	crates/blit-core/src/manifest.rs
A	crates/blit-core/src/transfer_session/mod.rs
A	crates/blit-core/src/transfer_session/transport.rs
A	crates/blit-core/tests/transfer_session_roles.rs

 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-05 (**owner directive D-2026-07-05-1: ONE
transfer path, direction-invariant by construction** — plan
`docs/plan/ONE_TRANSFER_PATH.md` drafted, in codex review, awaiting
the owner's Active flip. **All SMALL_FILE_CEILING work is paused**
(sf-2 landed + graded earlier this date; sf-3a+ blocked). Earlier:
sf-1/sf-2 landed, 10 GbE benchmark session complete, w9-3 landed.)
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check rides the next push.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 flip: "flip the plan and go") — otp-1 in progress**
  — owner directive 2026-07-05, verbatim in the plan doc: ONE block of transfer code; direction/initiator/verb can
  NEVER affect wall time by blit's doing, impossible by construction
  because the per-direction drivers and the `Push`/`PullSync` RPCs
  are deleted. One `TransferSession` (roles SOURCE/DESTINATION), one
  `Transfer` RPC, one choreography (streaming source manifest,
  destination diffs, sf-2 shape-corrected dial as the only stream
  policy); gRPC fallback becomes a byte-carrier option; delegated =
  daemon-initiated session; local rides an in-process transport.
  Slices otp-1..13; converge-up constraint (unified path must match
  the better direction per cell ±10%); benchmark verdict cells must
  be symmetric-fs disk-to-disk (owner: "tmp on one side, spinning
  rust on the other is not a valid test"), tmpfs = wire-reference
  rows only. **D-2026-07-05-2: no version compatibility, EVER —
  same-build peers only, mismatched builds refuse at session open
  (strict handshake specified in otp-1); REV4's negotiate-down
  clause is void, annotated.** **otp-1 `[x]`** (`a3e2acb`+`f861579`,
  codex 6/6 accepted+fixed; contract: `docs/TRANSFER_SESSION.md`;
  suite 1483 → 1484/0). Current slice: **otp-3 TransferSession core**
  (otp-2 symmetric baseline is rig-gated; must land before otp-10).
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1 `[x]`
  sf-2 `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`, codex 1/1,
  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
  baseline. Its principle stands: ceiling-driven, never
  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
  retracted — do not re-litigate). Evidence at
  `docs/bench/10gbe-2026-07-05/`; binaries staged at `blit-bin/`.
- **Tool comparison measured (2026-07-05)** — blit fastest on all
  large/pull/local cells at the wire ceiling; rsyncd faster on small/
  mixed push (the paused plan's target cells). CSVs + full detail:
  `docs/bench/10gbe-2026-07-05/`, DEVLOG 2026-07-05 00:51.
- **10 GbE benchmark session DONE (2026-07-04/05)** — REV4 sign-off
  data in; owner declarations pending (see Blocked). Push/pull 1 GiB
  ≈ 9.5 of 9.88 Gbit/s; **ue-1 band holds** (1.8×); no organic
  resize (one stream saturates 10 GbE) — ue-2 interpretation call.
  Digest: DEVLOG 2026-07-05 00:34; evidence
  `docs/bench/10gbe-2026-07-05/`.
- **Earlier 2026-07-04: w9-3 + eleven review-queue rows all `[x]`**
  — DEVLOG 2026-07-04 entries; commit map in REVIEW.md.
- **REV4 code-complete**; measurement gates DATA-COMPLETE — only the
  owner declarations remain. Residue: Queue item 4. Windows: suite
  green on the owner's machine (erratum D-2026-07-04-2 settled).
- **Active context**: REV4 plan Active (D-2026-06-20-5); codex loop
  governs all code + plan changes (D-2026-07-04-1); REVIEW.md is the
  queue/status index.

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1 `[x]`. Current:
   otp-3 (TransferSession core, role-parameterized, in-process
   transport). otp-2 (symmetric baseline) is RIG-GATED — runs when
   the 10 GbE rig is available, must land before otp-10 cutover.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2,
   REV4 → Shipped (zero-copy resolved — D-2026-07-05-3). Optional
   owner-gated measurement follow-ups (Win 11 bare-metal datapoint;
   disk-path variants; >ARC-size push) — note the disk-path items
   are largely absorbed by otp-2/otp-12's symmetric-rig matrices. Env: bench
   binaries staged at `skippy:/mnt/generic-pool/video/blit-bin/`
   (/tmp and /home on skippy are noexec).
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
   open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
   (mirror-executor consolidation) likely lands for free inside
   otp-6's one-delete-rule slice; re-check before picking it up.
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: revisit gate
   declared met (UNAS 8 Pro daemon CPU-bound below 10 GbE from SSD
   cache). Executes AFTER ONE_TRANSFER_PATH cutover as a
   runtime-selected write strategy in the unified receive sink
   (design input: eval doc §If-FAST-evidence; dead module still
   deletes in w8-1). UNAS is the measurement rig; symmetric-endpoint
   methodology applies. **Rig `zoey` (verified 2026-07-05)**: UNAS 8
   Pro, 4×Cortex-A57 aarch64, Debian 11 userland (glibc 2.31), kernel
   5.10, 15 GiB; test dir `root@zoey:/volume/a595ddbf-…/.srv/
   .unifi-drive/michael/.data/blit-temp/`. **Build recipe** (static
   musl — sidesteps the old glibc): rustup target
   `aarch64-unknown-linux-musl` + `aarch64-linux-gnu-gcc` as
   LINKER/CC/AR for that target, `RUSTFLAGS="-C
   target-feature=+crt-static -C link-self-contained=yes"`, `cargo
   build --release --target aarch64-unknown-linux-musl -p blit-daemon
   -p blit-cli`. Binaries verified executing on zoey 2026-07-05.
   **Owner constraints (2026-07-05, standing)**: ALL activity on
   zoey is restricted to that blit-temp folder — test daemon module
   roots there, test data there, nothing written outside it, ever.
   Zero-copy is to be TESTED on this rig when the post-cutover slice
   set reaches it (standing owner authorization for that test, within
   the folder restriction); no daemon runs on zoey before then
   without a fresh go.
6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   tuning residue (w3-1 scoped it out).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**.
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2, D-2026-07-05-1) and
  **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
  code-complete; measurement gates remain (see Active context).
- Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
  `…_REV2.md`, `…_REV3.md`.
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
  is retired as the grading mechanism (its `findings/`/`results/`
  records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked
  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
  (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (all owner declarations; checkpoints are owner-only)

- **Three 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
  holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE),
  REV4 → Shipped. (The zero-copy revisit verdict and the a/b/c
  question are RESOLVED — D-2026-07-05-3, unparked; measured skippy
  data 1.43 cores daemon-receive / 0.45 client at 9.5 Gbit/s stays
  recorded in DEVLOG + DIAGNOSIS.md.)
- **Push go**: local commits `f6e592e`..HEAD await the ref-listing +
  approval flow; windows-latest CI on the w9-3 harness fix rides it.
- `Cargo.lock`: dependency-refresh drift committed at `04c9c6d` (was
  unavoidable — blit-core gained `rand`); revert selectively if
  unwanted, otherwise settled.

## Open questions

- **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
- **(OPEN, new 2026-07-04)** `725aa07` tracked a 236-file stale
  worktree snapshot (`.claude/worktrees/vigilant-mayer/`, incl. a
  full `crates/` copy). Keep or `git rm -r`? Agent rec: remove;
  deletion awaits an owner go.
- **(OPEN, new 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still
  describe `determine_remote_tuning`/`TuningParams` (stale since
  ue-r2-1e, `TuningParams` now deleted) — fold into w10-docs-batch or
  rewrite sooner? Agent rec: w10.
- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: the 10 GbE
  session delivered the measurement evidence; flip awaits the three
  declarations in Blocked (was four — zero-copy resolved,
  D-2026-07-05-3).
- **(OPEN, new 2026-07-05)** CLI foot-gun found during the session:
  `blit copy src_large dst` with an existing local dir, no `./`,
  parses the bare name as an mDNS discovery endpoint and errors
  "remote source must include a module or root"
  (blit-app endpoints.rs). Should local-path existence win over the
  discovery interpretation, or at least improve the error? Candidate
  review-queue row; owner to slot.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed). The
  daemon-spawn e2e load-flakiness is now root-caused and fixed on
  Linux (w9-3: port-TOCTOU wrong-daemon race + cargo-lock contention;
  claimed-port set + OnceLock build + child-death check). Remaining
  check: windows-latest CI on the next push (10d89e0 predates the
  w9-3 fix, so daemon-spawn flakes there would not be news).

## Handoff log (newest first, keep ≤ 3)

- **2026-07-05 (25th)** @ `cb96e91`+records — **plan Active
  (D-2026-07-05-4) + otp-1 landed and graded** (contract
  `docs/TRANSFER_SESSION.md`, `a3e2acb`, codex 6/6 → `f861579`;
  suite 1484/0). Same session: D-2026-07-05-2 (same-build only) and
  D-2026-07-05-3 (zero-copy unparked, zoey rig proven — static
  aarch64-musl binaries executing in the owner's blit-temp folder,
  recipe in queue item 5). In-flight: none. **Exact first action
  next session**: otp-3 (TransferSession core in blit-core —
  role-parameterized state machine over the existing engine, in-
  process transport, both role assignments over the same fixtures)
  through the codex loop, per `docs/TRANSFER_SESSION.md`. otp-2 is
  rig-gated (before otp-10). Owner declarations: three 10 GbE gates
  + push go remain in Blocked.
- (older entries pruned — see DEVLOG 2026-07-05 06:45 and earlier)

exec
/usr/bin/zsh -lc 'git show --stat ef9ffa1' in /home/michael/dev/Blit
 succeeded in 0ms:
commit ef9ffa14f57a3e0e64904d6a43564d8b3c7f9f49
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 13:46:33 2026 -0400

    otp-3: TransferSession core, role-parameterized, in-process transport
    
    One session state machine (run_source/run_destination) over a
    FrameTransport per docs/TRANSFER_SESSION.md: shared hello/open/accept
    phase with the strict same-build handshake (D-2026-07-05-2),
    streaming manifest, destination-owned incremental diff
    (manifest::header_transfer_status extraction), in-stream record
    grammar with fail-fast violations, destination-computed summary.
    Role suite runs every fixture under both initiator layouts and pins
    identical need sets, summaries, and byte-identical trees — the
    owner's invariance property (D-2026-07-05-1) as tests.
    
    Suite 1484 -> 1500. Gate: fmt/clippy/test clean.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

 .review/findings/otp-3-transfer-session-core.md    |  214 +++-
 crates/blit-core/Cargo.toml                        |    3 +
 crates/blit-core/build.rs                          |   47 +
 crates/blit-core/src/lib.rs                        |    1 +
 crates/blit-core/src/manifest.rs                   |   75 +-
 crates/blit-core/src/transfer_session/mod.rs       | 1262 ++++++++++++++++++++
 crates/blit-core/src/transfer_session/transport.rs |  142 +++
 crates/blit-core/tests/transfer_session_roles.rs   |  710 +++++++++++
 8 files changed, 2375 insertions(+), 79 deletions(-)

exec
/usr/bin/zsh -lc 'git diff --shortstat ef9ffa1''^ ef9ffa1' in /home/michael/dev/Blit
 succeeded in 0ms:
 8 files changed, 2375 insertions(+), 79 deletions(-)

codex
The required docs line up on the main acceptance points: same-build refusal, role-tagged frames, destination-owned diff, in-stream grammar, bounded queues, and test count 1484 to 1500. I’m now checking the implementation details for gaps against those specific contract requirements.
exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '1,260p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '261,520p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Unified transfer session — the ONE block of transfer code
     2	//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
     3	//!
     4	//! A transfer has a SOURCE role and a DESTINATION role; which end
     5	//! initiated and which CLI verb was used select roles, never code.
     6	//! Both roles run the drivers below over a [`transport::FrameTransport`];
     7	//! the wire contract they implement — phases, frame table, record
     8	//! grammar, error semantics — is `docs/TRANSFER_SESSION.md` (otp-1).
     9	//!
    10	//! otp-3 scope: the role-parameterized state machine over the existing
    11	//! engine with the in-process transport and the in-stream byte
    12	//! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
    13	//! progress wiring land at otp-4; mirror otp-6; resume otp-7;
    14	//! delegated otp-9 (see the slice list in the plan).
    15	
    16	pub mod transport;
    17	
    18	use std::collections::{HashMap, HashSet};
    19	use std::fmt;
    20	use std::path::{Path, PathBuf};
    21	use std::sync::{Arc, Mutex as StdMutex};
    22	
    23	use eyre::Result;
    24	use tokio::io::{AsyncReadExt, AsyncWriteExt};
    25	use tokio::sync::mpsc;
    26	
    27	use crate::generated::transfer_frame::Frame;
    28	use crate::generated::{
    29	    session_error, ComparisonMode, FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch,
    30	    NeedComplete, NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
    31	    TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
    32	};
    33	use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
    34	use crate::remote::transfer::diff_planner;
    35	use crate::remote::transfer::payload::PreparedPayload;
    36	use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
    37	use crate::remote::transfer::source::TransferSource;
    38	use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
    39	use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
    40	use crate::transfer_plan::PlanOptions;
    41	use transport::{FrameRx, FrameTransport, FrameTx};
    42	
    43	/// Belt-and-braces wire-shape version, bumped on any change to the
    44	/// frame set or grammar. Exchanged (and exact-matched) in
    45	/// `SessionHello` alongside the build id (D-2026-07-05-2).
    46	pub const CONTRACT_VERSION: u32 = 1;
    47	
    48	/// Payload chunk size on the in-stream carrier. Same unit the gRPC
    49	/// control plane uses today; the data plane (otp-4) has its own.
    50	const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;
    51	
    52	/// Manifest entries buffered per destination diff batch. Mirrors the
    53	/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
    54	/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
    55	/// blocking pool instead of inline per entry.
    56	const DEST_DIFF_CHUNK: usize = 128;
    57	
    58	/// Buffer of the in-memory pipe that feeds wire file-record bytes
    59	/// into `FsTransferSink::write_file_stream`. Bounds destination-side
    60	/// buffering per file record.
    61	const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
    62	
    63	/// This build's session identity: `<crate version>+<git sha>[.dirty]`
    64	/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
    65	/// "unknown" when git was unavailable at compile time.
    66	pub fn session_build_id() -> &'static str {
    67	    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
    68	}
    69	
    70	/// The identity this end presents in `SessionHello`. Defaults to the
    71	/// real compile-time identity; tests inject mismatches.
    72	#[derive(Debug, Clone)]
    73	pub struct HelloConfig {
    74	    pub build_id: String,
    75	    pub contract_version: u32,
    76	}
    77	
    78	impl Default for HelloConfig {
    79	    fn default() -> Self {
    80	        Self {
    81	            build_id: session_build_id().to_string(),
    82	            contract_version: CONTRACT_VERSION,
    83	        }
    84	    }
    85	}
    86	
    87	/// Which handshake part this end plays. Orthogonal to role: all four
    88	/// initiator/role combinations run the same state machine (contract
    89	/// §Invariants 3).
    90	pub enum SessionEndpoint {
    91	    /// This end opened the transport; it sends `SessionOpen`.
    92	    /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
    93	    Initiator { open: Box<SessionOpen> },
    94	    /// This end answers `SessionOpen` with `SessionAccept`. Daemon
    95	    /// module/path/read-only validation attaches here at otp-4.
    96	    Responder,
    97	}
    98	
    99	impl SessionEndpoint {
   100	    /// Convenience constructor so callers don't spell the `Box`.
   101	    pub fn initiator(open: SessionOpen) -> Self {
   102	        SessionEndpoint::Initiator {
   103	            open: Box::new(open),
   104	        }
   105	    }
   106	}
   107	
   108	pub struct SourceSessionConfig {
   109	    pub hello: HelloConfig,
   110	    pub endpoint: SessionEndpoint,
   111	    /// Engine planner knobs (tar/large/raw thresholds). Local to the
   112	    /// source end — strategy selection is planner-owned and never
   113	    /// crosses the wire (contract §Transport selection).
   114	    pub plan_options: PlanOptions,
   115	}
   116	
   117	pub struct DestinationSessionConfig {
   118	    pub hello: HelloConfig,
   119	    pub endpoint: SessionEndpoint,
   120	}
   121	
   122	/// A session-terminating fault: either end refusing, aborting, or
   123	/// catching the peer in a protocol violation. Carried as the error
   124	/// payload of the drivers' `eyre::Report`s — downcast to inspect the
   125	/// wire code.
   126	#[derive(Debug, Clone)]
   127	pub struct SessionFault {
   128	    pub code: session_error::Code,
   129	    pub message: String,
   130	    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
   131	    /// which end is stale (contract §Errors).
   132	    pub local_build_id: String,
   133	    pub peer_build_id: String,
   134	    /// True when the peer already knows about this fault — it sent
   135	    /// the `SessionError` frame itself, or this end already emitted
   136	    /// one. Drivers must not send another.
   137	    pub peer_notified: bool,
   138	}
   139	
   140	impl SessionFault {
   141	    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
   142	        Self {
   143	            code,
   144	            message: message.into(),
   145	            local_build_id: String::new(),
   146	            peer_build_id: String::new(),
   147	            peer_notified: false,
   148	        }
   149	    }
   150	
   151	    fn protocol_violation(message: impl Into<String>) -> Self {
   152	        Self::new(session_error::Code::ProtocolViolation, message)
   153	    }
   154	
   155	    fn internal(message: impl Into<String>) -> Self {
   156	        Self::new(session_error::Code::Internal, message)
   157	    }
   158	
   159	    fn from_wire(err: SessionError) -> Self {
   160	        Self {
   161	            code: session_error::Code::try_from(err.code)
   162	                .unwrap_or(session_error::Code::SessionErrorUnspecified),
   163	            message: err.message,
   164	            // The peer reports its view: its "local" is our peer.
   165	            local_build_id: err.peer_build_id,
   166	            peer_build_id: err.local_build_id,
   167	            peer_notified: true,
   168	        }
   169	    }
   170	
   171	    fn to_wire(&self) -> SessionError {
   172	        SessionError {
   173	            code: self.code as i32,
   174	            message: self.message.clone(),
   175	            local_build_id: self.local_build_id.clone(),
   176	            peer_build_id: self.peer_build_id.clone(),
   177	        }
   178	    }
   179	}
   180	
   181	impl fmt::Display for SessionFault {
   182	    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
   183	        write!(f, "session {}: {}", self.code.as_str_name(), self.message)
   184	    }
   185	}
   186	
   187	impl std::error::Error for SessionFault {}
   188	
   189	/// Downcast a driver-internal error back to its fault, wrapping
   190	/// non-fault failures (fs errors, planner errors, transport failures)
   191	/// as INTERNAL — an end that aborts says why before closing.
   192	fn fault_from_report(report: eyre::Report) -> SessionFault {
   193	    match report.downcast::<SessionFault>() {
   194	        Ok(fault) => fault,
   195	        Err(other) => SessionFault::internal(format!("{other:#}")),
   196	    }
   197	}
   198	
   199	fn frame(f: Frame) -> TransferFrame {
   200	    TransferFrame { frame: Some(f) }
   201	}
   202	
   203	fn error_frame(fault: &SessionFault) -> TransferFrame {
   204	    frame(Frame::Error(fault.to_wire()))
   205	}
   206	
   207	/// Short frame identifier for protocol-violation messages.
   208	fn frame_name(f: &Option<Frame>) -> &'static str {
   209	    match f {
   210	        Some(Frame::Hello(_)) => "SessionHello",
   211	        Some(Frame::Open(_)) => "SessionOpen",
   212	        Some(Frame::Accept(_)) => "SessionAccept",
   213	        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
   214	        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
   215	        Some(Frame::NeedBatch(_)) => "NeedBatch",
   216	        Some(Frame::NeedComplete(_)) => "NeedComplete",
   217	        Some(Frame::BlockHashes(_)) => "BlockHashList",
   218	        Some(Frame::FileBegin(_)) => "FileBegin",
   219	        Some(Frame::FileData(_)) => "FileData",
   220	        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
   221	        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
   222	        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
   223	        Some(Frame::Block(_)) => "BlockTransfer",
   224	        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
   225	        Some(Frame::Resize(_)) => "DataPlaneResize",
   226	        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
   227	        Some(Frame::SourceDone(_)) => "SourceDone",
   228	        Some(Frame::Summary(_)) => "TransferSummary",
   229	        Some(Frame::Error(_)) => "SessionError",
   230	        None => "empty frame",
   231	    }
   232	}
   233	
   234	fn complement(role: TransferRole) -> TransferRole {
   235	    match role {
   236	        TransferRole::Source => TransferRole::Destination,
   237	        TransferRole::Destination => TransferRole::Source,
   238	        TransferRole::Unspecified => TransferRole::Unspecified,
   239	    }
   240	}
   241	
   242	/// Per-role capability check of the operation a `SessionOpen`
   243	/// describes. otp-3 refuses what later slices implement rather than
   244	/// silently ignoring it (fail-fast; contract §Errors).
   245	type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
   246	
   247	fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
   248	    if open.resume.as_ref().is_some_and(|r| r.enabled) {
   249	        return Err(SessionFault::internal(
   250	            "resume is not implemented on the unified session yet (otp-7)",
   251	        ));
   252	    }
   253	    if open
   254	        .filter
   255	        .as_ref()
   256	        .is_some_and(|f| *f != FilterSpec::default())
   257	    {
   258	        return Err(SessionFault::internal(
   259	            "filters are not implemented on the unified session yet (otp-6)",
   260	        ));

 succeeded in 0ms:
   261	    }
   262	    Ok(())
   263	}
   264	
   265	fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
   266	    if open.mirror_enabled {
   267	        return Err(SessionFault::internal(
   268	            "mirror is not implemented on the unified session yet (otp-6)",
   269	        ));
   270	    }
   271	    if open.resume.as_ref().is_some_and(|r| r.enabled) {
   272	        return Err(SessionFault::internal(
   273	            "resume is not implemented on the unified session yet (otp-7)",
   274	        ));
   275	    }
   276	    Ok(())
   277	}
   278	
   279	/// Outcome of the HELLO + OPEN phases.
   280	struct Negotiated {
   281	    open: SessionOpen,
   282	    #[allow(dead_code)] // capacity/grant consumed from otp-4 on
   283	    accept: SessionAccept,
   284	}
   285	
   286	/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
   287	/// scoping requirement). Sends the refusal `SessionError` itself when
   288	/// it detects the fault locally; returned faults are `peer_notified`.
   289	async fn establish(
   290	    transport: &mut FrameTransport,
   291	    hello: &HelloConfig,
   292	    endpoint: &SessionEndpoint,
   293	    local_role: TransferRole,
   294	    validate_open: &OpenValidator,
   295	) -> Result<Negotiated> {
   296	    // HELLO both ways, exact match (D-2026-07-05-2). First frame each
   297	    // direction; no ordering between the two directions.
   298	    transport
   299	        .send(frame(Frame::Hello(SessionHello {
   300	            build_id: hello.build_id.clone(),
   301	            contract_version: hello.contract_version,
   302	        })))
   303	        .await?;
   304	
   305	    let peer_hello = match expect_frame(transport).await? {
   306	        Frame::Hello(h) => h,
   307	        other => {
   308	            return Err(notify_and_wrap(
   309	                transport,
   310	                SessionFault::protocol_violation(format!(
   311	                    "expected SessionHello, got {}",
   312	                    frame_name(&Some(other))
   313	                )),
   314	            )
   315	            .await)
   316	        }
   317	    };
   318	
   319	    if peer_hello.build_id != hello.build_id
   320	        || peer_hello.contract_version != hello.contract_version
   321	    {
   322	        let fault = SessionFault {
   323	            code: session_error::Code::BuildMismatch,
   324	            message: format!(
   325	                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
   326	                hello.build_id, hello.contract_version,
   327	                peer_hello.build_id, peer_hello.contract_version,
   328	            ),
   329	            local_build_id: hello.build_id.clone(),
   330	            peer_build_id: peer_hello.build_id.clone(),
   331	            peer_notified: false,
   332	        };
   333	        return Err(notify_and_wrap(transport, fault).await);
   334	    }
   335	
   336	    match endpoint {
   337	        SessionEndpoint::Initiator { open } => {
   338	            let open = open.as_ref().clone();
   339	            transport.send(frame(Frame::Open(open.clone()))).await?;
   340	            let accept = match expect_frame(transport).await? {
   341	                Frame::Accept(a) => a,
   342	                other => {
   343	                    return Err(notify_and_wrap(
   344	                        transport,
   345	                        SessionFault::protocol_violation(format!(
   346	                            "expected SessionAccept, got {}",
   347	                            frame_name(&Some(other))
   348	                        )),
   349	                    )
   350	                    .await)
   351	                }
   352	            };
   353	            Ok(Negotiated { open, accept })
   354	        }
   355	        SessionEndpoint::Responder => {
   356	            let open = match expect_frame(transport).await? {
   357	                Frame::Open(o) => o,
   358	                other => {
   359	                    return Err(notify_and_wrap(
   360	                        transport,
   361	                        SessionFault::protocol_violation(format!(
   362	                            "expected SessionOpen, got {}",
   363	                            frame_name(&Some(other))
   364	                        )),
   365	                    )
   366	                    .await)
   367	                }
   368	            };
   369	            // The initiator declares ITS role; this responder end must
   370	            // hold the complement.
   371	            let declared =
   372	                TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
   373	            if declared != complement(local_role) {
   374	                return Err(notify_and_wrap(
   375	                    transport,
   376	                    SessionFault::protocol_violation(format!(
   377	                        "initiator declared role {} but this responder is {}",
   378	                        declared.as_str_name(),
   379	                        local_role.as_str_name()
   380	                    )),
   381	                )
   382	                .await);
   383	            }
   384	            if let Err(fault) = validate_open(&open) {
   385	                // Refusal is a SessionError instead of SessionAccept,
   386	                // never a silent close (contract §Phase state machine).
   387	                return Err(notify_and_wrap(transport, fault).await);
   388	            }
   389	            let accept = SessionAccept {
   390	                // The byte RECEIVER advertises capacity at session
   391	                // open (D-2026-06-20-1/-2); consumed by the dial when
   392	                // the data plane lands (otp-4).
   393	                receiver_capacity: if local_role == TransferRole::Destination {
   394	                    Some(crate::engine::local_receiver_capacity())
   395	                } else {
   396	                    None
   397	                },
   398	                // No grant = in-stream byte carrier, otp-3's only one.
   399	                data_plane: None,
   400	            };
   401	            transport.send(frame(Frame::Accept(accept.clone()))).await?;
   402	            Ok(Negotiated { open, accept })
   403	        }
   404	    }
   405	}
   406	
   407	/// Receive one frame during establish; peer errors and closes become
   408	/// terminal faults.
   409	async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
   410	    match transport.recv().await? {
   411	        Some(TransferFrame {
   412	            frame: Some(Frame::Error(err)),
   413	        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
   414	        Some(TransferFrame { frame: Some(f) }) => Ok(f),
   415	        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
   416	            SessionFault::protocol_violation("frame with empty oneof"),
   417	        )),
   418	        None => Err(eyre::Report::new(SessionFault::internal(
   419	            "peer closed during session establish",
   420	        ))),
   421	    }
   422	}
   423	
   424	/// Send the fault to the peer (best effort), mark it notified, and
   425	/// wrap it for return.
   426	async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
   427	    let _ = transport.send(error_frame(&fault)).await;
   428	    fault.peer_notified = true;
   429	    eyre::Report::new(fault)
   430	}
   431	
   432	// ---------------------------------------------------------------------------
   433	// SOURCE driver
   434	// ---------------------------------------------------------------------------
   435	
   436	/// Events the source's receive half forwards to its send half. The
   437	/// channel is unbounded but bounded by construction: every `Need`
   438	/// consumes a distinct sent-manifest entry (unknown or repeated paths
   439	/// fault the session), so the queue never exceeds the source's own
   440	/// manifest size — the contract's bounded-buffering rule holds.
   441	enum SourceEvent {
   442	    Need(FileHeader),
   443	    NeedComplete,
   444	    Summary(TransferSummary),
   445	    Fault(SessionFault),
   446	}
   447	
   448	/// Run the SOURCE role of one transfer session over `transport`.
   449	/// Returns the destination-computed `TransferSummary` (contract: the
   450	/// end that wrote the bytes is the end that attests to them).
   451	pub async fn run_source(
   452	    cfg: SourceSessionConfig,
   453	    transport: FrameTransport,
   454	    source: Arc<dyn TransferSource>,
   455	) -> Result<TransferSummary> {
   456	    let mut transport = transport;
   457	    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
   458	        // Own-config coherence: a source initiator declares SOURCE.
   459	        let declared = TransferRole::try_from(open.initiator_role);
   460	        if declared != Ok(TransferRole::Source) {
   461	            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
   462	        }
   463	        if let Err(fault) = source_open_validator(open) {
   464	            eyre::bail!("run_source initiator config unsupported: {fault}");
   465	        }
   466	    }
   467	
   468	    let negotiated = establish(
   469	        &mut transport,
   470	        &cfg.hello,
   471	        &cfg.endpoint,
   472	        TransferRole::Source,
   473	        &source_open_validator,
   474	    )
   475	    .await?;
   476	
   477	    let (mut tx, rx) = transport.split();
   478	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
   479	    let (event_tx, event_rx) = mpsc::unbounded_channel();
   480	    // AbortOnDrop: an early error return below must abort the receive
   481	    // half instead of leaking it (same rationale as design-2 / w4-1).
   482	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
   483	        rx,
   484	        Arc::clone(&sent),
   485	        event_tx,
   486	    )));
   487	
   488	    match source_send_half(&cfg, &negotiated, &mut tx, source, sent, event_rx).await {
   489	        Ok(summary) => Ok(summary),
   490	        Err(report) => {
   491	            let mut fault = fault_from_report(report);
   492	            if !fault.peer_notified {
   493	                let _ = tx.send(error_frame(&fault)).await;
   494	                fault.peer_notified = true;
   495	            }
   496	            Err(eyre::Report::new(fault))
   497	        }
   498	    }
   499	}
   500	
   501	/// Receive half of the source driver: drains the transport for the
   502	/// whole session so destination sends can never deadlock against a
   503	/// blocked source send, and routes the destination lane to the send
   504	/// half. Terminates on summary, error, close, or violation.
   505	async fn source_recv_half(
   506	    mut rx: Box<dyn FrameRx>,
   507	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   508	    events: mpsc::UnboundedSender<SourceEvent>,
   509	) {
   510	    loop {
   511	        let received = match rx.recv().await {
   512	            Ok(Some(f)) => f,
   513	            Ok(None) => {
   514	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
   515	                    "peer closed before TransferSummary",
   516	                )));
   517	                return;
   518	            }
   519	            Err(err) => {
   520	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(

exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '521,780p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   521	                    "transport receive failed: {err:#}"
   522	                ))));
   523	                return;
   524	            }
   525	        };
   526	        match received.frame {
   527	            Some(Frame::NeedBatch(batch)) => {
   528	                for entry in batch.entries {
   529	                    if entry.resume {
   530	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   531	                            format!(
   532	                                "resume-flagged need for '{}' in a session opened without resume",
   533	                                entry.relative_path
   534	                            ),
   535	                        )));
   536	                        return;
   537	                    }
   538	                    let header = sent
   539	                        .lock()
   540	                        .expect("sent-manifest lock poisoned")
   541	                        .remove(&entry.relative_path);
   542	                    match header {
   543	                        Some(h) => {
   544	                            let _ = events.send(SourceEvent::Need(h));
   545	                        }
   546	                        None => {
   547	                            let _ = events.send(SourceEvent::Fault(
   548	                                SessionFault::protocol_violation(format!(
   549	                                    "need for unknown or already-needed path '{}'",
   550	                                    entry.relative_path
   551	                                )),
   552	                            ));
   553	                            return;
   554	                        }
   555	                    }
   556	                }
   557	            }
   558	            Some(Frame::NeedComplete(_)) => {
   559	                let _ = events.send(SourceEvent::NeedComplete);
   560	            }
   561	            Some(Frame::Summary(summary)) => {
   562	                let _ = events.send(SourceEvent::Summary(summary));
   563	                return;
   564	            }
   565	            Some(Frame::Error(err)) => {
   566	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
   567	                return;
   568	            }
   569	            other => {
   570	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   571	                    format!("{} on the source's receive lane", frame_name(&other)),
   572	                )));
   573	                return;
   574	            }
   575	        }
   576	    }
   577	}
   578	
   579	async fn source_send_half(
   580	    cfg: &SourceSessionConfig,
   581	    negotiated: &Negotiated,
   582	    tx: &mut Box<dyn FrameTx>,
   583	    source: Arc<dyn TransferSource>,
   584	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   585	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
   586	) -> Result<TransferSummary> {
   587	    let mut pending: Vec<FileHeader> = Vec::new();
   588	    let mut need_complete = false;
   589	
   590	    // Streaming manifest: entries go out as enumeration produces them
   591	    // (immediate start in every direction — plan §Design 2). The open
   592	    // carries no source path: the source end owns its local endpoint.
   593	    let _ = &negotiated.open;
   594	    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
   595	    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
   596	    while let Some(header) = header_rx.recv().await {
   597	        sent.lock()
   598	            .expect("sent-manifest lock poisoned")
   599	            .insert(header.relative_path.clone(), header.clone());
   600	        tx.send(frame(Frame::ManifestEntry(header))).await?;
   601	        // Faults detected by the receive half abort the stream now,
   602	        // not after the full scan; needs just accumulate.
   603	        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
   604	    }
   605	    let scanned = scan_handle
   606	        .await
   607	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
   608	    let scan_complete = unreadable
   609	        .lock()
   610	        .expect("unreadable list lock poisoned")
   611	        .is_empty();
   612	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
   613	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
   614	        scan_complete,
   615	    })))
   616	    .await?;
   617	
   618	    // Payload phase. In-stream record grammar: payload records only
   619	    // after ManifestComplete, strictly serialized per record
   620	    // (contract §Transport selection). Needs accumulated while a
   621	    // record batch was being sent become the next planner batch.
   622	    let mut read_buf = vec![0u8; IN_STREAM_CHUNK];
   623	    loop {
   624	        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
   625	        if !pending.is_empty() {
   626	            let batch = std::mem::take(&mut pending);
   627	            send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf).await?;
   628	            continue;
   629	        }
   630	        if need_complete {
   631	            break;
   632	        }
   633	        match events.recv().await {
   634	            Some(event) => {
   635	                handle_source_event(event, &mut pending, &mut need_complete)?;
   636	            }
   637	            None => {
   638	                return Err(eyre::Report::new(SessionFault::internal(
   639	                    "source receive half ended before NeedComplete",
   640	                )))
   641	            }
   642	        }
   643	    }
   644	
   645	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
   646	
   647	    // CLOSING: the destination is the scorer; the next event must be
   648	    // its summary (the receive half ends after forwarding it).
   649	    match events.recv().await {
   650	        Some(SourceEvent::Summary(summary)) => Ok(summary),
   651	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
   652	        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
   653	            format!("need for '{}' after NeedComplete", h.relative_path),
   654	        ))),
   655	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
   656	            SessionFault::protocol_violation("duplicate NeedComplete"),
   657	        )),
   658	        None => Err(eyre::Report::new(SessionFault::internal(
   659	            "source receive half ended before TransferSummary",
   660	        ))),
   661	    }
   662	}
   663	
   664	fn drain_source_events(
   665	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
   666	    pending: &mut Vec<FileHeader>,
   667	    need_complete: &mut bool,
   668	) -> Result<()> {
   669	    while let Ok(event) = events.try_recv() {
   670	        handle_source_event(event, pending, need_complete)?;
   671	    }
   672	    Ok(())
   673	}
   674	
   675	fn handle_source_event(
   676	    event: SourceEvent,
   677	    pending: &mut Vec<FileHeader>,
   678	    need_complete: &mut bool,
   679	) -> Result<()> {
   680	    match event {
   681	        SourceEvent::Need(header) => {
   682	            if *need_complete {
   683	                return Err(eyre::Report::new(SessionFault::protocol_violation(
   684	                    format!("need for '{}' after NeedComplete", header.relative_path),
   685	                )));
   686	            }
   687	            pending.push(header);
   688	            Ok(())
   689	        }
   690	        SourceEvent::NeedComplete => {
   691	            if *need_complete {
   692	                return Err(eyre::Report::new(SessionFault::protocol_violation(
   693	                    "duplicate NeedComplete",
   694	                )));
   695	            }
   696	            *need_complete = true;
   697	            Ok(())
   698	        }
   699	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
   700	            "TransferSummary before SourceDone",
   701	        ))),
   702	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
   703	    }
   704	}
   705	
   706	/// Plan one batch of needed headers with the engine planner and emit
   707	/// the resulting payload records per the in-stream grammar.
   708	async fn send_payload_records(
   709	    tx: &mut Box<dyn FrameTx>,
   710	    source: &Arc<dyn TransferSource>,
   711	    plan_options: PlanOptions,
   712	    batch: Vec<FileHeader>,
   713	    read_buf: &mut [u8],
   714	) -> Result<()> {
   715	    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
   716	    for payload in payloads {
   717	        match source.prepare_payload(payload).await? {
   718	            PreparedPayload::File(header) => {
   719	                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
   720	                if header.size == 0 {
   721	                    continue; // record complete at 0 cumulative bytes
   722	                }
   723	                let mut reader = source.open_file(&header).await?;
   724	                let mut remaining = header.size;
   725	                while remaining > 0 {
   726	                    let want = read_buf.len().min(remaining as usize);
   727	                    let got = reader.read(&mut read_buf[..want]).await?;
   728	                    if got == 0 {
   729	                        // Shorter on disk than the manifest promised —
   730	                        // the record can no longer complete at
   731	                        // header.size; abort rather than pad.
   732	                        eyre::bail!(
   733	                            "'{}' hit EOF with {} bytes still promised",
   734	                            header.relative_path,
   735	                            remaining
   736	                        );
   737	                    }
   738	                    tx.send(frame(Frame::FileData(FileData {
   739	                        content: read_buf[..got].to_vec(),
   740	                    })))
   741	                    .await?;
   742	                    remaining -= got as u64;
   743	                }
   744	            }
   745	            PreparedPayload::TarShard { headers, data } => {
   746	                tx.send(frame(Frame::TarShardHeader(TarShardHeader {
   747	                    files: headers,
   748	                    archive_size: data.len() as u64,
   749	                })))
   750	                .await?;
   751	                for chunk in data.chunks(IN_STREAM_CHUNK) {
   752	                    tx.send(frame(Frame::TarShardChunk(
   753	                        crate::generated::TarShardChunk {
   754	                            content: chunk.to_vec(),
   755	                        },
   756	                    )))
   757	                    .await?;
   758	                }
   759	                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
   760	                    .await?;
   761	            }
   762	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   763	                // The outbound planner never emits these (resume is
   764	                // receive-originated and lands at otp-7).
   765	                eyre::bail!("resume payload planned in a non-resume session");
   766	            }
   767	        }
   768	    }
   769	    Ok(())
   770	}
   771	
   772	// ---------------------------------------------------------------------------
   773	// DESTINATION driver
   774	// ---------------------------------------------------------------------------
   775	
   776	/// What the destination end can report after a completed session.
   777	#[derive(Debug, Clone)]
   778	pub struct DestinationOutcome {
   779	    /// The summary this end computed and sent (contract: DESTINATION
   780	    /// is the scorer).

exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '1041,1300p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1041	    .await
  1042	    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
  1043	
  1044	    let entries: Vec<NeedEntry> = needed
  1045	        .into_iter()
  1046	        // A path the source manifests twice is diffed twice but
  1047	        // needed at most once.
  1048	        .filter(|path| outstanding.insert(path.clone()))
  1049	        .map(|relative_path| {
  1050	            needed_paths.push(relative_path.clone());
  1051	            NeedEntry {
  1052	                relative_path,
  1053	                resume: false, // resume lands at otp-7
  1054	            }
  1055	        })
  1056	        .collect();
  1057	    if entries.is_empty() {
  1058	        return Ok(());
  1059	    }
  1060	    transport
  1061	        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
  1062	        .await?;
  1063	    Ok(())
  1064	}
  1065	
  1066	/// Does the destination need this manifest entry? Stats its own file
  1067	/// and delegates the verdict to `manifest::header_transfer_status` —
  1068	/// the same mode-aware owner `compare_manifests` uses, fed from a
  1069	/// live stat instead of a materialized target manifest.
  1070	fn destination_needs(
  1071	    header: &FileHeader,
  1072	    dst_root: &Path,
  1073	    canonical_dst_root: Option<&Path>,
  1074	    opts: &CompareOptions,
  1075	) -> Result<bool> {
  1076	    let dst = match canonical_dst_root {
  1077	        Some(canonical) => {
  1078	            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
  1079	        }
  1080	        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
  1081	    }
  1082	    .map_err(|err| {
  1083	        SessionFault::protocol_violation(format!(
  1084	            "manifest path '{}' escapes the destination root: {err:#}",
  1085	            header.relative_path
  1086	        ))
  1087	    })?;
  1088	
  1089	    let target = match std::fs::metadata(&dst) {
  1090	        Ok(meta) if meta.is_file() => {
  1091	            let mtime = match meta.modified() {
  1092	                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
  1093	                    Ok(d) => d.as_secs() as i64,
  1094	                    Err(e) => -(e.duration().as_secs() as i64),
  1095	                },
  1096	                Err(_) => 0,
  1097	            };
  1098	            Some((meta.len(), mtime))
  1099	        }
  1100	        // Absent — or present as a directory/other, which a file
  1101	        // write must replace: both diff as "target does not have it"
  1102	        // (matches the push daemon's file_requires_upload).
  1103	        _ => None,
  1104	    };
  1105	    let status = header_transfer_status(
  1106	        header,
  1107	        // Destination-side checksums are never precomputed; Checksum
  1108	        // mode therefore transfers (the conservative arm of
  1109	        // compare_file), matching what push does today.
  1110	        target.map(|(size, mtime)| (size, mtime, &[] as &[u8])),
  1111	        opts,
  1112	    );
  1113	    Ok(matches!(status, FileStatus::New | FileStatus::Modified))
  1114	}
  1115	
  1116	/// Receive one strictly-serialized file record (`file_begin` already
  1117	/// consumed) and stream its bytes into the sink through a bounded
  1118	/// in-memory pipe — record completion is exactly `header.size`
  1119	/// cumulative bytes (contract §Transport selection).
  1120	async fn receive_file_record(
  1121	    transport: &mut FrameTransport,
  1122	    sink: &FsTransferSink,
  1123	    header: &FileHeader,
  1124	) -> Result<crate::remote::transfer::SinkOutcome> {
  1125	    let (mut pipe_wr, mut pipe_rd) = tokio::io::duplex(FILE_RECORD_PIPE_BYTES);
  1126	    let write = sink.write_file_stream(header, &mut pipe_rd);
  1127	    let feed = async {
  1128	        let mut remaining = header.size;
  1129	        while remaining > 0 {
  1130	            let received = match transport.recv().await? {
  1131	                Some(f) => f,
  1132	                None => {
  1133	                    return Err(eyre::Report::new(SessionFault::internal(format!(
  1134	                        "peer closed inside file record '{}'",
  1135	                        header.relative_path
  1136	                    ))))
  1137	                }
  1138	            };
  1139	            match received.frame {
  1140	                Some(Frame::FileData(data)) => {
  1141	                    let len = data.content.len() as u64;
  1142	                    if len > remaining {
  1143	                        return Err(violation(format!(
  1144	                            "file record '{}' overran its size by {} byte(s)",
  1145	                            header.relative_path,
  1146	                            len - remaining
  1147	                        )));
  1148	                    }
  1149	                    pipe_wr.write_all(&data.content).await?;
  1150	                    remaining -= len;
  1151	                }
  1152	                other => {
  1153	                    // Strict serialization: nothing may interleave
  1154	                    // with an open record on the source lane.
  1155	                    return Err(violation(format!(
  1156	                        "{} inside file record '{}' ({} byte(s) short)",
  1157	                        frame_name(&other),
  1158	                        header.relative_path,
  1159	                        remaining
  1160	                    )));
  1161	                }
  1162	            }
  1163	        }
  1164	        pipe_wr.shutdown().await?;
  1165	        Ok(())
  1166	    };
  1167	    let (outcome, ()) = tokio::try_join!(write, feed)?;
  1168	    Ok(outcome)
  1169	}
  1170	
  1171	/// Receive one tar-shard record (`tar_shard_header` already consumed):
  1172	/// buffer to exactly `archive_size` (bounded by the shared tar cap)
  1173	/// and hand the archive to the sink's tar-safety unpack path.
  1174	async fn receive_tar_record(
  1175	    transport: &mut FrameTransport,
  1176	    sink: &FsTransferSink,
  1177	    shard: TarShardHeader,
  1178	) -> Result<crate::remote::transfer::SinkOutcome> {
  1179	    if shard.archive_size > MAX_TAR_SHARD_BYTES {
  1180	        return Err(violation(format!(
  1181	            "tar shard of {} bytes exceeds the {} byte cap",
  1182	            shard.archive_size, MAX_TAR_SHARD_BYTES
  1183	        )));
  1184	    }
  1185	    let mut data: Vec<u8> = Vec::new();
  1186	    data.try_reserve_exact(shard.archive_size as usize)
  1187	        .map_err(|err| eyre::eyre!("allocating {} byte tar shard: {err}", shard.archive_size))?;
  1188	    loop {
  1189	        let received = match transport.recv().await? {
  1190	            Some(f) => f,
  1191	            None => {
  1192	                return Err(eyre::Report::new(SessionFault::internal(
  1193	                    "peer closed inside tar shard record",
  1194	                )))
  1195	            }
  1196	        };
  1197	        match received.frame {
  1198	            Some(Frame::TarShardChunk(chunk)) => {
  1199	                if data.len() as u64 + chunk.content.len() as u64 > shard.archive_size {
  1200	                    return Err(violation(format!(
  1201	                        "tar shard record overran its declared {} bytes",
  1202	                        shard.archive_size
  1203	                    )));
  1204	                }
  1205	                data.extend_from_slice(&chunk.content);
  1206	            }
  1207	            Some(Frame::TarShardComplete(_)) => {
  1208	                if data.len() as u64 != shard.archive_size {
  1209	                    return Err(violation(format!(
  1210	                        "tar shard record completed at {} of {} declared bytes",
  1211	                        data.len(),
  1212	                        shard.archive_size
  1213	                    )));
  1214	                }
  1215	                return sink
  1216	                    .write_payload(PreparedPayload::TarShard {
  1217	                        headers: shard.files,
  1218	                        data,
  1219	                    })
  1220	                    .await;
  1221	            }
  1222	            other => {
  1223	                return Err(violation(format!(
  1224	                    "{} inside tar shard record",
  1225	                    frame_name(&other)
  1226	                )));
  1227	            }
  1228	        }
  1229	    }
  1230	}
  1231	
  1232	#[cfg(test)]
  1233	mod tests {
  1234	    use super::*;
  1235	
  1236	    #[test]
  1237	    fn build_id_has_version_and_git_components() {
  1238	        let id = session_build_id();
  1239	        let (version, git) = id.split_once('+').expect("build id must be version+git");
  1240	        assert_eq!(version, env!("CARGO_PKG_VERSION"));
  1241	        assert!(!git.is_empty(), "git component must be non-empty");
  1242	    }
  1243	
  1244	    #[test]
  1245	    fn fault_round_trips_the_wire_shape() {
  1246	        let fault = SessionFault {
  1247	            code: session_error::Code::BuildMismatch,
  1248	            message: "boom".into(),
  1249	            local_build_id: "1.0+aaa".into(),
  1250	            peer_build_id: "1.0+bbb".into(),
  1251	            peer_notified: false,
  1252	        };
  1253	        let wire = fault.to_wire();
  1254	        let back = SessionFault::from_wire(wire);
  1255	        assert_eq!(back.code, session_error::Code::BuildMismatch);
  1256	        assert_eq!(back.message, "boom");
  1257	        // from_wire swaps perspective: the sender's local is our peer.
  1258	        assert_eq!(back.peer_build_id, "1.0+aaa");
  1259	        assert_eq!(back.local_build_id, "1.0+bbb");
  1260	        assert!(back.peer_notified);
  1261	    }
  1262	}

exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '781,1040p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   781	    pub summary: TransferSummary,
   782	    /// Paths this end put on the need list, in emission order. The
   783	    /// role suite pins these identical across role assignments — the
   784	    /// executable form of the owner's invariance requirement.
   785	    pub needed_paths: Vec<String>,
   786	}
   787	
   788	/// Run the DESTINATION role of one transfer session over `transport`,
   789	/// writing under `dst_root`. Diffs the streamed manifest against its
   790	/// own filesystem (the destination is the one diff owner — plan
   791	/// §Design 3), returns the summary it computed and sent.
   792	pub async fn run_destination(
   793	    cfg: DestinationSessionConfig,
   794	    transport: FrameTransport,
   795	    dst_root: PathBuf,
   796	) -> Result<DestinationOutcome> {
   797	    let mut transport = transport;
   798	    let endpoint = match cfg.endpoint {
   799	        SessionEndpoint::Initiator { mut open } => {
   800	            let declared = TransferRole::try_from(open.initiator_role);
   801	            if declared != Ok(TransferRole::Destination) {
   802	                eyre::bail!(
   803	                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
   804	                );
   805	            }
   806	            if let Err(fault) = destination_open_validator(&open) {
   807	                eyre::bail!("run_destination initiator config unsupported: {fault}");
   808	            }
   809	            // Dial contract: the byte receiver advertises capacity in
   810	            // its open when it is the initiator (contract §Invariants 5).
   811	            if open.receiver_capacity.is_none() {
   812	                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
   813	            }
   814	            SessionEndpoint::Initiator { open }
   815	        }
   816	        SessionEndpoint::Responder => SessionEndpoint::Responder,
   817	    };
   818	
   819	    let negotiated = establish(
   820	        &mut transport,
   821	        &cfg.hello,
   822	        &endpoint,
   823	        TransferRole::Destination,
   824	        &destination_open_validator,
   825	    )
   826	    .await?;
   827	
   828	    match destination_session(&mut transport, &negotiated, &dst_root).await {
   829	        Ok(outcome) => Ok(outcome),
   830	        Err(report) => {
   831	            let mut fault = fault_from_report(report);
   832	            if !fault.peer_notified {
   833	                let _ = transport.send(error_frame(&fault)).await;
   834	                fault.peer_notified = true;
   835	            }
   836	            Err(eyre::Report::new(fault))
   837	        }
   838	    }
   839	}
   840	
   841	fn violation(message: String) -> eyre::Report {
   842	    eyre::Report::new(SessionFault::protocol_violation(message))
   843	}
   844	
   845	async fn destination_session(
   846	    transport: &mut FrameTransport,
   847	    negotiated: &Negotiated,
   848	    dst_root: &Path,
   849	) -> Result<DestinationOutcome> {
   850	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
   851	        .unwrap_or(ComparisonMode::Unspecified);
   852	    let compare_opts = CompareOptions {
   853	        mode: compare_mode.into(),
   854	        ignore_existing: negotiated.open.ignore_existing,
   855	        include_deletions: false, // mirror lands at otp-6
   856	    };
   857	    // src_root is only consumed by local File payloads, which never
   858	    // occur on a session destination (payload bytes arrive as records
   859	    // and go through the stream/tar write paths).
   860	    let sink = FsTransferSink::new(
   861	        PathBuf::new(),
   862	        dst_root.to_path_buf(),
   863	        FsSinkConfig {
   864	            preserve_times: true,
   865	            dry_run: false,
   866	            checksum: None,
   867	            resume: false,
   868	            compare_mode,
   869	        },
   870	    );
   871	    // Same canonical-containment chokepoint the sink write paths use
   872	    // (R46-F3), applied to diff stats so a hostile manifest path can't
   873	    // make the destination stat outside its root.
   874	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
   875	
   876	    let mut pending: Vec<FileHeader> = Vec::new();
   877	    let mut outstanding: HashSet<String> = HashSet::new();
   878	    let mut needed_paths: Vec<String> = Vec::new();
   879	    let mut manifest_complete = false;
   880	    let mut files_written: u64 = 0;
   881	    let mut bytes_written: u64 = 0;
   882	
   883	    loop {
   884	        let received = match transport.recv().await? {
   885	            Some(f) => f,
   886	            None => {
   887	                return Err(eyre::Report::new(SessionFault::internal(
   888	                    "peer closed mid-session",
   889	                )))
   890	            }
   891	        };
   892	        match received.frame {
   893	            Some(Frame::ManifestEntry(header)) => {
   894	                if manifest_complete {
   895	                    return Err(violation(format!(
   896	                        "manifest entry '{}' after ManifestComplete",
   897	                        header.relative_path
   898	                    )));
   899	                }
   900	                pending.push(header);
   901	                if pending.len() >= DEST_DIFF_CHUNK {
   902	                    let chunk = std::mem::take(&mut pending);
   903	                    diff_chunk_and_send_needs(
   904	                        transport,
   905	                        chunk,
   906	                        dst_root,
   907	                        canonical_dst_root.as_deref(),
   908	                        &compare_opts,
   909	                        &mut outstanding,
   910	                        &mut needed_paths,
   911	                    )
   912	                    .await?;
   913	                }
   914	            }
   915	            Some(Frame::ManifestComplete(_complete)) => {
   916	                if manifest_complete {
   917	                    return Err(violation("duplicate ManifestComplete".into()));
   918	                }
   919	                // (scan_complete gates mirror purges from otp-6 on;
   920	                // nothing consumes it in otp-3.)
   921	                let chunk = std::mem::take(&mut pending);
   922	                diff_chunk_and_send_needs(
   923	                    transport,
   924	                    chunk,
   925	                    dst_root,
   926	                    canonical_dst_root.as_deref(),
   927	                    &compare_opts,
   928	                    &mut outstanding,
   929	                    &mut needed_paths,
   930	                )
   931	                .await?;
   932	                // NeedComplete only after ManifestComplete received
   933	                // AND every entry diffed — both true here.
   934	                transport
   935	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
   936	                    .await?;
   937	                manifest_complete = true;
   938	            }
   939	            Some(Frame::FileBegin(header)) => {
   940	                if !manifest_complete {
   941	                    return Err(violation(format!(
   942	                        "payload record for '{}' before ManifestComplete",
   943	                        header.relative_path
   944	                    )));
   945	                }
   946	                if !outstanding.remove(&header.relative_path) {
   947	                    return Err(violation(format!(
   948	                        "payload for '{}' which is not on the need list",
   949	                        header.relative_path
   950	                    )));
   951	                }
   952	                let outcome = receive_file_record(transport, &sink, &header).await?;
   953	                files_written += outcome.files_written as u64;
   954	                bytes_written += outcome.bytes_written;
   955	            }
   956	            Some(Frame::TarShardHeader(shard)) => {
   957	                if !manifest_complete {
   958	                    return Err(violation("tar shard record before ManifestComplete".into()));
   959	                }
   960	                for h in &shard.files {
   961	                    if !outstanding.remove(&h.relative_path) {
   962	                        return Err(violation(format!(
   963	                            "tar shard entry '{}' which is not on the need list",
   964	                            h.relative_path
   965	                        )));
   966	                    }
   967	                }
   968	                let outcome = receive_tar_record(transport, &sink, shard).await?;
   969	                files_written += outcome.files_written as u64;
   970	                bytes_written += outcome.bytes_written;
   971	            }
   972	            Some(Frame::SourceDone(_)) => {
   973	                if !manifest_complete {
   974	                    return Err(violation("SourceDone before ManifestComplete".into()));
   975	                }
   976	                if !outstanding.is_empty() {
   977	                    return Err(violation(format!(
   978	                        "SourceDone with {} needed file(s) never sent",
   979	                        outstanding.len()
   980	                    )));
   981	                }
   982	                let summary = TransferSummary {
   983	                    files_transferred: files_written,
   984	                    bytes_transferred: bytes_written,
   985	                    entries_deleted: 0, // mirror lands at otp-6
   986	                    in_stream_carrier_used: true,
   987	                    files_resumed: 0, // resume lands at otp-7
   988	                };
   989	                transport.send(frame(Frame::Summary(summary))).await?;
   990	                return Ok(DestinationOutcome {
   991	                    summary,
   992	                    needed_paths,
   993	                });
   994	            }
   995	            Some(Frame::Error(err)) => {
   996	                return Err(eyre::Report::new(SessionFault::from_wire(err)));
   997	            }
   998	            other => {
   999	                // Everything else is off-lane or off-phase here:
  1000	                // destination-lane frames echoed back, resume frames
  1001	                // in a non-resume session (otp-7), resize with no
  1002	                // data plane to resize (otp-4), stray handshake
  1003	                // frames, bare FileData/TarShardChunk outside a
  1004	                // record. Fail fast, no tolerant parsing.
  1005	                return Err(violation(format!(
  1006	                    "{} not valid on the destination's receive lane in this phase",
  1007	                    frame_name(&other)
  1008	                )));
  1009	            }
  1010	        }
  1011	    }
  1012	}
  1013	
  1014	/// Stat-and-compare one chunk of manifest entries on the blocking
  1015	/// pool (2+ syscalls per entry — same rationale as the daemon's
  1016	/// w4-4 chunked checks), then stream the resulting need batch.
  1017	async fn diff_chunk_and_send_needs(
  1018	    transport: &mut FrameTransport,
  1019	    chunk: Vec<FileHeader>,
  1020	    dst_root: &Path,
  1021	    canonical_dst_root: Option<&Path>,
  1022	    compare_opts: &CompareOptions,
  1023	    outstanding: &mut HashSet<String>,
  1024	    needed_paths: &mut Vec<String>,
  1025	) -> Result<()> {
  1026	    if chunk.is_empty() {
  1027	        return Ok(());
  1028	    }
  1029	    let dst_root = dst_root.to_path_buf();
  1030	    let canonical = canonical_dst_root.map(Path::to_path_buf);
  1031	    let opts = compare_opts.clone();
  1032	    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
  1033	        let mut needed = Vec::new();
  1034	        for header in &chunk {
  1035	            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
  1036	                needed.push(header.relative_path.clone());
  1037	            }
  1038	        }
  1039	        Ok(needed)
  1040	    })

exec
/usr/bin/zsh -lc 'git show ef9ffa1:crates/blit-core/src/transfer_session/transport.rs | nl -ba' in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Frame transports for the unified transfer session.
     2	//!
     3	//! The session drivers in this module's parent speak
     4	//! [`TransferFrame`]s through the `FrameTx`/`FrameRx` halves and never
     5	//! know what carries them. otp-3 ships the in-process pair below;
     6	//! otp-4 adds a gRPC-backed implementation over the `Transfer` RPC
     7	//! (transport substitution, not new choreography —
     8	//! docs/TRANSFER_SESSION.md); otp-11 reuses the in-process pair for
     9	//! local transfers.
    10	
    11	use async_trait::async_trait;
    12	use eyre::{eyre, Result};
    13	use tokio::sync::mpsc;
    14	
    15	use crate::generated::TransferFrame;
    16	
    17	/// Sending half of a frame transport. `send` applies the transport's
    18	/// own backpressure (bounded channel here, HTTP/2 flow control on the
    19	/// wire) — the session contract deliberately leans on it instead of
    20	/// buffering (docs/TRANSFER_SESSION.md §Phase state machine).
    21	#[async_trait]
    22	pub trait FrameTx: Send {
    23	    async fn send(&mut self, frame: TransferFrame) -> Result<()>;
    24	}
    25	
    26	/// Receiving half of a frame transport. `Ok(None)` means the peer
    27	/// closed the stream cleanly; transport-level failures are `Err`.
    28	#[async_trait]
    29	pub trait FrameRx: Send {
    30	    async fn recv(&mut self) -> Result<Option<TransferFrame>>;
    31	}
    32	
    33	/// One endpoint's bidirectional frame stream, splittable so a driver
    34	/// can run its send and receive halves concurrently (the source
    35	/// driver must keep draining need batches while it streams manifest
    36	/// entries, or a full channel in each direction deadlocks the pair).
    37	pub struct FrameTransport {
    38	    tx: Box<dyn FrameTx>,
    39	    rx: Box<dyn FrameRx>,
    40	}
    41	
    42	impl FrameTransport {
    43	    pub fn new(tx: Box<dyn FrameTx>, rx: Box<dyn FrameRx>) -> Self {
    44	        Self { tx, rx }
    45	    }
    46	
    47	    pub async fn send(&mut self, frame: TransferFrame) -> Result<()> {
    48	        self.tx.send(frame).await
    49	    }
    50	
    51	    pub async fn recv(&mut self) -> Result<Option<TransferFrame>> {
    52	        self.rx.recv().await
    53	    }
    54	
    55	    pub fn split(self) -> (Box<dyn FrameTx>, Box<dyn FrameRx>) {
    56	        (self.tx, self.rx)
    57	    }
    58	}
    59	
    60	/// Bounded per-direction capacity of the in-process pair. Small on
    61	/// purpose: the session must stay live under transport backpressure
    62	/// (both drivers are exercised against it in the role suite), and a
    63	/// deep channel would only hide ordering bugs the wire will expose.
    64	pub const IN_PROCESS_CHANNEL_FRAMES: usize = 64;
    65	
    66	struct MpscFrameTx {
    67	    tx: mpsc::Sender<TransferFrame>,
    68	}
    69	
    70	#[async_trait]
    71	impl FrameTx for MpscFrameTx {
    72	    async fn send(&mut self, frame: TransferFrame) -> Result<()> {
    73	        self.tx
    74	            .send(frame)
    75	            .await
    76	            .map_err(|_| eyre!("in-process transport peer closed"))
    77	    }
    78	}
    79	
    80	struct MpscFrameRx {
    81	    rx: mpsc::Receiver<TransferFrame>,
    82	}
    83	
    84	#[async_trait]
    85	impl FrameRx for MpscFrameRx {
    86	    async fn recv(&mut self) -> Result<Option<TransferFrame>> {
    87	        Ok(self.rx.recv().await)
    88	    }
    89	}
    90	
    91	/// Two connected in-process endpoints: what one sends, the other
    92	/// receives. Both roles of a local transfer (otp-11) — and every
    93	/// otp-3 test — run over this pair.
    94	pub fn in_process_pair() -> (FrameTransport, FrameTransport) {
    95	    let (a_tx, b_rx) = mpsc::channel(IN_PROCESS_CHANNEL_FRAMES);
    96	    let (b_tx, a_rx) = mpsc::channel(IN_PROCESS_CHANNEL_FRAMES);
    97	    (
    98	        FrameTransport::new(
    99	            Box::new(MpscFrameTx { tx: a_tx }),
   100	            Box::new(MpscFrameRx { rx: a_rx }),
   101	        ),
   102	        FrameTransport::new(
   103	            Box::new(MpscFrameTx { tx: b_tx }),
   104	            Box::new(MpscFrameRx { rx: b_rx }),
   105	        ),
   106	    )
   107	}
   108	
   109	#[cfg(test)]
   110	mod tests {
   111	    use super::*;
   112	    use crate::generated::{transfer_frame, SourceDone};
   113	
   114	    fn source_done_frame() -> TransferFrame {
   115	        TransferFrame {
   116	            frame: Some(transfer_frame::Frame::SourceDone(SourceDone {})),
   117	        }
   118	    }
   119	
   120	    #[tokio::test]
   121	    async fn pair_delivers_frames_both_directions() {
   122	        let (mut a, mut b) = in_process_pair();
   123	        a.send(source_done_frame()).await.unwrap();
   124	        b.send(source_done_frame()).await.unwrap();
   125	        assert!(matches!(
   126	            b.recv().await.unwrap().unwrap().frame,
   127	            Some(transfer_frame::Frame::SourceDone(_))
   128	        ));
   129	        assert!(matches!(
   130	            a.recv().await.unwrap().unwrap().frame,
   131	            Some(transfer_frame::Frame::SourceDone(_))
   132	        ));
   133	    }
   134	
   135	    #[tokio::test]
   136	    async fn dropped_peer_reads_as_clean_close_and_send_error() {
   137	        let (mut a, b) = in_process_pair();
   138	        drop(b);
   139	        assert!(a.recv().await.unwrap().is_none(), "closed peer = Ok(None)");
   140	        assert!(a.send(source_done_frame()).await.is_err());
   141	    }
   142	}

codex
One likely risk area is the destination receive loop: it is where the contract’s byte-identical, byte-accounting, and fail-fast rules all meet. I’m checking that path against the source emission path and existing sink/planner behavior.
exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/src/manifest.rs | nl -ba | sed -n '1,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Unified manifest comparison for incremental transfers.
     2	//!
     3	//! This module provides manifest comparison logic used by both push and pull
     4	//! operations to determine which files need to be transferred.
     5	
     6	use crate::generated::{ComparisonMode, FileHeader};
     7	use std::collections::HashMap;
     8	
     9	/// How to compare files between source and target.
    10	#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    11	pub enum CompareMode {
    12	    /// Default: Compare size and mtime, skip if target is newer (safe).
    13	    #[default]
    14	    Default,
    15	    /// Compare only by size, ignore modification time.
    16	    SizeOnly,
    17	    /// Transfer all files unconditionally.
    18	    IgnoreTimes,
    19	    /// Force transfer even if target is newer (dangerous for mirror).
    20	    Force,
    21	    /// Checksum mode: Transfer if checksums differ (slower but more accurate).
    22	    /// For remote transfers, server computes checksums on demand.
    23	    Checksum,
    24	}
    25	
    26	/// Canonical mapping from the wire enum. `Unspecified` folds to the
    27	/// historical default, matching `NormalizedTransferOperation` and the
    28	/// diff planner's defensive handling.
    29	impl From<ComparisonMode> for CompareMode {
    30	    fn from(mode: ComparisonMode) -> Self {
    31	        match mode {
    32	            ComparisonMode::Checksum => CompareMode::Checksum,
    33	            ComparisonMode::SizeOnly => CompareMode::SizeOnly,
    34	            ComparisonMode::IgnoreTimes => CompareMode::IgnoreTimes,
    35	            ComparisonMode::Force => CompareMode::Force,
    36	            ComparisonMode::Unspecified | ComparisonMode::SizeMtime => CompareMode::Default,
    37	        }
    38	    }
    39	}
    40	
    41	/// Status of a file after manifest comparison.
    42	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    43	pub enum FileStatus {
    44	    /// File exists on both sides with matching size and mtime.
    45	    Unchanged,
    46	    /// File exists on both sides but size or mtime differs.
    47	    Modified,
    48	    /// File exists on source but not on target.
    49	    New,
    50	    /// File exists on target and should be skipped (ignore_existing mode).
    51	    SkippedExisting,
    52	}
    53	
    54	/// Result of comparing a single file.
    55	#[derive(Debug, Clone)]
    56	pub struct FileComparison {
    57	    pub relative_path: String,
    58	    pub status: FileStatus,
    59	    /// Size of the source file (for transfer planning).
    60	    pub size: u64,
    61	}
    62	
    63	/// Result of comparing two manifests.
    64	#[derive(Debug, Default)]
    65	pub struct ManifestDiff {
    66	    /// Files that need to be transferred (new or modified).
    67	    pub files_to_transfer: Vec<FileComparison>,
    68	    /// Files that exist on target but not on source (for mirror mode deletion).
    69	    pub files_to_delete: Vec<String>,
    70	    /// Total bytes that need to be transferred.
    71	    pub bytes_to_transfer: u64,
    72	    /// Total files on source.
    73	    pub source_file_count: usize,
    74	    /// Total files on target.
    75	    pub target_file_count: usize,
    76	}
    77	
    78	/// Options for manifest comparison.
    79	#[derive(Debug, Clone, Default)]
    80	pub struct CompareOptions {
    81	    /// How to compare files.
    82	    pub mode: CompareMode,
    83	    /// If true, skip files that already exist on target (regardless of differences).
    84	    pub ignore_existing: bool,
    85	    /// If true, track files to delete for mirror mode.
    86	    pub include_deletions: bool,
    87	}
    88	
    89	/// Compare source manifest against target manifest to determine what needs transferring.
    90	///
    91	/// For push: source = client files, target = server files
    92	/// For pull: source = server files, target = client files
    93	///
    94	/// # Arguments
    95	/// * `source` - Files on the source side (what we have)
    96	/// * `target` - Files on the target side (what exists at destination)
    97	/// * `options` - Comparison options controlling behavior
    98	pub fn compare_manifests(
    99	    source: &[FileHeader],
   100	    target: &[FileHeader],
   101	    options: &CompareOptions,
   102	) -> ManifestDiff {
   103	    let mut diff = ManifestDiff {
   104	        source_file_count: source.len(),
   105	        target_file_count: target.len(),
   106	        ..Default::default()
   107	    };
   108	
   109	    // Build lookup from target manifest: path -> (size, mtime, checksum)
   110	    let target_map: HashMap<&str, (u64, i64, &[u8])> = target
   111	        .iter()
   112	        .map(|h| {
   113	            (
   114	                h.relative_path.as_str(),
   115	                (h.size, h.mtime_seconds, h.checksum.as_slice()),
   116	            )
   117	        })
   118	        .collect();
   119	
   120	    // Compare each source file against target
   121	    for src in source {
   122	        let status = header_transfer_status(
   123	            src,
   124	            target_map
   125	                .get(src.relative_path.as_str())
   126	                .map(|&(size, mtime, checksum)| (size, mtime, checksum)),
   127	            options,
   128	        );
   129	
   130	        if status == FileStatus::New || status == FileStatus::Modified {
   131	            diff.bytes_to_transfer += src.size;
   132	            diff.files_to_transfer.push(FileComparison {
   133	                relative_path: src.relative_path.clone(),
   134	                status,
   135	                size: src.size,
   136	            });
   137	        }
   138	    }
   139	
   140	    // Track deletions for mirror mode
   141	    if options.include_deletions {
   142	        let source_set: std::collections::HashSet<&str> =
   143	            source.iter().map(|h| h.relative_path.as_str()).collect();
   144	
   145	        for target_file in target {
   146	            if !source_set.contains(target_file.relative_path.as_str()) {
   147	                diff.files_to_delete.push(target_file.relative_path.clone());
   148	            }
   149	        }
   150	    }
   151	
   152	    diff
   153	}
   154	
   155	/// Per-entry form of [`compare_manifests`]: status of one source
   156	/// header against the target's view of the same path —
   157	/// `Some((size, mtime_seconds, checksum))` when the target has the
   158	/// path, `None` when it doesn't. This is the single owner of the
   159	/// mode-aware header-vs-target decision; `compare_manifests` and the
   160	/// unified `transfer_session` destination diff (which stats its own
   161	/// filesystem per entry instead of materializing a full target
   162	/// manifest) both call it.
   163	pub fn header_transfer_status(
   164	    src: &FileHeader,
   165	    target: Option<(u64, i64, &[u8])>,
   166	    options: &CompareOptions,
   167	) -> FileStatus {
   168	    match target {
   169	        None => FileStatus::New,
   170	        Some((target_size, target_mtime, target_checksum)) => {
   171	            // File exists on target
   172	            if options.ignore_existing {
   173	                // Skip all existing files regardless of differences
   174	                FileStatus::SkippedExisting
   175	            } else {
   176	                compare_file(
   177	                    src,
   178	                    target_size,
   179	                    target_mtime,
   180	                    target_checksum,
   181	                    options.mode,
   182	                )
   183	            }
   184	        }
   185	    }
   186	}
   187	
   188	/// Compare a single file using the specified comparison mode.
   189	fn compare_file(
   190	    src: &FileHeader,
   191	    target_size: u64,
   192	    target_mtime: i64,
   193	    target_checksum: &[u8],
   194	    mode: CompareMode,
   195	) -> FileStatus {
   196	    match mode {
   197	        CompareMode::IgnoreTimes => {
   198	            // Transfer all files unconditionally
   199	            FileStatus::Modified
   200	        }
   201	        CompareMode::Force => {
   202	            // R58-F9: Force means "transfer regardless of target
   203	            // state" per the proto contract (proto/blit.proto:443)
   204	            // and the diff_planner's always-copy behavior. The
   205	            // size/mtime comparison previously here disagreed with
   206	            // both — if the user said --force, the manifest layer
   207	            // should NOT second-guess them. Always Modified.
   208	            let _ = (target_size, target_mtime, target_checksum);
   209	            FileStatus::Modified
   210	        }
   211	        CompareMode::SizeOnly => {
   212	            // Compare only by size, ignore mtime
   213	            if src.size != target_size {
   214	                FileStatus::Modified
   215	            } else {
   216	                FileStatus::Unchanged
   217	            }
   218	        }
   219	        CompareMode::Default => {
   220	            // Compare size and mtime, skip if target is newer (safe default)
   221	            if src.size != target_size {
   222	                FileStatus::Modified
   223	            } else if src.mtime_seconds > target_mtime {
   224	                // Source is newer - transfer
   225	                FileStatus::Modified
   226	            } else {
   227	                // Target is same age or newer - skip (safe)
   228	                FileStatus::Unchanged
   229	            }
   230	        }
   231	        CompareMode::Checksum => {
   232	            // Checksum mode: Compare using checksums if available
   233	            if src.size != target_size {
   234	                FileStatus::Modified
   235	            } else if !src.checksum.is_empty() && !target_checksum.is_empty() {
   236	                // Both have checksums - compare them
   237	                if src.checksum == target_checksum {
   238	                    FileStatus::Unchanged
   239	                } else {
   240	                    FileStatus::Modified

exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/tests/transfer_session_roles.rs | nl -ba | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Role-parameterized session suite (otp-3).
     2	//!
     3	//! Every fixture runs under BOTH role assignments — the initiator as
     4	//! SOURCE (push-shaped) and the initiator as DESTINATION (pull-shaped)
     5	//! — over the in-process transport, and the outcomes must be
     6	//! IDENTICAL: same need-list set, same summary counts, same bytes on
     7	//! disk. This is the owner's invariance requirement
     8	//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1) in its first
     9	//! executable form: there is no per-direction code to diverge, and
    10	//! this suite pins that the one code path really is
    11	//! initiator-indifferent.
    12	
    13	use std::collections::BTreeMap;
    14	use std::path::Path;
    15	use std::sync::Arc;
    16	use std::time::Duration;
    17	
    18	use blit_core::generated::transfer_frame::Frame;
    19	use blit_core::generated::{
    20	    session_error, ComparisonMode, FileHeader, ManifestComplete, NeedBatch, NeedEntry,
    21	    SessionHello, SessionOpen, TransferFrame, TransferRole, TransferSummary,
    22	};
    23	use blit_core::remote::transfer::source::FsTransferSource;
    24	use blit_core::transfer_plan::PlanOptions;
    25	use blit_core::transfer_session::transport::{in_process_pair, FrameTransport};
    26	use blit_core::transfer_session::{
    27	    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, HelloConfig,
    28	    SessionEndpoint, SessionFault, SourceSessionConfig, CONTRACT_VERSION,
    29	};
    30	
    31	const SUITE_TIMEOUT: Duration = Duration::from_secs(120);
    32	
    33	/// (relative path, content, mtime seconds). Fixture mtimes are fixed
    34	/// epochs so both role-assignment runs see byte-for-byte identical
    35	/// trees.
    36	type FileSpec = (&'static str, Vec<u8>, i64);
    37	
    38	fn write_tree(root: &Path, files: &[FileSpec]) {
    39	    for (rel, content, mtime) in files {
    40	        let path = root.join(rel);
    41	        if let Some(parent) = path.parent() {
    42	            std::fs::create_dir_all(parent).unwrap();
    43	        }
    44	        std::fs::write(&path, content).unwrap();
    45	        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
    46	    }
    47	}
    48	
    49	/// Every regular file under `root` as rel-path → bytes.
    50	fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    51	    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    52	        for entry in std::fs::read_dir(dir).unwrap() {
    53	            let entry = entry.unwrap();
    54	            let path = entry.path();
    55	            if path.is_dir() {
    56	                walk(root, &path, out);
    57	            } else {
    58	                let rel = path
    59	                    .strip_prefix(root)
    60	                    .unwrap()
    61	                    .to_string_lossy()
    62	                    .replace('\\', "/");
    63	                out.insert(rel, std::fs::read(&path).unwrap());
    64	            }
    65	        }
    66	    }
    67	    let mut out = BTreeMap::new();
    68	    if root.exists() {
    69	        walk(root, root, &mut out);
    70	    }
    71	    out
    72	}
    73	
    74	fn assert_trees_identical(src: &Path, dst: &Path) {
    75	    let src_tree = collect_tree(src);
    76	    let dst_tree = collect_tree(dst);
    77	    assert_eq!(
    78	        src_tree.keys().collect::<Vec<_>>(),
    79	        dst_tree.keys().collect::<Vec<_>>(),
    80	        "path sets differ between {src:?} and {dst:?}"
    81	    );
    82	    for (rel, bytes) in &src_tree {
    83	        assert_eq!(
    84	            bytes, &dst_tree[rel],
    85	            "content differs for '{rel}' between {src:?} and {dst:?}"
    86	        );
    87	    }
    88	}
    89	
    90	fn basic_open(initiator_role: TransferRole) -> SessionOpen {
    91	    SessionOpen {
    92	        initiator_role: initiator_role as i32,
    93	        compare_mode: ComparisonMode::SizeMtime as i32,
    94	        in_stream_bytes: true,
    95	        ..Default::default()
    96	    }
    97	}
    98	
    99	/// Drive one full session between `src_root` and `dst_root` with the
   100	/// given end acting as initiator. Data direction is FIXED
   101	/// (src_root → dst_root); the parameter only swaps which end opens
   102	/// the session — the thing the owner's invariant says must not
   103	/// matter.
   104	async fn run_session(
   105	    initiator_role: TransferRole,
   106	    src_root: &Path,
   107	    dst_root: &Path,
   108	    plan_options: PlanOptions,
   109	) -> (
   110	    eyre::Result<TransferSummary>,
   111	    eyre::Result<DestinationOutcome>,
   112	) {
   113	    let open = basic_open(initiator_role);
   114	    let (source_endpoint, dest_endpoint) = match initiator_role {
   115	        TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
   116	        TransferRole::Destination => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
   117	        TransferRole::Unspecified => panic!("fixture must pick a role"),
   118	    };
   119	    let source_cfg = SourceSessionConfig {
   120	        hello: HelloConfig::default(),
   121	        endpoint: source_endpoint,
   122	        plan_options,
   123	    };
   124	    let dest_cfg = DestinationSessionConfig {
   125	        hello: HelloConfig::default(),
   126	        endpoint: dest_endpoint,
   127	    };
   128	    let (a, b) = in_process_pair();
   129	    let source = Arc::new(FsTransferSource::new(src_root.to_path_buf()));
   130	    tokio::time::timeout(SUITE_TIMEOUT, async {
   131	        tokio::join!(
   132	            run_source(source_cfg, a, source),
   133	            run_destination(dest_cfg, b, dst_root.to_path_buf()),
   134	        )
   135	    })
   136	    .await
   137	    .expect("session run timed out")
   138	}
   139	
   140	/// Run the same fixture under both role assignments (fresh trees per
   141	/// run) and pin the invariance property: identical need sets,
   142	/// identical summaries, byte-identical destinations.
   143	async fn assert_invariant_across_roles(
   144	    src_files: &[FileSpec],
   145	    dst_files: &[FileSpec],
   146	    plan_options: PlanOptions,
   147	) -> (TransferSummary, Vec<String>) {
   148	    let mut per_role: Vec<(TransferSummary, Vec<String>)> = Vec::new();
   149	    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
   150	        let tmp = tempfile::tempdir().unwrap();
   151	        let src_root = tmp.path().join("src");
   152	        let dst_root = tmp.path().join("dst");
   153	        std::fs::create_dir_all(&src_root).unwrap();
   154	        std::fs::create_dir_all(&dst_root).unwrap();
   155	        write_tree(&src_root, src_files);
   156	        write_tree(&dst_root, dst_files);
   157	
   158	        let (source_result, dest_result) =
   159	            run_session(initiator_role, &src_root, &dst_root, plan_options).await;
   160	        let source_summary = source_result
   161	            .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
   162	        let dest_outcome = dest_result.unwrap_or_else(|e| {
   163	            panic!("destination failed under initiator {initiator_role:?}: {e:#}")
   164	        });
   165	
   166	        assert_eq!(
   167	            source_summary, dest_outcome.summary,
   168	            "both ends must hold the same summary (initiator {initiator_role:?})"
   169	        );
   170	        assert!(
   171	            source_summary.in_stream_carrier_used,
   172	            "otp-3 sessions ride the in-stream carrier"
   173	        );
   174	        assert_trees_identical(&src_root, &dst_root);
   175	
   176	        let mut needed = dest_outcome.needed_paths.clone();
   177	        needed.sort();
   178	        per_role.push((dest_outcome.summary, needed));
   179	    }
   180	
   181	    let (summary_a, needed_a) = per_role.remove(0);
   182	    let (summary_b, needed_b) = per_role.remove(0);
   183	    assert_eq!(
   184	        needed_a, needed_b,
   185	        "need-list set must be identical whichever end initiates"
   186	    );
   187	    assert_eq!(
   188	        summary_a, summary_b,
   189	        "summary must be identical whichever end initiates"
   190	    );
   191	    (summary_a, needed_a)
   192	}
   193	
   194	fn fault_of(err: &eyre::Report) -> &SessionFault {
   195	    err.downcast_ref::<SessionFault>()
   196	        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
   197	}
   198	
   199	// ---------------------------------------------------------------------------
   200	// Fixtures
   201	// ---------------------------------------------------------------------------
   202	
   203	/// Mixed small tree: nested dirs, an empty file, a name with spaces,
   204	/// and a file larger than the in-stream chunk so file records span
   205	/// multiple FileData frames.
   206	fn small_tree() -> Vec<FileSpec> {
   207	    vec![
   208	        ("a.txt", b"alpha".to_vec(), 1_600_000_001),
   209	        ("empty.bin", Vec::new(), 1_600_000_002),
   210	        ("dir one/b.log", vec![0xAB; 4096], 1_600_000_003),
   211	        (
   212	            "dir one/deeper/c.dat",
   213	            b"gamma-content".to_vec(),
   214	            1_600_000_004,
   215	        ),
   216	        // 3 MiB + 17 so the record needs 4 FileData frames and ends
   217	        // on a partial chunk.
   218	        (
   219	            "big/blob.bin",
   220	            make_patterned(3 * 1024 * 1024 + 17),
   221	            1_600_000_005,
   222	        ),
   223	    ]
   224	}
   225	
   226	fn make_patterned(len: usize) -> Vec<u8> {
   227	    (0..len).map(|i| (i % 251) as u8).collect()
   228	}
   229	
   230	#[tokio::test]
   231	async fn small_tree_byte_identical_under_both_initiators() {
   232	    let src = small_tree();
   233	    let (summary, needed) = assert_invariant_across_roles(&src, &[], PlanOptions::default()).await;
   234	    assert_eq!(summary.files_transferred, src.len() as u64);
   235	    assert_eq!(
   236	        summary.bytes_transferred,
   237	        src.iter().map(|(_, c, _)| c.len() as u64).sum::<u64>()
   238	    );
   239	    assert_eq!(summary.entries_deleted, 0);
   240	    assert_eq!(summary.files_resumed, 0);
   241	    assert_eq!(
   242	        needed.len(),
   243	        src.len(),
   244	        "empty destination needs everything"
   245	    );
   246	}
   247	
   248	#[tokio::test]
   249	async fn tiny_file_tree_tar_shard_records_under_both_initiators() {
   250	    // 200 tiny files under nested dirs; force_tar makes the planner's
   251	    // tar-shard choice deterministic so the tar record grammar
   252	    // (header + chunks + complete → tar-safety unpack) is exercised
   253	    // under both role assignments.
   254	    let mut src: Vec<FileSpec> = Vec::new();
   255	    let names: Vec<String> = (0..200)
   256	        .map(|i| format!("shards/d{}/f{:03}.txt", i % 7, i))
   257	        .collect();
   258	    let leaked: Vec<&'static str> = names
   259	        .into_iter()
   260	        .map(|n| Box::leak(n.into_boxed_str()) as &'static str)

exec
/usr/bin/zsh -lc 'git diff ef9ffa1''^ ef9ffa1 -- crates/blit-core/src/manifest.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/manifest.rs b/crates/blit-core/src/manifest.rs
index a71690d..cae239c 100644
--- a/crates/blit-core/src/manifest.rs
+++ b/crates/blit-core/src/manifest.rs
@@ -3,7 +3,7 @@
 //! This module provides manifest comparison logic used by both push and pull
 //! operations to determine which files need to be transferred.
 
-use crate::generated::FileHeader;
+use crate::generated::{ComparisonMode, FileHeader};
 use std::collections::HashMap;
 
 /// How to compare files between source and target.
@@ -23,6 +23,21 @@ pub enum CompareMode {
     Checksum,
 }
 
+/// Canonical mapping from the wire enum. `Unspecified` folds to the
+/// historical default, matching `NormalizedTransferOperation` and the
+/// diff planner's defensive handling.
+impl From<ComparisonMode> for CompareMode {
+    fn from(mode: ComparisonMode) -> Self {
+        match mode {
+            ComparisonMode::Checksum => CompareMode::Checksum,
+            ComparisonMode::SizeOnly => CompareMode::SizeOnly,
+            ComparisonMode::IgnoreTimes => CompareMode::IgnoreTimes,
+            ComparisonMode::Force => CompareMode::Force,
+            ComparisonMode::Unspecified | ComparisonMode::SizeMtime => CompareMode::Default,
+        }
+    }
+}
+
 /// Status of a file after manifest comparison.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum FileStatus {
@@ -104,24 +119,13 @@ pub fn compare_manifests(
 
     // Compare each source file against target
     for src in source {
-        let status = match target_map.get(src.relative_path.as_str()) {
-            None => FileStatus::New,
-            Some(&(target_size, target_mtime, target_checksum)) => {
-                // File exists on target
-                if options.ignore_existing {
-                    // Skip all existing files regardless of differences
-                    FileStatus::SkippedExisting
-                } else {
-                    compare_file(
-                        src,
-                        target_size,
-                        target_mtime,
-                        target_checksum,
-                        options.mode,
-                    )
-                }
-            }
-        };
+        let status = header_transfer_status(
+            src,
+            target_map
+                .get(src.relative_path.as_str())
+                .map(|&(size, mtime, checksum)| (size, mtime, checksum)),
+            options,
+        );
 
         if status == FileStatus::New || status == FileStatus::Modified {
             diff.bytes_to_transfer += src.size;
@@ -148,6 +152,39 @@ pub fn compare_manifests(
     diff
 }
 
+/// Per-entry form of [`compare_manifests`]: status of one source
+/// header against the target's view of the same path —
+/// `Some((size, mtime_seconds, checksum))` when the target has the
+/// path, `None` when it doesn't. This is the single owner of the
+/// mode-aware header-vs-target decision; `compare_manifests` and the
+/// unified `transfer_session` destination diff (which stats its own
+/// filesystem per entry instead of materializing a full target
+/// manifest) both call it.
+pub fn header_transfer_status(
+    src: &FileHeader,
+    target: Option<(u64, i64, &[u8])>,
+    options: &CompareOptions,
+) -> FileStatus {
+    match target {
+        None => FileStatus::New,
+        Some((target_size, target_mtime, target_checksum)) => {
+            // File exists on target
+            if options.ignore_existing {
+                // Skip all existing files regardless of differences
+                FileStatus::SkippedExisting
+            } else {
+                compare_file(
+                    src,
+                    target_size,
+                    target_mtime,
+                    target_checksum,
+                    options.mode,
+                )
+            }
+        }
+    }
+}
+
 /// Compare a single file using the specified comparison mode.
 fn compare_file(
     src: &FileHeader,

exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/tests/transfer_session_roles.rs | nl -ba | sed -n '261,520p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   261	        .collect();
   262	    for (i, name) in leaked.iter().enumerate() {
   263	        src.push((
   264	            name,
   265	            format!("tiny-{i}").into_bytes(),
   266	            1_600_100_000 + i as i64,
   267	        ));
   268	    }
   269	    let plan = PlanOptions {
   270	        force_tar: true,
   271	        ..PlanOptions::default()
   272	    };
   273	    let (summary, needed) = assert_invariant_across_roles(&src, &[], plan).await;
   274	    assert_eq!(summary.files_transferred, 200);
   275	    assert_eq!(needed.len(), 200);
   276	}
   277	
   278	#[tokio::test]
   279	async fn incremental_transfer_needs_only_missing_and_changed() {
   280	    let src: Vec<FileSpec> = vec![
   281	        // Identical on both sides (same size, same mtime) → skipped.
   282	        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
   283	        // Same size, source newer → transferred.
   284	        ("newer.txt", b"NEW-eight".to_vec(), 1_600_000_200),
   285	        // Absent on destination → transferred.
   286	        ("sub/missing.txt", b"fresh".to_vec(), 1_600_000_300),
   287	    ];
   288	    let dst: Vec<FileSpec> = vec![
   289	        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
   290	        ("newer.txt", b"old-eight".to_vec(), 1_600_000_100),
   291	    ];
   292	    let (summary, needed) = assert_invariant_across_roles(&src, &dst, PlanOptions::default()).await;
   293	    assert_eq!(
   294	        needed,
   295	        vec!["newer.txt".to_string(), "sub/missing.txt".to_string()],
   296	        "need list must be exactly the changed + missing files"
   297	    );
   298	    assert_eq!(summary.files_transferred, 2);
   299	    assert_eq!(summary.bytes_transferred, 9 + 5);
   300	}
   301	
   302	#[tokio::test]
   303	async fn preexisting_identical_tree_yields_empty_need_list() {
   304	    let files: Vec<FileSpec> = vec![
   305	        ("one.txt", b"matching".to_vec(), 1_600_000_400),
   306	        ("nested/two.txt", b"also matching".to_vec(), 1_600_000_500),
   307	    ];
   308	    let (summary, needed) =
   309	        assert_invariant_across_roles(&files, &files, PlanOptions::default()).await;
   310	    assert!(needed.is_empty(), "identical trees must need nothing");
   311	    assert_eq!(summary.files_transferred, 0);
   312	    assert_eq!(summary.bytes_transferred, 0);
   313	}
   314	
   315	#[tokio::test]
   316	async fn preserves_mtime_on_streamed_files() {
   317	    // Not part of the role matrix — pins that the file-record write
   318	    // path applies the manifest mtime (parity with today's receive
   319	    // paths, which the byte-identical asserts alone wouldn't catch).
   320	    let tmp = tempfile::tempdir().unwrap();
   321	    let src_root = tmp.path().join("src");
   322	    let dst_root = tmp.path().join("dst");
   323	    std::fs::create_dir_all(&src_root).unwrap();
   324	    std::fs::create_dir_all(&dst_root).unwrap();
   325	    write_tree(
   326	        &src_root,
   327	        &[("stamped.txt", b"stamp me".to_vec(), 1_555_555_555)],
   328	    );
   329	
   330	    let (source_result, dest_result) = run_session(
   331	        TransferRole::Source,
   332	        &src_root,
   333	        &dst_root,
   334	        PlanOptions::default(),
   335	    )
   336	    .await;
   337	    source_result.unwrap();
   338	    dest_result.unwrap();
   339	
   340	    let meta = std::fs::metadata(dst_root.join("stamped.txt")).unwrap();
   341	    let mtime = filetime::FileTime::from_last_modification_time(&meta);
   342	    assert_eq!(mtime.unix_seconds(), 1_555_555_555);
   343	}
   344	
   345	// ---------------------------------------------------------------------------
   346	// Handshake refusals
   347	// ---------------------------------------------------------------------------
   348	
   349	#[tokio::test]
   350	async fn build_mismatch_refused_under_both_initiators() {
   351	    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
   352	        let tmp = tempfile::tempdir().unwrap();
   353	        let src_root = tmp.path().join("src");
   354	        let dst_root = tmp.path().join("dst");
   355	        std::fs::create_dir_all(&src_root).unwrap();
   356	        std::fs::create_dir_all(&dst_root).unwrap();
   357	
   358	        let open = basic_open(initiator_role);
   359	        let (source_endpoint, dest_endpoint) = match initiator_role {
   360	            TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
   361	            _ => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
   362	        };
   363	        let source_cfg = SourceSessionConfig {
   364	            hello: HelloConfig {
   365	                build_id: "0.1.0+aaaaaaaaaaaa".into(),
   366	                contract_version: CONTRACT_VERSION,
   367	            },
   368	            endpoint: source_endpoint,
   369	            plan_options: PlanOptions::default(),
   370	        };
   371	        let dest_cfg = DestinationSessionConfig {
   372	            hello: HelloConfig {
   373	                build_id: "0.1.0+bbbbbbbbbbbb".into(),
   374	                contract_version: CONTRACT_VERSION,
   375	            },
   376	            endpoint: dest_endpoint,
   377	        };
   378	        let (a, b) = in_process_pair();
   379	        let source = Arc::new(FsTransferSource::new(src_root.clone()));
   380	        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
   381	            tokio::join!(
   382	                run_source(source_cfg, a, source),
   383	                run_destination(dest_cfg, b, dst_root.clone()),
   384	            )
   385	        })
   386	        .await
   387	        .unwrap();
   388	
   389	        for (end, err) in [
   390	            ("source", source_result.unwrap_err()),
   391	            ("destination", dest_result.err().unwrap()),
   392	        ] {
   393	            let fault = fault_of(&err);
   394	            assert_eq!(
   395	                fault.code,
   396	                session_error::Code::BuildMismatch,
   397	                "{end} must refuse with BUILD_MISMATCH (initiator {initiator_role:?})"
   398	            );
   399	            assert!(
   400	                fault.message.contains("aaaaaaaaaaaa") && fault.message.contains("bbbbbbbbbbbb"),
   401	                "{end} must name both build ids, got: {}",
   402	                fault.message
   403	            );
   404	        }
   405	        assert!(
   406	            collect_tree(&dst_root).is_empty(),
   407	            "no bytes may move on a refused handshake"
   408	        );
   409	    }
   410	}
   411	
   412	#[tokio::test]
   413	async fn contract_version_mismatch_is_refused() {
   414	    let tmp = tempfile::tempdir().unwrap();
   415	    let src_root = tmp.path().join("src");
   416	    let dst_root = tmp.path().join("dst");
   417	    std::fs::create_dir_all(&src_root).unwrap();
   418	    std::fs::create_dir_all(&dst_root).unwrap();
   419	
   420	    let source_cfg = SourceSessionConfig {
   421	        hello: HelloConfig::default(),
   422	        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
   423	        plan_options: PlanOptions::default(),
   424	    };
   425	    let dest_cfg = DestinationSessionConfig {
   426	        hello: HelloConfig {
   427	            build_id: HelloConfig::default().build_id,
   428	            contract_version: CONTRACT_VERSION + 1,
   429	        },
   430	        endpoint: SessionEndpoint::Responder,
   431	    };
   432	    let (a, b) = in_process_pair();
   433	    let source = Arc::new(FsTransferSource::new(src_root));
   434	    let (source_result, dest_result) = tokio::join!(
   435	        run_source(source_cfg, a, source),
   436	        run_destination(dest_cfg, b, dst_root),
   437	    );
   438	    assert_eq!(
   439	        fault_of(&source_result.unwrap_err()).code,
   440	        session_error::Code::BuildMismatch
   441	    );
   442	    assert_eq!(
   443	        fault_of(&dest_result.err().unwrap()).code,
   444	        session_error::Code::BuildMismatch
   445	    );
   446	}
   447	
   448	#[tokio::test]
   449	async fn mirror_request_is_refused_until_its_slice_lands() {
   450	    // otp-3 refuses what it does not implement rather than silently
   451	    // ignoring it: a mirror-enabled open must fail the session at the
   452	    // OPEN phase, from the destination (the end that would execute
   453	    // deletions).
   454	    let tmp = tempfile::tempdir().unwrap();
   455	    let src_root = tmp.path().join("src");
   456	    let dst_root = tmp.path().join("dst");
   457	    std::fs::create_dir_all(&src_root).unwrap();
   458	    std::fs::create_dir_all(&dst_root).unwrap();
   459	
   460	    let mut open = basic_open(TransferRole::Source);
   461	    open.mirror_enabled = true;
   462	    let source_cfg = SourceSessionConfig {
   463	        hello: HelloConfig::default(),
   464	        endpoint: SessionEndpoint::initiator(open),
   465	        plan_options: PlanOptions::default(),
   466	    };
   467	    let dest_cfg = DestinationSessionConfig {
   468	        hello: HelloConfig::default(),
   469	        endpoint: SessionEndpoint::Responder,
   470	    };
   471	    let (a, b) = in_process_pair();
   472	    let source = Arc::new(FsTransferSource::new(src_root));
   473	    let (source_result, dest_result) = tokio::join!(
   474	        run_source(source_cfg, a, source),
   475	        run_destination(dest_cfg, b, dst_root),
   476	    );
   477	    let source_fault = fault_of(&source_result.unwrap_err()).clone();
   478	    assert_eq!(source_fault.code, session_error::Code::Internal);
   479	    assert!(
   480	        source_fault.message.contains("otp-6"),
   481	        "refusal must say when mirror lands, got: {}",
   482	        source_fault.message
   483	    );
   484	    assert!(dest_result.is_err());
   485	}
   486	
   487	// ---------------------------------------------------------------------------
   488	// Protocol-violation fail-fast (scripted peer)
   489	// ---------------------------------------------------------------------------
   490	
   491	fn wire(frame: Frame) -> TransferFrame {
   492	    TransferFrame { frame: Some(frame) }
   493	}
   494	
   495	async fn recv_or_panic(t: &mut FrameTransport) -> Frame {
   496	    t.recv()
   497	        .await
   498	        .unwrap()
   499	        .expect("peer closed unexpectedly")
   500	        .frame
   501	        .expect("empty frame")
   502	}
   503	
   504	fn hello_frame() -> TransferFrame {
   505	    let hello = HelloConfig::default();
   506	    wire(Frame::Hello(SessionHello {
   507	        build_id: hello.build_id,
   508	        contract_version: hello.contract_version,
   509	    }))
   510	}
   511	
   512	#[tokio::test]
   513	async fn payload_record_before_manifest_complete_is_protocol_violation() {
   514	    let tmp = tempfile::tempdir().unwrap();
   515	    let dst_root = tmp.path().join("dst");
   516	    std::fs::create_dir_all(&dst_root).unwrap();
   517	
   518	    let dest_cfg = DestinationSessionConfig {
   519	        hello: HelloConfig::default(),
   520	        endpoint: SessionEndpoint::Responder,

exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/tests/transfer_session_roles.rs | nl -ba | sed -n '521,760p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   521	    };
   522	    let (mut peer, dest_transport) = in_process_pair();
   523	    let dest = tokio::spawn(run_destination(dest_cfg, dest_transport, dst_root));
   524	
   525	    // Scripted source peer: valid handshake, then a payload record
   526	    // while its manifest is still open — the contract's example
   527	    // violation ("payload records may begin only AFTER the source's
   528	    // ManifestComplete").
   529	    peer.send(hello_frame()).await.unwrap();
   530	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
   531	    peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
   532	        .await
   533	        .unwrap();
   534	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
   535	
   536	    let header = FileHeader {
   537	        relative_path: "early.bin".into(),
   538	        size: 4,
   539	        mtime_seconds: 1_600_000_000,
   540	        permissions: 0o644,
   541	        checksum: vec![],
   542	    };
   543	    peer.send(wire(Frame::ManifestEntry(header.clone())))
   544	        .await
   545	        .unwrap();
   546	    peer.send(wire(Frame::FileBegin(header))).await.unwrap();
   547	
   548	    // The destination must answer with a SessionError frame naming
   549	    // the violation...
   550	    let refusal = loop {
   551	        match recv_or_panic(&mut peer).await {
   552	            Frame::Error(e) => break e,
   553	            // need batches may legitimately arrive first
   554	            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
   555	            other => panic!("expected SessionError, got {other:?}"),
   556	        }
   557	    };
   558	    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
   559	
   560	    // ...and its driver must fail with the same fault.
   561	    let dest_err = dest.await.unwrap().unwrap_err();
   562	    assert_eq!(
   563	        fault_of(&dest_err).code,
   564	        session_error::Code::ProtocolViolation
   565	    );
   566	    assert!(
   567	        collect_tree(tmp.path()).is_empty(),
   568	        "no bytes may land from a violating record"
   569	    );
   570	}
   571	
   572	#[tokio::test]
   573	async fn need_for_unknown_path_faults_the_source() {
   574	    let tmp = tempfile::tempdir().unwrap();
   575	    let src_root = tmp.path().join("src");
   576	    std::fs::create_dir_all(&src_root).unwrap();
   577	    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);
   578	
   579	    let source_cfg = SourceSessionConfig {
   580	        hello: HelloConfig::default(),
   581	        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
   582	        plan_options: PlanOptions::default(),
   583	    };
   584	    let (source_transport, mut peer) = in_process_pair();
   585	    let source = Arc::new(FsTransferSource::new(src_root));
   586	    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
   587	
   588	    // Scripted destination peer: valid handshake, then a need for a
   589	    // path that was never manifested.
   590	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
   591	    peer.send(hello_frame()).await.unwrap();
   592	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
   593	    peer.send(wire(Frame::Accept(Default::default())))
   594	        .await
   595	        .unwrap();
   596	    loop {
   597	        match recv_or_panic(&mut peer).await {
   598	            Frame::ManifestEntry(_) => continue,
   599	            Frame::ManifestComplete(_) => break,
   600	            other => panic!("expected manifest stream, got {other:?}"),
   601	        }
   602	    }
   603	    peer.send(wire(Frame::NeedBatch(NeedBatch {
   604	        entries: vec![NeedEntry {
   605	            relative_path: "never-manifested.txt".into(),
   606	            resume: false,
   607	        }],
   608	    })))
   609	    .await
   610	    .unwrap();
   611	
   612	    let source_err = source_task.await.unwrap().unwrap_err();
   613	    let fault = fault_of(&source_err);
   614	    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
   615	    assert!(fault.message.contains("never-manifested.txt"));
   616	
   617	    // The source must have told the peer why before aborting.
   618	    let refusal = match recv_or_panic(&mut peer).await {
   619	        Frame::Error(e) => e,
   620	        other => panic!("expected SessionError, got {other:?}"),
   621	    };
   622	    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
   623	}
   624	
   625	#[tokio::test]
   626	async fn resume_flagged_need_is_refused_in_non_resume_session() {
   627	    let tmp = tempfile::tempdir().unwrap();
   628	    let src_root = tmp.path().join("src");
   629	    std::fs::create_dir_all(&src_root).unwrap();
   630	    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);
   631	
   632	    let source_cfg = SourceSessionConfig {
   633	        hello: HelloConfig::default(),
   634	        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
   635	        plan_options: PlanOptions::default(),
   636	    };
   637	    let (source_transport, mut peer) = in_process_pair();
   638	    let source = Arc::new(FsTransferSource::new(src_root));
   639	    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
   640	
   641	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
   642	    peer.send(hello_frame()).await.unwrap();
   643	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
   644	    peer.send(wire(Frame::Accept(Default::default())))
   645	        .await
   646	        .unwrap();
   647	    loop {
   648	        match recv_or_panic(&mut peer).await {
   649	            Frame::ManifestEntry(_) => continue,
   650	            Frame::ManifestComplete(_) => break,
   651	            other => panic!("expected manifest stream, got {other:?}"),
   652	        }
   653	    }
   654	    peer.send(wire(Frame::NeedBatch(NeedBatch {
   655	        entries: vec![NeedEntry {
   656	            relative_path: "real.txt".into(),
   657	            resume: true,
   658	        }],
   659	    })))
   660	    .await
   661	    .unwrap();
   662	
   663	    let source_err = source_task.await.unwrap().unwrap_err();
   664	    assert_eq!(
   665	        fault_of(&source_err).code,
   666	        session_error::Code::ProtocolViolation
   667	    );
   668	}
   669	
   670	#[tokio::test]
   671	async fn manifest_entry_after_manifest_complete_is_protocol_violation() {
   672	    let tmp = tempfile::tempdir().unwrap();
   673	    let dst_root = tmp.path().join("dst");
   674	    std::fs::create_dir_all(&dst_root).unwrap();
   675	
   676	    let dest_cfg = DestinationSessionConfig {
   677	        hello: HelloConfig::default(),
   678	        endpoint: SessionEndpoint::Responder,
   679	    };
   680	    let (mut peer, dest_transport) = in_process_pair();
   681	    let dest = tokio::spawn(run_destination(dest_cfg, dest_transport, dst_root));
   682	
   683	    peer.send(hello_frame()).await.unwrap();
   684	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
   685	    peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
   686	        .await
   687	        .unwrap();
   688	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
   689	
   690	    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
   691	        scan_complete: true,
   692	    })))
   693	    .await
   694	    .unwrap();
   695	    peer.send(wire(Frame::ManifestEntry(FileHeader {
   696	        relative_path: "late.txt".into(),
   697	        size: 1,
   698	        mtime_seconds: 1,
   699	        permissions: 0o644,
   700	        checksum: vec![],
   701	    })))
   702	    .await
   703	    .unwrap();
   704	
   705	    let dest_err = dest.await.unwrap().unwrap_err();
   706	    assert_eq!(
   707	        fault_of(&dest_err).code,
   708	        session_error::Code::ProtocolViolation
   709	    );
   710	}

exec
/usr/bin/zsh -lc 'git show ef9ffa1:crates/blit-core/build.rs | nl -ba' in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use protoc_bin_vendored::protoc_bin_path;
     2	use std::path::PathBuf;
     3	use std::process::Command;
     4	
     5	/// Best-effort git identity for the same-build session handshake
     6	/// (D-2026-07-05-2, docs/TRANSFER_SESSION.md §Invariants 2). Returns
     7	/// "<short sha>[.dirty]" or "unknown" when git/repo is unavailable
     8	/// (e.g. building from a source tarball).
     9	fn git_build_suffix(manifest_dir: &std::path::Path) -> String {
    10	    let run = |args: &[&str]| -> Option<String> {
    11	        let out = Command::new("git")
    12	            .args(args)
    13	            .current_dir(manifest_dir)
    14	            .output()
    15	            .ok()?;
    16	        if !out.status.success() {
    17	            return None;
    18	        }
    19	        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    20	    };
    21	
    22	    let Some(sha) = run(&["rev-parse", "--short=12", "HEAD"]).filter(|s| !s.is_empty()) else {
    23	        return "unknown".to_string();
    24	    };
    25	
    26	    // Track HEAD so the sha refreshes on commit/branch switch without
    27	    // rebuilding on every unrelated file change. Dirty state is
    28	    // best-effort: it is sampled when the build script runs, so a
    29	    // tree that becomes dirty without touching HEAD can keep a stale
    30	    // clean flag until the next rebuild — the sha component is the
    31	    // load-bearing part of the handshake.
    32	    if let Some(git_dir) = run(&["rev-parse", "--absolute-git-dir"]) {
    33	        println!("cargo:rerun-if-changed={git_dir}/HEAD");
    34	        println!("cargo:rerun-if-changed={git_dir}/refs");
    35	    }
    36	
    37	    let dirty = run(&["status", "--porcelain"])
    38	        .map(|s| !s.is_empty())
    39	        .unwrap_or(false);
    40	    if dirty {
    41	        format!("{sha}.dirty")
    42	    } else {
    43	        sha
    44	    }
    45	}
    46	
    47	fn main() -> Result<(), Box<dyn std::error::Error>> {
    48	    let protoc_path = protoc_bin_path()?;
    49	    std::env::set_var("PROTOC", protoc_path);
    50	
    51	    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    52	    let proto_dir = manifest_dir.join("..").join("..").join("proto");
    53	    let proto_file = proto_dir.join("blit.proto");
    54	
    55	    println!("cargo:rerun-if-changed={}", proto_file.display());
    56	    println!(
    57	        "cargo:rustc-env=BLIT_GIT_SHA={}",
    58	        git_build_suffix(&manifest_dir)
    59	    );
    60	
    61	    tonic_prost_build::configure()
    62	        .build_server(true)
    63	        .build_client(true)
    64	        .compile_protos(&[proto_file.as_path()], &[proto_dir.as_path()])?;
    65	    Ok(())
    66	}

exec
/usr/bin/zsh -lc "git show ef9ffa1:crates/blit-core/src/manifest.rs | nl -ba | sed -n '240,360p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   240	                    FileStatus::Modified
   241	                }
   242	            } else {
   243	                // Checksums not available - must transfer for verification
   244	                // (This happens when server checksums are disabled)
   245	                FileStatus::Modified
   246	            }
   247	        }
   248	    }
   249	}
   250	
   251	/// Build a manifest from FileHeader list, returning paths that need transfer.
   252	pub fn files_needing_transfer(diff: &ManifestDiff) -> Vec<String> {
   253	    diff.files_to_transfer
   254	        .iter()
   255	        .map(|f| f.relative_path.clone())
   256	        .collect()
   257	}
   258	
   259	#[cfg(test)]
   260	mod tests {
   261	    use super::*;
   262	
   263	    fn header(path: &str, size: u64, mtime: i64) -> FileHeader {
   264	        FileHeader {
   265	            relative_path: path.to_string(),
   266	            size,
   267	            mtime_seconds: mtime,
   268	            permissions: 0o644,
   269	            checksum: vec![],
   270	        }
   271	    }
   272	
   273	    fn default_opts() -> CompareOptions {
   274	        CompareOptions::default()
   275	    }
   276	
   277	    fn opts_with_deletions() -> CompareOptions {
   278	        CompareOptions {
   279	            include_deletions: true,
   280	            ..Default::default()
   281	        }
   282	    }
   283	
   284	    #[test]
   285	    fn test_empty_manifests() {
   286	        let diff = compare_manifests(&[], &[], &default_opts());
   287	        assert!(diff.files_to_transfer.is_empty());
   288	        assert!(diff.files_to_delete.is_empty());
   289	        assert_eq!(diff.bytes_to_transfer, 0);
   290	    }
   291	
   292	    #[test]
   293	    fn test_all_new_files() {
   294	        let source = vec![header("a.txt", 100, 1000), header("b.txt", 200, 1000)];
   295	        let target = vec![];
   296	
   297	        let diff = compare_manifests(&source, &target, &default_opts());
   298	        assert_eq!(diff.files_to_transfer.len(), 2);
   299	        assert_eq!(diff.bytes_to_transfer, 300);
   300	        assert!(diff
   301	            .files_to_transfer
   302	            .iter()
   303	            .all(|f| f.status == FileStatus::New));
   304	    }
   305	
   306	    #[test]
   307	    fn test_unchanged_files() {
   308	        let source = vec![header("a.txt", 100, 1000)];
   309	        let target = vec![header("a.txt", 100, 1000)];
   310	
   311	        let diff = compare_manifests(&source, &target, &default_opts());
   312	        assert!(diff.files_to_transfer.is_empty());
   313	        assert_eq!(diff.bytes_to_transfer, 0);
   314	    }
   315	
   316	    #[test]
   317	    fn test_modified_by_size() {
   318	        let source = vec![header("a.txt", 200, 1000)];
   319	        let target = vec![header("a.txt", 100, 1000)];
   320	
   321	        let diff = compare_manifests(&source, &target, &default_opts());
   322	        assert_eq!(diff.files_to_transfer.len(), 1);
   323	        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
   324	        assert_eq!(diff.bytes_to_transfer, 200);
   325	    }
   326	
   327	    #[test]
   328	    fn test_modified_by_mtime() {
   329	        let source = vec![header("a.txt", 100, 2000)];
   330	        let target = vec![header("a.txt", 100, 1000)];
   331	
   332	        let diff = compare_manifests(&source, &target, &default_opts());
   333	        assert_eq!(diff.files_to_transfer.len(), 1);
   334	        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
   335	    }
   336	
   337	    #[test]
   338	    fn test_target_newer_unchanged() {
   339	        // If target is newer, we don't overwrite (source is not newer) - safe default
   340	        let source = vec![header("a.txt", 100, 1000)];
   341	        let target = vec![header("a.txt", 100, 2000)];
   342	
   343	        let diff = compare_manifests(&source, &target, &default_opts());
   344	        assert!(diff.files_to_transfer.is_empty());
   345	    }
   346	
   347	    #[test]
   348	    fn test_force_mode_overwrites_newer() {
   349	        // Force mode should transfer even if target is newer
   350	        let source = vec![header("a.txt", 100, 1000)];
   351	        let target = vec![header("a.txt", 100, 2000)];
   352	
   353	        let opts = CompareOptions {
   354	            mode: CompareMode::Force,
   355	            ..Default::default()
   356	        };
   357	        let diff = compare_manifests(&source, &target, &opts);
   358	        assert_eq!(diff.files_to_transfer.len(), 1);
   359	        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
   360	    }

exec
/usr/bin/zsh -lc 'rg -n "message TransferSummary|message SessionOpen|enum ComparisonMode|message NeedBatch|oneof frame|message TarShardHeader|message FileData|message FileHeader|message SessionError" proto/blit.proto' in /home/michael/dev/Blit
 succeeded in 0ms:
350:message FileHeader {
358:message FileData {
362:message TarShardHeader {
729:enum ComparisonMode {
1245:message SessionOpen {
1310:message NeedBatch {
1322:message TransferSummary {
1331:message SessionError {
1357:  oneof frame {

exec
/usr/bin/zsh -lc "rg -n \"fn plan_push_payloads|pub fn plan_push_payloads|prepare_payload|enum PreparedPayload|write_file_stream|write_payload|struct FsTransferSource|impl TransferSource\" crates/blit-core/src/remote/transfer crates/blit-core/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/pipeline.rs:198:                        .prepare_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:213:                        .write_payload(prepared)
crates/blit-core/src/remote/transfer/pipeline.rs:412:/// Both directions converge on `TransferSink::write_payload`: file data
crates/blit-core/src/remote/transfer/pipeline.rs:413:/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
crates/blit-core/src/remote/transfer/pipeline.rs:447:                    .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/pipeline.rs:466:                    .write_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:499:                    .write_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:520:                    .write_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:660:    /// Sink that fails the first `write_payload` with a recognisable
crates/blit-core/src/remote/transfer/pipeline.rs:674:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1107:        async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1120:        async fn write_file_stream(
crates/blit-core/src/remote/transfer/pipeline.rs:1455:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1549:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1569:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1912:        async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/transfer_session/mod.rs:59:/// into `FsTransferSink::write_file_stream`. Bounds destination-side
crates/blit-core/src/transfer_session/mod.rs:717:        match source.prepare_payload(payload).await? {
crates/blit-core/src/transfer_session/mod.rs:1126:    let write = sink.write_file_stream(header, &mut pipe_rd);
crates/blit-core/src/transfer_session/mod.rs:1216:                    .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/payload.rs:43:pub async fn prepare_payload(
crates/blit-core/src/remote/transfer/payload.rs:74:/// payload variant — they go through `TransferSink::write_file_stream`
crates/blit-core/src/remote/transfer/payload.rs:78:pub enum PreparedPayload {
crates/blit-core/src/remote/transfer/payload.rs:218:        async move { source.prepare_payload(payload).await }
crates/blit-core/src/remote/transfer/mod.rs:23:    build_tar_shard, payload_file_count, plan_transfer_payloads, prepare_payload,
crates/blit-core/src/remote/transfer/diff_planner.rs:46:pub fn plan_push_payloads(
crates/blit-core/src/remote/transfer/sink.rs:46:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;
crates/blit-core/src/remote/transfer/sink.rs:55:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:61:            "{} does not support write_file_stream (called for {})",
crates/blit-core/src/remote/transfer/sink.rs:130:    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
crates/blit-core/src/remote/transfer/sink.rs:133:    /// `write_file_stream` passes it into
crates/blit-core/src/remote/transfer/sink.rs:174:    /// `write_file_stream` reports every chunk the data plane
crates/blit-core/src/remote/transfer/sink.rs:218:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:300:        // write_payload, not write_file_stream, so the chunk-
crates/blit-core/src/remote/transfer/sink.rs:309:        // `write_file_stream`'s dry-run branch.
crates/blit-core/src/remote/transfer/sink.rs:319:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:456:    // R47-F1: the FsTransferSink::write_payload arm for
crates/blit-core/src/remote/transfer/sink.rs:462:    // write_file_stream uses.
crates/blit-core/src/remote/transfer/sink.rs:565:    // R47-F1: tar shards arriving on FsTransferSink::write_payload
crates/blit-core/src/remote/transfer/sink.rs:718:    // dance as write_file_stream — see commit 946bd77).
crates/blit-core/src/remote/transfer/sink.rs:771:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:807:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:863:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:884:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:951:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:1129:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:1294:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1327:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1336:    /// R58-F4 regression: dry-run for `write_payload` must NOT
crates/blit-core/src/remote/transfer/sink.rs:1364:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1375:    /// R58-F4 regression for the streaming receive path. `write_file_stream`
crates/blit-core/src/remote/transfer/sink.rs:1380:    async fn fs_sink_dry_run_write_file_stream_does_not_create_dirs() {
crates/blit-core/src/remote/transfer/sink.rs:1401:        let outcome = sink.write_file_stream(&header, &mut reader).await.unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1437:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1493:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1528:        sink.write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1540:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1559:            .write_payload(PreparedPayload::TarShard { headers, data })
crates/blit-core/src/remote/transfer/sink.rs:1588:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1643:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1702:        sink.write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1758:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1815:        sink.write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1899:        // Use write_file_stream so we exercise the sink.rs:218 site that
crates/blit-core/src/remote/transfer/sink.rs:1903:        let result = sink.write_file_stream(&header, &mut empty).await;
crates/blit-core/src/remote/transfer/sink.rs:1977:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2013:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2062:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2076:    /// R47-F1 regression: the `write_payload` arm for
crates/blit-core/src/remote/transfer/sink.rs:2087:    async fn fs_sink_write_payload_file_rejects_escape() {
crates/blit-core/src/remote/transfer/sink.rs:2120:            .write_payload(payload)
crates/blit-core/src/remote/transfer/sink.rs:2134:    /// R47-F1 regression: the `write_payload` arm for
crates/blit-core/src/remote/transfer/sink.rs:2144:    async fn fs_sink_write_payload_tar_shard_rejects_escape() {
crates/blit-core/src/remote/transfer/sink.rs:2193:            .write_payload(payload)
crates/blit-core/src/remote/transfer/sink.rs:2207:    /// c-1b round 2 regression: tar shards land via `write_payload`,
crates/blit-core/src/remote/transfer/sink.rs:2208:    /// not `write_file_stream`, so the chunk-granular byte hook
crates/blit-core/src/remote/transfer/sink.rs:2210:    /// `write_payload` now reports `outcome.bytes_written` against
crates/blit-core/src/remote/transfer/sink.rs:2213:    async fn write_payload_reports_tar_shard_bytes_against_byte_progress() {
crates/blit-core/src/remote/transfer/sink.rs:2264:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:2282:    /// also land via `write_payload`. Their `bytes_written`
crates/blit-core/src/remote/transfer/sink.rs:2286:    async fn write_payload_reports_file_block_bytes_against_byte_progress() {
crates/blit-core/src/remote/transfer/sink.rs:2312:            .write_payload(PreparedPayload::FileBlock {
crates/blit-core/src/remote/transfer/source.rs:29:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload>;
crates/blit-core/src/remote/transfer/source.rs:49:pub struct FsTransferSource {
crates/blit-core/src/remote/transfer/source.rs:60:impl TransferSource for FsTransferSource {
crates/blit-core/src/remote/transfer/source.rs:77:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:78:        use crate::remote::transfer::payload::prepare_payload;
crates/blit-core/src/remote/transfer/source.rs:79:        prepare_payload(payload, self.root.clone()).await
crates/blit-core/src/remote/transfer/source.rs:120:/// Extracted from `RemoteTransferSource::prepare_payload` so the
crates/blit-core/src/remote/transfer/source.rs:232:impl TransferSource for RemoteTransferSource {
crates/blit-core/src/remote/transfer/source.rs:263:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:350:impl TransferSource for FilteredSource {
crates/blit-core/src/remote/transfer/source.rs:381:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:382:        self.inner.prepare_payload(payload).await
crates/blit-core/src/remote/transfer/source.rs:474:    impl TransferSource for StubSource {
crates/blit-core/src/remote/transfer/source.rs:497:        async fn prepare_payload(&self, _: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/pull.rs:1912:        // Receive path uses write_file_stream (wire payloads), not
crates/blit-core/src/remote/transfer/diff_planner.rs:46:pub fn plan_push_payloads(
crates/blit-core/src/remote/transfer/pipeline.rs:198:                        .prepare_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:213:                        .write_payload(prepared)
crates/blit-core/src/remote/transfer/pipeline.rs:412:/// Both directions converge on `TransferSink::write_payload`: file data
crates/blit-core/src/remote/transfer/pipeline.rs:413:/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
crates/blit-core/src/remote/transfer/pipeline.rs:447:                    .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/pipeline.rs:466:                    .write_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:499:                    .write_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:520:                    .write_payload(payload)
crates/blit-core/src/remote/transfer/pipeline.rs:660:    /// Sink that fails the first `write_payload` with a recognisable
crates/blit-core/src/remote/transfer/pipeline.rs:674:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1107:        async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1120:        async fn write_file_stream(
crates/blit-core/src/remote/transfer/pipeline.rs:1455:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1549:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1569:        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/pipeline.rs:1912:        async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/payload.rs:43:pub async fn prepare_payload(
crates/blit-core/src/remote/transfer/payload.rs:74:/// payload variant — they go through `TransferSink::write_file_stream`
crates/blit-core/src/remote/transfer/payload.rs:78:pub enum PreparedPayload {
crates/blit-core/src/remote/transfer/payload.rs:218:        async move { source.prepare_payload(payload).await }
crates/blit-core/src/remote/transfer/sink.rs:46:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;
crates/blit-core/src/remote/transfer/sink.rs:55:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:61:            "{} does not support write_file_stream (called for {})",
crates/blit-core/src/remote/transfer/sink.rs:130:    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
crates/blit-core/src/remote/transfer/sink.rs:133:    /// `write_file_stream` passes it into
crates/blit-core/src/remote/transfer/sink.rs:174:    /// `write_file_stream` reports every chunk the data plane
crates/blit-core/src/remote/transfer/sink.rs:218:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:300:        // write_payload, not write_file_stream, so the chunk-
crates/blit-core/src/remote/transfer/sink.rs:309:        // `write_file_stream`'s dry-run branch.
crates/blit-core/src/remote/transfer/sink.rs:319:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:456:    // R47-F1: the FsTransferSink::write_payload arm for
crates/blit-core/src/remote/transfer/sink.rs:462:    // write_file_stream uses.
crates/blit-core/src/remote/transfer/sink.rs:565:    // R47-F1: tar shards arriving on FsTransferSink::write_payload
crates/blit-core/src/remote/transfer/sink.rs:718:    // dance as write_file_stream — see commit 946bd77).
crates/blit-core/src/remote/transfer/sink.rs:771:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:807:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:863:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:884:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:951:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:1129:    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
crates/blit-core/src/remote/transfer/sink.rs:1294:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1327:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1336:    /// R58-F4 regression: dry-run for `write_payload` must NOT
crates/blit-core/src/remote/transfer/sink.rs:1364:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1375:    /// R58-F4 regression for the streaming receive path. `write_file_stream`
crates/blit-core/src/remote/transfer/sink.rs:1380:    async fn fs_sink_dry_run_write_file_stream_does_not_create_dirs() {
crates/blit-core/src/remote/transfer/sink.rs:1401:        let outcome = sink.write_file_stream(&header, &mut reader).await.unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1437:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1493:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1528:        sink.write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1540:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1559:            .write_payload(PreparedPayload::TarShard { headers, data })
crates/blit-core/src/remote/transfer/sink.rs:1588:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1643:            .write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1702:        sink.write_payload(PreparedPayload::File(header))
crates/blit-core/src/remote/transfer/sink.rs:1758:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1815:        sink.write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1899:        // Use write_file_stream so we exercise the sink.rs:218 site that
crates/blit-core/src/remote/transfer/sink.rs:1903:        let result = sink.write_file_stream(&header, &mut empty).await;
crates/blit-core/src/remote/transfer/sink.rs:1977:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2013:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2062:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2076:    /// R47-F1 regression: the `write_payload` arm for
crates/blit-core/src/remote/transfer/sink.rs:2087:    async fn fs_sink_write_payload_file_rejects_escape() {
crates/blit-core/src/remote/transfer/sink.rs:2120:            .write_payload(payload)
crates/blit-core/src/remote/transfer/sink.rs:2134:    /// R47-F1 regression: the `write_payload` arm for
crates/blit-core/src/remote/transfer/sink.rs:2144:    async fn fs_sink_write_payload_tar_shard_rejects_escape() {
crates/blit-core/src/remote/transfer/sink.rs:2193:            .write_payload(payload)
crates/blit-core/src/remote/transfer/sink.rs:2207:    /// c-1b round 2 regression: tar shards land via `write_payload`,
crates/blit-core/src/remote/transfer/sink.rs:2208:    /// not `write_file_stream`, so the chunk-granular byte hook
crates/blit-core/src/remote/transfer/sink.rs:2210:    /// `write_payload` now reports `outcome.bytes_written` against
crates/blit-core/src/remote/transfer/sink.rs:2213:    async fn write_payload_reports_tar_shard_bytes_against_byte_progress() {
crates/blit-core/src/remote/transfer/sink.rs:2264:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:2282:    /// also land via `write_payload`. Their `bytes_written`
crates/blit-core/src/remote/transfer/sink.rs:2286:    async fn write_payload_reports_file_block_bytes_against_byte_progress() {
crates/blit-core/src/remote/transfer/sink.rs:2312:            .write_payload(PreparedPayload::FileBlock {
crates/blit-core/src/remote/transfer/mod.rs:23:    build_tar_shard, payload_file_count, plan_transfer_payloads, prepare_payload,
crates/blit-core/src/remote/transfer/source.rs:29:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload>;
crates/blit-core/src/remote/transfer/source.rs:49:pub struct FsTransferSource {
crates/blit-core/src/remote/transfer/source.rs:60:impl TransferSource for FsTransferSource {
crates/blit-core/src/remote/transfer/source.rs:77:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:78:        use crate::remote::transfer::payload::prepare_payload;
crates/blit-core/src/remote/transfer/source.rs:79:        prepare_payload(payload, self.root.clone()).await
crates/blit-core/src/remote/transfer/source.rs:120:/// Extracted from `RemoteTransferSource::prepare_payload` so the
crates/blit-core/src/remote/transfer/source.rs:232:impl TransferSource for RemoteTransferSource {
crates/blit-core/src/remote/transfer/source.rs:263:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:350:impl TransferSource for FilteredSource {
crates/blit-core/src/remote/transfer/source.rs:381:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:382:        self.inner.prepare_payload(payload).await
crates/blit-core/src/remote/transfer/source.rs:474:    impl TransferSource for StubSource {
crates/blit-core/src/remote/transfer/source.rs:497:        async fn prepare_payload(&self, _: TransferPayload) -> Result<PreparedPayload> {

exec
/usr/bin/zsh -lc "rg -n \"pub const CONTROL_PLANE_CHUNK_SIZE|CONTROL_PLANE_CHUNK_SIZE|MAX_TAR_SHARD_BYTES\" crates/blit-core/src/remote/transfer -g '*.rs'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rg -n \"safe_join_contained|canonical_dest_root|safe_join\\(\" crates/blit-core/src/path_safety.rs crates/blit-core/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/pipeline.rs:563:/// is `tar_safety::MAX_TAR_SHARD_BYTES` so the wire-side reader
crates/blit-core/src/remote/transfer/pipeline.rs:568:    crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES as usize;
crates/blit-core/src/remote/transfer/payload.rs:19:use super::data_plane::CONTROL_PLANE_CHUNK_SIZE;
crates/blit-core/src/remote/transfer/payload.rs:240:        super::grpc_fallback::clamp_fallback_chunk_size(chunk_bytes.max(CONTROL_PLANE_CHUNK_SIZE));
crates/blit-core/src/remote/transfer/mod.rs:18:    generate_sub_token, receive_stream_double_buffered, DataPlaneSession, CONTROL_PLANE_CHUNK_SIZE,
crates/blit-core/src/remote/transfer/grpc_fallback.rs:101:/// Matches [`super::data_plane::CONTROL_PLANE_CHUNK_SIZE`] (1 MiB) by
crates/blit-core/src/remote/transfer/data_plane.rs:15:pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
crates/blit-core/src/remote/transfer/sink.rs:965:        // The `.max(CONTROL_PLANE_CHUNK_SIZE)` keeps the protobuf
crates/blit-core/src/remote/transfer/sink.rs:973:                .max(super::data_plane::CONTROL_PLANE_CHUNK_SIZE),
crates/blit-core/src/remote/transfer/sink.rs:1140:        // shape (`.max(CONTROL_PLANE_CHUNK_SIZE)` floor + clamp ceiling)
crates/blit-core/src/remote/transfer/sink.rs:1144:                .max(super::data_plane::CONTROL_PLANE_CHUNK_SIZE),
crates/blit-core/src/remote/transfer/source.rs:116:/// size must stay within `tar_safety::MAX_TAR_SHARD_BYTES` so a
crates/blit-core/src/remote/transfer/source.rs:124:    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/source.rs:127:        if header.size > MAX_TAR_SHARD_BYTES {
crates/blit-core/src/remote/transfer/source.rs:132:                MAX_TAR_SHARD_BYTES
crates/blit-core/src/remote/transfer/source.rs:140:    if total_bytes > MAX_TAR_SHARD_BYTES {
crates/blit-core/src/remote/transfer/source.rs:144:            MAX_TAR_SHARD_BYTES
crates/blit-core/src/remote/transfer/source.rs:166:    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/source.rs:171:    if expected_size > MAX_TAR_SHARD_BYTES {
crates/blit-core/src/remote/transfer/source.rs:176:            MAX_TAR_SHARD_BYTES
crates/blit-core/src/remote/transfer/source.rs:194:    // at `MAX_TAR_SHARD_BYTES` (above), so the addition can't overflow.
crates/blit-core/src/remote/transfer/source.rs:665:    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/source.rs:686:        let headers = vec![fh("a", MAX_TAR_SHARD_BYTES)];
crates/blit-core/src/remote/transfer/source.rs:692:        let headers = vec![fh("huge", MAX_TAR_SHARD_BYTES + 1)];
crates/blit-core/src/remote/transfer/source.rs:705:        let half = MAX_TAR_SHARD_BYTES / 2 + 1;
crates/blit-core/src/remote/transfer/source.rs:789:        use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/source.rs:791:        let err = read_remote_entry_bounded(reader, MAX_TAR_SHARD_BYTES + 1, "huge.txt")
crates/blit-core/src/remote/transfer/tar_safety.rs:50:pub const MAX_TAR_SHARD_BYTES: u64 = 256 * 1024 * 1024;
crates/blit-core/src/remote/transfer/tar_safety.rs:70:            max_entry_bytes: MAX_TAR_SHARD_BYTES,

 succeeded in 0ms:
crates/blit-core/src/path_safety.rs:27://! `safe_join(root, wire)` performs a *lexical* containment check.
crates/blit-core/src/path_safety.rs:142:pub fn safe_join(root: &Path, wire_path: &str) -> Result<PathBuf> {
crates/blit-core/src/path_safety.rs:181:    let target = safe_join(canonical_module_root, wire_path)?;
crates/blit-core/src/path_safety.rs:283:pub fn canonical_dest_root(dest_root: &Path) -> Result<PathBuf> {
crates/blit-core/src/path_safety.rs:312:/// `canonical_dest_root` for the canonical root capture.
crates/blit-core/src/path_safety.rs:321:pub fn safe_join_contained(
crates/blit-core/src/path_safety.rs:326:    let target = safe_join(dest_root, wire_path)?;
crates/blit-core/src/path_safety.rs:378:            safe_join(root, "").unwrap(),
crates/blit-core/src/path_safety.rs:395:            safe_join(root, "foo/bar.txt").unwrap(),
crates/blit-core/src/path_safety.rs:473:        assert!(safe_join(root, "../escape").is_err());
crates/blit-core/src/path_safety.rs:474:        assert!(safe_join(root, "/etc/passwd").is_err());
crates/blit-core/src/path_safety.rs:475:        assert!(safe_join(root, "C:\\evil").is_err());
crates/blit-core/src/path_safety.rs:482:            safe_join(root, "a/b/c/d/e.txt").unwrap(),
crates/blit-core/src/path_safety.rs:491:            safe_join(root, "résumé/日本語/file.txt").unwrap(),
crates/blit-core/src/path_safety.rs:606:        let joined = safe_join(root, wire).unwrap();
crates/blit-core/src/path_safety.rs:760:    /// R46-F3: `canonical_dest_root` walks deepest-existing
crates/blit-core/src/path_safety.rs:765:    fn canonical_dest_root_handles_nonexistent_dest() {
crates/blit-core/src/path_safety.rs:768:        let canonical = canonical_dest_root(&nonexistent_dest).unwrap();
crates/blit-core/src/path_safety.rs:778:    /// R46-F3: `safe_join_contained` is the all-in-one helper for
crates/blit-core/src/path_safety.rs:785:    fn safe_join_contained_rejects_symlink_escape() {
crates/blit-core/src/path_safety.rs:796:        let canonical = canonical_dest_root(&dest_root).unwrap();
crates/blit-core/src/path_safety.rs:797:        let err = safe_join_contained(&canonical, &dest_root, "link/victim.txt").unwrap_err();
crates/blit-core/src/path_safety.rs:806:    fn safe_join_contained_passes_for_clean_path() {
crates/blit-core/src/path_safety.rs:810:        let canonical = canonical_dest_root(&dest_root).unwrap();
crates/blit-core/src/path_safety.rs:811:        let target = safe_join_contained(&canonical, &dest_root, "subdir/file.txt").unwrap();
crates/blit-core/src/path_safety.rs:818:    fn safe_join_contained_still_rejects_parent_traversal() {
crates/blit-core/src/path_safety.rs:822:        let canonical = canonical_dest_root(&dest_root).unwrap();
crates/blit-core/src/path_safety.rs:823:        let err = safe_join_contained(&canonical, &dest_root, "../escape").unwrap_err();
crates/blit-core/src/transfer_session/mod.rs:874:    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
crates/blit-core/src/transfer_session/mod.rs:1078:            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
crates/blit-core/src/transfer_session/mod.rs:1080:        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
crates/blit-core/src/path_safety.rs:27://! `safe_join(root, wire)` performs a *lexical* containment check.
crates/blit-core/src/path_safety.rs:142:pub fn safe_join(root: &Path, wire_path: &str) -> Result<PathBuf> {
crates/blit-core/src/path_safety.rs:181:    let target = safe_join(canonical_module_root, wire_path)?;
crates/blit-core/src/path_safety.rs:283:pub fn canonical_dest_root(dest_root: &Path) -> Result<PathBuf> {
crates/blit-core/src/path_safety.rs:312:/// `canonical_dest_root` for the canonical root capture.
crates/blit-core/src/path_safety.rs:321:pub fn safe_join_contained(
crates/blit-core/src/path_safety.rs:326:    let target = safe_join(dest_root, wire_path)?;
crates/blit-core/src/path_safety.rs:378:            safe_join(root, "").unwrap(),
crates/blit-core/src/path_safety.rs:395:            safe_join(root, "foo/bar.txt").unwrap(),
crates/blit-core/src/path_safety.rs:473:        assert!(safe_join(root, "../escape").is_err());
crates/blit-core/src/path_safety.rs:474:        assert!(safe_join(root, "/etc/passwd").is_err());
crates/blit-core/src/path_safety.rs:475:        assert!(safe_join(root, "C:\\evil").is_err());
crates/blit-core/src/path_safety.rs:482:            safe_join(root, "a/b/c/d/e.txt").unwrap(),
crates/blit-core/src/path_safety.rs:491:            safe_join(root, "résumé/日本語/file.txt").unwrap(),
crates/blit-core/src/path_safety.rs:606:        let joined = safe_join(root, wire).unwrap();
crates/blit-core/src/path_safety.rs:760:    /// R46-F3: `canonical_dest_root` walks deepest-existing
crates/blit-core/src/path_safety.rs:765:    fn canonical_dest_root_handles_nonexistent_dest() {
crates/blit-core/src/path_safety.rs:768:        let canonical = canonical_dest_root(&nonexistent_dest).unwrap();
crates/blit-core/src/path_safety.rs:778:    /// R46-F3: `safe_join_contained` is the all-in-one helper for
crates/blit-core/src/path_safety.rs:785:    fn safe_join_contained_rejects_symlink_escape() {
crates/blit-core/src/path_safety.rs:796:        let canonical = canonical_dest_root(&dest_root).unwrap();
crates/blit-core/src/path_safety.rs:797:        let err = safe_join_contained(&canonical, &dest_root, "link/victim.txt").unwrap_err();
crates/blit-core/src/path_safety.rs:806:    fn safe_join_contained_passes_for_clean_path() {
crates/blit-core/src/path_safety.rs:810:        let canonical = canonical_dest_root(&dest_root).unwrap();
crates/blit-core/src/path_safety.rs:811:        let target = safe_join_contained(&canonical, &dest_root, "subdir/file.txt").unwrap();
crates/blit-core/src/path_safety.rs:818:    fn safe_join_contained_still_rejects_parent_traversal() {
crates/blit-core/src/path_safety.rs:822:        let canonical = canonical_dest_root(&dest_root).unwrap();
crates/blit-core/src/path_safety.rs:823:        let err = safe_join_contained(&canonical, &dest_root, "../escape").unwrap_err();
crates/blit-core/src/remote/pull.rs:574:        let canonical_dest_root = crate::path_safety::canonical_dest_root(dest_root).ok();
crates/blit-core/src/remote/pull.rs:746:                        canonical_dest_root.as_deref(),
crates/blit-core/src/remote/pull.rs:841:                        canonical_dest_root.as_deref(),
crates/blit-core/src/remote/pull.rs:895:                        canonical_dest_root.as_deref(),
crates/blit-core/src/remote/pull.rs:930:                        canonical_dest_root.as_deref(),
crates/blit-core/src/remote/pull.rs:980:                        canonical_dest_root.as_deref(),
crates/blit-core/src/remote/pull.rs:1373:    canonical_dest_root: Option<&Path>,
crates/blit-core/src/remote/pull.rs:1395:    if let Some(canonical) = canonical_dest_root {
crates/blit-core/src/remote/pull.rs:1979:/// `canonical_dest_root` is captured once at the entry of `pull()`
crates/blit-core/src/remote/pull.rs:1984:    canonical_dest_root: Option<&Path>,
crates/blit-core/src/remote/pull.rs:1988:    if let Some(canonical) = canonical_dest_root {
crates/blit-core/src/remote/transfer/sink.rs:153:        let canonical_dst_root = crate::path_safety::canonical_dest_root(&dst_root).ok();
crates/blit-core/src/remote/transfer/sink.rs:193:                crate::path_safety::safe_join_contained(canonical, &self.dst_root, wire_path)
crates/blit-core/src/remote/transfer/sink.rs:202:                crate::path_safety::safe_join(&self.dst_root, wire_path)
crates/blit-core/src/remote/transfer/sink.rs:465:            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
crates/blit-core/src/remote/transfer/sink.rs:477:            crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
crates/blit-core/src/remote/transfer/sink.rs:653:            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
crates/blit-core/src/remote/transfer/sink.rs:656:        None => crate::path_safety::safe_join(dst_root, relative_path)
crates/blit-core/src/remote/transfer/sink.rs:698:            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
crates/blit-core/src/remote/transfer/sink.rs:701:        None => crate::path_safety::safe_join(dst_root, relative_path)
crates/blit-core/src/remote/transfer/tar_safety.rs:168:        let dest_path = path_safety::safe_join(dst_root, &rel_string)

exec
/usr/bin/zsh -lc "sed -n '1,140p' crates/blit-core/src/remote/transfer/diff_planner.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Unified diff + payload planning stage.
     2	//!
     3	//! Sits between `TransferSource::scan` (which emits headers from the
     4	//! origin's filesystem) and `execute_sink_pipeline_streaming` (which
     5	//! dispatches payloads to one or more sinks). Decides:
     6	//!
     7	//!   1. Which source headers represent files that genuinely need to
     8	//!      transfer (against the target's destination state).
     9	//!   2. What payload shapes the surviving files become (whole-file
    10	//!      `File` payloads, batched `TarShard`, or — once step 4 lands —
    11	//!      block-level resume `FileBlock` + `FileBlockComplete` pairs).
    12	//!
    13	//! Step 3a of `docs/plan/PIPELINE_UNIFICATION.md`. Today this module
    14	//! consolidates the local-mirror path that lived in `orchestrator.rs`
    15	//! (`filter_headers_for_copy` + the call to `plan_transfer_payloads`).
    16	//! Push and pull will adopt the same module in 3b and step 4.
    17	//!
    18	//! `ComparisonMode` in `proto/blit.proto` is the canonical input shape.
    19	//! As of R2-F1 (`docs/reviews/followup_review_2026-05-02.md`) we honor
    20	//! every variant with concrete semantics — no silent fall-through to
    21	//! size+mtime. This means callers passing `SizeOnly`, `IgnoreTimes`,
    22	//! or `Force` get the behavior the wire enum
    23	//! advertises, not whatever the historical default happened to do.
    24	
    25	use std::path::Path;
    26	
    27	use eyre::{Context, Result};
    28	
    29	use crate::generated::{ComparisonMode, FileHeader};
    30	use crate::remote::transfer::payload::{plan_transfer_payloads, TransferPayload};
    31	use crate::transfer_plan::PlanOptions;
    32	
    33	/// Push origins outsource the diff to the daemon: the client sends its
    34	/// source manifest, daemon returns a NeedList, client filters to the
    35	/// intersection. By the time we plan payloads, the headers are already
    36	/// filtered. This re-exports the existing payload planner under the
    37	/// diff_planner module so the push-client call site goes through the
    38	/// unified module — there's no separate comparison stage to consolidate
    39	/// (the comparison happens on the daemon, not the client).
    40	///
    41	/// When step 4 lands and the daemon-side diff moves into this module
    42	/// for the pull case, push could in principle use the same daemon-side
    43	/// helper instead of the round-trip-via-NeedList protocol. That would
    44	/// be a deeper protocol change tracked under remote→remote re-evaluation
    45	/// (step 5 of `docs/plan/PIPELINE_UNIFICATION.md`).
    46	pub fn plan_push_payloads(
    47	    headers: Vec<FileHeader>,
    48	    source_root: &Path,
    49	    plan_options: PlanOptions,
    50	) -> Result<Vec<TransferPayload>> {
    51	    plan_transfer_payloads(headers, source_root, plan_options).context("planning push payloads")
    52	}
    53	
    54	/// Input bundle for the local-mirror diff stage. Origin and target
    55	/// are co-located (both on the same filesystem), so the comparison
    56	/// can stat the destination directly without a wire roundtrip.
    57	pub struct LocalDiffInputs<'a> {
    58	    /// Source-rooted absolute path. Headers' `relative_path` is
    59	    /// joined under this to find the source bytes.
    60	    pub src_root: &'a Path,
    61	    /// Destination-rooted absolute path. Headers' `relative_path` is
    62	    /// joined under this to compare against existing target state.
    63	    pub dst_root: &'a Path,
    64	    /// How to decide whether a target-existing file matches.
    65	    pub compare_mode: ComparisonMode,
    66	    /// When true, skip any file the destination already has,
    67	    /// regardless of `compare_mode`. Orthogonal axis; matches the
    68	    /// `ignore_existing` field on `TransferOperationSpec`.
    69	    pub ignore_existing: bool,
    70	    /// Knobs for the tar / large / raw planner (unchanged from the
    71	    /// pre-extraction call site).
    72	    pub plan_options: PlanOptions,
    73	    /// When false, every source header passes the comparison stage —
    74	    /// equivalent to `--ignore-times`/`--force` in user-facing terms.
    75	    /// Used by the orchestrator when its `skip_unchanged` flag is off.
    76	    pub skip_unchanged: bool,
    77	}
    78	
    79	/// Filter source headers down to those that need transferring against
    80	/// a local destination, then plan the surviving headers into payloads.
    81	///
    82	/// This is the single entry point the local-mirror path uses. Future
    83	/// origin paths (push client, pull daemon) will gain their own entry
    84	/// points on this module — same diff + planning algorithm, different
    85	/// "where the destination lives" assumption.
    86	pub fn plan_local_mirror(
    87	    source_headers: Vec<FileHeader>,
    88	    inputs: LocalDiffInputs<'_>,
    89	) -> Result<Vec<TransferPayload>> {
    90	    let headers_to_copy = if inputs.skip_unchanged {
    91	        filter_unchanged(
    92	            &source_headers,
    93	            inputs.src_root,
    94	            inputs.dst_root,
    95	            inputs.compare_mode,
    96	            inputs.ignore_existing,
    97	        )
    98	    } else {
    99	        source_headers
   100	    };
   101	
   102	    plan_transfer_payloads(headers_to_copy, inputs.src_root, inputs.plan_options)
   103	        .context("planning payloads after diff stage")
   104	}
   105	
   106	/// Drop headers whose destination file already matches the source
   107	/// under the chosen comparison mode. Keeps headers that need transfer.
   108	///
   109	/// `ignore_existing` is the orthogonal "skip if dst exists" axis from
   110	/// `TransferOperationSpec`: when true, present destination files are
   111	/// dropped before `compare_mode` is consulted at all.
   112	///
   113	/// This is the local-mirror flavor: it stats the destination directly.
   114	/// Remote-source variants (where the destination manifest arrives over
   115	/// the wire) live in their own helpers — TBD step 4.
   116	///
   117	/// Every `ComparisonMode` variant is implemented (R2-F1). `Unspecified`
   118	/// behaves as `SizeMtime` (the historical default) — callers should fold
   119	/// `Unspecified` away via `NormalizedTransferOperation::from_spec`
   120	/// before reaching this function, but we accept it defensively.
   121	pub fn filter_unchanged(
   122	    headers: &[FileHeader],
   123	    src_root: &Path,
   124	    dst_root: &Path,
   125	    compare_mode: ComparisonMode,
   126	    ignore_existing: bool,
   127	) -> Vec<FileHeader> {
   128	    headers
   129	        .iter()
   130	        .filter(|h| {
   131	            let src = src_root.join(&h.relative_path);
   132	            let dst = dst_root.join(&h.relative_path);
   133	            if ignore_existing && dst.exists() {
   134	                return false;
   135	            }
   136	            local_needs_copy(&src, &dst, compare_mode).unwrap_or(true)
   137	        })
   138	        .cloned()
   139	        .collect()
   140	}

exec
/usr/bin/zsh -lc "sed -n '70,140p' crates/blit-core/src/remote/transfer/payload.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	/// (they carry self-contained data). The receive pipeline additionally
     2	/// uses `FileBlock` / `FileBlockComplete` for the resume protocol.
     3	///
     4	/// Streaming file bytes (4 GiB pulls, no point buffering) are NOT a
     5	/// payload variant — they go through `TransferSink::write_file_stream`
     6	/// directly so the receiver can hand the sink a borrowed reader without
     7	/// fighting `'static` trait-object lifetimes.
     8	#[derive(Debug)]
     9	pub enum PreparedPayload {
    10	    /// Whole file, source has it accessible by `src_root.join(relative_path)`.
    11	    /// The sink performs a (zero-copy when possible) local copy.
    12	    File(FileHeader),
    13	    /// In-memory tar shard. Already buffered (bounded by the planner's
    14	    /// shard threshold).
    15	    TarShard {
    16	        headers: Vec<FileHeader>,
    17	        data: Vec<u8>,
    18	    },
    19	    /// Resume: write `bytes` at `offset` into the existing file at
    20	    /// `dst_root.join(relative_path)`.
    21	    FileBlock {
    22	        relative_path: String,
    23	        offset: u64,
    24	        bytes: Vec<u8>,
    25	    },
    26	    /// Resume: finalize the file at `dst_root.join(relative_path)` by
    27	    /// truncating to `total_size` and stamping mtime + perms.
    28	    /// Metadata is carried inline so a "mtime touched, content
    29	    /// identical" mirror correctly updates the destination's mtime
    30	    /// even when zero blocks needed to be transferred.
    31	    FileBlockComplete {
    32	        relative_path: String,
    33	        total_size: u64,
    34	        mtime_seconds: i64,
    35	        permissions: u32,
    36	    },
    37	}
    38	
    39	pub const DEFAULT_PAYLOAD_PREFETCH: usize = 8;
    40	
    41	pub fn plan_transfer_payloads(
    42	    headers: Vec<FileHeader>,
    43	    source_root: &Path,
    44	    options: PlanOptions,
    45	) -> Result<Vec<TransferPayload>> {
    46	    if headers.is_empty() {
    47	        return Ok(Vec::new());
    48	    }
    49	
    50	    let mut entries: Vec<FileEntry> = Vec::with_capacity(headers.len());
    51	    for header in &headers {
    52	        let rel_path = Path::new(&header.relative_path);
    53	        let absolute = source_root.join(rel_path);
    54	        entries.push(FileEntry {
    55	            path: absolute,
    56	            size: header.size,
    57	            is_directory: false,
    58	        });
    59	    }
    60	
    61	    let mut header_map: HashMap<String, FileHeader> = headers
    62	        .into_iter()
    63	        .map(|header| (header.relative_path.clone(), header))
    64	        .collect();
    65	
    66	    let tasks = transfer_plan::build_plan(&entries, source_root, options);
    67	    let mut payloads: Vec<TransferPayload> = Vec::new();
    68	
    69	    for task in tasks {
    70	        match task {
    71	            TransferTask::TarShard(paths) => {

exec
/usr/bin/zsh -lc "sed -n '40,110p' crates/blit-core/src/remote/transfer/source.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	    async fn open_file(
     2	        &self,
     3	        header: &FileHeader,
     4	    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>>;
     5	
     6	    /// Returns the root path of the source (if applicable).
     7	    fn root(&self) -> &Path;
     8	}
     9	
    10	pub struct FsTransferSource {
    11	    root: PathBuf,
    12	}
    13	
    14	impl FsTransferSource {
    15	    pub fn new(root: PathBuf) -> Self {
    16	        Self { root }
    17	    }
    18	}
    19	
    20	#[async_trait]
    21	impl TransferSource for FsTransferSource {
    22	    fn scan(
    23	        &self,
    24	        filter: Option<FileFilter>,
    25	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    26	    ) -> (
    27	        mpsc::Receiver<FileHeader>,
    28	        tokio::task::JoinHandle<Result<u64>>,
    29	    ) {
    30	        use crate::remote::push::client::helpers::spawn_manifest_task;
    31	        spawn_manifest_task(
    32	            self.root.clone(),
    33	            filter.unwrap_or_default(),
    34	            unreadable_paths,
    35	        )
    36	    }
    37	
    38	    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
    39	        use crate::remote::transfer::payload::prepare_payload;
    40	        prepare_payload(payload, self.root.clone()).await
    41	    }
    42	
    43	    async fn check_availability(
    44	        &self,
    45	        headers: Vec<FileHeader>,
    46	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    47	    ) -> Result<Vec<FileHeader>> {
    48	        use crate::remote::push::client::helpers::filter_readable_headers;
    49	        filter_readable_headers(&self.root, headers, &unreadable_paths).await
    50	    }
    51	
    52	    async fn open_file(
    53	        &self,
    54	        header: &FileHeader,
    55	    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
    56	        // An empty relative_path means "the root itself" — used when the
    57	        // enumeration root is a single file. Don't join, because
    58	        // PathBuf::join with some Path representations can produce a
    59	        // trailing-slash form that OS interprets as "descend into" and
    60	        // fails with ENOTDIR when the root is a regular file.
    61	        let path = if header.relative_path.is_empty() {
    62	            self.root.clone()
    63	        } else {
    64	            self.root.join(&header.relative_path)
    65	        };
    66	        let file = fs::File::open(&path).await?;
    67	        Ok(Box::new(file))
    68	    }
    69	
    70	    fn root(&self) -> &Path {
    71	        &self.root

exec
/usr/bin/zsh -lc "sed -n '218,380p' crates/blit-core/src/remote/transfer/sink.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
     2	        // Resume payloads need async I/O (file open + seek + write
     3	        // through tokio). Local-source payloads (File / TarShard) stay
     4	        // on a blocking thread so the zero-copy cascade and tar
     5	        // extraction can use std::fs.
     6	        let outcome = match payload {
     7	            PreparedPayload::FileBlock {
     8	                relative_path,
     9	                offset,
    10	                bytes,
    11	            } => {
    12	                write_file_block_payload(
    13	                    &self.dst_root,
    14	                    self.canonical_dst_root.as_deref(),
    15	                    &relative_path,
    16	                    offset,
    17	                    bytes,
    18	                )
    19	                .await?
    20	            }
    21	            PreparedPayload::FileBlockComplete {
    22	                relative_path,
    23	                total_size,
    24	                mtime_seconds,
    25	                permissions,
    26	            } => {
    27	                let outcome = write_file_block_complete(
    28	                    &self.dst_root,
    29	                    self.canonical_dst_root.as_deref(),
    30	                    &relative_path,
    31	                    total_size,
    32	                    mtime_seconds,
    33	                    permissions,
    34	                )
    35	                .await?;
    36	                if outcome.files_written > 0 {
    37	                    self.track(&relative_path);
    38	                }
    39	                outcome
    40	            }
    41	            PreparedPayload::File(_) | PreparedPayload::TarShard { .. } => {
    42	                // Capture paths for tracking before payload moves into
    43	                // the spawn_blocking closure.
    44	                let tracked_paths: Vec<String> = match &payload {
    45	                    PreparedPayload::File(h) => vec![h.relative_path.clone()],
    46	                    PreparedPayload::TarShard { headers, .. } => {
    47	                        headers.iter().map(|h| h.relative_path.clone()).collect()
    48	                    }
    49	                    _ => Vec::new(),
    50	                };
    51	                let src_root = self.src_root.clone();
    52	                let dst_root = self.dst_root.clone();
    53	                let canonical_dst_root = self.canonical_dst_root.clone();
    54	                let config = self.config.clone();
    55	                let outcome = tokio::task::spawn_blocking(move || match payload {
    56	                    PreparedPayload::File(header) => write_file_payload(
    57	                        &src_root,
    58	                        &dst_root,
    59	                        canonical_dst_root.as_deref(),
    60	                        &header,
    61	                        &config,
    62	                    ),
    63	                    PreparedPayload::TarShard { headers, data } => write_tar_shard_payload(
    64	                        &dst_root,
    65	                        canonical_dst_root.as_deref(),
    66	                        &headers,
    67	                        &data,
    68	                        &config,
    69	                    ),
    70	                    _ => unreachable!("outer match guarantees File or TarShard"),
    71	                })
    72	                .await
    73	                .context("sink worker panicked")??;
    74	                if outcome.files_written > 0 {
    75	                    for path in tracked_paths {
    76	                        self.track(&path);
    77	                    }
    78	                }
    79	                outcome
    80	            }
    81	        };
    82	        // c-1b round 2: tar shards and resume blocks land via
    83	        // write_payload, not write_file_stream, so the chunk-
    84	        // granular `receive_stream_double_buffered` hook never
    85	        // fires for them. Report `outcome.bytes_written` here so
    86	        // `GetState.active[].bytes_completed` reflects bytes
    87	        // landed on disk for ALL payload shapes, not just
    88	        // streamed files. Dry-run write paths return
    89	        // `bytes_written: 0` (see `write_file_payload` and
    90	        // `write_tar_shard_payload`'s dry-run early returns), so
    91	        // adding 0 is a no-op for previews — same semantics as
    92	        // `write_file_stream`'s dry-run branch.
    93	        if let Some(bp) = &self.byte_progress {
    94	            bp.report(outcome.bytes_written);
    95	        }
    96	        Ok(outcome)
    97	    }
    98	
    99	    /// Stream file bytes from the wire to the destination filesystem
   100	    /// using the same double-buffered helper the send side uses. This
   101	    /// is what makes push and pull receive symmetric on the FsTransferSink.
   102	    async fn write_file_stream(
   103	        &self,
   104	        header: &FileHeader,
   105	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   106	    ) -> Result<SinkOutcome> {
   107	        use crate::remote::transfer::data_plane::{
   108	            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
   109	        };
   110	
   111	        // R46-F3: lexical resolve + canonical containment check via
   112	        // resolve_destination. Pre-fix this was a bare safe_join,
   113	        // which rejected lexical traversal (`../`) but didn't catch
   114	        // the case where dst_root contained a pre-existing symlink
   115	        // pointing outside (`dst_root/link → /outside`); a peer-
   116	        // controlled relative path `link/file` would then write to
   117	        // `/outside/file`.
   118	        let dst = self
   119	            .resolve_destination(&header.relative_path)
   120	            .with_context(|| format!("validating receive path {:?}", header.relative_path))?;
   121	
   122	        // R58-F4: dry-run must be side-effect-free. Drain the wire
   123	        // for protocol-stream alignment, but skip the parent-mkdir
   124	        // and the file write. Pre-fix the parent-mkdir ran before
   125	        // the dry-run check below, so `--dry-run` over a remote
   126	        // transfer would create destination directories.
   127	        if self.config.dry_run {
   128	            let mut sink = tokio::io::sink();
   129	            // Dry-run: drain wire bytes for protocol alignment.
   130	            // Do NOT report against `byte_progress` — by contract
   131	            // dry-run is side-effect-free and these bytes never
   132	            // hit user disk; we don't want a daemon-side bytes_completed
   133	            // counter to advance for an aborted preview.
   134	            receive_stream_double_buffered(
   135	                reader,
   136	                &mut sink,
   137	                header.size,
   138	                RECEIVE_CHUNK_SIZE,
   139	                None,
   140	            )
   141	            .await
   142	            .with_context(|| format!("draining {} (dry-run)", header.relative_path))?;
   143	            return Ok(SinkOutcome {
   144	                files_written: 1,
   145	                bytes_written: 0,
   146	            });
   147	        }
   148	
   149	        if let Some(parent) = dst.parent() {
   150	            tokio::fs::create_dir_all(parent)
   151	                .await
   152	                .with_context(|| format!("creating directory {}", parent.display()))?;
   153	        }
   154	
   155	        {
   156	            use tokio::io::AsyncWriteExt as _;
   157	            let mut file = tokio::fs::File::create(&dst)
   158	                .await
   159	                .with_context(|| format!("creating {}", dst.display()))?;
   160	            receive_stream_double_buffered(
   161	                reader,
   162	                &mut file,
   163	                header.size,

exec
/usr/bin/zsh -lc "sed -n '319,430p' crates/blit-core/src/remote/transfer/sink.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	    async fn write_file_stream(
     2	        &self,
     3	        header: &FileHeader,
     4	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
     5	    ) -> Result<SinkOutcome> {
     6	        use crate::remote::transfer::data_plane::{
     7	            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
     8	        };
     9	
    10	        // R46-F3: lexical resolve + canonical containment check via
    11	        // resolve_destination. Pre-fix this was a bare safe_join,
    12	        // which rejected lexical traversal (`../`) but didn't catch
    13	        // the case where dst_root contained a pre-existing symlink
    14	        // pointing outside (`dst_root/link → /outside`); a peer-
    15	        // controlled relative path `link/file` would then write to
    16	        // `/outside/file`.
    17	        let dst = self
    18	            .resolve_destination(&header.relative_path)
    19	            .with_context(|| format!("validating receive path {:?}", header.relative_path))?;
    20	
    21	        // R58-F4: dry-run must be side-effect-free. Drain the wire
    22	        // for protocol-stream alignment, but skip the parent-mkdir
    23	        // and the file write. Pre-fix the parent-mkdir ran before
    24	        // the dry-run check below, so `--dry-run` over a remote
    25	        // transfer would create destination directories.
    26	        if self.config.dry_run {
    27	            let mut sink = tokio::io::sink();
    28	            // Dry-run: drain wire bytes for protocol alignment.
    29	            // Do NOT report against `byte_progress` — by contract
    30	            // dry-run is side-effect-free and these bytes never
    31	            // hit user disk; we don't want a daemon-side bytes_completed
    32	            // counter to advance for an aborted preview.
    33	            receive_stream_double_buffered(
    34	                reader,
    35	                &mut sink,
    36	                header.size,
    37	                RECEIVE_CHUNK_SIZE,
    38	                None,
    39	            )
    40	            .await
    41	            .with_context(|| format!("draining {} (dry-run)", header.relative_path))?;
    42	            return Ok(SinkOutcome {
    43	                files_written: 1,
    44	                bytes_written: 0,
    45	            });
    46	        }
    47	
    48	        if let Some(parent) = dst.parent() {
    49	            tokio::fs::create_dir_all(parent)
    50	                .await
    51	                .with_context(|| format!("creating directory {}", parent.display()))?;
    52	        }
    53	
    54	        {
    55	            use tokio::io::AsyncWriteExt as _;
    56	            let mut file = tokio::fs::File::create(&dst)
    57	                .await
    58	                .with_context(|| format!("creating {}", dst.display()))?;
    59	            receive_stream_double_buffered(
    60	                reader,
    61	                &mut file,
    62	                header.size,
    63	                RECEIVE_CHUNK_SIZE,
    64	                self.byte_progress.as_ref(),
    65	            )
    66	            .await
    67	            .with_context(|| format!("writing {}", dst.display()))?;
    68	            // Flush the tokio File's internal buffer state (does NOT
    69	            // fsync — just ensures user-space buffering is drained
    70	            // before we drop the handle and apply mtime). Without
    71	            // this, set_file_mtime races with deferred writes from
    72	            // tokio's blocking-thread pool: 5/8 of mtimes were
    73	            // observed silently bumped to "now" on the receive side.
    74	            //
    75	            // POST_REVIEW_FIXES §1.1: flush failure is a data-loss
    76	            // signal — the user believes the file is durable when it
    77	            // isn't. Propagate, don't swallow.
    78	            file.flush()
    79	                .await
    80	                .with_context(|| format!("flushing {}", dst.display()))?;
    81	        }
    82	        // Handle dropped → kernel close() complete → no further
    83	        // metadata churn from this file. Now safe to set mtime by path.
    84	
    85	        // Intentionally no sync_all: ZFS commits per fsync are
    86	        // multi-second on spinning rust and crater throughput
    87	        // (9.3 → 3.3 Gbps observed). The transfer's durability signal
    88	        // is its END marker plus the OS's own flush; matches rsync's
    89	        // default behavior. Add a config flag if a caller needs sync.
    90	
    91	        if self.config.preserve_times && header.mtime_seconds > 0 {
    92	            let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
    93	            // Best-effort: cross-fs, root-owned, or ACL-protected
    94	            // destinations can refuse mtime updates. Surface via
    95	            // `log::warn!` so the failure is visible without making
    96	            // it a hard transfer error. POST_REVIEW_FIXES §1.1.
    97	            if let Err(e) = filetime::set_file_mtime(&dst, ft) {
    98	                log::warn!("set mtime on {}: {}", dst.display(), e);
    99	            }
   100	        }
   101	
   102	        // Permissions arrive on the wire (Unix mode bits). Apply best-
   103	        // effort; ignore failures (cross-fs, root-owned dst, etc.).
   104	        #[cfg(unix)]
   105	        if header.permissions != 0 {
   106	            use std::os::unix::fs::PermissionsExt;
   107	            if let Err(e) =
   108	                std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(header.permissions))
   109	            {
   110	                log::warn!("set permissions on {}: {}", dst.display(), e);
   111	            }
   112	        }

exec
/usr/bin/zsh -lc "sed -n '350,390p' proto/blit.proto | nl -ba && sed -n '1240,1378p' proto/blit.proto | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	message FileHeader {
     2	  string relative_path = 1;
     3	  uint64 size = 2;
     4	  int64 mtime_seconds = 3;
     5	  uint32 permissions = 4;
     6	  bytes checksum = 5;  // Blake3 hash (32 bytes), empty if not computed
     7	}
     8	
     9	message FileData {
    10	  bytes content = 1;
    11	}
    12	
    13	message TarShardHeader {
    14	  repeated FileHeader files = 1;
    15	  uint64 archive_size = 2;
    16	}
    17	
    18	message TarShardChunk {
    19	  bytes content = 1;
    20	}
    21	
    22	message TarShardComplete {}
    23	
    24	message ManifestComplete {
    25	  // R59 #1 F1: client tells the daemon whether its source-side
    26	  // scan finished cleanly. False when any subtree was unreadable
    27	  // (EACCES, ELOOP, IO errors). Required for the daemon to
    28	  // safely purge in mirror mode — see PushHeader.require_complete_scan.
    29	  // Pre-fix the daemon purged destination entries unconditionally
    30	  // after upload, so a permission error mid-scan caused silent
    31	  // data loss on the destination.
    32	  bool scan_complete = 1;
    33	}
    34	message UploadComplete {}
    35	message Ack {}
    36	
    37	// Acknowledgment for PullSync with server capabilities
    38	message PullSyncAck {
    39	  bool server_checksums_enabled = 1;  // Whether daemon computed checksums for manifest
    40	}
    41	message FileList { repeated string relative_paths = 1; }
     1	  // Bumped on any wire-shape change; exact match required.
     2	  uint32 contract_version = 2;
     3	}
     4	
     5	// Initiator's second frame: the whole operation, roles included.
     6	message SessionOpen {
     7	  // Role the INITIATOR takes; the responder takes the other.
     8	  TransferRole initiator_role = 1;
     9	  // Responder-side module (empty = default root export) and path
    10	  // within it. The initiator-side path never crosses the wire — the
    11	  // initiator owns its local endpoint.
    12	  string module = 2;
    13	  string path = 3;
    14	  FilterSpec filter = 4;
    15	  ComparisonMode compare_mode = 5;
    16	  // Mirror is explicit: enabled + scope. No implicit mirror.
    17	  bool mirror_enabled = 6;
    18	  MirrorMode mirror_kind = 7;
    19	  ResumeSettings resume = 8;
    20	  // Request the in-stream byte carrier (diagnostics / unreachable
    21	  // data-plane environments). The responder may also force it via a
    22	  // grant-less SessionAccept when it cannot bind a listener.
    23	  bool in_stream_bytes = 9;
    24	  bool ignore_existing = 10;
    25	  bool require_complete_scan = 11;
    26	  // Set iff the initiator is DESTINATION (dial contract: the byte
    27	  // receiver advertises capacity — D-2026-06-20-1/-2; absent/0 =
    28	  // unknown hardware value, conservative, never "old peer").
    29	  CapacityProfile receiver_capacity = 12;
    30	}
    31	
    32	// Responder's reply. Refusals are SessionError frames, never silent
    33	// closes.
    34	message SessionAccept {
    35	  // Set iff the responder is DESTINATION.
    36	  CapacityProfile receiver_capacity = 1;
    37	  // Absent = in-stream carrier (requested, or listener bind failed).
    38	  DataPlaneGrant data_plane = 2;
    39	}
    40	
    41	// TCP data-plane grant. The RESPONDER always binds; the INITIATOR
    42	// always dials (connection topology, not choreography — byte
    43	// direction on the sockets is set by role: SOURCE writes).
    44	message DataPlaneGrant {
    45	  uint32 tcp_port = 1;
    46	  bytes session_token = 2;
    47	  // ACCEPT ceiling, not a dial order (D-2026-06-20-1/-2: the SENDER
    48	  // owns the dial): the number of epoch-0 accept slots the responder
    49	  // arms — min(engine dial floor, DESTINATION capacity ceiling).
    50	  // SOURCE may dial fewer; unclaimed slots expire. Growth happens
    51	  // only via SOURCE-initiated resize (sf-2 shape correction), one
    52	  // armed accept per ADD epoch.
    53	  uint32 initial_streams = 3;
    54	  // Epoch-0 socket credential. Auth handshake per socket
    55	  // (docs/TRANSFER_SESSION.md §Transport): session_token (16 bytes)
    56	  // then epoch0_sub_token (16 bytes) for epoch-0 sockets;
    57	  // session_token then the epoch's sub_token for resize-ADD sockets.
    58	  // One socket per armed slot; unclaimed slots expire.
    59	  bytes epoch0_sub_token = 4;
    60	}
    61	
    62	// DESTINATION → SOURCE: files the destination wants, in batches.
    63	message NeedEntry {
    64	  string relative_path = 1;
    65	  // RELIABLE resume exception (docs/TRANSFER_SESSION.md): when true,
    66	  // the destination's BlockHashList for this file follows, and the
    67	  // source must not send any byte of this file before receiving it;
    68	  // stale/mismatched partials fall back to full-file transfer.
    69	  bool resume = 2;
    70	}
    71	message NeedBatch {
    72	  repeated NeedEntry entries = 1;
    73	}
    74	// DESTINATION's promise that no further NeedBatch follows.
    75	message NeedComplete {}
    76	
    77	// SOURCE's promise that every requested payload byte is flushed.
    78	message SourceDone {}
    79	
    80	// DESTINATION → SOURCE at close: the end that wrote bytes and
    81	// executed deletes attests to the outcome (one summary shape for
    82	// every direction; replaces PushSummary/PullSummary at cutover).
    83	message TransferSummary {
    84	  uint64 files_transferred = 1;
    85	  uint64 bytes_transferred = 2;
    86	  uint64 entries_deleted = 3;   // mirror executed destination-local
    87	  bool in_stream_carrier_used = 4;
    88	  uint64 files_resumed = 5;
    89	}
    90	
    91	// Structured refusal/abort — an end says why before closing.
    92	message SessionError {
    93	  enum Code {
    94	    SESSION_ERROR_UNSPECIFIED = 0;
    95	    BUILD_MISMATCH = 1;
    96	    MODULE_UNKNOWN = 2;
    97	    READ_ONLY = 3;
    98	    DELEGATION_REFUSED = 4;
    99	    SCAN_INCOMPLETE = 5;
   100	    PROTOCOL_VIOLATION = 6;
   101	    DATA_PLANE_FAILED = 7;
   102	    CANCELLED = 8;
   103	    INTERNAL = 9;
   104	  }
   105	  Code code = 1;
   106	  string message = 2;
   107	  // BUILD_MISMATCH: both build ids, so the operator sees exactly
   108	  // which end is stale.
   109	  string local_build_id = 3;
   110	  string peer_build_id = 4;
   111	}
   112	
   113	// The single frame type BOTH wire directions carry. Which frames an
   114	// end may send is determined by ROLE and phase
   115	// (docs/TRANSFER_SESSION.md frame table — field numbers frozen
   116	// there). Reuses the engine's existing payload vocabulary verbatim.
   117	message TransferFrame {
   118	  oneof frame {
   119	    SessionHello hello = 1;
   120	    SessionOpen open = 2;
   121	    SessionAccept accept = 3;
   122	    FileHeader manifest_entry = 4;
   123	    ManifestComplete manifest_complete = 5;
   124	    NeedBatch need_batch = 6;
   125	    NeedComplete need_complete = 7;
   126	    BlockHashList block_hashes = 8;
   127	    FileHeader file_begin = 9;
   128	    FileData file_data = 10;
   129	    TarShardHeader tar_shard_header = 11;
   130	    TarShardChunk tar_shard_chunk = 12;
   131	    TarShardComplete tar_shard_complete = 13;
   132	    BlockTransfer block = 14;
   133	    BlockTransferComplete block_complete = 15;
   134	    DataPlaneResize resize = 16;
   135	    DataPlaneResizeAck resize_ack = 17;
   136	    SourceDone source_done = 18;
   137	    TransferSummary summary = 19;
   138	    SessionError error = 20;
   139	  }

codex
The source and destination share the new session code, but several protections depend on how existing helpers behave. I’m checking those helpers now, especially whether the sink reports exact bytes and whether the planner can produce payload records that the destination accounts for correctly.
exec
/usr/bin/zsh -lc "sed -n '380,520p' crates/blit-core/src/remote/transfer/sink.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	                header.size,
     2	                RECEIVE_CHUNK_SIZE,
     3	                self.byte_progress.as_ref(),
     4	            )
     5	            .await
     6	            .with_context(|| format!("writing {}", dst.display()))?;
     7	            // Flush the tokio File's internal buffer state (does NOT
     8	            // fsync — just ensures user-space buffering is drained
     9	            // before we drop the handle and apply mtime). Without
    10	            // this, set_file_mtime races with deferred writes from
    11	            // tokio's blocking-thread pool: 5/8 of mtimes were
    12	            // observed silently bumped to "now" on the receive side.
    13	            //
    14	            // POST_REVIEW_FIXES §1.1: flush failure is a data-loss
    15	            // signal — the user believes the file is durable when it
    16	            // isn't. Propagate, don't swallow.
    17	            file.flush()
    18	                .await
    19	                .with_context(|| format!("flushing {}", dst.display()))?;
    20	        }
    21	        // Handle dropped → kernel close() complete → no further
    22	        // metadata churn from this file. Now safe to set mtime by path.
    23	
    24	        // Intentionally no sync_all: ZFS commits per fsync are
    25	        // multi-second on spinning rust and crater throughput
    26	        // (9.3 → 3.3 Gbps observed). The transfer's durability signal
    27	        // is its END marker plus the OS's own flush; matches rsync's
    28	        // default behavior. Add a config flag if a caller needs sync.
    29	
    30	        if self.config.preserve_times && header.mtime_seconds > 0 {
    31	            let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
    32	            // Best-effort: cross-fs, root-owned, or ACL-protected
    33	            // destinations can refuse mtime updates. Surface via
    34	            // `log::warn!` so the failure is visible without making
    35	            // it a hard transfer error. POST_REVIEW_FIXES §1.1.
    36	            if let Err(e) = filetime::set_file_mtime(&dst, ft) {
    37	                log::warn!("set mtime on {}: {}", dst.display(), e);
    38	            }
    39	        }
    40	
    41	        // Permissions arrive on the wire (Unix mode bits). Apply best-
    42	        // effort; ignore failures (cross-fs, root-owned dst, etc.).
    43	        #[cfg(unix)]
    44	        if header.permissions != 0 {
    45	            use std::os::unix::fs::PermissionsExt;
    46	            if let Err(e) =
    47	                std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(header.permissions))
    48	            {
    49	                log::warn!("set permissions on {}: {}", dst.display(), e);
    50	            }
    51	        }
    52	        #[cfg(not(unix))]
    53	        let _ = header.permissions;
    54	
    55	        self.track(&header.relative_path);
    56	
    57	        Ok(SinkOutcome {
    58	            files_written: 1,
    59	            bytes_written: header.size,
    60	        })
    61	    }
    62	
    63	    fn root(&self) -> &Path {
    64	        &self.dst_root
    65	    }
    66	}
    67	
    68	/// Copy a single file using the zero-copy cascade in `copy::file_copy`.
    69	fn write_file_payload(
    70	    src_root: &Path,
    71	    dst_root: &Path,
    72	    canonical_dst_root: Option<&Path>,
    73	    header: &FileHeader,
    74	    config: &FsSinkConfig,
    75	) -> Result<SinkOutcome> {
    76	    let src = src_root.join(&header.relative_path);
    77	    // R47-F1: the FsTransferSink::write_payload arm for
    78	    // PreparedPayload::File hit this helper, which previously
    79	    // joined dst_root + header.relative_path lexically. A peer-
    80	    // controlled `link/file` with a pre-existing `dst/link →
    81	    // /outside` symlink would write outside the destination root.
    82	    // Route through the same canonical-containment chokepoint that
    83	    // write_file_stream uses.
    84	    let dst = match canonical_dst_root {
    85	        Some(canonical) => {
    86	            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
    87	                .with_context(|| {
    88	                    format!("validating file payload path {:?}", header.relative_path)
    89	                })?
    90	        }
    91	        None => {
    92	            log::warn!(
    93	                "write_file_payload at '{}' has no canonical root; \
    94	                 falls back to lexical-only path check (R47-F1 \
    95	                 escape protection unavailable)",
    96	                dst_root.display()
    97	            );
    98	            crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
    99	                format!("validating file payload path {:?}", header.relative_path)
   100	            })?
   101	        }
   102	    };
   103	
   104	    // R58-F4: dry-run must be side-effect-free. Bail before the
   105	    // parent-mkdir so a dry-run doesn't create destination
   106	    // directories on disk.
   107	    if config.dry_run {
   108	        return Ok(SinkOutcome {
   109	            files_written: 1,
   110	            bytes_written: 0,
   111	        });
   112	    }
   113	
   114	    if let Some(parent) = dst.parent() {
   115	        std::fs::create_dir_all(parent)
   116	            .with_context(|| format!("creating directory {}", parent.display()))?;
   117	    }
   118	
   119	    let mut did_copy = false;
   120	    let mut clone_succeeded = false;
   121	
   122	    if config.resume {
   123	        let outcome = resume_copy_file(&src, &dst, 0)
   124	            .with_context(|| format!("resume copy {}", header.relative_path))?;
   125	        did_copy = outcome.bytes_transferred > 0;
   126	    } else if crate::copy::file_needs_copy_with_mode(&src, &dst, config.compare_mode)? {
   127	        let sizer = BufferSizer::default();
   128	        let logger = NoopLogger;
   129	        let outcome = copy_file(&src, &dst, &sizer, false, &logger)
   130	            .with_context(|| format!("copy {}", header.relative_path))?;
   131	        did_copy = true;
   132	        clone_succeeded = outcome.clone_succeeded;
   133	    }
   134	
   135	    if config.preserve_times && did_copy && !clone_succeeded {
   136	        if let Ok(meta) = std::fs::metadata(&src) {
   137	            if let Ok(modified) = meta.modified() {
   138	                let ft = FileTime::from_system_time(modified);
   139	                if let Err(e) = filetime::set_file_mtime(&dst, ft) {
   140	                    log::warn!("set mtime on {}: {}", dst.display(), e);
   141	                }

exec
/usr/bin/zsh -lc "sed -n '430,560p' crates/blit-core/src/remote/transfer/sink.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	        }
     2	        #[cfg(not(unix))]
     3	        let _ = header.permissions;
     4	
     5	        self.track(&header.relative_path);
     6	
     7	        Ok(SinkOutcome {
     8	            files_written: 1,
     9	            bytes_written: header.size,
    10	        })
    11	    }
    12	
    13	    fn root(&self) -> &Path {
    14	        &self.dst_root
    15	    }
    16	}
    17	
    18	/// Copy a single file using the zero-copy cascade in `copy::file_copy`.
    19	fn write_file_payload(
    20	    src_root: &Path,
    21	    dst_root: &Path,
    22	    canonical_dst_root: Option<&Path>,
    23	    header: &FileHeader,
    24	    config: &FsSinkConfig,
    25	) -> Result<SinkOutcome> {
    26	    let src = src_root.join(&header.relative_path);
    27	    // R47-F1: the FsTransferSink::write_payload arm for
    28	    // PreparedPayload::File hit this helper, which previously
    29	    // joined dst_root + header.relative_path lexically. A peer-
    30	    // controlled `link/file` with a pre-existing `dst/link →
    31	    // /outside` symlink would write outside the destination root.
    32	    // Route through the same canonical-containment chokepoint that
    33	    // write_file_stream uses.
    34	    let dst = match canonical_dst_root {
    35	        Some(canonical) => {
    36	            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
    37	                .with_context(|| {
    38	                    format!("validating file payload path {:?}", header.relative_path)
    39	                })?
    40	        }
    41	        None => {
    42	            log::warn!(
    43	                "write_file_payload at '{}' has no canonical root; \
    44	                 falls back to lexical-only path check (R47-F1 \
    45	                 escape protection unavailable)",
    46	                dst_root.display()
    47	            );
    48	            crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
    49	                format!("validating file payload path {:?}", header.relative_path)
    50	            })?
    51	        }
    52	    };
    53	
    54	    // R58-F4: dry-run must be side-effect-free. Bail before the
    55	    // parent-mkdir so a dry-run doesn't create destination
    56	    // directories on disk.
    57	    if config.dry_run {
    58	        return Ok(SinkOutcome {
    59	            files_written: 1,
    60	            bytes_written: 0,
    61	        });
    62	    }
    63	
    64	    if let Some(parent) = dst.parent() {
    65	        std::fs::create_dir_all(parent)
    66	            .with_context(|| format!("creating directory {}", parent.display()))?;
    67	    }
    68	
    69	    let mut did_copy = false;
    70	    let mut clone_succeeded = false;
    71	
    72	    if config.resume {
    73	        let outcome = resume_copy_file(&src, &dst, 0)
    74	            .with_context(|| format!("resume copy {}", header.relative_path))?;
    75	        did_copy = outcome.bytes_transferred > 0;
    76	    } else if crate::copy::file_needs_copy_with_mode(&src, &dst, config.compare_mode)? {
    77	        let sizer = BufferSizer::default();
    78	        let logger = NoopLogger;
    79	        let outcome = copy_file(&src, &dst, &sizer, false, &logger)
    80	            .with_context(|| format!("copy {}", header.relative_path))?;
    81	        did_copy = true;
    82	        clone_succeeded = outcome.clone_succeeded;
    83	    }
    84	
    85	    if config.preserve_times && did_copy && !clone_succeeded {
    86	        if let Ok(meta) = std::fs::metadata(&src) {
    87	            if let Ok(modified) = meta.modified() {
    88	                let ft = FileTime::from_system_time(modified);
    89	                if let Err(e) = filetime::set_file_mtime(&dst, ft) {
    90	                    log::warn!("set mtime on {}: {}", dst.display(), e);
    91	                }
    92	            }
    93	        }
    94	    }
    95	
    96	    Ok(SinkOutcome {
    97	        files_written: 1,
    98	        bytes_written: if did_copy { header.size } else { 0 },
    99	    })
   100	}
   101	
   102	/// Extract an in-memory tar shard to the destination directory.
   103	fn write_tar_shard_payload(
   104	    dst_root: &Path,
   105	    canonical_dst_root: Option<&Path>,
   106	    headers: &[FileHeader],
   107	    data: &[u8],
   108	    config: &FsSinkConfig,
   109	) -> Result<SinkOutcome> {
   110	    if config.dry_run {
   111	        return Ok(SinkOutcome {
   112	            files_written: headers.len(),
   113	            bytes_written: 0,
   114	        });
   115	    }
   116	
   117	    // Two-phase extraction:
   118	    //   1. Validate + parse the tar serially via the shared
   119	    //      `tar_safety` helper. Tar is a sequential format — entries
   120	    //      can't be read in parallel out of one Archive — and this
   121	    //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
   122	    //   2. Write files to disk in parallel via rayon. Inode creation
   123	    //      and write are the bottleneck for many-small-files shards;
   124	    //      4–8 worker cores can saturate ZFS' inode pipeline.
   125	    //
   126	    // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
   127	    // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
   128	    // toward CPU-or-fs limits.
   129	    use rayon::prelude::*;
   130	
   131	    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};

exec
/usr/bin/zsh -lc "sed -n '540,740p' crates/blit-core/src/remote/transfer/sink.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	        return Ok(SinkOutcome {
     2	            files_written: headers.len(),
     3	            bytes_written: 0,
     4	        });
     5	    }
     6	
     7	    // Two-phase extraction:
     8	    //   1. Validate + parse the tar serially via the shared
     9	    //      `tar_safety` helper. Tar is a sequential format — entries
    10	    //      can't be read in parallel out of one Archive — and this
    11	    //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
    12	    //   2. Write files to disk in parallel via rayon. Inode creation
    13	    //      and write are the bottleneck for many-small-files shards;
    14	    //      4–8 worker cores can saturate ZFS' inode pipeline.
    15	    //
    16	    // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
    17	    // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
    18	    // toward CPU-or-fs limits.
    19	    use rayon::prelude::*;
    20	
    21	    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};
    22	
    23	    let opts = TarShardExtractOptions::default();
    24	    let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;
    25	
    26	    // R47-F1: tar shards arriving on FsTransferSink::write_payload
    27	    // (push-receive on the daemon flows through here too) only had
    28	    // lexical safe_join inside safe_extract_tar_shard. A pre-
    29	    // existing dst/link → /outside escape symlink would let an
    30	    // entry path like `link/victim` write through the symlink.
    31	    // Verify each extracted entry's destination against the
    32	    // canonical root before writing.
    33	    if let Some(canonical) = canonical_dst_root {
    34	        for f in &extracted {
    35	            crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
    36	                format!("tar shard entry {:?} escapes destination root", f.dest_path)
    37	            })?;
    38	        }
    39	    } else {
    40	        log::warn!(
    41	            "write_tar_shard_payload at '{}' has no canonical root; \
    42	             tar-shard receive falls back to lexical-only path \
    43	             checks (R47-F1 escape protection unavailable)",
    44	            dst_root.display()
    45	        );
    46	    }
    47	
    48	    // Honor the sink's preserve_times toggle by stripping mtimes that
    49	    // the helper would otherwise apply. Permissions are best-effort
    50	    // either way (matches the historical FsTransferSink policy).
    51	    if !config.preserve_times {
    52	        for f in &mut extracted {
    53	            f.mtime = None;
    54	        }
    55	    }
    56	
    57	    // Write in parallel. Each closure does its own create_dir_all +
    58	    // fs::write + best-effort mtime/permission application — same
    59	    // policy as `tar_safety::write_extracted_file` but inlined so we
    60	    // can return per-file byte counts for the SinkOutcome.
    61	    let results: Vec<Result<u64>> = extracted
    62	        .into_par_iter()
    63	        .map(|f: ExtractedFile| -> Result<u64> {
    64	            if let Some(parent) = f.dest_path.parent() {
    65	                std::fs::create_dir_all(parent)
    66	                    .with_context(|| format!("create dir {}", parent.display()))?;
    67	            }
    68	            std::fs::write(&f.dest_path, &f.contents)
    69	                .with_context(|| format!("write {}", f.dest_path.display()))?;
    70	            if let Some(ft) = f.mtime {
    71	                if let Err(e) = filetime::set_file_mtime(&f.dest_path, ft) {
    72	                    log::warn!("set mtime on {}: {}", f.dest_path.display(), e);
    73	                }
    74	            }
    75	            #[cfg(unix)]
    76	            if let Some(perms) = f.permissions {
    77	                use std::os::unix::fs::PermissionsExt;
    78	                if let Err(e) =
    79	                    std::fs::set_permissions(&f.dest_path, std::fs::Permissions::from_mode(perms))
    80	                {
    81	                    log::warn!("set permissions on {}: {}", f.dest_path.display(), e);
    82	                }
    83	            }
    84	            Ok(f.size)
    85	        })
    86	        .collect();
    87	
    88	    let mut files_written = 0usize;
    89	    let mut bytes_written = 0u64;
    90	    for r in results {
    91	        bytes_written += r?;
    92	        files_written += 1;
    93	    }
    94	
    95	    Ok(SinkOutcome {
    96	        files_written,
    97	        bytes_written,
    98	    })
    99	}
   100	
   101	/// Resume protocol: overwrite a block of an existing file at the given offset.
   102	async fn write_file_block_payload(
   103	    dst_root: &Path,
   104	    canonical_dst_root: Option<&Path>,
   105	    relative_path: &str,
   106	    offset: u64,
   107	    bytes: Vec<u8>,
   108	) -> Result<SinkOutcome> {
   109	    use tokio::io::{AsyncSeekExt, AsyncWriteExt};
   110	
   111	    // R46-F3: contained resolve when canonical root is available.
   112	    let dst = match canonical_dst_root {
   113	        Some(canonical) => {
   114	            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
   115	                .with_context(|| format!("validating block-write path {:?}", relative_path))?
   116	        }
   117	        None => crate::path_safety::safe_join(dst_root, relative_path)
   118	            .with_context(|| format!("validating block-write path {:?}", relative_path))?,
   119	    };
   120	    let bytes_len = bytes.len() as u64;
   121	    // Resume blocks patch existing files at offset; we want to create
   122	    // if missing but never truncate (subsequent block records share
   123	    // the file).
   124	    let mut file = tokio::fs::OpenOptions::new()
   125	        .write(true)
   126	        .create(true)
   127	        .truncate(false)
   128	        .open(&dst)
   129	        .await
   130	        .with_context(|| format!("opening {} for block write", dst.display()))?;
   131	    file.seek(std::io::SeekFrom::Start(offset))
   132	        .await
   133	        .with_context(|| format!("seeking {} to offset {}", dst.display(), offset))?;
   134	    file.write_all(&bytes)
   135	        .await
   136	        .with_context(|| format!("writing block to {}", dst.display()))?;
   137	    Ok(SinkOutcome {
   138	        files_written: 0, // Resume blocks patch in-place; finalization counts the file.
   139	        bytes_written: bytes_len,
   140	    })
   141	}
   142	
   143	/// Resume protocol: finalize a resumed file by truncating to total_size,
   144	/// then stamp mtime + perms from the wire. The mtime stamp is what makes
   145	/// the "mtime touched, content identical" mirror case correct — block-hash
   146	/// compare sends zero blocks, but BLOCK_COMPLETE still updates the dest
   147	/// mtime to match the source.
   148	async fn write_file_block_complete(
   149	    dst_root: &Path,
   150	    canonical_dst_root: Option<&Path>,
   151	    relative_path: &str,
   152	    total_size: u64,
   153	    mtime_seconds: i64,
   154	    permissions: u32,
   155	) -> Result<SinkOutcome> {
   156	    // R46-F3: contained resolve when canonical root is available.
   157	    let dst = match canonical_dst_root {
   158	        Some(canonical) => {
   159	            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
   160	                .with_context(|| format!("validating block-complete path {:?}", relative_path))?
   161	        }
   162	        None => crate::path_safety::safe_join(dst_root, relative_path)
   163	            .with_context(|| format!("validating block-complete path {:?}", relative_path))?,
   164	    };
   165	    {
   166	        let file = tokio::fs::OpenOptions::new()
   167	            .write(true)
   168	            .open(&dst)
   169	            .await
   170	            .with_context(|| format!("opening {} for truncation", dst.display()))?;
   171	        file.set_len(total_size)
   172	            .await
   173	            .with_context(|| format!("truncating {} to {}", dst.display(), total_size))?;
   174	        file.sync_all()
   175	            .await
   176	            .with_context(|| format!("syncing {}", dst.display()))?;
   177	    }
   178	    // Stamp mtime + perms after the file handle is closed (same race
   179	    // dance as write_file_stream — see commit 946bd77).
   180	    if mtime_seconds > 0 {
   181	        let ft = FileTime::from_unix_time(mtime_seconds, 0);
   182	        if let Err(e) = filetime::set_file_mtime(&dst, ft) {
   183	            log::warn!("set mtime on {}: {}", dst.display(), e);
   184	        }
   185	    }
   186	    #[cfg(unix)]
   187	    if permissions != 0 {
   188	        use std::os::unix::fs::PermissionsExt;
   189	        if let Err(e) = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(permissions))
   190	        {
   191	            log::warn!("set permissions on {}: {}", dst.display(), e);
   192	        }
   193	    }
   194	    #[cfg(not(unix))]
   195	    let _ = permissions;
   196	    Ok(SinkOutcome {
   197	        files_written: 1,
   198	        bytes_written: 0,
   199	    })
   200	}
   201	

exec
/usr/bin/zsh -lc "sed -n '140,230p' crates/blit-core/src/remote/transfer/payload.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	            TransferTask::TarShard(paths) => {
     2	                let mut shard_headers: Vec<FileHeader> = Vec::with_capacity(paths.len());
     3	                for path in paths {
     4	                    let rel = normalize_relative_path(&path);
     5	                    if let Some(header) = header_map.remove(&rel) {
     6	                        shard_headers.push(header);
     7	                    }
     8	                }
     9	                if !shard_headers.is_empty() {
    10	                    payloads.push(TransferPayload::TarShard {
    11	                        headers: shard_headers,
    12	                    });
    13	                }
    14	            }
    15	            TransferTask::RawBundle(paths) => {
    16	                for path in paths {
    17	                    let rel = normalize_relative_path(&path);
    18	                    if let Some(header) = header_map.remove(&rel) {
    19	                        payloads.push(TransferPayload::File(header));
    20	                    }
    21	                }
    22	            }
    23	            TransferTask::Large { path } => {
    24	                let rel = normalize_relative_path(&path);
    25	                if let Some(header) = header_map.remove(&rel) {
    26	                    payloads.push(TransferPayload::File(header));
    27	                }
    28	            }
    29	        }
    30	    }
    31	
    32	    for (_, header) in header_map.into_iter() {
    33	        payloads.push(TransferPayload::File(header));
    34	    }
    35	
    36	    // Sort payloads: tar shards first (small, distribute well across streams),
    37	    // then files ascending by size. This ensures all streams stay busy with
    38	    // small work before a single large file monopolizes one stream's tail.
    39	    // Resume variants (FileBlock / FileBlockComplete) are receive-only and
    40	    // never appear here — plan_transfer_payloads is the outbound planner.
    41	    payloads.sort_by_key(|p| match p {
    42	        TransferPayload::TarShard { .. } => (0, 0),
    43	        TransferPayload::File(h) => (1, h.size),
    44	        TransferPayload::FileBlock { size, .. } => (2, *size),
    45	        TransferPayload::FileBlockComplete { .. } => (3, 0),
    46	    });
    47	
    48	    Ok(payloads)
    49	}
    50	
    51	pub fn payload_file_count(payloads: &[TransferPayload]) -> usize {
    52	    payloads
    53	        .iter()
    54	        .map(|payload| match payload {
    55	            TransferPayload::File(_) => 1,
    56	            TransferPayload::TarShard { headers } => headers.len(),
    57	            // Resume payloads patch existing files in-place — they
    58	            // don't add to the "files transferred" count.
    59	            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => 0,
    60	        })
    61	        .sum()
    62	}
    63	
    64	fn normalize_relative_path(path: &Path) -> String {
    65	    // Canonical POSIX form — see `crate::path_posix` for why a
    66	    // component-walk is correct on every platform and the historical
    67	    // string `replace('\\', "/")` was destructive on POSIX.
    68	    crate::path_posix::relative_path_to_posix(path)
    69	}
    70	
    71	pub fn prepared_payload_stream(
    72	    payloads: Vec<TransferPayload>,
    73	    source: Arc<dyn TransferSource>,
    74	    prefetch: usize,
    75	) -> impl futures::Stream<Item = Result<PreparedPayload>> {
    76	    let capacity = prefetch.max(1);
    77	    stream::iter(payloads.into_iter().map(move |payload| {
    78	        let source = source.clone();
    79	        async move { source.prepare_payload(payload).await }
    80	    }))
    81	    .buffered(capacity)
    82	}
    83	
    84	pub async fn transfer_payloads_via_control_plane(
    85	    source: Arc<dyn TransferSource>,
    86	    payloads: Vec<TransferPayload>,
    87	    tx: &mpsc::Sender<ClientPushRequest>,
    88	    finish: bool,
    89	    progress: Option<&RemoteTransferProgress>,
    90	    chunk_bytes: usize,
    91	    payload_prefetch: usize,

exec
/usr/bin/zsh -lc "sed -n '136,220p' crates/blit-core/src/path_safety.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	///
     2	/// Validates the path with `validate_wire_path`, then joins. An empty
     3	/// wire path returns `root` unchanged — this is the load-bearing
     4	/// single-file-destination case where `root` is itself the final file
     5	/// path; `PathBuf::join("")` would otherwise append a trailing
     6	/// separator that `File::create` rejects with `ENOTDIR`.
     7	pub fn safe_join(root: &Path, wire_path: &str) -> Result<PathBuf> {
     8	    let validated = validate_wire_path(wire_path)?;
     9	    if validated.as_os_str().is_empty() {
    10	        Ok(root.to_path_buf())
    11	    } else {
    12	        Ok(root.join(validated))
    13	    }
    14	}
    15	
    16	/// Resolve a wire-supplied relative path under a daemon module root
    17	/// AND verify the resolved location stays inside that root after
    18	/// symlink resolution. Returns the lexical target path (not the
    19	/// canonicalized one) so callers write to the path they expect.
    20	///
    21	/// This is the F2 chokepoint from
    22	/// `docs/reviews/codebase_review_2026-05-01.md`. `safe_join` is
    23	/// lexical — it rejects `../`, absolute paths, etc. — but does not
    24	/// follow symlinks. A module that contains `module_root/link`
    25	/// pointing at `/etc` would let a wire request for `link/passwd`
    26	/// pass `safe_join` and then have the daemon read `/etc/passwd`.
    27	/// `contained_join` closes that gap by canonicalizing the deepest
    28	/// existing ancestor of the target and confirming it stays under
    29	/// `canonical_module_root`.
    30	///
    31	/// `canonical_module_root` MUST already be the canonicalized form
    32	/// (the daemon canonicalizes module paths at load time). The check
    33	/// fails closed if either canonicalize call fails for a reason
    34	/// other than NotFound.
    35	///
    36	/// Note: this is a check-then-use API with a TOCTOU window. Between
    37	/// the canonicalize call and the actual filesystem operation, a
    38	/// symlink within the parent could in principle be replaced. The
    39	/// fully race-proof alternative would be openat(2) + O_NOFOLLOW
    40	/// per-component descent, which is significantly more code. For
    41	/// the threat model — authenticated peers and operator-trusted
    42	/// module roots — the canonicalize-and-check approach matches
    43	/// rsync's chroot module behavior and forecloses the practical
    44	/// attack vector (a module containing pre-existing escape symlinks).
    45	pub fn contained_join(canonical_module_root: &Path, wire_path: &str) -> Result<PathBuf> {
    46	    let target = safe_join(canonical_module_root, wire_path)?;
    47	
    48	    // Walk to the deepest existing ancestor and canonicalize. For a
    49	    // read of an existing file, that's the file itself; for a write
    50	    // creating a new file or directory tree, it's the deepest dir
    51	    // that already exists.
    52	    let mut probe: PathBuf = target.clone();
    53	    let canonical_ancestor = loop {
    54	        match std::fs::canonicalize(&probe) {
    55	            Ok(c) => break c,
    56	            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
    57	                if !probe.pop() {
    58	                    bail!(
    59	                        "path '{}' has no canonicalizable ancestor (root '{}' missing?)",
    60	                        target.display(),
    61	                        canonical_module_root.display()
    62	                    );
    63	                }
    64	            }
    65	            Err(e) => {
    66	                bail!(
    67	                    "canonicalize '{}' for containment check: {}",
    68	                    probe.display(),
    69	                    e
    70	                );
    71	            }
    72	        }
    73	    };
    74	
    75	    if !canonical_ancestor.starts_with(canonical_module_root) {
    76	        bail!(
    77	            "path '{}' resolves to '{}' which escapes module root '{}'",
    78	            target.display(),
    79	            canonical_ancestor.display(),
    80	            canonical_module_root.display()
    81	        );
    82	    }
    83	
    84	    Ok(target)
    85	}

exec
/usr/bin/zsh -lc "sed -n '120,220p' crates/blit-core/src/remote/transfer/source.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	/// Extracted from `RemoteTransferSource::prepare_payload` so the
     2	/// bounds are unit-testable without a `RemotePullClient` mock.
     3	/// Closes F7 of `docs/reviews/codebase_review_2026-05-01.md`.
     4	fn validate_remote_tar_shard_sizes(headers: &[FileHeader]) -> Result<()> {
     5	    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
     6	
     7	    for header in headers {
     8	        if header.size > MAX_TAR_SHARD_BYTES {
     9	            bail!(
    10	                "remote-source tar entry '{}' size {} exceeds local cap {} bytes",
    11	                header.relative_path,
    12	                header.size,
    13	                MAX_TAR_SHARD_BYTES
    14	            );
    15	        }
    16	    }
    17	    let total_bytes: u64 = headers
    18	        .iter()
    19	        .try_fold(0u64, |acc, h| acc.checked_add(h.size))
    20	        .ok_or_else(|| eyre::eyre!("remote-source tar shard size sum overflows u64"))?;
    21	    if total_bytes > MAX_TAR_SHARD_BYTES {
    22	        bail!(
    23	            "remote-source tar shard total size {} exceeds local cap {} bytes",
    24	            total_bytes,
    25	            MAX_TAR_SHARD_BYTES
    26	        );
    27	    }
    28	    Ok(())
    29	}
    30	
    31	/// Read exactly `expected_size` bytes from a remote-source stream
    32	/// into a bounded `Vec<u8>`. Closes R11-F1 of
    33	/// `docs/reviews/followup_review_2026-05-02.md`: previously the
    34	/// caller did `try_reserve_exact(size)` then `read_to_end(...)`,
    35	/// which only bounded the *reservation* — `read_to_end` would still
    36	/// grow the Vec past the bound if the remote source streamed extra
    37	/// bytes. Now the read itself is wrapped with `take(size + 1)` so
    38	/// over-reads are bounded at one byte past the declared size, and
    39	/// the post-read length check rejects both lie-large and lie-small.
    40	///
    41	/// Extracted as a free function so it's unit-testable against any
    42	/// `AsyncRead` (a real `RemotePullClient` stream isn't required).
    43	async fn read_remote_entry_bounded<R>(reader: R, expected_size: u64, label: &str) -> Result<Vec<u8>>
    44	where
    45	    R: tokio::io::AsyncRead + Unpin,
    46	{
    47	    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
    48	
    49	    // Defense-in-depth: this helper is private and current callers
    50	    // pre-validate, but reject explicitly so any future caller that
    51	    // bypasses validation can't allocate past the cap.
    52	    if expected_size > MAX_TAR_SHARD_BYTES {
    53	        bail!(
    54	            "remote-source entry '{}' size {} exceeds local cap {} bytes",
    55	            label,
    56	            expected_size,
    57	            MAX_TAR_SHARD_BYTES
    58	        );
    59	    }
    60	
    61	    let mut data: Vec<u8> = Vec::new();
    62	    data.try_reserve_exact(expected_size as usize)
    63	        .map_err(|err| {
    64	            eyre::eyre!(
    65	                "allocating buffer for remote-source entry '{}' (size {}): {}",
    66	                label,
    67	                expected_size,
    68	                err
    69	            )
    70	        })?;
    71	
    72	    // Read at most `expected_size + 1` bytes. The +1 is the over-read
    73	    // canary: if the post-read length is `expected_size + 1` we know
    74	    // the source sent more than declared. `expected_size` is bounded
    75	    // at `MAX_TAR_SHARD_BYTES` (above), so the addition can't overflow.
    76	    let read_limit = expected_size + 1;
    77	    let mut limited = reader.take(read_limit);
    78	    limited.read_to_end(&mut data).await?;
    79	
    80	    if data.len() as u64 != expected_size {
    81	        bail!(
    82	            "remote-source entry '{}' returned {} bytes; manifest declared {}",
    83	            label,
    84	            data.len(),
    85	            expected_size
    86	        );
    87	    }
    88	    Ok(data)
    89	}
    90	
    91	/// Transfer source backed by a remote daemon.
    92	///
    93	/// This is intentionally the legacy remote→remote relay primitive: the CLI
    94	/// pulls bytes from a source daemon, then pushes them to a destination daemon.
    95	/// Default remote→remote transfers now use destination-side `DelegatedPull`, so
    96	/// payload bytes flow source→destination without crossing the CLI host.
    97	/// `RemoteTransferSource` remains for the explicit `--relay-via-cli` escape
    98	/// hatch and for any future topology where the CLI really must be in the byte
    99	/// path.
   100	pub struct RemoteTransferSource {
   101	    client: RemotePullClient,

exec
/usr/bin/zsh -lc "sed -n '283,355p' crates/blit-core/src/path_safety.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	pub fn canonical_dest_root(dest_root: &Path) -> Result<PathBuf> {
     2	    let mut probe: PathBuf = dest_root.to_path_buf();
     3	    loop {
     4	        match std::fs::canonicalize(&probe) {
     5	            Ok(c) => return Ok(c),
     6	            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
     7	                if !probe.pop() {
     8	                    bail!(
     9	                        "destination root '{}' has no canonicalizable \
    10	                         ancestor — filesystem root unreachable",
    11	                        dest_root.display()
    12	                    );
    13	                }
    14	            }
    15	            Err(e) => {
    16	                bail!(
    17	                    "canonicalize '{}' for destination-root capture: {}",
    18	                    probe.display(),
    19	                    e
    20	                );
    21	            }
    22	        }
    23	    }
    24	}
    25	
    26	/// One-stop "lexically resolve a wire path against a destination
    27	/// root, then verify the result stays inside the canonical root."
    28	/// R46-F3: this is the local-receive analogue of `contained_join`
    29	/// (which is the daemon-module-root chokepoint). Pairs with
    30	/// `canonical_dest_root` for the canonical root capture.
    31	///
    32	/// Returns the lexical target path (NOT the canonicalized form)
    33	/// so the caller writes to the path they expect; the returned
    34	/// path is guaranteed to resolve under `canonical_root` post-
    35	/// symlink-following, so it's safe to write/read/delete. There
    36	/// remains a TOCTOU window between this check and the actual
    37	/// filesystem op — the threat model accepts that, the same as
    38	/// `contained_join` does on the daemon side.
    39	pub fn safe_join_contained(
    40	    canonical_root: &Path,
    41	    dest_root: &Path,
    42	    wire_path: &str,
    43	) -> Result<PathBuf> {
    44	    let target = safe_join(dest_root, wire_path)?;
    45	    verify_contained(canonical_root, &target)?;
    46	    Ok(target)
    47	}
    48	
    49	/// Detect strings that represent Windows-absolute paths regardless of
    50	/// the host platform. This catches forms that `Path::components` on
    51	/// Unix does not flag (because `C:` and `\` are normal characters
    52	/// there, and `\foo` parses as a single path component).
    53	///
    54	/// Recognized:
    55	///   - `\\?\...` and `\\.\...` (NT and DOS device paths)
    56	///   - `\\server\share\...` (UNC paths)
    57	///   - `\foo`, `\foo\bar`, bare `\` (Windows-root-shaped — caught
    58	///     here so receivers running Unix don't accept them as relative
    59	///     filenames; R1-F1 of `docs/reviews/followup_review_2026-05-02.md`)
    60	///   - `//server/share/...` (UNC-shaped with forward slashes)
    61	///   - `C:\...`, `C:/...`, or just `C:` for any letter (drive-relative)
    62	fn looks_like_windows_absolute(s: &str) -> bool {
    63	    // UNC and NT/DOS device paths.
    64	    if s.starts_with("\\\\") || s.starts_with("//") {
    65	        return true;
    66	    }
    67	    // Single leading backslash. On Unix this is a normal char as far as
    68	    // Path::components is concerned, but in the protocol context this
    69	    // is a Windows-shaped root path (e.g. `\Windows\System32`) and
    70	    // should be rejected uniformly across hosts.
    71	    if s.starts_with('\\') {
    72	        return true;
    73	    }

exec
/usr/bin/zsh -lc "rg -n \"directory.*file|file.*directory|is_dir|replace.*directory|file-dir|dir.*conflict\" crates/blit-core/tests crates/blit-core/src/remote -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/tests/transfer_session_roles.rs:55:            if path.is_dir() {
crates/blit-core/tests/mirror_planner_tests.rs:32:        is_directory: false,
crates/blit-core/tests/mirror_planner_tests.rs:58:        is_directory: false,
crates/blit-core/tests/mirror_planner_tests.rs:79:        is_directory: true,
crates/blit-core/tests/mirror_planner_tests.rs:451:            if !job.entry.is_directory && planner.should_copy_entry(job, &src_root, &dest_root) {
crates/blit-core/tests/mirror_planner_tests.rs:465:    let dest_file_entries = fs_enum::enumerate_directory_filtered(&dest_root, &mut dest_filter)?;
crates/blit-core/tests/mirror_planner_tests.rs:469:        if entry.is_directory {
crates/blit-core/src/remote/transfer/payload.rs:126:            is_directory: false,
crates/blit-core/src/remote/transfer/sink.rs:1994:        // dst_root is the file path itself, not a directory.

exec
/usr/bin/zsh -lc "sed -n '232,310p' crates/blit-core/src/remote/transfer/source.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	impl TransferSource for RemoteTransferSource {
     2	    fn scan(
     3	        &self,
     4	        _filter: Option<FileFilter>,
     5	        _unreadable_paths: Arc<Mutex<Vec<String>>>,
     6	    ) -> (
     7	        mpsc::Receiver<FileHeader>,
     8	        tokio::task::JoinHandle<Result<u64>>,
     9	    ) {
    10	        let (tx, rx) = mpsc::channel(64);
    11	        let mut client = self.client.clone();
    12	        let root = self.root.clone();
    13	        // Filter is NOT applied here — the universal `FilteredSource`
    14	        // decorator (see this module) is the single chokepoint that all
    15	        // src/dst combinations route through. Per-source filtering would
    16	        // mean every new TransferSource impl must remember to wire it up.
    17	
    18	        let handle = tokio::spawn(async move {
    19	            let headers = client.scan_remote_files(&root).await?;
    20	            let count = headers.len() as u64;
    21	            for header in headers {
    22	                if tx.send(header).await.is_err() {
    23	                    break;
    24	                }
    25	            }
    26	            Ok(count)
    27	        });
    28	
    29	        (rx, handle)
    30	    }
    31	
    32	    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
    33	        match payload {
    34	            TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
    35	            TransferPayload::TarShard { headers } => {
    36	                // F7 of docs/reviews/codebase_review_2026-05-01.md: bound
    37	                // the relay's allocation against a hostile or buggy
    38	                // remote source. This is the send-side mirror of R6-F1.
    39	                // The size validation is extracted into a testable
    40	                // helper so the bounds are pinned without needing a
    41	                // mock RemotePullClient.
    42	                validate_remote_tar_shard_sizes(&headers)?;
    43	
    44	                let mut builder = tar::Builder::new(Vec::new());
    45	                for header in headers.clone() {
    46	                    let stream = self
    47	                        .client
    48	                        .open_remote_file(Path::new(&header.relative_path))
    49	                        .await?;
    50	                    let data =
    51	                        read_remote_entry_bounded(stream, header.size, &header.relative_path)
    52	                            .await?;
    53	
    54	                    let mut tar_header = tar::Header::new_gnu();
    55	                    tar_header.set_path(&header.relative_path)?;
    56	                    tar_header.set_size(header.size);
    57	                    tar_header.set_mode(header.permissions);
    58	                    tar_header.set_mtime(header.mtime_seconds as u64);
    59	                    tar_header.set_cksum();
    60	
    61	                    builder.append_data(&mut tar_header, &header.relative_path, &data[..])?;
    62	                }
    63	                let data = builder.into_inner()?;
    64	                Ok(PreparedPayload::TarShard { headers, data })
    65	            }
    66	            // Resume payloads originate on the receive side only.
    67	            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
    68	                bail!("FileBlock payloads cannot be prepared from a remote source")
    69	            }
    70	        }
    71	    }
    72	
    73	    async fn check_availability(
    74	        &self,
    75	        headers: Vec<FileHeader>,
    76	        _unreadable_paths: Arc<Mutex<Vec<String>>>,
    77	    ) -> Result<Vec<FileHeader>> {
    78	        // Assume all remote files are available if we scanned them
    79	        Ok(headers)

exec
/usr/bin/zsh -lc "sed -n '310,350p' crates/blit-core/src/remote/transfer/source.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	        Ok(headers)
     2	    }
     3	
     4	    async fn open_file(
     5	        &self,
     6	        header: &FileHeader,
     7	    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
     8	        let stream = self
     9	            .client
    10	            .open_remote_file(Path::new(&header.relative_path))
    11	            .await?;
    12	        Ok(Box::new(stream))
    13	    }
    14	
    15	    fn root(&self) -> &Path {
    16	        &self.root
    17	    }
    18	}
    19	
    20	/// Decorator that applies a `FileFilter` uniformly to any inner
    21	/// `TransferSource`'s scan output. This is the SINGLE filter chokepoint
    22	/// for every src/dst combination (local→local, push, pull, remote→remote);
    23	/// per-source filtering is intentionally avoided so that future source
    24	/// implementations get filtering for free and parity is enforced.
    25	///
    26	/// The orchestrator/CLI wraps the real source once with this decorator
    27	/// before handing it to the pipeline. All other methods delegate to the
    28	/// inner source unchanged.
    29	pub struct FilteredSource {
    30	    inner: Arc<dyn TransferSource>,
    31	    filter: FileFilter,
    32	}
    33	
    34	impl FilteredSource {
    35	    pub fn new(inner: Arc<dyn TransferSource>, filter: FileFilter) -> Self {
    36	        Self { inner, filter }
    37	    }
    38	}
    39	
    40	#[async_trait]
    41	impl TransferSource for FilteredSource {

exec
/usr/bin/zsh -lc 'rg -n "pub fn build_plan|fn build_plan|struct FileEntry" crates/blit-core/src/transfer_plan.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
50:pub fn build_plan(

exec
/usr/bin/zsh -lc "sed -n '1,130p' crates/blit-core/src/transfer_plan.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::collections::HashMap;
     2	use std::path::{Path, PathBuf};
     3	
     4	/// Adaptive transfer task classification shared across push, pull, and local engines.
     5	#[derive(Clone, Debug)]
     6	pub enum TransferTask {
     7	    TarShard(Vec<PathBuf>),
     8	    /// Bundle of medium files to send back-to-back in a single worker turn.
     9	    RawBundle(Vec<PathBuf>),
    10	    /// Large single file; delta/range logic decides stripes internally.
    11	    Large {
    12	        path: PathBuf,
    13	    },
    14	}
    15	
    16	/// Planner tuning options shared across engines.
    17	#[derive(Clone, Copy, Debug)]
    18	pub struct PlanOptions {
    19	    pub force_tar: bool,
    20	    pub small_target: Option<u64>,
    21	    pub small_count_target: Option<usize>,
    22	    pub medium_target: Option<u64>,
    23	}
    24	
    25	impl PlanOptions {
    26	    pub fn new() -> Self {
    27	        Self {
    28	            force_tar: false,
    29	            small_target: None,
    30	            small_count_target: None,
    31	            medium_target: None,
    32	        }
    33	    }
    34	}
    35	
    36	impl Default for PlanOptions {
    37	    fn default() -> Self {
    38	        Self::new()
    39	    }
    40	}
    41	
    42	/// Build an adaptive transfer task queue from enumerated file entries.
    43	///
    44	/// The heuristics mirror the original `net_async::client::build_plan` logic so that
    45	/// every mode (push, pull, local) can share the same task ordering. Wire
    46	/// chunk sizing is NOT planned here — it is owned by the live
    47	/// [`crate::engine::TransferDial`] (w2-2: this module's static 16/32 MiB
    48	/// chunk ladder was dead policy — every remote path overrode it from the
    49	/// dial and no consumer read the planned value).
    50	pub fn build_plan(
    51	    files: &[crate::fs_enum::FileEntry],
    52	    rootsrc: &Path,
    53	    options: PlanOptions,
    54	) -> Vec<TransferTask> {
    55	    let mut size_map: HashMap<PathBuf, u64> = HashMap::new();
    56	    let mut small: Vec<PathBuf> = Vec::new();
    57	    let mut medium: Vec<(PathBuf, u64)> = Vec::new();
    58	    let mut total_medium_bytes: u64 = 0;
    59	    let mut large_files: Vec<TransferTask> = Vec::new();
    60	    let mut total_bytes: u128 = 0;
    61	    for e in files {
    62	        if e.is_directory {
    63	            continue;
    64	        }
    65	        let rel = e
    66	            .path
    67	            .strip_prefix(rootsrc)
    68	            .unwrap_or(&e.path)
    69	            .to_path_buf();
    70	        size_map.insert(rel.clone(), e.size);
    71	        total_bytes += e.size as u128;
    72	        if e.size < 1_048_576 {
    73	            // <1MB
    74	            small.push(rel);
    75	        } else if e.size < 256 * 1_048_576 {
    76	            // <256MB
    77	            medium.push((rel, e.size));
    78	            total_medium_bytes = total_medium_bytes.saturating_add(e.size);
    79	        } else {
    80	            // Large: schedule as single large-file task; range/delta decided when sending
    81	            large_files.push(TransferTask::Large { path: rel.clone() });
    82	        }
    83	    }
    84	    // Shard small files into larger tars for multi-GB workloads
    85	    small.sort_by_key(|p| p.as_os_str().len());
    86	
    87	    let mut small_tasks: Vec<TransferTask> = Vec::new();
    88	    let small_count = small.len();
    89	    let total_small_bytes: u64 = small.iter().fold(0u64, |acc, p| {
    90	        acc.saturating_add(*size_map.get(p).unwrap_or(&(64 * 1024)))
    91	    });
    92	    let avg_small_size = if small_count == 0 {
    93	        0
    94	    } else {
    95	        total_small_bytes / small_count as u64
    96	    };
    97	
    98	    // Tar shards only make sense for 2+ files (batching) — a single file
    99	    // gains nothing from tar wrapping and breaks the empty-path case
   100	    // produced by enumerating a file root directly.
   101	    let use_tar = if options.force_tar {
   102	        small_count >= 1
   103	    } else if small_count < 2 {
   104	        false
   105	    } else {
   106	        small_count >= 32 || avg_small_size <= 128 * 1024
   107	    };
   108	
   109	    if use_tar {
   110	        let mut target_shard = options.small_target.unwrap_or(8 * 1024 * 1024);
   111	        if total_small_bytes >= 768 * 1024 * 1024 {
   112	            target_shard = target_shard.max(64 * 1024 * 1024);
   113	        } else if total_small_bytes >= 256 * 1024 * 1024 {
   114	            target_shard = target_shard.max(32 * 1024 * 1024);
   115	        } else {
   116	            target_shard = target_shard.max(4 * 1024 * 1024);
   117	        }
   118	        let mut count_target = options
   119	            .small_count_target
   120	            .unwrap_or(if small_count >= 2048 {
   121	                2048
   122	            } else if small_count >= 1024 {
   123	                1024
   124	            } else {
   125	                256
   126	            });
   127	        count_target = count_target.clamp(128, 4096);
   128	
   129	        let mut cur: Vec<PathBuf> = Vec::new();
   130	        let mut cur_bytes: u64 = 0;

exec
/usr/bin/zsh -lc "sed -n '1,80p' crates/blit-core/src/remote/transfer/payload.rs | nl -ba" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::collections::HashMap;
     2	use std::path::{Path, PathBuf};
     3	
     4	use eyre::{bail, eyre, Context, Result};
     5	use futures::{stream, StreamExt};
     6	use tokio::io::AsyncReadExt;
     7	use tokio::sync::mpsc;
     8	use tokio::task;
     9	
    10	use crate::fs_enum::FileEntry;
    11	use crate::generated::client_push_request::Payload as ClientPayload;
    12	use crate::generated::{
    13	    ClientPushRequest, FileData, FileHeader, TarShardChunk, TarShardComplete, TarShardHeader,
    14	    UploadComplete,
    15	};
    16	use crate::transfer_plan::{self, PlanOptions, TransferTask};
    17	use tar::{Builder, EntryType, Header};
    18	
    19	use super::data_plane::CONTROL_PLANE_CHUNK_SIZE;
    20	use super::progress::RemoteTransferProgress;
    21	use crate::remote::transfer::source::TransferSource;
    22	use std::sync::Arc;
    23	
    24	#[derive(Debug, Clone)]
    25	pub enum TransferPayload {
    26	    File(FileHeader),
    27	    TarShard {
    28	        headers: Vec<FileHeader>,
    29	    },
    30	    /// Resume protocol: overwrite a block of an existing file.
    31	    FileBlock {
    32	        relative_path: String,
    33	        offset: u64,
    34	        size: u64,
    35	    },
    36	    /// Resume protocol: finalize a resumed file (truncate to total_size).
    37	    FileBlockComplete {
    38	        relative_path: String,
    39	        total_size: u64,
    40	    },
    41	}
    42	
    43	pub async fn prepare_payload(
    44	    payload: TransferPayload,
    45	    source_root: PathBuf,
    46	) -> Result<PreparedPayload> {
    47	    match payload {
    48	        TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
    49	        TransferPayload::TarShard { headers } => {
    50	            let headers_clone = headers.clone();
    51	            let source_root_clone = source_root.clone();
    52	            let data =
    53	                task::spawn_blocking(move || build_tar_shard(&source_root_clone, &headers_clone))
    54	                    .await
    55	                    .map_err(|err| eyre!("tar shard worker failed: {err}"))??;
    56	            Ok(PreparedPayload::TarShard { headers, data })
    57	        }
    58	        // Resume payloads can only originate on the receive side (parsed
    59	        // off the wire by DataPlaneSource); the file-system source never
    60	        // produces them.
    61	        TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
    62	            bail!("FileBlock payloads cannot be prepared from a filesystem source")
    63	        }
    64	    }
    65	}
    66	
    67	/// A payload ready for a sink to consume.
    68	///
    69	/// `File` and `TarShard` are used by both outbound and inbound paths
    70	/// (they carry self-contained data). The receive pipeline additionally
    71	/// uses `FileBlock` / `FileBlockComplete` for the resume protocol.
    72	///
    73	/// Streaming file bytes (4 GiB pulls, no point buffering) are NOT a
    74	/// payload variant — they go through `TransferSink::write_file_stream`
    75	/// directly so the receiver can hand the sink a borrowed reader without
    76	/// fighting `'static` trait-object lifetimes.
    77	#[derive(Debug)]
    78	pub enum PreparedPayload {
    79	    /// Whole file, source has it accessible by `src_root.join(relative_path)`.
    80	    /// The sink performs a (zero-copy when possible) local copy.

codex
I’ve found one concrete protocol-state issue so far: the source side accepts `NeedComplete` without knowing whether its manifest is complete. I’m doing one more pass for any similarly concrete acceptance-gap before I write the verdict.
exec
/usr/bin/zsh -lc 'rg -n "otp-1|otp-2|otp-3|otp-4|slice" docs/plan/ONE_TRANSFER_PATH.md' in /home/michael/dev/Blit
 succeeded in 0ms:
56:  code coexists in-tree during the migration slices solely so each
57:  slice lands green — that is migration scaffolding, not wire
68:  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
69:  after cutover, not one of this plan's slices — see the Design note
94:  and otp-1's contract names it explicitly.
97:  selection — is a reviewed doc+proto slice **before** any behavior
99:- Every slice through the codex loop (D-2026-07-04-1); tree green
100:  after every slice; transitional coexistence of old+new paths is
101:  scaffolding only — the plan is not Shipped until the deletion slice
127:      slice's finding doc.
175:   follow-on slice set after cutover. Strategy selection reads
209:(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
227:mitigated by strangler slices with the tree green throughout and a
228:non-optional deletion slice. Per-cell regression risk on today's
230:baseline parity pins per slice. Wire break — lockstep upgrade,
237:One coherent, testable change per slice — sized for the `.review/`
238:loop. Tree green after every slice; old paths keep working until
241:1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
255:   pre-cutover code and die with their messages at otp-10, per the
258:2. **otp-2 symmetric baseline (harness + rig, no production code)**:
264:3. **otp-3 TransferSession core (blit-core)**: role-parameterized
268:4. **otp-4 daemon serves `Transfer`, client initiates as SOURCE**
282:10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
288:11. **otp-11 local transfers** ride the in-process transport; the
290:12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
294:13. **otp-13 verdict**: acceptance checklist walked with the owner;
303:  their slices through the codex loop. — owner

exec
/usr/bin/zsh -lc "sed -n '220,420p' docs/plan/ONE_TRANSFER_PATH.md" in /home/michael/dev/Blit
 succeeded in 0ms:
`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
`blit-tui` (progress/jobs consume the same events).

**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
mitigated by strangler slices with the tree green throughout and a
non-optional deletion slice. Per-cell regression risk on today's
faster direction — mitigated by the converge-up constraint and
baseline parity pins per slice. Wire break — lockstep upgrade,
owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
Progress/jobs/TUI integration churn — the session emits the existing
event contract (w6-1) at the same boundaries.

## Slices

One coherent, testable change per slice — sized for the `.review/`
loop. Tree green after every slice; old paths keep working until
otp-9 deletes them.

1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
   `Transfer` RPC and message set — roles, phases, field numbers,
   the **strict same-build handshake** (exact protocol/build identity
   exchanged at session open; any mismatch is refused with a clear
   error — D-2026-07-05-2; pinned by test when the session lands),
   the receiver capacity profile + bounded-unilateral dial contract
   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
   that exists), transport selection, resume phase ordering (the
   RELIABLE exception above), mirror phase, error/cancel semantics.
   No feature-capability bits: same build implies same features.
   The new proto text must carry NO version-tolerance semantics; the
   capacity profile's absent/0 fields mean "unknown hardware value"
   only, never "old peer" (today's proto comments frame some of that
   contract as old-peer fallback — those comment blocks describe live
   pre-cutover code and die with their messages at otp-10, per the
   D-2026-07-05-2 review adjudication). Codex-reviewed before any
   code consumes it.
2. **otp-2 symmetric baseline (harness + rig, no production code)**:
   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
   cells, cold caches, tmpfs rows re-labeled wire-reference only —
   and record the OLD paths' per-cell, per-direction baseline on the
   rig. This is the converge-up reference the acceptance criteria
   compare against (codex F4).
3. **otp-3 TransferSession core (blit-core)**: role-parameterized
   state machine over the existing engine with an in-process
   transport; unit/e2e tests run BOTH role assignments over the same
   fixtures — the invariance property enters the test suite here.
4. **otp-4 daemon serves `Transfer`, client initiates as SOURCE**
   (remote push-equivalent rides the session); A/B parity pins vs
   old push (byte-identical trees, summary parity, sf-2 pin ported).
5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-
   equivalent) — the same code with roles flipped; the parity suite
   reruns with no per-direction test code.
6. **otp-6 mirror + filters** on the session (one delete rule).
7. **otp-7 resume** block phase (ordering + stale-partial pins per
   the Design's RELIABLE exception).
8. **otp-8 fallback byte-carrier** (control-stream frames) as the
   session's alternate transport.
9. **otp-9 delegated transfer** = daemon-initiated session; bespoke
   delegated-pull driver retired behind the existing gate;
   `DelegatedPull` RPC reduced to trigger + progress relay.
10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
    operation through the session; `Push`/`PullSync` and all four
    drivers deleted from the tree and the proto, no bridge
    (D-2026-07-05-2); ported-test accounting proves count never
    dropped. Deletion proof recorded, incl. the DelegatedPull
    no-payload-bytes assertion.
11. **otp-11 local transfers** ride the in-process transport; the
    separate local orchestration is deleted; local perf pins hold.
12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
    on the unified path — initiator/verb invariance A/B within noise
    AND every cell ≤ the better old direction + noise; committed as
    this plan's acceptance evidence.
13. **otp-13 verdict**: acceptance checklist walked with the owner;
    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
    against the unified baseline — owner call at that point.

## Open questions

- None requiring owner input now — scope, wire, and process were
  delegated (Directive section). Slice-level unknowns (exact proto
  shapes, resume edge semantics, TUI event wiring) are settled inside
  their slices through the codex loop. — owner

exec
/usr/bin/zsh -lc 'rg -n "NeedComplete|ManifestComplete|protocol|bounded|byte-identical|byte-accounting|same-build|build" docs/TRANSFER_SESSION.md docs/plan/ONE_TRANSFER_PATH.md .review/findings/otp-3-transfer-session-core.md' in /home/michael/dev/Blit
 succeeded in 0ms:
docs/plan/ONE_TRANSFER_PATH.md:9:build only)** — annotated in REV4 §Constraints
docs/plan/ONE_TRANSFER_PATH.md:50:  rule: "backward compatibility is NOT a consideration... same build
docs/plan/ONE_TRANSFER_PATH.md:52:  client talks only to a blit-daemon from the same build; the session
docs/plan/ONE_TRANSFER_PATH.md:81:- REV4 invariants carry: byte-identical results, StallGuard,
docs/plan/ONE_TRANSFER_PATH.md:82:  cancellation, byte-accounting. Existing pins are ported (not
docs/plan/ONE_TRANSFER_PATH.md:88:- **The bounded-unilateral dial contract carries unchanged**
docs/plan/ONE_TRANSFER_PATH.md:90:  dial, bounded by the byte RECEIVER's advertised capacity profile
docs/plan/ONE_TRANSFER_PATH.md:183:8. Summary/byte-accounting: one record shape.
docs/plan/ONE_TRANSFER_PATH.md:243:   the **strict same-build handshake** (exact protocol/build identity
docs/plan/ONE_TRANSFER_PATH.md:246:   the receiver capacity profile + bounded-unilateral dial contract
docs/plan/ONE_TRANSFER_PATH.md:250:   No feature-capability bits: same build implies same features.
docs/plan/ONE_TRANSFER_PATH.md:270:   old push (byte-identical trees, summary parity, sf-2 pin ported).
docs/TRANSFER_SESSION.md:7:(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)
docs/TRANSFER_SESSION.md:22:2. **Same build only (D-2026-07-05-2).** The first frame each way is
docs/TRANSFER_SESSION.md:23:   `SessionHello{build_id, contract_version}`. Both ends compare for
docs/TRANSFER_SESSION.md:26:   fields, no feature-capability bits — same build implies same
docs/TRANSFER_SESSION.md:27:   features. `build_id` = `<crate version>+<git commit hash>[.dirty]`
docs/TRANSFER_SESSION.md:45:   is. The byte SENDER (SOURCE) owns the live dial bounded by that
docs/TRANSFER_SESSION.md:61:  |     both verify build_id exact match; mismatch => SessionError + close
docs/TRANSFER_SESSION.md:71:  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
docs/TRANSFER_SESSION.md:72:  |  DEST streams:    NeedBatch* ... NeedComplete                  |
docs/TRANSFER_SESSION.md:95:- `NeedComplete` is DESTINATION's promise that no further need
docs/TRANSFER_SESSION.md:97:  It may be sent only after BOTH: the source's `ManifestComplete`
docs/TRANSFER_SESSION.md:103:  each end holds only bounded internal queues (the engine's existing
docs/TRANSFER_SESSION.md:105:  Nothing in the contract requires unbounded buffering of the peer's
docs/TRANSFER_SESSION.md:123:| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
docs/TRANSFER_SESSION.md:125:| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
docs/TRANSFER_SESSION.md:141:`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
docs/TRANSFER_SESSION.md:147:`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
docs/TRANSFER_SESSION.md:150:Deliberately absent: `PeerCapabilities` (same build = same
docs/TRANSFER_SESSION.md:194:  only AFTER the source's `ManifestComplete` — this per-transport
docs/TRANSFER_SESSION.md:207:- `SessionError{code, message}` codes (plus both build ids on
docs/TRANSFER_SESSION.md:218:- StallGuard, byte-accounting, and progress events (w6-1 contract)
.review/findings/otp-3-transfer-session-core.md:17:identical summary, and byte-identical destination tree.
.review/findings/otp-3-transfer-session-core.md:23:  `in_process_pair()` on bounded mpsc (64 frames/direction).
.review/findings/otp-3-transfer-session-core.md:25:  - `session_build_id()` = `CARGO_PKG_VERSION+BLIT_GIT_SHA[.dirty]`
.review/findings/otp-3-transfer-session-core.md:26:    (build.rs emits the sha; rerun-if-changed on `.git/HEAD` +
.review/findings/otp-3-transfer-session-core.md:27:    `.git/refs`; dirty flag sampled at build-script time, best-effort
.review/findings/otp-3-transfer-session-core.md:39:    half (deadlock-freedom: the transport is bounded both ways, so a
.review/findings/otp-3-transfer-session-core.md:47:    exactly `header.size`) and tar records (existing tar builder via
.review/findings/otp-3-transfer-session-core.md:48:    `prepare_payload`), only after `ManifestComplete` (in-stream
.review/findings/otp-3-transfer-session-core.md:49:    carrier rule). `SourceDone` only after `NeedComplete` + queue
.review/findings/otp-3-transfer-session-core.md:55:    mid-manifest; `NeedComplete` only after ManifestComplete + all
.review/findings/otp-3-transfer-session-core.md:57:    `FsTransferSink::write_file_stream` fed by a bounded
.review/findings/otp-3-transfer-session-core.md:62:    ManifestComplete, record interleave/overrun/short-complete,
.review/findings/otp-3-transfer-session-core.md:68:  - Faults are `SessionFault` (wire code + message + both build ids +
.review/findings/otp-3-transfer-session-core.md:97:- `crates/blit-core/build.rs` (BLIT_GIT_SHA emission)
.review/findings/otp-3-transfer-session-core.md:111:  3 MiB file, empty file, spaced/nested names) byte-identical under
.review/findings/otp-3-transfer-session-core.md:116:  0/0 summary; mtime preservation on streamed files; build-id
.review/findings/otp-3-transfer-session-core.md:120:  violations fail fast (payload record before ManifestComplete, need
.review/findings/otp-3-transfer-session-core.md:122:  after ManifestComplete) with the error frame observed on the wire.
.review/findings/otp-3-transfer-session-core.md:125:- `mod.rs` (2): build-id shape; fault wire round-trip (perspective
.review/findings/otp-3-transfer-session-core.md:157:  flush per 128-entry diff chunk and at ManifestComplete. Cadence
.review/findings/otp-3-transfer-session-core.md:162:- `require_complete_scan` + `ManifestComplete.scan_complete` travel

exec
/usr/bin/zsh -lc 'git diff ef9ffa1''^ ef9ffa1 -- crates/blit-core/src/transfer_session/mod.rs --check' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
new file mode 100644
index 0000000..7345b6a
--- /dev/null
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -0,0 +1,1262 @@
+//! Unified transfer session — the ONE block of transfer code
+//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
+//!
+//! A transfer has a SOURCE role and a DESTINATION role; which end
+//! initiated and which CLI verb was used select roles, never code.
+//! Both roles run the drivers below over a [`transport::FrameTransport`];
+//! the wire contract they implement — phases, frame table, record
+//! grammar, error semantics — is `docs/TRANSFER_SESSION.md` (otp-1).
+//!
+//! otp-3 scope: the role-parameterized state machine over the existing
+//! engine with the in-process transport and the in-stream byte
+//! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
+//! progress wiring land at otp-4; mirror otp-6; resume otp-7;
+//! delegated otp-9 (see the slice list in the plan).
+
+pub mod transport;
+
+use std::collections::{HashMap, HashSet};
+use std::fmt;
+use std::path::{Path, PathBuf};
+use std::sync::{Arc, Mutex as StdMutex};
+
+use eyre::Result;
+use tokio::io::{AsyncReadExt, AsyncWriteExt};
+use tokio::sync::mpsc;
+
+use crate::generated::transfer_frame::Frame;
+use crate::generated::{
+    session_error, ComparisonMode, FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch,
+    NeedComplete, NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
+    TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
+};
+use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
+use crate::remote::transfer::diff_planner;
+use crate::remote::transfer::payload::PreparedPayload;
+use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
+use crate::remote::transfer::source::TransferSource;
+use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
+use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
+use crate::transfer_plan::PlanOptions;
+use transport::{FrameRx, FrameTransport, FrameTx};
+
+/// Belt-and-braces wire-shape version, bumped on any change to the
+/// frame set or grammar. Exchanged (and exact-matched) in
+/// `SessionHello` alongside the build id (D-2026-07-05-2).
+pub const CONTRACT_VERSION: u32 = 1;
+
+/// Payload chunk size on the in-stream carrier. Same unit the gRPC
+/// control plane uses today; the data plane (otp-4) has its own.
+const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;
+
+/// Manifest entries buffered per destination diff batch. Mirrors the
+/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
+/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
+/// blocking pool instead of inline per entry.
+const DEST_DIFF_CHUNK: usize = 128;
+
+/// Buffer of the in-memory pipe that feeds wire file-record bytes
+/// into `FsTransferSink::write_file_stream`. Bounds destination-side
+/// buffering per file record.
+const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
+
+/// This build's session identity: `<crate version>+<git sha>[.dirty]`
+/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
+/// "unknown" when git was unavailable at compile time.
+pub fn session_build_id() -> &'static str {
+    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
+}
+
+/// The identity this end presents in `SessionHello`. Defaults to the
+/// real compile-time identity; tests inject mismatches.
+#[derive(Debug, Clone)]
+pub struct HelloConfig {
+    pub build_id: String,
+    pub contract_version: u32,
+}
+
+impl Default for HelloConfig {
+    fn default() -> Self {
+        Self {
+            build_id: session_build_id().to_string(),
+            contract_version: CONTRACT_VERSION,
+        }
+    }
+}
+
+/// Which handshake part this end plays. Orthogonal to role: all four
+/// initiator/role combinations run the same state machine (contract
+/// §Invariants 3).
+pub enum SessionEndpoint {
+    /// This end opened the transport; it sends `SessionOpen`.
+    /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
+    Initiator { open: Box<SessionOpen> },
+    /// This end answers `SessionOpen` with `SessionAccept`. Daemon
+    /// module/path/read-only validation attaches here at otp-4.
+    Responder,
+}
+
+impl SessionEndpoint {
+    /// Convenience constructor so callers don't spell the `Box`.
+    pub fn initiator(open: SessionOpen) -> Self {
+        SessionEndpoint::Initiator {
+            open: Box::new(open),
+        }
+    }
+}
+
+pub struct SourceSessionConfig {
+    pub hello: HelloConfig,
+    pub endpoint: SessionEndpoint,
+    /// Engine planner knobs (tar/large/raw thresholds). Local to the
+    /// source end — strategy selection is planner-owned and never
+    /// crosses the wire (contract §Transport selection).
+    pub plan_options: PlanOptions,
+}
+
+pub struct DestinationSessionConfig {
+    pub hello: HelloConfig,
+    pub endpoint: SessionEndpoint,
+}
+
+/// A session-terminating fault: either end refusing, aborting, or
+/// catching the peer in a protocol violation. Carried as the error
+/// payload of the drivers' `eyre::Report`s — downcast to inspect the
+/// wire code.
+#[derive(Debug, Clone)]
+pub struct SessionFault {
+    pub code: session_error::Code,
+    pub message: String,
+    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
+    /// which end is stale (contract §Errors).
+    pub local_build_id: String,
+    pub peer_build_id: String,
+    /// True when the peer already knows about this fault — it sent
+    /// the `SessionError` frame itself, or this end already emitted
+    /// one. Drivers must not send another.
+    pub peer_notified: bool,
+}
+
+impl SessionFault {
+    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
+        Self {
+            code,
+            message: message.into(),
+            local_build_id: String::new(),
+            peer_build_id: String::new(),
+            peer_notified: false,
+        }
+    }
+
+    fn protocol_violation(message: impl Into<String>) -> Self {
+        Self::new(session_error::Code::ProtocolViolation, message)
+    }
+
+    fn internal(message: impl Into<String>) -> Self {
+        Self::new(session_error::Code::Internal, message)
+    }
+
+    fn from_wire(err: SessionError) -> Self {
+        Self {
+            code: session_error::Code::try_from(err.code)
+                .unwrap_or(session_error::Code::SessionErrorUnspecified),
+            message: err.message,
+            // The peer reports its view: its "local" is our peer.
+            local_build_id: err.peer_build_id,
+            peer_build_id: err.local_build_id,
+            peer_notified: true,
+        }
+    }
+
+    fn to_wire(&self) -> SessionError {
+        SessionError {
+            code: self.code as i32,
+            message: self.message.clone(),
+            local_build_id: self.local_build_id.clone(),
+            peer_build_id: self.peer_build_id.clone(),
+        }
+    }
+}
+
+impl fmt::Display for SessionFault {
+    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
+        write!(f, "session {}: {}", self.code.as_str_name(), self.message)
+    }
+}
+
+impl std::error::Error for SessionFault {}
+
+/// Downcast a driver-internal error back to its fault, wrapping
+/// non-fault failures (fs errors, planner errors, transport failures)
+/// as INTERNAL — an end that aborts says why before closing.
+fn fault_from_report(report: eyre::Report) -> SessionFault {
+    match report.downcast::<SessionFault>() {
+        Ok(fault) => fault,
+        Err(other) => SessionFault::internal(format!("{other:#}")),
+    }
+}
+
+fn frame(f: Frame) -> TransferFrame {
+    TransferFrame { frame: Some(f) }
+}
+
+fn error_frame(fault: &SessionFault) -> TransferFrame {
+    frame(Frame::Error(fault.to_wire()))
+}
+
+/// Short frame identifier for protocol-violation messages.
+fn frame_name(f: &Option<Frame>) -> &'static str {
+    match f {
+        Some(Frame::Hello(_)) => "SessionHello",
+        Some(Frame::Open(_)) => "SessionOpen",
+        Some(Frame::Accept(_)) => "SessionAccept",
+        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
+        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
+        Some(Frame::NeedBatch(_)) => "NeedBatch",
+        Some(Frame::NeedComplete(_)) => "NeedComplete",
+        Some(Frame::BlockHashes(_)) => "BlockHashList",
+        Some(Frame::FileBegin(_)) => "FileBegin",
+        Some(Frame::FileData(_)) => "FileData",
+        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
+        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
+        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
+        Some(Frame::Block(_)) => "BlockTransfer",
+        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
+        Some(Frame::Resize(_)) => "DataPlaneResize",
+        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
+        Some(Frame::SourceDone(_)) => "SourceDone",
+        Some(Frame::Summary(_)) => "TransferSummary",
+        Some(Frame::Error(_)) => "SessionError",
+        None => "empty frame",
+    }
+}
+
+fn complement(role: TransferRole) -> TransferRole {
+    match role {
+        TransferRole::Source => TransferRole::Destination,
+        TransferRole::Destination => TransferRole::Source,
+        TransferRole::Unspecified => TransferRole::Unspecified,
+    }
+}
+
+/// Per-role capability check of the operation a `SessionOpen`
+/// describes. otp-3 refuses what later slices implement rather than
+/// silently ignoring it (fail-fast; contract §Errors).
+type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
+
+fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
+    if open.resume.as_ref().is_some_and(|r| r.enabled) {
+        return Err(SessionFault::internal(
+            "resume is not implemented on the unified session yet (otp-7)",
+        ));
+    }
+    if open
+        .filter
+        .as_ref()
+        .is_some_and(|f| *f != FilterSpec::default())
+    {
+        return Err(SessionFault::internal(
+            "filters are not implemented on the unified session yet (otp-6)",
+        ));
+    }
+    Ok(())
+}
+
+fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
+    if open.mirror_enabled {
+        return Err(SessionFault::internal(
+            "mirror is not implemented on the unified session yet (otp-6)",
+        ));
+    }
+    if open.resume.as_ref().is_some_and(|r| r.enabled) {
+        return Err(SessionFault::internal(
+            "resume is not implemented on the unified session yet (otp-7)",
+        ));
+    }
+    Ok(())
+}
+
+/// Outcome of the HELLO + OPEN phases.
+struct Negotiated {
+    open: SessionOpen,
+    #[allow(dead_code)] // capacity/grant consumed from otp-4 on
+    accept: SessionAccept,
+}
+
+/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
+/// scoping requirement). Sends the refusal `SessionError` itself when
+/// it detects the fault locally; returned faults are `peer_notified`.
+async fn establish(
+    transport: &mut FrameTransport,
+    hello: &HelloConfig,
+    endpoint: &SessionEndpoint,
+    local_role: TransferRole,
+    validate_open: &OpenValidator,
+) -> Result<Negotiated> {
+    // HELLO both ways, exact match (D-2026-07-05-2). First frame each
+    // direction; no ordering between the two directions.
+    transport
+        .send(frame(Frame::Hello(SessionHello {
+            build_id: hello.build_id.clone(),
+            contract_version: hello.contract_version,
+        })))
+        .await?;
+
+    let peer_hello = match expect_frame(transport).await? {
+        Frame::Hello(h) => h,
+        other => {
+            return Err(notify_and_wrap(
+                transport,
+                SessionFault::protocol_violation(format!(
+                    "expected SessionHello, got {}",
+                    frame_name(&Some(other))
+                )),
+            )
+            .await)
+        }
+    };
+
+    if peer_hello.build_id != hello.build_id
+        || peer_hello.contract_version != hello.contract_version
+    {
+        let fault = SessionFault {
+            code: session_error::Code::BuildMismatch,
+            message: format!(
+                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
+                hello.build_id, hello.contract_version,
+                peer_hello.build_id, peer_hello.contract_version,
+            ),
+            local_build_id: hello.build_id.clone(),
+            peer_build_id: peer_hello.build_id.clone(),
+            peer_notified: false,
+        };
+        return Err(notify_and_wrap(transport, fault).await);
+    }
+
+    match endpoint {
+        SessionEndpoint::Initiator { open } => {
+            let open = open.as_ref().clone();
+            transport.send(frame(Frame::Open(open.clone()))).await?;
+            let accept = match expect_frame(transport).await? {
+                Frame::Accept(a) => a,
+                other => {
+                    return Err(notify_and_wrap(
+                        transport,
+                        SessionFault::protocol_violation(format!(
+                            "expected SessionAccept, got {}",
+                            frame_name(&Some(other))
+                        )),
+                    )
+                    .await)
+                }
+            };
+            Ok(Negotiated { open, accept })
+        }
+        SessionEndpoint::Responder => {
+            let open = match expect_frame(transport).await? {
+                Frame::Open(o) => o,
+                other => {
+                    return Err(notify_and_wrap(
+                        transport,
+                        SessionFault::protocol_violation(format!(
+                            "expected SessionOpen, got {}",
+                            frame_name(&Some(other))
+                        )),
+                    )
+                    .await)
+                }
+            };
+            // The initiator declares ITS role; this responder end must
+            // hold the complement.
+            let declared =
+                TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
+            if declared != complement(local_role) {
+                return Err(notify_and_wrap(
+                    transport,
+                    SessionFault::protocol_violation(format!(
+                        "initiator declared role {} but this responder is {}",
+                        declared.as_str_name(),
+                        local_role.as_str_name()
+                    )),
+                )
+                .await);
+            }
+            if let Err(fault) = validate_open(&open) {
+                // Refusal is a SessionError instead of SessionAccept,
+                // never a silent close (contract §Phase state machine).
+                return Err(notify_and_wrap(transport, fault).await);
+            }
+            let accept = SessionAccept {
+                // The byte RECEIVER advertises capacity at session
+                // open (D-2026-06-20-1/-2); consumed by the dial when
+                // the data plane lands (otp-4).
+                receiver_capacity: if local_role == TransferRole::Destination {
+                    Some(crate::engine::local_receiver_capacity())
+                } else {
+                    None
+                },
+                // No grant = in-stream byte carrier, otp-3's only one.
+                data_plane: None,
+            };
+            transport.send(frame(Frame::Accept(accept.clone()))).await?;
+            Ok(Negotiated { open, accept })
+        }
+    }
+}
+
+/// Receive one frame during establish; peer errors and closes become
+/// terminal faults.
+async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
+    match transport.recv().await? {
+        Some(TransferFrame {
+            frame: Some(Frame::Error(err)),
+        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
+        Some(TransferFrame { frame: Some(f) }) => Ok(f),
+        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
+            SessionFault::protocol_violation("frame with empty oneof"),
+        )),
+        None => Err(eyre::Report::new(SessionFault::internal(
+            "peer closed during session establish",
+        ))),
+    }
+}
+
+/// Send the fault to the peer (best effort), mark it notified, and
+/// wrap it for return.
+async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
+    let _ = transport.send(error_frame(&fault)).await;
+    fault.peer_notified = true;
+    eyre::Report::new(fault)
+}
+
+// ---------------------------------------------------------------------------
+// SOURCE driver
+// ---------------------------------------------------------------------------
+
+/// Events the source's receive half forwards to its send half. The
+/// channel is unbounded but bounded by construction: every `Need`
+/// consumes a distinct sent-manifest entry (unknown or repeated paths
+/// fault the session), so the queue never exceeds the source's own
+/// manifest size — the contract's bounded-buffering rule holds.
+enum SourceEvent {
+    Need(FileHeader),
+    NeedComplete,
+    Summary(TransferSummary),
+    Fault(SessionFault),
+}
+
+/// Run the SOURCE role of one transfer session over `transport`.
+/// Returns the destination-computed `TransferSummary` (contract: the
+/// end that wrote the bytes is the end that attests to them).
+pub async fn run_source(
+    cfg: SourceSessionConfig,
+    transport: FrameTransport,
+    source: Arc<dyn TransferSource>,
+) -> Result<TransferSummary> {
+    let mut transport = transport;
+    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
+        // Own-config coherence: a source initiator declares SOURCE.
+        let declared = TransferRole::try_from(open.initiator_role);
+        if declared != Ok(TransferRole::Source) {
+            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
+        }
+        if let Err(fault) = source_open_validator(open) {
+            eyre::bail!("run_source initiator config unsupported: {fault}");
+        }
+    }
+
+    let negotiated = establish(
+        &mut transport,
+        &cfg.hello,
+        &cfg.endpoint,
+        TransferRole::Source,
+        &source_open_validator,
+    )
+    .await?;
+
+    let (mut tx, rx) = transport.split();
+    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
+    let (event_tx, event_rx) = mpsc::unbounded_channel();
+    // AbortOnDrop: an early error return below must abort the receive
+    // half instead of leaking it (same rationale as design-2 / w4-1).
+    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
+        rx,
+        Arc::clone(&sent),
+        event_tx,
+    )));
+
+    match source_send_half(&cfg, &negotiated, &mut tx, source, sent, event_rx).await {
+        Ok(summary) => Ok(summary),
+        Err(report) => {
+            let mut fault = fault_from_report(report);
+            if !fault.peer_notified {
+                let _ = tx.send(error_frame(&fault)).await;
+                fault.peer_notified = true;
+            }
+            Err(eyre::Report::new(fault))
+        }
+    }
+}
+
+/// Receive half of the source driver: drains the transport for the
+/// whole session so destination sends can never deadlock against a
+/// blocked source send, and routes the destination lane to the send
+/// half. Terminates on summary, error, close, or violation.
+async fn source_recv_half(
+    mut rx: Box<dyn FrameRx>,
+    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
+    events: mpsc::UnboundedSender<SourceEvent>,
+) {
+    loop {
+        let received = match rx.recv().await {
+            Ok(Some(f)) => f,
+            Ok(None) => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
+                    "peer closed before TransferSummary",
+                )));
+                return;
+            }
+            Err(err) => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
+                    "transport receive failed: {err:#}"
+                ))));
+                return;
+            }
+        };
+        match received.frame {
+            Some(Frame::NeedBatch(batch)) => {
+                for entry in batch.entries {
+                    if entry.resume {
+                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
+                            format!(
+                                "resume-flagged need for '{}' in a session opened without resume",
+                                entry.relative_path
+                            ),
+                        )));
+                        return;
+                    }
+                    let header = sent
+                        .lock()
+                        .expect("sent-manifest lock poisoned")
+                        .remove(&entry.relative_path);
+                    match header {
+                        Some(h) => {
+                            let _ = events.send(SourceEvent::Need(h));
+                        }
+                        None => {
+                            let _ = events.send(SourceEvent::Fault(
+                                SessionFault::protocol_violation(format!(
+                                    "need for unknown or already-needed path '{}'",
+                                    entry.relative_path
+                                )),
+                            ));
+                            return;
+                        }
+                    }
+                }
+            }
+            Some(Frame::NeedComplete(_)) => {
+                let _ = events.send(SourceEvent::NeedComplete);
+            }
+            Some(Frame::Summary(summary)) => {
+                let _ = events.send(SourceEvent::Summary(summary));
+                return;
+            }
+            Some(Frame::Error(err)) => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
+                return;
+            }
+            other => {
+                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
+                    format!("{} on the source's receive lane", frame_name(&other)),
+                )));
+                return;
+            }
+        }
+    }
+}
+
+async fn source_send_half(
+    cfg: &SourceSessionConfig,
+    negotiated: &Negotiated,
+    tx: &mut Box<dyn FrameTx>,
+    source: Arc<dyn TransferSource>,
+    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
+    mut events: mpsc::UnboundedReceiver<SourceEvent>,
+) -> Result<TransferSummary> {
+    let mut pending: Vec<FileHeader> = Vec::new();
+    let mut need_complete = false;
+
+    // Streaming manifest: entries go out as enumeration produces them
+    // (immediate start in every direction — plan §Design 2). The open
+    // carries no source path: the source end owns its local endpoint.
+    let _ = &negotiated.open;
+    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
+    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
+    while let Some(header) = header_rx.recv().await {
+        sent.lock()
+            .expect("sent-manifest lock poisoned")
+            .insert(header.relative_path.clone(), header.clone());
+        tx.send(frame(Frame::ManifestEntry(header))).await?;
+        // Faults detected by the receive half abort the stream now,
+        // not after the full scan; needs just accumulate.
+        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
+    }
+    let scanned = scan_handle
+        .await
+        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
+    let scan_complete = unreadable
+        .lock()
+        .expect("unreadable list lock poisoned")
+        .is_empty();
+    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
+    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
+        scan_complete,
+    })))
+    .await?;
+
+    // Payload phase. In-stream record grammar: payload records only
+    // after ManifestComplete, strictly serialized per record
+    // (contract §Transport selection). Needs accumulated while a
+    // record batch was being sent become the next planner batch.
+    let mut read_buf = vec![0u8; IN_STREAM_CHUNK];
+    loop {
+        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
+        if !pending.is_empty() {
+            let batch = std::mem::take(&mut pending);
+            send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf).await?;
+            continue;
+        }
+        if need_complete {
+            break;
+        }
+        match events.recv().await {
+            Some(event) => {
+                handle_source_event(event, &mut pending, &mut need_complete)?;
+            }
+            None => {
+                return Err(eyre::Report::new(SessionFault::internal(
+                    "source receive half ended before NeedComplete",
+                )))
+            }
+        }
+    }
+
+    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
+
+    // CLOSING: the destination is the scorer; the next event must be
+    // its summary (the receive half ends after forwarding it).
+    match events.recv().await {
+        Some(SourceEvent::Summary(summary)) => Ok(summary),
+        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
+        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
+            format!("need for '{}' after NeedComplete", h.relative_path),
+        ))),
+        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
+            SessionFault::protocol_violation("duplicate NeedComplete"),
+        )),
+        None => Err(eyre::Report::new(SessionFault::internal(
+            "source receive half ended before TransferSummary",
+        ))),
+    }
+}
+
+fn drain_source_events(
+    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
+    pending: &mut Vec<FileHeader>,
+    need_complete: &mut bool,
+) -> Result<()> {
+    while let Ok(event) = events.try_recv() {
+        handle_source_event(event, pending, need_complete)?;
+    }
+    Ok(())
+}
+
+fn handle_source_event(
+    event: SourceEvent,
+    pending: &mut Vec<FileHeader>,
+    need_complete: &mut bool,
+) -> Result<()> {
+    match event {
+        SourceEvent::Need(header) => {
+            if *need_complete {
+                return Err(eyre::Report::new(SessionFault::protocol_violation(
+                    format!("need for '{}' after NeedComplete", header.relative_path),
+                )));
+            }
+            pending.push(header);
+            Ok(())
+        }
+        SourceEvent::NeedComplete => {
+            if *need_complete {
+                return Err(eyre::Report::new(SessionFault::protocol_violation(
+                    "duplicate NeedComplete",
+                )));
+            }
+            *need_complete = true;
+            Ok(())
+        }
+        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
+            "TransferSummary before SourceDone",
+        ))),
+        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
+    }
+}
+
+/// Plan one batch of needed headers with the engine planner and emit
+/// the resulting payload records per the in-stream grammar.
+async fn send_payload_records(
+    tx: &mut Box<dyn FrameTx>,
+    source: &Arc<dyn TransferSource>,
+    plan_options: PlanOptions,
+    batch: Vec<FileHeader>,
+    read_buf: &mut [u8],
+) -> Result<()> {
+    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
+    for payload in payloads {
+        match source.prepare_payload(payload).await? {
+            PreparedPayload::File(header) => {
+                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
+                if header.size == 0 {
+                    continue; // record complete at 0 cumulative bytes
+                }
+                let mut reader = source.open_file(&header).await?;
+                let mut remaining = header.size;
+                while remaining > 0 {
+                    let want = read_buf.len().min(remaining as usize);
+                    let got = reader.read(&mut read_buf[..want]).await?;
+                    if got == 0 {
+                        // Shorter on disk than the manifest promised —
+                        // the record can no longer complete at
+                        // header.size; abort rather than pad.
+                        eyre::bail!(
+                            "'{}' hit EOF with {} bytes still promised",
+                            header.relative_path,
+                            remaining
+                        );
+                    }
+                    tx.send(frame(Frame::FileData(FileData {
+                        content: read_buf[..got].to_vec(),
+                    })))
+                    .await?;
+                    remaining -= got as u64;
+                }
+            }
+            PreparedPayload::TarShard { headers, data } => {
+                tx.send(frame(Frame::TarShardHeader(TarShardHeader {
+                    files: headers,
+                    archive_size: data.len() as u64,
+                })))
+                .await?;
+                for chunk in data.chunks(IN_STREAM_CHUNK) {
+                    tx.send(frame(Frame::TarShardChunk(
+                        crate::generated::TarShardChunk {
+                            content: chunk.to_vec(),
+                        },
+                    )))
+                    .await?;
+                }
+                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
+                    .await?;
+            }
+            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
+                // The outbound planner never emits these (resume is
+                // receive-originated and lands at otp-7).
+                eyre::bail!("resume payload planned in a non-resume session");
+            }
+        }
+    }
+    Ok(())
+}
+
+// ---------------------------------------------------------------------------
+// DESTINATION driver
+// ---------------------------------------------------------------------------
+
+/// What the destination end can report after a completed session.
+#[derive(Debug, Clone)]
+pub struct DestinationOutcome {
+    /// The summary this end computed and sent (contract: DESTINATION
+    /// is the scorer).
+    pub summary: TransferSummary,
+    /// Paths this end put on the need list, in emission order. The
+    /// role suite pins these identical across role assignments — the
+    /// executable form of the owner's invariance requirement.
+    pub needed_paths: Vec<String>,
+}
+
+/// Run the DESTINATION role of one transfer session over `transport`,
+/// writing under `dst_root`. Diffs the streamed manifest against its
+/// own filesystem (the destination is the one diff owner — plan
+/// §Design 3), returns the summary it computed and sent.
+pub async fn run_destination(
+    cfg: DestinationSessionConfig,
+    transport: FrameTransport,
+    dst_root: PathBuf,
+) -> Result<DestinationOutcome> {
+    let mut transport = transport;
+    let endpoint = match cfg.endpoint {
+        SessionEndpoint::Initiator { mut open } => {
+            let declared = TransferRole::try_from(open.initiator_role);
+            if declared != Ok(TransferRole::Destination) {
+                eyre::bail!(
+                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
+                );
+            }
+            if let Err(fault) = destination_open_validator(&open) {
+                eyre::bail!("run_destination initiator config unsupported: {fault}");
+            }
+            // Dial contract: the byte receiver advertises capacity in
+            // its open when it is the initiator (contract §Invariants 5).
+            if open.receiver_capacity.is_none() {
+                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
+            }
+            SessionEndpoint::Initiator { open }
+        }
+        SessionEndpoint::Responder => SessionEndpoint::Responder,
+    };
+
+    let negotiated = establish(
+        &mut transport,
+        &cfg.hello,
+        &endpoint,
+        TransferRole::Destination,
+        &destination_open_validator,
+    )
+    .await?;
+
+    match destination_session(&mut transport, &negotiated, &dst_root).await {
+        Ok(outcome) => Ok(outcome),
+        Err(report) => {
+            let mut fault = fault_from_report(report);
+            if !fault.peer_notified {
+                let _ = transport.send(error_frame(&fault)).await;
+                fault.peer_notified = true;
+            }
+            Err(eyre::Report::new(fault))
+        }
+    }
+}
+
+fn violation(message: String) -> eyre::Report {
+    eyre::Report::new(SessionFault::protocol_violation(message))
+}
+
+async fn destination_session(
+    transport: &mut FrameTransport,
+    negotiated: &Negotiated,
+    dst_root: &Path,
+) -> Result<DestinationOutcome> {
+    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
+        .unwrap_or(ComparisonMode::Unspecified);
+    let compare_opts = CompareOptions {
+        mode: compare_mode.into(),
+        ignore_existing: negotiated.open.ignore_existing,
+        include_deletions: false, // mirror lands at otp-6
+    };
+    // src_root is only consumed by local File payloads, which never
+    // occur on a session destination (payload bytes arrive as records
+    // and go through the stream/tar write paths).
+    let sink = FsTransferSink::new(
+        PathBuf::new(),
+        dst_root.to_path_buf(),
+        FsSinkConfig {
+            preserve_times: true,
+            dry_run: false,
+            checksum: None,
+            resume: false,
+            compare_mode,
+        },
+    );
+    // Same canonical-containment chokepoint the sink write paths use
+    // (R46-F3), applied to diff stats so a hostile manifest path can't
+    // make the destination stat outside its root.
+    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
+
+    let mut pending: Vec<FileHeader> = Vec::new();
+    let mut outstanding: HashSet<String> = HashSet::new();
+    let mut needed_paths: Vec<String> = Vec::new();
+    let mut manifest_complete = false;
+    let mut files_written: u64 = 0;
+    let mut bytes_written: u64 = 0;
+
+    loop {
+        let received = match transport.recv().await? {
+            Some(f) => f,
+            None => {
+                return Err(eyre::Report::new(SessionFault::internal(
+                    "peer closed mid-session",
+                )))
+            }
+        };
+        match received.frame {
+            Some(Frame::ManifestEntry(header)) => {
+                if manifest_complete {
+                    return Err(violation(format!(
+                        "manifest entry '{}' after ManifestComplete",
+                        header.relative_path
+                    )));
+                }
+                pending.push(header);
+                if pending.len() >= DEST_DIFF_CHUNK {
+                    let chunk = std::mem::take(&mut pending);
+                    diff_chunk_and_send_needs(
+                        transport,
+                        chunk,
+                        dst_root,
+                        canonical_dst_root.as_deref(),
+                        &compare_opts,
+                        &mut outstanding,
+                        &mut needed_paths,
+                    )
+                    .await?;
+                }
+            }
+            Some(Frame::ManifestComplete(_complete)) => {
+                if manifest_complete {
+                    return Err(violation("duplicate ManifestComplete".into()));
+                }
+                // (scan_complete gates mirror purges from otp-6 on;
+                // nothing consumes it in otp-3.)
+                let chunk = std::mem::take(&mut pending);
+                diff_chunk_and_send_needs(
+                    transport,
+                    chunk,
+                    dst_root,
+                    canonical_dst_root.as_deref(),
+                    &compare_opts,
+                    &mut outstanding,
+                    &mut needed_paths,
+                )
+                .await?;
+                // NeedComplete only after ManifestComplete received
+                // AND every entry diffed — both true here.
+                transport
+                    .send(frame(Frame::NeedComplete(NeedComplete {})))
+                    .await?;
+                manifest_complete = true;
+            }
+            Some(Frame::FileBegin(header)) => {
+                if !manifest_complete {
+                    return Err(violation(format!(
+                        "payload record for '{}' before ManifestComplete",
+                        header.relative_path
+                    )));
+                }
+                if !outstanding.remove(&header.relative_path) {
+                    return Err(violation(format!(
+                        "payload for '{}' which is not on the need list",
+                        header.relative_path
+                    )));
+                }
+                let outcome = receive_file_record(transport, &sink, &header).await?;
+                files_written += outcome.files_written as u64;
+                bytes_written += outcome.bytes_written;
+            }
+            Some(Frame::TarShardHeader(shard)) => {
+                if !manifest_complete {
+                    return Err(violation("tar shard record before ManifestComplete".into()));
+                }
+                for h in &shard.files {
+                    if !outstanding.remove(&h.relative_path) {
+                        return Err(violation(format!(
+                            "tar shard entry '{}' which is not on the need list",
+                            h.relative_path
+                        )));
+                    }
+                }
+                let outcome = receive_tar_record(transport, &sink, shard).await?;
+                files_written += outcome.files_written as u64;
+                bytes_written += outcome.bytes_written;
+            }
+            Some(Frame::SourceDone(_)) => {
+                if !manifest_complete {
+                    return Err(violation("SourceDone before ManifestComplete".into()));
+                }
+                if !outstanding.is_empty() {
+                    return Err(violation(format!(
+                        "SourceDone with {} needed file(s) never sent",
+                        outstanding.len()
+                    )));
+                }
+                let summary = TransferSummary {
+                    files_transferred: files_written,
+                    bytes_transferred: bytes_written,
+                    entries_deleted: 0, // mirror lands at otp-6
+                    in_stream_carrier_used: true,
+                    files_resumed: 0, // resume lands at otp-7
+                };
+                transport.send(frame(Frame::Summary(summary))).await?;
+                return Ok(DestinationOutcome {
+                    summary,
+                    needed_paths,
+                });
+            }
+            Some(Frame::Error(err)) => {
+                return Err(eyre::Report::new(SessionFault::from_wire(err)));
+            }
+            other => {
+                // Everything else is off-lane or off-phase here:
+                // destination-lane frames echoed back, resume frames
+                // in a non-resume session (otp-7), resize with no
+                // data plane to resize (otp-4), stray handshake
+                // frames, bare FileData/TarShardChunk outside a
+                // record. Fail fast, no tolerant parsing.
+                return Err(violation(format!(
+                    "{} not valid on the destination's receive lane in this phase",
+                    frame_name(&other)
+                )));
+            }
+        }
+    }
+}
+
+/// Stat-and-compare one chunk of manifest entries on the blocking
+/// pool (2+ syscalls per entry — same rationale as the daemon's
+/// w4-4 chunked checks), then stream the resulting need batch.
+async fn diff_chunk_and_send_needs(
+    transport: &mut FrameTransport,
+    chunk: Vec<FileHeader>,
+    dst_root: &Path,
+    canonical_dst_root: Option<&Path>,
+    compare_opts: &CompareOptions,
+    outstanding: &mut HashSet<String>,
+    needed_paths: &mut Vec<String>,
+) -> Result<()> {
+    if chunk.is_empty() {
+        return Ok(());
+    }
+    let dst_root = dst_root.to_path_buf();
+    let canonical = canonical_dst_root.map(Path::to_path_buf);
+    let opts = compare_opts.clone();
+    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
+        let mut needed = Vec::new();
+        for header in &chunk {
+            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
+                needed.push(header.relative_path.clone());
+            }
+        }
+        Ok(needed)
+    })
+    .await
+    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
+
+    let entries: Vec<NeedEntry> = needed
+        .into_iter()
+        // A path the source manifests twice is diffed twice but
+        // needed at most once.
+        .filter(|path| outstanding.insert(path.clone()))
+        .map(|relative_path| {
+            needed_paths.push(relative_path.clone());
+            NeedEntry {
+                relative_path,
+                resume: false, // resume lands at otp-7
+            }
+        })
+        .collect();
+    if entries.is_empty() {
+        return Ok(());
+    }
+    transport
+        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
+        .await?;
+    Ok(())
+}
+
+/// Does the destination need this manifest entry? Stats its own file
+/// and delegates the verdict to `manifest::header_transfer_status` —
+/// the same mode-aware owner `compare_manifests` uses, fed from a
+/// live stat instead of a materialized target manifest.
+fn destination_needs(
+    header: &FileHeader,
+    dst_root: &Path,
+    canonical_dst_root: Option<&Path>,
+    opts: &CompareOptions,
+) -> Result<bool> {
+    let dst = match canonical_dst_root {
+        Some(canonical) => {
+            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
+        }
+        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
+    }
+    .map_err(|err| {
+        SessionFault::protocol_violation(format!(
+            "manifest path '{}' escapes the destination root: {err:#}",
+            header.relative_path
+        ))
+    })?;
+
+    let target = match std::fs::metadata(&dst) {
+        Ok(meta) if meta.is_file() => {
+            let mtime = match meta.modified() {
+                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
+                    Ok(d) => d.as_secs() as i64,
+                    Err(e) => -(e.duration().as_secs() as i64),
+                },
+                Err(_) => 0,
+            };
+            Some((meta.len(), mtime))
+        }
+        // Absent — or present as a directory/other, which a file
+        // write must replace: both diff as "target does not have it"
+        // (matches the push daemon's file_requires_upload).
+        _ => None,
+    };
+    let status = header_transfer_status(
+        header,
+        // Destination-side checksums are never precomputed; Checksum
+        // mode therefore transfers (the conservative arm of
+        // compare_file), matching what push does today.
+        target.map(|(size, mtime)| (size, mtime, &[] as &[u8])),
+        opts,
+    );
+    Ok(matches!(status, FileStatus::New | FileStatus::Modified))
+}
+
+/// Receive one strictly-serialized file record (`file_begin` already
+/// consumed) and stream its bytes into the sink through a bounded
+/// in-memory pipe — record completion is exactly `header.size`
+/// cumulative bytes (contract §Transport selection).
+async fn receive_file_record(
+    transport: &mut FrameTransport,
+    sink: &FsTransferSink,
+    header: &FileHeader,
+) -> Result<crate::remote::transfer::SinkOutcome> {
+    let (mut pipe_wr, mut pipe_rd) = tokio::io::duplex(FILE_RECORD_PIPE_BYTES);
+    let write = sink.write_file_stream(header, &mut pipe_rd);
+    let feed = async {
+        let mut remaining = header.size;
+        while remaining > 0 {
+            let received = match transport.recv().await? {
+                Some(f) => f,
+                None => {
+                    return Err(eyre::Report::new(SessionFault::internal(format!(
+                        "peer closed inside file record '{}'",
+                        header.relative_path
+                    ))))
+                }
+            };
+            match received.frame {
+                Some(Frame::FileData(data)) => {
+                    let len = data.content.len() as u64;
+                    if len > remaining {
+                        return Err(violation(format!(
+                            "file record '{}' overran its size by {} byte(s)",
+                            header.relative_path,
+                            len - remaining
+                        )));
+                    }
+                    pipe_wr.write_all(&data.content).await?;
+                    remaining -= len;
+                }
+                other => {
+                    // Strict serialization: nothing may interleave
+                    // with an open record on the source lane.
+                    return Err(violation(format!(
+                        "{} inside file record '{}' ({} byte(s) short)",
+                        frame_name(&other),
+                        header.relative_path,
+                        remaining
+                    )));
+                }
+            }
+        }
+        pipe_wr.shutdown().await?;
+        Ok(())
+    };
+    let (outcome, ()) = tokio::try_join!(write, feed)?;
+    Ok(outcome)
+}
+
+/// Receive one tar-shard record (`tar_shard_header` already consumed):
+/// buffer to exactly `archive_size` (bounded by the shared tar cap)
+/// and hand the archive to the sink's tar-safety unpack path.
+async fn receive_tar_record(
+    transport: &mut FrameTransport,
+    sink: &FsTransferSink,
+    shard: TarShardHeader,
+) -> Result<crate::remote::transfer::SinkOutcome> {
+    if shard.archive_size > MAX_TAR_SHARD_BYTES {
+        return Err(violation(format!(
+            "tar shard of {} bytes exceeds the {} byte cap",
+            shard.archive_size, MAX_TAR_SHARD_BYTES
+        )));
+    }
+    let mut data: Vec<u8> = Vec::new();
+    data.try_reserve_exact(shard.archive_size as usize)
+        .map_err(|err| eyre::eyre!("allocating {} byte tar shard: {err}", shard.archive_size))?;
+    loop {
+        let received = match transport.recv().await? {
+            Some(f) => f,
+            None => {
+                return Err(eyre::Report::new(SessionFault::internal(
+                    "peer closed inside tar shard record",
+                )))
+            }
+        };
+        match received.frame {
+            Some(Frame::TarShardChunk(chunk)) => {
+                if data.len() as u64 + chunk.content.len() as u64 > shard.archive_size {
+                    return Err(violation(format!(
+                        "tar shard record overran its declared {} bytes",
+                        shard.archive_size
+                    )));
+                }
+                data.extend_from_slice(&chunk.content);
+            }
+            Some(Frame::TarShardComplete(_)) => {
+                if data.len() as u64 != shard.archive_size {
+                    return Err(violation(format!(
+                        "tar shard record completed at {} of {} declared bytes",
+                        data.len(),
+                        shard.archive_size
+                    )));
+                }
+                return sink
+                    .write_payload(PreparedPayload::TarShard {
+                        headers: shard.files,
+                        data,
+                    })
+                    .await;
+            }
+            other => {
+                return Err(violation(format!(
+                    "{} inside tar shard record",
+                    frame_name(&other)
+                )));
+            }
+        }
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn build_id_has_version_and_git_components() {
+        let id = session_build_id();
+        let (version, git) = id.split_once('+').expect("build id must be version+git");
+        assert_eq!(version, env!("CARGO_PKG_VERSION"));
+        assert!(!git.is_empty(), "git component must be non-empty");
+    }
+
+    #[test]
+    fn fault_round_trips_the_wire_shape() {
+        let fault = SessionFault {
+            code: session_error::Code::BuildMismatch,
+            message: "boom".into(),
+            local_build_id: "1.0+aaa".into(),
+            peer_build_id: "1.0+bbb".into(),
+            peer_notified: false,
+        };
+        let wire = fault.to_wire();
+        let back = SessionFault::from_wire(wire);
+        assert_eq!(back.code, session_error::Code::BuildMismatch);
+        assert_eq!(back.message, "boom");
+        // from_wire swaps perspective: the sender's local is our peer.
+        assert_eq!(back.peer_build_id, "1.0+aaa");
+        assert_eq!(back.local_build_id, "1.0+bbb");
+        assert!(back.peer_notified);
+    }
+}

exec
/usr/bin/zsh -lc "git show ef9ffa1:.review/findings/otp-3-transfer-session-core.md | rg -n \"1484|1500|\\+16|Gate\"" in /home/michael/dev/Blit
 succeeded in 0ms:
108:Suite 1484 → 1500 (+16; count never dropped). New:
128:Gate: `cargo fmt --check` ✓, `clippy --workspace --all-targets
129:-D warnings` ✓, `cargo test --workspace` 1500/0 ✓.

codex
- [crates/blit-core/build.rs:22](/home/michael/dev/Blit/crates/blit-core/build.rs:22) — **High** — Same-build is not actually guaranteed. If git identity is unavailable, all builds collapse to `0.1.0+unknown`; also the build script only watches git HEAD/refs, so dirty source edits can rebuild with a stale clean SHA. That contradicts D-2026-07-05-2’s exact same-build refusal.

- [crates/blit-core/src/transfer_session/mod.rs:558](/home/michael/dev/Blit/crates/blit-core/src/transfer_session/mod.rs:558) — **Medium** — The SOURCE receive half accepts `NeedComplete` unconditionally. Per `docs/TRANSFER_SESSION.md`, `NeedComplete` may only arrive after SOURCE’s `ManifestComplete`; a scripted/misordered destination can send it early and the source will not fail fast, violating the protocol-state acceptance criterion.

VERDICT: FAIL — fixes required before accepting otp-3.
tokens used
154,893
- [crates/blit-core/build.rs:22](/home/michael/dev/Blit/crates/blit-core/build.rs:22) — **High** — Same-build is not actually guaranteed. If git identity is unavailable, all builds collapse to `0.1.0+unknown`; also the build script only watches git HEAD/refs, so dirty source edits can rebuild with a stale clean SHA. That contradicts D-2026-07-05-2’s exact same-build refusal.

- [crates/blit-core/src/transfer_session/mod.rs:558](/home/michael/dev/Blit/crates/blit-core/src/transfer_session/mod.rs:558) — **Medium** — The SOURCE receive half accepts `NeedComplete` unconditionally. Per `docs/TRANSFER_SESSION.md`, `NeedComplete` may only arrive after SOURCE’s `ManifestComplete`; a scripted/misordered destination can send it early and the source will not fail fast, violating the protocol-state acceptance criterion.

VERDICT: FAIL — fixes required before accepting otp-3.
