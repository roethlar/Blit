# pf-0-prereg — adjudication of codex ROUND 2 (of `7921adc`)

reviewer: **gpt-5.6-sol**, effort **ultra** (codex exec, read-only; model read
from `~/.codex/config.toml`, not assumed)
codex verdict: **NOT READY** — 5 BLOCKER, 3 HIGH
adjudicated by: Claude (author)
outcome: **8 raised → 8 Accepted, 0 Rejected**

Round 1 found 7 defects; I fixed them and round 2 found 8 more, five of them
blocking. That is not review fatigue — the fixes were genuinely inadequate, and
they were inadequate for **one root reason** that I had already learned the
same day on a different rig and failed to carry across.

## THE ROOT DEFECT (it explains F1, F3, F4 and F6 at once)

**Every threshold in revision 2 was invented, because the design had no noise
model.** `RUNS=8` estimates variance *within* a session; the entire MTU
comparison is *between* sessions. And MTU was perfectly aliased with session
order (always 9000 first, 1500 second).

Codex's counterexample is decisive and I could not refute it: from the recorded
1500 medians `(win, mac) = (939, 1221)`, a shared **985 ms floor** at 9000
yields ratio **1.000**, `r = 100%`, a fast-arm regression of only **4.9%**
(inside my invented 5% tolerance), and "both arms slower" is false. **A pure
masking artifact passes every guard I wrote** — including the guard I had
already tightened once in response to round 1.

The galling part: **this session had already proved the same lesson on the
local Windows rig hours earlier** — that blit's absolute times are bi-stable
across sessions (1388 ms vs 2225 ms, identical binary) and that only
*within-session* interleaved comparisons are trustworthy. I wrote that up, and
then designed a rig-W experiment whose central comparison spans two sessions.

**Fix (revision 3)**: a counterbalanced **A-B-B-A** design (9000, 1500, 1500,
9000). MTU is no longer aliased with order, and the **same-MTU replicate pairs
supply a measured noise floor `N`** for every quantity the rule uses. Every
threshold is now expressed against `N`. The 46 ms fast-arm regression in
codex's counterexample either exceeds this rig's measured session noise or it
does not — that is an empirical question, and now it gets an empirical answer.

## F1 (BLOCKER) — "MTU confounded with session order; RUNS=8 is not session variance" — **ACCEPTED**
Correct, and central. Fixed by A-B-B-A + same-MTU replicates (above).

## F2 (BLOCKER) — "the recovery ratio has no valid domain" — **ACCEPTED**
Correct. `r = (Δ_1500 − Δ_9000)/Δ_1500` is undefined/unstable when `Δ_1500` is
≈0 or noisy, and a **negative `Δ_9000`** yields `r > 100%` even though it
represents a *new* invariance failure in the opposite direction. The parent says
plainly that `Δ ≈ 0` proves nothing (`OTP12_PERF_FINDINGS.md:498`).
**Fix**: a domain guard evaluated FIRST (`Δ_1500 ≤ N_Δ` → **INCONCLUSIVE**, a
registered outcome), plus explicit **INVERSION** and overshoot rules.

## F3 (HIGH) — "RUNS=8 does not resolve what the rule hinges on; no variance model" — **ACCEPTED**
Correct; same root cause. The noise floor is now measured, not assumed.

## F4 (BLOCKER) — "the masking guards still admit the artifact" — **ACCEPTED**
Correct — see the counterexample above. **Fix**: the fast-arm guard is now
`N_arm`-based rather than a made-up 5%; the slow arm must converge to the fast
arm's **1500 value** (not a shared floor); and a both-arms-slower result is
reported as **degradation**, never as a pass.

## F5 (BLOCKER) — "the fill/null downgrade contradicts itself" — **ACCEPTED**
Correct and embarrassing: I wrote that segment fill is unmeasured, then
concluded a null would prove per-packet cost irrelevant — an inference that
*requires* the fill measurement I had just said I lacked. I also let byte/MSS
quotients (upper bounds under full fill) drift into prose about segments
"falling".
**Fix**: the only supported null conclusion is now stated verbatim — *"raising
the MTU did not improve these cells under the observed packetization"* — and the
segment counts are explicitly labelled upper bounds assuming full fill.

## F6 (BLOCKER) — "the ≥5% positive-control falsifier is unsound" — **ACCEPTED**
Correct, and it would have killed a true result. `large` is **throughput-bound**
while `mixed` is packet-sensitive, so `large` need not move even when jumbo
genuinely helps `mixed`. Codex's case: `(939,1221) → (939,1000)` = `r = 78.4%`,
invariance 1.065, `large` unchanged — my falsifier would have voided it. The 5%
threshold also had no noise basis.
**Fix**: `wm_tcp_large` is **withdrawn as a gate** and demoted to **reported
corroborating context**. It can support a mechanism; it cannot void a result.

## F7 (HIGH) — "MSS is proven only at session start" — **ACCEPTED**
Correct. A `getsockopt` sample proves one socket at one instant; the harness
opens its own transfer connections later and records neither MTU nor MSS.
**Fix**: MSS is recorded at session **start AND end**, and a session whose MSS
is not the expected value at both points is **VOID**. This still does not prove
every connection individually (that needs a harness change) — the residual is
stated in the doc rather than hidden.

## F8 (HIGH) — "the CELLS subset silently removes verdict evidence" — **ACCEPTED**
Correct, and I had independently found the same thing while re-reading
`compute_verdicts`. The four chosen cells have no block-1 counterparts, so eight
`NO-SAME-SESSION-REF` rows are emitted (`bench_otp12_win.sh:715`) and **no**
discriminator-gap rows can emit (`:743` requires all four contributing cells).
**Fix**: **declared explicitly** in the pre-registration rather than discovered
in the output. It is acceptable here because the measurand is the **invariance**
row, computed entirely within one session; acceptance evidence is `pf-final`'s
job, not this experiment's.

## F9 (HIGH) — "the rebaseline consequence is incomplete" — **ACCEPTED**
Correct. Revision 2 said only **P2** was blocked by the stale (MTU 1500)
committed baseline. In fact **P1's `pull_tcp_mixed` bar and the parent's global
rule also consume committed references** (`OTP12_PERF_FINDINGS.md:541`, `:553`).
So at fleet jumbo, **formal acceptance of P1 and of the global rule — not just
P2 — needs a re-recorded baseline** plus a fixed-reference harness change.
**Fix**: stated in the doc as a plan amendment that goes through the loop.

## Codex confirmed FIXED from round 1

- **F7 (round 1) — the committed-baseline void inventory is now complete.**
  "Invariance and actually emitted `old_session` rows are MTU-matched,
  conditional on MTU remaining stable."
- The `CELLS` names are all accepted by the harness allowlist (executability
  confirmed).
- The segment arithmetic is confirmed: ~741 535 (`large`) vs ~377 840 (`mixed`)
  at MSS 1448 — so the round-1 F6 factual correction stands.

## Cost of the redesign

A-B-B-A at RUNS=8 over 4 cells = 4 sessions × 64 timed runs ≈ **90 minutes** of
rig time, versus ~60 for the unsound 2-session design. That is the price of an
experiment whose central comparison is not confounded with the order it ran in.

fix sha: recorded on landing (this verdict is committed WITH revision 3).
