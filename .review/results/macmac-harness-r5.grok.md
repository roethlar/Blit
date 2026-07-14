I'll re-ground in project state, then re-drive every prior finding and hunt for defects the fixes may have introduced.Shell was blocked; I'll use ptk and locate the instrument code to re-drive the nine findings.Running the guard suite and mutations, and reading the harness for the new paths.Tests pass; next I'll re-drive each finding with synthetic sessions and dig into control-void edge cases and harness paths.Writing a driver script and running attack cases via ptk.# Round-5 verification of `a9460ce` (Mac↔Mac instrument)

Guard suite on this tree: **17/17 cases**, **11/11 mutations killed**, fuzz 300/300. Timed-run gate still refuses without `CLEARED_BY_REVIEW=1` (`exit 2`). **No rig data taken.**

---

## Part 1 — Verification of prior findings

| # | Finding | Status | Evidence |
|---|---------|--------|----------|
| 1 | **BLOCKER**: Δ_ref-sized control escapes RIG-VOID → clean VANISHES | **FIXED** for the demonstrated case; **rule incomplete** (see New #1) | Original re-drive: measurand noise + controls `d_i=230×8 @ src=2500` → **`RIG-VOID`**, all four controls `PARTIAL bar=PASS D=+230`. Tiny host×role `d=5` → **`VANISHES`** (does not false-void). |
| 2 | **HIGH**: trusts `meta.complete`, never counts pairs / coverage | **FIXED** | `n=1` + `complete=yes` → **`INCOMPLETE`** (not VANISHES). `complete()` requires `len(paired) >= REQUIRED_PAIRS` and `cov >= 0.95`. |
| 3 | **HIGH**: session precedence hid one-direction REPRODUCES | **FIXED** | `nq=REPRODUCES 8/8` + `qn=BAR-FAIL-INCONSISTENT` → **`REPRODUCES`** with sibling caveat. `MIXED-SIGN` still wins over single-sided repro. |
| 4 | **HIGH**: end-load could not void | **FIXED** | `SESSION_VOID_REASON=end-load…` → **`RIG-VOID`** even when measurand data would REPRODUCE. Harness `end_load_gate()` sets the env the engine reads. |
| 5 | **MEDIUM**: preflight ran cases not mutations | **FIXED** | `preflight()` runs both `otp12pf_mac_verdict_test.py` and `--mutations` (lines 418–421). |
| 6 | **MEDIUM**: n=8 CI always `[min,max]` → null-incapable | **FIXED** (by design) | One noisy pair → `INCONCLUSIVE-UNDERPOWERED`. Escalation `RUNS=16` registered; at n=16 a single outlier no longer blocks null (CI `[d₄,d₁₃]`). See New #3 on p-hackability. |
| 7 | **MEDIUM**: “CI and sign are duals” false once a zero exists | **PARTIALLY** | Prereg corrects the dual claim; session BAR-FAIL text now says “CI includes 0, or sign does not reject.” Module docstring still says “pairs do not agree in sign” (`otp12pf_mac_verdict.py:76,279`). Guard-test comment still says “mathematical DUALS.” Behavioral hole remains (New #2). |
| 8 | **MEDIUM**: control caveat called D=+230 “sub-bar” | **FIXED** for controls | D=+230 controls now **void** (no caveat path). Residual: session **PARTIAL** text still says “sub-bar asymmetry” for measurand D=+230 @ 2500 (`:443`) — same wording class, different path. |
| 9 | **MEDIUM**: mutation proof incomplete; fuzz pinned controls | **FIXED** | 11 mutations incl. round-4 holes; fuzz perturbs controls. |
| LOW | Non-numeric `ms` | **FIXED (modified form)** | `ms='NaN'` → **exit 2**, stderr `CORRUPT ROW…`. **Agree with the modification** — soft `INCOMPLETE` would hide corruption. |

### Finding #1 rule — is it correct? Can it false-void a good rig?

**Principle is right**: controls must pass the same absolute equivalence test as the measurand (`null_excl` via `min(bar_breach, Δ_ref)`), not the ratio bar alone. Tiny host×role (q faster) correctly survives (`d=5` → VANISHES with caveat).

**Implementation is not the principle.** `_ctrl_dirty` only treats **PARTIAL ∧ ¬null_excl** as dirty beyond bar/unstable/repro outcomes. It does **not** treat **UNDERPOWERED** (fails equivalence) as dirty — so “clean ⇔ passes equivalence” is false in code and half-false in the prereg (which both claims that definition and then exempts UNDERPOWERED to a caveat).

**False void of a good rig: yes.** One control outlier voids the session even when the median is trivial:

- controls `d=[1,1,1,1,1,1,1,250]` → D=+1, CI=[+1,+250], PARTIAL, ¬null_excl → **`RIG-VOID`**

That fails closed (no false science claim), but a good rig with one noisy control pair dies. At n=8 that is structural: CI is the full range.

---

## Part 2 — What the fix broke / still open (new findings)

### NEW-1 — **BLOCKER** — Dirty controls still escape via UNDERPOWERED → false **VANISHES**
**File:** `scripts/otp12pf_mac_verdict.py:340-348, 351-354`

**Same structural habit as rounds 3–4:** absolute materiality was applied to the PARTIAL control path and not to the rest of “not clean.”

**Drive (clean measurand; every control D=+230 with one −10 pair):**

```
controls d = [230,230,230,230,230,230,230,-10] @ src=2500
→ control: UNDERPOWERED  ratio=1.092 bar=PASS  D=+230  CI=[-10,+230]  7/8 pos
→ SESSION VERDICT: VANISHES
```

One slightly negative pair kills `pos_effect` (CI includes 0), so the control is UNDERPOWERED rather than PARTIAL; UNDERPOWERED is only a **caveat**, not a void. Headline null while every control still carries a Δ_ref-sized median on a slow arm — the original blocker, one branch down.

Prereg §session RIG-VOID says clean means the control “passes the same equivalence test”; UNDERPOWERED **fails** that test. Code and prereg disagree; the unsafe side wins.

---

### NEW-2 — **HIGH** — Zero pair + near-bar consistent positives → false **VANISHES**
**File:** `scripts/otp12pf_mac_verdict.py:252-256, 272-285`

Because `pos_effect` requires `CI_lo > 0` and at n=8 the CI is `[min,max]`, **any single non-positive pair prevents effect detection**. Equivalence still fires if `max < margin_hi`.

**Drive:**

```
measurand d = [0,99,99,99,99,99,99,99] @ src=1000
→ VANISHES  ratio=1.099 bar=PASS  D=+99  CI=[0,99]  sign_p=0.0156 (7/7 pos)
```

Sign test **rejects** (real one-sided pattern); engine still claims equivalence / “P1 vanishes.” Finding #7’s dual gap is not only documentary: the null path does not require “sign does not reject.”

---

### NEW-3 — **MEDIUM** — `RUNS=16` escalation is socially gated only (p-hackable)
**File:** `scripts/bench_otp12pf_mac.sh:406-407`

Prereg: escalation only after an `INCONCLUSIVE-UNDERPOWERED` session, and must **name** that session. Code: any `UNDERPOWERED_ESCALATION=1` enables n=16. No check of a prior verdict path, no once-only lock, no stored session id. Operator (or agent) can re-roll a disliked n=8 result under a larger, outlier-tolerant CI.

---

### NEW-4 — **MEDIUM** — Session PARTIAL still says “sub-bar” for Δ_ref-sized measurand effects
**File:** `scripts/otp12pf_mac_verdict.py:441-444`

```
d=230×8 @ src=2500 → PARTIAL  ratio=1.092  D=+230
why: "a real but sub-bar asymmetry"
```

Finding #8 fixed the control caveat; the same soft language remains on the measurand session path for effects that are only bar-sub because the arm is slow.

---

### NEW-5 — **MEDIUM** — Control void can false-void a good rig (one outlier)
**File:** `scripts/otp12pf_mac_verdict.py:340-348` + n=8 full-range CI

Covered under #1 rule analysis. Severity lower than NEW-1 (RIG-VOID, not false null), but still a broken instrument for expected control noise.

---

### NEW-6 — **LOW** — Doc/comment drift after finding #7
- `otp12pf_mac_verdict.py:76,279` still “pairs do not agree in sign”
- `otp12pf_mac_verdict_test.py:237-241` still “mathematical DUALS”
- Prereg dual correction is correct; engine/test lag

---

### Hunted items — summary

| Hunt target | Result |
|-------------|--------|
| bar-vs-Δ_ref remaining | Measurand margin OK. Control PARTIAL path OK. **UNDERPOWERED controls and session PARTIAL wording still wrong.** |
| REPRODUCES outranks UNSTABLE/BAR-FAIL | Does **not** leak null (VANISHES needs all cells). Can report REPRODUCES with messy sibling — intentional per prereg “either direction.” MIXED-SIGN still outranks. |
| RUNS=16 | Legitimate power gain works; **p-hackable** (NEW-3). |
| Coverage / pair count | Rejects n=1, n=7, valid=no thinning, cov&lt;95%. Accepts valid n=8/16. |
| End-load void | Works. Note: **INCOMPLETE outranks** SESSION_VOID (short session + end-load → INCOMPLETE, not RIG-VOID) — fails closed, can hide *why*. |
| Gates pass while broken | Guard/mutations solid for *known* holes; **do not cover NEW-1/NEW-2**. SELFTEST does not exercise RUNS=16 naming. |

---

## Reproductions (commands & outcomes)

Engine driven via synthetic CSVs (same shape as the guard harness). Core outcomes:

| Case | Command shape | Result |
|------|---------------|--------|
| Original blocker | controls `[230]*8` @ 2500, measurand noise | **`RIG-VOID`** |
| NEW escape | controls `[230]*7+[-10]` @ 2500, measurand noise | **`VANISHES`** |
| Zero false null | measurand `[0,99×7]` @ 1000 | **`VANISHES`**, sign_p=0.0156 |
| Tiny control | controls `[5]*8` | **`VANISHES`** (good) |
| False void | controls `[1×7,250]` | **`RIG-VOID`** |
| n=1 / meta lie | `d=[0]`, complete=yes | **`INCOMPLETE`** |
| End-load | `SESSION_VOID_REASON=…` | **`RIG-VOID`** |
| Corrupt ms | non-numeric `ms` | **exit 2** |
| Guard | `python3 scripts/otp12pf_mac_verdict_test.py{, --mutations}` | **17/17**, **11/11** |

---

## Assessment

Nine prior findings are closed in the places they were demonstrated. The rework again fixed **the branch that was shown**, not the class:

1. Round-3: bar-tied margin on **measurand**, left on controls.  
2. Round-4: absolute materiality on control **PARTIAL**, left **UNDERPOWERED** controls able to carry D=+230 and still allow **VANISHES**.  
3. Plus a zero-boundary null path that claims equivalence while the sign test rejects a near-bar effect.

I agree with the loud `exit 2` for corrupt rows.

---

**VERDICT: NOT SAFE TO RUN**
