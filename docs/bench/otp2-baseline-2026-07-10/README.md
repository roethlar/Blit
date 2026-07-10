# otp-2 — OLD-path symmetric disk-to-disk baseline (2026-07-10)

**Status**: Recorded (the ONE_TRANSFER_PATH converge-up reference)
**Build**: `731023b` (both ends, same commit — client macOS arm64
release, daemon static aarch64-musl via cargo-zigbuild)
**Harness**: `scripts/bench_otp2_baseline.sh` (methodology in its
header comment; this README records the rig, the numbers, and the
findings the probe runs produced)

## Rig

- **Client**: the owner's Mac (Apple Silicon), source/dest data on the
  internal APFS SSD (`~/blit-bench-work`, never `/tmp`).
- **Daemon**: `zoey` (UNAS 8 Pro; Alpine-based aarch64, 4 slow cores,
  16 GiB RAM; 8-spindle pool ~102 TiB behind a mirrored-NVMe write
  tier). All daemon-side state confined to the owner's `blit-temp`
  folder (standing safety rule).
- **Link**: Thunderbolt 10GbE (Mac `en9`, 10.1.10.54) ↔ zoey
  (10.1.10.206), same /24, ~0.4 ms RTT. Endpoint pinned by IP.
- Owner-stated and confirmed: zoey's CPU cannot saturate the link;
  cells are CPU/storage-bound, which is fine — the reference is
  per-cell on identical hardware, not wire-speed.

## Verdict-cell results (medians, ms; 4 cold runs/cell)

| fixture | push tcp | push grpc | pull tcp | pull grpc |
|---------|---------:|----------:|---------:|----------:|
| large (1 GiB)            | 2886 | 4665 | 1707 | 2815 |
| small (10k × 4 KiB)      | 4048 | 5400 | 2552 | 4084 |
| mixed (512 MiB + 5k×2K)  | 2648 | 4051 | 1510 | 2255 |

Full per-run data: `runs.csv`; summary incl. avg/best: `summary.csv`.
Sanity: TCP < gRPC in all 12 cells; 1 GiB pull ≈ 5.0 Gbit/s durable,
push ≈ 3.0 Gbit/s durable; small-file cells are per-file-cost bound
(push ≈ 405 µs/file, pull ≈ 255 µs/file on zoey's 4 slow cores —
same shape, higher constant, as the July skippy diagnosis).

## Methodology findings (why the harness looks the way it does)

1. **Naive transfer-return timing is a write-cache lottery.**
   Probe 1 (`probe1-no-sync-runs.csv`): push cells swung 4–8×
   (mixed: 1.4 s / 6.1 s / 11.6 s) purely on how much of the payload
   zoey's write tier absorbed before writeback throttled. Fix:
   the timed window includes a destination-end `sync`
   (durable-at-destination timing).
2. **The daemon host's write path is stateful.** Probe 2
   (`probe2-no-drain-runs.csv`): even durable-timed pushes ascend
   2.7 s → 13.4 s within a session as the NVMe tier fills and
   destages to the spindles. Fix: a drain-wait before every run
   (three consecutive 2 s windows with < 2 MiB written across all
   disks) restores agreement; a manual drained probe gave
   4.5/2.7/3.1 s (the residual first-run outlier is why the cell
   statistic is the MEDIAN of 4).
3. **Residual push spread is ±10–20% with one outlier per ~4 runs;
   pull spread is ±2–8%.** Prescription for the otp-12 acceptance
   run: pull cells may be compared against this baseline's absolute
   medians with the plan's ±10% noise band; push-cell verdicts should
   ADDITIONALLY be confirmed by interleaved same-session A/B (the
   pre-cutover binaries of `731023b` stay staged in zoey's
   `blit-temp` for exactly this).
4. **This rig's write ends are hardware-asymmetric** (client SSD vs
   daemon pool+tier): pull beats push in every cell for physics
   reasons, ~1.6–1.7× uniformly. OPEN QUESTION for the owner
   (recorded in `docs/STATE.md`): the plan's acceptance bullet
   "every cell ≤ the better of that cell's two old directions +
   noise" presupposed hardware-symmetric endpoints (the sf-1-era
   32-core/tmpfs rig). On this rig that bar is physically
   unreachable for push cells regardless of code. Proposed reading:
   per-direction converge-up (new ≤ old, same cell, +10%) is the
   verdict on hardware-asymmetric rigs; the cross-direction bar
   applies only where the endpoint hardware is actually symmetric.
   The code-level direction-invariance is separately enforced by
   construction (one driver, role-parameterized suite).

## Wire-reference data (explicitly NOT verdict cells)

The July 2026-07-05 measurements (`docs/bench/10gbe-2026-07-05/`)
used tmpfs local ends, ARC-warm re-reads, and no sync — deliberate
engine-vs-wire isolation on a different rig (skippy ↔ netwatch-01,
32 cores each). Per this plan slice they are re-labeled
**wire-reference only**: never compare directions or absolute times
from that data against these verdict cells.

## Reproduction

```
export ZOEY_SSH=root@zoey
export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
export ZOEY_HOST=10.1.10.206
RUNS=4 ./scripts/bench_otp2_baseline.sh
```

Requires: the staged same-commit daemon in `$ZOEY_TEMP`, a NOPASSWD
sudoers rule for `/usr/sbin/purge` on the client, and SSH key auth to
the daemon host.
