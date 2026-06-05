# audit-h3c slice 1 — gRPC fallback frame contract + unified receive helper

**Source**: 2026-06-04 audit chain, R3 finding **H3** (GPT-12 / DoS-class hardening gap).
**Re-scope (2026-06-05)**: original h3c framing as "wrap pull.rs:752 in
`tokio::time::timeout`" was rejected by the owner because (a) message-granular timeout
against TCP-sized chunks would break legitimately slow healthy transfers, and (b) hardcoded
seconds violates the project's adapt-to-conditions principle. Replaced with a two-slice
**H3 Liveness Contract**:

- **Slice 1 (this slice)** — gRPC fallback frame sizing + unified receive helper.
- **Slice 2 (pending)** — dynamic progress-cadence watchdog + retryable
  `io::ErrorKind::TimedOut` plumbing.

## What

`tonic::Streaming<T>::message()` only resolves after a full protobuf message is decoded.
The TCP fast path uses `AsyncRead`/`AsyncWrite` and observes byte-level progress via
`StallGuard` / `StallGuardWriter` — every successful `poll_read`/`poll_write` resets the
idle deadline. The gRPC fallback path cannot do that: it sees only complete messages, so
TCP-sized chunks (16-64 MiB from `RemoteTuning`) over a slow link produce 30+ second
gaps between observable messages. Any per-message wall-clock timeout would trip on
healthy transfers.

The structural fix is to size gRPC fallback frames separately from TCP tuning, capped
small enough that even a floor-throughput link (~100 KB/s — mobile, satellite, throttled
WAN) produces messages at observable cadence (~10s per 1 MiB frame). Slice 2's watchdog
will then have something tangible to measure cadence against.

## Approach

1. **New module `transfer/grpc_fallback.rs`** with:
   - `GRPC_FALLBACK_CHUNK_BYTES = 1 MiB` — the ceiling. Sized so floor-throughput
     produces ~10s message cadence (1 MiB / 100 KB/s).
   - `clamp_fallback_chunk_size(n)` — `n.min(ceiling)`.
   - `FallbackRecv<T>` async trait (impl for `tonic::Streaming<T>`) and
     `recv_fallback_message` helper — the single chokepoint slice 2's watchdog wraps.
   - Module doc explicitly enumerates **in-scope** sites + **out-of-scope** sibling
     awaits (per memory `feedback-server-await-timeouts`: not-covered list prevents
     silent drift).

2. **Send-side cap** applied at the three sites that emit `FileData` /
   `TarShardChunk` over the gRPC control plane:
   - `sink::GrpcFallbackSink` (push, client → daemon)
   - `sink::GrpcServerStreamingSink` (pull, daemon → client)
   - `payload::transfer_payloads_via_control_plane` (dead-code today but `pub`
     re-exported — defense-in-depth against future external-crate callers)

3. **Receive-side routing** through the helper at three sites (the audit chain's named
   h3c surface):
   - `pull.rs:316` — plain `pull()` entry (force-gRPC bulk transfer).
   - `pull.rs:484` — `scan_remote_files` (force-gRPC metadata scan).
   - `pull.rs:752` — `pull_sync_with_spec` (the load-bearing control + data loop named
     by GPT-12 / R3 §H3).

   Each site has a `TODO(audit-h3c-2)` comment pinning the error-chain conversion slice
   2 must change so the retry classifier fires on stall errors (today the conversions
   strip `tonic::Status` to a plain `String`, which `is_retryable` can't classify).

4. **Audit-chain breadcrumbs** in `stall_guard.rs` module doc + `R2 §H3 Remediation
   status` reflect slice 1 shipping + slice 2 pending so the chain isn't misread.

## Files changed

- `crates/blit-core/src/remote/transfer/grpc_fallback.rs` — NEW (260 lines including
  9 tests).
- `crates/blit-core/src/remote/transfer/mod.rs` — `pub mod grpc_fallback;`.
- `crates/blit-core/src/remote/transfer/sink.rs` — clamp at both sinks; 4 new tests
  (FileData + TarShard, push side + pull side).
- `crates/blit-core/src/remote/transfer/payload.rs` — clamp at the dead-code
  defense-in-depth site.
- `crates/blit-core/src/remote/pull.rs` — 3 sites routed through helper, 3
  TODO(audit-h3c-2) comments at the error-conversion call sites.
- `crates/blit-core/src/remote/transfer/stall_guard.rs` — module doc updated for h3c
  status.
- `docs/audit/AUDIT_REPORT_2026-06-04_R2.md` — §H3 Remediation status updated with the
  two-slice contract.

## Tests added (5 new, total +5 in blit-core: 321 → 326)

In `grpc_fallback::tests`:
- `recv_fallback_message_passes_through_ok_some` — `Ok(Some(v))` unchanged.
- `recv_fallback_message_passes_through_ok_none_clean_eof` — `Ok(None)` is clean EOF,
  not a stall.
- `recv_fallback_message_propagates_tonic_status_err` — `Err(Status)` preserved with
  code + message intact.

The 4 existing `clamp_fallback_chunk_size` tests remain.

In `sink::tests`:
- `grpc_fallback_sink_caps_file_data_at_grpc_ceiling` — 3 MiB file at 64 MiB tuning
  produces exactly 3 × 1 MiB FileData frames.
- `grpc_server_streaming_sink_caps_file_data_at_grpc_ceiling` — daemon-side mirror.
- `grpc_fallback_sink_caps_tar_shard_chunks_at_grpc_ceiling` — same cap on the
  TarShardChunk branch.
- `grpc_server_streaming_sink_caps_tar_shard_chunks_at_grpc_ceiling` — daemon-side
  mirror.

Drain loops in the four new sink tests use `drop(sink)` + bounded `expected_frames`
count instead of `rx.is_empty()` — robust against future spawned-emitter refactors.

Validation green from a wiped target dir: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

## Known gaps (deliberate slice 1 ↔ slice 2 boundary)

- **No timeout or watchdog yet.** `recv_fallback_message` is a pass-through. Slice 2
  is where actual stall detection lands.
- **Error chain still strips `tonic::Status` to `String`** at the three pull.rs call
  sites. Slice 2 must change those conversions so `io::ErrorKind::TimedOut` from the
  watchdog survives in the eyre chain and `is_retryable` fires. `TODO(audit-h3c-2)`
  comments at each site mark this.
- **Floor-throughput assumption (100 KB/s)** is documented in
  `grpc_fallback.rs:85-91` but not pinned by an owner-decision memory. Slice 2 should
  ratify or revise as part of designing the watchdog cadence policy.
- **Helper signature may change in slice 2** if the watchdog needs per-call cadence
  state. Helper docstring acknowledges this — small touch at each of the 3 call sites
  is acceptable scope creep for slice 2.

## Out of audit-h3c scope (named in `grpc_fallback.rs` module doc)

- `pull.rs:~1210` — `RemoteFileStream::poll_read` (AsyncRead adapter, structurally
  different).
- `push/client/helpers.rs:245` — CLI push response forwarder (symmetric PUSH analog,
  not on the audit chain's h3c surface).
- `blit-app/src/transfers/remote.rs:734, :878` — app-side delegated-pull progress
  consumer (control stream observability, separate concern).
- `daemon/service/push/data_plane.rs:347` — daemon-side fallback receive (out of CLI
  scope).
- `daemon/service/pull_sync.rs:307/341/798/966` and `daemon/service/push/control.rs:62`
  — daemon-side receives (handled by HTTP/2 keepalive + cancel-on-disconnect).

## Cross-references

- R3 finding H3 (GPT-12 cited `pull.rs:752` as the original h3c target).
- R2 §H3 Remediation status (updated by this slice).
- `audit-h3a` (master `dd51a1c`) and `audit-h3b` (master `c14db51`) — sibling slices
  in the same H3 cluster. Both use byte-level `StallGuard`/`StallGuardWriter` on the
  TCP data plane; h3c uses message-level frame sizing because the gRPC layer is
  structurally different.
- Memory `feedback-server-await-timeouts` — explicit out-of-scope list per the rule.
- Adversarial verify workflow `wq0g4btzv` ran 3 reviewers (correctness / test-coverage
  / audit-chain-and-principles); all concerns at MED+ severity addressed in this round.
