# otp-2 — OLD-path PER-DIRECTION disk-to-disk baseline (2026-07-10)

**Status**: Recorded. **Scope caveat (load-bearing)**: this rig's
endpoints are hardware-asymmetric (client SSD vs daemon pool), and
D-2026-07-05-1 rules that cross-direction performance comparisons are
valid **only on symmetric endpoints**. This dataset therefore anchors
**per-direction converge-up** (new ≤ old, same cell) and CANNOT anchor
the otp-12 acceptance bar's cross-direction half ("every cell ≤ the
better of that cell's two old directions"). Whether that half is
evaluated on a symmetric pair the owner designates, or waived in favor
of per-direction convergence, is an owner decision recorded as an open
question in `docs/STATE.md`.

**Build**: `e757dcc` (both ends, same commit — client macOS arm64
release; daemon static aarch64-musl via
`cargo zigbuild --release --target aarch64-unknown-linux-musl`).
**Harness**: `scripts/bench_otp2_baseline.sh` (methodology in its
header; the probe CSVs here are the evidence that earned each rule).

## Rig

- **Client**: the owner's Mac (Apple Silicon), data on the internal
  APFS SSD (`~/blit-bench-work`, never `/tmp`).
- **Daemon**: `zoey` (UNAS 8 Pro; Alpine-based aarch64, 4 slow cores,
  16 GiB RAM; 8-spindle pool ~102 TiB behind a mirrored-NVMe write
  tier). All daemon-side state confined to the owner's `blit-temp`
  folder (standing safety rule).
- **Link**: Thunderbolt 10GbE (Mac `en9`) ↔ zoey (10.1.10.206), same
  /24, ~0.4 ms RTT, endpoint pinned by IP.
- Owner-stated and confirmed: zoey's CPU cannot saturate the link;
  cells are CPU/storage-bound (the reference is per-cell on identical
  hardware, not wire-speed).

## Verdict-cell results (median of 4 cold, drained, durable runs; ms)

| fixture | push tcp | push grpc | pull tcp | pull grpc |
|---------|---------:|----------:|---------:|----------:|
| large (1 GiB)            | 3025 | 5211 | 1664 | 2383 |
| small (10k × 4 KiB)      | 3929 | 5220 | 2699 | 4238 |
| mixed (512 MiB + 5k×2K)  | 2666 | 3884 | 1503 | 2258 |

Per-run data: `runs.csv`; avg/best alongside medians: `summary.csv`;
per-run drain outcomes: `drain.log` (exactly one DRAIN-TIMEOUT, the
expected post-staging first run — its value, 3283 ms, is not even its
cell's maximum). Rounding: all times integer ms; an even-count median
is the floor of the mean of the middle two.

Sanity: TCP < gRPC in all 12 cells. 1 GiB durable ≈ 2.8 Gbit/s push /
5.2 Gbit/s pull. Small files are per-file-cost bound (push ≈ 393
µs/file, pull ≈ 270 µs/file on zoey's 4 slow cores — the same
per-file-bound shape as the July skippy diagnosis, at a slower
constant). Old-pull beats old-push in every cell, ×1.23–×2.19
depending on the cell — consistent with the destination-hardware
asymmetry (pushes write durably to the pool; pulls write to the SSD),
which is exactly why D-2026-07-05-1 excludes cross-direction verdicts
on such endpoints.

## Run-to-run stability (this dataset)

- **Pull cells**: within ±6% of the cell median, except one
  `pull_tcp_large` run at +21%.
- **Push cells**: within ±16% of the cell median, except one high
  outlier in each gRPC push cell (+49% and +90%) — the pool's tiered
  write path never fully stops being stateful. The MEDIAN is the cell
  statistic precisely because of these; both outliers are visible in
  `runs.csv`.
- **Cross-session check**: an independent earlier session with the
  pre-review harness (`probe4-prereview-session-runs.csv`; bare-sync
  pull windows, no sync-before-drain) produced medians agreeing
  within ~10% on most cells — plus a visible +~150 ms on 10k-file
  pull cells here, the honest per-file fsync durability cost the
  pre-review harness under-counted.
- **otp-12 prescription**: push-cell verdicts should be confirmed by
  interleaved same-session A/B (old-build vs new-build alternating),
  not by comparison to absolute numbers alone. The old-path binaries
  of this commit stay staged in zoey's `blit-temp` for that.

## Methodology findings (why the harness looks the way it does)

1. **Naive transfer-return timing is a write-cache lottery**
   (`probe1-no-sync-runs.csv`): per-cell spread up to 8.0× (mixed
   push 1446/6119/11577 ms; large push 3.8×; small push 1.7×) purely
   from how much of the payload zoey's write tier absorbed before
   writeback throttled. Fix: durable-at-destination windows.
2. **Durability must be equivalent on both ends**: Linux `sync`
   waits for writeback (push windows); macOS `sync(2)` only
   SCHEDULES writes, so pull windows fsync every landed file instead
   (media-level F_FULLFSYNC deliberately not used — the Linux side
   does not pay media flush either).
3. **The daemon host's write path is stateful**
   (`probe2-no-drain-runs.csv`): even durable-timed pushes ascend
   2.7 s → 13.4 s within a session as the NVMe tier fills and
   destages. Fix: sync-then-drain before every run (three
   consecutive 2 s windows with < 2 MiB written across all disks;
   timeouts are recorded per run label, never silent).
   `probe3-drained-pushes.csv` is the manual confirmation probe
   (4531/2731/3063 ms — agreement restored, residual first-run
   outlier → median).
4. **Wall clock, not monotonic**: start/end stamps are separate
   processes; a cross-process `time.monotonic()` attempt produced
   0/negative windows while the daemon logs showed multi-second
   transfers. Wall-clock windows are seconds long; the median absorbs
   any rare clock-step outlier.

## Wire-reference data (explicitly NOT verdict cells)

The July 2026-07-05 measurements (`docs/bench/10gbe-2026-07-05/`)
used tmpfs local ends, ARC-warm re-reads, and no sync — deliberate
engine-vs-wire isolation on a different rig (skippy ↔ netwatch-01,
32 cores each). Per this plan slice they are re-labeled
**wire-reference only**: never compare directions or absolute times
from that data against these verdict cells (D-2026-07-05-1).

## Reproduction

```
export ZOEY_SSH=root@zoey
export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
export ZOEY_HOST=10.1.10.206
RUNS=4 ./scripts/bench_otp2_baseline.sh
```

Requires: the staged same-commit daemon in `$ZOEY_TEMP`, a NOPASSWD
sudoers rule for `/usr/sbin/purge` on the client, python3 on the
client, and SSH key auth to the daemon host.
