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
session id: 019f657c-e8c9-7470-b01d-2462f2f42d90
--------
user
Review the complete immutable diff 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524 for otp12-pf1-rigw-harness. This is a high-stakes q to netwatch-01 benchmark instrument governed by docs/plan/OTP12_PERF_FINDINGS.md and .review/findings/otp12-pf1-rigw-harness.md. Inspect the actual diff and current source, not only the last commit. Check correctness and regressions, exact fixed 128-arm schedule, one Transfer RPC per arm, SOURCE always sends and DESTINATION always receives under either caller, role-independent worker targets, identical physical source and destination paths across initiator roles, q-arrival settle anchoring with teardown charged, destination-keyed durability, exact evidence and analyzer rules, standalone LAUNCHER_SMOKE control flow, CIM PID journal gate and identity-scoped recovery, cleanup and failure preservation, no endpoint-policy mutation, and mutation-sensitive offline guards. Specifically verify G3 path parity assertions and G4 destination/finalization/cleanup/marker/signal guards fail closed under macOS Bash 3.2 and are nonvacuous. Distinguish material observable defects from style. Output a concise markdown findings list with file:line, severity, predicted failure, and rationale, then exactly one final VERDICT: PASS or VERDICT: NEEDS FIXES. A no-finding PASS is valid. Do not invoke skills and do not touch live endpoints.
codex
I’ll review the immutable range and current source read-only, with particular attention to G3/G4 fail-closed behavior on Bash 3.2 and whether the offline guards actually detect mutations. I’ll report only material observable defects, then the required single verdict line.
exec
/bin/zsh -lc "sed -n '1,260p' AGENTS.md
sed -n '1,300p' .agents/repo-guidance.md
sed -n '1,260p' .agents/state.md
git status --short --branch
git rev-parse 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d 6f517ea1bdbea2f7d83f15c086d2bf5f764cf524
git diff --stat 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524
git diff --name-status 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
git: error: couldn't create cache file '/tmp/xcrun_db-xDndbDns' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-cVTL3kkm' (errno=Operation not permitted)
## master...origin/master [ahead 55]
?? .review/results/otp12-pf1-rigw-harness-r4.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ihiSRaUf' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-u65GR9q8' (errno=Operation not permitted)
4c7c7544db69289cf2e5fc0cf21093b40f00bc0d
6f517ea1bdbea2f7d83f15c086d2bf5f764cf524
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ChueJW2E' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-a60qrXfo' (errno=Operation not permitted)
 .agents/machines.md                                |     7 +
 .review/findings/otp12-pf1-rigw-harness.md         |   291 +
 .review/results/otp12-pf1-rigw-harness-r2.codex.md | 20683 +++++++++++++++++++
 .../otp12-pf1-rigw-harness-r2.gpt-verdict.md       |    81 +
 .../otp12-pf1-rigw-harness-r2.grok-verdict.md      |    30 +
 .../results/otp12-pf1-rigw-harness-r2.grok.json    |    32 +
 .review/results/otp12-pf1-rigw-harness-r3.codex.md |   125 +
 .../otp12-pf1-rigw-harness-r3.gpt-verdict.md       |    27 +
 .../otp12-pf1-rigw-harness-r3.grok-verdict.md      |    47 +
 .../results/otp12-pf1-rigw-harness-r3.grok.json    |    36 +
 .review/results/otp12-pf1-rigw-harness.codex.md    |     7 +
 .../results/otp12-pf1-rigw-harness.gpt-verdict.md  |    72 +
 DEVLOG.md                                          |     1 +
 REVIEW.md                                          |     1 +
 .../blit-core/src/transfer_session/data_plane.rs   |     9 +-
 crates/blit-core/tests/transfer_session_roles.rs   |     3 +
 scripts/bench_otp12pf_rigw.sh                      |  2460 +++
 scripts/otp12pf_rigw_analyze.py                    |  2020 ++
 scripts/otp12pf_rigw_analyze_test.py               |  1069 +
 19 files changed, 26996 insertions(+), 5 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-e9CD05lR' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-e7K8ZqNS' (errno=Operation not permitted)
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
A	.review/results/otp12-pf1-rigw-harness.codex.md
A	.review/results/otp12-pf1-rigw-harness.gpt-verdict.md
M	DEVLOG.md
M	REVIEW.md
M	crates/blit-core/src/transfer_session/data_plane.rs
M	crates/blit-core/tests/transfer_session_roles.rs
A	scripts/bench_otp12pf_rigw.sh
A	scripts/otp12pf_rigw_analyze.py
A	scripts/otp12pf_rigw_analyze_test.py

exec
/bin/zsh -lc 'rg -n "''^(#|##|###)|otp12-pf1|RIGW|128|Transfer|SOURCE|DESTINATION|settle|durab|LAUNCHER_SMOKE|journal|endpoint|G3|G4|evidence|analy" docs/plan/OTP12_PERF_FINDINGS.md' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "sed -n '1,260p' docs/STATE.md
sed -n '1,360p' .review/findings/otp12-pf1-rigw-harness.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
1:# otp-12 perf findings — investigate + fix before acceptance (design)
15:factual claim was settled by *measurement* (the same-OS rig refuted a
25:decision. pf-1 proceeds either way: it produces the evidence that
50:## The two findings (evidence, both committed)
72:**What the evidence actually supports — and the confound it does NOT
78:Every invariance cell compares two arms that share the same endpoints
105:### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)
141:     same-OS result is the honest evidence base: criterion 1 asks for
163:> on a scratch probe (and a first harness revision) that ran the durability
165:> initiator is the SOURCE, which only read, so its sync was a no-op and the
168:> durability the other got free — multi-second on skippy's ZFS — which
172:> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
177:### The residual confound (WHICH code) still needs a counterfactual
240:on zoey — that PASS must not be read as absence or masking evidence.
282:## pf-0 — the environmental control (MTU): **KILLED as a material cause of P1** (recorded 2026-07-14)
285:(`docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md`); evidence + full
379:## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
384:  session the SOURCE is the responder: each sf-2 resize epoch is
385:  ACCEPTED off the source's listener while the DESTINATION dials
400:  claim. Only the dial/accept inversion counterfactual in pf-1 can settle H1.
416:  layouts drain the same fixed 128-entry destination need loop, so
442:  opened at one stream (after its 128-file early flush) then resized
474:  FsTransferSink → disk`
513:## Method (the investigation slice — no behavior changes)
601:## pf-1 decision rule — UNIFORM, pre-registered (added round 5)
655:## Fix criteria (pre-registered; the owner walks the final numbers)
697:## Staging (each through the codex loop)
703:  pre-pf-1 evidence.
709:  D (delegated, netwatch-01↔skippy)**. **No mixed-build evidence: every
715:  which are **replication and control evidence, not acceptance
716:  evidence**.
721:  evidence at all. "Not implicated" scopes what pf-1 must
734:  dated evidence dirs. **Then** otp-12d assembles the matrix from
737:## Known gaps
764:  **not** waive the parent plan's delegated-parity bar, whose evidence

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
# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows

**Slice**: OTP12 performance-finding pf-1, P1 rig harness only.
**Status**: G3 and G4 fixed; fresh complete review pending.

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
  to the arm's durable total before comparison.
- Compute paired differences `d_i = destination_init_i − source_init_i`, the
  registered split drifts, role-order drift, the full paired range that guards
  the known bimodal fast arm, trace observer bias, and conservative
  `N_resolution`. Reports retain every sorted arm/difference distribution and
  use only per-endpoint monotonic clocks for phase intervals. Cross-host clock
  samples quantify uncertainty and are never silently subtracted.

## Files

- `crates/blit-core/src/transfer_session/data_plane.rs` — SOURCE dial
  trace attachment now follows the matching dial-end marker at epoch zero
  and every resize epoch.
- `crates/blit-core/tests/transfer_session_roles.rs` — both initiator layouts
  pin action-end before attachment on both endpoint roles.
- `scripts/bench_otp12pf_rigw.sh` — q-side registered runner and endpoint
  gates.
- `scripts/otp12pf_rigw_analyze.py` — exact schedule, trace, clock, phase, and
  resolution validator/reporter.
- `scripts/otp12pf_rigw_analyze_test.py` — complete synthetic session and
  fail-closed mutations.
- `.agents/machines.md` — current direction-specific MSS and q SSH endpoint
  fact.

## Tests

- `SELFTEST=1 bash scripts/bench_otp12pf_rigw.sh` proves the exact block/arm
  inventory and canonical path construction without contacting either rig
  endpoint. Every path assertion has an explicit failure path because macOS
  Bash 3.2 does not reliably apply `set -e` to bare `[[ ... ]]` commands.
- `python3 scripts/otp12pf_rigw_analyze_test.py` builds complete synthetic
  evidence (128 arms, 768 clock samples, split client/daemon phase logs) and
  rejects missing clock rows, missing endpoint trace, trace-off leakage,
  gRPC trace leakage, schedule drift, sequence gaps, and terminal/inventory
  corruption. It pins the split/range/role-order/observer resolution math and
  all exported reports.
- The same self-test runs under q's actual macOS Bash and Python so Bash 3.2
  and platform behavior are exercised, not inferred from nagatha.
- Mutation proof: removing role-order drift and the full paired-range term from
  `N_pair` makes the synthetic diagnostic fail (`N_resolution` falls from 70
  ms to 40 ms); restoring them returns the analyzer suite to green.
- Mutation proof: excluding successful client logs from trace discovery makes
  the synthetic diagnostic fail on a missing SOURCE/DESTINATION endpoint;
  restoring both client and daemon evidence roots returns all tests to green.
- Mutation proof: reducing the clock-row formatter from 12 fields to 11 makes
  the harness self-test fail before analysis; restoring the exact 12-column
  schema returns the local and q/macOS self-tests to green.
- The analyzer rejects a missing `settled_ms` column, non-integer values, and
  values outside `[250,1000)`. Synthetic evidence supplies the lower valid
  bound so every accepted arm proves the registered settle window.
- The analyzer parses each timing component once, requires exact Decimal
  `total_ms = transfer_ms + (settled_ms - 250) + flush_ms`, and uses that
  durable total for every paired median, delta, distribution, observer-bias,
  and resolution-floor value. Only the common first 250 ms remains excluded;
  excess observation latency is charged. Corrupt totals are rejected;
  role-specific flush mutations prove the summaries cannot fall back to the
  pre-durability transfer time, and an equal client-to-durability regression
  proves asymmetric settle/flush partitioning cannot manufacture a role delta.
- All asserted causal phase pairs are endpoint-local and require both producer
  order and nondecreasing monotonic elapsed time. Socket action completion must
  precede trace attachment; attached payload sockets must progress through
  first write/receive before their role's data-plane completion; resize and
  planner prerequisite chains are also pinned. The resize DAG additionally
  requires sent proposal before SOURCE socket acquisition, attachment before
  SOURCE settlement, final settlement/ACK before role-local completion, and
  the exact receive→arm→ready→accept or receive→dial→attach→prepared chain on
  the DESTINATION. Mutations reverse every one of those edges while preserving
  exact contiguous producer sequences and must fail. Swapping completion ahead
  of a first write, swapping attachment ahead of action completion, or
  reversing a causal elapsed interval also makes the analyzer suite fail.
- Mutation proof: restoring SOURCE dial attachment ahead of `socket_dial_end`
  makes the two-initiator Rust phase test fail at epoch zero and resize epoch
  one; restoring end-before-attachment returns it to green. No cross-endpoint
  or concurrent send/ACK ordering is asserted.
- Fixture and landed manifests encode each UTF-8 POSIX relative path in base64
  beside its decimal file size, sort under ordinal/C locale rules, and reject
  nonregular or reparse entries. The analyzer recomputes all digests, requires
  exact q/Windows canonical equality and exactly 128 landed manifest files,
  and rejects swapped per-file sizes, renamed paths, wrong root layout, or a
  forged recorded digest even when file count and total bytes are unchanged.
- The harness atomically claims a never-existing evidence directory before it
  installs the EXIT trap or writes a byte. Existing paths are rejected
  unchanged, with explicit stale `SESSION-COMPLETE`/`SESSION-VOID` diagnostics;
  offline guards also pin rejection of unrelated retained content.
- Every arm resets its exact destination with explicit error propagation,
  verifies deletion landed, and proves the replacement is an empty plain
  directory before draining caches or starting the timer. The q self-test
  mutation makes removal fail under the production `||` call shape and must
  remain rejected; a Windows source-contract guard forbids suppressed removal
  errors and requires absence, directory, reparse, and emptiness checks.
- SOURCE- and DESTINATION-initiated arms resolve to the same canonical
  endpoint-local destination path and remote module-relative path. The
  self-test pins both direction/role pairs with explicit `|| die` guards and
  rejects any `run_arm` source that lets the role-bearing evidence ID reach a
  measured destination. Adding the initiator role to
  `destination_relative_path` now turns the Bash 3.2 self-test red at the first
  q destination-path assertion; restoring the role-invariant path returns it
  to green.
- The failure handler removes any completion marker, stops only remembered
  identity-checked daemons, appends teardown errors without replacing the
  primary void reason, and never initiates session-tree deletion. HUP, INT,
  and TERM enter that same bounded failure path. Offline process tests exercise
  all three signals and prove both owned teardown paths run while remaining
  evidence paths are reported for inspection.
- Successful finalization first proves no remembered daemon or open port,
  requires analyzer-accepted local evidence, removes and verifies both exact
  Windows trees and the exact q tree, rechecks the port, and only then atomically
  renames `SESSION-COMPLETE.tmp`. Cross-host deletion is not transactional: a
  partial finalization failure keeps the complete local evidence and reports
  remote paths as “may remain,” never as certainly preserved. A zero exit is
  rejected unless the registered marker is a regular one-line file containing
  the exact build SHA with no VOID or temporary marker; preflight-only runs
  cannot create it. Mutations for failed Windows removal, a surviving q tree,
  a pre/post-cleanup open port, missing/wrong completion markers, stale
  preflight markers, and cleanup before completion all fail the self-test.
- Windows launcher and daemon PIDs are numeric and identity-checked before any
  termination: exact executable/name, one anchored block-specific `cmd.exe`
  command line, and daemon parent PID when both processes exist. Startup also
  verifies the same CIM identities immediately. Offline source-contract
  mutations fail if command-line, parent, or validate-before-stop guards move
  or disappear. If startup fails after CIM creation but before either PID file
  is readable, the generated launcher waits on a bounded block-local gate and
  cannot execute the daemon until its PID is atomically placed and read back;
  without that gate it exits on its own. Teardown recovers only the unique
  exact block-specific launcher command and its parented daemon; after stopping
  the launcher it also finds, validates, and stops a child that raced the first
  query. The live daemon smoke remains required to prove CIM quoting.
  Mutations accepting any `cmd.exe`, accepting an unparented daemon, skipping
  the bounded gate wait, opening the gate before PID placement/readback, or
  skipping the late child's exact executable validation each turn the
  self-test red.
- `LAUNCHER_SMOKE=1` is a mutually exclusive standalone live mode. After the
  full provenance and endpoint preflight, it starts only the exact Windows CIM
  launcher and daemon, proves q can reach the registered port, identity-stops
  both processes, proves both endpoint ports closed, and completes strict
  session-tree cleanup. It never registers a run, starts q's daemon, times a
  transfer, invokes the analyzer, or writes `SESSION-COMPLETE`. An offline
  call-order test and source guard pin the start/reach/stop/closed/cleanup
  sequence and keep the smoke branch ahead of registered-run state. Mutations
  removing its pre-start port gate, start, reachability probe, exact stop/log
  collection, block clear, strict cleanup/failure path, or main-branch return,
  and a mutation setting registered state, each turn the self-test red.
- Mutation proof: replacing the absolute-deadline wait with a no-op makes the
  harness self-test fail because it returns before +250 ms. Moving the
  successful Windows client-log fetch ahead of the durability marker makes
  the production-order self-test fail. Restoring both returns the harness and
  analyzer self-tests to green.
- A delayed fake Windows-result producer emits its exact sentinel and then
  holds the pipe open; the q arrival stamp must predate producer teardown by a
  broad bound. Moving the stamp to EOF or restoring a fresh post-return q
  anchor makes the self-test fail. Reverting q to Python's process-relative
  macOS `time.monotonic_ns()` also fails an explicit cross-process clock guard;
  every carried q timestamp uses `clock_gettime(CLOCK_MONOTONIC)`. Both client
  wrappers carry the q completion stamp as the fourth result field consumed by
  `run_arm`, and live preflight proves the flushed Windows sentinel reaches q
  before the remote producer exits.
- Every trace-on TCP session must prove the complete seven-epoch one-stream
  ramp from one to eight live sockets on both roles, including exact proposal,
  preparation, ACK, settlement, attachment, and role-local ordering evidence.
  Removing epoch 7 makes the targeted analyzer guard fail; disabling exact
  target/live validation makes all four final-epoch SOURCE and DESTINATION
  mutations fail. Restoring both guards returns the analyzer suite to green.
- The build-identity self-test accepts the exact 12-character clean marker and
  mutation-proves that the same marker with `.dirty` is rejected. Live q and
  Windows gates apply that positive-and-negative check to both binaries.
- The repository gate is green: `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, the documentation gate, analyzer tests, and shell
  syntax checks all passed.

## Known gaps

- No rig datum is produced by this slice. The full live run waits for fresh
  mandatory Codex adjudication, exact isolated builds, a successful live
  launcher smoke, and a green endpoint preflight.
- This four-cell run is the reduced P1 phase diagnostic, not the entire pf-1
  hard gate. The active plan still requires the separately reviewed
  small-fixture/P2 work, phase report, and `0f922de` historical control before
  pf-1 closes.
- q was not quiet during the first read-only readiness sample on 2026-07-15:
  Time Machine AutoBackup was enabled and Spotlight was using substantial CPU.
  The harness reports and refuses those conditions; it does not mutate them.

## Reviewer comments

Initial Codex review (`gpt-5.6-sol`, `xhigh`, codex-cli 0.144.4) reviewed
`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..0fb8237c2e6f63feb9cfc613d8af1602730061b0`
and returned `NEEDS FIXES` with three High findings. All three were accepted
and fixed independently: destination reset fail-closed at `661cf75`, excess
settle accounting at `1617546`, and the complete resize causal-edge audit plus
emitter alignment at `2dd977e`. See the raw review and adjudication under
`.review/results/otp12-pf1-rigw-harness.*`.

Round-2 Codex reviewed the complete immutable range through `8fbd486` and
returned `NEEDS FIXES`: it independently confirmed F1–F3 closed, then found two
new High defects. F4 is an uncharged Windows-client interval before q captures
the settle anchor. F5 is the role-bearing `rid` selecting different physical
destination paths for paired arms, contrary to the only-initiator-varies
contract. Both were accepted and fixed in order: F5 at `1231e42`, then F4 at
`6ba5408`. A separate runbook audit found the missing standalone launcher mode,
fixed at `18d3cde`; follow-up safety audit found the pre-PID-journal CIM race,
fixed at `454ebce`. The additive Grok second eye returned a schema-valid
`ACCEPTED` verdict with three independent red-to-green guards, but it does not
override the mandatory Codex findings. See the round-2 raw and adjudication
records under `.review/results/otp12-pf1-rigw-harness-r2.*`. Fresh review of
the complete fixed range is pending; no rig run is authorized yet.

Round-3 Codex reviewed
`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..53bb5e56a864abe0ee2d2b00c411846a1e7d24d5`
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

exec
/bin/zsh -lc "git log --reverse --format='%H %s' 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524
git diff --numstat 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524 -- crates/blit-core/src/transfer_session/data_plane.rs crates/blit-core/tests/transfer_session_roles.rs scripts/bench_otp12pf_rigw.sh scripts/otp12pf_rigw_analyze.py scripts/otp12pf_rigw_analyze_test.py" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Ewt7hPGJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jM7IKbiX' (errno=Operation not permitted)
bd7a4c21f2bd2d9c1aa8e125f01a256f89fc0696 Add draft pf-1 rig-W phase harness
ae494d3c5b8d48b7407d8543a552cf1825e6e517 Reject dirty rig-W build identities
7f81b2e86c7eaff1038d4e4b0d096fca677d85fe Pin rig-W TCP sessions to eight streams
3208870445e2bf6818d2670fbd9c5594d235437c Record complete rig-W harness audit
333c21ad09646f4eb440e3beecdaae277c84355d Equalize rig-W post-client settling
48bcaeaea63e713eb9e17763efa21e636c57978c Grade rig-W evidence on durable total time
a71c4ff7213ca5ad168d5405c4a498843fe4af11 Reject causally impossible rig-W traces
4eaf18165304a211ba2158dd82f7b3e3cfc7ae11 Verify exact rig-W landed tree identity
a63ebfcae38f3234a9d5ab7285bda628e8390f2a Require fresh rig-W evidence directories
d105a48d78f252832dbd60321f6a1b293206392d Verify rig-W Windows launcher ownership
0fb8237c2e6f63feb9cfc613d8af1602730061b0 Fail closed rig-W session finalization
d26733505a8f17c6de84f137cb3b3453e0c6a5ee Record rig-W Codex review findings
661cf75e0e7c5b44763d8666736ffe43edafb153 Reject incomplete rig-W destination resets
1617546f040385069a1d6af4f45770249098cf57 Charge rig-W excess settle latency
2dd977e9d387d838b2cd7dfbdc3ffd812a5f06c1 Complete rig-W resize causal guards
8fbd4866cbf83ab6af4d8a0467dbb9680172d3b0 Record rig-W audit fixes
6b95f034e996238a16a8ce25ffb89b555ecbf3a3 Record rig-W round-two review
1231e4252b004c494608c82dafc93c81105d6aa1 Make rig-W destination paths role-invariant
6ba5408754d41719414555479890722c2e9c5e38 Anchor rig-W settle at client result arrival
18d3cdef200ebf19c1b00e66fdd6159ec43796ec Add standalone rig-W launcher smoke
454ebce16e59ddc86ebfa6915149fdc51be68e52 Journal Windows launcher before daemon start
53bb5e56a864abe0ee2d2b00c411846a1e7d24d5 Record rig-W round-two closures
5e90c769168b2f763d24847fdfa85b548d3f4b7c Record rig-W round-three review
27c94b0170507c696a6c092bf45e19f466b95131 Fix rig-W path parity guard
7e9d2d531977a90b034a662c68e889d495d6fd8b Fix rig-W lifecycle selftest guards
6f517ea1bdbea2f7d83f15c086d2bf5f764cf524 Record rig-W Bash guard fixes
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-e6e3YHWX' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-An0OwfcT' (errno=Operation not permitted)
4	5	crates/blit-core/src/transfer_session/data_plane.rs
3	0	crates/blit-core/tests/transfer_session_roles.rs
2460	0	scripts/bench_otp12pf_rigw.sh
2020	0	scripts/otp12pf_rigw_analyze.py
1069	0	scripts/otp12pf_rigw_analyze_test.py

codex
The range is 27 commits, not a single patch, and the checked-out tree matches the target apart from an unrelated untracked review artifact. I’m now tracing the executable contracts end-to-end—schedule construction, per-arm timing/durability, launcher recovery/finalization, and then the G3/G4 mutation tests—against the preregistered rules.
exec
/bin/zsh -lc "sed -n '1,220p' docs/plan/OTP12_PERF_FINDINGS.md
sed -n '379,780p' docs/plan/OTP12_PERF_FINDINGS.md" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'rg -n "''^[a-zA-Z_][a-zA-Z0-9_]*'"\\(\\)|"'^main'"\\(\\)|SELFTEST|LAUNCHER_SMOKE|SCHEDULE|SESSION_|ARM|run_arm|destination_relative_path|settled_ms|flush|Transfer|SOURCE|DESTINATION|ENDPOINT|firewall|CIM|PID|signal\" scripts/bench_otp12pf_rigw.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
6:# implementations: SOURCE always sends and DESTINATION always receives.
8:# Transfer RPC and therefore which endpoint dials the peer.
25:SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
28:SELFTEST=${SELFTEST:-0}
30:LAUNCHER_SMOKE=${LAUNCHER_SMOKE:-0}
66:SESSION_TAG=$(date -u +%Y%m%dT%H%M%SZ).$$
67:OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12pf-rigw-$SESSION_TAG}
68:WIN_SESSION="$WIN_ROOT/rigw-pf1/$SESSION_TAG"
77:log() {
86:die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
87:append_void_line() {
90:session_void() {
98:reserve_evidence_dir() {
127:claim_output_dir() {
135:wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
144:SESSION_FINALIZED=0
146:Q_SESSION_MAY_EXIST=0
147:WIN_SESSION_MAY_EXIST=0
150:teardown_die() {
160:reject_registered_overrides() {
170:validate_mode_selection() {
172:    for name in SELFTEST PREFLIGHT_ONLY LAUNCHER_SMOKE; do
181:        || die "SELFTEST, PREFLIGHT_ONLY, and LAUNCHER_SMOKE are mutually exclusive"
184:emit_schedule() {
193:q_source_path() { printf '%s/src_%s' "$Q_MODULE" "$1"; }
194:win_source_path() { printf '%s/src_%s' "$WIN_MODULE" "$1"; }
195:destination_relative_path() {
200:    printf 'rigw-sessions/%s/destination/container' "$SESSION_TAG"
202:q_destination_path() {
203:    printf '%s/%s' "$Q_MODULE" "$(destination_relative_path "$1")"
205:win_destination_path() {
206:    printf '%s/%s' "$WIN_MODULE" "$(destination_relative_path "$1")"
208:arm_destination_path() {
216:arm_destination_argument() {
218:    relative=$(destination_relative_path "$role") || return 2
227:append_clock_row() {
230:q_monotonic_ns() {
233:settle_until_deadline() {
245:stamp_result_arrival_on_q() {
267:successful_windows_log_phase_ok() {
270:fetch_successful_windows_client_log() {
276:embeds_clean_q() {
283:selftest() {
285:    local selftest_client_done selftest_deadline selftest_settle_done run_arm_source
289:    local signal signal_dir signal_rc contract_tmp on_exit_source append_tmp
298:        SELFTEST=1
300:        LAUNCHER_SMOKE=0
306:        SELFTEST=2
308:        LAUNCHER_SMOKE=0
337:    local destination_rel="rigw-sessions/$SESSION_TAG/destination/container"
339:        || die "q SOURCE-initiated destination path changed"
341:        || die "q DESTINATION-initiated destination path changed"
343:        || die "Windows SOURCE-initiated destination path changed"
345:        || die "Windows DESTINATION-initiated destination path changed"
351:        || die "Windows-to-q SOURCE-initiated destination argument changed"
353:        || die "Windows-to-q DESTINATION-initiated destination argument changed"
355:        || die "q-to-Windows SOURCE-initiated destination argument changed"
357:        || die "q-to-Windows DESTINATION-initiated destination argument changed"
394:    run_arm_source=$(declare -f run_arm)
395:    python3 - "$run_arm_source" <<'PY' || die "run_arm post-client ordering changed"
406:    'flush_out=$(flush_verify_q "$dest")',
407:    'flush_out=$(flush_verify_win "$dest")',
410:    'total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))',
417:        raise SystemExit(f"missing run_arm ordering marker: {marker}") from exc
419:    raise SystemExit(f"run_arm ordering markers out of order: {positions}")
423:    '$SESSION_TAG/$rid/container',
427:        raise SystemExit(f"forbidden run_arm pattern returned: {forbidden}")
477:    raise SystemExit(f"missing pre-PID-file recovery marker: {exc}") from exc
479:    raise SystemExit("empty-PID return can bypass exact Windows process discovery")
536:    raise SystemExit(f"Windows PID journal does not precede launch gate: {controller_positions}")
560:                die "cmd-only Windows recovery fell into the empty-PID port branch"
565:            || die "cmd-only Windows recovery retained remembered PIDs"
579:    "WIN_SESSION_MAY_EXIST=1",
598:    "SESSION_FINALIZED",
602:    "run_arm",
611:    "Q_SESSION_MAY_EXIST",
617:branch_start = main.index('if [[ "$LAUNCHER_SMOKE" == 1 ]]')
622:branch_markers = ('if [[ "$LAUNCHER_SMOKE" == 1 ]]', "launcher_smoke;", "return;", "fi;")
635:        SESSION_TAG=offline-smoke
637:        SESSION_FINALIZED=0
639:        Q_SESSION_MAY_EXIST=0
640:        WIN_SESSION_MAY_EXIST=0
649:                && "$WIN_SESSION_MAY_EXIST" == 1 \
694:            [[ "$WIN_SESSION_MAY_EXIST" == 1 && -z "$current_block" \
698:            WIN_SESSION_MAY_EXIST=0
705:        [[ "$REGISTERED_RUN_STARTED" == 0 && "$SESSION_FINALIZED" == 0 \
707:            && "$Q_SESSION_MAY_EXIST" == 0 \
708:            && "$WIN_SESSION_MAY_EXIST" == 0 \
854:        [[ "$SESSION_FINALIZED" == 1 ]] \
855:            || die "registered finalization did not set SESSION_FINALIZED"
864:        SESSION_TAG=fail-remote
865:        Q_SESSION_MAY_EXIST=1
866:        WIN_SESSION_MAY_EXIST=1
875:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
877:        [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
883:        SESSION_TAG=open-port
884:        Q_SESSION_MAY_EXIST=1
885:        WIN_SESSION_MAY_EXIST=1
894:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
899:        SESSION_TAG=surviving-q
900:        Q_SESSION_MAY_EXIST=1
901:        WIN_SESSION_MAY_EXIST=1
911:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
916:        SESSION_TAG=succeeds
917:        Q_SESSION_MAY_EXIST=1
918:        WIN_SESSION_MAY_EXIST=1
927:        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]] \
929:        [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
934:        SESSION_TAG=late-port
935:        Q_SESSION_MAY_EXIST=1
936:        WIN_SESSION_MAY_EXIST=1
952:            SESSION_TAG="remembered-$remembered"
974:    "'$WIN_MODULE/rigw-sessions/$SESSION_TAG'",
992:    mkdir -p "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG"
993:    printf 'retain me\n' > "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel"
1005:        SESSION_FINALIZED=0
1007:        Q_SESSION_MAY_EXIST=1
1008:        WIN_SESSION_MAY_EXIST=1
1030:    grep -Fq 'cleanup errors: Windows PID recovery failed' "$failure_tmp/evidence/SESSION-VOID" \
1032:    grep -Fq "q session evidence may remain; inspect $failure_tmp/q-module/rigw-sessions/$SESSION_TAG" \
1035:    grep -Fq "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG" \
1041:    [[ "$(< "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
1075:        SESSION_FINALIZED=0
1077:        WIN_SESSION_MAY_EXIST=0
1098:        SESSION_FINALIZED=1
1123:        SESSION_FINALIZED=1
1146:        SESSION_FINALIZED=0
1169:        SESSION_FINALIZED=0
1192:            SESSION_FINALIZED=0
1211:    for signal in HUP INT TERM; do
1212:        signal_dir="$failure_tmp/signal-$signal"
1213:        mkdir "$signal_dir"
1222:SESSION_FINALIZED=0
1224:Q_SESSION_MAY_EXIST=1
1225:WIN_SESSION_MAY_EXIST=1
1230:win_daemon_stop() {
1234:q_daemon_stop() {
1240:install_signal_traps
1244:' _ "$SCRIPT_DIR/bench_otp12pf_rigw.sh" "$signal_dir" "$signal"
1245:        signal_rc=$?
1247:        [[ "$signal_rc" == 1 ]] \
1248:            || die "$signal cleanup returned $signal_rc, expected 1"
1249:        grep -Fxq "received $signal" "$signal_dir/SESSION-VOID" \
1250:            || die "$signal cleanup omitted its signal reason"
1251:        [[ "$(LC_ALL=C sort "$signal_dir/stops")" == $'q\nwindows' ]] \
1252:            || die "$signal cleanup did not invoke both exact-owned teardown paths"
1253:        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
1254:            || die "$signal cleanup left SESSION-COMPLETE"
1266:    log "SELFTEST OK: exact four-block/128-arm schedule and analyzer guards"
1269:sha256_q() { shasum -a 256 "$1" | awk '{print $1}'; }
1270:sha256_win() {
1275:float_le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }
1277:q_load1() {
1281:q_spotlight_cpu() {
1287:q_time_machine_gate() {
1298:q_quiet_gate() {
1318:win_quiet_gate() {
1339:q_topology_gate() {
1368:win_topology_gate() {
1385:q_to_win_mss() {
1394:win_to_q_mss() {
1407:mss_gate() {
1420:firewall_gate() {
1427:") || die "existing Windows firewall rule is absent/unreadable; harness will not create it"
1434:        || die "Windows firewall rule mismatch: '$out'"
1435:    log "firewall verified only: existing inbound allow is scoped to $WIN_ACTIVE"
1438:ports_closed() {
1446:timer_gate() {
1462:windows_result_stream_gate() {
1482:fixture_shape_q() {
1493:fixture_shape_win() {
1500:write_q_tree_manifest() {
1545:write_win_tree_manifest() {
1574:matching_manifest_digest() {
1580:verify_fixtures() {
1584:    WIN_SESSION_MAY_EXIST=1
1614:write_manifest() {
1629:provenance_gate() {
1655:preflight() {
1666:    firewall_gate
1675:q_daemon_stop() {
1682:            || { teardown_die "refusing to stop q PID $pid because it is not the launched daemon: $cmd"; return 1; }
1689:            && { teardown_die "q daemon PID $pid survived exact teardown"; return 1; }
1694:win_daemon_stop() {
1719:            teardown_die "Windows PID recovery failed for block $current_block"
1728:            teardown_die "Windows PID files are empty but port $PORT may still be open"
1734:        || { teardown_die "invalid remembered Windows daemon PID '$pid'"; return 1; }
1736:        || { teardown_die "invalid remembered Windows launcher PID '$cmdpid'"; return 1; }
1758:  if (\$d.Name -ine 'blit-daemon.exe' -or \$actual -ine '$WIN_ACTIVE') { throw \"daemon PID identity mismatch: \$(\$d.Name) \$(\$d.ExecutablePath)\" }
1765:# Every identity is validated before either remembered PID is stopped.
1789:fetch_win_file() {
1807:collect_block_logs() {
1815:stop_daemons() {
1823:q_daemon_start() {
1837:        BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
1841:        env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID \
1851:win_daemon_start() {
1853:    # The CIM-created batch launcher is allowed to exist before its PID is
1855:    # PID has been atomically placed and read back. Without the gate it times
1875:\$trace = if ('$state' -eq 'on') { @('set BLIT_TRACE_SESSION_PHASES=1','set BLIT_TRACE_RUN_ID=$run_id') } else { @('set BLIT_TRACE_SESSION_PHASES=','set BLIT_TRACE_RUN_ID=') }
1895:if (\$persistedLauncher -ne [string]\$r.ProcessId) { throw \"launcher PID persistence mismatch: \$persistedLauncher\" }
1911:        || session_void "cannot parse Windows daemon PIDs from '$out'"
1914:start_daemons() {
1926:record_clock_samples() {
1940:drain_both() {
1961:prepare_destination() {
1986:flush_verify_q() {
1999:flush_verify_win() {
2009:q_client_run() {
2013:        trace_env=(BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id")
2015:    env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID "${trace_env[@]}" \
2029:win_client_run() {
2034:if ('$state' -eq 'on') { \$env:BLIT_TRACE_SESSION_PHASES='1'; \$env:BLIT_TRACE_RUN_ID='$run_id' }
2035:else { Remove-Item Env:BLIT_TRACE_SESSION_PHASES,Env:BLIT_TRACE_RUN_ID -ErrorAction SilentlyContinue }
2045:session_id_from_log() {
2058:run_arm() {
2060:    local direction carrier shape flag="" dest dest_arg rid qerr werr client_rel client_abs remote_err result result_tag result_extra transfer_ms rc flush_out flush_ms count bytes want drain session_id total anchor_now_ns
2061:    local windows_client=0 arm_phase=client_done client_done_ns settle_deadline_ns settle_done_ns settled_ms
2132:    settled_ms=$(((settle_done_ns - client_done_ns) / 1000000))
2133:    [[ "$settled_ms" -ge "$SETTLE_MIN_MS" && "$settled_ms" -lt "$SETTLE_MAX_MS" ]] \
2134:        || session_void "$rid post-client settle was ${settled_ms}ms, expected [$SETTLE_MIN_MS,$SETTLE_MAX_MS)"
2142:        flush_out=$(flush_verify_q "$dest") || session_void "$rid q durability probe failed"
2146:        flush_out=$(flush_verify_win "$dest") || session_void "$rid Windows durability probe failed"
2152:    IFS='|' read -r _ flush_ms count bytes <<<"$flush_out"
2156:    [[ "$flush_ms" =~ ^[0-9]+$ ]] || session_void "$rid flush timer malformed: '$flush_out'"
2190:    total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))
2193:        "$transfer_ms" "$settled_ms" "$flush_ms" "$total" "$landed_root" \
2196:    log "$rid: transfer=${transfer_ms}ms settled=${settled_ms}ms flush=${flush_ms}ms total=${total}ms session=${session_id:-none}"
2199:cell_order() {
2208:run_block() {
2209:    local block="$1" state="$2" pass="$3" first="$4" last="$5" run_id="${SESSION_TAG}-b${block}-${state}"
2225:            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$first_role" 1
2226:            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$second_role" 2
2235:end_gate() {
2244:strict_success_cleanup() {
2247:        || { LAST_ERROR="strict cleanup found remembered q daemon PID $q_daemon_pid"; return 1; }
2249:        || { LAST_ERROR="strict cleanup found remembered Windows daemon PID $win_daemon_pid"; return 1; }
2251:        || { LAST_ERROR="strict cleanup found remembered Windows launcher PID $win_cmd_pid"; return 1; }
2259:\$paths = @('$WIN_MODULE/rigw-sessions/$SESSION_TAG', '$WIN_SESSION')
2268:    WIN_SESSION_MAY_EXIST=0
2269:    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
2270:        rm -rf -- "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
2273:    [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
2274:        && ! -L "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
2276:    Q_SESSION_MAY_EXIST=0
2282:launcher_smoke() {
2283:    local run_id="${SESSION_TAG}-launcher-smoke"
2284:    WIN_SESSION_MAY_EXIST=1
2295:    log "LAUNCHER_SMOKE OK: exact Windows CIM launcher started, reached, identity-stopped, and cleaned; no transfer timed"
2298:finalize_registered_session() {
2300:    SESSION_FINALIZED=0
2314:    SESSION_FINALIZED=1
2317:record_failure_evidence() {
2319:    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
2320:        append_void_line "q session evidence may remain; inspect $Q_MODULE/rigw-sessions/$SESSION_TAG"
2322:    if [[ "$WIN_SESSION_MAY_EXIST" == 1 ]]; then
2323:        append_void_line "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG"
2327:on_signal() {
2328:    local signal="$1" code="$2"
2329:    LAST_ERROR="received $signal"
2334:install_signal_traps() {
2335:    trap 'on_signal HUP 129' HUP
2336:    trap 'on_signal INT 130' INT
2337:    trap 'on_signal TERM 143' TERM
2340:registered_completion_marker_valid() {
2352:on_exit() {
2365:        && "$SESSION_FINALIZED" != 1 ]]; then
2375:        && "$SESSION_FINALIZED" != 0 ]]; then
2411:main() {
2413:    if [[ "$SELFTEST" == 1 ]]; then selftest; return; fi
2419:    install_signal_traps
2426:    if [[ "$LAUNCHER_SMOKE" == 1 ]]; then
2432:    Q_SESSION_MAY_EXIST=1
2433:    mkdir -p "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
2435:    printf '%s\n' 'block,trace_state,pass,cell,role,pair,role_order,transfer_ms,settled_ms,flush_ms,total_ms,landed_root,tree_manifest_sha256,exit,drain,valid,run_id,session_id,client_log' > "$RUNS_CSV"
2438:    WIN_SESSION_MAY_EXIST=1
2458:if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then

 succeeded in 0ms:
# otp-12 perf findings — investigate + fix before acceptance (design)

**Status**: Active
**Approved**: D-2026-07-13-1 — owner, 2026-07-13, verbatim:
**"one more round with codex on the plan then just write the code and
reviewloop slice by slice. that converges faster than plans with no
ground truth to test."** The final round ran (round 5, verdict NOT READY,
3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
counterfactual, F3 the inexecutable decision rule); all three are fixed
in this revision, and implementation now proceeds **slice by slice, each
through the codex loop** (D-2026-07-04-1 unchanged). A non-converged plan
verdict is no longer a gate — the plan's earlier "flip to Active at codex
convergence" rule is superseded by D-2026-07-13-1, because rounds 2–5
were increasingly finding defects in the *prose* while the plan's central
factual claim was settled by *measurement* (the same-OS rig refuted a
claim four review rounds had left standing). pf-1 exists to generate
ground truth; it starts now.

**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
assumed by this plan):** P1 has **no escape hatch on the books**.
D-2026-07-12-1 waives a cross-direction converge-up miss only for a cell
that is *already* invariance-passing; P1 is the invariance failure
itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
plan pursues) or the owner must **amend acceptance criterion 1** in a new
decision. pf-1 proceeds either way: it produces the evidence that
decision would rest on.
**Created**: 2026-07-12
**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
say the quiet part: "Unification that slows the fast direction fails
review." P1 is a miss of the parent's HEADLINE acceptance criterion
(initiator/verb invariance, ±10%) — not a nice-to-have.
**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
if an investigation slice needs one, it stops and this doc is amended
through the loop first.

**Sequencing (corrected 2026-07-13).** This doc originally deferred
otp-12c/12d/13 outright. In fact **otp-12c RAN on 2026-07-13** under a
fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
at the cutover sha `f35702a`; `docs/bench/otp12c-{delegated,win}-2026-07-13/`).
That does not change this plan's standing, and the rows are not lost
work — under `pf-final` they are **pre-fix rows, void for acceptance**,
but they serve two real purposes: (a) an **independent replication** of
both findings at the shipped sha (below), which is exactly the
independent corroboration the round-2 review said P1 lacked; and (b) the
pre-pf-1 control the investigation needs. **otp-12d and otp-13 remain
deferred** until P1/P2 are fixed or explained at code level — assembling
an acceptance matrix out of pre-fix rows would build the artifact otp-13
walks from rows this plan declares void.

## The two findings (evidence, both committed)

**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
(`docs/bench/otp12-win-2026-07-12/`, replicated in
`docs/bench/otp12c-win-2026-07-13/`). `wm_tcp_mixed` invariance FAILs in
**two independent sessions**, and got WORSE at the shipped sha:

| session | build | mac_init | win_init | ratio | arm spreads |
|---|---|---|---|---|---|
| 12b (2026-07-12) | `e21cf84` | 1127 | 911 | **1.237** | 8.2 / 3.3% |
| 12c-win (2026-07-13) | `f35702a` (cutover) | 1221 | 939 | **1.300** | 6.4 / 8.4% |

Corroborated by block-1 `pull_tcp_mixed` new-vs-old-same-session:
**1.313** (12b: 1138/867) and **1.247** (12c-win: 1192/956).

**This cannot be re-run away.** Both sessions' arm spreads are far below
D2's 25% escalation trigger, so no escalation session is even available;
the cells stand as measured. (The 12c-win session was a fresh staging on
a different day at a different sha — the round-2 review's objection that
the 1.313 corroboration was "same rig/session, not independent" is now
answered by an independent session reproducing the same cell.)

**What the evidence actually supports — and the confound it does NOT
escape** (corrected, review round 3; an earlier draft of this section
claimed the `mw` cell was a clean control isolating "destination
initiation" as the cause. It is not, and the correction matters because
it re-aims the hypotheses):

Every invariance cell compares two arms that share the same endpoints
and the same data direction, so **within** a cell the initiator is the
only variable — that part is clean. Arm medians (12c-win):

| cell | data direction | dest-initiated arm | source-initiated arm | ratio | spreads |
|---|---|---|---|---|---|
| `wm_tcp_mixed` | Win→Mac | 1221 | 939 | **1.300 FAIL** | 6.4 / 8.4% |
| `mw_tcp_mixed` | Mac→Win | 1477 | 1415 | 1.044 PASS | 20.8 / 20.5% |

The initiator penalty is therefore **real and large in the Win→Mac
direction only**. In Mac→Win the two layouts are within noise, and the
ordering even **flips between sessions** (12b: dest-initiated 1502 was
*faster* than source-initiated 1587), on spreads of 17–25%.

Crossing from `wm` to `mw` is **not** a controlled swap of one variable:
it also swaps the destination filesystem (APFS vs NTFS), the TCP stack,
which host runs the client, and the flush method. So the supported
signature is an **interaction — TCP × mixed × Win→Mac × initiator** —
not "destination initiation" on its own.

Worse, on a two-host rig the failing configuration is **confounded by
construction**: in the slow arm the destination is the Mac (which dials)
*and* the source is Windows (which accepts). With only two hosts, **host
identity IS role** — "Mac-as-dialing-destination" and
"Windows-as-accepting-source" are the same configuration and cannot be
separated by any number of additional runs on this rig.

### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)

**Evidence: `docs/bench/otp12-perf-2026-07-13/` — magneto↔skippy, Linux on
BOTH ends, real 10 GbE, full otp-12 methodology** (cold caches both ends,
destination drained, ABBA, pair-void, RUNS=4; 64 runs, 8/8 cells, zero
voided). Harness `scripts/bench_otp12pf_linux.sh`.

**P1 does NOT reproduce.** Its own cell passes with room to spare:

| cell | srcinit | destinit | ratio | outcome |
|---|---|---|---|---|
| `sm_tcp_mixed` (P1's cell) | 1745 | 1905 | **1.092** | PASS |
| `ms_tcp_mixed` (P1's cell) | 2085 | 2079 | **1.003** | PASS |

**8/8 invariance cells PASS** (`ms_grpc_mixed` via its pre-registered
RUNS=8 escalation → 1.063). There is no destination-initiator penalty at
all when both ends are Linux.

Therefore:

- **P1 requires the Mac↔Windows pairing.** It is NOT a pure layout
  property of blit's code — a pure layout cost would have appeared here,
  on the same code, same carrier, same fixture.

- **⚠ BUT P1 HAS NO ESCAPE HATCH TODAY (review round 5, BLOCKER).** An
  earlier revision of this section said D-2026-07-12-1 lets the owner
  accept P1 as a platform residue. **It does not.** That decision excuses
  a **cross-direction converge-up** miss for a cell that has ALREADY
  satisfied its precondition **"(b) is initiator/verb-invariant within
  ±10%"** (`docs/DECISIONS.md` D-2026-07-12-1). **P1 IS the invariance
  failure** (`wm_tcp_mixed` 1.300 FAIL) — the precondition it would need
  is the very thing it violates. No decision on the books waives it.
  Therefore exactly two exits exist, and pf-1 must aim at them:
  1. **FIX IT** — P1 ≤ 1.10 on rig W. This remains the default and the
     bar (`ONE_TRANSFER_PATH.md` acceptance criterion 1 is mandatory).
  2. **A NEW OWNER DECISION amending criterion 1** — for which the
     same-OS result is the honest evidence base: criterion 1 asks for
     invariance "on a symmetric rig", Mac↔Windows was designated only
     because no better pair existed, and one now does — magneto↔skippy,
     where blit measures **8/8 invariant**. An owner could reasonably
     rule that criterion 1 is judged on the rig that isolates blit's own
     behaviour, with the Mac↔Windows delta recorded as platform residue.
     **That ruling does not exist. It must not be assumed, and this plan
     must not be written as though it will be granted.**
- **This does NOT fully exonerate the code.** It rules out a pure layout
  property; it does not rule out a code path whose cost only becomes
  material under a particular platform — e.g. a slow accept branch on the
  Windows side, which is exactly what H1 accuses. H1/H5/H6 stay LIVE but
  are now **narrowed to platform-interacting mechanisms**, and only the
  dial/accept inversion counterfactual on rig W can finish the job.
- **P2 is untested by this rig** (it is a converge bar vs the OLD build,
  and no `0f922de` build is staged on these hosts). Nothing here speaks
  to it.

> **⚠ A RETRACTED CLAIM LIVED HERE.** An earlier revision of this section
> asserted the opposite — "P1 reproduces at 1.78 → the confound breaks
> toward CODE → the fix is mandatory and cannot be waived" — and STATE and
> the acceptance plan were amended to match. That was **WRONG**. It rested
> on a scratch probe (and a first harness revision) that ran the durability
> `sync` inside the INITIATING host's timed bracket: in the push arm the
> initiator is the SOURCE, which only read, so its sync was a no-op and the
> destination's writeback was never paid; in the pull arm the initiator IS
> the destination, so it paid the full writeback. One arm was charged for
> durability the other got free — multi-second on skippy's ZFS — which
> manufactured "failures" on every carrier and fixture, **including the
> gRPC control that is supposed to be clean**. That carrier-independence is
> what exposed it: a real code effect is carrier-specific; an accounting
> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
> never by verb — the otp-2w rule, re-learned). The retraction is recorded
> rather than quietly overwritten because the wrong number was reported to
> the owner and briefly drove this plan.

### The residual confound (WHICH code) still needs a counterfactual

Breaking platform-vs-code does NOT tell us *which* layout property costs
the time. On any two-host rig, host identity remains welded to role, so
"the accepting end" cannot be separated from "that host" by more runs:

- **pf-1 must compare all four rig-W arms** (both cells × both
  initiators), not two, and report the interaction — not a single ratio.
- **The disambiguator is a dial/accept inversion counterfactual, not a
  rig** — but it is **NOT sufficient on its own** (review round 5): the
  inversion swaps the source's `Accept`, the destination's `Dial`, AND
  the epoch-0 topology **simultaneously**, so a positive result implicates
  *the topology pair*, not H1 specifically. It cannot distinguish
  source-accept serialization from synchronous destination dialing
  (the `destination_session` `Frame::Resize` /
  `DestRecvPlane::Initiator` branch), nor prove the resize-specific claim.
  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
  1. **dial/accept inversion** — same direction, same hosts, same fixture;
     only who dials changes. Implicates the topology pair (or exonerates it).
  2. **no-resize / pre-opened streams** — force the final stream count at
     epoch 0 so no resize epoch ever fires. If the gap survives with zero
     resizes, H1's resize-specific mechanism is **KILLED** regardless of
     what (1) shows (and note `dial.rs:474`: all three fixtures already
     target 8 streams, so resize *count* was never the discriminator).
  3. **per-side ordering** — hold the topology fixed and vary only whether
     the destination's dial-before-ACK is synchronous. Separates the two
     halves the inversion conflates.
  H1 is CONFIRMED only if the wall-time recovery tracks the **accept role**
  across (1) AND survives (2); it is KILLED if the gap persists with no
  resizes, or if (3) shows the cost is the synchronous dial rather than the
  accept branch. Any of these that changes connection topology — (1) and
  (2) do — **trips this plan's Contract stop-and-amend rule**
  (`TRANSFER_SESSION.md` amended through the loop BEFORE the flag is
  written). Same-build-both-ends (D-2026-07-05-2) means no compatibility
  surface is created.
  **H1 is also WEAKENED by the Linux null** (it predicts a layout cost that
  did not appear on a real-network same-OS pair), so pf-1 must be prepared
  to kill it and fall through to H5/H6/H7.
- **The same-platform loopback run is a ONE-WAY test** (corrected — an
  earlier draft of this section had it backwards). A dest-initiator
  penalty that still appears on Mac↔Mac loopback proves **pure layout**
  (code). Its ABSENCE proves **nothing**: loopback has no NIC, near-zero
  RTT and a huge MTU, so it erases exactly the per-epoch accept/dial
  round-trip cost H1 accuses. A negative local result is **INCONCLUSIVE**
## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)

- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
  connection-initiating end DIALS; byte direction is role-set
  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
  session the SOURCE is the responder: each sf-2 resize epoch is
  ACCEPTED off the source's listener while the DESTINATION dials
  (otp-5b-2: `SourceSockets` Dial/Accept branches;
  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
  accept/dial round-trips or serialization in the accept branch that the
  dial branch does not pay.
  **⚠ H1 ACCUSES CODE, NOT A PLATFORM (canonical; added 2026-07-14 after the
  shorthand misled two sessions).** The word "Windows" appears nowhere above.
  Windows is merely *who happens to be the accepting source* in P1's slow arm on
  rig W, so other docs say "H1's Windows accept branch" as **shorthand for where
  the accused code runs on that rig** — it is NOT a claim that H1 requires
  Windows. Two consequences, both load-bearing: (a) **a reproduction of P1 on a
  non-Windows pair does NOT kill H1** — the accused code runs there too, so it is
  *consistent with* H1 (and "consistent with H1" is not confirmation, below);
  (b) **a disappearance of P1 without Windows does not CONFIRM H1** either — it
  would only mean the accused cost is platform-conditional, which is a further
  claim. Only the dial/accept inversion counterfactual in pf-1 can settle H1.
  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
  was "mixed exercises resize hardest", but **all three fixtures target
  eight streams before clamping** (`src/dial.rs:474`) — so resize
  *count* cannot explain mixed-only behaviour, and H1 must name what
  about mixed differs (shard-boundary timing? the tar-shard small half
  interleaving with the big-file stream at the moment epochs fire?) or
  be killed. **H1 also names the wrong half without proof**: it accuses
  `Accept` while the destination's **synchronous dial-before-ACK** path
  (`destination_session`'s `Frame::Resize` /
  `DestRecvPlane::Initiator` branch) is an equally good suspect. pf-1 must
  separate them with the dial/accept inversion counterfactual below —
  "consistent with H1" is not confirmation.
- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
  interleave cannot happen — resize begins only after
  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
  layouts drain the same fixed 128-entry destination need loop, so
  batch emission cannot interleave with the resize controller during
  manifest/need emission in either layout. Kept only as a residual: if
  pf-1 timing shows a layout-dependent need-batch delta anyway, the
  mechanism must be re-derived from the trace, not from this text.
- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
  already killed its named candidates (the small half is tar-sharded and
  written with parallel per-file `create_dir_all`/`fs::write`, NO
  per-file flush; per-file progress emission to the served push
  destination is disabled — `remote/transfer/sink.rs`; and old push used
  the same served sink, so fsync/flush policy and progress emission are
  NOT old/new deltas). What was left — "dest-side directory work/handle
  churn" — **names no old/new code delta at all**, and its only probe
  (precreate-vs-not) is explicitly environmental and cannot attribute
  code (Method 3(a)). A hypothesis that cannot be confirmed *or* killed
  by pf-1 is not a hypothesis; keeping it would let pf-1 close with a
  shrug. It is therefore retired, and its one code-attributable
  descendant — a per-member cost on the TCP receive path that old push
  did not pay — lives on as **H6**, which names an executed-path delta.
  H3 may only be revived if the pf-1 trace names a concrete old/new
  delta in the destination directory/handle path; the 12b cross-block
  precreated-container lead (8%, NTFS) is recorded as an environmental
  lead for that trace, not as an attribution.
- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
  unchanged since `0f922de` (`remote/transfer/data_plane.rs`; the
  earlier `dial.rs` attribution was wrong), and old small push ALSO
  opened at one stream (after its 128-file early flush) then resized
  live — so neither framing nor "fixed-count opening" discriminates.
  What survives of H4 is ramp cadence/shard-boundary timing only, and
  it is subordinate to H5.
- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
  scan/diff/transfer overlap on the TCP plane — current code withholds
  every TCP payload until `ManifestComplete`
  (`transfer_session/mod.rs`), while old push negotiated and queued
  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
  gRPC's in-stream carrier did not change comparably — which matches
  the exact signature "TCP regressed while gRPC did not" (zoey gRPC at
  parity 1.001, Windows gRPC faster; NOT "gRPC uniformly at parity" —
  review round 3). NOTE: an H5 fix
  reorders session phases and multi-ADD/pipelined epochs conflict with
  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
  ordering), so any H5 fix triggers this plan's Contract
  stop-and-amend rule BEFORE implementation.
- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
  need-claim locking on the TCP receive plane — TCP receive
  (`NeedListSink`) takes a separate mutex/hash-set claim per member
  (`NeedListSink::claim`, called per member by its tar-shard
  `write_payload` arm), while the gRPC path claims a whole shard under one
  lock (the `destination_session` `TarShardHeader` arm).
  TCP-only and per-member (so small-file-heavy) — matches the P2
  signature independently of H5. Discriminated by the pf-1 per-member
  locking timings (Method 3(e), now unconditional).
  **Historical control — corrected (review round 3): test the EXECUTED
  path, not source presence.** `NeedListSink` *exists* in the tree at
  `0f922de`, so "does the symbol exist there" is the wrong question and
  would wrongly force H6 into a "multiplied claim frequency" story. What
  matters is what old push actually RAN: at `0f922de` the served push
  data plane goes `socket → StallGuard → execute_receive_pipeline →
  FsTransferSink → disk`
  (`crates/blit-daemon/src/service/push/data_plane.rs:185-206`) —
  it **bypasses `NeedListSink` entirely** and takes no per-member claim.
  So H6's claim is precise and falsifiable: the unified TCP receive path
  introduced a per-member lock/hash-set claim on a path whose old
  counterpart took none. pf-1 confirms it by (a) reading the executed
  old path (done — cited above) and (b) the per-member locking timings;
  it is KILLED if those timings do not scale with member count or do not
  account for a material share of the P2 gap. If H6 is confirmed, the P2
  fix bar applies unchanged (≤ 1.10 against BOTH references, BOTH rigs);
  no separate bar is granted.
  **H6's WALL-TIME counterfactual (added round 5 — timings alone would
  strand pf-1 under the uniform decision rule):** behind a debug flag,
  claim the whole tar shard under ONE lock on the TCP receive path —
  i.e. give TCP the same batch-claim shape the gRPC path already uses
  (`transfer_session/mod.rs:3047`), rather than a per-member claim
  (`data_plane.rs:1167`). This is safe and wire-neutral (it changes only
  the granularity of a local mutex/hash-set claim, not any frame), so it
  does NOT trip the Contract rule. Grade its recovery against `Δ_P2` on
  the uniform scale. If per-member claiming is the cost, batch-claiming
  recovers it; if not, H6 dies with a number rather than a shrug.

- **H7 (P2; added by review round 4 — the SHARED-controller candidate
  the gRPC caveat predicted)**: HEAD's need/manifest bookkeeping is
  heavier than old push's per entry. The unified source keeps a
  **mutex-protected sent-manifest map** with per-entry insertion and
  removal, and routes each need through a **per-need event-channel hop**
  (`transfer_session/mod.rs:1038`, `:1123`, `:1350`); old push used a
  **task-local map and handled need batches inline**, with no lock and no
  channel hop per entry. This is **per-entry**, so it scales with FILE
  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
  **shared by BOTH carriers**. That is the precise class the round-3
  gRPC caveat warned about: a shared regression can hide under gRPC's
  larger carrier-specific gain, so "TCP-only symptom" does NOT exonerate
  shared code. No prior hypothesis tested it. Discriminated by: per-entry
  bookkeeping timings scaled against file count, plus the wall-time
  counterfactual (a task-local/batch-inline path behind a debug flag).
  H7 and H6 are independent and may BOTH contribute.

## Method (the investigation slice — no behavior changes)

1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
   in-process/two-process rigs on the Mac with the otp-2 fixture shapes.
   P1 uses the wire-neutral, low-frequency structured timeline enabled by
   `BLIT_TRACE_SESSION_PHASES=1` on both processes (same
   `BLIT_TRACE_RUN_ID`): resize epochs (arm queue→ready→accept/dial→ack),
   need-batch emission, planner in/out, per-socket first write/receive, and
   completion. The older `--trace-data-plane` output is NOT a timing input:
   it is initiator-only and may emit per file. P2's per-member sink
   open/write/close, claim-lock, and tar-shard timings are a separate
   high-volume probe slice so they cannot perturb the focused P1 observer.
   This P1 trace slice alone does not satisfy the pf-1 HARD GATE below.
2. **A/B the role layouts in one process**: the generic otp-3 role helper
   forces the in-stream carrier, but `transfer_session_roles.rs` already
   contains real loopback-TCP tests for both initiator layouts. The
   timing-harness variant MUST reuse or factor that TCP harness; it reports
   phase timings per layout for mixed and small fixtures. A positive
   layout-dependent delta in a named phase confirms; local ABSENCE
   does not kill H1 (loopback removes the Windows↔Mac topology). So
   that H1 stays falsifiable: if the local run is negative, pf-1
   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
   CELLS fixtures) before pf-1 may close — every hypothesis exits
   pf-1 confirmed or killed, never "unfalsified" (review round 2).
3. **Historical control, then bisect P2**: old push is deleted from
   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
   build and run; the control is an old-vs-new run on identical
   fixtures. The new tracing spans do NOT exist in `0f922de` (review
   round 2), so the control is observed externally — phase boundaries
   from wire + filesystem timestamps and stdout progress, with event
   semantics mapped span-for-span to the new names — or, where that is
   too coarse, a minimal probe backport onto the pinned `0f922de`
   source with identical event names. Either way every timed
   configuration runs an instrumentation-on/off pair to bound observer
   overhead (per-member tracing across ~10k files can perturb a
   double-digit share of the measured gap). Experiments, corrected per
   review 2026-07-12: (a) precreate-vs-not stays but is
   environmental-only (it cannot attribute code); (b) the flush/
   instrument toggles missed the tar-shard path — instrument the
   tar-shard write path itself; (c) REPLACED (review round 2) — the
   ramp pin discriminated nothing (old push also opened at one
   stream), but H4 keeps a code-level counterfactual: a batch-cadence
   replay toggle that processes need batches at the recorded old-push
   shard-boundary cadence; (d) NEW, for H5 — the overlap experiment,
   metric DEFINED (review round 2: "manifest-complete→first-payload
   gap" was underdefined, and for old push the quantity is expected to
   be NEGATIVE, which an unsigned "gap" cannot express). Record, per
   run, on ONE common clock with a SIGNED offset from the
   `ManifestComplete` event, three separately-named events on the
   source side plus one on the destination:
   `t_manifest_complete`; `t_first_payload_queued` (the payload enters
   the send queue); `t_first_socket_write` (first byte handed to the
   TCP data plane); `t_first_payload_received` (destination side —
   requires the two clocks to be reconciled, so record the ssh/NTP
   offset per run and report it with the number, or state that the
   destination event was not usable). The overlap DIFFERENCE is
   established only if `t_first_socket_write − t_manifest_complete` is
   ≈0-or-positive on the new build and provably NEGATIVE on the pinned
   `0f922de` control for the SAME fixture — i.e. old push really did put
   TCP bytes on the wire before its manifest completed, and the new
   session does not.
   **That timestamp proves ORDERING, not CAUSATION, so it cannot confirm
   H5 (review round 3).** H5 is confirmed only by a causal
   counterfactual: a debug-flag toggle that restores mid-manifest TCP
   payload queueing (queueing/ordering only — if it cannot be done
   without a wire change, this plan's Contract stop-and-amend rule fires
   FIRST) and measures WALL TIME on the same fixture and rig,
   interleaved old-vs-new. Pre-registered: H5 is CONFIRMED iff the
   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
   KILLED if it restores the old ordering but does not move wall time —
   which would prove the lost overlap is real and irrelevant, and hand
   P2 to H6;
   (e) per-member locking/framing timings are now an unconditional pf-1
   measurement (they discriminate H6), not contingent on the trace
   implicating them.
4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
   local rig is Mac↔Mac loopback: it removes the very platform terms P1
   is confounded with, and it may equally fail to surface P2 (whose
   Windows arms are the sharpest). So the rule is symmetric — **if a
   finding does not reproduce locally, pf-1 REQUIRES the rig-side
   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
   since P2 was measured on both) with the same spans and the CELLS
   fixtures, before pf-1 may close. Every hypothesis exits pf-1
   confirmed or killed — never "did not reproduce, moving on".
5. Every experiment lands as a committed probe record under
   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   loop per slice as usual.

## pf-1 decision rule — UNIFORM, pre-registered (added round 5)

Round-4 review: individual hypotheses had no shared decision threshold —
H1 accepted any positive phase delta, H4's cadence replay had no
threshold, H5 left a 1–49% recovery undecided, H6 left "material share"
undefined. A phase-timing delta is **descriptive**; only wall time
decides. So ONE rule governs every hypothesis (H1, H4, H5, H6, H7):

- Each hypothesis must have a **wall-time counterfactual**: a debug-flag
  variant that removes or restores exactly the accused mechanism, run
  interleaved against the unmodified build on the same rig and fixture.
  A hypothesis with no counterfactual **cannot be confirmed** — it is
  carried as UNTESTED and pf-1 does not close.
- **`Δ` is defined per finding and per rig — it is NOT one number**
  (review round 5: the earlier text left it ambiguous between P1's
  layout gap and P2's old/new gap, which are different quantities):
  - **`Δ_P1(rig)`** = `destinit_median − srcinit_median` for
    `wm_tcp_mixed` on THAT rig (an invariance gap: new-vs-new, no old
    build involved). On rig W it is 1221 − 939 = **282 ms** — a **single
    nagatha session**; §pf-0 re-estimates it from four sessions on the `q`
    pairing, rules out **between-session** grading of any counterfactual, and
    requires pf-1 to measure its own **paired within-session** floor before
    grading. Read §pf-0 before grading any recovery against `Δ_P1`. On
    magneto↔skippy it is ~0 (8/8 pass) — so
    **P1 counterfactuals are graded on rig W only**; a Linux-rig recovery is
    meaningless against a gap that does not exist there.
  - **`Δ_P2(rig)`** = `new_median − old_same_session_median` for
    `push_tcp_small` on THAT rig (a converge gap, requires the `0f922de`
    build on that rig). netwatch-01: 1975 − 1644 = **331 ms**; zoey:
    4033 − 3636 = **397 ms**.
  Every reported recovery names its `Δ` and its rig. A counterfactual run
  on a rig whose `Δ` is ~0 proves nothing and is not reported as a kill.
- **Overlapping causes are attributed SEQUENTIALLY, never summed**
  (review round 5: H4/H7, and H6/H7, can each recover the same
  milliseconds, so independent recoveries would double-count and could
  "explain" >100% of `Δ`). Procedure: grade each hypothesis's recovery
  ALONE against the unmodified build; then, for every confirmed
  hypothesis in descending order of solo recovery, measure the
  **incremental** recovery of adding it to the already-applied set. The
  ≥70% closure test below is evaluated on the **cumulative combined**
  build, not on the sum of solo recoveries.
- The counterfactual's wall-time recovery `r` (as a share of the named
  `Δ`) is graded on a **pre-registered scale**, no post-hoc bands:
  - `r ≥ 50%` → **CONFIRMED DOMINANT** (fix it first)
  - `20% ≤ r < 50%` → **CONFIRMED CONTRIBUTING** (fix it, but it is not
    the whole story — keep hunting)
  - `r < 20%` → **KILLED** as a material cause (recorded, not pursued)
- **pf-1 closes only when the confirmed contributions account for ≥ 70%
  of `Δ`** for each finding. If they do not, the residue is unexplained
  and pf-1 **stays open** with the shortfall stated in the probe record —
  never "several hypotheses were consistent, moving on".
- Every measurement runs instrumentation-on/off pairs (per-member tracing
  across ~10k files can itself perturb a double-digit share of `Δ`).

## Fix criteria (pre-registered; the owner walks the final numbers)

- **The global rule dominates every bar below** (review round 2 flagged
  a contradiction between "necessary, not sufficient" and the `⇔`
  bars — the `⇔`s are hereby scoped as *definitions of the named
  finding's own bar*, never as a sufficient condition for acceptance).
  Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in
  EVERY acceptance cell passes independently against BOTH its
  same-session reference AND the committed baseline — no arm may exceed
  1.10 against either reference even when its counterpart bar passes
  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
  P2 bars below but regresses any other cell against either reference is
  **not** accepted.
- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
  `pull_tcp_mixed` ≤ 1.10 against BOTH references on the netwatch-01
  rig (CELLS escalation session, RUNS=8), with `wm_grpc_mixed` and the
  other invariance PASSes unregressed against both references. (Meeting
  this bar does not by itself accept the build — see the global rule.)
- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
  (same-session AND committed) on BOTH rigs (CELLS sessions), with the
  gRPC small-push cells unregressed. **"Unregressed" is given a
  reference and a tolerance (review round 3)**: each gRPC small-push
  cell must stay ≤ 1.10 against both of its own references AND must not
  worsen by more than **10% against its own pre-fix median on the same
  rig** (zoey 4731 ms; netwatch-01 2264 ms at 12c-win). The second
  clause exists because those cells currently range 0.801–1.001 — a fix
  that dragged Windows gRPC from 0.85 back to 1.05 would still pass a
  bare ≤1.10 bar while having eaten a real, measured win.
- Cross-direction converge-up is a SEPARATE bar (review round 2):
  every final cross-direction row must still meet the parent plan's
  new-vs-old ceiling (`ONE_TRANSFER_PATH.md` acceptance) or satisfy
  the registered platform-residue discriminator — invariance plus the
  per-direction bars alone would pass if a "fix" slowed BOTH layouts
  equally, violating converge-up.
- No suite regressions; the test count may not drop from the immediately
  preceding reviewed workspace baseline recorded by the repo. Any new pins
  carry guard proofs (temporary revert) per the loop.
- If investigation attributes part of a gap to something the plan's
  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
  that residue is RECORDED with its experiment and goes to the owner's
  otp-13 walk — never silently accepted.

## Staging (each through the codex loop)

- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
  the two-layout phase-timing report (TCP-carrier mode included) + the
  `0f922de` historical control; probe record committed AND
  codex-reviewed BEFORE any pf-2 branch exists. No fix lands on
  pre-pf-1 evidence.
- **pf-2..n**: one fix slice per confirmed root cause (smallest
  change that moves the phase timing; A/B'd locally before rig time).
- **pf-final**: NOT just the two escalation cells — the final build
  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
  gRPC controls) on **all THREE rigs: Z (zoey), W (netwatch-01) and
  D (delegated, netwatch-01↔skippy)**. **No mixed-build evidence: every
  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
  (corrected, review round 2 — "every row" was impossible: the
  same-session `old` arms and the committed baselines are OLD builds by
  construction, which is the entire point of a reference). Pre-fix
  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
  which are **replication and control evidence, not acceptance
  evidence**.
  **Rig D is included even though it is not a suspect (review round
  3).** Voiding otp-12c's pre-fix rows while re-running only Z and W
  would leave the parent plan's **delegated-parity bar**
  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
  evidence at all. "Not implicated" scopes what pf-1 must
  *instrument* — it does not waive an acceptance bar. Rig D's TCP
  verdict cells (+ the gRPC smoke) therefore rerun on the final build;
  both arms are new-build by construction there (rig D has no old
  baseline), so the whole cell is re-measured.
  **Every gRPC row the acceptance method requires reruns
  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
  earlier "if shared code changed, the gRPC cells rerun too" left the
  decision to the author's own judgement of what counts as shared, which
  is exactly the loophole H7 exploits: a shared regression can hide under
  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
  complete Z/W gRPC converge and invariance rows, so those are
  final-build rows, full stop — no conditional. Results land in fresh
  dated evidence dirs. **Then** otp-12d assembles the matrix from
  final-build rows, and the otp-13 owner walk reads it.

## Known gaps

- H1–H5 were graded against the actual tree by codex review
  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
  The old drivers are deleted from HEAD, but the pinned `0f922de`
  source/binaries diff and run fine — historical claims get live
  controls in pf-1, not pin-archaeology.
- zoey never measured P1: its rig anchors converge-up only, so there
  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
  says nothing about layout asymmetry (review 2026-07-12). pf-1's
  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
  qualifies per the 12b wm numbers).
- **The 12c-win rows are replication, not acceptance** (2026-07-13).
  They are pre-fix by definition, so `pf-final` voids them for
  acceptance; their value is that they (a) reproduce P1 and P2 on an
  independent session at the shipped sha, (b) supply the
  opposite-direction control (`mw_tcp_mixed` 1.044 PASS vs
  `wm_tcp_mixed` 1.300 FAIL — same carrier, same fixture) that narrows
  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
  P2 1.149→1.201), so neither is drifting toward the bar on its own.
- **Rig-D delegated parity is not a SUSPECT, but it is still an
  ACCEPTANCE bar** (2026-07-13; scoped correctly at review round 3): the
  delegated-vs-direct matrix passed 7/7
  (`docs/bench/otp12c-delegated-2026-07-13/`), so delegation adds no
  measurable cost and pf-1 need not instrument the delegated trigger
  path. That is a statement about *where to look for the bug* — it does
  **not** waive the parent plan's delegated-parity bar, whose evidence
  is pre-fix and therefore void under pf-final. Rig D reruns on the
  final build (see pf-final).

exec
/bin/zsh -lc 'rg -n "''^(class |def |[A-Z][A-Z0-9_]*'"\\s*=)|schedule|128|role|source|destination|durab|settled|total_ms|trace|marker|manifest|final|cleanup|N_pair|resolution|observer|evidence\" scripts/otp12pf_rigw_analyze.py scripts/otp12pf_rigw_analyze_test.py" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/otp12pf_rigw_analyze_test.py:17:MODULE_PATH = Path(__file__).with_name("otp12pf_rigw_analyze.py")
scripts/otp12pf_rigw_analyze_test.py:18:SPEC = importlib.util.spec_from_file_location("otp12pf_rigw_analyze", MODULE_PATH)
scripts/otp12pf_rigw_analyze_test.py:25:class SyntheticSession:
scripts/otp12pf_rigw_analyze_test.py:33:        self._build_manifest_evidence()
scripts/otp12pf_rigw_analyze_test.py:38:    def _delta(trace_state: str, cell: str, pair: int) -> int:
scripts/otp12pf_rigw_analyze_test.py:41:        if trace_state == "off":
scripts/otp12pf_rigw_analyze_test.py:45:    def _trace_events(
scripts/otp12pf_rigw_analyze_test.py:46:        self, run_id: str, session_id: str, scheduled_role: str
scripts/otp12pf_rigw_analyze_test.py:48:        initiator = "SOURCE" if scheduled_role == "source_init" else "DESTINATION"
scripts/otp12pf_rigw_analyze_test.py:49:        source_action = "dial" if initiator == "SOURCE" else "accept"
scripts/otp12pf_rigw_analyze_test.py:50:        destination_action = "accept" if initiator == "SOURCE" else "dial"
scripts/otp12pf_rigw_analyze_test.py:53:            endpoint_role: str,
scripts/otp12pf_rigw_analyze_test.py:66:                "endpoint_role": endpoint_role,
scripts/otp12pf_rigw_analyze_test.py:67:                "initiator_role": initiator,
scripts/otp12pf_rigw_analyze_test.py:73:        source: list[dict[str, object]] = []
scripts/otp12pf_rigw_analyze_test.py:74:        destination: list[dict[str, object]] = []
scripts/otp12pf_rigw_analyze_test.py:76:        def source_event(name: str, **extra: object) -> None:
scripts/otp12pf_rigw_analyze_test.py:77:            seq = len(source)
scripts/otp12pf_rigw_analyze_test.py:78:            source.append(event("SOURCE", seq, seq, name, **extra))
scripts/otp12pf_rigw_analyze_test.py:80:        def destination_event(name: str, **extra: object) -> None:
scripts/otp12pf_rigw_analyze_test.py:81:            seq = len(destination)
scripts/otp12pf_rigw_analyze_test.py:82:            destination.append(event("DESTINATION", seq, seq, name, **extra))
scripts/otp12pf_rigw_analyze_test.py:84:        source_event(f"socket_{source_action}_begin", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:85:        source_event(f"socket_{source_action}_end", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:86:        source_event("socket_trace_attached", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:87:        source_event("manifest_complete_send_begin")
scripts/otp12pf_rigw_analyze_test.py:88:        source_event("manifest_complete_sent", count=1)
scripts/otp12pf_rigw_analyze_test.py:89:        source_event("need_batch_received", batch=0, count=1)
scripts/otp12pf_rigw_analyze_test.py:90:        source_event("planner_begin", batch=0, count=1)
scripts/otp12pf_rigw_analyze_test.py:91:        source_event("planner_end", batch=0, count=1)
scripts/otp12pf_rigw_analyze_test.py:94:            source_event(
scripts/otp12pf_rigw_analyze_test.py:100:            source_event(
scripts/otp12pf_rigw_analyze_test.py:106:            source_event(
scripts/otp12pf_rigw_analyze_test.py:112:            source_event(
scripts/otp12pf_rigw_analyze_test.py:118:            source_event(f"socket_{source_action}_begin", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:119:            source_event(f"socket_{source_action}_end", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:120:            source_event("socket_trace_attached", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:121:            source_event(
scripts/otp12pf_rigw_analyze_test.py:122:                "source_settled",
scripts/otp12pf_rigw_analyze_test.py:128:        source_event("first_payload_queued")
scripts/otp12pf_rigw_analyze_test.py:129:        source_event("socket_write_begin", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:130:        source_event("first_socket_write", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:131:        source_event("data_plane_complete")
scripts/otp12pf_rigw_analyze_test.py:132:        source_event("summary_received")
scripts/otp12pf_rigw_analyze_test.py:134:        destination_event(f"socket_{destination_action}_begin", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:135:        destination_event(f"socket_{destination_action}_end", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:136:        destination_event("socket_trace_attached", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:137:        destination_event("manifest_complete_received")
scripts/otp12pf_rigw_analyze_test.py:138:        destination_event("need_batch_send_begin", batch=0, count=1)
scripts/otp12pf_rigw_analyze_test.py:139:        destination_event("need_batch_sent", batch=0, count=1)
scripts/otp12pf_rigw_analyze_test.py:142:            destination_event(
scripts/otp12pf_rigw_analyze_test.py:149:                destination_event(
scripts/otp12pf_rigw_analyze_test.py:154:                destination_event(
scripts/otp12pf_rigw_analyze_test.py:155:                    "destination_prepared",
scripts/otp12pf_rigw_analyze_test.py:160:                destination_event(
scripts/otp12pf_rigw_analyze_test.py:166:                destination_event(
scripts/otp12pf_rigw_analyze_test.py:172:                destination_event("resize_arm_ready", epoch=epoch)
scripts/otp12pf_rigw_analyze_test.py:173:                destination_event("socket_accept_begin", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:174:                destination_event("socket_accept_end", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:175:                destination_event("socket_trace_attached", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:177:                destination_event("socket_dial_begin", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:178:                destination_event("socket_dial_end", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:179:                destination_event("socket_trace_attached", epoch=epoch, socket=0)
scripts/otp12pf_rigw_analyze_test.py:180:                destination_event(
scripts/otp12pf_rigw_analyze_test.py:181:                    "destination_prepared",
scripts/otp12pf_rigw_analyze_test.py:186:                destination_event(
scripts/otp12pf_rigw_analyze_test.py:192:                destination_event(
scripts/otp12pf_rigw_analyze_test.py:198:        destination_event("first_payload_received", epoch=0, socket=0)
scripts/otp12pf_rigw_analyze_test.py:199:        destination_event("data_plane_complete")
scripts/otp12pf_rigw_analyze_test.py:200:        destination_event("summary_send_begin")
scripts/otp12pf_rigw_analyze_test.py:201:        destination_event("summary_sent")
scripts/otp12pf_rigw_analyze_test.py:202:        return source + destination
scripts/otp12pf_rigw_analyze_test.py:212:                    for role_order, role in enumerate(
scripts/otp12pf_rigw_analyze_test.py:213:                        analyzer.expected_roles(pair), start=1
scripts/otp12pf_rigw_analyze_test.py:215:                        source_ms = 100
scripts/otp12pf_rigw_analyze_test.py:217:                            source_ms
scripts/otp12pf_rigw_analyze_test.py:218:                            if role == "source_init"
scripts/otp12pf_rigw_analyze_test.py:219:                            else source_ms + self._delta(block.trace_state, cell, pair)
scripts/otp12pf_rigw_analyze_test.py:221:                        settled_ms = 250
scripts/otp12pf_rigw_analyze_test.py:224:                            f"client/b{block.number}-{cell}-p{pair}-{role}.log"
scripts/otp12pf_rigw_analyze_test.py:227:                        traced_tcp = block.trace_state == "on" and cell in analyzer.TCP_CELLS
scripts/otp12pf_rigw_analyze_test.py:229:                        if traced_tcp:
scripts/otp12pf_rigw_analyze_test.py:232:                            self.events.extend(self._trace_events(run_id, session_id, role))
scripts/otp12pf_rigw_analyze_test.py:236:                                "trace_state": block.trace_state,
scripts/otp12pf_rigw_analyze_test.py:239:                                "role": role,
scripts/otp12pf_rigw_analyze_test.py:241:                                "role_order": str(role_order),
scripts/otp12pf_rigw_analyze_test.py:243:                                "settled_ms": str(settled_ms),
scripts/otp12pf_rigw_analyze_test.py:245:                                "total_ms": str(
scripts/otp12pf_rigw_analyze_test.py:247:                                    + settled_ms
scripts/otp12pf_rigw_analyze_test.py:277:                            "role": row["role"],
scripts/otp12pf_rigw_analyze_test.py:290:    def _manifest_data(shape: str) -> bytes:
scripts/otp12pf_rigw_analyze_test.py:302:    def _build_manifest_evidence(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:310:            data = self._manifest_data(shape)
scripts/otp12pf_rigw_analyze_test.py:312:            q_relative = f"fixtures/src_{shape}.manifest"
scripts/otp12pf_rigw_analyze_test.py:313:            win_relative = f"fixtures/windows-src_{shape}.manifest"
scripts/otp12pf_rigw_analyze_test.py:320:                    "q_manifest": q_relative,
scripts/otp12pf_rigw_analyze_test.py:321:                    "windows_manifest": win_relative,
scripts/otp12pf_rigw_analyze_test.py:325:        with (self.root / "fixture-manifests.csv").open("w", newline="") as handle:
scripts/otp12pf_rigw_analyze_test.py:328:                fieldnames=("shape", "sha256", "q_manifest", "windows_manifest"),
scripts/otp12pf_rigw_analyze_test.py:336:            row["tree_manifest_sha256"] = digest
scripts/otp12pf_rigw_analyze_test.py:338:                f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
scripts/otp12pf_rigw_analyze_test.py:340:            (landed / f"{rid}.manifest").write_bytes(data)
scripts/otp12pf_rigw_analyze_test.py:351:        trace = self.root / "trace" / "nested"
scripts/otp12pf_rigw_analyze_test.py:352:        trace.mkdir(parents=True, exist_ok=True)
scripts/otp12pf_rigw_analyze_test.py:360:        with (trace / "daemon.log").open("w") as handle:
scripts/otp12pf_rigw_analyze_test.py:365:                    event["endpoint_role"] == event["initiator_role"]
scripts/otp12pf_rigw_analyze_test.py:374:class RigWAnalyzerTests(unittest.TestCase):
scripts/otp12pf_rigw_analyze_test.py:380:    def traced_session_id(session: SyntheticSession, initiator_role: str) -> str:
scripts/otp12pf_rigw_analyze_test.py:385:                if event["initiator_role"] == initiator_role
scripts/otp12pf_rigw_analyze_test.py:393:        endpoint_role: str,
scripts/otp12pf_rigw_analyze_test.py:401:            and event["endpoint_role"] == endpoint_role
scripts/otp12pf_rigw_analyze_test.py:410:        assert len({event["endpoint_role"] for event in desired_order}) == 1
scripts/otp12pf_rigw_analyze_test.py:419:    def test_complete_schedule_exact_floor_bias_and_exports(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:421:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:423:        self.assertEqual(str(result.observer_bias), "20")
scripts/otp12pf_rigw_analyze_test.py:424:        self.assertEqual(str(result.n_resolution), "70")
scripts/otp12pf_rigw_analyze_test.py:427:                (row["cell"], row["trace_state"]): row
scripts/otp12pf_rigw_analyze_test.py:432:        self.assertEqual(off["measurand"], "durable_total_ms")
scripts/otp12pf_rigw_analyze_test.py:439:        self.assertEqual(off["source_first_delta_median_ms"], "45")
scripts/otp12pf_rigw_analyze_test.py:440:        self.assertEqual(off["destination_first_delta_median_ms"], "45")
scripts/otp12pf_rigw_analyze_test.py:441:        self.assertEqual(off["role_order_drift_ms"], "0")
scripts/otp12pf_rigw_analyze_test.py:448:        self.assertEqual(on["observer_bias_ms"], "20")
scripts/otp12pf_rigw_analyze_test.py:449:        self.assertEqual(on["n_resolution_ms"], "70")
scripts/otp12pf_rigw_analyze_test.py:454:        self.assertEqual(len(clocks), 128)
scripts/otp12pf_rigw_analyze_test.py:461:        self.assertTrue(any(row["source_file"].startswith("client/") for row in phase_rows))
scripts/otp12pf_rigw_analyze_test.py:462:        self.assertTrue(any(row["source_file"].startswith("trace/") for row in phase_rows))
scripts/otp12pf_rigw_analyze_test.py:465:                row["total_ms"]
scripts/otp12pf_rigw_analyze_test.py:468:                    + int(row["settled_ms"])
scripts/otp12pf_rigw_analyze_test.py:479:        self.assertTrue(all(row["endpoint_role"] in {"SOURCE", "DESTINATION"} for row in intervals))
scripts/otp12pf_rigw_analyze_test.py:481:    def test_registered_schedule_is_pair_outer_with_reverse_block_bases(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:482:        schedule = analyzer.expected_schedule()
scripts/otp12pf_rigw_analyze_test.py:487:                for block, cell, scheduled_pair, _role, role_order in schedule
scripts/otp12pf_rigw_analyze_test.py:489:                and scheduled_pair == pair
scripts/otp12pf_rigw_analyze_test.py:490:                and role_order == 1
scripts/otp12pf_rigw_analyze_test.py:503:                role
scripts/otp12pf_rigw_analyze_test.py:504:                for block, _cell, pair, role, role_order in schedule
scripts/otp12pf_rigw_analyze_test.py:505:                if block.number == 1 and role_order == 1 and _cell == base[0]
scripts/otp12pf_rigw_analyze_test.py:507:            ["source_init", "destination_init", "destination_init", "source_init"],
scripts/otp12pf_rigw_analyze_test.py:510:    def test_missing_trace_endpoint_is_rejected(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:512:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:519:                and event["endpoint_role"] == "DESTINATION"
scripts/otp12pf_rigw_analyze_test.py:523:        with self.assertRaisesRegex(analyzer.AnalysisError, "missing endpoint role"):
scripts/otp12pf_rigw_analyze_test.py:526:    def test_trace_off_leak_is_rejected(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:528:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:534:        with self.assertRaisesRegex(analyzer.AnalysisError, "trace leak: trace-off block 1"):
scripts/otp12pf_rigw_analyze_test.py:537:    def test_grpc_trace_leak_is_rejected(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:539:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:548:    def test_schedule_mismatch_is_rejected(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:550:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:553:        with self.assertRaisesRegex(analyzer.AnalysisError, "schedule mismatch"):
scripts/otp12pf_rigw_analyze_test.py:556:    def test_settled_ms_schema_and_bounds_are_fail_closed(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:558:            with self.subTest(settled_ms=value):
scripts/otp12pf_rigw_analyze_test.py:560:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:561:                session.rows[0]["settled_ms"] = value
scripts/otp12pf_rigw_analyze_test.py:563:                with self.assertRaisesRegex(analyzer.AnalysisError, "settled_ms"):
scripts/otp12pf_rigw_analyze_test.py:567:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:570:        lines[0] = lines[0].replace("settled_ms,", "")
scripts/otp12pf_rigw_analyze_test.py:577:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:578:        session.rows[0]["total_ms"] = "999"
scripts/otp12pf_rigw_analyze_test.py:582:            "total_ms must equal transfer_ms \\+ \\(settled_ms - 250\\) \\+ flush_ms",
scripts/otp12pf_rigw_analyze_test.py:586:    def test_role_specific_flush_is_included_in_delta_and_floor(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:588:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:589:        destination_flush = (18, 16, 14, 12, 10, 8, 6, 4)
scripts/otp12pf_rigw_analyze_test.py:591:            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
scripts/otp12pf_rigw_analyze_test.py:595:                if row["role"] == "source_init"
scripts/otp12pf_rigw_analyze_test.py:596:                else destination_flush[int(row["pair"]) - 1]
scripts/otp12pf_rigw_analyze_test.py:599:            row["total_ms"] = str(
scripts/otp12pf_rigw_analyze_test.py:601:                + int(row["settled_ms"])
scripts/otp12pf_rigw_analyze_test.py:610:                (row["cell"], row["trace_state"]): row
scripts/otp12pf_rigw_analyze_test.py:618:        self.assertEqual(str(result.n_resolution), "56")
scripts/otp12pf_rigw_analyze_test.py:620:    def test_excess_settle_is_charged_without_false_role_delta(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:622:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:624:            "source_init": set(),
scripts/otp12pf_rigw_analyze_test.py:625:            "destination_init": set(),
scripts/otp12pf_rigw_analyze_test.py:629:            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
scripts/otp12pf_rigw_analyze_test.py:632:            if row["role"] == "source_init":
scripts/otp12pf_rigw_analyze_test.py:633:                settled_ms, flush_ms = 999, 1
scripts/otp12pf_rigw_analyze_test.py:635:                settled_ms, flush_ms = 250, 750
scripts/otp12pf_rigw_analyze_test.py:637:            row["settled_ms"] = str(settled_ms)
scripts/otp12pf_rigw_analyze_test.py:639:            row["total_ms"] = str(
scripts/otp12pf_rigw_analyze_test.py:641:                + settled_ms
scripts/otp12pf_rigw_analyze_test.py:645:            old_formula_totals[row["role"]].add(transfer_ms + flush_ms)
scripts/otp12pf_rigw_analyze_test.py:646:            actual_elapsed.add(transfer_ms + settled_ms + flush_ms)
scripts/otp12pf_rigw_analyze_test.py:648:        self.assertEqual(old_formula_totals["source_init"], {101})
scripts/otp12pf_rigw_analyze_test.py:649:        self.assertEqual(old_formula_totals["destination_init"], {850})
scripts/otp12pf_rigw_analyze_test.py:655:                (row["cell"], row["trace_state"]): row
scripts/otp12pf_rigw_analyze_test.py:659:        self.assertEqual(off["source_init_median_ms"], "850")
scripts/otp12pf_rigw_analyze_test.py:660:        self.assertEqual(off["destination_init_median_ms"], "850")
scripts/otp12pf_rigw_analyze_test.py:666:    def test_landed_manifest_rejects_swapped_sizes_and_renamed_paths(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:674:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:687:                    f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
scripts/otp12pf_rigw_analyze_test.py:689:                (session.root / "landed" / f"{rid}.manifest").write_bytes(data)
scripts/otp12pf_rigw_analyze_test.py:690:                row["tree_manifest_sha256"] = digest
scripts/otp12pf_rigw_analyze_test.py:694:                    "landed relative-path/size manifest does not match canonical",
scripts/otp12pf_rigw_analyze_test.py:698:    def test_landed_root_and_recorded_manifest_digest_are_exact(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:700:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:707:        self.addCleanup(temporary_digest.cleanup)
scripts/otp12pf_rigw_analyze_test.py:708:        digest_session.rows[0]["tree_manifest_sha256"] = "0" * 64
scripts/otp12pf_rigw_analyze_test.py:711:            analyzer.AnalysisError, "landed manifest digest mismatch"
scripts/otp12pf_rigw_analyze_test.py:717:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:722:                and event["endpoint_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:731:    def test_payload_write_must_precede_source_completion(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:733:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:739:            and event["endpoint_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:746:            and event["endpoint_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:758:    def test_socket_action_end_must_precede_trace_attachment(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:760:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:766:            and event["endpoint_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:775:            and event["endpoint_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:776:            and event["event"] == "socket_trace_attached"
scripts/otp12pf_rigw_analyze_test.py:784:            "SOURCE/socket_.*_end -> SOURCE/socket_trace_attached",
scripts/otp12pf_rigw_analyze_test.py:790:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:796:            and event["endpoint_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:797:            and event["event"] == "socket_trace_attached"
scripts/otp12pf_rigw_analyze_test.py:804:            and event["endpoint_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:812:            "SOURCE/socket_trace_attached -> SOURCE/socket_write_begin",
scripts/otp12pf_rigw_analyze_test.py:816:    def test_destination_resize_prerequisites_are_causal(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:835:                "socket_trace_attached",
scripts/otp12pf_rigw_analyze_test.py:836:                "destination_prepared",
scripts/otp12pf_rigw_analyze_test.py:839:        for initiator_role, start_name, end_name in cases:
scripts/otp12pf_rigw_analyze_test.py:841:                initiator_role=initiator_role,
scripts/otp12pf_rigw_analyze_test.py:845:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:846:                session_id = self.traced_session_id(session, initiator_role)
scripts/otp12pf_rigw_analyze_test.py:861:    def test_source_resize_prerequisites_are_causal(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:862:        for initiator_role, source_action in (
scripts/otp12pf_rigw_analyze_test.py:867:                initiator_role=initiator_role,
scripts/otp12pf_rigw_analyze_test.py:868:                edge=f"resize_sent->socket_{source_action}_begin",
scripts/otp12pf_rigw_analyze_test.py:871:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:872:                session_id = self.traced_session_id(session, initiator_role)
scripts/otp12pf_rigw_analyze_test.py:883:                    f"socket_{source_action}_begin",
scripts/otp12pf_rigw_analyze_test.py:890:                    f"SOURCE/resize_sent -> SOURCE/socket_{source_action}_begin",
scripts/otp12pf_rigw_analyze_test.py:895:                initiator_role=initiator_role,
scripts/otp12pf_rigw_analyze_test.py:896:                edge="socket_trace_attached->source_settled",
scripts/otp12pf_rigw_analyze_test.py:899:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:900:                session_id = self.traced_session_id(session, initiator_role)
scripts/otp12pf_rigw_analyze_test.py:902:                    session, session_id, "SOURCE", "socket_trace_attached", 1
scripts/otp12pf_rigw_analyze_test.py:904:                settled = self.phase_event(
scripts/otp12pf_rigw_analyze_test.py:905:                    session, session_id, "SOURCE", "source_settled", 1
scripts/otp12pf_rigw_analyze_test.py:907:                self.reorder_local_events([settled, attached])
scripts/otp12pf_rigw_analyze_test.py:911:                    "SOURCE/socket_trace_attached -> SOURCE/source_settled",
scripts/otp12pf_rigw_analyze_test.py:915:    def test_final_resize_settlement_precedes_data_plane_completion(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:916:        for initiator_role in ("SOURCE", "DESTINATION"):
scripts/otp12pf_rigw_analyze_test.py:918:                initiator_role=initiator_role,
scripts/otp12pf_rigw_analyze_test.py:919:                edge="SOURCE/source_settled->data_plane_complete",
scripts/otp12pf_rigw_analyze_test.py:922:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:923:                session_id = self.traced_session_id(session, initiator_role)
scripts/otp12pf_rigw_analyze_test.py:924:                settled = self.phase_event(
scripts/otp12pf_rigw_analyze_test.py:925:                    session, session_id, "SOURCE", "source_settled", 7
scripts/otp12pf_rigw_analyze_test.py:940:                    [first_queued, write_begin, first_write, complete, settled]
scripts/otp12pf_rigw_analyze_test.py:945:                    "SOURCE/source_settled -> SOURCE/data_plane_complete",
scripts/otp12pf_rigw_analyze_test.py:950:                initiator_role=initiator_role,
scripts/otp12pf_rigw_analyze_test.py:954:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:955:                session_id = self.traced_session_id(session, initiator_role)
scripts/otp12pf_rigw_analyze_test.py:981:    def test_destination_preparation_action_is_role_correlated(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:983:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:987:            if event["event"] == "destination_prepared"
scripts/otp12pf_rigw_analyze_test.py:988:            and event["initiator_role"] == "SOURCE"
scripts/otp12pf_rigw_analyze_test.py:997:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:1006:        for endpoint_role in ("SOURCE", "DESTINATION"):
scripts/otp12pf_rigw_analyze_test.py:1007:            role_events = [
scripts/otp12pf_rigw_analyze_test.py:1011:                and event["endpoint_role"] == endpoint_role
scripts/otp12pf_rigw_analyze_test.py:1014:                sorted(role_events, key=lambda item: int(item["producer_seq"]))
scripts/otp12pf_rigw_analyze_test.py:1023:    def test_final_resize_target_and_live_fields_are_exact_on_both_roles(self) -> None:
scripts/otp12pf_rigw_analyze_test.py:1025:            ("SOURCE", "source_settled", "target_streams"),
scripts/otp12pf_rigw_analyze_test.py:1026:            ("SOURCE", "source_settled", "live_streams"),
scripts/otp12pf_rigw_analyze_test.py:1030:        for endpoint_role, event_name, field in mutations:
scripts/otp12pf_rigw_analyze_test.py:1032:                endpoint_role=endpoint_role, event=event_name, field=field
scripts/otp12pf_rigw_analyze_test.py:1035:                self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze_test.py:1037:                marker = next(
scripts/otp12pf_rigw_analyze_test.py:1041:                    and event["endpoint_role"] == endpoint_role
scripts/otp12pf_rigw_analyze_test.py:1045:                marker[field] = 7
scripts/otp12pf_rigw_analyze_test.py:1049:                    f"{endpoint_role}/{event_name} epoch 7 {field} must be 8",
scripts/otp12pf_rigw_analyze_test.py:1055:        self.addCleanup(temporary.cleanup)
scripts/otp12pf_rigw_analyze.py:5:four-block schedule and writes reports only after the CSV and every structured
scripts/otp12pf_rigw_analyze.py:6:TCP trace have passed validation.  Phase intervals are derived exclusively
scripts/otp12pf_rigw_analyze.py:7:from one endpoint's ``elapsed_ns`` clock; ``unix_ns`` is retained as evidence
scripts/otp12pf_rigw_analyze.py:30:CELLS = (
scripts/otp12pf_rigw_analyze.py:36:TCP_CELLS = frozenset(cell for cell in CELLS if "_tcp_" in cell)
scripts/otp12pf_rigw_analyze.py:37:TARGET_CELL = "wm_tcp_mixed"
scripts/otp12pf_rigw_analyze.py:38:ROLES = ("source_init", "destination_init")
scripts/otp12pf_rigw_analyze.py:39:CSV_FIELDS = (
scripts/otp12pf_rigw_analyze.py:41:    "trace_state",
scripts/otp12pf_rigw_analyze.py:44:    "role",
scripts/otp12pf_rigw_analyze.py:46:    "role_order",
scripts/otp12pf_rigw_analyze.py:48:    "settled_ms",
scripts/otp12pf_rigw_analyze.py:50:    "total_ms",
scripts/otp12pf_rigw_analyze.py:52:    "tree_manifest_sha256",
scripts/otp12pf_rigw_analyze.py:60:CLOCK_FIELDS = (
scripts/otp12pf_rigw_analyze.py:65:    "role",
scripts/otp12pf_rigw_analyze.py:74:TRACE_PREFIX = "[session-phase] "
scripts/otp12pf_rigw_analyze.py:75:SESSION_ID_RE = re.compile(r"^[0-9a-f]{16}$")
scripts/otp12pf_rigw_analyze.py:76:SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
scripts/otp12pf_rigw_analyze.py:77:SETTLE_MIN_MS = 250
scripts/otp12pf_rigw_analyze.py:78:SETTLE_MAX_MS = 1000
scripts/otp12pf_rigw_analyze.py:79:MEASURAND = "durable_total_ms"
scripts/otp12pf_rigw_analyze.py:83:class BlockSpec:
scripts/otp12pf_rigw_analyze.py:85:    trace_state: str
scripts/otp12pf_rigw_analyze.py:91:BLOCKS = (
scripts/otp12pf_rigw_analyze.py:99:class AnalysisError(RuntimeError):
scripts/otp12pf_rigw_analyze.py:100:    """The evidence is incomplete, contaminated, or off schedule."""
scripts/otp12pf_rigw_analyze.py:104:class RunRow:
scripts/otp12pf_rigw_analyze.py:106:    schedule_index: int
scripts/otp12pf_rigw_analyze.py:108:    trace_state: str
scripts/otp12pf_rigw_analyze.py:111:    role: str
scripts/otp12pf_rigw_analyze.py:113:    role_order: int
scripts/otp12pf_rigw_analyze.py:115:    settled_ms: int
scripts/otp12pf_rigw_analyze.py:117:    total_ms: Decimal
scripts/otp12pf_rigw_analyze.py:119:    tree_manifest_sha256: str
scripts/otp12pf_rigw_analyze.py:129:class TraceEvent:
scripts/otp12pf_rigw_analyze.py:130:    source_file: str
scripts/otp12pf_rigw_analyze.py:131:    source_line: int
scripts/otp12pf_rigw_analyze.py:143:    def endpoint_role(self) -> str:
scripts/otp12pf_rigw_analyze.py:144:        return self.raw["endpoint_role"]
scripts/otp12pf_rigw_analyze.py:160:class ClockSample:
scripts/otp12pf_rigw_analyze.py:173:class ModeDescription:
scripts/otp12pf_rigw_analyze.py:187:class ConditionStats:
scripts/otp12pf_rigw_analyze.py:189:    trace_state: str
scripts/otp12pf_rigw_analyze.py:190:    source_values: tuple[Decimal, ...]
scripts/otp12pf_rigw_analyze.py:191:    destination_values: tuple[Decimal, ...]
scripts/otp12pf_rigw_analyze.py:193:    source_median: Decimal
scripts/otp12pf_rigw_analyze.py:194:    destination_median: Decimal
scripts/otp12pf_rigw_analyze.py:203:    source_first_delta_median: Decimal
scripts/otp12pf_rigw_analyze.py:204:    destination_first_delta_median: Decimal
scripts/otp12pf_rigw_analyze.py:205:    role_order_drift: Decimal
scripts/otp12pf_rigw_analyze.py:212:class AnalysisResult:
scripts/otp12pf_rigw_analyze.py:219:    observer_bias: Decimal
scripts/otp12pf_rigw_analyze.py:220:    n_resolution: Decimal
scripts/otp12pf_rigw_analyze.py:221:    trace_event_count: int
scripts/otp12pf_rigw_analyze.py:224:def decimal_text(value: Decimal) -> str:
scripts/otp12pf_rigw_analyze.py:230:def parse_decimal(value: str, field: str, line: int) -> Decimal:
scripts/otp12pf_rigw_analyze.py:242:def parse_int(value: str, field: str, line: int, source: str = "runs.csv") -> int:
scripts/otp12pf_rigw_analyze.py:247:            f"{source} line {line}: {field} is not an integer: {value!r}"
scripts/otp12pf_rigw_analyze.py:251:def expected_roles(pair: int) -> tuple[str, str]:
scripts/otp12pf_rigw_analyze.py:257:def expected_schedule() -> list[tuple[BlockSpec, str, int, str, int]]:
scripts/otp12pf_rigw_analyze.py:263:                for role_order, role in enumerate(
scripts/otp12pf_rigw_analyze.py:264:                    expected_roles(pair), start=1
scripts/otp12pf_rigw_analyze.py:266:                    expected.append((block, cell, pair, role, role_order))
scripts/otp12pf_rigw_analyze.py:270:def _safe_client_log(root: Path, value: str, line: int) -> None:
scripts/otp12pf_rigw_analyze.py:284:def _read_tree_manifest(path: Path, label: str) -> tuple[bytes, str]:
scripts/otp12pf_rigw_analyze.py:289:        raise AnalysisError(f"{label}: manifest must be non-empty and newline-terminated")
scripts/otp12pf_rigw_analyze.py:293:        raise AnalysisError(f"{label}: manifest is not ASCII") from exc
scripts/otp12pf_rigw_analyze.py:296:        raise AnalysisError(f"{label}: manifest lines are not exact sorted unique inventory")
scripts/otp12pf_rigw_analyze.py:321:def _load_fixture_manifests(root: Path) -> dict[str, tuple[bytes, str]]:
scripts/otp12pf_rigw_analyze.py:322:    index_path = root / "fixture-manifests.csv"
scripts/otp12pf_rigw_analyze.py:327:        fields = ("shape", "sha256", "q_manifest", "windows_manifest")
scripts/otp12pf_rigw_analyze.py:329:            raise AnalysisError("fixture-manifests.csv header mismatch")
scripts/otp12pf_rigw_analyze.py:332:        raise AnalysisError("fixture-manifests.csv must contain mixed,large exactly")
scripts/otp12pf_rigw_analyze.py:335:        f"src_{shape}.manifest" for shape in ("mixed", "large")
scripts/otp12pf_rigw_analyze.py:336:    } | {f"windows-src_{shape}.manifest" for shape in ("mixed", "large")}
scripts/otp12pf_rigw_analyze.py:348:            "fixture manifest file inventory mismatch: expected "
scripts/otp12pf_rigw_analyze.py:355:        q_relative = f"fixtures/src_{shape}.manifest"
scripts/otp12pf_rigw_analyze.py:356:        win_relative = f"fixtures/windows-src_{shape}.manifest"
scripts/otp12pf_rigw_analyze.py:357:        if row["q_manifest"] != q_relative or row["windows_manifest"] != win_relative:
scripts/otp12pf_rigw_analyze.py:358:            raise AnalysisError(f"fixture-manifests.csv {shape}: path mapping mismatch")
scripts/otp12pf_rigw_analyze.py:359:        q_data, q_digest = _read_tree_manifest(root / q_relative, f"q src_{shape}")
scripts/otp12pf_rigw_analyze.py:360:        win_data, win_digest = _read_tree_manifest(
scripts/otp12pf_rigw_analyze.py:364:            raise AnalysisError(f"canonical q/Windows src_{shape} manifests differ")
scripts/otp12pf_rigw_analyze.py:366:            raise AnalysisError(f"fixture-manifests.csv {shape}: digest mismatch")
scripts/otp12pf_rigw_analyze.py:371:def load_runs(root: Path) -> list[RunRow]:
scripts/otp12pf_rigw_analyze.py:372:    fixture_manifests = _load_fixture_manifests(root)
scripts/otp12pf_rigw_analyze.py:387:    schedule = expected_schedule()
scripts/otp12pf_rigw_analyze.py:388:    if len(raw_rows) != len(schedule):
scripts/otp12pf_rigw_analyze.py:390:            f"runs.csv schedule incomplete: expected {len(schedule)} rows, got {len(raw_rows)}"
scripts/otp12pf_rigw_analyze.py:394:    for index, (raw, expected) in enumerate(zip(raw_rows, schedule), start=0):
scripts/otp12pf_rigw_analyze.py:396:        block_spec, cell, pair, role, role_order = expected
scripts/otp12pf_rigw_analyze.py:397:        actual_schedule = (
scripts/otp12pf_rigw_analyze.py:399:            raw["trace_state"],
scripts/otp12pf_rigw_analyze.py:403:            raw["role"],
scripts/otp12pf_rigw_analyze.py:404:            parse_int(raw["role_order"], "role_order", line),
scripts/otp12pf_rigw_analyze.py:406:        wanted_schedule = (
scripts/otp12pf_rigw_analyze.py:408:            block_spec.trace_state,
scripts/otp12pf_rigw_analyze.py:412:            role,
scripts/otp12pf_rigw_analyze.py:413:            role_order,
scripts/otp12pf_rigw_analyze.py:415:        if actual_schedule != wanted_schedule:
scripts/otp12pf_rigw_analyze.py:417:                f"runs.csv line {line}: schedule mismatch; expected {wanted_schedule}, "
scripts/otp12pf_rigw_analyze.py:418:                f"got {actual_schedule}"
scripts/otp12pf_rigw_analyze.py:430:        traced_tcp = block_spec.trace_state == "on" and cell in TCP_CELLS
scripts/otp12pf_rigw_analyze.py:431:        if traced_tcp:
scripts/otp12pf_rigw_analyze.py:434:                    f"runs.csv line {line}: trace-on TCP session_id must be 16 lowercase hex"
scripts/otp12pf_rigw_analyze.py:438:                f"runs.csv line {line}: session_id must be blank for trace-off or gRPC arms"
scripts/otp12pf_rigw_analyze.py:441:        settled_ms = parse_int(raw["settled_ms"], "settled_ms", line)
scripts/otp12pf_rigw_analyze.py:442:        if not SETTLE_MIN_MS <= settled_ms < SETTLE_MAX_MS:
scripts/otp12pf_rigw_analyze.py:444:                f"runs.csv line {line}: settled_ms must be in "
scripts/otp12pf_rigw_analyze.py:445:                f"[{SETTLE_MIN_MS},{SETTLE_MAX_MS}), got {settled_ms}"
scripts/otp12pf_rigw_analyze.py:449:        total_ms = parse_decimal(raw["total_ms"], "total_ms", line)
scripts/otp12pf_rigw_analyze.py:450:        settle_excess_ms = Decimal(settled_ms - SETTLE_MIN_MS)
scripts/otp12pf_rigw_analyze.py:451:        expected_total_ms = transfer_ms + settle_excess_ms + flush_ms
scripts/otp12pf_rigw_analyze.py:452:        if total_ms != expected_total_ms:
scripts/otp12pf_rigw_analyze.py:454:                f"runs.csv line {line}: total_ms must equal transfer_ms + "
scripts/otp12pf_rigw_analyze.py:455:                f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms "
scripts/otp12pf_rigw_analyze.py:456:                f"exactly; got {decimal_text(total_ms)} != "
scripts/otp12pf_rigw_analyze.py:457:                f"{decimal_text(transfer_ms)} + ({settled_ms} - "
scripts/otp12pf_rigw_analyze.py:467:        recorded_digest = raw["tree_manifest_sha256"]
scripts/otp12pf_rigw_analyze.py:470:                f"runs.csv line {line}: tree_manifest_sha256 must be 64 lowercase hex"
scripts/otp12pf_rigw_analyze.py:472:        rid = f"b{block_spec.number}_{cell}_p{pair}_{role}"
scripts/otp12pf_rigw_analyze.py:473:        landed_data, landed_digest = _read_tree_manifest(
scripts/otp12pf_rigw_analyze.py:474:            root / "landed" / f"{rid}.manifest", f"landed manifest {rid}"
scripts/otp12pf_rigw_analyze.py:476:        canonical_data, canonical_digest = fixture_manifests[shape]
scripts/otp12pf_rigw_analyze.py:478:            raise AnalysisError(f"runs.csv line {line}: landed manifest digest mismatch")
scripts/otp12pf_rigw_analyze.py:481:                f"runs.csv line {line}: landed relative-path/size manifest "
scripts/otp12pf_rigw_analyze.py:487:                schedule_index=index,
scripts/otp12pf_rigw_analyze.py:489:                trace_state=block_spec.trace_state,
scripts/otp12pf_rigw_analyze.py:492:                role=role,
scripts/otp12pf_rigw_analyze.py:494:                role_order=role_order,
scripts/otp12pf_rigw_analyze.py:496:                settled_ms=settled_ms,
scripts/otp12pf_rigw_analyze.py:498:                total_ms=total_ms,
scripts/otp12pf_rigw_analyze.py:500:                tree_manifest_sha256=recorded_digest,
scripts/otp12pf_rigw_analyze.py:512:        f"b{row.block}_{row.cell}_p{row.pair}_{row.role}.manifest"
scripts/otp12pf_rigw_analyze.py:526:            "landed manifest file inventory mismatch: expected exactly 128 registered "
scripts/otp12pf_rigw_analyze.py:543:        if row.trace_state == "on" and row.cell in TCP_CELLS
scripts/otp12pf_rigw_analyze.py:546:        raise AnalysisError("trace-on TCP (run_id, session_id) values must be unique")
scripts/otp12pf_rigw_analyze.py:550:def load_clock_samples(root: Path, rows: Sequence[RunRow]) -> list[ClockSample]:
scripts/otp12pf_rigw_analyze.py:585:            raw["role"],
scripts/otp12pf_rigw_analyze.py:594:            run.role,
scripts/otp12pf_rigw_analyze.py:600:                f"clock-samples.csv line {line}: schedule mismatch; expected "
scripts/otp12pf_rigw_analyze.py:652:def _require_json_string(raw: dict[str, Any], name: str, where: str) -> None:
scripts/otp12pf_rigw_analyze.py:657:def _require_json_int(raw: dict[str, Any], name: str, where: str) -> None:
scripts/otp12pf_rigw_analyze.py:663:def load_trace_events(root: Path) -> list[TraceEvent]:
scripts/otp12pf_rigw_analyze.py:664:    evidence_roots = (root / "trace", root / "client")
scripts/otp12pf_rigw_analyze.py:665:    for evidence_root in evidence_roots:
scripts/otp12pf_rigw_analyze.py:666:        if not evidence_root.is_dir():
scripts/otp12pf_rigw_analyze.py:667:            raise AnalysisError(f"missing trace evidence directory: {evidence_root}")
scripts/otp12pf_rigw_analyze.py:670:    for evidence_root in evidence_roots:
scripts/otp12pf_rigw_analyze.py:671:        for candidate in sorted(path for path in evidence_root.rglob("*") if path.is_file()):
scripts/otp12pf_rigw_analyze.py:699:                        "endpoint_role",
scripts/otp12pf_rigw_analyze.py:700:                        "initiator_role",
scripts/otp12pf_rigw_analyze.py:708:                    if raw["endpoint_role"] not in ("SOURCE", "DESTINATION"):
scripts/otp12pf_rigw_analyze.py:709:                        raise AnalysisError(f"{where}: invalid endpoint_role")
scripts/otp12pf_rigw_analyze.py:710:                    if raw["initiator_role"] not in ("SOURCE", "DESTINATION"):
scripts/otp12pf_rigw_analyze.py:711:                        raise AnalysisError(f"{where}: invalid initiator_role")
scripts/otp12pf_rigw_analyze.py:726:            raise AnalysisError(f"{relative}: trace log is not UTF-8") from exc
scripts/otp12pf_rigw_analyze.py:730:def _one_event(events: Sequence[TraceEvent], role: str, name: str, label: str) -> TraceEvent:
scripts/otp12pf_rigw_analyze.py:731:    found = [event for event in events if event.endpoint_role == role and event.event == name]
scripts/otp12pf_rigw_analyze.py:733:        raise AnalysisError(f"{label}: expected one {role}/{name}, got {len(found)}")
scripts/otp12pf_rigw_analyze.py:737:def _correlation_keys(
scripts/otp12pf_rigw_analyze.py:738:    events: Sequence[TraceEvent], role: str, name: str, label: str
scripts/otp12pf_rigw_analyze.py:740:    selected = [event for event in events if event.endpoint_role == role and event.event == name]
scripts/otp12pf_rigw_analyze.py:746:            raise AnalysisError(f"{label}: {role}/{name} lacks epoch/socket correlation")
scripts/otp12pf_rigw_analyze.py:749:        raise AnalysisError(f"{label}: duplicate {role}/{name} epoch/socket marker")
scripts/otp12pf_rigw_analyze.py:753:def _marker_map(
scripts/otp12pf_rigw_analyze.py:755:    role: str,
scripts/otp12pf_rigw_analyze.py:762:    selected = [event for event in events if event.endpoint_role == role and event.event == name]
scripts/otp12pf_rigw_analyze.py:764:        raise AnalysisError(f"{label}: missing {role}/{name} inventory")
scripts/otp12pf_rigw_analyze.py:771:                raise AnalysisError(f"{label}: {role}/{name} lacks {field} correlation")
scripts/otp12pf_rigw_analyze.py:775:            raise AnalysisError(f"{label}: duplicate {role}/{name} marker for {key}")
scripts/otp12pf_rigw_analyze.py:780:def _assert_same_keys(
scripts/otp12pf_rigw_analyze.py:785:    for name, markers in named_maps[1:]:
scripts/otp12pf_rigw_analyze.py:786:        if set(markers) != wanted:
scripts/otp12pf_rigw_analyze.py:789:                f"vs {name}={sorted(markers)}"
scripts/otp12pf_rigw_analyze.py:794:def _assert_before(label: str, start: TraceEvent, end: TraceEvent) -> None:
scripts/otp12pf_rigw_analyze.py:796:        start.endpoint_role != end.endpoint_role
scripts/otp12pf_rigw_analyze.py:801:            f"{label}: invalid local sequence {start.endpoint_role}/{start.event} "
scripts/otp12pf_rigw_analyze.py:802:            f"-> {end.endpoint_role}/{end.event}"
scripts/otp12pf_rigw_analyze.py:806:def _assert_event_fields(
scripts/otp12pf_rigw_analyze.py:814:                f"{label}: {event.endpoint_role}/{event.event} epoch {epoch} "
scripts/otp12pf_rigw_analyze.py:819:def validate_traces(
scripts/otp12pf_rigw_analyze.py:828:        if row.trace_state == "on" and row.cell in TCP_CELLS
scripts/otp12pf_rigw_analyze.py:836:                state = next(row.trace_state for row in rows if row.block == block)
scripts/otp12pf_rigw_analyze.py:839:                        f"trace leak: trace-off block {block} emitted {event.session_id} "
scripts/otp12pf_rigw_analyze.py:840:                        f"at {event.source_file}:{event.source_line}"
scripts/otp12pf_rigw_analyze.py:843:                    f"trace leak: block {block} emitted an unregistered (including possible "
scripts/otp12pf_rigw_analyze.py:845:                    f"{event.source_file}:{event.source_line}"
scripts/otp12pf_rigw_analyze.py:848:                f"stale/foreign trace run_id {event.run_id!r} at "
scripts/otp12pf_rigw_analyze.py:849:                f"{event.source_file}:{event.source_line}"
scripts/otp12pf_rigw_analyze.py:858:            f"missing trace for block {row.block} {row.cell} pair {row.pair} "
scripts/otp12pf_rigw_analyze.py:859:            f"{row.role} ({run_id}/{session_id}); {len(missing)} session(s) missing"
scripts/otp12pf_rigw_analyze.py:865:            f"block {row.block} {row.cell} pair {row.pair} {row.role} "
scripts/otp12pf_rigw_analyze.py:868:        expected_initiator = "SOURCE" if row.role == "source_init" else "DESTINATION"
scripts/otp12pf_rigw_analyze.py:869:        roles = {event.endpoint_role for event in group}
scripts/otp12pf_rigw_analyze.py:870:        if roles != {"SOURCE", "DESTINATION"}:
scripts/otp12pf_rigw_analyze.py:872:                f"{label}: missing endpoint role; expected SOURCE+DESTINATION, got {sorted(roles)}"
scripts/otp12pf_rigw_analyze.py:874:        if {event.raw["initiator_role"] for event in group} != {expected_initiator}:
scripts/otp12pf_rigw_analyze.py:875:            raise AnalysisError(f"{label}: initiator_role does not match scheduled role")
scripts/otp12pf_rigw_analyze.py:877:        by_role: dict[str, list[TraceEvent]] = {}
scripts/otp12pf_rigw_analyze.py:879:            by_role.setdefault(event.endpoint_role, []).append(event)
scripts/otp12pf_rigw_analyze.py:880:        for endpoint_role, endpoint_events in by_role.items():
scripts/otp12pf_rigw_analyze.py:884:                    f"{label}: {endpoint_role} producer_seq is not exact contiguous 0..n-1: "
scripts/otp12pf_rigw_analyze.py:888:        manifest_begin = _one_event(
scripts/otp12pf_rigw_analyze.py:889:            group, "SOURCE", "manifest_complete_send_begin", label
scripts/otp12pf_rigw_analyze.py:891:        manifest_sent = _one_event(group, "SOURCE", "manifest_complete_sent", label)
scripts/otp12pf_rigw_analyze.py:892:        _one_event(group, "DESTINATION", "manifest_complete_received", label)
scripts/otp12pf_rigw_analyze.py:894:        _assert_before(label, manifest_begin, manifest_sent)
scripts/otp12pf_rigw_analyze.py:895:        _assert_before(label, manifest_sent, first_queued)
scripts/otp12pf_rigw_analyze.py:897:        need_begin = _marker_map(
scripts/otp12pf_rigw_analyze.py:900:        need_sent = _marker_map(
scripts/otp12pf_rigw_analyze.py:903:        need_received = _marker_map(
scripts/otp12pf_rigw_analyze.py:929:        planner_begin = _marker_map(
scripts/otp12pf_rigw_analyze.py:932:        planner_end = _marker_map(group, "SOURCE", "planner_end", ("batch",), label)
scripts/otp12pf_rigw_analyze.py:946:            ("resize_proposed", _marker_map(group, "SOURCE", "resize_proposed", ("epoch",), label)),
scripts/otp12pf_rigw_analyze.py:949:                _marker_map(group, "SOURCE", "resize_send_begin", ("epoch",), label),
scripts/otp12pf_rigw_analyze.py:951:            ("resize_sent", _marker_map(group, "SOURCE", "resize_sent", ("epoch",), label)),
scripts/otp12pf_rigw_analyze.py:954:                _marker_map(group, "DESTINATION", "resize_received", ("epoch",), label),
scripts/otp12pf_rigw_analyze.py:957:                "destination_prepared",
scripts/otp12pf_rigw_analyze.py:958:                _marker_map(group, "DESTINATION", "destination_prepared", ("epoch",), label),
scripts/otp12pf_rigw_analyze.py:962:                _marker_map(
scripts/otp12pf_rigw_analyze.py:968:                _marker_map(group, "DESTINATION", "resize_ack_sent", ("epoch",), label),
scripts/otp12pf_rigw_analyze.py:972:                _marker_map(group, "SOURCE", "resize_ack_received", ("epoch",), label),
scripts/otp12pf_rigw_analyze.py:975:                "source_settled",
scripts/otp12pf_rigw_analyze.py:976:                _marker_map(group, "SOURCE", "source_settled", ("epoch",), label),
scripts/otp12pf_rigw_analyze.py:998:                resize["destination_prepared"][key],
scripts/otp12pf_rigw_analyze.py:1002:                resize["destination_prepared"][key],
scripts/otp12pf_rigw_analyze.py:1011:                label, resize["resize_ack_received"][key], resize["source_settled"][key]
scripts/otp12pf_rigw_analyze.py:1026:                resize["destination_prepared"][key],
scripts/otp12pf_rigw_analyze.py:1042:                resize["source_settled"][key],
scripts/otp12pf_rigw_analyze.py:1049:            if resize["destination_prepared"][key].raw.get("action") != expected_prepared_action:
scripts/otp12pf_rigw_analyze.py:1051:                    f"{label}: resize epoch {key[0]} destination_prepared action must be "
scripts/otp12pf_rigw_analyze.py:1057:                resize["source_settled"][(epoch,)],
scripts/otp12pf_rigw_analyze.py:1066:        source_complete = _one_event(group, "SOURCE", "data_plane_complete", label)
scripts/otp12pf_rigw_analyze.py:1067:        source_summary = _one_event(group, "SOURCE", "summary_received", label)
scripts/otp12pf_rigw_analyze.py:1068:        destination_complete = _one_event(group, "DESTINATION", "data_plane_complete", label)
scripts/otp12pf_rigw_analyze.py:1069:        destination_summary_begin = _one_event(
scripts/otp12pf_rigw_analyze.py:1072:        destination_summary = _one_event(group, "DESTINATION", "summary_sent", label)
scripts/otp12pf_rigw_analyze.py:1073:        if source_complete.producer_seq >= source_summary.producer_seq:
scripts/otp12pf_rigw_analyze.py:1076:            destination_complete.producer_seq
scripts/otp12pf_rigw_analyze.py:1077:            < destination_summary_begin.producer_seq
scripts/otp12pf_rigw_analyze.py:1078:            < destination_summary.producer_seq
scripts/otp12pf_rigw_analyze.py:1081:        _assert_before(label, resize["source_settled"][(7,)], source_complete)
scripts/otp12pf_rigw_analyze.py:1082:        _assert_before(label, resize["resize_ack_sent"][(7,)], destination_complete)
scripts/otp12pf_rigw_analyze.py:1084:        source_attachment_events = _marker_map(
scripts/otp12pf_rigw_analyze.py:1087:            "socket_trace_attached",
scripts/otp12pf_rigw_analyze.py:1091:        destination_attachment_events = _marker_map(
scripts/otp12pf_rigw_analyze.py:1094:            "socket_trace_attached",
scripts/otp12pf_rigw_analyze.py:1098:        source_attached = set(source_attachment_events)
scripts/otp12pf_rigw_analyze.py:1099:        destination_attached = set(destination_attachment_events)
scripts/otp12pf_rigw_analyze.py:1100:        if not source_attached or source_attached != destination_attached:
scripts/otp12pf_rigw_analyze.py:1101:            raise AnalysisError(f"{label}: two-role socket attachment correlation mismatch")
scripts/otp12pf_rigw_analyze.py:1103:        if source_attached != expected_attached:
scripts/otp12pf_rigw_analyze.py:1105:                f"{label}: socket attachment inventory {sorted(source_attached)} does not "
scripts/otp12pf_rigw_analyze.py:1108:        for endpoint_role, complete in (
scripts/otp12pf_rigw_analyze.py:1109:            ("SOURCE", source_complete),
scripts/otp12pf_rigw_analyze.py:1110:            ("DESTINATION", destination_complete),
scripts/otp12pf_rigw_analyze.py:1115:                if event.endpoint_role == endpoint_role
scripts/otp12pf_rigw_analyze.py:1116:                and event.event == "socket_trace_attached"
scripts/otp12pf_rigw_analyze.py:1120:        source_action = "dial" if expected_initiator == "SOURCE" else "accept"
scripts/otp12pf_rigw_analyze.py:1121:        destination_action = "accept" if expected_initiator == "SOURCE" else "dial"
scripts/otp12pf_rigw_analyze.py:1125:        for endpoint_role, action in (
scripts/otp12pf_rigw_analyze.py:1126:            ("SOURCE", source_action),
scripts/otp12pf_rigw_analyze.py:1127:            ("DESTINATION", destination_action),
scripts/otp12pf_rigw_analyze.py:1129:            begins = _marker_map(
scripts/otp12pf_rigw_analyze.py:1131:                endpoint_role,
scripts/otp12pf_rigw_analyze.py:1136:            ends = _marker_map(
scripts/otp12pf_rigw_analyze.py:1138:                endpoint_role,
scripts/otp12pf_rigw_analyze.py:1145:                ((f"{endpoint_role}_{action}_begin", begins), (f"{endpoint_role}_{action}_end", ends)),
scripts/otp12pf_rigw_analyze.py:1148:                raise AnalysisError(f"{label}: {endpoint_role} socket action inventory mismatch")
scripts/otp12pf_rigw_analyze.py:1150:            if _marker_map(
scripts/otp12pf_rigw_analyze.py:1152:                endpoint_role,
scripts/otp12pf_rigw_analyze.py:1159:                    f"{label}: {endpoint_role} unexpectedly mixed dial and accept actions"
scripts/otp12pf_rigw_analyze.py:1162:                source_attachment_events
scripts/otp12pf_rigw_analyze.py:1163:                if endpoint_role == "SOURCE"
scripts/otp12pf_rigw_analyze.py:1164:                else destination_attachment_events
scripts/otp12pf_rigw_analyze.py:1169:            action_events[endpoint_role] = (begins, ends)
scripts/otp12pf_rigw_analyze.py:1171:        source_action_begins, source_action_ends = action_events["SOURCE"]
scripts/otp12pf_rigw_analyze.py:1172:        destination_action_begins, destination_action_ends = action_events[
scripts/otp12pf_rigw_analyze.py:1180:                source_action_begins[action_key],
scripts/otp12pf_rigw_analyze.py:1185:                source_action_begins[action_key],
scripts/otp12pf_rigw_analyze.py:1189:                source_action_ends[action_key],
scripts/otp12pf_rigw_analyze.py:1190:                resize["source_settled"][(epoch,)],
scripts/otp12pf_rigw_analyze.py:1194:                source_attachment_events[action_key],
scripts/otp12pf_rigw_analyze.py:1195:                resize["source_settled"][(epoch,)],
scripts/otp12pf_rigw_analyze.py:1198:        arm_begin = _marker_map(
scripts/otp12pf_rigw_analyze.py:1206:        arm_ready = _marker_map(
scripts/otp12pf_rigw_analyze.py:1219:                raise AnalysisError(f"{label}: destination resize-arm inventory mismatch")
scripts/otp12pf_rigw_analyze.py:1234:                    resize["destination_prepared"][arm_key],
scripts/otp12pf_rigw_analyze.py:1240:                    destination_action_begins[(arm_key[0], 0)],
scripts/otp12pf_rigw_analyze.py:1245:                    destination_action_begins[(arm_key[0], 0)],
scripts/otp12pf_rigw_analyze.py:1248:            raise AnalysisError(f"{label}: destination initiator unexpectedly emitted arm events")
scripts/otp12pf_rigw_analyze.py:1254:                    destination_action_begins[(epoch, 0)],
scripts/otp12pf_rigw_analyze.py:1258:                    destination_action_ends[(epoch, 0)],
scripts/otp12pf_rigw_analyze.py:1259:                    resize["destination_prepared"][(epoch,)],
scripts/otp12pf_rigw_analyze.py:1263:                    destination_attachment_events[(epoch, 0)],
scripts/otp12pf_rigw_analyze.py:1264:                    resize["destination_prepared"][(epoch,)],
scripts/otp12pf_rigw_analyze.py:1266:        write_begin_events = _marker_map(
scripts/otp12pf_rigw_analyze.py:1273:        write_events = _marker_map(
scripts/otp12pf_rigw_analyze.py:1280:        receive_events = _marker_map(
scripts/otp12pf_rigw_analyze.py:1292:        if not writes.issubset(source_attached):
scripts/otp12pf_rigw_analyze.py:1293:            raise AnalysisError(f"{label}: SOURCE payload socket was not trace-attached")
scripts/otp12pf_rigw_analyze.py:1294:        if not receives.issubset(destination_attached):
scripts/otp12pf_rigw_analyze.py:1295:            raise AnalysisError(f"{label}: DESTINATION payload socket was not trace-attached")
scripts/otp12pf_rigw_analyze.py:1301:            _assert_before(label, source_attachment_events[action_key], begin)
scripts/otp12pf_rigw_analyze.py:1303:            _assert_before(label, write, source_complete)
scripts/otp12pf_rigw_analyze.py:1305:                label, destination_attachment_events[action_key], received
scripts/otp12pf_rigw_analyze.py:1307:            _assert_before(label, received, destination_complete)
scripts/otp12pf_rigw_analyze.py:1311:def condition_stats(rows: Sequence[RunRow], cell: str, trace_state: str) -> ConditionStats:
scripts/otp12pf_rigw_analyze.py:1313:        row for row in rows if row.cell == cell and row.trace_state == trace_state
scripts/otp12pf_rigw_analyze.py:1317:        if row.role in by_pair.setdefault(row.pair, {}):
scripts/otp12pf_rigw_analyze.py:1319:                f"duplicate timing for {cell}/{trace_state}/pair {row.pair}/{row.role}"
scripts/otp12pf_rigw_analyze.py:1321:        by_pair[row.pair][row.role] = row.total_ms
scripts/otp12pf_rigw_analyze.py:1324:            f"{cell}/{trace_state}: expected paired observations 1..8, got {sorted(by_pair)}"
scripts/otp12pf_rigw_analyze.py:1328:            raise AnalysisError(f"{cell}/{trace_state}/pair {pair}: incomplete role pair")
scripts/otp12pf_rigw_analyze.py:1329:    source = tuple(by_pair[pair]["source_init"] for pair in range(1, 9))
scripts/otp12pf_rigw_analyze.py:1330:    destination = tuple(by_pair[pair]["destination_init"] for pair in range(1, 9))
scripts/otp12pf_rigw_analyze.py:1331:    deltas = tuple(dest - src for src, dest in zip(source, destination))
scripts/otp12pf_rigw_analyze.py:1332:    source_first_pairs = {
scripts/otp12pf_rigw_analyze.py:1333:        row.pair for row in selected if row.role == "source_init" and row.role_order == 1
scripts/otp12pf_rigw_analyze.py:1335:    destination_first_pairs = {
scripts/otp12pf_rigw_analyze.py:1338:        if row.role == "destination_init" and row.role_order == 1
scripts/otp12pf_rigw_analyze.py:1340:    if source_first_pairs | destination_first_pairs != set(range(1, 9)):
scripts/otp12pf_rigw_analyze.py:1341:        raise AnalysisError(f"{cell}/{trace_state}: incomplete role-order partition")
scripts/otp12pf_rigw_analyze.py:1342:    if source_first_pairs & destination_first_pairs:
scripts/otp12pf_rigw_analyze.py:1343:        raise AnalysisError(f"{cell}/{trace_state}: overlapping role-order partition")
scripts/otp12pf_rigw_analyze.py:1344:    source_first = median(
scripts/otp12pf_rigw_analyze.py:1345:        tuple(deltas[pair - 1] for pair in sorted(source_first_pairs))
scripts/otp12pf_rigw_analyze.py:1347:    destination_first = median(
scripts/otp12pf_rigw_analyze.py:1348:        tuple(deltas[pair - 1] for pair in sorted(destination_first_pairs))
scripts/otp12pf_rigw_analyze.py:1354:    source_median = median(source)
scripts/otp12pf_rigw_analyze.py:1355:    destination_median = median(destination)
scripts/otp12pf_rigw_analyze.py:1358:        trace_state=trace_state,
scripts/otp12pf_rigw_analyze.py:1359:        source_values=source,
scripts/otp12pf_rigw_analyze.py:1360:        destination_values=destination,
scripts/otp12pf_rigw_analyze.py:1362:        source_median=source_median,
scripts/otp12pf_rigw_analyze.py:1363:        destination_median=destination_median,
scripts/otp12pf_rigw_analyze.py:1364:        delta=destination_median - source_median,
scripts/otp12pf_rigw_analyze.py:1372:        source_first_delta_median=source_first,
scripts/otp12pf_rigw_analyze.py:1373:        destination_first_delta_median=destination_first,
scripts/otp12pf_rigw_analyze.py:1374:        role_order_drift=abs(source_first - destination_first),
scripts/otp12pf_rigw_analyze.py:1380:            abs(source_first - destination_first),
scripts/otp12pf_rigw_analyze.py:1386:def largest_gap_modes(values: Iterable[Decimal]) -> ModeDescription:
scripts/otp12pf_rigw_analyze.py:1398:def _atomic_csv(path: Path, fields: Sequence[str], rows: Iterable[dict[str, Any]]) -> None:
scripts/otp12pf_rigw_analyze.py:1409:def _atomic_text(path: Path, contents: str) -> None:
scripts/otp12pf_rigw_analyze.py:1418:def _summary_rows(
scripts/otp12pf_rigw_analyze.py:1419:    stats: Sequence[ConditionStats], observer_bias: Decimal, n_resolution: Decimal
scripts/otp12pf_rigw_analyze.py:1423:        source_modes = largest_gap_modes(item.source_values)
scripts/otp12pf_rigw_analyze.py:1424:        destination_modes = largest_gap_modes(item.destination_values)
scripts/otp12pf_rigw_analyze.py:1430:                "trace_state": item.trace_state,
scripts/otp12pf_rigw_analyze.py:1433:                "source_init_median_ms": decimal_text(item.source_median),
scripts/otp12pf_rigw_analyze.py:1434:                "destination_init_median_ms": decimal_text(item.destination_median),
scripts/otp12pf_rigw_analyze.py:1443:                "source_first_delta_median_ms": decimal_text(
scripts/otp12pf_rigw_analyze.py:1444:                    item.source_first_delta_median
scripts/otp12pf_rigw_analyze.py:1446:                "destination_first_delta_median_ms": decimal_text(
scripts/otp12pf_rigw_analyze.py:1447:                    item.destination_first_delta_median
scripts/otp12pf_rigw_analyze.py:1449:                "role_order_drift_ms": decimal_text(item.role_order_drift),
scripts/otp12pf_rigw_analyze.py:1453:                "observer_bias_ms": decimal_text(observer_bias) if target else "",
scripts/otp12pf_rigw_analyze.py:1454:                "n_resolution_ms": decimal_text(n_resolution) if target else "",
scripts/otp12pf_rigw_analyze.py:1455:                "source_init_sorted_ms": ";".join(
scripts/otp12pf_rigw_analyze.py:1456:                    decimal_text(value) for value in sorted(item.source_values)
scripts/otp12pf_rigw_analyze.py:1458:                "destination_init_sorted_ms": ";".join(
scripts/otp12pf_rigw_analyze.py:1459:                    decimal_text(value) for value in sorted(item.destination_values)
scripts/otp12pf_rigw_analyze.py:1464:                "source_init_largest_gap_ms": decimal_text(source_modes.gap),
scripts/otp12pf_rigw_analyze.py:1465:                "source_init_descriptive_modes_ms": source_modes.render(),
scripts/otp12pf_rigw_analyze.py:1466:                "destination_init_largest_gap_ms": decimal_text(destination_modes.gap),
scripts/otp12pf_rigw_analyze.py:1467:                "destination_init_descriptive_modes_ms": destination_modes.render(),
scripts/otp12pf_rigw_analyze.py:1475:SUMMARY_FIELDS = (
scripts/otp12pf_rigw_analyze.py:1477:    "trace_state",
scripts/otp12pf_rigw_analyze.py:1480:    "source_init_median_ms",
scripts/otp12pf_rigw_analyze.py:1481:    "destination_init_median_ms",
scripts/otp12pf_rigw_analyze.py:1490:    "source_first_delta_median_ms",
scripts/otp12pf_rigw_analyze.py:1491:    "destination_first_delta_median_ms",
scripts/otp12pf_rigw_analyze.py:1492:    "role_order_drift_ms",
scripts/otp12pf_rigw_analyze.py:1496:    "observer_bias_ms",
scripts/otp12pf_rigw_analyze.py:1497:    "n_resolution_ms",
scripts/otp12pf_rigw_analyze.py:1498:    "source_init_sorted_ms",
scripts/otp12pf_rigw_analyze.py:1499:    "destination_init_sorted_ms",
scripts/otp12pf_rigw_analyze.py:1501:    "source_init_largest_gap_ms",
scripts/otp12pf_rigw_analyze.py:1502:    "source_init_descriptive_modes_ms",
scripts/otp12pf_rigw_analyze.py:1503:    "destination_init_largest_gap_ms",
scripts/otp12pf_rigw_analyze.py:1504:    "destination_init_descriptive_modes_ms",
scripts/otp12pf_rigw_analyze.py:1510:def _distribution_rows(stats: Sequence[ConditionStats]) -> list[dict[str, str]]:
scripts/otp12pf_rigw_analyze.py:1514:            ("source_init_total", item.source_values),
scripts/otp12pf_rigw_analyze.py:1515:            ("destination_init_total", item.destination_values),
scripts/otp12pf_rigw_analyze.py:1526:                        "trace_state": item.trace_state,
scripts/otp12pf_rigw_analyze.py:1539:CLOCK_SUMMARY_FIELDS = (
scripts/otp12pf_rigw_analyze.py:1545:    "role",
scripts/otp12pf_rigw_analyze.py:1546:    "role_order",
scripts/otp12pf_rigw_analyze.py:1558:def _clock_summary_rows(samples: Sequence[ClockSample]) -> list[dict[str, str]]:
scripts/otp12pf_rigw_analyze.py:1561:        grouped.setdefault(sample.run.schedule_index, {}).setdefault(sample.phase, []).append(sample)
scripts/otp12pf_rigw_analyze.py:1563:    for schedule_index in sorted(grouped):
scripts/otp12pf_rigw_analyze.py:1564:        phases = grouped[schedule_index]
scripts/otp12pf_rigw_analyze.py:1566:            raise AnalysisError(f"clock samples for schedule row {schedule_index} lack a phase")
scripts/otp12pf_rigw_analyze.py:1577:                "role": run.role,
scripts/otp12pf_rigw_analyze.py:1578:                "role_order": str(run.role_order),
scripts/otp12pf_rigw_analyze.py:1592:EVENT_FIELDS = (
scripts/otp12pf_rigw_analyze.py:1594:    "trace_state",
scripts/otp12pf_rigw_analyze.py:1598:    "role",
scripts/otp12pf_rigw_analyze.py:1599:    "role_order",
scripts/otp12pf_rigw_analyze.py:1601:    "settled_ms",
scripts/otp12pf_rigw_analyze.py:1603:    "total_ms",
scripts/otp12pf_rigw_analyze.py:1606:    "endpoint_role",
scripts/otp12pf_rigw_analyze.py:1607:    "initiator_role",
scripts/otp12pf_rigw_analyze.py:1620:    "source_file",
scripts/otp12pf_rigw_analyze.py:1621:    "source_line",
scripts/otp12pf_rigw_analyze.py:1625:def _event_row(row: RunRow, event: TraceEvent) -> dict[str, str]:
scripts/otp12pf_rigw_analyze.py:1629:        "trace_state": row.trace_state,
scripts/otp12pf_rigw_analyze.py:1633:        "role": row.role,
scripts/otp12pf_rigw_analyze.py:1634:        "role_order": str(row.role_order),
scripts/otp12pf_rigw_analyze.py:1636:        "settled_ms": str(row.settled_ms),
scripts/otp12pf_rigw_analyze.py:1638:        "total_ms": decimal_text(row.total_ms),
scripts/otp12pf_rigw_analyze.py:1641:        "endpoint_role": event.endpoint_role,
scripts/otp12pf_rigw_analyze.py:1642:        "initiator_role": raw["initiator_role"],
scripts/otp12pf_rigw_analyze.py:1647:        "source_file": event.source_file,
scripts/otp12pf_rigw_analyze.py:1648:        "source_line": str(event.source_line),
scripts/otp12pf_rigw_analyze.py:1668:INTERVAL_FIELDS = (
scripts/otp12pf_rigw_analyze.py:1670:    "trace_state",
scripts/otp12pf_rigw_analyze.py:1674:    "role",
scripts/otp12pf_rigw_analyze.py:1677:    "endpoint_role",
scripts/otp12pf_rigw_analyze.py:1678:    "initiator_role",
scripts/otp12pf_rigw_analyze.py:1692:SPAN_SPECS = (
scripts/otp12pf_rigw_analyze.py:1695:    ("manifest_complete_send_begin", "manifest_complete_sent", ()),
scripts/otp12pf_rigw_analyze.py:1708:def _interval_base(row: RunRow, endpoint_role: str, initiator_role: str) -> dict[str, str]:
scripts/otp12pf_rigw_analyze.py:1711:        "trace_state": row.trace_state,
scripts/otp12pf_rigw_analyze.py:1715:        "role": row.role,
scripts/otp12pf_rigw_analyze.py:1718:        "endpoint_role": endpoint_role,
scripts/otp12pf_rigw_analyze.py:1719:        "initiator_role": initiator_role,
scripts/otp12pf_rigw_analyze.py:1723:def _make_interval(
scripts/otp12pf_rigw_analyze.py:1731:    if start.endpoint_role != end.endpoint_role:
scripts/otp12pf_rigw_analyze.py:1736:            f"{row.run_id}/{row.session_id}/{start.endpoint_role}: negative local interval "
scripts/otp12pf_rigw_analyze.py:1740:        row, start.endpoint_role, start.raw["initiator_role"]
scripts/otp12pf_rigw_analyze.py:1759:def _phase_rows(
scripts/otp12pf_rigw_analyze.py:1764:    traced_rows = [
scripts/otp12pf_rigw_analyze.py:1765:        row for row in rows if row.trace_state == "on" and row.cell in TCP_CELLS
scripts/otp12pf_rigw_analyze.py:1767:    for row in traced_rows:
scripts/otp12pf_rigw_analyze.py:1769:        for endpoint_role in ("SOURCE", "DESTINATION"):
scripts/otp12pf_rigw_analyze.py:1770:            endpoint = [event for event in group if event.endpoint_role == endpoint_role]
scripts/otp12pf_rigw_analyze.py:1817:def _markdown(
scripts/otp12pf_rigw_analyze.py:1819:    observer_bias: Decimal,
scripts/otp12pf_rigw_analyze.py:1820:    n_resolution: Decimal,
scripts/otp12pf_rigw_analyze.py:1821:    trace_event_count: int,
scripts/otp12pf_rigw_analyze.py:1825:    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
scripts/otp12pf_rigw_analyze.py:1829:        "Validation: PASS — exact four-block OFF–ON–ON–OFF schedule, forward/reverse "
scripts/otp12pf_rigw_analyze.py:1830:        "cell and role ordering, 8 valid role pairs per trace state/cell, trace-off and "
scripts/otp12pf_rigw_analyze.py:1831:        "gRPC trace absence, and correlated two-role TCP terminal traces.",
scripts/otp12pf_rigw_analyze.py:1835:        "| cell | trace | source total median ms | destination total median ms | Δ total ms | paired total d median ms | N_pair_split total ms | role-order drift total ms | paired range total ms | N_pair total ms |",
scripts/otp12pf_rigw_analyze.py:1844:                    item.trace_state,
scripts/otp12pf_rigw_analyze.py:1845:                    decimal_text(item.source_median),
scripts/otp12pf_rigw_analyze.py:1846:                    decimal_text(item.destination_median),
scripts/otp12pf_rigw_analyze.py:1850:                    decimal_text(item.role_order_drift),
scripts/otp12pf_rigw_analyze.py:1860:            "The authoritative wall-time measurand is `total_ms = transfer_ms + "
scripts/otp12pf_rigw_analyze.py:1861:            f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms`: client execution plus every "
scripts/otp12pf_rigw_analyze.py:1863:            "the destination durability "
scripts/otp12pf_rigw_analyze.py:1866:            "distributions, observer bias, and resolution floors.",
scripts/otp12pf_rigw_analyze.py:1868:            "`Δ = median(destination_init total_ms) − median(source_init total_ms)`. "
scripts/otp12pf_rigw_analyze.py:1869:            "Each paired `d_i = destination_init total_ms_i − source_init total_ms_i`. "
scripts/otp12pf_rigw_analyze.py:1870:            "`N_pair_split = max(|median(d_1..d_4) − median(d_5..d_8)|, "
scripts/otp12pf_rigw_analyze.py:1872:            "The independent role-order drift is "
scripts/otp12pf_rigw_analyze.py:1873:            "`|median(d_source-first) − median(d_destination-first)|`; the S,D,D,S "
scripts/otp12pf_rigw_analyze.py:1874:            "schedule means this is not the odd/even partition. The conservative "
scripts/otp12pf_rigw_analyze.py:1875:            "operative `N_pair = max(N_pair_split, role-order drift, max(d) − min(d))`, "
scripts/otp12pf_rigw_analyze.py:1879:            f"Δ_on={decimal_text(target['on'].delta)} ms, observer_bias="
scripts/otp12pf_rigw_analyze.py:1880:            f"|Δ_on−Δ_off|={decimal_text(observer_bias)} ms, N_pair_off="
scripts/otp12pf_rigw_analyze.py:1881:            f"{decimal_text(target['off'].n_pair)} ms, N_pair_on="
scripts/otp12pf_rigw_analyze.py:1882:            f"{decimal_text(target['on'].n_pair)} ms, and N_resolution="
scripts/otp12pf_rigw_analyze.py:1883:            f"{decimal_text(n_resolution)} ms.",
scripts/otp12pf_rigw_analyze.py:1885:            "This run measures the observer and paired resolution floors; it does not "
scripts/otp12pf_rigw_analyze.py:1892:            "| cell | trace | metric | sorted ms | largest gap ms | descriptive modes |",
scripts/otp12pf_rigw_analyze.py:1898:            ("source_init total_ms", item.source_values),
scripts/otp12pf_rigw_analyze.py:1899:            ("destination_init total_ms", item.destination_values),
scripts/otp12pf_rigw_analyze.py:1900:            ("paired total_ms d", item.paired_deltas),
scripts/otp12pf_rigw_analyze.py:1905:                f"| {item.cell} | {item.trace_state} | {metric} | {ordered} | "
scripts/otp12pf_rigw_analyze.py:1911:            "## Phase evidence",
scripts/otp12pf_rigw_analyze.py:1913:            f"`phase_events.csv` contains {trace_event_count} structured events. "
scripts/otp12pf_rigw_analyze.py:1916:            "Each phase-event row carries the arm's validated `transfer_ms`, `settled_ms`, "
scripts/otp12pf_rigw_analyze.py:1917:            "`flush_ms`, and authoritative `total_ms`.",
scripts/otp12pf_rigw_analyze.py:1922:            "## Clock-offset evidence",
scripts/otp12pf_rigw_analyze.py:1925:            f"of {clock_arm_count} scheduled arms and reports its midpoint offset. These "
scripts/otp12pf_rigw_analyze.py:1934:def analyze(root: Path | str) -> AnalysisResult:
scripts/otp12pf_rigw_analyze.py:1940:    events = load_trace_events(output_dir)
scripts/otp12pf_rigw_analyze.py:1941:    grouped = validate_traces(rows, events)
scripts/otp12pf_rigw_analyze.py:1943:        condition_stats(rows, cell, trace_state)
scripts/otp12pf_rigw_analyze.py:1945:        for trace_state in ("off", "on")
scripts/otp12pf_rigw_analyze.py:1947:    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
scripts/otp12pf_rigw_analyze.py:1948:    observer_bias = abs(target["on"].delta - target["off"].delta)
scripts/otp12pf_rigw_analyze.py:1949:    n_resolution = max(target["off"].n_pair, target["on"].n_pair, observer_bias)
scripts/otp12pf_rigw_analyze.py:1961:        _summary_rows(stats, observer_bias, n_resolution),
scripts/otp12pf_rigw_analyze.py:1967:            "trace_state",
scripts/otp12pf_rigw_analyze.py:1986:            observer_bias,
scripts/otp12pf_rigw_analyze.py:1987:            n_resolution,
scripts/otp12pf_rigw_analyze.py:2000:        observer_bias=observer_bias,
scripts/otp12pf_rigw_analyze.py:2001:        n_resolution=n_resolution,
scripts/otp12pf_rigw_analyze.py:2002:        trace_event_count=len(events),
scripts/otp12pf_rigw_analyze.py:2006:def main(argv: Sequence[str] | None = None) -> int:

exec
/bin/zsh -lc 'git rev-parse HEAD
git diff --check 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524
git diff --unified=80 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524 -- crates/blit-core/src/transfer_session/data_plane.rs crates/blit-core/tests/transfer_session_roles.rs' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nwiCyhsg' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Zh4DaKgm' (errno=Operation not permitted)
6f517ea1bdbea2f7d83f15c086d2bf5f764cf524
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HwkK3Pcw' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-cHgeDMSR' (errno=Operation not permitted)
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
git: error: couldn't create cache file '/tmp/xcrun_db-YZvoVNIL' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-dmp2M2jg' (errno=Operation not permitted)
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

exec
/bin/zsh -lc "awk '/"'^[A-Za-z_][A-Za-z0-9_]*'"\\(\\)[[:space:]]*\\{/{print NR \":\" "'$0}'"' scripts/bench_otp12pf_rigw.sh
awk '/"'^(def|class) /{print NR ":" $0}'"' scripts/otp12pf_rigw_analyze.py
awk '/"'^(def|class) /{print NR ":" $0}'"' scripts/otp12pf_rigw_analyze_test.py" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
77:log() {
86:die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
87:append_void_line() {
90:session_void() {
98:reserve_evidence_dir() {
127:claim_output_dir() {
135:wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
150:teardown_die() {
160:reject_registered_overrides() {
170:validate_mode_selection() {
184:emit_schedule() {
193:q_source_path() { printf '%s/src_%s' "$Q_MODULE" "$1"; }
194:win_source_path() { printf '%s/src_%s' "$WIN_MODULE" "$1"; }
195:destination_relative_path() {
202:q_destination_path() {
205:win_destination_path() {
208:arm_destination_path() {
216:arm_destination_argument() {
227:append_clock_row() {
230:q_monotonic_ns() {
233:settle_until_deadline() {
245:stamp_result_arrival_on_q() {
267:successful_windows_log_phase_ok() {
270:fetch_successful_windows_client_log() {
276:embeds_clean_q() {
283:selftest() {
1230:win_daemon_stop() {
1234:q_daemon_stop() {
1269:sha256_q() { shasum -a 256 "$1" | awk '{print $1}'; }
1270:sha256_win() {
1275:float_le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }
1277:q_load1() {
1281:q_spotlight_cpu() {
1287:q_time_machine_gate() {
1298:q_quiet_gate() {
1318:win_quiet_gate() {
1339:q_topology_gate() {
1368:win_topology_gate() {
1385:q_to_win_mss() {
1394:win_to_q_mss() {
1407:mss_gate() {
1420:firewall_gate() {
1438:ports_closed() {
1446:timer_gate() {
1462:windows_result_stream_gate() {
1482:fixture_shape_q() {
1493:fixture_shape_win() {
1500:write_q_tree_manifest() {
1545:write_win_tree_manifest() {
1574:matching_manifest_digest() {
1580:verify_fixtures() {
1614:write_manifest() {
1629:provenance_gate() {
1655:preflight() {
1675:q_daemon_stop() {
1694:win_daemon_stop() {
1789:fetch_win_file() {
1807:collect_block_logs() {
1815:stop_daemons() {
1823:q_daemon_start() {
1851:win_daemon_start() {
1914:start_daemons() {
1926:record_clock_samples() {
1940:drain_both() {
1961:prepare_destination() {
1986:flush_verify_q() {
1999:flush_verify_win() {
2009:q_client_run() {
2029:win_client_run() {
2045:session_id_from_log() {
2058:run_arm() {
2199:cell_order() {
2208:run_block() {
2235:end_gate() {
2244:strict_success_cleanup() {
2282:launcher_smoke() {
2298:finalize_registered_session() {
2317:record_failure_evidence() {
2327:on_signal() {
2334:install_signal_traps() {
2340:registered_completion_marker_valid() {
2352:on_exit() {
2411:main() {
83:class BlockSpec:
99:class AnalysisError(RuntimeError):
104:class RunRow:
129:class TraceEvent:
160:class ClockSample:
173:class ModeDescription:
187:class ConditionStats:
212:class AnalysisResult:
224:def decimal_text(value: Decimal) -> str:
230:def parse_decimal(value: str, field: str, line: int) -> Decimal:
242:def parse_int(value: str, field: str, line: int, source: str = "runs.csv") -> int:
251:def expected_roles(pair: int) -> tuple[str, str]:
257:def expected_schedule() -> list[tuple[BlockSpec, str, int, str, int]]:
270:def _safe_client_log(root: Path, value: str, line: int) -> None:
284:def _read_tree_manifest(path: Path, label: str) -> tuple[bytes, str]:
321:def _load_fixture_manifests(root: Path) -> dict[str, tuple[bytes, str]]:
371:def load_runs(root: Path) -> list[RunRow]:
550:def load_clock_samples(root: Path, rows: Sequence[RunRow]) -> list[ClockSample]:
652:def _require_json_string(raw: dict[str, Any], name: str, where: str) -> None:
657:def _require_json_int(raw: dict[str, Any], name: str, where: str) -> None:
663:def load_trace_events(root: Path) -> list[TraceEvent]:
730:def _one_event(events: Sequence[TraceEvent], role: str, name: str, label: str) -> TraceEvent:
737:def _correlation_keys(
753:def _marker_map(
780:def _assert_same_keys(
794:def _assert_before(label: str, start: TraceEvent, end: TraceEvent) -> None:
806:def _assert_event_fields(
819:def validate_traces(
1311:def condition_stats(rows: Sequence[RunRow], cell: str, trace_state: str) -> ConditionStats:
1386:def largest_gap_modes(values: Iterable[Decimal]) -> ModeDescription:
1398:def _atomic_csv(path: Path, fields: Sequence[str], rows: Iterable[dict[str, Any]]) -> None:
1409:def _atomic_text(path: Path, contents: str) -> None:
1418:def _summary_rows(
1510:def _distribution_rows(stats: Sequence[ConditionStats]) -> list[dict[str, str]]:
1558:def _clock_summary_rows(samples: Sequence[ClockSample]) -> list[dict[str, str]]:
1625:def _event_row(row: RunRow, event: TraceEvent) -> dict[str, str]:
1708:def _interval_base(row: RunRow, endpoint_role: str, initiator_role: str) -> dict[str, str]:
1723:def _make_interval(
1759:def _phase_rows(
1817:def _markdown(
1934:def analyze(root: Path | str) -> AnalysisResult:
2006:def main(argv: Sequence[str] | None = None) -> int:
25:class SyntheticSession:
374:class RigWAnalyzerTests(unittest.TestCase):

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '1230,1654p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '1655,2057p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '1,282p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '2058,2460p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
    64	WIN_PURGE="$WIN_ROOT/purge-standby.ps1"
    65
    66	SESSION_TAG=$(date -u +%Y%m%dT%H%M%SZ).$$
    67	OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12pf-rigw-$SESSION_TAG}
    68	WIN_SESSION="$WIN_ROOT/rigw-pf1/$SESSION_TAG"
    69
    70	LOG="$OUT_DIR/bench.log"
    71	RUNS_CSV="$OUT_DIR/runs.csv"
    72	CLOCK_CSV="$OUT_DIR/clock-samples.csv"
    73
    74	LAST_ERROR=""
    75	OUTPUT_CLAIMED=0
    76	OUTPUT_CLAIM_ERROR=""
    77	log() {
    78	    local line
    79	    line="$(date -u +%H:%M:%SZ) $*"
    80	    if [[ "$OUTPUT_CLAIMED" == 1 ]]; then
    81	        printf '%s\n' "$line" | tee -a "$LOG"
    82	    else
    83	        printf '%s\n' "$line" >&2
    84	    fi
    85	}
    86	die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
    87	append_void_line() {
    88	    printf '%s\n' "$1" >> "$OUT_DIR/SESSION-VOID"
    89	}
    90	session_void() {
    91	    local reason="$1"
    92	    LAST_ERROR="$reason"
    93	    append_void_line "$reason"
    94	    log "SESSION-VOID: $reason"
    95	    exit 1
    96	}
    97
    98	reserve_evidence_dir() {
    99	    local target="$1" parent
   100	    OUTPUT_CLAIM_ERROR=""
   101	    if [[ -e "$target" || -L "$target" ]]; then
   102	        if [[ -f "$target/SESSION-COMPLETE" ]]; then
   103	            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-COMPLETE: $target"
   104	        elif [[ -f "$target/SESSION-VOID" ]]; then
   105	            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-VOID: $target"
   106	        else
   107	            OUTPUT_CLAIM_ERROR="refusing existing output path (must be fresh): $target"
   108	        fi
   109	        return 1
   110	    fi
   111	    parent=$(dirname "$target")
   112	    mkdir -p "$parent" || {
   113	        OUTPUT_CLAIM_ERROR="cannot create output parent: $parent"
   114	        return 1
   115	    }
   116	    mkdir "$target" || {
   117	        OUTPUT_CLAIM_ERROR="cannot atomically claim output directory: $target"
   118	        return 1
   119	    }
   120	    mkdir "$target/trace" "$target/client" "$target/fixtures" "$target/landed" || {
   121	        OUTPUT_CLAIM_ERROR="cannot initialize output directory: $target"
   122	        rm -rf "$target"
   123	        return 1
   124	    }
   125	}
   126
   127	claim_output_dir() {
   128	    reserve_evidence_dir "$OUT_DIR" || return 1
   129	    OUTPUT_CLAIMED=1
   130	}
   131
   132	SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto \
   133	    -o ConnectTimeout=5 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 \
   134	    -o "ControlPath=$HOME/.ssh/cm-rigw-%r@%h-%p" -o ControlPersist=300)
   135	wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
   136
   137	q_daemon_pid=""
   138	win_daemon_pid=""
   139	win_cmd_pid=""
   140	current_block=""
   141	CLEANUP_MODE=0
   142	CLEANUP_ERROR=""
   143	REGISTERED_RUN_STARTED=0
   144	SESSION_FINALIZED=0
   145	STRICT_CLEANUP_VERIFIED=0
   146	Q_SESSION_MAY_EXIST=0
   147	WIN_SESSION_MAY_EXIST=0
   148	LOCAL_EVIDENCE_COMPLETE=0
   149
   150	teardown_die() {
   151	    local reason="$1"
   152	    if [[ "$CLEANUP_MODE" == 1 ]]; then
   153	        CLEANUP_ERROR="${CLEANUP_ERROR:+$CLEANUP_ERROR; }$reason"
   154	        log "CLEANUP-ERROR: $reason"
   155	        return 1
   156	    fi
   157	    session_void "$reason"
   158	}
   159
   160	reject_registered_overrides() {
   161	    local name
   162	    for name in RUNS CELLS MAC_HOST WIN_HOST WIN_SSH_OVERRIDE PORT_OVERRIDE \
   163	        Q_NIC_OVERRIDE Q_IP_OVERRIDE TRACE_ORDER PAIRS_PER_BLOCK_OVERRIDE; do
   164	        if [[ -n "${!name+x}" ]]; then
   165	            die "$name is not configurable for the registered rig-W diagnostic"
   166	        fi
   167	    done
   168	}
   169
   170	validate_mode_selection() {
   171	    local name value enabled=0
   172	    for name in SELFTEST PREFLIGHT_ONLY LAUNCHER_SMOKE; do
   173	        value=${!name}
   174	        [[ "$value" == 0 || "$value" == 1 ]] \
   175	            || die "$name must be exactly 0 or 1"
   176	        if [[ "$value" == 1 ]]; then
   177	            enabled=$((enabled + 1))
   178	        fi
   179	    done
   180	    [[ "$enabled" -le 1 ]] \
   181	        || die "SELFTEST, PREFLIGHT_ONLY, and LAUNCHER_SMOKE are mutually exclusive"
   182	}
   183
   184	emit_schedule() {
   185	    cat <<'EOF'
   186	1,off,forward,1,4
   187	2,on,reverse,1,4
   188	3,on,forward,5,8
   189	4,off,reverse,5,8
   190	EOF
   191	}
   192
   193	q_source_path() { printf '%s/src_%s' "$Q_MODULE" "$1"; }
   194	win_source_path() { printf '%s/src_%s' "$WIN_MODULE" "$1"; }
   195	destination_relative_path() {
   196	    # Accept the role so callers cannot accidentally omit the parity axis, but
   197	    # deliberately keep it out of the measured path.  Every arm in this
   198	    # registered session reuses this one endpoint-local destination.
   199	    case "$1" in source_init|destination_init);; *) return 2;; esac
   200	    printf 'rigw-sessions/%s/destination/container' "$SESSION_TAG"
   201	}
   202	q_destination_path() {
   203	    printf '%s/%s' "$Q_MODULE" "$(destination_relative_path "$1")"
   204	}
   205	win_destination_path() {
   206	    printf '%s/%s' "$WIN_MODULE" "$(destination_relative_path "$1")"
   207	}
   208	arm_destination_path() {
   209	    local direction="$1" role="$2"
   210	    case "$direction" in
   211	        wm) q_destination_path "$role";;
   212	        mw) win_destination_path "$role";;
   213	        *) return 2;;
   214	    esac
   215	}
   216	arm_destination_argument() {
   217	    local direction="$1" role="$2" relative
   218	    relative=$(destination_relative_path "$role") || return 2
   219	    case "$direction/$role" in
   220	        wm/source_init) printf '%s:%s:/bench/%s/' "$Q_IP" "$PORT" "$relative";;
   221	        wm/destination_init) q_destination_path "$role";;
   222	        mw/source_init) printf '%s:%s:/bench/%s/' "$WIN_IP" "$PORT" "$relative";;
   223	        mw/destination_init) win_destination_path "$role";;
   224	        *) return 2;;
   225	    esac
   226	}
   227	append_clock_row() {
   228	    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' "$@"
   229	}
   230	q_monotonic_ns() {
   231	    python3 -c 'import time; print(time.clock_gettime_ns(time.CLOCK_MONOTONIC))'
   232	}
   233	settle_until_deadline() {
   234	    python3 - "$1" <<'PY'
   235	import sys, time
   236
   237	deadline_ns = int(sys.argv[1])
   238	clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
   239	remaining_ns = deadline_ns - clock_ns()
   240	if remaining_ns > 0:
   241	    time.sleep(remaining_ns / 1_000_000_000)
   242	print(clock_ns())
   243	PY
   244	}
   245	stamp_result_arrival_on_q() {
   246	    python3 -c '
   247	import sys, time
   248
   249	result = None
   250	stamp_ns = None
   251	for raw in sys.stdin:
   252	    line = raw.rstrip("\r\n")
   253	    if not line.startswith("R|"):
   254	        continue
   255	    if result is not None:
   256	        raise SystemExit("multiple Windows client result sentinels")
   257	    fields = line.split("|")
   258	    if len(fields) != 3:
   259	        raise SystemExit("malformed Windows client result sentinel")
   260	    result = line
   261	    stamp_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
   262	if result is None or stamp_ns is None:
   263	    raise SystemExit("missing Windows client result sentinel")
   264	print(f"{result}|{stamp_ns}")
   265	'
   266	}
   267	successful_windows_log_phase_ok() {
   268	    [[ "$1" == durability_verified ]]
   269	}
   270	fetch_successful_windows_client_log() {
   271	    local arm_phase="$1" remote_err="$2" local_err="$3"
   272	    successful_windows_log_phase_ok "$arm_phase" \
   273	        || session_void "refusing successful Windows client-log fetch before destination durability"
   274	    fetch_win_file "$remote_err" "$local_err"
   275	}
   276	embeds_clean_q() {
   277	    local path="$1"
   278	    LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID" "$path" || return 1
   279	    LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID.dirty" "$path" && return 1
   280	    return 0
   281	}
   282

 succeeded in 0ms:
  1230	win_daemon_stop() {
  1231	    printf "windows\n" >> "$OUT_DIR/stops"
  1232	    win_daemon_pid=""; win_cmd_pid=""; current_block=""
  1233	}
  1234	q_daemon_stop() {
  1235	    printf "q\n" >> "$OUT_DIR/stops"
  1236	    q_daemon_pid=""
  1237	}
  1238	printf "must disappear\n" > "$OUT_DIR/SESSION-COMPLETE"
  1239	trap on_exit EXIT
  1240	install_signal_traps
  1241	kill -s "$3" "$$"
  1242	sleep 2
  1243	exit 99
  1244	' _ "$SCRIPT_DIR/bench_otp12pf_rigw.sh" "$signal_dir" "$signal"
  1245	        signal_rc=$?
  1246	        set -e
  1247	        [[ "$signal_rc" == 1 ]] \
  1248	            || die "$signal cleanup returned $signal_rc, expected 1"
  1249	        grep -Fxq "received $signal" "$signal_dir/SESSION-VOID" \
  1250	            || die "$signal cleanup omitted its signal reason"
  1251	        [[ "$(LC_ALL=C sort "$signal_dir/stops")" == $'q\nwindows' ]] \
  1252	            || die "$signal cleanup did not invoke both exact-owned teardown paths"
  1253	        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
  1254	            || die "$signal cleanup left SESSION-COMPLETE"
  1255	    done
  1256	    rm -rf "$failure_tmp"
  1257
  1258	    analyzer_log=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-analyzer.XXXXXX")
  1259	    if ! python3 "$SCRIPT_DIR/otp12pf_rigw_analyze_test.py" \
  1260	        > "$analyzer_log" 2>&1; then
  1261	        cat "$analyzer_log" >&2
  1262	        rm -f "$analyzer_log"
  1263	        die "analyzer self-tests failed"
  1264	    fi
  1265	    rm -f "$analyzer_log"
  1266	    log "SELFTEST OK: exact four-block/128-arm schedule and analyzer guards"
  1267	}
  1268
  1269	sha256_q() { shasum -a 256 "$1" | awk '{print $1}'; }
  1270	sha256_win() {
  1271	    wssh "(Get-FileHash -Algorithm SHA256 -LiteralPath '$1').Hash.ToLower()" \
  1272	        | tr -d '\r' | tail -1
  1273	}
  1274
  1275	float_le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }
  1276
  1277	q_load1() {
  1278	    /usr/sbin/sysctl -n vm.loadavg | awk '{gsub(/[{}]/, ""); print $1}'
  1279	}
  1280
  1281	q_spotlight_cpu() {
  1282	    ps -axo %cpu=,comm= | awk '
  1283	        $2 ~ /(mds|mds_stores|mdworker|mdbulkimport)$/ { sum += $1 }
  1284	        END { printf "%.1f\n", sum + 0 }'
  1285	}
  1286
  1287	q_time_machine_gate() {
  1288	    local auto status
  1289	    auto=$(/usr/bin/defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null) \
  1290	        || die "q Time Machine AutoBackup setting is unreadable"
  1291	    [[ "$auto" == 0 ]] \
  1292	        || die "q Time Machine AutoBackup is enabled ($auto); do not mutate it from the harness"
  1293	    status=$(/usr/bin/tmutil status) || die "q Time Machine status is unreadable"
  1294	    grep -q 'Running = 0;' <<<"$status" \
  1295	        || die "q Time Machine is running"
  1296	}
  1297
  1298	q_quiet_gate() {
  1299	    local offenders load spot
  1300	    offenders=$(ps -axo pid=,comm= | awk -v owned="${q_daemon_pid:-}" '
  1301	        {
  1302	          n=$2; sub(/^.*\//, "", n)
  1303	          if ($1 != owned && (n == "cargo" || n == "rustc" || n == "blit-daemon" || n ~ /^codex($|-)/))
  1304	            print $1 ":" n
  1305	        }')
  1306	    [[ -z "$offenders" ]] || die "q has benchmark-conflicting processes: $offenders"
  1307	    q_time_machine_gate
  1308	    load=$(q_load1)
  1309	    [[ "$load" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse q load1 '$load'"
  1310	    float_le "$load" "$LOAD1_MAX" || die "q load1 $load exceeds $LOAD1_MAX"
  1311	    spot=$(q_spotlight_cpu)
  1312	    [[ "$spot" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Spotlight CPU '$spot'"
  1313	    float_le "$spot" "$SPOTLIGHT_CPU_MAX" \
  1314	        || die "q Spotlight CPU $spot% exceeds $SPOTLIGHT_CPU_MAX%"
  1315	    log "quiet q: load1=$load Spotlight=${spot}% TimeMachine=disabled/stopped"
  1316	}
  1317
  1318	win_quiet_gate() {
  1319	    local out avg
  1320	    out=$(wssh '
  1321	$ErrorActionPreference = "Stop"
  1322	$bad = Get-Process cargo,rustc,blit-daemon -ErrorAction SilentlyContinue
  1323	if ($bad) { "BAD|" + (($bad | ForEach-Object { "$($_.Id):$($_.ProcessName)" }) -join ","); exit 7 }
  1324	$samples = 1..3 | ForEach-Object {
  1325	  $v = (Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average
  1326	  Start-Sleep -Seconds 1
  1327	  [double]$v
  1328	}
  1329	"CPU|$([math]::Round(($samples | Measure-Object -Average).Average,1))"
  1330	') || die "Windows quiet probe failed: $out"
  1331	    out=${out//$'\r'/}
  1332	    [[ "$out" != *BAD\|* ]] || die "Windows has benchmark-conflicting processes: $out"
  1333	    avg=$(sed -n 's/^CPU|//p' <<<"$out" | tail -1)
  1334	    [[ "$avg" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Windows CPU from '$out'"
  1335	    float_le "$avg" "$WIN_CPU_MAX" || die "Windows CPU ${avg}% exceeds ${WIN_CPU_MAX}%"
  1336	    log "quiet Windows: CPU=${avg}% and no cargo/rustc/blit-daemon"
  1337	}
  1338
  1339	q_topology_gate() {
  1340	    local raw route arp mtu media status iface route_mtu peer_mac
  1341	    [[ "$(hostname)" == "$Q_EXPECT_HOST" ]] \
  1342	        || die "this harness must execute on $Q_EXPECT_HOST, got $(hostname)"
  1343	    raw=$(/sbin/ifconfig "$Q_NIC") || die "cannot read q $Q_NIC"
  1344	    mtu=$(sed -n 's/.*[[:space:]]mtu[[:space:]]\([0-9][0-9]*\).*/\1/p' <<<"$raw" | head -1)
  1345	    media=$(sed -n 's/^[[:space:]]*media:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
  1346	    status=$(sed -n 's/^[[:space:]]*status:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
  1347	    grep -q "inet $Q_IP " <<<"$raw" || die "$Q_NIC does not own $Q_IP"
  1348	    grep -qi "ether $Q_MAC" <<<"$raw" || die "$Q_NIC MAC is not $Q_MAC"
  1349	    [[ "$mtu" == "$REGISTERED_MTU" ]] || die "$Q_NIC MTU is $mtu, expected $REGISTERED_MTU"
  1350	    [[ "$media" == *"$REGISTERED_MEDIA"* ]] || die "$Q_NIC media is '$media', expected $REGISTERED_MEDIA"
  1351	    [[ "$status" == active ]] || die "$Q_NIC status is '$status'"
  1352
  1353	    route=$(/sbin/route -n get "$WIN_IP") || die "q route probe failed"
  1354	    iface=$(awk '/interface:/ {print $2; exit}' <<<"$route")
  1355	    route_mtu=$(awk '/mtu/ {getline; print $(NF-1); exit}' <<<"$route")
  1356	    [[ "$iface" == "$Q_NIC" ]] || die "q routes $WIN_IP via $iface, expected $Q_NIC"
  1357	    [[ "$route_mtu" == "$REGISTERED_MTU" ]] \
  1358	        || die "q route to $WIN_IP reports MTU $route_mtu, expected $REGISTERED_MTU"
  1359	    /sbin/ping -c 1 -W 1000 "$WIN_IP" >/dev/null || die "q cannot ping $WIN_IP"
  1360	    arp=$(/usr/sbin/arp -n "$WIN_IP") || die "q ARP probe failed"
  1361	    peer_mac=$(sed -n 's/.* at \([^ ]*\) on .*/\1/p' <<<"$arp" | tr 'A-F' 'a-f')
  1362	    [[ "$peer_mac" == "$(tr 'A-F' 'a-f' <<<"${WIN_MAC//-/:}")" ]] \
  1363	        || die "q ARP for $WIN_IP is $peer_mac, expected peer ${WIN_MAC//-/:}"
  1364	    [[ "$peer_mac" != "$Q_MAC" ]] || die "q ARP points at q's own MAC (black-hole host route)"
  1365	    log "fabric q: $Q_NIC $Q_IP mtu=$mtu media=$media route=$iface peer=$peer_mac"
  1366	}
  1367
  1368	win_topology_gate() {
  1369	    local out
  1370	    out=$(wssh "
  1371	\$ErrorActionPreference = 'Stop'
  1372	\$a = Get-NetAdapter -Name '$WIN_NIC'
  1373	\$ip = Get-NetIPAddress -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4 | Where-Object IPAddress -eq '$WIN_IP'
  1374	\$ni = Get-NetIPInterface -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4
  1375	\$route = Find-NetRoute -RemoteIPAddress '$Q_IP' | Select-Object -First 1
  1376	if (-not \$ip) { throw 'registered IPv4 address absent' }
  1377	\"W|\$(\$a.Status)|\$(\$a.LinkSpeed)|\$(\$a.ReceiveLinkSpeed)|\$(\$a.TransmitLinkSpeed)|\$(\$a.MacAddress)|\$(\$ni.ConnectionState)|\$(\$ni.NlMtu)|\$(\$route.InterfaceAlias)|\$(\$route.IPAddress)\"
  1378	") || die "Windows topology probe failed: $out"
  1379	    out=${out//$'\r'/}
  1380	    [[ "$out" == "W|Up|10 Gbps|10000000000|10000000000|$WIN_MAC|Connected|$REGISTERED_MTU|$WIN_NIC|$WIN_IP" ]] \
  1381	        || die "Windows topology mismatch: '$out'"
  1382	    log "fabric Windows: $WIN_NIC $WIN_IP mtu=$REGISTERED_MTU link=10Gbps route/source pinned"
  1383	}
  1384
  1385	q_to_win_mss() {
  1386	    python3 - "$WIN_IP" <<'PY'
  1387	import socket, sys
  1388	s = socket.create_connection((sys.argv[1], 22), timeout=5)
  1389	print(f"{s.getsockopt(socket.IPPROTO_TCP, socket.TCP_MAXSEG)} {s.getsockname()[0]}")
  1390	s.close()
  1391	PY
  1392	}
  1393
  1394	win_to_q_mss() {
  1395	    wssh "
  1396	\$ErrorActionPreference = 'Stop'
  1397	\$s = [Net.Sockets.Socket]::new([Net.Sockets.AddressFamily]::InterNetwork,[Net.Sockets.SocketType]::Stream,[Net.Sockets.ProtocolType]::Tcp)
  1398	\$s.Connect('$Q_IP',22)
  1399	\$name = [Net.Sockets.SocketOptionName]4
  1400	\$b = \$s.GetSocketOption([Net.Sockets.SocketOptionLevel]::Tcp,\$name,4)
  1401	\$m = [BitConverter]::ToInt32(\$b,0)
  1402	\"M|\${m}|\$(\$s.LocalEndPoint.Address)\"
  1403	\$s.Dispose()
  1404	" | tr -d '\r' | tail -1
  1405	}
  1406
  1407	mss_gate() {
  1408	    local qout wout qm qip wm wip
  1409	    qout=$(q_to_win_mss) || die "q→Windows MSS probe failed"
  1410	    read -r qm qip <<<"$qout"
  1411	    [[ "$qm" == "$Q_TO_WIN_MSS" && "$qip" == "$Q_IP" ]] \
  1412	        || die "q→Windows MSS/source is '$qout', expected $Q_TO_WIN_MSS $Q_IP"
  1413	    wout=$(win_to_q_mss) || die "Windows→q MSS probe failed"
  1414	    IFS='|' read -r _ wm wip <<<"$wout"
  1415	    [[ "$wm" == "$WIN_TO_Q_MSS" && "$wip" == "$WIN_IP" ]] \
  1416	        || die "Windows→q MSS/source is '$wout', expected M|$WIN_TO_Q_MSS|$WIN_IP"
  1417	    log "path MSS: q→Windows=$qm via $qip; Windows→q=$wm via $wip"
  1418	}
  1419
  1420	firewall_gate() {
  1421	    local out
  1422	    out=$(wssh "
  1423	\$r = Get-NetFirewallRule -DisplayName 'blit-otp12-daemon' -ErrorAction SilentlyContinue
  1424	if (-not \$r) { exit 4 }
  1425	\$app = \$r | Get-NetFirewallApplicationFilter
  1426	\"F|\$(\$r.Enabled)|\$(\$r.Action)|\$(\$r.Direction)|\$(\$app.Program)\"
  1427	") || die "existing Windows firewall rule is absent/unreadable; harness will not create it"
  1428	    out=${out//$'\r'/}
  1429	    out=$(sed 's#\\#/#g' <<<"$out")
  1430	    out=$(tr 'A-Z' 'a-z' <<<"$out")
  1431	    local expected
  1432	    expected=$(tr 'A-Z' 'a-z' <<<"F|True|Allow|Inbound|$WIN_ACTIVE")
  1433	    [[ "$out" == "$expected" ]] \
  1434	        || die "Windows firewall rule mismatch: '$out'"
  1435	    log "firewall verified only: existing inbound allow is scoped to $WIN_ACTIVE"
  1436	}
  1437
  1438	ports_closed() {
  1439	    if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  1440	        return 1
  1441	    fi
  1442	    wssh "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
  1443	        >/dev/null 2>&1
  1444	}
  1445
  1446	timer_gate() {
  1447	    local qms wout wms
  1448	    qms=$(python3 - <<'PY'
  1449	import time
  1450	clock_ns=lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
  1451	t=clock_ns(); time.sleep(1); print(round((clock_ns()-t)/1_000_000))
  1452	PY
  1453	)
  1454	    [[ "$qms" -ge 950 && "$qms" -le 1050 ]] || die "q one-second timer calibrated to ${qms}ms"
  1455	    wout=$(wssh '$s=[Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Seconds 1; $s.Stop(); "T|$([int]$s.Elapsed.TotalMilliseconds)"') \
  1456	        || die "Windows timer calibration failed"
  1457	    wout=${wout//$'\r'/}; wms=${wout##*|}
  1458	    [[ "$wms" -ge 950 && "$wms" -le 1050 ]] || die "Windows one-second timer calibrated to ${wms}ms"
  1459	    log "timer calibration: q=${qms}ms Windows=${wms}ms"
  1460	}
  1461
  1462	windows_result_stream_gate() {
  1463	    local before after result tag ms rc stamp extra teardown_ns
  1464	    before=$(q_monotonic_ns)
  1465	    result=$(wssh \
  1466	        '[Console]::Out.WriteLine("R|17|0"); [Console]::Out.Flush(); Start-Sleep -Milliseconds 350' \
  1467	        | stamp_result_arrival_on_q) \
  1468	        || die "Windows result-stream probe failed"
  1469	    after=$(q_monotonic_ns)
  1470	    IFS='|' read -r tag ms rc stamp extra <<<"$result"
  1471	    [[ "$tag" == R && "$ms" == 17 && "$rc" == 0 \
  1472	        && "$stamp" =~ ^[0-9]+$ && -z "$extra" ]] \
  1473	        || die "Windows result-stream probe returned '$result'"
  1474	    [[ "$stamp" -ge "$before" && "$stamp" -le "$after" ]] \
  1475	        || die "Windows result-stream q stamp is outside the probe lifetime"
  1476	    teardown_ns=$((after - stamp))
  1477	    [[ "$teardown_ns" -ge 250000000 ]] \
  1478	        || die "Windows result stream was not observable before remote teardown"
  1479	    log "Windows result stream reaches q before remote teardown"
  1480	}
  1481
  1482	fixture_shape_q() {
  1483	    python3 - "$1" <<'PY'
  1484	import os, sys
  1485	n=b=0
  1486	for root, dirs, files in os.walk(sys.argv[1]):
  1487	    for name in files:
  1488	        p=os.path.join(root,name); n+=1; b+=os.path.getsize(p)
  1489	print(f"{n},{b}")
  1490	PY
  1491	}
  1492
  1493	fixture_shape_win() {
  1494	    wssh "
  1495	\$f = Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
  1496	\"S|\$(\$f.Count)|\$(if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 })\"
  1497	" | tr -d '\r' | sed -n 's/^S|//p' | tr '|' ',' | tail -1
  1498	}
  1499
  1500	write_q_tree_manifest() {
  1501	    python3 - "$1" "$2" "${3:-}" <<'PY'
  1502	import base64, os, pathlib, stat, sys
  1503
  1504	root = pathlib.Path(sys.argv[1])
  1505	output = pathlib.Path(sys.argv[2])
  1506	expected_root = sys.argv[3]
  1507	if not root.is_dir() or root.is_symlink():
  1508	    raise SystemExit(f"manifest root is not a plain directory: {root}")
  1509	if expected_root:
  1510	    entries = list(root.iterdir())
  1511	    if (
  1512	        len(entries) != 1
  1513	        or entries[0].name != expected_root
  1514	        or not entries[0].is_dir()
  1515	        or entries[0].is_symlink()
  1516	    ):
  1517	        raise SystemExit(
  1518	            f"landed container must contain exactly plain directory {expected_root}"
  1519	        )
  1520	    root = entries[0]
  1521
  1522	lines = []
  1523	def walk_error(error):
  1524	    raise error
  1525
  1526	for current, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
  1527	    for name in dirs:
  1528	        path = pathlib.Path(current, name)
  1529	        mode = path.lstat().st_mode
  1530	        if not stat.S_ISDIR(mode) or stat.S_ISLNK(mode):
  1531	            raise SystemExit(f"non-directory/reparse entry in manifest: {path}")
  1532	    for name in files:
  1533	        path = pathlib.Path(current, name)
  1534	        info = path.lstat()
  1535	        if not stat.S_ISREG(info.st_mode):
  1536	            raise SystemExit(f"non-regular entry in manifest: {path}")
  1537	        relative = path.relative_to(root).as_posix()
  1538	        encoded = base64.b64encode(relative.encode("utf-8")).decode("ascii")
  1539	        lines.append(f"{encoded},{info.st_size}")
  1540	lines.sort()
  1541	output.write_text("".join(f"{line}\n" for line in lines), encoding="ascii")
  1542	PY
  1543	}
  1544
  1545	write_win_tree_manifest() {
  1546	    local root="$1" remote_out="$2" local_out="$3" expected_root="${4:-}"
  1547	    wssh "
  1548	\$ErrorActionPreference = 'Stop'
  1549	\$root = (Resolve-Path -LiteralPath '$root').Path.TrimEnd([char]92,[char]47)
  1550	if ('$expected_root') {
  1551	  \$entries = @(Get-ChildItem -LiteralPath \$root -Force -ErrorAction Stop)
  1552	  if (\$entries.Count -ne 1 -or -not \$entries[0].PSIsContainer -or \$entries[0].Name -cne '$expected_root' -or ((\$entries[0].Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'landed root layout mismatch' }
  1553	  \$root = \$entries[0].FullName.TrimEnd([char]92,[char]47)
  1554	}
  1555	\$lines = [Collections.Generic.List[string]]::new()
  1556	foreach (\$item in @(Get-ChildItem -LiteralPath \$root -Recurse -Force -ErrorAction Stop)) {
  1557	  if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw \"reparse entry in manifest: \$(\$item.FullName)\" }
  1558	  if (\$item.PSIsContainer) { continue }
  1559	  if (-not (\$item -is [IO.FileInfo])) { throw \"non-regular entry in manifest: \$(\$item.FullName)\" }
  1560	  \$rel = \$item.FullName.Substring(\$root.Length).TrimStart([char]92,[char]47).Replace([char]92,[char]47)
  1561	  \$b64 = [Convert]::ToBase64String([Text.UTF8Encoding]::new(\$false,\$true).GetBytes(\$rel))
  1562	  \$lines.Add(\"\$b64,\$([uint64]\$item.Length)\")
  1563	}
  1564	\$ordered = [string[]]\$lines.ToArray()
  1565	[Array]::Sort(\$ordered, [StringComparer]::Ordinal)
  1566	\$text = if (\$ordered.Count) { (\$ordered -join \"`n\") + \"`n\" } else { '' }
  1567	[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName('$remote_out')) | Out-Null
  1568	[IO.File]::WriteAllText('$remote_out', \$text, [Text.UTF8Encoding]::new(\$false))
  1569	" || return 1
  1570	    fetch_win_file "$remote_out" "$local_out" || return 1
  1571	    LC_ALL=C sort -o "$local_out" "$local_out"
  1572	}
  1573
  1574	matching_manifest_digest() {
  1575	    local canonical="$1" landed="$2"
  1576	    cmp -s "$canonical" "$landed" || return 1
  1577	    sha256_q "$landed"
  1578	}
  1579
  1580	verify_fixtures() {
  1581	    local shape want qgot wgot qmanifest wmanifest qhash
  1582	    printf '%s\n' 'shape,sha256,q_manifest,windows_manifest' \
  1583	        > "$OUT_DIR/fixture-manifests.csv"
  1584	    WIN_SESSION_MAY_EXIST=1
  1585	    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION/fixtures' | Out-Null" \
  1586	        || die "cannot create Windows fixture evidence directory"
  1587	    for shape in mixed large; do
  1588	        case "$shape" in
  1589	            mixed) want=5001,547110912;;
  1590	            large) want=1,1073741824;;
  1591	        esac
  1592	        qgot=$(fixture_shape_q "$(q_source_path "$shape")")
  1593	        wgot=$(fixture_shape_win "$(win_source_path "$shape")")
  1594	        [[ "$qgot" == "$want" ]] || die "q src_$shape is $qgot, expected $want"
  1595	        [[ "$wgot" == "$want" ]] || die "Windows canonical src_$shape is $wgot, expected $want"
  1596	        qmanifest="$OUT_DIR/fixtures/src_$shape.manifest"
  1597	        wmanifest="$OUT_DIR/fixtures/windows-src_$shape.manifest"
  1598	        write_q_tree_manifest "$(q_source_path "$shape")" "$qmanifest" \
  1599	            || die "q src_$shape manifest failed"
  1600	        write_win_tree_manifest \
  1601	            "$(win_source_path "$shape")" \
  1602	            "$WIN_SESSION/fixtures/src_$shape.manifest" "$wmanifest" \
  1603	            || die "Windows src_$shape manifest failed"
  1604	        qhash=$(matching_manifest_digest "$qmanifest" "$wmanifest") \
  1605	            || die "q and Windows src_$shape relative-path/size manifests differ"
  1606	        printf '%s,%s,%s,%s\n' \
  1607	            "$shape" "$qhash" "fixtures/src_$shape.manifest" \
  1608	            "fixtures/windows-src_$shape.manifest" \
  1609	            >> "$OUT_DIR/fixture-manifests.csv"
  1610	    done
  1611	    log "canonical fixtures verified byte-for-byte by relative path and size on both hosts"
  1612	}
  1613
  1614	write_manifest() {
  1615	    local qbh qdh wbh wdh
  1616	    qbh=$(sha256_q "$Q_BLIT"); qdh=$(sha256_q "$Q_DAEMON")
  1617	    wbh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit.exe")
  1618	    wdh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit-daemon.exe")
  1619	    cat > "$OUT_DIR/staging-manifest.csv" <<EOF
  1620	host,role,commit,sha256,path
  1621	q,client,$HEAD_FULL,$qbh,$Q_BLIT
  1622	q,daemon,$HEAD_FULL,$qdh,$Q_DAEMON
  1623	windows,client,$HEAD_FULL,$wbh,$WIN_BINS/$HEAD_SHORT/blit.exe
  1624	windows,daemon,$HEAD_FULL,$wdh,$WIN_BINS/$HEAD_SHORT/blit-daemon.exe
  1625	EOF
  1626	    WIN_DAEMON_HASH=$wdh
  1627	}
  1628
  1629	provenance_gate() {
  1630	    [[ -n "$EXPECT_SHA" ]] || die "EXPECT_SHA=<full reviewed commit> is required"
  1631	    HEAD_FULL=$(git -C "$REPO_ROOT" rev-parse HEAD)
  1632	    HEAD_SHORT=$(git -C "$REPO_ROOT" rev-parse --short=7 HEAD)
  1633	    HEAD_BUILD_ID=$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD)
  1634	    [[ "$EXPECT_SHA" == "$HEAD_FULL" ]] \
  1635	        || die "EXPECT_SHA=$EXPECT_SHA but isolated clone is $HEAD_FULL"
  1636	    [[ -z $(git -C "$REPO_ROOT" status --porcelain --untracked-files=normal) ]] \
  1637	        || die "isolated q clone is dirty"
  1638	    [[ -x "$Q_BLIT" && -x "$Q_DAEMON" ]] || die "q release binaries are absent"
  1639	    embeds_clean_q "$Q_BLIT" \
  1640	        || die "q client does not embed a clean +$HEAD_BUILD_ID"
  1641	    embeds_clean_q "$Q_DAEMON" \
  1642	        || die "q daemon does not embed a clean +$HEAD_BUILD_ID"
  1643	    wssh "
  1644	if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe')) { exit 2 }
  1645	if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe')) { exit 3 }
  1646	if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 4 }
  1647	if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 5 }
  1648	if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 6 }
  1649	if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 7 }
  1650	" || die "Windows binaries are missing or do not embed a clean +$HEAD_BUILD_ID"
  1651	    write_manifest
  1652	    log "provenance exact: $HEAD_FULL on q and Windows"
  1653	}
  1654

 succeeded in 0ms:
  1655	preflight() {
  1656	    reject_registered_overrides
  1657	    command -v python3 >/dev/null || die "python3 required"
  1658	    command -v lsof >/dev/null || die "lsof required"
  1659	    command -v nc >/dev/null || die "nc required"
  1660	    sudo -n /usr/sbin/purge >/dev/null || die "q NOPASSWD purge grant is absent"
  1661	    provenance_gate
  1662	    ports_closed || die "port $PORT already has a listener on q or Windows"
  1663	    q_topology_gate
  1664	    win_topology_gate
  1665	    mss_gate
  1666	    firewall_gate
  1667	    q_quiet_gate
  1668	    win_quiet_gate
  1669	    timer_gate
  1670	    windows_result_stream_gate
  1671	    verify_fixtures
  1672	    log "PREFLIGHT OK: registered rig, exact binaries, canonical paths, quiet endpoints"
  1673	}
  1674
  1675	q_daemon_stop() {
  1676	    local pid="$q_daemon_pid" i
  1677	    [[ -z "$pid" ]] && return 0
  1678	    if kill -0 "$pid" 2>/dev/null; then
  1679	        local cmd
  1680	        cmd=$(ps -p "$pid" -o command= 2>/dev/null || true)
  1681	        [[ "$cmd" == *"$Q_DAEMON"* ]] \
  1682	            || { teardown_die "refusing to stop q PID $pid because it is not the launched daemon: $cmd"; return 1; }
  1683	        kill "$pid" || true
  1684	        for ((i=0; i<40; i++)); do
  1685	            kill -0 "$pid" 2>/dev/null || break
  1686	            sleep 0.25
  1687	        done
  1688	        kill -0 "$pid" 2>/dev/null \
  1689	            && { teardown_die "q daemon PID $pid survived exact teardown"; return 1; }
  1690	    fi
  1691	    q_daemon_pid=""
  1692	}
  1693
  1694	win_daemon_stop() {
  1695	    local pid="$win_daemon_pid" cmdpid="$win_cmd_pid" out pid_probe
  1696	    if [[ -z "$pid" && -z "$cmdpid" && -n "$current_block" ]]; then
  1697	        if ! pid_probe=$(wssh "
  1698	\$ErrorActionPreference = 'Stop'
  1699	\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
  1700	\$d = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/daemon.pid' -ErrorAction SilentlyContinue
  1701	\$c = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/launcher.pid' -ErrorAction SilentlyContinue
  1702	if (-not \$c) {
  1703	  \$launchers = @(Get-CimInstance Win32_Process -Filter \"Name='cmd.exe'\" | Where-Object {
  1704	    \$actual = if (\$_.CommandLine) { \$_.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1705	    \$actual -ieq \$expectedLauncher
  1706	  })
  1707	  if (\$launchers.Count -gt 1) { throw \"multiple exact launchers match \$expectedLauncher\" }
  1708	  if (\$launchers.Count -eq 1) { \$c = [string]\$launchers[0].ProcessId }
  1709	}
  1710	if (-not \$d -and \$c -match '^[0-9]+$') {
  1711	  \$children = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1712	    \$_.ParentProcessId -eq [int]\$c
  1713	  })
  1714	  if (\$children.Count -gt 1) { throw \"multiple daemon children belong to launcher \$c\" }
  1715	  if (\$children.Count -eq 1) { \$d = [string]\$children[0].ProcessId }
  1716	}
  1717	\"P|\$c|\$d\"
  1718	" 2>/dev/null | tr -d '\r' | tail -1); then
  1719	            teardown_die "Windows PID recovery failed for block $current_block"
  1720	            return 1
  1721	        fi
  1722	        IFS='|' read -r _ cmdpid pid <<<"$pid_probe"
  1723	    fi
  1724	    if [[ -z "$pid" && -z "$cmdpid" ]]; then
  1725	        if [[ -n "$current_block" ]] && ! wssh \
  1726	            "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
  1727	            >/dev/null 2>&1; then
  1728	            teardown_die "Windows PID files are empty but port $PORT may still be open"
  1729	            return 1
  1730	        fi
  1731	        return 0
  1732	    fi
  1733	    [[ -z "$pid" || "$pid" =~ ^[0-9]+$ ]] \
  1734	        || { teardown_die "invalid remembered Windows daemon PID '$pid'"; return 1; }
  1735	    [[ -z "$cmdpid" || "$cmdpid" =~ ^[0-9]+$ ]] \
  1736	        || { teardown_die "invalid remembered Windows launcher PID '$cmdpid'"; return 1; }
  1737	    [[ -n "$current_block" ]] \
  1738	        || { teardown_die "cannot verify Windows launcher without a current block"; return 1; }
  1739	    out=$(wssh "
  1740	\$ErrorActionPreference = 'Stop'
  1741	\$pid0 = if ('$pid' -match '^[0-9]+$') { [int]'$pid' } else { \$null }
  1742	\$cmd0 = if ('$cmdpid' -match '^[0-9]+$') { [int]'$cmdpid' } else { \$null }
  1743	\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
  1744	\$c = if (\$cmd0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$cmd0\" -ErrorAction SilentlyContinue } else { \$null }
  1745	if (\$pid0) {
  1746	  \$d = Get-CimInstance Win32_Process -Filter \"ProcessId=\$pid0\" -ErrorAction SilentlyContinue
  1747	} elseif (\$cmd0) {
  1748	  \$children = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1749	    \$_.ParentProcessId -eq \$cmd0
  1750	  })
  1751	  if (\$children.Count -gt 1) { throw \"multiple daemon children belong to launcher \$cmd0\" }
  1752	  \$d = if (\$children.Count -eq 1) { \$children[0] } else { \$null }
  1753	} else {
  1754	  \$d = \$null
  1755	}
  1756	if (\$d) {
  1757	  \$actual = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1758	  if (\$d.Name -ine 'blit-daemon.exe' -or \$actual -ine '$WIN_ACTIVE') { throw \"daemon PID identity mismatch: \$(\$d.Name) \$(\$d.ExecutablePath)\" }
  1759	  if (\$cmd0 -and \$d.ParentProcessId -ne \$cmd0) { throw \"daemon parent mismatch: \$(\$d.ParentProcessId) != \$cmd0\" }
  1760	}
  1761	if (\$c) {
  1762	  \$actualLauncher = if (\$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1763	  if (\$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$expectedLauncher) { throw \"launcher command mismatch: \$(\$c.Name) \$actualLauncher\" }
  1764	}
  1765	# Every identity is validated before either remembered PID is stopped.
  1766	\$stoppedDaemonPid = if (\$d) { [int]\$d.ProcessId } else { \$null }
  1767	if (\$d) { Stop-Process -Id \$stoppedDaemonPid -Force }
  1768	if (\$c) { Stop-Process -Id \$cmd0 -Force }
  1769	Start-Sleep -Milliseconds 250
  1770	if (\$cmd0) {
  1771	  \$lateChildren = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1772	    \$_.ParentProcessId -eq \$cmd0
  1773	  })
  1774	  foreach (\$late in \$lateChildren) {
  1775	    \$actualLate = if (\$late.ExecutablePath) { \$late.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1776	    if (\$actualLate -ine '$WIN_ACTIVE') { throw \"late daemon child identity mismatch: \$(\$late.ExecutablePath)\" }
  1777	    Stop-Process -Id \$late.ProcessId -Force
  1778	  }
  1779	  if (\$lateChildren.Count -gt 0) { Start-Sleep -Milliseconds 250 }
  1780	}
  1781	if (\$stoppedDaemonPid -and (Get-Process -Id \$stoppedDaemonPid -ErrorAction SilentlyContinue)) { throw 'daemon survived teardown' }
  1782	if (\$cmd0 -and (Get-Process -Id \$cmd0 -ErrorAction SilentlyContinue)) { throw 'launcher survived teardown' }
  1783	if (\$cmd0 -and (@(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object { \$_.ParentProcessId -eq \$cmd0 }).Count -gt 0)) { throw 'late daemon child survived teardown' }
  1784	'STOPPED'
  1785	") || { teardown_die "Windows exact daemon teardown failed: $out"; return 1; }
  1786	    win_daemon_pid=""; win_cmd_pid=""
  1787	}
  1788
  1789	fetch_win_file() {
  1790	    local remote="$1" local_path="$2" tmp="$local_path.base64" remote_hash local_hash
  1791	    wssh "
  1792	\$b = [IO.File]::ReadAllBytes('$remote')
  1793	[Convert]::ToBase64String(\$b)
  1794	" | tr -d '\r\n' > "$tmp" || session_void "failed to fetch Windows log $remote"
  1795	    python3 - "$tmp" "$local_path" <<'PY'
  1796	import base64, pathlib, sys
  1797	src, dst = map(pathlib.Path, sys.argv[1:])
  1798	dst.write_bytes(base64.b64decode(src.read_text(), validate=True))
  1799	src.unlink()
  1800	PY
  1801	    remote_hash=$(sha256_win "$remote")
  1802	    local_hash=$(sha256_q "$local_path")
  1803	    [[ "$remote_hash" == "$local_hash" ]] \
  1804	        || session_void "Windows log hash mismatch for $remote"
  1805	}
  1806
  1807	collect_block_logs() {
  1808	    local block="$1" dir="$OUT_DIR/trace/block_$block"
  1809	    mkdir -p "$dir"
  1810	    fetch_win_file "$WIN_SESSION/block_$block/daemon.err" "$dir/windows-daemon.err"
  1811	    wssh "Remove-Item -LiteralPath '$WIN_SESSION/block_$block' -Recurse -Force -ErrorAction Stop" \
  1812	        >/dev/null || session_void "failed to remove retrieved Windows block $block logs"
  1813	}
  1814
  1815	stop_daemons() {
  1816	    local block="$1"
  1817	    win_daemon_stop
  1818	    q_daemon_stop
  1819	    collect_block_logs "$block"
  1820	    ports_closed || session_void "port $PORT still listening after block $block teardown"
  1821	}
  1822
  1823	q_daemon_start() {
  1824	    local block="$1" state="$2" run_id="$3" dir="$OUT_DIR/trace/block_$block"
  1825	    mkdir -p "$dir"
  1826	    cat > "$dir/q-daemon.toml" <<EOF
  1827	[daemon]
  1828	bind = "0.0.0.0"
  1829	port = $PORT
  1830	no_mdns = true
  1831
  1832	[[module]]
  1833	name = "bench"
  1834	path = "$Q_MODULE"
  1835	EOF
  1836	    if [[ "$state" == on ]]; then
  1837	        BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
  1838	            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
  1839	            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
  1840	    else
  1841	        env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID \
  1842	            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
  1843	            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
  1844	    fi
  1845	    q_daemon_pid=$!
  1846	    sleep 1
  1847	    kill -0 "$q_daemon_pid" 2>/dev/null \
  1848	        || session_void "q daemon failed to start in block $block"
  1849	}
  1850
  1851	win_daemon_start() {
  1852	    local block="$1" state="$2" run_id="$3" out
  1853	    # The CIM-created batch launcher is allowed to exist before its PID is
  1854	    # journaled, but launch.ok prevents it from executing the daemon until the
  1855	    # PID has been atomically placed and read back. Without the gate it times
  1856	    # out, so teardown never has to identify an unjournaled orphan daemon.
  1857	    out=$(wssh "
  1858	\$ErrorActionPreference = 'Stop'
  1859	New-Item -ItemType Directory -Force -Path '$WIN_SESSION/block_$block','$WIN_BINS/active' | Out-Null
  1860	\$startupState = @(
  1861	  '$WIN_SESSION/block_$block/launch.ok',
  1862	  '$WIN_SESSION/block_$block/launcher.pid',
  1863	  '$WIN_SESSION/block_$block/launcher.pid.tmp',
  1864	  '$WIN_SESSION/block_$block/daemon.pid'
  1865	)
  1866	foreach (\$path in \$startupState) {
  1867	  if (Test-Path -LiteralPath \$path) { throw \"stale launcher state: \$path\" }
  1868	}
  1869	Copy-Item -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -Destination '$WIN_ACTIVE' -Force
  1870	if ((Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_ACTIVE').Hash.ToLower() -ne '$WIN_DAEMON_HASH') { throw 'active daemon hash mismatch' }
  1871	Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.toml' -Value @(
  1872	  '[daemon]', 'bind = \"0.0.0.0\"', 'port = $PORT', 'no_mdns = true', '',
  1873	  '[[module]]', 'name = \"bench\"', 'path = \"$WIN_MODULE\"'
  1874	)
  1875	\$trace = if ('$state' -eq 'on') { @('set BLIT_TRACE_SESSION_PHASES=1','set BLIT_TRACE_RUN_ID=$run_id') } else { @('set BLIT_TRACE_SESSION_PHASES=','set BLIT_TRACE_RUN_ID=') }
  1876	Set-Content -LiteralPath '$WIN_SESSION/block_$block/start.cmd' -Value @(
  1877	  '@echo off',
  1878	  'set /a BLIT_LAUNCH_WAIT=0',
  1879	  ':wait_for_launch_ok',
  1880	  'if exist \"$WIN_SESSION/block_$block/launch.ok\" goto launch_ready',
  1881	  'set /a BLIT_LAUNCH_WAIT+=1',
  1882	  'if %BLIT_LAUNCH_WAIT% GEQ 15 exit /b 111',
  1883	  '>nul 2>&1 ping -n 2 127.0.0.1',
  1884	  'goto wait_for_launch_ok',
  1885	  ':launch_ready',
  1886	  \$trace[0], \$trace[1],
  1887	  '\"$WIN_ACTIVE\" --config \"$WIN_SESSION/block_$block/daemon.toml\" > \"$WIN_SESSION/block_$block/daemon.out\" 2> \"$WIN_SESSION/block_$block/daemon.err\"'
  1888	)
  1889	\$launcherCommand = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$block/start.cmd\"\"'
  1890	\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = \$launcherCommand }
  1891	if (\$r.ReturnValue -ne 0) { throw \"launcher return \$(\$r.ReturnValue)\" }
  1892	Set-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Value ([string]\$r.ProcessId) -NoNewline
  1893	Move-Item -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Destination '$WIN_SESSION/block_$block/launcher.pid' -ErrorAction Stop
  1894	\$persistedLauncher = (Get-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid' -Raw -ErrorAction Stop).Trim()
  1895	if (\$persistedLauncher -ne [string]\$r.ProcessId) { throw \"launcher PID persistence mismatch: \$persistedLauncher\" }
  1896	New-Item -ItemType File -Path '$WIN_SESSION/block_$block/launch.ok' -ErrorAction Stop | Out-Null
  1897	Start-Sleep -Seconds 2
  1898	\$c = Get-CimInstance Win32_Process -Filter \"ProcessId=\$(\$r.ProcessId)\" -ErrorAction SilentlyContinue
  1899	\$actualLauncher = if (\$c -and \$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1900	if (-not \$c -or \$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$launcherCommand) { throw \"launcher identity mismatch: \$(\$c.Name) \$actualLauncher\" }
  1901	\$d = Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object ParentProcessId -eq \$r.ProcessId | Select-Object -First 1
  1902	if (-not \$d) { Get-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.err' -ErrorAction SilentlyContinue; throw 'daemon child absent' }
  1903	\$actualDaemon = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1904	if (\$actualDaemon -ine '$WIN_ACTIVE' -or \$d.ParentProcessId -ne \$r.ProcessId) { throw \"daemon identity mismatch: \$(\$d.ExecutablePath) parent=\$(\$d.ParentProcessId)\" }
  1905	Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.pid' -Value \$d.ProcessId
  1906	\"P|\$(\$r.ProcessId)|\$(\$d.ProcessId)\"
  1907	") || session_void "Windows daemon failed to start in block $block: $out"
  1908	    out=${out//$'\r'/}
  1909	    IFS='|' read -r _ win_cmd_pid win_daemon_pid <<<"$(grep '^P|' <<<"$out" | tail -1)"
  1910	    [[ "$win_cmd_pid" =~ ^[0-9]+$ && "$win_daemon_pid" =~ ^[0-9]+$ ]] \
  1911	        || session_void "cannot parse Windows daemon PIDs from '$out'"
  1912	}
  1913
  1914	start_daemons() {
  1915	    local block="$1" state="$2" run_id="$3"
  1916	    ports_closed || session_void "port $PORT occupied before block $block"
  1917	    q_daemon_start "$block" "$state" "$run_id"
  1918	    win_daemon_start "$block" "$state" "$run_id"
  1919	    sleep 1
  1920	    nc -z -w 3 "$WIN_IP" "$PORT" || session_void "q cannot reach Windows daemon in block $block"
  1921	    wssh "if (-not (Test-NetConnection -ComputerName '$Q_IP' -Port $PORT -InformationLevel Quiet)) { exit 8 }" \
  1922	        >/dev/null || session_void "Windows cannot reach q daemon in block $block"
  1923	    log "block $block daemons up, trace=$state, run_id=$run_id"
  1924	}
  1925
  1926	record_clock_samples() {
  1927	    local block="$1" run_id="$2" cell="$3" pair="$4" role="$5" phase="$6" sample before after remote rtt midpoint offset
  1928	    for sample in 1 2 3; do
  1929	        before=$(python3 -c 'import time; print(time.time_ns())')
  1930	        remote=$(wssh '([DateTime]::UtcNow.Ticks - 621355968000000000) * 100' | tr -cd '0-9')
  1931	        after=$(python3 -c 'import time; print(time.time_ns())')
  1932	        [[ "$remote" =~ ^[0-9]+$ ]] || session_void "clock probe returned '$remote'"
  1933	        rtt=$((after - before)); midpoint=$((before + rtt / 2)); offset=$((remote - midpoint))
  1934	        append_clock_row \
  1935	            "$block" "$run_id" "$cell" "$pair" "$role" "$phase" "$sample" \
  1936	            "$before" "$remote" "$after" "$rtt" "$offset" >> "$CLOCK_CSV"
  1937	    done
  1938	}
  1939
  1940	drain_both() {
  1941	    sync || return 1
  1942	    sudo -n /usr/sbin/purge >/dev/null || return 1
  1943	    wssh "
  1944	\$ErrorActionPreference = 'Stop'
  1945	Write-VolumeCache D
  1946	\$quiet = 0
  1947	for (\$i=0; \$i -lt 30; \$i++) {
  1948	  \$w = (Get-Counter '\\PhysicalDisk(_Total)\\Disk Write Bytes/sec' -SampleInterval 1 -MaxSamples 1).CounterSamples[0].CookedValue
  1949	  if (\$null -ne \$w -and [double]\$w -lt 1048576) { \$quiet++ } else { \$quiet=0 }
  1950	  if (\$quiet -ge 3) { break }
  1951	}
  1952	if (\$quiet -lt 3) { throw 'DRAIN-TIMEOUT' }
  1953	if (-not (Test-Path -LiteralPath '$WIN_PURGE')) { throw 'purge helper absent' }
  1954	& pwsh -NoProfile -File '$WIN_PURGE'
  1955	if (\$LASTEXITCODE -ne 0) { throw \"purge helper rc \$LASTEXITCODE\" }
  1956	'drained'
  1957	" >/dev/null || return 1
  1958	    printf drained
  1959	}
  1960
  1961	prepare_destination() {
  1962	    local direction="$1" dest="$2" first
  1963	    if [[ "$direction" == wm ]]; then
  1964	        rm -rf -- "$dest" || return 1
  1965	        [[ ! -e "$dest" && ! -L "$dest" ]] || return 1
  1966	        mkdir -p -- "$dest" || return 1
  1967	        [[ -d "$dest" && ! -L "$dest" ]] || return 1
  1968	        first=$(find "$dest" -mindepth 1 -maxdepth 1 -print -quit) || return 1
  1969	        [[ -z "$first" ]] || return 1
  1970	    else
  1971	        wssh "
  1972	\$ErrorActionPreference = 'Stop'
  1973	if (Test-Path -LiteralPath '$dest') {
  1974	  Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
  1975	}
  1976	if (Test-Path -LiteralPath '$dest') { throw 'destination removal did not land' }
  1977	New-Item -ItemType Directory -Force -Path '$dest' -ErrorAction Stop | Out-Null
  1978	if (-not (Test-Path -LiteralPath '$dest' -PathType Container)) { throw 'destination is not a directory' }
  1979	\$item = Get-Item -LiteralPath '$dest' -Force -ErrorAction Stop
  1980	if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'destination is a reparse point' }
  1981	if (@(Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop).Count -ne 0) { throw 'destination is not empty' }
  1982	" || return 1
  1983	    fi
  1984	}
  1985
  1986	flush_verify_q() {
  1987	    python3 - "$1" <<'PY'
  1988	import os, sys, time
  1989	t=time.monotonic_ns(); n=b=0
  1990	for root, dirs, files in os.walk(sys.argv[1]):
  1991	    for name in files:
  1992	        p=os.path.join(root,name)
  1993	        fd=os.open(p,os.O_RDONLY); os.fsync(fd); os.close(fd)
  1994	        n+=1; b+=os.path.getsize(p)
  1995	print(f"F|{round((time.monotonic_ns()-t)/1_000_000)}|{n}|{b}")
  1996	PY
  1997	}
  1998
  1999	flush_verify_win() {
  2000	    wssh "
  2001	\$ErrorActionPreference = 'Stop'
  2002	\$sw=[Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache D; \$sw.Stop()
  2003	\$f=Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
  2004	\$bytes=if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 }
  2005	\"F|\$([int]\$sw.Elapsed.TotalMilliseconds)|\$(\$f.Count)|\$bytes\"
  2006	" | tr -d '\r' | tail -1
  2007	}
  2008
  2009	q_client_run() {
  2010	    local state="$1" run_id="$2" err="$3"; shift 3
  2011	    local trace_env=()
  2012	    if [[ "$state" == on ]]; then
  2013	        trace_env=(BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id")
  2014	    fi
  2015	    env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID "${trace_env[@]}" \
  2016	        python3 - "$err" "$Q_BLIT" "$@" <<'PY'
  2017	import os, subprocess, sys, time
  2018	err, argv = sys.argv[1], sys.argv[2:]
  2019	clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
  2020	with open(err, "wb") as e:
  2021	    t=clock_ns()
  2022	    p=subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=e, env=os.environ.copy())
  2023	    done_ns=clock_ns()
  2024	    ms=round((done_ns-t)/1_000_000)
  2025	print(f"R|{ms}|{p.returncode}|{done_ns}")
  2026	PY
  2027	}
  2028
  2029	win_client_run() {
  2030	    local state="$1" run_id="$2" remote_err="$3"; shift 3
  2031	    local src="$1" dst="$2" flag="${3:-}" out
  2032	    out=$(wssh "
  2033	\$ErrorActionPreference = 'Stop'
  2034	if ('$state' -eq 'on') { \$env:BLIT_TRACE_SESSION_PHASES='1'; \$env:BLIT_TRACE_RUN_ID='$run_id' }
  2035	else { Remove-Item Env:BLIT_TRACE_SESSION_PHASES,Env:BLIT_TRACE_RUN_ID -ErrorAction SilentlyContinue }
  2036	\$sw=[Diagnostics.Stopwatch]::StartNew()
  2037	& '$WIN_BINS/$HEAD_SHORT/blit.exe' copy '$src' '$dst' --yes $flag > \$null 2> '$remote_err'
  2038	\$rc=\$LASTEXITCODE; \$sw.Stop()
  2039	[Console]::Out.WriteLine(\"R|\$([int]\$sw.Elapsed.TotalMilliseconds)|\${rc}\")
  2040	[Console]::Out.Flush()
  2041	" | stamp_result_arrival_on_q) || true
  2042	    printf '%s\n' "$out"
  2043	}
  2044
  2045	session_id_from_log() {
  2046	    python3 - "$1" <<'PY'
  2047	import json, re, sys
  2048	ids=set()
  2049	with open(sys.argv[1], errors="replace") as f:
  2050	    for line in f:
  2051	        if line.startswith("[session-phase] "):
  2052	            ids.add(json.loads(line[len("[session-phase] "):])["session_id"])
  2053	if len(ids)>1: raise SystemExit(f"multiple session ids: {sorted(ids)}")
  2054	print(next(iter(ids), ""))
  2055	PY
  2056	}
  2057

 succeeded in 0ms:
  2058	run_arm() {
  2059	    local block="$1" state="$2" pass="$3" run_id="$4" cell="$5" pair="$6" role="$7" role_order="$8"
  2060	    local direction carrier shape flag="" dest dest_arg rid qerr werr client_rel client_abs remote_err result result_tag result_extra transfer_ms rc flush_out flush_ms count bytes want drain session_id total anchor_now_ns
  2061	    local windows_client=0 arm_phase=client_done client_done_ns settle_deadline_ns settle_done_ns settled_ms
  2062	    local landed_root landed_manifest canonical_manifest remote_manifest tree_manifest_sha256
  2063	    direction=${cell%%_*}
  2064	    carrier=${cell#*_}; carrier=${carrier%%_*}
  2065	    shape=${cell##*_}
  2066	    [[ "$carrier" == grpc ]] && flag=--force-grpc
  2067	    rid="b${block}_${cell}_p${pair}_${role}"
  2068	    qerr="$OUT_DIR/client/$rid.err"
  2069	    remote_err="$WIN_SESSION/block_$block/$rid.client.err"
  2070	    werr="$OUT_DIR/client/$rid.windows.err"
  2071
  2072	    dest=$(arm_destination_path "$direction" "$role") \
  2073	        || session_void "unregistered destination path for $direction/$role"
  2074	    dest_arg=$(arm_destination_argument "$direction" "$role") \
  2075	        || session_void "unregistered destination argument for $direction/$role"
  2076	    prepare_destination "$direction" "$dest" \
  2077	        || session_void "$rid could not precreate its destination container"
  2078
  2079	    drain=$(drain_both) || session_void "$rid cache/drain gate failed"
  2080	    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" before
  2081
  2082	    if [[ "$direction/$role" == wm/source_init ]]; then
  2083	        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
  2084	        result=$(win_client_run "$state" "$run_id" "$remote_err" \
  2085	            "$(win_source_path "$shape")" "$dest_arg" "$flag")
  2086	    elif [[ "$direction/$role" == wm/destination_init ]]; then
  2087	        client_abs="$qerr"; client_rel="client/$rid.err"
  2088	        result=$(q_client_run "$state" "$run_id" "$qerr" \
  2089	            copy "$WIN_IP:$PORT:/bench/src_$shape" "$dest_arg" --yes ${flag:+$flag})
  2090	    elif [[ "$direction/$role" == mw/source_init ]]; then
  2091	        client_abs="$qerr"; client_rel="client/$rid.err"
  2092	        result=$(q_client_run "$state" "$run_id" "$qerr" \
  2093	            copy "$(q_source_path "$shape")" "$dest_arg" --yes ${flag:+$flag})
  2094	    elif [[ "$direction/$role" == mw/destination_init ]]; then
  2095	        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
  2096	        result=$(win_client_run "$state" "$run_id" "$remote_err" \
  2097	            "$Q_IP:$PORT:/bench/src_$shape" "$dest_arg" "$flag")
  2098	    else
  2099	        session_void "unregistered arm $direction/$role"
  2100	    fi
  2101
  2102	    # Both wrappers carry a q-monotonic completion anchor: immediate child
  2103	    # return for a q client, and result-line arrival for a Windows client.
  2104	    # Wrapper/SSH teardown after that anchor is therefore inside the absolute
  2105	    # settle interval.  The first 250 ms is the common excluded observation
  2106	    # budget; every overrun remains charged to the durable total below.
  2107	    IFS='|' read -r result_tag transfer_ms rc client_done_ns result_extra <<<"$result"
  2108	    if [[ "$result_tag" != R || ! "$transfer_ms" =~ ^[0-9]+$ \
  2109	        || ! "$rc" =~ ^[0-9]+$ || ! "$client_done_ns" =~ ^[0-9]+$ \
  2110	        || -n "$result_extra" ]]; then
  2111	        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
  2112	        session_void "$rid timer/client sentinel malformed: '$result'"
  2113	    fi
  2114	    if [[ "$rc" != 0 ]]; then
  2115	        # Fetch this client log opportunistically; the failure trap also keeps
  2116	        # the remote session tree intact for postmortem evidence.
  2117	        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
  2118	        session_void "$rid client failed rc=$rc (see $client_rel)"
  2119	    fi
  2120
  2121	    anchor_now_ns=$(q_monotonic_ns)
  2122	    [[ "$client_done_ns" -le "$anchor_now_ns" ]] \
  2123	        || session_void "$rid client completion anchor is in the future"
  2124	    [[ $((anchor_now_ns - client_done_ns)) -lt $((SETTLE_MAX_MS * 1000000)) ]] \
  2125	        || session_void "$rid client wrapper teardown already exceeded the settle bound"
  2126	    settle_deadline_ns=$((client_done_ns + SETTLE_NS))
  2127
  2128	    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after
  2129	    settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")
  2130	    [[ "$settle_done_ns" =~ ^[0-9]+$ && "$settle_done_ns" -ge "$settle_deadline_ns" ]] \
  2131	        || session_void "$rid absolute post-client settle returned early: '$settle_done_ns'"
  2132	    settled_ms=$(((settle_done_ns - client_done_ns) / 1000000))
  2133	    [[ "$settled_ms" -ge "$SETTLE_MIN_MS" && "$settled_ms" -lt "$SETTLE_MAX_MS" ]] \
  2134	        || session_void "$rid post-client settle was ${settled_ms}ms, expected [$SETTLE_MIN_MS,$SETTLE_MAX_MS)"
  2135
  2136	    # The destination OS—not the initiator role—selects the durability and
  2137	    # landed-tree probe.  This remains outside transfer_ms.
  2138	    landed_root="src_$shape"
  2139	    landed_manifest="$OUT_DIR/landed/$rid.manifest"
  2140	    canonical_manifest="$OUT_DIR/fixtures/src_$shape.manifest"
  2141	    if [[ "$direction" == wm ]]; then
  2142	        flush_out=$(flush_verify_q "$dest") || session_void "$rid q durability probe failed"
  2143	        write_q_tree_manifest "$dest" "$landed_manifest" "$landed_root" \
  2144	            || session_void "$rid q landed root/manifest verification failed"
  2145	    else
  2146	        flush_out=$(flush_verify_win "$dest") || session_void "$rid Windows durability probe failed"
  2147	        remote_manifest="$WIN_SESSION/block_$block/$rid.tree.manifest"
  2148	        write_win_tree_manifest \
  2149	            "$dest" "$remote_manifest" "$landed_manifest" "$landed_root" \
  2150	            || session_void "$rid Windows landed root/manifest verification failed"
  2151	    fi
  2152	    IFS='|' read -r _ flush_ms count bytes <<<"$flush_out"
  2153	    case "$shape" in mixed) want='5001|547110912';; large) want='1|1073741824';; esac
  2154	    [[ "$count|$bytes" == "$want" ]] \
  2155	        || session_void "$rid landed $count files/$bytes bytes, expected $want"
  2156	    [[ "$flush_ms" =~ ^[0-9]+$ ]] || session_void "$rid flush timer malformed: '$flush_out'"
  2157	    tree_manifest_sha256=$(matching_manifest_digest \
  2158	        "$canonical_manifest" "$landed_manifest") \
  2159	        || session_void "$rid landed relative-path/size manifest differs from canonical"
  2160	    [[ "$tree_manifest_sha256" =~ ^[0-9a-f]{64}$ ]] \
  2161	        || session_void "$rid tree manifest digest is malformed"
  2162	    if [[ "$direction" == wm ]]; then
  2163	        rm -rf -- "$dest" || session_void "$rid failed to remove verified q destination"
  2164	        [[ ! -e "$dest" && ! -L "$dest" ]] \
  2165	            || session_void "$rid verified q destination survived removal"
  2166	    else
  2167	        wssh "
  2168	\$ErrorActionPreference = 'Stop'
  2169	Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
  2170	if (Test-Path -LiteralPath '$dest') { throw 'verified destination survived removal' }
  2171	" \
  2172	            || session_void "$rid failed to remove verified Windows destination"
  2173	    fi
  2174	    arm_phase=durability_verified
  2175
  2176	    if [[ "$windows_client" == 1 ]]; then
  2177	        fetch_successful_windows_client_log "$arm_phase" "$remote_err" "$werr"
  2178	    fi
  2179
  2180	    session_id=$(session_id_from_log "$client_abs") \
  2181	        || session_void "$rid client trace is malformed"
  2182	    if [[ "$state" == on && "$carrier" == tcp ]]; then
  2183	        [[ "$session_id" =~ ^[0-9a-f]{16}$ ]] \
  2184	            || session_void "$rid trace-on TCP client has session_id '$session_id'"
  2185	    else
  2186	        [[ -z "$session_id" ]] \
  2187	            || session_void "$rid emitted TCP phase trace in state=$state carrier=$carrier"
  2188	    fi
  2189
  2190	    total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))
  2191	    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
  2192	        "$block" "$state" "$pass" "$cell" "$role" "$pair" "$role_order" \
  2193	        "$transfer_ms" "$settled_ms" "$flush_ms" "$total" "$landed_root" \
  2194	        "$tree_manifest_sha256" "$rc" "$drain" yes "$run_id" "$session_id" \
  2195	        "$client_rel" >> "$RUNS_CSV"
  2196	    log "$rid: transfer=${transfer_ms}ms settled=${settled_ms}ms flush=${flush_ms}ms total=${total}ms session=${session_id:-none}"
  2197	}
  2198
  2199	cell_order() {
  2200	    local pass="$1" round="$2"
  2201	    local forward='wm_tcp_mixed mw_tcp_mixed wm_grpc_mixed wm_tcp_large'
  2202	    local reverse='wm_tcp_large wm_grpc_mixed mw_tcp_mixed wm_tcp_mixed'
  2203	    local base
  2204	    [[ "$pass" == forward ]] && base="$forward" || base="$reverse"
  2205	    case "$round" in 1|4) printf '%s\n' "$base";; 2|3) [[ "$base" == "$forward" ]] && printf '%s\n' "$reverse" || printf '%s\n' "$forward";; esac
  2206	}
  2207
  2208	run_block() {
  2209	    local block="$1" state="$2" pass="$3" first="$4" last="$5" run_id="${SESSION_TAG}-b${block}-${state}"
  2210	    local round pair cells cell first_role second_role
  2211	    q_quiet_gate; win_quiet_gate
  2212	    start_daemons "$block" "$state" "$run_id"
  2213	    for ((round=1; round<=PAIRS_PER_BLOCK; round++)); do
  2214	        pair=$((first + round - 1))
  2215	        [[ "$pair" -le "$last" ]] || session_void "block $block pair schedule overflow"
  2216	        q_quiet_gate
  2217	        case "$round" in
  2218	            1|4) first_role=source_init; second_role=destination_init;;
  2219	            2|3) first_role=destination_init; second_role=source_init;;
  2220	        esac
  2221	        cells=$(cell_order "$pass" "$round")
  2222	        local old_ifs="$IFS"; IFS=' '
  2223	        for cell in $cells; do
  2224	            IFS="$old_ifs"
  2225	            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$first_role" 1
  2226	            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$second_role" 2
  2227	            IFS=' '
  2228	        done
  2229	        IFS="$old_ifs"
  2230	    done
  2231	    stop_daemons "$block"
  2232	    q_quiet_gate; win_quiet_gate
  2233	}
  2234
  2235	end_gate() {
  2236	    q_topology_gate
  2237	    win_topology_gate
  2238	    mss_gate
  2239	    q_quiet_gate
  2240	    win_quiet_gate
  2241	    ports_closed || session_void "end gate found a listener on port $PORT"
  2242	}
  2243
  2244	strict_success_cleanup() {
  2245	    STRICT_CLEANUP_VERIFIED=0
  2246	    [[ -z "$q_daemon_pid" ]] \
  2247	        || { LAST_ERROR="strict cleanup found remembered q daemon PID $q_daemon_pid"; return 1; }
  2248	    [[ -z "$win_daemon_pid" ]] \
  2249	        || { LAST_ERROR="strict cleanup found remembered Windows daemon PID $win_daemon_pid"; return 1; }
  2250	    [[ -z "$win_cmd_pid" ]] \
  2251	        || { LAST_ERROR="strict cleanup found remembered Windows launcher PID $win_cmd_pid"; return 1; }
  2252	    [[ -z "$current_block" ]] \
  2253	        || { LAST_ERROR="strict cleanup found current block $current_block"; return 1; }
  2254
  2255	    ports_closed \
  2256	        || { LAST_ERROR="strict cleanup found port $PORT still listening"; return 1; }
  2257	    wssh "
  2258	\$ErrorActionPreference = 'Stop'
  2259	\$paths = @('$WIN_MODULE/rigw-sessions/$SESSION_TAG', '$WIN_SESSION')
  2260	foreach (\$path in \$paths) {
  2261	  if (Test-Path -LiteralPath \$path) {
  2262	    Remove-Item -LiteralPath \$path -Recurse -Force -ErrorAction Stop
  2263	  }
  2264	  if (Test-Path -LiteralPath \$path) { throw \"strict cleanup left \$path\" }
  2265	}
  2266	    " >/dev/null \
  2267	        || { LAST_ERROR="strict cleanup could not remove and verify Windows session trees"; return 1; }
  2268	    WIN_SESSION_MAY_EXIST=0
  2269	    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
  2270	        rm -rf -- "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
  2271	            || { LAST_ERROR="strict cleanup could not remove q session tree"; return 1; }
  2272	    fi
  2273	    [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
  2274	        && ! -L "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
  2275	        || { LAST_ERROR="strict cleanup found a surviving or unexpected q session tree"; return 1; }
  2276	    Q_SESSION_MAY_EXIST=0
  2277	    ports_closed \
  2278	        || { LAST_ERROR="strict cleanup found port $PORT reopened during deletion"; return 1; }
  2279	    STRICT_CLEANUP_VERIFIED=1
  2280	}
  2281
  2282	launcher_smoke() {
  2283	    local run_id="${SESSION_TAG}-launcher-smoke"
  2284	    WIN_SESSION_MAY_EXIST=1
  2285	    current_block=launcher-smoke
  2286	    ports_closed \
  2287	        || session_void "port $PORT occupied before launcher smoke"
  2288	    win_daemon_start "$current_block" off "$run_id"
  2289	    nc -z -w 3 "$WIN_IP" "$PORT" \
  2290	        || session_void "q cannot reach Windows daemon in launcher smoke"
  2291	    stop_daemons "$current_block"
  2292	    current_block=""
  2293	    strict_success_cleanup \
  2294	        || session_void "launcher smoke cleanup failed: ${LAST_ERROR:-unknown error}"
  2295	    log "LAUNCHER_SMOKE OK: exact Windows CIM launcher started, reached, identity-stopped, and cleaned; no transfer timed"
  2296	}
  2297
  2298	finalize_registered_session() {
  2299	    local complete_tmp="$OUT_DIR/SESSION-COMPLETE.tmp"
  2300	    SESSION_FINALIZED=0
  2301	    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 ]] \
  2302	        || { LAST_ERROR="refusing cleanup before local evidence is complete"; return 1; }
  2303	    strict_success_cleanup || return 1
  2304	    [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
  2305	        || { LAST_ERROR="strict cleanup returned without verification"; return 1; }
  2306	    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]] \
  2307	        || { LAST_ERROR="refusing to complete a void session"; return 1; }
  2308	    [[ ! -e "$OUT_DIR/SESSION-COMPLETE" && ! -L "$OUT_DIR/SESSION-COMPLETE" ]] \
  2309	        || { LAST_ERROR="refusing to replace an existing completion marker"; return 1; }
  2310	    [[ ! -e "$complete_tmp" && ! -L "$complete_tmp" ]] \
  2311	        || { LAST_ERROR="refusing to replace an existing completion temporary"; return 1; }
  2312	    printf '%s\n' "$HEAD_FULL" > "$complete_tmp" || return 1
  2313	    mv "$complete_tmp" "$OUT_DIR/SESSION-COMPLETE" || return 1
  2314	    SESSION_FINALIZED=1
  2315	}
  2316
  2317	record_failure_evidence() {
  2318	    append_void_line "local evidence preserved at $OUT_DIR"
  2319	    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
  2320	        append_void_line "q session evidence may remain; inspect $Q_MODULE/rigw-sessions/$SESSION_TAG"
  2321	    fi
  2322	    if [[ "$WIN_SESSION_MAY_EXIST" == 1 ]]; then
  2323	        append_void_line "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG"
  2324	    fi
  2325	}
  2326
  2327	on_signal() {
  2328	    local signal="$1" code="$2"
  2329	    LAST_ERROR="received $signal"
  2330	    trap '' HUP INT TERM
  2331	    exit "$code"
  2332	}
  2333
  2334	install_signal_traps() {
  2335	    trap 'on_signal HUP 129' HUP
  2336	    trap 'on_signal INT 130' INT
  2337	    trap 'on_signal TERM 143' TERM
  2338	}
  2339
  2340	registered_completion_marker_valid() {
  2341	    local marker="$OUT_DIR/SESSION-COMPLETE" lines
  2342	    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 \
  2343	        && -n "${HEAD_FULL:-}" && -f "$marker" && ! -L "$marker" ]] || return 1
  2344	    lines=$(LC_ALL=C wc -l < "$marker") || return 1
  2345	    lines=${lines//[[:space:]]/}
  2346	    [[ "$lines" == 1 && "$(< "$marker")" == "$HEAD_FULL" ]] || return 1
  2347	    [[ ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
  2348	        && ! -L "$OUT_DIR/SESSION-COMPLETE.tmp" ]] || return 1
  2349	    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]]
  2350	}
  2351
  2352	on_exit() {
  2353	    local rc=$?
  2354	    trap - EXIT
  2355	    trap '' HUP INT TERM
  2356	    set +e
  2357	    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
  2358	        && ( -e "$OUT_DIR/SESSION-VOID" || -L "$OUT_DIR/SESSION-VOID" \
  2359	            || -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
  2360	            || -L "$OUT_DIR/SESSION-COMPLETE.tmp" ) ]]; then
  2361	        LAST_ERROR="successful exit retained a failure or temporary marker"
  2362	        rc=1
  2363	    fi
  2364	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 \
  2365	        && "$SESSION_FINALIZED" != 1 ]]; then
  2366	        LAST_ERROR="registered run returned without finalizing the session"
  2367	        rc=1
  2368	    fi
  2369	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 ]] \
  2370	        && ! registered_completion_marker_valid; then
  2371	        LAST_ERROR="registered completion marker is absent or invalid"
  2372	        rc=1
  2373	    fi
  2374	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
  2375	        && "$SESSION_FINALIZED" != 0 ]]; then
  2376	        LAST_ERROR="non-registered run claimed registered finalization"
  2377	        rc=1
  2378	    fi
  2379	    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
  2380	        && ( -e "$OUT_DIR/SESSION-COMPLETE" \
  2381	            || -L "$OUT_DIR/SESSION-COMPLETE" ) ]]; then
  2382	        LAST_ERROR="non-registered run left a completion marker"
  2383	        rc=1
  2384	    fi
  2385	    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
  2386	        && "$STRICT_CLEANUP_VERIFIED" != 1 ]]; then
  2387	        LAST_ERROR="successful exit lacked verified strict cleanup"
  2388	        rc=1
  2389	    fi
  2390
  2391	    if [[ $rc -ne 0 ]]; then
  2392	        rm -f -- "$OUT_DIR/SESSION-COMPLETE" "$OUT_DIR/SESSION-COMPLETE.tmp" \
  2393	            || CLEANUP_ERROR="${CLEANUP_ERROR:+$CLEANUP_ERROR; }could not remove completion marker"
  2394	        if [[ ! -s "$OUT_DIR/SESSION-VOID" ]]; then
  2395	            append_void_line "${LAST_ERROR:-unexpected harness failure rc=$rc}"
  2396	        fi
  2397	        CLEANUP_MODE=1
  2398	        if [[ -n "$win_daemon_pid" || -n "$win_cmd_pid" || -n "$current_block" ]]; then
  2399	            win_daemon_stop || true
  2400	        fi
  2401	        if [[ -n "$q_daemon_pid" ]]; then q_daemon_stop || true; fi
  2402	        if [[ -n "$CLEANUP_ERROR" ]]; then
  2403	            append_void_line "cleanup errors: $CLEANUP_ERROR"
  2404	        fi
  2405	        record_failure_evidence
  2406	        exit 1
  2407	    fi
  2408	    exit 0
  2409	}
  2410
  2411	main() {
  2412	    validate_mode_selection
  2413	    if [[ "$SELFTEST" == 1 ]]; then selftest; return; fi
  2414	    if ! claim_output_dir; then
  2415	        printf '%s\n' "FATAL: $OUTPUT_CLAIM_ERROR" >&2
  2416	        return 1
  2417	    fi
  2418	    trap on_exit EXIT
  2419	    install_signal_traps
  2420	    preflight
  2421	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
  2422	        strict_success_cleanup || session_void "preflight cleanup failed: ${LAST_ERROR:-unknown error}"
  2423	        log "PREFLIGHT_ONLY: no daemon started and no transfer timed"
  2424	        return
  2425	    fi
  2426	    if [[ "$LAUNCHER_SMOKE" == 1 ]]; then
  2427	        launcher_smoke
  2428	        return
  2429	    fi
  2430
  2431	    REGISTERED_RUN_STARTED=1
  2432	    Q_SESSION_MAY_EXIST=1
  2433	    mkdir -p "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
  2434	        || session_void "cannot create registered q session directory"
  2435	    printf '%s\n' 'block,trace_state,pass,cell,role,pair,role_order,transfer_ms,settled_ms,flush_ms,total_ms,landed_root,tree_manifest_sha256,exit,drain,valid,run_id,session_id,client_log' > "$RUNS_CSV"
  2436	    printf '%s\n' 'block,run_id,cell,pair,role,phase,sample,q_before_ns,windows_ns,q_after_ns,rtt_ns,offset_windows_minus_q_ns' > "$CLOCK_CSV"
  2437	    emit_schedule > "$OUT_DIR/schedule.csv"
  2438	    WIN_SESSION_MAY_EXIST=1
  2439	    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION' | Out-Null" \
  2440	        || session_void "cannot create registered Windows session directory"
  2441
  2442	    local block state pass first last
  2443	    while IFS=, read -r block state pass first last; do
  2444	        current_block="$block"
  2445	        run_block "$block" "$state" "$pass" "$first" "$last"
  2446	        current_block=""
  2447	    done < <(emit_schedule)
  2448
  2449	    end_gate
  2450	    python3 "$SCRIPT_DIR/otp12pf_rigw_analyze.py" "$OUT_DIR" \
  2451	        || session_void "phase/distribution analyzer rejected the session"
  2452	    LOCAL_EVIDENCE_COMPLETE=1
  2453	    log "ANALYZER ACCEPTED: exact local evidence inventory; finalizing session"
  2454	    finalize_registered_session \
  2455	        || session_void "registered finalization failed: ${LAST_ERROR:-unknown error}"
  2456	}
  2457
  2458	if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  2459	    main "$@"
  2460	fi

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '1914,2057p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '1400,1654p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1914	start_daemons() {
  1915	    local block="$1" state="$2" run_id="$3"
  1916	    ports_closed || session_void "port $PORT occupied before block $block"
  1917	    q_daemon_start "$block" "$state" "$run_id"
  1918	    win_daemon_start "$block" "$state" "$run_id"
  1919	    sleep 1
  1920	    nc -z -w 3 "$WIN_IP" "$PORT" || session_void "q cannot reach Windows daemon in block $block"
  1921	    wssh "if (-not (Test-NetConnection -ComputerName '$Q_IP' -Port $PORT -InformationLevel Quiet)) { exit 8 }" \
  1922	        >/dev/null || session_void "Windows cannot reach q daemon in block $block"
  1923	    log "block $block daemons up, trace=$state, run_id=$run_id"
  1924	}
  1925
  1926	record_clock_samples() {
  1927	    local block="$1" run_id="$2" cell="$3" pair="$4" role="$5" phase="$6" sample before after remote rtt midpoint offset
  1928	    for sample in 1 2 3; do
  1929	        before=$(python3 -c 'import time; print(time.time_ns())')
  1930	        remote=$(wssh '([DateTime]::UtcNow.Ticks - 621355968000000000) * 100' | tr -cd '0-9')
  1931	        after=$(python3 -c 'import time; print(time.time_ns())')
  1932	        [[ "$remote" =~ ^[0-9]+$ ]] || session_void "clock probe returned '$remote'"
  1933	        rtt=$((after - before)); midpoint=$((before + rtt / 2)); offset=$((remote - midpoint))
  1934	        append_clock_row \
  1935	            "$block" "$run_id" "$cell" "$pair" "$role" "$phase" "$sample" \
  1936	            "$before" "$remote" "$after" "$rtt" "$offset" >> "$CLOCK_CSV"
  1937	    done
  1938	}
  1939
  1940	drain_both() {
  1941	    sync || return 1
  1942	    sudo -n /usr/sbin/purge >/dev/null || return 1
  1943	    wssh "
  1944	\$ErrorActionPreference = 'Stop'
  1945	Write-VolumeCache D
  1946	\$quiet = 0
  1947	for (\$i=0; \$i -lt 30; \$i++) {
  1948	  \$w = (Get-Counter '\\PhysicalDisk(_Total)\\Disk Write Bytes/sec' -SampleInterval 1 -MaxSamples 1).CounterSamples[0].CookedValue
  1949	  if (\$null -ne \$w -and [double]\$w -lt 1048576) { \$quiet++ } else { \$quiet=0 }
  1950	  if (\$quiet -ge 3) { break }
  1951	}
  1952	if (\$quiet -lt 3) { throw 'DRAIN-TIMEOUT' }
  1953	if (-not (Test-Path -LiteralPath '$WIN_PURGE')) { throw 'purge helper absent' }
  1954	& pwsh -NoProfile -File '$WIN_PURGE'
  1955	if (\$LASTEXITCODE -ne 0) { throw \"purge helper rc \$LASTEXITCODE\" }
  1956	'drained'
  1957	" >/dev/null || return 1
  1958	    printf drained
  1959	}
  1960
  1961	prepare_destination() {
  1962	    local direction="$1" dest="$2" first
  1963	    if [[ "$direction" == wm ]]; then
  1964	        rm -rf -- "$dest" || return 1
  1965	        [[ ! -e "$dest" && ! -L "$dest" ]] || return 1
  1966	        mkdir -p -- "$dest" || return 1
  1967	        [[ -d "$dest" && ! -L "$dest" ]] || return 1
  1968	        first=$(find "$dest" -mindepth 1 -maxdepth 1 -print -quit) || return 1
  1969	        [[ -z "$first" ]] || return 1
  1970	    else
  1971	        wssh "
  1972	\$ErrorActionPreference = 'Stop'
  1973	if (Test-Path -LiteralPath '$dest') {
  1974	  Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
  1975	}
  1976	if (Test-Path -LiteralPath '$dest') { throw 'destination removal did not land' }
  1977	New-Item -ItemType Directory -Force -Path '$dest' -ErrorAction Stop | Out-Null
  1978	if (-not (Test-Path -LiteralPath '$dest' -PathType Container)) { throw 'destination is not a directory' }
  1979	\$item = Get-Item -LiteralPath '$dest' -Force -ErrorAction Stop
  1980	if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'destination is a reparse point' }
  1981	if (@(Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop).Count -ne 0) { throw 'destination is not empty' }
  1982	" || return 1
  1983	    fi
  1984	}
  1985
  1986	flush_verify_q() {
  1987	    python3 - "$1" <<'PY'
  1988	import os, sys, time
  1989	t=time.monotonic_ns(); n=b=0
  1990	for root, dirs, files in os.walk(sys.argv[1]):
  1991	    for name in files:
  1992	        p=os.path.join(root,name)
  1993	        fd=os.open(p,os.O_RDONLY); os.fsync(fd); os.close(fd)
  1994	        n+=1; b+=os.path.getsize(p)
  1995	print(f"F|{round((time.monotonic_ns()-t)/1_000_000)}|{n}|{b}")
  1996	PY
  1997	}
  1998
  1999	flush_verify_win() {
  2000	    wssh "
  2001	\$ErrorActionPreference = 'Stop'
  2002	\$sw=[Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache D; \$sw.Stop()
  2003	\$f=Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
  2004	\$bytes=if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 }
  2005	\"F|\$([int]\$sw.Elapsed.TotalMilliseconds)|\$(\$f.Count)|\$bytes\"
  2006	" | tr -d '\r' | tail -1
  2007	}
  2008
  2009	q_client_run() {
  2010	    local state="$1" run_id="$2" err="$3"; shift 3
  2011	    local trace_env=()
  2012	    if [[ "$state" == on ]]; then
  2013	        trace_env=(BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id")
  2014	    fi
  2015	    env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID "${trace_env[@]}" \
  2016	        python3 - "$err" "$Q_BLIT" "$@" <<'PY'
  2017	import os, subprocess, sys, time
  2018	err, argv = sys.argv[1], sys.argv[2:]
  2019	clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
  2020	with open(err, "wb") as e:
  2021	    t=clock_ns()
  2022	    p=subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=e, env=os.environ.copy())
  2023	    done_ns=clock_ns()
  2024	    ms=round((done_ns-t)/1_000_000)
  2025	print(f"R|{ms}|{p.returncode}|{done_ns}")
  2026	PY
  2027	}
  2028
  2029	win_client_run() {
  2030	    local state="$1" run_id="$2" remote_err="$3"; shift 3
  2031	    local src="$1" dst="$2" flag="${3:-}" out
  2032	    out=$(wssh "
  2033	\$ErrorActionPreference = 'Stop'
  2034	if ('$state' -eq 'on') { \$env:BLIT_TRACE_SESSION_PHASES='1'; \$env:BLIT_TRACE_RUN_ID='$run_id' }
  2035	else { Remove-Item Env:BLIT_TRACE_SESSION_PHASES,Env:BLIT_TRACE_RUN_ID -ErrorAction SilentlyContinue }
  2036	\$sw=[Diagnostics.Stopwatch]::StartNew()
  2037	& '$WIN_BINS/$HEAD_SHORT/blit.exe' copy '$src' '$dst' --yes $flag > \$null 2> '$remote_err'
  2038	\$rc=\$LASTEXITCODE; \$sw.Stop()
  2039	[Console]::Out.WriteLine(\"R|\$([int]\$sw.Elapsed.TotalMilliseconds)|\${rc}\")
  2040	[Console]::Out.Flush()
  2041	" | stamp_result_arrival_on_q) || true
  2042	    printf '%s\n' "$out"
  2043	}
  2044
  2045	session_id_from_log() {
  2046	    python3 - "$1" <<'PY'
  2047	import json, re, sys
  2048	ids=set()
  2049	with open(sys.argv[1], errors="replace") as f:
  2050	    for line in f:
  2051	        if line.startswith("[session-phase] "):
  2052	            ids.add(json.loads(line[len("[session-phase] "):])["session_id"])
  2053	if len(ids)>1: raise SystemExit(f"multiple session ids: {sorted(ids)}")
  2054	print(next(iter(ids), ""))
  2055	PY
  2056	}
  2057

 succeeded in 0ms:
  1400	\$b = \$s.GetSocketOption([Net.Sockets.SocketOptionLevel]::Tcp,\$name,4)
  1401	\$m = [BitConverter]::ToInt32(\$b,0)
  1402	\"M|\${m}|\$(\$s.LocalEndPoint.Address)\"
  1403	\$s.Dispose()
  1404	" | tr -d '\r' | tail -1
  1405	}
  1406
  1407	mss_gate() {
  1408	    local qout wout qm qip wm wip
  1409	    qout=$(q_to_win_mss) || die "q→Windows MSS probe failed"
  1410	    read -r qm qip <<<"$qout"
  1411	    [[ "$qm" == "$Q_TO_WIN_MSS" && "$qip" == "$Q_IP" ]] \
  1412	        || die "q→Windows MSS/source is '$qout', expected $Q_TO_WIN_MSS $Q_IP"
  1413	    wout=$(win_to_q_mss) || die "Windows→q MSS probe failed"
  1414	    IFS='|' read -r _ wm wip <<<"$wout"
  1415	    [[ "$wm" == "$WIN_TO_Q_MSS" && "$wip" == "$WIN_IP" ]] \
  1416	        || die "Windows→q MSS/source is '$wout', expected M|$WIN_TO_Q_MSS|$WIN_IP"
  1417	    log "path MSS: q→Windows=$qm via $qip; Windows→q=$wm via $wip"
  1418	}
  1419
  1420	firewall_gate() {
  1421	    local out
  1422	    out=$(wssh "
  1423	\$r = Get-NetFirewallRule -DisplayName 'blit-otp12-daemon' -ErrorAction SilentlyContinue
  1424	if (-not \$r) { exit 4 }
  1425	\$app = \$r | Get-NetFirewallApplicationFilter
  1426	\"F|\$(\$r.Enabled)|\$(\$r.Action)|\$(\$r.Direction)|\$(\$app.Program)\"
  1427	") || die "existing Windows firewall rule is absent/unreadable; harness will not create it"
  1428	    out=${out//$'\r'/}
  1429	    out=$(sed 's#\\#/#g' <<<"$out")
  1430	    out=$(tr 'A-Z' 'a-z' <<<"$out")
  1431	    local expected
  1432	    expected=$(tr 'A-Z' 'a-z' <<<"F|True|Allow|Inbound|$WIN_ACTIVE")
  1433	    [[ "$out" == "$expected" ]] \
  1434	        || die "Windows firewall rule mismatch: '$out'"
  1435	    log "firewall verified only: existing inbound allow is scoped to $WIN_ACTIVE"
  1436	}
  1437
  1438	ports_closed() {
  1439	    if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  1440	        return 1
  1441	    fi
  1442	    wssh "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
  1443	        >/dev/null 2>&1
  1444	}
  1445
  1446	timer_gate() {
  1447	    local qms wout wms
  1448	    qms=$(python3 - <<'PY'
  1449	import time
  1450	clock_ns=lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)
  1451	t=clock_ns(); time.sleep(1); print(round((clock_ns()-t)/1_000_000))
  1452	PY
  1453	)
  1454	    [[ "$qms" -ge 950 && "$qms" -le 1050 ]] || die "q one-second timer calibrated to ${qms}ms"
  1455	    wout=$(wssh '$s=[Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Seconds 1; $s.Stop(); "T|$([int]$s.Elapsed.TotalMilliseconds)"') \
  1456	        || die "Windows timer calibration failed"
  1457	    wout=${wout//$'\r'/}; wms=${wout##*|}
  1458	    [[ "$wms" -ge 950 && "$wms" -le 1050 ]] || die "Windows one-second timer calibrated to ${wms}ms"
  1459	    log "timer calibration: q=${qms}ms Windows=${wms}ms"
  1460	}
  1461
  1462	windows_result_stream_gate() {
  1463	    local before after result tag ms rc stamp extra teardown_ns
  1464	    before=$(q_monotonic_ns)
  1465	    result=$(wssh \
  1466	        '[Console]::Out.WriteLine("R|17|0"); [Console]::Out.Flush(); Start-Sleep -Milliseconds 350' \
  1467	        | stamp_result_arrival_on_q) \
  1468	        || die "Windows result-stream probe failed"
  1469	    after=$(q_monotonic_ns)
  1470	    IFS='|' read -r tag ms rc stamp extra <<<"$result"
  1471	    [[ "$tag" == R && "$ms" == 17 && "$rc" == 0 \
  1472	        && "$stamp" =~ ^[0-9]+$ && -z "$extra" ]] \
  1473	        || die "Windows result-stream probe returned '$result'"
  1474	    [[ "$stamp" -ge "$before" && "$stamp" -le "$after" ]] \
  1475	        || die "Windows result-stream q stamp is outside the probe lifetime"
  1476	    teardown_ns=$((after - stamp))
  1477	    [[ "$teardown_ns" -ge 250000000 ]] \
  1478	        || die "Windows result stream was not observable before remote teardown"
  1479	    log "Windows result stream reaches q before remote teardown"
  1480	}
  1481
  1482	fixture_shape_q() {
  1483	    python3 - "$1" <<'PY'
  1484	import os, sys
  1485	n=b=0
  1486	for root, dirs, files in os.walk(sys.argv[1]):
  1487	    for name in files:
  1488	        p=os.path.join(root,name); n+=1; b+=os.path.getsize(p)
  1489	print(f"{n},{b}")
  1490	PY
  1491	}
  1492
  1493	fixture_shape_win() {
  1494	    wssh "
  1495	\$f = Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
  1496	\"S|\$(\$f.Count)|\$(if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 })\"
  1497	" | tr -d '\r' | sed -n 's/^S|//p' | tr '|' ',' | tail -1
  1498	}
  1499
  1500	write_q_tree_manifest() {
  1501	    python3 - "$1" "$2" "${3:-}" <<'PY'
  1502	import base64, os, pathlib, stat, sys
  1503
  1504	root = pathlib.Path(sys.argv[1])
  1505	output = pathlib.Path(sys.argv[2])
  1506	expected_root = sys.argv[3]
  1507	if not root.is_dir() or root.is_symlink():
  1508	    raise SystemExit(f"manifest root is not a plain directory: {root}")
  1509	if expected_root:
  1510	    entries = list(root.iterdir())
  1511	    if (
  1512	        len(entries) != 1
  1513	        or entries[0].name != expected_root
  1514	        or not entries[0].is_dir()
  1515	        or entries[0].is_symlink()
  1516	    ):
  1517	        raise SystemExit(
  1518	            f"landed container must contain exactly plain directory {expected_root}"
  1519	        )
  1520	    root = entries[0]
  1521
  1522	lines = []
  1523	def walk_error(error):
  1524	    raise error
  1525
  1526	for current, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
  1527	    for name in dirs:
  1528	        path = pathlib.Path(current, name)
  1529	        mode = path.lstat().st_mode
  1530	        if not stat.S_ISDIR(mode) or stat.S_ISLNK(mode):
  1531	            raise SystemExit(f"non-directory/reparse entry in manifest: {path}")
  1532	    for name in files:
  1533	        path = pathlib.Path(current, name)
  1534	        info = path.lstat()
  1535	        if not stat.S_ISREG(info.st_mode):
  1536	            raise SystemExit(f"non-regular entry in manifest: {path}")
  1537	        relative = path.relative_to(root).as_posix()
  1538	        encoded = base64.b64encode(relative.encode("utf-8")).decode("ascii")
  1539	        lines.append(f"{encoded},{info.st_size}")
  1540	lines.sort()
  1541	output.write_text("".join(f"{line}\n" for line in lines), encoding="ascii")
  1542	PY
  1543	}
  1544
  1545	write_win_tree_manifest() {
  1546	    local root="$1" remote_out="$2" local_out="$3" expected_root="${4:-}"
  1547	    wssh "
  1548	\$ErrorActionPreference = 'Stop'
  1549	\$root = (Resolve-Path -LiteralPath '$root').Path.TrimEnd([char]92,[char]47)
  1550	if ('$expected_root') {
  1551	  \$entries = @(Get-ChildItem -LiteralPath \$root -Force -ErrorAction Stop)
  1552	  if (\$entries.Count -ne 1 -or -not \$entries[0].PSIsContainer -or \$entries[0].Name -cne '$expected_root' -or ((\$entries[0].Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'landed root layout mismatch' }
  1553	  \$root = \$entries[0].FullName.TrimEnd([char]92,[char]47)
  1554	}
  1555	\$lines = [Collections.Generic.List[string]]::new()
  1556	foreach (\$item in @(Get-ChildItem -LiteralPath \$root -Recurse -Force -ErrorAction Stop)) {
  1557	  if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw \"reparse entry in manifest: \$(\$item.FullName)\" }
  1558	  if (\$item.PSIsContainer) { continue }
  1559	  if (-not (\$item -is [IO.FileInfo])) { throw \"non-regular entry in manifest: \$(\$item.FullName)\" }
  1560	  \$rel = \$item.FullName.Substring(\$root.Length).TrimStart([char]92,[char]47).Replace([char]92,[char]47)
  1561	  \$b64 = [Convert]::ToBase64String([Text.UTF8Encoding]::new(\$false,\$true).GetBytes(\$rel))
  1562	  \$lines.Add(\"\$b64,\$([uint64]\$item.Length)\")
  1563	}
  1564	\$ordered = [string[]]\$lines.ToArray()
  1565	[Array]::Sort(\$ordered, [StringComparer]::Ordinal)
  1566	\$text = if (\$ordered.Count) { (\$ordered -join \"`n\") + \"`n\" } else { '' }
  1567	[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName('$remote_out')) | Out-Null
  1568	[IO.File]::WriteAllText('$remote_out', \$text, [Text.UTF8Encoding]::new(\$false))
  1569	" || return 1
  1570	    fetch_win_file "$remote_out" "$local_out" || return 1
  1571	    LC_ALL=C sort -o "$local_out" "$local_out"
  1572	}
  1573
  1574	matching_manifest_digest() {
  1575	    local canonical="$1" landed="$2"
  1576	    cmp -s "$canonical" "$landed" || return 1
  1577	    sha256_q "$landed"
  1578	}
  1579
  1580	verify_fixtures() {
  1581	    local shape want qgot wgot qmanifest wmanifest qhash
  1582	    printf '%s\n' 'shape,sha256,q_manifest,windows_manifest' \
  1583	        > "$OUT_DIR/fixture-manifests.csv"
  1584	    WIN_SESSION_MAY_EXIST=1
  1585	    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION/fixtures' | Out-Null" \
  1586	        || die "cannot create Windows fixture evidence directory"
  1587	    for shape in mixed large; do
  1588	        case "$shape" in
  1589	            mixed) want=5001,547110912;;
  1590	            large) want=1,1073741824;;
  1591	        esac
  1592	        qgot=$(fixture_shape_q "$(q_source_path "$shape")")
  1593	        wgot=$(fixture_shape_win "$(win_source_path "$shape")")
  1594	        [[ "$qgot" == "$want" ]] || die "q src_$shape is $qgot, expected $want"
  1595	        [[ "$wgot" == "$want" ]] || die "Windows canonical src_$shape is $wgot, expected $want"
  1596	        qmanifest="$OUT_DIR/fixtures/src_$shape.manifest"
  1597	        wmanifest="$OUT_DIR/fixtures/windows-src_$shape.manifest"
  1598	        write_q_tree_manifest "$(q_source_path "$shape")" "$qmanifest" \
  1599	            || die "q src_$shape manifest failed"
  1600	        write_win_tree_manifest \
  1601	            "$(win_source_path "$shape")" \
  1602	            "$WIN_SESSION/fixtures/src_$shape.manifest" "$wmanifest" \
  1603	            || die "Windows src_$shape manifest failed"
  1604	        qhash=$(matching_manifest_digest "$qmanifest" "$wmanifest") \
  1605	            || die "q and Windows src_$shape relative-path/size manifests differ"
  1606	        printf '%s,%s,%s,%s\n' \
  1607	            "$shape" "$qhash" "fixtures/src_$shape.manifest" \
  1608	            "fixtures/windows-src_$shape.manifest" \
  1609	            >> "$OUT_DIR/fixture-manifests.csv"
  1610	    done
  1611	    log "canonical fixtures verified byte-for-byte by relative path and size on both hosts"
  1612	}
  1613
  1614	write_manifest() {
  1615	    local qbh qdh wbh wdh
  1616	    qbh=$(sha256_q "$Q_BLIT"); qdh=$(sha256_q "$Q_DAEMON")
  1617	    wbh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit.exe")
  1618	    wdh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit-daemon.exe")
  1619	    cat > "$OUT_DIR/staging-manifest.csv" <<EOF
  1620	host,role,commit,sha256,path
  1621	q,client,$HEAD_FULL,$qbh,$Q_BLIT
  1622	q,daemon,$HEAD_FULL,$qdh,$Q_DAEMON
  1623	windows,client,$HEAD_FULL,$wbh,$WIN_BINS/$HEAD_SHORT/blit.exe
  1624	windows,daemon,$HEAD_FULL,$wdh,$WIN_BINS/$HEAD_SHORT/blit-daemon.exe
  1625	EOF
  1626	    WIN_DAEMON_HASH=$wdh
  1627	}
  1628
  1629	provenance_gate() {
  1630	    [[ -n "$EXPECT_SHA" ]] || die "EXPECT_SHA=<full reviewed commit> is required"
  1631	    HEAD_FULL=$(git -C "$REPO_ROOT" rev-parse HEAD)
  1632	    HEAD_SHORT=$(git -C "$REPO_ROOT" rev-parse --short=7 HEAD)
  1633	    HEAD_BUILD_ID=$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD)
  1634	    [[ "$EXPECT_SHA" == "$HEAD_FULL" ]] \
  1635	        || die "EXPECT_SHA=$EXPECT_SHA but isolated clone is $HEAD_FULL"
  1636	    [[ -z $(git -C "$REPO_ROOT" status --porcelain --untracked-files=normal) ]] \
  1637	        || die "isolated q clone is dirty"
  1638	    [[ -x "$Q_BLIT" && -x "$Q_DAEMON" ]] || die "q release binaries are absent"
  1639	    embeds_clean_q "$Q_BLIT" \
  1640	        || die "q client does not embed a clean +$HEAD_BUILD_ID"
  1641	    embeds_clean_q "$Q_DAEMON" \
  1642	        || die "q daemon does not embed a clean +$HEAD_BUILD_ID"
  1643	    wssh "
  1644	if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe')) { exit 2 }
  1645	if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe')) { exit 3 }
  1646	if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 4 }
  1647	if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 5 }
  1648	if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 6 }
  1649	if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 7 }
  1650	" || die "Windows binaries are missing or do not embed a clean +$HEAD_BUILD_ID"
  1651	    write_manifest
  1652	    log "provenance exact: $HEAD_FULL on q and Windows"
  1653	}
  1654

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '1655,1913p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1655	preflight() {
  1656	    reject_registered_overrides
  1657	    command -v python3 >/dev/null || die "python3 required"
  1658	    command -v lsof >/dev/null || die "lsof required"
  1659	    command -v nc >/dev/null || die "nc required"
  1660	    sudo -n /usr/sbin/purge >/dev/null || die "q NOPASSWD purge grant is absent"
  1661	    provenance_gate
  1662	    ports_closed || die "port $PORT already has a listener on q or Windows"
  1663	    q_topology_gate
  1664	    win_topology_gate
  1665	    mss_gate
  1666	    firewall_gate
  1667	    q_quiet_gate
  1668	    win_quiet_gate
  1669	    timer_gate
  1670	    windows_result_stream_gate
  1671	    verify_fixtures
  1672	    log "PREFLIGHT OK: registered rig, exact binaries, canonical paths, quiet endpoints"
  1673	}
  1674
  1675	q_daemon_stop() {
  1676	    local pid="$q_daemon_pid" i
  1677	    [[ -z "$pid" ]] && return 0
  1678	    if kill -0 "$pid" 2>/dev/null; then
  1679	        local cmd
  1680	        cmd=$(ps -p "$pid" -o command= 2>/dev/null || true)
  1681	        [[ "$cmd" == *"$Q_DAEMON"* ]] \
  1682	            || { teardown_die "refusing to stop q PID $pid because it is not the launched daemon: $cmd"; return 1; }
  1683	        kill "$pid" || true
  1684	        for ((i=0; i<40; i++)); do
  1685	            kill -0 "$pid" 2>/dev/null || break
  1686	            sleep 0.25
  1687	        done
  1688	        kill -0 "$pid" 2>/dev/null \
  1689	            && { teardown_die "q daemon PID $pid survived exact teardown"; return 1; }
  1690	    fi
  1691	    q_daemon_pid=""
  1692	}
  1693
  1694	win_daemon_stop() {
  1695	    local pid="$win_daemon_pid" cmdpid="$win_cmd_pid" out pid_probe
  1696	    if [[ -z "$pid" && -z "$cmdpid" && -n "$current_block" ]]; then
  1697	        if ! pid_probe=$(wssh "
  1698	\$ErrorActionPreference = 'Stop'
  1699	\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
  1700	\$d = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/daemon.pid' -ErrorAction SilentlyContinue
  1701	\$c = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/launcher.pid' -ErrorAction SilentlyContinue
  1702	if (-not \$c) {
  1703	  \$launchers = @(Get-CimInstance Win32_Process -Filter \"Name='cmd.exe'\" | Where-Object {
  1704	    \$actual = if (\$_.CommandLine) { \$_.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1705	    \$actual -ieq \$expectedLauncher
  1706	  })
  1707	  if (\$launchers.Count -gt 1) { throw \"multiple exact launchers match \$expectedLauncher\" }
  1708	  if (\$launchers.Count -eq 1) { \$c = [string]\$launchers[0].ProcessId }
  1709	}
  1710	if (-not \$d -and \$c -match '^[0-9]+$') {
  1711	  \$children = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1712	    \$_.ParentProcessId -eq [int]\$c
  1713	  })
  1714	  if (\$children.Count -gt 1) { throw \"multiple daemon children belong to launcher \$c\" }
  1715	  if (\$children.Count -eq 1) { \$d = [string]\$children[0].ProcessId }
  1716	}
  1717	\"P|\$c|\$d\"
  1718	" 2>/dev/null | tr -d '\r' | tail -1); then
  1719	            teardown_die "Windows PID recovery failed for block $current_block"
  1720	            return 1
  1721	        fi
  1722	        IFS='|' read -r _ cmdpid pid <<<"$pid_probe"
  1723	    fi
  1724	    if [[ -z "$pid" && -z "$cmdpid" ]]; then
  1725	        if [[ -n "$current_block" ]] && ! wssh \
  1726	            "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
  1727	            >/dev/null 2>&1; then
  1728	            teardown_die "Windows PID files are empty but port $PORT may still be open"
  1729	            return 1
  1730	        fi
  1731	        return 0
  1732	    fi
  1733	    [[ -z "$pid" || "$pid" =~ ^[0-9]+$ ]] \
  1734	        || { teardown_die "invalid remembered Windows daemon PID '$pid'"; return 1; }
  1735	    [[ -z "$cmdpid" || "$cmdpid" =~ ^[0-9]+$ ]] \
  1736	        || { teardown_die "invalid remembered Windows launcher PID '$cmdpid'"; return 1; }
  1737	    [[ -n "$current_block" ]] \
  1738	        || { teardown_die "cannot verify Windows launcher without a current block"; return 1; }
  1739	    out=$(wssh "
  1740	\$ErrorActionPreference = 'Stop'
  1741	\$pid0 = if ('$pid' -match '^[0-9]+$') { [int]'$pid' } else { \$null }
  1742	\$cmd0 = if ('$cmdpid' -match '^[0-9]+$') { [int]'$cmdpid' } else { \$null }
  1743	\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
  1744	\$c = if (\$cmd0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$cmd0\" -ErrorAction SilentlyContinue } else { \$null }
  1745	if (\$pid0) {
  1746	  \$d = Get-CimInstance Win32_Process -Filter \"ProcessId=\$pid0\" -ErrorAction SilentlyContinue
  1747	} elseif (\$cmd0) {
  1748	  \$children = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1749	    \$_.ParentProcessId -eq \$cmd0
  1750	  })
  1751	  if (\$children.Count -gt 1) { throw \"multiple daemon children belong to launcher \$cmd0\" }
  1752	  \$d = if (\$children.Count -eq 1) { \$children[0] } else { \$null }
  1753	} else {
  1754	  \$d = \$null
  1755	}
  1756	if (\$d) {
  1757	  \$actual = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1758	  if (\$d.Name -ine 'blit-daemon.exe' -or \$actual -ine '$WIN_ACTIVE') { throw \"daemon PID identity mismatch: \$(\$d.Name) \$(\$d.ExecutablePath)\" }
  1759	  if (\$cmd0 -and \$d.ParentProcessId -ne \$cmd0) { throw \"daemon parent mismatch: \$(\$d.ParentProcessId) != \$cmd0\" }
  1760	}
  1761	if (\$c) {
  1762	  \$actualLauncher = if (\$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1763	  if (\$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$expectedLauncher) { throw \"launcher command mismatch: \$(\$c.Name) \$actualLauncher\" }
  1764	}
  1765	# Every identity is validated before either remembered PID is stopped.
  1766	\$stoppedDaemonPid = if (\$d) { [int]\$d.ProcessId } else { \$null }
  1767	if (\$d) { Stop-Process -Id \$stoppedDaemonPid -Force }
  1768	if (\$c) { Stop-Process -Id \$cmd0 -Force }
  1769	Start-Sleep -Milliseconds 250
  1770	if (\$cmd0) {
  1771	  \$lateChildren = @(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object {
  1772	    \$_.ParentProcessId -eq \$cmd0
  1773	  })
  1774	  foreach (\$late in \$lateChildren) {
  1775	    \$actualLate = if (\$late.ExecutablePath) { \$late.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1776	    if (\$actualLate -ine '$WIN_ACTIVE') { throw \"late daemon child identity mismatch: \$(\$late.ExecutablePath)\" }
  1777	    Stop-Process -Id \$late.ProcessId -Force
  1778	  }
  1779	  if (\$lateChildren.Count -gt 0) { Start-Sleep -Milliseconds 250 }
  1780	}
  1781	if (\$stoppedDaemonPid -and (Get-Process -Id \$stoppedDaemonPid -ErrorAction SilentlyContinue)) { throw 'daemon survived teardown' }
  1782	if (\$cmd0 -and (Get-Process -Id \$cmd0 -ErrorAction SilentlyContinue)) { throw 'launcher survived teardown' }
  1783	if (\$cmd0 -and (@(Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object { \$_.ParentProcessId -eq \$cmd0 }).Count -gt 0)) { throw 'late daemon child survived teardown' }
  1784	'STOPPED'
  1785	") || { teardown_die "Windows exact daemon teardown failed: $out"; return 1; }
  1786	    win_daemon_pid=""; win_cmd_pid=""
  1787	}
  1788
  1789	fetch_win_file() {
  1790	    local remote="$1" local_path="$2" tmp="$local_path.base64" remote_hash local_hash
  1791	    wssh "
  1792	\$b = [IO.File]::ReadAllBytes('$remote')
  1793	[Convert]::ToBase64String(\$b)
  1794	" | tr -d '\r\n' > "$tmp" || session_void "failed to fetch Windows log $remote"
  1795	    python3 - "$tmp" "$local_path" <<'PY'
  1796	import base64, pathlib, sys
  1797	src, dst = map(pathlib.Path, sys.argv[1:])
  1798	dst.write_bytes(base64.b64decode(src.read_text(), validate=True))
  1799	src.unlink()
  1800	PY
  1801	    remote_hash=$(sha256_win "$remote")
  1802	    local_hash=$(sha256_q "$local_path")
  1803	    [[ "$remote_hash" == "$local_hash" ]] \
  1804	        || session_void "Windows log hash mismatch for $remote"
  1805	}
  1806
  1807	collect_block_logs() {
  1808	    local block="$1" dir="$OUT_DIR/trace/block_$block"
  1809	    mkdir -p "$dir"
  1810	    fetch_win_file "$WIN_SESSION/block_$block/daemon.err" "$dir/windows-daemon.err"
  1811	    wssh "Remove-Item -LiteralPath '$WIN_SESSION/block_$block' -Recurse -Force -ErrorAction Stop" \
  1812	        >/dev/null || session_void "failed to remove retrieved Windows block $block logs"
  1813	}
  1814
  1815	stop_daemons() {
  1816	    local block="$1"
  1817	    win_daemon_stop
  1818	    q_daemon_stop
  1819	    collect_block_logs "$block"
  1820	    ports_closed || session_void "port $PORT still listening after block $block teardown"
  1821	}
  1822
  1823	q_daemon_start() {
  1824	    local block="$1" state="$2" run_id="$3" dir="$OUT_DIR/trace/block_$block"
  1825	    mkdir -p "$dir"
  1826	    cat > "$dir/q-daemon.toml" <<EOF
  1827	[daemon]
  1828	bind = "0.0.0.0"
  1829	port = $PORT
  1830	no_mdns = true
  1831
  1832	[[module]]
  1833	name = "bench"
  1834	path = "$Q_MODULE"
  1835	EOF
  1836	    if [[ "$state" == on ]]; then
  1837	        BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
  1838	            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
  1839	            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
  1840	    else
  1841	        env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID \
  1842	            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
  1843	            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
  1844	    fi
  1845	    q_daemon_pid=$!
  1846	    sleep 1
  1847	    kill -0 "$q_daemon_pid" 2>/dev/null \
  1848	        || session_void "q daemon failed to start in block $block"
  1849	}
  1850
  1851	win_daemon_start() {
  1852	    local block="$1" state="$2" run_id="$3" out
  1853	    # The CIM-created batch launcher is allowed to exist before its PID is
  1854	    # journaled, but launch.ok prevents it from executing the daemon until the
  1855	    # PID has been atomically placed and read back. Without the gate it times
  1856	    # out, so teardown never has to identify an unjournaled orphan daemon.
  1857	    out=$(wssh "
  1858	\$ErrorActionPreference = 'Stop'
  1859	New-Item -ItemType Directory -Force -Path '$WIN_SESSION/block_$block','$WIN_BINS/active' | Out-Null
  1860	\$startupState = @(
  1861	  '$WIN_SESSION/block_$block/launch.ok',
  1862	  '$WIN_SESSION/block_$block/launcher.pid',
  1863	  '$WIN_SESSION/block_$block/launcher.pid.tmp',
  1864	  '$WIN_SESSION/block_$block/daemon.pid'
  1865	)
  1866	foreach (\$path in \$startupState) {
  1867	  if (Test-Path -LiteralPath \$path) { throw \"stale launcher state: \$path\" }
  1868	}
  1869	Copy-Item -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -Destination '$WIN_ACTIVE' -Force
  1870	if ((Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_ACTIVE').Hash.ToLower() -ne '$WIN_DAEMON_HASH') { throw 'active daemon hash mismatch' }
  1871	Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.toml' -Value @(
  1872	  '[daemon]', 'bind = \"0.0.0.0\"', 'port = $PORT', 'no_mdns = true', '',
  1873	  '[[module]]', 'name = \"bench\"', 'path = \"$WIN_MODULE\"'
  1874	)
  1875	\$trace = if ('$state' -eq 'on') { @('set BLIT_TRACE_SESSION_PHASES=1','set BLIT_TRACE_RUN_ID=$run_id') } else { @('set BLIT_TRACE_SESSION_PHASES=','set BLIT_TRACE_RUN_ID=') }
  1876	Set-Content -LiteralPath '$WIN_SESSION/block_$block/start.cmd' -Value @(
  1877	  '@echo off',
  1878	  'set /a BLIT_LAUNCH_WAIT=0',
  1879	  ':wait_for_launch_ok',
  1880	  'if exist \"$WIN_SESSION/block_$block/launch.ok\" goto launch_ready',
  1881	  'set /a BLIT_LAUNCH_WAIT+=1',
  1882	  'if %BLIT_LAUNCH_WAIT% GEQ 15 exit /b 111',
  1883	  '>nul 2>&1 ping -n 2 127.0.0.1',
  1884	  'goto wait_for_launch_ok',
  1885	  ':launch_ready',
  1886	  \$trace[0], \$trace[1],
  1887	  '\"$WIN_ACTIVE\" --config \"$WIN_SESSION/block_$block/daemon.toml\" > \"$WIN_SESSION/block_$block/daemon.out\" 2> \"$WIN_SESSION/block_$block/daemon.err\"'
  1888	)
  1889	\$launcherCommand = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$block/start.cmd\"\"'
  1890	\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = \$launcherCommand }
  1891	if (\$r.ReturnValue -ne 0) { throw \"launcher return \$(\$r.ReturnValue)\" }
  1892	Set-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Value ([string]\$r.ProcessId) -NoNewline
  1893	Move-Item -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp' -Destination '$WIN_SESSION/block_$block/launcher.pid' -ErrorAction Stop
  1894	\$persistedLauncher = (Get-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid' -Raw -ErrorAction Stop).Trim()
  1895	if (\$persistedLauncher -ne [string]\$r.ProcessId) { throw \"launcher PID persistence mismatch: \$persistedLauncher\" }
  1896	New-Item -ItemType File -Path '$WIN_SESSION/block_$block/launch.ok' -ErrorAction Stop | Out-Null
  1897	Start-Sleep -Seconds 2
  1898	\$c = Get-CimInstance Win32_Process -Filter \"ProcessId=\$(\$r.ProcessId)\" -ErrorAction SilentlyContinue
  1899	\$actualLauncher = if (\$c -and \$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  1900	if (-not \$c -or \$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$launcherCommand) { throw \"launcher identity mismatch: \$(\$c.Name) \$actualLauncher\" }
  1901	\$d = Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object ParentProcessId -eq \$r.ProcessId | Select-Object -First 1
  1902	if (-not \$d) { Get-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.err' -ErrorAction SilentlyContinue; throw 'daemon child absent' }
  1903	\$actualDaemon = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  1904	if (\$actualDaemon -ine '$WIN_ACTIVE' -or \$d.ParentProcessId -ne \$r.ProcessId) { throw \"daemon identity mismatch: \$(\$d.ExecutablePath) parent=\$(\$d.ParentProcessId)\" }
  1905	Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.pid' -Value \$d.ProcessId
  1906	\"P|\$(\$r.ProcessId)|\$(\$d.ProcessId)\"
  1907	") || session_void "Windows daemon failed to start in block $block: $out"
  1908	    out=${out//$'\r'/}
  1909	    IFS='|' read -r _ win_cmd_pid win_daemon_pid <<<"$(grep '^P|' <<<"$out" | tail -1)"
  1910	    [[ "$win_cmd_pid" =~ ^[0-9]+$ && "$win_daemon_pid" =~ ^[0-9]+$ ]] \
  1911	        || session_void "cannot parse Windows daemon PIDs from '$out'"
  1912	}
  1913

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '283,620p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '621,920p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   621	branch = main[branch_start:registered]
   622	branch_markers = ('if [[ "$LAUNCHER_SMOKE" == 1 ]]', "launcher_smoke;", "return;", "fi;")
   623	branch_positions = [branch.index(marker) for marker in branch_markers]
   624	if branch_positions != sorted(branch_positions) or branch.count("return;") != 1:
   625	    raise SystemExit(f"launcher-smoke branch can fall through: {branch_positions}")
   626	PY
   627
   628	    launcher_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-launcher-smoke.XXXXXX")
   629	    launcher_calls="$launcher_tmp/calls"
   630	    (
   631	        OUT_DIR="$launcher_tmp/evidence"
   632	        mkdir "$OUT_DIR"
   633	        LOG="$OUT_DIR/bench.log"
   634	        OUTPUT_CLAIMED=1
   635	        SESSION_TAG=offline-smoke
   636	        REGISTERED_RUN_STARTED=0
   637	        SESSION_FINALIZED=0
   638	        STRICT_CLEANUP_VERIFIED=0
   639	        Q_SESSION_MAY_EXIST=0
   640	        WIN_SESSION_MAY_EXIST=0
   641	        LOCAL_EVIDENCE_COMPLETE=0
   642	        current_block=""
   643	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""
   644	        port_checks=0
   645	        log() { :; }
   646	        win_daemon_start() {
   647	            [[ "$1" == launcher-smoke && "$2" == off \
   648	                && "$3" == offline-smoke-launcher-smoke \
   649	                && "$WIN_SESSION_MAY_EXIST" == 1 \
   650	                && "$current_block" == launcher-smoke ]] \
   651	                || die "offline launcher smoke started with wrong identity"
   652	            printf 'start\n' >> "$launcher_calls"
   653	            win_cmd_pid=22; win_daemon_pid=33
   654	        }
   655	        nc() {
   656	            [[ "$*" == "-z -w 3 $WIN_IP $PORT" \
   657	                && "$win_cmd_pid" == 22 && "$win_daemon_pid" == 33 ]] \
   658	                || die "offline launcher smoke reachability ran out of order"
   659	            printf 'reach\n' >> "$launcher_calls"
   660	        }
   661	        win_daemon_stop() {
   662	            [[ "$current_block" == launcher-smoke \
   663	                && "$win_cmd_pid" == 22 && "$win_daemon_pid" == 33 ]] \
   664	                || die "offline launcher smoke stopped the wrong daemon"
   665	            printf 'stop\n' >> "$launcher_calls"
   666	            win_cmd_pid=""; win_daemon_pid=""
   667	        }
   668	        q_daemon_stop() {
   669	            [[ -z "$q_daemon_pid" ]] \
   670	                || die "offline launcher smoke unexpectedly owned a q daemon"
   671	            printf 'q-stop-empty\n' >> "$launcher_calls"
   672	        }
   673	        collect_block_logs() {
   674	            [[ "$1" == launcher-smoke && -z "$win_cmd_pid" \
   675	                && -z "$win_daemon_pid" ]] \
   676	                || die "offline launcher smoke collected before exact stop"
   677	            printf 'collect\n' >> "$launcher_calls"
   678	        }
   679	        ports_closed() {
   680	            port_checks=$((port_checks + 1))
   681	            if [[ "$port_checks" == 1 ]]; then
   682	                [[ "$current_block" == launcher-smoke \
   683	                    && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   684	                    || die "offline launcher smoke skipped its pre-start port check"
   685	                printf 'closed-pre\n' >> "$launcher_calls"
   686	            else
   687	                [[ "$port_checks" == 2 && "$current_block" == launcher-smoke \
   688	                    && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   689	                    || die "offline launcher smoke checked ports before exact stop"
   690	                printf 'closed-post\n' >> "$launcher_calls"
   691	            fi
   692	        }
   693	        strict_success_cleanup() {
   694	            [[ "$WIN_SESSION_MAY_EXIST" == 1 && -z "$current_block" \
   695	                && -z "$q_daemon_pid" && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   696	                || die "offline launcher smoke cleaned before exact stop"
   697	            printf 'cleanup\n' >> "$launcher_calls"
   698	            WIN_SESSION_MAY_EXIST=0
   699	            STRICT_CLEANUP_VERIFIED=1
   700	        }
   701	        launcher_smoke
   702	        [[ "$(< "$launcher_calls")" == \
   703	            $'closed-pre\nstart\nreach\nstop\nq-stop-empty\ncollect\nclosed-post\ncleanup' ]] \
   704	            || die "offline launcher-smoke call order changed"
   705	        [[ "$REGISTERED_RUN_STARTED" == 0 && "$SESSION_FINALIZED" == 0 \
   706	            && "$STRICT_CLEANUP_VERIFIED" == 1 \
   707	            && "$Q_SESSION_MAY_EXIST" == 0 \
   708	            && "$WIN_SESSION_MAY_EXIST" == 0 \
   709	            && "$LOCAL_EVIDENCE_COMPLETE" == 0 ]] \
   710	            || die "offline launcher smoke changed registered state"
   711	        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" \
   712	            && ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
   713	            && ! -e "$OUT_DIR/SESSION-VOID" ]] \
   714	            || die "offline launcher smoke left a session marker"
   715	    )
   716	    rm -rf "$launcher_tmp"
   717
   718	    HEAD_BUILD_ID=0123456789ab
   719	    identity_file=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-identity.XXXXXX")
   720	    printf 'blit+%s\0' "$HEAD_BUILD_ID" > "$identity_file"
   721	    embeds_clean_q "$identity_file" || die "clean 12-character build identity was rejected"
   722	    printf 'blit+%s.dirty.ffffffffffff\0' "$HEAD_BUILD_ID" > "$identity_file"
   723	    if embeds_clean_q "$identity_file"; then
   724	        rm -f "$identity_file"
   725	        die "dirty build identity was accepted"
   726	    fi
   727	    rm -f "$identity_file"
   728
   729	    manifest_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-manifest.XXXXXX")
   730	    mkdir -p "$manifest_tmp/source/sub" "$manifest_tmp/container/src_mixed/sub"
   731	    printf 'a' > "$manifest_tmp/source/a"
   732	    printf 'bc' > "$manifest_tmp/source/sub/b"
   733	    printf 'a' > "$manifest_tmp/container/src_mixed/a"
   734	    printf 'bc' > "$manifest_tmp/container/src_mixed/sub/b"
   735	    canonical_manifest="$manifest_tmp/canonical.manifest"
   736	    landed_manifest="$manifest_tmp/landed.manifest"
   737	    write_q_tree_manifest "$manifest_tmp/source" "$canonical_manifest"
   738	    write_q_tree_manifest \
   739	        "$manifest_tmp/container" "$landed_manifest" src_mixed
   740	    tree_digest=$(matching_manifest_digest "$canonical_manifest" "$landed_manifest") \
   741	        || die "identical relative-path/size manifests did not match"
   742	    [[ "$tree_digest" =~ ^[0-9a-f]{64}$ ]] \
   743	        || die "tree manifest digest is malformed"
   744	    printf 'aa' > "$manifest_tmp/container/src_mixed/a"
   745	    printf 'b' > "$manifest_tmp/container/src_mixed/sub/b"
   746	    write_q_tree_manifest \
   747	        "$manifest_tmp/container" "$landed_manifest" src_mixed
   748	    if matching_manifest_digest "$canonical_manifest" "$landed_manifest" >/dev/null; then
   749	        rm -rf "$manifest_tmp"
   750	        die "same-count/same-byte tree with swapped file sizes was accepted"
   751	    fi
   752	    rm -rf "$manifest_tmp/container/src_mixed"
   753	    mkdir -p "$manifest_tmp/container/wrapper/src_mixed"
   754	    if write_q_tree_manifest \
   755	        "$manifest_tmp/container" "$landed_manifest" src_mixed 2>/dev/null; then
   756	        rm -rf "$manifest_tmp"
   757	        die "wrong landed root wrapper was accepted"
   758	    fi
   759	    rm -rf "$manifest_tmp"
   760
   761	    freshness_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-freshness.XXXXXX")
   762	    reserve_evidence_dir "$freshness_tmp/new-evidence" \
   763	        || die "fresh evidence directory was rejected: $OUTPUT_CLAIM_ERROR"
   764	    for marker in SESSION-COMPLETE SESSION-VOID unrelated.txt; do
   765	        freshness_case="$freshness_tmp/$marker"
   766	        mkdir "$freshness_case"
   767	        printf 'preserve-me\n' > "$freshness_case/$marker"
   768	        before=$(sha256_q "$freshness_case/$marker")
   769	        if reserve_evidence_dir "$freshness_case"; then
   770	            rm -rf "$freshness_tmp"
   771	            die "stale output directory containing $marker was accepted"
   772	        fi
   773	        [[ "$(sha256_q "$freshness_case/$marker")" == "$before" ]] \
   774	            || die "stale output rejection modified $marker"
   775	    done
   776	    rm -rf "$freshness_tmp"
   777
   778	    destination_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-destination.XXXXXX")
   779	    mkdir -p "$destination_tmp/container/src_mixed"
   780	    printf 'stale\n' > "$destination_tmp/container/src_mixed/stale"
   781	    (
   782	        rm() { return 73; }
   783	        if prepare_destination wm "$destination_tmp/container"; then
   784	            die "q destination reset masked a failed removal"
   785	        fi
   786	    )
   787	    [[ "$(< "$destination_tmp/container/src_mixed/stale")" == stale ]] \
   788	        || die "failed q destination reset modified retained evidence"
   789	    prepare_destination wm "$destination_tmp/container" \
   790	        || die "q destination reset rejected a removable tree"
   791	    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]] \
   792	        || die "q destination reset did not leave a plain directory"
   793	    [[ -z "$(find "$destination_tmp/container" -mindepth 1 -maxdepth 1 -print -quit)" ]] \
   794	        || die "q destination reset left stale content"
   795	    rm -rf "$destination_tmp"
   796
   797	    prepare_destination_source=$(declare -f prepare_destination)
   798	    python3 - "$prepare_destination_source" <<'PY' \
   799	        || die "Windows destination reset source contract changed"
   800	import sys
   801
   802	source = sys.argv[1]
   803	for marker in (
   804	    r"\$ErrorActionPreference = 'Stop'",
   805	    r"Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop",
   806	    r"Test-Path -LiteralPath '$dest' -PathType Container",
   807	    r"Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop",
   808	    'ReparsePoint',
   809	):
   810	    if marker not in source:
   811	        raise SystemExit(f"missing Windows destination reset marker: {marker}")
   812	windows = source.split('else', 1)[1]
   813	if 'SilentlyContinue' in windows:
   814	    raise SystemExit("Windows destination reset suppresses removal errors")
   815	PY
   816
   817	    finalize_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-finalize.XXXXXX")
   818	    (
   819	        OUT_DIR="$finalize_tmp/fails"
   820	        mkdir "$OUT_DIR"
   821	        HEAD_FULL=0123456789abcdef
   822	        LOCAL_EVIDENCE_COMPLETE=1
   823	        strict_success_cleanup() { return 1; }
   824	        if finalize_registered_session; then
   825	            die "registered finalization accepted failed strict cleanup"
   826	        fi
   827	        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] \
   828	            || die "failed strict cleanup left SESSION-COMPLETE"
   829	    )
   830	    (
   831	        OUT_DIR="$finalize_tmp/incomplete-local"
   832	        mkdir "$OUT_DIR"
   833	        HEAD_FULL=0123456789abcdef
   834	        LOCAL_EVIDENCE_COMPLETE=0
   835	        strict_success_cleanup() {
   836	            die "finalization cleaned paths before local evidence was complete"
   837	        }
   838	        if finalize_registered_session; then
   839	            die "registered finalization accepted incomplete local evidence"
   840	        fi
   841	        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]]
   842	    )
   843	    (
   844	        OUT_DIR="$finalize_tmp/succeeds"
   845	        mkdir "$OUT_DIR"
   846	        HEAD_FULL=0123456789abcdef
   847	        LOCAL_EVIDENCE_COMPLETE=1
   848	        strict_success_cleanup() {
   849	            [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] || return 1
   850	            STRICT_CLEANUP_VERIFIED=1
   851	        }
   852	        finalize_registered_session \
   853	            || die "registered finalization rejected verified strict cleanup"
   854	        [[ "$SESSION_FINALIZED" == 1 ]] \
   855	            || die "registered finalization did not set SESSION_FINALIZED"
   856	        [[ "$(< "$OUT_DIR/SESSION-COMPLETE")" == "$HEAD_FULL" ]]
   857	    )
   858
   859	    cleanup_tmp="$finalize_tmp/strict"
   860	    mkdir -p "$cleanup_tmp/q/rigw-sessions/fail-remote"
   861	    printf 'retain me\n' > "$cleanup_tmp/q/rigw-sessions/fail-remote/sentinel"
   862	    (
   863	        Q_MODULE="$cleanup_tmp/q"
   864	        SESSION_TAG=fail-remote
   865	        Q_SESSION_MAY_EXIST=1
   866	        WIN_SESSION_MAY_EXIST=1
   867	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   868	        ports_closed() { return 0; }
   869	        wssh() { return 1; }
   870	        if strict_success_cleanup; then
   871	            die "strict cleanup accepted a Windows deletion failure"
   872	        fi
   873	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
   874	            || die "Windows cleanup failure was marked strictly verified"
   875	        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
   876	            || die "Windows cleanup failure deleted q evidence first"
   877	        [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
   878	            || die "Windows cleanup failure modified q evidence"
   879	    )
   880	    mkdir -p "$cleanup_tmp/q/rigw-sessions/open-port"
   881	    (
   882	        Q_MODULE="$cleanup_tmp/q"
   883	        SESSION_TAG=open-port
   884	        Q_SESSION_MAY_EXIST=1
   885	        WIN_SESSION_MAY_EXIST=1
   886	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   887	        ports_closed() { return 1; }
   888	        wssh() { die "strict cleanup reached deletion with an open port"; }
   889	        if strict_success_cleanup; then
   890	            die "strict cleanup accepted an open port"
   891	        fi
   892	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
   893	            || die "open-port cleanup failure was marked strictly verified"
   894	        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
   895	    )
   896	    mkdir -p "$cleanup_tmp/q/rigw-sessions/surviving-q"
   897	    (
   898	        Q_MODULE="$cleanup_tmp/q"
   899	        SESSION_TAG=surviving-q
   900	        Q_SESSION_MAY_EXIST=1
   901	        WIN_SESSION_MAY_EXIST=1
   902	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   903	        ports_closed() { return 0; }
   904	        wssh() { return 0; }
   905	        rm() { return 0; }
   906	        if strict_success_cleanup; then
   907	            die "strict cleanup accepted a surviving q session tree"
   908	        fi
   909	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
   910	            || die "surviving q session tree was marked strictly verified"
   911	        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
   912	    )
   913	    mkdir -p "$cleanup_tmp/q/rigw-sessions/succeeds"
   914	    (
   915	        Q_MODULE="$cleanup_tmp/q"
   916	        SESSION_TAG=succeeds
   917	        Q_SESSION_MAY_EXIST=1
   918	        WIN_SESSION_MAY_EXIST=1
   919	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   920	        port_checks=0

 succeeded in 0ms:
   283	selftest() {
   284	    local got expected rows source_first destination_first clock_probe identity_file
   285	    local selftest_client_done selftest_deadline selftest_settle_done run_arm_source
   286	    local manifest_tmp canonical_manifest landed_manifest tree_digest
   287	    local freshness_tmp freshness_case marker before analyzer_log
   288	    local win_stop_source win_start_source finalize_tmp failure_tmp trap_calls trap_rc
   289	    local signal signal_dir signal_rc contract_tmp on_exit_source append_tmp
   290	    local cleanup_tmp remembered port_checks strict_cleanup_source
   291	    local destination_tmp prepare_destination_source stamped_result stamp_before stamp_after
   292	    local stamp_tag stamp_ms stamp_rc stamp_ns stamp_extra stamp_teardown_ns
   293	    local cross_clock_before cross_clock_after cross_clock_delta
   294	    local launcher_tmp launcher_calls launcher_source main_source
   295	    local win_recovery_tmp
   296	    reject_registered_overrides
   297	    if (
   298	        SELFTEST=1
   299	        PREFLIGHT_ONLY=1
   300	        LAUNCHER_SMOKE=0
   301	        validate_mode_selection
   302	    ) >/dev/null 2>&1; then
   303	        die "multiple harness modes were accepted"
   304	    fi
   305	    if (
   306	        SELFTEST=2
   307	        PREFLIGHT_ONLY=0
   308	        LAUNCHER_SMOKE=0
   309	        validate_mode_selection
   310	    ) >/dev/null 2>&1; then
   311	        die "invalid harness mode value was accepted"
   312	    fi
   313	    got=$(emit_schedule)
   314	    expected=$'1,off,forward,1,4\n2,on,reverse,1,4\n3,on,forward,5,8\n4,off,reverse,5,8'
   315	    [[ "$got" == "$expected" ]] || die "registered block schedule changed"
   316
   317	    rows=0; source_first=0; destination_first=0
   318	    local block state pass first last round pair first_role
   319	    while IFS=, read -r block state pass first last; do
   320	        for ((round=1; round<=PAIRS_PER_BLOCK; round++)); do
   321	            pair=$((first + round - 1))
   322	            case "$round" in
   323	                1|4) first_role=source_init; source_first=$((source_first + 4));;
   324	                2|3) first_role=destination_init; destination_first=$((destination_first + 4));;
   325	            esac
   326	            [[ "$pair" -ge "$first" && "$pair" -le "$last" && -n "$first_role" ]]
   327	            rows=$((rows + 8)) # four cells × two adjacent roles
   328	        done
   329	    done < <(emit_schedule)
   330	    [[ "$rows" == 128 ]] || die "schedule emitted $rows arms, expected 128"
   331	    [[ "$source_first" == 32 && "$destination_first" == 32 ]] \
   332	        || die "schedule role-first balance changed"
   333	    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]] \
   334	        || die "q source path construction changed"
   335	    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]] \
   336	        || die "Windows source path construction changed"
   337	    local destination_rel="rigw-sessions/$SESSION_TAG/destination/container"
   338	    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]] \
   339	        || die "q SOURCE-initiated destination path changed"
   340	    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]] \
   341	        || die "q DESTINATION-initiated destination path changed"
   342	    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]] \
   343	        || die "Windows SOURCE-initiated destination path changed"
   344	    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
   345	        || die "Windows DESTINATION-initiated destination path changed"
   346	    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]] \
   347	        || die "Windows-to-q physical destination depends on initiator role"
   348	    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]] \
   349	        || die "q-to-Windows physical destination depends on initiator role"
   350	    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]] \
   351	        || die "Windows-to-q SOURCE-initiated destination argument changed"
   352	    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]] \
   353	        || die "Windows-to-q DESTINATION-initiated destination argument changed"
   354	    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]] \
   355	        || die "q-to-Windows SOURCE-initiated destination argument changed"
   356	    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
   357	        || die "q-to-Windows DESTINATION-initiated destination argument changed"
   358	    clock_probe=$(append_clock_row 1 run cell 1 source_init before 1 10 11 12 2 0)
   359	    [[ "$(awk -F, '{print NF}' <<<"$clock_probe")" == 12 ]] \
   360	        || die "clock sample row is not exactly 12 columns"
   361	    [[ "$SETTLE_NS" == 250000000 && "$SETTLE_MIN_MS" == 250 && "$SETTLE_MAX_MS" == 1000 ]] \
   362	        || die "registered post-client settle bounds changed"
   363	    cross_clock_before=$(q_monotonic_ns)
   364	    sleep 0.05
   365	    cross_clock_after=$(q_monotonic_ns)
   366	    cross_clock_delta=$((cross_clock_after - cross_clock_before))
   367	    [[ "$cross_clock_delta" -ge 40000000 && "$cross_clock_delta" -lt 500000000 ]] \
   368	        || die "q monotonic clock is not comparable across processes"
   369	    selftest_client_done=$(q_monotonic_ns)
   370	    selftest_deadline=$((selftest_client_done + SETTLE_NS))
   371	    selftest_settle_done=$(settle_until_deadline "$selftest_deadline")
   372	    [[ "$selftest_settle_done" =~ ^[0-9]+$ && "$selftest_settle_done" -ge "$selftest_deadline" ]] \
   373	        || die "absolute post-client deadline wait returned early"
   374	    stamp_before=$(q_monotonic_ns)
   375	    stamped_result=$(
   376	        { printf '%s\n' 'R|17|0'; sleep 0.35; } | stamp_result_arrival_on_q
   377	    ) || die "q result-arrival stamper rejected one exact sentinel"
   378	    stamp_after=$(q_monotonic_ns)
   379	    IFS='|' read -r stamp_tag stamp_ms stamp_rc stamp_ns stamp_extra <<<"$stamped_result"
   380	    [[ "$stamp_tag" == R && "$stamp_ms" == 17 && "$stamp_rc" == 0 \
   381	        && "$stamp_ns" =~ ^[0-9]+$ && -z "$stamp_extra" ]] \
   382	        || die "q result-arrival stamper returned '$stamped_result'"
   383	    [[ "$stamp_ns" -ge "$stamp_before" && "$stamp_ns" -le "$stamp_after" ]] \
   384	        || die "q result-arrival stamp is outside the producer lifetime"
   385	    stamp_teardown_ns=$((stamp_after - stamp_ns))
   386	    [[ "$stamp_teardown_ns" -ge 250000000 ]] \
   387	        || die "q result-arrival stamp moved after producer teardown"
   388	    if successful_windows_log_phase_ok client_done; then
   389	        die "successful Windows client log was fetchable before durability"
   390	    fi
   391	    successful_windows_log_phase_ok durability_verified \
   392	        || die "successful Windows client log was blocked after durability"
   393
   394	    run_arm_source=$(declare -f run_arm)
   395	    python3 - "$run_arm_source" <<'PY' || die "run_arm post-client ordering changed"
   396	import sys
   397
   398	source = sys.argv[1]
   399	markers = (
   400	    'dest=$(arm_destination_path "$direction" "$role")',
   401	    'dest_arg=$(arm_destination_argument "$direction" "$role")',
   402	    "read -r result_tag transfer_ms rc client_done_ns result_extra",
   403	    'settle_deadline_ns=$((client_done_ns + SETTLE_NS))',
   404	    'record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after',
   405	    'settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")',
   406	    'flush_out=$(flush_verify_q "$dest")',
   407	    'flush_out=$(flush_verify_win "$dest")',
   408	    'arm_phase=durability_verified',
   409	    'fetch_successful_windows_client_log "$arm_phase" "$remote_err" "$werr"',
   410	    'total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))',
   411	)
   412	positions = []
   413	for marker in markers:
   414	    try:
   415	        positions.append(source.index(marker))
   416	    except ValueError as exc:
   417	        raise SystemExit(f"missing run_arm ordering marker: {marker}") from exc
   418	if positions != sorted(positions):
   419	    raise SystemExit(f"run_arm ordering markers out of order: {positions}")
   420	for forbidden in (
   421	    'q_destination_path "$rid"',
   422	    'win_destination_path "$rid"',
   423	    '$SESSION_TAG/$rid/container',
   424	    'client_done_ns=$(q_monotonic_ns)',
   425	):
   426	    if forbidden in source:
   427	        raise SystemExit(f"forbidden run_arm pattern returned: {forbidden}")
   428	PY
   429
   430	    python3 - "$(declare -f q_client_run)" "$(declare -f win_client_run)" <<'PY' \
   431	        || die "client completion-anchor contract changed"
   432	import sys
   433
   434	q_client, win_client = sys.argv[1:]
   435	q_markers = (
   436	    "clock_ns = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC)",
   437	    "p=subprocess.run(",
   438	    "done_ns=clock_ns()",
   439	    'print(f"R|{ms}|{p.returncode}|{done_ns}")',
   440	)
   441	q_positions = [q_client.index(marker) for marker in q_markers]
   442	if q_positions != sorted(q_positions):
   443	    raise SystemExit(f"q completion markers out of order: {q_positions}")
   444	for marker in (
   445	    "[Console]::Out.WriteLine(",
   446	    "[Console]::Out.Flush()",
   447	    "| stamp_result_arrival_on_q",
   448	):
   449	    if marker not in win_client:
   450	        raise SystemExit(f"missing streamed Windows completion marker: {marker}")
   451	PY
   452
   453	    win_stop_source=$(declare -f win_daemon_stop)
   454	    win_start_source=$(declare -f win_daemon_start)
   455	    python3 - "$win_stop_source" "$win_start_source" <<'PY' \
   456	        || die "Windows launcher/daemon identity contract changed"
   457	import sys
   458
   459	stop, start = sys.argv[1:]
   460	try:
   461	    recovery, remote_stop = stop.split(r"\$pid0 = if", 1)
   462	except ValueError as exc:
   463	    raise SystemExit("Windows stop script lost its recovery boundary") from exc
   464	recovery_markers = (
   465	    r"if (-not \$c)",
   466	    r"\$actual -ieq \$expectedLauncher",
   467	    r"\$launchers.Count -gt 1",
   468	    r"if (-not \$d -and \$c -match '^[0-9]+$')",
   469	    r"\$_.ParentProcessId -eq [int]\$c",
   470	    r"\$children.Count -gt 1",
   471	    r'\"P|\$c|\$d\"',
   472	)
   473	try:
   474	    recovery_positions = [recovery.index(marker) for marker in recovery_markers]
   475	    empty_pid_branch = recovery.index('if [[ -z "$pid" && -z "$cmdpid" ]]')
   476	except ValueError as exc:
   477	    raise SystemExit(f"missing pre-PID-file recovery marker: {exc}") from exc
   478	if recovery_positions != sorted(recovery_positions) or recovery_positions[-1] >= empty_pid_branch:
   479	    raise SystemExit("empty-PID return can bypass exact Windows process discovery")
   480	for marker in (
   481	    r"elseif (\$cmd0)",
   482	    r"\$_.ParentProcessId -eq \$cmd0",
   483	    r"\$children.Count -gt 1",
   484	):
   485	    if marker not in remote_stop:
   486	        raise SystemExit(f"missing parent-based daemon recovery marker: {marker}")
   487	stop_markers = (
   488	    r"\$d.ParentProcessId -ne \$cmd0",
   489	    r"\$c.Name -ine 'cmd.exe'",
   490	    r"\$actualLauncher -ine \$expectedLauncher",
   491	    r"Stop-Process -Id \$stoppedDaemonPid",
   492	    r"Stop-Process -Id \$cmd0",
   493	)
   494	try:
   495	    positions = [remote_stop.index(marker) for marker in stop_markers]
   496	except ValueError as exc:
   497	    raise SystemExit(f"missing exact stop identity marker: {exc}") from exc
   498	if max(positions[:3]) >= min(positions[3:]):
   499	    raise SystemExit("a Windows process can be stopped before all identities validate")
   500	late_markers = (
   501	    r"Stop-Process -Id \$cmd0",
   502	    r"\$actualLate -ine '$WIN_ACTIVE'",
   503	    r"Stop-Process -Id \$late.ProcessId",
   504	    "late daemon child survived teardown",
   505	)
   506	late_positions = [remote_stop.index(marker) for marker in late_markers]
   507	if late_positions != sorted(late_positions):
   508	    raise SystemExit(f"late Windows child recovery is out of order: {late_positions}")
   509
   510	try:
   511	    generated_start, start_controller = start.split(r"\$launcherCommand =", 1)
   512	except ValueError as exc:
   513	    raise SystemExit("Windows start script lost its controller boundary") from exc
   514	batch_markers = (
   515	    "':wait_for_launch_ok'",
   516	    "launch.ok\\\" goto launch_ready",
   517	    "BLIT_LAUNCH_WAIT% GEQ 15 exit /b 111",
   518	    "'goto wait_for_launch_ok'",
   519	    "':launch_ready'",
   520	    r"'\"$WIN_ACTIVE\" --config",
   521	)
   522	batch_positions = [generated_start.index(marker) for marker in batch_markers]
   523	if batch_positions != sorted(batch_positions):
   524	    raise SystemExit(f"bounded Windows launch gate is out of order: {batch_positions}")
   525	controller_markers = (
   526	    "Invoke-CimMethod",
   527	    "launcher.pid.tmp' -Value",
   528	    "Move-Item -LiteralPath '$WIN_SESSION/block_$block/launcher.pid.tmp'",
   529	    r"\$persistedLauncher = (Get-Content",
   530	    r"\$persistedLauncher -ne [string]\$r.ProcessId",
   531	    "New-Item -ItemType File -Path '$WIN_SESSION/block_$block/launch.ok'",
   532	    "Start-Sleep -Seconds 2",
   533	)
   534	controller_positions = [start_controller.index(marker) for marker in controller_markers]
   535	if controller_positions != sorted(controller_positions):
   536	    raise SystemExit(f"Windows PID journal does not precede launch gate: {controller_positions}")
   537	for marker in (
   538	    r"\$actualLauncher -ine \$launcherCommand",
   539	    r"\$actualDaemon -ine '$WIN_ACTIVE'",
   540	    r"\$d.ParentProcessId -ne \$r.ProcessId",
   541	):
   542	    if marker not in start:
   543	        raise SystemExit(f"missing start identity marker: {marker}")
   544	PY
   545
   546	    win_recovery_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-win-recovery.XXXXXX")
   547	    (
   548	        current_block=launcher-smoke
   549	        win_daemon_pid=""
   550	        win_cmd_pid=""
   551	        wssh() {
   552	            local script="$1"
   553	            if [[ "$script" == *'$pid0 = if'* ]]; then
   554	                printf 'stop\n' >> "$win_recovery_tmp/calls"
   555	                printf 'STOPPED\n'
   556	            elif [[ "$script" == *'multiple exact launchers match'* ]]; then
   557	                printf 'recover\n' >> "$win_recovery_tmp/calls"
   558	                printf 'P|22|\n'
   559	            else
   560	                die "cmd-only Windows recovery fell into the empty-PID port branch"
   561	            fi
   562	        }
   563	        win_daemon_stop || die "cmd-only Windows recovery was rejected"
   564	        [[ -z "$win_daemon_pid" && -z "$win_cmd_pid" ]] \
   565	            || die "cmd-only Windows recovery retained remembered PIDs"
   566	        [[ "$(< "$win_recovery_tmp/calls")" == $'recover\nstop' ]] \
   567	            || die "cmd-only Windows recovery skipped exact stop"
   568	    )
   569	    rm -rf "$win_recovery_tmp"
   570
   571	    launcher_source=$(declare -f launcher_smoke)
   572	    main_source=$(declare -f main)
   573	    python3 - "$launcher_source" "$main_source" <<'PY' \
   574	        || die "standalone launcher-smoke control flow changed"
   575	import sys
   576
   577	smoke, main = sys.argv[1:]
   578	smoke_markers = (
   579	    "WIN_SESSION_MAY_EXIST=1",
   580	    "current_block=launcher-smoke",
   581	    "ports_closed",
   582	    'win_daemon_start "$current_block" off "$run_id"',
   583	    'nc -z -w 3 "$WIN_IP" "$PORT"',
   584	    'stop_daemons "$current_block"',
   585	    'current_block=""',
   586	    "strict_success_cleanup || session_void",
   587	)
   588	positions = []
   589	for marker in smoke_markers:
   590	    try:
   591	        positions.append(smoke.index(marker))
   592	    except ValueError as exc:
   593	        raise SystemExit(f"missing launcher-smoke marker: {marker}") from exc
   594	if positions != sorted(positions):
   595	    raise SystemExit(f"launcher-smoke markers out of order: {positions}")
   596	for forbidden in (
   597	    "REGISTERED_RUN_STARTED",
   598	    "SESSION_FINALIZED",
   599	    "SESSION-COMPLETE",
   600	    "q_daemon_start",
   601	    "start_daemons",
   602	    "run_arm",
   603	    "run_block",
   604	    "RUNS_CSV",
   605	    "CLOCK_CSV",
   606	    "emit_schedule",
   607	    "schedule.csv",
   608	    "otp12pf_rigw_analyze.py",
   609	    "finalize_registered_session",
   610	    "LOCAL_EVIDENCE_COMPLETE",
   611	    "Q_SESSION_MAY_EXIST",
   612	):
   613	    if forbidden in smoke:
   614	        raise SystemExit(f"launcher smoke reached registered work: {forbidden}")
   615	mode_check = main.index("validate_mode_selection")
   616	preflight = main.index("preflight;")
   617	branch_start = main.index('if [[ "$LAUNCHER_SMOKE" == 1 ]]')
   618	registered = main.index("REGISTERED_RUN_STARTED=1", branch_start)
   619	if not mode_check < preflight < branch_start < registered:
   620	    raise SystemExit("main launcher-smoke gate moved around preflight or registration")

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '921,1267p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   921	        ports_closed() { port_checks=$((port_checks + 1)); return 0; }
   922	        wssh() { return 0; }
   923	        strict_success_cleanup || die "strict cleanup rejected a clean session"
   924	        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
   925	            || die "successful strict cleanup did not set verification state"
   926	        [[ "$port_checks" == 2 ]] || die "strict cleanup ran $port_checks port checks"
   927	        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]] \
   928	            || die "successful strict cleanup retained may-exist state"
   929	        [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
   930	    )
   931	    mkdir -p "$cleanup_tmp/q/rigw-sessions/late-port"
   932	    (
   933	        Q_MODULE="$cleanup_tmp/q"
   934	        SESSION_TAG=late-port
   935	        Q_SESSION_MAY_EXIST=1
   936	        WIN_SESSION_MAY_EXIST=1
   937	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   938	        port_checks=0
   939	        ports_closed() {
   940	            port_checks=$((port_checks + 1))
   941	            [[ "$port_checks" == 1 ]]
   942	        }
   943	        wssh() { return 0; }
   944	        if strict_success_cleanup; then
   945	            die "strict cleanup accepted a listener appearing during deletion"
   946	        fi
   947	        [[ "$STRICT_CLEANUP_VERIFIED" == 0 && "$port_checks" == 2 ]]
   948	    )
   949	    for remembered in q daemon launcher block; do
   950	        (
   951	            Q_MODULE="$cleanup_tmp/q"
   952	            SESSION_TAG="remembered-$remembered"
   953	            q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
   954	            case "$remembered" in
   955	                q) q_daemon_pid=11;;
   956	                daemon) win_daemon_pid=22;;
   957	                launcher) win_cmd_pid=33;;
   958	                block) current_block=4;;
   959	            esac
   960	            ports_closed() { die "strict cleanup ignored remembered $remembered state"; }
   961	            if strict_success_cleanup; then
   962	                die "strict cleanup accepted remembered $remembered state"
   963	            fi
   964	            [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
   965	        )
   966	    done
   967	    strict_cleanup_source=$(declare -f strict_success_cleanup)
   968	    python3 - "$strict_cleanup_source" <<'PY' \
   969	        || die "strict cleanup source contract changed"
   970	import sys
   971
   972	source = sys.argv[1]
   973	for marker in (
   974	    "'$WIN_MODULE/rigw-sessions/$SESSION_TAG'",
   975	    "'$WIN_SESSION'",
   976	    r"Remove-Item -LiteralPath \$path -Recurse -Force -ErrorAction Stop",
   977	    r'if (Test-Path -LiteralPath \$path) { throw',
   978	):
   979	    if marker not in source:
   980	        raise SystemExit(f"missing strict Windows cleanup marker: {marker}")
   981	if source.count('ports_closed') != 2:
   982	    raise SystemExit("strict cleanup must check closed ports before and after deletion")
   983	if source.index('ports_closed') > source.index('Remove-Item -LiteralPath'):
   984	    raise SystemExit("strict cleanup deletes evidence before its first port check")
   985	if source.rindex('ports_closed') < source.index('rm -rf --'):
   986	    raise SystemExit("strict cleanup lacks a post-deletion port check")
   987	PY
   988	    rm -rf "$finalize_tmp"
   989
   990	    failure_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-failure.XXXXXX")
   991	    trap_calls="$failure_tmp/remote-calls"
   992	    mkdir -p "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG"
   993	    printf 'retain me\n' > "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel"
   994	    set +e
   995	    (
   996	        set +e
   997	        OUT_DIR="$failure_tmp/evidence"
   998	        mkdir "$OUT_DIR"
   999	        LOG="$OUT_DIR/bench.log"
  1000	        OUTPUT_CLAIMED=1
  1001	        printf 'primary failure\n' > "$OUT_DIR/SESSION-VOID"
  1002	        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE"
  1003	        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE.tmp"
  1004	        REGISTERED_RUN_STARTED=1
  1005	        SESSION_FINALIZED=0
  1006	        STRICT_CLEANUP_VERIFIED=0
  1007	        Q_SESSION_MAY_EXIST=1
  1008	        WIN_SESSION_MAY_EXIST=1
  1009	        Q_MODULE="$failure_tmp/q-module"
  1010	        current_block=1
  1011	        q_daemon_pid=""
  1012	        win_daemon_pid=""
  1013	        win_cmd_pid=""
  1014	        wssh() {
  1015	            printf '%s\n' "$*" >> "$trap_calls"
  1016	            return 1
  1017	        }
  1018	        false
  1019	        on_exit
  1020	    )
  1021	    trap_rc=$?
  1022	    set -e
  1023	    [[ "$trap_rc" == 1 ]] || die "failure trap returned $trap_rc, expected 1"
  1024	    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]] \
  1025	        || die "failure trap left SESSION-COMPLETE"
  1026	    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]] \
  1027	        || die "failure trap left SESSION-COMPLETE.tmp"
  1028	    grep -Fxq 'primary failure' "$failure_tmp/evidence/SESSION-VOID" \
  1029	        || die "failure trap discarded the primary reason"
  1030	    grep -Fq 'cleanup errors: Windows PID recovery failed' "$failure_tmp/evidence/SESSION-VOID" \
  1031	        || die "failure trap omitted its cleanup error"
  1032	    grep -Fq "q session evidence may remain; inspect $failure_tmp/q-module/rigw-sessions/$SESSION_TAG" \
  1033	        "$failure_tmp/evidence/SESSION-VOID" \
  1034	        || die "failure trap omitted the q evidence path"
  1035	    grep -Fq "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG" \
  1036	        "$failure_tmp/evidence/SESSION-VOID" \
  1037	        || die "failure trap omitted the Windows evidence path"
  1038	    if grep -Fq 'Remove-Item' "$trap_calls"; then
  1039	        die "failure trap issued destructive remote cleanup"
  1040	    fi
  1041	    [[ "$(< "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
  1042	        || die "failure trap modified q session evidence"
  1043	    on_exit_source=$(declare -f on_exit)
  1044	    if [[ "$on_exit_source" == *'rm -rf'* \
  1045	        || "$on_exit_source" == *'Remove-Item'* \
  1046	        || "$on_exit_source" == *'strict_success_cleanup'* ]]; then
  1047	        die "failure trap contains a destructive session-cleanup path"
  1048	    fi
  1049
  1050	    append_tmp="$failure_tmp/append-contract"
  1051	    mkdir "$append_tmp"
  1052	    printf 'original reason\n' > "$append_tmp/SESSION-VOID"
  1053	    set +e
  1054	    (
  1055	        OUT_DIR="$append_tmp"
  1056	        LOG="$OUT_DIR/bench.log"
  1057	        OUTPUT_CLAIMED=1
  1058	        session_void 'later context'
  1059	    ) >/dev/null 2>&1
  1060	    trap_rc=$?
  1061	    set -e
  1062	    [[ "$trap_rc" == 1 ]] || die "session_void append probe returned $trap_rc"
  1063	    [[ "$(< "$append_tmp/SESSION-VOID")" == $'original reason\nlater context' ]] \
  1064	        || die "session_void overwrote an earlier failure reason"
  1065
  1066	    contract_tmp="$failure_tmp/exit-contract"
  1067	    mkdir "$contract_tmp"
  1068	    set +e
  1069	    (
  1070	        set +e
  1071	        OUT_DIR="$contract_tmp"
  1072	        LOG="$OUT_DIR/bench.log"
  1073	        OUTPUT_CLAIMED=1
  1074	        REGISTERED_RUN_STARTED=1
  1075	        SESSION_FINALIZED=0
  1076	        STRICT_CLEANUP_VERIFIED=0
  1077	        WIN_SESSION_MAY_EXIST=0
  1078	        true
  1079	        on_exit
  1080	    )
  1081	    trap_rc=$?
  1082	    set -e
  1083	    [[ "$trap_rc" == 1 ]] \
  1084	        || die "unfinalized registered zero-exit returned $trap_rc"
  1085	    grep -Fq 'registered run returned without finalizing the session' \
  1086	        "$contract_tmp/SESSION-VOID" \
  1087	        || die "unfinalized registered zero-exit omitted its reason"
  1088
  1089	    contract_tmp="$failure_tmp/marker-contract"
  1090	    mkdir "$contract_tmp"
  1091	    set +e
  1092	    (
  1093	        set +e
  1094	        OUT_DIR="$contract_tmp"
  1095	        LOG="$OUT_DIR/bench.log"
  1096	        OUTPUT_CLAIMED=1
  1097	        REGISTERED_RUN_STARTED=1
  1098	        SESSION_FINALIZED=1
  1099	        STRICT_CLEANUP_VERIFIED=1
  1100	        LOCAL_EVIDENCE_COMPLETE=1
  1101	        HEAD_FULL=0123456789abcdef
  1102	        true
  1103	        on_exit
  1104	    )
  1105	    trap_rc=$?
  1106	    set -e
  1107	    [[ "$trap_rc" == 1 ]] \
  1108	        || die "finalized flags without a completion marker returned $trap_rc"
  1109	    grep -Fq 'registered completion marker is absent or invalid' \
  1110	        "$contract_tmp/SESSION-VOID" \
  1111	        || die "missing registered completion marker omitted its reason"
  1112
  1113	    contract_tmp="$failure_tmp/wrong-marker-contract"
  1114	    mkdir "$contract_tmp"
  1115	    printf 'wrong-build\n' > "$contract_tmp/SESSION-COMPLETE"
  1116	    set +e
  1117	    (
  1118	        set +e
  1119	        OUT_DIR="$contract_tmp"
  1120	        LOG="$OUT_DIR/bench.log"
  1121	        OUTPUT_CLAIMED=1
  1122	        REGISTERED_RUN_STARTED=1
  1123	        SESSION_FINALIZED=1
  1124	        STRICT_CLEANUP_VERIFIED=1
  1125	        LOCAL_EVIDENCE_COMPLETE=1
  1126	        HEAD_FULL=0123456789abcdef
  1127	        true
  1128	        on_exit
  1129	    )
  1130	    trap_rc=$?
  1131	    set -e
  1132	    [[ "$trap_rc" == 1 ]] \
  1133	        || die "wrong completion marker returned $trap_rc"
  1134	    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
  1135	        || die "wrong completion marker survived failure handling"
  1136
  1137	    contract_tmp="$failure_tmp/preflight-contract"
  1138	    mkdir "$contract_tmp"
  1139	    set +e
  1140	    (
  1141	        set +e
  1142	        OUT_DIR="$contract_tmp"
  1143	        LOG="$OUT_DIR/bench.log"
  1144	        OUTPUT_CLAIMED=1
  1145	        REGISTERED_RUN_STARTED=0
  1146	        SESSION_FINALIZED=0
  1147	        STRICT_CLEANUP_VERIFIED=0
  1148	        true
  1149	        on_exit
  1150	    )
  1151	    trap_rc=$?
  1152	    set -e
  1153	    [[ "$trap_rc" == 1 ]] \
  1154	        || die "unclean preflight zero-exit returned $trap_rc"
  1155	    grep -Fq 'successful exit lacked verified strict cleanup' \
  1156	        "$contract_tmp/SESSION-VOID" \
  1157	        || die "unclean preflight zero-exit omitted its reason"
  1158
  1159	    contract_tmp="$failure_tmp/preflight-marker-contract"
  1160	    mkdir "$contract_tmp"
  1161	    printf 'not allowed\n' > "$contract_tmp/SESSION-COMPLETE"
  1162	    set +e
  1163	    (
  1164	        set +e
  1165	        OUT_DIR="$contract_tmp"
  1166	        LOG="$OUT_DIR/bench.log"
  1167	        OUTPUT_CLAIMED=1
  1168	        REGISTERED_RUN_STARTED=0
  1169	        SESSION_FINALIZED=0
  1170	        STRICT_CLEANUP_VERIFIED=1
  1171	        true
  1172	        on_exit
  1173	    )
  1174	    trap_rc=$?
  1175	    set -e
  1176	    [[ "$trap_rc" == 1 ]] \
  1177	        || die "preflight completion marker returned $trap_rc"
  1178	    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
  1179	        || die "preflight completion marker survived failure handling"
  1180
  1181	    for marker in SESSION-VOID SESSION-COMPLETE.tmp; do
  1182	        contract_tmp="$failure_tmp/preflight-$marker-contract"
  1183	        mkdir "$contract_tmp"
  1184	        printf 'not allowed\n' > "$contract_tmp/$marker"
  1185	        set +e
  1186	        (
  1187	            set +e
  1188	            OUT_DIR="$contract_tmp"
  1189	            LOG="$OUT_DIR/bench.log"
  1190	            OUTPUT_CLAIMED=1
  1191	            REGISTERED_RUN_STARTED=0
  1192	            SESSION_FINALIZED=0
  1193	            STRICT_CLEANUP_VERIFIED=1
  1194	            true
  1195	            on_exit
  1196	        )
  1197	        trap_rc=$?
  1198	        set -e
  1199	        [[ "$trap_rc" == 1 ]] \
  1200	            || die "preflight $marker returned $trap_rc"
  1201	        if [[ "$marker" == SESSION-VOID ]]; then
  1202	            [[ "$(sed -n '1p' "$contract_tmp/SESSION-VOID")" == 'not allowed' ]] \
  1203	                || die "preflight VOID rejection replaced its primary reason"
  1204	        else
  1205	            grep -Fq 'successful exit retained a failure or temporary marker' \
  1206	                "$contract_tmp/SESSION-VOID" \
  1207	                || die "preflight $marker omitted its rejection reason"
  1208	        fi
  1209	    done
  1210
  1211	    for signal in HUP INT TERM; do
  1212	        signal_dir="$failure_tmp/signal-$signal"
  1213	        mkdir "$signal_dir"
  1214	        set +e
  1215	        bash -c '
  1216	set -Eeuo pipefail
  1217	source "$1"
  1218	OUT_DIR="$2"
  1219	LOG="$OUT_DIR/bench.log"
  1220	OUTPUT_CLAIMED=1
  1221	REGISTERED_RUN_STARTED=1
  1222	SESSION_FINALIZED=0
  1223	STRICT_CLEANUP_VERIFIED=0
  1224	Q_SESSION_MAY_EXIST=1
  1225	WIN_SESSION_MAY_EXIST=1
  1226	current_block=1
  1227	q_daemon_pid=111
  1228	win_daemon_pid=222
  1229	win_cmd_pid=333
  1230	win_daemon_stop() {
  1231	    printf "windows\n" >> "$OUT_DIR/stops"
  1232	    win_daemon_pid=""; win_cmd_pid=""; current_block=""
  1233	}
  1234	q_daemon_stop() {
  1235	    printf "q\n" >> "$OUT_DIR/stops"
  1236	    q_daemon_pid=""
  1237	}
  1238	printf "must disappear\n" > "$OUT_DIR/SESSION-COMPLETE"
  1239	trap on_exit EXIT
  1240	install_signal_traps
  1241	kill -s "$3" "$$"
  1242	sleep 2
  1243	exit 99
  1244	' _ "$SCRIPT_DIR/bench_otp12pf_rigw.sh" "$signal_dir" "$signal"
  1245	        signal_rc=$?
  1246	        set -e
  1247	        [[ "$signal_rc" == 1 ]] \
  1248	            || die "$signal cleanup returned $signal_rc, expected 1"
  1249	        grep -Fxq "received $signal" "$signal_dir/SESSION-VOID" \
  1250	            || die "$signal cleanup omitted its signal reason"
  1251	        [[ "$(LC_ALL=C sort "$signal_dir/stops")" == $'q\nwindows' ]] \
  1252	            || die "$signal cleanup did not invoke both exact-owned teardown paths"
  1253	        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
  1254	            || die "$signal cleanup left SESSION-COMPLETE"
  1255	    done
  1256	    rm -rf "$failure_tmp"
  1257
  1258	    analyzer_log=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-analyzer.XXXXXX")
  1259	    if ! python3 "$SCRIPT_DIR/otp12pf_rigw_analyze_test.py" \
  1260	        > "$analyzer_log" 2>&1; then
  1261	        cat "$analyzer_log" >&2
  1262	        rm -f "$analyzer_log"
  1263	        die "analyzer self-tests failed"
  1264	    fi
  1265	    rm -f "$analyzer_log"
  1266	    log "SELFTEST OK: exact four-block/128-arm schedule and analyzer guards"
  1267	}

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '621,816p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   621	branch = main[branch_start:registered]
   622	branch_markers = ('if [[ "$LAUNCHER_SMOKE" == 1 ]]', "launcher_smoke;", "return;", "fi;")
   623	branch_positions = [branch.index(marker) for marker in branch_markers]
   624	if branch_positions != sorted(branch_positions) or branch.count("return;") != 1:
   625	    raise SystemExit(f"launcher-smoke branch can fall through: {branch_positions}")
   626	PY
   627
   628	    launcher_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-launcher-smoke.XXXXXX")
   629	    launcher_calls="$launcher_tmp/calls"
   630	    (
   631	        OUT_DIR="$launcher_tmp/evidence"
   632	        mkdir "$OUT_DIR"
   633	        LOG="$OUT_DIR/bench.log"
   634	        OUTPUT_CLAIMED=1
   635	        SESSION_TAG=offline-smoke
   636	        REGISTERED_RUN_STARTED=0
   637	        SESSION_FINALIZED=0
   638	        STRICT_CLEANUP_VERIFIED=0
   639	        Q_SESSION_MAY_EXIST=0
   640	        WIN_SESSION_MAY_EXIST=0
   641	        LOCAL_EVIDENCE_COMPLETE=0
   642	        current_block=""
   643	        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""
   644	        port_checks=0
   645	        log() { :; }
   646	        win_daemon_start() {
   647	            [[ "$1" == launcher-smoke && "$2" == off \
   648	                && "$3" == offline-smoke-launcher-smoke \
   649	                && "$WIN_SESSION_MAY_EXIST" == 1 \
   650	                && "$current_block" == launcher-smoke ]] \
   651	                || die "offline launcher smoke started with wrong identity"
   652	            printf 'start\n' >> "$launcher_calls"
   653	            win_cmd_pid=22; win_daemon_pid=33
   654	        }
   655	        nc() {
   656	            [[ "$*" == "-z -w 3 $WIN_IP $PORT" \
   657	                && "$win_cmd_pid" == 22 && "$win_daemon_pid" == 33 ]] \
   658	                || die "offline launcher smoke reachability ran out of order"
   659	            printf 'reach\n' >> "$launcher_calls"
   660	        }
   661	        win_daemon_stop() {
   662	            [[ "$current_block" == launcher-smoke \
   663	                && "$win_cmd_pid" == 22 && "$win_daemon_pid" == 33 ]] \
   664	                || die "offline launcher smoke stopped the wrong daemon"
   665	            printf 'stop\n' >> "$launcher_calls"
   666	            win_cmd_pid=""; win_daemon_pid=""
   667	        }
   668	        q_daemon_stop() {
   669	            [[ -z "$q_daemon_pid" ]] \
   670	                || die "offline launcher smoke unexpectedly owned a q daemon"
   671	            printf 'q-stop-empty\n' >> "$launcher_calls"
   672	        }
   673	        collect_block_logs() {
   674	            [[ "$1" == launcher-smoke && -z "$win_cmd_pid" \
   675	                && -z "$win_daemon_pid" ]] \
   676	                || die "offline launcher smoke collected before exact stop"
   677	            printf 'collect\n' >> "$launcher_calls"
   678	        }
   679	        ports_closed() {
   680	            port_checks=$((port_checks + 1))
   681	            if [[ "$port_checks" == 1 ]]; then
   682	                [[ "$current_block" == launcher-smoke \
   683	                    && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   684	                    || die "offline launcher smoke skipped its pre-start port check"
   685	                printf 'closed-pre\n' >> "$launcher_calls"
   686	            else
   687	                [[ "$port_checks" == 2 && "$current_block" == launcher-smoke \
   688	                    && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   689	                    || die "offline launcher smoke checked ports before exact stop"
   690	                printf 'closed-post\n' >> "$launcher_calls"
   691	            fi
   692	        }
   693	        strict_success_cleanup() {
   694	            [[ "$WIN_SESSION_MAY_EXIST" == 1 && -z "$current_block" \
   695	                && -z "$q_daemon_pid" && -z "$win_cmd_pid" && -z "$win_daemon_pid" ]] \
   696	                || die "offline launcher smoke cleaned before exact stop"
   697	            printf 'cleanup\n' >> "$launcher_calls"
   698	            WIN_SESSION_MAY_EXIST=0
   699	            STRICT_CLEANUP_VERIFIED=1
   700	        }
   701	        launcher_smoke
   702	        [[ "$(< "$launcher_calls")" == \
   703	            $'closed-pre\nstart\nreach\nstop\nq-stop-empty\ncollect\nclosed-post\ncleanup' ]] \
   704	            || die "offline launcher-smoke call order changed"
   705	        [[ "$REGISTERED_RUN_STARTED" == 0 && "$SESSION_FINALIZED" == 0 \
   706	            && "$STRICT_CLEANUP_VERIFIED" == 1 \
   707	            && "$Q_SESSION_MAY_EXIST" == 0 \
   708	            && "$WIN_SESSION_MAY_EXIST" == 0 \
   709	            && "$LOCAL_EVIDENCE_COMPLETE" == 0 ]] \
   710	            || die "offline launcher smoke changed registered state"
   711	        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" \
   712	            && ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
   713	            && ! -e "$OUT_DIR/SESSION-VOID" ]] \
   714	            || die "offline launcher smoke left a session marker"
   715	    )
   716	    rm -rf "$launcher_tmp"
   717
   718	    HEAD_BUILD_ID=0123456789ab
   719	    identity_file=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-identity.XXXXXX")
   720	    printf 'blit+%s\0' "$HEAD_BUILD_ID" > "$identity_file"
   721	    embeds_clean_q "$identity_file" || die "clean 12-character build identity was rejected"
   722	    printf 'blit+%s.dirty.ffffffffffff\0' "$HEAD_BUILD_ID" > "$identity_file"
   723	    if embeds_clean_q "$identity_file"; then
   724	        rm -f "$identity_file"
   725	        die "dirty build identity was accepted"
   726	    fi
   727	    rm -f "$identity_file"
   728
   729	    manifest_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-manifest.XXXXXX")
   730	    mkdir -p "$manifest_tmp/source/sub" "$manifest_tmp/container/src_mixed/sub"
   731	    printf 'a' > "$manifest_tmp/source/a"
   732	    printf 'bc' > "$manifest_tmp/source/sub/b"
   733	    printf 'a' > "$manifest_tmp/container/src_mixed/a"
   734	    printf 'bc' > "$manifest_tmp/container/src_mixed/sub/b"
   735	    canonical_manifest="$manifest_tmp/canonical.manifest"
   736	    landed_manifest="$manifest_tmp/landed.manifest"
   737	    write_q_tree_manifest "$manifest_tmp/source" "$canonical_manifest"
   738	    write_q_tree_manifest \
   739	        "$manifest_tmp/container" "$landed_manifest" src_mixed
   740	    tree_digest=$(matching_manifest_digest "$canonical_manifest" "$landed_manifest") \
   741	        || die "identical relative-path/size manifests did not match"
   742	    [[ "$tree_digest" =~ ^[0-9a-f]{64}$ ]] \
   743	        || die "tree manifest digest is malformed"
   744	    printf 'aa' > "$manifest_tmp/container/src_mixed/a"
   745	    printf 'b' > "$manifest_tmp/container/src_mixed/sub/b"
   746	    write_q_tree_manifest \
   747	        "$manifest_tmp/container" "$landed_manifest" src_mixed
   748	    if matching_manifest_digest "$canonical_manifest" "$landed_manifest" >/dev/null; then
   749	        rm -rf "$manifest_tmp"
   750	        die "same-count/same-byte tree with swapped file sizes was accepted"
   751	    fi
   752	    rm -rf "$manifest_tmp/container/src_mixed"
   753	    mkdir -p "$manifest_tmp/container/wrapper/src_mixed"
   754	    if write_q_tree_manifest \
   755	        "$manifest_tmp/container" "$landed_manifest" src_mixed 2>/dev/null; then
   756	        rm -rf "$manifest_tmp"
   757	        die "wrong landed root wrapper was accepted"
   758	    fi
   759	    rm -rf "$manifest_tmp"
   760
   761	    freshness_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-freshness.XXXXXX")
   762	    reserve_evidence_dir "$freshness_tmp/new-evidence" \
   763	        || die "fresh evidence directory was rejected: $OUTPUT_CLAIM_ERROR"
   764	    for marker in SESSION-COMPLETE SESSION-VOID unrelated.txt; do
   765	        freshness_case="$freshness_tmp/$marker"
   766	        mkdir "$freshness_case"
   767	        printf 'preserve-me\n' > "$freshness_case/$marker"
   768	        before=$(sha256_q "$freshness_case/$marker")
   769	        if reserve_evidence_dir "$freshness_case"; then
   770	            rm -rf "$freshness_tmp"
   771	            die "stale output directory containing $marker was accepted"
   772	        fi
   773	        [[ "$(sha256_q "$freshness_case/$marker")" == "$before" ]] \
   774	            || die "stale output rejection modified $marker"
   775	    done
   776	    rm -rf "$freshness_tmp"
   777
   778	    destination_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-destination.XXXXXX")
   779	    mkdir -p "$destination_tmp/container/src_mixed"
   780	    printf 'stale\n' > "$destination_tmp/container/src_mixed/stale"
   781	    (
   782	        rm() { return 73; }
   783	        if prepare_destination wm "$destination_tmp/container"; then
   784	            die "q destination reset masked a failed removal"
   785	        fi
   786	    )
   787	    [[ "$(< "$destination_tmp/container/src_mixed/stale")" == stale ]] \
   788	        || die "failed q destination reset modified retained evidence"
   789	    prepare_destination wm "$destination_tmp/container" \
   790	        || die "q destination reset rejected a removable tree"
   791	    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]] \
   792	        || die "q destination reset did not leave a plain directory"
   793	    [[ -z "$(find "$destination_tmp/container" -mindepth 1 -maxdepth 1 -print -quit)" ]] \
   794	        || die "q destination reset left stale content"
   795	    rm -rf "$destination_tmp"
   796
   797	    prepare_destination_source=$(declare -f prepare_destination)
   798	    python3 - "$prepare_destination_source" <<'PY' \
   799	        || die "Windows destination reset source contract changed"
   800	import sys
   801
   802	source = sys.argv[1]
   803	for marker in (
   804	    r"\$ErrorActionPreference = 'Stop'",
   805	    r"Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop",
   806	    r"Test-Path -LiteralPath '$dest' -PathType Container",
   807	    r"Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop",
   808	    'ReparsePoint',
   809	):
   810	    if marker not in source:
   811	        raise SystemExit(f"missing Windows destination reset marker: {marker}")
   812	windows = source.split('else', 1)[1]
   813	if 'SilentlyContinue' in windows:
   814	    raise SystemExit("Windows destination reset suppresses removal errors")
   815	PY
   816

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '1,370p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '371,729p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '1081,1408p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '730,1080p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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

 succeeded in 0ms:
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

 succeeded in 0ms:
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

 succeeded in 0ms:
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

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '730,990p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '484,729p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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

 succeeded in 0ms:
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

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '1409,1758p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze.py | sed -n '1759,2020p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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

 succeeded in 0ms:
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

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '500,705p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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

exec
/bin/zsh -lc "rg -n '"'^    def test_'"' scripts/otp12pf_rigw_analyze_test.py" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
419:    def test_complete_schedule_exact_floor_bias_and_exports(self) -> None:
481:    def test_registered_schedule_is_pair_outer_with_reverse_block_bases(self) -> None:
510:    def test_missing_trace_endpoint_is_rejected(self) -> None:
526:    def test_trace_off_leak_is_rejected(self) -> None:
537:    def test_grpc_trace_leak_is_rejected(self) -> None:
548:    def test_schedule_mismatch_is_rejected(self) -> None:
556:    def test_settled_ms_schema_and_bounds_are_fail_closed(self) -> None:
575:    def test_corrupt_total_is_rejected(self) -> None:
586:    def test_role_specific_flush_is_included_in_delta_and_floor(self) -> None:
620:    def test_excess_settle_is_charged_without_false_role_delta(self) -> None:
666:    def test_landed_manifest_rejects_swapped_sizes_and_renamed_paths(self) -> None:
698:    def test_landed_root_and_recorded_manifest_digest_are_exact(self) -> None:
715:    def test_sequence_gap_and_missing_terminal_are_rejected(self) -> None:
731:    def test_payload_write_must_precede_source_completion(self) -> None:
758:    def test_socket_action_end_must_precede_trace_attachment(self) -> None:
788:    def test_causal_elapsed_time_cannot_run_backwards(self) -> None:
816:    def test_destination_resize_prerequisites_are_causal(self) -> None:
861:    def test_source_resize_prerequisites_are_causal(self) -> None:
915:    def test_final_resize_settlement_precedes_data_plane_completion(self) -> None:
981:    def test_destination_preparation_action_is_role_correlated(self) -> None:
995:    def test_resize_ramp_requires_all_seven_epochs(self) -> None:
1023:    def test_final_resize_target_and_live_fields_are_exact_on_both_roles(self) -> None:
1053:    def test_clock_inventory_and_midpoint_math_are_fail_closed(self) -> None:

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze_test.py | sed -n '419,714p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   419	    def test_complete_schedule_exact_floor_bias_and_exports(self) -> None:
   420	        temporary, session = self.make_session()
   421	        self.addCleanup(temporary.cleanup)
   422	        result = analyzer.analyze(session.root)
   423	        self.assertEqual(str(result.observer_bias), "20")
   424	        self.assertEqual(str(result.n_resolution), "70")
   425	        with result.summary_csv.open(newline="") as handle:
   426	            rows = {
   427	                (row["cell"], row["trace_state"]): row
   428	                for row in csv.DictReader(handle)
   429	            }
   430	        off = rows[(analyzer.TARGET_CELL, "off")]
   431	        on = rows[(analyzer.TARGET_CELL, "on")]
   432	        self.assertEqual(off["measurand"], "durable_total_ms")
   433	        self.assertEqual(off["delta_ms"], "45")
   434	        self.assertEqual(off["paired_delta_median_ms"], "45")
   435	        self.assertEqual(off["first4_delta_median_ms"], "25")
   436	        self.assertEqual(off["last4_delta_median_ms"], "65")
   437	        self.assertEqual(off["first_last_drift_ms"], "40")
   438	        self.assertEqual(off["odd_even_drift_ms"], "10")
   439	        self.assertEqual(off["source_first_delta_median_ms"], "45")
   440	        self.assertEqual(off["destination_first_delta_median_ms"], "45")
   441	        self.assertEqual(off["role_order_drift_ms"], "0")
   442	        self.assertEqual(off["n_pair_split_ms"], "40")
   443	        self.assertEqual(off["paired_delta_range_ms"], "70")
   444	        self.assertEqual(off["n_pair_ms"], "70")
   445	        self.assertEqual(on["delta_ms"], "25")
   446	        self.assertEqual(on["n_pair_split_ms"], "10")
   447	        self.assertEqual(on["n_pair_ms"], "10")
   448	        self.assertEqual(on["observer_bias_ms"], "20")
   449	        self.assertEqual(on["n_resolution_ms"], "70")
   450	        self.assertTrue(result.summary_md.is_file())
   451	        self.assertTrue(result.distributions_csv.is_file())
   452	        with result.clock_summary_csv.open(newline="") as handle:
   453	            clocks = list(csv.DictReader(handle))
   454	        self.assertEqual(len(clocks), 128)
   455	        self.assertTrue(all(row["before_sample"] == "1" for row in clocks))
   456	        self.assertTrue(all(row["after_sample"] == "1" for row in clocks))
   457	        self.assertTrue(all(row["selected_offset_change_ns"] == "100" for row in clocks))
   458	        with result.phase_events_csv.open(newline="") as handle:
   459	            phase_rows = list(csv.DictReader(handle))
   460	        self.assertEqual(len(phase_rows), len(session.events))
   461	        self.assertTrue(any(row["source_file"].startswith("client/") for row in phase_rows))
   462	        self.assertTrue(any(row["source_file"].startswith("trace/") for row in phase_rows))
   463	        self.assertTrue(
   464	            all(
   465	                row["total_ms"]
   466	                == str(
   467	                    int(row["transfer_ms"])
   468	                    + int(row["settled_ms"])
   469	                    - analyzer.SETTLE_MIN_MS
   470	                    + int(row["flush_ms"])
   471	                )
   472	                for row in phase_rows
   473	            )
   474	        )
   475	        with result.phase_intervals_csv.open(newline="") as handle:
   476	            intervals = list(csv.DictReader(handle))
   477	        self.assertTrue(intervals)
   478	        self.assertTrue(all(int(row["duration_ns"]) >= 0 for row in intervals))
   479	        self.assertTrue(all(row["endpoint_role"] in {"SOURCE", "DESTINATION"} for row in intervals))
   480
   481	    def test_registered_schedule_is_pair_outer_with_reverse_block_bases(self) -> None:
   482	        schedule = analyzer.expected_schedule()
   483
   484	        def cells_for(block_number: int, pair: int) -> list[str]:
   485	            return [
   486	                cell
   487	                for block, cell, scheduled_pair, _role, role_order in schedule
   488	                if block.number == block_number
   489	                and scheduled_pair == pair
   490	                and role_order == 1
   491	            ]
   492
   493	        base = list(analyzer.CELLS)
   494	        reverse = list(reversed(base))
   495	        self.assertEqual(cells_for(1, 1), base)
   496	        self.assertEqual(cells_for(1, 2), reverse)
   497	        self.assertEqual(cells_for(2, 1), reverse)
   498	        self.assertEqual(cells_for(2, 2), base)
   499	        self.assertEqual(cells_for(3, 5), base)
   500	        self.assertEqual(cells_for(4, 5), reverse)
   501	        self.assertEqual(
   502	            [
   503	                role
   504	                for block, _cell, pair, role, role_order in schedule
   505	                if block.number == 1 and role_order == 1 and _cell == base[0]
   506	            ],
   507	            ["source_init", "destination_init", "destination_init", "source_init"],
   508	        )
   509
   510	    def test_missing_trace_endpoint_is_rejected(self) -> None:
   511	        temporary, session = self.make_session()
   512	        self.addCleanup(temporary.cleanup)
   513	        first_session = next(event["session_id"] for event in session.events)
   514	        session.events = [
   515	            event
   516	            for event in session.events
   517	            if not (
   518	                event["session_id"] == first_session
   519	                and event["endpoint_role"] == "DESTINATION"
   520	            )
   521	        ]
   522	        session.write()
   523	        with self.assertRaisesRegex(analyzer.AnalysisError, "missing endpoint role"):
   524	            analyzer.analyze(session.root)
   525
   526	    def test_trace_off_leak_is_rejected(self) -> None:
   527	        temporary, session = self.make_session()
   528	        self.addCleanup(temporary.cleanup)
   529	        leaked = dict(session.events[0])
   530	        leaked["run_id"] = "rigw-block-1"
   531	        leaked["session_id"] = "ffffffffffffffff"
   532	        session.events.append(leaked)
   533	        session.write()
   534	        with self.assertRaisesRegex(analyzer.AnalysisError, "trace leak: trace-off block 1"):
   535	            analyzer.analyze(session.root)
   536
   537	    def test_grpc_trace_leak_is_rejected(self) -> None:
   538	        temporary, session = self.make_session()
   539	        self.addCleanup(temporary.cleanup)
   540	        leaked = dict(session.events[0])
   541	        leaked["run_id"] = "rigw-block-2"
   542	        leaked["session_id"] = "eeeeeeeeeeeeeeee"
   543	        session.events.append(leaked)
   544	        session.write()
   545	        with self.assertRaisesRegex(analyzer.AnalysisError, "possible gRPC"):
   546	            analyzer.analyze(session.root)
   547
   548	    def test_schedule_mismatch_is_rejected(self) -> None:
   549	        temporary, session = self.make_session()
   550	        self.addCleanup(temporary.cleanup)
   551	        session.rows[0]["cell"] = "wm_tcp_large"
   552	        session.write()
   553	        with self.assertRaisesRegex(analyzer.AnalysisError, "schedule mismatch"):
   554	            analyzer.analyze(session.root)
   555
   556	    def test_settled_ms_schema_and_bounds_are_fail_closed(self) -> None:
   557	        for value in ("249", "1000", "not-an-integer"):
   558	            with self.subTest(settled_ms=value):
   559	                temporary, session = self.make_session()
   560	                self.addCleanup(temporary.cleanup)
   561	                session.rows[0]["settled_ms"] = value
   562	                session.write()
   563	                with self.assertRaisesRegex(analyzer.AnalysisError, "settled_ms"):
   564	                    analyzer.analyze(session.root)
   565
   566	        temporary, session = self.make_session()
   567	        self.addCleanup(temporary.cleanup)
   568	        with (session.root / "runs.csv").open() as handle:
   569	            lines = handle.readlines()
   570	        lines[0] = lines[0].replace("settled_ms,", "")
   571	        (session.root / "runs.csv").write_text("".join(lines))
   572	        with self.assertRaisesRegex(analyzer.AnalysisError, "header mismatch"):
   573	            analyzer.analyze(session.root)
   574
   575	    def test_corrupt_total_is_rejected(self) -> None:
   576	        temporary, session = self.make_session()
   577	        self.addCleanup(temporary.cleanup)
   578	        session.rows[0]["total_ms"] = "999"
   579	        session.write()
   580	        with self.assertRaisesRegex(
   581	            analyzer.AnalysisError,
   582	            "total_ms must equal transfer_ms \\+ \\(settled_ms - 250\\) \\+ flush_ms",
   583	        ):
   584	            analyzer.analyze(session.root)
   585
   586	    def test_role_specific_flush_is_included_in_delta_and_floor(self) -> None:
   587	        temporary, session = self.make_session()
   588	        self.addCleanup(temporary.cleanup)
   589	        destination_flush = (18, 16, 14, 12, 10, 8, 6, 4)
   590	        for row in session.rows:
   591	            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
   592	                continue
   593	            flush_ms = (
   594	                10
   595	                if row["role"] == "source_init"
   596	                else destination_flush[int(row["pair"]) - 1]
   597	            )
   598	            row["flush_ms"] = str(flush_ms)
   599	            row["total_ms"] = str(
   600	                int(row["transfer_ms"])
   601	                + int(row["settled_ms"])
   602	                - analyzer.SETTLE_MIN_MS
   603	                + flush_ms
   604	            )
   605	        session.write()
   606
   607	        result = analyzer.analyze(session.root)
   608	        with result.summary_csv.open(newline="") as handle:
   609	            rows = {
   610	                (row["cell"], row["trace_state"]): row
   611	                for row in csv.DictReader(handle)
   612	            }
   613	        off = rows[(analyzer.TARGET_CELL, "off")]
   614	        self.assertEqual(off["delta_ms"], "46")
   615	        self.assertEqual(off["paired_delta_median_ms"], "46")
   616	        self.assertEqual(off["paired_delta_range_ms"], "56")
   617	        self.assertEqual(off["n_pair_ms"], "56")
   618	        self.assertEqual(str(result.n_resolution), "56")
   619
   620	    def test_excess_settle_is_charged_without_false_role_delta(self) -> None:
   621	        temporary, session = self.make_session()
   622	        self.addCleanup(temporary.cleanup)
   623	        old_formula_totals: dict[str, set[int]] = {
   624	            "source_init": set(),
   625	            "destination_init": set(),
   626	        }
   627	        actual_elapsed: set[int] = set()
   628	        for row in session.rows:
   629	            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
   630	                continue
   631	            transfer_ms = 100
   632	            if row["role"] == "source_init":
   633	                settled_ms, flush_ms = 999, 1
   634	            else:
   635	                settled_ms, flush_ms = 250, 750
   636	            row["transfer_ms"] = str(transfer_ms)
   637	            row["settled_ms"] = str(settled_ms)
   638	            row["flush_ms"] = str(flush_ms)
   639	            row["total_ms"] = str(
   640	                transfer_ms
   641	                + settled_ms
   642	                - analyzer.SETTLE_MIN_MS
   643	                + flush_ms
   644	            )
   645	            old_formula_totals[row["role"]].add(transfer_ms + flush_ms)
   646	            actual_elapsed.add(transfer_ms + settled_ms + flush_ms)
   647	        self.assertEqual(actual_elapsed, {1100})
   648	        self.assertEqual(old_formula_totals["source_init"], {101})
   649	        self.assertEqual(old_formula_totals["destination_init"], {850})
   650	        session.write()
   651
   652	        result = analyzer.analyze(session.root)
   653	        with result.summary_csv.open(newline="") as handle:
   654	            rows = {
   655	                (row["cell"], row["trace_state"]): row
   656	                for row in csv.DictReader(handle)
   657	            }
   658	        off = rows[(analyzer.TARGET_CELL, "off")]
   659	        self.assertEqual(off["source_init_median_ms"], "850")
   660	        self.assertEqual(off["destination_init_median_ms"], "850")
   661	        self.assertEqual(off["delta_ms"], "0")
   662	        self.assertEqual(off["paired_delta_median_ms"], "0")
   663	        self.assertEqual(off["paired_delta_range_ms"], "0")
   664	        self.assertEqual(off["n_pair_ms"], "0")
   665
   666	    def test_landed_manifest_rejects_swapped_sizes_and_renamed_paths(self) -> None:
   667	        mutations = (
   668	            (("a.txt", 2), ("sub/b.txt", 1)),
   669	            (("renamed.txt", 1), ("sub/b.txt", 2)),
   670	        )
   671	        for entries in mutations:
   672	            with self.subTest(entries=entries):
   673	                temporary, session = self.make_session()
   674	                self.addCleanup(temporary.cleanup)
   675	                row = next(
   676	                    row
   677	                    for row in session.rows
   678	                    if row["cell"].endswith("_mixed")
   679	                )
   680	                lines = sorted(
   681	                    f"{base64.b64encode(path.encode()).decode()},{size}"
   682	                    for path, size in entries
   683	                )
   684	                data = "".join(f"{line}\n" for line in lines).encode("ascii")
   685	                digest = hashlib.sha256(data).hexdigest()
   686	                rid = (
   687	                    f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
   688	                )
   689	                (session.root / "landed" / f"{rid}.manifest").write_bytes(data)
   690	                row["tree_manifest_sha256"] = digest
   691	                session.write()
   692	                with self.assertRaisesRegex(
   693	                    analyzer.AnalysisError,
   694	                    "landed relative-path/size manifest does not match canonical",
   695	                ):
   696	                    analyzer.analyze(session.root)
   697
   698	    def test_landed_root_and_recorded_manifest_digest_are_exact(self) -> None:
   699	        temporary, session = self.make_session()
   700	        self.addCleanup(temporary.cleanup)
   701	        session.rows[0]["landed_root"] = "wrapper/src_large"
   702	        session.write()
   703	        with self.assertRaisesRegex(analyzer.AnalysisError, "landed_root must be"):
   704	            analyzer.analyze(session.root)
   705
   706	        temporary_digest, digest_session = self.make_session()
   707	        self.addCleanup(temporary_digest.cleanup)
   708	        digest_session.rows[0]["tree_manifest_sha256"] = "0" * 64
   709	        digest_session.write()
   710	        with self.assertRaisesRegex(
   711	            analyzer.AnalysisError, "landed manifest digest mismatch"
   712	        ):
   713	            analyzer.analyze(digest_session.root)
   714

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze_test.py | sed -n '1,418p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env python3
     2	"""Synthetic guards for otp12pf_rigw_analyze.py."""
     3
     4	from __future__ import annotations
     5
     6	import base64
     7	import csv
     8	import hashlib
     9	import importlib.util
    10	import json
    11	import sys
    12	import tempfile
    13	import unittest
    14	from pathlib import Path
    15
    16
    17	MODULE_PATH = Path(__file__).with_name("otp12pf_rigw_analyze.py")
    18	SPEC = importlib.util.spec_from_file_location("otp12pf_rigw_analyze", MODULE_PATH)
    19	assert SPEC and SPEC.loader
    20	analyzer = importlib.util.module_from_spec(SPEC)
    21	sys.modules[SPEC.name] = analyzer
    22	SPEC.loader.exec_module(analyzer)
    23
    24
    25	class SyntheticSession:
    26	    def __init__(self, root: Path) -> None:
    27	        self.root = root
    28	        self.rows: list[dict[str, str]] = []
    29	        self.events: list[dict[str, object]] = []
    30	        self.clock_rows: list[dict[str, str]] = []
    31	        self._session_counter = 1
    32	        self._build()
    33	        self._build_manifest_evidence()
    34	        self._build_clock_samples()
    35	        self.write()
    36
    37	    @staticmethod
    38	    def _delta(trace_state: str, cell: str, pair: int) -> int:
    39	        if cell != analyzer.TARGET_CELL:
    40	            return 5
    41	        if trace_state == "off":
    42	            return (10, 20, 30, 40, 50, 60, 70, 80)[pair - 1]
    43	        return (20, 20, 20, 20, 30, 30, 30, 30)[pair - 1]
    44
    45	    def _trace_events(
    46	        self, run_id: str, session_id: str, scheduled_role: str
    47	    ) -> list[dict[str, object]]:
    48	        initiator = "SOURCE" if scheduled_role == "source_init" else "DESTINATION"
    49	        source_action = "dial" if initiator == "SOURCE" else "accept"
    50	        destination_action = "accept" if initiator == "SOURCE" else "dial"
    51
    52	        def event(
    53	            endpoint_role: str,
    54	            seq: int,
    55	            elapsed: int,
    56	            name: str,
    57	            **extra: object,
    58	        ) -> dict[str, object]:
    59	            value: dict[str, object] = {
    60	                "schema": 1,
    61	                "run_id": run_id,
    62	                "session_id": session_id,
    63	                "producer_seq": seq,
    64	                "unix_ns": 1_000_000 + elapsed,
    65	                "elapsed_ns": elapsed,
    66	                "endpoint_role": endpoint_role,
    67	                "initiator_role": initiator,
    68	                "event": name,
    69	            }
    70	            value.update(extra)
    71	            return value
    72
    73	        source: list[dict[str, object]] = []
    74	        destination: list[dict[str, object]] = []
    75
    76	        def source_event(name: str, **extra: object) -> None:
    77	            seq = len(source)
    78	            source.append(event("SOURCE", seq, seq, name, **extra))
    79
    80	        def destination_event(name: str, **extra: object) -> None:
    81	            seq = len(destination)
    82	            destination.append(event("DESTINATION", seq, seq, name, **extra))
    83
    84	        source_event(f"socket_{source_action}_begin", epoch=0, socket=0)
    85	        source_event(f"socket_{source_action}_end", epoch=0, socket=0)
    86	        source_event("socket_trace_attached", epoch=0, socket=0)
    87	        source_event("manifest_complete_send_begin")
    88	        source_event("manifest_complete_sent", count=1)
    89	        source_event("need_batch_received", batch=0, count=1)
    90	        source_event("planner_begin", batch=0, count=1)
    91	        source_event("planner_end", batch=0, count=1)
    92	        for epoch in range(1, 8):
    93	            target = epoch + 1
    94	            source_event(
    95	                "resize_proposed",
    96	                epoch=epoch,
    97	                target_streams=target,
    98	                live_streams=epoch,
    99	            )
   100	            source_event(
   101	                "resize_send_begin",
   102	                epoch=epoch,
   103	                target_streams=target,
   104	                live_streams=epoch,
   105	            )
   106	            source_event(
   107	                "resize_sent",
   108	                epoch=epoch,
   109	                target_streams=target,
   110	                live_streams=epoch,
   111	            )
   112	            source_event(
   113	                "resize_ack_received",
   114	                epoch=epoch,
   115	                accepted=True,
   116	                live_streams=target,
   117	            )
   118	            source_event(f"socket_{source_action}_begin", epoch=epoch, socket=0)
   119	            source_event(f"socket_{source_action}_end", epoch=epoch, socket=0)
   120	            source_event("socket_trace_attached", epoch=epoch, socket=0)
   121	            source_event(
   122	                "source_settled",
   123	                epoch=epoch,
   124	                target_streams=target,
   125	                live_streams=target,
   126	                accepted=True,
   127	            )
   128	        source_event("first_payload_queued")
   129	        source_event("socket_write_begin", epoch=0, socket=0)
   130	        source_event("first_socket_write", epoch=0, socket=0)
   131	        source_event("data_plane_complete")
   132	        source_event("summary_received")
   133
   134	        destination_event(f"socket_{destination_action}_begin", epoch=0, socket=0)
   135	        destination_event(f"socket_{destination_action}_end", epoch=0, socket=0)
   136	        destination_event("socket_trace_attached", epoch=0, socket=0)
   137	        destination_event("manifest_complete_received")
   138	        destination_event("need_batch_send_begin", batch=0, count=1)
   139	        destination_event("need_batch_sent", batch=0, count=1)
   140	        for epoch in range(1, 8):
   141	            target = epoch + 1
   142	            destination_event(
   143	                "resize_received",
   144	                epoch=epoch,
   145	                target_streams=target,
   146	                live_streams=epoch,
   147	            )
   148	            if initiator == "SOURCE":
   149	                destination_event(
   150	                    "resize_arm_queue_begin",
   151	                    epoch=epoch,
   152	                    target_streams=target,
   153	                )
   154	                destination_event(
   155	                    "destination_prepared",
   156	                    epoch=epoch,
   157	                    target_streams=target,
   158	                    action="arm_queued",
   159	                )
   160	                destination_event(
   161	                    "resize_ack_send_begin",
   162	                    epoch=epoch,
   163	                    accepted=True,
   164	                    live_streams=target,
   165	                )
   166	                destination_event(
   167	                    "resize_ack_sent",
   168	                    epoch=epoch,
   169	                    accepted=True,
   170	                    live_streams=target,
   171	                )
   172	                destination_event("resize_arm_ready", epoch=epoch)
   173	                destination_event("socket_accept_begin", epoch=epoch, socket=0)
   174	                destination_event("socket_accept_end", epoch=epoch, socket=0)
   175	                destination_event("socket_trace_attached", epoch=epoch, socket=0)
   176	            else:
   177	                destination_event("socket_dial_begin", epoch=epoch, socket=0)
   178	                destination_event("socket_dial_end", epoch=epoch, socket=0)
   179	                destination_event("socket_trace_attached", epoch=epoch, socket=0)
   180	                destination_event(
   181	                    "destination_prepared",
   182	                    epoch=epoch,
   183	                    target_streams=target,
   184	                    action="dial_complete",
   185	                )
   186	                destination_event(
   187	                    "resize_ack_send_begin",
   188	                    epoch=epoch,
   189	                    accepted=True,
   190	                    live_streams=target,
   191	                )
   192	                destination_event(
   193	                    "resize_ack_sent",
   194	                    epoch=epoch,
   195	                    accepted=True,
   196	                    live_streams=target,
   197	                )
   198	        destination_event("first_payload_received", epoch=0, socket=0)
   199	        destination_event("data_plane_complete")
   200	        destination_event("summary_send_begin")
   201	        destination_event("summary_sent")
   202	        return source + destination
   203
   204	    def _build(self) -> None:
   205	        client_dir = self.root / "client"
   206	        client_dir.mkdir(parents=True)
   207	        for block in analyzer.BLOCKS:
   208	            run_id = f"rigw-block-{block.number}"
   209	            for round_index, pair in enumerate(block.pairs):
   210	                cells = block.cells if round_index in (0, 3) else tuple(reversed(block.cells))
   211	                for cell in cells:
   212	                    for role_order, role in enumerate(
   213	                        analyzer.expected_roles(pair), start=1
   214	                    ):
   215	                        source_ms = 100
   216	                        transfer_ms = (
   217	                            source_ms
   218	                            if role == "source_init"
   219	                            else source_ms + self._delta(block.trace_state, cell, pair)
   220	                        )
   221	                        settled_ms = 250
   222	                        flush_ms = 1
   223	                        client_log = (
   224	                            f"client/b{block.number}-{cell}-p{pair}-{role}.log"
   225	                        )
   226	                        (self.root / client_log).write_text("synthetic client log\n")
   227	                        traced_tcp = block.trace_state == "on" and cell in analyzer.TCP_CELLS
   228	                        session_id = ""
   229	                        if traced_tcp:
   230	                            session_id = f"{self._session_counter:016x}"
   231	                            self._session_counter += 1
   232	                            self.events.extend(self._trace_events(run_id, session_id, role))
   233	                        self.rows.append(
   234	                            {
   235	                                "block": str(block.number),
   236	                                "trace_state": block.trace_state,
   237	                                "pass": block.pass_name,
   238	                                "cell": cell,
   239	                                "role": role,
   240	                                "pair": str(pair),
   241	                                "role_order": str(role_order),
   242	                                "transfer_ms": str(transfer_ms),
   243	                                "settled_ms": str(settled_ms),
   244	                                "flush_ms": str(flush_ms),
   245	                                "total_ms": str(
   246	                                    transfer_ms
   247	                                    + settled_ms
   248	                                    - analyzer.SETTLE_MIN_MS
   249	                                    + flush_ms
   250	                                ),
   251	                                "exit": "0",
   252	                                "drain": "drained",
   253	                                "valid": "yes",
   254	                                "run_id": run_id,
   255	                                "session_id": session_id,
   256	                                "client_log": client_log,
   257	                            }
   258	                        )
   259
   260	    def _build_clock_samples(self) -> None:
   261	        q_clock = 1_000_000_000
   262	        for row in self.rows:
   263	            for phase in ("before", "after"):
   264	                for sample in range(1, 4):
   265	                    rtt = 10 + sample
   266	                    q_before = q_clock
   267	                    q_after = q_before + rtt
   268	                    offset = int(row["block"]) * 1_000 + (100 if phase == "after" else 0)
   269	                    midpoint = q_before + rtt // 2
   270	                    windows = midpoint + offset
   271	                    self.clock_rows.append(
   272	                        {
   273	                            "block": row["block"],
   274	                            "run_id": row["run_id"],
   275	                            "cell": row["cell"],
   276	                            "pair": row["pair"],
   277	                            "role": row["role"],
   278	                            "phase": phase,
   279	                            "sample": str(sample),
   280	                            "q_before_ns": str(q_before),
   281	                            "windows_ns": str(windows),
   282	                            "q_after_ns": str(q_after),
   283	                            "rtt_ns": str(rtt),
   284	                            "offset_windows_minus_q_ns": str(offset),
   285	                        }
   286	                    )
   287	                    q_clock = q_after + 100
   288
   289	    @staticmethod
   290	    def _manifest_data(shape: str) -> bytes:
   291	        entries = (
   292	            (("a.txt", 1), ("sub/b.txt", 2))
   293	            if shape == "mixed"
   294	            else (("large.bin", 3),)
   295	        )
   296	        lines = sorted(
   297	            f"{base64.b64encode(path.encode()).decode()},{size}"
   298	            for path, size in entries
   299	        )
   300	        return "".join(f"{line}\n" for line in lines).encode("ascii")
   301
   302	    def _build_manifest_evidence(self) -> None:
   303	        fixtures = self.root / "fixtures"
   304	        landed = self.root / "landed"
   305	        fixtures.mkdir()
   306	        landed.mkdir()
   307	        index_rows: list[dict[str, str]] = []
   308	        fixture_data: dict[str, tuple[bytes, str]] = {}
   309	        for shape in ("mixed", "large"):
   310	            data = self._manifest_data(shape)
   311	            digest = hashlib.sha256(data).hexdigest()
   312	            q_relative = f"fixtures/src_{shape}.manifest"
   313	            win_relative = f"fixtures/windows-src_{shape}.manifest"
   314	            (self.root / q_relative).write_bytes(data)
   315	            (self.root / win_relative).write_bytes(data)
   316	            index_rows.append(
   317	                {
   318	                    "shape": shape,
   319	                    "sha256": digest,
   320	                    "q_manifest": q_relative,
   321	                    "windows_manifest": win_relative,
   322	                }
   323	            )
   324	            fixture_data[shape] = (data, digest)
   325	        with (self.root / "fixture-manifests.csv").open("w", newline="") as handle:
   326	            writer = csv.DictWriter(
   327	                handle,
   328	                fieldnames=("shape", "sha256", "q_manifest", "windows_manifest"),
   329	            )
   330	            writer.writeheader()
   331	            writer.writerows(index_rows)
   332	        for row in self.rows:
   333	            shape = row["cell"].rsplit("_", 1)[1]
   334	            data, digest = fixture_data[shape]
   335	            row["landed_root"] = f"src_{shape}"
   336	            row["tree_manifest_sha256"] = digest
   337	            rid = (
   338	                f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
   339	            )
   340	            (landed / f"{rid}.manifest").write_bytes(data)
   341
   342	    def write(self) -> None:
   343	        with (self.root / "runs.csv").open("w", newline="") as handle:
   344	            writer = csv.DictWriter(handle, fieldnames=analyzer.CSV_FIELDS)
   345	            writer.writeheader()
   346	            writer.writerows(self.rows)
   347	        with (self.root / "clock-samples.csv").open("w", newline="") as handle:
   348	            writer = csv.DictWriter(handle, fieldnames=analyzer.CLOCK_FIELDS)
   349	            writer.writeheader()
   350	            writer.writerows(self.clock_rows)
   351	        trace = self.root / "trace" / "nested"
   352	        trace.mkdir(parents=True, exist_ok=True)
   353	        client_by_session = {
   354	            row["session_id"]: self.root / row["client_log"]
   355	            for row in self.rows
   356	            if row["session_id"]
   357	        }
   358	        for client_log in client_by_session.values():
   359	            client_log.write_text("synthetic client log\n")
   360	        with (trace / "daemon.log").open("w") as handle:
   361	            handle.write("ignored daemon line\n")
   362	            for event in self.events:
   363	                line = analyzer.TRACE_PREFIX + json.dumps(event, sort_keys=True) + "\n"
   364	                if (
   365	                    event["endpoint_role"] == event["initiator_role"]
   366	                    and event["session_id"] in client_by_session
   367	                ):
   368	                    with client_by_session[event["session_id"]].open("a") as client_handle:
   369	                        client_handle.write(line)
   370	                else:
   371	                    handle.write(line)
   372
   373
   374	class RigWAnalyzerTests(unittest.TestCase):
   375	    def make_session(self) -> tuple[tempfile.TemporaryDirectory[str], SyntheticSession]:
   376	        temporary = tempfile.TemporaryDirectory()
   377	        return temporary, SyntheticSession(Path(temporary.name))
   378
   379	    @staticmethod
   380	    def traced_session_id(session: SyntheticSession, initiator_role: str) -> str:
   381	        return str(
   382	            next(
   383	                event["session_id"]
   384	                for event in session.events
   385	                if event["initiator_role"] == initiator_role
   386	            )
   387	        )
   388
   389	    @staticmethod
   390	    def phase_event(
   391	        session: SyntheticSession,
   392	        session_id: str,
   393	        endpoint_role: str,
   394	        event_name: str,
   395	        epoch: int | None,
   396	    ) -> dict[str, object]:
   397	        return next(
   398	            event
   399	            for event in session.events
   400	            if event["session_id"] == session_id
   401	            and event["endpoint_role"] == endpoint_role
   402	            and event["event"] == event_name
   403	            and event.get("epoch") == epoch
   404	        )
   405
   406	    @staticmethod
   407	    def reorder_local_events(desired_order: list[dict[str, object]]) -> None:
   408	        fields = ("producer_seq", "elapsed_ns", "unix_ns")
   409	        assert len({event["session_id"] for event in desired_order}) == 1
   410	        assert len({event["endpoint_role"] for event in desired_order}) == 1
   411	        slots = sorted(
   412	            (tuple(event[field] for field in fields) for event in desired_order),
   413	            key=lambda slot: int(slot[0]),
   414	        )
   415	        for event, slot in zip(desired_order, slots):
   416	            for field, value in zip(fields, slot):
   417	                event[field] = value
   418

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze_test.py | sed -n '715,1069p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   715	    def test_sequence_gap_and_missing_terminal_are_rejected(self) -> None:
   716	        temporary, session = self.make_session()
   717	        self.addCleanup(temporary.cleanup)
   718	        first_session = next(event["session_id"] for event in session.events)
   719	        for event in session.events:
   720	            if (
   721	                event["session_id"] == first_session
   722	                and event["endpoint_role"] == "SOURCE"
   723	                and event["producer_seq"] == 2
   724	            ):
   725	                event["producer_seq"] = 9
   726	                break
   727	        session.write()
   728	        with self.assertRaisesRegex(analyzer.AnalysisError, "producer_seq"):
   729	            analyzer.analyze(session.root)
   730
   731	    def test_payload_write_must_precede_source_completion(self) -> None:
   732	        temporary, session = self.make_session()
   733	        self.addCleanup(temporary.cleanup)
   734	        first_session = next(event["session_id"] for event in session.events)
   735	        write = next(
   736	            event
   737	            for event in session.events
   738	            if event["session_id"] == first_session
   739	            and event["endpoint_role"] == "SOURCE"
   740	            and event["event"] == "first_socket_write"
   741	        )
   742	        complete = next(
   743	            event
   744	            for event in session.events
   745	            if event["session_id"] == first_session
   746	            and event["endpoint_role"] == "SOURCE"
   747	            and event["event"] == "data_plane_complete"
   748	        )
   749	        for field in ("producer_seq", "elapsed_ns", "unix_ns"):
   750	            write[field], complete[field] = complete[field], write[field]
   751	        session.write()
   752	        with self.assertRaisesRegex(
   753	            analyzer.AnalysisError,
   754	            "SOURCE/first_socket_write -> SOURCE/data_plane_complete",
   755	        ):
   756	            analyzer.analyze(session.root)
   757
   758	    def test_socket_action_end_must_precede_trace_attachment(self) -> None:
   759	        temporary, session = self.make_session()
   760	        self.addCleanup(temporary.cleanup)
   761	        first_session = next(event["session_id"] for event in session.events)
   762	        action_end = next(
   763	            event
   764	            for event in session.events
   765	            if event["session_id"] == first_session
   766	            and event["endpoint_role"] == "SOURCE"
   767	            and event.get("epoch") == 0
   768	            and str(event["event"]).startswith("socket_")
   769	            and str(event["event"]).endswith("_end")
   770	        )
   771	        attached = next(
   772	            event
   773	            for event in session.events
   774	            if event["session_id"] == first_session
   775	            and event["endpoint_role"] == "SOURCE"
   776	            and event["event"] == "socket_trace_attached"
   777	            and event.get("epoch") == 0
   778	        )
   779	        for field in ("producer_seq", "elapsed_ns", "unix_ns"):
   780	            action_end[field], attached[field] = attached[field], action_end[field]
   781	        session.write()
   782	        with self.assertRaisesRegex(
   783	            analyzer.AnalysisError,
   784	            "SOURCE/socket_.*_end -> SOURCE/socket_trace_attached",
   785	        ):
   786	            analyzer.analyze(session.root)
   787
   788	    def test_causal_elapsed_time_cannot_run_backwards(self) -> None:
   789	        temporary, session = self.make_session()
   790	        self.addCleanup(temporary.cleanup)
   791	        first_session = next(event["session_id"] for event in session.events)
   792	        attached = next(
   793	            event
   794	            for event in session.events
   795	            if event["session_id"] == first_session
   796	            and event["endpoint_role"] == "SOURCE"
   797	            and event["event"] == "socket_trace_attached"
   798	            and event.get("epoch") == 0
   799	        )
   800	        write_begin = next(
   801	            event
   802	            for event in session.events
   803	            if event["session_id"] == first_session
   804	            and event["endpoint_role"] == "SOURCE"
   805	            and event["event"] == "socket_write_begin"
   806	            and event.get("epoch") == 0
   807	        )
   808	        write_begin["elapsed_ns"] = int(attached["elapsed_ns"]) - 1
   809	        session.write()
   810	        with self.assertRaisesRegex(
   811	            analyzer.AnalysisError,
   812	            "SOURCE/socket_trace_attached -> SOURCE/socket_write_begin",
   813	        ):
   814	            analyzer.analyze(session.root)
   815
   816	    def test_destination_resize_prerequisites_are_causal(self) -> None:
   817	        cases = (
   818	            (
   819	                "SOURCE",
   820	                "resize_received",
   821	                "resize_arm_queue_begin",
   822	            ),
   823	            (
   824	                "SOURCE",
   825	                "resize_arm_ready",
   826	                "socket_accept_begin",
   827	            ),
   828	            (
   829	                "DESTINATION",
   830	                "resize_received",
   831	                "socket_dial_begin",
   832	            ),
   833	            (
   834	                "DESTINATION",
   835	                "socket_trace_attached",
   836	                "destination_prepared",
   837	            ),
   838	        )
   839	        for initiator_role, start_name, end_name in cases:
   840	            with self.subTest(
   841	                initiator_role=initiator_role,
   842	                edge=f"{start_name}->{end_name}",
   843	            ):
   844	                temporary, session = self.make_session()
   845	                self.addCleanup(temporary.cleanup)
   846	                session_id = self.traced_session_id(session, initiator_role)
   847	                start = self.phase_event(
   848	                    session, session_id, "DESTINATION", start_name, 1
   849	                )
   850	                end = self.phase_event(
   851	                    session, session_id, "DESTINATION", end_name, 1
   852	                )
   853	                self.reorder_local_events([end, start])
   854	                session.write()
   855	                with self.assertRaisesRegex(
   856	                    analyzer.AnalysisError,
   857	                    f"DESTINATION/{start_name} -> DESTINATION/{end_name}",
   858	                ):
   859	                    analyzer.analyze(session.root)
   860
   861	    def test_source_resize_prerequisites_are_causal(self) -> None:
   862	        for initiator_role, source_action in (
   863	            ("SOURCE", "dial"),
   864	            ("DESTINATION", "accept"),
   865	        ):
   866	            with self.subTest(
   867	                initiator_role=initiator_role,
   868	                edge=f"resize_sent->socket_{source_action}_begin",
   869	            ):
   870	                temporary, session = self.make_session()
   871	                self.addCleanup(temporary.cleanup)
   872	                session_id = self.traced_session_id(session, initiator_role)
   873	                sent = self.phase_event(
   874	                    session, session_id, "SOURCE", "resize_sent", 1
   875	                )
   876	                ack = self.phase_event(
   877	                    session, session_id, "SOURCE", "resize_ack_received", 1
   878	                )
   879	                action_begin = self.phase_event(
   880	                    session,
   881	                    session_id,
   882	                    "SOURCE",
   883	                    f"socket_{source_action}_begin",
   884	                    1,
   885	                )
   886	                self.reorder_local_events([ack, action_begin, sent])
   887	                session.write()
   888	                with self.assertRaisesRegex(
   889	                    analyzer.AnalysisError,
   890	                    f"SOURCE/resize_sent -> SOURCE/socket_{source_action}_begin",
   891	                ):
   892	                    analyzer.analyze(session.root)
   893
   894	            with self.subTest(
   895	                initiator_role=initiator_role,
   896	                edge="socket_trace_attached->source_settled",
   897	            ):
   898	                temporary, session = self.make_session()
   899	                self.addCleanup(temporary.cleanup)
   900	                session_id = self.traced_session_id(session, initiator_role)
   901	                attached = self.phase_event(
   902	                    session, session_id, "SOURCE", "socket_trace_attached", 1
   903	                )
   904	                settled = self.phase_event(
   905	                    session, session_id, "SOURCE", "source_settled", 1
   906	                )
   907	                self.reorder_local_events([settled, attached])
   908	                session.write()
   909	                with self.assertRaisesRegex(
   910	                    analyzer.AnalysisError,
   911	                    "SOURCE/socket_trace_attached -> SOURCE/source_settled",
   912	                ):
   913	                    analyzer.analyze(session.root)
   914
   915	    def test_final_resize_settlement_precedes_data_plane_completion(self) -> None:
   916	        for initiator_role in ("SOURCE", "DESTINATION"):
   917	            with self.subTest(
   918	                initiator_role=initiator_role,
   919	                edge="SOURCE/source_settled->data_plane_complete",
   920	            ):
   921	                temporary, session = self.make_session()
   922	                self.addCleanup(temporary.cleanup)
   923	                session_id = self.traced_session_id(session, initiator_role)
   924	                settled = self.phase_event(
   925	                    session, session_id, "SOURCE", "source_settled", 7
   926	                )
   927	                first_queued = self.phase_event(
   928	                    session, session_id, "SOURCE", "first_payload_queued", None
   929	                )
   930	                write_begin = self.phase_event(
   931	                    session, session_id, "SOURCE", "socket_write_begin", 0
   932	                )
   933	                first_write = self.phase_event(
   934	                    session, session_id, "SOURCE", "first_socket_write", 0
   935	                )
   936	                complete = self.phase_event(
   937	                    session, session_id, "SOURCE", "data_plane_complete", None
   938	                )
   939	                self.reorder_local_events(
   940	                    [first_queued, write_begin, first_write, complete, settled]
   941	                )
   942	                session.write()
   943	                with self.assertRaisesRegex(
   944	                    analyzer.AnalysisError,
   945	                    "SOURCE/source_settled -> SOURCE/data_plane_complete",
   946	                ):
   947	                    analyzer.analyze(session.root)
   948
   949	            with self.subTest(
   950	                initiator_role=initiator_role,
   951	                edge="DESTINATION/resize_ack_sent->data_plane_complete",
   952	            ):
   953	                temporary, session = self.make_session()
   954	                self.addCleanup(temporary.cleanup)
   955	                session_id = self.traced_session_id(session, initiator_role)
   956	                ack_sent = self.phase_event(
   957	                    session, session_id, "DESTINATION", "resize_ack_sent", 7
   958	                )
   959	                first_received = self.phase_event(
   960	                    session,
   961	                    session_id,
   962	                    "DESTINATION",
   963	                    "first_payload_received",
   964	                    0,
   965	                )
   966	                complete = self.phase_event(
   967	                    session,
   968	                    session_id,
   969	                    "DESTINATION",
   970	                    "data_plane_complete",
   971	                    None,
   972	                )
   973	                self.reorder_local_events([first_received, complete, ack_sent])
   974	                session.write()
   975	                with self.assertRaisesRegex(
   976	                    analyzer.AnalysisError,
   977	                    "DESTINATION/resize_ack_sent -> DESTINATION/data_plane_complete",
   978	                ):
   979	                    analyzer.analyze(session.root)
   980
   981	    def test_destination_preparation_action_is_role_correlated(self) -> None:
   982	        temporary, session = self.make_session()
   983	        self.addCleanup(temporary.cleanup)
   984	        prepared = next(
   985	            event
   986	            for event in session.events
   987	            if event["event"] == "destination_prepared"
   988	            and event["initiator_role"] == "SOURCE"
   989	        )
   990	        prepared["action"] = "dial_complete"
   991	        session.write()
   992	        with self.assertRaisesRegex(analyzer.AnalysisError, "must be arm_queued"):
   993	            analyzer.analyze(session.root)
   994
   995	    def test_resize_ramp_requires_all_seven_epochs(self) -> None:
   996	        temporary, session = self.make_session()
   997	        self.addCleanup(temporary.cleanup)
   998	        first_session = next(event["session_id"] for event in session.events)
   999	        session.events = [
  1000	            event
  1001	            for event in session.events
  1002	            if not (
  1003	                event["session_id"] == first_session and event.get("epoch") == 7
  1004	            )
  1005	        ]
  1006	        for endpoint_role in ("SOURCE", "DESTINATION"):
  1007	            role_events = [
  1008	                event
  1009	                for event in session.events
  1010	                if event["session_id"] == first_session
  1011	                and event["endpoint_role"] == endpoint_role
  1012	            ]
  1013	            for producer_seq, event in enumerate(
  1014	                sorted(role_events, key=lambda item: int(item["producer_seq"]))
  1015	            ):
  1016	                event["producer_seq"] = producer_seq
  1017	        session.write()
  1018	        with self.assertRaisesRegex(
  1019	            analyzer.AnalysisError, "resize epochs must be exactly 1..7"
  1020	        ):
  1021	            analyzer.analyze(session.root)
  1022
  1023	    def test_final_resize_target_and_live_fields_are_exact_on_both_roles(self) -> None:
  1024	        mutations = (
  1025	            ("SOURCE", "source_settled", "target_streams"),
  1026	            ("SOURCE", "source_settled", "live_streams"),
  1027	            ("DESTINATION", "resize_received", "target_streams"),
  1028	            ("DESTINATION", "resize_ack_sent", "live_streams"),
  1029	        )
  1030	        for endpoint_role, event_name, field in mutations:
  1031	            with self.subTest(
  1032	                endpoint_role=endpoint_role, event=event_name, field=field
  1033	            ):
  1034	                temporary, session = self.make_session()
  1035	                self.addCleanup(temporary.cleanup)
  1036	                first_session = next(event["session_id"] for event in session.events)
  1037	                marker = next(
  1038	                    event
  1039	                    for event in session.events
  1040	                    if event["session_id"] == first_session
  1041	                    and event["endpoint_role"] == endpoint_role
  1042	                    and event["event"] == event_name
  1043	                    and event.get("epoch") == 7
  1044	                )
  1045	                marker[field] = 7
  1046	                session.write()
  1047	                with self.assertRaisesRegex(
  1048	                    analyzer.AnalysisError,
  1049	                    f"{endpoint_role}/{event_name} epoch 7 {field} must be 8",
  1050	                ):
  1051	                    analyzer.analyze(session.root)
  1052
  1053	    def test_clock_inventory_and_midpoint_math_are_fail_closed(self) -> None:
  1054	        temporary, session = self.make_session()
  1055	        self.addCleanup(temporary.cleanup)
  1056	        session.clock_rows[0]["offset_windows_minus_q_ns"] = "999"
  1057	        session.write()
  1058	        with self.assertRaisesRegex(analyzer.AnalysisError, "offset mismatch"):
  1059	            analyzer.analyze(session.root)
  1060
  1061	        session.clock_rows[0]["offset_windows_minus_q_ns"] = "1000"
  1062	        session.clock_rows.pop()
  1063	        session.write()
  1064	        with self.assertRaisesRegex(analyzer.AnalysisError, "inventory incomplete"):
  1065	            analyzer.analyze(session.root)
  1066
  1067
  1068	if __name__ == "__main__":
  1069	    unittest.main()

exec
/bin/zsh -lc "rg -n \"Transfer\\(|transfer\\(|TransferRequest|rpc Transfer|async fn transfer|copy\" crates/blit-cli/src crates/blit-daemon/src proto/blit.proto | head -n 260" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
proto/blit.proto:15:  rpc Transfer(stream TransferFrame) returns (stream TransferFrame);
proto/blit.proto:39:  // destination daemon when both endpoints in a `blit copy` are
proto/blit.proto:388:  // meaningful when this is a mirror operation; ignored for plain copy.
proto/blit.proto:432:  // spec-level copy had no readers.
proto/blit.proto:495:  // No mirror deletions; copy/update only.
proto/blit.proto:625:  uint64 bytes_zero_copy = 3;
crates/blit-daemon/src/active_jobs.rs:1202:    async fn transfer_ids_unique_under_concurrent_registers() {
crates/blit-cli/src/cli.rs:5:/// Appended to `--help` (not `-h`) for copy/mirror/move so the three
crates/blit-cli/src/cli.rs:9:  blit copy SRC/  DEST/   merge SRC's contents into DEST (no basename nesting)
crates/blit-cli/src/cli.rs:10:  blit copy SRC   DEST/   nest SRC under DEST  -> DEST/<basename(SRC)>
crates/blit-cli/src/cli.rs:11:  blit copy SRC   DEST    if DEST exists as a dir: nest; else DEST becomes the copy
crates/blit-cli/src/cli.rs:12:  blit copy f.txt DEST/   DEST/f.txt (into the directory)
crates/blit-cli/src/cli.rs:13:  blit copy f.txt new.txt rename (when new.txt does not exist)
crates/blit-cli/src/cli.rs:15:A trailing slash on SRC means \"copy the contents\". Without one, the basename is
crates/blit-cli/src/cli.rs:23:  blit check verifies that a destination tree matches what `blit copy` or
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
crates/blit-cli/src/cli.rs:632:            Cli::try_parse_from(["blit", "copy", "--retry", "3", "--wait", "10", "src", "dst"])
crates/blit-cli/src/cli.rs:660:            "copy",
crates/blit-daemon/src/service/delegated_session_e2e.rs:219:/// anymore, and a plain copy (mirror off) must not delete anything.
crates/blit-daemon/src/service/delegated_session_e2e.rs:227:    // Plain copy first: the extraneous file must survive.
crates/blit-daemon/src/service/delegated_session_e2e.rs:232:        "a plain delegated copy never deletes"
crates/blit-cli/src/main.rs:59:                run_transfer(&ctx, &args, TransferKind::Copy)
crates/blit-cli/src/main.rs:66:                run_transfer(&ctx, &args, TransferKind::Mirror)
crates/blit-daemon/src/service/delegated_pull.rs:4://! a `blit copy` are remote. The destination daemon validates the
crates/blit-daemon/src/service/delegated_pull.rs:66:/// in-scope destination files to vanish during a plain copy.
crates/blit-daemon/src/service/delegated_pull.rs:255:    let _guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/delegated_pull.rs:382:                bytes_zero_copy: 0,
crates/blit-daemon/src/service/delegated_pull.rs:495:        // non-empty paths_to_delete to a plain copy would cause the
crates/blit-daemon/src/service/transfer_session_e2e.rs:1046:/// dest copy with ONE corrupt byte at offset 3 MiB (older mtime). With
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
crates/blit-cli/src/transfers/mod.rs:575:    fn copy_local_transfers_file() -> Result<()> {
crates/blit-cli/src/transfers/mod.rs:616:        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-cli/src/transfers/mod.rs:623:    fn copy_local_dry_run_creates_no_files() -> Result<()> {
crates/blit-cli/src/transfers/mod.rs:664:        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-cli/src/transfers/mod.rs:672:    // above (`copy_local_transfers_file`,
crates/blit-cli/src/transfers/mod.rs:673:    // `copy_local_dry_run_creates_no_files`).
crates/blit-cli/src/transfers/mod.rs:734:            .block_on(run_transfer(&ctx, &args, TransferKind::Copy))
crates/blit-daemon/src/service/util.rs:88:/// include a destination subpath (rsync "copy into here" semantics),
crates/blit-cli/src/transfers/remote.rs:170:pub async fn run_remote_push_transfer(
crates/blit-cli/src/transfers/remote.rs:189:/// same-size file whose content differs would destroy the only copy;
crates/blit-cli/src/transfers/remote.rs:191:/// through the shared copy mapping (SizeMtime default, whose
crates/blit-cli/src/transfers/remote.rs:338:pub async fn run_remote_pull_transfer(
crates/blit-cli/src/transfers/remote.rs:351:        false, // emit success summary inline (copy/mirror default)
crates/blit-cli/src/transfers/remote.rs:479:    // JSON. Keys only the deleted driver could fill (bytes_zero_copy —
crates/blit-cli/src/transfers/remote.rs:499:    // (files_requested, bytes_zero_copy, first_payload_ms) are gone;
crates/blit-cli/src/transfers/remote.rs:516:    // keep their exact wording; the old driver-only zero-copy clause
crates/blit-cli/src/transfers/remote.rs:517:    // is gone (always 0 on the session — zero-copy returns as a
crates/blit-daemon/src/service/core.rs:357:    async fn transfer(
crates/blit-daemon/src/service/core.rs:375:        let guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/core.rs:1350:    async fn transfer_cancel_emits_framed_cancelled_error() {
crates/blit-daemon/src/service/core.rs:2112:    async fn progress_event_cannot_arrive_after_terminal_for_same_transfer() {
crates/blit-daemon/src/service/core.rs:2255:    async fn event_matches_filter_matches_only_target_transfer() {
crates/blit-cli/src/transfers/local.rs:11:/// printed inline. Most CLI paths (copy / mirror) want this; move
crates/blit-cli/src/transfers/local.rs:15:pub async fn run_local_transfer(
crates/blit-cli/src/transfers/local.rs:33:/// copy, the same otp-10a F1 hazard the remote move verbs closed.
crates/blit-cli/src/transfers/local.rs:359:        "operation": if mirror { "mirror" } else { "copy" },
crates/blit-daemon/src/metrics.rs:89:    pub fn enter_transfer(self: Arc<Self>) -> ActiveGuard {
crates/blit-daemon/src/metrics.rs:160:        let _g = Arc::clone(&m).enter_transfer();
crates/blit-daemon/src/metrics.rs:172:        let g = Arc::clone(&m).enter_transfer();
crates/blit-daemon/src/metrics.rs:188:            let _g = Arc::clone(&m).enter_transfer();
crates/blit-daemon/src/metrics.rs:197:        // enter_transfer() and drop must still release the gauge.
crates/blit-daemon/src/metrics.rs:201:            let _g = m_for_panic.enter_transfer();
crates/blit-daemon/src/metrics.rs:220:        let g1 = Arc::clone(&m).enter_transfer();
crates/blit-daemon/src/metrics.rs:221:        let g2 = Arc::clone(&m).enter_transfer();
crates/blit-daemon/src/metrics.rs:222:        let g3 = Arc::clone(&m).enter_transfer();
crates/blit-daemon/src/metrics.rs:291:        let _other = Arc::clone(&m).enter_transfer();
crates/blit-daemon/src/metrics.rs:292:        let finishing = Arc::clone(&m).enter_transfer();
crates/blit-cli/src/transfers/remote_remote_direct.rs:70:/// skipped and the source-delete destroyed the only copy — the exact
crates/blit-cli/src/transfers/remote_remote_direct.rs:182:            bytes_zero_copy: 0,
crates/blit-cli/src/transfers/remote_remote_direct.rs:256:        "bytes_zero_copy": summary.bytes_zero_copy,
crates/blit-cli/src/transfers/remote_remote_direct.rs:270:        "Delegated remote-to-remote transfer complete: {} file(s), {} bytes (zero-copy {} bytes){} from {} to {}.",
crates/blit-cli/src/transfers/remote_remote_direct.rs:273:        summary.bytes_zero_copy,
crates/blit-cli/src/transfers/remote_remote_direct.rs:373:    fn delegated_copy_passes_compare_flags_through() {
crates/blit-cli/src/check.rs:31:    // Build filter via the same chokepoint that copy/mirror/move use, so
crates/blit-cli/src/check.rs:32:    // `blit check --exclude '*.tmp'` matches `blit copy --exclude '*.tmp'`.
crates/blit-cli/src/profile.rs:11:                "copy": coefficient_json(&pred.copy),
crates/blit-cli/src/profile.rs:67:    print_coefficient_block("copy", &pred.copy);
crates/blit-daemon/src/runtime.rs:15:    /// handler to bake in a destination subpath (rsync-style "copy
crates/blit-cli/src/diagnostics.rs:112:            blit_core::perf_history::TransferMode::Copy => "copy",

exec
/bin/zsh -lc 'rg -n "session_phase|socket_trace_attached|socket_dial_(begin|end)|socket_accept_(begin|end)|first_payload|resize_(proposed|send|sent|received|ack)|source_settled|destination_prepared|data_plane_complete" crates/blit-core/src/transfer_session crates/blit-core/src | head -n 260' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/transfer_session/mod.rs:49:use crate::remote::transfer::session_phase::{
crates/blit-core/src/transfer_session/mod.rs:224:    pub session_phase_trace: SessionPhaseTrace,
crates/blit-core/src/transfer_session/mod.rs:280:    pub session_phase_trace: SessionPhaseTrace,
crates/blit-core/src/transfer_session/mod.rs:669:fn bind_session_phase_trace(
crates/blit-core/src/transfer_session/mod.rs:695:async fn flush_session_phase_trace(trace: Option<&BoundSessionPhaseTrace>) {
crates/blit-core/src/transfer_session/mod.rs:1079:    let phase_trace = bind_session_phase_trace(
crates/blit-core/src/transfer_session/mod.rs:1080:        instruments.session_phase_trace.clone(),
crates/blit-core/src/transfer_session/mod.rs:1267:                        "resize_ack_received",
crates/blit-core/src/transfer_session/mod.rs:1749:            flush_session_phase_trace(phase_trace.as_ref()).await;
crates/blit-core/src/transfer_session/mod.rs:1935:                        "source_settled",
crates/blit-core/src/transfer_session/mod.rs:1952:                        "source_settled",
crates/blit-core/src/transfer_session/mod.rs:1993:                "resize_proposed",
crates/blit-core/src/transfer_session/mod.rs:2004:                "resize_send_begin",
crates/blit-core/src/transfer_session/mod.rs:2022:                "resize_sent",
crates/blit-core/src/transfer_session/mod.rs:2820:    let phase_trace = bind_session_phase_trace(
crates/blit-core/src/transfer_session/mod.rs:2821:        instruments.session_phase_trace.clone(),
crates/blit-core/src/transfer_session/mod.rs:3347:                        "resize_received",
crates/blit-core/src/transfer_session/mod.rs:3412:                                        "destination_prepared",
crates/blit-core/src/transfer_session/mod.rs:3429:                                    "destination_prepared",
crates/blit-core/src/transfer_session/mod.rs:3454:                        "resize_ack_send_begin",
crates/blit-core/src/transfer_session/mod.rs:3472:                        "resize_ack_sent",
crates/blit-core/src/transfer_session/mod.rs:3531:                            trace.event("data_plane_complete", SessionPhaseFields::default());
crates/blit-core/src/transfer_session/mod.rs:3672:                flush_session_phase_trace(phase_trace.as_ref()).await;
crates/blit-core/src/transfer_session/data_plane.rs:57:use crate::remote::transfer::session_phase::{BoundSessionPhaseTrace, SessionPhaseFields};
crates/blit-core/src/transfer_session/data_plane.rs:277:                    "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:288:                    "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:350:                                "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:377:                            "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:449:            "socket_trace_attached",
crates/blit-core/src/transfer_session/data_plane.rs:628:                "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:646:                "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:696:                "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:714:                "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:877:                "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:899:                "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:907:        let session = session.with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
crates/blit-core/src/transfer_session/data_plane.rs:998:                "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:1009:                "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:1025:        .with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
crates/blit-core/src/transfer_session/data_plane.rs:1131:                        "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:1153:                        "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:1161:                session.with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
crates/blit-core/src/transfer_session/data_plane.rs:1168:                        "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:1179:                        "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:1195:                .with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
crates/blit-core/src/transfer_session/data_plane.rs:1240:            trace.first_payload_queued_at(queued_at);
crates/blit-core/src/transfer_session/data_plane.rs:1271:            trace.event("data_plane_complete", SessionPhaseFields::default());
crates/blit-core/src/transfer_session/local.rs:620:            session_phase_trace: Default::default(),
crates/blit-core/src/transfer_session/local.rs:894:                session_phase_trace: Default::default(),
crates/blit-core/src/remote/transfer/pipeline.rs:447:    phase_trace: Option<crate::remote::transfer::session_phase::BoundSessionPhaseTrace>,
crates/blit-core/src/remote/transfer/pipeline.rs:464:                trace.socket_first_payload_received(epoch, socket_id);
crates/blit-core/src/remote/transfer/session_phase.rs:285:    pub(crate) fn first_payload_queued_at(&self, at: SessionPhaseStamp) {
crates/blit-core/src/remote/transfer/session_phase.rs:286:        self.emit_at(at, "first_payload_queued", SessionPhaseFields::default());
crates/blit-core/src/remote/transfer/session_phase.rs:300:    pub(crate) fn socket_first_payload_received(&self, epoch: u32, socket: u32) {
crates/blit-core/src/remote/transfer/session_phase.rs:302:            "first_payload_received",
crates/blit-core/src/remote/transfer/mod.rs:11:pub mod session_phase;
crates/blit-core/src/remote/transfer/mod.rs:41:pub use session_phase::{SessionPhaseEvent, SessionPhaseRole, SessionPhaseTrace};
crates/blit-core/src/remote/transfer/data_plane.rs:66:    phase_trace: Option<super::session_phase::BoundSessionPhaseTrace>,
crates/blit-core/src/remote/transfer/data_plane.rs:194:    pub(crate) fn with_session_phase_trace(
crates/blit-core/src/remote/transfer/data_plane.rs:196:        trace: Option<super::session_phase::BoundSessionPhaseTrace>,
crates/blit-core/src/remote/transfer/data_plane.rs:202:                "socket_trace_attached",
crates/blit-core/src/remote/transfer/data_plane.rs:203:                super::session_phase::SessionPhaseFields {
crates/blit-core/src/remote/transfer/data_plane.rs:217:    fn take_first_payload_write_trace(
crates/blit-core/src/remote/transfer/data_plane.rs:219:    ) -> Option<super::session_phase::BoundSessionPhaseTrace> {
crates/blit-core/src/remote/transfer/data_plane.rs:227:            super::session_phase::SessionPhaseFields {
crates/blit-core/src/remote/transfer/data_plane.rs:335:        let phase_write = self.take_first_payload_write_trace();
crates/blit-core/src/remote/transfer/data_plane.rs:517:        let phase_write = self.take_first_payload_write_trace();
crates/blit-core/src/remote/transfer/data_plane.rs:615:        let phase_write = self.take_first_payload_write_trace();
crates/blit-core/src/remote/transfer/data_plane.rs:675:        let phase_write = self.take_first_payload_write_trace();
crates/blit-core/src/remote/transfer/session_client.rs:180:            session_phase_trace: Default::default(),
crates/blit-core/src/remote/transfer/session_client.rs:343:            session_phase_trace: Default::default(),
crates/blit-core/src/transfer_session/mod.rs:49:use crate::remote::transfer::session_phase::{
crates/blit-core/src/transfer_session/mod.rs:224:    pub session_phase_trace: SessionPhaseTrace,
crates/blit-core/src/transfer_session/mod.rs:280:    pub session_phase_trace: SessionPhaseTrace,
crates/blit-core/src/transfer_session/mod.rs:669:fn bind_session_phase_trace(
crates/blit-core/src/transfer_session/mod.rs:695:async fn flush_session_phase_trace(trace: Option<&BoundSessionPhaseTrace>) {
crates/blit-core/src/transfer_session/mod.rs:1079:    let phase_trace = bind_session_phase_trace(
crates/blit-core/src/transfer_session/mod.rs:1080:        instruments.session_phase_trace.clone(),
crates/blit-core/src/transfer_session/mod.rs:1267:                        "resize_ack_received",
crates/blit-core/src/transfer_session/mod.rs:1749:            flush_session_phase_trace(phase_trace.as_ref()).await;
crates/blit-core/src/transfer_session/mod.rs:1935:                        "source_settled",
crates/blit-core/src/transfer_session/mod.rs:1952:                        "source_settled",
crates/blit-core/src/transfer_session/mod.rs:1993:                "resize_proposed",
crates/blit-core/src/transfer_session/mod.rs:2004:                "resize_send_begin",
crates/blit-core/src/transfer_session/mod.rs:2022:                "resize_sent",
crates/blit-core/src/transfer_session/mod.rs:2820:    let phase_trace = bind_session_phase_trace(
crates/blit-core/src/transfer_session/mod.rs:2821:        instruments.session_phase_trace.clone(),
crates/blit-core/src/transfer_session/mod.rs:3347:                        "resize_received",
crates/blit-core/src/transfer_session/mod.rs:3412:                                        "destination_prepared",
crates/blit-core/src/transfer_session/mod.rs:3429:                                    "destination_prepared",
crates/blit-core/src/transfer_session/mod.rs:3454:                        "resize_ack_send_begin",
crates/blit-core/src/transfer_session/mod.rs:3472:                        "resize_ack_sent",
crates/blit-core/src/transfer_session/mod.rs:3531:                            trace.event("data_plane_complete", SessionPhaseFields::default());
crates/blit-core/src/transfer_session/mod.rs:3672:                flush_session_phase_trace(phase_trace.as_ref()).await;
crates/blit-core/src/transfer_session/data_plane.rs:57:use crate::remote::transfer::session_phase::{BoundSessionPhaseTrace, SessionPhaseFields};
crates/blit-core/src/transfer_session/data_plane.rs:277:                    "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:288:                    "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:350:                                "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:377:                            "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:449:            "socket_trace_attached",
crates/blit-core/src/transfer_session/data_plane.rs:628:                "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:646:                "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:696:                "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:714:                "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:877:                "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:899:                "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:907:        let session = session.with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
crates/blit-core/src/transfer_session/data_plane.rs:998:                "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:1009:                "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:1025:        .with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
crates/blit-core/src/transfer_session/data_plane.rs:1131:                        "socket_dial_begin",
crates/blit-core/src/transfer_session/data_plane.rs:1153:                        "socket_dial_end",
crates/blit-core/src/transfer_session/data_plane.rs:1161:                session.with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
crates/blit-core/src/transfer_session/data_plane.rs:1168:                        "socket_accept_begin",
crates/blit-core/src/transfer_session/data_plane.rs:1179:                        "socket_accept_end",
crates/blit-core/src/transfer_session/data_plane.rs:1195:                .with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
crates/blit-core/src/transfer_session/data_plane.rs:1240:            trace.first_payload_queued_at(queued_at);
crates/blit-core/src/transfer_session/data_plane.rs:1271:            trace.event("data_plane_complete", SessionPhaseFields::default());
crates/blit-core/src/transfer_session/local.rs:620:            session_phase_trace: Default::default(),
crates/blit-core/src/transfer_session/local.rs:894:                session_phase_trace: Default::default(),

exec
/bin/zsh -lc 'git show --stat --oneline 27c94b0170507c696a6c092bf45e19f466b95131
git show --format=fuller --no-ext-diff --unified=40 27c94b0170507c696a6c092bf45e19f466b95131 -- scripts/bench_otp12pf_rigw.sh
git show --stat --oneline 7e9d2d531977a90b034a662c68e889d495d6fd8b
git show --format=fuller --no-ext-diff --unified=50 7e9d2d531977a90b034a662c68e889d495d6fd8b -- scripts/bench_otp12pf_rigw.sh' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-9FUYGVdf' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-OxE7vbrp' (errno=Operation not permitted)
27c94b0 Fix rig-W path parity guard
 .review/findings/otp12-pf1-rigw-harness.md | 11 ++++++---
 scripts/bench_otp12pf_rigw.sh              | 36 ++++++++++++++++++++----------
 2 files changed, 32 insertions(+), 15 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-fHIJK9S6' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-A6zilsN5' (errno=Operation not permitted)
commit 27c94b0170507c696a6c092bf45e19f466b95131
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Wed Jul 15 07:05:53 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Wed Jul 15 07:05:53 2026 -0400

    Fix rig-W path parity guard

diff --git a/scripts/bench_otp12pf_rigw.sh b/scripts/bench_otp12pf_rigw.sh
index 0df6c44..205f835 100755
--- a/scripts/bench_otp12pf_rigw.sh
+++ b/scripts/bench_otp12pf_rigw.sh
@@ -293,93 +293,105 @@ selftest() {
     local cross_clock_before cross_clock_after cross_clock_delta
     local launcher_tmp launcher_calls launcher_source main_source
     local win_recovery_tmp
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
-    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]]
-    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]]
+    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]] \
+        || die "q source path construction changed"
+    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]] \
+        || die "Windows source path construction changed"
     local destination_rel="rigw-sessions/$SESSION_TAG/destination/container"
-    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]]
-    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]]
-    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]]
-    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]]
-    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]]
-    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]]
-    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]]
-    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]]
-    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]]
-    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]]
+    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]] \
+        || die "q SOURCE-initiated destination path changed"
+    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]] \
+        || die "q DESTINATION-initiated destination path changed"
+    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]] \
+        || die "Windows SOURCE-initiated destination path changed"
+    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
+        || die "Windows DESTINATION-initiated destination path changed"
+    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]] \
+        || die "Windows-to-q physical destination depends on initiator role"
+    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]] \
+        || die "q-to-Windows physical destination depends on initiator role"
+    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]] \
+        || die "Windows-to-q SOURCE-initiated destination argument changed"
+    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]] \
+        || die "Windows-to-q DESTINATION-initiated destination argument changed"
+    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]] \
+        || die "q-to-Windows SOURCE-initiated destination argument changed"
+    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
+        || die "q-to-Windows DESTINATION-initiated destination argument changed"
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

git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-SNmvwHL1' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-dpIFKPyQ' (errno=Operation not permitted)
7e9d2d5 Fix rig-W lifecycle selftest guards
 .review/findings/otp12-pf1-rigw-harness.md | 17 +++++++++++++++-
 scripts/bench_otp12pf_rigw.sh              | 31 ++++++++++++++++++++----------
 2 files changed, 37 insertions(+), 11 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-W7cvmOKJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7criJ4Nk' (errno=Operation not permitted)
commit 7e9d2d531977a90b034a662c68e889d495d6fd8b
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Wed Jul 15 07:12:44 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Wed Jul 15 07:12:44 2026 -0400

    Fix rig-W lifecycle selftest guards

diff --git a/scripts/bench_otp12pf_rigw.sh b/scripts/bench_otp12pf_rigw.sh
index 205f835..83735d1 100755
--- a/scripts/bench_otp12pf_rigw.sh
+++ b/scripts/bench_otp12pf_rigw.sh
@@ -741,328 +741,337 @@ PY
         || die "identical relative-path/size manifests did not match"
     [[ "$tree_digest" =~ ^[0-9a-f]{64}$ ]] \
         || die "tree manifest digest is malformed"
     printf 'aa' > "$manifest_tmp/container/src_mixed/a"
     printf 'b' > "$manifest_tmp/container/src_mixed/sub/b"
     write_q_tree_manifest \
         "$manifest_tmp/container" "$landed_manifest" src_mixed
     if matching_manifest_digest "$canonical_manifest" "$landed_manifest" >/dev/null; then
         rm -rf "$manifest_tmp"
         die "same-count/same-byte tree with swapped file sizes was accepted"
     fi
     rm -rf "$manifest_tmp/container/src_mixed"
     mkdir -p "$manifest_tmp/container/wrapper/src_mixed"
     if write_q_tree_manifest \
         "$manifest_tmp/container" "$landed_manifest" src_mixed 2>/dev/null; then
         rm -rf "$manifest_tmp"
         die "wrong landed root wrapper was accepted"
     fi
     rm -rf "$manifest_tmp"

     freshness_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-freshness.XXXXXX")
     reserve_evidence_dir "$freshness_tmp/new-evidence" \
         || die "fresh evidence directory was rejected: $OUTPUT_CLAIM_ERROR"
     for marker in SESSION-COMPLETE SESSION-VOID unrelated.txt; do
         freshness_case="$freshness_tmp/$marker"
         mkdir "$freshness_case"
         printf 'preserve-me\n' > "$freshness_case/$marker"
         before=$(sha256_q "$freshness_case/$marker")
         if reserve_evidence_dir "$freshness_case"; then
             rm -rf "$freshness_tmp"
             die "stale output directory containing $marker was accepted"
         fi
         [[ "$(sha256_q "$freshness_case/$marker")" == "$before" ]] \
             || die "stale output rejection modified $marker"
     done
     rm -rf "$freshness_tmp"

     destination_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-destination.XXXXXX")
     mkdir -p "$destination_tmp/container/src_mixed"
     printf 'stale\n' > "$destination_tmp/container/src_mixed/stale"
     (
         rm() { return 73; }
         if prepare_destination wm "$destination_tmp/container"; then
             die "q destination reset masked a failed removal"
         fi
     )
     [[ "$(< "$destination_tmp/container/src_mixed/stale")" == stale ]] \
         || die "failed q destination reset modified retained evidence"
     prepare_destination wm "$destination_tmp/container" \
         || die "q destination reset rejected a removable tree"
-    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]]
+    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]] \
+        || die "q destination reset did not leave a plain directory"
     [[ -z "$(find "$destination_tmp/container" -mindepth 1 -maxdepth 1 -print -quit)" ]] \
         || die "q destination reset left stale content"
     rm -rf "$destination_tmp"

     prepare_destination_source=$(declare -f prepare_destination)
     python3 - "$prepare_destination_source" <<'PY' \
         || die "Windows destination reset source contract changed"
 import sys

 source = sys.argv[1]
 for marker in (
     r"\$ErrorActionPreference = 'Stop'",
     r"Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop",
     r"Test-Path -LiteralPath '$dest' -PathType Container",
     r"Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop",
     'ReparsePoint',
 ):
     if marker not in source:
         raise SystemExit(f"missing Windows destination reset marker: {marker}")
 windows = source.split('else', 1)[1]
 if 'SilentlyContinue' in windows:
     raise SystemExit("Windows destination reset suppresses removal errors")
 PY

     finalize_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-finalize.XXXXXX")
     (
         OUT_DIR="$finalize_tmp/fails"
         mkdir "$OUT_DIR"
         HEAD_FULL=0123456789abcdef
         LOCAL_EVIDENCE_COMPLETE=1
         strict_success_cleanup() { return 1; }
         if finalize_registered_session; then
             die "registered finalization accepted failed strict cleanup"
         fi
         [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] \
             || die "failed strict cleanup left SESSION-COMPLETE"
     )
     (
         OUT_DIR="$finalize_tmp/incomplete-local"
         mkdir "$OUT_DIR"
         HEAD_FULL=0123456789abcdef
         LOCAL_EVIDENCE_COMPLETE=0
         strict_success_cleanup() {
             die "finalization cleaned paths before local evidence was complete"
         }
         if finalize_registered_session; then
             die "registered finalization accepted incomplete local evidence"
         fi
         [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]]
     )
     (
         OUT_DIR="$finalize_tmp/succeeds"
         mkdir "$OUT_DIR"
         HEAD_FULL=0123456789abcdef
         LOCAL_EVIDENCE_COMPLETE=1
         strict_success_cleanup() {
             [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] || return 1
             STRICT_CLEANUP_VERIFIED=1
         }
         finalize_registered_session \
             || die "registered finalization rejected verified strict cleanup"
-        [[ "$SESSION_FINALIZED" == 1 ]]
+        [[ "$SESSION_FINALIZED" == 1 ]] \
+            || die "registered finalization did not set SESSION_FINALIZED"
         [[ "$(< "$OUT_DIR/SESSION-COMPLETE")" == "$HEAD_FULL" ]]
     )

     cleanup_tmp="$finalize_tmp/strict"
     mkdir -p "$cleanup_tmp/q/rigw-sessions/fail-remote"
     printf 'retain me\n' > "$cleanup_tmp/q/rigw-sessions/fail-remote/sentinel"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=fail-remote
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         ports_closed() { return 0; }
         wssh() { return 1; }
         if strict_success_cleanup; then
             die "strict cleanup accepted a Windows deletion failure"
         fi
-        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
+            || die "Windows cleanup failure was marked strictly verified"
         [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
             || die "Windows cleanup failure deleted q evidence first"
         [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
             || die "Windows cleanup failure modified q evidence"
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/open-port"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=open-port
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         ports_closed() { return 1; }
         wssh() { die "strict cleanup reached deletion with an open port"; }
         if strict_success_cleanup; then
             die "strict cleanup accepted an open port"
         fi
-        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
+            || die "open-port cleanup failure was marked strictly verified"
         [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/surviving-q"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=surviving-q
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         ports_closed() { return 0; }
         wssh() { return 0; }
         rm() { return 0; }
         if strict_success_cleanup; then
             die "strict cleanup accepted a surviving q session tree"
         fi
-        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
+            || die "surviving q session tree was marked strictly verified"
         [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/succeeds"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=succeeds
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         port_checks=0
         ports_closed() { port_checks=$((port_checks + 1)); return 0; }
         wssh() { return 0; }
         strict_success_cleanup || die "strict cleanup rejected a clean session"
-        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
+            || die "successful strict cleanup did not set verification state"
         [[ "$port_checks" == 2 ]] || die "strict cleanup ran $port_checks port checks"
-        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]]
+        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]] \
+            || die "successful strict cleanup retained may-exist state"
         [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/late-port"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=late-port
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         port_checks=0
         ports_closed() {
             port_checks=$((port_checks + 1))
             [[ "$port_checks" == 1 ]]
         }
         wssh() { return 0; }
         if strict_success_cleanup; then
             die "strict cleanup accepted a listener appearing during deletion"
         fi
         [[ "$STRICT_CLEANUP_VERIFIED" == 0 && "$port_checks" == 2 ]]
     )
     for remembered in q daemon launcher block; do
         (
             Q_MODULE="$cleanup_tmp/q"
             SESSION_TAG="remembered-$remembered"
             q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
             case "$remembered" in
                 q) q_daemon_pid=11;;
                 daemon) win_daemon_pid=22;;
                 launcher) win_cmd_pid=33;;
                 block) current_block=4;;
             esac
             ports_closed() { die "strict cleanup ignored remembered $remembered state"; }
             if strict_success_cleanup; then
                 die "strict cleanup accepted remembered $remembered state"
             fi
             [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
         )
     done
     strict_cleanup_source=$(declare -f strict_success_cleanup)
     python3 - "$strict_cleanup_source" <<'PY' \
         || die "strict cleanup source contract changed"
 import sys

 source = sys.argv[1]
 for marker in (
     "'$WIN_MODULE/rigw-sessions/$SESSION_TAG'",
     "'$WIN_SESSION'",
     r"Remove-Item -LiteralPath \$path -Recurse -Force -ErrorAction Stop",
     r'if (Test-Path -LiteralPath \$path) { throw',
 ):
     if marker not in source:
         raise SystemExit(f"missing strict Windows cleanup marker: {marker}")
 if source.count('ports_closed') != 2:
     raise SystemExit("strict cleanup must check closed ports before and after deletion")
 if source.index('ports_closed') > source.index('Remove-Item -LiteralPath'):
     raise SystemExit("strict cleanup deletes evidence before its first port check")
 if source.rindex('ports_closed') < source.index('rm -rf --'):
     raise SystemExit("strict cleanup lacks a post-deletion port check")
 PY
     rm -rf "$finalize_tmp"

     failure_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-failure.XXXXXX")
     trap_calls="$failure_tmp/remote-calls"
     mkdir -p "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG"
     printf 'retain me\n' > "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel"
     set +e
     (
         set +e
         OUT_DIR="$failure_tmp/evidence"
         mkdir "$OUT_DIR"
         LOG="$OUT_DIR/bench.log"
         OUTPUT_CLAIMED=1
         printf 'primary failure\n' > "$OUT_DIR/SESSION-VOID"
         printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE"
         printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE.tmp"
         REGISTERED_RUN_STARTED=1
         SESSION_FINALIZED=0
         STRICT_CLEANUP_VERIFIED=0
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         Q_MODULE="$failure_tmp/q-module"
         current_block=1
         q_daemon_pid=""
         win_daemon_pid=""
         win_cmd_pid=""
         wssh() {
             printf '%s\n' "$*" >> "$trap_calls"
             return 1
         }
         false
         on_exit
     )
     trap_rc=$?
     set -e
     [[ "$trap_rc" == 1 ]] || die "failure trap returned $trap_rc, expected 1"
-    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]]
-    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]]
+    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]] \
+        || die "failure trap left SESSION-COMPLETE"
+    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]] \
+        || die "failure trap left SESSION-COMPLETE.tmp"
     grep -Fxq 'primary failure' "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap discarded the primary reason"
     grep -Fq 'cleanup errors: Windows PID recovery failed' "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap omitted its cleanup error"
     grep -Fq "q session evidence may remain; inspect $failure_tmp/q-module/rigw-sessions/$SESSION_TAG" \
         "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap omitted the q evidence path"
     grep -Fq "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG" \
         "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap omitted the Windows evidence path"
     if grep -Fq 'Remove-Item' "$trap_calls"; then
         die "failure trap issued destructive remote cleanup"
     fi
     [[ "$(< "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
         || die "failure trap modified q session evidence"
     on_exit_source=$(declare -f on_exit)
     if [[ "$on_exit_source" == *'rm -rf'* \
         || "$on_exit_source" == *'Remove-Item'* \
         || "$on_exit_source" == *'strict_success_cleanup'* ]]; then
         die "failure trap contains a destructive session-cleanup path"
     fi

     append_tmp="$failure_tmp/append-contract"
     mkdir "$append_tmp"
     printf 'original reason\n' > "$append_tmp/SESSION-VOID"
     set +e
     (
         OUT_DIR="$append_tmp"
         LOG="$OUT_DIR/bench.log"
         OUTPUT_CLAIMED=1
         session_void 'later context'
     ) >/dev/null 2>&1
     trap_rc=$?
     set -e
     [[ "$trap_rc" == 1 ]] || die "session_void append probe returned $trap_rc"
     [[ "$(< "$append_tmp/SESSION-VOID")" == $'original reason\nlater context' ]] \
         || die "session_void overwrote an earlier failure reason"

     contract_tmp="$failure_tmp/exit-contract"
     mkdir "$contract_tmp"
     set +e
     (
         set +e
         OUT_DIR="$contract_tmp"
         LOG="$OUT_DIR/bench.log"
         OUTPUT_CLAIMED=1
         REGISTERED_RUN_STARTED=1
         SESSION_FINALIZED=0
         STRICT_CLEANUP_VERIFIED=0
         WIN_SESSION_MAY_EXIST=0
@@ -1179,115 +1188,117 @@ PY
             OUT_DIR="$contract_tmp"
             LOG="$OUT_DIR/bench.log"
             OUTPUT_CLAIMED=1
             REGISTERED_RUN_STARTED=0
             SESSION_FINALIZED=0
             STRICT_CLEANUP_VERIFIED=1
             true
             on_exit
         )
         trap_rc=$?
         set -e
         [[ "$trap_rc" == 1 ]] \
             || die "preflight $marker returned $trap_rc"
         if [[ "$marker" == SESSION-VOID ]]; then
             [[ "$(sed -n '1p' "$contract_tmp/SESSION-VOID")" == 'not allowed' ]] \
                 || die "preflight VOID rejection replaced its primary reason"
         else
             grep -Fq 'successful exit retained a failure or temporary marker' \
                 "$contract_tmp/SESSION-VOID" \
                 || die "preflight $marker omitted its rejection reason"
         fi
     done

     for signal in HUP INT TERM; do
         signal_dir="$failure_tmp/signal-$signal"
         mkdir "$signal_dir"
         set +e
         bash -c '
 set -Eeuo pipefail
 source "$1"
 OUT_DIR="$2"
 LOG="$OUT_DIR/bench.log"
 OUTPUT_CLAIMED=1
 REGISTERED_RUN_STARTED=1
 SESSION_FINALIZED=0
 STRICT_CLEANUP_VERIFIED=0
 Q_SESSION_MAY_EXIST=1
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
+printf "must disappear\n" > "$OUT_DIR/SESSION-COMPLETE"
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
-        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]]
+        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
+            || die "$signal cleanup left SESSION-COMPLETE"
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

exec
/bin/zsh -lc 'git diff --no-ext-diff --unified=12 5e90c769168b2f763d24847fdfa85b548d3f4b7c..27c94b0170507c696a6c092bf45e19f466b95131 -- scripts/bench_otp12pf_rigw.sh
git diff --no-ext-diff --unified=12 27c94b0170507c696a6c092bf45e19f466b95131..7e9d2d531977a90b034a662c68e889d495d6fd8b -- scripts/bench_otp12pf_rigw.sh' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-fovcpQR3' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-r03RmOq0' (errno=Operation not permitted)
diff --git a/scripts/bench_otp12pf_rigw.sh b/scripts/bench_otp12pf_rigw.sh
index 0df6c44..205f835 100755
--- a/scripts/bench_otp12pf_rigw.sh
+++ b/scripts/bench_otp12pf_rigw.sh
@@ -321,37 +321,49 @@ selftest() {
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
-    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]]
-    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]]
+    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]] \
+        || die "q source path construction changed"
+    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]] \
+        || die "Windows source path construction changed"
     local destination_rel="rigw-sessions/$SESSION_TAG/destination/container"
-    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]]
-    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]]
-    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]]
-    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]]
-    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]]
-    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]]
-    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]]
-    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]]
-    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]]
-    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]]
+    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]] \
+        || die "q SOURCE-initiated destination path changed"
+    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]] \
+        || die "q DESTINATION-initiated destination path changed"
+    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]] \
+        || die "Windows SOURCE-initiated destination path changed"
+    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
+        || die "Windows DESTINATION-initiated destination path changed"
+    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]] \
+        || die "Windows-to-q physical destination depends on initiator role"
+    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]] \
+        || die "q-to-Windows physical destination depends on initiator role"
+    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]] \
+        || die "Windows-to-q SOURCE-initiated destination argument changed"
+    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]] \
+        || die "Windows-to-q DESTINATION-initiated destination argument changed"
+    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]] \
+        || die "q-to-Windows SOURCE-initiated destination argument changed"
+    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
+        || die "q-to-Windows DESTINATION-initiated destination argument changed"
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
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nOXWw38C' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-V6vMq6My' (errno=Operation not permitted)
diff --git a/scripts/bench_otp12pf_rigw.sh b/scripts/bench_otp12pf_rigw.sh
index 205f835..83735d1 100755
--- a/scripts/bench_otp12pf_rigw.sh
+++ b/scripts/bench_otp12pf_rigw.sh
@@ -779,25 +779,26 @@ PY
     mkdir -p "$destination_tmp/container/src_mixed"
     printf 'stale\n' > "$destination_tmp/container/src_mixed/stale"
     (
         rm() { return 73; }
         if prepare_destination wm "$destination_tmp/container"; then
             die "q destination reset masked a failed removal"
         fi
     )
     [[ "$(< "$destination_tmp/container/src_mixed/stale")" == stale ]] \
         || die "failed q destination reset modified retained evidence"
     prepare_destination wm "$destination_tmp/container" \
         || die "q destination reset rejected a removable tree"
-    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]]
+    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]] \
+        || die "q destination reset did not leave a plain directory"
     [[ -z "$(find "$destination_tmp/container" -mindepth 1 -maxdepth 1 -print -quit)" ]] \
         || die "q destination reset left stale content"
     rm -rf "$destination_tmp"

     prepare_destination_source=$(declare -f prepare_destination)
     python3 - "$prepare_destination_source" <<'PY' \
         || die "Windows destination reset source contract changed"
 import sys

 source = sys.argv[1]
 for marker in (
     r"\$ErrorActionPreference = 'Stop'",
@@ -841,93 +842,99 @@ PY
     )
     (
         OUT_DIR="$finalize_tmp/succeeds"
         mkdir "$OUT_DIR"
         HEAD_FULL=0123456789abcdef
         LOCAL_EVIDENCE_COMPLETE=1
         strict_success_cleanup() {
             [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] || return 1
             STRICT_CLEANUP_VERIFIED=1
         }
         finalize_registered_session \
             || die "registered finalization rejected verified strict cleanup"
-        [[ "$SESSION_FINALIZED" == 1 ]]
+        [[ "$SESSION_FINALIZED" == 1 ]] \
+            || die "registered finalization did not set SESSION_FINALIZED"
         [[ "$(< "$OUT_DIR/SESSION-COMPLETE")" == "$HEAD_FULL" ]]
     )

     cleanup_tmp="$finalize_tmp/strict"
     mkdir -p "$cleanup_tmp/q/rigw-sessions/fail-remote"
     printf 'retain me\n' > "$cleanup_tmp/q/rigw-sessions/fail-remote/sentinel"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=fail-remote
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         ports_closed() { return 0; }
         wssh() { return 1; }
         if strict_success_cleanup; then
             die "strict cleanup accepted a Windows deletion failure"
         fi
-        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
+            || die "Windows cleanup failure was marked strictly verified"
         [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
             || die "Windows cleanup failure deleted q evidence first"
         [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
             || die "Windows cleanup failure modified q evidence"
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/open-port"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=open-port
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         ports_closed() { return 1; }
         wssh() { die "strict cleanup reached deletion with an open port"; }
         if strict_success_cleanup; then
             die "strict cleanup accepted an open port"
         fi
-        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
+            || die "open-port cleanup failure was marked strictly verified"
         [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/surviving-q"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=surviving-q
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         ports_closed() { return 0; }
         wssh() { return 0; }
         rm() { return 0; }
         if strict_success_cleanup; then
             die "strict cleanup accepted a surviving q session tree"
         fi
-        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
+            || die "surviving q session tree was marked strictly verified"
         [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/succeeds"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=succeeds
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         port_checks=0
         ports_closed() { port_checks=$((port_checks + 1)); return 0; }
         wssh() { return 0; }
         strict_success_cleanup || die "strict cleanup rejected a clean session"
-        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]]
+        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
+            || die "successful strict cleanup did not set verification state"
         [[ "$port_checks" == 2 ]] || die "strict cleanup ran $port_checks port checks"
-        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]]
+        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]] \
+            || die "successful strict cleanup retained may-exist state"
         [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
     )
     mkdir -p "$cleanup_tmp/q/rigw-sessions/late-port"
     (
         Q_MODULE="$cleanup_tmp/q"
         SESSION_TAG=late-port
         Q_SESSION_MAY_EXIST=1
         WIN_SESSION_MAY_EXIST=1
         q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
         port_checks=0
         ports_closed() {
             port_checks=$((port_checks + 1))
@@ -1005,26 +1012,28 @@ PY
         win_daemon_pid=""
         win_cmd_pid=""
         wssh() {
             printf '%s\n' "$*" >> "$trap_calls"
             return 1
         }
         false
         on_exit
     )
     trap_rc=$?
     set -e
     [[ "$trap_rc" == 1 ]] || die "failure trap returned $trap_rc, expected 1"
-    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]]
-    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]]
+    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]] \
+        || die "failure trap left SESSION-COMPLETE"
+    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]] \
+        || die "failure trap left SESSION-COMPLETE.tmp"
     grep -Fxq 'primary failure' "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap discarded the primary reason"
     grep -Fq 'cleanup errors: Windows PID recovery failed' "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap omitted its cleanup error"
     grep -Fq "q session evidence may remain; inspect $failure_tmp/q-module/rigw-sessions/$SESSION_TAG" \
         "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap omitted the q evidence path"
     grep -Fq "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG" \
         "$failure_tmp/evidence/SESSION-VOID" \
         || die "failure trap omitted the Windows evidence path"
     if grep -Fq 'Remove-Item' "$trap_calls"; then
         die "failure trap issued destructive remote cleanup"
@@ -1217,39 +1226,41 @@ WIN_SESSION_MAY_EXIST=1
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
+printf "must disappear\n" > "$OUT_DIR/SESSION-COMPLETE"
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
-        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]]
+        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \
+            || die "$signal cleanup left SESSION-COMPLETE"
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

exec
/bin/zsh -lc "awk 'NR>=283 && NR<=1267 && /\\[\\[/ {print NR \":\" "'$0}'"' scripts/bench_otp12pf_rigw.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
315:    [[ "$got" == "$expected" ]] || die "registered block schedule changed"
326:            [[ "$pair" -ge "$first" && "$pair" -le "$last" && -n "$first_role" ]]
330:    [[ "$rows" == 128 ]] || die "schedule emitted $rows arms, expected 128"
331:    [[ "$source_first" == 32 && "$destination_first" == 32 ]] \
333:    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]] \
335:    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]] \
338:    [[ "$(q_destination_path source_init)" == "$Q_MODULE/$destination_rel" ]] \
340:    [[ "$(q_destination_path destination_init)" == "$Q_MODULE/$destination_rel" ]] \
342:    [[ "$(win_destination_path source_init)" == "$WIN_MODULE/$destination_rel" ]] \
344:    [[ "$(win_destination_path destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
346:    [[ "$(arm_destination_path wm source_init)" == "$(arm_destination_path wm destination_init)" ]] \
348:    [[ "$(arm_destination_path mw source_init)" == "$(arm_destination_path mw destination_init)" ]] \
350:    [[ "$(arm_destination_argument wm source_init)" == "$Q_IP:$PORT:/bench/$destination_rel/" ]] \
352:    [[ "$(arm_destination_argument wm destination_init)" == "$Q_MODULE/$destination_rel" ]] \
354:    [[ "$(arm_destination_argument mw source_init)" == "$WIN_IP:$PORT:/bench/$destination_rel/" ]] \
356:    [[ "$(arm_destination_argument mw destination_init)" == "$WIN_MODULE/$destination_rel" ]] \
359:    [[ "$(awk -F, '{print NF}' <<<"$clock_probe")" == 12 ]] \
361:    [[ "$SETTLE_NS" == 250000000 && "$SETTLE_MIN_MS" == 250 && "$SETTLE_MAX_MS" == 1000 ]] \
367:    [[ "$cross_clock_delta" -ge 40000000 && "$cross_clock_delta" -lt 500000000 ]] \
372:    [[ "$selftest_settle_done" =~ ^[0-9]+$ && "$selftest_settle_done" -ge "$selftest_deadline" ]] \
380:    [[ "$stamp_tag" == R && "$stamp_ms" == 17 && "$stamp_rc" == 0 \
383:    [[ "$stamp_ns" -ge "$stamp_before" && "$stamp_ns" -le "$stamp_after" ]] \
386:    [[ "$stamp_teardown_ns" -ge 250000000 ]] \
475:    empty_pid_branch = recovery.index('if [[ -z "$pid" && -z "$cmdpid" ]]')
553:            if [[ "$script" == *'$pid0 = if'* ]]; then
556:            elif [[ "$script" == *'multiple exact launchers match'* ]]; then
564:        [[ -z "$win_daemon_pid" && -z "$win_cmd_pid" ]] \
566:        [[ "$(< "$win_recovery_tmp/calls")" == $'recover\nstop' ]] \
617:branch_start = main.index('if [[ "$LAUNCHER_SMOKE" == 1 ]]')
622:branch_markers = ('if [[ "$LAUNCHER_SMOKE" == 1 ]]', "launcher_smoke;", "return;", "fi;")
647:            [[ "$1" == launcher-smoke && "$2" == off \
656:            [[ "$*" == "-z -w 3 $WIN_IP $PORT" \
662:            [[ "$current_block" == launcher-smoke \
669:            [[ -z "$q_daemon_pid" ]] \
674:            [[ "$1" == launcher-smoke && -z "$win_cmd_pid" \
681:            if [[ "$port_checks" == 1 ]]; then
682:                [[ "$current_block" == launcher-smoke \
687:                [[ "$port_checks" == 2 && "$current_block" == launcher-smoke \
694:            [[ "$WIN_SESSION_MAY_EXIST" == 1 && -z "$current_block" \
702:        [[ "$(< "$launcher_calls")" == \
705:        [[ "$REGISTERED_RUN_STARTED" == 0 && "$SESSION_FINALIZED" == 0 \
711:        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" \
742:    [[ "$tree_digest" =~ ^[0-9a-f]{64}$ ]] \
773:        [[ "$(sha256_q "$freshness_case/$marker")" == "$before" ]] \
787:    [[ "$(< "$destination_tmp/container/src_mixed/stale")" == stale ]] \
791:    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]] \
793:    [[ -z "$(find "$destination_tmp/container" -mindepth 1 -maxdepth 1 -print -quit)" ]] \
827:        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] \
841:        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]]
849:            [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] || return 1
854:        [[ "$SESSION_FINALIZED" == 1 ]] \
856:        [[ "$(< "$OUT_DIR/SESSION-COMPLETE")" == "$HEAD_FULL" ]]
873:        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
875:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
877:        [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
892:        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
894:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
909:        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]] \
911:        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
924:        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
926:        [[ "$port_checks" == 2 ]] || die "strict cleanup ran $port_checks port checks"
927:        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]] \
929:        [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
941:            [[ "$port_checks" == 1 ]]
947:        [[ "$STRICT_CLEANUP_VERIFIED" == 0 && "$port_checks" == 2 ]]
964:            [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
1023:    [[ "$trap_rc" == 1 ]] || die "failure trap returned $trap_rc, expected 1"
1024:    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]] \
1026:    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]] \
1041:    [[ "$(< "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
1044:    if [[ "$on_exit_source" == *'rm -rf'* \
1062:    [[ "$trap_rc" == 1 ]] || die "session_void append probe returned $trap_rc"
1063:    [[ "$(< "$append_tmp/SESSION-VOID")" == $'original reason\nlater context' ]] \
1083:    [[ "$trap_rc" == 1 ]] \
1107:    [[ "$trap_rc" == 1 ]] \
1132:    [[ "$trap_rc" == 1 ]] \
1134:    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
1153:    [[ "$trap_rc" == 1 ]] \
1176:    [[ "$trap_rc" == 1 ]] \
1178:    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
1199:        [[ "$trap_rc" == 1 ]] \
1201:        if [[ "$marker" == SESSION-VOID ]]; then
1202:            [[ "$(sed -n '1p' "$contract_tmp/SESSION-VOID")" == 'not allowed' ]] \
1247:        [[ "$signal_rc" == 1 ]] \
1251:        [[ "$(LC_ALL=C sort "$signal_dir/stops")" == $'q\nwindows' ]] \
1253:        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]] \

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_rigw.sh | sed -n '2058,2205p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  2058	run_arm() {
  2059	    local block="$1" state="$2" pass="$3" run_id="$4" cell="$5" pair="$6" role="$7" role_order="$8"
  2060	    local direction carrier shape flag="" dest dest_arg rid qerr werr client_rel client_abs remote_err result result_tag result_extra transfer_ms rc flush_out flush_ms count bytes want drain session_id total anchor_now_ns
  2061	    local windows_client=0 arm_phase=client_done client_done_ns settle_deadline_ns settle_done_ns settled_ms
  2062	    local landed_root landed_manifest canonical_manifest remote_manifest tree_manifest_sha256
  2063	    direction=${cell%%_*}
  2064	    carrier=${cell#*_}; carrier=${carrier%%_*}
  2065	    shape=${cell##*_}
  2066	    [[ "$carrier" == grpc ]] && flag=--force-grpc
  2067	    rid="b${block}_${cell}_p${pair}_${role}"
  2068	    qerr="$OUT_DIR/client/$rid.err"
  2069	    remote_err="$WIN_SESSION/block_$block/$rid.client.err"
  2070	    werr="$OUT_DIR/client/$rid.windows.err"
  2071
  2072	    dest=$(arm_destination_path "$direction" "$role") \
  2073	        || session_void "unregistered destination path for $direction/$role"
  2074	    dest_arg=$(arm_destination_argument "$direction" "$role") \
  2075	        || session_void "unregistered destination argument for $direction/$role"
  2076	    prepare_destination "$direction" "$dest" \
  2077	        || session_void "$rid could not precreate its destination container"
  2078
  2079	    drain=$(drain_both) || session_void "$rid cache/drain gate failed"
  2080	    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" before
  2081
  2082	    if [[ "$direction/$role" == wm/source_init ]]; then
  2083	        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
  2084	        result=$(win_client_run "$state" "$run_id" "$remote_err" \
  2085	            "$(win_source_path "$shape")" "$dest_arg" "$flag")
  2086	    elif [[ "$direction/$role" == wm/destination_init ]]; then
  2087	        client_abs="$qerr"; client_rel="client/$rid.err"
  2088	        result=$(q_client_run "$state" "$run_id" "$qerr" \
  2089	            copy "$WIN_IP:$PORT:/bench/src_$shape" "$dest_arg" --yes ${flag:+$flag})
  2090	    elif [[ "$direction/$role" == mw/source_init ]]; then
  2091	        client_abs="$qerr"; client_rel="client/$rid.err"
  2092	        result=$(q_client_run "$state" "$run_id" "$qerr" \
  2093	            copy "$(q_source_path "$shape")" "$dest_arg" --yes ${flag:+$flag})
  2094	    elif [[ "$direction/$role" == mw/destination_init ]]; then
  2095	        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
  2096	        result=$(win_client_run "$state" "$run_id" "$remote_err" \
  2097	            "$Q_IP:$PORT:/bench/src_$shape" "$dest_arg" "$flag")
  2098	    else
  2099	        session_void "unregistered arm $direction/$role"
  2100	    fi
  2101
  2102	    # Both wrappers carry a q-monotonic completion anchor: immediate child
  2103	    # return for a q client, and result-line arrival for a Windows client.
  2104	    # Wrapper/SSH teardown after that anchor is therefore inside the absolute
  2105	    # settle interval.  The first 250 ms is the common excluded observation
  2106	    # budget; every overrun remains charged to the durable total below.
  2107	    IFS='|' read -r result_tag transfer_ms rc client_done_ns result_extra <<<"$result"
  2108	    if [[ "$result_tag" != R || ! "$transfer_ms" =~ ^[0-9]+$ \
  2109	        || ! "$rc" =~ ^[0-9]+$ || ! "$client_done_ns" =~ ^[0-9]+$ \
  2110	        || -n "$result_extra" ]]; then
  2111	        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
  2112	        session_void "$rid timer/client sentinel malformed: '$result'"
  2113	    fi
  2114	    if [[ "$rc" != 0 ]]; then
  2115	        # Fetch this client log opportunistically; the failure trap also keeps
  2116	        # the remote session tree intact for postmortem evidence.
  2117	        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
  2118	        session_void "$rid client failed rc=$rc (see $client_rel)"
  2119	    fi
  2120
  2121	    anchor_now_ns=$(q_monotonic_ns)
  2122	    [[ "$client_done_ns" -le "$anchor_now_ns" ]] \
  2123	        || session_void "$rid client completion anchor is in the future"
  2124	    [[ $((anchor_now_ns - client_done_ns)) -lt $((SETTLE_MAX_MS * 1000000)) ]] \
  2125	        || session_void "$rid client wrapper teardown already exceeded the settle bound"
  2126	    settle_deadline_ns=$((client_done_ns + SETTLE_NS))
  2127
  2128	    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after
  2129	    settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")
  2130	    [[ "$settle_done_ns" =~ ^[0-9]+$ && "$settle_done_ns" -ge "$settle_deadline_ns" ]] \
  2131	        || session_void "$rid absolute post-client settle returned early: '$settle_done_ns'"
  2132	    settled_ms=$(((settle_done_ns - client_done_ns) / 1000000))
  2133	    [[ "$settled_ms" -ge "$SETTLE_MIN_MS" && "$settled_ms" -lt "$SETTLE_MAX_MS" ]] \
  2134	        || session_void "$rid post-client settle was ${settled_ms}ms, expected [$SETTLE_MIN_MS,$SETTLE_MAX_MS)"
  2135
  2136	    # The destination OS—not the initiator role—selects the durability and
  2137	    # landed-tree probe.  This remains outside transfer_ms.
  2138	    landed_root="src_$shape"
  2139	    landed_manifest="$OUT_DIR/landed/$rid.manifest"
  2140	    canonical_manifest="$OUT_DIR/fixtures/src_$shape.manifest"
  2141	    if [[ "$direction" == wm ]]; then
  2142	        flush_out=$(flush_verify_q "$dest") || session_void "$rid q durability probe failed"
  2143	        write_q_tree_manifest "$dest" "$landed_manifest" "$landed_root" \
  2144	            || session_void "$rid q landed root/manifest verification failed"
  2145	    else
  2146	        flush_out=$(flush_verify_win "$dest") || session_void "$rid Windows durability probe failed"
  2147	        remote_manifest="$WIN_SESSION/block_$block/$rid.tree.manifest"
  2148	        write_win_tree_manifest \
  2149	            "$dest" "$remote_manifest" "$landed_manifest" "$landed_root" \
  2150	            || session_void "$rid Windows landed root/manifest verification failed"
  2151	    fi
  2152	    IFS='|' read -r _ flush_ms count bytes <<<"$flush_out"
  2153	    case "$shape" in mixed) want='5001|547110912';; large) want='1|1073741824';; esac
  2154	    [[ "$count|$bytes" == "$want" ]] \
  2155	        || session_void "$rid landed $count files/$bytes bytes, expected $want"
  2156	    [[ "$flush_ms" =~ ^[0-9]+$ ]] || session_void "$rid flush timer malformed: '$flush_out'"
  2157	    tree_manifest_sha256=$(matching_manifest_digest \
  2158	        "$canonical_manifest" "$landed_manifest") \
  2159	        || session_void "$rid landed relative-path/size manifest differs from canonical"
  2160	    [[ "$tree_manifest_sha256" =~ ^[0-9a-f]{64}$ ]] \
  2161	        || session_void "$rid tree manifest digest is malformed"
  2162	    if [[ "$direction" == wm ]]; then
  2163	        rm -rf -- "$dest" || session_void "$rid failed to remove verified q destination"
  2164	        [[ ! -e "$dest" && ! -L "$dest" ]] \
  2165	            || session_void "$rid verified q destination survived removal"
  2166	    else
  2167	        wssh "
  2168	\$ErrorActionPreference = 'Stop'
  2169	Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
  2170	if (Test-Path -LiteralPath '$dest') { throw 'verified destination survived removal' }
  2171	" \
  2172	            || session_void "$rid failed to remove verified Windows destination"
  2173	    fi
  2174	    arm_phase=durability_verified
  2175
  2176	    if [[ "$windows_client" == 1 ]]; then
  2177	        fetch_successful_windows_client_log "$arm_phase" "$remote_err" "$werr"
  2178	    fi
  2179
  2180	    session_id=$(session_id_from_log "$client_abs") \
  2181	        || session_void "$rid client trace is malformed"
  2182	    if [[ "$state" == on && "$carrier" == tcp ]]; then
  2183	        [[ "$session_id" =~ ^[0-9a-f]{16}$ ]] \
  2184	            || session_void "$rid trace-on TCP client has session_id '$session_id'"
  2185	    else
  2186	        [[ -z "$session_id" ]] \
  2187	            || session_void "$rid emitted TCP phase trace in state=$state carrier=$carrier"
  2188	    fi
  2189
  2190	    total=$((transfer_ms + settled_ms - SETTLE_MIN_MS + flush_ms))
  2191	    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
  2192	        "$block" "$state" "$pass" "$cell" "$role" "$pair" "$role_order" \
  2193	        "$transfer_ms" "$settled_ms" "$flush_ms" "$total" "$landed_root" \
  2194	        "$tree_manifest_sha256" "$rc" "$drain" yes "$run_id" "$session_id" \
  2195	        "$client_rel" >> "$RUNS_CSV"
  2196	    log "$rid: transfer=${transfer_ms}ms settled=${settled_ms}ms flush=${flush_ms}ms total=${total}ms session=${session_id:-none}"
  2197	}
  2198
  2199	cell_order() {
  2200	    local pass="$1" round="$2"
  2201	    local forward='wm_tcp_mixed mw_tcp_mixed wm_grpc_mixed wm_tcp_large'
  2202	    local reverse='wm_tcp_large wm_grpc_mixed mw_tcp_mixed wm_tcp_mixed'
  2203	    local base
  2204	    [[ "$pass" == forward ]] && base="$forward" || base="$reverse"
  2205	    case "$round" in 1|4) printf '%s\n' "$base";; 2|3) [[ "$base" == "$forward" ]] && printf '%s\n' "$reverse" || printf '%s\n' "$forward";; esac

codex
The production flow is internally consistent so far: each arm invokes one `blit copy`; data direction stays fixed while caller role flips; both roles resolve to the same endpoint-local source and destination trees; durability is selected by the destination OS; and the q monotonic arrival stamp charges any wrapper/SSH teardown beyond the common 250 ms budget. I’m finishing with offline-only syntax/self-tests and mutation-focused inspection—no preflight or endpoint commands.
exec
/bin/zsh -lc 'cargo test -p blit-core --test transfer_session_roles session_phase_trace_is_complete_and_inert_under_both_initiators -- --exact' in /Users/michael/Dev/blit_v2
 succeeded in 4523ms:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.81s
     Running tests/transfer_session_roles.rs (target/debug/deps/transfer_session_roles-a5a0e2d3c2f7aafe)

running 1 test
test session_phase_trace_is_complete_and_inert_under_both_initiators ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 0.40s


exec
/bin/zsh -lc "nl -ba crates/blit-cli/src/transfers/remote.rs | sed -n '130,390p'
nl -ba crates/blit-cli/src/transfers/mod.rs | sed -n '180,230p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   130	                    totals.files, totals.bytes, avg_bps
   131	                );
   132	            } else {
   133	                let avg_mib = avg_bps / (1024.0 * 1024.0);
   134	                println!(
   135	                    "[progress] final: {} file(s) transferred \u{2022} {:.2} MiB total \u{2022} {:.2} MiB/s avg",
   136	                    totals.files,
   137	                    totals.bytes as f64 / (1024.0 * 1024.0),
   138	                    avg_mib,
   139	                );
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
   180	        if !confirm_destructive_operation(&prompt, args.yes)? {
   181	            println!("Aborted.");
   182	            return Ok(());
   183	        }
   184	    }
   185
   186	    // Banner goes to stderr so stdout stays reserved for the summary /
   187	    // JSON output. Version dropped — `blit --version` is the right place
   188	    // for that, not every invocation.
   189	    if !args.json {
   190	        eprintln!("starting {} {} -> {}", operation, src_display, dst_display);
   191	        if args.verbose && dst_display != pre_resolve_display {
   192	            eprintln!(
   193	                "  (destination resolved by rsync trailing-slash rule: {} -> {})",
   194	                pre_resolve_display, dst_display
   195	            );
   196	        }
   197	    }
   198
   199	    match select_transfer_route(src_endpoint, dst_endpoint, mode) {
   200	        TransferRoute::LocalToLocal { src, dst, mirror } => {
   201	            if !src.exists() {
   202	                bail!("source path does not exist: {}", src.display());
   203	            }
   204	            run_local_transfer(ctx, args, &src, &dst, mirror)
   205	                .await
   206	                .map(|_| ())
   207	        }
   208	        TransferRoute::LocalToRemote { src, dst, mirror } => {
   209	            if !src.exists() {
   210	                bail!("source path does not exist: {}", src.display());
   211	            }
   212	            ensure_remote_push_supported(args)?;
   213	            ensure_remote_destination_supported(&dst)?;
   214	            run_remote_push_transfer(args, src, dst, mirror).await
   215	        }
   216	        TransferRoute::RemoteToLocal { src, dst, mirror } => {
   217	            ensure_remote_pull_supported(args)?;
   218	            ensure_remote_source_supported(&src)?;
   219	            run_remote_pull_transfer(
   220	                args, src, &dst, mirror, false, // not a move — source survives
   221	            )
   222	            .await
   223	        }
   224	        TransferRoute::RemoteToRemoteDelegated { src, dst, mirror } => {
   225	            ensure_remote_source_supported(&src)?;
   226	            ensure_remote_destination_supported(&dst)?;
   227	            ensure_remote_pull_supported(args)?;
   228	            run_remote_to_remote_direct(args, src, dst, mirror, false /* not a move */).await
   229	        }
   230	    }

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/session_client.rs | sed -n '1,400p'
nl -ba crates/blit-daemon/src/service/core.rs | sed -n '330,410p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "bash -c 'set -e; [[ x == y ]] || exit 41; printf bad' >/dev/null 2>&1; printf 'or_guard_rc=%s\\n' \""'$?"
bash -c '"'set -e; ( true; [[ x == y ]] )' >/dev/null 2>&1; printf 'final_subshell_rc=%s\\n' \""'$?"
bash -c '"'set -e; [[ x == y ]]; printf survived' >/dev/null 2>&1; printf 'bare_guard_rc=%s\\n' \""'$?"' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
or_guard_rc=41
final_subshell_rc=1
bare_guard_rc=0

 succeeded in 0ms:
     1	//! Client-side entry for initiating a unified transfer session.
     2	//!
     3	//! [`run_push_session`] declares the SOURCE role (push-equivalent,
     4	//! otp-4): open the `Transfer` RPC, stream the manifest + payloads; the
     5	//! daemon answers as the DESTINATION Responder. [`run_pull_session`]
     6	//! declares the DESTINATION role (pull-equivalent, otp-5a): the daemon
     7	//! answers as the SOURCE Responder and streams its module tree, which
     8	//! this end diffs and writes. Both build a gRPC-backed [`FrameTransport`]
     9	//! over `BlitClient::transfer` and run the matching role driver; role is
    10	//! carried in `SessionOpen.initiator_role`, never a second code path.
    11	//!
    12	//! Verb wiring: the push-shaped verb (CLI `copy`/`mirror`/`move` to a
    13	//! remote destination, TUI F1 push) rides [`run_push_session`] since
    14	//! otp-10a via `blit_app::transfers::remote::run_remote_push`; the
    15	//! pull-shaped verb (remote source → local destination, TUI F3 pull)
    16	//! rides [`run_pull_session`] since otp-10b-2 via
    17	//! `blit_app::transfers::remote::run_remote_pull`. Both push (otp-4b)
    18	//! and pull (otp-5b) default to the TCP data plane; the in-stream
    19	//! carrier is the requested fallback either direction.
    20
    21	use std::path::PathBuf;
    22	use std::sync::Arc;
    23	use std::time::Duration;
    24
    25	use eyre::{eyre, Result};
    26	use tokio::sync::mpsc;
    27	use tokio_stream::wrappers::ReceiverStream;
    28	use tonic::transport::{Channel, Endpoint};
    29
    30	use crate::generated::blit_client::BlitClient;
    31	use crate::generated::{
    32	    ComparisonMode, FilterSpec, MirrorMode, ResumeSettings, SessionOpen, TransferRole,
    33	    TransferSummary,
    34	};
    35	use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
    36	use crate::remote::transfer::source::TransferSource;
    37	use crate::remote::transfer::{ByteProgressSink, RemoteTransferProgress};
    38	use crate::transfer_plan::PlanOptions;
    39	use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
    40	use crate::transfer_session::{
    41	    run_destination, run_source, DestinationInstruments, DestinationOutcome,
    42	    DestinationSessionConfig, DestinationTarget, HelloConfig, SessionEndpoint, SourceInstruments,
    43	    SourceSessionConfig,
    44	};
    45
    46	/// The push-shaped session options. The full verb surface rides here
    47	/// since otp-10a (mirror, filters, progress, trace); the SOURCE owns
    48	/// the planner knobs, the DESTINATION owns the compare decision.
    49	pub struct PushSessionOptions {
    50	    pub compare_mode: ComparisonMode,
    51	    pub ignore_existing: bool,
    52	    pub require_complete_scan: bool,
    53	    pub plan_options: PlanOptions,
    54	    /// Force the in-stream byte carrier instead of the TCP data plane
    55	    /// (otp-4b). Default `false` = the responder grants a data plane and
    56	    /// payloads ride TCP sockets; `true` is the diagnostics / unreachable
    57	    /// data-plane fallback (`--force-grpc`-shaped).
    58	    pub in_stream_bytes: bool,
    59	    /// otp-7b: negotiate the resume block phase (`SessionOpen.resume`).
    60	    /// Changed dest partials are then patched block-wise instead of
    61	    /// re-transferred whole.
    62	    pub resume: bool,
    63	    /// Requested resume block size in bytes; `0` lets the DESTINATION
    64	    /// choose (currently 1 MiB). The destination clamps to its
    65	    /// carrier's bounds either way. Ignored unless `resume` is true.
    66	    pub resume_block_size: u32,
    67	    /// otp-10a: source-side scan filter, riding `SessionOpen.filter`
    68	    /// (the session honors it since otp-6a — this is the client
    69	    /// wiring; symmetric with [`PullSessionOptions::filter`]). This
    70	    /// SOURCE applies it to its own scan through the universal
    71	    /// `FilteredSource` chokepoint; the DESTINATION uses it to scope
    72	    /// mirror deletions. `None` scans everything.
    73	    pub filter: Option<FilterSpec>,
    74	    /// otp-10a: mirror on the session (otp-6b's one delete rule — the
    75	    /// daemon DESTINATION diffs the complete source manifest against
    76	    /// its tree at SourceDone and deletes extraneous entries locally).
    77	    /// Explicit enabled + scope per the contract; `MirrorMode::Off`
    78	    /// with `mirror_enabled` set is refused at OPEN.
    79	    pub mirror_enabled: bool,
    80	    pub mirror_kind: MirrorMode,
    81	    /// otp-10a: w6-1 progress events from this SOURCE's send side —
    82	    /// need batches as the denominator, `Payload`/`FileComplete` per
    83	    /// file sent on either carrier. The CLI progress line and the TUI
    84	    /// footer consume these exactly as they did from the old driver.
    85	    pub progress: Option<RemoteTransferProgress>,
    86	    /// otp-10a: emit `[data-plane-client]` connect traces on the data
    87	    /// plane sockets this SOURCE dials (`--trace-data-plane`).
    88	    pub trace_data_plane: bool,
    89	}
    90
    91	impl Default for PushSessionOptions {
    92	    fn default() -> Self {
    93	        Self {
    94	            compare_mode: ComparisonMode::SizeMtime,
    95	            ignore_existing: false,
    96	            require_complete_scan: false,
    97	            plan_options: PlanOptions::default(),
    98	            in_stream_bytes: false,
    99	            resume: false,
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
   381	            path: RemotePath::Module {
   382	                module: "m".into(),
   383	                rel_path,
   384	            },
   385	        }
   386	    }
   387
   388	    /// codex otp-10a F2: a rel_path assembled via `PathBuf::join` (the
   389	    /// rsync destination-resolution rule appends the source file name
   390	    /// this way) must reach the wire in POSIX form on every platform —
   391	    /// on Windows the joined form carries a native `\` that would
   392	    /// otherwise land verbatim in `SessionOpen.path`.
   393	    #[test]
   394	    fn joined_rel_path_reaches_the_wire_in_posix_form() {
   395	        let rel = PathBuf::from("sub").join("dir").join("file.txt");
   396	        let (module, path) = endpoint_module_path(&endpoint(rel)).expect("module form resolves");
   397	        assert_eq!(module, "m");
   398	        assert_eq!(path, "sub/dir/file.txt");
   399	    }
   400
   330	        DaemonEvent {
   331	            payload: Some(daemon_event::Payload::TransferError(TransferError {
   332	                transfer_id: guard.transfer_id().to_string(),
   333	                message: error_message.unwrap_or("").to_string(),
   334	            })),
   335	        }
   336	    }
   337	}
   338
   339	#[tonic::async_trait]
   340	impl Blit for BlitService {
   341	    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
   342	    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
   343	    type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
   344	    type SubscribeStream =
   345	        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DaemonEvent, Status>> + Send>>;
   346	    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
   347
   348	    /// ONE_TRANSFER_PATH otp-4a: the daemon serves the unified session
   349	    /// by running `run_destination` as the Responder — the byte
   350	    /// RECEIVER of a client-initiated SOURCE push. Mirrors `push`:
   351	    /// register a jobs row, race the session against cancel/hangup, and
   352	    /// return the response stream immediately (the session runs in the
   353	    /// spawned task, feeding the `ReceiverStream`). Session refusals
   354	    /// travel to the peer as `SessionError` frames; the daemon-specific
   355	    /// module resolution + transport assembly live in `super::transfer`.
   356	    /// Contract: docs/TRANSFER_SESSION.md.
   357	    async fn transfer(
   358	        &self,
   359	        request: Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
   360	    ) -> Result<Response<Self::TransferStream>, Status> {
   361	        let peer = peer_addr_string(&request);
   362	        let modules = Arc::clone(&self.modules);
   363	        let default_root = self.default_root.clone();
   364	        // Operator policy applies to served sessions exactly as it did
   365	        // to the old handlers: --force-grpc-data grants no TCP data
   366	        // plane (codex otp-10a F3); --no-server-checksums refuses
   367	        // Checksum opens (otp-10b-1).
   368	        let policy = blit_core::transfer_session::ResponderPolicy {
   369	            force_in_stream: self.force_grpc_data,
   370	            refuse_checksum_compare: !self.server_checksums_enabled,
   371	        };
   372	        let (tx, rx) = mpsc::channel(32);
   373	        let inbound = request.into_inner();
   374	        let metrics = Arc::clone(&self.metrics);
   375	        let guard = Arc::clone(&metrics).enter_transfer();
   376	        // Jobs row: registered with a Push placeholder and an empty
   377	        // endpoint — the KIND and module/path all arrive in the
   378	        // SessionOpen, mid-handshake inside the session. The on_open
   379	        // hook below (codex otp-10b-2 F4) fixes the row, counts the
   380	        // right metric, and emits the started event the moment the
   381	        // open resolves; the row supports CancelJob throughout.
   382	        let job = self.active_jobs.register(
   383	            ActiveJobKind::Push,
   384	            peer.clone(),
   385	            String::new(),
   386	            String::new(),
   387	        );
   388	        let started = std::time::Instant::now();
   389	        let events_tx = self.events_tx();
   390	        let started_emitted = Arc::new(std::sync::atomic::AtomicBool::new(false));
   391	        let on_open: Arc<super::transfer::OnSessionOpen> = {
   392	            let updater = job.updater();
   393	            let active_jobs = self.active_jobs.clone();
   394	            let events_tx = self.events_tx();
   395	            let metrics = Arc::clone(&self.metrics);
   396	            let peer = peer.clone();
   397	            let started_emitted = Arc::clone(&started_emitted);
   398	            Arc::new(move |kind: ActiveJobKind, module: &str, path: &str| {
   399	                // The metric counts at open-resolve — the first point
   400	                // the daemon knows push- from pull-equivalent. A
   401	                // session refused in the handshake counts nothing
   402	                // (the old dispatchers counted refusals as pushes).
   403	                match kind {
   404	                    ActiveJobKind::Push => metrics.inc_push(),
   405	                    _ => metrics.inc_pull(),
   406	                }
   407	                updater.set_kind_and_endpoint(kind, module.to_string(), path.to_string());
   408	                started_emitted.store(true, std::sync::atomic::Ordering::Release);
   409	                let event = DaemonEvent {
   410	                    payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {

exec
/bin/zsh -lc 'rg -n "N_pair|N_resolution|observer_bias|paired.*range|role-order|OFF.*ON|128|250" docs/plan/OTP12_PERF_FINDINGS.md .review/findings/otp12-pf1-rigw-harness.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/plan/OTP12_PERF_FINDINGS.md:416:  layouts drain the same fixed 128-entry destination need loop, so
docs/plan/OTP12_PERF_FINDINGS.md:442:  opened at one stream (after its 128-file early flush) then resized
.review/findings/otp12-pf1-rigw-harness.md:23:  endpoint is reset and reused by all 128 arms; role-bearing run IDs are kept
.review/findings/otp12-pf1-rigw-harness.md:29:- Run a fixed OFF–ON–ON–OFF four-block schedule over
.review/findings/otp12-pf1-rigw-harness.md:33:  four/four role-first balance (128 timed transfers).
.review/findings/otp12-pf1-rigw-harness.md:48:  the same three after-clock samples and wait only to the absolute +250 ms
.review/findings/otp12-pf1-rigw-harness.md:50:  settle must remain in `[250,1000)` ms and is retained in `runs.csv`.
.review/findings/otp12-pf1-rigw-harness.md:53:  Windows disk writes must drain. The common first 250 ms of post-client
.review/findings/otp12-pf1-rigw-harness.md:57:  registered split drifts, role-order drift, the full paired range that guards
.review/findings/otp12-pf1-rigw-harness.md:59:  `N_resolution`. Reports retain every sorted arm/difference distribution and
.review/findings/otp12-pf1-rigw-harness.md:86:  evidence (128 arms, 768 clock samples, split client/daemon phase logs) and
.review/findings/otp12-pf1-rigw-harness.md:89:  corruption. It pins the split/range/role-order/observer resolution math and
.review/findings/otp12-pf1-rigw-harness.md:93:- Mutation proof: removing role-order drift and the full paired-range term from
.review/findings/otp12-pf1-rigw-harness.md:94:  `N_pair` makes the synthetic diagnostic fail (`N_resolution` falls from 70
.review/findings/otp12-pf1-rigw-harness.md:103:  values outside `[250,1000)`. Synthetic evidence supplies the lower valid
.review/findings/otp12-pf1-rigw-harness.md:106:  `total_ms = transfer_ms + (settled_ms - 250) + flush_ms`, and uses that
.review/findings/otp12-pf1-rigw-harness.md:108:  and resolution-floor value. Only the common first 250 ms remains excluded;
.review/findings/otp12-pf1-rigw-harness.md:132:  exact q/Windows canonical equality and exactly 128 landed manifest files,
.review/findings/otp12-pf1-rigw-harness.md:198:  harness self-test fail because it returns before +250 ms. Moving the

exec
/bin/zsh -lc "nl -ba .review/findings/otp12-pf1-rigw-harness.md | sed -n '1,230p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows
     2
     3	**Slice**: OTP12 performance-finding pf-1, P1 rig harness only.
     4	**Status**: G3 and G4 fixed; fresh complete review pending.
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

exec
/bin/zsh -lc "awk 'NR<283 || NR>1267 {if (/\\[\\[/) print NR \":\" "'$0}'"' scripts/bench_otp12pf_rigw.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
80:    if [[ "$OUTPUT_CLAIMED" == 1 ]]; then
101:    if [[ -e "$target" || -L "$target" ]]; then
102:        if [[ -f "$target/SESSION-COMPLETE" ]]; then
104:        elif [[ -f "$target/SESSION-VOID" ]]; then
152:    if [[ "$CLEANUP_MODE" == 1 ]]; then
164:        if [[ -n "${!name+x}" ]]; then
174:        [[ "$value" == 0 || "$value" == 1 ]] \
176:        if [[ "$value" == 1 ]]; then
180:    [[ "$enabled" -le 1 ]] \
268:    [[ "$1" == durability_verified ]]
1291:    [[ "$auto" == 0 ]] \
1306:    [[ -z "$offenders" ]] || die "q has benchmark-conflicting processes: $offenders"
1309:    [[ "$load" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse q load1 '$load'"
1312:    [[ "$spot" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Spotlight CPU '$spot'"
1332:    [[ "$out" != *BAD\|* ]] || die "Windows has benchmark-conflicting processes: $out"
1334:    [[ "$avg" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Windows CPU from '$out'"
1341:    [[ "$(hostname)" == "$Q_EXPECT_HOST" ]] \
1344:    mtu=$(sed -n 's/.*[[:space:]]mtu[[:space:]]\([0-9][0-9]*\).*/\1/p' <<<"$raw" | head -1)
1345:    media=$(sed -n 's/^[[:space:]]*media:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
1346:    status=$(sed -n 's/^[[:space:]]*status:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
1349:    [[ "$mtu" == "$REGISTERED_MTU" ]] || die "$Q_NIC MTU is $mtu, expected $REGISTERED_MTU"
1350:    [[ "$media" == *"$REGISTERED_MEDIA"* ]] || die "$Q_NIC media is '$media', expected $REGISTERED_MEDIA"
1351:    [[ "$status" == active ]] || die "$Q_NIC status is '$status'"
1356:    [[ "$iface" == "$Q_NIC" ]] || die "q routes $WIN_IP via $iface, expected $Q_NIC"
1357:    [[ "$route_mtu" == "$REGISTERED_MTU" ]] \
1362:    [[ "$peer_mac" == "$(tr 'A-F' 'a-f' <<<"${WIN_MAC//-/:}")" ]] \
1364:    [[ "$peer_mac" != "$Q_MAC" ]] || die "q ARP points at q's own MAC (black-hole host route)"
1380:    [[ "$out" == "W|Up|10 Gbps|10000000000|10000000000|$WIN_MAC|Connected|$REGISTERED_MTU|$WIN_NIC|$WIN_IP" ]] \
1411:    [[ "$qm" == "$Q_TO_WIN_MSS" && "$qip" == "$Q_IP" ]] \
1415:    [[ "$wm" == "$WIN_TO_Q_MSS" && "$wip" == "$WIN_IP" ]] \
1433:    [[ "$out" == "$expected" ]] \
1454:    [[ "$qms" -ge 950 && "$qms" -le 1050 ]] || die "q one-second timer calibrated to ${qms}ms"
1458:    [[ "$wms" -ge 950 && "$wms" -le 1050 ]] || die "Windows one-second timer calibrated to ${wms}ms"
1471:    [[ "$tag" == R && "$ms" == 17 && "$rc" == 0 \
1474:    [[ "$stamp" -ge "$before" && "$stamp" -le "$after" ]] \
1477:    [[ "$teardown_ns" -ge 250000000 ]] \
1594:        [[ "$qgot" == "$want" ]] || die "q src_$shape is $qgot, expected $want"
1595:        [[ "$wgot" == "$want" ]] || die "Windows canonical src_$shape is $wgot, expected $want"
1630:    [[ -n "$EXPECT_SHA" ]] || die "EXPECT_SHA=<full reviewed commit> is required"
1634:    [[ "$EXPECT_SHA" == "$HEAD_FULL" ]] \
1636:    [[ -z $(git -C "$REPO_ROOT" status --porcelain --untracked-files=normal) ]] \
1638:    [[ -x "$Q_BLIT" && -x "$Q_DAEMON" ]] || die "q release binaries are absent"
1677:    [[ -z "$pid" ]] && return 0
1681:        [[ "$cmd" == *"$Q_DAEMON"* ]] \
1696:    if [[ -z "$pid" && -z "$cmdpid" && -n "$current_block" ]]; then
1724:    if [[ -z "$pid" && -z "$cmdpid" ]]; then
1725:        if [[ -n "$current_block" ]] && ! wssh \
1733:    [[ -z "$pid" || "$pid" =~ ^[0-9]+$ ]] \
1735:    [[ -z "$cmdpid" || "$cmdpid" =~ ^[0-9]+$ ]] \
1737:    [[ -n "$current_block" ]] \
1803:    [[ "$remote_hash" == "$local_hash" ]] \
1832:[[module]]
1836:    if [[ "$state" == on ]]; then
1873:  '[[module]]', 'name = \"bench\"', 'path = \"$WIN_MODULE\"'
1910:    [[ "$win_cmd_pid" =~ ^[0-9]+$ && "$win_daemon_pid" =~ ^[0-9]+$ ]] \
1932:        [[ "$remote" =~ ^[0-9]+$ ]] || session_void "clock probe returned '$remote'"
1963:    if [[ "$direction" == wm ]]; then
1965:        [[ ! -e "$dest" && ! -L "$dest" ]] || return 1
1967:        [[ -d "$dest" && ! -L "$dest" ]] || return 1
1969:        [[ -z "$first" ]] || return 1
2012:    if [[ "$state" == on ]]; then
2066:    [[ "$carrier" == grpc ]] && flag=--force-grpc
2082:    if [[ "$direction/$role" == wm/source_init ]]; then
2086:    elif [[ "$direction/$role" == wm/destination_init ]]; then
2090:    elif [[ "$direction/$role" == mw/source_init ]]; then
2094:    elif [[ "$direction/$role" == mw/destination_init ]]; then
2108:    if [[ "$result_tag" != R || ! "$transfer_ms" =~ ^[0-9]+$ \
2111:        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
2114:    if [[ "$rc" != 0 ]]; then
2117:        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
2122:    [[ "$client_done_ns" -le "$anchor_now_ns" ]] \
2124:    [[ $((anchor_now_ns - client_done_ns)) -lt $((SETTLE_MAX_MS * 1000000)) ]] \
2130:    [[ "$settle_done_ns" =~ ^[0-9]+$ && "$settle_done_ns" -ge "$settle_deadline_ns" ]] \
2133:    [[ "$settled_ms" -ge "$SETTLE_MIN_MS" && "$settled_ms" -lt "$SETTLE_MAX_MS" ]] \
2141:    if [[ "$direction" == wm ]]; then
2154:    [[ "$count|$bytes" == "$want" ]] \
2156:    [[ "$flush_ms" =~ ^[0-9]+$ ]] || session_void "$rid flush timer malformed: '$flush_out'"
2160:    [[ "$tree_manifest_sha256" =~ ^[0-9a-f]{64}$ ]] \
2162:    if [[ "$direction" == wm ]]; then
2164:        [[ ! -e "$dest" && ! -L "$dest" ]] \
2176:    if [[ "$windows_client" == 1 ]]; then
2182:    if [[ "$state" == on && "$carrier" == tcp ]]; then
2183:        [[ "$session_id" =~ ^[0-9a-f]{16}$ ]] \
2186:        [[ -z "$session_id" ]] \
2204:    [[ "$pass" == forward ]] && base="$forward" || base="$reverse"
2205:    case "$round" in 1|4) printf '%s\n' "$base";; 2|3) [[ "$base" == "$forward" ]] && printf '%s\n' "$reverse" || printf '%s\n' "$forward";; esac
2215:        [[ "$pair" -le "$last" ]] || session_void "block $block pair schedule overflow"
2246:    [[ -z "$q_daemon_pid" ]] \
2248:    [[ -z "$win_daemon_pid" ]] \
2250:    [[ -z "$win_cmd_pid" ]] \
2252:    [[ -z "$current_block" ]] \
2269:    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
2273:    [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
2301:    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 ]] \
2304:    [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
2306:    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]] \
2308:    [[ ! -e "$OUT_DIR/SESSION-COMPLETE" && ! -L "$OUT_DIR/SESSION-COMPLETE" ]] \
2310:    [[ ! -e "$complete_tmp" && ! -L "$complete_tmp" ]] \
2319:    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
2322:    if [[ "$WIN_SESSION_MAY_EXIST" == 1 ]]; then
2342:    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 \
2345:    lines=${lines//[[:space:]]/}
2346:    [[ "$lines" == 1 && "$(< "$marker")" == "$HEAD_FULL" ]] || return 1
2347:    [[ ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
2349:    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]]
2357:    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
2364:    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 \
2369:    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 ]] \
2374:    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
2379:    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
2385:    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
2391:    if [[ $rc -ne 0 ]]; then
2394:        if [[ ! -s "$OUT_DIR/SESSION-VOID" ]]; then
2398:        if [[ -n "$win_daemon_pid" || -n "$win_cmd_pid" || -n "$current_block" ]]; then
2401:        if [[ -n "$q_daemon_pid" ]]; then q_daemon_stop || true; fi
2402:        if [[ -n "$CLEANUP_ERROR" ]]; then
2413:    if [[ "$SELFTEST" == 1 ]]; then selftest; return; fi
2421:    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
2426:    if [[ "$LAUNCHER_SMOKE" == 1 ]]; then
2458:if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then

exec
/bin/zsh -lc "nl -ba crates/blit-cli/src/cli.rs | sed -n '160,380p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   160	    pub json: bool,
   161	}
   162
   163	#[derive(Subcommand)]
   164	pub enum DiagnosticsCommand {
   165	    /// Manage performance history capture (enable/disable/clear)
   166	    Perf(PerfArgs),
   167	    /// Emit a diagnostic snapshot for a SRC -> DEST invocation (no transfer performed)
   168	    Dump(DiagnosticsDumpArgs),
   169	}
   170
   171	#[derive(Args, Clone, Debug)]
   172	pub struct DiagnosticsDumpArgs {
   173	    /// Source path or remote endpoint (same syntax as `blit copy`)
   174	    pub source: String,
   175	    /// Destination path or remote endpoint (same syntax as `blit copy`)
   176	    pub destination: String,
   177	    /// Emit JSON instead of the default human-readable report
   178	    #[arg(long)]
   179	    pub json: bool,
   180	}
   181
   182	#[derive(Args, Clone, Debug)]
   183	pub struct PerfArgs {
   184	    /// Number of recent records to display (0 = all)
   185	    #[arg(long, default_value_t = 50)]
   186	    pub limit: usize,
   187	    /// Enable performance history capture
   188	    #[arg(long, conflicts_with = "disable")]
   189	    pub enable: bool,
   190	    /// Disable performance history capture
   191	    #[arg(long, conflicts_with = "enable")]
   192	    pub disable: bool,
   193	    /// Remove the stored performance history file
   194	    #[arg(long)]
   195	    pub clear: bool,
   196	    /// Output as JSON
   197	    #[arg(long)]
   198	    pub json: bool,
   199	}
   200
   201	#[derive(Args, Clone, Debug)]
   202	#[command(after_long_help = PATH_SEMANTICS_HELP)]
   203	pub struct TransferArgs {
   204	    /// Source path or remote endpoint (host:/module/path).
   205	    ///
   206	    /// Trailing slash means "copy contents" (merge). Without a trailing slash,
   207	    /// the source directory is nested under the destination (if destination is
   208	    /// a container) or used as the exact target (otherwise).
   209	    pub source: String,
   210	    /// Destination path or remote endpoint.
   211	    ///
   212	    /// Trailing slash means "into this directory" (container). See `blit(1)`
   213	    /// for the full rsync-style resolution rules.
   214	    pub destination: String,
   215
   216	    // -- Common options (no heading — rendered in the default "Options"
   217	    // section so first-time users see them at the top).
   218	    /// Perform a dry run without making changes
   219	    #[arg(long)]
   220	    pub dry_run: bool,
   221	    /// Keep verbose transfer logs
   222	    #[arg(long, short = 'v')]
   223	    pub verbose: bool,
   224	    /// Show an interactive progress indicator.
   225	    ///
   226	    /// Auto-enabled when stdout is a TTY (and --json is not set) so
   227	    /// interactive users get feedback by default; piping/redirecting
   228	    /// stdout disables it so scripts aren't affected. Use this flag to
   229	    /// force-enable when stdout is not a TTY (e.g. under `tee`).
   230	    #[arg(long, short = 'p')]
   231	    pub progress: bool,
   232	    /// Skip confirmation prompt for destructive operations (mirror deletions, move)
   233	    #[arg(long, short = 'y')]
   234	    pub yes: bool,
   235	    /// Output as JSON. With -p, emits NDJSON progress to stderr. Final
   236	    /// transfer summary is written to stdout as a JSON object.
   237	    #[arg(long)]
   238	    pub json: bool,
   239
   240	    // -- Comparison options: how blit decides which files to transfer.
   241	    /// Force checksum comparison of files (slower but more accurate)
   242	    #[arg(long, short = 'c', help_heading = "Comparison")]
   243	    pub checksum: bool,
   244	    /// Compare only by size, ignoring modification time
   245	    #[arg(long, conflicts_with = "checksum", help_heading = "Comparison")]
   246	    pub size_only: bool,
   247	    /// Transfer all files unconditionally, ignoring size and modification time
   248	    #[arg(long, conflicts_with_all = ["checksum", "size_only"], help_heading = "Comparison")]
   249	    pub ignore_times: bool,
   250	    /// Skip files that already exist on the destination (regardless of differences)
   251	    #[arg(long, conflicts_with = "force", help_heading = "Comparison")]
   252	    pub ignore_existing: bool,
   253	    /// Force exact mirror even if destination files are newer (dangerous)
   254	    #[arg(long, help_heading = "Comparison")]
   255	    pub force: bool,
   256	    /// Mirror deletion scope: `subset` (default) deletes only files in the
   257	    /// source filter scope; `all` deletes any destination file absent from
   258	    /// the (filtered) source set, including files that wouldn't have been
   259	    /// transferred in the first place. `all` is destructive — use with
   260	    /// caution.
   261	    #[arg(long, value_name = "SCOPE", default_value = "subset", value_parser = ["subset", "all"], help_heading = "Comparison")]
   262	    pub delete_scope: String,
   263
   264	    // -- Reliability options: recovery + retries.
   265	    /// Resume interrupted transfers using block-level comparison
   266	    #[arg(long, help_heading = "Reliability")]
   267	    pub resume: bool,
   268	    /// Retry the transfer up to N times on a transient failure (network
   269	    /// drop, stall timeout). 0 (default) disables retries. Because
   270	    /// transfers are resumable, each retry continues rather than
   271	    /// restarts.
   272	    #[arg(
   273	        long,
   274	        value_name = "N",
   275	        default_value_t = 0,
   276	        help_heading = "Reliability"
   277	    )]
   278	    pub retry: u32,
   279	    /// Seconds to wait between retries (see --retry).
   280	    #[arg(
   281	        long,
   282	        value_name = "SECS",
   283	        default_value_t = 5,
   284	        help_heading = "Reliability"
   285	    )]
   286	    pub wait: u64,
   287
   288	    // -- Filtering: restrict which files are eligible for transfer.
   289	    // Filters apply identically to all source/destination combinations
   290	    // (local-local, push, pull, remote-remote) — they live on the
   291	    // pipeline's TransferSource so every path enforces them.
   292	    /// Exclude files matching this glob pattern (repeatable)
   293	    #[arg(long, action = clap::ArgAction::Append, value_name = "PATTERN", help_heading = "Filtering")]
   294	    pub exclude: Vec<String>,
   295	    /// Include only files matching this glob pattern (repeatable). When set,
   296	    /// any include match is required; excludes still apply on top.
   297	    #[arg(long, action = clap::ArgAction::Append, value_name = "PATTERN", help_heading = "Filtering")]
   298	    pub include: Vec<String>,
   299	    /// Only transfer files listed in FILE (one relative path per line, # comments allowed)
   300	    #[arg(long, value_name = "FILE", help_heading = "Filtering")]
   301	    pub files_from: Option<PathBuf>,
   302	    /// Minimum file size to transfer (e.g. 100K, 10M, 1G)
   303	    #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
   304	    pub min_size: Option<String>,
   305	    /// Maximum file size to transfer (e.g. 1G, 500M)
   306	    #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
   307	    pub max_size: Option<String>,
   308	    /// Only transfer files older than this duration (e.g. 1h, 7d, 30m)
   309	    #[arg(long, value_name = "DURATION", help_heading = "Filtering")]
   310	    pub min_age: Option<String>,
   311	    /// Only transfer files newer than this duration (e.g. 1h, 7d, 30m)
   312	    #[arg(long, value_name = "DURATION", help_heading = "Filtering")]
   313	    pub max_age: Option<String>,
   314
   315	    // -- Performance / debug knobs — niche, kept at the bottom so new
   316	    // users aren't distracted by them.
   317	    /// Force gRPC control-plane data path instead of hybrid TCP
   318	    #[arg(long, help_heading = "Performance / debug")]
   319	    pub force_grpc: bool,
   320	    /// Fire-and-forget: hand the transfer to the destination
   321	    /// daemon and exit as soon as it starts.
   322	    ///
   323	    /// The CLI awaits the daemon's `Started` event (which
   324	    /// includes the daemon-assigned `transfer_id`), prints
   325	    /// it plus a `blit jobs cancel` hint, and returns. The
   326	    /// destination daemon completes the transfer regardless
   327	    /// of CLI connection state. Useful for long remote→remote
   328	    /// transfers that should outlive the operator's shell.
   329	    ///
   330	    /// Only valid for remote→remote transfers (the daemon-to-daemon
   331	    /// delegated byte path), and not for `blit move`.
   332	    ///
   333	    /// Rejected with a clear error for:
   334	    /// - local-source or local-destination transfers (CLI is in
   335	    ///   the byte path)
   336	    /// - `blit move` (the source-delete step needs the CLI to
   337	    ///   await transfer completion)
   338	    #[arg(long)]
   339	    pub detach: bool,
   340	    /// Discard all writes — local copy only (read+pipeline benchmark).
   341	    ///
   342	    /// Reads and prepares all source data normally but does not write to
   343	    /// the destination. Use this to isolate whether a bottleneck is on
   344	    /// the source or destination side. Example:
   345	    ///
   346	    ///   blit copy /data/large-dataset /tmp/unused --null -v
   347	    ///
   348	    /// **Restrictions** (R54-F1): --null is supported only by `blit copy`
   349	    /// between two local paths. The CLI rejects it with `blit mirror`
   350	    /// (the destination-purge step would still delete files), with
   351	    /// `blit move` (the source-delete step would erase the source with
   352	    /// no copy), and with any remote endpoint (the remote push/pull
   353	    /// paths don't honor the null sink, so the flag would be silently
   354	    /// ignored).
   355	    ///
   356	    /// Performance-history records are tagged (`null_sink` lane) so
   357	    /// real-transfer profiling never learns from null-sink runs.
   358	    #[arg(long, help_heading = "Performance / debug")]
   359	    pub null: bool,
   360
   361	    // -- Hidden flags (don't appear in --help).
   362	    /// Limit worker threads (advanced debugging only)
   363	    #[arg(long, hide = true)]
   364	    pub workers: Option<usize>,
   365	    /// Emit verbose TCP data-plane diagnostics (advanced debugging only)
   366	    #[arg(long, hide = true)]
   367	    pub trace_data_plane: bool,
   368	}
   369
   370	impl TransferArgs {
   371	    /// Effective progress setting: explicit `--progress` wins; otherwise
   372	    /// enable automatically when stdout is a TTY and `--json` is off. This
   373	    /// matches rsync/rclone/restic defaults so a first-time interactive
   374	    /// user isn't staring at a silent terminal for 60+ seconds on a big
   375	    /// transfer, while piped/redirected stdout keeps scripts unaffected.
   376	    pub fn effective_progress(&self) -> bool {
   377	        if self.progress {
   378	            return true;
   379	        }
   380	        if self.json {

exec
/bin/zsh -lc 'rg -n "retry|Retry" crates/blit-cli/src/cli.rs crates/blit-cli/src/transfers crates/blit-app/src/transfers crates/blit-core/src/remote/transfer/session_client.rs | head -n 260' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-cli/src/cli.rs:268:    /// Retry the transfer up to N times on a transient failure (network
crates/blit-cli/src/cli.rs:270:    /// transfers are resumable, each retry continues rather than
crates/blit-cli/src/cli.rs:278:    pub retry: u32,
crates/blit-cli/src/cli.rs:279:    /// Seconds to wait between retries (see --retry).
crates/blit-cli/src/cli.rs:620:    /// retry-wait: the `--retry`/`--wait` flags parse, default to no
crates/blit-cli/src/cli.rs:623:    fn retry_wait_flags_parse_and_default() {
crates/blit-cli/src/cli.rs:628:        assert_eq!(args.retry, 0, "retry defaults to 0 (no retries)");
crates/blit-cli/src/cli.rs:632:            Cli::try_parse_from(["blit", "copy", "--retry", "3", "--wait", "10", "src", "dst"])
crates/blit-cli/src/cli.rs:637:        assert_eq!(args.retry, 3);
crates/blit-app/src/transfers/retry.rs:1://! Retry-with-wait for transfers (owner-approved robocopy-style
crates/blit-app/src/transfers/retry.rs:2://! `--retry`/`--wait`). Part 1: the retryable-error classifier and the
crates/blit-app/src/transfers/retry.rs:3://! generic retry loop. Part 2 wires the CLI flags and the transfer
crates/blit-app/src/transfers/retry.rs:6://! This is viable because blit transfers are **resumable** — a retry
crates/blit-app/src/transfers/retry.rs:8://! missing/changed files, so a retry continues rather than restarts. The
crates/blit-app/src/transfers/retry.rs:10://! fast, retryable failure this loop catches.
crates/blit-app/src/transfers/retry.rs:17:// w5-2: the classifier moved to blit-core (single owner of retry
crates/blit-app/src/transfers/retry.rs:20:pub use blit_core::remote::retry::is_retryable;
crates/blit-app/src/transfers/retry.rs:24:/// only when [`is_retryable`] accepts the error; a fatal error returns
crates/blit-app/src/transfers/retry.rs:25:/// immediately. `retries == 0` reproduces the no-retry default.
crates/blit-app/src/transfers/retry.rs:27:/// The transfer's resumability means each retry continues the prior
crates/blit-app/src/transfers/retry.rs:39:                if attempt_no >= retries || !is_retryable(&err) {
crates/blit-app/src/transfers/retry.rs:44:                    "blit: transfer failed, retrying ({attempt_no}/{retries}) in {}s: {err:#}",
crates/blit-app/src/transfers/retry.rs:66:    fn classifies_transient_io_as_retryable() {
crates/blit-app/src/transfers/retry.rs:67:        assert!(is_retryable(&io_err(io::ErrorKind::TimedOut)));
crates/blit-app/src/transfers/retry.rs:68:        assert!(is_retryable(&io_err(io::ErrorKind::ConnectionReset)));
crates/blit-app/src/transfers/retry.rs:69:        assert!(is_retryable(&io_err(io::ErrorKind::BrokenPipe)));
crates/blit-app/src/transfers/retry.rs:70:        assert!(is_retryable(&io_err(io::ErrorKind::UnexpectedEof)));
crates/blit-app/src/transfers/retry.rs:74:    fn classifies_fatal_errors_as_not_retryable() {
crates/blit-app/src/transfers/retry.rs:76:        assert!(!is_retryable(&eyre::eyre!("path escapes module root")));
crates/blit-app/src/transfers/retry.rs:78:        assert!(!is_retryable(&io_err(io::ErrorKind::PermissionDenied)));
crates/blit-app/src/transfers/retry.rs:79:        assert!(!is_retryable(&io_err(io::ErrorKind::NotFound)));
crates/blit-app/src/transfers/retry.rs:83:    async fn retries_a_retryable_failure_then_succeeds() {
crates/blit-app/src/transfers/retry.rs:102:    async fn does_not_retry_a_fatal_failure() {
crates/blit-app/src/transfers/retry.rs:122:        assert_eq!(calls.get(), 1, "retries=0 ⇒ no retries even if retryable");
crates/blit-app/src/transfers/retry.rs:126:    async fn exhausts_retry_budget_then_returns_last_error() {
crates/blit-cli/src/transfers/mod.rs:602:            retry: 0,
crates/blit-cli/src/transfers/mod.rs:650:            retry: 0,
crates/blit-cli/src/transfers/mod.rs:704:            retry: 0,
crates/blit-app/src/transfers/mod.rs:20:pub mod retry;
crates/blit-cli/src/transfers/remote_remote_direct.rs:323:            retry: 0,
crates/blit-app/src/transfers/remote.rs:632:                         Upgrade the destination daemon, or retry without --detach."

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/session_phase.rs | sed -n '1,360p'
nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '170,245p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	//! Low-frequency, wire-neutral timing events for transfer-session probes.
     2	//!
     3	//! This is deliberately separate from `--trace-data-plane`, whose
     4	//! per-file human-readable output is too intrusive for performance work.
     5	//! Production emission is enabled with `BLIT_TRACE_SESSION_PHASES=1` and
     6	//! correlated across processes with `BLIT_TRACE_RUN_ID`. Tests may inject
     7	//! an in-memory emitter through [`SessionPhaseTrace::capture`].
     8
     9	use serde::Serialize;
    10	use std::io::Write;
    11	use std::sync::atomic::{AtomicU64, Ordering};
    12	use std::sync::Arc;
    13	use std::time::{Instant, SystemTime, UNIX_EPOCH};
    14
    15	const TRACE_ENV: &str = "BLIT_TRACE_SESSION_PHASES";
    16	const RUN_ID_ENV: &str = "BLIT_TRACE_RUN_ID";
    17
    18	/// Semantic role of this endpoint in the transfer. This is intentionally
    19	/// independent of which endpoint initiated the connection.
    20	#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    21	#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    22	pub enum SessionPhaseRole {
    23	    Source,
    24	    Destination,
    25	}
    26
    27	/// One structured timing event. Optional correlation fields are omitted
    28	/// from JSON rather than written as nulls so rig logs stay compact.
    29	#[derive(Clone, Debug, Serialize)]
    30	pub struct SessionPhaseEvent {
    31	    pub schema: u8,
    32	    pub run_id: String,
    33	    pub session_id: String,
    34	    pub producer_seq: u64,
    35	    pub unix_ns: u128,
    36	    pub elapsed_ns: u128,
    37	    pub endpoint_role: SessionPhaseRole,
    38	    pub initiator_role: SessionPhaseRole,
    39	    pub event: &'static str,
    40	    #[serde(skip_serializing_if = "Option::is_none")]
    41	    pub action: Option<&'static str>,
    42	    #[serde(skip_serializing_if = "Option::is_none")]
    43	    pub epoch: Option<u32>,
    44	    #[serde(skip_serializing_if = "Option::is_none")]
    45	    pub socket: Option<u32>,
    46	    #[serde(skip_serializing_if = "Option::is_none")]
    47	    pub batch: Option<u64>,
    48	    #[serde(skip_serializing_if = "Option::is_none")]
    49	    pub count: Option<u64>,
    50	    #[serde(skip_serializing_if = "Option::is_none")]
    51	    pub target_streams: Option<u32>,
    52	    #[serde(skip_serializing_if = "Option::is_none")]
    53	    pub live_streams: Option<u32>,
    54	    #[serde(skip_serializing_if = "Option::is_none")]
    55	    pub accepted: Option<bool>,
    56	}
    57
    58	/// Optional event fields shared by the small set of phase hooks.
    59	#[derive(Clone, Copy, Debug, Default)]
    60	pub(crate) struct SessionPhaseFields {
    61	    pub(crate) action: Option<&'static str>,
    62	    pub(crate) epoch: Option<u32>,
    63	    pub(crate) socket: Option<u32>,
    64	    pub(crate) batch: Option<u64>,
    65	    pub(crate) count: Option<u64>,
    66	    pub(crate) target_streams: Option<u32>,
    67	    pub(crate) live_streams: Option<u32>,
    68	    pub(crate) accepted: Option<bool>,
    69	}
    70
    71	type EventEmitter = dyn Fn(SessionPhaseEvent) + Send + Sync + 'static;
    72	type FlushEmitter = dyn Fn() + Send + Sync + 'static;
    73
    74	enum PhaseWriterOutput {
    75	    Line(String),
    76	    Flush,
    77	}
    78
    79	#[derive(Clone)]
    80	struct TraceEmitter {
    81	    run_id: Arc<str>,
    82	    emit: Arc<EventEmitter>,
    83	    flush: Arc<FlushEmitter>,
    84	}
    85
    86	/// An unbound phase-event sink carried by source/destination instruments.
    87	/// The default remains inactive unless the explicit process-level probe
    88	/// environment flag is set.
    89	#[derive(Clone)]
    90	pub struct SessionPhaseTrace {
    91	    emitter: Option<TraceEmitter>,
    92	    allow_env: bool,
    93	}
    94
    95	impl Default for SessionPhaseTrace {
    96	    fn default() -> Self {
    97	        Self {
    98	            emitter: None,
    99	            allow_env: true,
   100	        }
   101	    }
   102	}
   103
   104	impl SessionPhaseTrace {
   105	    /// Build a deterministic capture sink for integration tests and local
   106	    /// diagnostic harnesses. This does not consult process environment.
   107	    pub fn capture(
   108	        run_id: impl Into<String>,
   109	        emit: impl Fn(SessionPhaseEvent) + Send + Sync + 'static,
   110	    ) -> Self {
   111	        Self {
   112	            emitter: Some(TraceEmitter {
   113	                run_id: Arc::from(run_id.into()),
   114	                emit: Arc::new(emit),
   115	                flush: Arc::new(|| {}),
   116	            }),
   117	            allow_env: false,
   118	        }
   119	    }
   120
   121	    /// Force tracing off even when the process-level probe flag is set.
   122	    /// Used by the trace-on/off behavior guard.
   123	    pub fn disabled() -> Self {
   124	        Self {
   125	            emitter: None,
   126	            allow_env: false,
   127	        }
   128	    }
   129
   130	    /// Preserve an injected sink; otherwise enable the low-frequency JSONL
   131	    /// emitter when the debug environment flag is truthy.
   132	    pub(crate) fn or_from_env(self) -> Self {
   133	        self.or_from_env_with(|name| std::env::var(name).ok(), Self::stderr_writer)
   134	    }
   135
   136	    fn or_from_env_with(
   137	        self,
   138	        read: impl FnMut(&str) -> Option<String>,
   139	        writer: impl FnOnce(String) -> Self,
   140	    ) -> Self {
   141	        if self.emitter.is_some() || !self.allow_env {
   142	            return self;
   143	        }
   144	        let Some(run_id) = trace_env_run_id(read) else {
   145	            return self;
   146	        };
   147	        writer(run_id)
   148	    }
   149
   150	    fn stderr_writer(run_id: String) -> Self {
   151	        let stderr = std::io::stderr();
   152	        Self::threaded_writer(run_id, move |output| {
   153	            let mut stderr = stderr.lock();
   154	            match output {
   155	                PhaseWriterOutput::Line(line) => {
   156	                    let _ = writeln!(stderr, "{line}");
   157	                }
   158	                PhaseWriterOutput::Flush => {
   159	                    let _ = stderr.flush();
   160	                }
   161	            }
   162	        })
   163	    }
   164
   165	    fn threaded_writer(
   166	        run_id: String,
   167	        mut output: impl FnMut(PhaseWriterOutput) + Send + 'static,
   168	    ) -> Self {
   169	        enum WriterMessage {
   170	            Event(SessionPhaseEvent),
   171	            Flush(std::sync::mpsc::SyncSender<()>),
   172	        }
   173
   174	        let (tx, rx) = std::sync::mpsc::channel::<WriterMessage>();
   175	        let spawned = std::thread::Builder::new()
   176	            .name("blit-session-phase".into())
   177	            .spawn(move || {
   178	                while let Ok(message) = rx.recv() {
   179	                    match message {
   180	                        WriterMessage::Event(event) => {
   181	                            output(PhaseWriterOutput::Line(json_line(&event)));
   182	                        }
   183	                        WriterMessage::Flush(done) => {
   184	                            output(PhaseWriterOutput::Flush);
   185	                            let _ = done.send(());
   186	                        }
   187	                    }
   188	                }
   189	            });
   190	        if spawned.is_err() {
   191	            return Self::disabled();
   192	        }
   193	        let event_tx = tx.clone();
   194	        let flush_tx = tx;
   195	        Self {
   196	            emitter: Some(TraceEmitter {
   197	                run_id: Arc::from(run_id),
   198	                emit: Arc::new(move |event| {
   199	                    let _ = event_tx.send(WriterMessage::Event(event));
   200	                }),
   201	                flush: Arc::new(move || {
   202	                    let (done_tx, done_rx) = std::sync::mpsc::sync_channel(0);
   203	                    if flush_tx.send(WriterMessage::Flush(done_tx)).is_ok() {
   204	                        let _ = done_rx.recv();
   205	                    }
   206	                }),
   207	            }),
   208	            allow_env: false,
   209	        }
   210	    }
   211
   212	    pub(crate) fn bind(
   213	        &self,
   214	        session_token: &[u8],
   215	        endpoint_role: SessionPhaseRole,
   216	        initiator_role: SessionPhaseRole,
   217	    ) -> Option<BoundSessionPhaseTrace> {
   218	        let emitter = self.emitter.clone()?;
   219	        let digest = blake3::hash(session_token).to_hex();
   220	        Some(BoundSessionPhaseTrace {
   221	            emitter,
   222	            session_id: Arc::from(&digest.as_str()[..16]),
   223	            endpoint_role,
   224	            initiator_role,
   225	            origin: Instant::now(),
   226	            producer_seq: Arc::new(AtomicU64::new(0)),
   227	        })
   228	    }
   229	}
   230
   231	fn trace_env_run_id(mut read: impl FnMut(&str) -> Option<String>) -> Option<String> {
   232	    let enabled = read(TRACE_ENV).is_some_and(|value| {
   233	        matches!(
   234	            value.trim().to_ascii_lowercase().as_str(),
   235	            "1" | "true" | "yes" | "on"
   236	        )
   237	    });
   238	    enabled.then(|| read(RUN_ID_ENV).unwrap_or_else(|| format!("pid-{}", std::process::id())))
   239	}
   240
   241	fn json_line(event: &SessionPhaseEvent) -> String {
   242	    match serde_json::to_string(event) {
   243	        Ok(line) => format!("[session-phase] {line}"),
   244	        Err(err) => format!("[session-phase] serialization_error={err}"),
   245	    }
   246	}
   247
   248	/// A phase trace bound to one endpoint of one TCP transfer session.
   249	#[derive(Clone)]
   250	pub(crate) struct BoundSessionPhaseTrace {
   251	    emitter: TraceEmitter,
   252	    session_id: Arc<str>,
   253	    endpoint_role: SessionPhaseRole,
   254	    initiator_role: SessionPhaseRole,
   255	    origin: Instant,
   256	    producer_seq: Arc<AtomicU64>,
   257	}
   258
   259	pub(crate) struct SessionPhaseStamp {
   260	    instant: Instant,
   261	    unix_ns: u128,
   262	    producer_seq: u64,
   263	}
   264
   265	impl BoundSessionPhaseTrace {
   266	    pub(crate) fn event(&self, event: &'static str, fields: SessionPhaseFields) {
   267	        self.emit_at(self.stamp(), event, fields);
   268	    }
   269
   270	    pub(crate) fn stamp(&self) -> SessionPhaseStamp {
   271	        SessionPhaseStamp {
   272	            instant: Instant::now(),
   273	            unix_ns: SystemTime::now()
   274	                .duration_since(UNIX_EPOCH)
   275	                .unwrap_or_default()
   276	                .as_nanos(),
   277	            producer_seq: self.producer_seq.fetch_add(1, Ordering::Relaxed),
   278	        }
   279	    }
   280
   281	    /// Capture the queue-admission time before releasing the payload to a
   282	    /// worker, then emit it after the send commits. This preserves the
   283	    /// causal timestamp without putting logging on the queue's critical
   284	    /// path before the payload exists.
   285	    pub(crate) fn first_payload_queued_at(&self, at: SessionPhaseStamp) {
   286	        self.emit_at(at, "first_payload_queued", SessionPhaseFields::default());
   287	    }
   288
   289	    pub(crate) fn socket_first_write(&self, epoch: u32, socket: u32) {
   290	        self.event(
   291	            "first_socket_write",
   292	            SessionPhaseFields {
   293	                epoch: Some(epoch),
   294	                socket: Some(socket),
   295	                ..Default::default()
   296	            },
   297	        );
   298	    }
   299
   300	    pub(crate) fn socket_first_payload_received(&self, epoch: u32, socket: u32) {
   301	        self.event(
   302	            "first_payload_received",
   303	            SessionPhaseFields {
   304	                epoch: Some(epoch),
   305	                socket: Some(socket),
   306	                ..Default::default()
   307	            },
   308	        );
   309	    }
   310
   311	    pub(crate) fn flush(&self) {
   312	        (self.emitter.flush)();
   313	    }
   314
   315	    fn emit_at(&self, at: SessionPhaseStamp, event: &'static str, fields: SessionPhaseFields) {
   316	        let record = SessionPhaseEvent {
   317	            schema: 1,
   318	            run_id: self.emitter.run_id.to_string(),
   319	            session_id: self.session_id.to_string(),
   320	            producer_seq: at.producer_seq,
   321	            unix_ns: at.unix_ns,
   322	            elapsed_ns: at.instant.saturating_duration_since(self.origin).as_nanos(),
   323	            endpoint_role: self.endpoint_role,
   324	            initiator_role: self.initiator_role,
   325	            event,
   326	            action: fields.action,
   327	            epoch: fields.epoch,
   328	            socket: fields.socket,
   329	            batch: fields.batch,
   330	            count: fields.count,
   331	            target_streams: fields.target_streams,
   332	            live_streams: fields.live_streams,
   333	            accepted: fields.accepted,
   334	        };
   335	        (self.emitter.emit)(record);
   336	    }
   337	}
   338
   339	#[cfg(test)]
   340	mod tests {
   341	    use super::*;
   342	    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
   343	    use std::sync::{Arc, Mutex};
   344
   345	    #[test]
   346	    fn production_env_config_uses_the_canonical_flag_and_run_id() {
   347	        let mut requested = Vec::new();
   348	        let run_id = trace_env_run_id(|name| {
   349	            requested.push(name.to_string());
   350	            match name {
   351	                TRACE_ENV => Some("true".into()),
   352	                RUN_ID_ENV => Some("rig-w-pair-7".into()),
   353	                _ => None,
   354	            }
   355	        });
   356	        assert_eq!(run_id.as_deref(), Some("rig-w-pair-7"));
   357	        assert_eq!(
   358	            requested,
   359	            vec![TRACE_ENV.to_string(), RUN_ID_ENV.to_string()]
   360	        );
   170	        stream: TcpStream,
   171	        trace: bool,
   172	        chunk_bytes: usize,
   173	        payload_prefetch: usize,
   174	        pool: Arc<BufferPool>,
   175	        probe: P,
   176	    ) -> Self {
   177	        let payload_prefetch = payload_prefetch.max(1);
   178	        let chunk_bytes = chunk_bytes.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
   179	        Self {
   180	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
   181	            pool,
   182	            trace,
   183	            chunk_bytes,
   184	            payload_prefetch,
   185	            bytes_sent: 0,
   186	            probe,
   187	            phase_trace: None,
   188	            phase_epoch: 0,
   189	            phase_socket: 0,
   190	            phase_write_armed: false,
   191	        }
   192	    }
   193
   194	    pub(crate) fn with_session_phase_trace(
   195	        mut self,
   196	        trace: Option<super::session_phase::BoundSessionPhaseTrace>,
   197	        epoch: u32,
   198	        socket: u32,
   199	    ) -> Self {
   200	        if let Some(trace) = &trace {
   201	            trace.event(
   202	                "socket_trace_attached",
   203	                super::session_phase::SessionPhaseFields {
   204	                    epoch: Some(epoch),
   205	                    socket: Some(socket),
   206	                    ..Default::default()
   207	                },
   208	            );
   209	        }
   210	        self.phase_write_armed = trace.is_some();
   211	        self.phase_trace = trace;
   212	        self.phase_epoch = epoch;
   213	        self.phase_socket = socket;
   214	        self
   215	    }
   216
   217	    fn take_first_payload_write_trace(
   218	        &mut self,
   219	    ) -> Option<super::session_phase::BoundSessionPhaseTrace> {
   220	        if !self.phase_write_armed {
   221	            return None;
   222	        }
   223	        self.phase_write_armed = false;
   224	        let trace = self.phase_trace.clone()?;
   225	        trace.event(
   226	            "socket_write_begin",
   227	            super::session_phase::SessionPhaseFields {
   228	                epoch: Some(self.phase_epoch),
   229	                socket: Some(self.phase_socket),
   230	                ..Default::default()
   231	            },
   232	        );
   233	        Some(trace)
   234	    }
   235
   236	    pub async fn send_payloads(
   237	        &mut self,
   238	        source: Arc<dyn TransferSource>,
   239	        payloads: Vec<TransferPayload>,
   240	    ) -> Result<()> {
   241	        self.send_payloads_with_progress(source, payloads, None)
   242	            .await
   243	    }
   244
   245	    pub async fn send_payloads_with_progress(

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '640,720p'
nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1040,1110p'
nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2790,2850p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
  1040	    let negotiated = establish(
  1041	        &mut transport,
  1042	        &cfg.hello,
  1043	        &cfg.endpoint,
  1044	        TransferRole::Source,
  1045	        &source_open_validator,
  1046	        // run_source only ever resolves nothing: a SOURCE *initiator*
  1047	        // owns its own root, and a SOURCE *responder* driven directly
  1048	        // (the in-process role suite) is handed a Fixed source. The
  1049	        // daemon SOURCE responder resolves module→root inside
  1050	        // `run_responder`, not here (otp-5).
  1051	        None,
  1052	    )
  1053	    .await?;
  1054
  1055	    drive_source(
  1056	        cfg.plan_options,
  1057	        cfg.data_plane_host,
  1058	        cfg.instruments,
  1059	        negotiated,
  1060	        transport,
  1061	        source,
  1062	    )
  1063	    .await
  1064	}
  1065
  1066	/// The SOURCE session body after establish: spawn the receive half,
  1067	/// run the send half, and map a fault to a peer-notified report. Shared
  1068	/// by [`run_source`] (initiator or direct-responder) and
  1069	/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
  1070	/// choreography is single-sourced.
  1071	async fn drive_source(
  1072	    plan_options: PlanOptions,
  1073	    data_plane_host: Option<String>,
  1074	    instruments: SourceInstruments,
  1075	    mut negotiated: Negotiated,
  1076	    transport: FrameTransport,
  1077	    source: Arc<dyn TransferSource>,
  1078	) -> Result<TransferSummary> {
  1079	    let phase_trace = bind_session_phase_trace(
  1080	        instruments.session_phase_trace.clone(),
  1081	        &negotiated,
  1082	        SessionPhaseRole::Source,
  1083	    );
  1084	    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
  1085	    // its send sockets on; a SOURCE initiator (push) has none and dials the
  1086	    // grant it received instead. Take it here so the send half owns it.
  1087	    let responder_data_plane = negotiated.responder_data_plane.take();
  1088	    let (mut tx, rx) = transport.split();
  1089	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
  1090	    // Set by the send half the moment ManifestComplete goes out. On
  1091	    // an ordered transport, a NeedComplete arriving while this is
  1092	    // still false is provably premature — the peer cannot have
  1093	    // received what we have not sent (contract: NeedComplete only
  1094	    // after ManifestComplete received + all entries diffed).
  1095	    let manifest_sent = Arc::new(AtomicBool::new(false));
  1096	    let (event_tx, event_rx) = mpsc::unbounded_channel();
  1097	    // Fault side-channel (codex otp-8 F1): the in-stream send path
  1098	    // races this signal against blocked record sends; see
  1099	    // `SourceEventSender`.
  1100	    let (fault_tx, fault_rx) = watch::channel(None::<SessionFault>);
  1101	    // AbortOnDrop: an early error return below must abort the receive
  1102	    // half instead of leaking it (same rationale as design-2 / w4-1).
  1103	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
  1104	        rx,
  1105	        Arc::clone(&sent),
  1106	        Arc::clone(&manifest_sent),
  1107	        resume_negotiated(&negotiated.open),
  1108	        // otp-10a: the recv half owns need-batch arrival, which is the
  1109	        // push-direction progress denominator (contract on
  1110	        // `ProgressEvent::ManifestBatch`: "push: need-list batches").
  2790	            deleted_dirs += 1;
  2791	            continue;
  2792	        }
  2793	        #[cfg(windows)]
  2794	        crate::win_fs::clear_readonly_recursive(dir);
  2795	        match std::fs::remove_dir(dir) {
  2796	            Ok(()) => deleted_dirs += 1,
  2797	            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
  2798	            // FilteredSubset: the dir still holds out-of-scope files the
  2799	            // filter excluded from enumeration; leaving it is the scope
  2800	            // contract, not a failure (engine/mirror.rs R58-F6). `Some(66)`
  2801	            // is ENOTEMPTY on macOS/BSD, which maps to a different ErrorKind.
  2802	            Err(e)
  2803	                if tolerate_nonempty_dirs
  2804	                    && (e.kind() == std::io::ErrorKind::DirectoryNotEmpty
  2805	                        || e.raw_os_error() == Some(66)) => {}
  2806	            Err(e) => return Err(eyre::eyre!("mirror delete dir {}: {e}", dir.display())),
  2807	        }
  2808	    }
  2809	    Ok((deleted_files, deleted_dirs))
  2810	}
  2811
  2812	async fn destination_session(
  2813	    transport: &mut FrameTransport,
  2814	    negotiated: Negotiated,
  2815	    dst_root: &Path,
  2816	    data_plane_host: Option<&str>,
  2817	    instruments: DestinationInstruments,
  2818	    local_apply: Option<local::LocalApply>,
  2819	) -> Result<DestinationOutcome> {
  2820	    let phase_trace = bind_session_phase_trace(
  2821	        instruments.session_phase_trace.clone(),
  2822	        &negotiated,
  2823	        SessionPhaseRole::Destination,
  2824	    );
  2825	    // otp-10b-2: the receive side's w6-1 progress lane. Need batches are
  2826	    // the denominator (reported where they're emitted, below); per-file
  2827	    // events ride each carrier's record handling.
  2828	    let progress = instruments.progress;
  2829	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  2830	        .unwrap_or(ComparisonMode::Unspecified);
  2831	    // Session deletions run via the otp-6b mirror pass (a whole-tree
  2832	    // diff at SourceDone), never a per-entry flag.
  2833	    let compare_opts = CompareOptions {
  2834	        mode: compare_mode.into(),
  2835	        ignore_existing: negotiated.open.ignore_existing,
  2836	    };
  2837	    // src_root is only consumed by local File payloads, which never
  2838	    // occur on a WIRE session destination (payload bytes arrive as
  2839	    // records and go through the stream/tar write paths); the LOCAL
  2840	    // carrier (otp-11) brings its own fully-configured sink, where
  2841	    // File payloads are the point. `Arc` so the data-plane receive
  2842	    // task (otp-4b) can share the one sink across sockets.
  2843	    let sink: Arc<dyn TransferSink> = match &local_apply {
  2844	        Some(la) => Arc::clone(&la.sink),
  2845	        None => {
  2846	            let mut sink = FsTransferSink::new(
  2847	                PathBuf::new(),
  2848	                dst_root.to_path_buf(),
  2849	                FsSinkConfig {
  2850	                    preserve_times: true,

exec
/bin/zsh -lc 'rg -n "env::var|std::env|BLIT_|RAYON|TOKIO_WORKER|MAX_STREAM" crates/blit-core/src crates/blit-cli/src crates/blit-daemon/src | head -n 320' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-cli/src/cli.rs:49:    /// Replaces the pre-0.1.1 `BLIT_TEST_COUNTER_FILE` env var
crates/blit-daemon/src/service/admin.rs:401:    // Pattern matching is glob-based, matching `BLIT_UTILS_PLAN.md`.
crates/blit-core/src/remote/instrumentation.rs:9://! `BLIT_TEST_COUNTER_FILE` env var instead of the CLI flag. Env vars
crates/blit-cli/src/main.rs:47:    // audit-l39: pre-0.1.1 this was BLIT_TEST_COUNTER_FILE. Env vars
crates/blit-core/src/remote/transfer/session_phase.rs:5://! Production emission is enabled with `BLIT_TRACE_SESSION_PHASES=1` and
crates/blit-core/src/remote/transfer/session_phase.rs:6://! correlated across processes with `BLIT_TRACE_RUN_ID`. Tests may inject
crates/blit-core/src/remote/transfer/session_phase.rs:15:const TRACE_ENV: &str = "BLIT_TRACE_SESSION_PHASES";
crates/blit-core/src/remote/transfer/session_phase.rs:16:const RUN_ID_ENV: &str = "BLIT_TRACE_RUN_ID";
crates/blit-core/src/remote/transfer/session_phase.rs:133:        self.or_from_env_with(|name| std::env::var(name).ok(), Self::stderr_writer)
crates/blit-daemon/src/runtime.rs:292:            let cwd = std::env::current_dir().context("failed to determine working directory")?;
crates/blit-cli/src/diagnostics.rs:178:        "invocation": std::env::args().collect::<Vec<_>>(),
crates/blit-core/src/transfer_session/mod.rs:136:/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
crates/blit-core/src/transfer_session/mod.rs:139:    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
crates/blit-core/src/transfer_session/mod.rs:223:    /// `BLIT_TRACE_SESSION_PHASES=1` on each endpoint.
crates/blit-core/src/stderr_log.rs:49:/// Resolve the max level from a `BLIT_LOG` value (`off|error|warn|info|
crates/blit-core/src/stderr_log.rs:60:/// Default max level is warn; `BLIT_LOG` overrides. Idempotent: only the
crates/blit-core/src/stderr_log.rs:65:        log::set_max_level(level_from_env(std::env::var("BLIT_LOG").ok().as_deref()));
crates/blit-core/src/mdns.rs:15:pub const BLIT_SERVICE_TYPE: &str = "_blit._tcp.local.";
crates/blit-core/src/mdns.rs:159:        BLIT_SERVICE_TYPE,
crates/blit-core/src/mdns.rs:189:        .browse(BLIT_SERVICE_TYPE)
crates/blit-core/src/mdns.rs:222:    let _ = daemon.stop_browse(BLIT_SERVICE_TYPE);
crates/blit-core/src/mdns.rs:239:        .strip_suffix(BLIT_SERVICE_TYPE)
crates/blit-core/src/dial.rs:40:pub const DIAL_FLOOR_MAX_STREAMS: usize = 8;
crates/blit-core/src/dial.rs:46:pub const DIAL_CEILING_MAX_STREAMS: usize = 32;
crates/blit-core/src/dial.rs:77:        max_streams: DIAL_CEILING_MAX_STREAMS as u32,
crates/blit-core/src/dial.rs:92:        .unwrap_or(DIAL_CEILING_MAX_STREAMS)
crates/blit-core/src/dial.rs:93:        .clamp(1, DIAL_CEILING_MAX_STREAMS)
crates/blit-core/src/dial.rs:304:            max_streams: AtomicUsize::new(DIAL_FLOOR_MAX_STREAMS.clamp(1, ceiling_streams.max(1))),
crates/blit-core/src/dial.rs:854:        assert_eq!(dial.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
crates/blit-core/src/dial.rs:878:        assert_eq!(generous.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_rigw_analyze_test.py | sed -n '419,665p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   419	    def test_complete_schedule_exact_floor_bias_and_exports(self) -> None:
   420	        temporary, session = self.make_session()
   421	        self.addCleanup(temporary.cleanup)
   422	        result = analyzer.analyze(session.root)
   423	        self.assertEqual(str(result.observer_bias), "20")
   424	        self.assertEqual(str(result.n_resolution), "70")
   425	        with result.summary_csv.open(newline="") as handle:
   426	            rows = {
   427	                (row["cell"], row["trace_state"]): row
   428	                for row in csv.DictReader(handle)
   429	            }
   430	        off = rows[(analyzer.TARGET_CELL, "off")]
   431	        on = rows[(analyzer.TARGET_CELL, "on")]
   432	        self.assertEqual(off["measurand"], "durable_total_ms")
   433	        self.assertEqual(off["delta_ms"], "45")
   434	        self.assertEqual(off["paired_delta_median_ms"], "45")
   435	        self.assertEqual(off["first4_delta_median_ms"], "25")
   436	        self.assertEqual(off["last4_delta_median_ms"], "65")
   437	        self.assertEqual(off["first_last_drift_ms"], "40")
   438	        self.assertEqual(off["odd_even_drift_ms"], "10")
   439	        self.assertEqual(off["source_first_delta_median_ms"], "45")
   440	        self.assertEqual(off["destination_first_delta_median_ms"], "45")
   441	        self.assertEqual(off["role_order_drift_ms"], "0")
   442	        self.assertEqual(off["n_pair_split_ms"], "40")
   443	        self.assertEqual(off["paired_delta_range_ms"], "70")
   444	        self.assertEqual(off["n_pair_ms"], "70")
   445	        self.assertEqual(on["delta_ms"], "25")
   446	        self.assertEqual(on["n_pair_split_ms"], "10")
   447	        self.assertEqual(on["n_pair_ms"], "10")
   448	        self.assertEqual(on["observer_bias_ms"], "20")
   449	        self.assertEqual(on["n_resolution_ms"], "70")
   450	        self.assertTrue(result.summary_md.is_file())
   451	        self.assertTrue(result.distributions_csv.is_file())
   452	        with result.clock_summary_csv.open(newline="") as handle:
   453	            clocks = list(csv.DictReader(handle))
   454	        self.assertEqual(len(clocks), 128)
   455	        self.assertTrue(all(row["before_sample"] == "1" for row in clocks))
   456	        self.assertTrue(all(row["after_sample"] == "1" for row in clocks))
   457	        self.assertTrue(all(row["selected_offset_change_ns"] == "100" for row in clocks))
   458	        with result.phase_events_csv.open(newline="") as handle:
   459	            phase_rows = list(csv.DictReader(handle))
   460	        self.assertEqual(len(phase_rows), len(session.events))
   461	        self.assertTrue(any(row["source_file"].startswith("client/") for row in phase_rows))
   462	        self.assertTrue(any(row["source_file"].startswith("trace/") for row in phase_rows))
   463	        self.assertTrue(
   464	            all(
   465	                row["total_ms"]
   466	                == str(
   467	                    int(row["transfer_ms"])
   468	                    + int(row["settled_ms"])
   469	                    - analyzer.SETTLE_MIN_MS
   470	                    + int(row["flush_ms"])
   471	                )
   472	                for row in phase_rows
   473	            )
   474	        )
   475	        with result.phase_intervals_csv.open(newline="") as handle:
   476	            intervals = list(csv.DictReader(handle))
   477	        self.assertTrue(intervals)
   478	        self.assertTrue(all(int(row["duration_ns"]) >= 0 for row in intervals))
   479	        self.assertTrue(all(row["endpoint_role"] in {"SOURCE", "DESTINATION"} for row in intervals))
   480
   481	    def test_registered_schedule_is_pair_outer_with_reverse_block_bases(self) -> None:
   482	        schedule = analyzer.expected_schedule()
   483
   484	        def cells_for(block_number: int, pair: int) -> list[str]:
   485	            return [
   486	                cell
   487	                for block, cell, scheduled_pair, _role, role_order in schedule
   488	                if block.number == block_number
   489	                and scheduled_pair == pair
   490	                and role_order == 1
   491	            ]
   492
   493	        base = list(analyzer.CELLS)
   494	        reverse = list(reversed(base))
   495	        self.assertEqual(cells_for(1, 1), base)
   496	        self.assertEqual(cells_for(1, 2), reverse)
   497	        self.assertEqual(cells_for(2, 1), reverse)
   498	        self.assertEqual(cells_for(2, 2), base)
   499	        self.assertEqual(cells_for(3, 5), base)
   500	        self.assertEqual(cells_for(4, 5), reverse)
   501	        self.assertEqual(
   502	            [
   503	                role
   504	                for block, _cell, pair, role, role_order in schedule
   505	                if block.number == 1 and role_order == 1 and _cell == base[0]
   506	            ],
   507	            ["source_init", "destination_init", "destination_init", "source_init"],
   508	        )
   509
   510	    def test_missing_trace_endpoint_is_rejected(self) -> None:
   511	        temporary, session = self.make_session()
   512	        self.addCleanup(temporary.cleanup)
   513	        first_session = next(event["session_id"] for event in session.events)
   514	        session.events = [
   515	            event
   516	            for event in session.events
   517	            if not (
   518	                event["session_id"] == first_session
   519	                and event["endpoint_role"] == "DESTINATION"
   520	            )
   521	        ]
   522	        session.write()
   523	        with self.assertRaisesRegex(analyzer.AnalysisError, "missing endpoint role"):
   524	            analyzer.analyze(session.root)
   525
   526	    def test_trace_off_leak_is_rejected(self) -> None:
   527	        temporary, session = self.make_session()
   528	        self.addCleanup(temporary.cleanup)
   529	        leaked = dict(session.events[0])
   530	        leaked["run_id"] = "rigw-block-1"
   531	        leaked["session_id"] = "ffffffffffffffff"
   532	        session.events.append(leaked)
   533	        session.write()
   534	        with self.assertRaisesRegex(analyzer.AnalysisError, "trace leak: trace-off block 1"):
   535	            analyzer.analyze(session.root)
   536
   537	    def test_grpc_trace_leak_is_rejected(self) -> None:
   538	        temporary, session = self.make_session()
   539	        self.addCleanup(temporary.cleanup)
   540	        leaked = dict(session.events[0])
   541	        leaked["run_id"] = "rigw-block-2"
   542	        leaked["session_id"] = "eeeeeeeeeeeeeeee"
   543	        session.events.append(leaked)
   544	        session.write()
   545	        with self.assertRaisesRegex(analyzer.AnalysisError, "possible gRPC"):
   546	            analyzer.analyze(session.root)
   547
   548	    def test_schedule_mismatch_is_rejected(self) -> None:
   549	        temporary, session = self.make_session()
   550	        self.addCleanup(temporary.cleanup)
   551	        session.rows[0]["cell"] = "wm_tcp_large"
   552	        session.write()
   553	        with self.assertRaisesRegex(analyzer.AnalysisError, "schedule mismatch"):
   554	            analyzer.analyze(session.root)
   555
   556	    def test_settled_ms_schema_and_bounds_are_fail_closed(self) -> None:
   557	        for value in ("249", "1000", "not-an-integer"):
   558	            with self.subTest(settled_ms=value):
   559	                temporary, session = self.make_session()
   560	                self.addCleanup(temporary.cleanup)
   561	                session.rows[0]["settled_ms"] = value
   562	                session.write()
   563	                with self.assertRaisesRegex(analyzer.AnalysisError, "settled_ms"):
   564	                    analyzer.analyze(session.root)
   565
   566	        temporary, session = self.make_session()
   567	        self.addCleanup(temporary.cleanup)
   568	        with (session.root / "runs.csv").open() as handle:
   569	            lines = handle.readlines()
   570	        lines[0] = lines[0].replace("settled_ms,", "")
   571	        (session.root / "runs.csv").write_text("".join(lines))
   572	        with self.assertRaisesRegex(analyzer.AnalysisError, "header mismatch"):
   573	            analyzer.analyze(session.root)
   574
   575	    def test_corrupt_total_is_rejected(self) -> None:
   576	        temporary, session = self.make_session()
   577	        self.addCleanup(temporary.cleanup)
   578	        session.rows[0]["total_ms"] = "999"
   579	        session.write()
   580	        with self.assertRaisesRegex(
   581	            analyzer.AnalysisError,
   582	            "total_ms must equal transfer_ms \\+ \\(settled_ms - 250\\) \\+ flush_ms",
   583	        ):
   584	            analyzer.analyze(session.root)
   585
   586	    def test_role_specific_flush_is_included_in_delta_and_floor(self) -> None:
   587	        temporary, session = self.make_session()
   588	        self.addCleanup(temporary.cleanup)
   589	        destination_flush = (18, 16, 14, 12, 10, 8, 6, 4)
   590	        for row in session.rows:
   591	            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
   592	                continue
   593	            flush_ms = (
   594	                10
   595	                if row["role"] == "source_init"
   596	                else destination_flush[int(row["pair"]) - 1]
   597	            )
   598	            row["flush_ms"] = str(flush_ms)
   599	            row["total_ms"] = str(
   600	                int(row["transfer_ms"])
   601	                + int(row["settled_ms"])
   602	                - analyzer.SETTLE_MIN_MS
   603	                + flush_ms
   604	            )
   605	        session.write()
   606
   607	        result = analyzer.analyze(session.root)
   608	        with result.summary_csv.open(newline="") as handle:
   609	            rows = {
   610	                (row["cell"], row["trace_state"]): row
   611	                for row in csv.DictReader(handle)
   612	            }
   613	        off = rows[(analyzer.TARGET_CELL, "off")]
   614	        self.assertEqual(off["delta_ms"], "46")
   615	        self.assertEqual(off["paired_delta_median_ms"], "46")
   616	        self.assertEqual(off["paired_delta_range_ms"], "56")
   617	        self.assertEqual(off["n_pair_ms"], "56")
   618	        self.assertEqual(str(result.n_resolution), "56")
   619
   620	    def test_excess_settle_is_charged_without_false_role_delta(self) -> None:
   621	        temporary, session = self.make_session()
   622	        self.addCleanup(temporary.cleanup)
   623	        old_formula_totals: dict[str, set[int]] = {
   624	            "source_init": set(),
   625	            "destination_init": set(),
   626	        }
   627	        actual_elapsed: set[int] = set()
   628	        for row in session.rows:
   629	            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
   630	                continue
   631	            transfer_ms = 100
   632	            if row["role"] == "source_init":
   633	                settled_ms, flush_ms = 999, 1
   634	            else:
   635	                settled_ms, flush_ms = 250, 750
   636	            row["transfer_ms"] = str(transfer_ms)
   637	            row["settled_ms"] = str(settled_ms)
   638	            row["flush_ms"] = str(flush_ms)
   639	            row["total_ms"] = str(
   640	                transfer_ms
   641	                + settled_ms
   642	                - analyzer.SETTLE_MIN_MS
   643	                + flush_ms
   644	            )
   645	            old_formula_totals[row["role"]].add(transfer_ms + flush_ms)
   646	            actual_elapsed.add(transfer_ms + settled_ms + flush_ms)
   647	        self.assertEqual(actual_elapsed, {1100})
   648	        self.assertEqual(old_formula_totals["source_init"], {101})
   649	        self.assertEqual(old_formula_totals["destination_init"], {850})
   650	        session.write()
   651
   652	        result = analyzer.analyze(session.root)
   653	        with result.summary_csv.open(newline="") as handle:
   654	            rows = {
   655	                (row["cell"], row["trace_state"]): row
   656	                for row in csv.DictReader(handle)
   657	            }
   658	        off = rows[(analyzer.TARGET_CELL, "off")]
   659	        self.assertEqual(off["source_init_median_ms"], "850")
   660	        self.assertEqual(off["destination_init_median_ms"], "850")
   661	        self.assertEqual(off["delta_ms"], "0")
   662	        self.assertEqual(off["paired_delta_median_ms"], "0")
   663	        self.assertEqual(off["paired_delta_range_ms"], "0")
   664	        self.assertEqual(off["n_pair_ms"], "0")
   665

exec
/bin/zsh -lc "rg -n \"B.?≥|T/2|resolution|within-session|observer|instrument\" docs/plan/OTP12_PERF_FINDINGS.md | sed -n '1,200p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
222:  counterfactual and the rig-side instrumented run (Method 2).
319:  and that grade stands as registered; the *resolution limit* is stated here so
331:- **This does NOT prove the interleaved design has enough resolution** — that is
332:  a different (paired, within-session) variance, and pf-0 did not measure it.
333:  **pf-1 must measure its own paired within-session noise floor on the
334:  unmodified build and register a resolution check** (its smallest reportable
360:The resolution — re-record each rig's baseline at MTU 9000 and re-freeze —
515:1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
524:   high-volume probe slice so they cannot perturb the focused P1 observer.
534:   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
546:   configuration runs an instrumentation-on/off pair to bound observer
551:   instrument toggles missed the tar-shard path — instrument the
593:   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
622:    requires pf-1 to measure its own **paired within-session** floor before
652:- Every measurement runs instrumentation-on/off pairs (per-member tracing
699:- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
722:  *instrument* — it does not waive an acceptance bar. Rig D's TCP
762:  measurable cost and pf-1 need not instrument the delegated trigger

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '300,350p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   300	point estimate of the MTU contribution to P1 is ~0. The null is **not vacuous**
   301	— the manipulation demonstrably reached the wire (`wm_tcp_large` ran **3–4%
   302	faster at jumbo on both arms**, and both `wm_tcp_mixed` arms sped up slightly) —
   303	and the benefit is **symmetric**, which is why it cannot explain an
   304	**asymmetry**. P1 FAILED in all four sessions (1.237–1.362) regardless of MTU;
   305	all controls passed in all four.
   306
   307	**What it does NOT license (do not restate this result as more than it is).**
   308	- **The wire is not exonerated, and "P1 is code-shaped" is NOT established
   309	  here.** MTU is *one* environmental variable. Segment **fill** is unmeasured
   310	  (8948 is the MSS *ceiling*), so underfilled segments, a bottleneck elsewhere,
   311	  or a smaller wire contribution are all still live. This result kills **MTU**,
   312	  not "the environment".
   313	- **It is not powered to exclude a CONTRIBUTING-size MTU effect.** The
   314	  CONFIRMED-CONTRIBUTING threshold is 20% of Δ_P1 ≈ **46 ms**, which is
   315	  **below the rig's measured between-session noise floor of 78 ms**. So the
   316	  experiment can exclude a **DOMINANT** effect (50% ≈ 114 ms, comfortably above
   317	  the floor) but **cannot exclude a contributing-size one** — a 46 ms effect
   318	  could be swamped. The registered rule returns KILLED on the point estimate,
   319	  and that grade stands as registered; the *resolution limit* is stated here so
   320	  the grade is never read as a stronger exclusion than the data supports.
   321	- It confirms no hypothesis. pf-1 still owns attribution.
   322
   323	**`Δ_P1(rig W)` is re-estimated, and the noise floor constrains how pf-1 may
   324	grade.** The `282 ms` above is a **single nagatha session**; four sessions on
   325	the `q` pairing give **Δ_P1 ≈ 230 ms** (229 at 1500, 236 at 9000).
   326
   327	- **Between-session grading of a counterfactual is now definitively ruled out**
   328	  on this rig: a 46 ms (20%) recovery is smaller than the 78 ms between-session
   329	  floor, so an unpaired before/after across sessions cannot separate
   330	  CONTRIBUTING from KILLED.
   331	- **This does NOT prove the interleaved design has enough resolution** — that is
   332	  a different (paired, within-session) variance, and pf-0 did not measure it.
   333	  **pf-1 must measure its own paired within-session noise floor on the
   334	  unmodified build and register a resolution check** (its smallest reportable
   335	  recovery must exceed that floor) *before* grading any hypothesis. A pf-1
   336	  recovery quoted without its paired floor is uninterpretable.
   337	- **The noise is not diffuse — it is a bistable fast arm.** The `win_init` runs
   338	  are **bimodal** (roughly ~730 ms and ~840 ms clusters); S1 drew 6 low/2 high
   339	  and S4 drew 2 low/6 high **at the same MTU**, and that mixture — not MTU — is
   340	  what produced the 72 ms `win_init` replicate spread and hence N_Δ. The
   341	  `mac_init` arm is by contrast stable to **5–6 ms**. **Trap for pf-1: a
   342	  counterfactual that merely shifts the mode mixture would masquerade as a
   343	  recovery.** Grade on the run distribution, not the median alone. (The MTU
   344	  verdict is robust to this: pooling all 16 runs per condition gives
   345	  Δ_9000 = 232, Δ_1500 = 221.5, r = −4.7% — same KILLED grade.)
   346
   347	**RESOLVED — the committed baselines are RE-RECORDED at MTU 9000
   348	(D-2026-07-14-1, owner, 2026-07-14).** The exposure pf-0 surfaced: the committed
   349	anti-drift ceilings were recorded at **MTU 1500**, before the fabric went jumbo,
   350	and **a reference must share the MTU of the sessions graded against it.**

exec
/bin/zsh -lc 'rg -n "D-2026-07-14-4|B.?≥.?T/2|T/2|B <" docs/DECISIONS.md docs/plan .review/findings/otp12-pf1-rigw-harness.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/DECISIONS.md:219:- What the rule now IS (rev 8): per cell, the paired ABBA differences `d_i = destinit_i − srcinit_i`, their median, and one **exact order-statistic CI** (coverage ≥95%). One threshold `T = min(10% of the srcinit median, Δ_ref = 230 ms)` — the effect must matter by the project's own invariance bar **and** be the size of the one rig W measured. Four cell states, mutually exclusive **by construction** (no labels a new case can walk past): `EFFECT` (CI_lo ≥ +T), `INVERTED` (CI_hi ≤ −T), `NONE` (CI strictly inside ±T — a genuine equivalence result), `UNCLEAR` (anything else). **Controls must all be `NONE` at the tighter `T/2`**, or no verdict about the measurand is read at all.
docs/DECISIONS.md:224:## D-2026-07-14-4 — a rig whose permitted bias reaches HALF the effect threshold is NOT CLEAN: the session refuses to grade
docs/DECISIONS.md:225:- Decision: on any measurand cell, if the residual arm bias the controls could not exclude (`B`) reaches **`T/2`**, the session verdict is **`CONTROLS-NOT-CLEAN`** and **no verdict is read** — not a reproduction, and not a null. Owner, 2026-07-14, choosing between "refuse to grade" and "grade it anyway, flag it", verbatim: **"Refuse to grade"**.
docs/DECISIONS.md:226:- The defect it closes (round-11 codex, HIGH, `.review/results/macmac-harness-r11.codex-engine.md`; grok found the same dead-zone independently): `T` is **capped** at Δ_ref = 230 ms, but the bias a *clean* control is permitted to carry is a **fraction of its arm** (≤5%), and that fraction is scaled onto whatever arm it is applied to. On a slow measurand the two diverge: with clean controls at `+49 ms` on a 1000 ms arm (4.9%), a measurand at `src = 10000 ms` gets `T = 230` but `B = 490`. Then `T − B < 0`, so **a null is arithmetically impossible**, while `T + B` still licenses an `EFFECT` of which **up to 68% is permitted rig bias** — at a ratio of only 1.072, i.e. inside the project's own invariance bar. A confidently wrong reproduction, off a rig certified clean.
docs/DECISIONS.md:227:- Why refusing beats flagging: it is the same principle the rule already applies to a dirty control — *a noisy rig is fixed by a quieter rig, not by grading it with an asterisk*. `B ≥ T/2` means the rig's own permitted noise is at least half the effect being hunted; nothing read off it can be attributed. Cost accepted: a marginal rig yields **no answer** and must be quietened and re-run.

exec
/bin/zsh -lc "nl -ba docs/DECISIONS.md | sed -n '195,232p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   195	- Decision: the frozen committed baselines that `pf-final` grades against are **re-recorded with their OLD builds at MTU 9000**, so acceptance compares old and new like-for-like on the fabric the fleet actually runs. Owner, 2026-07-14, choosing between three presented options, verbatim: **"Re-record the baseline at 9000"**. The 2026-07-10 baselines are **retained as historical MTU-1500 records** — superseded as the acceptance reference, never deleted or rewritten.
   196	- **REVISED 2026-07-14 after codex review (`.review/results/pf-0-rebaseline-decision.*`; NOT READY, 6 findings, 6/6 accepted).** The owner's choice stands unchanged; the first draft of this entry was **not executable** and its rationale was over-applied. Corrections are folded in below and marked. The revision does not reopen the decision — it makes it performable.
   197	- Why (**corrected — the first draft over-applied pf-0**): pf-0's "3–4% faster at jumbo" is measured on **one cell (`wm_tcp_large`), one rig (W), and both arms of the NEW build** — it is **not** a measured old-vs-new leniency, and pf-0 measured **no** small cells, **no** rig-Z cells, and **no** OLD-build MTU response at all (its own committed-reference rows were VOID at jumbo). So the justification is **not** "the ceiling is loose by 3–4%" — that number cannot be generalized across cells or rigs. It is **methodological**: *an acceptance reference and the sessions graded against it must share the MTU of the fabric under test.* pf-0 proves the mismatch is real and that MTU moves wall time on at least one cell; that alone makes a mismatched ceiling unsound in an unknown direction. The **known** direction, where measured, is lenient — which is the wrong error for a bar guarding the one class of finding (P1/P2) between blit and shipping.
   198	- Scope — **BOTH rigs, not just rig W.** Each harness hardcodes its own committed reference, and both predate the 2026-07-13 fabric-wide jumbo raise: rig W `scripts/bench_otp12_win.sh:105` → `docs/bench/otp2w-baseline-2026-07-10/`; rig Z `scripts/bench_otp12_zoey.sh:102` → `docs/bench/otp2-baseline-2026-07-10/`. **Verified, not assumed** (2026-07-14): netwatch-01 "ran at MTU 1500 for EVERY benchmark ever recorded" (`.agents/machines.md`), and zoey's pre-jumbo `systemd-networkd` configs — backed up as `*.premtu`, dated 2026-04-30 — carry **no `MTUBytes` stanza**, i.e. the default 1500; the 9000 configs were written 2026-07-13. Rig D (delegated) has **no** old baseline and is unaffected.
   199	- **THE NON-LOOSENING GUARD (added on review — without it this decision breaks the very control it amends).** `OTP12_ACCEPTANCE_RUN.md` D2 exists precisely so that *"the fixed pre-cutover bar must not be loosened by a slower old rerun"* (its design finding F2). A re-record re-rolls **hardware, OS/disk state and day** as well as MTU — rig W's Mac end is now `q`, not the nagatha that recorded 2026-07-10 — so an unguarded re-record could **loosen** the bar, which is exactly what F2 forbids. Therefore, applying F2 (not inventing a new rule): **the acceptance reference for each cell is the per-cell MINIMUM of {the 2026-07-10 committed median, the re-recorded 9000 median}.** It can only tighten, never loosen. Any cell whose re-record is **slower** than 2026-07-10 is **flagged for investigation, never silently adopted** — the old build getting slower on faster hardware would mean the rig or the method drifted, and that must be explained before any acceptance run is graded.
   200	- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
   201	  * **Rig W** re-records on `0f922de` (its original old build), provenance manifest-verified.
   202	  * **Rig Z has NO clean "original old build" to reuse** (caught on review): the otp-2 baseline's *client* was a clean `e757dcc` but the *daemon* it actually ran was a **dirty** `731023b` build — which D1/D6's clean-matched-pair discipline forbids reusing. Resolution: rig Z re-records on a **CLEAN `e757dcc` pair**, which is sound because `git diff 731023b e757dcc -- crates proto Cargo.toml Cargo.lock` is **empty** (the committed daemon code is identical — otp-2 README correction), and because otp-12a **already** staged a clean `e757dcc` rebuild for its old arm, so this is precedent, not a new reference build.
   203	  * `BASELINE_SUMMARY` is hardcoded **by design** (no override) so a run cannot quietly re-point its own ceiling. Re-pointing it is therefore a reviewed source edit, not an env var — and the new value must be a **committed** dated dir.
   204	  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
   205	- Supersedes: the *pin* in `OTP12_ACCEPTANCE_RUN.md` **D2 and D5** (both name the committed **2026-07-10** median; D2 was missed in the first draft, leaving the two sections contradicting each other — caught on review). The **freeze principle stands**: a baseline is immutable once recorded, no run may re-point its own reference, and **the bar can never be loosened** (the guard above). What changes is only *which* frozen record the harness grades against, once. The 2026-07-10 baselines are **retained, unmodified, as historical MTU-1500 records** and their READMEs are re-labelled accordingly. Closes the OPEN item raised in `OTP12_PERF_FINDINGS.md` §pf-0.
   206
   207	## D-2026-07-14-2 — a SECOND reviewer (grok) may be added to the loop for hard calls; codex remains the default
   208	- Decision: the review loop may run a **second, independent model (`grok`)** alongside codex on high-stakes slices. Owner, 2026-07-14, verbatim: **"Reviewloop grok for another opinion"**. Codex remains the **default and mandatory** reviewer; grok is **additive, never a substitute**, and never runs alone.
   209	- Why the original rule said otherwise, and why this does not break it: `docs/agent/GPT_REVIEW_LOOP.md` says "Codex is the only reviewer... do not add same-model self-review panels, Claude subagent reviewers, or any other substitute". That rule exists to stop **the author's own model grading its own work** (the Identity rule). Grok is neither the author's model nor a substitute for codex, so a second *independent* reviewer serves the rule's purpose rather than defeating it. **Claude subagent reviewers remain forbidden.**
   210	- Evidence it earns its keep (the first use, same day): on the Mac↔Mac instrument, grok reviewed independently, **CONFIRMED both of codex's blockers with its own measurements** (a 500 ms sleep reading as ~3 ms through the broken two-process timer; a rig-W-sized effect still reporting `VANISHES`), and found **three defects codex missed** — including a **RIG-VOID gate that fails open, which grok reproduced** (controls at ratio 1.200/bar FAIL while the session still emitted `VANISHES`). Two independent models converging on a blocker is far stronger than one; a defect only one of them finds is exactly the value of the second. Records: `.review/results/macmac-harness-r2.{gpt,grok}-verdict.md`.
   211	- When to use it: high-stakes slices — a **benchmark instrument** (this project has retracted three claims to harness bugs), a decision rule that will be applied to data, or any adjudication the owner flags. Not every slice; the cost is real (each review is minutes).
   212	- Adjudication is unchanged: **both reviewers are claim sources, not authorities.** Every finding is verified against source before it is accepted, and rejections must cite the file:line that disproves them.
   213	- Supersedes: the "Codex is the only reviewer" sentence in `docs/agent/GPT_REVIEW_LOOP.md` §Shape, which is amended in the same commit to point here.
   214
   215	## D-2026-07-14-3 — the Mac↔Mac decision rule is SIMPLIFIED: one statistic, one threshold, four cell states
   216	- Decision: the mechanized decision rule for the Mac↔Mac rig is **cut back to the smallest thing that still prevents post-hoc rationalization**. Owner, 2026-07-14, verbatim: **"simplify"**, chosen over "harden" after seven review rounds.
   217	- The problem it settles: the instrument has two halves — the **measurement** (harness: transfers, timing, rig gates) and the **decision rule** (engine: what the numbers mean). The measurement half is close to done and is verifiable by running it (`SELFTEST=1`). The decision rule had grown to ~10 outcomes, five thresholds, a certification tier and a precedence stack, and **four of the last five BLOCKERs were in the rule, not the measurement** — each a corner where the branches interacted to produce a confidently wrong verdict. The complexity was buying nothing the owner uses: he reads the table of numbers regardless.
   218	- What is KEPT (this is what pre-registration is actually for): the question, the statistic, and the thresholds are all **fixed before any data exists**, and the harness **computes the verdict** — so no one can look at the numbers and then invent a favourable reading.
   219	- What the rule now IS (rev 8): per cell, the paired ABBA differences `d_i = destinit_i − srcinit_i`, their median, and one **exact order-statistic CI** (coverage ≥95%). One threshold `T = min(10% of the srcinit median, Δ_ref = 230 ms)` — the effect must matter by the project's own invariance bar **and** be the size of the one rig W measured. Four cell states, mutually exclusive **by construction** (no labels a new case can walk past): `EFFECT` (CI_lo ≥ +T), `INVERTED` (CI_hi ≤ −T), `NONE` (CI strictly inside ±T — a genuine equivalence result), `UNCLEAR` (anything else). **Controls must all be `NONE` at the tighter `T/2`**, or no verdict about the measurand is read at all.
   220	- What was DELETED, and why it is safe: the 1.10 bar takes no part in inference (it is the acceptance criterion — computed and reported, never consulted); the sign test is reported, not decided on (at n=8 the CI already implies it); and `UNSTABLE`, `PARTIAL`, `BAR-FAIL-INCONSISTENT`, `UNDERPOWERED` and the precedence stack are gone — **a wide CI absorbs bimodality automatically and lands in `UNCLEAR`**, which is exactly what those branches were hand-coding. All eight runs of every arm are still printed, so bimodality stays visible.
   221	- Cost accepted: `UNCLEAR` and a failed control certification are now the same kind of answer — "not enough power" — and there is **NO escalation**: a noisy rig is fixed by a **quieter rig**, not more pairs. (**CORRECTED 2026-07-14**: this line originally registered `RUNS=16` as the remedy for both. The owner removed the escalation the same day — `n` is **exactly 8**, and the harness refuses any other value (`bench_otp12pf_mac.sh` preflight) because a null is judged on the **full range**, which only *widens* with `n`: more pairs can never rescue an `UNCLEAR` rig nor certify a control. The stale line was found by `drift` while fixing round 11.)
   222	- Supersedes: the rev-4/5/6/7 decision rules in `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`. **Legitimate only because NO DATA HAS EVER BEEN TAKEN** — before the first run is the only honest time to change a pre-registered rule.
   223
   224	## D-2026-07-14-4 — a rig whose permitted bias reaches HALF the effect threshold is NOT CLEAN: the session refuses to grade
   225	- Decision: on any measurand cell, if the residual arm bias the controls could not exclude (`B`) reaches **`T/2`**, the session verdict is **`CONTROLS-NOT-CLEAN`** and **no verdict is read** — not a reproduction, and not a null. Owner, 2026-07-14, choosing between "refuse to grade" and "grade it anyway, flag it", verbatim: **"Refuse to grade"**.
   226	- The defect it closes (round-11 codex, HIGH, `.review/results/macmac-harness-r11.codex-engine.md`; grok found the same dead-zone independently): `T` is **capped** at Δ_ref = 230 ms, but the bias a *clean* control is permitted to carry is a **fraction of its arm** (≤5%), and that fraction is scaled onto whatever arm it is applied to. On a slow measurand the two diverge: with clean controls at `+49 ms` on a 1000 ms arm (4.9%), a measurand at `src = 10000 ms` gets `T = 230` but `B = 490`. Then `T − B < 0`, so **a null is arithmetically impossible**, while `T + B` still licenses an `EFFECT` of which **up to 68% is permitted rig bias** — at a ratio of only 1.072, i.e. inside the project's own invariance bar. A confidently wrong reproduction, off a rig certified clean.
   227	- Why refusing beats flagging: it is the same principle the rule already applies to a dirty control — *a noisy rig is fixed by a quieter rig, not by grading it with an asterisk*. `B ≥ T/2` means the rig's own permitted noise is at least half the effect being hunted; nothing read off it can be attributed. Cost accepted: a marginal rig yields **no answer** and must be quietened and re-run.
   228	- Also fixed in the same round (mechanical, no owner choice — `B` must only ever HARDEN a verdict): (a) `B` could make the **session** verdict *easier* via `MIXED` precedence — a bigger `B` pushed one cell out of `INVERTED`, the `MIXED` branch stopped firing, and an inconclusive session reported `REPRODUCES`. The `MIXED` test is now taken on the **unhardened** cell states, so extra control noise can never turn an inconclusive session into a reproduction. (b) The **arm** medians used the low-median convention registered only for the paired differences; a bimodal arm therefore pulled `srcinit_med` *down*, shrinking `T`, making an `EFFECT` **easier**. The arms now use the conventional even-`n` median.
   229	- Supersedes: nothing. Amends the rule of D-2026-07-14-3 (rev 10 → rev 11 of the pre-registration); **legitimate only because NO DATA HAS EVER BEEN TAKEN.**
   230
   231	## D-2026-07-14-5 — the FIRST review of anything asks "is this the best way to do it", not "is this correct per the plan"
   232	- Decision: the first codex/grok round on a slice, a plan or an instrument is framed around **the end goal**, not around the agent's own plan. It states the goal in plain terms, hands over the artefacts, and asks: will this achieve it? what would you do instead, or first? what does each possible outcome buy? is the *shape* right? — and **only then** correctness. Owner, 2026-07-14, verbatim: **"less 'is this code correct per the plan' and more 'is this the best way to do it'"**, and **"You keep finding problems in the plan you wrote, so it's likely codex will have a more coherent idea."**

exec
/bin/zsh -lc "python3 -c 'import ast, pathlib; [ast.parse(pathlib.Path(p).read_text()) for p in (\"scripts/otp12pf_rigw_analyze.py\", \"scripts/otp12pf_rigw_analyze_test.py\")]; print(\"python syntax ok\")'
git diff --check 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524
git status --short" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-ynncwqJT' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-mJLf2DHe' (errno=Operation not permitted)
python syntax ok
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-aX9X2aFs' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-4ZOP4Mdo' (errno=Operation not permitted)
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
git: error: couldn't create cache file '/tmp/xcrun_db-InfU9LWT' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-WciikgPx' (errno=Operation not permitted)
?? .review/results/otp12-pf1-rigw-harness-r4.codex.md
?? .review/results/otp12-pf1-rigw-harness-r4.grok.json

exec
/bin/zsh -lc 'git diff --no-ext-diff --unified=30 4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6f517ea1bdbea2f7d83f15c086d2bf5f764cf524 -- .agents/machines.md REVIEW.md DEVLOG.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-F11EWCQI' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Ak9Q0iql' (errno=Operation not permitted)
diff --git a/.agents/machines.md b/.agents/machines.md
index 5f8f853..3188f82 100644
--- a/.agents/machines.md
+++ b/.agents/machines.md
@@ -101,60 +101,67 @@ every Linux target in play).

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
diff --git a/DEVLOG.md b/DEVLOG.md
index f23c2ae..232b31b 100644
--- a/DEVLOG.md
+++ b/DEVLOG.md
@@ -1,36 +1,37 @@
 # DEVLOG

 Entries are latest-first. Each line starts with an ISO 8601 timestamp.
 Per R5-F5 of `docs/reviews/followup_review_2026-05-02.md`: new entries
 go at the top of the file, immediately below this header, so reviewers
 scanning chronologically don't miss appended-at-the-bottom changes.
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


diff --git a/REVIEW.md b/REVIEW.md
index adc6b9c..cb1fc1f 100644
--- a/REVIEW.md
+++ b/REVIEW.md
@@ -52,60 +52,61 @@ sf-3b… count is set by sf-3a's analysis, rows added as filed).
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
+| otp12-pf1-rigw-harness | Draft reduced q↔netwatch-01 P1 diagnostic using only semantic SOURCE/DESTINATION initiator roles. Fixed OFF–ON–ON–OFF 128-arm schedule; both daemons restart per block; successful client + daemon traces correlate by run/session; destination-keyed durability, one role-invariant destination path, exact relative-path/size landed manifests, conservative paired/bimodal/observer resolution, fabric/process/PID/port gates, and no endpoint-policy mutation. Initial Codex 3/3 High fixes landed. Round-2 Codex 2/2 High fixes landed: role-independent physical paths and q-arrival settle anchors. Coder audits added a standalone live launcher smoke and a pre-daemon PID journal gate with exact recovery. Round-3 Codex PASS; additive Grok schema-valid `REOPENED` because bare Bash 3.2 path assertions let a role-in-path mutation remain green. G3 fixed with explicit parity guards; systematic audit then fixed G4's remaining material lifecycle guards and seeded the signal-marker proof. Both are mutation-proved with full gates green; fresh complete review pending; no rig data accepted. | `[~]` | review `d267335`+`8fbd486`+`6b95f03`+`53bb5e5`; fixes `661cf75`+`1617546`+`2dd977e`+`1231e42`+`6ba5408`+`18d3cde`+`454ebce`+`27c94b0`+`7e9d2d5`; r4 pending |
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

exec
/bin/zsh -lc 'rg -n "exact.*8|eight|worker|stream.*both|both.*initiator|max_streams" crates/blit-core/tests/transfer_session_roles.rs | tail -n 120' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
201:            "both ends must hold the same summary (initiator {initiator_role:?})"
264:async fn small_tree_byte_identical_under_both_initiators() {
282:async fn tiny_file_tree_tar_shard_records_under_both_initiators() {
317:        ("newer.txt", b"NEW-eight".to_vec(), 1_600_000_200),
323:        ("newer.txt", b"old-eight".to_vec(), 1_600_000_100),
402:            "both ends must hold the same summary (initiator {initiator_role:?})"
504:            "both ends must hold the same summary (initiator {initiator_role:?})"
534:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
566:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
591:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
1165:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
1230:        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
1269:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
1786:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
1787:async fn session_phase_trace_is_complete_and_inert_under_both_initiators() {
1865:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
1944:#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
1974:            max_streams: 0,
2016:        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
2057:async fn build_mismatch_refused_under_both_initiators() {
2231:async fn mirror_all_purges_extraneous_under_both_initiators() {
2265:            "both ends agree (init {initiator_role:?})"
2695:async fn source_filter_limits_manifest_under_both_initiators() {
2729:            "both ends agree (init {initiator_role:?})"

exec
/bin/zsh -lc "nl -ba crates/blit-core/tests/transfer_session_roles.rs | sed -n '1120,1285p'
nl -ba crates/blit-core/tests/transfer_session_roles.rs | sed -n '1920,2035p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1120	    std::fs::create_dir_all(&src_root).unwrap();
  1121	    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_001_200)]);
  1122
  1123	    let source_cfg = SourceSessionConfig {
  1124	        instruments: Default::default(),
  1125	        hello: HelloConfig::default(),
  1126	        endpoint: SessionEndpoint::initiator(resume_open(TransferRole::Source, RESUME_BS)),
  1127	        plan_options: PlanOptions::default(),
  1128	        data_plane_host: None,
  1129	    };
  1130	    let (source_transport, mut peer) = in_process_pair();
  1131	    let source = Arc::new(FsTransferSource::new(src_root));
  1132	    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
  1133
  1134	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
  1135	    peer.send(hello_frame()).await.unwrap();
  1136	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
  1137	    peer.send(wire(Frame::Accept(Default::default())))
  1138	        .await
  1139	        .unwrap();
  1140	    loop {
  1141	        match recv_or_panic(&mut peer).await {
  1142	            Frame::ManifestEntry(_) => continue,
  1143	            Frame::ManifestComplete(_) => break,
  1144	            other => panic!("expected manifest stream, got {other:?}"),
  1145	        }
  1146	    }
  1147	    peer.send(wire(Frame::BlockHashes(BlockHashList {
  1148	        relative_path: "real.txt".into(),
  1149	        block_size: RESUME_BS,
  1150	        hashes: Vec::new(),
  1151	    })))
  1152	    .await
  1153	    .unwrap();
  1154
  1155	    let source_err = source_task.await.unwrap().unwrap_err();
  1156	    let fault = fault_of(&source_err);
  1157	    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
  1158	    assert!(
  1159	        fault.message.contains("without a held resume need"),
  1160	        "got: {}",
  1161	        fault.message
  1162	    );
  1163	}
  1164
  1165	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1166	async fn many_tiny_files_reach_shape_target_when_source_initiates() {
  1167	    // sf-2 pin ported onto the unified session (otp-4b-2). The responder
  1168	    // grants the zero-knowledge single stream (no manifest seen at
  1169	    // SessionAccept); a 10k-tiny-file transfer over the TCP data plane
  1170	    // must re-run the shape table over the accumulated need list and grow
  1171	    // the stream count past 1 via `DataPlaneResize{ADD}`. Mirrors the old
  1172	    // push sf-2 pin (`shape_resize_e2e.rs`), now on the session: the
  1173	    // settled count is read from the destination's `data_plane_streams`.
  1174	    let tmp = tempfile::tempdir().unwrap();
  1175	    let src_root = tmp.path().join("src");
  1176	    let dst_root = tmp.path().join("dst");
  1177	    std::fs::create_dir_all(&src_root).unwrap();
  1178	    std::fs::create_dir_all(&dst_root).unwrap();
  1179	    const FILE_COUNT: usize = 10_000;
  1180	    for i in 0..FILE_COUNT {
  1181	        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
  1182	    }
  1183
  1184	    // SOURCE initiator over the TCP data plane: the control lane rides the
  1185	    // in-process transport; the data-plane sockets ride loopback TCP (the
  1186	    // responder binds 0.0.0.0:0 and the source dials 127.0.0.1).
  1187	    let open = SessionOpen {
  1188	        initiator_role: TransferRole::Source as i32,
  1189	        compare_mode: ComparisonMode::SizeMtime as i32,
  1190	        in_stream_bytes: false,
  1191	        ..Default::default()
  1192	    };
  1193	    let source_cfg = SourceSessionConfig {
  1194	        instruments: Default::default(),
  1195	        hello: HelloConfig::default(),
  1196	        endpoint: SessionEndpoint::initiator(open),
  1197	        plan_options: PlanOptions::default(),
  1198	        data_plane_host: Some("127.0.0.1".into()),
  1199	    };
  1200	    let dest_cfg = DestinationSessionConfig {
  1201	        hello: HelloConfig::default(),
  1202	        endpoint: SessionEndpoint::Responder,
  1203	        data_plane_host: None,
  1204	        instruments: Default::default(),
  1205	        local_apply: None,
  1206	    };
  1207	    let (a, b) = in_process_pair();
  1208	    let source = Arc::new(FsTransferSource::new(src_root.clone()));
  1209	    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
  1210	        tokio::join!(
  1211	            run_source(source_cfg, a, source),
  1212	            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
  1213	        )
  1214	    })
  1215	    .await
  1216	    .expect("session run timed out");
  1217
  1218	    let summary = source_result.expect("source succeeds");
  1219	    let outcome = dest_result.expect("destination succeeds");
  1220	    assert!(
  1221	        !summary.in_stream_carrier_used,
  1222	        "the sf-2 pin must ride the TCP data plane"
  1223	    );
  1224	    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
  1225	    let streams = outcome
  1226	        .data_plane_streams
  1227	        .expect("data plane ran, stream count recorded");
  1228	    assert_eq!(
  1229	        streams, 8,
  1230	        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
  1231	         target regardless of which endpoint initiated the session"
  1232	    );
  1233	    assert_trees_identical(&src_root, &dst_root);
  1234	}
  1235
  1236	/// Hold one resize ACK in the SOURCE receive half. The data-plane payload
  1237	/// lane is independent, so a correct nonblocking ramp can keep moving work
  1238	/// while this control frame waits; a pre-dispatch settle cannot.
  1239	struct GateNthResizeAckRx {
  1240	    inner: Box<dyn FrameRx>,
  1241	    gate_at: usize,
  1242	    seen: Arc<AtomicUsize>,
  1243	    delivered: Arc<AtomicUsize>,
  1244	    waiting: Arc<AtomicBool>,
  1245	    reached: Arc<tokio::sync::Notify>,
  1246	    release: Arc<tokio::sync::Notify>,
  1247	}
  1248
  1249	#[async_trait::async_trait]
  1250	impl FrameRx for GateNthResizeAckRx {
  1251	    async fn recv(&mut self) -> eyre::Result<Option<TransferFrame>> {
  1252	        let frame = self.inner.recv().await?;
  1253	        let is_resize_ack = frame
  1254	            .as_ref()
  1255	            .is_some_and(|frame| matches!(frame.frame.as_ref(), Some(Frame::ResizeAck(_))));
  1256	        if is_resize_ack {
  1257	            let ordinal = self.seen.fetch_add(1, Ordering::SeqCst) + 1;
  1258	            if ordinal == self.gate_at {
  1259	                self.waiting.store(true, Ordering::SeqCst);
  1260	                self.reached.notify_one();
  1261	                self.release.notified().await;
  1262	            }
  1263	            self.delivered.fetch_add(1, Ordering::SeqCst);
  1264	        }
  1265	        Ok(frame)
  1266	    }
  1267	}
  1268
  1269	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1270	async fn payload_does_not_wait_for_shape_ramp_under_either_initiator() {
  1271	    // 2,000 one-byte files produce shape target 4. Gate ACK #2 (the 2→3
  1272	    // epoch), require ALL payloads to finish sending while it remains held,
  1273	    // then release it and require the same exact final target under both
  1274	    // connection-role layouts. This deterministically guards first-byte /
  1275	    // useful-work progress without relying on wall-clock RTT assumptions.
  1276	    const FILE_COUNT: usize = 2_000;
  1277	    const TARGET_STREAMS: usize = 4;
  1278
  1279	    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
  1280	        let tmp = tempfile::tempdir().unwrap();
  1281	        let src_root = tmp.path().join("src");
  1282	        let dst_root = tmp.path().join("dst");
  1283	        std::fs::create_dir_all(&src_root).unwrap();
  1284	        std::fs::create_dir_all(&dst_root).unwrap();
  1285	        for i in 0..FILE_COUNT {
  1920	        )
  1921	    })
  1922	    .await
  1923	    .expect("session run timed out");
  1924
  1925	    let summary = source_result.expect("source responder succeeds");
  1926	    let outcome = dest_result.expect("destination initiator succeeds");
  1927	    assert!(
  1928	        !summary.in_stream_carrier_used,
  1929	        "the pull data plane must ride TCP, not the in-stream carrier"
  1930	    );
  1931	    assert_eq!(
  1932	        summary, outcome.summary,
  1933	        "both ends must hold the same summary"
  1934	    );
  1935	    assert_eq!(outcome.summary.files_transferred, 4);
  1936	    assert_eq!(
  1937	        outcome.data_plane_streams,
  1938	        Some(1),
  1939	        "a 4-file need list stays single-stream (below the shape threshold)"
  1940	    );
  1941	    assert_trees_identical(&src_root, &dst_root);
  1942	}
  1943
  1944	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1945	async fn many_tiny_files_reach_shape_target_when_destination_initiates() {
  1946	    // otp-5b-2: the sf-2 shape correction in the PULL direction — the
  1947	    // mirror of `many_tiny_files_reach_shape_target_when_source_initiates`
  1948	    // (push). Here the DESTINATION is the *initiator* (dials the epoch-N
  1949	    // sockets it grows to) and the SOURCE is the *responder* (accepts them
  1950	    // off its bound listener). The control-lane `DataPlaneResize{ADD}` /
  1951	    // `DataPlaneResizeAck` frames are identical to push; only the transport
  1952	    // action flips. A 10k-tiny-file transfer must re-run the shape table
  1953	    // over the accumulated need list and grow the stream count past 1.
  1954	    let tmp = tempfile::tempdir().unwrap();
  1955	    let src_root = tmp.path().join("src");
  1956	    let dst_root = tmp.path().join("dst");
  1957	    std::fs::create_dir_all(&src_root).unwrap();
  1958	    std::fs::create_dir_all(&dst_root).unwrap();
  1959	    const FILE_COUNT: usize = 10_000;
  1960	    for i in 0..FILE_COUNT {
  1961	        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
  1962	    }
  1963
  1964	    // DESTINATION initiator; SOURCE responder — roles flipped from the push
  1965	    // shape test, the data plane following connection role.
  1966	    let open = SessionOpen {
  1967	        initiator_role: TransferRole::Destination as i32,
  1968	        compare_mode: ComparisonMode::SizeMtime as i32,
  1969	        in_stream_bytes: false,
  1970	        // Wire contract: zero means unknown, not a one-stream cap. Pin it
  1971	        // on the destination-initiator orientation, where this end both
  1972	        // advertises and enforces the receiver ceiling.
  1973	        receiver_capacity: Some(CapacityProfile {
  1974	            max_streams: 0,
  1975	            ..Default::default()
  1976	        }),
  1977	        ..Default::default()
  1978	    };
  1979	    let source_cfg = SourceSessionConfig {
  1980	        instruments: Default::default(),
  1981	        hello: HelloConfig::default(),
  1982	        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
  1983	        plan_options: PlanOptions::default(),
  1984	        data_plane_host: None, // a responder never dials
  1985	    };
  1986	    let dest_cfg = DestinationSessionConfig {
  1987	        hello: HelloConfig::default(),
  1988	        endpoint: SessionEndpoint::initiator(open), // dials + receives
  1989	        data_plane_host: Some("127.0.0.1".into()),
  1990	        instruments: Default::default(),
  1991	        local_apply: None,
  1992	    };
  1993	    let (a, b) = in_process_pair();
  1994	    let source = Arc::new(FsTransferSource::new(src_root.clone()));
  1995	    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
  1996	        tokio::join!(
  1997	            run_source(source_cfg, a, source),
  1998	            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
  1999	        )
  2000	    })
  2001	    .await
  2002	    .expect("session run timed out");
  2003
  2004	    let summary = source_result.expect("source responder succeeds");
  2005	    let outcome = dest_result.expect("destination initiator succeeds");
  2006	    assert!(
  2007	        !summary.in_stream_carrier_used,
  2008	        "the pull sf-2 pin must ride the TCP data plane"
  2009	    );
  2010	    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
  2011	    let streams = outcome
  2012	        .data_plane_streams
  2013	        .expect("data plane ran, stream count recorded");
  2014	    assert_eq!(
  2015	        streams, 8,
  2016	        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
  2017	         target regardless of which endpoint initiated the session"
  2018	    );
  2019	    assert_trees_identical(&src_root, &dst_root);
  2020	}
  2021
  2022	#[tokio::test]
  2023	async fn preserves_mtime_on_streamed_files() {
  2024	    // Not part of the role matrix — pins that the file-record write
  2025	    // path applies the manifest mtime (parity with today's receive
  2026	    // paths, which the byte-identical asserts alone wouldn't catch).
  2027	    let tmp = tempfile::tempdir().unwrap();
  2028	    let src_root = tmp.path().join("src");
  2029	    let dst_root = tmp.path().join("dst");
  2030	    std::fs::create_dir_all(&src_root).unwrap();
  2031	    std::fs::create_dir_all(&dst_root).unwrap();
  2032	    write_tree(
  2033	        &src_root,
  2034	        &[("stamped.txt", b"stamp me".to_vec(), 1_555_555_555)],
  2035	    );

context compacted
codex
- No material observable defects found.

VERDICT: PASS
tokens used
283,824
- No material observable defects found.

VERDICT: PASS
