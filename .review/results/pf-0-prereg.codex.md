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
session id: 019f5d88-f4e0-7260-82b6-dff0aed694a9
--------
user
Review the diff of commit 35b9620 (run: git show 35b9620). It adds docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md — the pre-registered design and decision rule for an ENVIRONMENTAL experiment on benchmark rig W (Mac 10.1.10.54 <-> Windows netwatch-01), to be run BEFORE any code work.

This is a plan/docs change under docs/plan/OTP12_PERF_FINDINGS.md (Status: Active). Governance: docs/DECISIONS.md, AGENTS.md, .agents/repo-guidance.md.

BACKGROUND: finding P1 is an initiator/verb INVARIANCE failure — the cell wm_tcp_mixed (TCP carrier, mixed fixture, Windows->Mac data) measures 1.300 (dest-initiated 1221ms vs source-initiated 939ms) where the parent plan docs/plan/ONE_TRANSFER_PATH.md requires <=1.10. It does NOT reproduce on a same-OS Linux rig (8/8 pass). Windows ran at MTU 1500 for every benchmark ever recorded; the fleet is now at MTU 9000, never yet exercised. The experiment asks whether jumbo frames dissolve P1.

YOUR JOB — be adversarial about the EXPERIMENT DESIGN, not the prose. The author has already retracted three claims this session by trusting unvalidated instruments, so assume a fourth defect is present and find it. Read the actual evidence; do not take the doc's word for anything:
  - scripts/bench_otp12_win.sh (the harness; note BASELINE_SUMMARY near line 105 and the compute_verdicts python block at the end)
  - docs/bench/otp12-win-2026-07-12/summary.csv and verdicts.csv (session 12b)
  - docs/bench/otp12c-win-2026-07-13/summary.csv and verdicts.csv (session 12c)
  - docs/bench/otp2w-baseline-2026-07-10/summary.csv (the committed reference)
  - docs/plan/OTP12_PERF_FINDINGS.md (the UNIFORM decision rule, Fix criteria, pf-1/pf-final staging) and docs/plan/ONE_TRANSFER_PATH.md (acceptance criteria)
  - .agents/machines.md (rig facts, the macOS ping trap, the Mac NIC change)

Check specifically, each with file:line or CSV-row evidence:

1. CAUSAL VALIDITY. Does the planned run isolate MTU? The doc claims the real control is "Aquantia x MTU 1500 x f35702a" (a combination never run) rather than "the Mac NIC changed", because 12b ran on the same Aquantia adapter at MTU 1500 and P1 still failed at 1.237. Verify that claim against machines.md and the 12b CSVs. Is the control correctly identified? Is it needed only on a PASS, as the doc says, or also on a FAIL or INDETERMINATE outcome?

2. THE VOID-ROWS CLAIM. The doc asserts that at jumbo every "old_committed" and "cross ... min_old_committed" verdict row is void (because the committed baseline otp2w-baseline-2026-07-10 was recorded at MTU 1500), while invariance rows and same-session converge rows stay sound. VERIFY THIS AGAINST THE HARNESS CODE, not against the doc. Is it correct? Does it have the consequence the doc claims for P2's bar? Does anything else in the harness silently depend on the committed baseline?

3. THE DECISION RULE. Bands are r<=1.10 PASS, r>=1.20 "MTU is not the cause", 1.10<r<1.20 INDETERMINATE (escalate to RUNS=8). Masking guards: the fast arm (win_init) must stay <=1033ms and the slow arm (mac_init) must fall to <=1024ms (>=70% of the 282ms gap). Are these thresholds sound and consistent with the parent plan's own UNIFORM rule and Fix criteria? Is the r>=1.20 band safe — could MTU be a real contributing cause and still leave r>=1.20? Is a 4-sample median adequate for any of these calls? Are the two guards actually sufficient to catch a bottleneck-compression (masking) artifact, where both arms compress toward a shared floor and the ratio falls without anything being fixed?

4. INSTRUMENT. The doc rests on a measured MSS of 8948 in both directions (macOS getsockopt(TCP_MAXSEG); Linux ss -ti) and DISCARDS the Windows NIC packet counter as coalescing (it reported 10680 bytes per received "packet", larger than a 9014-byte frame). Is the MSS evidence sufficient to conclude that blit's own TCP data plane runs jumbo segments during the timed transfers? Name precisely what it does not establish.

5. GOVERNANCE. Does running this experiment conflict with the ACTIVE plan's pf-1 HARD GATE ("no fix lands on pre-pf-1 evidence"), its Contract stop-and-amend rule, or any entry in docs/DECISIONS.md? Does the doc overclaim what a PASS would license?

Output a concise markdown findings list — each finding with file:line, severity, and rationale — then a final VERDICT line (READY or NOT READY, with the blockers named). Be concise; do not invoke skills.
codex
I’ll trace each claimed control, metric, and threshold back to the harness, CSVs, machine record, and governing plans, then report only experiment-design defects with exact anchors.
exec
/bin/zsh -lc "pwd && rg --files -g 'AGENTS.md' -g '.agents/repo-guidance.md' -g '.agents/state.md' -g '.agents/machines.md' -g 'docs/DECISIONS.md' -g 'docs/plan/OTP12_PERF_FINDINGS.md' -g 'docs/plan/ONE_TRANSFER_PATH.md' -g 'scripts/bench_otp12_win.sh' -g 'docs/bench/otp12-win-2026-07-12/{summary.csv,verdicts.csv}' -g 'docs/bench/otp12c-win-2026-07-13/{summary.csv,verdicts.csv}' -g 'docs/bench/otp2w-baseline-2026-07-10/summary.csv' -g 'docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md' && git status --short --branch && git remote -v && git ls-remote origin HEAD && git rev-parse refs/remotes/origin/HEAD 2>/dev/null || true" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/Users/michael/Dev/blit_v2
AGENTS.md
docs/DECISIONS.md
scripts/bench_otp12_win.sh
docs/plan/ONE_TRANSFER_PATH.md
docs/plan/OTP12_PERF_FINDINGS.md
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md
docs/bench/otp2w-baseline-2026-07-10/summary.csv
docs/bench/otp12c-win-2026-07-13/verdicts.csv
docs/bench/otp12c-win-2026-07-13/summary.csv
docs/bench/otp12-win-2026-07-12/verdicts.csv
docs/bench/otp12-win-2026-07-12/summary.csv
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-qPGkwdsR' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-l08tW8ln' (errno=Operation not permitted)
## master...origin/master [ahead 1]
?? .review/results/pf-0-prereg.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-yhcySGU0' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1p2OZZKs' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-8u42PETu' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-SfjVg1xx' (errno=Operation not permitted)
fatal: unable to access 'http://q:3000/michael/blit_v2.git/': Could not resolve host: q

codex
The canonical remote is unreachable from this sandbox, so I’m treating recorded state as potentially stale; the commit and all requested local evidence are available. The worktree also has one unrelated untracked review file, which I’ll leave untouched.
exec
/bin/zsh -lc 'nl -ba AGENTS.md && nl -ba .agents/repo-guidance.md && nl -ba .agents/state.md && nl -ba .agents/machines.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# Agent Guidance
     2	
     3	## Prime Invariants
     4	<!-- prime:begin — keep terse; re-grounded after compaction -->
     5	These outrank everything below. After a context compaction, re-read this block from AGENTS.md before continuing.
     6	
     7	- Words first. Answer questions and musings in words; act only on an explicit instruction or go. A handed-over report, plan, or spec is evidence to assess, not a decision to implement.
     8	- No code change without an approved plan; docs and other non-code edits don't need one (e.g. a README). When unsure, treat it as code. Do not expand scope without approval.
     9	- Commit each slice as it lands; never leave finished work uncommitted. History-rewrite and destructive or outward-facing actions always need an explicit go. Push policy: see `.agents/push-policy.md`.
    10	- Repo is memory. Durable truth lives in the repo, not chat or working memory. Under context pressure, re-ground from AGENTS.md; prefer a fresh session when degraded.
    11	<!-- prime:end -->
    12	
    13	## Repo-Specific Guidance
    14	
    15	@.agents/repo-guidance.md
    16	
    17	Repo-specific rules live in `.agents/repo-guidance.md`, imported above (read it directly if your harness does not process `@` imports). It extends this file and never overrides it — flag any genuine conflict.
    18	
    19	## Universal Invariants
    20	
    21	- The Prime Invariants above are the hardest-to-reverse rules; this section adds the rest.
    22	- Agent-local or harness-local memory stores kept outside the repo are not durable memory, on any harness. Persist project-specific durable knowledge into the repo's `.agents/` files; reserve out-of-repo stores for genuinely cross-project facts (owner identity, preferences).
    23	- Record important repo facts, decisions, invariants, verification rules, non-goals, and open questions in repo files, or explicitly report them as unrecorded. Write them generalized, tied to repo evidence or explicit human intent, so they make sense without the conversation that produced them — never as transient chat wording. Label inferred-but-unverified facts as assumptions until repo evidence or explicit human approval supports them.
    24	- Keep one canonical location for each durable truth. Prefer pointers over duplicating the same rule; never keep a second copy of a count or enumeration another doc owns.
    25	- Establish one immediately discoverable current-state entry point (`.agents/state.md`). Do not reconstruct current state from chat, long journals, or tool-local memory.
    26	- When repo documents disagree, flag the conflict instead of silently choosing whichever source is convenient. Code and tests are evidence for behavior; approved plans and guidance are evidence for intent.
    27	- Specific over generic: an explicit authority or scope boundary, or a rule or decision whose wording removes discretion for the case it names ("unconditional", "no per-run choice", "deterministic"), outranks every generic default for that case — flag-conflicts, one-canonical-location, smallest-guidance-set included. Apply it as written; do not reopen the case it settles as a conflict or approval question against surrounding repo state such as git history. Generic defaults govern only questions no more specific rule has already resolved.
    28	- Prefer the smallest durable guidance set that fits the repo.
    29	- Do not circumvent a roadblock whose provenance you have not established — a failing test, a guard or assertion, a lint or type error, a `.gitignore` rule, a refusal or permission denial, a config prohibition, a CI gate. Before removing or bypassing one, inspect its origin thoroughly enough to confirm it is not load-bearing; if you cannot, treat it as legitimate and stop or ask.
    30	- Escalate an iterative process on stalled progress, never on duration. Each cycle must bank a verifiable delta — a test moving red→green, a finding closed with its guard proof, a build or type error resolved, a committed slice; a cycle that produces none is a stall. After a few consecutive stalled cycles (state the threshold you are using; default ~2-3), stop and surface to a human. A long run that banks a delta each cycle is healthy and must not be capped on duration or turn count.
    31	- `AGENTS.md` is governance only — it must be portable. The test: would this line still be true and useful if copied unchanged into an unrelated repo? Process, invariants, and operator definitions pass. Anything true only of *this* repo — a concrete source path, the repo's own name as a fact, its verification commands, a restatement of current state or the decisions queue — fails and lives in `.agents/`, with `AGENTS.md` pointing to it, never restating it. References to the toolkit's own standard layout — `.agents/state.md`, operator names — are portable and allowed.
    32	- `AGENTS.md` is the toolkit template, installed and replaced whole by governance refresh; no agent hand-edits it. Durable repo-specific rules go to `.agents/repo-guidance.md` and facts to the other `.agents/` files; a proposed `AGENTS.md` edit is out of bounds — question it, do not perform it.
    33	
    34	## Session Startup
    35	
    36	1. Read `AGENTS.md`, `.agents/repo-guidance.md`, and `.agents/state.md` if present, plus relevant `.agents/` files, before making changes; note any untracked or ignored agent-control files that affect the task.
    37	2. Clone freshness: before trusting `.agents/state.md`, compare this clone against its canonical remote with a read-only check (`git ls-remote <remote> HEAD` against the local ref). Behind or diverged — say so and treat recorded state as possibly stale; unreachable — proceed with a one-line caveat, never block.
    38	3. This repo ships a compaction re-ground hook (Claude Code; other harnesses only as listed in the toolkit's harness-capabilities record); if your harness gates hooks until the workspace is trusted, say what the hook does and run the trust step only on an explicit go — never bypass the gate.
    39	
    40	## Source Of Truth
    41	
    42	1. Human request.
    43	2. `AGENTS.md`, extended by `.agents/repo-guidance.md` (extends, never overrides).
    44	3. `.agents/state.md` for current work; `.agents/decisions.md` for settled decisions; approved `.agents/playbooks/*`.
    45	4. Current code, tests, and CI as evidence for behavior.
    46	5. Existing docs, only when consistent with current repo evidence.
    47	
    48	When sources disagree, apply the flag-conflicts invariant (Universal Invariants): surface the conflict and fix the lower-authority source, or ask which should win.
    49	
    50	## Operator Requests
    51	
    52	Treat these owner words as process requests:
    53	
    54	- `catchup`: re-read `AGENTS.md` (the Prime Invariants in full), `.agents/state.md`, and active repo docs; summarize current state, next action, blockers, and one proposed first action. Make no changes until the human responds.
    55	- `handoff`: a fast save-my-place snapshot — seconds, not minutes. Update `.agents/state.md` `## Now` / `## Next` (and `## Blockers` if something is live) so the next session resumes without chat context: in-flight work, next action, stop. No archive rotation, no re-verification sweep, no mandatory re-anchoring of volatile facts — that hygiene belongs to `drift`. Machine-specific facts (CLI paths, local tool versions, host layout) go to the tracked `.agents/machines.md` under a heading for the current machine, dated, created on first use — never into `.agents/state.md`, which stays portable and may at most point there.
    56	- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves — `AGENTS.md` and `.agents/*` — are in scope as drift targets, not just sources of truth. `drift` also owns the deliberate state-hygiene pass: rotate landed or superseded `## Now` entries verbatim to `docs/history/state-archive.md` (create on first use); re-verify the recorded basis of every parked or blocked item and move anything falsified into `## Blockers` with the new evidence; volatile facts (CI state, counts) carry `as of <commit>` and are re-verified or dropped; push status is never recorded in state files — git owns it, sessions check it live, and unpushed work is mentioned only in the moment it matters — so any recorded push-state line is deleted on sight, not refreshed; a count or enumeration another file owns is pointed to, never copied; machine-specific facts relocate to `.agents/machines.md`, and stale entries there are pruned.
    57	- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
    58	- `plan`: draft or update a durable plan before broad implementation work. Plan documents are written for agents, never the owner: self-contained and technical, implementable by a completely cold, less-capable agent — no human-facing summary prose, no chat or session references that need the originating conversation to make sense. The owner does not read plan documents; present every decision a plan needs in chat as roughly 25-50 plain-English words — the problem, the change, the cost or risk — one decision at a time, never a batch, no jargon. Record the owner's approved wording durably (the decisions log, the plan's status line) so the approval survives the chat.
    59	- `playbook <name>`: read `.agents/playbooks/<name>.md` and follow it. Playbooks are approved durable workflows; this operator is how a session invokes one by name. If the named playbook does not exist, say so rather than guessing.
    60	
    61	## Verification
    62	
    63	Use the repo's current automated verification entry point recorded in `.agents/repo-guidance.md` (Verification).
    64	
    65	- For code changes, run the current automated verification before claiming completion.
    66	- When a change ships with a new test, prove the test guards it: temporarily revert the change, confirm the test fails, restore it, confirm everything passes. A test that passes with its fix reverted is vacuous and must be replaced.
    67	- For docs-only changes, code verification is not required unless the docs affect setup, commands, runtime behavior, generated files, or user-visible behavior.
    68	- For behavior that automation does not cover, run the relevant manual check, smoke test, or playtest, or state clearly that it was not run.
    69	- If no verification entry point is recorded yet, identify the likely command from repo evidence, record it, and label uncertainty. Ask the human only when evidence conflicts, no plausible command exists, or the command appears destructive, expensive, credentialed, or otherwise unsafe to run automatically.
    70	
    71	## Git Safety
    72	
    73	- Never conclude a branch is merged from ancestry alone: `git branch --merged` can lie after an `-s ours` or octopus merge records ancestry without content. Verify the content actually arrived (`git diff <branch> <main>`) before deleting anything or treating work as landed.
    74	- When working through a list of findings or fixes, address exactly one item per commit and commit each before starting the next. Batch sweeps spanning many findings happen only on the owner's explicit request. Whether work happens on a branch is this repo's policy, not this rule's.
    75	- Do not rewrite history or restructure existing commits without explicit owner approval: no `git commit --amend`, `rebase`, `squash`, or force-push, and no reordering or collapsing commits already made. The owner's approval authorizes the scoped commit as announced — it does not authorize a later rewrite of it. Default to a new commit per fix; if history genuinely needs reshaping, stop and ask.
    76	
    77	## Final Response
    78	
    79	Explain what changed, what was validated, and any remaining risk in plain English.
     1	# Repo-Specific Guidance
     2	<!-- Extends AGENTS.md; never overrides it. Rules and pointers only — state
     3	     lives in .agents/state.md. -->
     4	
     5	## Mission Detail
     6	
     7	Blit is a high-performance, extensible file enumeration, planning, transfer,
     8	and orchestration platform for local and remote backups, migration, and
     9	cross-platform syncing, with CLI and daemon interfaces (`crates/blit-cli`,
    10	`crates/blit-daemon`), async-aware planning, and Windows/Linux/macOS support.
    11	
    12	## Reading Order
    13	
    14	This repo predates the toolkit's `.agents/state.md` / `.agents/decisions.md`
    15	convention and keeps its own canonical files at different paths; the
    16	`.agents/` files below are pointer stubs, not duplicates. Read in this order:
    17	
    18	1. `docs/STATE.md` — single entry point for current active work, queue, and
    19	   blockers (the canonical equivalent of `.agents/state.md`; see
    20	   `.agents/state.md` for why the path differs).
    21	2. The active plan doc(s) `docs/STATE.md` names (under `docs/plan/`).
    22	3. `REVIEW.md` + `.review/` — review-loop status for in-flight findings.
    23	4. `docs/DECISIONS.md` — settled decisions and supersessions (the canonical
    24	   equivalent of `.agents/decisions.md`).
    25	5. `docs/agent/PROTOCOL.md` — the executable procedures behind the trigger
    26	   vocabulary (`catchup`, `plan`, `decision`, `handoff`, `drift`, plus the
    27	   repo-specific `slice` operator below).
    28	6. Everything else in `docs/` — reference or historical; check its
    29	   `**Status**:` header.
    30	7. Code and tests are ground truth for behavior; plans are ground truth for
    31	   intent. A mismatch is a drift finding, not permission to pick whichever is
    32	   convenient.
    33	
    34	`DEVLOG.md` is append-only history — write to it, never read it for current
    35	state. `TODO.md` is the long-horizon backlog; the actionable queue lives in
    36	`docs/STATE.md` and `REVIEW.md`. `.serena/memories/` and any tool-local
    37	memory are scratch, never authoritative.
    38	
    39	## Operator Vocabulary (repo-specific extension)
    40	
    41	`AGENTS.md`'s Operator Requests section defines the toolkit's generic
    42	vocabulary (`catchup`, `handoff`, `drift`, `decision`, `plan`, `playbook`).
    43	In this repo every one of those words resolves to a procedure in
    44	`docs/agent/PROTOCOL.md`, not to the generic `.agents/state.md`/
    45	`.agents/decisions.md` files directly — read the matching section there and
    46	execute it exactly:
    47	
    48	- `catchup` → re-ground from `docs/STATE.md` + active docs; summarize
    49	  now/next/blockers.
    50	- `plan <topic>` → interview the owner, write `docs/plan/<NAME>.md`; no code
    51	  until `**Status**: Active`.
    52	- `decision <topic>` → record in `docs/DECISIONS.md`, propagate
    53	  supersessions.
    54	- `handoff` → update `docs/STATE.md` for the next session; prune to caps.
    55	- `drift [scope]` → audit a doc against code; fix docs, file findings, raise
    56	  questions.
    57	- `slice` (repo-specific, no generic-template equivalent) → pick up the next
    58	  review finding and run it through the codex review loop
    59	  (`docs/agent/GPT_REVIEW_LOOP.md`).
    60	
    61	**Review policy (D-2026-07-04-1): every code change and every plan change
    62	goes through the codex review loop in `docs/agent/GPT_REVIEW_LOOP.md` — no
    63	exceptions.** The `.review/README.md` async sentinel hand-off is retired;
    64	its `findings/`/`results/` records and `REVIEW.md` remain the record store.
    65	
    66	Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
    67	Antigravity exposes `catchup`/`handoff` as workspace skills in
    68	`.agents/skills/`. This repo drafts `.agents/playbooks/reviewloop.md` as a template, but the codex review loop and `docs/agent/PROTOCOL.md` already cover that role for review-loop work.
    69	
    70	## Verification
    71	
    72	```bash
    73	cargo fmt --all -- --check
    74	cargo clippy --workspace --all-targets -- -D warnings
    75	cargo test --workspace
    76	```
    77	
    78	- Test count may grow but never drop versus the prior baseline unless the
    79	  removal is called out in the finding doc's Known gaps.
    80	- Windows parity: after touching platform-specific code (`win_fs`, planners),
    81	  run `scripts/windows/run-blit-tests.ps1`.
    82	- Docs gate (CI): a push touching `crates/**` or `proto/**` must also touch
    83	  `docs/STATE.md`, unless the commit message contains `[state: skip]`
    84	  (reserved for mechanical changes). `scripts/agent/check-docs.sh` must pass;
    85	  run it locally before pushing docs changes.
    86	- This section is the canonical home for the verification commands (the
    87	  `.agents/repo-map.json` mirror was retired 2026-07-08 with the toolkit's
    88	  JSON layer).
    89	
    90	## Remotes & Sync
    91	
    92	- `origin` — `http://q:3000/michael/blit_v2.git` (**LAN gitea**). This is
    93	  what a bare `git push` / `git ls-remote origin` talks to.
    94	- `github` — `https://github.com/roethlar/Blit.git` (**GitHub**).
    95	- **`git push` does NOT update GitHub.** The two remotes are independent
    96	  and nothing auto-syncs them: pushing `origin` moves the LAN gitea only,
    97	  and GitHub needs its own explicit `git push github`. Either can lag the
    98	  other by many commits. A ref-listing before a push must therefore name
    99	  the remote's **URL**, not just "origin" — the name alone does not say
   100	  which host is being published to.
   101	- **CORRECTED 2026-07-13** against `git remote -v` (evidence in the
   102	  session's push output: a bare `git push` printed
   103	  `To http://q:3000/michael/blit_v2.git`, while `git push github` printed
   104	  `To https://github.com/roethlar/Blit.git` and fast-forwarded GitHub
   105	  `6d37a22..be1989d` — i.e. GitHub had been dozens of commits stale while
   106	  every agent push went to the mirror). The prior text had the two names
   107	  **inverted** (`origin` = GitHub, plus a `gitea` remote that does not
   108	  exist) and asserted it had been "verified against `git remote -v`
   109	  2026-07-04" while disparaging an earlier revision that was in fact
   110	  correct. Do not re-invert this without re-reading `git remote -v`.
   111	- Push policy: `.agents/push-policy.md` (ask). This repo's git-safety rules
   112	  go well beyond a simple push policy — see Earned Practices below.
   113	
   114	## Earned Practices
   115	
   116	These are absolute; they exist because an unapproved `git merge -s ours`
   117	octopus (commit `c793df2`) was pushed to `origin/master` without the owner's
   118	consent (`docs/DECISIONS.md` D-2026-06-07-1).
   119	
   120	- **No agent-created branches.** Agents never create git branches on their
   121	  own decision. All work happens on `master` or the branch the owner already
   122	  checked out.
   123	- **Owner is the sole gate for git operations that publish, rewrite, or
   124	  destroy.** No `push`, `push --force`/`--force-with-lease`,
   125	  `reset --hard`, rebase or other history rewrite, `commit --amend` on
   126	  pushed commits, or deletion of any branch/tag/ref (local or remote)
   127	  without the owner approving that exact action in the current session.
   128	  Working-tree edits, local commits, and read-only inspection
   129	  (`status`/`log`/`diff`/`show`) need no special approval.
   130	- **Branch deletion is by explicit name only** — the owner names the branch,
   131	  the agent deletes that branch.
   132	- **Before any push:** list the exact local refs, remote refs, and
   133	  destination remotes, then stop and wait for approval.
   134	- **`--merged`/`--no-merged` are unreliable in this repo.** The `-s ours`
   135	  octopus made two now-abandoned branch tips ancestors of `master`, so
   136	  `git branch --merged master` falsely lists them as merged and a plain
   137	  `git merge` of those branches no-ops without landing any code
   138	  (`docs/DECISIONS.md` D-2026-06-07-2). Verify content actually arrived
   139	  (`git diff <branch> master`) before treating anything as landed or
   140	  deleting it.
   141	- **Checkpoints are owner-only.** Only an explicit owner message satisfies a
   142	  checkpoint or verification step. Agents report observations; the owner
   143	  declares pass/fail. Never self-certify a gate or continue a plan past one
   144	  because the condition appears met. Approvals are single-use, step-specific,
   145	  never carried across sessions. When the owner asks a question or thinks out
   146	  loud, answer in plain English and stop — act only on an explicit decision.
   147	
   148	## Style
   149	
   150	- Rust edition 2021; format with rustfmt. Modules snake_case, types
   151	  PascalCase, constants SHOUT_CASE; match existing names
   152	  (`transfer_session`, `TransferSession`, `PLAN_OPTIONS`).
   153	- No blocking calls inside async contexts (use async send APIs in Tokio).
   154	- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
   155	  tests deterministic; capture long logs under `logs/`.
   156	
   157	## Project Map
   158	
   159	- `crates/blit-core/` — core library (enumeration, planner, the unified
   160	  `transfer_session` — every transfer, local included, since otp-11);
   161	  most logic and unit tests live here. New modules get re-exported in
   162	  `crates/blit-core/src/lib.rs`.
   163	- `crates/blit-cli/`, `crates/blit-daemon/` — CLI and daemon binaries; admin
   164	  verbs (scan, ls, find, du, df, rm, completions, profile, list-modules) live
   165	  in `blit-cli` alongside transfer commands.
   166	- `crates/blit-app/`, `crates/blit-tui/` — TUI application layers.
   167	- `crates/blit-prometheus-bridge/` — metrics bridge.
   168	- `proto/blit.proto` — gRPC definitions; `blit-core`'s build script vendors
   169	  protoc.
   170	- Integration tests live per-crate (`crates/blit-cli/tests/`,
   171	  `crates/blit-core/tests/`); the root `Cargo.toml` is a virtual workspace,
   172	  so a root-level `tests/` dir would never be compiled. `scripts/` — helper
   173	  tooling.
   174	- `blit-utils` was intentionally removed; its admin verbs now live in
   175	  `blit-cli`. Do not re-add it as a workspace member.
   176	
   177	## Commit and Docs Hygiene
   178	
   179	- Commit subject: short imperative ("Add streaming planner heartbeat").
   180	  Review-loop commits: `Fix <id>: <one-line summary>` per `.review/README.md`.
   181	- After meaningful work: append a `DEVLOG.md` entry (newest-first, ISO
   182	  timestamp) and update `docs/STATE.md` — the `handoff` procedure does both.
   183	- Every doc in `docs/plan/` carries a `**Status**:` header, one of:
   184	  `Draft | Active | Shipped | Superseded | Historical`. Superseding a doc
   185	  requires a `docs/DECISIONS.md` entry naming winner and loser, and an edit
   186	  to the superseded text.
   187	- `docs/STATE.md` stays ≤ 200 lines with ≤ 3 handoff entries; prune the
   188	  overflow into `DEVLOG.md`.
     1	# Agent State (pointer)
     2	
     3	Current state for this repo lives canonically in **`docs/STATE.md`** — read that
     4	file, not this one. This stub exists only so tools expecting the standard
     5	`.agents/` layout find their way.
     6	
     7	Do not record state here. The path `docs/STATE.md` is load-bearing: the CI docs
     8	gate (`.github/workflows/docs-gate.yml`), the doc lint
     9	(`scripts/agent/check-docs.sh`), the session hooks (`scripts/agent/context.sh`),
    10	and the `handoff` procedure in `docs/agent/PROTOCOL.md` all read or enforce it.
    11	
    12	- Now / Next / Blockers: `docs/STATE.md`
    13	- Verification commands: `.agents/repo-guidance.md` (Verification)
    14	- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`
     1	# Machine-local facts (per AGENTS.md `handoff`: never in docs/STATE.md)
     2	
     3	## This Mac (owner's workstation) — recorded 2026-07-10, moved here 2026-07-11
     4	
     5	- Rig SSH keys installed: zoey (root), Windows box (`michael@10.1.10.173`),
     6	  skippy (`admin@skippy`).
     7	- NOPASSWD sudoers rule for the zoey pool-drain/purge helper.
     8	- zig + cargo-zigbuild toolchain (aarch64-musl static daemon builds).
     9	- ssh ControlMaster sockets configured for the rigs.
    10	
    11	## Additional Linux hosts — BUILD ONLY (owner rule, 2026-07-12)
    12	
    13	Owner: "Use it only for building binaries. Same with the VM. Build
    14	only if needed." Neither is a benchmark end, ever — and native builds
    15	there are a FALLBACK only (the Mac's zigbuild already cross-compiles
    16	every Linux target in play).
    17	
    18	- `michael@magneto` — Arch Linux x86_64 (kernel 7.1.3-arch1-1), 4
    19	  cores, 32 GiB RAM, 10 GbE, WD SN850 NVMe. ~~Busy BitTorrent machine —
    20	  never a bench end~~ **SUPERSEDED 2026-07-13: magneto IS a valid bench
    21	  end** (owner: "power efficient intel, but it should be fast enough to
    22	  saturate 10GbE where Zoey is definitely not"; services quiescable on
    23	  request). Only ONE NIC is in use: `enp1s0f1` = 10.1.10.10 (the other
    24	  three have no IP). Used as the **same-OS rig** magneto↔skippy —
    25	  the only pair in the fleet that can measure blit's layout with zero
    26	  platform terms. NOPASSWD `drop_caches` grant added 2026-07-13.
    27	- `michael@altiera` — Linux, 2.5 GbE (enp1s0=10.1.10.53,
    28	  enp2s0=10.1.10.59, both MTU 9000). **NOT usable as a bench end**: at
    29	  2.5 GbE the link (~312 MB/s) is slower than the fixtures need, so it
    30	  bottlenecks BOTH arms of an invariance pair equally, dragging the
    31	  ratio toward 1.0 and MASKING the effect under test — zoey's failure
    32	  mode by a different route.
    33	
    34	## Network / MTU (rig-critical — read before touching MTU)
    35	
    36	- **THE macOS PING TRAP (cost ~1h on 2026-07-13; do not repeat).**
    37	  macOS caps **raw sockets** at 8192 bytes via `net.inet.raw.maxdgram`,
    38	  and `ping` uses a raw socket. So DF pings above ~8164 payload FAIL
    39	  from a Mac **no matter what the real path MTU is**. This is a limit on
    40	  the ping tool, NOT on the network, and it does **not** affect TCP.
    41	  I misread it as "macOS cannot transmit jumbo frames", blamed the
    42	  switch, then blamed two innocent adapters, and had the owner swap
    43	  hardware for nothing. **Verify jumbo with a real TCP transfer** (e.g.
    44	  `scp` a large file), never with `ping`.
    45	- **Jumbo works end-to-end at MTU 9000** (verified 2026-07-13 by real
    46	  TCP, not ping): Mac↔Windows 231/225 MB/s, Mac↔skippy 157 MB/s (all
    47	  ssh-encrypted, so CPU-bound floors — the wire is not the limit). The
    48	  UniFi switching passes 9018-byte frames fine.
    49	- **Windows (netwatch-01) ran at MTU 1500 for EVERY benchmark ever
    50	  recorded** (otp-2w, otp-12a/b/c). It was raised to 9000 on 2026-07-13.
    51	  Every prior measurement therefore negotiated down to a 1460-byte MSS:
    52	  **jumbo has never been exercised in a blit benchmark.** Those numbers
    53	  are valid — they are simply *1500-MTU* numbers — and rig W at jumbo is
    54	  a genuinely untested condition. magneto is still 1500 (raise
    55	  `enp1s0f1` to 9000 to make the Linux rig jumbo too).
    56	- Mismatched MTUs on one L2 segment are fine: TCP MSS negotiation
    57	  handles it, each host advertising what it can receive. What is NOT
    58	  fine is a host advertising a size it cannot actually send.
    59	- **Fleet MTU as of 2026-07-13 — the whole 10 GbE fabric is now 9000:**
    60	
    61	  | host | iface | MTU | persistent? |
    62	  |---|---|---|---|
    63	  | Mac | `en9` (Aquantia) | 9000 | yes (macOS net service) |
    64	  | netwatch-01 | Ethernet | 9000 | yes (raised 1500→9000 today) |
    65	  | skippy | `enp66s0f1` | 9000 | yes |
    66	  | **zoey** | `enp0s0` (RJ45, NFS data .206) | **9000** | yes — `[Link] MTUBytes=9000` in `/etc/systemd/network/enp0s0.network` |
    67	  | **zoey** | `enp0s1` (SFP, mgmt .210) | **9000** | yes — same, in `enp0s1.network` |
    68	  | altiera | `enp1s0`/`enp2s0` | 9000 | yes (NetworkManager profiles) |
    69	  | magneto | `enp1s0f1` | 9000 | yes — NM profile `Wired connection 3` saved `mtu=9000` (2026-07-13) |
    70	
    71	  **Verified end-to-end 2026-07-13**: a jumbo DF ping from skippy reaches
    72	  magneto, zoey, altiera, netwatch-01 AND the Mac — all OK. Every 10 GbE
    73	  pair in the fleet carries 9000-byte frames. (Always test from a LINUX
    74	  host; the Mac's `ping` cannot send >8192 — see the raw-socket trap.)
    75	
    76	- **zoey (UniFi UNAS Pro) jumbo — how it was done, and the trap.**
    77	  Debian 11 + `systemd-networkd`; NIC `maxmtu` is 9216 so the hardware is
    78	  fine. Persistence = a `[Link]` / `MTUBytes=9000` stanza in each
    79	  `/etc/systemd/network/enp0s*.network` (originals backed up as
    80	  `*.premtu`). Proven by `networkctl reload && networkctl reconfigure`
    81	  with the static IP intact — no reboot needed. **TRAP: `/` is an
    82	  overlayfs** (`lowerdir=/mnt/.rofs` read-only + writable upper), so a
    83	  UniFi *firmware update* can replace the base image and silently drop
    84	  this. Re-check after any UNAS update:
    85	  `ssh root@zoey 'cat /sys/class/net/enp0s0/mtu'` → want 9000.
    86	  Method for any risky remote NIC change: arm a self-healing revert
    87	  first — `nohup setsid bash -c 'sleep 90; [ -f /tmp/ok ] || ip link set
    88	  IFACE mtu 1500' &` — then confirm with `touch /tmp/ok`. Change the NIC
    89	  you are NOT ssh'd through when a second one exists.
    90	- **Live NFS/TCP connections do NOT pick up a new MTU.** MSS is fixed at
    91	  connect time, so an existing mount keeps its old segment size until it
    92	  reconnects (reboot/remount). Not worth forcing for low-bandwidth
    93	  mounts.
    94	- Two-NICs-on-one-subnet (both `altiera` and `zoey`, and it is the
    95	  default `arp_ignore=0 arp_announce=0`) invites ARP flux + asymmetric
    96	  routing. Working today; a latent source of intermittent stalls.
    97	- Local VM on the Mac — Ubuntu ARM (aarch64), per owner. Build-only
    98	  fallback likewise.
    99	
   100	## Rig residue (recorded 2026-07-10)
   101	
   102	- **The Mac's 10GbE IP and NIC CHANGED 2026-07-13** — this is a live
   103	  confound in the otp-12 numbers, not a bookkeeping detail:
   104	  * **now: `en9` = 10.1.10.54**, a Thunderbolt **Aquantia** adapter,
   105	    MTU 9000, 10Gbase-T. (SSH into the Mac = `michael@10.1.10.54`;
   106	    Remote Login is ON and netwatch-01's key is in the Mac's
   107	    `authorized_keys`, so Windows→Mac ssh/sftp works.)
   108	  * otp-12b (`wm_tcp_mixed` **1.237**) ran on the Aquantia at
   109	    **10.1.10.54**; otp-12c (**1.300**) ran on a Thunderbolt-5 dock's
   110	    built-in 10GbE at **10.1.10.91**. **Different NICs.** So the
   111	    "1.237 → 1.300, it got worse at the cutover sha" reading is
   112	    CONFOUNDED by a hardware change and must not be cited as evidence
   113	    of a code regression. Both runs still showed the same qualitative
   114	    asymmetry; only the delta is suspect.
   115	  * Harnesses take the Mac IP via `MAC_HOST=` — pass **10.1.10.54**
   116	    (older invocations in the DEVLOG say 10.1.10.91).
   117	- Windows box = **`michael@netwatch-01`, IP 10.1.10.177 as of
   118	  2026-07-12** (the earlier-recorded 10.1.10.173 is STALE — DHCP; ssh
   119	  by hostname; if the bare name stops resolving, `netwatch-01.local` or
   120	  the IP both work — the host key is filed under both). **MTU raised
   121	  1500 → 9000 on 2026-07-13** (see Network/MTU above). SMB File Sharing
   122	  is now ON on the Mac and Windows is authenticated to it
   123	  (`net use \\10.1.10.91\blit-bench-work`), so robocopy can reach it.
   124	  Rules: `blit-bench-daemon` (otp-2w, repo-path-scoped)
   125	  + `blit-otp12-daemon` (active-path-scoped) + staged
   126	  `purge-standby.ps1`; repo checkout DETACHED at `e21cf84` since the
   127	  otp-12b session (owner's `bench-cargo-lock` stash untouched); old
   128	  `0f922de` exes aside-copied at `D:\blit-test\bins\0f922de\`; run
   129	  bins under `D:\blit-test\bins\<sha>\`.
   130	- **Rig pairing constraint (owner, 2026-07-13): zoey's CPU is too slow
   131	  to be a match for skippy** — a zoey↔skippy pair is NOT a valid
   132	  symmetric/performance-matched rig; a zoey endpoint becomes the
   133	  bottleneck and MASKS data-plane effects rather than measuring them.
   134	  Consequence, recorded so it is not re-proposed: the fleet has **no
   135	  same-OS, real-network, performance-matched pair** (one Mac; zoey too
   136	  slow for skippy; magneto is a busy BitTorrent box — build-only, never
   137	  a bench end). Platform-vs-role confounds on a two-host rig therefore
   138	  cannot be broken by rig juggling and need a code-level counterfactual
   139	  (see `docs/plan/OTP12_PERF_FINDINGS.md`).
   140	- zoey: binaries staged 2026-07-10 in `blit-temp/` — **corrected
   141	  2026-07-12**: the staged daemon embeds `731023bfc8a1.dirty.…`, NOT
   142	  `e757dcc` as previously recorded (otp-2 README carries the full
   143	  correction note; daemon code is identical between the two commits).
   144	  Kept untouched as the otp-2 artifact; otp-12a stages clean sha-named
   145	  rebuilds beside it (`blit-daemon-e757dcc`, `blit-daemon-<run sha>`).
   146	  blit-temp path: `/volume/a595ddbf-d201-4e55-8183-ec78c8cd83e0/.srv/`
   147	  `.unifi-drive/michael/.data/blit-temp`.

exec
/bin/zsh -lc 'git show --stat --oneline --decorate --no-renames 35b9620 && git show --format=fuller --no-ext-diff --unified=80 35b9620 -- docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md && nl -ba docs/STATE.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-KDcqJOUT' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-OWsZLJdy' (errno=Operation not permitted)
35b9620 (HEAD -> master) otp-12 jumbo re-run: pre-registration before any rig time
 .../otp12-jumbo-win-2026-07-13/PREREGISTRATION.md  | 185 +++++++++++++++++++++
 1 file changed, 185 insertions(+)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HvltTHQw' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-3l7AYtyY' (errno=Operation not permitted)
commit 35b96204ddb2cda8339561aeee3efc843d01c24e
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Mon Jul 13 18:10:00 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Mon Jul 13 18:10:00 2026 -0400

    otp-12 jumbo re-run: pre-registration before any rig time
    
    Windows sat at MTU 1500 for every benchmark ever recorded; the fleet is
    now at 9000. P1 (wm_tcp_mixed invariance, 1.300 FAIL) is the packet-
    heaviest cell, so jumbo may dissolve it. Record the design and the
    decision rule BEFORE the data exists, so the rule cannot be authored
    around the numbers.
    
    Instrument validated first (the session lesson): negotiated TCP MSS on
    the rig-W path is 8948 both directions (getsockopt(TCP_MAXSEG) + Linux
    ss -ti) vs 1448 at MTU 1500 — a MEASURED 6.18x segment reduction, not an
    assumed one. A candidate instrument was tested and DISCARDED: the Windows
    NIC counter reports 10680 bytes per received "packet", larger than a
    9014-byte frame, so it coalesces and cannot discriminate 1500 from 9000.
    
    Two non-obvious consequences recorded up front:
    - Every old_committed / cross verdict row is VOID at jumbo — the committed
      baseline was recorded at MTU 1500, so those rows flatter the new arm by
      construction. Invariance and same-session rows stay sound, so P1 is still
      measured cleanly; P2 cannot close until the baseline is re-recorded.
    - The confound is NOT "the Mac NIC changed": 12b ran on this same Aquantia
      at MTU 1500 and P1 failed at 1.237. The one combination never run is
      Aquantia x 1500 x f35702a — that, not the NIC, is the control.
    
    Pre-registered bands, fast/slow-arm masking guards, and the confirmations
    required before any "jumbo dissolves P1" claim are in the doc.
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
    Claude-Session: https://claude.ai/code/session_01F6G7FJ7JkVPqXtQMEai62o

diff --git a/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md b/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md
new file mode 100644
index 0000000..c3cc552
--- /dev/null
+++ b/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md
@@ -0,0 +1,185 @@
+# otp-12 rig-W jumbo re-run — PRE-REGISTRATION (written before any timed run)
+
+**Status**: Pre-registered. **No data exists yet.** This file is committed
+BEFORE the run so the decision rule cannot be authored after seeing the
+numbers. Results land beside it in `README.md` + CSVs; this file is not
+edited once data exists (corrections go in `README.md`, marked as such).
+
+**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (**Active**, D-2026-07-13-1);
+`docs/STATE.md` Queue 1a names this run as the next action, ahead of any
+code. **This is an ENVIRONMENTAL experiment, not a code counterfactual** —
+it is not one of H1–H7 and it changes nothing in the tree. It runs first
+because it is the cheapest experiment available and it can invalidate the
+premise of the expensive ones.
+
+## The question
+
+`netwatch-01` ran at **MTU 1500 for every benchmark ever recorded**
+(otp-2w, otp-12a/b/c — `.agents/machines.md` §Network/MTU). Jumbo has
+therefore **never once been exercised** by a blit benchmark. P1 — the
+headline invariance failure — is the **TCP × mixed × destination-initiator**
+cell, i.e. the packet-heaviest fixture we own. If per-packet cost is the
+mechanism, ~6× fewer packets is exactly where it would show.
+
+**Falsifiable premise**: MTU 1500 → 9000 is causally responsible for some
+or all of P1's 282 ms invariance gap (`Δ_P1(rig W)`, plan §decision rule).
+
+## Instrument validation — performed BEFORE the run
+
+This session has retracted three claims, all from trusting an unvalidated
+instrument. So the instrument is validated first, and the validation is
+recorded whether or not it flatters the hypothesis.
+
+| check | method | result |
+|---|---|---|
+| Mac interface MTU | `ifconfig en9` | 9000 (`en9` = **10.1.10.54**, Aquantia) |
+| Windows interface MTU | `Get-NetIPInterface` | `NlMtu` **9000** |
+| Windows NIC jumbo | `Get-NetAdapterAdvancedProperty` | `Jumbo Packet = 9014 Bytes` (adapter level, not just IP) |
+| L2 path, Win→Mac | DF ping, 8972 B payload | **OK**, 0% loss |
+| L2 path, Linux→Mac and →Win | skippy `ping -M do -s 8972` | **OK**, both |
+| L2 path, Mac→Win | DF ping, 8000 B payload | **OK** (macOS raw-socket cap of 8192 forbids more — the known ping trap, not a network limit) |
+| **negotiated TCP MSS, Mac→Win** | **`getsockopt(TCP_MAXSEG)`** on a live socket | **8948** |
+| **Mac's advertised MSS** | Linux `ss -ti` toward the Mac | **8948** (so Windows sends 8948 to the Mac) |
+| **Win's advertised MSS** | Linux `ss -ti` toward netwatch-01 | **8948** |
+
+`8948 = 9000 − 40 (IP+TCP) − 12 (TCP timestamps)`. At MTU 1500 the MSS was
+**1448**. **Segment-count reduction is therefore 6.18×, MEASURED, in BOTH
+directions of P1's cell** — not assumed. `getsockopt`/`ss` read the TCP
+control block, so this number cannot be faked by offload or coalescing.
+
+**An instrument was tested and DISCARDED.** Windows
+`Get-NetAdapterStatistics` reported **10 680 received bytes per "packet"**
+during a 1 GiB TCP transfer — *larger than a 9014-byte frame*, so that NIC
+coalesces on receive despite reporting `RSC IPv4Enabled: False`. NIC packet
+counters **cannot** discriminate 1500 from 9000 on this rig and must not be
+cited. (Recorded because it would have "confirmed" jumbo either way.)
+
+## What is held constant, and what is not
+
+| variable | 12b (2026-07-12) | 12c (2026-07-13) | THIS RUN |
+|---|---|---|---|
+| new-arm sha | `e21cf84` | `f35702a` | **`f35702a`** (same worktree, binaries verified to embed `+f35702a`) |
+| old-arm sha | `0f922de` | `0f922de` | **`0f922de`** |
+| harness | `bench_otp12_win.sh` | same | **same** |
+| Mac NIC | Aquantia @ **10.1.10.54** | TB5 dock @ 10.1.10.91 | **Aquantia @ 10.1.10.54** |
+| **Windows MTU** | **1500** | **1500** | **9000** |
+| `wm_tcp_mixed` invariance | 1.237 FAIL | 1.300 FAIL | ? |
+
+**Neither prior session is a single-variable control**, and the pre-existing
+STATE note ("the Mac's NIC also changed") understates the situation — but
+also mis-locates it. The confound is **not** the NIC in general: **12b ran
+on this very Aquantia adapter at MTU 1500 and P1 FAILED at 1.237.** So an
+Aquantia-vs-dock difference cannot by itself dissolve P1. The one
+combination **no session has ever run** is `Aquantia × MTU 1500 × f35702a`.
+That — not "the NIC" — is the control this experiment may need.
+
+## Verdict rows that this run VOIDS (important, and not obvious)
+
+The harness grades every cell against **two** references: the same-session
+old arm, and the **committed baseline** `otp2w-baseline-2026-07-10/summary.csv`
+(`bench_otp12_win.sh:105`, pre-registered and not overridable).
+
+**That committed baseline was recorded at MTU 1500.** At jumbo:
+
+- **Same-session rows stay sound.** Both arms run on today's network, so
+  `converge … old_session` and every **invariance** row (new-vs-new, one
+  session) are fair comparisons. **P1 is an invariance row — it is
+  measured cleanly at jumbo.** So is P2's same-session row.
+- **Every `old_committed` and `cross … min_old_committed` row is VOID.**
+  They compare a jumbo new arm against a 1500-MTU reference: the network
+  improved under the reference, so those rows are flattering by
+  construction. A PASS there is not evidence of convergence.
+
+**Consequence for the plan, stated up front:** P2's bar requires ≤1.10
+against **BOTH** references (`OTP12_PERF_FINDINGS.md` §Fix criteria). At
+jumbo the committed reference is stale, so **P2's committed-reference row
+cannot be satisfied until the committed baseline is re-recorded at jumbo.**
+This run does not close P2 regardless of outcome. If the fleet stays at
+jumbo, `pf-final` needs a re-baselined committed reference — that is a plan
+amendment, and it goes through the loop; it is not assumed here.
+
+## Pre-registered predictions and decision rule
+
+Reference values, `wm_tcp_mixed` (12c): dest-initiated arm (`mac_init`)
+**1221 ms**, source-initiated arm (`win_init`) **939 ms**, ratio **1.300**;
+`Δ_P1(rig W)` = **282 ms**.
+
+**If the MTU premise is TRUE**, the run shows all of:
+1. `wm_tcp_mixed` invariance ratio falls to **≤ 1.10**;
+2. it falls because the **slow arm speeds up** — `mac_init` drops toward
+   939 ms — **not** because the fast arm slows down;
+3. **absolute times move somewhere**: packet-heavy cells (mixed, small) show
+   real speedups against 12c. This is the run's built-in positive control.
+   If literally nothing moves in absolute terms, the measured MSS of 8948
+   is contradicted by the wall clock and the run is suspect, not a null.
+
+**Bands (pre-registered, no post-hoc adjustment).** Ratio `r` = the
+`wm_tcp_mixed` invariance ratio at RUNS=4:
+
+- **`r ≤ 1.10`** → P1 does not reproduce at jumbo. **This is NOT yet a
+  conclusion** — it triggers BOTH confirmations below before any claim is
+  recorded.
+- **`r ≥ 1.20`** → **MTU is not the cause.** P1 stands (12b 1.237, 12c 1.300
+  both sit here). No control run needed: the asymmetry survived a 6.18×
+  packet reduction on two different NICs. Proceed to pf-1 unchanged.
+- **`1.10 < r < 1.20`** → **INDETERMINATE.** Session-to-session drift on this
+  cell is already ~5% (1.237 → 1.300 on the same 1500 network), so a
+  4-sample median cannot resolve this band. Escalate to **RUNS=8** on the P1
+  cells (the plan's D2 escalation) before saying anything.
+
+**THE MASKING TRAP** (the failure mode that disqualified zoey and altiera as
+rigs — `.agents/machines.md`): a ratio can fall toward 1.0 because a *shared*
+bottleneck compresses both arms, not because the defect was fixed. Two
+concrete guards, both required for `r ≤ 1.10` to count:
+
+- **Fast-arm guard**: `win_init` median must not regress — it must stay
+  ≤ 939 × 1.10 ≈ **1033 ms**. If the ratio "passed" because the fast arm got
+  slower, that is degradation wearing a PASS, and it is reported as such.
+- **Slow-arm guard**: `mac_init` must fall by **≥ 70% of Δ_P1** (≥ 197 ms of
+  the 282 ms), i.e. to **≤ 1024 ms**. This is the plan's own ≥70% closure
+  threshold (§pf-1 decision rule), reused so the environmental cause is held
+  to the same bar as a code cause. A ratio that passes while the slow arm
+  barely moved means both arms drifted, not that P1 was fixed.
+
+**Required confirmations before ANY "jumbo dissolves P1" claim is recorded:**
+1. **RUNS=8 escalation** on `wm_tcp_mixed` + `pull_tcp_mixed` (P1's bar is
+   defined at RUNS=8, plan §Fix criteria).
+2. **The control run**: `Aquantia × MTU 1500 × f35702a`, `CELLS=wm_tcp_mixed,
+   mw_tcp_mixed,pull_tcp_mixed` — the one combination never run. It
+   discriminates the last live alternative: if the control **reproduces**
+   ~1.24–1.30, MTU is confirmed causal; if the control **passes at 1500**,
+   then MTU is exonerated and 12c's 1.300 was an artifact of the TB5 dock,
+   which would retroactively void 12c's P1 row rather than confirm it.
+   Note this control requires flipping the Windows MTU back to 1500 and back
+   again — a rig change, and it needs the owner's go.
+
+**What no outcome licenses.** Even a clean PASS does not by itself close P1
+under the parent plan: `OTP12_PERF_FINDINGS.md` §Fix criteria defines P1's
+bar on the netwatch-01 rig at RUNS=8 against both references, and the global
+rule requires every other cell to hold too. And it would not close P2, whose
+committed reference is void at jumbo (above). **A PASS here changes what the
+next experiment is; it does not end the investigation.**
+
+## The run
+
+Full 24-cell matrix, RUNS=4, ABBA, pair-void — a straight replication of the
+12c session with MTU as the intended difference. Full matrix rather than a
+`CELLS` subset because the controls (`mw_tcp_mixed` opposite direction,
+`wm_grpc_mixed` opposite carrier, `wm_tcp_{large,small}` opposite fixture)
+are what make P1's cell interpretable, and because cell ordering / cache and
+thermal history would otherwise differ from the session being replicated.
+
+```sh
+cd /Users/michael/Dev/blit_v2_f35702a          # clean detached worktree @ f35702a
+MAC_HOST=10.1.10.54 OLD_CLIENT_PROVENANCE_BY_BUILD=1 \
+  bash scripts/bench_otp12_win.sh
+```
+
+Staging verified before writing this file: worktree clean at `f35702a`;
+`target/release/{blit,blit-daemon}` embed `+f35702a`; Windows has
+`D:\blit-test\bins\{0f922de,f35702a}`; no stale daemon on either host; Mac
+old client at `~/blit-bench-work/bins/blit-0f922de`.
+
+**Known rig-state change made during validation** (recorded, not hidden):
+netwatch-01's `known_hosts` gained the Mac's key at its new IP 10.1.10.54
+(it only had the retired .91). No blit code, config, or fixture was touched.
     1	# STATE — single entry point for "what is true right now"
     2	
     3	Last updated: 2026-07-13
     4	
     5	- **NEXT ACTION — run the jumbo re-run BEFORE any code (Queue 1a).** Windows sat at **MTU 1500 for every benchmark ever recorded**, so jumbo was never once exercised; the whole fleet is now verified at **9000** (`.agents/machines.md` §Network/MTU). P1's failing cell is TCP × **mixed** — the most packet-heavy fixture we test — exactly where ~6× fewer packets could move the number. Re-run rig-W invariance at jumbo first; it may dissolve P1 outright. Control the confound: the Mac's NIC ALSO changed (Aquantia @ .54, was the TB5 dock @ .91), so if the asymmetry vanishes, re-run at 1500 on the SAME adapter to separate MTU from hardware.
     6	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
     7	- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
     8	- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**
     9	
    10	Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
    11	≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
    12	procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
    13	
    14	## Now (active work)
    15	
    16	- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
    17	  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
    18	  invariant (plan doc, verbatim): ONE block of transfer code;
    19	  direction/initiator/verb can NEVER affect wall time by blit's doing
    20	  — impossible by construction because the per-direction drivers and
    21	  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
    22	  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
    23	  cells. **D-2026-07-05-2: same-build peers only, refusal at session
    24	  open.** Progress (each slice through the codex loop; per-slice
    25	  detail lives in DEVLOG + `.review/`, NOT here):
    26	  - **Closed `[x]`: otp-1, otp-3 … otp-9** — the whole session machine
    27	    (contract, role drivers, daemon serving, both data planes + resize +
    28	    cancel, mirror/filters, resume, fallback carrier, delegated).
    29	    SizeMtime = data-safe skip. Detail: DEVLOG 2026-07-10.
    30	  - **otp-2 `[x]`** — baselines. zoey = per-direction reference;
    31	    Mac↔Windows = cross-direction rig (otp-2w). Evidence
    32	    `docs/bench/otp2{,w}-baseline-2026-07-10/`. Key reading: old push
    33	    trails old pull on BOTH rigs.
    34	  - **otp-10 `[x]`** — verb cutover + **THE CUTOVER DELETION**: 4
    35	    drivers + `Push`/`PullSync` + 13 messages out of tree AND proto
    36	    (−13.8k lines, no bridge); relay removed (D-2026-07-11-1).
    37	    Detail: DEVLOG 2026-07-11.
    38	  - **otp-11 `[x]`** — local transfers ride the session; **11b deleted
    39	    the whole old orchestration** (−6.2k lines: orchestrator, engine,
    40	    local_worker, auto_tune, change_journal — the last one an UNSOUND
    41	    fast path that silently lost data, repro in
    42	    `docs/bench/otp11-local-2026-07-11/`). The deletion-proof acceptance
    43	    line COMPLETES. Detail: DEVLOG 2026-07-12.
    44	- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
    45	  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
    46	  resume/re-derive on the unified baseline. Principle: ceiling-driven,
    47	  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
    48	- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
    49	  in Blocked); the codex loop governs all changes (D-2026-07-04-1).
    50	
    51	## Queue (ordered)
    52	
    53	1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
    54	   the only work item until it ships**: slices otp-1..13 through the
    55	   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
    56	   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
    57	   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
    58	   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
    59	   (Mac↔Windows)** `[x]`. 12a: 10 PASS, 2 to the walk. 12b — THE
    60	   INVARIANCE CRITERION: 11/12 PASS (1.003–1.057); wm_tcp_mixed 1.237
    61	   (TCP×mixed×dest-initiator, code-shaped); push_tcp_small 1.149
    62	   (both rigs); Win→Mac beats the better old direction 6/6; Mac→Win
    63	   gap shapes recorded for the walk
    64	   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
    65	   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
    66	   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
    67	   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
    68	   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
    69	   **otp-12d and otp-13 are DEFERRED, not next** — otp-12c's rows are
    70	   PRE-FIX, and `docs/plan/OTP12_PERF_FINDINGS.md` (pf-final) voids
    71	   pre-fix new arms for acceptance. Assembling the acceptance matrix now
    72	   would build otp-13's artifact from void rows.
    73	1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
    74	   (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
    75	   reviewloop slice by slice"; implementation proceeds, each slice
    76	   through the codex loop).
    77	   **RUN THIS FIRST — the cheapest experiment we have never run**:
    78	   Windows sat at **MTU 1500 for every benchmark ever recorded**, so
    79	   jumbo was never exercised; it is now at 9000 (`.agents/machines.md`
    80	   §Network/MTU). P1's failing cell is TCP × **mixed** (one big file +
    81	   5000 small), i.e. the packet-heaviest workload — exactly where ~6×
    82	   fewer packets could move the number. **Re-run rig-W invariance at
    83	   jumbo before touching any code.** Confound to control: the Mac's NIC
    84	   ALSO changed (Aquantia @ .54, was the TB5 dock @ .91) — so if the
    85	   asymmetry vanishes, re-run once with Windows back at 1500 on the SAME
    86	   adapter to prove it was the MTU and not the hardware.
    87	   **P1 misses the plan's HEADLINE criterion on rig W** (initiator/verb
    88	   invariance): `wm_tcp_mixed` FAILs twice — 1.237 and 1.300 — on tight
    89	   spreads, so not re-runnable away. (Do NOT read 1.237→1.300 as a
    90	   regression: **different Mac NICs**, see machines.md.) **But it does
    91	   NOT reproduce on a same-OS rig**: Linux both ends = **8/8 PASS**, P1's
    92	   cell at 1.092/1.003 (`docs/bench/otp12-perf-2026-07-13/`) → it is
    93	   platform-INTERACTING, not pure layout. **P1 HAS NO ESCAPE HATCH**
    94	   (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for
    95	   a cell that ALREADY passes invariance — P1 *is* the invariance
    96	   failure. So: **fix it to ≤1.10, or the owner amends acceptance
    97	   criterion 1.** Not assumed either way. P2 (`push_tcp_small` 1.105–
    98	   1.201, both rigs) is a converge bar vs the OLD build and is UNTESTED
    99	   on the Linux rig. Sequence: **jumbo re-run → pf-1 → fix → pf-final
   100	   (ALL THREE rigs) → otp-12d → otp-13.**
   101	2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   102	   Shipped (zero-copy resolved — D-2026-07-05-3). Follow-ups largely
   103	   absorbed by otp-2/otp-12's rig matrices.
   104	3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   105	   resumes/re-derives after ONE_TRANSFER_PATH ships.
   106	4. **PAUSED: design-review queue** (`REVIEW.md`; w7-1 topmost open row —
   107	   likely landed inside otp-6's one-delete-rule slice; re-check first).
   108	5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   109	   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   110	   cutover as a runtime-selected write strategy in the unified receive
   111	   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   112	   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   113	   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   114	   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   115	   no daemon runs on zoey without a fresh go.
   116	6. **Post-REV4 residue** (unowned): epoch-0/early-ADD hardening; remote
   117	   perf-history lanes (1e gap); receive-side dial tuning residue (w3-1
   118	   scoped it out); the source send half's bounded `dp.queue()` is not
   119	   raced against control-lane events (codex otp-7b-1 F3; residual: the
   120	   narrow CANCELLED→INTERNAL decay); the CLI progress monitor lives
   121	   through the in-session mirror purge (display-only; fix = the M-C
   122	   `AppProgressEvent` phase reshape — codex otp-10b-2 F5).
   123	
   124	## Authoritative docs right now
   125	
   126	- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
   127	  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
   128	  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
   129	- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
   130	  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
   131	  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
   132	  (history only).
   133	- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
   134	  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
   135	  is retired as the grading mechanism (its `findings/`/`results/`
   136	  records and the REVIEW.md index remain live).
   137	- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
   138	  rows) + `.review/findings/` + `.review/results/`.
   139	- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
   140	  D-2026-06-12-1, executes w8-1; **capability unparked
   141	  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
   142	  (gated on Round 1),
   143	  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).
   144	
   145	## Blocked / waiting (all owner declarations; checkpoints are owner-only)
   146	
   147	- **Rigs**: owner go standing through otp-12. zoey (12a), netwatch-01
   148	  (12b), netwatch-01↔skippy (12c) done; **magneto↔skippy = the same-OS
   149	  rig** (new 2026-07-13). Rig facts + the macOS ping/MTU trap:
   150	  `.agents/machines.md`.
   151	- **otp-12c RECORDED 2026-07-13** (pre-fix rows = replication/control
   152	  evidence, NOT acceptance evidence; Queue 1a):
   153	  `docs/bench/otp12c-win-2026-07-13/` (198 runs) and
   154	  `otp12c-delegated-2026-07-13/` (**rig D 7/7 PASS**). Codex: FAIL →
   155	  **7/7 accepted**. Detail: DEVLOG 2026-07-13.
   156	- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
   157	  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)
   158	
   159	## Open questions
   160	
   161	- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
   162	  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
   163	- **(SLOTTED 2026-07-12 — owner ack)** `docs/WHITEPAPER.md` §8 (~line
   164	  592) describes the deleted `determine_remote_tuning` — fix folded
   165	  into **w10-docs-batch**; no one-off edit.
   166	- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: w9-3 fixed the
   167	  Linux daemon-spawn flakiness; windows-latest CI pending a push.
   168	  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
   169	  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
   170	  historical) — own finding if it persists on a quiet machine.
   171	- *(Resolved 2026-07-12/13 — SizeMtime data-safe SKIP, the `725aa07`
   172	  snapshot, the CLI `./NAME` foot-gun, otp-5b-3 mid-copy cancel, the
   173	  change-journal premise: all landed; see DEVLOG.)*
   174	
   175	## Handoff log (newest first, keep ≤ 3)
   176	
   177	- **2026-07-13 (46th)** — **otp-12c closed** (rig D 7/7; codex 7/7
   178	  accepted). **Same-OS rig built** (magneto↔skippy): Linux both ends =
   179	  **8/8 PASS**, so P1 is platform-INTERACTING, not pure layout. Perf plan
   180	  → **ACTIVE** (D-2026-07-13-1). Also landed: mid-copy cancel e2e + the
   181	  D4 mid-record fault fix (`920c6a7`), CLI `./NAME` hint (`ace91de`), CI
   182	  fmt fix (`bb28ddd`, suite **1488**).
   183	  **THREE claims of mine were reported and RETRACTED this session** —
   184	  all from trusting an unvalidated instrument: (1) "P1 is code" (1.78),
   185	  from a harness that keyed durability to the *initiator*, not the
   186	  destination (fixed `2c0af86`); (2) "P1 is acceptable platform residue"
   187	  (D-2026-07-12-1 does not cover an invariance failure — codex r5 F1);
   188	  (3) "macOS can't send jumbo / the switch is broken" (it was
   189	  `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — cost the
   190	  owner an adapter swap for nothing). **Verify the instrument before the
   191	  measurement.** In-flight: none; tree clean.
   192	  **Next**: the jumbo re-run (Queue 1a) → pf-1.
   193	- *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*

exec
/bin/zsh -lc 'wc -l docs/plan/OTP12_PERF_FINDINGS.md docs/plan/ONE_TRANSFER_PATH.md docs/DECISIONS.md scripts/bench_otp12_win.sh && rg -n "''^(#|##|###)|UNIFORM|Fix criteria|pf-1|pf-final|Contract|HARD GATE|stop|amend|P1|P2|RUNS=8|1'"\\.10|70%|old_committed|min_old_committed|baseline\" docs/plan/OTP12_PERF_FINDINGS.md docs/plan/ONE_TRANSFER_PATH.md docs/DECISIONS.md scripts/bench_otp12_win.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     646 docs/plan/OTP12_PERF_FINDINGS.md
     318 docs/plan/ONE_TRANSFER_PATH.md
     177 docs/DECISIONS.md
     835 scripts/bench_otp12_win.sh
    1976 total
docs/plan/ONE_TRANSFER_PATH.md:1:# One transfer path — direction-invariant by construction
docs/plan/ONE_TRANSFER_PATH.md:14:## Directive (owner, 2026-07-05, verbatim)
docs/plan/ONE_TRANSFER_PATH.md:35:## Goal
docs/plan/ONE_TRANSFER_PATH.md:47:## Non-goals
docs/plan/ONE_TRANSFER_PATH.md:75:## Constraints
docs/plan/ONE_TRANSFER_PATH.md:109:## Acceptance criteria
docs/plan/ONE_TRANSFER_PATH.md:124:      corrected symmetric-fs harness records a per-cell baseline of
docs/plan/ONE_TRANSFER_PATH.md:132:      `docs/plan/OTP12_ACCEPTANCE_RUN.md` D2.)
docs/plan/ONE_TRANSFER_PATH.md:146:- [ ] Suite green throughout; final test count ≥ pre-plan baseline
docs/plan/ONE_TRANSFER_PATH.md:155:## Design
docs/plan/ONE_TRANSFER_PATH.md:244:baseline parity pins per slice. Wire break — lockstep upgrade,
docs/plan/ONE_TRANSFER_PATH.md:249:## Slices
docs/plan/ONE_TRANSFER_PATH.md:272:2. **otp-2 symmetric baseline (harness + rig, no production code)**:
docs/plan/ONE_TRANSFER_PATH.md:275:   and record the OLD paths' per-cell, per-direction baseline on the
docs/plan/ONE_TRANSFER_PATH.md:311:    against the unified baseline — owner call at that point.
docs/plan/ONE_TRANSFER_PATH.md:313:## Open questions
scripts/bench_otp12_win.sh:1:#!/usr/bin/env bash
scripts/bench_otp12_win.sh:2:# otp-12b: the Mac<->Windows acceptance session (ONE_TRANSFER_PATH slice
scripts/bench_otp12_win.sh:3:# otp-12, sub-slice 12b; design: docs/plan/OTP12_ACCEPTANCE_RUN.md
scripts/bench_otp12_win.sh:4:# D1-D3/D5/D6). Two blocks on the owner-designated closest-spec pair:
scripts/bench_otp12_win.sh:5:#
scripts/bench_otp12_win.sh:6:#   BLOCK 1 — converge-up (Mac-initiated, matches the otp-2w recorded
scripts/bench_otp12_win.sh:7:#   conditions): {large,small,mixed} x {push,pull} x {tcp,grpc} = 12
scripts/bench_otp12_win.sh:8:#   comparisons, matched-pair interleaved A/B — arm "old" = the pinned
scripts/bench_otp12_win.sh:9:#   pre-cutover pair (default 0f922de: Mac client rebuilt in a detached
scripts/bench_otp12_win.sh:10:#   worktree; Windows daemon built natively at that commit), arm "new"
scripts/bench_otp12_win.sh:11:#   = the run commit's pair. Verdicts against BOTH references (the
scripts/bench_otp12_win.sh:12:#   same-session old arm AND docs/bench/otp2w-baseline-2026-07-10/
scripts/bench_otp12_win.sh:13:#   summary.csv), per design D2 as amended.
scripts/bench_otp12_win.sh:14:#
scripts/bench_otp12_win.sh:15:#   BLOCK 2 — initiator/verb invariance (NEW pair only; the owner's
scripts/bench_otp12_win.sh:16:#   sentence, measured): per data direction x fixture x carrier, arm
scripts/bench_otp12_win.sh:17:#   "mac_init" vs arm "win_init" interleaved ABBA. Data Mac->Win (mw_*):
scripts/bench_otp12_win.sh:18:#   Mac client pushes vs Windows client pulls the SAME physical source
scripts/bench_otp12_win.sh:19:#   (the Mac module root IS $MAC_WORK — design F6). Data Win->Mac
scripts/bench_otp12_win.sh:20:#   (wm_*): Mac client pulls vs Windows client pushes the same staged
scripts/bench_otp12_win.sh:21:#   tree on D:. Cell grammar: <mw|wm>_<carrier>_<fixture>. Every arm
scripts/bench_otp12_win.sh:22:#   also gets converge rows against its data direction's old references
scripts/bench_otp12_win.sh:23:#   (design F3: no tolerance compounding), plus the F4 cross-direction
scripts/bench_otp12_win.sh:24:#   rows and the D-2026-07-12-1 discriminator gap rows (recorded, never
scripts/bench_otp12_win.sh:25:#   self-adjudicated).
scripts/bench_otp12_win.sh:26:#
scripts/bench_otp12_win.sh:27:# Methodology inherited verbatim from scripts/bench_otp2w_baseline.sh
scripts/bench_otp12_win.sh:28:# (self-timed durability: Write-VolumeCache on Windows / per-file fsync
scripts/bench_otp12_win.sh:29:# walk on macOS, keyed by DESTINATION OS never verb; Get-Counter drain;
scripts/bench_otp12_win.sh:30:# standby-list purge + macOS purge; WMI daemon launch — Windows OpenSSH
scripts/bench_otp12_win.sh:31:# kills session children; TOML literal-string module paths; stale-daemon
scripts/bench_otp12_win.sh:32:# refusal + PID-scoped teardown) and from bench_otp12_zoey.sh (ABBA
scripts/bench_otp12_win.sh:33:# counterbalance, pair-void valid-run rule with 2xRUNS cap + INCOMPLETE,
scripts/bench_otp12_win.sh:34:# exit codes checked, +sha provenance, sha256 staging manifest,
scripts/bench_otp12_win.sh:35:# PREFLIGHT_ONLY, CELLS allowlist for D2 escalations, per-run
scripts/bench_otp12_win.sh:36:# destination sweep after the measured flush — the zoey I/O-storm
scripts/bench_otp12_win.sh:37:# lesson, kept uniform here).
scripts/bench_otp12_win.sh:38:#
scripts/bench_otp12_win.sh:39:# Windows-side timed windows (win_init arms) are measured ON Windows —
scripts/bench_otp12_win.sh:40:# a Stopwatch brackets the blit.exe invocation inside one ssh call and
scripts/bench_otp12_win.sh:41:# prints "<ms>,<exit>"; the ssh round trip stays outside the window by
scripts/bench_otp12_win.sh:42:# construction (the otp-2w F3 rule applied to a whole client run).
scripts/bench_otp12_win.sh:43:#
scripts/bench_otp12_win.sh:44:# Usage (from the client Mac):
scripts/bench_otp12_win.sh:45:#   export WIN_SSH=michael@10.1.10.173
scripts/bench_otp12_win.sh:46:#   export WIN_HOST=10.1.10.173
scripts/bench_otp12_win.sh:47:#   export WIN_TEST='D:\blit-test'
scripts/bench_otp12_win.sh:48:#   export MAC_HOST=<the Mac's 10GbE IP>      # required, no default
scripts/bench_otp12_win.sh:49:#   RUNS=4 ./scripts/bench_otp12_win.sh
scripts/bench_otp12_win.sh:50:#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
scripts/bench_otp12_win.sh:51:#   CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_win.sh   # escalation
scripts/bench_otp12_win.sh:52:#
scripts/bench_otp12_win.sh:53:# Staging prerequisites (the rig session does these before preflight):
scripts/bench_otp12_win.sh:54:#   * Mac: clean tree at the run commit; `cargo build --release` (client
scripts/bench_otp12_win.sh:55:#     AND daemon — the Mac daemon serves block 2); old client rebuilt at
scripts/bench_otp12_win.sh:56:#     $OLD_SHA in a detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA.
scripts/bench_otp12_win.sh:57:#   * Windows: BEFORE moving the checkout, copy the detached-build exes
scripts/bench_otp12_win.sh:58:#     aside to $WIN_TEST\bins\$OLD_SHA\; then fresh git bundle ->
scripts/bench_otp12_win.sh:59:#     checkout the run commit -> native `cargo build --release` ->
scripts/bench_otp12_win.sh:60:#     copy blit-daemon.exe AND blit.exe to $WIN_TEST\bins\<run sha>\.
scripts/bench_otp12_win.sh:61:#     Daemons always LAUNCH from the fixed path
scripts/bench_otp12_win.sh:62:#     $WIN_TEST\bins\active\blit-daemon.exe (arm swap = Copy-Item over
scripts/bench_otp12_win.sh:63:#     it) so ONE program-scoped firewall rule covers both arms
scripts/bench_otp12_win.sh:64:#     ("blit-otp12-daemon"; the otp-2w rule points at the repo path and
scripts/bench_otp12_win.sh:65:#     is left alone).
scripts/bench_otp12_win.sh:66:#   * Pre-cutover CLIENT binaries embed no build id (otp-12a-run F1):
scripts/bench_otp12_win.sh:67:#     old-client provenance = the clean-worktree rebuild + the manifest,
scripts/bench_otp12_win.sh:68:#     acknowledged via OLD_CLIENT_PROVENANCE_BY_BUILD=1.
scripts/bench_otp12_win.sh:75:# Defaults match the box's 2026-07-12 reality: hostname netwatch-01,
scripts/bench_otp12_win.sh:76:# IP 10.1.10.177 (the previously recorded 10.1.10.173 went stale —
scripts/bench_otp12_win.sh:77:# DHCP; machines.md).
scripts/bench_otp12_win.sh:79:WIN_HOST=${WIN_HOST:-10.1.10.177}
scripts/bench_otp12_win.sh:88:# The Mac module root IS the fixture workdir (design F6): both
scripts/bench_otp12_win.sh:89:# initiators of a Mac->Win cell read the same physical inodes. NOT
scripts/bench_otp12_win.sh:90:# overridable (codex otp-12b F6) — an override could point the two
scripts/bench_otp12_win.sh:91:# initiators at different trees or devices.
scripts/bench_otp12_win.sh:104:# Fixed committed reference (pre-registered, D2) — no override.
scripts/bench_otp12_win.sh:105:BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2w-baseline-2026-07-10/summary.csv"
scripts/bench_otp12_win.sh:120:# --- Self-timed durability (destination-OS-keyed, never verb-keyed) ----
scripts/bench_otp12_win.sh:145:# --- Provenance + manifest (otp-12a lessons: +sha form, fail closed) ---
scripts/bench_otp12_win.sh:170:    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
scripts/bench_otp12_win.sh:198:        die "a blit-daemon is already running on the Windows host — stop it first"
scripts/bench_otp12_win.sh:201:        die "a blit-daemon is already running on the Mac — stop it first"
scripts/bench_otp12_win.sh:236:# --- One-time host setup (idempotent) ----------------------------------
scripts/bench_otp12_win.sh:260:# --- Windows daemon lifecycle (arm-swapped via the fixed active path) ---
scripts/bench_otp12_win.sh:305:win_daemon_stop() {
scripts/bench_otp12_win.sh:322:    [[ -n "$WIN_ARM" ]] && win_daemon_stop
scripts/bench_otp12_win.sh:326:# --- Mac daemon lifecycle (new build only; serves block 2) --------------
scripts/bench_otp12_win.sh:349:mac_daemon_stop() {
scripts/bench_otp12_win.sh:362:    if [[ "$WIN_DAEMON_STARTED" == 1 ]]; then win_daemon_stop; sweep_win_push_dirs; fi
scripts/bench_otp12_win.sh:363:    [[ "$MAC_DAEMON_STARTED" == 1 ]] && mac_daemon_stop
scripts/bench_otp12_win.sh:367:# --- Drain + cold caches -------------------------------------------------
scripts/bench_otp12_win.sh:398:# --- Fixtures (shape-verified; the otp-12a F2 rule) ----------------------
scripts/bench_otp12_win.sh:466:# --- Timed runs -----------------------------------------------------------
scripts/bench_otp12_win.sh:474:# Mac-initiated runs (block 1 both arms; block 2 mac_init arms).
scripts/bench_otp12_win.sh:510:# Windows-initiated runs (block 2 win_init arms): the transfer window is
scripts/bench_otp12_win.sh:511:# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
scripts/bench_otp12_win.sh:551:# One interleaved comparison; ABBA; pair-void; INCOMPLETE at the cap.
scripts/bench_otp12_win.sh:552:# run_one <cell> <armA> <armB> <fnA...>|<fnB...> dispatch happens via
scripts/bench_otp12_win.sh:553:# small wrappers below to keep bash 3.2-simple.
scripts/bench_otp12_win.sh:597:# Block-1 arm wrappers (Mac-initiated; daemon arm follows the run arm).
scripts/bench_otp12_win.sh:603:# Block-2 arm wrappers (new pair; both daemons stay up). Both arms of a
scripts/bench_otp12_win.sh:604:# pair do IDENTICAL work (codex otp-12b F5): no-trailing-slash sources
scripts/bench_otp12_win.sh:605:# everywhere AND a destination CONTAINER precreated OUTSIDE the timed
scripts/bench_otp12_win.sh:606:# window on every arm — each transfer lands the same one-level-nested
scripts/bench_otp12_win.sh:607:# `container/src_<w>` tree, and no arm pays an in-window container
scripts/bench_otp12_win.sh:608:# mkdir the other does not. (Block 1 keeps the otp-2w shapes verbatim
scripts/bench_otp12_win.sh:609:# for baseline comparability.)
scripts/bench_otp12_win.sh:641:# --- Verdicts (design D2 as amended; F3; F4 + discriminator recorded) ----
scripts/bench_otp12_win.sh:668:def bar(new, ref):   # new <= ref * 1.10, integer-exact
scripts/bench_otp12_win.sh:687:# Block 1: converge-up, both references (12a logic verbatim).
scripts/bench_otp12_win.sh:691:        out.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
scripts/bench_otp12_win.sh:698:    out.write(f"{cell},converge,new,old_session,{new_m},{old_m},{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
scripts/bench_otp12_win.sh:699:    out.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
scripts/bench_otp12_win.sh:702:    out.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
scripts/bench_otp12_win.sh:704:# Block 2: invariance + per-arm converge (F3) + cross rows (F4) +
scripts/bench_otp12_win.sh:705:# discriminator gap rows (D-2026-07-12-1; recorded, not adjudicated).
scripts/bench_otp12_win.sh:709:        out.write(f"{cell},invariance,mac_init,win_init,,,,1.10,INCOMPLETE\n")
scripts/bench_otp12_win.sh:713:    inv = bar(hi, lo)   # max/min <= 1.10
scripts/bench_otp12_win.sh:714:    out.write(f"{cell},invariance,mac_init,win_init,{a},{b},{hi/lo:.3f},1.10,{'PASS' if inv else 'FAIL'}\n")
scripts/bench_otp12_win.sh:729:            out.write(f"{cell},converge,{armname},old_session,{val},{old_sess},{val/old_sess:.3f},1.10,{'PASS' if bar(val, old_sess) else 'FAIL'}\n")
scripts/bench_otp12_win.sh:731:            out.write(f"{cell},converge,{armname},old_session,{val},,,1.10,NO-SAME-SESSION-REF\n")
scripts/bench_otp12_win.sh:732:        out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
scripts/bench_otp12_win.sh:741:    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
scripts/bench_otp12_win.sh:743:# Discriminator gap rows (D-2026-07-12-1; recorded, never adjudicated):
scripts/bench_otp12_win.sh:744:# emitted only when ALL FOUR contributing cells are complete (codex
scripts/bench_otp12_win.sh:745:# otp-12b F4). Row operands are labeled exactly (codex otp-12b F12).
scripts/bench_otp12_win.sh:763:# --- Matrix ----------------------------------------------------------------
scripts/bench_otp12_win.sh:819:    win_daemon_stop
scripts/bench_otp12_win.sh:820:    mac_daemon_stop
docs/DECISIONS.md:1:# DECISIONS — settled choices
docs/DECISIONS.md:13:## D-<YYYY-MM-DD>-<n> — <short title>
docs/DECISIONS.md:21:## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
docs/DECISIONS.md:26:## D-2026-05-31-2 — Pick-not-Type TUI direction
docs/DECISIONS.md:31:## D-2026-06-04-1 — R3 overrides R2 in the audit chain
docs/DECISIONS.md:36:## D-2026-06-04-2 — Env vars are out for app + diagnostic config
docs/DECISIONS.md:41:## D-2026-06-04-3 — Streaming planner ratified, build deferred
docs/DECISIONS.md:46:## D-2026-06-06-1 — STATE.md precedence model adopted
docs/DECISIONS.md:51:## D-2026-06-07-1 — Keep the `c793df2` octopus on master; no history rewrite
docs/DECISIONS.md:57:## D-2026-06-07-2 — Adaptive-streams lands via cherry-pick/rebase, excluding the WIP
docs/DECISIONS.md:62:## D-2026-06-11-1 — Design-coherence review plan Active; ratification covers Phase A only
docs/DECISIONS.md:67:## D-2026-06-11-2 — Design-review queue ratified in full; Pull-RPC delete; zero_copy gets a FAST evaluation
docs/DECISIONS.md:72:## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
docs/DECISIONS.md:77:## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
docs/DECISIONS.md:83:## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
docs/DECISIONS.md:89:## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
docs/DECISIONS.md:94:## D-2026-06-20-4 — Unified transfer engine plan review freeze
docs/DECISIONS.md:99:## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
docs/DECISIONS.md:104:## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
docs/DECISIONS.md:109:## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
docs/DECISIONS.md:114:## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
docs/DECISIONS.md:115:- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
docs/DECISIONS.md:119:## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
docs/DECISIONS.md:120:- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
docs/DECISIONS.md:124:## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
docs/DECISIONS.md:129:## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
docs/DECISIONS.md:130:- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
docs/DECISIONS.md:134:## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
docs/DECISIONS.md:139:## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
docs/DECISIONS.md:142:- Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.
docs/DECISIONS.md:144:## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
docs/DECISIONS.md:145:- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
docs/DECISIONS.md:149:## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
docs/DECISIONS.md:154:## D-2026-07-10-1 — Resume wire bounds on the in-stream carrier (amends OTP7_RESUME D5)
docs/DECISIONS.md:157:- Supersedes: OTP7_RESUME.md D5's "clamped to `MAX_BLOCK_SIZE`" wording (amended in place, same commit).
docs/DECISIONS.md:159:## D-2026-07-10-2 — Resume block-size ceiling is per carrier (completes the D-2026-07-10-1 revisit)
docs/DECISIONS.md:162:- Supersedes: nothing — completes the revisit D-2026-07-10-1 explicitly deferred to otp-7b (OTP7_RESUME.md D5 amended in place, same commit).
docs/DECISIONS.md:164:## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
docs/DECISIONS.md:169:## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
docs/DECISIONS.md:170:- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
docs/DECISIONS.md:172:- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
docs/DECISIONS.md:174:## D-2026-07-13-1 — OTP12_PERF_FINDINGS goes Active after one final codex round; implementation proceeds slice-by-slice
docs/DECISIONS.md:175:- Decision: `docs/plan/OTP12_PERF_FINDINGS.md` flips **Draft → Active** after ONE final codex round, and implementation then proceeds regardless of whether that round returns a "converged" verdict — owner, 2026-07-13, verbatim: **"one more round with codex on the plan then just write the code and reviewloop slice by slice. that converges faster than plans with no ground truth to test."** Each code slice still goes through the codex review loop (D-2026-07-04-1, unchanged); what is retired is *plan-only* iteration as the gate on starting work. The plan's own Status line ("the flip to Active happens at codex convergence") is amended by this decision: the round happens, its accepted findings are fixed, and then code starts — a non-converged verdict is no longer a blocker, it is input to the first slice.
docs/DECISIONS.md:176:- Why: rounds 2–4 each returned real findings, but they were increasingly findings about the *plan text* (falsifiability wording, thresholds, bar phrasing) rather than about reality, and the plan's central factual claim was settled not by review but by *measurement* — the same-OS rig, which refuted a claim four review rounds had left standing (`docs/bench/otp12-perf-2026-07-13/`; a wrong "P1 is code" claim was reported and retracted the same day). Ground truth comes from instrumented code and rigs, not from more prose; pf-1 exists precisely to generate it. Continuing to polish the plan has diminishing returns against the cost of not yet having a single measured counterfactual.
docs/DECISIONS.md:177:- Supersedes: the "flip to Active at codex convergence" gate in `OTP12_PERF_FINDINGS.md`'s Status line (rewritten in place, same commit). Does NOT supersede D-2026-07-04-1 — every code slice is still codex-reviewed before the next begins.
docs/plan/OTP12_PERF_FINDINGS.md:1:# otp-12 perf findings — investigate + fix before acceptance (design)
docs/plan/OTP12_PERF_FINDINGS.md:8:3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
docs/plan/OTP12_PERF_FINDINGS.md:16:claim four review rounds had left standing). pf-1 exists to generate
docs/plan/OTP12_PERF_FINDINGS.md:19:**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
docs/plan/OTP12_PERF_FINDINGS.md:20:assumed by this plan):** P1 has **no escape hatch on the books**.
docs/plan/OTP12_PERF_FINDINGS.md:22:that is *already* invariance-passing; P1 is the invariance failure
docs/plan/OTP12_PERF_FINDINGS.md:23:itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
docs/plan/OTP12_PERF_FINDINGS.md:24:plan pursues) or the owner must **amend acceptance criterion 1** in a new
docs/plan/OTP12_PERF_FINDINGS.md:25:decision. pf-1 proceeds either way: it produces the evidence that
docs/plan/OTP12_PERF_FINDINGS.md:30:review." P1 is a miss of the parent's HEADLINE acceptance criterion
docs/plan/OTP12_PERF_FINDINGS.md:32:**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
docs/plan/OTP12_PERF_FINDINGS.md:33:if an investigation slice needs one, it stops and this doc is amended
docs/plan/OTP12_PERF_FINDINGS.md:38:fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
docs/plan/OTP12_PERF_FINDINGS.md:41:work — under `pf-final` they are **pre-fix rows, void for acceptance**,
docs/plan/OTP12_PERF_FINDINGS.md:44:independent corroboration the round-2 review said P1 lacked; and (b) the
docs/plan/OTP12_PERF_FINDINGS.md:45:pre-pf-1 control the investigation needs. **otp-12d and otp-13 remain
docs/plan/OTP12_PERF_FINDINGS.md:46:deferred** until P1/P2 are fixed or explained at code level — assembling
docs/plan/OTP12_PERF_FINDINGS.md:50:## The two findings (evidence, both committed)
docs/plan/OTP12_PERF_FINDINGS.md:52:**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
docs/plan/OTP12_PERF_FINDINGS.md:105:### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)
docs/plan/OTP12_PERF_FINDINGS.md:112:**P1 does NOT reproduce.** Its own cell passes with room to spare:
docs/plan/OTP12_PERF_FINDINGS.md:116:| `sm_tcp_mixed` (P1's cell) | 1745 | 1905 | **1.092** | PASS |
docs/plan/OTP12_PERF_FINDINGS.md:117:| `ms_tcp_mixed` (P1's cell) | 2085 | 2079 | **1.003** | PASS |
docs/plan/OTP12_PERF_FINDINGS.md:120:RUNS=8 escalation → 1.063). There is no destination-initiator penalty at
docs/plan/OTP12_PERF_FINDINGS.md:125:- **P1 requires the Mac↔Windows pairing.** It is NOT a pure layout
docs/plan/OTP12_PERF_FINDINGS.md:129:- **⚠ BUT P1 HAS NO ESCAPE HATCH TODAY (review round 5, BLOCKER).** An
docs/plan/OTP12_PERF_FINDINGS.md:131:  accept P1 as a platform residue. **It does not.** That decision excuses
docs/plan/OTP12_PERF_FINDINGS.md:134:  ±10%"** (`docs/DECISIONS.md` D-2026-07-12-1). **P1 IS the invariance
docs/plan/OTP12_PERF_FINDINGS.md:137:  Therefore exactly two exits exist, and pf-1 must aim at them:
docs/plan/OTP12_PERF_FINDINGS.md:138:  1. **FIX IT** — P1 ≤ 1.10 on rig W. This remains the default and the
docs/plan/OTP12_PERF_FINDINGS.md:140:  2. **A NEW OWNER DECISION amending criterion 1** — for which the
docs/plan/OTP12_PERF_FINDINGS.md:155:- **P2 is untested by this rig** (it is a converge bar vs the OLD build,
docs/plan/OTP12_PERF_FINDINGS.md:160:> asserted the opposite — "P1 reproduces at 1.78 → the confound breaks
docs/plan/OTP12_PERF_FINDINGS.md:162:> the acceptance plan were amended to match. That was **WRONG**. It rested
docs/plan/OTP12_PERF_FINDINGS.md:177:### The residual confound (WHICH code) still needs a counterfactual
docs/plan/OTP12_PERF_FINDINGS.md:183:- **pf-1 must compare all four rig-W arms** (both cells × both
docs/plan/OTP12_PERF_FINDINGS.md:192:  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
docs/plan/OTP12_PERF_FINDINGS.md:207:  (2) do — **trips this plan's Contract stop-and-amend rule**
docs/plan/OTP12_PERF_FINDINGS.md:208:  (`TRANSFER_SESSION.md` amended through the loop BEFORE the flag is
docs/plan/OTP12_PERF_FINDINGS.md:212:  did not appear on a real-network same-OS pair), so pf-1 must be prepared
docs/plan/OTP12_PERF_FINDINGS.md:226:is exactly why pf-1 exists.
docs/plan/OTP12_PERF_FINDINGS.md:238:new-vs-old check, not a two-layout measurement. P1 was never measured
docs/plan/OTP12_PERF_FINDINGS.md:241:**P2 — unified small-file push pays ~10–20% vs old push, both rigs**,
docs/plan/OTP12_PERF_FINDINGS.md:246:| 12a zoey (RUNS=8, tight) | `e757dcc` old arm | — | — | **1.105** |
docs/plan/OTP12_PERF_FINDINGS.md:266:whatever P2 is, it is TCP-data-plane-specific, source-initiated, and
docs/plan/OTP12_PERF_FINDINGS.md:271:therefore NOT exonerated by the gRPC numbers, and pf-1 must attribute
docs/plan/OTP12_PERF_FINDINGS.md:281:## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
docs/plan/OTP12_PERF_FINDINGS.md:283:- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
docs/plan/OTP12_PERF_FINDINGS.md:300:  (`transfer_session/mod.rs:3113`) is an equally good suspect. pf-1 must
docs/plan/OTP12_PERF_FINDINGS.md:303:- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
docs/plan/OTP12_PERF_FINDINGS.md:309:  pf-1 timing shows a layout-dependent need-batch delta anyway, the
docs/plan/OTP12_PERF_FINDINGS.md:311:- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
docs/plan/OTP12_PERF_FINDINGS.md:321:  by pf-1 is not a hypothesis; keeping it would let pf-1 close with a
docs/plan/OTP12_PERF_FINDINGS.md:325:  H3 may only be revived if the pf-1 trace names a concrete old/new
docs/plan/OTP12_PERF_FINDINGS.md:329:- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
docs/plan/OTP12_PERF_FINDINGS.md:336:- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
docs/plan/OTP12_PERF_FINDINGS.md:347:  ordering), so any H5 fix triggers this plan's Contract
docs/plan/OTP12_PERF_FINDINGS.md:348:  stop-and-amend rule BEFORE implementation.
docs/plan/OTP12_PERF_FINDINGS.md:349:- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
docs/plan/OTP12_PERF_FINDINGS.md:354:  TCP-only and per-member (so small-file-heavy) — matches the P2
docs/plan/OTP12_PERF_FINDINGS.md:355:  signature independently of H5. Discriminated by the pf-1 per-member
docs/plan/OTP12_PERF_FINDINGS.md:368:  counterpart took none. pf-1 confirms it by (a) reading the executed
docs/plan/OTP12_PERF_FINDINGS.md:371:  account for a material share of the P2 gap. If H6 is confirmed, the P2
docs/plan/OTP12_PERF_FINDINGS.md:372:  fix bar applies unchanged (≤ 1.10 against BOTH references, BOTH rigs);
docs/plan/OTP12_PERF_FINDINGS.md:375:  strand pf-1 under the uniform decision rule):** behind a debug flag,
docs/plan/OTP12_PERF_FINDINGS.md:381:  does NOT trip the Contract rule. Grade its recovery against `Δ_P2` on
docs/plan/OTP12_PERF_FINDINGS.md:385:- **H7 (P2; added by review round 4 — the SHARED-controller candidate
docs/plan/OTP12_PERF_FINDINGS.md:393:  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
docs/plan/OTP12_PERF_FINDINGS.md:402:## Method (the investigation slice — no behavior changes)
docs/plan/OTP12_PERF_FINDINGS.md:417:   that H1 stays falsifiable: if the local run is negative, pf-1
docs/plan/OTP12_PERF_FINDINGS.md:419:   CELLS fixtures) before pf-1 may close — every hypothesis exits
docs/plan/OTP12_PERF_FINDINGS.md:420:   pf-1 confirmed or killed, never "unfalsified" (review round 2).
docs/plan/OTP12_PERF_FINDINGS.md:421:3. **Historical control, then bisect P2**: old push is deleted from
docs/plan/OTP12_PERF_FINDINGS.md:462:   without a wire change, this plan's Contract stop-and-amend rule fires
docs/plan/OTP12_PERF_FINDINGS.md:465:   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
docs/plan/OTP12_PERF_FINDINGS.md:468:   P2 to H6;
docs/plan/OTP12_PERF_FINDINGS.md:469:   (e) per-member locking/framing timings are now an unconditional pf-1
docs/plan/OTP12_PERF_FINDINGS.md:472:4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
docs/plan/OTP12_PERF_FINDINGS.md:473:   local rig is Mac↔Mac loopback: it removes the very platform terms P1
docs/plan/OTP12_PERF_FINDINGS.md:474:   is confounded with, and it may equally fail to surface P2 (whose
docs/plan/OTP12_PERF_FINDINGS.md:476:   finding does not reproduce locally, pf-1 REQUIRES the rig-side
docs/plan/OTP12_PERF_FINDINGS.md:477:   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
docs/plan/OTP12_PERF_FINDINGS.md:478:   since P2 was measured on both) with the same spans and the CELLS
docs/plan/OTP12_PERF_FINDINGS.md:479:   fixtures, before pf-1 may close. Every hypothesis exits pf-1
docs/plan/OTP12_PERF_FINDINGS.md:485:## pf-1 decision rule — UNIFORM, pre-registered (added round 5)
docs/plan/OTP12_PERF_FINDINGS.md:497:  carried as UNTESTED and pf-1 does not close.
docs/plan/OTP12_PERF_FINDINGS.md:499:  (review round 5: the earlier text left it ambiguous between P1's
docs/plan/OTP12_PERF_FINDINGS.md:500:  layout gap and P2's old/new gap, which are different quantities):
docs/plan/OTP12_PERF_FINDINGS.md:501:  - **`Δ_P1(rig)`** = `destinit_median − srcinit_median` for
docs/plan/OTP12_PERF_FINDINGS.md:504:    magneto↔skippy it is ~0 (8/8 pass) — so **P1 counterfactuals are
docs/plan/OTP12_PERF_FINDINGS.md:507:  - **`Δ_P2(rig)`** = `new_median − old_same_session_median` for
docs/plan/OTP12_PERF_FINDINGS.md:520:  ≥70% closure test below is evaluated on the **cumulative combined**
docs/plan/OTP12_PERF_FINDINGS.md:528:- **pf-1 closes only when the confirmed contributions account for ≥ 70%
docs/plan/OTP12_PERF_FINDINGS.md:530:  and pf-1 **stays open** with the shortfall stated in the probe record —
docs/plan/OTP12_PERF_FINDINGS.md:535:## Fix criteria (pre-registered; the owner walks the final numbers)
docs/plan/OTP12_PERF_FINDINGS.md:541:  Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in
docs/plan/OTP12_PERF_FINDINGS.md:543:  same-session reference AND the committed baseline — no arm may exceed
docs/plan/OTP12_PERF_FINDINGS.md:544:  1.10 against either reference even when its counterpart bar passes
docs/plan/OTP12_PERF_FINDINGS.md:545:  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
docs/plan/OTP12_PERF_FINDINGS.md:546:  P2 bars below but regresses any other cell against either reference is
docs/plan/OTP12_PERF_FINDINGS.md:548:- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
docs/plan/OTP12_PERF_FINDINGS.md:549:  `pull_tcp_mixed` ≤ 1.10 against BOTH references on the netwatch-01
docs/plan/OTP12_PERF_FINDINGS.md:550:  rig (CELLS escalation session, RUNS=8), with `wm_grpc_mixed` and the
docs/plan/OTP12_PERF_FINDINGS.md:553:- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
docs/plan/OTP12_PERF_FINDINGS.md:557:  cell must stay ≤ 1.10 against both of its own references AND must not
docs/plan/OTP12_PERF_FINDINGS.md:562:  bare ≤1.10 bar while having eaten a real, measured win.
docs/plan/OTP12_PERF_FINDINGS.md:577:## Staging (each through the codex loop)
docs/plan/OTP12_PERF_FINDINGS.md:579:- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
docs/plan/OTP12_PERF_FINDINGS.md:583:  pre-pf-1 evidence.
docs/plan/OTP12_PERF_FINDINGS.md:586:- **pf-final**: NOT just the two escalation cells — the final build
docs/plan/OTP12_PERF_FINDINGS.md:592:  same-session `old` arms and the committed baselines are OLD builds by
docs/plan/OTP12_PERF_FINDINGS.md:600:  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
docs/plan/OTP12_PERF_FINDINGS.md:601:  evidence at all. "Not implicated" scopes what pf-1 must
docs/plan/OTP12_PERF_FINDINGS.md:605:  baseline), so the whole cell is re-measured.
docs/plan/OTP12_PERF_FINDINGS.md:611:  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
docs/plan/OTP12_PERF_FINDINGS.md:613:  final-build rows, full stop — no conditional. Results land in fresh
docs/plan/OTP12_PERF_FINDINGS.md:617:## Known gaps
docs/plan/OTP12_PERF_FINDINGS.md:623:  controls in pf-1, not pin-archaeology.
docs/plan/OTP12_PERF_FINDINGS.md:624:- zoey never measured P1: its rig anchors converge-up only, so there
docs/plan/OTP12_PERF_FINDINGS.md:626:  says nothing about layout asymmetry (review 2026-07-12). pf-1's
docs/plan/OTP12_PERF_FINDINGS.md:627:  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
docs/plan/OTP12_PERF_FINDINGS.md:630:  They are pre-fix by definition, so `pf-final` voids them for
docs/plan/OTP12_PERF_FINDINGS.md:631:  acceptance; their value is that they (a) reproduce P1 and P2 on an
docs/plan/OTP12_PERF_FINDINGS.md:635:  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
docs/plan/OTP12_PERF_FINDINGS.md:636:  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
docs/plan/OTP12_PERF_FINDINGS.md:637:  P2 1.149→1.201), so neither is drifting toward the bar on its own.
docs/plan/OTP12_PERF_FINDINGS.md:642:  measurable cost and pf-1 need not instrument the delegated trigger
docs/plan/OTP12_PERF_FINDINGS.md:645:  is pre-fix and therefore void under pf-final. Rig D reruns on the
docs/plan/OTP12_PERF_FINDINGS.md:646:  final build (see pf-final).

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '1,280p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp-12 perf findings — investigate + fix before acceptance (design)
     2	
     3	**Status**: Active
     4	**Approved**: D-2026-07-13-1 — owner, 2026-07-13, verbatim:
     5	**"one more round with codex on the plan then just write the code and
     6	reviewloop slice by slice. that converges faster than plans with no
     7	ground truth to test."** The final round ran (round 5, verdict NOT READY,
     8	3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
     9	counterfactual, F3 the inexecutable decision rule); all three are fixed
    10	in this revision, and implementation now proceeds **slice by slice, each
    11	through the codex loop** (D-2026-07-04-1 unchanged). A non-converged plan
    12	verdict is no longer a gate — the plan's earlier "flip to Active at codex
    13	convergence" rule is superseded by D-2026-07-13-1, because rounds 2–5
    14	were increasingly finding defects in the *prose* while the plan's central
    15	factual claim was settled by *measurement* (the same-OS rig refuted a
    16	claim four review rounds had left standing). pf-1 exists to generate
    17	ground truth; it starts now.
    18	
    19	**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
    20	assumed by this plan):** P1 has **no escape hatch on the books**.
    21	D-2026-07-12-1 waives a cross-direction converge-up miss only for a cell
    22	that is *already* invariance-passing; P1 is the invariance failure
    23	itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
    24	plan pursues) or the owner must **amend acceptance criterion 1** in a new
    25	decision. pf-1 proceeds either way: it produces the evidence that
    26	decision would rest on.
    27	**Created**: 2026-07-12
    28	**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
    29	say the quiet part: "Unification that slows the fast direction fails
    30	review." P1 is a miss of the parent's HEADLINE acceptance criterion
    31	(initiator/verb invariance, ±10%) — not a nice-to-have.
    32	**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
    33	if an investigation slice needs one, it stops and this doc is amended
    34	through the loop first.
    35	
    36	**Sequencing (corrected 2026-07-13).** This doc originally deferred
    37	otp-12c/12d/13 outright. In fact **otp-12c RAN on 2026-07-13** under a
    38	fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
    39	at the cutover sha `f35702a`; `docs/bench/otp12c-{delegated,win}-2026-07-13/`).
    40	That does not change this plan's standing, and the rows are not lost
    41	work — under `pf-final` they are **pre-fix rows, void for acceptance**,
    42	but they serve two real purposes: (a) an **independent replication** of
    43	both findings at the shipped sha (below), which is exactly the
    44	independent corroboration the round-2 review said P1 lacked; and (b) the
    45	pre-pf-1 control the investigation needs. **otp-12d and otp-13 remain
    46	deferred** until P1/P2 are fixed or explained at code level — assembling
    47	an acceptance matrix out of pre-fix rows would build the artifact otp-13
    48	walks from rows this plan declares void.
    49	
    50	## The two findings (evidence, both committed)
    51	
    52	**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
    53	(`docs/bench/otp12-win-2026-07-12/`, replicated in
    54	`docs/bench/otp12c-win-2026-07-13/`). `wm_tcp_mixed` invariance FAILs in
    55	**two independent sessions**, and got WORSE at the shipped sha:
    56	
    57	| session | build | mac_init | win_init | ratio | arm spreads |
    58	|---|---|---|---|---|---|
    59	| 12b (2026-07-12) | `e21cf84` | 1127 | 911 | **1.237** | 8.2 / 3.3% |
    60	| 12c-win (2026-07-13) | `f35702a` (cutover) | 1221 | 939 | **1.300** | 6.4 / 8.4% |
    61	
    62	Corroborated by block-1 `pull_tcp_mixed` new-vs-old-same-session:
    63	**1.313** (12b: 1138/867) and **1.247** (12c-win: 1192/956).
    64	
    65	**This cannot be re-run away.** Both sessions' arm spreads are far below
    66	D2's 25% escalation trigger, so no escalation session is even available;
    67	the cells stand as measured. (The 12c-win session was a fresh staging on
    68	a different day at a different sha — the round-2 review's objection that
    69	the 1.313 corroboration was "same rig/session, not independent" is now
    70	answered by an independent session reproducing the same cell.)
    71	
    72	**What the evidence actually supports — and the confound it does NOT
    73	escape** (corrected, review round 3; an earlier draft of this section
    74	claimed the `mw` cell was a clean control isolating "destination
    75	initiation" as the cause. It is not, and the correction matters because
    76	it re-aims the hypotheses):
    77	
    78	Every invariance cell compares two arms that share the same endpoints
    79	and the same data direction, so **within** a cell the initiator is the
    80	only variable — that part is clean. Arm medians (12c-win):
    81	
    82	| cell | data direction | dest-initiated arm | source-initiated arm | ratio | spreads |
    83	|---|---|---|---|---|---|
    84	| `wm_tcp_mixed` | Win→Mac | 1221 | 939 | **1.300 FAIL** | 6.4 / 8.4% |
    85	| `mw_tcp_mixed` | Mac→Win | 1477 | 1415 | 1.044 PASS | 20.8 / 20.5% |
    86	
    87	The initiator penalty is therefore **real and large in the Win→Mac
    88	direction only**. In Mac→Win the two layouts are within noise, and the
    89	ordering even **flips between sessions** (12b: dest-initiated 1502 was
    90	*faster* than source-initiated 1587), on spreads of 17–25%.
    91	
    92	Crossing from `wm` to `mw` is **not** a controlled swap of one variable:
    93	it also swaps the destination filesystem (APFS vs NTFS), the TCP stack,
    94	which host runs the client, and the flush method. So the supported
    95	signature is an **interaction — TCP × mixed × Win→Mac × initiator** —
    96	not "destination initiation" on its own.
    97	
    98	Worse, on a two-host rig the failing configuration is **confounded by
    99	construction**: in the slow arm the destination is the Mac (which dials)
   100	*and* the source is Windows (which accepts). With only two hosts, **host
   101	identity IS role** — "Mac-as-dialing-destination" and
   102	"Windows-as-accepting-source" are the same configuration and cannot be
   103	separated by any number of additional runs on this rig.
   104	
   105	### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)
   106	
   107	**Evidence: `docs/bench/otp12-perf-2026-07-13/` — magneto↔skippy, Linux on
   108	BOTH ends, real 10 GbE, full otp-12 methodology** (cold caches both ends,
   109	destination drained, ABBA, pair-void, RUNS=4; 64 runs, 8/8 cells, zero
   110	voided). Harness `scripts/bench_otp12pf_linux.sh`.
   111	
   112	**P1 does NOT reproduce.** Its own cell passes with room to spare:
   113	
   114	| cell | srcinit | destinit | ratio | outcome |
   115	|---|---|---|---|---|
   116	| `sm_tcp_mixed` (P1's cell) | 1745 | 1905 | **1.092** | PASS |
   117	| `ms_tcp_mixed` (P1's cell) | 2085 | 2079 | **1.003** | PASS |
   118	
   119	**8/8 invariance cells PASS** (`ms_grpc_mixed` via its pre-registered
   120	RUNS=8 escalation → 1.063). There is no destination-initiator penalty at
   121	all when both ends are Linux.
   122	
   123	Therefore:
   124	
   125	- **P1 requires the Mac↔Windows pairing.** It is NOT a pure layout
   126	  property of blit's code — a pure layout cost would have appeared here,
   127	  on the same code, same carrier, same fixture.
   128	
   129	- **⚠ BUT P1 HAS NO ESCAPE HATCH TODAY (review round 5, BLOCKER).** An
   130	  earlier revision of this section said D-2026-07-12-1 lets the owner
   131	  accept P1 as a platform residue. **It does not.** That decision excuses
   132	  a **cross-direction converge-up** miss for a cell that has ALREADY
   133	  satisfied its precondition **"(b) is initiator/verb-invariant within
   134	  ±10%"** (`docs/DECISIONS.md` D-2026-07-12-1). **P1 IS the invariance
   135	  failure** (`wm_tcp_mixed` 1.300 FAIL) — the precondition it would need
   136	  is the very thing it violates. No decision on the books waives it.
   137	  Therefore exactly two exits exist, and pf-1 must aim at them:
   138	  1. **FIX IT** — P1 ≤ 1.10 on rig W. This remains the default and the
   139	     bar (`ONE_TRANSFER_PATH.md` acceptance criterion 1 is mandatory).
   140	  2. **A NEW OWNER DECISION amending criterion 1** — for which the
   141	     same-OS result is the honest evidence base: criterion 1 asks for
   142	     invariance "on a symmetric rig", Mac↔Windows was designated only
   143	     because no better pair existed, and one now does — magneto↔skippy,
   144	     where blit measures **8/8 invariant**. An owner could reasonably
   145	     rule that criterion 1 is judged on the rig that isolates blit's own
   146	     behaviour, with the Mac↔Windows delta recorded as platform residue.
   147	     **That ruling does not exist. It must not be assumed, and this plan
   148	     must not be written as though it will be granted.**
   149	- **This does NOT fully exonerate the code.** It rules out a pure layout
   150	  property; it does not rule out a code path whose cost only becomes
   151	  material under a particular platform — e.g. a slow accept branch on the
   152	  Windows side, which is exactly what H1 accuses. H1/H5/H6 stay LIVE but
   153	  are now **narrowed to platform-interacting mechanisms**, and only the
   154	  dial/accept inversion counterfactual on rig W can finish the job.
   155	- **P2 is untested by this rig** (it is a converge bar vs the OLD build,
   156	  and no `0f922de` build is staged on these hosts). Nothing here speaks
   157	  to it.
   158	
   159	> **⚠ A RETRACTED CLAIM LIVED HERE.** An earlier revision of this section
   160	> asserted the opposite — "P1 reproduces at 1.78 → the confound breaks
   161	> toward CODE → the fix is mandatory and cannot be waived" — and STATE and
   162	> the acceptance plan were amended to match. That was **WRONG**. It rested
   163	> on a scratch probe (and a first harness revision) that ran the durability
   164	> `sync` inside the INITIATING host's timed bracket: in the push arm the
   165	> initiator is the SOURCE, which only read, so its sync was a no-op and the
   166	> destination's writeback was never paid; in the pull arm the initiator IS
   167	> the destination, so it paid the full writeback. One arm was charged for
   168	> durability the other got free — multi-second on skippy's ZFS — which
   169	> manufactured "failures" on every carrier and fixture, **including the
   170	> gRPC control that is supposed to be clean**. That carrier-independence is
   171	> what exposed it: a real code effect is carrier-specific; an accounting
   172	> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
   173	> never by verb — the otp-2w rule, re-learned). The retraction is recorded
   174	> rather than quietly overwritten because the wrong number was reported to
   175	> the owner and briefly drove this plan.
   176	
   177	### The residual confound (WHICH code) still needs a counterfactual
   178	
   179	Breaking platform-vs-code does NOT tell us *which* layout property costs
   180	the time. On any two-host rig, host identity remains welded to role, so
   181	"the accepting end" cannot be separated from "that host" by more runs:
   182	
   183	- **pf-1 must compare all four rig-W arms** (both cells × both
   184	  initiators), not two, and report the interaction — not a single ratio.
   185	- **The disambiguator is a dial/accept inversion counterfactual, not a
   186	  rig** — but it is **NOT sufficient on its own** (review round 5): the
   187	  inversion swaps the source's `Accept`, the destination's `Dial`, AND
   188	  the epoch-0 topology **simultaneously**, so a positive result implicates
   189	  *the topology pair*, not H1 specifically. It cannot distinguish
   190	  source-accept serialization from synchronous destination dialing
   191	  (`transfer_session/mod.rs:3113`), nor prove the resize-specific claim.
   192	  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
   193	  1. **dial/accept inversion** — same direction, same hosts, same fixture;
   194	     only who dials changes. Implicates the topology pair (or exonerates it).
   195	  2. **no-resize / pre-opened streams** — force the final stream count at
   196	     epoch 0 so no resize epoch ever fires. If the gap survives with zero
   197	     resizes, H1's resize-specific mechanism is **KILLED** regardless of
   198	     what (1) shows (and note `dial.rs:474`: all three fixtures already
   199	     target 8 streams, so resize *count* was never the discriminator).
   200	  3. **per-side ordering** — hold the topology fixed and vary only whether
   201	     the destination's dial-before-ACK is synchronous. Separates the two
   202	     halves the inversion conflates.
   203	  H1 is CONFIRMED only if the wall-time recovery tracks the **accept role**
   204	  across (1) AND survives (2); it is KILLED if the gap persists with no
   205	  resizes, or if (3) shows the cost is the synchronous dial rather than the
   206	  accept branch. Any of these that changes connection topology — (1) and
   207	  (2) do — **trips this plan's Contract stop-and-amend rule**
   208	  (`TRANSFER_SESSION.md` amended through the loop BEFORE the flag is
   209	  written). Same-build-both-ends (D-2026-07-05-2) means no compatibility
   210	  surface is created.
   211	  **H1 is also WEAKENED by the Linux null** (it predicts a layout cost that
   212	  did not appear on a real-network same-OS pair), so pf-1 must be prepared
   213	  to kill it and fall through to H5/H6/H7.
   214	- **The same-platform loopback run is a ONE-WAY test** (corrected — an
   215	  earlier draft of this section had it backwards). A dest-initiator
   216	  penalty that still appears on Mac↔Mac loopback proves **pure layout**
   217	  (code). Its ABSENCE proves **nothing**: loopback has no NIC, near-zero
   218	  RTT and a huge MTU, so it erases exactly the per-epoch accept/dial
   219	  round-trip cost H1 accuses. A negative local result is **INCONCLUSIVE**
   220	  and never reads as "no code bug" — it escalates to the inversion
   221	  counterfactual and the rig-side instrumented run (Method 2).
   222	
   223	This refines rather than weakens H1: H1 accuses the **source's accept
   224	branch** under resize, and the source in the slow arm is Windows —
   225	consistent. But consistency is not confirmation, and the confound above
   226	is exactly why pf-1 exists.
   227	
   228	The rest of the signature is unchanged and sharp:
   229	- **carrier**: TCP only — `wm_grpc_mixed` **1.021 PASS** (12b: 1.013);
   230	- **fixture**: mixed only — `wm_tcp_large` **1.039** and `wm_tcp_small`
   231	  **1.027** both PASS;
   232	- **isolation**: in 12c-win, 11 of 12 invariance cells pass at
   233	  1.003–1.044. `wm_tcp_mixed` is the sole outlier, by a wide margin.
   234	
   235	Also present in 12a's data? NOT testable there (review 2026-07-12):
   236	zoey's rig anchors converge-up only (12a README), so it has no
   237	mac_init/win_init invariance pair; its pull_tcp_mixed 0.966 is a
   238	new-vs-old check, not a two-layout measurement. P1 was never measured
   239	on zoey — that PASS must not be read as absence or masking evidence.
   240	
   241	**P2 — unified small-file push pays ~10–20% vs old push, both rigs**,
   242	`push_tcp_small` new-vs-old-same-session:
   243	
   244	| session | build | new | old | ratio |
   245	|---|---|---|---|---|
   246	| 12a zoey (RUNS=8, tight) | `e757dcc` old arm | — | — | **1.105** |
   247	| 12b netwatch-01 (3–4% spreads) | `e21cf84` | 2080 | 1811 | **1.149** |
   248	| 12c-win (2026-07-13) | `f35702a` (cutover) | 1975 | 1644 | **1.201** |
   249	
   250	**gRPC small push did NOT regress** (correction, review round 2: the
   251	earlier "win 0.98-ish per cells" was wrong against the committed CSVs;
   252	range corrected again in round 3). `push_grpc_small` new-vs-old,
   253	same-session / committed:
   254	
   255	| rig | same-session | committed |
   256	|---|---|---|
   257	| zoey | **1.001** | 0.907 |
   258	| netwatch-01 (12b) | **0.801** | 0.835 |
   259	| netwatch-01 (12c-win) | **0.852** | 0.802 |
   260	
   261	So the cross-rig range is **0.801–1.001**: gRPC small push is at parity
   262	on zoey and materially FASTER on Windows. The honest statement is **"TCP
   263	regressed while gRPC did not"** — not "gRPC is uniformly faster".
   264	
   265	That asymmetry is the finding's sharpest constraint on mechanism:
   266	whatever P2 is, it is TCP-data-plane-specific, source-initiated, and
   267	small-file-heavy (10k×4 KiB). **But it is a constraint, not a proof of
   268	innocence** (review round 3): an aggregate gRPC *improvement* cannot
   269	exclude a shared regression on both carriers that a larger
   270	gRPC-specific gain simply masks. Shared controller/planner/sink code is
   271	therefore NOT exonerated by the gRPC numbers, and pf-1 must attribute
   272	the TCP gap to a named delta rather than infer "TCP-only ⇒ not shared".
   273	
   274	Cross-block note (12b README): block-2 `mw_tcp_small` mac_init measured
   275	1922 vs block-1 new 2080 in the same session — the only mechanical
   276	difference is block-2's precreated destination container and per-arm
   277	path shapes; the investigation must confirm or kill that lead. It is a
   278	lead, not an attribution (a precreated container is environmental and
   279	cannot attribute code — Method 3(a)).
   280	

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '281,646p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   281	## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
   282	
   283	- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
   284	  connection-initiating end DIALS; byte direction is role-set
   285	  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
   286	  session the SOURCE is the responder: each sf-2 resize epoch is
   287	  ACCEPTED off the source's listener while the DESTINATION dials
   288	  (otp-5b-2: `SourceSockets` Dial/Accept branches;
   289	  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
   290	  accept/dial round-trips or serialization in the accept branch that the
   291	  dial branch does not pay.
   292	  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
   293	  was "mixed exercises resize hardest", but **all three fixtures target
   294	  eight streams before clamping** (`src/dial.rs:474`) — so resize
   295	  *count* cannot explain mixed-only behaviour, and H1 must name what
   296	  about mixed differs (shard-boundary timing? the tar-shard small half
   297	  interleaving with the big-file stream at the moment epochs fire?) or
   298	  be killed. **H1 also names the wrong half without proof**: it accuses
   299	  `Accept` while the destination's **synchronous dial-before-ACK** path
   300	  (`transfer_session/mod.rs:3113`) is an equally good suspect. pf-1 must
   301	  separate them with the dial/accept inversion counterfactual below —
   302	  "consistent with H1" is not confirmation.
   303	- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
   304	  interleave cannot happen — resize begins only after
   305	  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
   306	  layouts drain the same fixed 128-entry destination need loop, so
   307	  batch emission cannot interleave with the resize controller during
   308	  manifest/need emission in either layout. Kept only as a residual: if
   309	  pf-1 timing shows a layout-dependent need-batch delta anyway, the
   310	  mechanism must be re-derived from the trace, not from this text.
   311	- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
   312	  already killed its named candidates (the small half is tar-sharded and
   313	  written with parallel per-file `create_dir_all`/`fs::write`, NO
   314	  per-file flush; per-file progress emission to the served push
   315	  destination is disabled — `remote/transfer/sink.rs`; and old push used
   316	  the same served sink, so fsync/flush policy and progress emission are
   317	  NOT old/new deltas). What was left — "dest-side directory work/handle
   318	  churn" — **names no old/new code delta at all**, and its only probe
   319	  (precreate-vs-not) is explicitly environmental and cannot attribute
   320	  code (Method 3(a)). A hypothesis that cannot be confirmed *or* killed
   321	  by pf-1 is not a hypothesis; keeping it would let pf-1 close with a
   322	  shrug. It is therefore retired, and its one code-attributable
   323	  descendant — a per-member cost on the TCP receive path that old push
   324	  did not pay — lives on as **H6**, which names an executed-path delta.
   325	  H3 may only be revived if the pf-1 trace names a concrete old/new
   326	  delta in the destination directory/handle path; the 12b cross-block
   327	  precreated-container lead (8%, NTFS) is recorded as an environmental
   328	  lead for that trace, not as an attribution.
   329	- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
   330	  unchanged since `0f922de` (`remote/transfer/data_plane.rs`; the
   331	  earlier `dial.rs` attribution was wrong), and old small push ALSO
   332	  opened at one stream (after its 128-file early flush) then resized
   333	  live — so neither framing nor "fixed-count opening" discriminates.
   334	  What survives of H4 is ramp cadence/shard-boundary timing only, and
   335	  it is subordinate to H5.
   336	- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
   337	  scan/diff/transfer overlap on the TCP plane — current code withholds
   338	  every TCP payload until `ManifestComplete`
   339	  (`transfer_session/mod.rs`), while old push negotiated and queued
   340	  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
   341	  gRPC's in-stream carrier did not change comparably — which matches
   342	  the exact signature "TCP regressed while gRPC did not" (zoey gRPC at
   343	  parity 1.001, Windows gRPC faster; NOT "gRPC uniformly at parity" —
   344	  review round 3). NOTE: an H5 fix
   345	  reorders session phases and multi-ADD/pipelined epochs conflict with
   346	  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
   347	  ordering), so any H5 fix triggers this plan's Contract
   348	  stop-and-amend rule BEFORE implementation.
   349	- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
   350	  need-claim locking on the TCP receive plane — TCP receive
   351	  (`NeedListSink`) takes a separate mutex/hash-set claim per member
   352	  (`transfer_session/data_plane.rs:1167`), while the gRPC path claims
   353	  a whole shard under one lock (`transfer_session/mod.rs:3047`).
   354	  TCP-only and per-member (so small-file-heavy) — matches the P2
   355	  signature independently of H5. Discriminated by the pf-1 per-member
   356	  locking timings (Method 3(e), now unconditional).
   357	  **Historical control — corrected (review round 3): test the EXECUTED
   358	  path, not source presence.** `NeedListSink` *exists* in the tree at
   359	  `0f922de`, so "does the symbol exist there" is the wrong question and
   360	  would wrongly force H6 into a "multiplied claim frequency" story. What
   361	  matters is what old push actually RAN: at `0f922de` the served push
   362	  data plane goes `socket → StallGuard → execute_receive_pipeline →
   363	  FsTransferSink → disk`
   364	  (`crates/blit-daemon/src/service/push/data_plane.rs:185-206`) —
   365	  it **bypasses `NeedListSink` entirely** and takes no per-member claim.
   366	  So H6's claim is precise and falsifiable: the unified TCP receive path
   367	  introduced a per-member lock/hash-set claim on a path whose old
   368	  counterpart took none. pf-1 confirms it by (a) reading the executed
   369	  old path (done — cited above) and (b) the per-member locking timings;
   370	  it is KILLED if those timings do not scale with member count or do not
   371	  account for a material share of the P2 gap. If H6 is confirmed, the P2
   372	  fix bar applies unchanged (≤ 1.10 against BOTH references, BOTH rigs);
   373	  no separate bar is granted.
   374	  **H6's WALL-TIME counterfactual (added round 5 — timings alone would
   375	  strand pf-1 under the uniform decision rule):** behind a debug flag,
   376	  claim the whole tar shard under ONE lock on the TCP receive path —
   377	  i.e. give TCP the same batch-claim shape the gRPC path already uses
   378	  (`transfer_session/mod.rs:3047`), rather than a per-member claim
   379	  (`data_plane.rs:1167`). This is safe and wire-neutral (it changes only
   380	  the granularity of a local mutex/hash-set claim, not any frame), so it
   381	  does NOT trip the Contract rule. Grade its recovery against `Δ_P2` on
   382	  the uniform scale. If per-member claiming is the cost, batch-claiming
   383	  recovers it; if not, H6 dies with a number rather than a shrug.
   384	
   385	- **H7 (P2; added by review round 4 — the SHARED-controller candidate
   386	  the gRPC caveat predicted)**: HEAD's need/manifest bookkeeping is
   387	  heavier than old push's per entry. The unified source keeps a
   388	  **mutex-protected sent-manifest map** with per-entry insertion and
   389	  removal, and routes each need through a **per-need event-channel hop**
   390	  (`transfer_session/mod.rs:1038`, `:1123`, `:1350`); old push used a
   391	  **task-local map and handled need batches inline**, with no lock and no
   392	  channel hop per entry. This is **per-entry**, so it scales with FILE
   393	  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
   394	  **shared by BOTH carriers**. That is the precise class the round-3
   395	  gRPC caveat warned about: a shared regression can hide under gRPC's
   396	  larger carrier-specific gain, so "TCP-only symptom" does NOT exonerate
   397	  shared code. No prior hypothesis tested it. Discriminated by: per-entry
   398	  bookkeeping timings scaled against file count, plus the wall-time
   399	  counterfactual (a task-local/batch-inline path behind a debug flag).
   400	  H7 and H6 are independent and may BOTH contribute.
   401	
   402	## Method (the investigation slice — no behavior changes)
   403	
   404	1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
   405	   in-process/two-process rigs on the Mac with the otp-2 fixture
   406	   shapes; `--trace-data-plane` + targeted `tracing` spans (added
   407	   behind a debug flag, kept) around: resize epochs (arm→accept/dial→
   408	   ack), need-batch emission times, per-file sink open/write/close in
   409	   the receive path, shard planner in/out timestamps.
   410	2. **A/B the role layouts in one process**: the role suite already
   411	   runs both initiator layouts over identical fixtures (otp-3) — but
   412	   it forces the in-stream carrier (`transfer_session_roles.rs`), so
   413	   the timing-harness variant MUST add a TCP-carrier mode; it reports
   414	   phase timings per layout for mixed and small fixtures. A positive
   415	   layout-dependent delta in a named phase confirms; local ABSENCE
   416	   does not kill H1 (loopback removes the Windows↔Mac topology). So
   417	   that H1 stays falsifiable: if the local run is negative, pf-1
   418	   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
   419	   CELLS fixtures) before pf-1 may close — every hypothesis exits
   420	   pf-1 confirmed or killed, never "unfalsified" (review round 2).
   421	3. **Historical control, then bisect P2**: old push is deleted from
   422	   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
   423	   build and run; the control is an old-vs-new run on identical
   424	   fixtures. The new tracing spans do NOT exist in `0f922de` (review
   425	   round 2), so the control is observed externally — phase boundaries
   426	   from wire + filesystem timestamps and stdout progress, with event
   427	   semantics mapped span-for-span to the new names — or, where that is
   428	   too coarse, a minimal probe backport onto the pinned `0f922de`
   429	   source with identical event names. Either way every timed
   430	   configuration runs an instrumentation-on/off pair to bound observer
   431	   overhead (per-member tracing across ~10k files can perturb a
   432	   double-digit share of the measured gap). Experiments, corrected per
   433	   review 2026-07-12: (a) precreate-vs-not stays but is
   434	   environmental-only (it cannot attribute code); (b) the flush/
   435	   instrument toggles missed the tar-shard path — instrument the
   436	   tar-shard write path itself; (c) REPLACED (review round 2) — the
   437	   ramp pin discriminated nothing (old push also opened at one
   438	   stream), but H4 keeps a code-level counterfactual: a batch-cadence
   439	   replay toggle that processes need batches at the recorded old-push
   440	   shard-boundary cadence; (d) NEW, for H5 — the overlap experiment,
   441	   metric DEFINED (review round 2: "manifest-complete→first-payload
   442	   gap" was underdefined, and for old push the quantity is expected to
   443	   be NEGATIVE, which an unsigned "gap" cannot express). Record, per
   444	   run, on ONE common clock with a SIGNED offset from the
   445	   `ManifestComplete` event, three separately-named events on the
   446	   source side plus one on the destination:
   447	   `t_manifest_complete`; `t_first_payload_queued` (the payload enters
   448	   the send queue); `t_first_socket_write` (first byte handed to the
   449	   TCP data plane); `t_first_payload_received` (destination side —
   450	   requires the two clocks to be reconciled, so record the ssh/NTP
   451	   offset per run and report it with the number, or state that the
   452	   destination event was not usable). The overlap DIFFERENCE is
   453	   established only if `t_first_socket_write − t_manifest_complete` is
   454	   ≈0-or-positive on the new build and provably NEGATIVE on the pinned
   455	   `0f922de` control for the SAME fixture — i.e. old push really did put
   456	   TCP bytes on the wire before its manifest completed, and the new
   457	   session does not.
   458	   **That timestamp proves ORDERING, not CAUSATION, so it cannot confirm
   459	   H5 (review round 3).** H5 is confirmed only by a causal
   460	   counterfactual: a debug-flag toggle that restores mid-manifest TCP
   461	   payload queueing (queueing/ordering only — if it cannot be done
   462	   without a wire change, this plan's Contract stop-and-amend rule fires
   463	   FIRST) and measures WALL TIME on the same fixture and rig,
   464	   interleaved old-vs-new. Pre-registered: H5 is CONFIRMED iff the
   465	   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
   466	   KILLED if it restores the old ordering but does not move wall time —
   467	   which would prove the lost overlap is real and irrelevant, and hand
   468	   P2 to H6;
   469	   (e) per-member locking/framing timings are now an unconditional pf-1
   470	   measurement (they discriminate H6), not contingent on the trace
   471	   implicating them.
   472	4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
   473	   local rig is Mac↔Mac loopback: it removes the very platform terms P1
   474	   is confounded with, and it may equally fail to surface P2 (whose
   475	   Windows arms are the sharpest). So the rule is symmetric — **if a
   476	   finding does not reproduce locally, pf-1 REQUIRES the rig-side
   477	   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
   478	   since P2 was measured on both) with the same spans and the CELLS
   479	   fixtures, before pf-1 may close. Every hypothesis exits pf-1
   480	   confirmed or killed — never "did not reproduce, moving on".
   481	5. Every experiment lands as a committed probe record under
   482	   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   483	   loop per slice as usual.
   484	
   485	## pf-1 decision rule — UNIFORM, pre-registered (added round 5)
   486	
   487	Round-4 review: individual hypotheses had no shared decision threshold —
   488	H1 accepted any positive phase delta, H4's cadence replay had no
   489	threshold, H5 left a 1–49% recovery undecided, H6 left "material share"
   490	undefined. A phase-timing delta is **descriptive**; only wall time
   491	decides. So ONE rule governs every hypothesis (H1, H4, H5, H6, H7):
   492	
   493	- Each hypothesis must have a **wall-time counterfactual**: a debug-flag
   494	  variant that removes or restores exactly the accused mechanism, run
   495	  interleaved against the unmodified build on the same rig and fixture.
   496	  A hypothesis with no counterfactual **cannot be confirmed** — it is
   497	  carried as UNTESTED and pf-1 does not close.
   498	- **`Δ` is defined per finding and per rig — it is NOT one number**
   499	  (review round 5: the earlier text left it ambiguous between P1's
   500	  layout gap and P2's old/new gap, which are different quantities):
   501	  - **`Δ_P1(rig)`** = `destinit_median − srcinit_median` for
   502	    `wm_tcp_mixed` on THAT rig (an invariance gap: new-vs-new, no old
   503	    build involved). On rig W it is 1221 − 939 = **282 ms**. On
   504	    magneto↔skippy it is ~0 (8/8 pass) — so **P1 counterfactuals are
   505	    graded on rig W only**; a Linux-rig recovery is meaningless against a
   506	    gap that does not exist there.
   507	  - **`Δ_P2(rig)`** = `new_median − old_same_session_median` for
   508	    `push_tcp_small` on THAT rig (a converge gap, requires the `0f922de`
   509	    build on that rig). netwatch-01: 1975 − 1644 = **331 ms**; zoey:
   510	    4033 − 3636 = **397 ms**.
   511	  Every reported recovery names its `Δ` and its rig. A counterfactual run
   512	  on a rig whose `Δ` is ~0 proves nothing and is not reported as a kill.
   513	- **Overlapping causes are attributed SEQUENTIALLY, never summed**
   514	  (review round 5: H4/H7, and H6/H7, can each recover the same
   515	  milliseconds, so independent recoveries would double-count and could
   516	  "explain" >100% of `Δ`). Procedure: grade each hypothesis's recovery
   517	  ALONE against the unmodified build; then, for every confirmed
   518	  hypothesis in descending order of solo recovery, measure the
   519	  **incremental** recovery of adding it to the already-applied set. The
   520	  ≥70% closure test below is evaluated on the **cumulative combined**
   521	  build, not on the sum of solo recoveries.
   522	- The counterfactual's wall-time recovery `r` (as a share of the named
   523	  `Δ`) is graded on a **pre-registered scale**, no post-hoc bands:
   524	  - `r ≥ 50%` → **CONFIRMED DOMINANT** (fix it first)
   525	  - `20% ≤ r < 50%` → **CONFIRMED CONTRIBUTING** (fix it, but it is not
   526	    the whole story — keep hunting)
   527	  - `r < 20%` → **KILLED** as a material cause (recorded, not pursued)
   528	- **pf-1 closes only when the confirmed contributions account for ≥ 70%
   529	  of `Δ`** for each finding. If they do not, the residue is unexplained
   530	  and pf-1 **stays open** with the shortfall stated in the probe record —
   531	  never "several hypotheses were consistent, moving on".
   532	- Every measurement runs instrumentation-on/off pairs (per-member tracing
   533	  across ~10k files can itself perturb a double-digit share of `Δ`).
   534	
   535	## Fix criteria (pre-registered; the owner walks the final numbers)
   536	
   537	- **The global rule dominates every bar below** (review round 2 flagged
   538	  a contradiction between "necessary, not sufficient" and the `⇔`
   539	  bars — the `⇔`s are hereby scoped as *definitions of the named
   540	  finding's own bar*, never as a sufficient condition for acceptance).
   541	  Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in
   542	  EVERY acceptance cell passes independently against BOTH its
   543	  same-session reference AND the committed baseline — no arm may exceed
   544	  1.10 against either reference even when its counterpart bar passes
   545	  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
   546	  P2 bars below but regresses any other cell against either reference is
   547	  **not** accepted.
   548	- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
   549	  `pull_tcp_mixed` ≤ 1.10 against BOTH references on the netwatch-01
   550	  rig (CELLS escalation session, RUNS=8), with `wm_grpc_mixed` and the
   551	  other invariance PASSes unregressed against both references. (Meeting
   552	  this bar does not by itself accept the build — see the global rule.)
   553	- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
   554	  (same-session AND committed) on BOTH rigs (CELLS sessions), with the
   555	  gRPC small-push cells unregressed. **"Unregressed" is given a
   556	  reference and a tolerance (review round 3)**: each gRPC small-push
   557	  cell must stay ≤ 1.10 against both of its own references AND must not
   558	  worsen by more than **10% against its own pre-fix median on the same
   559	  rig** (zoey 4731 ms; netwatch-01 2264 ms at 12c-win). The second
   560	  clause exists because those cells currently range 0.801–1.001 — a fix
   561	  that dragged Windows gRPC from 0.85 back to 1.05 would still pass a
   562	  bare ≤1.10 bar while having eaten a real, measured win.
   563	- Cross-direction converge-up is a SEPARATE bar (review round 2):
   564	  every final cross-direction row must still meet the parent plan's
   565	  new-vs-old ceiling (`ONE_TRANSFER_PATH.md` acceptance) or satisfy
   566	  the registered platform-residue discriminator — invariance plus the
   567	  per-direction bars alone would pass if a "fix" slowed BOTH layouts
   568	  equally, violating converge-up.
   569	- No suite regressions; the floor is ≥ the CURRENT count (1484 —
   570	  ≥1483 would permit silently losing a test); any new pins carry
   571	  guard proofs (temporary revert) per the loop.
   572	- If investigation attributes part of a gap to something the plan's
   573	  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
   574	  that residue is RECORDED with its experiment and goes to the owner's
   575	  otp-13 walk — never silently accepted.
   576	
   577	## Staging (each through the codex loop)
   578	
   579	- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
   580	  the two-layout phase-timing report (TCP-carrier mode included) + the
   581	  `0f922de` historical control; probe record committed AND
   582	  codex-reviewed BEFORE any pf-2 branch exists. No fix lands on
   583	  pre-pf-1 evidence.
   584	- **pf-2..n**: one fix slice per confirmed root cause (smallest
   585	  change that moves the phase timing; A/B'd locally before rig time).
   586	- **pf-final**: NOT just the two escalation cells — the final build
   587	  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
   588	  gRPC controls) on **all THREE rigs: Z (zoey), W (netwatch-01) and
   589	  D (delegated, netwatch-01↔skippy)**. **No mixed-build evidence: every
   590	  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
   591	  (corrected, review round 2 — "every row" was impossible: the
   592	  same-session `old` arms and the committed baselines are OLD builds by
   593	  construction, which is the entire point of a reference). Pre-fix
   594	  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
   595	  which are **replication and control evidence, not acceptance
   596	  evidence**.
   597	  **Rig D is included even though it is not a suspect (review round
   598	  3).** Voiding otp-12c's pre-fix rows while re-running only Z and W
   599	  would leave the parent plan's **delegated-parity bar**
   600	  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
   601	  evidence at all. "Not implicated" scopes what pf-1 must
   602	  *instrument* — it does not waive an acceptance bar. Rig D's TCP
   603	  verdict cells (+ the gRPC smoke) therefore rerun on the final build;
   604	  both arms are new-build by construction there (rig D has no old
   605	  baseline), so the whole cell is re-measured.
   606	  **Every gRPC row the acceptance method requires reruns
   607	  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
   608	  earlier "if shared code changed, the gRPC cells rerun too" left the
   609	  decision to the author's own judgement of what counts as shared, which
   610	  is exactly the loophole H7 exploits: a shared regression can hide under
   611	  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
   612	  complete Z/W gRPC converge and invariance rows, so those are
   613	  final-build rows, full stop — no conditional. Results land in fresh
   614	  dated evidence dirs. **Then** otp-12d assembles the matrix from
   615	  final-build rows, and the otp-13 owner walk reads it.
   616	
   617	## Known gaps
   618	
   619	- H1–H5 were graded against the actual tree by codex review
   620	  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
   621	  The old drivers are deleted from HEAD, but the pinned `0f922de`
   622	  source/binaries diff and run fine — historical claims get live
   623	  controls in pf-1, not pin-archaeology.
   624	- zoey never measured P1: its rig anchors converge-up only, so there
   625	  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
   626	  says nothing about layout asymmetry (review 2026-07-12). pf-1's
   627	  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
   628	  qualifies per the 12b wm numbers).
   629	- **The 12c-win rows are replication, not acceptance** (2026-07-13).
   630	  They are pre-fix by definition, so `pf-final` voids them for
   631	  acceptance; their value is that they (a) reproduce P1 and P2 on an
   632	  independent session at the shipped sha, (b) supply the
   633	  opposite-direction control (`mw_tcp_mixed` 1.044 PASS vs
   634	  `wm_tcp_mixed` 1.300 FAIL — same carrier, same fixture) that narrows
   635	  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
   636	  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
   637	  P2 1.149→1.201), so neither is drifting toward the bar on its own.
   638	- **Rig-D delegated parity is not a SUSPECT, but it is still an
   639	  ACCEPTANCE bar** (2026-07-13; scoped correctly at review round 3): the
   640	  delegated-vs-direct matrix passed 7/7
   641	  (`docs/bench/otp12c-delegated-2026-07-13/`), so delegation adds no
   642	  measurable cost and pf-1 need not instrument the delegated trigger
   643	  path. That is a statement about *where to look for the bug* — it does
   644	  **not** waive the parent plan's delegated-parity bar, whose evidence
   645	  is pre-fix and therefore void under pf-final. Rig D reruns on the
   646	  final build (see pf-final).

exec
/bin/zsh -lc "nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '1,180p' && nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '240,318p' && nl -ba docs/DECISIONS.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# One transfer path — direction-invariant by construction
     2	
     3	**Status**: Active
     4	**Created**: 2026-07-05
     5	**Supersedes**: post-REV4 residue item "pull 1s-start restructuring"
     6	(absorbed here); pauses `docs/plan/SMALL_FILE_CEILING.md` after sf-2
     7	(D-2026-07-05-1). REV4's mixed-version-peers constraint is superseded
     8	outright by **D-2026-07-05-2 (no version compatibility, ever — same
     9	build only)** — annotated in REV4 §Constraints
    10	**Decision ref**: D-2026-07-05-1 (directive + pause);
    11	**D-2026-07-05-4 (Draft → Active, owner "flip the plan and go",
    12	2026-07-05)**
    13	
    14	## Directive (owner, 2026-07-05, verbatim)
    15	
    16	> "make ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF
    17	> ANYTHING EVER using anything else because anything else does not
    18	> exist."
    19	
    20	> "just make it so that I NEVER see a situation where pull is faster
    21	> than push or vice versa. that CAN NEVER be possible because of
    22	> something blit did. it should be identical if I start the transfer
    23	> from skippy and push to this machine or if I start the transfer on
    24	> this machine and pull from skippy."
    25	
    26	> On benchmark methodology: "tmp on one side, spinning rust on the
    27	> other is not a valid test."
    28	
    29	Scope, wire, and process were explicitly delegated to the agent
    30	("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
    31	owner's requirement is the invariant; everything below is the
    32	architecture that makes the invariant impossible to violate rather
    33	than merely maintained by discipline.
    34	
    35	## Goal
    36	
    37	One `TransferSession` implementation owns every byte transfer blit
    38	performs. A transfer has a SOURCE role and a DESTINATION role; which
    39	end initiated, and which CLI verb was used, select roles — they do not
    40	select code. When this plan ships, the per-direction drivers (client
    41	push driver, daemon push-receive, client pull driver, daemon
    42	pull-send, delegated-pull driver, local orchestration) **do not
    43	exist**: for fixed endpoints and dataset, direction/initiator/verb
    44	cannot affect behavior or wall time by blit's doing, because there is
    45	no second code path to differ.
    46	
    47	## Non-goals
    48	
    49	- Version compatibility of ANY kind (D-2026-07-05-2, owner standing
    50	  rule: "backward compatibility is NOT a consideration... same build
    51	  only. do not engineer tech debt into an unshipped product"). A blit
    52	  client talks only to a blit-daemon from the same build; the session
    53	  handshake REFUSES a mismatched peer outright. No negotiate-down, no
    54	  advisory fields, no feature-capability bits for version skew.
    55	  `Push`/`PullSync` are deleted at cutover with no bridge. (Old-path
    56	  code coexists in-tree during the migration slices solely so each
    57	  slice lands green — that is migration scaffolding, not wire
    58	  compatibility.)
    59	- Making different hardware perform identically. If src and dst sit
    60	  on different disks, the two *data directions* still differ by
    61	  physics; the invariant is that the same data direction between the
    62	  same endpoints is identical regardless of who initiates and which
    63	  verb is used.
    64	- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
    65	- New features. This is a consolidation; capability parity with
    66	  today (mirror, filters, resume, fallback, delegation, progress,
    67	  jobs, cancellation) is the bar. Zero-copy receive is **unparked**
    68	  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
    69	  after cutover, not one of this plan's slices — see the Design note
    70	  on the write-strategy seam. One narrow owner-granted exception
    71	  (D-2026-07-09-1, otp-7b): the CLI end-of-operation fault summary —
    72	  name the file(s) a session fault affected and suggest a re-run —
    73	  lands inside otp-7. Nothing else new rides this plan.
    74	
    75	## Constraints
    76	
    77	- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
    78	  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
    79	  violated at the choreography layer.
    80	- **Converge up, not down**: per benchmark cell, the unified session
    81	  must match the better of today's two directions (within ±10% run
    82	  noise), not their average. Unification that slows the fast
    83	  direction fails review.
    84	- REV4 invariants carry: byte-identical results, StallGuard,
    85	  cancellation, byte-accounting. Existing pins are ported (not
    86	  dropped) as tests become role-parameterized; test count never
    87	  drops.
    88	- The sf-2 shape-correction behavior (stream count corrects as the
    89	  need list accumulates) becomes the one and only stream policy —
    90	  both directions inherit it by construction; its pins carry over.
    91	- **The bounded-unilateral dial contract carries unchanged**
    92	  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
    93	  dial, bounded by the byte RECEIVER's advertised capacity profile
    94	  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
    95	  unlimited). The session's role model must express this — profile
    96	  travels DESTINATION→SOURCE at setup regardless of who initiated —
    97	  and otp-1's contract names it explicitly.
    98	- Wire contract discipline (REV4 rule): the unified session's proto —
    99	  messages, field numbers, capability negotiation, transport
   100	  selection — is a reviewed doc+proto slice **before** any behavior
   101	  depends on it.
   102	- Every slice through the codex loop (D-2026-07-04-1); tree green
   103	  after every slice; transitional coexistence of old+new paths is
   104	  scaffolding only — the plan is not Shipped until the deletion slice
   105	  lands and the deletion proof is recorded.
   106	- Windows parity: suite green on the owner's machine + windows-latest
   107	  CI before Shipped.
   108	
   109	## Acceptance criteria
   110	
   111	- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
   112	      on a symmetric rig (same filesystem class both ends, cold
   113	      caches, disk-to-disk), for each data direction and workload
   114	      (large / 10k-small / mixed): wall time initiating from end A vs
   115	      end B, and via push-verb vs pull-verb, differs only within
   116	      run-to-run noise (±10%). Matrix committed as evidence.
   117	      (Instantiation: no same-fs-class 10 GbE pair exists in the
   118	      fleet; the owner designated Mac↔Windows as the closest-spec
   119	      cross-direction rig, 2026-07-10 — otp-2w README §Status. The
   120	      invariance A/B stays valid there because both arms of a pair
   121	      share the same endpoints, so endpoint asymmetry cancels within
   122	      each pair; cross-direction evaluation per D-2026-07-12-1.)
   123	- [ ] **Converge up, measured (codex F4)**: before cutover, the
   124	      corrected symmetric-fs harness records a per-cell baseline of
   125	      the OLD paths, both directions; after cutover, every unified
   126	      cell must be ≤ the better of that cell's two old directions
   127	      + run noise (±10%). A symmetric-but-slower result fails.
   128	      (Evaluation rule on the owner-designated cross-direction rig:
   129	      a cell that meets per-direction converge-up and invariance but
   130	      misses this bar only by a discriminator-attributed destination
   131	      write-path residue counts as satisfied — D-2026-07-12-1;
   132	      `docs/plan/OTP12_ACCEPTANCE_RUN.md` D2.)
   133	- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
   134	      (driver), daemon `push/control.rs` choreography, daemon
   135	      `pull_sync.rs` choreography, the delegated-pull driver, the
   136	      separate local orchestration path, and the `Push`/`PullSync`
   137	      RPCs no longer exist in the tree; one `TransferSession` and one
   138	      `Transfer` RPC remain. The `DelegatedPull` RPC may survive only
   139	      as trigger + progress relay — the proof must show it carries no
   140	      payload bytes (codex F3). Recorded file-by-file in the final
   141	      slice's finding doc.
   142	- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
   143	      guard), filters, block-resume, gRPC fallback carrier, delegated
   144	      transfer, progress events, jobs/cancel, read-only enforcement —
   145	      each demonstrated by ported tests on the session.
   146	- [ ] Suite green throughout; final test count ≥ pre-plan baseline
   147	      (1483); all REV4 invariant pins and the sf-2 pin pass
   148	      role-parameterized.
   149	- [ ] Benchmark methodology corrected and recorded: symmetric-fs
   150	      cells are the verdict cells; tmpfs cells remain only as
   151	      explicitly-labeled wire-reference rows (never compared across
   152	      directions with asymmetric endpoints).
   153	- [ ] Windows: full suite green (owner machine) + windows-latest CI.
   154	
   155	## Design
   156	
   157	**What already is one code** (kept, becomes the session's engine):
   158	`remote/transfer/` — pipeline, sink/source abstractions, data plane,
   159	diff planner, tar-shard, stall guard, progress, `operation_spec` (the
   160	REV4 unified contract), and the engine dial (stream policy incl. sf-2
   161	shape correction). The defect layer is above it: four driver loops
   162	choreograph these pieces differently per direction.
   163	
   164	**The one choreography** (roles, not directions):
   165	
   166	1. Initiator opens the single bidi `Transfer` RPC and sends the
   167	   operation spec: which end is SOURCE, which is DESTINATION, path/
   168	   module, filters, mirror/resume flags, capabilities.
   169	2. SOURCE enumerates and **streams** its manifest immediately (no
   170	   buffered-enumeration phase — this generalizes push's fast start;
   171	   pull's full-enumeration-then-negotiate slow start is deleted, which
   172	   absorbs the "pull 1s-start" residue item).
   173	3. DESTINATION diffs incrementally against its own filesystem and
   174	   returns need-list batches (one diff owner, always the end that
   175	   owns the target fs — push's proven model; pull_sync's
   176	   source-side diff is deleted).
   177	4. The data plane opens at the dial floor immediately; stream count
   178	   shape-corrects as the need list accumulates (sf-2 mechanism, now
   179	   the only policy, both roles).
   180	5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
   240	**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
   241	mitigated by strangler slices with the tree green throughout and a
   242	non-optional deletion slice. Per-cell regression risk on today's
   243	faster direction — mitigated by the converge-up constraint and
   244	baseline parity pins per slice. Wire break — lockstep upgrade,
   245	owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
   246	Progress/jobs/TUI integration churn — the session emits the existing
   247	event contract (w6-1) at the same boundaries.
   248	
   249	## Slices
   250	
   251	One coherent, testable change per slice — sized for the `.review/`
   252	loop. Tree green after every slice; old paths keep working until
   253	otp-9 deletes them.
   254	
   255	1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
   256	   `Transfer` RPC and message set — roles, phases, field numbers,
   257	   the **strict same-build handshake** (exact protocol/build identity
   258	   exchanged at session open; any mismatch is refused with a clear
   259	   error — D-2026-07-05-2; pinned by test when the session lands),
   260	   the receiver capacity profile + bounded-unilateral dial contract
   261	   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
   262	   that exists), transport selection, resume phase ordering (the
   263	   RELIABLE exception above), mirror phase, error/cancel semantics.
   264	   No feature-capability bits: same build implies same features.
   265	   The new proto text must carry NO version-tolerance semantics; the
   266	   capacity profile's absent/0 fields mean "unknown hardware value"
   267	   only, never "old peer" (today's proto comments frame some of that
   268	   contract as old-peer fallback — those comment blocks describe live
   269	   pre-cutover code and die with their messages at otp-10, per the
   270	   D-2026-07-05-2 review adjudication). Codex-reviewed before any
   271	   code consumes it.
   272	2. **otp-2 symmetric baseline (harness + rig, no production code)**:
   273	   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
   274	   cells, cold caches, tmpfs rows re-labeled wire-reference only —
   275	   and record the OLD paths' per-cell, per-direction baseline on the
   276	   rig. This is the converge-up reference the acceptance criteria
   277	   compare against (codex F4).
   278	3. **otp-3 TransferSession core (blit-core)**: role-parameterized
   279	   state machine over the existing engine with an in-process
   280	   transport; unit/e2e tests run BOTH role assignments over the same
   281	   fixtures — the invariance property enters the test suite here.
   282	4. **otp-4 daemon serves `Transfer`, client initiates as SOURCE**
   283	   (remote push-equivalent rides the session); A/B parity pins vs
   284	   old push (byte-identical trees, summary parity, sf-2 pin ported).
   285	5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-
   286	   equivalent) — the same code with roles flipped; the parity suite
   287	   reruns with no per-direction test code.
   288	6. **otp-6 mirror + filters** on the session (one delete rule).
   289	7. **otp-7 resume** block phase (ordering + stale-partial pins per
   290	   the Design's RELIABLE exception). Slice design: `docs/plan/OTP7_RESUME.md`
   291	   (staged 7a in-stream / 7b data-plane).
   292	8. **otp-8 fallback byte-carrier** (control-stream frames) as the
   293	   session's alternate transport.
   294	9. **otp-9 delegated transfer** = daemon-initiated session; bespoke
   295	   delegated-pull driver retired behind the existing gate;
   296	   `DelegatedPull` RPC reduced to trigger + progress relay.
   297	10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
   298	    operation through the session; `Push`/`PullSync` and all four
   299	    drivers deleted from the tree and the proto, no bridge
   300	    (D-2026-07-05-2); ported-test accounting proves count never
   301	    dropped. Deletion proof recorded, incl. the DelegatedPull
   302	    no-payload-bytes assertion.
   303	11. **otp-11 local transfers** ride the in-process transport; the
   304	    separate local orchestration is deleted; local perf pins hold.
   305	12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
   306	    on the unified path — initiator/verb invariance A/B within noise
   307	    AND every cell ≤ the better old direction + noise; committed as
   308	    this plan's acceptance evidence.
   309	13. **otp-13 verdict**: acceptance checklist walked with the owner;
   310	    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
   311	    against the unified baseline — owner call at that point.
   312	
   313	## Open questions
   314	
   315	- None requiring owner input now — scope, wire, and process were
   316	  delegated (Directive section). Slice-level unknowns (exact proto
   317	  shapes, resume edge semantics, TUI event wiring) are settled inside
   318	  their slices through the codex loop. — owner
     1	# DECISIONS — settled choices
     2	
     3	**Status**: Active
     4	
     5	Append-only ledger of decisions that future sessions must not relitigate or miss.
     6	Add entries via the `decision` procedure in `docs/agent/PROTOCOL.md`. Newest last.
     7	When a decision supersedes plan text, the plan text gets edited in the same
     8	session — this file is the index, not a substitute for fixing the doc.
     9	
    10	Format:
    11	
    12	```
    13	## D-<YYYY-MM-DD>-<n> — <short title>
    14	- Decision: <one line>
    15	- Why: <one line>
    16	- Supersedes: <doc §/decision ID, or "nothing">
    17	```
    18	
    19	---
    20	
    21	## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
    22	- Decision: `RELEASE_PLAN_v2_2026-05-04.md` is a frozen reference, no longer the active source of truth.
    23	- Why: 0.1.0 tagged 2026-05-31; the plan served its purpose.
    24	- Supersedes: RELEASE_PLAN_v2_2026-05-04.md as active plan.
    25	
    26	## D-2026-05-31-2 — Pick-not-Type TUI direction
    27	- Decision: `TUI_REWORK.md` (dual-pane, M1–M6) supersedes `TUI_DESIGN.md` §6 trigger-modal text inputs and the F3 free-text destination prompt.
    28	- Why: any field requiring the operator to recall and type an off-screen path is an interface failure.
    29	- Supersedes: TUI_DESIGN.md §6 (portions).
    30	
    31	## D-2026-06-04-1 — R3 overrides R2 in the audit chain
    32	- Decision: where R2 and R3 disagree on a finding's severity or content, R3 wins; see the ID-override table in `AUDIT_REPORT_2026-06-04_INDEX.md`.
    33	- Why: R3 incorporates the GPT R2 critique and severity rebalance.
    34	- Supersedes: conflicting R2 entries.
    35	
    36	## D-2026-06-04-2 — Env vars are out for app + diagnostic config
    37	- Decision: no environment-variable configuration carve-out (R3-L39); purge completed via `audit-l39-m27-env-var-purge`.
    38	- Why: owner policy — config surfaces stay explicit.
    39	- Supersedes: nothing (clarifies prior ambiguity).
    40	
    41	## D-2026-06-04-3 — Streaming planner ratified, build deferred
    42	- Decision: `greenfield_plan_v6.md` §1.1 (streaming planner + 1 s heartbeat + 10 s stall detector) is canonical but not yet built; multi-slice implementation queued after audit Round 1 (H10b).
    43	- Why: data-loss/DoS hardening takes priority; the plan claim is ratified rather than retired.
    44	- Supersedes: nothing.
    45	
    46	## D-2026-06-06-1 — STATE.md precedence model adopted
    47	- Decision: `docs/STATE.md` is the single entry point for current state, with the precedence order in `AGENTS.md` §1; DEVLOG.md is write-only history, TODO.md is backlog-only, tool-local memories are scratch.
    48	- Why: state smeared across TODO/DEVLOG/plan-README/Serena was the drift mechanism the 2026-06-04 audit documented (drift-* findings, M28).
    49	- Supersedes: "Agent-Specific Expectations" in the previous AGENTS.md (Serena memories as session persistence).
    50	
    51	## D-2026-06-07-1 — Keep the `c793df2` octopus on master; no history rewrite
    52	- Decision: `c793df2` (a `git merge -s ours` octopus whose parents are `600023a` + `eafb187` + `d9d4ec7`) stays on `origin/master`; we do **not** rewrite history or force-push to remove it.
    53	- Why: its tree is byte-identical to `600023a` (`git diff 600023a c793df2` is empty) and the workspace builds, so it is cosmetically ugly but harmless; rewriting already-pushed shared history is riskier than the wart. The merge was pushed without owner approval — the corrective is the new AGENTS.md §8 Git-safety contract, not a second unsafe operation.
    54	- Consequence (the trap): because `eafb187` and `d9d4ec7` are now *ancestors* of master, `git branch --merged` falsely reports them merged and a plain `git merge` of either no-ops without landing code. `d9d4ec7` (adaptive-streams-pr3-resizable) does **not** build and its files are not in master's tree. Branch cleanup in this repo is by explicit name only, never `--merged`.
    55	- Supersedes: nothing.
    56	
    57	## D-2026-06-07-2 — Adaptive-streams lands via cherry-pick/rebase, excluding the WIP
    58	- Decision: the adaptive-streams stack (live-progress → PR1 telemetry → PR2 work-queue → PR2 review fix, up to `eafb187`) lands later as a planned `docs/plan/` slice via cherry-pick or rebase onto fresh commits — never via `git merge` of the branch (see D-2026-06-07-1 trap). `d9d4ec7` (PR3 WIP, "DOES NOT BUILD") is explicitly excluded until it is finished and compiles.
    59	- Why: the `-s ours` octopus recorded those tips as parents without landing their code, so the feature is not actually in master; a real merge would no-op. The one real conflict (`data_plane.rs`: `StallGuardWriter` vs the `Probe` generic) must be resolved by hand, which only a cherry-pick/rebase surfaces.
    60	- Supersedes: nothing.
    61	
    62	## D-2026-06-11-1 — Design-coherence review plan Active; ratification covers Phase A only
    63	- Decision: `docs/plan/DESIGN_COHERENCE_REVIEW.md` flipped Draft → Active. Owner approval authorizes **Phase A only** (concept-ownership map + per-crate stratum inventory); Phases B and C each need a fresh go/no-go at the preceding checkpoint. Interview decisions bound into the plan: blit-tui light pass, owner ratifies each Phase C finding, wire-breaking recommendations in scope (proto not frozen).
    64	- Why: the repo was built by many models across several greenfield restarts and the owner judges it too inconsistently designed to trust as-is; mapping concept ownership precedes any re-scope (audit-h3c slice 2) or feature landing (adaptive-streams) so the fixes get designed once.
    65	- Supersedes: nothing.
    66	
    67	## D-2026-06-11-2 — Design-review queue ratified in full; Pull-RPC delete; zero_copy gets a FAST evaluation
    68	- Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
    69	- Why: review program (D-2026-06-11-1) delivered all three phases; owner is the gate for queue entry and exercised it in full.
    70	- Supersedes: nothing (completes D-2026-06-11-1; `DESIGN_COHERENCE_REVIEW.md` flips Active → Shipped).
    71	
    72	## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
    73	- Decision: `zero_copy.rs` is deleted rather than wired in. The w8-1b evaluation (`docs/plan/ZERO_COPY_RECEIVE_EVAL.md`) recommended deletion and the owner agreed (2026-06-12 session). The deletion executes inside w8-1 once the w5-1 sentinel (lib.rs) is graded — it is no longer excluded from that sweep.
    74	- Why: the dead draft busy-waits on EAGAIN (would be rewritten, not revived); wiring needs a raw-fd special case beside a permanent buffered fallback; the CPU saving is a fraction of one core, Linux-only, and unmeasured. Revisit gate: 10 GbE benchmarks showing receive-side CPU saturation — design notes preserved in the eval doc.
    75	- Supersedes: D-2026-06-11-2 item (b) (zero_copy exclusion from W8.1 was pending this evaluation; the evaluation is done).
    76	
    77	## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
    78	- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
    79	- Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
    80	- Scope consequence: this **moots the standalone premise** of the queued incremental work and absorbs the goals — w2-2 (three ladders → one dial) is `ue-1b`; w2-3 multi-stream pull (`MULTISTREAM_PULL.md`) is `ue-1d` via the unified sequencer; w2-4 (delete deprecated Pull RPC) is `ue-1e`; adaptive-streams cherry-pick is `ue-1a`. `MULTISTREAM_PULL.md` is superseded as a standalone plan (kept as reference); its goal survives inside this plan. The design-review queue's correctness findings (w4-1 etc.) are independent and unaffected.
    81	- Supersedes: the "ground-up redesign" framing of the 2026-06-14 open question recorded in STATE.md (that open question is now closed); `MULTISTREAM_PULL.md` as a standalone plan (goal absorbed into `UNIFIED_TRANSFER_ENGINE.md` slice `ue-1d`).
    82	
    83	## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
    84	- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` is **Active**. Owner approved with four parameters that bind the design: (q1) **no probe-then-go phase** — the engine starts moving within ~1s at conservative defaults bounded by the receiver ceiling and the tuner adjusts dials live from the first byte; the "small-transfer threshold" is obviated (no probe to skip), and the **planner** carries the workload-shape judgment (file count vs bytes) that the old size gate proxied. (q2) the receiver advertises a **rich capacity profile** (CPU cores, disk class, load, max streams, drain estimate) — "more data serves the ubergoal"; do not minimize the negotiation payload. (q3) engine type **deferred to the agent**, who recommends a new src/dst-agnostic `TransferEngine` + a local adapter over renaming `TransferOrchestrator` in place — ratified at `ue-1c`. (q4) `ue-2` (mid-transfer stream add/drop via PR3's resize proto) is **in scope at Active**, sequenced last; 11 months of owner benchmarking is the justification, the 10 GbE rig is sign-off not a gate.
    85	- Why: owner answered the four gating questions (the stated Draft→Active condition) and said "active now." q1 materially improved the design — live-from-first-byte removes the fragile size threshold and collapses the A/B/C probe staging into "adjust what is cheap in `ue-1b`, add stream resize in `ue-2`."
    86	- Inference flagged for owner (now vetoed — see D-2026-06-20-3): the agent had proposed folding the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b) in as the planner half and superseding its "after audit Round 1" timing. **Owner vetoed 2026-06-20.** The absorption is dropped; D-2026-06-04-3 stands unchanged. The engine's workload-shape-awareness + first-byte-within-~1s requirements remain, stated on their own merits, not as the H10b concept.
    87	- Supersedes: the "A-first warmup probe" and "size-gated skip-probe" framings in the Draft version of `UNIFIED_TRANSFER_ENGINE.md` (already edited in-place). *(The proposed supersession of D-2026-06-04-3's streaming-planner timing is withdrawn per the owner veto — see D-2026-06-20-3.)*
    88	
    89	## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
    90	- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
    91	- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
    92	- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).
    93	
    94	## D-2026-06-20-4 — Unified transfer engine plan review freeze
    95	- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
    96	- Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
    97	- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.
    98	
    99	## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
   100	- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
   101	- Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
   102	- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*
   103	
   104	## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
   105	- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
   106	- Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
   107	- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.
   108	
   109	## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
   110	- Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
   111	- Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
   112	- Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).
   113	
   114	## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
   115	- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
   116	- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
   117	- Supersedes: nothing (closes the STATE.md "commit erratum" blocked item).
   118	
   119	## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
   120	- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
   121	- Why: owner call 2026-07-04 ("flip it"). The original "disconnect is the cancel" rationale predates w4-3's race wiring; the flip is now policy-only, and cancel-from-anywhere (second terminal, TUI) is strictly more operable than find-and-kill-the-client.
   122	- Supersedes: the DelegatedPull-only cancellation policy recorded in `active_jobs.rs`'s `supports_cancellation` rustdoc (edited when the slice lands) and the corresponding "policy deliberately unchanged" scope note in the w4-3 finding doc (which anticipated exactly this flip).
   123	
   124	## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
   125	- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
   126	- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
   127	- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).
   128	
   129	## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
   130	- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
   131	- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
   132	- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.
   133	
   134	## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
   135	- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
   136	- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
   137	- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).
   138	
   139	## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
   140	- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
   141	- Why: the 10 GbE session showed skippy's 32-core receiver at 1.43 cores — gate not met on that rig — but the gate was always about CPU-bound receivers, and the owner now operates one. On a CPU-bound receiver, cutting the userspace copy is exactly the FAST lever the eval preserved design notes for.
   142	- Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.
   143	
   144	## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
   145	- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
   146	- Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
   147	- Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).
   148	
   149	## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
   150	- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
   151	- Why: owner answered Q1–Q3 in session 2026-07-09; the flip is the approval the plan procedure requires. In the same exchange the owner re-confirmed the broader progress-display redesign (persistent stats block + scrolling file frame, "probably a TUI") — that stays a queued TODO.md item ("CLI transfer output redesign"), NOT otp-7 scope, and needs its own plan.
   152	- Supersedes: nothing (the plan doc's Open-questions section is rewritten as resolved in the same commit).
   153	
   154	## D-2026-07-10-1 — Resume wire bounds on the in-stream carrier (amends OTP7_RESUME D5)
   155	- Decision: The session's resume block phase is bounded so no legal open can produce a frame the gRPC-served in-stream carrier cannot deliver, nor an amplified hash list (codex otp-7a F1). The DESTINATION clamps `ResumeSettings.block_size` into **[64 KiB, 2 MiB]** (`MIN_RESUME_BLOCK_SIZE`, `MAX_IN_STREAM_RESUME_BLOCK_SIZE`; `0` ⇒ 1 MiB default) — floor kills block_size=1's 32× hash-list amplification, ceiling keeps a one-block `BlockTransfer` frame under tonic's default 4 MiB decode limit — and caps any one `BlockHashList` at **65_536 hashes** (2 MiB of hashes); a partial with more blocks degrades to the empty list, i.e. the plan-D1 graceful full-transfer fallback, never an oversized frame. The SOURCE range-validates the wire block size at frame arrival (same-build peers, D-2026-07-05-2: out-of-range is a protocol violation, not a negotiation). otp-7b revisits the ceiling for the TCP data plane, whose binary block records carry no protobuf envelope.
   156	- Why: plan D5 as drafted clamped only to `MAX_BLOCK_SIZE` (64 MiB), which is fine for local block copies but 16× over the unraised tonic frame limit the served in-stream carrier actually has — a legal open would fail mid-transfer (RELIABLE violation), and a hostile-or-buggy tiny block size would OOM-amplify the hash list. Pinned by `resume_block_size_floor_clamps_tiny_requests`, `resume_block_size_ceiling_clamps_oversized_requests` (guard-proven by clamp removal), and the pure-fn cap boundary test.
   157	- Supersedes: OTP7_RESUME.md D5's "clamped to `MAX_BLOCK_SIZE`" wording (amended in place, same commit).
   158	
   159	## D-2026-07-10-2 — Resume block-size ceiling is per carrier (completes the D-2026-07-10-1 revisit)
   160	- Decision: The resume block-size ceiling the DESTINATION clamps to (and the SOURCE range-validates at `BlockHashList` arrival) is **the carrier's**: **2 MiB** on the in-stream carrier (unchanged, D-2026-07-10-1) and **64 MiB** on the TCP data plane (`MAX_DATA_PLANE_RESUME_BLOCK_SIZE` = the receive pipeline's `MAX_WIRE_BLOCK_BYTES` = the old resume path's `MAX_BLOCK_SIZE`). Both ends decide by grant presence — grant ⇒ data plane — so same-build peers agree without negotiation. The floor (64 KiB) and the 65_536-hash `BlockHashList` cap are carrier-independent (the hash list always rides the control lane as protobuf); a partial with more blocks than the cap still degrades to the D1 full-transfer fallback. Session-wide block size stays; per-file block-size auto-scaling for very large partials (>4 TiB at 64 MiB blocks) remains future work.
   161	- Why: binary data-plane `BLOCK` records carry no protobuf envelope, so the 2 MiB tonic-frame rationale does not apply there; the wire already enforces `MAX_WIRE_BLOCK_BYTES` = 64 MiB on the receive side. A larger ceiling lets a data-plane session keep block-wise resume for partials up to 4 TiB (65_536 × 64 MiB) instead of degrading to full transfer at 128 GiB (the 2 MiB-ceiling limit).
   162	- Supersedes: nothing — completes the revisit D-2026-07-10-1 explicitly deferred to otp-7b (OTP7_RESUME.md D5 amended in place, same commit).
   163	
   164	## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
   165	- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
   166	- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
   167	- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).
   168	
   169	## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
   170	- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
   171	- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
   172	- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
   173	
   174	## D-2026-07-13-1 — OTP12_PERF_FINDINGS goes Active after one final codex round; implementation proceeds slice-by-slice
   175	- Decision: `docs/plan/OTP12_PERF_FINDINGS.md` flips **Draft → Active** after ONE final codex round, and implementation then proceeds regardless of whether that round returns a "converged" verdict — owner, 2026-07-13, verbatim: **"one more round with codex on the plan then just write the code and reviewloop slice by slice. that converges faster than plans with no ground truth to test."** Each code slice still goes through the codex review loop (D-2026-07-04-1, unchanged); what is retired is *plan-only* iteration as the gate on starting work. The plan's own Status line ("the flip to Active happens at codex convergence") is amended by this decision: the round happens, its accepted findings are fixed, and then code starts — a non-converged verdict is no longer a blocker, it is input to the first slice.
   176	- Why: rounds 2–4 each returned real findings, but they were increasingly findings about the *plan text* (falsifiability wording, thresholds, bar phrasing) rather than about reality, and the plan's central factual claim was settled not by review but by *measurement* — the same-OS rig, which refuted a claim four review rounds had left standing (`docs/bench/otp12-perf-2026-07-13/`; a wrong "P1 is code" claim was reported and retracted the same day). Ground truth comes from instrumented code and rigs, not from more prose; pf-1 exists precisely to generate it. Continuing to polish the plan has diminishing returns against the cost of not yet having a single measured counterfactual.
   177	- Supersedes: the "flip to Active at codex convergence" gate in `OTP12_PERF_FINDINGS.md`'s Status line (rewritten in place, same commit). Does NOT supersede D-2026-07-04-1 — every code slice is still codex-reviewed before the next begins.

exec
/bin/zsh -lc 'for f in docs/bench/otp12-win-2026-07-12/summary.csv docs/bench/otp12-win-2026-07-12/verdicts.csv docs/bench/otp12c-win-2026-07-13/summary.csv docs/bench/otp12c-win-2026-07-13/verdicts.csv docs/bench/otp2w-baseline-2026-07-10/summary.csv; do wc -l "$f"; nl -ba "$f"; done' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
      49 docs/bench/otp12-win-2026-07-12/summary.csv
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	mw_grpc_large,mac_init,1911,1914,1910,0.9,0,4
     3	mw_grpc_large,win_init,1931,1933,1924,1.1,0,4
     4	mw_grpc_mixed,mac_init,1829,1839,1810,4.4,0,4
     5	mw_grpc_mixed,win_init,1842,1845,1834,1.6,0,4
     6	mw_grpc_small,mac_init,2261,2248,2094,13.5,0,4
     7	mw_grpc_small,win_init,2227,2221,2096,11.4,0,4
     8	mw_tcp_large,mac_init,1914,1930,1897,5.2,0,4
     9	mw_tcp_large,win_init,1920,1923,1918,0.8,0,4
    10	mw_tcp_mixed,mac_init,1587,1589,1463,17.4,0,4
    11	mw_tcp_mixed,win_init,1502,1584,1486,24.2,0,4
    12	mw_tcp_small,mac_init,1922,1913,1884,2.2,0,4
    13	mw_tcp_small,win_init,1935,1947,1917,4.5,0,4
    14	pull_grpc_large,new,966,967,963,1.0,0,4
    15	pull_grpc_large,old,978,979,970,2.0,0,4
    16	pull_grpc_mixed,new,1244,1243,1217,4.4,0,4
    17	pull_grpc_mixed,old,1408,4576,1274,1015.6,0,4
    18	pull_grpc_small,new,1294,1292,1270,3.1,0,4
    19	pull_grpc_small,old,1525,1544,1504,7.8,0,4
    20	pull_tcp_large,new,959,962,955,2.0,0,4
    21	pull_tcp_large,old,964,964,956,1.9,0,4
    22	pull_tcp_mixed,new,1138,1147,1127,5.2,0,4
    23	pull_tcp_mixed,old,867,875,855,6.7,0,4
    24	pull_tcp_small,new,1237,1266,1223,12.0,0,4
    25	pull_tcp_small,old,1360,1359,1234,20.1,0,4
    26	push_grpc_large,new,1919,1921,1913,1.0,0,4
    27	push_grpc_large,old,1913,1914,1903,1.4,0,4
    28	push_grpc_mixed,new,2081,2108,1967,17.2,0,4
    29	push_grpc_mixed,old,2177,2174,2142,2.8,0,4
    30	push_grpc_small,new,2357,2325,2188,9.7,0,4
    31	push_grpc_small,old,2942,2901,2750,8.1,0,4
    32	push_tcp_large,new,1904,1906,1893,1.7,0,4
    33	push_tcp_large,old,1908,1913,1903,1.7,0,4
    34	push_tcp_mixed,new,1776,1830,1709,20.5,0,4
    35	push_tcp_mixed,old,1687,1699,1635,9.4,0,4
    36	push_tcp_small,new,2080,2075,2031,3.8,0,4
    37	push_tcp_small,old,1811,1816,1796,3.0,0,4
    38	wm_grpc_large,mac_init,964,963,960,0.6,0,4
    39	wm_grpc_large,win_init,993,992,985,1.4,0,4
    40	wm_grpc_mixed,mac_init,1246,1243,1208,5.3,0,4
    41	wm_grpc_mixed,win_init,1262,1264,1240,4.4,0,4
    42	wm_grpc_small,mac_init,1375,1379,1333,7.7,0,4
    43	wm_grpc_small,win_init,1326,1321,1302,2.1,0,4
    44	wm_tcp_large,mac_init,962,961,956,0.8,0,4
    45	wm_tcp_large,win_init,984,1594,961,258.7,0,4
    46	wm_tcp_mixed,mac_init,1127,1147,1122,8.2,0,4
    47	wm_tcp_mixed,win_init,911,911,897,3.3,0,4
    48	wm_tcp_small,mac_init,1253,1272,1216,12.5,0,4
    49	wm_tcp_small,win_init,1267,1274,1233,7.9,0,4
     121 docs/bench/otp12-win-2026-07-12/verdicts.csv
     1	comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
     2	pull_grpc_large,converge,new,old_session,966,978,0.988,1.10,PASS
     3	pull_grpc_large,converge,new,old_committed,966,1289,0.749,1.10,PASS
     4	pull_grpc_large,converge,new,combined,966,,,1.10,PASS
     5	pull_grpc_mixed,converge,new,old_session,1244,1408,0.884,1.10,PASS
     6	pull_grpc_mixed,converge,new,old_committed,1244,1408,0.884,1.10,PASS
     7	pull_grpc_mixed,converge,new,combined,1244,,,1.10,PASS
     8	pull_grpc_small,converge,new,old_session,1294,1525,0.849,1.10,PASS
     9	pull_grpc_small,converge,new,old_committed,1294,1462,0.885,1.10,PASS
    10	pull_grpc_small,converge,new,combined,1294,,,1.10,PASS
    11	pull_tcp_large,converge,new,old_session,959,964,0.995,1.10,PASS
    12	pull_tcp_large,converge,new,old_committed,959,1294,0.741,1.10,PASS
    13	pull_tcp_large,converge,new,combined,959,,,1.10,PASS
    14	pull_tcp_mixed,converge,new,old_session,1138,867,1.313,1.10,FAIL
    15	pull_tcp_mixed,converge,new,old_committed,1138,1284,0.886,1.10,PASS
    16	pull_tcp_mixed,converge,new,combined,1138,,,1.10,FAIL-SAME-SESSION
    17	pull_tcp_small,converge,new,old_session,1237,1360,0.910,1.10,PASS
    18	pull_tcp_small,converge,new,old_committed,1237,1280,0.966,1.10,PASS
    19	pull_tcp_small,converge,new,combined,1237,,,1.10,PASS
    20	push_grpc_large,converge,new,old_session,1919,1913,1.003,1.10,PASS
    21	push_grpc_large,converge,new,old_committed,1919,3065,0.626,1.10,PASS
    22	push_grpc_large,converge,new,combined,1919,,,1.10,PASS
    23	push_grpc_mixed,converge,new,old_session,2081,2177,0.956,1.10,PASS
    24	push_grpc_mixed,converge,new,old_committed,2081,2687,0.774,1.10,PASS
    25	push_grpc_mixed,converge,new,combined,2081,,,1.10,PASS
    26	push_grpc_small,converge,new,old_session,2357,2942,0.801,1.10,PASS
    27	push_grpc_small,converge,new,old_committed,2357,2822,0.835,1.10,PASS
    28	push_grpc_small,converge,new,combined,2357,,,1.10,PASS
    29	push_tcp_large,converge,new,old_session,1904,1908,0.998,1.10,PASS
    30	push_tcp_large,converge,new,old_committed,1904,3054,0.623,1.10,PASS
    31	push_tcp_large,converge,new,combined,1904,,,1.10,PASS
    32	push_tcp_mixed,converge,new,old_session,1776,1687,1.053,1.10,PASS
    33	push_tcp_mixed,converge,new,old_committed,1776,2288,0.776,1.10,PASS
    34	push_tcp_mixed,converge,new,combined,1776,,,1.10,PASS
    35	push_tcp_small,converge,new,old_session,2080,1811,1.149,1.10,FAIL
    36	push_tcp_small,converge,new,old_committed,2080,1868,1.113,1.10,FAIL
    37	push_tcp_small,converge,new,combined,2080,,,1.10,FAIL-BOTH
    38	mw_grpc_large,invariance,mac_init,win_init,1911,1931,1.010,1.10,PASS
    39	mw_grpc_large,converge,mac_init,old_session,1911,1913,0.999,1.10,PASS
    40	mw_grpc_large,converge,mac_init,old_committed,1911,3065,0.623,1.10,PASS
    41	mw_grpc_large,converge,win_init,old_session,1931,1913,1.009,1.10,PASS
    42	mw_grpc_large,converge,win_init,old_committed,1931,3065,0.630,1.10,PASS
    43	mw_grpc_large,cross,worst_arm,min_old_committed,1931,1289,1.498,1.10,FAIL
    44	mw_grpc_mixed,invariance,mac_init,win_init,1829,1842,1.007,1.10,PASS
    45	mw_grpc_mixed,converge,mac_init,old_session,1829,2177,0.840,1.10,PASS
    46	mw_grpc_mixed,converge,mac_init,old_committed,1829,2687,0.681,1.10,PASS
    47	mw_grpc_mixed,converge,win_init,old_session,1842,2177,0.846,1.10,PASS
    48	mw_grpc_mixed,converge,win_init,old_committed,1842,2687,0.686,1.10,PASS
    49	mw_grpc_mixed,cross,worst_arm,min_old_committed,1842,1408,1.308,1.10,FAIL
    50	mw_grpc_small,invariance,mac_init,win_init,2261,2227,1.015,1.10,PASS
    51	mw_grpc_small,converge,mac_init,old_session,2261,2942,0.769,1.10,PASS
    52	mw_grpc_small,converge,mac_init,old_committed,2261,2822,0.801,1.10,PASS
    53	mw_grpc_small,converge,win_init,old_session,2227,2942,0.757,1.10,PASS
    54	mw_grpc_small,converge,win_init,old_committed,2227,2822,0.789,1.10,PASS
    55	mw_grpc_small,cross,worst_arm,min_old_committed,2261,1462,1.547,1.10,FAIL
    56	mw_tcp_large,invariance,mac_init,win_init,1914,1920,1.003,1.10,PASS
    57	mw_tcp_large,converge,mac_init,old_session,1914,1908,1.003,1.10,PASS
    58	mw_tcp_large,converge,mac_init,old_committed,1914,3054,0.627,1.10,PASS
    59	mw_tcp_large,converge,win_init,old_session,1920,1908,1.006,1.10,PASS
    60	mw_tcp_large,converge,win_init,old_committed,1920,3054,0.629,1.10,PASS
    61	mw_tcp_large,cross,worst_arm,min_old_committed,1920,1294,1.484,1.10,FAIL
    62	mw_tcp_mixed,invariance,mac_init,win_init,1587,1502,1.057,1.10,PASS
    63	mw_tcp_mixed,converge,mac_init,old_session,1587,1687,0.941,1.10,PASS
    64	mw_tcp_mixed,converge,mac_init,old_committed,1587,2288,0.694,1.10,PASS
    65	mw_tcp_mixed,converge,win_init,old_session,1502,1687,0.890,1.10,PASS
    66	mw_tcp_mixed,converge,win_init,old_committed,1502,2288,0.656,1.10,PASS
    67	mw_tcp_mixed,cross,worst_arm,min_old_committed,1587,1284,1.236,1.10,FAIL
    68	mw_tcp_small,invariance,mac_init,win_init,1922,1935,1.007,1.10,PASS
    69	mw_tcp_small,converge,mac_init,old_session,1922,1811,1.061,1.10,PASS
    70	mw_tcp_small,converge,mac_init,old_committed,1922,1868,1.029,1.10,PASS
    71	mw_tcp_small,converge,win_init,old_session,1935,1811,1.068,1.10,PASS
    72	mw_tcp_small,converge,win_init,old_committed,1935,1868,1.036,1.10,PASS
    73	mw_tcp_small,cross,worst_arm,min_old_committed,1935,1280,1.512,1.10,FAIL
    74	wm_grpc_large,invariance,mac_init,win_init,964,993,1.030,1.10,PASS
    75	wm_grpc_large,converge,mac_init,old_session,964,978,0.986,1.10,PASS
    76	wm_grpc_large,converge,mac_init,old_committed,964,1289,0.748,1.10,PASS
    77	wm_grpc_large,converge,win_init,old_session,993,978,1.015,1.10,PASS
    78	wm_grpc_large,converge,win_init,old_committed,993,1289,0.770,1.10,PASS
    79	wm_grpc_large,cross,worst_arm,min_old_committed,993,1289,0.770,1.10,PASS
    80	wm_grpc_mixed,invariance,mac_init,win_init,1246,1262,1.013,1.10,PASS
    81	wm_grpc_mixed,converge,mac_init,old_session,1246,1408,0.885,1.10,PASS
    82	wm_grpc_mixed,converge,mac_init,old_committed,1246,1408,0.885,1.10,PASS
    83	wm_grpc_mixed,converge,win_init,old_session,1262,1408,0.896,1.10,PASS
    84	wm_grpc_mixed,converge,win_init,old_committed,1262,1408,0.896,1.10,PASS
    85	wm_grpc_mixed,cross,worst_arm,min_old_committed,1262,1408,0.896,1.10,PASS
    86	wm_grpc_small,invariance,mac_init,win_init,1375,1326,1.037,1.10,PASS
    87	wm_grpc_small,converge,mac_init,old_session,1375,1525,0.902,1.10,PASS
    88	wm_grpc_small,converge,mac_init,old_committed,1375,1462,0.940,1.10,PASS
    89	wm_grpc_small,converge,win_init,old_session,1326,1525,0.870,1.10,PASS
    90	wm_grpc_small,converge,win_init,old_committed,1326,1462,0.907,1.10,PASS
    91	wm_grpc_small,cross,worst_arm,min_old_committed,1375,1462,0.940,1.10,PASS
    92	wm_tcp_large,invariance,mac_init,win_init,962,984,1.023,1.10,PASS
    93	wm_tcp_large,converge,mac_init,old_session,962,964,0.998,1.10,PASS
    94	wm_tcp_large,converge,mac_init,old_committed,962,1294,0.743,1.10,PASS
    95	wm_tcp_large,converge,win_init,old_session,984,964,1.021,1.10,PASS
    96	wm_tcp_large,converge,win_init,old_committed,984,1294,0.760,1.10,PASS
    97	wm_tcp_large,cross,worst_arm,min_old_committed,984,1294,0.760,1.10,PASS
    98	wm_tcp_mixed,invariance,mac_init,win_init,1127,911,1.237,1.10,FAIL
    99	wm_tcp_mixed,converge,mac_init,old_session,1127,867,1.300,1.10,FAIL
   100	wm_tcp_mixed,converge,mac_init,old_committed,1127,1284,0.878,1.10,PASS
   101	wm_tcp_mixed,converge,win_init,old_session,911,867,1.051,1.10,PASS
   102	wm_tcp_mixed,converge,win_init,old_committed,911,1284,0.710,1.10,PASS
   103	wm_tcp_mixed,cross,worst_arm,min_old_committed,1127,1284,0.878,1.10,PASS
   104	wm_tcp_small,invariance,mac_init,win_init,1253,1267,1.011,1.10,PASS
   105	wm_tcp_small,converge,mac_init,old_session,1253,1360,0.921,1.10,PASS
   106	wm_tcp_small,converge,mac_init,old_committed,1253,1280,0.979,1.10,PASS
   107	wm_tcp_small,converge,win_init,old_session,1267,1360,0.932,1.10,PASS
   108	wm_tcp_small,converge,win_init,old_committed,1267,1280,0.990,1.10,PASS
   109	wm_tcp_small,cross,worst_arm,min_old_committed,1267,1280,0.990,1.10,PASS
   110	gap_tcp_large,cross-gap,old_push,old_pull,1908,964,1.979,,RECORDED
   111	gap_tcp_large,cross-gap,new_mw_worst,new_wm_worst,1920,984,1.951,,RECORDED
   112	gap_tcp_small,cross-gap,old_push,old_pull,1811,1360,1.332,,RECORDED
   113	gap_tcp_small,cross-gap,new_mw_worst,new_wm_worst,1935,1267,1.527,,RECORDED
   114	gap_tcp_mixed,cross-gap,old_push,old_pull,1687,867,1.946,,RECORDED
   115	gap_tcp_mixed,cross-gap,new_mw_worst,new_wm_worst,1587,1127,1.408,,RECORDED
   116	gap_grpc_large,cross-gap,old_push,old_pull,1913,978,1.956,,RECORDED
   117	gap_grpc_large,cross-gap,new_mw_worst,new_wm_worst,1931,993,1.945,,RECORDED
   118	gap_grpc_small,cross-gap,old_push,old_pull,2942,1525,1.929,,RECORDED
   119	gap_grpc_small,cross-gap,new_mw_worst,new_wm_worst,2261,1375,1.644,,RECORDED
   120	gap_grpc_mixed,cross-gap,old_push,old_pull,2177,1408,1.546,,RECORDED
   121	gap_grpc_mixed,cross-gap,new_mw_worst,new_wm_worst,1842,1262,1.460,,RECORDED
      49 docs/bench/otp12c-win-2026-07-13/summary.csv
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	mw_grpc_large,mac_init,1701,1699,1690,0.9,0,4
     3	mw_grpc_large,win_init,1750,1783,1741,8.6,0,4
     4	mw_grpc_mixed,mac_init,1794,1801,1783,2.7,0,4
     5	mw_grpc_mixed,win_init,1837,1839,1812,3.3,0,4
     6	mw_grpc_small,mac_init,1964,1970,1961,1.7,0,4
     7	mw_grpc_small,win_init,1981,1983,1952,3.5,0,4
     8	mw_tcp_large,mac_init,1702,1701,1695,0.6,1,5
     9	mw_tcp_large,win_init,1707,1713,1704,1.8,1,5
    10	mw_tcp_mixed,mac_init,1415,1480,1402,20.5,0,4
    11	mw_tcp_mixed,win_init,1477,1538,1450,20.8,0,4
    12	mw_tcp_small,mac_init,1781,1792,1748,6.3,0,4
    13	mw_tcp_small,win_init,1814,1818,1801,2.3,0,4
    14	pull_grpc_large,new,978,988,970,5.8,0,4
    15	pull_grpc_large,old,976,976,971,1.1,0,4
    16	pull_grpc_mixed,new,1366,1365,1322,6.4,0,4
    17	pull_grpc_mixed,old,1395,1396,1371,3.9,0,4
    18	pull_grpc_small,new,1479,1477,1459,2.3,0,4
    19	pull_grpc_small,old,1663,4783,1581,799.9,0,4
    20	pull_tcp_large,new,961,961,956,1.2,0,4
    21	pull_tcp_large,old,964,964,959,0.9,0,4
    22	pull_tcp_mixed,new,1192,1210,1176,9.0,0,4
    23	pull_tcp_mixed,old,956,963,948,4.5,0,4
    24	pull_tcp_small,new,1301,1298,1231,10.6,0,4
    25	pull_tcp_small,old,1480,1467,1372,12.1,0,4
    26	push_grpc_large,new,1700,1699,1686,1.5,0,4
    27	push_grpc_large,old,1696,1696,1685,1.4,0,4
    28	push_grpc_mixed,new,2102,2103,2030,7.3,1,5
    29	push_grpc_mixed,old,2106,2117,1811,35.1,1,5
    30	push_grpc_small,new,2264,2264,2239,2.2,0,4
    31	push_grpc_small,old,2656,2654,2504,11.8,0,4
    32	push_tcp_large,new,1700,1716,1694,4.4,0,4
    33	push_tcp_large,old,1707,1704,1689,1.5,0,4
    34	push_tcp_mixed,new,1703,1668,1458,24.2,0,4
    35	push_tcp_mixed,old,1491,1498,1459,6.4,0,4
    36	push_tcp_small,new,1975,1977,1831,16.3,0,4
    37	push_tcp_small,old,1644,1651,1620,4.8,0,4
    38	wm_grpc_large,mac_init,965,968,962,2.0,0,4
    39	wm_grpc_large,win_init,996,1000,988,3.1,0,4
    40	wm_grpc_mixed,mac_init,1393,1381,1302,10.4,0,4
    41	wm_grpc_mixed,win_init,1364,1379,1346,7.1,0,4
    42	wm_grpc_small,mac_init,1503,1541,1475,14.0,0,4
    43	wm_grpc_small,win_init,1521,1544,1497,9.3,0,4
    44	wm_tcp_large,mac_init,960,960,956,1.2,0,4
    45	wm_tcp_large,win_init,997,1017,991,9.3,0,4
    46	wm_tcp_mixed,mac_init,1221,1225,1192,6.4,0,4
    47	wm_tcp_mixed,win_init,939,948,920,8.4,0,4
    48	wm_tcp_small,mac_init,1330,1363,1319,11.8,1,5
    49	wm_tcp_small,win_init,1366,1374,1354,4.3,1,5
     121 docs/bench/otp12c-win-2026-07-13/verdicts.csv
     1	comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
     2	pull_grpc_large,converge,new,old_session,978,976,1.002,1.10,PASS
     3	pull_grpc_large,converge,new,old_committed,978,1289,0.759,1.10,PASS
     4	pull_grpc_large,converge,new,combined,978,,,1.10,PASS
     5	pull_grpc_mixed,converge,new,old_session,1366,1395,0.979,1.10,PASS
     6	pull_grpc_mixed,converge,new,old_committed,1366,1408,0.970,1.10,PASS
     7	pull_grpc_mixed,converge,new,combined,1366,,,1.10,PASS
     8	pull_grpc_small,converge,new,old_session,1479,1663,0.889,1.10,PASS
     9	pull_grpc_small,converge,new,old_committed,1479,1462,1.012,1.10,PASS
    10	pull_grpc_small,converge,new,combined,1479,,,1.10,PASS
    11	pull_tcp_large,converge,new,old_session,961,964,0.997,1.10,PASS
    12	pull_tcp_large,converge,new,old_committed,961,1294,0.743,1.10,PASS
    13	pull_tcp_large,converge,new,combined,961,,,1.10,PASS
    14	pull_tcp_mixed,converge,new,old_session,1192,956,1.247,1.10,FAIL
    15	pull_tcp_mixed,converge,new,old_committed,1192,1284,0.928,1.10,PASS
    16	pull_tcp_mixed,converge,new,combined,1192,,,1.10,FAIL-SAME-SESSION
    17	pull_tcp_small,converge,new,old_session,1301,1480,0.879,1.10,PASS
    18	pull_tcp_small,converge,new,old_committed,1301,1280,1.016,1.10,PASS
    19	pull_tcp_small,converge,new,combined,1301,,,1.10,PASS
    20	push_grpc_large,converge,new,old_session,1700,1696,1.002,1.10,PASS
    21	push_grpc_large,converge,new,old_committed,1700,3065,0.555,1.10,PASS
    22	push_grpc_large,converge,new,combined,1700,,,1.10,PASS
    23	push_grpc_mixed,converge,new,old_session,2102,2106,0.998,1.10,PASS
    24	push_grpc_mixed,converge,new,old_committed,2102,2687,0.782,1.10,PASS
    25	push_grpc_mixed,converge,new,combined,2102,,,1.10,PASS
    26	push_grpc_small,converge,new,old_session,2264,2656,0.852,1.10,PASS
    27	push_grpc_small,converge,new,old_committed,2264,2822,0.802,1.10,PASS
    28	push_grpc_small,converge,new,combined,2264,,,1.10,PASS
    29	push_tcp_large,converge,new,old_session,1700,1707,0.996,1.10,PASS
    30	push_tcp_large,converge,new,old_committed,1700,3054,0.557,1.10,PASS
    31	push_tcp_large,converge,new,combined,1700,,,1.10,PASS
    32	push_tcp_mixed,converge,new,old_session,1703,1491,1.142,1.10,FAIL
    33	push_tcp_mixed,converge,new,old_committed,1703,2288,0.744,1.10,PASS
    34	push_tcp_mixed,converge,new,combined,1703,,,1.10,FAIL-SAME-SESSION
    35	push_tcp_small,converge,new,old_session,1975,1644,1.201,1.10,FAIL
    36	push_tcp_small,converge,new,old_committed,1975,1868,1.057,1.10,PASS
    37	push_tcp_small,converge,new,combined,1975,,,1.10,FAIL-SAME-SESSION
    38	mw_grpc_large,invariance,mac_init,win_init,1701,1750,1.029,1.10,PASS
    39	mw_grpc_large,converge,mac_init,old_session,1701,1696,1.003,1.10,PASS
    40	mw_grpc_large,converge,mac_init,old_committed,1701,3065,0.555,1.10,PASS
    41	mw_grpc_large,converge,win_init,old_session,1750,1696,1.032,1.10,PASS
    42	mw_grpc_large,converge,win_init,old_committed,1750,3065,0.571,1.10,PASS
    43	mw_grpc_large,cross,worst_arm,min_old_committed,1750,1289,1.358,1.10,FAIL
    44	mw_grpc_mixed,invariance,mac_init,win_init,1794,1837,1.024,1.10,PASS
    45	mw_grpc_mixed,converge,mac_init,old_session,1794,2106,0.852,1.10,PASS
    46	mw_grpc_mixed,converge,mac_init,old_committed,1794,2687,0.668,1.10,PASS
    47	mw_grpc_mixed,converge,win_init,old_session,1837,2106,0.872,1.10,PASS
    48	mw_grpc_mixed,converge,win_init,old_committed,1837,2687,0.684,1.10,PASS
    49	mw_grpc_mixed,cross,worst_arm,min_old_committed,1837,1408,1.305,1.10,FAIL
    50	mw_grpc_small,invariance,mac_init,win_init,1964,1981,1.009,1.10,PASS
    51	mw_grpc_small,converge,mac_init,old_session,1964,2656,0.739,1.10,PASS
    52	mw_grpc_small,converge,mac_init,old_committed,1964,2822,0.696,1.10,PASS
    53	mw_grpc_small,converge,win_init,old_session,1981,2656,0.746,1.10,PASS
    54	mw_grpc_small,converge,win_init,old_committed,1981,2822,0.702,1.10,PASS
    55	mw_grpc_small,cross,worst_arm,min_old_committed,1981,1462,1.355,1.10,FAIL
    56	mw_tcp_large,invariance,mac_init,win_init,1702,1707,1.003,1.10,PASS
    57	mw_tcp_large,converge,mac_init,old_session,1702,1707,0.997,1.10,PASS
    58	mw_tcp_large,converge,mac_init,old_committed,1702,3054,0.557,1.10,PASS
    59	mw_tcp_large,converge,win_init,old_session,1707,1707,1.000,1.10,PASS
    60	mw_tcp_large,converge,win_init,old_committed,1707,3054,0.559,1.10,PASS
    61	mw_tcp_large,cross,worst_arm,min_old_committed,1707,1294,1.319,1.10,FAIL
    62	mw_tcp_mixed,invariance,mac_init,win_init,1415,1477,1.044,1.10,PASS
    63	mw_tcp_mixed,converge,mac_init,old_session,1415,1491,0.949,1.10,PASS
    64	mw_tcp_mixed,converge,mac_init,old_committed,1415,2288,0.618,1.10,PASS
    65	mw_tcp_mixed,converge,win_init,old_session,1477,1491,0.991,1.10,PASS
    66	mw_tcp_mixed,converge,win_init,old_committed,1477,2288,0.646,1.10,PASS
    67	mw_tcp_mixed,cross,worst_arm,min_old_committed,1477,1284,1.150,1.10,FAIL
    68	mw_tcp_small,invariance,mac_init,win_init,1781,1814,1.019,1.10,PASS
    69	mw_tcp_small,converge,mac_init,old_session,1781,1644,1.083,1.10,PASS
    70	mw_tcp_small,converge,mac_init,old_committed,1781,1868,0.953,1.10,PASS
    71	mw_tcp_small,converge,win_init,old_session,1814,1644,1.103,1.10,FAIL
    72	mw_tcp_small,converge,win_init,old_committed,1814,1868,0.971,1.10,PASS
    73	mw_tcp_small,cross,worst_arm,min_old_committed,1814,1280,1.417,1.10,FAIL
    74	wm_grpc_large,invariance,mac_init,win_init,965,996,1.032,1.10,PASS
    75	wm_grpc_large,converge,mac_init,old_session,965,976,0.989,1.10,PASS
    76	wm_grpc_large,converge,mac_init,old_committed,965,1289,0.749,1.10,PASS
    77	wm_grpc_large,converge,win_init,old_session,996,976,1.020,1.10,PASS
    78	wm_grpc_large,converge,win_init,old_committed,996,1289,0.773,1.10,PASS
    79	wm_grpc_large,cross,worst_arm,min_old_committed,996,1289,0.773,1.10,PASS
    80	wm_grpc_mixed,invariance,mac_init,win_init,1393,1364,1.021,1.10,PASS
    81	wm_grpc_mixed,converge,mac_init,old_session,1393,1395,0.999,1.10,PASS
    82	wm_grpc_mixed,converge,mac_init,old_committed,1393,1408,0.989,1.10,PASS
    83	wm_grpc_mixed,converge,win_init,old_session,1364,1395,0.978,1.10,PASS
    84	wm_grpc_mixed,converge,win_init,old_committed,1364,1408,0.969,1.10,PASS
    85	wm_grpc_mixed,cross,worst_arm,min_old_committed,1393,1408,0.989,1.10,PASS
    86	wm_grpc_small,invariance,mac_init,win_init,1503,1521,1.012,1.10,PASS
    87	wm_grpc_small,converge,mac_init,old_session,1503,1663,0.904,1.10,PASS
    88	wm_grpc_small,converge,mac_init,old_committed,1503,1462,1.028,1.10,PASS
    89	wm_grpc_small,converge,win_init,old_session,1521,1663,0.915,1.10,PASS
    90	wm_grpc_small,converge,win_init,old_committed,1521,1462,1.040,1.10,PASS
    91	wm_grpc_small,cross,worst_arm,min_old_committed,1521,1462,1.040,1.10,PASS
    92	wm_tcp_large,invariance,mac_init,win_init,960,997,1.039,1.10,PASS
    93	wm_tcp_large,converge,mac_init,old_session,960,964,0.996,1.10,PASS
    94	wm_tcp_large,converge,mac_init,old_committed,960,1294,0.742,1.10,PASS
    95	wm_tcp_large,converge,win_init,old_session,997,964,1.034,1.10,PASS
    96	wm_tcp_large,converge,win_init,old_committed,997,1294,0.770,1.10,PASS
    97	wm_tcp_large,cross,worst_arm,min_old_committed,997,1294,0.770,1.10,PASS
    98	wm_tcp_mixed,invariance,mac_init,win_init,1221,939,1.300,1.10,FAIL
    99	wm_tcp_mixed,converge,mac_init,old_session,1221,956,1.277,1.10,FAIL
   100	wm_tcp_mixed,converge,mac_init,old_committed,1221,1284,0.951,1.10,PASS
   101	wm_tcp_mixed,converge,win_init,old_session,939,956,0.982,1.10,PASS
   102	wm_tcp_mixed,converge,win_init,old_committed,939,1284,0.731,1.10,PASS
   103	wm_tcp_mixed,cross,worst_arm,min_old_committed,1221,1284,0.951,1.10,PASS
   104	wm_tcp_small,invariance,mac_init,win_init,1330,1366,1.027,1.10,PASS
   105	wm_tcp_small,converge,mac_init,old_session,1330,1480,0.899,1.10,PASS
   106	wm_tcp_small,converge,mac_init,old_committed,1330,1280,1.039,1.10,PASS
   107	wm_tcp_small,converge,win_init,old_session,1366,1480,0.923,1.10,PASS
   108	wm_tcp_small,converge,win_init,old_committed,1366,1280,1.067,1.10,PASS
   109	wm_tcp_small,cross,worst_arm,min_old_committed,1366,1280,1.067,1.10,PASS
   110	gap_tcp_large,cross-gap,old_push,old_pull,1707,964,1.771,,RECORDED
   111	gap_tcp_large,cross-gap,new_mw_worst,new_wm_worst,1707,997,1.712,,RECORDED
   112	gap_tcp_small,cross-gap,old_push,old_pull,1644,1480,1.111,,RECORDED
   113	gap_tcp_small,cross-gap,new_mw_worst,new_wm_worst,1814,1366,1.328,,RECORDED
   114	gap_tcp_mixed,cross-gap,old_push,old_pull,1491,956,1.560,,RECORDED
   115	gap_tcp_mixed,cross-gap,new_mw_worst,new_wm_worst,1477,1221,1.210,,RECORDED
   116	gap_grpc_large,cross-gap,old_push,old_pull,1696,976,1.738,,RECORDED
   117	gap_grpc_large,cross-gap,new_mw_worst,new_wm_worst,1750,996,1.757,,RECORDED
   118	gap_grpc_small,cross-gap,old_push,old_pull,2656,1663,1.597,,RECORDED
   119	gap_grpc_small,cross-gap,new_mw_worst,new_wm_worst,1981,1521,1.302,,RECORDED
   120	gap_grpc_mixed,cross-gap,old_push,old_pull,2106,1395,1.510,,RECORDED
   121	gap_grpc_mixed,cross-gap,new_mw_worst,new_wm_worst,1837,1393,1.319,,RECORDED
      13 docs/bench/otp2w-baseline-2026-07-10/summary.csv
     1	cell,median_ms,avg_ms,best_ms
     2	push_tcp_large,3054,3052,3041
     3	push_grpc_large,3065,3067,3058
     4	pull_tcp_large,1294,1294,1288
     5	pull_grpc_large,1289,1289,1288
     6	push_tcp_small,1868,1869,1841
     7	push_grpc_small,2822,2862,2787
     8	pull_tcp_small,1280,1315,1259
     9	pull_grpc_small,1462,1477,1414
    10	push_tcp_mixed,2288,2299,2266
    11	push_grpc_mixed,2687,2675,2551
    12	pull_tcp_mixed,1284,1304,1272
    13	pull_grpc_mixed,1408,1409,1378

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '70,190p' && nl -ba scripts/bench_otp12_win.sh | sed -n '630,770p' && nl -ba scripts/bench_otp12_win.sh | sed -n '760,835p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    70	set -euo pipefail
    71	
    72	SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    73	REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
    74	
    75	# Defaults match the box's 2026-07-12 reality: hostname netwatch-01,
    76	# IP 10.1.10.177 (the previously recorded 10.1.10.173 went stale —
    77	# DHCP; machines.md).
    78	WIN_SSH=${WIN_SSH:-michael@netwatch-01}
    79	WIN_HOST=${WIN_HOST:-10.1.10.177}
    80	WIN_TEST=${WIN_TEST:-'D:\blit-test'}
    81	WIN_DRIVE=${WIN_DRIVE:-D}
    82	MAC_HOST=${MAC_HOST:?set MAC_HOST to the Mac 10GbE IP that the Windows-initiated arms dial}
    83	PORT=${PORT:-9031}
    84	RUNS=${RUNS:-4}
    85	PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
    86	CELLS=${CELLS:-}
    87	MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
    88	# The Mac module root IS the fixture workdir (design F6): both
    89	# initiators of a Mac->Win cell read the same physical inodes. NOT
    90	# overridable (codex otp-12b F6) — an override could point the two
    91	# initiators at different trees or devices.
    92	MAC_MODULE_ROOT="$MAC_WORK"
    93	
    94	OLD_SHA=${OLD_SHA_WIN:-0f922de}
    95	NEW_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
    96	NEW_BLIT=${NEW_BLIT:-$REPO_ROOT/target/release/blit}
    97	MAC_DAEMON=${MAC_DAEMON:-$REPO_ROOT/target/release/blit-daemon}
    98	OLD_BLIT=${OLD_BLIT:-$MAC_WORK/bins/blit-$OLD_SHA}
    99	WIN_BINS="$WIN_TEST\\bins"
   100	OLD_WIN_DAEMON="$WIN_BINS\\$OLD_SHA\\blit-daemon.exe"
   101	NEW_WIN_DAEMON="$WIN_BINS\\$NEW_SHA\\blit-daemon.exe"
   102	ACTIVE_WIN_DAEMON="$WIN_BINS\\active\\blit-daemon.exe"
   103	WIN_BLIT="$WIN_BINS\\$NEW_SHA\\blit.exe"
   104	# Fixed committed reference (pre-registered, D2) — no override.
   105	BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2w-baseline-2026-07-10/summary.csv"
   106	
   107	OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_win_$(date +%Y%m%dT%H%M%S)}
   108	mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs" "$MAC_WORK"
   109	
   110	WIN_MODULE="$WIN_TEST\\bench-module"
   111	WIN_REMOTE="$WIN_HOST:$PORT:/bench/"
   112	MAC_REMOTE="$MAC_HOST:$PORT:/bench/"
   113	
   114	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
   115	die() { log "FATAL: $*"; exit 1; }
   116	SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
   117	wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
   118	now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
   119	
   120	# --- Self-timed durability (destination-OS-keyed, never verb-keyed) ----
   121	flush_win_ms() {   # Windows volume flush, self-timed; prints ms or NA
   122	    # Sentinel-framed and error-terminating (codex otp-12b F7): a
   123	    # failed flush or garbage output must never read as a plausible
   124	    # number — NA makes the caller VOID the run per the D2 rule.
   125	    local v
   126	    v=$(wssh "\$ErrorActionPreference = 'Stop'; \$sw = [Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache $WIN_DRIVE; \$sw.Stop(); \"F:\$([int]\$sw.Elapsed.TotalMilliseconds):F\"" 2>/dev/null \
   127	        | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p' | head -1)
   128	    echo "${v:-NA}"
   129	}
   130	fsync_tree_ms() {   # macOS per-file fsync walk; prints its own elapsed ms
   131	    python3 - "$1" <<'PYEOF'
   132	import os, sys, time
   133	t = time.monotonic()
   134	for root, dirs, files in os.walk(sys.argv[1]):
   135	    for name in files:
   136	        fd = os.open(os.path.join(root, name), os.O_RDONLY)
   137	        os.fsync(fd)
   138	        os.close(fd)
   139	print(int((time.monotonic() - t) * 1000))
   140	PYEOF
   141	}
   142	
   143	want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }
   144	
   145	# --- Provenance + manifest (otp-12a lessons: +sha form, fail closed) ---
   146	sha256_local() {
   147	    local h
   148	    h=$(shasum -a 256 "$1" | cut -d' ' -f1) || die "sha256 failed for $1"
   149	    [[ ${#h} -eq 64 ]] || die "sha256 produced '$h' for $1"
   150	    echo "$h"
   151	}
   152	sha256_win() {
   153	    local h
   154	    h=$(wssh "(Get-FileHash -Algorithm SHA256 '$1').Hash" | tr -cd '0-9A-Fa-f' | tr 'A-F' 'a-f') \
   155	        || die "remote sha256 failed for $1"
   156	    [[ ${#h} -eq 64 ]] || die "remote sha256 produced '$h' for $1"
   157	    echo "$h"
   158	}
   159	win_embeds() {   # $1 = exe path, $2 = sha; exit 0 iff '+sha' present
   160	    wssh "if (Select-String -Path '$1' -SimpleMatch -Quiet -Pattern '+$2') { 'yes' } else { exit 1 }" >/dev/null
   161	}
   162	
   163	preflight() {
   164	    [[ "$RUNS" == 4 || "$RUNS" == 8 ]] \
   165	        || die "RUNS must be 4 (standard) or 8 (the D2 escalation) — got '$RUNS'"
   166	    [[ -x "$NEW_BLIT" ]] || die "missing $NEW_BLIT (cargo build --release first)"
   167	    [[ -x "$MAC_DAEMON" ]] || die "missing $MAC_DAEMON (the Mac daemon serves block 2)"
   168	    [[ -x "$OLD_BLIT" ]] || die "old client not staged at $OLD_BLIT (detached worktree rebuild at $OLD_SHA)"
   169	    command -v python3 >/dev/null || die "python3 required"
   170	    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
   171	    sudo -n /usr/sbin/purge || die "need the NOPASSWD purge sudoers rule"
   172	    wssh "if (-not (Test-Path '$OLD_WIN_DAEMON')) { exit 1 }" \
   173	        || die "old daemon not staged at $OLD_WIN_DAEMON (copy the detached-build exe aside BEFORE moving the checkout)"
   174	    wssh "if (-not (Test-Path '$NEW_WIN_DAEMON')) { exit 1 }" \
   175	        || die "new daemon not staged at $NEW_WIN_DAEMON (native build at $NEW_SHA)"
   176	    wssh "if (-not (Test-Path '$WIN_BLIT')) { exit 1 }" \
   177	        || die "new Windows client not staged at $WIN_BLIT"
   178	    # Provenance: +sha form (bare shas match cargo build-dir paths).
   179	    LC_ALL=C grep -qa "+$NEW_SHA" "$NEW_BLIT" \
   180	        || die "$NEW_BLIT does not embed +$NEW_SHA — rebuild at the run commit"
   181	    LC_ALL=C grep -qa "+$NEW_SHA" "$MAC_DAEMON" \
   182	        || die "$MAC_DAEMON does not embed +$NEW_SHA — rebuild at the run commit"
   183	    win_embeds "$NEW_WIN_DAEMON" "$NEW_SHA" \
   184	        || die "$NEW_WIN_DAEMON does not embed +$NEW_SHA — restage the native build"
   185	    win_embeds "$WIN_BLIT" "$NEW_SHA" \
   186	        || die "$WIN_BLIT does not embed +$NEW_SHA — restage the native build"
   187	    win_embeds "$OLD_WIN_DAEMON" "$OLD_SHA" \
   188	        || die "$OLD_WIN_DAEMON does not embed +$OLD_SHA — the staged old daemon is not the pinned pair"
   189	    if LC_ALL=C grep -qa "+$OLD_SHA" "$OLD_BLIT"; then
   190	        :
   630	    log "smoke ok: old pair"
   631	    win_ensure new
   632	    "$NEW_BLIT" copy "$MAC_WORK/smoke_src" "${WIN_REMOTE}push_${SESSION_TAG}_smoke_new/" --yes \
   633	        > /dev/null 2> "$OUT_DIR/blit-logs/smoke_new.err" || die "new-pair smoke FAILED (BUILD_MISMATCH here = staged daemon is not $NEW_SHA)"
   634	    log "smoke ok: new pair"
   635	    win_client_run "smoke_winmac" "s0" "${MAC_REMOTE}smoke_src/" "$WIN_MODULE\\pull_${SESSION_TAG}_smoke\\" ""
   636	    [[ "$T_RC" == 0 ]] || die "win->mac smoke FAILED (rc=$T_RC — macOS application firewall blocking the Mac daemon? see blit-logs/smoke_winmac_s0.err)"
   637	    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_${SESSION_TAG}_smoke' -ErrorAction SilentlyContinue" || true
   638	    log "smoke ok: win->mac (mac daemon reachable; firewall clear)"
   639	}
   640	
   641	# --- Verdicts (design D2 as amended; F3; F4 + discriminator recorded) ----
   642	compute_verdicts() {
   643	    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
   644	import csv, sys
   645	runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
   646	rows = list(csv.DictReader(open(runs_p)))
   647	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   648	base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}
   649	
   650	by_arm, voided = {}, {}
   651	for r in rows:
   652	    key = (r["cell"], r["arm"])
   653	    if r["valid"] == "yes":
   654	        by_arm.setdefault(key, []).append(int(r["ms"]))
   655	    else:
   656	        voided[key] = voided.get(key, 0) + 1
   657	
   658	def median(v):
   659	    v = sorted(v); n = len(v)
   660	    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
   661	
   662	def complete(cell):
   663	    if cell not in meta or meta[cell]["complete"] != "yes":
   664	        return False
   665	    arms = [a for (c, a) in by_arm if c == cell]
   666	    return len(arms) == 2
   667	
   668	def bar(new, ref):   # new <= ref * 1.10, integer-exact
   669	    return 10 * new <= 11 * ref
   670	
   671	out = open(verdicts_p, "w")
   672	out.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
   673	
   674	with open(summary_p, "w") as f:
   675	    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
   676	    for (cell, arm) in sorted(by_arm):
   677	        if not complete(cell):
   678	            continue
   679	        v = by_arm[(cell, arm)]
   680	        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   681	        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
   682	                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")
   683	
   684	def m(cell, arm):
   685	    return median(by_arm[(cell, arm)]) if (cell, arm) in by_arm else None
   686	
   687	# Block 1: converge-up, both references (12a logic verbatim).
   688	b1_cells = sorted(c for c in meta if c.split("_")[0] in ("push", "pull"))
   689	for cell in b1_cells:
   690	    if not complete(cell):
   691	        out.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
   692	        continue
   693	    new_m, old_m = m(cell, "new"), m(cell, "old")
   694	    if cell not in base:
   695	        sys.exit(f"FATAL: no committed reference row for {cell}")
   696	    ref_m = base[cell]
   697	    p1, p2 = bar(new_m, old_m), bar(new_m, ref_m)
   698	    out.write(f"{cell},converge,new,old_session,{new_m},{old_m},{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
   699	    out.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
   700	    combined = ("PASS" if p1 and p2 else "FAIL-REFERENCE-DRIFT" if p1
   701	                else "FAIL-SAME-SESSION" if p2 else "FAIL-BOTH")
   702	    out.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
   703	
   704	# Block 2: invariance + per-arm converge (F3) + cross rows (F4) +
   705	# discriminator gap rows (D-2026-07-12-1; recorded, not adjudicated).
   706	b2_cells = sorted(c for c in meta if c.split("_")[0] in ("mw", "wm"))
   707	for cell in b2_cells:
   708	    if not complete(cell):
   709	        out.write(f"{cell},invariance,mac_init,win_init,,,,1.10,INCOMPLETE\n")
   710	        continue
   711	    a, b = m(cell, "mac_init"), m(cell, "win_init")
   712	    hi, lo = max(a, b), min(a, b)
   713	    inv = bar(hi, lo)   # max/min <= 1.10
   714	    out.write(f"{cell},invariance,mac_init,win_init,{a},{b},{hi/lo:.3f},1.10,{'PASS' if inv else 'FAIL'}\n")
   715	    # F3: each arm independently meets the direction's converge bars.
   716	    # Committed references are MANDATORY (fail closed, codex otp-12b
   717	    # F8); the same-session reference requires the block-1 counterpart
   718	    # COMPLETE (codex otp-12b F4 — a partial median never referees),
   719	    # else the row says so in registered vocabulary.
   720	    d, carrier, fixture = cell.split("_")
   721	    verb = "push" if d == "mw" else "pull"
   722	    b1 = f"{verb}_{carrier}_{fixture}"
   723	    if b1 not in base:
   724	        sys.exit(f"FATAL: no committed reference row for {b1} (needed by {cell})")
   725	    ref_m = base[b1]
   726	    old_sess = m(b1, "old") if complete(b1) else None
   727	    for armname, val in (("mac_init", a), ("win_init", b)):
   728	        if old_sess is not None:
   729	            out.write(f"{cell},converge,{armname},old_session,{val},{old_sess},{val/old_sess:.3f},1.10,{'PASS' if bar(val, old_sess) else 'FAIL'}\n")
   730	        else:
   731	            out.write(f"{cell},converge,{armname},old_session,{val},,,1.10,NO-SAME-SESSION-REF\n")
   732	        out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
   733	    # F4 cross: each direction vs min of the two committed old
   734	    # directions for this fixture x carrier (mandatory, fail closed).
   735	    p_ref = base.get(f"push_{carrier}_{fixture}")
   736	    l_ref = base.get(f"pull_{carrier}_{fixture}")
   737	    if p_ref is None or l_ref is None:
   738	        sys.exit(f"FATAL: committed push/pull reference missing for {carrier}_{fixture}")
   739	    cross_ref = min(p_ref, l_ref)
   740	    worst = max(a, b)
   741	    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
   742	
   743	# Discriminator gap rows (D-2026-07-12-1; recorded, never adjudicated):
   744	# emitted only when ALL FOUR contributing cells are complete (codex
   745	# otp-12b F4). Row operands are labeled exactly (codex otp-12b F12).
   746	for carrier in ("tcp", "grpc"):
   747	    for fixture in ("large", "small", "mixed"):
   748	        four = [f"push_{carrier}_{fixture}", f"pull_{carrier}_{fixture}",
   749	                f"mw_{carrier}_{fixture}", f"wm_{carrier}_{fixture}"]
   750	        if not all(complete(c) for c in four):
   751	            continue
   752	        po, lo_ = m(four[0], "old"), m(four[1], "old")
   753	        mw_w = max(m(four[2], "mac_init"), m(four[2], "win_init"))
   754	        wm_w = max(m(four[3], "mac_init"), m(four[3], "win_init"))
   755	        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push,old_pull,"
   756	                  f"{po},{lo_},{po/lo_:.3f},,RECORDED\n")
   757	        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw_worst,new_wm_worst,"
   758	                  f"{mw_w},{wm_w},{mw_w/wm_w:.3f},,RECORDED\n")
   759	out.close()
   760	PYEOF
   761	}
   762	
   763	# --- Matrix ----------------------------------------------------------------
   764	main() {
   765	    preflight
   766	    write_manifest
   767	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   768	        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   769	        exit 0
   770	    fi
   760	PYEOF
   761	}
   762	
   763	# --- Matrix ----------------------------------------------------------------
   764	main() {
   765	    preflight
   766	    write_manifest
   767	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   768	        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   769	        exit 0
   770	    fi
   771	    log "session $SESSION_TAG  old=$OLD_SHA new=$NEW_SHA  win: $WIN_HOST  mac: $MAC_HOST"
   772	
   773	    gen_fixtures
   774	    setup_host
   775	    mac_daemon_start
   776	    smoke
   777	    stage_pull_sources
   778	
   779	    local w flag carrier
   780	    # BLOCK 1 — converge-up (old vs new, Mac-initiated).
   781	    for w in large small mixed; do
   782	        for carrier in tcp grpc; do
   783	            [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
   784	            CUR_W="$w"; CUR_FLAG="$flag"
   785	            if want_cell "push_${carrier}_${w}"; then
   786	                run_pair_loop "push_${carrier}_${w}" old new b1_push_old b1_push_new
   787	            fi
   788	            if want_cell "pull_${carrier}_${w}"; then
   789	                run_pair_loop "pull_${carrier}_${w}" old new b1_pull_old b1_pull_new
   790	            fi
   791	        done
   792	    done
   793	
   794	    # BLOCK 2 — invariance (mac_init vs win_init, new pair only).
   795	    win_ensure new
   796	    for w in large small mixed; do
   797	        for carrier in tcp grpc; do
   798	            [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
   799	            CUR_W="$w"; CUR_FLAG="$flag"
   800	            if want_cell "mw_${carrier}_${w}"; then
   801	                run_pair_loop "mw_${carrier}_${w}" mac_init win_init b2_mw_mac b2_mw_win
   802	            fi
   803	            if want_cell "wm_${carrier}_${w}"; then
   804	                run_pair_loop "wm_${carrier}_${w}" mac_init win_init b2_wm_mac b2_wm_win
   805	            fi
   806	        done
   807	    done
   808	
   809	    if [[ -n "$CELLS" ]]; then
   810	        local c
   811	        for c in ${CELLS//,/ }; do
   812	            # Header excluded — CELLS=cell must not match "cell,…"
   813	            # (codex otp-12b F10).
   814	            tail -n +2 "$META" | grep -q "^$c," \
   815	                || die "CELLS entry '$c' matched no comparison — nothing was measured for it"
   816	        done
   817	    fi
   818	
   819	    win_daemon_stop
   820	    mac_daemon_stop
   821	    compute_verdicts
   822	
   823	    log ""
   824	    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
   825	    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
   826	    log ""
   827	    log "=== VERDICTS (D2 both-references; invariance; F4 cross + gap rows) ==="
   828	    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
   829	    log "runs: $CSV"
   830	}
   831	
   832	SESSION_TAG=$(date +%H%M%S).$$
   833	trap on_exit EXIT
   834	T_MS=0; T_RC=0
   835	main "$@"

exec
/bin/zsh -lc "rg -n \"D2|RUNS|spread|median|escalat|25%|1\\.10\" docs/plan/OTP12_ACCEPTANCE_RUN.md && nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '50,180p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
118:| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
119:| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
133:rigs; pre-registered, no randomness, codex design F5) with `RUNS=4` per
154:### D2 — verdict arithmetic (what the evidence computes; the owner declares)
156:All statistics per the recorded baselines: integer ms; median of 4, even
157:count = floor of the mean of the middle two; per-cell spread
163:position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
166:a median over fewer than RUNS valid runs.
169:  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
170:  interleaved old arm AND the committed 2026-07-10 baseline median for
182:  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
185:  `max(delegated, direct)/min ≤ 1.10`.
187:  each unified direction's median vs
188:  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
202:bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
205:text defined the trigger but not which session governs): the RUNS=8
206:escalation session's medians govern the escalated comparison's combined
207:outcome — more data where noise or a straddle made RUNS=4 undecidable is
208:the escalation's entire purpose. The RUNS=4 rows stay committed and
300:  nonzero exit voids the interleave pair per the D2 valid-run rule — a
306:(`valid` = the PAIR's fate under the D2 valid-run rule — an
310:`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
311:(medians over valid runs only — the D2 valid-run rule)
325:carries the registered D2 set
329:incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
422:  Defender at its normal state). D2's discriminator computation is the
444:  `min(old_push, old_pull) × 1.10` bar only by a discriminator-attributed
    50	findings, `docs/bench/otp2w-baseline-2026-07-10/README.md` §Timing-overhead
    51	correction).
    52	
    53	## What otp-12 must produce (plan anchors)
    54	
    55	1. **Invariance matrix** (criterion 1): per data direction × workload
    56	   (large / 10k-small / mixed), wall time initiating from end A vs end B —
    57	   push-verb vs pull-verb — within run noise (±10%). Committed as evidence.
    58	2. **Converge-up matrix** (criterion 2 / codex F4): every unified cell ≤ the
    59	   better of that cell's two old directions + noise (±10%), against the
    60	   recorded old-path baselines, confirmed by interleaved same-session
    61	   old-vs-new A/B (the otp-2 README's standing prescription for this rig
    62	   class).
    63	3. **Delegated cells** (owner rig designation, 2026-07-10, STATE Blocked):
    64	   remote↔remote on the Windows box + skippy — the delegated trigger must
    65	   not cost wall time vs the same session driven directly.
    66	
    67	## Current state (verified at HEAD `ce36da3`)
    68	
    69	Load-bearing facts, with evidence:
    70	
    71	- One `copy` verb drives everything; a remote endpoint is `host:/module/path`
    72	  or `host:port:/module/path`, default port 9031
    73	  (`crates/blit-core/src/remote/endpoint.rs:28,64-91,165-195`).
    74	- Carrier switch: default = TCP data plane (responder binds an EPHEMERAL
    75	  listener, initiator dials — `transfer_session/data_plane.rs:129,204`;
    76	  grant present ⇒ TCP, `transfer_session/mod.rs:805`); `--force-grpc`
    77	  forces the in-stream carrier (`blit-cli/src/cli.rs:317-319`), and rides
    78	  the delegated spec too (`proto/blit.proto:408`,
    79	  `blit-daemon/src/service/delegated_pull.rs:334`).
    80	- Remote↔remote is delegated-only (D-2026-07-11-1): `blit copy A:/m/p B:/m/q`
    81	  always calls `DelegatedPull` on the **destination** daemon, which initiates
    82	  the one session against the source daemon in the DESTINATION role
    83	  (`blit-app/src/transfers/remote.rs:462-484`,
    84	  `delegated_pull.rs:312-327,352`). There is no push-shaped delegated form.
    85	  The RPC carries trigger + progress only (no-payload proof recorded at
    86	  otp-10: `cli_data_plane_outbound_bytes == 0`).
    87	- Delegation gate: destination daemon config `[delegation]
    88	  allow_delegated_pull = true` + `allowed_source_hosts` allowlist
    89	  (`blit-daemon/src/runtime.rs:139-145`); per-module `delegation_allowed`.
    90	- Same-build handshake (D-2026-07-05-2): first frame both directions; exact
    91	  `build_id` + `contract_version` equality or `BuildMismatch` refusal
    92	  (`transfer_session/mod.rs:660-701`). Dirty builds mint distinct ids
    93	  (`blit-core/build.rs:28-97`) — **all arms must be clean-tree builds; arms
    94	  swap BOTH ends together (matched pairs)**.
    95	- Old-arm binaries route the OLD drivers: `e757dcc` (zoey pair, staged in
    96	  `blit-temp/` — `.agents/machines.md`) and `0f922de` (Windows pair, checkout
    97	  detached there) both PREDATE the verb cutover (`0fbc966`), so their verbs
    98	  still call `Push`/`PullSync` — they are genuine old-path arms. Verified by
    99	  ancestry + `git ls-tree` (old drivers present at both shas).
   100	- July skippy binaries (`/mnt/generic-pool/video/blit-bin/`) are REV4-era:
   101	  unknown commit, no `Transfer` RPC, no handshake — **unusable for any
   102	  otp-12 arm**; skippy gets fresh staging (D6).
   103	- Baselines on record: `docs/bench/otp2-baseline-2026-07-10/` (zoey,
   104	  per-direction only — hardware-asymmetric endpoints, D-2026-07-05-1
   105	  corollary) and `docs/bench/otp2w-baseline-2026-07-10/` (Mac↔Windows, the
   106	  owner-designated cross-direction rig).
   107	- Flags a harness touches that changed since the old scripts: none — `copy`,
   108	  `--yes`, `--force-grpc` are name-stable; `--diagnostics-counter-file` is a
   109	  global flag preceding the subcommand.
   110	- SizeMtime safe-skip delta (STATE open question) cannot affect these cells:
   111	  every timed run writes into a fresh, never-seen destination, so no
   112	  same-size/dest-newer candidates exist in any arm.
   113	
   114	## Rigs and what each anchors
   115	
   116	| rig | endpoints | anchors | why scoped so |
   117	|-----|-----------|---------|---------------|
   118	| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
   119	| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
   120	| **D** | Windows daemon ↔ skippy daemon (TrueNAS, x86_64), Mac as delegating CLI | delegated-vs-direct parity (trigger invariance) | owner-designated delegated rig; no old baseline exists on this pair |
   121	
   122	Contingency: skippy is available for Mac↔Linux cells "if needed" (owner) —
   123	used only if zoey is unavailable (it was under maintenance 2026-07-11); such
   124	a substitution records fresh baselines and is per-direction only.
   125	
   126	## Design decisions
   127	
   128	### D1 — matched-pair interleaved A/B (build identity is the axis)
   129	
   130	Each comparison interleaves arms in the deterministic counterbalanced
   131	order `A,B,B,A,A,B,B,A` (ABBA per pair-of-pairs — each arm leads half the
   132	pairs, so arm never confounds with within-pair position on the stateful
   133	rigs; pre-registered, no randomness, codex design F5) with `RUNS=4` per
   134	arm (8 timed runs per comparison). A = `old` (rig Z/W converge-up) or
   135	`delegated` (rig D). Interleaving is the verdict method, not a nicety:
   136	zoey's tiered write path never fully stops being stateful (otp-2 README
   137	§Run-to-run stability) and interleaving holds Defender state equal across
   138	arms on Windows (otp-2w README §Readings). Arm swap = stop one daemon
   139	pair, start the other (PID-scoped, stale-refusal preserved), always
   140	outside the timed window. Old arms exist only where an old baseline exists
   141	(rigs Z and W); invariance and delegated arms are new-build only — the old
   142	path is known non-invariant (the plan's founding defect) and has no
   143	delegated baseline.
   144	
   145	Build discipline: one clean commit per arm. New arm = the run commit (same
   146	sha, all hosts). Old arms = the pinned baseline shas (`e757dcc` zoey,
   147	`0f922de` Windows). Old-arm Mac clients are rebuilt at the pinned sha in a
   148	detached worktree (`git worktree add --detach` — the otp-11a precedent) and
   149	stashed at `~/blit-bench-work/bins/blit-<sha>`. The handshake enforces new-
   150	arm pair identity at the first frame; old arms predate it, so old-arm
   151	provenance rests on the staging record (`.agents/machines.md`) plus a
   152	sha256 manifest recorded in the evidence (Known gaps).
   153	
   154	### D2 — verdict arithmetic (what the evidence computes; the owner declares)
   155	
   156	All statistics per the recorded baselines: integer ms; median of 4, even
   157	count = floor of the mean of the middle two; per-cell spread
   158	`(max−min)/min` recorded.
   159	
   160	**Valid-run rule (codex design F7)**: a run with a nonzero blit exit OR an
   161	undrained pre-run window VOIDS its whole interleave pair (both arms at
   162	that counterbalance position); the pair is re-run — appended at the same
   163	position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
   164	pair attempts per comparison. At the cap the cell is recorded
   165	`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
   166	a median over fewer than RUNS valid runs.
   167	
   168	- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
   169	  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
   170	  interleaved old arm AND the committed 2026-07-10 baseline median for
   171	  that cell (codex design F2: the fixed pre-cutover bar must not be
   172	  loosened by a slower old rerun). A cell passing same-session but
   173	  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
   174	  gets one pre-registered fresh-session re-run; a persisting drift stands
   175	  as a recorded failure for the otp-13 walk. **Every unified arm of a
   176	  data direction — both initiators on rig W, both blocks — must meet
   177	  these bars independently** (codex design F3: the invariance ratio is an
   178	  additional constraint, never a substitute ceiling — otherwise
   179	  tolerances compound to 1.21×).
   180	- **Invariance (rig W, hard bar — the owner's sentence)**: per fixture ×

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '180,220p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   180	- **Invariance (rig W, hard bar — the owner's sentence)**: per fixture ×
   181	  carrier × data direction, arm A (Mac-initiated) vs arm B
   182	  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
   183	  rows; grpc rows are recorded, same bar, labeled secondary.
   184	- **Delegated parity (rig D, hard bar)**: per fixture × direction,
   185	  `max(delegated, direct)/min ≤ 1.10`.
   186	- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
   187	  each unified direction's median vs
   188	  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
   189	  while passing per-direction converge-up AND invariance, the evidence
   190	  additionally computes the **platform-residue discriminator** the otp-2w
   191	  README pre-registered: compare the old arm's direction gap
   192	  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
   193	  session. Gap unchanged ⇒ the residue exists identically without blit's
   194	  old choreography and lands on the platform write path (NTFS/Defender vs
   195	  APFS — the plan's Non-goals: different hardware need not perform
   196	  identically); gap closed ⇒ the code was the cost and the bar is met. The
   197	  README records BOTH computations per cell; a discriminator-attributed
   198	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
   199	  the otp-13 walk reviews the recorded numbers.
   200	
   201	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
   202	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
   203	interleaved in a fresh session; both sessions are committed.
   204	**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
   205	text defined the trigger but not which session governs): the RUNS=8
   206	escalation session's medians govern the escalated comparison's combined
   207	outcome — more data where noise or a straddle made RUNS=4 undecidable is
   208	the escalation's entire purpose. The RUNS=4 rows stay committed and
   209	visible; the otp-13 walk sees both sessions.**
   210	
   211	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
   212	
   213	For a FIXED data direction the two initiators are:
   214	
   215	- **Mac→Windows**: arm A = Mac client pushes
   216	  (`blit copy $MAC_WORK/src_<w> $WIN_HOST:9031:/bench/<fresh>/ --yes`);
   217	  arm B = Windows client pulls
   218	  (`blit.exe copy $MAC_HOST:9031:/bench/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   219	- **Windows→Mac**: arm A = Mac client pulls (staged
   220	  `pull_src_<w>/src_<w>/` source, the otp-2w pattern); arm B = Windows

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '460,630p' && rg -n \"TcpStream|set_nodelay|nodelay|TCP_MAXSEG|MSS|connect\\(|accept\\(|data plane|SourceSockets|DestinationSockets\" crates/blit-core/src/remote/transfer_session crates/blit-core/src/remote/transfer -g '*.rs' | head -250" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   460	        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
   461	        [[ "$got" == "$want" ]] || die "pull_src_$w still wrong after staging ($got/$want)"
   462	        log "  staged pull_src_$w ($got files)"
   463	    done
   464	}
   465	
   466	# --- Timed runs -----------------------------------------------------------
   467	CSV="$OUT_DIR/runs.csv"
   468	echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
   469	META="$OUT_DIR/meta.csv"
   470	echo "cell,pairs_attempted,complete" > "$META"
   471	
   472	RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
   473	
   474	# Mac-initiated runs (block 1 both arms; block 2 mac_init arms).
   475	mac_push_run() {   # blit_bin cell rid dest_remote src [flags...]
   476	    local blit="$1" cell="$2" rid="$3" dest="$4" src="$5"; shift 5
   477	    local start end rc=0
   478	    drop_caches "${cell}-$rid"
   479	    start=$(now_ms)
   480	    "$blit" copy "$src" "${dest}push_${SESSION_TAG}_${cell}_${rid}/" --yes "$@" \
   481	        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
   482	    end=$(now_ms)
   483	    if [[ "$dest" == "$WIN_REMOTE" ]]; then
   484	        RUN_FLUSH=$(flush_win_ms)
   485	        wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\push_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
   486	    else
   487	        RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
   488	        rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
   489	    fi
   490	    RUN_VALID=yes
   491	    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
   492	    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc
   493	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   494	}
   495	mac_pull_run() {   # blit_bin cell rid remote_src [flags...]
   496	    local blit="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
   497	    local start end rc=0
   498	    local dst="$MAC_WORK/dst_pull_${SESSION_TAG}_${cell}_${rid}"
   499	    mkdir -p "$dst"
   500	    drop_caches "${cell}-$rid"
   501	    start=$(now_ms)
   502	    "$blit" copy "$rsrc" "$dst" --yes "$@" \
   503	        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
   504	    end=$(now_ms)
   505	    RUN_FLUSH=$(fsync_tree_ms "$dst")
   506	    rm -rf "$dst"
   507	    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc; RUN_VALID=yes
   508	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   509	}
   510	# Windows-initiated runs (block 2 win_init arms): the transfer window is
   511	# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
   512	win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
   513	    # Sentinel-framed (codex otp-12b F7): anything but a clean
   514	    # "R:<ms>,<rc>:R" — pwsh noise, a crash, a negative exit — parses
   515	    # to T_RC=99 and voids the run; nothing can masquerade as a time.
   516	    local cell="$1" rid="$2" src="$3" dst="$4" flags="${5:-}"
   517	    local out
   518	    # ${rc} braces are load-bearing: PowerShell parses bare `$rc:R` as
   519	    # a SCOPE-qualified variable (like $env:PATH), so the sentinel
   520	    # never printed and every win-initiated run read rc=99 (found live
   521	    # at the first win->mac smoke, 2026-07-12).
   522	    out=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$src' '$dst' --yes $flags > \$null 2> '$WIN_TEST\\client-err.log'; \$rc = \$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\${rc}:R\"" \
   523	        | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)
   524	    if [[ "$out" == *,* ]]; then T_MS=${out%%,*}; T_RC=${out##*,}; else T_MS=0; T_RC=99; fi
   525	    if [[ "$T_RC" != 0 ]]; then
   526	        wssh "Get-Content '$WIN_TEST\\client-err.log' -ErrorAction SilentlyContinue | Select-Object -First 20" \
   527	            > "$OUT_DIR/blit-logs/${cell}_${rid}.err" 2>&1 || true
   528	    fi
   529	}
   530	win_pull_run() {   # cell rid remote_src(from mac) [flag]; dest = win module
   531	    local cell="$1" rid="$2" rsrc="$3" flag="${4:-}"
   532	    drop_caches "${cell}-$rid"
   533	    win_client_run "$cell" "$rid" "$rsrc" "$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}" "$flag"
   534	    RUN_FLUSH=$(flush_win_ms)
   535	    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
   536	    RUN_VALID=yes
   537	    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
   538	    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC
   539	    [[ "$T_RC" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   540	}
   541	win_push_run() {   # cell rid src(win local path) [flag]; dest = mac module
   542	    local cell="$1" rid="$2" src="$3" flag="${4:-}"
   543	    drop_caches "${cell}-$rid"
   544	    win_client_run "$cell" "$rid" "$src" "${MAC_REMOTE}push_${SESSION_TAG}_${cell}_${rid}/" "$flag"
   545	    RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
   546	    rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
   547	    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC; RUN_VALID=yes
   548	    [[ "$T_RC" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   549	}
   550	
   551	# One interleaved comparison; ABBA; pair-void; INCOMPLETE at the cap.
   552	# run_one <cell> <armA> <armB> <fnA...>|<fnB...> dispatch happens via
   553	# small wrappers below to keep bash 3.2-simple.
   554	run_pair_loop() {   # cell armA armB runA_fn runB_fn (fns take: cell rid)
   555	    local cell="$1" armA="$2" armB="$3" fnA="$4" fnB="$5"
   556	    local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
   557	    log "=== $cell ($armA vs $armB, ABBA, $RUNS pairs) ==="
   558	    while (( valid < RUNS && attempts < max_attempts )); do
   559	        attempts=$(( attempts + 1 ))
   560	        local order pair_valid=yes arm fn rid rowA="" rowB=""
   561	        if (( slot % 2 )); then order="A B"; else order="B A"; fi
   562	        for arm in $order; do
   563	            local aname bld init
   564	            if [[ "$arm" == A ]]; then fn="$fnA"; aname="$armA"; else fn="$fnB"; aname="$armB"; fi
   565	            # The arm is part of every rid — and therefore every
   566	            # destination path — so the two arms of a slot can never
   567	            # collide on leftover data if a sweep fails (codex otp-12b
   568	            # F3; the zoey harness always had this).
   569	            rid="${aname}_s${slot}a${attempts}"
   570	            "$fn" "$cell" "$rid"
   571	            [[ "$RUN_VALID" == yes ]] || pair_valid=no
   572	            case "$aname" in
   573	                old) bld="$OLD_SHA"; init=mac;;
   574	                new|mac_init) bld="$NEW_SHA"; init=mac;;
   575	                win_init) bld="$NEW_SHA"; init=win;;
   576	            esac
   577	            local row="$cell,$aname,$bld,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
   578	            if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
   579	            log "  $cell/$aname slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
   580	        done
   581	        echo "$rowA,$pair_valid" >> "$CSV"
   582	        echo "$rowB,$pair_valid" >> "$CSV"
   583	        if [[ "$pair_valid" == yes ]]; then
   584	            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   585	        else
   586	            log "  $cell: pair at slot $slot VOIDED — re-running the slot"
   587	        fi
   588	    done
   589	    if (( valid < RUNS )); then
   590	        echo "$cell,$attempts,no" >> "$META"
   591	        log "  $cell INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
   592	    else
   593	        echo "$cell,$attempts,yes" >> "$META"
   594	    fi
   595	}
   596	
   597	# Block-1 arm wrappers (Mac-initiated; daemon arm follows the run arm).
   598	CUR_W=""; CUR_FLAG=""
   599	b1_push_old() { win_ensure old; mac_push_run "$OLD_BLIT" "$1" "$2" "$WIN_REMOTE" "$MAC_WORK/src_$CUR_W" $CUR_FLAG; }
   600	b1_push_new() { win_ensure new; mac_push_run "$NEW_BLIT" "$1" "$2" "$WIN_REMOTE" "$MAC_WORK/src_$CUR_W" $CUR_FLAG; }
   601	b1_pull_old() { win_ensure old; mac_pull_run "$OLD_BLIT" "$1" "$2" "${WIN_REMOTE}pull_src_$CUR_W/src_$CUR_W/" $CUR_FLAG; }
   602	b1_pull_new() { win_ensure new; mac_pull_run "$NEW_BLIT" "$1" "$2" "${WIN_REMOTE}pull_src_$CUR_W/src_$CUR_W/" $CUR_FLAG; }
   603	# Block-2 arm wrappers (new pair; both daemons stay up). Both arms of a
   604	# pair do IDENTICAL work (codex otp-12b F5): no-trailing-slash sources
   605	# everywhere AND a destination CONTAINER precreated OUTSIDE the timed
   606	# window on every arm — each transfer lands the same one-level-nested
   607	# `container/src_<w>` tree, and no arm pays an in-window container
   608	# mkdir the other does not. (Block 1 keeps the otp-2w shapes verbatim
   609	# for baseline comparability.)
   610	b2_mw_mac() {
   611	    wssh "New-Item -ItemType Directory -Force -Path '$WIN_MODULE\\push_${SESSION_TAG}_${1}_${2}' | Out-Null"
   612	    mac_push_run "$NEW_BLIT" "$1" "$2" "$WIN_REMOTE" "$MAC_WORK/src_$CUR_W" $CUR_FLAG
   613	}
   614	b2_mw_win() {
   615	    wssh "New-Item -ItemType Directory -Force -Path '$WIN_MODULE\\pull_${SESSION_TAG}_${1}_${2}' | Out-Null"
   616	    win_pull_run "$1" "$2" "${MAC_REMOTE}src_$CUR_W" "$CUR_FLAG"
   617	}
   618	b2_wm_mac() { mac_pull_run "$NEW_BLIT" "$1" "$2" "${WIN_REMOTE}pull_src_$CUR_W/src_$CUR_W" $CUR_FLAG; }
   619	b2_wm_win() {
   620	    mkdir -p "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${1}_${2}"
   621	    win_push_run "$1" "$2" "$WIN_MODULE\\pull_src_$CUR_W\\src_$CUR_W" "$CUR_FLAG"
   622	}
   623	
   624	smoke() {   # arm smoke transfers (untimed): old pair, new pair, win->mac
   625	    mkdir -p "$MAC_WORK/smoke_src"
   626	    echo "otp12b-smoke" > "$MAC_WORK/smoke_src/probe.txt"
   627	    win_ensure old
   628	    "$OLD_BLIT" copy "$MAC_WORK/smoke_src" "${WIN_REMOTE}push_${SESSION_TAG}_smoke_old/" --yes \
   629	        > /dev/null 2> "$OUT_DIR/blit-logs/smoke_old.err" || die "old-pair smoke FAILED"
   630	    log "smoke ok: old pair"
rg: crates/blit-core/src/remote/transfer_session: No such file or directory (os error 2)
crates/blit-core/src/remote/transfer/resume_diff.rs:4://! block; the TCP data plane (`DataPlaneSink`) emits a binary `BLOCK`
crates/blit-core/src/remote/transfer/tcp_info.rs:29:pub fn sample_stream(stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
crates/blit-core/src/remote/transfer/tcp_info.rs:35:    // return. `fd` is borrowed from a live `TcpStream` for the duration
crates/blit-core/src/remote/transfer/tcp_info.rs:60:pub fn sample_stream(_stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
crates/blit-core/src/remote/transfer/tcp_info.rs:76:            async { tokio::net::TcpStream::connect(addr).await.unwrap() },
crates/blit-core/src/remote/transfer/tcp_info.rs:77:            async { listener.accept().await.unwrap() },
crates/blit-core/src/remote/transfer/socket.rs:16://! regress to an unbounded `TcpStream::connect`.
crates/blit-core/src/remote/transfer/socket.rs:24:use tokio::net::TcpStream;
crates/blit-core/src/remote/transfer/socket.rs:29:/// daemon called `listener.accept().await` with no timeout — a peer
crates/blit-core/src/remote/transfer/socket.rs:84:pub fn configure_data_socket(stream: &TcpStream, tcp_buffer_size: Option<usize>) -> io::Result<()> {
crates/blit-core/src/remote/transfer/socket.rs:86:    socket.set_tcp_nodelay(true)?;
crates/blit-core/src/remote/transfer/socket.rs:111:/// TCP data plane: a firewalled or black-holed data port — the daemon
crates/blit-core/src/remote/transfer/socket.rs:127:) -> eyre::Result<TcpStream> {
crates/blit-core/src/remote/transfer/socket.rs:146:) -> eyre::Result<TcpStream> {
crates/blit-core/src/remote/transfer/socket.rs:147:    let mut stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await {
crates/blit-core/src/remote/transfer/socket.rs:148:        Ok(connected) => connected.with_context(|| format!("connecting data plane {addr}"))?,
crates/blit-core/src/remote/transfer/socket.rs:186:    async fn loopback_pair() -> (TcpStream, TcpStream) {
crates/blit-core/src/remote/transfer/socket.rs:189:        let (client, accepted) = tokio::join!(TcpStream::connect(addr), listener.accept());
crates/blit-core/src/remote/transfer/socket.rs:194:    /// The full policy lands on the socket: nodelay and keepalive read
crates/blit-core/src/remote/transfer/socket.rs:199:    async fn applies_nodelay_keepalive_and_buffers() {
crates/blit-core/src/remote/transfer/socket.rs:206:            sock.tcp_nodelay().expect("read nodelay"),
crates/blit-core/src/remote/transfer/socket.rs:280:            listener.accept(),
crates/blit-core/src/remote/transfer/socket.rs:286:            SockRef::from(&stream).tcp_nodelay().expect("read nodelay"),
crates/blit-core/src/remote/transfer/socket.rs:320:            listener.accept(),
crates/blit-core/src/remote/transfer/socket.rs:373:    /// `None` = kernel-default buffers: nodelay/keepalive still land,
crates/blit-core/src/remote/transfer/socket.rs:384:        assert!(sock.tcp_nodelay().expect("read nodelay"));
crates/blit-core/src/remote/transfer/tar_safety.rs:8://!     (TCP data plane on the pull-client side and local-local sink)
crates/blit-core/src/remote/transfer/progress.rs:121:    /// sink it hands the data plane shares the atomic stored
crates/blit-core/src/remote/transfer/progress.rs:123:    /// the same value the data plane is incrementing.
crates/blit-core/src/remote/transfer/progress.rs:129:    /// the data plane after each chunk write. `Relaxed`
crates/blit-core/src/remote/transfer/progress.rs:400:/// the `ByteProgressSink` pattern so the data plane can carry it the
crates/blit-core/src/remote/transfer/stall_guard.rs:24://!   daemon-side pull data plane is a SENDER (daemon writes bytes to
crates/blit-core/src/remote/transfer/stall_guard.rs:128:/// audit-h3b: the daemon-side pull data plane writes bytes to the puller.
crates/blit-core/src/remote/transfer/session_client.rs:18://! and pull (otp-5b) default to the TCP data plane; the in-stream
crates/blit-core/src/remote/transfer/session_client.rs:54:    /// Force the in-stream byte carrier instead of the TCP data plane
crates/blit-core/src/remote/transfer/session_client.rs:55:    /// (otp-4b). Default `false` = the responder grants a data plane and
crates/blit-core/src/remote/transfer/session_client.rs:132:        // otp-4b: default to the TCP data plane; the responder grants it
crates/blit-core/src/remote/transfer/session_client.rs:173:        // The initiator dials the data plane on the same host it reached
crates/blit-core/src/remote/transfer/session_client.rs:210:    /// Force the in-stream byte carrier instead of the TCP data plane
crates/blit-core/src/remote/transfer/session_client.rs:274:/// otp-5b: the default carrier is the TCP data plane — the SOURCE
crates/blit-core/src/remote/transfer/session_client.rs:278:/// fallback (diagnostics / unreachable data plane).
crates/blit-core/src/remote/transfer/session_client.rs:307:        // otp-5b: default to the TCP data plane; the SOURCE responder
crates/blit-core/src/remote/transfer/session_client.rs:335:        // The initiator dials the data plane on the same host it reached
crates/blit-core/src/remote/transfer/session_client.rs:453:    let channel = tokio::time::timeout(Duration::from_secs(30), conn.connect())
crates/blit-core/src/remote/transfer/data_plane.rs:4:use tokio::net::TcpStream;
crates/blit-core/src/remote/transfer/data_plane.rs:59:    stream: StallGuardWriter<TcpStream>,
crates/blit-core/src/remote/transfer/data_plane.rs:88:        stream: TcpStream,
crates/blit-core/src/remote/transfer/data_plane.rs:98:    /// Connect to a data plane endpoint with buffer pooling.
crates/blit-core/src/remote/transfer/data_plane.rs:100:    pub async fn connect(
crates/blit-core/src/remote/transfer/data_plane.rs:127:    /// attaches `LiveProbe` telemetry to the push data plane; the
crates/blit-core/src/remote/transfer/data_plane.rs:151:            .context("dialing push data plane")?;
crates/blit-core/src/remote/transfer/data_plane.rs:166:        stream: TcpStream,
crates/blit-core/src/remote/transfer/data_plane.rs:246:            .context("flushing data plane stream")
crates/blit-core/src/remote/transfer/data_plane.rs:303:        // inline so push and pull data plane records carry the same
crates/blit-core/src/remote/transfer/data_plane.rs:757:        .context("reading from data plane stream")?;
crates/blit-core/src/remote/transfer/data_plane.rs:785:    /// the copy equals the bytes the data plane reported writing.
crates/blit-core/src/remote/transfer/data_plane.rs:886:    use tokio::net::{TcpListener, TcpStream};
crates/blit-core/src/remote/transfer/data_plane.rs:895:            let (mut sock, _) = listener.accept().await.unwrap();
crates/blit-core/src/remote/transfer/data_plane.rs:900:        let client = TcpStream::connect(addr).await.unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:711:    /// of the previous generic "data plane pipeline closed
crates/blit-core/src/remote/transfer/pipeline.rs:1045:            // execute_receive_pipeline takes &mut TcpStream. Use a real
crates/blit-core/src/remote/transfer/pipeline.rs:1053:                tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept(),);
crates/blit-core/src/remote/transfer/sink.rs:4://! Implementations handle the actual write: local filesystem, TCP data plane, etc.
crates/blit-core/src/remote/transfer/sink.rs:158:    /// `write_file_stream` reports every chunk the data plane
crates/blit-core/src/remote/transfer/sink.rs:749:// DataPlaneSink — TCP data plane writer
crates/blit-core/src/remote/transfer/sink.rs:752:/// Writes payloads to a remote daemon via the TCP data plane binary protocol.

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/socket.rs | sed -n '75,175p' && nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '1,270p' && nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '740,900p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    75	///   the value is a connect-time snapshot of
    76	///   [`TransferDial::tcp_buffer_bytes`](crate::dial::TransferDial::tcp_buffer_bytes)
    77	///   where a dial is in scope (epoch-0 sockets therefore run kernel
    78	///   defaults; resize-ADD sockets get the ramped size), and `None`
    79	///   where none is (the pull client and the daemon push receiver hold
    80	///   no dial).
    81	///
    82	/// Errors only if `TCP_NODELAY` cannot be set (or the fd/socket
    83	/// handle is unusable, which the same call surfaces).
    84	pub fn configure_data_socket(stream: &TcpStream, tcp_buffer_size: Option<usize>) -> io::Result<()> {
    85	    let socket = SockRef::from(stream);
    86	    socket.set_tcp_nodelay(true)?;
    87	    // `set_tcp_keepalive` also flips SO_KEEPALIVE on, so this is the
    88	    // whole keepalive story in one call.
    89	    let keepalive = TcpKeepalive::new()
    90	        .with_time(TCP_KEEPALIVE_IDLE)
    91	        .with_interval(TCP_KEEPALIVE_INTERVAL)
    92	        .with_retries(TCP_KEEPALIVE_RETRIES);
    93	    if let Err(e) = socket.set_tcp_keepalive(&keepalive) {
    94	        log::warn!("set TCP keepalive on data-plane socket: {}", e);
    95	    }
    96	    if let Some(size) = tcp_buffer_size {
    97	        if let Err(e) = socket.set_send_buffer_size(size) {
    98	            log::warn!("set TCP send buffer to {} bytes: {}", size, e);
    99	        }
   100	        if let Err(e) = socket.set_recv_buffer_size(size) {
   101	            log::warn!("set TCP recv buffer to {} bytes: {}", size, e);
   102	        }
   103	    }
   104	    Ok(())
   105	}
   106	
   107	/// design-3: dial a data-plane endpoint with the shared bounds — the
   108	/// client-side mirror of the daemon's bounded accept. Connect is
   109	/// bounded by [`DATA_PLANE_ACCEPT_TIMEOUT`] (the audit-2 wave bounded
   110	/// every control-plane connect at the same 30 s but never reached the
   111	/// TCP data plane: a firewalled or black-holed data port — the daemon
   112	/// advertises a fresh ephemeral port per transfer, and asymmetric
   113	/// firewalls that pass the control port but block ephemerals are
   114	/// common — hung for the kernel SYN timeout, 60–127 s, with no
   115	/// message). The handshake-token write is bounded by
   116	/// [`DATA_PLANE_TOKEN_TIMEOUT`], mirroring the acceptor's bounded
   117	/// token read. Applies [`configure_data_socket`] in between.
   118	///
   119	/// On timeout the error chain carries an `io::ErrorKind::TimedOut`
   120	/// source so `remote::retry::is_retryable` classifies it as a
   121	/// transient transport failure (`--retry` re-dials instead of giving
   122	/// up on a deterministic-looking error).
   123	pub async fn dial_data_plane(
   124	    addr: &str,
   125	    handshake: &[u8],
   126	    tcp_buffer_size: Option<usize>,
   127	) -> eyre::Result<TcpStream> {
   128	    dial_data_plane_with_timeouts(
   129	        addr,
   130	        handshake,
   131	        tcp_buffer_size,
   132	        DATA_PLANE_ACCEPT_TIMEOUT,
   133	        DATA_PLANE_TOKEN_TIMEOUT,
   134	    )
   135	    .await
   136	}
   137	
   138	/// Timeout-parameterized core of [`dial_data_plane`], so tests can pin
   139	/// the bounded-failure shape without waiting out the production 30 s.
   140	async fn dial_data_plane_with_timeouts(
   141	    addr: &str,
   142	    handshake: &[u8],
   143	    tcp_buffer_size: Option<usize>,
   144	    connect_timeout: Duration,
   145	    token_timeout: Duration,
   146	) -> eyre::Result<TcpStream> {
   147	    let mut stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await {
   148	        Ok(connected) => connected.with_context(|| format!("connecting data plane {addr}"))?,
   149	        Err(_) => {
   150	            return Err(eyre::Report::new(io::Error::new(
   151	                io::ErrorKind::TimedOut,
   152	                format!("connect did not complete within {connect_timeout:?}"),
   153	            ))
   154	            .wrap_err(format!(
   155	                "data-plane connect to {addr} timed out after {connect_timeout:?} — the \
   156	                 port is likely unreachable (the daemon advertises a fresh ephemeral \
   157	                 data port per transfer; a firewall that passes the control port but \
   158	                 blocks ephemeral ports produces exactly this failure)"
   159	            )));
   160	        }
   161	    };
   162	    configure_data_socket(&stream, tcp_buffer_size).context("setting TCP_NODELAY")?;
   163	    match tokio::time::timeout(token_timeout, stream.write_all(handshake)).await {
   164	        Ok(written) => {
   165	            written.with_context(|| format!("writing data-plane handshake token to {addr}"))?
   166	        }
   167	        Err(_) => {
   168	            return Err(eyre::Report::new(io::Error::new(
   169	                io::ErrorKind::TimedOut,
   170	                format!("handshake write did not complete within {token_timeout:?}"),
   171	            ))
   172	            .wrap_err(format!(
   173	                "data-plane handshake to {addr} stalled for {token_timeout:?} — the peer \
   174	                 accepted the connection but is not reading"
   175	            )));
     1	use eyre::{bail, Context, Result};
     2	use futures::StreamExt;
     3	use tokio::io::{AsyncReadExt, AsyncWriteExt};
     4	use tokio::net::TcpStream;
     5	
     6	use crate::buffer::BufferPool;
     7	use crate::generated::FileHeader;
     8	
     9	use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
    10	use super::progress::{NoProbe, Probe};
    11	use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
    12	use crate::remote::transfer::source::TransferSource;
    13	use std::sync::Arc;
    14	
    15	pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
    16	pub const DATA_PLANE_RECORD_FILE: u8 = 0;
    17	pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
    18	pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
    19	pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
    20	pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
    21	
    22	/// ue-r2-2: length of the per-epoch resize credential a data socket
    23	/// echoes after the one-time token when resize was negotiated
    24	/// (`DataTransferNegotiation.epoch0_sub_token` for the initial
    25	/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).
    26	pub const SUB_TOKEN_LEN: usize = 16;
    27	
    28	/// Generate one 16-byte resize sub-token. Same fallible-RNG posture
    29	/// as the daemon's one-time token (audit-3b): a missing system RNG is
    30	/// an error, never a weaker credential.
    31	pub fn generate_sub_token() -> eyre::Result<Vec<u8>> {
    32	    use rand::{rngs::SysRng, TryRng};
    33	    let mut buf = vec![0u8; SUB_TOKEN_LEN];
    34	    SysRng
    35	        .try_fill_bytes(&mut buf)
    36	        .map_err(|err| eyre::eyre!("system RNG unavailable: {err}"))?;
    37	    Ok(buf)
    38	}
    39	
    40	/// A single data-plane TCP stream and its send loop.
    41	///
    42	/// Generic over a [`Probe`] so the byte-copy hot path can carry
    43	/// per-stream telemetry under adaptive mode at **zero cost** when the
    44	/// probe is [`NoProbe`] (the default): the instrumented branches are
    45	/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
    46	/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
    47	/// bare type and get the `NoProbe` default; the adaptive controller
    48	/// constructs `DataPlaneSession<LiveProbe>` via
    49	/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
    50	///
    51	/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
    52	/// reader (TCP backpressure from a slow / wedged peer) trips after
    53	/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
    54	/// of pinning the worker for OS-level TCP retransmit exhaustion
    55	/// (15+ minutes). All existing `self.stream.write_all/.flush` call
    56	/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
    57	/// so no per-site change was needed.
    58	pub struct DataPlaneSession<P: Probe = NoProbe> {
    59	    stream: StallGuardWriter<TcpStream>,
    60	    pool: Arc<BufferPool>,
    61	    trace: bool,
    62	    chunk_bytes: usize,
    63	    payload_prefetch: usize,
    64	    bytes_sent: u64,
    65	    probe: P,
    66	}
    67	
    68	macro_rules! trace_client {
    69	    ($session:expr, $($arg:tt)*) => {
    70	        if $session.trace {
    71	            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
    72	        }
    73	    };
    74	}
    75	
    76	impl DataPlaneSession<NoProbe> {
    77	    /// Create a session from an existing stream with buffer pooling.
    78	    ///
    79	    /// Produces the un-instrumented `NoProbe` variant — the default for
    80	    /// every non-adaptive caller. audit-h3b: the stream is wrapped in
    81	    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
    82	    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
    83	    /// observable write progress instead of pinning the worker for
    84	    /// OS-level TCP retransmit exhaustion. The production call sites
    85	    /// (`daemon/service/pull.rs`, `daemon/service/pull_sync.rs`, and the
    86	    /// resume path) inherit the guard without code changes.
    87	    pub async fn from_stream(
    88	        stream: TcpStream,
    89	        trace: bool,
    90	        chunk_bytes: usize,
    91	        payload_prefetch: usize,
    92	        pool: Arc<BufferPool>,
    93	    ) -> Self {
    94	        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
    95	            .await
    96	    }
    97	
    98	    /// Connect to a data plane endpoint with buffer pooling.
    99	    #[allow(clippy::too_many_arguments)]
   100	    pub async fn connect(
   101	        host: &str,
   102	        port: u32,
   103	        token: &[u8],
   104	        chunk_bytes: usize,
   105	        payload_prefetch: usize,
   106	        trace: bool,
   107	        tcp_buffer_size: Option<usize>,
   108	        pool: Arc<BufferPool>,
   109	    ) -> Result<Self> {
   110	        Self::connect_with_probe(
   111	            host,
   112	            port,
   113	            token,
   114	            chunk_bytes,
   115	            payload_prefetch,
   116	            trace,
   117	            tcp_buffer_size,
   118	            pool,
   119	            NoProbe,
   120	        )
   121	        .await
   122	    }
   123	}
   124	
   125	impl<P: Probe> DataPlaneSession<P> {
   126	    /// `connect` with an explicit probe (ue-r2-1e: the dial tuner
   127	    /// attaches `LiveProbe` telemetry to the push data plane; the
   128	    /// probe-free path monomorphizes to `NoProbe` and reads no clock).
   129	    #[allow(clippy::too_many_arguments)]
   130	    pub async fn connect_with_probe(
   131	        host: &str,
   132	        port: u32,
   133	        token: &[u8],
   134	        chunk_bytes: usize,
   135	        payload_prefetch: usize,
   136	        trace: bool,
   137	        tcp_buffer_size: Option<usize>,
   138	        pool: Arc<BufferPool>,
   139	        probe: P,
   140	    ) -> Result<Self> {
   141	        let addr = format!("{}:{}", host, port);
   142	        if trace {
   143	            eprintln!("[data-plane-client] connecting to {}", addr);
   144	        }
   145	        // design-3: bounded dial (connect + w1-2 socket policy +
   146	        // negotiation-token write) via the shared data-plane helper —
   147	        // one owner for every client-side data-plane dial, both
   148	        // directions.
   149	        let stream = super::socket::dial_data_plane(&addr, token, tcp_buffer_size)
   150	            .await
   151	            .context("dialing push data plane")?;
   152	
   153	        Ok(
   154	            Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, probe)
   155	                .await,
   156	        )
   157	    }
   158	}
   159	
   160	impl<P: Probe> DataPlaneSession<P> {
   161	    /// Create a session carrying an arbitrary [`Probe`]. The generic
   162	    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
   163	    /// the adaptive controller calls this with a `LiveProbe` to enable
   164	    /// per-stream telemetry.
   165	    pub async fn from_stream_with_probe(
   166	        stream: TcpStream,
   167	        trace: bool,
   168	        chunk_bytes: usize,
   169	        payload_prefetch: usize,
   170	        pool: Arc<BufferPool>,
   171	        probe: P,
   172	    ) -> Self {
   173	        let payload_prefetch = payload_prefetch.max(1);
   174	        let chunk_bytes = chunk_bytes.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
   175	        Self {
   176	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
   177	            pool,
   178	            trace,
   179	            chunk_bytes,
   180	            payload_prefetch,
   181	            bytes_sent: 0,
   182	            probe,
   183	        }
   184	    }
   185	
   186	    pub async fn send_payloads(
   187	        &mut self,
   188	        source: Arc<dyn TransferSource>,
   189	        payloads: Vec<TransferPayload>,
   190	    ) -> Result<()> {
   191	        self.send_payloads_with_progress(source, payloads, None)
   192	            .await
   193	    }
   194	
   195	    pub async fn send_payloads_with_progress(
   196	        &mut self,
   197	        source: Arc<dyn TransferSource>,
   198	        payloads: Vec<TransferPayload>,
   199	        progress: Option<&super::progress::RemoteTransferProgress>,
   200	    ) -> Result<()> {
   201	        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
   202	        while let Some(prepared) = stream.next().await {
   203	            match prepared? {
   204	                PreparedPayload::File(header) => {
   205	                    if let Err(err) = self.send_file(source.clone(), &header).await {
   206	                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
   207	                    }
   208	                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
   209	                    if let Some(progress) = progress {
   210	                        progress.report_payload(0, header.size);
   211	                        progress.report_file_complete(header.relative_path.clone());
   212	                    }
   213	                }
   214	                PreparedPayload::TarShard { headers, data } => {
   215	                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
   216	                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
   217	                        return Err(err.wrap_err("sending tar shard"));
   218	                    }
   219	                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
   220	                    if let Some(progress) = progress {
   221	                        for header in &headers {
   222	                            progress.report_payload(0, header.size);
   223	                            progress.report_file_complete(header.relative_path.clone());
   224	                        }
   225	                    }
   226	                }
   227	                PreparedPayload::FileBlock { .. }
   228	                | PreparedPayload::FileBlockComplete { .. }
   229	                | PreparedPayload::ResumeFile { .. } => {
   230	                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
   231	                }
   232	            }
   233	        }
   234	
   235	        Ok(())
   236	    }
   237	
   238	    pub async fn finish(&mut self) -> Result<()> {
   239	        self.stream
   240	            .write_all(&[DATA_PLANE_RECORD_END])
   241	            .await
   242	            .context("writing transfer terminator")?;
   243	        self.stream
   244	            .flush()
   245	            .await
   246	            .context("flushing data plane stream")
   247	    }
   248	
   249	    pub fn bytes_sent(&self) -> u64 {
   250	        self.bytes_sent
   251	    }
   252	
   253	    pub async fn send_file(
   254	        &mut self,
   255	        source: Arc<dyn TransferSource>,
   256	        header: &FileHeader,
   257	    ) -> Result<()> {
   258	        let rel = &header.relative_path;
   259	        let mut file = source
   260	            .open_file(header)
   261	            .await
   262	            .with_context(|| format!("opening {}", rel))?;
   263	        self.send_file_from_reader(header, &mut file).await
   264	    }
   265	
   266	    /// Send a file payload whose bytes come from an arbitrary async
   267	    /// reader (not a local file). Used by `DataPlaneSink` for the
   268	    /// remote→remote relay case, where bytes arrive from an inbound
   269	    /// `DataPlaneSource` and need to be forwarded to the next hop.
   270	    ///
   740	    #[cfg(not(unix))]
   741	    let _ = permissions;
   742	    Ok(SinkOutcome {
   743	        files_written: 1,
   744	        bytes_written: 0,
   745	    })
   746	}
   747	
   748	// ---------------------------------------------------------------------------
   749	// DataPlaneSink — TCP data plane writer
   750	// ---------------------------------------------------------------------------
   751	
   752	/// Writes payloads to a remote daemon via the TCP data plane binary protocol.
   753	///
   754	/// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
   755	/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
   756	pub struct DataPlaneSink<P: Probe = NoProbe> {
   757	    session: tokio::sync::Mutex<DataPlaneSession<P>>,
   758	    source: Arc<dyn TransferSource>,
   759	    dst_root: PathBuf,
   760	}
   761	
   762	impl<P: Probe> DataPlaneSink<P> {
   763	    pub fn new(
   764	        session: DataPlaneSession<P>,
   765	        source: Arc<dyn TransferSource>,
   766	        dst_root: PathBuf,
   767	    ) -> Self {
   768	        Self {
   769	            session: tokio::sync::Mutex::new(session),
   770	            source,
   771	            dst_root,
   772	        }
   773	    }
   774	}
   775	
   776	#[async_trait]
   777	impl<P: Probe> TransferSink for DataPlaneSink<P> {
   778	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   779	        let mut session = self.session.lock().await;
   780	        match payload {
   781	            PreparedPayload::File(header) => {
   782	                let size = header.size;
   783	                // otp-7b-2: name the file structurally on failure, so a
   784	                // mid-record fault reaches the end-of-operation summary.
   785	                session
   786	                    .send_file(self.source.clone(), &header)
   787	                    .await
   788	                    .with_context(|| format!("sending {}", header.relative_path))
   789	                    .map_err(|e| {
   790	                        e.wrap_err(crate::remote::transfer::faulted_path::FaultedPath(
   791	                            header.relative_path.clone(),
   792	                        ))
   793	                    })?;
   794	                Ok(SinkOutcome {
   795	                    files_written: 1,
   796	                    bytes_written: size,
   797	                })
   798	            }
   799	            PreparedPayload::TarShard { headers, data } => {
   800	                let bytes: u64 = headers.iter().map(|h| h.size).sum();
   801	                let count = headers.len();
   802	                session
   803	                    .send_prepared_tar_shard(headers, &data)
   804	                    .await
   805	                    .context("sending tar shard")?;
   806	                Ok(SinkOutcome {
   807	                    files_written: count,
   808	                    bytes_written: bytes,
   809	                })
   810	            }
   811	            // Resume payloads can't be relayed without a reverse-resume
   812	            // protocol on the next hop. Reject explicitly.
   813	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   814	                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
   815	            }
   816	            // otp-7b: one resume-flagged file's whole block phase. The
   817	            // session lock is held across the record, so every BLOCK and
   818	            // the closing BLOCK_COMPLETE ride THIS socket in order —
   819	            // the same strict serialization the in-stream carrier gets
   820	            // from its single control lane. The complete record carries
   821	            // mtime+perms from the manifest header so a zero-block
   822	            // resume still stamps metadata at the destination.
   823	            PreparedPayload::ResumeFile {
   824	                header,
   825	                block_size,
   826	                dest_hashes,
   827	            } => {
   828	                use crate::remote::transfer::resume_diff::{ResumeBlockDiff, ResumeDiffEvent};
   829	                let path = header.relative_path.clone();
   830	                let record = async {
   831	                    // codex otp-7b-1 F1: a mostly-matching scan is a
   832	                    // long SILENT read+hash — arm keepalive ticks well
   833	                    // inside the receiver's stall window and answer each
   834	                    // with a zero-length BLOCK (a no-op in-place write),
   835	                    // so a healthy scan never reads as a stalled peer.
   836	                    let mut diff = ResumeBlockDiff::open(
   837	                        &self.source,
   838	                        &header,
   839	                        block_size as usize,
   840	                        dest_hashes,
   841	                    )
   842	                    .await?
   843	                    .with_keepalive(
   844	                        crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT / 3,
   845	                    );
   846	                    let mut bytes_written: u64 = 0;
   847	                    while let Some(event) = diff.next_event().await? {
   848	                        match event {
   849	                            ResumeDiffEvent::Stale { offset, bytes } => {
   850	                                session
   851	                                    .send_block(&header.relative_path, offset, bytes)
   852	                                    .await
   853	                                    .context("sending resume block")?;
   854	                                bytes_written += bytes.len() as u64;
   855	                            }
   856	                            ResumeDiffEvent::KeepAlive { offset } => {
   857	                                session
   858	                                    .send_block(&header.relative_path, offset, &[])
   859	                                    .await
   860	                                    .context("sending resume keepalive block")?;
   861	                            }
   862	                        }
   863	                    }
   864	                    session
   865	                        .send_block_complete(
   866	                            &header.relative_path,
   867	                            header.size,
   868	                            header.mtime_seconds,
   869	                            header.permissions,
   870	                        )
   871	                        .await
   872	                        .context("sending resume block complete")?;
   873	                    Ok(SinkOutcome {
   874	                        files_written: 1,
   875	                        bytes_written,
   876	                    })
   877	                }
   878	                .await;
   879	                // otp-7b-2: any failure inside the record names its file
   880	                // structurally (the end-of-operation summary's identity).
   881	                record.map_err(|e: eyre::Report| {
   882	                    e.wrap_err(crate::remote::transfer::faulted_path::FaultedPath(path))
   883	                })
   884	            }
   885	        }
   886	    }
   887	
   888	    /// Relay case: bytes arrive on `reader` (e.g. from a DataPlaneSource
   889	    /// during a remote→remote transfer) and forward to the next hop.
   890	    async fn write_file_stream(
   891	        &self,
   892	        header: &FileHeader,
   893	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   894	    ) -> Result<SinkOutcome> {
   895	        let size = header.size;
   896	        let mut session = self.session.lock().await;
   897	        session
   898	            .send_file_from_reader(header, reader)
   899	            .await
   900	            .with_context(|| format!("relaying {}", header.relative_path))?;

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '250,430p' && rg -n \"DATA_PLANE_BUFFER_FLOOR|chunk_bytes\" crates/blit-core/src -g '*.rs' | head -120" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   250	        self.bytes_sent
   251	    }
   252	
   253	    pub async fn send_file(
   254	        &mut self,
   255	        source: Arc<dyn TransferSource>,
   256	        header: &FileHeader,
   257	    ) -> Result<()> {
   258	        let rel = &header.relative_path;
   259	        let mut file = source
   260	            .open_file(header)
   261	            .await
   262	            .with_context(|| format!("opening {}", rel))?;
   263	        self.send_file_from_reader(header, &mut file).await
   264	    }
   265	
   266	    /// Send a file payload whose bytes come from an arbitrary async
   267	    /// reader (not a local file). Used by `DataPlaneSink` for the
   268	    /// remote→remote relay case, where bytes arrive from an inbound
   269	    /// `DataPlaneSource` and need to be forwarded to the next hop.
   270	    ///
   271	    /// Same wire format and double-buffered loop as `send_file`.
   272	    pub async fn send_file_from_reader(
   273	        &mut self,
   274	        header: &FileHeader,
   275	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   276	    ) -> Result<()> {
   277	        let rel = &header.relative_path;
   278	        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);
   279	
   280	        let path_bytes = rel.as_bytes();
   281	        if path_bytes.len() > u32::MAX as usize {
   282	            bail!("relative path too long for transfer: {}", rel);
   283	        }
   284	
   285	        self.stream
   286	            .write_all(&[DATA_PLANE_RECORD_FILE])
   287	            .await
   288	            .context("writing data-plane record tag")?;
   289	        self.stream
   290	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   291	            .await
   292	            .context("writing path length")?;
   293	        self.stream
   294	            .write_all(path_bytes)
   295	            .await
   296	            .context("writing path bytes")?;
   297	
   298	        self.stream
   299	            .write_all(&header.size.to_be_bytes())
   300	            .await
   301	            .context("writing file size")?;
   302	        // Wire-format extension (2026-05-01): include mtime + permissions
   303	        // inline so push and pull data plane records carry the same
   304	        // information. Lets the receive pipeline apply metadata via
   305	        // FsTransferSink without consulting an out-of-band manifest cache.
   306	        self.stream
   307	            .write_all(&header.mtime_seconds.to_be_bytes())
   308	            .await
   309	            .context("writing mtime")?;
   310	        self.stream
   311	            .write_all(&header.permissions.to_be_bytes())
   312	            .await
   313	            .context("writing permissions")?;
   314	
   315	        // Double-buffered I/O: overlaps source reads with network writes
   316	        self.send_file_double_buffered(reader, header, rel).await?;
   317	
   318	        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);
   319	
   320	        Ok(())
   321	    }
   322	
   323	    /// Double-buffered file sending: overlaps disk reads with network writes.
   324	    /// Uses two buffers from the pool to enable concurrent I/O operations.
   325	    ///
   326	    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
   327	    /// This hides disk latency behind network latency for improved throughput.
   328	    async fn send_file_double_buffered(
   329	        &mut self,
   330	        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   331	        header: &FileHeader,
   332	        rel: &str,
   333	    ) -> Result<()> {
   334	        let mut remaining = header.size;
   335	        if remaining == 0 {
   336	            return Ok(());
   337	        }
   338	
   339	        // Acquire two buffers for double-buffering
   340	        let mut buf_a = self.pool.acquire().await;
   341	        let mut buf_b = self.pool.acquire().await;
   342	
   343	        // Initial read into buf_a
   344	        let mut bytes_a = file
   345	            .read(buf_a.as_mut_slice())
   346	            .await
   347	            .with_context(|| format!("reading {}", rel))?;
   348	
   349	        if bytes_a == 0 {
   350	            bail!(
   351	                "unexpected EOF while reading {} ({} bytes remaining)",
   352	                rel,
   353	                remaining
   354	            );
   355	        }
   356	        // Clamp to the declared size before subtracting. A source that
   357	        // returns more bytes than `header.size` — a file that grew after
   358	        // the manifest was computed, or a lying `TransferSource` — would
   359	        // otherwise underflow `remaining` (debug: panic; release: wrap to
   360	        // u64::MAX → runaway loop) and push undeclared bytes onto the
   361	        // framed stream. We send exactly `header.size` and ignore excess.
   362	        bytes_a = (bytes_a as u64).min(remaining) as usize;
   363	        remaining -= bytes_a as u64;
   364	
   365	        // Main loop: write buf_a while reading into buf_b
   366	        while remaining > 0 {
   367	            // Per-stream telemetry: time ONLY the socket write as the
   368	            // backpressure signal. ue-r2-1e (carried ue-r2-1a review
   369	            // finding): the old code timed the whole overlapped
   370	            // write+read join, so a slow disk READ inflated
   371	            // "write blocked" and would bias the dial tuner
   372	            // conservative. The async block's clock starts when the
   373	            // join first polls it and stops when write_all completes —
   374	            // the concurrent read neither extends nor shortens it.
   375	            // Gated on the compile-time `P::ACTIVE` constant so
   376	            // `DataPlaneSession<NoProbe>` reads no clock.
   377	            let write_slice = &buf_a.as_slice()[..bytes_a];
   378	            let stream = &mut self.stream;
   379	            let (write_outcome, read_result) = tokio::join!(
   380	                async {
   381	                    let started = if P::ACTIVE {
   382	                        Some(std::time::Instant::now())
   383	                    } else {
   384	                        None
   385	                    };
   386	                    let result = stream.write_all(write_slice).await;
   387	                    (result, started.map(|t| t.elapsed()))
   388	                },
   389	                file.read(buf_b.as_mut_slice())
   390	            );
   391	
   392	            let (write_result, write_elapsed) = write_outcome;
   393	            write_result.with_context(|| format!("sending {}", rel))?;
   394	            if let Some(elapsed) = write_elapsed {
   395	                self.probe.note_write_blocked(elapsed.as_nanos() as u64);
   396	            }
   397	            self.probe.record_bytes(bytes_a as u64);
   398	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   399	
   400	            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
   401	
   402	            if bytes_b == 0 && remaining > 0 {
   403	                bail!(
   404	                    "unexpected EOF while reading {} ({} bytes remaining)",
   405	                    rel,
   406	                    remaining
   407	                );
   408	            }
   409	            // Same clamp as the initial read: never subtract more than
   410	            // `remaining`, so an over-returning reader can neither
   411	            // underflow the counter nor send undeclared bytes.
   412	            let bytes_b = (bytes_b as u64).min(remaining) as usize;
   413	            remaining -= bytes_b as u64;
   414	
   415	            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
   416	            std::mem::swap(&mut buf_a, &mut buf_b);
   417	            bytes_a = bytes_b;
   418	        }
   419	
   420	        // Final write: send the last chunk in buf_a. This is a pure
   421	        // write (no overlapped read), so the timing is cleanly
   422	        // attributable to socket-write backpressure.
   423	        if bytes_a > 0 {
   424	            let tail_start = if P::ACTIVE {
   425	                Some(std::time::Instant::now())
   426	            } else {
   427	                None
   428	            };
   429	            self.stream
   430	                .write_all(&buf_a.as_slice()[..bytes_a])
crates/blit-core/src/buffer.rs:23:pub const DATA_PLANE_BUFFER_FLOOR: usize = 64 * KB;
crates/blit-core/src/buffer.rs:166:/// let pool = Arc::new(BufferPool::for_data_plane(dial.chunk_bytes(), stream_count));
crates/blit-core/src/buffer.rs:225:/// - `buffer_size = chunk_bytes.max(DATA_PLANE_BUFFER_FLOOR)`, shrunk
crates/blit-core/src/buffer.rs:233:    chunk_bytes: usize,
crates/blit-core/src/buffer.rs:243:    let mut buffer_size = chunk_bytes.max(DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:248:        buffer_size = (cap / (streams * 2)).max(DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:300:    /// (down to [`DATA_PLANE_BUFFER_FLOOR`]) instead of the concurrency:
crates/blit-core/src/buffer.rs:307:    pub fn for_data_plane(chunk_bytes: usize, streams: usize) -> Self {
crates/blit-core/src/buffer.rs:309:            data_plane_pool_params(chunk_bytes, streams, available_memory_bytes());
crates/blit-core/src/buffer.rs:549:        assert_eq!(buffer_size, DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:551:        assert_eq!(budget, DATA_PLANE_BUFFER_FLOOR * 6 * 2);
crates/blit-core/src/buffer.rs:586:        assert_eq!(buffer_size, DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:587:        assert_eq!(budget, DATA_PLANE_BUFFER_FLOOR * 16 * 2);
crates/blit-core/src/buffer.rs:596:        for &chunk in &[0, 1024, DATA_PLANE_BUFFER_FLOOR, MIB, 16 * MIB, 64 * MIB] {
crates/blit-core/src/buffer.rs:607:                    assert!(buffer_size >= DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:626:        assert!(pool.buffer_size() >= DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/remote/transfer/data_plane.rs:62:    chunk_bytes: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:90:        chunk_bytes: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:94:        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
crates/blit-core/src/remote/transfer/data_plane.rs:104:        chunk_bytes: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:114:            chunk_bytes,
crates/blit-core/src/remote/transfer/data_plane.rs:134:        chunk_bytes: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:154:            Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, probe)
crates/blit-core/src/remote/transfer/data_plane.rs:168:        chunk_bytes: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:174:        let chunk_bytes = chunk_bytes.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/remote/transfer/data_plane.rs:179:            chunk_bytes,
crates/blit-core/src/remote/transfer/data_plane.rs:506:        for chunk in data.chunks(self.chunk_bytes.max(1)) {
crates/blit-core/src/remote/transfer/data_plane.rs:643:/// the send side's pooled buffers (the dial's chunk_bytes, 16–64 MiB):
crates/blit-core/src/remote/transfer/data_plane.rs:693:    let cap = buffer_size.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/dial.rs:11://! - **Cheap dials** — `chunk_bytes`, `prefetch_count`: atomics the
crates/blit-core/src/dial.rs:79:        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
crates/blit-core/src/dial.rs:87:    chunk_bytes: AtomicUsize,
crates/blit-core/src/dial.rs:112:    ceiling_chunk_bytes: usize,
crates/blit-core/src/dial.rs:150:            if profile.max_chunk_bytes > 0 {
crates/blit-core/src/dial.rs:151:                ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
crates/blit-core/src/dial.rs:165:                    ceiling_chunk.min(inflight.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR));
crates/blit-core/src/dial.rs:171:            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
crates/blit-core/src/dial.rs:181:            ceiling_chunk_bytes: ceiling_chunk,
crates/blit-core/src/dial.rs:193:    pub fn chunk_bytes(&self) -> usize {
crates/blit-core/src/dial.rs:194:        self.chunk_bytes.load(Ordering::Relaxed)
crates/blit-core/src/dial.rs:248:        self.chunk_bytes.load(Ordering::Relaxed) >= self.ceiling_chunk_bytes
crates/blit-core/src/dial.rs:253:        self.chunk_bytes.load(Ordering::Relaxed)
crates/blit-core/src/dial.rs:254:            <= DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes)
crates/blit-core/src/dial.rs:404:        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
crates/blit-core/src/dial.rs:405:        let next = (chunk.saturating_mul(2)).min(self.ceiling_chunk_bytes);
crates/blit-core/src/dial.rs:407:            self.chunk_bytes.store(next, Ordering::Relaxed);
crates/blit-core/src/dial.rs:429:        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
crates/blit-core/src/dial.rs:430:        let next = (chunk / 2).max(DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes));
crates/blit-core/src/dial.rs:432:            self.chunk_bytes.store(next, Ordering::Relaxed);
crates/blit-core/src/dial.rs:618:            max_chunk_bytes: max_chunk,
crates/blit-core/src/dial.rs:626:        assert_eq!(dial.chunk_bytes(), 16 * MIB);
crates/blit-core/src/dial.rs:639:        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
crates/blit-core/src/dial.rs:650:        assert_eq!(dial.chunk_bytes(), 32 * MIB, "chunk ceiling from profile");
crates/blit-core/src/dial.rs:659:        assert_eq!(tight.chunk_bytes(), 8 * MIB);
crates/blit-core/src/dial.rs:664:        assert_eq!(generous.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
crates/blit-core/src/dial.rs:674:        assert_eq!(dial.chunk_bytes(), 32 * MIB);
crates/blit-core/src/dial.rs:680:        assert_eq!(dial.chunk_bytes(), 16 * MIB);
crates/blit-core/src/dial.rs:682:        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
crates/blit-core/src/dial.rs:753:        assert_eq!(dial.chunk_bytes(), 16 * MIB, "idle tick is no signal");
crates/blit-core/src/dial.rs:759:            if dial.chunk_bytes() > 16 * MIB {
crates/blit-core/src/dial.rs:764:        assert_eq!(dial.chunk_bytes(), 32 * MIB, "stepped up once");
crates/blit-core/src/transfer_session/data_plane.rs:664:    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
crates/blit-core/src/transfer_session/data_plane.rs:709:        dial.chunk_bytes(),
crates/blit-core/src/transfer_session/data_plane.rs:719:            dial.chunk_bytes(),
crates/blit-core/src/transfer_session/data_plane.rs:805:        dial.chunk_bytes(),
crates/blit-core/src/transfer_session/data_plane.rs:815:            dial.chunk_bytes(),
crates/blit-core/src/transfer_session/data_plane.rs:920:                    self.dial.chunk_bytes(),
crates/blit-core/src/transfer_session/data_plane.rs:936:                    self.dial.chunk_bytes(),

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '188,238p' && nl -ba docs/bench/otp12c-win-2026-07-13/runs.csv | sed -n '168,190p' && nl -ba docs/bench/otp12-win-2026-07-12/README.md | sed -n '1,40p' && nl -ba docs/bench/otp12c-win-2026-07-13/README.md | sed -n '1,42p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   188	        || die "$OLD_WIN_DAEMON does not embed +$OLD_SHA — the staged old daemon is not the pinned pair"
   189	    if LC_ALL=C grep -qa "+$OLD_SHA" "$OLD_BLIT"; then
   190	        :
   191	    elif [[ "${OLD_CLIENT_PROVENANCE_BY_BUILD:-0}" == 1 ]]; then
   192	        log "old client: no embedded +$OLD_SHA id (pre-cutover binary); provenance = clean-worktree build + manifest (acknowledged)"
   193	    else
   194	        die "$OLD_BLIT does not embed +$OLD_SHA; if it is the pre-cutover client rebuilt clean per D6, re-run with OLD_CLIENT_PROVENANCE_BY_BUILD=1"
   195	    fi
   196	    # Stale refusal, both hosts.
   197	    if wssh "if (Get-Process blit-daemon -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }" 2>/dev/null; then
   198	        die "a blit-daemon is already running on the Windows host — stop it first"
   199	    fi
   200	    if pgrep -x blit-daemon >/dev/null 2>&1; then
   201	        die "a blit-daemon is already running on the Mac — stop it first"
   202	    fi
   203	    [[ -z $(git -C "$REPO_ROOT" status --porcelain) ]] \
   204	        || die "working tree DIRTY — the recorded run must be a clean checkout of $NEW_SHA"
   205	    log "preflight OK  old pair: $OLD_SHA  new pair: $NEW_SHA  runs/arm: $RUNS  mac endpoint: $MAC_HOST:$PORT"
   206	}
   207	
   208	OLD_WIN_DAEMON_HASH=""
   209	NEW_WIN_DAEMON_HASH=""
   210	write_manifest() {
   211	    # Hashes are captured into variables FIRST so a failure dies for
   212	    # real — `echo "$(die …)"` only exits the subshell (codex otp-12b
   213	    # F1, the otp-12a F3 lesson re-applied).
   214	    local f="$OUT_DIR/staging-manifest.txt"
   215	    local h_oc h_nc h_md h_wc h_ref
   216	    h_oc=$(sha256_local "$OLD_BLIT")
   217	    h_nc=$(sha256_local "$NEW_BLIT")
   218	    h_md=$(sha256_local "$MAC_DAEMON")
   219	    OLD_WIN_DAEMON_HASH=$(sha256_win "$OLD_WIN_DAEMON")
   220	    NEW_WIN_DAEMON_HASH=$(sha256_win "$NEW_WIN_DAEMON")
   221	    h_wc=$(sha256_win "$WIN_BLIT")
   222	    h_ref=$(sha256_local "$BASELINE_SUMMARY")
   223	    {
   224	        echo "arm,role,sha,sha256,path"
   225	        echo "old,client,$OLD_SHA,$h_oc,$OLD_BLIT"
   226	        echo "new,client,$NEW_SHA,$h_nc,$NEW_BLIT"
   227	        echo "new,mac-daemon,$NEW_SHA,$h_md,$MAC_DAEMON"
   228	        echo "old,win-daemon,$OLD_SHA,$OLD_WIN_DAEMON_HASH,$OLD_WIN_DAEMON"
   229	        echo "new,win-daemon,$NEW_SHA,$NEW_WIN_DAEMON_HASH,$NEW_WIN_DAEMON"
   230	        echo "new,win-client,$NEW_SHA,$h_wc,$WIN_BLIT"
   231	        echo "-,reference,-,$h_ref,$BASELINE_SUMMARY"
   232	    } > "$f"
   233	    log "staging manifest recorded (7 hashes)"
   234	}
   235	
   236	# --- One-time host setup (idempotent) ----------------------------------
   237	setup_host() {
   238	    scp -q -o BatchMode=yes "$SCRIPT_DIR/windows/purge-standby.ps1" \
   168	mw_tcp_mixed,mac_init,f35702a,mac,1,1690,667,0,drained_6s,yes
   169	mw_tcp_mixed,win_init,f35702a,win,1,1479,650,0,drained_8s,yes
   170	mw_tcp_mixed,mac_init,f35702a,mac,2,1417,645,0,drained_6s,yes
   171	mw_tcp_mixed,win_init,f35702a,win,2,1450,655,0,drained_6s,yes
   172	mw_tcp_mixed,mac_init,f35702a,mac,3,1402,655,0,drained_8s,yes
   173	mw_tcp_mixed,win_init,f35702a,win,3,1751,680,0,drained_8s,yes
   174	mw_tcp_mixed,mac_init,f35702a,mac,4,1414,651,0,drained_8s,yes
   175	mw_tcp_mixed,win_init,f35702a,win,4,1475,656,0,drained_8s,yes
   176	wm_tcp_mixed,mac_init,f35702a,mac,1,1268,81,0,drained_8s,yes
   177	wm_tcp_mixed,win_init,f35702a,win,1,923,81,0,drained_6s,yes
   178	wm_tcp_mixed,mac_init,f35702a,mac,2,1192,79,0,drained_8s,yes
   179	wm_tcp_mixed,win_init,f35702a,win,2,920,81,0,drained_8s,yes
   180	wm_tcp_mixed,mac_init,f35702a,mac,3,1238,80,0,drained_8s,yes
   181	wm_tcp_mixed,win_init,f35702a,win,3,997,83,0,drained_8s,yes
   182	wm_tcp_mixed,mac_init,f35702a,mac,4,1205,82,0,drained_8s,yes
   183	wm_tcp_mixed,win_init,f35702a,win,4,955,83,0,drained_6s,yes
   184	mw_grpc_mixed,mac_init,f35702a,mac,1,1789,680,0,drained_8s,yes
   185	mw_grpc_mixed,win_init,f35702a,win,1,1830,679,0,drained_8s,yes
   186	mw_grpc_mixed,mac_init,f35702a,mac,2,1800,685,0,drained_6s,yes
   187	mw_grpc_mixed,win_init,f35702a,win,2,1872,681,0,drained_6s,yes
   188	mw_grpc_mixed,mac_init,f35702a,mac,3,1783,695,0,drained_8s,yes
   189	mw_grpc_mixed,win_init,f35702a,win,3,1844,671,0,drained_8s,yes
   190	mw_grpc_mixed,mac_init,f35702a,mac,4,1832,667,0,drained_6s,yes
     1	# otp-12b — Mac↔Windows acceptance session: converge-up + initiator/verb invariance (2026-07-12)
     2	
     3	**Status**: Recorded. **Scope**: the owner-designated closest-spec pair —
     4	rig W carries the plan's cross-direction half AND the headline
     5	initiator/verb-invariance criterion (`docs/plan/OTP12_ACCEPTANCE_RUN.md`
     6	D2–D3; parent criteria 1–2 as annotated by D-2026-07-12-1). **This
     7	README declares nothing** — pass/fail belongs to the owner at otp-13;
     8	it records the computed rows.
     9	
    10	**Harness**: `scripts/bench_otp12_win.sh` at run commit `e21cf84`
    11	(design/harness codex rounds: 12 findings accepted at `d3eae58`; two
    12	found-live fixes after first rig contact: the pwsh scope-qualified
    13	`$rc:R` sentinel parse at `e21cf84`, and the CR-in-drain-outcome CSV
    14	split at `856af64` — see Post-processing). RUNS=4, ABBA, pair-void rule;
    15	**192 timed runs, zero voided pairs, zero drain anomalies**.
    16	
    17	## Builds (matched pairs, sha-verified; 7 hashes in `staging-manifest.txt`)
    18	
    19	- **old arm**: `0f922de` both ends — Mac client rebuilt clean in a
    20	  detached worktree (pre-cutover clients embed no id:
    21	  `OLD_CLIENT_PROVENANCE_BY_BUILD=1`, provenance = build procedure +
    22	  manifest); Windows daemon = the aside-copied native detached-checkout
    23	  build (embeds `+0f922de`, Select-String-verified).
    24	- **new arm**: `e21cf84` both ends (Mac local build; Windows native
    25	  build from a fresh bundle; `blit.exe` client likewise staged).
    26	- Rig note: the box is `netwatch-01` at **10.1.10.177** (the recorded
    27	  10.1.10.173 went stale — DHCP); Mac 10 GbE at 10.1.10.54.
    28	
    29	## Post-processing (recorded, reproducible)
    30	
    31	The session's `runs.csv` was CR-sanitized after the run (`tr -d '\r'`;
    32	original committed as `runs-raw-crlf.csv`): pwsh emits CRLF and the
    33	bare `\r` in the drain column split every row under python's
    34	universal-newline csv reader, verdicting everything INCOMPLETE off 192
    35	valid runs. `verdicts.csv`/`summary.csv` were recomputed with the
    36	harness's own verdict pass over the sanitized rows; the harness now
    37	strips CRs at source (`856af64`). No timing value was altered.
    38	
    39	## Block 1 — converge-up (Mac-initiated, old vs new interleaved): 10/12 PASS
    40	
     1	# otp-12c — Mac↔Windows re-run at the cutover sha (2026-07-13)
     2	
     3	**Status**: Recorded. **Scope**: the rig-W matrix re-executed with the
     4	new arm at `f35702a` — the sha the shipped cutover binaries embed (12b
     5	ran its new arm at `e21cf84`). Old arm unchanged (`0f922de`). Run as
     6	the direct-path baseline ahead of the 12c delegated session
     7	(netwatch-01↔skippy). **This README declares nothing** — pass/fail
     8	belongs to the owner at otp-13; it records the computed rows.
     9	
    10	**Harness**: `scripts/bench_otp12_win.sh` at run commit `f35702a`
    11	(clean detached worktree `blit_v2_f35702a`; includes the 12b fixes
    12	`856af64`/`49dee5c`/`b0a7bd9` — CR-strip at source held: 0 CR bytes in
    13	`runs.csv`, no post-processing this time). Invocation:
    14	`MAC_HOST=10.1.10.91 OLD_CLIENT_PROVENANCE_BY_BUILD=1
    15	bash scripts/bench_otp12_win.sh`. RUNS=4, ABBA, pair-void rule;
    16	**198 timed runs, 24/24 cells complete**; 3 pairs voided on
    17	DRAIN-TIMEOUT and re-run to completion (`push_grpc_mixed` slot 3,
    18	`mw_tcp_large` slot 2, `wm_tcp_small` slot 1 — see
    19	`drain-outcomes.txt`).
    20	Session `005904.49434`, 00:59–01:51 local. Endpoints: Mac
    21	`10.1.10.91:9031`, netwatch-01 `10.1.10.177:9031`.
    22	
    23	## Builds (sha-verified; 7 hashes in `staging-manifest.txt`)
    24	
    25	- **old arm**: `0f922de` both ends. Mac client is pre-cutover and
    26	  embeds no id — provenance = clean-worktree build + staging manifest
    27	  (`OLD_CLIENT_PROVENANCE_BY_BUILD=1` acknowledged, logged in
    28	  `bench.log` preflight lines).
    29	- **new arm**: `f35702a` both ends (embed-verified at preflight).
    30	
    31	## Verdicts — 120 rows: 93 PASS / 12 FAIL / 3 FAIL-SAME-SESSION / 12 RECORDED
    32	
    33	FAIL rows verbatim from `verdicts.csv`:
    34	
    35	```
    36	pull_tcp_mixed,converge,new,old_session,1192,956,1.247,1.10,FAIL
    37	pull_tcp_mixed,converge,new,combined,1192,,,1.10,FAIL-SAME-SESSION
    38	push_tcp_mixed,converge,new,old_session,1703,1491,1.142,1.10,FAIL
    39	push_tcp_mixed,converge,new,combined,1703,,,1.10,FAIL-SAME-SESSION
    40	push_tcp_small,converge,new,old_session,1975,1644,1.201,1.10,FAIL
    41	push_tcp_small,converge,new,combined,1975,,,1.10,FAIL-SAME-SESSION
    42	mw_grpc_large,cross,worst_arm,min_old_committed,1750,1289,1.358,1.10,FAIL

exec
/bin/zsh -lc 'nl -ba docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp-12 rig-W jumbo re-run — PRE-REGISTRATION (written before any timed run)
     2	
     3	**Status**: Pre-registered. **No data exists yet.** This file is committed
     4	BEFORE the run so the decision rule cannot be authored after seeing the
     5	numbers. Results land beside it in `README.md` + CSVs; this file is not
     6	edited once data exists (corrections go in `README.md`, marked as such).
     7	
     8	**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (**Active**, D-2026-07-13-1);
     9	`docs/STATE.md` Queue 1a names this run as the next action, ahead of any
    10	code. **This is an ENVIRONMENTAL experiment, not a code counterfactual** —
    11	it is not one of H1–H7 and it changes nothing in the tree. It runs first
    12	because it is the cheapest experiment available and it can invalidate the
    13	premise of the expensive ones.
    14	
    15	## The question
    16	
    17	`netwatch-01` ran at **MTU 1500 for every benchmark ever recorded**
    18	(otp-2w, otp-12a/b/c — `.agents/machines.md` §Network/MTU). Jumbo has
    19	therefore **never once been exercised** by a blit benchmark. P1 — the
    20	headline invariance failure — is the **TCP × mixed × destination-initiator**
    21	cell, i.e. the packet-heaviest fixture we own. If per-packet cost is the
    22	mechanism, ~6× fewer packets is exactly where it would show.
    23	
    24	**Falsifiable premise**: MTU 1500 → 9000 is causally responsible for some
    25	or all of P1's 282 ms invariance gap (`Δ_P1(rig W)`, plan §decision rule).
    26	
    27	## Instrument validation — performed BEFORE the run
    28	
    29	This session has retracted three claims, all from trusting an unvalidated
    30	instrument. So the instrument is validated first, and the validation is
    31	recorded whether or not it flatters the hypothesis.
    32	
    33	| check | method | result |
    34	|---|---|---|
    35	| Mac interface MTU | `ifconfig en9` | 9000 (`en9` = **10.1.10.54**, Aquantia) |
    36	| Windows interface MTU | `Get-NetIPInterface` | `NlMtu` **9000** |
    37	| Windows NIC jumbo | `Get-NetAdapterAdvancedProperty` | `Jumbo Packet = 9014 Bytes` (adapter level, not just IP) |
    38	| L2 path, Win→Mac | DF ping, 8972 B payload | **OK**, 0% loss |
    39	| L2 path, Linux→Mac and →Win | skippy `ping -M do -s 8972` | **OK**, both |
    40	| L2 path, Mac→Win | DF ping, 8000 B payload | **OK** (macOS raw-socket cap of 8192 forbids more — the known ping trap, not a network limit) |
    41	| **negotiated TCP MSS, Mac→Win** | **`getsockopt(TCP_MAXSEG)`** on a live socket | **8948** |
    42	| **Mac's advertised MSS** | Linux `ss -ti` toward the Mac | **8948** (so Windows sends 8948 to the Mac) |
    43	| **Win's advertised MSS** | Linux `ss -ti` toward netwatch-01 | **8948** |
    44	
    45	`8948 = 9000 − 40 (IP+TCP) − 12 (TCP timestamps)`. At MTU 1500 the MSS was
    46	**1448**. **Segment-count reduction is therefore 6.18×, MEASURED, in BOTH
    47	directions of P1's cell** — not assumed. `getsockopt`/`ss` read the TCP
    48	control block, so this number cannot be faked by offload or coalescing.
    49	
    50	**An instrument was tested and DISCARDED.** Windows
    51	`Get-NetAdapterStatistics` reported **10 680 received bytes per "packet"**
    52	during a 1 GiB TCP transfer — *larger than a 9014-byte frame*, so that NIC
    53	coalesces on receive despite reporting `RSC IPv4Enabled: False`. NIC packet
    54	counters **cannot** discriminate 1500 from 9000 on this rig and must not be
    55	cited. (Recorded because it would have "confirmed" jumbo either way.)
    56	
    57	## What is held constant, and what is not
    58	
    59	| variable | 12b (2026-07-12) | 12c (2026-07-13) | THIS RUN |
    60	|---|---|---|---|
    61	| new-arm sha | `e21cf84` | `f35702a` | **`f35702a`** (same worktree, binaries verified to embed `+f35702a`) |
    62	| old-arm sha | `0f922de` | `0f922de` | **`0f922de`** |
    63	| harness | `bench_otp12_win.sh` | same | **same** |
    64	| Mac NIC | Aquantia @ **10.1.10.54** | TB5 dock @ 10.1.10.91 | **Aquantia @ 10.1.10.54** |
    65	| **Windows MTU** | **1500** | **1500** | **9000** |
    66	| `wm_tcp_mixed` invariance | 1.237 FAIL | 1.300 FAIL | ? |
    67	
    68	**Neither prior session is a single-variable control**, and the pre-existing
    69	STATE note ("the Mac's NIC also changed") understates the situation — but
    70	also mis-locates it. The confound is **not** the NIC in general: **12b ran
    71	on this very Aquantia adapter at MTU 1500 and P1 FAILED at 1.237.** So an
    72	Aquantia-vs-dock difference cannot by itself dissolve P1. The one
    73	combination **no session has ever run** is `Aquantia × MTU 1500 × f35702a`.
    74	That — not "the NIC" — is the control this experiment may need.
    75	
    76	## Verdict rows that this run VOIDS (important, and not obvious)
    77	
    78	The harness grades every cell against **two** references: the same-session
    79	old arm, and the **committed baseline** `otp2w-baseline-2026-07-10/summary.csv`
    80	(`bench_otp12_win.sh:105`, pre-registered and not overridable).
    81	
    82	**That committed baseline was recorded at MTU 1500.** At jumbo:
    83	
    84	- **Same-session rows stay sound.** Both arms run on today's network, so
    85	  `converge … old_session` and every **invariance** row (new-vs-new, one
    86	  session) are fair comparisons. **P1 is an invariance row — it is
    87	  measured cleanly at jumbo.** So is P2's same-session row.
    88	- **Every `old_committed` and `cross … min_old_committed` row is VOID.**
    89	  They compare a jumbo new arm against a 1500-MTU reference: the network
    90	  improved under the reference, so those rows are flattering by
    91	  construction. A PASS there is not evidence of convergence.
    92	
    93	**Consequence for the plan, stated up front:** P2's bar requires ≤1.10
    94	against **BOTH** references (`OTP12_PERF_FINDINGS.md` §Fix criteria). At
    95	jumbo the committed reference is stale, so **P2's committed-reference row
    96	cannot be satisfied until the committed baseline is re-recorded at jumbo.**
    97	This run does not close P2 regardless of outcome. If the fleet stays at
    98	jumbo, `pf-final` needs a re-baselined committed reference — that is a plan
    99	amendment, and it goes through the loop; it is not assumed here.
   100	
   101	## Pre-registered predictions and decision rule
   102	
   103	Reference values, `wm_tcp_mixed` (12c): dest-initiated arm (`mac_init`)
   104	**1221 ms**, source-initiated arm (`win_init`) **939 ms**, ratio **1.300**;
   105	`Δ_P1(rig W)` = **282 ms**.
   106	
   107	**If the MTU premise is TRUE**, the run shows all of:
   108	1. `wm_tcp_mixed` invariance ratio falls to **≤ 1.10**;
   109	2. it falls because the **slow arm speeds up** — `mac_init` drops toward
   110	   939 ms — **not** because the fast arm slows down;
   111	3. **absolute times move somewhere**: packet-heavy cells (mixed, small) show
   112	   real speedups against 12c. This is the run's built-in positive control.
   113	   If literally nothing moves in absolute terms, the measured MSS of 8948
   114	   is contradicted by the wall clock and the run is suspect, not a null.
   115	
   116	**Bands (pre-registered, no post-hoc adjustment).** Ratio `r` = the
   117	`wm_tcp_mixed` invariance ratio at RUNS=4:
   118	
   119	- **`r ≤ 1.10`** → P1 does not reproduce at jumbo. **This is NOT yet a
   120	  conclusion** — it triggers BOTH confirmations below before any claim is
   121	  recorded.
   122	- **`r ≥ 1.20`** → **MTU is not the cause.** P1 stands (12b 1.237, 12c 1.300
   123	  both sit here). No control run needed: the asymmetry survived a 6.18×
   124	  packet reduction on two different NICs. Proceed to pf-1 unchanged.
   125	- **`1.10 < r < 1.20`** → **INDETERMINATE.** Session-to-session drift on this
   126	  cell is already ~5% (1.237 → 1.300 on the same 1500 network), so a
   127	  4-sample median cannot resolve this band. Escalate to **RUNS=8** on the P1
   128	  cells (the plan's D2 escalation) before saying anything.
   129	
   130	**THE MASKING TRAP** (the failure mode that disqualified zoey and altiera as
   131	rigs — `.agents/machines.md`): a ratio can fall toward 1.0 because a *shared*
   132	bottleneck compresses both arms, not because the defect was fixed. Two
   133	concrete guards, both required for `r ≤ 1.10` to count:
   134	
   135	- **Fast-arm guard**: `win_init` median must not regress — it must stay
   136	  ≤ 939 × 1.10 ≈ **1033 ms**. If the ratio "passed" because the fast arm got
   137	  slower, that is degradation wearing a PASS, and it is reported as such.
   138	- **Slow-arm guard**: `mac_init` must fall by **≥ 70% of Δ_P1** (≥ 197 ms of
   139	  the 282 ms), i.e. to **≤ 1024 ms**. This is the plan's own ≥70% closure
   140	  threshold (§pf-1 decision rule), reused so the environmental cause is held
   141	  to the same bar as a code cause. A ratio that passes while the slow arm
   142	  barely moved means both arms drifted, not that P1 was fixed.
   143	
   144	**Required confirmations before ANY "jumbo dissolves P1" claim is recorded:**
   145	1. **RUNS=8 escalation** on `wm_tcp_mixed` + `pull_tcp_mixed` (P1's bar is
   146	   defined at RUNS=8, plan §Fix criteria).
   147	2. **The control run**: `Aquantia × MTU 1500 × f35702a`, `CELLS=wm_tcp_mixed,
   148	   mw_tcp_mixed,pull_tcp_mixed` — the one combination never run. It
   149	   discriminates the last live alternative: if the control **reproduces**
   150	   ~1.24–1.30, MTU is confirmed causal; if the control **passes at 1500**,
   151	   then MTU is exonerated and 12c's 1.300 was an artifact of the TB5 dock,
   152	   which would retroactively void 12c's P1 row rather than confirm it.
   153	   Note this control requires flipping the Windows MTU back to 1500 and back
   154	   again — a rig change, and it needs the owner's go.
   155	
   156	**What no outcome licenses.** Even a clean PASS does not by itself close P1
   157	under the parent plan: `OTP12_PERF_FINDINGS.md` §Fix criteria defines P1's
   158	bar on the netwatch-01 rig at RUNS=8 against both references, and the global
   159	rule requires every other cell to hold too. And it would not close P2, whose
   160	committed reference is void at jumbo (above). **A PASS here changes what the
   161	next experiment is; it does not end the investigation.**
   162	
   163	## The run
   164	
   165	Full 24-cell matrix, RUNS=4, ABBA, pair-void — a straight replication of the
   166	12c session with MTU as the intended difference. Full matrix rather than a
   167	`CELLS` subset because the controls (`mw_tcp_mixed` opposite direction,
   168	`wm_grpc_mixed` opposite carrier, `wm_tcp_{large,small}` opposite fixture)
   169	are what make P1's cell interpretable, and because cell ordering / cache and
   170	thermal history would otherwise differ from the session being replicated.
   171	
   172	```sh
   173	cd /Users/michael/Dev/blit_v2_f35702a          # clean detached worktree @ f35702a
   174	MAC_HOST=10.1.10.54 OLD_CLIENT_PROVENANCE_BY_BUILD=1 \
   175	  bash scripts/bench_otp12_win.sh
   176	```
   177	
   178	Staging verified before writing this file: worktree clean at `f35702a`;
   179	`target/release/{blit,blit-daemon}` embed `+f35702a`; Windows has
   180	`D:\blit-test\bins\{0f922de,f35702a}`; no stale daemon on either host; Mac
   181	old client at `~/blit-bench-work/bins/blit-0f922de`.
   182	
   183	**Known rig-state change made during validation** (recorded, not hidden):
   184	netwatch-01's `known_hosts` gained the Mac's key at its new IP 10.1.10.54
   185	(it only had the retired .91). No blit code, config, or fixture was touched.

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '270,340p' && rg -n \"frozen|baseline\" docs/bench/otp2w-baseline-2026-07-10/README.md | head -60 && nl -ba docs/bench/otp2w-baseline-2026-07-10/README.md | sed -n '1,80p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   270	self-timed flush — deliberately INCLUDING the trigger RPC + relay overhead
   271	(that is the honest end-to-end cost of delegation; on this LAN the trigger
   272	is sub-ms against multi-second cells). The direct arm is self-timed on the
   273	initiating host plus the same flush. Destination flush: Windows ⇒
   274	`Write-VolumeCache`; skippy ⇒ self-timed `sync` bracketed by
   275	`/proc/uptime` reads in one ssh shell. Cold caches: standby-purge (Windows)
   276	+ `drop_caches` (skippy, root/sudo) both ends every run; drain the
   277	destination disk (Windows counter loop; skippy `/proc/diskstats` quiet-
   278	window loop with a device-regex knob).
   279	
   280	Carrier: TCP is the verdict carrier; one secondary grpc pair
   281	(large × skippy→Windows, both arms) is recorded as a smoke row — carrier
   282	selection reads `SessionOpen.in_stream_bytes`/policy, never role or
   283	initiator (`transfer_session/mod.rs:790,805`), and carrier invariance is
   284	measured properly on rig W.
   285	
   286	Config: BOTH daemons get `[delegation] allow_delegated_pull = true` with
   287	`allowed_source_hosts` naming the peer (each is destination in one
   288	direction); bench modules writable, `delegation_allowed` not narrowed.
   289	
   290	### D5 — three self-contained scripts; the frozen baselines stay frozen
   291	
   292	`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
   293	`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
   294	precedent: duplicate the shape, don't refactor recorded evidence;
   295	`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
   296	old scripts, both recorded sharp edges:
   297	
   298	- **Exit codes are checked**: the old harnesses swallow the blit exit code
   299	  inside the timed window; otp-12 records it per run (`exit` column) and a
   300	  nonzero exit voids the interleave pair per the D2 valid-run rule — a
   301	  failed transfer must never contribute a time.
   302	- **Multi-token flags ride an array**, not an unquoted scalar.
   303	
   304	CSV schema (all rigs):
   305	`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
   306	(`valid` = the PAIR's fate under the D2 valid-run rule — an
   307	individually-clean run whose partner voided reads `no`; amended at the
   308	12a harness slice)
   309	`summary.csv`:
   310	`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
   311	(medians over valid runs only — the D2 valid-run rule)
   312	`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
   313	where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
   314	otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
   315	reference CSVs; corrected at the 12a review, codex F9),
   316	`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
   317	direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
   318	discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
   319	self-adjudicated; added at the 12b harness slice), `arm` ∈
   320	`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
   321	`initiator` = host name, `kind` ∈
   322	`converge|invariance|delegated|cross|cross-gap`.
   323	Verdict outcome vocabulary (closed; 12b review, codex F12): per-reference
   324	rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
   325	carries the registered D2 set
   326	(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
   327	`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
   328	converge row whose same-session block-1 counterpart is absent or
   329	incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
   330	artifact — the committed-reference row still governs). Nothing else is
   331	legal, and a missing committed-reference row aborts the verdict pass
   332	(fail closed).
   333	
   334	Fixtures: identical shapes to otp-2 (1 GiB large / 10k×4 KiB small /
   335	512 MiB+5k×2 KiB mixed), generated with the existing recipes (BSD vs GNU
   336	`dd` block-size spelling handled per host), staged untimed; pull sources
   337	shared across arms (bytes are bytes — recorded explicitly); every timed
   338	destination is fresh and never-seen (`SESSION_TAG` + arm + run in the
   339	path).
   340	
1:# otp-2w — OLD-path baseline on the owner-designated cross-direction rig (2026-07-10)
10:The zoey dataset (`docs/bench/otp2-baseline-2026-07-10/`) remains the
29:  `scripts/bench_otp2w_baseline.sh` + `scripts/windows/purge-standby.ps1`):
93:RUNS=4 ./scripts/bench_otp2w_baseline.sh
     1	# otp-2w — OLD-path baseline on the owner-designated cross-direction rig (2026-07-10)
     2	
     3	**Status**: Recorded. This is the rig the owner designated for the
     4	otp-12 acceptance bar's **cross-direction half** after the Mac↔zoey
     5	session established (per D-2026-07-05-1's symmetric-endpoint rule)
     6	that hardware-asymmetric pairs support per-direction verdicts only —
     7	owner: "mac to windows would be closer spec. windows is faster, both
     8	have 10gbe." Closer-spec is the owner's designation, not a claim of
     9	identical platforms: APFS and NTFS write paths differ (see Readings).
    10	The zoey dataset (`docs/bench/otp2-baseline-2026-07-10/`) remains the
    11	per-direction reference for the slow-pool rig.
    12	
    13	**Build**: `0f922de` binaries both ends — client macOS arm64 release;
    14	daemon built natively on the host (source delivered as a git bundle —
    15	the commits were unpushed and pushes are owner-gated; a bundle is a
    16	plain file copy between the owner's machines). The recorded run used
    17	the harness as of the codex-fix round.
    18	
    19	## Rig
    20	
    21	- **Client**: the owner's Mac (Apple Silicon, APFS NVMe SSD), data in
    22	  `~/blit-bench-work`.
    23	- **Daemon host**: Windows 11 (10.0.26200), Ryzen 9 9950X3D
    24	  (32 threads), 96 GiB RAM, module root on `D:` (PCIe Gen5 NVMe,
    25	  Crucial T705). Repo at `F:\dev\blit_v2`; everything the bench
    26	  writes lives under the owner-designated `D:\blit-test`.
    27	- **Link**: Thunderbolt 10GbE (Mac) ↔ 10 Gbps NIC (host), ~0.4 ms.
    28	- **Host plumbing** (first-of-kind on Windows, embodied in
    29	  `scripts/bench_otp2w_baseline.sh` + `scripts/windows/purge-standby.ps1`):
    30	  OpenSSH with PowerShell 7 default shell (multiplexed —
    31	  ControlMaster); daemon launched via WMI `Win32_Process.Create`
    32	  because Windows OpenSSH kills session children on disconnect
    33	  (reproduced live); launch REFUSES over a stale daemon and teardown
    34	  kills the recorded PID only; cold caches = standby-list purge
    35	  (`NtSetSystemInformation`, admin, every API step checked); durable
    36	  pushes = self-timed `Write-VolumeCache D`; drain = `Get-Counter`
    37	  PhysicalDisk write bytes/sec, three consecutive quiet 2 s samples,
    38	  failed probes warn rather than pass; ONE program-scoped inbound
    39	  firewall rule (`blit-bench-daemon`; remove with
    40	  `Remove-NetFirewallRule -DisplayName blit-bench-daemon`). Config
    41	  paths are TOML LITERAL strings — double-quoted TOML corrupts
    42	  Windows paths (`\b` is an escape).
    43	
    44	## Verdict-cell results (median of 4 cold, drained, durable runs; ms)
    45	
    46	| fixture | push tcp | push grpc | pull tcp | pull grpc |
    47	|---------|---------:|----------:|---------:|----------:|
    48	| large (1 GiB)            | 3054 | 3065 | 1294 | 1289 |
    49	| small (10k × 4 KiB)      | 1868 | 2822 | 1280 | 1462 |
    50	| mixed (512 MiB + 5k×2K)  | 2288 | 2687 | 1284 | 1408 |
    51	
    52	Per-run data: `runs.csv`; `drain-outcomes.txt` shows zero anomalies.
    53	Stability: per-cell (max−min)/min spreads 0.2–14.5%; 4 cells ≤ 2%,
    54	9 cells ≤ 9%. Rounding: integer ms; even-count median = floor of the
    55	mean of the middle two.
    56	
    57	## Readings (recorded, not adjudicated)
    58	
    59	- Pull ≈ 6.6 Gbit/s durable on the 1 GiB cell; push ≈ 2.8 Gbit/s.
    60	  **Old push trails old pull ×1.46–×2.38 per cell on this
    61	  close-spec pair** (large 2.36, small 1.46, mixed 1.78 on TCP).
    62	- On the large fixture the carrier makes NO difference in either
    63	  direction (push 3054 vs 3065; pull 1294 vs 1289) — the wire is not
    64	  the bottleneck; the ceilings are the endpoint read/write paths.
    65	- Whether the push gap is Windows write-path cost (NTFS, Defender
    66	  real-time scanning — left at its normal state) or the old
    67	  push-receive code is exactly what otp-12's interleaved old-vs-new
    68	  discriminates: if the unified path's push closes toward pull, it
    69	  was the code (the plan's founding bet, D-2026-07-05-1); if not,
    70	  the residue is the platform write path, measurable as the same gap
    71	  in both arms. Interleaved A/B keeps the Defender state identical
    72	  across arms.
    73	
    74	## Timing-overhead correction (probe1)
    75	
    76	The first recorded session (`probe1-sshoverhead-{runs,summary}.csv`)
    77	wrapped `ssh host Write-VolumeCache` inside the timed window; a
    78	per-connection cost of ~0.5 s (plus pwsh spawn and module load,
    79	~1.2 s total measured idle) landed on every PUSH window and none of
    80	the pull windows, inflating push medians by ~0.5–0.6 s and the

exec
/bin/zsh -lc "nl -ba docs/DECISIONS.md | sed -n '104,116p;124,148p;164,177p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   104	## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
   105	- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
   106	- Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
   107	- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.
   108	
   109	## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
   110	- Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
   111	- Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
   112	- Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).
   113	
   114	## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
   115	- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
   116	- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
   124	## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
   125	- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
   126	- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
   127	- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).
   128	
   129	## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
   130	- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
   131	- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
   132	- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.
   133	
   134	## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
   135	- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
   136	- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
   137	- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).
   138	
   139	## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
   140	- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
   141	- Why: the 10 GbE session showed skippy's 32-core receiver at 1.43 cores — gate not met on that rig — but the gate was always about CPU-bound receivers, and the owner now operates one. On a CPU-bound receiver, cutting the userspace copy is exactly the FAST lever the eval preserved design notes for.
   142	- Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.
   143	
   144	## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
   145	- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
   146	- Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
   147	- Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).
   148	
   164	## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
   165	- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
   166	- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
   167	- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).
   168	
   169	## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
   170	- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
   171	- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
   172	- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
   173	
   174	## D-2026-07-13-1 — OTP12_PERF_FINDINGS goes Active after one final codex round; implementation proceeds slice-by-slice
   175	- Decision: `docs/plan/OTP12_PERF_FINDINGS.md` flips **Draft → Active** after ONE final codex round, and implementation then proceeds regardless of whether that round returns a "converged" verdict — owner, 2026-07-13, verbatim: **"one more round with codex on the plan then just write the code and reviewloop slice by slice. that converges faster than plans with no ground truth to test."** Each code slice still goes through the codex review loop (D-2026-07-04-1, unchanged); what is retired is *plan-only* iteration as the gate on starting work. The plan's own Status line ("the flip to Active happens at codex convergence") is amended by this decision: the round happens, its accepted findings are fixed, and then code starts — a non-converged verdict is no longer a blocker, it is input to the first slice.
   176	- Why: rounds 2–4 each returned real findings, but they were increasingly findings about the *plan text* (falsifiability wording, thresholds, bar phrasing) rather than about reality, and the plan's central factual claim was settled not by review but by *measurement* — the same-OS rig, which refuted a claim four review rounds had left standing (`docs/bench/otp12-perf-2026-07-13/`; a wrong "P1 is code" claim was reported and retracted the same day). Ground truth comes from instrumented code and rigs, not from more prose; pf-1 exists precisely to generate it. Continuing to polish the plan has diminishing returns against the cost of not yet having a single measured counterfactual.
   177	- Supersedes: the "flip to Active at codex convergence" gate in `OTP12_PERF_FINDINGS.md`'s Status line (rewritten in place, same commit). Does NOT supersede D-2026-07-04-1 — every code slice is still codex-reviewed before the next begins.

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '398,466p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   398	# --- Fixtures (shape-verified; the otp-12a F2 rule) ----------------------
   399	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   400	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   401	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   402	fixture_shape() {
   403	    find "$1" -type f -exec stat -f%z {} + 2>/dev/null \
   404	        | awk '{ s += $1 } END { printf "%d,%d\n", NR, s }'
   405	}
   406	verify_fixture() {
   407	    local w="$1" want_count want_bytes got
   408	    want_count=$(eval echo "\$FIX_COUNT_$w")
   409	    want_bytes=$(eval echo "\$FIX_BYTES_$w")
   410	    got=$(fixture_shape "$MAC_WORK/src_$w")
   411	    [[ "$got" == "$want_count,$want_bytes" ]] \
   412	        || die "fixture src_$w has shape $got, want $want_count,$want_bytes — remove $MAC_WORK/src_$w and re-run"
   413	}
   414	gen_fixtures() {
   415	    if [[ ! -d "$MAC_WORK/src_large" ]]; then
   416	        mkdir -p "$MAC_WORK/src_large"
   417	        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
   418	    fi
   419	    if [[ ! -d "$MAC_WORK/src_small" ]]; then
   420	        mkdir -p "$MAC_WORK/src_small"
   421	        for i in $(seq 1 10000); do
   422	            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
   423	            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
   424	        done
   425	    fi
   426	    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
   427	        mkdir -p "$MAC_WORK/src_mixed"
   428	        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
   429	        for i in $(seq 1 5000); do
   430	            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
   431	            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
   432	        done
   433	    fi
   434	    local w
   435	    for w in large small mixed; do verify_fixture "$w"; done
   436	    log "fixtures verified (count + byte sum)"
   437	}
   438	
   439	win_module_count() {   # $1 = subpath under the module; prints file count
   440	    wssh "(Get-ChildItem -Path '$WIN_MODULE\\$1' -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count" | tr -cd '0-9'
   441	}
   442	stage_pull_sources() {
   443	    # Shared across arms by design (D5); verified by remote file count;
   444	    # staged with the NEW pair; the same trees serve block 1 pulls and
   445	    # block 2 win_init pushes (one physical source per direction, F6).
   446	    log "staging pull sources on the Windows module (untimed, new pair)"
   447	    win_ensure new
   448	    local w want got
   449	    for w in large small mixed; do
   450	        want=$(eval echo "\$FIX_COUNT_$w")
   451	        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
   452	        if [[ "$got" == "$want" ]]; then
   453	            log "  pull_src_$w verified ($got files, kept)"
   454	            continue
   455	        fi
   456	        log "  pull_src_$w has $got/$want files — (re)staging"
   457	        "$NEW_BLIT" copy "$MAC_WORK/src_$w" "${WIN_REMOTE}pull_src_$w/" --yes \
   458	            > /dev/null 2> "$OUT_DIR/blit-logs/stage_$w.err" \
   459	            || die "staging pull_src_$w failed"
   460	        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
   461	        [[ "$got" == "$want" ]] || die "pull_src_$w still wrong after staging ($got/$want)"
   462	        log "  staged pull_src_$w ($got files)"
   463	    done
   464	}
   465	
   466	# --- Timed runs -----------------------------------------------------------

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '440,535p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   440	        }
   441	
   442	        // Buffers return to pool automatically on drop
   443	        Ok(())
   444	    }
   445	
   446	    pub async fn send_prepared_tar_shard(
   447	        &mut self,
   448	        headers: Vec<FileHeader>,
   449	        data: &[u8],
   450	    ) -> Result<()> {
   451	        let shard_len = headers.len();
   452	        let preview = headers
   453	            .first()
   454	            .map(|h| h.relative_path.as_str())
   455	            .unwrap_or("<empty>");
   456	        trace_client!(
   457	            self,
   458	            "sending tar shard with {} file(s), {} bytes (first='{}')",
   459	            shard_len,
   460	            data.len(),
   461	            preview
   462	        );
   463	        self.stream
   464	            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
   465	            .await
   466	            .context("writing tar shard record tag")?;
   467	        self.stream
   468	            .write_all(&(headers.len() as u32).to_be_bytes())
   469	            .await
   470	            .context("writing tar shard count")?;
   471	
   472	        for header in headers {
   473	            let rel_bytes = header.relative_path.as_bytes();
   474	            if rel_bytes.len() > u32::MAX as usize {
   475	                bail!(
   476	                    "relative path too long for transfer: {}",
   477	                    header.relative_path
   478	                );
   479	            }
   480	            self.stream
   481	                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
   482	                .await
   483	                .context("writing shard path length")?;
   484	            self.stream
   485	                .write_all(rel_bytes)
   486	                .await
   487	                .context("writing shard path bytes")?;
   488	            self.stream
   489	                .write_all(&header.size.to_be_bytes())
   490	                .await
   491	                .context("writing shard size")?;
   492	            self.stream
   493	                .write_all(&header.mtime_seconds.to_be_bytes())
   494	                .await
   495	                .context("writing shard mtime")?;
   496	            self.stream
   497	                .write_all(&header.permissions.to_be_bytes())
   498	                .await
   499	                .context("writing shard permissions")?;
   500	        }
   501	
   502	        self.stream
   503	            .write_all(&(data.len() as u64).to_be_bytes())
   504	            .await
   505	            .context("writing tar shard length")?;
   506	        for chunk in data.chunks(self.chunk_bytes.max(1)) {
   507	            // codex ue-r2-1e F3: shard writes carry the small-file
   508	            // workloads — without a blocked signal here the tuner sees
   509	            // a saturated link as a clean one. Same P::ACTIVE gating as
   510	            // the file loop: NoProbe reads no clock.
   511	            let started = if P::ACTIVE {
   512	                Some(std::time::Instant::now())
   513	            } else {
   514	                None
   515	            };
   516	            self.stream
   517	                .write_all(chunk)
   518	                .await
   519	                .context("writing tar shard payload")?;
   520	            if let Some(t) = started {
   521	                self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   522	            }
   523	            self.probe.record_bytes(chunk.len() as u64);
   524	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
   525	        }
   526	        trace_client!(
   527	            self,
   528	            "tar shard payload sent ({} file(s), {} bytes)",
   529	            shard_len,
   530	            data.len()
   531	        );
   532	
   533	        Ok(())
   534	    }
   535	

codex
- **BLOCKER — The run does not isolate MTU.** The missing condition is correctly identified: 12b used Aquantia/MTU 1500 but `e21cf84` ([machines.md:102](/Users/michael/Dev/blit_v2/.agents/machines.md:102), [12b summary.csv:46](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/summary.csv:46)); 12c used `f35702a`/MTU 1500 but the TB5 dock ([12c README.md:4](/Users/michael/Dev/blit_v2/docs/bench/otp12c-win-2026-07-13/README.md:4), [summary.csv:46](/Users/michael/Dev/blit_v2/docs/bench/otp12c-win-2026-07-13/summary.csv:46)). Thus `Aquantia × 1500 × f35702a` is the right control ([PREREGISTRATION.md:68](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:68)), but it is required after every outcome for causal attribution—not only PASS. A jumbo FAIL proves only that P1 persists; INDETERMINATE+RUNS=8 still lacks an MTU counterfactual. Moreover, the proposed three-cell control conflicts with the document’s own full-matrix/order rationale ([PREREGISTRATION.md:147](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:147), [PREREGISTRATION.md:165](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:165)); compare identical subsets or full matrices at both MTUs.

- **BLOCKER — The `r ≥ 1.20` causal rejection is invalid.** The premise is that MTU causes “some or all” of the gap ([PREREGISTRATION.md:24](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:24)). With `win_init=939`, `r=1.20` permits `mac_init≈1127`, a 94/282 = 33% recovery—**CONFIRMED CONTRIBUTING** under the parent’s 20–50% rule, not “MTU is not the cause” ([OTP12_PERF_FINDINGS.md:522](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:522)). That band can say only “jumbo is insufficient to dissolve P1.”

- **HIGH — RUNS=4 and the uncertainty band cannot support definitive calls.** The claimed 5% “drift” comes from sessions where both SHA and NIC changed ([PREREGISTRATION.md:59](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:59)); it is not a noise estimate. The parent defines P1’s bar at RUNS=8 ([OTP12_PERF_FINDINGS.md:548](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:548)), while the design excludes MTU at four samples. The harness also prints three-decimal ratios but adjudicates 1.10 with exact integer arithmetic ([bench_otp12_win.sh:668](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:668), [bench_otp12_win.sh:714](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:714)); band boundaries must specify raw-median arithmetic.

- **BLOCKER — The masking guards admit the masking artifact.** A shared 1000 ms floor passes all three tests: ratio 1.0, fast arm `1000≤1033`, slow arm `1000≤1024` ([PREREGISTRATION.md:130](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:130)). The 70% calculation is also not the parent’s gap closure: `mac=1024`, `win=931` passes every guard, but leaves a 93 ms gap—only `(282−93)/282=67%` closure. The parent defines Δ as the arm difference ([OTP12_PERF_FINDINGS.md:501](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:501)), not a slow-arm-only threshold.

- **BLOCKER — MSS validates capability, not Blit’s treatment.** The probe sockets establish an MSS ceiling/path capability, not the claimed measured 6.18× reduction in Blit segment count ([PREREGISTRATION.md:41](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:41)). They do not establish Blit-socket MSS on every timed stream, absence of carrier fallback, actual payload-segment sizes, or on-wire counts under application write boundaries and offload. The harness records none of those; it merely leaves TCP as default ([bench_otp12_win.sh:783](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:783)). Discarding the coalesced NIC counter is correct, but no actual-segment instrument replaces it. Likewise, MSS 8948 with unchanged wall time is a valid “per-packet cost is irrelevant” null—not an instrument contradiction ([PREREGISTRATION.md:107](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:107)).

- **HIGH — The packet-load premise and positive control are mis-specified.** The harness defines large as 1 GiB, mixed as 547 MB/5001 files, and small as 41 MB/10,000 files ([bench_otp12_win.sh:398](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:398)); mixed is not demonstrably the “packet-heaviest” fixture. The vague requirement that “mixed, small” speed up “somewhere” supplies neither cells nor thresholds and omits the larger bulk-packet control, allowing post-hoc selection across 24 cells.

- **HIGH — The void-row inventory is incomplete.** Invariance and same-session rows are baseline-independent, and explicit `old_committed`/`min_old_committed` rows do consume the fixed baseline ([bench_otp12_win.sh:693](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:693), [bench_otp12_win.sh:711](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:711), [bench_otp12_win.sh:723](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:723)). But every block-1 `combined` verdict also embeds the committed result ([bench_otp12_win.sh:697](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:697)); 12b P2 says `FAIL-BOTH`, while 12c says `FAIL-SAME-SESSION` solely because the committed leg changed ([12b verdicts.csv:35](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/verdicts.csv:35), [12c verdicts.csv:35](/Users/michael/Dev/blit_v2/docs/bench/otp12c-win-2026-07-13/verdicts.csv:35)). Those combined rows must also be quarantined. The stated P2 consequence is otherwise correct: under this void premise, jumbo can establish only its same-session leg, while the parent requires both references ([OTP12_PERF_FINDINGS.md:553](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:553)).

- **NOTE — Running the experiment itself does not violate governance.** It lands no fix, changes no wire contract, and STATE explicitly sequences it before code ([docs/STATE.md:5](/Users/michael/Dev/blit_v2/docs/STATE.md:5), [OTP12_PERF_FINDINGS.md:32](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:32), [OTP12_PERF_FINDINGS.md:579](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:579)). A PASS licenses evidence for a plan amendment only; it cannot skip or reshape pf-1, rebaseline, or close P1/P2 without that reviewed amendment. The final disclaimer correctly denies direct acceptance ([PREREGISTRATION.md:156](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:156)).

**VERDICT: NOT READY — blockers are the unmatched/conditional MTU control, invalid causal kill band, porous masking guards, and unverified Blit-level treatment/null rule.**
tokens used
160,727
- **BLOCKER — The run does not isolate MTU.** The missing condition is correctly identified: 12b used Aquantia/MTU 1500 but `e21cf84` ([machines.md:102](/Users/michael/Dev/blit_v2/.agents/machines.md:102), [12b summary.csv:46](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/summary.csv:46)); 12c used `f35702a`/MTU 1500 but the TB5 dock ([12c README.md:4](/Users/michael/Dev/blit_v2/docs/bench/otp12c-win-2026-07-13/README.md:4), [summary.csv:46](/Users/michael/Dev/blit_v2/docs/bench/otp12c-win-2026-07-13/summary.csv:46)). Thus `Aquantia × 1500 × f35702a` is the right control ([PREREGISTRATION.md:68](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:68)), but it is required after every outcome for causal attribution—not only PASS. A jumbo FAIL proves only that P1 persists; INDETERMINATE+RUNS=8 still lacks an MTU counterfactual. Moreover, the proposed three-cell control conflicts with the document’s own full-matrix/order rationale ([PREREGISTRATION.md:147](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:147), [PREREGISTRATION.md:165](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:165)); compare identical subsets or full matrices at both MTUs.

- **BLOCKER — The `r ≥ 1.20` causal rejection is invalid.** The premise is that MTU causes “some or all” of the gap ([PREREGISTRATION.md:24](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:24)). With `win_init=939`, `r=1.20` permits `mac_init≈1127`, a 94/282 = 33% recovery—**CONFIRMED CONTRIBUTING** under the parent’s 20–50% rule, not “MTU is not the cause” ([OTP12_PERF_FINDINGS.md:522](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:522)). That band can say only “jumbo is insufficient to dissolve P1.”

- **HIGH — RUNS=4 and the uncertainty band cannot support definitive calls.** The claimed 5% “drift” comes from sessions where both SHA and NIC changed ([PREREGISTRATION.md:59](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:59)); it is not a noise estimate. The parent defines P1’s bar at RUNS=8 ([OTP12_PERF_FINDINGS.md:548](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:548)), while the design excludes MTU at four samples. The harness also prints three-decimal ratios but adjudicates 1.10 with exact integer arithmetic ([bench_otp12_win.sh:668](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:668), [bench_otp12_win.sh:714](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:714)); band boundaries must specify raw-median arithmetic.

- **BLOCKER — The masking guards admit the masking artifact.** A shared 1000 ms floor passes all three tests: ratio 1.0, fast arm `1000≤1033`, slow arm `1000≤1024` ([PREREGISTRATION.md:130](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:130)). The 70% calculation is also not the parent’s gap closure: `mac=1024`, `win=931` passes every guard, but leaves a 93 ms gap—only `(282−93)/282=67%` closure. The parent defines Δ as the arm difference ([OTP12_PERF_FINDINGS.md:501](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:501)), not a slow-arm-only threshold.

- **BLOCKER — MSS validates capability, not Blit’s treatment.** The probe sockets establish an MSS ceiling/path capability, not the claimed measured 6.18× reduction in Blit segment count ([PREREGISTRATION.md:41](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:41)). They do not establish Blit-socket MSS on every timed stream, absence of carrier fallback, actual payload-segment sizes, or on-wire counts under application write boundaries and offload. The harness records none of those; it merely leaves TCP as default ([bench_otp12_win.sh:783](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:783)). Discarding the coalesced NIC counter is correct, but no actual-segment instrument replaces it. Likewise, MSS 8948 with unchanged wall time is a valid “per-packet cost is irrelevant” null—not an instrument contradiction ([PREREGISTRATION.md:107](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:107)).

- **HIGH — The packet-load premise and positive control are mis-specified.** The harness defines large as 1 GiB, mixed as 547 MB/5001 files, and small as 41 MB/10,000 files ([bench_otp12_win.sh:398](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:398)); mixed is not demonstrably the “packet-heaviest” fixture. The vague requirement that “mixed, small” speed up “somewhere” supplies neither cells nor thresholds and omits the larger bulk-packet control, allowing post-hoc selection across 24 cells.

- **HIGH — The void-row inventory is incomplete.** Invariance and same-session rows are baseline-independent, and explicit `old_committed`/`min_old_committed` rows do consume the fixed baseline ([bench_otp12_win.sh:693](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:693), [bench_otp12_win.sh:711](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:711), [bench_otp12_win.sh:723](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:723)). But every block-1 `combined` verdict also embeds the committed result ([bench_otp12_win.sh:697](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:697)); 12b P2 says `FAIL-BOTH`, while 12c says `FAIL-SAME-SESSION` solely because the committed leg changed ([12b verdicts.csv:35](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/verdicts.csv:35), [12c verdicts.csv:35](/Users/michael/Dev/blit_v2/docs/bench/otp12c-win-2026-07-13/verdicts.csv:35)). Those combined rows must also be quarantined. The stated P2 consequence is otherwise correct: under this void premise, jumbo can establish only its same-session leg, while the parent requires both references ([OTP12_PERF_FINDINGS.md:553](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:553)).

- **NOTE — Running the experiment itself does not violate governance.** It lands no fix, changes no wire contract, and STATE explicitly sequences it before code ([docs/STATE.md:5](/Users/michael/Dev/blit_v2/docs/STATE.md:5), [OTP12_PERF_FINDINGS.md:32](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:32), [OTP12_PERF_FINDINGS.md:579](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:579)). A PASS licenses evidence for a plan amendment only; it cannot skip or reshape pf-1, rebaseline, or close P1/P2 without that reviewed amendment. The final disclaimer correctly denies direct acceptance ([PREREGISTRATION.md:156](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:156)).

**VERDICT: NOT READY — blockers are the unmatched/conditional MTU control, invalid causal kill band, porous masking guards, and unverified Blit-level treatment/null rule.**
