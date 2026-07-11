# Machine-local facts (per AGENTS.md `handoff`: never in docs/STATE.md)

## This Mac (owner's workstation) — recorded 2026-07-10, moved here 2026-07-11

- Rig SSH keys installed: zoey (root), Windows box (`michael@10.1.10.173`),
  skippy (`admin@skippy`).
- NOPASSWD sudoers rule for the zoey pool-drain/purge helper.
- zig + cargo-zigbuild toolchain (aarch64-musl static daemon builds).
- ssh ControlMaster sockets configured for the rigs.

## Rig residue (recorded 2026-07-10)

- Windows box: `blit-bench-daemon` firewall rule + staged
  `purge-standby.ps1`; repo checkout DETACHED at `0f922de` with the
  owner's prior state stashed (`bench-cargo-lock`).
- zoey: `e757dcc` binaries staged in `blit-temp/` (kept for otp-12
  interleaved A/B).
