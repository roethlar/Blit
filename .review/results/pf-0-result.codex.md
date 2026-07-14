Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f5f2b-209b-74a1-ab0e-5d90f518f9b9
--------
user
Review the diff of commit 63f400e (run: git show 63f400e). It is a PLAN change to docs/plan/OTP12_PERF_FINDINGS.md recording the outcome of the pre-registered MTU experiment whose design is docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md (rev 4) and whose evidence + analysis is docs/bench/otp12-jumbo-win-2026-07-13/README.md (committed as 363fa6f; raw data in that dir: S1_9000/ S2_1500/ S3_1500/ S4_9000/ each with summary.csv, verdicts.csv, runs.csv, mss-start/end.txt).

Check, hardest first:
(1) FAITHFULNESS TO THE PRE-REGISTRATION: was the registered decision rule applied EXACTLY as written (domain guard first, noise floor N_D from same-MTU replicates, r formula, the 20/50 percent scale, the registered edge cases, the masking guard rebuilt on measured noise)? Recompute r, Delta_9000, Delta_1500, N_D and N_arm from the committed summary.csv files yourself and say whether the numbers in the README and the plan match the data. Flag ANY post-hoc threshold, any rule bent to fit the numbers, or any conclusion the prereg does not license.
(2) IS THE NULL SOUND? The README argues the null is not vacuous because wm_tcp_large improved 3-4 percent at jumbo. Is that argument valid, and are there alternative explanations for a null that the analysis fails to exclude (e.g. segment fill, a bottleneck elsewhere, the noise floor swamping a real effect -- note N_D=78ms vs the 7ms difference between conditions: is the experiment even POWERED to detect a contributing-size effect, and does the doc admit that honestly)?
(3) The new claim that a 20 percent recovery (~46ms) is below the 78ms between-session noise floor, and therefore pf-1 counterfactuals must be interleaved within-session. Is that inference correct, and is 78ms the right floor to compare against a WITHIN-session interleaved measurement (or is the doc conflating between-session and within-session noise)?
(4) Internal coherence with the rest of OTP12_PERF_FINDINGS.md and with docs/DECISIONS.md; whether the OPEN committed-baseline consequence is stated correctly (does acceptance really require the committed reference at jumbo, and is VOID the right call?).
(5) Anything the plan now overstates or understates.

Output a concise markdown findings list -- each finding with file:line, severity, rationale -- then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll independently recompute the registered metrics, audit the rule ordering and edge cases, and cross-check the plan against the README, the rest of the findings document, and `docs/DECISIONS.md`. I’ll make no changes.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
The review checkout is clean except for an unrelated untracked review artifact, which I’m leaving untouched. The canonical remote is ahead of this checkout (`github/HEAD e6845ca`, local `63f400e`), so I’ll treat current-state notes as potentially stale; the requested commit and its committed evidence are present locally.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
2026-07-14T05:52:45.236115Z ERROR codex_core::tools::router: error=timeout_ms must be at least 10000
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
- **BLOCKER — [OTP12_PERF_FINDINGS.md:281](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:281):** `EXCLUDED`, “environmental escape … closed,” and “code, not wire” exceed the registered `KILLED as a material cause` outcome. The preregistration limits the null to “raising MTU did not improve these cells under observed packetization” because segment fill remains unknown ([PREREGISTRATION.md:107](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:107)). MSS and the bulk-cell improvement show the treatment reached the path, but do not exclude underfilled segments, another bottleneck, or a smaller wire contribution.

- **BLOCKER — [README.md:34](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:34):** The registered point-estimate rule was followed, but the experiment is not powered for its own CONTRIBUTING boundary: `20% × 229 = 45.8 ms < N_Δ = 78 ms`. The domain guard establishes that P1 exists above noise; it does not establish that a 20% MTU effect is detectable. A contributing-size recovery could therefore be swamped. The README admits segment-fill uncertainty, but not this power limitation.

- **HIGH — [OTP12_PERF_FINDINGS.md:321](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:321):** Acceptance really does require both references, but automatic `VOID`, “only two ways,” and blocking `pf-final` are not authorized by the current contract. The committed 2026-07-10 baseline is explicitly a fixed anti-drift ceiling ([OTP12_ACCEPTANCE_RUN.md:168](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:168)), and the baselines remain frozen ([OTP12_ACCEPTANCE_RUN.md:290](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:290)); D-2026-07-05-4 keeps those pins standing. Ignoring committed rows is correct for this MTU experiment’s causal analysis, but pf-final currently compares jumbo new against both jumbo same-session old and the frozen historical ceiling. Re-recording or adding an MTU-mismatch void rule requires an owner amendment.

- **HIGH — [OTP12_PERF_FINDINGS.md:307](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:307):** `78 ms` is between-session variability. It supports rejecting an unpaired, separate-session 46 ms comparison, but cannot prove that interleaving has “enough resolution” or bound all counterfactuals as claimed again at line 565. Pf-1 needs a contemporaneous unmodified `Δ` and paired within-session variance; neither is measured here.

- **MEDIUM — [OTP12_PERF_FINDINGS.md:294](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:294):** H5, H6, and H7 are P2 hypotheses, not P1 hypotheses. This MTU result neither supplies nor preserves their alleged P1 premise; only H1 is relevant here.

- **LOW — [OTP12_PERF_FINDINGS.md:283](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:283):** “Rev 4, committed before any datum existed” is literally inaccurate: rev 4 followed a discarded A-B-B-A run and incorporated the measured `q` baseline. The retained S1–S4 data came later and the decision formulas were unchanged from rev 3, so no post-hoc threshold was found.

- **LOW — [README.md:65](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:65):** The preregistration does not specify how replicate medians become condition-level per-arm values for the masking guard; the README silently uses means. Every plausible raw/mean comparison gives the same guard outcome, so this did not bend the verdict, but “exactly as pre-registered” overstates specificity.

- **INFO — [README.md:17](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:17):** Independent recomputation matches the data: session deltas `275/241/217/197 ms`; `Δ_9000=236`, `Δ_1500=229`, `N_Δ=78`, `r=-3.0568%`, arm differences `6/72/5/19 ms`, and `N_arm=72`. Domain guard, registered edge cases, masking checks, MSS gates, and the 20/50 scale were otherwise applied correctly. The reported large-cell improvements are also correct: `960→924 ms` and `945→916 ms`.

**VERDICT: NOT READY — arithmetic-faithful, but the causal exclusion, power claim, interleaving inference, and pf-final baseline consequence are not sound as written.**
tokens used
124,709
- **BLOCKER — [OTP12_PERF_FINDINGS.md:281](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:281):** `EXCLUDED`, “environmental escape … closed,” and “code, not wire” exceed the registered `KILLED as a material cause` outcome. The preregistration limits the null to “raising MTU did not improve these cells under observed packetization” because segment fill remains unknown ([PREREGISTRATION.md:107](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:107)). MSS and the bulk-cell improvement show the treatment reached the path, but do not exclude underfilled segments, another bottleneck, or a smaller wire contribution.

- **BLOCKER — [README.md:34](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:34):** The registered point-estimate rule was followed, but the experiment is not powered for its own CONTRIBUTING boundary: `20% × 229 = 45.8 ms < N_Δ = 78 ms`. The domain guard establishes that P1 exists above noise; it does not establish that a 20% MTU effect is detectable. A contributing-size recovery could therefore be swamped. The README admits segment-fill uncertainty, but not this power limitation.

- **HIGH — [OTP12_PERF_FINDINGS.md:321](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:321):** Acceptance really does require both references, but automatic `VOID`, “only two ways,” and blocking `pf-final` are not authorized by the current contract. The committed 2026-07-10 baseline is explicitly a fixed anti-drift ceiling ([OTP12_ACCEPTANCE_RUN.md:168](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:168)), and the baselines remain frozen ([OTP12_ACCEPTANCE_RUN.md:290](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:290)); D-2026-07-05-4 keeps those pins standing. Ignoring committed rows is correct for this MTU experiment’s causal analysis, but pf-final currently compares jumbo new against both jumbo same-session old and the frozen historical ceiling. Re-recording or adding an MTU-mismatch void rule requires an owner amendment.

- **HIGH — [OTP12_PERF_FINDINGS.md:307](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:307):** `78 ms` is between-session variability. It supports rejecting an unpaired, separate-session 46 ms comparison, but cannot prove that interleaving has “enough resolution” or bound all counterfactuals as claimed again at line 565. Pf-1 needs a contemporaneous unmodified `Δ` and paired within-session variance; neither is measured here.

- **MEDIUM — [OTP12_PERF_FINDINGS.md:294](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:294):** H5, H6, and H7 are P2 hypotheses, not P1 hypotheses. This MTU result neither supplies nor preserves their alleged P1 premise; only H1 is relevant here.

- **LOW — [OTP12_PERF_FINDINGS.md:283](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:283):** “Rev 4, committed before any datum existed” is literally inaccurate: rev 4 followed a discarded A-B-B-A run and incorporated the measured `q` baseline. The retained S1–S4 data came later and the decision formulas were unchanged from rev 3, so no post-hoc threshold was found.

- **LOW — [README.md:65](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:65):** The preregistration does not specify how replicate medians become condition-level per-arm values for the masking guard; the README silently uses means. Every plausible raw/mean comparison gives the same guard outcome, so this did not bend the verdict, but “exactly as pre-registered” overstates specificity.

- **INFO — [README.md:17](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:17):** Independent recomputation matches the data: session deltas `275/241/217/197 ms`; `Δ_9000=236`, `Δ_1500=229`, `N_Δ=78`, `r=-3.0568%`, arm differences `6/72/5/19 ms`, and `N_arm=72`. Domain guard, registered edge cases, masking checks, MSS gates, and the 20/50 scale were otherwise applied correctly. The reported large-cell improvements are also correct: `960→924 ms` and `945→916 ms`.

**VERDICT: NOT READY — arithmetic-faithful, but the causal exclusion, power claim, interleaving inference, and pf-final baseline consequence are not sound as written.**
