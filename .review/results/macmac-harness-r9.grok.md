Reviewing the Mac↔Mac benchmark instrument at `8830fda`: reading the commit and four files, then driving the engine with synthetic sessions to probe for wrong confident verdicts.Running the guard tests and probing the decision engine with synthetic sessions for false confident verdicts.Checking one more class-(a) angle: whether B should track the range, and whether the unguarded tighten is the only load-bearing step.# Mac↔Mac instrument review — `8830fda` (rev 9)

Guard: **29/29 cases, 10/10 mutations, 4/4 direct rule tests, 300/300 fuzz**. No data taken.

---

## Findings

| # | Sev | File:line | Rationale |
|---|-----|-----------|-----------|
| 1 | **HIGH** | `otp12pf_mac_verdict.py:232` | **B is the wrong residual.** Spec intent: “bias clean controls failed to exclude.” Controls certify on the **full range** at T/2, but B = `max\|CI bound\|`. At n>8 the CI **trims** the same outliers the range still admits → B understates → **false REPRODUCES**. Class (a): fixed CI-trim for NONE, left it on B. At n=8 CI≡range so the bug is latent in the intended run. |
| 2 | **HIGH** | `otp12pf_mac_verdict.py:193` | **`len(d) < PAIRS` only — never `!=`.** Extra valid pairs are graded with n>8 (CI trims). `REQUIRED_PAIRS=16` is refused, but **16 rows + `REQUIRED_PAIRS=8` is accepted**. Harness won’t emit >8; the engine will grade them. |
| 3 | **HIGH** | `otp12pf_mac_verdict.py:311,342`; `PREREGISTRATION.md:276` vs `:297–305` | **Escalation not fully dead.** Verdict prose still says “Re-run at RUNS=16”; PREREG §session still names `RUNS=16` as remedy while §“NO escalation” deletes it. That path is exactly where F1–F2 bite. |
| 4 | **HIGH** | `otp12pf_mac_verdict.py:31–36`; `PREREGISTRATION.md:6–13` | **Authoritative prose still states the pre-rev-9 rule** (`NONE` = CI inside ±T). Body/classify use range. Next “align code with header” reintroduces the round-8 false null. Class (b) cousin: the rewrite executed; the top-of-file rule did not. |
| 5 | **MEDIUM** | `otp12pf_mac_verdict.py:242–247` | **B-tighten is load-bearing and unguarded.** `classify(..., T+B, T−B)` makes NONE *easier* (wide range gate); only the second `if` restores `T−B`. Remove it → **false DOES-NOT-REPRODUCE**. No mutation. |
| 6 | **MEDIUM** | `otp12pf_mac_verdict.py:164–183` (+ design) | **Asymmetry is sound at n=8** (CI≡range; EFFECT needs every pair ≥ T+B). **EFFECT is the weak side only for n>8** (stragglers trimmed) — intentional for positive claims, but live whenever F2 allows n>8. |
| 7 | **MEDIUM** | `bench_otp12pf_mac.sh:844–855` | **Interleave under voids is not contemporaneous.** Slot-major retries in place: one cell can burn many attempts before siblings start the same slot. Shared-window claim degrades exactly when contamination is likely. Does not alone forge a verdict. |
| 8 | **LOW** | `PREREGISTRATION.md:318–320` | Guard counts stale: says 26 cases / 9 mutations; code has **29 / 10**. |
| 9 | **LOW** | `bench_otp12pf_mac.sh:1–17,90–98` | Banner still “round 3”; clearance gate is fine. |
| 10 | **note** | tests | Direct-test replacement for range-NONE mutation is **correct**: at n=8, `median_ci` is k=1 ⇒ CI≡range; a session cannot kill CI↔range. |

**Not findings (checked):** B cannot make EFFECT *easier* (larger B only hardens); dirty controls do not feed B; opposite-sign control bias ≥~T_neg/2 dirties rather than under-B’s; harness ABBA/CSV/meta/bash 3.2 OK; settle measured+voided; drain status on outer pipeline fail-closed; `RUNS` forced to 8 in preflight; 1.10 bar and sign test reported only.

---

## Reproductions

### F1 — B from CI understates residual bias → false REPRODUCES (n=16)

Controls: three pairs at +40, thirteen at +5 (range=[5,40], all inside T/2=50 → clean). CI=[5,5] → **B=5** instead of **40**. Measurand all +110 (T=100).

```text
SESSION VERDICT: REPRODUCES
  nq_grpc_mixed  NONE  ... CI=[+5,+5] range=[+5,+40] ...
  nq_tcp_mixed   EFFECT ... T=100ms B=5ms   # needs only 105; range-B would need 140
```

At n=8 same residual shows B=40 → UNCLEAR (correct).

### F2 — n>8 accepted; EFFECT trims a contradicting pair

```text
# 8×200 + one 10  (n=9, REQUIRED_PAIRS=8)
SESSION VERDICT: REPRODUCES
  nq_tcp_mixed EFFECT CI=[+200,+200] range=[+10,+200]

# same low pair at n=8
SESSION VERDICT: UNCLEAR
```

### F5 — delete B-tighten → false null

Measured +60, controls +49 (B=49, T=100, need max < 51 for NONE):

```text
good engine: SESSION VERDICT: UNCLEAR
no tighten:  SESSION VERDICT: DOES-NOT-REPRODUCE   # false null
```

### F3 — stale RUNS=16 still emitted

```text
the CI spans the threshold ... Re-run at RUNS=16.
```

(`REQUIRED_PAIRS=16` is refused; **16 data rows + REQUIRED_PAIRS=8 is not**.)

### Asymmetry / B direction (sound at n=8)

| Attack | Result |
|--------|--------|
| Outlier +800 of 8 | UNCLEAR (not REPRODUCES) |
| B=0 / 5 / 40 on effect=100 | REPRODUCES → UNCLEAR → UNCLEAR (B only hardens) |
| Opposite control −49 | CONTROLS-NOT-CLEAN (hits T_neg/2 ≈ −45) |
| Bimodal range ±110 | UNCLEAR (range blocks null) |
| n=16 bimodal measurand | UNCLEAR (range blocks null) |

### Guard

```text
29/29 cases passed; 10/10 mutations killed; 4 direct rule tests ok
```

---

## Attack summary

1. **Asymmetry** — Sound at **n=8** (CI≡range). EFFECT is *not* weak there. Weakness is n>8 + no n pin. Direct tests correctly cover range-NONE where sessions cannot.
2. **B** — Directionless `max|·|` is fine; **CI vs range is not.** Should be `max(|rng_lo|,|rng_hi|)` (what T/2 failed to exclude). Cannot game “more bias → easier EFFECT” at n=8.
3. **Interleave** — Structure OK; void-retry accounting OK; contemporaneity soft under voids.
4. **Escalation** — Preflight/engine refuse RUNS≠8 / REQUIRED_PAIRS≠8; **messages and PREREG still prescribe 16**; engine still grades n>8.
5. **Next recurring defect** — (a) CI-trim fixed for NONE, **not for B**. (b) Range-NONE implemented, **headers still document CI-NONE**; B-tighten executes but has **no mutation** (can’t fail the guard if deleted).
6. **Guard** — Non-vacuous for listed mutations. **No mutation** for: range-NONE (direct test OK), B-tighten, B-from-range, exact-n, interleave, dead RUNS=16 text.

---

## VERDICT: **NOT READY TO RUN**

A clean harness session at n=8 is hard to push into a **confident wrong** REPRODUCES/DOES-NOT-REPRODUCE with the current rule math. The instrument is still not safe to clear: **B uses the quantity the rev-9 rewrite just forbade for nulls**, **n is not pinned to 8**, and **the instrument’s own text still points at RUNS=16**, where those holes produce a bankable false REPRODUCES (reproduced). Fix B→range (or pin n≡8 and scrub every RUNS=16 path), enforce exact pair count, fix contradictory prose, and mutation-guard the B-tighten before clearance.
