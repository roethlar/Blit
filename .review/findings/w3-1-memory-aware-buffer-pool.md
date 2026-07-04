# w3-1-memory-aware-buffer-pool — BufferPool::for_data_plane owns the formula + memory cap

**Branch**: `master` (D-2026-06-07-1 branchless policy)
**Commit**: see REVIEW.md row
**Source findings**: constants-network-pool-ignores-memory,
duplication-buffer-pool-sizing-formula,
constants-receive-chunk-1mib-asymmetry —
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`; slice spec
`docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` W3.1.

## What

The data-plane BufferPool sizing formula (`pool_size = streams*2+4`,
`buffer_size = chunk_bytes.max(64 KiB)`,
`budget = buffer_size*pool_size*2`) was pasted at three sites with no
awareness of host memory: at the dial's 16-64 MiB chunk sizes the
formula authorizes multi-GiB budgets on any host (OOM-by-constant on
small-RAM machines). The spec's `for_data_plane(tuning, streams)`
signature predates w2-2's `TuningParams` deletion; the tuning owner is
`engine::TransferDial`, so the constructor takes the dial-read
`chunk_bytes` plus a stream authorization.

A 5-agent audit workflow re-verified the 2026-06-11 evidence against
post-REV4 HEAD before coding:

- Exactly three formula sites survive (`push/client/mod.rs:181-184`,
  `pull_sync.rs:721-725`, `pull_sync.rs:1320-1324`); the daemon
  `pull.rs` copy died with the Pull RPC (ue-r2-1h). The
  `TarShardExecutor` pool (daemon push, gRPC fallback receive) is a
  fixed-shape pool with no dial/stream inputs — out of scope, its
  unification is recorded post-0.1.0 debt in its own doc comment.
- The third site is **not** the formula: hand-rolled `pool_size = 4`,
  single-stream resume path. Its pool is inert at runtime (resume only
  calls `send_block*`, which write caller slices; `pool.acquire` is
  reached only from `send_file_double_buffered`), so unifying it to
  the formula's 6 is behavior-neutral.
- `send_file_double_buffered` holds **two** pool buffers per stream,
  acquired sequentially — hold-one-wait-for-second. Any budget below
  2 buffers/stream can deadlock a sender against itself; the
  constructor's liveness floor exists for this.
- Shrinking pool buffers below the session's `chunk_bytes` is
  wire-safe: file bytes travel raw under a size-carrying header (no
  per-chunk framing), and the effective send granularity already IS
  the pool's buffer size, not `chunk_bytes`.
- `dial.ceiling_max_streams()` is a hard bound on live streams under
  resize, enforced at proposal generation, sender add, and receiver
  accept; ADDed streams share the epoch-0 pool on both elastic paths.

## Approach

`BufferPool::for_data_plane(chunk_bytes, streams)` in `buffer.rs`,
backed by pure `data_plane_pool_params(chunk_bytes, streams,
available_memory)` so tests pin every regime deterministically:

- **Formula unchanged when memory is plentiful** (pinned): pure hoist
  on normal hosts.
- **Cap**: budget ≤ available_memory/4 (the pool's own doc example,
  previously ignored by every call site).
- **Liveness floor**: budget ≥ buffer_size × streams × 2 always wins
  over the cap; when the cap can't hold two full-size buffers per
  stream, `buffer_size` shrinks toward `DATA_PLANE_BUFFER_FLOOR`
  (64 KiB) instead of the concurrency.
- **Resize-enabled paths authorize `dial.ceiling_max_streams()`**
  instead of the epoch-0 count — closes both sites' "growing the pool
  live is a W3.1 concern" deferral without live-growth machinery:
  allocation is lazy, so the ceiling authorization costs nothing until
  resize ADDs streams, and an ADDed stream draws from a budget that
  already covers it. Non-elastic paths pass their exact count; the
  resume path passes 1.
- `available_memory_bytes()` hoisted from `BufferSizer` — **fixing a
  real units bug**: the old helper multiplied sysinfo's value by 1024
  ("reports in kilobytes"), but sysinfo 0.38 returns bytes (verified
  against the vendored crate source). Available memory was
  over-reported 1024×, which had made `BufferSizer`'s own /10 cap
  vacuous and would have made the new /4 cap vacuous too. Also
  `System::new()` + `refresh_memory()` instead of `new_all()` (same
  value, no process-table walk); zero-report fallback (512 MiB) kept
  as `sanitize_available_memory`.
- `DATA_PLANE_BUFFER_FLOOR` (64 KiB) exported from `buffer.rs` and
  adopted at the same-semantic floor sites: the session chunk clamp
  (`data_plane.rs` `from_stream_with_probe`), the receive-buffer clamp
  (`receive_stream_double_buffered`), and the dial's inflight-derived
  chunk floor (`dial.rs`, whose comment already said "matching the
  session's minimum buffer"). Coincidentally-equal 64 KiB literals
  (wire path cap, control-plane flush threshold, planner size bins)
  deliberately untouched.
- Comment-truth: `RECEIVE_CHUNK_SIZE`'s false "matches the send side"
  claim rewritten (receive is deliberately 1 MiB vs the sender's
  16-64 MiB pooled buffers; no per-chunk framing makes the asymmetry
  legal); BufferPool header example now shows `for_data_plane`.

## Files

- `crates/blit-core/src/buffer.rs` — floor const, memory helpers
  (units fix), `for_data_plane` + `data_plane_pool_params`, doc
  example, 8 new tests.
- `crates/blit-core/src/remote/push/client/mod.rs` — site 1: elastic
  authorization (`ceiling_max_streams` when `resize_sub` + dial
  present), formula lines deleted.
- `crates/blit-daemon/src/service/pull_sync.rs` — site 2 (multistream,
  elastic authorization under `resize_on`) and site 3 (resume,
  streams=1), formula lines deleted.
- `crates/blit-core/src/remote/transfer/data_plane.rs` — floor const
  at both clamps; `RECEIVE_CHUNK_SIZE` comment truth.
- `crates/blit-core/src/engine/dial.rs` — floor const at the inflight
  clamp.

## Tests

8 new in `blit-core` (`buffer::pool_tests`): legacy-parity pin, floor
pin, cap pin, shrink-preserving-liveness pin, tiny-cap liveness pin, a
full-grid liveness/floor property sweep, zero-memory fallback pin, and
a real-sysinfo construction smoke. Guard proof: disabling the
cap+floor line makes 3 pins fail (cap, shrink, tiny-cap); restored,
all pass. Workspace: 1452 → 1460 passed / 0 failed / 2 ignored across
37 suites; fmt + clippy clean (macOS host).

## Known gaps

- Receive-side buffer size stays the fixed 1 MiB `RECEIVE_CHUNK_SIZE`
  (now honestly documented). Threading the dial into the receive path
  is the rest of constants-receive-chunk-1mib-asymmetry — a separate
  slice if the owner wants it; the wire needs no change.
- The resume path's inert pool and its dead `payload_prefetch = 8`
  literal are left as-is (changing the 8 is behavior-free noise; the
  audit verified `send_block*` never touches prefetch or the pool).
- The units fix means `BufferSizer`'s /10 cap can now genuinely bind
  on low-memory hosts (it never did before) — that is the intended
  memory-aware behavior, not a regression; its unit tests inject
  memory and are unaffected.
- No e2e exercises a genuinely memory-capped host; the capped regimes
  are pinned at the params layer. `remote_regression.rs`'s pull-sync
  deadlock canary passes (liveness floor holds on real transfers).
- Windows parity: buffer.rs is platform-neutral and the new tests are
  deterministic (no sysinfo dependence except the smoke test);
  windows-latest CI on next push covers it. sysinfo's Windows backend
  reports bytes from `GlobalMemoryStatusEx` — same units fix applies.
