I'll review the instrument for confident-wrong-answer risk: pull the commit, read the four files at HEAD, then hunt both defect classes with shell where possible.I'll run the guard suite and drive the engine with synthetic sessions to try forcing a wrong verdict, while probing gates for fail-open paths.Hunting edge cases next: control-rule gaps, selftest misclassification, escalation forgery, and dead settle proof.Probing bash gates, selftest classification, escalation, and other dead paths with real execution:# Mac↔Mac instrument review (HEAD `aebd50b`)

Guard suite: **22/22 cases, 15/15 mutations, 300/300 fuzz** — all green. That does not mean the instrument cannot emit a confident wrong answer.

---

## Findings

### F1 — BLOCKER (Class 1): open-interval margin certifies ~Δ_ref control effects → `VANISHES`
**Where:** `otp12pf_mac_verdict.py:303,416-435,520-544`  
**What:** Contaminating uses `ci_lo >= margin_hi`; certification uses `ci_hi < margin_hi`. At `D=+229` (Δ_ref−1) on a slow arm, controls are PARTIAL, **null_excl=yes**, **certified**, session **`VANISHES`**, prose says “every control is CERTIFIED clean.” At `D=+230` correctly `RIG-VOID`. Same headline shape as r4/r5 (null while controls carry ~full reference effect), one millisecond under the post-fix boundary.  
**Worse:** at n=16, `control_d=[229]*10+[0]*6` → CI `[0,229]` still null_excl → `VANISHES`.

### F2 — BLOCKER (Class 2): SELFTEST “proves settle” without checking settle ran
**Where:** `bench_otp12pf_mac.sh:869-880`; claim in `PREREGISTRATION.md:18-21,432-433`  
**What:** `selftest_fsync` only checks `files==2` and `bytes==6`. **`ms=0` still scores `[OK]`.** A dead settle (the r5 disaster class) passes the proof that was added specifically to catch it. Log line “settle included” is narrative, not an assert.

### F3 — HIGH (Class 2): blind timer probe scored `[FIRED]`, selftest can exit 0
**Where:** `bench_otp12pf_mac.sh:259,848-860`  
**What:** `timer_gate` die text *“returned nothing — refusing”* does **not** match the BROKEN regex (`cannot (read|…)|BROKE|…|refusing \(a gate`). Blind timer → **`[FIRED]`** → does not increment `SELFTEST_BROKEN` → **SELFTEST can PASS while the measurand clock probe is blind.** Same fail-open the selftest claims to hunt.

### F4 — HIGH (Class 1 / completeness): obligation rewrite still label-adjacent at the margin edge
**Where:** `otp12pf_mac_verdict.py:416-426` vs `429-430`  
**What:** Restructuring is real for *labels* (no more void-if-in-{PARTIAL,UNDERPOWERED,…}). The next hole is the **open vs closed margin cut**: contaminating = “at or beyond”; certified = “strictly inside.” No obligation asks “is the control carrying a near-margin, sign-significant arm asymmetry that makes a null uninterpretable?” — TOST alone answers, and answers `VANISHES` at 229.

### F5 — MEDIUM (Class 2): escalation still operator-forgeable
**Where:** `bench_otp12pf_mac.sh:495-505,956-958`  
**What:** Gate only checks `session_verdict.txt` text + absence of `ESCALATED`. A one-line forged dir, or a **copy** of a prior session without the marker, re-authorizes `RUNS=16`. Burn is path-local, not content-bound. Better than a bare flag; still a re-roll surface.

### F6 — MEDIUM: SSH RTT is measured, never enforced
**Where:** `bench_otp12pf_mac.sh:455-472`  
**What:** Residual free-writeback asymmetry is *bounded by* RTT in the doc; if mux/RTT is large, residual can approach or exceed `SETTLE_MS` with **no refuse**. Only non-numeric RTT dies. Protection is observational, not a gate.

### F7 — MEDIUM: end-load / drain selftest honesty
**Where:** `bench_otp12pf_mac.sh:916-924`  
**What:** Unreadable end-load voids the session in production (good) but selftest always scores it **`[FIRED]`**, never BROKEN. `DRAIN-TIMEOUT` (disk busy) scored **`[BROKEN]`** (wrong class; fail-closed for the sweep). Classification still not trustworthy.

### F8 — MEDIUM: guard mutations do not cover harness Class-2 fixes
**Where:** `otp12pf_mac_verdict_test.py` only  
**What:** 15 mutations are engine-only and mostly faithful (stale detection works; kill = case fails). **No mutation** for settle execution, `pgrep_state`, escalation, selftest taxonomy, SSH RTT, drain rc. Engine green ≠ harness protections execute. Cases without mutations include false-void (“tiny control must NOT void”), INVERSION, codex r1 190ms, clean `VANISHES`.

### F9 — LOW: doc / docstring drift
**Where:** `otp12pf_mac_verdict.py:63-66` still documents `pos_effect = CI_lo > 0 ∧ sign_p`; `PREREGISTRATION.md:216-218` still says **17 cases / 11 mutations** while also mentioning 22/15 elsewhere. Misleading under pressure; not a runtime path.

### F10 — LOW: engine callable without controls → `VANISHES`
**Where:** engine env `CONTROL_CELLS` / `REGISTERED_CELLS`  
**What:** Standalone engine with empty controls emits `VANISHES`. Harness assigns cell sets as literals (not env-defaults) — OK for the intended entrypoint; still a footgun if the engine is invoked alone.

---

## What looks solid (verified)

| Area | Result |
|------|--------|
| Three-question cell taxonomy | Direction = sign test; magnitude = CI/bar-with-direction; equivalence = CI vs margin. r5 cases (inverse bar +1ms, 0+99×7, uncertified controls) held under drive |
| Control obligation (non-boundary) | Contaminating → `RIG-VOID`; uncertified → blocks null only; tiny +5ms → `VANISHES` not void; repro survives uncertified control |
| Pinned constants | Harness refuses if listed vars **present**; engine refuses mismatched `DELTA_REF_MS`; matching `230` also refused by harness (presence) |
| Single `pgrep_state` | Only process probe; quiescence + stale-daemon both use it |
| SETTLE computation | Top-level `SETTLE_SEC=0.250` sleeps ~250ms; old `\"` awk path still fails if revived; sleep failure → `F:NA` → pair void |
| Guard suite | 22/22, 15/15 killed, fuzz clean |

---

## Reproductions

```bash
# F1: Δ_ref-1 certifies; Δ_ref voids
python3 - <<'PY'
import sys; sys.path.insert(0,'scripts')
from otp12pf_mac_verdict_test import session
null=[-4,-2,-1,0,0,1,2,3]
print('229', session(measurand_d=null, src=2000, control_d=[229]*8, control_src=2500))
print('230', session(measurand_d=null, src=2000, control_d=[230]*8, control_src=2500))
print('7x229+0', session(measurand_d=null, src=2000, control_d=[0]+[229]*7, control_src=2500))
print('7x230+0', session(measurand_d=null, src=2000, control_d=[0]+[230]*7, control_src=2500))
print('n16 CI~[0,229]', session(measurand_d=null*2, src=2000,
    control_d=[229]*10+[0]*6, control_src=2500, pairs=16))
PY
# → 229 VANISHES | 230 RIG-VOID | 7x229+0 VANISHES | 7x230+0 INCONCLUSIVE-UNDERPOWERED | n16 VANISHES
```

```bash
# F2: selftest accepts zero-duration walk
# (selftest_fsync condition only)
ms=0 files=2 bytes=6
# → would log [OK] fsync/settle — settle duration never asserted
```

```bash
# F3: blind timer → FIRED not BROKEN
err='nagatha: the timer probe returned nothing — refusing'
grep -qiE 'cannot (read|sample|probe|measure|resolve|answer)|BROKE|did not answer|no sentinel|refusing \(a gate' <<<"$err" \
  && echo BROKEN || echo FIRED
# → FIRED
```

```bash
# F5: escalation content gate accepts forgery
tmp=$(mktemp -d)
echo 'SESSION VERDICT: INCONCLUSIVE-UNDERPOWERED' > "$tmp/session_verdict.txt"
# UNDERPOWERED_ESCALATION=$tmp RUNS=16 … would pass the verdict-text check
```

---

## VERDICT: **NOT READY TO RUN**

Two independent confident-wrong / confident-blind paths remain:

1. **Decision rule:** `VANISHES` + “CERTIFIED clean” with controls at **+229 ms** (and n=16 variants with CI up to 229) — Class 1’s next branch after the obligation rewrite.  
2. **Selftest:** settle “proof” is vacuous and blind timer is mis-scored `[FIRED]` — Class 2 exactly as in rounds that shipped dead protections under green ceremony.

Clear F1–F3 (and preferably F5–F6) before any timed run; do not treat 22/15 green as clearance.
