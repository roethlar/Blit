# macmac round 9 — adjudication (codex + grok), prereg rev 10

**Slice**: `8830fda` (rev 9). **Fixed across `446549e`, `2264ae2`, `7d72f77`, `224c4b8`.**
**NO DATA HAS EVER BEEN TAKEN.**

- **grok**: NOT READY TO RUN. Raw: `macmac-harness-r9.grok.md`
- **codex**: its FIRST round-9 run was **killed by a content filter** ("flagged for possible
  cybersecurity risk") after reading 85k tokens and **produced no review at all** —
  `macmac-harness-r9.codex.md` contains only the files it read plus stale round-1 material
  quoted out of the pre-registration's own history. **IGNORE IT.** Re-run with the
  adversarial framing removed: `macmac-harness-r9b.codex.md` → NOT READY TO RUN.

**Adjudication: 8 findings, 8 ACCEPTED, 0 rejected.**

## BLOCKER — n must be EXACTLY 8 (grok, reproduced)
I removed 16 from the registered pair counts and left the completeness check saying
`len(d) >= PAIRS`. So a **16-pair CSV was graded**: `median_ci` then picked k=4 and **trimmed
the three pairs at −500** while keeping the thirteen at +200 → **`REPRODUCES`**.

The entire rule leans on the property that **at n=8 the ≥95% interval IS `[min, max]`** and
therefore cannot trim. That only holds if n is *exactly* 8. **Fixed where I looked, not where
it lived — again**, and that is the class this instrument keeps producing.

## BLOCKER — B must be RELATIVE, not raw milliseconds (codex)
The controls run **different fixtures at different arm speeds**. The same 4.9% arm bias is
**122 ms** on a 2500 ms large-file control and **24 ms** on a fast one. Carrying raw ms
across them **under-penalises a measurand faster than the control** — the dangerous
direction, because it licenses a measurand effect that is **mostly rig**. B is now a
**fraction of the arm**, scaled to the cell it is applied to.

## HIGH — B must come from the control's RANGE, not its CI (grok)
The CI is an interval for the **median**, and it **trims**. A bound on what the rig might be
carrying must never be computed by trimming. grok drove a control with range `[5,40]` giving
`B=5` instead of `40`, and a measurand at +105 then read `REPRODUCES`.

## HIGH — the null's B-tightening had no mutation (grok)
`a NULL must fit inside T − B` (bias could be **masking** an effect) was load-bearing and
unguarded. It has a case and a mutation now: measured +60 with controls at +49 must be
`UNCLEAR`, and deleting the tighten makes it a **false null**.

## HIGH — the drain accepted `"."` as a number (codex)
The validator tested the *shape* (digits and periods). `"."` passes, awk reads it as **0**,
and `0 < 2 MB/s` **CERTIFIES QUIET**. A shape test is not a number test.

## HIGH — the cell order was fixed, measurands always first (codex)
Within every slot both measurands ran ahead of every control — so a **periodic** transient
could land on the measurands and **never on the controls that exist to catch it**. The order
now rotates by slot.

## HIGH — `$SELF` was not canonicalized (codex)
The clean-instrument git check could miss when the script is invoked by a relative path.

## MEDIUM — the verdict still advised `RUN S=16` (grok)
An escalation that no longer exists. **Advice that cannot be executed is not a protection.**

---

## Two guards that could NOT be killed, and were not kept as fake proofs

At the registered n=8, the ≥95% CI **is** `[min, max]` — the same two numbers — so **no
session can distinguish the CI from the RANGE**. Two mutations (the null's range rule, and
B-from-range) are therefore *inert through a session* and would have "survived" while proving
nothing. They are now **asserted directly** instead: the rule is called with the CI and the
range differing, and the n=8 identity is checked over 200 random draws. *A mutation that
cannot be killed is not a proof.*

## State
**32 cases, 12/12 mutations killed, 5 direct rule checks.** SELFTEST green on both hosts; the
settle MEASURED at 260 ms. **Nine rounds, ~100 findings, all accepted, none rejected, and not
one datum taken.**
