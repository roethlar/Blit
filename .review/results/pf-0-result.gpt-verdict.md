# pf-0-result — adjudication of the codex review

**Slice**: `63f400e` — plan amendment recording the pf-0 (MTU) experiment's
outcome in `docs/plan/OTP12_PERF_FINDINGS.md`; evidence `363fa6f`
(`docs/bench/otp12-jumbo-win-2026-07-13/`).
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = ultra` (codex-cli;
read from `~/.codex/config.toml`, not assumed).
**Raw review**: `.review/results/pf-0-result.codex.md`
**Verdict**: NOT READY — 2 BLOCKER, 2 HIGH, 1 MEDIUM, 2 LOW, 1 INFO.
**Adjudication: 7 findings, 7 ACCEPTED, 0 rejected.**
**Fix commit**: see `Fix sha` below.

The review independently recomputed every number from the committed
`summary.csv` files and confirmed them (session Δ = 275/241/217/197;
Δ_9000 = 236; Δ_1500 = 229; N_Δ = 78; r = −3.0568%; N_arm = 72; the
`wm_tcp_large` 960→924 / 945→916 improvements). **The arithmetic and the
rule-application were faithful; every accepted finding is about a CLAIM that
outran the data.** That is the correct thing for the loop to catch here.

---

## BLOCKER 1 — "EXCLUDED / environmental escape closed / code, not wire" overreaches → **ACCEPTED**

The registered outcome is `KILLED as a material cause`, and the
pre-registration (`PREREGISTRATION.md:107`) explicitly limits a null to *"raising
the MTU did not improve these cells under the observed packetization"* because
**segment fill is unmeasured**. My text turned that into "the environmental
escape for P1 is closed" and "P1 is a property of the code, not of the wire" —
which excludes *the environment* and *the wire in general* from a single
MTU manipulation. It also silently re-ran the exact error the pre-registration's
own round-2 F5 was written to prevent.

**Fix**: section retitled to "KILLED as a material cause of P1"; added an
explicit "does NOT license" block — MTU is one environmental variable, segment
fill is unmeasured, underfilled segments / another bottleneck / a smaller wire
contribution remain live, and "P1 is code-shaped" is **not** established here.

## BLOCKER 2 — the experiment is not powered for its own CONTRIBUTING boundary → **ACCEPTED**

`20% × 229 = 45.8 ms` is **below** the measured between-session floor
`N_Δ = 78 ms`. The domain guard proves P1 exists above noise; it does **not**
prove a 20%-size MTU effect would have been *detected*. So the run can exclude a
DOMINANT effect (≥114 ms) but **cannot** exclude a contributing-size one. Neither
the README nor the plan admitted this. Sharpest finding of the review.

**Fix**: added a resolution-limit table + statement to the evidence README and
to the plan section. The KILLED grade stands as the pre-registered rule returns
it (the rule grades the point estimate, which is ~0); what changes is that the
grade may no longer be read as a stronger exclusion than the data supports.

## HIGH 1 — the pf-final "VOID / only two ways / blocked" consequence is not mine to assert → **ACCEPTED**

The committed 2026-07-10 baseline is a deliberately **frozen anti-drift ceiling**
(`OTP12_ACCEPTANCE_RUN.md` D2/D5), and D-2026-07-05-4 keeps the pins standing. I
declared jumbo rows VOID, enumerated "only two ways forward", and asserted
pf-final was blocked — none of which the acceptance contract authorizes an
agent to decide.

**Fix**: reframed as an **exposure stated for the owner**, asserting no void rule.
Kept (and sharpened) the substantive point the evidence actually supports: jumbo
made both arms 3–4% faster, so a jumbo NEW arm graded against a 1500-recorded
ceiling is **lenient, not conservative** — the MTU gain flatters the ratio and
could let a real regression pass. Any resolution changes the frozen-baseline
contract or the rig config, so it needs an owner amendment.

## HIGH 2 — 78 ms is *between*-session noise; it cannot certify the interleaved design → **ACCEPTED**

I wrote that the same-session interleave "is the only design with enough
resolution". The 78 ms floor rules **out** unpaired between-session grading; it
says nothing about whether the *paired within-session* variance is small enough.
Those are different quantities and pf-0 measured only the first.

**Fix**: split the claim. Between-session grading is definitively ruled out;
whether interleaving suffices is **unknown**, so **pf-1 must measure its own
paired within-session floor on the unmodified build and register a resolution
check (smallest reportable recovery > that floor) before grading any
hypothesis.** A pf-1 recovery quoted without its paired floor is uninterpretable.

## MEDIUM — H5/H6/H7 are **P2** hypotheses, not P1 → **ACCEPTED** (and the error was pre-existing)

Verified against the plan's own hypothesis list: `H1 (P1)`, `H2 (P1)`,
`H3 (P2)`, `H4 (P2)`, `H5 (P2)`, `H6 (P2)`, `H7 (P2)`. Only H1 (plus H2's
residual) bears on P1, so an MTU result cannot "preserve the premise" of
H5/H6/H7 — they are not P1 hypotheses at all.

**This is a propagated repo error, not one I introduced**: the "H1/H5/H6/H7"
framing already stood in `docs/bench/otp12-q-baseline-2026-07-13/README.md`
(lines 8, 45) and `docs/STATE.md:178`. I copied it. All three are corrected in
the fix commit, with a correction note in the q-baseline README (its *result* is
unaffected).

## LOW 1 — "committed before any datum existed" is literally false → **ACCEPTED**

Rev 4 post-dates the discarded A-B-B-A attempt and the `q` baseline. What is
true — and what actually matters for pre-registration — is that the **decision
rule, thresholds and guards were fixed in rev 3, before any S1–S4 datum, and rev
4 did not touch them** (it re-described the rig).

**Fix**: precise provenance wording in both the plan section and the README.

## LOW 2 — the masking guard's condition-level values silently use means → **ACCEPTED**

The pre-registration never said how two replicate medians collapse into one
value per arm; I used their mean without saying so, while claiming the analysis
was "exactly as pre-registered". The guard outcome is identical under every
plausible alternative (either replicate alone, or pooled runs), so no verdict
moved — but the spec's precision was overstated.

**Fix**: the choice is disclosed in the README's masking-guard section.

## INFO — independent recomputation matches → noted, no action

---

## Added by the author (not a codex finding): the fast arm is BISTABLE

Recomputing medians from the raw `runs.csv` (rather than trusting
`summary.csv`) shows `win_init` is **bimodal** — clusters near ~730 ms and
~840 ms. S1 drew 6 low / 2 high and S4 drew 2 low / 6 high **at the same MTU**;
that mixture, not MTU, produced the 72 ms `win_init` replicate spread and hence
N_Δ. `mac_init` is stable to 5–6 ms. Recorded in both docs because it is (a) the
mechanical explanation of the noise floor the two BLOCKERs turn on, and (b) a
**trap for pf-1**: a counterfactual that merely shifts the mode mixture would
masquerade as a recovery. The MTU verdict is robust to it — pooling all 16 runs
per condition gives r = −4.7%, the same KILLED grade.

---

**Fix sha**: `11f0c2a` — `pf-0-result: address review (7 findings) — the claims outran the data`
**Validation**: `bash scripts/agent/check-docs.sh` → OK (docs/plan-only change;
per D-2026-07-04-1 the docs gate replaces the cargo gate, and the review step
still ran).
