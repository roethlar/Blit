I'll review the instrument for confident-wrong-answer risks: start with `git show 1e03063`, read the four HEAD files, then drive the verdict engine and gates with synthetic cases.Shell was blocked by policy; I'll use the ptk MCP path and load the harness next.Driving the engine with synthetic sessions and probing harness gates for fail-open / vacuous paths.Checking one more Class-2 pattern: whether `link_gate`/`timer_gate` failures without `die_blind` mis-score under `gate_probe`, and whether partial `REGISTERED_CELLS` is test-covered.# Round 7 review — Mac↔Mac instrument @ `b3d42b7` (instrument @ `1e03063`)

Guard suite: **27/27 cases, 18/18 mutations, 300/300 fuzz** — all green. That does not clear the class holes below.

---

## Findings

### F1 — BLOCKER (Class 1: next branch of “must have controls”)
**`scripts/otp12pf_mac_verdict.py:131-136, 409, 484-531`**

R6 refused an *empty* `CONTROL_CELLS` string. It still accepts a *non-empty name list* with **no control rows/meta**. Then:

```python
ctrl = [c for c in CONTROL_CELLS if c in cell_outcome]  # → []
```

Empty `ctrl` ⇒ no void, no uncertified ⇒ full measurand verdict.

| Attack | Result |
|---|---|
| Controls named, absent from CSV; null measurand | **`VANISHES`** |
| Same; material REPRO measurand | **`REPRODUCES`** |
| `CONTROL_CELLS=measurands` (self-control) | **`VANISHES`** |
| Truly empty `CONTROL_CELLS` | refuses (rc=2) — only this branch is closed |

Same defect class as r6’s footgun: the precondition checks **labels**, not **graded controls**. No case/mutation covers presence.

Harness path hardcodes full cells, so a normal timed run is safer — but the **mechanized rule** (separately executable, hashed, re-gradeable) still issues a confident answer with zero control evidence.

---

### F2 — BLOCKER (Class 2: protection that never actually binds)
**`scripts/bench_otp12pf_mac.sh:498-529, 1018-1025`**

Escalation checks only:
- four filenames exist  
- first line text `INCONCLUSIVE-UNDERPOWERED`  
- manifest contains `binary_identity=f35702a`  
- `runs.csv` sha not already burned  

It does **not** re-grade the prior CSV, require prior `RUNS=8`, or bind verdict text to data.

**Reproduced:** a temp dir with those four files and a one-line forged verdict would authorize `RUNS=16` (`ESCALATION_FORGE_WOULD_PASS`). A **new** forged `runs.csv` ⇒ new hash ⇒ **unlimited** “once” escalations.

This is the r5/r6 “flag is not a justification” fix applied to **filename theater**, not evidence.

---

### F3 — HIGH (Class 1: half-margin leaves a permanent dead zone)
**`scripts/otp12pf_mac_verdict.py:473-481, 516-531`**

| Control | n | Verdict |
|---|---|---|
| D=+120 @ src=2500 (below full margin 230, above half 115) | 8 | `CONTROLS-UNCERTIFIED` |
| Same | 16 | `CONTROLS-UNCERTIFIED` (escalation cannot fix a **consistent** effect) |
| D=+114 (half−1) | 8 | **`VANISHES`** (certifies “clean”) |
| D=+114 + material measurand | 8 | **`REPRODUCES`** |
| D=+230 | 8 | `RIG-VOID` (OK) |

So:
- A **good** rig with modest host×role slightly over half-margin can **never** produce null *or* reproduction (instrument useless — the failure mode you named).  
- A control carrying **~half of Δ_ref** still certifies; session prose can claim clean controls while gRPC/large carry ~114 ms of the ~230 ms reference effect (specificity claim is still soft).

---

### F4 — HIGH (Class 2: settle is harness-only; engine is blind to it)
**`scripts/bench_otp12pf_mac.sh:725-771` vs `otp12pf_mac_verdict.py` (no `settled_ms`)**

Settle is measured in-process and voids in the harness. The engine **never reads** `settled_ms` / does not re-check the void.

**Reproduced:** every row `settled_ms=0`, `valid=yes` → still **`VANISHES`**.

Any re-grade, CSV edit, or future harness path that sets `valid=yes` without `settle_ok` **revives the free-writeback manufacturing path in the decision rule**. Same class as “fix asserted in the log, not enforced where the answer is computed.”

---

### F5 — HIGH (spec/code conflict on the registered rule)
**`docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:6-15` vs `:247-261` vs `otp12pf_mac_verdict.py:332-337`**

- One-paragraph + engine: bar **out of inference**; `material = ci_lo >= breach_hi` only.  
- Decision-rule table still: `material = bar_fail_pos or CI_lo >= BREACH_HI`.

Verified in code: r6 bar-substitute attack (`+1 ms` CI, marginal bar FAIL) → **`PARTIAL`**, not `REPRODUCES`.  
But the pre-registration **disagrees with itself**. A human “walking the numbers” from the table can re-apply the rule that rounds 5–6 already showed is false.

---

### F6 — MEDIUM (Class 2: SELFTEST blindness scoring)
**`scripts/bench_otp12pf_mac.sh:893-908` + gates that `die`/exit without `PROBE-BLIND`**

`gate_probe` treats any non-`PROBE-BLIND` failure as **`[FIRED]`**.

**Reproduced:** silent `false` → **`[FIRED]`**, not `[BROKEN]`.

A hard `hrun`/ssh failure inside `link_gate` / `timer_gate` (assignment fails under `set -e` before `die_blind`) can score **FIRED** while the probe never answered. SELFTEST can still exit 0 if `SELFTEST_BROKEN==0`. Taxonomy is only honest for the explicit `die_blind` sites.

---

### F7 — MEDIUM (guard gaps)
**`scripts/otp12pf_mac_verdict_test.py`**

| Fix | Mutation? |
|---|---|
| Controls non-empty env | no case for **absent** controls (F1) |
| Half-margin cert | yes (`D=+229`) |
| Uncertified blocks REPRO | yes |
| Bar out of material | yes |
| DELTA_REF pin | yes (killed via case fail → `CONTROLS-UNCERTIFIED`, not via `VANISHES`) |
| Escalation / settle / SELFTEST | **not in engine suite** |

Mutations that judge “case fails” are real for their targets; they do not cover F1/F2/F4.

---

### F8 — LOW
**Prereg `:315-318` vs code `:453-457`:** prereg says bar FAIL ⇒ contaminating/`RIG-VOID`; code uses bar FAIL only to refuse certification → `CONTROLS-UNCERTIFIED`. Still blocks a measurand claim; label/precedence drift only.

---

## What looks fixed (spot-checked)

| Claim | Status |
|---|---|
| Bar out of REPRODUCES/VANISHES inference | **Holds** in engine (`material` is CI-only); r6 attack → PARTIAL |
| Direction = sign; magnitude = CI; equivalence = CI vs margin | **Holds** in cell taxonomy |
| Dirty control D=+230 | **RIG-VOID** (not null) |
| Uncertified blocks REPRO | **CONTROLS-UNCERTIFIED** |
| Empty `CONTROL_CELLS` | refuses |
| `DELTA_REF_MS` mismatch | refuses; matching `230` allowed |
| Settle in-process + measured | **Works** in isolation (`settled_ms≈255`); voided if not elapsed **in harness** |
| `die_blind` marker | Present at probe-failure sites listed |
| 27/18 guard | Passes; not sufficient for F1/F2/F4 |

---

## Reproductions (commands + outcomes)

```text
# F1 — named controls, no rows → confident null / repro
→ SESSION VERDICT: VANISHES
→ SESSION VERDICT: REPRODUCES

# F3 — half-margin trap / soft cert
D=+120 @src=2500 n=8|16 → CONTROLS-UNCERTIFIED
D=+114               → VANISHES
D=+114 + repro meas  → REPRODUCES

# F4 — settle column ignored by engine
settled_ms=0 on all rows, valid=yes → VANISHES

# F2 — escalation forge (gate predicates only)
four files + "SESSION VERDICT: INCONCLUSIVE-UNDERPOWERED"
+ binary_identity=f35702a → ESCALATION_FORGE_WOULD_PASS

# Guard suite
27/27 cases, 18/18 mutations, 300/300 fuzz, exit 0
```

---

## VERDICT: **NOT READY TO RUN**

Two class recurrences are live:

1. **Class 1:** control precondition closed for “empty env list,” still open for “names without graded controls” (and half-margin still has a permanent non-concluding band plus soft cert at half−ε).  
2. **Class 2:** escalation is still assertion-shaped; settle is not enforced in the engine that emits the answer; SELFTEST can still call a mute failure `[FIRED]`.

Do not take data until at least **F1** and **F2** are closed (and preferably **F3**/**F4** so the instrument can both conclude and keep settle load-bearing at the verdict boundary).
