# macmac-harness round 5 — adjudication (codex + grok)

**Slice**: `a9460ce` — the instrument after rounds 3 and 4.
**Reviewers** (both, per D-2026-07-14-2 — this is a benchmark instrument):
- **codex** `gpt-5.6-sol` @ ultra. Raw: `.review/results/macmac-harness-r5.codex.md`
  → **NOT READY TO RUN.** 3 BLOCKER, 6 HIGH, 2 MEDIUM.
- **grok** (xAI), read-only + shell, asked to verify its own round-4 findings and then
  hunt what the fix broke. Raw: `.review/results/macmac-harness-r5.grok.md`
  → **NOT SAFE TO RUN.**

**Adjudication: 12 findings, 12 ACCEPTED, 0 rejected.** Fixed in the commit below.
**Plus one defect the review did not find, which its finding exposed — see THE SETTLE.**

## The verdict on my own work

Grok's sentence is the one that matters:

> *"The rework again fixed **the branch that was shown**, not the class."*

Both reviewers independently found **the same materiality bug for the third time**, in
a branch I had not been shown. That is no longer a coincidence; it is a method
failure, and it is now fixed as a *class*: direction, magnitude and equivalence were
tangled together in one expression, so every patch closed one path and left the
others open. They are now three separate questions answered by three separate
statistics, and the control rule is written as *what a control must PROVE* rather than
as a list of outcome labels that a new label can walk past.

---

## ⛔ THE SETTLE HAS NEVER RUN — NOT ONCE, IN ANY REVISION

Codex filed a HIGH: *"failure of the required settle `sleep` is ignored, because the
succeeding Python fsync walk supplies the command status."* True. **But executing it
showed the status was ALWAYS failure.**

    sleep $(awk -v m="$SETTLE_MS" 'BEGIN{printf \"%.3f\", m/1000}')

A command substitution is parsed **fresh** by bash, so the `\"` escapes — which are
correct inside `hrun`'s two-level strings, and correct everywhere else in this file —
were **literal backslashes** to awk. Measured:

    awk: syntax error at source line 1
    usage: sleep number[unit] ...
    -> the walk ran IMMEDIATELY, every time

**The awk errored on every call, `sleep` got an empty argument and failed, and the old
code discarded its exit status.** So:

- **`SETTLE_MS` has never been applied**, on either arm, in any revision.
- It was introduced in **`24660ae`** — *the commit that added it to fix the
  free-writeback asymmetry that REVERSES SIGN WITH DIRECTION*, the artifact judged
  capable of **manufacturing a one-directional P1 out of nothing**. **The fix for that
  BLOCKER never executed.**
- The pre-registration has asserted "a fixed, equal `SETTLE_MS` precedes the fsync on
  BOTH arms" through revisions **3, 4 and 5**. It was **not true when written and has
  not been true since.**

No data was ever taken, so nothing is retracted. But this is the fourth time this
project has been saved only by not having run yet, and it is the sharpest possible
argument for the rule that found it: **`bash -n` is not an execution.** The settle is
now computed once at top level (no nested quoting), validated, and its failure VOIDS
the pair. `SELFTEST=1` now walks a real tree and proves the settle ran.

---

## BLOCKER — `bar == FAIL` is DIRECTION-BLIND → **ACCEPTED (codex; I reproduced it before accepting)**

`material = (bar == "FAIL") or ...` — but a bar failure has no direction, so a bar
failure in the **inverse** direction made a **positive** effect material. Verified:

    src = [1000]*7 + [1200]*6 + [5000]*3      (n=16, the escalation arm)
    d   = [+1]*13 + [-4500]*3
    -> marginal medians: srcinit 1200 vs destinit 1001 -> ratio 1.199, bar FAIL (INVERSE)
    -> paired: CI = [+1,+1], sign_p = .0213 (13/16 positive)
    -> OLD: material = (bar==FAIL) = True, pos_effect = True  =>  REPRODUCES

**P1 "reproducing" off a ONE MILLISECOND effect.** The bar is computed on the marginal
medians while the CI is computed on the paired differences, and those two can point in
**opposite directions**. **Fix**: `bar_fail_pos = bar FAIL **and** d_med > s_med` (and
the mirror). A bar failure is material only to a claim that points the same way.

## BLOCKER — an `UNDERPOWERED` control escapes the void (the SAME bug, third branch) → **ACCEPTED (both reviewers, both reproduced)**

Round 4 made a `PARTIAL` control void the rig. But **one zero pair** drags `ci_lo` to 0,
which killed `pos_effect` and demoted the cell from `PARTIAL` to `UNDERPOWERED` — and
`UNDERPOWERED` was on the *does-not-void* list. Codex drove `d = [0, 230×7]`; grok drove
`d = [230×7, −10]`. Both produced a headline **`VANISHES` with every control carrying
the full rig-W effect** (`D = +230`, ratio 1.092).

**Fix**: the control rule no longer asks *"which label did this cell get?"* It asks the
two questions a control exists to answer:
1. **CONTAMINATING?** — a directional effect whose CI sits at/beyond the margin, or a
   bar failure, or bimodality → **RIG-VOID**. Nothing here can be trusted.
2. **CERTIFIED CLEAN?** — is its effect *excluded* as smaller than the margin? If not,
   **a null is not available** (→ `INCONCLUSIVE-UNDERPOWERED`), because "the measurand
   shows nothing" is not evidence of absence when the rig might be carrying the effect
   everywhere. It does **not** kill a reproduction: a merely noisy control cannot
   manufacture a consistent 8/8 one-directional effect in the measurand, and voiding
   real evidence on that basis would be its own false negative (grok, NEW-5).

## BLOCKER — the registered constants were ENV-OVERRIDABLE → **ACCEPTED (codex)**

`DELTA_REF_MS=240` turned a `RIG-VOID` into a `VANISHES`. `LOAD_MAX`, `DRAIN_MBPS`,
`SETTLE_MS` and the timer tolerance were equally unpinned. **A pre-registered rule the
operator can retune from the command line, after the data exists, in the direction of
the answer they want, is not a pre-registration.** **Fix**: literals in both the harness
and the engine; the harness **refuses to start** if one is merely *present* in the
environment, and the engine refuses a mismatched `DELTA_REF_MS`.

## HIGH — the escalation was p-hackable → **ACCEPTED (codex)**
`UNDERPOWERED_ESCALATION=1` was sufficient: no prior session named, none verified,
"once" unenforced. A re-roll button with a serious name. **Fix**: it must now name the
prior session **directory**, the harness **reads its `session_verdict.txt`** and refuses
unless it says `INCONCLUSIVE-UNDERPOWERED`, and it **burns** the escalation (an
`ESCALATED` marker) so one underpowered session cannot authorise a second re-roll. The
trigger is evidence on disk, not an operator's assertion.

## HIGH — the drain fails open in the paths I did not touch → **ACCEPTED (codex)**
A failed/unparsable `diskutil` silently fell back to the **synthesized** APFS disk —
whose counters can read **idle while the physical store saturates** — so the fallback
was not a harmless default but a **false quiet**. And inside the loop, a numeric
`iostat` line followed by a **nonzero exit** still accumulated "quiet" samples.
**Fix**: an APFS volume whose physical store cannot be resolved **refuses**; the
iostat exit code is checked **before** its value is used.

## HIGH — the second `pgrep` site still failed open → **ACCEPTED (codex)**
I fixed the fail-open in `quiescence_gate` and left the identical bug in the
**stale-daemon probe**, where `rc ≥ 2` (a broken probe, a failed ssh) read as "no daemon
running". **Fix**: there is now exactly **ONE** process probe in the file (`pgrep_state`
→ `RUNNING|NONE|BROKEN`), so there is no second site left to forget. *Same class as the
BLOCKER above: fixed where I saw it, not where it lived.*

## HIGH — `SELFTEST` was not an honest gate test → **ACCEPTED (codex)**
It labelled **every** nonzero result `[FIRED]` — *including a probe that could not
answer at all* — and exited zero, while claiming "every gate executes" despite never
touching drain, purge, daemon, fsync/settle, stale-daemon or end-load. **A self-test
that overstates itself is the very fail-open it exists to hunt.** **Fix**: three states
(`[OK]` / `[FIRED]` = a genuinely unmet condition / `[BROKEN]` = the probe is blind),
**exit 1 on any BROKEN**, the missing paths are exercised for real, and it now **prints
what it does NOT cover**. It immediately earned itself: it caught the dead settle, and
it caught *itself* breaking its own next gate (running `resolve_disk` in a subshell
discarded the global it exists to set, so the drain then had no device).

## HIGH — guard-test coverage was materially incomplete → **ACCEPTED (codex)**
The helper hardcoded **constant** source times and `REQUIRED_PAIRS=8`, so the n=16
blocker was **unguardable by construction**; the fuzz only checked that the label was a
known one. **Fix**: per-pair source vectors, `pairs=16`, and cases for every round-5
finding. **22 cases, 15 mutations, all killed.** Five mutations went **STALE** when the
engine was restructured — the stale-detector caught them rather than passing silently —
and the mutation harness now judges a kill by **whether the case FAILS**, not by
matching a verdict I named in advance.

## MEDIUM — ssh RTT ignored the child return codes → **ACCEPTED (codex)**
A fast-failing ssh would have produced a small, plausible "bound" that flattered the
settle claim. **Fix**: a nonzero rc refuses.

## MEDIUM — daemon PID leak on a failed validation → **ACCEPTED (codex)**
The pid was stored **after** the alive/listening checks, so a daemon that started and
then failed validation was `die`d on while the EXIT trap did not know it existed —
leaking a live daemon holding the port. **Fix**: own the pid immediately.

## MEDIUM/LOW — wording that softened a dirty signal → **ACCEPTED (grok NEW-4, NEW-6)**
`BAR-FAIL-INCONSISTENT` still said "the pairs do not agree in sign" when a zero pair
(not disagreement) had blocked the CI; the engine and test still called the CI and the
sign test "mathematical duals" after the prereg had corrected it. Both fixed.

## grok NEW-5 — the control rule could FALSE-VOID a good rig → **ACCEPTED**
A single outlier pair in a control would have hard-voided the session. **A false void is
also a broken instrument.** Resolved by the two-question split above: an outlier makes a
control *uncertified* (blocks the **null**, remedy = the registered `RUNS=16`
escalation), while only an effect whose CI sits **at or beyond the margin** *contaminates*
(kills the session).

## What grok verified as genuinely fixed (round 4)
All nine of its round-4 findings, re-driven: the Δ_ref control void, the n=1/meta lie
(now `INCOMPLETE`), the masked one-direction `REPRODUCES`, the end-load void, the
corrupt-row `exit 2` ("I agree with the loud exit 2"), and the guard/mutation suite
(17/17, 11/11 at the time, "not vacuous"). The statistics were independently re-derived
by **both** reviewers and check out: n=8 coverage **99.21875%**, n=16 **97.8729%**,
sign p at k=8 **.0078125** and k=7 **.0703125**, `BREACH_LO = −src/11`,
`margin = min(breach, 230)`.

---

## Assessment

**Five rounds. 56 findings. 56 accepted. 0 rejected.** And the instrument has *still*
never taken a datum — which is the only reason none of this became a retraction.

The pattern is now named precisely enough to design against, and it was not "I make
careless mistakes": it is that **I fix the instance I am shown and do not sweep the
class**. Three rounds of the same materiality bug, and a fail-open `pgrep` duplicated in
a site I had already fixed elsewhere. The structural answers in this round — one process
probe, one materiality question asked three ways, a control rule phrased as an
obligation rather than a label list, constants pinned in code, and a self-test that
fails when a gate goes blind — are aimed at the class, not the instances.

**Status: NOT CLEARED. A round-6 review must assume the habit is still present.**
