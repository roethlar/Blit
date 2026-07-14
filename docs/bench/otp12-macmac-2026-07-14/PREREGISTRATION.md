# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 10**. **NO DATA EXISTS YET.**

> ## THE RULE IN ONE PARAGRAPH (rev 8 — D-2026-07-14-3, owner: "simplify")
>
> Per cell, take the **paired** ABBA differences, their median, and one **exact CI**.
> Compare that CI against **one threshold** `T = min(10% of the source arm, 230 ms)`.
> Four states, exhaustive by construction: **EFFECT** (CI clears +T), **INVERTED** (CI
> clears −T), **NONE** (CI lies inside ±T — an effect of size T is *excluded*), **UNCLEAR**
> (the CI spans a threshold). **Every control must be NONE at T/2, or no verdict about the
> measurand is read at all** — not a reproduction, and not a null. The 1.10 bar is
> reported and takes **no part** in this; the sign test is reported, not decided on.
>
> That is the whole rule. Seven review rounds found 80+ defects and **four of the last five
> BLOCKERs were in the decision rule, not the measurement** — the complexity *was* the
> defect. What pre-registration is actually for is kept: the question, the statistic and the
> thresholds are fixed **before any data exists**, and the harness **computes** the verdict.

> ## ⛔ CORRECTION THAT THIS DOCUMENT OWES ITS READER
>
> **Revisions 3, 4 and 5 of this document asserted that a fixed, equal `SETTLE_MS`
> window precedes the fsync on both arms. THAT WAS NEVER TRUE.** The settle was
> computed by an `awk` inside a command substitution whose quoting was wrong, so the
> awk errored, `sleep` received an empty argument and failed, and the code discarded
> its exit status. **The settle has never executed — not once, in any revision.**
>
> It was introduced in `24660ae` — **the commit that added it to fix the
> free-writeback asymmetry that reverses sign with direction**, i.e. the artifact
> judged capable of *manufacturing a one-directional P1 out of nothing*. **The fix for
> that BLOCKER never ran.**
>
> Nothing is retracted, because **no data was ever taken**. It is fixed, it is
> validated at preflight, and `SELFTEST=1` now proves it on a real tree. But this
> document was wrong for three revisions, and it says so here rather than quietly
> correcting the text below.

Every revision of this document and its instrument has been reviewed before it
measured anything, and **every review has found defects capable of a false claim**:

- Round 1 (design, `f0343f4`): NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → **9/9
  accepted** (`.review/results/macmac-prereg.gpt-verdict.md`).
- Round 2 (instrument, `e1e351d`): NOT READY — 3 BLOCKER + 6 HIGH + 1 MEDIUM + 1
  LOW → **11/11 accepted** (`.review/results/macmac-harness.gpt-verdict.md`).
- Round 3 (reworked instrument, `24660ae`): **NOT READY** — codex: 5 BLOCKER + 6
  HIGH + 1 MEDIUM → **12/12 accepted**; **grok** (second reviewer, D-2026-07-14-2)
  independently **confirmed both blockers with its own measurements** and found **3
  more** → **15/15 accepted**.
  (`.review/results/macmac-harness-r2.{gpt,grok}-verdict.md`)
- Round 4 (the round-3 rework, `cae2e0f`): **NOT SAFE TO RUN** — **grok**, which
  **drove the engine to a clean `VANISHES` while every control carried the full
  rig-W effect** → **9 findings, 9 accepted** (1 BLOCKER, 3 HIGH, 4 MEDIUM, 1 LOW).
  (`.review/results/macmac-harness-r3.grok-verdict.md`)
- Round 5 (the round-4 rework, `a9460ce`): **NOT READY / NOT SAFE TO RUN** — **codex**
  (3 BLOCKER, 6 HIGH, 2 MEDIUM) **and grok**, which converged on the **same BLOCKER
  independently**: the materiality bug, **for the third round running**, in a branch
  neither had been shown. → **12 findings, 12 accepted.** Plus **the dead settle**
  (above), which the review's finding exposed but did not itself find.
  (`.review/results/macmac-harness-r5.verdict.md`)

- Round 6 (the round-5 rework, `aebd50b`): **NOT READY** — **codex** (3 BLOCKER) **and
  grok** (2 BLOCKER), converging *again* on both hunted classes: the **marginal bar still
  substituted for paired magnitude** (a **1 ms** paired effect reported `REPRODUCES` at
  n=16), a control at **D=+229** — *one millisecond* under the reference effect —
  **certified as clean**, uncertified controls **blocked only the null and not a
  reproduction**, and the settle repair was **still not provable** (a no-op `sleep` would
  have passed while the log narrated "settle included"). → **13 findings, 13 accepted.**
  (`.review/results/macmac-harness-r6.{codex,grok}.md`)
- Round 7 (`1e03063`): **NOT READY** from both again — the drain fails open (a
  `drained_*` value followed by a non-zero exit), rev 7's text contradicted itself, and
  the settle could still be shadowed. → **the owner chose to SIMPLIFY the rule rather than
  harden it again (D-2026-07-14-3).** This document is the result.
  (`.review/results/macmac-harness-r7.{codex,grok}.md`)

**Seven rounds. 80+ findings, all accepted, none rejected. Still no datum taken** — which is
the only reason none of it became a retraction.

**The rule below was rewritten in rev 8, and amended in 4–7 before that. That is
legitimate only because NO DATA HAS EVER BEEN TAKEN** — before the first run is the only honest time
to change a pre-registered rule, and every amendment is forced by a reviewer's
finding, not by a number anyone has seen.

**The pattern to distrust: every rework has introduced a defect of its own.** Round
2's killer (the timer) was introduced by the round-1 rework. Round 4's BLOCKER (the
control void) is the *same structural error* as round 3's — the equivalence margin
was fixed for the **measurand** and left bar-tied for the **controls**, so a control
carrying a full rig-W-sized effect was labelled "sub-bar" and escaped the void.
**Fixing a bug in one place is not fixing its class.**

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).

## What this experiment answers — and what it does NOT

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

The question, scoped to **this pair** (rev 2 said "a platform-general cost of the
layout"; a rig with two machines cannot license that):

> **Can P1 occur WITHOUT a Windows peer — on this pair of Macs?**

| outcome | what it licenses — and its limit |
|---|---|
| **P1 REPRODUCES** | P1 **does not require a Windows peer** (on this pair), so it is **not** waivable as "Windows residue", and every code-level hypothesis strengthens. **Limits**: it does **not** establish a platform-*general* cost (two Macs are not "all platforms"); it does **not** name the mechanism; it does **not** kill H1 (the code H1 accuses runs here too); and it leaves **macOS/APFS** and **host×role** explanations fully **OPEN** — "not Windows-specific" is not "not platform-specific" (round-3 BLOCKER). |
| **P1 does NOT reproduce (null)** | P1 **did not occur on this pair**. That is **consistent with** "the Windows peer is required" — but does **not prove it**: it could equally be a property of *these two machines*, their disks, or this macOS version. It does **not** confirm H1 either. |

A null is only reportable at all if the rig could have **seen** an effect of size T —
i.e. if the CI excludes one. Otherwise the verdict is `UNCLEAR`, which is **not** a null.

**This run does NOT bear on an escape hatch for P1, because P1 HAS NONE**
(round-3 BLOCKER; parent + codex r5 F1). D-2026-07-12-1 waives only a
*cross-direction* miss for a cell that **already passes** invariance — P1 *is* the
invariance failure. Rev 3 said this run bore on "whether P1 must be fixed in code
**or could be accepted as platform residue**". The second half was never on the
table: **P1 is fixed to ≤1.10, or the owner amends acceptance criterion 1.**
What this rig changes is the *hypothesis space*, not the *obligation*.

## Rig

- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
  deliberate, and the harness **refuses any other build**.
- **Both Macs are bench ENDS.** The codex loop cannot run during a session; the
  quiescence gate enforces it on **both** hosts and has fired correctly in
  practice (it refuses while the owner's `codex` runs on nagatha).

**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Switching the
initiator also **reassigns which machine runs the CLI and which runs the daemon**,
and `q` is the faster Mac. Only arm-independent costs cancel; **host×role
interactions do not.** Handled by *measuring both data directions and reporting
them separately*, not by assertion — and no conclusion may lean on the
cancellation being perfect.

## Cells

Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
`destinit` (dest's CLI pulls).

    REGISTERED = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
                 nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
                 nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)

`RUNS=8`, ABBA-counterbalanced, pair-void. **All six cells must be present and
complete.** A partial set that is merely *filtered* would let a one-cell run emit
`VANISHES` while claiming both cells vanished (round-3 BLOCKER); missing cells are
`INCOMPLETE` and no verdict is read.

**Both directions are measured, but a reproduction is NOT required in both
(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
directions here would rewrite the finding. So: **a reproduction in EITHER
direction demonstrates the cost without a Windows peer.** Because the two
directions differ in *which machine is the destination*, a one-directional result
is explicitly **not** dismissible as "machine asymmetry" (rev 1 did exactly that,
which would have let a real reproduction be waved away).

## THE RULE (rev 8 — D-2026-07-14-3, owner: "simplify")

Seven review rounds found 80+ defects, and **four of the last five BLOCKERs were in the
DECISION RULE, not in the measurement**: a 1 ms effect reported as a reproduction; a
control carrying 229 of 230 ms certified "clean"; a null printed while every control was
dirty. The rule had ~10 outcomes, five thresholds, a certification tier and a precedence
stack. **The complexity was the defect.** It is replaced by the smallest thing that still
prevents post-hoc rationalization.

**What pre-registration is actually for, and what is kept:** the question, the statistic
and the thresholds are fixed **before any data exists**, and the **harness computes the
verdict** — so no one can look at the numbers and then invent a favourable reading.

### The statistic (paired, because the design is paired)

    per ABBA slot i:  d_i = destinit_i − srcinit_i      (positive = destination slower)
      D  = median(d_i)                                  low median, even n
      CI = EXACT distribution-free order-statistic interval on the population median —
           the narrowest whose coverage is >= 95%.
           n=8  -> [min(d), max(d)]   coverage 99.22%
           n=16 -> [d(4), d(13)]      coverage 97.87%

No bootstrap (the old one claimed 95% and delivered 92.97%). No approximation.

### The threshold (one)

    T_pos = min(srcinit_med / 10,  Δ_ref)        Δ_ref = 230 ms, rig W's measured effect
    T_neg = −min(srcinit_med / 11, Δ_ref)

`src/10` is the project's own **1.10 invariance bar**; `Δ_ref` is the effect rig W
actually measured. **The smaller of the two** — an effect must matter by *both* standards.
The negative bound is `−src/11`, **not** `−src/10`, because the bar is symmetric in
**ratio**, not in milliseconds.

### The four cell states — mutually exclusive and exhaustive BY CONSTRUCTION

They partition the CI's position relative to the thresholds. **There is no label here for
a new case to walk past**, which is precisely what went wrong seven rounds running.

| state | condition |
|---|---|
| **EFFECT** | `CI_lo >= T_pos + B` — destination-initiated is slower, by at least T |
| **INVERTED** | `CI_hi <= T_neg − B` — source-initiated is slower, by at least T |
| **NONE** | **the FULL RANGE** lies inside `(T_neg, T_pos)` — *every* pair, not just the median. An effect of size T is **EXCLUDED** (equivalence) |
| **UNCLEAR** | anything else — the CI spans a threshold; the rig cannot answer |

**A NULL IS JUDGED ON THE RANGE, AN EFFECT ON THE CI — and that asymmetry is the point
(round-8, codex, BLOCKER).** The ≥95% CI is the *narrowest* valid interval, so at n>8 it
**trims outliers**; a **bimodal** arm then yields a *narrow median CI* and a **false null**
(codex drove `CI = [1,1]` from modes at ±110). **An equivalence claim must never be
reachable by trimming away the very pairs that contradict it.** A *positive* claim may use
the CI: pairs clearing T is evidence, and a few stragglers do not undo it.

*This is also why bimodality needs no special branch — it cannot hide from the range. The
previous rule hand-coded an `UNSTABLE` override for exactly this, and got it wrong.*

### The controls are a PRECONDITION, at HALF the threshold

**Every control must be `NONE` at `T/2`.** Half, because certifying a control with the
very number that *defines* the effect is incoherent: it would let the gRPC control carry
all but 1 ms of P1 while we call the rig clean (round 6 drove exactly that).

**If any control fails, NO verdict about the measurand is read — not a reproduction, and
not a null.** Uncertainty about a rig-wide confound is not evidence that the confound is
absent, and P1's whole claim is that the effect is *specific* to TCP × mixed.

**And "clean" is not "zero" (round-8, codex, BLOCKER).** A control sitting at `+49` with
`T/2 = 50` certifies — but *that 49 ms of arm bias may be riding in the measurand too*, so a
measurand effect of exactly `T` could be half real and half rig. The bias the controls **fail
to exclude** is therefore carried into the measurand's thresholds:

    B = the arm bias the clean controls could NOT rule out, taken from each control's
        full RANGE (not its CI — the CI trims, and a bound must never be computed by
        trimming), as a FRACTION of the arm, then scaled to the cell it is applied to.

    an EFFECT must clear   T + B     (the bias could be INFLATING it)
    a NULL must fit inside T − B     (the bias could be MASKING an effect)

**B is RELATIVE, not raw milliseconds (round-9, codex, BLOCKER).** The controls run
different fixtures at different arm speeds: the *same* 4.9% bias is 122 ms on a 2500 ms
large-file control and 24 ms on a fast one. Carrying raw ms across them **under-penalises a
measurand faster than the control** — and that is the dangerous direction, because it would
license a measurand effect that is mostly rig.

If the controls are genuinely clean, `B` is a few ms and this barely moves. If they are
marginal, it bites — which is the point.

### The controls are CONTEMPORANEOUS with the measurands

The schedule is **slot-major**: within slot *i*, **every** cell takes one ABBA pair before
any cell takes slot *i+1*. All six cells therefore span the same wall-clock window.

**And the order ROTATES by slot (round-9, codex, HIGH).** A *fixed* order put both measurand
cells ahead of every control in every slot — so a **periodic** transient could land on the
measurands and never on the controls that exist to catch it. Over 8 slots each cell occupies
each position.

*(Round-8, codex, HIGH: both measurand cells used to run first and the controls afterwards
— so **the controls certified a window they were never in**. A transient could hit the
measurand and be gone before the controls ran, and they would pronounce the rig clean.)*

### The session verdict

1. **INCOMPLETE** — any registered cell short of its `RUNS` pairs, or with a CI below 95%
   coverage. (Checked against the **data**; `meta.complete` is not believed.)
2. **RIG-VOID** — the harness voided the session (end-load; see Gates).
3. **CONTROLS-NOT-CLEAN** — any control is not `NONE` at `T/2`.
4. **MIXED** — one direction `EFFECT`, the other `INVERTED`: a host×role interaction this
   rig cannot decompose.
5. **REPRODUCES** — `EFFECT` in **either** direction. *(P1's rig-W signature is
   one-directional, so demanding both would rewrite the finding. A messy sibling is
   reported, never substituted.)*
6. **INVERTED** — a new finding; never banked as "P1 absent".
7. **DOES-NOT-REPRODUCE** — **both** measurand cells `NONE`, with clean controls. A
   genuine equivalence result.
8. **UNCLEAR** — otherwise. **This is not a null.** There is no remedy in more pairs: fix the rig.

### What is deliberately ABSENT, and why that is safe

- **The 1.10 bar takes NO part in inference.** It is computed on the *marginal medians*,
  reported in every row as the project's **acceptance** criterion, and never consulted.
  The marginal and paired statistics can disagree in **direction and magnitude**, and
  every attempt to let one stand in for the other produced a false verdict.
- **The sign test is reported, not decided on.** At n=8 the CI already implies it
  (`CI_lo >= T > 0` means *every* pair clears T), so making it a second gate only added
  an interaction to get wrong. It is printed per cell.
- **No `UNSTABLE` / `PARTIAL` / `BAR-FAIL-INCONSISTENT` / `UNDERPOWERED` branches, and no
  precedence stack.** A bimodal arm **widens the CI**, and a wide CI lands in `UNCLEAR` —
  which is exactly what those branches were hand-coding. Every run of every arm is still
  printed in `summary.csv`, so bimodality remains visible to the reader.
- **A real but SUB-THRESHOLD effect is reported, not buried.** A cell can be `NONE` and
  still carry a consistent effect below T (e.g. 99 ms on a 1000 ms arm, on 7 of 8 pairs).
  The verdict prints a NOTE naming it. It does not change the outcome — the threshold was
  registered in advance — but it is **not nothing**, and it does not hide inside the word
  "none".

### There is NO escalation. `n` is EXACTLY 8.

The old `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the
**full range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR`
rig, nor certify a marginal control; and if you already have an `EFFECT`, you do not need
them.

**And `n` must be EXACTLY 8, not "at least 8" (round-9, grok, BLOCKER).** At the registered
n=8 the ≥95% interval **is** `[min, max]` — it *cannot* trim. At any larger n it starts
trimming outliers, and a bimodal arm then yields a narrow median CI and a false verdict:
grok drove a 16-pair CSV (three pairs at −500 trimmed away, thirteen at +200 left) straight
to **`REPRODUCES`**. A cell carrying any count but the registered one is **`INCOMPLETE`**.
*(I removed 16 from the registered list and left the completeness check saying `>=`. Fixed
where I looked, not where it lived — again.)*

**A noisy rig is fixed by a quieter rig, not by more pairs — and `UNCLEAR` says exactly
that.** Removing it also removes its entire p-hacking guard surface (a "once" marker, a
verdict check, a data-hash burn), none of which now has to be right.

### The registered constants are PINNED IN CODE

`DELTA_REF_MS`, `SETTLE_MS`, `LOAD_MAX`, `DRAIN_MBPS` and the rest are **literals** in
both the harness and the engine. The harness **refuses to start** if one is merely
*present* in the environment. *(They were once `${VAR:-default}`, and `DELTA_REF_MS=240`
turned a void into a null — i.e. the rule could be retuned from the command line, after
the data existed, in the direction of the answer you want. **That is not a
pre-registration.**)* To change one: amend this document and put it back through review.

### The guard test

`scripts/otp12pf_mac_verdict_test.py`:

- **34 cases** — nearly every one a defect a reviewer actually drove out of a previous
  revision of this engine.
- **14 mutations**, each reverting one fix in a copy of the engine; the named case must then
  **fail** (14/14 killed). **Not every case has a mutation** — 14 of the 34 do. The rest are
  behavioural cases (the rig must be able to *say* each thing it can say) with no single
  line to revert. *(Round-10, codex: the previous wording claimed every case was
  mutation-proven, which was not true. An overstated guard is a guard you trust too much.)*
- **5 direct rule checks**, including the identity everything leans on: at n=8 the ≥95%
  interval **is** `[min, max]` (verified over 200 random draws), so nothing can be trimmed.
  Two guards live here rather than in a mutation **because at n=8 no synthetic session can
  distinguish the CI from the RANGE** — a mutation that cannot be killed is not a proof.
- a **300-input fuzz** over the measurand **and** the controls.

It runs at preflight, cases *and* mutations; a vacuous guard refuses the run.

## The instrument — what round 3 found, and what now guards it

**THE TIMER WAS MEASURING FSYNC NOISE (round-3 BLOCKER; I introduced it in the
rework that fixed round 2).** The transfer timer captured `time.monotonic()` in
**two separate `python3 -c` processes** and subtracted them. On macOS that clock is
**process-relative**. Measured on this rig: a **1000 ms sleep read as −1 ms on
nagatha and 2 ms on q** — *negative*. Every `ms` row would have been ≈ `fsync_ms`
alone, and the invariance ratio — **the entire measurand** — would have been
computed on fsync noise, which can manufacture or mask a one-directional effect at
will. The rig would have produced a clean session, 0 voided pairs, and a confident,
meaningless verdict. **Grok measured the same defect independently** (a 500 ms sleep
reading ~3 ms) before being shown codex's finding.

The repo **already documented this trap** — `bench_otp12_zoey.sh:116` uses
`time.time()` and says why — and I reintroduced it anyway. **The lesson is not "add
a reviewer"; it is READ THE EXISTING HARNESSES BEFORE WRITING A NEW ONE.**

Now: **one process times itself and spawns the client**, and — the structural fix —
**preflight PROVES THE CLOCK on both hosts against a known 1000 ms sleep before any
data is taken**, and a run whose timer returns a non-positive value **VOIDS** rather
than entering the data as a "fast" row. The timing bug class cannot ship again
without the instrument catching it on the rig.

**Two defects that could have MANUFACTURED the result (round-2, still guarded):**

1. **The durability check was fail-open.** `os.walk()` on a missing path returns
   **0 files in 0 ms** — a missing tree reads as a *fast, successful flush*. The two
   arms need **different** landed paths (blit uses rsync-style slash semantics: a
   push to `/bench/RUNDIR/` lands at `RUNDIR/src_<W>`; a pull into `RUNDIR` lands
   **directly in** `RUNDIR`). A wrong path would charge one arm **zero** durability
   while the other paid full — the otp-2w bug that once manufactured P1.
   **Guarded**: the fsync walk returns its **file count and byte sum**, and the pair
   **VOIDs** unless both match the fixture exactly.
2. **The free-writeback gap REVERSES SIGN WITH DIRECTION.** Between a client exiting
   and the fsync starting, the OS writes back dirty pages **for free**, and that gap
   is longer for whichever arm ran over ssh — and *which arm that is flips with the
   data direction*. Since P1's signature is one-directional, this artifact could
   produce a one-directional "reproduction" **out of nothing**.
   **⛔ AND UNTIL REV 6, THE SETTLE NEVER RAN AT ALL (see the correction at the top).**
   The `awk` computing its duration sat in a command substitution with the wrong
   quoting, so it errored, `sleep` got an empty argument and failed, and the exit
   status was discarded. Revisions 3–5 asserted this fix while it was dead — including
   the revision that *introduced* it to close this very BLOCKER.

   **Now, and only now: equalized, and BOUNDED — not "removed" (round-3 HIGH).** The
   settle window is **equal on both arms** (250 ms, computed once at top level,
   validated at startup, and its failure **VOIDS the pair**). The residual is the ssh
   dispatch difference, **measured at ~15 ms** (median of 5, warm mux, recorded in the
   manifest every session; a failed ssh now refuses rather than contributing a
   flattering number). A pre-fsync delay of 10/20/200 ms produced **no measurable
   change** in fsync time here (72–94 ms, no trend) — APFS fsync on this rig is
   per-file-metadata bound, not writeback bound — so a 15 ms residual cannot plausibly
   move it. **That is a bound from measurement, not a removal by construction, and this
   document no longer claims otherwise.** `SELFTEST=1` walks a real tree and proves the
   settle actually executed.

## Gates — every one fails CLOSED, and every one is EXECUTED

Round 2 found the round-1 "fixes" **had never been run** (`bash -n` is not an
execution): the preflight **could not succeed at all** — `grep -c` exits 1 on no
match, so a **clean** binary tripped the dirty-marker probe and died, and `norm_mac`
used gawk's `strtonum()`, absent from stock macOS awk.

`SELFTEST=1` **exercises the gates for real and takes no data.** It reports three
states — `[OK]`, `[FIRED]` (a genuinely unmet condition: the gate *works*), and
`[BROKEN]` (**the probe cannot answer at all**) — and **exits non-zero on any BROKEN**,
because *a blind gate is precisely what fails open on the night*. It also **prints what
it does NOT cover**.

*(Round-5 codex, HIGH: the previous self-test labelled **every** nonzero result
`[FIRED]` — including a probe that could not answer — exited zero, and claimed "every
gate executes" while never touching drain, purge, daemon, fsync/settle, stale-daemon or
end-load. **A self-test that overstates itself is the very fail-open it exists to
hunt.**)*

It has now earned itself three times: it caught `link_gate` **refusing a perfectly good
link** (`arp -n <ip>` prints **one line per interface** — `q` holds entries for nagatha
on en0, en1 *and* en8 — so the unfiltered MAC was a three-line string that could never
equal one MAC; the gate now checks the entry **on the egress NIC**, the more correct
question anyway); it caught **the dead settle**; and it caught **itself** breaking its
own next gate (it ran `resolve_disk` in a subshell, which discarded the global that
function exists to set, so the drain then had no device and blamed the harness).

- **QUIESCENCE, BOTH MACS** — refuse if `codex`/`cargo`/`rustc` runs on **either**
  Mac. `pgrep` rc≥2 is an **error**, not "quiet" (rev 3 could not tell them apart).
- **TIME MACHINE, BOTH MACS** — refuse if a backup is running **or if autobackup is
  merely ENABLED** (macOS repeats hourly; pf-0's fired 1 minute before its run). A
  read error refuses.
- **SPOTLIGHT, BOTH MACS** — `mds_stores` CPU, taken as the **MAX across samples**
  (rev 3 took the last, so a late idle sample could overwrite an earlier busy one);
  a failed `top` is an **error**, not 0%.
- **LOAD** — `load1` on both Macs at start **and end**. A start `load1` above 3.0
  refuses; an **end** `load1` above 3.0 **VOIDS THE SESSION** (`RIG-VOID`), because a
  mid-session load spike is exactly the contamination the start gate exists to stop.
  *(Round-4, grok: rev 4 moved the end-load logging before the verdict and its
  comment claimed a session "can void on it" — but the code only **logged** it and
  graded anyway. A doc claim the code did not honour: the very defect class this
  review exists to kill.)*
- **COLD CACHES** both ends every run (`sudo -n /usr/sbin/purge`); a failed purge
  **VOIDS the pair**.
- **DRAIN** — destination disk quiet before each window (`< 2 MB/s`, 3 consecutive
  2 s samples). The device is **RESOLVED from the module path** through its APFS
  physical store (grok: rev 3 hardcoded `disk0` and could certify a disk the data
  never touched — and on APFS a *synthesized* disk can read idle while the physical
  store saturates). A **non-numeric** `iostat` sample is an **error**, never "quiet"
  (rev 3 read it as zero and **certified drainage**).
- **DURABILITY** — the per-file `fsync` walk runs **on the destination host for both
  arms**, is timed, and returns `NA` on a missing tree → the pair **VOIDS**.
- **FIXTURES** verified by **count AND byte sum** on both ends before any timed run.
- **PROVENANCE** — clean `+f35702a` on all four binaries (never `.dirty`); the
  harness, the **verdict engine** and its **guard test** are all hashed into the
  manifest; the instrument must be **committed and clean** in git (a modified
  harness must not be able to claim the reviewed commit); `EXPECT_SHA` must equal
  the **registered** build. `die` inside `$(...)` exits only the subshell, so the
  hash functions now **return non-zero** and the caller dies (rev 3 wrote an **empty
  hash** and called it provenance).
- **DAEMON LIFECYCLE** — the pid comes from `$!` (not `pgrep | head -1`, which picks
  the first of whatever is running); it must be **alive AND LISTENING** on the port;
  teardown is **verified** (a failed probe is a failure, not "GONE") and a survivor
  is recorded, not discarded.
- **LINK** — peer ARP **on the egress NIC** resolves to the **peer's** MAC (a host
  route on a directly-connected subnet installs a black hole that still reports the
  right interface), and the route egresses the 10GbE NIC (macOS routes by service
  order, so a 1GbE NIC can win and every run would go over gigabit).
- **THE VERDICT ENGINE'S OWN GUARD TEST RUNS AT PREFLIGHT — cases AND mutations.**
  If the decision rule fails its own cases, or if the proof that guards it turns out
  to be **vacuous** (a mutation survives), **no data is taken**. *(Round-4, grok: rev
  4's preflight ran only the cases, so a silently-reverted fix could still pass if
  the cases happened to pass for another reason.)*

## What this does NOT establish

- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
- **A reproduction does not exonerate macOS/APFS or host×role** — "not
  Windows-specific" is **not** "not platform-specific" (round-3 BLOCKER).
- **It does not bear on an escape hatch for P1 — P1 has none.**
- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
  pair is staged on the Macs.
- **A null here cannot retract rig W.** P1 is measured and reproduced four times
  there; this rig can only speak about the *pairing*.

## The runs

```sh
SELFTEST=1 bash scripts/bench_otp12pf_mac.sh                 # exercise the gates, no data
PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
CLEARED_BY_REVIEW=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh   # the run
```

The harness **refuses a timed run** until the round-3 review is adjudicated
(`CLEARED_BY_REVIEW`); the no-data modes always work. It **computes; it declares
nothing** — the verdict is read off the rule above.
