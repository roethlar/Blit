# w4-4 — Blocking work off the runtime: chunked manifest checks + fully-offloaded pull enumeration

**Source**: Design-review queue row `w4-4-blocking-work-off-runtime`
(`docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W4.4; evidence
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`
`async-daemon-push-manifest-blocking-stat-per-entry` +
`async-daemon-single-file-checksum-blocks-runtime`).
**Severity**: Medium (FAST/RELIABLE — blocking syscalls and full-file
hashing on tokio runtime workers stall every co-scheduled task:
other RPC handlers, the 10 Hz progress ticker, Subscribe forwarders).

## What (both halves re-verified at HEAD before coding)

**Half A — push manifest loop.** `handle_push_stream`'s `FileManifest`
arm ran `file_requires_upload` inline per entry: a canonical
containment check (`resolve_contained_path` → `verify_contained`'s
`std::fs::canonicalize` ancestor walk, ≥2 syscalls for a
not-yet-created destination) plus `fs::metadata` — ~3M+ blocking
syscalls on an executor worker for a 1M-file push; milliseconds each
on NFS/CIFS or cold caches.

**Half B — pull_sync enumeration.** `collect_pull_entries_with_checksums`
(relocated from the deleted `service/pull.rs` at ue-r2-1h) offloaded
only its directory branch to `spawn_blocking`+rayon; the single-file
branch ran inline in the async fn — two `std::fs::metadata` probes,
the filter check, and (with `--checksum`) `build_file_header`'s full
synchronous Blake3 of an arbitrarily large file, pinning a runtime
worker for the whole hash. The top-level `root.is_file()`/`is_dir()`
stats were inline too.

## Approach

**Half A — chunked `spawn_blocking` batches** (the spec's first
option; the lexical-containment alternative was rejected because it
weakens the F2 posture — a symlinked intermediate directory inside
the module could redirect the stat outside it, and the check is
canonical everywhere else by design):

- Manifest entries buffer into `pending_manifest`
  (`PendingManifestEntry { rel, sanitized, file }`); the wire-path
  sanitization now happens unconditionally at buffer time (previously
  only for required files — the header is discarded otherwise, so
  this is behavior-neutral).
- The buffer drains when `manifest_drain_due` fires: chunk full
  (`MANIFEST_CHECK_CHUNK` = `FILE_LIST_EARLY_FLUSH_ENTRIES` = 128 —
  the cadence a fast-streaming push sees) **or** the oldest buffered
  entry has waited past `MANIFEST_CHECK_MAX_DELAY`
  (= `FILE_LIST_EARLY_FLUSH_DELAY`, 5 ms) — the codex-review fix:
  without the delay trigger, a slowly-enumerating client's first
  need-list (and mid-manifest TCP spin-up) would wait for 128 trickled
  entries instead of milliseconds, since the batcher's own 64 KiB/5 ms
  triggers only evaluate inside `push()` calls, which chunking
  confines to drain time. The trigger is evaluated on the next
  arrival, matching the batcher's own push-time flush semantics. Each
  drain runs through `drain_manifest_checks`: ONE `spawn_blocking` runs
  the batch's `file_requires_upload` calls (batch moved in and back
  out — no per-entry clones), then need-list pushes + `files_to_upload`
  happen in async context in original manifest order.
- The mid-manifest TCP data-plane spin-up moved from per-entry to
  post-chunk-drain (`if flushed && …` unchanged otherwise): the plane
  still spins up mid-manifest on the first need-list flush, at chunk
  granularity. design-4's invariant (no fallback negotiation
  mid-manifest) is untouched — the guard and the forced-gRPC path are
  identical.
- `ManifestComplete` drains the sub-chunk remainder before `break`
  (no spin-up there: the post-manifest path owns negotiation once the
  manifest is done — same wire outcome, the early spin-up was purely
  a pipelining optimization that no longer applies at that point).
- `file_requires_upload` itself is unchanged — same containment, same
  size+mtime comparison, now called on a blocking thread.

**Half B — one `spawn_blocking` for the whole collection**: the
async fn is now a thin wrapper that spawns
`collect_pull_entries_sync` (the previous body verbatim, with the
directory branch's inner `spawn_blocking` unwrapped since the whole
fn already runs on a blocking thread; rayon `par_iter` unchanged).
Signature, error mapping (`Status::internal("enumeration task
failed: …")` on join error), and all semantics identical.

## Files

- `crates/blit-daemon/src/service/push/control.rs` — buffer +
  `MANIFEST_CHECK_CHUNK` + `drain_manifest_checks` + arm restructure;
  3 new unit tests.
- `crates/blit-daemon/src/service/pull_sync.rs` —
  `collect_pull_entries_with_checksums` → wrapper +
  `collect_pull_entries_sync`.

## Tests

- +4 `manifest_check_batch_tests` (blit-daemon 170 → 174): decision
  parity (up-to-date skipped; stale + missing queued with sanitized
  POSIX paths in manifest order; buffer drained), empty-drain no-op,
  containment-escape rejection through the batched path (unix
  symlink; the escape arm is unix-gated, the helper itself is
  platform-shared and pinned by path_safety's suite), and the
  `manifest_drain_due` trigger contract (chunk + delay, review fix).
- Mutation check: batched check forced to all-`true` → the decision-
  parity test fails; restored → green.
- Half B rides the existing pins: `single_file_filter_tests` and the
  enumeration tests call the same async fn; the 500-file design-5
  regression e2e + the remote push suites drive the chunked manifest
  path end-to-end (≥3 chunk drains + remainder + post-drain spin-up).
- Workspace gate: fmt + clippy clean; suite count grew 1472 → 1476
  (1475 at the slice commit, +1 trigger test with the review fix).

## User-visible changes

None intended: same requires-upload decisions, same need-list content
and order, same negotiation semantics. The only externally observable
timing difference is that need-list replies for a large push are
evaluated in 128-entry chunks (the reply batcher already flushed at
this threshold) and the daemon stays responsive during manifest
processing and checksum enumeration.

## Known gaps

- No dedicated pin asserts "no blocking syscalls on the runtime" —
  not expressible as a unit test; the structural change is the
  reviewed artifact.
- The manifest-phase disconnect blindness during compute phases is
  w4-3/w4-5 territory (already landed) and the checksum-collect
  abortability follow-up remains the W4.3 spec's optional follow-on,
  not this slice.
- `expected_rel_files` still accumulates unboundedly for the mirror
  purge phase (pre-existing, out of scope).
