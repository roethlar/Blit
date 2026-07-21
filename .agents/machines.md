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
  | magneto | `enp1s0f1` | 9000 | yes — NM profile `Wired connection 3` saved `mtu=9000` (2026-07-13) |

  **Verified end-to-end 2026-07-13**: a jumbo DF ping from skippy reaches
  magneto, zoey, altiera, netwatch-01 AND the Mac — all OK. Every 10 GbE
  pair in the fleet carries 9000-byte frames. (Always test from a LINUX
  host; the Mac's `ping` cannot send >8192 — see the raw-socket trap.)

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

## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)

`ssh michael@q` — Apple **M4 Mac mini**, 16 GB, macOS 26.5.2, arm64. It is now
the rig-W Mac end: **quiet, dedicated, and faster than nagatha** (1 GiB in
~908 ms ≈ 1.18 GB/s, vs nagatha's ~1.3–1.8 s). Using it **decouples the codex
review loop from rig-W benchmarking** — the contention that destroyed a
53-minute experiment (below).

- **CURRENT STATE (observed 2026-07-21 at repo `31c12c9`):** q's reconnected
  Aquantia is restored as registered `en8` / `10.1.10.54`, MTU 9000, active
  10Gbase-T, and the route to Windows uses `en8`. Windows DHCP has moved
  `NETWATCH-01` from `10.1.10.177` back to `10.1.10.173`:
  `.177` has incomplete ARP/no TCP 22, while DNS and strict-host-key SSH from
  nagatha identify `.173` as `NETWATCH-01`. q has the same trusted host key
  under `.177` but no `.173` entry. D-2026-07-21-1 directs ldt-4 to adapt to
  `.173`; add only the already verified matching key under that address after
  exact code review. Never bypass host-key checking.
- **CURRENT IDENTITY/STAGING (2026-07-21):** q now reports resolver-derived
  `hostname=Q.local`; stable macOS `LocalHostName=Q` and `ComputerName=Q` are
  unchanged. Exact `322a161` is staged at
  `~/Dev/blit_v2_harness_322a161` from complete-history bundle
  `~/blit-ldt4-stage-322a161.bundle` (SHA-256
  `429e4bea6bfecd497ebd73e4972ef736c189fa09c482c8ff4cb5301c6cf279da`;
  target history count 1,938); q Bash 3.2 and all 76 analyzer tests pass.
  q's `.173` trust entry was added only by aliasing its three already trusted
  matching `.177` keys; pre-edit backup is
  `~/.ssh/known_hosts.ldt4-pre-173-20260721T202154Z`. Strict SSH returns
  `NETWATCH-01` / `.173`.

- **10GbE**: `en8` = **10.1.10.54**, MTU **9000**, media 10Gbase-T. This is the
  **Aquantia adapter physically moved off nagatha**, so nagatha's 10GbE is now a
  *different* NIC at **10.1.10.92** (also MTU 9000). Any doc naming
  "Aquantia @ .54 on nagatha" is stale.
- **⚠ THE MULTI-NIC ROUTING TRAP (cost ~1h).** `q` has THREE IPs on
  10.1.10.0/24 — `en0` (1GbE, .221), `en1` (Wi-Fi, .108), `en8` (10GbE, .54) —
  and macOS routes the subnet via the highest-ranked **network service**, not by
  which IP "matches". `en0` outranked `en8`, so **every benchmark would have run
  over gigabit**. Fixed by promoting the service that owns `en8` — confusingly
  named **"Thunderbolt Ethernet Slot 3"** — to rank 1
  (`sudo networksetup -ordernetworkservices …`). It has the same router
  (10.1.10.1), so `q` keeps its internet.
- **DO NOT "fix" this with a host route.**
  `sudo route -n add -host 10.1.10.177 -interface en8` on a *directly-connected*
  subnet installs a next hop of **the interface's own MAC** — a black hole. It
  drops 100% of packets while `route -n get` still cheerfully reports
  `interface: en8`. Verify with `arp -n <peer>`: the MAC must be the PEER's, not
  `q`'s (`00:01:d2:19:04:a3`).
- **An ssh transfer CANNOT verify this link.** ssh caps at ~79 MB/s on this path
  (nagatha's known-good 10GbE scores the same 79), which is *below* the gigabit
  ceiling — so a degraded link and a healthy one look identical through it. Use
  `ifconfig en8 | grep media` (the PHY's negotiated rate) and blit's own
  `wm_tcp_large` time (~908 ms for 1 GiB = 10GbE; ~10 s = 1GbE).
- **MSS is directional on this pair (rechecked 2026-07-15):** five live
  `getsockopt(TCP_MAXSEG)` samples were **8948 q→netwatch-01** and five were
  **8960 netwatch-01→q**, with local sources `.54` and `.177` respectively.
  A rig gate must pin the observed value for each direction rather than repeat
  the older shorthand “8948 both directions.” On q, the saved host key for the
  bare `netwatch-01` name is stale; the pinned numeric `10.1.10.177` entry is
  valid and is the benchmark control endpoint. Do not bypass host-key checking.
- **Staged**: repo clone at `~/Dev/blit_v2_f35702a` (detached `f35702a`, cloned
  from the LOCAL gitea — `q` *is* the gitea host); `target/release/{blit,blit-daemon}`
  arm64 copied from nagatha (embed-verified `+f35702a`); old client at
  `~/blit-bench-work/bins/blit-0f922de`; fixtures in `~/blit-bench-work`.
  NOPASSWD `/usr/sbin/purge` granted (`/etc/sudoers.d/blit-bench`, mode 0440 —
  `visudo -c` rejects any other mode). ssh key authorized on netwatch-01 in
  **`C:\ProgramData\ssh\administrators_authorized_keys`** (michael is an admin
  there, so the per-user file is ignored). macOS firewall is OFF on `q`.
- **ldt-4 harness staged 2026-07-21:** complete-history retained bundle
  `~/blit-ldt4-stage-d53b5fd.bundle` has SHA-256
  `08cb33935c66cf55e04a88ff0ff3a2a2633d4a443d3f8f7856808ee5303d0af7`
  on nagatha and q. New retained checkout
  `~/Dev/blit_v2_harness_d53b5fd` is clean, detached at exact `d53b5fd`, and
  has the expected 1,930-commit target history. q's native Bash 3.2 no-SSH
  harness self-test passes all 96 arms. This stages no new product binary; the
  registered accepted `406a7e5` artifacts remain the run payload.
- **`q` RUNS GITEA** (it is `origin`, `http://q:3000`). It idles cheaply, but
  **do not push to `origin` during a benchmark session**.

## THE MAC IS A BENCH END — keep it quiet (recorded 2026-07-13, learned the hard way)

**A rig-W (Mac↔Windows) benchmark requires a QUIET Mac.** The Mac is not a
neutral driver: it runs the client in `mac_init` arms and serves the daemon in
`win_init` arms. Any heavy Mac process contaminates the measurement — and
**asymmetrically**, because `mac_init` runs the client locally while `win_init`
runs it on Windows. CPU starvation therefore **inflates Δ and MANUFACTURES P1**,
the very finding under test. (Same shape as the 2026-07-13 durability
retraction: a cost billed to one arm and not the other.)

- **This actually happened** (2026-07-13, first A-B-B-A attempt): codex jobs ran
  on the Mac for the whole 53-minute window. The same-MTU replicates caught it —
  `wm_tcp_large` read 911 ms in S1 and 1847 ms in S4 **at the same MTU**, and the
  noise floor came out at 473 ms, larger than the 325 ms gap under test. The run
  was discarded. Without the replicates it would have looked clean and been
  reported.
- **Offenders**: `codex` (the review loop!), `cargo`/`rustc`, Spotlight
  reindexing, any build. **The review loop and a rig-W session cannot run at the
  same time** — sequence them.
- **The runner gates on this**: it refuses to start a session while
  codex/cargo/rustc is running, and records `load1` per session so contamination
  is visible in the evidence rather than hidden in the noise.
- **NEVER blanket-kill to get quiet.** `pkill -f codex` killed the OWNER's own
  codex sessions (2026-07-13). Ask the owner to clear the machine, or kill only
  PIDs you can prove you launched.
- The Linux rig (magneto↔skippy) does not involve the Mac and has no such
  constraint — but P1's cell only exists on the Mac↔Windows pairing, so it
  cannot substitute.

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
