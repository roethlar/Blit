# otp-12 rig-W MTU experiment — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 3**. **No data exists yet.**
Codex round 1 (of `35b9620`): NOT READY, 4 BLOCKER + 3 HIGH → 7/7 accepted.
Codex round 2 (of `7921adc`): **NOT READY again**, 5 BLOCKER + 3 HIGH → 8/8
accepted. Adjudications: `.review/results/pf-0-prereg{,-r2}.gpt-verdict.md`.
This file is committed BEFORE the data so the decision rule cannot be authored
around the numbers.

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (**Active**, D-2026-07-13-1).
An ENVIRONMENTAL experiment, not a code counterfactual; not one of H1–H7. **A
PASS licenses evidence for a plan amendment only** — it cannot reshape pf-1,
re-baseline, or close P1/P2 without a reviewed amendment.

## What round 2 broke, and the one fix that repairs most of it

Round 2's blockers all trace to **one root defect**: *every threshold in
revision 2 was arbitrary, because the design had no noise model.* `RUNS=8`
estimates variance **within** a session, while the entire MTU comparison is
**between** sessions — and MTU was perfectly aliased with session order.

Codex's counterexample, which revision 2's guards all passed:

> From the 1500 medians `(win, mac) = (939, 1221)`, a shared **985 ms floor**
> at 9000 gives ratio **1.000** and `r = 100%`; the fast arm regresses only
> **4.9%** (inside the 5% tolerance) and "both arms slower" is false. A pure
> masking artifact scores a perfect result.

A **measured** noise floor kills that counterexample (a 46 ms fast-arm
regression is either inside the rig's noise or it is not — that is an
empirical question, not a number to invent). So the design now **measures
session-to-session noise** rather than assuming it.

## Design — counterbalanced, with same-MTU replicates (round-2 F1)

**Four sessions, order A-B-B-A**:

| session | MTU | role |
|---|---|---|
| S1 | **9000** | condition A |
| S2 | **1500** | condition B |
| S3 | **1500** | condition B **replicate** |
| S4 | **9000** | condition A **replicate** |

- **MTU is no longer aliased with order** (A first *and* last).
- **The same-MTU pairs are the noise model.** S1↔S4 (maximally separated in
  time — the conservative estimate) and S2↔S3 (adjacent) give the
  session-to-session variability of every quantity the decision rule uses,
  **with MTU held constant**. This is the "sham repeat" round 2 asked for.

**⚠ RIG CHANGED (revision 4, 2026-07-13) — the Mac end is now `q`.** Revisions
1–3 named nagatha with "Aquantia @ 10.1.10.54". **That adapter is physically in
`q` now**; nagatha's 10GbE is a different NIC at 10.1.10.92. The rig-W Mac end
is the **M4 Mac mini `q`** (10.1.10.54, MTU 9000, MSS 8948) — quiet and
dedicated, which matters because **the Mac is a bench END and the codex review
loop cannot run during a session** (`.agents/machines.md`; a 53-minute A-B-B-A
attempt was destroyed exactly this way and discarded).

**The design is rig-independent** — A-B-B-A compares MTU conditions *within one
rig* and derives its noise floor from same-MTU replicates *on that rig* — so
moving the Mac end does not weaken it. But the rig must be named honestly, and
the four sessions must all run on `q`.

**`q` is a VALID rig for this experiment, measured not assumed**
(`docs/bench/otp12-q-baseline-2026-07-13/`): P1 **reproduces** there —
`wm_tcp_mixed` **1.385 FAIL** at MTU 9000 — while all three control cells PASS
at **1.002–1.043** in the same session, bounding the rig's asymmetry noise at
~2–4%. That single-condition baseline is **not** this experiment (no same-MTU
replicate, so no session-level noise floor); it establishes that the gap under
study exists on this rig.

Held constant across all four: sha `f35702a` (clone `~/Dev/blit_v2_f35702a` on
`q`, binaries embed-verified), old arm `0f922de`, the **`q` Mac end**
(10.1.10.54, `en8`), harness, fixtures, `CELLS`, `RUNS=8`.

`CELLS=wm_tcp_mixed,wm_tcp_large,mw_tcp_mixed,wm_grpc_mixed`

- `wm_tcp_mixed` — **P1's cell; the measurand**
- `wm_tcp_large` — bulk-packet **context** (see below; NOT a gate)
- `mw_tcp_mixed` — opposite-direction control (1.044 at 1500)
- `wm_grpc_mixed` — opposite-carrier control (1.021 at 1500)

**Declared omission (round-2 F8, do not discover this in the output):** these
four cells have no block-1 counterparts in `CELLS`, so `compute_verdicts` will
emit **`NO-SAME-SESSION-REF`** for their converge rows
(`bench_otp12_win.sh:715`) and **no discriminator-gap rows at all**
(`bench_otp12_win.sh:743` needs all four contributing cells). That is
**expected and acceptable**: this is an *experiment*, not an acceptance run —
acceptance is `pf-final`'s job, and the measurand here is the **invariance**
row, which is computed entirely within one session and needs no counterpart.

## Instrument — what is measured, and what is NOT

| check | method | result |
|---|---|---|
| interface MTU, both ends | `ifconfig` / `Get-NetIPInterface` | 9000 / 9000 (NIC `Jumbo Packet = 9014`) |
| L2 path, both ways | DF ping (Win→Mac 8972 B; Linux→both 8972 B; Mac→Win 8000 B — the macOS 8192 raw-socket cap forbids more) | OK |
| **negotiated MSS** | `getsockopt(TCP_MAXSEG)` + Linux `ss -ti` | **8948** each way (1448 at MTU 1500) |

**Per-session MSS gate (round-2 F7).** A `getsockopt` sample proves **one
socket at one instant**; it cannot prove the condition held for a whole
session. So each session records the MSS **at start AND at end**, and a
session whose MSS is not the expected value (8948 / 1448) at **both** points
is **VOID**. This does not prove every transfer connection individually — that
would need a harness change — and the residual is stated rather than hidden.

**WHAT THE MSS DOES NOT ESTABLISH (round-1 F5, sharpened by round-2 F5).**
8948 is the **ceiling**, not the **fill**. Whether blit's data plane emits
full-size segments is **unmeasured** — application write boundaries, Nagle,
and record framing can all leave segments short. Therefore:

- The segment counts below are **upper bounds assuming full fill**, not
  measured segment counts.
- **A null result supports exactly one conclusion**: *"raising the MTU did not
  improve these cells under the observed packetization."* It does **NOT**
  establish that per-packet cost is irrelevant to blit — that inference
  requires the fill measurement we do not have. (Revision 2 asserted both and
  contradicted itself; round-2 F5.)

**A discarded instrument**: Windows `Get-NetAdapterStatistics` reported
**10 680 received bytes per "packet"** during a 1 GiB transfer — larger than a
9014-byte frame — so that NIC coalesces on receive despite `RSC
IPv4Enabled: False`. NIC packet counters cannot discriminate 1500 from 9000
here and are not used.

**Premise, corrected (round-1 F6; arithmetic re-verified by round 2).** At
MSS 1448, assuming full fill: `large` ≈ **741 500** segments, `mixed` ≈
**377 800**, `small` ≈ **28 300**. **`large` is the packet-heaviest fixture,
by ~2×** — *not* `mixed`, as `docs/STATE.md` and revision 1 both claimed.
`mixed` is P1's cell because that is where the failure was **observed**.

## The noise model (computed from the data, before any verdict)

For each quantity the rule uses, the **noise floor `N`** is the larger of the
two same-MTU replicate differences — the conservative choice:

    Δ(session)   = median(mac_init) − median(win_init)     [wm_tcp_mixed]
    N_Δ          = max( |Δ_S1 − Δ_S4| , |Δ_S2 − Δ_S3| )
    N_arm        = max over {win_init, mac_init} of the same-MTU |differences|

`Δ_9000` = mean(Δ_S1, Δ_S4); `Δ_1500` = mean(Δ_S2, Δ_S3).

**Every threshold below is expressed against `N`. No invented tolerances.**

## Decision rule — pre-registered, with an explicit domain (round-2 F2)

**Domain guard, evaluated FIRST.** If `Δ_1500 ≤ N_Δ` — the gap we are trying
to explain is not reliably present above this rig's own session noise — the
experiment is **INCONCLUSIVE** and no recovery is computed. (The parent says
plainly that a `Δ ≈ 0` proves nothing: `OTP12_PERF_FINDINGS.md:498`.) This is
a real possible outcome and it is registered, not a formality.

Otherwise the MTU recovery is

    r = (Δ_1500 − Δ_9000) / Δ_1500

graded on the parent's **uniform pre-registered scale**
(`OTP12_PERF_FINDINGS.md:516`):

- `r ≥ 50%` → **CONFIRMED DOMINANT**
- `20% ≤ r < 50%` → **CONFIRMED CONTRIBUTING**
- `r < 20%` → **KILLED** as a material cause

**Edge cases, registered (round-2 F2 — revision 2 left these undefined):**

- **`Δ_9000 < 0`** (the destination-initiated arm becomes the *faster* one):
  report as **INVERSION**, not as `r > 100%`. An inversion is a *new*
  invariance failure in the opposite direction and must be reported as such,
  not banked as a win.
- **`r > 100%`** by any other route: report the raw arms; do not claim
  >100% recovery of a gap.
- **`Δ_9000 ≤ N_Δ`**: the residual gap is inside the noise — state that the
  gap is *not distinguishable from zero*, rather than claiming exact closure.

Reported **separately** (different questions): **does P1 pass at jumbo?**
`wm_tcp_mixed` invariance ≤ 1.10 using the harness's exact integer arithmetic
`10·hi ≤ 11·lo` (`bench_otp12_win.sh:668`) — not the printed 3-decimal ratio.

### Masking guard — rebuilt on the measured noise (round-2 F4)

Revision 2's guards passed a shared-floor artifact. The replacement:

- **Fast-arm guard**: `win_init` at 9000 must not regress against `win_init` at
  1500 **by more than `N_arm`**. Codex's counterexample (939 → 985, a 46 ms
  regression) now **FAILS unless 46 ms is genuinely inside this rig's measured
  session noise** — which the same-MTU replicates decide, and which is exactly
  the empirical question revision 2 answered by inventing a 5% tolerance.
- **Convergence target**: the slow arm must approach the fast arm's **1500
  value**, not a shared floor. `mac_9000 ≤ win_1500 + N_arm`.
- **Both-arms-slower**: if both arms at 9000 exceed their 1500 counterparts by
  more than `N_arm` while the ratio improves, that is **bottleneck
  compression** and is reported as a **degradation**, never as a P1 pass.

A ratio improvement that satisfies none of these is **not** a fix.

### `wm_tcp_large` is CONTEXT, not a gate (round-2 F6)

Revision 2 made `large` a falsifier: "if `large` does not improve ≥5%, any
movement in `mixed` is not an MTU effect." **That is unsound and is
withdrawn.** Codex's counterexample: `(939,1221) → (939,1000)` gives
`r = 78.4%` and invariance 1.065 — a real, plausible MTU effect — while
`large`, being **throughput-bound** rather than packet-bound, need not move at
all. The falsifier would have killed a true result. The 5% threshold also had
no noise basis.

**Replacement**: `large`'s change is **reported as corroborating context**. If
it improves, it supports a per-packet-cost mechanism. If it does not, that is
**compatible** with `mixed` being packet-sensitive while `large` is
throughput-bound, and it **does not void** the `mixed` result.

## Verdict rows VOID at jumbo (round-1 F7 — round 2 confirms the inventory is now complete)

The harness grades against `otp2w-baseline-2026-07-10/summary.csv`
(`bench_otp12_win.sh:105`), **recorded at MTU 1500**. In the 9000 condition:

- **VOID**: every `converge … old_committed` row, every
  `cross … min_old_committed` row, **and every block-1 `combined` row** (which
  is PASS only if the committed leg passes — `bench_otp12_win.sh:697-702`).
- **SOUND**: **invariance** rows (new-vs-new, one session — the measurand) and
  `converge … old_session` rows. Both are MTU-matched by construction.

**None of this experiment's conclusions use the committed baseline.**

**Consequence, widened (round-2 F9).** Revision 2 said only P2 was blocked. In
fact **P1's `pull_tcp_mixed` bar and the parent's global rule also require the
committed reference** (`OTP12_PERF_FINDINGS.md:541`, `:553`). So if the fleet
stays at jumbo, **formal acceptance of P1 and the global rule — not merely
P2 — requires a re-recorded committed baseline** and a fixed-reference harness
change. That is a plan amendment; it goes through the loop and is not assumed
here.

## Residual limitations (stated, not hidden)

- **Segment fill is unmeasured** (above). A null cannot distinguish "blit
  leaves segments short" from "per-packet cost does not matter".
- **The MSS gate is start-and-end, not per-connection.** A mid-session change
  that reverted before the end would go undetected.
- `netwatch-01` intermittently fails to resolve (DHCP/mDNS); every session
  passes `WIN_SSH=michael@10.1.10.177` explicitly so a resolution failure
  cannot silently retarget a run.

## The runs

From `/Users/michael/Dev/blit_v2_f35702a` (clean, `f35702a`), for each session
in order **A(9000), B(1500), B(1500), A(9000)** — flipping the Windows MTU
between conditions and recording MSS at session start and end:

```sh
# Windows MTU: Set-NetIPInterface -InterfaceAlias Ethernet -NlMtu <9000|1500>
WIN_SSH=michael@10.1.10.177 MAC_HOST=10.1.10.54 \
  OLD_CLIENT_PROVENANCE_BY_BUILD=1 RUNS=8 \
  CELLS=wm_tcp_mixed,wm_tcp_large,mw_tcp_mixed,wm_grpc_mixed \
  bash scripts/bench_otp12_win.sh
# restore MTU 9000 afterwards
```

Preflight verified 2026-07-13: worktree clean at `f35702a`; binaries embed
`+f35702a`; `D:\blit-test\bins\{0f922de,f35702a}` staged; no stale daemon on
either host; staging manifest recorded (7 hashes); MSS re-checked live at 8948.
