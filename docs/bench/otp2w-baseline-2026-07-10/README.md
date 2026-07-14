# otp-2w — OLD-path baseline on the owner-designated cross-direction rig (2026-07-10)

> **⚠ SUPERSEDED AS THE ACCEPTANCE REFERENCE (D-2026-07-14-1, 2026-07-14) —
> retained as a HISTORICAL MTU-1500 record; the data below is unmodified.**
> This baseline was recorded with **netwatch-01 at MTU 1500** (it ran at 1500 for
> every benchmark ever recorded; raised to 9000 on 2026-07-13). A reference must
> share the MTU of the sessions graded against it, so rig W re-records once at
> MTU 9000 on `0f922de`, and the acceptance reference becomes the **per-cell
> minimum** of {this median, the re-recorded 9000 median} — it can only tighten,
> never loosen (`OTP12_ACCEPTANCE_RUN.md` D2/F2). **Do not cite this file as the
> live acceptance ceiling.**

**Status**: Recorded (historical). This is the rig the owner designated for the
otp-12 acceptance bar's **cross-direction half** after the Mac↔zoey
session established (per D-2026-07-05-1's symmetric-endpoint rule)
that hardware-asymmetric pairs support per-direction verdicts only —
owner: "mac to windows would be closer spec. windows is faster, both
have 10gbe." Closer-spec is the owner's designation, not a claim of
identical platforms: APFS and NTFS write paths differ (see Readings).
The zoey dataset (`docs/bench/otp2-baseline-2026-07-10/`) remains the
per-direction reference for the slow-pool rig.

**Build**: `0f922de` binaries both ends — client macOS arm64 release;
daemon built natively on the host (source delivered as a git bundle —
the commits were unpushed and pushes are owner-gated; a bundle is a
plain file copy between the owner's machines). The recorded run used
the harness as of the codex-fix round.

## Rig

- **Client**: the owner's Mac (Apple Silicon, APFS NVMe SSD), data in
  `~/blit-bench-work`.
- **Daemon host**: Windows 11 (10.0.26200), Ryzen 9 9950X3D
  (32 threads), 96 GiB RAM, module root on `D:` (PCIe Gen5 NVMe,
  Crucial T705). Repo at `F:\dev\blit_v2`; everything the bench
  writes lives under the owner-designated `D:\blit-test`.
- **Link**: Thunderbolt 10GbE (Mac) ↔ 10 Gbps NIC (host), ~0.4 ms.
- **Host plumbing** (first-of-kind on Windows, embodied in
  `scripts/bench_otp2w_baseline.sh` + `scripts/windows/purge-standby.ps1`):
  OpenSSH with PowerShell 7 default shell (multiplexed —
  ControlMaster); daemon launched via WMI `Win32_Process.Create`
  because Windows OpenSSH kills session children on disconnect
  (reproduced live); launch REFUSES over a stale daemon and teardown
  kills the recorded PID only; cold caches = standby-list purge
  (`NtSetSystemInformation`, admin, every API step checked); durable
  pushes = self-timed `Write-VolumeCache D`; drain = `Get-Counter`
  PhysicalDisk write bytes/sec, three consecutive quiet 2 s samples,
  failed probes warn rather than pass; ONE program-scoped inbound
  firewall rule (`blit-bench-daemon`; remove with
  `Remove-NetFirewallRule -DisplayName blit-bench-daemon`). Config
  paths are TOML LITERAL strings — double-quoted TOML corrupts
  Windows paths (`\b` is an escape).

## Verdict-cell results (median of 4 cold, drained, durable runs; ms)

| fixture | push tcp | push grpc | pull tcp | pull grpc |
|---------|---------:|----------:|---------:|----------:|
| large (1 GiB)            | 3054 | 3065 | 1294 | 1289 |
| small (10k × 4 KiB)      | 1868 | 2822 | 1280 | 1462 |
| mixed (512 MiB + 5k×2K)  | 2288 | 2687 | 1284 | 1408 |

Per-run data: `runs.csv`; `drain-outcomes.txt` shows zero anomalies.
Stability: per-cell (max−min)/min spreads 0.2–14.5%; 4 cells ≤ 2%,
9 cells ≤ 9%. Rounding: integer ms; even-count median = floor of the
mean of the middle two.

## Readings (recorded, not adjudicated)

- Pull ≈ 6.6 Gbit/s durable on the 1 GiB cell; push ≈ 2.8 Gbit/s.
  **Old push trails old pull ×1.46–×2.38 per cell on this
  close-spec pair** (large 2.36, small 1.46, mixed 1.78 on TCP).
- On the large fixture the carrier makes NO difference in either
  direction (push 3054 vs 3065; pull 1294 vs 1289) — the wire is not
  the bottleneck; the ceilings are the endpoint read/write paths.
- Whether the push gap is Windows write-path cost (NTFS, Defender
  real-time scanning — left at its normal state) or the old
  push-receive code is exactly what otp-12's interleaved old-vs-new
  discriminates: if the unified path's push closes toward pull, it
  was the code (the plan's founding bet, D-2026-07-05-1); if not,
  the residue is the platform write path, measurable as the same gap
  in both arms. Interleaved A/B keeps the Defender state identical
  across arms.

## Timing-overhead correction (probe1)

The first recorded session (`probe1-sshoverhead-{runs,summary}.csv`)
wrapped `ssh host Write-VolumeCache` inside the timed window; a
per-connection cost of ~0.5 s (plus pwsh spawn and module load,
~1.2 s total measured idle) landed on every PUSH window and none of
the pull windows, inflating push medians by ~0.5–0.6 s and the
push/pull ratios accordingly (codex otp-2w F3, upheld and
quantified). The recorded dataset uses SELF-TIMED durability steps —
the flush measures its own duration on the destination and only that
is added to the transfer segment — on both rigs.

## Reproduction

```
export WIN_SSH=michael@10.1.10.173
export WIN_HOST=10.1.10.173
export WIN_REPO='F:\dev\blit_v2'
export WIN_TEST='D:\blit-test'
RUNS=4 ./scripts/bench_otp2w_baseline.sh
```

Requires: daemon built on the host (`cargo build --release` in
`$WIN_REPO`), OpenSSH key auth with an admin token, python3 + the
NOPASSWD purge rule on the client.
