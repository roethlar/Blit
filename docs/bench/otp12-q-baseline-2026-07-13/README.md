# P1 REPRODUCES ON A SECOND MAC — rig `q` ↔ netwatch-01, MTU 9000 (2026-07-13)

**Status**: Evidence (recorded). **This README declares nothing** — it records
what was measured.

**Why this session exists**: P1 (`wm_tcp_mixed` invariance failure) had **only
ever been measured on one Mac** — nagatha. Every live hypothesis in
`docs/plan/OTP12_PERF_FINDINGS.md` (H1, H5, H6, H7) assumes P1 is a property of
the **macOS↔Windows pairing**. *That assumption had never been tested.* This
session tests it on a different Mac.

**Rig (NEW — `q`)**: Apple M4 Mac mini, 16 GB, macOS 26.5.2, arm64. 10GbE =
`en8` (the Aquantia adapter physically moved from nagatha), **10.1.10.54**,
**MTU 9000**, negotiated **MSS 8948**. Peer: netwatch-01 (`10.1.10.177`,
MTU 9000). Harness `scripts/bench_otp12_win.sh` @ `f35702a`, RUNS=8, ABBA,
pair-void, cold caches + drain both ends, destination-keyed durability.
Binaries: `f35702a` both ends (embed-verified), arm64 copied from nagatha.

## Result

| cell | mac_init | win_init | ratio | verdict |
|---|---:|---:|---:|---|
| **`wm_tcp_mixed`** (P1's cell) | **1093** | **789** | **1.385** | **FAIL** |
| `mw_tcp_mixed` (opposite direction) | 1688 | 1618 | 1.043 | PASS |
| `wm_grpc_mixed` (opposite carrier) | 1254 | 1230 | 1.020 | PASS |
| `wm_tcp_large` (opposite fixture) | 909 | 907 | 1.002 | PASS |

**P1 reproduces on a different Mac, harder** (1.385 here; 1.237 and 1.300 on
nagatha — different hardware, so the magnitudes are not comparable, but the
*failure* is).

### The controls ARE the noise model, and they are tight

All three control cells passed at **1.002–1.043** *in the same session, same
rig, same conditions*. So this rig's asymmetry noise is ~**2–4%** — and P1 is
**38.5%**, an order of magnitude outside it. This is a **within-session**
comparison, which is the only kind this project has learned to trust
(`.agents/machines.md`, and the local-rig bi-stability in
`docs/bench/win-local-ab-2026-07-13/`).

### What it establishes

- **P1 is NOT a nagatha artifact.** Different Mac, different CPU and disk (M4
  mini vs nagatha), same adapter → the failure follows the **platform pairing**,
  not the machine. The assumption under H1/H5/H6/H7 survives its first real test.
- **The signature is unchanged and sharp**: TCP only (gRPC passes at 1.020),
  `mixed` only (`large` passes at 1.002), destination-initiator only (the
  reverse direction passes at 1.043).
- **P1 FAILS AT MTU 9000.** This session ran at jumbo. So **jumbo does not
  dissolve P1** — the premise behind `docs/STATE.md`'s Queue 1a.

### What it does NOT establish

- **It does not quantify MTU's contribution.** "Jumbo doesn't fix it" is not
  "MTU contributes nothing" — MTU could still be a CONFIRMED CONTRIBUTING cause
  on the parent plan's own 20–50% band while P1 still fails its 1.10 bar. That
  requires the matched 1500 arm, which is what the counterbalanced A-B-B-A run
  in `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` (rev 3) exists
  to measure. **This session is NOT that experiment** — it is a single
  condition, with no same-MTU replicate and therefore no session-level noise
  floor.
- **It is not acceptance evidence.** `pf-final` owns acceptance, and its rows
  are pre-registered on the *designated* rig.

## Rig-validity checks (all passed before any timed run)

The link was proven, not assumed — three prior instruments lied tonight:

- `en8` media: **10Gbase-T**, full-duplex, active; MTU 9000.
- Route to netwatch-01 → `en8`, with netwatch-01's **real** MAC in ARP.
  (An earlier `route add -host … -interface en8` created a **black hole**: the
  next hop resolved to `q`'s *own* NIC MAC, 100% packet loss, while
  `route -n get` still reported `interface: en8`. Fixed by promoting the
  10GbE's network *service* above the 1GbE's instead.)
- Client socket source = **10.1.10.54**, MSS = **8948**.
- **Throughput**: `wm_tcp_large` moves 1 GiB in **~908 ms ≈ 1.18 GB/s** —
  saturating 10GbE (a 1GbE link would need ~10 s). Note an ssh-based transfer
  test **cannot** detect a 1GbE fallback here: ssh caps at ~79 MB/s on this path
  regardless (nagatha's known-good 10GbE scores the same 79), so it is an ssh
  test, not a link test.

## Files

`summary.csv`, `verdicts.csv`, `runs.csv` (64 timed runs, 0 voided),
`meta.csv`, `staging-manifest.txt` (7 hashes).
