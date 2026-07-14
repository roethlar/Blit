# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 7**. **NO DATA EXISTS YET.**

> ## THE RULE IN ONE PARAGRAPH (rev 7)
>
> **All inference is PAIRED.** The 1.10 bar is computed on the *marginal medians*; it is
> the project's **acceptance** criterion, it is reported in every row, and it takes **no
> part in inference** — because the marginal and paired statistics can disagree in both
> direction and magnitude, and every attempt to let the bar stand in for paired evidence
> produced a false verdict (rounds 3–6). **Direction** is the sign test. **Magnitude** is
> the paired CI. **Equivalence** is the CI against the margin. **The controls are a
> precondition**: unless every control is certified to carry less than *half* the
> material effect, **no verdict about the measurand may be read — not a null, and not a
> reproduction.**

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
- Round 6 (the round-5 rework, `aebd50b`): **NOT READY** — **codex** (3 BLOCKER) **and
  grok** (2 BLOCKER), converging *again* on both hunted classes: the **marginal bar still
  substituted for paired magnitude** (a **1 ms** paired effect reported `REPRODUCES` at
  n=16), a control at **D=+229** — *one millisecond* under the reference effect —
  **certified as clean**, uncertified controls **blocked only the null and not a
  reproduction**, and the settle repair was **still not provable** (a no-op `sleep` would
  have passed while the log narrated "settle included"). → **13 findings, 13 accepted.**
  (`.review/results/macmac-harness-r6.{codex,grok}.md`)
- Round 5 (the round-4 rework, `a9460ce`): **NOT READY / NOT SAFE TO RUN** — **codex**
  (3 BLOCKER, 6 HIGH, 2 MEDIUM) **and grok**, which converged on the **same BLOCKER
  independently**: the materiality bug, **for the third round running**, in a branch
  neither had been shown. → **12 findings, 12 accepted.** Plus **the dead settle**
  (above), which the review's finding exposed but did not itself find.
  (`.review/results/macmac-harness-r5.verdict.md`)

**Six rounds. 69 findings. 69 accepted. 0 rejected. Still no datum taken** — which is
the only reason none of it became a retraction.

**The rule below has been amended in rev 4 and again in rev 5. That is legitimate
only because NO DATA HAS BEEN TAKEN** — before the first run is the only honest time
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

A null is only reportable at all if the rig could have **seen** a rig-W-sized
effect — see the POWER GATE. Otherwise it is `INCONCLUSIVE-UNDERPOWERED`.

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

## The paired statistic (REV 4 — the rev-3 rule was broken three ways)

    per ABBA slot i:  d_i = destinit_i − srcinit_i     (positive = P1's direction)
      D    = median(d_i)                                (LOW median, applied everywhere)
      CI   = EXACT distribution-free order-statistic interval on the median
      sign = exact two-sided binomial test on the count of positive d_i

**1. The CI is exact, not bootstrapped (round-3 HIGH).** Rev 3 used a 10k seeded
bootstrap and called it 95%. At n=8 it resolves to ≈`[d₂, d₇]`, whose true
coverage is **92.97%**, and the resamples add no information. Rev 4 uses the
narrowest order-statistic interval `[d₍ₖ₎, d₍ₙ₊₁₋ₖ₎]` whose exact coverage
`1 − 2·P(Bin(n,½) ≤ k−1)` is **≥ 95%**. At n=8 that is **k=1 → `[min(d), max(d)]`,
coverage 99.22%**. n=8 admits **no** exact 95% interval; the conservative side is
taken **deliberately**, and the true coverage is printed in every row.

**2. The sign test now PARTICIPATES (round-3 HIGH).** Rev 3 computed it and never
read it, so 7/8 positive pairs could report `REPRODUCES` while the registered
two-sided test said `p = .0703`. An effect now requires **both** `CI` exclusion of
zero **and** `sign_p < .05`. At n=8 that means **all eight pairs must agree in
sign** (k=8 → p=.0078; k=7 → p=.0703, not significant).

*Rev 4 called these two conditions mathematical **duals**. **That was wrong once a
zero difference exists** (round-4, grok): the sign test **drops zeros**, so
`d = [0, 300…360]` gives 7/7 positive → `p = .0156`, **significant** — while the CI's
lower bound is exactly `0`, which is not `> 0`. The **CI is therefore strictly the
more conservative** of the two, and it binds. They coincide only when no `d_i = 0`.
The conjunction is kept deliberately: it is conservative in the direction that
matters (against a false reproduction), and if `n` or the coverage level ever
changes, neither condition silently weakens.*

**3. The margins are the effect's, not the bar's (round-3 BLOCKER, both reviewers,
both reproduced it).**

    BREACH_HI  = +src_med / 10     the effect that reaches ratio 1.10
    BREACH_LO  = −src_med / 11     the effect that reaches the INVERSE 1.10
                                   (NOT −src/10: the bar is symmetric in RATIO,
                                    so the two boundaries are NOT symmetric in ms.
                                    Rev 3 called a CI of [−190, 0] on src=2000
                                    "VANISHES" though −190 IS an inversion of 1.105.)

    MARGIN_HI  = min(BREACH_HI, Δ_ref)      Δ_ref = 230 ms, rig W's measured Δ_P1
    MARGIN_LO  = max(BREACH_LO, −Δ_ref)

Rev 3 tied the equivalence margin to the **bar alone**, and on a slow arm **the bar
is WIDER than the effect it is supposed to exclude**. Codex's counterexample, which
grok reproduced independently: `src = 2500` with **all eight `d_i = 230`** — a
rig-W-sized effect **in every single pair** — gives ratio 1.092 (bar PASSES), CI
`[230, 230]`, margin `0.10 × 2500 = 250` ⊃ CI → **rev 3 said `VANISHES`**.

Δ_ref is an **absolute floor** on the margin, in ms, deliberately: a null must
exclude an effect **the size of the one rig W actually measured**, however slow
this rig's arms turn out to be. The margin is always the *tighter* of the two,
i.e. the hardest to vanish.

## POWER GATE — a null must be an EQUIVALENCE result, not an absence of evidence

pf-0 reported a KILL with an instrument that could not resolve the effect it
killed. This design pre-empts that:

- A **null is only reportable** if the CI lies **strictly inside** `(MARGIN_LO,
  MARGIN_HI)`. That is a genuine *equivalence* claim: "an effect big enough to
  matter is ruled out."
- If the CI **cannot exclude** the margin, the cell is **UNDERPOWERED** and the
  session verdict is **INCONCLUSIVE-UNDERPOWERED**. A PASS is then *not* "P1
  vanishes" — it is "this rig could not have seen it".
- A **reproduction** needs no such gate: an effect that is seen is seen.

## Decision rule — computed BY THE HARNESS, exhaustive, in strict precedence

`scripts/otp12pf_mac_verdict.py` emits `session_verdict.txt`. **The verdict is not
applied by hand after the numbers are visible.** `scripts/otp12pf_mac_verdict_test.py`
guards it: **17 cases, every one a defect a reviewer actually found**, each
**mutation-proven** — reverting that fix in a copy of the engine makes exactly that
case fail (**11/11 mutations killed**) — plus a **300-input fuzz over the measurand
AND the controls** asserting the taxonomy has **no unreportable region**. *(Round-4,
grok: the old fuzz pinned the controls at a clean value, so every dirty-control path
— the one hiding the BLOCKER — went unexercised. A mutation whose target text has
drifted is reported as **STALE**, not silently passed.)*

**The bar is integer-exact (`10·hi ≤ 11·lo`) and `≤ 1.10` PASSES** — the project's
acceptance semantics, unchanged.

### THE THREE QUESTIONS, KEPT SEPARATE (rev 6)

Rounds 3, 4 and 5 each produced the **same class of defect**, because direction,
magnitude and equivalence were tangled into one expression: every patch closed the one
path it was shown and left the others open. They are now three questions, each answered
by the statistic that can actually answer it.

    DIRECTION   -- the SIGN TEST.   directional = sign_p < .05  (zeros dropped)
                                    dir_pos / dir_neg by the majority sign
    MAGNITUDE   -- the CI.          material     = bar_fail_pos or CI_lo >= BREACH_HI
                                    material_neg = bar_fail_neg or CI_hi <= BREACH_LO
    EQUIVALENCE -- the CI vs MARGIN. null_excl   = CI strictly inside (MARGIN_LO, MARGIN_HI)

    bar_fail_pos = bar FAILS **and** destinit_med > srcinit_med
    bar_fail_neg = bar FAILS **and** destinit_med < srcinit_med

**A bar failure carries a DIRECTION (round-5 codex, BLOCKER).** It did not, and the bar
is computed on the **marginal medians** while the CI is computed on the **paired
differences** — and those two can point *opposite ways*. Verified: at n=16, thirteen
`+1 ms` pairs plus three pairs falling below the whole distribution make the marginal
medians fail the bar in the **inverse** direction (1200 vs 1001, ratio 1.199) while
every surviving pair is `+1 ms`. The old rule called that **`REPRODUCES`** — *P1
reproducing off one millisecond.* A bar failure is now material only to a claim that
points **the same way** as the failure.

**DIRECTION IS THE SIGN TEST'S JOB, AND THE CI MUST NOT BE ABLE TO VETO IT (round-5 grok,
BLOCKER).** The old `pos_effect` demanded `CI_lo > 0`, so **a single zero pair** vetoed
it: `d = [0, 99×7]` at `src=1000` was "no effect" *and* `null_excl` (99 < margin 100) and
reported **`VANISHES`** — while the sign test **rejected** at `p = .0156`. Seven of eight
pairs carried a 99 ms effect, **one millisecond under the bar**, and it was called
equivalence.

Materiality is also decoupled from a bar *failure*, because `≤1.10` PASSES: a **precise
10% effect was unreportable by construction** (grok). An effect whose CI reaches the 10%
threshold is material even where the bar exactly holds.

| cell outcome | condition |
|---|---|
| **REPRODUCES** | `pos_effect` **and** `material` |
| **INVERSION** | `neg_effect` **and** `material_neg` |
| **PARTIAL** | a real effect (either sign) that is **not** material |
| **VANISHES** | no effect **and** `null_excl` — a genuine equivalence result |
| **UNDERPOWERED** | no effect and the CI **cannot exclude** the margin |
| **BAR-FAIL-INCONSISTENT** | the bar **FAILS** but the pairs do **not** agree in sign. The medians breach 1.10 while the paired evidence contradicts itself (pf-0's bistability, in a new dress). **Never a null, never a clean reproduction.** |
| **UNSTABLE** | *(override)* an arm is bimodal **and** the bar verdict flips on pooled runs |
| **INCOMPLETE** | the cell did not finish its registered pairs |

Session precedence (first match wins; every cell's own outcome is still recorded):

1. **INCOMPLETE** — any registered cell missing, **short of its `RUNS` pairs**, or
   graded on a **CI below 95% coverage**. *(Round-4, grok, **reproduced**: the engine
   trusted `meta.complete == yes` and required only ≥1 pair, so a **one-pair** CSV
   emitted **`VANISHES` at 0% CI coverage** — a confident false equivalence claim.
   The engine is separately executable and hashed into the manifest, so it must not
   depend on the harness telling it the truth: it now counts the pairs itself.)*
2. **RIG-VOID / the controls.** Three rounds running, this rule was written as *"void
   if the control's outcome is one of {…}"* — and **three times an effect walked through
   a label that was not on the list**:
   - *r3 (grok, reproduced): a **bar-FAIL** control whose CI crossed zero was
     `INCONCLUSIVE` → escaped. Session emitted `VANISHES` with controls at ratio 1.200.*
   - *r4 (grok, reproduced): a **Δ_ref-sized** control effect on a slow arm was
     `PARTIAL` → escaped. Session emitted `VANISHES` with every control at `D=+230`.*
   - *r5 (codex **and** grok, both reproduced): **one zero pair** demoted that same
     control from `PARTIAL` to `UNDERPOWERED` → escaped **again**. Same headline.*

   **So it is no longer written that way.** A control is not asked which label it got.
   It is asked the two questions it exists to answer:

   **(a) Is it CONTAMINATING?** — it carries a **directional** effect whose CI sits **at
   or beyond the margin**, or it **fails the bar**, or it is **bimodal**. → **RIG-VOID**:
   the rig is carrying the very effect we came to measure, and *nothing* here can be
   trusted. (Also RIG-VOID if the **harness** voided the session — end-load, see Gates.)

   **(b) Is it CERTIFIED CLEAN?** — is its effect **excluded** as smaller than the
   margin (`null_excl`)? **If not, A NULL IS NOT AVAILABLE** → `INCONCLUSIVE-UNDERPOWERED`.
   "The measurand shows nothing" is **not evidence of absence** when the rig cannot be
   shown free of a material arm asymmetry — a rig-wide artifact plus a cancellation
   looks *exactly* like a null.

   An uncertified control **does not kill a REPRODUCTION**: a merely *noisy* control
   cannot manufacture a consistent 8/8 one-directional effect in the measurand, and
   voiding real evidence on that basis would be its own false negative *(grok r5: a
   false void is also a broken instrument)*. A **tiny** consistent asymmetry (host×role
   — `q` is the faster Mac) is margin-excluded, certifies clean, and is **never silent**:
   it prints as a CONTROL CAVEAT against P1's TCP-only/mixed-only claim.
3. **MIXED-SIGN** — one direction REPRODUCES and the other INVERTS: a host×role
   interaction this rig **cannot decompose**. Inconclusive for the question.
4. **REPRODUCES** — **either direction**. → *P1 can occur without a Windows peer, on
   this pair* (with every limit in the table at the top). *(Round-4, grok,
   **reproduced**: `UNSTABLE` and `BAR-FAIL-INCONSISTENT` outranked this, so a **clean
   8/8 reproduction** in `nq` was reported as `BAR-FAIL-INCONSISTENT` merely because
   `qn` was noisy — a **false NON-reproduction** against this document's own
   "either direction" rule. A messy sibling is now **reported, not substituted**.
   Demoting them cannot leak a null: `VANISHES` requires **all** measurand cells to
   vanish, so a messy sibling still blocks it.)*
5. **INVERSION** — a new finding; never banked as "P1 absent".
6. **UNSTABLE** — a bimodal arm whose verdict flips. Reported as unstable, not resolved.
7. **BAR-FAIL-INCONSISTENT** — self-contradictory measurand; report the runs verbatim.
8. **INCONCLUSIVE-UNDERPOWERED** — the null branch is unavailable.
9. **VANISHES** — **both** TCP×mixed cells exclude a `min(bar_breach, Δ_ref)`-sized effect.
10. **PARTIAL** — a real but margin-excluded asymmetry; pf-1 owns it.
11. **INCONCLUSIVE** — catch-all; report the cells verbatim. *(The fuzz shows it is
    unreachable; it exists so no input can fall out of the taxonomy.)*

**No outcome may be reported that is not one of these.**

### The escalation, registered in advance

At `n=8` the ≥95% order-statistic interval **is the full range** `[min, max]`, so a
**single** noisy pair with `|d| ≥ margin` blocks a null **forever**: the rig can then
only ever say `INCONCLUSIVE-UNDERPOWERED` (round-4, grok — *a null-incapable
instrument is also broken, just less dangerously*).

**The one registered escalation**: a session that returns `INCONCLUSIVE-UNDERPOWERED`
may be re-run **once** at `RUNS=16`, where the interval is `[d₍₄₎, d₍₁₃₎]` (coverage
**97.9%**) and tolerates three outliers per side.

**It may be triggered by a POWER failure and by nothing else**, and that is now
**enforced, not merely asserted** (round-5 codex, HIGH: `UNDERPOWERED_ESCALATION=1` was
sufficient on its own — no prior session named, none verified, "once" unenforced. *A
re-roll button with a serious-sounding name.*):

- `UNDERPOWERED_ESCALATION` must name the **prior session directory**;
- the harness **reads its `session_verdict.txt`** and refuses unless it says
  `INCONCLUSIVE-UNDERPOWERED`;
- it **burns** the escalation (writes an `ESCALATED` marker), so one underpowered
  session cannot authorise a second, third, nth re-roll. **"Once" means once.**
- the trigger is recorded in the manifest (`escalated_from=`).

The trigger is **evidence on disk**, not an operator's assertion. The decision rule is
**unchanged** in the escalated run.

### The registered constants are PINNED IN CODE

`DELTA_REF_MS`, `SETTLE_MS`, `LOAD_MAX`, `DRAIN_MBPS`, `DRAIN_QUIET`, `DRAIN_ITERS` and
the timer tolerance are **literals**, in both the harness and the engine. The harness
**refuses to start** if one is merely *present* in the environment; the engine refuses a
mismatched `DELTA_REF_MS`.

*(Round-5 codex, BLOCKER: they were `${VAR:-default}`, and `DELTA_REF_MS=240` turned a
`RIG-VOID` into a `VANISHES`. **A pre-registered rule the operator can retune from the
command line, after the data exists, in the direction of the answer they want, is not a
pre-registration at all.**)* To change one: amend this document and put it back through
review. That is the entire point of the document.

**Bistability is a STATISTIC, not a vibe.** pf-0 found the rig-W fast arm bimodal,
where the mode *mixture* moved a median 72 ms at constant conditions. Here: an arm
whose runs split into two clusters separated by more than the paired spread, **and**
whose bar verdict flips when graded on pooled runs rather than medians, is
**UNSTABLE**. All 8 runs of every arm are printed in `summary.csv`, so this is
checkable rather than asserted.

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
