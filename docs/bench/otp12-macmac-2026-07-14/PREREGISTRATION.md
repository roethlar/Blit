# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 2**. **No data exists yet.**
Codex round 1 (of `f0343f4`): **NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → 9/9
accepted.** Adjudication: `.review/results/macmac-prereg.gpt-verdict.md`.
Committed BEFORE the data so the decision rule cannot be authored around the
numbers (the pf-0 discipline).

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).

## What revision 1 got WRONG, and what this experiment actually answers

Revision 1 claimed this rig "discriminates H1 outright": *P1 reproduces
macOS↔macOS ⇒ H1 dies, because H1 accuses the Windows accept branch.*

**That inference is invalid, and the premise is false.** H1, verbatim in the
parent, accuses **blit's own code paths** — the `SourceSockets` Dial/Accept
branches, `InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's
synchronous dial-before-ACK at `transfer_session/mod.rs:3113`. **The word
"Windows" appears nowhere in H1.** Windows is merely *who happens to be the
accepting source* in P1's slow arm on rig W. The accused code runs on macOS too.
So a Mac↔Mac reproduction is **consistent with H1**, not fatal to it — and the
parent already warns that *"'consistent with H1' is not confirmation."*

The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
accept branch") and copied without checking H1's text. **That is a repo error and
it is corrected wherever it appears.**

**What this rig CAN answer — and it is still decision-relevant:**

> **Does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost
> of the destination-initiated layout?**

| outcome | what it licenses |
|---|---|
| **P1 REPRODUCES macOS↔macOS** | The failure needs **no Windows peer**. P1 is **not platform residue** — it is a cost of the layout/code that survives with the Windows half removed. This **closes the "accept it as platform residue" escape** (the D-2026-07-12-1 shape) and **strengthens every code-level hypothesis, H1 included**. It does **not** name the mechanism. |
| **P1 VANISHES macOS↔macOS** | The failure **requires the Windows peer**: it is pairing-dependent / platform-interacting. Code-only mechanisms that should bite on any OS are **weakened**; a Windows-specific cost, or a macOS↔Windows interaction, rises. It does **not** confirm H1 — H1's accept branch would then have to be *platform-conditionally* slow, which is a further claim needing pf-1's counterfactual. |

Either outcome materially reshapes the hypothesis space and bears directly on
whether P1 **must be fixed in code** or **could be accepted as platform residue**.
That is why it runs before pf-1. **It is not an H1 kill/confirm and this document
must never be cited as one.**

## Rig

- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
  deliberate.

**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Revision 1 claimed
it did. It does not: switching the initiator also **reassigns which machine runs
the CLI and which runs the daemon**, and `q` is the faster Mac. Only
arm-independent costs cancel; **host×role interactions do not.** This is handled
by *measuring both data directions and reporting them separately* (below), not by
assertion — and any conclusion that depends on the cancellation being perfect is
out of bounds.

## Cells

Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
`destinit` (dest's CLI pulls).

    CELLS = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
            nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
            nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)

`RUNS=8`, ABBA-counterbalanced, pair-void.

**Both directions are measured, but a reproduction is NOT required in both
(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
directions here would rewrite the finding. So: **a reproduction in EITHER
direction demonstrates the layout cost without a Windows peer.** Whether it is
direction-symmetric is reported as a descriptive fact — and, because the two
directions differ in *which machine is the destination*, a one-directional result
is explicitly **not** dismissible as "machine asymmetry" (revision 1 did exactly
that, which would have let a real reproduction be waved away).

## The noise model — PAIRED and within-cell (round-1 HIGH; revision 1's was not a noise floor at all)

Revision 1 defined `N` = max |ratio−1| over the four control cells. That is **not
a noise floor**: it is four point estimates drawn from different carriers,
fixtures and destinations, so it conflates *genuine control-specific initiator
effects* with *sampling noise*, and could equally mask a real effect or bless a
fake one.

Replaced with the **paired within-cell** statistic — the same construction pf-0's
review demanded of pf-1:

    For each cell, each ABBA slot i yields a matched pair (srcinit_i, destinit_i).
      d_i   = destinit_i − srcinit_i          (positive = P1's direction)
      D     = median(d_i)                     <- the effect
      S     = the spread of d_i               <- the PAIRED noise (report max−min AND IQR)
      MDE   = the smallest |D| this cell can resolve, taken as S (conservative)

`D` and `S` come from the *same* slots, under the *same* conditions, so ABBA
pairing is respected and between-session drift cannot enter. Every threshold below
is expressed against `S`, the 1.10 bar, or rig W's measured `Δ_P1 ≈ 230 ms` — none
is invented.

## POWER GATE — evaluated BEFORE any "vanish" claim (round-1 HIGH; pf-0's exact error, pre-empted)

pf-0 reported a KILL with an instrument that could not have resolved the effect it
killed. That must not recur.

For each TCP×mixed cell, **before** reading a verdict:

1. Compute `MDE` (above) and the effect size that a rig-W-scale P1 would have
   here: `Δ_ref = 230 ms` (rig W's Δ_P1), and also in ratio terms against **this
   rig's own fast arm** — because the 1.10 bar is a *ratio*, a 230 ms effect is
   only visible if the fast arm is fast enough (at a 2.3 s fast arm, 230 ms is
   exactly 1.10 and would sit **on** the bar).
2. **If `MDE > Δ_ref`, or if `Δ_ref` on this cell's fast arm does not exceed the
   1.10 bar, the cell is UNDERPOWERED and a PASS there is INCONCLUSIVE — it may
   NOT be reported as "P1 vanishes".** The rig gets reported as unable to see the
   effect, and the experiment does not close.

A **reproduction** does not need this gate (an effect that is seen is seen); a
**null** does.

## Decision rule — pre-registered, exhaustive, mutually exclusive, evaluated in order

Invariance uses the harness's **exact integer arithmetic** (`10·hi ≤ 11·lo`),
never the printed ratio. Per TCP×mixed cell: `D` = median paired difference,
`S` = paired spread.

1. **RIG-VOID.** Any control cell FAILS the 1.10 bar → the rig is not measuring
   cleanly and **no verdict is read**. (A rig whose gRPC control fails cannot
   adjudicate a TCP-only claim.) Report and stop.
2. **REPRODUCES (in a named direction).** A TCP×mixed cell FAILS the 1.10 bar with
   `D > 0` **and** `D > S`. Reported per direction; **either direction suffices.**
   → *P1 does not need a Windows peer.*
3. **INVERSION (in a named direction).** A TCP×mixed cell FAILS with `D < 0` and
   `|D| > S` (source-initiated is the slow arm). A **new finding**, reported as
   such — never banked as "P1 absent" and never counted as a reproduction.
4. **VANISHES.** *Both* TCP×mixed cells PASS the 1.10 bar, **and** `|D| ≤ S` in
   both, **and both cells cleared the POWER GATE.** → *P1 requires the Windows
   peer.* If the power gate was not cleared, this branch is unavailable and the
   result is **INCONCLUSIVE-UNDERPOWERED**.
5. **PARTIAL.** Any TCP×mixed cell PASSES the bar but has `|D| > S` in P1's
   direction — a real, sub-bar asymmetry. Reported with `D` stated against
   `Δ_ref = 230 ms`. Neither a reproduction nor a vanish; pf-1 owns it.
6. **MIXED-SIGN.** One direction reproduces (case 2) and the other inverts
   (case 3). Reported verbatim as a **host×role interaction**, which the rig
   cannot decompose. Explicitly **inconclusive** for the pairing question.

Cases 2/3/5/6 are read per direction and then combined by this order; the first
matching case that applies to the *session* is the headline, with every cell's own
outcome recorded. **No case is left unmapped, and no outcome may be reported that
is not one of these.**

**Bistability override, defined as a statistic, not a vibe (round-1 HIGH).** pf-0
found the rig-W fast arm bimodal, where the mode *mixture* moved a median 72 ms at
constant conditions. Here: if any arm's 8 runs split into two clusters separated by
more than `S` **and** the cell's verdict would flip when graded on the pooled runs
rather than the medians, the cell is reported **UNSTABLE**, not resolved. All 8
runs of every arm are printed in `summary.csv` so this is checkable, not asserted.

## Gates — fail-closed (round-1 HIGH: revision 1 only *warned* on the one that bit pf-0)

A run that misses any of these is **VOID**, not "close enough":

- **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
  **either** Mac (both are bench **ends** here — nagatha is no longer just the
  driver). *(Already proven live: this gate fired on its first invocation and
  refused to start while the codex review of revision 1 was running.)*
- **TIME MACHINE, BOTH MACS — FAIL-CLOSED, not a warning.** Refuse to start if a
  backup is running **or if autobackup is merely ENABLED**, because macOS repeats
  hourly and a backup can begin *inside* the window (pf-0's did, 1 minute before
  the run; one destination is a network share on `skippy` — the same 10 GbE
  fabric). Revision 1 downgraded this to a warning; that is exactly the hole pf-0
  exposed.
- **SPOTLIGHT, BOTH MACS.** `mds_stores` is a recorded contaminant
  (`.agents/machines.md`). Refuse to start while it is actively indexing.
- **LOAD THRESHOLD.** `load1` recorded on both Macs at start **and end**; a start
  `load1` above **3.0** on either end VOIDs the session (the Macs idle at ~1.5–2).
- **Cold caches both ends every run** via `sudo -n /usr/sbin/purge` (NOPASSWD
  granted on both); a failed purge **VOIDS the pair** — a warm row is worse than
  no row.
- **Destination-keyed durability, never verb-keyed**: the macOS per-file `fsync`
  walk runs **on the destination host for both arms**, is **timed**, and a failed
  walk returns `NA` → the pair **VOIDS** (it must never read as a plausible flush).
  (The otp-2w rule: a sync inside the initiator's bracket charges the pull arm for
  writeback the push arm gets free and *manufactures* invariance failures — the
  gRPC control is what exposed it.)
- **Drain**: destination disk quiet before each timed window — macOS `iostat`,
  `< 2 MB/s` for 3 consecutive 2 s samples; `DRAIN-TIMEOUT` VOIDs the pair.
- **Fixtures verified by count on both ends** (`large` 1, `mixed` 5001,
  `small` 10000) before any timed run — the arms must read the same trees.
- **Provenance**: clean `+f35702a` on all four binaries; sha256 staging manifest.
- **Link validity, measured not assumed**: peer ARP resolves to the **peer's** MAC
  (a host route on a directly-connected subnet installs a black hole that still
  reports the right interface); an ssh throughput test **cannot** validate this
  link (~79 MB/s either way regardless) and is not used.

## What this does NOT establish

- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
  pair is staged on the Macs. P2 on the Linux rig is a separate, outstanding
  experiment.
- **A null here cannot retract rig W.** P1 is measured and reproduced four times
  there; this rig can only speak about the *pairing*.
- **It cannot decompose host×role.** nagatha and `q` differ; outcome 6 exists
  precisely because that case is beyond this rig.

## The runs

```sh
EXPECT_SHA=f35702a RUNS=8 \
  CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large \
  bash scripts/bench_otp12pf_mac.sh
```

Harness: `scripts/bench_otp12pf_mac.sh`. It **computes; it declares nothing** —
the verdict is read off the rule above.
