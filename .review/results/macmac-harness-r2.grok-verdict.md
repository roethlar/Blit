# macmac-harness round 2 — SECOND OPINION (grok)

**Slice**: `24660ae` (the instrument as reviewed; the DO-NOT-RUN `exit 2` guard in
`45a547b` landed after grok began reading, which is why grok notes the banner was
"commentary only").
**Reviewer**: `grok` (xAI), headless single-turn, read-only.
**Raw**: `.review/results/macmac-harness-r2.grok.md`
**Verdict**: **NOT SAFE TO RUN on the nagatha↔q rig.**
**Requested by the owner** ("Reviewloop grok for another opinion"), which amends
the loop's single-reviewer rule — see D-2026-07-14-2.

## Why the second opinion earned its place

Grok reviewed **independently** (it was asked for its own findings before being
shown any of codex's) and then adjudicated codex's two critical claims. It
**CONFIRMED both**, with its own empirical evidence, and found three defects codex
did not.

### It confirmed the two blockers — with independent measurements

- **The timer.** Grok measured it itself: a 500 ms sleep reads as **~3 ms** through
  the harness's two-process `time.monotonic()` pattern, **~522 ms** with
  `time.time()`, and **~510 ms** single-process. Same conclusion, arrived at
  separately: `RUN_MS ≈ RUN_FLUSH`, so invariance would be graded on **fsync
  noise**.
- **The equivalence margin.** Grok reproduced the `VANISHES`-with-a-real-effect
  case exactly (`srcinit=2500`, all eight `d_i=230` → ratio 1.092, bar PASS,
  CI `[230,230]` ⊂ ±250 → **VANISHES**), and named the root cause the same way:
  `DELTA_REF_MS` is written to CSV and **never used in a decision**.

Two independent models converging on both blockers is much stronger evidence than
either alone — and it is the reason to keep the second opinion for the hard calls.

### Three findings codex did NOT report

1. **RIG-VOID fails open — and grok REPRODUCED it.** The pre-registration says any
   control failing the bar voids the rig. The code additionally requires the
   control's *outcome* to be outside `{VANISHES, INCONCLUSIVE, UNDERPOWERED}`, so
   a control with **bar FAIL but a CI crossing zero → INCONCLUSIVE** escapes
   RIG-VOID. Grok drove it: gRPC controls at **ratio 1.200, bar FAIL**, and the
   session still emitted **`VANISHES`**.
2. **The drain watches the wrong disk.** `iostat ... disk0` is hardcoded; if the
   bench volume's stats are not on that device, the drain certifies a disk the
   data never touched (false quiet, or a missed void).
3. **An exact 1.10 ratio can never REPRODUCE.** The integer bar `10·hi ≤ 11·lo`
   makes exactly 1.10 a **PASS**, so a precise 10% effect is unreportable by
   construction.

### And one observation that stings

Grok noted that **the zoey harness already documents why cross-process
`monotonic()` is wrong.** The repo had already learned this lesson and written it
down; I reintroduced the bug anyway. The lesson is not "add another reviewer" —
it is **read the existing harnesses before writing a new one**.

## What it did NOT dispute

Grok explicitly cleared, as **not** arm-biased: the destination-keyed fsync with
count/byte voiding, the equal settle, the purge-then-drain order, ABBA pairing,
pair-void on exit/cold/drain, and the registered-cell allowlist — calling them
"real improvements over past harnesses", while stressing that none of it makes the
instrument safe while the timer and the equivalence rule stand.

## Net

- **codex round 2**: 12 findings.
- **grok**: confirms both blockers independently, adds RIG-VOID-fails-open (with a
  reproduction), the `disk0` drain, and the exact-1.10 gap.
- **Combined fix list for round 3** is the union of both. Nothing from either
  reviewer was rejected.
