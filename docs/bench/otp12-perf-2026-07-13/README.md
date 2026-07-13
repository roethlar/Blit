# otp-12pf probe — P1 reproduces on a same-OS rig (magneto↔skippy, 2026-07-13)

**Status**: Probe record (NOT evidence-grade — see Limits). **Purpose**:
break the platform-vs-code confound that blocked
`docs/plan/OTP12_PERF_FINDINGS.md` pf-1. **This record declares
nothing**; acceptance belongs to the owner at otp-13.

## The question

P1 (destination-initiated TCP mixed transfers pay ~25–30%) was measured
only on rig W (Mac↔Windows). On a **two-host** rig, host identity IS
role: in the failing arm the destination is the Mac (which dials) *and*
the source is Windows (which accepts). Those cannot be separated, so the
finding was consistent with two very different stories:

1. **code** — the destination-initiator layout is genuinely slower
   (H1/H5/H6), or
2. **platform residue** — a macOS/Windows TCP-stack or write-path
   artifact, which D-2026-07-12-1 would let the owner accept as
   satisfied.

Rig juggling cannot decide it: the fleet has no same-OS,
performance-matched, real-network pair — one 10 GbE Mac; **zoey's CPU is
too slow to partner skippy** (owner, 2026-07-13 — a zoey endpoint
bottlenecks and MASKS the effect); magneto was recorded build-only.

## The rig (new)

Owner offered magneto as a bench end and confirmed it "should be fast
enough to saturate 10 GbE where zoey is definitely not".

| host | CPU | disk | link |
|---|---|---|---|
| **skippy** | AMD EPYC (32 threads) | ZFS `generic-pool` | 10 GbE (10.1.10.143) |
| **magneto** | Intel, power-efficient (4 cores) | WD SN850 NVMe (Gen4) | 10 GbE (10.1.10.10) |

**Linux on both ends** — so a reproduction here cannot be a macOS or
Windows artifact. Endpoints need not match each other: an invariance
comparison holds both endpoints fixed and varies only the initiator, so
endpoint asymmetry cancels *within* each pair (`ONE_TRANSFER_PATH.md`
acceptance criterion 1). What zoey failed was the absolute-speed floor
(it would mask the effect); magneto clears it.

Binaries: the same `+f35702a` x86_64-musl build both ends (same-build
handshake, D-2026-07-05-2, would refuse otherwise). Fixture: `mixed`
(512 MiB + 5000 × 2 KiB = 547,110,912 B / 5001 files), shape-verified on
both hosts.

## Result — P1 REPRODUCES with no Mac and no Windows in the path

Wall time (ms), CLI-side `/proc/uptime` bracket incl. `sync`, 3 runs:

| data direction | arm | runs | median |
|---|---|---|---|
| skippy → magneto | source-initiated (skippy pushes) | 900, 980, 950 | **950** |
| skippy → magneto | **destination-initiated (magneto pulls)** | 1450, 1690, 1840 | **1690** |
| magneto → skippy | source-initiated (magneto pushes) | 1320, 1500, 1340 | **1340** |
| magneto → skippy | **destination-initiated (skippy pulls)** | 1540, 5100, 6370 | unstable |

**skippy→magneto: destination-initiated is 1.78× source-initiated**
(1690 / 950) — the same shape as rig W's `wm_tcp_mixed`, and **larger**
than the 1.300 measured there.

## Reading (numbers only; no adjudication)

- **The confound is broken, and it breaks toward CODE.** With Linux on
  both ends there is no macOS/Windows asymmetry left to attribute the
  gap to. The **platform-residue explanation for P1 is dead**;
  D-2026-07-12-1's escape hatch does not apply to it. P1 is a property
  of blit's layout — H1/H5/H6 remain live and a fix is mandatory for the
  parent plan's headline invariance criterion.
- Viability, answered: magneto's wall times (0.9–1.8 s) sit alongside rig
  W's (~1.2–1.5 s). It is a usable bench end, not a second zoey.
- The magneto→skippy destination-initiated arm is **unstable** (1540 vs
  6370) and unexplained. Recorded, not interpreted; the real harness
  (cold caches, drains, ABBA, pair-void) must resolve it before any
  number from that arm is cited.

## Limits — why this is a probe and not evidence

- **No cold caches.** magneto has no `NOPASSWD` `drop_caches` grant, so
  neither end was purged between runs. (Within a data direction both arms
  read the same source file on the same host, so cache state is broadly
  comparable *within* the comparison that matters — but this is an
  argument, not a control.)
- No disk drains, no ABBA interleave, no pair-void rule, RUNS=3, no
  staging manifest, exit codes not gated. None of the otp-12 methodology.
- Therefore: **no row here may be cited for acceptance**, and pf-final's
  final-build rule voids it regardless.

## To promote this to pf-1 evidence

1. `NOPASSWD` sudoers on magneto, matching skippy's grant:
   `michael ALL=(root) NOPASSWD: /usr/bin/tee /proc/sys/vm/drop_caches`
2. Torrent services quiesced for the session (owner offered).
3. A harness pass in the otp-12 shape (cold both ends, drain the
   destination, ABBA, RUNS≥4, pair-void, staging manifest) — the
   `bench_otp12_delegated.sh` plumbing already covers a Linux↔Linux pair
   and is the natural base.

## Reproduction

`scripts/` has no committed harness for this pair yet (the probe ran from
a scratch script). Steps: stage the `+f35702a` musl `blit`/`blit-daemon`
on magneto; daemon on each host (port 9031, module `bench` → a scratch
dir on the NVMe / the pool); push the `mixed` fixture to magneto once
(untimed); then per arm run the CLI on the initiating host, bracketing
`copy … --yes` + `sync` with `/proc/uptime` reads in one ssh shell.
