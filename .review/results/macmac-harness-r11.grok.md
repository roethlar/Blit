I'll review the decision rule, its tests, the pre-registration, and the harness for confident-but-wrong verdicts. Reading those files now.Harness was truncated; checking CSV schema, remaining harness logic, and probing edge cases with Python.Probing B-edge cases, mutual exclusivity, harness/CSV pairing, and doc–code mismatches.Checking one more harness detail: whether invalid rows can still bias pairing, and whether the prereg summary omits B.## Prior-review items

| Claim | Status |
|---|---|
| Cell identities/roles pinned; env mismatch refused | **Closed** — code + case + mutation |
| Completeness = exactly 8 pairs **and** 8 rows/arm | **Code closed; arm-count unguarded** (below) |
| Non-positive timings refused | **Closed** in code; mutation weak (below) |
| Guard claim: 14/34 mutated, rest behavioural, 2 direct because CI≡RANGE at n=8 | **Closed** — claim accurate; n=8 reasoning correct |

Tests: **34/34**, **14/14 mutations killed**, **300/300 fuzz**, 5 direct rule checks pass.

---

## Findings

### `scripts/otp12pf_mac_verdict.py`

| Loc | Sev | Finding |
|---|---|---|
| 216–218 | **HIGH** | **Arm-count completeness is load-bearing and unguarded.** Code requires `len(by[srcinit])==8` and `len(by[destinit])==8`. Removing only those two conjuncts still passes **all 34 cases** and is **not** what the completeness mutation isolates (that mutation kills on a short `len(d)`). Synthetic CSV: 8 pairs with `d=50` on `src=1000` (true `T=100` → null) plus extra valid low `srcinit` rows → mutant reports **`REPRODUCES`**; unmutated engine correctly **`INCOMPLETE`**. Same defect class as prior vacuous guards. |
| 186–206 vs 282–287 | **MEDIUM** | **NONE thresholds applied twice, inconsistently.** `classify(..., t_pos+B, t_neg-B)` makes the NONE window *wider*; a second check then tightens to `T±B`. Net effect is correct (B only hardens) and the tight path is mutated, but the intermediate call encodes the wrong null rule — easy to “simplify” into a real bug. |
| 37–42 (module docstring) | **MEDIUM** | Docstring still states NONE as `-T < CI_lo and CI_hi < +T` with **no B and no RANGE**. Contradicts `classify`, pass 3, and the prereg body. |
| 200–206 + 279 | **MEDIUM** | **`B` can exceed `T`, making a null impossible.** Fast control near `T/2` (e.g. `control_d=24`, `control_src=500` → `B_frac≈0.048`) on a slow measurand (`src=5000`, `T=230`, `B≈240`): zero effect → **`UNCLEAR`**, not `DOES-NOT-REPRODUCE`. Conservative (not a false null/effect), but unregistered as a structural dead-zone when arm speeds differ a lot. |
| 200 / zeros→`classify(0,…)` | *(closed)* | `classify(0,0,0,0,0,0)` is **`EFFECT`**. Refused by `ms_of` non-positive check. With clean non-zero controls and zero measurand arms, dropping that check yields **`REPRODUCES`**. |

**Q1 — Exclusive & exhaustive?** Yes for the four cell states under `src>0` (hence `T_pos>0`). Grid/fuzz: the only multi-predicate hit is the degenerate `T=B=0` zero-timing case, which is refused. Session outcomes are a separate, ordered partition.

**Q2 — B sound both ways?** Hardens EFFECT (`T+B`) and INVERTED (`T_neg−B`); null post-check hardens to `T−B`. Does not make any decisive verdict easier. Can license an effect only if residual bias exceeds what clean controls’ ranges imply (model risk). Can exceed `T` (above). Zero only when clean control ranges are 0 (or no clean control contributes — then session is incomplete/dirty).

**Q3 — CI vs RANGE?** At `n=8` they are the same two numbers; nothing in live grading depends on them differing. EFFECT is **not** weaker at registered `n`. At `n>8` EFFECT *would* be weaker (trimmed CI); engine refuses other `n`. Direct unit test correctly encodes the RANGE-null semantics.

**Q4 — `n` exactly 8?** Enforced via `REGISTERED_PAIRS=(8,)`, `len(d)==PAIRS`, arm counts, harness `RUNS==8`. Silent n=8 assumptions (sign-test “every pair clears T”, CI≡RANGE) match registered `n` only — OK while refuse holds.

### `scripts/otp12pf_mac_verdict_test.py`

| Loc | Sev | Finding |
|---|---|---|
| (no case) | **HIGH** | **No case and no mutation for the arm-count conjunct** (see above). Combined completeness mutation only proves “don’t trust `meta.complete` alone / count pairs.” |
| 247–249, 320–322 | **LOW** | Zero-timing case sets **all** arms to 0; mutant becomes `CONTROLS-NOT-CLEAN`, not `REPRODUCES`. Mutation is non-vacuous on the `ENGINE-REFUSED` contract but does **not** exercise the dangerous zeros→`REPRODUCES` path (needs clean positive controls + zero measurand). |
| 334–384 | *(OK)* | Reasoning for 2 direct guards is **correct**: at `n=8`, CI and RANGE are identical; no synthetic session can kill a “use CI for null” mutation. Identity check (200 draws) holds. |

**Fixes with no mutation (among important ones):** arm-count completeness; RANGE-vs-CI null (direct only — justified); most behavioural outcomes (REPRODUCES/MIXED/INVERTED paths) — fine as behavioural.

### `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`

| Loc | Sev | Finding |
|---|---|---|
| 6–13 vs 220–250 | **MEDIUM** | **Opening “rule in one paragraph” is stale:** NONE via **CI** inside ±T; no RANGE; no B; no `−src/11`. Body (rev 8+) is what the code implements. Same class as prior doc≠code defects. |
| 214–218 | **LOW** | State table lists NONE as full range inside `(T_neg, T_pos)` **without B**; B section correctly requires `T−B`. |

### `scripts/bench_otp12pf_mac.sh`

| Loc | Sev | Finding |
|---|---|---|
| 19, 90–98, 506 | **LOW** | Header still cites prereg **rev 4**; clearance narrative still “round 3.” Instrument has moved on (rev 10 / later rounds). Stale provenance for the gate that clears data-taking. |
| 124 vs 140–151 | **LOW** | `RUNS` is `${RUNS:-8}` and **not** in the “refuse if present in env” pin list; only `preflight` requires `RUNS==8`. Weaker than other registered constants (works, but inconsistent pin style). |
| 886–891 | *(OK)* | Pair validity is applied to **both** rows together → harness itself cannot emit unpaired `valid=yes` arms (engine arm-count is defense-in-depth for CSV integrity / future harness edits). |
| 846–921, 707–746, 241–272 | *(OK)* | Slot-major + rotation, measured settle, single-process timer, fail-closed probes look aligned with the prereg. Selftest correctly **does not** claim ABBA/daemon coverage. |

No harness defect found that, with the current engine, would by itself mint a confident wrong **session** verdict on honest gates; residual risk is still “instrument complexity,” not a new smoking gun in this read.

---

## Answers (short)

1. **Exclusive & exhaustive** — yes (zeros degeneracy refused).  
2. **B** — hardens both sides; can exceed T (null dead-zone); not zero when controls show residual range.  
3. **CI vs RANGE** — coincide at n=8; EFFECT not weaker there; n>8 path refused.  
4. **n=8** — enforced where it matters; arm-count is the soft underbelly of the *guard*, not the check.  
5. **Code vs prereg** — body≈code; summary paragraph and engine docstring lag.  
6. **Guard** — 14 mutations real; 2 direct CI/RANGE guards justified; **arm-count fix has no mutation/case**.

---

**VERDICT: NOT READY**

Core rule at n=8 looks sound against confident false EFFECT/NONE given clean controls, but the arm-count completeness fix (prior BLOCKER class) can be deleted with a green guard suite and then grade a skewed CSV as **`REPRODUCES`**, and the registered summary text still describes the wrong null statistic.
