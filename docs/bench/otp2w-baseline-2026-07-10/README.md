# otp-2w — OLD-path baseline on the owner-designated cross-direction rig (2026-07-10)

**Status**: Recorded. This is the rig the owner designated for the
otp-12 acceptance bar's **cross-direction half** after the Mac↔zoey
session established (per D-2026-07-05-1's symmetric-endpoint rule)
that hardware-asymmetric pairs support per-direction verdicts only —
owner: "mac to windows would be closer spec. windows is faster, both
have 10gbe." The zoey dataset
(`docs/bench/otp2-baseline-2026-07-10/`) remains the per-direction
reference for the slow-pool rig; THIS dataset anchors both halves on
the close-spec pair.

**Build**: `0f922de` both ends — client macOS arm64 release; daemon
built natively on the host (source delivered as a git bundle — the
commits were unpushed and pushes are owner-gated; a bundle is a plain
file copy between the owner's machines).

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
  OpenSSH with PowerShell 7 default shell; the daemon is launched via
  WMI `Win32_Process.Create` because Windows OpenSSH kills session
  children on disconnect (a `Start-Process` daemon dies with the ssh
  session); cold caches = standby-list purge
  (`NtSetSystemInformation`, admin); durable pushes =
  `Write-VolumeCache D`; drain = `Get-Counter` PhysicalDisk write
  bytes/sec, three consecutive quiet 2 s samples; ONE program-scoped
  inbound firewall rule (`blit-bench-daemon`; remove with
  `Remove-NetFirewallRule -DisplayName blit-bench-daemon`). Config
  paths are TOML LITERAL strings — double-quoted TOML corrupts
  Windows paths (`\b` is an escape).

## Verdict-cell results (median of 4 cold, drained, durable runs; ms)

| fixture | push tcp | push grpc | pull tcp | pull grpc |
|---------|---------:|----------:|---------:|----------:|
| large (1 GiB)            | 3549 | 3562 | 1309 | 1316 |
| small (10k × 4 KiB)      | 2503 | 3330 | 1381 | 1494 |
| mixed (512 MiB + 5k×2K)  | 2844 | 3241 | 1316 | 1438 |

Per-run data: `runs.csv`; `drain.log` shows ZERO drain timeouts.
Stability is verdict-grade: per-cell spread ≤ 2% for 8 of 12 cells,
worst 11.9% (`pull_tcp_small`). Rounding: integer ms; even-count
median = floor of the mean of the middle two.

## Readings (recorded, not adjudicated)

- Pull ≈ 6.6 Gbit/s durable on the 1 GiB cell; push ≈ 2.4 Gbit/s.
  **Old push trails old pull ×1.8–×2.7 in every cell even on this
  close-spec pair.**
- On large pushes the carrier makes NO difference (TCP 3549 vs gRPC
  3562) — the wire is not the bottleneck; the ceiling is the
  receive/write side (Windows filesystem write path, possibly
  Defender real-time scanning, and/or the old push-receive code).
- Whether that ×2.7 is OS write-path cost or the old code's doing is
  exactly what the otp-12 comparison discriminates: if the unified
  path's push on this rig closes toward pull's number, it was the
  code — the plan's founding bet (D-2026-07-05-1). If it doesn't
  close, the residue is the platform write path, measurable as the
  same gap in both old and new.
- Windows Defender status was NOT altered for this baseline; the
  acceptance run must use the same Defender state for old and new
  (interleaved same-session A/B makes that automatic).

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
