# macmac-harness round 3 — adjudication (grok) + a BLOCKED codex review

**Slice**: `cae2e0f` — the round-3 rework of the Mac↔Mac instrument (harness,
verdict engine, guard test) and pre-registration rev 4.
**Reviewers**:
- **grok** (xAI), headless, read-only (Edit/Write denied), with shell execution so it
  could *drive* the engine — which is exactly how it earned its keep.
  Raw: `.review/results/macmac-harness-r3.grok.md`
- **codex** (`gpt-5.6-sol` @ ultra): **DID NOT RUN.** Usage limit hit; credits are
  exhausted until **2026-07-19**. `.review/results/macmac-harness-r3.codex.md`
  contains the error, not a review.

**grok's verdict: NOT SAFE TO RUN.**
**Adjudication: 9 findings, 9 ACCEPTED, 0 rejected.** Fixed in `a9460ce`.

## ⛔ THE SLICE IS NOT CLEARED, AND CANNOT BE BY THIS RECORD

**D-2026-07-14-2 makes codex the mandatory reviewer and says grok is "additive,
never a substitute, and never runs alone."** Codex could not review round 3. One
reviewer — however good, and grok was very good — **does not clear this slice**.
The harness continues to **refuse a timed run** (`exit 2` without
`CLEARED_BY_REVIEW=1`). **This is an owner decision** (see the handoff): wait for
codex on 2026-07-19, buy credits, or amend the rule. **No agent may self-authorize
it**, and the fact that grok found nine real defects — including a BLOCKER — is the
argument *for* the second reviewer, not against it.

---

## BLOCKER — a Δ_ref-sized control effect ESCAPES RIG-VOID → **ACCEPTED (reproduced, then re-reproduced here)**

`otp12pf_mac_verdict.py` — the control void tested the **bar**, while the measurand's
margin had been fixed to `min(bar_breach, Δ_ref)`. **On a slow arm the bar is WIDER
than Δ_ref.** So a control carrying a **real, 8/8, rig-W-sized effect** (`d_i = 230`
in *every* pair) at `src = 2500` → ratio **1.092**, bar **PASS** → outcome `PARTIAL`
→ **not voided**.

Grok drove it, and I reproduced it before accepting:

    measurand = clean noise;  ALL FOUR controls d_i = 230 x8 @ src=2500
    -> SESSION VERDICT: VANISHES
       nq_grpc_mixed  PARTIAL  ratio=1.092 bar=PASS  D=+230ms CI=[+230,+230] sign_p=0.0078 (8/8)

**A headline null while every control carries the exact effect size the power gate is
built around.** This is the *same structural error* as round 3's — I fixed the
bar-tied margin for the **measurand** and left it bar-tied for the **controls**.
**Fixing a bug in one place is not fixing its class.**

**Fix**: a control voids the rig unless **its own effect is excluded as smaller than
the margin** (i.e. unless the control itself passes the equivalence test). A tiny
consistent asymmetry (host×role — `q` is the faster Mac) is margin-excluded and does
**not** void, or every session would die; a margin-sized one voids.

## HIGH — the engine trusts `meta.complete` and never checks `n` or coverage → **ACCEPTED (reproduced)**

`complete()` required only `meta.complete == yes` **and ≥1 pair**. A one-pair CSV with
a lying meta graded as a full cell:

    n=1, complete=yes  ->  SESSION VERDICT: VANISHES     (ci_coverage = 0.0000)

The engine is **separately executable** and is hashed into the manifest precisely so
it can be re-run on the CSVs — so it must not depend on the harness telling it the
truth. **Fix**: the engine counts the pairs itself (`REQUIRED_PAIRS`, default 8) and
**refuses to grade any CI below the registered 95% coverage** → `INCOMPLETE`.

## HIGH — session precedence hid a clean one-direction REPRODUCES → **ACCEPTED (reproduced)**

The pre-registration answers the question on **either direction**. But `UNSTABLE` and
`BAR-FAIL-INCONSISTENT` outranked `REPRODUCES`, so:

    nq_tcp_mixed = REPRODUCES (clean 8/8, +300..370)
    qn_tcp_mixed = BAR-FAIL-INCONSISTENT (noisy)
    -> SESSION VERDICT: BAR-FAIL-INCONSISTENT      <- a FALSE NON-REPRODUCTION

Not a false null, but a false *non*-reproduction — against this document's own rule.
**Fix**: `REPRODUCES` outranks them; the messy sibling is **reported, not
substituted**. It cannot leak a null: `VANISHES` requires **all** measurand cells to
vanish, so a messy sibling still blocks it. `MIXED-SIGN` (repro + inversion) still
outranks, because that *is* evidence of the host×role artifact.

## HIGH — the end-load could not void anything → **ACCEPTED**

My own comment said the end-load was captured before the verdict "so a session can
void on it". **The code only logged it.** A mid-session load spike — the exact
contamination the start gate exists to prevent — still produced a full verdict.
**A doc claim the code did not honour: the defect class this entire review exists to
kill, committed in the same commit that boasts about killing it.**
**Fix**: `end_load_gate()` → `SESSION_VOID_REASON` → the engine emits `RIG-VOID`.

## MEDIUM — preflight ran the cases but not the mutations → **ACCEPTED**
A silently-reverted fix could pass preflight if the cases still passed for another
reason. **Fix**: preflight runs **both**; a surviving mutation refuses the run.

## MEDIUM — at n=8 the CI is always `[min,max]`, so the rig may be NULL-INCAPABLE → **ACCEPTED**
Correct math, deliberate conservatism — but one noisy pair with `|d| ≥ margin` blocks
a null **forever**, and a rig that can only ever say `UNDERPOWERED` is also broken,
just less dangerously. **Fix**: a **pre-registered escalation** — an
`INCONCLUSIVE-UNDERPOWERED` session may be re-run once at `RUNS=16` (interval
`[d₍₄₎,d₍₁₃₎]`, coverage 97.9%, tolerates 3 outliers/side). The harness refuses
`RUNS=16` without `UNDERPOWERED_ESCALATION=1`. **Triggered by a power failure and
nothing else** — never to re-roll a result.

## MEDIUM — "CI and sign test are duals" is FALSE once a zero exists → **ACCEPTED**
`d = [0, 300…360]` → sign test drops the zero → 7/7 → `p = .0156` (significant), while
`CI_lo = 0`, which is not `> 0`. The **CI is strictly more conservative** and binds.
Worse, the `BAR-FAIL-INCONSISTENT` text then claimed "the pairs do not agree in sign"
— **which is false**; they do, and a zero blocked the CI. **Fix**: the prereg's
duality claim is corrected, and the verdict text now says what is actually true (the
CI includes 0, or the sign test does not reject).

## MEDIUM — the control caveat called a `D=+230` control "a real sub-bar asymmetry" → **ACCEPTED**
Softened a dirty-rig signal into noise. Mostly moot once Δ_ref-sized controls VOID;
the surviving wording now says **"excluded as smaller than the margin"**, which is
what it means.

## MEDIUM — the mutation proof was faithful but INCOMPLETE → **ACCEPTED**
Grok verified the seven mutations were **real kills, not fake ones** ("all seven
old-strings exist; each revert produced the forbidden verdict — not vacuous"), but
they covered none of the new holes, and the fuzz pinned the controls at a clean
value, so **every dirty-control path went unexercised**. **Fix**: **17 cases,
11 mutations** (4 new, one per round-4 fix), and the fuzz now perturbs **controls
as well as the measurand**.

Two of my round-4 mutations initially **failed honestly and I fixed them rather than
the report**: one went **STALE** (its target text had drifted in a refactor — the
stale-detector caught it) and one **SURVIVED** because the new coverage guard killed
the mutant first, meaning it no longer proved what it named.

## LOW — a non-numeric `ms` crashed the engine → **ACCEPTED, with a modification**
Grok suggested mapping it to `INCOMPLETE`. **Rejected in that form**: silently
softening a corrupt row hides the corruption. It now **fails loudly and closed** —
a named error and `exit 2`. A benchmark whose rows do not parse has no verdict.

---

## What grok could NOT force (and tried)

- A false `REPRODUCES` from the rule alone, without planting a real 8/8 material effect.
- `VANISHES` while the **measurand** carried a consistent rig-W effect (the margin +
  effect-before-equivalence ordering hold).
- A bar-FAIL control escaping the void (round-2 hole stays closed).
- A missing registered cell escaping `INCOMPLETE`.

It also re-derived the statistics independently and they check out: n=8/k=1 coverage
**99.21875%**, k=2 **92.96875%** (the old bootstrap interval), sign p at k=8
**0.0078125**, at k=7 **0.0703125**, `BREACH_LO = −src/11`, margin `min(breach, 230)`.

## Assessment

Four rounds, 44 findings, **44 accepted, 0 rejected** — and *every single rework has
introduced a defect of its own*. The instrument is now materially better than it was:
grok could not drive a false null through the measurand path, and every hole it did
find is closed and mutation-proven. But **the last two BLOCKERs were both "I fixed
the bug where I first saw it, and not where it also lived"**, which is not a run of
bad luck; it is a habit. The next review must assume the same habit is still present.

**Status: NOT CLEARED. Awaiting codex (mandatory reviewer, unavailable until
2026-07-19) or an owner decision.**
