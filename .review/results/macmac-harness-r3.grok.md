Independent review of the Round 3 instrument: I'll read the commit and specs, then drive the verdict engine with synthetic sessions to try to force wrong answers.Reading the four instrument files and the decision rule carefully.Reading the harness for timer, fail-closed gates, and preflight, then driving the verdict engine with adversarial sessions.Digging into pair-count enforcement, timer code, and fail-closed gates — then constructing adversarial sessions.Probing the control-PARTIAL hole, precedence conflicts, n-pair trust, and harness/load issues.Checking a few remaining harness/spec mismatches, then writing the findings.# Round 3 independent review — `cae2e0f`

Independent of any `.review/results/*r3*` material. Scope: can this instrument emit a **confident wrong** session verdict?

Guard suite on this tree: **12/12 cases**, **7/7 mutations killed**, **300/300 fuzz** outcomes registered. That proves the *listed* regressions stay dead; it does **not** prove the instrument is closed.

---

## Findings

| file:line | severity | finding |
|---|---|---|
| `scripts/otp12pf_mac_verdict.py:289-294` + `:301-307` | **BLOCKER** | **RIG-VOID still fails open for a dirty control that the bar forgives.** Void only if `bar==FAIL` or outcome ∈ {UNSTABLE, REPRODUCES, INVERSION, BAR-FAIL-INCONSISTENT}. A control with a **real, 8/8, rig-W-sized effect** (`d_i=230` every pair) on a slow arm (`src=2500`, ratio **1.092**, bar **PASS**) is only **PARTIAL**, so it does **not** void. Session still emits **VANISHES** with a “CONTROL CAVEAT”. Same structural hole as rev‑3’s bar-tied margin: on a slow arm the bar is wider than Δ_ref, so a full Δ_ref control effect is labeled “sub-bar” and escapes. Guard/mutations never cover this path (they only kill bar-FAIL escape). |
| `scripts/otp12pf_mac_verdict.py:119-125`, `:192-196`, `:277-280` | **HIGH** | **Engine trusts `meta.complete==yes` and never requires n=8 or ≥95% CI coverage.** `complete()` only needs ≥1 valid pair. With `complete=yes` and **n=1** zero-diff (or n=3/4), engine emits **VANISHES** at **ci_coverage=0% / 75% / 87.5%**. Harness *usually* only marks complete at 8 valids, but the mechanized rule is still fail-open if meta is wrong, partial CSVs are regraded, or any future caller lies. A null with coverage 0% is a confident false equivalence claim. |
| `scripts/otp12pf_mac_verdict.py:318-340` vs prereg “either direction” | **HIGH** | **Session precedence can hide a clean one-direction REPRODUCES.** Pre-reg: reproduction in *either* direction answers the question. Code ranks **BAR-FAIL-INCONSISTENT** and **UNSTABLE** above **REPRODUCES**. Driven: `nq_tcp_mixed=REPRODUCES` (8/8, +300…370) + `qn_tcp_mixed=BAR-FAIL-INCONSISTENT` → session **BAR-FAIL-INCONSISTENT**, not REPRODUCES. Not a false null, but a **false non-reproduction** against the registered “either direction” rule. |
| `scripts/bench_otp12pf_mac.sh:742-745` + comment at 742-743 | **HIGH** | **End-load cannot void the session.** Comment says end-load is recorded before verdict so a session *can* void on it; code only `log`s `load1 (end)` then `compute_verdicts`. Start load dies; end load never gates. A mid-session load spike (the contamination mode the preflight exists to stop) still yields a full computed verdict. |
| `scripts/bench_otp12pf_mac.sh:402-403` vs `689-693` | **MEDIUM** | **Preflight runs the case suite, not `--mutations`.** SELFTEST kills vacuous guards; a timed run’s preflight does not. Vacuous reintroduction of a “fixed” rule can pass preflight if cases still pass for other reasons. |
| `scripts/otp12pf_mac_verdict.py:134-156`, prereg CI section | **MEDIUM** | **At n=8 the CI is always `[min,max]` (99.22%).** Correct math; deliberate. Practical cost: **one** noisy pair with \|d\| ≥ margin blocks VANISHES forever → session stuck at **INCONCLUSIVE-UNDERPOWERED**. That is not a false claim, but a null-incapable instrument if pair noise is ≥ ~src/10. Reproduction still possible. |
| `scripts/otp12pf_mac_verdict.py:214-215` + prereg dual claim | **MEDIUM** | **CI and sign test are not duals once zeros exist.** Example: `d=[0,300…360]`, bar FAIL, `sign_p=0.0156` (7/7 pos) but `CI_lo=0` → not `pos_effect` → **BAR-FAIL-INCONSISTENT**. Conservative against false REPRODUCES; the session text still says pairs “do not agree in sign,” which is **false** here (nonzeros all agree; a zero blocked the CI). |
| `scripts/otp12pf_mac_verdict.py:365-368` (caveat copy) | **MEDIUM** | Caveat always calls control PARTIAL/UNDERPOWERED a “**real sub-bar asymmetry**.” Driven PARTIAL controls with **D=+230, CI=[230,230]** are not “sub-bar” in any absolute sense—only ratio-sub-bar. Softens a dirty-rig signal into “sub-bar noise.” |
| `scripts/otp12pf_mac_verdict_test.py:53-55,160-172` | **MEDIUM** | **Mutation proof is faithful for the seven strings it patches** (targets present; each revert produces the forbidden verdict). It is **incomplete**: no mutation for PARTIAL/Δ_ref control void, no n&lt;8 complete guard, no precedence/either-direction case. Fuzz only perturbs the measurand; controls stay `[5]*8` (themselves PARTIAL). |
| `scripts/otp12pf_mac_verdict.py:107` (`int(r["ms"])`) | **LOW** | Non-numeric `ms` **crashes** the engine (exit 1) rather than mapping to INCOMPLETE. With `set -e` the harness aborts (fail-closed for the run), but this is not a soft taxonomy outcome. |
| Timer / gates (harness) | *(fixed / OK)* | Same-process `time_argv`, timer preflight, non-positive transfer VOID, drain device resolved, pgrep/top/iostat fail-closed shapes, pair-void, durability count+bytes, clean-build pin, CLEARED_BY_REVIEW refuse — look sound on read + local same-process sleep check. **Not re-proven on nagatha↔q this review.** |

---

## Reproductions (driven)

### R1 — BLOCKER: VANISHES while every control has a full Δ_ref effect (bar PASS)

Synthetic session: measurand noise `[-4,-2,-1,0,0,1,2,3]` @ src=2000; all four controls `d_i=230` ×8 @ src=2500.

```
SESSION VERDICT: VANISHES
CONTROL CAVEAT ... nq_grpc_mixed(PARTIAL), ... (does not void)
  nq_grpc_mixed  PARTIAL  ratio=1.092 bar=PASS  D=+230ms CI=[+230,+230] (99.2%) sign_p=0.0078 (8/8)
  nq_tcp_mixed   VANISHES ratio=1.000 bar=PASS  D=+0ms   CI=[-4,+3]
  … same pattern on qn_* and both large controls
```

Headline null while controls show the exact effect size the power gate is built around.

### R2 — Regression: bar-FAIL control still voids (good)

Same clean measurand; control `[-100,-50,300…380]` @ 1000 → **RIG-VOID**. Round‑2 hole stays closed for *bar FAIL*.

### R3 — HIGH: n=1 + `complete=yes` → VANISHES @ 0% coverage

```
SESSION VERDICT: VANISHES
  nq_tcp_mixed VANISHES ... CI=[+0,+0] (0.0%)
```

### R4 — HIGH: one-direction REPRODUCES masked

`nq_tcp_mixed` REPRODUCES (300–370ms) + `qn_tcp_mixed` BAR-FAIL-INCONSISTENT → session **BAR-FAIL-INCONSISTENT**, not REPRODUCES.

### R5 — Measurand margin fix holds (good)

All eight `d_i=230` @ src=2500 on the measurand → **PARTIAL**, not VANISHES (rev‑3 killer stays dead).

### R6 — Sign/CI dual break

`[0,300…360]` @ 1000: `sign_p=0.0156 (7/7 pos)`, `CI=[0,360]` → **BAR-FAIL-INCONSISTENT**, with text claiming sign disagreement.

### Stats spot-check

| claim | computed |
|---|---|
| n=8, k=1 coverage | **99.21875%** (matches “99.22%”) |
| n=8, k=2 coverage | **92.96875%** (old bootstrap interval) |
| sign n=8, k=8 | p=**0.0078125** |
| sign n=8, k=7 | p=**0.0703125** (not &lt;0.05) |
| BREACH_LO | `-src/11` (not `-src/10`) — case still underpowered correctly |
| MARGIN | `min(breach, 230)` / `max(breach_lo, -230)` as coded |

### Mutation faithfulness

All seven mutation old-strings exist in the engine; each revert produced the forbidden verdict (`VANISHES` or `PARTIAL`/`REPRODUCES` as named). **Not vacuous for those seven fixes.** Missing coverage is the defect, not fake kills.

### Harness vs pre-reg (short)

| area | match? |
|---|---|
| Timer one-process + preflight proof + non-positive VOID | yes |
| RUNS fixed 8; incomplete if &lt;8 valid | harness yes; **engine no** |
| RIG-VOID on control bar FAIL | yes |
| RIG-VOID on control absolute/Δ_ref effect | **no** |
| End load can void | **documented intent no; code no** |
| Either-direction REPRODUCES | **cell rule yes; session precedence no** |
| Fail-closed probes (pgrep/top/iostat/TM/load start) | yes on read |
| CLEARED_BY_REVIEW blocks timed data | yes |

---

## What I could *not* force

- **False REPRODUCES** from the decision rule alone without planting a real 8/8 positive material effect (good).
- **VANISHES** while the *measurand* has a consistent rig-W effect (margin + effect-before-equivalence hold).
- Bar-FAIL control escape (fixed).
- Missing registered cell → still INCOMPLETE (fixed).

Residual physical artifact (settle/ssh ~15 ms) is documented; unlikely alone to manufacture material REPRODUCES on mixed-scale arms, but it was not re-measured on the rig here.

---

## VERDICT: **NOT SAFE TO RUN**

The round‑2 RIG-VOID class is only **half** closed: bar-FAIL controls void; **Δ_ref-sized PARTIAL controls do not**, and I drove a clean **VANISHES** while every control sat at **D=+230 / 8/8 / ratio 1.092**. Until control voiding uses the same absolute materiality the power gate claims (or voids any control `pos_effect` / `ci_lo ≥ DELTA_REF`), plus engine-side **n=8 & coverage** enforcement, this instrument can still print a confident null on a dirty or under-specified session.
