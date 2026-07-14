# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 1**. **No data exists yet.** This file is
committed BEFORE the data so the decision rule cannot be authored around the
numbers (the pf-0 discipline; `docs/bench/otp12-jumbo-win-2026-07-13/`).

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1), queue
item 1(ii). This is the **missing cell of the 2×2**, and unlike pf-0 it is not
an environmental control — **it discriminates a hypothesis.**

## The question, and why it is decisive

P1 = the destination-initiated TCP×mixed arm pays ~25–38%. It has only ever been
measured on **macOS↔Windows**. The 2×2:

| pair | P1? | evidence |
|---|---|---|
| Linux↔Linux (magneto↔skippy) | **NO** (8/8 PASS) | `docs/bench/otp12-perf-2026-07-13/` |
| macOS↔Windows (rig W) | **YES** (1.237 / 1.300 / 1.385 / 1.362) | 12b, 12c, q-baseline, pf-0 |
| **macOS↔macOS** | **UNTESTED** | *this run* |

**H1 accuses the WINDOWS side**: it names the source-side `Accept` branch /
per-epoch accept-dial round-trips, with Windows as the accepting source in P1's
slow arm. So:

- **P1 REPRODUCES macOS↔macOS** → the failure needs **no Windows peer**. It is
  macOS-side (or pure layout), and **H1 as written DIES** — it accuses a machine
  that is not in the rig.
- **P1 VANISHES macOS↔macOS** → the failure **requires the Windows peer**, which
  is what H1 predicts, and H1 is **strongly supported** (not confirmed — see
  "What this does not establish").

Either way a live hypothesis moves. That is why this runs before pf-1: pf-1 would
otherwise instrument a mechanism the rig can already exonerate or convict.

## Rig — both ends macOS, and the platform terms cancel

- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000,
  10Gbase-T.
- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000,
  10Gbase-T.
- **Build (same-build rule, D-2026-07-05-2)**: `f35702a`, **clean `+f35702a`
  embedded on all four binaries** (verified; the `.dirty` form is rejected). This
  is the **cutover sha behind every P1 measurement** (12c, q-baseline, pf-0), so
  the comparison is apples-to-apples. Note HEAD is **not** code-identical to
  `f35702a` (`crates/blit-app/src/endpoints.rs` + a test changed since), so the
  build is pinned deliberately, not taken from HEAD.

**Endpoint asymmetry is NOT a confound, and this is the load-bearing argument.**
nagatha and `q` are different machines (`q` is the faster). An invariance
comparison **holds both endpoints and the data direction fixed and varies only
which host's CLI initiates** — so machine asymmetry cancels *within* each cell.
This is the same argument the Linux rig rests on (`bench_otp12pf_linux.sh`
header) and it is why the rig does not need matched hardware.

## Cells

Grammar `<nq|qn>_<carrier>_<fixture>` — `nq_*` = data **nagatha→q**, `qn_*` = data
**q→nagatha**. Arms per cell, the ONLY variable:

- `srcinit` — the SOURCE host's CLI pushes (source-initiated)
- `destinit` — the DEST host's CLI pulls (destination-initiated)

    CELLS = nq_tcp_mixed, qn_tcp_mixed     <- THE MEASURAND (P1's shape, both directions)
            nq_grpc_mixed, qn_grpc_mixed   <- carrier control (P1 is TCP-only)
            nq_tcp_large,  qn_tcp_large    <- fixture control (P1 is mixed-only)

`RUNS=8`, ABBA-counterbalanced, pair-void.

**Both data directions are measured deliberately.** P1 is a claim about the
*layout* (destination-initiated is slow), not about a host. If it is layout, it
must appear in **both** directions on a symmetric-OS rig. If it appears in only
one direction, that is a *machine* effect, not P1 — and the rule below says so
rather than letting it be read as a reproduction.

## The noise model — MEASURED in-session, never assumed (the pf-0 lesson)

The three control cells (`grpc_mixed`, `tcp_large`, both directions) give this
rig's **within-session asymmetry noise band `N`**: `N = max |ratio − 1|` over all
four control-cell ratios in this session. Within-session is the only comparison
this project trusts (`.agents/machines.md`; the q-baseline's controls read
1.002–1.043, bounding rig-W noise at 2–4%).

**No threshold below is invented; every one is expressed against `N` or against
the plan's existing 1.10 bar.**

## Decision rule — pre-registered, with the ambiguous cases named

Invariance is graded with the harness's **exact integer arithmetic**
(`10·hi ≤ 11·lo`), never the printed 3-decimal ratio. Let
`Δ(cell) = destinit_median − srcinit_median` (positive = destination-initiated is
slower = P1's direction).

Evaluated in order:

1. **RIG-VALIDITY GATE (first).** If any control cell FAILS the 1.10 bar, the rig
   is not measuring cleanly and the session is **VOID** — no verdict. (A rig whose
   gRPC control fails cannot adjudicate a TCP-only claim.)
2. **REPRODUCES → H1 DIES.** Both TCP×mixed cells FAIL the 1.10 bar **with Δ > 0**
   (destination-initiated slower — P1's direction), while all controls PASS.
3. **VANISHES → H1 STRONGLY SUPPORTED.** Both TCP×mixed cells PASS the 1.10 bar
   **and** each |ratio − 1| ≤ `N` (the asymmetry is inside the rig's own control
   noise, i.e. not merely sub-bar but *absent*).
4. **ONE-DIRECTION-ONLY → NOT a reproduction.** Exactly one TCP×mixed cell fails
   while the other passes. P1 is a layout claim and must be direction-symmetric on
   a symmetric-OS rig, so this is reported as a **MACHINE asymmetry** (nagatha and
   `q` differ) and is **INCONCLUSIVE for H1**. It is registered here precisely so
   it cannot be spun as a half-reproduction afterwards.
5. **SUB-BAR BUT REAL → PARTIAL.** Both cells PASS the 1.10 bar but some
   |ratio − 1| > `N` in P1's direction: a real asymmetry that does not reach the
   bar. Reported as **PARTIAL — H1 weakened, not killed**, with the effect size
   stated against rig W's `Δ_P1 ≈ 230 ms`. pf-1 then owns it.
6. **INVERSION.** Δ < 0 beyond `N` (source-initiated is the slower arm). Reported
   as an inversion — a new finding, **never** banked as "P1 absent".

**Grade the DISTRIBUTION, not just the median (pf-0's bistability finding).**
pf-0 found the rig-W fast arm bimodal (~730/~840 ms clusters), where the
mode *mixture* moved the median by 72 ms at constant MTU. Every cell here reports
min / median / spread and all 8 runs per arm; **a verdict that flips when the
per-run distribution is inspected must be reported as unstable, not resolved.**

## Gates (a run that misses any of these is VOID, not "close enough")

Inherited from otp-12/pf-0, plus **two gates pf-0 proved were missing**:

- **QUIESCENCE, BOTH MACS (new).** Refuse to start if `codex`, `cargo` or `rustc`
  is running on **either** Mac. Both are bench ends here — nagatha is no longer
  merely the driver. Record `load1` on both at session start and end.
- **TIME MACHINE, BOTH MACS (new — the hole pf-0 found).** The existing quiet-gate
  catches only codex/cargo/rustc and **would have sailed straight through a Time
  Machine backup**, which fired 1 minute before pf-0's run and repeats hourly.
  Refuse to start if a backup is running; **warn loudly** if autobackup is enabled
  (one destination is a network share on `skippy` — the same 10 GbE fabric).
- **Cold caches both ends every run** — `sudo -n /usr/sbin/purge` (NOPASSWD
  granted on both). A failed purge **VOIDS the pair**; runs must never read warm.
- **Destination durability, keyed by the DESTINATION host, never the verb** — the
  macOS per-file `fsync` walk, applied identically to both arms. (The otp-2w rule,
  re-learned the hard way: a sync inside the initiator's bracket charges the pull
  arm for writeback the push arm gets free, and *manufactures* invariance
  failures — including on the gRPC control.)
- **Provenance**: every binary embeds a **clean** `+f35702a` (and not
  `+f35702a.dirty`); sha256 staging manifest recorded.
- **Link validity, measured not assumed** (`.agents/machines.md`): `en11`/`en8`
  media = 10Gbase-T; ARP for the peer resolves to the **peer's** MAC (not our own
  — the black-hole trap); and a real 1 GiB blit transfer lands at ~0.9–1.2 s. An
  ssh throughput test **cannot** validate this link (it caps at ~79 MB/s either
  way) and is not used.
- Fresh never-seen destination per run; ABBA interleave; nonzero exit or undrained
  window VOIDS the pair (cap `2×RUNS`, then INCOMPLETE).

## What this does NOT establish (stated before the data, not after)

- **A reproduction kills H1 *as written*; it does not name the mechanism.** It
  would prove the failure needs no Windows peer — it would not say *which* macOS
  or layout path pays. pf-1 still owns attribution.
- **A vanish does not CONFIRM H1**, it supports it. "Requires a Windows peer" is
  consistent with H1's accept-branch story and with other Windows-side mechanisms
  (H6/H7's shapes are P2's, but a Windows-specific cost is not H1's exclusive
  property). Confirmation needs pf-1's counterfactual, not this rig.
- **This rig cannot measure P2.** P2 is a converge bar (new vs OLD build) and no
  old-build pair is staged on the Macs. P2 on the Linux rig is a separate,
  outstanding experiment.
- **Absence of P1 here is not absence on rig W.** The rig-W failure is measured
  and reproduced four times; this run cannot retract it.

## The runs

From the `f35702a` clone, driven from `q` (so nagatha carries only the
daemon/client work it must):

```sh
EXPECT_SHA=f35702a RUNS=8 \
  CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large \
  bash scripts/bench_otp12pf_mac.sh
```

Harness: `scripts/bench_otp12pf_mac.sh` (new; the same-OS shape of
`bench_otp12pf_linux.sh` with macOS cold-cache/durability and the two new Mac
gates). It **computes; it declares nothing** — the verdict is read off the rule
above.
