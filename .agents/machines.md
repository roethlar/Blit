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
  cores, 32 GiB RAM, 10 GbE, WD SN850 NVMe. ~~Busy BitTorrent machine —
  never a bench end~~ **SUPERSEDED 2026-07-13: magneto IS a valid bench
  end** (owner: "power efficient intel, but it should be fast enough to
  saturate 10GbE where Zoey is definitely not"; services quiescable on
  request). Only ONE NIC is in use: `enp1s0f1` = 10.1.10.10 (the other
  three have no IP). Used as the **same-OS rig** magneto↔skippy —
  the only pair in the fleet that can measure blit's layout with zero
  platform terms. NOPASSWD `drop_caches` grant added 2026-07-13.
- `michael@altiera` — Linux, 2.5 GbE (enp1s0=10.1.10.53,
  enp2s0=10.1.10.59, both MTU 9000). **NOT usable as a bench end**: at
  2.5 GbE the link (~312 MB/s) is slower than the fixtures need, so it
  bottlenecks BOTH arms of an invariance pair equally, dragging the
  ratio toward 1.0 and MASKING the effect under test — zoey's failure
  mode by a different route.

## Network / MTU (rig-critical — read before touching MTU)

- **THE macOS PING TRAP (cost ~1h on 2026-07-13; do not repeat).**
  macOS caps **raw sockets** at 8192 bytes via `net.inet.raw.maxdgram`,
  and `ping` uses a raw socket. So DF pings above ~8164 payload FAIL
  from a Mac **no matter what the real path MTU is**. This is a limit on
  the ping tool, NOT on the network, and it does **not** affect TCP.
  I misread it as "macOS cannot transmit jumbo frames", blamed the
  switch, then blamed two innocent adapters, and had the owner swap
  hardware for nothing. **Verify jumbo with a real TCP transfer** (e.g.
  `scp` a large file), never with `ping`.
- **Jumbo works end-to-end at MTU 9000** (verified 2026-07-13 by real
  TCP, not ping): Mac↔Windows 231/225 MB/s, Mac↔skippy 157 MB/s (all
  ssh-encrypted, so CPU-bound floors — the wire is not the limit). The
  UniFi switching passes 9018-byte frames fine.
- **Windows (netwatch-01) ran at MTU 1500 for EVERY benchmark ever
  recorded** (otp-2w, otp-12a/b/c). It was raised to 9000 on 2026-07-13.
  Every prior measurement therefore negotiated down to a 1460-byte MSS:
  **jumbo has never been exercised in a blit benchmark.** Those numbers
  are valid — they are simply *1500-MTU* numbers — and rig W at jumbo is
  a genuinely untested condition. magneto is still 1500 (raise
  `enp1s0f1` to 9000 to make the Linux rig jumbo too).
- Mismatched MTUs on one L2 segment are fine: TCP MSS negotiation
  handles it, each host advertising what it can receive. What is NOT
  fine is a host advertising a size it cannot actually send.
- **Fleet MTU as of 2026-07-13 — the whole 10 GbE fabric is now 9000:**

  | host | iface | MTU | persistent? |
  |---|---|---|---|
  | Mac | `en9` (Aquantia) | 9000 | yes (macOS net service) |
  | netwatch-01 | Ethernet | 9000 | yes (raised 1500→9000 today) |
  | skippy | `enp66s0f1` | 9000 | yes |
  | **zoey** | `enp0s0` (RJ45, NFS data .206) | **9000** | yes — `[Link] MTUBytes=9000` in `/etc/systemd/network/enp0s0.network` |
  | **zoey** | `enp0s1` (SFP, mgmt .210) | **9000** | yes — same, in `enp0s1.network` |
  | altiera | `enp1s0`/`enp2s0` | 9000 | yes (NetworkManager profiles) |
  | magneto | `enp1s0f1` | 9000 | **NO — NM profile still `mtu=auto`**; needs `nmcli con mod "Wired connection 3" 802-3-ethernet.mtu 9000` or it reverts on reboot |

- **zoey (UniFi UNAS Pro) jumbo — how it was done, and the trap.**
  Debian 11 + `systemd-networkd`; NIC `maxmtu` is 9216 so the hardware is
  fine. Persistence = a `[Link]` / `MTUBytes=9000` stanza in each
  `/etc/systemd/network/enp0s*.network` (originals backed up as
  `*.premtu`). Proven by `networkctl reload && networkctl reconfigure`
  with the static IP intact — no reboot needed. **TRAP: `/` is an
  overlayfs** (`lowerdir=/mnt/.rofs` read-only + writable upper), so a
  UniFi *firmware update* can replace the base image and silently drop
  this. Re-check after any UNAS update:
  `ssh root@zoey 'cat /sys/class/net/enp0s0/mtu'` → want 9000.
  Method for any risky remote NIC change: arm a self-healing revert
  first — `nohup setsid bash -c 'sleep 90; [ -f /tmp/ok ] || ip link set
  IFACE mtu 1500' &` — then confirm with `touch /tmp/ok`. Change the NIC
  you are NOT ssh'd through when a second one exists.
- **Live NFS/TCP connections do NOT pick up a new MTU.** MSS is fixed at
  connect time, so an existing mount keeps its old segment size until it
  reconnects (reboot/remount). Not worth forcing for low-bandwidth
  mounts.
- Two-NICs-on-one-subnet (both `altiera` and `zoey`, and it is the
  default `arp_ignore=0 arp_announce=0`) invites ARP flux + asymmetric
  routing. Working today; a latent source of intermittent stalls.
- Local VM on the Mac — Ubuntu ARM (aarch64), per owner. Build-only
  fallback likewise.

## Rig residue (recorded 2026-07-10)

- **The Mac's 10GbE IP and NIC CHANGED 2026-07-13** — this is a live
  confound in the otp-12 numbers, not a bookkeeping detail:
  * **now: `en9` = 10.1.10.54**, a Thunderbolt **Aquantia** adapter,
    MTU 9000, 10Gbase-T. (SSH into the Mac = `michael@10.1.10.54`;
    Remote Login is ON and netwatch-01's key is in the Mac's
    `authorized_keys`, so Windows→Mac ssh/sftp works.)
  * otp-12b (`wm_tcp_mixed` **1.237**) ran on the Aquantia at
    **10.1.10.54**; otp-12c (**1.300**) ran on a Thunderbolt-5 dock's
    built-in 10GbE at **10.1.10.91**. **Different NICs.** So the
    "1.237 → 1.300, it got worse at the cutover sha" reading is
    CONFOUNDED by a hardware change and must not be cited as evidence
    of a code regression. Both runs still showed the same qualitative
    asymmetry; only the delta is suspect.
  * Harnesses take the Mac IP via `MAC_HOST=` — pass **10.1.10.54**
    (older invocations in the DEVLOG say 10.1.10.91).
- Windows box = **`michael@netwatch-01`, IP 10.1.10.177 as of
  2026-07-12** (the earlier-recorded 10.1.10.173 is STALE — DHCP; ssh
  by hostname; if the bare name stops resolving, `netwatch-01.local` or
  the IP both work — the host key is filed under both). **MTU raised
  1500 → 9000 on 2026-07-13** (see Network/MTU above). SMB File Sharing
  is now ON on the Mac and Windows is authenticated to it
  (`net use \\10.1.10.91\blit-bench-work`), so robocopy can reach it.
  Rules: `blit-bench-daemon` (otp-2w, repo-path-scoped)
  + `blit-otp12-daemon` (active-path-scoped) + staged
  `purge-standby.ps1`; repo checkout DETACHED at `e21cf84` since the
  otp-12b session (owner's `bench-cargo-lock` stash untouched); old
  `0f922de` exes aside-copied at `D:\blit-test\bins\0f922de\`; run
  bins under `D:\blit-test\bins\<sha>\`.
- **Rig pairing constraint (owner, 2026-07-13): zoey's CPU is too slow
  to be a match for skippy** — a zoey↔skippy pair is NOT a valid
  symmetric/performance-matched rig; a zoey endpoint becomes the
  bottleneck and MASKS data-plane effects rather than measuring them.
  Consequence, recorded so it is not re-proposed: the fleet has **no
  same-OS, real-network, performance-matched pair** (one Mac; zoey too
  slow for skippy; magneto is a busy BitTorrent box — build-only, never
  a bench end). Platform-vs-role confounds on a two-host rig therefore
  cannot be broken by rig juggling and need a code-level counterfactual
  (see `docs/plan/OTP12_PERF_FINDINGS.md`).
- zoey: binaries staged 2026-07-10 in `blit-temp/` — **corrected
  2026-07-12**: the staged daemon embeds `731023bfc8a1.dirty.…`, NOT
  `e757dcc` as previously recorded (otp-2 README carries the full
  correction note; daemon code is identical between the two commits).
  Kept untouched as the otp-2 artifact; otp-12a stages clean sha-named
  rebuilds beside it (`blit-daemon-e757dcc`, `blit-daemon-<run sha>`).
  blit-temp path: `/volume/a595ddbf-d201-4e55-8183-ec78c8cd83e0/.srv/`
  `.unifi-drive/michael/.data/blit-temp`.
