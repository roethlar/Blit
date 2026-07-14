I'll review the instrument for confident-wrong-answer risk: inspect commit 8830fda, read the four files at HEAD, then drive the engine with synthetic sessions.HEAD is past `8830fda`; I'll compare the four instrument files and finish reading the harness, then attack the decision rule with synthetic sessions.Attacking the decision rule with synthetic sessions and hunting for unguarded paths, dead protections, and B/interleave edge cases.Probing a class-(a) hole: whether the pair-count guard still fails if `!= PAIRS` is weakened to `< PAIRS`.# Mac↔Mac instrument review — workspace HEAD (post-`8830fda`)

**Scope note:** You named HEAD as `8830fda`; this clone’s HEAD is `eb864ac` (9 commits ahead of origin), with four later commits on these four files (exact-n, B-relative, B-from-range, cell-order rotate). Review is of **current tree**, which already includes those r9 closures. Guard: **32/32 cases, 12/12 mutations, 300/300 fuzz** — all green.

---

## Findings

| # | Sev | Location | Finding |
|---|-----|----------|---------|
| 1 | **HIGH** | `otp12pf_mac_verdict.py:37–42`; `PREREGISTRATION.md:6–13` | **Authoritative short-form rule is still rev-8.** Module “FOUR CELL STATES” and the prereg one-paragraph box still say **NONE = CI inside ±T**, with **no B**. Body + `classify()` use **range + B**. Same class as r9 F4: the next “align code to the summary” reintroduces the round-8 false null. Code path for a real run is fine; the load-bearing text is not. |
| 2 | **MEDIUM** | `PREREGISTRATION.md:215–217` | **State table is inconsistent with itself.** EFFECT/INVERTED include `±B`; NONE still says range ⊂ `(T_neg, T_pos)` without B, while the B section (lines 249–250) requires `T±B`. An implementer reading only the table under-tightens nulls. |
| 3 | **MEDIUM** | `otp12pf_mac_verdict.py:200`; `otp12pf_mac_verdict_test.py:284–287` | **Exact-`n` is case-guarded, not mutation-complete.** Live rule is `len(d) != PAIRS` (correct). Weakening to `len(d) < PAIRS` yields **false `REPRODUCES`** on the 16-pair trim session. The SHORT-cell mutation still passes under that weakening (`INCOMPLETE`); only the LONG **case** kills it. Preflight runs cases, so a full guard run still catches it — but the mutation that claims to prove pair-counting does not prove *exact* equality. Class (a) on the guard. |
| 4 | **LOW** | `PREREGISTRATION.md:340–342` | Guard section still claims **26 cases / 9 mutations**; tree has **32 / 12**. |
| 5 | **LOW** | `PREREGISTRATION.md:41–73` vs status “rev 10” | Review ledger stops at round 7; r8/r9 amendments live only in the rule body. |
| 6 | **LOW** | `bench_otp12pf_mac.sh:851–855` + prereg ~268 | Rotation is real and fixes fixed-order controls; with 6 cells × 8 slots, positions are **not** uniform (period-6: slots 1≡7, 2≡8). Spec overclaims “each cell occupies each position” as if balanced. |
| 7 | **LOW** | `bench_otp12pf_mac.sh:19` | Header still cites prereg “rev 4”. |

**No BLOCKER found that yields a confident wrong measurand verdict under the registered experiment (`n≡8`, harness-produced CSVs).**

---

## Attack results (priority order)

### 1. Asymmetry (range for null, CI for effect) — **sound at n=8**

At n=8 the ≥95% order-stat interval is **only** k=1 → **CI ≡ [min, max]** (cov 99.22%). Confirmed 200 random draws + unit identity test.

So for a real session the two sides use the **same numbers**:

| Claim | Condition at n=8 | False claim via outliers/bimodality? |
|--------|------------------|--------------------------------------|
| EFFECT | `min(d) ≥ T+B` | **No** — every pair must clear |
| NONE | full range ⊂ `(T_neg+B, T_pos−B)` | **No** — one straggler → UNCLEAR |
| UNCLEAR | anything else | safe refuse |

Driven:

```text
outlier [10×7, 800]     → UNCLEAR      (not REPRODUCES)
7/8 clear T, one −5     → UNCLEAR
bimodal ±110 @ src=730  → UNCLEAR      (not DOES-NOT-REPRODUCE)
all pairs at T          → REPRODUCES
all pairs at T−1        → DOES-NOT-REPRODUCE   # by design (sub-threshold)
one pair at T, rest 0   → UNCLEAR
half +300 / half −5     → UNCLEAR
```

**EFFECT is not the weak side at registered n** — it is the *stricter* claim (needs every pair). The CI/range split is latent insurance if someone re-registers n>8; engine `REGISTERED_PAIRS=(8,)` refuses that. Direct `classify()` tests (CI≠range) are the right proof; a session mutation cannot kill that distinction at n=8 — **that reasoning is correct**.

Simulated n>8 trim path (if n were allowed): `classify(100,100,-50,100,90,-81)` → **EFFECT** with stragglers — which is the *intended* positive-side tolerance, and is unreachable while n is pinned.

### 2. B (control-bias carry) — **not gameable toward false EFFECT**

- Taken from clean controls’ **range** as **fraction of arm**, scaled to the measurand (`otp12pf_mac_verdict.py:250,258–262`).
- Dirty controls → `CONTROLS-NOT-CLEAN` (blocks EFFECT and null).
- Larger residual → **larger** B → EFFECT harder, null tighter. Opposite-sign residual does **not** ease EFFECT.
- Relative B: 4.9% on a fast control no longer under-penalises a slower measurand (case + mutation).

Driven:

```text
meas +100, ctrl +49     → UNCLEAR      (not REPRODUCES at exact T)
meas +149, ctrl +49     → REPRODUCES   (clears T+B)
meas +130, ctrl +40     → UNCLEAR
meas +130, ctrl 0       → REPRODUCES
meas +60,  ctrl +49     → UNCLEAR      (null tighten; not DOES-NOT-REPRODUCE)
ctrl d=+49 clean; −46 dirty (T_neg uses src/11) — asymmetric, conservative
```

`max |bound|` on the **range** is right for a residual bound; median would understate. At n=8 range≡CI so the old “B from CI” hole is dormant; code already uses `rng`.

### 3. Interleave / void / bash 3.2 — **OK for wrong-answer risk**

- Slot-major + per-slot rotation; ABBA alternates by slot; CSV `run` = slot; both arms share pair `valid`.
- Void → retry in place; pair-level valid (one arm fail voids both); exhausted cell → `complete=no` → session `INCOMPLETE` (does not grade others as a silent null).
- No `declare -A` / `mapfile` / `${x^^}`.
- Exhaustion desynchronises that cell’s window from siblings, but incompleteness refuses a verdict.

### 4. Escalation — **gone**

- Harness: `[[ "$RUNS" == 8 ]] || die`
- Engine: `REGISTERED_PAIRS = (8,)` refuses other `REQUIRED_PAIRS`
- No live escalation / once-marker / data-hash burn paths
- Doc still *mentions* removed RUNS=16 historically; not executable

### 5. Recurring classes — next instances

| Class | This round |
|--------|------------|
| **(a) fixed the branch, not the class** | Exact-`n`: LONG **case** exists, but the **mutation** still only kills total removal of the count check via SHORT — not `!=` → `<` (F3). Stale short-form rule vs body (F1) is the same class in prose. |
| **(b) protection that never executes / can’t fail** | Settle path is now sleep+measure+void+selftest — **executes**. No new dead settle found. Closest cousin: **summary prose that does not constrain the running rule** (F1) — a “protection” that only exists in the wrong paragraph. |

### 6. Guard non-vacuous?

- 12/12 mutations kill; 4 direct rule checks + n=8 identity hold.
- Replacing the range-vs-CI session mutation with direct `classify()` tests is **right** at n=8.
- **No mutation** for: B from CI vs range (dormant at n=8); `!=` vs `<` on pair count (F3); NONE-on-CI in `classify` (covered by direct tests, not mutations).
- Case list **does** cover LONG n=16 → `INCOMPLETE`.

---

## Reproductions

```bash
# Guards (pass)
python3 scripts/otp12pf_mac_verdict_test.py          # 32/32 + unit + fuzz
python3 scripts/otp12pf_mac_verdict_test.py --mutations  # 12/12 KILLED

# Timed run still gated
bash scripts/bench_otp12pf_mac.sh   # exit 2, CLEARED_BY_REVIEW refuse

# Weakening exact-n → false REPRODUCES (mutant of live code)
# len(d) != PAIRS  →  len(d) < PAIRS, then 16-pair session:
#   REAL:  INCOMPLETE
#   MUTANT: REPRODUCES   # three −500 trimmed, thirteen +200 → CI clears T
# SHORT mutation case still INCOMPLETE under the same mutant → mutation vacuous for this weakening
```

Synthetic EFFECT/null probes (all on live engine): see attack tables above; no false `REPRODUCES` / false `DOES-NOT-REPRODUCE` found for harness-shaped n=8 data.

---

## VERDICT: **READY TO RUN**

Under the registered design (`n≡8`, interleaved harness, pinned constants), I could **not** force a confident wrong `REPRODUCES` or `DOES-NOT-REPRODUCE`. The asymmetry is dormant but sound; EFFECT is min-based (strong); B hardens rather than softens; escalation is actually gone; settle is measured and can void.

Clear with eyes open on **F1** (rewrite the short-form rule and module header so they cannot be used to “correct” the code back to a false null) and **F3** (mutation should kill `len < PAIRS`, not only full deletion). Those are reintroduction / guard gaps, not a live false-answer path on this tree.
