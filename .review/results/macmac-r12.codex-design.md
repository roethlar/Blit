Reading additional input from stdin...
OpenAI Codex v0.144.4
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f62b1-0257-7cd1-9dc8-4e1923dbe9bb
--------
user
You are being asked whether an experiment is worth running, and whether it will answer the question it claims to answer. Do NOT assume the plan is right. The plan is mine, it has been rewritten eleven times, and every round has found something wrong with it — including the plan itself. Argue with it.

THE SITUATION

A file-transfer tool (blit) has one performance finding standing between it and release. Call it P1:

  On a macOS <-> Windows pair, over TCP, on a "mixed" fixture (5001 files, 547MB),
  the transfer is ~25-38% slower when the DESTINATION machine initiates it than when
  the SOURCE machine does. Same code, same bytes, same direction of data flow — only
  which end typed the command changes. Measured four times (ratios 1.237, 1.300,
  1.385, 1.362). The project's acceptance criterion says no transfer may vary by more
  than 10% based on who initiated it, so P1 blocks the release.

What is known:
  - Linux <-> Linux: NO P1 (8/8 cells pass, the same cell reads 1.092/1.003).
  - macOS <-> Windows: P1, every time.
  - MTU is NOT the cause (a 256-run A-B-B-A experiment at 9000 vs 1500 killed it: r = -3.1%).
  - macOS <-> macOS: NEVER TESTED. That is the missing cell of the 2x2.
  - A code-level hypothesis (H1) accuses blit's own dial/accept paths — which run on macOS too.

THE PROPOSED EXPERIMENT (the thing you are reviewing)

Run the destination-initiated vs source-initiated comparison on a pair of Macs (nagatha
and q, 10GbE, MTU 9000), to answer ONE question: can P1 occur WITHOUT a Windows peer?

  - 6 cells: 2 measurands (TCP x mixed, both directions) + 4 controls (gRPC x mixed both
    directions, TCP x large-file both directions). The controls exist because P1 is claimed
    to be specific to TCP x mixed: if the controls show the same asymmetry, the rig itself is
    biased and nothing can be read off it.
  - 8 ABBA pairs per cell, slot-major (every cell takes one pair before any cell takes its
    next), so all cells see the same wall-clock window and the same transients.
  - A pre-registered decision rule, computed by the harness, so no one can look at the numbers
    and then invent a favourable reading.

The files:
  docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md   the design + the decision rule
  scripts/bench_otp12pf_mac.sh                            the harness (gates, ABBA, timing)
  scripts/otp12pf_mac_verdict.py                          the decision rule, mechanized
  scripts/otp12pf_mac_verdict_test.py                     its guard test
  docs/plan/OTP12_PERF_FINDINGS.md                        the parent plan (what P1/P2 are)

NO DATA HAS EVER BEEN TAKEN. The instrument has been reviewed eleven times, ~110 defects
found and fixed, and it has still never run. That is itself a signal I want you to weigh.

WHAT I ACTUALLY WANT FROM YOU — in this order:

1. WILL THIS EXPERIMENT ANSWER THE QUESTION? Take the goal seriously and forget my design for
   a moment. Given what is known (the 2x2 above, MTU killed, H1 accusing blit's own code), is
   a Mac<->Mac reproduction run the right next move at all? What would you do instead, or
   first? Consider cheaper, more direct, more diagnostic alternatives — e.g. instrumenting the
   dial/accept path directly, packet capture, a targeted micro-benchmark of the suspected code
   path, or bisecting the code hypothesis. If the experiment is worth running, say so. If it
   is a detour that will produce an ambiguous result whatever happens, say THAT — plainly.

2. WHAT DOES EACH POSSIBLE OUTCOME ACTUALLY BUY? The rig can say REPRODUCES, DOES-NOT-
   REPRODUCE, MIXED, UNCLEAR, CONTROLS-NOT-CLEAN, INCOMPLETE, RIG-VOID. For each, what does
   the project legitimately learn, and what would we be tempted to conclude that we may NOT?
   Is there an outcome that would change what we do next? If NO outcome changes the next
   action, the experiment is theatre and you should say so.

3. IS THE DESIGN SOUND, and is it the right SHAPE? Are the controls the right controls? Is
   ABBA + slot-major the right schedule? Is n=8 pairs per cell enough to see a 25-38% effect,
   and is it enough to support the NULL the rule can emit? (The rule judges a null on the full
   range of all 8 paired differences.) Is anything important missing — a confound not
   controlled, a cell not run, an alternative explanation the design cannot separate?

4. IS THE DECISION RULE THE RIGHT RULE? It is deliberately austere: one threshold
   T = min(10% of the source arm, 230ms), four cell states from an exact order-statistic CI,
   controls must be clean at T/2, and a bias term B (what the clean controls could not exclude)
   that must only ever HARDEN a verdict. Is that sound? Is it too clever? Would a
   statistician recognize it, and would they trust a null out of it? Note n is EXACTLY 8, so
   the >=95% CI IS [min,max].

5. ONLY THEN: correctness bugs. Anything in the harness or the rule that would produce a WRONG
   measurement or a WRONG verdict. Two defect classes recur in every round and you should
   assume both are still present: (a) a fix applied where it was SHOWN rather than where the
   defect LIVED (the duplicate site is left broken), and (b) a protection that never actually
   executes, or that cannot fail — e.g. an equal-settle window that was dead for three
   revisions while the document asserted it, and, this round, pinned constants placed above
   the check meant to enforce them so the harness refused every run.

You may run `SELFTEST=1 bash scripts/bench_otp12pf_mac.sh` (exercises the gates on both Macs,
takes NO data), and `python3 scripts/otp12pf_mac_verdict_test.py [--mutations]`.

Be direct. If the honest answer is "this is a well-built instrument pointed at the wrong
question", that is the most valuable thing you can tell me, and it will not offend me. If it
is worth running, tell me what you would change before it runs, and what the result will and
will not license.

Output: a short prose verdict on (1) and (2) first — that matters more than a findings list —
then the findings for (3)(4)(5) with file:line and severity. End with a final line:
RUN IT / RUN IT WITH CHANGES / DO NOT RUN IT — and one sentence saying why. Do not invoke skills.
codex
I’ll treat the causal question separately from the rig, then audit the preregistration, harness, verdict logic, and tests against one another. I’ll run the two authorized self-tests if the environment permits, but I won’t change files.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
The repo’s current state already retracts one important earlier claim: Mac↔Mac is not an H1 discriminator because H1 accuses code that runs on both operating systems. I’m now checking whether the narrower “Windows necessary?” question changes the project’s next action enough to justify this instrument.
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
codex
The schedule is more careful than the causal question warrants, but I’ve found a structural issue already: the end-of-session “fabric re-check” verifies only MSS and source IP, not negotiated link speed. A 10GbE link that renegotiates to 1GbE can keep the same MTU, MSS, and IP and still be graded. I’m checking for the same live/dead protection pattern elsewhere and validating the verdict tests.
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
mcp: ptk/ptk_job started
mcp: ptk/ptk_job (completed)
mcp: ptk/ptk_job started
mcp: ptk/ptk_job (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_job started
mcp: ptk/ptk_job (completed)
mcp: ptk/ptk_job started
mcp: ptk/ptk_job (completed)
mcp: ptk/ptk_job started
mcp: ptk/ptk_job (completed)
mcp: ptk/ptk_job started
mcp: ptk/ptk_job (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
The Mac↔Mac run is not the right next move. At best, a positive result gives you a convenient local reproduction rig. It does not adjudicate H1, identify a cause, or alter P1’s release obligation. A negative result is especially weak because the Mac↔Mac harness introduces the corrected equal-settle method that was not used for the Mac↔Windows measurements defining P1.

First, rerun the known-failing Mac↔Windows cell with this corrected timing method while recording dial, accept, ACK, resize-epoch, first-byte, and completion timestamps; take a packet capture concurrently. If P1 survives, test H1 with a causal dial/accept counterfactual. A targeted microbenchmark comes after the trace identifies a phase, because the mixed-only interaction may disappear in isolation. A cheap two-cell Mac↔Mac screen could then establish whether local debugging is possible; the current 96-transfer apparatus is disproportionate.

## What each outcome buys

| Outcome | Legitimate learning | Not licensed |
|---|---|---|
| `REPRODUCES` | P1-shaped behavior can occur on these Macs without Windows; use them as a local debugging rig. | H1 confirmed or killed; platform-general behavior; Windows exonerated as a contributor; release acceptance. |
| `DOES-NOT-REPRODUCE` | Under this method and window, both directions exclude a median gap of the registered size, subject to the statistical assumptions. | Windows is necessary; H1 confirmed; the four Mac↔Windows observations retracted; no tail or intermittent effect. |
| `MIXED` | Strong host×role, ordering, or endpoint interaction requiring decomposition. | Either reproduction or null. |
| `INVERTED` | A new, opposite initiator asymmetry. | P1 absent. |
| `UNCLEAR` | The rig did not resolve the margin. | A weak null or partial reproduction. |
| `CONTROLS-NOT-CLEAN` | Either a control has initiator asymmetry or the controls cannot bound the proposed bias. | Anything about P1 itself. |
| `INCOMPLETE` | Only an execution/feasibility failure. | Any performance inference. |
| `RIG-VOID` | Environmental conditions invalidated the session. | Any performance inference. |

Only `REPRODUCES` changes anything operationally: it changes where debugging can happen. Every substantive path still leads to direct instrumentation and a causal counterfactual, so the full experiment is close to theatre even before its remaining defects.

## (3) Design findings

- **BLOCKER — no same-method positive control.** The new instrument deliberately inserts and verifies a settle before fsync at [bench_otp12pf_mac.sh:925](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:925), because the previous timing gap could manufacture a one-directional effect. The original rig-W harness flushes immediately at [bench_otp12_win.sh:505](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:505). Before asking whether Windows is necessary, establish that P1 still exists on Mac↔Windows under the corrected method. Otherwise a Mac↔Mac null conflates platform pairing with measurement repair.

- **HIGH — the controls do not fully support “TCP × mixed-specific.”** The registered controls are gRPC-mixed and TCP-large at [PREREGISTRATION.md:164](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:164), but prior evidence defining the signature also uses TCP-small as a clean fixture control at [OTP12_PERF_FINDINGS.md:228](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:228). Mixed contains 5,000 small files; TCP-small is the most relevant check for a small-file/metadata interaction masquerading as “mixed-only.”

- **HIGH — n=8 is enough for an exact median CI, not enough to promise a useful answer.** Under independent, identically distributed paired differences, `[min,max]` has 99.22% coverage for the population median. But `EFFECT` requires every pair to clear the margin and `NONE` requires every pair inside it. One soft pair produces `UNCLEAR`; modest control noise produces `CONTROLS-NOT-CLEAN`. No empirical power calculation justifies expecting decisiveness. ABBA controls first-order drift but does not establish independence or stationarity. See [PREREGISTRATION.md:201](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:201) and [PREREGISTRATION.md:380](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:380).

- **HIGH — the null’s estimand is unclear.** If the claim is median equivalence, the exact CI is recognizable and defensible under IID assumptions. If “no transfer may vary by more than 10%” means individual or future transfers, eight observed values inside the margin cannot establish that; it is not a population tolerance bound. The document alternates between median inference and “every pair” rhetoric at [PREREGISTRATION.md:250](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:250).

- **MEDIUM — in-place retries weaken contemporaneous controls.** A voided cell retries repeatedly before its siblings run at [bench_otp12pf_mac.sh:1118](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:1118). After several long retries, its valid observation and the controls no longer share the same transient. Retry after completing the rest of the slot, or explicitly abandon that slot.

AB/BA alternation, slot-major traversal, and rotation are otherwise the right general scheduling shape.

## (4) Decision-rule findings

- **BLOCKER — `min()` implements the opposite of the stated policy.** The document says an effect must matter by both the 10% bar and the 230 ms reference at [PREREGISTRATION.md:212](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:212). `min(src/10, 230)` lets either smaller standard win. The test deliberately labels `230 ms @ 2500 ms` as `REPRODUCES` at [otp12pf_mac_verdict_test.py:111](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:111), even though its ratio is 1.092 and the engine’s own acceptance bar says `PASS`. Thus the engine can confidently report “P1 reproduces” when P1’s defining invariance criterion passes.

- **BLOCKER — the exact 10% boundary is simultaneously PASS and REPRODUCES.** The bar treats `≤1.10` as PASS at [otp12pf_mac_verdict.py:282](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:282), while `classify()` uses `CI_lo >= T` at [otp12pf_mac_verdict.py:251](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:251). The guard explicitly requires an exact 10% effect to reproduce at [otp12pf_mac_verdict_test.py:217](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:217). That is a direct policy contradiction, not statistical conservatism.

- **HIGH — `B` is not a statistically justified bias bound.** It assumes rig bias scales multiplicatively with arm time at [otp12pf_mac_verdict.py:303](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:303). The known SSH dispatch residual is naturally additive in milliseconds, while free-writeback response may be nonlinear. A 49 ms additive bias observed on a slow control is scaled downward on a faster measurand and can under-harden a verdict. No control evidence establishes the chosen proportional model. A conservative bound would need a predeclared confound model—at minimum the maximum of additive and proportional bounds—or closely speed-matched controls.

- **HIGH — the `B ≥ T/2` refusal checks only the positive margin.** The implementation checks `t_pos` at [otp12pf_mac_verdict.py:343](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:343), while the actual negative margin is smaller (`src/11`) at [otp12pf_mac_verdict.py:220](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:220). `B` can therefore exceed half the relevant inverted/null margin while the session still grades.

- **MEDIUM — the preregistration does not register one unambiguous rule.** Its summary describes a single symmetric `±T` at [PREREGISTRATION.md:7](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:7), while the body and engine use separate `src/10` and `src/11` margins at [PREREGISTRATION.md:214](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:214). That matters to `INVERTED`, `NONE`, control certification, and the bias refusal.

The four implemented cell states are otherwise mutually exclusive and exhaustive for valid positive timings. Exact n=8 enforcement, per-cell B hardening, and MIXED-on-unhardened-states look internally correct. A statistician would recognize the order-statistic CI; they would not recognize the B construction and session policy as a calibrated inferential procedure.

## (5) Correctness bugs

- **BLOCKER — the end-fabric protection does not recheck the full fabric it promises.** Startup checks MTU, negotiated `10Gbase-T`, and link status at [bench_otp12pf_mac.sh:499](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:499). The end gate checks only MSS and local IP at [bench_otp12pf_mac.sh:557](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:557). A link can renegotiate from 10GbE to 1GbE while retaining MTU 9000, MSS 8948, and the same IP; the session will grade. This is exactly the duplicate-site defect class.

- **HIGH — failed landed-tree cleanup records a marker but does not invalidate anything.** `clear_landed()` touches `LANDED-TREE-SURVIVED` at [bench_otp12pf_mac.sh:1041](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:1041), but the marker is never consulted before verdict computation at [bench_otp12pf_mac.sh:1354](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:1354). Repeated survivors can change disk occupancy and later I/O while the session still reports a confident verdict.

- **MEDIUM — manifest fabric/hash probes discard producer status.** Local verdict/test hashes are not validated at [bench_otp12pf_mac.sh:739](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:739), and `mss_read` is embedded inside `echo` at [bench_otp12pf_mac.sh:750](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:750). A failed reader produces an empty “measured fabric” entry while `echo` succeeds and the run continues.

- **MEDIUM — mutation “kills” can occur for the wrong reason.** The mutation runner counts any unexpected outcome—including a crash or a different safe refusal—as a kill at [otp12pf_mac_verdict_test.py:537](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:537). In the run, the missing-cell mutation was “killed” by `ENGINE-CRASH`, and the zero-time mutation by `CONTROLS-NOT-CLEAN`, rather than exercising their claimed dangerous verdicts. The suite detects edits, but some mutations do not prove the safety property attributed to them.

Validation completed: the verdict suite reported 40/40 cases, 19/19 mutations, and 300/300 fuzz inputs. The harness selftest completed with zero blind gates; quiescence correctly fired because `codex` was running. It took no transfer data. As the selftest itself states, it does not exercise daemon lifecycle, smoke transfer, ABBA execution, pair voiding, or the manifest.

**DO NOT RUN IT — first validate P1 with the corrected measurement on the known-failing Mac↔Windows pair and instrument the suspected path; the current rule can call a 1.092/PASS cell “REPRODUCES,” and the end-fabric gate is incomplete.**
tokens used
200,566
The Mac↔Mac run is not the right next move. At best, a positive result gives you a convenient local reproduction rig. It does not adjudicate H1, identify a cause, or alter P1’s release obligation. A negative result is especially weak because the Mac↔Mac harness introduces the corrected equal-settle method that was not used for the Mac↔Windows measurements defining P1.

First, rerun the known-failing Mac↔Windows cell with this corrected timing method while recording dial, accept, ACK, resize-epoch, first-byte, and completion timestamps; take a packet capture concurrently. If P1 survives, test H1 with a causal dial/accept counterfactual. A targeted microbenchmark comes after the trace identifies a phase, because the mixed-only interaction may disappear in isolation. A cheap two-cell Mac↔Mac screen could then establish whether local debugging is possible; the current 96-transfer apparatus is disproportionate.

## What each outcome buys

| Outcome | Legitimate learning | Not licensed |
|---|---|---|
| `REPRODUCES` | P1-shaped behavior can occur on these Macs without Windows; use them as a local debugging rig. | H1 confirmed or killed; platform-general behavior; Windows exonerated as a contributor; release acceptance. |
| `DOES-NOT-REPRODUCE` | Under this method and window, both directions exclude a median gap of the registered size, subject to the statistical assumptions. | Windows is necessary; H1 confirmed; the four Mac↔Windows observations retracted; no tail or intermittent effect. |
| `MIXED` | Strong host×role, ordering, or endpoint interaction requiring decomposition. | Either reproduction or null. |
| `INVERTED` | A new, opposite initiator asymmetry. | P1 absent. |
| `UNCLEAR` | The rig did not resolve the margin. | A weak null or partial reproduction. |
| `CONTROLS-NOT-CLEAN` | Either a control has initiator asymmetry or the controls cannot bound the proposed bias. | Anything about P1 itself. |
| `INCOMPLETE` | Only an execution/feasibility failure. | Any performance inference. |
| `RIG-VOID` | Environmental conditions invalidated the session. | Any performance inference. |

Only `REPRODUCES` changes anything operationally: it changes where debugging can happen. Every substantive path still leads to direct instrumentation and a causal counterfactual, so the full experiment is close to theatre even before its remaining defects.

## (3) Design findings

- **BLOCKER — no same-method positive control.** The new instrument deliberately inserts and verifies a settle before fsync at [bench_otp12pf_mac.sh:925](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:925), because the previous timing gap could manufacture a one-directional effect. The original rig-W harness flushes immediately at [bench_otp12_win.sh:505](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:505). Before asking whether Windows is necessary, establish that P1 still exists on Mac↔Windows under the corrected method. Otherwise a Mac↔Mac null conflates platform pairing with measurement repair.

- **HIGH — the controls do not fully support “TCP × mixed-specific.”** The registered controls are gRPC-mixed and TCP-large at [PREREGISTRATION.md:164](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:164), but prior evidence defining the signature also uses TCP-small as a clean fixture control at [OTP12_PERF_FINDINGS.md:228](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:228). Mixed contains 5,000 small files; TCP-small is the most relevant check for a small-file/metadata interaction masquerading as “mixed-only.”

- **HIGH — n=8 is enough for an exact median CI, not enough to promise a useful answer.** Under independent, identically distributed paired differences, `[min,max]` has 99.22% coverage for the population median. But `EFFECT` requires every pair to clear the margin and `NONE` requires every pair inside it. One soft pair produces `UNCLEAR`; modest control noise produces `CONTROLS-NOT-CLEAN`. No empirical power calculation justifies expecting decisiveness. ABBA controls first-order drift but does not establish independence or stationarity. See [PREREGISTRATION.md:201](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:201) and [PREREGISTRATION.md:380](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:380).

- **HIGH — the null’s estimand is unclear.** If the claim is median equivalence, the exact CI is recognizable and defensible under IID assumptions. If “no transfer may vary by more than 10%” means individual or future transfers, eight observed values inside the margin cannot establish that; it is not a population tolerance bound. The document alternates between median inference and “every pair” rhetoric at [PREREGISTRATION.md:250](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:250).

- **MEDIUM — in-place retries weaken contemporaneous controls.** A voided cell retries repeatedly before its siblings run at [bench_otp12pf_mac.sh:1118](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:1118). After several long retries, its valid observation and the controls no longer share the same transient. Retry after completing the rest of the slot, or explicitly abandon that slot.

AB/BA alternation, slot-major traversal, and rotation are otherwise the right general scheduling shape.

## (4) Decision-rule findings

- **BLOCKER — `min()` implements the opposite of the stated policy.** The document says an effect must matter by both the 10% bar and the 230 ms reference at [PREREGISTRATION.md:212](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:212). `min(src/10, 230)` lets either smaller standard win. The test deliberately labels `230 ms @ 2500 ms` as `REPRODUCES` at [otp12pf_mac_verdict_test.py:111](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:111), even though its ratio is 1.092 and the engine’s own acceptance bar says `PASS`. Thus the engine can confidently report “P1 reproduces” when P1’s defining invariance criterion passes.

- **BLOCKER — the exact 10% boundary is simultaneously PASS and REPRODUCES.** The bar treats `≤1.10` as PASS at [otp12pf_mac_verdict.py:282](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:282), while `classify()` uses `CI_lo >= T` at [otp12pf_mac_verdict.py:251](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:251). The guard explicitly requires an exact 10% effect to reproduce at [otp12pf_mac_verdict_test.py:217](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:217). That is a direct policy contradiction, not statistical conservatism.

- **HIGH — `B` is not a statistically justified bias bound.** It assumes rig bias scales multiplicatively with arm time at [otp12pf_mac_verdict.py:303](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:303). The known SSH dispatch residual is naturally additive in milliseconds, while free-writeback response may be nonlinear. A 49 ms additive bias observed on a slow control is scaled downward on a faster measurand and can under-harden a verdict. No control evidence establishes the chosen proportional model. A conservative bound would need a predeclared confound model—at minimum the maximum of additive and proportional bounds—or closely speed-matched controls.

- **HIGH — the `B ≥ T/2` refusal checks only the positive margin.** The implementation checks `t_pos` at [otp12pf_mac_verdict.py:343](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:343), while the actual negative margin is smaller (`src/11`) at [otp12pf_mac_verdict.py:220](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:220). `B` can therefore exceed half the relevant inverted/null margin while the session still grades.

- **MEDIUM — the preregistration does not register one unambiguous rule.** Its summary describes a single symmetric `±T` at [PREREGISTRATION.md:7](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:7), while the body and engine use separate `src/10` and `src/11` margins at [PREREGISTRATION.md:214](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:214). That matters to `INVERTED`, `NONE`, control certification, and the bias refusal.

The four implemented cell states are otherwise mutually exclusive and exhaustive for valid positive timings. Exact n=8 enforcement, per-cell B hardening, and MIXED-on-unhardened-states look internally correct. A statistician would recognize the order-statistic CI; they would not recognize the B construction and session policy as a calibrated inferential procedure.

## (5) Correctness bugs

- **BLOCKER — the end-fabric protection does not recheck the full fabric it promises.** Startup checks MTU, negotiated `10Gbase-T`, and link status at [bench_otp12pf_mac.sh:499](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:499). The end gate checks only MSS and local IP at [bench_otp12pf_mac.sh:557](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:557). A link can renegotiate from 10GbE to 1GbE while retaining MTU 9000, MSS 8948, and the same IP; the session will grade. This is exactly the duplicate-site defect class.

- **HIGH — failed landed-tree cleanup records a marker but does not invalidate anything.** `clear_landed()` touches `LANDED-TREE-SURVIVED` at [bench_otp12pf_mac.sh:1041](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:1041), but the marker is never consulted before verdict computation at [bench_otp12pf_mac.sh:1354](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:1354). Repeated survivors can change disk occupancy and later I/O while the session still reports a confident verdict.

- **MEDIUM — manifest fabric/hash probes discard producer status.** Local verdict/test hashes are not validated at [bench_otp12pf_mac.sh:739](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:739), and `mss_read` is embedded inside `echo` at [bench_otp12pf_mac.sh:750](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:750). A failed reader produces an empty “measured fabric” entry while `echo` succeeds and the run continues.

- **MEDIUM — mutation “kills” can occur for the wrong reason.** The mutation runner counts any unexpected outcome—including a crash or a different safe refusal—as a kill at [otp12pf_mac_verdict_test.py:537](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:537). In the run, the missing-cell mutation was “killed” by `ENGINE-CRASH`, and the zero-time mutation by `CONTROLS-NOT-CLEAN`, rather than exercising their claimed dangerous verdicts. The suite detects edits, but some mutations do not prove the safety property attributed to them.

Validation completed: the verdict suite reported 40/40 cases, 19/19 mutations, and 300/300 fuzz inputs. The harness selftest completed with zero blind gates; quiescence correctly fired because `codex` was running. It took no transfer data. As the selftest itself states, it does not exercise daemon lifecycle, smoke transfer, ABBA execution, pair voiding, or the manifest.

**DO NOT RUN IT — first validate P1 with the corrected measurement on the known-failing Mac↔Windows pair and instrument the suspected path; the current rule can call a 1.092/PASS cell “REPRODUCES,” and the end-fabric gate is incomplete.**
