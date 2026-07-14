I'll attack the rewrite as an instrument that must not produce a confident wrong answer: inspect the commit and the four files, then drive the engine with synthetic cases.Files are in hand. Next I'll drive the engine with synthetic sessions and probe the harness protections that tend to fail open.Guard tests pass. Driving synthetic attacks on bimodality, the sign-test claim, bar/NONE disagreement, and control preconditions.Dead-zone and control-bleed look serious. Digging into multi-control probability, harness fail-opens, and whether `ratio` can diverge from the paired rule.Checking one harness fail-open candidate: whether `timed_run` still accepts drain via a loose glob after the strict case filter, and whether fsync errors can be swallowed.# Mac↔Mac instrument review — rev 8 / `79c1f2d`

Guard tests: **26/26 cases, 9/9 mutations, 300/300 fuzz** — all pass. That does not make the instrument safe; several failure modes sit outside what the suite mutates.

---

## Findings

| # | Location | Sev | Finding |
|---|----------|-----|---------|
| 1 | `otp12pf_mac_verdict.py:155-162,191,237` + n=8 CI=`[min,max]` | **BLOCKER** | **Dead zone.** Controls must be `NONE` at **T/2** with n=8 CI = full range: **one** pair with \|d\| ≥ T/2 fails a control. At stated 2–4% arm noise (paired σ ≈ that×√2), P(one control clean) ≈ 0.45 / 0.10 / 0.02; P(all four independent) ≈ 0.05 / 0 / 0. Escalation n=16 helps at ~2% (≈0.95) and fails at 3–4% (≈0.28 / 0.01). Correlated controls still leave n=8 as a coin flip at 2%. Instrument systematically cannot conclude; “only CONTROLS-NOT-CLEAN” is a broken instrument. |
| 2 | `otp12pf_mac_verdict.py:52-55,164-174`; claim in docstring | **BLOCKER** | **T/2 still certifies a half-Δ_ref confound.** Slow arms (T=230): controls with **d=+114** → `ctrl=NONE`; measurand **d=+230** → `REPRODUCES`. Driven: all four controls carry **49.6% of Δ_ref** and the session still prints a confident reproduction. Same class as “229 of 230 certified clean”, half-fixed not fixed. Specificity (TCP-only / mixed-only) is not enforced. |
| 3 | `otp12pf_mac_verdict.py:52-55` (“bimodal widens CI”) | **HIGH** | **UNSTABLE deletion is unsound.** Bimodality does **not** always widen the paired CI. Lockstep modes → narrow CI. Constructed pf-0-style src **730/840**, dest **800**: CI=`[-40,+70]`, T≈73 → cell `NONE` → session **`DOES-NOT-REPRODUCE`** (“genuine equivalence”) while the source arm is classically bimodal. Summary still has the numbers; the **verdict label does not refuse**. |
| 4 | `otp12pf_mac_verdict.py:49-51`; n=16 path | **HIGH** | **“Sign test redundant” is false at n=16** (registered escalation). At n=8, CI=`[min,max]` ⇒ `CI_lo≥T` ⇒ all pairs ≥T. At n=16, CI=`[d_(4),d_(13)]`: data `[-500,-400,-300]+[250]×13` → **`REPRODUCES`**, sign 13/16, three pairs strongly inverted. Median claim is coherent; the written redundancy claim and “every pair clears T” intuition are not. |
| 5 | `otp12pf_mac_verdict.py:302-311` | **MEDIUM** | **Subthreshold NOTE is noisy.** Fires on control d=+5 (sign_p=0.008) and on the half-P1 control bleed (d=+114). Real measurand notes can be buried; tiny control noise is called “NOT nothing.” |
| 6 | `bench_otp12pf_mac.sh:717-721` vs `:732,:814` | **MEDIUM** | **Drain class half-fixed.** Producer now tokenizes (`case` single token) — multi-line `drained_*\nDRAIN-ERROR` is **rejected** there. Consumers still use **`[[ "$RUN_DRAIN" == drained* ]]`**, which **still matches multi-line** if the producer ever regresses or a second path sets `RUN_DRAIN`. Fixed the site you were shown, not every consumer. |
| 7 | `otp12pf_mac_verdict_test.py` mutations | **MEDIUM** | **Guard is thin.** 9 mutations cover 9/26 cases. No mutations for: bar-not-in-inference (1 ms / bar-fail cases), most dirty-control history, MIXED/one-direction, UNCLEAR-vs-null, n=1 CI, dead-zone “still answers”, etc. Cases-only was a prior fail-open class. **Harness has no mutations at all.** |
| 8 | `bench_otp12pf_mac.sh:488-533` | **LOW** | Escalation **code** triggers on `UNCLEAR\|CONTROLS-NOT-CLEAN`; **comments/log** still say `INCONCLUSIVE-UNDERPOWERED`. Doc/code drift, not a re-roll hole. |
| 9 | `otp12pf_mac_verdict.py:164-174` | **—** | Four states are **exhaustive and mutually exclusive** (grid over CI vs ±T; no collisions). OK. |
| 10 | Dirty-control suite | **—** | Dirty / UNCLEAR / mixed controls → **`CONTROLS-NOT-CLEAN`** for both null and reproduction paths. OK for that question. |
| 11 | Bar vs NONE @ ratio 1.30 | **—** | With T=`min(src/10,230)`, a cell that is `NONE` cannot carry marginal ratio ~1.30 under the paired model. Bar-out-of-inference is safe here; subthreshold is the residual (NOTE). |

---

## Reproductions

### A. Half-Δ_ref controls + REPRODUCES (finding 2)

```text
measurand d=[230]×8, src=2500
control   d=[114]×8, src=2500
→ SESSION VERDICT: REPRODUCES
  controls: ctrl=NONE  D=+114  T=230  (T/2=115)
  measurands: EFFECT   D=+230
```

### B. Bimodal arm, confident null (finding 3)

```text
src ∈ {730,840}, dest=800 (paired d ∈ {70,-40})
→ DOES-NOT-REPRODUCE, wording: "genuine equivalence"
  nq_tcp_mixed NONE  CI=[-40,+70] T=73
```

### C. n=16 sign discordance (finding 4)

```text
d=[-500,-400,-300]+[250]×13, src=2500, pairs=16
→ REPRODUCES, sign_p with 13/16 (three large negatives outside CI)
```

### D. Dead zone (finding 1) — Monte Carlo

```text
src=1000, noise σ_arm=2% of arm, n=8:
  P(one control NONE at T/2) ≈ 0.45
  P(all 4 independent clean) ≈ 0.05
noise=3%: P(all 4) ≈ 0
n=16, noise=2%: ≈ 0.96; noise=4%: ≈ 0.01
```

### E. Drain consumer still fail-opens on multi-line (finding 6)

```bash
raw=$'drained_3x2s\nDRAIN-ERROR'
[[ "$raw" == drained* ]] && echo FAIL_OPEN   # prints FAIL_OPEN
# drain_host case rejects the same string today; timed_run/prep_run would not.
```

### F. Dirty rig (clean result)

Controls at T/2, UNCLEAR, mixed, INVERTED, or EFFECT all → `CONTROLS-NOT-CLEAN` (including blocking `REPRODUCES`).

---

## What simplification got right

- Four states really partition the CI; no precedence stack to walk past.
- Controls gate **both** reproduction and null.
- Bar not used for inference (and ratio 1.30 + NONE is not reachable under this T).
- Sign out of the n=8 decision is fine; the bug is the **claim** and n=16.
- `DELTA_REF` pin, data-not-meta completeness, session void precedence: hold under drive tests.
- Settle is measured in-process; selftest checks elapsed ms (structure looks executable).

---

## VERDICT: **NOT READY TO RUN**

Two independent ways to a **confident, over-strong answer** remain: (1) **`REPRODUCES` while every control carries ~½ Δ_ref** under T/2 on slow arms; (2) **`DOES-NOT-REPRODUCE` on a clearly bimodal arm** because UNSTABLE was deleted and lockstep/bimodal differences need not widen the paired CI. Separately, the **T/2 + n=8 full-range control gate** makes a clean, bankable session unlikely at the stated noise — so the instrument may also be **unable to conclude**, which the owner already defined as broken.

Minimum before data: raise or redesign control certification (not “half of a capped 230 ms” on slow arms), restore a **data-quality** refuse for arm multimodality (or prove lockstep is impossible under ABBA), and re-check dead-zone power with a **stated noise model** (σ vs range) plus either n=16-first for controls or a less brittle control statistic than `[min,max]` at T/2.
