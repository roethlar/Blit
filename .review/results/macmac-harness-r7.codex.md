Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f60f9-2761-7293-b722-ec18f2f81d88
--------
user
Review the Mac<->Mac benchmark instrument at HEAD (commit b3d42b7; the instrument files last changed in 1e03063). Run: git show 1e03063, and read the four files at HEAD.

NO DATA HAS EVER BEEN TAKEN. This instrument has now been through SIX review rounds: 69 findings, 69 accepted, 0 rejected. It decides whether a performance finding (P1: destination-initiated TCP x mixed pays ~25-38%) reproduces on a macOS<->macOS rig. This project has RETRACTED THREE CLAIMS to harness bugs.

Files:
- scripts/bench_otp12pf_mac.sh        the harness
- scripts/otp12pf_mac_verdict.py      the MECHANIZED DECISION RULE
- scripts/otp12pf_mac_verdict_test.py guard test (27 cases) + mutation proof (18 mutations)
- docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md   rev 7, the spec the code must implement

THE QUESTION IS NOT "is this nice code". It is: CAN THIS INSTRUMENT PRODUCE A CONFIDENT, WRONG ANSWER?

TWO DEFECT CLASSES HAVE RECURRED IN EVERY SINGLE ROUND. Assume BOTH are still present and hunt them:

CLASS 1 — "FIXED THE BRANCH THAT WAS SHOWN, NOT THE CLASS." The same materiality bug escaped in THREE consecutive rounds, each time through a branch the previous fix had not covered:
  r3: the equivalence margin was bar-tied on the MEASURAND -> fixed there;
  r4: ...still bar-tied for CONTROLS via the PARTIAL label -> fixed there;
  r5: ...still escaping via the UNDERPOWERED label (one zero pair demotes the cell) -> and separately, `bar == FAIL` had NO DIRECTION, so a +1ms effect at n=16 reported REPRODUCES.
Also: a fail-open `pgrep` was fixed in the quiescence gate and left identical in the stale-daemon probe.
Round 6 restructured these: direction/magnitude/equivalence are now three separate questions (sign test / CI / CI-vs-margin); the control rule is written as an OBLIGATION (contaminating? certified?) rather than a list of outcome labels; there is exactly one process probe. VERIFY THAT THE RESTRUCTURING IS ACTUALLY COMPLETE, and find the next branch it missed.

CLASS 2 — "A FIX THAT NEVER EXECUTED." Round 5 discovered that SETTLE_MS — the equal pre-fsync window introduced specifically to neutralise a free-writeback artifact capable of MANUFACTURING a one-directional result — HAD NEVER RUN. The awk computing its duration sat in a command substitution with the wrong quoting, so it errored, `sleep` got an empty argument and failed, and its exit status was discarded. The pre-registration asserted that fix for THREE revisions while it was dead. `bash -n` sees nothing.
HUNT FOR MORE OF THESE. Which other claimed protections do not actually execute, or execute but cannot fail? Check every gate, every guard, every void path, every sentinel/sed extraction, every `|| true`, every exit-status that gets discarded by a following command. Prefer RUNNING things over reading them.

Also verify specifically:
- the three-question split: sign test = DIRECTION, CI = MAGNITUDE, CI-vs-margin = EQUIVALENCE. Any place they are still tangled? Any input where the taxonomy gives a wrong or unreportable answer?
- the control rule (contaminating -> RIG-VOID; uncertified -> blocks the NULL but not a REPRODUCTION). Can a dirty rig still produce a null? Can a GOOD rig be falsely voided?
- the pinned constants (harness refuses if they are merely present in the env; engine refuses a mismatched DELTA_REF_MS). Any remaining way to retune the rule from outside?
- the RUNS=16 escalation: it must name the prior session dir, the harness reads its session_verdict.txt, and it burns an ESCALATED marker. Still p-hackable?
- SELFTEST: it now reports [OK]/[FIRED]/[BROKEN] and exits nonzero on BROKEN. Is that classification honest, or can a broken probe still be scored [FIRED] (or vice versa)?
- the guard test: 27 cases, 18 mutations. Are the mutations faithful (do they revert the real fix)? Which fixes have NO mutation? The mutation harness now judges a kill by whether the CASE FAILS.
- bash: quoting through ssh (printf %q + heredocs), pipefail, subshell state loss (a gate that sets a global run inside $() loses it), set -e interactions, the ABBA loop, void accounting, trap/cleanup.

If you have shell access, USE IT: drive the verdict engine with synthetic sessions and try to force a wrong verdict, and try to make a gate pass while blind.

Output a concise markdown findings list — each with file:line, severity (BLOCKER/HIGH/MEDIUM/LOW), rationale — then any reproductions with the actual commands and output, then a final VERDICT line: READY TO RUN or NOT READY TO RUN. Be concise; do not invoke skills.

ROUND 7 CONTEXT — what changed since you last saw it (commit 1e03063), and what to attack:
- THE BAR NO LONGER TAKES PART IN INFERENCE AT ALL. It is computed and reported (the project's 1.10 acceptance criterion) but direction/magnitude/equivalence are decided ONLY by the paired statistics. Verify there is no remaining path where a MARGINAL statistic influences a verdict.
- Controls are now a PRECONDITION: contaminating -> RIG-VOID; not certified (must carry < HALF the material effect) -> CONTROLS-UNCERTIFIED and NO measurand verdict at all, neither null nor reproduction. Attack: can a dirty rig still produce ANY verdict? Can a GOOD rig be blocked forever (is the half-margin so tight the controls can never certify, making the instrument useless)? That second failure mode matters: an instrument that can never conclude is also broken.
- The settle is now performed and MEASURED inside the same python process as the fsync walk, reported as settled_ms (a CSV column), and the pair VOIDS if it did not elapse. Attack: can it still be skipped, shadowed, or faked?
- SELFTEST classifies blindness by an EXPLICIT marker (die_blind -> FATAL[PROBE-BLIND]), not by grepping prose, and exits nonzero on any BROKEN. Attack: can a blind gate still score [OK] or [FIRED]?
- ssh dispatch is now ENFORCED (<= SETTLE_MS/4), the escalation is bound to the prior session's runs.csv HASH, and the engine refuses to run with no controls.
Assume BOTH defect classes are STILL present. Find the next one.
codex
I’ll treat this as an adversarial instrument audit, not a style review. I’ll first re-ground in the repository rules and exact diff, then execute the guard/mutation/self-test paths and synthetic verdict sessions, with special attention to protections that appear present but cannot fire.
exec
/bin/zsh -lc 'pwd && git status --short --branch && git remote -v && git rev-parse HEAD && git show --stat --oneline --decorate 1e03063 && git show --format=fuller --no-ext-diff --find-renames 1e03063' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/Users/michael/Dev/blit_v2
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-U9LcCYZA' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GvQtPKQh' (errno=Operation not permitted)
## master...origin/master [ahead 7]
?? .review/results/macmac-harness-r7.codex.md
?? .review/results/macmac-harness-r7.grok.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LT4FGpsJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZLWsEq7x' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-JANm9Ych' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-eHMQ9KXB' (errno=Operation not permitted)
b3d42b7a40adee75b001c112f47325096542abf2
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-3uAktwH1' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nOmhXukv' (errno=Operation not permitted)
1e03063 macmac round 6: stop patching branches — the bar no longer takes part in inference
 .review/results/macmac-harness-r6.codex.md         | 274 +++++++++++++++++++++
 .review/results/macmac-harness-r6.grok.md          | 113 +++++++++
 .../otp12-macmac-2026-07-14/PREREGISTRATION.md     |  24 +-
 scripts/bench_otp12pf_mac.sh                       | 146 ++++++++---
 scripts/otp12pf_mac_verdict.py                     | 161 ++++++++----
 scripts/otp12pf_mac_verdict_test.py                | 111 +++++++--
 6 files changed, 707 insertions(+), 122 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-KNUD1Dtr' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-9ZjBop8W' (errno=Operation not permitted)
commit 1e03063f39c43d2d0ccf152b91364487c9404ed7
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Tue Jul 14 10:11:37 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Tue Jul 14 10:11:37 2026 -0400

    macmac round 6: stop patching branches — the bar no longer takes part in inference
    
    codex (3 BLOCKER, 2 HIGH, 2 MEDIUM) and grok (2 BLOCKER, 2 HIGH, 4 MEDIUM, 2 LOW) both
    returned NOT READY on aebd50b and converged, again, on the same two classes I had asked
    them to hunt. 13 findings, 13 accepted, 0 rejected. NO DATA TAKEN.
    
    CLASS 1, FOURTH INSTANCE — "fixed the branch that was shown, not the class":
    
    - The MARGINAL-median bar was still a substitute for PAIRED magnitude. Round 5 made the
      bar failure direction-aware; codex simply moved the outliers so the bar failed in the
      MATCHING direction: at n=16, three outliers shift the marginal median (1000 -> 1201,
      ratio 1.201, bar FAIL) while every pair in the CI is +1ms. material = bar_fail_pos ->
      REPRODUCES. P1 "reproducing" off a ONE MILLISECOND paired effect. Verified before
      accepting.
      FIX (the class, not the branch): THE BAR NO LONGER TAKES PART IN INFERENCE AT ALL. It
      is the project's ACCEPTANCE criterion — computed, reported, never consulted for
      direction or magnitude. All inference is PAIRED: direction = the sign test, magnitude
      = the paired CI, equivalence = the CI against the margin.
    
    - Certification used the SAME threshold as materiality, so a control carrying D=+229 —
      ONE MILLISECOND under the reference effect — certified as "clean" and the session
      printed VANISHES with the prose "every control is CERTIFIED clean" (grok, reproduced;
      worse at n=16 with zeros padding the CI). Certifying a control with the very threshold
      that DEFINES the effect is incoherent: it would let the gRPC control carry all but 1ms
      of P1 while we claim P1 is TCP-only.
      FIX: a control must carry LESS THAN HALF the material effect. That is not an invented
      number — it is the specificity claim itself, made checkable.
    
    - Uncertified controls blocked only the NULL. With every control at D=+230 the engine
      still confidently declared P1 REPRODUCED (codex). Uncertainty about a rig-wide
      confound is not evidence that the confound is absent.
      FIX: the controls are a PRECONDITION. If any control cannot be certified clean, NO
      measurand verdict is read — not a null, and NOT a reproduction. New registered
      outcome: CONTROLS-UNCERTIFIED.
    
    CLASS 2, SECOND INSTANCE — "a protection that cannot be observed is not a protection":
    
    - The settle repair was still not provable. `sleep` is PATH/function-resolved, the walk's
      timer starts AFTER it, and the self-test only counted files — so a no-op sleep would
      pass while the log narrated "settle included". grok measured a 44ms "250ms settle".
      A log line is a sentence, not an assertion, and that is exactly how the settle stayed
      dead for three revisions.
      FIX: the settle is PERFORMED AND MEASURED INSIDE THE SAME PYTHON PROCESS as the walk,
      timed by the same monotonic clock, and REPORTED (settled_ms, now a CSV column). The
      pair VOIDS if it did not elapse. There is no shell sleep left to shadow and no exit
      status left to discard. SELFTEST now ASSERTS it: measured 260ms on both hosts.
    
    - SELFTEST classified gates by GREPPING THEIR PROSE, and timer_gate's "returned nothing
      — refusing" did not match the regex — so A BLIND MEASURAND CLOCK scored [FIRED] and
      the self-test PASSED (grok). The classifier was the same fail-open it exists to hunt.
      FIX: blindness is marked EXPLICITLY (die_blind -> FATAL[PROBE-BLIND]), never inferred
      from wording. 10 sites. DRAIN-TIMEOUT (busy disk) is now [FIRED]; DRAIN-ERROR and an
      unreadable end-load are [BROKEN].
    
    Also accepted: resolve_disk fell through to the SYNTHESIZED APFS disk on an exit-ZERO
    but empty diskutil (whose counters can read idle while the physical store saturates);
    the ssh dispatch bound was measured and never ENFORCED (a measured bound that is not
    enforced is a note, not a protection — now a gate at SETTLE_MS/4); the escalation was
    authorised by any directory containing the right words (now it must carry runs.csv,
    meta.csv and a manifest on the registered build, and the burn is bound to the runs.csv
    HASH, so copying the session cannot buy a second re-roll); the engine graded happily
    with NO controls at all; and the docstring still described the rule three revisions back.
    
    Guard: 27 cases, 18/18 mutations KILLED, 300-input fuzz. Five mutations went STALE again
    when the engine was restructured — the stale-detector caught every one — and re-aiming
    them surfaced something worth recording: the bar clause in certification is PROVABLY
    REDUNDANT at n=8 (a CI inside +-half-margin bounds the median shift to <=5%) but
    LOAD-BEARING at n=16, where three outliers move the marginal median while the CI stays
    tight. It has a case now.
    
    Prereg -> rev 7.
    
    Six rounds. 69 findings. 69 accepted. 0 rejected. Still not one datum taken.
    
    Records: .review/results/macmac-harness-r6.{codex,grok}.md
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
    Claude-Session: https://claude.ai/code/session_01BAcgnhwAsA3eN86n597PqB

diff --git a/.review/results/macmac-harness-r6.codex.md b/.review/results/macmac-harness-r6.codex.md
new file mode 100644
index 0000000..942b223
--- /dev/null
+++ b/.review/results/macmac-harness-r6.codex.md
@@ -0,0 +1,274 @@
+Reading additional input from stdin...
+OpenAI Codex v0.144.3
+--------
+workdir: /Users/michael/Dev/blit_v2
+model: gpt-5.6-sol
+provider: openai
+approval: never
+sandbox: read-only
+reasoning effort: xhigh
+reasoning summaries: none
+session id: 019f60d2-5cc7-7af0-9d81-ee25612a476b
+--------
+user
+Review the Mac<->Mac benchmark instrument at HEAD (commit aebd50b). Run: git show aebd50b, and read the four files at HEAD.
+
+NO DATA HAS EVER BEEN TAKEN. This instrument has now been through FIVE review rounds: 56 findings, 56 accepted, 0 rejected. It decides whether a performance finding (P1: destination-initiated TCP x mixed pays ~25-38%) reproduces on a macOS<->macOS rig. This project has RETRACTED THREE CLAIMS to harness bugs.
+
+Files:
+- scripts/bench_otp12pf_mac.sh        the harness
+- scripts/otp12pf_mac_verdict.py      the MECHANIZED DECISION RULE
+- scripts/otp12pf_mac_verdict_test.py guard test (22 cases) + mutation proof (15 mutations)
+- docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md   rev 6, the spec the code must implement
+
+THE QUESTION IS NOT "is this nice code". It is: CAN THIS INSTRUMENT PRODUCE A CONFIDENT, WRONG ANSWER?
+
+TWO DEFECT CLASSES HAVE RECURRED IN EVERY SINGLE ROUND. Assume BOTH are still present and hunt them:
+
+CLASS 1 — "FIXED THE BRANCH THAT WAS SHOWN, NOT THE CLASS." The same materiality bug escaped in THREE consecutive rounds, each time through a branch the previous fix had not covered:
+  r3: the equivalence margin was bar-tied on the MEASURAND -> fixed there;
+  r4: ...still bar-tied for CONTROLS via the PARTIAL label -> fixed there;
+  r5: ...still escaping via the UNDERPOWERED label (one zero pair demotes the cell) -> and separately, `bar == FAIL` had NO DIRECTION, so a +1ms effect at n=16 reported REPRODUCES.
+Also: a fail-open `pgrep` was fixed in the quiescence gate and left identical in the stale-daemon probe.
+Round 6 restructured these: direction/magnitude/equivalence are now three separate questions (sign test / CI / CI-vs-margin); the control rule is written as an OBLIGATION (contaminating? certified?) rather than a list of outcome labels; there is exactly one process probe. VERIFY THAT THE RESTRUCTURING IS ACTUALLY COMPLETE, and find the next branch it missed.
+
+CLASS 2 — "A FIX THAT NEVER EXECUTED." Round 5 discovered that SETTLE_MS — the equal pre-fsync window introduced specifically to neutralise a free-writeback artifact capable of MANUFACTURING a one-directional result — HAD NEVER RUN. The awk computing its duration sat in a command substitution with the wrong quoting, so it errored, `sleep` got an empty argument and failed, and its exit status was discarded. The pre-registration asserted that fix for THREE revisions while it was dead. `bash -n` sees nothing.
+HUNT FOR MORE OF THESE. Which other claimed protections do not actually execute, or execute but cannot fail? Check every gate, every guard, every void path, every sentinel/sed extraction, every `|| true`, every exit-status that gets discarded by a following command. Prefer RUNNING things over reading them.
+
+Also verify specifically:
+- the three-question split: sign test = DIRECTION, CI = MAGNITUDE, CI-vs-margin = EQUIVALENCE. Any place they are still tangled? Any input where the taxonomy gives a wrong or unreportable answer?
+- the control rule (contaminating -> RIG-VOID; uncertified -> blocks the NULL but not a REPRODUCTION). Can a dirty rig still produce a null? Can a GOOD rig be falsely voided?
+- the pinned constants (harness refuses if they are merely present in the env; engine refuses a mismatched DELTA_REF_MS). Any remaining way to retune the rule from outside?
+- the RUNS=16 escalation: it must name the prior session dir, the harness reads its session_verdict.txt, and it burns an ESCALATED marker. Still p-hackable?
+- SELFTEST: it now reports [OK]/[FIRED]/[BROKEN] and exits nonzero on BROKEN. Is that classification honest, or can a broken probe still be scored [FIRED] (or vice versa)?
+- the guard test: 22 cases, 15 mutations. Are the mutations faithful (do they revert the real fix)? Which fixes have NO mutation? The mutation harness now judges a kill by whether the CASE FAILS.
+- bash: quoting through ssh (printf %q + heredocs), pipefail, subshell state loss (a gate that sets a global run inside $() loses it), set -e interactions, the ABBA loop, void accounting, trap/cleanup.
+
+If you have shell access, USE IT: drive the verdict engine with synthetic sessions and try to force a wrong verdict, and try to make a gate pass while blind.
+
+Output a concise markdown findings list — each with file:line, severity (BLOCKER/HIGH/MEDIUM/LOW), rationale — then any reproductions with the actual commands and output, then a final VERDICT line: READY TO RUN or NOT READY TO RUN. Be concise; do not invoke skills.
+codex
+I’ll treat this as an adversarial instrument audit, not a style review. I’ll first re-ground on the repo rules and exact commit state, then execute the verdict/guard machinery and targeted synthetic and shell-failure probes, with special attention to branch completeness and protections that can fail invisibly.
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+codex
+The requested target is exactly local HEAD (`aebd50b`), but the canonical remote currently points at `fa03258`; I’m keeping the audit pinned to the user-specified commit. The worktree also has two untracked review-result files, which I will not modify or treat as authority.
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+codex
+A concrete branch-completeness failure is emerging in the decision rule: although direction is now tied to the bar’s sign, `bar_fail_pos`/`bar_fail_neg` still bypass the CI magnitude test. I’m reproducing whether a marginal-median bar flip plus a tiny paired effect can still become `REPRODUCES`, then I’ll check whether the guard/mutations can see it.
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+codex
+The official suite is green (22/22, 15/15), but it does not cover the marginal-bar/paired-CI disagreement: a synthetic n=8 session with paired median `D=+1 ms` was labeled `REPRODUCES` solely because one outlier moved the marginal median. I’m now probing the escalation evidence and self-test classifications for similarly “present but not actually enforced” protections.
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+codex
+The control obligation is structurally complete for the null branch—an uncertified control cannot reach `VANISHES`—but it creates a separate false-positive path: `REPRODUCES` ignores uncertified controls entirely. I reproduced a headline reproduction while all four controls showed `D=+230 ms`, 7/7 nonzero positives, and CI `[0,230]`; the verdict text emitted no control caveat.
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+mcp: ptk/ptk_invoke started
+mcp: ptk/ptk_invoke (completed)
+codex
+## Findings
+
+- [scripts/otp12pf_mac_verdict.py:299](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:299) — **BLOCKER** — The three-way split is incomplete. `material = bar_fail_pos OR CI_lo >= breach_hi`, so a marginal-median bar failure still bypasses the CI magnitude test. At n=16, paired `D=1 ms`, CI `[1,1]`, and sign 16/16 produces `REPRODUCES` after three outliers move the marginal median. The same defect makes a control simultaneously “certified clean” (`null_excl=yes`) and “contaminating” via [line 418](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:418), falsely yielding `RIG-VOID`. The preregistration itself embeds this contradiction at [PREREGISTRATION.md:227](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:227).
+
+- [scripts/otp12pf_mac_verdict.py:433](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:433) — **BLOCKER** — Uncertified controls block only `VANISHES`; the `REPRODUCES` branch at [line 485](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:485) ignores them and emits no control caveat. With every control showing `D=+230 ms`, CI `[0,230]`, and 7/7 nonzero positives, the engine still confidently declares P1 reproduced. Uncertainty about a rig-wide confound is not evidence that the confound is absent.
+
+- [scripts/bench_otp12pf_mac.sh:693](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:693) — **BLOCKER** — The settle protection can again be dead while SELFTEST reports `[OK]`. `sleep` is PATH/function-resolved; the reported `ms` starts only after it at [line 700](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:700), while SELFTEST checks only file counts at [lines 874–880](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:874). A successful no-op `sleep` passed in 44 ms while claiming a 250 ms settle was included. Timed preflight does not require or run this duration proof.
+
+- [scripts/bench_otp12pf_mac.sh:413](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:413) — **HIGH** — `resolve_disk` catches a nonzero `diskutil`, but an exit-zero empty/unparseable response is treated as “not APFS” and falls back to the synthesized `df` device. The actual function accepted `UNPARSEABLE-BUT-EXIT-ZERO`, logged the device as physical, and returned success. This can certify the wrong disk as quiet.
+
+- [scripts/bench_otp12pf_mac.sh:495](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:495) — **HIGH** — RUNS=16 remains p-hackable. Any directory containing the expected first verdict line authorizes escalation; provenance, hashes, build, and prior `runs=8` are never checked. Moreover, only the source session gets `ESCALATED`; the resulting n=16 session remains eligible to authorize another n=16 rerun. A prior manifest explicitly saying `runs=16` was accepted.
+
+- [scripts/bench_otp12pf_mac.sh:848](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:848) — **MEDIUM** — SELFTEST’s `[FIRED]`/`[BROKEN]` classification is substring-based and dishonest for uncovered wording. The real timer error “probe returned nothing” was scored `[FIRED]`; an unreadable end-load is unconditionally scored `[FIRED]` at [line 922](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:922). SELFTEST can therefore exit zero with blind probes, although the corresponding runtime gates themselves fail closed.
+
+- [scripts/otp12pf_mac_verdict_test.py:303](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:303) — **MEDIUM** — The 15 mutations run and are honestly killed by the case assertion, but the proof is incomplete. Seven of 22 cases are never selected by a mutation. There is no faithful mutation for the exact n=8/n=16 CI selection/coverage, the r1 range rule, corrupt-row refusal, or the “tiny control must not void” protection. The mutation titled `UNSTABLE/BAR-FAIL-INCONSISTENT` restores only BAR-FAIL precedence, not UNSTABLE precedence. The preregistration still falsely reports “17 cases, 11 mutations, each mutation-proven” at [PREREGISTRATION.md:214](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:214).
+
+## Reproductions
+
+Official checks:
+
+```text
+$ bash -n scripts/bench_otp12pf_mac.sh
+$ python3 scripts/otp12pf_mac_verdict_test.py
+22/22 cases passed
+$ python3 scripts/otp12pf_mac_verdict_test.py --mutations
+15/15 mutations killed
+```
+
+False reproduction from a bar/CI disagreement:
+
+```text
+src = [1000]*8 + [2000]*8
+d   = [4001]*3 + [1]*13
+pairs = 16
+→ REPRODUCES
+```
+
+Here `src_med=1000`, `dest_med=2001`, but paired `D=1` and the registered CI is `[1,1]`.
+
+Dirty controls:
+
+```text
+measurand: [230]*8 @ src=1000
+every control: [0] + [230]*7 @ src=2500
+
+→ SESSION VERDICT: REPRODUCES
+→ controls: PARTIAL, D=+230, CI=[0,+230], sign_p=.0156
+```
+
+Dead settle accepted by the actual extracted functions:
+
+```text
+$ sleep(){ return 0; }; export -f sleep
+$ selftest_fsync n
+[OK] fsync/settle — walked 2 files/6 bytes in 0ms (settle 250ms included...)
+wall_ms=44
+BROKEN=0
+```
+
+Blind disk resolution:
+
+```text
+diskutil → "UNPARSEABLE-BUT-EXIT-ZERO", exit 0
+iostat disk9 → 0.0
+
+drain device on nagatha: disk9 (... idle probe 0.0 MB/s)
+resolve_disk rc=0 N_DISK=disk9
+```
+
+Escalation:
+
+```text
+prior/session_verdict.txt: SESSION VERDICT: INCONCLUSIVE-UNDERPOWERED
+prior/staging-manifest.txt: # ... runs=16
+
+escalation: RUNS=16, triggered by .../prior (verified INCONCLUSIVE-UNDERPOWERED)
+```
+
+The listed constant refusal and mismatched `DELTA_REF_MS` refusal worked. The exact CI coverages, sign test, single `pgrep_state` probe, ABBA pairing, and formal dirty-control→null blocking also checked out.
+
+**VERDICT: NOT READY TO RUN**
+tokens used
+172,154
+## Findings
+
+- [scripts/otp12pf_mac_verdict.py:299](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:299) — **BLOCKER** — The three-way split is incomplete. `material = bar_fail_pos OR CI_lo >= breach_hi`, so a marginal-median bar failure still bypasses the CI magnitude test. At n=16, paired `D=1 ms`, CI `[1,1]`, and sign 16/16 produces `REPRODUCES` after three outliers move the marginal median. The same defect makes a control simultaneously “certified clean” (`null_excl=yes`) and “contaminating” via [line 418](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:418), falsely yielding `RIG-VOID`. The preregistration itself embeds this contradiction at [PREREGISTRATION.md:227](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:227).
+
+- [scripts/otp12pf_mac_verdict.py:433](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:433) — **BLOCKER** — Uncertified controls block only `VANISHES`; the `REPRODUCES` branch at [line 485](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:485) ignores them and emits no control caveat. With every control showing `D=+230 ms`, CI `[0,230]`, and 7/7 nonzero positives, the engine still confidently declares P1 reproduced. Uncertainty about a rig-wide confound is not evidence that the confound is absent.
+
+- [scripts/bench_otp12pf_mac.sh:693](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:693) — **BLOCKER** — The settle protection can again be dead while SELFTEST reports `[OK]`. `sleep` is PATH/function-resolved; the reported `ms` starts only after it at [line 700](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:700), while SELFTEST checks only file counts at [lines 874–880](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:874). A successful no-op `sleep` passed in 44 ms while claiming a 250 ms settle was included. Timed preflight does not require or run this duration proof.
+
+- [scripts/bench_otp12pf_mac.sh:413](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:413) — **HIGH** — `resolve_disk` catches a nonzero `diskutil`, but an exit-zero empty/unparseable response is treated as “not APFS” and falls back to the synthesized `df` device. The actual function accepted `UNPARSEABLE-BUT-EXIT-ZERO`, logged the device as physical, and returned success. This can certify the wrong disk as quiet.
+
+- [scripts/bench_otp12pf_mac.sh:495](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:495) — **HIGH** — RUNS=16 remains p-hackable. Any directory containing the expected first verdict line authorizes escalation; provenance, hashes, build, and prior `runs=8` are never checked. Moreover, only the source session gets `ESCALATED`; the resulting n=16 session remains eligible to authorize another n=16 rerun. A prior manifest explicitly saying `runs=16` was accepted.
+
+- [scripts/bench_otp12pf_mac.sh:848](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:848) — **MEDIUM** — SELFTEST’s `[FIRED]`/`[BROKEN]` classification is substring-based and dishonest for uncovered wording. The real timer error “probe returned nothing” was scored `[FIRED]`; an unreadable end-load is unconditionally scored `[FIRED]` at [line 922](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:922). SELFTEST can therefore exit zero with blind probes, although the corresponding runtime gates themselves fail closed.
+
+- [scripts/otp12pf_mac_verdict_test.py:303](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:303) — **MEDIUM** — The 15 mutations run and are honestly killed by the case assertion, but the proof is incomplete. Seven of 22 cases are never selected by a mutation. There is no faithful mutation for the exact n=8/n=16 CI selection/coverage, the r1 range rule, corrupt-row refusal, or the “tiny control must not void” protection. The mutation titled `UNSTABLE/BAR-FAIL-INCONSISTENT` restores only BAR-FAIL precedence, not UNSTABLE precedence. The preregistration still falsely reports “17 cases, 11 mutations, each mutation-proven” at [PREREGISTRATION.md:214](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:214).
+
+## Reproductions
+
+Official checks:
+
+```text
+$ bash -n scripts/bench_otp12pf_mac.sh
+$ python3 scripts/otp12pf_mac_verdict_test.py
+22/22 cases passed
+$ python3 scripts/otp12pf_mac_verdict_test.py --mutations
+15/15 mutations killed
+```
+
+False reproduction from a bar/CI disagreement:
+
+```text
+src = [1000]*8 + [2000]*8
+d   = [4001]*3 + [1]*13
+pairs = 16
+→ REPRODUCES
+```
+
+Here `src_med=1000`, `dest_med=2001`, but paired `D=1` and the registered CI is `[1,1]`.
+
+Dirty controls:
+
+```text
+measurand: [230]*8 @ src=1000
+every control: [0] + [230]*7 @ src=2500
+
+→ SESSION VERDICT: REPRODUCES
+→ controls: PARTIAL, D=+230, CI=[0,+230], sign_p=.0156
+```
+
+Dead settle accepted by the actual extracted functions:
+
+```text
+$ sleep(){ return 0; }; export -f sleep
+$ selftest_fsync n
+[OK] fsync/settle — walked 2 files/6 bytes in 0ms (settle 250ms included...)
+wall_ms=44
+BROKEN=0
+```
+
+Blind disk resolution:
+
+```text
+diskutil → "UNPARSEABLE-BUT-EXIT-ZERO", exit 0
+iostat disk9 → 0.0
+
+drain device on nagatha: disk9 (... idle probe 0.0 MB/s)
+resolve_disk rc=0 N_DISK=disk9
+```
+
+Escalation:
+
+```text
+prior/session_verdict.txt: SESSION VERDICT: INCONCLUSIVE-UNDERPOWERED
+prior/staging-manifest.txt: # ... runs=16
+
+escalation: RUNS=16, triggered by .../prior (verified INCONCLUSIVE-UNDERPOWERED)
+```
+
+The listed constant refusal and mismatched `DELTA_REF_MS` refusal worked. The exact CI coverages, sign test, single `pgrep_state` probe, ABBA pairing, and formal dirty-control→null blocking also checked out.
+
+**VERDICT: NOT READY TO RUN**
diff --git a/.review/results/macmac-harness-r6.grok.md b/.review/results/macmac-harness-r6.grok.md
new file mode 100644
index 0000000..ab94e32
--- /dev/null
+++ b/.review/results/macmac-harness-r6.grok.md
@@ -0,0 +1,113 @@
+I'll review the instrument for confident-wrong-answer risk: pull the commit, read the four files at HEAD, then hunt both defect classes with shell where possible.I'll run the guard suite and drive the engine with synthetic sessions to try forcing a wrong verdict, while probing gates for fail-open paths.Hunting edge cases next: control-rule gaps, selftest misclassification, escalation forgery, and dead settle proof.Probing bash gates, selftest classification, escalation, and other dead paths with real execution:# Mac↔Mac instrument review (HEAD `aebd50b`)
+
+Guard suite: **22/22 cases, 15/15 mutations, 300/300 fuzz** — all green. That does not mean the instrument cannot emit a confident wrong answer.
+
+---
+
+## Findings
+
+### F1 — BLOCKER (Class 1): open-interval margin certifies ~Δ_ref control effects → `VANISHES`
+**Where:** `otp12pf_mac_verdict.py:303,416-435,520-544`  
+**What:** Contaminating uses `ci_lo >= margin_hi`; certification uses `ci_hi < margin_hi`. At `D=+229` (Δ_ref−1) on a slow arm, controls are PARTIAL, **null_excl=yes**, **certified**, session **`VANISHES`**, prose says “every control is CERTIFIED clean.” At `D=+230` correctly `RIG-VOID`. Same headline shape as r4/r5 (null while controls carry ~full reference effect), one millisecond under the post-fix boundary.  
+**Worse:** at n=16, `control_d=[229]*10+[0]*6` → CI `[0,229]` still null_excl → `VANISHES`.
+
+### F2 — BLOCKER (Class 2): SELFTEST “proves settle” without checking settle ran
+**Where:** `bench_otp12pf_mac.sh:869-880`; claim in `PREREGISTRATION.md:18-21,432-433`  
+**What:** `selftest_fsync` only checks `files==2` and `bytes==6`. **`ms=0` still scores `[OK]`.** A dead settle (the r5 disaster class) passes the proof that was added specifically to catch it. Log line “settle included” is narrative, not an assert.
+
+### F3 — HIGH (Class 2): blind timer probe scored `[FIRED]`, selftest can exit 0
+**Where:** `bench_otp12pf_mac.sh:259,848-860`  
+**What:** `timer_gate` die text *“returned nothing — refusing”* does **not** match the BROKEN regex (`cannot (read|…)|BROKE|…|refusing \(a gate`). Blind timer → **`[FIRED]`** → does not increment `SELFTEST_BROKEN` → **SELFTEST can PASS while the measurand clock probe is blind.** Same fail-open the selftest claims to hunt.
+
+### F4 — HIGH (Class 1 / completeness): obligation rewrite still label-adjacent at the margin edge
+**Where:** `otp12pf_mac_verdict.py:416-426` vs `429-430`  
+**What:** Restructuring is real for *labels* (no more void-if-in-{PARTIAL,UNDERPOWERED,…}). The next hole is the **open vs closed margin cut**: contaminating = “at or beyond”; certified = “strictly inside.” No obligation asks “is the control carrying a near-margin, sign-significant arm asymmetry that makes a null uninterpretable?” — TOST alone answers, and answers `VANISHES` at 229.
+
+### F5 — MEDIUM (Class 2): escalation still operator-forgeable
+**Where:** `bench_otp12pf_mac.sh:495-505,956-958`  
+**What:** Gate only checks `session_verdict.txt` text + absence of `ESCALATED`. A one-line forged dir, or a **copy** of a prior session without the marker, re-authorizes `RUNS=16`. Burn is path-local, not content-bound. Better than a bare flag; still a re-roll surface.
+
+### F6 — MEDIUM: SSH RTT is measured, never enforced
+**Where:** `bench_otp12pf_mac.sh:455-472`  
+**What:** Residual free-writeback asymmetry is *bounded by* RTT in the doc; if mux/RTT is large, residual can approach or exceed `SETTLE_MS` with **no refuse**. Only non-numeric RTT dies. Protection is observational, not a gate.
+
+### F7 — MEDIUM: end-load / drain selftest honesty
+**Where:** `bench_otp12pf_mac.sh:916-924`  
+**What:** Unreadable end-load voids the session in production (good) but selftest always scores it **`[FIRED]`**, never BROKEN. `DRAIN-TIMEOUT` (disk busy) scored **`[BROKEN]`** (wrong class; fail-closed for the sweep). Classification still not trustworthy.
+
+### F8 — MEDIUM: guard mutations do not cover harness Class-2 fixes
+**Where:** `otp12pf_mac_verdict_test.py` only  
+**What:** 15 mutations are engine-only and mostly faithful (stale detection works; kill = case fails). **No mutation** for settle execution, `pgrep_state`, escalation, selftest taxonomy, SSH RTT, drain rc. Engine green ≠ harness protections execute. Cases without mutations include false-void (“tiny control must NOT void”), INVERSION, codex r1 190ms, clean `VANISHES`.
+
+### F9 — LOW: doc / docstring drift
+**Where:** `otp12pf_mac_verdict.py:63-66` still documents `pos_effect = CI_lo > 0 ∧ sign_p`; `PREREGISTRATION.md:216-218` still says **17 cases / 11 mutations** while also mentioning 22/15 elsewhere. Misleading under pressure; not a runtime path.
+
+### F10 — LOW: engine callable without controls → `VANISHES`
+**Where:** engine env `CONTROL_CELLS` / `REGISTERED_CELLS`  
+**What:** Standalone engine with empty controls emits `VANISHES`. Harness assigns cell sets as literals (not env-defaults) — OK for the intended entrypoint; still a footgun if the engine is invoked alone.
+
+---
+
+## What looks solid (verified)
+
+| Area | Result |
+|------|--------|
+| Three-question cell taxonomy | Direction = sign test; magnitude = CI/bar-with-direction; equivalence = CI vs margin. r5 cases (inverse bar +1ms, 0+99×7, uncertified controls) held under drive |
+| Control obligation (non-boundary) | Contaminating → `RIG-VOID`; uncertified → blocks null only; tiny +5ms → `VANISHES` not void; repro survives uncertified control |
+| Pinned constants | Harness refuses if listed vars **present**; engine refuses mismatched `DELTA_REF_MS`; matching `230` also refused by harness (presence) |
+| Single `pgrep_state` | Only process probe; quiescence + stale-daemon both use it |
+| SETTLE computation | Top-level `SETTLE_SEC=0.250` sleeps ~250ms; old `\"` awk path still fails if revived; sleep failure → `F:NA` → pair void |
+| Guard suite | 22/22, 15/15 killed, fuzz clean |
+
+---
+
+## Reproductions
+
+```bash
+# F1: Δ_ref-1 certifies; Δ_ref voids
+python3 - <<'PY'
+import sys; sys.path.insert(0,'scripts')
+from otp12pf_mac_verdict_test import session
+null=[-4,-2,-1,0,0,1,2,3]
+print('229', session(measurand_d=null, src=2000, control_d=[229]*8, control_src=2500))
+print('230', session(measurand_d=null, src=2000, control_d=[230]*8, control_src=2500))
+print('7x229+0', session(measurand_d=null, src=2000, control_d=[0]+[229]*7, control_src=2500))
+print('7x230+0', session(measurand_d=null, src=2000, control_d=[0]+[230]*7, control_src=2500))
+print('n16 CI~[0,229]', session(measurand_d=null*2, src=2000,
+    control_d=[229]*10+[0]*6, control_src=2500, pairs=16))
+PY
+# → 229 VANISHES | 230 RIG-VOID | 7x229+0 VANISHES | 7x230+0 INCONCLUSIVE-UNDERPOWERED | n16 VANISHES
+```
+
+```bash
+# F2: selftest accepts zero-duration walk
+# (selftest_fsync condition only)
+ms=0 files=2 bytes=6
+# → would log [OK] fsync/settle — settle duration never asserted
+```
+
+```bash
+# F3: blind timer → FIRED not BROKEN
+err='nagatha: the timer probe returned nothing — refusing'
+grep -qiE 'cannot (read|sample|probe|measure|resolve|answer)|BROKE|did not answer|no sentinel|refusing \(a gate' <<<"$err" \
+  && echo BROKEN || echo FIRED
+# → FIRED
+```
+
+```bash
+# F5: escalation content gate accepts forgery
+tmp=$(mktemp -d)
+echo 'SESSION VERDICT: INCONCLUSIVE-UNDERPOWERED' > "$tmp/session_verdict.txt"
+# UNDERPOWERED_ESCALATION=$tmp RUNS=16 … would pass the verdict-text check
+```
+
+---
+
+## VERDICT: **NOT READY TO RUN**
+
+Two independent confident-wrong / confident-blind paths remain:
+
+1. **Decision rule:** `VANISHES` + “CERTIFIED clean” with controls at **+229 ms** (and n=16 variants with CI up to 229) — Class 1’s next branch after the obligation rewrite.  
+2. **Selftest:** settle “proof” is vacuous and blind timer is mis-scored `[FIRED]` — Class 2 exactly as in rounds that shipped dead protections under green ceremony.
+
+Clear F1–F3 (and preferably F5–F6) before any timed run; do not treat 22/15 green as clearance.
diff --git a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
index 2e2e283..a2b3eb0 100644
--- a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
+++ b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
@@ -1,6 +1,18 @@
 # otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
 
-**Status**: Pre-registered, **revision 6**. **NO DATA EXISTS YET.**
+**Status**: Pre-registered, **revision 7**. **NO DATA EXISTS YET.**
+
+> ## THE RULE IN ONE PARAGRAPH (rev 7)
+>
+> **All inference is PAIRED.** The 1.10 bar is computed on the *marginal medians*; it is
+> the project's **acceptance** criterion, it is reported in every row, and it takes **no
+> part in inference** — because the marginal and paired statistics can disagree in both
+> direction and magnitude, and every attempt to let the bar stand in for paired evidence
+> produced a false verdict (rounds 3–6). **Direction** is the sign test. **Magnitude** is
+> the paired CI. **Equivalence** is the CI against the margin. **The controls are a
+> precondition**: unless every control is certified to carry less than *half* the
+> material effect, **no verdict about the measurand may be read — not a null, and not a
+> reproduction.**
 
 > ## ⛔ CORRECTION THAT THIS DOCUMENT OWES ITS READER
 >
@@ -36,6 +48,14 @@ measured anything, and **every review has found defects capable of a false claim
   **drove the engine to a clean `VANISHES` while every control carried the full
   rig-W effect** → **9 findings, 9 accepted** (1 BLOCKER, 3 HIGH, 4 MEDIUM, 1 LOW).
   (`.review/results/macmac-harness-r3.grok-verdict.md`)
+- Round 6 (the round-5 rework, `aebd50b`): **NOT READY** — **codex** (3 BLOCKER) **and
+  grok** (2 BLOCKER), converging *again* on both hunted classes: the **marginal bar still
+  substituted for paired magnitude** (a **1 ms** paired effect reported `REPRODUCES` at
+  n=16), a control at **D=+229** — *one millisecond* under the reference effect —
+  **certified as clean**, uncertified controls **blocked only the null and not a
+  reproduction**, and the settle repair was **still not provable** (a no-op `sleep` would
+  have passed while the log narrated "settle included"). → **13 findings, 13 accepted.**
+  (`.review/results/macmac-harness-r6.{codex,grok}.md`)
 - Round 5 (the round-4 rework, `a9460ce`): **NOT READY / NOT SAFE TO RUN** — **codex**
   (3 BLOCKER, 6 HIGH, 2 MEDIUM) **and grok**, which converged on the **same BLOCKER
   independently**: the materiality bug, **for the third round running**, in a branch
@@ -43,7 +63,7 @@ measured anything, and **every review has found defects capable of a false claim
   (above), which the review's finding exposed but did not itself find.
   (`.review/results/macmac-harness-r5.verdict.md`)
 
-**Five rounds. 56 findings. 56 accepted. 0 rejected. Still no datum taken** — which is
+**Six rounds. 69 findings. 69 accepted. 0 rejected. Still no datum taken** — which is
 the only reason none of it became a retraction.
 
 **The rule below has been amended in rev 4 and again in rev 5. That is legitimate
diff --git a/scripts/bench_otp12pf_mac.sh b/scripts/bench_otp12pf_mac.sh
index b0b9b64..ce81e61 100755
--- a/scripts/bench_otp12pf_mac.sh
+++ b/scripts/bench_otp12pf_mac.sh
@@ -188,6 +188,7 @@ CELLS="$REGISTERED_CELLS"
 SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
 OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
 ESCALATED_FROM=""          # set only by the verified RUNS=16 escalation
+PRIOR_RUNS_SHA=""          # the data hash the escalation is bound to
 
 MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
 SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
@@ -197,6 +198,10 @@ qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
 mkdir -p "$OUT_DIR/blit-logs"
 log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
 die() { log "FATAL: $*"; exit 1; }
+# A gate that CANNOT ANSWER is BLIND, and blindness is what fails open on the night.
+# It is marked EXPLICITLY here, never inferred from the wording of a message —
+# inferring it from prose is how a blind timer came to be scored as a working gate.
+die_blind() { log "FATAL[PROBE-BLIND]: $*"; exit 1; }
 nocr() { tr -d '\r'; }
 
 # --- host abstraction: $1 = n (local) | q (remote) -----------------------------
@@ -256,9 +261,9 @@ PYEOF" | nocr | sed -n 's/.*R:\(-\{0,1\}[0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | he
 timer_gate() {
   local h="$1" out ms rc lo hi
   out="$(time_argv "$h" /bin/sleep 1)"
-  [[ "$out" == *,* ]] || die "$(hname "$h"): the timer probe returned nothing — refusing"
+  [[ "$out" == *,* ]] || die_blind "$(hname "$h"): the timer probe returned nothing — refusing"
   ms="${out%%,*}"; rc="${out##*,}"
-  [[ "$rc" == 0 ]] || die "$(hname "$h"): the timer probe's own child exited $rc"
+  [[ "$rc" == 0 ]] || die_blind "$(hname "$h"): the timer probe's own child exited $rc"
   lo=$(( 1000 - TIMER_TOLERANCE_MS )); hi=$(( 1000 + TIMER_TOLERANCE_MS ))
   if (( ms < lo || ms > hi )); then
     die "$(hname "$h"): THE TIMER IS LYING — a 1000 ms sleep measured ${ms} ms (allowed ${lo}-${hi}).
@@ -339,7 +344,7 @@ quiescence_gate() {
     case "$(pgrep_state "$h" "$p")" in
       RUNNING) busy="$busy $p" ;;
       NONE)    : ;;
-      *)       die "$(hname "$h"): the quiescence probe for '$p' BROKE — refusing (a gate that cannot answer must not answer 'fine')" ;;
+      *)       die_blind "$(hname "$h"): the quiescence probe for '$p' BROKE — refusing (a gate that cannot answer must not answer 'fine')" ;;
     esac
   done
   [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running:$busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
@@ -348,10 +353,10 @@ quiescence_gate() {
 timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   local h="$1" running auto
   running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
-  [[ "$running" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
+  [[ "$running" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
   [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
   auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1" | nocr | tr -cd '0-9')" || auto=""
-  [[ "$auto" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
+  [[ "$auto" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
   [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
 }
 
@@ -361,7 +366,7 @@ spotlight_gate() {
   # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
   cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
     | awk '/^mds_stores/{ if (\$2+0 > m) m = \$2+0 } END{ if (NR == 0) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
-  [[ "$cpu" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
+  [[ "$cpu" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
   [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
 }
 
@@ -369,7 +374,7 @@ load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
 load_gate() {
   local h="$1" l ok
   l="$(load1 "$h")" || l=""
-  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die "$(hname "$h"): cannot read load1 (got '$l') — refusing"
+  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die_blind "$(hname "$h"): cannot read load1 (got '$l') — refusing"
   ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
 }
@@ -377,7 +382,7 @@ load_gate() {
 link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
   local h="$1" o peer_ip want got route_nic nic
   o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"; nic="$(hnic "$h")"
-  [[ -n "$want" ]] || die "$(hname "$o"): its configured MAC does not parse — refusing"
+  [[ -n "$want" ]] || die_blind "$(hname "$o"): its configured MAC does not parse — refusing"
   hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
     || die "$(hname "$h") cannot ping $peer_ip — the link is down"
   # The ARP entry ON THE NIC THE TRAFFIC WILL EGRESS. `arp -n <ip>` prints one line
@@ -413,6 +418,7 @@ resolve_disk() {
   dev="$(hrun "$h" "d=\$(df '$p' 2>/dev/null | awk 'NR==2{print \$1}' | sed 's|^/dev/||')
 [ -n \"\$d\" ] || { echo 'D:NO-DF:D'; exit 0; }
 info=\$(diskutil info \"\$d\" 2>/dev/null) || { echo 'D:NO-DISKUTIL:D'; exit 0; }
+[ -n \"\$info\" ] || { echo 'D:EMPTY-DISKUTIL:D'; exit 0; }
 if echo \"\$info\" | grep -q 'APFS'; then
   ps=\$(echo \"\$info\" | awk -F: '/APFS Physical Store/{gsub(/[ \t]/, \"\", \$2); print \$2}' | head -1)
   [ -n \"\$ps\" ] || { echo 'D:APFS-NO-STORE:D'; exit 0; }
@@ -468,8 +474,11 @@ for _ in range(5):
     ts.append((time.monotonic() - t) * 1000.0)
 print(int(statistics.median(ts)))
 ' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
-  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
-  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms — this BOUNDS the residual settle-gap asymmetry (the settle itself is ${SETTLE_MS} ms, EQUAL on both arms)"
+  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
+  local rtt_max=$(( SETTLE_MS / 4 ))
+  (( SSH_RTT_MS <= rtt_max )) \
+    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
+  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
 }
 
 # =============================================================================
@@ -495,15 +504,29 @@ preflight() {
     local prior="${UNDERPOWERED_ESCALATION:-}" v
     [[ -n "$prior" ]] \
       || die "RUNS=16 is the escalation arm. Set UNDERPOWERED_ESCALATION=<path to the prior session dir> that returned INCONCLUSIVE-UNDERPOWERED. It buys POWER; it is NOT a re-roll."
-    [[ -f "$prior/session_verdict.txt" ]] \
-      || die "UNDERPOWERED_ESCALATION='$prior' has no session_verdict.txt — the escalation must name a REAL prior session"
+    # The trigger must be a REAL SESSION, not a directory that merely contains the right
+    # words (round-6, codex HIGH + grok F5: "any directory containing the expected first
+    # verdict line authorizes escalation; provenance, hashes, build and prior runs=8 are
+    # never checked"). So the prior session must carry its own DATA and MANIFEST, and
+    # the escalation is bound to the CONTENT of that data, not to its path.
+    for _f in session_verdict.txt runs.csv meta.csv staging-manifest.txt; do
+      [[ -f "$prior/$_f" ]] \
+        || die "UNDERPOWERED_ESCALATION='$prior' has no $_f — the escalation must name a REAL prior session, not a directory with the right words in it"
+    done
     v="$(head -1 "$prior/session_verdict.txt" | sed -n 's/^SESSION VERDICT: *//p')"
     [[ "$v" == "INCONCLUSIVE-UNDERPOWERED" ]] \
       || die "the prior session '$prior' returned '$v', not INCONCLUSIVE-UNDERPOWERED. RUNS=16 is triggered by a POWER FAILURE and by nothing else — re-running a result you dislike at higher n is p-hacking, and this gate exists to stop it."
-    [[ ! -f "$prior/ESCALATED" ]] \
-      || die "the prior session '$prior' has ALREADY been escalated once (see $prior/ESCALATED). 'Once' means once."
+    grep -q "binary_identity=$REGISTERED_BUILD" "$prior/staging-manifest.txt" \
+      || die "the prior session '$prior' was not run on the registered build $REGISTERED_BUILD — it cannot authorise an escalation"
+    # "Once" is bound to the DATA, not the directory: copying the session elsewhere does
+    # not buy a second re-roll, because the burn records the runs.csv hash.
+    PRIOR_RUNS_SHA="$(shasum -a 256 "$prior/runs.csv" | cut -d' ' -f1)"
+    if [[ -f "$REPO_ROOT/logs/ESCALATED-SESSIONS" ]] \
+       && grep -q "$PRIOR_RUNS_SHA" "$REPO_ROOT/logs/ESCALATED-SESSIONS"; then
+      die "this exact session's data (runs.csv $PRIOR_RUNS_SHA) has ALREADY authorised an escalation — see logs/ESCALATED-SESSIONS. 'Once' means once, and it is bound to the DATA, not the path."
+    fi
     ESCALATED_FROM="$prior"
-    log "  escalation: RUNS=16, triggered by $prior (verified INCONCLUSIVE-UNDERPOWERED)"
+    log "  escalation: RUNS=16, triggered by $prior (verified INCONCLUSIVE-UNDERPOWERED, build $REGISTERED_BUILD, runs.csv $PRIOR_RUNS_SHA)"
   fi
   [[ "$EXPECT_SHA" == "$REGISTERED_BUILD" ]] \
     || die "EXPECT_SHA='$EXPECT_SHA' but the PRE-REGISTERED build is $REGISTERED_BUILD — a run against another build is not the registered experiment"
@@ -683,21 +706,33 @@ prep_run() {   # $1 = dest host
 }
 
 # --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
-RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0
-fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes" or "NA 0 0"
+RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
+fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
   local out
-  # THE SETTLE IS REQUIRED, SO ITS FAILURE MUST BE FATAL (round-5 codex, HIGH): the
-  # command status came from the python that followed, so a failed `sleep` was
-  # invisible and the row stayed VALID — with the direction-reversing free-writeback
-  # gap restored, which is the artifact the settle exists to equalize.
-  out="$(hrun "$1" "sleep $SETTLE_SEC || { echo 'F:NA:0:0:F'; exit 0; }
-python3 - '$2' <<'PYEOF'
+  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
+  #
+  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
+  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
+  # RAN. Round 6 then found the repair was still not provable: `sleep` is
+  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
+  # counted files — so a no-op `sleep` would pass while the log narrated "settle
+  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
+  #
+  # A protection that cannot be OBSERVED is not a protection. The settle now happens
+  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
+  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
+  # to shadow, no exit status left to discard, and no narration left to trust.
+  out="$(hrun "$1" "python3 - '$SETTLE_SEC' '$2' <<'PYEOF'
 import os, sys, time
-p = sys.argv[1]
+settle = float(sys.argv[1])
+p = sys.argv[2]
+t0 = time.monotonic()
+time.sleep(settle)
+settled_ms = int((time.monotonic() - t0) * 1000)
 if not os.path.isdir(p):
-    print('F:NA:0:0:F')          # a MISSING tree must never read as a fast flush
+    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
     raise SystemExit
-t = time.monotonic()             # ONE process: this interval is measured by one clock
+t = time.monotonic()
 files = 0
 nbytes = 0
 for root, _d, fs in os.walk(p):
@@ -708,10 +743,12 @@ for root, _d, fs in os.walk(p):
         os.fsync(fd)
         os.close(fd)
         files += 1
-print('F:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes))
-PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3/p' | head -1)" || out=""
-  echo "${out:-NA 0 0}"
+print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
+PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
+  echo "${out:-NA 0 0 0}"
 }
+# The settle actually elapsed, on the destination's own clock. Anything else voids.
+settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
 
 # --- one timed run ------------------------------------------------------------
 RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
@@ -721,9 +758,17 @@ timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=f
   prep_run "$dh"
   out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
   if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
-  read -r RUN_FLUSH RUN_FILES RUN_BYTES <<<"$(fsync_tree "$dh" "$landed")"
+  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
   RUN_VALID=yes
   wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
+  # The equal settle is the ONLY thing standing between this rig and a free-writeback
+  # artifact that REVERSES SIGN WITH DIRECTION — i.e. that can manufacture P1 out of
+  # nothing. It has already been silently dead once. If it did not measurably elapse,
+  # the row is not a fast row; it is a VOID row.
+  if ! settle_ok "$RUN_SETTLED"; then
+    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
+    RUN_VALID=no
+  fi
   if [[ "$RUN_FLUSH" == NA ]]; then
     log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
     RUN_VALID=no; RUN_FLUSH=0
@@ -777,7 +822,7 @@ run_pair_loop() {
       if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"
       else arm_destinit "$sh" "$dh" "$run"; fi
       [[ "$RUN_VALID" == yes ]] || pair=no
-      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
+      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
       if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
       log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
     done
@@ -851,7 +896,7 @@ gate_probe() {
   err="$( { "$@"; } 2>&1 )" || rc=1
   if (( rc == 0 )); then
     log "  [OK]     $label — answers, and the condition holds"
-  elif grep -qiE 'cannot (read|sample|probe|measure|resolve|answer)|BROKE|did not answer|no sentinel|refusing \(a gate' <<<"$err"; then
+  elif grep -q 'PROBE-BLIND' <<<"$err"; then
     SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 ))
     log "  [BROKEN] $label — THE PROBE COULD NOT ANSWER. A blind gate fails open on the night."
   else
@@ -867,17 +912,25 @@ gate_probe() {
 # measurement AND the equal-settle window — the two things that once manufactured P1 —
 # and the self-test never touched them.
 selftest_fsync() {
-  local h="$1" d out ms files bytes
+  local h="$1" d ms files bytes settled
   d="$(hmod "$h")/selftest_${SESSION_TAG}"
   hrun "$h" "rm -rf '$d' && mkdir -p '$d' && printf 'aaaa' > '$d/a' && printf 'bb' > '$d/b'" \
     || { log "  [BROKEN] fsync/settle — cannot stage a probe tree"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); return 1; }
-  read -r ms files bytes <<<"$(fsync_tree "$h" "$d")"
+  read -r ms files bytes settled <<<"$(fsync_tree "$h" "$d")"
   hrun "$h" "rm -rf '$d'" >/dev/null 2>&1 || true
   if [[ "$ms" == NA || "$files" != 2 || "$bytes" != 6 ]]; then
     log "  [BROKEN] fsync/settle — walk returned ms=$ms files=$files bytes=$bytes, want 2 files / 6 bytes"
     SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
   fi
-  log "  [OK]     fsync/settle — walked 2 files/6 bytes in ${ms}ms (settle ${SETTLE_MS}ms included, counts VERIFIED)"
+  # THE SETTLE MUST BE PROVED, NOT NARRATED (round-6, both reviewers). The old check
+  # counted files and then LOGGED "settle included" — which is a sentence, not an
+  # assertion. It would have passed with the settle stone dead, which is precisely how
+  # the settle stayed dead for three revisions.
+  if ! settle_ok "$settled"; then
+    log "  [BROKEN] fsync/settle — THE SETTLE DID NOT ELAPSE: measured ${settled}ms, want >= ${SETTLE_MS}ms"
+    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
+  fi
+  log "  [OK]     fsync/settle — 2 files/6 bytes walked in ${ms}ms; settle MEASURED at ${settled}ms (>= ${SETTLE_MS}ms), counts VERIFIED"
 }
 
 selftest() {
@@ -913,15 +966,26 @@ selftest() {
       RUNNING) log "  [FIRED]  stale daemon  (one IS running — the gate would refuse)"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
       *)       log "  [BROKEN] stale daemon  — the probe could not answer"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
     esac
+    # DRAIN-TIMEOUT is a genuinely busy disk (the gate WORKING); DRAIN-ERROR is a blind
+    # probe. Scoring them the same made the classification untrustworthy (grok r6, F7).
     local dr; dr="$(drain_host "$h")"
-    if [[ "$dr" == drained* ]]; then log "  [OK]     drain loop    ($dr)"
-    else log "  [BROKEN] drain loop    — returned '$dr'"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); fi
+    case "$dr" in
+      drained*)      log "  [OK]     drain loop    ($dr)" ;;
+      DRAIN-TIMEOUT) log "  [FIRED]  drain loop    — the disk is genuinely busy; the gate would void the pair"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
+      *)             log "  [BROKEN] drain loop    — the probe could not answer ('$dr')"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
+    esac
     selftest_fsync "$h"
     log "  [--]     mac parse (no gawk strtonum): $(hmac "$h") -> $(hmac "$h" | norm_mac)"
   done
   SESSION_VOID_REASON=""; end_load_gate
-  if [[ -z "$SESSION_VOID_REASON" ]]; then log "  [OK]     end-load gate (both Macs under $LOAD_MAX; it CAN void a session)"
-  else log "  [FIRED]  end-load gate — $SESSION_VOID_REASON"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)); fi
+  if [[ -z "$SESSION_VOID_REASON" ]]; then
+    log "  [OK]     end-load gate (both Macs under $LOAD_MAX; it CAN void a session)"
+  elif [[ "$SESSION_VOID_REASON" == *"could not be read"* ]]; then
+    # An UNREADABLE end-load is a blind probe, not a busy machine (grok r6, F7).
+    log "  [BROKEN] end-load gate — $SESSION_VOID_REASON"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1))
+  else
+    log "  [FIRED]  end-load gate — $SESSION_VOID_REASON"; SELFTEST_FIRED=$((SELFTEST_FIRED+1))
+  fi
   measure_ssh_rtt
   log ""
   log "SELFTEST: $SELFTEST_FIRED gate(s) refused a genuinely unmet condition; $SELFTEST_BROKEN blind."
@@ -956,9 +1020,11 @@ main() {
   if [[ -n "$ESCALATED_FROM" ]]; then
     echo "escalated to $SESSION_TAG (RUNS=$RUNS) on $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
       >> "$ESCALATED_FROM/ESCALATED"
+    # Bound to the DATA, so a copy of the session cannot buy a second re-roll.
+    echo "$PRIOR_RUNS_SHA $ESCALATED_FROM -> $SESSION_TAG" >> "$REPO_ROOT/logs/ESCALATED-SESSIONS"
   fi
   log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
-  echo "cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
+  echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
   echo "cell,pairs_attempted,complete" > "$META"
   daemon_start n; daemon_start q
   smoke n; smoke q
diff --git a/scripts/otp12pf_mac_verdict.py b/scripts/otp12pf_mac_verdict.py
index 92a06ae..193df7b 100644
--- a/scripts/otp12pf_mac_verdict.py
+++ b/scripts/otp12pf_mac_verdict.py
@@ -60,25 +60,37 @@ THE STATISTIC
                                                rig-W-sized effect however slow this
                                                rig's arms are.
 
+THE THREE QUESTIONS (rev 7) -- kept apart, because tangling them produced the SAME
+class of defect in rounds 3, 4, 5 AND 6. ALL INFERENCE IS PAIRED; the bar (marginal
+medians) is the project's ACCEPTANCE criterion and takes no part in inference.
+
+  DIRECTION   = the SIGN TEST      directional = sign_p < .05  (zeros dropped)
+  MAGNITUDE   = the paired CI      material     = CI_lo >= BREACH_HI
+                                   material_neg = CI_hi <= BREACH_LO
+  EQUIVALENCE = the CI vs MARGIN   null_excl    = CI strictly inside the margin
+
 PER-CELL OUTCOMES (exhaustive; no unreportable region)
-  pos_effect  = CI_lo > 0 and sign_p < .05         (a real destination-slower effect)
-  neg_effect  = CI_hi < 0 and sign_p < .05         (a real source-slower effect)
-  material    = bar FAILS or CI_lo >= BREACH_HI    (it reaches the 10% threshold)
-  material_neg= bar FAILS or CI_hi <= BREACH_LO
-  null_excl   = CI lies STRICTLY inside (MARGIN_LO, MARGIN_HI)
-
-  REPRODUCES            pos_effect and material
-  INVERSION             neg_effect and material_neg
-  PARTIAL               a real effect (either sign) that is NOT material
-  VANISHES              no effect AND null_excl -- a genuine EQUIVALENCE result
-  UNDERPOWERED          no effect and the CI cannot exclude the margin -> a PASS
+  REPRODUCES            dir_pos and material
+  INVERSION             dir_neg and material_neg
+  PARTIAL               a real direction whose magnitude is NOT material
+  VANISHES              no direction AND null_excl -- a genuine EQUIVALENCE result
+  UNDERPOWERED          no direction and the CI cannot exclude the margin -> a PASS
                         here is NOT "P1 vanishes"; the rig could not have seen it
-  BAR-FAIL-INCONSISTENT bar FAILS but the pairs establish NO consistent direction
-                        (the sign test does not reject). The medians
-                        breach 1.10 while the paired evidence contradicts itself
-                        (pf-0's bistability, in a new dress). NEVER a null.
+  BAR-FAIL-INCONSISTENT the bar FAILS but the pairs establish NO consistent direction
   UNSTABLE              (override) an arm is bimodal AND the bar flips on pooled runs
   INCOMPLETE            the cell did not finish its registered pairs
+
+THE CONTROLS ARE A PRECONDITION, NOT A FOOTNOTE
+  CONTAMINATING  a directional effect whose CI sits at/beyond the margin, or bimodal
+                 -> RIG-VOID. The rig is carrying the effect we came to measure.
+  CERTIFIED      bar PASSES and the paired CI lies strictly inside HALF the margin.
+                 Half, because certifying a control with the very threshold that
+                 DEFINES the effect is incoherent -- it would let a control carry all
+                 but 1 ms of P1 and still call the rig clean (round-6, grok).
+  otherwise      NOT CERTIFIED -> CONTROLS-UNCERTIFIED, and NO measurand verdict may
+                 be read: not a null, and NOT a reproduction either. Uncertainty about
+                 a rig-wide confound is not evidence that the confound is absent
+                 (round-6, codex).
 """
 import csv, os, sys
 from math import comb
@@ -112,6 +124,16 @@ def cells_env(name):
 
 VERDICT_CELLS = cells_env("VERDICT_CELLS")
 CONTROL_CELLS = cells_env("CONTROL_CELLS")
+# The controls are a PRECONDITION for reading any verdict, so an engine invoked
+# WITHOUT them cannot grade anything (round-6 grok, LOW: called standalone with no
+# controls it happily emitted VANISHES -- a footgun aimed at exactly the person who
+# would re-grade a CSV by hand).
+if not VERDICT_CELLS or not CONTROL_CELLS:
+    sys.stderr.write(
+        "REFUSING: VERDICT_CELLS and CONTROL_CELLS must both be set. The controls are "
+        "a precondition for any verdict -- an engine with no controls cannot certify "
+        "the rig, and must not pretend to.\n")
+    raise SystemExit(2)
 # The full registered set must be PRESENT and COMPLETE. A partial CELLS set that is
 # merely filtered lets a one-cell run emit VANISHES while claiming "both" cells
 # vanished (codex r2 BLOCKER 1).
@@ -293,15 +315,26 @@ with open(pair_p, "w") as f:
         # `VANISHES`, while the sign test REJECTED at p = .0156. Seven of eight pairs
         # carried a 99 ms effect, one millisecond under the bar, and it was called
         # equivalence. DIRECTION is the sign test's job, not the CI's.
-        directional = p < 0.05
+        # ALL INFERENCE IS PAIRED. The bar is computed on the MARGINAL medians; the CI
+        # on the PAIRED differences. They are different statistics and they can point
+        # OPPOSITE WAYS (round-5), or agree in direction while disagreeing wildly in
+        # magnitude (round-6). Rev 6 tried to fix that by making the bar failure
+        # direction-aware -- and codex promptly drove `material` again: at n=16 a
+        # paired D of ONE MILLISECOND (CI [1,1], 16/16 positive) still reported
+        # REPRODUCES, because three outliers moved the MARGINAL median enough to fail
+        # the bar in the matching direction, and `material` accepted a bar failure as
+        # a substitute for paired magnitude.
+        #
+        # So the bar no longer participates in INFERENCE AT ALL. It is the project's
+        # ACCEPTANCE criterion: it is computed, reported in every row, and used to
+        # judge a CELL against the 1.10 invariance bar -- but direction and magnitude
+        # are decided by the paired statistics, and by nothing else.
+        directional = p < 0.05                       # DIRECTION  -- the sign test
         dir_pos = directional and k > (n - k)
         dir_neg = directional and k < (n - k)
-        bar_fail_pos = (bar == "FAIL") and d_med > s_med     # the bar failed the SAME way
-        bar_fail_neg = (bar == "FAIL") and d_med < s_med
-        material = bar_fail_pos or (ci_lo >= breach_hi)
-        material_neg = bar_fail_neg or (ci_hi <= breach_lo)
-        null_excl = (ci_lo > margin_lo) and (ci_hi < margin_hi)
-        pos_effect, neg_effect = dir_pos, dir_neg
+        material = ci_lo >= breach_hi                # MAGNITUDE  -- the paired CI, only
+        material_neg = ci_hi <= breach_lo
+        null_excl = (ci_lo > margin_lo) and (ci_hi < margin_hi)   # EQUIVALENCE
 
         # UNSTABLE, as a STATISTIC not a vibe: an arm splits into two clusters
         # separated by more than the paired spread, AND the bar verdict flips when
@@ -413,28 +446,50 @@ verd = [c for c in VERDICT_CELLS if c in cell_outcome]
 #      one-directional effect in the measurand, and voiding real evidence on that
 #      basis would be its own false negative -- grok, round-5 NEW-5, which is why
 #      an unproven control blocks the null rather than killing the session.)
+# CONTAMINATING: the rig is CARRYING the effect we came to measure. Nothing here can
+# be trusted -> RIG-VOID. Paired evidence only (a marginal-median bar failure with
+# clean pairs is not contamination -- it made a control simultaneously "certified" and
+# "contaminating", a contradiction codex drove to a FALSE RIG-VOID).
 def _ctrl_contaminating(c):
     dt = cell_detail.get(c, {})
-    if dt.get("bar") == "FAIL" or cell_outcome[c] == "UNSTABLE":
+    if cell_outcome[c] == "UNSTABLE":
         return True
-    if cell_outcome[c] in ("REPRODUCES", "INVERSION"):
-        return True
-    # A directional effect whose magnitude reaches the margin: the rig itself is
-    # carrying the effect we are trying to measure.
-    if dt.get("directional") and dt.get("ci_at_or_beyond_margin"):
-        return True
-    return False
+    return bool(dt.get("directional") and dt.get("ci_at_or_beyond_margin"))
 
 
+# CERTIFIED CLEAN: and the threshold for a CONTROL must be STRICTLY TIGHTER than the
+# effect we claim to detect in the MEASURAND. Round-6 (grok, BLOCKER): certification
+# used the SAME margin as materiality, so a control carrying D = +229 ms -- ONE
+# MILLISECOND under the reference effect -- certified as "clean", and the session
+# printed VANISHES with the prose "every control is CERTIFIED clean". Certifying a
+# control with the very threshold that defines the effect is incoherent: it would let
+# us claim P1 is TCP-only while the gRPC control carries all but 1 ms of it.
+#
+# So a control must carry LESS THAN HALF the material effect. That is not an invented
+# number: it is the specificity claim itself, made checkable. P1 is asserted to be
+# TCP-only and mixed-only; if a control carries half the effect, that assertion is not
+# readable off this rig. (At src=2500 -> 115 ms; at src=1000 -> 50 ms; i.e. ~5% of the
+# arm, which is the rig noise measured on the q-baseline, 2-4%.)
 def _ctrl_certified(c):
-    return bool(cell_detail.get(c, {}).get("null_excl"))
+    dt = cell_detail.get(c, {})
+    if not dt:
+        return False
+    if dt.get("bar") == "FAIL":
+        return False            # a control breaching the acceptance bar certifies nothing
+    lo, hi = dt["ci"]
+    m_hi, m_lo = dt["margin"]
+    return (lo > m_lo / 2.0) and (hi < m_hi / 2.0)
 
 
 ctrl_void = [c for c in ctrl if _ctrl_contaminating(c)]
-# A control that is not CONTAMINATING but is also not CERTIFIED cannot support a null.
+# NOT CERTIFIED => NO VERDICT MAY BE READ ABOUT THE MEASURAND -- not a null, and NOT A
+# REPRODUCTION EITHER (round-6 codex, BLOCKER: uncertified controls blocked only
+# VANISHES, so with every control at D=+230 the engine still confidently declared P1
+# REPRODUCED). "Uncertainty about a rig-wide confound is not evidence that the confound
+# is absent" -- and P1's whole claim is that the effect is specific to TCP x mixed.
 ctrl_uncertified = [c for c in ctrl if c not in ctrl_void and not _ctrl_certified(c)]
-# Controls carrying a real-but-margin-excluded asymmetry (host x role: q is faster)
-# do not void and do not block -- but they are NEVER silent.
+# Controls that certify clean but still carry a real, tiny asymmetry (host x role -- q
+# is the faster Mac) do not block anything, and are NEVER silent.
 ctrl_caveat = [c for c in ctrl
                if c not in ctrl_void and c not in ctrl_uncertified
                and cell_outcome[c] == "PARTIAL"]
@@ -452,11 +507,28 @@ elif SESSION_VOID_REASON:
            % SESSION_VOID_REASON)
 elif ctrl_void:
     verdict = "RIG-VOID"
-    why = ("control cell(s) are not clean: %s. A rig whose gRPC/large control "
-           "misbehaves cannot adjudicate a TCP-only claim. NO verdict may be read."
+    why = ("control cell(s) are CONTAMINATING -- the rig is carrying the very effect "
+           "this experiment measures: %s. NO verdict may be read."
            % ", ".join("%s(%s,bar=%s)" % (c, cell_outcome[c],
                                           cell_detail.get(c, {}).get("bar", "?"))
                        for c in ctrl_void))
+elif ctrl_uncertified:
+    # BEFORE any measurand branch. A control that cannot be certified clean blocks
+    # EVERY verdict -- the null AND the reproduction. P1 is claimed TCP-only and
+    # mixed-only; if the gRPC/large controls might be carrying the same arm asymmetry,
+    # then neither "it reproduced" nor "it vanished" is readable off this rig.
+    verdict = "CONTROLS-UNCERTIFIED"
+    why = ("control cell(s) could NOT be certified free of an arm asymmetry: %s. A "
+           "control must carry LESS THAN HALF the material effect for P1's TCP-only / "
+           "mixed-only claim to be readable here. Until they do, NO measurand verdict "
+           "may be read -- not a null, and NOT a reproduction: uncertainty about a "
+           "rig-wide confound is not evidence that the confound is absent. Re-run with "
+           "the registered RUNS=16 escalation to buy the power to certify them."
+           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d])"
+                       % (c, cell_outcome[c], cell_detail.get(c, {}).get("D", 0),
+                          cell_detail.get(c, {}).get("ci", (0, 0))[0],
+                          cell_detail.get(c, {}).get("ci", (0, 0))[1])
+                       for c in ctrl_uncertified))
 else:
     outs = {c: cell_outcome[c] for c in verd}
     repro = [c for c, o in outs.items() if o == "REPRODUCES"]
@@ -517,23 +589,6 @@ else:
         why = ("cells cannot exclude an effect of size min(bar_breach, %d ms): %s. A "
                "PASS here is NOT 'P1 vanishes' -- the instrument could not have seen "
                "it (pf-0's error, pre-empted)." % (DELTA_REF, ", ".join(under)))
-    elif van and len(van) == len(verd) and ctrl_uncertified:
-        # A NULL REQUIRES CLEAN CONTROLS -- not merely non-voiding ones. If a control
-        # cannot be shown free of a material arm asymmetry, then "the measurand shows
-        # no effect" is not evidence of absence: the rig might be carrying one
-        # everywhere, and a cancellation would look exactly like this. Round-5, BOTH
-        # reviewers, reproduced: controls at d=[0, 230x7] (and at [230x7, -10]) were
-        # UNDERPOWERED rather than PARTIAL, escaped the void, and the session printed
-        # a clean VANISHES with every control carrying the full rig-W effect.
-        verdict = "INCONCLUSIVE-UNDERPOWERED"
-        why = ("the measurand cells look null, but the NULL IS NOT AVAILABLE: control "
-               "cell(s) %s could not be certified free of an effect of size "
-               "min(bar_breach, %d ms). A null is only meaningful when the controls "
-               "are PROVEN clean -- otherwise a rig-wide arm asymmetry could be "
-               "producing the same picture. Re-run with the registered RUNS=16 "
-               "escalation to buy the power to certify them."
-               % (", ".join("%s(%s)" % (c, cell_outcome[c]) for c in ctrl_uncertified),
-                  DELTA_REF))
     elif van and len(van) == len(verd):
         verdict = "VANISHES"
         why = ("both TCP-mixed cells EXCLUDE an effect of size min(bar_breach, %d ms), "
diff --git a/scripts/otp12pf_mac_verdict_test.py b/scripts/otp12pf_mac_verdict_test.py
index d3610fc..c0e7e33 100644
--- a/scripts/otp12pf_mac_verdict_test.py
+++ b/scripts/otp12pf_mac_verdict_test.py
@@ -47,7 +47,8 @@ MEASURANDS = ("nq_tcp_mixed", "qn_tcp_mixed")
 REGISTERED = MEASURANDS + CONTROLS
 OUTCOMES = {"REPRODUCES", "INVERSION", "PARTIAL", "VANISHES", "UNDERPOWERED",
             "BAR-FAIL-INCONSISTENT", "UNSTABLE", "INCOMPLETE", "MIXED-SIGN",
-            "RIG-VOID", "INCONCLUSIVE", "INCONCLUSIVE-UNDERPOWERED"}
+            "RIG-VOID", "INCONCLUSIVE", "INCONCLUSIVE-UNDERPOWERED",
+            "CONTROLS-UNCERTIFIED"}
 
 
 def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=(),
@@ -71,7 +72,7 @@ def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=
     present = [c for c in REGISTERED if c not in drop_cells]
     with open(runs, "w") as f:
         w = csv.writer(f)
-        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid".split(","))
+        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid".split(","))
         for cell in present:
             if cell in per_cell:
                 d, s = per_cell[cell]
@@ -81,8 +82,8 @@ def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=
                 d, s = control_d, control_src
             srcs = s if isinstance(s, list) else [s] * len(d)
             for i, (di, si) in enumerate(zip(d, srcs), 1):
-                w.writerow([cell, "srcinit", "x", "h", i, si, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
-                w.writerow([cell, "destinit", "x", "h", i, si + di, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
+                w.writerow([cell, "srcinit", "x", "h", i, si, 0, 250, 1, 1, 0, "drained_1x2s", "cold", "yes"])
+                w.writerow([cell, "destinit", "x", "h", i, si + di, 0, 250, 1, 1, 0, "drained_1x2s", "cold", "yes"])
     with open(meta, "w") as f:
         f.write("cell,pairs_attempted,complete\n")
         for cell in present:
@@ -139,10 +140,10 @@ CASES = [
      dict(measurand_d=[-20, 300, 310, 320, 330, 340, 350, 360], src=1000),
      "BAR-FAIL-INCONSISTENT", "REPRODUCES"),
 
-    ("grok (reproduced live): a bar-FAIL control whose CI crosses zero must VOID",
+    ("grok (reproduced live): a bar-FAIL control whose CI crosses zero blocks everything",
      dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
           control_d=[-100, -50, 300, 320, 340, 350, 360, 380], control_src=1000),
-     "RIG-VOID", "VANISHES"),
+     "CONTROLS-UNCERTIFIED", "VANISHES"),
 
     ("codex r2: a missing registered cell is INCOMPLETE, never filtered away",
      dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
@@ -222,14 +223,53 @@ CASES = [
     ("codex r5 (reproduced): an UNDERPOWERED control carrying D=+230 blocks the null",
      dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
           control_d=[0] + [230] * 7, control_src=2500),
-     "INCONCLUSIVE-UNDERPOWERED", "VANISHES"),
+     "CONTROLS-UNCERTIFIED", "VANISHES"),
 
     # grok, same class, different shape: one NEGATIVE pair kills the sign test, so the
     # control is not even directional -- and it still must not support a null.
     ("grok r5 (reproduced): a non-directional but UNCERTIFIED control blocks the null",
      dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
           control_d=[230] * 7 + [-10], control_src=2500),
-     "INCONCLUSIVE-UNDERPOWERED", "VANISHES"),
+     "CONTROLS-UNCERTIFIED", "VANISHES"),
+
+    # --- ROUND 6 -------------------------------------------------------------
+    # grok BLOCKER: certification used the SAME threshold as materiality, so a control
+    # carrying D = +229 -- ONE MILLISECOND under the reference effect -- certified as
+    # "clean" and the session printed VANISHES, prose and all. A control must carry
+    # LESS THAN HALF the material effect, or "P1 is TCP-only" is not readable here.
+    ("grok r6 (reproduced): a control at D=+229 (Delta_ref - 1) must NOT certify clean",
+     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
+          control_d=[229] * 8, control_src=2500),
+     "CONTROLS-UNCERTIFIED", "VANISHES"),
+
+    ("grok r6: ...nor at n=16 with zeros padding the CI down to [0,229]",
+     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3] * 2, src=2000,
+          control_d=[229] * 10 + [0] * 6, control_src=2500, pairs=16),
+     "CONTROLS-UNCERTIFIED", "VANISHES"),
+
+    # codex BLOCKER: uncertified controls blocked only the NULL. With every control at
+    # D=+230 the engine still confidently declared P1 REPRODUCED. Uncertainty about a
+    # rig-wide confound is not evidence that the confound is absent.
+    ("codex r6 (reproduced): uncertified controls must block a REPRODUCTION too",
+     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000,
+          control_d=[0] + [230] * 7, control_src=2500),
+     "CONTROLS-UNCERTIFIED", "REPRODUCES"),
+
+    # A control whose MARGINAL bar fails while its PAIRED CI is tight (n=16, three
+    # outliers move the median) must not certify the rig.
+    ("codex r6: a control's marginal bar FAIL cannot certify, even with a tight CI",
+     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3] * 2, src=2000,
+          control_d=[400] * 3 + [5] * 13, control_src=[1000] * 8 + [1200] * 8, pairs=16),
+     "CONTROLS-UNCERTIFIED", "VANISHES"),
+
+    # codex BLOCKER: the MARGINAL-median bar bypassed the PAIRED magnitude test. Three
+    # outliers move the marginal median enough to fail the bar in the matching
+    # direction, while every pair in the CI is +1ms.
+    ("codex r6 (reproduced): a marginal bar FAIL cannot substitute for paired magnitude",
+     dict(measurand_d=[400] * 3 + [1] * 13,
+          src=[1000] * 8 + [1200] * 8,
+          control_d=[5] * 16, control_src=1000, pairs=16),
+     "PARTIAL", "REPRODUCES"),
 
     # grok BLOCKER: the zero-boundary null. A single zero pair vetoed `ci_lo > 0`, so
     # a 99ms effect on 7 of 8 pairs -- ONE MILLISECOND under the bar -- was "no effect"
@@ -324,13 +364,16 @@ MUTATIONS = [
      ["        breach_lo = -s_med / 11.0", "        breach_lo = -s_med / 10.0"],
      "negative bound", "VANISHES"),
 
-    # The round-2 fail-open, faithfully: the void ignored the BAR entirely.
-    ("RIG-VOID ignores the bar -> fails open (grok r2, reproduced live)",
-     ['    if dt.get("bar") == "FAIL" or cell_outcome[c] == "UNSTABLE":\n'
-      '        return True',
-      '    if cell_outcome[c] == "UNSTABLE":\n'
-      '        return True'],
-     "bar-FAIL control", "VANISHES"),
+    # A control whose MARGINAL bar fails cannot certify the rig, even when its PAIRED
+    # CI is tight. At n=8 that is provably unreachable (a CI inside +-half-margin bounds
+    # the median shift to <=5%), but at n=16 three outliers move the marginal median
+    # while the CI stays at [5,5] -- so the clause is load-bearing exactly there.
+    ("a bar-FAIL control certifies the rig anyway (codex r6)",
+     ['    if dt.get("bar") == "FAIL":\n'
+      '        return False            # a control breaching the acceptance bar certifies nothing',
+      '    if False:\n'
+      '        return False'],
+     "marginal bar FAIL cannot certify", "VANISHES"),
 
     # The fix is BOTH halves: the cell loop must walk the REGISTERED set (not merely
     # what turned up in the CSV), and absent cells must be marked INCOMPLETE rather
@@ -344,8 +387,8 @@ MUTATIONS = [
      "missing registered cell", "VANISHES"),
 
     ("materiality requires a bar FAIL, so exact 1.10 is unreachable (grok)",
-     ['        material = bar_fail_pos or (ci_lo >= breach_hi)',
-      '        material = bar_fail_pos'],
+     ['        material = ci_lo >= breach_hi                # MAGNITUDE  -- the paired CI, only',
+      '        material = (bar == "FAIL")'],
      "EXACT 1.10", "PARTIAL"),
 
     # The sign test no longer PARTICIPATES: direction is asserted regardless of it.
@@ -359,10 +402,8 @@ MUTATIONS = [
 
     # --- ROUND 3 (grok) -------------------------------------------------------
     ("control void ignores absolute materiality -> a Delta_ref control escapes (grok r3)",
-     ['    if dt.get("directional") and dt.get("ci_at_or_beyond_margin"):\n'
-      '        return True',
-      '    if False:\n'
-      '        return True'],
+     ['    return bool(dt.get("directional") and dt.get("ci_at_or_beyond_margin"))',
+      '    return False'],
      "Delta_ref-sized control effect", "VANISHES"),
 
     ("engine trusts meta.complete and never checks n (grok r3)",
@@ -388,15 +429,14 @@ MUTATIONS = [
 
     # --- ROUND 5 -------------------------------------------------------------
     ("`bar == FAIL` is direction-blind, so +1ms is 'material' (codex r5)",
-     ['        bar_fail_pos = (bar == "FAIL") and d_med > s_med     # the bar failed the SAME way\n'
-      '        bar_fail_neg = (bar == "FAIL") and d_med < s_med',
-      '        bar_fail_pos = (bar == "FAIL")\n'
-      '        bar_fail_neg = (bar == "FAIL")'],
+     ['        material = ci_lo >= breach_hi                # MAGNITUDE  -- the paired CI, only\n'
+      '        material_neg = ci_hi <= breach_lo',
+      '        material = (bar == "FAIL") or ci_lo >= breach_hi\n'
+      '        material_neg = (bar == "FAIL") or ci_hi <= breach_lo'],
      "INVERSE direction cannot make +1ms", "REPRODUCES"),
 
     ("an UNCERTIFIED control can still support a null (codex r5 + grok r5)",
-     ["    elif van and len(van) == len(verd) and ctrl_uncertified:",
-      "    elif False:"],
+     ["elif ctrl_uncertified:", "elif ctrl_uncertified and False:"],
      "UNDERPOWERED control carrying D=+230", "VANISHES"),
 
     ("the CI vetoes DIRECTION, so one zero pair turns 99ms into equivalence (grok r5)",
@@ -404,6 +444,23 @@ MUTATIONS = [
       "        directional = p < 0.05 and ci_lo > 0 and ci_hi > 0"],
      "7/8 pairs at 99ms", "VANISHES"),
 
+    # --- ROUND 6 -------------------------------------------------------------
+    ("the MARGINAL bar substitutes for PAIRED magnitude (codex r6)",
+     ["        material = ci_lo >= breach_hi                # MAGNITUDE  -- the paired CI, only\n"
+      "        material_neg = ci_hi <= breach_lo",
+      "        material = ((bar == \"FAIL\") and d_med > s_med) or ci_lo >= breach_hi\n"
+      "        material_neg = ((bar == \"FAIL\") and d_med < s_med) or ci_hi <= breach_lo"],
+     "marginal bar FAIL cannot substitute", "REPRODUCES"),
+
+    ("a control certifies clean at the SAME threshold as materiality (grok r6)",
+     ["    return (lo > m_lo / 2.0) and (hi < m_hi / 2.0)",
+      "    return (lo > m_lo) and (hi < m_hi)"],
+     "D=+229 (Delta_ref - 1)", "VANISHES"),
+
+    ("uncertified controls block only the NULL, not a REPRODUCTION (codex r6)",
+     ["elif ctrl_uncertified:", "elif ctrl_uncertified and False:"],
+     "block a REPRODUCTION too", "REPRODUCES"),
+
     ("the registered DELTA_REF is taken from the environment again (codex r5)",
      ['DELTA_REF = REGISTERED_DELTA_REF_MS\n'
       '_env_delta = os.environ.get("DELTA_REF_MS")',

codex
HEAD is the requested `b3d42b7`; the worktree has two unrelated untracked review-result files, which I will leave untouched. Local `master` is seven commits ahead of `origin/master`, so I’m pinning every conclusion to the requested HEAD and will separately note whether the read-only remote freshness check is reachable.
codex
The canonical remote is unreachable in this sandbox (`Could not resolve host: q`), so freshness beyond the local refs cannot be established; this does not block the HEAD-pinned review. I’m now reading the canonical project state and all four instrument files in full, then I’ll map every claimed protection to an executable failure path before running them.
exec
/bin/zsh -lc "wc -l docs/STATE.md scripts/bench_otp12pf_mac.sh scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md && sed -n '1,260p' docs/STATE.md && nl -ba scripts/bench_otp12pf_mac.sh | sed -n '1,1400p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     200 docs/STATE.md
    1066 scripts/bench_otp12pf_mac.sh
     647 scripts/otp12pf_mac_verdict.py
     523 scripts/otp12pf_mac_verdict_test.py
     550 docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
    2986 total
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-14 (49th session)

- **⛔ NEXT ACTION IS AN OWNER CALL, NOT A RUN. The Mac↔Mac rig (Queue 1(ii)) is BLOCKED: `codex` — the MANDATORY reviewer — is OUT OF CREDITS until 2026-07-19.** The instrument is reworked (`a9460ce`, prereg **rev 5**) but **NOT CLEARED**, and the harness **refuses a timed run** (`exit 2` without `CLEARED_BY_REVIEW=1`). **NO DATA TAKEN.** Round 4 was reviewed by **grok alone**, and D-2026-07-14-2 says grok is *"additive, never a substitute, and never runs alone"* — so **no agent may clear this**. **Owner: wait for codex (2026-07-19) / buy credits / amend the rule.** (Grok found 9 real defects including a BLOCKER, which is the argument *for* the second reviewer, not a reason to promote it.) When cleared: **⚠ THE OWNER MUST CLOSE THEIR CODEX SESSIONS** — nagatha is a bench **END** and the quiescence gate refuses while `codex`/`cargo`/`rustc` runs on **either** Mac (it fires correctly today). Time Machine OFF on both. Rig: nagatha `10.1.10.92` ↔ `q` `10.1.10.54`, 10GbE/9000, build pinned `f35702a`. **Then `pf-1`** (the HARD GATE), which two pf-0 results BIND: between-session grading is dead (a 20% recovery = 46 ms sits under the 78 ms floor), so pf-1 must **measure its own paired within-session floor** before grading; and the fast arm is **BISTABLE** — grade the distribution, not the median.
- **THE INSTRUMENT IS THE RISK — 44 findings across FOUR reviews of this one harness, 44 accepted, 0 rejected.** Three project claims have already been retracted to harness bugs. **Every rework of this instrument has introduced a defect of its own**: round 3's killer (a timer reading a 1000 ms transfer as **−1 ms** — cross-process `monotonic()` is process-relative on macOS, so the entire measurand would have been **fsync noise**) was introduced by the round-2 rework; round 4's BLOCKER (grok **drove a clean `VANISHES` while every control carried the full rig-W effect**) was the *same* bar-vs-Δ_ref error as round 3, fixed for the measurand and left in the controls. Standing rules, earned the hard way: **verify the instrument before believing the measurement**; **READ THE EXISTING HARNESSES BEFORE WRITING A NEW ONE** (zoey *already documented* the monotonic trap); **`bash -n` is not an execution** (round 1's "fixes" had never been run — the preflight could not even succeed); and **fixing a bug in one place is not fixing its class.**
- **⚠ THE MAC↔MAC RIG IS *NOT* AN H1 DISCRIMINATOR — retracted 2026-07-14.** The earlier claim ("reproduces ⇒ **H1 DIES**, H1 accuses the *Windows* accept branch") was **WRONG**: H1 accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **"Windows" appears nowhere in H1**, and that code runs on macOS too, so a reproduction is *consistent with* H1. H1 now carries a **canonical note** in the parent plan so the shorthand cannot mislead again. What the rig **does** answer, scoped to **this pair**: **can P1 occur WITHOUT a Windows peer?** Reproduces ⇒ P1 is **not** waivable "platform residue" and code hypotheses strengthen (it does **not** prove a platform-*general* cost). A null ⇒ P1 did not reproduce **on this pair** — consistent with "Windows required" but **not proof** of it, and only reportable at all if the run **excludes a bar-breaching effect** (else `INCONCLUSIVE-UNDERPOWERED`).
- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
  invariant (plan doc, verbatim): ONE block of transfer code;
  direction/initiator/verb can NEVER affect wall time by blit's doing
  — impossible by construction because the per-direction drivers and
  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
  cells. **D-2026-07-05-2: same-build peers only, refusal at session
  open.** Progress (each slice through the codex loop; per-slice
  detail lives in DEVLOG + `.review/`, NOT here):
  - **Closed `[x]`: otp-1 … otp-11** — the whole session machine, the
    baselines (otp-2/2w), the **CUTOVER DELETION** (4 drivers +
    `Push`/`PullSync` + 13 messages out of tree AND proto, −13.8k lines,
    no bridge; relay removed D-2026-07-11-1), and **otp-11b's deletion of
    the entire old orchestration** (−6.2k lines: orchestrator, engine,
    local_worker, auto_tune, change_journal — the last an UNSOUND fast
    path that silently lost data). The deletion-proof acceptance line
    COMPLETES. Detail: DEVLOG 2026-07-10/11/12; evidence
    `docs/bench/otp2{,w}-baseline-2026-07-10/`, `otp11-local-2026-07-11/`.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
  in Blocked); the codex loop governs all changes (D-2026-07-04-1).

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
   (Mac↔Windows)** `[x]`. 12a: 10 PASS, 2 to the walk. 12b — THE
   INVARIANCE CRITERION: 11/12 PASS (1.003–1.057); wm_tcp_mixed 1.237
   (TCP×mixed×dest-initiator, code-shaped); push_tcp_small 1.149
   (both rigs); Win→Mac beats the better old direction 6/6; Mac→Win
   gap shapes recorded for the walk
   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
   **otp-12d and otp-13 are DEFERRED, not next** — otp-12c's rows are
   PRE-FIX, and `docs/plan/OTP12_PERF_FINDINGS.md` (pf-final) voids
   pre-fix new arms for acceptance. Assembling the acceptance matrix now
   would build otp-13's artifact from void rows.
1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
   (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
   reviewloop slice by slice"; implementation proceeds, each slice
   through the codex loop).
   Two experiments come BEFORE any code; both docs own their detail.
   **(i) The A-B-B-A MTU run on `q` — `[x]` DONE 2026-07-14: MTU KILLED**
   (`r = −3.1%`; `docs/bench/otp12-jumbo-win-2026-07-13/`). See the pf-0
   bullet at the top for the two limits it puts on pf-1.
   **(ii) THE MAC↔MAC RIG — the missing cell of the 2×2** (owner,
   2026-07-13). Linux↔Linux = **no P1** (8/8 PASS); macOS↔Windows = **P1**
   (1.237/1.300/1.385/1.362); macOS↔macOS = **?** Design, decision rule and
   the retraction of the "H1 dies" framing: **see NEXT ACTION at the top**
   and the rev-2 pre-registration. **Both Macs are bench ENDS: the codex
   loop CANNOT run during the session** (the gate enforces it).
   **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a
   *cross-direction* miss for a cell that ALREADY passes invariance — P1
   *is* the invariance failure. **Fix it to ≤1.10, or the owner amends
   acceptance criterion 1.** Not assumed either way. P2
   (`push_tcp_small` 1.105–1.201) is a converge bar vs the OLD build,
   UNTESTED on the Linux rig. Sequence: **MTU run + Mac↔Mac → pf-1 → fix
   → pf-final (ALL rigs) → otp-12d → otp-13.**
1b. **AFTER otp-12 — the Windows/local pair, planned TOGETHER** (same tar
   path, opposite directions: a fidelity fix ADDS per-file work to a path
   already losing to robocopy, so planning them apart optimises one against
   the other). Both docs own their detail; do not restate it here.
   - **`docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` (D-2026-07-13-3)**
     — Windows attributes + ADS silently dropped, exit 0, **both routes
     (measured)**; loss is **conditional on file count**
     (`transfer_plan.rs:103-109`). Unlanded Windows support, NOT a regression.
     **Fix = WIRE CONTRACT change** → amend `TRANSFER_SESSION.md` first.
   - **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2)** — local
     apply **does not scale** (8 workers buy 1.05×; robocopy gets ~2.2× from 8
     threads) and ships **one** worker. At EQUAL concurrency blit BEATS
     robocopy; at 8-vs-8 it loses 1.9×. `docs/bench/win-local-ab-2026-07-13/`.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Follow-ups largely
   absorbed by otp-2/otp-12's rig matrices.
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md`; w7-1 topmost open row —
   likely landed inside otp-6's one-delete-rule slice; re-check first).
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   no daemon runs on zoey without a fresh go.
6. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
  is retired as the grading mechanism (its `findings/`/`results/`
  records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked
  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
  (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (all owner declarations; checkpoints are owner-only)

- **Rigs**: owner go standing through otp-12. zoey (12a), netwatch-01
  (12b), netwatch-01↔skippy (12c) done; **magneto↔skippy = the same-OS
  rig** (new 2026-07-13). Rig facts + the macOS ping/MTU trap:
  `.agents/machines.md`.
- **otp-12c RECORDED 2026-07-13** (pre-fix rows = replication/control
  evidence, NOT acceptance evidence; Queue 1a):
  `docs/bench/otp12c-win-2026-07-13/` (198 runs) and
  `otp12c-delegated-2026-07-13/` (**rig D 7/7 PASS**). Codex: FAIL →
  **7/7 accepted**. Detail: DEVLOG 2026-07-13.
- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)

## Open questions

- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
- **(SLOTTED 2026-07-12 — owner ack)** `docs/WHITEPAPER.md` §8 (~line
  592) describes the deleted `determine_remote_tuning` — fix folded
  into **w10-docs-batch**; no one-off edit.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: w9-3 fixed the
  Linux daemon-spawn flakiness; windows-latest CI pending a push.
  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.
- *(Resolved 2026-07-12/13: SizeMtime SKIP, `725aa07`, the `./NAME` foot-gun,
  otp-5b-3 cancel, the change-journal premise — all landed; see DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-14 (49th)** — **Mac↔Mac instrument rounds 3 + 4: 24 more findings, 24
  accepted, NO DATA; the harness now REFUSES a timed run.** R3 (`cae2e0f`, codex 12 +
  grok 3): **my timer read a 1000 ms sleep as −1 ms** (cross-process `monotonic()` is
  process-relative on macOS) — every row would have been fsync noise. Preflight now
  **proves the clock on both hosts** first. R4 (`a9460ce`, grok alone, 9): it **drove a
  clean `VANISHES` while every control carried the full rig-W effect** — the *same*
  bar-vs-Δ_ref error as R3, fixed for the measurand, left in the controls. Also: the
  engine trusted `meta.complete` (a 1-pair CSV graded at 0% CI coverage); precedence
  **hid a clean one-direction REPRODUCES**; my own comment claimed end-load could void
  a session when the code only logged it. Guard: 17 cases, **11/11 mutations killed**.
  Full: **DEVLOG 2026-07-14 16:30Z**.
  **⛔ BLOCKED — CODEX IS OUT OF CREDITS UNTIL 2026-07-19**, so R4 has no codex review
  and D-2026-07-14-2 forbids clearing on grok alone. **OWNER CALL: wait / buy credits /
  amend the rule.** In-flight: none; no rig time taken.
  **NEXT: that call → round-5 review → the run → pf-1.**
- **2026-07-14 (48th)** — **pf-0 ran; MTU is KILLED as a cause of P1** (`r = −3.1%`;
  256 runs, 0 voided). codex NOT READY → **7/7 accepted** (`11f0c2a`): every *claim*
  outran the data — the run is **not powered** for a *contributing*-size effect
  (46 ms < the 78 ms floor), and declaring the frozen baseline VOID was **not an
  agent's call**. **The fast arm is BISTABLE** (the mode mixture, not MTU, sets the
  noise floor). TM on `q` fired 1 min before the run (owner disabled it; the quiet-gate
  does not catch it); a **physically flapping `en8`** killed three starts.
  Full: **DEVLOG 2026-07-14 06:20Z**.
- **2026-07-13/14 (47th)** — P1 reproduces on a second Mac (`q`); new bench Mac;
  Windows attrs+ADS bug (D-2026-07-13-3); the robocopy headline was WRONG
  (D-2026-07-13-2); MTU prereg rev 1→4. Full: **DEVLOG 2026-07-14 00:15Z**.
- *(46th and earlier pruned to the cap — see DEVLOG 2026-07-06..14.)*
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# ⛔ NOT CLEARED TO RUN — REWORKED IN ROUND 3, REVIEW NOT YET PASSED ⛔
     4	#
     5	# The round-3 rework (this file) addresses all 15 findings from codex round 2 and
     6	# grok's second opinion. It has NOT been reviewed. The review is the gate, not the
     7	# rework: three rounds running, every revision of this instrument has shipped a
     8	# defect capable of a false claim, and two of them were introduced BY THE REWORK
     9	# THAT FIXED THE PREVIOUS ONE.
    10	#
    11	#   .review/results/macmac-harness-r2.gpt-verdict.md    (codex, 12 findings)
    12	#   .review/results/macmac-harness-r2.grok-verdict.md   (grok, +3 findings)
    13	#
    14	# Clearing it: land the round-3 review, adjudicate, and delete this block plus the
    15	# CLEARED_BY_REVIEW guard below. Until then `SELFTEST=1` and `PREFLIGHT_ONLY=1`
    16	# work (they take NO data); a timed run refuses.
    17	# =============================================================================
    18	# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
    19	# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (rev 4)
    20	# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
    21	#
    22	# WHY THIS RIG EXISTS
    23	# -------------------
    24	# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
    25	# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
    26	# cell. It answers ONE question, SCOPED TO THIS PAIR:
    27	#
    28	#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
    29	#
    30	#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
    31	#     "platform residue" that can be waived; code-level hypotheses strengthen. It
    32	#     leaves macOS/APFS and host x role explanations OPEN.
    33	#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
    34	#     "Windows is required", but does NOT prove it: it could equally be a
    35	#     property of these two machines, their disks, or this macOS version.
    36	#
    37	# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
    38	# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
    39	# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
    40	# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
    41	# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
    42	# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
    43	#
    44	# THE INSTRUMENT IS THE RISK. Three claims in this project have been retracted to
    45	# harness bugs, and this harness alone has now had 20 defects found across two
    46	# reviews. What round 2 caught, and what is fixed here:
    47	#
    48	#   * THE TIMER WAS MEASURING FSYNC NOISE. It captured time.monotonic() in TWO
    49	#     separate `python3 -c` processes and subtracted them. On macOS that clock is
    50	#     PROCESS-RELATIVE: a 1000 ms sleep measured -1 ms on nagatha and 2 ms on q
    51	#     (measured; yes, negative). Every `ms` row would have been ~= fsync_ms alone,
    52	#     and the invariance ratio — THE ENTIRE MEASURAND — would have been computed on
    53	#     fsync noise, which can manufacture or mask a one-directional effect at will.
    54	#     The repo ALREADY documents this trap (bench_otp12_zoey.sh:116 uses time.time()
    55	#     precisely because monotonic is wrong across processes) and I reintroduced it
    56	#     anyway. Now: ONE process times itself and spawns the client (time_argv), and
    57	#     PREFLIGHT PROVES IT on both hosts against a known sleep before any data.
    58	#   * The preflight COULD NOT SUCCEED: `grep -c` exits 1 on no match, so a CLEAN
    59	#     binary tripped the dirty-marker probe and died; and norm_mac used gawk's
    60	#     strtonum(), absent from stock macOS awk. The round-1 "fixes" were never
    61	#     executed — I ran `bash -n`, not the gates. Every gate below is now exercised
    62	#     by SELFTEST=1, which runs them for real.
    63	#   * Gates FAILED OPEN: pgrep errors read as "quiet"; a failed `top` read as 0%
    64	#     CPU and a late idle sample could overwrite a busy one; non-numeric `iostat`
    65	#     read as zero and CERTIFIED drainage; the drain watched a hardcoded `disk0`
    66	#     that the data need never touch (grok); `die` inside $(...) exited only the
    67	#     subshell, so an empty hash still landed. Every probe is now sentinel-framed,
    68	#     rc-aware, and fails CLOSED.
    69	#
    70	# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
    71	# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
    72	# one ssh), so dispatch is outside the window by construction.
    73	#
    74	# Usage:
    75	#   SELFTEST=1       bash scripts/bench_otp12pf_mac.sh   # exercise every gate, no data
    76	#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    77	#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh # the run (needs review clearance)
    78	# =============================================================================
    79	set -euo pipefail
    80	
    81	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    82	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    83	SELF="${BASH_SOURCE[0]}"
    84	VERDICT_PY="$SCRIPT_DIR/otp12pf_mac_verdict.py"
    85	VERDICT_TEST="$SCRIPT_DIR/otp12pf_mac_verdict_test.py"
    86	
    87	SELFTEST="${SELFTEST:-0}"
    88	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
    89	
    90	# The review is the gate. A timed run refuses until round 3 is adjudicated; the
    91	# no-data modes stay available so the gates can be exercised.
    92	if [[ "$SELFTEST" != 1 && "$PREFLIGHT_ONLY" != 1 && "${CLEARED_BY_REVIEW:-0}" != 1 ]]; then
    93	  echo "REFUSING: this harness was reworked in round 3 and has NOT passed review." >&2
    94	  echo "Every previous revision shipped a defect capable of a false claim, and two" >&2
    95	  echo "were introduced by the rework that fixed the last one. Land the round-3" >&2
    96	  echo "review first. SELFTEST=1 and PREFLIGHT_ONLY=1 take no data and still run." >&2
    97	  exit 2
    98	fi
    99	
   100	# The pre-registered build. Not overridable by accident: a run against an
   101	# unregistered build is not the registered experiment.
   102	REGISTERED_BUILD="f35702a"
   103	
   104	# --- nagatha: LOCAL end (driver) ---------------------------------------------
   105	N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
   106	N_NIC="${N_NIC:-en11}"
   107	N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
   108	N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
   109	N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
   110	N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
   111	N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
   112	
   113	# --- q: REMOTE end ------------------------------------------------------------
   114	Q_SSH="${Q_SSH:-michael@q}"
   115	Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
   116	Q_NIC="${Q_NIC:-en8}"
   117	Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
   118	Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
   119	Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
   120	Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
   121	Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
   122	
   123	PORT="${PORT:-9031}"
   124	RUNS="${RUNS:-8}"
   125	
   126	# =============================================================================
   127	# THE REGISTERED CONSTANTS. **NOT OVERRIDABLE.**
   128	#
   129	# Round-5 (codex, BLOCKER): these were `${VAR:-default}`, so the pre-registered
   130	# decision rule could be edited FROM THE COMMAND LINE — `DELTA_REF_MS=240` turned a
   131	# RIG-VOID into a VANISHES. A pre-registration that the operator can retune, after
   132	# the data exists, in the direction of the answer they want, IS NOT A
   133	# PRE-REGISTRATION AT ALL.
   134	#
   135	# They are literals, and the harness REFUSES to start if one is merely PRESENT in the
   136	# environment — a deviation must be loud, never silently ignored. The check reads the
   137	# environment BEFORE the assignments below, or an override would be masked by the
   138	# very line meant to pin it.
   139	# =============================================================================
   140	_overrides=""
   141	for _v in SETTLE_MS LOAD_MAX DRAIN_ITERS DRAIN_QUIET DRAIN_MBPS DELTA_REF_MS TIMER_TOLERANCE_MS; do
   142	  [[ -n "${!_v+set}" ]] && _overrides="$_overrides $_v=${!_v}"
   143	done
   144	if [[ -n "$_overrides" ]]; then
   145	  echo "REFUSING: the pre-registered constants are NOT tunable, and these are set in the" >&2
   146	  echo "environment:$_overrides" >&2
   147	  echo "A rule the operator can retune after seeing the data is not a pre-registration." >&2
   148	  echo "To change one, amend docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md and" >&2
   149	  echo "put it back through review. That is the entire point of the document." >&2
   150	  exit 2
   151	fi
   152	
   153	SETTLE_MS=250              # equal pre-fsync window on BOTH arms
   154	# Computed ONCE, HERE, at top level — and this line is load-bearing history.
   155	#
   156	# It used to be computed inline as `sleep $(awk ... 'BEGIN{printf \"%.3f\", m/1000}')`
   157	# INSIDE the double-quoted hrun string. A command substitution is parsed FRESH by
   158	# bash, so those `\"` escapes — which are correct for hrun's two-level strings — were
   159	# literal backslashes to awk. **The awk errored on EVERY call, `sleep` got an empty
   160	# argument and FAILED, and the old code ignored its exit status because the python
   161	# walk that followed supplied the status.**
   162	#
   163	# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
   164	# it. And 24660ae is the commit that added it TO FIX the free-writeback asymmetry
   165	# that reverses sign with direction — the artifact judged capable of MANUFACTURING a
   166	# one-directional P1 out of nothing. The pre-registration has claimed an equal settle
   167	# on both arms through revisions 3, 4 and 5. It was never applied.
   168	#
   169	# Found only by EXECUTING it (round-5 codex flagged the ignored exit status; running
   170	# it showed the status was ALWAYS failure). `bash -n` sees nothing here.
   171	SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
   172	[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
   173	LOAD_MAX=3.0               # start AND end load1 bar on both Macs
   174	DRAIN_ITERS=60
   175	DRAIN_QUIET=3
   176	DRAIN_MBPS=2               # destination disk must be below this to start a window
   177	DELTA_REF_MS=230           # rig W's measured Delta_P1 — THE reference effect
   178	TIMER_TOLERANCE_MS=120     # the timer self-test's allowed error on a 1000 ms sleep
   179	
   180	# The REGISTERED cell set. The verdict engine requires ALL of them present and
   181	# complete: a partial set that is merely filtered lets a ONE-CELL run emit
   182	# "VANISHES" while claiming both cells vanished (codex r2 BLOCKER 1).
   183	REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   184	CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   185	VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
   186	CELLS="$REGISTERED_CELLS"
   187	
   188	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   189	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
   190	ESCALATED_FROM=""          # set only by the verified RUNS=16 escalation
   191	PRIOR_RUNS_SHA=""          # the data hash the escalation is bound to
   192	
   193	MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
   194	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   195	         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
   196	qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
   197	
   198	mkdir -p "$OUT_DIR/blit-logs"
   199	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
   200	die() { log "FATAL: $*"; exit 1; }
   201	# A gate that CANNOT ANSWER is BLIND, and blindness is what fails open on the night.
   202	# It is marked EXPLICITLY here, never inferred from the wording of a message —
   203	# inferring it from prose is how a blind timer came to be scored as a working gate.
   204	die_blind() { log "FATAL[PROBE-BLIND]: $*"; exit 1; }
   205	nocr() { tr -d '\r'; }
   206	
   207	# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
   208	# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
   209	# falls through to the wrong host (the trap the Linux harness documents).
   210	# `bash -c` locally pins the inner shell so local and remote parse identically.
   211	# pipefail is set in BOTH children: without it a failed probe at the head of a
   212	# pipeline is masked by a successful `tail`/`awk` and the gate reads "fine".
   213	hrun() {
   214	  local h="$1"; shift
   215	  local cmd="set -o pipefail
   216	$*"
   217	  if [[ "$h" == n ]]; then bash -c "$cmd"; else qssh "bash -c $(printf '%q' "$cmd")"; fi
   218	}
   219	hblit()   { if [[ "$1" == n ]]; then echo "$N_BLIT";   else echo "$Q_BLIT";   fi; }
   220	hdaemon() { if [[ "$1" == n ]]; then echo "$N_DAEMON"; else echo "$Q_DAEMON"; fi; }
   221	hmod()    { if [[ "$1" == n ]]; then echo "$N_MODULE"; else echo "$Q_MODULE"; fi; }
   222	hip()     { if [[ "$1" == n ]]; then echo "$N_IP";     else echo "$Q_IP";     fi; }
   223	hnic()    { if [[ "$1" == n ]]; then echo "$N_NIC";    else echo "$Q_NIC";    fi; }
   224	hmac()    { if [[ "$1" == n ]]; then echo "$N_MAC";    else echo "$Q_MAC";    fi; }
   225	hname()   { if [[ "$1" == n ]]; then echo nagatha;     else echo q;           fi; }
   226	other()   { if [[ "$1" == n ]]; then echo q;           else echo n;           fi; }
   227	
   228	# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
   229	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   230	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   231	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   232	fix_count() { case "$1" in large) echo $FIX_COUNT_large;; mixed) echo $FIX_COUNT_mixed;; small) echo $FIX_COUNT_small;; esac; }
   233	fix_bytes() { case "$1" in large) echo $FIX_BYTES_large;; mixed) echo $FIX_BYTES_mixed;; small) echo $FIX_BYTES_small;; esac; }
   234	
   235	# =============================================================================
   236	# THE TIMER. One process times itself AND spawns the client, so the interval is
   237	# measured by a single clock and python's startup cost falls outside it.
   238	#
   239	# NEVER bracket a command with two separate `python3 -c 'time.monotonic()'` calls:
   240	# on macOS that clock is PROCESS-RELATIVE and the difference is garbage (measured:
   241	# -1 ms and 2 ms for a 1000 ms sleep). bench_otp12_zoey.sh:116 already said so.
   242	# =============================================================================
   243	time_argv() {   # $1 = host; rest = argv. Echoes "MS,RC" or "" on a broken probe.
   244	  local h="$1"; shift
   245	  local qa="" a
   246	  for a in "$@"; do qa="$qa $(printf '%q' "$a")"; done
   247	  hrun "$h" "python3 - $qa <<'PYEOF'
   248	import subprocess, sys, time
   249	argv = [a for a in sys.argv[1:] if a]          # an empty flag must not become argv
   250	err = open('/tmp/mm-client.err', 'wb')
   251	t = time.monotonic()
   252	rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=err)
   253	ms = int((time.monotonic() - t) * 1000)
   254	err.close()
   255	print('R:%d,%d:R' % (ms, rc))
   256	PYEOF" | nocr | sed -n 's/.*R:\(-\{0,1\}[0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1
   257	}
   258	
   259	# The gate that makes the timer bug unshippable: prove the clock on the rig,
   260	# against a known interval, before any data is taken.
   261	timer_gate() {
   262	  local h="$1" out ms rc lo hi
   263	  out="$(time_argv "$h" /bin/sleep 1)"
   264	  [[ "$out" == *,* ]] || die_blind "$(hname "$h"): the timer probe returned nothing — refusing"
   265	  ms="${out%%,*}"; rc="${out##*,}"
   266	  [[ "$rc" == 0 ]] || die_blind "$(hname "$h"): the timer probe's own child exited $rc"
   267	  lo=$(( 1000 - TIMER_TOLERANCE_MS )); hi=$(( 1000 + TIMER_TOLERANCE_MS ))
   268	  if (( ms < lo || ms > hi )); then
   269	    die "$(hname "$h"): THE TIMER IS LYING — a 1000 ms sleep measured ${ms} ms (allowed ${lo}-${hi}).
   270	This is the round-2 killer: cross-process time.monotonic() on macOS is PROCESS-RELATIVE and
   271	read -1 ms / 2 ms for this exact sleep. Every row would be fsync noise. REFUSING to take data."
   272	  fi
   273	  log "  timer ok on $(hname "$h"): a 1000 ms sleep measures ${ms} ms"
   274	}
   275	
   276	# --- provenance ---------------------------------------------------------------
   277	# `die` inside $(...) exits only the SUBSHELL, so the outer command substitution
   278	# succeeds with an empty value. These return non-zero instead and the CALLER dies.
   279	embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
   280	  local h="$1" p="$2" raw hit dirty
   281	  # `grep -c` exits 1 on NO MATCH, which is not an error. Only rc>=2 is. The old
   282	  # `|| echo X` turned a clean binary's legitimate "0" into "0\nX" and DIED.
   283	  raw="$(hrun "$h" "c=\$(grep -c -a -- '+$EXPECT_SHA' '$p'); rc=\$?
   284	d=\$(grep -c -a -- '+$EXPECT_SHA.dirty' '$p'); rd=\$?
   285	if [ \$rc -ge 2 ] || [ \$rd -ge 2 ]; then echo 'E:ERR:E'; else echo \"E:\$c:\$d:E\"; fi" \
   286	    | nocr | sed -n 's/.*E:\([0-9]*\):\([0-9]*\):E.*/\1 \2/p' | head -1)" || return 1
   287	  [[ -n "$raw" ]] || return 1
   288	  read -r hit dirty <<<"$raw"
   289	  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
   290	  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
   291	}
   292	sha256_of() {      # returns non-zero on a short/empty hash; the CALLER must `|| die`
   293	  local h="$1" p="$2" v
   294	  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')" || return 1
   295	  [[ ${#v} -eq 64 ]] || return 1
   296	  echo "$v"
   297	}
   298	
   299	# --- gates: every one fails CLOSED --------------------------------------------
   300	# Stock macOS awk has no strtonum() (that is gawk). Hand-rolled hex, so the ARP
   301	# comparison actually runs instead of erroring out.
   302	norm_mac() {
   303	  awk -F: '
   304	    function hex(s,   i,c,d,v) {
   305	      v = 0; s = tolower(s)
   306	      for (i = 1; i <= length(s); i++) {
   307	        c = substr(s, i, 1); d = index("0123456789abcdef", c) - 1
   308	        if (d < 0) return -1
   309	        v = v * 16 + d
   310	      }
   311	      return v
   312	    }
   313	    {
   314	      if (NF != 6) { print ""; next }
   315	      out = ""; ok = 1
   316	      for (i = 1; i <= NF; i++) {
   317	        v = hex($i)
   318	        if (v < 0 || v > 255) { ok = 0; break }
   319	        out = out sprintf("%s%02x", (i > 1 ? ":" : ""), v)
   320	      }
   321	      print (ok ? out : "")
   322	    }'
   323	}
   324	
   325	# THE ONLY process probe in this harness. pgrep: 0 = found, 1 = none, >=2 = ERROR.
   326	# Echoes RUNNING | NONE | BROKEN. A probe that cannot answer must NEVER answer "fine",
   327	# and there must be exactly ONE of these -- round 5 found the fail-open surviving in a
   328	# duplicate site precisely because there were two.
   329	pgrep_state() {
   330	  local h="$1" pat="$2" raw
   331	  raw="$(hrun "$h" "pgrep -x '$pat' >/dev/null 2>&1; rc=\$?
   332	if [ \$rc -eq 0 ]; then echo 'G:RUNNING:G'
   333	elif [ \$rc -eq 1 ]; then echo 'G:NONE:G'
   334	else echo 'G:BROKEN:G'; fi" | nocr | sed -n 's/.*G:\([A-Z]*\):G.*/\1/p' | head -1)" || raw=""
   335	  case "$raw" in
   336	    RUNNING|NONE|BROKEN) echo "$raw" ;;
   337	    *)                   echo BROKEN ;;   # no sentinel back == a broken probe
   338	  esac
   339	}
   340	
   341	quiescence_gate() {
   342	  local h="$1" p busy=""
   343	  for p in codex cargo rustc; do
   344	    case "$(pgrep_state "$h" "$p")" in
   345	      RUNNING) busy="$busy $p" ;;
   346	      NONE)    : ;;
   347	      *)       die_blind "$(hname "$h"): the quiescence probe for '$p' BROKE — refusing (a gate that cannot answer must not answer 'fine')" ;;
   348	    esac
   349	  done
   350	  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running:$busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
   351	}
   352	
   353	timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   354	  local h="$1" running auto
   355	  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
   356	  [[ "$running" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
   357	  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
   358	  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1" | nocr | tr -cd '0-9')" || auto=""
   359	  [[ "$auto" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
   360	  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
   361	}
   362	
   363	spotlight_gate() {
   364	  local h="$1" cpu
   365	  # The MAX across samples, not the last: a late idle sample could overwrite an
   366	  # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
   367	  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
   368	    | awk '/^mds_stores/{ if (\$2+0 > m) m = \$2+0 } END{ if (NR == 0) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
   369	  [[ "$cpu" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
   370	  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
   371	}
   372	
   373	load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
   374	load_gate() {
   375	  local h="$1" l ok
   376	  l="$(load1 "$h")" || l=""
   377	  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die_blind "$(hname "$h"): cannot read load1 (got '$l') — refusing"
   378	  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   379	  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
   380	}
   381	
   382	link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
   383	  local h="$1" o peer_ip want got route_nic nic
   384	  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"; nic="$(hnic "$h")"
   385	  [[ -n "$want" ]] || die_blind "$(hname "$o"): its configured MAC does not parse — refusing"
   386	  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
   387	    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
   388	  # The ARP entry ON THE NIC THE TRAFFIC WILL EGRESS. `arp -n <ip>` prints one line
   389	  # PER INTERFACE that has an entry — q holds entries for nagatha on en0, en1 AND
   390	  # en8 — so an unfiltered $4 yields a MULTI-LINE string that can never equal a
   391	  # single MAC. (Measured: this refused a perfectly good link. It is also the more
   392	  # correct check: a stale entry on the 1GbE NIC is irrelevant to the 10GbE path.)
   393	  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk -v nic='$nic' '\$5 == \"on\" && \$6 == nic {print \$4}' | head -1" | nocr | norm_mac)"
   394	  [[ -n "$got" ]] || die "$(hname "$h"): no ARP entry for $peer_ip ON $nic — the 10GbE path has not resolved the peer"
   395	  [[ "$got" == "$want" ]] \
   396	    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
   397	  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
   398	  [[ "$route_nic" == "$(hnic "$h")" ]] \
   399	    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
   400	}
   401	
   402	# --- the drain device: RESOLVED, never hardcoded (grok) ------------------------
   403	# `iostat disk0` can certify a disk the data never touched. Worse, on APFS the
   404	# volume lives on a SYNTHESIZED disk whose stats may be empty while the physical
   405	# store is saturated — a false "quiet". Resolve the module path to its PHYSICAL
   406	# store and verify iostat actually reports it.
   407	N_DISK=""; Q_DISK=""
   408	hdisk() { if [[ "$1" == n ]]; then echo "$N_DISK"; else echo "$Q_DISK"; fi; }
   409	resolve_disk() {
   410	  local h="$1" p dev
   411	  p="$(hmod "$h")"
   412	  # A FAILED `diskutil` MUST NOT silently fall back to the synthesized disk (round-5
   413	  # codex, HIGH). On APFS the volume lives on a synthesized container whose iostat
   414	  # counters can read IDLE while the physical store is saturated — so falling back to
   415	  # it is not a harmless default, it is a FALSE QUIET that certifies drainage on a
   416	  # device the data never touched. If the volume is APFS, the physical-store lookup
   417	  # must SUCCEED or the gate refuses.
   418	  dev="$(hrun "$h" "d=\$(df '$p' 2>/dev/null | awk 'NR==2{print \$1}' | sed 's|^/dev/||')
   419	[ -n \"\$d\" ] || { echo 'D:NO-DF:D'; exit 0; }
   420	info=\$(diskutil info \"\$d\" 2>/dev/null) || { echo 'D:NO-DISKUTIL:D'; exit 0; }
   421	[ -n \"\$info\" ] || { echo 'D:EMPTY-DISKUTIL:D'; exit 0; }
   422	if echo \"\$info\" | grep -q 'APFS'; then
   423	  ps=\$(echo \"\$info\" | awk -F: '/APFS Physical Store/{gsub(/[ \t]/, \"\", \$2); print \$2}' | head -1)
   424	  [ -n \"\$ps\" ] || { echo 'D:APFS-NO-STORE:D'; exit 0; }
   425	  d=\"\$ps\"
   426	fi
   427	echo \"D:\$(echo \"\$d\" | sed -E 's/s[0-9]+\$//'):D\"" | nocr | sed -n 's/.*D:\([^:]*\):D.*/\1/p' | head -1)"
   428	  # Returns non-zero rather than dying, so the CALLER decides. (The self-test runs
   429	  # each gate in a subshell to survive a refusal — and a `die` in there was invisible
   430	  # while the global it sets was discarded, so the drain then had no device and
   431	  # reported DRAIN-ERROR. The self-test was breaking its own next gate.)
   432	  if [[ ! "$dev" =~ ^disk[0-9]+$ ]]; then
   433	    log "$(hname "$h"): cannot resolve the PHYSICAL disk behind $p (got '$dev') — a drain that watches the wrong device certifies a disk the data never touched, and on APFS a synthesized disk can read idle while the physical store saturates"
   434	    return 1
   435	  fi
   436	  # It must actually REPORT: an iostat that emits nothing for this device would
   437	  # make every sample non-numeric, and the drain must never read that as quiet.
   438	  local probe
   439	  probe="$(hrun "$h" "iostat -d -w 1 -c 2 '$dev' 2>/dev/null | tail -1 | awk '{print \$3}'" | nocr)" || probe=""
   440	  if [[ ! "$probe" =~ ^[0-9]+\.?[0-9]*$ ]]; then
   441	    log "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — cannot certify drainage"
   442	    return 1
   443	  fi
   444	  if [[ "$h" == n ]]; then N_DISK="$dev"; else Q_DISK="$dev"; fi
   445	  log "  drain device on $(hname "$h"): $dev (backs $p, idle probe ${probe} MB/s)"
   446	}
   447	
   448	# --- the settle-gap bound (NOT a removal — a measured bound) -------------------
   449	# Between the client exiting and the fsync starting, the OS writes back dirty pages
   450	# FOR FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
   451	# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
   452	# ssh return-path difference, which is bounded by the round-trip time measured here.
   453	# It is NOT "removed by construction", and the pre-registration no longer says so.
   454	#
   455	# Timed in ONE process, for the same reason the transfer is. Bracketing each ssh
   456	# with two `python3 -c time.time()` calls would have charged it TWO interpreter
   457	# startups (~30 ms) and reported them as network latency — measured: it read 35 ms
   458	# for a round trip that is actually ~5 ms. The instrument's own bound would have
   459	# been wrong by 7x, in the direction that flatters nothing and confuses everything.
   460	SSH_RTT_MS=0
   461	measure_ssh_rtt() {
   462	  # A FAILED ssh must not contribute a plausible number (round-5 codex, MEDIUM): a
   463	  # fast-failing connection would report a small "bound" and flatter the settle claim.
   464	  SSH_RTT_MS="$(python3 -c '
   465	import statistics, subprocess, sys, time
   466	argv = sys.argv[1:]
   467	ts = []
   468	for _ in range(5):
   469	    t = time.monotonic()
   470	    rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
   471	    if rc != 0:
   472	        print("SSH-FAILED")
   473	        raise SystemExit
   474	    ts.append((time.monotonic() - t) * 1000.0)
   475	print(int(statistics.median(ts)))
   476	' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
   477	  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
   478	  local rtt_max=$(( SETTLE_MS / 4 ))
   479	  (( SSH_RTT_MS <= rtt_max )) \
   480	    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
   481	  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
   482	}
   483	
   484	# =============================================================================
   485	preflight() {
   486	  # RUNS=8 is the registered value. RUNS=16 is the ONLY registered escalation, and
   487	  # it may be used for exactly ONE reason: a prior session returned
   488	  # INCONCLUSIVE-UNDERPOWERED. It must NEVER be used to chase a result someone
   489	  # dislikes -- that is the p-hacking this pre-registration exists to prevent.
   490	  #
   491	  # Why it exists (round-3 grok, MEDIUM): at n=8 the >=95% order-statistic interval
   492	  # is the FULL RANGE [min,max], so ONE noisy pair with |d| >= margin blocks a null
   493	  # forever and the rig can only ever say UNDERPOWERED -- a null-incapable
   494	  # instrument is broken too, just less dangerously. At n=16 the interval is
   495	  # [d(4), d(13)] (coverage 97.9%), which tolerates three outliers per side.
   496	  [[ "$RUNS" == 8 || "$RUNS" == 16 ]] \
   497	    || die "RUNS must be 8 (registered) or 16 (the registered escalation, valid ONLY after an INCONCLUSIVE-UNDERPOWERED session) — got '$RUNS'"
   498	  if [[ "$RUNS" == 16 ]]; then
   499	    # A FLAG IS NOT A JUSTIFICATION (round-5 codex, HIGH). `UNDERPOWERED_ESCALATION=1`
   500	    # was sufficient on its own: no prior session named, none verified, "once"
   501	    # unenforced. That is a re-roll button with a serious-sounding name. The
   502	    # escalation must now POINT AT the underpowered session and the harness READS ITS
   503	    # VERDICT — the trigger is evidence on disk, not an operator's assertion.
   504	    local prior="${UNDERPOWERED_ESCALATION:-}" v
   505	    [[ -n "$prior" ]] \
   506	      || die "RUNS=16 is the escalation arm. Set UNDERPOWERED_ESCALATION=<path to the prior session dir> that returned INCONCLUSIVE-UNDERPOWERED. It buys POWER; it is NOT a re-roll."
   507	    # The trigger must be a REAL SESSION, not a directory that merely contains the right
   508	    # words (round-6, codex HIGH + grok F5: "any directory containing the expected first
   509	    # verdict line authorizes escalation; provenance, hashes, build and prior runs=8 are
   510	    # never checked"). So the prior session must carry its own DATA and MANIFEST, and
   511	    # the escalation is bound to the CONTENT of that data, not to its path.
   512	    for _f in session_verdict.txt runs.csv meta.csv staging-manifest.txt; do
   513	      [[ -f "$prior/$_f" ]] \
   514	        || die "UNDERPOWERED_ESCALATION='$prior' has no $_f — the escalation must name a REAL prior session, not a directory with the right words in it"
   515	    done
   516	    v="$(head -1 "$prior/session_verdict.txt" | sed -n 's/^SESSION VERDICT: *//p')"
   517	    [[ "$v" == "INCONCLUSIVE-UNDERPOWERED" ]] \
   518	      || die "the prior session '$prior' returned '$v', not INCONCLUSIVE-UNDERPOWERED. RUNS=16 is triggered by a POWER FAILURE and by nothing else — re-running a result you dislike at higher n is p-hacking, and this gate exists to stop it."
   519	    grep -q "binary_identity=$REGISTERED_BUILD" "$prior/staging-manifest.txt" \
   520	      || die "the prior session '$prior' was not run on the registered build $REGISTERED_BUILD — it cannot authorise an escalation"
   521	    # "Once" is bound to the DATA, not the directory: copying the session elsewhere does
   522	    # not buy a second re-roll, because the burn records the runs.csv hash.
   523	    PRIOR_RUNS_SHA="$(shasum -a 256 "$prior/runs.csv" | cut -d' ' -f1)"
   524	    if [[ -f "$REPO_ROOT/logs/ESCALATED-SESSIONS" ]] \
   525	       && grep -q "$PRIOR_RUNS_SHA" "$REPO_ROOT/logs/ESCALATED-SESSIONS"; then
   526	      die "this exact session's data (runs.csv $PRIOR_RUNS_SHA) has ALREADY authorised an escalation — see logs/ESCALATED-SESSIONS. 'Once' means once, and it is bound to the DATA, not the path."
   527	    fi
   528	    ESCALATED_FROM="$prior"
   529	    log "  escalation: RUNS=16, triggered by $prior (verified INCONCLUSIVE-UNDERPOWERED, build $REGISTERED_BUILD, runs.csv $PRIOR_RUNS_SHA)"
   530	  fi
   531	  [[ "$EXPECT_SHA" == "$REGISTERED_BUILD" ]] \
   532	    || die "EXPECT_SHA='$EXPECT_SHA' but the PRE-REGISTERED build is $REGISTERED_BUILD — a run against another build is not the registered experiment"
   533	  # The instrument must be the REVIEWED instrument: a modified harness must not be
   534	  # able to claim the reviewed commit.
   535	  git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
   536	    || die "the instrument has UNCOMMITTED changes (harness/verdict/test) — it cannot claim the reviewed commit. Commit or stash first."
   537	  # The decision rule proves itself before it grades anything — AND proves the proof
   538	  # is not vacuous. Running only the cases would let a silently-reverted fix pass
   539	  # preflight if the cases still happen to pass for another reason (round-3 grok).
   540	  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
   541	    || die "the verdict engine's OWN guard test FAILS (see $OUT_DIR/verdict-guard-test.txt) — the decision rule is broken; refusing to take data"
   542	  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
   543	    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt); the rule is not actually guarded, refusing to take data"
   544	  log "verdict-engine guard test passed ($(grep -cE ' ok$' "$OUT_DIR/verdict-guard-test.txt" || true) cases, $(grep -cE 'KILLED' "$OUT_DIR/verdict-mutations.txt" || true) mutations killed)"
   545	
   546	  local h p w want got wantb gotb
   547	  for h in n q; do
   548	    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
   549	    timer_gate "$h"                       # THE measurand's clock, proved on the rig
   550	    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
   551	      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
   552	      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
   553	    done
   554	    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
   555	    # THE SAME pgrep FAIL-OPEN AS THE QUIESCENCE GATE, IN A DUPLICATE SITE I DID NOT
   556	    # TOUCH (round-5 codex, HIGH). `if hrun ... pgrep; then die; fi` reads rc>=2 (a
   557	    # BROKEN probe, or a failed ssh) as "no daemon is running" and sails on. Every
   558	    # process probe now goes through this one rc-aware helper -- there is no second
   559	    # site left to forget.
   560	    case "$(pgrep_state "$h" blit-daemon)" in
   561	      RUNNING) die "$(hname "$h"): a blit-daemon is already running — stop it first" ;;
   562	      NONE)    : ;;
   563	      *)       die "$(hname "$h"): cannot probe for a stale blit-daemon — refusing (a gate that cannot answer must not answer 'fine')" ;;
   564	    esac
   565	    for w in large mixed small; do
   566	      want="$(fix_count "$w")"; wantb="$(fix_bytes "$w")"
   567	      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
   568	      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
   569	      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
   570	        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
   571	    done
   572	    link_gate "$h"
   573	    resolve_disk "$h" || die "$(hname "$h"): cannot establish the drain device — refusing"
   574	  done
   575	  measure_ssh_rtt
   576	  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   577	  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
   578	}
   579	
   580	write_manifest() {
   581	  local f="$OUT_DIR/staging-manifest.txt" h nb nd qb qd vh th
   582	  # Hashes computed FIRST, in the caller's shell: `die` inside $(...) exits only the
   583	  # subshell, so the old code wrote an EMPTY hash and called it provenance.
   584	  nb="$(sha256_of n "$N_BLIT")"   || die "nagatha: cannot hash $N_BLIT"
   585	  nd="$(sha256_of n "$N_DAEMON")" || die "nagatha: cannot hash $N_DAEMON"
   586	  qb="$(sha256_of q "$Q_BLIT")"   || die "q: cannot hash $Q_BLIT"
   587	  qd="$(sha256_of q "$Q_DAEMON")" || die "q: cannot hash $Q_DAEMON"
   588	  vh="$(shasum -a 256 "$VERDICT_PY" | cut -d' ' -f1)"
   589	  th="$(shasum -a 256 "$VERDICT_TEST" | cut -d' ' -f1)"
   590	  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
   591	    echo "# verdict_sha256=$vh verdict_test_sha256=$th"   # the engine grades separately: hash it too
   592	    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
   593	    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
   594	    echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
   595	    echo "# escalated_from=${ESCALATED_FROM:-none}"   # a RUNS=16 run must carry its trigger
   596	    echo "# cells=$CELLS"
   597	    echo "host,role,sha,sha256,path"
   598	    echo "nagatha,client,$EXPECT_SHA,$nb,$N_BLIT"
   599	    echo "nagatha,daemon,$EXPECT_SHA,$nd,$N_DAEMON"
   600	    echo "q,client,$EXPECT_SHA,$qb,$Q_BLIT"
   601	    echo "q,daemon,$EXPECT_SHA,$qd,$Q_DAEMON"; } > "$f"
   602	  log "staging manifest recorded (harness + verdict-engine + 4 binary hashes, every threshold)"
   603	}
   604	
   605	# --- daemons ------------------------------------------------------------------
   606	N_PID=""; Q_PID=""; TEARDOWN_FAILED=0
   607	daemon_start() {
   608	  local h="$1" cfg mod bin pid
   609	  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   610	  # The daemon's OWN pid, from $! — not `pgrep | head -1`, which picks the first of
   611	  # whatever happens to be running.
   612	  pid="$(hrun "$h" "mkdir -p '$mod' || exit 1
   613	printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg' || exit 1
   614	nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
   615	echo \"P:\$!:P\"" | nocr | sed -n 's/.*P:\([0-9][0-9]*\):P.*/\1/p' | head -1)"
   616	  [[ "$pid" =~ ^[0-9]+$ ]] || die "$(hname "$h"): daemon did not report a pid (see $mod/mm-daemon.log)"
   617	  # OWN THE PID BEFORE VALIDATING IT (round-5 codex, MEDIUM): the old code stored it
   618	  # only AFTER the alive/listening checks, so a daemon that started but failed
   619	  # validation was `die`d on while the EXIT trap did not yet know its pid — leaking a
   620	  # live daemon holding the port for the next session to trip over.
   621	  if [[ "$h" == n ]]; then N_PID="$pid"; else Q_PID="$pid"; fi
   622	  sleep 2
   623	  hrun "$h" "ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon" \
   624	    || die "$(hname "$h"): daemon pid $pid is not alive (see $mod/mm-daemon.log)"
   625	  # ALIVE is not SERVING: it must hold the port we are about to measure through.
   626	  hrun "$h" "lsof -nP -a -p $pid -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1" \
   627	    || die "$(hname "$h"): daemon pid $pid is NOT LISTENING on $PORT (see $mod/mm-daemon.log)"
   628	  log "$(hname "$h") daemon up (pid $pid, listening) on $(hip "$h"):$PORT"
   629	}
   630	# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
   631	# handshake reached some listener's backlog — not that the daemon speaks blit).
   632	smoke() {
   633	  local h="$1" o
   634	  o="$(other "$h")"
   635	  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 \
   636	    || die "$(hname "$o"): cannot stage the smoke fixture"
   637	  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
   638	    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
   639	    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
   640	  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
   641	  log "smoke ok: $(hname "$h") daemon serves blit"
   642	}
   643	daemon_stop() {
   644	  local h="$1" pid state
   645	  if [[ "$h" == n ]]; then pid="$N_PID"; else pid="$Q_PID"; fi
   646	  [[ -n "$pid" ]] || return 0
   647	  hrun "$h" "kill $pid 2>/dev/null || true
   648	for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done
   649	if ps -p $pid >/dev/null 2>&1; then kill -9 $pid 2>/dev/null || true; sleep 1; fi" >/dev/null 2>&1 || true
   650	  # A teardown that cannot be VERIFIED is a failure, not a success. The old probe
   651	  # called a FAILED ssh "GONE".
   652	  state="$(hrun "$h" "if ps -p $pid >/dev/null 2>&1; then echo 'S:ALIVE:S'; else echo 'S:GONE:S'; fi" \
   653	    | nocr | sed -n 's/.*S:\([A-Z]*\):S.*/\1/p' | head -1)" || state=""
   654	  if [[ "$state" != GONE ]]; then
   655	    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown or could not be probed (got '$state') — port $PORT may still be held"
   656	    TEARDOWN_FAILED=1
   657	    touch "$OUT_DIR/TEARDOWN-FAILED"
   658	    return 1
   659	  fi
   660	  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
   661	}
   662	cleanup() {
   663	  daemon_stop n || true
   664	  daemon_stop q || true
   665	  rm -rf "$MUX" 2>/dev/null || true
   666	  if [[ "$TEARDOWN_FAILED" == 1 ]]; then
   667	    log "ERROR: a daemon survived teardown — see $OUT_DIR/TEARDOWN-FAILED. Clean it up before the next session."
   668	  fi
   669	}
   670	trap cleanup EXIT
   671	
   672	# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
   673	RUN_DRAIN=""; RUN_COLD=""
   674	drain_host() {   # $1 = host. Echoes drained_<n>x2s | DRAIN-TIMEOUT | DRAIN-ERROR
   675	  local h="$1" dev
   676	  dev="$(hdisk "$h")"
   677	  [[ -n "$dev" ]] || { echo DRAIN-ERROR; return 0; }
   678	  # A FAILED iostat must not certify quiet even when it printed a parseable line
   679	  # (round-5 codex, HIGH: a numeric line followed by a NONZERO EXIT still accumulated
   680	  # "quiet" samples). The exit code is now checked BEFORE the value is used.
   681	  hrun "$h" "quiet=0
   682	for i in \$(seq 1 $DRAIN_ITERS); do
   683	  out=\$(iostat -d -w 2 -c 2 '$dev' 2>/dev/null); rc=\$?
   684	  if [ \$rc -ne 0 ]; then echo DRAIN-ERROR; exit 0; fi
   685	  w=\$(echo \"\$out\" | tail -1 | awk '{print \$3}')
   686	  case \"\$w\" in
   687	    ''|*[!0-9.]*) echo DRAIN-ERROR; exit 0 ;;   # non-numeric must NEVER certify quiet
   688	  esac
   689	  ok=\$(awk -v w=\"\$w\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   690	  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   691	  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
   692	done
   693	echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
   694	}
   695	prep_run() {   # $1 = dest host
   696	  local dh="$1" cn=ok cq=ok out
   697	  # Purge BOTH ends first — the purge itself dirties the disk, so a drain certified
   698	  # BEFORE it proves nothing.
   699	  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   700	  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   701	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   702	  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
   703	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
   704	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   705	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   706	}
   707	
   708	# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
   709	RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
   710	fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
   711	  local out
   712	  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
   713	  #
   714	  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
   715	  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
   716	  # RAN. Round 6 then found the repair was still not provable: `sleep` is
   717	  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
   718	  # counted files — so a no-op `sleep` would pass while the log narrated "settle
   719	  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
   720	  #
   721	  # A protection that cannot be OBSERVED is not a protection. The settle now happens
   722	  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
   723	  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
   724	  # to shadow, no exit status left to discard, and no narration left to trust.
   725	  out="$(hrun "$1" "python3 - '$SETTLE_SEC' '$2' <<'PYEOF'
   726	import os, sys, time
   727	settle = float(sys.argv[1])
   728	p = sys.argv[2]
   729	t0 = time.monotonic()
   730	time.sleep(settle)
   731	settled_ms = int((time.monotonic() - t0) * 1000)
   732	if not os.path.isdir(p):
   733	    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
   734	    raise SystemExit
   735	t = time.monotonic()
   736	files = 0
   737	nbytes = 0
   738	for root, _d, fs in os.walk(p):
   739	    for name in fs:
   740	        fp = os.path.join(root, name)
   741	        nbytes += os.path.getsize(fp)
   742	        fd = os.open(fp, os.O_RDONLY)
   743	        os.fsync(fd)
   744	        os.close(fd)
   745	        files += 1
   746	print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
   747	PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
   748	  echo "${out:-NA 0 0 0}"
   749	}
   750	# The settle actually elapsed, on the destination's own clock. Anything else voids.
   751	settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
   752	
   753	# --- one timed run ------------------------------------------------------------
   754	RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
   755	timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
   756	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
   757	  bin="$(hblit "$ih")"
   758	  prep_run "$dh"
   759	  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
   760	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   761	  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
   762	  RUN_VALID=yes
   763	  wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
   764	  # The equal settle is the ONLY thing standing between this rig and a free-writeback
   765	  # artifact that REVERSES SIGN WITH DIRECTION — i.e. that can manufacture P1 out of
   766	  # nothing. It has already been silently dead once. If it did not measurably elapse,
   767	  # the row is not a fast row; it is a VOID row.
   768	  if ! settle_ok "$RUN_SETTLED"; then
   769	    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
   770	    RUN_VALID=no
   771	  fi
   772	  if [[ "$RUN_FLUSH" == NA ]]; then
   773	    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
   774	    RUN_VALID=no; RUN_FLUSH=0
   775	  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
   776	    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
   777	    RUN_VALID=no
   778	  fi
   779	  # A negative or absurd transfer time means the CLOCK failed, not that the transfer
   780	  # was fast. It must never enter the data.
   781	  if [[ ! "$RUN_MS" =~ ^[0-9]+$ ]] || (( RUN_MS < 1 )); then
   782	    log "  VOID: transfer timer returned '$RUN_MS' — the clock failed (round 2's killer). NOT a fast run."
   783	    RUN_VALID=no; RUN_MS=0
   784	  fi
   785	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   786	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
   787	}
   788	
   789	# --- arms ---------------------------------------------------------------------
   790	# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
   791	# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
   792	# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
   793	# above is what makes a wrong path fatal instead of silently free.
   794	CUR_W=""; CUR_FLAG=""
   795	arm_srcinit() {
   796	  local sh="$1" dh="$2" run="$3"
   797	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
   798	            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
   799	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   800	}
   801	arm_destinit() {
   802	  local sh="$1" dh="$2" run="$3"
   803	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
   804	            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
   805	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   806	}
   807	
   808	CSV="$OUT_DIR/runs.csv"
   809	META="$OUT_DIR/meta.csv"
   810	
   811	run_pair_loop() {
   812	  local cell="$1" sh="$2" dh="$3"
   813	  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   814	  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   815	  while (( valid < RUNS && attempts < max )); do
   816	    attempts=$(( attempts + 1 ))
   817	    local order pair=yes rowA="" rowB="" arm aname init rid run
   818	    if (( slot % 2 )); then order="A B"; else order="B A"; fi
   819	    for arm in $order; do
   820	      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   821	      rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
   822	      if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"
   823	      else arm_destinit "$sh" "$dh" "$run"; fi
   824	      [[ "$RUN_VALID" == yes ]] || pair=no
   825	      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   826	      if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
   827	      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   828	    done
   829	    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   830	    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   831	    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
   832	  done
   833	  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
   834	  else echo "$cell,$attempts,yes" >> "$META"; fi
   835	}
   836	
   837	SESSION_VOID_REASON=""
   838	# The end-load is a CONDITION OF THE SESSION, not a log line. A mid-session load
   839	# spike is exactly the contamination the start gate exists to prevent, and until now
   840	# it could not void anything: the code logged `load1 (end)` and computed a verdict
   841	# anyway, while the comment claimed a session "can void on it" (round-3 grok, HIGH —
   842	# a doc claim the code did not honour, which is the defect class this whole review
   843	# exists to kill).
   844	end_load_gate() {
   845	  local h l ok
   846	  for h in n q; do
   847	    l="$(load1 "$h")" || l=""
   848	    if [[ ! "$l" =~ ^[0-9]+\.?[0-9]*$ ]]; then
   849	      SESSION_VOID_REASON="end-load on $(hname "$h") could not be read (got '$l') — a session whose end conditions are unknown cannot be graded"
   850	      return
   851	    fi
   852	    ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   853	    if [[ "$ok" != 1 ]]; then
   854	      SESSION_VOID_REASON="end-load on $(hname "$h") is $l (> $LOAD_MAX) — the machine was NOT quiet at the end of the session, so a contaminant may have entered the timed windows"
   855	      return
   856	    fi
   857	  done
   858	}
   859	
   860	compute_verdicts() {
   861	  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" \
   862	  CONTROL_CELLS="$CONTROL_CELLS" REGISTERED_CELLS="$REGISTERED_CELLS" \
   863	  REQUIRED_PAIRS="$RUNS" SESSION_VOID_REASON="$SESSION_VOID_REASON" \
   864	  python3 "$VERDICT_PY" \
   865	    "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/paired.csv" \
   866	    "$OUT_DIR/verdicts.csv" "$OUT_DIR/session_verdict.txt"
   867	}
   868	
   869	# =============================================================================
   870	# SELFTEST — exercise every gate for real, take NO data.
   871	#
   872	# This exists because round 1's "fixes" were never executed: I ran `bash -n` and
   873	# shipped a preflight that COULD NOT SUCCEED (grep -c's exit 1, gawk's strtonum).
   874	# A syntax check is not an execution.
   875	# =============================================================================
   876	SELFTEST_FIRED=0; SELFTEST_BROKEN=0
   877	# A gate can end in three states, and the old self-test collapsed two of them
   878	# (round-5 codex, HIGH: "every nonzero result — including a BROKEN probe — is labeled
   879	# [FIRED], and the self-test exits zero"). That is the same fail-open it exists to
   880	# hunt, committed by the hunter:
   881	#
   882	#   [OK]     the probe answered and the condition holds.
   883	#   [FIRED]  the probe answered and the condition is genuinely UNMET (codex is
   884	#            running, Time Machine is on). The gate WORKS. Not a self-test failure.
   885	#   [BROKEN] the probe could not answer at all. THE GATE IS BLIND, and the self-test
   886	#            FAILS (exit 1) — a blind gate is exactly what fails open on the night.
   887	#
   888	# The two are told apart by the refusal text: every "cannot answer" die() in this file
   889	# says so in the words below, and every genuine-condition die() does not.
   890	# A REPORTER, never a gate: it must always return 0, or `set -e` aborts the sweep at
   891	# the first refusal and the remaining gates go untested (which is exactly what it did
   892	# the first time it ran — the self-test could not even test itself).
   893	gate_probe() {
   894	  local label="$1"; shift
   895	  local err rc=0
   896	  err="$( { "$@"; } 2>&1 )" || rc=1
   897	  if (( rc == 0 )); then
   898	    log "  [OK]     $label — answers, and the condition holds"
   899	  elif grep -q 'PROBE-BLIND' <<<"$err"; then
   900	    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 ))
   901	    log "  [BROKEN] $label — THE PROBE COULD NOT ANSWER. A blind gate fails open on the night."
   902	  else
   903	    SELFTEST_FIRED=$(( SELFTEST_FIRED + 1 ))
   904	    log "  [FIRED]  $label — the gate REFUSED a genuinely unmet condition. It works."
   905	  fi
   906	  # Never hide what the gate said — including its own evidence on success.
   907	  [[ -n "$err" ]] && sed 's/^/           | /' <<<"$err" | tee -a "$OUT_DIR/bench.log" >&2
   908	  return 0
   909	}
   910	
   911	# The fsync/settle path, exercised for real on a throwaway tree. It is the durability
   912	# measurement AND the equal-settle window — the two things that once manufactured P1 —
   913	# and the self-test never touched them.
   914	selftest_fsync() {
   915	  local h="$1" d ms files bytes settled
   916	  d="$(hmod "$h")/selftest_${SESSION_TAG}"
   917	  hrun "$h" "rm -rf '$d' && mkdir -p '$d' && printf 'aaaa' > '$d/a' && printf 'bb' > '$d/b'" \
   918	    || { log "  [BROKEN] fsync/settle — cannot stage a probe tree"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); return 1; }
   919	  read -r ms files bytes settled <<<"$(fsync_tree "$h" "$d")"
   920	  hrun "$h" "rm -rf '$d'" >/dev/null 2>&1 || true
   921	  if [[ "$ms" == NA || "$files" != 2 || "$bytes" != 6 ]]; then
   922	    log "  [BROKEN] fsync/settle — walk returned ms=$ms files=$files bytes=$bytes, want 2 files / 6 bytes"
   923	    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
   924	  fi
   925	  # THE SETTLE MUST BE PROVED, NOT NARRATED (round-6, both reviewers). The old check
   926	  # counted files and then LOGGED "settle included" — which is a sentence, not an
   927	  # assertion. It would have passed with the settle stone dead, which is precisely how
   928	  # the settle stayed dead for three revisions.
   929	  if ! settle_ok "$settled"; then
   930	    log "  [BROKEN] fsync/settle — THE SETTLE DID NOT ELAPSE: measured ${settled}ms, want >= ${SETTLE_MS}ms"
   931	    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
   932	  fi
   933	  log "  [OK]     fsync/settle — 2 files/6 bytes walked in ${ms}ms; settle MEASURED at ${settled}ms (>= ${SETTLE_MS}ms), counts VERIFIED"
   934	}
   935	
   936	selftest() {
   937	  local h
   938	  log "SELFTEST — exercising the gates for real. No transfer, NO DATA."
   939	  log "instrument: harness=$HARNESS_SHA256"
   940	  log "--- the verdict engine's own guard test (incl. mutation proof) ---"
   941	  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
   942	    || die "the verdict guard test FAILS (see $OUT_DIR/verdict-guard-test.txt)"
   943	  log "  $(grep -E '^[0-9]+/[0-9]+ cases passed' "$OUT_DIR/verdict-guard-test.txt")"
   944	  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
   945	    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt)"
   946	  log "  $(grep -E '^[0-9]+/[0-9]+ mutations killed' "$OUT_DIR/verdict-mutations.txt") — every reverted fix is caught"
   947	  for h in n q; do
   948	    log "--- $(hname "$h") ---"
   949	    gate_probe "timer         (the measurand's clock)" timer_gate "$h"
   950	    gate_probe "quiescence    (codex/cargo/rustc)"     quiescence_gate "$h"
   951	    gate_probe "time machine  (running OR enabled)"    timemachine_gate "$h"
   952	    gate_probe "spotlight     (mds_stores CPU)"        spotlight_gate "$h"
   953	    gate_probe "load  start   (load1 <= $LOAD_MAX)"      load_gate "$h"
   954	    gate_probe "link          (ARP on the egress NIC + 10GbE route)" link_gate "$h"
   955	    # NOT through gate_probe: it runs its argument in a SUBSHELL (so a `die` cannot
   956	    # abort the sweep), and resolve_disk's whole job is to SET a global. Called there,
   957	    # the assignment was discarded and the drain loop below then had no device and
   958	    # reported DRAIN-ERROR — the self-test was breaking its own next gate and blaming
   959	    # the harness.
   960	    if resolve_disk "$h"; then log "  [OK]     drain device  (resolved via the APFS physical store)"
   961	    else log "  [BROKEN] drain device  — could not resolve the physical disk"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); fi
   962	    # The paths the old self-test claimed and did not run (round-5 codex, HIGH):
   963	    gate_probe "purge         (sudo -n, or every run reads WARM)" hrun "$h" "sudo -n /usr/sbin/purge"
   964	    case "$(pgrep_state "$h" blit-daemon)" in
   965	      NONE)    log "  [OK]     stale daemon  (rc-aware probe: none running)" ;;
   966	      RUNNING) log "  [FIRED]  stale daemon  (one IS running — the gate would refuse)"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
   967	      *)       log "  [BROKEN] stale daemon  — the probe could not answer"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
   968	    esac
   969	    # DRAIN-TIMEOUT is a genuinely busy disk (the gate WORKING); DRAIN-ERROR is a blind
   970	    # probe. Scoring them the same made the classification untrustworthy (grok r6, F7).
   971	    local dr; dr="$(drain_host "$h")"
   972	    case "$dr" in
   973	      drained*)      log "  [OK]     drain loop    ($dr)" ;;
   974	      DRAIN-TIMEOUT) log "  [FIRED]  drain loop    — the disk is genuinely busy; the gate would void the pair"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
   975	      *)             log "  [BROKEN] drain loop    — the probe could not answer ('$dr')"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
   976	    esac
   977	    selftest_fsync "$h"
   978	    log "  [--]     mac parse (no gawk strtonum): $(hmac "$h") -> $(hmac "$h" | norm_mac)"
   979	  done
   980	  SESSION_VOID_REASON=""; end_load_gate
   981	  if [[ -z "$SESSION_VOID_REASON" ]]; then
   982	    log "  [OK]     end-load gate (both Macs under $LOAD_MAX; it CAN void a session)"
   983	  elif [[ "$SESSION_VOID_REASON" == *"could not be read"* ]]; then
   984	    # An UNREADABLE end-load is a blind probe, not a busy machine (grok r6, F7).
   985	    log "  [BROKEN] end-load gate — $SESSION_VOID_REASON"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1))
   986	  else
   987	    log "  [FIRED]  end-load gate — $SESSION_VOID_REASON"; SELFTEST_FIRED=$((SELFTEST_FIRED+1))
   988	  fi
   989	  measure_ssh_rtt
   990	  log ""
   991	  log "SELFTEST: $SELFTEST_FIRED gate(s) refused a genuinely unmet condition; $SELFTEST_BROKEN blind."
   992	  log "NOT exercised here (they need a real transfer): daemon start/lsof/teardown, the"
   993	  log "smoke transfer, the ABBA pair loop, pair-voiding, and the manifest. PREFLIGHT_ONLY=1"
   994	  log "covers the manifest and the build-provenance gates. This self-test does NOT claim"
   995	  log "to run every gate — the previous one did, and it was not true."
   996	  log "THIS IS NOT CLEARANCE TO TAKE DATA. The review is."
   997	  if (( SELFTEST_BROKEN > 0 )); then
   998	    log "SELFTEST FAILED: $SELFTEST_BROKEN gate(s) are BLIND."
   999	    exit 1
  1000	  fi
  1001	  log "SELFTEST PASSED: every gate exercised here can answer."
  1002	}
  1003	
  1004	main() {
  1005	  if [[ "$SELFTEST" == 1 ]]; then
  1006	    EXPECT_SHA="${EXPECT_SHA:-$REGISTERED_BUILD}"
  1007	    HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
  1008	    HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
  1009	    selftest
  1010	    exit 0
  1011	  fi
  1012	  preflight
  1013	  write_manifest
  1014	  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
  1015	    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
  1016	    exit 0
  1017	  fi
  1018	  # "Once" means once: burn the escalation the moment it is used, so the same
  1019	  # underpowered session cannot authorise a second, third, nth re-roll.
  1020	  if [[ -n "$ESCALATED_FROM" ]]; then
  1021	    echo "escalated to $SESSION_TAG (RUNS=$RUNS) on $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  1022	      >> "$ESCALATED_FROM/ESCALATED"
  1023	    # Bound to the DATA, so a copy of the session cannot buy a second re-roll.
  1024	    echo "$PRIOR_RUNS_SHA $ESCALATED_FROM -> $SESSION_TAG" >> "$REPO_ROOT/logs/ESCALATED-SESSIONS"
  1025	  fi
  1026	  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
  1027	  echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
  1028	  echo "cell,pairs_attempted,complete" > "$META"
  1029	  daemon_start n; daemon_start q
  1030	  smoke n; smoke q
  1031	
  1032	  local carrier w flag cell
  1033	  for w in mixed large small; do
  1034	    for carrier in tcp grpc; do
  1035	      if [[ "$carrier" == grpc ]]; then flag="--force-grpc"; else flag=""; fi
  1036	      CUR_W="$w"; CUR_FLAG="$flag"
  1037	      cell="nq_${carrier}_${w}"; if [[ ",$CELLS," == *",$cell,"* ]]; then run_pair_loop "$cell" n q; fi
  1038	      cell="qn_${carrier}_${w}"; if [[ ",$CELLS," == *",$cell,"* ]]; then run_pair_loop "$cell" q n; fi
  1039	    done
  1040	  done
  1041	
  1042	  # End-load BEFORE the verdict is computed, and it can VOID the session.
  1043	  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
  1044	  end_load_gate
  1045	  if [[ -n "$SESSION_VOID_REASON" ]]; then
  1046	    log "ERROR: SESSION VOID — $SESSION_VOID_REASON"
  1047	    touch "$OUT_DIR/SESSION-VOID"
  1048	  fi
  1049	  compute_verdicts
  1050	  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
  1051	  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
  1052	  log "=== PAIRED STATS (the rule is graded on these) ==="
  1053	  column -t -s, "$OUT_DIR/paired.csv" | tee -a "$OUT_DIR/bench.log"
  1054	  log "=== SESSION VERDICT (computed by the harness from the PRE-REGISTERED rule) ==="
  1055	  cat "$OUT_DIR/session_verdict.txt" | tee -a "$OUT_DIR/bench.log"
  1056	  log "runs: $CSV"
  1057	}
  1058	
  1059	# EXPECT_SHA is required for anything that touches the rig's binaries; SELFTEST
  1060	# supplies the registered default so the gates can be exercised without ceremony.
  1061	if [[ "$SELFTEST" != 1 ]]; then
  1062	  EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"
  1063	  HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
  1064	  HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
  1065	fi
  1066	main "$@"

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '1,380p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# ⛔ NOT CLEARED TO RUN — REWORKED IN ROUND 3, REVIEW NOT YET PASSED ⛔
     4	#
     5	# The round-3 rework (this file) addresses all 15 findings from codex round 2 and
     6	# grok's second opinion. It has NOT been reviewed. The review is the gate, not the
     7	# rework: three rounds running, every revision of this instrument has shipped a
     8	# defect capable of a false claim, and two of them were introduced BY THE REWORK
     9	# THAT FIXED THE PREVIOUS ONE.
    10	#
    11	#   .review/results/macmac-harness-r2.gpt-verdict.md    (codex, 12 findings)
    12	#   .review/results/macmac-harness-r2.grok-verdict.md   (grok, +3 findings)
    13	#
    14	# Clearing it: land the round-3 review, adjudicate, and delete this block plus the
    15	# CLEARED_BY_REVIEW guard below. Until then `SELFTEST=1` and `PREFLIGHT_ONLY=1`
    16	# work (they take NO data); a timed run refuses.
    17	# =============================================================================
    18	# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
    19	# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (rev 4)
    20	# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
    21	#
    22	# WHY THIS RIG EXISTS
    23	# -------------------
    24	# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
    25	# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
    26	# cell. It answers ONE question, SCOPED TO THIS PAIR:
    27	#
    28	#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
    29	#
    30	#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
    31	#     "platform residue" that can be waived; code-level hypotheses strengthen. It
    32	#     leaves macOS/APFS and host x role explanations OPEN.
    33	#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
    34	#     "Windows is required", but does NOT prove it: it could equally be a
    35	#     property of these two machines, their disks, or this macOS version.
    36	#
    37	# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
    38	# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
    39	# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
    40	# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
    41	# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
    42	# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
    43	#
    44	# THE INSTRUMENT IS THE RISK. Three claims in this project have been retracted to
    45	# harness bugs, and this harness alone has now had 20 defects found across two
    46	# reviews. What round 2 caught, and what is fixed here:
    47	#
    48	#   * THE TIMER WAS MEASURING FSYNC NOISE. It captured time.monotonic() in TWO
    49	#     separate `python3 -c` processes and subtracted them. On macOS that clock is
    50	#     PROCESS-RELATIVE: a 1000 ms sleep measured -1 ms on nagatha and 2 ms on q
    51	#     (measured; yes, negative). Every `ms` row would have been ~= fsync_ms alone,
    52	#     and the invariance ratio — THE ENTIRE MEASURAND — would have been computed on
    53	#     fsync noise, which can manufacture or mask a one-directional effect at will.
    54	#     The repo ALREADY documents this trap (bench_otp12_zoey.sh:116 uses time.time()
    55	#     precisely because monotonic is wrong across processes) and I reintroduced it
    56	#     anyway. Now: ONE process times itself and spawns the client (time_argv), and
    57	#     PREFLIGHT PROVES IT on both hosts against a known sleep before any data.
    58	#   * The preflight COULD NOT SUCCEED: `grep -c` exits 1 on no match, so a CLEAN
    59	#     binary tripped the dirty-marker probe and died; and norm_mac used gawk's
    60	#     strtonum(), absent from stock macOS awk. The round-1 "fixes" were never
    61	#     executed — I ran `bash -n`, not the gates. Every gate below is now exercised
    62	#     by SELFTEST=1, which runs them for real.
    63	#   * Gates FAILED OPEN: pgrep errors read as "quiet"; a failed `top` read as 0%
    64	#     CPU and a late idle sample could overwrite a busy one; non-numeric `iostat`
    65	#     read as zero and CERTIFIED drainage; the drain watched a hardcoded `disk0`
    66	#     that the data need never touch (grok); `die` inside $(...) exited only the
    67	#     subshell, so an empty hash still landed. Every probe is now sentinel-framed,
    68	#     rc-aware, and fails CLOSED.
    69	#
    70	# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
    71	# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
    72	# one ssh), so dispatch is outside the window by construction.
    73	#
    74	# Usage:
    75	#   SELFTEST=1       bash scripts/bench_otp12pf_mac.sh   # exercise every gate, no data
    76	#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    77	#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh # the run (needs review clearance)
    78	# =============================================================================
    79	set -euo pipefail
    80	
    81	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    82	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    83	SELF="${BASH_SOURCE[0]}"
    84	VERDICT_PY="$SCRIPT_DIR/otp12pf_mac_verdict.py"
    85	VERDICT_TEST="$SCRIPT_DIR/otp12pf_mac_verdict_test.py"
    86	
    87	SELFTEST="${SELFTEST:-0}"
    88	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
    89	
    90	# The review is the gate. A timed run refuses until round 3 is adjudicated; the
    91	# no-data modes stay available so the gates can be exercised.
    92	if [[ "$SELFTEST" != 1 && "$PREFLIGHT_ONLY" != 1 && "${CLEARED_BY_REVIEW:-0}" != 1 ]]; then
    93	  echo "REFUSING: this harness was reworked in round 3 and has NOT passed review." >&2
    94	  echo "Every previous revision shipped a defect capable of a false claim, and two" >&2
    95	  echo "were introduced by the rework that fixed the last one. Land the round-3" >&2
    96	  echo "review first. SELFTEST=1 and PREFLIGHT_ONLY=1 take no data and still run." >&2
    97	  exit 2
    98	fi
    99	
   100	# The pre-registered build. Not overridable by accident: a run against an
   101	# unregistered build is not the registered experiment.
   102	REGISTERED_BUILD="f35702a"
   103	
   104	# --- nagatha: LOCAL end (driver) ---------------------------------------------
   105	N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
   106	N_NIC="${N_NIC:-en11}"
   107	N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
   108	N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
   109	N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
   110	N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
   111	N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
   112	
   113	# --- q: REMOTE end ------------------------------------------------------------
   114	Q_SSH="${Q_SSH:-michael@q}"
   115	Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
   116	Q_NIC="${Q_NIC:-en8}"
   117	Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
   118	Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
   119	Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
   120	Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
   121	Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
   122	
   123	PORT="${PORT:-9031}"
   124	RUNS="${RUNS:-8}"
   125	
   126	# =============================================================================
   127	# THE REGISTERED CONSTANTS. **NOT OVERRIDABLE.**
   128	#
   129	# Round-5 (codex, BLOCKER): these were `${VAR:-default}`, so the pre-registered
   130	# decision rule could be edited FROM THE COMMAND LINE — `DELTA_REF_MS=240` turned a
   131	# RIG-VOID into a VANISHES. A pre-registration that the operator can retune, after
   132	# the data exists, in the direction of the answer they want, IS NOT A
   133	# PRE-REGISTRATION AT ALL.
   134	#
   135	# They are literals, and the harness REFUSES to start if one is merely PRESENT in the
   136	# environment — a deviation must be loud, never silently ignored. The check reads the
   137	# environment BEFORE the assignments below, or an override would be masked by the
   138	# very line meant to pin it.
   139	# =============================================================================
   140	_overrides=""
   141	for _v in SETTLE_MS LOAD_MAX DRAIN_ITERS DRAIN_QUIET DRAIN_MBPS DELTA_REF_MS TIMER_TOLERANCE_MS; do
   142	  [[ -n "${!_v+set}" ]] && _overrides="$_overrides $_v=${!_v}"
   143	done
   144	if [[ -n "$_overrides" ]]; then
   145	  echo "REFUSING: the pre-registered constants are NOT tunable, and these are set in the" >&2
   146	  echo "environment:$_overrides" >&2
   147	  echo "A rule the operator can retune after seeing the data is not a pre-registration." >&2
   148	  echo "To change one, amend docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md and" >&2
   149	  echo "put it back through review. That is the entire point of the document." >&2
   150	  exit 2
   151	fi
   152	
   153	SETTLE_MS=250              # equal pre-fsync window on BOTH arms
   154	# Computed ONCE, HERE, at top level — and this line is load-bearing history.
   155	#
   156	# It used to be computed inline as `sleep $(awk ... 'BEGIN{printf \"%.3f\", m/1000}')`
   157	# INSIDE the double-quoted hrun string. A command substitution is parsed FRESH by
   158	# bash, so those `\"` escapes — which are correct for hrun's two-level strings — were
   159	# literal backslashes to awk. **The awk errored on EVERY call, `sleep` got an empty
   160	# argument and FAILED, and the old code ignored its exit status because the python
   161	# walk that followed supplied the status.**
   162	#
   163	# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
   164	# it. And 24660ae is the commit that added it TO FIX the free-writeback asymmetry
   165	# that reverses sign with direction — the artifact judged capable of MANUFACTURING a
   166	# one-directional P1 out of nothing. The pre-registration has claimed an equal settle
   167	# on both arms through revisions 3, 4 and 5. It was never applied.
   168	#
   169	# Found only by EXECUTING it (round-5 codex flagged the ignored exit status; running
   170	# it showed the status was ALWAYS failure). `bash -n` sees nothing here.
   171	SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
   172	[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
   173	LOAD_MAX=3.0               # start AND end load1 bar on both Macs
   174	DRAIN_ITERS=60
   175	DRAIN_QUIET=3
   176	DRAIN_MBPS=2               # destination disk must be below this to start a window
   177	DELTA_REF_MS=230           # rig W's measured Delta_P1 — THE reference effect
   178	TIMER_TOLERANCE_MS=120     # the timer self-test's allowed error on a 1000 ms sleep
   179	
   180	# The REGISTERED cell set. The verdict engine requires ALL of them present and
   181	# complete: a partial set that is merely filtered lets a ONE-CELL run emit
   182	# "VANISHES" while claiming both cells vanished (codex r2 BLOCKER 1).
   183	REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   184	CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   185	VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
   186	CELLS="$REGISTERED_CELLS"
   187	
   188	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   189	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
   190	ESCALATED_FROM=""          # set only by the verified RUNS=16 escalation
   191	PRIOR_RUNS_SHA=""          # the data hash the escalation is bound to
   192	
   193	MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
   194	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   195	         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
   196	qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
   197	
   198	mkdir -p "$OUT_DIR/blit-logs"
   199	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
   200	die() { log "FATAL: $*"; exit 1; }
   201	# A gate that CANNOT ANSWER is BLIND, and blindness is what fails open on the night.
   202	# It is marked EXPLICITLY here, never inferred from the wording of a message —
   203	# inferring it from prose is how a blind timer came to be scored as a working gate.
   204	die_blind() { log "FATAL[PROBE-BLIND]: $*"; exit 1; }
   205	nocr() { tr -d '\r'; }
   206	
   207	# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
   208	# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
   209	# falls through to the wrong host (the trap the Linux harness documents).
   210	# `bash -c` locally pins the inner shell so local and remote parse identically.
   211	# pipefail is set in BOTH children: without it a failed probe at the head of a
   212	# pipeline is masked by a successful `tail`/`awk` and the gate reads "fine".
   213	hrun() {
   214	  local h="$1"; shift
   215	  local cmd="set -o pipefail
   216	$*"
   217	  if [[ "$h" == n ]]; then bash -c "$cmd"; else qssh "bash -c $(printf '%q' "$cmd")"; fi
   218	}
   219	hblit()   { if [[ "$1" == n ]]; then echo "$N_BLIT";   else echo "$Q_BLIT";   fi; }
   220	hdaemon() { if [[ "$1" == n ]]; then echo "$N_DAEMON"; else echo "$Q_DAEMON"; fi; }
   221	hmod()    { if [[ "$1" == n ]]; then echo "$N_MODULE"; else echo "$Q_MODULE"; fi; }
   222	hip()     { if [[ "$1" == n ]]; then echo "$N_IP";     else echo "$Q_IP";     fi; }
   223	hnic()    { if [[ "$1" == n ]]; then echo "$N_NIC";    else echo "$Q_NIC";    fi; }
   224	hmac()    { if [[ "$1" == n ]]; then echo "$N_MAC";    else echo "$Q_MAC";    fi; }
   225	hname()   { if [[ "$1" == n ]]; then echo nagatha;     else echo q;           fi; }
   226	other()   { if [[ "$1" == n ]]; then echo q;           else echo n;           fi; }
   227	
   228	# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
   229	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   230	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   231	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   232	fix_count() { case "$1" in large) echo $FIX_COUNT_large;; mixed) echo $FIX_COUNT_mixed;; small) echo $FIX_COUNT_small;; esac; }
   233	fix_bytes() { case "$1" in large) echo $FIX_BYTES_large;; mixed) echo $FIX_BYTES_mixed;; small) echo $FIX_BYTES_small;; esac; }
   234	
   235	# =============================================================================
   236	# THE TIMER. One process times itself AND spawns the client, so the interval is
   237	# measured by a single clock and python's startup cost falls outside it.
   238	#
   239	# NEVER bracket a command with two separate `python3 -c 'time.monotonic()'` calls:
   240	# on macOS that clock is PROCESS-RELATIVE and the difference is garbage (measured:
   241	# -1 ms and 2 ms for a 1000 ms sleep). bench_otp12_zoey.sh:116 already said so.
   242	# =============================================================================
   243	time_argv() {   # $1 = host; rest = argv. Echoes "MS,RC" or "" on a broken probe.
   244	  local h="$1"; shift
   245	  local qa="" a
   246	  for a in "$@"; do qa="$qa $(printf '%q' "$a")"; done
   247	  hrun "$h" "python3 - $qa <<'PYEOF'
   248	import subprocess, sys, time
   249	argv = [a for a in sys.argv[1:] if a]          # an empty flag must not become argv
   250	err = open('/tmp/mm-client.err', 'wb')
   251	t = time.monotonic()
   252	rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=err)
   253	ms = int((time.monotonic() - t) * 1000)
   254	err.close()
   255	print('R:%d,%d:R' % (ms, rc))
   256	PYEOF" | nocr | sed -n 's/.*R:\(-\{0,1\}[0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1
   257	}
   258	
   259	# The gate that makes the timer bug unshippable: prove the clock on the rig,
   260	# against a known interval, before any data is taken.
   261	timer_gate() {
   262	  local h="$1" out ms rc lo hi
   263	  out="$(time_argv "$h" /bin/sleep 1)"
   264	  [[ "$out" == *,* ]] || die_blind "$(hname "$h"): the timer probe returned nothing — refusing"
   265	  ms="${out%%,*}"; rc="${out##*,}"
   266	  [[ "$rc" == 0 ]] || die_blind "$(hname "$h"): the timer probe's own child exited $rc"
   267	  lo=$(( 1000 - TIMER_TOLERANCE_MS )); hi=$(( 1000 + TIMER_TOLERANCE_MS ))
   268	  if (( ms < lo || ms > hi )); then
   269	    die "$(hname "$h"): THE TIMER IS LYING — a 1000 ms sleep measured ${ms} ms (allowed ${lo}-${hi}).
   270	This is the round-2 killer: cross-process time.monotonic() on macOS is PROCESS-RELATIVE and
   271	read -1 ms / 2 ms for this exact sleep. Every row would be fsync noise. REFUSING to take data."
   272	  fi
   273	  log "  timer ok on $(hname "$h"): a 1000 ms sleep measures ${ms} ms"
   274	}
   275	
   276	# --- provenance ---------------------------------------------------------------
   277	# `die` inside $(...) exits only the SUBSHELL, so the outer command substitution
   278	# succeeds with an empty value. These return non-zero instead and the CALLER dies.
   279	embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
   280	  local h="$1" p="$2" raw hit dirty
   281	  # `grep -c` exits 1 on NO MATCH, which is not an error. Only rc>=2 is. The old
   282	  # `|| echo X` turned a clean binary's legitimate "0" into "0\nX" and DIED.
   283	  raw="$(hrun "$h" "c=\$(grep -c -a -- '+$EXPECT_SHA' '$p'); rc=\$?
   284	d=\$(grep -c -a -- '+$EXPECT_SHA.dirty' '$p'); rd=\$?
   285	if [ \$rc -ge 2 ] || [ \$rd -ge 2 ]; then echo 'E:ERR:E'; else echo \"E:\$c:\$d:E\"; fi" \
   286	    | nocr | sed -n 's/.*E:\([0-9]*\):\([0-9]*\):E.*/\1 \2/p' | head -1)" || return 1
   287	  [[ -n "$raw" ]] || return 1
   288	  read -r hit dirty <<<"$raw"
   289	  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
   290	  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
   291	}
   292	sha256_of() {      # returns non-zero on a short/empty hash; the CALLER must `|| die`
   293	  local h="$1" p="$2" v
   294	  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')" || return 1
   295	  [[ ${#v} -eq 64 ]] || return 1
   296	  echo "$v"
   297	}
   298	
   299	# --- gates: every one fails CLOSED --------------------------------------------
   300	# Stock macOS awk has no strtonum() (that is gawk). Hand-rolled hex, so the ARP
   301	# comparison actually runs instead of erroring out.
   302	norm_mac() {
   303	  awk -F: '
   304	    function hex(s,   i,c,d,v) {
   305	      v = 0; s = tolower(s)
   306	      for (i = 1; i <= length(s); i++) {
   307	        c = substr(s, i, 1); d = index("0123456789abcdef", c) - 1
   308	        if (d < 0) return -1
   309	        v = v * 16 + d
   310	      }
   311	      return v
   312	    }
   313	    {
   314	      if (NF != 6) { print ""; next }
   315	      out = ""; ok = 1
   316	      for (i = 1; i <= NF; i++) {
   317	        v = hex($i)
   318	        if (v < 0 || v > 255) { ok = 0; break }
   319	        out = out sprintf("%s%02x", (i > 1 ? ":" : ""), v)
   320	      }
   321	      print (ok ? out : "")
   322	    }'
   323	}
   324	
   325	# THE ONLY process probe in this harness. pgrep: 0 = found, 1 = none, >=2 = ERROR.
   326	# Echoes RUNNING | NONE | BROKEN. A probe that cannot answer must NEVER answer "fine",
   327	# and there must be exactly ONE of these -- round 5 found the fail-open surviving in a
   328	# duplicate site precisely because there were two.
   329	pgrep_state() {
   330	  local h="$1" pat="$2" raw
   331	  raw="$(hrun "$h" "pgrep -x '$pat' >/dev/null 2>&1; rc=\$?
   332	if [ \$rc -eq 0 ]; then echo 'G:RUNNING:G'
   333	elif [ \$rc -eq 1 ]; then echo 'G:NONE:G'
   334	else echo 'G:BROKEN:G'; fi" | nocr | sed -n 's/.*G:\([A-Z]*\):G.*/\1/p' | head -1)" || raw=""
   335	  case "$raw" in
   336	    RUNNING|NONE|BROKEN) echo "$raw" ;;
   337	    *)                   echo BROKEN ;;   # no sentinel back == a broken probe
   338	  esac
   339	}
   340	
   341	quiescence_gate() {
   342	  local h="$1" p busy=""
   343	  for p in codex cargo rustc; do
   344	    case "$(pgrep_state "$h" "$p")" in
   345	      RUNNING) busy="$busy $p" ;;
   346	      NONE)    : ;;
   347	      *)       die_blind "$(hname "$h"): the quiescence probe for '$p' BROKE — refusing (a gate that cannot answer must not answer 'fine')" ;;
   348	    esac
   349	  done
   350	  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running:$busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
   351	}
   352	
   353	timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   354	  local h="$1" running auto
   355	  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
   356	  [[ "$running" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
   357	  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
   358	  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1" | nocr | tr -cd '0-9')" || auto=""
   359	  [[ "$auto" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
   360	  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
   361	}
   362	
   363	spotlight_gate() {
   364	  local h="$1" cpu
   365	  # The MAX across samples, not the last: a late idle sample could overwrite an
   366	  # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
   367	  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
   368	    | awk '/^mds_stores/{ if (\$2+0 > m) m = \$2+0 } END{ if (NR == 0) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
   369	  [[ "$cpu" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
   370	  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
   371	}
   372	
   373	load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
   374	load_gate() {
   375	  local h="$1" l ok
   376	  l="$(load1 "$h")" || l=""
   377	  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die_blind "$(hname "$h"): cannot read load1 (got '$l') — refusing"
   378	  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   379	  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
   380	}

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '381,760p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   381	
   382	link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
   383	  local h="$1" o peer_ip want got route_nic nic
   384	  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"; nic="$(hnic "$h")"
   385	  [[ -n "$want" ]] || die_blind "$(hname "$o"): its configured MAC does not parse — refusing"
   386	  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
   387	    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
   388	  # The ARP entry ON THE NIC THE TRAFFIC WILL EGRESS. `arp -n <ip>` prints one line
   389	  # PER INTERFACE that has an entry — q holds entries for nagatha on en0, en1 AND
   390	  # en8 — so an unfiltered $4 yields a MULTI-LINE string that can never equal a
   391	  # single MAC. (Measured: this refused a perfectly good link. It is also the more
   392	  # correct check: a stale entry on the 1GbE NIC is irrelevant to the 10GbE path.)
   393	  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk -v nic='$nic' '\$5 == \"on\" && \$6 == nic {print \$4}' | head -1" | nocr | norm_mac)"
   394	  [[ -n "$got" ]] || die "$(hname "$h"): no ARP entry for $peer_ip ON $nic — the 10GbE path has not resolved the peer"
   395	  [[ "$got" == "$want" ]] \
   396	    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
   397	  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
   398	  [[ "$route_nic" == "$(hnic "$h")" ]] \
   399	    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
   400	}
   401	
   402	# --- the drain device: RESOLVED, never hardcoded (grok) ------------------------
   403	# `iostat disk0` can certify a disk the data never touched. Worse, on APFS the
   404	# volume lives on a SYNTHESIZED disk whose stats may be empty while the physical
   405	# store is saturated — a false "quiet". Resolve the module path to its PHYSICAL
   406	# store and verify iostat actually reports it.
   407	N_DISK=""; Q_DISK=""
   408	hdisk() { if [[ "$1" == n ]]; then echo "$N_DISK"; else echo "$Q_DISK"; fi; }
   409	resolve_disk() {
   410	  local h="$1" p dev
   411	  p="$(hmod "$h")"
   412	  # A FAILED `diskutil` MUST NOT silently fall back to the synthesized disk (round-5
   413	  # codex, HIGH). On APFS the volume lives on a synthesized container whose iostat
   414	  # counters can read IDLE while the physical store is saturated — so falling back to
   415	  # it is not a harmless default, it is a FALSE QUIET that certifies drainage on a
   416	  # device the data never touched. If the volume is APFS, the physical-store lookup
   417	  # must SUCCEED or the gate refuses.
   418	  dev="$(hrun "$h" "d=\$(df '$p' 2>/dev/null | awk 'NR==2{print \$1}' | sed 's|^/dev/||')
   419	[ -n \"\$d\" ] || { echo 'D:NO-DF:D'; exit 0; }
   420	info=\$(diskutil info \"\$d\" 2>/dev/null) || { echo 'D:NO-DISKUTIL:D'; exit 0; }
   421	[ -n \"\$info\" ] || { echo 'D:EMPTY-DISKUTIL:D'; exit 0; }
   422	if echo \"\$info\" | grep -q 'APFS'; then
   423	  ps=\$(echo \"\$info\" | awk -F: '/APFS Physical Store/{gsub(/[ \t]/, \"\", \$2); print \$2}' | head -1)
   424	  [ -n \"\$ps\" ] || { echo 'D:APFS-NO-STORE:D'; exit 0; }
   425	  d=\"\$ps\"
   426	fi
   427	echo \"D:\$(echo \"\$d\" | sed -E 's/s[0-9]+\$//'):D\"" | nocr | sed -n 's/.*D:\([^:]*\):D.*/\1/p' | head -1)"
   428	  # Returns non-zero rather than dying, so the CALLER decides. (The self-test runs
   429	  # each gate in a subshell to survive a refusal — and a `die` in there was invisible
   430	  # while the global it sets was discarded, so the drain then had no device and
   431	  # reported DRAIN-ERROR. The self-test was breaking its own next gate.)
   432	  if [[ ! "$dev" =~ ^disk[0-9]+$ ]]; then
   433	    log "$(hname "$h"): cannot resolve the PHYSICAL disk behind $p (got '$dev') — a drain that watches the wrong device certifies a disk the data never touched, and on APFS a synthesized disk can read idle while the physical store saturates"
   434	    return 1
   435	  fi
   436	  # It must actually REPORT: an iostat that emits nothing for this device would
   437	  # make every sample non-numeric, and the drain must never read that as quiet.
   438	  local probe
   439	  probe="$(hrun "$h" "iostat -d -w 1 -c 2 '$dev' 2>/dev/null | tail -1 | awk '{print \$3}'" | nocr)" || probe=""
   440	  if [[ ! "$probe" =~ ^[0-9]+\.?[0-9]*$ ]]; then
   441	    log "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — cannot certify drainage"
   442	    return 1
   443	  fi
   444	  if [[ "$h" == n ]]; then N_DISK="$dev"; else Q_DISK="$dev"; fi
   445	  log "  drain device on $(hname "$h"): $dev (backs $p, idle probe ${probe} MB/s)"
   446	}
   447	
   448	# --- the settle-gap bound (NOT a removal — a measured bound) -------------------
   449	# Between the client exiting and the fsync starting, the OS writes back dirty pages
   450	# FOR FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
   451	# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
   452	# ssh return-path difference, which is bounded by the round-trip time measured here.
   453	# It is NOT "removed by construction", and the pre-registration no longer says so.
   454	#
   455	# Timed in ONE process, for the same reason the transfer is. Bracketing each ssh
   456	# with two `python3 -c time.time()` calls would have charged it TWO interpreter
   457	# startups (~30 ms) and reported them as network latency — measured: it read 35 ms
   458	# for a round trip that is actually ~5 ms. The instrument's own bound would have
   459	# been wrong by 7x, in the direction that flatters nothing and confuses everything.
   460	SSH_RTT_MS=0
   461	measure_ssh_rtt() {
   462	  # A FAILED ssh must not contribute a plausible number (round-5 codex, MEDIUM): a
   463	  # fast-failing connection would report a small "bound" and flatter the settle claim.
   464	  SSH_RTT_MS="$(python3 -c '
   465	import statistics, subprocess, sys, time
   466	argv = sys.argv[1:]
   467	ts = []
   468	for _ in range(5):
   469	    t = time.monotonic()
   470	    rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
   471	    if rc != 0:
   472	        print("SSH-FAILED")
   473	        raise SystemExit
   474	    ts.append((time.monotonic() - t) * 1000.0)
   475	print(int(statistics.median(ts)))
   476	' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
   477	  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
   478	  local rtt_max=$(( SETTLE_MS / 4 ))
   479	  (( SSH_RTT_MS <= rtt_max )) \
   480	    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
   481	  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
   482	}
   483	
   484	# =============================================================================
   485	preflight() {
   486	  # RUNS=8 is the registered value. RUNS=16 is the ONLY registered escalation, and
   487	  # it may be used for exactly ONE reason: a prior session returned
   488	  # INCONCLUSIVE-UNDERPOWERED. It must NEVER be used to chase a result someone
   489	  # dislikes -- that is the p-hacking this pre-registration exists to prevent.
   490	  #
   491	  # Why it exists (round-3 grok, MEDIUM): at n=8 the >=95% order-statistic interval
   492	  # is the FULL RANGE [min,max], so ONE noisy pair with |d| >= margin blocks a null
   493	  # forever and the rig can only ever say UNDERPOWERED -- a null-incapable
   494	  # instrument is broken too, just less dangerously. At n=16 the interval is
   495	  # [d(4), d(13)] (coverage 97.9%), which tolerates three outliers per side.
   496	  [[ "$RUNS" == 8 || "$RUNS" == 16 ]] \
   497	    || die "RUNS must be 8 (registered) or 16 (the registered escalation, valid ONLY after an INCONCLUSIVE-UNDERPOWERED session) — got '$RUNS'"
   498	  if [[ "$RUNS" == 16 ]]; then
   499	    # A FLAG IS NOT A JUSTIFICATION (round-5 codex, HIGH). `UNDERPOWERED_ESCALATION=1`
   500	    # was sufficient on its own: no prior session named, none verified, "once"
   501	    # unenforced. That is a re-roll button with a serious-sounding name. The
   502	    # escalation must now POINT AT the underpowered session and the harness READS ITS
   503	    # VERDICT — the trigger is evidence on disk, not an operator's assertion.
   504	    local prior="${UNDERPOWERED_ESCALATION:-}" v
   505	    [[ -n "$prior" ]] \
   506	      || die "RUNS=16 is the escalation arm. Set UNDERPOWERED_ESCALATION=<path to the prior session dir> that returned INCONCLUSIVE-UNDERPOWERED. It buys POWER; it is NOT a re-roll."
   507	    # The trigger must be a REAL SESSION, not a directory that merely contains the right
   508	    # words (round-6, codex HIGH + grok F5: "any directory containing the expected first
   509	    # verdict line authorizes escalation; provenance, hashes, build and prior runs=8 are
   510	    # never checked"). So the prior session must carry its own DATA and MANIFEST, and
   511	    # the escalation is bound to the CONTENT of that data, not to its path.
   512	    for _f in session_verdict.txt runs.csv meta.csv staging-manifest.txt; do
   513	      [[ -f "$prior/$_f" ]] \
   514	        || die "UNDERPOWERED_ESCALATION='$prior' has no $_f — the escalation must name a REAL prior session, not a directory with the right words in it"
   515	    done
   516	    v="$(head -1 "$prior/session_verdict.txt" | sed -n 's/^SESSION VERDICT: *//p')"
   517	    [[ "$v" == "INCONCLUSIVE-UNDERPOWERED" ]] \
   518	      || die "the prior session '$prior' returned '$v', not INCONCLUSIVE-UNDERPOWERED. RUNS=16 is triggered by a POWER FAILURE and by nothing else — re-running a result you dislike at higher n is p-hacking, and this gate exists to stop it."
   519	    grep -q "binary_identity=$REGISTERED_BUILD" "$prior/staging-manifest.txt" \
   520	      || die "the prior session '$prior' was not run on the registered build $REGISTERED_BUILD — it cannot authorise an escalation"
   521	    # "Once" is bound to the DATA, not the directory: copying the session elsewhere does
   522	    # not buy a second re-roll, because the burn records the runs.csv hash.
   523	    PRIOR_RUNS_SHA="$(shasum -a 256 "$prior/runs.csv" | cut -d' ' -f1)"
   524	    if [[ -f "$REPO_ROOT/logs/ESCALATED-SESSIONS" ]] \
   525	       && grep -q "$PRIOR_RUNS_SHA" "$REPO_ROOT/logs/ESCALATED-SESSIONS"; then
   526	      die "this exact session's data (runs.csv $PRIOR_RUNS_SHA) has ALREADY authorised an escalation — see logs/ESCALATED-SESSIONS. 'Once' means once, and it is bound to the DATA, not the path."
   527	    fi
   528	    ESCALATED_FROM="$prior"
   529	    log "  escalation: RUNS=16, triggered by $prior (verified INCONCLUSIVE-UNDERPOWERED, build $REGISTERED_BUILD, runs.csv $PRIOR_RUNS_SHA)"
   530	  fi
   531	  [[ "$EXPECT_SHA" == "$REGISTERED_BUILD" ]] \
   532	    || die "EXPECT_SHA='$EXPECT_SHA' but the PRE-REGISTERED build is $REGISTERED_BUILD — a run against another build is not the registered experiment"
   533	  # The instrument must be the REVIEWED instrument: a modified harness must not be
   534	  # able to claim the reviewed commit.
   535	  git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
   536	    || die "the instrument has UNCOMMITTED changes (harness/verdict/test) — it cannot claim the reviewed commit. Commit or stash first."
   537	  # The decision rule proves itself before it grades anything — AND proves the proof
   538	  # is not vacuous. Running only the cases would let a silently-reverted fix pass
   539	  # preflight if the cases still happen to pass for another reason (round-3 grok).
   540	  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
   541	    || die "the verdict engine's OWN guard test FAILS (see $OUT_DIR/verdict-guard-test.txt) — the decision rule is broken; refusing to take data"
   542	  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
   543	    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt); the rule is not actually guarded, refusing to take data"
   544	  log "verdict-engine guard test passed ($(grep -cE ' ok$' "$OUT_DIR/verdict-guard-test.txt" || true) cases, $(grep -cE 'KILLED' "$OUT_DIR/verdict-mutations.txt" || true) mutations killed)"
   545	
   546	  local h p w want got wantb gotb
   547	  for h in n q; do
   548	    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
   549	    timer_gate "$h"                       # THE measurand's clock, proved on the rig
   550	    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
   551	      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
   552	      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
   553	    done
   554	    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
   555	    # THE SAME pgrep FAIL-OPEN AS THE QUIESCENCE GATE, IN A DUPLICATE SITE I DID NOT
   556	    # TOUCH (round-5 codex, HIGH). `if hrun ... pgrep; then die; fi` reads rc>=2 (a
   557	    # BROKEN probe, or a failed ssh) as "no daemon is running" and sails on. Every
   558	    # process probe now goes through this one rc-aware helper -- there is no second
   559	    # site left to forget.
   560	    case "$(pgrep_state "$h" blit-daemon)" in
   561	      RUNNING) die "$(hname "$h"): a blit-daemon is already running — stop it first" ;;
   562	      NONE)    : ;;
   563	      *)       die "$(hname "$h"): cannot probe for a stale blit-daemon — refusing (a gate that cannot answer must not answer 'fine')" ;;
   564	    esac
   565	    for w in large mixed small; do
   566	      want="$(fix_count "$w")"; wantb="$(fix_bytes "$w")"
   567	      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
   568	      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
   569	      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
   570	        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
   571	    done
   572	    link_gate "$h"
   573	    resolve_disk "$h" || die "$(hname "$h"): cannot establish the drain device — refusing"
   574	  done
   575	  measure_ssh_rtt
   576	  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   577	  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
   578	}
   579	
   580	write_manifest() {
   581	  local f="$OUT_DIR/staging-manifest.txt" h nb nd qb qd vh th
   582	  # Hashes computed FIRST, in the caller's shell: `die` inside $(...) exits only the
   583	  # subshell, so the old code wrote an EMPTY hash and called it provenance.
   584	  nb="$(sha256_of n "$N_BLIT")"   || die "nagatha: cannot hash $N_BLIT"
   585	  nd="$(sha256_of n "$N_DAEMON")" || die "nagatha: cannot hash $N_DAEMON"
   586	  qb="$(sha256_of q "$Q_BLIT")"   || die "q: cannot hash $Q_BLIT"
   587	  qd="$(sha256_of q "$Q_DAEMON")" || die "q: cannot hash $Q_DAEMON"
   588	  vh="$(shasum -a 256 "$VERDICT_PY" | cut -d' ' -f1)"
   589	  th="$(shasum -a 256 "$VERDICT_TEST" | cut -d' ' -f1)"
   590	  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
   591	    echo "# verdict_sha256=$vh verdict_test_sha256=$th"   # the engine grades separately: hash it too
   592	    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
   593	    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
   594	    echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
   595	    echo "# escalated_from=${ESCALATED_FROM:-none}"   # a RUNS=16 run must carry its trigger
   596	    echo "# cells=$CELLS"
   597	    echo "host,role,sha,sha256,path"
   598	    echo "nagatha,client,$EXPECT_SHA,$nb,$N_BLIT"
   599	    echo "nagatha,daemon,$EXPECT_SHA,$nd,$N_DAEMON"
   600	    echo "q,client,$EXPECT_SHA,$qb,$Q_BLIT"
   601	    echo "q,daemon,$EXPECT_SHA,$qd,$Q_DAEMON"; } > "$f"
   602	  log "staging manifest recorded (harness + verdict-engine + 4 binary hashes, every threshold)"
   603	}
   604	
   605	# --- daemons ------------------------------------------------------------------
   606	N_PID=""; Q_PID=""; TEARDOWN_FAILED=0
   607	daemon_start() {
   608	  local h="$1" cfg mod bin pid
   609	  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   610	  # The daemon's OWN pid, from $! — not `pgrep | head -1`, which picks the first of
   611	  # whatever happens to be running.
   612	  pid="$(hrun "$h" "mkdir -p '$mod' || exit 1
   613	printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg' || exit 1
   614	nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
   615	echo \"P:\$!:P\"" | nocr | sed -n 's/.*P:\([0-9][0-9]*\):P.*/\1/p' | head -1)"
   616	  [[ "$pid" =~ ^[0-9]+$ ]] || die "$(hname "$h"): daemon did not report a pid (see $mod/mm-daemon.log)"
   617	  # OWN THE PID BEFORE VALIDATING IT (round-5 codex, MEDIUM): the old code stored it
   618	  # only AFTER the alive/listening checks, so a daemon that started but failed
   619	  # validation was `die`d on while the EXIT trap did not yet know its pid — leaking a
   620	  # live daemon holding the port for the next session to trip over.
   621	  if [[ "$h" == n ]]; then N_PID="$pid"; else Q_PID="$pid"; fi
   622	  sleep 2
   623	  hrun "$h" "ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon" \
   624	    || die "$(hname "$h"): daemon pid $pid is not alive (see $mod/mm-daemon.log)"
   625	  # ALIVE is not SERVING: it must hold the port we are about to measure through.
   626	  hrun "$h" "lsof -nP -a -p $pid -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1" \
   627	    || die "$(hname "$h"): daemon pid $pid is NOT LISTENING on $PORT (see $mod/mm-daemon.log)"
   628	  log "$(hname "$h") daemon up (pid $pid, listening) on $(hip "$h"):$PORT"
   629	}
   630	# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
   631	# handshake reached some listener's backlog — not that the daemon speaks blit).
   632	smoke() {
   633	  local h="$1" o
   634	  o="$(other "$h")"
   635	  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 \
   636	    || die "$(hname "$o"): cannot stage the smoke fixture"
   637	  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
   638	    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
   639	    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
   640	  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
   641	  log "smoke ok: $(hname "$h") daemon serves blit"
   642	}
   643	daemon_stop() {
   644	  local h="$1" pid state
   645	  if [[ "$h" == n ]]; then pid="$N_PID"; else pid="$Q_PID"; fi
   646	  [[ -n "$pid" ]] || return 0
   647	  hrun "$h" "kill $pid 2>/dev/null || true
   648	for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done
   649	if ps -p $pid >/dev/null 2>&1; then kill -9 $pid 2>/dev/null || true; sleep 1; fi" >/dev/null 2>&1 || true
   650	  # A teardown that cannot be VERIFIED is a failure, not a success. The old probe
   651	  # called a FAILED ssh "GONE".
   652	  state="$(hrun "$h" "if ps -p $pid >/dev/null 2>&1; then echo 'S:ALIVE:S'; else echo 'S:GONE:S'; fi" \
   653	    | nocr | sed -n 's/.*S:\([A-Z]*\):S.*/\1/p' | head -1)" || state=""
   654	  if [[ "$state" != GONE ]]; then
   655	    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown or could not be probed (got '$state') — port $PORT may still be held"
   656	    TEARDOWN_FAILED=1
   657	    touch "$OUT_DIR/TEARDOWN-FAILED"
   658	    return 1
   659	  fi
   660	  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
   661	}
   662	cleanup() {
   663	  daemon_stop n || true
   664	  daemon_stop q || true
   665	  rm -rf "$MUX" 2>/dev/null || true
   666	  if [[ "$TEARDOWN_FAILED" == 1 ]]; then
   667	    log "ERROR: a daemon survived teardown — see $OUT_DIR/TEARDOWN-FAILED. Clean it up before the next session."
   668	  fi
   669	}
   670	trap cleanup EXIT
   671	
   672	# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
   673	RUN_DRAIN=""; RUN_COLD=""
   674	drain_host() {   # $1 = host. Echoes drained_<n>x2s | DRAIN-TIMEOUT | DRAIN-ERROR
   675	  local h="$1" dev
   676	  dev="$(hdisk "$h")"
   677	  [[ -n "$dev" ]] || { echo DRAIN-ERROR; return 0; }
   678	  # A FAILED iostat must not certify quiet even when it printed a parseable line
   679	  # (round-5 codex, HIGH: a numeric line followed by a NONZERO EXIT still accumulated
   680	  # "quiet" samples). The exit code is now checked BEFORE the value is used.
   681	  hrun "$h" "quiet=0
   682	for i in \$(seq 1 $DRAIN_ITERS); do
   683	  out=\$(iostat -d -w 2 -c 2 '$dev' 2>/dev/null); rc=\$?
   684	  if [ \$rc -ne 0 ]; then echo DRAIN-ERROR; exit 0; fi
   685	  w=\$(echo \"\$out\" | tail -1 | awk '{print \$3}')
   686	  case \"\$w\" in
   687	    ''|*[!0-9.]*) echo DRAIN-ERROR; exit 0 ;;   # non-numeric must NEVER certify quiet
   688	  esac
   689	  ok=\$(awk -v w=\"\$w\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   690	  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   691	  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
   692	done
   693	echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
   694	}
   695	prep_run() {   # $1 = dest host
   696	  local dh="$1" cn=ok cq=ok out
   697	  # Purge BOTH ends first — the purge itself dirties the disk, so a drain certified
   698	  # BEFORE it proves nothing.
   699	  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   700	  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   701	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   702	  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
   703	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
   704	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   705	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   706	}
   707	
   708	# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
   709	RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
   710	fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
   711	  local out
   712	  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
   713	  #
   714	  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
   715	  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
   716	  # RAN. Round 6 then found the repair was still not provable: `sleep` is
   717	  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
   718	  # counted files — so a no-op `sleep` would pass while the log narrated "settle
   719	  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
   720	  #
   721	  # A protection that cannot be OBSERVED is not a protection. The settle now happens
   722	  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
   723	  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
   724	  # to shadow, no exit status left to discard, and no narration left to trust.
   725	  out="$(hrun "$1" "python3 - '$SETTLE_SEC' '$2' <<'PYEOF'
   726	import os, sys, time
   727	settle = float(sys.argv[1])
   728	p = sys.argv[2]
   729	t0 = time.monotonic()
   730	time.sleep(settle)
   731	settled_ms = int((time.monotonic() - t0) * 1000)
   732	if not os.path.isdir(p):
   733	    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
   734	    raise SystemExit
   735	t = time.monotonic()
   736	files = 0
   737	nbytes = 0
   738	for root, _d, fs in os.walk(p):
   739	    for name in fs:
   740	        fp = os.path.join(root, name)
   741	        nbytes += os.path.getsize(fp)
   742	        fd = os.open(fp, os.O_RDONLY)
   743	        os.fsync(fd)
   744	        os.close(fd)
   745	        files += 1
   746	print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
   747	PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
   748	  echo "${out:-NA 0 0 0}"
   749	}
   750	# The settle actually elapsed, on the destination's own clock. Anything else voids.
   751	settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
   752	
   753	# --- one timed run ------------------------------------------------------------
   754	RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
   755	timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
   756	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
   757	  bin="$(hblit "$ih")"
   758	  prep_run "$dh"
   759	  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
   760	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict.py | sed -n '1,360p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env python3
     2	"""Mechanize the Mac<->Mac pre-registered decision rule (PREREGISTRATION.md rev 4).
     3	
     4	The harness must COMPUTE the verdict, not leave it to be applied by hand after the
     5	numbers are visible -- that is what pre-registration exists to prevent.
     6	
     7	WHAT ROUND 2 BROKE, AND WHAT REV 4 FIXES (codex r2 + grok, 15 findings)
     8	----------------------------------------------------------------------
     9	Every one of these let a real effect read as absent, or a dirty rig read as clean:
    10	
    11	  * The equivalence margin was tied to the BAR alone. On a slow arm the bar is
    12	    WIDER than the effect we are trying to exclude: srcinit=2500 with all eight
    13	    d_i = 230 (a rig-W-sized effect in EVERY pair) gave ratio 1.092 (bar PASS),
    14	    CI [230,230], margin 0.10*2500 = 250 -> "VANISHES". Both reviewers reproduced
    15	    it. The margin is now min(BAR_BREACH, DELTA_REF) -- a null must exclude an
    16	    effect the size of the one rig W actually measured, not merely one the bar
    17	    would forgive.
    18	  * The negative margin was wrong for a symmetric RATIO bar. The bar is symmetric
    19	    in ratio, so the inverting boundary is -src/11 (-9.09%), NOT -0.10*src: a CI
    20	    of [-190,0] on src=2000 was called VANISHES though -190 IS an inversion ratio
    21	    of 1.105 -- over the bar.
    22	  * The bootstrap CI was not 95% at n=8 (it resolved to ~[d2,d7], true coverage
    23	    92.97%) and the 10k seeded resamples added no information. Replaced with the
    24	    EXACT distribution-free order-statistic interval, and its true coverage is
    25	    printed, not assumed.
    26	  * The sign test was computed and never read, so 7/8 positive pairs could report
    27	    REPRODUCES while the registered two-sided sign test said p = .0703.
    28	  * RIG-VOID FAILED OPEN (grok, reproduced live): the code demanded a control both
    29	    fail the bar AND land outside a set of outcomes, so a control with bar FAIL and
    30	    a CI crossing zero (-> INCONCLUSIVE) ESCAPED the void -- grok drove a session
    31	    that emitted VANISHES with its gRPC controls sitting at ratio 1.200, bar FAIL.
    32	    A control that fails the bar now voids the rig, unconditionally.
    33	  * A partial CELLS set was FILTERED rather than marked INCOMPLETE, so a one-cell
    34	    run could emit VANISHES while claiming "both" cells vanished. The full
    35	    REGISTERED set must be present and complete.
    36	  * An exact 1.10 ratio could never REPRODUCE (grok): the bar is `<= 1.10 PASSES`
    37	    (the project's acceptance semantics, kept), and REPRODUCES required a bar FAIL,
    38	    so a precise 10% effect was unreportable by construction. Materiality is now
    39	    "bar FAILS **or** the CI's near bound reaches the 10% threshold".
    40	
    41	THE STATISTIC
    42	-------------
    43	  d_i  = destinit_i - srcinit_i     (positive = destination-initiated is slower)
    44	  D    = median(d_i)                (LOW median for even n, applied everywhere)
    45	  CI   = EXACT distribution-free order-statistic interval on the population median:
    46	         the narrowest [d_(k), d_(n+1-k)] whose coverage 1 - 2*P(Bin(n,1/2) <= k-1)
    47	         is >= 95%. At n=8 that is k=1 -> [min(d), max(d)], coverage 99.22%.
    48	         n=8 admits NO exact 95% interval; the conservative side is chosen
    49	         deliberately, and the true coverage is reported in every row.
    50	  sign = exact two-sided binomial test on the count of positive d_i (zeros dropped).
    51	         At n=8, p < .05 requires ALL EIGHT pairs to share a sign (k=8 -> p=.0078;
    52	         k=7 -> p=.0703, NOT significant).
    53	
    54	  BAR        : integer-exact, 10*hi <= 11*lo. `<= 1.10` PASSES (project semantics).
    55	  BREACH_HI  = +src/10   (the effect that reaches ratio 1.10)
    56	  BREACH_LO  = -src/11   (the effect that reaches INVERSE ratio 1.10 -- NOT -src/10)
    57	  MARGIN_HI  = min(BREACH_HI, DELTA_REF)    <- the equivalence margin. DELTA_REF is
    58	  MARGIN_LO  = max(BREACH_LO, -DELTA_REF)      an ABSOLUTE floor (rig W's measured
    59	                                               230 ms): a null must exclude a
    60	                                               rig-W-sized effect however slow this
    61	                                               rig's arms are.
    62	
    63	THE THREE QUESTIONS (rev 7) -- kept apart, because tangling them produced the SAME
    64	class of defect in rounds 3, 4, 5 AND 6. ALL INFERENCE IS PAIRED; the bar (marginal
    65	medians) is the project's ACCEPTANCE criterion and takes no part in inference.
    66	
    67	  DIRECTION   = the SIGN TEST      directional = sign_p < .05  (zeros dropped)
    68	  MAGNITUDE   = the paired CI      material     = CI_lo >= BREACH_HI
    69	                                   material_neg = CI_hi <= BREACH_LO
    70	  EQUIVALENCE = the CI vs MARGIN   null_excl    = CI strictly inside the margin
    71	
    72	PER-CELL OUTCOMES (exhaustive; no unreportable region)
    73	  REPRODUCES            dir_pos and material
    74	  INVERSION             dir_neg and material_neg
    75	  PARTIAL               a real direction whose magnitude is NOT material
    76	  VANISHES              no direction AND null_excl -- a genuine EQUIVALENCE result
    77	  UNDERPOWERED          no direction and the CI cannot exclude the margin -> a PASS
    78	                        here is NOT "P1 vanishes"; the rig could not have seen it
    79	  BAR-FAIL-INCONSISTENT the bar FAILS but the pairs establish NO consistent direction
    80	  UNSTABLE              (override) an arm is bimodal AND the bar flips on pooled runs
    81	  INCOMPLETE            the cell did not finish its registered pairs
    82	
    83	THE CONTROLS ARE A PRECONDITION, NOT A FOOTNOTE
    84	  CONTAMINATING  a directional effect whose CI sits at/beyond the margin, or bimodal
    85	                 -> RIG-VOID. The rig is carrying the effect we came to measure.
    86	  CERTIFIED      bar PASSES and the paired CI lies strictly inside HALF the margin.
    87	                 Half, because certifying a control with the very threshold that
    88	                 DEFINES the effect is incoherent -- it would let a control carry all
    89	                 but 1 ms of P1 and still call the rig clean (round-6, grok).
    90	  otherwise      NOT CERTIFIED -> CONTROLS-UNCERTIFIED, and NO measurand verdict may
    91	                 be read: not a null, and NOT a reproduction either. Uncertainty about
    92	                 a rig-wide confound is not evidence that the confound is absent
    93	                 (round-6, codex).
    94	"""
    95	import csv, os, sys
    96	from math import comb
    97	
    98	runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]
    99	
   100	# ---- THE REGISTERED CONSTANTS ARE PINNED IN CODE, NOT TAKEN FROM THE ENVIRONMENT --
   101	# Round-5 (codex, BLOCKER): they were env-overridable, so `DELTA_REF_MS=240` turned a
   102	# RIG-VOID into a VANISHES -- i.e. the pre-registered decision rule could be edited
   103	# from the command line, by the same person who wants a particular answer, AFTER the
   104	# data existed. The whole point of pre-registration is that this is impossible.
   105	#
   106	# A deviation is not silently accepted and not silently ignored: it REFUSES.
   107	REGISTERED_DELTA_REF_MS = 230        # rig W's measured Delta_P1 (the reference effect)
   108	REGISTERED_PAIRS = (8, 16)           # 8 registered; 16 the UNDERPOWERED escalation
   109	MIN_COVERAGE = 0.95
   110	
   111	DELTA_REF = REGISTERED_DELTA_REF_MS
   112	_env_delta = os.environ.get("DELTA_REF_MS")
   113	if _env_delta is not None and _env_delta.strip() != str(REGISTERED_DELTA_REF_MS):
   114	    sys.stderr.write(
   115	        "REFUSING: DELTA_REF_MS=%r but the PRE-REGISTERED reference effect is %d ms. "
   116	        "The decision rule is not tunable from the environment -- that is what "
   117	        "pre-registration exists to prevent.\n" % (_env_delta, REGISTERED_DELTA_REF_MS))
   118	    raise SystemExit(2)
   119	
   120	
   121	def cells_env(name):
   122	    return [c for c in os.environ.get(name, "").split(",") if c]
   123	
   124	
   125	VERDICT_CELLS = cells_env("VERDICT_CELLS")
   126	CONTROL_CELLS = cells_env("CONTROL_CELLS")
   127	# The controls are a PRECONDITION for reading any verdict, so an engine invoked
   128	# WITHOUT them cannot grade anything (round-6 grok, LOW: called standalone with no
   129	# controls it happily emitted VANISHES -- a footgun aimed at exactly the person who
   130	# would re-grade a CSV by hand).
   131	if not VERDICT_CELLS or not CONTROL_CELLS:
   132	    sys.stderr.write(
   133	        "REFUSING: VERDICT_CELLS and CONTROL_CELLS must both be set. The controls are "
   134	        "a precondition for any verdict -- an engine with no controls cannot certify "
   135	        "the rig, and must not pretend to.\n")
   136	    raise SystemExit(2)
   137	# The full registered set must be PRESENT and COMPLETE. A partial CELLS set that is
   138	# merely filtered lets a one-cell run emit VANISHES while claiming "both" cells
   139	# vanished (codex r2 BLOCKER 1).
   140	REGISTERED_CELLS = cells_env("REGISTERED_CELLS") or (VERDICT_CELLS + CONTROL_CELLS)
   141	# The engine is separately executable and is hashed into the manifest, so it must
   142	# not depend on the harness telling it the truth. Round-3 grok (HIGH): it trusted
   143	# `meta.complete == yes` and never checked n, so a CSV with ONE pair and a lying
   144	# meta produced VANISHES at 0% CI coverage -- a confident false equivalence claim.
   145	REQUIRED_PAIRS = int(os.environ.get("REQUIRED_PAIRS", "8"))
   146	if REQUIRED_PAIRS not in REGISTERED_PAIRS:
   147	    sys.stderr.write(
   148	        "REFUSING: REQUIRED_PAIRS=%d is not a registered pair count %s.\n"
   149	        % (REQUIRED_PAIRS, REGISTERED_PAIRS))
   150	    raise SystemExit(2)
   151	# A session-level void the HARNESS detected (e.g. end-load above the bar). The
   152	# engine must be able to refuse a verdict on evidence it cannot see itself.
   153	SESSION_VOID_REASON = os.environ.get("SESSION_VOID_REASON", "").strip()
   154	
   155	rows = list(csv.DictReader(open(runs_p)))
   156	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   157	
   158	
   159	def ms_of(r):
   160	    """A corrupt row must stop the grading, LOUDLY. Mapping it to a soft outcome
   161	    would hide the corruption; a traceback would obscure it (round-3 grok, LOW)."""
   162	    try:
   163	        return int(r["ms"])
   164	    except (TypeError, ValueError):
   165	        sys.stderr.write(
   166	            "CORRUPT ROW: cell=%s arm=%s run=%s has non-numeric ms=%r. Refusing to "
   167	            "grade -- a benchmark whose rows do not parse has no verdict.\n"
   168	            % (r.get("cell"), r.get("arm"), r.get("run"), r.get("ms")))
   169	        raise SystemExit(2)
   170	
   171	
   172	by, slots, void = {}, {}, {}
   173	for r in rows:
   174	    key = (r["cell"], r["arm"])
   175	    if r["valid"] == "yes":
   176	        by.setdefault(key, []).append(ms_of(r))
   177	        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = ms_of(r)
   178	    else:
   179	        void[key] = void.get(key, 0) + 1
   180	
   181	
   182	def med(v):
   183	    """Low median for even n, stated once and applied consistently."""
   184	    v = sorted(v)
   185	    return v[(len(v) - 1) // 2]
   186	
   187	
   188	def complete(c):
   189	    """COMPLETE is checked against the DATA, not against meta's say-so.
   190	
   191	    Round-3 (grok, HIGH): this trusted `meta.complete == yes` and required only
   192	    >= 1 pair, so a one-pair CSV with a lying meta graded as a full cell and
   193	    emitted VANISHES at 0% CI coverage. The pair count is now enforced here, and
   194	    the CI's coverage is enforced at the grading site.
   195	    """
   196	    if c not in meta or meta[c].get("complete") != "yes":
   197	        return False
   198	    arms = [a for (cc, a) in by if cc == c]
   199	    if "srcinit" not in arms or "destinit" not in arms:
   200	        return False
   201	    return len(paired(c)) >= REQUIRED_PAIRS
   202	
   203	
   204	def paired(c):
   205	    return [v["destinit"] - v["srcinit"]
   206	            for (cc, _run), v in sorted(slots.items())
   207	            if cc == c and "srcinit" in v and "destinit" in v]
   208	
   209	
   210	def median_ci(d):
   211	    """EXACT distribution-free CI on the population median.
   212	
   213	    [d_(k), d_(n+1-k)] covers the median with probability
   214	    1 - 2*P(Bin(n,1/2) <= k-1). Pick the LARGEST k (narrowest interval) whose
   215	    coverage is still >= 95%. Returns (lo, hi, coverage). No bootstrap: at n=8 the
   216	    bootstrap median CI resolves to ~[d2,d7] (92.97%) while claiming 95%.
   217	    """
   218	    d = sorted(d)
   219	    n = len(d)
   220	    if n == 0:
   221	        return 0, 0, 0.0
   222	    if n == 1:
   223	        return d[0], d[0], 0.0
   224	    best = None
   225	    for k in range(1, n // 2 + 1):
   226	        tail = sum(comb(n, i) for i in range(0, k)) / (2.0 ** n)
   227	        cov = 1.0 - 2.0 * tail
   228	        if cov >= 0.95:
   229	            best = (d[k - 1], d[n - k], cov)      # larger k => narrower
   230	    if best is None:                              # n too small for 95% at any k
   231	        return d[0], d[-1], 1.0 - 2.0 / (2.0 ** n)
   232	    return best
   233	
   234	
   235	def sign_p(d):
   236	    """Exact two-sided binomial test on the count of positive differences."""
   237	    nz = [x for x in d if x != 0]
   238	    n = len(nz)
   239	    if n == 0:
   240	        return 1.0, 0, 0
   241	    k = sum(1 for x in nz if x > 0)
   242	    tail = sum(comb(n, i) for i in range(0, min(k, n - k) + 1))
   243	    return min(1.0, 2.0 * tail / (2 ** n)), k, n
   244	
   245	
   246	def bar_of(hi, lo):
   247	    """Integer-exact. `<= 1.10` PASSES -- the project's acceptance semantics."""
   248	    return "PASS" if 10 * hi <= 11 * lo else "FAIL"
   249	
   250	
   251	# ---- summary: every run printed (pf-0's bistability lesson) ------------------
   252	with open(sum_p, "w") as f:
   253	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,runs\n")
   254	    for (c, a) in sorted(by):
   255	        v = by[(c, a)]
   256	        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   257	        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s\n" % (
   258	            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
   259	            void.get((c, a), 0), " ".join(str(x) for x in v)))
   260	
   261	# ---- paired stats + per-cell outcome ----------------------------------------
   262	cell_outcome, cell_detail = {}, {}
   263	all_cells = sorted(set(REGISTERED_CELLS) | set(meta))
   264	with open(pair_p, "w") as f:
   265	    f.write("cell,n_pairs,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,ci_coverage,"
   266	            "sign_p,k_pos_of_n,breach_hi_ms,breach_lo_ms,margin_hi_ms,margin_lo_ms,"
   267	            "delta_ref_ms,null_excluded,unstable,outcome\n")
   268	    for c in all_cells:
   269	        if not complete(c):
   270	            cell_outcome[c] = "INCOMPLETE"
   271	            f.write("%s,0,,,,,,,,,,,,,,,%d,,,INCOMPLETE\n" % (c, DELTA_REF))
   272	            continue
   273	        d = paired(c)
   274	        s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   275	        hi, lo = max(s_med, d_med), min(s_med, d_med)
   276	        bar = bar_of(hi, lo)
   277	        D = med(d)
   278	        ci_lo, ci_hi, cov = median_ci(d)
   279	        p, k, n = sign_p(d)
   280	
   281	        # A CI that does not reach the registered confidence level cannot ground
   282	        # ANY outcome -- least of all a null. Grading on it is how the n=1 session
   283	        # emitted VANISHES at 0% coverage.
   284	        if cov < MIN_COVERAGE:
   285	            cell_outcome[c] = "INCOMPLETE"
   286	            f.write("%s,%d,,,,,,,,%.4f,,,,,,,%d,,,INCOMPLETE\n" % (c, len(d), cov, DELTA_REF))
   287	            continue
   288	
   289	        # The bar is symmetric in RATIO, so the two boundaries are NOT symmetric in
   290	        # ms: +src/10 reaches 1.10, but only -src/11 reaches the INVERSE 1.10.
   291	        breach_hi = s_med / 10.0
   292	        breach_lo = -s_med / 11.0
   293	        # A null must exclude an effect the size of the one rig W measured (230 ms),
   294	        # not merely one the bar would forgive -- on a slow arm the bar is WIDER.
   295	        margin_hi = min(breach_hi, float(DELTA_REF))
   296	        margin_lo = max(breach_lo, -float(DELTA_REF))
   297	
   298	        # THE THREE QUESTIONS, KEPT SEPARATE. Rounds 3, 4 and 5 all produced the same
   299	        # class of defect by tangling them together, so they are now disentangled and
   300	        # each is answered by the statistic that can actually answer it:
   301	        #
   302	        #   DIRECTION  -- the SIGN TEST. Is there a consistent direction at all?
   303	        #   MAGNITUDE  -- the CI. Is the effect big enough to matter, IN THAT DIRECTION?
   304	        #   EQUIVALENCE-- the CI vs the MARGIN. Is a material effect EXCLUDED?
   305	        #
   306	        # Round-5 (codex, BLOCKER): `bar == "FAIL"` carried NO DIRECTION, yet made an
   307	        # effect of EITHER sign "material" -- so at n=16, thirteen +1 ms pairs and three
   308	        # -110 ms pairs (marginal medians failing the bar in the INVERSE direction) gave
   309	        # a clean `REPRODUCES` for a ONE MILLISECOND effect. A bar failure is only
   310	        # material to a claim that points the SAME WAY as the bar failure.
   311	        #
   312	        # Round-5 (grok, BLOCKER): a single ZERO pair dragged `ci_lo` to 0, which killed
   313	        # the old `pos_effect` (it demanded `ci_lo > 0`) -- so `d = [0, 99x7]` at
   314	        # src=1000 was "no effect" AND null_excl (99 < margin 100) and reported
   315	        # `VANISHES`, while the sign test REJECTED at p = .0156. Seven of eight pairs
   316	        # carried a 99 ms effect, one millisecond under the bar, and it was called
   317	        # equivalence. DIRECTION is the sign test's job, not the CI's.
   318	        # ALL INFERENCE IS PAIRED. The bar is computed on the MARGINAL medians; the CI
   319	        # on the PAIRED differences. They are different statistics and they can point
   320	        # OPPOSITE WAYS (round-5), or agree in direction while disagreeing wildly in
   321	        # magnitude (round-6). Rev 6 tried to fix that by making the bar failure
   322	        # direction-aware -- and codex promptly drove `material` again: at n=16 a
   323	        # paired D of ONE MILLISECOND (CI [1,1], 16/16 positive) still reported
   324	        # REPRODUCES, because three outliers moved the MARGINAL median enough to fail
   325	        # the bar in the matching direction, and `material` accepted a bar failure as
   326	        # a substitute for paired magnitude.
   327	        #
   328	        # So the bar no longer participates in INFERENCE AT ALL. It is the project's
   329	        # ACCEPTANCE criterion: it is computed, reported in every row, and used to
   330	        # judge a CELL against the 1.10 invariance bar -- but direction and magnitude
   331	        # are decided by the paired statistics, and by nothing else.
   332	        directional = p < 0.05                       # DIRECTION  -- the sign test
   333	        dir_pos = directional and k > (n - k)
   334	        dir_neg = directional and k < (n - k)
   335	        material = ci_lo >= breach_hi                # MAGNITUDE  -- the paired CI, only
   336	        material_neg = ci_hi <= breach_lo
   337	        null_excl = (ci_lo > margin_lo) and (ci_hi < margin_hi)   # EQUIVALENCE
   338	
   339	        # UNSTABLE, as a STATISTIC not a vibe: an arm splits into two clusters
   340	        # separated by more than the paired spread, AND the bar verdict flips when
   341	        # graded on pooled runs instead of medians.
   342	        unstable = "no"
   343	        for arm in ("srcinit", "destinit"):
   344	            v = sorted(by[(c, arm)])
   345	            gaps = [(v[i + 1] - v[i], i) for i in range(len(v) - 1)]
   346	            gmax = max(gaps)[0] if gaps else 0
   347	            if gmax > (max(d) - min(d)) and gmax > 0:
   348	                a_src = sum(by[(c, "srcinit")]) / float(len(by[(c, "srcinit")]))
   349	                a_dst = sum(by[(c, "destinit")]) / float(len(by[(c, "destinit")]))
   350	                if bar_of(max(a_src, a_dst), min(a_src, a_dst)) != bar:
   351	                    unstable = "yes"
   352	
   353	        if dir_pos and material:
   354	            out = "REPRODUCES"
   355	        elif dir_neg and material_neg:
   356	            out = "INVERSION"
   357	        elif directional:
   358	            # A real, consistent direction that is NOT material. NEVER a null -- this
   359	            # is where grok's [0, 99x7] belongs, not in VANISHES.
   360	            out = "PARTIAL"

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict.py | sed -n '361,700p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   361	        elif bar == "FAIL":
   362	            # The medians breach 1.10 but the pairs establish no consistent direction.
   363	            out = "BAR-FAIL-INCONSISTENT"
   364	        elif null_excl:
   365	            out = "VANISHES"
   366	        else:
   367	            out = "UNDERPOWERED"
   368	        if unstable == "yes":
   369	            out = "UNSTABLE"
   370	
   371	        cell_outcome[c] = out
   372	        cell_detail[c] = dict(
   373	            D=D, ci=(ci_lo, ci_hi), cov=cov, p=p, k=k, n=n, bar=bar,
   374	            ratio=hi / lo if lo else 0.0,
   375	            breach=(breach_hi, breach_lo),
   376	            margin=(margin_hi, margin_lo), null_excl=null_excl,
   377	            directional=directional,
   378	            # The whole CI sits at or beyond the margin, in the direction of the
   379	            # effect: the cell is CARRYING a material asymmetry, not merely failing to
   380	            # exclude one.
   381	            ci_at_or_beyond_margin=(dir_pos and ci_lo >= margin_hi)
   382	            or (dir_neg and ci_hi <= margin_lo))
   383	        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%.4f,%d/%d,%d,%d,%d,%d,%d,%s,%s,%s\n" % (
   384	            c, len(d), s_med, d_med, (hi / lo if lo else 0.0), bar, D, ci_lo, ci_hi, cov,
   385	            p, k, n, round(breach_hi), round(breach_lo), round(margin_hi), round(margin_lo),
   386	            DELTA_REF, "yes" if null_excl else "no", unstable, out))
   387	
   388	# ---- per-cell invariance rows (unchanged shape) ------------------------------
   389	with open(ver_p, "w") as f:
   390	    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,delta_ms,bar,outcome\n")
   391	    for c in all_cells:
   392	        if not complete(c):
   393	            f.write("%s,invariance,srcinit,destinit,,,,,1.10,INCOMPLETE\n" % c)
   394	            continue
   395	        s, dd = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   396	        hi, lo = max(s, dd), min(s, dd)
   397	        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,1.10,%s\n" % (
   398	            c, s, dd, hi / lo if lo else 0.0, dd - s, bar_of(hi, lo)))
   399	
   400	# ---- SESSION VERDICT: strict precedence, exhaustive --------------------------
   401	lines = []
   402	# Every REGISTERED cell must be present and complete. Absent cells are INCOMPLETE,
   403	# never filtered away (codex r2 BLOCKER 1).
   404	missing = [c for c in REGISTERED_CELLS if c not in cell_outcome]
   405	for c in missing:
   406	    cell_outcome[c] = "INCOMPLETE"
   407	incomplete = [c for c in REGISTERED_CELLS if cell_outcome.get(c) == "INCOMPLETE"]
   408	
   409	ctrl = [c for c in CONTROL_CELLS if c in cell_outcome]
   410	verd = [c for c in VERDICT_CELLS if c in cell_outcome]
   411	
   412	# RIG-VOID. A control must be CLEAN, and "clean" is measured by the SAME absolute
   413	# materiality the power gate uses -- not by the bar alone.
   414	#
   415	# Round 2 (grok): a control with bar FAIL whose CI crossed zero escaped the void,
   416	# and a session emitted VANISHES with its controls at ratio 1.200. Fixed.
   417	# Round 3 (grok, BLOCKER -- REPRODUCED): the SAME structural hole survived one level
   418	# down. A control with a real, 8/8, rig-W-sized effect (d_i = 230 in every pair) on
   419	# a SLOW arm (src=2500 -> ratio 1.092) is bar-PASS, lands as PARTIAL, and escaped
   420	# the void -- so the session printed a clean VANISHES while every control carried
   421	# the exact effect size the power gate is built around. On a slow arm the bar is
   422	# WIDER than DELTA_REF; that is the very thing the margin exists to fix, and the
   423	# control rule was still using the bar.
   424	#
   425	# A control therefore voids the rig unless its own effect is EXCLUDED as smaller
   426	# than the margin (null_excl) -- i.e. unless the control itself passes the
   427	# equivalence test. A tiny consistent asymmetry (host x role: q is the faster Mac)
   428	# is immaterial and does NOT void; a margin-sized one does.
   429	# WHAT A CONTROL MUST PROVE -- expressed as the question, not as a list of labels.
   430	#
   431	# Three rounds running, this rule was written as "void if the outcome is one of
   432	# {...}", and three times an effect walked through a label that was not on the list:
   433	#   r3: a bar-FAIL control whose CI crossed zero was INCONCLUSIVE -> escaped.
   434	#   r4: a Delta_ref-sized control effect on a slow arm was PARTIAL -> escaped.
   435	#   r5: ONE zero pair made a 7/8 Delta_ref control UNDERPOWERED -> escaped, and the
   436	#       session printed VANISHES with every control carrying D=+230.
   437	# So it is no longer written that way. There are exactly two questions:
   438	#
   439	#   1. Is the control CONTAMINATING? -- it carries a directional effect whose whole
   440	#      CI sits at or beyond the margin, or it fails the bar, or it is bimodal.
   441	#      Nothing in this rig can be trusted; the session is RIG-VOID.
   442	#   2. Is the control CERTIFIED CLEAN? -- its effect is EXCLUDED as smaller than the
   443	#      margin (null_excl). If it is not, we cannot say the rig is free of a
   444	#      material arm asymmetry, so A NULL IS NOT AVAILABLE. (It does not void a
   445	#      REPRODUCTION: a merely NOISY control does not manufacture a consistent 8/8
   446	#      one-directional effect in the measurand, and voiding real evidence on that
   447	#      basis would be its own false negative -- grok, round-5 NEW-5, which is why
   448	#      an unproven control blocks the null rather than killing the session.)
   449	# CONTAMINATING: the rig is CARRYING the effect we came to measure. Nothing here can
   450	# be trusted -> RIG-VOID. Paired evidence only (a marginal-median bar failure with
   451	# clean pairs is not contamination -- it made a control simultaneously "certified" and
   452	# "contaminating", a contradiction codex drove to a FALSE RIG-VOID).
   453	def _ctrl_contaminating(c):
   454	    dt = cell_detail.get(c, {})
   455	    if cell_outcome[c] == "UNSTABLE":
   456	        return True
   457	    return bool(dt.get("directional") and dt.get("ci_at_or_beyond_margin"))
   458	
   459	
   460	# CERTIFIED CLEAN: and the threshold for a CONTROL must be STRICTLY TIGHTER than the
   461	# effect we claim to detect in the MEASURAND. Round-6 (grok, BLOCKER): certification
   462	# used the SAME margin as materiality, so a control carrying D = +229 ms -- ONE
   463	# MILLISECOND under the reference effect -- certified as "clean", and the session
   464	# printed VANISHES with the prose "every control is CERTIFIED clean". Certifying a
   465	# control with the very threshold that defines the effect is incoherent: it would let
   466	# us claim P1 is TCP-only while the gRPC control carries all but 1 ms of it.
   467	#
   468	# So a control must carry LESS THAN HALF the material effect. That is not an invented
   469	# number: it is the specificity claim itself, made checkable. P1 is asserted to be
   470	# TCP-only and mixed-only; if a control carries half the effect, that assertion is not
   471	# readable off this rig. (At src=2500 -> 115 ms; at src=1000 -> 50 ms; i.e. ~5% of the
   472	# arm, which is the rig noise measured on the q-baseline, 2-4%.)
   473	def _ctrl_certified(c):
   474	    dt = cell_detail.get(c, {})
   475	    if not dt:
   476	        return False
   477	    if dt.get("bar") == "FAIL":
   478	        return False            # a control breaching the acceptance bar certifies nothing
   479	    lo, hi = dt["ci"]
   480	    m_hi, m_lo = dt["margin"]
   481	    return (lo > m_lo / 2.0) and (hi < m_hi / 2.0)
   482	
   483	
   484	ctrl_void = [c for c in ctrl if _ctrl_contaminating(c)]
   485	# NOT CERTIFIED => NO VERDICT MAY BE READ ABOUT THE MEASURAND -- not a null, and NOT A
   486	# REPRODUCTION EITHER (round-6 codex, BLOCKER: uncertified controls blocked only
   487	# VANISHES, so with every control at D=+230 the engine still confidently declared P1
   488	# REPRODUCED). "Uncertainty about a rig-wide confound is not evidence that the confound
   489	# is absent" -- and P1's whole claim is that the effect is specific to TCP x mixed.
   490	ctrl_uncertified = [c for c in ctrl if c not in ctrl_void and not _ctrl_certified(c)]
   491	# Controls that certify clean but still carry a real, tiny asymmetry (host x role -- q
   492	# is the faster Mac) do not block anything, and are NEVER silent.
   493	ctrl_caveat = [c for c in ctrl
   494	               if c not in ctrl_void and c not in ctrl_uncertified
   495	               and cell_outcome[c] == "PARTIAL"]
   496	
   497	if incomplete:
   498	    verdict = "INCOMPLETE"
   499	    why = ("registered cells missing, short of their %d pairs, or graded on a CI "
   500	           "below the registered %.0f%% coverage: %s. The full registered set must "
   501	           "complete before any verdict is read."
   502	           % (REQUIRED_PAIRS, 100 * MIN_COVERAGE, ", ".join(incomplete)))
   503	elif SESSION_VOID_REASON:
   504	    # Evidence the engine cannot see for itself (end-load, an operator abort).
   505	    verdict = "RIG-VOID"
   506	    why = ("the harness voided the session: %s. NO verdict may be read."
   507	           % SESSION_VOID_REASON)
   508	elif ctrl_void:
   509	    verdict = "RIG-VOID"
   510	    why = ("control cell(s) are CONTAMINATING -- the rig is carrying the very effect "
   511	           "this experiment measures: %s. NO verdict may be read."
   512	           % ", ".join("%s(%s,bar=%s)" % (c, cell_outcome[c],
   513	                                          cell_detail.get(c, {}).get("bar", "?"))
   514	                       for c in ctrl_void))
   515	elif ctrl_uncertified:
   516	    # BEFORE any measurand branch. A control that cannot be certified clean blocks
   517	    # EVERY verdict -- the null AND the reproduction. P1 is claimed TCP-only and
   518	    # mixed-only; if the gRPC/large controls might be carrying the same arm asymmetry,
   519	    # then neither "it reproduced" nor "it vanished" is readable off this rig.
   520	    verdict = "CONTROLS-UNCERTIFIED"
   521	    why = ("control cell(s) could NOT be certified free of an arm asymmetry: %s. A "
   522	           "control must carry LESS THAN HALF the material effect for P1's TCP-only / "
   523	           "mixed-only claim to be readable here. Until they do, NO measurand verdict "
   524	           "may be read -- not a null, and NOT a reproduction: uncertainty about a "
   525	           "rig-wide confound is not evidence that the confound is absent. Re-run with "
   526	           "the registered RUNS=16 escalation to buy the power to certify them."
   527	           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d])"
   528	                       % (c, cell_outcome[c], cell_detail.get(c, {}).get("D", 0),
   529	                          cell_detail.get(c, {}).get("ci", (0, 0))[0],
   530	                          cell_detail.get(c, {}).get("ci", (0, 0))[1])
   531	                       for c in ctrl_uncertified))
   532	else:
   533	    outs = {c: cell_outcome[c] for c in verd}
   534	    repro = [c for c, o in outs.items() if o == "REPRODUCES"]
   535	    inv = [c for c, o in outs.items() if o == "INVERSION"]
   536	    unst = [c for c, o in outs.items() if o == "UNSTABLE"]
   537	    barfi = [c for c, o in outs.items() if o == "BAR-FAIL-INCONSISTENT"]
   538	    van = [c for c, o in outs.items() if o == "VANISHES"]
   539	    part = [c for c, o in outs.items() if o == "PARTIAL"]
   540	    under = [c for c, o in outs.items() if o == "UNDERPOWERED"]
   541	
   542	    # PRECEDENCE. A clean reproduction in EITHER direction answers the registered
   543	    # question, and a messy SIBLING cell does not retract it (round-3 grok, HIGH:
   544	    # UNSTABLE and BAR-FAIL-INCONSISTENT outranked REPRODUCES, so a clean 8/8
   545	    # reproduction in nq was reported as BAR-FAIL-INCONSISTENT because qn was noisy
   546	    # -- a FALSE NON-REPRODUCTION against the pre-registration's "either direction"
   547	    # rule). MIXED-SIGN still outranks it: a reproduction in one direction and an
   548	    # INVERSION in the other is evidence of the host x role artifact itself.
   549	    #
   550	    # Demoting UNSTABLE below REPRODUCES cannot leak a null: VANISHES requires ALL
   551	    # measurand cells to VANISH, so any unstable sibling still blocks it.
   552	    if repro and inv:
   553	        verdict = "MIXED-SIGN"
   554	        why = ("reproduces in %s but INVERTS in %s -- a host x role interaction "
   555	               "this rig cannot decompose. INCONCLUSIVE for the pairing question."
   556	               % (", ".join(repro), ", ".join(inv)))
   557	    elif repro:
   558	        verdict = "REPRODUCES"
   559	        why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: it "
   560	               "shows P1 CAN occur macOS<->macOS, so P1 is not waivable as 'Windows "
   561	               "residue'. It does NOT establish a platform-general layout cost, it "
   562	               "does NOT name the mechanism, it does NOT kill H1 (H1 accuses code, and "
   563	               "that code runs here too), and it leaves macOS/APFS and host x role "
   564	               "explanations OPEN." % ", ".join(repro))
   565	        messy = [c for c in (unst + barfi)]
   566	        if messy:
   567	            why += ("\n\nSIBLING CAVEAT: the other direction is not clean (%s). The "
   568	                    "pre-registration answers the question on EITHER direction, so "
   569	                    "the reproduction stands -- but the sibling is reported, not "
   570	                    "buried, and it is NOT evidence of an inversion."
   571	                    % ", ".join("%s(%s)" % (c, cell_outcome[c]) for c in messy))
   572	    elif inv:
   573	        verdict = "INVERSION"
   574	        why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank "
   575	               "this as 'P1 absent'." % ", ".join(inv))
   576	    elif unst:
   577	        verdict = "UNSTABLE"
   578	        why = ("bimodal arm(s) whose bar verdict flips on pooled runs: %s. Report as "
   579	               "unstable, NOT resolved." % ", ".join(unst))
   580	    elif barfi:
   581	        verdict = "BAR-FAIL-INCONSISTENT"
   582	        why = ("the medians breach the 1.10 bar in %s, but the paired evidence does "
   583	               "NOT establish a consistent effect (the CI includes 0, or the sign "
   584	               "test does not reject). This is NOT a null and NOT a clean "
   585	               "reproduction: the cell contradicts itself (pf-0's bistability shape). "
   586	               "Report the runs verbatim." % ", ".join(barfi))
   587	    elif under:
   588	        verdict = "INCONCLUSIVE-UNDERPOWERED"
   589	        why = ("cells cannot exclude an effect of size min(bar_breach, %d ms): %s. A "
   590	               "PASS here is NOT 'P1 vanishes' -- the instrument could not have seen "
   591	               "it (pf-0's error, pre-empted)." % (DELTA_REF, ", ".join(under)))
   592	    elif van and len(van) == len(verd):
   593	        verdict = "VANISHES"
   594	        why = ("both TCP-mixed cells EXCLUDE an effect of size min(bar_breach, %d ms), "
   595	               "and every control is CERTIFIED clean (a genuine equivalence result). "
   596	               "Scoped to THIS pair: P1 did not reproduce macOS<->macOS. That is "
   597	               "CONSISTENT with 'the Windows peer is required' but does NOT prove it -- "
   598	               "it could equally be a property of these two machines, their disks, or "
   599	               "this macOS version." % DELTA_REF)
   600	    elif part:
   601	        verdict = "PARTIAL"
   602	        why = ("a real but sub-bar asymmetry in: %s. Neither a reproduction nor a "
   603	               "vanish; pf-1 owns it." % ", ".join(part))
   604	    else:
   605	        verdict = "INCONCLUSIVE"
   606	        why = "no registered case matched cleanly; report the cells verbatim."
   607	
   608	    if ctrl_caveat:
   609	        # NOT "sub-bar": a Delta_ref-sized control effect is bar-sub only because the
   610	        # arm is slow, and those now VOID. What survives here is either excluded as
   611	        # smaller than the MARGIN, or undetectable. Say that, precisely.
   612	        why += ("\n\nCONTROL CAVEAT (does not void the rig, and is not silent): %s. A "
   613	                "PARTIAL control carries a real asymmetry that is EXCLUDED as smaller "
   614	                "than the margin (min(bar_breach, %d ms)); an UNDERPOWERED control "
   615	                "could not resolve one either way. P1 is claimed TCP-only and "
   616	                "mixed-only; weigh this against that claim."
   617	                % (", ".join("%s(%s)" % (c, cell_outcome[c]) for c in ctrl_caveat),
   618	                   DELTA_REF))
   619	
   620	lines.append("SESSION VERDICT: %s" % verdict)
   621	lines.append("")
   622	lines.append(why)
   623	lines.append("")
   624	lines.append("Per-cell outcomes (the rule is graded on paired.csv):")
   625	for c in sorted(cell_outcome):
   626	    dt = cell_detail.get(c)
   627	    if dt:
   628	        lines.append(
   629	            "  %-14s %-22s ratio=%.3f bar=%s  D=%+dms CI=[%+d,%+d] (%.1f%%) "
   630	            "margin=[%+d,%+d] sign_p=%.4f (%d/%d pos)"
   631	            % (c, cell_outcome[c], dt["ratio"], dt["bar"], dt["D"],
   632	               dt["ci"][0], dt["ci"][1], 100 * dt["cov"],
   633	               round(dt["margin"][1]), round(dt["margin"][0]),
   634	               dt["p"], dt["k"], dt["n"]))
   635	    else:
   636	        lines.append("  %-14s %s" % (c, cell_outcome[c]))
   637	lines.append("")
   638	lines.append("CI = exact order-statistic interval on the median; its true coverage is")
   639	lines.append("printed per cell (n=8 admits no exact 95% interval -- the conservative")
   640	lines.append("side is taken deliberately). A null requires the CI to lie strictly")
   641	lines.append("inside the margin, which is min(bar_breach, DELTA_REF=%dms)." % DELTA_REF)
   642	lines.append("")
   643	lines.append("This file is COMPUTED from the pre-registered rule. It declares nothing")
   644	lines.append("beyond it, and the owner walks the numbers.")
   645	
   646	open(sess_p, "w").write("\n".join(lines) + "\n")
   647	print("\n".join(lines))

