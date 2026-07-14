I'll treat this as a design challenge, not a rubber stamp: read the preregistration, harness, and verdict rule, run the selftests, then argue whether the Mac↔Mac run can actually answer the question.Continuing the harness (ABBA schedule and measurement path) and running the selftests.Running the verdict guard tests and the harness selftest, and probing a few design-risk spots.Checking a few more concrete failure modes that could produce a wrong verdict:## 1–2. Verdict on the question and the outcomes

**The experiment can answer the question it claims — and that question is the wrong next move.**

The only clean test of “can P1 occur with no Windows peer?” is a real same-OS pair that is not Linux↔Linux. Mac↔Mac over 10GbE is that cell. Loopback would not do (the parent plan already says a negative loopback is inconclusive). So as a *scientific* 2×2 fill, the *shape* of the question is fine.

As a *project* move, it is a detour.

What is already known:

| Pair | P1? |
|---|---|
| Linux↔Linux | No (8/8) |
| macOS↔Windows | Yes, four times, 1.24–1.39 |
| macOS↔macOS | Untested |

MTU is dead as a dominant cause. H1 names **blit code that also runs on macOS**, not “Windows.” P1 has **no escape hatch** on the books: fix to ≤1.10 on rig W, or the owner amends criterion 1. Nothing in this rig creates a waiver.

So:

| Outcome | Legitimate learning | Temptation you must not take | Changes next engineering action? |
|---|---|---|---|
| **REPRODUCES** | P1 can appear with no Windows peer **on this pair**. Not “Windows residue.” Code-level and macOS/APFS/host×role explanations stay open. Does **not** kill or confirm H1. | “H1 is confirmed” / “platform-general layout cost” / “skip Windows investigation” | **No.** Still pf-1 / fix on **rig W**, where release is blocked. |
| **DOES-NOT-REPRODUCE** | On **this** pair, an effect of size T was excluded with clean controls. Consistent with “Windows peer matters”; **does not prove** it (could be these disks, these Macs, this macOS). | “P1 is Windows-only, waive it” / “H1 confirmed” | **No.** P1 still fails on W. Still pf-1 on W (maybe with more weight on Windows-side accept). |
| **MIXED** | Host×role interaction on this pair; cannot attribute initiator cost. | Reading one direction as “P1” and ignoring the invert | **No.** Inconclusive; still W. |
| **INVERTED** | New finding (source-init slower). | “P1 absent” | **No.** New puzzle; P1 on W untouched. |
| **UNCLEAR** | Rig could not resolve ±T. | Treating as soft null | **No.** Fix noise or abandon; do not add pairs (rule forbids it). |
| **CONTROLS-NOT-CLEAN** | gRPC/large (or residual bias B) not clean enough to attribute TCP×mixed | “Controls dirty ⇒ P1 is rig-wide ⇒ not real on W” (W already has clean controls in the original data) | **No.** Re-quiet or stop. |
| **INCOMPLETE / RIG-VOID** | Session failed integrity | Anything causal | **No.** |

**No outcome changes the release-critical next step:** attribute and fix (or formally re-scope) P1 on the macOS↔Windows pair. Mac↔Mac only reweights priors inside pf-1, and only if it returns a *decisive* REPRODUCES / DOES-NOT-REPRODUCE. Under the rule’s own austerity, decisive outcomes are not the mode you should bet on (below).

The one non-engineering fork: a clean null, stacked on Linux null, would be the honest evidence package **if** the owner is weighing amending criterion 1. The prereg correctly says that decision does not exist and this run does not create it. If you are not about to have that conversation, the 2×2 is map-making while the ship is on fire.

**What I would do instead / first**

1. **pf-1 on rig W**, where the 25–38% effect is real: dial/accept inversion, no-resize, per-side dial-before-ACK ordering — the parent’s own H1 discriminators. That can change code.
2. **Cheap diagnostics on W**: spans on `SourceSockets` Dial/Accept, `add_dialed_stream`, dial-before-ACK; optional packet timing of accept/dial RTT under mixed TCP. Hours, not another review epoch.
3. **Only if** you need the platform map for a criterion conversation: a **minimal** Mac↔Mac TCP×mixed both directions (even informal n≈4) before this six-cell cathedral. If ratios sit at ~1.00 vs ~1.30, you already know; then either formalize or stop.

Eleven rounds, ~110 defects, **zero data** is not “we are careful.” It is a signal that the instrument became the project. Selftest and the verdict suite **pass now**. Further review rounds are how you never ship the measurement or the fix.

---

## 3. Design shape

**Controls (right idea, harsh implementation)**  
gRPC×mixed and TCP×large, both directions, match P1’s claimed specificity (TCP-only, mixed-only on W). Slot-major + rotated cell order correctly stops “controls certify a window they never shared.” Good.

**ABBA + slot-major**  
Sound for drift and shared transients. Caveat: void retries run **in place** before the next cell, so “same slot” is not “same wall-clock moment.” Session-level co-temporal is true; per-slot simultaneity is not. Acceptable if you do not overclaim.

**n = 8 and the full-range null**  
At n=8 the ≥95% order-statistic CI **is** `[min, max]` (coverage 99.2%). EFFECT and NONE both effectively require **every pair** to cooperate. That is deliberate and honest for equivalence — and it is **power-hostile**.

Rough session-level simulation (true +250 ms on both measurands; four null controls):

| Meas. pair σ | Ctrl pair σ | Rough outcome mix |
|---|---|---|
| 40 ms | 15 ms | ~92% REPRODUCES, ~8% CONTROLS-NOT-CLEAN |
| 60 ms | 20 ms | ~56% REPRODUCES, ~41% CONTROLS-NOT-CLEAN |
| 80 ms | 25 ms | ~17% REPRODUCES, ~78% CONTROLS-NOT-CLEAN |

Four independent full-range “NONE at T/2” controls: P(all clean) collapses once pair noise is ~20–25 ms (≈39% → ≈10%). Rig W already showed **~110 ms bimodality on a fast arm** between sessions; ABBA helps, but it does not make control ranges tiny by magic. **The design’s most likely “successful” execution may be CONTROLS-NOT-CLEAN** — which buys a re-run, not an answer.

**Missing / unseparated**

- Host×role is only split by running both data directions; nagatha vs `q` speed still confounds one-directional effects (prereg admits this; MIXED is the escape). Fine if you obey MIXED.
- No mid-session quiescence re-check (start + end load only). Residual.
- Δ_ref = 230 ms is **W’s** absolute effect. On faster Mac↔Mac arms, T often becomes `src/10` (stricter null, easier small relative EFFECT). Not wrong, but the null gets harder on a fast pair.
- Does not separate macOS-as-dest vs Windows-as-source (would need Mac↔Linux or the W inversion). Out of scope for the stated question; still the more diagnostic experiment for *mechanism*.

**Severity summary (design)**

| Sev | Finding |
|---|---|
| **HIGH** | Four controls × full-range NONE at **T/2** × n=8 makes **CONTROLS-NOT-CLEAN** a likely modal outcome under mild pair noise; then the session answers nothing. |
| **HIGH** | EFFECT also needs **min(d) ≥ T+B** (every pair). A real ~25% effect with one soft pair → UNCLEAR, not REPRODUCES. Conservative, but weak power. |
| **MEDIUM** | Slot-major + in-place retry ≠ per-slot contemporaneous controls. |
| **MEDIUM** | No TCP×small control (optional; W already says small is clean). |
| **LOW** | Δ_ref imported from another pairing; dual threshold is intentional, document when T is bar-limited vs Δ-limited. |

---

## 4. Decision rule

A statistician would recognize the pieces: **paired differences**, **exact sign/order-statistic CI on the median**, **range-based equivalence** (stricter than TOST on the median), **pre-registered threshold**, **control precondition**. They would **trust a NONE** as “no pair showed an effect near T” — stronger than “median is small,” and hard to game. They would **not** call it a standard 95% test of H₀: median = 0; they would call it a **deliberately austere decision procedure**.

What is sound:

- One T; four exhaustive cell states; no bar-in-the-loop; sign test reported only.
- B only hardens; MIXED on unhardened states (fixes noisier-rig → stronger-claim).
- B ≥ T/2 → refuse to grade (closes capped-T vs fractional-B dead zone).
- n exactly 8 (no trim); incomplete on wrong n.
- −src/11 for inverse 1.10 ratio symmetry — correct.

What is “too clever” in the bad sense:

- **B as max fractional range across controls**, then scaled onto the measurand, plus T/2 control certification, plus B ≥ T/2 session kill: each piece fixed a real past bug; **together they stack conservatism** until many honest quiet-ish sessions cannot speak.
- At n=8, “CI vs range” is the same two numbers; the split is future-proofing. Fine, slightly ornamental today.

Would I trust a null out of it? **Yes**, if I got one. Would I expect to get one on a live APFS/10GbE pair with purge/drain/fsync in the loop? **Often no** — UNCLEAR / CONTROLS-NOT-CLEAN instead. That is the rule working as a nervous instrument, not as a sharp classifier.

| Sev | Finding |
|---|---|
| **HIGH** | Rule is trustworthy when decisive; **under-powered for decisiveness** under plausible noise (see §3). |
| **MEDIUM** | Stacked conservatism (T/2 controls + B + B≥T/2 + full-range null) may be more complex than D-2026-07-14-3 “simplify” still wants. |
| **LOW** | EFFECT inclusive vs NONE strict at the boundary — conservative, asymmetric, OK. |

---

## 5. Correctness (harness / rule) — bugs that wrong the measurement or verdict

Selftest: **PASSED** (0 blind; quiescence correctly **FIRED** on nagatha while `codex` runs).  
Verdict: **40/40 cases, 19/19 mutations killed, 300/300 fuzz, n=8 CI≡range identity OK.**

I did **not** find a live “false REPRODUCES / false DOES-NOT-REPRODUCE” in the graded path comparable to the historical timer/settle/control-void blockers. Remaining issues are smaller or process-level.

| Sev | Where | Issue |
|---|---|---|
| **MEDIUM** | `bench_otp12pf_mac.sh` ~1118–1126, 1052–1063 | In-place void retries break the “controls share the measurand’s moment” story inside a slot. Can mis-fire CONTROLS clean while measurand retried under different load (or the reverse). |
| **MEDIUM** | `bench_otp12pf_mac.sh` 1322–1325; main path | Selftest **still does not** run daemon lifecycle, smoke, ABBA, or pair-void. First full path execution **is** the first data session. Historically this class is where defects hide. Not a known wrong formula; residual risk. |
| **MEDIUM** | `otp12pf_mac_verdict.py` 229–257 + control loop 305–323 | Not a code bug: full-range control NONE is so tight that **correct** code yields **frequent non-answers**. Wrong *utility*, not wrong arithmetic. |
| **LOW** | `bench_otp12pf_mac.sh` 1008–1013 vs preflight RTT | Per-pair RTT void is good; document already registers it. Residual free-writeback bound is measurement-backed, not proof of zero. |
| **LOW** | `otp12pf_mac_verdict_test.py` header vs body | Doc counts (34 cases / 14 mutations) drift vs 40 / 19. Guard is not vacuous; commentary is stale. |
| **LOW** | `bench_otp12pf_mac.sh` 1–23, 96–104 | Header still “round-11 / round-12 gate” narrative; process, not measurand. |
| **INFO** | Historical classes you named | Settle is now in-process and selftest-proved; constants pinned **after** override check; topology/MSS gated; timer single-process + gate. Those specific landmines look closed **as of this selftest**. |

I am **not** claiming the harness is bug-free. I am claiming: at this revision, the failure mode that should scare you most is **a clean, confident non-answer** (or a twelfth review), not an obvious arithmetic reverse of P1.

---

## Bottom line

This is a **well-built instrument pointed at a real but low-leverage question**. It will answer “Windows-free P1 on nagatha↔q?” only if noise is unusually kind; many runs will return CONTROLS-NOT-CLEAN / UNCLEAR and license nothing. When it is decisive, **you still do pf-1 on rig W**. The expensive thing left is not another design pass — it is either taking data once or admitting the 2×2 cell is optional science and moving to the pair that blocks release.

**DO NOT RUN IT — skip Mac↔Mac for now and spend the session on pf-1 / dial-accept diagnostics on the macOS↔Windows pair where P1 is known; no outcome of this rig changes that obligation, and the rule is likely to spend the run on CONTROLS-NOT-CLEAN.**
