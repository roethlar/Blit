# Machine-local facts (per AGENTS.md `handoff`: never in docs/STATE.md)

## This Mac (owner's workstation) — recorded 2026-07-10, moved here 2026-07-11

- Rig SSH keys installed: zoey (root), Windows box (`michael@10.1.10.173`),
  skippy (`admin@skippy`).
- NOPASSWD sudoers rule for the zoey pool-drain/purge helper.
- zig + cargo-zigbuild toolchain (aarch64-musl static daemon builds).
- ssh ControlMaster sockets configured for the rigs.

## Additional Linux hosts — BUILD ONLY (owner rule, 2026-07-12)

Owner: "Use it only for building binaries. Same with the VM. Build
only if needed." Neither is a benchmark end, ever — and native builds
there are a FALLBACK only (the Mac's zigbuild already cross-compiles
every Linux target in play).

- `michael@magneto` — Arch Linux x86_64 (kernel 7.1.3-arch1-1), 4
  cores, 32 GiB RAM, 10 GbE. **Busy BitTorrent machine** — do not
  benchmark against it or load it. ssh key auth works (probed
  2026-07-12).
- Local VM on the Mac — Ubuntu ARM (aarch64), per owner. Build-only
  fallback likewise.

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
