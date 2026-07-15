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
session id: 019f65b5-5358-7811-a34f-aae8e9d460e8
--------
user
Review the complete immutable diff 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946 for otp12-pf1-rigw-harness. This is a high-stakes q to netwatch-01 benchmark instrument governed by docs/plan/OTP12_PERF_FINDINGS.md and .review/findings/otp12-pf1-rigw-harness.md. Inspect the actual full diff/current source, not only the last commit. Check material correctness and regressions: fixed 128-arm OFF-ON-ON-OFF schedule, exactly one Transfer RPC per arm, SOURCE always sends and DESTINATION always receives under either caller, role-independent worker targets, identical endpoint-local physical paths across initiator roles, q-arrival settle anchoring with teardown charged, destination-keyed durability, exact evidence/analyzer rules, launcher smoke, CIM PID journaling/identity recovery, fail-closed cleanup/failure preservation, and observe/refuse-only endpoint policy. Reconfirm G5's registered-interface ARP parser. Focus on round-5 accepted F1/G6: the Windows standby purge helper must come from the exact reviewed clean checkout, stage only after read-only endpoint gates into a fresh per-session tree, reject reparse points, verify SHA-256 after copy/atomic move and immediately before every arm, require the exact success sentinel, appear in the staging manifest, and be removed by strict cleanup. Its Bash 3.2 offline mocks/order guard must fail when post-stage or per-arm verification is removed. Look for quoting, SCP/PowerShell, lifecycle, manifest, TOCTOU, or mode-flow defects that could admit warm-cache or unreviewed behavior. Distinguish material observable defects from style. Output a concise markdown findings list with file:line, severity, predicted failure, and rationale, then exactly one final VERDICT: PASS or VERDICT: NEEDS FIXES. A no-finding PASS is valid. Do not invoke skills, mutate files, use network, or touch live endpoints.
codex
I’ll review the immutable range directly, then trace each benchmark invariant into the resulting source and tests. I’ll keep all actions read-only and offline, with no endpoint access or repository changes.
exec
/bin/zsh -lc "pwd; sed -n '1,260p' AGENTS.md; sed -n '1,280p' .agents/repo-guidance.md; sed -n '1,260p' .agents/state.md; git status --short; git rev-parse --verify 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d"'^{commit}; git rev-parse --verify 75a9a33ce600e4707438ed885de2ce0cdf27d946''^{commit}; git diff --stat 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946; git diff --name-status 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/Users/michael/Dev/blit_v2
# Agent Guidance

## Prime Invariants
<!-- prime:begin — keep terse; re-grounded after compaction -->
These outrank everything below. After a context compaction, re-read this block from AGENTS.md before continuing.

- Words first. Answer questions and musings in words; act only on an explicit instruction or go. A handed-over report, plan, or spec is evidence to assess, not a decision to implement.
- No code change without an approved plan; docs and other non-code edits don't need one (e.g. a README). When unsure, treat it as code. Do not expand scope without approval.
- Commit each slice as it lands; never leave finished work uncommitted. History-rewrite and destructive or outward-facing actions always need an explicit go. Push policy: see `.agents/push-policy.md`.
- Repo is memory. Durable truth lives in the repo, not chat or working memory. Under context pressure, re-ground from AGENTS.md; prefer a fresh session when degraded.
<!-- prime:end -->

## Repo-Specific Guidance

@.agents/repo-guidance.md

Repo-specific rules live in `.agents/repo-guidance.md`, imported above (read it directly if your harness does not process `@` imports). It extends this file and never overrides it — flag any genuine conflict.

## Universal Invariants

- The Prime Invariants above are the hardest-to-reverse rules; this section adds the rest.
- Agent-local or harness-local memory stores kept outside the repo are not durable memory, on any harness. Persist project-specific durable knowledge into the repo's `.agents/` files; reserve out-of-repo stores for genuinely cross-project facts (owner identity, preferences).
- Record important repo facts, decisions, invariants, verification rules, non-goals, and open questions in repo files, or explicitly report them as unrecorded. Write them generalized, tied to repo evidence or explicit human intent, so they make sense without the conversation that produced them — never as transient chat wording. Label inferred-but-unverified facts as assumptions until repo evidence or explicit human approval supports them.
- Keep one canonical location for each durable truth. Prefer pointers over duplicating the same rule; never keep a second copy of a count or enumeration another doc owns.
- Establish one immediately discoverable current-state entry point (`.agents/state.md`). Do not reconstruct current state from chat, long journals, or tool-local memory.
- When repo documents disagree, flag the conflict instead of silently choosing whichever source is convenient. Code and tests are evidence for behavior; approved plans and guidance are evidence for intent.
- Specific over generic: an explicit authority or scope boundary, or a rule or decision whose wording removes discretion for the case it names ("unconditional", "no per-run choice", "deterministic"), outranks every generic default for that case — flag-conflicts, one-canonical-location, smallest-guidance-set included. Apply it as written; do not reopen the case it settles as a conflict or approval question against surrounding repo state such as git history. Generic defaults govern only questions no more specific rule has already resolved.
- Prefer the smallest durable guidance set that fits the repo.
- Do not circumvent a roadblock whose provenance you have not established — a failing test, a guard or assertion, a lint or type error, a `.gitignore` rule, a refusal or permission denial, a config prohibition, a CI gate. Before removing or bypassing one, inspect its origin thoroughly enough to confirm it is not load-bearing; if you cannot, treat it as legitimate and stop or ask.
- Escalate an iterative process on stalled progress, never on duration. Each cycle must bank a verifiable delta — a test moving red→green, a finding closed with its guard proof, a build or type error resolved, a committed slice; a cycle that produces none is a stall. After a few consecutive stalled cycles (state the threshold you are using; default ~2-3), stop and surface to a human. A long run that banks a delta each cycle is healthy and must not be capped on duration or turn count.
- `AGENTS.md` is governance only — it must be portable. The test: would this line still be true and useful if copied unchanged into an unrelated repo? Process, invariants, and operator definitions pass. Anything true only of *this* repo — a concrete source path, the repo's own name as a fact, its verification commands, a restatement of current state or the decisions queue — fails and lives in `.agents/`, with `AGENTS.md` pointing to it, never restating it. References to the toolkit's own standard layout — `.agents/state.md`, operator names — are portable and allowed.
- `AGENTS.md` is the toolkit template, installed and replaced whole by governance refresh; no agent hand-edits it. Durable repo-specific rules go to `.agents/repo-guidance.md` and facts to the other `.agents/` files; a proposed `AGENTS.md` edit is out of bounds — question it, do not perform it.

## Session Startup

1. Read `AGENTS.md`, `.agents/repo-guidance.md`, and `.agents/state.md` if present, plus relevant `.agents/` files, before making changes; note any untracked or ignored agent-control files that affect the task.
2. Clone freshness: before trusting `.agents/state.md`, compare this clone against its canonical remote with a read-only check (`git ls-remote <remote> HEAD` against the local ref). Behind or diverged — say so and treat recorded state as possibly stale; unreachable — proceed with a one-line caveat, never block.
3. This repo ships a compaction re-ground hook (Claude Code; other harnesses only as listed in the toolkit's harness-capabilities record); if your harness gates hooks until the workspace is trusted, say what the hook does and run the trust step only on an explicit go — never bypass the gate.

## Source Of Truth

1. Human request.
2. `AGENTS.md`, extended by `.agents/repo-guidance.md` (extends, never overrides).
3. `.agents/state.md` for current work; `.agents/decisions.md` for settled decisions; approved `.agents/playbooks/*`.
4. Current code, tests, and CI as evidence for behavior.
5. Existing docs, only when consistent with current repo evidence.

When sources disagree, apply the flag-conflicts invariant (Universal Invariants): surface the conflict and fix the lower-authority source, or ask which should win.

## Operator Requests

Treat these owner words as process requests:

- `catchup`: re-read `AGENTS.md` (the Prime Invariants in full), `.agents/state.md`, and active repo docs; summarize current state, next action, blockers, and one proposed first action. Make no changes until the human responds.
- `handoff`: a fast save-my-place snapshot — seconds, not minutes. Update `.agents/state.md` `## Now` / `## Next` (and `## Blockers` if something is live) so the next session resumes without chat context: in-flight work, next action, stop. No archive rotation, no re-verification sweep, no mandatory re-anchoring of volatile facts — that hygiene belongs to `drift`. Machine-specific facts (CLI paths, local tool versions, host layout) go to the tracked `.agents/machines.md` under a heading for the current machine, dated, created on first use — never into `.agents/state.md`, which stays portable and may at most point there.
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves — `AGENTS.md` and `.agents/*` — are in scope as drift targets, not just sources of truth. `drift` also owns the deliberate state-hygiene pass: rotate landed or superseded `## Now` entries verbatim to `docs/history/state-archive.md` (create on first use); re-verify the recorded basis of every parked or blocked item and move anything falsified into `## Blockers` with the new evidence; volatile facts (CI state, counts) carry `as of <commit>` and are re-verified or dropped; push status is never recorded in state files — git owns it, sessions check it live, and unpushed work is mentioned only in the moment it matters — so any recorded push-state line is deleted on sight, not refreshed; a count or enumeration another file owns is pointed to, never copied; machine-specific facts relocate to `.agents/machines.md`, and stale entries there are pruned.
- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
- `plan`: draft or update a durable plan before broad implementation work. Plan documents are written for agents, never the owner: self-contained and technical, implementable by a completely cold, less-capable agent — no human-facing summary prose, no chat or session references that need the originating conversation to make sense. The owner does not read plan documents; present every decision a plan needs in chat as roughly 25-50 plain-English words — the problem, the change, the cost or risk — one decision at a time, never a batch, no jargon. Record the owner's approved wording durably (the decisions log, the plan's status line) so the approval survives the chat.
- `playbook <name>`: read `.agents/playbooks/<name>.md` and follow it. Playbooks are approved durable workflows; this operator is how a session invokes one by name. If the named playbook does not exist, say so rather than guessing.

## Verification

Use the repo's current automated verification entry point recorded in `.agents/repo-guidance.md` (Verification).

- For code changes, run the current automated verification before claiming completion.
- When a change ships with a new test, prove the test guards it: temporarily revert the change, confirm the test fails, restore it, confirm everything passes. A test that passes with its fix reverted is vacuous and must be replaced.
- For docs-only changes, code verification is not required unless the docs affect setup, commands, runtime behavior, generated files, or user-visible behavior.
- For behavior that automation does not cover, run the relevant manual check, smoke test, or playtest, or state clearly that it was not run.
- If no verification entry point is recorded yet, identify the likely command from repo evidence, record it, and label uncertainty. Ask the human only when evidence conflicts, no plausible command exists, or the command appears destructive, expensive, credentialed, or otherwise unsafe to run automatically.

## Git Safety

- Never conclude a branch is merged from ancestry alone: `git branch --merged` can lie after an `-s ours` or octopus merge records ancestry without content. Verify the content actually arrived (`git diff <branch> <main>`) before deleting anything or treating work as landed.
- When working through a list of findings or fixes, address exactly one item per commit and commit each before starting the next. Batch sweeps spanning many findings happen only on the owner's explicit request. Whether work happens on a branch is this repo's policy, not this rule's.
- Do not rewrite history or restructure existing commits without explicit owner approval: no `git commit --amend`, `rebase`, `squash`, or force-push, and no reordering or collapsing commits already made. The owner's approval authorizes the scoped commit as announced — it does not authorize a later rewrite of it. Default to a new commit per fix; if history genuinely needs reshaping, stop and ask.

## Final Response

Explain what changed, what was validated, and any remaining risk in plain English.
# Repo-Specific Guidance
<!-- Extends AGENTS.md; never overrides it. Rules and pointers only — state
     lives in .agents/state.md. -->

## Mission Detail

Blit is a high-performance, extensible file enumeration, planning, transfer,
and orchestration platform for local and remote backups, migration, and
cross-platform syncing, with CLI and daemon interfaces (`crates/blit-cli`,
`crates/blit-daemon`), async-aware planning, and Windows/Linux/macOS support.

## Reading Order

This repo predates the toolkit's `.agents/state.md` / `.agents/decisions.md`
convention and keeps its own canonical files at different paths; the
`.agents/` files below are pointer stubs, not duplicates. Read in this order:

1. `docs/STATE.md` — single entry point for current active work, queue, and
   blockers (the canonical equivalent of `.agents/state.md`; see
   `.agents/state.md` for why the path differs).
2. The active plan doc(s) `docs/STATE.md` names (under `docs/plan/`).
3. `REVIEW.md` + `.review/` — review-loop status for in-flight findings.
4. `docs/DECISIONS.md` — settled decisions and supersessions (the canonical
   equivalent of `.agents/decisions.md`).
5. `docs/agent/PROTOCOL.md` — the executable procedures behind the trigger
   vocabulary (`catchup`, `plan`, `decision`, `handoff`, `drift`, plus the
   repo-specific `slice` operator below).
6. Everything else in `docs/` — reference or historical; check its
   `**Status**:` header.
7. Code and tests are ground truth for behavior; plans are ground truth for
   intent. A mismatch is a drift finding, not permission to pick whichever is
   convenient.

`DEVLOG.md` is append-only history — write to it, never read it for current
state. `TODO.md` is the long-horizon backlog; the actionable queue lives in
`docs/STATE.md` and `REVIEW.md`. `.serena/memories/` and any tool-local
memory are scratch, never authoritative.

## Operator Vocabulary (repo-specific extension)

`AGENTS.md`'s Operator Requests section defines the toolkit's generic
vocabulary (`catchup`, `handoff`, `drift`, `decision`, `plan`, `playbook`).
In this repo every one of those words resolves to a procedure in
`docs/agent/PROTOCOL.md`, not to the generic `.agents/state.md`/
`.agents/decisions.md` files directly — read the matching section there and
execute it exactly:

- `catchup` → re-ground from `docs/STATE.md` + active docs; summarize
  now/next/blockers.
- `plan <topic>` → interview the owner, write `docs/plan/<NAME>.md`; no code
  until `**Status**: Active`.
- `decision <topic>` → record in `docs/DECISIONS.md`, propagate
  supersessions.
- `handoff` → update `docs/STATE.md` for the next session; prune to caps.
- `drift [scope]` → audit a doc against code; fix docs, file findings, raise
  questions.
- `slice` (repo-specific, no generic-template equivalent) → pick up the next
  review finding and run it through the codex review loop
  (`docs/agent/GPT_REVIEW_LOOP.md`).

**Review policy (D-2026-07-04-1): every code change and every plan change
goes through the codex review loop in `docs/agent/GPT_REVIEW_LOOP.md` — no
exceptions.** The `.review/README.md` async sentinel hand-off is retired;
its `findings/`/`results/` records and `REVIEW.md` remain the record store.

Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
Antigravity exposes `catchup`/`handoff` as workspace skills in
`.agents/skills/`. This repo drafts `.agents/playbooks/reviewloop.md` as a template, but the codex review loop and `docs/agent/PROTOCOL.md` already cover that role for review-loop work.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Test count may grow but never drop versus the prior baseline unless the
  removal is called out in the finding doc's Known gaps.
- Windows parity: after touching platform-specific code (`win_fs`, planners),
  run `scripts/windows/run-blit-tests.ps1`.
- Docs gate (CI): a push touching `crates/**` or `proto/**` must also touch
  `docs/STATE.md`, unless the commit message contains `[state: skip]`
  (reserved for mechanical changes). `scripts/agent/check-docs.sh` must pass;
  run it locally before pushing docs changes.
- This section is the canonical home for the verification commands (the
  `.agents/repo-map.json` mirror was retired 2026-07-08 with the toolkit's
  JSON layer).

## Remotes & Sync

- `origin` — `http://q:3000/michael/blit_v2.git` (**LAN gitea**). This is
  what a bare `git push` / `git ls-remote origin` talks to.
- `github` — `https://github.com/roethlar/Blit.git` (**GitHub**).
- **`git push` does NOT update GitHub.** The two remotes are independent
  and nothing auto-syncs them: pushing `origin` moves the LAN gitea only,
  and GitHub needs its own explicit `git push github`. Either can lag the
  other by many commits. A ref-listing before a push must therefore name
  the remote's **URL**, not just "origin" — the name alone does not say
  which host is being published to.
- **CORRECTED 2026-07-13** against `git remote -v` (evidence in the
  session's push output: a bare `git push` printed
  `To http://q:3000/michael/blit_v2.git`, while `git push github` printed
  `To https://github.com/roethlar/Blit.git` and fast-forwarded GitHub
  `6d37a22..be1989d` — i.e. GitHub had been dozens of commits stale while
  every agent push went to the mirror). The prior text had the two names
  **inverted** (`origin` = GitHub, plus a `gitea` remote that does not
  exist) and asserted it had been "verified against `git remote -v`
  2026-07-04" while disparaging an earlier revision that was in fact
  correct. Do not re-invert this without re-reading `git remote -v`.
- Push policy: `.agents/push-policy.md` (ask). This repo's git-safety rules
  go well beyond a simple push policy — see Earned Practices below.

## Earned Practices

These are absolute; they exist because an unapproved `git merge -s ours`
octopus (commit `c793df2`) was pushed to `origin/master` without the owner's
consent (`docs/DECISIONS.md` D-2026-06-07-1).

- **No agent-created branches.** Agents never create git branches on their
  own decision. All work happens on `master` or the branch the owner already
  checked out.
- **Owner is the sole gate for git operations that publish, rewrite, or
  destroy.** No `push`, `push --force`/`--force-with-lease`,
  `reset --hard`, rebase or other history rewrite, `commit --amend` on
  pushed commits, or deletion of any branch/tag/ref (local or remote)
  without the owner approving that exact action in the current session.
  Working-tree edits, local commits, and read-only inspection
  (`status`/`log`/`diff`/`show`) need no special approval.
- **Branch deletion is by explicit name only** — the owner names the branch,
  the agent deletes that branch.
- **Before any push:** list the exact local refs, remote refs, and
  destination remotes, then stop and wait for approval.
- **`--merged`/`--no-merged` are unreliable in this repo.** The `-s ours`
  octopus made two now-abandoned branch tips ancestors of `master`, so
  `git branch --merged master` falsely lists them as merged and a plain
  `git merge` of those branches no-ops without landing any code
  (`docs/DECISIONS.md` D-2026-06-07-2). Verify content actually arrived
  (`git diff <branch> master`) before treating anything as landed or
  deleting it.
- **Checkpoints are owner-only.** Only an explicit owner message satisfies a
  checkpoint or verification step. Agents report observations; the owner
  declares pass/fail. Never self-certify a gate or continue a plan past one
  because the condition appears met. Approvals are single-use, step-specific,
  never carried across sessions. When the owner asks a question or thinks out
  loud, answer in plain English and stop — act only on an explicit decision.

## Style

- Rust edition 2021; format with rustfmt. Modules snake_case, types
  PascalCase, constants SHOUT_CASE; match existing names
  (`transfer_session`, `TransferSession`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, the unified
  `transfer_session` — every transfer, local included, since otp-11);
  most logic and unit tests live here. New modules get re-exported in
  `crates/blit-core/src/lib.rs`.
- `crates/blit-cli/`, `crates/blit-daemon/` — CLI and daemon binaries; admin
  verbs (scan, ls, find, du, df, rm, completions, profile, list-modules) live
  in `blit-cli` alongside transfer commands.
- `crates/blit-app/`, `crates/blit-tui/` — TUI application layers.
- `crates/blit-prometheus-bridge/` — metrics bridge.
- `proto/blit.proto` — gRPC definitions; `blit-core`'s build script vendors
  protoc.
- Integration tests live per-crate (`crates/blit-cli/tests/`,
  `crates/blit-core/tests/`); the root `Cargo.toml` is a virtual workspace,
  so a root-level `tests/` dir would never be compiled. `scripts/` — helper
  tooling.
- `blit-utils` was intentionally removed; its admin verbs now live in
  `blit-cli`. Do not re-add it as a workspace member.

## Commit and Docs Hygiene

- Commit subject: short imperative ("Add streaming planner heartbeat").
  Review-loop commits: `Fix <id>: <one-line summary>` per `.review/README.md`.
- After meaningful work: append a `DEVLOG.md` entry (newest-first, ISO
  timestamp) and update `docs/STATE.md` — the `handoff` procedure does both.
- Every doc in `docs/plan/` carries a `**Status**:` header, one of:
  `Draft | Active | Shipped | Superseded | Historical`. Superseding a doc
  requires a `docs/DECISIONS.md` entry naming winner and loser, and an edit
  to the superseded text.
- `docs/STATE.md` stays ≤ 200 lines with ≤ 3 handoff entries; prune the
  overflow into `DEVLOG.md`.
# Agent State (pointer)

Current state for this repo lives canonically in **`docs/STATE.md`** — read that
file, not this one. This stub exists only so tools expecting the standard
`.agents/` layout find their way.

Do not record state here. The path `docs/STATE.md` is load-bearing: the CI docs
gate (`.github/workflows/docs-gate.yml`), the doc lint
(`scripts/agent/check-docs.sh`), the session hooks (`scripts/agent/context.sh`),
and the `handoff` procedure in `docs/agent/PROTOCOL.md` all read or enforce it.

- Now / Next / Blockers: `docs/STATE.md`
- Verification commands: `.agents/repo-guidance.md` (Verification)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-i5LRDrrg' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-mEBxYXjD' (errno=Operation not permitted)
?? .review/results/otp12-pf1-rigw-harness-r6.codex.md
?? .review/results/otp12-pf1-rigw-harness-r6.grok.json
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-X5NhfLtG' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-AFqbdnge' (errno=Operation not permitted)
4c7c7544db69289cf2e5fc0cf21093b40f00bc0d
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jTk1BVeF' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Lv6EDEeS' (errno=Operation not permitted)
75a9a33ce600e4707438ed885de2ce0cdf27d946
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-svna9OYv' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-E0HcldFR' (errno=Operation not permitted)
 .agents/machines.md                                |     7 +
 .review/findings/otp12-pf1-rigw-harness.md         |   353 +
 .review/results/otp12-pf1-rigw-harness-r2.codex.md | 20683 +++++++++++++++++++
 .../otp12-pf1-rigw-harness-r2.gpt-verdict.md       |    81 +
 .../otp12-pf1-rigw-harness-r2.grok-verdict.md      |    30 +
 .../results/otp12-pf1-rigw-harness-r2.grok.json    |    32 +
 .review/results/otp12-pf1-rigw-harness-r3.codex.md |   125 +
 .../otp12-pf1-rigw-harness-r3.gpt-verdict.md       |    27 +
 .../otp12-pf1-rigw-harness-r3.grok-verdict.md      |    47 +
 .../results/otp12-pf1-rigw-harness-r3.grok.json    |    36 +
 .review/results/otp12-pf1-rigw-harness-r4.codex.md | 15212 ++++++++++++++
 .../otp12-pf1-rigw-harness-r4.gpt-verdict.md       |    16 +
 .../otp12-pf1-rigw-harness-r4.grok-verdict.md      |    32 +
 .../results/otp12-pf1-rigw-harness-r4.grok.json    |    36 +
 .review/results/otp12-pf1-rigw-harness-r5.codex.md |   129 +
 .../otp12-pf1-rigw-harness-r5.gpt-verdict.md       |    36 +
 .../otp12-pf1-rigw-harness-r5.grok-verdict.md      |    22 +
 .../results/otp12-pf1-rigw-harness-r5.grok.json    |    30 +
 .review/results/otp12-pf1-rigw-harness.codex.md    |     7 +
 .../results/otp12-pf1-rigw-harness.gpt-verdict.md  |    72 +
 DEVLOG.md                                          |     4 +
 REVIEW.md                                          |     1 +
 .../blit-core/src/transfer_session/data_plane.rs   |     9 +-
 crates/blit-core/tests/transfer_session_roles.rs   |     3 +
 scripts/bench_otp12pf_rigw.sh                      |  2600 +++
 scripts/otp12pf_rigw_analyze.py                    |  2020 ++
 scripts/otp12pf_rigw_analyze_test.py               |  1069 +
 27 files changed, 42714 insertions(+), 5 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1IvSjZmT' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-5aedSsVt' (errno=Operation not permitted)
M	.agents/machines.md
A	.review/findings/otp12-pf1-rigw-harness.md
A	.review/results/otp12-pf1-rigw-harness-r2.codex.md
A	.review/results/otp12-pf1-rigw-harness-r2.gpt-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r2.grok-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r2.grok.json
A	.review/results/otp12-pf1-rigw-harness-r3.codex.md
A	.review/results/otp12-pf1-rigw-harness-r3.gpt-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r3.grok-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r3.grok.json
A	.review/results/otp12-pf1-rigw-harness-r4.codex.md
A	.review/results/otp12-pf1-rigw-harness-r4.gpt-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r4.grok-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r4.grok.json
A	.review/results/otp12-pf1-rigw-harness-r5.codex.md
A	.review/results/otp12-pf1-rigw-harness-r5.gpt-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r5.grok-verdict.md
A	.review/results/otp12-pf1-rigw-harness-r5.grok.json
A	.review/results/otp12-pf1-rigw-harness.codex.md
A	.review/results/otp12-pf1-rigw-harness.gpt-verdict.md
M	DEVLOG.md
M	REVIEW.md
M	crates/blit-core/src/transfer_session/data_plane.rs
M	crates/blit-core/tests/transfer_session_roles.rs
A	scripts/bench_otp12pf_rigw.sh
A	scripts/otp12pf_rigw_analyze.py
A	scripts/otp12pf_rigw_analyze_test.py

codex
The target range is available locally and touches three substantive areas: a small transfer-role fix, the 2,600-line harness, and the analyzer plus its tests. The worktree has two unrelated untracked round-6 result files; I will not use or modify them, and I’m anchoring source inspection to commit `75a9a33` rather than the worktree.
exec
/bin/zsh -lc "sed -n '1,260p' docs/STATE.md; rg -n '"'^#{1,4} |''^'"\\*\\*Status|F1|G5|G6|standby|purge|registered|ARP|sentinel|manifest|Bash 3\\.2|128|OFF|Transfer RPC|SOURCE|DESTINATION|settle|durability|observe|refuse' docs/plan/OTP12_PERF_FINDINGS.md .review/findings/otp12-pf1-rigw-harness.md; git log --oneline --decorate --reverse 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-15 (pf-1 rig-W instrumentation authorized; no new rig data taken)

- **NEXT ACTION — PF-1 RIG-W INSTRUMENTATION IS ACTIVE:** review the wire-neutral TCP phase trace and its reduced q↔netwatch-01 harness, then run the preregistered paired diagnostic. The owner selected this path on 2026-07-15. The worker-count disparity is closed and is not a blocker; no new hardware data has been taken.
- **ONE TRANSFER PATH IS PROVED.** There is one `Transfer` RPC. When the caller is DESTINATION, it connects to the SOURCE daemon; that daemon sends through the same SOURCE pipeline. Push/pull-facing adapters only select roles. The connection initiator still opens sockets to the responder for NAT/firewall reachability; that topology does not select byte logic or worker policy.
- **WORKER PARITY IS CLOSED.** The identical 10,000-file fixture now reaches exactly 8 workers under both initiator layouts (old guard: 3 vs 2; destination-initiator `max_streams=0`: 1). Payload starts while resize ACKs are pending, refusal is terminal, and resize arbitration is atomic. Final Codex re-review: PASS; workspace gate: 1,490 passed, 2 ignored.
- **WHY NO MAC↔MAC DATA YET:** the current verdict engine can label a 1.092 cell both `PASS` and `REPRODUCES`, and the end-fabric gate can grade after a 10GbE→1GbE renegotiation because it rechecks MSS/IP but not link speed. Those are measurement blockers, not transfer-path blockers. P1 remains real on macOS↔Windows; no Mac↔Mac data exists.

- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent code state: every transfer rides the ONE session. The otp-12 worker-parity repair is reviewed at `42b9b38`; full workspace fmt/clippy/test is green (1,490 passed, 2 ignored, as of that commit).
- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go").** The invariant (plan doc,
  verbatim): ONE block of transfer code; direction/initiator/verb can
  NEVER affect wall time by blit's doing — impossible by construction
  because the per-direction drivers and `Push`/`PullSync` are deleted
  at cutover. Slices otp-1..13; converge-up per cell (±10%);
  symmetric-fs disk-to-disk verdict cells. **D-2026-07-05-2:
  same-build peers only, refusal at session open.**
  - **Slices otp-1 … otp-11 are all `[x]` CLOSED** — the session
    machine, the baselines, the cutover deletion (−13.8k lines) and
    otp-11b's deletion of the old orchestration (−6.2k). The
    deletion-proof acceptance line COMPLETES. The closed-slice record
    was rotated verbatim to `docs/history/state-archive.md`
    (2026-07-14 drift); per-slice detail lives in DEVLOG + `.review/`.
  - **Open: otp-12d and otp-13** — both DEFERRED behind pf-final, see
    Queue 1.
  - **otp-12 worker-parity repair `[x]`** — both initiator layouts reach the same exact target; zero receiver capacity means unknown/default in both; payload proceeds while resize ACKs are pending; resize refusal is terminal. Final Codex re-review PASS. This is code/integration proof, not hardware acceptance.
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
1a. **`docs/plan/OTP12_PERF_FINDINGS.md` (ACTIVE, D-2026-07-13-1).**
    pf-0 is complete: MTU was killed as a dominant cause. **The owner selected
    pf-1 instrumentation on rig W on 2026-07-15.** The TCP phase-trace slice
    and reduced paired q↔netwatch-01 harness must each clear review before rig
    time; the phase report and `0f922de` historical control remain part of the
    pf-1 HARD GATE. No Mac↔Mac data has been taken, and worker parity is no
    longer a blocker. Then: pf-1 → pf-final (all rigs) → otp-12d → otp-13.
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

- **The Mac↔Mac measurement is blocked on the open round-12 instrument-correctness findings and both Macs being bench-quiet.** Round 11 is fixed at `bfae311`; worker parity is fixed and reviewed at `42b9b38`. This is no longer a push/pull-path or worker-count blocker. No Mac↔Mac data exists. Detail: `.review/results/macmac-r12.codex-design.md` and `.review/results/macmac-r12.codex-harness.md`.
- **Rig facts:** `.agents/machines.md` is canonical; do not restate host pairings here.
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
  Linux daemon-spawn flakiness; **windows-latest CI has never been
  observed green — check it live, do not record push state here.**
  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.
- *(Resolved 2026-07-12/13: SizeMtime SKIP, `725aa07`, the `./NAME` foot-gun,
  otp-5b-3 cancel, the change-journal premise — all landed; see DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-15 (52nd)** — **Round 11 FIXED (7 commits, each guard-proved; instrument at `bfae311`,
  prereg rev 11), round 12 REVIEWED, and the reframe changed the plan.** Framed per D-2026-07-14-5
  around the end goal, codex + grok BOTH said **DO NOT RUN** Mac↔Mac; codex raised a would-be
  BLOCKER (P1 measured with an un-settled harness — re-measure first). **I relayed that as a pivot
  WITHOUT checking the data; owner refused and demanded consensus.** Adjudication (both, from the
  CSVs): **P1 IS REAL** — flush symmetric (72/73 ms) vs a ~300 ms effect in *transfer* time, gRPC
  control passes at 1.020, Linux no-P1. The release blocker is genuine. D-2026-07-14-4 (`B≥T/2`
  refuses) and D-2026-07-14-5 (first-review reframe) recorded. Also: rebuilt nagatha's missing
  `f35702a` worktree+binaries. No crates/proto, no rig time, no data. Full: **DEVLOG 02:30Z**.
- **2026-07-14 (51st)** — **BOTH MACS CONFIRMED READY (owner); DEVLOG backfilled for rounds
  7–11. No code, no rig time, no data.** TM autobackup = 0 on both; zero `blit-daemon`. A ready rig
  is not a ready instrument — round 11's BLOCKER stood (now fixed, 52nd). Full: **DEVLOG 22:45Z**.
- **2026-07-14 (50th, `f933097`)** — **`drift`: STATE hygiene.** Handoff log was four rounds stale.
  Created `docs/history/state-archive.md`, anchored `Suite 1488 as of bb28ddd`. Full: **DEVLOG 21:10Z**.
- *(49th and earlier pruned to the cap — full entries in DEVLOG 2026-07-06..15.)*
.review/findings/otp12-pf1-rigw-harness.md:1:# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows
.review/findings/otp12-pf1-rigw-harness.md:4:**Status**: Reopened — G6 fixed and guard-proved; fresh complete review pending.
.review/findings/otp12-pf1-rigw-harness.md:6:## What
.review/findings/otp12-pf1-rigw-harness.md:12:Those properties either destroy the SOURCE/DESTINATION comparison or make the
.review/findings/otp12-pf1-rigw-harness.md:15:## Approach
.review/findings/otp12-pf1-rigw-harness.md:17:- Use semantic `source_init` and `destination_init` arms. SOURCE sends and
.review/findings/otp12-pf1-rigw-harness.md:18:  DESTINATION receives in both arms; the varied property is only which
.review/findings/otp12-pf1-rigw-harness.md:23:  endpoint is reset and reused by all 128 arms; role-bearing run IDs are kept
.review/findings/otp12-pf1-rigw-harness.md:27:  relative-path/size manifests to match, pins the one exact `src_<shape>` root,
.review/findings/otp12-pf1-rigw-harness.md:28:  and retains an identical manifest and digest for every accepted arm.
.review/findings/otp12-pf1-rigw-harness.md:29:- Run a fixed OFF–ON–ON–OFF four-block schedule over
.review/findings/otp12-pf1-rigw-harness.md:33:  four/four role-first balance (128 timed transfers).
.review/findings/otp12-pf1-rigw-harness.md:44:- Use destination-keyed durability: q file fsync for Windows→q and Windows
.review/findings/otp12-pf1-rigw-harness.md:49:  deadline before durability. The measured
.review/findings/otp12-pf1-rigw-harness.md:50:  settle must remain in `[250,1000)` ms and is retained in `runs.csv`.
.review/findings/otp12-pf1-rigw-harness.md:51:  Successful Windows client logs are retrieved only after durability and the
.review/findings/otp12-pf1-rigw-harness.md:52:  current landed count/byte verification. Both caches are purged before every arm and
.review/findings/otp12-pf1-rigw-harness.md:54:  observation remains excluded, but every excess settle millisecond is charged
.review/findings/otp12-pf1-rigw-harness.md:57:  registered split drifts, role-order drift, the full paired range that guards
.review/findings/otp12-pf1-rigw-harness.md:58:  the known bimodal fast arm, trace observer bias, and conservative
.review/findings/otp12-pf1-rigw-harness.md:63:## Files
.review/findings/otp12-pf1-rigw-harness.md:65:- `crates/blit-core/src/transfer_session/data_plane.rs` — SOURCE dial
.review/findings/otp12-pf1-rigw-harness.md:70:- `scripts/bench_otp12pf_rigw.sh` — q-side registered runner and endpoint
.review/findings/otp12-pf1-rigw-harness.md:79:## Tests
.review/findings/otp12-pf1-rigw-harness.md:84:  Bash 3.2 does not reliably apply `set -e` to bare `[[ ... ]]` commands.
.review/findings/otp12-pf1-rigw-harness.md:86:  evidence (128 arms, 768 clock samples, split client/daemon phase logs) and
.review/findings/otp12-pf1-rigw-harness.md:89:  corruption. It pins the split/range/role-order/observer resolution math and
.review/findings/otp12-pf1-rigw-harness.md:91:- The same self-test runs under q's actual macOS Bash and Python so Bash 3.2
.review/findings/otp12-pf1-rigw-harness.md:97:  the synthetic diagnostic fail on a missing SOURCE/DESTINATION endpoint;
.review/findings/otp12-pf1-rigw-harness.md:102:- The analyzer rejects a missing `settled_ms` column, non-integer values, and
.review/findings/otp12-pf1-rigw-harness.md:104:  bound so every accepted arm proves the registered settle window.
.review/findings/otp12-pf1-rigw-harness.md:106:  `total_ms = transfer_ms + (settled_ms - 250) + flush_ms`, and uses that
.review/findings/otp12-pf1-rigw-harness.md:107:  durable total for every paired median, delta, distribution, observer-bias,
.review/findings/otp12-pf1-rigw-harness.md:111:  pre-durability transfer time, and an equal client-to-durability regression
.review/findings/otp12-pf1-rigw-harness.md:112:  proves asymmetric settle/flush partitioning cannot manufacture a role delta.
.review/findings/otp12-pf1-rigw-harness.md:118:  requires sent proposal before SOURCE socket acquisition, attachment before
.review/findings/otp12-pf1-rigw-harness.md:119:  SOURCE settlement, final settlement/ACK before role-local completion, and
.review/findings/otp12-pf1-rigw-harness.md:121:  the DESTINATION. Mutations reverse every one of those edges while preserving
.review/findings/otp12-pf1-rigw-harness.md:125:- Mutation proof: restoring SOURCE dial attachment ahead of `socket_dial_end`
.review/findings/otp12-pf1-rigw-harness.md:129:- Fixture and landed manifests encode each UTF-8 POSIX relative path in base64
.review/findings/otp12-pf1-rigw-harness.md:132:  exact q/Windows canonical equality and exactly 128 landed manifest files,
.review/findings/otp12-pf1-rigw-harness.md:145:- SOURCE- and DESTINATION-initiated arms resolve to the same canonical
.review/findings/otp12-pf1-rigw-harness.md:150:  `destination_relative_path` now turns the Bash 3.2 self-test red at the first
.review/findings/otp12-pf1-rigw-harness.md:165:  rejected unless the registered marker is a regular one-line file containing
.review/findings/otp12-pf1-rigw-harness.md:188:  launcher and daemon, proves q can reach the registered port, identity-stops
.review/findings/otp12-pf1-rigw-harness.md:193:  sequence and keep the smoke branch ahead of registered-run state. Mutations
.review/findings/otp12-pf1-rigw-harness.md:196:  and a mutation setting registered state, each turn the self-test red.
.review/findings/otp12-pf1-rigw-harness.md:199:  successful Windows client-log fetch ahead of the durability marker makes
.review/findings/otp12-pf1-rigw-harness.md:202:- A delayed fake Windows-result producer emits its exact sentinel and then
.review/findings/otp12-pf1-rigw-harness.md:209:  `run_arm`, and live preflight proves the flushed Windows sentinel reaches q
.review/findings/otp12-pf1-rigw-harness.md:213:  preparation, ACK, settlement, attachment, and role-local ordering evidence.
.review/findings/otp12-pf1-rigw-harness.md:215:  target/live validation makes all four final-epoch SOURCE and DESTINATION
.review/findings/otp12-pf1-rigw-harness.md:225:## Known gaps
.review/findings/otp12-pf1-rigw-harness.md:236:  The harness reports and refuses those conditions; it does not mutate them.
.review/findings/otp12-pf1-rigw-harness.md:238:## Reviewer comments
.review/findings/otp12-pf1-rigw-harness.md:244:settle accounting at `1617546`, and the complete resize causal-edge audit plus
.review/findings/otp12-pf1-rigw-harness.md:249:returned `NEEDS FIXES`: it independently confirmed F1–F3 closed, then found two
.review/findings/otp12-pf1-rigw-harness.md:251:the settle anchor. F5 is the role-bearing `rid` selecting different physical
.review/findings/otp12-pf1-rigw-harness.md:268:survive failure under macOS Bash 3.2. Grok's role-in-path mutation produced
.review/findings/otp12-pf1-rigw-harness.md:277:`[[ ... ]]` assertions that macOS Bash 3.2 can let fall through to a later
.review/findings/otp12-pf1-rigw-harness.md:286:signal each turns the Bash 3.2 self-test red at the intended assertion;
.review/findings/otp12-pf1-rigw-harness.md:300:required before the registered run.
.review/findings/otp12-pf1-rigw-harness.md:302:The first live launcher-smoke attempt on q refused before launching a daemon
.review/findings/otp12-pf1-rigw-harness.md:303:or timing a transfer. G5 is accepted as a High instrument-correctness finding:
.review/findings/otp12-pf1-rigw-harness.md:304:q legitimately has the Windows peer cached on `en0`, `en1`, and registered
.review/findings/otp12-pf1-rigw-harness.md:305:`en8`, but the ARP gate concatenated all three MAC rows. It therefore rejected
.review/findings/otp12-pf1-rigw-harness.md:309:parses exactly the registered interface, requires one result, and pins the
.review/findings/otp12-pf1-rigw-harness.md:310:real three-interface shape in the Bash 3.2 self-test. No daemon started and no
.review/findings/otp12-pf1-rigw-harness.md:317:confirmed G5, the exact 128-arm schedule, and role-invariant endpoint-local
.review/findings/otp12-pf1-rigw-harness.md:318:paths, then returned `NEEDS FIXES` with one separate High finding. G6 is
.review/findings/otp12-pf1-rigw-harness.md:320:`D:/blit-test/purge-standby.ps1` by existence and exit status only, rather
.review/findings/otp12-pf1-rigw-harness.md:324:G5 after independently driving the ARP interface mutation red and restoring
.review/findings/otp12-pf1-rigw-harness.md:325:the Bash 3.2 self-test green. Its detached worktree ended clean and was
.review/findings/otp12-pf1-rigw-harness.md:329:G6 now takes the purge helper only from the exact clean q checkout. After all
.review/findings/otp12-pf1-rigw-harness.md:335:`standby-purged` success line in addition to exit zero. The helper is therefore
.review/findings/otp12-pf1-rigw-harness.md:339:The Bash 3.2 self-test functionally mocks both stage and per-arm commands.
.review/findings/otp12-pf1-rigw-harness.md:342:turns it red before the mocked purge can pass; restoring it returns green. A
.review/findings/otp12-pf1-rigw-harness.md:347:G6 was fixed at `888be4754387311e28e14d687721fd3d1315f82c`.
docs/plan/OTP12_PERF_FINDINGS.md:1:# otp-12 perf findings — investigate + fix before acceptance (design)
docs/plan/OTP12_PERF_FINDINGS.md:3:**Status**: Active
docs/plan/OTP12_PERF_FINDINGS.md:8:3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
docs/plan/OTP12_PERF_FINDINGS.md:15:factual claim was settled by *measurement* (the same-OS rig refuted a
docs/plan/OTP12_PERF_FINDINGS.md:50:## The two findings (evidence, both committed)
docs/plan/OTP12_PERF_FINDINGS.md:105:### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)
docs/plan/OTP12_PERF_FINDINGS.md:119:**8/8 invariance cells PASS** (`ms_grpc_mixed` via its pre-registered
docs/plan/OTP12_PERF_FINDINGS.md:163:> on a scratch probe (and a first harness revision) that ran the durability
docs/plan/OTP12_PERF_FINDINGS.md:165:> initiator is the SOURCE, which only read, so its sync was a no-op and the
docs/plan/OTP12_PERF_FINDINGS.md:168:> durability the other got free — multi-second on skippy's ZFS — which
docs/plan/OTP12_PERF_FINDINGS.md:172:> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
docs/plan/OTP12_PERF_FINDINGS.md:177:### The residual confound (WHICH code) still needs a counterfactual
docs/plan/OTP12_PERF_FINDINGS.md:282:## pf-0 — the environmental control (MTU): **KILLED as a material cause of P1** (recorded 2026-07-14)
docs/plan/OTP12_PERF_FINDINGS.md:284:Executed as pre-registered
docs/plan/OTP12_PERF_FINDINGS.md:287:and guards were registered in rev 3, before any of the S1–S4 data existed, and
docs/plan/OTP12_PERF_FINDINGS.md:298:**What this licenses — exactly the registered outcome, and no more.** Raising
docs/plan/OTP12_PERF_FINDINGS.md:299:the MTU **did not improve these cells under the observed packetization**: the
docs/plan/OTP12_PERF_FINDINGS.md:318:  could be swamped. The registered rule returns KILLED on the point estimate,
docs/plan/OTP12_PERF_FINDINGS.md:319:  and that grade stands as registered; the *resolution limit* is stated here so
docs/plan/OTP12_PERF_FINDINGS.md:379:## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
docs/plan/OTP12_PERF_FINDINGS.md:384:  session the SOURCE is the responder: each sf-2 resize epoch is
docs/plan/OTP12_PERF_FINDINGS.md:385:  ACCEPTED off the source's listener while the DESTINATION dials
docs/plan/OTP12_PERF_FINDINGS.md:400:  claim. Only the dial/accept inversion counterfactual in pf-1 can settle H1.
docs/plan/OTP12_PERF_FINDINGS.md:416:  layouts drain the same fixed 128-entry destination need loop, so
docs/plan/OTP12_PERF_FINDINGS.md:418:  manifest/need emission in either layout. Kept only as a residual: if
docs/plan/OTP12_PERF_FINDINGS.md:442:  opened at one stream (after its 128-file early flush) then resized
docs/plan/OTP12_PERF_FINDINGS.md:450:  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
docs/plan/OTP12_PERF_FINDINGS.md:497:  the gRPC caveat predicted)**: HEAD's need/manifest bookkeeping is
docs/plan/OTP12_PERF_FINDINGS.md:499:  **mutex-protected sent-manifest map** with per-entry insertion and
docs/plan/OTP12_PERF_FINDINGS.md:513:## Method (the investigation slice — no behavior changes)
docs/plan/OTP12_PERF_FINDINGS.md:524:   high-volume probe slice so they cannot perturb the focused P1 observer.
docs/plan/OTP12_PERF_FINDINGS.md:541:   round 2), so the control is observed externally — phase boundaries
docs/plan/OTP12_PERF_FINDINGS.md:546:   configuration runs an instrumentation-on/off pair to bound observer
docs/plan/OTP12_PERF_FINDINGS.md:557:   metric DEFINED (review round 2: "manifest-complete→first-payload
docs/plan/OTP12_PERF_FINDINGS.md:563:   `t_manifest_complete`; `t_first_payload_queued` (the payload enters
docs/plan/OTP12_PERF_FINDINGS.md:569:   established only if `t_first_socket_write − t_manifest_complete` is
docs/plan/OTP12_PERF_FINDINGS.md:572:   TCP bytes on the wire before its manifest completed, and the new
docs/plan/OTP12_PERF_FINDINGS.md:576:   counterfactual: a debug-flag toggle that restores mid-manifest TCP
docs/plan/OTP12_PERF_FINDINGS.md:580:   interleaved old-vs-new. Pre-registered: H5 is CONFIRMED iff the
docs/plan/OTP12_PERF_FINDINGS.md:601:## pf-1 decision rule — UNIFORM, pre-registered (added round 5)
docs/plan/OTP12_PERF_FINDINGS.md:643:  `Δ`) is graded on a **pre-registered scale**, no post-hoc bands:
docs/plan/OTP12_PERF_FINDINGS.md:655:## Fix criteria (pre-registered; the owner walks the final numbers)
docs/plan/OTP12_PERF_FINDINGS.md:686:  the registered platform-residue discriminator — invariance plus the
docs/plan/OTP12_PERF_FINDINGS.md:697:## Staging (each through the codex loop)
docs/plan/OTP12_PERF_FINDINGS.md:737:## Known gaps
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZbE4oeca' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-wrxgueZ9' (errno=Operation not permitted)
bd7a4c2 Add draft pf-1 rig-W phase harness
ae494d3 Reject dirty rig-W build identities
7f81b2e Pin rig-W TCP sessions to eight streams
3208870 Record complete rig-W harness audit
333c21a Equalize rig-W post-client settling
48bcaea Grade rig-W evidence on durable total time
a71c4ff Reject causally impossible rig-W traces
4eaf181 Verify exact rig-W landed tree identity
a63ebfc Require fresh rig-W evidence directories
d105a48 Verify rig-W Windows launcher ownership
0fb8237 Fail closed rig-W session finalization
d267335 Record rig-W Codex review findings
661cf75 Reject incomplete rig-W destination resets
1617546 Charge rig-W excess settle latency
2dd977e Complete rig-W resize causal guards
8fbd486 Record rig-W audit fixes
6b95f03 Record rig-W round-two review
1231e42 Make rig-W destination paths role-invariant
6ba5408 Anchor rig-W settle at client result arrival
18d3cde Add standalone rig-W launcher smoke
454ebce Journal Windows launcher before daemon start
53bb5e5 Record rig-W round-two closures
5e90c76 Record rig-W round-three review
27c94b0 Fix rig-W path parity guard
7e9d2d5 Fix rig-W lifecycle selftest guards
6f517ea Record rig-W Bash guard fixes
4ebbed4 Record rig-W round-four acceptance
06b3322 Fix otp12-pf1-rigw-harness: scope ARP peer to interface
3ed9a4f Record rig-W round-five review
888be47 Fix otp12-pf1-rigw-harness: pin purge helper provenance
75a9a33 (HEAD -> master) Record rig-W purge provenance fix

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:.review/findings/otp12-pf1-rigw-harness.md | nl -ba; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:docs/plan/OTP12_PERF_FINDINGS.md | sed -n '379,760p' | nl -ba -v379" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-dg0rtllp' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-g3bwSvYk' (errno=Operation not permitted)
     1	# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows
     2
     3	**Slice**: OTP12 performance-finding pf-1, P1 rig harness only.
     4	**Status**: Reopened — G6 fixed and guard-proved; fresh complete review pending.
     5
     6	## What
     7
     8	The acceptance harness cannot be reused unchanged for the phase diagnostic.
     9	It retains old/new and push/pull-shaped orchestration, drains Windows even
    10	when q is the destination, keeps one daemon alive across instrumentation-state
    11	changes, discards successful client stderr, and can create a firewall rule.
    12	Those properties either destroy the SOURCE/DESTINATION comparison or make the
    13	new two-endpoint trace uncorrelatable.
    14
    15	## Approach
    16
    17	- Use semantic `source_init` and `destination_init` arms. SOURCE sends and
    18	  DESTINATION receives in both arms; the varied property is only which
    19	  endpoint initiates the one `Transfer` session.
    20	- Pin one canonical source tree per direction and fixture. Both roles read the
    21	  same q or Windows physical path and land into a precreated container of the
    22	  same depth and shape. One session-scoped canonical destination path per
    23	  endpoint is reset and reused by all 128 arms; role-bearing run IDs are kept
    24	  only in evidence names and never enter a measured path. Session scoping
    25	  preserves failed-run endpoint evidence without reintroducing a within-run
    26	  path axis. The harness requires the q and Windows canonical
    27	  relative-path/size manifests to match, pins the one exact `src_<shape>` root,
    28	  and retains an identical manifest and digest for every accepted arm.
    29	- Run a fixed OFF–ON–ON–OFF four-block schedule over
    30	  `wm_tcp_mixed`, `mw_tcp_mixed`, `wm_grpc_mixed`, and `wm_tcp_large`.
    31	  Pair rounds traverse cells forward/reverse/reverse/forward and run the two
    32	  roles adjacently, producing eight pairs per trace state and cell with a
    33	  four/four role-first balance (128 timed transfers).
    34	- Stop and restart both exact daemons for every block, including ON→ON. Each
    35	  block has a common run ID; every TCP client log supplies the 16-hex session
    36	  fingerprint that correlates its peer daemon records. Windows logs are
    37	  retrieved through base64 with SHA-256 verification.
    38	- Fail closed on the exact build, route/interface/IP/MAC/MTU/link speeds,
    39	  direction-specific negotiated MSS, firewall-rule identity, timer
    40	  calibration, load, Time Machine, Spotlight, Windows CPU/disk drain, stale
    41	  processes, PID ownership, port teardown, trace leakage, incomplete trace
    42	  inventory, or landed-tree mismatch. The harness never changes firewall,
    43	  MTU, routing, Time Machine, Spotlight, or unrelated processes.
    44	- Use destination-keyed durability: q file fsync for Windows→q and Windows
    45	  volume flush for q→Windows. Both client locations capture the same q
    46	  monotonic completion anchor: immediate subprocess return on q, or the
    47	  streamed Windows result line as q receives it before SSH teardown. They take
    48	  the same three after-clock samples and wait only to the absolute +250 ms
    49	  deadline before durability. The measured
    50	  settle must remain in `[250,1000)` ms and is retained in `runs.csv`.
    51	  Successful Windows client logs are retrieved only after durability and the
    52	  current landed count/byte verification. Both caches are purged before every arm and
    53	  Windows disk writes must drain. The common first 250 ms of post-client
    54	  observation remains excluded, but every excess settle millisecond is charged
    55	  to the arm's durable total before comparison.
    56	- Compute paired differences `d_i = destination_init_i − source_init_i`, the
    57	  registered split drifts, role-order drift, the full paired range that guards
    58	  the known bimodal fast arm, trace observer bias, and conservative
    59	  `N_resolution`. Reports retain every sorted arm/difference distribution and
    60	  use only per-endpoint monotonic clocks for phase intervals. Cross-host clock
    61	  samples quantify uncertainty and are never silently subtracted.
    62
    63	## Files
    64
    65	- `crates/blit-core/src/transfer_session/data_plane.rs` — SOURCE dial
    66	  trace attachment now follows the matching dial-end marker at epoch zero
    67	  and every resize epoch.
    68	- `crates/blit-core/tests/transfer_session_roles.rs` — both initiator layouts
    69	  pin action-end before attachment on both endpoint roles.
    70	- `scripts/bench_otp12pf_rigw.sh` — q-side registered runner and endpoint
    71	  gates.
    72	- `scripts/otp12pf_rigw_analyze.py` — exact schedule, trace, clock, phase, and
    73	  resolution validator/reporter.
    74	- `scripts/otp12pf_rigw_analyze_test.py` — complete synthetic session and
    75	  fail-closed mutations.
    76	- `.agents/machines.md` — current direction-specific MSS and q SSH endpoint
    77	  fact.
    78
    79	## Tests
    80
    81	- `SELFTEST=1 bash scripts/bench_otp12pf_rigw.sh` proves the exact block/arm
    82	  inventory and canonical path construction without contacting either rig
    83	  endpoint. Every path assertion has an explicit failure path because macOS
    84	  Bash 3.2 does not reliably apply `set -e` to bare `[[ ... ]]` commands.
    85	- `python3 scripts/otp12pf_rigw_analyze_test.py` builds complete synthetic
    86	  evidence (128 arms, 768 clock samples, split client/daemon phase logs) and
    87	  rejects missing clock rows, missing endpoint trace, trace-off leakage,
    88	  gRPC trace leakage, schedule drift, sequence gaps, and terminal/inventory
    89	  corruption. It pins the split/range/role-order/observer resolution math and
    90	  all exported reports.
    91	- The same self-test runs under q's actual macOS Bash and Python so Bash 3.2
    92	  and platform behavior are exercised, not inferred from nagatha.
    93	- Mutation proof: removing role-order drift and the full paired-range term from
    94	  `N_pair` makes the synthetic diagnostic fail (`N_resolution` falls from 70
    95	  ms to 40 ms); restoring them returns the analyzer suite to green.
    96	- Mutation proof: excluding successful client logs from trace discovery makes
    97	  the synthetic diagnostic fail on a missing SOURCE/DESTINATION endpoint;
    98	  restoring both client and daemon evidence roots returns all tests to green.
    99	- Mutation proof: reducing the clock-row formatter from 12 fields to 11 makes
   100	  the harness self-test fail before analysis; restoring the exact 12-column
   101	  schema returns the local and q/macOS self-tests to green.
   102	- The analyzer rejects a missing `settled_ms` column, non-integer values, and
   103	  values outside `[250,1000)`. Synthetic evidence supplies the lower valid
   104	  bound so every accepted arm proves the registered settle window.
   105	- The analyzer parses each timing component once, requires exact Decimal
   106	  `total_ms = transfer_ms + (settled_ms - 250) + flush_ms`, and uses that
   107	  durable total for every paired median, delta, distribution, observer-bias,
   108	  and resolution-floor value. Only the common first 250 ms remains excluded;
   109	  excess observation latency is charged. Corrupt totals are rejected;
   110	  role-specific flush mutations prove the summaries cannot fall back to the
   111	  pre-durability transfer time, and an equal client-to-durability regression
   112	  proves asymmetric settle/flush partitioning cannot manufacture a role delta.
   113	- All asserted causal phase pairs are endpoint-local and require both producer
   114	  order and nondecreasing monotonic elapsed time. Socket action completion must
   115	  precede trace attachment; attached payload sockets must progress through
   116	  first write/receive before their role's data-plane completion; resize and
   117	  planner prerequisite chains are also pinned. The resize DAG additionally
   118	  requires sent proposal before SOURCE socket acquisition, attachment before
   119	  SOURCE settlement, final settlement/ACK before role-local completion, and
   120	  the exact receive→arm→ready→accept or receive→dial→attach→prepared chain on
   121	  the DESTINATION. Mutations reverse every one of those edges while preserving
   122	  exact contiguous producer sequences and must fail. Swapping completion ahead
   123	  of a first write, swapping attachment ahead of action completion, or
   124	  reversing a causal elapsed interval also makes the analyzer suite fail.
   125	- Mutation proof: restoring SOURCE dial attachment ahead of `socket_dial_end`
   126	  makes the two-initiator Rust phase test fail at epoch zero and resize epoch
   127	  one; restoring end-before-attachment returns it to green. No cross-endpoint
   128	  or concurrent send/ACK ordering is asserted.
   129	- Fixture and landed manifests encode each UTF-8 POSIX relative path in base64
   130	  beside its decimal file size, sort under ordinal/C locale rules, and reject
   131	  nonregular or reparse entries. The analyzer recomputes all digests, requires
   132	  exact q/Windows canonical equality and exactly 128 landed manifest files,
   133	  and rejects swapped per-file sizes, renamed paths, wrong root layout, or a
   134	  forged recorded digest even when file count and total bytes are unchanged.
   135	- The harness atomically claims a never-existing evidence directory before it
   136	  installs the EXIT trap or writes a byte. Existing paths are rejected
   137	  unchanged, with explicit stale `SESSION-COMPLETE`/`SESSION-VOID` diagnostics;
   138	  offline guards also pin rejection of unrelated retained content.
   139	- Every arm resets its exact destination with explicit error propagation,
   140	  verifies deletion landed, and proves the replacement is an empty plain
   141	  directory before draining caches or starting the timer. The q self-test
   142	  mutation makes removal fail under the production `||` call shape and must
   143	  remain rejected; a Windows source-contract guard forbids suppressed removal
   144	  errors and requires absence, directory, reparse, and emptiness checks.
   145	- SOURCE- and DESTINATION-initiated arms resolve to the same canonical
   146	  endpoint-local destination path and remote module-relative path. The
   147	  self-test pins both direction/role pairs with explicit `|| die` guards and
   148	  rejects any `run_arm` source that lets the role-bearing evidence ID reach a
   149	  measured destination. Adding the initiator role to
   150	  `destination_relative_path` now turns the Bash 3.2 self-test red at the first
   151	  q destination-path assertion; restoring the role-invariant path returns it
   152	  to green.
   153	- The failure handler removes any completion marker, stops only remembered
   154	  identity-checked daemons, appends teardown errors without replacing the
   155	  primary void reason, and never initiates session-tree deletion. HUP, INT,
   156	  and TERM enter that same bounded failure path. Offline process tests exercise
   157	  all three signals and prove both owned teardown paths run while remaining
   158	  evidence paths are reported for inspection.
   159	- Successful finalization first proves no remembered daemon or open port,
   160	  requires analyzer-accepted local evidence, removes and verifies both exact
   161	  Windows trees and the exact q tree, rechecks the port, and only then atomically
   162	  renames `SESSION-COMPLETE.tmp`. Cross-host deletion is not transactional: a
   163	  partial finalization failure keeps the complete local evidence and reports
   164	  remote paths as “may remain,” never as certainly preserved. A zero exit is
   165	  rejected unless the registered marker is a regular one-line file containing
   166	  the exact build SHA with no VOID or temporary marker; preflight-only runs
   167	  cannot create it. Mutations for failed Windows removal, a surviving q tree,
   168	  a pre/post-cleanup open port, missing/wrong completion markers, stale
   169	  preflight markers, and cleanup before completion all fail the self-test.
   170	- Windows launcher and daemon PIDs are numeric and identity-checked before any
   171	  termination: exact executable/name, one anchored block-specific `cmd.exe`
   172	  command line, and daemon parent PID when both processes exist. Startup also
   173	  verifies the same CIM identities immediately. Offline source-contract
   174	  mutations fail if command-line, parent, or validate-before-stop guards move
   175	  or disappear. If startup fails after CIM creation but before either PID file
   176	  is readable, the generated launcher waits on a bounded block-local gate and
   177	  cannot execute the daemon until its PID is atomically placed and read back;
   178	  without that gate it exits on its own. Teardown recovers only the unique
   179	  exact block-specific launcher command and its parented daemon; after stopping
   180	  the launcher it also finds, validates, and stops a child that raced the first
   181	  query. The live daemon smoke remains required to prove CIM quoting.
   182	  Mutations accepting any `cmd.exe`, accepting an unparented daemon, skipping
   183	  the bounded gate wait, opening the gate before PID placement/readback, or
   184	  skipping the late child's exact executable validation each turn the
   185	  self-test red.
   186	- `LAUNCHER_SMOKE=1` is a mutually exclusive standalone live mode. After the
   187	  full provenance and endpoint preflight, it starts only the exact Windows CIM
   188	  launcher and daemon, proves q can reach the registered port, identity-stops
   189	  both processes, proves both endpoint ports closed, and completes strict
   190	  session-tree cleanup. It never registers a run, starts q's daemon, times a
   191	  transfer, invokes the analyzer, or writes `SESSION-COMPLETE`. An offline
   192	  call-order test and source guard pin the start/reach/stop/closed/cleanup
   193	  sequence and keep the smoke branch ahead of registered-run state. Mutations
   194	  removing its pre-start port gate, start, reachability probe, exact stop/log
   195	  collection, block clear, strict cleanup/failure path, or main-branch return,
   196	  and a mutation setting registered state, each turn the self-test red.
   197	- Mutation proof: replacing the absolute-deadline wait with a no-op makes the
   198	  harness self-test fail because it returns before +250 ms. Moving the
   199	  successful Windows client-log fetch ahead of the durability marker makes
   200	  the production-order self-test fail. Restoring both returns the harness and
   201	  analyzer self-tests to green.
   202	- A delayed fake Windows-result producer emits its exact sentinel and then
   203	  holds the pipe open; the q arrival stamp must predate producer teardown by a
   204	  broad bound. Moving the stamp to EOF or restoring a fresh post-return q
   205	  anchor makes the self-test fail. Reverting q to Python's process-relative
   206	  macOS `time.monotonic_ns()` also fails an explicit cross-process clock guard;
   207	  every carried q timestamp uses `clock_gettime(CLOCK_MONOTONIC)`. Both client
   208	  wrappers carry the q completion stamp as the fourth result field consumed by
   209	  `run_arm`, and live preflight proves the flushed Windows sentinel reaches q
   210	  before the remote producer exits.
   211	- Every trace-on TCP session must prove the complete seven-epoch one-stream
   212	  ramp from one to eight live sockets on both roles, including exact proposal,
   213	  preparation, ACK, settlement, attachment, and role-local ordering evidence.
   214	  Removing epoch 7 makes the targeted analyzer guard fail; disabling exact
   215	  target/live validation makes all four final-epoch SOURCE and DESTINATION
   216	  mutations fail. Restoring both guards returns the analyzer suite to green.
   217	- The build-identity self-test accepts the exact 12-character clean marker and
   218	  mutation-proves that the same marker with `.dirty` is rejected. Live q and
   219	  Windows gates apply that positive-and-negative check to both binaries.
   220	- The repository gate is green: `cargo fmt --all -- --check`,
   221	  `cargo clippy --workspace --all-targets -- -D warnings`,
   222	  `cargo test --workspace`, the documentation gate, analyzer tests, and shell
   223	  syntax checks all passed.
   224
   225	## Known gaps
   226
   227	- No rig datum is produced by this slice. The full live run waits for fresh
   228	  mandatory Codex adjudication, exact isolated builds, a successful live
   229	  launcher smoke, and a green endpoint preflight.
   230	- This four-cell run is the reduced P1 phase diagnostic, not the entire pf-1
   231	  hard gate. The active plan still requires the separately reviewed
   232	  small-fixture/P2 work, phase report, and `0f922de` historical control before
   233	  pf-1 closes.
   234	- q was not quiet during the first read-only readiness sample on 2026-07-15:
   235	  Time Machine AutoBackup was enabled and Spotlight was using substantial CPU.
   236	  The harness reports and refuses those conditions; it does not mutate them.
   237
   238	## Reviewer comments
   239
   240	Initial Codex review (`gpt-5.6-sol`, `xhigh`, codex-cli 0.144.4) reviewed
   241	`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..0fb8237c2e6f63feb9cfc613d8af1602730061b0`
   242	and returned `NEEDS FIXES` with three High findings. All three were accepted
   243	and fixed independently: destination reset fail-closed at `661cf75`, excess
   244	settle accounting at `1617546`, and the complete resize causal-edge audit plus
   245	emitter alignment at `2dd977e`. See the raw review and adjudication under
   246	`.review/results/otp12-pf1-rigw-harness.*`.
   247
   248	Round-2 Codex reviewed the complete immutable range through `8fbd486` and
   249	returned `NEEDS FIXES`: it independently confirmed F1–F3 closed, then found two
   250	new High defects. F4 is an uncharged Windows-client interval before q captures
   251	the settle anchor. F5 is the role-bearing `rid` selecting different physical
   252	destination paths for paired arms, contrary to the only-initiator-varies
   253	contract. Both were accepted and fixed in order: F5 at `1231e42`, then F4 at
   254	`6ba5408`. A separate runbook audit found the missing standalone launcher mode,
   255	fixed at `18d3cde`; follow-up safety audit found the pre-PID-journal CIM race,
   256	fixed at `454ebce`. The additive Grok second eye returned a schema-valid
   257	`ACCEPTED` verdict with three independent red-to-green guards, but it does not
   258	override the mandatory Codex findings. See the round-2 raw and adjudication
   259	records under `.review/results/otp12-pf1-rigw-harness-r2.*`. Fresh review of
   260	the complete fixed range is pending; no rig run is authorized yet.
   261
   262	Round-3 Codex reviewed
   263	`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..53bb5e56a864abe0ee2d2b00c411846a1e7d24d5`
   264	and returned `PASS` with no findings. The additive Grok review of the same
   265	immutable range returned schema-valid `REOPENED`, `guard_confirmed=false`.
   266	G3 is accepted: production role-invariant path construction is correct, but
   267	the path-construction/parity assertions are bare `[[ ... ]]` commands that can
   268	survive failure under macOS Bash 3.2. Grok's role-in-path mutation produced
   269	different physical destinations while `SELFTEST=1` still exited zero. The
   270	timing-anchor and launcher-journal mutations independently went red-to-green.
   271	See `.review/results/otp12-pf1-rigw-harness-r3.*`. G3 was fixed at `27c94b0`;
   272	the complete range still requires fresh review before any rig activity.
   273
   274	Coder follow-up audit admitted G4 as a separate High instrument-correctness
   275	finding. Destination-type, finalization-state, strict-cleanup-state,
   276	completion-marker-removal, and signal-cleanup checks still used bare
   277	`[[ ... ]]` assertions that macOS Bash 3.2 can let fall through to a later
   278	successful command. A regression could therefore leave an unsafe destination
   279	type, false cleanup state, or stale completion marker while the offline
   280	self-test still exited zero. G4 gives each material lifecycle assertion an
   281	explicit failure path and seeds the signal test with a completion marker, so
   282	that its absence check is not vacuous. Final-command subshell predicates and
   283	intentional predicate returns are unchanged. Removing the production
   284	`SESSION_FINALIZED=1`, retaining `Q_SESSION_MAY_EXIST=1` after successful
   285	cleanup, or conditionally skipping completion-marker removal for a received
   286	signal each turns the Bash 3.2 self-test red at the intended assertion;
   287	restoring all three returns it to green.
   288
   289	G4 was fixed at `7e9d2d5`. The full workspace format, strict-clippy, and test
   290	gate; 23 analyzer tests; Bash syntax and self-test; documentation gate; and
   291	diff check are green for both G3 and G4. No endpoint was contacted.
   292
   293	Round-4 mandatory Codex and additive Grok reviewed the complete immutable
   294	range through `6f517ea1bdbea2f7d83f15c086d2bf5f764cf524`. Codex returned
   295	`PASS` with no material finding. Grok returned schema-valid `ACCEPTED`,
   296	`guard_confirmed=true`, exact SHAs, and independently drove the G3 role-path
   297	mutation plus G4 finalization, may-exist, and marker-removal mutations red
   298	before restoring every offline suite green. Its detached worktree ended clean
   299	and was removed. Review is closed; launcher smoke and endpoint preflight remain
   300	required before the registered run.
   301
   302	The first live launcher-smoke attempt on q refused before launching a daemon
   303	or timing a transfer. G5 is accepted as a High instrument-correctness finding:
   304	q legitimately has the Windows peer cached on `en0`, `en1`, and registered
   305	`en8`, but the ARP gate concatenated all three MAC rows. It therefore rejected
   306	the correct peer even though `route -n get` selected `en8`. The failed attempt
   307	is retained as `SESSION-VOID` under
   308	`logs/otp12pf-rigw-20260715T113500Z-launcher` in the isolated q clone. The fix
   309	parses exactly the registered interface, requires one result, and pins the
   310	real three-interface shape in the Bash 3.2 self-test. No daemon started and no
   311	endpoint policy changed. Removing the interface predicate makes the self-test
   312	red on the three-row fixture; restoring it returns the complete self-test to
   313	green.
   314
   315	Round-5 reviewed the complete immutable range through
   316	`06b33228d502c51da24bc2a78fba7eddcf6c0723`. Mandatory Codex independently
   317	confirmed G5, the exact 128-arm schedule, and role-invariant endpoint-local
   318	paths, then returned `NEEDS FIXES` with one separate High finding. G6 is
   319	accepted: the harness runs the endpoint's pre-existing
   320	`D:/blit-test/purge-standby.ps1` by existence and exit status only, rather
   321	than staging and hashing the reviewed repository helper. A stale or no-op
   322	helper could therefore make a warm-cache run look valid. Additive Grok
   323	returned schema-valid `ACCEPTED`, exact SHAs, and `guard_confirmed=true` for
   324	G5 after independently driving the ARP interface mutation red and restoring
   325	the Bash 3.2 self-test green. Its detached worktree ended clean and was
   326	removed. No endpoint was contacted. See the round-5 raw reviews and
   327	adjudications under `.review/results/otp12-pf1-rigw-harness-r5.*`.
   328
   329	G6 now takes the purge helper only from the exact clean q checkout. After all
   330	read-only endpoint/fabric/quiet gates pass, the harness reserves a fresh
   331	per-session Windows tree, copies the reviewed helper to a temporary path,
   332	rejects reparse points, verifies SHA-256 before and after the atomic move, and
   333	records the helper hash/path alongside the four executable hashes. Every arm
   334	rechecks that same hash immediately before invocation and requires exactly one
   335	`standby-purged` success line in addition to exit zero. The helper is therefore
   336	covered by the executable snapshot and strict session-tree cleanup rather than
   337	trusted as endpoint state.
   338
   339	The Bash 3.2 self-test functionally mocks both stage and per-arm commands.
   340	Removing the final post-move hash comparison turns it red at the staging
   341	contract; restoring it returns green. Removing the per-arm hash comparison
   342	turns it red before the mocked purge can pass; restoring it returns green. A
   343	separate order guard pins the first remote write after provenance, port,
   344	topology, MSS, firewall, quietness, timer, and result-stream checks. No endpoint
   345	was contacted by the fix or its mutation proofs.
   346
   347	G6 was fixed at `888be4754387311e28e14d687721fd3d1315f82c`.
   348	Format, strict clippy, Bash syntax/self-test, all 23 analyzer tests, the docs
   349	gate, and diff checks passed. The first full workspace test attempt hit the
   350	recorded macOS `blit_utils::test_utils_list_modules` daemon-start race once;
   351	the isolated test then passed, and a complete quiet rerun passed with two
   352	expected ignores. Fresh complete Codex plus additive Grok review is still
   353	required before any build or endpoint contact.
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-eQfchzFF' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-j5dbh4f8' (errno=Operation not permitted)
   379	## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
   380
   381	- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
   382	  connection-initiating end DIALS; byte direction is role-set
   383	  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
   384	  session the SOURCE is the responder: each sf-2 resize epoch is
   385	  ACCEPTED off the source's listener while the DESTINATION dials
   386	  (otp-5b-2: `SourceSockets` Dial/Accept branches;
   387	  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
   388	  accept/dial round-trips or serialization in the accept branch that the
   389	  dial branch does not pay.
   390	  **⚠ H1 ACCUSES CODE, NOT A PLATFORM (canonical; added 2026-07-14 after the
   391	  shorthand misled two sessions).** The word "Windows" appears nowhere above.
   392	  Windows is merely *who happens to be the accepting source* in P1's slow arm on
   393	  rig W, so other docs say "H1's Windows accept branch" as **shorthand for where
   394	  the accused code runs on that rig** — it is NOT a claim that H1 requires
   395	  Windows. Two consequences, both load-bearing: (a) **a reproduction of P1 on a
   396	  non-Windows pair does NOT kill H1** — the accused code runs there too, so it is
   397	  *consistent with* H1 (and "consistent with H1" is not confirmation, below);
   398	  (b) **a disappearance of P1 without Windows does not CONFIRM H1** either — it
   399	  would only mean the accused cost is platform-conditional, which is a further
   400	  claim. Only the dial/accept inversion counterfactual in pf-1 can settle H1.
   401	  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
   402	  was "mixed exercises resize hardest", but **all three fixtures target
   403	  eight streams before clamping** (`src/dial.rs:474`) — so resize
   404	  *count* cannot explain mixed-only behaviour, and H1 must name what
   405	  about mixed differs (shard-boundary timing? the tar-shard small half
   406	  interleaving with the big-file stream at the moment epochs fire?) or
   407	  be killed. **H1 also names the wrong half without proof**: it accuses
   408	  `Accept` while the destination's **synchronous dial-before-ACK** path
   409	  (`destination_session`'s `Frame::Resize` /
   410	  `DestRecvPlane::Initiator` branch) is an equally good suspect. pf-1 must
   411	  separate them with the dial/accept inversion counterfactual below —
   412	  "consistent with H1" is not confirmation.
   413	- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
   414	  interleave cannot happen — resize begins only after
   415	  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
   416	  layouts drain the same fixed 128-entry destination need loop, so
   417	  batch emission cannot interleave with the resize controller during
   418	  manifest/need emission in either layout. Kept only as a residual: if
   419	  pf-1 timing shows a layout-dependent need-batch delta anyway, the
   420	  mechanism must be re-derived from the trace, not from this text.
   421	- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
   422	  already killed its named candidates (the small half is tar-sharded and
   423	  written with parallel per-file `create_dir_all`/`fs::write`, NO
   424	  per-file flush; per-file progress emission to the served push
   425	  destination is disabled — `remote/transfer/sink.rs`; and old push used
   426	  the same served sink, so fsync/flush policy and progress emission are
   427	  NOT old/new deltas). What was left — "dest-side directory work/handle
   428	  churn" — **names no old/new code delta at all**, and its only probe
   429	  (precreate-vs-not) is explicitly environmental and cannot attribute
   430	  code (Method 3(a)). A hypothesis that cannot be confirmed *or* killed
   431	  by pf-1 is not a hypothesis; keeping it would let pf-1 close with a
   432	  shrug. It is therefore retired, and its one code-attributable
   433	  descendant — a per-member cost on the TCP receive path that old push
   434	  did not pay — lives on as **H6**, which names an executed-path delta.
   435	  H3 may only be revived if the pf-1 trace names a concrete old/new
   436	  delta in the destination directory/handle path; the 12b cross-block
   437	  precreated-container lead (8%, NTFS) is recorded as an environmental
   438	  lead for that trace, not as an attribution.
   439	- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
   440	  unchanged since `0f922de` (`remote/transfer/data_plane.rs`; the
   441	  earlier `dial.rs` attribution was wrong), and old small push ALSO
   442	  opened at one stream (after its 128-file early flush) then resized
   443	  live — so neither framing nor "fixed-count opening" discriminates.
   444	  What survives of H4 is ramp cadence/shard-boundary timing only, and
   445	  it is subordinate to H5.
   446	- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
   447	  scan/diff/transfer overlap on the TCP plane — current code withholds
   448	  every TCP payload until `ManifestComplete`
   449	  (`transfer_session/mod.rs`), while old push negotiated and queued
   450	  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
   451	  gRPC's in-stream carrier did not change comparably — which matches
   452	  the exact signature "TCP regressed while gRPC did not" (zoey gRPC at
   453	  parity 1.001, Windows gRPC faster; NOT "gRPC uniformly at parity" —
   454	  review round 3). NOTE: an H5 fix
   455	  reorders session phases and multi-ADD/pipelined epochs conflict with
   456	  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
   457	  ordering), so any H5 fix triggers this plan's Contract
   458	  stop-and-amend rule BEFORE implementation.
   459	- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
   460	  need-claim locking on the TCP receive plane — TCP receive
   461	  (`NeedListSink`) takes a separate mutex/hash-set claim per member
   462	  (`NeedListSink::claim`, called per member by its tar-shard
   463	  `write_payload` arm), while the gRPC path claims a whole shard under one
   464	  lock (the `destination_session` `TarShardHeader` arm).
   465	  TCP-only and per-member (so small-file-heavy) — matches the P2
   466	  signature independently of H5. Discriminated by the pf-1 per-member
   467	  locking timings (Method 3(e), now unconditional).
   468	  **Historical control — corrected (review round 3): test the EXECUTED
   469	  path, not source presence.** `NeedListSink` *exists* in the tree at
   470	  `0f922de`, so "does the symbol exist there" is the wrong question and
   471	  would wrongly force H6 into a "multiplied claim frequency" story. What
   472	  matters is what old push actually RAN: at `0f922de` the served push
   473	  data plane goes `socket → StallGuard → execute_receive_pipeline →
   474	  FsTransferSink → disk`
   475	  (`crates/blit-daemon/src/service/push/data_plane.rs:185-206`) —
   476	  it **bypasses `NeedListSink` entirely** and takes no per-member claim.
   477	  So H6's claim is precise and falsifiable: the unified TCP receive path
   478	  introduced a per-member lock/hash-set claim on a path whose old
   479	  counterpart took none. pf-1 confirms it by (a) reading the executed
   480	  old path (done — cited above) and (b) the per-member locking timings;
   481	  it is KILLED if those timings do not scale with member count or do not
   482	  account for a material share of the P2 gap. If H6 is confirmed, the P2
   483	  fix bar applies unchanged (≤ 1.10 against BOTH references, BOTH rigs);
   484	  no separate bar is granted.
   485	  **H6's WALL-TIME counterfactual (added round 5 — timings alone would
   486	  strand pf-1 under the uniform decision rule):** behind a debug flag,
   487	  claim the whole tar shard under ONE lock on the TCP receive path —
   488	  i.e. give TCP the same batch-claim shape the gRPC path already uses
   489	  (`transfer_session/mod.rs:3047`), rather than a per-member claim
   490	  (`data_plane.rs:1167`). This is safe and wire-neutral (it changes only
   491	  the granularity of a local mutex/hash-set claim, not any frame), so it
   492	  does NOT trip the Contract rule. Grade its recovery against `Δ_P2` on
   493	  the uniform scale. If per-member claiming is the cost, batch-claiming
   494	  recovers it; if not, H6 dies with a number rather than a shrug.
   495
   496	- **H7 (P2; added by review round 4 — the SHARED-controller candidate
   497	  the gRPC caveat predicted)**: HEAD's need/manifest bookkeeping is
   498	  heavier than old push's per entry. The unified source keeps a
   499	  **mutex-protected sent-manifest map** with per-entry insertion and
   500	  removal, and routes each need through a **per-need event-channel hop**
   501	  (`transfer_session/mod.rs:1038`, `:1123`, `:1350`); old push used a
   502	  **task-local map and handled need batches inline**, with no lock and no
   503	  channel hop per entry. This is **per-entry**, so it scales with FILE
   504	  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
   505	  **shared by BOTH carriers**. That is the precise class the round-3
   506	  gRPC caveat warned about: a shared regression can hide under gRPC's
   507	  larger carrier-specific gain, so "TCP-only symptom" does NOT exonerate
   508	  shared code. No prior hypothesis tested it. Discriminated by: per-entry
   509	  bookkeeping timings scaled against file count, plus the wall-time
   510	  counterfactual (a task-local/batch-inline path behind a debug flag).
   511	  H7 and H6 are independent and may BOTH contribute.
   512
   513	## Method (the investigation slice — no behavior changes)
   514
   515	1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
   516	   in-process/two-process rigs on the Mac with the otp-2 fixture shapes.
   517	   P1 uses the wire-neutral, low-frequency structured timeline enabled by
   518	   `BLIT_TRACE_SESSION_PHASES=1` on both processes (same
   519	   `BLIT_TRACE_RUN_ID`): resize epochs (arm queue→ready→accept/dial→ack),
   520	   need-batch emission, planner in/out, per-socket first write/receive, and
   521	   completion. The older `--trace-data-plane` output is NOT a timing input:
   522	   it is initiator-only and may emit per file. P2's per-member sink
   523	   open/write/close, claim-lock, and tar-shard timings are a separate
   524	   high-volume probe slice so they cannot perturb the focused P1 observer.
   525	   This P1 trace slice alone does not satisfy the pf-1 HARD GATE below.
   526	2. **A/B the role layouts in one process**: the generic otp-3 role helper
   527	   forces the in-stream carrier, but `transfer_session_roles.rs` already
   528	   contains real loopback-TCP tests for both initiator layouts. The
   529	   timing-harness variant MUST reuse or factor that TCP harness; it reports
   530	   phase timings per layout for mixed and small fixtures. A positive
   531	   layout-dependent delta in a named phase confirms; local ABSENCE
   532	   does not kill H1 (loopback removes the Windows↔Mac topology). So
   533	   that H1 stays falsifiable: if the local run is negative, pf-1
   534	   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
   535	   CELLS fixtures) before pf-1 may close — every hypothesis exits
   536	   pf-1 confirmed or killed, never "unfalsified" (review round 2).
   537	3. **Historical control, then bisect P2**: old push is deleted from
   538	   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
   539	   build and run; the control is an old-vs-new run on identical
   540	   fixtures. The new tracing spans do NOT exist in `0f922de` (review
   541	   round 2), so the control is observed externally — phase boundaries
   542	   from wire + filesystem timestamps and stdout progress, with event
   543	   semantics mapped span-for-span to the new names — or, where that is
   544	   too coarse, a minimal probe backport onto the pinned `0f922de`
   545	   source with identical event names. Either way every timed
   546	   configuration runs an instrumentation-on/off pair to bound observer
   547	   overhead (per-member tracing across ~10k files can perturb a
   548	   double-digit share of the measured gap). Experiments, corrected per
   549	   review 2026-07-12: (a) precreate-vs-not stays but is
   550	   environmental-only (it cannot attribute code); (b) the flush/
   551	   instrument toggles missed the tar-shard path — instrument the
   552	   tar-shard write path itself; (c) REPLACED (review round 2) — the
   553	   ramp pin discriminated nothing (old push also opened at one
   554	   stream), but H4 keeps a code-level counterfactual: a batch-cadence
   555	   replay toggle that processes need batches at the recorded old-push
   556	   shard-boundary cadence; (d) NEW, for H5 — the overlap experiment,
   557	   metric DEFINED (review round 2: "manifest-complete→first-payload
   558	   gap" was underdefined, and for old push the quantity is expected to
   559	   be NEGATIVE, which an unsigned "gap" cannot express). Record, per
   560	   run, on ONE common clock with a SIGNED offset from the
   561	   `ManifestComplete` event, three separately-named events on the
   562	   source side plus one on the destination:
   563	   `t_manifest_complete`; `t_first_payload_queued` (the payload enters
   564	   the send queue); `t_first_socket_write` (first byte handed to the
   565	   TCP data plane); `t_first_payload_received` (destination side —
   566	   requires the two clocks to be reconciled, so record the ssh/NTP
   567	   offset per run and report it with the number, or state that the
   568	   destination event was not usable). The overlap DIFFERENCE is
   569	   established only if `t_first_socket_write − t_manifest_complete` is
   570	   ≈0-or-positive on the new build and provably NEGATIVE on the pinned
   571	   `0f922de` control for the SAME fixture — i.e. old push really did put
   572	   TCP bytes on the wire before its manifest completed, and the new
   573	   session does not.
   574	   **That timestamp proves ORDERING, not CAUSATION, so it cannot confirm
   575	   H5 (review round 3).** H5 is confirmed only by a causal
   576	   counterfactual: a debug-flag toggle that restores mid-manifest TCP
   577	   payload queueing (queueing/ordering only — if it cannot be done
   578	   without a wire change, this plan's Contract stop-and-amend rule fires
   579	   FIRST) and measures WALL TIME on the same fixture and rig,
   580	   interleaved old-vs-new. Pre-registered: H5 is CONFIRMED iff the
   581	   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
   582	   KILLED if it restores the old ordering but does not move wall time —
   583	   which would prove the lost overlap is real and irrelevant, and hand
   584	   P2 to H6;
   585	   (e) per-member locking/framing timings are now an unconditional pf-1
   586	   measurement (they discriminate H6), not contingent on the trace
   587	   implicating them.
   588	4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
   589	   local rig is Mac↔Mac loopback: it removes the very platform terms P1
   590	   is confounded with, and it may equally fail to surface P2 (whose
   591	   Windows arms are the sharpest). So the rule is symmetric — **if a
   592	   finding does not reproduce locally, pf-1 REQUIRES the rig-side
   593	   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
   594	   since P2 was measured on both) with the same spans and the CELLS
   595	   fixtures, before pf-1 may close. Every hypothesis exits pf-1
   596	   confirmed or killed — never "did not reproduce, moving on".
   597	5. Every experiment lands as a committed probe record under
   598	   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   599	   loop per slice as usual.
   600
   601	## pf-1 decision rule — UNIFORM, pre-registered (added round 5)
   602
   603	Round-4 review: individual hypotheses had no shared decision threshold —
   604	H1 accepted any positive phase delta, H4's cadence replay had no
   605	threshold, H5 left a 1–49% recovery undecided, H6 left "material share"
   606	undefined. A phase-timing delta is **descriptive**; only wall time
   607	decides. So ONE rule governs every hypothesis (H1, H4, H5, H6, H7):
   608
   609	- Each hypothesis must have a **wall-time counterfactual**: a debug-flag
   610	  variant that removes or restores exactly the accused mechanism, run
   611	  interleaved against the unmodified build on the same rig and fixture.
   612	  A hypothesis with no counterfactual **cannot be confirmed** — it is
   613	  carried as UNTESTED and pf-1 does not close.
   614	- **`Δ` is defined per finding and per rig — it is NOT one number**
   615	  (review round 5: the earlier text left it ambiguous between P1's
   616	  layout gap and P2's old/new gap, which are different quantities):
   617	  - **`Δ_P1(rig)`** = `destinit_median − srcinit_median` for
   618	    `wm_tcp_mixed` on THAT rig (an invariance gap: new-vs-new, no old
   619	    build involved). On rig W it is 1221 − 939 = **282 ms** — a **single
   620	    nagatha session**; §pf-0 re-estimates it from four sessions on the `q`
   621	    pairing, rules out **between-session** grading of any counterfactual, and
   622	    requires pf-1 to measure its own **paired within-session** floor before
   623	    grading. Read §pf-0 before grading any recovery against `Δ_P1`. On
   624	    magneto↔skippy it is ~0 (8/8 pass) — so
   625	    **P1 counterfactuals are graded on rig W only**; a Linux-rig recovery is
   626	    meaningless against a gap that does not exist there.
   627	  - **`Δ_P2(rig)`** = `new_median − old_same_session_median` for
   628	    `push_tcp_small` on THAT rig (a converge gap, requires the `0f922de`
   629	    build on that rig). netwatch-01: 1975 − 1644 = **331 ms**; zoey:
   630	    4033 − 3636 = **397 ms**.
   631	  Every reported recovery names its `Δ` and its rig. A counterfactual run
   632	  on a rig whose `Δ` is ~0 proves nothing and is not reported as a kill.
   633	- **Overlapping causes are attributed SEQUENTIALLY, never summed**
   634	  (review round 5: H4/H7, and H6/H7, can each recover the same
   635	  milliseconds, so independent recoveries would double-count and could
   636	  "explain" >100% of `Δ`). Procedure: grade each hypothesis's recovery
   637	  ALONE against the unmodified build; then, for every confirmed
   638	  hypothesis in descending order of solo recovery, measure the
   639	  **incremental** recovery of adding it to the already-applied set. The
   640	  ≥70% closure test below is evaluated on the **cumulative combined**
   641	  build, not on the sum of solo recoveries.
   642	- The counterfactual's wall-time recovery `r` (as a share of the named
   643	  `Δ`) is graded on a **pre-registered scale**, no post-hoc bands:
   644	  - `r ≥ 50%` → **CONFIRMED DOMINANT** (fix it first)
   645	  - `20% ≤ r < 50%` → **CONFIRMED CONTRIBUTING** (fix it, but it is not
   646	    the whole story — keep hunting)
   647	  - `r < 20%` → **KILLED** as a material cause (recorded, not pursued)
   648	- **pf-1 closes only when the confirmed contributions account for ≥ 70%
   649	  of `Δ`** for each finding. If they do not, the residue is unexplained
   650	  and pf-1 **stays open** with the shortfall stated in the probe record —
   651	  never "several hypotheses were consistent, moving on".
   652	- Every measurement runs instrumentation-on/off pairs (per-member tracing
   653	  across ~10k files can itself perturb a double-digit share of `Δ`).
   654
   655	## Fix criteria (pre-registered; the owner walks the final numbers)
   656
   657	- **The global rule dominates every bar below** (review round 2 flagged
   658	  a contradiction between "necessary, not sufficient" and the `⇔`
   659	  bars — the `⇔`s are hereby scoped as *definitions of the named
   660	  finding's own bar*, never as a sufficient condition for acceptance).
   661	  Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in
   662	  EVERY acceptance cell passes independently against BOTH its
   663	  same-session reference AND the committed baseline — no arm may exceed
   664	  1.10 against either reference even when its counterpart bar passes
   665	  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
   666	  P2 bars below but regresses any other cell against either reference is
   667	  **not** accepted.
   668	- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
   669	  `pull_tcp_mixed` ≤ 1.10 against BOTH references on the netwatch-01
   670	  rig (CELLS escalation session, RUNS=8), with `wm_grpc_mixed` and the
   671	  other invariance PASSes unregressed against both references. (Meeting
   672	  this bar does not by itself accept the build — see the global rule.)
   673	- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
   674	  (same-session AND committed) on BOTH rigs (CELLS sessions), with the
   675	  gRPC small-push cells unregressed. **"Unregressed" is given a
   676	  reference and a tolerance (review round 3)**: each gRPC small-push
   677	  cell must stay ≤ 1.10 against both of its own references AND must not
   678	  worsen by more than **10% against its own pre-fix median on the same
   679	  rig** (zoey 4731 ms; netwatch-01 2264 ms at 12c-win). The second
   680	  clause exists because those cells currently range 0.801–1.001 — a fix
   681	  that dragged Windows gRPC from 0.85 back to 1.05 would still pass a
   682	  bare ≤1.10 bar while having eaten a real, measured win.
   683	- Cross-direction converge-up is a SEPARATE bar (review round 2):
   684	  every final cross-direction row must still meet the parent plan's
   685	  new-vs-old ceiling (`ONE_TRANSFER_PATH.md` acceptance) or satisfy
   686	  the registered platform-residue discriminator — invariance plus the
   687	  per-direction bars alone would pass if a "fix" slowed BOTH layouts
   688	  equally, violating converge-up.
   689	- No suite regressions; the test count may not drop from the immediately
   690	  preceding reviewed workspace baseline recorded by the repo. Any new pins
   691	  carry guard proofs (temporary revert) per the loop.
   692	- If investigation attributes part of a gap to something the plan's
   693	  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
   694	  that residue is RECORDED with its experiment and goes to the owner's
   695	  otp-13 walk — never silently accepted.
   696
   697	## Staging (each through the codex loop)
   698
   699	- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
   700	  the two-layout phase-timing report (TCP-carrier mode included) + the
   701	  `0f922de` historical control; probe record committed AND
   702	  codex-reviewed BEFORE any pf-2 branch exists. No fix lands on
   703	  pre-pf-1 evidence.
   704	- **pf-2..n**: one fix slice per confirmed root cause (smallest
   705	  change that moves the phase timing; A/B'd locally before rig time).
   706	- **pf-final**: NOT just the two escalation cells — the final build
   707	  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
   708	  gRPC controls) on **all THREE rigs: Z (zoey), W (netwatch-01) and
   709	  D (delegated, netwatch-01↔skippy)**. **No mixed-build evidence: every
   710	  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
   711	  (corrected, review round 2 — "every row" was impossible: the
   712	  same-session `old` arms and the committed baselines are OLD builds by
   713	  construction, which is the entire point of a reference). Pre-fix
   714	  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
   715	  which are **replication and control evidence, not acceptance
   716	  evidence**.
   717	  **Rig D is included even though it is not a suspect (review round
   718	  3).** Voiding otp-12c's pre-fix rows while re-running only Z and W
   719	  would leave the parent plan's **delegated-parity bar**
   720	  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
   721	  evidence at all. "Not implicated" scopes what pf-1 must
   722	  *instrument* — it does not waive an acceptance bar. Rig D's TCP
   723	  verdict cells (+ the gRPC smoke) therefore rerun on the final build;
   724	  both arms are new-build by construction there (rig D has no old
   725	  baseline), so the whole cell is re-measured.
   726	  **Every gRPC row the acceptance method requires reruns
   727	  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
   728	  earlier "if shared code changed, the gRPC cells rerun too" left the
   729	  decision to the author's own judgement of what counts as shared, which
   730	  is exactly the loophole H7 exploits: a shared regression can hide under
   731	  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
   732	  complete Z/W gRPC converge and invariance rows, so those are
   733	  final-build rows, full stop — no conditional. Results land in fresh
   734	  dated evidence dirs. **Then** otp-12d assembles the matrix from
   735	  final-build rows, and the otp-13 owner walk reads it.
   736
   737	## Known gaps
   738
   739	- H1–H5 were graded against the actual tree by codex review
   740	  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
   741	  The old drivers are deleted from HEAD, but the pinned `0f922de`
   742	  source/binaries diff and run fine — historical claims get live
   743	  controls in pf-1, not pin-archaeology.
   744	- zoey never measured P1: its rig anchors converge-up only, so there
   745	  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
   746	  says nothing about layout asymmetry (review 2026-07-12). pf-1's
   747	  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
   748	  qualifies per the 12b wm numbers).
   749	- **The 12c-win rows are replication, not acceptance** (2026-07-13).
   750	  They are pre-fix by definition, so `pf-final` voids them for
   751	  acceptance; their value is that they (a) reproduce P1 and P2 on an
   752	  independent session at the shipped sha, (b) supply the
   753	  opposite-direction control (`mw_tcp_mixed` 1.044 PASS vs
   754	  `wm_tcp_mixed` 1.300 FAIL — same carrier, same fixture) that narrows
   755	  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
   756	  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
   757	  P2 1.149→1.201), so neither is drifting toward the bar on its own.
   758	- **Rig-D delegated parity is not a SUSPECT, but it is still an
   759	  ACCEPTANCE bar** (2026-07-13; scoped correctly at review round 3): the
   760	  delegated-vs-direct matrix passed 7/7

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:.review/findings/otp12-pf1-rigw-harness.md | sed -n '225,380p' | nl -ba -v225; git diff --unified=80 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946 -- crates/blit-core/src/transfer_session/data_plane.rs crates/blit-core/tests/transfer_session_roles.rs" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-dMHRtdXQ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-QSwS9kS5' (errno=Operation not permitted)
   225	## Known gaps
   226
   227	- No rig datum is produced by this slice. The full live run waits for fresh
   228	  mandatory Codex adjudication, exact isolated builds, a successful live
   229	  launcher smoke, and a green endpoint preflight.
   230	- This four-cell run is the reduced P1 phase diagnostic, not the entire pf-1
   231	  hard gate. The active plan still requires the separately reviewed
   232	  small-fixture/P2 work, phase report, and `0f922de` historical control before
   233	  pf-1 closes.
   234	- q was not quiet during the first read-only readiness sample on 2026-07-15:
   235	  Time Machine AutoBackup was enabled and Spotlight was using substantial CPU.
   236	  The harness reports and refuses those conditions; it does not mutate them.
   237
   238	## Reviewer comments
   239
   240	Initial Codex review (`gpt-5.6-sol`, `xhigh`, codex-cli 0.144.4) reviewed
   241	`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..0fb8237c2e6f63feb9cfc613d8af1602730061b0`
   242	and returned `NEEDS FIXES` with three High findings. All three were accepted
   243	and fixed independently: destination reset fail-closed at `661cf75`, excess
   244	settle accounting at `1617546`, and the complete resize causal-edge audit plus
   245	emitter alignment at `2dd977e`. See the raw review and adjudication under
   246	`.review/results/otp12-pf1-rigw-harness.*`.
   247
   248	Round-2 Codex reviewed the complete immutable range through `8fbd486` and
   249	returned `NEEDS FIXES`: it independently confirmed F1–F3 closed, then found two
   250	new High defects. F4 is an uncharged Windows-client interval before q captures
   251	the settle anchor. F5 is the role-bearing `rid` selecting different physical
   252	destination paths for paired arms, contrary to the only-initiator-varies
   253	contract. Both were accepted and fixed in order: F5 at `1231e42`, then F4 at
   254	`6ba5408`. A separate runbook audit found the missing standalone launcher mode,
   255	fixed at `18d3cde`; follow-up safety audit found the pre-PID-journal CIM race,
   256	fixed at `454ebce`. The additive Grok second eye returned a schema-valid
   257	`ACCEPTED` verdict with three independent red-to-green guards, but it does not
   258	override the mandatory Codex findings. See the round-2 raw and adjudication
   259	records under `.review/results/otp12-pf1-rigw-harness-r2.*`. Fresh review of
   260	the complete fixed range is pending; no rig run is authorized yet.
   261
   262	Round-3 Codex reviewed
   263	`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..53bb5e56a864abe0ee2d2b00c411846a1e7d24d5`
   264	and returned `PASS` with no findings. The additive Grok review of the same
   265	immutable range returned schema-valid `REOPENED`, `guard_confirmed=false`.
   266	G3 is accepted: production role-invariant path construction is correct, but
   267	the path-construction/parity assertions are bare `[[ ... ]]` commands that can
   268	survive failure under macOS Bash 3.2. Grok's role-in-path mutation produced
   269	different physical destinations while `SELFTEST=1` still exited zero. The
   270	timing-anchor and launcher-journal mutations independently went red-to-green.
   271	See `.review/results/otp12-pf1-rigw-harness-r3.*`. G3 was fixed at `27c94b0`;
   272	the complete range still requires fresh review before any rig activity.
   273
   274	Coder follow-up audit admitted G4 as a separate High instrument-correctness
   275	finding. Destination-type, finalization-state, strict-cleanup-state,
   276	completion-marker-removal, and signal-cleanup checks still used bare
   277	`[[ ... ]]` assertions that macOS Bash 3.2 can let fall through to a later
   278	successful command. A regression could therefore leave an unsafe destination
   279	type, false cleanup state, or stale completion marker while the offline
   280	self-test still exited zero. G4 gives each material lifecycle assertion an
   281	explicit failure path and seeds the signal test with a completion marker, so
   282	that its absence check is not vacuous. Final-command subshell predicates and
   283	intentional predicate returns are unchanged. Removing the production
   284	`SESSION_FINALIZED=1`, retaining `Q_SESSION_MAY_EXIST=1` after successful
   285	cleanup, or conditionally skipping completion-marker removal for a received
   286	signal each turns the Bash 3.2 self-test red at the intended assertion;
   287	restoring all three returns it to green.
   288
   289	G4 was fixed at `7e9d2d5`. The full workspace format, strict-clippy, and test
   290	gate; 23 analyzer tests; Bash syntax and self-test; documentation gate; and
   291	diff check are green for both G3 and G4. No endpoint was contacted.
   292
   293	Round-4 mandatory Codex and additive Grok reviewed the complete immutable
   294	range through `6f517ea1bdbea2f7d83f15c086d2bf5f764cf524`. Codex returned
   295	`PASS` with no material finding. Grok returned schema-valid `ACCEPTED`,
   296	`guard_confirmed=true`, exact SHAs, and independently drove the G3 role-path
   297	mutation plus G4 finalization, may-exist, and marker-removal mutations red
   298	before restoring every offline suite green. Its detached worktree ended clean
   299	and was removed. Review is closed; launcher smoke and endpoint preflight remain
   300	required before the registered run.
   301
   302	The first live launcher-smoke attempt on q refused before launching a daemon
   303	or timing a transfer. G5 is accepted as a High instrument-correctness finding:
   304	q legitimately has the Windows peer cached on `en0`, `en1`, and registered
   305	`en8`, but the ARP gate concatenated all three MAC rows. It therefore rejected
   306	the correct peer even though `route -n get` selected `en8`. The failed attempt
   307	is retained as `SESSION-VOID` under
   308	`logs/otp12pf-rigw-20260715T113500Z-launcher` in the isolated q clone. The fix
   309	parses exactly the registered interface, requires one result, and pins the
   310	real three-interface shape in the Bash 3.2 self-test. No daemon started and no
   311	endpoint policy changed. Removing the interface predicate makes the self-test
   312	red on the three-row fixture; restoring it returns the complete self-test to
   313	green.
   314
   315	Round-5 reviewed the complete immutable range through
   316	`06b33228d502c51da24bc2a78fba7eddcf6c0723`. Mandatory Codex independently
   317	confirmed G5, the exact 128-arm schedule, and role-invariant endpoint-local
   318	paths, then returned `NEEDS FIXES` with one separate High finding. G6 is
   319	accepted: the harness runs the endpoint's pre-existing
   320	`D:/blit-test/purge-standby.ps1` by existence and exit status only, rather
   321	than staging and hashing the reviewed repository helper. A stale or no-op
   322	helper could therefore make a warm-cache run look valid. Additive Grok
   323	returned schema-valid `ACCEPTED`, exact SHAs, and `guard_confirmed=true` for
   324	G5 after independently driving the ARP interface mutation red and restoring
   325	the Bash 3.2 self-test green. Its detached worktree ended clean and was
   326	removed. No endpoint was contacted. See the round-5 raw reviews and
   327	adjudications under `.review/results/otp12-pf1-rigw-harness-r5.*`.
   328
   329	G6 now takes the purge helper only from the exact clean q checkout. After all
   330	read-only endpoint/fabric/quiet gates pass, the harness reserves a fresh
   331	per-session Windows tree, copies the reviewed helper to a temporary path,
   332	rejects reparse points, verifies SHA-256 before and after the atomic move, and
   333	records the helper hash/path alongside the four executable hashes. Every arm
   334	rechecks that same hash immediately before invocation and requires exactly one
   335	`standby-purged` success line in addition to exit zero. The helper is therefore
   336	covered by the executable snapshot and strict session-tree cleanup rather than
   337	trusted as endpoint state.
   338
   339	The Bash 3.2 self-test functionally mocks both stage and per-arm commands.
   340	Removing the final post-move hash comparison turns it red at the staging
   341	contract; restoring it returns green. Removing the per-arm hash comparison
   342	turns it red before the mocked purge can pass; restoring it returns green. A
   343	separate order guard pins the first remote write after provenance, port,
   344	topology, MSS, firewall, quietness, timer, and result-stream checks. No endpoint
   345	was contacted by the fix or its mutation proofs.
   346
   347	G6 was fixed at `888be4754387311e28e14d687721fd3d1315f82c`.
   348	Format, strict clippy, Bash syntax/self-test, all 23 analyzer tests, the docs
   349	gate, and diff checks passed. The first full workspace test attempt hit the
   350	recorded macOS `blit_utils::test_utils_list_modules` daemon-start race once;
   351	the isolated test then passed, and a complete quiet rerun passed with two
   352	expected ignores. Fresh complete Codex plus additive Grok review is still
   353	required before any build or endpoint contact.
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-QtO8PDN1' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GA5zidIl' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/transfer_session/data_plane.rs b/crates/blit-core/src/transfer_session/data_plane.rs
index 0ecc303..95978c3 100644
--- a/crates/blit-core/src/transfer_session/data_plane.rs
+++ b/crates/blit-core/src/transfer_session/data_plane.rs
@@ -816,172 +816,172 @@ pub(super) struct SourceDataPlane {
     // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
     pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
     // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
     // advertised capacity (contract §Invariants 5). The resize drives only
     // its shape-correction stream count; the cheap-dial tuner is future
     // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
     dial: Arc<TransferDial>,
     source: Arc<dyn TransferSource>,
     session_token: Vec<u8>,
     pool: Arc<BufferPool>,
     /// `[data-plane-client]` connect traces (`--trace-data-plane`,
     /// otp-10a). Applied to the epoch-0 sockets at construction and to
     /// each epoch-N resize socket in [`Self::add_stream`].
     trace: bool,
     /// How each epoch-N resize socket is acquired (dial for the SOURCE
     /// initiator, accept for the SOURCE responder). The data plane grows
     /// mid-transfer in both cases; the control-lane resize choreography is
     /// identical — only this transport action flips (otp-5b-2).
     sockets: SourceSockets,
     phase_trace: Option<BoundSessionPhaseTrace>,
     queue_trace_armed: AtomicBool,
 }

 /// Dial the granted data plane and start the elastic send pipeline.
 /// `host` is the responder's host (the initiator connected the control
 /// plane to it; the data plane rides the same host on the granted port —
 /// contract §Transport: the initiator always dials). `receiver_capacity`
 /// is the DESTINATION's advertised profile from `SessionAccept`; it
 /// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
 /// never unlimited).
 pub(super) async fn dial_source_data_plane(
     host: &str,
     grant: &DataPlaneGrant,
     receiver_capacity: Option<&CapacityProfile>,
     source: Arc<dyn TransferSource>,
     instruments: &SourceInstruments,
     phase_trace: Option<BoundSessionPhaseTrace>,
 ) -> Result<SourceDataPlane> {
     let initial = grant.initial_streams.max(1) as usize;
     // The byte sender's dial, bounded by the receiver's advertised
     // capacity. Seed the settled live count to the granted epoch-0
     // streams — every shape-resize proposal steps from here.
     let dial = TransferDial::conservative_within(receiver_capacity).shared();
     dial.set_negotiated_streams(initial);

     // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
     let mut handshake = grant.session_token.clone();
     handshake.extend_from_slice(&grant.epoch0_sub_token);

     // Provision the pool for the dial ceiling so resize-added sockets
     // draw buffers from the same pool without re-pooling (as old push
     // does — a shared pool sized for the maximum stream count).
     let pool = Arc::new(BufferPool::for_data_plane(
         dial.chunk_bytes(),
         dial.ceiling_max_streams().max(1),
     ));
     let trace = instruments.trace_data_plane;
     let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
     for socket_id in 0..initial {
         if let Some(phase) = &phase_trace {
             phase.event(
                 "socket_dial_begin",
                 SessionPhaseFields {
                     epoch: Some(0),
                     socket: Some(socket_id as u32),
                     ..Default::default()
                 },
             );
         }
         let session = DataPlaneSession::connect(
             host,
             grant.tcp_port,
             &handshake,
             dial.chunk_bytes(),
             dial.prefetch_count(),
             trace,
             dial.tcp_buffer_bytes(),
             Arc::clone(&pool),
         )
         .await
-        .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?
-        .with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
+        .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?;
         if let Some(phase) = &phase_trace {
             phase.event(
                 "socket_dial_end",
                 SessionPhaseFields {
                     epoch: Some(0),
                     socket: Some(socket_id as u32),
                     ..Default::default()
                 },
             );
         }
+        let session = session.with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
         // The source-side sink never reads its dst_root (it only sends);
         // `root()` is consulted by the relay/receive case, not here.
         sinks.push(Arc::new(DataPlaneSink::new(
             session,
             Arc::clone(&source),
             PathBuf::new(),
         )));
     }

     let prefetch = dial.prefetch_count().max(1);
     let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
     let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
     let pipe_source = Arc::clone(&source);
     let pipe_progress = instruments.progress.clone();
     // Bounded by AbortOnDrop: a fault on the control lane that drops the
     // SourceDataPlane aborts the pipeline task instead of leaking it.
     let pipeline = AbortOnDrop::new(tokio::spawn(async move {
         execute_sink_pipeline_elastic(
             pipe_source,
             sinks,
             payload_rx,
             prefetch,
             pipe_progress.as_ref(),
             Some(control_rx),
         )
         .await
     }));
     let queue_trace_armed = phase_trace.is_some();
     Ok(SourceDataPlane {
         payload_tx: Some(payload_tx),
         control_tx,
         pipeline: Some(pipeline),
         dial,
         source,
         session_token: grant.session_token.clone(),
         pool,
         trace,
         // SOURCE initiator: each epoch-N resize socket is dialed to the
         // granted host:port.
         sockets: SourceSockets::Dial {
             host: host.to_string(),
             tcp_port: grant.tcp_port,
         },
         phase_trace,
         queue_trace_armed: AtomicBool::new(queue_trace_armed),
     })
 }

 /// Accept the granted epoch-0 socket(s) off a bound responder listener and
 /// start the elastic SEND pipeline over them — the SOURCE **responder**
 /// half of the pull data plane (otp-5b-1). Symmetric with
 /// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
 /// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
 /// socket acquisition differs (accept here, dial there).
 /// `DataPlaneSession::from_stream` builds a send session from an already-
 /// accepted socket — the same primitive the old `pull_sync` daemon-send
 /// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
 /// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
 /// wherever it initiates). The bound listener is retained so each epoch-N
 /// resize socket is accepted off it (otp-5b-2): the DESTINATION initiator
 /// dials, this end accepts, the control-lane frames identical to push.
 pub(super) async fn accept_source_data_plane(
     bound: ResponderDataPlane,
     receiver_capacity: Option<&CapacityProfile>,
     source: Arc<dyn TransferSource>,
     instruments: &SourceInstruments,
     phase_trace: Option<BoundSessionPhaseTrace>,
 ) -> Result<SourceDataPlane> {
     let initial = bound.initial_streams.max(1) as usize;
     // The byte sender's dial, bounded by the receiver's advertised
     // capacity; seed the live count to the granted epoch-0 streams. Growth
     // is via resize (otp-5b-2): the accept-based epoch-N socket steps from
     // here, one stream per epoch, same as the SOURCE initiator.
     let dial = TransferDial::conservative_within(receiver_capacity).shared();
     dial.set_negotiated_streams(initial);

     // Epoch-0 credential the dialing DESTINATION presents:
     // session_token ‖ epoch0_sub_token (contract §Transport).
     let mut epoch0 = bound.session_token.clone();
     epoch0.extend_from_slice(&bound.epoch0_sub_token);
@@ -1070,173 +1070,172 @@ impl SourceDataPlane {
     pub(super) fn phase_trace(&self) -> Option<&BoundSessionPhaseTrace> {
         self.phase_trace.as_ref()
     }

     /// The live dial (the byte sender owns it). The driver reads
     /// `live_streams()` for observability and calls `resize_settled` as
     /// each proposal completes.
     pub(super) fn dial(&self) -> &Arc<TransferDial> {
         &self.dial
     }

     /// sf-2 shape correction: propose one ADD toward the stream count the
     /// accumulated need list implies, if none is in flight and the shape
     /// wants more than the current live count. Mints the resize
     /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
     /// the record back on the matching ack.
     pub(super) fn propose_resize(
         &self,
         needed_bytes: u64,
         needed_count: usize,
     ) -> Result<Option<PendingResize>> {
         let desired =
             initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
                 as usize;
         let Some(proposal) = self.dial.propose_shape_resize(desired) else {
             return Ok(None);
         };
         let sub_token = generate_sub_token()
             .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
         Ok(Some(PendingResize {
             epoch: proposal.epoch,
             target_streams: proposal.target_streams as u32,
             sub_token,
         }))
     }

     /// Acquire the epoch-N data socket for an accepted resize and hand it
     /// to the running pipeline (`SinkControl::Add`). The SOURCE initiator
     /// (push) DIALS it; the SOURCE responder (pull, otp-5b-2) ACCEPTS the
     /// socket the DESTINATION initiator dials after its ack, off the same
     /// listener epoch-0 came in on. A dial/accept failure is FATAL
     /// (fail-fast): a same-build peer that established epoch-0 failing an
     /// epoch-N socket is a transport fault worth surfacing — and faulting
     /// the session aborts the peer's counterpart via AbortOnDrop, so no
     /// slot orphans. (Old push recovers non-fatally via an arm TTL; the
     /// session trades that for simplicity — noted in the finding doc.) If
     /// the pipeline is already gone (transfer completing under the ADD),
     /// the just-acquired socket is closed cleanly so the peer's worker sees
     /// its END, not a reset.
     ///
     /// The accept is bounded and unambiguous: at most one resize is in
     /// flight (the driver's `pending_resize`) and epoch-0 is already
     /// accepted, so the next connection off the listener is exactly this
     /// resize's socket — verified against `session_token ‖ sub_token`.
     pub(super) async fn add_stream(&self, epoch: u32, sub_token: &[u8]) -> Result<()> {
         let session = match &self.sockets {
             SourceSockets::Dial { host, tcp_port } => {
                 let mut handshake = self.session_token.clone();
                 handshake.extend_from_slice(sub_token);
                 if let Some(phase) = &self.phase_trace {
                     phase.event(
                         "socket_dial_begin",
                         SessionPhaseFields {
                             epoch: Some(epoch),
                             socket: Some(0),
                             ..Default::default()
                         },
                     );
                 }
                 let session = DataPlaneSession::connect(
                     host,
                     *tcp_port,
                     &handshake,
                     self.dial.chunk_bytes(),
                     self.dial.prefetch_count(),
                     self.trace,
                     self.dial.tcp_buffer_bytes(),
                     Arc::clone(&self.pool),
                 )
                 .await
-                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?
-                .with_session_phase_trace(self.phase_trace.clone(), epoch, 0);
+                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?;
                 if let Some(phase) = &self.phase_trace {
                     phase.event(
                         "socket_dial_end",
                         SessionPhaseFields {
                             epoch: Some(epoch),
                             socket: Some(0),
                             ..Default::default()
                         },
                     );
                 }
-                session
+                session.with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
             }
             SourceSockets::Accept { listener } => {
                 let mut expected = self.session_token.clone();
                 expected.extend_from_slice(sub_token);
                 if let Some(phase) = &self.phase_trace {
                     phase.event(
                         "socket_accept_begin",
                         SessionPhaseFields {
                             epoch: Some(epoch),
                             socket: Some(0),
                             ..Default::default()
                         },
                     );
                 }
                 let socket = accept_authenticated(listener, &expected).await?;
                 if let Some(phase) = &self.phase_trace {
                     phase.event(
                         "socket_accept_end",
                         SessionPhaseFields {
                             epoch: Some(epoch),
                             socket: Some(0),
                             ..Default::default()
                         },
                     );
                 }
                 DataPlaneSession::from_stream(
                     socket,
                     self.trace,
                     self.dial.chunk_bytes(),
                     self.dial.prefetch_count(),
                     Arc::clone(&self.pool),
                 )
                 .await
                 .with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
             }
         };
         let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
             session,
             Arc::clone(&self.source),
             PathBuf::new(),
         ));
         if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
             if let SinkControl::Add(sink) = returned.0 {
                 let _ = sink.finish().await;
             }
         }
         Ok(())
     }

     /// Feed one planned payload into the bounded send pipeline. The caller
     /// selects this send against control events so resize acknowledgements
     /// keep growing the same work-stealing queue under backpressure.
     pub(super) async fn queue(&self, payload: TransferPayload) -> Result<()> {
         let tx = self.payload_tx.as_ref().ok_or_else(|| {
             eyre::Report::new(SessionFault::internal("data plane already finished"))
         })?;
         if self.phase_trace.is_none() || !self.queue_trace_armed.load(Ordering::Relaxed) {
             return tx
                 .send(payload)
                 .await
                 .map_err(|_| dp_fault("data-plane send pipeline closed before all payloads sent"));
         }
         let permit = tx
             .reserve()
             .await
             .map_err(|_| dp_fault("data-plane send pipeline closed before all payloads sent"))?;
         let queued_at = if self.queue_trace_armed.load(Ordering::Relaxed)
             && self
                 .queue_trace_armed
                 .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
                 .is_ok()
         {
             self.phase_trace.as_ref().map(BoundSessionPhaseTrace::stamp)
         } else {
             None
         };
         permit.send(payload);
         if let (Some(trace), Some(queued_at)) = (&self.phase_trace, queued_at) {
             trace.first_payload_queued_at(queued_at);
         }
diff --git a/crates/blit-core/tests/transfer_session_roles.rs b/crates/blit-core/tests/transfer_session_roles.rs
index 734301a..e09d794 100644
--- a/crates/blit-core/tests/transfer_session_roles.rs
+++ b/crates/blit-core/tests/transfer_session_roles.rs
@@ -1668,161 +1668,164 @@ fn assert_phase_trace_partial_order(events: &[SessionPhaseEvent], initiator: Tra
             attached_keys.iter().copied().collect::<BTreeSet<_>>().len(),
             "duplicate {role:?} socket trace attachment"
         );
         assert_eq!(
             attached_keys.into_iter().collect::<BTreeSet<_>>(),
             expected_attached,
             "every acquired {role:?} socket must carry the phase trace"
         );
     }

     let need_begin = one_phase_batch(events, destination, "need_batch_send_begin", 0);
     let need_sent = one_phase_batch(events, destination, "need_batch_sent", 0);
     one_phase_batch(events, source, "need_batch_received", 0);
     assert!(need_begin.elapsed_ns <= need_sent.elapsed_ns);
     assert!(
         phase_position(events, destination, "need_batch_send_begin", None, Some(0),)
             < phase_position(events, source, "need_batch_received", None, Some(0),)
     );
     let planner_begin = one_phase_batch(events, source, "planner_begin", 0);
     let planner_end = one_phase_batch(events, source, "planner_end", 0);
     assert!(planner_begin.elapsed_ns <= planner_end.elapsed_ns);

     let proposed = one_phase_event(events, source, "resize_proposed", Some(1));
     let resize_send_begin = one_phase_event(events, source, "resize_send_begin", Some(1));
     let resize_sent = one_phase_event(events, source, "resize_sent", Some(1));
     let resize_received = one_phase_event(events, destination, "resize_received", Some(1));
     let prepared = one_phase_event(events, destination, "destination_prepared", Some(1));
     let ack_send_begin = one_phase_event(events, destination, "resize_ack_send_begin", Some(1));
     let ack_sent = one_phase_event(events, destination, "resize_ack_sent", Some(1));
     let ack_received = one_phase_event(events, source, "resize_ack_received", Some(1));
     let settled = one_phase_event(events, source, "source_settled", Some(1));
     assert!(proposed.elapsed_ns <= resize_send_begin.elapsed_ns);
     assert!(resize_send_begin.elapsed_ns <= resize_sent.elapsed_ns);
     assert!(
         phase_position(events, source, "resize_send_begin", Some(1), None,)
             < phase_position(events, destination, "resize_received", Some(1), None,)
     );
     assert!(resize_received.elapsed_ns <= prepared.elapsed_ns);
     assert!(prepared.elapsed_ns <= ack_send_begin.elapsed_ns);
     assert!(ack_send_begin.elapsed_ns <= ack_sent.elapsed_ns);
     assert!(
         phase_position(events, destination, "resize_ack_send_begin", Some(1), None,)
             < phase_position(events, source, "resize_ack_received", Some(1), None,)
     );
     assert!(ack_received.elapsed_ns <= settled.elapsed_ns);
     assert_eq!(prepared.accepted, None);
     assert_eq!(settled.accepted, Some(true));

     let (source_epoch0, destination_epoch0, source_epoch1, destination_epoch1) = match initiator {
         TransferRole::Source => {
             assert_eq!(prepared.action, Some("arm_queued"));
             let arm_queue_begin =
                 one_phase_event(events, destination, "resize_arm_queue_begin", Some(1));
             let arm_ready = one_phase_event(events, destination, "resize_arm_ready", Some(1));
             let accept_begin = one_phase_event(events, destination, "socket_accept_begin", Some(1));
             assert!(arm_queue_begin.elapsed_ns <= prepared.elapsed_ns);
             assert!(
                 phase_position(events, destination, "resize_arm_queue_begin", Some(1), None,)
                     < phase_position(events, destination, "resize_arm_ready", Some(1), None,)
             );
             assert!(arm_ready.elapsed_ns <= accept_begin.elapsed_ns);
             ("dial", "accept", "dial", "accept")
         }
         TransferRole::Destination => {
             assert_eq!(prepared.action, Some("dial_complete"));
             let dial_end = one_phase_event(events, destination, "socket_dial_end", Some(1));
             assert!(dial_end.elapsed_ns <= prepared.elapsed_ns);
             ("accept", "dial", "accept", "dial")
         }
         TransferRole::Unspecified => unreachable!(),
     };

     for (role, action, epoch) in [
         (source, source_epoch0, 0),
         (destination, destination_epoch0, 0),
         (source, source_epoch1, 1),
         (destination, destination_epoch1, 1),
     ] {
         let begin = one_phase_event(events, role, &format!("socket_{action}_begin"), Some(epoch));
         let end = one_phase_event(events, role, &format!("socket_{action}_end"), Some(epoch));
+        let attached = one_phase_event(events, role, "socket_trace_attached", Some(epoch));
         assert!(begin.elapsed_ns <= end.elapsed_ns);
+        assert!(end.producer_seq < attached.producer_seq);
+        assert!(end.elapsed_ns <= attached.elapsed_ns);
     }
     let socket_begin = one_phase_event(
         events,
         source,
         &format!("socket_{source_epoch1}_begin"),
         Some(1),
     );
     let socket_end = one_phase_event(
         events,
         source,
         &format!("socket_{source_epoch1}_end"),
         Some(1),
     );
     assert!(ack_received.elapsed_ns <= socket_begin.elapsed_ns);
     assert!(socket_begin.elapsed_ns <= socket_end.elapsed_ns);
     assert!(socket_end.elapsed_ns <= settled.elapsed_ns);

     let source_complete = one_phase_event(events, source, "data_plane_complete", None);
     let destination_complete = one_phase_event(events, destination, "data_plane_complete", None);
     let summary_begin = one_phase_event(events, destination, "summary_send_begin", None);
     let summary_sent = one_phase_event(events, destination, "summary_sent", None);
     one_phase_event(events, source, "summary_received", None);
     assert!(
         source_complete.elapsed_ns
             <= one_phase_event(events, source, "summary_received", None).elapsed_ns
     );
     assert!(destination_complete.elapsed_ns <= summary_begin.elapsed_ns);
     assert!(summary_begin.elapsed_ns <= summary_sent.elapsed_ns);
     assert!(
         phase_position(events, destination, "summary_send_begin", None, None)
             < phase_position(events, source, "summary_received", None, None)
     );
 }

 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
 async fn session_phase_trace_is_complete_and_inert_under_both_initiators() {
     let source_off = run_phase_trace_case(TransferRole::Source, false).await;
     let source_on = run_phase_trace_case(TransferRole::Source, true).await;
     let destination_off = run_phase_trace_case(TransferRole::Destination, false).await;
     let destination_on = run_phase_trace_case(TransferRole::Destination, true).await;

     assert!(source_off.events.is_empty());
     assert!(destination_off.events.is_empty());
     for (off, on) in [
         (&source_off, &source_on),
         (&destination_off, &destination_on),
     ] {
         assert_eq!(off.summary, on.summary);
         assert_eq!(off.needed_paths, on.needed_paths);
         assert_eq!(off.data_plane_streams, on.data_plane_streams);
         assert_eq!(off.tree, on.tree);
         assert_eq!(on.data_plane_streams, Some(2));
     }
     assert_eq!(source_on.summary, destination_on.summary);
     assert_eq!(source_on.needed_paths, destination_on.needed_paths);
     assert_eq!(source_on.tree, destination_on.tree);

     let source_session_id = source_on.events[0].session_id.clone();
     let destination_session_id = destination_on.events[0].session_id.clone();
     assert_ne!(
         source_session_id, destination_session_id,
         "independent sessions need distinct correlation fingerprints"
     );

     for (case, initiator, initiator_phase_role) in [
         (&source_on, TransferRole::Source, SessionPhaseRole::Source),
         (
             &destination_on,
             TransferRole::Destination,
             SessionPhaseRole::Destination,
         ),
     ] {
         let session_ids: BTreeSet<_> = case
             .events
             .iter()
             .map(|event| event.session_id.as_str())
             .collect();
         assert_eq!(session_ids.len(), 1);
         let session_id = *session_ids.first().unwrap();
         assert_eq!(session_id.len(), 16);

codex
The reviewed contract is unusually strict but internally specific: evidence is valid only if the varied axis is the control-plane initiator, while cache state, endpoint paths, stream targets, durability accounting, and executable provenance remain identical. I’ve confirmed the Rust trace change only moves attachment after dial completion; I’m now auditing the shell lifecycle and analyzer as the primary risk surface.
exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/bench_otp12pf_rigw.sh | rg -n '"'^[A-Za-z_][A-Za-z0-9_]*'"\\(\\)|"'^readonly |''^[A-Z][A-Z0-9_]*=|standby|purge|ARP|arp|manifest|run_arm|Transfer|transfer|source_init|destination_init|LAUNCHER_SMOKE|SELFTEST|cleanup|CIM|Pid|PID|sentinel|SENTINEL|stage|reparse|hash|sha256|SHA256|scp|ssh|powershell|PowerShell|settle|SESSION-(COMPLETE|VOID)|trap|OFF|ON'"' ; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/otp12pf_rigw_analyze.py | rg -n '"'^def |''^class |''^[A-Z][A-Z0-9_]* =|128|OFF|ON|schedule|Transfer|SOURCE|DESTINATION|settle|durab|manifest|hash|clock|phase|resolution|observer|role_order|pair|trace|epoch|target|live'"'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-6uile3Qj' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ivFk5cml' (errno=Operation not permitted)
4:# Execute this script ON q, from an isolated clean clone of the reviewed
6:# implementations: SOURCE always sends and DESTINATION always receives.
8:# Transfer RPC and therefore which endpoint dials the peer.
10:# Registered diagnostic (128 timed transfers):
11:#   B1 trace OFF, forward cell order, pairs 1..4
12:#   B2 trace ON,  reverse cell order, pairs 1..4
13:#   B3 trace ON,  forward cell order, pairs 5..8
14:#   B4 trace OFF, reverse cell order, pairs 5..8
25:SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
26:REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
28:SELFTEST=${SELFTEST:-0}
29:PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
30:LAUNCHER_SMOKE=${LAUNCHER_SMOKE:-0}
31:EXPECT_SHA=${EXPECT_SHA:-}
35:# every q→Windows control and transfer uses the pinned numeric endpoint.
36:Q_EXPECT_HOST=q.lan
37:Q_NIC=en8
38:Q_IP=10.1.10.54
39:Q_MAC=00:01:d2:19:04:a3
40:WIN_SSH=michael@10.1.10.177
41:WIN_IP=10.1.10.177
42:WIN_NIC=Ethernet
43:WIN_MAC=34-5A-60-3E-78-8B
44:REGISTERED_MTU=9000
45:REGISTERED_MEDIA=10Gbase-T
46:Q_TO_WIN_MSS=8948
47:WIN_TO_Q_MSS=8960
48:PORT=9031
49:PAIRS_PER_BLOCK=4
50:LOAD1_MAX=3.0
51:SPOTLIGHT_CPU_MAX=10.0
52:WIN_CPU_MAX=20.0
53:SETTLE_NS=250000000
54:SETTLE_MIN_MS=250
55:SETTLE_MAX_MS=1000
57:Q_MODULE="$HOME/blit-bench-work"
58:Q_BLIT="$REPO_ROOT/target/release/blit"
59:Q_DAEMON="$REPO_ROOT/target/release/blit-daemon"
60:WIN_ROOT='D:/blit-test'
61:WIN_MODULE="$WIN_ROOT/rigw-module"
62:WIN_BINS="$WIN_ROOT/bins"
63:WIN_ACTIVE="$WIN_BINS/active/blit-daemon.exe"
64:WIN_PURGE_SOURCE="$SCRIPT_DIR/windows/purge-standby.ps1"
66:SESSION_TAG=$(date -u +%Y%m%dT%H%M%SZ).$$
67:OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12pf-rigw-$SESSION_TAG}
68:WIN_SESSION="$WIN_ROOT/rigw-pf1/$SESSION_TAG"
69:WIN_PURGE="$WIN_SESSION/purge-standby.ps1"
70:WIN_PURGE_HASH=""
72:LOG="$OUT_DIR/bench.log"
73:RUNS_CSV="$OUT_DIR/runs.csv"
74:CLOCK_CSV="$OUT_DIR/clock-samples.csv"
76:LAST_ERROR=""
77:OUTPUT_CLAIMED=0
78:OUTPUT_CLAIM_ERROR=""
79:log() {
88:die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
89:append_void_line() {
90:    printf '%s\n' "$1" >> "$OUT_DIR/SESSION-VOID"
92:session_void() {
96:    log "SESSION-VOID: $reason"
100:reserve_evidence_dir() {
104:        if [[ -f "$target/SESSION-COMPLETE" ]]; then
105:            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-COMPLETE: $target"
106:        elif [[ -f "$target/SESSION-VOID" ]]; then
107:            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-VOID: $target"
129:claim_output_dir() {
134:SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto \
136:    -o "ControlPath=$HOME/.ssh/cm-rigw-%r@%h-%p" -o ControlPersist=300)
137:wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
138:wscp() { scp "${SSH_MUX[@]}" "$@"; }
144:CLEANUP_MODE=0
145:CLEANUP_ERROR=""
146:REGISTERED_RUN_STARTED=0
147:SESSION_FINALIZED=0
148:STRICT_CLEANUP_VERIFIED=0
149:Q_SESSION_MAY_EXIST=0
150:WIN_SESSION_MAY_EXIST=0
151:LOCAL_EVIDENCE_COMPLETE=0
153:teardown_die() {
163:reject_registered_overrides() {
173:validate_mode_selection() {
175:    for name in SELFTEST PREFLIGHT_ONLY LAUNCHER_SMOKE; do
184:        || die "SELFTEST, PREFLIGHT_ONLY, and LAUNCHER_SMOKE are mutually exclusive"
187:emit_schedule() {
196:q_source_path() { printf '%s/src_%s' "$Q_MODULE" "$1"; }
197:win_source_path() { printf '%s/src_%s' "$WIN_MODULE" "$1"; }
198:destination_relative_path() {
202:    case "$1" in source_init|destination_init);; *) return 2;; esac
203:    printf 'rigw-sessions/%s/destination/container' "$SESSION_TAG"
205:q_destination_path() {
208:win_destination_path() {
211:arm_destination_path() {
219:arm_destination_argument() {
223:        wm/source_init) printf '%s:%s:/bench/%s/' "$Q_IP" "$PORT" "$relative";;
224:        wm/destination_init) q_destination_path "$role";;
225:        mw/source_init) printf '%s:%s:/bench/%s/' "$WIN_IP" "$PORT" "$relative";;
226:        mw/destination_init) win_destination_path "$role";;
230:append_clock_row() {
233:q_monotonic_ns() {
234:    python3 -c 'import time; print(time.clock_gettime_ns(time.CLOCK_MONOTONIC))'
236:settle_until_deadline() {
241:clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
248:stamp_result_arrival_on_q() {
259:        raise SystemExit("multiple Windows client result sentinels")
262:        raise SystemExit("malformed Windows client result sentinel")
264:    stamp_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
266:    raise SystemExit("missing Windows client result sentinel")
270:successful_windows_log_phase_ok() {
273:fetch_successful_windows_client_log() {
279:embeds_clean_q() {
286:selftest() {
288:    local selftest_client_done selftest_deadline selftest_settle_done run_arm_source
289:    local manifest_tmp canonical_manifest landed_manifest tree_digest
291:    local win_stop_source win_start_source finalize_tmp failure_tmp trap_calls trap_rc
293:    local cleanup_tmp remembered port_checks strict_cleanup_source
298:    local win_recovery_tmp purge_contract_tmp purge_hash drained preflight_source
301:        SELFTEST=1
302:        PREFLIGHT_ONLY=1
303:        LAUNCHER_SMOKE=0
309:        SELFTEST=2
310:        PREFLIGHT_ONLY=0
311:        LAUNCHER_SMOKE=0
326:                1|4) first_role=source_init; source_first=$((source_first + 4));;
327:                2|3) first_role=destination_init; destination_first=$((destination_first + 4));;
340:    local destination_rel="rigw-sessions/$SESSION_TAG/destination/container"
341:    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]] \
343:    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]] \
344:        || die "q DESTINATION-initiated destination path changed"
345:    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]] \
347:    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
348:        || die "Windows DESTINATION-initiated destination path changed"
349:    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]] \
351:    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]] \
353:    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]] \
355:    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]] \
356:        || die "Windows-to-q DESTINATION-initiated destination argument changed"
357:    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]] \
359:    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
360:        || die "q-to-Windows DESTINATION-initiated destination argument changed"
361:    local arp_fixture
362:    arp_fixture=$'? (10.1.10.177) at 34:5a:60:3e:78:8b on en0 ifscope [ethernet]\n? (10.1.10.177) at 34:5a:60:3e:78:8b on en1 ifscope [ethernet]\n? (10.1.10.177) at 34:5a:60:3e:78:8b on en8 ifscope [ethernet]'
363:    [[ "$(q_peer_mac_from_arp en8 <<<"$arp_fixture")" == "34:5a:60:3e:78:8b" ]] \
364:        || die "q ARP parser did not select exactly the registered interface"
365:    purge_contract_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-purge-contract.XXXXXX")
366:    printf '%s\n' '# reviewed purge helper fixture' > "$purge_contract_tmp/purge-standby.ps1"
367:    purge_hash=$(sha256_q "$purge_contract_tmp/purge-standby.ps1")
369:        WIN_PURGE_SOURCE="$purge_contract_tmp/purge-standby.ps1"
370:        WIN_SESSION='D:/blit-test/rigw-pf1/selftest'
371:        WIN_PURGE="$WIN_SESSION/purge-standby.ps1"
373:        wscp() {
377:        wssh() {
380:                [[ "$command" == *"New-Item -ItemType Directory -Path '$WIN_SESSION'"* ]] \
384:            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE.tmp'"* ]] \
386:            [[ "$command" == *"if (\$tmpHash -cne '$purge_hash')"* ]] || return 94
389:            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE'"* ]] \
391:            [[ "$command" == *"if (\$finalHash -cne '$purge_hash')"* ]] || return 97
392:            printf 'H|%s\n' "$purge_hash"
394:        stage_purge_helper
395:        [[ "$WIN_PURGE_HASH" == "$purge_hash" ]] || exit 98
397:        rm -rf "$purge_contract_tmp"
398:        die "reviewed Windows purge helper was not staged and hash-verified"
401:        WIN_PURGE='D:/blit-test/rigw-pf1/selftest/purge-standby.ps1'
402:        WIN_PURGE_HASH="$purge_hash"
405:        wssh() {
407:            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE'"* ]] \
409:            [[ "$command" == *"if (\$purgeHash -cne '$WIN_PURGE_HASH')"* ]] \
411:            [[ "$command" == *"\$purgeOutput = @(& pwsh -NoProfile -File '$WIN_PURGE')"* ]] \
413:            [[ "$command" == *"\$purgeOutput.Count -ne 1"* \
414:                && "$command" == *"[string]\$purgeOutput[0] -cne 'standby-purged'"* ]] \
420:        rm -rf "$purge_contract_tmp"
421:        die "Windows purge helper was not hash-verified per arm with exact success output"
423:    rm -rf "$purge_contract_tmp"
426:        || die "purge-helper staging moved ahead of read-only endpoint gates"
441:    "stage_purge_helper",
442:    "write_manifest",
446:if positions != sorted(positions) or source.count("stage_purge_helper") != 1:
449:    clock_probe=$(append_clock_row 1 run cell 1 source_init before 1 10 11 12 2 0)
453:        || die "registered post-client settle bounds changed"
462:    selftest_settle_done=$(settle_until_deadline "$selftest_deadline")
463:    [[ "$selftest_settle_done" =~ ^[0-9]+$ && "$selftest_settle_done" -ge "$selftest_deadline" ]] \
468:    ) || die "q result-arrival stamper rejected one exact sentinel"
485:    run_arm_source=$(declare -f run_arm)
486:    python3 - "$run_arm_source" <<'PY' || die "run_arm post-client ordering changed"
493:    "read -r result_tag transfer_ms rc client_done_ns result_extra",
494:    'settle_deadline_ns=$((client_done_ns + SETTLE_NS))',
496:    'settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")',
501:    'total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))',
508:        raise SystemExit(f"missing run_arm ordering marker: {marker}") from exc
510:    raise SystemExit(f"run_arm ordering markers out of order: {positions}")
514:    '$SESSION_TAG/$rid/container',
518:        raise SystemExit(f"forbidden run_arm pattern returned: {forbidden}")
527:    "clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)",
568:    raise SystemExit(f"missing pre-PID-file recovery marker: {exc}") from exc
570:    raise SystemExit("empty-PID return can bypass exact Windows process discovery")
582:    r"Stop-Process -Id \$stoppedDaemonPid",
619:    "Move-Item -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp'",
622:    "New-Item -ItemType File -Path '$WIN_SESSION/block_$block/launch.ok'",
627:    raise SystemExit(f"Windows PID journal does not precede launch gate: {controller_positions}")
642:        wssh() {
651:                die "cmd-only Windows recovery fell into the empty-PID port branch"
656:            || die "cmd-only Windows recovery retained remembered PIDs"
670:    "WIN_SESSION_MAY_EXIST=1",
677:    "strict_success_cleanup || session_void",
689:    "SESSION_FINALIZED",
690:    "SESSION-COMPLETE",
693:    "run_arm",
702:    "Q_SESSION_MAY_EXIST",
708:branch_start = main.index('if [[ "$LAUNCHER_SMOKE" == 1 ]]')
713:branch_markers = ('if [[ "$LAUNCHER_SMOKE" == 1 ]]', "launcher_smoke;", "return;", "fi;")
726:        SESSION_TAG=offline-smoke
728:        SESSION_FINALIZED=0
730:        Q_SESSION_MAY_EXIST=0
731:        WIN_SESSION_MAY_EXIST=0
740:                && "$WIN_SESSION_MAY_EXIST" == 1 \
784:        strict_success_cleanup() {
785:            [[ "$WIN_SESSION_MAY_EXIST" == 1 && -z "$current_block" \
788:            printf 'cleanup\n' >> "$launcher_calls"
789:            WIN_SESSION_MAY_EXIST=0
794:            $'closed-pre\nstart\nreach\nstop\nq-stop-empty\ncollect\nclosed-post\ncleanup' ]] \
796:        [[ "$REGISTERED_RUN_STARTED" == 0 && "$SESSION_FINALIZED" == 0 \
798:            && "$Q_SESSION_MAY_EXIST" == 0 \
799:            && "$WIN_SESSION_MAY_EXIST" == 0 \
802:        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" \
803:            && ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
804:            && ! -e "$OUT_DIR/SESSION-VOID" ]] \
820:    manifest_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-manifest.XXXXXX")
821:    mkdir -p "$manifest_tmp/source/sub" "$manifest_tmp/container/src_mixed/sub"
822:    printf 'a' > "$manifest_tmp/source/a"
823:    printf 'bc' > "$manifest_tmp/source/sub/b"
824:    printf 'a' > "$manifest_tmp/container/src_mixed/a"
825:    printf 'bc' > "$manifest_tmp/container/src_mixed/sub/b"
826:    canonical_manifest="$manifest_tmp/canonical.manifest"
827:    landed_manifest="$manifest_tmp/landed.manifest"
828:    write_q_tree_manifest "$manifest_tmp/source" "$canonical_manifest"
829:    write_q_tree_manifest \
830:        "$manifest_tmp/container" "$landed_manifest" src_mixed
831:    tree_digest=$(matching_manifest_digest "$canonical_manifest" "$landed_manifest") \
832:        || die "identical relative-path/size manifests did not match"
834:        || die "tree manifest digest is malformed"
835:    printf 'aa' > "$manifest_tmp/container/src_mixed/a"
836:    printf 'b' > "$manifest_tmp/container/src_mixed/sub/b"
837:    write_q_tree_manifest \
838:        "$manifest_tmp/container" "$landed_manifest" src_mixed
839:    if matching_manifest_digest "$canonical_manifest" "$landed_manifest" >/dev/null; then
840:        rm -rf "$manifest_tmp"
843:    rm -rf "$manifest_tmp/container/src_mixed"
844:    mkdir -p "$manifest_tmp/container/wrapper/src_mixed"
845:    if write_q_tree_manifest \
846:        "$manifest_tmp/container" "$landed_manifest" src_mixed 2>/dev/null; then
847:        rm -rf "$manifest_tmp"
850:    rm -rf "$manifest_tmp"
855:    for marker in SESSION-COMPLETE SESSION-VOID unrelated.txt; do
859:        before=$(sha256_q "$freshness_case/$marker")
864:        [[ "$(sha256_q "$freshness_case/$marker")" == "$before" ]] \
914:        strict_success_cleanup() { return 1; }
916:            die "registered finalization accepted failed strict cleanup"
918:        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] \
919:            || die "failed strict cleanup left SESSION-COMPLETE"
926:        strict_success_cleanup() {
932:        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]]
939:        strict_success_cleanup() {
940:            [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] || return 1
944:            || die "registered finalization rejected verified strict cleanup"
945:        [[ "$SESSION_FINALIZED" == 1 ]] \
946:            || die "registered finalization did not set SESSION_FINALIZED"
947:        [[ "$(< "$OUT_DIR/SESSION-COMPLETE")" == "$HEAD_FULL" ]]
950:    cleanup_tmp="$finalize_tmp/strict"
951:    mkdir -p "$cleanup_tmp/q/rigw-sessions/fail-remote"
952:    printf 'retain me\n' > "$cleanup_tmp/q/rigw-sessions/fail-remote/sentinel"
954:        Q_MODULE="$cleanup_tmp/q"
955:        SESSION_TAG=fail-remote
956:        Q_SESSION_MAY_EXIST=1
957:        WIN_SESSION_MAY_EXIST=1
960:        wssh() { return 1; }
961:        if strict_success_cleanup; then
962:            die "strict cleanup accepted a Windows deletion failure"
965:            || die "Windows cleanup failure was marked strictly verified"
966:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
967:            || die "Windows cleanup failure deleted q evidence first"
968:        [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
969:            || die "Windows cleanup failure modified q evidence"
971:    mkdir -p "$cleanup_tmp/q/rigw-sessions/open-port"
973:        Q_MODULE="$cleanup_tmp/q"
974:        SESSION_TAG=open-port
975:        Q_SESSION_MAY_EXIST=1
976:        WIN_SESSION_MAY_EXIST=1
979:        wssh() { die "strict cleanup reached deletion with an open port"; }
980:        if strict_success_cleanup; then
981:            die "strict cleanup accepted an open port"
984:            || die "open-port cleanup failure was marked strictly verified"
985:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
987:    mkdir -p "$cleanup_tmp/q/rigw-sessions/surviving-q"
989:        Q_MODULE="$cleanup_tmp/q"
990:        SESSION_TAG=surviving-q
991:        Q_SESSION_MAY_EXIST=1
992:        WIN_SESSION_MAY_EXIST=1
995:        wssh() { return 0; }
997:        if strict_success_cleanup; then
998:            die "strict cleanup accepted a surviving q session tree"
1002:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
1004:    mkdir -p "$cleanup_tmp/q/rigw-sessions/succeeds"
1006:        Q_MODULE="$cleanup_tmp/q"
1007:        SESSION_TAG=succeeds
1008:        Q_SESSION_MAY_EXIST=1
1009:        WIN_SESSION_MAY_EXIST=1
1013:        wssh() { return 0; }
1014:        strict_success_cleanup || die "strict cleanup rejected a clean session"
1016:            || die "successful strict cleanup did not set verification state"
1017:        [[ "$port_checks" == 2 ]] || die "strict cleanup ran $port_checks port checks"
1018:        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]] \
1019:            || die "successful strict cleanup retained may-exist state"
1020:        [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
1022:    mkdir -p "$cleanup_tmp/q/rigw-sessions/late-port"
1024:        Q_MODULE="$cleanup_tmp/q"
1025:        SESSION_TAG=late-port
1026:        Q_SESSION_MAY_EXIST=1
1027:        WIN_SESSION_MAY_EXIST=1
1034:        wssh() { return 0; }
1035:        if strict_success_cleanup; then
1036:            die "strict cleanup accepted a listener appearing during deletion"
1042:            Q_MODULE="$cleanup_tmp/q"
1043:            SESSION_TAG="remembered-$remembered"
1051:            ports_closed() { die "strict cleanup ignored remembered $remembered state"; }
1052:            if strict_success_cleanup; then
1053:                die "strict cleanup accepted remembered $remembered state"
1058:    strict_cleanup_source=$(declare -f strict_success_cleanup)
1059:    python3 - "$strict_cleanup_source" <<'PY' \
1060:        || die "strict cleanup source contract changed"
1065:    "'$WIN_MODULE/rigw-sessions/$SESSION_TAG'",
1066:    "'$WIN_SESSION'",
1071:        raise SystemExit(f"missing strict Windows cleanup marker: {marker}")
1073:    raise SystemExit("strict cleanup must check closed ports before and after deletion")
1075:    raise SystemExit("strict cleanup deletes evidence before its first port check")
1077:    raise SystemExit("strict cleanup lacks a post-deletion port check")
1082:    trap_calls="$failure_tmp/remote-calls"
1083:    mkdir -p "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG"
1084:    printf 'retain me\n' > "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel"
1092:        printf 'primary failure\n' > "$OUT_DIR/SESSION-VOID"
1093:        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE"
1094:        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE.tmp"
1096:        SESSION_FINALIZED=0
1098:        Q_SESSION_MAY_EXIST=1
1099:        WIN_SESSION_MAY_EXIST=1
1105:        wssh() {
1106:            printf '%s\n' "$*" >> "$trap_calls"
1112:    trap_rc=$?
1114:    [[ "$trap_rc" == 1 ]] || die "failure trap returned $trap_rc, expected 1"
1115:    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]] \
1116:        || die "failure trap left SESSION-COMPLETE"
1117:    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]] \
1118:        || die "failure trap left SESSION-COMPLETE.tmp"
1119:    grep -Fxq 'primary failure' "$failure_tmp/evidence/SESSION-VOID" \
1120:        || die "failure trap discarded the primary reason"
1121:    grep -Fq 'cleanup errors: Windows PID recovery failed' "$failure_tmp/evidence/SESSION-VOID" \
1122:        || die "failure trap omitted its cleanup error"
1123:    grep -Fq "q session evidence may remain; inspect $failure_tmp/q-module/rigw-sessions/$SESSION_TAG" \
1124:        "$failure_tmp/evidence/SESSION-VOID" \
1125:        || die "failure trap omitted the q evidence path"
1126:    grep -Fq "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG" \
1127:        "$failure_tmp/evidence/SESSION-VOID" \
1128:        || die "failure trap omitted the Windows evidence path"
1129:    if grep -Fq 'Remove-Item' "$trap_calls"; then
1130:        die "failure trap issued destructive remote cleanup"
1132:    [[ "$(< "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
1133:        || die "failure trap modified q session evidence"
1137:        || "$on_exit_source" == *'strict_success_cleanup'* ]]; then
1138:        die "failure trap contains a destructive session-cleanup path"
1143:    printf 'original reason\n' > "$append_tmp/SESSION-VOID"
1151:    trap_rc=$?
1153:    [[ "$trap_rc" == 1 ]] || die "session_void append probe returned $trap_rc"
1154:    [[ "$(< "$append_tmp/SESSION-VOID")" == $'original reason\nlater context' ]] \
1166:        SESSION_FINALIZED=0
1168:        WIN_SESSION_MAY_EXIST=0
1172:    trap_rc=$?
1174:    [[ "$trap_rc" == 1 ]] \
1175:        || die "unfinalized registered zero-exit returned $trap_rc"
1177:        "$contract_tmp/SESSION-VOID" \
1189:        SESSION_FINALIZED=1
1196:    trap_rc=$?
1198:    [[ "$trap_rc" == 1 ]] \
1199:        || die "finalized flags without a completion marker returned $trap_rc"
1201:        "$contract_tmp/SESSION-VOID" \
1206:    printf 'wrong-build\n' > "$contract_tmp/SESSION-COMPLETE"
1214:        SESSION_FINALIZED=1
1221:    trap_rc=$?
1223:    [[ "$trap_rc" == 1 ]] \
1224:        || die "wrong completion marker returned $trap_rc"
1225:    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
1237:        SESSION_FINALIZED=0
1242:    trap_rc=$?
1244:    [[ "$trap_rc" == 1 ]] \
1245:        || die "unclean preflight zero-exit returned $trap_rc"
1246:    grep -Fq 'successful exit lacked verified strict cleanup' \
1247:        "$contract_tmp/SESSION-VOID" \
1252:    printf 'not allowed\n' > "$contract_tmp/SESSION-COMPLETE"
1260:        SESSION_FINALIZED=0
1265:    trap_rc=$?
1267:    [[ "$trap_rc" == 1 ]] \
1268:        || die "preflight completion marker returned $trap_rc"
1269:    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
1272:    for marker in SESSION-VOID SESSION-COMPLETE.tmp; do
1283:            SESSION_FINALIZED=0
1288:        trap_rc=$?
1290:        [[ "$trap_rc" == 1 ]] \
1291:            || die "preflight $marker returned $trap_rc"
1292:        if [[ "$marker" == SESSION-VOID ]]; then
1293:            [[ "$(sed -n '1p' "$contract_tmp/SESSION-VOID")" == 'not allowed' ]] \
1297:                "$contract_tmp/SESSION-VOID" \
1309:OUT_DIR="$2"
1310:LOG="$OUT_DIR/bench.log"
1311:OUTPUT_CLAIMED=1
1312:REGISTERED_RUN_STARTED=1
1313:SESSION_FINALIZED=0
1314:STRICT_CLEANUP_VERIFIED=0
1315:Q_SESSION_MAY_EXIST=1
1316:WIN_SESSION_MAY_EXIST=1
1321:win_daemon_stop() {
1325:q_daemon_stop() {
1329:printf "must disappear\n" > "$OUT_DIR/SESSION-COMPLETE"
1330:trap on_exit EXIT
1331:install_signal_traps
1339:            || die "$signal cleanup returned $signal_rc, expected 1"
1340:        grep -Fxq "received $signal" "$signal_dir/SESSION-VOID" \
1341:            || die "$signal cleanup omitted its signal reason"
1343:            || die "$signal cleanup did not invoke both exact-owned teardown paths"
1344:        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
1345:            || die "$signal cleanup left SESSION-COMPLETE"
1357:    log "SELFTEST OK: exact four-block/128-arm schedule and analyzer guards"
1360:sha256_q() { shasum -a 256 "$1" | awk '{print $1}'; }
1361:sha256_win() {
1362:    wssh "(Get-FileHash -Algorithm SHA256 -LiteralPath '$1').Hash.ToLower()" \
1366:stage_purge_helper() {
1367:    local staged_tmp="$WIN_PURGE.tmp" remote_hash
1369:        || die "reviewed Windows purge helper is absent or not a plain file"
1370:    WIN_PURGE_HASH=$(sha256_q "$WIN_PURGE_SOURCE") \
1371:        || die "cannot hash reviewed Windows purge helper"
1373:        || die "reviewed Windows purge helper hash is malformed: $WIN_PURGE_HASH"
1375:    WIN_SESSION_MAY_EXIST=1
1376:    wssh "
1378:if (Test-Path -LiteralPath '$WIN_SESSION') { throw 'refusing existing Windows session tree' }
1379:New-Item -ItemType Directory -Path '$WIN_SESSION' -ErrorAction Stop | Out-Null
1380:" || die "cannot reserve fresh Windows session tree for reviewed purge helper"
1381:    wscp "$WIN_PURGE_SOURCE" "$WIN_SSH:$staged_tmp" \
1382:        || die "cannot stage reviewed Windows purge helper"
1383:    remote_hash=$(wssh "
1385:\$tmpItem = Get-Item -LiteralPath '$staged_tmp' -Force -ErrorAction Stop
1386:if ((\$tmpItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'staged purge helper is a reparse point' }
1387:\$tmpHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$staged_tmp').Hash.ToLower()
1388:if (\$tmpHash -cne '$WIN_PURGE_HASH') { throw \"staged purge helper hash mismatch: \$tmpHash\" }
1389:Move-Item -LiteralPath '$staged_tmp' -Destination '$WIN_PURGE' -ErrorAction Stop
1391:if ((\$finalItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'final purge helper is a reparse point' }
1392:\$finalHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE').Hash.ToLower()
1393:if (\$finalHash -cne '$WIN_PURGE_HASH') { throw \"final purge helper hash mismatch: \$finalHash\" }
1395:" | tr -d '\r') || die "cannot verify reviewed Windows purge helper"
1396:    [[ "$remote_hash" == "H|$WIN_PURGE_HASH" ]] \
1397:        || die "Windows purge helper verification returned '$remote_hash'"
1400:float_le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }
1402:q_load1() {
1406:q_spotlight_cpu() {
1412:q_time_machine_gate() {
1423:q_quiet_gate() {
1443:win_quiet_gate() {
1445:    out=$(wssh '
1464:q_topology_gate() {
1465:    local raw route arp mtu media status iface route_mtu peer_mac
1485:    arp=$(/usr/sbin/arp -n "$WIN_IP") || die "q ARP probe failed"
1486:    peer_mac=$(q_peer_mac_from_arp "$Q_NIC" <<<"$arp")
1488:        || die "q ARP for $WIN_IP did not yield exactly one $Q_NIC entry: $peer_mac"
1490:        || die "q ARP for $WIN_IP is $peer_mac, expected peer ${WIN_MAC//-/:}"
1491:    [[ "$peer_mac" != "$Q_MAC" ]] || die "q ARP points at q's own MAC (black-hole host route)"
1495:q_peer_mac_from_arp() {
1501:win_topology_gate() {
1503:    out=$(wssh "
1518:q_to_win_mss() {
1527:win_to_q_mss() {
1528:    wssh "
1540:mss_gate() {
1553:firewall_gate() {
1555:    out=$(wssh "
1571:ports_closed() {
1575:    wssh "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
1579:timer_gate() {
1583:clock_ns=lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
1588:    wout=$(wssh '$s=[Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Seconds 1; $s.Stop(); "T|$([int]$s.Elapsed.TotalMilliseconds)"') \
1595:windows_result_stream_gate() {
1598:    result=$(wssh \
1615:fixture_shape_q() {
1626:fixture_shape_win() {
1627:    wssh "
1633:write_q_tree_manifest() {
1641:    raise SystemExit(f"manifest root is not a plain directory: {root}")
1664:            raise SystemExit(f"non-directory/reparse entry in manifest: {path}")
1669:            raise SystemExit(f"non-regular entry in manifest: {path}")
1678:write_win_tree_manifest() {
1680:    wssh "
1690:  if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw \"reparse entry in manifest: \$(\$item.FullName)\" }
1692:  if (-not (\$item -is [IO.FileInfo])) { throw \"non-regular entry in manifest: \$(\$item.FullName)\" }
1707:matching_manifest_digest() {
1710:    sha256_q "$landed"
1713:verify_fixtures() {
1714:    local shape want qgot wgot qmanifest wmanifest qhash
1715:    printf '%s\n' 'shape,sha256,q_manifest,windows_manifest' \
1716:        > "$OUT_DIR/fixture-manifests.csv"
1717:    WIN_SESSION_MAY_EXIST=1
1718:    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION/fixtures' | Out-Null" \
1729:        qmanifest="$OUT_DIR/fixtures/src_$shape.manifest"
1730:        wmanifest="$OUT_DIR/fixtures/windows-src_$shape.manifest"
1731:        write_q_tree_manifest "$(q_source_path "$shape")" "$qmanifest" \
1732:            || die "q src_$shape manifest failed"
1733:        write_win_tree_manifest \
1735:            "$WIN_SESSION/fixtures/src_$shape.manifest" "$wmanifest" \
1736:            || die "Windows src_$shape manifest failed"
1737:        qhash=$(matching_manifest_digest "$qmanifest" "$wmanifest") \
1738:            || die "q and Windows src_$shape relative-path/size manifests differ"
1740:            "$shape" "$qhash" "fixtures/src_$shape.manifest" \
1741:            "fixtures/windows-src_$shape.manifest" \
1742:            >> "$OUT_DIR/fixture-manifests.csv"
1747:write_manifest() {
1749:    qbh=$(sha256_q "$Q_BLIT"); qdh=$(sha256_q "$Q_DAEMON")
1750:    wbh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit.exe")
1751:    wdh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit-daemon.exe")
1752:    cat > "$OUT_DIR/staging-manifest.csv" <<EOF
1753:host,role,commit,sha256,path
1755:q,daemon,$HEAD_FULL,$qdh,$Q_DAEMON
1760:    WIN_DAEMON_HASH=$wdh
1763:provenance_gate() {
1772:    [[ -x "$Q_BLIT" && -x "$Q_DAEMON" ]] || die "q release binaries are absent"
1775:    embeds_clean_q "$Q_DAEMON" \
1777:    wssh "
1787:preflight() {
1792:    command -v scp >/dev/null || die "scp required"
1793:    sudo -n /usr/sbin/purge >/dev/null || die "q NOPASSWD purge grant is absent"
1804:    stage_purge_helper
1805:    write_manifest
1810:q_daemon_stop() {
1816:        [[ "$cmd" == *"$Q_DAEMON"* ]] \
1817:            || { teardown_die "refusing to stop q PID $pid because it is not the launched daemon: $cmd"; return 1; }
1824:            && { teardown_die "q daemon PID $pid survived exact teardown"; return 1; }
1829:win_daemon_stop() {
1832:        if ! pid_probe=$(wssh "
1834:\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
1835:\$d = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/daemon.pid' -ErrorAction SilentlyContinue
1836:\$c = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/launcher.pid' -ErrorAction SilentlyContinue
1854:            teardown_die "Windows PID recovery failed for block $current_block"
1860:        if [[ -n "$current_block" ]] && ! wssh \
1863:            teardown_die "Windows PID files are empty but port $PORT may still be open"
1869:        || { teardown_die "invalid remembered Windows daemon PID '$pid'"; return 1; }
1871:        || { teardown_die "invalid remembered Windows launcher PID '$cmdpid'"; return 1; }
1874:    out=$(wssh "
1878:\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
1893:  if (\$d.Name -ine 'blit-daemon.exe' -or \$actual -ine '$WIN_ACTIVE') { throw \"daemon PID identity mismatch: \$(\$d.Name) \$(\$d.ExecutablePath)\" }
1900:# Every identity is validated before either remembered PID is stopped.
1901:\$stoppedDaemonPid = if (\$d) { [int]\$d.ProcessId } else { \$null }
1902:if (\$d) { Stop-Process -Id \$stoppedDaemonPid -Force }
1916:if (\$stoppedDaemonPid -and (Get-Process -Id \$stoppedDaemonPid -ErrorAction SilentlyContinue)) { throw 'daemon survived teardown' }
1924:fetch_win_file() {
1925:    local remote="$1" local_path="$2" tmp="$local_path.base64" remote_hash local_hash
1926:    wssh "
1936:    remote_hash=$(sha256_win "$remote")
1937:    local_hash=$(sha256_q "$local_path")
1938:    [[ "$remote_hash" == "$local_hash" ]] \
1939:        || session_void "Windows log hash mismatch for $remote"
1942:collect_block_logs() {
1945:    fetch_win_file "$WIN_SESSION/block_$block/daemon.err" "$dir/windows-daemon.err"
1946:    wssh "Remove-Item -LiteralPath '$WIN_SESSION/block_$block' -Recurse -Force -ErrorAction Stop" \
1950:stop_daemons() {
1958:q_daemon_start() {
1972:        BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
1973:            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
1976:        env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID \
1977:            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
1986:win_daemon_start() {
1988:    # The CIM-created batch launcher is allowed to exist before its PID is
1990:    # PID has been atomically placed and read back. Without the gate it times
1992:    out=$(wssh "
1994:New-Item -ItemType Directory -Force -Path '$WIN_SESSION/block_$block','$WIN_BINS/active' | Out-Null
1996:  '$WIN_SESSION/block_$block/launch.ok',
1997:  '$WIN_SESSION/block_$block/launcher.pid',
1998:  '$WIN_SESSION/block_$block/launcher.pid.tmp',
1999:  '$WIN_SESSION/block_$block/daemon.pid'
2005:if ((Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_ACTIVE').Hash.ToLower() -ne '$WIN_DAEMON_HASH') { throw 'active daemon hash mismatch' }
2006:Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.toml' -Value @(
2010:\$trace = if ('$state' -eq 'on') { @('set BLIT_TRACE_SESSION_PHASES=1','set BLIT_TRACE_RUN_ID=$run_id') } else { @('set BLIT_TRACE_SESSION_PHASES=','set BLIT_TRACE_RUN_ID=') }
2011:Set-Content -LiteralPath '$WIN_SESSION/block_$block/start.cmd' -Value @(
2015:  'if exist \"$WIN_SESSION/block_$block/launch.ok\" goto launch_ready',
2022:  '\"$WIN_ACTIVE\" --config \"$WIN_SESSION/block_$block/daemon.toml\" > \"$WIN_SESSION/block_$block/daemon.out\" 2> \"$WIN_SESSION/block_$block/daemon.err\"'
2024:\$launcherCommand = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$block/start.cmd\"\"'
2027:Set-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Value ([string]\$r.ProcessId) -NoNewline
2028:Move-Item -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Destination '$WIN_SESSION/block_$block/launcher.pid' -ErrorAction Stop
2029:\$persistedLauncher = (Get-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid' -Raw -ErrorAction Stop).Trim()
2030:if (\$persistedLauncher -ne [string]\$r.ProcessId) { throw \"launcher PID persistence mismatch: \$persistedLauncher\" }
2031:New-Item -ItemType File -Path '$WIN_SESSION/block_$block/launch.ok' -ErrorAction Stop | Out-Null
2037:if (-not \$d) { Get-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.err' -ErrorAction SilentlyContinue; throw 'daemon child absent' }
2040:Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.pid' -Value \$d.ProcessId
2046:        || session_void "cannot parse Windows daemon PIDs from '$out'"
2049:start_daemons() {
2056:    wssh "if (-not (Test-NetConnection -ComputerName '$Q_IP' -Port $PORT -InformationLevel Quiet)) { exit 8 }" \
2061:record_clock_samples() {
2065:        remote=$(wssh '([DateTime]::UtcNow.Ticks - 621355968000000000) * 100' | tr -cd '0-9')
2075:drain_both() {
2077:    sudo -n /usr/sbin/purge >/dev/null || return 1
2078:    wssh "
2088:\$purgeItem = Get-Item -LiteralPath '$WIN_PURGE' -Force -ErrorAction Stop
2089:if ((\$purgeItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'purge helper is a reparse point' }
2090:\$purgeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE').Hash.ToLower()
2091:if (\$purgeHash -cne '$WIN_PURGE_HASH') { throw \"purge helper hash mismatch: \$purgeHash\" }
2092:\$purgeOutput = @(& pwsh -NoProfile -File '$WIN_PURGE')
2093:\$purgeRc = \$LASTEXITCODE
2094:if (\$purgeRc -ne 0) { throw \"purge helper rc \$purgeRc\" }
2095:if (\$purgeOutput.Count -ne 1 -or [string]\$purgeOutput[0] -cne 'standby-purged') { throw \"purge helper output mismatch: \$(\$purgeOutput -join '|')\" }
2101:prepare_destination() {
2111:        wssh "
2120:if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'destination is a reparse point' }
2126:flush_verify_q() {
2133:        fd=os.open(p,os.O_RDONLY); os.fsync(fd); os.close(fd)
2139:flush_verify_win() {
2140:    wssh "
2149:q_client_run() {
2153:        trace_env=(BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id")
2155:    env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID "${trace_env[@]}" \
2159:clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
2169:win_client_run() {
2172:    out=$(wssh "
2174:if ('$state' -eq 'on') { \$env:BLIT_TRACE_SESSION_PHASES='1'; \$env:BLIT_TRACE_RUN_ID='$run_id' }
2175:else { Remove-Item Env:BLIT_TRACE_SESSION_PHASES,Env:BLIT_TRACE_RUN_ID -ErrorAction SilentlyContinue }
2185:session_id_from_log() {
2198:run_arm() {
2200:    local direction carrier shape flag="" dest dest_arg rid qerr werr client_rel client_abs remote_err result result_tag result_extra transfer_ms rc flush_out flush_ms count bytes want drain session_id total anchor_now_ns
2201:    local windows_client=0 arm_phase=client_done client_done_ns settle_deadline_ns settle_done_ns settled_ms
2202:    local landed_root landed_manifest canonical_manifest remote_manifest tree_manifest_sha256
2209:    remote_err="$WIN_SESSION/block_$block/$rid.client.err"
2222:    if [[ "$direction/$role" == wm/source_init ]]; then
2226:    elif [[ "$direction/$role" == wm/destination_init ]]; then
2230:    elif [[ "$direction/$role" == mw/source_init ]]; then
2234:    elif [[ "$direction/$role" == mw/destination_init ]]; then
2245:    # settle interval.  The first 250 ms is the common excluded observation
2247:    IFS='|' read -r result_tag transfer_ms rc client_done_ns result_extra <<<"$result"
2248:    if [[ "$result_tag" != R || ! "$transfer_ms" =~ ^[0-9]+$ \
2252:        session_void "$rid timer/client sentinel malformed: '$result'"
2255:        # Fetch this client log opportunistically; the failure trap also keeps
2265:        || session_void "$rid client wrapper teardown already exceeded the settle bound"
2266:    settle_deadline_ns=$((client_done_ns + SETTLE_NS))
2269:    settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")
2270:    [[ "$settle_done_ns" =~ ^[0-9]+$ && "$settle_done_ns" -ge "$settle_deadline_ns" ]] \
2271:        || session_void "$rid absolute post-client settle returned early: '$settle_done_ns'"
2272:    settled_ms=$(((settle_done_ns - client_done_ns) / 1000000))
2273:    [[ "$settled_ms" -ge "$SETTLE_MIN_MS" && "$settled_ms" -lt "$SETTLE_MAX_MS" ]] \
2274:        || session_void "$rid post-client settle was ${settled_ms}ms, expected [$SETTLE_MIN_MS,$SETTLE_MAX_MS)"
2277:    # landed-tree probe.  This remains outside transfer_ms.
2279:    landed_manifest="$OUT_DIR/landed/$rid.manifest"
2280:    canonical_manifest="$OUT_DIR/fixtures/src_$shape.manifest"
2283:        write_q_tree_manifest "$dest" "$landed_manifest" "$landed_root" \
2284:            || session_void "$rid q landed root/manifest verification failed"
2287:        remote_manifest="$WIN_SESSION/block_$block/$rid.tree.manifest"
2288:        write_win_tree_manifest \
2289:            "$dest" "$remote_manifest" "$landed_manifest" "$landed_root" \
2290:            || session_void "$rid Windows landed root/manifest verification failed"
2297:    tree_manifest_sha256=$(matching_manifest_digest \
2298:        "$canonical_manifest" "$landed_manifest") \
2299:        || session_void "$rid landed relative-path/size manifest differs from canonical"
2300:    [[ "$tree_manifest_sha256" =~ ^[0-9a-f]{64}$ ]] \
2301:        || session_void "$rid tree manifest digest is malformed"
2307:        wssh "
2330:    total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))
2333:        "$transfer_ms" "$settled_ms" "$flush_ms" "$total" "$landed_root" \
2334:        "$tree_manifest_sha256" "$rc" "$drain" yes "$run_id" "$session_id" \
2336:    log "$rid: transfer=${transfer_ms}ms settled=${settled_ms}ms flush=${flush_ms}ms total=${total}ms session=${session_id:-none}"
2339:cell_order() {
2348:run_block() {
2349:    local block="$1" state="$2" pass="$3" first="$4" last="$5" run_id="${SESSION_TAG}-b${block}-${state}"
2358:            1|4) first_role=source_init; second_role=destination_init;;
2359:            2|3) first_role=destination_init; second_role=source_init;;
2365:            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$first_role" 1
2366:            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$second_role" 2
2375:end_gate() {
2384:strict_success_cleanup() {
2387:        || { LAST_ERROR="strict cleanup found remembered q daemon PID $q_daemon_pid"; return 1; }
2389:        || { LAST_ERROR="strict cleanup found remembered Windows daemon PID $win_daemon_pid"; return 1; }
2391:        || { LAST_ERROR="strict cleanup found remembered Windows launcher PID $win_cmd_pid"; return 1; }
2393:        || { LAST_ERROR="strict cleanup found current block $current_block"; return 1; }
2396:        || { LAST_ERROR="strict cleanup found port $PORT still listening"; return 1; }
2397:    wssh "
2399:\$paths = @('$WIN_MODULE/rigw-sessions/$SESSION_TAG', '$WIN_SESSION')
2404:  if (Test-Path -LiteralPath \$path) { throw \"strict cleanup left \$path\" }
2407:        || { LAST_ERROR="strict cleanup could not remove and verify Windows session trees"; return 1; }
2408:    WIN_SESSION_MAY_EXIST=0
2409:    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
2410:        rm -rf -- "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
2411:            || { LAST_ERROR="strict cleanup could not remove q session tree"; return 1; }
2413:    [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
2414:        && ! -L "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
2415:        || { LAST_ERROR="strict cleanup found a surviving or unexpected q session tree"; return 1; }
2416:    Q_SESSION_MAY_EXIST=0
2418:        || { LAST_ERROR="strict cleanup found port $PORT reopened during deletion"; return 1; }
2422:launcher_smoke() {
2423:    local run_id="${SESSION_TAG}-launcher-smoke"
2424:    WIN_SESSION_MAY_EXIST=1
2433:    strict_success_cleanup \
2434:        || session_void "launcher smoke cleanup failed: ${LAST_ERROR:-unknown error}"
2435:    log "LAUNCHER_SMOKE OK: exact Windows CIM launcher started, reached, identity-stopped, and cleaned; no transfer timed"
2438:finalize_registered_session() {
2439:    local complete_tmp="$OUT_DIR/SESSION-COMPLETE.tmp"
2440:    SESSION_FINALIZED=0
2442:        || { LAST_ERROR="refusing cleanup before local evidence is complete"; return 1; }
2443:    strict_success_cleanup || return 1
2445:        || { LAST_ERROR="strict cleanup returned without verification"; return 1; }
2446:    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]] \
2448:    [[ ! -e "$OUT_DIR/SESSION-COMPLETE" && ! -L "$OUT_DIR/SESSION-COMPLETE" ]] \
2453:    mv "$complete_tmp" "$OUT_DIR/SESSION-COMPLETE" || return 1
2454:    SESSION_FINALIZED=1
2457:record_failure_evidence() {
2459:    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
2460:        append_void_line "q session evidence may remain; inspect $Q_MODULE/rigw-sessions/$SESSION_TAG"
2462:    if [[ "$WIN_SESSION_MAY_EXIST" == 1 ]]; then
2463:        append_void_line "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG"
2467:on_signal() {
2470:    trap '' HUP INT TERM
2474:install_signal_traps() {
2475:    trap 'on_signal HUP 129' HUP
2476:    trap 'on_signal INT 130' INT
2477:    trap 'on_signal TERM 143' TERM
2480:registered_completion_marker_valid() {
2481:    local marker="$OUT_DIR/SESSION-COMPLETE" lines
2487:    [[ ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
2488:        && ! -L "$OUT_DIR/SESSION-COMPLETE.tmp" ]] || return 1
2489:    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]]
2492:on_exit() {
2494:    trap - EXIT
2495:    trap '' HUP INT TERM
2498:        && ( -e "$OUT_DIR/SESSION-VOID" || -L "$OUT_DIR/SESSION-VOID" \
2499:            || -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
2500:            || -L "$OUT_DIR/SESSION-COMPLETE.tmp" ) ]]; then
2505:        && "$SESSION_FINALIZED" != 1 ]]; then
2515:        && "$SESSION_FINALIZED" != 0 ]]; then
2520:        && ( -e "$OUT_DIR/SESSION-COMPLETE" \
2521:            || -L "$OUT_DIR/SESSION-COMPLETE" ) ]]; then
2527:        LAST_ERROR="successful exit lacked verified strict cleanup"
2532:        rm -f -- "$OUT_DIR/SESSION-COMPLETE" "$OUT_DIR/SESSION-COMPLETE.tmp" \
2534:        if [[ ! -s "$OUT_DIR/SESSION-VOID" ]]; then
2543:            append_void_line "cleanup errors: $CLEANUP_ERROR"
2551:main() {
2553:    if [[ "$SELFTEST" == 1 ]]; then selftest; return; fi
2558:    trap on_exit EXIT
2559:    install_signal_traps
2561:    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
2562:        strict_success_cleanup || session_void "preflight cleanup failed: ${LAST_ERROR:-unknown error}"
2563:        log "PREFLIGHT_ONLY: no daemon started and no transfer timed"
2566:    if [[ "$LAUNCHER_SMOKE" == 1 ]]; then
2572:    Q_SESSION_MAY_EXIST=1
2573:    mkdir -p "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
2575:    printf '%s\n' 'block,trace_state,pass,cell,role,pair,role_order,transfer_ms,settled_ms,flush_ms,total_ms,landed_root,tree_manifest_sha256,exit,drain,valid,run_id,session_id,client_log' > "$RUNS_CSV"
2578:    WIN_SESSION_MAY_EXIST=1
2579:    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION' | Out-Null" \
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HrYlhVhV' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-8LEonkIO' (errno=Operation not permitted)
5:four-block schedule and writes reports only after the CSV and every structured
6:TCP trace have passed validation.  Phase intervals are derived exclusively
7:from one endpoint's ``elapsed_ns`` clock; ``unix_ns`` is retained as evidence
17:import hashlib
30:CELLS = (
36:TCP_CELLS = frozenset(cell for cell in CELLS if "_tcp_" in cell)
37:TARGET_CELL = "wm_tcp_mixed"
38:ROLES = ("source_init", "destination_init")
39:CSV_FIELDS = (
41:    "trace_state",
45:    "pair",
46:    "role_order",
48:    "settled_ms",
52:    "tree_manifest_sha256",
60:CLOCK_FIELDS = (
64:    "pair",
66:    "phase",
74:TRACE_PREFIX = "[session-phase] "
75:SESSION_ID_RE = re.compile(r"^[0-9a-f]{16}$")
76:SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
77:SETTLE_MIN_MS = 250
78:SETTLE_MAX_MS = 1000
79:MEASURAND = "durable_total_ms"
83:class BlockSpec:
85:    trace_state: str
87:    pairs: range
91:BLOCKS = (
99:class AnalysisError(RuntimeError):
100:    """The evidence is incomplete, contaminated, or off schedule."""
104:class RunRow:
106:    schedule_index: int
108:    trace_state: str
112:    pair: int
113:    role_order: int
115:    settled_ms: int
119:    tree_manifest_sha256: str
129:class TraceEvent:
160:class ClockSample:
163:    phase: str
173:class ModeDescription:
187:class ConditionStats:
189:    trace_state: str
192:    paired_deltas: tuple[Decimal, ...]
196:    paired_delta_median: Decimal
205:    role_order_drift: Decimal
206:    paired_delta_range: Decimal
207:    n_pair_split: Decimal
208:    n_pair: Decimal
212:class AnalysisResult:
216:    phase_events_csv: Path
217:    phase_intervals_csv: Path
218:    clock_summary_csv: Path
219:    observer_bias: Decimal
220:    n_resolution: Decimal
221:    trace_event_count: int
224:def decimal_text(value: Decimal) -> str:
230:def parse_decimal(value: str, field: str, line: int) -> Decimal:
242:def parse_int(value: str, field: str, line: int, source: str = "runs.csv") -> int:
251:def expected_roles(pair: int) -> tuple[str, str]:
253:    round_index = (pair - 1) % 4
257:def expected_schedule() -> list[tuple[BlockSpec, str, int, str, int]]:
260:        for round_index, pair in enumerate(block.pairs):
263:                for role_order, role in enumerate(
264:                    expected_roles(pair), start=1
266:                    expected.append((block, cell, pair, role, role_order))
270:def _safe_client_log(root: Path, value: str, line: int) -> None:
284:def _read_tree_manifest(path: Path, label: str) -> tuple[bytes, str]:
289:        raise AnalysisError(f"{label}: manifest must be non-empty and newline-terminated")
293:        raise AnalysisError(f"{label}: manifest is not ASCII") from exc
296:        raise AnalysisError(f"{label}: manifest lines are not exact sorted unique inventory")
318:    return data, hashlib.sha256(data).hexdigest()
321:def _load_fixture_manifests(root: Path) -> dict[str, tuple[bytes, str]]:
322:    index_path = root / "fixture-manifests.csv"
327:        fields = ("shape", "sha256", "q_manifest", "windows_manifest")
329:            raise AnalysisError("fixture-manifests.csv header mismatch")
332:        raise AnalysisError("fixture-manifests.csv must contain mixed,large exactly")
335:        f"src_{shape}.manifest" for shape in ("mixed", "large")
336:    } | {f"windows-src_{shape}.manifest" for shape in ("mixed", "large")}
348:            "fixture manifest file inventory mismatch: expected "
355:        q_relative = f"fixtures/src_{shape}.manifest"
356:        win_relative = f"fixtures/windows-src_{shape}.manifest"
357:        if row["q_manifest"] != q_relative or row["windows_manifest"] != win_relative:
358:            raise AnalysisError(f"fixture-manifests.csv {shape}: path mapping mismatch")
359:        q_data, q_digest = _read_tree_manifest(root / q_relative, f"q src_{shape}")
360:        win_data, win_digest = _read_tree_manifest(
364:            raise AnalysisError(f"canonical q/Windows src_{shape} manifests differ")
366:            raise AnalysisError(f"fixture-manifests.csv {shape}: digest mismatch")
371:def load_runs(root: Path) -> list[RunRow]:
372:    fixture_manifests = _load_fixture_manifests(root)
387:    schedule = expected_schedule()
388:    if len(raw_rows) != len(schedule):
390:            f"runs.csv schedule incomplete: expected {len(schedule)} rows, got {len(raw_rows)}"
394:    for index, (raw, expected) in enumerate(zip(raw_rows, schedule), start=0):
396:        block_spec, cell, pair, role, role_order = expected
397:        actual_schedule = (
399:            raw["trace_state"],
402:            parse_int(raw["pair"], "pair", line),
404:            parse_int(raw["role_order"], "role_order", line),
406:        wanted_schedule = (
408:            block_spec.trace_state,
411:            pair,
413:            role_order,
415:        if actual_schedule != wanted_schedule:
417:                f"runs.csv line {line}: schedule mismatch; expected {wanted_schedule}, "
418:                f"got {actual_schedule}"
423:                f"runs.csv line {line}: SESSION-VOID arm "
430:        traced_tcp = block_spec.trace_state == "on" and cell in TCP_CELLS
431:        if traced_tcp:
432:            if not SESSION_ID_RE.fullmatch(session_id):
434:                    f"runs.csv line {line}: trace-on TCP session_id must be 16 lowercase hex"
438:                f"runs.csv line {line}: session_id must be blank for trace-off or gRPC arms"
441:        settled_ms = parse_int(raw["settled_ms"], "settled_ms", line)
442:        if not SETTLE_MIN_MS <= settled_ms < SETTLE_MAX_MS:
444:                f"runs.csv line {line}: settled_ms must be in "
445:                f"[{SETTLE_MIN_MS},{SETTLE_MAX_MS}), got {settled_ms}"
450:        settle_excess_ms = Decimal(settled_ms - SETTLE_MIN_MS)
451:        expected_total_ms = transfer_ms + settle_excess_ms + flush_ms
455:                f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms "
457:                f"{decimal_text(transfer_ms)} + ({settled_ms} - "
467:        recorded_digest = raw["tree_manifest_sha256"]
470:                f"runs.csv line {line}: tree_manifest_sha256 must be 64 lowercase hex"
472:        rid = f"b{block_spec.number}_{cell}_p{pair}_{role}"
473:        landed_data, landed_digest = _read_tree_manifest(
474:            root / "landed" / f"{rid}.manifest", f"landed manifest {rid}"
476:        canonical_data, canonical_digest = fixture_manifests[shape]
478:            raise AnalysisError(f"runs.csv line {line}: landed manifest digest mismatch")
481:                f"runs.csv line {line}: landed relative-path/size manifest "
487:                schedule_index=index,
489:                trace_state=block_spec.trace_state,
493:                pair=pair,
494:                role_order=role_order,
496:                settled_ms=settled_ms,
500:                tree_manifest_sha256=recorded_digest,
512:        f"b{row.block}_{row.cell}_p{row.pair}_{row.role}.manifest"
526:            "landed manifest file inventory mismatch: expected exactly 128 registered "
543:        if row.trace_state == "on" and row.cell in TCP_CELLS
546:        raise AnalysisError("trace-on TCP (run_id, session_id) values must be unique")
550:def load_clock_samples(root: Path, rows: Sequence[RunRow]) -> list[ClockSample]:
551:    path = root / "clock-samples.csv"
558:                "clock-samples.csv header mismatch: expected "
566:        (run, phase, sample)
568:        for phase in ("before", "after")
573:            "clock-samples.csv inventory incomplete: expected "
578:    for index, (raw, (run, phase, sample)) in enumerate(zip(raw_samples, expected)):
581:            parse_int(raw["block"], "block", line, "clock-samples.csv"),
584:            parse_int(raw["pair"], "pair", line, "clock-samples.csv"),
586:            raw["phase"],
587:            parse_int(raw["sample"], "sample", line, "clock-samples.csv"),
593:            run.pair,
595:            phase,
600:                f"clock-samples.csv line {line}: schedule mismatch; expected "
604:            raw["q_before_ns"], "q_before_ns", line, "clock-samples.csv"
607:            raw["windows_ns"], "windows_ns", line, "clock-samples.csv"
610:            raw["q_after_ns"], "q_after_ns", line, "clock-samples.csv"
612:        rtt = parse_int(raw["rtt_ns"], "rtt_ns", line, "clock-samples.csv")
617:            "clock-samples.csv",
621:                f"clock-samples.csv line {line}: q/windows times must be positive and "
628:                f"clock-samples.csv line {line}: rtt_ns mismatch; expected "
633:                f"clock-samples.csv line {line}: offset mismatch; expected "
640:                phase=phase,
652:def _require_json_string(raw: dict[str, Any], name: str, where: str) -> None:
657:def _require_json_int(raw: dict[str, Any], name: str, where: str) -> None:
663:def load_trace_events(root: Path) -> list[TraceEvent]:
664:    evidence_roots = (root / "trace", root / "client")
667:            raise AnalysisError(f"missing trace evidence directory: {evidence_root}")
690:                    except json.JSONDecodeError as exc:
691:                        raise AnalysisError(f"{where}: malformed session-phase JSON: {exc}") from exc
693:                        raise AnalysisError(f"{where}: session-phase payload is not an object")
695:                        raise AnalysisError(f"{where}: unsupported session-phase schema")
706:                    if not SESSION_ID_RE.fullmatch(raw["session_id"]):
708:                    if raw["endpoint_role"] not in ("SOURCE", "DESTINATION"):
710:                    if raw["initiator_role"] not in ("SOURCE", "DESTINATION"):
713:                        "epoch",
717:                        "target_streams",
718:                        "live_streams",
726:            raise AnalysisError(f"{relative}: trace log is not UTF-8") from exc
730:def _one_event(events: Sequence[TraceEvent], role: str, name: str, label: str) -> TraceEvent:
737:def _correlation_keys(
743:        epoch = event.raw.get("epoch")
745:        if not isinstance(epoch, int) or not isinstance(socket, int):
746:            raise AnalysisError(f"{label}: {role}/{name} lacks epoch/socket correlation")
747:        keys.append((epoch, socket))
749:        raise AnalysisError(f"{label}: duplicate {role}/{name} epoch/socket marker")
753:def _marker_map(
780:def _assert_same_keys(
794:def _assert_before(label: str, start: TraceEvent, end: TraceEvent) -> None:
806:def _assert_event_fields(
809:    epoch = event.raw.get("epoch")
814:                f"{label}: {event.endpoint_role}/{event.event} epoch {epoch} "
819:def validate_traces(
828:        if row.trace_state == "on" and row.cell in TCP_CELLS
836:                state = next(row.trace_state for row in rows if row.block == block)
839:                        f"trace leak: trace-off block {block} emitted {event.session_id} "
843:                    f"trace leak: block {block} emitted an unregistered (including possible "
848:                f"stale/foreign trace run_id {event.run_id!r} at "
858:            f"missing trace for block {row.block} {row.cell} pair {row.pair} "
865:            f"block {row.block} {row.cell} pair {row.pair} {row.role} "
868:        expected_initiator = "SOURCE" if row.role == "source_init" else "DESTINATION"
870:        if roles != {"SOURCE", "DESTINATION"}:
872:                f"{label}: missing endpoint role; expected SOURCE+DESTINATION, got {sorted(roles)}"
875:            raise AnalysisError(f"{label}: initiator_role does not match scheduled role")
888:        manifest_begin = _one_event(
889:            group, "SOURCE", "manifest_complete_send_begin", label
891:        manifest_sent = _one_event(group, "SOURCE", "manifest_complete_sent", label)
892:        _one_event(group, "DESTINATION", "manifest_complete_received", label)
893:        first_queued = _one_event(group, "SOURCE", "first_payload_queued", label)
894:        _assert_before(label, manifest_begin, manifest_sent)
895:        _assert_before(label, manifest_sent, first_queued)
898:            group, "DESTINATION", "need_batch_send_begin", ("batch",), label
901:            group, "DESTINATION", "need_batch_sent", ("batch",), label
904:            group, "SOURCE", "need_batch_received", ("batch",), label
930:            group, "SOURCE", "planner_begin", ("batch",), label
932:        planner_end = _marker_map(group, "SOURCE", "planner_end", ("batch",), label)
946:            ("resize_proposed", _marker_map(group, "SOURCE", "resize_proposed", ("epoch",), label)),
949:                _marker_map(group, "SOURCE", "resize_send_begin", ("epoch",), label),
951:            ("resize_sent", _marker_map(group, "SOURCE", "resize_sent", ("epoch",), label)),
954:                _marker_map(group, "DESTINATION", "resize_received", ("epoch",), label),
958:                _marker_map(group, "DESTINATION", "destination_prepared", ("epoch",), label),
963:                    group, "DESTINATION", "resize_ack_send_begin", ("epoch",), label
968:                _marker_map(group, "DESTINATION", "resize_ack_sent", ("epoch",), label),
972:                _marker_map(group, "SOURCE", "resize_ack_received", ("epoch",), label),
975:                "source_settled",
976:                _marker_map(group, "SOURCE", "source_settled", ("epoch",), label),
979:        resize_epochs = _assert_same_keys(label, resize_maps)
980:        expected_resize_epochs = {(epoch,) for epoch in range(1, 8)}
981:        if resize_epochs != expected_resize_epochs:
983:                f"{label}: resize epochs must be exactly 1..7, got "
984:                f"{sorted(epoch[0] for epoch in resize_epochs)}"
988:            "arm_queued" if expected_initiator == "SOURCE" else "dial_complete"
990:        for key in sorted(resize_epochs):
991:            epoch = key[0]
992:            target = epoch + 1
1011:                label, resize["resize_ack_received"][key], resize["source_settled"][key]
1017:                    {"target_streams": target, "live_streams": epoch},
1022:                {"target_streams": target, "live_streams": epoch},
1027:                {"target_streams": target},
1033:                    {"accepted": True, "live_streams": target},
1038:                {"accepted": True, "live_streams": target},
1042:                resize["source_settled"][key],
1045:                    "target_streams": target,
1046:                    "live_streams": target,
1051:                    f"{label}: resize epoch {key[0]} destination_prepared action must be "
1054:        for epoch in range(1, 7):
1057:                resize["source_settled"][(epoch,)],
1058:                resize["resize_proposed"][(epoch + 1,)],
1062:                resize["resize_ack_sent"][(epoch,)],
1063:                resize["resize_received"][(epoch + 1,)],
1066:        source_complete = _one_event(group, "SOURCE", "data_plane_complete", label)
1067:        source_summary = _one_event(group, "SOURCE", "summary_received", label)
1068:        destination_complete = _one_event(group, "DESTINATION", "data_plane_complete", label)
1070:            group, "DESTINATION", "summary_send_begin", label
1072:        destination_summary = _one_event(group, "DESTINATION", "summary_sent", label)
1074:            raise AnalysisError(f"{label}: SOURCE terminal inventory is out of sequence")
1080:            raise AnalysisError(f"{label}: DESTINATION terminal inventory is out of sequence")
1081:        _assert_before(label, resize["source_settled"][(7,)], source_complete)
1086:            "SOURCE",
1087:            "socket_trace_attached",
1088:            ("epoch", "socket"),
1093:            "DESTINATION",
1094:            "socket_trace_attached",
1095:            ("epoch", "socket"),
1102:        expected_attached = {(0, 0)} | {(epoch[0], 0) for epoch in resize_epochs}
1106:                f"match epoch-0 plus accepted resize epochs {sorted(expected_attached)}"
1109:            ("SOURCE", source_complete),
1110:            ("DESTINATION", destination_complete),
1116:                and event.event == "socket_trace_attached"
1120:        source_action = "dial" if expected_initiator == "SOURCE" else "accept"
1121:        destination_action = "accept" if expected_initiator == "SOURCE" else "dial"
1126:            ("SOURCE", source_action),
1127:            ("DESTINATION", destination_action),
1133:                ("epoch", "socket"),
1140:                ("epoch", "socket"),
1154:                ("epoch", "socket"),
1163:                if endpoint_role == "SOURCE"
1171:        source_action_begins, source_action_ends = action_events["SOURCE"]
1173:            "DESTINATION"
1175:        for (epoch,) in sorted(resize_epochs):
1176:            action_key = (epoch, 0)
1179:                resize["resize_sent"][(epoch,)],
1184:                resize["resize_ack_received"][(epoch,)],
1190:                resize["source_settled"][(epoch,)],
1195:                resize["source_settled"][(epoch,)],
1200:            "DESTINATION",
1202:            ("epoch",),
1208:            "DESTINATION",
1210:            ("epoch",),
1214:        if expected_initiator == "SOURCE":
1215:            arm_epochs = _assert_same_keys(
1218:            if arm_epochs != resize_epochs:
1220:            for arm_key in arm_epochs:
1224:                    {"target_streams": arm_key[0] + 1},
1250:            for (epoch,) in sorted(resize_epochs):
1253:                    resize["resize_received"][(epoch,)],
1254:                    destination_action_begins[(epoch, 0)],
1258:                    destination_action_ends[(epoch, 0)],
1259:                    resize["destination_prepared"][(epoch,)],
1263:                    destination_attachment_events[(epoch, 0)],
1264:                    resize["destination_prepared"][(epoch,)],
1268:            "SOURCE",
1270:            ("epoch", "socket"),
1275:            "SOURCE",
1277:            ("epoch", "socket"),
1282:            "DESTINATION",
1284:            ("epoch", "socket"),
1293:            raise AnalysisError(f"{label}: SOURCE payload socket was not trace-attached")
1295:            raise AnalysisError(f"{label}: DESTINATION payload socket was not trace-attached")
1311:def condition_stats(rows: Sequence[RunRow], cell: str, trace_state: str) -> ConditionStats:
1313:        row for row in rows if row.cell == cell and row.trace_state == trace_state
1315:    by_pair: dict[int, dict[str, Decimal]] = {}
1317:        if row.role in by_pair.setdefault(row.pair, {}):
1319:                f"duplicate timing for {cell}/{trace_state}/pair {row.pair}/{row.role}"
1321:        by_pair[row.pair][row.role] = row.total_ms
1322:    if sorted(by_pair) != list(range(1, 9)):
1324:            f"{cell}/{trace_state}: expected paired observations 1..8, got {sorted(by_pair)}"
1326:    for pair, arms in by_pair.items():
1328:            raise AnalysisError(f"{cell}/{trace_state}/pair {pair}: incomplete role pair")
1329:    source = tuple(by_pair[pair]["source_init"] for pair in range(1, 9))
1330:    destination = tuple(by_pair[pair]["destination_init"] for pair in range(1, 9))
1332:    source_first_pairs = {
1333:        row.pair for row in selected if row.role == "source_init" and row.role_order == 1
1335:    destination_first_pairs = {
1336:        row.pair
1338:        if row.role == "destination_init" and row.role_order == 1
1340:    if source_first_pairs | destination_first_pairs != set(range(1, 9)):
1341:        raise AnalysisError(f"{cell}/{trace_state}: incomplete role-order partition")
1342:    if source_first_pairs & destination_first_pairs:
1343:        raise AnalysisError(f"{cell}/{trace_state}: overlapping role-order partition")
1345:        tuple(deltas[pair - 1] for pair in sorted(source_first_pairs))
1348:        tuple(deltas[pair - 1] for pair in sorted(destination_first_pairs))
1358:        trace_state=trace_state,
1361:        paired_deltas=deltas,
1365:        paired_delta_median=median(deltas),
1374:        role_order_drift=abs(source_first - destination_first),
1375:        paired_delta_range=max(deltas) - min(deltas),
1376:        n_pair_split=max(abs(first4 - last4), abs(odd - even)),
1377:        n_pair=max(
1386:def largest_gap_modes(values: Iterable[Decimal]) -> ModeDescription:
1398:def _atomic_csv(path: Path, fields: Sequence[str], rows: Iterable[dict[str, Any]]) -> None:
1409:def _atomic_text(path: Path, contents: str) -> None:
1418:def _summary_rows(
1419:    stats: Sequence[ConditionStats], observer_bias: Decimal, n_resolution: Decimal
1425:        delta_modes = largest_gap_modes(item.paired_deltas)
1426:        target = item.cell == TARGET_CELL
1430:                "trace_state": item.trace_state,
1432:                "pairs": "8",
1436:                "paired_delta_median_ms": decimal_text(item.paired_delta_median),
1449:                "role_order_drift_ms": decimal_text(item.role_order_drift),
1450:                "paired_delta_range_ms": decimal_text(item.paired_delta_range),
1451:                "n_pair_split_ms": decimal_text(item.n_pair_split),
1452:                "n_pair_ms": decimal_text(item.n_pair),
1453:                "observer_bias_ms": decimal_text(observer_bias) if target else "",
1454:                "n_resolution_ms": decimal_text(n_resolution) if target else "",
1461:                "paired_delta_sorted_ms": ";".join(
1462:                    decimal_text(value) for value in sorted(item.paired_deltas)
1468:                "paired_delta_largest_gap_ms": decimal_text(delta_modes.gap),
1469:                "paired_delta_descriptive_modes_ms": delta_modes.render(),
1475:SUMMARY_FIELDS = (
1477:    "trace_state",
1479:    "pairs",
1483:    "paired_delta_median_ms",
1492:    "role_order_drift_ms",
1493:    "paired_delta_range_ms",
1494:    "n_pair_split_ms",
1495:    "n_pair_ms",
1496:    "observer_bias_ms",
1497:    "n_resolution_ms",
1500:    "paired_delta_sorted_ms",
1505:    "paired_delta_largest_gap_ms",
1506:    "paired_delta_descriptive_modes_ms",
1510:def _distribution_rows(stats: Sequence[ConditionStats]) -> list[dict[str, str]]:
1516:            ("paired_total_delta", item.paired_deltas),
1526:                        "trace_state": item.trace_state,
1539:CLOCK_SUMMARY_FIELDS = (
1544:    "pair",
1546:    "role_order",
1558:def _clock_summary_rows(samples: Sequence[ClockSample]) -> list[dict[str, str]]:
1561:        grouped.setdefault(sample.run.schedule_index, {}).setdefault(sample.phase, []).append(sample)
1563:    for schedule_index in sorted(grouped):
1564:        phases = grouped[schedule_index]
1565:        if set(phases) != {"before", "after"}:
1566:            raise AnalysisError(f"clock samples for schedule row {schedule_index} lack a phase")
1567:        before = min(phases["before"], key=lambda item: (item.rtt_ns, item.sample))
1568:        after = min(phases["after"], key=lambda item: (item.rtt_ns, item.sample))
1576:                "pair": str(run.pair),
1578:                "role_order": str(run.role_order),
1592:EVENT_FIELDS = (
1594:    "trace_state",
1597:    "pair",
1599:    "role_order",
1601:    "settled_ms",
1613:    "epoch",
1617:    "target_streams",
1618:    "live_streams",
1625:def _event_row(row: RunRow, event: TraceEvent) -> dict[str, str]:
1629:        "trace_state": row.trace_state,
1632:        "pair": str(row.pair),
1634:        "role_order": str(row.role_order),
1636:        "settled_ms": str(row.settled_ms),
1652:        "epoch",
1656:        "target_streams",
1657:        "live_streams",
1668:INTERVAL_FIELDS = (
1670:    "trace_state",
1673:    "pair",
1692:SPAN_SPECS = (
1693:    ("socket_dial_begin", "socket_dial_end", ("epoch", "socket")),
1694:    ("socket_accept_begin", "socket_accept_end", ("epoch", "socket")),
1695:    ("manifest_complete_send_begin", "manifest_complete_sent", ()),
1698:    ("resize_send_begin", "resize_sent", ("epoch",)),
1699:    ("resize_ack_send_begin", "resize_ack_sent", ("epoch",)),
1700:    ("resize_arm_queue_begin", "resize_arm_ready", ("epoch",)),
1701:    ("socket_write_begin", "first_socket_write", ("epoch", "socket")),
1708:def _interval_base(row: RunRow, endpoint_role: str, initiator_role: str) -> dict[str, str]:
1711:        "trace_state": row.trace_state,
1714:        "pair": str(row.pair),
1723:def _make_interval(
1759:def _phase_rows(
1764:    traced_rows = [
1765:        row for row in rows if row.trace_state == "on" and row.cell in TCP_CELLS
1767:    for row in traced_rows:
1769:        for endpoint_role in ("SOURCE", "DESTINATION"):
1817:def _markdown(
1819:    observer_bias: Decimal,
1820:    n_resolution: Decimal,
1821:    trace_event_count: int,
1823:    clock_arm_count: int,
1825:    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
1827:        "# otp-12 pf-1 rig-W phase report",
1829:        "Validation: PASS — exact four-block OFF–ON–ON–OFF schedule, forward/reverse "
1830:        "cell and role ordering, 8 valid role pairs per trace state/cell, trace-off and "
1831:        "gRPC trace absence, and correlated two-role TCP terminal traces.",
1835:        "| cell | trace | source total median ms | destination total median ms | Δ total ms | paired total d median ms | N_pair_split total ms | role-order drift total ms | paired range total ms | N_pair total ms |",
1844:                    item.trace_state,
1848:                    decimal_text(item.paired_delta_median),
1849:                    decimal_text(item.n_pair_split),
1850:                    decimal_text(item.role_order_drift),
1851:                    decimal_text(item.paired_delta_range),
1852:                    decimal_text(item.n_pair),
1861:            f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms`: client execution plus every "
1863:            "the destination durability "
1866:            "distributions, observer bias, and resolution floors.",
1869:            "Each paired `d_i = destination_init total_ms_i − source_init total_ms_i`. "
1870:            "`N_pair_split = max(|median(d_1..d_4) − median(d_5..d_8)|, "
1874:            "schedule means this is not the odd/even partition. The conservative "
1875:            "operative `N_pair = max(N_pair_split, role-order drift, max(d) − min(d))`, "
1878:            f"For target `{TARGET_CELL}`: Δ_off={decimal_text(target['off'].delta)} ms, "
1879:            f"Δ_on={decimal_text(target['on'].delta)} ms, observer_bias="
1880:            f"|Δ_on−Δ_off|={decimal_text(observer_bias)} ms, N_pair_off="
1881:            f"{decimal_text(target['off'].n_pair)} ms, N_pair_on="
1882:            f"{decimal_text(target['on'].n_pair)} ms, and N_resolution="
1883:            f"{decimal_text(n_resolution)} ms.",
1885:            "This run measures the observer and paired resolution floors; it does not "
1892:            "| cell | trace | metric | sorted ms | largest gap ms | descriptive modes |",
1900:            ("paired total_ms d", item.paired_deltas),
1905:                f"| {item.cell} | {item.trace_state} | {metric} | {ordered} | "
1913:            f"`phase_events.csv` contains {trace_event_count} structured events. "
1914:            f"`phase_intervals.csv` contains {interval_count} local-clock intervals.",
1916:            "Each phase-event row carries the arm's validated `transfer_ms`, `settled_ms`, "
1924:            f"`clock_summary.csv` selects the minimum-RTT before and after sample for each "
1925:            f"of {clock_arm_count} scheduled arms and reports its midpoint offset. These "
1926:            "samples document cross-host uncertainty only; no cross-host phase duration is "
1934:def analyze(root: Path | str) -> AnalysisResult:
1939:    clock_samples = load_clock_samples(output_dir, rows)
1940:    events = load_trace_events(output_dir)
1941:    grouped = validate_traces(rows, events)
1943:        condition_stats(rows, cell, trace_state)
1945:        for trace_state in ("off", "on")
1947:    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
1948:    observer_bias = abs(target["on"].delta - target["off"].delta)
1949:    n_resolution = max(target["off"].n_pair, target["on"].n_pair, observer_bias)
1950:    event_rows, interval_rows = _phase_rows(rows, grouped)
1955:    phase_events_csv = output_dir / "phase_events.csv"
1956:    phase_intervals_csv = output_dir / "phase_intervals.csv"
1957:    clock_summary_csv = output_dir / "clock_summary.csv"
1961:        _summary_rows(stats, observer_bias, n_resolution),
1967:            "trace_state",
1978:    _atomic_csv(phase_events_csv, EVENT_FIELDS, event_rows)
1979:    _atomic_csv(phase_intervals_csv, INTERVAL_FIELDS, interval_rows)
1980:    clock_rows = _clock_summary_rows(clock_samples)
1981:    _atomic_csv(clock_summary_csv, CLOCK_SUMMARY_FIELDS, clock_rows)
1986:            observer_bias,
1987:            n_resolution,
1990:            len(clock_rows),
1997:        phase_events_csv=phase_events_csv,
1998:        phase_intervals_csv=phase_intervals_csv,
1999:        clock_summary_csv=clock_summary_csv,
2000:        observer_bias=observer_bias,
2001:        n_resolution=n_resolution,
2002:        trace_event_count=len(events),
2006:def main(argv: Sequence[str] | None = None) -> int:

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/bench_otp12pf_rigw.sh | nl -ba | sed -n '1,450p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uW4VEkq9' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-VWcKgVVd' (errno=Operation not permitted)
     1	#!/usr/bin/env bash
     2	# bench_otp12pf_rigw.sh — focused pf-1 P1 phase diagnostic on q ↔ Windows.
     3	#
     4	# Execute this script ON q, from an isolated clean clone of the reviewed
     5	# commit.  It measures semantic initiator roles, never legacy push/pull
     6	# implementations: SOURCE always sends and DESTINATION always receives.
     7	# The only varied property within a pair is which endpoint initiates the
     8	# Transfer RPC and therefore which endpoint dials the peer.
     9	#
    10	# Registered diagnostic (128 timed transfers):
    11	#   B1 trace OFF, forward cell order, pairs 1..4
    12	#   B2 trace ON,  reverse cell order, pairs 1..4
    13	#   B3 trace ON,  forward cell order, pairs 5..8
    14	#   B4 trace OFF, reverse cell order, pairs 5..8
    15	# Each round traverses cells base/reverse/reverse/base and runs the two roles
    16	# adjacently.  Each trace state therefore has eight valid role pairs per cell,
    17	# balanced four/four for which role goes first.
    18	#
    19	# This is the reduced P1 rig diagnostic.  It does NOT by itself close pf-1:
    20	# the active plan separately requires the small-fixture/P2 work and 0f922de
    21	# historical control before the pf-1 hard gate is complete.
    22
    23	set -Eeuo pipefail
    24
    25	SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    26	REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
    27
    28	SELFTEST=${SELFTEST:-0}
    29	PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
    30	LAUNCHER_SMOKE=${LAUNCHER_SMOKE:-0}
    31	EXPECT_SHA=${EXPECT_SHA:-}
    32
    33	# The experiment identity is deliberately not configurable.  In particular,
    34	# using a hostname here would hit q's stale netwatch-01 known_hosts entry;
    35	# every q→Windows control and transfer uses the pinned numeric endpoint.
    36	Q_EXPECT_HOST=q.lan
    37	Q_NIC=en8
    38	Q_IP=10.1.10.54
    39	Q_MAC=00:01:d2:19:04:a3
    40	WIN_SSH=michael@10.1.10.177
    41	WIN_IP=10.1.10.177
    42	WIN_NIC=Ethernet
    43	WIN_MAC=34-5A-60-3E-78-8B
    44	REGISTERED_MTU=9000
    45	REGISTERED_MEDIA=10Gbase-T
    46	Q_TO_WIN_MSS=8948
    47	WIN_TO_Q_MSS=8960
    48	PORT=9031
    49	PAIRS_PER_BLOCK=4
    50	LOAD1_MAX=3.0
    51	SPOTLIGHT_CPU_MAX=10.0
    52	WIN_CPU_MAX=20.0
    53	SETTLE_NS=250000000
    54	SETTLE_MIN_MS=250
    55	SETTLE_MAX_MS=1000
    56
    57	Q_MODULE="$HOME/blit-bench-work"
    58	Q_BLIT="$REPO_ROOT/target/release/blit"
    59	Q_DAEMON="$REPO_ROOT/target/release/blit-daemon"
    60	WIN_ROOT='D:/blit-test'
    61	WIN_MODULE="$WIN_ROOT/rigw-module"
    62	WIN_BINS="$WIN_ROOT/bins"
    63	WIN_ACTIVE="$WIN_BINS/active/blit-daemon.exe"
    64	WIN_PURGE_SOURCE="$SCRIPT_DIR/windows/purge-standby.ps1"
    65
    66	SESSION_TAG=$(date -u +%Y%m%dT%H%M%SZ).$$
    67	OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12pf-rigw-$SESSION_TAG}
    68	WIN_SESSION="$WIN_ROOT/rigw-pf1/$SESSION_TAG"
    69	WIN_PURGE="$WIN_SESSION/purge-standby.ps1"
    70	WIN_PURGE_HASH=""
    71
    72	LOG="$OUT_DIR/bench.log"
    73	RUNS_CSV="$OUT_DIR/runs.csv"
    74	CLOCK_CSV="$OUT_DIR/clock-samples.csv"
    75
    76	LAST_ERROR=""
    77	OUTPUT_CLAIMED=0
    78	OUTPUT_CLAIM_ERROR=""
    79	log() {
    80	    local line
    81	    line="$(date -u +%H:%M:%SZ) $*"
    82	    if [[ "$OUTPUT_CLAIMED" == 1 ]]; then
    83	        printf '%s\n' "$line" | tee -a "$LOG"
    84	    else
    85	        printf '%s\n' "$line" >&2
    86	    fi
    87	}
    88	die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
    89	append_void_line() {
    90	    printf '%s\n' "$1" >> "$OUT_DIR/SESSION-VOID"
    91	}
    92	session_void() {
    93	    local reason="$1"
    94	    LAST_ERROR="$reason"
    95	    append_void_line "$reason"
    96	    log "SESSION-VOID: $reason"
    97	    exit 1
    98	}
    99
   100	reserve_evidence_dir() {
   101	    local target="$1" parent
   102	    OUTPUT_CLAIM_ERROR=""
   103	    if [[ -e "$target" || -L "$target" ]]; then
   104	        if [[ -f "$target/SESSION-COMPLETE" ]]; then
   105	            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-COMPLETE: $target"
   106	        elif [[ -f "$target/SESSION-VOID" ]]; then
   107	            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-VOID: $target"
   108	        else
   109	            OUTPUT_CLAIM_ERROR="refusing existing output path (must be fresh): $target"
   110	        fi
   111	        return 1
   112	    fi
   113	    parent=$(dirname "$target")
   114	    mkdir -p "$parent" || {
   115	        OUTPUT_CLAIM_ERROR="cannot create output parent: $parent"
   116	        return 1
   117	    }
   118	    mkdir "$target" || {
   119	        OUTPUT_CLAIM_ERROR="cannot atomically claim output directory: $target"
   120	        return 1
   121	    }
   122	    mkdir "$target/trace" "$target/client" "$target/fixtures" "$target/landed" || {
   123	        OUTPUT_CLAIM_ERROR="cannot initialize output directory: $target"
   124	        rm -rf "$target"
   125	        return 1
   126	    }
   127	}
   128
   129	claim_output_dir() {
   130	    reserve_evidence_dir "$OUT_DIR" || return 1
   131	    OUTPUT_CLAIMED=1
   132	}
   133
   134	SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto \
   135	    -o ConnectTimeout=5 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 \
   136	    -o "ControlPath=$HOME/.ssh/cm-rigw-%r@%h-%p" -o ControlPersist=300)
   137	wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
   138	wscp() { scp "${SSH_MUX[@]}" "$@"; }
   139
   140	q_daemon_pid=""
   141	win_daemon_pid=""
   142	win_cmd_pid=""
   143	current_block=""
   144	CLEANUP_MODE=0
   145	CLEANUP_ERROR=""
   146	REGISTERED_RUN_STARTED=0
   147	SESSION_FINALIZED=0
   148	STRICT_CLEANUP_VERIFIED=0
   149	Q_SESSION_MAY_EXIST=0
   150	WIN_SESSION_MAY_EXIST=0
   151	LOCAL_EVIDENCE_COMPLETE=0
   152
   153	teardown_die() {
   154	    local reason="$1"
   155	    if [[ "$CLEANUP_MODE" == 1 ]]; then
   156	        CLEANUP_ERROR="${CLEANUP_ERROR:+$CLEANUP_ERROR; }$reason"
   157	        log "CLEANUP-ERROR: $reason"
   158	        return 1
   159	    fi
   160	    session_void "$reason"
   161	}
   162
   163	reject_registered_overrides() {
   164	    local name
   165	    for name in RUNS CELLS MAC_HOST WIN_HOST WIN_SSH_OVERRIDE PORT_OVERRIDE \
   166	        Q_NIC_OVERRIDE Q_IP_OVERRIDE TRACE_ORDER PAIRS_PER_BLOCK_OVERRIDE; do
   167	        if [[ -n "${!name+x}" ]]; then
   168	            die "$name is not configurable for the registered rig-W diagnostic"
   169	        fi
   170	    done
   171	}
   172
   173	validate_mode_selection() {
   174	    local name value enabled=0
   175	    for name in SELFTEST PREFLIGHT_ONLY LAUNCHER_SMOKE; do
   176	        value=${!name}
   177	        [[ "$value" == 0 || "$value" == 1 ]] \
   178	            || die "$name must be exactly 0 or 1"
   179	        if [[ "$value" == 1 ]]; then
   180	            enabled=$((enabled + 1))
   181	        fi
   182	    done
   183	    [[ "$enabled" -le 1 ]] \
   184	        || die "SELFTEST, PREFLIGHT_ONLY, and LAUNCHER_SMOKE are mutually exclusive"
   185	}
   186
   187	emit_schedule() {
   188	    cat <<'EOF'
   189	1,off,forward,1,4
   190	2,on,reverse,1,4
   191	3,on,forward,5,8
   192	4,off,reverse,5,8
   193	EOF
   194	}
   195
   196	q_source_path() { printf '%s/src_%s' "$Q_MODULE" "$1"; }
   197	win_source_path() { printf '%s/src_%s' "$WIN_MODULE" "$1"; }
   198	destination_relative_path() {
   199	    # Accept the role so callers cannot accidentally omit the parity axis, but
   200	    # deliberately keep it out of the measured path.  Every arm in this
   201	    # registered session reuses this one endpoint-local destination.
   202	    case "$1" in source_init|destination_init);; *) return 2;; esac
   203	    printf 'rigw-sessions/%s/destination/container' "$SESSION_TAG"
   204	}
   205	q_destination_path() {
   206	    printf '%s/%s' "$Q_MODULE" "$(destination_relative_path "$1")"
   207	}
   208	win_destination_path() {
   209	    printf '%s/%s' "$WIN_MODULE" "$(destination_relative_path "$1")"
   210	}
   211	arm_destination_path() {
   212	    local direction="$1" role="$2"
   213	    case "$direction" in
   214	        wm) q_destination_path "$role";;
   215	        mw) win_destination_path "$role";;
   216	        *) return 2;;
   217	    esac
   218	}
   219	arm_destination_argument() {
   220	    local direction="$1" role="$2" relative
   221	    relative=$(destination_relative_path "$role") || return 2
   222	    case "$direction/$role" in
   223	        wm/source_init) printf '%s:%s:/bench/%s/' "$Q_IP" "$PORT" "$relative";;
   224	        wm/destination_init) q_destination_path "$role";;
   225	        mw/source_init) printf '%s:%s:/bench/%s/' "$WIN_IP" "$PORT" "$relative";;
   226	        mw/destination_init) win_destination_path "$role";;
   227	        *) return 2;;
   228	    esac
   229	}
   230	append_clock_row() {
   231	    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' "$@"
   232	}
   233	q_monotonic_ns() {
   234	    python3 -c 'import time; print(time.clock_gettime_ns(time.CLOCK_MONOTONIC))'
   235	}
   236	settle_until_deadline() {
   237	    python3 - "$1" <<'PY'
   238	import sys, time
   239
   240	deadline_ns = int(sys.argv[1])
   241	clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
   242	remaining_ns = deadline_ns - clock_ns()
   243	if remaining_ns > 0:
   244	    time.sleep(remaining_ns / 1_000_000_000)
   245	print(clock_ns())
   246	PY
   247	}
   248	stamp_result_arrival_on_q() {
   249	    python3 -c '
   250	import sys, time
   251
   252	result = None
   253	stamp_ns = None
   254	for raw in sys.stdin:
   255	    line = raw.rstrip("\r\n")
   256	    if not line.startswith("R|"):
   257	        continue
   258	    if result is not None:
   259	        raise SystemExit("multiple Windows client result sentinels")
   260	    fields = line.split("|")
   261	    if len(fields) != 3:
   262	        raise SystemExit("malformed Windows client result sentinel")
   263	    result = line
   264	    stamp_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
   265	if result is None or stamp_ns is None:
   266	    raise SystemExit("missing Windows client result sentinel")
   267	print(f"{result}|{stamp_ns}")
   268	'
   269	}
   270	successful_windows_log_phase_ok() {
   271	    [[ "$1" == durability_verified ]]
   272	}
   273	fetch_successful_windows_client_log() {
   274	    local arm_phase="$1" remote_err="$2" local_err="$3"
   275	    successful_windows_log_phase_ok "$arm_phase" \
   276	        || session_void "refusing successful Windows client-log fetch before destination durability"
   277	    fetch_win_file "$remote_err" "$local_err"
   278	}
   279	embeds_clean_q() {
   280	    local path="$1"
   281	    LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID" "$path" || return 1
   282	    LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID.dirty" "$path" && return 1
   283	    return 0
   284	}
   285
   286	selftest() {
   287	    local got expected rows source_first destination_first clock_probe identity_file
   288	    local selftest_client_done selftest_deadline selftest_settle_done run_arm_source
   289	    local manifest_tmp canonical_manifest landed_manifest tree_digest
   290	    local freshness_tmp freshness_case marker before analyzer_log
   291	    local win_stop_source win_start_source finalize_tmp failure_tmp trap_calls trap_rc
   292	    local signal signal_dir signal_rc contract_tmp on_exit_source append_tmp
   293	    local cleanup_tmp remembered port_checks strict_cleanup_source
   294	    local destination_tmp prepare_destination_source stamped_result stamp_before stamp_after
   295	    local stamp_tag stamp_ms stamp_rc stamp_ns stamp_extra stamp_teardown_ns
   296	    local cross_clock_before cross_clock_after cross_clock_delta
   297	    local launcher_tmp launcher_calls launcher_source main_source
   298	    local win_recovery_tmp purge_contract_tmp purge_hash drained preflight_source
   299	    reject_registered_overrides
   300	    if (
   301	        SELFTEST=1
   302	        PREFLIGHT_ONLY=1
   303	        LAUNCHER_SMOKE=0
   304	        validate_mode_selection
   305	    ) >/dev/null 2>&1; then
   306	        die "multiple harness modes were accepted"
   307	    fi
   308	    if (
   309	        SELFTEST=2
   310	        PREFLIGHT_ONLY=0
   311	        LAUNCHER_SMOKE=0
   312	        validate_mode_selection
   313	    ) >/dev/null 2>&1; then
   314	        die "invalid harness mode value was accepted"
   315	    fi
   316	    got=$(emit_schedule)
   317	    expected=$'1,off,forward,1,4\n2,on,reverse,1,4\n3,on,forward,5,8\n4,off,reverse,5,8'
   318	    [[ "$got" == "$expected" ]] || die "registered block schedule changed"
   319
   320	    rows=0; source_first=0; destination_first=0
   321	    local block state pass first last round pair first_role
   322	    while IFS=, read -r block state pass first last; do
   323	        for ((round=1; round<=PAIRS_PER_BLOCK; round++)); do
   324	            pair=$((first + round - 1))
   325	            case "$round" in
   326	                1|4) first_role=source_init; source_first=$((source_first + 4));;
   327	                2|3) first_role=destination_init; destination_first=$((destination_first + 4));;
   328	            esac
   329	            [[ "$pair" -ge "$first" && "$pair" -le "$last" && -n "$first_role" ]]
   330	            rows=$((rows + 8)) # four cells × two adjacent roles
   331	        done
   332	    done < <(emit_schedule)
   333	    [[ "$rows" == 128 ]] || die "schedule emitted $rows arms, expected 128"
   334	    [[ "$source_first" == 32 && "$destination_first" == 32 ]] \
   335	        || die "schedule role-first balance changed"
   336	    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]] \
   337	        || die "q source path construction changed"
   338	    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]] \
   339	        || die "Windows source path construction changed"
   340	    local destination_rel="rigw-sessions/$SESSION_TAG/destination/container"
   341	    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]] \
   342	        || die "q SOURCE-initiated destination path changed"
   343	    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]] \
   344	        || die "q DESTINATION-initiated destination path changed"
   345	    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]] \
   346	        || die "Windows SOURCE-initiated destination path changed"
   347	    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
   348	        || die "Windows DESTINATION-initiated destination path changed"
   349	    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]] \
   350	        || die "Windows-to-q physical destination depends on initiator role"
   351	    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]] \
   352	        || die "q-to-Windows physical destination depends on initiator role"
   353	    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]] \
   354	        || die "Windows-to-q SOURCE-initiated destination argument changed"
   355	    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]] \
   356	        || die "Windows-to-q DESTINATION-initiated destination argument changed"
   357	    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]] \
   358	        || die "q-to-Windows SOURCE-initiated destination argument changed"
   359	    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
   360	        || die "q-to-Windows DESTINATION-initiated destination argument changed"
   361	    local arp_fixture
   362	    arp_fixture=$'? (10.1.10.177) at 34:5a:60:3e:78:8b on en0 ifscope [ethernet]\n? (10.1.10.177) at 34:5a:60:3e:78:8b on en1 ifscope [ethernet]\n? (10.1.10.177) at 34:5a:60:3e:78:8b on en8 ifscope [ethernet]'
   363	    [[ "$(q_peer_mac_from_arp en8 <<<"$arp_fixture")" == "34:5a:60:3e:78:8b" ]] \
   364	        || die "q ARP parser did not select exactly the registered interface"
   365	    purge_contract_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-purge-contract.XXXXXX")
   366	    printf '%s\n' '# reviewed purge helper fixture' > "$purge_contract_tmp/purge-standby.ps1"
   367	    purge_hash=$(sha256_q "$purge_contract_tmp/purge-standby.ps1")
   368	    if ! (
   369	        WIN_PURGE_SOURCE="$purge_contract_tmp/purge-standby.ps1"
   370	        WIN_SESSION='D:/blit-test/rigw-pf1/selftest'
   371	        WIN_PURGE="$WIN_SESSION/purge-standby.ps1"
   372	        WIN_PURGE_HASH=""
   373	        wscp() {
   374	            [[ "$#" == 2 && "$1" == "$WIN_PURGE_SOURCE" \
   375	                && "$2" == "$WIN_SSH:$WIN_PURGE.tmp" ]] || return 91
   376	        }
   377	        wssh() {
   378	            local command="$1"
   379	            if [[ "$command" == *"refusing existing Windows session tree"* ]]; then
   380	                [[ "$command" == *"New-Item -ItemType Directory -Path '$WIN_SESSION'"* ]] \
   381	                    || return 92
   382	                return 0
   383	            fi
   384	            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE.tmp'"* ]] \
   385	                || return 93
   386	            [[ "$command" == *"if (\$tmpHash -cne '$purge_hash')"* ]] || return 94
   387	            [[ "$command" == *"Move-Item -LiteralPath '$WIN_PURGE.tmp' -Destination '$WIN_PURGE'"* ]] \
   388	                || return 95
   389	            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE'"* ]] \
   390	                || return 96
   391	            [[ "$command" == *"if (\$finalHash -cne '$purge_hash')"* ]] || return 97
   392	            printf 'H|%s\n' "$purge_hash"
   393	        }
   394	        stage_purge_helper
   395	        [[ "$WIN_PURGE_HASH" == "$purge_hash" ]] || exit 98
   396	    ); then
   397	        rm -rf "$purge_contract_tmp"
   398	        die "reviewed Windows purge helper was not staged and hash-verified"
   399	    fi
   400	    if ! (
   401	        WIN_PURGE='D:/blit-test/rigw-pf1/selftest/purge-standby.ps1'
   402	        WIN_PURGE_HASH="$purge_hash"
   403	        sync() { return 0; }
   404	        sudo() { return 0; }
   405	        wssh() {
   406	            local command="$1"
   407	            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE'"* ]] \
   408	                || return 101
   409	            [[ "$command" == *"if (\$purgeHash -cne '$WIN_PURGE_HASH')"* ]] \
   410	                || return 102
   411	            [[ "$command" == *"\$purgeOutput = @(& pwsh -NoProfile -File '$WIN_PURGE')"* ]] \
   412	                || return 103
   413	            [[ "$command" == *"\$purgeOutput.Count -ne 1"* \
   414	                && "$command" == *"[string]\$purgeOutput[0] -cne 'standby-purged'"* ]] \
   415	                || return 104
   416	        }
   417	        drained=$(drain_both)
   418	        [[ "$drained" == drained ]] || exit 105
   419	    ); then
   420	        rm -rf "$purge_contract_tmp"
   421	        die "Windows purge helper was not hash-verified per arm with exact success output"
   422	    fi
   423	    rm -rf "$purge_contract_tmp"
   424	    preflight_source=$(declare -f preflight)
   425	    python3 - "$preflight_source" <<'PY' \
   426	        || die "purge-helper staging moved ahead of read-only endpoint gates"
   427	import sys
   428
   429	source = sys.argv[1]
   430	markers = (
   431	    "provenance_gate",
   432	    "ports_closed",
   433	    "q_topology_gate",
   434	    "win_topology_gate",
   435	    "mss_gate",
   436	    "firewall_gate",
   437	    "q_quiet_gate",
   438	    "win_quiet_gate",
   439	    "timer_gate",
   440	    "windows_result_stream_gate",
   441	    "stage_purge_helper",
   442	    "write_manifest",
   443	    "verify_fixtures",
   444	)
   445	positions = [source.index(marker) for marker in markers]
   446	if positions != sorted(positions) or source.count("stage_purge_helper") != 1:
   447	    raise SystemExit(f"preflight marker order changed: {positions}")
   448	PY
   449	    clock_probe=$(append_clock_row 1 run cell 1 source_init before 1 10 11 12 2 0)
   450	    [[ "$(awk -F, '{print NF}' <<<"$clock_probe")" == 12 ]] \

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/bench_otp12pf_rigw.sh | nl -ba | sed -n '451,900p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XeiKsIY9' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Z5VvCiom' (errno=Operation not permitted)
   451	        || die "clock sample row is not exactly 12 columns"
   452	    [[ "$SETTLE_NS" == 250000000 && "$SETTLE_MIN_MS" == 250 && "$SETTLE_MAX_MS" == 1000 ]] \
   453	        || die "registered post-client settle bounds changed"
   454	    cross_clock_before=$(q_monotonic_ns)
   455	    sleep 0.05
   456	    cross_clock_after=$(q_monotonic_ns)
   457	    cross_clock_delta=$((cross_clock_after - cross_clock_before))
   458	    [[ "$cross_clock_delta" -ge 40000000 && "$cross_clock_delta" -lt 500000000 ]] \
   459	        || die "q monotonic clock is not comparable across processes"
   460	    selftest_client_done=$(q_monotonic_ns)
   461	    selftest_deadline=$((selftest_client_done + SETTLE_NS))
   462	    selftest_settle_done=$(settle_until_deadline "$selftest_deadline")
   463	    [[ "$selftest_settle_done" =~ ^[0-9]+$ && "$selftest_settle_done" -ge "$selftest_deadline" ]] \
   464	        || die "absolute post-client deadline wait returned early"
   465	    stamp_before=$(q_monotonic_ns)
   466	    stamped_result=$(
   467	        { printf '%s\n' 'R|17|0'; sleep 0.35; } | stamp_result_arrival_on_q
   468	    ) || die "q result-arrival stamper rejected one exact sentinel"
   469	    stamp_after=$(q_monotonic_ns)
   470	    IFS='|' read -r stamp_tag stamp_ms stamp_rc stamp_ns stamp_extra <<<"$stamped_result"
   471	    [[ "$stamp_tag" == R && "$stamp_ms" == 17 && "$stamp_rc" == 0 \
   472	        && "$stamp_ns" =~ ^[0-9]+$ && -z "$stamp_extra" ]] \
   473	        || die "q result-arrival stamper returned '$stamped_result'"
   474	    [[ "$stamp_ns" -ge "$stamp_before" && "$stamp_ns" -le "$stamp_after" ]] \
   475	        || die "q result-arrival stamp is outside the producer lifetime"
   476	    stamp_teardown_ns=$((stamp_after - stamp_ns))
   477	    [[ "$stamp_teardown_ns" -ge 250000000 ]] \
   478	        || die "q result-arrival stamp moved after producer teardown"
   479	    if successful_windows_log_phase_ok client_done; then
   480	        die "successful Windows client log was fetchable before durability"
   481	    fi
   482	    successful_windows_log_phase_ok durability_verified \
   483	        || die "successful Windows client log was blocked after durability"
   484
   485	    run_arm_source=$(declare -f run_arm)
   486	    python3 - "$run_arm_source" <<'PY' || die "run_arm post-client ordering changed"
   487	import sys
   488
   489	source = sys.argv[1]
   490	markers = (
   491	    'dest=$(arm_destination_path "$direction" "$role")',
   492	    'dest_arg=$(arm_destination_argument "$direction" "$role")',
   493	    "read -r result_tag transfer_ms rc client_done_ns result_extra",
   494	    'settle_deadline_ns=$((client_done_ns + SETTLE_NS))',
   495	    'record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after',
   496	    'settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")',
   497	    'flush_out=$(flush_verify_q "$dest")',
   498	    'flush_out=$(flush_verify_win "$dest")',
   499	    'arm_phase=durability_verified',
   500	    'fetch_successful_windows_client_log "$arm_phase" "$remote_err" "$werr"',
   501	    'total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))',
   502	)
   503	positions = []
   504	for marker in markers:
   505	    try:
   506	        positions.append(source.index(marker))
   507	    except ValueError as exc:
   508	        raise SystemExit(f"missing run_arm ordering marker: {marker}") from exc
   509	if positions != sorted(positions):
   510	    raise SystemExit(f"run_arm ordering markers out of order: {positions}")
   511	for forbidden in (
   512	    'q_destination_path "$rid"',
   513	    'win_destination_path "$rid"',
   514	    '$SESSION_TAG/$rid/container',
   515	    'client_done_ns=$(q_monotonic_ns)',
   516	):
   517	    if forbidden in source:
   518	        raise SystemExit(f"forbidden run_arm pattern returned: {forbidden}")
   519	PY
   520
   521	    python3 - "$(declare -f q_client_run)" "$(declare -f win_client_run)" <<'PY' \
   522	        || die "client completion-anchor contract changed"
   523	import sys
   524
   525	q_client, win_client = sys.argv[1:]
   526	q_markers = (
   527	    "clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)",
   528	    "p=subprocess.run(",
   529	    "done_ns=clock_ns()",
   530	    'print(f"R|{ms}|{p.returncode}|{done_ns}")',
   531	)
   532	q_positions = [q_client.index(marker) for marker in q_markers]
   533	if q_positions != sorted(q_positions):
   534	    raise SystemExit(f"q completion markers out of order: {q_positions}")
   535	for marker in (
   536	    "[Console]::Out.WriteLine(",
   537	    "[Console]::Out.Flush()",
   538	    "| stamp_result_arrival_on_q",
   539	):
   540	    if marker not in win_client:
   541	        raise SystemExit(f"missing streamed Windows completion marker: {marker}")
   542	PY
   543
   544	    win_stop_source=$(declare -f win_daemon_stop)
   545	    win_start_source=$(declare -f win_daemon_start)
   546	    python3 - "$win_stop_source" "$win_start_source" <<'PY' \
   547	        || die "Windows launcher/daemon identity contract changed"
   548	import sys
   549
   550	stop, start = sys.argv[1:]
   551	try:
   552	    recovery, remote_stop = stop.split(r"\$pid0 = if", 1)
   553	except ValueError as exc:
   554	    raise SystemExit("Windows stop script lost its recovery boundary") from exc
   555	recovery_markers = (
   556	    r"if (-not \$c)",
   557	    r"\$actual -ieq \$expectedLauncher",
   558	    r"\$launchers.Count -gt 1",
   559	    r"if (-not \$d -and \$c -match '^[0-9]+$')",
   560	    r"\$_.ParentProcessId -eq [int]\$c",
   561	    r"\$children.Count -gt 1",
   562	    r'\"P|\$c|\$d\"',
   563	)
   564	try:
   565	    recovery_positions = [recovery.index(marker) for marker in recovery_markers]
   566	    empty_pid_branch = recovery.index('if [[ -z "$pid" && -z "$cmdpid" ]]')
   567	except ValueError as exc:
   568	    raise SystemExit(f"missing pre-PID-file recovery marker: {exc}") from exc
   569	if recovery_positions != sorted(recovery_positions) or recovery_positions[-1] >= empty_pid_branch:
   570	    raise SystemExit("empty-PID return can bypass exact Windows process discovery")
   571	for marker in (
   572	    r"elseif (\$cmd0)",
   573	    r"\$_.ParentProcessId -eq \$cmd0",
   574	    r"\$children.Count -gt 1",
   575	):
   576	    if marker not in remote_stop:
   577	        raise SystemExit(f"missing parent-based daemon recovery marker: {marker}")
   578	stop_markers = (
   579	    r"\$d.ParentProcessId -ne \$cmd0",
   580	    r"\$c.Name -ine 'cmd.exe'",
   581	    r"\$actualLauncher -ine \$expectedLauncher",
   582	    r"Stop-Process -Id \$stoppedDaemonPid",
   583	    r"Stop-Process -Id \$cmd0",
   584	)
   585	try:
   586	    positions = [remote_stop.index(marker) for marker in stop_markers]
   587	except ValueError as exc:
   588	    raise SystemExit(f"missing exact stop identity marker: {exc}") from exc
   589	if max(positions[:3]) >= min(positions[3:]):
   590	    raise SystemExit("a Windows process can be stopped before all identities validate")
   591	late_markers = (
   592	    r"Stop-Process -Id \$cmd0",
   593	    r"\$actualLate -ine '$WIN_ACTIVE'",
   594	    r"Stop-Process -Id \$late.ProcessId",
   595	    "late daemon child survived teardown",
   596	)
   597	late_positions = [remote_stop.index(marker) for marker in late_markers]
   598	if late_positions != sorted(late_positions):
   599	    raise SystemExit(f"late Windows child recovery is out of order: {late_positions}")
   600
   601	try:
   602	    generated_start, start_controller = start.split(r"\$launcherCommand =", 1)
   603	except ValueError as exc:
   604	    raise SystemExit("Windows start script lost its controller boundary") from exc
   605	batch_markers = (
   606	    "':wait_for_launch_ok'",
   607	    "launch.ok\\\" goto launch_ready",
   608	    "BLIT_LAUNCH_WAIT% GEQ 15 exit /b 111",
   609	    "'goto wait_for_launch_ok'",
   610	    "':launch_ready'",
   611	    r"'\"$WIN_ACTIVE\" --config",
   612	)
   613	batch_positions = [generated_start.index(marker) for marker in batch_markers]
   614	if batch_positions != sorted(batch_positions):
   615	    raise SystemExit(f"bounded Windows launch gate is out of order: {batch_positions}")
   616	controller_markers = (
   617	    "Invoke-CimMethod",
   618	    "launcher.pid.tmp' -Value",
   619	    "Move-Item -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp'",
   620	    r"\$persistedLauncher = (Get-Content",
   621	    r"\$persistedLauncher -ne [string]\$r.ProcessId",
   622	    "New-Item -ItemType File -Path '$WIN_SESSION/block_$block/launch.ok'",
   623	    "Start-Sleep -Seconds 2",
   624	)
   625	controller_positions = [start_controller.index(marker) for marker in controller_markers]
   626	if controller_positions != sorted(controller_positions):
   627	    raise SystemExit(f"Windows PID journal does not precede launch gate: {controller_positions}")
   628	for marker in (
   629	    r"\$actualLauncher -ine \$launcherCommand",
   630	    r"\$actualDaemon -ine '$WIN_ACTIVE'",
   631	    r"\$d.ParentProcessId -ne \$r.ProcessId",
   632	):
   633	    if marker not in start:
   634	        raise SystemExit(f"missing start identity marker: {marker}")
   635	PY
   636
   637	    win_recovery_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-win-recovery.XXXXXX")
   638	    (
   639	        current_block=launcher-smoke
   640	        win_daemon_pid=""
   641	        win_cmd_pid=""
   642	        wssh() {
   643	            local script="$1"
   644	            if [[ "$script" == *'$pid0 = if'* ]]; then
   645	                printf 'stop\n' >> "$win_recovery_tmp/calls"
   646	                printf 'STOPPED\n'
   647	            elif [[ "$script" == *'multiple exact launchers match'* ]]; then
   648	                printf 'recover\n' >> "$win_recovery_tmp/calls"
   649	                printf 'P|22|\n'
   650	            else
   651	                die "cmd-only Windows recovery fell into the empty-PID port branch"
   652	            fi
   653	        }
   654	        win_daemon_stop || die "cmd-only Windows recovery was rejected"
   655	        [[ -z "$win_daemon_pid" && -z "$win_cmd_pid" ]] \
   656	            || die "cmd-only Windows recovery retained remembered PIDs"
   657	        [[ "$(< "$win_recovery_tmp/calls")" == $'recover\nstop' ]] \
   658	            || die "cmd-only Windows recovery skipped exact stop"
   659	    )
   660	    rm -rf "$win_recovery_tmp"
   661
   662	    launcher_source=$(declare -f launcher_smoke)
   663	    main_source=$(declare -f main)
   664	    python3 - "$launcher_source" "$main_source" <<'PY' \
   665	        || die "standalone launcher-smoke control flow changed"
   666	import sys
   667
   668	smoke, main = sys.argv[1:]
   669	smoke_markers = (
   670	    "WIN_SESSION_MAY_EXIST=1",
   671	    "current_block=launcher-smoke",
   672	    "ports_closed",
   673	    'win_daemon_start "$current_block" off "$run_id"',
   674	    'nc -z -w 3 "$WIN_IP" "$PORT"',
   675	    'stop_daemons "$current_block"',
   676	    'current_block=""',
   677	    "strict_success_cleanup || session_void",
   678	)
   679	positions = []
   680	for marker in smoke_markers:
   681	    try:
   682	        positions.append(smoke.index(marker))
   683	    except ValueError as exc:
   684	        raise SystemExit(f"missing launcher-smoke marker: {marker}") from exc
   685	if positions != sorted(positions):
   686	    raise SystemExit(f"launcher-smoke markers out of order: {positions}")
   687	for forbidden in (
   688	    "REGISTERED_RUN_STARTED",
   689	    "SESSION_FINALIZED",
   690	    "SESSION-COMPLETE",
   691	    "q_daemon_start",
   692	    "start_daemons",
   693	    "run_arm",
   694	    "run_block",
   695	    "RUNS_CSV",
   696	    "CLOCK_CSV",
   697	    "emit_schedule",
   698	    "schedule.csv",
   699	    "otp12pf_rigw_analyze.py",
   700	    "finalize_registered_session",
   701	    "LOCAL_EVIDENCE_COMPLETE",
   702	    "Q_SESSION_MAY_EXIST",
   703	):
   704	    if forbidden in smoke:
   705	        raise SystemExit(f"launcher smoke reached registered work: {forbidden}")
   706	mode_check = main.index("validate_mode_selection")
   707	preflight = main.index("preflight;")
   708	branch_start = main.index('if [[ "$LAUNCHER_SMOKE" == 1 ]]')
   709	registered = main.index("REGISTERED_RUN_STARTED=1", branch_start)
   710	if not mode_check < preflight < branch_start < registered:
   711	    raise SystemExit("main launcher-smoke gate moved around preflight or registration")
   712	branch = main[branch_start:registered]
   713	branch_markers = ('if [[ "$LAUNCHER_SMOKE" == 1 ]]', "launcher_smoke;", "return;", "fi;")
   714	branch_positions = [branch.index(marker) for marker in branch_markers]
   715	if branch_positions != sorted(branch_positions) or branch.count("return;") != 1:
   716	    raise SystemExit(f"launcher-smoke branch can fall through: {branch_positions}")
   717	PY
   718
   719	    launcher_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-launcher-smoke.XXXXXX")
   720	    launcher_calls="$launcher_tmp/calls"
   721	    (
   722	        OUT_DIR="$launcher_tmp/evidence"
   723	        mkdir "$OUT_DIR"
   724	        LOG="$OUT_DIR/bench.log"
   725	        OUTPUT_CLAIMED=1
   726	        SESSION_TAG=offline-smoke
   727	        REGISTERED_RUN_STARTED=0
   728	        SESSION_FINALIZED=0
   729	        STRICT_CLEANUP_VERIFIED=0
   730	        Q_SESSION_MAY_EXIST=0
   731	        WIN_SESSION_MAY_EXIST=0
   732	        LOCAL_EVIDENCE_COMPLETE=0
   733	        current_block=""
   734	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""
   735	        port_checks=0
   736	        log() { :; }
   737	        win_daemon_start() {
   738	            [[ "$1" == launcher-smoke && "$2" == off \
   739	                && "$3" == offline-smoke-launcher-smoke \
   740	                && "$WIN_SESSION_MAY_EXIST" == 1 \
   741	                && "$current_block" == launcher-smoke ]] \
   742	                || die "offline launcher smoke started with wrong identity"
   743	            printf 'start\n' >> "$launcher_calls"
   744	            win_cmd_pid=22; win_daemon_pid=33
   745	        }
   746	        nc() {
   747	            [[ "$*" == "-z -w 3 $WIN_IP $PORT" \
   748	                && "$win_cmd_pid" == 22 && "$win_daemon_pid" == 33 ]] \
   749	                || die "offline launcher smoke reachability ran out of order"
   750	            printf 'reach\n' >> "$launcher_calls"
   751	        }
   752	        win_daemon_stop() {
   753	            [[ "$current_block" == launcher-smoke \
   754	                && "$win_cmd_pid" == 22 && "$win_daemon_pid" == 33 ]] \
   755	                || die "offline launcher smoke stopped the wrong daemon"
   756	            printf 'stop\n' >> "$launcher_calls"
   757	            win_cmd_pid=""; win_daemon_pid=""
   758	        }
   759	        q_daemon_stop() {
   760	            [[ -z "$q_daemon_pid" ]] \
   761	                || die "offline launcher smoke unexpectedly owned a q daemon"
   762	            printf 'q-stop-empty\n' >> "$launcher_calls"
   763	        }
   764	        collect_block_logs() {
   765	            [[ "$1" == launcher-smoke && -z "$win_cmd_pid" \
   766	                && -z "$win_daemon_pid" ]] \
   767	                || die "offline launcher smoke collected before exact stop"
   768	            printf 'collect\n' >> "$launcher_calls"
   769	        }
   770	        ports_closed() {
   771	            port_checks=$((port_checks + 1))
   772	            if [[ "$port_checks" == 1 ]]; then
   773	                [[ "$current_block" == launcher-smoke \
   774	                    && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   775	                    || die "offline launcher smoke skipped its pre-start port check"
   776	                printf 'closed-pre\n' >> "$launcher_calls"
   777	            else
   778	                [[ "$port_checks" == 2 && "$current_block" == launcher-smoke \
   779	                    && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   780	                    || die "offline launcher smoke checked ports before exact stop"
   781	                printf 'closed-post\n' >> "$launcher_calls"
   782	            fi
   783	        }
   784	        strict_success_cleanup() {
   785	            [[ "$WIN_SESSION_MAY_EXIST" == 1 && -z "$current_block" \
   786	                && -z "$q_daemon_pid" && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   787	                || die "offline launcher smoke cleaned before exact stop"
   788	            printf 'cleanup\n' >> "$launcher_calls"
   789	            WIN_SESSION_MAY_EXIST=0
   790	            STRICT_CLEANUP_VERIFIED=1
   791	        }
   792	        launcher_smoke
   793	        [[ "$(< "$launcher_calls")" == \
   794	            $'closed-pre\nstart\nreach\nstop\nq-stop-empty\ncollect\nclosed-post\ncleanup' ]] \
   795	            || die "offline launcher-smoke call order changed"
   796	        [[ "$REGISTERED_RUN_STARTED" == 0 && "$SESSION_FINALIZED" == 0 \
   797	            && "$STRICT_CLEANUP_VERIFIED" == 1 \
   798	            && "$Q_SESSION_MAY_EXIST" == 0 \
   799	            && "$WIN_SESSION_MAY_EXIST" == 0 \
   800	            && "$LOCAL_EVIDENCE_COMPLETE" == 0 ]] \
   801	            || die "offline launcher smoke changed registered state"
   802	        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" \
   803	            && ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
   804	            && ! -e "$OUT_DIR/SESSION-VOID" ]] \
   805	            || die "offline launcher smoke left a session marker"
   806	    )
   807	    rm -rf "$launcher_tmp"
   808
   809	    HEAD_BUILD_ID=0123456789ab
   810	    identity_file=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-identity.XXXXXX")
   811	    printf 'blit+%s\0' "$HEAD_BUILD_ID" > "$identity_file"
   812	    embeds_clean_q "$identity_file" || die "clean 12-character build identity was rejected"
   813	    printf 'blit+%s.dirty.ffffffffffff\0' "$HEAD_BUILD_ID" > "$identity_file"
   814	    if embeds_clean_q "$identity_file"; then
   815	        rm -f "$identity_file"
   816	        die "dirty build identity was accepted"
   817	    fi
   818	    rm -f "$identity_file"
   819
   820	    manifest_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-manifest.XXXXXX")
   821	    mkdir -p "$manifest_tmp/source/sub" "$manifest_tmp/container/src_mixed/sub"
   822	    printf 'a' > "$manifest_tmp/source/a"
   823	    printf 'bc' > "$manifest_tmp/source/sub/b"
   824	    printf 'a' > "$manifest_tmp/container/src_mixed/a"
   825	    printf 'bc' > "$manifest_tmp/container/src_mixed/sub/b"
   826	    canonical_manifest="$manifest_tmp/canonical.manifest"
   827	    landed_manifest="$manifest_tmp/landed.manifest"
   828	    write_q_tree_manifest "$manifest_tmp/source" "$canonical_manifest"
   829	    write_q_tree_manifest \
   830	        "$manifest_tmp/container" "$landed_manifest" src_mixed
   831	    tree_digest=$(matching_manifest_digest "$canonical_manifest" "$landed_manifest") \
   832	        || die "identical relative-path/size manifests did not match"
   833	    [[ "$tree_digest" =~ ^[0-9a-f]{64}$ ]] \
   834	        || die "tree manifest digest is malformed"
   835	    printf 'aa' > "$manifest_tmp/container/src_mixed/a"
   836	    printf 'b' > "$manifest_tmp/container/src_mixed/sub/b"
   837	    write_q_tree_manifest \
   838	        "$manifest_tmp/container" "$landed_manifest" src_mixed
   839	    if matching_manifest_digest "$canonical_manifest" "$landed_manifest" >/dev/null; then
   840	        rm -rf "$manifest_tmp"
   841	        die "same-count/same-byte tree with swapped file sizes was accepted"
   842	    fi
   843	    rm -rf "$manifest_tmp/container/src_mixed"
   844	    mkdir -p "$manifest_tmp/container/wrapper/src_mixed"
   845	    if write_q_tree_manifest \
   846	        "$manifest_tmp/container" "$landed_manifest" src_mixed 2>/dev/null; then
   847	        rm -rf "$manifest_tmp"
   848	        die "wrong landed root wrapper was accepted"
   849	    fi
   850	    rm -rf "$manifest_tmp"
   851
   852	    freshness_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-freshness.XXXXXX")
   853	    reserve_evidence_dir "$freshness_tmp/new-evidence" \
   854	        || die "fresh evidence directory was rejected: $OUTPUT_CLAIM_ERROR"
   855	    for marker in SESSION-COMPLETE SESSION-VOID unrelated.txt; do
   856	        freshness_case="$freshness_tmp/$marker"
   857	        mkdir "$freshness_case"
   858	        printf 'preserve-me\n' > "$freshness_case/$marker"
   859	        before=$(sha256_q "$freshness_case/$marker")
   860	        if reserve_evidence_dir "$freshness_case"; then
   861	            rm -rf "$freshness_tmp"
   862	            die "stale output directory containing $marker was accepted"
   863	        fi
   864	        [[ "$(sha256_q "$freshness_case/$marker")" == "$before" ]] \
   865	            || die "stale output rejection modified $marker"
   866	    done
   867	    rm -rf "$freshness_tmp"
   868
   869	    destination_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-destination.XXXXXX")
   870	    mkdir -p "$destination_tmp/container/src_mixed"
   871	    printf 'stale\n' > "$destination_tmp/container/src_mixed/stale"
   872	    (
   873	        rm() { return 73; }
   874	        if prepare_destination wm "$destination_tmp/container"; then
   875	            die "q destination reset masked a failed removal"
   876	        fi
   877	    )
   878	    [[ "$(< "$destination_tmp/container/src_mixed/stale")" == stale ]] \
   879	        || die "failed q destination reset modified retained evidence"
   880	    prepare_destination wm "$destination_tmp/container" \
   881	        || die "q destination reset rejected a removable tree"
   882	    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]] \
   883	        || die "q destination reset did not leave a plain directory"
   884	    [[ -z "$(find "$destination_tmp/container" -mindepth 1 -maxdepth 1 -print -quit)" ]] \
   885	        || die "q destination reset left stale content"
   886	    rm -rf "$destination_tmp"
   887
   888	    prepare_destination_source=$(declare -f prepare_destination)
   889	    python3 - "$prepare_destination_source" <<'PY' \
   890	        || die "Windows destination reset source contract changed"
   891	import sys
   892
   893	source = sys.argv[1]
   894	for marker in (
   895	    r"\$ErrorActionPreference = 'Stop'",
   896	    r"Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop",
   897	    r"Test-Path -LiteralPath '$dest' -PathType Container",
   898	    r"Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop",
   899	    'ReparsePoint',
   900	):

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/bench_otp12pf_rigw.sh | nl -ba | sed -n '901,1350p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-8nAVE94v' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GIoc1byv' (errno=Operation not permitted)
   901	    if marker not in source:
   902	        raise SystemExit(f"missing Windows destination reset marker: {marker}")
   903	windows = source.split('else', 1)[1]
   904	if 'SilentlyContinue' in windows:
   905	    raise SystemExit("Windows destination reset suppresses removal errors")
   906	PY
   907
   908	    finalize_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-finalize.XXXXXX")
   909	    (
   910	        OUT_DIR="$finalize_tmp/fails"
   911	        mkdir "$OUT_DIR"
   912	        HEAD_FULL=0123456789abcdef
   913	        LOCAL_EVIDENCE_COMPLETE=1
   914	        strict_success_cleanup() { return 1; }
   915	        if finalize_registered_session; then
   916	            die "registered finalization accepted failed strict cleanup"
   917	        fi
   918	        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] \
   919	            || die "failed strict cleanup left SESSION-COMPLETE"
   920	    )
   921	    (
   922	        OUT_DIR="$finalize_tmp/incomplete-local"
   923	        mkdir "$OUT_DIR"
   924	        HEAD_FULL=0123456789abcdef
   925	        LOCAL_EVIDENCE_COMPLETE=0
   926	        strict_success_cleanup() {
   927	            die "finalization cleaned paths before local evidence was complete"
   928	        }
   929	        if finalize_registered_session; then
   930	            die "registered finalization accepted incomplete local evidence"
   931	        fi
   932	        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]]
   933	    )
   934	    (
   935	        OUT_DIR="$finalize_tmp/succeeds"
   936	        mkdir "$OUT_DIR"
   937	        HEAD_FULL=0123456789abcdef
   938	        LOCAL_EVIDENCE_COMPLETE=1
   939	        strict_success_cleanup() {
   940	            [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] || return 1
   941	            STRICT_CLEANUP_VERIFIED=1
   942	        }
   943	        finalize_registered_session \
   944	            || die "registered finalization rejected verified strict cleanup"
   945	        [[ "$SESSION_FINALIZED" == 1 ]] \
   946	            || die "registered finalization did not set SESSION_FINALIZED"
   947	        [[ "$(< "$OUT_DIR/SESSION-COMPLETE")" == "$HEAD_FULL" ]]
   948	    )
   949
   950	    cleanup_tmp="$finalize_tmp/strict"
   951	    mkdir -p "$cleanup_tmp/q/rigw-sessions/fail-remote"
   952	    printf 'retain me\n' > "$cleanup_tmp/q/rigw-sessions/fail-remote/sentinel"
   953	    (
   954	        Q_MODULE="$cleanup_tmp/q"
   955	        SESSION_TAG=fail-remote
   956	        Q_SESSION_MAY_EXIST=1
   957	        WIN_SESSION_MAY_EXIST=1
   958	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   959	        ports_closed() { return 0; }
   960	        wssh() { return 1; }
   961	        if strict_success_cleanup; then
   962	            die "strict cleanup accepted a Windows deletion failure"
   963	        fi
   964	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
   965	            || die "Windows cleanup failure was marked strictly verified"
   966	        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
   967	            || die "Windows cleanup failure deleted q evidence first"
   968	        [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
   969	            || die "Windows cleanup failure modified q evidence"
   970	    )
   971	    mkdir -p "$cleanup_tmp/q/rigw-sessions/open-port"
   972	    (
   973	        Q_MODULE="$cleanup_tmp/q"
   974	        SESSION_TAG=open-port
   975	        Q_SESSION_MAY_EXIST=1
   976	        WIN_SESSION_MAY_EXIST=1
   977	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   978	        ports_closed() { return 1; }
   979	        wssh() { die "strict cleanup reached deletion with an open port"; }
   980	        if strict_success_cleanup; then
   981	            die "strict cleanup accepted an open port"
   982	        fi
   983	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
   984	            || die "open-port cleanup failure was marked strictly verified"
   985	        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
   986	    )
   987	    mkdir -p "$cleanup_tmp/q/rigw-sessions/surviving-q"
   988	    (
   989	        Q_MODULE="$cleanup_tmp/q"
   990	        SESSION_TAG=surviving-q
   991	        Q_SESSION_MAY_EXIST=1
   992	        WIN_SESSION_MAY_EXIST=1
   993	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   994	        ports_closed() { return 0; }
   995	        wssh() { return 0; }
   996	        rm() { return 0; }
   997	        if strict_success_cleanup; then
   998	            die "strict cleanup accepted a surviving q session tree"
   999	        fi
  1000	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
  1001	            || die "surviving q session tree was marked strictly verified"
  1002	        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
  1003	    )
  1004	    mkdir -p "$cleanup_tmp/q/rigw-sessions/succeeds"
  1005	    (
  1006	        Q_MODULE="$cleanup_tmp/q"
  1007	        SESSION_TAG=succeeds
  1008	        Q_SESSION_MAY_EXIST=1
  1009	        WIN_SESSION_MAY_EXIST=1
  1010	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
  1011	        port_checks=0
  1012	        ports_closed() { port_checks=$((port_checks + 1)); return 0; }
  1013	        wssh() { return 0; }
  1014	        strict_success_cleanup || die "strict cleanup rejected a clean session"
  1015	        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
  1016	            || die "successful strict cleanup did not set verification state"
  1017	        [[ "$port_checks" == 2 ]] || die "strict cleanup ran $port_checks port checks"
  1018	        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]] \
  1019	            || die "successful strict cleanup retained may-exist state"
  1020	        [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
  1021	    )
  1022	    mkdir -p "$cleanup_tmp/q/rigw-sessions/late-port"
  1023	    (
  1024	        Q_MODULE="$cleanup_tmp/q"
  1025	        SESSION_TAG=late-port
  1026	        Q_SESSION_MAY_EXIST=1
  1027	        WIN_SESSION_MAY_EXIST=1
  1028	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
  1029	        port_checks=0
  1030	        ports_closed() {
  1031	            port_checks=$((port_checks + 1))
  1032	            [[ "$port_checks" == 1 ]]
  1033	        }
  1034	        wssh() { return 0; }
  1035	        if strict_success_cleanup; then
  1036	            die "strict cleanup accepted a listener appearing during deletion"
  1037	        fi
  1038	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 && "$port_checks" == 2 ]]
  1039	    )
  1040	    for remembered in q daemon launcher block; do
  1041	        (
  1042	            Q_MODULE="$cleanup_tmp/q"
  1043	            SESSION_TAG="remembered-$remembered"
  1044	            q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
  1045	            case "$remembered" in
  1046	                q) q_daemon_pid=11;;
  1047	                daemon) win_daemon_pid=22;;
  1048	                launcher) win_cmd_pid=33;;
  1049	                block) current_block=4;;
  1050	            esac
  1051	            ports_closed() { die "strict cleanup ignored remembered $remembered state"; }
  1052	            if strict_success_cleanup; then
  1053	                die "strict cleanup accepted remembered $remembered state"
  1054	            fi
  1055	            [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
  1056	        )
  1057	    done
  1058	    strict_cleanup_source=$(declare -f strict_success_cleanup)
  1059	    python3 - "$strict_cleanup_source" <<'PY' \
  1060	        || die "strict cleanup source contract changed"
  1061	import sys
  1062
  1063	source = sys.argv[1]
  1064	for marker in (
  1065	    "'$WIN_MODULE/rigw-sessions/$SESSION_TAG'",
  1066	    "'$WIN_SESSION'",
  1067	    r"Remove-Item -LiteralPath \$path -Recurse -Force -ErrorAction Stop",
  1068	    r'if (Test-Path -LiteralPath \$path) { throw',
  1069	):
  1070	    if marker not in source:
  1071	        raise SystemExit(f"missing strict Windows cleanup marker: {marker}")
  1072	if source.count('ports_closed') != 2:
  1073	    raise SystemExit("strict cleanup must check closed ports before and after deletion")
  1074	if source.index('ports_closed') > source.index('Remove-Item -LiteralPath'):
  1075	    raise SystemExit("strict cleanup deletes evidence before its first port check")
  1076	if source.rindex('ports_closed') < source.index('rm -rf --'):
  1077	    raise SystemExit("strict cleanup lacks a post-deletion port check")
  1078	PY
  1079	    rm -rf "$finalize_tmp"
  1080
  1081	    failure_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-failure.XXXXXX")
  1082	    trap_calls="$failure_tmp/remote-calls"
  1083	    mkdir -p "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG"
  1084	    printf 'retain me\n' > "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel"
  1085	    set +e
  1086	    (
  1087	        set +e
  1088	        OUT_DIR="$failure_tmp/evidence"
  1089	        mkdir "$OUT_DIR"
  1090	        LOG="$OUT_DIR/bench.log"
  1091	        OUTPUT_CLAIMED=1
  1092	        printf 'primary failure\n' > "$OUT_DIR/SESSION-VOID"
  1093	        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE"
  1094	        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE.tmp"
  1095	        REGISTERED_RUN_STARTED=1
  1096	        SESSION_FINALIZED=0
  1097	        STRICT_CLEANUP_VERIFIED=0
  1098	        Q_SESSION_MAY_EXIST=1
  1099	        WIN_SESSION_MAY_EXIST=1
  1100	        Q_MODULE="$failure_tmp/q-module"
  1101	        current_block=1
  1102	        q_daemon_pid=""
  1103	        win_daemon_pid=""
  1104	        win_cmd_pid=""
  1105	        wssh() {
  1106	            printf '%s\n' "$*" >> "$trap_calls"
  1107	            return 1
  1108	        }
  1109	        false
  1110	        on_exit
  1111	    )
  1112	    trap_rc=$?
  1113	    set -e
  1114	    [[ "$trap_rc" == 1 ]] || die "failure trap returned $trap_rc, expected 1"
  1115	    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]] \
  1116	        || die "failure trap left SESSION-COMPLETE"
  1117	    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]] \
  1118	        || die "failure trap left SESSION-COMPLETE.tmp"
  1119	    grep -Fxq 'primary failure' "$failure_tmp/evidence/SESSION-VOID" \
  1120	        || die "failure trap discarded the primary reason"
  1121	    grep -Fq 'cleanup errors: Windows PID recovery failed' "$failure_tmp/evidence/SESSION-VOID" \
  1122	        || die "failure trap omitted its cleanup error"
  1123	    grep -Fq "q session evidence may remain; inspect $failure_tmp/q-module/rigw-sessions/$SESSION_TAG" \
  1124	        "$failure_tmp/evidence/SESSION-VOID" \
  1125	        || die "failure trap omitted the q evidence path"
  1126	    grep -Fq "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG" \
  1127	        "$failure_tmp/evidence/SESSION-VOID" \
  1128	        || die "failure trap omitted the Windows evidence path"
  1129	    if grep -Fq 'Remove-Item' "$trap_calls"; then
  1130	        die "failure trap issued destructive remote cleanup"
  1131	    fi
  1132	    [[ "$(< "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
  1133	        || die "failure trap modified q session evidence"
  1134	    on_exit_source=$(declare -f on_exit)
  1135	    if [[ "$on_exit_source" == *'rm -rf'* \
  1136	        || "$on_exit_source" == *'Remove-Item'* \
  1137	        || "$on_exit_source" == *'strict_success_cleanup'* ]]; then
  1138	        die "failure trap contains a destructive session-cleanup path"
  1139	    fi
  1140
  1141	    append_tmp="$failure_tmp/append-contract"
  1142	    mkdir "$append_tmp"
  1143	    printf 'original reason\n' > "$append_tmp/SESSION-VOID"
  1144	    set +e
  1145	    (
  1146	        OUT_DIR="$append_tmp"
  1147	        LOG="$OUT_DIR/bench.log"
  1148	        OUTPUT_CLAIMED=1
  1149	        session_void 'later context'
  1150	    ) >/dev/null 2>&1
  1151	    trap_rc=$?
  1152	    set -e
  1153	    [[ "$trap_rc" == 1 ]] || die "session_void append probe returned $trap_rc"
  1154	    [[ "$(< "$append_tmp/SESSION-VOID")" == $'original reason\nlater context' ]] \
  1155	        || die "session_void overwrote an earlier failure reason"
  1156
  1157	    contract_tmp="$failure_tmp/exit-contract"
  1158	    mkdir "$contract_tmp"
  1159	    set +e
  1160	    (
  1161	        set +e
  1162	        OUT_DIR="$contract_tmp"
  1163	        LOG="$OUT_DIR/bench.log"
  1164	        OUTPUT_CLAIMED=1
  1165	        REGISTERED_RUN_STARTED=1
  1166	        SESSION_FINALIZED=0
  1167	        STRICT_CLEANUP_VERIFIED=0
  1168	        WIN_SESSION_MAY_EXIST=0
  1169	        true
  1170	        on_exit
  1171	    )
  1172	    trap_rc=$?
  1173	    set -e
  1174	    [[ "$trap_rc" == 1 ]] \
  1175	        || die "unfinalized registered zero-exit returned $trap_rc"
  1176	    grep -Fq 'registered run returned without finalizing the session' \
  1177	        "$contract_tmp/SESSION-VOID" \
  1178	        || die "unfinalized registered zero-exit omitted its reason"
  1179
  1180	    contract_tmp="$failure_tmp/marker-contract"
  1181	    mkdir "$contract_tmp"
  1182	    set +e
  1183	    (
  1184	        set +e
  1185	        OUT_DIR="$contract_tmp"
  1186	        LOG="$OUT_DIR/bench.log"
  1187	        OUTPUT_CLAIMED=1
  1188	        REGISTERED_RUN_STARTED=1
  1189	        SESSION_FINALIZED=1
  1190	        STRICT_CLEANUP_VERIFIED=1
  1191	        LOCAL_EVIDENCE_COMPLETE=1
  1192	        HEAD_FULL=0123456789abcdef
  1193	        true
  1194	        on_exit
  1195	    )
  1196	    trap_rc=$?
  1197	    set -e
  1198	    [[ "$trap_rc" == 1 ]] \
  1199	        || die "finalized flags without a completion marker returned $trap_rc"
  1200	    grep -Fq 'registered completion marker is absent or invalid' \
  1201	        "$contract_tmp/SESSION-VOID" \
  1202	        || die "missing registered completion marker omitted its reason"
  1203
  1204	    contract_tmp="$failure_tmp/wrong-marker-contract"
  1205	    mkdir "$contract_tmp"
  1206	    printf 'wrong-build\n' > "$contract_tmp/SESSION-COMPLETE"
  1207	    set +e
  1208	    (
  1209	        set +e
  1210	        OUT_DIR="$contract_tmp"
  1211	        LOG="$OUT_DIR/bench.log"
  1212	        OUTPUT_CLAIMED=1
  1213	        REGISTERED_RUN_STARTED=1
  1214	        SESSION_FINALIZED=1
  1215	        STRICT_CLEANUP_VERIFIED=1
  1216	        LOCAL_EVIDENCE_COMPLETE=1
  1217	        HEAD_FULL=0123456789abcdef
  1218	        true
  1219	        on_exit
  1220	    )
  1221	    trap_rc=$?
  1222	    set -e
  1223	    [[ "$trap_rc" == 1 ]] \
  1224	        || die "wrong completion marker returned $trap_rc"
  1225	    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
  1226	        || die "wrong completion marker survived failure handling"
  1227
  1228	    contract_tmp="$failure_tmp/preflight-contract"
  1229	    mkdir "$contract_tmp"
  1230	    set +e
  1231	    (
  1232	        set +e
  1233	        OUT_DIR="$contract_tmp"
  1234	        LOG="$OUT_DIR/bench.log"
  1235	        OUTPUT_CLAIMED=1
  1236	        REGISTERED_RUN_STARTED=0
  1237	        SESSION_FINALIZED=0
  1238	        STRICT_CLEANUP_VERIFIED=0
  1239	        true
  1240	        on_exit
  1241	    )
  1242	    trap_rc=$?
  1243	    set -e
  1244	    [[ "$trap_rc" == 1 ]] \
  1245	        || die "unclean preflight zero-exit returned $trap_rc"
  1246	    grep -Fq 'successful exit lacked verified strict cleanup' \
  1247	        "$contract_tmp/SESSION-VOID" \
  1248	        || die "unclean preflight zero-exit omitted its reason"
  1249
  1250	    contract_tmp="$failure_tmp/preflight-marker-contract"
  1251	    mkdir "$contract_tmp"
  1252	    printf 'not allowed\n' > "$contract_tmp/SESSION-COMPLETE"
  1253	    set +e
  1254	    (
  1255	        set +e
  1256	        OUT_DIR="$contract_tmp"
  1257	        LOG="$OUT_DIR/bench.log"
  1258	        OUTPUT_CLAIMED=1
  1259	        REGISTERED_RUN_STARTED=0
  1260	        SESSION_FINALIZED=0
  1261	        STRICT_CLEANUP_VERIFIED=1
  1262	        true
  1263	        on_exit
  1264	    )
  1265	    trap_rc=$?
  1266	    set -e
  1267	    [[ "$trap_rc" == 1 ]] \
  1268	        || die "preflight completion marker returned $trap_rc"
  1269	    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
  1270	        || die "preflight completion marker survived failure handling"
  1271
  1272	    for marker in SESSION-VOID SESSION-COMPLETE.tmp; do
  1273	        contract_tmp="$failure_tmp/preflight-$marker-contract"
  1274	        mkdir "$contract_tmp"
  1275	        printf 'not allowed\n' > "$contract_tmp/$marker"
  1276	        set +e
  1277	        (
  1278	            set +e
  1279	            OUT_DIR="$contract_tmp"
  1280	            LOG="$OUT_DIR/bench.log"
  1281	            OUTPUT_CLAIMED=1
  1282	            REGISTERED_RUN_STARTED=0
  1283	            SESSION_FINALIZED=0
  1284	            STRICT_CLEANUP_VERIFIED=1
  1285	            true
  1286	            on_exit
  1287	        )
  1288	        trap_rc=$?
  1289	        set -e
  1290	        [[ "$trap_rc" == 1 ]] \
  1291	            || die "preflight $marker returned $trap_rc"
  1292	        if [[ "$marker" == SESSION-VOID ]]; then
  1293	            [[ "$(sed -n '1p' "$contract_tmp/SESSION-VOID")" == 'not allowed' ]] \
  1294	                || die "preflight VOID rejection replaced its primary reason"
  1295	        else
  1296	            grep -Fq 'successful exit retained a failure or temporary marker' \
  1297	                "$contract_tmp/SESSION-VOID" \
  1298	                || die "preflight $marker omitted its rejection reason"
  1299	        fi
  1300	    done
  1301
  1302	    for signal in HUP INT TERM; do
  1303	        signal_dir="$failure_tmp/signal-$signal"
  1304	        mkdir "$signal_dir"
  1305	        set +e
  1306	        bash -c '
  1307	set -Eeuo pipefail
  1308	source "$1"
  1309	OUT_DIR="$2"
  1310	LOG="$OUT_DIR/bench.log"
  1311	OUTPUT_CLAIMED=1
  1312	REGISTERED_RUN_STARTED=1
  1313	SESSION_FINALIZED=0
  1314	STRICT_CLEANUP_VERIFIED=0
  1315	Q_SESSION_MAY_EXIST=1
  1316	WIN_SESSION_MAY_EXIST=1
  1317	current_block=1
  1318	q_daemon_pid=111
  1319	win_daemon_pid=222
  1320	win_cmd_pid=333
  1321	win_daemon_stop() {
  1322	    printf "windows\n" >> "$OUT_DIR/stops"
  1323	    win_daemon_pid=""; win_cmd_pid=""; current_block=""
  1324	}
  1325	q_daemon_stop() {
  1326	    printf "q\n" >> "$OUT_DIR/stops"
  1327	    q_daemon_pid=""
  1328	}
  1329	printf "must disappear\n" > "$OUT_DIR/SESSION-COMPLETE"
  1330	trap on_exit EXIT
  1331	install_signal_traps
  1332	kill -s "$3" "$$"
  1333	sleep 2
  1334	exit 99
  1335	' _ "$SCRIPT_DIR/bench_otp12pf_rigw.sh" "$signal_dir" "$signal"
  1336	        signal_rc=$?
  1337	        set -e
  1338	        [[ "$signal_rc" == 1 ]] \
  1339	            || die "$signal cleanup returned $signal_rc, expected 1"
  1340	        grep -Fxq "received $signal" "$signal_dir/SESSION-VOID" \
  1341	            || die "$signal cleanup omitted its signal reason"
  1342	        [[ "$(LC_ALL=C sort "$signal_dir/stops")" == $'q\nwindows' ]] \
  1343	            || die "$signal cleanup did not invoke both exact-owned teardown paths"
  1344	        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
  1345	            || die "$signal cleanup left SESSION-COMPLETE"
  1346	    done
  1347	    rm -rf "$failure_tmp"
  1348
  1349	    analyzer_log=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-analyzer.XXXXXX")
  1350	    if ! python3 "$SCRIPT_DIR/otp12pf_rigw_analyze_test.py" \

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/bench_otp12pf_rigw.sh | nl -ba | sed -n '1351,1800p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HhZuOBkh' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nN6asova' (errno=Operation not permitted)
  1351	        > "$analyzer_log" 2>&1; then
  1352	        cat "$analyzer_log" >&2
  1353	        rm -f "$analyzer_log"
  1354	        die "analyzer self-tests failed"
  1355	    fi
  1356	    rm -f "$analyzer_log"
  1357	    log "SELFTEST OK: exact four-block/128-arm schedule and analyzer guards"
  1358	}
  1359
  1360	sha256_q() { shasum -a 256 "$1" | awk '{print $1}'; }
  1361	sha256_win() {
  1362	    wssh "(Get-FileHash -Algorithm SHA256 -LiteralPath '$1').Hash.ToLower()" \
  1363	        | tr -d '\r' | tail -1
  1364	}
  1365
  1366	stage_purge_helper() {
  1367	    local staged_tmp="$WIN_PURGE.tmp" remote_hash
  1368	    [[ -f "$WIN_PURGE_SOURCE" && ! -L "$WIN_PURGE_SOURCE" ]] \
  1369	        || die "reviewed Windows purge helper is absent or not a plain file"
  1370	    WIN_PURGE_HASH=$(sha256_q "$WIN_PURGE_SOURCE") \
  1371	        || die "cannot hash reviewed Windows purge helper"
  1372	    [[ "$WIN_PURGE_HASH" =~ ^[0-9a-f]{64}$ ]] \
  1373	        || die "reviewed Windows purge helper hash is malformed: $WIN_PURGE_HASH"
  1374
  1375	    WIN_SESSION_MAY_EXIST=1
  1376	    wssh "
  1377	\$ErrorActionPreference = 'Stop'
  1378	if (Test-Path -LiteralPath '$WIN_SESSION') { throw 'refusing existing Windows session tree' }
  1379	New-Item -ItemType Directory -Path '$WIN_SESSION' -ErrorAction Stop | Out-Null
  1380	" || die "cannot reserve fresh Windows session tree for reviewed purge helper"
  1381	    wscp "$WIN_PURGE_SOURCE" "$WIN_SSH:$staged_tmp" \
  1382	        || die "cannot stage reviewed Windows purge helper"
  1383	    remote_hash=$(wssh "
  1384	\$ErrorActionPreference = 'Stop'
  1385	\$tmpItem = Get-Item -LiteralPath '$staged_tmp' -Force -ErrorAction Stop
  1386	if ((\$tmpItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'staged purge helper is a reparse point' }
  1387	\$tmpHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$staged_tmp').Hash.ToLower()
  1388	if (\$tmpHash -cne '$WIN_PURGE_HASH') { throw \"staged purge helper hash mismatch: \$tmpHash\" }
  1389	Move-Item -LiteralPath '$staged_tmp' -Destination '$WIN_PURGE' -ErrorAction Stop
  1390	\$finalItem = Get-Item -LiteralPath '$WIN_PURGE' -Force -ErrorAction Stop
  1391	if ((\$finalItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'final purge helper is a reparse point' }
  1392	\$finalHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE').Hash.ToLower()
  1393	if (\$finalHash -cne '$WIN_PURGE_HASH') { throw \"final purge helper hash mismatch: \$finalHash\" }
  1394	\"H|\$finalHash\"
  1395	" | tr -d '\r') || die "cannot verify reviewed Windows purge helper"
  1396	    [[ "$remote_hash" == "H|$WIN_PURGE_HASH" ]] \
  1397	        || die "Windows purge helper verification returned '$remote_hash'"
  1398	}
  1399
  1400	float_le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }
  1401
  1402	q_load1() {
  1403	    /usr/sbin/sysctl -n vm.loadavg | awk '{gsub(/[{}]/, ""); print $1}'
  1404	}
  1405
  1406	q_spotlight_cpu() {
  1407	    ps -axo %cpu=,comm= | awk '
  1408	        $2 ~ /(mds|mds_stores|mdworker|mdbulkimport)$/ { sum += $1 }
  1409	        END { printf "%.1f\n", sum + 0 }'
  1410	}
  1411
  1412	q_time_machine_gate() {
  1413	    local auto status
  1414	    auto=$(/usr/bin/defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null) \
  1415	        || die "q Time Machine AutoBackup setting is unreadable"
  1416	    [[ "$auto" == 0 ]] \
  1417	        || die "q Time Machine AutoBackup is enabled ($auto); do not mutate it from the harness"
  1418	    status=$(/usr/bin/tmutil status) || die "q Time Machine status is unreadable"
  1419	    grep -q 'Running = 0;' <<<"$status" \
  1420	        || die "q Time Machine is running"
  1421	}
  1422
  1423	q_quiet_gate() {
  1424	    local offenders load spot
  1425	    offenders=$(ps -axo pid=,comm= | awk -v owned="${q_daemon_pid:-}" '
  1426	        {
  1427	          n=$2; sub(/^.*\//, "", n)
  1428	          if ($1 != owned && (n == "cargo" || n == "rustc" || n == "blit-daemon" || n ~ /^codex($|-)/))
  1429	            print $1 ":" n
  1430	        }')
  1431	    [[ -z "$offenders" ]] || die "q has benchmark-conflicting processes: $offenders"
  1432	    q_time_machine_gate
  1433	    load=$(q_load1)
  1434	    [[ "$load" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse q load1 '$load'"
  1435	    float_le "$load" "$LOAD1_MAX" || die "q load1 $load exceeds $LOAD1_MAX"
  1436	    spot=$(q_spotlight_cpu)
  1437	    [[ "$spot" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Spotlight CPU '$spot'"
  1438	    float_le "$spot" "$SPOTLIGHT_CPU_MAX" \
  1439	        || die "q Spotlight CPU $spot% exceeds $SPOTLIGHT_CPU_MAX%"
  1440	    log "quiet q: load1=$load Spotlight=${spot}% TimeMachine=disabled/stopped"
  1441	}
  1442
  1443	win_quiet_gate() {
  1444	    local out avg
  1445	    out=$(wssh '
  1446	$ErrorActionPreference = "Stop"
  1447	$bad = Get-Process cargo,rustc,blit-daemon -ErrorAction SilentlyContinue
  1448	if ($bad) { "BAD|" + (($bad | ForEach-Object { "$($_.Id):$($_.ProcessName)" }) -join ","); exit 7 }
  1449	$samples = 1..3 | ForEach-Object {
  1450	  $v = (Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average
  1451	  Start-Sleep -Seconds 1
  1452	  [double]$v
  1453	}
  1454	"CPU|$([math]::Round(($samples | Measure-Object -Average).Average,1))"
  1455	') || die "Windows quiet probe failed: $out"
  1456	    out=${out//$'\r'/}
  1457	    [[ "$out" != *BAD\|* ]] || die "Windows has benchmark-conflicting processes: $out"
  1458	    avg=$(sed -n 's/^CPU|//p' <<<"$out" | tail -1)
  1459	    [[ "$avg" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Windows CPU from '$out'"
  1460	    float_le "$avg" "$WIN_CPU_MAX" || die "Windows CPU ${avg}% exceeds ${WIN_CPU_MAX}%"
  1461	    log "quiet Windows: CPU=${avg}% and no cargo/rustc/blit-daemon"
  1462	}
  1463
  1464	q_topology_gate() {
  1465	    local raw route arp mtu media status iface route_mtu peer_mac
  1466	    [[ "$(hostname)" == "$Q_EXPECT_HOST" ]] \
  1467	        || die "this harness must execute on $Q_EXPECT_HOST, got $(hostname)"
  1468	    raw=$(/sbin/ifconfig "$Q_NIC") || die "cannot read q $Q_NIC"
  1469	    mtu=$(sed -n 's/.*[[:space:]]mtu[[:space:]]\([0-9][0-9]*\).*/\1/p' <<<"$raw" | head -1)
  1470	    media=$(sed -n 's/^[[:space:]]*media:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
  1471	    status=$(sed -n 's/^[[:space:]]*status:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
  1472	    grep -q "inet $Q_IP " <<<"$raw" || die "$Q_NIC does not own $Q_IP"
  1473	    grep -qi "ether $Q_MAC" <<<"$raw" || die "$Q_NIC MAC is not $Q_MAC"
  1474	    [[ "$mtu" == "$REGISTERED_MTU" ]] || die "$Q_NIC MTU is $mtu, expected $REGISTERED_MTU"
  1475	    [[ "$media" == *"$REGISTERED_MEDIA"* ]] || die "$Q_NIC media is '$media', expected $REGISTERED_MEDIA"
  1476	    [[ "$status" == active ]] || die "$Q_NIC status is '$status'"
  1477
  1478	    route=$(/sbin/route -n get "$WIN_IP") || die "q route probe failed"
  1479	    iface=$(awk '/interface:/ {print $2; exit}' <<<"$route")
  1480	    route_mtu=$(awk '/mtu/ {getline; print $(NF-1); exit}' <<<"$route")
  1481	    [[ "$iface" == "$Q_NIC" ]] || die "q routes $WIN_IP via $iface, expected $Q_NIC"
  1482	    [[ "$route_mtu" == "$REGISTERED_MTU" ]] \
  1483	        || die "q route to $WIN_IP reports MTU $route_mtu, expected $REGISTERED_MTU"
  1484	    /sbin/ping -c 1 -W 1000 "$WIN_IP" >/dev/null || die "q cannot ping $WIN_IP"
  1485	    arp=$(/usr/sbin/arp -n "$WIN_IP") || die "q ARP probe failed"
  1486	    peer_mac=$(q_peer_mac_from_arp "$Q_NIC" <<<"$arp")
  1487	    [[ -n "$peer_mac" && "$peer_mac" != *$'\n'* ]] \
  1488	        || die "q ARP for $WIN_IP did not yield exactly one $Q_NIC entry: $peer_mac"
  1489	    [[ "$peer_mac" == "$(tr 'A-F' 'a-f' <<<"${WIN_MAC//-/:}")" ]] \
  1490	        || die "q ARP for $WIN_IP is $peer_mac, expected peer ${WIN_MAC//-/:}"
  1491	    [[ "$peer_mac" != "$Q_MAC" ]] || die "q ARP points at q's own MAC (black-hole host route)"
  1492	    log "fabric q: $Q_NIC $Q_IP mtu=$mtu media=$media route=$iface peer=$peer_mac"
  1493	}
  1494
  1495	q_peer_mac_from_arp() {
  1496	    local nic="$1"
  1497	    awk -v nic="$nic" \
  1498	        '$3 == "at" && $5 == "on" && $6 == nic { print tolower($4) }'
  1499	}
  1500
  1501	win_topology_gate() {
  1502	    local out
  1503	    out=$(wssh "
  1504	\$ErrorActionPreference = 'Stop'
  1505	\$a = Get-NetAdapter -Name '$WIN_NIC'
  1506	\$ip = Get-NetIPAddress -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4 | Where-Object IPAddress -eq '$WIN_IP'
  1507	\$ni = Get-NetIPInterface -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4
  1508	\$route = Find-NetRoute -RemoteIPAddress '$Q_IP' | Select-Object -First 1
  1509	if (-not \$ip) { throw 'registered IPv4 address absent' }
  1510	\"W|\$(\$a.Status)|\$(\$a.LinkSpeed)|\$(\$a.ReceiveLinkSpeed)|\$(\$a.TransmitLinkSpeed)|\$(\$a.MacAddress)|\$(\$ni.ConnectionState)|\$(\$ni.NlMtu)|\$(\$route.InterfaceAlias)|\$(\$route.IPAddress)\"
  1511	") || die "Windows topology probe failed: $out"
  1512	    out=${out//$'\r'/}
  1513	    [[ "$out" == "W|Up|10 Gbps|10000000000|10000000000|$WIN_MAC|Connected|$REGISTERED_MTU|$WIN_NIC|$WIN_IP" ]] \
  1514	        || die "Windows topology mismatch: '$out'"
  1515	    log "fabric Windows: $WIN_NIC $WIN_IP mtu=$REGISTERED_MTU link=10Gbps route/source pinned"
  1516	}
  1517
  1518	q_to_win_mss() {
  1519	    python3 - "$WIN_IP" <<'PY'
  1520	import socket, sys
  1521	s = socket.create_connection((sys.argv[1], 22), timeout=5)
  1522	print(f"{s.getsockopt(socket.IPPROTO_TCP, socket.TCP_MAXSEG)} {s.getsockname()[0]}")
  1523	s.close()
  1524	PY
  1525	}
  1526
  1527	win_to_q_mss() {
  1528	    wssh "
  1529	\$ErrorActionPreference = 'Stop'
  1530	\$s = [Net.Sockets.Socket]::new([Net.Sockets.AddressFamily]::InterNetwork,[Net.Sockets.SocketType]::Stream,[Net.Sockets.ProtocolType]::Tcp)
  1531	\$s.Connect('$Q_IP',22)
  1532	\$name = [Net.Sockets.SocketOptionName]4
  1533	\$b = \$s.GetSocketOption([Net.Sockets.SocketOptionLevel]::Tcp,\$name,4)
  1534	\$m = [BitConverter]::ToInt32(\$b,0)
  1535	\"M|\${m}|\$(\$s.LocalEndPoint.Address)\"
  1536	\$s.Dispose()
  1537	" | tr -d '\r' | tail -1
  1538	}
  1539
  1540	mss_gate() {
  1541	    local qout wout qm qip wm wip
  1542	    qout=$(q_to_win_mss) || die "q→Windows MSS probe failed"
  1543	    read -r qm qip <<<"$qout"
  1544	    [[ "$qm" == "$Q_TO_WIN_MSS" && "$qip" == "$Q_IP" ]] \
  1545	        || die "q→Windows MSS/source is '$qout', expected $Q_TO_WIN_MSS $Q_IP"
  1546	    wout=$(win_to_q_mss) || die "Windows→q MSS probe failed"
  1547	    IFS='|' read -r _ wm wip <<<"$wout"
  1548	    [[ "$wm" == "$WIN_TO_Q_MSS" && "$wip" == "$WIN_IP" ]] \
  1549	        || die "Windows→q MSS/source is '$wout', expected M|$WIN_TO_Q_MSS|$WIN_IP"
  1550	    log "path MSS: q→Windows=$qm via $qip; Windows→q=$wm via $wip"
  1551	}
  1552
  1553	firewall_gate() {
  1554	    local out
  1555	    out=$(wssh "
  1556	\$r = Get-NetFirewallRule -DisplayName 'blit-otp12-daemon' -ErrorAction SilentlyContinue
  1557	if (-not \$r) { exit 4 }
  1558	\$app = \$r | Get-NetFirewallApplicationFilter
  1559	\"F|\$(\$r.Enabled)|\$(\$r.Action)|\$(\$r.Direction)|\$(\$app.Program)\"
  1560	") || die "existing Windows firewall rule is absent/unreadable; harness will not create it"
  1561	    out=${out//$'\r'/}
  1562	    out=$(sed 's#\\#/#g' <<<"$out")
  1563	    out=$(tr 'A-Z' 'a-z' <<<"$out")
  1564	    local expected
  1565	    expected=$(tr 'A-Z' 'a-z' <<<"F|True|Allow|Inbound|$WIN_ACTIVE")
  1566	    [[ "$out" == "$expected" ]] \
  1567	        || die "Windows firewall rule mismatch: '$out'"
  1568	    log "firewall verified only: existing inbound allow is scoped to $WIN_ACTIVE"
  1569	}
  1570
  1571	ports_closed() {
  1572	    if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  1573	        return 1
  1574	    fi
  1575	    wssh "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
  1576	        >/dev/null 2>&1
  1577	}
  1578
  1579	timer_gate() {
  1580	    local qms wout wms
  1581	    qms=$(python3 - <<'PY'
  1582	import time
  1583	clock_ns=lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
  1584	t=clock_ns(); time.sleep(1); print(round((clock_ns()-t)/1_000_000))
  1585	PY
  1586	)
  1587	    [[ "$qms" -ge 950 && "$qms" -le 1050 ]] || die "q one-second timer calibrated to ${qms}ms"
  1588	    wout=$(wssh '$s=[Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Seconds 1; $s.Stop(); "T|$([int]$s.Elapsed.TotalMilliseconds)"') \
  1589	        || die "Windows timer calibration failed"
  1590	    wout=${wout//$'\r'/}; wms=${wout##*|}
  1591	    [[ "$wms" -ge 950 && "$wms" -le 1050 ]] || die "Windows one-second timer calibrated to ${wms}ms"
  1592	    log "timer calibration: q=${qms}ms Windows=${wms}ms"
  1593	}
  1594
  1595	windows_result_stream_gate() {
  1596	    local before after result tag ms rc stamp extra teardown_ns
  1597	    before=$(q_monotonic_ns)
  1598	    result=$(wssh \
  1599	        '[Console]::Out.WriteLine("R|17|0"); [Console]::Out.Flush(); Start-Sleep -Milliseconds 350' \
  1600	        | stamp_result_arrival_on_q) \
  1601	        || die "Windows result-stream probe failed"
  1602	    after=$(q_monotonic_ns)
  1603	    IFS='|' read -r tag ms rc stamp extra <<<"$result"
  1604	    [[ "$tag" == R && "$ms" == 17 && "$rc" == 0 \
  1605	        && "$stamp" =~ ^[0-9]+$ && -z "$extra" ]] \
  1606	        || die "Windows result-stream probe returned '$result'"
  1607	    [[ "$stamp" -ge "$before" && "$stamp" -le "$after" ]] \
  1608	        || die "Windows result-stream q stamp is outside the probe lifetime"
  1609	    teardown_ns=$((after - stamp))
  1610	    [[ "$teardown_ns" -ge 250000000 ]] \
  1611	        || die "Windows result stream was not observable before remote teardown"
  1612	    log "Windows result stream reaches q before remote teardown"
  1613	}
  1614
  1615	fixture_shape_q() {
  1616	    python3 - "$1" <<'PY'
  1617	import os, sys
  1618	n=b=0
  1619	for root, dirs, files in os.walk(sys.argv[1]):
  1620	    for name in files:
  1621	        p=os.path.join(root,name); n+=1; b+=os.path.getsize(p)
  1622	print(f"{n},{b}")
  1623	PY
  1624	}
  1625
  1626	fixture_shape_win() {
  1627	    wssh "
  1628	\$f = Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
  1629	\"S|\$(\$f.Count)|\$(if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 })\"
  1630	" | tr -d '\r' | sed -n 's/^S|//p' | tr '|' ',' | tail -1
  1631	}
  1632
  1633	write_q_tree_manifest() {
  1634	    python3 - "$1" "$2" "${3:-}" <<'PY'
  1635	import base64, os, pathlib, stat, sys
  1636
  1637	root = pathlib.Path(sys.argv[1])
  1638	output = pathlib.Path(sys.argv[2])
  1639	expected_root = sys.argv[3]
  1640	if not root.is_dir() or root.is_symlink():
  1641	    raise SystemExit(f"manifest root is not a plain directory: {root}")
  1642	if expected_root:
  1643	    entries = list(root.iterdir())
  1644	    if (
  1645	        len(entries) != 1
  1646	        or entries[0].name != expected_root
  1647	        or not entries[0].is_dir()
  1648	        or entries[0].is_symlink()
  1649	    ):
  1650	        raise SystemExit(
  1651	            f"landed container must contain exactly plain directory {expected_root}"
  1652	        )
  1653	    root = entries[0]
  1654
  1655	lines = []
  1656	def walk_error(error):
  1657	    raise error
  1658
  1659	for current, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
  1660	    for name in dirs:
  1661	        path = pathlib.Path(current, name)
  1662	        mode = path.lstat().st_mode
  1663	        if not stat.S_ISDIR(mode) or stat.S_ISLNK(mode):
  1664	            raise SystemExit(f"non-directory/reparse entry in manifest: {path}")
  1665	    for name in files:
  1666	        path = pathlib.Path(current, name)
  1667	        info = path.lstat()
  1668	        if not stat.S_ISREG(info.st_mode):
  1669	            raise SystemExit(f"non-regular entry in manifest: {path}")
  1670	        relative = path.relative_to(root).as_posix()
  1671	        encoded = base64.b64encode(relative.encode("utf-8")).decode("ascii")
  1672	        lines.append(f"{encoded},{info.st_size}")
  1673	lines.sort()
  1674	output.write_text("".join(f"{line}\n" for line in lines), encoding="ascii")
  1675	PY
  1676	}
  1677
  1678	write_win_tree_manifest() {
  1679	    local root="$1" remote_out="$2" local_out="$3" expected_root="${4:-}"
  1680	    wssh "
  1681	\$ErrorActionPreference = 'Stop'
  1682	\$root = (Resolve-Path -LiteralPath '$root').Path.TrimEnd([char]92,[char]47)
  1683	if ('$expected_root') {
  1684	  \$entries = @(Get-ChildItem -LiteralPath \$root -Force -ErrorAction Stop)
  1685	  if (\$entries.Count -ne 1 -or -not \$entries[0].PSIsContainer -or \$entries[0].Name -cne '$expected_root' -or ((\$entries[0].Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'landed root layout mismatch' }
  1686	  \$root = \$entries[0].FullName.TrimEnd([char]92,[char]47)
  1687	}
  1688	\$lines = [Collections.Generic.List[string]]::new()
  1689	foreach (\$item in @(Get-ChildItem -LiteralPath \$root -Recurse -Force -ErrorAction Stop)) {
  1690	  if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw \"reparse entry in manifest: \$(\$item.FullName)\" }
  1691	  if (\$item.PSIsContainer) { continue }
  1692	  if (-not (\$item -is [IO.FileInfo])) { throw \"non-regular entry in manifest: \$(\$item.FullName)\" }
  1693	  \$rel = \$item.FullName.Substring(\$root.Length).TrimStart([char]92,[char]47).Replace([char]92,[char]47)
  1694	  \$b64 = [Convert]::ToBase64String([Text.UTF8Encoding]::new(\$false,\$true).GetBytes(\$rel))
  1695	  \$lines.Add(\"\$b64,\$([uint64]\$item.Length)\")
  1696	}
  1697	\$ordered = [string[]]\$lines.ToArray()
  1698	[Array]::Sort(\$ordered, [StringComparer]::Ordinal)
  1699	\$text = if (\$ordered.Count) { (\$ordered -join \"`n\") + \"`n\" } else { '' }
  1700	[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName('$remote_out')) | Out-Null
  1701	[IO.File]::WriteAllText('$remote_out', \$text, [Text.UTF8Encoding]::new(\$false))
  1702	" || return 1
  1703	    fetch_win_file "$remote_out" "$local_out" || return 1
  1704	    LC_ALL=C sort -o "$local_out" "$local_out"
  1705	}
  1706
  1707	matching_manifest_digest() {
  1708	    local canonical="$1" landed="$2"
  1709	    cmp -s "$canonical" "$landed" || return 1
  1710	    sha256_q "$landed"
  1711	}
  1712
  1713	verify_fixtures() {
  1714	    local shape want qgot wgot qmanifest wmanifest qhash
  1715	    printf '%s\n' 'shape,sha256,q_manifest,windows_manifest' \
  1716	        > "$OUT_DIR/fixture-manifests.csv"
  1717	    WIN_SESSION_MAY_EXIST=1
  1718	    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION/fixtures' | Out-Null" \
  1719	        || die "cannot create Windows fixture evidence directory"
  1720	    for shape in mixed large; do
  1721	        case "$shape" in
  1722	            mixed) want=5001,547110912;;
  1723	            large) want=1,1073741824;;
  1724	        esac
  1725	        qgot=$(fixture_shape_q "$(q_source_path "$shape")")
  1726	        wgot=$(fixture_shape_win "$(win_source_path "$shape")")
  1727	        [[ "$qgot" == "$want" ]] || die "q src_$shape is $qgot, expected $want"
  1728	        [[ "$wgot" == "$want" ]] || die "Windows canonical src_$shape is $wgot, expected $want"
  1729	        qmanifest="$OUT_DIR/fixtures/src_$shape.manifest"
  1730	        wmanifest="$OUT_DIR/fixtures/windows-src_$shape.manifest"
  1731	        write_q_tree_manifest "$(q_source_path "$shape")" "$qmanifest" \
  1732	            || die "q src_$shape manifest failed"
  1733	        write_win_tree_manifest \
  1734	            "$(win_source_path "$shape")" \
  1735	            "$WIN_SESSION/fixtures/src_$shape.manifest" "$wmanifest" \
  1736	            || die "Windows src_$shape manifest failed"
  1737	        qhash=$(matching_manifest_digest "$qmanifest" "$wmanifest") \
  1738	            || die "q and Windows src_$shape relative-path/size manifests differ"
  1739	        printf '%s,%s,%s,%s\n' \
  1740	            "$shape" "$qhash" "fixtures/src_$shape.manifest" \
  1741	            "fixtures/windows-src_$shape.manifest" \
  1742	            >> "$OUT_DIR/fixture-manifests.csv"
  1743	    done
  1744	    log "canonical fixtures verified byte-for-byte by relative path and size on both hosts"
  1745	}
  1746
  1747	write_manifest() {
  1748	    local qbh qdh wbh wdh
  1749	    qbh=$(sha256_q "$Q_BLIT"); qdh=$(sha256_q "$Q_DAEMON")
  1750	    wbh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit.exe")
  1751	    wdh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit-daemon.exe")
  1752	    cat > "$OUT_DIR/staging-manifest.csv" <<EOF
  1753	host,role,commit,sha256,path
  1754	q,client,$HEAD_FULL,$qbh,$Q_BLIT
  1755	q,daemon,$HEAD_FULL,$qdh,$Q_DAEMON
  1756	windows,client,$HEAD_FULL,$wbh,$WIN_BINS/$HEAD_SHORT/blit.exe
  1757	windows,daemon,$HEAD_FULL,$wdh,$WIN_BINS/$HEAD_SHORT/blit-daemon.exe
  1758	windows,cache-helper,$HEAD_FULL,$WIN_PURGE_HASH,$WIN_PURGE
  1759	EOF
  1760	    WIN_DAEMON_HASH=$wdh
  1761	}
  1762
  1763	provenance_gate() {
  1764	    [[ -n "$EXPECT_SHA" ]] || die "EXPECT_SHA=<full reviewed commit> is required"
  1765	    HEAD_FULL=$(git -C "$REPO_ROOT" rev-parse HEAD)
  1766	    HEAD_SHORT=$(git -C "$REPO_ROOT" rev-parse --short=7 HEAD)
  1767	    HEAD_BUILD_ID=$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD)
  1768	    [[ "$EXPECT_SHA" == "$HEAD_FULL" ]] \
  1769	        || die "EXPECT_SHA=$EXPECT_SHA but isolated clone is $HEAD_FULL"
  1770	    [[ -z $(git -C "$REPO_ROOT" status --porcelain --untracked-files=normal) ]] \
  1771	        || die "isolated q clone is dirty"
  1772	    [[ -x "$Q_BLIT" && -x "$Q_DAEMON" ]] || die "q release binaries are absent"
  1773	    embeds_clean_q "$Q_BLIT" \
  1774	        || die "q client does not embed a clean +$HEAD_BUILD_ID"
  1775	    embeds_clean_q "$Q_DAEMON" \
  1776	        || die "q daemon does not embed a clean +$HEAD_BUILD_ID"
  1777	    wssh "
  1778	if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe')) { exit 2 }
  1779	if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe')) { exit 3 }
  1780	if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 4 }
  1781	if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 5 }
  1782	if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 6 }
  1783	if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 7 }
  1784	" || die "Windows binaries are missing or do not embed a clean +$HEAD_BUILD_ID"
  1785	}
  1786
  1787	preflight() {
  1788	    reject_registered_overrides
  1789	    command -v python3 >/dev/null || die "python3 required"
  1790	    command -v lsof >/dev/null || die "lsof required"
  1791	    command -v nc >/dev/null || die "nc required"
  1792	    command -v scp >/dev/null || die "scp required"
  1793	    sudo -n /usr/sbin/purge >/dev/null || die "q NOPASSWD purge grant is absent"
  1794	    provenance_gate
  1795	    ports_closed || die "port $PORT already has a listener on q or Windows"
  1796	    q_topology_gate
  1797	    win_topology_gate
  1798	    mss_gate
  1799	    firewall_gate
  1800	    q_quiet_gate

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/bench_otp12pf_rigw.sh | nl -ba | sed -n '1801,2250p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-im9fVXTM' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-kaIpbPIs' (errno=Operation not permitted)
  1801	    win_quiet_gate
  1802	    timer_gate
  1803	    windows_result_stream_gate
  1804	    stage_purge_helper
  1805	    write_manifest
  1806	    verify_fixtures
  1807	    log "PREFLIGHT OK: registered rig, exact binaries/helper, canonical paths, quiet endpoints"
  1808	}
  1809
  1810	q_daemon_stop() {
  1811	    local pid="$q_daemon_pid" i
  1812	    [[ -z "$pid" ]] && return 0
  1813	    if kill -0 "$pid" 2>/dev/null; then
  1814	        local cmd
  1815	        cmd=$(ps -p "$pid" -o command= 2>/dev/null || true)
  1816	        [[ "$cmd" == *"$Q_DAEMON"* ]] \
  1817	            || { teardown_die "refusing to stop q PID $pid because it is not the launched daemon: $cmd"; return 1; }
  1818	        kill "$pid" || true
  1819	        for ((i=0; i<40; i++)); do
  1820	            kill -0 "$pid" 2>/dev/null || break
  1821	            sleep 0.25
  1822	        done
  1823	        kill -0 "$pid" 2>/dev/null \
  1824	            && { teardown_die "q daemon PID $pid survived exact teardown"; return 1; }
  1825	    fi
  1826	    q_daemon_pid=""
  1827	}
  1828
  1829	win_daemon_stop() {
  1830	    local pid="$win_daemon_pid" cmdpid="$win_cmd_pid" out pid_probe
  1831	    if [[ -z "$pid" && -z "$cmdpid" && -n "$current_block" ]]; then
  1832	        if ! pid_probe=$(wssh "
  1833	\$ErrorActionPreference = 'Stop'
  1834	\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
  1835	\$d = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/daemon.pid' -ErrorAction SilentlyContinue
  1836	\$c = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/launcher.pid' -ErrorAction SilentlyContinue
  1837	if (-not \$c) {
  1838	  \$launchers = @(Get-CimInstance Win32_Process -Filter \"Name='cmd.exe'\" | Where-Object {
  1839	    \$actual = if (\$_.CommandLine) { \$_.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1840	    \$actual -ieq \$expectedLauncher
  1841	  })
  1842	  if (\$launchers.Count -gt 1) { throw \"multiple exact launchers match \$expectedLauncher\" }
  1843	  if (\$launchers.Count -eq 1) { \$c = [string]\$launchers[0].ProcessId }
  1844	}
  1845	if (-not \$d -and \$c -match '^[0-9]+$') {
  1846	  \$children = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1847	    \$_.ParentProcessId -eq [int]\$c
  1848	  })
  1849	  if (\$children.Count -gt 1) { throw \"multiple daemon children belong to launcher \$c\" }
  1850	  if (\$children.Count -eq 1) { \$d = [string]\$children[0].ProcessId }
  1851	}
  1852	\"P|\$c|\$d\"
  1853	" 2>/dev/null | tr -d '\r' | tail -1); then
  1854	            teardown_die "Windows PID recovery failed for block $current_block"
  1855	            return 1
  1856	        fi
  1857	        IFS='|' read -r _ cmdpid pid <<<"$pid_probe"
  1858	    fi
  1859	    if [[ -z "$pid" && -z "$cmdpid" ]]; then
  1860	        if [[ -n "$current_block" ]] && ! wssh \
  1861	            "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
  1862	            >/dev/null 2>&1; then
  1863	            teardown_die "Windows PID files are empty but port $PORT may still be open"
  1864	            return 1
  1865	        fi
  1866	        return 0
  1867	    fi
  1868	    [[ -z "$pid" || "$pid" =~ ^[0-9]+$ ]] \
  1869	        || { teardown_die "invalid remembered Windows daemon PID '$pid'"; return 1; }
  1870	    [[ -z "$cmdpid" || "$cmdpid" =~ ^[0-9]+$ ]] \
  1871	        || { teardown_die "invalid remembered Windows launcher PID '$cmdpid'"; return 1; }
  1872	    [[ -n "$current_block" ]] \
  1873	        || { teardown_die "cannot verify Windows launcher without a current block"; return 1; }
  1874	    out=$(wssh "
  1875	\$ErrorActionPreference = 'Stop'
  1876	\$pid0 = if ('$pid' -match '^[0-9]+$') { [int]'$pid' } else { \$null }
  1877	\$cmd0 = if ('$cmdpid' -match '^[0-9]+$') { [int]'$cmdpid' } else { \$null }
  1878	\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
  1879	\$c = if (\$cmd0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$cmd0\" -ErrorAction SilentlyContinue } else { \$null }
  1880	if (\$pid0) {
  1881	  \$d = Get-CimInstance Win32_Process -Filter \"ProcessId=\$pid0\" -ErrorAction SilentlyContinue
  1882	} elseif (\$cmd0) {
  1883	  \$children = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1884	    \$_.ParentProcessId -eq \$cmd0
  1885	  })
  1886	  if (\$children.Count -gt 1) { throw \"multiple daemon children belong to launcher \$cmd0\" }
  1887	  \$d = if (\$children.Count -eq 1) { \$children[0] } else { \$null }
  1888	} else {
  1889	  \$d = \$null
  1890	}
  1891	if (\$d) {
  1892	  \$actual = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1893	  if (\$d.Name -ine 'blit-daemon.exe' -or \$actual -ine '$WIN_ACTIVE') { throw \"daemon PID identity mismatch: \$(\$d.Name) \$(\$d.ExecutablePath)\" }
  1894	  if (\$cmd0 -and \$d.ParentProcessId -ne \$cmd0) { throw \"daemon parent mismatch: \$(\$d.ParentProcessId) != \$cmd0\" }
  1895	}
  1896	if (\$c) {
  1897	  \$actualLauncher = if (\$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1898	  if (\$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$expectedLauncher) { throw \"launcher command mismatch: \$(\$c.Name) \$actualLauncher\" }
  1899	}
  1900	# Every identity is validated before either remembered PID is stopped.
  1901	\$stoppedDaemonPid = if (\$d) { [int]\$d.ProcessId } else { \$null }
  1902	if (\$d) { Stop-Process -Id \$stoppedDaemonPid -Force }
  1903	if (\$c) { Stop-Process -Id \$cmd0 -Force }
  1904	Start-Sleep -Milliseconds 250
  1905	if (\$cmd0) {
  1906	  \$lateChildren = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1907	    \$_.ParentProcessId -eq \$cmd0
  1908	  })
  1909	  foreach (\$late in \$lateChildren) {
  1910	    \$actualLate = if (\$late.ExecutablePath) { \$late.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1911	    if (\$actualLate -ine '$WIN_ACTIVE') { throw \"late daemon child identity mismatch: \$(\$late.ExecutablePath)\" }
  1912	    Stop-Process -Id \$late.ProcessId -Force
  1913	  }
  1914	  if (\$lateChildren.Count -gt 0) { Start-Sleep -Milliseconds 250 }
  1915	}
  1916	if (\$stoppedDaemonPid -and (Get-Process -Id \$stoppedDaemonPid -ErrorAction SilentlyContinue)) { throw 'daemon survived teardown' }
  1917	if (\$cmd0 -and (Get-Process -Id \$cmd0 -ErrorAction SilentlyContinue)) { throw 'launcher survived teardown' }
  1918	if (\$cmd0 -and (@(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object { \$_.ParentProcessId -eq \$cmd0 }).Count -gt 0)) { throw 'late daemon child survived teardown' }
  1919	'STOPPED'
  1920	") || { teardown_die "Windows exact daemon teardown failed: $out"; return 1; }
  1921	    win_daemon_pid=""; win_cmd_pid=""
  1922	}
  1923
  1924	fetch_win_file() {
  1925	    local remote="$1" local_path="$2" tmp="$local_path.base64" remote_hash local_hash
  1926	    wssh "
  1927	\$b = [IO.File]::ReadAllBytes('$remote')
  1928	[Convert]::ToBase64String(\$b)
  1929	" | tr -d '\r\n' > "$tmp" || session_void "failed to fetch Windows log $remote"
  1930	    python3 - "$tmp" "$local_path" <<'PY'
  1931	import base64, pathlib, sys
  1932	src, dst = map(pathlib.Path, sys.argv[1:])
  1933	dst.write_bytes(base64.b64decode(src.read_text(), validate=True))
  1934	src.unlink()
  1935	PY
  1936	    remote_hash=$(sha256_win "$remote")
  1937	    local_hash=$(sha256_q "$local_path")
  1938	    [[ "$remote_hash" == "$local_hash" ]] \
  1939	        || session_void "Windows log hash mismatch for $remote"
  1940	}
  1941
  1942	collect_block_logs() {
  1943	    local block="$1" dir="$OUT_DIR/trace/block_$block"
  1944	    mkdir -p "$dir"
  1945	    fetch_win_file "$WIN_SESSION/block_$block/daemon.err" "$dir/windows-daemon.err"
  1946	    wssh "Remove-Item -LiteralPath '$WIN_SESSION/block_$block' -Recurse -Force -ErrorAction Stop" \
  1947	        >/dev/null || session_void "failed to remove retrieved Windows block $block logs"
  1948	}
  1949
  1950	stop_daemons() {
  1951	    local block="$1"
  1952	    win_daemon_stop
  1953	    q_daemon_stop
  1954	    collect_block_logs "$block"
  1955	    ports_closed || session_void "port $PORT still listening after block $block teardown"
  1956	}
  1957
  1958	q_daemon_start() {
  1959	    local block="$1" state="$2" run_id="$3" dir="$OUT_DIR/trace/block_$block"
  1960	    mkdir -p "$dir"
  1961	    cat > "$dir/q-daemon.toml" <<EOF
  1962	[daemon]
  1963	bind = "0.0.0.0"
  1964	port = $PORT
  1965	no_mdns = true
  1966
  1967	[[module]]
  1968	name = "bench"
  1969	path = "$Q_MODULE"
  1970	EOF
  1971	    if [[ "$state" == on ]]; then
  1972	        BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
  1973	            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
  1974	            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
  1975	    else
  1976	        env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID \
  1977	            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
  1978	            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
  1979	    fi
  1980	    q_daemon_pid=$!
  1981	    sleep 1
  1982	    kill -0 "$q_daemon_pid" 2>/dev/null \
  1983	        || session_void "q daemon failed to start in block $block"
  1984	}
  1985
  1986	win_daemon_start() {
  1987	    local block="$1" state="$2" run_id="$3" out
  1988	    # The CIM-created batch launcher is allowed to exist before its PID is
  1989	    # journaled, but launch.ok prevents it from executing the daemon until the
  1990	    # PID has been atomically placed and read back. Without the gate it times
  1991	    # out, so teardown never has to identify an unjournaled orphan daemon.
  1992	    out=$(wssh "
  1993	\$ErrorActionPreference = 'Stop'
  1994	New-Item -ItemType Directory -Force -Path '$WIN_SESSION/block_$block','$WIN_BINS/active' | Out-Null
  1995	\$startupState = @(
  1996	  '$WIN_SESSION/block_$block/launch.ok',
  1997	  '$WIN_SESSION/block_$block/launcher.pid',
  1998	  '$WIN_SESSION/block_$block/launcher.pid.tmp',
  1999	  '$WIN_SESSION/block_$block/daemon.pid'
  2000	)
  2001	foreach (\$path in \$startupState) {
  2002	  if (Test-Path -LiteralPath \$path) { throw \"stale launcher state: \$path\" }
  2003	}
  2004	Copy-Item -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -Destination '$WIN_ACTIVE' -Force
  2005	if ((Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_ACTIVE').Hash.ToLower() -ne '$WIN_DAEMON_HASH') { throw 'active daemon hash mismatch' }
  2006	Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.toml' -Value @(
  2007	  '[daemon]', 'bind = \"0.0.0.0\"', 'port = $PORT', 'no_mdns = true', '',
  2008	  '[[module]]', 'name = \"bench\"', 'path = \"$WIN_MODULE\"'
  2009	)
  2010	\$trace = if ('$state' -eq 'on') { @('set BLIT_TRACE_SESSION_PHASES=1','set BLIT_TRACE_RUN_ID=$run_id') } else { @('set BLIT_TRACE_SESSION_PHASES=','set BLIT_TRACE_RUN_ID=') }
  2011	Set-Content -LiteralPath '$WIN_SESSION/block_$block/start.cmd' -Value @(
  2012	  '@echo off',
  2013	  'set /a BLIT_LAUNCH_WAIT=0',
  2014	  ':wait_for_launch_ok',
  2015	  'if exist \"$WIN_SESSION/block_$block/launch.ok\" goto launch_ready',
  2016	  'set /a BLIT_LAUNCH_WAIT+=1',
  2017	  'if %BLIT_LAUNCH_WAIT% GEQ 15 exit /b 111',
  2018	  '>nul 2>&1 ping -n 2 127.0.0.1',
  2019	  'goto wait_for_launch_ok',
  2020	  ':launch_ready',
  2021	  \$trace[0], \$trace[1],
  2022	  '\"$WIN_ACTIVE\" --config \"$WIN_SESSION/block_$block/daemon.toml\" > \"$WIN_SESSION/block_$block/daemon.out\" 2> \"$WIN_SESSION/block_$block/daemon.err\"'
  2023	)
  2024	\$launcherCommand = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$block/start.cmd\"\"'
  2025	\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = \$launcherCommand }
  2026	if (\$r.ReturnValue -ne 0) { throw \"launcher return \$(\$r.ReturnValue)\" }
  2027	Set-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Value ([string]\$r.ProcessId) -NoNewline
  2028	Move-Item -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Destination '$WIN_SESSION/block_$block/launcher.pid' -ErrorAction Stop
  2029	\$persistedLauncher = (Get-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid' -Raw -ErrorAction Stop).Trim()
  2030	if (\$persistedLauncher -ne [string]\$r.ProcessId) { throw \"launcher PID persistence mismatch: \$persistedLauncher\" }
  2031	New-Item -ItemType File -Path '$WIN_SESSION/block_$block/launch.ok' -ErrorAction Stop | Out-Null
  2032	Start-Sleep -Seconds 2
  2033	\$c = Get-CimInstance Win32_Process -Filter \"ProcessId=\$(\$r.ProcessId)\" -ErrorAction SilentlyContinue
  2034	\$actualLauncher = if (\$c -and \$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  2035	if (-not \$c -or \$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$launcherCommand) { throw \"launcher identity mismatch: \$(\$c.Name) \$actualLauncher\" }
  2036	\$d = Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object ParentProcessId -eq \$r.ProcessId | Select-Object -First 1
  2037	if (-not \$d) { Get-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.err' -ErrorAction SilentlyContinue; throw 'daemon child absent' }
  2038	\$actualDaemon = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  2039	if (\$actualDaemon -ine '$WIN_ACTIVE' -or \$d.ParentProcessId -ne \$r.ProcessId) { throw \"daemon identity mismatch: \$(\$d.ExecutablePath) parent=\$(\$d.ParentProcessId)\" }
  2040	Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.pid' -Value \$d.ProcessId
  2041	\"P|\$(\$r.ProcessId)|\$(\$d.ProcessId)\"
  2042	") || session_void "Windows daemon failed to start in block $block: $out"
  2043	    out=${out//$'\r'/}
  2044	    IFS='|' read -r _ win_cmd_pid win_daemon_pid <<<"$(grep '^P|' <<<"$out" | tail -1)"
  2045	    [[ "$win_cmd_pid" =~ ^[0-9]+$ && "$win_daemon_pid" =~ ^[0-9]+$ ]] \
  2046	        || session_void "cannot parse Windows daemon PIDs from '$out'"
  2047	}
  2048
  2049	start_daemons() {
  2050	    local block="$1" state="$2" run_id="$3"
  2051	    ports_closed || session_void "port $PORT occupied before block $block"
  2052	    q_daemon_start "$block" "$state" "$run_id"
  2053	    win_daemon_start "$block" "$state" "$run_id"
  2054	    sleep 1
  2055	    nc -z -w 3 "$WIN_IP" "$PORT" || session_void "q cannot reach Windows daemon in block $block"
  2056	    wssh "if (-not (Test-NetConnection -ComputerName '$Q_IP' -Port $PORT -InformationLevel Quiet)) { exit 8 }" \
  2057	        >/dev/null || session_void "Windows cannot reach q daemon in block $block"
  2058	    log "block $block daemons up, trace=$state, run_id=$run_id"
  2059	}
  2060
  2061	record_clock_samples() {
  2062	    local block="$1" run_id="$2" cell="$3" pair="$4" role="$5" phase="$6" sample before after remote rtt midpoint offset
  2063	    for sample in 1 2 3; do
  2064	        before=$(python3 -c 'import time; print(time.time_ns())')
  2065	        remote=$(wssh '([DateTime]::UtcNow.Ticks - 621355968000000000) * 100' | tr -cd '0-9')
  2066	        after=$(python3 -c 'import time; print(time.time_ns())')
  2067	        [[ "$remote" =~ ^[0-9]+$ ]] || session_void "clock probe returned '$remote'"
  2068	        rtt=$((after - before)); midpoint=$((before + rtt / 2)); offset=$((remote - midpoint))
  2069	        append_clock_row \
  2070	            "$block" "$run_id" "$cell" "$pair" "$role" "$phase" "$sample" \
  2071	            "$before" "$remote" "$after" "$rtt" "$offset" >> "$CLOCK_CSV"
  2072	    done
  2073	}
  2074
  2075	drain_both() {
  2076	    sync || return 1
  2077	    sudo -n /usr/sbin/purge >/dev/null || return 1
  2078	    wssh "
  2079	\$ErrorActionPreference = 'Stop'
  2080	Write-VolumeCache D
  2081	\$quiet = 0
  2082	for (\$i=0; \$i -lt 30; \$i++) {
  2083	  \$w = (Get-Counter '\\PhysicalDisk(_Total)\\Disk Write Bytes/sec' -SampleInterval 1 -MaxSamples 1).CounterSamples[0].CookedValue
  2084	  if (\$null -ne \$w -and [double]\$w -lt 1048576) { \$quiet++ } else { \$quiet=0 }
  2085	  if (\$quiet -ge 3) { break }
  2086	}
  2087	if (\$quiet -lt 3) { throw 'DRAIN-TIMEOUT' }
  2088	\$purgeItem = Get-Item -LiteralPath '$WIN_PURGE' -Force -ErrorAction Stop
  2089	if ((\$purgeItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'purge helper is a reparse point' }
  2090	\$purgeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE').Hash.ToLower()
  2091	if (\$purgeHash -cne '$WIN_PURGE_HASH') { throw \"purge helper hash mismatch: \$purgeHash\" }
  2092	\$purgeOutput = @(& pwsh -NoProfile -File '$WIN_PURGE')
  2093	\$purgeRc = \$LASTEXITCODE
  2094	if (\$purgeRc -ne 0) { throw \"purge helper rc \$purgeRc\" }
  2095	if (\$purgeOutput.Count -ne 1 -or [string]\$purgeOutput[0] -cne 'standby-purged') { throw \"purge helper output mismatch: \$(\$purgeOutput -join '|')\" }
  2096	'drained'
  2097	" >/dev/null || return 1
  2098	    printf drained
  2099	}
  2100
  2101	prepare_destination() {
  2102	    local direction="$1" dest="$2" first
  2103	    if [[ "$direction" == wm ]]; then
  2104	        rm -rf -- "$dest" || return 1
  2105	        [[ ! -e "$dest" && ! -L "$dest" ]] || return 1
  2106	        mkdir -p -- "$dest" || return 1
  2107	        [[ -d "$dest" && ! -L "$dest" ]] || return 1
  2108	        first=$(find "$dest" -mindepth 1 -maxdepth 1 -print -quit) || return 1
  2109	        [[ -z "$first" ]] || return 1
  2110	    else
  2111	        wssh "
  2112	\$ErrorActionPreference = 'Stop'
  2113	if (Test-Path -LiteralPath '$dest') {
  2114	  Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
  2115	}
  2116	if (Test-Path -LiteralPath '$dest') { throw 'destination removal did not land' }
  2117	New-Item -ItemType Directory -Force -Path '$dest' -ErrorAction Stop | Out-Null
  2118	if (-not (Test-Path -LiteralPath '$dest' -PathType Container)) { throw 'destination is not a directory' }
  2119	\$item = Get-Item -LiteralPath '$dest' -Force -ErrorAction Stop
  2120	if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'destination is a reparse point' }
  2121	if (@(Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop).Count -ne 0) { throw 'destination is not empty' }
  2122	" || return 1
  2123	    fi
  2124	}
  2125
  2126	flush_verify_q() {
  2127	    python3 - "$1" <<'PY'
  2128	import os, sys, time
  2129	t=time.monotonic_ns(); n=b=0
  2130	for root, dirs, files in os.walk(sys.argv[1]):
  2131	    for name in files:
  2132	        p=os.path.join(root,name)
  2133	        fd=os.open(p,os.O_RDONLY); os.fsync(fd); os.close(fd)
  2134	        n+=1; b+=os.path.getsize(p)
  2135	print(f"F|{round((time.monotonic_ns()-t)/1_000_000)}|{n}|{b}")
  2136	PY
  2137	}
  2138
  2139	flush_verify_win() {
  2140	    wssh "
  2141	\$ErrorActionPreference = 'Stop'
  2142	\$sw=[Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache D; \$sw.Stop()
  2143	\$f=Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
  2144	\$bytes=if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 }
  2145	\"F|\$([int]\$sw.Elapsed.TotalMilliseconds)|\$(\$f.Count)|\$bytes\"
  2146	" | tr -d '\r' | tail -1
  2147	}
  2148
  2149	q_client_run() {
  2150	    local state="$1" run_id="$2" err="$3"; shift 3
  2151	    local trace_env=()
  2152	    if [[ "$state" == on ]]; then
  2153	        trace_env=(BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id")
  2154	    fi
  2155	    env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID "${trace_env[@]}" \
  2156	        python3 - "$err" "$Q_BLIT" "$@" <<'PY'
  2157	import os, subprocess, sys, time
  2158	err, argv = sys.argv[1], sys.argv[2:]
  2159	clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
  2160	with open(err, "wb") as e:
  2161	    t=clock_ns()
  2162	    p=subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=e, env=os.environ.copy())
  2163	    done_ns=clock_ns()
  2164	    ms=round((done_ns-t)/1_000_000)
  2165	print(f"R|{ms}|{p.returncode}|{done_ns}")
  2166	PY
  2167	}
  2168
  2169	win_client_run() {
  2170	    local state="$1" run_id="$2" remote_err="$3"; shift 3
  2171	    local src="$1" dst="$2" flag="${3:-}" out
  2172	    out=$(wssh "
  2173	\$ErrorActionPreference = 'Stop'
  2174	if ('$state' -eq 'on') { \$env:BLIT_TRACE_SESSION_PHASES='1'; \$env:BLIT_TRACE_RUN_ID='$run_id' }
  2175	else { Remove-Item Env:BLIT_TRACE_SESSION_PHASES,Env:BLIT_TRACE_RUN_ID -ErrorAction SilentlyContinue }
  2176	\$sw=[Diagnostics.Stopwatch]::StartNew()
  2177	& '$WIN_BINS/$HEAD_SHORT/blit.exe' copy '$src' '$dst' --yes $flag > \$null 2> '$remote_err'
  2178	\$rc=\$LASTEXITCODE; \$sw.Stop()
  2179	[Console]::Out.WriteLine(\"R|\$([int]\$sw.Elapsed.TotalMilliseconds)|\${rc}\")
  2180	[Console]::Out.Flush()
  2181	" | stamp_result_arrival_on_q) || true
  2182	    printf '%s\n' "$out"
  2183	}
  2184
  2185	session_id_from_log() {
  2186	    python3 - "$1" <<'PY'
  2187	import json, re, sys
  2188	ids=set()
  2189	with open(sys.argv[1], errors="replace") as f:
  2190	    for line in f:
  2191	        if line.startswith("[session-phase] "):
  2192	            ids.add(json.loads(line[len("[session-phase] "):])["session_id"])
  2193	if len(ids)>1: raise SystemExit(f"multiple session ids: {sorted(ids)}")
  2194	print(next(iter(ids), ""))
  2195	PY
  2196	}
  2197
  2198	run_arm() {
  2199	    local block="$1" state="$2" pass="$3" run_id="$4" cell="$5" pair="$6" role="$7" role_order="$8"
  2200	    local direction carrier shape flag="" dest dest_arg rid qerr werr client_rel client_abs remote_err result result_tag result_extra transfer_ms rc flush_out flush_ms count bytes want drain session_id total anchor_now_ns
  2201	    local windows_client=0 arm_phase=client_done client_done_ns settle_deadline_ns settle_done_ns settled_ms
  2202	    local landed_root landed_manifest canonical_manifest remote_manifest tree_manifest_sha256
  2203	    direction=${cell%%_*}
  2204	    carrier=${cell#*_}; carrier=${carrier%%_*}
  2205	    shape=${cell##*_}
  2206	    [[ "$carrier" == grpc ]] && flag=--force-grpc
  2207	    rid="b${block}_${cell}_p${pair}_${role}"
  2208	    qerr="$OUT_DIR/client/$rid.err"
  2209	    remote_err="$WIN_SESSION/block_$block/$rid.client.err"
  2210	    werr="$OUT_DIR/client/$rid.windows.err"
  2211
  2212	    dest=$(arm_destination_path "$direction" "$role") \
  2213	        || session_void "unregistered destination path for $direction/$role"
  2214	    dest_arg=$(arm_destination_argument "$direction" "$role") \
  2215	        || session_void "unregistered destination argument for $direction/$role"
  2216	    prepare_destination "$direction" "$dest" \
  2217	        || session_void "$rid could not precreate its destination container"
  2218
  2219	    drain=$(drain_both) || session_void "$rid cache/drain gate failed"
  2220	    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" before
  2221
  2222	    if [[ "$direction/$role" == wm/source_init ]]; then
  2223	        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
  2224	        result=$(win_client_run "$state" "$run_id" "$remote_err" \
  2225	            "$(win_source_path "$shape")" "$dest_arg" "$flag")
  2226	    elif [[ "$direction/$role" == wm/destination_init ]]; then
  2227	        client_abs="$qerr"; client_rel="client/$rid.err"
  2228	        result=$(q_client_run "$state" "$run_id" "$qerr" \
  2229	            copy "$WIN_IP:$PORT:/bench/src_$shape" "$dest_arg" --yes ${flag:+$flag})
  2230	    elif [[ "$direction/$role" == mw/source_init ]]; then
  2231	        client_abs="$qerr"; client_rel="client/$rid.err"
  2232	        result=$(q_client_run "$state" "$run_id" "$qerr" \
  2233	            copy "$(q_source_path "$shape")" "$dest_arg" --yes ${flag:+$flag})
  2234	    elif [[ "$direction/$role" == mw/destination_init ]]; then
  2235	        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
  2236	        result=$(win_client_run "$state" "$run_id" "$remote_err" \
  2237	            "$Q_IP:$PORT:/bench/src_$shape" "$dest_arg" "$flag")
  2238	    else
  2239	        session_void "unregistered arm $direction/$role"
  2240	    fi
  2241
  2242	    # Both wrappers carry a q-monotonic completion anchor: immediate child
  2243	    # return for a q client, and result-line arrival for a Windows client.
  2244	    # Wrapper/SSH teardown after that anchor is therefore inside the absolute
  2245	    # settle interval.  The first 250 ms is the common excluded observation
  2246	    # budget; every overrun remains charged to the durable total below.
  2247	    IFS='|' read -r result_tag transfer_ms rc client_done_ns result_extra <<<"$result"
  2248	    if [[ "$result_tag" != R || ! "$transfer_ms" =~ ^[0-9]+$ \
  2249	        || ! "$rc" =~ ^[0-9]+$ || ! "$client_done_ns" =~ ^[0-9]+$ \
  2250	        || -n "$result_extra" ]]; then

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/bench_otp12pf_rigw.sh | nl -ba | sed -n '2251,2700p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-p19K4cMx' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1ap6obzX' (errno=Operation not permitted)
  2251	        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
  2252	        session_void "$rid timer/client sentinel malformed: '$result'"
  2253	    fi
  2254	    if [[ "$rc" != 0 ]]; then
  2255	        # Fetch this client log opportunistically; the failure trap also keeps
  2256	        # the remote session tree intact for postmortem evidence.
  2257	        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
  2258	        session_void "$rid client failed rc=$rc (see $client_rel)"
  2259	    fi
  2260
  2261	    anchor_now_ns=$(q_monotonic_ns)
  2262	    [[ "$client_done_ns" -le "$anchor_now_ns" ]] \
  2263	        || session_void "$rid client completion anchor is in the future"
  2264	    [[ $((anchor_now_ns - client_done_ns)) -lt $((SETTLE_MAX_MS * 1000000)) ]] \
  2265	        || session_void "$rid client wrapper teardown already exceeded the settle bound"
  2266	    settle_deadline_ns=$((client_done_ns + SETTLE_NS))
  2267
  2268	    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after
  2269	    settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")
  2270	    [[ "$settle_done_ns" =~ ^[0-9]+$ && "$settle_done_ns" -ge "$settle_deadline_ns" ]] \
  2271	        || session_void "$rid absolute post-client settle returned early: '$settle_done_ns'"
  2272	    settled_ms=$(((settle_done_ns - client_done_ns) / 1000000))
  2273	    [[ "$settled_ms" -ge "$SETTLE_MIN_MS" && "$settled_ms" -lt "$SETTLE_MAX_MS" ]] \
  2274	        || session_void "$rid post-client settle was ${settled_ms}ms, expected [$SETTLE_MIN_MS,$SETTLE_MAX_MS)"
  2275
  2276	    # The destination OS—not the initiator role—selects the durability and
  2277	    # landed-tree probe.  This remains outside transfer_ms.
  2278	    landed_root="src_$shape"
  2279	    landed_manifest="$OUT_DIR/landed/$rid.manifest"
  2280	    canonical_manifest="$OUT_DIR/fixtures/src_$shape.manifest"
  2281	    if [[ "$direction" == wm ]]; then
  2282	        flush_out=$(flush_verify_q "$dest") || session_void "$rid q durability probe failed"
  2283	        write_q_tree_manifest "$dest" "$landed_manifest" "$landed_root" \
  2284	            || session_void "$rid q landed root/manifest verification failed"
  2285	    else
  2286	        flush_out=$(flush_verify_win "$dest") || session_void "$rid Windows durability probe failed"
  2287	        remote_manifest="$WIN_SESSION/block_$block/$rid.tree.manifest"
  2288	        write_win_tree_manifest \
  2289	            "$dest" "$remote_manifest" "$landed_manifest" "$landed_root" \
  2290	            || session_void "$rid Windows landed root/manifest verification failed"
  2291	    fi
  2292	    IFS='|' read -r _ flush_ms count bytes <<<"$flush_out"
  2293	    case "$shape" in mixed) want='5001|547110912';; large) want='1|1073741824';; esac
  2294	    [[ "$count|$bytes" == "$want" ]] \
  2295	        || session_void "$rid landed $count files/$bytes bytes, expected $want"
  2296	    [[ "$flush_ms" =~ ^[0-9]+$ ]] || session_void "$rid flush timer malformed: '$flush_out'"
  2297	    tree_manifest_sha256=$(matching_manifest_digest \
  2298	        "$canonical_manifest" "$landed_manifest") \
  2299	        || session_void "$rid landed relative-path/size manifest differs from canonical"
  2300	    [[ "$tree_manifest_sha256" =~ ^[0-9a-f]{64}$ ]] \
  2301	        || session_void "$rid tree manifest digest is malformed"
  2302	    if [[ "$direction" == wm ]]; then
  2303	        rm -rf -- "$dest" || session_void "$rid failed to remove verified q destination"
  2304	        [[ ! -e "$dest" && ! -L "$dest" ]] \
  2305	            || session_void "$rid verified q destination survived removal"
  2306	    else
  2307	        wssh "
  2308	\$ErrorActionPreference = 'Stop'
  2309	Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
  2310	if (Test-Path -LiteralPath '$dest') { throw 'verified destination survived removal' }
  2311	" \
  2312	            || session_void "$rid failed to remove verified Windows destination"
  2313	    fi
  2314	    arm_phase=durability_verified
  2315
  2316	    if [[ "$windows_client" == 1 ]]; then
  2317	        fetch_successful_windows_client_log "$arm_phase" "$remote_err" "$werr"
  2318	    fi
  2319
  2320	    session_id=$(session_id_from_log "$client_abs") \
  2321	        || session_void "$rid client trace is malformed"
  2322	    if [[ "$state" == on && "$carrier" == tcp ]]; then
  2323	        [[ "$session_id" =~ ^[0-9a-f]{16}$ ]] \
  2324	            || session_void "$rid trace-on TCP client has session_id '$session_id'"
  2325	    else
  2326	        [[ -z "$session_id" ]] \
  2327	            || session_void "$rid emitted TCP phase trace in state=$state carrier=$carrier"
  2328	    fi
  2329
  2330	    total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))
  2331	    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
  2332	        "$block" "$state" "$pass" "$cell" "$role" "$pair" "$role_order" \
  2333	        "$transfer_ms" "$settled_ms" "$flush_ms" "$total" "$landed_root" \
  2334	        "$tree_manifest_sha256" "$rc" "$drain" yes "$run_id" "$session_id" \
  2335	        "$client_rel" >> "$RUNS_CSV"
  2336	    log "$rid: transfer=${transfer_ms}ms settled=${settled_ms}ms flush=${flush_ms}ms total=${total}ms session=${session_id:-none}"
  2337	}
  2338
  2339	cell_order() {
  2340	    local pass="$1" round="$2"
  2341	    local forward='wm_tcp_mixed mw_tcp_mixed wm_grpc_mixed wm_tcp_large'
  2342	    local reverse='wm_tcp_large wm_grpc_mixed mw_tcp_mixed wm_tcp_mixed'
  2343	    local base
  2344	    [[ "$pass" == forward ]] && base="$forward" || base="$reverse"
  2345	    case "$round" in 1|4) printf '%s\n' "$base";; 2|3) [[ "$base" == "$forward" ]] && printf '%s\n' "$reverse" || printf '%s\n' "$forward";; esac
  2346	}
  2347
  2348	run_block() {
  2349	    local block="$1" state="$2" pass="$3" first="$4" last="$5" run_id="${SESSION_TAG}-b${block}-${state}"
  2350	    local round pair cells cell first_role second_role
  2351	    q_quiet_gate; win_quiet_gate
  2352	    start_daemons "$block" "$state" "$run_id"
  2353	    for ((round=1; round<=PAIRS_PER_BLOCK; round++)); do
  2354	        pair=$((first + round - 1))
  2355	        [[ "$pair" -le "$last" ]] || session_void "block $block pair schedule overflow"
  2356	        q_quiet_gate
  2357	        case "$round" in
  2358	            1|4) first_role=source_init; second_role=destination_init;;
  2359	            2|3) first_role=destination_init; second_role=source_init;;
  2360	        esac
  2361	        cells=$(cell_order "$pass" "$round")
  2362	        local old_ifs="$IFS"; IFS=' '
  2363	        for cell in $cells; do
  2364	            IFS="$old_ifs"
  2365	            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$first_role" 1
  2366	            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$second_role" 2
  2367	            IFS=' '
  2368	        done
  2369	        IFS="$old_ifs"
  2370	    done
  2371	    stop_daemons "$block"
  2372	    q_quiet_gate; win_quiet_gate
  2373	}
  2374
  2375	end_gate() {
  2376	    q_topology_gate
  2377	    win_topology_gate
  2378	    mss_gate
  2379	    q_quiet_gate
  2380	    win_quiet_gate
  2381	    ports_closed || session_void "end gate found a listener on port $PORT"
  2382	}
  2383
  2384	strict_success_cleanup() {
  2385	    STRICT_CLEANUP_VERIFIED=0
  2386	    [[ -z "$q_daemon_pid" ]] \
  2387	        || { LAST_ERROR="strict cleanup found remembered q daemon PID $q_daemon_pid"; return 1; }
  2388	    [[ -z "$win_daemon_pid" ]] \
  2389	        || { LAST_ERROR="strict cleanup found remembered Windows daemon PID $win_daemon_pid"; return 1; }
  2390	    [[ -z "$win_cmd_pid" ]] \
  2391	        || { LAST_ERROR="strict cleanup found remembered Windows launcher PID $win_cmd_pid"; return 1; }
  2392	    [[ -z "$current_block" ]] \
  2393	        || { LAST_ERROR="strict cleanup found current block $current_block"; return 1; }
  2394
  2395	    ports_closed \
  2396	        || { LAST_ERROR="strict cleanup found port $PORT still listening"; return 1; }
  2397	    wssh "
  2398	\$ErrorActionPreference = 'Stop'
  2399	\$paths = @('$WIN_MODULE/rigw-sessions/$SESSION_TAG', '$WIN_SESSION')
  2400	foreach (\$path in \$paths) {
  2401	  if (Test-Path -LiteralPath \$path) {
  2402	    Remove-Item -LiteralPath \$path -Recurse -Force -ErrorAction Stop
  2403	  }
  2404	  if (Test-Path -LiteralPath \$path) { throw \"strict cleanup left \$path\" }
  2405	}
  2406	    " >/dev/null \
  2407	        || { LAST_ERROR="strict cleanup could not remove and verify Windows session trees"; return 1; }
  2408	    WIN_SESSION_MAY_EXIST=0
  2409	    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
  2410	        rm -rf -- "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
  2411	            || { LAST_ERROR="strict cleanup could not remove q session tree"; return 1; }
  2412	    fi
  2413	    [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
  2414	        && ! -L "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
  2415	        || { LAST_ERROR="strict cleanup found a surviving or unexpected q session tree"; return 1; }
  2416	    Q_SESSION_MAY_EXIST=0
  2417	    ports_closed \
  2418	        || { LAST_ERROR="strict cleanup found port $PORT reopened during deletion"; return 1; }
  2419	    STRICT_CLEANUP_VERIFIED=1
  2420	}
  2421
  2422	launcher_smoke() {
  2423	    local run_id="${SESSION_TAG}-launcher-smoke"
  2424	    WIN_SESSION_MAY_EXIST=1
  2425	    current_block=launcher-smoke
  2426	    ports_closed \
  2427	        || session_void "port $PORT occupied before launcher smoke"
  2428	    win_daemon_start "$current_block" off "$run_id"
  2429	    nc -z -w 3 "$WIN_IP" "$PORT" \
  2430	        || session_void "q cannot reach Windows daemon in launcher smoke"
  2431	    stop_daemons "$current_block"
  2432	    current_block=""
  2433	    strict_success_cleanup \
  2434	        || session_void "launcher smoke cleanup failed: ${LAST_ERROR:-unknown error}"
  2435	    log "LAUNCHER_SMOKE OK: exact Windows CIM launcher started, reached, identity-stopped, and cleaned; no transfer timed"
  2436	}
  2437
  2438	finalize_registered_session() {
  2439	    local complete_tmp="$OUT_DIR/SESSION-COMPLETE.tmp"
  2440	    SESSION_FINALIZED=0
  2441	    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 ]] \
  2442	        || { LAST_ERROR="refusing cleanup before local evidence is complete"; return 1; }
  2443	    strict_success_cleanup || return 1
  2444	    [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
  2445	        || { LAST_ERROR="strict cleanup returned without verification"; return 1; }
  2446	    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]] \
  2447	        || { LAST_ERROR="refusing to complete a void session"; return 1; }
  2448	    [[ ! -e "$OUT_DIR/SESSION-COMPLETE" && ! -L "$OUT_DIR/SESSION-COMPLETE" ]] \
  2449	        || { LAST_ERROR="refusing to replace an existing completion marker"; return 1; }
  2450	    [[ ! -e "$complete_tmp" && ! -L "$complete_tmp" ]] \
  2451	        || { LAST_ERROR="refusing to replace an existing completion temporary"; return 1; }
  2452	    printf '%s\n' "$HEAD_FULL" > "$complete_tmp" || return 1
  2453	    mv "$complete_tmp" "$OUT_DIR/SESSION-COMPLETE" || return 1
  2454	    SESSION_FINALIZED=1
  2455	}
  2456
  2457	record_failure_evidence() {
  2458	    append_void_line "local evidence preserved at $OUT_DIR"
  2459	    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
  2460	        append_void_line "q session evidence may remain; inspect $Q_MODULE/rigw-sessions/$SESSION_TAG"
  2461	    fi
  2462	    if [[ "$WIN_SESSION_MAY_EXIST" == 1 ]]; then
  2463	        append_void_line "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG"
  2464	    fi
  2465	}
  2466
  2467	on_signal() {
  2468	    local signal="$1" code="$2"
  2469	    LAST_ERROR="received $signal"
  2470	    trap '' HUP INT TERM
  2471	    exit "$code"
  2472	}
  2473
  2474	install_signal_traps() {
  2475	    trap 'on_signal HUP 129' HUP
  2476	    trap 'on_signal INT 130' INT
  2477	    trap 'on_signal TERM 143' TERM
  2478	}
  2479
  2480	registered_completion_marker_valid() {
  2481	    local marker="$OUT_DIR/SESSION-COMPLETE" lines
  2482	    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 \
  2483	        && -n "${HEAD_FULL:-}" && -f "$marker" && ! -L "$marker" ]] || return 1
  2484	    lines=$(LC_ALL=C wc -l < "$marker") || return 1
  2485	    lines=${lines//[[:space:]]/}
  2486	    [[ "$lines" == 1 && "$(< "$marker")" == "$HEAD_FULL" ]] || return 1
  2487	    [[ ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
  2488	        && ! -L "$OUT_DIR/SESSION-COMPLETE.tmp" ]] || return 1
  2489	    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]]
  2490	}
  2491
  2492	on_exit() {
  2493	    local rc=$?
  2494	    trap - EXIT
  2495	    trap '' HUP INT TERM
  2496	    set +e
  2497	    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
  2498	        && ( -e "$OUT_DIR/SESSION-VOID" || -L "$OUT_DIR/SESSION-VOID" \
  2499	            || -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
  2500	            || -L "$OUT_DIR/SESSION-COMPLETE.tmp" ) ]]; then
  2501	        LAST_ERROR="successful exit retained a failure or temporary marker"
  2502	        rc=1
  2503	    fi
  2504	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 \
  2505	        && "$SESSION_FINALIZED" != 1 ]]; then
  2506	        LAST_ERROR="registered run returned without finalizing the session"
  2507	        rc=1
  2508	    fi
  2509	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 ]] \
  2510	        && ! registered_completion_marker_valid; then
  2511	        LAST_ERROR="registered completion marker is absent or invalid"
  2512	        rc=1
  2513	    fi
  2514	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
  2515	        && "$SESSION_FINALIZED" != 0 ]]; then
  2516	        LAST_ERROR="non-registered run claimed registered finalization"
  2517	        rc=1
  2518	    fi
  2519	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
  2520	        && ( -e "$OUT_DIR/SESSION-COMPLETE" \
  2521	            || -L "$OUT_DIR/SESSION-COMPLETE" ) ]]; then
  2522	        LAST_ERROR="non-registered run left a completion marker"
  2523	        rc=1
  2524	    fi
  2525	    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
  2526	        && "$STRICT_CLEANUP_VERIFIED" != 1 ]]; then
  2527	        LAST_ERROR="successful exit lacked verified strict cleanup"
  2528	        rc=1
  2529	    fi
  2530
  2531	    if [[ $rc -ne 0 ]]; then
  2532	        rm -f -- "$OUT_DIR/SESSION-COMPLETE" "$OUT_DIR/SESSION-COMPLETE.tmp" \
  2533	            || CLEANUP_ERROR="${CLEANUP_ERROR:+$CLEANUP_ERROR; }could not remove completion marker"
  2534	        if [[ ! -s "$OUT_DIR/SESSION-VOID" ]]; then
  2535	            append_void_line "${LAST_ERROR:-unexpected harness failure rc=$rc}"
  2536	        fi
  2537	        CLEANUP_MODE=1
  2538	        if [[ -n "$win_daemon_pid" || -n "$win_cmd_pid" || -n "$current_block" ]]; then
  2539	            win_daemon_stop || true
  2540	        fi
  2541	        if [[ -n "$q_daemon_pid" ]]; then q_daemon_stop || true; fi
  2542	        if [[ -n "$CLEANUP_ERROR" ]]; then
  2543	            append_void_line "cleanup errors: $CLEANUP_ERROR"
  2544	        fi
  2545	        record_failure_evidence
  2546	        exit 1
  2547	    fi
  2548	    exit 0
  2549	}
  2550
  2551	main() {
  2552	    validate_mode_selection
  2553	    if [[ "$SELFTEST" == 1 ]]; then selftest; return; fi
  2554	    if ! claim_output_dir; then
  2555	        printf '%s\n' "FATAL: $OUTPUT_CLAIM_ERROR" >&2
  2556	        return 1
  2557	    fi
  2558	    trap on_exit EXIT
  2559	    install_signal_traps
  2560	    preflight
  2561	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
  2562	        strict_success_cleanup || session_void "preflight cleanup failed: ${LAST_ERROR:-unknown error}"
  2563	        log "PREFLIGHT_ONLY: no daemon started and no transfer timed"
  2564	        return
  2565	    fi
  2566	    if [[ "$LAUNCHER_SMOKE" == 1 ]]; then
  2567	        launcher_smoke
  2568	        return
  2569	    fi
  2570
  2571	    REGISTERED_RUN_STARTED=1
  2572	    Q_SESSION_MAY_EXIST=1
  2573	    mkdir -p "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
  2574	        || session_void "cannot create registered q session directory"
  2575	    printf '%s\n' 'block,trace_state,pass,cell,role,pair,role_order,transfer_ms,settled_ms,flush_ms,total_ms,landed_root,tree_manifest_sha256,exit,drain,valid,run_id,session_id,client_log' > "$RUNS_CSV"
  2576	    printf '%s\n' 'block,run_id,cell,pair,role,phase,sample,q_before_ns,windows_ns,q_after_ns,rtt_ns,offset_windows_minus_q_ns' > "$CLOCK_CSV"
  2577	    emit_schedule > "$OUT_DIR/schedule.csv"
  2578	    WIN_SESSION_MAY_EXIST=1
  2579	    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION' | Out-Null" \
  2580	        || session_void "cannot create registered Windows session directory"
  2581
  2582	    local block state pass first last
  2583	    while IFS=, read -r block state pass first last; do
  2584	        current_block="$block"
  2585	        run_block "$block" "$state" "$pass" "$first" "$last"
  2586	        current_block=""
  2587	    done < <(emit_schedule)
  2588
  2589	    end_gate
  2590	    python3 "$SCRIPT_DIR/otp12pf_rigw_analyze.py" "$OUT_DIR" \
  2591	        || session_void "phase/distribution analyzer rejected the session"
  2592	    LOCAL_EVIDENCE_COMPLETE=1
  2593	    log "ANALYZER ACCEPTED: exact local evidence inventory; finalizing session"
  2594	    finalize_registered_session \
  2595	        || session_void "registered finalization failed: ${LAST_ERROR:-unknown error}"
  2596	}
  2597
  2598	if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  2599	    main "$@"
  2600	fi

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/otp12pf_rigw_analyze.py | nl -ba | sed -n '1,500p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-vjuSRdZK' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-QSpFOZTU' (errno=Operation not permitted)
     1	#!/usr/bin/env python3
     2	"""Validate and analyze the registered otp-12 pf-1 rig-W session.
     3
     4	The analyzer is intentionally fail closed.  It accepts only the registered
     5	four-block schedule and writes reports only after the CSV and every structured
     6	TCP trace have passed validation.  Phase intervals are derived exclusively
     7	from one endpoint's ``elapsed_ns`` clock; ``unix_ns`` is retained as evidence
     8	but is never subtracted across hosts.
     9	"""
    10
    11	from __future__ import annotations
    12
    13	import argparse
    14	import base64
    15	import binascii
    16	import csv
    17	import hashlib
    18	import json
    19	import os
    20	import re
    21	import sys
    22	import tempfile
    23	from dataclasses import dataclass
    24	from decimal import Decimal, InvalidOperation
    25	from pathlib import Path, PurePosixPath
    26	from statistics import median
    27	from typing import Any, Iterable, Sequence
    28
    29
    30	CELLS = (
    31	    "wm_tcp_mixed",
    32	    "mw_tcp_mixed",
    33	    "wm_grpc_mixed",
    34	    "wm_tcp_large",
    35	)
    36	TCP_CELLS = frozenset(cell for cell in CELLS if "_tcp_" in cell)
    37	TARGET_CELL = "wm_tcp_mixed"
    38	ROLES = ("source_init", "destination_init")
    39	CSV_FIELDS = (
    40	    "block",
    41	    "trace_state",
    42	    "pass",
    43	    "cell",
    44	    "role",
    45	    "pair",
    46	    "role_order",
    47	    "transfer_ms",
    48	    "settled_ms",
    49	    "flush_ms",
    50	    "total_ms",
    51	    "landed_root",
    52	    "tree_manifest_sha256",
    53	    "exit",
    54	    "drain",
    55	    "valid",
    56	    "run_id",
    57	    "session_id",
    58	    "client_log",
    59	)
    60	CLOCK_FIELDS = (
    61	    "block",
    62	    "run_id",
    63	    "cell",
    64	    "pair",
    65	    "role",
    66	    "phase",
    67	    "sample",
    68	    "q_before_ns",
    69	    "windows_ns",
    70	    "q_after_ns",
    71	    "rtt_ns",
    72	    "offset_windows_minus_q_ns",
    73	)
    74	TRACE_PREFIX = "[session-phase] "
    75	SESSION_ID_RE = re.compile(r"^[0-9a-f]{16}$")
    76	SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
    77	SETTLE_MIN_MS = 250
    78	SETTLE_MAX_MS = 1000
    79	MEASURAND = "durable_total_ms"
    80
    81
    82	@dataclass(frozen=True)
    83	class BlockSpec:
    84	    number: int
    85	    trace_state: str
    86	    pass_name: str
    87	    pairs: range
    88	    cells: tuple[str, ...]
    89
    90
    91	BLOCKS = (
    92	    BlockSpec(1, "off", "forward", range(1, 5), CELLS),
    93	    BlockSpec(2, "on", "reverse", range(1, 5), tuple(reversed(CELLS))),
    94	    BlockSpec(3, "on", "forward", range(5, 9), CELLS),
    95	    BlockSpec(4, "off", "reverse", range(5, 9), tuple(reversed(CELLS))),
    96	)
    97
    98
    99	class AnalysisError(RuntimeError):
   100	    """The evidence is incomplete, contaminated, or off schedule."""
   101
   102
   103	@dataclass(frozen=True)
   104	class RunRow:
   105	    csv_line: int
   106	    schedule_index: int
   107	    block: int
   108	    trace_state: str
   109	    pass_name: str
   110	    cell: str
   111	    role: str
   112	    pair: int
   113	    role_order: int
   114	    transfer_ms: Decimal
   115	    settled_ms: int
   116	    flush_ms: Decimal
   117	    total_ms: Decimal
   118	    landed_root: str
   119	    tree_manifest_sha256: str
   120	    exit_code: int
   121	    drain: str
   122	    valid: str
   123	    run_id: str
   124	    session_id: str
   125	    client_log: str
   126
   127
   128	@dataclass(frozen=True)
   129	class TraceEvent:
   130	    source_file: str
   131	    source_line: int
   132	    raw: dict[str, Any]
   133
   134	    @property
   135	    def run_id(self) -> str:
   136	        return self.raw["run_id"]
   137
   138	    @property
   139	    def session_id(self) -> str:
   140	        return self.raw["session_id"]
   141
   142	    @property
   143	    def endpoint_role(self) -> str:
   144	        return self.raw["endpoint_role"]
   145
   146	    @property
   147	    def producer_seq(self) -> int:
   148	        return self.raw["producer_seq"]
   149
   150	    @property
   151	    def elapsed_ns(self) -> int:
   152	        return self.raw["elapsed_ns"]
   153
   154	    @property
   155	    def event(self) -> str:
   156	        return self.raw["event"]
   157
   158
   159	@dataclass(frozen=True)
   160	class ClockSample:
   161	    csv_line: int
   162	    run: RunRow
   163	    phase: str
   164	    sample: int
   165	    q_before_ns: int
   166	    windows_ns: int
   167	    q_after_ns: int
   168	    rtt_ns: int
   169	    offset_ns: int
   170
   171
   172	@dataclass(frozen=True)
   173	class ModeDescription:
   174	    gap: Decimal
   175	    left: tuple[Decimal, ...]
   176	    right: tuple[Decimal, ...]
   177
   178	    def render(self) -> str:
   179	        left = ";".join(decimal_text(value) for value in self.left)
   180	        if not self.right:
   181	            return f"[{left}]"
   182	        right = ";".join(decimal_text(value) for value in self.right)
   183	        return f"[{left}] | [{right}]"
   184
   185
   186	@dataclass(frozen=True)
   187	class ConditionStats:
   188	    cell: str
   189	    trace_state: str
   190	    source_values: tuple[Decimal, ...]
   191	    destination_values: tuple[Decimal, ...]
   192	    paired_deltas: tuple[Decimal, ...]
   193	    source_median: Decimal
   194	    destination_median: Decimal
   195	    delta: Decimal
   196	    paired_delta_median: Decimal
   197	    first4_delta_median: Decimal
   198	    last4_delta_median: Decimal
   199	    first_last_drift: Decimal
   200	    odd_delta_median: Decimal
   201	    even_delta_median: Decimal
   202	    odd_even_drift: Decimal
   203	    source_first_delta_median: Decimal
   204	    destination_first_delta_median: Decimal
   205	    role_order_drift: Decimal
   206	    paired_delta_range: Decimal
   207	    n_pair_split: Decimal
   208	    n_pair: Decimal
   209
   210
   211	@dataclass(frozen=True)
   212	class AnalysisResult:
   213	    summary_csv: Path
   214	    summary_md: Path
   215	    distributions_csv: Path
   216	    phase_events_csv: Path
   217	    phase_intervals_csv: Path
   218	    clock_summary_csv: Path
   219	    observer_bias: Decimal
   220	    n_resolution: Decimal
   221	    trace_event_count: int
   222
   223
   224	def decimal_text(value: Decimal) -> str:
   225	    if value == value.to_integral_value():
   226	        return str(int(value))
   227	    return format(value, "f").rstrip("0").rstrip(".")
   228
   229
   230	def parse_decimal(value: str, field: str, line: int) -> Decimal:
   231	    try:
   232	        result = Decimal(value)
   233	    except InvalidOperation as exc:
   234	        raise AnalysisError(f"runs.csv line {line}: {field} is not numeric: {value!r}") from exc
   235	    if not result.is_finite() or result < 0:
   236	        raise AnalysisError(
   237	            f"runs.csv line {line}: {field} must be a finite non-negative number"
   238	        )
   239	    return result
   240
   241
   242	def parse_int(value: str, field: str, line: int, source: str = "runs.csv") -> int:
   243	    try:
   244	        return int(value)
   245	    except ValueError as exc:
   246	        raise AnalysisError(
   247	            f"{source} line {line}: {field} is not an integer: {value!r}"
   248	        ) from exc
   249
   250
   251	def expected_roles(pair: int) -> tuple[str, str]:
   252	    """The registered S/D/D/S first-arm pattern in each four-round block."""
   253	    round_index = (pair - 1) % 4
   254	    return ROLES if round_index in (0, 3) else tuple(reversed(ROLES))
   255
   256
   257	def expected_schedule() -> list[tuple[BlockSpec, str, int, str, int]]:
   258	    expected: list[tuple[BlockSpec, str, int, str, int]] = []
   259	    for block in BLOCKS:
   260	        for round_index, pair in enumerate(block.pairs):
   261	            cells = block.cells if round_index in (0, 3) else tuple(reversed(block.cells))
   262	            for cell in cells:
   263	                for role_order, role in enumerate(
   264	                    expected_roles(pair), start=1
   265	                ):
   266	                    expected.append((block, cell, pair, role, role_order))
   267	    return expected
   268
   269
   270	def _safe_client_log(root: Path, value: str, line: int) -> None:
   271	    if not value:
   272	        raise AnalysisError(f"runs.csv line {line}: client_log is blank")
   273	    relative = Path(value)
   274	    if relative.is_absolute():
   275	        raise AnalysisError(f"runs.csv line {line}: client_log must be relative: {value!r}")
   276	    root_resolved = root.resolve()
   277	    candidate = (root / relative).resolve()
   278	    if candidate != root_resolved and root_resolved not in candidate.parents:
   279	        raise AnalysisError(f"runs.csv line {line}: client_log escapes output dir: {value!r}")
   280	    if not candidate.is_file():
   281	        raise AnalysisError(f"runs.csv line {line}: client_log does not exist: {value!r}")
   282
   283
   284	def _read_tree_manifest(path: Path, label: str) -> tuple[bytes, str]:
   285	    if not path.is_file():
   286	        raise AnalysisError(f"missing {label}: {path}")
   287	    data = path.read_bytes()
   288	    if not data or not data.endswith(b"\n"):
   289	        raise AnalysisError(f"{label}: manifest must be non-empty and newline-terminated")
   290	    try:
   291	        text = data.decode("ascii")
   292	    except UnicodeDecodeError as exc:
   293	        raise AnalysisError(f"{label}: manifest is not ASCII") from exc
   294	    lines = text.splitlines()
   295	    if lines != sorted(lines) or len(lines) != len(set(lines)):
   296	        raise AnalysisError(f"{label}: manifest lines are not exact sorted unique inventory")
   297	    for line_number, line in enumerate(lines, start=1):
   298	        try:
   299	            encoded, size_text = line.split(",", 1)
   300	        except ValueError as exc:
   301	            raise AnalysisError(
   302	                f"{label} line {line_number}: expected base64_path,decimal_size"
   303	            ) from exc
   304	        if not size_text.isascii() or not size_text.isdecimal():
   305	            raise AnalysisError(f"{label} line {line_number}: invalid decimal size")
   306	        try:
   307	            relative = base64.b64decode(encoded, validate=True).decode("utf-8")
   308	        except (binascii.Error, UnicodeDecodeError, ValueError) as exc:
   309	            raise AnalysisError(f"{label} line {line_number}: invalid UTF-8 base64 path") from exc
   310	        parsed = PurePosixPath(relative)
   311	        if (
   312	            not relative
   313	            or parsed.is_absolute()
   314	            or relative != parsed.as_posix()
   315	            or any(part in ("", ".", "..") for part in parsed.parts)
   316	        ):
   317	            raise AnalysisError(f"{label} line {line_number}: unsafe/noncanonical path")
   318	    return data, hashlib.sha256(data).hexdigest()
   319
   320
   321	def _load_fixture_manifests(root: Path) -> dict[str, tuple[bytes, str]]:
   322	    index_path = root / "fixture-manifests.csv"
   323	    if not index_path.is_file():
   324	        raise AnalysisError(f"missing {index_path}")
   325	    with index_path.open(newline="", encoding="utf-8") as handle:
   326	        reader = csv.DictReader(handle)
   327	        fields = ("shape", "sha256", "q_manifest", "windows_manifest")
   328	        if tuple(reader.fieldnames or ()) != fields:
   329	            raise AnalysisError("fixture-manifests.csv header mismatch")
   330	        rows = list(reader)
   331	    if [row["shape"] for row in rows] != ["mixed", "large"]:
   332	        raise AnalysisError("fixture-manifests.csv must contain mixed,large exactly")
   333	    fixture_dir = root / "fixtures"
   334	    expected_files = {
   335	        f"src_{shape}.manifest" for shape in ("mixed", "large")
   336	    } | {f"windows-src_{shape}.manifest" for shape in ("mixed", "large")}
   337	    actual_files = (
   338	        {
   339	            path.relative_to(fixture_dir).as_posix()
   340	            for path in fixture_dir.rglob("*")
   341	            if path.is_file()
   342	        }
   343	        if fixture_dir.is_dir()
   344	        else set()
   345	    )
   346	    if actual_files != expected_files:
   347	        raise AnalysisError(
   348	            "fixture manifest file inventory mismatch: expected "
   349	            f"{sorted(expected_files)}, got {sorted(actual_files)}"
   350	        )
   351
   352	    result: dict[str, tuple[bytes, str]] = {}
   353	    for row in rows:
   354	        shape = row["shape"]
   355	        q_relative = f"fixtures/src_{shape}.manifest"
   356	        win_relative = f"fixtures/windows-src_{shape}.manifest"
   357	        if row["q_manifest"] != q_relative or row["windows_manifest"] != win_relative:
   358	            raise AnalysisError(f"fixture-manifests.csv {shape}: path mapping mismatch")
   359	        q_data, q_digest = _read_tree_manifest(root / q_relative, f"q src_{shape}")
   360	        win_data, win_digest = _read_tree_manifest(
   361	            root / win_relative, f"Windows src_{shape}"
   362	        )
   363	        if q_data != win_data or q_digest != win_digest:
   364	            raise AnalysisError(f"canonical q/Windows src_{shape} manifests differ")
   365	        if row["sha256"] != q_digest:
   366	            raise AnalysisError(f"fixture-manifests.csv {shape}: digest mismatch")
   367	        result[shape] = (q_data, q_digest)
   368	    return result
   369
   370
   371	def load_runs(root: Path) -> list[RunRow]:
   372	    fixture_manifests = _load_fixture_manifests(root)
   373	    runs_path = root / "runs.csv"
   374	    if not runs_path.is_file():
   375	        raise AnalysisError(f"missing {runs_path}")
   376	    with runs_path.open(newline="", encoding="utf-8") as handle:
   377	        reader = csv.DictReader(handle)
   378	        if tuple(reader.fieldnames or ()) != CSV_FIELDS:
   379	            raise AnalysisError(
   380	                "runs.csv header mismatch: expected "
   381	                + ",".join(CSV_FIELDS)
   382	                + "; got "
   383	                + ",".join(reader.fieldnames or ())
   384	            )
   385	        raw_rows = list(reader)
   386
   387	    schedule = expected_schedule()
   388	    if len(raw_rows) != len(schedule):
   389	        raise AnalysisError(
   390	            f"runs.csv schedule incomplete: expected {len(schedule)} rows, got {len(raw_rows)}"
   391	        )
   392
   393	    rows: list[RunRow] = []
   394	    for index, (raw, expected) in enumerate(zip(raw_rows, schedule), start=0):
   395	        line = index + 2
   396	        block_spec, cell, pair, role, role_order = expected
   397	        actual_schedule = (
   398	            parse_int(raw["block"], "block", line),
   399	            raw["trace_state"],
   400	            raw["pass"],
   401	            raw["cell"],
   402	            parse_int(raw["pair"], "pair", line),
   403	            raw["role"],
   404	            parse_int(raw["role_order"], "role_order", line),
   405	        )
   406	        wanted_schedule = (
   407	            block_spec.number,
   408	            block_spec.trace_state,
   409	            block_spec.pass_name,
   410	            cell,
   411	            pair,
   412	            role,
   413	            role_order,
   414	        )
   415	        if actual_schedule != wanted_schedule:
   416	            raise AnalysisError(
   417	                f"runs.csv line {line}: schedule mismatch; expected {wanted_schedule}, "
   418	                f"got {actual_schedule}"
   419	            )
   420	        exit_code = parse_int(raw["exit"], "exit", line)
   421	        if exit_code != 0 or raw["drain"] != "drained" or raw["valid"] != "yes":
   422	            raise AnalysisError(
   423	                f"runs.csv line {line}: SESSION-VOID arm "
   424	                f"(exit={exit_code}, drain={raw['drain']!r}, valid={raw['valid']!r})"
   425	            )
   426	        run_id = raw["run_id"]
   427	        if not run_id:
   428	            raise AnalysisError(f"runs.csv line {line}: run_id is blank")
   429	        session_id = raw["session_id"]
   430	        traced_tcp = block_spec.trace_state == "on" and cell in TCP_CELLS
   431	        if traced_tcp:
   432	            if not SESSION_ID_RE.fullmatch(session_id):
   433	                raise AnalysisError(
   434	                    f"runs.csv line {line}: trace-on TCP session_id must be 16 lowercase hex"
   435	                )
   436	        elif session_id:
   437	            raise AnalysisError(
   438	                f"runs.csv line {line}: session_id must be blank for trace-off or gRPC arms"
   439	            )
   440	        _safe_client_log(root, raw["client_log"], line)
   441	        settled_ms = parse_int(raw["settled_ms"], "settled_ms", line)
   442	        if not SETTLE_MIN_MS <= settled_ms < SETTLE_MAX_MS:
   443	            raise AnalysisError(
   444	                f"runs.csv line {line}: settled_ms must be in "
   445	                f"[{SETTLE_MIN_MS},{SETTLE_MAX_MS}), got {settled_ms}"
   446	            )
   447	        transfer_ms = parse_decimal(raw["transfer_ms"], "transfer_ms", line)
   448	        flush_ms = parse_decimal(raw["flush_ms"], "flush_ms", line)
   449	        total_ms = parse_decimal(raw["total_ms"], "total_ms", line)
   450	        settle_excess_ms = Decimal(settled_ms - SETTLE_MIN_MS)
   451	        expected_total_ms = transfer_ms + settle_excess_ms + flush_ms
   452	        if total_ms != expected_total_ms:
   453	            raise AnalysisError(
   454	                f"runs.csv line {line}: total_ms must equal transfer_ms + "
   455	                f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms "
   456	                f"exactly; got {decimal_text(total_ms)} != "
   457	                f"{decimal_text(transfer_ms)} + ({settled_ms} - "
   458	                f"{SETTLE_MIN_MS}) + {decimal_text(flush_ms)}"
   459	            )
   460	        shape = cell.rsplit("_", 1)[1]
   461	        landed_root = raw["landed_root"]
   462	        expected_root = f"src_{shape}"
   463	        if landed_root != expected_root:
   464	            raise AnalysisError(
   465	                f"runs.csv line {line}: landed_root must be {expected_root!r}"
   466	            )
   467	        recorded_digest = raw["tree_manifest_sha256"]
   468	        if not SHA256_RE.fullmatch(recorded_digest):
   469	            raise AnalysisError(
   470	                f"runs.csv line {line}: tree_manifest_sha256 must be 64 lowercase hex"
   471	            )
   472	        rid = f"b{block_spec.number}_{cell}_p{pair}_{role}"
   473	        landed_data, landed_digest = _read_tree_manifest(
   474	            root / "landed" / f"{rid}.manifest", f"landed manifest {rid}"
   475	        )
   476	        canonical_data, canonical_digest = fixture_manifests[shape]
   477	        if landed_digest != recorded_digest:
   478	            raise AnalysisError(f"runs.csv line {line}: landed manifest digest mismatch")
   479	        if landed_data != canonical_data or landed_digest != canonical_digest:
   480	            raise AnalysisError(
   481	                f"runs.csv line {line}: landed relative-path/size manifest "
   482	                f"does not match canonical src_{shape}"
   483	            )
   484	        rows.append(
   485	            RunRow(
   486	                csv_line=line,
   487	                schedule_index=index,
   488	                block=block_spec.number,
   489	                trace_state=block_spec.trace_state,
   490	                pass_name=block_spec.pass_name,
   491	                cell=cell,
   492	                role=role,
   493	                pair=pair,
   494	                role_order=role_order,
   495	                transfer_ms=transfer_ms,
   496	                settled_ms=settled_ms,
   497	                flush_ms=flush_ms,
   498	                total_ms=total_ms,
   499	                landed_root=landed_root,
   500	                tree_manifest_sha256=recorded_digest,

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/otp12pf_rigw_analyze.py | nl -ba | sed -n '501,900p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZcAzEYy7' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-4LWMKMBV' (errno=Operation not permitted)
   501	                exit_code=exit_code,
   502	                drain=raw["drain"],
   503	                valid=raw["valid"],
   504	                run_id=run_id,
   505	                session_id=session_id,
   506	                client_log=raw["client_log"],
   507	            )
   508	        )
   509
   510	    landed_dir = root / "landed"
   511	    expected_landed = {
   512	        f"b{row.block}_{row.cell}_p{row.pair}_{row.role}.manifest"
   513	        for row in rows
   514	    }
   515	    actual_landed = (
   516	        {
   517	            path.relative_to(landed_dir).as_posix()
   518	            for path in landed_dir.rglob("*")
   519	            if path.is_file()
   520	        }
   521	        if landed_dir.is_dir()
   522	        else set()
   523	    )
   524	    if actual_landed != expected_landed:
   525	        raise AnalysisError(
   526	            "landed manifest file inventory mismatch: expected exactly 128 registered "
   527	            f"files, got {len(actual_landed)}"
   528	        )
   529
   530	    block_ids: dict[int, set[str]] = {}
   531	    for row in rows:
   532	        block_ids.setdefault(row.block, set()).add(row.run_id)
   533	    for block, ids in sorted(block_ids.items()):
   534	        if len(ids) != 1:
   535	            raise AnalysisError(f"block {block}: run_id is not block-level: {sorted(ids)}")
   536	    run_ids = [next(iter(block_ids[block.number])) for block in BLOCKS]
   537	    if len(set(run_ids)) != len(run_ids):
   538	        raise AnalysisError("block run_id values must be unique across the four blocks")
   539
   540	    session_keys = [
   541	        (row.run_id, row.session_id)
   542	        for row in rows
   543	        if row.trace_state == "on" and row.cell in TCP_CELLS
   544	    ]
   545	    if len(session_keys) != len(set(session_keys)):
   546	        raise AnalysisError("trace-on TCP (run_id, session_id) values must be unique")
   547	    return rows
   548
   549
   550	def load_clock_samples(root: Path, rows: Sequence[RunRow]) -> list[ClockSample]:
   551	    path = root / "clock-samples.csv"
   552	    if not path.is_file():
   553	        raise AnalysisError(f"missing {path}")
   554	    with path.open(newline="", encoding="utf-8") as handle:
   555	        reader = csv.DictReader(handle)
   556	        if tuple(reader.fieldnames or ()) != CLOCK_FIELDS:
   557	            raise AnalysisError(
   558	                "clock-samples.csv header mismatch: expected "
   559	                + ",".join(CLOCK_FIELDS)
   560	                + "; got "
   561	                + ",".join(reader.fieldnames or ())
   562	            )
   563	        raw_samples = list(reader)
   564
   565	    expected = [
   566	        (run, phase, sample)
   567	        for run in rows
   568	        for phase in ("before", "after")
   569	        for sample in range(1, 4)
   570	    ]
   571	    if len(raw_samples) != len(expected):
   572	        raise AnalysisError(
   573	            "clock-samples.csv inventory incomplete: expected "
   574	            f"{len(expected)} samples (3 before + 3 after per arm), got {len(raw_samples)}"
   575	        )
   576
   577	    result: list[ClockSample] = []
   578	    for index, (raw, (run, phase, sample)) in enumerate(zip(raw_samples, expected)):
   579	        line = index + 2
   580	        actual_key = (
   581	            parse_int(raw["block"], "block", line, "clock-samples.csv"),
   582	            raw["run_id"],
   583	            raw["cell"],
   584	            parse_int(raw["pair"], "pair", line, "clock-samples.csv"),
   585	            raw["role"],
   586	            raw["phase"],
   587	            parse_int(raw["sample"], "sample", line, "clock-samples.csv"),
   588	        )
   589	        expected_key = (
   590	            run.block,
   591	            run.run_id,
   592	            run.cell,
   593	            run.pair,
   594	            run.role,
   595	            phase,
   596	            sample,
   597	        )
   598	        if actual_key != expected_key:
   599	            raise AnalysisError(
   600	                f"clock-samples.csv line {line}: schedule mismatch; expected "
   601	                f"{expected_key}, got {actual_key}"
   602	            )
   603	        q_before = parse_int(
   604	            raw["q_before_ns"], "q_before_ns", line, "clock-samples.csv"
   605	        )
   606	        windows = parse_int(
   607	            raw["windows_ns"], "windows_ns", line, "clock-samples.csv"
   608	        )
   609	        q_after = parse_int(
   610	            raw["q_after_ns"], "q_after_ns", line, "clock-samples.csv"
   611	        )
   612	        rtt = parse_int(raw["rtt_ns"], "rtt_ns", line, "clock-samples.csv")
   613	        offset = parse_int(
   614	            raw["offset_windows_minus_q_ns"],
   615	            "offset_windows_minus_q_ns",
   616	            line,
   617	            "clock-samples.csv",
   618	        )
   619	        if q_before <= 0 or windows <= 0 or q_after <= q_before:
   620	            raise AnalysisError(
   621	                f"clock-samples.csv line {line}: q/windows times must be positive and "
   622	                "q_before_ns < q_after_ns"
   623	            )
   624	        computed_rtt = q_after - q_before
   625	        computed_offset = windows - (q_before + computed_rtt // 2)
   626	        if rtt <= 0 or rtt != computed_rtt:
   627	            raise AnalysisError(
   628	                f"clock-samples.csv line {line}: rtt_ns mismatch; expected "
   629	                f"{computed_rtt}, got {rtt}"
   630	            )
   631	        if offset != computed_offset:
   632	            raise AnalysisError(
   633	                f"clock-samples.csv line {line}: offset mismatch; expected "
   634	                f"{computed_offset}, got {offset}"
   635	            )
   636	        result.append(
   637	            ClockSample(
   638	                csv_line=line,
   639	                run=run,
   640	                phase=phase,
   641	                sample=sample,
   642	                q_before_ns=q_before,
   643	                windows_ns=windows,
   644	                q_after_ns=q_after,
   645	                rtt_ns=rtt,
   646	                offset_ns=offset,
   647	            )
   648	        )
   649	    return result
   650
   651
   652	def _require_json_string(raw: dict[str, Any], name: str, where: str) -> None:
   653	    if not isinstance(raw.get(name), str) or not raw[name]:
   654	        raise AnalysisError(f"{where}: {name} must be a non-empty string")
   655
   656
   657	def _require_json_int(raw: dict[str, Any], name: str, where: str) -> None:
   658	    value = raw.get(name)
   659	    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
   660	        raise AnalysisError(f"{where}: {name} must be a non-negative integer")
   661
   662
   663	def load_trace_events(root: Path) -> list[TraceEvent]:
   664	    evidence_roots = (root / "trace", root / "client")
   665	    for evidence_root in evidence_roots:
   666	        if not evidence_root.is_dir():
   667	            raise AnalysisError(f"missing trace evidence directory: {evidence_root}")
   668	    paths: list[Path] = []
   669	    seen_resolved: set[Path] = set()
   670	    for evidence_root in evidence_roots:
   671	        for candidate in sorted(path for path in evidence_root.rglob("*") if path.is_file()):
   672	            resolved = candidate.resolve()
   673	            if resolved in seen_resolved:
   674	                continue
   675	            seen_resolved.add(resolved)
   676	            paths.append(candidate)
   677	    events: list[TraceEvent] = []
   678	    for path in paths:
   679	        relative = path.relative_to(root).as_posix()
   680	        try:
   681	            handle = path.open(encoding="utf-8")
   682	            with handle:
   683	                for line_number, line in enumerate(handle, start=1):
   684	                    if not line.startswith(TRACE_PREFIX):
   685	                        continue
   686	                    payload = line[len(TRACE_PREFIX) :].rstrip("\r\n")
   687	                    where = f"{relative}:{line_number}"
   688	                    try:
   689	                        raw = json.loads(payload)
   690	                    except json.JSONDecodeError as exc:
   691	                        raise AnalysisError(f"{where}: malformed session-phase JSON: {exc}") from exc
   692	                    if not isinstance(raw, dict):
   693	                        raise AnalysisError(f"{where}: session-phase payload is not an object")
   694	                    if raw.get("schema") != 1 or isinstance(raw.get("schema"), bool):
   695	                        raise AnalysisError(f"{where}: unsupported session-phase schema")
   696	                    for name in (
   697	                        "run_id",
   698	                        "session_id",
   699	                        "endpoint_role",
   700	                        "initiator_role",
   701	                        "event",
   702	                    ):
   703	                        _require_json_string(raw, name, where)
   704	                    for name in ("producer_seq", "unix_ns", "elapsed_ns"):
   705	                        _require_json_int(raw, name, where)
   706	                    if not SESSION_ID_RE.fullmatch(raw["session_id"]):
   707	                        raise AnalysisError(f"{where}: session_id is not 16 lowercase hex")
   708	                    if raw["endpoint_role"] not in ("SOURCE", "DESTINATION"):
   709	                        raise AnalysisError(f"{where}: invalid endpoint_role")
   710	                    if raw["initiator_role"] not in ("SOURCE", "DESTINATION"):
   711	                        raise AnalysisError(f"{where}: invalid initiator_role")
   712	                    for name in (
   713	                        "epoch",
   714	                        "socket",
   715	                        "batch",
   716	                        "count",
   717	                        "target_streams",
   718	                        "live_streams",
   719	                    ):
   720	                        if name in raw:
   721	                            _require_json_int(raw, name, where)
   722	                    if "accepted" in raw and not isinstance(raw["accepted"], bool):
   723	                        raise AnalysisError(f"{where}: accepted must be boolean")
   724	                    events.append(TraceEvent(relative, line_number, raw))
   725	        except UnicodeDecodeError as exc:
   726	            raise AnalysisError(f"{relative}: trace log is not UTF-8") from exc
   727	    return events
   728
   729
   730	def _one_event(events: Sequence[TraceEvent], role: str, name: str, label: str) -> TraceEvent:
   731	    found = [event for event in events if event.endpoint_role == role and event.event == name]
   732	    if len(found) != 1:
   733	        raise AnalysisError(f"{label}: expected one {role}/{name}, got {len(found)}")
   734	    return found[0]
   735
   736
   737	def _correlation_keys(
   738	    events: Sequence[TraceEvent], role: str, name: str, label: str
   739	) -> set[tuple[int, int]]:
   740	    selected = [event for event in events if event.endpoint_role == role and event.event == name]
   741	    keys: list[tuple[int, int]] = []
   742	    for event in selected:
   743	        epoch = event.raw.get("epoch")
   744	        socket = event.raw.get("socket")
   745	        if not isinstance(epoch, int) or not isinstance(socket, int):
   746	            raise AnalysisError(f"{label}: {role}/{name} lacks epoch/socket correlation")
   747	        keys.append((epoch, socket))
   748	    if len(keys) != len(set(keys)):
   749	        raise AnalysisError(f"{label}: duplicate {role}/{name} epoch/socket marker")
   750	    return set(keys)
   751
   752
   753	def _marker_map(
   754	    events: Sequence[TraceEvent],
   755	    role: str,
   756	    name: str,
   757	    key_fields: tuple[str, ...],
   758	    label: str,
   759	    *,
   760	    required: bool = True,
   761	) -> dict[tuple[int, ...], TraceEvent]:
   762	    selected = [event for event in events if event.endpoint_role == role and event.event == name]
   763	    if required and not selected:
   764	        raise AnalysisError(f"{label}: missing {role}/{name} inventory")
   765	    result: dict[tuple[int, ...], TraceEvent] = {}
   766	    for event in selected:
   767	        values: list[int] = []
   768	        for field in key_fields:
   769	            value = event.raw.get(field)
   770	            if isinstance(value, bool) or not isinstance(value, int) or value < 0:
   771	                raise AnalysisError(f"{label}: {role}/{name} lacks {field} correlation")
   772	            values.append(value)
   773	        key = tuple(values)
   774	        if key in result:
   775	            raise AnalysisError(f"{label}: duplicate {role}/{name} marker for {key}")
   776	        result[key] = event
   777	    return result
   778
   779
   780	def _assert_same_keys(
   781	    label: str, named_maps: Sequence[tuple[str, dict[tuple[int, ...], TraceEvent]]]
   782	) -> set[tuple[int, ...]]:
   783	    first_name, first = named_maps[0]
   784	    wanted = set(first)
   785	    for name, markers in named_maps[1:]:
   786	        if set(markers) != wanted:
   787	            raise AnalysisError(
   788	                f"{label}: correlation mismatch {first_name}={sorted(wanted)} "
   789	                f"vs {name}={sorted(markers)}"
   790	            )
   791	    return wanted
   792
   793
   794	def _assert_before(label: str, start: TraceEvent, end: TraceEvent) -> None:
   795	    if (
   796	        start.endpoint_role != end.endpoint_role
   797	        or start.producer_seq >= end.producer_seq
   798	        or start.elapsed_ns > end.elapsed_ns
   799	    ):
   800	        raise AnalysisError(
   801	            f"{label}: invalid local sequence {start.endpoint_role}/{start.event} "
   802	            f"-> {end.endpoint_role}/{end.event}"
   803	        )
   804
   805
   806	def _assert_event_fields(
   807	    label: str, event: TraceEvent, expected: dict[str, Any]
   808	) -> None:
   809	    epoch = event.raw.get("epoch")
   810	    for field, wanted in expected.items():
   811	        actual = event.raw.get(field)
   812	        if actual != wanted:
   813	            raise AnalysisError(
   814	                f"{label}: {event.endpoint_role}/{event.event} epoch {epoch} "
   815	                f"{field} must be {wanted!r}, got {actual!r}"
   816	            )
   817
   818
   819	def validate_traces(
   820	    rows: Sequence[RunRow], events: Sequence[TraceEvent]
   821	) -> dict[tuple[str, str], list[TraceEvent]]:
   822	    block_by_run: dict[str, int] = {}
   823	    for row in rows:
   824	        block_by_run[row.run_id] = row.block
   825	    expected_rows = {
   826	        (row.run_id, row.session_id): row
   827	        for row in rows
   828	        if row.trace_state == "on" and row.cell in TCP_CELLS
   829	    }
   830	    grouped: dict[tuple[str, str], list[TraceEvent]] = {}
   831	    for event in events:
   832	        key = (event.run_id, event.session_id)
   833	        if key not in expected_rows:
   834	            if event.run_id in block_by_run:
   835	                block = block_by_run[event.run_id]
   836	                state = next(row.trace_state for row in rows if row.block == block)
   837	                if state == "off":
   838	                    raise AnalysisError(
   839	                        f"trace leak: trace-off block {block} emitted {event.session_id} "
   840	                        f"at {event.source_file}:{event.source_line}"
   841	                    )
   842	                raise AnalysisError(
   843	                    f"trace leak: block {block} emitted an unregistered (including possible "
   844	                    f"gRPC) session {event.session_id} at "
   845	                    f"{event.source_file}:{event.source_line}"
   846	                )
   847	            raise AnalysisError(
   848	                f"stale/foreign trace run_id {event.run_id!r} at "
   849	                f"{event.source_file}:{event.source_line}"
   850	            )
   851	        grouped.setdefault(key, []).append(event)
   852
   853	    missing = sorted(set(expected_rows) - set(grouped))
   854	    if missing:
   855	        run_id, session_id = missing[0]
   856	        row = expected_rows[(run_id, session_id)]
   857	        raise AnalysisError(
   858	            f"missing trace for block {row.block} {row.cell} pair {row.pair} "
   859	            f"{row.role} ({run_id}/{session_id}); {len(missing)} session(s) missing"
   860	        )
   861
   862	    for key, row in expected_rows.items():
   863	        group = grouped[key]
   864	        label = (
   865	            f"block {row.block} {row.cell} pair {row.pair} {row.role} "
   866	            f"({row.run_id}/{row.session_id})"
   867	        )
   868	        expected_initiator = "SOURCE" if row.role == "source_init" else "DESTINATION"
   869	        roles = {event.endpoint_role for event in group}
   870	        if roles != {"SOURCE", "DESTINATION"}:
   871	            raise AnalysisError(
   872	                f"{label}: missing endpoint role; expected SOURCE+DESTINATION, got {sorted(roles)}"
   873	            )
   874	        if {event.raw["initiator_role"] for event in group} != {expected_initiator}:
   875	            raise AnalysisError(f"{label}: initiator_role does not match scheduled role")
   876
   877	        by_role: dict[str, list[TraceEvent]] = {}
   878	        for event in group:
   879	            by_role.setdefault(event.endpoint_role, []).append(event)
   880	        for endpoint_role, endpoint_events in by_role.items():
   881	            seqs = sorted(event.producer_seq for event in endpoint_events)
   882	            if seqs != list(range(len(endpoint_events))):
   883	                raise AnalysisError(
   884	                    f"{label}: {endpoint_role} producer_seq is not exact contiguous 0..n-1: "
   885	                    f"{seqs}"
   886	                )
   887
   888	        manifest_begin = _one_event(
   889	            group, "SOURCE", "manifest_complete_send_begin", label
   890	        )
   891	        manifest_sent = _one_event(group, "SOURCE", "manifest_complete_sent", label)
   892	        _one_event(group, "DESTINATION", "manifest_complete_received", label)
   893	        first_queued = _one_event(group, "SOURCE", "first_payload_queued", label)
   894	        _assert_before(label, manifest_begin, manifest_sent)
   895	        _assert_before(label, manifest_sent, first_queued)
   896
   897	        need_begin = _marker_map(
   898	            group, "DESTINATION", "need_batch_send_begin", ("batch",), label
   899	        )
   900	        need_sent = _marker_map(

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/otp12pf_rigw_analyze.py | nl -ba | sed -n '888,1320p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-UuQ7QQ91' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-3ASEk7PE' (errno=Operation not permitted)
   888	        manifest_begin = _one_event(
   889	            group, "SOURCE", "manifest_complete_send_begin", label
   890	        )
   891	        manifest_sent = _one_event(group, "SOURCE", "manifest_complete_sent", label)
   892	        _one_event(group, "DESTINATION", "manifest_complete_received", label)
   893	        first_queued = _one_event(group, "SOURCE", "first_payload_queued", label)
   894	        _assert_before(label, manifest_begin, manifest_sent)
   895	        _assert_before(label, manifest_sent, first_queued)
   896
   897	        need_begin = _marker_map(
   898	            group, "DESTINATION", "need_batch_send_begin", ("batch",), label
   899	        )
   900	        need_sent = _marker_map(
   901	            group, "DESTINATION", "need_batch_sent", ("batch",), label
   902	        )
   903	        need_received = _marker_map(
   904	            group, "SOURCE", "need_batch_received", ("batch",), label
   905	        )
   906	        need_keys = _assert_same_keys(
   907	            label,
   908	            (
   909	                ("need_send_begin", need_begin),
   910	                ("need_sent", need_sent),
   911	                ("need_received", need_received),
   912	            ),
   913	        )
   914	        if need_keys != {(batch,) for batch in range(len(need_keys))}:
   915	            raise AnalysisError(f"{label}: need batch correlation is not contiguous from zero")
   916	        for key in need_keys:
   917	            _assert_before(label, need_begin[key], need_sent[key])
   918	            counts = {
   919	                need_begin[key].raw.get("count"),
   920	                need_sent[key].raw.get("count"),
   921	                need_received[key].raw.get("count"),
   922	            }
   923	            if len(counts) != 1 or not all(
   924	                isinstance(value, int) and not isinstance(value, bool) and value > 0
   925	                for value in counts
   926	            ):
   927	                raise AnalysisError(f"{label}: need batch {key[0]} count correlation mismatch")
   928
   929	        planner_begin = _marker_map(
   930	            group, "SOURCE", "planner_begin", ("batch",), label
   931	        )
   932	        planner_end = _marker_map(group, "SOURCE", "planner_end", ("batch",), label)
   933	        planner_keys = _assert_same_keys(
   934	            label, (("planner_begin", planner_begin), ("planner_end", planner_end))
   935	        )
   936	        if planner_keys != {(batch,) for batch in range(len(planner_keys))}:
   937	            raise AnalysisError(f"{label}: planner batch correlation is not contiguous from zero")
   938	        for key in planner_keys:
   939	            _assert_before(label, planner_begin[key], planner_end[key])
   940	        earliest_planner_end = min(
   941	            planner_end.values(), key=lambda event: event.producer_seq
   942	        )
   943	        _assert_before(label, earliest_planner_end, first_queued)
   944
   945	        resize_maps = (
   946	            ("resize_proposed", _marker_map(group, "SOURCE", "resize_proposed", ("epoch",), label)),
   947	            (
   948	                "resize_send_begin",
   949	                _marker_map(group, "SOURCE", "resize_send_begin", ("epoch",), label),
   950	            ),
   951	            ("resize_sent", _marker_map(group, "SOURCE", "resize_sent", ("epoch",), label)),
   952	            (
   953	                "resize_received",
   954	                _marker_map(group, "DESTINATION", "resize_received", ("epoch",), label),
   955	            ),
   956	            (
   957	                "destination_prepared",
   958	                _marker_map(group, "DESTINATION", "destination_prepared", ("epoch",), label),
   959	            ),
   960	            (
   961	                "resize_ack_send_begin",
   962	                _marker_map(
   963	                    group, "DESTINATION", "resize_ack_send_begin", ("epoch",), label
   964	                ),
   965	            ),
   966	            (
   967	                "resize_ack_sent",
   968	                _marker_map(group, "DESTINATION", "resize_ack_sent", ("epoch",), label),
   969	            ),
   970	            (
   971	                "resize_ack_received",
   972	                _marker_map(group, "SOURCE", "resize_ack_received", ("epoch",), label),
   973	            ),
   974	            (
   975	                "source_settled",
   976	                _marker_map(group, "SOURCE", "source_settled", ("epoch",), label),
   977	            ),
   978	        )
   979	        resize_epochs = _assert_same_keys(label, resize_maps)
   980	        expected_resize_epochs = {(epoch,) for epoch in range(1, 8)}
   981	        if resize_epochs != expected_resize_epochs:
   982	            raise AnalysisError(
   983	                f"{label}: resize epochs must be exactly 1..7, got "
   984	                f"{sorted(epoch[0] for epoch in resize_epochs)}"
   985	            )
   986	        resize = dict(resize_maps)
   987	        expected_prepared_action = (
   988	            "arm_queued" if expected_initiator == "SOURCE" else "dial_complete"
   989	        )
   990	        for key in sorted(resize_epochs):
   991	            epoch = key[0]
   992	            target = epoch + 1
   993	            _assert_before(label, resize["resize_proposed"][key], resize["resize_send_begin"][key])
   994	            _assert_before(label, resize["resize_send_begin"][key], resize["resize_sent"][key])
   995	            _assert_before(
   996	                label,
   997	                resize["resize_received"][key],
   998	                resize["destination_prepared"][key],
   999	            )
  1000	            _assert_before(
  1001	                label,
  1002	                resize["destination_prepared"][key],
  1003	                resize["resize_ack_send_begin"][key],
  1004	            )
  1005	            _assert_before(
  1006	                label,
  1007	                resize["resize_ack_send_begin"][key],
  1008	                resize["resize_ack_sent"][key],
  1009	            )
  1010	            _assert_before(
  1011	                label, resize["resize_ack_received"][key], resize["source_settled"][key]
  1012	            )
  1013	            for name in ("resize_proposed", "resize_send_begin", "resize_sent"):
  1014	                _assert_event_fields(
  1015	                    label,
  1016	                    resize[name][key],
  1017	                    {"target_streams": target, "live_streams": epoch},
  1018	                )
  1019	            _assert_event_fields(
  1020	                label,
  1021	                resize["resize_received"][key],
  1022	                {"target_streams": target, "live_streams": epoch},
  1023	            )
  1024	            _assert_event_fields(
  1025	                label,
  1026	                resize["destination_prepared"][key],
  1027	                {"target_streams": target},
  1028	            )
  1029	            for name in ("resize_ack_send_begin", "resize_ack_sent"):
  1030	                _assert_event_fields(
  1031	                    label,
  1032	                    resize[name][key],
  1033	                    {"accepted": True, "live_streams": target},
  1034	                )
  1035	            _assert_event_fields(
  1036	                label,
  1037	                resize["resize_ack_received"][key],
  1038	                {"accepted": True, "live_streams": target},
  1039	            )
  1040	            _assert_event_fields(
  1041	                label,
  1042	                resize["source_settled"][key],
  1043	                {
  1044	                    "accepted": True,
  1045	                    "target_streams": target,
  1046	                    "live_streams": target,
  1047	                },
  1048	            )
  1049	            if resize["destination_prepared"][key].raw.get("action") != expected_prepared_action:
  1050	                raise AnalysisError(
  1051	                    f"{label}: resize epoch {key[0]} destination_prepared action must be "
  1052	                    f"{expected_prepared_action}"
  1053	                )
  1054	        for epoch in range(1, 7):
  1055	            _assert_before(
  1056	                label,
  1057	                resize["source_settled"][(epoch,)],
  1058	                resize["resize_proposed"][(epoch + 1,)],
  1059	            )
  1060	            _assert_before(
  1061	                label,
  1062	                resize["resize_ack_sent"][(epoch,)],
  1063	                resize["resize_received"][(epoch + 1,)],
  1064	            )
  1065
  1066	        source_complete = _one_event(group, "SOURCE", "data_plane_complete", label)
  1067	        source_summary = _one_event(group, "SOURCE", "summary_received", label)
  1068	        destination_complete = _one_event(group, "DESTINATION", "data_plane_complete", label)
  1069	        destination_summary_begin = _one_event(
  1070	            group, "DESTINATION", "summary_send_begin", label
  1071	        )
  1072	        destination_summary = _one_event(group, "DESTINATION", "summary_sent", label)
  1073	        if source_complete.producer_seq >= source_summary.producer_seq:
  1074	            raise AnalysisError(f"{label}: SOURCE terminal inventory is out of sequence")
  1075	        if not (
  1076	            destination_complete.producer_seq
  1077	            < destination_summary_begin.producer_seq
  1078	            < destination_summary.producer_seq
  1079	        ):
  1080	            raise AnalysisError(f"{label}: DESTINATION terminal inventory is out of sequence")
  1081	        _assert_before(label, resize["source_settled"][(7,)], source_complete)
  1082	        _assert_before(label, resize["resize_ack_sent"][(7,)], destination_complete)
  1083
  1084	        source_attachment_events = _marker_map(
  1085	            group,
  1086	            "SOURCE",
  1087	            "socket_trace_attached",
  1088	            ("epoch", "socket"),
  1089	            label,
  1090	        )
  1091	        destination_attachment_events = _marker_map(
  1092	            group,
  1093	            "DESTINATION",
  1094	            "socket_trace_attached",
  1095	            ("epoch", "socket"),
  1096	            label,
  1097	        )
  1098	        source_attached = set(source_attachment_events)
  1099	        destination_attached = set(destination_attachment_events)
  1100	        if not source_attached or source_attached != destination_attached:
  1101	            raise AnalysisError(f"{label}: two-role socket attachment correlation mismatch")
  1102	        expected_attached = {(0, 0)} | {(epoch[0], 0) for epoch in resize_epochs}
  1103	        if source_attached != expected_attached:
  1104	            raise AnalysisError(
  1105	                f"{label}: socket attachment inventory {sorted(source_attached)} does not "
  1106	                f"match epoch-0 plus accepted resize epochs {sorted(expected_attached)}"
  1107	            )
  1108	        for endpoint_role, complete in (
  1109	            ("SOURCE", source_complete),
  1110	            ("DESTINATION", destination_complete),
  1111	        ):
  1112	            for attached in (
  1113	                event
  1114	                for event in group
  1115	                if event.endpoint_role == endpoint_role
  1116	                and event.event == "socket_trace_attached"
  1117	            ):
  1118	                _assert_before(label, attached, complete)
  1119
  1120	        source_action = "dial" if expected_initiator == "SOURCE" else "accept"
  1121	        destination_action = "accept" if expected_initiator == "SOURCE" else "dial"
  1122	        action_events: dict[
  1123	            str, tuple[dict[tuple[int, ...], TraceEvent], dict[tuple[int, ...], TraceEvent]]
  1124	        ] = {}
  1125	        for endpoint_role, action in (
  1126	            ("SOURCE", source_action),
  1127	            ("DESTINATION", destination_action),
  1128	        ):
  1129	            begins = _marker_map(
  1130	                group,
  1131	                endpoint_role,
  1132	                f"socket_{action}_begin",
  1133	                ("epoch", "socket"),
  1134	                label,
  1135	            )
  1136	            ends = _marker_map(
  1137	                group,
  1138	                endpoint_role,
  1139	                f"socket_{action}_end",
  1140	                ("epoch", "socket"),
  1141	                label,
  1142	            )
  1143	            action_keys = _assert_same_keys(
  1144	                label,
  1145	                ((f"{endpoint_role}_{action}_begin", begins), (f"{endpoint_role}_{action}_end", ends)),
  1146	            )
  1147	            if action_keys != expected_attached:
  1148	                raise AnalysisError(f"{label}: {endpoint_role} socket action inventory mismatch")
  1149	            other_action = "accept" if action == "dial" else "dial"
  1150	            if _marker_map(
  1151	                group,
  1152	                endpoint_role,
  1153	                f"socket_{other_action}_begin",
  1154	                ("epoch", "socket"),
  1155	                label,
  1156	                required=False,
  1157	            ):
  1158	                raise AnalysisError(
  1159	                    f"{label}: {endpoint_role} unexpectedly mixed dial and accept actions"
  1160	                )
  1161	            attachments = (
  1162	                source_attachment_events
  1163	                if endpoint_role == "SOURCE"
  1164	                else destination_attachment_events
  1165	            )
  1166	            for action_key in action_keys:
  1167	                _assert_before(label, begins[action_key], ends[action_key])
  1168	                _assert_before(label, ends[action_key], attachments[action_key])
  1169	            action_events[endpoint_role] = (begins, ends)
  1170
  1171	        source_action_begins, source_action_ends = action_events["SOURCE"]
  1172	        destination_action_begins, destination_action_ends = action_events[
  1173	            "DESTINATION"
  1174	        ]
  1175	        for (epoch,) in sorted(resize_epochs):
  1176	            action_key = (epoch, 0)
  1177	            _assert_before(
  1178	                label,
  1179	                resize["resize_sent"][(epoch,)],
  1180	                source_action_begins[action_key],
  1181	            )
  1182	            _assert_before(
  1183	                label,
  1184	                resize["resize_ack_received"][(epoch,)],
  1185	                source_action_begins[action_key],
  1186	            )
  1187	            _assert_before(
  1188	                label,
  1189	                source_action_ends[action_key],
  1190	                resize["source_settled"][(epoch,)],
  1191	            )
  1192	            _assert_before(
  1193	                label,
  1194	                source_attachment_events[action_key],
  1195	                resize["source_settled"][(epoch,)],
  1196	            )
  1197
  1198	        arm_begin = _marker_map(
  1199	            group,
  1200	            "DESTINATION",
  1201	            "resize_arm_queue_begin",
  1202	            ("epoch",),
  1203	            label,
  1204	            required=False,
  1205	        )
  1206	        arm_ready = _marker_map(
  1207	            group,
  1208	            "DESTINATION",
  1209	            "resize_arm_ready",
  1210	            ("epoch",),
  1211	            label,
  1212	            required=False,
  1213	        )
  1214	        if expected_initiator == "SOURCE":
  1215	            arm_epochs = _assert_same_keys(
  1216	                label, (("resize_arm_queue_begin", arm_begin), ("resize_arm_ready", arm_ready))
  1217	            )
  1218	            if arm_epochs != resize_epochs:
  1219	                raise AnalysisError(f"{label}: destination resize-arm inventory mismatch")
  1220	            for arm_key in arm_epochs:
  1221	                _assert_event_fields(
  1222	                    label,
  1223	                    arm_begin[arm_key],
  1224	                    {"target_streams": arm_key[0] + 1},
  1225	                )
  1226	                _assert_before(
  1227	                    label,
  1228	                    resize["resize_received"][arm_key],
  1229	                    arm_begin[arm_key],
  1230	                )
  1231	                _assert_before(
  1232	                    label,
  1233	                    arm_begin[arm_key],
  1234	                    resize["destination_prepared"][arm_key],
  1235	                )
  1236	                _assert_before(label, arm_begin[arm_key], arm_ready[arm_key])
  1237	                _assert_before(
  1238	                    label,
  1239	                    arm_ready[arm_key],
  1240	                    destination_action_begins[(arm_key[0], 0)],
  1241	                )
  1242	                _assert_before(
  1243	                    label,
  1244	                    arm_begin[arm_key],
  1245	                    destination_action_begins[(arm_key[0], 0)],
  1246	                )
  1247	        elif arm_begin or arm_ready:
  1248	            raise AnalysisError(f"{label}: destination initiator unexpectedly emitted arm events")
  1249	        else:
  1250	            for (epoch,) in sorted(resize_epochs):
  1251	                _assert_before(
  1252	                    label,
  1253	                    resize["resize_received"][(epoch,)],
  1254	                    destination_action_begins[(epoch, 0)],
  1255	                )
  1256	                _assert_before(
  1257	                    label,
  1258	                    destination_action_ends[(epoch, 0)],
  1259	                    resize["destination_prepared"][(epoch,)],
  1260	                )
  1261	                _assert_before(
  1262	                    label,
  1263	                    destination_attachment_events[(epoch, 0)],
  1264	                    resize["destination_prepared"][(epoch,)],
  1265	                )
  1266	        write_begin_events = _marker_map(
  1267	            group,
  1268	            "SOURCE",
  1269	            "socket_write_begin",
  1270	            ("epoch", "socket"),
  1271	            label,
  1272	        )
  1273	        write_events = _marker_map(
  1274	            group,
  1275	            "SOURCE",
  1276	            "first_socket_write",
  1277	            ("epoch", "socket"),
  1278	            label,
  1279	        )
  1280	        receive_events = _marker_map(
  1281	            group,
  1282	            "DESTINATION",
  1283	            "first_payload_received",
  1284	            ("epoch", "socket"),
  1285	            label,
  1286	        )
  1287	        write_begins = set(write_begin_events)
  1288	        writes = set(write_events)
  1289	        receives = set(receive_events)
  1290	        if not writes or write_begins != writes or writes != receives:
  1291	            raise AnalysisError(f"{label}: payload socket correlation mismatch")
  1292	        if not writes.issubset(source_attached):
  1293	            raise AnalysisError(f"{label}: SOURCE payload socket was not trace-attached")
  1294	        if not receives.issubset(destination_attached):
  1295	            raise AnalysisError(f"{label}: DESTINATION payload socket was not trace-attached")
  1296	        for action_key in writes:
  1297	            begin = write_begin_events[action_key]
  1298	            write = write_events[action_key]
  1299	            received = receive_events[action_key]
  1300	            _assert_before(label, first_queued, write)
  1301	            _assert_before(label, source_attachment_events[action_key], begin)
  1302	            _assert_before(label, begin, write)
  1303	            _assert_before(label, write, source_complete)
  1304	            _assert_before(
  1305	                label, destination_attachment_events[action_key], received
  1306	            )
  1307	            _assert_before(label, received, destination_complete)
  1308	    return grouped
  1309
  1310
  1311	def condition_stats(rows: Sequence[RunRow], cell: str, trace_state: str) -> ConditionStats:
  1312	    selected = [
  1313	        row for row in rows if row.cell == cell and row.trace_state == trace_state
  1314	    ]
  1315	    by_pair: dict[int, dict[str, Decimal]] = {}
  1316	    for row in selected:
  1317	        if row.role in by_pair.setdefault(row.pair, {}):
  1318	            raise AnalysisError(
  1319	                f"duplicate timing for {cell}/{trace_state}/pair {row.pair}/{row.role}"
  1320	            )

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/otp12pf_rigw_analyze.py | nl -ba | sed -n '1311,1758p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-pWLSuMUC' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-cl0iNM5U' (errno=Operation not permitted)
  1311	def condition_stats(rows: Sequence[RunRow], cell: str, trace_state: str) -> ConditionStats:
  1312	    selected = [
  1313	        row for row in rows if row.cell == cell and row.trace_state == trace_state
  1314	    ]
  1315	    by_pair: dict[int, dict[str, Decimal]] = {}
  1316	    for row in selected:
  1317	        if row.role in by_pair.setdefault(row.pair, {}):
  1318	            raise AnalysisError(
  1319	                f"duplicate timing for {cell}/{trace_state}/pair {row.pair}/{row.role}"
  1320	            )
  1321	        by_pair[row.pair][row.role] = row.total_ms
  1322	    if sorted(by_pair) != list(range(1, 9)):
  1323	        raise AnalysisError(
  1324	            f"{cell}/{trace_state}: expected paired observations 1..8, got {sorted(by_pair)}"
  1325	        )
  1326	    for pair, arms in by_pair.items():
  1327	        if set(arms) != set(ROLES):
  1328	            raise AnalysisError(f"{cell}/{trace_state}/pair {pair}: incomplete role pair")
  1329	    source = tuple(by_pair[pair]["source_init"] for pair in range(1, 9))
  1330	    destination = tuple(by_pair[pair]["destination_init"] for pair in range(1, 9))
  1331	    deltas = tuple(dest - src for src, dest in zip(source, destination))
  1332	    source_first_pairs = {
  1333	        row.pair for row in selected if row.role == "source_init" and row.role_order == 1
  1334	    }
  1335	    destination_first_pairs = {
  1336	        row.pair
  1337	        for row in selected
  1338	        if row.role == "destination_init" and row.role_order == 1
  1339	    }
  1340	    if source_first_pairs | destination_first_pairs != set(range(1, 9)):
  1341	        raise AnalysisError(f"{cell}/{trace_state}: incomplete role-order partition")
  1342	    if source_first_pairs & destination_first_pairs:
  1343	        raise AnalysisError(f"{cell}/{trace_state}: overlapping role-order partition")
  1344	    source_first = median(
  1345	        tuple(deltas[pair - 1] for pair in sorted(source_first_pairs))
  1346	    )
  1347	    destination_first = median(
  1348	        tuple(deltas[pair - 1] for pair in sorted(destination_first_pairs))
  1349	    )
  1350	    first4 = median(deltas[:4])
  1351	    last4 = median(deltas[4:])
  1352	    odd = median(deltas[0::2])
  1353	    even = median(deltas[1::2])
  1354	    source_median = median(source)
  1355	    destination_median = median(destination)
  1356	    return ConditionStats(
  1357	        cell=cell,
  1358	        trace_state=trace_state,
  1359	        source_values=source,
  1360	        destination_values=destination,
  1361	        paired_deltas=deltas,
  1362	        source_median=source_median,
  1363	        destination_median=destination_median,
  1364	        delta=destination_median - source_median,
  1365	        paired_delta_median=median(deltas),
  1366	        first4_delta_median=first4,
  1367	        last4_delta_median=last4,
  1368	        first_last_drift=abs(first4 - last4),
  1369	        odd_delta_median=odd,
  1370	        even_delta_median=even,
  1371	        odd_even_drift=abs(odd - even),
  1372	        source_first_delta_median=source_first,
  1373	        destination_first_delta_median=destination_first,
  1374	        role_order_drift=abs(source_first - destination_first),
  1375	        paired_delta_range=max(deltas) - min(deltas),
  1376	        n_pair_split=max(abs(first4 - last4), abs(odd - even)),
  1377	        n_pair=max(
  1378	            abs(first4 - last4),
  1379	            abs(odd - even),
  1380	            abs(source_first - destination_first),
  1381	            max(deltas) - min(deltas),
  1382	        ),
  1383	    )
  1384
  1385
  1386	def largest_gap_modes(values: Iterable[Decimal]) -> ModeDescription:
  1387	    ordered = tuple(sorted(values))
  1388	    if not ordered:
  1389	        raise AnalysisError("cannot describe an empty distribution")
  1390	    gaps = tuple(right - left for left, right in zip(ordered, ordered[1:]))
  1391	    if not gaps or max(gaps) == 0:
  1392	        return ModeDescription(Decimal(0), ordered, ())
  1393	    gap = max(gaps)
  1394	    split = gaps.index(gap) + 1
  1395	    return ModeDescription(gap, ordered[:split], ordered[split:])
  1396
  1397
  1398	def _atomic_csv(path: Path, fields: Sequence[str], rows: Iterable[dict[str, Any]]) -> None:
  1399	    with tempfile.NamedTemporaryFile(
  1400	        mode="w", newline="", encoding="utf-8", dir=path.parent, delete=False
  1401	    ) as handle:
  1402	        temporary = Path(handle.name)
  1403	        writer = csv.DictWriter(handle, fieldnames=fields, extrasaction="raise")
  1404	        writer.writeheader()
  1405	        writer.writerows(rows)
  1406	    os.replace(temporary, path)
  1407
  1408
  1409	def _atomic_text(path: Path, contents: str) -> None:
  1410	    with tempfile.NamedTemporaryFile(
  1411	        mode="w", encoding="utf-8", dir=path.parent, delete=False
  1412	    ) as handle:
  1413	        temporary = Path(handle.name)
  1414	        handle.write(contents)
  1415	    os.replace(temporary, path)
  1416
  1417
  1418	def _summary_rows(
  1419	    stats: Sequence[ConditionStats], observer_bias: Decimal, n_resolution: Decimal
  1420	) -> list[dict[str, str]]:
  1421	    result: list[dict[str, str]] = []
  1422	    for item in stats:
  1423	        source_modes = largest_gap_modes(item.source_values)
  1424	        destination_modes = largest_gap_modes(item.destination_values)
  1425	        delta_modes = largest_gap_modes(item.paired_deltas)
  1426	        target = item.cell == TARGET_CELL
  1427	        result.append(
  1428	            {
  1429	                "cell": item.cell,
  1430	                "trace_state": item.trace_state,
  1431	                "measurand": MEASURAND,
  1432	                "pairs": "8",
  1433	                "source_init_median_ms": decimal_text(item.source_median),
  1434	                "destination_init_median_ms": decimal_text(item.destination_median),
  1435	                "delta_ms": decimal_text(item.delta),
  1436	                "paired_delta_median_ms": decimal_text(item.paired_delta_median),
  1437	                "first4_delta_median_ms": decimal_text(item.first4_delta_median),
  1438	                "last4_delta_median_ms": decimal_text(item.last4_delta_median),
  1439	                "first_last_drift_ms": decimal_text(item.first_last_drift),
  1440	                "odd_delta_median_ms": decimal_text(item.odd_delta_median),
  1441	                "even_delta_median_ms": decimal_text(item.even_delta_median),
  1442	                "odd_even_drift_ms": decimal_text(item.odd_even_drift),
  1443	                "source_first_delta_median_ms": decimal_text(
  1444	                    item.source_first_delta_median
  1445	                ),
  1446	                "destination_first_delta_median_ms": decimal_text(
  1447	                    item.destination_first_delta_median
  1448	                ),
  1449	                "role_order_drift_ms": decimal_text(item.role_order_drift),
  1450	                "paired_delta_range_ms": decimal_text(item.paired_delta_range),
  1451	                "n_pair_split_ms": decimal_text(item.n_pair_split),
  1452	                "n_pair_ms": decimal_text(item.n_pair),
  1453	                "observer_bias_ms": decimal_text(observer_bias) if target else "",
  1454	                "n_resolution_ms": decimal_text(n_resolution) if target else "",
  1455	                "source_init_sorted_ms": ";".join(
  1456	                    decimal_text(value) for value in sorted(item.source_values)
  1457	                ),
  1458	                "destination_init_sorted_ms": ";".join(
  1459	                    decimal_text(value) for value in sorted(item.destination_values)
  1460	                ),
  1461	                "paired_delta_sorted_ms": ";".join(
  1462	                    decimal_text(value) for value in sorted(item.paired_deltas)
  1463	                ),
  1464	                "source_init_largest_gap_ms": decimal_text(source_modes.gap),
  1465	                "source_init_descriptive_modes_ms": source_modes.render(),
  1466	                "destination_init_largest_gap_ms": decimal_text(destination_modes.gap),
  1467	                "destination_init_descriptive_modes_ms": destination_modes.render(),
  1468	                "paired_delta_largest_gap_ms": decimal_text(delta_modes.gap),
  1469	                "paired_delta_descriptive_modes_ms": delta_modes.render(),
  1470	            }
  1471	        )
  1472	    return result
  1473
  1474
  1475	SUMMARY_FIELDS = (
  1476	    "cell",
  1477	    "trace_state",
  1478	    "measurand",
  1479	    "pairs",
  1480	    "source_init_median_ms",
  1481	    "destination_init_median_ms",
  1482	    "delta_ms",
  1483	    "paired_delta_median_ms",
  1484	    "first4_delta_median_ms",
  1485	    "last4_delta_median_ms",
  1486	    "first_last_drift_ms",
  1487	    "odd_delta_median_ms",
  1488	    "even_delta_median_ms",
  1489	    "odd_even_drift_ms",
  1490	    "source_first_delta_median_ms",
  1491	    "destination_first_delta_median_ms",
  1492	    "role_order_drift_ms",
  1493	    "paired_delta_range_ms",
  1494	    "n_pair_split_ms",
  1495	    "n_pair_ms",
  1496	    "observer_bias_ms",
  1497	    "n_resolution_ms",
  1498	    "source_init_sorted_ms",
  1499	    "destination_init_sorted_ms",
  1500	    "paired_delta_sorted_ms",
  1501	    "source_init_largest_gap_ms",
  1502	    "source_init_descriptive_modes_ms",
  1503	    "destination_init_largest_gap_ms",
  1504	    "destination_init_descriptive_modes_ms",
  1505	    "paired_delta_largest_gap_ms",
  1506	    "paired_delta_descriptive_modes_ms",
  1507	)
  1508
  1509
  1510	def _distribution_rows(stats: Sequence[ConditionStats]) -> list[dict[str, str]]:
  1511	    output: list[dict[str, str]] = []
  1512	    for item in stats:
  1513	        for metric, values in (
  1514	            ("source_init_total", item.source_values),
  1515	            ("destination_init_total", item.destination_values),
  1516	            ("paired_total_delta", item.paired_deltas),
  1517	        ):
  1518	            modes = largest_gap_modes(values)
  1519	            ordered = tuple(sorted(values))
  1520	            split = len(modes.left) if modes.right else None
  1521	            for rank, value in enumerate(ordered, start=1):
  1522	                mode = "single" if split is None else ("lower" if rank <= split else "upper")
  1523	                output.append(
  1524	                    {
  1525	                        "cell": item.cell,
  1526	                        "trace_state": item.trace_state,
  1527	                        "measurand": MEASURAND,
  1528	                        "metric": metric,
  1529	                        "rank": str(rank),
  1530	                        "value_ms": decimal_text(value),
  1531	                        "descriptive_mode": mode,
  1532	                        "largest_gap_after": "yes" if split == rank else "no",
  1533	                        "largest_gap_ms": decimal_text(modes.gap),
  1534	                    }
  1535	                )
  1536	    return output
  1537
  1538
  1539	CLOCK_SUMMARY_FIELDS = (
  1540	    "block",
  1541	    "run_id",
  1542	    "pass",
  1543	    "cell",
  1544	    "pair",
  1545	    "role",
  1546	    "role_order",
  1547	    "before_sample",
  1548	    "before_min_rtt_ns",
  1549	    "before_offset_windows_minus_q_ns",
  1550	    "after_sample",
  1551	    "after_min_rtt_ns",
  1552	    "after_offset_windows_minus_q_ns",
  1553	    "selected_max_rtt_ns",
  1554	    "selected_offset_change_ns",
  1555	)
  1556
  1557
  1558	def _clock_summary_rows(samples: Sequence[ClockSample]) -> list[dict[str, str]]:
  1559	    grouped: dict[int, dict[str, list[ClockSample]]] = {}
  1560	    for sample in samples:
  1561	        grouped.setdefault(sample.run.schedule_index, {}).setdefault(sample.phase, []).append(sample)
  1562	    output: list[dict[str, str]] = []
  1563	    for schedule_index in sorted(grouped):
  1564	        phases = grouped[schedule_index]
  1565	        if set(phases) != {"before", "after"}:
  1566	            raise AnalysisError(f"clock samples for schedule row {schedule_index} lack a phase")
  1567	        before = min(phases["before"], key=lambda item: (item.rtt_ns, item.sample))
  1568	        after = min(phases["after"], key=lambda item: (item.rtt_ns, item.sample))
  1569	        run = before.run
  1570	        output.append(
  1571	            {
  1572	                "block": str(run.block),
  1573	                "run_id": run.run_id,
  1574	                "pass": run.pass_name,
  1575	                "cell": run.cell,
  1576	                "pair": str(run.pair),
  1577	                "role": run.role,
  1578	                "role_order": str(run.role_order),
  1579	                "before_sample": str(before.sample),
  1580	                "before_min_rtt_ns": str(before.rtt_ns),
  1581	                "before_offset_windows_minus_q_ns": str(before.offset_ns),
  1582	                "after_sample": str(after.sample),
  1583	                "after_min_rtt_ns": str(after.rtt_ns),
  1584	                "after_offset_windows_minus_q_ns": str(after.offset_ns),
  1585	                "selected_max_rtt_ns": str(max(before.rtt_ns, after.rtt_ns)),
  1586	                "selected_offset_change_ns": str(after.offset_ns - before.offset_ns),
  1587	            }
  1588	        )
  1589	    return output
  1590
  1591
  1592	EVENT_FIELDS = (
  1593	    "block",
  1594	    "trace_state",
  1595	    "pass",
  1596	    "cell",
  1597	    "pair",
  1598	    "role",
  1599	    "role_order",
  1600	    "transfer_ms",
  1601	    "settled_ms",
  1602	    "flush_ms",
  1603	    "total_ms",
  1604	    "run_id",
  1605	    "session_id",
  1606	    "endpoint_role",
  1607	    "initiator_role",
  1608	    "producer_seq",
  1609	    "elapsed_ns",
  1610	    "unix_ns",
  1611	    "event",
  1612	    "action",
  1613	    "epoch",
  1614	    "socket",
  1615	    "batch",
  1616	    "count",
  1617	    "target_streams",
  1618	    "live_streams",
  1619	    "accepted",
  1620	    "source_file",
  1621	    "source_line",
  1622	)
  1623
  1624
  1625	def _event_row(row: RunRow, event: TraceEvent) -> dict[str, str]:
  1626	    raw = event.raw
  1627	    output = {
  1628	        "block": str(row.block),
  1629	        "trace_state": row.trace_state,
  1630	        "pass": row.pass_name,
  1631	        "cell": row.cell,
  1632	        "pair": str(row.pair),
  1633	        "role": row.role,
  1634	        "role_order": str(row.role_order),
  1635	        "transfer_ms": decimal_text(row.transfer_ms),
  1636	        "settled_ms": str(row.settled_ms),
  1637	        "flush_ms": decimal_text(row.flush_ms),
  1638	        "total_ms": decimal_text(row.total_ms),
  1639	        "run_id": row.run_id,
  1640	        "session_id": row.session_id,
  1641	        "endpoint_role": event.endpoint_role,
  1642	        "initiator_role": raw["initiator_role"],
  1643	        "producer_seq": str(event.producer_seq),
  1644	        "elapsed_ns": str(event.elapsed_ns),
  1645	        "unix_ns": str(raw["unix_ns"]),
  1646	        "event": event.event,
  1647	        "source_file": event.source_file,
  1648	        "source_line": str(event.source_line),
  1649	    }
  1650	    for name in (
  1651	        "action",
  1652	        "epoch",
  1653	        "socket",
  1654	        "batch",
  1655	        "count",
  1656	        "target_streams",
  1657	        "live_streams",
  1658	        "accepted",
  1659	    ):
  1660	        value = raw.get(name, "")
  1661	        if isinstance(value, bool):
  1662	            output[name] = str(value).lower()
  1663	        else:
  1664	            output[name] = str(value)
  1665	    return output
  1666
  1667
  1668	INTERVAL_FIELDS = (
  1669	    "block",
  1670	    "trace_state",
  1671	    "pass",
  1672	    "cell",
  1673	    "pair",
  1674	    "role",
  1675	    "run_id",
  1676	    "session_id",
  1677	    "endpoint_role",
  1678	    "initiator_role",
  1679	    "interval_kind",
  1680	    "interval_name",
  1681	    "correlation",
  1682	    "start_event",
  1683	    "end_event",
  1684	    "start_producer_seq",
  1685	    "end_producer_seq",
  1686	    "start_elapsed_ns",
  1687	    "end_elapsed_ns",
  1688	    "duration_ns",
  1689	)
  1690
  1691
  1692	SPAN_SPECS = (
  1693	    ("socket_dial_begin", "socket_dial_end", ("epoch", "socket")),
  1694	    ("socket_accept_begin", "socket_accept_end", ("epoch", "socket")),
  1695	    ("manifest_complete_send_begin", "manifest_complete_sent", ()),
  1696	    ("need_batch_send_begin", "need_batch_sent", ("batch",)),
  1697	    ("planner_begin", "planner_end", ("batch",)),
  1698	    ("resize_send_begin", "resize_sent", ("epoch",)),
  1699	    ("resize_ack_send_begin", "resize_ack_sent", ("epoch",)),
  1700	    ("resize_arm_queue_begin", "resize_arm_ready", ("epoch",)),
  1701	    ("socket_write_begin", "first_socket_write", ("epoch", "socket")),
  1702	    ("data_plane_complete", "summary_received", ()),
  1703	    ("data_plane_complete", "summary_send_begin", ()),
  1704	    ("summary_send_begin", "summary_sent", ()),
  1705	)
  1706
  1707
  1708	def _interval_base(row: RunRow, endpoint_role: str, initiator_role: str) -> dict[str, str]:
  1709	    return {
  1710	        "block": str(row.block),
  1711	        "trace_state": row.trace_state,
  1712	        "pass": row.pass_name,
  1713	        "cell": row.cell,
  1714	        "pair": str(row.pair),
  1715	        "role": row.role,
  1716	        "run_id": row.run_id,
  1717	        "session_id": row.session_id,
  1718	        "endpoint_role": endpoint_role,
  1719	        "initiator_role": initiator_role,
  1720	    }
  1721
  1722
  1723	def _make_interval(
  1724	    row: RunRow,
  1725	    start: TraceEvent,
  1726	    end: TraceEvent,
  1727	    kind: str,
  1728	    name: str,
  1729	    correlation: str,
  1730	) -> dict[str, str]:
  1731	    if start.endpoint_role != end.endpoint_role:
  1732	        raise AnalysisError("internal error: attempted a cross-endpoint interval")
  1733	    duration = end.elapsed_ns - start.elapsed_ns
  1734	    if duration < 0:
  1735	        raise AnalysisError(
  1736	            f"{row.run_id}/{row.session_id}/{start.endpoint_role}: negative local interval "
  1737	            f"{start.event}->{end.event}"
  1738	        )
  1739	    output = _interval_base(
  1740	        row, start.endpoint_role, start.raw["initiator_role"]
  1741	    )
  1742	    output.update(
  1743	        {
  1744	            "interval_kind": kind,
  1745	            "interval_name": name,
  1746	            "correlation": correlation,
  1747	            "start_event": start.event,
  1748	            "end_event": end.event,
  1749	            "start_producer_seq": str(start.producer_seq),
  1750	            "end_producer_seq": str(end.producer_seq),
  1751	            "start_elapsed_ns": str(start.elapsed_ns),
  1752	            "end_elapsed_ns": str(end.elapsed_ns),
  1753	            "duration_ns": str(duration),
  1754	        }
  1755	    )
  1756	    return output
  1757
  1758

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/otp12pf_rigw_analyze.py | nl -ba | sed -n '1759,2050p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-xdbbw35N' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7I49tMis' (errno=Operation not permitted)
  1759	def _phase_rows(
  1760	    rows: Sequence[RunRow], grouped: dict[tuple[str, str], list[TraceEvent]]
  1761	) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
  1762	    event_rows: list[dict[str, str]] = []
  1763	    interval_rows: list[dict[str, str]] = []
  1764	    traced_rows = [
  1765	        row for row in rows if row.trace_state == "on" and row.cell in TCP_CELLS
  1766	    ]
  1767	    for row in traced_rows:
  1768	        group = grouped[(row.run_id, row.session_id)]
  1769	        for endpoint_role in ("SOURCE", "DESTINATION"):
  1770	            endpoint = [event for event in group if event.endpoint_role == endpoint_role]
  1771	            for event in sorted(endpoint, key=lambda item: item.producer_seq):
  1772	                event_rows.append(_event_row(row, event))
  1773
  1774	            chronological = sorted(
  1775	                endpoint, key=lambda item: (item.elapsed_ns, item.producer_seq)
  1776	            )
  1777	            for start, end in zip(chronological, chronological[1:]):
  1778	                interval_rows.append(
  1779	                    _make_interval(
  1780	                        row,
  1781	                        start,
  1782	                        end,
  1783	                        "adjacent_local_timeline",
  1784	                        f"{start.event}->{end.event}",
  1785	                        "",
  1786	                    )
  1787	                )
  1788
  1789	            for start_name, end_name, keys in SPAN_SPECS:
  1790	                starts = [event for event in endpoint if event.event == start_name]
  1791	                ends = [event for event in endpoint if event.event == end_name]
  1792	                start_groups: dict[tuple[Any, ...], list[TraceEvent]] = {}
  1793	                end_groups: dict[tuple[Any, ...], list[TraceEvent]] = {}
  1794	                for event in starts:
  1795	                    start_groups.setdefault(tuple(event.raw.get(key) for key in keys), []).append(event)
  1796	                for event in ends:
  1797	                    end_groups.setdefault(tuple(event.raw.get(key) for key in keys), []).append(event)
  1798	                for correlation_key in sorted(set(start_groups) & set(end_groups)):
  1799	                    if len(start_groups[correlation_key]) != 1 or len(end_groups[correlation_key]) != 1:
  1800	                        continue
  1801	                    correlation = ";".join(
  1802	                        f"{key}={value}" for key, value in zip(keys, correlation_key)
  1803	                    )
  1804	                    interval_rows.append(
  1805	                        _make_interval(
  1806	                            row,
  1807	                            start_groups[correlation_key][0],
  1808	                            end_groups[correlation_key][0],
  1809	                            "named_local_span",
  1810	                            f"{start_name}->{end_name}",
  1811	                            correlation,
  1812	                        )
  1813	                    )
  1814	    return event_rows, interval_rows
  1815
  1816
  1817	def _markdown(
  1818	    stats: Sequence[ConditionStats],
  1819	    observer_bias: Decimal,
  1820	    n_resolution: Decimal,
  1821	    trace_event_count: int,
  1822	    interval_count: int,
  1823	    clock_arm_count: int,
  1824	) -> str:
  1825	    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
  1826	    lines = [
  1827	        "# otp-12 pf-1 rig-W phase report",
  1828	        "",
  1829	        "Validation: PASS — exact four-block OFF–ON–ON–OFF schedule, forward/reverse "
  1830	        "cell and role ordering, 8 valid role pairs per trace state/cell, trace-off and "
  1831	        "gRPC trace absence, and correlated two-role TCP terminal traces.",
  1832	        "",
  1833	        "## Durable total wall-time summaries",
  1834	        "",
  1835	        "| cell | trace | source total median ms | destination total median ms | Δ total ms | paired total d median ms | N_pair_split total ms | role-order drift total ms | paired range total ms | N_pair total ms |",
  1836	        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
  1837	    ]
  1838	    for item in stats:
  1839	        lines.append(
  1840	            "| "
  1841	            + " | ".join(
  1842	                (
  1843	                    item.cell,
  1844	                    item.trace_state,
  1845	                    decimal_text(item.source_median),
  1846	                    decimal_text(item.destination_median),
  1847	                    decimal_text(item.delta),
  1848	                    decimal_text(item.paired_delta_median),
  1849	                    decimal_text(item.n_pair_split),
  1850	                    decimal_text(item.role_order_drift),
  1851	                    decimal_text(item.paired_delta_range),
  1852	                    decimal_text(item.n_pair),
  1853	                )
  1854	            )
  1855	            + " |"
  1856	        )
  1857	    lines.extend(
  1858	        [
  1859	            "",
  1860	            "The authoritative wall-time measurand is `total_ms = transfer_ms + "
  1861	            f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms`: client execution plus every "
  1862	            f"millisecond beyond the common {SETTLE_MIN_MS} ms observation budget and "
  1863	            "the destination durability "
  1864	            f"probe. Only the common first {SETTLE_MIN_MS} ms is excluded from summaries, "
  1865	            "deltas, "
  1866	            "distributions, observer bias, and resolution floors.",
  1867	            "",
  1868	            "`Δ = median(destination_init total_ms) − median(source_init total_ms)`. "
  1869	            "Each paired `d_i = destination_init total_ms_i − source_init total_ms_i`. "
  1870	            "`N_pair_split = max(|median(d_1..d_4) − median(d_5..d_8)|, "
  1871	            "|median(d_odd) − median(d_even)|)`. The conservative operative "
  1872	            "The independent role-order drift is "
  1873	            "`|median(d_source-first) − median(d_destination-first)|`; the S,D,D,S "
  1874	            "schedule means this is not the odd/even partition. The conservative "
  1875	            "operative `N_pair = max(N_pair_split, role-order drift, max(d) − min(d))`, "
  1876	            "so a balanced bimodal mixture cannot produce a zero floor.",
  1877	            "",
  1878	            f"For target `{TARGET_CELL}`: Δ_off={decimal_text(target['off'].delta)} ms, "
  1879	            f"Δ_on={decimal_text(target['on'].delta)} ms, observer_bias="
  1880	            f"|Δ_on−Δ_off|={decimal_text(observer_bias)} ms, N_pair_off="
  1881	            f"{decimal_text(target['off'].n_pair)} ms, N_pair_on="
  1882	            f"{decimal_text(target['on'].n_pair)} ms, and N_resolution="
  1883	            f"{decimal_text(n_resolution)} ms.",
  1884	            "",
  1885	            "This run measures the observer and paired resolution floors; it does not "
  1886	            "grade any hypothesis recovery.",
  1887	            "",
  1888	            "## Sorted distributions and descriptive largest-gap modes",
  1889	            "",
  1890	            "The split is descriptive only; it does not assert statistical modality.",
  1891	            "",
  1892	            "| cell | trace | metric | sorted ms | largest gap ms | descriptive modes |",
  1893	            "|---|---:|---|---|---:|---|",
  1894	        ]
  1895	    )
  1896	    for item in stats:
  1897	        for metric, values in (
  1898	            ("source_init total_ms", item.source_values),
  1899	            ("destination_init total_ms", item.destination_values),
  1900	            ("paired total_ms d", item.paired_deltas),
  1901	        ):
  1902	            modes = largest_gap_modes(values)
  1903	            ordered = ";".join(decimal_text(value) for value in sorted(values))
  1904	            lines.append(
  1905	                f"| {item.cell} | {item.trace_state} | {metric} | {ordered} | "
  1906	                f"{decimal_text(modes.gap)} | {modes.render()} |"
  1907	            )
  1908	    lines.extend(
  1909	        [
  1910	            "",
  1911	            "## Phase evidence",
  1912	            "",
  1913	            f"`phase_events.csv` contains {trace_event_count} structured events. "
  1914	            f"`phase_intervals.csv` contains {interval_count} local-clock intervals.",
  1915	            "",
  1916	            "Each phase-event row carries the arm's validated `transfer_ms`, `settled_ms`, "
  1917	            "`flush_ms`, and authoritative `total_ms`.",
  1918	            "",
  1919	            "Every interval uses `elapsed_ns` from one endpoint only. `unix_ns` is retained "
  1920	            "in the event export for provenance and is never used for cross-host subtraction.",
  1921	            "",
  1922	            "## Clock-offset evidence",
  1923	            "",
  1924	            f"`clock_summary.csv` selects the minimum-RTT before and after sample for each "
  1925	            f"of {clock_arm_count} scheduled arms and reports its midpoint offset. These "
  1926	            "samples document cross-host uncertainty only; no cross-host phase duration is "
  1927	            "computed or graded.",
  1928	            "",
  1929	        ]
  1930	    )
  1931	    return "\n".join(lines)
  1932
  1933
  1934	def analyze(root: Path | str) -> AnalysisResult:
  1935	    output_dir = Path(root)
  1936	    if not output_dir.is_dir():
  1937	        raise AnalysisError(f"output directory does not exist: {output_dir}")
  1938	    rows = load_runs(output_dir)
  1939	    clock_samples = load_clock_samples(output_dir, rows)
  1940	    events = load_trace_events(output_dir)
  1941	    grouped = validate_traces(rows, events)
  1942	    stats = [
  1943	        condition_stats(rows, cell, trace_state)
  1944	        for cell in CELLS
  1945	        for trace_state in ("off", "on")
  1946	    ]
  1947	    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
  1948	    observer_bias = abs(target["on"].delta - target["off"].delta)
  1949	    n_resolution = max(target["off"].n_pair, target["on"].n_pair, observer_bias)
  1950	    event_rows, interval_rows = _phase_rows(rows, grouped)
  1951
  1952	    summary_csv = output_dir / "summary.csv"
  1953	    summary_md = output_dir / "summary.md"
  1954	    distributions_csv = output_dir / "distributions.csv"
  1955	    phase_events_csv = output_dir / "phase_events.csv"
  1956	    phase_intervals_csv = output_dir / "phase_intervals.csv"
  1957	    clock_summary_csv = output_dir / "clock_summary.csv"
  1958	    _atomic_csv(
  1959	        summary_csv,
  1960	        SUMMARY_FIELDS,
  1961	        _summary_rows(stats, observer_bias, n_resolution),
  1962	    )
  1963	    _atomic_csv(
  1964	        distributions_csv,
  1965	        (
  1966	            "cell",
  1967	            "trace_state",
  1968	            "measurand",
  1969	            "metric",
  1970	            "rank",
  1971	            "value_ms",
  1972	            "descriptive_mode",
  1973	            "largest_gap_after",
  1974	            "largest_gap_ms",
  1975	        ),
  1976	        _distribution_rows(stats),
  1977	    )
  1978	    _atomic_csv(phase_events_csv, EVENT_FIELDS, event_rows)
  1979	    _atomic_csv(phase_intervals_csv, INTERVAL_FIELDS, interval_rows)
  1980	    clock_rows = _clock_summary_rows(clock_samples)
  1981	    _atomic_csv(clock_summary_csv, CLOCK_SUMMARY_FIELDS, clock_rows)
  1982	    _atomic_text(
  1983	        summary_md,
  1984	        _markdown(
  1985	            stats,
  1986	            observer_bias,
  1987	            n_resolution,
  1988	            len(events),
  1989	            len(interval_rows),
  1990	            len(clock_rows),
  1991	        ),
  1992	    )
  1993	    return AnalysisResult(
  1994	        summary_csv=summary_csv,
  1995	        summary_md=summary_md,
  1996	        distributions_csv=distributions_csv,
  1997	        phase_events_csv=phase_events_csv,
  1998	        phase_intervals_csv=phase_intervals_csv,
  1999	        clock_summary_csv=clock_summary_csv,
  2000	        observer_bias=observer_bias,
  2001	        n_resolution=n_resolution,
  2002	        trace_event_count=len(events),
  2003	    )
  2004
  2005
  2006	def main(argv: Sequence[str] | None = None) -> int:
  2007	    parser = argparse.ArgumentParser(description=__doc__)
  2008	    parser.add_argument("output_dir", type=Path, help="rig-W harness output directory")
  2009	    args = parser.parse_args(argv)
  2010	    try:
  2011	        result = analyze(args.output_dir)
  2012	    except AnalysisError as exc:
  2013	        print(f"ANALYSIS-INVALID: {exc}", file=sys.stderr)
  2014	        return 2
  2015	    print(f"ANALYSIS-PASS: {result.summary_md}")
  2016	    return 0
  2017
  2018
  2019	if __name__ == "__main__":
  2020	    raise SystemExit(main())

codex
The schedule, role construction, per-arm destination reuse, single CLI invocation, destination-keyed durability, settle anchor, 1→8 resize target validation, G5 ARP interface filter, and strict cleanup are all present in the production path. One provenance concern has emerged: the checkout cleanliness check and the helper’s actual staging hash are separated by all endpoint gates, with no comparison to the reviewed Git blob at staging time; I’m checking whether the tests or another gate closes that window.
exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/otp12pf_rigw_analyze_test.py | rg -n '"'^class |''^    def test_|''^def |purge|stage|hash|sentinel|schedule|128|settle|manifest|clock|trace|resize|role|observer|resolution|launcher|cleanup|source_init|destination_init'"' ; git diff --unified=50 06b33228d502c51da24bc2a78fba7eddcf6c0723..75a9a33ce600e4707438ed885de2ce0cdf27d946 -- scripts/bench_otp12pf_rigw.sh scripts/otp12pf_rigw_analyze.py scripts/otp12pf_rigw_analyze_test.py .review/findings/otp12-pf1-rigw-harness.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-C0OfkqyO' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-xF7pAmD3' (errno=Operation not permitted)
8:import hashlib
25:class SyntheticSession:
30:        self.clock_rows: list[dict[str, str]] = []
33:        self._build_manifest_evidence()
34:        self._build_clock_samples()
38:    def _delta(trace_state: str, cell: str, pair: int) -> int:
41:        if trace_state == "off":
45:    def _trace_events(
46:        self, run_id: str, session_id: str, scheduled_role: str
48:        initiator = "SOURCE" if scheduled_role == "source_init" else "DESTINATION"
53:            endpoint_role: str,
66:                "endpoint_role": endpoint_role,
67:                "initiator_role": initiator,
86:        source_event("socket_trace_attached", epoch=0, socket=0)
87:        source_event("manifest_complete_send_begin")
88:        source_event("manifest_complete_sent", count=1)
95:                "resize_proposed",
101:                "resize_send_begin",
107:                "resize_sent",
113:                "resize_ack_received",
120:            source_event("socket_trace_attached", epoch=epoch, socket=0)
122:                "source_settled",
136:        destination_event("socket_trace_attached", epoch=0, socket=0)
137:        destination_event("manifest_complete_received")
143:                "resize_received",
150:                    "resize_arm_queue_begin",
161:                    "resize_ack_send_begin",
167:                    "resize_ack_sent",
172:                destination_event("resize_arm_ready", epoch=epoch)
175:                destination_event("socket_trace_attached", epoch=epoch, socket=0)
179:                destination_event("socket_trace_attached", epoch=epoch, socket=0)
187:                    "resize_ack_send_begin",
193:                    "resize_ack_sent",
212:                    for role_order, role in enumerate(
213:                        analyzer.expected_roles(pair), start=1
218:                            if role == "source_init"
219:                            else source_ms + self._delta(block.trace_state, cell, pair)
221:                        settled_ms = 250
224:                            f"client/b{block.number}-{cell}-p{pair}-{role}.log"
227:                        traced_tcp = block.trace_state == "on" and cell in analyzer.TCP_CELLS
229:                        if traced_tcp:
232:                            self.events.extend(self._trace_events(run_id, session_id, role))
236:                                "trace_state": block.trace_state,
239:                                "role": role,
241:                                "role_order": str(role_order),
243:                                "settled_ms": str(settled_ms),
247:                                    + settled_ms
260:    def _build_clock_samples(self) -> None:
261:        q_clock = 1_000_000_000
266:                    q_before = q_clock
271:                    self.clock_rows.append(
277:                            "role": row["role"],
287:                    q_clock = q_after + 100
290:    def _manifest_data(shape: str) -> bytes:
302:    def _build_manifest_evidence(self) -> None:
310:            data = self._manifest_data(shape)
311:            digest = hashlib.sha256(data).hexdigest()
312:            q_relative = f"fixtures/src_{shape}.manifest"
313:            win_relative = f"fixtures/windows-src_{shape}.manifest"
320:                    "q_manifest": q_relative,
321:                    "windows_manifest": win_relative,
325:        with (self.root / "fixture-manifests.csv").open("w", newline="") as handle:
328:                fieldnames=("shape", "sha256", "q_manifest", "windows_manifest"),
336:            row["tree_manifest_sha256"] = digest
338:                f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
340:            (landed / f"{rid}.manifest").write_bytes(data)
347:        with (self.root / "clock-samples.csv").open("w", newline="") as handle:
350:            writer.writerows(self.clock_rows)
351:        trace = self.root / "trace" / "nested"
352:        trace.mkdir(parents=True, exist_ok=True)
360:        with (trace / "daemon.log").open("w") as handle:
365:                    event["endpoint_role"] == event["initiator_role"]
374:class RigWAnalyzerTests(unittest.TestCase):
380:    def traced_session_id(session: SyntheticSession, initiator_role: str) -> str:
385:                if event["initiator_role"] == initiator_role
393:        endpoint_role: str,
401:            and event["endpoint_role"] == endpoint_role
410:        assert len({event["endpoint_role"] for event in desired_order}) == 1
419:    def test_complete_schedule_exact_floor_bias_and_exports(self) -> None:
421:        self.addCleanup(temporary.cleanup)
423:        self.assertEqual(str(result.observer_bias), "20")
424:        self.assertEqual(str(result.n_resolution), "70")
427:                (row["cell"], row["trace_state"]): row
441:        self.assertEqual(off["role_order_drift_ms"], "0")
448:        self.assertEqual(on["observer_bias_ms"], "20")
449:        self.assertEqual(on["n_resolution_ms"], "70")
452:        with result.clock_summary_csv.open(newline="") as handle:
453:            clocks = list(csv.DictReader(handle))
454:        self.assertEqual(len(clocks), 128)
455:        self.assertTrue(all(row["before_sample"] == "1" for row in clocks))
456:        self.assertTrue(all(row["after_sample"] == "1" for row in clocks))
457:        self.assertTrue(all(row["selected_offset_change_ns"] == "100" for row in clocks))
462:        self.assertTrue(any(row["source_file"].startswith("trace/") for row in phase_rows))
468:                    + int(row["settled_ms"])
479:        self.assertTrue(all(row["endpoint_role"] in {"SOURCE", "DESTINATION"} for row in intervals))
481:    def test_registered_schedule_is_pair_outer_with_reverse_block_bases(self) -> None:
482:        schedule = analyzer.expected_schedule()
487:                for block, cell, scheduled_pair, _role, role_order in schedule
489:                and scheduled_pair == pair
490:                and role_order == 1
503:                role
504:                for block, _cell, pair, role, role_order in schedule
505:                if block.number == 1 and role_order == 1 and _cell == base[0]
507:            ["source_init", "destination_init", "destination_init", "source_init"],
510:    def test_missing_trace_endpoint_is_rejected(self) -> None:
512:        self.addCleanup(temporary.cleanup)
519:                and event["endpoint_role"] == "DESTINATION"
523:        with self.assertRaisesRegex(analyzer.AnalysisError, "missing endpoint role"):
526:    def test_trace_off_leak_is_rejected(self) -> None:
528:        self.addCleanup(temporary.cleanup)
534:        with self.assertRaisesRegex(analyzer.AnalysisError, "trace leak: trace-off block 1"):
537:    def test_grpc_trace_leak_is_rejected(self) -> None:
539:        self.addCleanup(temporary.cleanup)
548:    def test_schedule_mismatch_is_rejected(self) -> None:
550:        self.addCleanup(temporary.cleanup)
553:        with self.assertRaisesRegex(analyzer.AnalysisError, "schedule mismatch"):
556:    def test_settled_ms_schema_and_bounds_are_fail_closed(self) -> None:
558:            with self.subTest(settled_ms=value):
560:                self.addCleanup(temporary.cleanup)
561:                session.rows[0]["settled_ms"] = value
563:                with self.assertRaisesRegex(analyzer.AnalysisError, "settled_ms"):
567:        self.addCleanup(temporary.cleanup)
570:        lines[0] = lines[0].replace("settled_ms,", "")
575:    def test_corrupt_total_is_rejected(self) -> None:
577:        self.addCleanup(temporary.cleanup)
582:            "total_ms must equal transfer_ms \\+ \\(settled_ms - 250\\) \\+ flush_ms",
586:    def test_role_specific_flush_is_included_in_delta_and_floor(self) -> None:
588:        self.addCleanup(temporary.cleanup)
591:            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
595:                if row["role"] == "source_init"
601:                + int(row["settled_ms"])
610:                (row["cell"], row["trace_state"]): row
618:        self.assertEqual(str(result.n_resolution), "56")
620:    def test_excess_settle_is_charged_without_false_role_delta(self) -> None:
622:        self.addCleanup(temporary.cleanup)
624:            "source_init": set(),
625:            "destination_init": set(),
629:            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
632:            if row["role"] == "source_init":
633:                settled_ms, flush_ms = 999, 1
635:                settled_ms, flush_ms = 250, 750
637:            row["settled_ms"] = str(settled_ms)
641:                + settled_ms
645:            old_formula_totals[row["role"]].add(transfer_ms + flush_ms)
646:            actual_elapsed.add(transfer_ms + settled_ms + flush_ms)
648:        self.assertEqual(old_formula_totals["source_init"], {101})
649:        self.assertEqual(old_formula_totals["destination_init"], {850})
655:                (row["cell"], row["trace_state"]): row
659:        self.assertEqual(off["source_init_median_ms"], "850")
660:        self.assertEqual(off["destination_init_median_ms"], "850")
666:    def test_landed_manifest_rejects_swapped_sizes_and_renamed_paths(self) -> None:
674:                self.addCleanup(temporary.cleanup)
685:                digest = hashlib.sha256(data).hexdigest()
687:                    f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
689:                (session.root / "landed" / f"{rid}.manifest").write_bytes(data)
690:                row["tree_manifest_sha256"] = digest
694:                    "landed relative-path/size manifest does not match canonical",
698:    def test_landed_root_and_recorded_manifest_digest_are_exact(self) -> None:
700:        self.addCleanup(temporary.cleanup)
707:        self.addCleanup(temporary_digest.cleanup)
708:        digest_session.rows[0]["tree_manifest_sha256"] = "0" * 64
711:            analyzer.AnalysisError, "landed manifest digest mismatch"
715:    def test_sequence_gap_and_missing_terminal_are_rejected(self) -> None:
717:        self.addCleanup(temporary.cleanup)
722:                and event["endpoint_role"] == "SOURCE"
731:    def test_payload_write_must_precede_source_completion(self) -> None:
733:        self.addCleanup(temporary.cleanup)
739:            and event["endpoint_role"] == "SOURCE"
746:            and event["endpoint_role"] == "SOURCE"
758:    def test_socket_action_end_must_precede_trace_attachment(self) -> None:
760:        self.addCleanup(temporary.cleanup)
766:            and event["endpoint_role"] == "SOURCE"
775:            and event["endpoint_role"] == "SOURCE"
776:            and event["event"] == "socket_trace_attached"
784:            "SOURCE/socket_.*_end -> SOURCE/socket_trace_attached",
788:    def test_causal_elapsed_time_cannot_run_backwards(self) -> None:
790:        self.addCleanup(temporary.cleanup)
796:            and event["endpoint_role"] == "SOURCE"
797:            and event["event"] == "socket_trace_attached"
804:            and event["endpoint_role"] == "SOURCE"
812:            "SOURCE/socket_trace_attached -> SOURCE/socket_write_begin",
816:    def test_destination_resize_prerequisites_are_causal(self) -> None:
820:                "resize_received",
821:                "resize_arm_queue_begin",
825:                "resize_arm_ready",
830:                "resize_received",
835:                "socket_trace_attached",
839:        for initiator_role, start_name, end_name in cases:
841:                initiator_role=initiator_role,
845:                self.addCleanup(temporary.cleanup)
846:                session_id = self.traced_session_id(session, initiator_role)
861:    def test_source_resize_prerequisites_are_causal(self) -> None:
862:        for initiator_role, source_action in (
867:                initiator_role=initiator_role,
868:                edge=f"resize_sent->socket_{source_action}_begin",
871:                self.addCleanup(temporary.cleanup)
872:                session_id = self.traced_session_id(session, initiator_role)
874:                    session, session_id, "SOURCE", "resize_sent", 1
877:                    session, session_id, "SOURCE", "resize_ack_received", 1
890:                    f"SOURCE/resize_sent -> SOURCE/socket_{source_action}_begin",
895:                initiator_role=initiator_role,
896:                edge="socket_trace_attached->source_settled",
899:                self.addCleanup(temporary.cleanup)
900:                session_id = self.traced_session_id(session, initiator_role)
902:                    session, session_id, "SOURCE", "socket_trace_attached", 1
904:                settled = self.phase_event(
905:                    session, session_id, "SOURCE", "source_settled", 1
907:                self.reorder_local_events([settled, attached])
911:                    "SOURCE/socket_trace_attached -> SOURCE/source_settled",
915:    def test_final_resize_settlement_precedes_data_plane_completion(self) -> None:
916:        for initiator_role in ("SOURCE", "DESTINATION"):
918:                initiator_role=initiator_role,
919:                edge="SOURCE/source_settled->data_plane_complete",
922:                self.addCleanup(temporary.cleanup)
923:                session_id = self.traced_session_id(session, initiator_role)
924:                settled = self.phase_event(
925:                    session, session_id, "SOURCE", "source_settled", 7
940:                    [first_queued, write_begin, first_write, complete, settled]
945:                    "SOURCE/source_settled -> SOURCE/data_plane_complete",
950:                initiator_role=initiator_role,
951:                edge="DESTINATION/resize_ack_sent->data_plane_complete",
954:                self.addCleanup(temporary.cleanup)
955:                session_id = self.traced_session_id(session, initiator_role)
957:                    session, session_id, "DESTINATION", "resize_ack_sent", 7
977:                    "DESTINATION/resize_ack_sent -> DESTINATION/data_plane_complete",
981:    def test_destination_preparation_action_is_role_correlated(self) -> None:
983:        self.addCleanup(temporary.cleanup)
988:            and event["initiator_role"] == "SOURCE"
995:    def test_resize_ramp_requires_all_seven_epochs(self) -> None:
997:        self.addCleanup(temporary.cleanup)
1006:        for endpoint_role in ("SOURCE", "DESTINATION"):
1007:            role_events = [
1011:                and event["endpoint_role"] == endpoint_role
1014:                sorted(role_events, key=lambda item: int(item["producer_seq"]))
1019:            analyzer.AnalysisError, "resize epochs must be exactly 1..7"
1023:    def test_final_resize_target_and_live_fields_are_exact_on_both_roles(self) -> None:
1025:            ("SOURCE", "source_settled", "target_streams"),
1026:            ("SOURCE", "source_settled", "live_streams"),
1027:            ("DESTINATION", "resize_received", "target_streams"),
1028:            ("DESTINATION", "resize_ack_sent", "live_streams"),
1030:        for endpoint_role, event_name, field in mutations:
1032:                endpoint_role=endpoint_role, event=event_name, field=field
1035:                self.addCleanup(temporary.cleanup)
1041:                    and event["endpoint_role"] == endpoint_role
1049:                    f"{endpoint_role}/{event_name} epoch 7 {field} must be 8",
1053:    def test_clock_inventory_and_midpoint_math_are_fail_closed(self) -> None:
1055:        self.addCleanup(temporary.cleanup)
1056:        session.clock_rows[0]["offset_windows_minus_q_ns"] = "999"
1061:        session.clock_rows[0]["offset_windows_minus_q_ns"] = "1000"
1062:        session.clock_rows.pop()
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-QeJQfAiW' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-31nYMOYd' (errno=Operation not permitted)
diff --git a/.review/findings/otp12-pf1-rigw-harness.md b/.review/findings/otp12-pf1-rigw-harness.md
index 58f9a0f..0831567 100644
--- a/.review/findings/otp12-pf1-rigw-harness.md
+++ b/.review/findings/otp12-pf1-rigw-harness.md
@@ -1,54 +1,54 @@
 # otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows

 **Slice**: OTP12 performance-finding pf-1, P1 rig harness only.
-**Status**: Reopened — G5 fixed and guard-proved; fresh complete review pending.
+**Status**: Reopened — G6 fixed and guard-proved; fresh complete review pending.

 ## What

 The acceptance harness cannot be reused unchanged for the phase diagnostic.
 It retains old/new and push/pull-shaped orchestration, drains Windows even
 when q is the destination, keeps one daemon alive across instrumentation-state
 changes, discards successful client stderr, and can create a firewall rule.
 Those properties either destroy the SOURCE/DESTINATION comparison or make the
 new two-endpoint trace uncorrelatable.

 ## Approach

 - Use semantic `source_init` and `destination_init` arms. SOURCE sends and
   DESTINATION receives in both arms; the varied property is only which
   endpoint initiates the one `Transfer` session.
 - Pin one canonical source tree per direction and fixture. Both roles read the
   same q or Windows physical path and land into a precreated container of the
   same depth and shape. One session-scoped canonical destination path per
   endpoint is reset and reused by all 128 arms; role-bearing run IDs are kept
   only in evidence names and never enter a measured path. Session scoping
   preserves failed-run endpoint evidence without reintroducing a within-run
   path axis. The harness requires the q and Windows canonical
   relative-path/size manifests to match, pins the one exact `src_<shape>` root,
   and retains an identical manifest and digest for every accepted arm.
 - Run a fixed OFF–ON–ON–OFF four-block schedule over
   `wm_tcp_mixed`, `mw_tcp_mixed`, `wm_grpc_mixed`, and `wm_tcp_large`.
   Pair rounds traverse cells forward/reverse/reverse/forward and run the two
   roles adjacently, producing eight pairs per trace state and cell with a
   four/four role-first balance (128 timed transfers).
 - Stop and restart both exact daemons for every block, including ON→ON. Each
   block has a common run ID; every TCP client log supplies the 16-hex session
   fingerprint that correlates its peer daemon records. Windows logs are
   retrieved through base64 with SHA-256 verification.
 - Fail closed on the exact build, route/interface/IP/MAC/MTU/link speeds,
   direction-specific negotiated MSS, firewall-rule identity, timer
   calibration, load, Time Machine, Spotlight, Windows CPU/disk drain, stale
   processes, PID ownership, port teardown, trace leakage, incomplete trace
   inventory, or landed-tree mismatch. The harness never changes firewall,
   MTU, routing, Time Machine, Spotlight, or unrelated processes.
 - Use destination-keyed durability: q file fsync for Windows→q and Windows
   volume flush for q→Windows. Both client locations capture the same q
   monotonic completion anchor: immediate subprocess return on q, or the
   streamed Windows result line as q receives it before SSH teardown. They take
   the same three after-clock samples and wait only to the absolute +250 ms
   deadline before durability. The measured
   settle must remain in `[250,1000)` ms and is retained in `runs.csv`.
   Successful Windows client logs are retrieved only after durability and the
   current landed count/byte verification. Both caches are purged before every arm and
   Windows disk writes must drain. The common first 250 ms of post-client
   observation remains excluded, but every excess settle millisecond is charged
@@ -264,50 +264,90 @@ Round-3 Codex reviewed
 and returned `PASS` with no findings. The additive Grok review of the same
 immutable range returned schema-valid `REOPENED`, `guard_confirmed=false`.
 G3 is accepted: production role-invariant path construction is correct, but
 the path-construction/parity assertions are bare `[[ ... ]]` commands that can
 survive failure under macOS Bash 3.2. Grok's role-in-path mutation produced
 different physical destinations while `SELFTEST=1` still exited zero. The
 timing-anchor and launcher-journal mutations independently went red-to-green.
 See `.review/results/otp12-pf1-rigw-harness-r3.*`. G3 was fixed at `27c94b0`;
 the complete range still requires fresh review before any rig activity.

 Coder follow-up audit admitted G4 as a separate High instrument-correctness
 finding. Destination-type, finalization-state, strict-cleanup-state,
 completion-marker-removal, and signal-cleanup checks still used bare
 `[[ ... ]]` assertions that macOS Bash 3.2 can let fall through to a later
 successful command. A regression could therefore leave an unsafe destination
 type, false cleanup state, or stale completion marker while the offline
 self-test still exited zero. G4 gives each material lifecycle assertion an
 explicit failure path and seeds the signal test with a completion marker, so
 that its absence check is not vacuous. Final-command subshell predicates and
 intentional predicate returns are unchanged. Removing the production
 `SESSION_FINALIZED=1`, retaining `Q_SESSION_MAY_EXIST=1` after successful
 cleanup, or conditionally skipping completion-marker removal for a received
 signal each turns the Bash 3.2 self-test red at the intended assertion;
 restoring all three returns it to green.

 G4 was fixed at `7e9d2d5`. The full workspace format, strict-clippy, and test
 gate; 23 analyzer tests; Bash syntax and self-test; documentation gate; and
 diff check are green for both G3 and G4. No endpoint was contacted.

 Round-4 mandatory Codex and additive Grok reviewed the complete immutable
 range through `6f517ea1bdbea2f7d83f15c086d2bf5f764cf524`. Codex returned
 `PASS` with no material finding. Grok returned schema-valid `ACCEPTED`,
 `guard_confirmed=true`, exact SHAs, and independently drove the G3 role-path
 mutation plus G4 finalization, may-exist, and marker-removal mutations red
 before restoring every offline suite green. Its detached worktree ended clean
 and was removed. Review is closed; launcher smoke and endpoint preflight remain
 required before the registered run.

 The first live launcher-smoke attempt on q refused before launching a daemon
 or timing a transfer. G5 is accepted as a High instrument-correctness finding:
 q legitimately has the Windows peer cached on `en0`, `en1`, and registered
 `en8`, but the ARP gate concatenated all three MAC rows. It therefore rejected
 the correct peer even though `route -n get` selected `en8`. The failed attempt
 is retained as `SESSION-VOID` under
 `logs/otp12pf-rigw-20260715T113500Z-launcher` in the isolated q clone. The fix
 parses exactly the registered interface, requires one result, and pins the
 real three-interface shape in the Bash 3.2 self-test. No daemon started and no
 endpoint policy changed. Removing the interface predicate makes the self-test
 red on the three-row fixture; restoring it returns the complete self-test to
 green.
+
+Round-5 reviewed the complete immutable range through
+`06b33228d502c51da24bc2a78fba7eddcf6c0723`. Mandatory Codex independently
+confirmed G5, the exact 128-arm schedule, and role-invariant endpoint-local
+paths, then returned `NEEDS FIXES` with one separate High finding. G6 is
+accepted: the harness runs the endpoint's pre-existing
+`D:/blit-test/purge-standby.ps1` by existence and exit status only, rather
+than staging and hashing the reviewed repository helper. A stale or no-op
+helper could therefore make a warm-cache run look valid. Additive Grok
+returned schema-valid `ACCEPTED`, exact SHAs, and `guard_confirmed=true` for
+G5 after independently driving the ARP interface mutation red and restoring
+the Bash 3.2 self-test green. Its detached worktree ended clean and was
+removed. No endpoint was contacted. See the round-5 raw reviews and
+adjudications under `.review/results/otp12-pf1-rigw-harness-r5.*`.
+
+G6 now takes the purge helper only from the exact clean q checkout. After all
+read-only endpoint/fabric/quiet gates pass, the harness reserves a fresh
+per-session Windows tree, copies the reviewed helper to a temporary path,
+rejects reparse points, verifies SHA-256 before and after the atomic move, and
+records the helper hash/path alongside the four executable hashes. Every arm
+rechecks that same hash immediately before invocation and requires exactly one
+`standby-purged` success line in addition to exit zero. The helper is therefore
+covered by the executable snapshot and strict session-tree cleanup rather than
+trusted as endpoint state.
+
+The Bash 3.2 self-test functionally mocks both stage and per-arm commands.
+Removing the final post-move hash comparison turns it red at the staging
+contract; restoring it returns green. Removing the per-arm hash comparison
+turns it red before the mocked purge can pass; restoring it returns green. A
+separate order guard pins the first remote write after provenance, port,
+topology, MSS, firewall, quietness, timer, and result-stream checks. No endpoint
+was contacted by the fix or its mutation proofs.
+
+G6 was fixed at `888be4754387311e28e14d687721fd3d1315f82c`.
+Format, strict clippy, Bash syntax/self-test, all 23 analyzer tests, the docs
+gate, and diff checks passed. The first full workspace test attempt hit the
+recorded macOS `blit_utils::test_utils_list_modules` daemon-start race once;
+the isolated test then passed, and a complete quiet rerun passed with two
+expected ignores. Fresh complete Codex plus additive Grok review is still
+required before any build or endpoint contact.
diff --git a/scripts/bench_otp12pf_rigw.sh b/scripts/bench_otp12pf_rigw.sh
index 1537a9a..add1b95 100755
--- a/scripts/bench_otp12pf_rigw.sh
+++ b/scripts/bench_otp12pf_rigw.sh
@@ -14,172 +14,175 @@
 #   B4 trace OFF, reverse cell order, pairs 5..8
 # Each round traverses cells base/reverse/reverse/base and runs the two roles
 # adjacently.  Each trace state therefore has eight valid role pairs per cell,
 # balanced four/four for which role goes first.
 #
 # This is the reduced P1 rig diagnostic.  It does NOT by itself close pf-1:
 # the active plan separately requires the small-fixture/P2 work and 0f922de
 # historical control before the pf-1 hard gate is complete.

 set -Eeuo pipefail

 SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
 REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

 SELFTEST=${SELFTEST:-0}
 PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
 LAUNCHER_SMOKE=${LAUNCHER_SMOKE:-0}
 EXPECT_SHA=${EXPECT_SHA:-}

 # The experiment identity is deliberately not configurable.  In particular,
 # using a hostname here would hit q's stale netwatch-01 known_hosts entry;
 # every q→Windows control and transfer uses the pinned numeric endpoint.
 Q_EXPECT_HOST=q.lan
 Q_NIC=en8
 Q_IP=10.1.10.54
 Q_MAC=00:01:d2:19:04:a3
 WIN_SSH=michael@10.1.10.177
 WIN_IP=10.1.10.177
 WIN_NIC=Ethernet
 WIN_MAC=34-5A-60-3E-78-8B
 REGISTERED_MTU=9000
 REGISTERED_MEDIA=10Gbase-T
 Q_TO_WIN_MSS=8948
 WIN_TO_Q_MSS=8960
 PORT=9031
 PAIRS_PER_BLOCK=4
 LOAD1_MAX=3.0
 SPOTLIGHT_CPU_MAX=10.0
 WIN_CPU_MAX=20.0
 SETTLE_NS=250000000
 SETTLE_MIN_MS=250
 SETTLE_MAX_MS=1000

 Q_MODULE="$HOME/blit-bench-work"
 Q_BLIT="$REPO_ROOT/target/release/blit"
 Q_DAEMON="$REPO_ROOT/target/release/blit-daemon"
 WIN_ROOT='D:/blit-test'
 WIN_MODULE="$WIN_ROOT/rigw-module"
 WIN_BINS="$WIN_ROOT/bins"
 WIN_ACTIVE="$WIN_BINS/active/blit-daemon.exe"
-WIN_PURGE="$WIN_ROOT/purge-standby.ps1"
+WIN_PURGE_SOURCE="$SCRIPT_DIR/windows/purge-standby.ps1"

 SESSION_TAG=$(date -u +%Y%m%dT%H%M%SZ).$$
 OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12pf-rigw-$SESSION_TAG}
 WIN_SESSION="$WIN_ROOT/rigw-pf1/$SESSION_TAG"
+WIN_PURGE="$WIN_SESSION/purge-standby.ps1"
+WIN_PURGE_HASH=""

 LOG="$OUT_DIR/bench.log"
 RUNS_CSV="$OUT_DIR/runs.csv"
 CLOCK_CSV="$OUT_DIR/clock-samples.csv"

 LAST_ERROR=""
 OUTPUT_CLAIMED=0
 OUTPUT_CLAIM_ERROR=""
 log() {
     local line
     line="$(date -u +%H:%M:%SZ) $*"
     if [[ "$OUTPUT_CLAIMED" == 1 ]]; then
         printf '%s\n' "$line" | tee -a "$LOG"
     else
         printf '%s\n' "$line" >&2
     fi
 }
 die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
 append_void_line() {
     printf '%s\n' "$1" >> "$OUT_DIR/SESSION-VOID"
 }
 session_void() {
     local reason="$1"
     LAST_ERROR="$reason"
     append_void_line "$reason"
     log "SESSION-VOID: $reason"
     exit 1
 }

 reserve_evidence_dir() {
     local target="$1" parent
     OUTPUT_CLAIM_ERROR=""
     if [[ -e "$target" || -L "$target" ]]; then
         if [[ -f "$target/SESSION-COMPLETE" ]]; then
             OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-COMPLETE: $target"
         elif [[ -f "$target/SESSION-VOID" ]]; then
             OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-VOID: $target"
         else
             OUTPUT_CLAIM_ERROR="refusing existing output path (must be fresh): $target"
         fi
         return 1
     fi
     parent=$(dirname "$target")
     mkdir -p "$parent" || {
         OUTPUT_CLAIM_ERROR="cannot create output parent: $parent"
         return 1
     }
     mkdir "$target" || {
         OUTPUT_CLAIM_ERROR="cannot atomically claim output directory: $target"
         return 1
     }
     mkdir "$target/trace" "$target/client" "$target/fixtures" "$target/landed" || {
         OUTPUT_CLAIM_ERROR="cannot initialize output directory: $target"
         rm -rf "$target"
         return 1
     }
 }

 claim_output_dir() {
     reserve_evidence_dir "$OUT_DIR" || return 1
     OUTPUT_CLAIMED=1
 }

 SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto \
     -o ConnectTimeout=5 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 \
     -o "ControlPath=$HOME/.ssh/cm-rigw-%r@%h-%p" -o ControlPersist=300)
 wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
+wscp() { scp "${SSH_MUX[@]}" "$@"; }

 q_daemon_pid=""
 win_daemon_pid=""
 win_cmd_pid=""
 current_block=""
 CLEANUP_MODE=0
 CLEANUP_ERROR=""
 REGISTERED_RUN_STARTED=0
 SESSION_FINALIZED=0
 STRICT_CLEANUP_VERIFIED=0
 Q_SESSION_MAY_EXIST=0
 WIN_SESSION_MAY_EXIST=0
 LOCAL_EVIDENCE_COMPLETE=0

 teardown_die() {
     local reason="$1"
     if [[ "$CLEANUP_MODE" == 1 ]]; then
         CLEANUP_ERROR="${CLEANUP_ERROR:+$CLEANUP_ERROR; }$reason"
         log "CLEANUP-ERROR: $reason"
         return 1
     fi
     session_void "$reason"
 }

 reject_registered_overrides() {
     local name
     for name in RUNS CELLS MAC_HOST WIN_HOST WIN_SSH_OVERRIDE PORT_OVERRIDE \
         Q_NIC_OVERRIDE Q_IP_OVERRIDE TRACE_ORDER PAIRS_PER_BLOCK_OVERRIDE; do
         if [[ -n "${!name+x}" ]]; then
             die "$name is not configurable for the registered rig-W diagnostic"
         fi
     done
 }

 validate_mode_selection() {
     local name value enabled=0
     for name in SELFTEST PREFLIGHT_ONLY LAUNCHER_SMOKE; do
         value=${!name}
         [[ "$value" == 0 || "$value" == 1 ]] \
             || die "$name must be exactly 0 or 1"
         if [[ "$value" == 1 ]]; then
             enabled=$((enabled + 1))
         fi
     done
     [[ "$enabled" -le 1 ]] \
         || die "SELFTEST, PREFLIGHT_ONLY, and LAUNCHER_SMOKE are mutually exclusive"
 }

 emit_schedule() {
     cat <<'EOF'
@@ -245,167 +248,251 @@ PY
 stamp_result_arrival_on_q() {
     python3 -c '
 import sys, time

 result = None
 stamp_ns = None
 for raw in sys.stdin:
     line = raw.rstrip("\r\n")
     if not line.startswith("R|"):
         continue
     if result is not None:
         raise SystemExit("multiple Windows client result sentinels")
     fields = line.split("|")
     if len(fields) != 3:
         raise SystemExit("malformed Windows client result sentinel")
     result = line
     stamp_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
 if result is None or stamp_ns is None:
     raise SystemExit("missing Windows client result sentinel")
 print(f"{result}|{stamp_ns}")
 '
 }
 successful_windows_log_phase_ok() {
     [[ "$1" == durability_verified ]]
 }
 fetch_successful_windows_client_log() {
     local arm_phase="$1" remote_err="$2" local_err="$3"
     successful_windows_log_phase_ok "$arm_phase" \
         || session_void "refusing successful Windows client-log fetch before destination durability"
     fetch_win_file "$remote_err" "$local_err"
 }
 embeds_clean_q() {
     local path="$1"
     LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID" "$path" || return 1
     LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID.dirty" "$path" && return 1
     return 0
 }

 selftest() {
     local got expected rows source_first destination_first clock_probe identity_file
     local selftest_client_done selftest_deadline selftest_settle_done run_arm_source
     local manifest_tmp canonical_manifest landed_manifest tree_digest
     local freshness_tmp freshness_case marker before analyzer_log
     local win_stop_source win_start_source finalize_tmp failure_tmp trap_calls trap_rc
     local signal signal_dir signal_rc contract_tmp on_exit_source append_tmp
     local cleanup_tmp remembered port_checks strict_cleanup_source
     local destination_tmp prepare_destination_source stamped_result stamp_before stamp_after
     local stamp_tag stamp_ms stamp_rc stamp_ns stamp_extra stamp_teardown_ns
     local cross_clock_before cross_clock_after cross_clock_delta
     local launcher_tmp launcher_calls launcher_source main_source
-    local win_recovery_tmp
+    local win_recovery_tmp purge_contract_tmp purge_hash drained preflight_source
     reject_registered_overrides
     if (
         SELFTEST=1
         PREFLIGHT_ONLY=1
         LAUNCHER_SMOKE=0
         validate_mode_selection
     ) >/dev/null 2>&1; then
         die "multiple harness modes were accepted"
     fi
     if (
         SELFTEST=2
         PREFLIGHT_ONLY=0
         LAUNCHER_SMOKE=0
         validate_mode_selection
     ) >/dev/null 2>&1; then
         die "invalid harness mode value was accepted"
     fi
     got=$(emit_schedule)
     expected=$'1,off,forward,1,4\n2,on,reverse,1,4\n3,on,forward,5,8\n4,off,reverse,5,8'
     [[ "$got" == "$expected" ]] || die "registered block schedule changed"

     rows=0; source_first=0; destination_first=0
     local block state pass first last round pair first_role
     while IFS=, read -r block state pass first last; do
         for ((round=1; round<=PAIRS_PER_BLOCK; round++)); do
             pair=$((first + round - 1))
             case "$round" in
                 1|4) first_role=source_init; source_first=$((source_first + 4));;
                 2|3) first_role=destination_init; destination_first=$((destination_first + 4));;
             esac
             [[ "$pair" -ge "$first" && "$pair" -le "$last" && -n "$first_role" ]]
             rows=$((rows + 8)) # four cells × two adjacent roles
         done
     done < <(emit_schedule)
     [[ "$rows" == 128 ]] || die "schedule emitted $rows arms, expected 128"
     [[ "$source_first" == 32 && "$destination_first" == 32 ]] \
         || die "schedule role-first balance changed"
     [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]] \
         || die "q source path construction changed"
     [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]] \
         || die "Windows source path construction changed"
     local destination_rel="rigw-sessions/$SESSION_TAG/destination/container"
     [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]] \
         || die "q SOURCE-initiated destination path changed"
     [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]] \
         || die "q DESTINATION-initiated destination path changed"
     [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]] \
         || die "Windows SOURCE-initiated destination path changed"
     [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
         || die "Windows DESTINATION-initiated destination path changed"
     [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]] \
         || die "Windows-to-q physical destination depends on initiator role"
     [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]] \
         || die "q-to-Windows physical destination depends on initiator role"
     [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]] \
         || die "Windows-to-q SOURCE-initiated destination argument changed"
     [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]] \
         || die "Windows-to-q DESTINATION-initiated destination argument changed"
     [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]] \
         || die "q-to-Windows SOURCE-initiated destination argument changed"
     [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
         || die "q-to-Windows DESTINATION-initiated destination argument changed"
     local arp_fixture
     arp_fixture=$'? (10.1.10.177) at 34:5a:60:3e:78:8b on en0 ifscope [ethernet]\n? (10.1.10.177) at 34:5a:60:3e:78:8b on en1 ifscope [ethernet]\n? (10.1.10.177) at 34:5a:60:3e:78:8b on en8 ifscope [ethernet]'
     [[ "$(q_peer_mac_from_arp en8 <<<"$arp_fixture")" == "34:5a:60:3e:78:8b" ]] \
         || die "q ARP parser did not select exactly the registered interface"
+    purge_contract_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-purge-contract.XXXXXX")
+    printf '%s\n' '# reviewed purge helper fixture' > "$purge_contract_tmp/purge-standby.ps1"
+    purge_hash=$(sha256_q "$purge_contract_tmp/purge-standby.ps1")
+    if ! (
+        WIN_PURGE_SOURCE="$purge_contract_tmp/purge-standby.ps1"
+        WIN_SESSION='D:/blit-test/rigw-pf1/selftest'
+        WIN_PURGE="$WIN_SESSION/purge-standby.ps1"
+        WIN_PURGE_HASH=""
+        wscp() {
+            [[ "$#" == 2 && "$1" == "$WIN_PURGE_SOURCE" \
+                && "$2" == "$WIN_SSH:$WIN_PURGE.tmp" ]] || return 91
+        }
+        wssh() {
+            local command="$1"
+            if [[ "$command" == *"refusing existing Windows session tree"* ]]; then
+                [[ "$command" == *"New-Item -ItemType Directory -Path '$WIN_SESSION'"* ]] \
+                    || return 92
+                return 0
+            fi
+            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE.tmp'"* ]] \
+                || return 93
+            [[ "$command" == *"if (\$tmpHash -cne '$purge_hash')"* ]] || return 94
+            [[ "$command" == *"Move-Item -LiteralPath '$WIN_PURGE.tmp' -Destination '$WIN_PURGE'"* ]] \
+                || return 95
+            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE'"* ]] \
+                || return 96
+            [[ "$command" == *"if (\$finalHash -cne '$purge_hash')"* ]] || return 97
+            printf 'H|%s\n' "$purge_hash"
+        }
+        stage_purge_helper
+        [[ "$WIN_PURGE_HASH" == "$purge_hash" ]] || exit 98
+    ); then
+        rm -rf "$purge_contract_tmp"
+        die "reviewed Windows purge helper was not staged and hash-verified"
+    fi
+    if ! (
+        WIN_PURGE='D:/blit-test/rigw-pf1/selftest/purge-standby.ps1'
+        WIN_PURGE_HASH="$purge_hash"
+        sync() { return 0; }
+        sudo() { return 0; }
+        wssh() {
+            local command="$1"
+            [[ "$command" == *"Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE'"* ]] \
+                || return 101
+            [[ "$command" == *"if (\$purgeHash -cne '$WIN_PURGE_HASH')"* ]] \
+                || return 102
+            [[ "$command" == *"\$purgeOutput = @(& pwsh -NoProfile -File '$WIN_PURGE')"* ]] \
+                || return 103
+            [[ "$command" == *"\$purgeOutput.Count -ne 1"* \
+                && "$command" == *"[string]\$purgeOutput[0] -cne 'standby-purged'"* ]] \
+                || return 104
+        }
+        drained=$(drain_both)
+        [[ "$drained" == drained ]] || exit 105
+    ); then
+        rm -rf "$purge_contract_tmp"
+        die "Windows purge helper was not hash-verified per arm with exact success output"
+    fi
+    rm -rf "$purge_contract_tmp"
+    preflight_source=$(declare -f preflight)
+    python3 - "$preflight_source" <<'PY' \
+        || die "purge-helper staging moved ahead of read-only endpoint gates"
+import sys
+
+source = sys.argv[1]
+markers = (
+    "provenance_gate",
+    "ports_closed",
+    "q_topology_gate",
+    "win_topology_gate",
+    "mss_gate",
+    "firewall_gate",
+    "q_quiet_gate",
+    "win_quiet_gate",
+    "timer_gate",
+    "windows_result_stream_gate",
+    "stage_purge_helper",
+    "write_manifest",
+    "verify_fixtures",
+)
+positions = [source.index(marker) for marker in markers]
+if positions != sorted(positions) or source.count("stage_purge_helper") != 1:
+    raise SystemExit(f"preflight marker order changed: {positions}")
+PY
     clock_probe=$(append_clock_row 1 run cell 1 source_init before 1 10 11 12 2 0)
     [[ "$(awk -F, '{print NF}' <<<"$clock_probe")" == 12 ]] \
         || die "clock sample row is not exactly 12 columns"
     [[ "$SETTLE_NS" == 250000000 && "$SETTLE_MIN_MS" == 250 && "$SETTLE_MAX_MS" == 1000 ]] \
         || die "registered post-client settle bounds changed"
     cross_clock_before=$(q_monotonic_ns)
     sleep 0.05
     cross_clock_after=$(q_monotonic_ns)
     cross_clock_delta=$((cross_clock_after - cross_clock_before))
     [[ "$cross_clock_delta" -ge 40000000 && "$cross_clock_delta" -lt 500000000 ]] \
         || die "q monotonic clock is not comparable across processes"
     selftest_client_done=$(q_monotonic_ns)
     selftest_deadline=$((selftest_client_done + SETTLE_NS))
     selftest_settle_done=$(settle_until_deadline "$selftest_deadline")
     [[ "$selftest_settle_done" =~ ^[0-9]+$ && "$selftest_settle_done" -ge "$selftest_deadline" ]] \
         || die "absolute post-client deadline wait returned early"
     stamp_before=$(q_monotonic_ns)
     stamped_result=$(
         { printf '%s\n' 'R|17|0'; sleep 0.35; } | stamp_result_arrival_on_q
     ) || die "q result-arrival stamper rejected one exact sentinel"
     stamp_after=$(q_monotonic_ns)
     IFS='|' read -r stamp_tag stamp_ms stamp_rc stamp_ns stamp_extra <<<"$stamped_result"
     [[ "$stamp_tag" == R && "$stamp_ms" == 17 && "$stamp_rc" == 0 \
         && "$stamp_ns" =~ ^[0-9]+$ && -z "$stamp_extra" ]] \
         || die "q result-arrival stamper returned '$stamped_result'"
     [[ "$stamp_ns" -ge "$stamp_before" && "$stamp_ns" -le "$stamp_after" ]] \
         || die "q result-arrival stamp is outside the producer lifetime"
     stamp_teardown_ns=$((stamp_after - stamp_ns))
     [[ "$stamp_teardown_ns" -ge 250000000 ]] \
         || die "q result-arrival stamp moved after producer teardown"
     if successful_windows_log_phase_ok client_done; then
         die "successful Windows client log was fetchable before durability"
     fi
     successful_windows_log_phase_ok durability_verified \
         || die "successful Windows client log was blocked after durability"

     run_arm_source=$(declare -f run_arm)
     python3 - "$run_arm_source" <<'PY' || die "run_arm post-client ordering changed"
 import sys

 source = sys.argv[1]
 markers = (
     'dest=$(arm_destination_path "$direction" "$role")',
     'dest_arg=$(arm_destination_argument "$direction" "$role")',
     "read -r result_tag transfer_ms rc client_done_ns result_extra",
     'settle_deadline_ns=$((client_done_ns + SETTLE_NS))',
     'record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after',
     'settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")',
     'flush_out=$(flush_verify_q "$dest")',
     'flush_out=$(flush_verify_win "$dest")',
@@ -1229,100 +1316,134 @@ Q_SESSION_MAY_EXIST=1
 WIN_SESSION_MAY_EXIST=1
 current_block=1
 q_daemon_pid=111
 win_daemon_pid=222
 win_cmd_pid=333
 win_daemon_stop() {
     printf "windows\n" >> "$OUT_DIR/stops"
     win_daemon_pid=""; win_cmd_pid=""; current_block=""
 }
 q_daemon_stop() {
     printf "q\n" >> "$OUT_DIR/stops"
     q_daemon_pid=""
 }
 printf "must disappear\n" > "$OUT_DIR/SESSION-COMPLETE"
 trap on_exit EXIT
 install_signal_traps
 kill -s "$3" "$$"
 sleep 2
 exit 99
 ' _ "$SCRIPT_DIR/bench_otp12pf_rigw.sh" "$signal_dir" "$signal"
         signal_rc=$?
         set -e
         [[ "$signal_rc" == 1 ]] \
             || die "$signal cleanup returned $signal_rc, expected 1"
         grep -Fxq "received $signal" "$signal_dir/SESSION-VOID" \
             || die "$signal cleanup omitted its signal reason"
         [[ "$(LC_ALL=C sort "$signal_dir/stops")" == $'q\nwindows' ]] \
             || die "$signal cleanup did not invoke both exact-owned teardown paths"
         [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
             || die "$signal cleanup left SESSION-COMPLETE"
     done
     rm -rf "$failure_tmp"

     analyzer_log=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-analyzer.XXXXXX")
     if ! python3 "$SCRIPT_DIR/otp12pf_rigw_analyze_test.py" \
         > "$analyzer_log" 2>&1; then
         cat "$analyzer_log" >&2
         rm -f "$analyzer_log"
         die "analyzer self-tests failed"
     fi
     rm -f "$analyzer_log"
     log "SELFTEST OK: exact four-block/128-arm schedule and analyzer guards"
 }

 sha256_q() { shasum -a 256 "$1" | awk '{print $1}'; }
 sha256_win() {
     wssh "(Get-FileHash -Algorithm SHA256 -LiteralPath '$1').Hash.ToLower()" \
         | tr -d '\r' | tail -1
 }

+stage_purge_helper() {
+    local staged_tmp="$WIN_PURGE.tmp" remote_hash
+    [[ -f "$WIN_PURGE_SOURCE" && ! -L "$WIN_PURGE_SOURCE" ]] \
+        || die "reviewed Windows purge helper is absent or not a plain file"
+    WIN_PURGE_HASH=$(sha256_q "$WIN_PURGE_SOURCE") \
+        || die "cannot hash reviewed Windows purge helper"
+    [[ "$WIN_PURGE_HASH" =~ ^[0-9a-f]{64}$ ]] \
+        || die "reviewed Windows purge helper hash is malformed: $WIN_PURGE_HASH"
+
+    WIN_SESSION_MAY_EXIST=1
+    wssh "
+\$ErrorActionPreference = 'Stop'
+if (Test-Path -LiteralPath '$WIN_SESSION') { throw 'refusing existing Windows session tree' }
+New-Item -ItemType Directory -Path '$WIN_SESSION' -ErrorAction Stop | Out-Null
+" || die "cannot reserve fresh Windows session tree for reviewed purge helper"
+    wscp "$WIN_PURGE_SOURCE" "$WIN_SSH:$staged_tmp" \
+        || die "cannot stage reviewed Windows purge helper"
+    remote_hash=$(wssh "
+\$ErrorActionPreference = 'Stop'
+\$tmpItem = Get-Item -LiteralPath '$staged_tmp' -Force -ErrorAction Stop
+if ((\$tmpItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'staged purge helper is a reparse point' }
+\$tmpHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$staged_tmp').Hash.ToLower()
+if (\$tmpHash -cne '$WIN_PURGE_HASH') { throw \"staged purge helper hash mismatch: \$tmpHash\" }
+Move-Item -LiteralPath '$staged_tmp' -Destination '$WIN_PURGE' -ErrorAction Stop
+\$finalItem = Get-Item -LiteralPath '$WIN_PURGE' -Force -ErrorAction Stop
+if ((\$finalItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'final purge helper is a reparse point' }
+\$finalHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE').Hash.ToLower()
+if (\$finalHash -cne '$WIN_PURGE_HASH') { throw \"final purge helper hash mismatch: \$finalHash\" }
+\"H|\$finalHash\"
+" | tr -d '\r') || die "cannot verify reviewed Windows purge helper"
+    [[ "$remote_hash" == "H|$WIN_PURGE_HASH" ]] \
+        || die "Windows purge helper verification returned '$remote_hash'"
+}
+
 float_le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }

 q_load1() {
     /usr/sbin/sysctl -n vm.loadavg | awk '{gsub(/[{}]/, ""); print $1}'
 }

 q_spotlight_cpu() {
     ps -axo %cpu=,comm= | awk '
         $2 ~ /(mds|mds_stores|mdworker|mdbulkimport)$/ { sum += $1 }
         END { printf "%.1f\n", sum + 0 }'
 }

 q_time_machine_gate() {
     local auto status
     auto=$(/usr/bin/defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null) \
         || die "q Time Machine AutoBackup setting is unreadable"
     [[ "$auto" == 0 ]] \
         || die "q Time Machine AutoBackup is enabled ($auto); do not mutate it from the harness"
     status=$(/usr/bin/tmutil status) || die "q Time Machine status is unreadable"
     grep -q 'Running = 0;' <<<"$status" \
         || die "q Time Machine is running"
 }

 q_quiet_gate() {
     local offenders load spot
     offenders=$(ps -axo pid=,comm= | awk -v owned="${q_daemon_pid:-}" '
         {
           n=$2; sub(/^.*\//, "", n)
           if ($1 != owned && (n == "cargo" || n == "rustc" || n == "blit-daemon" || n ~ /^codex($|-)/))
             print $1 ":" n
         }')
     [[ -z "$offenders" ]] || die "q has benchmark-conflicting processes: $offenders"
     q_time_machine_gate
     load=$(q_load1)
     [[ "$load" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse q load1 '$load'"
     float_le "$load" "$LOAD1_MAX" || die "q load1 $load exceeds $LOAD1_MAX"
     spot=$(q_spotlight_cpu)
     [[ "$spot" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Spotlight CPU '$spot'"
     float_le "$spot" "$SPOTLIGHT_CPU_MAX" \
         || die "q Spotlight CPU $spot% exceeds $SPOTLIGHT_CPU_MAX%"
     log "quiet q: load1=$load Spotlight=${spot}% TimeMachine=disabled/stopped"
 }

 win_quiet_gate() {
     local out avg
     out=$(wssh '
 $ErrorActionPreference = "Stop"
 $bad = Get-Process cargo,rustc,blit-daemon -ErrorAction SilentlyContinue
 if ($bad) { "BAD|" + (($bad | ForEach-Object { "$($_.Id):$($_.ProcessName)" }) -join ","); exit 7 }
 $samples = 1..3 | ForEach-Object {
@@ -1587,148 +1708,150 @@ matching_manifest_digest() {
     local canonical="$1" landed="$2"
     cmp -s "$canonical" "$landed" || return 1
     sha256_q "$landed"
 }

 verify_fixtures() {
     local shape want qgot wgot qmanifest wmanifest qhash
     printf '%s\n' 'shape,sha256,q_manifest,windows_manifest' \
         > "$OUT_DIR/fixture-manifests.csv"
     WIN_SESSION_MAY_EXIST=1
     wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION/fixtures' | Out-Null" \
         || die "cannot create Windows fixture evidence directory"
     for shape in mixed large; do
         case "$shape" in
             mixed) want=5001,547110912;;
             large) want=1,1073741824;;
         esac
         qgot=$(fixture_shape_q "$(q_source_path "$shape")")
         wgot=$(fixture_shape_win "$(win_source_path "$shape")")
         [[ "$qgot" == "$want" ]] || die "q src_$shape is $qgot, expected $want"
         [[ "$wgot" == "$want" ]] || die "Windows canonical src_$shape is $wgot, expected $want"
         qmanifest="$OUT_DIR/fixtures/src_$shape.manifest"
         wmanifest="$OUT_DIR/fixtures/windows-src_$shape.manifest"
         write_q_tree_manifest "$(q_source_path "$shape")" "$qmanifest" \
             || die "q src_$shape manifest failed"
         write_win_tree_manifest \
             "$(win_source_path "$shape")" \
             "$WIN_SESSION/fixtures/src_$shape.manifest" "$wmanifest" \
             || die "Windows src_$shape manifest failed"
         qhash=$(matching_manifest_digest "$qmanifest" "$wmanifest") \
             || die "q and Windows src_$shape relative-path/size manifests differ"
         printf '%s,%s,%s,%s\n' \
             "$shape" "$qhash" "fixtures/src_$shape.manifest" \
             "fixtures/windows-src_$shape.manifest" \
             >> "$OUT_DIR/fixture-manifests.csv"
     done
     log "canonical fixtures verified byte-for-byte by relative path and size on both hosts"
 }

 write_manifest() {
     local qbh qdh wbh wdh
     qbh=$(sha256_q "$Q_BLIT"); qdh=$(sha256_q "$Q_DAEMON")
     wbh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit.exe")
     wdh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit-daemon.exe")
     cat > "$OUT_DIR/staging-manifest.csv" <<EOF
 host,role,commit,sha256,path
 q,client,$HEAD_FULL,$qbh,$Q_BLIT
 q,daemon,$HEAD_FULL,$qdh,$Q_DAEMON
 windows,client,$HEAD_FULL,$wbh,$WIN_BINS/$HEAD_SHORT/blit.exe
 windows,daemon,$HEAD_FULL,$wdh,$WIN_BINS/$HEAD_SHORT/blit-daemon.exe
+windows,cache-helper,$HEAD_FULL,$WIN_PURGE_HASH,$WIN_PURGE
 EOF
     WIN_DAEMON_HASH=$wdh
 }

 provenance_gate() {
     [[ -n "$EXPECT_SHA" ]] || die "EXPECT_SHA=<full reviewed commit> is required"
     HEAD_FULL=$(git -C "$REPO_ROOT" rev-parse HEAD)
     HEAD_SHORT=$(git -C "$REPO_ROOT" rev-parse --short=7 HEAD)
     HEAD_BUILD_ID=$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD)
     [[ "$EXPECT_SHA" == "$HEAD_FULL" ]] \
         || die "EXPECT_SHA=$EXPECT_SHA but isolated clone is $HEAD_FULL"
     [[ -z $(git -C "$REPO_ROOT" status --porcelain --untracked-files=normal) ]] \
         || die "isolated q clone is dirty"
     [[ -x "$Q_BLIT" && -x "$Q_DAEMON" ]] || die "q release binaries are absent"
     embeds_clean_q "$Q_BLIT" \
         || die "q client does not embed a clean +$HEAD_BUILD_ID"
     embeds_clean_q "$Q_DAEMON" \
         || die "q daemon does not embed a clean +$HEAD_BUILD_ID"
     wssh "
 if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe')) { exit 2 }
 if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe')) { exit 3 }
 if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 4 }
 if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 5 }
 if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 6 }
 if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 7 }
 " || die "Windows binaries are missing or do not embed a clean +$HEAD_BUILD_ID"
-    write_manifest
-    log "provenance exact: $HEAD_FULL on q and Windows"
 }

 preflight() {
     reject_registered_overrides
     command -v python3 >/dev/null || die "python3 required"
     command -v lsof >/dev/null || die "lsof required"
     command -v nc >/dev/null || die "nc required"
+    command -v scp >/dev/null || die "scp required"
     sudo -n /usr/sbin/purge >/dev/null || die "q NOPASSWD purge grant is absent"
     provenance_gate
     ports_closed || die "port $PORT already has a listener on q or Windows"
     q_topology_gate
     win_topology_gate
     mss_gate
     firewall_gate
     q_quiet_gate
     win_quiet_gate
     timer_gate
     windows_result_stream_gate
+    stage_purge_helper
+    write_manifest
     verify_fixtures
-    log "PREFLIGHT OK: registered rig, exact binaries, canonical paths, quiet endpoints"
+    log "PREFLIGHT OK: registered rig, exact binaries/helper, canonical paths, quiet endpoints"
 }

 q_daemon_stop() {
     local pid="$q_daemon_pid" i
     [[ -z "$pid" ]] && return 0
     if kill -0 "$pid" 2>/dev/null; then
         local cmd
         cmd=$(ps -p "$pid" -o command= 2>/dev/null || true)
         [[ "$cmd" == *"$Q_DAEMON"* ]] \
             || { teardown_die "refusing to stop q PID $pid because it is not the launched daemon: $cmd"; return 1; }
         kill "$pid" || true
         for ((i=0; i<40; i++)); do
             kill -0 "$pid" 2>/dev/null || break
             sleep 0.25
         done
         kill -0 "$pid" 2>/dev/null \
             && { teardown_die "q daemon PID $pid survived exact teardown"; return 1; }
     fi
     q_daemon_pid=""
 }

 win_daemon_stop() {
     local pid="$win_daemon_pid" cmdpid="$win_cmd_pid" out pid_probe
     if [[ -z "$pid" && -z "$cmdpid" && -n "$current_block" ]]; then
         if ! pid_probe=$(wssh "
 \$ErrorActionPreference = 'Stop'
 \$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
 \$d = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/daemon.pid' -ErrorAction SilentlyContinue
 \$c = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/launcher.pid' -ErrorAction SilentlyContinue
 if (-not \$c) {
   \$launchers = @(Get-CimInstance Win32_Process -Filter \"Name='cmd.exe'\" | Where-Object {
     \$actual = if (\$_.CommandLine) { \$_.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
     \$actual -ieq \$expectedLauncher
   })
   if (\$launchers.Count -gt 1) { throw \"multiple exact launchers match \$expectedLauncher\" }
   if (\$launchers.Count -eq 1) { \$c = [string]\$launchers[0].ProcessId }
 }
 if (-not \$d -and \$c -match '^[0-9]+$') {
   \$children = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
     \$_.ParentProcessId -eq [int]\$c
   })
   if (\$children.Count -gt 1) { throw \"multiple daemon children belong to launcher \$c\" }
   if (\$children.Count -eq 1) { \$d = [string]\$children[0].ProcessId }
 }
 \"P|\$c|\$d\"
 " 2>/dev/null | tr -d '\r' | tail -1); then
             teardown_die "Windows PID recovery failed for block $current_block"
             return 1
         fi
         IFS='|' read -r _ cmdpid pid <<<"$pid_probe"
@@ -1915,103 +2038,108 @@ if (-not \$d) { Get-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.err'
 \$actualDaemon = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
 if (\$actualDaemon -ine '$WIN_ACTIVE' -or \$d.ParentProcessId -ne \$r.ProcessId) { throw \"daemon identity mismatch: \$(\$d.ExecutablePath) parent=\$(\$d.ParentProcessId)\" }
 Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.pid' -Value \$d.ProcessId
 \"P|\$(\$r.ProcessId)|\$(\$d.ProcessId)\"
 ") || session_void "Windows daemon failed to start in block $block: $out"
     out=${out//$'\r'/}
     IFS='|' read -r _ win_cmd_pid win_daemon_pid <<<"$(grep '^P|' <<<"$out" | tail -1)"
     [[ "$win_cmd_pid" =~ ^[0-9]+$ && "$win_daemon_pid" =~ ^[0-9]+$ ]] \
         || session_void "cannot parse Windows daemon PIDs from '$out'"
 }

 start_daemons() {
     local block="$1" state="$2" run_id="$3"
     ports_closed || session_void "port $PORT occupied before block $block"
     q_daemon_start "$block" "$state" "$run_id"
     win_daemon_start "$block" "$state" "$run_id"
     sleep 1
     nc -z -w 3 "$WIN_IP" "$PORT" || session_void "q cannot reach Windows daemon in block $block"
     wssh "if (-not (Test-NetConnection -ComputerName '$Q_IP' -Port $PORT -InformationLevel Quiet)) { exit 8 }" \
         >/dev/null || session_void "Windows cannot reach q daemon in block $block"
     log "block $block daemons up, trace=$state, run_id=$run_id"
 }

 record_clock_samples() {
     local block="$1" run_id="$2" cell="$3" pair="$4" role="$5" phase="$6" sample before after remote rtt midpoint offset
     for sample in 1 2 3; do
         before=$(python3 -c 'import time; print(time.time_ns())')
         remote=$(wssh '([DateTime]::UtcNow.Ticks - 621355968000000000) * 100' | tr -cd '0-9')
         after=$(python3 -c 'import time; print(time.time_ns())')
         [[ "$remote" =~ ^[0-9]+$ ]] || session_void "clock probe returned '$remote'"
         rtt=$((after - before)); midpoint=$((before + rtt / 2)); offset=$((remote - midpoint))
         append_clock_row \
             "$block" "$run_id" "$cell" "$pair" "$role" "$phase" "$sample" \
             "$before" "$remote" "$after" "$rtt" "$offset" >> "$CLOCK_CSV"
     done
 }

 drain_both() {
     sync || return 1
     sudo -n /usr/sbin/purge >/dev/null || return 1
     wssh "
 \$ErrorActionPreference = 'Stop'
 Write-VolumeCache D
 \$quiet = 0
 for (\$i=0; \$i -lt 30; \$i++) {
   \$w = (Get-Counter '\\PhysicalDisk(_Total)\\Disk Write Bytes/sec' -SampleInterval 1 -MaxSamples 1).CounterSamples[0].CookedValue
   if (\$null -ne \$w -and [double]\$w -lt 1048576) { \$quiet++ } else { \$quiet=0 }
   if (\$quiet -ge 3) { break }
 }
 if (\$quiet -lt 3) { throw 'DRAIN-TIMEOUT' }
-if (-not (Test-Path -LiteralPath '$WIN_PURGE')) { throw 'purge helper absent' }
-& pwsh -NoProfile -File '$WIN_PURGE'
-if (\$LASTEXITCODE -ne 0) { throw \"purge helper rc \$LASTEXITCODE\" }
+\$purgeItem = Get-Item -LiteralPath '$WIN_PURGE' -Force -ErrorAction Stop
+if ((\$purgeItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'purge helper is a reparse point' }
+\$purgeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_PURGE').Hash.ToLower()
+if (\$purgeHash -cne '$WIN_PURGE_HASH') { throw \"purge helper hash mismatch: \$purgeHash\" }
+\$purgeOutput = @(& pwsh -NoProfile -File '$WIN_PURGE')
+\$purgeRc = \$LASTEXITCODE
+if (\$purgeRc -ne 0) { throw \"purge helper rc \$purgeRc\" }
+if (\$purgeOutput.Count -ne 1 -or [string]\$purgeOutput[0] -cne 'standby-purged') { throw \"purge helper output mismatch: \$(\$purgeOutput -join '|')\" }
 'drained'
 " >/dev/null || return 1
     printf drained
 }

 prepare_destination() {
     local direction="$1" dest="$2" first
     if [[ "$direction" == wm ]]; then
         rm -rf -- "$dest" || return 1
         [[ ! -e "$dest" && ! -L "$dest" ]] || return 1
         mkdir -p -- "$dest" || return 1
         [[ -d "$dest" && ! -L "$dest" ]] || return 1
         first=$(find "$dest" -mindepth 1 -maxdepth 1 -print -quit) || return 1
         [[ -z "$first" ]] || return 1
     else
         wssh "
 \$ErrorActionPreference = 'Stop'
 if (Test-Path -LiteralPath '$dest') {
   Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
 }
 if (Test-Path -LiteralPath '$dest') { throw 'destination removal did not land' }
 New-Item -ItemType Directory -Force -Path '$dest' -ErrorAction Stop | Out-Null
 if (-not (Test-Path -LiteralPath '$dest' -PathType Container)) { throw 'destination is not a directory' }
 \$item = Get-Item -LiteralPath '$dest' -Force -ErrorAction Stop
 if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'destination is a reparse point' }
 if (@(Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop).Count -ne 0) { throw 'destination is not empty' }
 " || return 1
     fi
 }

 flush_verify_q() {
     python3 - "$1" <<'PY'
 import os, sys, time
 t=time.monotonic_ns(); n=b=0
 for root, dirs, files in os.walk(sys.argv[1]):
     for name in files:
         p=os.path.join(root,name)
         fd=os.open(p,os.O_RDONLY); os.fsync(fd); os.close(fd)
         n+=1; b+=os.path.getsize(p)
 print(f"F|{round((time.monotonic_ns()-t)/1_000_000)}|{n}|{b}")
 PY
 }

 flush_verify_win() {
     wssh "
 \$ErrorActionPreference = 'Stop'
 \$sw=[Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache D; \$sw.Stop()
 \$f=Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
 \$bytes=if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 }
 \"F|\$([int]\$sw.Elapsed.TotalMilliseconds)|\$(\$f.Count)|\$bytes\"

exec
/bin/zsh -lc 'git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:scripts/windows/purge-standby.ps1 | nl -ba; git ls-tree -r 75a9a33ce600e4707438ed885de2ce0cdf27d946 scripts/windows/purge-standby.ps1; git diff --check 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946; git diff --numstat 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946 -- scripts/bench_otp12pf_rigw.sh scripts/otp12pf_rigw_analyze.py scripts/otp12pf_rigw_analyze_test.py' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-hHHnl32R' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-yB3nHYW9' (errno=Operation not permitted)
     1	# Empty the Windows standby list (the file cache) — the drop_caches
     2	# equivalent for the otp-2w benchmark (scripts/bench_otp2w_baseline.sh
     3	# stages this file to the daemon host and invokes it before every
     4	# timed run). Requires an administrator token; enables
     5	# SeProfileSingleProcessPrivilege, then asks the memory manager to
     6	# purge the standby list (SystemMemoryListInformation / command 4).
     7	# Every step is checked and reported (codex otp-2w F5):
     8	# AdjustTokenPrivileges can "succeed" while assigning nothing
     9	# (ERROR_NOT_ALL_ASSIGNED) — that is surfaced as the causal error
    10	# instead of an opaque NTSTATUS from the purge itself.
    11	$ErrorActionPreference = 'Stop'
    12
    13	Add-Type -TypeDefinition @"
    14	using System;
    15	using System.ComponentModel;
    16	using System.Runtime.InteropServices;
    17
    18	public static class StandbyPurge
    19	{
    20	    [StructLayout(LayoutKind.Sequential)]
    21	    public struct LUID { public uint LowPart; public int HighPart; }
    22
    23	    [StructLayout(LayoutKind.Sequential)]
    24	    public struct TOKEN_PRIVILEGES { public int Count; public LUID Luid; public int Attr; }
    25
    26	    [DllImport("advapi32.dll", SetLastError = true)]
    27	    public static extern bool OpenProcessToken(IntPtr h, int acc, ref IntPtr tok);
    28
    29	    [DllImport("advapi32.dll", SetLastError = true)]
    30	    public static extern bool LookupPrivilegeValue(string host, string name, ref LUID luid);
    31
    32	    [DllImport("advapi32.dll", SetLastError = true)]
    33	    public static extern bool AdjustTokenPrivileges(IntPtr tok, bool dis, ref TOKEN_PRIVILEGES newst, int len, IntPtr prev, IntPtr rel);
    34
    35	    [DllImport("kernel32.dll")]
    36	    public static extern IntPtr GetCurrentProcess();
    37
    38	    [DllImport("kernel32.dll", SetLastError = true)]
    39	    public static extern bool CloseHandle(IntPtr h);
    40
    41	    [DllImport("ntdll.dll")]
    42	    public static extern uint NtSetSystemInformation(int infoClass, ref int info, int len);
    43
    44	    const int ERROR_NOT_ALL_ASSIGNED = 1300;
    45
    46	    public static uint Purge()
    47	    {
    48	        IntPtr tok = IntPtr.Zero;
    49	        if (!OpenProcessToken(GetCurrentProcess(), 0x20 /*ADJUST*/ | 0x8 /*QUERY*/, ref tok))
    50	            throw new Win32Exception(Marshal.GetLastWin32Error(), "OpenProcessToken");
    51	        try
    52	        {
    53	            LUID luid = new LUID();
    54	            if (!LookupPrivilegeValue(null, "SeProfileSingleProcessPrivilege", ref luid))
    55	                throw new Win32Exception(Marshal.GetLastWin32Error(), "LookupPrivilegeValue");
    56	            TOKEN_PRIVILEGES tp;
    57	            tp.Count = 1; tp.Luid = luid; tp.Attr = 0x2 /*ENABLED*/;
    58	            if (!AdjustTokenPrivileges(tok, false, ref tp, 0, IntPtr.Zero, IntPtr.Zero))
    59	                throw new Win32Exception(Marshal.GetLastWin32Error(), "AdjustTokenPrivileges");
    60	            int gle = Marshal.GetLastWin32Error();
    61	            if (gle == ERROR_NOT_ALL_ASSIGNED)
    62	                throw new Win32Exception(gle,
    63	                    "SeProfileSingleProcessPrivilege was not assigned (token is not elevated?)");
    64	            int cmd = 4; // MemoryPurgeStandbyList
    65	            return NtSetSystemInformation(80 /*SystemMemoryListInformation*/, ref cmd, 4);
    66	        }
    67	        finally
    68	        {
    69	            CloseHandle(tok);
    70	        }
    71	    }
    72	}
    73	"@
    74
    75	$rc = [StandbyPurge]::Purge()
    76	if ($rc -ne 0) { throw "NtSetSystemInformation failed: 0x$($rc.ToString('x'))" }
    77	Write-Output "standby-purged"
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-6LW20gd0' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-a2NnJTx9' (errno=Operation not permitted)
100644 blob 3e4e9ccf1aa1685b3447f4091a3fdd3d4c1f84d8	scripts/windows/purge-standby.ps1
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-V1wH0Xkg' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Q8QwU45d' (errno=Operation not permitted)
.review/results/otp12-pf1-rigw-harness-r2.codex.md:1417: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18091: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18113: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18117: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18171: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18210: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18238: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18247: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18254: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18279: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18387: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18433: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18445: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18471: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18495: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18524: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18541: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18548: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18564: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18571: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18640: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18684: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18713: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18750: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18759: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18763: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18889: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18902: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:18907: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:19046: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:19070: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:19125: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:19172: trailing whitespace.
+
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20367: trailing whitespace.
+  1515
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20385: trailing whitespace.
+  1533
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20401: trailing whitespace.
+  1549
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20414: trailing whitespace.
+  1562
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20428: trailing whitespace.
+  1576
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20436: trailing whitespace.
+  1584
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20439: trailing whitespace.
+  1587
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20459: trailing whitespace.
+  1607
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20466: trailing whitespace.
+  1614
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20478: trailing whitespace.
+  1626
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20486: trailing whitespace.
+  1634
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20544: trailing whitespace.
+  1170
.review/results/otp12-pf1-rigw-harness-r2.codex.md:20571: trailing whitespace.
+  1197
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-vzUULOqG' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-odVZHDAC' (errno=Operation not permitted)
2600	0	scripts/bench_otp12pf_rigw.sh
2020	0	scripts/otp12pf_rigw_analyze.py
1069	0	scripts/otp12pf_rigw_analyze_test.py

exec
/bin/zsh -lc "rg -n 'enum Commands|Copy|copy|transfer\\(|TransferRequest|transfer_session|TransferRole|InitiatorRole|source.*remote|destination.*remote' crates/blit-cli/src crates/blit-core/src/transfer_session | head -400; git grep -n 'rpc.*transfer\\|\\.transfer(' 75a9a33ce600e4707438ed885de2ce0cdf27d946 -- crates/blit-cli crates/blit-core | head -200" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-cli/src/cli.rs:5:/// Appended to `--help` (not `-h`) for copy/mirror/move so the three
crates/blit-cli/src/cli.rs:9:  blit copy SRC/  DEST/   merge SRC's contents into DEST (no basename nesting)
crates/blit-cli/src/cli.rs:10:  blit copy SRC   DEST/   nest SRC under DEST  -> DEST/<basename(SRC)>
crates/blit-cli/src/cli.rs:11:  blit copy SRC   DEST    if DEST exists as a dir: nest; else DEST becomes the copy
crates/blit-cli/src/cli.rs:12:  blit copy f.txt DEST/   DEST/f.txt (into the directory)
crates/blit-cli/src/cli.rs:13:  blit copy f.txt new.txt rename (when new.txt does not exist)
crates/blit-cli/src/cli.rs:15:A trailing slash on SRC means \"copy the contents\". Without one, the basename is
crates/blit-cli/src/cli.rs:23:  blit check verifies that a destination tree matches what `blit copy` or
crates/blit-cli/src/cli.rs:63:pub enum Commands {
crates/blit-cli/src/cli.rs:64:    /// Copy files between local and/or remote locations (rsync-style slash semantics)
crates/blit-cli/src/cli.rs:65:    Copy(TransferArgs),
crates/blit-cli/src/cli.rs:68:    /// Move files (copy + remove source, rsync-style slash semantics)
crates/blit-cli/src/cli.rs:173:    /// Source path or remote endpoint (same syntax as `blit copy`)
crates/blit-cli/src/cli.rs:175:    /// Destination path or remote endpoint (same syntax as `blit copy`)
crates/blit-cli/src/cli.rs:206:    /// Trailing slash means "copy contents" (merge). Without a trailing slash,
crates/blit-cli/src/cli.rs:340:    /// Discard all writes — local copy only (read+pipeline benchmark).
crates/blit-cli/src/cli.rs:346:    ///   blit copy /data/large-dataset /tmp/unused --null -v
crates/blit-cli/src/cli.rs:348:    /// **Restrictions** (R54-F1): --null is supported only by `blit copy`
crates/blit-cli/src/cli.rs:352:    /// no copy), and with any remote endpoint (the remote push/pull
crates/blit-cli/src/cli.rs:545:/// `--exclude '*.tmp'` on `blit copy`.
crates/blit-cli/src/cli.rs:564:/// Use `blit check` to verify that a `blit copy` or `blit mirror`
crates/blit-cli/src/cli.rs:624:        let cli = Cli::try_parse_from(["blit", "copy", "src", "dst"]).expect("parse defaults");
crates/blit-cli/src/cli.rs:625:        let Commands::Copy(args) = cli.command else {
crates/blit-cli/src/cli.rs:626:            panic!("expected Copy");
crates/blit-cli/src/cli.rs:632:            Cli::try_parse_from(["blit", "copy", "--retry", "3", "--wait", "10", "src", "dst"])
crates/blit-cli/src/cli.rs:634:        let Commands::Copy(args) = cli.command else {
crates/blit-cli/src/cli.rs:635:            panic!("expected Copy");
crates/blit-cli/src/cli.rs:660:            "copy",
crates/blit-core/src/transfer_session/mod.rs:37:use crate::copy::DEFAULT_BLOCK_SIZE;
crates/blit-core/src/transfer_session/mod.rs:44:    TransferRole, TransferSummary,
crates/blit-core/src/transfer_session/mod.rs:485:fn complement(role: TransferRole) -> TransferRole {
crates/blit-core/src/transfer_session/mod.rs:487:        TransferRole::Source => TransferRole::Destination,
crates/blit-core/src/transfer_session/mod.rs:488:        TransferRole::Destination => TransferRole::Source,
crates/blit-core/src/transfer_session/mod.rs:489:        TransferRole::Unspecified => TransferRole::Unspecified,
crates/blit-core/src/transfer_session/mod.rs:640:#[derive(Clone, Copy, Default)]
crates/blit-core/src/transfer_session/mod.rs:685:    let initiator_role = match TransferRole::try_from(negotiated.open.initiator_role).ok()? {
crates/blit-core/src/transfer_session/mod.rs:686:        TransferRole::Source => SessionPhaseRole::Source,
crates/blit-core/src/transfer_session/mod.rs:687:        TransferRole::Destination => SessionPhaseRole::Destination,
crates/blit-core/src/transfer_session/mod.rs:688:        TransferRole::Unspecified => return None,
crates/blit-core/src/transfer_session/mod.rs:759:    local_role: TransferRole,
crates/blit-core/src/transfer_session/mod.rs:766:    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
crates/blit-core/src/transfer_session/mod.rs:809:                if local_role == TransferRole::Destination && resolved.read_only {
crates/blit-core/src/transfer_session/mod.rs:845:        receiver_capacity: if local_role == TransferRole::Destination {
crates/blit-core/src/transfer_session/mod.rs:869:    local_role: TransferRole,
crates/blit-core/src/transfer_session/mod.rs:1031:        let declared = TransferRole::try_from(open.initiator_role);
crates/blit-core/src/transfer_session/mod.rs:1032:        if declared != Ok(TransferRole::Source) {
crates/blit-core/src/transfer_session/mod.rs:1044:        TransferRole::Source,
crates/blit-core/src/transfer_session/mod.rs:2458:            let declared = TransferRole::try_from(open.initiator_role);
crates/blit-core/src/transfer_session/mod.rs:2459:            if declared != Ok(TransferRole::Destination) {
crates/blit-core/src/transfer_session/mod.rs:2486:        TransferRole::Destination,
crates/blit-core/src/transfer_session/mod.rs:2587:    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
crates/blit-core/src/transfer_session/mod.rs:2590:        TransferRole::Source => {
crates/blit-core/src/transfer_session/mod.rs:2598:                TransferRole::Destination,
crates/blit-core/src/transfer_session/mod.rs:2635:        TransferRole::Destination => {
crates/blit-core/src/transfer_session/mod.rs:2643:                TransferRole::Source,
crates/blit-core/src/transfer_session/mod.rs:2680:        TransferRole::Unspecified => Err(notify_and_wrap(
crates/blit-cli/src/main.rs:56:        Commands::Copy(args) => {
crates/blit-cli/src/main.rs:59:                run_transfer(&ctx, &args, TransferKind::Copy)
crates/blit-cli/src/main.rs:66:                run_transfer(&ctx, &args, TransferKind::Mirror)
crates/blit-core/src/transfer_session/local.rs:11://! block-clone / copy_file_range where the platform has them), so no
crates/blit-core/src/transfer_session/local.rs:27:use crate::generated::{FileHeader, MirrorMode, SessionOpen, TransferRole};
crates/blit-core/src/transfer_session/local.rs:53:#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
crates/blit-core/src/transfer_session/local.rs:67:/// so local copy/mirror behaves the same as a same-options remote run.
crates/blit-core/src/transfer_session/local.rs:68:#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
crates/blit-core/src/transfer_session/local.rs:134:/// Options for executing a local mirror/copy operation. The dead
crates/blit-core/src/transfer_session/local.rs:153:    /// `--force` honored on local copy/mirror the same way the remote
crates/blit-core/src/transfer_session/local.rs:354:    /// pipeline's copy-what-is-readable posture; the caller-side move
crates/blit-core/src/transfer_session/local.rs:426:/// `nested_destination_does_not_self_copy`: without it, each run
crates/blit-core/src/transfer_session/local.rs:541:        initiator_role: TransferRole::Source as i32,
crates/blit-core/src/transfer_session/local.rs:743:        TransferMode::Copy
crates/blit-core/src/transfer_session/local.rs:779:    use crate::transfer_session::DestinationOutcome;
crates/blit-core/src/transfer_session/local.rs:857:            initiator_role: TransferRole::Source as i32,
crates/blit-core/src/transfer_session/local.rs:1222:            initiator_role: TransferRole::Source as i32,
crates/blit-cli/src/diagnostics.rs:112:            blit_core::perf_history::TransferMode::Copy => "copy",
crates/blit-cli/src/transfers/mod.rs:42:use blit_app::endpoints::{ensure_remote_destination_supported, ensure_remote_source_supported};
crates/blit-cli/src/transfers/mod.rs:101:pub async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: TransferKind) -> Result<()> {
crates/blit-cli/src/transfers/mod.rs:108:        TransferKind::Copy => "copy",
crates/blit-cli/src/transfers/mod.rs:122:    //   - `blit copy --null` to/from a remote endpoint: the
crates/blit-cli/src/transfers/mod.rs:128:    // copy only. Reject the other combinations at the CLI;
crates/blit-cli/src/transfers/mod.rs:139:                 operation. Use `blit copy --null SRC DST` (local \
crates/blit-cli/src/transfers/mod.rs:151:                 `blit copy --null SRC DST` between two local \
crates/blit-cli/src/transfers/mod.rs:204:            run_local_transfer(ctx, args, &src, &dst, mirror)
crates/blit-cli/src/transfers/mod.rs:214:            run_remote_push_transfer(args, src, dst, mirror).await
crates/blit-cli/src/transfers/mod.rs:219:            run_remote_pull_transfer(
crates/blit-cli/src/transfers/mod.rs:248:        // source around forever (silent move-becomes-copy) or
crates/blit-cli/src/transfers/mod.rs:254:             would silently turn a move into a copy. Use \
crates/blit-cli/src/transfers/mod.rs:255:             `blit copy --detach SRC DST` and `blit rm SRC` once you've \
crates/blit-cli/src/transfers/mod.rs:281:             Run `blit copy` with filters first, then `blit rm` the \
crates/blit-cli/src/transfers/mod.rs:299:             `blit copy --ignore-existing` first, then `blit rm` \
crates/blit-cli/src/transfers/mod.rs:316:             copy. Use --null only with `blit copy SRC DST` \
crates/blit-cli/src/transfers/mod.rs:333:    // destroying the only copy, the exact hazard the move mapping
crates/blit-cli/src/transfers/mod.rs:398:            // means "copy + delete source," NOT "purge unrelated
crates/blit-cli/src/transfers/mod.rs:407:            // "successful copy" document.
crates/blit-cli/src/transfers/mod.rs:415:            // during the copy and then permanently removed from
crates/blit-cli/src/transfers/mod.rs:429:                     skipped during the copy — deleting the source \
crates/blit-cli/src/transfers/mod.rs:469:            ensure_remote_source_supported(&remote)?;
crates/blit-cli/src/transfers/mod.rs:472:            // source via delete_remote_path below, and the move
crates/blit-cli/src/transfers/mod.rs:503:            ensure_remote_destination_supported(&remote)?;
crates/blit-cli/src/transfers/mod.rs:575:    fn copy_local_transfers_file() -> Result<()> {
crates/blit-cli/src/transfers/mod.rs:616:        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-cli/src/transfers/mod.rs:623:    fn copy_local_dry_run_creates_no_files() -> Result<()> {
crates/blit-cli/src/transfers/mod.rs:664:        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-cli/src/transfers/mod.rs:672:    // above (`copy_local_transfers_file`,
crates/blit-cli/src/transfers/mod.rs:673:    // `copy_local_dry_run_creates_no_files`).
crates/blit-cli/src/transfers/mod.rs:734:            .block_on(run_transfer(&ctx, &args, TransferKind::Copy))
crates/blit-cli/src/transfers/remote.rs:170:pub async fn run_remote_push_transfer(
crates/blit-cli/src/transfers/remote.rs:176:    run_remote_push_transfer_inner(args, source, remote, mirror_mode, false, false)
crates/blit-cli/src/transfers/remote.rs:189:/// same-size file whose content differs would destroy the only copy;
crates/blit-cli/src/transfers/remote.rs:190:/// the mapping makes the delete safe by construction. Copy/mirror map
crates/blit-cli/src/transfers/remote.rs:191:/// through the shared copy mapping (SizeMtime default, whose
crates/blit-cli/src/transfers/remote.rs:199:    run_remote_push_transfer_inner(args, source, remote, mirror_mode, true, true).await
crates/blit-cli/src/transfers/remote.rs:226:    use blit_core::transfer_session::SessionFault;
crates/blit-cli/src/transfers/remote.rs:338:pub async fn run_remote_pull_transfer(
crates/blit-cli/src/transfers/remote.rs:351:        false, // emit success summary inline (copy/mirror default)
crates/blit-cli/src/transfers/remote.rs:479:    // JSON. Keys only the deleted driver could fill (bytes_zero_copy —
crates/blit-cli/src/transfers/remote.rs:499:    // (files_requested, bytes_zero_copy, first_payload_ms) are gone;
crates/blit-cli/src/transfers/remote.rs:516:    // keep their exact wording; the old driver-only zero-copy clause
crates/blit-cli/src/transfers/remote.rs:517:    // is gone (always 0 on the session — zero-copy returns as a
crates/blit-cli/src/transfers/remote.rs:600:    use blit_core::transfer_session::SessionFault;
crates/blit-cli/src/check.rs:31:    // Build filter via the same chokepoint that copy/mirror/move use, so
crates/blit-cli/src/check.rs:32:    // `blit check --exclude '*.tmp'` matches `blit copy --exclude '*.tmp'`.
crates/blit-cli/src/transfers/local.rs:4:use blit_core::transfer_session::{LocalMirrorOptions, LocalMirrorSummary, TransferOutcome};
crates/blit-cli/src/transfers/local.rs:11:/// printed inline. Most CLI paths (copy / mirror) want this; move
crates/blit-cli/src/transfers/local.rs:15:pub async fn run_local_transfer(
crates/blit-cli/src/transfers/local.rs:33:/// copy, the same otp-10a F1 hazard the remote move verbs closed.
crates/blit-cli/src/transfers/local.rs:117:            if mirror { "Mirroring" } else { "Copying" },
crates/blit-cli/src/transfers/local.rs:151:    use blit_core::transfer_session::{LocalCompareMode, LocalMirrorDeleteScope};
crates/blit-cli/src/transfers/local.rs:243:    let operation = if mirror { "Mirror" } else { "Copy" };
crates/blit-cli/src/transfers/local.rs:359:        "operation": if mirror { "mirror" } else { "copy" },
crates/blit-cli/src/profile.rs:11:                "copy": coefficient_json(&pred.copy),
crates/blit-cli/src/profile.rs:67:    print_coefficient_block("copy", &pred.copy);
crates/blit-cli/src/transfers/remote_remote_direct.rs:70:/// skipped and the source-delete destroyed the only copy — the exact
crates/blit-cli/src/transfers/remote_remote_direct.rs:182:            bytes_zero_copy: 0,
crates/blit-cli/src/transfers/remote_remote_direct.rs:252:        "source": format_remote_endpoint(src),
crates/blit-cli/src/transfers/remote_remote_direct.rs:253:        "destination": format_remote_endpoint(dst),
crates/blit-cli/src/transfers/remote_remote_direct.rs:256:        "bytes_zero_copy": summary.bytes_zero_copy,
crates/blit-cli/src/transfers/remote_remote_direct.rs:270:        "Delegated remote-to-remote transfer complete: {} file(s), {} bytes (zero-copy {} bytes){} from {} to {}.",
crates/blit-cli/src/transfers/remote_remote_direct.rs:273:        summary.bytes_zero_copy,
crates/blit-cli/src/transfers/remote_remote_direct.rs:370:    /// Copy/mirror keep the user's flags untouched (the old
crates/blit-cli/src/transfers/remote_remote_direct.rs:373:    fn delegated_copy_passes_compare_flags_through() {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-mrvcO88q' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HTp0rU25' (errno=Operation not permitted)
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_client.rs:156:        .transfer(ReceiverStream::new(out_rx))
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_client.rs:327:        .transfer(ReceiverStream::new(out_rx))

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-cli/src/transfers/remote.rs | nl -ba | sed -n '140,410p'; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_client.rs | nl -ba | sed -n '100,380p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XR92Q8eO' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-FbA5eIUU' (errno=Operation not permitted)
   140	            }
   141	        } else if !totals.started() && totals.manifest_files > 0 {
   142	            if json {
   143	                eprintln!(
   144	                    "{{\"event\":\"manifest\",\"total_files\":{}}}",
   145	                    totals.manifest_files
   146	                );
   147	            } else {
   148	                println!(
   149	                    "[progress] manifest enumerated {} file(s)",
   150	                    totals.manifest_files
   151	                );
   152	            }
   153	        }
   154	    });
   155
   156	    (Some(progress), Some(join))
   157	}
   158
   159	/// The user's compare flags, lifted off clap once for both verbs —
   160	/// the inputs to the one `transfers::compare` mapping (otp-10b-2).
   161	fn verb_compare_flags(args: &TransferArgs) -> CompareFlags {
   162	    CompareFlags {
   163	        checksum: args.checksum,
   164	        size_only: args.size_only,
   165	        ignore_times: args.ignore_times,
   166	        force: args.force,
   167	    }
   168	}
   169
   170	pub async fn run_remote_push_transfer(
   171	    args: &TransferArgs,
   172	    source: PathBuf,
   173	    remote: RemoteEndpoint,
   174	    mirror_mode: bool,
   175	) -> Result<()> {
   176	    run_remote_push_transfer_inner(args, source, remote, mirror_mode, false, false)
   177	        .await
   178	        .map(|_| ())
   179	}
   180
   181	/// R51-F4: move's variant of [`run_remote_push_transfer`]. Returns
   182	/// the push summary instead of printing inline so the caller can
   183	/// defer output until after source-delete.
   184	///
   185	/// codex otp-10a F1: move maps through `move_comparison_mode` —
   186	/// `IgnoreTimes` (transfer every file unconditionally), or `Checksum`
   187	/// when the user asked for it (a content-proven skip is safe). Move
   188	/// deletes the source on success, so a metadata-shaped skip of a
   189	/// same-size file whose content differs would destroy the only copy;
   190	/// the mapping makes the delete safe by construction. Copy/mirror map
   191	/// through the shared copy mapping (SizeMtime default, whose
   192	/// same-size dest-newer skip is the standing owner question).
   193	pub async fn run_remote_push_transfer_deferred(
   194	    args: &TransferArgs,
   195	    source: PathBuf,
   196	    remote: RemoteEndpoint,
   197	    mirror_mode: bool,
   198	) -> Result<DeferredPushState> {
   199	    run_remote_push_transfer_inner(args, source, remote, mirror_mode, true, true).await
   200	}
   201
   202	pub struct DeferredPushState {
   203	    pub summary: blit_core::generated::TransferSummary,
   204	    pub destination: String,
   205	}
   206
   207	pub fn print_deferred_push_result(args: &TransferArgs, state: &DeferredPushState) {
   208	    if args.json {
   209	        print_push_json(&state.summary, &state.destination);
   210	    } else {
   211	        describe_push_result(&state.summary, &state.destination);
   212	    }
   213	}
   214
   215	/// otp-10a: a failed session names the file a fault touched
   216	/// (D-2026-07-09-1) — extract that end-of-operation summary from the
   217	/// error chain, so the operator sees which file to re-run for without
   218	/// digging through it. Applies to both fault shapes: a `SessionFault`
   219	/// raised by a running session and a `TransferOpenRefusal` from a
   220	/// session that never opened (whose inner fault never names a file —
   221	/// `end_of_operation_summary` then returns `None`). Extraction is
   222	/// split from the printing so the chain-walking is unit-pinned
   223	/// (codex otp-10a F7).
   224	fn session_fault_summary(err: &eyre::Report) -> Option<String> {
   225	    use blit_core::remote::transfer::session_client::TransferOpenRefusal;
   226	    use blit_core::transfer_session::SessionFault;
   227	    err.chain()
   228	        .find_map(|cause| {
   229	            cause
   230	                .downcast_ref::<SessionFault>()
   231	                .or_else(|| cause.downcast_ref::<TransferOpenRefusal>().map(|r| &r.0))
   232	        })
   233	        .and_then(|fault| fault.end_of_operation_summary())
   234	}
   235
   236	fn emit_session_fault_summary(err: &eyre::Report) {
   237	    if let Some(line) = session_fault_summary(err) {
   238	        eprintln!("{line}");
   239	    }
   240	}
   241
   242	async fn run_remote_push_transfer_inner(
   243	    args: &TransferArgs,
   244	    source: PathBuf,
   245	    remote: RemoteEndpoint,
   246	    mirror_mode: bool,
   247	    move_verb: bool,
   248	    defer_output: bool,
   249	) -> Result<DeferredPushState> {
   250	    let show_progress = args.effective_progress() || args.verbose;
   251	    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
   252	        show_progress,
   253	        args.verbose,
   254	        args.json,
   255	        defer_output, // R53-F1: suppress the final progress line on move
   256	    );
   257
   258	    // Filter parity: the wire FilterSpec rides `SessionOpen.filter`
   259	    // (otp-10a); the session's SOURCE end applies it through the
   260	    // universal `FilteredSource` chokepoint and the daemon DESTINATION
   261	    // scopes mirror deletions with it — identical rules to what
   262	    // `--exclude/--include/--min-size/...` produce on pull.
   263	    let filter_spec = super::build_filter_spec(args)?;
   264
   265	    // R59 #1 F2: translate the user's --delete-scope flag to the wire
   266	    // MirrorMode enum. Default to FilteredSubset so `push --include …
   267	    // --mirror` deletes only files in scope. R59 #1 F1: require a
   268	    // complete source scan for any mirror operation — a partial scan
   269	    // could cause silent dest-side data loss when the daemon purges
   270	    // entries it (wrongly) thinks are absent from the source.
   271	    let mirror_kind = if mirror_mode {
   272	        if args.delete_scope_all() {
   273	            blit_core::generated::MirrorMode::All
   274	        } else {
   275	            blit_core::generated::MirrorMode::FilteredSubset
   276	        }
   277	    } else {
   278	        blit_core::generated::MirrorMode::Off
   279	    };
   280
   281	    // otp-10b-2: the ONE args→compare mapping, shared with the pull
   282	    // verb (the old push driver ignored every compare flag).
   283	    let compare_mode = if move_verb {
   284	        move_comparison_mode(verb_compare_flags(args))
   285	    } else {
   286	        comparison_mode(verb_compare_flags(args))
   287	    };
   288
   289	    let execution = PushExecution {
   290	        source,
   291	        remote: remote.clone(),
   292	        filter: Some(filter_spec),
   293	        mirror_mode,
   294	        mirror_kind,
   295	        force_grpc: args.force_grpc,
   296	        trace_data_plane: args.trace_data_plane,
   297	        // Mirror needs a complete source scan (R59 #1 F1). Move-push
   298	        // keeps otp-10a's posture instead: the readable subset lands,
   299	        // the unreadable accumulator fails the call, and the deferred
   300	        // print + source-delete gate never fire.
   301	        require_complete_scan: mirror_mode,
   302	        resume: args.resume,
   303	        resume_block_size: 0, // destination default (1 MiB)
   304	        compare_mode,
   305	        ignore_existing: args.ignore_existing,
   306	        remote_label: format_remote_endpoint(&remote),
   307	    };
   308
   309	    // Push has no caller-side destructive step (mirror-delete is
   310	    // daemon-side and surfaces via the summary), so unlike the pull
   311	    // lifecycle there is no need to drop the progress handle
   312	    // *before* a follow-up library call — the monitor's lifetime
   313	    // already matches the RPC.
   314	    let outcome = run_remote_push(execution, progress_handle.as_ref()).await;
   315
   316	    drop(progress_handle);
   317	    if let Some(task) = progress_task {
   318	        let _ = task.await;
   319	    }
   320
   321	    let outcome = match outcome {
   322	        Ok(outcome) => outcome,
   323	        Err(err) => {
   324	            emit_session_fault_summary(&err);
   325	            return Err(err);
   326	        }
   327	    };
   328	    let state = DeferredPushState {
   329	        summary: outcome.summary,
   330	        destination: outcome.destination,
   331	    };
   332	    if !defer_output {
   333	        print_deferred_push_result(args, &state);
   334	    }
   335	    Ok(state)
   336	}
   337
   338	pub async fn run_remote_pull_transfer(
   339	    args: &TransferArgs,
   340	    remote: RemoteEndpoint,
   341	    dest_root: &Path,
   342	    mirror_mode: bool,
   343	    move_verb: bool,
   344	) -> Result<()> {
   345	    run_remote_pull_transfer_inner(
   346	        args,
   347	        remote,
   348	        dest_root,
   349	        mirror_mode,
   350	        move_verb,
   351	        false, // emit success summary inline (copy/mirror default)
   352	    )
   353	    .await
   354	    .map(|_| ())
   355	}
   356
   357	/// R51-F4: move's variant of `run_remote_pull_transfer` — runs the
   358	/// transfer but does NOT emit the success summary. Caller is
   359	/// responsible for printing after source-delete completes (or
   360	/// refusing to print on source-delete failure).
   361	pub async fn run_remote_pull_transfer_deferred(
   362	    args: &TransferArgs,
   363	    remote: RemoteEndpoint,
   364	    dest_root: &Path,
   365	    mirror_mode: bool,
   366	    move_verb: bool,
   367	) -> Result<DeferredPullState> {
   368	    run_remote_pull_transfer_inner(args, remote, dest_root, mirror_mode, move_verb, true).await
   369	}
   370
   371	pub fn print_deferred_pull_result(args: &TransferArgs, state: &DeferredPullState) {
   372	    if args.json {
   373	        print_pull_json(&state.summary, &state.dest_root);
   374	    } else {
   375	        describe_pull_result(&state.summary, &state.dest_root);
   376	    }
   377	}
   378
   379	async fn run_remote_pull_transfer_inner(
   380	    args: &TransferArgs,
   381	    remote: RemoteEndpoint,
   382	    dest_root: &Path,
   383	    mirror_mode: bool,
   384	    move_verb: bool,
   385	    defer_output: bool,
   386	) -> Result<DeferredPullState> {
   387	    // Filter parity: the wire FilterSpec rides `SessionOpen.filter`
   388	    // (otp-10b-2); the daemon SOURCE applies it through the universal
   389	    // `FilteredSource` chokepoint and this DESTINATION scopes mirror
   390	    // deletions with it — identical rules to push, by construction.
   391	    let filter_spec = super::build_filter_spec(args)?;
   392
   393	    let show_progress = args.effective_progress() || args.verbose;
   394	    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
   395	        show_progress,
   396	        args.verbose,
   397	        args.json,
   398	        defer_output, // R53-F1: suppress final progress line on move
   399	    );
   400
   401	    // R59 #1 F2: --delete-scope → wire MirrorMode, same mapping as the
   402	    // push verb (FilteredSubset default so `--include … --mirror`
   403	    // deletes only in-scope entries).
   404	    let mirror_kind = if mirror_mode {
   405	        if args.delete_scope_all() {
   406	            blit_core::generated::MirrorMode::All
   407	        } else {
   408	            blit_core::generated::MirrorMode::FilteredSubset
   409	        }
   410	    } else {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LokpJ4wv' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-QoQTN1Yc' (errno=Operation not permitted)
   100	            resume_block_size: 0,
   101	            filter: None,
   102	            mirror_enabled: false,
   103	            mirror_kind: MirrorMode::Off,
   104	            progress: None,
   105	            trace_data_plane: false,
   106	        }
   107	    }
   108	}
   109
   110	/// Connect to `endpoint`'s daemon and run one SOURCE-role transfer
   111	/// session pushing `source`'s tree into the endpoint's module/path.
   112	/// Returns the destination-computed [`TransferSummary`] (contract:
   113	/// DESTINATION is the scorer).
   114	pub async fn run_push_session(
   115	    endpoint: &RemoteEndpoint,
   116	    source: Arc<dyn TransferSource>,
   117	    options: PushSessionOptions,
   118	) -> Result<TransferSummary> {
   119	    // The responder resolves module→root; the initiator's own local
   120	    // path never crosses the wire (contract §SessionOpen).
   121	    let (module, path) = endpoint_module_path(endpoint)?;
   122
   123	    let mut client = connect_transfer_client(endpoint).await?;
   124
   125	    let open = SessionOpen {
   126	        initiator_role: TransferRole::Source as i32,
   127	        module,
   128	        path,
   129	        compare_mode: options.compare_mode as i32,
   130	        ignore_existing: options.ignore_existing,
   131	        require_complete_scan: options.require_complete_scan,
   132	        // otp-4b: default to the TCP data plane; the responder grants it
   133	        // in SessionAccept unless this asks for the in-stream fallback.
   134	        in_stream_bytes: options.in_stream_bytes,
   135	        // otp-7b: resume rides the open (plan D6 — the flag is in the
   136	        // open, so resume runs identically whichever end initiated).
   137	        resume: options.resume.then_some(ResumeSettings {
   138	            enabled: true,
   139	            block_size: options.resume_block_size,
   140	        }),
   141	        // otp-10a: filter + mirror ride the open (otp-6a/6b session
   142	        // support; this is the client wiring, symmetric with pull's
   143	        // otp-9a).
   144	        filter: options.filter,
   145	        mirror_enabled: options.mirror_enabled,
   146	        mirror_kind: options.mirror_kind as i32,
   147	        ..Default::default()
   148	    };
   149
   150	    // Open the bidi RPC: the request stream is fed by `out_tx`, the
   151	    // response stream is the inbound half. The handler returns its
   152	    // response stream immediately (it spawns the session), so this
   153	    // await resolves before any frame flows — no deadlock.
   154	    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
   155	    let inbound = client
   156	        .transfer(ReceiverStream::new(out_rx))
   157	        .await
   158	        .map_err(|status| eyre::Report::new(transfer_open_refusal(status)))?
   159	        .into_inner();
   160	    let transport = grpc_client_transport(out_tx, inbound);
   161
   162	    // otp-10a: own the unreadable-scan accumulator so a partial source
   163	    // scan fails the push after the session completes — the old push
   164	    // driver's exact posture (send what's readable, then error), which
   165	    // `blit move`'s source-delete gate relies on: an error here means
   166	    // move never deletes a source whose files were silently skipped.
   167	    let unreadable: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
   168
   169	    let cfg = SourceSessionConfig {
   170	        hello: HelloConfig::default(),
   171	        endpoint: SessionEndpoint::initiator(open),
   172	        plan_options: options.plan_options,
   173	        // The initiator dials the data plane on the same host it reached
   174	        // the control plane on (contract §Transport: initiator dials).
   175	        data_plane_host: Some(endpoint.host.clone()),
   176	        instruments: SourceInstruments {
   177	            progress: options.progress,
   178	            unreadable: Some(Arc::clone(&unreadable)),
   179	            trace_data_plane: options.trace_data_plane,
   180	            session_phase_trace: Default::default(),
   181	        },
   182	    };
   183	    let summary = run_source(cfg, transport, source).await?;
   184
   185	    let unreadable = unreadable
   186	        .lock()
   187	        .map_err(|err| eyre!("unreadable-path accumulator poisoned: {err}"))?;
   188	    if !unreadable.is_empty() {
   189	        let preview: Vec<_> = unreadable.iter().take(5).cloned().collect();
   190	        let mut message = format!(
   191	            "{} file(s) were skipped due to permission or access errors: {}",
   192	            unreadable.len(),
   193	            preview.join(", ")
   194	        );
   195	        if unreadable.len() > preview.len() {
   196	            message.push_str(&format!(" (and {} more)", unreadable.len() - preview.len()));
   197	        }
   198	        return Err(eyre!(message));
   199	    }
   200	    Ok(summary)
   201	}
   202
   203	/// The pull-shaped subset of session options the landed slices support.
   204	/// Mirror and filters ride the open since otp-9a (the session honors
   205	/// them since otp-6). The DESTINATION owns the compare decision; the
   206	/// SOURCE owns the planner knobs (none cross the wire).
   207	pub struct PullSessionOptions {
   208	    pub compare_mode: ComparisonMode,
   209	    pub ignore_existing: bool,
   210	    pub require_complete_scan: bool,
   211	    /// Force the in-stream byte carrier instead of the TCP data plane
   212	    /// (otp-5b). Default `false` = the SOURCE responder grants a data
   213	    /// plane and this DESTINATION initiator dials + receives over TCP
   214	    /// sockets; `true` is the diagnostics / unreachable data-plane
   215	    /// fallback. Symmetric with [`PushSessionOptions::in_stream_bytes`].
   216	    pub in_stream_bytes: bool,
   217	    /// otp-7b: negotiate the resume block phase — symmetric with
   218	    /// [`PushSessionOptions::resume`] (plan D6: the flag is in the open,
   219	    /// so resume runs identically whichever end initiated).
   220	    pub resume: bool,
   221	    /// Requested resume block size in bytes; `0` lets the DESTINATION
   222	    /// (this end) choose. Ignored unless `resume` is true.
   223	    pub resume_block_size: u32,
   224	    /// otp-9a: source-side scan filter, riding `SessionOpen.filter`
   225	    /// (the session honors it since otp-6a — this is the client
   226	    /// wiring). `None` scans everything.
   227	    pub filter: Option<FilterSpec>,
   228	    /// otp-9a: mirror on the session (otp-6b's one delete rule — this
   229	    /// DESTINATION diffs the complete source manifest against its tree
   230	    /// at SourceDone and deletes extraneous entries locally). Explicit
   231	    /// enabled + scope per the contract; `MirrorMode::Off` with
   232	    /// `mirror_enabled` set is refused at OPEN.
   233	    pub mirror_enabled: bool,
   234	    pub mirror_kind: MirrorMode,
   235	    /// otp-9a: live counter the session sink reports applied payload
   236	    /// bytes against (the delegated dst daemon's jobs row, otp-9).
   237	    pub byte_progress: Option<ByteProgressSink>,
   238	    /// otp-10b-2: w6-1 progress events from this DESTINATION's receive
   239	    /// side — need batches as the denominator, `Payload`/`FileComplete`
   240	    /// per record received on either carrier. The CLI progress line and
   241	    /// the TUI footer consume these exactly as they did from the old
   242	    /// driver. Symmetric with [`PushSessionOptions::progress`].
   243	    pub progress: Option<RemoteTransferProgress>,
   244	    /// otp-10b-2: emit `[data-plane-client]` connect traces on the data
   245	    /// plane sockets this DESTINATION dials (`--trace-data-plane`).
   246	    pub trace_data_plane: bool,
   247	}
   248
   249	impl Default for PullSessionOptions {
   250	    fn default() -> Self {
   251	        Self {
   252	            compare_mode: ComparisonMode::SizeMtime,
   253	            ignore_existing: false,
   254	            require_complete_scan: false,
   255	            in_stream_bytes: false,
   256	            resume: false,
   257	            resume_block_size: 0,
   258	            filter: None,
   259	            mirror_enabled: false,
   260	            mirror_kind: MirrorMode::Off,
   261	            byte_progress: None,
   262	            progress: None,
   263	            trace_data_plane: false,
   264	        }
   265	    }
   266	}
   267
   268	/// Connect to `endpoint`'s daemon and run one DESTINATION-role transfer
   269	/// session pulling the endpoint's module/path tree into `dest_root`
   270	/// (pull-equivalent, otp-5a). The client initiates and declares
   271	/// DESTINATION, so the daemon becomes the SOURCE Responder (streaming
   272	/// its module tree). Returns the [`DestinationOutcome`] this end
   273	/// computed (contract: the DESTINATION is the scorer).
   274	///
   275	/// otp-5b: the default carrier is the TCP data plane — the SOURCE
   276	/// responder binds+grants+accepts sockets while sending, and this
   277	/// DESTINATION initiator dials + receives over them (the transport/role
   278	/// decoupling). `PullSessionOptions::in_stream_bytes` forces the in-stream
   279	/// fallback (diagnostics / unreachable data plane).
   280	pub async fn run_pull_session(
   281	    endpoint: &RemoteEndpoint,
   282	    dest_root: PathBuf,
   283	    options: PullSessionOptions,
   284	) -> Result<DestinationOutcome> {
   285	    let client = connect_transfer_client(endpoint).await?;
   286	    run_pull_session_with_client(client, endpoint, dest_root, options).await
   287	}
   288
   289	/// [`run_pull_session`] over an already-connected client (otp-9b). The
   290	/// delegated dst daemon connects separately so a connect failure keeps
   291	/// its own error phase (`ConnectSource`) structurally, without string
   292	/// matching on the session error.
   293	pub async fn run_pull_session_with_client(
   294	    mut client: BlitClient<Channel>,
   295	    endpoint: &RemoteEndpoint,
   296	    dest_root: PathBuf,
   297	    options: PullSessionOptions,
   298	) -> Result<DestinationOutcome> {
   299	    let (module, path) = endpoint_module_path(endpoint)?;
   300
   301	    let open = SessionOpen {
   302	        initiator_role: TransferRole::Destination as i32,
   303	        module,
   304	        path,
   305	        compare_mode: options.compare_mode as i32,
   306	        ignore_existing: options.ignore_existing,
   307	        require_complete_scan: options.require_complete_scan,
   308	        // otp-5b: default to the TCP data plane; the SOURCE responder
   309	        // grants it in SessionAccept unless this asks for the in-stream
   310	        // fallback.
   311	        in_stream_bytes: options.in_stream_bytes,
   312	        // otp-7b: resume rides the open, role-agnostic (plan D6).
   313	        resume: options.resume.then_some(ResumeSettings {
   314	            enabled: true,
   315	            block_size: options.resume_block_size,
   316	        }),
   317	        // otp-9a: filter + mirror ride the open (otp-6a/6b session
   318	        // support; this is the client wiring).
   319	        filter: options.filter,
   320	        mirror_enabled: options.mirror_enabled,
   321	        mirror_kind: options.mirror_kind as i32,
   322	        ..Default::default()
   323	    };
   324
   325	    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
   326	    let inbound = client
   327	        .transfer(ReceiverStream::new(out_rx))
   328	        .await
   329	        .map_err(|status| eyre::Report::new(transfer_open_refusal(status)))?
   330	        .into_inner();
   331	    let transport = grpc_client_transport(out_tx, inbound);
   332
   333	    let cfg = DestinationSessionConfig {
   334	        hello: HelloConfig::default(),
   335	        endpoint: SessionEndpoint::initiator(open),
   336	        // The initiator dials the data plane on the same host it reached
   337	        // the control plane on (contract §Transport: initiator dials).
   338	        data_plane_host: Some(endpoint.host.clone()),
   339	        instruments: DestinationInstruments {
   340	            progress: options.progress,
   341	            byte_progress: options.byte_progress,
   342	            trace_data_plane: options.trace_data_plane,
   343	            session_phase_trace: Default::default(),
   344	        },
   345	        local_apply: None,
   346	    };
   347	    run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await
   348	}
   349
   350	/// Derive the wire `(module, path)` from a resolved endpoint. Empty
   351	/// module targets the daemon's default root export; a discovery-form
   352	/// endpoint is not resolvable to a transfer target. The path is
   353	/// POSIX-normalized (codex otp-10a F2): a `rel_path` that went through
   354	/// `PathBuf::join` (the CLI's rsync destination-resolution rule does)
   355	/// carries native `\` separators on Windows, and `to_string_lossy`
   356	/// would put them on the wire verbatim — a Unix daemon then creates a
   357	/// literal `sub\dir` entry. Every wire-bound relative path routes
   358	/// through `path_posix` (the win-1 rule).
   359	fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
   360	    use crate::path_posix::relative_path_to_posix;
   361	    match &endpoint.path {
   362	        RemotePath::Module { module, rel_path } => {
   363	            Ok((module.clone(), relative_path_to_posix(rel_path)))
   364	        }
   365	        RemotePath::Root { rel_path } => Ok((String::new(), relative_path_to_posix(rel_path))),
   366	        RemotePath::Discovery => Err(eyre!(
   367	            "a transfer session needs a resolved module or root endpoint, not a discovery form"
   368	        )),
   369	    }
   370	}
   371
   372	#[cfg(test)]
   373	mod endpoint_module_path_tests {
   374	    use super::*;
   375	    use std::path::PathBuf;
   376
   377	    fn endpoint(rel_path: PathBuf) -> RemoteEndpoint {
   378	        RemoteEndpoint {
   379	            host: "h".into(),
   380	            port: 9031,

exec
/bin/zsh -lc "git grep -n 'max_streams\\|receiver_capacity\\|CapacityProfile' 75a9a33ce600e4707438ed885de2ce0cdf27d946 -- crates/blit-core/src/remote/transfer crates/blit-core/src/transfer_session crates/blit-cli/src | head -350; git grep -n 'data_plane_streams.*8\\|worker.*parity\\|both initiator\\|destination.*capacity' 75a9a33ce600e4707438ed885de2ce0cdf27d946 -- crates/blit-core/tests crates/blit-core/src/transfer_session | head -250" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-iEk4Ek1h' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HDCfrZbr' (errno=Operation not permitted)
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:53:use crate::dial::{initial_stream_proposal, local_receiver_capacity, TransferDial};
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:54:use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:164:    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:165:    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:195:    /// The receiver's advertised `max_streams` — the control loop refuses
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:240:        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:842:/// contract §Transport: the initiator always dials). `receiver_capacity`
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:849:    receiver_capacity: Option<&CapacityProfile>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:858:    let dial = TransferDial::conservative_within(receiver_capacity).shared();
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:870:        dial.ceiling_max_streams().max(1),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:964:/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:971:    receiver_capacity: Option<&CapacityProfile>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:981:    let dial = TransferDial::conservative_within(receiver_capacity).shared();
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:991:        dial.ceiling_max_streams().max(1),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:1092:            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:845:        receiver_capacity: if local_role == TransferRole::Destination {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:846:            Some(crate::dial::local_receiver_capacity())
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:1342:                negotiated.open.receiver_capacity.as_ref(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:1362:                        negotiated.accept.receiver_capacity.as_ref(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:2469:            if open.receiver_capacity.is_none() {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:2470:                open.receiver_capacity = Some(crate::dial::local_receiver_capacity());
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:2944:    // max_streams — both directions resize (push arms+accepts, otp-4b-2;
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:2989:                    // even when the caller advertised a max_streams below this
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:2994:                        negotiated.open.receiver_capacity.as_ref(),
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-bBonCsFn' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jp2y6gc3' (errno=Operation not permitted)

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '740,875p'; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '2420,2510p'; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/dial.rs | nl -ba | sed -n '1,280p'; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/tests/transfer_session_roles.rs | nl -ba | rg -n '8|capacity|worker|stream|initiator' | tail -120" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-xuI3pn8e' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-e8zhtG9M' (errno=Operation not permitted)
   740	            peer_notified: false,
   741	            relative_path: None,
   742	            io_kind: None,
   743	        };
   744	        return Err(notify_and_wrap(transport, fault).await);
   745	    }
   746	    Ok(())
   747	}
   748
   749	/// The responder half of establish AFTER the `SessionOpen` is read:
   750	/// complement check, `validate_open`, endpoint resolution, data-plane
   751	/// prepare, and `SessionAccept`. Factored out so both `establish` (which
   752	/// reads the open then calls this) and `run_responder` (which reads the
   753	/// open, dispatches on the declared role, then calls this with the
   754	/// resolved local role) share one implementation. Sends the refusal
   755	/// `SessionError` itself; returned faults are `peer_notified`.
   756	async fn responder_finish(
   757	    transport: &mut FrameTransport,
   758	    open: SessionOpen,
   759	    local_role: TransferRole,
   760	    validate_open: &OpenValidator,
   761	    resolve_open: Option<&OpenResolver>,
   762	    policy: &ResponderPolicy,
   763	) -> Result<Negotiated> {
   764	    // The initiator declares ITS role; this responder end must
   765	    // hold the complement.
   766	    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
   767	    if declared != complement(local_role) {
   768	        return Err(notify_and_wrap(
   769	            transport,
   770	            SessionFault::protocol_violation(format!(
   771	                "initiator declared role {} but this responder is {}",
   772	                declared.as_str_name(),
   773	                local_role.as_str_name()
   774	            )),
   775	        )
   776	        .await);
   777	    }
   778	    // otp-10b-1: an operator who disabled server-side checksum hashing
   779	    // refuses a content-compare session outright — the session never
   780	    // silently degrades a `--checksum` request to a weaker compare.
   781	    if policy.refuse_checksum_compare && open.compare_mode == ComparisonMode::Checksum as i32 {
   782	        return Err(notify_and_wrap(
   783	            transport,
   784	            SessionFault::new(
   785	                session_error::Code::ChecksumDisabled,
   786	                "checksum comparison is disabled on this daemon \
   787	                 (--no-server-checksums / server_checksums_enabled = false)",
   788	            ),
   789	        )
   790	        .await);
   791	    }
   792	    if let Err(fault) = validate_open(&open) {
   793	        // Refusal is a SessionError instead of SessionAccept,
   794	        // never a silent close (contract §Phase state machine).
   795	        return Err(notify_and_wrap(transport, fault).await);
   796	    }
   797	    // Responder endpoint resolution (otp-4): map the wire
   798	    // module/path to a local root and enforce read-only, both
   799	    // BEFORE SessionAccept so a refusal replaces the accept
   800	    // (never follows it). The resolver is caller-supplied
   801	    // (daemon module lookup); a fixed-root responder passes
   802	    // None and resolves nothing here.
   803	    let resolved_root = match resolve_open {
   804	        Some(resolve) => match resolve(&open).await {
   805	            Ok(resolved) => {
   806	                // A read-only module is fatal only for a
   807	                // DESTINATION (it would write); a SOURCE
   808	                // responder (otp-5, daemon-send) reads happily.
   809	                if local_role == TransferRole::Destination && resolved.read_only {
   810	                    return Err(notify_and_wrap(
   811	                        transport,
   812	                        SessionFault::read_only("destination module is read-only".to_string()),
   813	                    )
   814	                    .await);
   815	                }
   816	                Some(resolved.root)
   817	            }
   818	            Err(fault) => return Err(notify_and_wrap(transport, fault).await),
   819	        },
   820	        None => None,
   821	    };
   822	    // Data plane (otp-4b/5b): a responder binds a TCP listener and grants
   823	    // it, unless the initiator requested the in-stream carrier or the bind
   824	    // fails (grant-less accept ⇒ in-stream fallback). This is role-agnostic
   825	    // (otp-5b): the RESPONDER binds+accepts and the INITIATOR dials, while
   826	    // byte direction is set by role — a DESTINATION responder accepts+
   827	    // receives (push, otp-4b), a SOURCE responder accepts+sends (pull,
   828	    // otp-5b). The bound listener travels in `Negotiated.responder_data_plane`
   829	    // and is consumed by whichever role's driver runs.
   830	    //
   831	    //
   832	    // otp-7b: resume sessions ride the data plane too — block records
   833	    // travel as binary BLOCK/BLOCK_COMPLETE records on the sockets (the
   834	    // otp-7a in-stream frames remain the fallback carrier), so the grant
   835	    // is no longer suppressed for a resume session.
   836	    let responder_data_plane = if open.in_stream_bytes || policy.force_in_stream {
   837	        None
   838	    } else {
   839	        data_plane::prepare_responder_data_plane().await
   840	    };
   841	    let accept = SessionAccept {
   842	        // The byte RECEIVER advertises capacity at session
   843	        // open (D-2026-06-20-1/-2); consumed by the dial when
   844	        // the data plane lands (otp-4b).
   845	        receiver_capacity: if local_role == TransferRole::Destination {
   846	            Some(crate::dial::local_receiver_capacity())
   847	        } else {
   848	            None
   849	        },
   850	        // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
   851	        data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
   852	    };
   853	    transport.send(frame(Frame::Accept(accept.clone()))).await?;
   854	    Ok(Negotiated {
   855	        open,
   856	        accept,
   857	        resolved_root,
   858	        responder_data_plane,
   859	    })
   860	}
   861
   862	/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
   863	/// scoping requirement). Sends the refusal `SessionError` itself when
   864	/// it detects the fault locally; returned faults are `peer_notified`.
   865	async fn establish(
   866	    transport: &mut FrameTransport,
   867	    hello: &HelloConfig,
   868	    endpoint: &SessionEndpoint,
   869	    local_role: TransferRole,
   870	    validate_open: &OpenValidator,
   871	    // Consulted only on the Responder branch, after the received open
   872	    // passes `validate_open` and before SessionAccept. `None` = the
   873	    // caller supplies the root itself (Initiator, or fixed-root test).
   874	    resolve_open: Option<&OpenResolver>,
   875	) -> Result<Negotiated> {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GKfmSk1T' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZgIPMJzF' (errno=Operation not permitted)
  2420	// ---------------------------------------------------------------------------
  2421
  2422	/// What the destination end can report after a completed session.
  2423	#[derive(Debug, Clone)]
  2424	pub struct DestinationOutcome {
  2425	    /// The summary this end computed and sent (contract: DESTINATION
  2426	    /// is the scorer).
  2427	    pub summary: TransferSummary,
  2428	    /// Paths this end put on the need list, in emission order. The
  2429	    /// role suite pins these identical across role assignments — the
  2430	    /// executable form of the owner's invariance requirement.
  2431	    pub needed_paths: Vec<String>,
  2432	    /// The settled data-plane stream count this end observed (epoch-0 +
  2433	    /// accepted resizes), or `None` for the in-stream carrier. The sf-2
  2434	    /// pin (otp-4b-2) reads it to assert shape correction grew the
  2435	    /// stream set past the zero-knowledge single-stream grant.
  2436	    pub data_plane_streams: Option<usize>,
  2437	}
  2438
  2439	/// Run the DESTINATION role of one transfer session over `transport`,
  2440	/// writing under the root named by `target`. Diffs the streamed
  2441	/// manifest against its own filesystem (the destination is the one
  2442	/// diff owner — plan §Design 3), returns the summary it computed and
  2443	/// sent.
  2444	///
  2445	/// `target` is [`DestinationTarget::Fixed`] when the root is known up
  2446	/// front (an Initiator's own local root, or a test), or
  2447	/// [`DestinationTarget::Resolve`] when the root must be resolved from
  2448	/// the received `SessionOpen` mid-handshake (the daemon Responder,
  2449	/// where the wire module name selects the root).
  2450	pub async fn run_destination(
  2451	    cfg: DestinationSessionConfig,
  2452	    transport: FrameTransport,
  2453	    target: DestinationTarget,
  2454	) -> Result<DestinationOutcome> {
  2455	    let mut transport = transport;
  2456	    let endpoint = match cfg.endpoint {
  2457	        SessionEndpoint::Initiator { mut open } => {
  2458	            let declared = TransferRole::try_from(open.initiator_role);
  2459	            if declared != Ok(TransferRole::Destination) {
  2460	                eyre::bail!(
  2461	                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
  2462	                );
  2463	            }
  2464	            if let Err(fault) = destination_open_validator(&open) {
  2465	                eyre::bail!("run_destination initiator config unsupported: {fault}");
  2466	            }
  2467	            // Dial contract: the byte receiver advertises capacity in
  2468	            // its open when it is the initiator (contract §Invariants 5).
  2469	            if open.receiver_capacity.is_none() {
  2470	                open.receiver_capacity = Some(crate::dial::local_receiver_capacity());
  2471	            }
  2472	            SessionEndpoint::Initiator { open }
  2473	        }
  2474	        SessionEndpoint::Responder => SessionEndpoint::Responder,
  2475	    };
  2476
  2477	    let resolve_open: Option<&OpenResolver> = match &target {
  2478	        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  2479	        DestinationTarget::Fixed(_) => None,
  2480	    };
  2481
  2482	    let negotiated = establish(
  2483	        &mut transport,
  2484	        &cfg.hello,
  2485	        &endpoint,
  2486	        TransferRole::Destination,
  2487	        &destination_open_validator,
  2488	        resolve_open,
  2489	    )
  2490	    .await?;
  2491
  2492	    // The resolver's root (Responder + Resolve) wins; otherwise the
  2493	    // caller-supplied Fixed root.
  2494	    let dst_root = match negotiated.resolved_root.clone() {
  2495	        Some(root) => root,
  2496	        None => match &target {
  2497	            DestinationTarget::Fixed(root) => root.clone(),
  2498	            // Unreachable: a Resolve target always yields a root on the
  2499	            // Responder branch, and establish only skips resolution on
  2500	            // the Initiator branch (which pairs with a Fixed root).
  2501	            DestinationTarget::Resolve(_) => {
  2502	                return Err(eyre::Report::new(SessionFault::internal(
  2503	                    "resolver target produced no destination root",
  2504	                )));
  2505	            }
  2506	        },
  2507	    };
  2508
  2509	    drive_destination(
  2510	        &mut transport,
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1dwDrIBD' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-zwofL8jO' (errno=Operation not permitted)
     1	//! The single live transfer dial (`ue-r2-1e`, REV4 Design §4).
     2	//!
     3	//! Sender-owned, receiver-bounded: the byte sender constructs one
     4	//! `TransferDial` per transfer, clamped by the receiver's advertised
     5	//! [`CapacityProfile`] (the `ue-r2-1b` wire fields), starts at the
     6	//! conservative floor (D-2026-06-20-1/-2: no probe phase, no
     7	//! size-gated start — begin immediately and tune live), and a tuner
     8	//! steps the cheap dials from the PR1 stream telemetry.
     9	//!
    10	//! Mutability model (the C-ready seam `ue-r2-2` builds on):
    11	//! - **Cheap dials** — `chunk_bytes`, `prefetch_count`: atomics the
    12	//!   tuner steps mid-transfer. Consumers read them when a session,
    13	//!   pipeline, or fallback batch is set up, so a step takes effect for
    14	//!   sockets/batches started afterwards (epoch-N resize adds, the next
    15	//!   gRPC-fallback batch) — existing sessions keep their snapshot.
    16	//! - **Connect-time dials** — `tcp_buffer_bytes`, buffer-pool sizing:
    17	//!   read when a socket/pool is built; changes affect sockets opened
    18	//!   afterwards (no setsockopt on live sockets this slice).
    19	//! - **Negotiated once** — `initial_streams`/`max_streams`: stream
    20	//!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
    21	//!   dial only carries the negotiation-time value and the
    22	//!   profile-clamped ceiling.
    23	//!
    24	//! This replaces the size-keyed `determine_remote_tuning` static
    25	//! ladder: the ladder's floor tier is the dial's start, its top tier
    26	//! is the dial's default ceiling, and everything between is reached by
    27	//! ramping on evidence instead of guessing from `total_bytes`.
    28
    29	use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
    30	use std::sync::{Arc, Mutex};
    31
    32	use crate::generated::CapacityProfile;
    33
    34	const MIB: usize = 1024 * 1024;
    35
    36	/// Floor (conservative start) values — the old ladder's smallest tier.
    37	pub const DIAL_FLOOR_CHUNK_BYTES: usize = 16 * MIB;
    38	pub const DIAL_FLOOR_PREFETCH: usize = 4;
    39	pub const DIAL_FLOOR_INITIAL_STREAMS: usize = 4;
    40	pub const DIAL_FLOOR_MAX_STREAMS: usize = 8;
    41
    42	/// Default ceilings — the old ladder's top tier (a fully ramped dial
    43	/// matches today's best static behavior).
    44	pub const DIAL_CEILING_CHUNK_BYTES: usize = 64 * MIB;
    45	pub const DIAL_CEILING_PREFETCH: usize = 32;
    46	pub const DIAL_CEILING_MAX_STREAMS: usize = 32;
    47	pub const DIAL_CEILING_TCP_BUFFER_BYTES: usize = 8 * MIB;
    48
    49	/// Tuner policy (initial, deliberately simple): sampled every
    50	/// [`DIAL_TUNER_TICK`]; below [`DIAL_STEP_UP_BLOCKED_RATIO`] blocked
    51	/// time the pipe is not back-pressured → step up; above
    52	/// [`DIAL_STEP_DOWN_BLOCKED_RATIO`] → step down. One step per tick
    53	/// (hysteresis by construction).
    54	pub const DIAL_TUNER_TICK: std::time::Duration = std::time::Duration::from_millis(500);
    55	pub const DIAL_STEP_UP_BLOCKED_RATIO: f64 = 0.05;
    56	pub const DIAL_STEP_DOWN_BLOCKED_RATIO: f64 = 0.30;
    57
    58	/// Resize policy (`ue-r2-2`): streams are the EXPENSIVE dial — a step
    59	/// costs a control round-trip plus a TCP connect — so they move only
    60	/// after the cheap dials are pinned at a bound and the signal has held
    61	/// for [`RESIZE_SUSTAIN_TICKS`] consecutive ticks, and never within
    62	/// [`RESIZE_COOLDOWN_TICKS`] of the previous settle. One stream per
    63	/// epoch (the wire carries one `sub_token` per ADD).
    64	pub const RESIZE_COOLDOWN_TICKS: u32 = 4;
    65	pub const RESIZE_SUSTAIN_TICKS: i32 = 2;
    66
    67	/// The capacity profile this host advertises when it is the byte
    68	/// RECEIVER (ue-r2-1e: the first real sender of the ue-r2-1b wire
    69	/// fields). Honest system facts only — fields we cannot measure yet
    70	/// stay 0 (= unknown per the wire contract), never fabricated:
    71	/// ceilings mirror what today's receive paths actually accept.
    72	pub fn local_receiver_capacity() -> CapacityProfile {
    73	    CapacityProfile {
    74	        cpu_cores: num_cpus::get() as u32,
    75	        drain_class: 0,
    76	        load_percent: 0,
    77	        max_streams: DIAL_CEILING_MAX_STREAMS as u32,
    78	        drain_rate_bytes_per_sec: 0,
    79	        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
    80	        max_inflight_bytes: (DIAL_CEILING_CHUNK_BYTES * DIAL_CEILING_PREFETCH) as u64,
    81	    }
    82	}
    83
    84	/// Resolve the receiver's advertised stream ceiling with the wire
    85	/// contract's `0 = unknown` semantics. Both the SOURCE-owned dial and the
    86	/// DESTINATION's resize admission must call this one function; otherwise a
    87	/// destination-initiated session can interpret the same profile as a
    88	/// one-stream cap while its source interprets it as the default ceiling.
    89	pub fn receiver_stream_ceiling(profile: Option<&CapacityProfile>) -> usize {
    90	    profile
    91	        .and_then(|capacity| (capacity.max_streams > 0).then_some(capacity.max_streams as usize))
    92	        .unwrap_or(DIAL_CEILING_MAX_STREAMS)
    93	        .clamp(1, DIAL_CEILING_MAX_STREAMS)
    94	}
    95
    96	/// Serialized wire-epoch state. Resize proposals are rare (at most one per
    97	/// control-lane round trip), so one short critical section is preferable to
    98	/// a split-atomic check/CAS sequence that can reopen a refused transfer or
    99	/// reuse an epoch after an intervening settlement.
   100	#[derive(Debug, Default)]
   101	struct ResizeEpochState {
   102	    settled_epoch: u32,
   103	    pending_epoch: Option<u32>,
   104	    refused: bool,
   105	}
   106
   107	impl ResizeEpochState {
   108	    fn settle(&mut self, epoch: u32, accepted: bool) -> bool {
   109	        if self.pending_epoch != Some(epoch) || epoch == 0 {
   110	            return false;
   111	        }
   112	        if !accepted {
   113	            self.refused = true;
   114	        }
   115	        self.settled_epoch = epoch;
   116	        self.pending_epoch = None;
   117	        true
   118	    }
   119	}
   120
   121	struct ResizeEpochGuard<'a> {
   122	    inner: std::sync::MutexGuard<'a, ResizeEpochState>,
   123	    #[cfg(test)]
   124	    acquisition: usize,
   125	}
   126
   127	impl std::ops::Deref for ResizeEpochGuard<'_> {
   128	    type Target = ResizeEpochState;
   129
   130	    fn deref(&self) -> &Self::Target {
   131	        &self.inner
   132	    }
   133	}
   134
   135	impl std::ops::DerefMut for ResizeEpochGuard<'_> {
   136	    fn deref_mut(&mut self) -> &mut Self::Target {
   137	        &mut self.inner
   138	    }
   139	}
   140
   141	#[cfg(test)]
   142	impl ResizeEpochGuard<'_> {
   143	    fn acquisition(&self) -> usize {
   144	        self.acquisition
   145	    }
   146	}
   147
   148	#[cfg(test)]
   149	struct ResizeTickTestHook {
   150	    entered: std::sync::Barrier,
   151	    release: std::sync::Barrier,
   152	    entered_acquisition: AtomicUsize,
   153	    claimed_acquisition: AtomicUsize,
   154	}
   155
   156	#[cfg(test)]
   157	impl std::fmt::Debug for ResizeTickTestHook {
   158	    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
   159	        f.write_str("ResizeTickTestHook")
   160	    }
   161	}
   162
   163	#[cfg(test)]
   164	impl ResizeTickTestHook {
   165	    fn new() -> Self {
   166	        Self {
   167	            entered: std::sync::Barrier::new(2),
   168	            release: std::sync::Barrier::new(2),
   169	            entered_acquisition: AtomicUsize::new(0),
   170	            claimed_acquisition: AtomicUsize::new(0),
   171	        }
   172	    }
   173	}
   174
   175	#[cfg(test)]
   176	struct ResizeSettleTestHook {
   177	    observed: std::sync::Barrier,
   178	    contended: std::sync::atomic::AtomicBool,
   179	}
   180
   181	#[cfg(test)]
   182	impl std::fmt::Debug for ResizeSettleTestHook {
   183	    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
   184	        f.write_str("ResizeSettleTestHook")
   185	    }
   186	}
   187
   188	#[cfg(test)]
   189	impl ResizeSettleTestHook {
   190	    fn new() -> Self {
   191	        Self {
   192	            observed: std::sync::Barrier::new(2),
   193	            contended: std::sync::atomic::AtomicBool::new(false),
   194	        }
   195	    }
   196	}
   197
   198	/// The one mutable tuning object for a transfer.
   199	#[derive(Debug)]
   200	pub struct TransferDial {
   201	    chunk_bytes: AtomicUsize,
   202	    prefetch_count: AtomicUsize,
   203	    /// 0 = unset (kernel default), matching the old `Option<usize>`.
   204	    tcp_buffer_bytes: AtomicUsize,
   205	    initial_streams: AtomicUsize,
   206	    max_streams: AtomicUsize,
   207	    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
   208	    // resize ids; 0 is reserved for the initial stream set) ──────────
   209	    /// Settled live stream count. Epoch-0 write is
   210	    /// `set_negotiated_streams`; later writes come from
   211	    /// `resize_settled` on an accepted epoch.
   212	    live_streams: AtomicUsize,
   213	    /// Last settled epoch, in-flight proposal, and terminal-refusal bit.
   214	    /// These fields form one arbitration state: observing/claiming them
   215	    /// separately permits an ABA race across a concurrent settlement.
   216	    resize_epochs: Mutex<ResizeEpochState>,
   217	    #[cfg(test)]
   218	    resize_tick_test_hook: Mutex<Option<Arc<ResizeTickTestHook>>>,
   219	    #[cfg(test)]
   220	    resize_settle_test_hook: Mutex<Option<Arc<ResizeSettleTestHook>>>,
   221	    #[cfg(test)]
   222	    resize_lock_sequence: AtomicUsize,
   223	    /// Resize-eligible ticks since the last settle (cooldown clock).
   224	    ticks_since_settle: AtomicU32,
   225	    /// Consecutive same-direction tick counter: positive = "pipe clean
   226	    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
   227	    /// dials floored" streak. Any other tick resets it.
   228	    resize_sustain: AtomicI32,
   229	    // Profile-clamped bounds, fixed at construction.
   230	    ceiling_chunk_bytes: usize,
   231	    ceiling_prefetch: usize,
   232	    ceiling_max_streams: usize,
   233	    ceiling_tcp_buffer_bytes: usize,
   234	}
   235
   236	/// One engine resize decision (`ue-r2-2`). The adapter that owns the
   237	/// control stream turns this into a wire `DataPlaneResize` (the engine
   238	/// stays wire-type-free here on purpose) and MUST eventually call
   239	/// [`TransferDial::resize_settled`] for the epoch — with what actually
   240	/// happened — or no further proposals are produced.
   241	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
   242	pub struct ResizeProposal {
   243	    /// The wire epoch for this change (`resize_epoch() + 1`).
   244	    pub epoch: u32,
   245	    /// Absolute desired live count (idempotent, per the proto).
   246	    pub target_streams: usize,
   247	    /// Convenience: `target_streams > live` at proposal time.
   248	    pub add: bool,
   249	}
   250
   251	impl TransferDial {
   252	    fn lock_resize_epochs(&self) -> ResizeEpochGuard<'_> {
   253	        ResizeEpochGuard {
   254	            inner: self
   255	                .resize_epochs
   256	                .lock()
   257	                .expect("resize epoch state poisoned"),
   258	            #[cfg(test)]
   259	            acquisition: self
   260	                .resize_lock_sequence
   261	                .fetch_add(1, Ordering::SeqCst)
   262	                .saturating_add(1),
   263	        }
   264	    }
   265
   266	    /// Conservative start with default ceilings (no receiver profile).
   267	    pub fn conservative() -> Self {
   268	        Self::conservative_within(None)
   269	    }
   270
   271	    /// Conservative start bounded by the receiver's advertised
   272	    /// capacity profile. Per the `ue-r2-1b` contract, `0`/absent
   273	    /// fields mean UNKNOWN and keep the (already conservative)
   274	    /// default ceiling — never "unlimited". A profile can only lower
   275	    /// ceilings, never raise them above the defaults this slice.
   276	    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
   277	        let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
   278	        let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
   279	        let ceiling_streams = receiver_stream_ceiling(profile);
   280	        let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2v6Snk7i' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-eagkaSyV' (errno=Operation not permitted)
2859:  2859	    );
2860:  2860	    assert!(dst_root.join("keep.txt").exists());
2861:  2861	    assert!(!dst_root.join("drop.log").exists());
2862:  2862	}
2863:  2863
2864:  2864	// ---------------------------------------------------------------------------
2865:  2865	// Protocol-violation fail-fast (scripted peer)
2866:  2866	// ---------------------------------------------------------------------------
2867:  2867
2868:  2868	fn wire(frame: Frame) -> TransferFrame {
2869:  2869	    TransferFrame { frame: Some(frame) }
2870:  2870	}
2871:  2871
2872:  2872	async fn recv_or_panic(t: &mut FrameTransport) -> Frame {
2873:  2873	    t.recv()
2874:  2874	        .await
2875:  2875	        .unwrap()
2876:  2876	        .expect("peer closed unexpectedly")
2877:  2877	        .frame
2878:  2878	        .expect("empty frame")
2879:  2879	}
2880:  2880
2881:  2881	fn hello_frame() -> TransferFrame {
2882:  2882	    let hello = HelloConfig::default();
2883:  2883	    wire(Frame::Hello(SessionHello {
2884:  2884	        build_id: hello.build_id,
2885:  2885	        contract_version: hello.contract_version,
2886:  2886	    }))
2887:  2887	}
2888:  2888
2889:  2889	#[tokio::test]
2890:  2890	async fn payload_record_before_manifest_complete_is_protocol_violation() {
2891:  2891	    let tmp = tempfile::tempdir().unwrap();
2892:  2892	    let dst_root = tmp.path().join("dst");
2893:  2893	    std::fs::create_dir_all(&dst_root).unwrap();
2894:  2894
2895:  2895	    let dest_cfg = DestinationSessionConfig {
2896:  2896	        hello: HelloConfig::default(),
2897:  2897	        endpoint: SessionEndpoint::Responder,
2898:  2898	        data_plane_host: None,
2899:  2899	        instruments: Default::default(),
2908:  2908
2918:  2918	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
2928:  2928	        .await
2938:  2938	            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
2948:  2948	        session_error::Code::ProtocolViolation
2958:  2958	    let tmp = tempfile::tempdir().unwrap();
2966:  2966	        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
2968:  2968	        data_plane_host: None,
2978:  2978	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
2980:  2980	        .await
2981:  2981	        .unwrap();
2982:  2982	    loop {
2983:  2983	        match recv_or_panic(&mut peer).await {
2984:  2984	            Frame::ManifestEntry(_) => continue,
2985:  2985	            Frame::ManifestComplete(_) => break,
2986:  2986	            other => panic!("expected manifest stream, got {other:?}"),
2987:  2987	        }
2988:  2988	    }
2989:  2989	    peer.send(wire(Frame::NeedBatch(NeedBatch {
2998:  2998	    let source_err = source_task.await.unwrap().unwrap_err();
3008:  3008	    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
3018:  3018	    let source_cfg = SourceSessionConfig {
3021:  3021	        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
3028:  3028
3038:  3038	            Frame::ManifestComplete(_) => break,
3039:  3039	            other => panic!("expected manifest stream, got {other:?}"),
3048:  3048	    .await
3058:  3058	#[tokio::test]
3068:  3068	    let tmp = tempfile::tempdir().unwrap();
3078:  3078	    let source_cfg = SourceSessionConfig {
3080:  3080	        hello: HelloConfig::default(),
3081:  3081	        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
3082:  3082	        plan_options: PlanOptions::default(),
3083:  3083	        data_plane_host: None,
3084:  3084	    };
3085:  3085	    let (source_transport, mut peer) = in_process_pair();
3086:  3086	    let source = Arc::new(FsTransferSource::new(src_root));
3087:  3087	    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
3088:  3088
3089:  3089	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
3098:  3098	        .await
3108:  3108	            Frame::ManifestComplete(_) => {
3118:  3118	    let fault = fault_of(&source_err);
3128:  3128	async fn manifest_entry_after_manifest_complete_is_protocol_violation() {
3138:  3138	        local_apply: None,
3148:  3148	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
3158:  3158	    .unwrap();
3168:  3168
3178:  3178	/// pull's `--checksum` behavior, now role-agnostic), under both
3179:  3179	/// initiator layouts. Control: the same fixture under SizeMtime
3180:  3180	/// transfers (source mtime is newer), proving the skip is the
3181:  3181	/// checksum's doing and the pin is not vacuous.
3182:  3182	#[tokio::test]
3183:  3183	async fn checksum_compare_skips_content_equal_files_regardless_of_mtime() {
3184:  3184	    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
3185:  3185	        for (mode, expected) in [
3186:  3186	            (ComparisonMode::SizeMtime, 1u64), // control: mtime differs
3187:  3187	            (ComparisonMode::Checksum, 0u64),  // content-equal skips
3188:  3188	        ] {
3189:  3189	            let tmp = tempfile::tempdir().unwrap();
3194:  3194	            write_tree(&src_root, &[("same.bin", vec![7u8; 4096], 2_000)]);
3195:  3195	            write_tree(&dst_root, &[("same.bin", vec![7u8; 4096], 1_000)]);
3198:  3198	                compare_mode: mode as i32,
3199:  3199	                ..basic_open(initiator_role)
3204:  3204	                panic!("source failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
3207:  3207	                panic!("destination failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
3208:  3208	            });
3211:  3211	                "{mode:?} under initiator {initiator_role:?}"
3218:  3218	/// mtime, DIFFERENT content — must transfer under Checksum, both
3219:  3219	/// initiator layouts. Control first: SizeMtime skips it (that is the
3223:  3223	    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
3228:  3228	            let tmp = tempfile::tempdir().unwrap();
3233:  3233	            write_tree(&src_root, &[("stealth.bin", vec![1u8; 4096], 1_000)]);
3234:  3234	            write_tree(&dst_root, &[("stealth.bin", vec![2u8; 4096], 1_000)]);
3238:  3238	                ..basic_open(initiator_role)
3243:  3243	                panic!("source failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
3246:  3246	                panic!("destination failed ({mode:?}, initiator {initiator_role:?}): {e:#}")
3248:  3248	            assert_eq!(
3250:  3250	                "{mode:?} under initiator {initiator_role:?}"

exec
/bin/zsh -lc "git diff --unified=80 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946 -- .agents/machines.md REVIEW.md DEVLOG.md .review/findings/otp12-pf1-rigw-harness.md | sed -n '1,1200p'; git diff --quiet 75a9a33ce600e4707438ed885de2ce0cdf27d946 -- crates/blit-core/src/transfer_session/data_plane.rs crates/blit-core/tests/transfer_session_roles.rs scripts/bench_otp12pf_rigw.sh scripts/otp12pf_rigw_analyze.py scripts/otp12pf_rigw_analyze_test.py; printf 'tracked_target_diff_rc=%s\\n' \""'$?"; bash -n scripts/bench_otp12pf_rigw.sh; python3 -B -c '"'import ast, pathlib; [ast.parse(pathlib.Path(p).read_text()) for p in (\"scripts/otp12pf_rigw_analyze.py\", \"scripts/otp12pf_rigw_analyze_test.py\")]'; printf 'syntax_rc=%s\\n' \""'$?"' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-j9UuWKze' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-fkguCP4Z' (errno=Operation not permitted)
diff --git a/.agents/machines.md b/.agents/machines.md
index 5f8f853..3188f82 100644
--- a/.agents/machines.md
+++ b/.agents/machines.md
@@ -51,160 +51,167 @@ every Linux target in play).
   Every prior measurement therefore negotiated down to a 1460-byte MSS:
   **jumbo has never been exercised in a blit benchmark.** Those numbers
   are valid — they are simply *1500-MTU* numbers — and rig W at jumbo is
   a genuinely untested condition. magneto is still 1500 (raise
   `enp1s0f1` to 9000 to make the Linux rig jumbo too).
 - Mismatched MTUs on one L2 segment are fine: TCP MSS negotiation
   handles it, each host advertising what it can receive. What is NOT
   fine is a host advertising a size it cannot actually send.
 - **Fleet MTU as of 2026-07-13 — the whole 10 GbE fabric is now 9000:**

   | host | iface | MTU | persistent? |
   |---|---|---|---|
   | Mac | `en9` (Aquantia) | 9000 | yes (macOS net service) |
   | netwatch-01 | Ethernet | 9000 | yes (raised 1500→9000 today) |
   | skippy | `enp66s0f1` | 9000 | yes |
   | **zoey** | `enp0s0` (RJ45, NFS data .206) | **9000** | yes — `[Link] MTUBytes=9000` in `/etc/systemd/network/enp0s0.network` |
   | **zoey** | `enp0s1` (SFP, mgmt .210) | **9000** | yes — same, in `enp0s1.network` |
   | altiera | `enp1s0`/`enp2s0` | 9000 | yes (NetworkManager profiles) |
   | magneto | `enp1s0f1` | 9000 | yes — NM profile `Wired connection 3` saved `mtu=9000` (2026-07-13) |

   **Verified end-to-end 2026-07-13**: a jumbo DF ping from skippy reaches
   magneto, zoey, altiera, netwatch-01 AND the Mac — all OK. Every 10 GbE
   pair in the fleet carries 9000-byte frames. (Always test from a LINUX
   host; the Mac's `ping` cannot send >8192 — see the raw-socket trap.)

 - **zoey (UniFi UNAS Pro) jumbo — how it was done, and the trap.**
   Debian 11 + `systemd-networkd`; NIC `maxmtu` is 9216 so the hardware is
   fine. Persistence = a `[Link]` / `MTUBytes=9000` stanza in each
   `/etc/systemd/network/enp0s*.network` (originals backed up as
   `*.premtu`). Proven by `networkctl reload && networkctl reconfigure`
   with the static IP intact — no reboot needed. **TRAP: `/` is an
   overlayfs** (`lowerdir=/mnt/.rofs` read-only + writable upper), so a
   UniFi *firmware update* can replace the base image and silently drop
   this. Re-check after any UNAS update:
   `ssh root@zoey 'cat /sys/class/net/enp0s0/mtu'` → want 9000.
   Method for any risky remote NIC change: arm a self-healing revert
   first — `nohup setsid bash -c 'sleep 90; [ -f /tmp/ok ] || ip link set
   IFACE mtu 1500' &` — then confirm with `touch /tmp/ok`. Change the NIC
   you are NOT ssh'd through when a second one exists.
 - **Live NFS/TCP connections do NOT pick up a new MTU.** MSS is fixed at
   connect time, so an existing mount keeps its old segment size until it
   reconnects (reboot/remount). Not worth forcing for low-bandwidth
   mounts.
 - Two-NICs-on-one-subnet (both `altiera` and `zoey`, and it is the
   default `arp_ignore=0 arp_announce=0`) invites ARP flux + asymmetric
   routing. Working today; a latent source of intermittent stalls.
 - Local VM on the Mac — Ubuntu ARM (aarch64), per owner. Build-only
   fallback likewise.

 ## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)

 `ssh michael@q` — Apple **M4 Mac mini**, 16 GB, macOS 26.5.2, arm64. It is now
 the rig-W Mac end: **quiet, dedicated, and faster than nagatha** (1 GiB in
 ~908 ms ≈ 1.18 GB/s, vs nagatha's ~1.3–1.8 s). Using it **decouples the codex
 review loop from rig-W benchmarking** — the contention that destroyed a
 53-minute experiment (below).

 - **10GbE**: `en8` = **10.1.10.54**, MTU **9000**, media 10Gbase-T. This is the
   **Aquantia adapter physically moved off nagatha**, so nagatha's 10GbE is now a
   *different* NIC at **10.1.10.92** (also MTU 9000). Any doc naming
   "Aquantia @ .54 on nagatha" is stale.
 - **⚠ THE MULTI-NIC ROUTING TRAP (cost ~1h).** `q` has THREE IPs on
   10.1.10.0/24 — `en0` (1GbE, .221), `en1` (Wi-Fi, .108), `en8` (10GbE, .54) —
   and macOS routes the subnet via the highest-ranked **network service**, not by
   which IP "matches". `en0` outranked `en8`, so **every benchmark would have run
   over gigabit**. Fixed by promoting the service that owns `en8` — confusingly
   named **"Thunderbolt Ethernet Slot 3"** — to rank 1
   (`sudo networksetup -ordernetworkservices …`). It has the same router
   (10.1.10.1), so `q` keeps its internet.
 - **DO NOT "fix" this with a host route.**
   `sudo route -n add -host 10.1.10.177 -interface en8` on a *directly-connected*
   subnet installs a next hop of **the interface's own MAC** — a black hole. It
   drops 100% of packets while `route -n get` still cheerfully reports
   `interface: en8`. Verify with `arp -n <peer>`: the MAC must be the PEER's, not
   `q`'s (`00:01:d2:19:04:a3`).
 - **An ssh transfer CANNOT verify this link.** ssh caps at ~79 MB/s on this path
   (nagatha's known-good 10GbE scores the same 79), which is *below* the gigabit
   ceiling — so a degraded link and a healthy one look identical through it. Use
   `ifconfig en8 | grep media` (the PHY's negotiated rate) and blit's own
   `wm_tcp_large` time (~908 ms for 1 GiB = 10GbE; ~10 s = 1GbE).
+- **MSS is directional on this pair (rechecked 2026-07-15):** five live
+  `getsockopt(TCP_MAXSEG)` samples were **8948 q→netwatch-01** and five were
+  **8960 netwatch-01→q**, with local sources `.54` and `.177` respectively.
+  A rig gate must pin the observed value for each direction rather than repeat
+  the older shorthand “8948 both directions.” On q, the saved host key for the
+  bare `netwatch-01` name is stale; the pinned numeric `10.1.10.177` entry is
+  valid and is the benchmark control endpoint. Do not bypass host-key checking.
 - **Staged**: repo clone at `~/Dev/blit_v2_f35702a` (detached `f35702a`, cloned
   from the LOCAL gitea — `q` *is* the gitea host); `target/release/{blit,blit-daemon}`
   arm64 copied from nagatha (embed-verified `+f35702a`); old client at
   `~/blit-bench-work/bins/blit-0f922de`; fixtures in `~/blit-bench-work`.
   NOPASSWD `/usr/sbin/purge` granted (`/etc/sudoers.d/blit-bench`, mode 0440 —
   `visudo -c` rejects any other mode). ssh key authorized on netwatch-01 in
   **`C:\ProgramData\ssh\administrators_authorized_keys`** (michael is an admin
   there, so the per-user file is ignored). macOS firewall is OFF on `q`.
 - **`q` RUNS GITEA** (it is `origin`, `http://q:3000`). It idles cheaply, but
   **do not push to `origin` during a benchmark session**.

 ## THE MAC IS A BENCH END — keep it quiet (recorded 2026-07-13, learned the hard way)

 **A rig-W (Mac↔Windows) benchmark requires a QUIET Mac.** The Mac is not a
 neutral driver: it runs the client in `mac_init` arms and serves the daemon in
 `win_init` arms. Any heavy Mac process contaminates the measurement — and
 **asymmetrically**, because `mac_init` runs the client locally while `win_init`
 runs it on Windows. CPU starvation therefore **inflates Δ and MANUFACTURES P1**,
 the very finding under test. (Same shape as the 2026-07-13 durability
 retraction: a cost billed to one arm and not the other.)

 - **This actually happened** (2026-07-13, first A-B-B-A attempt): codex jobs ran
   on the Mac for the whole 53-minute window. The same-MTU replicates caught it —
   `wm_tcp_large` read 911 ms in S1 and 1847 ms in S4 **at the same MTU**, and the
   noise floor came out at 473 ms, larger than the 325 ms gap under test. The run
   was discarded. Without the replicates it would have looked clean and been
   reported.
 - **Offenders**: `codex` (the review loop!), `cargo`/`rustc`, Spotlight
   reindexing, any build. **The review loop and a rig-W session cannot run at the
   same time** — sequence them.
 - **The runner gates on this**: it refuses to start a session while
   codex/cargo/rustc is running, and records `load1` per session so contamination
   is visible in the evidence rather than hidden in the noise.
 - **NEVER blanket-kill to get quiet.** `pkill -f codex` killed the OWNER's own
   codex sessions (2026-07-13). Ask the owner to clear the machine, or kill only
   PIDs you can prove you launched.
 - The Linux rig (magneto↔skippy) does not involve the Mac and has no such
   constraint — but P1's cell only exists on the Mac↔Windows pairing, so it
   cannot substitute.

 ## Rig residue (recorded 2026-07-10)

 - **The Mac's 10GbE IP and NIC CHANGED 2026-07-13** — this is a live
   confound in the otp-12 numbers, not a bookkeeping detail:
   * **now: `en9` = 10.1.10.54**, a Thunderbolt **Aquantia** adapter,
     MTU 9000, 10Gbase-T. (SSH into the Mac = `michael@10.1.10.54`;
     Remote Login is ON and netwatch-01's key is in the Mac's
     `authorized_keys`, so Windows→Mac ssh/sftp works.)
   * otp-12b (`wm_tcp_mixed` **1.237**) ran on the Aquantia at
     **10.1.10.54**; otp-12c (**1.300**) ran on a Thunderbolt-5 dock's
     built-in 10GbE at **10.1.10.91**. **Different NICs.** So the
     "1.237 → 1.300, it got worse at the cutover sha" reading is
     CONFOUNDED by a hardware change and must not be cited as evidence
     of a code regression. Both runs still showed the same qualitative
     asymmetry; only the delta is suspect.
   * Harnesses take the Mac IP via `MAC_HOST=` — pass **10.1.10.54**
     (older invocations in the DEVLOG say 10.1.10.91).
 - Windows box = **`michael@netwatch-01`, IP 10.1.10.177 as of
   2026-07-12** (the earlier-recorded 10.1.10.173 is STALE — DHCP; ssh
   by hostname; if the bare name stops resolving, `netwatch-01.local` or
   the IP both work — the host key is filed under both). **MTU raised
   1500 → 9000 on 2026-07-13** (see Network/MTU above). SMB File Sharing
   is now ON on the Mac and Windows is authenticated to it
   (`net use \\10.1.10.91\blit-bench-work`), so robocopy can reach it.
   Rules: `blit-bench-daemon` (otp-2w, repo-path-scoped)
   + `blit-otp12-daemon` (active-path-scoped) + staged
   `purge-standby.ps1`; repo checkout DETACHED at `e21cf84` since the
   otp-12b session (owner's `bench-cargo-lock` stash untouched); old
   `0f922de` exes aside-copied at `D:\blit-test\bins\0f922de\`; run
   bins under `D:\blit-test\bins\<sha>\`.
 - **Rig pairing constraint (owner, 2026-07-13): zoey's CPU is too slow
   to be a match for skippy** — a zoey↔skippy pair is NOT a valid
   symmetric/performance-matched rig; a zoey endpoint becomes the
   bottleneck and MASKS data-plane effects rather than measuring them.
   Consequence, recorded so it is not re-proposed: the fleet has **no
   same-OS, real-network, performance-matched pair** (one Mac; zoey too
   slow for skippy; magneto is a busy BitTorrent box — build-only, never
   a bench end). Platform-vs-role confounds on a two-host rig therefore
   cannot be broken by rig juggling and need a code-level counterfactual
   (see `docs/plan/OTP12_PERF_FINDINGS.md`).
diff --git a/.review/findings/otp12-pf1-rigw-harness.md b/.review/findings/otp12-pf1-rigw-harness.md
new file mode 100644
index 0000000..0831567
--- /dev/null
+++ b/.review/findings/otp12-pf1-rigw-harness.md
@@ -0,0 +1,353 @@
+# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows
+
+**Slice**: OTP12 performance-finding pf-1, P1 rig harness only.
+**Status**: Reopened — G6 fixed and guard-proved; fresh complete review pending.
+
+## What
+
+The acceptance harness cannot be reused unchanged for the phase diagnostic.
+It retains old/new and push/pull-shaped orchestration, drains Windows even
+when q is the destination, keeps one daemon alive across instrumentation-state
+changes, discards successful client stderr, and can create a firewall rule.
+Those properties either destroy the SOURCE/DESTINATION comparison or make the
+new two-endpoint trace uncorrelatable.
+
+## Approach
+
+- Use semantic `source_init` and `destination_init` arms. SOURCE sends and
+  DESTINATION receives in both arms; the varied property is only which
+  endpoint initiates the one `Transfer` session.
+- Pin one canonical source tree per direction and fixture. Both roles read the
+  same q or Windows physical path and land into a precreated container of the
+  same depth and shape. One session-scoped canonical destination path per
+  endpoint is reset and reused by all 128 arms; role-bearing run IDs are kept
+  only in evidence names and never enter a measured path. Session scoping
+  preserves failed-run endpoint evidence without reintroducing a within-run
+  path axis. The harness requires the q and Windows canonical
+  relative-path/size manifests to match, pins the one exact `src_<shape>` root,
+  and retains an identical manifest and digest for every accepted arm.
+- Run a fixed OFF–ON–ON–OFF four-block schedule over
+  `wm_tcp_mixed`, `mw_tcp_mixed`, `wm_grpc_mixed`, and `wm_tcp_large`.
+  Pair rounds traverse cells forward/reverse/reverse/forward and run the two
+  roles adjacently, producing eight pairs per trace state and cell with a
+  four/four role-first balance (128 timed transfers).
+- Stop and restart both exact daemons for every block, including ON→ON. Each
+  block has a common run ID; every TCP client log supplies the 16-hex session
+  fingerprint that correlates its peer daemon records. Windows logs are
+  retrieved through base64 with SHA-256 verification.
+- Fail closed on the exact build, route/interface/IP/MAC/MTU/link speeds,
+  direction-specific negotiated MSS, firewall-rule identity, timer
+  calibration, load, Time Machine, Spotlight, Windows CPU/disk drain, stale
+  processes, PID ownership, port teardown, trace leakage, incomplete trace
+  inventory, or landed-tree mismatch. The harness never changes firewall,
+  MTU, routing, Time Machine, Spotlight, or unrelated processes.
+- Use destination-keyed durability: q file fsync for Windows→q and Windows
+  volume flush for q→Windows. Both client locations capture the same q
+  monotonic completion anchor: immediate subprocess return on q, or the
+  streamed Windows result line as q receives it before SSH teardown. They take
+  the same three after-clock samples and wait only to the absolute +250 ms
+  deadline before durability. The measured
+  settle must remain in `[250,1000)` ms and is retained in `runs.csv`.
+  Successful Windows client logs are retrieved only after durability and the
+  current landed count/byte verification. Both caches are purged before every arm and
+  Windows disk writes must drain. The common first 250 ms of post-client
+  observation remains excluded, but every excess settle millisecond is charged
+  to the arm's durable total before comparison.
+- Compute paired differences `d_i = destination_init_i − source_init_i`, the
+  registered split drifts, role-order drift, the full paired range that guards
+  the known bimodal fast arm, trace observer bias, and conservative
+  `N_resolution`. Reports retain every sorted arm/difference distribution and
+  use only per-endpoint monotonic clocks for phase intervals. Cross-host clock
+  samples quantify uncertainty and are never silently subtracted.
+
+## Files
+
+- `crates/blit-core/src/transfer_session/data_plane.rs` — SOURCE dial
+  trace attachment now follows the matching dial-end marker at epoch zero
+  and every resize epoch.
+- `crates/blit-core/tests/transfer_session_roles.rs` — both initiator layouts
+  pin action-end before attachment on both endpoint roles.
+- `scripts/bench_otp12pf_rigw.sh` — q-side registered runner and endpoint
+  gates.
+- `scripts/otp12pf_rigw_analyze.py` — exact schedule, trace, clock, phase, and
+  resolution validator/reporter.
+- `scripts/otp12pf_rigw_analyze_test.py` — complete synthetic session and
+  fail-closed mutations.
+- `.agents/machines.md` — current direction-specific MSS and q SSH endpoint
+  fact.
+
+## Tests
+
+- `SELFTEST=1 bash scripts/bench_otp12pf_rigw.sh` proves the exact block/arm
+  inventory and canonical path construction without contacting either rig
+  endpoint. Every path assertion has an explicit failure path because macOS
+  Bash 3.2 does not reliably apply `set -e` to bare `[[ ... ]]` commands.
+- `python3 scripts/otp12pf_rigw_analyze_test.py` builds complete synthetic
+  evidence (128 arms, 768 clock samples, split client/daemon phase logs) and
+  rejects missing clock rows, missing endpoint trace, trace-off leakage,
+  gRPC trace leakage, schedule drift, sequence gaps, and terminal/inventory
+  corruption. It pins the split/range/role-order/observer resolution math and
+  all exported reports.
+- The same self-test runs under q's actual macOS Bash and Python so Bash 3.2
+  and platform behavior are exercised, not inferred from nagatha.
+- Mutation proof: removing role-order drift and the full paired-range term from
+  `N_pair` makes the synthetic diagnostic fail (`N_resolution` falls from 70
+  ms to 40 ms); restoring them returns the analyzer suite to green.
+- Mutation proof: excluding successful client logs from trace discovery makes
+  the synthetic diagnostic fail on a missing SOURCE/DESTINATION endpoint;
+  restoring both client and daemon evidence roots returns all tests to green.
+- Mutation proof: reducing the clock-row formatter from 12 fields to 11 makes
+  the harness self-test fail before analysis; restoring the exact 12-column
+  schema returns the local and q/macOS self-tests to green.
+- The analyzer rejects a missing `settled_ms` column, non-integer values, and
+  values outside `[250,1000)`. Synthetic evidence supplies the lower valid
+  bound so every accepted arm proves the registered settle window.
+- The analyzer parses each timing component once, requires exact Decimal
+  `total_ms = transfer_ms + (settled_ms - 250) + flush_ms`, and uses that
+  durable total for every paired median, delta, distribution, observer-bias,
+  and resolution-floor value. Only the common first 250 ms remains excluded;
+  excess observation latency is charged. Corrupt totals are rejected;
+  role-specific flush mutations prove the summaries cannot fall back to the
+  pre-durability transfer time, and an equal client-to-durability regression
+  proves asymmetric settle/flush partitioning cannot manufacture a role delta.
+- All asserted causal phase pairs are endpoint-local and require both producer
+  order and nondecreasing monotonic elapsed time. Socket action completion must
+  precede trace attachment; attached payload sockets must progress through
+  first write/receive before their role's data-plane completion; resize and
+  planner prerequisite chains are also pinned. The resize DAG additionally
+  requires sent proposal before SOURCE socket acquisition, attachment before
+  SOURCE settlement, final settlement/ACK before role-local completion, and
+  the exact receive→arm→ready→accept or receive→dial→attach→prepared chain on
+  the DESTINATION. Mutations reverse every one of those edges while preserving
+  exact contiguous producer sequences and must fail. Swapping completion ahead
+  of a first write, swapping attachment ahead of action completion, or
+  reversing a causal elapsed interval also makes the analyzer suite fail.
+- Mutation proof: restoring SOURCE dial attachment ahead of `socket_dial_end`
+  makes the two-initiator Rust phase test fail at epoch zero and resize epoch
+  one; restoring end-before-attachment returns it to green. No cross-endpoint
+  or concurrent send/ACK ordering is asserted.
+- Fixture and landed manifests encode each UTF-8 POSIX relative path in base64
+  beside its decimal file size, sort under ordinal/C locale rules, and reject
+  nonregular or reparse entries. The analyzer recomputes all digests, requires
+  exact q/Windows canonical equality and exactly 128 landed manifest files,
+  and rejects swapped per-file sizes, renamed paths, wrong root layout, or a
+  forged recorded digest even when file count and total bytes are unchanged.
+- The harness atomically claims a never-existing evidence directory before it
+  installs the EXIT trap or writes a byte. Existing paths are rejected
+  unchanged, with explicit stale `SESSION-COMPLETE`/`SESSION-VOID` diagnostics;
+  offline guards also pin rejection of unrelated retained content.
+- Every arm resets its exact destination with explicit error propagation,
+  verifies deletion landed, and proves the replacement is an empty plain
+  directory before draining caches or starting the timer. The q self-test
+  mutation makes removal fail under the production `||` call shape and must
+  remain rejected; a Windows source-contract guard forbids suppressed removal
+  errors and requires absence, directory, reparse, and emptiness checks.
+- SOURCE- and DESTINATION-initiated arms resolve to the same canonical
+  endpoint-local destination path and remote module-relative path. The
+  self-test pins both direction/role pairs with explicit `|| die` guards and
+  rejects any `run_arm` source that lets the role-bearing evidence ID reach a
+  measured destination. Adding the initiator role to
+  `destination_relative_path` now turns the Bash 3.2 self-test red at the first
+  q destination-path assertion; restoring the role-invariant path returns it
+  to green.
+- The failure handler removes any completion marker, stops only remembered
+  identity-checked daemons, appends teardown errors without replacing the
+  primary void reason, and never initiates session-tree deletion. HUP, INT,
+  and TERM enter that same bounded failure path. Offline process tests exercise
+  all three signals and prove both owned teardown paths run while remaining
+  evidence paths are reported for inspection.
+- Successful finalization first proves no remembered daemon or open port,
+  requires analyzer-accepted local evidence, removes and verifies both exact
+  Windows trees and the exact q tree, rechecks the port, and only then atomically
+  renames `SESSION-COMPLETE.tmp`. Cross-host deletion is not transactional: a
+  partial finalization failure keeps the complete local evidence and reports
+  remote paths as “may remain,” never as certainly preserved. A zero exit is
+  rejected unless the registered marker is a regular one-line file containing
+  the exact build SHA with no VOID or temporary marker; preflight-only runs
+  cannot create it. Mutations for failed Windows removal, a surviving q tree,
+  a pre/post-cleanup open port, missing/wrong completion markers, stale
+  preflight markers, and cleanup before completion all fail the self-test.
+- Windows launcher and daemon PIDs are numeric and identity-checked before any
+  termination: exact executable/name, one anchored block-specific `cmd.exe`
+  command line, and daemon parent PID when both processes exist. Startup also
+  verifies the same CIM identities immediately. Offline source-contract
+  mutations fail if command-line, parent, or validate-before-stop guards move
+  or disappear. If startup fails after CIM creation but before either PID file
+  is readable, the generated launcher waits on a bounded block-local gate and
+  cannot execute the daemon until its PID is atomically placed and read back;
+  without that gate it exits on its own. Teardown recovers only the unique
+  exact block-specific launcher command and its parented daemon; after stopping
+  the launcher it also finds, validates, and stops a child that raced the first
+  query. The live daemon smoke remains required to prove CIM quoting.
+  Mutations accepting any `cmd.exe`, accepting an unparented daemon, skipping
+  the bounded gate wait, opening the gate before PID placement/readback, or
+  skipping the late child's exact executable validation each turn the
+  self-test red.
+- `LAUNCHER_SMOKE=1` is a mutually exclusive standalone live mode. After the
+  full provenance and endpoint preflight, it starts only the exact Windows CIM
+  launcher and daemon, proves q can reach the registered port, identity-stops
+  both processes, proves both endpoint ports closed, and completes strict
+  session-tree cleanup. It never registers a run, starts q's daemon, times a
+  transfer, invokes the analyzer, or writes `SESSION-COMPLETE`. An offline
+  call-order test and source guard pin the start/reach/stop/closed/cleanup
+  sequence and keep the smoke branch ahead of registered-run state. Mutations
+  removing its pre-start port gate, start, reachability probe, exact stop/log
+  collection, block clear, strict cleanup/failure path, or main-branch return,
+  and a mutation setting registered state, each turn the self-test red.
+- Mutation proof: replacing the absolute-deadline wait with a no-op makes the
+  harness self-test fail because it returns before +250 ms. Moving the
+  successful Windows client-log fetch ahead of the durability marker makes
+  the production-order self-test fail. Restoring both returns the harness and
+  analyzer self-tests to green.
+- A delayed fake Windows-result producer emits its exact sentinel and then
+  holds the pipe open; the q arrival stamp must predate producer teardown by a
+  broad bound. Moving the stamp to EOF or restoring a fresh post-return q
+  anchor makes the self-test fail. Reverting q to Python's process-relative
+  macOS `time.monotonic_ns()` also fails an explicit cross-process clock guard;
+  every carried q timestamp uses `clock_gettime(CLOCK_MONOTONIC)`. Both client
+  wrappers carry the q completion stamp as the fourth result field consumed by
+  `run_arm`, and live preflight proves the flushed Windows sentinel reaches q
+  before the remote producer exits.
+- Every trace-on TCP session must prove the complete seven-epoch one-stream
+  ramp from one to eight live sockets on both roles, including exact proposal,
+  preparation, ACK, settlement, attachment, and role-local ordering evidence.
+  Removing epoch 7 makes the targeted analyzer guard fail; disabling exact
+  target/live validation makes all four final-epoch SOURCE and DESTINATION
+  mutations fail. Restoring both guards returns the analyzer suite to green.
+- The build-identity self-test accepts the exact 12-character clean marker and
+  mutation-proves that the same marker with `.dirty` is rejected. Live q and
+  Windows gates apply that positive-and-negative check to both binaries.
+- The repository gate is green: `cargo fmt --all -- --check`,
+  `cargo clippy --workspace --all-targets -- -D warnings`,
+  `cargo test --workspace`, the documentation gate, analyzer tests, and shell
+  syntax checks all passed.
+
+## Known gaps
+
+- No rig datum is produced by this slice. The full live run waits for fresh
+  mandatory Codex adjudication, exact isolated builds, a successful live
+  launcher smoke, and a green endpoint preflight.
+- This four-cell run is the reduced P1 phase diagnostic, not the entire pf-1
+  hard gate. The active plan still requires the separately reviewed
+  small-fixture/P2 work, phase report, and `0f922de` historical control before
+  pf-1 closes.
+- q was not quiet during the first read-only readiness sample on 2026-07-15:
+  Time Machine AutoBackup was enabled and Spotlight was using substantial CPU.
+  The harness reports and refuses those conditions; it does not mutate them.
+
+## Reviewer comments
+
+Initial Codex review (`gpt-5.6-sol`, `xhigh`, codex-cli 0.144.4) reviewed
+`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..0fb8237c2e6f63feb9cfc613d8af1602730061b0`
+and returned `NEEDS FIXES` with three High findings. All three were accepted
+and fixed independently: destination reset fail-closed at `661cf75`, excess
+settle accounting at `1617546`, and the complete resize causal-edge audit plus
+emitter alignment at `2dd977e`. See the raw review and adjudication under
+`.review/results/otp12-pf1-rigw-harness.*`.
+
+Round-2 Codex reviewed the complete immutable range through `8fbd486` and
+returned `NEEDS FIXES`: it independently confirmed F1–F3 closed, then found two
+new High defects. F4 is an uncharged Windows-client interval before q captures
+the settle anchor. F5 is the role-bearing `rid` selecting different physical
+destination paths for paired arms, contrary to the only-initiator-varies
+contract. Both were accepted and fixed in order: F5 at `1231e42`, then F4 at
+`6ba5408`. A separate runbook audit found the missing standalone launcher mode,
+fixed at `18d3cde`; follow-up safety audit found the pre-PID-journal CIM race,
+fixed at `454ebce`. The additive Grok second eye returned a schema-valid
+`ACCEPTED` verdict with three independent red-to-green guards, but it does not
+override the mandatory Codex findings. See the round-2 raw and adjudication
+records under `.review/results/otp12-pf1-rigw-harness-r2.*`. Fresh review of
+the complete fixed range is pending; no rig run is authorized yet.
+
+Round-3 Codex reviewed
+`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..53bb5e56a864abe0ee2d2b00c411846a1e7d24d5`
+and returned `PASS` with no findings. The additive Grok review of the same
+immutable range returned schema-valid `REOPENED`, `guard_confirmed=false`.
+G3 is accepted: production role-invariant path construction is correct, but
+the path-construction/parity assertions are bare `[[ ... ]]` commands that can
+survive failure under macOS Bash 3.2. Grok's role-in-path mutation produced
+different physical destinations while `SELFTEST=1` still exited zero. The
+timing-anchor and launcher-journal mutations independently went red-to-green.
+See `.review/results/otp12-pf1-rigw-harness-r3.*`. G3 was fixed at `27c94b0`;
+the complete range still requires fresh review before any rig activity.
+
+Coder follow-up audit admitted G4 as a separate High instrument-correctness
+finding. Destination-type, finalization-state, strict-cleanup-state,
+completion-marker-removal, and signal-cleanup checks still used bare
+`[[ ... ]]` assertions that macOS Bash 3.2 can let fall through to a later
+successful command. A regression could therefore leave an unsafe destination
+type, false cleanup state, or stale completion marker while the offline
+self-test still exited zero. G4 gives each material lifecycle assertion an
+explicit failure path and seeds the signal test with a completion marker, so
+that its absence check is not vacuous. Final-command subshell predicates and
+intentional predicate returns are unchanged. Removing the production
+`SESSION_FINALIZED=1`, retaining `Q_SESSION_MAY_EXIST=1` after successful
+cleanup, or conditionally skipping completion-marker removal for a received
+signal each turns the Bash 3.2 self-test red at the intended assertion;
+restoring all three returns it to green.
+
+G4 was fixed at `7e9d2d5`. The full workspace format, strict-clippy, and test
+gate; 23 analyzer tests; Bash syntax and self-test; documentation gate; and
+diff check are green for both G3 and G4. No endpoint was contacted.
+
+Round-4 mandatory Codex and additive Grok reviewed the complete immutable
+range through `6f517ea1bdbea2f7d83f15c086d2bf5f764cf524`. Codex returned
+`PASS` with no material finding. Grok returned schema-valid `ACCEPTED`,
+`guard_confirmed=true`, exact SHAs, and independently drove the G3 role-path
+mutation plus G4 finalization, may-exist, and marker-removal mutations red
+before restoring every offline suite green. Its detached worktree ended clean
+and was removed. Review is closed; launcher smoke and endpoint preflight remain
+required before the registered run.
+
+The first live launcher-smoke attempt on q refused before launching a daemon
+or timing a transfer. G5 is accepted as a High instrument-correctness finding:
+q legitimately has the Windows peer cached on `en0`, `en1`, and registered
+`en8`, but the ARP gate concatenated all three MAC rows. It therefore rejected
+the correct peer even though `route -n get` selected `en8`. The failed attempt
+is retained as `SESSION-VOID` under
+`logs/otp12pf-rigw-20260715T113500Z-launcher` in the isolated q clone. The fix
+parses exactly the registered interface, requires one result, and pins the
+real three-interface shape in the Bash 3.2 self-test. No daemon started and no
+endpoint policy changed. Removing the interface predicate makes the self-test
+red on the three-row fixture; restoring it returns the complete self-test to
+green.
+
+Round-5 reviewed the complete immutable range through
+`06b33228d502c51da24bc2a78fba7eddcf6c0723`. Mandatory Codex independently
+confirmed G5, the exact 128-arm schedule, and role-invariant endpoint-local
+paths, then returned `NEEDS FIXES` with one separate High finding. G6 is
+accepted: the harness runs the endpoint's pre-existing
+`D:/blit-test/purge-standby.ps1` by existence and exit status only, rather
+than staging and hashing the reviewed repository helper. A stale or no-op
+helper could therefore make a warm-cache run look valid. Additive Grok
+returned schema-valid `ACCEPTED`, exact SHAs, and `guard_confirmed=true` for
+G5 after independently driving the ARP interface mutation red and restoring
+the Bash 3.2 self-test green. Its detached worktree ended clean and was
+removed. No endpoint was contacted. See the round-5 raw reviews and
+adjudications under `.review/results/otp12-pf1-rigw-harness-r5.*`.
+
+G6 now takes the purge helper only from the exact clean q checkout. After all
+read-only endpoint/fabric/quiet gates pass, the harness reserves a fresh
+per-session Windows tree, copies the reviewed helper to a temporary path,
+rejects reparse points, verifies SHA-256 before and after the atomic move, and
+records the helper hash/path alongside the four executable hashes. Every arm
+rechecks that same hash immediately before invocation and requires exactly one
+`standby-purged` success line in addition to exit zero. The helper is therefore
+covered by the executable snapshot and strict session-tree cleanup rather than
+trusted as endpoint state.
+
+The Bash 3.2 self-test functionally mocks both stage and per-arm commands.
+Removing the final post-move hash comparison turns it red at the staging
+contract; restoring it returns green. Removing the per-arm hash comparison
+turns it red before the mocked purge can pass; restoring it returns green. A
+separate order guard pins the first remote write after provenance, port,
+topology, MSS, firewall, quietness, timer, and result-stream checks. No endpoint
+was contacted by the fix or its mutation proofs.
+
+G6 was fixed at `888be4754387311e28e14d687721fd3d1315f82c`.
+Format, strict clippy, Bash syntax/self-test, all 23 analyzer tests, the docs
+gate, and diff checks passed. The first full workspace test attempt hit the
+recorded macOS `blit_utils::test_utils_list_modules` daemon-start race once;
+the isolated test then passed, and a complete quiet rerun passed with two
+expected ignores. Fresh complete Codex plus additive Grok review is still
+required before any build or endpoint contact.
diff --git a/DEVLOG.md b/DEVLOG.md
index f23c2ae..47fdd36 100644
--- a/DEVLOG.md
+++ b/DEVLOG.md
@@ -1,86 +1,90 @@
 # DEVLOG

 Entries are latest-first. Each line starts with an ISO 8601 timestamp.
 Per R5-F5 of `docs/reviews/followup_review_2026-05-02.md`: new entries
 go at the top of the file, immediately below this header, so reviewers
 scanning chronologically don't miss appended-at-the-bottom changes.
+**2026-07-15 12:09:40Z** - **CODER (otp12 pf-1 rig-W G6 purge-helper provenance fixed; still NO DATA):** Accepted round-5 Codex F1. The harness no longer trusts `D:/blit-test/purge-standby.ps1` as endpoint state: after every read-only endpoint/fabric/quiet gate, it stages `scripts/windows/purge-standby.ps1` from the exact clean checkout into the fresh per-session Windows tree, rejects reparse points, SHA-256 verifies before/after the atomic move, records the fifth provenance row, then re-hashes and requires the exact `standby-purged` sentinel before every arm. Functional Bash 3.2 mocks pin stage/copy/hash/move and per-arm hash/invoke/sentinel contracts; removing either final staged hash verification or the per-arm hash comparison turns self-test red, restoration green. The first endpoint write is source-order pinned after the read-only gates. No endpoint was contacted; fresh complete Codex + Grok review remains required before rebuild or launcher retry.
+**2026-07-15 12:01:21Z** - **REVIEW (otp12 pf-1 rig-W harness round 5 reopened; still NO DATA):** Mandatory Codex (`gpt-5.6-sol`, xhigh, codex-cli 0.144.4) and additive Grok (`grok-4.5`, grok 0.2.101) reviewed the complete immutable `4c7c754..06b3322` range. Both independently confirmed G5: q's real en0/en1/en8 ARP fixture selects exactly registered en8; removing that predicate turns Bash 3.2 self-test red and restoration green. Codex also confirmed the exact 128-arm schedule and role-invariant endpoint-local paths, then returned NEEDS FIXES with one accepted High: the harness trusts an endpoint-resident `purge-standby.ps1` by existence/exit only, so a stale/no-op helper can leave Windows warm while the run records cold. Grok returned schema-valid `ACCEPTED`, `guard_confirmed=true`, exact SHAs; its detached worktree ended clean and was removed. G6 will stage/hash the reviewed helper per session, re-hash and require its success sentinel per arm, and mutation-prove that contract before fresh complete review. No endpoint was contacted.
+**2026-07-15 11:28:55Z** - **REVIEW (otp12 pf-1 rig-W harness round 4 accepted; still NO DATA):** Mandatory Codex (`gpt-5.6-sol`, xhigh, codex-cli 0.144.4) reviewed the complete immutable `4c7c754..6f517ea` range and returned PASS with no material finding after checking role/session/path/timing/analyzer/launcher/recovery/cleanup semantics and the repaired Bash contexts. Additive Grok (`grok-4.5`, grok 0.2.101) returned a schema-valid `ACCEPTED`, `guard_confirmed=true`, exact base/head, and independently mutation-proved G3 role-path parity plus G4 finalization, cleanup-ownership, and signal-marker removal red→green under Bash 3.2; restored self-test, 23 analyzer tests, and 41 role tests passed. Its detached worktree ended clean at `6f517ea` and was removed. No endpoint was contacted. The immutable reviewed SHA, not this later verdict-record commit, is the only build allowed into launcher smoke, endpoint preflight, and the registered run.
+**2026-07-15 11:13:07Z** - **REVIEW + CODER (otp12 pf-1 rig-W harness rounds 3–4; still NO DATA):** Mandatory Codex (`gpt-5.6-sol`, xhigh) reviewed the complete immutable range through `53bb5e5` and returned PASS with no findings. The owner-requested additive Grok (`grok-4.5`) independently confirmed one `Transfer`, SOURCE-send/DESTINATION-receive semantics, worker parity, timing anchor, launcher gate, and cleanup, but returned schema-valid REOPENED / `guard_confirmed=false`: under macOS Bash 3.2, the bare path-parity `[[ ... ]]` assertions survived a real role-in-path mutation and the self-test exited zero. G3 fixed every path assertion with explicit failure handling (`27c94b0`); the exact mutation now fails and restoration passes. A systematic follow-up audit admitted separate G4: material destination/finalization/cleanup/marker/signal assertions had the same Bash 3.2 risk, and the signal marker check was unseeded. G4 fixed them and seeded the signal case (`7e9d2d5`); missing finalization, retained may-exist state, and skipped signal-marker removal each mutate red, restoration green. Both slices passed fmt, strict clippy, the full workspace suite, 23 analyzer tests, Bash syntax/self-test, docs, and diff checks. No endpoint contacted; fresh complete Codex + Grok review is required before launcher smoke or preflight.
 **2026-07-15 05:51:14Z** - **REVIEW (otp12 pf-1 TCP phase trace):** The implementation at `5b8cc29` completed the mandatory Codex loop with a clean `gpt-5.6-sol` PASS, independent focused/full tests, and the previously recorded red→green mutations. The owner-requested additive Grok second eye was run with `grok 0.2.101` / `grok-4.5` in a clean detached worktree at the exact `4dba35a..5b8cc29` range. Both the initial response and the playbook's one schema retry failed closed: `structuredOutput` was null, output contained four then five concatenated payloads, `stopReason` was `Cancelled`, and the free-form thought contradicted the payload or admitted the guard had not run; the retry's payloads explicitly had `guard_confirmed=false`. Recorded as a CONTESTED reviewer-protocol failure, not a Grok PASS and not an admitted code finding. The worktree stayed clean; no code fix was required. Records: `.review/results/otp12-pf1-session-phase-trace.{codex.md,gpt-verdict.md,grok-second-eye.json,grok-second-eye-retry.json,grok-verdict.md}`.
 **2026-07-15 03:55:46Z** - **REVIEW (otp-12 worker parity, Grok second eye):** Ran the repo reviewloop with `grok 0.2.101` / `grok-4.5` against the exact code range `6b0f01c..42b9b38` in a detached disposable worktree. The structured verdict was **ACCEPTED**, `guard_confirmed=true`, with exact base/head SHAs and no material finding. Grok independently reran both exact-8 role tests, the both-layout gated-resize-ACK progress test, and refusal/arbitration/unknown-capacity guards. Its own production mutation changed zero receiver capacity back into a one-stream cap: the destination-initiator role test failed at 1 versus 8, then passed after restoring the reviewed head. The review worktree was clean and removed. Raw JSON + adjudication: `.review/results/otp-12-worker-parity.grok-second-eye.json`, `.review/results/otp-12-worker-parity.grok-verdict.md`.
 **2026-07-15 03:19:30Z** - **CODER (otp-12 worker parity closed; no hardware data):** Re-grounded after the prior session's incorrect push/pull conclusion and proved the current architecture from source: there is one byte-moving `Transfer` RPC; a destination-side caller connects to the SOURCE daemon, declares DESTINATION, and that daemon sends through the same SOURCE pipeline; only initiator→responder socket acquisition differs for NAT/firewall reachability. The worker concern itself was real: identical 10,000-file fixtures settled at 3 vs 2 workers, and destination initiation interpreted legal `max_streams=0` as a one-worker cap. Fixed shared receiver-ceiling semantics, nonblocking payload/resize convergence, terminal refusal, and atomic tuner/shape/settlement arbitration. Final role pins are exactly 8/8; with resize ACK #2 held, both layouts transfer all 2,000 tiny files before convergence and land identical trees. Codex review converged through five FAIL rounds (2+1+2+1+1 findings, all accepted): pre-payload serialized RTTs, refusal reproposal, split-atomic races/ABA, stale tuner decisions, and scheduler-dependent guards. The final guard observes a real settlement `try_lock` and carries one guard-owned acquisition identity from eligibility through claim; drop/reacquire, omitted refusal, pre-dispatch settlement, and release-only `debug_assert!` mutations all go red. Final Codex re-review PASS with no findings; fmt, strict clippy, docs, release compilation, repeated debug/release guards, and the full workspace suite are green (1,490 passed, 2 ignored). Commits `a76b785`, `cfd9dd7`, `8e993aa`, `641916e`, `f7f12ec`, `42b9b38`. **No Mac↔Mac run was performed:** worker parity is no longer a blocker, but the live round-12 instrument can call a 1.092/acceptance-PASS cell `REPRODUCES` and its end gate can grade after a 10GbE→1GbE renegotiation because it omits link speed. **No next hardware experiment was selected:** the owner still chooses between repairing/re-reviewing that instrument before the missing Mac↔Mac cell and instrumenting P1's dial/accept path on rig W as the round-12 reviewers recommended.
 **2026-07-15 02:30:00Z** - **CODER + ADJUDICATION (round 11 fixed and re-reviewed round 12 — and the round-12 reframe caught that the whole experiment was aimed at a question the existing data already answers; still NO DATA, claude)**: **Fixed all of round 11 across 7 commits, each with a guard proof. ENGINE (`4270e42`, `584845f`, `352ed5c`, `7b685f9`, `4bfbb32`):** `B` applied once inside `classify` and only ever hardening; **`B ≥ T/2` on any measurand ⇒ `CONTROLS-NOT-CLEAN`** (owner D-2026-07-14-4, verbatim "Refuse to grade" — on a slow measurand `T` is capped at Δ_ref while the permitted control bias is a *fraction* of the arm, so `T−B` can go negative and license an "effect" that is mostly rig); **MIXED decided on the UNHARDENED cell states** so a noisier rig can't upgrade an inconclusive session to `REPRODUCES`; the **arm medians use the conventional even-n median** not the low median (the low median shrank `T` on a bimodal arm — and rig W's fast arm is known bimodal); and the **arm-count check finally has a case AND a selective mutation** (both reviewers flagged it unguarded — deleting it left the suite green while a skewed CSV graded `REPRODUCES`; the test helper gained `extra_rows` so an unpaired-valid-row CSV is expressible at all). Guard: **40 cases, 19 mutations, all killed.** **HARNESS (`1562cde`):** the round-11 **BLOCKER closed** — the registered topology is now pinned in code and refused from the environment; `topology_gate` proves the NIC (MTU 9000, negotiated 10Gbase-T, active) and `mss_gate` proves the PATH (negotiated MSS 8948 via `getsockopt(TCP_MAXSEG)`, the gate pf-0 used, plus the egress IP), re-checked at session end and voiding on change; plus the four HIGHs (drain producer statuses, `resolve_disk`'s `df`/`grep` statuses, both Time Machine gates no longer coercing `"0%"` to "disabled", `ps` no longer mapping every error to GONE) and the MEDIUMs. **Two of my OWN fixes were caught by RUNNING the self-test, invisible to `bash -n`:** the pinned topology literals were first placed ABOVE the override check so the harness refused EVERY run (a protection that cannot PASS is as dead as one that cannot FAIL), and a new sentinel was `:`-delimited around a MAC address (all colons) so a gate went BLIND on a good link. **Fabric mutation proof:** pointing the rig at nagatha's 1GbE `en0` is caught by THREE independent gates (link, NIC, path); each fabric claim fires on its own when falsified. **PREREGISTRATION → rev 11** (`bfae311`): the summary, the state table, the two new rule clauses and the two unregistered gates (fabric + per-run RTT) all now match the code. **THE REFRAME — D-2026-07-14-5 (owner): the FIRST review asks "is this the best way to do it", not "is this correct per the plan".** The owner's diagnosis, verbatim: *"You keep finding problems in the plan you wrote, so it's likely codex will have a more coherent idea."* Round 12 was re-dispatched under this rule — codex and grok each given the END GOAL and told a well-built-instrument-pointed-at-the-wrong-question was the most valuable answer available. **BOTH said DO NOT RUN IT**, and codex's design review raised a would-be BLOCKER: the harness that measured P1 (`bench_otp12_win.sh:505`) flushes with no settle, so a free-writeback artifact could have MANUFACTURED P1, and the numbers should be re-measured before proceeding. **I relayed that to the owner as a pivot — WRONGLY, without checking it against the recorded data.** The owner refused the pivot and demanded it be put to consensus. **ADJUDICATION (`p1-adjudication-r1.{codex,grok}.md`): both reviewers, independently, from the actual CSVs, reached `P1 REAL` (high confidence), reversing the artifact claim.** The artifact hypothesis fails every one of its own predictions in the recorded data: on `wm_tcp_mixed` the **flush is symmetric** (72 vs 73 ms) against a **~300 ms** effect; the effect is entirely in **transfer time** — removing flush makes P1 *worse* (1.385 → **1.417**), and every slow-arm run (1012–1104 ms) exceeds every fast-arm run (647–820 ms) with **zero overlap**; the **same-fixture gRPC control passes at 1.020** (a writeback artifact would hit it identically — same files, same dirty pages); and Linux uses the same immediate-flush method with **no P1** (flush 780 vs 780). Grok confirmed across all three P1 sessions (98.6% / ~100% / 99.0% of the effect in transfer time). Both independently cited the project's own precedent: a *real* durability-accounting artifact was caught here once (`2c0af86`) precisely because it **polluted the gRPC control** — P1 is carrier-specific, so it passes that test the way a genuine code effect must. **P1 is a real property of the macOS↔Windows TCP-mixed transfer path; the release blocker is genuine, not measurement error.** Both point next at instrumenting the dial/accept transfer path on the rig where P1 lives, not at the Mac↔Mac run or a settle re-measure. **NO CODE (crates/proto) TOUCHED, NO RIG TIME, NO DATA — suite still 1488 as of `bb28ddd`.** Also this session: recreated nagatha's missing `f35702a` worktree + release build (STATE's "both Macs ready" had covered daemons and Time Machine, not binaries). **NEXT: owner's call on direction — instrument the TCP dial/accept path on rig W (both reviewers' recommendation), vs the parked Mac↔Mac run.** D-2026-07-14-4 and D-2026-07-14-5 recorded.

 **2026-07-14 22:45:00Z** - **CODER (rounds 7–11 of the Mac↔Mac instrument — the owner said "simplify", and the RULE turned out to be where the bugs lived; still NO DATA, claude)**: *Backfills the gap the 21:10Z drift pass flagged: rounds 7–11 had landed with no DEVLOG entry.* **ROUND 7 (`f7f6e17`) — both reviewers NOT READY again, and the diagnosis changed the plan.** Every new finding was mine: **the drain failed open AGAIN** — my own round-5 rc fix was itself fail-open, because if `hrun` prints `drained_*` and *then* exits non-zero, `|| echo DRAIN-ERROR` appends a second line and the value **still starts with `drained`**; prereg rev 7 **contradicted itself** (older sections still described the bar-based materiality the rewrite had removed); and `CONTROLS-UNCERTIFIED` told the operator to escalate to `RUNS=16` while the harness refused anything but 8. The pattern was now unmistakable: **four of the last five BLOCKERs were in the DECISION RULE, not in the measurement.** **THE OWNER'S CALL — D-2026-07-14-3, verbatim: "simplify"** (`30d4374`). Not "harden it again". **ROUND 8 (`79c1f2d` rewrite → `08570b5` review): the rule was cut from 647 engine lines to 321** — one statistic, one threshold, four mutually exclusive states — **and the simplification promptly introduced a REAL hole of its own.** codex found it **in the rewrite's own reasoning, not in a branch**: I had deleted the `UNSTABLE` state arguing "a bimodal arm widens the CI, so it lands in UNCLEAR". **That is TRUE at n=8 — where the ≥95% order-statistic CI *is* the full range and therefore cannot trim — and FALSE at n=16**, where the interval `[d₍₄₎,d₍₁₃₎]` **trims three outliers per side**, so a bimodal arm yields a *narrow* CI and a **false null**. It drove `CI=[1,1]`. **ROUND 9 (`8830fda` rework → three findings, 8/8 accepted):** grok reproduced a **BLOCKER** I had created by half-deleting the escalation — I removed 16 from the registered pair counts but left the completeness check reading `len(d) >= PAIRS`, so **a 16-pair CSV was graded, the CI picked k=4, and it TRIMMED the three pairs at −500 while keeping the thirteen at +200 → `REPRODUCES`.** The entire rule leans on the n=8 identity (the interval cannot trim); **that only holds if n is EXACTLY 8**, and it now is. grok also killed **`B` (the residual arm bias the controls could not rule out)**: I had defined it as the max |CI bound| over clean controls, but **the CI is the wrong quantity — it is an interval for the MEDIAN and it trims**; the honest bound is the control's **full RANGE**. grok drove a control with range `[5,40]` yielding `B=5` instead of `40`, and a measurand at `+105` then read `REPRODUCES`. codex (reworded prompt) added that **`B` carried RAW MILLISECONDS between controls of different arm speeds** — the same 4.9% bias is 122 ms on a 2500 ms large-file control and a different number on a fast one — so **`B` is now RELATIVE to the arm**. Also: the drain accepted **`"."`** as a number. **ROUND 9's OTHER LESSON, recorded because it cost a whole review: codex's first round-9 run was KILLED BY A CONTENT FILTER** after reading 85k tokens and **produced no review at all** — its file contains only stale round-1 material quoted out of the prereg's own history (`eb864ac`). **A killed run's file must never be mistaken for a review.** The trigger is the framing: *"find the fail-open protection, assume the defect class is present"* reads as vulnerability scanning when it sits on ssh/sudo/pgrep code. **Split the review (engine / harness) and word it as plain measurement-correctness, and it goes through.** **ROUND 10 — two BLOCKERs, and both were self-inflicted by deletions.** Engine (`0caca92`): **the registered cell IDENTITIES AND ROLES came from mutable environment variables** — omitting a dirty control from `CONTROL_CELLS`, or dropping it from `REGISTERED_CELLS`, let the session grade anyway. **Which cells are controls is part of the pre-registration, exactly like `DELTA_REF`, and for exactly the same reason: otherwise the rule can be retuned from the command line, after the data exists, toward the answer you want.** Pinned in code; the engine now refuses an env set that disagrees. (Also: all-zero rows reported an `EFFECT`.) Harness (`8997f92`): **THE BUILD PIN WAS GONE, AND I DELETED IT MYSELF.** `EXPECT_SHA` was never compared to `REGISTERED_BUILD`, so **any sha was accepted — including `f35702a.dirty`**. It was collateral damage from cutting the escalation block out: adjacent lines, and the slice took both. **A regression introduced by a deletion, in the very commit that was removing machinery to make the thing safer.** Plus: five RTT samples at *preflight* are not a bound on the ssh dispatch of a run taken minutes later. **ROUND 11 (`e65863c`) — THE ENGINE HAS NO BLOCKERS FOR THE FIRST TIME IN ELEVEN ROUNDS.** Two HIGHs remain in it, both real: **`B` can EXCEED `T`** on a slow measurand (`T` is capped at Δ_ref=230 ms while the controls' permitted bias is a *fraction* of the arm, so above ~4600 ms `T−B` goes negative — a null becomes impossible while `T+B` licenses an "effect" that is mostly rig); and **`B` hardens each cell but can make the SESSION verdict EASIER via `MIXED` precedence** (if `B` pushes one cell out of `EFFECT`, `MIXED` stops firing and the session reports `REPRODUCES` instead of the inconclusive `MIXED`). **THE HARNESS STILL HAS ONE BLOCKER: the registered topology is not enforced — NIC/IP/MAC are env-overridable and THE MTU IS NEVER CHECKED**, so the run could silently go over the **1GbE** NIC or at **MTU 1500**. pf-0 spent 256 runs establishing that MTU moves wall time; **a rig must prove it is on the fabric it claims.** Four HIGHs with it: `resolve_disk` discards the `df` pipeline status; **both Time Machine gates use `tr -cd '0-9'`, so a malformed `"0%"` reads as "disabled"**; the per-pair RTT gate exists in code but **not in the pre-registration**; the drain still discards its producers' statuses. **THE ARC, stated plainly: eleven rounds, ~110 findings, 100% accepted, none rejected — and the instrument has still never taken a single datum.** Three project claims were already retracted to harness bugs, which is why this discipline exists. **Two defect classes recur in EVERY round and the next review must assume both are present: (1) "fixed the branch I was shown, not the class"** (the same materiality bug escaped four rounds; a fail-open `pgrep` was fixed in one gate and left in its duplicate; the drain was fixed by VALUE and left failing by STATUS) **and (2) "a protection that never executes, or cannot fail"** (`SETTLE_MS` had never run in any revision — a quoting bug killed the `sleep` and its status was discarded — while the prereg asserted it for three revisions). Earned rules: **verify the instrument before believing the measurement; `bash -n` is not an execution; a protection that cannot be observed is not a protection; a mutation that cannot be killed is not a proof.** Suite untouched (**1488 as of `bb28ddd`**; zero `crates/`/`proto/` changes in any of these commits). **NEXT: fix round 11 (1 harness BLOCKER + 4 HIGH; 2 engine HIGH), re-review, THEN run.**

 **2026-07-14 21:10:00Z** - **DRIFT (STATE hygiene: the handoff log was four review rounds stale — the one place a cold session actually reads, claude)**: Owner ran `drift`, scoped to the state-hygiene pass. **No code touched, no rig time, no data taken.** **THE FINDING**: `docs/STATE.md`'s newest handoff entry (49th) said *"In-flight: the round-7 review … READ THEM FIRST. NEXT: adjudicate round 7"* — while **rounds 7, 8, 9, 9b, 10 and 11 had all landed since** (`1e03063..e65863c`, results on disk in `.review/results/macmac-harness-r{7,8,9,9b,10,11}.*`). The top of STATE was current (it correctly described round 11's open findings); only the **entry point** was lying, and the entry point is what a fresh session reads to decide what to do. **A cold session trusting it would have re-adjudicated closed work on an instrument that had moved five times underneath the entry.** Corrected in place with the git evidence rather than rewritten into a new narrative. **The rest of STATE verified TRUE** — every doc path, every `docs/bench/` directory, the pre-registration really is **rev 10**, the harness really is still at round 10's `8997f92` (no round-11 fixes started), and STATE's summary of round 11 matches the three result files line for line. Also fixed, all evidence-backed: **`docs/history/state-archive.md` created** — the rotation target AGENTS.md's `drift` definition names, which had **never existed** — and the landed otp-1..11 closed-slice record rotated into it verbatim, leaving a pointer; **`Suite 1488` was a volatile count with no anchor** → now **`1488 as of bb28ddd`**, the last commit to touch `crates/`+`proto/` (everything since is docs/scripts, so it stands without a re-run); **the rig's IPs and MTU were restated in STATE** → replaced with a pointer to `.agents/machines.md`, which owns host facts; **`"windows-latest CI pending a push"` was a recorded push-state line** (the rules say delete on sight — git owns push state) **and it was also false**: local, `origin` and `github` are all at `7fc48d3`; and the Mac↔Mac run added to `## Blocked` as a **pointer, not a restatement**. **⚠ THE GAP I DID NOT FILL: DEVLOG has NO ENTRY FOR ROUNDS 7–11.** Its newest entry before this one is 18:45Z (rounds 5+6) and it ends by naming round 7 as next. Five rounds — including the decision-rule rewrite under **D-2026-07-14-3** and the **build-pin regression** caught in round 10 — have no history entry; their only record is the review result files and the commit messages. **Drift audits history; it does not author history it did not witness**, so this is flagged rather than reconstructed. Docs only: `check-docs.sh` OK, STATE **199/200** lines. Commit `f933097`. **NEXT: fix round 11's findings (2 HIGH in the engine; 1 BLOCKER + 4 HIGH in the harness — the registered 10GbE/MTU-9000 topology is not enforced), then re-review, then the run.**

 **2026-07-14 18:45:00Z** - **CODER (rounds 5 AND 6: THE SETTLE HAD NEVER RUN — not once, in any revision — and the same materiality bug escaped a THIRD and then a FOURTH time; 25 more findings, 25 accepted, still no datum taken, claude)**: **ROUND 5 — codex (3 BLOCKER, 6 HIGH, 2 MEDIUM) + grok, converging independently; 12/12 accepted (`aebd50b`).** **THE HEADLINE IS A DEFECT THE REVIEW DID NOT FIND, WHICH ITS FINDING EXPOSED.** Codex filed a HIGH — *"failure of the required settle `sleep` is ignored, because the succeeding python fsync walk supplies the command status"* — and **executing it showed the status was ALWAYS failure**: the `awk` computing the settle duration sat inside a **command substitution**, which bash parses **fresh**, so the `\"` escapes (correct in `hrun`'s two-level strings, and correct everywhere else in the file) were **literal backslashes** to awk. Measured: `awk: syntax error`, `usage: sleep number[unit]`. **The awk errored on every call, `sleep` got an empty argument and failed, and the walk ran immediately, every time. SETTLE_MS HAS NEVER BEEN APPLIED — and it was introduced in `24660ae`, THE COMMIT THAT ADDED IT TO FIX the free-writeback asymmetry that reverses sign with direction — the artifact judged capable of MANUFACTURING a one-directional P1 out of nothing. The fix for that BLOCKER never executed, and the pre-registration asserted it through revisions 3, 4 and 5.** Nothing is retracted only because **no data was ever taken** — the fourth time this project has been saved by not having run yet, and the sharpest possible argument for the rule that found it: **`bash -n` is not an execution.** Round 5's other BLOCKERs: **`bar == FAIL` was DIRECTION-BLIND** (the bar is computed on the MARGINAL medians, the CI on the PAIRED differences, and they can point OPPOSITE ways — at n=16, thirteen `+1 ms` pairs plus three that fall below the whole distribution make the medians fail the bar *inversely* while every surviving pair is +1 ms → **`REPRODUCES` off ONE MILLISECOND**); an **UNDERPOWERED control escaped the void** — the same materiality bug's **third** branch, found independently by **both** reviewers (one zero pair drags `ci_lo` to 0, demoting a control carrying the *full* rig-W effect from `PARTIAL`, which round 4 made void, to `UNDERPOWERED`, which it did not) → a clean **`VANISHES` with every control at `D=+230`**; and **the registered constants were ENV-OVERRIDABLE** — `DELTA_REF_MS=240` turned a `RIG-VOID` into a `VANISHES`, i.e. **the pre-registered rule could be retuned from the command line, after the data existed, in the direction of the answer you want.** Also: the **same fail-open `pgrep`** I had fixed in the quiescence gate, still sitting in the **stale-daemon probe**; the drain **falling back to the APFS *synthesized* disk** (whose counters read **idle while the physical store saturates** — a false quiet, not a harmless default); a **p-hackable escalation** (a flag was sufficient); and **`SELFTEST` itself dishonest** — it labelled *every* nonzero result `[FIRED]`, **including a probe that could not answer**, exited zero, and claimed "every gate executes" while never touching drain, purge, daemon, fsync/settle or end-load. **ROUND 6 — codex (3 BLOCKER) + grok (2 BLOCKER); 13/13 accepted (`1e03063`).** Grok's sentence is the one that matters: ***"The rework again fixed the branch that was shown, not the class."*** The materiality bug escaped a **FOURTH** time: round 5 made the bar failure *direction-aware*, so codex simply **moved the outliers** so the bar failed in the **matching** direction — three outliers shift the marginal median (1000→1201, ratio 1.201) while every pair in the CI is `+1 ms` → **`REPRODUCES` off a one-millisecond paired effect** (verified before accepting). And certification used the **same threshold as materiality**, so a control carrying **`D = +229`** — *one millisecond* under the reference effect — **certified as "clean"** and the session printed `VANISHES` with the prose *"every control is CERTIFIED clean"*. **Certifying a control with the very threshold that DEFINES the effect is incoherent**: it would let the gRPC control carry all but 1 ms of P1 while we claim P1 is TCP-only. Codex added: **uncertified controls blocked only the NULL** — with every control at `D=+230` the engine still confidently declared **P1 REPRODUCED** (*"uncertainty about a rig-wide confound is not evidence that the confound is absent"*). And the settle repair was **still not provable** (Class 2, second instance): `sleep` is PATH-resolved, the walk's timer starts *after* it, and the self-test only counted files — so a **no-op sleep would have passed** while the log narrated "settle included". **A log line is a sentence, not an assertion — which is precisely how the settle stayed dead for three revisions.** **THE REV-7 ANSWERS ARE STRUCTURAL, aimed at the classes**: (1) **the 1.10 bar takes NO PART IN INFERENCE AT ALL** — it is the project's *acceptance* criterion, computed and reported; **direction** is the sign test, **magnitude** is the paired CI, **equivalence** is the CI against the margin, and no marginal statistic may decide anything; (2) **the controls are a PRECONDITION** — unless every control certifies below **HALF** the material effect, **no measurand verdict is read, neither a null nor a reproduction** (new registered outcome `CONTROLS-UNCERTIFIED`); (3) **the settle is performed and MEASURED inside the same python process as the fsync walk** (`settled_ms`, now a CSV column; the pair **VOIDS** if it did not elapse; SELFTEST measured **260 ms** on both hosts); (4) **blindness is marked EXPLICITLY** (`die_blind` → `FATAL[PROBE-BLIND]`, 10 sites) rather than inferred by **grepping a gate's prose** — grok found `timer_gate`'s wording didn't match the regex, so **a blind measurand clock scored `[FIRED]` and the self-test PASSED**; (5) there is exactly **ONE** process probe in the file, so there is no second site to forget. Also landed: the ssh dispatch bound is now **ENFORCED** (a measured bound that is not enforced is a note, not a protection), the escalation is bound to the prior session's **runs.csv hash** (a copy cannot buy a second re-roll), and the engine **refuses to grade with no controls**. Guard: **27 cases, 18/18 mutations KILLED**, 300-input fuzz over measurand **and** controls; five mutations went **STALE** when the engine was restructured and the stale-detector caught every one. **Codex ran out of credits mid-session (the owner redeemed a reset).** **Six rounds, 69 findings, 69 accepted, 0 rejected — and not one datum taken, which is the only reason none of it is a retraction.** Suite untouched (**1488**; zero crates/proto changes; docs gate OK). **NEXT: adjudicate the round-7 review (launched, results in `.review/results/macmac-harness-r7.*`), then the run.**

 **2026-07-14 16:30:00Z** - **CODER (rounds 3 AND 4 of the Mac↔Mac instrument: my timer measured a 1000 ms transfer as −1 ms, and then grok drove a clean "VANISHES" with every control dirty — 24 more findings, 24 accepted, STILL NOT CLEARED TO RUN, claude)**: **NO DATA TAKEN, and the harness now refuses a timed run outright (`exit 2` without `CLEARED_BY_REVIEW=1`).** Two more rounds on the instrument. **ROUND 3 — codex (12 findings) + grok (3 more), 15/15 accepted (`cae2e0f`).** **THE KILLER WAS MINE, INTRODUCED BY THE REWORK THAT FIXED ROUND 2**: the transfer timer captured `time.monotonic()` in **two separate `python3 -c` processes** and subtracted them. On macOS that clock is **process-relative** — I measured a **1000 ms sleep reading as −1 ms on nagatha and 2 ms on `q`**. *Negative.* Every `ms` row would have been ≈ the fsync time alone, and the invariance ratio — **the entire measurand** — would have been computed on **fsync noise**, which can manufacture or mask a one-directional effect at will. The rig would have produced a clean session, 0 voided pairs, and a **confident, meaningless verdict**. Grok caught it independently (a 500 ms sleep reading ~3 ms) before seeing codex's findings. **The repo had ALREADY WRITTEN THIS LESSON DOWN** — `bench_otp12_zoey.sh:116` uses `time.time()` and says *why* — and I reintroduced the bug anyway. **The lesson is not "add a reviewer"; it is READ THE EXISTING HARNESSES BEFORE WRITING A NEW ONE.** Fixed structurally: ONE process times itself and spawns the client, and **preflight now PROVES THE CLOCK on both hosts against a known 1000 ms sleep before any data is taken** (a non-positive transfer time VOIDs the run instead of entering the data as a "fast" row) — the bug class cannot ship again without the instrument catching it on the rig. Round 3 also found the **preflight could not succeed at all** (`grep -c` exits 1 on no match, so a **clean** binary tripped the dirty-marker probe and died; `norm_mac` used gawk's `strtonum()`, absent from macOS awk) — proof that **round 1's "fixes" were never executed**: I had run `bash -n`, not the gates. `SELFTEST=1` now runs every gate for real and takes no data; **it immediately earned itself twice** — `link_gate` was **refusing a perfectly good link** (`arp -n <ip>` prints one line **per interface**, and `q` holds entries for nagatha on en0, en1 *and* en8, so the MAC was a three-line string that could never match; it now checks the ARP entry on the NIC the traffic will **egress**, which is the more correct question anyway), and my ssh-RTT bound was **wrong by 7×** (35 ms vs ~15 ms) because `now_ms()` spawned a python per call — **I was timing interpreter startup and calling it network latency: the same cross-process trap as the timer, in a different dress.** The decision rule was broken three ways: the equivalence margin was tied to the **bar**, which on a slow arm is **wider than the effect it must exclude** (all eight `d_i = 230` at `src=2500` → **"VANISHES"**, a rig-W-sized effect in *every pair*); the negative bound was `−0.10·src` when the bar is symmetric in **ratio**, so `−src/11`; and the bootstrap CI was **not 95% at n=8** (it resolves to ≈`[d₂,d₇]`, true coverage 92.97%) while the sign test was computed and **never read**. **ROUND 4 — grok alone (9 findings, 9/9 accepted, `a9460ce`), and its headline is the one that stings**: it **drove the engine to a clean `VANISHES` while EVERY control carried the full rig-W effect** (`d_i = 230` ×8, ratio 1.092, bar PASS → `PARTIAL` → escaped RIG-VOID). **This is the SAME STRUCTURAL ERROR as round 3's**: I fixed the bar-tied margin for the **measurand** and left it bar-tied for the **controls**. **Fixing a bug in one place is not fixing its class — and that is now twice in a row.** Grok also reproduced: the engine **trusted `meta.complete`** and never counted pairs, so a **one-pair CSV emitted `VANISHES` at 0% CI coverage** (it is separately executable and hashed into the manifest precisely so it can regrade CSVs — so it must not trust the harness); and session precedence **hid a clean one-direction `REPRODUCES`** (8/8 in `nq`, reported as `BAR-FAIL-INCONSISTENT` because `qn` was noisy — a **false NON-reproduction** against the prereg's own "either direction" rule). And **my own comment lied**: it said the end-load was captured before the verdict "so a session can void on it" — **the code only logged it**. *A doc claim the code did not honour, in the commit that boasts about killing that exact defect class.* Also accepted: preflight ran the guard cases but **not the mutations**; at n=8 the ≥95% order-statistic interval **is** `[min,max]`, so one noisy pair blocks a null **forever** and the rig could only ever say `UNDERPOWERED` (**a null-incapable instrument is also broken**) → a **pre-registered escalation**: an `INCONCLUSIVE-UNDERPOWERED` session may re-run **once** at `RUNS=16` (coverage 97.9%), triggered by a **power failure and nothing else**, and the harness refuses it without `UNDERPOWERED_ESCALATION=1`; "the CI and sign test are duals" is **false once a zero exists** (`d=[0,300…360]` → 7/7, `p=.0156` significant, while `CI_lo = 0`), and the verdict text was **claiming a sign disagreement that did not exist**. One finding accepted **with a modification**: grok wanted a non-numeric `ms` soft-mapped to `INCOMPLETE`; softening a corrupt row **hides** the corruption, so it now fails **loudly and closed** (named error, exit 2). Guard test: **17 cases** (every one a defect a reviewer actually found), **11/11 mutations KILLED**, 300-input fuzz now perturbing **controls as well as the measurand** (grok: the old fuzz pinned controls clean, so **every dirty-control path — the one hiding the BLOCKER — went unexercised**). **Two of my own new mutations failed honestly and I fixed the mutations, not the report**: one went **STALE** (its target text drifted in a refactor; the stale-detector caught it) and one **SURVIVED** because the new coverage guard killed the mutant first, so it no longer proved what it named. Prereg → **rev 5**, amended before any data (the only honest time). **⛔ CODEX COULD NOT REVIEW ROUND 4 — its credits are exhausted until 2026-07-19**, so `.review/results/macmac-harness-r3.codex.md` holds a **usage-limit error, not a review**. **D-2026-07-14-2 makes codex MANDATORY and says grok "never runs alone", so the slice is NOT CLEARED and no agent may self-authorize it** — owner call: wait for codex, buy credits, or amend the rule. **Four rounds, 44 findings, 44 accepted, 0 rejected — and every rework has introduced a defect of its own.** Suite untouched (**1488**; zero crates/proto changes; docs gate OK). **NEXT: the owner's call on the codex blocker; then the round-5 review; then, only then, the run.**

 **2026-07-14 08:05:00Z** - **CODER (the Mac↔Mac rig FAILED REVIEW TWICE ON SUBSTANCE — and two instrument bugs were caught before any rig time, claude)**: **NO DATA WAS TAKEN. That is the headline, and it is a good outcome.** The Mac↔Mac experiment (queue 1(ii), the missing 2×2 cell) was pre-registered before any run, and the codex loop then dismantled it **twice**. **ROUND 1 (NOT READY — 1 BLOCKER, 7 HIGH, 1 LOW; 9/9 accepted).** The BLOCKER killed the experiment's whole reason for existing: I claimed *"P1 reproduces macOS↔macOS ⇒ **H1 DIES**, because H1 accuses the Windows accept branch."* **H1 does not accuse Windows.** Verbatim in the parent it accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **the word "Windows" appears nowhere in it**, and that code runs on macOS too, so a Mac↔Mac reproduction is **consistent with** H1, not fatal to it. (The plan itself warns: *"'consistent with H1' is not confirmation."*) **I took the framing from `docs/STATE.md` and never opened H1** — the SECOND time in one session I propagated a wrong claim about the hypotheses instead of reading them (the first: "H1/H5/H6/H7", where H5/H6/H7 are **P2**). Both fixed in STATE. Round 1 also killed: "endpoint asymmetry cancels" (it does not — switching the initiator also reassigns which Mac runs the CLI vs the daemon, and `q` is faster); "both directions must fail" (P1's rig-W signature is **one-directional** — demanding both would have let a real reproduction be waved away); a noise band that was **not a noise floor** (max of four point-estimate ratios across different carriers/fixtures/destinations); and gates that only **warned** on the Time Machine hole pf-0 had just exposed. **ROUND 2 of the rewrite (NOT READY AGAIN — 3 BLOCKER, 6 HIGH, 2 LOW; 11/11 accepted).** (1) **Rev 2 swapped one false dichotomy for another**: a reproduction on *these two Macs* could be **macOS/APFS or host×role residue**, not a "platform-general layout cost", and a null licenses only *"did not reproduce on this pair"*, **not** "Windows required" — and the "platform-residue escape" I claimed it would close **does not exist** (the parent states P1 has **no escape hatch**; D-2026-07-12-1 does not cover it). (2) **My power gate was BROKEN, with a damning counterexample**: `S = max−min` is a *range*, not a minimum-detectable-effect, and codex constructed `d = [0,180,180,190,190,200,200,200]` — **7 of 8 pairs positive, effect 83% of the 230 ms reference** — on which my rule returns **"VANISHES, powered=yes"**. It would have declared P1 *absent* on data showing a large consistent effect: **pf-0's exact error (a null from an instrument that cannot see the effect), reproduced inside the document written to prevent it.** (3) **The harness implements NONE of the registered rule** — `compute_verdicts` emits only per-cell PASS/FAIL, with no control gate, no clustering, no six outcomes; so a human applies the rule *after seeing the numbers*, which is what pre-registration exists to forbid. Rev 3 must make the **harness** the authority (it computes the verdict; the prose only describes the code) and use a real paired **equivalence** test (distribution-free CI on median(dᵢ); at n=8 the order stats [d₍₂₎,d₍₇₎]). **TWO INSTRUMENT BUGS CAUGHT BY LIVE VALIDATION, before either review and before any timed run** — the "verify the instrument before believing the measurement" rule paying for itself: (i) **the timed bracket was INVALID** — it read `t0` in one `python3 -c` and `t1` in another and subtracted them, but **`time.monotonic()`'s reference point is undefined across processes** (only same-process differences are valid); measured on this rig, consecutive cross-process reads returned **−1 ms and −4 ms**. It would have emitted plausible-looking garbage. And interpreter startup sat **inside** the window — and since the two arms of a cell are initiated by **different Macs**, any startup difference is charged to one arm: **the otp-2w failure mode (a cost billed to one arm and not the other) in a new disguise.** Fixed with a single python process bracketing the transfer. (ii) **Landed paths were assumed, so I measured them**: push lands at `<mod>/<tag>/src_<W>`, pull lands **contents directly** under `<mod>/<tag>` — both match what the harness computes. (A 10k-file push initially landed *nothing*; the cause was a **stale daemon with a different config**, which is exactly what the harness's stale-daemon refusal exists to catch.) Bracket validated live in all four shapes (local/ssh × push/pull × with/without `--force-grpc`); fixtures verified identical on both ends (1 / 5001 / 10000); rig left clean. **The quiescence gate also fired on its FIRST invocation**, refusing to start while the codex review of rev 1 was running on nagatha — nagatha is a bench END now, and the gate proves the review loop and a Mac↔Mac session genuinely cannot overlap. Suite untouched (**1488**; zero crates/proto changes; docs gate OK). **NEXT: rev 3 (prereg + harness), then run it.**

 **2026-07-14 09:40:00Z** - **CODER (the Mac↔Mac rig: pre-registration and instrument BOTH rejected by review, before a single timed run — and one of my own statistics would have declared a real P1 "absent", claude)**: Built the third harness (`scripts/bench_otp12pf_mac.sh`) for the missing 2×2 cell — macOS↔macOS on nagatha `.92` ↔ `q` `.54`. **Nothing has been measured yet, and that is the whole story: two codex rounds caught 20 defects across the design and the instrument, several of which would have produced a confidently wrong result from a clean-looking run.** **ROUND 1 — THE DESIGN (9 findings, 1 BLOCKER, 9/9 accepted).** The BLOCKER killed my central inference. I wrote "P1 reproduces macOS↔macOS ⇒ **H1 DIES**, because H1 accuses the Windows accept branch." **H1 does not accuse Windows.** Verbatim in the parent it accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — the word "Windows" appears **nowhere** in it. Windows merely *happens to be the accepting source* in P1's slow arm on rig W, and that code runs on macOS too, so a reproduction is **consistent with** H1, not fatal to it. **I took the framing from `docs/STATE.md` and never opened H1** — the *second* time this session I propagated a wrong claim about the hypotheses instead of reading them (the first: "H1/H5/H6/H7", when H5/H6/H7 are **P2**). H1 now carries a **canonical note** in the parent so the shorthand cannot mislead a third time. Also killed: "both directions must fail" (which would have **rewritten** P1, whose rig-W signature is one-directional — `wm` FAILS, `mw` PASSES — and let a real reproduction be waved away as "machine asymmetry"); "endpoint asymmetry cancels" (it does not — switching the initiator also reassigns which Mac runs the CLI vs the daemon, and `q` is faster); and a "VANISHES" branch with **no power gate** — pf-0's exact error, about to repeat. **ROUND 2 — THE INSTRUMENT (11 findings, 3 BLOCKER, 11/11 accepted).** (1) **The harness never computed its own registered rule** — it emitted per-cell PASS/FAIL only, so the six-outcome decision would have been applied **by hand, after seeing the numbers**, which is precisely what pre-registration exists to prevent. Now mechanized (`scripts/otp12pf_mac_verdict.py` → `session_verdict.txt`). (2) **My noise statistic would have declared a REAL effect absent.** I graded "vanished" against `S = max(d) − min(d)` — a **range**, which grows with n and is dominated by outliers. Codex's counterexample, which my code accepted: `srcinit=2000, d=[0,180,180,190,190,200,200,200]` → `D=190, S=200` → **"VANISHES"** — on **7/8 positive pairs**, an effect **83% the size of rig W's Δ_P1**. It would have reported "P1 requires the Windows peer" off an effect nearly as big as P1 itself. Replaced with a **bootstrap 95% CI on the median** (seeded → deterministic) + an **exact sign test**, and a null is now an **equivalence** result: VANISHES requires the CI to **exclude a bar-breaching effect**, not merely to look small; otherwise **INCONCLUSIVE-UNDERPOWERED**. Pinned by `otp12pf_mac_verdict_test.py` and **mutation-proven** (restore the range rule → the counterexample flips back to VANISHES and the test fails). (3) The inference **still** overreached ("platform-general cost" — two machines cannot license that); now scoped to **this pair**. **THE TWO INSTRUMENT BUGS THAT COULD HAVE MANUFACTURED THE RESULT**, both fixed: **(a) the durability check was FAIL-OPEN** — `os.walk()` of a missing/empty path returns **0 files in 0 ms and reads as a fast, successful flush**. The arms need *different* landed paths (blit's rsync slash semantics: a push to `/bench/RUN/` lands `RUN/src_<W>`; a pull into `RUN` lands files **directly in** `RUN` — verified empirically), so a wrong path would charge one arm **zero** durability — **the otp-2w bug that once manufactured P1**. The walk now returns its **file count and byte sum** and the pair VOIDs unless both match the fixture. I found this one myself, minutes before the review returned it. **(b) The free-writeback gap REVERSED SIGN WITH DIRECTION** — dirty pages flush "for free" between the client exiting and the fsync starting, and that gap is longer for whichever arm ran over ssh, which is `destinit` in `nq` and `srcinit` in `qn`. **P1's signature is one-directional, so this artifact could have produced the finding out of nothing.** Rather than argue, I **measured** it: a 10/20/200 ms pre-fsync delay produced **no measurable change** in fsync time (72–94 ms, no trend — APFS fsync here is per-file-metadata bound, not writeback bound), and fixed it structurally anyway with a fixed equal settle on both arms. Also: **every environmental gate failed OPEN** (a `pgrep` error read as "quiet", a `tmutil` read error read as **"disabled"**, `top` failures read as 0%) — the same class as pf-0's `ps` decaying average reporting a *finished* backup as 255%. All now fail **closed**. And the ARP gate accepted **any** MAC, so the documented **own-MAC black hole** would have passed; it now compares measured MACs both ways and asserts the route egresses the 10GbE NIC. **THE QUIESCENCE GATE PROVED ITSELF ON ITS FIRST INVOCATION** — it refused to start because codex was running on nagatha, which is now a bench **END**, not just the driver. Suite untouched (**1488**; zero crates/proto changes). **NEXT: round-2 review of the instrument, then the run — which needs BOTH Macs quiet (the owner's codex sessions must be closed).**

 **2026-07-14 07:10:00Z** - **CODER (D-2026-07-14-1: re-record the baselines at jumbo — and my first draft of the owner's own decision was NOT EXECUTABLE, claude)**: Owner chose, from three options, **"Re-record the baseline at 9000"** — because pf-0 showed the committed anti-drift ceilings were recorded at **MTU 1500** while the fabric now runs 9000, and a reference must share the MTU of the sessions graded against it. I recorded it (`d71c0ed`), propagated it, and sent it through the loop. **codex: NOT READY — 3 HIGH, 2 MEDIUM, 1 LOW, 6/6 accepted (`c9933d8`).** The owner's *decision* was never in question; **my rendering of it could not be performed.** (1) **The guard I never wrote.** `OTP12_ACCEPTANCE_RUN.md` D2 exists, in its own words, so that *"the fixed pre-cutover bar must not be loosened by a slower old rerun"* (its design finding F2) — and a re-record re-rolls **hardware and day state as well as MTU** (rig W's Mac end is now `q`, not the nagatha that recorded 2026-07-10). I proposed re-recording with **no non-loosening guard at all**: old-file immutability would have survived while the *functional* freeze quietly died. Applying F2 rather than inventing a rule, the reference is now the **per-cell MINIMUM of {2026-07-10 median, re-recorded 9000 median}** — it can only tighten, and a cell whose re-record is SLOWER is **flagged, never silently adopted** (the old build getting slower on faster hardware would mean the rig drifted). (2) **I amended D5 and MISSED D2**, which still mandated the 2026-07-10 median — the acceptance contract contradicting itself with one `BASELINE_SUMMARY` to satisfy both. Both baseline READMEs are now re-labelled **SUPERSEDED AS THE ACCEPTANCE REFERENCE / retained as historical MTU-1500 records** (data untouched). (3) **Rig Z has NO clean "original old build"** — its otp-2 baseline ran a **dirty `731023b`** daemon (the clean-pair discipline forbids reusing it) while its client was a clean `e757dcc`, so my "same old build" rule was **impossible** there. Resolved on evidence: `git diff 731023b e757dcc -- crates proto` is **EMPTY** (identical committed daemon code) and **otp-12a already staged a clean `e757dcc` rebuild** — so rig Z re-records on a clean `e757dcc` pair. Precedent, not novelty. (4) **The "3–4% faster at jumbo" rationale was OVER-APPLIED** — it is ONE cell (`wm_tcp_large`), ONE rig, and **both arms are the NEW build**; pf-0 measured no small cells, no rig-Z cells, no OLD-build MTU response. **Exactly the same failure mode the pf-0 review caught hours earlier: a real result stretched past its domain.** The justification is now **methodological** (reference and graded sessions must share the fabric's MTU), which is firmer than the number ever was. (5) **STATE contradicted itself**: I set NEXT ACTION to pf-1 while STATE's own queue still requires the **Mac↔Mac** experiment before any pf code, and the newest handoff still pointed at the decision I had just made. **Corrected: NEXT is the Mac↔Mac rig** — the missing 2×2 cell, and it **discriminates H1 outright** (reproduces ⇒ P1 is macOS-side and H1, which accuses the *Windows* accept branch, DIES; vanishes ⇒ H1 strongly supported) — then pf-1. The re-record is a **pf-final** prerequisite, not a pf-1 blocker. **Verified rather than assumed** (before the review landed): zoey really was at 1500 when otp-2 was recorded — its pre-jumbo `systemd-networkd` backups (`*.premtu`, dated 2026-04-30) carry **no `MTUBytes` stanza**; the 9000 configs were written 2026-07-13. **And a gate I broke**: I committed `bb912f4` while `check-docs.sh` was FAILING (STATE 202 lines vs its 200 cap) — the loop's gate is never-skip, never-commit-on-failure. Fixed in `9957d44`, recorded rather than amended away; every commit since ran the gate BEFORE committing. Suite untouched (**1488**; zero crates/proto changes).

 **2026-07-14 06:20:00Z** - **CODER (pf-0: MTU is KILLED as a cause of P1 — the A-B-B-A ran clean, and then codex proved my CLAIMS outran my DATA, claude)**: **THE RESULT**: the pre-registered A-B-B-A MTU experiment ran on rig `q` — four sessions (9000, 1500, 1500, 9000), RUNS=8, **256 timed runs, 0 voided**, MSS gate held at start AND end of every session (8948 jumbo / 1448 at 1500). `Δ_9000 = 236 ms`, `Δ_1500 = 229 ms`, measured noise floor `N_Δ = 78 ms`, **`r = −3.1%` → KILLED (r < 20%)**. Raising the MTU recovers **none** of P1's gap; P1 FAILS in all four sessions (1.237–1.362) regardless of MTU while every control passes in all four. **The null is not vacuous**: `wm_tcp_large` ran **3–4% faster at jumbo on BOTH arms**, so the manipulation demonstrably reached the wire — the benefit is **symmetric**, which is exactly why it cannot explain an **asymmetry**. Evidence: `docs/bench/otp12-jumbo-win-2026-07-13/` (`363fa6f`); plan amendment `63f400e`. **THEN CODEX (gpt-5.6-sol, ultra) RETURNED NOT READY — 7 findings, 7/7 ACCEPTED (`11f0c2a`)**, and the pattern is worth naming: it **independently recomputed every number and CONFIRMED them** (Δ 275/241/217/197, N_Δ=78, r=−3.0568%, N_arm=72). The arithmetic and the rule-application were faithful. **Every single finding was a CLAIM that outran the DATA.** (1) I wrote "**EXCLUDED**", "the environmental escape is closed", "P1 is a property of the code, not the wire" — from ONE environmental variable, with **segment fill unmeasured**. The registered outcome is "KILLED as a material cause"; the prereg's own round-2 F5 exists to prevent exactly that overreach and I ran it again. (2) **The sharpest one: the experiment is NOT POWERED for its own CONTRIBUTING boundary.** 20% of Δ_1500 = **46 ms**, which is **BELOW the 78 ms noise floor I myself measured**. It can exclude a DOMINANT effect (≥114 ms); it **cannot** exclude a contributing-size one. I reported a KILL without noticing my instrument could not have seen the thing it was killing. (3) I declared pf-final's committed rows **VOID** and enumerated "only two ways forward" — but that baseline is a deliberately **FROZEN anti-drift ceiling** (D2/D5) and rewriting the acceptance contract **is not an agent's call**. The substantive point survives and is sharper: jumbo made both arms 3–4% faster, so a jumbo NEW arm graded against a **1500-recorded** ceiling is **LENIENT, not conservative** — the MTU gain flatters the ratio and could let a real regression pass. (4) I claimed the same-session interleave "is the only design with enough resolution": 78 ms is **between**-session noise; it rules OUT cross-session grading but says **nothing** about the paired within-session variance, which pf-0 never measured. **pf-1 must now measure its own paired floor and register a resolution check before grading any hypothesis.** (5) **H5/H6/H7 are P2 hypotheses, not P1** — only H1 (+H2's residual) bears on P1. This error was **PRE-EXISTING and propagated**: it already stood in the q-baseline README and `docs/STATE.md`; I copied it without checking the plan's own hypothesis list. All three fixed. (6) "committed before any datum existed" is literally false (rev 4 post-dates the `q` baseline) — the *rule* was fixed in rev 3 before any S1–S4 datum, which is the claim that actually matters. (7) The masking guard silently collapsed replicate medians by **mean**; the prereg never specified it. Outcome invariant, but "exactly as pre-registered" overstated the spec. **MY OWN FINDING, which explains the noise floor the two BLOCKERs turn on**: recomputing medians from the RAW `runs.csv` instead of trusting `summary.csv` shows **the fast (`win_init`) arm is BIMODAL** (~730 ms and ~840 ms clusters). S1 drew 6 low/2 high; S4 drew 2 low/6 high **AT THE SAME MTU** — that **mode mixture, not MTU**, produced the 72 ms replicate spread that sets N_Δ. `mac_init` is stable to **5–6 ms**. **Trap for pf-1: a counterfactual that merely shifts the mixture would masquerade as a recovery** — grade the distribution, not the median. The MTU verdict is robust to it (pooling all 16 runs/condition: r = −4.7%, same grade). **RIG FAILURES, for the record**: Time Machine had autobackup ON on `q` and had fired 1 minute before the run (hourly cadence, one destination on `skippy` — the same 10 GbE fabric); the harness's quiet-gate only refuses on codex/cargo/rustc and **would have sailed straight through it**. Owner disabled it. Then three harness starts died at the old-pair smoke with a gRPC `transport error` — I blamed the MTU flip, then a daemon startup race, then the working directory, and **falsified all three myself** (the daemon binds in 169–665 ms; a hand-run smoke succeeded every time; 30/30 connects survived an MTU set). The real cause was a **physically flapping `en8`**, which the OWNER spotted and reseated; after that, 5×1 GiB at 891–897 ms and all four sessions ran clean. I should have said "intermittent" an hour earlier instead of hunting a deterministic culprit. A `bash -x` diagnostic session that DID pass was **discarded, not banked** — it differed from its own replicate, and the design requires the four sessions be identical. Suite untouched (**1488**; zero crates/proto changes — docs gate `check-docs.sh` OK). **NEXT: the owner's call on the MTU-mismatched frozen baseline (it blocks pf-final's assembly), then pf-1.**

 **2026-07-14 00:15:00Z** - **CODER (P1 REPRODUCES ON A SECOND MAC; a new dedicated bench Mac; and I contaminated an hour-long experiment with my own review loop, claude)**: **THE RESULT**: P1 had only ever been measured on ONE Mac (nagatha), yet every live hypothesis (H1/H5/H6/H7) assumes it is a property of the macOS<->Windows PAIRING. Nobody had ever tested that. On the new Mac `q` (M4 mini): `wm_tcp_mixed` = **1.385 FAIL** at MTU 9000, while ALL THREE controls PASS at **1.002-1.043 in the same session** — so the rig's asymmetry noise is ~2-4% and P1 is an order of magnitude outside it. **P1 follows the platform pairing, not the machine**, and the signature is unchanged and sharp: TCP only (gRPC 1.020), `mixed` only (`large` 1.002), destination-initiator only (reverse direction 1.043). **And it FAILS AT JUMBO** — so MTU 9000 does not dissolve P1, which was Queue 1a's whole premise. What is still unmeasured is how much MTU *contributes* (a FAIL proves only that jumbo is INSUFFICIENT); that needs the matched 1500 arm. Evidence: `docs/bench/otp12-q-baseline-2026-07-13/`. **THE OWNER'S QUESTION THAT REFRAMES THE HUNT**: *"have we tested mac<->mac?"* — **No, and it is now possible** (nagatha .92 + q .54, both 10GbE/9000). The 2x2: Linux<->Linux = NO P1 (8/8 PASS); macOS<->Windows = P1 (1.237/1.300/1.385); **macOS<->macOS = UNTESTED**. It discriminates the hypotheses outright — reproduces => P1 needs no Windows peer, it is macOS-side and **H1 DIES** (H1 accuses the WINDOWS accept branch); vanishes => P1 REQUIRES the Windows peer and H1 is strongly supported. The plan is currently hunting a mechanism without knowing WHICH HALF of the pair is at fault. Needs a 3rd harness variant (rig-W's is Windows-specific, the Linux one Linux-specific); scheduled for nagatha idle time. **THE FAILURE OF THE DAY — I contaminated my own experiment.** The first A-B-B-A MTU run (4 sessions, 53 min) ran with codex jobs chewing the Mac's CPU — **and the Mac is ONE END of rig W**. I had written that exact warning to the owner hours earlier and then did it anyway. The contamination is **asymmetric** (mac_init runs the client ON the Mac; win_init runs it on Windows), so CPU starvation **inflates Delta and MANUFACTURES P1** — the very finding under test. Same shape as the durability retraction: a cost billed to one arm and not the other. **The noise model caught it**: `wm_tcp_large` read 911ms in S1 and 1847ms in S4 AT THE SAME MTU, and the noise floor came out at 473ms — larger than the 325ms gap under test. Under my own rev-2 design (one session per condition) it would have looked clean and been REPORTED. The replicates existed only because codex round 2 forced them. Run discarded; a quiescence GATE now refuses to start a session while codex/cargo/rustc runs, and records load1 per session. Then I made it worse: `pkill -f codex` killed the OWNER'S OWN codex sessions. Never blanket-kill; ask. **MTU PREREG rev 1->4, codex 15/15 accepted across two rounds.** Round 1 (7 findings) killed a FACTUAL ERROR at the premise: "`mixed` is the most packet-heavy fixture" — the stated reason for the whole experiment, repeated in STATE — is FALSE (at MSS 1448, `large` ~741k segments vs `mixed` ~378k; **`large` is packet-heaviest by ~2x**). Round 2 (8 findings, 5 blocking) found the root defect: **every threshold I wrote was INVENTED, because the design had no noise model** — RUNS=8 measures variance WITHIN a session while the entire MTU comparison is BETWEEN sessions, and MTU was perfectly aliased with session order. Codex's counterexample passed every guard I had written (a shared 985ms floor => ratio 1.000, r=100%, fast arm regressing only 4.9%, inside my invented 5% tolerance). Fixed by counterbalanced **A-B-B-A** with same-MTU replicates supplying a MEASURED noise floor. Also withdrew a falsifier that would have KILLED a true result. **NEW BENCH RIG `q`** (M4 mini, 10GbE .54, MTU 9000, 1.18 GB/s — faster than nagatha): dedicated and quiet, which **permanently decouples the codex loop from rig-W benchmarking**. Validating it took four attempts because I kept accepting instruments that reported success while measuring nothing: (1) a host route that created a BLACK HOLE (next hop = the NIC's own MAC; 100% loss while `route -n get` still said `interface: en8`); (2) my own throughput probe reporting "**6787 MB/s — 10GbE CONFIRMED**" from a transfer that had FAILED on publickey and moved ZERO bytes; (3) its replacement flagging a FALSE failure (78 MB/s) until a control showed nagatha's known-good 10GbE scores the same 79 — the test was bound by Windows' ssh, not the wire. **The fix that worked every time was a CONTROL — an identical measurement against a known-good reference — not more care.** Rig facts + every trap: `.agents/machines.md`. Suite untouched (**1488**; zero crates/proto changes). **NEXT: the A-B-B-A MTU run on `q` (~55 min, staged and validated), and the Mac<->Mac rig.**

 **2026-07-13 21:30:00Z** - **CODER (blit silently drops Windows attributes + ADS — found while benchmarking against robocopy; and the local path does not scale, claude)**: Owner asked for a robocopy baseline and then, seeing the numbers, asked the question that broke my finding open: *"robocopy with /mt:N beats our tar streaming? or was that single-threaded robocopy?"* It was **8-thread robocopy against 1-worker blit** — I had passed `/MT:8` (matching `bench_baseline_tools.sh`) while blit's local apply ships **one** worker (`transfer_session/local.rs:602`; `--workers` is a hidden debug flag that sets `debug_mode`). My headline "blit is 2x slower" was an artifact of my own setup. **At EQUAL concurrency the result inverts**: blit BEATS robocopy at one thread (small 0.91, mixed 0.88) and LOSES at eight (1.92 / 1.61). The real defect is that **blit does not scale**: 8x the workers buys it 1.05x (small) / 1.19x (mixed) where robocopy gets ~2.2x from 8 threads (4-arm interleaved session, `docs/bench/win-local-ab-2026-07-13/`). blit's per-file path is FINE; its parallelism is not, and users get one worker by default. Plan drafted (`docs/plan/LOCAL_SMALL_FILE_PATH.md`, Draft) with L3 (no scaling) as prime suspect and L1 (tar framing) DEMOTED — framing cannot be the dominant cost of a tool that wins at one thread. **A rig instability found and NOT papered over**: blit's absolute time is bi-stable across sessions (1388ms vs 2225ms, identical binary and flags, flat within each session) while robocopy /MT:8 reads 697ms in every session. Cause: an 8-thread neighbour leaves the CPU boosted and blit's single-worker, syscall-bound run inherits it — so **absolute times on this rig are only meaningful WITHIN a session**. The 4-arm interleaved design (every arm sharing identical neighbours) is the control; ratios held across both regimes (0.879 -> 0.911), which is what interleaving is for. **THE FINDING THAT OUTRANKS THE PERF WORK** (D-2026-07-13-3, `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`): codex's review of the harness asked whether blit was simply doing MORE work than robocopy — and the answer is that it does **LESS**. **blit silently discards Windows file attributes (ReadOnly/Hidden/System) and alternate data streams, exit code 0, on BOTH the local and the remote route** (both MEASURED — a loopback daemon on netwatch-01 proved the wire path loses them identically). And the loss is **conditional on FILE COUNT**: `transfer_plan.rs:103-109` tars when there are >=2 small files AND (>=32 of them OR average <=128 KiB); otherwise files go through `CopyFileExW`, which carries attributes and ADS for free. Proven with identical 200 KiB files where only the COUNT varied — **40 files: LOST. 3 files: PRESERVED.** So the same file keeps its metadata copied alone and loses it copied beside 39 siblings, which means essentially every real directory (source tree, photo folder, documents) takes the lossy path. mtime survives; Unix permissions survive (`#[cfg(unix)]`) — the gap is Windows-only. Empty directories are absent too, but that is a **documented non-goal** (`blit check`'s help, `cli.rs:20-35`), not this bug — and note `test_push_empty_directory` only asserts the command SUCCEEDS, never that the directory arrived: a crash smoke test wearing a fidelity test's name. Owner's framing and the correct one: **unlanded Windows support, not a regression** — blit began as a Linux alternative to robocopy and the metadata half never shipped. Queued behind otp-12, to be planned TOGETHER with the local-scaling plan (they pull in opposite directions: a fidelity fix ADDS per-file work to a path already losing to robocopy). **Fixing it is a WIRE CONTRACT change** — the tar shard IS the wire format — so `docs/TRANSFER_SESSION.md` gets amended through the loop BEFORE any code. **Also**: the MTU experiment was pre-registered BEFORE any data (`35b9620`) and codex returned NOT READY with 4 BLOCKER + 3 HIGH — **all 7 accepted** (`7921adc`). The one that stings: "`mixed` is the most packet-heavy fixture we test" — the stated rationale for the whole experiment, repeated in STATE — is **FALSE**. At MSS 1448 `large` is ~741k segments vs `mixed` ~378k; **`large` is packet-heaviest by ~2x**. Also killed: my masking guards admitted the exact artifact they existed to catch (a shared 1000ms floor passed all three), and my "r>=1.20 means MTU is not the cause" band contradicted the parent plan's own 20-50% CONTRIBUTING grade. Design now measures BOTH MTU conditions (9000 AND 1500, identical CELLS, RUNS=8, same NIC and sha) — a lone jumbo run attributes nothing. Instrument validated first this time: negotiated MSS on the rig-W path is **8948 both directions** (`getsockopt(TCP_MAXSEG)` + Linux `ss -ti`), vs 1448 at 1500 — a MEASURED 6.18x segment reduction. A candidate instrument was tested and DISCARDED: the Windows NIC counter reports 10680 bytes per received "packet", larger than a 9014-byte frame, so it coalesces and cannot discriminate 1500 from 9000. **And a false attribution corrected**: `GPT_REVIEW_LOOP.md` still named `gpt-5.5` as the reviewer, so I signed a verdict file with it — `~/.codex/config.toml` is ground truth and says `gpt-5.6-sol` at effort `ultra`. Both fixed; the loop doc now says READ the config rather than name a model that goes stale. Suite untouched (**1488**; zero `crates/`/`proto/` changes). Next: the MTU experiment (Queue 1a).

 **2026-07-13 21:00:00Z** - **HOUSEKEEPING (STATE cap prune, claude)**: `docs/STATE.md` Queue item 6 ("Post-REV4 residue", unowned) pruned out of STATE to hold the 200-line cap; the list is unchanged and lives here so STATE can point rather than restate: epoch-0/early-ADD hardening; remote perf-history lanes (the 1e gap); receive-side dial tuning residue (w3-1 scoped it out); the source send half's bounded `dp.queue()` is not raced against control-lane events (codex otp-7b-1 F3; residual: the narrow CANCELLED->INTERNAL decay); the CLI progress monitor lives through the in-session mirror purge (display-only; fix = the M-C `AppProgressEvent` phase reshape — codex otp-10b-2 F5).

 **2026-07-13 20:00:00Z** - **CODER (otp-12c closed; the same-OS rig built; the fleet moved to jumbo; and THREE of my claims retracted, claude)**: **Shipped**: otp-12c through the codex loop (rig D delegated parity **7/7 PASS** — delegation costs nothing; review 7/7 accepted incl. F2, where I had misread D2's escalation amendment as converge-only and thereby ducked a verdict). A real bug found en route: a peer `Frame::Error` arriving mid-record was reported as a ProtocolViolation about frame position instead of surfacing the peer's own fault — a CANCELLED must stay CANCELLED (`920c6a7`, plan D4; both file and tar-shard receivers now match the block-record handling). Plus the CLI `./NAME` foot-gun hint (`ace91de`) and the CI fmt fix (`bb28ddd` — I had skipped `cargo fmt --all -- --check`, the FIRST line of the three-command gate, and reported slices green on partial checks; full gate now clean, suite **1488**). Perf plan flipped **ACTIVE** (D-2026-07-13-1, owner: "just write the code and reviewloop slice by slice — that converges faster than plans with no ground truth to test"), after a 5th codex round whose 3 blockers were all real and all fixed. **Built a new rig**: magneto↔skippy, Linux BOTH ends — the only pair in the fleet that can measure blit's layout with zero platform terms (owner promoted magneto from "never a bench end" after confirming it saturates 10GbE where zoey cannot). Result under full methodology (cold caches both ends, drains, ABBA, pair-void, 64 runs, zero voided): **8/8 invariance cells PASS**, P1's own cell at **1.092/1.003**. So **P1 does not reproduce on a same-OS pair** — it is platform-INTERACTING, not a pure layout property. It is NOT exonerated (a code path that only bites on one platform looks identical), and per codex r5 F1 it has **no escape hatch**: D-2026-07-12-1 waives only a cross-direction miss for a cell that ALREADY passes invariance, and P1 *is* the invariance failure. Fix it, or the owner amends criterion 1. **THE LESSON OF THE DAY — three retractions, one root cause.** (1) I reported "P1 reproduces at 1.78 on Linux↔Linux, therefore it is CODE" — WRONG: my harness ran the durability `sync` inside the *initiating* host's bracket, so the push arm (initiator = source, which only read) never paid the destination's writeback while the pull arm (initiator = destination) paid all of it. One arm was billed for durability the other got free — multi-second on skippy's ZFS — manufacturing "failures" on every carrier and fixture *including the gRPC control that is supposed to be clean*. **The carrier-independence is what exposed it**: a real code effect is carrier-specific, an accounting artifact is not. Fixed `2c0af86` (durability keyed by DESTINATION, never verb — the otp-2w rule this repo already knew and I broke). (2) I told the owner twice that P1 was "probably acceptable platform residue" — codex proved no decision on the books permits it. (3) I diagnosed "macOS cannot send jumbo frames, the switch is broken," had the owner swap a network adapter, and was **wrong again**: `net.inet.raw.maxdgram` caps *raw sockets* at 8192 and `ping` uses one, so DF pings above ~8164 fail from a Mac regardless of the real path MTU — while TCP, which does not use raw sockets, was fine the whole time (verified: 231/225/157 MB/s at 9000). My "TCP is blackholed" test was itself broken (`more > NUL` is a pager; I never baselined it). **Every one of the three was the same failure: I trusted a measuring instrument without first proving it measures.** Each was caught from outside me — a suspicious gRPC control, codex, and an owner who refused to accept the answer. **The fleet is now uniformly MTU 9000** (zoey converted — UniFi UNAS, systemd-networkd, `[Link] MTUBytes=9000`, proven by a live `networkctl reconfigure` with the static IP intact; overlayfs means a firmware update can silently revert it). **Which unlocks the next experiment and it is a big one**: Windows sat at **1500 for EVERY benchmark ever recorded**, so jumbo has never once been exercised in a blit measurement. P1's failing cell is TCP × *mixed* — the most packet-heavy fixture we test — precisely where ~6× fewer packets could move the number. **Next: re-run rig-W invariance at jumbo BEFORE touching any code** (Queue 1a); it may dissolve P1 outright. Confound to control: the Mac's NIC also changed (Aquantia @ .54, was the TB5 dock @ .91 — so 1.237→1.300 is NOT evidence of a code regression), so if the asymmetry vanishes, re-run at 1500 on the same adapter to separate MTU from hardware. Also recorded: `.agents/repo-guidance.md` had the remotes **inverted** (origin = gitea, NOT GitHub) — every agent push this session went to the mirror while GitHub sat dozens of commits stale until the owner pushed by hand.


 **2026-07-13 04:45:00Z** - **CODER (otp-12c CLOSED through the codex loop — 7/7 accepted, incl. the one finding that was against me, claude)**: Owner: "1. reviewloop codex / 2. push after 1." Note on invocation: `/playbook reviewloop codex` names `.agents/playbooks/reviewloop codex.md`, which does not exist; `.agents/playbooks/reviewloop.md` does, but it is a PORTABLE TEMPLATE assuming `.agents/state.md` + `.agents/review/index.md` + **per-finding branches** — all of which collide with this repo's actual layout and its "no agent-created branches" earned practice. Its own text and `.agents/repo-guidance.md` both defer to the repo: the canonical procedure for review-loop work here is `docs/agent/GPT_REVIEW_LOOP.md`, which is what I ran (flagged rather than silently substituted). **Codex (gpt-5.6-sol, 202k, range `dcbd6ea..9350b24`): FAIL → 7 findings, 7/7 ACCEPTED, 0 rejected.** It independently CONFIRMED the parts that mattered most: the otp-12b F5 arm asymmetry does **not** recur (both arms pass contents-form sources, land the identical tree, pay the same in-window destination mkdir — the exact confound I'd flagged for it to attack), every committed CSV recomputes exactly, both READMEs otherwise reserve acceptance to otp-13, and the range touches no `crates/`/`proto/`/Cargo files (so 1484 stands by construction, verified not re-run). **F2 (HIGH) is the finding that matters and it was against me**: my evidence README, findings doc and STATE all said the RUNS=4 FAILs and the RUNS=8 PASSes "neither supersedes the other — owner call", on the theory that D2's escalation-supersession amendment covered only the converge-up rows. **Wrong.** The escalation rule and its amendment sit in D2 *after all four bar definitions* and speak of "**a comparison**" generically — delegated parity included — and both FAIL cells met the trigger exactly as pre-registered (straddle 1.119/1.129 vs 1.10, arm spread 86.0%/55.4% > 25%). So the RUNS=8 re-run I ran *was* the pre-registered mechanism firing, and its medians **govern**: **rig D is 7/7 PASS** (5 at RUNS=4 + 2 escalated). Re-reading a pre-registered rule *after seeing the numbers*, in a direction that conveniently let the slice duck a verdict, is precisely what pre-registration exists to prevent — corrected in all three places. **F1 (HIGH) cold-cache fails open**: the sudo grant only WARNED and every per-run purge ended `|| true` — a silently-failed purge produced a WARM run that still counted valid (the same class as the `a2dea3f` no-op). Now a hard preflight gate (`COLD_REQUIRED=0` opts out) + per-run cold outcome recorded and a failure VOIDS the pair. **Checked rather than assumed whether this corrupts the recorded session: it does not** — the grant was probed live (`sudo -n -l` shows the NOPASSWD tee entry; a direct drop returned DROP_OK), the session ran at `a2dea3f` which uses exactly that grant, and the recorded drain outcomes vary run-to-run (6s/8s/4x2s/9x2s), which a no-op purge could not produce. Latent fail-open, not corrupted data. **F3** provenance: `proto/` was missing from the dirty-tree gate (blit-core compiles `proto/blit.proto` INTO the binaries) and `grep "+<sha>"` substring-matches `+<sha>.dirty.<hash>` — the exact shape that fooled otp-12a on zoey; both closed (`embeds_clean_{local,skippy,win}`). **F4** the manifest and every `runs.csv` row recorded harness HEAD (`a2dea3f`) as the build identity while the gated+hashed binaries embed `f35702a` — machine-readable fields now carry the gated identity, HEAD recorded separately. **F5** a failed skippy `sync` was masked by the succeeding `echo` (plausible flush on unflushed bytes → valid run) and a disk regex matching NO device reported `drained` — both fail closed now (NA→void; DRAIN-NODEV). **F6** both teardowns suppressed every failure, cleared their flags and logged "stopped" — the harness could exit 0 with a daemon still holding 9031; now verified, loud, and non-zero on a leak. **F7** the README listed `sw_tcp_small` (93.6% spread) among the FAILs though it PASSED at 1.034. Fixes `0fb4a64` (+ cap trim `4cc9b6e`); verdict `.review/results/otp-12c.gpt-verdict.md`; REVIEW row `[x]`. Gate: `bash -n` OK, `check-docs` OK, suite untouched at **1484**. The harness fixes bind the NEXT session — the recorded evidence stands on the harness as it ran. Next: **otp-12d assembly** → otp-13 owner walk.

 **2026-07-13 03:30:00Z** - **CODER (otp-12c RECORDED: the delegated rig-D session + a direct-path re-baseline at the cutover sha; NOT yet through the codex loop, claude)**: Owner "proceed with all of the testing without waiting on my go". **Rig prep** (netwatch-01↔skippy): skippy's July binaries were REV4-era (D-2026-07-05-2 refuses them at session open) → fresh `x86_64-unknown-linux-musl` zigbuild staged to the pool (`/mnt/generic-pool/video/blit-bin/`, old ones kept as `*.rev4-jul04`); the three fixture trees pulled onto skippy's `cmp/` module over the smoked TCP path (shape-verified: 1/1073741824, 10000/40960000, 5001/547110912 — byte-identical, large-file sha256 matches netwatch's); `[delegation] allow_delegated_pull` + IP allowlists written both ends (the gate is IP/CIDR only — hostnames fail the SSRF rule). **Rig foot-gun recorded**: Win32-OpenSSH REAPS `Start-Process` children when the spawning ssh session closes — the netwatch daemon died silently between turns and 9031 stealth-dropped, mimicking a firewall block (rules were never the problem; the smoke *is* the firewall test because the data plane binds ephemeral ports). WMI `Invoke-CimMethod Win32_Process Create` is the only durable launch. **Direct-path re-baseline** (`d12534d`, `docs/bench/otp12c-win-2026-07-13/`): the whole rig-W matrix re-run with the new arm at the CUTOVER sha `f35702a` (12b measured `e21cf84`), old arm `0f922de` — 198 timed runs, 24/24 cells, 3 DRAIN-TIMEOUT pairs voided+re-run, 0 CR residue (the `856af64` strip-at-source held); **93 PASS / 12 FAIL / 3 FAIL-SAME-SESSION / 12 RECORDED**; `wm_tcp_mixed` invariance **1.300** (12b: 1.237) — the TCP×mixed×dest-initiator cell did NOT wash out at the cutover sha; losses stay in TCP×{small,mixed}×push + `pull_tcp_mixed` while the new arm WINS the small-pull side (1301 vs 1480, 1479 vs 1663, 2264 vs 2656). **The delegated session** (`docs/bench/otp12c-delegated-2026-07-13/`, harness `scripts/bench_otp12_delegated.sh` @ `a2dea3f`, binaries `EXPECT_SHA=f35702a`, 5-hash manifest identical across both sessions): plan D4 parity = delegated (Mac CLI triggers DelegatedPull; no payload through the Mac) vs direct (the destination host's own CLI pulls), same bytes/data-plane/destination-disk/flush; 7 cells (`sw_*`/`ws_*` × {large,small,mixed} TCP + `sw_grpc_large` secondary). **Primary RUNS=4: 56 runs, 7/7 cells, 0 voided → 5 PASS / 2 FAIL** (`sw_tcp_mixed` 1.119, `ws_tcp_large` 1.129). **Confirmation RUNS=8 on exactly those two: 32 runs, 0 voided → both PASS** (1.035, 1.068) — and at n=8 the big spread appears on the DIRECT arm too (31.5%/64.0% vs the primary's delegated 86.0%/55.4%), so the 4-pair FAILs read as low-n noise rather than delegation cost; `ws_tcp_large`'s primary delegated BEST (3000 ms) already beat its direct best (3870 ms). Both records committed; **neither supersedes the other — the 4-vs-8 reading is the owner's call at otp-13** (the D2 escalation amendment says RUNS=8 governs, but this harness's rows are not the same shape, so the README self-adjudicates nothing). **Three harness bugs found live, each caught by the script's own gates**: `:?` default messages containing apostrophes silently swallowed the next 20 assignments (the exact otp-12b `772cfe6` bug, re-made) → `b49413d`; macOS `$TMPDIR` blows ssh's 104-byte ControlPath socket limit → `/tmp` mux dir, same commit; skippy's NOPASSWD grant is exactly `tee /proc/sys/vm/drop_caches`, so `sudo -n sh -c '…'` silently no-op'd the cache drop (runs would have read WARM) → probe+invoke via the exact grant, `a2dea3f`. **Owed: the codex loop** (D-2026-07-04-1) on the harness + both evidence commits — this slice is recorded, not closed. Next: **otp-12d assembly**, then the otp-13 owner walk. Suite untouched at **1484** (zero crates/proto changes).

 **2026-07-12 19:20:00Z** - **CODER (otp-12b CLOSED both halves: the Mac↔Windows acceptance session — THE INVARIANCE CRITERION MEASURED, claude)**: Rig facts: the box is `michael@netwatch-01` at **10.1.10.177** (recorded .173 stale — DHCP; machines.md + harness defaults corrected). Staging: old `0f922de` exes aside-copied BEFORE the checkout move; bundle → detached `e21cf84`; native build 15–21 s; sha-named bins; 7-hash manifest; `OLD_CLIENT_PROVENANCE_BY_BUILD=1`. **Two found-live harness bugs, both caught by the session's own guards**: (1) pwsh parses bare `$rc:R` as a SCOPE-qualified variable → the win-initiated sentinel never printed → the win→mac smoke failed fast rc=99 exactly as designed; manual repro proved the path itself fine (no macOS-firewall issue); `${rc}` fix `e21cf84`. (2) pwsh CRLF in the drain outcome split every runs.csv row under python universal newlines → 192 valid runs verdicted INCOMPLETE; CR strip at source `856af64`, recorded session CR-sanitized post-hoc with the raw file committed (no timing value altered). **The session** (`44c2046` + review fixes `49dee5c`, `docs/bench/otp12-win-2026-07-12/`, 192 runs, zero voided, zero drain anomalies): **Block 2 invariance — the owner's sentence measured: 11/12 PASS at 1.003–1.057**; the exception `wm_tcp_mixed` **1.237** (mac_init 1127 vs win_init 911, tight spreads): Win→Mac mixed over the TCP data plane is ~25% slower when the MAC initiates (pull-verb/destination role) — corroborated independently by block-1 (`pull_tcp_mixed` 1.313 vs today's old arm), absent on grpc (1.013) and other fixtures — a CODE-SHAPED finding for the walk (TCP×mixed×destination-initiator signature). Block 1 converge: 10/12; `push_tcp_small` **1.149 FAIL-BOTH** at 3–4% spreads — the zoey 1.105 gap CONFIRMED ON A SECOND RIG (real small-push session cost); `pull_tcp_mixed` FAIL-SAME-SESSION (same root as the invariance fail). Cross: **Win→Mac 6/6 beat even the better committed old direction (0.760–0.990)**; Mac→Win 6/6 miss it with gap shapes RECORDED per D-2026-07-12-1 (large unchanged 1.98→1.95 = the pre-ruled platform shape; mixed 1.946→1.408 + grpc_small 1.929→1.644 NARROWED = code improvement with residue for the owner; tcp_small WIDENED 1.332→1.527 = the small-push code component). Rig drift note: today's old arms run far faster than the 07-10 committed medians (old push_tcp_large 1908 vs 3054) — committed bars easy, same-session bars binding. No pre-registered escalation trigger fires (tight spreads, no straddles) — results stand. **Codex run-round (gpt-5.6-sol, 131k): FAIL → 3/3 accepted+fixed (`49dee5c`)** — F1 (High) the README self-adjudicated the D-2026-07-12-1 attribution (narrowed-gap cells are the owner's call; prose now records shapes only); F2 cross-row range 0.760–0.990; F3 comment count. Earlier same slice: harness round 12/12 accepted (`d3eae58`). Verdicts `.review/results/otp-12b{,-run}.gpt-verdict.md`. REVIEW row `[x]`; STATE current → **otp-12c (delegated, netwatch-01↔skippy — needs fresh skippy staging: x86_64-musl zigbuild to the pool, noexec /tmp)**, then 12d assembly, otp-13 owner walk. Suite untouched at **1484**.

 **2026-07-12 18:15:00Z** - **CODER (otp-12b harness half through the codex loop; magneto/VM recorded build-only, claude)**: Owner host facts recorded (`d5fd17f`+`1fd50d7`: magneto = Arch x86_64 10GbE but BUSY BitTorrent box — build-only fallback, never a bench end; Ubuntu ARM VM likewise; zigbuild covers every Linux target anyway → **otp-12c stays Windows↔skippy as designed**). **otp-12b harness** `d30b1e3` (+ quote-parity fix `772cfe6` — an apostrophe inside the `MAC_HOST :?` message broke bash parsing 323 lines downstream; the gate chain misreported under shell nesting, so `bash -n` is now its own verified step): `scripts/bench_otp12_win.sh` — block 1 = the otp-2w matrix as interleaved old(`0f922de`)/new matched pairs (both-reference verdicts); block 2 = **the plan's headline initiator/verb-invariance cells** (`mw_*`/`wm_*`, mac_init vs win_init ABBA, new pair only) + per-arm converge rows (F3) + F4 cross rows + D-2026-07-12-1 discriminator gap rows (`RECORDED`, never self-adjudicated); Windows plumbing verbatim from the frozen otp-2w script (WMI launch, stale-refusal, TOML literal paths, Get-Counter drain, standby purge); arm swap via a fixed active exe path (ONE program-scoped firewall rule); Mac daemon serves `$MAC_WORK` itself (design F6); win-initiated windows self-timed ON Windows (Stopwatch-in-one-ssh printing sentinel-framed `ms,exit`); win→mac smoke gates the macOS-firewall unknown. **Codex (gpt-5.6-sol, 109k): FAIL → 12/12 accepted+fixed (`d3eae58`), zero rejected**: F1 manifest-hash fail-closed regression (the 12a lesson re-applied); F2 arm swap now hash-verifies the active exe before launch (old arm has no handshake); F3 the arm was missing from destination paths — cross-arm no-op collision possible (the sharpest catch); F4 derived verdicts gated on complete(); F5 invariance arms made byte-identical (precreated containers outside the window, uniform nesting); F6 MAC_MODULE_ROOT hardcoded; F7 sentinel-framed fail-closed Windows timing (flush NA voids; parse failure = rc 99); F8 mandatory references + registered `NO-SAME-SESSION-REF`; F9 WMI cmd-pid recorded immediately + daemon resolved by ParentProcessId (no untracked window); F10 CELLS header false-match; F11 firewall rule verified not name-trusted; F12 vocabulary closed + gap-row labels exact. Verdict `.review/results/otp-12b.gpt-verdict.md`. Suite untouched at **1484**. Next: **the otp-12b RIG SESSION** — fresh bundle → Windows box, copy the `0f922de` detached-build exes aside FIRST, checkout the run commit + native build (daemon + blit.exe), stage sha-named bins, MAC_HOST export, PREFLIGHT_ONLY, then both blocks (~2-3.5 h).

 **2026-07-12 16:30:00Z** - **CODER (otp-12a CLOSED both halves: the zoey rig run recorded through the codex loop, claude)**: Owner rig go ("any of the previously used hosts") → the full zoey session, three acts. **(1) Provenance find**: the new preflight caught that zoey's staged "e757dcc" daemon actually embeds `731023bfc8a1.dirty.…` — the otp-2 README's "binaries both ends" claim was wrong on the daemon end (committed daemon code identical between the commits; dirt unknowable) → correction committed (`b2b6901`), both arms rebuilt CLEAN (old `e757dcc` worktree client + fresh musl daemon staged as `blit-daemon-e757dcc`; original artifact untouched); also live-fixed: BSD grep needs `-a` for binary matches (`b3729da`). **(2) The storm**: first matrix attempt drove zoey to load 444 with 10× run times BOTH arms — aborted; accumulated per-run push destinations implicated (probes with per-run deletion held at baseline 2466/2525/3714 vs committed 2702) → harness sweeps each dest after its measured flush (`042c06f`); CELLS escalation filter (`6bc9cb6`). **(3) The evidence** (`b0ebf73`, `docs/bench/otp12-zoey-2026-07-12/`, 3 sessions incl. the aborted storm): main RUNS=4 matrix 48 pairs none voided → 9 PASS + 3 escalated per D2; RUNS=8 escalation settles them — push_tcp_large **PASS** (0.959/0.912 — the RUNS=4 FAIL-BOTH was noise), pull_tcp_large **FAIL-REFERENCE-DRIFT** persisting (new-vs-old same-session **0.995**; the old arm itself runs 1.248× its own committed baseline — rig-side by strongest evidence, confound named), push_tcp_small **FAIL-SAME-SESSION 1.105** (stable across sessions, tight spreads — the one real marginal gap; still 0.935 vs the committed baseline) — both non-PASS cells carried to the otp-13 walk; the README declares nothing. **Codex run-round (gpt-5.6-sol, 144k): FAIL → 6/6 accepted+fixed (`fa18787`)** — numerics explicitly confirmed ("recomputed table numerically exact"); F1 provenance grep false-positived on a cargo build-dir path (now `+<sha>` form + explicit by-build acknowledgment for pre-cutover clients that embed no id); F2 the D2 escalation supersession rule was never pre-registered (now a dated amendment: RUNS=8 governs, RUNS=4 rows stay visible) + a wrong best-run claim corrected; F3 "provably rig-side" reframed correlation+parity with the dirty-reference confound named; F4 marginal-gap wording restated per CSVs; F5 mistyped CELLS dies; F6 stale header. Verdicts `.review/results/otp-12a-run.gpt-verdict.md` (+ the harness round earlier). REVIEW.md: otp-12a row `[x]`, stale otp-2 row closed (owner go). STATE: otp-12a recorded; **next otp-12b (Mac↔Windows: converge-up + cross-direction + initiator/verb invariance)**, then 12c delegated, 12d assembly, otp-13 owner walk. Suite untouched at **1484** (zero crates/proto changes across all of otp-12).

 **2026-07-12 05:30:00Z** - **CODER (otp-12a harness half: zoey converge-up A/B through the codex loop; design flipped Active, claude)**: Owner "yes to both" → `docs/plan/OTP12_ACCEPTANCE_RUN.md` **Draft → Active** (`260fb26`; approval recorded in the Status line — the doc's only open question was already ruled D-2026-07-12-1) and the stale REVIEW.md otp-2 row closed (`ae498d3` — it had sat `[ ]` "PENDING RIG ACCESS" since before the 2026-07-10 closes). **otp-12a harness** `8f4fbf9`: `scripts/bench_otp12_zoey.sh` — the otp-2 matrix as matched-pair interleaved old(`e757dcc`)/new A/B, ABBA counterbalance, pair-void valid-run rule (2×RUNS cap, INCOMPLETE surfaced), exit codes captured, stale-daemon refusal, sha256 staging manifest, PREFLIGHT_ONLY mode, python3 summary+verdicts against BOTH references (same-session old AND the committed otp-2 medians; D2 vocabulary); runs.csv schema +`valid` (doc amended). Frozen `bench_otp2_baseline.sh` untouched (D5). Gate: `bash -n` (shellcheck not installed — recorded); no crates/proto changes anywhere in otp-12, suite stands at the recorded **1484** (`git diff ce36da3..HEAD -- crates proto` empty; fresh fmt+clippy run green). **Codex (gpt-5.6-sol, 107k): REQUEST CHANGES → 9 findings, 9/9 accepted+fixed (`50dc135`), zero false positives**: F1 zero-valid cells vanished from verdicts + short medians (verdicts now iterate every attempted comparison; summary only for complete cells); F2 fixtures/pull-sources were existence-trusted (now count+byte-sum verified, converging re-stage); F3 provenance unenforced — stale-but-matching pairs pass the handshake (embedded-sha grep on all four binaries; dirty tree fatal; hash capture fails closed; reference hashed); F4 EXIT trap could kill an unowned recycled PID (session-gated + comm-verified); F5 reference silently overrideable + unregistered vocabulary (hardcoded; fail-closed; vocabulary registered in D5); F6 timed window wrote per-run logs (stdout→/dev/null, stderr-only); F7 reused pull dest (never-seen per-run paths, swept); F8 RUNS unrestricted (4|8 only); F9 doc cell grammar order. Verdict `.review/results/otp-12a.gpt-verdict.md`. Next: **the otp-12a RIG RUN** (blocked on: owner's fresh go for zoey daemon runs + zoey out of maintenance + staging: old client worktree rebuild, new musl daemon zigbuild) — then otp-12b (Windows harness).

 **2026-07-12 03:57:00Z** - **CODER (otp-12 design: acceptance-run matrix + harness plan through the codex loop, claude)**: `docs/plan/OTP12_ACCEPTANCE_RUN.md` (Draft) at `045da4a`; owner ruled Q1 in-session → **D-2026-07-12-1** (`bfb9670`: a rig-W cell that beats its own old direction and is initiator-invariant but misses `min(old_push, old_pull)×1.10` only by a discriminator-attributed destination write-path residue COUNTS as satisfying criterion 2's cross-direction half; both parent criteria annotated in place). Design: rig Z (Mac↔zoey) = per-direction converge-up via interleaved matched-pair old(`e757dcc`)/new A/B; rig W (Mac↔Windows, owner-designated closest-spec) = converge-up + cross-direction + the initiator/verb invariance matrix (first-of-kind reverse-initiator arms: Mac daemon, Windows client, self-timed remote windows, flush keyed by destination OS never verb); rig D (Windows↔skippy) = delegated-vs-direct parity, new-build only (July skippy binaries are pre-`Transfer`, unusable — fresh x86_64-musl staging on the pool); sub-slices 12a–12d; verdict arithmetic pre-registered; otp-12 declares nothing (otp-13 owner walk). **Codex (gpt-5.6-sol, 125k): CHANGES REQUIRED → 7 findings, 6 accepted+fixed (`92e1d51`) + F1 overtaken by the owner decision** (the exact concern was surfaced in chat before the review returned; residual criterion-1 instantiation note applied): F2 converge-up needs BOTH references (same-session old AND the committed 2026-07-10 medians; `FAIL-REFERENCE-DRIFT` + one pre-registered re-run); F3 every unified arm meets the bars independently (tolerance compounding to 1.21× killed); F4 acceptance-checkbox flips moved out of 12d to otp-13; F5 ABBA counterbalanced interleave (arm never confounds with within-pair order); F6 Mac module root = `$MAC_WORK` itself (both initiators read the same physical inodes); F7 nonzero-exit/undrained runs void their PAIR and re-run to RUNS valid pairs (capped, `INCOMPLETE` surfaced). Verdict `.review/results/otp-12-design.gpt-verdict.md`. Next: **otp-12a** (zoey harness + run — needs the standing fresh owner go for zoey daemon runs + zoey out of maintenance). Noted in-session: REVIEW.md's otp-2 row still reads `[ ]` PENDING RIG ACCESS though otp-2 closed 2026-07-10 (stale, unfixed — offered to the owner).

 **2026-07-12 08:00:00Z** - **CODER (otp-11b: THE LOCAL ORCHESTRATION DELETION — otp-11 CLOSED, every transfer on the one session, claude)**: slice `805e48c` + docs `b1650c4` + review fixes `9e810ee` (−6,197/+1,532). The last old path is out of the tree: `orchestrator/` (16 tests), `engine/` (strategy/streaming_plan/tuning/history/journal/single_file/mirror/options/summary; **`dial.rs` RELOCATED VERBATIM → `src/dial.rs`** — codex verified blob-identical, 17 tests carry), `local_worker.rs`, `auto_tune/` (absorbs the STATE residue "derive_local_plan_tuning fold-or-retire": retired), `change_journal/` (the UNSOUND journal skip — the addendum's data-loss repro; its `objc2-core-services` dep died with it), `copy/parallel.rs`+`stats.rs`, `lib.rs::CopyConfig`; the **otp-10c-2 F2 deferred `compare_manifests` sweep** (aggregate + `ManifestDiff`/`FileComparison`/`include_deletions` die; `header_transfer_status` survives as the one compare owner — 13 of its 16 tests converted to direct pins); stranded `plan_local_mirror`/`LocalDiffInputs`/`filter_unchanged` (14 tests → 12 converted: 9 direct `file_needs_copy_with_mode` pins — the sink defense layer — + 3 `plan_transfer_payloads` pins); `LocalMirrorOptions`/`Summary`/`TransferOutcome` re-homed → `transfer_session/local.rs` (unreachable engine-era axes dropped; `JournalSkip` + `PredictorEstimate` retired; CLI arms deleted); frontends re-import via `blit_core::transfer_session`; `docs/TRANSFER_SESSION.md` §Transport gains the precise LOCAL-carrier contract note (the design-round F1 commitment). **Test floor: suite 1513 → 1484 — the otp-13 ≥1483 criterion MET AT THE DELETION SLICE, margin +1, by real pins** (died-in-modules 41 + deleted integration files 10 + retired-with-surface 5; 25 conversions in place; +27 new: 15 session-route e2es incl. empty-source mirror full-delete + nested split counts + checksum/ignore-times/ignore-existing/size-only cells + resume fresh-dest + unreadable-subdir + planner-mix/scanned-bytes accounting, 7 unit pins incl. the `build_local_record` contract trio (R44-F1 split carried forward) + the `DestSubtreeExcludedSource` stream pin + the **streaming-overlap port** (`first_apply_lands_before_enumeration_completes`, the gated-source port of the engine's overlap pin — the 11a-F2 commitment), 2 `mirror_delete_pass` unit pins (plan-only; split), 2 manifest arm pins, 1 sink file-root pin). Guard proof: SizeOnly arm mutated → converted pin FAILS → restored. **Codex (gpt-5.6-sol, 203k): CHANGES REQUESTED → the core CONFIRMED** ("deletion, re-homes, converted coverage, remote-session behavior, one-transfer-path structure, and the 1484-pass suite check out") **+ 6 findings, 6/6 accepted+fixed** (`9e810ee`): B1 STATE staleness at the pinned commit (records-commit cadence, now landed); B2 live-doc sweep completed — WHITEPAPER local→local row now maps to `run_local_session`, §5 tuning marked HISTORICAL, bug-surface auto_tune/change_journal items struck dated, ARCHITECTURE diagram cells replaced, `.agents/repo-guidance.md` Style exemplar + Project Map updated, `diff_planner.rs` module doc rewritten; B3 predictor promises retyped (TUI F4 "populate the history", `blit profile` states the training retirement, `--null` help describes the history lane); B4 the throughput line prints the EFFECTIVE apply worker count (1 unless the debug limiter); B5 the accounting equation corrected (1513−41−10−5+27=1484; conversions don't appear in the delta); B6 dial = 17 tests. Verdict `.review/results/otp-11b.gpt-verdict.md`. **otp-11 (a + addendum + b) is CLOSED: local transfers ride the session, the separate local orchestration does not exist, the perf gate passed against sound baselines, and the deletion-proof acceptance line completes.** Next: **otp-12** (symmetric-rig acceptance run — rig availability in Blocked) then **otp-13** (owner checklist walk).

 **2026-07-12 05:00:00Z** - **CODER (otp-11a addendum review closed — codex CONFIRMED the unsoundness verdict, claude)**: `d74c1ac` reviewed (after one quota-blocked dispatch), fixes `5f1188c`. **Codex (gpt-5.6-sol): CHANGES REQUESTED → the core claims all CONFIRMED independently** — "the data-loss claim is confirmed, no later validation catches it, Windows's changed-USN mtime fallback is also unsound, and the regression pin would fail under reintroduction" — with 4 record-quality findings, 4/4 accepted+fixed: **A1 (Med)** the 610 ms sound baseline was a single observation — re-certified by 5-run interleaved medians with the old binary's `journal_cache.json` removed before each run (probe → Unknown → its sound full pass): old **507 ms** vs session **226 ms** = **2.2× — gate PASS on real medians**; **A2 (Med)** STATE's summary line still said 11b-blocked/1512 — fixed; **A3 (Low)** floor arithmetic redone from 1513 (−71 retirements = 1442 → ≈+41 pins to the ≥1483 floor); **A4 (Low)** Linux mechanism precision — no global event counter there: `compare_linux`'s FIRST arm is the root dir's CTIME, which a deep write equally never touches (macOS: event-id arm always differs → root-mtime fallback decides; Windows: strict-USN arm needs a write-quiet volume, decays to the same fallback) — README + pin comment retyped, conclusion unchanged. Verdict `.review/results/otp-11a-addendum.gpt-verdict.md`. **otp-11a is now fully closed including the addendum round. Next: otp-11b** — dial re-home, option/summary type re-home, the deletion (orchestrator/, engine/ remainder, local_worker, auto_tune, change_journal, copy/parallel+stats, CopyConfig), the compare_manifests sweep, stranded dead code, deletion proof + retirement accounting (≈+41 pins), Windows parity.

 **2026-07-12 03:30:00Z** - **CODER (otp-11a addendum: the change-journal question RESOLVED — the old fast path was unsound, claude)**: Owner rejected both framings of the no-op regression ("one violates FAST, the other violates SIMPLE — figure out a real fix"). The investigation found the premise false: the old engine's journal skip trusted `ChangeState::NoChanges`, whose macOS/Linux verdict (`change_journal/snapshot.rs compare_macos/compare_linux`) decays to ROOT-directory mtime equality whenever the global FSEvents event id / root ctime moved — which is every run on a live system — and a write to `src/sub/deep.txt` never touches the root dir's mtime. **Reproduced against the pre-otp-11 binary** (`d2bd843` release build): warm the journal with 3 mirror runs, modify a deep file, mirror again → "Filesystem journal fast-path: source/destination unchanged... Up to date" with the destination still holding the OLD content — silent data loss, transcript recorded in `docs/bench/otp11-local-2026-07-11/README.md`. Verdict: the 21 ms steady-state was fraudulent FAST (violates RELIABLE); the honest sound-vs-sound baseline is the old path's full pass (610 ms on 10k files) vs the session's 219 ms — **the session route is 2.8× faster than every sound no-op in the tree; the noop gate cell PASSES against the honest baseline; otp-11b is UNBLOCKED and its `change_journal/` deletion now removes a data-loss bug**. New regression pin `deep_modification_after_warm_runs_syncs` (warm repeated mirrors + deep modification MUST land — the exact lost shape). The principled O(changes) tier is filed as designed future work in the slice doc D3: journal-assisted no-op done SOUNDLY (Windows USN range replay; macOS FSEvents historical replay honoring must-rescan; fail-open) as a negotiated SESSION phase — both roles, both carriers, remote no-ops skip manifest streaming too; new wire surface, own reviewed slice set. Suite 1512 → **1513**.

 **2026-07-12 01:30:00Z** - **CODER (otp-11a: local transfers ride the session — the local route, claude)**: design `0da65d6`+`c7b463b`, slice `dfdddd6` + review fixes `e445e8d`, bench `631255b`. otp-11 staged 11a (route) / 11b (deletion) via a slice-design doc (`docs/plan/OTP11_LOCAL_SESSION.md`, D1–D3 — itself codex-reviewed as a plan change). The route: `run_local_session` joins `run_source`+`run_destination` over `in_process_pair()` (the otp-3 wiring); the LOCAL byte-carrier is a process-local `LocalApply` extension on `DestinationSessionConfig` — crate-private, NO wire representation, so no peer can ever select it — under which the destination plans needed headers (`plan_transfer_payloads`) and applies them in-process through `FsTransferSink` (clonefile/block-clone/copy_file_range preserved; `execute_sink_pipeline_streaming` stays alive as the apply pipeline); no NeedBatch is sent, nothing enters `outstanding`, a payload record still violates. `blit_app transfers/local.rs` (the one chokepoint, CLI+TUI) re-pointed; `LocalMirrorOptions` in / `LocalMirrorSummary` out (synthesized destination-side; outcome replicates the old strategy-gate reachability); mirror = the in-session delete rule with the user's FileFilter threaded process-locally, plan-only under dry-run, split (files,dirs) counts (the old `apply_mirror_deletions` tuple restored); move's IgnoreTimes mapping + caller-side unreadable gate untouched and now load-bearing (all 3 otp-10b-2 F3 regression pins green). Slice also fixed a latent sink bug the ported single-file pin exposed: file-root File payloads (empty rel path) hit trailing-slash ENOTDIR joins — `copy_root_file_payload` routes the identity case. **Design codex (gpt-5.6-sol, 241k): CHANGES REQUIRED → 10 findings adjudicated** (`.review/results/otp-11-design.gpt-verdict.md`): 3 already independently fixed in the slice (empty-rel sink, dest-subtree exclusion wrapper, mirror split/plan-only); doc amended (D1's "unchanged choreography" overclaim → the carrier delta stated precisely; symlink parity scoped to reachable options; local resume framed as the carrier's sink-level block phase + new pin; floor arithmetic redone honestly — 11b retires 71, needs ≈+44 real pins by otp-13). **Slice codex (gpt-5.6-sol, 267k): FAIL → 9 findings: 7 accepted+fixed** (`e445e8d`) **, 1 doc defect, 1 rejected**: ONE diff core extracted (`diff_chunk_verdicts` — both carriers, dispatch-only difference); apply pipeline aborts on drop (no writes behind a returned operation); **apply-time unreadable mirror guard** — verified the old engine refused mirror deletions on ANY unreadable entry incl. apply-time (engine/mod.rs R46-F2), the session's local carrier now carries that exact posture, pinned deterministically with a vanishing-source stub (mode-000 fixtures are caught at scan time) and guard-proven; `--workers` maps to apply-pipeline width under debug_mode (default 1 = the old shape); `--null` keeps its `null_sink` RunKind tag; bench harness hardened against swallowed failures; finding-doc evidence citation fixed. Rejected: "immediate-start regression" — diff batching is session-uniform (`DEST_DIFF_CHUNK`, both carriers since otp-4); making local eager-er would recreate per-topology drift; overlap pin ports at 11b. **Perf gate: huge/tree/small PASS both runs (1 GiB single file = 22 ms BOTH binaries — the clone survives; the cell byte-relay would have lost by orders of magnitude); the focused noop10k cell surfaced the real cost of retiring the change journal (D3): warm-journal old ~21 ms vs session full-diff ~219 ms on 10k files (the session BEATS the old non-journal pass, 610 ms). OWNER QUESTION — blocks 11b per the slice doc's gate rule** (options in STATE). 4 mutation guard proofs this slice (dest-subtree bypass, dry-run execute, split swap, apply-time guard). Suite 1488 → 1510 → **1512**/0 (2 ignored), fmt/clippy clean; two mid-session full-suite "failures" were dirty-tree BUILD_MISMATCH sampling artifacts (D-2026-07-05-2 refusing correctly across mid-edit builds), converged by a consistent clean rebuild. Verdict `.review/results/otp-11a.gpt-verdict.md`. Next: owner call on the journal question, then **otp-11b** — dial re-home, the deletion, compare_manifests sweep, deletion proof + retirement accounting (≈+44 pins).

 **2026-07-11 21:30:00Z** - **CODER (otp-10c-2: THE CUTOVER DELETION — otp-10c CLOSED, one transfer path by construction, claude)**: `7aac28b` + review fixes `995e1cc`. The plan's invariant is now structural: the four per-direction drivers (`remote/pull.rs`, `remote/push/`, daemon `service/push/`, `service/pull_sync.rs`), `rpc Push` + `rpc PullSync` + 13 exclusively-theirs messages (incl. `DataTransferNegotiation`, `PushSummary`/`PullSummary`, `PullSyncAck`, `metadata_only` — the relay scan flag whose caller died at 10c-1), the two wire-specific gRPC fallback sinks + `grpc_fallback.rs` (the session's in-stream carrier pins the same 1 MiB frame unit via `IN_STREAM_CHUNK`), and the dead dispatch/purge/util helpers are OUT of tree and proto — no bridge (D-2026-07-05-2); −13,771 lines. Relocated verbatim: the delegated spec builder (`DelegatedSpecOptions`/`delegated_spec_from_options` → operation_spec.rs; delegated trigger = its only consumer) and `FsTransferSource`'s fs-scan helpers (→ source.rs). The four A/B parity pins became ABSOLUTE tree+exact-count pins (the perf half lives in the committed otp-2/otp-2w baselines + otp-12's interleaved runs on zoey's pinned `e757dcc` binaries); dispatcher/updater pins re-pointed to the session variants; the DelegatedPull no-payload-bytes proof recorded (trigger-only request + progress-only oneof + the `cli_data_plane_outbound_bytes == 0` pin with its 10c-1 positive control). File-by-file deletion proof + all 106 test retirements enumerated in `.review/findings/otp-10c-2-driver-deletion.md`. Two full-run hiccups, neither this slice's code: the otp-3-reviewed dirty-tree BUILD_MISMATCH sampling window (converged by resample) and one w9-3-class daemon-spawn flake (3/3 green isolated). **Codex (gpt-5.6-sol, 282k tokens): NEEDS FIXES → 6/6 accepted, F6 owner-gated** (`995e1cc`) — and it independently confirmed the relocations verbatim, the no-payload proof, the A/B conversions, and the session invariants: **F1 (Med)** `TransferOperationSpec.client_capabilities`/`.receiver_capacity` + `PeerCapabilities` were semantically orphaned (nothing read them since otp-9b stopped forwarding the spec; the proto comments claimed a forwarding boundary that no longer exists) — deleted, fields reserved; **F2 (Med)** five more pub helpers lost their only callers with the drivers (`is_complete`, `files_needing_transfer`, `allows_relative`, the `FsTransferSink` path-tracker plumbing, `return_vec`) — deleted (`compare_manifests` also caller-less; deferred to otp-11's sweep as local-path-adjacent); **F3 (Med)** the relocated builder was live code with no direct pins — 7 new `delegated_spec_tests` (endpoint mapping, the old driver's full precedence table, mirror scopes, carriage + normalization round-trip), precedence mutation-proven; **F4 (Med)** my finding doc claimed containment coverage otp-6b never had — `mirror_delete_pass`'s per-target `verify_contained` wiring is now directly pinned (foreign-root refusal + real-root control), mutation-proven (the unpinned mutation genuinely survived the suite pre-fix), claim corrected in place; **F5 (Med)** `docs/API.md` had never been swept (whole Push/Pull/negotiation sections + `RemotePushClient` example + a version-negotiation claim contradicting D-2026-07-05-2) — rewritten; ARCHITECTURE/WHITEPAPER/transfer.rs residue fixed; REVIEW `w6-2b` re-scoped from the deleted handlers to the served-session dispatcher where the byte-counter gap really persists; **F6 (Low)** the tracked `.claude/worktrees/vigilant-mayer` snapshot still contains the old tree — true; its `git rm -r` is the standing `725aa07` owner question, so the deletion proof is scoped to the workspace until the owner's go. Suite 1586 → 1480 (the priced-in deletion) → **1488** (+8 codex-round pins; the otp-13 floor of 1483 is met again, margin +5 — watch it at otp-11). fmt/clippy clean. Verdict `.review/results/otp-10c-2.gpt-verdict.md`. Next: **otp-11** — local transfers ride the in-process transport; the separate local orchestration is deleted; local perf pins hold.

 **2026-07-11 18:30:00Z** - **CODER (otp-10c-1: `--relay-via-cli` removed — the relay read half is out of the tree, claude)**: `f53f5a4` + review fixes `27bef56`. The 10b-2 deferral ("relay's PullSync-read half decided at 10c") went to the owner as a live choice this session — remove the flag, or keep it as a stage-to-temp-dir composition of two unified sessions (the streaming relay is unrebuildable once PullSync dies: its on-demand per-file remote read is a capability the session deliberately lacks). **Owner picked removal — D-2026-07-11-1**: remote→remote is delegated-only, the CLI never in the byte path; the unreachable-source topology gets the manual two-hop (pull local, push), which the delegated CONNECT_SOURCE hint now states. Deleted: the flag, `TransferRoute::RemoteToRemoteRelay` (+ the `select_transfer_route` relay param), all four relay-combination gates (detach×/mirror×/move×/resume×relay — their data-loss reasoning is moot with no relay to combine), `RemoteTransferSource` + its two bounded-read helpers (F7/R11-F1 bounds guarded the relay's remote tar assembly) + the constructed-instance counter, and the `relay_fallback_suggestable` hint machinery. **`PushExecution.source` narrowed `Endpoint` → `PathBuf`** — a remote push source is now unrepresentable; CLI/TUI call sites follow. Suite 1605 → 1585: 20 relay-only tests retired with per-test accounting (finding doc), zero live behavior left unguarded. **Codex (gpt-5.6-sol): FAIL → 3/3 accepted + fixed** (`27bef56`): **F1 (Med, the catch of the round)** the deleted relay e2e was the diagnostics counter's ONLY positive control — every survivor asserts `== 0` and `read_counters` maps a missing file to 0, so silently broken instrumentation would leave the load-bearing delegated isolation pins green vacuously; new `local_to_remote_push_is_the_positive_counter_control` (real daemon, ≥ payload bytes through the same flag/file/parser), guard-proven by no-op'ing the recorder to its FAILING vacuity shape and restoring (its own first run also caught a wrong landing-path assumption — the rsync no-trailing-slash nesting — fixed before the proof); **F2 (Med)** live guidance still advertised the flag (`--detach` help, README, blit.1.md, DAEMON_CONFIG, perf page) — root cause: the slice's docs grep was head-truncated; the unbounded re-sweep also caught ARCHITECTURE.md + WHITEPAPER.md beyond codex's list; **F3 (Low)** comment retype (the chokepoint fake models "an impl that ignores scan(filter)", not the deleted type), LOCAL_ERROR_TELEMETRY route list annotated, REVIEW `relay-1-subpath-double-join` closed as moot. Suite → **1586**, fmt/clippy clean throughout. Verdict `.review/results/otp-10c-1.gpt-verdict.md`. Push-state note: origin/master moved to `6d37a22` since the 42nd-session count (a partial push landed); unpushed is now `6d37a22..HEAD`. Next: **otp-10c-2** — the four drivers (`remote/push/`, `remote/pull.rs`, daemon `service/push/`, `service/pull_sync.rs`) + `Push`/`PullSync` out of tree AND proto (exclusive messages only — `TransferOperationSpec`/`ManifestBatch`/`BytesProgress`/job-kind enum stay), the delegated spec builder relocated out of `pull.rs`, ported-test accounting incl. the A/B reference pins, the DelegatedPull no-payload-bytes assertion, file-by-file deletion proof.

 **2026-07-11 16:00:00Z** - **CODER (otp-10b-2: the pull-shaped verb rides the unified session — VERB CUTOVER COMPLETE, claude)**: `2014782` + review fixes `3534ffa`. Every pull-shaped verb (CLI copy/mirror/move from a remote source, TUI F3) now initiates a DESTINATION-role `Transfer` session through one chokepoint (`blit_app run_remote_pull`); with otp-10a no verb can reach either old driver — otp-10c is pure deletion. Landed: **ONE args→compare mapping for BOTH verbs** (`blit_app transfers/compare.rs`, the old pull's precedence verbatim; push's `--checksum` gate lifted and push now honors `--checksum`/`--size-only`/`--ignore-times`/`--force`/`--ignore-existing` exactly like pull); **move mapping = IgnoreTimes, or Checksum when asked** (the one skip that is content-proven safe before a source-delete) + a NEW `--size-only` move gate (the old move-pull honored it — a live skip-then-delete hole); dest-side **w6-1 progress** via new `DestinationInstruments` (need batches = the denominator, mirroring push; per-file `Payload`/`FileComplete` on both carriers — `execute_receive_pipeline` already spoke the contract, the in-stream arms report inline) + pull `--trace-data-plane` (receive dials, epoch-0 + resize); **mirror = the in-session one delete rule** (`apply_pull_mirror_purge` and the client-manifest upload leave the verb path); printers retype to the session `TransferSummary` (pinned `Pull complete:`/`[gRPC fallback]` kept; `bytes_zero_copy` dies with the driver); the old pull's pre-created dest parent proved redundant by mutation (the sink creates parent chains) and was not ported; `test_pull_multistream_many_files` deliberately ported to trace-based fan-out (the old driver's unconditional `[pull-data-plane]` line died with it). New pins: `pull_session_cutover.rs` (11 — A/B parity vs old pull on twin daemons, move-shaped + checksum cells with SizeMtime controls, resume both carriers, daemon `--force-grpc-data` on pull, single-file layout, `--ignore-existing`), pull-move + push-checksum-pair + `--size-only` binary e2es, TUI builder pin, compare-mapping unit pins. **Codex (gpt-5.6-sol): NEEDS FIXES → 6 findings: 5 accepted (1 in part) + fixed, 1 deferred** (`3534ffa`): **F1 (High)** the mirror delete pass awaited its blocking task without reading the control lane — a peer CANCELLED sat unread while deletions ran behind a cancelled session; the purge now races one biased lane read, flips the abort flag, and the peer's fault owns the outcome (scripted-peer pin, 2000-file tree); **F2 (High)** delegated remote→remote MOVE still rode SizeMtime-then-delete (the otp-10a F1 loss on the missed route) — `delegated_pull_options` forces ignore_times unless `--checksum`, wire-pinned, TUI options defended; **F3 (High, in part)** the rewritten gate texts made route-dependent claims — local move (CLI+TUI) now maps through the move rule explicitly and the texts state the uniform truth; the claimed LIVE local data loss did NOT reproduce (probed the binary: the non-mirror local path copies unconditionally regardless of compare mode) — the two new pins are otp-11 regression pins, documented as such; **F4 (Med)** every served session was counted/exposed as Push with an empty endpoint — an `on_open` hook now fires at open-resolve (new `ActiveJobUpdater`): row kind + endpoint, `inc_pull` vs `inc_push` by role, `TransferStarted` with real values (pull sessions record **PullSync** rows again — CancelJob-capable; pre-open deaths emit the placeholder started to keep event pairing); **F5 (Med) DEFERRED** — the progress monitor lives through the in-session purge (display-only ticks/avg dilution; the fix is the M-C `AppProgressEvent` phase reshape; STATE residue); **F6 (Low)** TUI F1 builder routes through the one mapping. Guard proofs: **12 temporary mutations across both rounds** (slice: mirror/filter/ignore_existing/resume/force_grpc wiring, need-batch denominator, data-plane per-file lane, CLI move mapping — reproduced the exact skip-then-delete loss at the binary level — and pull trace; fix round: the purge race gated off, the delegated forced-ignore_times dropped, the source-resolver kind swapped), each run to a FAILING pin and restored; mutation U (the ported parent-creation step) left its pin GREEN and the redundant code was deleted instead. Suite 1581 → **1605** (full workspace 1605/0), fmt/clippy clean throughout. Verdict `.review/results/otp-10b-2.gpt-verdict.md`. Next: **otp-10c** — the four drivers + `Push`/`PullSync` out of the tree AND the proto, ported-test accounting, file-by-file deletion proof (incl. the DelegatedPull no-payload-bytes assertion; relay's PullSync-read half decided there).

 **2026-07-11 10:00:00Z** - **CODER (otp-10b-1: checksum compare on the session — contract v3, claude)**: `e82859e` + review fixes `7d3a1f2`. Staged sub-slice before the pull-verb cutover (10b-2): old pull honors `--checksum` content-skip but the session's `COMPARISON_MODE_CHECKSUM` degraded to transfer-everything (neither end hashed). Now a real content compare, role-agnostic: the SOURCE fills each manifest `FileHeader.checksum` via the new `ChecksummingSource` decorator (hashes through the inner source's own `open_file` — source-impl-agnostic, composed OUTSIDE the filter so only in-scope files pay), the DESTINATION hashes its SAME-SIZE diff candidates inside the existing blocking-pool chunk (size mismatch is already Modified — no hash), and a daemon with `--no-server-checksums` refuses a Checksum open at OPEN with the new `CHECKSUM_DISABLED` code (proto + `docs/TRANSFER_SESSION.md`; CONTRACT_VERSION 2→3; the old F11 ack-refusal contract reborn) via `ResponderPolicy`, which absorbs otp-10a's `force_in_stream` bool. Pins: role suite BOTH initiator layouts with SizeMtime controls (content-equal-diff-mtime SKIPS; same-size+mtime content change TRANSFERS — the cell `--checksum` exists for), daemon e2e served-skip + both-role refusal; 3 guard proofs by mutation (source wrap, dest hash, refusal). **Codex (gpt-5.6-sol): NEEDS FIXES → 5/5 accepted+fixed** (`7d3a1f2`): **F1 (High)** an unhashable file was dropped from the manifest — only the SOURCE sees its own unreadable list, so a pull could report success with the file silently absent (byte-identity hole; violated the slice's own conservative rule) — it now EMITS with an empty checksum and transfers unconditionally, unit-pinned with a failing-open stub; **F2/F3 (Med)** teardown bounding — the hashing task stops within one 64 KiB chunk of its consumer dying (`tx.is_closed` probe), and the destination diff chunk got the otp-9b `AbortFlagOnDrop` (hoisted to module scope) + chunked `hash_file_abortable`, abort never decaying into conservative-transfer; **F4 (Med)** `CHECKSUM_DISABLED` → NEGOTIATE in the delegated phase map; **F5 (Low)** stale STATE residue line. Suite 1576 → **1581**, fmt/clippy clean. Also this session: `blit_utils` e2e flake (du_json/ls_remote) REPRODUCED at pre-otp-10a `6d37a22` (2/8 binary-level runs) — pre-existing w9-3 daemon-spawn class, green in isolation 5/5, not introduced by otp-10 work. Owner note: zoey under maintenance today — no rig activity this session (all local). Next: **otp-10b-2** — pull-shaped verb rides `run_pull_session` (ONE args→compare mapping for BOTH verbs incl. lifting push's `--checksum` gate, dest-side w6-1 progress, printers retype, mirror retires `apply_pull_mirror_purge` from the verb path, move-pull adopts IgnoreTimes).

 **2026-07-11 07:00:00Z** - **CODER (otp-10a: the push-shaped verb rides the unified session — first otp-10 cutover slice, claude)**: `0fbc966` + review fixes. The one verb chokepoint (`blit_app run_remote_push` — CLI copy/mirror/move-push, `--relay-via-cli`, TUI F1) now initiates `run_push_session` as SOURCE; no verb can reach `RemotePushClient::push` (deletion = 10c). Deferred wiring landed: `PushSessionOptions` mirror/filter (open mapping, otp-9a's pull twin), `--force-grpc`→in-stream, **w6-1 progress via new `SourceInstruments`** (recv half reports NeedBatch as the push denominator; both carriers per-file `Payload`+`FileComplete` — chosen over the STATE sketch's ByteProgressSink because the plan pins the w6-1 event contract and a bare byte counter can't feed the files/manifest denominators), `--trace-data-plane` (the remote_parity pin's `[data-plane-client]` mechanism), resume flags, verb-level `end_of_operation_summary` print (D-2026-07-09-1 Q2), and the **old-push unreadable-scan error** (send-what's-readable then fail) that `blit move`'s source-delete gate relies on. `PushExecutionOutcome` retyped `RemotePushReport`→session `TransferSummary` (JSON drops files_requested/bytes_zero_copy/first_payload_ms, gains files_resumed; human drops the post-hoc negotiation/data-port lines; `[gRPC fallback]`/"already up to date"/purge/Destination survive) so 10c is pure deletion; TUI touch = 2 field reads. **Codex (gpt-5.6-sol): NEEDS FIXES → 8 findings, 7 accepted+fixed, F1 in part**: **F1 (High)** the session's same-size-dest-newer skip + move's source-delete = silent data loss (guard-proof reproduced it: exit 0, wrong bytes at dest) — move now pushes `ComparisonMode::IgnoreTimes` (transfer unconditionally; also closes the OLD documented R54 matching-size+mtime move hole; copy-verb skip stays the standing owner question, not self-adjudicated); **F2 (High)** `endpoint_module_path` put native `\` on the wire after the CLI's rsync join — POSIX-normalized via `path_posix` (fixes pull's client too); **F3 (High)** served sessions ignored daemon `--force-grpc-data` — threaded through `run_responder`/`responder_finish` (grant-less accept); **F4 (Med)** relay+resume faults on the data plane (`RemoteTransferSource` can't prepare composite ResumeFile) but works in-stream — refused up front; **F5 (Med)** fault stringification stripped the `io::ErrorKind` the `--retry` classifier downcasts for — `SessionFault.io_kind` captured at `fault_from_report` + the 4 dial sites, classifier extended (contract-tested); **F6 (Med)** resumed files emitted no progress — both carriers now report stale-bytes + one FileComplete; **F7 (Med)** fault-summary print untested — extraction split out + 4 unit pins; **F8 (Low)** `build_spec` validated no globs — pre-connection validation restored. Suite 1555 → **1576** (17 new pins incl. A/B parity vs old push on twin daemons, byte-identical + equal counts); **10 guard proofs by temporary mutation across both rounds**, all restored green; fmt/clippy clean. Verdict `.review/results/otp-10a.gpt-verdict.md`. Next: **otp-10b** (pull-shaped verb; the one args→open compare-flag mapping for both verbs lands there), then 10c deletion.

 **2026-07-11 01:00:00Z** - **BENCH + REVIEW (otp-2w codex round: 7/7 accepted — a measurement bug found on BOTH rigs; both matrices re-run, claude)**: Codex on `0c43d2a`+`ceea6ed`: NEEDS FIXES (4 Med, 3 Low), all accepted. The big one, **F3**: push windows wrapped `ssh host <flush>` and thereby carried ~1.2 s of connection/shell overhead (measured: ~0.5 s ssh + pwsh spawn to Windows; ~1.2 s slow-core key exchange to zoey) that pull windows never paid — inflating every published push median and ratio on BOTH rigs. Fix is structural: **self-timed durability steps** (Stopwatch around `Write-VolumeCache`; `/proc/uptime` around zoey's `sync`; the fsync walk reports its own elapsed) so only the flush itself joins the window; ControlMaster multiplexing added for wall-time. Both matrices re-run; biased sessions kept as labeled probes. **Corrected medians** — Windows: push_tcp 3054/1868/2288, pull_tcp 1294/1280/1284 (large/small/mixed; ratios ×1.46–×2.38; carrier-insensitive large in BOTH directions; spreads 0.2–14.5%); zoey: push_tcp 2702/4263/2070, pull_tcp 1744/2784/1401 (ratios ×1.25–×1.75; pool noise up to 48.6% on one cell — median carries it). Also fixed: fail-open drain probe ($null counted as quiet), stale-daemon masking (launch refuses; PID-exact teardown), unchecked P/Invoke steps + leaked token handle in purge-standby.ps1, my 8/12-vs-7/12 spread miscount (recounted from CSVs), "NEAR-SYMMETRIC" wording (owner said closer-spec), drain.log references (the `*.log` gitignore had silently dropped the drain evidence — renamed drain-outcomes.txt). The F3 fix's own two bugs (cross-process monotonic → 0/negative windows; PowerShell CRLF vs bash arithmetic) caught by running and documented. Verdict `.review/results/otp-2w.gpt-verdict.md`. Owner also opened **skippy** (TrueNAS, admin@skippy, existing blit-bin folder) for remote↔remote/Mac↔Linux — recorded in STATE. Next: **otp-10 (cutover + deletion)** — nothing holds it.

 **2026-07-10 23:00:00Z** - **BENCH (otp-2w: Windows cross-direction baseline — otp-2 CLOSED both halves, claude)**: The owner adjudicated the asymmetric-rig question by designating Mac↔Windows ("mac to windows would be closer spec. windows is faster, both have 10gbe") and standing up OpenSSH on the host (Win11 26200, Ryzen 9950X3D/32t, 96 GiB, Gen5 NVMe `D:`, PS7 default shell, admin token). Same-commit builds both ends: `0f922de` shipped as a **git bundle** (unpushed commits; pushes owner-gated; bundle = file copy between the owner's machines), host's prior checkout stashed (`bench-cargo-lock`), native build 31 s. New harness `scripts/bench_otp2w_baseline.sh` + `scripts/windows/purge-standby.ps1` — the zoey methodology with the daemon-host half in PowerShell: standby-list purge (NtSetSystemInformation + SeProfileSingleProcessPrivilege) for cold caches, `Write-VolumeCache D` durable push windows, Get-Counter disk-write drain, and two Windows traps found live: TOML double-quoted strings corrupt `\b` in paths (config uses TOML literal strings) and **Windows OpenSSH kills session children on disconnect** — a Start-Process daemon died with its ssh session (reproduced), so the daemon launches via WMI `Win32_Process.Create` (survives; cmd /c supplies log redirection). One program-scoped inbound firewall rule (`blit-bench-daemon`, removal documented). Byte-identical smoke, then the 12-cell matrix: **zero drain timeouts, 8/12 cells ≤2% spread (worst 11.9%)** — verdict-grade. Medians (`docs/bench/otp2w-baseline-2026-07-10/`): push_tcp large/small/mixed 3549/2503/2844 ms; pull_tcp 1309/1381/1316 ms; grpc ≥ tcp everywhere. **Recorded reading (not adjudicated): old push trails old pull ×1.8–×2.7 even on this close-spec pair, and on large pushes the carrier makes NO difference (3549 vs 3562) — the ceiling is the receive/write side (NTFS/Defender and/or the old push-receive code); otp-12's interleaved old-vs-new per cell discriminates which.** STATE: the open question is RESOLVED (zoey = per-direction; Windows pair = cross-direction; otp-12 runs both, interleaved A/B for pushes); nothing holds otp-10 anymore. Next: **otp-10 (cutover + deletion)**.

 **2026-07-10 21:00:00Z** - **BENCH + REVIEW (otp-2: codex round — 8 findings, re-run under the fixed harness, claude)**: Codex on `e757dcc`: **NEEDS FIXES, 8/8 accepted (one in part)** — the strongest review round of the plan so far. **F1 (High, upheld)**: "symmetric baseline" was mislabeled — D-2026-07-05-1's own rule (cross-direction comparisons valid ONLY on symmetric endpoints; "tmp on one side, spinning rust on the other is not a valid test") excludes Mac↔zoey (SSD vs pool), so the dataset anchors PER-DIRECTION converge-up only; re-framed everywhere, and the otp-12 cross-direction half is now an explicit owner question (options: per-direction verdicts suffice, and/or designate a symmetric pair). **F3 (High)**: STATE had pre-adjudicated that question ("gate satisfied", "Current: otp-10") — retracted; otp-10 HOLDS on the owner call. **F2 (High)**: macOS `sync(2)` schedules rather than waits, so pull windows weren't durable like Linux-sync push windows — pulls now fsync every landed file (`fsync_tree`; the honest +~150 ms on 10k-file pulls is visible vs the pre-review session). **F4/F5 (Med)**: drain now syncs-first and records per-run timeouts (never silent); push destinations carry a per-invocation tag + EXIT sweep. **F6 (Med)**: README claims re-stated to exact CSV numbers (probe-1 spread per cell up to 8.0×; pull stability ±6% typical/+21% worst; push/pull ratios ×1.23–×2.19; the manual drained probe committed as `probe3-drained-pushes.csv`); "unreachable regardless of code" replaced by the decision citation. **F7/F8 (Low)**: rounding policy stated; python3 preflight-checked; the monotonic-clock half of F8 was tried and REVERTED with evidence — cross-process `time.monotonic()` has undefined reference points, and the attempt produced 0/negative windows while the daemon logs showed real multi-second transfers (aborted run; wall clock restored with the why in a comment). **Full matrix re-run under the fixed harness** (`e757dcc` binaries both ends): medians push_tcp large/small/mixed 3025/3929/2666 ms, pull_tcp 1664/2699/1503 ms, grpc uniformly slower; exactly ONE drain timeout (the expected post-staging first run, recorded in the committed `drain.log`); cross-session agreement with the pre-review run ~10% (kept as `probe4-prereview-session-runs.csv`). Evidence dir refreshed; verdict `.review/results/otp-2.gpt-verdict.md`. **NEXT: the owner's (a)/(b) adjudication in STATE unblocks otp-10.**

 **2026-07-10 19:00:00Z** - **BENCH (otp-2: symmetric disk-to-disk baseline RECORDED, claude)**: The rig gate lifted mid-session — owner opened `root@zoey` (work confined to the standing `blit-temp` rule), designated the Mac's Thunderbolt 10GbE as the client end, and noted zoey's CPU won't saturate the link (confirmed; cells are CPU/storage-bound, which the per-cell reference tolerates). Also captured as a standing instruction: the Windows 10GbE box + TrueNAS serve **remote↔remote (delegated)** testing when that stage arrives. Both ends built from `731023b` — macOS arm64 client + static aarch64-musl daemon via `cargo zigbuild` (the toolchain path the July session proved but never recorded: brew zig + rustup musl target + cargo-zigbuild; binaries staged in zoey `blit-temp`, byte-identical smoke round-trip before any timing). **Methodology corrections earned by probe runs** (both kept as committed evidence): naive transfer-return timing is a write-cache lottery on zoey's pool (push cells 4–8× spread — mixed 1.4/6.1/11.6 s); durable-at-destination windows (transfer + dest `sync`) then exposed the pool's STATEFUL tiered write path (NVMe mirror destaging to 8 spindles — identical pushes ascend 2.7→13.4 s as the tier fills); a per-run pool DRAIN (three consecutive quiet 2 s windows) restores agreement (drained probe 4.5/2.7/3.1 s), with MEDIAN-of-4 absorbing the residual one-in-four outlier. Final matrix (`docs/bench/otp2-baseline-2026-07-10/`, harness `scripts/bench_otp2_baseline.sh`): 12 verdict cells (3 fixtures × push/pull × tcp/grpc), pulls ±2–8%, push medians stable (residual ±10–20% spread → otp-12 prescription: interleaved same-session A/B for push verdicts; the `731023b` binary pair stays staged on zoey for that). Medians: push_tcp large/small/mixed 2886/4048/2648 ms; pull_tcp 1707/2552/1510 ms; grpc uniformly slower (sanity ✓); 1 GiB durable ≈ 3.0 Gbit/s push / 5.0 Gbit/s pull; small-file ≈ 405/255 µs per file — July's per-file-bound shape at zoey's slower constant. July tmpfs/warm data re-labeled wire-reference only (plan sub-deliverable). **NEW OPEN QUESTION routed to the owner** (STATE): the plan's cross-direction acceptance bar presupposes hardware-symmetric endpoints; this rig's write ends are asymmetric (SSD vs pool — old-pull beats old-push ~1.7× everywhere for physics reasons), so the proposal is per-direction converge-up on asymmetric rigs, cross-direction only on symmetric ones. Not self-adjudicated. Finding doc `.review/findings/otp-2-symmetric-baseline.md`; codex review of the slice follows this entry. Next: **otp-10 (cutover + deletion)** — the rig gate is satisfied.

 **2026-07-10 17:30:00Z** - **CODER (otp-9b: delegated transfer rides the unified session — otp-9 done, claude)**: The otp-9 core (`b2fd876` + review fixes `1ce73b5`). `run_delegated_pull`'s validation front half (locator, spec boundary, delegation gate, module opt-outs, containment) is untouched; the bespoke old-driver body is replaced by a DESTINATION-initiator `Transfer` session against the validated source IP: `connect_transfer_client` (audit-1's 30 s bound preserved) → `Started` → `run_pull_session_with_client` (new split, so connect failures keep `CONNECT_SOURCE` structurally) with the spec mapped onto `PullSessionOptions` (`force_grpc` → in-stream carrier — the `--force-grpc` plumbing lands for delegation; mirror through `delete_list_authorized`; resume/filter/compare verbatim) and the row's `ByteProgressSink` (otp-9a). Session summary → wire summary (`in_stream_carrier_used` → `tcp_fallback_used`, its historical meaning). **Dead by construction**: the pre-enumerated local manifest (session diffs incrementally), the source-attested delete list (mirror runs locally via the one delete rule — R30-F1/R32-F1/R34-F1 structural), the R25-F2/ue-r2-1b capability/receiver-capacity overrides (the session advertises this end's own capacity). Retired with their 9 tests (called out; 1558 → 1552): `dst_capabilities`, `apply_dst_capabilities_override`, `apply_delete_list`, `build_summary`, `enumerate_local_manifest`. New two-daemon in-process e2es (`delegated_session_e2e.rs`): session landing + dst-authoritative counts, local mirror purge (plain copy never deletes), force_grpc in-stream pin; CLI `remote_remote` suite green over the reroute incl. the no-CLI-byte-path isolation pin; the `RejectingPullSyncBlit`/`StallingPullSyncBlit` fakes now model refusal/stall on the Transfer surface. **Codex (gpt-5.6-sol): NEEDS FIXES → 4/4 accepted + fixed** (`1ce73b5`): **F1 (High, session-wide)** `require_complete_scan` was forwarded but never enforced — a delegated MOVE could omit unreadable source files, report success, then the CLI deletes the source; the destination now refuses `ManifestComplete{scan_complete=false}` with `SCAN_INCOMPLETE` (R49-F2 on the session; scripted-peer pin with bounded wait). **F2 (High, pre-existing since otp-6b)** the mirror pass ran plan+delete in one `spawn_blocking` a dropped/cancelled session future cannot stop — deletions could continue behind a cancelled job; `AbortFlagOnDrop` flips an `AtomicBool` the pass checks before every fs op (unit-pinned). **F3 (Med)** open-time Transfer failures other than Unimplemented/PermissionDenied lost NEGOTIATE — `TransferOpenRefusal` marker + pure `session_error_phase` classifier (unit-pinned). **F4 (Med)** the fakes' retained legacy PullSync behavior made the reroute unguarded — their `pull_sync` is now Unimplemented, so a revert fails the refusal + cancel pins. All guard proofs by temporary mutation, run live. Suite 1552 → **1555**, fmt/clippy clean. **otp-9 CLOSED; otp-1..9 `[x]`** (one `test_utils_find_dirs_only` under-load flake seen once mid-slice — w9-3 daemon-spawn class, 3/3 green isolated). Next: **otp-10** (cutover + deletion) — but otp-2 (symmetric baseline, RIG-GATED) comes first per the plan.

 **2026-07-10 15:30:00Z** - **CODER (otp-9a: pull session-client surface, claude)**: First otp-9 (delegated transfer) sub-slice, staged like 4b/5b/6/7: before the delegated-pull handler can initiate the unified session (otp-9b), the pull session client needed the surface the old driver had. `7bf8ef8`: `PullSessionOptions` gains `filter`/`mirror_enabled`/`mirror_kind` mapped onto `SessionOpen` (the session honors both since otp-6 — this is client wiring only) + `byte_progress`; `DestinationSessionConfig.byte_progress: Option<ByteProgressSink>` threads into the destination session's `FsTransferSink` via its existing `with_byte_progress` contract (the delegated dst daemon keeps its ActiveJobs row live with it; CLI progress reuses the seam at otp-10). Served responder path unchanged (`run_responder` passes `None` — the daemon row-counter wiring stays the core.rs follow-up). Pins (suite 1555 → **1558**): mirror-ALL purge via options (`entries_deleted` scored, trees identical), include-filter scoping the REMOTE scan via options, caller `AtomicU64` == `bytes_transferred`. Guard proofs by temporary revert: dropped open-mapping fails both option pins; disabled sink hook fails the counter pin at 0. **Codex (gpt-5.6-sol): NEEDS FIXES → 1/1 accepted + fixed** (`607a924`): F1 (Low) stale `PullSessionOptions` rustdoc contradicted the new fields — corrected (push doc left: still accurate until otp-10). Codex independently confirmed the byte model (applied payload bytes) and the deferral scoping. Verdict `.review/results/otp-9a.gpt-verdict.md`. fmt/clippy clean throughout. Next: **otp-9b** — reroute `run_delegated_pull` steps 9-12 onto a DESTINATION-initiator session against the validated source endpoint (connect timeout kept; Started/Summary progress relay from the session outcome; mirror deletions land via the session's one delete rule so `apply_delete_list`/`enumerate_local_manifest` retire from the handler), A/B parity pin old-vs-session delegated, and the no-payload-bytes-through-`DelegatedPull` assertion.

 **2026-07-10 14:15:00Z** - **CODER (otp-8: fallback byte-carrier — closed by assessment + wire residue pins, claude)**: Executed assess-first per the 41st handoff: the slice's substance (the in-stream carrier, its negotiation-time selection via `SessionOpen.in_stream_bytes` or grant-less accept on bind failure, both wire directions, resume, clamps, `in_stream_carrier_used` reporting, the `--force-grpc`-shaped session-client seam) landed incrementally across otp-3..7b and was already pinned; the plan's "selected at negotiation — not a separate transfer path" wording deliberately retires the old drivers' mid-flight TCP→gRPC downgrade, so no new selection machinery was owed. The genuine residue: in-stream RESUME had no pin over the real gRPC transport — the 2 MiB in-stream block ceiling (D-2026-07-10-1) exists for tonic's 4 MiB frame decode limit, which the in-process role suite cannot enforce. Landed `5ffc9be` (tests only): `push_session_resumes_partial_over_in_stream_carrier` (the otp-7b data-plane fixture forced in-stream) and `pull_session_resume_clamps_oversized_blocks_to_in_stream_ceiling` (an 8 MiB block-size request over a 6 MiB file with ONE corrupt byte at 3 MiB must move exactly one 2 MiB block — the assertion is sensitive to the exact ceiling, and an unclamped implementation ships a 6 MiB frame tonic rejects). Guard proofs: ceiling 2→1 MiB fails the clamp pin at exactly 1 MiB; ignoring the in-stream request fails the carrier-flag asserts. Suite 1550 → 1552. **Codex (gpt-5.6-sol): FAIL → 2/2 accepted + fixed** (`643294a`). **F1 (High, the assessment's own Known-gaps deferral falsified)**: the in-stream record sends ran inline with NOTHING racing the receive half's framed peer faults — where the data plane routes `dp.queue()` errors through `prefer_peer_fault` and races its drain against `recv_peer_fault`, the in-stream branches were bare `?`; a mid-transfer cancel over the in-stream carrier provably HUNG the client (new e2e failed its 10 s no-hang timeout pre-fix, the framed CANCELLED sitting unread in the events queue). Fix: `SourceEventSender` mirrors every queued `Fault` onto a `watch` signal; both in-stream send sites `tokio::select!` it biased fault-first — a non-consuming side channel, so mid-send `Need`s stay queued. Pinned by `mid_transfer_cancel_surfaces_cancelled_over_in_stream_carrier`. **F2 (Medium)**: one `TarShardHeader` frame carried a shard's whole header list — the planner caps count (≤4096) and content bytes but not encoded header size, so legal long-path shards exceed the tonic limit (pre-existing in the OLD gRPC fallback lane too, but post-cutover the session is the only path). Fix: `MAX_IN_STREAM_TAR_HEADER_BYTES` (2 MiB) + `bound_in_stream_tar_headers`, an in-stream-only post-planner splitter (same grammar, same planner decisions, only record boundaries move; data plane untouched — its binary records have the 64 MiB cap). Pinned by a pure splitter test + a frame-capturing wiring test (4096 × ~590-byte paths, >2 MiB encoded, must emit multiple under-bound frames); guard proof by wiring-line revert. Verdict `.review/results/otp-8.gpt-verdict.md`. Suite 1552 → **1555**, fmt/clippy clean, full workspace run 1555/0 (one earlier unrelated `test_utils_rm_file` transport-error flake under suite load — w9-3's daemon-spawn class, 3/3 green isolated, not this slice's code). **otp-8 CLOSED; otp-1..8 `[x]`.** Next: **otp-9** (delegated transfer = daemon-initiated session; bespoke delegated-pull driver retired behind the gate; `DelegatedPull` reduced to trigger + progress relay).

 **2026-07-10 07:30:00Z** - **CODER (otp-7b: resume over the TCP data plane + the D4 fault-summary rider, claude)**: otp-7b landed as two slices through the codex loop, closing otp-7. **otp-7b-1** (`ecac9b0`): the responder no longer suppresses the data-plane grant for resume sessions; each correlated (need, hash-list) pair queues as ONE composite `TransferPayload::ResumeFile` work item, so one pipeline worker emits the whole binary `BLOCK*`/`BLOCK_COMPLETE` record on one socket — strict per-file serialization with no cross-socket reorder against the finalization truncate. The block-diff is single-sourced (`remote/transfer/resume_diff.rs::ResumeBlockDiff`) for both carriers; the DEST's grant map + `files_resumed` are `Arc`-shared with `NeedListSink`, which replicates the in-stream claim strictness on the sockets (grant-only, in-bounds, size-verified, exactly-once, resume-flagged grants refuse file/tar delivery). Block-size ceiling is per carrier — **D-2026-07-10-2**: 2 MiB in-stream, 64 MiB data plane (= `MAX_WIRE_BLOCK_BYTES`), decided by grant presence. `Push/PullSessionOptions.resume{,_block_size}` wired to `SessionOpen.resume`. Pins: roles suite both initiator assignments over loopback data planes + daemon e2e push/pull; guard proofs by temporary revert (neutered diff, re-suppressed grant). **otp-7b-2** (`071799a`): the Q2 rider — `SessionFault.relative_path` (wire: `SessionError.relative_path=5`, CONTRACT_VERSION→2), a typed `FaultedPath` eyre-chain marker lifted by `fault_from_report`, and `end_of_operation_summary()` naming the affected file + suggesting a re-run (verb-level print lands at otp-10; pins at session-client/e2e level per the plan's staging). Plus the 7a-deferred cancel-during-resume daemon e2e (7a F4) and a **gate-discovered RELIABLE bug from 7a**: `write_file_block_payload` never flushed its tokio file handle — `write_all` returning ≠ bytes at the OS — which made the 7a mid-resume pin ~50% flaky under full-suite load (block 0 vanishing); one `flush().await` fixed it, 12/12 clean loops after. **Codex (gpt-5.6-sol; reviews delayed ~1 h by an account usage limit): 7b-1 FAIL → 3 accepted+fixed, 1 pre-fixed, 1 deferred, 1 rejected; 7b-2 NEEDS FIXES → 4/4 accepted+fixed** (`d48351d`): F1 (High) a mostly-matching hash scan is silent long enough to trip the receiver's 30 s StallGuard — `ResumeBlockDiff` now emits keepalive ticks (stall/3) the data-plane sink answers with zero-length BLOCK records (unit-pinned); F4 resume batches now drive the sf-2 shape resize; F6 new pin proves the 64 MiB data-plane ceiling (guard-proven by ceiling revert); F2 = the flush bug, already fixed in 7b-2; F3 (queue not raced vs control events) DEFERRED — pre-existing otp-4b shape, keepalive bounds the new window, both cancel e2es pin behavior, filed in STATE residue; F5 (diff buffer outside BufferPool) REJECTED as blocking — worst case needs an explicit 64 MiB request, documented in the finding doc. G1 `relative_path` became proto3 `optional` ("" is the valid single-file-root identity; renders non-blank); G2/G3 tagging completed on both carriers' record paths; G4 contract doc updated. Verdicts: `.review/results/otp-7b-{1,2}.gpt-verdict.md`. Suite **1540 → 1550** (fmt/clippy clean throughout). ONE_TRANSFER_PATH otp-1..7 now `[x]`; next: **otp-8** per the plan queue.

 **2026-07-10 04:30:00Z** - **CODER (otp-7a resume over the in-stream carrier, claude)**: The unified session now RESUMES (`4e5ff58` + review fixes). Choreography per `docs/plan/OTP7_RESUME.md` (Active, D-2026-07-09-1): the DESTINATION flags resume-eligible needs (plan D2 — non-empty regular-file partial + compare says transfer), sends each flagged grant's `BlockHashList` right after its `NeedBatch`, and applies `BlockTransfer`/`BlockTransferComplete` records in place through the existing sink (`write_file_block_payload`/`_complete`, truncate+fsync+mtime/perms stamp from the retained manifest header); the SOURCE holds a resume-flagged need until its hash list arrives (the contract's strict ordering — enforced by construction, plus a fail-fast if NeedComplete leaves a need without its list), then Blake3-diffs the live file block-by-block and sends only stale blocks; a stale/garbage hash means SEND, never trust (D1 — graceful full-transfer fallback, pinned). `TransferSummary.files_resumed` is real; resume sessions get no data-plane grant (blocks are control-lane frames until otp-7b). All four plan guard-proof pins run under BOTH initiator roles (D6). **Codex: FAIL → 6 findings: 4 accepted+fixed, 1 partial, 1 deferred.** F1 (High, accepted): resume frames could exceed tonic's default (unraised) 4 MiB decode limit on the gRPC-served in-stream carrier — the D5 clamp allowed 64 MiB blocks (a 16×-oversized BlockTransfer frame) and a block_size=1 open amplified the hash list 32×; fixed as **D-2026-07-10-1** (plan D5 amended in place): DEST clamps block size into [64 KiB, 2 MiB], one BlockHashList capped at 65_536 hashes with over-cap partials degrading to the empty-list full-transfer fallback, SOURCE range-validates at arrival. F3 (Med, accepted): whole-file and tar records could claim resume-granted paths, silently bypassing the hash choreography — both arms now reject, and SourceDone verifies no resume grant is left uncompleted. F5 (Low, accepted): block-size validation moved to frame arrival (fail fast, not after pending plain files transmit). F6b (Low, accepted): the mid-fault pin now proves the fault was mid-record (block 0 landed in the partial, byte `bs` untouched); F6a rejected — zero `bytes_transferred` IS the zero-blocks observable, demonstrated by guard proof. F2 (Med, partial): per-list buffering now bounded (≤2 MiB) by the F1 cap; the O(resume-needs) aggregate is documented as a Known gap (same shape as the pending-needs vector). F4 (Med, deferred): hash/block phases inherit the session's existing payload-phase cancel latency — not a 7a regression; cancel-during-resume e2e added to otp-7b's scope in the plan. **Guard proofs (5, all run live)**: neutered block-diff ⇒ only-changed-blocks + zero-blocks pins FAIL; trusted stale hashes ⇒ byte-identity FAILS on corrupt output; removed in-record Error handling ⇒ dest-fault pin FAILS; removed the block-size clamp ⇒ both clamp pins FAIL; removed the bypass check ⇒ the bypass pin fails bounded (the destination otherwise absorbs the record and hangs — which is why the pin waits on a clock). Suite: true pre-slice baseline **1530** (the recorded 1529 was a miscount — re-counted on the stashed pre-slice tree) → **1540** (+9 role-suite pins incl. clamp/bypass, +1 lib cap-boundary test). fmt/clippy clean. REVIEW.md: otp-7a row added; otp-6a/6b rows backfilled (the 07-06 session logged those closes only in DEVLOG). Next: **otp-7b** — resume over the TCP data plane + the D-2026-07-09-1 CLI end-of-op fault summary rider + cancel-during-resume e2e.

diff --git a/REVIEW.md b/REVIEW.md
index adc6b9c..761aa61 100644
--- a/REVIEW.md
+++ b/REVIEW.md
@@ -2,160 +2,161 @@

 See `.review/README.md` for the workflow contract.
 See `.review/findings/<id>.md` for per-finding details.

 ## Legend

 - `[ ]` Open — coder may pick up
 - `[~]` In progress — coded/committed, codex review + adjudication in
   flight (`docs/agent/GPT_REVIEW_LOOP.md`, D-2026-07-04-1)
 - `[x]` Verified — codex verdict adjudicated, accepted findings fixed
   (records: `.review/results/<id>.codex.md` + `<id>.gpt-verdict.md`;
   rows graded before 2026-07-04 carry `<id>.verified.json` from the
   retired async loop)

 ## Unified transfer engine (REV4) — code→GPT-review→fix loop

 Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` (Active). Loop:
 `docs/agent/GPT_REVIEW_LOOP.md` (D-2026-06-20-6). These rows do **not** use
 the async `.review/` sentinel; status here means: `[ ]` not started ·
 `[~]` coded+committed, GPT review/adjudication in flight · `[x]` reviewed,
 accepted findings fixed, validation green. Records per slice:
 `.review/findings/<id>.md`, `.review/results/<id>.codex.md`,
 `.review/results/<id>.gpt-verdict.md`. Order/deps: REV4 §"Slice dependencies".

 | ID | Title | Status | Commit(s) |
 |----|-------|--------|-----------|
 | ue-r2-1a | Salvage adaptive PR1+PR2 substrate; resolve StallGuard-vs-`Probe`; work-stealing behavior tests | `[x]` | `e569eea`…`771a632` + review fixes |
 | ue-r2-1b | Wire dial contract: capacity profile + peer capability + resize proto (`receiver_capacity=11`); compat tests | `[x]` | `2741dc8` + review fix `5bd345a` |
 | ue-r2-1c | `TransferEngine` shell + `TransferOrchestrator` as local adapter; local fast paths → engine strategies | `[x]` | `7730eb1`+`dc9b0ed`+`29e210b` + review fix `15e6334` |
 | ue-r2-1d | Streaming plan foundation (partial-scan InitialPlan/PlanUpdate); prove ~1s start; RELIABLE exceptions | `[x]` | `c08a5c1` + review fixes |
 | ue-r2-1e | Live cheap dials replace the `determine_remote_tuning` ladder | `[x]` | `3be9105`..`15968f4` + review fix `46da929` |
 | ue-r2-1f | Push converge through the engine; retire daemon `desired_streams` ladder | `[x]` | `a4a9f70` + review fix `0c8da50` |
 | ue-r2-1g | PullSync multistream through the engine (absorbs MULTISTREAM_PULL) | `[x]` | `48e583e` + review fix `4a2e58d` |
 | ue-r2-1h | Delete deprecated `Pull` RPC (+ its `pull_stream_count` ladder) after harvest; port relay onto PullSync | `[x]` | `2a13f53` (+`9f37a7a` baseline/staging-slip, `48c5a11` win-1) + review fix `f6f52d7` |
 | ue-r2-2 | Stream resize: negotiated `DataPlaneResize`/`Ack`, mid-transfer add/drop — **REV4 complete** | `[x]` | `042ca4b`..`0788e83` + review fix `ec4a3fe` |

 ## Small-file ceiling (SMALL_FILE_CEILING) — code→GPT-review→fix loop

 Plan: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4).
 Same codex loop and record formats as the REV4 section above. Slice
 order and gates live in the plan (sf-6 is owner-gated on wire design;
 sf-3b… count is set by sf-3a's analysis, rows added as filed).

 | ID | Title | Status | Commit(s) |
 |----|-------|--------|-----------|
 | sf-1 | Tripwire + stream-scaling harness (`scripts/bench_tripwires.sh`) — baseline re-runnable in one command. Codex NEEDS FIXES (6/6 accepted: teardown ownership, coverage-enforced verdict, external-daemon mode, ±10% flag, record accuracy) | `[x]` | `7202c1a` + review fix `80633df` |
 | sf-2 | Shape-correction stream resize — client re-runs the shape table over the accumulated need list, corrects the daemon's partial-manifest epoch-0 proposal upward via the ue-r2-2 resize wire; loopback e2e pins 10k-file push > 1 stream (guard proven by revert). Codex NEEDS FIXES (1/1 accepted: count from append-only `files_requested`, not the pruned set) | `[x]` | `c70c2ac` + review fix `7627e7b` |

 ## One transfer path (ONE_TRANSFER_PATH) — code→GPT-review→fix loop

 Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4).
 Same codex loop and record formats. Slice order lives in the plan;
 otp-2 (symmetric baseline) needs the 10 GbE rig and must land before
 otp-10 cutover — it may execute out of strict order when the rig is
 available.

 | ID | Title | Status | Commit(s) |
 |----|-------|--------|-----------|
 | otp-1 | Unified Transfer session wire+session contract — docs/TRANSFER_SESSION.md + Transfer RPC/messages + refusing stubs + reachability pin. Codex NEEDS FIXES (6/6 accepted: role-lane closing flow, accept-ceiling dial semantics, socket auth, in-stream record grammar, flow control/NeedComplete ordering, error-field drift) | `[x]` | `a3e2acb` + review fix `f861579` |
 | otp-2 | Symmetric baseline, CLOSED BOTH HALVES 2026-07-10: corrected harness matrix + OLD-path per-cell baselines. otp-2 zoey (per-direction reference; hardware-asymmetric scope note per D-2026-07-05-1) — codex 8 findings addressed, matrix re-run under the fixed harness. otp-2w Mac↔Windows (owner-designated cross-direction rig) — codex 7 findings addressed (self-timed durability), both matrices re-run. Evidence `docs/bench/otp2-baseline-2026-07-10/` + `otp2w-baseline-2026-07-10/`. *(Row was stale `[ ]` until 2026-07-12; closes were recorded in DEVLOG 2026-07-10 — fixed on owner go.)* | `[x]` | `e757dcc`+`4286c23` / `0c43d2a`+`7e732d4`+`ceea6ed` |
 | otp-3 | TransferSession core — role-parameterized drivers over `FrameTransport` (in-process pair), strict same-build hello, destination-owned incremental diff (`manifest::header_transfer_status`), in-stream record grammar fail-fast; role suite pins identical need sets/summaries/trees under both initiator layouts. Codex FAIL (2/2 accepted: build-identity false-match — non-collapsing dirty/unknown forms; early-NeedComplete gate) | `[x]` | `ef9ffa1` + review fix `d5796a1` |
 | otp-4a | Daemon serves `Transfer` (runs `run_destination` as Responder; client `run_source`s as SOURCE initiator over a gRPC `FrameTransport`, in-stream carrier). Responder-resolution API (`DestinationTarget` + async `OpenResolver` through `establish`); read-only/unknown-module refusals as `SessionError` frames; A/B byte-identical parity vs old push; unified SizeMtime = safe-skip (⚠ narrow owner-ack, STATE). Codex FAIL (1/1 accepted: cancel must emit a framed `SessionError{CANCELLED}`). | `[x]` | `4b07bbb` + review fix `25f538b` |
 | otp-4b | TCP data plane + resize + sf-2 pin ported to the session; deterministic mid-transfer cancel e2e. 4b-1 single-stream data plane (codex 3 passes), 4b-2 resize/multi-stream/sf-2 (codex PASS), 4b-3 mid-transfer cancel — source surfaces `SessionFault{CANCELLED}` over the data plane, no hang (codex 3 passes) | `[x]` | `881d412`+`e1aafcc`+`777dfc5` / `dce56de` / `3ae0a5f`+`a530005`+`46cc4bb` |
 | otp-5a | Daemon serves BOTH roles via new `run_responder` (dispatches on declared `initiator_role`): a DESTINATION initiator makes the daemon the SOURCE (pull-equivalent, streams its module tree, in-stream); a SOURCE initiator keeps otp-4 push. `establish`→`exchange_hello`+`responder_finish`; `run_source`/`run_destination` bodies→`drive_source`/`drive_destination`; new `SourceResponderTarget`; client `run_pull_session`. A/B byte-identical vs old `pull_sync`. Codex PASS (no findings). Data plane for the SOURCE responder is otp-5b. | `[x]` | `84be1cc` |
 | otp-5b-1 | Single-stream SOURCE-responder TCP data plane: decouples data-plane connection role (RESPONDER binds+accepts, INITIATOR dials) from byte role (SOURCE sends, DESTINATION receives). New `accept_source_data_plane` (SOURCE responder accepts+sends) + `dial_destination_data_plane` (DESTINATION initiator dials+receives), `DestRecvPlane` enum; `responder_finish` binds for either role; `run_pull_session` defaults to TCP. Single-stream (`resizable=false`); resize is otp-5b-2. Codex FAIL → 1 Med accepted+fixed (grant-without-host fail-fast). | `[x]` | `e6a0b3b`+`13485ee` |
 | otp-5b-2 | Pull data-plane resize: lifts otp-5b-1's single-stream cap so the pull data plane grows mid-transfer via sf-2 shape correction, exactly as push. Same `DataPlaneResize{ADD}`/`Ack` frames; only socket acquisition flips — SOURCE responder ACCEPTS each epoch-N socket off its listener, DESTINATION initiator DIALS it. `SourceSockets` enum (Dial/Accept); `add_stream` branches; `InitiatorReceivePlaneRun.add_dialed_stream`; `destination_session` initiator branch seeds `resize_live`+ceiling; `Frame::Resize` branches arm (responder) vs dial (initiator). Codex NEEDS FIXES → 1 Low accepted+fixed (ceiling uses advertised capacity, not a fresh local read). | `[x]` | `d579365`+`773a877` |
 | otp-6a | Filters on the session: `SessionOpen.filter` honored via the universal `FilteredSource` chokepoint (not the per-impl `scan(filter)` arg); globs validated at OPEN, peer-notified refusal. Codex F1 accepted (chokepoint, not scan arg). *(Row backfilled 2026-07-10 — the 07-06 session logged the close only in DEVLOG.)* | `[x]` | `c026692`+`0bb27f5` |
 | otp-6b | Mirror on the session — the ONE delete rule: DESTINATION diffs the complete source manifest at SourceDone, scan-complete-guarded + filter-scoped; `plan_session_deletions` + containment-checked `mirror_delete_pass`. Codex NEEDS FIXES → 2 accepted+fixed (High: keep-set now folds case on macOS too — case-insensitive-FS data-loss; Med: Windows read-only clear before delete). *(Row backfilled 2026-07-10.)* | `[x]` | `01d9c41`+`3c99557` |
 | otp-7a | Resume block phase over the in-stream carrier (`docs/plan/OTP7_RESUME.md` Active, D-2026-07-09-1): DEST flags eligible needs (D2), sends per-grant `BlockHashList`, applies block records in place; SOURCE holds a resume need until its hash list arrives, sends only stale blocks (D1 graceful stale fallback); `files_resumed` real; resume sessions in-stream-only until 7b. All four plan guard-proof pins run live under both initiator roles; 5 guard proofs by temporary revert. Codex FAIL → 6 findings: 4 accepted + fixed (wire bounds D-2026-07-10-1 — block size clamped [64 KiB, 2 MiB] + 65_536-hash cap; choreography-bypass records rejected; arrival-time validation; mid-fault pin observes the partial patch), 1 partial (aggregate hash-list buffering documented), 1 deferred to 7b (cancel-during-resume e2e). | `[x]` | `4e5ff58` + review fix `1919410` |
 | otp-10a | Push-shaped verb rides the session: `blit_app run_remote_push` (CLI copy/mirror/move-push + relay + TUI F1) reroutes onto `run_push_session`; deferred verb wiring lands — `PushSessionOptions` mirror/filter, `--force-grpc`→in-stream, w6-1 progress via new `SourceInstruments` (need-batch denominator; both carriers per-file lane), `--trace-data-plane`, resume flags, verb-level `end_of_operation_summary` print, old-push unreadable-scan error (move's source-delete gate); `PushExecutionOutcome` retyped to session `TransferSummary` so 10c is pure deletion. Codex NEEDS FIXES → 8 findings, 7 accepted+fixed, F1 in part (High: move now pushes `IgnoreTimes` — compare-skip + source-delete data loss, mutation-proven; copy half = standing owner Q. High: wire paths POSIX-normalized. High: daemon `--force-grpc-data` honored by sessions. Med: relay+resume refused; `SessionFault.io_kind` keeps `--retry` alive; resume w6-1 progress both carriers; fault-summary unit pins. Low: `build_spec` validates globs pre-connect). Suite 1555 → **1576**; 10 guard proofs by temporary mutation across both rounds. | `[x]` | `0fbc966` + review fixes `6b292ed` |
 | otp-10b-1 | Checksum compare on the session (contract v3): `COMPARISON_MODE_CHECKSUM` = real content compare both roles — SOURCE fills manifest Blake3 via new `ChecksummingSource` (through the inner source's `open_file`, outside the filter), DEST hashes same-size diff candidates in the blocking chunk; daemon `--no-server-checksums` refuses at OPEN with new `CHECKSUM_DISABLED` (ResponderPolicy absorbs otp-10a's force_in_stream). Role-suite pins both layouts with SizeMtime controls; e2e served-skip + both-role refusal. Codex NEEDS FIXES → 5/5 accepted+fixed (High: unhashable files now EMIT with empty checksum — the drop let pulls succeed with a file silently absent; hashing stop probes bound teardown to one 64 KiB chunk both ends; `AbortFlagOnDrop` hoisted; delegated phase map + STATE drift). Suite 1576 → **1581**; 3 mutation guard proofs. | `[x]` | `e82859e` + review fixes `7d3a1f2` |
 | otp-10c-2 | The cutover deletion — otp-10c CLOSED, one transfer path by construction: the four drivers (`remote/pull.rs` 2574 LOC, `remote/push/`, daemon `service/push/`, `service/pull_sync.rs`), `rpc Push` + `rpc PullSync` + 13 exclusive messages (incl. `DataTransferNegotiation`, the old summaries, `metadata_only`), the two wire-specific gRPC fallback sinks + `grpc_fallback.rs`, and every helper whose only callers died — out of tree AND proto, no bridge (D-2026-07-05-2). Relocated verbatim: the delegated spec builder (`DelegatedSpecOptions`/`delegated_spec_from_options` → operation_spec.rs) + `FsTransferSource`'s fs-scan helpers. A/B parity pins → absolute tree+count pins; DelegatedPull no-payload-bytes proof recorded (proto oneof + CLI byte-counter pins). Codex NEEDS FIXES → 6/6 accepted (F6 owner-gated): spec capability/capacity fields + `PeerCapabilities` deleted (orphaned since otp-9b); 5 more orphaned helpers out; the relocated builder re-pinned (7 tests) + `mirror_delete_pass` containment wiring pinned — both mutation-proven; `docs/API.md` (never swept) + 4 more doc/comment sites fixed; `w6-2b` re-scoped to the served-session dispatcher; the tracked `.claude/worktrees` snapshot deferred to the standing `725aa07` owner question. Suite 1586 → 1480 (106 retirements, all enumerated in the finding doc) → **1488** | `[x]` | `7aac28b` + review fixes `995e1cc` |
 | otp-10c-1 | `--relay-via-cli` removed (owner decision D-2026-07-11-1) — remote→remote is delegated-only, the CLI never in the byte path: flag + `RemoteToRemoteRelay` route + all four relay-combination gates deleted; `RemoteTransferSource` + bounded-read helpers + constructed-counter die; `PushExecution.source` narrows `Endpoint`→`PathBuf` (remote push source unrepresentable); delegated hints reworded (CONNECT_SOURCE → manual two-hop). Codex FAIL → 3/3 accepted+fixed (Med: counter's positive control restored — new push e2e, mutation-proven against a no-op'd recorder; Med: live guidance purge incl. ARCHITECTURE/WHITEPAPER beyond codex's list; Low: comment retype + relay-1 row closed moot). Suite 1605 → 1585 (20 relay-only tests retired, accounted) → **1586** | `[x]` | `f53f5a4` + review fixes `27bef56` |
 | otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |
 | otp-11b | THE LOCAL ORCHESTRATION DELETION — the last old path out of the tree (−6.2k lines): `orchestrator/`, `engine/` (dial RELOCATED VERBATIM → `src/dial.rs`, blob-identical, 17 tests), `local_worker`, `auto_tune/`, `change_journal/` (the UNSOUND journal skip — the 11a-addendum data-loss repro), `copy/parallel+stats`, `CopyConfig`; the otp-10c-2 F2 `compare_manifests` sweep (live compare owner `header_transfer_status` + `compare_file`/`CompareMode`/`CompareOptions`/`FileStatus` survive); stranded `plan_local_mirror`/`LocalDiffInputs`/`filter_unchanged`; types re-homed → `transfer_session/local.rs` (dead axes dropped, `JournalSkip`/`PredictorEstimate` retired); `TRANSFER_SESSION.md` local-carrier contract note. Codex CHANGES REQUESTED → core CONFIRMED ("deletion, re-homes, converted coverage, remote-session behavior, one-transfer-path structure, and the 1484-pass suite check out") + 6 docs/record findings, 6/6 fixed (live-doc sweep completed incl. WHITEPAPER/ARCHITECTURE/repo-guidance; predictor promises retyped; effective worker count printed; accounting equation corrected). Suite 1513 → **1484** (died-in-modules 41 + deleted files 10 + retired 5, conversions 25 in place, new +27; the otp-13 ≥1483 floor MET at the deletion slice, margin +1); SizeOnly mutation guard proof. The plan's deletion-proof acceptance line for "the separate local orchestration path" COMPLETES here. | `[x]` | slice `805e48c` + docs `b1650c4` + review fixes `9e810ee` |
 | otp-12a | Zoey converge-up A/B recorded (design `docs/plan/OTP12_ACCEPTANCE_RUN.md` Active — owner flip; D-2026-07-12-1 residue rule). Three codex rounds: design CHANGES REQUIRED 7 findings (6 accepted + 1 overtaken-by-owner-decision); harness REQUEST CHANGES 9/9 accepted (zero false positives); run round FAIL 6/6 accepted (provenance `+sha` form, D2 supersession amendment, drift/gap wording per CSVs). En route: otp-2 daemon provenance corrected (staged pair was dirty `731023b`, not `e757dcc`); zoey I/O-storm diagnosed → per-run dest sweep. Evidence `docs/bench/otp12-zoey-2026-07-12/` (3 sessions incl. aborted storm): **10 PASS; pull_tcp_large FAIL-REFERENCE-DRIFT (rig-side by strongest evidence); push_tcp_small FAIL-SAME-SESSION 1.105** — both carried to the otp-13 walk. | `[x]` | design `045da4a`+`92e1d51`; harness `8f4fbf9`+`50dc135`; run `b2b6901`+`b3729da`+`042c06f`+`6bc9cb6`+`b0ebf73`+fixes `fa18787` |
 | otp-12b | Mac↔Windows acceptance session recorded — THE INVARIANCE CRITERION MEASURED: 11/12 cells PASS at 1.003–1.057 (the owner's sentence holds); wm_tcp_mixed FAIL 1.237 (TCP×mixed×destination-initiator — real, block-1-corroborated, code-shaped). Converge 10/12 (push_tcp_small 1.149 FAIL-BOTH — matches zoey's 1.105, second rig; pull_tcp_mixed 1.313 same root). Cross: Win→Mac 6/6 beat the better old direction; Mac→Win gap rows recorded per D-2026-07-12-1 shapes (large unchanged / mixed+grpc_small narrowed / tcp_small widened), adjudication reserved to otp-13. Three codex rounds: harness FAIL 12/12 accepted; run-round FAIL 3/3 accepted (self-adjudication scrubbed); + two found-live fixes (pwsh `$rc:R` scope-parse sentinel; CR-split verdicts). 192 runs, zero voided. Evidence `docs/bench/otp12-win-2026-07-12/`. | `[x]` | harness `d30b1e3`+`772cfe6`+`d3eae58`; run `e21cf84`+`856af64`+`44c2046`+fixes `49dee5c` |
 | otp-12c | Rig-D delegated-parity session recorded (netwatch-01↔skippy) + a rig-W re-baseline at the CUTOVER sha `f35702a` (12b measured `e21cf84`, so no committed rig-W evidence existed at the sha the shipped binaries embed). New harness `scripts/bench_otp12_delegated.sh` (plan D4: delegated = Mac CLI triggers `DelegatedPull`, no payload through the Mac; direct = the destination host's own CLI pulls; same session code, roles, data plane, destination disk and flush — only the initiator differs). **Rig D: 7/7 PASS** — RUNS=4 gave 5 PASS / 2 FAIL (`sw_tcp_mixed` 1.119, `ws_tcp_large` 1.129); both FAIL cells met D2's pre-registered escalation trigger (straddle + >25% arm spread) and re-ran at RUNS=8, whose medians govern per the D2 supersession amendment → both PASS (1.035, 1.068), with the wide spread appearing on the *direct* arm too at higher n. 88 timed runs across two sessions, **zero voided pairs**. Rig-W re-baseline: 198 runs, 93 PASS / 12 FAIL / 3 FAIL-SAME-SESSION / 12 RECORDED — `wm_tcp_mixed` invariance **1.300** (12b: 1.237), i.e. the TCP×mixed×dest-initiator cell did NOT wash out at the cutover sha. Three harness bugs found live, each caught by the script's own gates (apostrophes in `:?` messages swallowing assignments — the otp-12b `772cfe6` bug re-made; macOS `$TMPDIR` blowing ssh's 104-byte ControlPath limit; skippy's `drop_caches` needing the exact NOPASSWD grant, whose generic form silently no-op'd → runs would have read WARM). Codex FAIL → **7/7 accepted, 0 rejected**: F1 cold-cache fail-open (HIGH — grant now a hard gate; a failed purge voids the pair); **F2 D2 misread (HIGH — the first draft scoped the escalation amendment to converge-up rows only and so ducked the verdict; the rule says "a comparison", delegated parity included → rig D 7/7 PASS)**; F3 provenance (`proto/` added to the dirty-tree gate; `+sha` no longer substring-matches `+sha.dirty.<hash>` — the otp-12a zoey trap); F4 machine-readable build fields recorded harness HEAD, not the gated binary identity; F5 silent `sync`/drain failures (failed sync → NA → void; a disk regex matching no device is DRAIN-NODEV, not drained); F6 teardown logged "stopped" without verifying (a survivor now exits nonzero); F7 a PASS listed among the FAILs. Codex independently confirmed the otp-12b F5 arm asymmetry does NOT recur and that every committed CSV recomputes exactly. Evidence `docs/bench/otp12c-{win,delegated}-2026-07-13/`. Acceptance reserved to the otp-13 owner walk. Suite untouched at **1484** (zero `crates/`/`proto/` changes). | `[x]` | harness `c26bc2d`+`b49413d`+`a2dea3f`; evidence `d12534d`+`68bb490`; record `9350b24` + review fixes `0fb4a64`+`4cc9b6e` |
 | otp-12-worker-parity | Acceptance repair: the one SOURCE-owned, receiver-bounded worker policy now reaches the same exact target under both initiator layouts. Guard proofs exposed 3 vs 2 workers and a destination-initiator unknown-capacity cap of 1; final 10,000-file role pins are 8/8. Payload dispatch proceeds while resize ACKs are pending; refusal is terminal; resize eligibility through claim is atomic with settlement and mutation-proved in debug/release. Five Codex FAIL rounds (2+1+2+1+1 findings, all accepted) converged to a final PASS with no findings; Grok second-eye review then returned ACCEPTED with its own zero-capacity red→green mutation proof. Full workspace gate: 1,490 passed, 2 ignored. No hardware run in this slice. | `[x]` | `a76b785` + `cfd9dd7` + `8e993aa` + `641916e` + `f7f12ec` + `42b9b38` |
 | otp12-pf1-session-phase-trace | Wire-neutral TCP-only pf-1 timing probe across both initiator layouts: correlated SOURCE/DESTINATION phase JSON, dial/accept/resize/need/planner/per-socket first-payload/completion coverage, dedicated writer thread with terminal flush, and explicit trace-state restart requirements. Deterministic guard pins both roles' epoch-0/epoch-1 attachment and every payload-carrying socket; production env/writer/flush path is mutation-proved. Full workspace gate: 1,493 passed, 2 ignored. Codex PASS with no findings. Requested Grok second eye was attempted twice but both responses were schema-invalid and internally contradictory; recorded fail-closed as a contested reviewer-protocol outcome, not a code finding or Grok PASS. No rig result. | `[x]` | `5b8cc29` |
+| otp12-pf1-rigw-harness | Reduced q↔netwatch-01 P1 diagnostic using only semantic SOURCE/DESTINATION initiator roles. Fixed OFF–ON–ON–OFF 128-arm schedule; both daemons restart per block; successful client + daemon traces correlate by run/session; destination-keyed durability, one role-invariant destination path, exact relative-path/size landed manifests, conservative paired/bimodal/observer resolution, fabric/process/PID/port gates, and no endpoint-policy mutation. Initial Codex 3/3 High and round-2 Codex 2/2 High fixes landed; coder audits added launcher smoke and pre-daemon PID journaling. Round-3 Grok reopened vacuous Bash 3.2 guards; G3/G4 fixed them and round 4 passed Codex + Grok. The first live launcher smoke refused safely before daemon start on q's valid multi-interface ARP cache; G5 scoped it to en8 and round-5 Codex + Grok independently confirmed the fix. Round-5 Codex then found one separate High: Windows cache purge trusted a pre-existing helper without reviewed-file provenance. G6 now stages the reviewed helper into the fresh session tree only after read-only gates, verifies its hash after copy and before every arm, requires its exact success sentinel, records it in the manifest, and mutation-proves both hash checks. Fresh complete review pending; no rig data. | `[~]` | reviewed `06b3322`; G6 fix `888be47`; review pending |
 | otp-10b-2 | Pull-shaped verb rides the session — verb cutover COMPLETE: `blit_app run_remote_pull` (CLI copy/mirror/move-pull + TUI F3) reroutes onto `run_pull_session`; ONE args→compare mapping for BOTH verbs (`transfers/compare.rs`; push `--checksum` gate lifted, every compare flag + `--ignore-existing` honored both directions); dest-side w6-1 progress + pull `--trace-data-plane` via new `DestinationInstruments`; printers retype to the session summary; mirror = the in-session one delete rule (`apply_pull_mirror_purge` off the verb path); move maps IgnoreTimes/Checksum-only + new `--size-only` move gate; A/B parity vs old pull on twin daemons; multistream e2e ported to trace-based fan-out. Codex NEEDS FIXES → 6 findings, 5 fixed (1 in part), 1 deferred (High: cancel frame mid-purge now aborts deletions; delegated move rode SizeMtime-then-delete — wire-pinned fix; gate texts + explicit local move mapping — the claimed live local loss did NOT reproduce, probe recorded, pins are otp-11 regression pins. Med: served sessions record real kind/endpoint/metrics — pull = PullSync rows; monitor-through-purge display deferred to the M-C reshape. Low: TUI builders through the one mapping). Suite 1581 → **1605**; 12 mutation guard proofs across both rounds. | `[x]` | `2014782` + review fixes `3534ffa` |

 ## Design-review queue (ratified D-2026-06-11-2, in execution order)

 Source: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` (slice specs) +
 `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md` (per-finding evidence).
 Coder loop: pick the topmost `[ ]` row. W2.3 requires a `docs/plan/` doc with
 **Status: Active** before code.

 | ID | Severity | Title | Status | Branch | Commit |
 |----|----------|-------|--------|--------|--------|
 | w5-1-log-backend | Medium | Install stderr log backend (warn) in all 4 binaries + one prefix convention; today every log::warn/error is discarded | `[x]` | master | `56bda09`+`7145202` |
 | w4-2-delete-push-upload-channel | Medium | Delete the 262,144-slot push upload channel (drain-and-discard; wedges gRPC-fallback pushes >262k files) | `[x]` | master | `03bcb1d` |
 | w5-2-retry-classifier-consolidation | Medium | Delete dead contradictory blit-core/errors.rs; move is_retryable into blit-core with contract test | `[x]` | master | `9c960dc` |
 | w4-1-abortondrop-family | High | Hoist AbortOnDrop; fix the remaining detach-on-drop sites (2 of 5 deleted with the Pull RPC at ue-r2-1h; design-2 now scopes to push/control.rs only; JoinSet for per-stream workers). Codex NEEDS FIXES (1 Low: relocated drop-test was vacuous) → fixed `bedfa52` | `[x]` | master | `65ecb93`+`bedfa52` |
 | w9-5-jobs-lifecycle-e2e | Medium | jobs/detach lifecycle e2e tests (Subscribe, watch fallback, cancel exit codes) — net before W4.3 | `[x]` | master | `ad773d8` |
 | w4-3-daemon-disconnect-racing | Medium | Daemon handlers race tx.closed()+cancel token (delegated_pull's select generalized to resolve_transfer_outcome + resolve_streaming_outcome; 2 live sites — pull spawn closure died with the Pull RPC at ue-r2-1h); false supports_cancellation comment fixed, dispatch policy itself unchanged (flip = open owner question, since decided D-2026-07-04-3 → w4-5). Codex PASS (0 findings) | `[x]` | master | `37d7f91` |
 | w4-5-supports-cancellation-flip | Medium | Flip supports_cancellation for Push/PullSync (owner-authorized D-2026-07-04-3): CancelJob + TUI F2 work on attached transfers; policy-only after w4-3's race wiring (one-predicate flip — Pull history-only stays gated; TUI/CLI needed zero logic changes); contract change exit 2→0 pinned at table + RPC-handler level, authz now covers flipped kinds; every old-policy comment surface updated incl. proto wire-contract doc. Codex NEEDS FIXES (1 Low: module-doc scope log still claimed Pull wired) → fixed `1708075` | `[x]` | master | `05a8b39`+`1708075` |
 | w1-2-data-socket-policy-helper | Medium | Shared configure_data_socket (NODELAY/keepalive/tuned buffers) hoisted to blit-core; pull client connect + daemon push/pull_sync accepts all route through it; pull_sync passes the dial's tcp_buffer_bytes (resize accept reads it live — the computed-and-discarded gap closed); daemon's silently-swallowing twin + socket2 dep deleted. design-3 (connect timeouts) untouched. Codex PASS (0 findings) | `[x]` | master | `16237e2` |
 | w1-3-tcp-keepalive-honesty | Medium | Real TcpKeepalive timing (idle 60s / interval 10s / retries 5) at the single site left after w1-2 (the shared helper; daemon copy already deleted, logs-failure clause satisfied structurally) — dead idle peer detected in ~2 min, not ~2 h; comments now true; socket2 features=["all"] for retries + test getters. Codex PASS (0 findings) | `[x]` | master | `865fc1e` |
 | w1-4-accept-token-constants | Low | Shared DATA_PLANE_ACCEPT_TIMEOUT(30s)/DATA_PLANE_TOKEN_TIMEOUT(15s) in remote::transfer::socket replacing the 3 declarations left at HEAD (4th died with the Pull RPC); values byte-identical. Codex NEEDS FIXES (1 Low: stall_guard comments named the deleted pair) → fixed `d17b089` | `[x]` | master | `6a19e1d`+`d17b089` |
 | w2-1-delete-warmup-machinery | Medium | Delete dead auto_tune warmup branches + analyze_warmup_result (honest static table) | `[x]` | master | `2a8a490` |
 | w2-2-stream-ladder-owner | Medium | Single stream-count/chunk owner: the 3 stream ladders died with REV4 (ue-r2-1e dial / -1f initial_stream_proposal takes file_count / -1h Pull RPC; absorption recorded D-2026-06-20-1); this slice closed the remaining leg — deleted the dead transfer_plan chunk lane (16/32 MiB ladder, Plan/PlannedPayloads wrappers, chunk_bytes_override + refresh sites, never-called plan_to_daemon_format, orphaned TuningParams); dial is the single chunk owner; W3.1's "settled tuning owner" = engine::TransferDial. Codex NEEDS FIXES (1 Low: new ensure_dial comment said "fallback batch" in the data-plane branch) → fixed `27f53a0` | `[x]` | master | `01209bc`+`27f53a0` |
 | w2-3-multistream-pull-plan | High | Multi-stream pull-sync: write plan doc (authorized D-2026-06-11-2), harvest deprecated Pull's pattern, implement — absorbed into REV4 (D-2026-06-20-1); delivered as `ue-r2-1g` | `[x]` | master | `48e583e` |
 | w2-4-delete-pull-rpc | High | Delete deprecated Pull RPC after w2-3 harvest (owner-decided, wire-breaking OK); port scan_remote_files — absorbed into REV4; delivered as `ue-r2-1h` | `[x]` | master | `2a13f53` |
 | w3-1-memory-aware-buffer-pool | High | BufferPool::for_data_plane(chunk_bytes, streams) owns the formula (streams*2+4, shared 64 KiB DATA_PLANE_BUFFER_FLOOR) + available/4 memory cap with a 2-buffers-per-stream liveness floor (buffer shrinks, never concurrency — the double-buffered sender holds 2); replaces the 3 pasted sites; elastic paths authorize dial.ceiling_max_streams() up front (closes both "growing the pool live is a W3.1 concern" deferrals); fixes the sysinfo units bug (0.38 returns bytes; old *1024 over-reported memory 1024x, making every cap vacuous); RECEIVE_CHUNK_SIZE comment truth. 8 params-layer pins, mutation-verified. Codex PASS (0 findings) | `[x]` | master | `f49f8f6` |
 | w6-1-progress-event-contract | Medium | ProgressEvent contract owned by blit-core: bytes ride Payload only (FileComplete's bytes field DELETED — design-1's class unrepresentable); files count once via byteless FileComplete{wire-relative path} or Payload.files (aggregate lane: delegated bridge, tar-shard appliers); ManifestBatch = direction-flavored denominator, documented. All producers normalized (TCP receive double-emit fixed; tar-shard members + resume lanes gain missing events; send side moves planned bytes to Payload; gRPC pull absolute-path leak fixed; 2 dead emitters conformed pending w8). Consumers collapsed onto shared ProgressTotals (CLI monitor — closes design-1 — + all 3 TUI forwarders; TUI's 3 accumulate_* rules deleted). +12 blit-core tests incl. 4 producer emission pins, 2 mutation-verified; 1460→1472/0/2. Codex PASS (0 findings) | `[x]` | master | `8fd8978` |
 | w6-2-progress-residue-verify | Medium | Verify-then-fix map §1.6 residue: all three claims CONFIRMED at HEAD `8fd8978` (delegated live progress wire-dead — zero BytesProgress producers; daemon row counters fed only by delegated dispatch core.rs:667 — push receive + pull_sync serve stay 0; TransferProgress/GetState/TransferComplete hardcode bytes_total/files_* to 0). Per the ratified spec each confirmed item became its own follow-on: w6-2a/-2b/-2c filed in the pending-review section (independent slices — 2a needs only the already-fed delegated counter; suggested order 2b→2a→2c on smallest-first grounds, coder's pick). Verification + filing only, no code. Codex NEEDS FIXES (2 Low, doc-coherence: "no code anywhere constructs" overstated vs consumer tests; 2b-as-substrate-for-2a wording) → both fixed | `[x]` | master | `0aba593` + fix |
 | w4-4-blocking-work-off-runtime | Medium | Blocking work off the runtime: push manifest requires-upload checks (canonical containment walk + stat, ~3M+ syscalls per 1M-file push, previously inline on a tokio worker) now buffer and run in chunked spawn_blocking batches (MANIFEST_CHECK_CHUNK=128 = need-list early-flush threshold; lexical-containment alternative rejected — weakens F2); need-list order kept, mid-manifest TCP spin-up moved to post-chunk-drain, ManifestComplete drains the remainder, design-4 untouched. collect_pull_entries_with_checksums runs ENTIRELY on one spawn_blocking thread (single-file branch's inline full-file Blake3 + metadata probes were pinning a worker). +4 tests incl. containment-escape via the batched path, mutation-verified; 1472→1476/0/2. Codex NEEDS FIXES (1 Medium: chunk-only draining muted the batcher's 64KiB/5ms early-flush for trickling manifests) → manifest_drain_due chunk-or-delay trigger, fixed `768e7e3` | `[x]` | master | `0feca34`+`768e7e3` |
 | w9-1-ungate-windows-tests | High | Remove blanket #[cfg(unix)] from remote transfer tests with nothing unix-specific | `[x]` | master | `9324559` |
 | w9-2-revive-root-tests | Medium | Relocate dead workspace-root tests/ into blit-core/tests (MirrorPlanner coverage); delete connection.rs; fix AGENTS.md §4 | `[x]` | master | `461525d` |
 | w9-3-test-harness-builder | Medium | One daemon-spawn harness: TestContext::builder() (read_only/delegation/extra_daemon_args) + spawn_daemon/spawn_second_daemon absorb the SEVEN clones at HEAD (audit counted 5; w9-4/w9-5 had each added another — the finding's prediction twice proven) plus 5 cli_bin/7 run_with_timeout/4 ChildGuard copies; daemon build OnceLock'd per test binary (R16-F1 independence kept; was ~75 nested cargo invocations serializing on the build-dir flock — the daemon-spawn load-flakiness home); new blit_core::remote::grpc_server owns the audit-1 HTTP/2 keepalive (30s/20s) as production_server_builder() — daemon main.rs + all FIVE fake tonic servers (not 3: remote_remote ×2, jobs_lifecycle, pull_sync_with_spec_wire ×2) route through it, zero bare Server::builder() left; port-collision race surfaced by the build de-serialization fixed two-layer (process-global claimed-port set + child-death readiness check). Net −1,251 test-tree lines; 1478→1479 same-method A/B (+1 keepalive pin, mutation-verified). Codex NEEDS FIXES (1 Medium: fake-server :0 bind bypassed the claimed set — wrong-listener race for mixed fake/daemon binaries) → claim_port() shared, fixed | `[x]` | master | `f6e592e`+`8641bc6` |
 | w9-4-readonly-enforcement-tests | Medium | Tests for all 3 read-only-module gates (push, purge, delegated pull) — zero coverage today | `[x]` | master | `4d67210` |
 | w7-1-mirror-executor-consolidation | Medium | One mirror/purge deletion executor + parallel enumerate_local_manifest in blit-core (R58-F3 class closure) | `[ ]` | — | — |
 | w7-2-filter-spec-chokepoint | Medium | filter_from_spec pub; push handler uses validated chokepoint (mirror-purge filter currently unvalidated) | `[ ]` | — | — |
 | w7-3-wire-metadata-helpers | Medium | Wire metadata + path helpers into blit-core; one mtime error convention; delete per-crate twins | `[ ]` | — | — |
 | w7-4-hash-reader-helper | Medium | checksum::hash_reader owning the 256 KiB loop; daemon build_file_header calls it | `[x]` | master | `6b2f433` |
 | w7-5-presenter-formatting | Medium | format_bps in blit_app::display (binary units); switch jobs.rs + 5 TUI copies | `[ ]` | — | — |
 | w7-6-default-port-pub | Low | RemoteEndpoint::DEFAULT_PORT pub; delete 9031 literals | `[x]` | master | `de04054` |
 | w8-1-foundation-deadcode-sweep | Medium | Delete tar_stream, delete.rs, copy/parallel+stats, chunked_copy_file, fs_enum leftovers (~800 lines). zero_copy EXCLUDED → w8-1b | `[ ]` | — | — |
 | w8-1b-zero-copy-fast-eval | Medium | Evaluate wiring splice/zero_copy into the receive pipeline (owner: FAST potential); outcome = plan doc or deletion | `[x]` | master | `6189d82` |
 | w8-2-delete-control-plane-payload | Medium | Delete transfer_payloads_via_control_plane (zero-caller duplicate); sequence with W1.1 chunk_bytes deletion | `[ ]` | — | — |
 | w8-3-deadcode-hygiene-sweep | Low | --interval-ms flag, blit-cli unused deps, blit-app stubs, stale #[allow(dead_code)] sweep | `[ ]` | — | — |
 | w5-3-daemon-status-helpers | Medium | internal_err({:#}) + io_to_status helpers; sweep ~69 chain-amputating + 116 Status::internal sites | `[ ]` | — | — |
 | w5-4-mpsc-sendfail-vocabulary | Medium | One honest mpsc send-failure vocabulary; prefer joining the exited task's real error | `[ ]` | — | — |
 | w5-5-logger-trait-cleanup | Low | Logger trait permanently-noop error channel cleanup | `[ ]` | — | — |
 | w9-6-test-misc | Low | Harness stderr capture; tuning-tier unit tests | `[ ]` | — | — |
 | w10-docs-batch | Medium | Docs batch: AGENTS.md ghost names, WORKFLOW_PHASE_2 re-status, --resume/--retry help scoping (help+manpage+README), comment-truth sweep | `[ ]` | — | — |

 ## Currently pending review

 | ID                | Severity | Title                                       | Status | Branch      | Commit    |
 |-------------------|----------|---------------------------------------------|--------|-------------|-----------|
 | relay-1-subpath-double-join | Low | `--relay-via-cli` with a subpath source scans `sub/sub` (endpoint rel_path joined twice). Pre-existing (deleted Pull-RPC code had the identical join); surfaced by the ue-r2-1h self-review panel; port kept parity, fix deferred. **CLOSED AS MOOT at otp-10c-1 (D-2026-07-11-1): the relay path and its scan were deleted with `--relay-via-cli`; nothing joins the rel_path twice because nothing joins it at all** | `[x]` | master | `f53f5a4` |
 | win-1-push-needlist-separators | High | Windows daemon push need-list echoed native separators — every nested push to a Windows daemon stalled 30s. One-line `relative_path_to_posix` fix; reviewed within the ue-r2-1h codex+panel batch | `[x]` | master | `48c5a11` |
 | design-1-cli-pull-byte-double-count | Medium | CLI pull progress double-counts bytes on the TCP data plane (producer reports both Payload and FileComplete with full bytes; CLI fold adds both). From design map §1.6, hand-verified. Fixed structurally by w6-1 (producer double-emit removed AND FileComplete's bytes field deleted — the class is unrepresentable); graded within the w6-1 codex round | `[x]` | master | `8fd8978` |
 | design-2-orphaned-daemon-data-planes | High | Daemon data-plane tasks detach (not abort) on control-stream death at 3 spawn sites; orphan unreachable by CancelJob. AbortOnDrop fix exists but never propagated. From design map §1.9, hand-verified. Fixed by w4-1 (2 of 3 sites deleted with the Pull RPC at ue-r2-1h; remaining push/control.rs site now wrapped); graded within the w4-1 codex round | `[x]` | master | `65ecb93` |
 | design-3-unbounded-data-plane-connects | Medium | Both TCP data-plane connects lacked timeouts (audit-2 fix never reached the data plane); hung 60-127s on black-holed ports. Fixed: shared `socket::dial_data_plane` (bounded connect via DATA_PLANE_ACCEPT_TIMEOUT + w1-2 policy + bounded handshake write via DATA_PLANE_TOKEN_TIMEOUT; TimedOut in the chain → is_retryable transient); both sites collapsed (pull connect_pull_stream incl. resize-ADD, push connect_with_probe incl. elastic). +3 tests incl. deterministic stalled-handshake shape pin, mutation-verified; 1476→1479/0/2. Codex PASS (0 findings) | `[x]` | master | `49dcec6` |
 | w6-2a-delegated-bytesprogress-producer | Medium | Delegated live progress is wire-dead: proto BytesProgress has zero producers — the dst daemon sends Started, silence, then one post-hoc ManifestBatch (delegated_pull.rs:363-369 deliberate 0.1.0 gap, :433). The row atomic is ALREADY fed (core.rs:667); bridge it onto the DelegatedPullProgress stream on the progress tick so CLI footer + TUI delegated pane go live. Client side needs nothing (w6-1 aggregate lane + report_bytes_progress ready). Filed by w6-2 verification | `[ ]` | — | — |
 | w6-2b-daemon-counters-push-pullsync | Medium | Daemon row byte counters stay 0 for served sessions. **Re-scoped at otp-10c-2** (the original prescription targeted the deleted push/pull_sync handlers): the served `Transfer` dispatcher builds its session config without `with_byte_progress`/`ByteProgressSink` (service/transfer.rs module-doc note), so GetState/TransferProgress/TransferComplete report 0 bytes for served-session rows of either role. Wire `job.bytes_counter()` through the served responder config, as `delegated_pull` already does (core.rs) and otp-9a did for the initiated side. Filed by w6-2 verification | `[ ]` | — | — |
 | w6-2c-daemon-progress-denominators | Medium | Daemon event stream has no denominators or file counts: TransferProgress hardcodes bytes_total/files_completed/files_total 0 (core.rs:240-242), TransferComplete.files 0 (:322-325, + tcp_fallback_used false :329), GetState bytes_total 0 (:994-996) — "N of M"/percent impossible for every consumer. Thread manifest totals + a files counter onto ActiveJobs rows. Filed by w6-2 verification | `[ ]` | — | — |
 | design-4-fallback-midmanifest-negotiation | High | Forced-gRPC pushes fail at ≥128 files (FILE_LIST_EARLY_FLUSH_ENTRIES; ~100 flaky). Mechanism VERIFIED two-sided: daemon announced fallback negotiation mid-manifest AND a force_grpc client streamed FileData with no negotiation at all — both racing the daemon manifest loop's FileData rejection. Fixed: daemon early-flush branch TCP-only; client gates fallback sends on fallback_negotiated. Owner-ratified 2026-06-12. NOTE: grade before design-5 (sequential overlapping commits on push/client/mod.rs) | `[x]` | master | `ddfeb58` |
 | design-5-send-failure-masks-rejection | Medium | Push rejection reason (e.g. read-only) masked by 'failed to send push request payload' when the client loses the send-vs-status race — first CI failure surfaced by the w9-1/w9-4 ungating (macOS+Windows). Fixed: prefer_server_error harvests the daemon's terminal status on send failure at the 3 manifest-phase sites; 500-file deterministic regression. Owner-ratified 2026-06-12 ("strong bias for proper fixes"). Grade after design-4 | `[x]` | master | `08d71a2` |
 | audit-h1-mirror-relay-incomplete-scan | Data-loss | Reject `mirror --relay-via-cli` for remote→remote (round 2: gate moved before mirror confirm prompt + yes=false regression test) | `[x]` | `master` | `4467faf` |
 | audit-h3a-push-receive-stall | Robustness | StallGuard on the daemon push-receive socket (`TRANSFER_STALL_TIMEOUT` hoist) — closes one of three remaining stall-guard gaps from R3 H3; symmetric with audit-1c CLI pull-receive | `[x]` | `master` | `dd51a1c` |
 | audit-m28-tui-sot-sweep | Docs | TUI source-of-truth sweep (round 2: audit INDEX + R3 updated to record 2026-06-04 owner ratification of H10b + resolution of L39/M27/M28) | `[x]` | `master` | `15fabbf` |
 | audit-l39-m27-env-var-purge | Convention | Owner-directed env-var purge (round 2: bench-script prose + Clap `hide_short_help` doc corrections in 3 sites) | `[x]` | `master` | `ec06a95` |
 | audit-h11-f1-confirm-detail-err | Data-loss UI | F1 confirm-detail explicit Local/Remote/Err arms + `debug_assert!` (round 2: re-armed at build-fix HEAD; h11 logic itself was correct in dirty tree, blocked by uncommitted Phase 6 dual-pane modules) | `[x]` | `master` | `1b3cb39` |
 | audit-h3b-pull-data-plane-write-stall | Robustness | New `StallGuardWriter<W>` wired inside `DataPlaneSession` (round 2: same build-fix re-arm + tightened `Ok(0)` semantics so a zero-byte poll_write doesn't reset the deadline) | `[x]` | `master` | `1b3cb39` |
 | audit-h3c-slice1-grpc-fallback-frame-contract | Robustness | Slice 1 of 2: gRPC fallback chunk cap at 1 MiB (`GRPC_FALLBACK_CHUNK_BYTES`) decoupled from TCP tuning; 3 CLI pull receive sites routed through `recv_fallback_message` (the chokepoint slice 2 will wrap with the dynamic progress watchdog). Round-2 adversarial concerns all addressed. Verified 2026-06-11 (owner accept; review assessment found the cap also fixes the tonic 4 MiB decode-limit failure — see DEVLOG, feeds slice-2 re-scope) | `[x]` | `master` | `bf4cc82` |
 | d-62-f1-trigger-error | Feature | Inline validation feedback in the F1 trigger modal (round 2) | `[x]` | `phase5/a1` | `0b47a72` |
 | d-63-f1-push-progress | Feature | Live byte/file footer for the F1 push (round 2) | `[x]` | `phase5/a1` | `aba54f8` |
 | d-64-f1-push-ttl | Feature | Auto-hide the F1 push outcome footer (round 2) | `[x]` | `phase5/a1` | `2f67e96` |
 | d-65-f1-push-mirror-move | Feature | Mirror/move for the F1 push direction (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
 | d-66-f4-clear-confirm | Feature | y/N gate on the F4 profile-history clear (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
 | d-67-help-clear-confirm | Feature | Flag the F4 clear y/N confirm in the `?` keymap (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
 | d-68-f1-remote-remote-copy | Feature | Remote→remote delegated copy from the F1 trigger (round 4) | `[x]` | `phase5/a1` | `c93bcd6` |
 | d-69-f1-delegated-progress | Feature | Live byte/file footer for remote→remote delegated copy | `[x]` | `phase5/a1` | `2f1f5d2` |
 | d-70-f1-delegated-mirror | Feature | Remote→remote delegated mirror from the F1 trigger | `[x]` | `phase5/a1` | `0b98666` |
 | d-71-f1-delegated-move | Feature | Remote→remote delegated move from the F1 trigger (round 3) | `[x]` | `phase5/a1` | `57ed8e9` |
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-NGVqjBuv' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-6IiOjCK6' (errno=Operation not permitted)
tracked_target_diff_rc=0
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-1SL9gab1' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-XjKeChbs' (errno=Operation not permitted)
syntax_rc=0

exec
/bin/zsh -lc "git grep -n 'session_phase_trace\\|BLIT_TRACE_SESSION_PHASES\\|bind_session_phase\\|BoundSessionPhaseTrace' 75a9a33ce600e4707438ed885de2ce0cdf27d946 -- crates/blit-core/src crates/blit-daemon/src | head -300; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '620,730p'; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '1290,1390p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rGmrEEqw' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-0XFcX1IR' (errno=Operation not permitted)
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/data_plane.rs:66:    phase_trace: Option<super::session_phase::BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/data_plane.rs:194:    pub(crate) fn with_session_phase_trace(
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/data_plane.rs:196:        trace: Option<super::session_phase::BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/data_plane.rs:219:    ) -> Option<super::session_phase::BoundSessionPhaseTrace> {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/pipeline.rs:447:    phase_trace: Option<crate::remote::transfer::session_phase::BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_client.rs:180:            session_phase_trace: Default::default(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_client.rs:343:            session_phase_trace: Default::default(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_phase.rs:5://! Production emission is enabled with `BLIT_TRACE_SESSION_PHASES=1` and
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_phase.rs:15:const TRACE_ENV: &str = "BLIT_TRACE_SESSION_PHASES";
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_phase.rs:217:    ) -> Option<BoundSessionPhaseTrace> {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_phase.rs:220:        Some(BoundSessionPhaseTrace {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_phase.rs:250:pub(crate) struct BoundSessionPhaseTrace {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/remote/transfer/session_phase.rs:265:impl BoundSessionPhaseTrace {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:57:use crate::remote::transfer::session_phase::{BoundSessionPhaseTrace, SessionPhaseFields};
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:238:        phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:261:        phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:443:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:593:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:608:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:835:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:852:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:907:        let session = session.with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:974:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:1025:        .with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:1070:    pub(super) fn phase_trace(&self) -> Option<&BoundSessionPhaseTrace> {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:1161:                session.with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:1195:                .with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs:1234:            self.phase_trace.as_ref().map(BoundSessionPhaseTrace::stamp)
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/local.rs:620:            session_phase_trace: Default::default(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/local.rs:894:                session_phase_trace: Default::default(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:50:    BoundSessionPhaseTrace, SessionPhaseFields, SessionPhaseRole, SessionPhaseTrace,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:223:    /// `BLIT_TRACE_SESSION_PHASES=1` on each endpoint.
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:224:    pub session_phase_trace: SessionPhaseTrace,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:280:    pub session_phase_trace: SessionPhaseTrace,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:669:fn bind_session_phase_trace(
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:673:) -> Option<BoundSessionPhaseTrace> {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:695:async fn flush_session_phase_trace(trace: Option<&BoundSessionPhaseTrace>) {
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:1079:    let phase_trace = bind_session_phase_trace(
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:1080:        instruments.session_phase_trace.clone(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:1157:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:1319:    phase_trace: Option<BoundSessionPhaseTrace>,
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:1749:            flush_session_phase_trace(phase_trace.as_ref()).await;
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:2820:    let phase_trace = bind_session_phase_trace(
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:2821:        instruments.session_phase_trace.clone(),
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:3672:                flush_session_phase_trace(phase_trace.as_ref()).await;
75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs:3804:    phase_trace: Option<&BoundSessionPhaseTrace>,
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-JugoN9QL' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rUnwR5Sn' (errno=Operation not permitted)
   620	        }
   621	    }
   622	    Ok(())
   623	}
   624
   625	/// Flips an abort flag when dropped, so a blocking-pool pass whose
   626	/// awaiting future is dropped (client disconnect, CancelJob) stops at
   627	/// its next flag check instead of running to completion behind a dead
   628	/// session. Introduced for the mirror delete pass (codex otp-9b F2);
   629	/// the destination diff's hash chunks share it (codex otp-10b-1 F3).
   630	struct AbortFlagOnDrop(Arc<AtomicBool>);
   631	impl Drop for AbortFlagOnDrop {
   632	    fn drop(&mut self) {
   633	        self.0.store(true, Ordering::Release);
   634	    }
   635	}
   636
   637	/// Operator policy a serving responder applies to every session it
   638	/// accepts (otp-10a F3 / otp-10b-1). Defaults are the permissive
   639	/// non-daemon posture; the daemon fills it from its runtime config.
   640	#[derive(Clone, Copy, Default)]
   641	pub struct ResponderPolicy {
   642	    /// `--force-grpc-data`: never grant a TCP data plane — every
   643	    /// served session rides the in-stream carrier regardless of what
   644	    /// the initiator asked for.
   645	    pub force_in_stream: bool,
   646	    /// `--no-server-checksums`: refuse `COMPARISON_MODE_CHECKSUM`
   647	    /// opens with `CHECKSUM_DISABLED` instead of hashing (or silently
   648	    /// degrading the compare).
   649	    pub refuse_checksum_compare: bool,
   650	}
   651
   652	/// Outcome of the HELLO + OPEN phases.
   653	struct Negotiated {
   654	    open: SessionOpen,
   655	    /// The responder's reply. The SOURCE initiator reads
   656	    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
   657	    accept: SessionAccept,
   658	    /// The write root a Responder's [`OpenResolver`] produced from the
   659	    /// received open, if one was supplied; `None` for an Initiator or a
   660	    /// fixed-root Responder (the caller supplies the root then).
   661	    resolved_root: Option<PathBuf>,
   662	    /// The bound data-plane listener + credentials a DESTINATION
   663	    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
   664	    /// on an Initiator, or when the responder granted no data plane
   665	    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
   666	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
   667	}
   668
   669	fn bind_session_phase_trace(
   670	    trace: SessionPhaseTrace,
   671	    negotiated: &Negotiated,
   672	    endpoint_role: SessionPhaseRole,
   673	) -> Option<BoundSessionPhaseTrace> {
   674	    let session_token = negotiated
   675	        .responder_data_plane
   676	        .as_ref()
   677	        .map(data_plane::ResponderDataPlane::session_token)
   678	        .or_else(|| {
   679	            negotiated
   680	                .accept
   681	                .data_plane
   682	                .as_ref()
   683	                .map(|grant| grant.session_token.as_slice())
   684	        })?;
   685	    let initiator_role = match TransferRole::try_from(negotiated.open.initiator_role).ok()? {
   686	        TransferRole::Source => SessionPhaseRole::Source,
   687	        TransferRole::Destination => SessionPhaseRole::Destination,
   688	        TransferRole::Unspecified => return None,
   689	    };
   690	    trace
   691	        .or_from_env()
   692	        .bind(session_token, endpoint_role, initiator_role)
   693	}
   694
   695	async fn flush_session_phase_trace(trace: Option<&BoundSessionPhaseTrace>) {
   696	    let Some(trace) = trace.cloned() else {
   697	        return;
   698	    };
   699	    let _ = tokio::task::spawn_blocking(move || trace.flush()).await;
   700	}
   701
   702	/// HELLO both ways, exact match (D-2026-07-05-2). First frame each
   703	/// direction; no ordering between the two directions. Factored out so a
   704	/// serving end (`run_responder`) can exchange HELLO, then read the OPEN
   705	/// and dispatch on the declared role before running a role driver.
   706	async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) -> Result<()> {
   707	    transport
   708	        .send(frame(Frame::Hello(SessionHello {
   709	            build_id: hello.build_id.clone(),
   710	            contract_version: hello.contract_version,
   711	        })))
   712	        .await?;
   713
   714	    let peer_hello = match expect_frame(transport).await? {
   715	        Frame::Hello(h) => h,
   716	        other => {
   717	            return Err(notify_and_wrap(
   718	                transport,
   719	                SessionFault::protocol_violation(format!(
   720	                    "expected SessionHello, got {}",
   721	                    frame_name(&Some(other))
   722	                )),
   723	            )
   724	            .await)
   725	        }
   726	    };
   727
   728	    if peer_hello.build_id != hello.build_id
   729	        || peer_hello.contract_version != hello.contract_version
   730	    {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-MochdxcL' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-B3H8D80Y' (errno=Operation not permitted)
  1290	                return;
  1291	            }
  1292	        }
  1293	    }
  1294	}
  1295
  1296	/// otp-7a: the send half's resume bookkeeping. A resume-flagged need is
  1297	/// HELD until its `BlockHashList` arrives (the contract's strict
  1298	/// ordering — the source must not send a byte of that file first); the
  1299	/// correlated pair then queues for the block phase.
  1300	#[derive(Default)]
  1301	struct ResumeSendState {
  1302	    held: HashMap<String, FileHeader>,
  1303	    ready: Vec<(FileHeader, BlockHashList)>,
  1304	}
  1305
  1306	#[allow(clippy::too_many_arguments)]
  1307	async fn source_send_half(
  1308	    plan_options: PlanOptions,
  1309	    data_plane_host: Option<&str>,
  1310	    instruments: SourceInstruments,
  1311	    negotiated: &Negotiated,
  1312	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
  1313	    tx: &mut Box<dyn FrameTx>,
  1314	    source: Arc<dyn TransferSource>,
  1315	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
  1316	    manifest_sent: &AtomicBool,
  1317	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
  1318	    mut fault_signal: watch::Receiver<Option<SessionFault>>,
  1319	    phase_trace: Option<BoundSessionPhaseTrace>,
  1320	) -> Result<TransferSummary> {
  1321	    let mut pending: Vec<FileHeader> = Vec::new();
  1322	    let mut resume: ResumeSendState = ResumeSendState::default();
  1323	    let mut need_complete = false;
  1324
  1325	    // Data plane (otp-4b/5b): set up the send sockets up front — BEFORE
  1326	    // streaming the manifest — so the peer sees the connections promptly
  1327	    // rather than waiting out a bounded-accept/connect timeout while a long
  1328	    // manifest streams. Which end connects depends on connection role
  1329	    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
  1330	    // listener; a SOURCE **initiator** (push) dials the grant it received.
  1331	    // Byte direction is the same either way (SOURCE sends), so both yield a
  1332	    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
  1333	    // in-stream carrier (fallback), which needs no early setup.
  1334	    let mut data_plane = match responder_data_plane {
  1335	        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
  1336	        // initiator advertised its capacity in the open (byte RECEIVER
  1337	        // advertises, wherever it initiates); the accept plane is single-
  1338	        // stream (otp-5b-1).
  1339	        Some(bound) => Some(
  1340	            data_plane::accept_source_data_plane(
  1341	                bound,
  1342	                negotiated.open.receiver_capacity.as_ref(),
  1343	                Arc::clone(&source),
  1344	                &instruments,
  1345	                phase_trace.clone(),
  1346	            )
  1347	            .await?,
  1348	        ),
  1349	        // SOURCE initiator (push, otp-4b): dial the grant if the responder
  1350	        // granted a data plane; else in-stream.
  1351	        None => match &negotiated.accept.data_plane {
  1352	            Some(grant) => {
  1353	                let host = data_plane_host.ok_or_else(|| {
  1354	                    eyre::Report::new(SessionFault::internal(
  1355	                        "responder granted a TCP data plane but this initiator has no host to dial",
  1356	                    ))
  1357	                })?;
  1358	                Some(
  1359	                    data_plane::dial_source_data_plane(
  1360	                        host,
  1361	                        grant,
  1362	                        negotiated.accept.receiver_capacity.as_ref(),
  1363	                        Arc::clone(&source),
  1364	                        &instruments,
  1365	                        phase_trace.clone(),
  1366	                    )
  1367	                    .await?,
  1368	                )
  1369	            }
  1370	            None => None,
  1371	        },
  1372	    };
  1373
  1374	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
  1375	    // fed to the shape table so the SOURCE grows the data-plane stream
  1376	    // count as the workload's shape becomes known. Append-only (a need is
  1377	    // counted once, when it arrives), and the in-flight resize record the
  1378	    // ack is matched against (at most one — the dial enforces it).
  1379	    let mut needed_bytes: u64 = 0;
  1380	    let mut needed_count: usize = 0;
  1381	    let mut pending_resize: Option<data_plane::PendingResize> = None;
  1382
  1383	    // Streaming manifest: entries go out as enumeration produces them
  1384	    // (immediate start in every direction — plan §Design 2). The open
  1385	    // carries no source path (the source end owns its local endpoint) but
  1386	    // does carry the include/exclude/size/age filter (otp-6a): only
  1387	    // matching files are manifested and transferred. The filter MUST ride
  1388	    // the wire (not be pre-wrapped by a local caller) because for pull the
  1389	    // SOURCE is the remote daemon responder — it, not the client, owns the
  1390	    // scan. Apply it through the universal `FilteredSource` decorator, the

exec
/bin/zsh -lc "git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/data_plane.rs | nl -ba | sed -n '130,280p'; git show 75a9a33ce600e4707438ed885de2ce0cdf27d946:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '2920,3025p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-YluGkRP6' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2jD7c1XD' (errno=Operation not permitted)
   130	    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
   131	        Ok(listener) => listener,
   132	        Err(err) => {
   133	            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
   134	            return None;
   135	        }
   136	    };
   137	    let port = match listener.local_addr() {
   138	        Ok(addr) => addr.port(),
   139	        Err(err) => {
   140	            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
   141	            return None;
   142	        }
   143	    };
   144	    // Two independent 16-byte credentials (contract §Transport: a socket
   145	    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
   146	    // is the fallible-RNG minter — a missing system RNG is an error, not
   147	    // a weaker credential.
   148	    let session_token = match generate_sub_token() {
   149	        Ok(token) => token,
   150	        Err(err) => {
   151	            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
   152	            return None;
   153	        }
   154	    };
   155	    let epoch0_sub_token = match generate_sub_token() {
   156	        Ok(token) => token,
   157	        Err(err) => {
   158	            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
   159	            return None;
   160	        }
   161	    };
   162	    // The grant is issued before any manifest is seen, so the proposal
   163	    // has zero knowledge: initial_streams == 1. All growth is via resize
   164	    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
   165	    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   166	    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
   167	    Some(ResponderDataPlane {
   168	        listener,
   169	        session_token,
   170	        epoch0_sub_token,
   171	        initial_streams,
   172	        port,
   173	    })
   174	}
   175
   176	/// Aggregated destination-side receive result: the write outcome plus
   177	/// the number of data sockets accepted (epoch-0 + accepted resizes),
   178	/// which IS the settled live stream count this end observed. The sf-2
   179	/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
   180	pub(super) struct ReceiveTotals {
   181	    pub(super) outcome: SinkOutcome,
   182	    pub(super) streams: usize,
   183	}
   184
   185	/// Live handle to a running responder data plane. The control loop arms
   186	/// resize credentials through [`Self::arm`] and joins the accept loop at
   187	/// `SourceDone` via [`Self::finish`].
   188	pub(super) struct ResponderDataPlaneRun {
   189	    arm_tx: mpsc::UnboundedSender<ResizeArm>,
   190	    task: AbortOnDrop<Result<ReceiveTotals>>,
   191	    /// The `session_token` half of every socket credential (the control
   192	    /// loop does not need it, but keeping it here documents the shape).
   193	    #[allow(dead_code)]
   194	    session_token: Vec<u8>,
   195	    /// The receiver's advertised `max_streams` — the control loop refuses
   196	    /// a resize that would grow past it (defense in depth; the source's
   197	    /// dial already clamps to the same ceiling).
   198	    pub(super) ceiling: usize,
   199	}
   200
   201	struct ResizeArm {
   202	    epoch: u32,
   203	    sub_token: Vec<u8>,
   204	}
   205
   206	impl ResponderDataPlane {
   207	    pub(super) fn session_token(&self) -> &[u8] {
   208	        &self.session_token
   209	    }
   210
   211	    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
   212	    pub(super) fn grant(&self) -> DataPlaneGrant {
   213	        DataPlaneGrant {
   214	            tcp_port: self.port as u32,
   215	            session_token: self.session_token.clone(),
   216	            initial_streams: self.initial_streams,
   217	            epoch0_sub_token: self.epoch0_sub_token.clone(),
   218	        }
   219	    }
   220
   221	    /// The epoch-0 stream count this responder granted (always 1 — the
   222	    /// zero-knowledge proposal). The control loop seeds its `resize_live`
   223	    /// counter from it.
   224	    pub(super) fn initial_streams(&self) -> u32 {
   225	        self.initial_streams
   226	    }
   227
   228	    /// Spawn the accept+receive loop and return a live handle. The loop
   229	    /// accepts the epoch-0 socket(s) immediately, then accepts one more
   230	    /// socket per armed resize credential until the control loop signals
   231	    /// `SourceDone` (drops the arm sender) and every receive worker has
   232	    /// drained its END. Runs concurrently with the control-stream diff
   233	    /// loop; the DESTINATION is the scorer, so it returns the totals.
   234	    pub(super) fn spawn(
   235	        self,
   236	        sink: Arc<dyn TransferSink>,
   237	        progress: Option<RemoteTransferProgress>,
   238	        phase_trace: Option<BoundSessionPhaseTrace>,
   239	    ) -> ResponderDataPlaneRun {
   240	        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   241	        let session_token = self.session_token.clone();
   242	        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<ResizeArm>();
   243	        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(
   244	            sink,
   245	            progress,
   246	            phase_trace,
   247	            arm_rx,
   248	        )));
   249	        ResponderDataPlaneRun {
   250	            arm_tx,
   251	            task,
   252	            session_token,
   253	            ceiling,
   254	        }
   255	    }
   256
   257	    async fn accept_loop(
   258	        self,
   259	        sink: Arc<dyn TransferSink>,
   260	        progress: Option<RemoteTransferProgress>,
   261	        phase_trace: Option<BoundSessionPhaseTrace>,
   262	        arm_rx: mpsc::UnboundedReceiver<ResizeArm>,
   263	    ) -> Result<ReceiveTotals> {
   264	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   265	        let mut epoch0 = self.session_token.clone();
   266	        epoch0.extend_from_slice(&self.epoch0_sub_token);
   267
   268	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   269	        let mut total = SinkOutcome::default();
   270	        let mut streams = 0usize;
   271
   272	        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
   273	        // grant is always 1; the loop handles N for symmetry).
   274	        for socket_id in 0..self.initial_streams {
   275	            if let Some(trace) = &phase_trace {
   276	                trace.event(
   277	                    "socket_accept_begin",
   278	                    SessionPhaseFields {
   279	                        epoch: Some(0),
   280	                        socket: Some(socket_id),
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-dbC4PIyR' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-WkaRbBEV' (errno=Operation not permitted)
  2920	    // never re-open a grant (a duplicate manifest path is granted at
  2921	    // most once regardless of delivery timing). `outstanding` is the
  2922	    // not-yet-delivered COMPLETION set — inserted for each freshly
  2923	    // granted path before its NeedBatch, claimed by both carriers (the
  2924	    // in-stream arms inline, the data-plane NeedListSink as payloads
  2925	    // land), and empty at SourceDone. A count proxy was insufficient
  2926	    // (F1); merging the two into one set raced the data-plane claim
  2927	    // against the diff (fix-review F1).
  2928	    let mut granted: HashSet<String> = HashSet::new();
  2929	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  2930
  2931	    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
  2932	    // bytes arrive on sockets (not the control lane). Set it up NOW —
  2933	    // concurrent with the diff loop below, and before the peer sends — so
  2934	    // the connections are established promptly. Which end connects depends
  2935	    // on connection role (otp-5b): a DESTINATION **responder** (push)
  2936	    // accepts sockets off its bound listener; a DESTINATION **initiator**
  2937	    // (pull) dials the grant it received on `data_plane_host`. Byte
  2938	    // direction is the same either way (DESTINATION receives). The
  2939	    // NeedListSink gives the socket receive the same need-list strictness
  2940	    // the in-stream control loop applies inline; AbortOnDrop (inside the
  2941	    // responder run) bounds the accept task to this future. `resize_live`
  2942	    // tracks the stream count this end has grown to (epoch-0 plus each
  2943	    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
  2944	    // max_streams — both directions resize (push arms+accepts, otp-4b-2;
  2945	    // pull dials, otp-5b-2), so both seed these from their epoch-0 streams.
  2946	    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  2947	        Arc::clone(&sink),
  2948	        Arc::clone(&outstanding),
  2949	        // otp-7b: only a resume session accepts block records on the
  2950	        // data plane; the sink validates + claims them against the same
  2951	        // shared grant state the in-stream arms use.
  2952	        resume_enabled.then(|| data_plane::ResumeRecv {
  2953	            headers: Arc::clone(&resume_headers),
  2954	            resumed: Arc::clone(&files_resumed),
  2955	        }),
  2956	    ));
  2957	    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
  2958	        match negotiated.responder_data_plane {
  2959	            // DESTINATION responder (push, otp-4b): accept + receive.
  2960	            Some(rdp) => {
  2961	                let initial = rdp.initial_streams() as usize;
  2962	                let run = rdp.spawn(recv_sink, progress.clone(), phase_trace.clone());
  2963	                let ceiling = run.ceiling;
  2964	                (
  2965	                    Some(data_plane::DestRecvPlane::Responder(run)),
  2966	                    initial,
  2967	                    ceiling,
  2968	                )
  2969	            }
  2970	            // DESTINATION initiator (pull, otp-5b): dial + receive when the
  2971	            // SOURCE responder granted a data plane and we have a host to dial.
  2972	            None => match (&negotiated.accept.data_plane, data_plane_host) {
  2973	                (Some(grant), Some(host)) => {
  2974	                    let initial = grant.initial_streams.max(1) as usize;
  2975	                    let run = data_plane::dial_destination_data_plane(
  2976	                        host,
  2977	                        grant,
  2978	                        recv_sink,
  2979	                        progress.clone(),
  2980	                        instruments.trace_data_plane,
  2981	                        phase_trace.clone(),
  2982	                    )
  2983	                    .await?;
  2984	                    // otp-5b-2: the pull data plane resizes too. Seed
  2985	                    // `resize_live` from the epoch-0 streams dialed and bound
  2986	                    // growth by the capacity THIS end advertised in its open
  2987	                    // (it is the byte receiver) — the exact ceiling the SOURCE
  2988	                    // responder's dial already clamps to, so both ends agree
  2989	                    // even when the caller advertised a max_streams below this
  2990	                    // host's fresh local reading (codex otp-5b-2 F1). On a
  2991	                    // Resize frame the initiator dials the epoch-N socket (vs
  2992	                    // the responder path's arm).
  2993	                    let ceiling = crate::dial::receiver_stream_ceiling(
  2994	                        negotiated.open.receiver_capacity.as_ref(),
  2995	                    );
  2996	                    (
  2997	                        Some(data_plane::DestRecvPlane::Initiator(run)),
  2998	                        initial,
  2999	                        ceiling,
  3000	                    )
  3001	                }
  3002	                // A grant with no host to dial is an inconsistent initiator
  3003	                // config: fail fast, mirroring the SOURCE initiator
  3004	                // (`source_send_half`). The SOURCE responder has already bound
  3005	                // and blocks accepting the socket this end would dial, so
  3006	                // silently taking the in-stream branch cannot fall back — it
  3007	                // would deadlock until the responder's accept times out. A
  3008	                // grant means the initiator MUST dial (contract §Transport).
  3009	                // (codex otp-5b-1 finding.)
  3010	                (Some(_), None) => {
  3011	                    return Err(eyre::Report::new(SessionFault::internal(
  3012	                        "responder granted a TCP data plane but this DESTINATION \
  3013	                     initiator has no host to dial",
  3014	                    )))
  3015	                }
  3016	                // No grant (the responder could not bind, or the initiator
  3017	                // asked for in-stream): the in-stream carrier.
  3018	                (None, _) => (None, 0usize, 0usize),
  3019	            },
  3020	        };
  3021
  3022	    // otp-7a/7b: the DESTINATION chooses the resume block size (plan D5
  3023	    // — it hashes first; the SOURCE reads the size from each
  3024	    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
  3025	    // (D-2026-07-10-1 in-stream, D-2026-07-10-2 data plane) — decided

codex
- `scripts/bench_otp12pf_rigw.sh:1370` / `:1770` — **High** — The helper is not content-bound to the reviewed Git blob. Checkout cleanliness is checked before several lengthy endpoint gates; staging later hashes whatever bytes then occupy the worktree. If the helper changes during that window to a no-op emitting `standby-purged`, its new hash is adopted, manifested, and accepted by every per-arm check, admitting warm-cache evidence. The offline mocks also do not test this Git-to-staged-content coupling.

VERDICT: NEEDS FIXES
tokens used
409,299
- `scripts/bench_otp12pf_rigw.sh:1370` / `:1770` — **High** — The helper is not content-bound to the reviewed Git blob. Checkout cleanliness is checked before several lengthy endpoint gates; staging later hashes whatever bytes then occupy the worktree. If the helper changes during that window to a no-op emitting `standby-purged`, its new hash is adopted, manifested, and accepted by every per-arm check, admitting warm-cache evidence. The offline mocks also do not test this Git-to-staged-content coupling.

VERDICT: NEEDS FIXES
