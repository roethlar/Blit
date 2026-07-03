# ue-r2-1e — codex (gpt-5.5) review output

Reviewer: `codex exec -s read-only` on `gpt-5.5` (headroom proxy), slice
range `e1a21a1..15968f4`. Findings excerpt (full transcript not
retained, per precedent; 200,205 tokens used; a first attempt hung on
an inherited-stdin quirk and was killed before producing output):

---

## Findings

- `crates/blit-core/src/engine/dial.rs:116` — **Medium** —
  `max_inflight_bytes` only lowers prefetch. If a receiver advertises
  `max_inflight_bytes < chunk_bytes` and leaves `max_chunk_bytes = 0`
  (unknown), the dial still starts at 16 MiB with prefetch 1, exceeding
  the advertised in-flight budget. The clamp needs to lower the chunk
  ceiling too when one chunk cannot fit.
- `crates/blit-core/src/engine/dial.rs:278` — **Medium** — The tuner
  ignores `bytes_sent` and treats an idle tick as clean telemetry.
  Fresh probes with zero bytes produce `blocked_ratio = 0.0`, so
  `apply_tick` steps up before the first byte or during
  source/preparation stalls. This weakens the conservative-start
  contract; the paused-clock test codifies the false-positive case.
- `crates/blit-core/src/remote/transfer/data_plane.rs:519` — **Medium**
  — TCP tar-shard payload writes record bytes but never call
  `note_write_blocked`. Small-file pushes are commonly tar-sharded, so
  their write pressure is invisible to the tuner and will look like
  zero-blocked traffic, ramping chunk/prefetch in the wrong direction
  on a saturated link.

VERDICT: NEEDS FIXES before accepting `ue-r2-1e`.
