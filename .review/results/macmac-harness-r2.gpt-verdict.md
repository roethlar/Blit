# macmac-harness — adjudication of the codex review (round 2)

**Slice**: `24660ae` — the reworked Mac↔Mac instrument (harness + verdict engine
+ guard test) and pre-registration rev 3.
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = ultra`.
**Raw review**: `.review/results/macmac-harness-r2.codex.md`
**Verdict**: **NOT READY — DO NOT RUN THE RIG.** 5 BLOCKER, 6 HIGH, 1 MEDIUM.
**Adjudication: 12 findings, 12 ACCEPTED, 0 rejected.**

**⛔ `scripts/bench_otp12pf_mac.sh` MUST NOT BE RUN in its current state.** The
round-1 fixes were real but incomplete, and the rework introduced a **new,
catastrophic** defect. No data has been taken.

---

## BLOCKER — the transfer timer measures ~1 ms for a 1000 ms transfer → **ACCEPTED (verified in 30 seconds)**

The worst defect found in this project's harness history, and I introduced it in
the rework. The timer captured `time.monotonic()` in **two separate `python3`
processes**:

```
t0=$(python3 -c 'import time;print(int(time.monotonic()*1000))')
<blit copy runs here>
t1=$(python3 -c 'import time;print(int(time.monotonic()*1000))')
```

**On macOS Python 3.9, `time.monotonic()` is PROCESS-RELATIVE** — it returns time
since *that process* started. Verified directly:

    nagatha: t0=4  t1=5  -> a 1000 ms sleep MEASURED AS 1 ms
    q:                   -> the same sleep measured as 0 ms
    single-process timer -> 1009 ms (correct)

**Consequence**: every `ms` row would have been ≈ `fsync_ms` alone. The
invariance ratio — the entire measurand — would have been computed on **fsync
noise**, which can manufacture or mask a one-directional effect at will. A clean
run with 0 voided pairs would have produced a confident, meaningless verdict.

The other harnesses do this correctly (`bench_otp12pf_linux.sh` brackets with
`/proc/uptime`, which is system-wide; `bench_otp12_win.sh` uses a single
PowerShell `Stopwatch`). I broke it by shelling out twice.

**Fix**: time the transfer inside **one** process that also spawns the client
(`python3 -c "t=monotonic(); rc=subprocess.call([...]); print(...)"`), so the
interval is measured by a single clock, and python's startup cost is outside it.

## BLOCKER — BLOCKER 1 is NOT fixed: the rule still is not enforced end-to-end → **ACCEPTED**

`CELLS` may omit controls or a measurand; absent cells are *filtered* rather than
marked `INCOMPLETE`, so **a one-cell run can emit `VANISHES` while claiming "both"
cells vanished**. A control with bar `FAIL` but outcome `INCONCLUSIVE` escapes
RIG-VOID, and `UNSTABLE` controls are ignored entirely.

**Fix**: require the full registered cell set to be present and complete; RIG-VOID
on **any** control that is not a clean PASS (including UNSTABLE/INCONCLUSIVE).

## BLOCKER — BLOCKER 2 REMAINS: a rig-W-sized effect still reports VANISHES → **ACCEPTED**

`DELTA_REF_MS = 230` **never participates in a decision**. Codex's new
counterexample: both measurands with `srcinit = 2500` and **all eight `d_i = 230`**
→ ratio 1.092 (under the bar), CI `[230, 230]`, margin `0.10 × 2500 = 250` → the
CI lies inside ±250 → **`VANISHES`** — with a rig-W-sized effect present *in every
single pair*.

The margin is tied to the **bar**, and on a slow fast-arm the bar is *wider* than
the reference effect. My "equivalence" test therefore excludes only what the bar
already tolerates.

**Fix**: the equivalence margin must be **`min(bar_breach, Δ_ref)`** — a null
requires excluding an effect the size of the *reference effect*, not merely one
the bar would forgive. If `Δ_ref` cannot be excluded → **UNDERPOWERED**.

## BLOCKER — preflight cannot succeed as written → **ACCEPTED**

`grep -c` exits **1** when there are no matches, so for a *clean* binary the
dirty-marker probe `grep -c ... || echo X` emits `0\nX`, which fails the numeric
check and dies. **The harness would refuse to start on a correct binary.**
Separately, `norm_mac` uses `strtonum()`, which is **gawk-only** — stock macOS
`/usr/bin/awk` errors with "undefined function strtonum".

(Fail-closed, so it could not have corrupted data — but it means the round-1
"fixes" were never actually executed end-to-end. I ran `bash -n`, not the gates.)

## BLOCKER — BLOCKER 3 only partly fixed → **ACCEPTED**

The null and the generated verdict text are now pair-scoped, but "not platform
residue" and "every code-level hypothesis strengthens" still **exclude live
macOS/APFS and host×role explanations**. Worse, the design implies this run
decides whether P1 *may be accepted as residue* — which **contradicts the parent
plan's explicit no-escape rule for P1**.

**Fix**: state that a reproduction leaves macOS/APFS and host×role explanations
open, and delete any implication that this run bears on an escape hatch that does
not exist.

## HIGH — the bootstrap CI is not a 95% CI at n=8, and the sign test is computed but never used → **ACCEPTED**

At n=8 the bootstrap median CI resolves to ≈`[d₂, d₇]`, whose exact
population-median coverage is **92.97%**, not 95%; the 10k seeded resamples add no
information (and the lower percentile uses index 250 rather than nearest-rank
249). The exact sign-test arithmetic is **correct**, but **no verdict reads it** —
so 7/8 positives can produce `REPRODUCES` while the registered two-sided sign test
says `p = .0703`.

**Fix**: use the exact distribution-free order-statistic interval and state its
true coverage, and make the sign test **participate** in the decision.

## HIGH — the equivalence interval is wrong for a symmetric ratio bar → **ACCEPTED**

The bar is symmetric in *ratio*, so the negative boundary is `−src/11` (≈ −9.09%),
not `−0.10 × src`. With `src = 2000` and CI `[−190, 0]` the engine says `VANISHES`
although −190 implies an **inversion ratio of 1.105** — over the bar. Also
`powered` tests CI *width* rather than *exclusion of the margins*, so it can record
`powered_for_null = no` alongside a `VANISHES` verdict.

## HIGH — `SETTLE_MS` does not remove the asymmetric gap → **ACCEPTED**

It adds 250 ms *after* the remote arm has already received extra free writeback
during the ssh return. The difference still reverses by direction; the code merely
**assumes** 250 ms saturates it. My measurement showed fsync is insensitive to a
10–200 ms delay, which bounds the effect — but "bounded by a measurement" is not
"removed by construction", and the doc claims the latter.

## HIGH — several gates still fail open → **ACCEPTED**

`pgrep` errors remain indistinguishable from "quiet"; a failed `top` becomes 0%
**and its last idle sample can overwrite an earlier busy one**; non-numeric
`iostat` becomes zero **and can certify drainage**. Both child bash processes run
with `pipefail` **off**. (Time Machine, numeric start-load and purge failures *do*
now fail closed.)

## HIGH — stale-daemon / teardown still fail open → **ACCEPTED**

A stale-daemon `pgrep` error passes; startup captures the *first* daemon PID; the
smoke proves only that *some* blit daemon answers. Teardown still calls a failed
ssh/`ps` probe "GONE", and cleanup discards a positively detected survivor.

## HIGH — provenance remains fail-open → **ACCEPTED**

`die` inside `$(sha256_of ...)` exits only the **subshell**; the outer `echo`
succeeds with an empty hash. The harness hash is recorded but never **compared**
to reviewed content; dirty worktrees are accepted; `EXPECT_SHA` is not pinned to
`f35702a`; and the separately-executable verdict engine is not hashed at all.

## MEDIUM — logging/lifecycle nits → **ACCEPTED**

End-load is only logged *after* verdict computation, and the landed-path/daemon
lifecycle logging is thin.

## HIGH — the guard test → **ACCEPTED (partially reassuring)**

Codex confirms the guard is **non-vacuous for the exact old range defect** and
would catch a bounds swap — but it does **not** cover the new counterexample
(`d_i = 230`, `src = 2500`) or the sign-test omission.

**Fix**: extend the guard with codex's new counterexample and a case that pins the
sign test.

---

## Assessment

Three rounds in, the instrument is still not fit to run, and the most dangerous
defect (**the timer**) was introduced by *my own rework*, not inherited. The
pattern across this session is consistent and worth naming: **I am reliably good
at the physics and reliably sloppy at the plumbing**, and every claim I have made
before review has been at least one step broader than the evidence.

The correct next step is **not** another same-session patch. It is a fresh pass
with the full finding list, then a **round-3 review**, and only then rig time.
`scripts/bench_otp12pf_mac.sh` carries a DO-NOT-RUN banner until that lands.
