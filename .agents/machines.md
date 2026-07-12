# Machine-local facts (per AGENTS.md `handoff`: never in docs/STATE.md)

## This Mac (owner's workstation) — recorded 2026-07-10, moved here 2026-07-11

- Rig SSH keys installed: zoey (root), Windows box (`michael@10.1.10.173`),
  skippy (`admin@skippy`).
- NOPASSWD sudoers rule for the zoey pool-drain/purge helper.
- zig + cargo-zigbuild toolchain (aarch64-musl static daemon builds).
- ssh ControlMaster sockets configured for the rigs.

## Additional Linux hosts (owner-offered 2026-07-12: "you can build
## binaries for linux")

- `michael@magneto` — Arch Linux x86_64 (kernel 7.1.3-arch1-1), 4
  cores, 32 GiB RAM; active NIC `enp1s0f1` is **10 GbE** (dual-port
  card, second port + 2 onboard ports down). ssh key auth works.
  Probed 2026-07-12. Target: plain `x86_64-unknown-linux-musl`
  zigbuild (same recipe as skippy staging).
- Local VM on the Mac — Ubuntu ARM (aarch64), per owner. Functional
  smoke target only: it shares the Mac's physical disk and carries
  virtualization timing noise, so it is NOT a verdict-cell end for
  otp-12 evidence.

## Rig residue (recorded 2026-07-10)

- Windows box: `blit-bench-daemon` firewall rule + staged
  `purge-standby.ps1`; repo checkout DETACHED at `0f922de` with the
  owner's prior state stashed (`bench-cargo-lock`).
- zoey: binaries staged 2026-07-10 in `blit-temp/` — **corrected
  2026-07-12**: the staged daemon embeds `731023bfc8a1.dirty.…`, NOT
  `e757dcc` as previously recorded (otp-2 README carries the full
  correction note; daemon code is identical between the two commits).
  Kept untouched as the otp-2 artifact; otp-12a stages clean sha-named
  rebuilds beside it (`blit-daemon-e757dcc`, `blit-daemon-<run sha>`).
  blit-temp path: `/volume/a595ddbf-d201-4e55-8183-ec78c8cd83e0/.srv/`
  `.unifi-drive/michael/.data/blit-temp`.
