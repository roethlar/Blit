# MTU IS NOT THE CAUSE OF P1 — A-B-B-A jumbo experiment, rig `q` ↔ netwatch-01 (2026-07-14)

**Status**: Evidence (recorded). This README applies the **pre-registered**
decision rule in `PREREGISTRATION.md` to the data, and states nothing the rule
does not license. **Provenance of the rule, stated precisely**: the decision
rule, thresholds and guards were fixed in **rev 3**, before any of the S1–S4
data existed, and rev 4 left them untouched (it re-described the *rig* after the
`q` baseline). So no threshold was authored around these numbers — but "the
document was written before any data existed" would be false, since a `q`
baseline and a discarded A-B-B-A attempt preceded rev 4. It is
**not** a plan amendment: per the pre-registration, a result here "licenses
evidence for a plan amendment only" — killing the MTU hypothesis in
`docs/plan/OTP12_PERF_FINDINGS.md` is a separate, reviewed change.

**Design executed as registered**: four sessions **A-B-B-A** = 9000, 1500,
1500, 9000, `RUNS=8`, `CELLS=wm_tcp_mixed,wm_tcp_large,mw_tcp_mixed,wm_grpc_mixed`,
sha `f35702a` both ends, old arm `0f922de`, Mac end `q` (10.1.10.54, `en8`).
**256 timed runs, 0 voided.**

## Result — `r = −3.1%` → **KILLED as a material cause**

| session | MTU | mac_init | win_init | **Δ** | ratio | invariance |
|---|---:|---:|---:|---:|---:|---|
| S1 | 9000 | 1035 | 760 | **275** | 1.362 | FAIL |
| S2 | 1500 | 1071 | 830 | **241** | 1.290 | FAIL |
| S3 | 1500 | 1066 | 849 | **217** | 1.256 | FAIL |
| S4 | 9000 | 1029 | 832 | **197** | 1.237 | FAIL |

    Δ_9000 = mean(275, 197) = 236 ms
    Δ_1500 = mean(241, 217) = 229 ms
    N_Δ    = max(|275−197|, |241−217|) = max(78, 24) = 78 ms   [measured noise floor]

**Domain guard (evaluated first)**: `Δ_1500 (229) > N_Δ (78)` — the gap under
study is present above this rig's own session-to-session noise, so the
experiment is **in domain** and the recovery is computed.

    r = (Δ_1500 − Δ_9000) / Δ_1500 = (229 − 236) / 229 = −3.1%

On the parent plan's uniform pre-registered scale (`r < 20%`), that is
**KILLED as a material cause**. Raising the MTU did not recover *any* of the
gap; the point estimate is slightly negative (the gap was nominally *wider* at
jumbo), but **|Δ_9000 − Δ_1500| = 7 ms is far inside the measured noise floor
of 78 ms** — so the honest statement is not "jumbo made it worse" but **"the
two conditions are indistinguishable: MTU has no measurable effect on Δ."**

**Registered edge cases**: no INVERSION (`Δ_9000 = 236 > 0`); `r` not >100%;
and `Δ_9000 (236) > N_Δ (78)`, so the residual gap is **not** inside the noise
— P1 survives jumbo as a real, measurable asymmetry.

**P1 fails in all four sessions** (1.237–1.362) regardless of MTU, by the
harness's exact integer arithmetic (`10·hi ≤ 11·lo`), not the printed ratio.

## ⚠ The resolution limit — this run cannot exclude a *contributing*-size effect

The registered rule grades the **point estimate**, and the point estimate is ~0.
But the experiment's own noise floor bounds what it could have seen:

| effect size | in ms (of Δ_1500 = 229) | vs floor N_Δ = 78 ms | can this run exclude it? |
|---|---:|---|---|
| DOMINANT (`r ≥ 50%`) | ≥ 114 ms | comfortably above | **yes** |
| CONTRIBUTING (`r ≥ 20%`) | ≥ 46 ms | **below the floor** | **NO** |

So the honest scope of this null is: **jumbo is not a dominant cause of P1, and
its measured contribution is indistinguishable from zero — but a
contributing-size (~46 ms) MTU effect could be swamped by this rig's
session-to-session noise and would not have been detected.** The KILLED grade
stands as the pre-registered rule returns it; it must not be re-read as a
stronger exclusion than that. (Pre-registration §"the noise model" fixed the
floor as *measured*, not assumed — this is the price of that honesty, and it is
stated rather than hidden.)

## Where the noise actually comes from: the fast arm is BISTABLE

The 78 ms floor is not diffuse jitter. The `win_init` runs are **bimodal** —
one cluster near ~730 ms and one near ~840 ms — and the two same-MTU replicates
simply drew different **mixtures** of the two modes:

    S1 (9000) win_init: 699 715 750 753 767 776 | 843 844      -> 6 low, 2 high, median 760
    S4 (9000) win_init: 752 755 | 825 828 836 837 838 860      -> 2 low, 6 high, median 832

Same MTU, same build, same rig: the 72 ms gap between those medians is a
**mode-mixture artifact**, and it is what sets N_Δ. The `mac_init` arm shows
nothing of the kind (replicate medians differ by **5 and 6 ms**). This matches
the local-rig bi-stability already recorded in
`docs/bench/win-local-ab-2026-07-13/`.

**Consequence for pf-1 (a trap):** a counterfactual that merely shifts the mode
mixture would look exactly like a partial recovery. Grade on the run
distribution, not the median alone.

**The MTU verdict is robust to it.** Pooling all 16 runs per condition (instead
of averaging session medians) gives `Δ_9000 = 232`, `Δ_1500 = 221.5`,
**`r = −4.7%`** — the same KILLED grade.

## The manipulation demonstrably reached the wire (the null is not vacuous)

The most important defense of a null result is proof that the treatment was
actually applied. Three independent instruments say it was:

- **MSS gate, start AND end of every session** (the rev-4 requirement):
  **8948/8948** in both jumbo sessions, **1448/1448** in both 1500 sessions.
  No session is VOID on this gate.
- **`wm_tcp_large` (registered as CONTEXT, never a gate)** got **3–4% faster at
  jumbo on both arms** (mac_init 960→924 ms, win_init 945→916 ms). Jumbo does
  real work on this path — it just does not touch the asymmetry.
- **Both arms of `wm_tcp_mixed` also sped up slightly at jumbo** (mac 1068→1032,
  win 840→796) while Δ stayed put. The benefit is **symmetric**, which is
  precisely why it cannot explain an **asymmetry**.

## Masking guard — the ratio did not improve, and no artifact is hiding a fix

Rebuilt on the measured noise (`N_arm = 72 ms`, the largest same-MTU replicate
difference across both arms). **Disclosure**: the pre-registration did not say
how the two replicate medians become one condition-level value per arm; this
analysis uses their **mean**. Every plausible alternative (either replicate
alone, or the pooled runs) gives the same guard outcome, but "exactly as
pre-registered" would overstate the spec's precision, so the choice is named
here rather than left implicit.

- **Fast-arm guard**: `win_init` at 9000 did **not** regress (−43.5 ms, i.e.
  faster). OK.
- **Convergence target**: `mac_9000 (1032) ≤ win_1500 (839.5) + N_arm (72) = 911.5`
  → **NOT MET**. The slow arm did not approach the fast arm.
- **Both-arms-slower (bottleneck compression)**: **False**.

So there is no shared-floor artifact and no compression — there is simply **no
fix**.

## Controls (all four sessions, both conditions)

| cell | S1 (9000) | S2 (1500) | S3 (1500) | S4 (9000) |
|---|---|---|---|---|
| `mw_tcp_mixed` (opposite direction) | 1.042 P | 0.979 P | 1.072 P | 1.021 P |
| `wm_grpc_mixed` (opposite carrier) | 0.994 P | 1.022 P | 1.016 P | 1.020 P |
| `wm_tcp_large` (opposite fixture) | 1.000 P | 1.015 P | 1.017 P | 1.017 P |

P1's signature is unchanged by MTU: **TCP only, `mixed` only,
destination-initiator only.**

## What this does NOT establish (carried from the pre-registration)

- **Segment fill is unmeasured.** 8948 is the MSS *ceiling*, not the *fill*.
  The only conclusion supported is: *"raising the MTU did not improve these
  cells under the observed packetization."* It does **not** prove per-packet
  cost is irrelevant to blit in general. (The `wm_tcp_large` speedup shows
  packetization matters *somewhere* — just not for Δ.)
- **The MSS gate is start-and-end, not per-connection.** A mid-session change
  that reverted before the end would go undetected.
- **Verdict rows VOID at jumbo**: every `converge … old_committed`,
  `cross … min_old_committed`, and block-1 `combined` row is graded against the
  MTU-1500 `otp2w-baseline-2026-07-10` reference and is **VOID in the 9000
  sessions**. None of the conclusions above use them. The **invariance** rows —
  the measurand — are new-vs-new within one session and are MTU-matched by
  construction.
- The `NO-SAME-SESSION-REF` / absent discriminator-gap rows are the **declared
  omission** (rev-4 F8), expected because these four cells have no block-1
  counterparts in `CELLS`.

## Rig log (recorded so it is not rediscovered)

- **Time Machine was disabled on `q` for the window** (owner-executed; autobackup
  had fired at 23:54 and macOS repeats hourly, which would have landed inside
  the ~70-minute run, and one of its destinations is a network share on
  `skippy` — i.e. the same 10 GbE fabric). **The harness's quiet-gate does not
  catch this**: it refuses to start on `codex`/`cargo`/`rustc` only.
- **`en8` was physically flapping before the run** and the owner reseated the
  connection. Three harness starts died at the old-pair smoke with a gRPC
  `transport error` while it was unstable; the daemon, the binaries, the
  firewall, the MTU-set and the daemon-start timing were each individually
  cleared (the daemon binds in 169–665 ms; a hand-run smoke succeeded
  repeatedly). After the reseat, a 5 × 1 GiB link test ran at **891–897 ms**
  (≈1.2 GB/s, 0 errors) and all four sessions then completed with 0 voided runs.
- A `bash -x` diagnostic run at MTU 9000 was **discarded, not banked**: it
  differed from its own replicate (tracing to disk on the bench Mac), and the
  design requires the four sessions be identical.
- `load1` on `q` sat at 1.5–2.3 through the sessions (macOS idle baseline on this
  box; instantaneous CPU was <3%).

## Files

`S1_9000/`, `S2_1500/`, `S3_1500/`, `S4_9000/` — each with `runs.csv` (64 timed
runs), `summary.csv`, `verdicts.csv`, `meta.csv`, `bench.log`, `session.log`,
`staging-manifest.txt`, and the session's `mss-start.txt` / `mss-end.txt` +
`load-start.txt` / `load-end.txt`.
