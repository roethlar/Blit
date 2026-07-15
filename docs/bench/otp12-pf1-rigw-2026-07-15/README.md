# otp-12 pf-1 rig-W paired phase diagnostic (2026-07-15)

**Status**: Evidence — exact registered session accepted by the harness and
independently recomputed. This is a current-build P1 diagnostic, not a causal
grade, fix proof, or acceptance run.

## Session identity and validity

- Pair: `q` ↔ `netwatch-01`; session `20260715T211759Z.30531`.
- Exact reviewed build on both endpoints:
  `8e019ef5e948b94a7aca7cb3a8d0be41204742af`.
- The sole terminal marker is `SESSION-COMPLETE` with that exact SHA.
  `SESSION-VOID` and `SESSION-COMPLETE.tmp` are absent.
- The registered OFF–ON–ON–OFF schedule completed all 128 unique arms:
  32 per block and cell, 64 per role and trace state, and 16 per pair index.
  Every row is exit zero, `drained`, and `valid=yes`; all 128 durable-total
  equations recompute exactly.
- The evidence has 768 valid clock samples, 128 client logs, 128 landed
  manifests, 48 unique trace-on TCP sessions, 11,392 raw/exported phase
  events, and 14,964 nonnegative endpoint-local intervals. There is no
  trace-off or gRPC trace leakage. Every landed manifest matches the declared
  canonical fixture.
- Successful finalization removed only the exact q and Windows session trees.
  Both endpoint ports were closed and no benchmark, daemon, build, session,
  or launcher process remained. The original complete evidence root remains
  retained on q at
  `/Users/michael/Dev/blit_v2_8e019ef-run/logs/otp12pf-rigw-20260715T211759Z.30531`.

The copied immutable payload (everything in this directory except this
README) is exactly 290 files / 20,517,586 bytes. Its SHA-256 inventory digest
is `1e8d815c74761f34f247eeccc931cccc6d0e69ea73baed68accd45b97d86e51f`,
computed over sorted lines of
`relative-path<TAB>byte-size<TAB>file-sha256<LF>`. Key files:

| artifact | SHA-256 |
|---|---|
| `SESSION-COMPLETE` | `f2cb09225086f73357ab53a6601fb4bd7ca0585e5a5d0b54ab4c13021d0450e1` |
| `runs.csv` | `69c10ae12f7591b93585670fcbb62f9021fdeeaf6c4a60e78277190d112bc979` |
| `clock-samples.csv` | `b2fbda4c5edd1fd3531c49b7d511ac9fe7c20f2945811adc16b18f8ded4f2845` |
| `summary.csv` | `4167357d4cf3d2cba560be234a0eba4ea6066da9d97c9865e29981dc13c8abef` |
| `phase_events.csv` | `088fce0369e504c363f7aa5ef87da6280747e31245651b83046953ef9f122a7e` |
| `phase_intervals.csv` | `e8c5df376a1e59e9a2353b20385c2521936de78d9962353c52967abfbc00a5aa` |

## Live worker/stream parity

The old initiator-dependent worker cap is absent on the live pair. For target
`wm_tcp_mixed`, all 16 traced arms (eight per initiator layout) reached resize
epoch 7 with `target_streams=8`, `live_streams=8`, and `accepted=true` on the
SOURCE trace; the matching DESTINATION trace also records accepted
`live_streams=8`. Thus SOURCE- and DESTINATION-initiated transfers reached the
same eight-stream target on both endpoint records. This proves live parity; it
does not by itself prove parity caused any wall-time change.

## Wall-time result

The authoritative measurand is the analyzer's durable total:
`transfer_ms + (settled_ms - 250) + flush_ms`. `Δ` below is
`destination_init - source_init`; ratio is `max/min`.

| cell | trace | source-init median ms | destination-init median ms | Δ ms | ratio | N_pair ms |
|---|---:|---:|---:|---:|---:|---:|
| `wm_tcp_mixed` | off | 1469.5 | 1368.5 | -101.0 | 1.0738 | 329 |
| `wm_tcp_mixed` | on | 1494.5 | 1367.0 | -127.5 | 1.0933 | 100 |
| `mw_tcp_mixed` | off | 2253.5 | 2305.5 | +52.0 | 1.0231 | 538 |
| `mw_tcp_mixed` | on | 2182.5 | 2311.0 | +128.5 | 1.0589 | 682 |
| `wm_grpc_mixed` | off | 1660.0 | 1425.0 | -235.0 | 1.1649 | 154 |
| `wm_grpc_mixed` | on | 1668.5 | 1443.5 | -225.0 | 1.1559 | 101 |
| `wm_tcp_large` | off | 1569.5 | 1428.0 | -141.5 | 1.0991 | 97 |
| `wm_tcp_large` | on | 1573.0 | 1430.5 | -142.5 | 1.0996 | 23 |

Historical P1 did not reproduce in the target cell: its direction reversed,
with DESTINATION initiation nominally 101–127.5 ms faster, and both point
ratios are within 1.10. This is not a formal P1 pass. The gRPC control fails
invariance, the large control is only just inside 1.10, and the target's
registered resolution is

```text
observer bias = |-127.5 - (-101)| = 26.5 ms
N_resolution  = max(N_pair_off=329, N_pair_on=100, 26.5) = 329 ms
```

That 329 ms floor exceeds both the historical P1 gap and its 20% and 50%
recovery thresholds. The current plan also forbids grading a counterfactual on
a rig where the baseline gap is absent. The only licensed interpretation is:
**valid current-build P1 non-reproduction; registered resolution check failed;
no hypothesis was confirmed or killed.**

## Two-layout phase timing (descriptive)

These are medians across the eight trace-on `wm_tcp_mixed` runs per layout.
All durations use one endpoint's monotonic clock. Resize rows are per-run sums
over epochs 1–7; overlapping spans are not additive wall-time attribution.

| endpoint-local span | source-init ms | destination-init ms | destination − source ms |
|---|---:|---:|---:|
| manifest send | 0.004 | 0.001 | -0.003 |
| manifest sent → first payload queued | 2.955 | 4.622 | +1.667 |
| first payload queued → epoch-0 first socket write | 714.535 | 716.177 | +1.642 |
| SOURCE resize proposed → source settled, epochs 1–7 sum | 31.396 | 10.484 | -20.913 |
| SOURCE resize sent → ACK received, epochs 1–7 sum | 29.131 | 10.190 | -18.940 |
| SOURCE ACK received → source settled, epochs 1–7 sum | 2.323 | 0.297 | -2.026 |
| DESTINATION resize received → ACK sent, epochs 1–7 sum | 0.017 | 1.387 | +1.370 |

The phase rows describe the run but do not establish causation. In particular,
there is no parity rollback or dial/accept inversion counterfactual, and the
baseline wall-time gap required by the plan is absent.

## What remains

- This four-cell diagnostic is not the complete pf-1 hard gate. The
  small-fixture/P2 instrumentation, `0f922de` historical control, and
  wall-time counterfactuals remain.
- The existing P1 topology ablations must not consume more rig time yet: with
  no positive baseline gap and a 329 ms floor, their result would be formally
  ungradeable. A further P1 experiment requires an owner-approved plan
  amendment defining an adequate-resolution design and the absent/reversed
  baseline case.
- P2's historical-control and instrumentation-on/off work is already in the
  active plan and can proceed independently.
- The generated `summary.md` is preserved byte-for-byte and contains one prose
  typo (`The conservative operative The independent ...`). q daemon logs also
  retain 64 nonfatal recents-persistence permission warnings. Neither affects
  transfer, inventory, trace, clock, or analyzer validity.
