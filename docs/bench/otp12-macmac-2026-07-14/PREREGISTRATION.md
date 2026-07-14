# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 3**. **No data exists yet.**
- Codex round 1 (of `f0343f4`, the design): NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW
  → **9/9 accepted** (`.review/results/macmac-prereg.gpt-verdict.md`).
- Codex round 2 (of `e1e351d`, the **instrument**): NOT READY — **3 BLOCKER** +
  6 HIGH + 1 MEDIUM + 1 LOW → **11/11 accepted**
  (`.review/results/macmac-harness.gpt-verdict.md`).

Committed BEFORE the data so the rule cannot be authored around the numbers.
**Two rounds of review have now caught, between them, an invalid inference, a
statistic that would have declared a real effect absent, a fail-open durability
check, and a timing artifact that reverses sign with direction — all before a
single timed run.**

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

**What this rig CAN answer — and revision 2 STILL overstated it (round-2 BLOCKER).**
Rev 2 asked whether P1 is "a platform-general cost of the layout". A rig with two
machines cannot license that. The claim is now scoped to **this pair**:

> **Can P1 occur WITHOUT a Windows peer — on this pair of Macs?**

| outcome | what it licenses — and its limit |
|---|---|
| **P1 REPRODUCES** | P1 **does not require a Windows peer** (on this pair). It is therefore **not** "platform residue" that could be waived under the D-2026-07-12-1 shape, and every code-level hypothesis strengthens. **Limits**: it does **not** establish a platform-*general* cost (two Macs are not "all platforms"), it does **not** name the mechanism, and it does **not** kill H1 — the code H1 accuses runs here too, so a reproduction is *consistent with* H1. |
| **P1 does NOT reproduce (null)** | P1 **did not occur on this pair**. That is **consistent with** "the Windows peer is required" — but does **not prove it**: it could equally be a property of *these two machines*, their disks, or this macOS version. It does **not** confirm H1 either. |

A null is only reportable at all if the rig could have **seen** a rig-W-sized
effect — see the POWER GATE. Otherwise it is `INCONCLUSIVE-UNDERPOWERED`.

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

## The paired statistic — and why revision 2's was BROKEN (round-2 BLOCKER)

Rev 1 used `N` = max |ratio−1| over four control cells: four point estimates from
different carriers, fixtures and destinations — not a noise floor at all. Rev 2
replaced it with the paired difference and `S = spread(d_i)` as the noise. **That
is still broken**, because a *range* grows with n and is dominated by outliers, so
a **large, consistent effect can hide under it**. Codex's counterexample, which
rev 2's rule accepted:

    srcinit = 2000 ms (×8);   d = [0, 180, 180, 190, 190, 200, 200, 200]
    -> D = 190, S = 200, bar PASSES, |D| <= S   =>   rev 2 says "VANISHES"

…on **7/8 positive pairs** and an effect **83% the size of rig W's Δ_P1**. It
would have reported "P1 requires the Windows peer" off an effect nearly as big as
P1 itself.

**Replaced with a real paired inference** (computed by
`scripts/otp12pf_mac_verdict.py`, and guarded by a test that asserts the
counterexample above no longer returns VANISHES):

    per ABBA slot i:  d_i = destinit_i − srcinit_i     (positive = P1's direction)
      D    = median(d_i)
      CI   = 95% BOOTSTRAP CI on the median (10k resamples, SEEDED -> deterministic)
      sign = exact two-sided binomial test on the count of positive d_i
      BAR_BREACH = 0.10 × srcinit_median   <- the effect that would reach the 1.10 bar

The median convention is the **low median** for even n, stated once and applied
everywhere (round-2 LOW).

## POWER GATE — a null must be an EQUIVALENCE result, not an absence of evidence

pf-0 reported a KILL with an instrument that could not resolve the effect it
killed. This design pre-empts that:

- A **null is only reportable** if the CI **excludes a bar-breaching effect** —
  i.e. the whole CI lies strictly inside ±`BAR_BREACH`. That is a genuine
  *equivalence* claim: "an effect big enough to matter is ruled out."
- If the CI is **too wide** to exclude it, the cell is **UNDERPOWERED** and the
  session verdict is **INCONCLUSIVE-UNDERPOWERED**. A PASS is then *not*
  "P1 vanishes" — it is "this rig could not have seen it".
- A **reproduction** needs no such gate: an effect that is seen is seen.

## Decision rule — computed BY THE HARNESS, exhaustive, in strict precedence

The harness emits `session_verdict.txt`. **The verdict is not applied by hand
after the numbers are visible** (round-2 BLOCKER: rev 2's harness computed only
PASS/FAIL, which would have left the rule to me, post-hoc).

Per cell (integer-exact bar `10·hi ≤ 11·lo`, never the printed ratio):

| cell outcome | condition |
|---|---|
| **REPRODUCES** | bar **FAILS** and `CI_lo > 0` |
| **INVERSION** | bar **FAILS** and `CI_hi < 0` |
| **VANISHES** | bar **PASSES** and the CI lies strictly inside ±`BAR_BREACH` |
| **UNDERPOWERED** | bar **PASSES** and the CI cannot exclude `BAR_BREACH` |
| **PARTIAL** | bar **PASSES**, CI excludes 0, effect not excluded as small |
| **UNSTABLE** | (override) an arm is bimodal *and* the bar verdict flips on pooled runs |

Session precedence (first match wins; every cell's own outcome is still recorded):

1. **INCOMPLETE** — any cell short of its pairs.
2. **RIG-VOID** — any **control** cell FAILS the bar. A rig whose gRPC/large
   control fails cannot adjudicate a TCP-only claim. No verdict is read.
3. **UNSTABLE** — a bimodal arm whose verdict flips. Reported as unstable, not
   resolved.
4. **MIXED-SIGN** — one direction REPRODUCES and the other INVERTS: a host×role
   interaction this rig **cannot decompose**. Inconclusive for the question.
5. **REPRODUCES** — either direction. → *P1 can occur without a Windows peer, on
   this pair.*
6. **INVERSION** — a new finding; never banked as "P1 absent".
7. **INCONCLUSIVE-UNDERPOWERED** — the null branch is unavailable.
8. **VANISHES** — both TCP×mixed cells exclude a bar-breaching effect.
9. **PARTIAL** — a real but sub-bar asymmetry; pf-1 owns it.

**No outcome may be reported that is not one of these.**

**Bistability is a STATISTIC, not a vibe.** pf-0 found the rig-W fast arm bimodal,
where the mode *mixture* moved a median 72 ms at constant conditions. Here: an arm
whose runs split into two clusters separated by more than the paired spread, **and**
whose bar verdict flips when graded on pooled runs rather than medians, is
**UNSTABLE**. All 8 runs of every arm are printed in `summary.csv`, so this is
checkable rather than asserted.

## The instrument — two defects that could have MANUFACTURED the result (round-2 HIGH)

**1. The durability check was fail-open.** `os.walk()` on a missing, unreadable or
empty path returns **0 files in 0 ms** — a missing tree reads as a *fast,
successful flush*. The two arms need **different** landed paths, because blit uses
rsync-style slash semantics (verified empirically: a push to `/bench/RUNDIR/` lands
the tree at `RUNDIR/src_<W>`; a pull into `RUNDIR` lands the files **directly in**
`RUNDIR`). A wrong path would have charged one arm **zero** durability while the
other paid full — the otp-2w bug that once manufactured P1.
**Fixed**: the fsync walk returns its **file count and byte sum**, and the pair
**VOIDs** unless both match the fixture exactly. An exit-0 zero-byte or partial
transfer can no longer become a valid *fast* row.

**2. The free-writeback gap REVERSED SIGN WITH DIRECTION.** Between a client
exiting and the fsync starting, the OS writes back dirty pages **for free**, and
that gap is longer for whichever arm ran over ssh:

    cell nq (dest = q):        srcinit = LOCAL client,  destinit = REMOTE client
    cell qn (dest = nagatha):  srcinit = REMOTE client, destinit = LOCAL client

So the *favoured arm flips with the data direction*. Since P1's signature is
**one-directional**, this artifact is capable of producing a one-directional
"reproduction" **out of nothing**.
**Measured before fixing** (the instrument is verified, not assumed): a pre-fsync
delay of **10 / 20 / 200 ms produced no measurable change in fsync time**
(72–94 ms, no trend) — APFS fsync here is per-file-metadata bound, not writeback
bound. **Fixed anyway, structurally**: a fixed, equal `SETTLE_MS` (250 ms) precedes
the fsync on **both** arms, so the asymmetry is removed by construction without
weakening what durability charges.

## Gates — fail-closed (round-1 HIGH: revision 1 only *warned*; round-2 HIGH: they all failed OPEN)

A run that misses any of these is **VOID**, not "close enough". **Every gate fails
CLOSED**: a gate that cannot answer must never answer "fine" (round 2 found
`pgrep` errors reading as "quiet", a `tmutil` read error reading as "disabled",
`top` failures reading as 0% — the same class as pf-0's `ps` decaying average that
reported a *finished* backup as 255%).

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
