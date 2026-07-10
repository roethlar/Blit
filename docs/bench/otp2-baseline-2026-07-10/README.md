# otp-2 — OLD-path PER-DIRECTION disk-to-disk baseline (2026-07-10)

**Status**: Recorded. **Scope (load-bearing)**: this rig's endpoints
are hardware-asymmetric (client SSD vs daemon pool), and
D-2026-07-05-1 rules that cross-direction performance comparisons are
valid **only on symmetric endpoints**. This dataset therefore anchors
**per-direction converge-up** (new ≤ old, same cell) and cannot anchor
the otp-12 acceptance bar's cross-direction half — the owner
designated the Mac↔Windows pair for that
(`docs/bench/otp2w-baseline-2026-07-10/`).

**Build**: `e757dcc` binaries both ends (client macOS arm64 release;
daemon static aarch64-musl via
`cargo zigbuild --release --target aarch64-unknown-linux-musl`); the
recorded run used the harness as of `ceea6ed`+review fixes.
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
| large (1 GiB)            | 2702 | 4510 | 1744 | 2585 |
| small (10k × 4 KiB)      | 4263 | 5217 | 2784 | 4188 |
| mixed (512 MiB + 5k×2K)  | 2070 | 3889 | 1401 | 2222 |

Per-run data: `runs.csv`; avg/best alongside medians: `summary.csv`;
per-run drain outcomes: `drain-outcomes.txt` (zero anomalies).
Rounding: integer ms; even-count median = floor of the mean of the
middle two.

Sanity: TCP < gRPC in all 12 cells. 1 GiB durable ≈ 3.2 Gbit/s push /
4.9 Gbit/s pull. Small files are per-file-cost bound (push ≈ 426
µs/file, pull ≈ 278 µs/file on zoey's 4 slow cores — the July skippy
diagnosis's per-file-bound shape at a slower constant). Old-pull beats
old-push in every cell, ×1.25–×1.75 — but on THESE endpoints that gap
is confounded with destination hardware (pool vs SSD), which is
exactly why D-2026-07-05-1 excludes cross-direction verdicts here.

## Run-to-run stability (this dataset)

Zero drain anomalies; per-cell (max−min)/min spreads: 5.6–26.5%
typical, worst 48.6% (`push_tcp_small` — one fast outlier run, the
others within 9% of each other). The pool's tiered write path never
fully stops being stateful; the MEDIAN is the cell statistic
precisely because of this, and every run is visible in `runs.csv`.
**otp-12 prescription**: on this rig, verdicts (especially push
cells) should be confirmed by interleaved same-session A/B
(old-build vs new-build alternating), not by absolute comparison
alone. The old-path binaries stay staged in zoey's `blit-temp`.

## Methodology findings (why the harness looks the way it does)

1. **Naive transfer-return timing is a write-cache lottery**
   (`probe1-no-sync-runs.csv`): per-cell spread up to 8.0× (mixed
   push 1446/6119/11577 ms) purely from how much of the payload the
   write tier absorbed before writeback throttled. Fix:
   durable-at-destination windows.
2. **Durability must be equivalent on both ends**: Linux `sync`
   waits for writeback (push windows); macOS `sync(2)` only
   SCHEDULES writes, so pull windows fsync every landed file instead
   (media-level F_FULLFSYNC deliberately not used — the Linux side
   does not pay media flush either).
3. **The daemon host's write path is stateful**
   (`probe2-no-drain-runs.csv`): durable-timed pushes ascend
   2.7 s → 13.4 s within a session as the NVMe tier fills and
   destages. Fix: sync-then-drain before every run (three
   consecutive quiet 2 s windows; timeouts recorded per run label,
   never silent). `probe3-drained-pushes.csv` is the manual
   confirmation probe.
4. **Wall clock, not monotonic**: start/end stamps are separate
   processes; cross-process `time.monotonic()` has undefined
   reference points and produced 0/negative windows (aborted run).
5. **The durability step must time ITSELF** (codex otp-2w F3,
   quantified): `ssh zoey sync` inside the window costs ~1.2 s of
   connection setup (slow-core key exchange, measured) that lands
   only on push cells. `probe5-sshoverhead-{runs,summary}.csv` is
   the affected session — its push medians run ~0.3–0.6 s high. The
   recorded dataset uses self-timed destination flushes (the remote
   `sync` measures its own duration via `/proc/uptime`; the local
   fsync walk reports its own elapsed), so connection/shell overhead
   is excluded from every window on both rigs.

## Wire-reference data (explicitly NOT verdict cells)

The July 2026-07-05 measurements (`docs/bench/10gbe-2026-07-05/`)
used tmpfs local ends, ARC-warm re-reads, and no sync — deliberate
engine-vs-wire isolation on a different rig. Per this plan slice they
are re-labeled **wire-reference only**: never compare directions or
absolute times from that data against these verdict cells
(D-2026-07-05-1). `probe4-prereview-session-runs.csv` is an earlier
session of THIS matrix kept for cross-session corroboration.

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
