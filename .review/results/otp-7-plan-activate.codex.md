Reading additional input from stdin...
OpenAI Codex v0.144.0
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f49ff-54ee-7813-857f-5a4de242d10f
--------
user
Review the diff of commit 70c9688 (run: git show 70c9688). It is a docs/plan-only change: flipping docs/plan/OTP7_RESUME.md from Draft to Active after the owner answered its Q1-Q3 (recorded as D-2026-07-09-1 in docs/DECISIONS.md), plus propagation to docs/STATE.md and a TODO.md row update. Check: internal coherence, no contradiction with docs/DECISIONS.md or docs/plan/ONE_TRANSFER_PATH.md, the owner answers are recorded faithfully and unambiguously, no stale cross-references left (e.g. remaining 'Draft' or 'awaiting owner' mentions of OTP7_RESUME anywhere in docs/), and scope statements are consistent (end-of-op fault summary IN otp-7; progress-display TUI redesign NOT in otp-7). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the target commit and the governing decision/plan documents, then cross-check all `docs/` references for stale OTP7 status or scope wording. No files will be changed.
exec
/bin/zsh -lc "sed -n '1,240p' /Users/michael/.codex/RTK.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# RTK - Rust Token Killer (Codex CLI)

**Usage**: Token-optimized CLI proxy for shell commands.

## Rule

Always prefix shell commands with `rtk`.

Examples:

```bash
rtk git status
rtk cargo test
rtk npm run build
rtk pytest -q
```

## Meta Commands

```bash
rtk gain            # Token savings analytics
rtk gain --history  # Recent command savings history
rtk proxy <cmd>     # Run raw command without filtering
```

## Verification

```bash
rtk --version
rtk gain
which rtk
```

exec
/bin/zsh -lc "sed -n '1,240p' AGENTS.md && sed -n '1,260p' .agents/repo-guidance.md && sed -n '1,260p' .agents/state.md" in /Users/michael/Dev/blit_v2
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
- `handoff`: update `.agents/state.md` so the next session can resume without chat context. Prune as you write: rotate landed or superseded entries verbatim to `docs/history/state-archive.md` (create on first use) so `## Now` holds only live items; volatile facts (push status, CI state, counts) carry `as of <commit>` and are re-verified or dropped, never left to rot; a count or enumeration another file owns is pointed to, never copied; machine-local facts are labeled `machine-local (<host>)` or omitted; re-verify the recorded basis of every parked or blocked item and move anything falsified into `## Blockers` with the new evidence.
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves — `AGENTS.md` and `.agents/*` — are in scope as drift targets, not just sources of truth.
- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
- `plan`: draft or update a durable plan before broad implementation work.
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

- `origin` — `https://github.com/roethlar/Blit.git` (GitHub, canonical).
- `gitea` — `http://q:3000/michael/blit_v2.git` (LAN gitea mirror; pushed
  manually alongside or after `origin`, not auto-synced by any hook or CI
  job — it can lag GitHub by a commit or more at any given time).
- (Names verified against `git remote -v` 2026-07-04; an earlier revision
  of this doc called GitHub `github` and the mirror `origin` — that never
  matched the actual config and misread `origin/master` references.)
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
  PascalCase, constants SHOUT_CASE; match existing names (`transfer_engine`,
  `TransferOrchestrator`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, transfer engine,
  orchestrator); most logic and unit tests live here. New modules get
  re-exported in `crates/blit-core/src/lib.rs`.
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

exec
/bin/zsh -lc 'git show --stat --oneline --decorate --no-renames 70c9688 && git show --format=fuller --no-ext-diff --no-renames 70c9688' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-bFcPefhR' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-6WypdN6I' (errno=Operation not permitted)
70c9688 (HEAD -> master) otp-7: flip OTP7_RESUME Active (D-2026-07-09-1, owner Q1-Q3)
 TODO.md                  | 17 ++++++++++++-----
 docs/DECISIONS.md        |  5 +++++
 docs/STATE.md            | 24 +++++++++++-------------
 docs/plan/OTP7_RESUME.md | 37 +++++++++++++++++++++++--------------
 4 files changed, 51 insertions(+), 32 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-pGvptn9k' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2p9pNeVQ' (errno=Operation not permitted)
commit 70c96880fa031bdb0318c0338cc7c359d2213ca4
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Thu Jul 9 23:06:00 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Thu Jul 9 23:06:00 2026 -0400

    otp-7: flip OTP7_RESUME Active (D-2026-07-09-1, owner Q1-Q3)
    
    Q1 contract-wins stale fallback; Q2 in-place patch + end-of-op fault
    summary rider; Q3 7a-then-7b. TODO CLI-output-redesign row re-confirmed
    with owner framing; STATE Now/Queue/Open-questions propagated.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
    Claude-Session: https://claude.ai/code/session_01NhUphCcW8hbiXEZBYbbJcv

diff --git a/TODO.md b/TODO.md
index b933835..a4dc893 100644
--- a/TODO.md
+++ b/TODO.md
@@ -269,11 +269,18 @@ explicitly-deferred logging epic (F15).
       same change (docs-after-behavior rule). Distinct from `audit-17`
       (destination-fs charset rejection) — that crash is only a
       *symptom* here; the exclude no-op is the reported bug.
-- [ ] **CLI transfer output redesign** (owner, 2026-07-06): current
-      `blit copy`/`mirror` output "doesn't convey any useful information
-      at all" — owner wants something closer to `rclone`/`cargo`: a
-      persistent stat block at a static screen location, plus a scrolling
-      list of in-flight/recent filenames, instead of what exists today.
+- [ ] **CLI transfer output redesign** (owner, 2026-07-06; re-confirmed
+      2026-07-09): current `blit copy`/`mirror` output "doesn't convey any
+      useful information at all" — owner wants something closer to
+      `rclone`/`cargo`: "a coherent info block with stats and a scrolling
+      list of files in a frame below, so probably a TUI?" (owner wording,
+      2026-07-09) — i.e. a persistent stat block at a static screen
+      location, plus a scrolling list of in-flight/recent filenames,
+      instead of what exists today. 2026-07-09 context: the owner hit this
+      while settling otp-7's error-surfacing question — "the current
+      progress display is absolutely useless for this". The narrow
+      end-of-operation fault summary (name failed files, suggest re-run)
+      ships with otp-7 (D-2026-07-09-1) and is NOT gated on this redesign.
       Confirmed by reading the actual code — there is no persistent/redraw
       rendering anywhere in the transfer output path, only plain
       scrolling `println!`/`eprintln!` lines: (1) the local/streaming-manifest
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index c86a7ce..dd2a783 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -145,3 +145,8 @@ Format:
 - Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
 - Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
 - Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).
+
+## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
+- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
+- Why: owner answered Q1–Q3 in session 2026-07-09; the flip is the approval the plan procedure requires. In the same exchange the owner re-confirmed the broader progress-display redesign (persistent stats block + scrolling file frame, "probably a TUI") — that stays a queued TODO.md item ("CLI transfer output redesign"), NOT otp-7 scope, and needs its own plan.
+- Supersedes: nothing (the plan doc's Open-questions section is rewritten as resolved in the same commit).
diff --git a/docs/STATE.md b/docs/STATE.md
index c828a43..5d3cb43 100644
--- a/docs/STATE.md
+++ b/docs/STATE.md
@@ -1,10 +1,10 @@
 # STATE — single entry point for "what is true right now"
 
-Last updated: 2026-07-06
+Last updated: 2026-07-09
 
 - 2026-07-04: Owner-approved dual push reached 3d8326b (origin: 10d89e0..3d8326b; gitea mirror: 2a77b9f..3d8326b). That push corrected a prior remote-name confusion; windows-latest CI on that push is the "meaningfully green" check referenced in prior notes.
 
-- Current session (2026-07-06): otp-6 CLOSED; otp-7 in DESIGN — slice design drafted at docs/plan/OTP7_RESUME.md (Draft). NO CODE until the owner answers Q1–Q3 and flips the plan Active. otp-6 (a/b) mirror + filters landed and graded. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
+- Current session (2026-07-09): owner answered otp-7's Q1–Q3 (D-2026-07-09-1) — docs/plan/OTP7_RESUME.md is **Active**; otp-7a (resume over the in-stream carrier) is the current slice, through the codex loop. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
 
 - Session work: filed audit-17 and audit-18; noted a CLI-output-redesign item in TODO.md; drafted+reviewed docs/plan/LOCAL_ERROR_TELEMETRY.md (Draft). A session-wide codex pass fixed 5 cross-doc staleness bugs.
 
@@ -49,9 +49,11 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
     rule: DESTINATION diffs the complete source manifest at SourceDone,
     scan-complete-guarded + filter-scoped. Codex High: keep-set now folds
     case on macOS too (case-insensitive-FS data-loss). Suite → **1529**.
-  - Current: **otp-7 IN DESIGN** — Draft `docs/plan/OTP7_RESUME.md`
-    (`9fb5e4a`) awaiting owner review (see Open questions); no code until
-    Active. otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
+  - Current: **otp-7 ACTIVE (D-2026-07-09-1)** — `docs/plan/OTP7_RESUME.md`
+    flipped Active 2026-07-09 (Q1 contract-wins fallback; Q2 in-place patch
+    + end-of-op fault summary rider; Q3 7a-then-7b). Implementing **otp-7a**
+    (resume over the in-stream carrier) through the codex loop.
+    otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
 - **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
   `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
   until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
@@ -69,8 +71,8 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
    the only work item until it ships**: slices otp-1..13 through the
    codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
    otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b) `[x]`. Current:
-   **otp-7 IN DESIGN** (`docs/plan/OTP7_RESUME.md` Draft, owner review;
-   no code until Active). otp-2 (symmetric baseline) is RIG-GATED —
+   **otp-7 ACTIVE** (`docs/plan/OTP7_RESUME.md`, D-2026-07-09-1) —
+   implementing otp-7a. otp-2 (symmetric baseline) is RIG-GATED —
    before otp-10 cutover.
 2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
    Shipped (zero-copy resolved — D-2026-07-05-3). Optional owner-gated
@@ -103,8 +105,8 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 ## Authoritative docs right now
 
 - **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
-  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Draft** — otp-7 slice
-  design, awaiting owner review before any code).
+  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
+  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
 - Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
   sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
   complete; measurement gates remain). REV4 superseded v1/REV2/REV3
@@ -136,10 +138,6 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 
 ## Open questions
 
-- **(OPEN — owner review, 2026-07-07, otp-7)** `docs/plan/OTP7_RESUME.md`
-  (Draft) awaits the owner's Q1–Q3 (graceful stale fallback; in-place-patch
-  mid-failure model; 7a-then-7b staging — all agent-rec yes) and the flip to
-  Active. That flip unblocks otp-7 implementation.
 - **(OPEN — owner ack, 2026-07-05, otp-4a)** Unified SizeMtime semantic:
   same-size + dest-NEWER — old push clobbers, session adopts **data-safe
   SKIP** (converge-up; `--force` still overwrites; pinned by
diff --git a/docs/plan/OTP7_RESUME.md b/docs/plan/OTP7_RESUME.md
index c794426..3634177 100644
--- a/docs/plan/OTP7_RESUME.md
+++ b/docs/plan/OTP7_RESUME.md
@@ -1,12 +1,13 @@
 # otp-7 — resume block phase (design)
 
-**Status**: Draft
+**Status**: Active (owner Q1–Q3 answered + "confirmed", 2026-07-09; D-2026-07-09-1)
 **Created**: 2026-07-07
 **Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-7.
 **Contract**: `docs/TRANSFER_SESSION.md` (resume exception + frame table, pinned otp-1).
-**Governs**: no code until the owner flips this to `**Status**: Active`
-(AGENTS.md; `.agents/repo-guidance.md` plan operator). Per D-2026-07-04-1 this
-plan change also goes through the codex loop.
+**Governs**: implementation proceeds 7a → 7b, one slice per codex loop pass
+(D-2026-07-04-1). Owner's deciding principle, quoted: "FAST, SIMPLE, RELIABLE
+file transfer. if we abort the whole thing when we could have fixed or
+surfaced a single error, we are violating all of those."
 
 ## Why this doc
 
@@ -119,6 +120,13 @@ path's `BlockHashList` (fail-fast if a block phase would start without one).
   hashes reflect whatever landed). The pin asserts the fault surfaces cleanly and no
   file is falsely counted `files_resumed`. (No stronger atomicity than the code we
   are replacing — called out as a Known gap, not a regression.)
+  **Owner rider (2026-07-09, Q2)**: the fault must also appear in the CLI's
+  **end-of-operation summary** — naming the affected file(s) and suggesting a
+  re-run to converge — not only as a mid-stream line that scrolls away. Small
+  CLI-layer deliverable, lands within otp-7 (the session already collects the
+  per-file fault; this is about where it is reported). The full progress-display
+  redesign it brushes against is a separate queued item (TODO.md "CLI transfer
+  output redesign") and is NOT in otp-7 scope.
 - **D5 — block size**: `ResumeSettings.block_size` clamped to `MAX_BLOCK_SIZE`, `0` ⇒
   `DEFAULT_BLOCK_SIZE`. The DEST chooses (it hashes first); the SOURCE reads the size
   from the `BlockHashList`, so the two never disagree.
@@ -154,16 +162,17 @@ mid-resume-failure cases")
    `SessionFault` surfaces to both ends, `files_resumed` not incremented for the
    aborted file, no deadlock.
 
-## Open questions for the owner
-
-- **Q1**: D1 (graceful stale fallback) reconciles the old data-plane hard-error
-  against the contract. Confirm the contract wins (agent rec: yes — it is the pinned
-  wire behavior and the safer one).
-- **Q2**: D4 keeps the old in-place-patch failure model (no temp+rename atomicity).
-  Acceptable as-is for otp-7, or do you want atomic partial-file handling as a
-  follow-up item? (agent rec: keep parity now, file a follow-up if wanted.)
-- **Q3**: Staging — 7a (in-stream) then 7b (data-plane), per the AskUserQuestion
-  answer's default. Confirm, or collapse into one.
+## Open questions — RESOLVED (owner, 2026-07-09; D-2026-07-09-1)
+
+- **Q1 — contract wins.** Stale/mismatched partial degrades gracefully to a
+  full-file transfer, never an abort. Owner's principle (quoted in the header)
+  is the rationale; D1 stands as written.
+- **Q2 — keep in-place patch, surface at end of op.** No temp+rename atomicity
+  for otp-7 (parity with the code being replaced). The owner's rider: the fault
+  is surfaced in the end-of-operation summary with a re-run suggestion — see D4.
+  No atomicity follow-up filed; convergence-on-retry is the reliability model.
+- **Q3 — 7a then 7b, no collapse.** Owner: "confirmed. no collapse. keep the
+  reviewloop codex playbook going slice by slice."
 
 ## Verification (when Active)
 

exec
/bin/zsh -lc 'git status --short && git remote -v && git rev-parse --show-toplevel && git rev-parse 70c9688''^{commit}' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-pXjn3v7f' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-F0QRnlAO' (errno=Operation not permitted)
?? .review/results/otp-7-plan-activate.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ATmiiDCc' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-upOrjHA9' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ePQ7jdSj' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-3SAzv9hB' (errno=Operation not permitted)
/Users/michael/Dev/blit_v2
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-hbxUEh6P' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-OAXJ825Q' (errno=Operation not permitted)
70c96880fa031bdb0318c0338cc7c359d2213ca4

exec
/bin/zsh -lc 'rtk git show 70c9688:docs/STATE.md | nl -ba' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'rtk git show 70c9688:docs/plan/OTP7_RESUME.md | nl -ba' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "rg -n -i --glob 'docs/**' 'OTP7_RESUME|otp-7.{0,80}(draft|awaiting|awaits|owner review|in design|open question)|(?:draft|awaiting|awaits|owner review|in design|open question).{0,80}otp-7|end-of-operation summary|progress-display|progress display|probably a TUI|CLI transfer output redesign'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/DECISIONS.md:149:## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
docs/DECISIONS.md:150:- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
docs/DECISIONS.md:151:- Why: owner answered Q1–Q3 in session 2026-07-09; the flip is the approval the plan procedure requires. In the same exchange the owner re-confirmed the broader progress-display redesign (persistent stats block + scrolling file frame, "probably a TUI") — that stays a queued TODO.md item ("CLI transfer output redesign"), NOT otp-7 scope, and needs its own plan.
docs/STATE.md:7:- Current session (2026-07-09): owner answered otp-7's Q1–Q3 (D-2026-07-09-1) — docs/plan/OTP7_RESUME.md is **Active**; otp-7a (resume over the in-stream carrier) is the current slice, through the codex loop. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
docs/STATE.md:52:  - Current: **otp-7 ACTIVE (D-2026-07-09-1)** — `docs/plan/OTP7_RESUME.md`
docs/STATE.md:74:   **otp-7 ACTIVE** (`docs/plan/OTP7_RESUME.md`, D-2026-07-09-1) —
docs/STATE.md:108:  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
docs/STATE.md:185:  session**: otp-7 — owner's Q1–Q3 on `docs/plan/OTP7_RESUME.md`, flip
docs/plan/OTP7_RESUME.md:124:  **end-of-operation summary** — naming the affected file(s) and suggesting a
docs/plan/OTP7_RESUME.md:127:  per-file fault; this is about where it is reported). The full progress-display
docs/plan/OTP7_RESUME.md:172:  is surfaced in the end-of-operation summary with a re-run suggestion — see D4.
docs/plan/ONE_TRANSFER_PATH.md:276:   the Design's RELIABLE exception). Slice design: `docs/plan/OTP7_RESUME.md`

 succeeded in 0ms:
     1	# STATE — single entry point for "what is true right now"
     2	
     3	Last updated: 2026-07-09
     4	
     5	- 2026-07-04: Owner-approved dual push reached 3d8326b (origin: 10d89e0..3d8326b; gitea mirror: 2a77b9f..3d8326b). That push corrected a prior remote-name confusion; windows-latest CI on that push is the "meaningfully green" check referenced in prior notes.
     6	
     7	- Current session (2026-07-09): owner answered otp-7's Q1–Q3 (D-2026-07-09-1) — docs/plan/OTP7_RESUME.md is **Active**; otp-7a (resume over the in-stream carrier) is the current slice, through the codex loop. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
     8	
     9	- Session work: filed audit-17 and audit-18; noted a CLI-output-redesign item in TODO.md; drafted+reviewed docs/plan/LOCAL_ERROR_TELEMETRY.md (Draft). A session-wide codex pass fixed 5 cross-doc staleness bugs.
    10	
    11	- Notes on push state: owner previously pushed master → GitHub at 10d89e0; local commits f6e592e..HEAD remain unpushed and windows-latest CI will ride the next push.
    12	
    13	Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
    14	≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
    15	procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
    16	
    17	## Now (active work)
    18	
    19	- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
    20	  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
    21	  invariant (plan doc, verbatim): ONE block of transfer code;
    22	  direction/initiator/verb can NEVER affect wall time by blit's doing
    23	  — impossible by construction because the per-direction drivers and
    24	  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
    25	  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
    26	  cells. **D-2026-07-05-2: same-build peers only, refusal at session
    27	  open.** Progress (each through the codex loop; closed-slice detail in
    28	  DEVLOG + `.review/` + REVIEW.md):
    29	  - **otp-1 / otp-3 / otp-4a `[x]`** — wire+session contract
    30	    (`docs/TRANSFER_SESSION.md`); role-parameterized drivers over the
    31	    in-process transport (invariance property in the role suite); daemon
    32	    serves `Transfer` as Responder, client push over gRPC; A/B
    33	    byte-identical vs old push; SizeMtime = data-safe skip (open Q below).
    34	  - **otp-4b (1/2/3) `[x]` — push data plane fully on the session, closed**:
    35	    single-stream TCP data plane, mid-transfer resize/multi-stream + sf-2
    36	    shape correction, deterministic mid-transfer cancel. Detail: DEVLOG.
    37	  - **otp-5a `[x]`** (`84be1cc`, codex PASS) — the one served `Transfer`
    38	    RPC serves BOTH roles via `run_responder` (SOURCE-init→daemon
    39	    DESTINATION = push; DEST-init→daemon SOURCE = pull, in-stream).
    40	  - **otp-5b (1/2) `[x]`** — the SOURCE-responder data plane, closed:
    41	    5b-1 (`e6a0b3b`+`13485ee`) decoupled connection role (RESPONDER
    42	    binds+accepts, INITIATOR dials) from byte role; 5b-2 (`d579365`+
    43	    `773a877`) lifted the single-stream cap — the pull data plane resizes
    44	    via sf-2 (same resize frames as push). Defaults to TCP; A/B
    45	    byte-identical vs old `pull_sync`. Suite → **1522**.
    46	  - **otp-6 (a/b) `[x]`** — mirror + filters on the session, closed.
    47	    6a (`c026692`+`0bb27f5`) honors `SessionOpen.filter` via the universal
    48	    `FilteredSource` chokepoint. 6b (`01d9c41`+`3c99557`) is the one delete
    49	    rule: DESTINATION diffs the complete source manifest at SourceDone,
    50	    scan-complete-guarded + filter-scoped. Codex High: keep-set now folds
    51	    case on macOS too (case-insensitive-FS data-loss). Suite → **1529**.
    52	  - Current: **otp-7 ACTIVE (D-2026-07-09-1)** — `docs/plan/OTP7_RESUME.md`
    53	    flipped Active 2026-07-09 (Q1 contract-wins fallback; Q2 in-place patch
    54	    + end-of-op fault summary rider; Q3 7a-then-7b). Implementing **otp-7a**
    55	    (resume over the in-stream carrier) through the codex loop.
    56	    otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
    57	- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
    58	  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
    59	  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
    60	  baseline. Principle stands: ceiling-driven, never competitor-relative
    61	  (D-2026-07-04-4; a ≥25% margin answer was retracted — do not
    62	  re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
    63	- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
    64	  measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s; owner
    65	  declarations pending in Blocked); 10 GbE session done; w9-3 + review rows
    66	  landed. Codex loop governs all changes (D-2026-07-04-1; DEVLOG 07-04/05).
    67	
    68	## Queue (ordered)
    69	
    70	1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
    71	   the only work item until it ships**: slices otp-1..13 through the
    72	   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
    73	   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b) `[x]`. Current:
    74	   **otp-7 ACTIVE** (`docs/plan/OTP7_RESUME.md`, D-2026-07-09-1) —
    75	   implementing otp-7a. otp-2 (symmetric baseline) is RIG-GATED —
    76	   before otp-10 cutover.
    77	2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
    78	   Shipped (zero-copy resolved — D-2026-07-05-3). Optional owner-gated
    79	   measurement follow-ups (Win 11 bare-metal; disk-path variants;
    80	   >ARC-size push) — disk-path items largely absorbed by otp-2/otp-12's
    81	   symmetric-rig matrices. Env: bench binaries at
    82	   `skippy:/mnt/generic-pool/video/blit-bin/` (/tmp, /home noexec there).
    83	3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
    84	   resumes/re-derives after ONE_TRANSFER_PATH ships.
    85	4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
    86	   open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
    87	   (mirror-executor consolidation) likely lands for free inside
    88	   otp-6's one-delete-rule slice; re-check before picking it up.
    89	5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
    90	   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
    91	   cutover as a runtime-selected write strategy in the unified receive
    92	   sink (design: eval doc §If-FAST-evidence; dead module deletes in
    93	   w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
    94	   2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
    95	   rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
    96	   test data, everything; nothing written outside it, ever. Zero-copy
    97	   is pre-authorized to be tested there when the post-cutover slice set
    98	   reaches it; no daemon runs on zoey before then without a fresh go.
    99	6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   100	   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   101	   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   102	   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   103	   tuning residue (w3-1 scoped it out).
   104	
   105	## Authoritative docs right now
   106	
   107	- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
   108	  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
   109	  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
   110	- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
   111	  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
   112	  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
   113	  (history only).
   114	- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
   115	  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
   116	  is retired as the grading mechanism (its `findings/`/`results/`
   117	  records and the REVIEW.md index remain live).
   118	- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
   119	  rows) + `.review/findings/` + `.review/results/`.
   120	- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
   121	  D-2026-06-12-1, executes w8-1; **capability unparked
   122	  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
   123	  (gated on Round 1),
   124	  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).
   125	
   126	## Blocked / waiting (all owner declarations; checkpoints are owner-only)
   127	
   128	- **Three 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
   129	  holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE),
   130	  REV4 → Shipped. (The zero-copy revisit verdict and the a/b/c
   131	  question are RESOLVED — D-2026-07-05-3, unparked; measured skippy
   132	  data 1.43 cores daemon-receive / 0.45 client at 9.5 Gbit/s stays
   133	  recorded in DEVLOG + DIAGNOSIS.md.)
   134	- **Push go**: local commits `f6e592e`..HEAD await the ref-listing +
   135	  approval flow; windows-latest CI on the w9-3 harness fix rides it.
   136	- `Cargo.lock`: fresh transitive drift (crossbeam-*, cc, etc.), same class
   137	  as `04c9c6d` — not this session's; owner's call to commit or revert.
   138	
   139	## Open questions
   140	
   141	- **(OPEN — owner ack, 2026-07-05, otp-4a)** Unified SizeMtime semantic:
   142	  same-size + dest-NEWER — old push clobbers, session adopts **data-safe
   143	  SKIP** (converge-up; `--force` still overwrites; pinned by
   144	  `same_size_newer_destination_is_skipped_not_clobbered`). Owner: confirm
   145	  or ask for old-push clobber. Reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
   146	- **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
   147	- **(OPEN, 2026-07-04)** `725aa07` tracked a 236-file stale worktree snapshot
   148	  (`.claude/worktrees/vigilant-mayer/`). Agent rec: `git rm -r`; awaits go.
   149	- **(OPEN, 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still describe
   150	  the deleted `determine_remote_tuning`/`TuningParams` — fold into
   151	  w10-docs-batch (agent rec) or rewrite sooner?
   152	- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: the 10 GbE
   153	  session delivered the measurement evidence; flip awaits the three
   154	  declarations in Blocked (was four — zero-copy resolved,
   155	  D-2026-07-05-3).
   156	- **(OPEN, new 2026-07-05)** CLI foot-gun found during the session:
   157	  `blit copy src_large dst` with an existing local dir, no `./`,
   158	  parses the bare name as an mDNS discovery endpoint and errors
   159	  "remote source must include a module or root"
   160	  (blit-app endpoints.rs). Should local-path existence win over the
   161	  discovery interpretation, or at least improve the error? Candidate
   162	  review-queue row; owner to slot.
   163	- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
   164	  locally; daemon-spawn e2e flakiness root-caused + fixed on Linux (w9-3:
   165	  port-TOCTOU race + cargo-lock contention). Remaining: windows-latest CI
   166	  on the next push (10d89e0 predates the w9-3 fix).
   167	
   168	## Handoff log (newest first, keep ≤ 3)
   169	
   170	- **2026-07-06 (39th)** @ `598f102` — **Session-wide codex review (5
   171	  findings, all fixed); one new backlog item filed; otp-7 still
   172	  untouched.** `/playbook reviewloop` named a generic template this repo's
   173	  own guidance says isn't the operative loop here (branch-per-finding
   174	  conflicts with no-agent-branches) — ran `GPT_REVIEW_LOOP.md`'s mechanism
   175	  instead over the whole session diff (`9876687..44de868`). 3 Medium + 2
   176	  Low, all cross-doc staleness/contradictions from mid-session edits not
   177	  propagating everywhere (stale Q5 header, a STATE/plan-doc contradiction,
   178	  a stale handoff entry, a date-drift note, an overstated claim) — fixed
   179	  `419f5d1`, verdict `.review/results/session-2026-07-06.gpt-verdict.md`.
   180	  Also: owner asked for a CLI transfer-output redesign (rclone/cargo-style
   181	  static stat block + file list; current output is bare scrolling
   182	  `println!`s, `helpers.rs:176`/`transfers/remote.rs:33-140`) — filed to
   183	  `TODO.md` only (`598f102`), needs its own `plan` when picked up.
   184	  **Exact first action next
   185	  session**: otp-7 — owner's Q1–Q3 on `docs/plan/OTP7_RESUME.md`, flip
   186	  Active, codex-review, implement otp-7a. In-flight: none. Done since
   187	  38th: the session-wide review pass; the CLI-output-redesign TODO item.
   188	- **2026-07-06 (38th)** @ `44de868` — **`LOCAL_ERROR_TELEMETRY.md` drafted
   189	  + reviewed twice (3+3 findings fixed), Q1-Q5 resolved; still Draft, no
   190	  code.** Full detail: DEVLOG 20:15Z/21:00Z entries and the plan doc's own
   191	  Q5 section (pickup timing); the 39th entry above covers the staleness
   192	  bugs this left behind, since fixed. Done since 37th: audit-17/18 filed
   193	  (`5628c03`, `deb3800`); the telemetry plan end-to-end.

 succeeded in 0ms:
     1	# otp-7 — resume block phase (design)
     2	
     3	**Status**: Active (owner Q1–Q3 answered + "confirmed", 2026-07-09; D-2026-07-09-1)
     4	**Created**: 2026-07-07
     5	**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-7.
     6	**Contract**: `docs/TRANSFER_SESSION.md` (resume exception + frame table, pinned otp-1).
     7	**Governs**: implementation proceeds 7a → 7b, one slice per codex loop pass
     8	(D-2026-07-04-1). Owner's deciding principle, quoted: "FAST, SIMPLE, RELIABLE
     9	file transfer. if we abort the whole thing when we could have fixed or
    10	surfaced a single error, we are violating all of those."
    11	
    12	## Why this doc
    13	
    14	otp-7 is the plan's **explicit RELIABLE exception**: resumed files use a
    15	strictly-ordered block-hash exchange, and the choreography is novel (unlike the
    16	mechanical carrier splits of otp-4b/5b/6). The owner asked for the design on paper
    17	before the intricate code. This doc records the choreography, the reuse map, the
    18	design decisions (most already settled by the contract), the staging, and the
    19	guard-proof targets — so implementation is a transcription, not a discovery.
    20	
    21	## What resume is (contract, already pinned in otp-1)
    22	
    23	A `NeedEntry` may be flagged `resume=true`. For such a file the DESTINATION sends
    24	its `BlockHashList` (Blake3 per block of the existing partial) and the SOURCE
    25	**must not send any byte of that file until it has received that list**. The SOURCE
    26	then transfers only the blocks whose hashes differ (or that the dest lacks), as
    27	`BlockTransfer` records, ending with `BlockTransferComplete{total_bytes}`. Stale or
    28	mismatched partials fall back to full-file transfer.
    29	
    30	Frames (field numbers frozen, `TRANSFER_SESSION.md`): `8 BlockHashList` (DEST),
    31	`14 BlockTransfer` (SOURCE), `15 BlockTransferComplete` (SOURCE). `SessionOpen.resume`
    32	carries `ResumeSettings{enabled, block_size}`. `NeedEntry.resume` is field 2.
    33	
    34	## What already exists (reused verbatim — no reinvention)
    35	
    36	- **Wire frames + payload enums**: `BlockHashList`/`BlockTransfer`/`BlockTransferComplete`
    37	  frames and `PreparedPayload::{FileBlock, FileBlockComplete}` are defined and
    38	  name-mapped in the session (`transfer_session/mod.rs:250,256,257`).
    39	- **DEST apply (reassembly)**: `FsTransferSink::write_file_block_payload`
    40	  (`sink.rs:641`, seek+write into the partial in place) and
    41	  `write_file_block_complete` (`sink.rs:687`, `set_len` + fsync + stamp mtime/perms).
    42	  In-place patch of the partial — the partial IS the destination; no temp+rename
    43	  (matches the old pull client).
    44	- **DEST block hashing**: `compute_block_hashes` (`remote/pull.rs:1139`) — streams
    45	  the partial in `block_size` chunks, `blake3::hash`, returns 32-byte digests; an
    46	  absent file returns an empty vec (the implicit full-file fallback).
    47	- **Block-diff reference**: `resume_copy_file` (`copy/file_copy/resume.rs:52`) is the
    48	  canonical block-compare (write a block iff beyond dst len, a partial tail, or
    49	  hashes differ; truncate if dst longer). The SOURCE-side diff is the same logic.
    50	- **Defaults**: `DEFAULT_BLOCK_SIZE` = 1 MiB, `MAX_BLOCK_SIZE` = 64 MiB
    51	  (`copy/file_copy/resume.rs:16,19`). `ResumeSettings.block_size == 0` ⇒ default.
    52	
    53	## What is new (the otp-7 work)
    54	
    55	1. **Un-stub the four refusal sites**: both open validators (`mod.rs:362,401`), the
    56	   source recv-half resume-need rejection (`mod.rs:799`), and the outbound-planner
    57	   FileBlock bail (`mod.rs:1446`).
    58	2. **The strict-ordering exchange choreography** in the session's source/dest halves.
    59	3. **A home for the SOURCE-side block-diff** — today hand-rolled in `pull_sync.rs`,
    60	   not on any trait (see Design decision D3).
    61	
    62	## Choreography (strict ordering)
    63	
    64	```
    65	DESTINATION (diff loop)                     SOURCE (send half)
    66	─────────────────────────                   ──────────────────
    67	for each manifest entry:
    68	  if resume-eligible (see D2):
    69	     NeedEntry{path, resume=true} ───────►  recv: ResumeNeed(header)
    70	     BlockHashList{path, bsz, hashes} ───►  recv: BlockHashes(path, hashes)
    71	                                            (send half correlates the two;
    72	                                             a resume need is HELD until its
    73	                                             BlockHashList arrives — the
    74	                                             RELIABLE ordering guarantee)
    75	  else:
    76	     NeedEntry{path, resume=false} ──────►  recv: Need(header)  (unchanged)
    77	
    78	                                            for a held resume need + its hashes:
    79	                                              read source file block-by-block,
    80	                                              blake3 each; for block i where
    81	                                              i >= hashes.len() OR hash != hashes[i]:
    82	  recv BlockTransfer{path,off,bytes} ◄────    send BlockTransfer{path, off, block}
    83	     sink.write_file_block_payload            (in-stream carrier: control-lane
    84	                                               frames; data-plane: send_block, 7b)
    85	  recv BlockTransferComplete{path,total} ◄─  send BlockTransferComplete{path,total}
    86	     sink.write_file_block_complete
    87	     files_resumed += 1
    88	```
    89	
    90	The source's per-file byte phase for a resume need is "send changed blocks then
    91	complete", replacing the whole-file record it sends for a non-resume need. Ordering
    92	is enforced on the SOURCE: it will not emit a block for a path before it holds that
    93	path's `BlockHashList` (fail-fast if a block phase would start without one).
    94	
    95	## Design decisions
    96	
    97	- **D1 — stale/mismatched partial ⇒ graceful full-file fallback**, per the contract
    98	  (`TRANSFER_SESSION.md:84`), NOT the hard `Status::internal` the old *data-plane*
    99	  path uses (`pull_sync.rs:1377`) — that is a pre-cutover quirk the gRPC path already
   100	  contradicts (`pull_sync.rs:1544`, graceful). An empty / short / all-mismatched
   101	  hash list simply means "send all blocks" = full transfer. **Reconcile in favor of
   102	  the contract.**
   103	- **D2 — resume eligibility** (which needs get `resume=true`): the file exists at the
   104	  dest as a non-empty partial AND `ResumeSettings.enabled` AND the compare says the
   105	  file must transfer (changed). A missing/empty dest file is a normal full transfer
   106	  (no resume flag, no BlockHashList). This mirrors the daemon's `effective_resume`
   107	  set (`pull_sync.rs:262`) minus the mtime-only-touch special case, which the session
   108	  already handles via SizeMtime skip.
   109	- **D3 — SOURCE block-diff home**: a free helper in the session
   110	  (`resume_block_diff(source, header, dest_hashes, block_size) -> stream of blocks`)
   111	  rather than a new `TransferSource` trait method. Rationale: it needs only
   112	  `source.open_file(header)` (already on the trait) + blake3, and keeping it out of
   113	  the trait avoids every future `TransferSource` impl re-implementing it (the same
   114	  reasoning that made `FilteredSource` the one filter chokepoint in otp-6a). Flag for
   115	  codex: confirm the helper doesn't belong on the trait.
   116	- **D4 — mid-resume-failure**: block writes patch the partial in place (no
   117	  temp+rename, matching the old client). A fault mid-block-transfer surfaces as a
   118	  `SessionFault` (peer-notified) and aborts; the partial is left partially patched,
   119	  and the NEXT resume re-syncs via a fresh block-hash exchange (the partial's new
   120	  hashes reflect whatever landed). The pin asserts the fault surfaces cleanly and no
   121	  file is falsely counted `files_resumed`. (No stronger atomicity than the code we
   122	  are replacing — called out as a Known gap, not a regression.)
   123	  **Owner rider (2026-07-09, Q2)**: the fault must also appear in the CLI's
   124	  **end-of-operation summary** — naming the affected file(s) and suggesting a
   125	  re-run to converge — not only as a mid-stream line that scrolls away. Small
   126	  CLI-layer deliverable, lands within otp-7 (the session already collects the
   127	  per-file fault; this is about where it is reported). The full progress-display
   128	  redesign it brushes against is a separate queued item (TODO.md "CLI transfer
   129	  output redesign") and is NOT in otp-7 scope.
   130	- **D5 — block size**: `ResumeSettings.block_size` clamped to `MAX_BLOCK_SIZE`, `0` ⇒
   131	  `DEFAULT_BLOCK_SIZE`. The DEST chooses (it hashes first); the SOURCE reads the size
   132	  from the `BlockHashList`, so the two never disagree.
   133	- **D6 — invariance**: resume runs identically whichever end initiated (the flag is
   134	  in the open; the DEST computes hashes and applies; the SOURCE diffs and sends). The
   135	  role suite runs both initiator assignments, as for every prior slice.
   136	
   137	## Staging
   138	
   139	- **otp-7a — resume over the in-stream carrier.** Fully exercisable in
   140	  `transfer_session_roles.rs` (both initiator roles, in-stream). Un-stub the
   141	  refusals; implement the choreography + block-diff helper + DEST hash-send + apply
   142	  wiring over the control-lane `BlockTransfer`/`Complete` frames; `files_resumed`.
   143	  Pins: happy-path partial, identical-file (zero blocks), stale-partial fallback,
   144	  mid-resume-failure.
   145	- **otp-7b — resume over the TCP data plane.** Port the block records onto the data
   146	  plane (`data_plane.rs::send_block`/`send_block_complete` already exist) with the
   147	  same choreography; e2e in the daemon harness. Follows 7a exactly as otp-4b-1→4b-2
   148	  and otp-5b-1→5b-2 did.
   149	
   150	## Guard-proof targets (the plan's mandate: "pins the stale-partial and
   151	mid-resume-failure cases")
   152	
   153	1. **Partial resume** — a multi-block file with some blocks already correct at the
   154	   dest: only the changed blocks move (assert BlockTransfer count / bytes), final
   155	   bytes identical, `files_resumed == 1`. Guard: neuter the block-diff so it sends
   156	   all blocks ⇒ the "only changed blocks moved" assertion FAILS.
   157	2. **Identical file** — zero blocks transferred, file untouched, still counted done.
   158	3. **Stale-partial fallback** — a dest partial that shares no blocks with the source
   159	   ⇒ full content lands, bytes identical, no hang/fault. Guard: force the source to
   160	   trust the stale hashes ⇒ corrupt output.
   161	4. **Mid-resume-failure** — inject a source fault mid-block-phase ⇒ a clean
   162	   `SessionFault` surfaces to both ends, `files_resumed` not incremented for the
   163	   aborted file, no deadlock.
   164	
   165	## Open questions — RESOLVED (owner, 2026-07-09; D-2026-07-09-1)
   166	
   167	- **Q1 — contract wins.** Stale/mismatched partial degrades gracefully to a
   168	  full-file transfer, never an abort. Owner's principle (quoted in the header)
   169	  is the rationale; D1 stands as written.
   170	- **Q2 — keep in-place patch, surface at end of op.** No temp+rename atomicity
   171	  for otp-7 (parity with the code being replaced). The owner's rider: the fault
   172	  is surfaced in the end-of-operation summary with a re-run suggestion — see D4.
   173	  No atomicity follow-up filed; convergence-on-retry is the reliability model.
   174	- **Q3 — 7a then 7b, no collapse.** Owner: "confirmed. no collapse. keep the
   175	  reviewloop codex playbook going slice by slice."
   176	
   177	## Verification (when Active)
   178	
   179	- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
   180	  `cargo test --workspace` (count must not drop).
   181	- Windows parity after touching `win_fs`/sink paths: `scripts/windows/run-blit-tests.ps1`.
   182	- Each sub-slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`), guard proof
   183	  per pin above.

exec
/bin/zsh -lc "rtk git show 70c9688:docs/DECISIONS.md | nl -ba | sed -n '130,175p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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

exec
/bin/zsh -lc "rtk git show 70c9688:docs/plan/ONE_TRANSFER_PATH.md | nl -ba | rg -n -i -C 8 'otp-7|resume|end-of-operation|progress|summary|tui|atomic|fault'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
58-    58	  compatibility.)
59-    59	- Making different hardware perform identically. If src and dst sit
60-    60	  on different disks, the two *data directions* still differ by
61-    61	  physics; the invariant is that the same data direction between the
62-    62	  same endpoints is identical regardless of who initiates and which
63-    63	  verb is used.
64-    64	- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
65-    65	- New features. This is a consolidation; capability parity with
66:    66	  today (mirror, filters, resume, fallback, delegation, progress,
67-    67	  jobs, cancellation) is the bar. Zero-copy receive is **unparked**
68-    68	  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
69-    69	  after cutover, not one of this plan's slices — see the Design note
70-    70	  on the write-strategy seam.
71-    71	
72-    72	## Constraints
73-    73	
74-    74	- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
--
117-   117	      cell must be ≤ the better of that cell's two old directions
118-   118	      + run noise (±10%). A symmetric-but-slower result fails.
119-   119	- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
120-   120	      (driver), daemon `push/control.rs` choreography, daemon
121-   121	      `pull_sync.rs` choreography, the delegated-pull driver, the
122-   122	      separate local orchestration path, and the `Push`/`PullSync`
123-   123	      RPCs no longer exist in the tree; one `TransferSession` and one
124-   124	      `Transfer` RPC remain. The `DelegatedPull` RPC may survive only
125:   125	      as trigger + progress relay — the proof must show it carries no
126-   126	      payload bytes (codex F3). Recorded file-by-file in the final
127-   127	      slice's finding doc.
128-   128	- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
129:   129	      guard), filters, block-resume, gRPC fallback carrier, delegated
130:   130	      transfer, progress events, jobs/cancel, read-only enforcement —
131-   131	      each demonstrated by ported tests on the session.
132-   132	- [ ] Suite green throughout; final test count ≥ pre-plan baseline
133-   133	      (1483); all REV4 invariant pins and the sf-2 pin pass
134-   134	      role-parameterized.
135-   135	- [ ] Benchmark methodology corrected and recorded: symmetric-fs
136-   136	      cells are the verdict cells; tmpfs cells remain only as
137-   137	      explicitly-labeled wire-reference rows (never compared across
138-   138	      directions with asymmetric endpoints).
139-   139	- [ ] Windows: full suite green (owner machine) + windows-latest CI.
140-   140	
141-   141	## Design
142-   142	
143-   143	**What already is one code** (kept, becomes the session's engine):
144-   144	`remote/transfer/` — pipeline, sink/source abstractions, data plane,
145:   145	diff planner, tar-shard, stall guard, progress, `operation_spec` (the
146-   146	REV4 unified contract), and the engine dial (stream policy incl. sf-2
147-   147	shape correction). The defect layer is above it: four driver loops
148-   148	choreograph these pieces differently per direction.
149-   149	
150-   150	**The one choreography** (roles, not directions):
151-   151	
152-   152	1. Initiator opens the single bidi `Transfer` RPC and sends the
153-   153	   operation spec: which end is SOURCE, which is DESTINATION, path/
154:   154	   module, filters, mirror/resume flags, capabilities.
155-   155	2. SOURCE enumerates and **streams** its manifest immediately (no
156-   156	   buffered-enumeration phase — this generalizes push's fast start;
157-   157	   pull's full-enumeration-then-negotiate slow start is deleted, which
158-   158	   absorbs the "pull 1s-start" residue item).
159-   159	3. DESTINATION diffs incrementally against its own filesystem and
160-   160	   returns need-list batches (one diff owner, always the end that
161-   161	   owns the target fs — push's proven model; pull_sync's
162-   162	   source-side diff is deleted).
163-   163	4. The data plane opens at the dial floor immediately; stream count
164-   164	   shape-corrects as the need list accumulates (sf-2 mechanism, now
165-   165	   the only policy, both roles).
166:   166	5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
167-   167	   the one pipeline into the data plane; DESTINATION writes through
168-   168	   the one receive path. The receive sink is built with a
169-   169	   **runtime-selected write-strategy seam**: buffered relay is the
170-   170	   universal strategy; capability-gated alternatives slot in behind
171-   171	   it without new paths — the first is zero-copy/splice
172-   172	   (D-2026-07-05-3, unparked for CPU-bound receivers like the
173-   173	   owner's UNAS 8 Pro; design input:
174-   174	   `ZERO_COPY_RECEIVE_EVAL.md` §If-FAST-evidence), landing as a
175-   175	   follow-on slice set after cutover. Strategy selection reads
176-   176	   capability and payload type, never role or initiator.
177-   177	6. Mirror: DESTINATION computes deletions from the completed source
178-   178	   manifest it received (filter-scoped, scan-complete-guarded) and
179-   179	   executes them locally. One rule, no per-direction delete
180-   180	   choreography.
181:   181	7. Resume: optional block-hash phase inside the same session, same
182-   182	   messages regardless of roles.
183:   183	8. Summary/byte-accounting: one record shape.
184-   184	
185-   185	**Transport facts vs choreography**: the connection-initiating end
186-   186	dials TCP data-plane sockets (NAT reality) — byte direction within a
187-   187	socket is set by role, not by who dialed. The gRPC-fallback lane
188-   188	becomes a *byte-carrier option* inside the same session (control-
189-   189	stream frames instead of TCP sockets), selected at negotiation — not
190-   190	a separate transfer path. Resize keeps its controller-at-sender rule.
191-   191	
192-   192	**Delegated transfer**: a daemon receiving a delegated request simply
193-   193	becomes an initiator of the same session against the other daemon
194-   194	(destination role on its module fs). The bespoke delegated-pull
195-   195	driver is deleted; the delegation *gate* (authorization) stays. The
196:   196	`DelegatedPull` RPC itself is client↔daemon trigger + progress relay
197:   197	(`DelegatedPullProgress` stream) — it never carries payload bytes;
198-   198	its handler shrinks to "authorize, spawn the session, relay the
199:   199	session's progress events." It stays wire-compatible or is folded at
200-   200	cutover — either way the deletion proof asserts no bytes flow
201-   201	through it (codex F3).
202-   202	
203:   203	**Resume ordering (RELIABLE exception, codex F5)**: resumed files use
204-   204	a strictly-ordered block-hash exchange — the DESTINATION's block map
205-   205	for a file must complete before the SOURCE sends any block of that
206-   206	file, and stale/mismatched partials fall back to full-file transfer.
207-   207	This is an explicit exception to the immediate-start rule, exactly as
208:   208	today's resume path is an explicit single-stream RELIABLE exception
209-   209	(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
210:   210	contract; otp-7 pins the stale-partial and mid-resume-failure cases
211-   211	in tests.
212-   212	
213-   213	**Local transfers**: the same session driver over an in-process
214-   214	transport (both roles in one process, no wire). The engine underneath
215-   215	is already shared; the separate local orchestration path is deleted
216-   216	in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
217-   217	guard the migration.
218-   218	
219-   219	**Affected crates**: `blit-core` (new `transfer_session` module;
220-   220	`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
221-   221	`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
222-   222	delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
223-   223	`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
224:   224	`blit-tui` (progress/jobs consume the same events).
225-   225	
226-   226	**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
227-   227	mitigated by strangler slices with the tree green throughout and a
228-   228	non-optional deletion slice. Per-cell regression risk on today's
229-   229	faster direction — mitigated by the converge-up constraint and
230-   230	baseline parity pins per slice. Wire break — lockstep upgrade,
231-   231	owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
232:   232	Progress/jobs/TUI integration churn — the session emits the existing
233-   233	event contract (w6-1) at the same boundaries.
234-   234	
235-   235	## Slices
236-   236	
237-   237	One coherent, testable change per slice — sized for the `.review/`
238-   238	loop. Tree green after every slice; old paths keep working until
239-   239	otp-9 deletes them.
240-   240	
241-   241	1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
242-   242	   `Transfer` RPC and message set — roles, phases, field numbers,
243-   243	   the **strict same-build handshake** (exact protocol/build identity
244-   244	   exchanged at session open; any mismatch is refused with a clear
245-   245	   error — D-2026-07-05-2; pinned by test when the session lands),
246-   246	   the receiver capacity profile + bounded-unilateral dial contract
247-   247	   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
248:   248	   that exists), transport selection, resume phase ordering (the
249-   249	   RELIABLE exception above), mirror phase, error/cancel semantics.
250-   250	   No feature-capability bits: same build implies same features.
251-   251	   The new proto text must carry NO version-tolerance semantics; the
252-   252	   capacity profile's absent/0 fields mean "unknown hardware value"
253-   253	   only, never "old peer" (today's proto comments frame some of that
254-   254	   contract as old-peer fallback — those comment blocks describe live
255-   255	   pre-cutover code and die with their messages at otp-10, per the
256-   256	   D-2026-07-05-2 review adjudication). Codex-reviewed before any
--
262-   262	   rig. This is the converge-up reference the acceptance criteria
263-   263	   compare against (codex F4).
264-   264	3. **otp-3 TransferSession core (blit-core)**: role-parameterized
265-   265	   state machine over the existing engine with an in-process
266-   266	   transport; unit/e2e tests run BOTH role assignments over the same
267-   267	   fixtures — the invariance property enters the test suite here.
268-   268	4. **otp-4 daemon serves `Transfer`, client initiates as SOURCE**
269-   269	   (remote push-equivalent rides the session); A/B parity pins vs
270:   270	   old push (byte-identical trees, summary parity, sf-2 pin ported).
271-   271	5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-
272-   272	   equivalent) — the same code with roles flipped; the parity suite
273-   273	   reruns with no per-direction test code.
274-   274	6. **otp-6 mirror + filters** on the session (one delete rule).
275:   275	7. **otp-7 resume** block phase (ordering + stale-partial pins per
276:   276	   the Design's RELIABLE exception). Slice design: `docs/plan/OTP7_RESUME.md`
277-   277	   (staged 7a in-stream / 7b data-plane).
278-   278	8. **otp-8 fallback byte-carrier** (control-stream frames) as the
279-   279	   session's alternate transport.
280-   280	9. **otp-9 delegated transfer** = daemon-initiated session; bespoke
281-   281	   delegated-pull driver retired behind the existing gate;
282:   282	   `DelegatedPull` RPC reduced to trigger + progress relay.
283:   283	10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
284-   284	    operation through the session; `Push`/`PullSync` and all four
285-   285	    drivers deleted from the tree and the proto, no bridge
286-   286	    (D-2026-07-05-2); ported-test accounting proves count never
287-   287	    dropped. Deletion proof recorded, incl. the DelegatedPull
288-   288	    no-payload-bytes assertion.
289-   289	11. **otp-11 local transfers** ride the in-process transport; the
290-   290	    separate local orchestration is deleted; local perf pins hold.
291-   291	12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
292-   292	    on the unified path — initiator/verb invariance A/B within noise
293-   293	    AND every cell ≤ the better old direction + noise; committed as
294-   294	    this plan's acceptance evidence.
295-   295	13. **otp-13 verdict**: acceptance checklist walked with the owner;
296:   296	    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
297-   297	    against the unified baseline — owner call at that point.
298-   298	
299-   299	## Open questions
300-   300	
301-   301	- None requiring owner input now — scope, wire, and process were
302-   302	  delegated (Directive section). Slice-level unknowns (exact proto
303:   303	  shapes, resume edge semantics, TUI event wiring) are settled inside
304-   304	  their slices through the codex loop. — owner

exec
/bin/zsh -lc "rtk git show 70c9688:TODO.md | nl -ba | sed -n '245,320p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   245	      `FileFilter::exclude_dirs` / `dir_globs` / `should_include_dir`
   246	      at `fs_enum.rs:274-295` is never assigned anywhere in `crates/`),
   247	      so the only incantation that works today is
   248	      `--exclude '.java/**'` (and likely `--exclude .java` too, for the
   249	      dir entry itself). Nothing warns that a pattern matched zero of
   250	      the configured globs — a silent no-op, same foot-gun class as the
   251	      endpoint-parse open question in `docs/STATE.md`. Docs gap:
   252	      `--help` says only "Exclude files matching this glob pattern"
   253	      (`crates/blit-cli/src/cli.rs:292`) with no hint that matching is
   254	      source-relative (not absolute) or that a directory needs `/**`;
   255	      rsync users reasonably expect leading-`/`-anchors-to-transfer-root
   256	      and trailing-`/`-matches-dir semantics, none of which blit
   257	      implements. Confirmed by reading the matcher end-to-end, not run.
   258	      Fix needs a design call (owner input required, `plan` this before
   259	      coding): options span (a) accept absolute patterns under the
   260	      source root by stripping the source prefix before matching;
   261	      (b) give directory patterns rsync-like subtree semantics and/or
   262	      add a real `--exclude-dir`; (c) at minimum, warn when a pattern is
   263	      structurally unmatchable (absolute but not under the source, or
   264	      literal with no possible relative/filename match) instead of
   265	      silently transferring everything. Whatever is chosen must apply
   266	      uniformly across local-mirror, push, pull, and remote-remote (all
   267	      route through the one `FileFilter`/`FilterInputs` chokepoint,
   268	      `cli.rs:288-291`) and ship `--help`/manpage/README updates in the
   269	      same change (docs-after-behavior rule). Distinct from `audit-17`
   270	      (destination-fs charset rejection) — that crash is only a
   271	      *symptom* here; the exclude no-op is the reported bug.
   272	- [ ] **CLI transfer output redesign** (owner, 2026-07-06; re-confirmed
   273	      2026-07-09): current `blit copy`/`mirror` output "doesn't convey any
   274	      useful information at all" — owner wants something closer to
   275	      `rclone`/`cargo`: "a coherent info block with stats and a scrolling
   276	      list of files in a frame below, so probably a TUI?" (owner wording,
   277	      2026-07-09) — i.e. a persistent stat block at a static screen
   278	      location, plus a scrolling list of in-flight/recent filenames,
   279	      instead of what exists today. 2026-07-09 context: the owner hit this
   280	      while settling otp-7's error-surfacing question — "the current
   281	      progress display is absolutely useless for this". The narrow
   282	      end-of-operation fault summary (name failed files, suggest re-run)
   283	      ships with otp-7 (D-2026-07-09-1) and is NOT gated on this redesign.
   284	      Confirmed by reading the actual code — there is no persistent/redraw
   285	      rendering anywhere in the transfer output path, only plain
   286	      scrolling `println!`/`eprintln!` lines: (1) the local/streaming-manifest
   287	      path's spinner + `"Enumerated N entries… (streaming manifest)"`
   288	      heartbeat (`crates/blit-core/src/remote/push/client/helpers.rs:176`,
   289	      the same call site audit-16 already flagged for its own separate
   290	      `--verbose`-gating bug); (2) the remote-transfer progress path's
   291	      once-a-second `"[progress] N/M files • X MiB copied • Y MiB/s avg •
   292	      Z MiB/s current"` line (`crates/blit-cli/src/transfers/remote.rs:33-140`,
   293	      `spawn_progress_monitor_with_options`), which just reprints a new
   294	      line every tick rather than redrawing in place. Neither path shows
   295	      a file list, a static stat panel, or does any cursor
   296	      repositioning — every line is transient and scrolls off, which
   297	      matches the owner's complaint exactly. This is a real UX/design
   298	      project, not a bug fix: likely needs a terminal-rendering approach
   299	      (raw ANSI cursor save/restore, or a crate like `indicatif`), has to
   300	      cover both the local and remote transfer paths above, has to decide
   301	      a fallback for non-TTY/`--json`/piped output (today's plain-line
   302	      output is presumably what scripts already parse — a redesign must
   303	      not break `--json` consumers), and touches `blit-cli`+`blit-app`.
   304	      Not designed here — needs its own `plan` before any code, per this
   305	      repo's governance (code changes require an approved plan) and the
   306	      Review policy (D-2026-07-04-1, all code through the codex loop).
   307	      Distinct from `docs/plan/TUI_REWORK.md` (Active), which is about
   308	      the separate interactive `blit-tui` navigation app, not this
   309	      inline CLI progress output during a transfer.
   310	
   311	### Deferred design calls
   312	
   313	These are intentionally not next-actionable. Don't pick them up
   314	without the listed prerequisite — they're tracked here so they
   315	don't get lost, not so the next agent reimplements them on a hunch.
   316	
   317	- [x] **Remote→remote re-evaluation** — resolved by
   318	  `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Phase 1 (`15991ed`)
   319	  added the `DelegatedPull` wire/gate/daemon path; Phase 2 makes
   320	  remote→remote CLI transfers direct by default, keeps

exec
/bin/zsh -lc 'rtk git ls-remote github HEAD' in /Users/michael/Dev/blit_v2
 succeeded in 340ms:
e5a78d90b88cdf12773b8e49a0750714968bc8bd	HEAD

codex
The first stale-reference pass found one likely miss in `docs/STATE.md`; I’m checking whether it is isolated and whether the parent plan’s “no new features” non-goal needs an explicit narrow exception for the newly mandated fault summary.
exec
/bin/zsh -lc "sed -n '1,260p' scripts/agent/check-docs.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
#!/usr/bin/env bash
# Doc-consistency lint. Run locally before pushing; CI runs it in docs-gate.yml.
set -u
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)" || exit 1
fail=0

req() {
  if [ ! -f "$1" ]; then
    echo "MISSING: $1"
    fail=1
  fi
}
req docs/STATE.md
req docs/DECISIONS.md
req docs/agent/PROTOCOL.md
req AGENTS.md
req CLAUDE.md

if [ -f docs/STATE.md ]; then
  grep -q '^Last updated:' docs/STATE.md || {
    echo "docs/STATE.md: missing 'Last updated:' line"
    fail=1
  }
  lines=$(wc -l < docs/STATE.md | tr -d ' ')
  if [ "$lines" -gt 200 ]; then
    echo "docs/STATE.md is $lines lines (cap 200) — prune handoffs into DEVLOG.md"
    fail=1
  fi
fi

for f in docs/plan/*.md; do
  [ -e "$f" ] || continue
  case "$f" in */README.md) continue ;; esac
  first=$(grep -m1 -E '^\*\*Status\*\*:' "$f" || true)
  if [ -z "$first" ]; then
    echo "$f: missing '**Status**:' header (Draft|Active|Shipped|Superseded|Historical)"
    fail=1
  elif ! printf '%s\n' "$first" | grep -qE '^\*\*Status\*\*:[[:space:]]*(Draft|Active|Shipped|Superseded|Historical)([[:space:]]|$)'; then
    echo "$f: first '**Status**:' line is off-vocabulary ($first) — expected Draft|Active|Shipped|Superseded|Historical"
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "check-docs: OK"
fi
exit "$fail"

exec
/bin/zsh -lc "rtk git show 70c9688:docs/STATE.md | nl -ba | sed -n '165,200p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   165	  port-TOCTOU race + cargo-lock contention). Remaining: windows-latest CI
   166	  on the next push (10d89e0 predates the w9-3 fix).
   167	
   168	## Handoff log (newest first, keep ≤ 3)
   169	
   170	- **2026-07-06 (39th)** @ `598f102` — **Session-wide codex review (5
   171	  findings, all fixed); one new backlog item filed; otp-7 still
   172	  untouched.** `/playbook reviewloop` named a generic template this repo's
   173	  own guidance says isn't the operative loop here (branch-per-finding
   174	  conflicts with no-agent-branches) — ran `GPT_REVIEW_LOOP.md`'s mechanism
   175	  instead over the whole session diff (`9876687..44de868`). 3 Medium + 2
   176	  Low, all cross-doc staleness/contradictions from mid-session edits not
   177	  propagating everywhere (stale Q5 header, a STATE/plan-doc contradiction,
   178	  a stale handoff entry, a date-drift note, an overstated claim) — fixed
   179	  `419f5d1`, verdict `.review/results/session-2026-07-06.gpt-verdict.md`.
   180	  Also: owner asked for a CLI transfer-output redesign (rclone/cargo-style
   181	  static stat block + file list; current output is bare scrolling
   182	  `println!`s, `helpers.rs:176`/`transfers/remote.rs:33-140`) — filed to
   183	  `TODO.md` only (`598f102`), needs its own `plan` when picked up.
   184	  **Exact first action next
   185	  session**: otp-7 — owner's Q1–Q3 on `docs/plan/OTP7_RESUME.md`, flip
   186	  Active, codex-review, implement otp-7a. In-flight: none. Done since
   187	  38th: the session-wide review pass; the CLI-output-redesign TODO item.
   188	- **2026-07-06 (38th)** @ `44de868` — **`LOCAL_ERROR_TELEMETRY.md` drafted
   189	  + reviewed twice (3+3 findings fixed), Q1-Q5 resolved; still Draft, no
   190	  code.** Full detail: DEVLOG 20:15Z/21:00Z entries and the plan doc's own
   191	  Q5 section (pickup timing); the 39th entry above covers the staleness
   192	  bugs this left behind, since fixed. Done since 37th: audit-17/18 filed
   193	  (`5628c03`, `deb3800`); the telemetry plan end-to-end.

exec
/bin/zsh -lc "rtk git grep -n -i -E 'SessionFault|end.of.operation|fault summary|failed files|re-run|rerun|mid.resume|abort' 70c9688 -- docs/TRANSFER_SESSION.md docs/plan/ONE_TRANSFER_PATH.md docs/plan/OTP7_RESUME.md docs/DECISIONS.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XPimYMNZ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-zNXkuPcg' (errno=Operation not permitted)
70c9688:docs/DECISIONS.md:150:- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
70c9688:docs/TRANSFER_SESSION.md:217:  or aborts says why before closing; operators never diagnose from a
70c9688:docs/plan/ONE_TRANSFER_PATH.md:210:contract; otp-7 pins the stale-partial and mid-resume-failure cases
70c9688:docs/plan/ONE_TRANSFER_PATH.md:273:   reruns with no per-direction test code.
70c9688:docs/plan/ONE_TRANSFER_PATH.md:291:12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
70c9688:docs/plan/OTP7_RESUME.md:9:file transfer. if we abort the whole thing when we could have fixed or
70c9688:docs/plan/OTP7_RESUME.md:116:- **D4 — mid-resume-failure**: block writes patch the partial in place (no
70c9688:docs/plan/OTP7_RESUME.md:118:  `SessionFault` (peer-notified) and aborts; the partial is left partially patched,
70c9688:docs/plan/OTP7_RESUME.md:124:  **end-of-operation summary** — naming the affected file(s) and suggesting a
70c9688:docs/plan/OTP7_RESUME.md:125:  re-run to converge — not only as a mid-stream line that scrolls away. Small
70c9688:docs/plan/OTP7_RESUME.md:144:  mid-resume-failure.
70c9688:docs/plan/OTP7_RESUME.md:151:mid-resume-failure cases")
70c9688:docs/plan/OTP7_RESUME.md:161:4. **Mid-resume-failure** — inject a source fault mid-block-phase ⇒ a clean
70c9688:docs/plan/OTP7_RESUME.md:162:   `SessionFault` surfaces to both ends, `files_resumed` not incremented for the
70c9688:docs/plan/OTP7_RESUME.md:163:   aborted file, no deadlock.
70c9688:docs/plan/OTP7_RESUME.md:168:  full-file transfer, never an abort. Owner's principle (quoted in the header)
70c9688:docs/plan/OTP7_RESUME.md:172:  is surfaced in the end-of-operation summary with a re-run suggestion — see D4.

exec
/bin/zsh -lc "rtk git grep -n -i -E 'otp.?7|OTP7_RESUME' 70c9688 -- docs ':"'!docs/history/**'"'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-cpgAt8hX' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-UciWH03B' (errno=Operation not permitted)
70c9688:docs/DECISIONS.md:149:## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
70c9688:docs/DECISIONS.md:150:- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
70c9688:docs/DECISIONS.md:151:- Why: owner answered Q1–Q3 in session 2026-07-09; the flip is the approval the plan procedure requires. In the same exchange the owner re-confirmed the broader progress-display redesign (persistent stats block + scrolling file frame, "probably a TUI") — that stays a queued TODO.md item ("CLI transfer output redesign"), NOT otp-7 scope, and needs its own plan.
70c9688:docs/STATE.md:7:- Current session (2026-07-09): owner answered otp-7's Q1–Q3 (D-2026-07-09-1) — docs/plan/OTP7_RESUME.md is **Active**; otp-7a (resume over the in-stream carrier) is the current slice, through the codex loop. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
70c9688:docs/STATE.md:52:  - Current: **otp-7 ACTIVE (D-2026-07-09-1)** — `docs/plan/OTP7_RESUME.md`
70c9688:docs/STATE.md:54:    + end-of-op fault summary rider; Q3 7a-then-7b). Implementing **otp-7a**
70c9688:docs/STATE.md:74:   **otp-7 ACTIVE** (`docs/plan/OTP7_RESUME.md`, D-2026-07-09-1) —
70c9688:docs/STATE.md:75:   implementing otp-7a. otp-2 (symmetric baseline) is RIG-GATED —
70c9688:docs/STATE.md:108:  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
70c9688:docs/STATE.md:109:  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
70c9688:docs/STATE.md:171:  findings, all fixed); one new backlog item filed; otp-7 still
70c9688:docs/STATE.md:185:  session**: otp-7 — owner's Q1–Q3 on `docs/plan/OTP7_RESUME.md`, flip
70c9688:docs/STATE.md:186:  Active, codex-review, implement otp-7a. In-flight: none. Done since
70c9688:docs/plan/LOCAL_ERROR_TELEMETRY.md:17:does not enter the Queue and no code lands until the current otp-7 slice
70c9688:docs/plan/LOCAL_ERROR_TELEMETRY.md:303:  begun in parallel with otp-7; or
70c9688:docs/plan/LOCAL_ERROR_TELEMETRY.md:305:  pick this up the moment the current otp-7 slice reaches a stopping point.
70c9688:docs/plan/ONE_TRANSFER_PATH.md:210:contract; otp-7 pins the stale-partial and mid-resume-failure cases
70c9688:docs/plan/ONE_TRANSFER_PATH.md:275:7. **otp-7 resume** block phase (ordering + stale-partial pins per
70c9688:docs/plan/ONE_TRANSFER_PATH.md:276:   the Design's RELIABLE exception). Slice design: `docs/plan/OTP7_RESUME.md`
70c9688:docs/plan/OTP7_RESUME.md:1:# otp-7 — resume block phase (design)
70c9688:docs/plan/OTP7_RESUME.md:5:**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-7.
70c9688:docs/plan/OTP7_RESUME.md:14:otp-7 is the plan's **explicit RELIABLE exception**: resumed files use a
70c9688:docs/plan/OTP7_RESUME.md:53:## What is new (the otp-7 work)
70c9688:docs/plan/OTP7_RESUME.md:126:  CLI-layer deliverable, lands within otp-7 (the session already collects the
70c9688:docs/plan/OTP7_RESUME.md:129:  output redesign") and is NOT in otp-7 scope.
70c9688:docs/plan/OTP7_RESUME.md:139:- **otp-7a — resume over the in-stream carrier.** Fully exercisable in
70c9688:docs/plan/OTP7_RESUME.md:145:- **otp-7b — resume over the TCP data plane.** Port the block records onto the data
70c9688:docs/plan/OTP7_RESUME.md:171:  for otp-7 (parity with the code being replaced). The owner's rider: the fault

exec
/bin/zsh -lc "rtk git show 70c9688:docs/plan/ONE_TRANSFER_PATH.md | nl -ba | sed -n '1,90p;195,220p;265,285p;295,310p'" in /Users/michael/Dev/blit_v2
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
    70	  on the write-strategy seam.
    71	
    72	## Constraints
    73	
    74	- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
    75	  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
    76	  violated at the choreography layer.
    77	- **Converge up, not down**: per benchmark cell, the unified session
    78	  must match the better of today's two directions (within ±10% run
    79	  noise), not their average. Unification that slows the fast
    80	  direction fails review.
    81	- REV4 invariants carry: byte-identical results, StallGuard,
    82	  cancellation, byte-accounting. Existing pins are ported (not
    83	  dropped) as tests become role-parameterized; test count never
    84	  drops.
    85	- The sf-2 shape-correction behavior (stream count corrects as the
    86	  need list accumulates) becomes the one and only stream policy —
    87	  both directions inherit it by construction; its pins carry over.
    88	- **The bounded-unilateral dial contract carries unchanged**
    89	  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
    90	  dial, bounded by the byte RECEIVER's advertised capacity profile
   195	driver is deleted; the delegation *gate* (authorization) stays. The
   196	`DelegatedPull` RPC itself is client↔daemon trigger + progress relay
   197	(`DelegatedPullProgress` stream) — it never carries payload bytes;
   198	its handler shrinks to "authorize, spawn the session, relay the
   199	session's progress events." It stays wire-compatible or is folded at
   200	cutover — either way the deletion proof asserts no bytes flow
   201	through it (codex F3).
   202	
   203	**Resume ordering (RELIABLE exception, codex F5)**: resumed files use
   204	a strictly-ordered block-hash exchange — the DESTINATION's block map
   205	for a file must complete before the SOURCE sends any block of that
   206	file, and stale/mismatched partials fall back to full-file transfer.
   207	This is an explicit exception to the immediate-start rule, exactly as
   208	today's resume path is an explicit single-stream RELIABLE exception
   209	(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
   210	contract; otp-7 pins the stale-partial and mid-resume-failure cases
   211	in tests.
   212	
   213	**Local transfers**: the same session driver over an in-process
   214	transport (both roles in one process, no wire). The engine underneath
   215	is already shared; the separate local orchestration path is deleted
   216	in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
   217	guard the migration.
   218	
   219	**Affected crates**: `blit-core` (new `transfer_session` module;
   220	`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
   265	   state machine over the existing engine with an in-process
   266	   transport; unit/e2e tests run BOTH role assignments over the same
   267	   fixtures — the invariance property enters the test suite here.
   268	4. **otp-4 daemon serves `Transfer`, client initiates as SOURCE**
   269	   (remote push-equivalent rides the session); A/B parity pins vs
   270	   old push (byte-identical trees, summary parity, sf-2 pin ported).
   271	5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-
   272	   equivalent) — the same code with roles flipped; the parity suite
   273	   reruns with no per-direction test code.
   274	6. **otp-6 mirror + filters** on the session (one delete rule).
   275	7. **otp-7 resume** block phase (ordering + stale-partial pins per
   276	   the Design's RELIABLE exception). Slice design: `docs/plan/OTP7_RESUME.md`
   277	   (staged 7a in-stream / 7b data-plane).
   278	8. **otp-8 fallback byte-carrier** (control-stream frames) as the
   279	   session's alternate transport.
   280	9. **otp-9 delegated transfer** = daemon-initiated session; bespoke
   281	   delegated-pull driver retired behind the existing gate;
   282	   `DelegatedPull` RPC reduced to trigger + progress relay.
   283	10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
   284	    operation through the session; `Push`/`PullSync` and all four
   285	    drivers deleted from the tree and the proto, no bridge
   295	13. **otp-13 verdict**: acceptance checklist walked with the owner;
   296	    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
   297	    against the unified baseline — owner call at that point.
   298	
   299	## Open questions
   300	
   301	- None requiring owner input now — scope, wire, and process were
   302	  delegated (Directive section). Slice-level unknowns (exact proto
   303	  shapes, resume edge semantics, TUI event wiring) are settled inside
   304	  their slices through the codex loop. — owner

exec
/bin/zsh -lc 'rtk git diff --check 70c9688''^ 70c9688 && rtk git diff-tree --no-commit-id --name-status -r 70c9688' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:

git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-FKCgrKJo' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Ub7b060r' (errno=Operation not permitted)
M	TODO.md
M	docs/DECISIONS.md
M	docs/STATE.md
M	docs/plan/OTP7_RESUME.md

exec
/bin/zsh -lc "rtk git show 70c9688:docs/plan/OTP7_RESUME.md | nl -ba | sed -n '135,195p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   135	  role suite runs both initiator assignments, as for every prior slice.
   136	
   137	## Staging
   138	
   139	- **otp-7a — resume over the in-stream carrier.** Fully exercisable in
   140	  `transfer_session_roles.rs` (both initiator roles, in-stream). Un-stub the
   141	  refusals; implement the choreography + block-diff helper + DEST hash-send + apply
   142	  wiring over the control-lane `BlockTransfer`/`Complete` frames; `files_resumed`.
   143	  Pins: happy-path partial, identical-file (zero blocks), stale-partial fallback,
   144	  mid-resume-failure.
   145	- **otp-7b — resume over the TCP data plane.** Port the block records onto the data
   146	  plane (`data_plane.rs::send_block`/`send_block_complete` already exist) with the
   147	  same choreography; e2e in the daemon harness. Follows 7a exactly as otp-4b-1→4b-2
   148	  and otp-5b-1→5b-2 did.
   149	
   150	## Guard-proof targets (the plan's mandate: "pins the stale-partial and
   151	mid-resume-failure cases")
   152	
   153	1. **Partial resume** — a multi-block file with some blocks already correct at the
   154	   dest: only the changed blocks move (assert BlockTransfer count / bytes), final
   155	   bytes identical, `files_resumed == 1`. Guard: neuter the block-diff so it sends
   156	   all blocks ⇒ the "only changed blocks moved" assertion FAILS.
   157	2. **Identical file** — zero blocks transferred, file untouched, still counted done.
   158	3. **Stale-partial fallback** — a dest partial that shares no blocks with the source
   159	   ⇒ full content lands, bytes identical, no hang/fault. Guard: force the source to
   160	   trust the stale hashes ⇒ corrupt output.
   161	4. **Mid-resume-failure** — inject a source fault mid-block-phase ⇒ a clean
   162	   `SessionFault` surfaces to both ends, `files_resumed` not incremented for the
   163	   aborted file, no deadlock.
   164	
   165	## Open questions — RESOLVED (owner, 2026-07-09; D-2026-07-09-1)
   166	
   167	- **Q1 — contract wins.** Stale/mismatched partial degrades gracefully to a
   168	  full-file transfer, never an abort. Owner's principle (quoted in the header)
   169	  is the rationale; D1 stands as written.
   170	- **Q2 — keep in-place patch, surface at end of op.** No temp+rename atomicity
   171	  for otp-7 (parity with the code being replaced). The owner's rider: the fault
   172	  is surfaced in the end-of-operation summary with a re-run suggestion — see D4.
   173	  No atomicity follow-up filed; convergence-on-retry is the reliability model.
   174	- **Q3 — 7a then 7b, no collapse.** Owner: "confirmed. no collapse. keep the
   175	  reviewloop codex playbook going slice by slice."
   176	
   177	## Verification (when Active)
   178	
   179	- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
   180	  `cargo test --workspace` (count must not drop).
   181	- Windows parity after touching `win_fs`/sink paths: `scripts/windows/run-blit-tests.ps1`.
   182	- Each sub-slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`), guard proof
   183	  per pin above.

exec
/bin/zsh -lc "rtk git show 70c9688:docs/TRANSFER_SESSION.md | nl -ba | sed -n '70,100p;200,225p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    70	  |     refusal is a SessionError, never a silent close
    71	  |                                        |
    72	  |==== from here the lanes are ROLES, not initiator/responder ====|
    73	  |  (whichever end holds SOURCE sends source-lane frames,          |
    74	  |   regardless of which end opened the RPC)                       |
    75	  |                                                                 |
    76	  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
    77	  |  DEST streams:    NeedBatch* ... NeedComplete                  |
    78	  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
    79	  |                   frames when the in-stream carrier is chosen) |
    80	  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
    81	  |                                                                 |
    82	  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
    83	  |  `resume=true` is followed by DEST's BlockHashList for that     |
    84	  |  file BEFORE SOURCE may send any byte of that file; stale or    |
    85	  |  mismatched partials fall back to full-file transfer.           |
    86	  |                                                                 |
    87	  |  mirror: DEST computes deletions LOCALLY from the completed     |
    88	  |  source manifest (filter-scoped, scan-complete-guarded) and     |
    89	  |  executes them itself. No delete list crosses the wire.         |
    90	  |                                                                 |
    91	  |  CLOSING (role-directed, both initiator layouts):               |
    92	  |    SOURCE -> DEST:  SourceDone (all requested payloads flushed) |
    93	  |    DEST -> SOURCE:  TransferSummary (DEST is the scorer)        |
    94	  |  then the INITIATOR closes the RPC stream.                      |
    95	```
    96	
    97	- Phase violations (a frame arriving in a phase where its role may
    98	  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
    99	  fail-fast, no tolerant parsing.
   100	- `NeedComplete` is DESTINATION's promise that no further need
   200	  ordering rule applies identically to both roles and mirrors the
   201	  design-4-proven fallback ordering, so manifest frames and payload
   202	  records never interleave. DESTINATION-lane frames (need batches,
   203	  acks, summary) are unaffected — they travel the other direction.
   204	- **Local (in-process):** the identical session state machine runs
   205	  with both roles in one process over an in-process frame channel —
   206	  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
   207	  file vs block) is planner-owned and reads workload shape +
   208	  capability, never role/initiator/transport.
   209	
   210	## Errors, cancel, stall
   211	
   212	- `SessionError{code, message}` codes (plus both build ids on
   213	  BUILD_MISMATCH):
   214	  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
   215	  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
   216	  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
   217	  or aborts says why before closing; operators never diagnose from a
   218	  bare stream reset.
   219	- `CancelJob` interop: the responder registers the session in
   220	  `ActiveJobs` at OPEN (same transfer_id contract as today); the
   221	  cancel token races the session exactly as w4-3 wired, and the
   222	  peer receives `SessionError{CANCELLED}`.
   223	- StallGuard, byte-accounting, and progress events (w6-1 contract)
   224	  attach at the same boundaries they do today; the session emits the
   225	  existing `DaemonEvent` payloads.

exec
/bin/zsh -lc 'rtk git diff 70c9688''^ 70c9688 --unified=80 -- docs/plan/OTP7_RESUME.md docs/STATE.md docs/DECISIONS.md TODO.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
TODO.md                  | 17 ++++++++++++-----
 docs/DECISIONS.md        |  5 +++++
 docs/STATE.md            | 24 +++++++++++-------------
 docs/plan/OTP7_RESUME.md | 37 +++++++++++++++++++++++--------------
 4 files changed, 51 insertions(+), 32 deletions(-)

diff --git a/TODO.md b/TODO.md
index b933835..a4dc893 100644
--- a/TODO.md
+++ b/TODO.md
@@ -192,165 +192,172 @@ explicitly-deferred logging epic (F15).
       so the corruption happens specifically at the `PathBuf → String`
       relative-path conversion, not at enumeration. `relative_path_to_posix`
       is the single canonical helper for this and is called from
       `engine/mirror.rs`, `mirror_planner.rs`, `remote/transfer/{payload,tar_safety}.rs`,
       `remote/endpoint.rs`, and `remote/push/client/helpers.rs` — i.e.
       local mirror and remote push both go through the lossy path, so
       this isn't a remote-only/proto-only constraint. Fix needs a
       design call (owner input required, `plan` this before coding):
       the wire `FileHeader.relative_path`/`FileBlock.relative_path`
       fields are proto3 `string`, which is UTF-8-only at the gRPC
       layer, so a full fix for the remote path needs an encoding
       scheme that round-trips arbitrary bytes through a UTF-8-safe
       string (e.g. percent-encode invalid bytes, or WTF-8) — local
       mirror has no such wire constraint and could preserve raw
       `OsString`/`PathBuf` throughout instead. Same failure class as
       audit-17 (one bad filename kills the entire run instead of
       being skipped/reported) — whatever skip/report/fail-fast
       behavior gets designed for audit-17 should likely cover this
       case too, but the root cause here is enumeration-side path
       corruption, not destination-fs charset rejection, so treat as
       a separate fix even if the error-handling policy ends up shared.
 - [ ] **audit-19** `--exclude` silently matches nothing for the path
       forms users actually type (absolute paths, and bare directory
       names), so an exclude that looks correct transfers the excluded
       tree anyway. Reported: `blit mirror /home/michael/
       /run/media/michael/USB_DRIVE -pvy --force --exclude
       /home/michael/.java` still descended into `.java/` and tried to
       write `.java/fonts/1.8.0_472/fcinfo-…-en.properties` — that write
       then hit `audit-17`'s `os error 22` (`sink.rs:608`) on the
       FAT-family destination, i.e. a working `--exclude` would also have
       side-stepped that crash. The filter *is* plumbed (local mirror
       enumerates via `FileFilter`, `transfers/local.rs:191` →
       `enumerate_directory_filtered`); this is a matching-semantics bug,
       not a dropped-filter plumbing bug. Two compounding root causes in
       `FileFilter::allows_entry` (`crates/blit-core/src/fs_enum.rs:194-240`):
       (1) **excludes are matched against the source-root-relative path
       and the bare filename, never the absolute path** — `path_str` is
       `rel_path` (`fs_enum.rs:211-213`), `filename` is
       `abs_path.file_name()` (`fs_enum.rs:207-210`). The candidate
       strings for this entry are `.java/fonts/…` (relative) and
       `fcinfo-…properties` (filename); a literal `/home/michael/.java`
       glob equals neither (globset needs a whole-string match, and
       `glob_match` with no `*` falls through to exact equality,
       `fs_enum.rs:399-417`). `--exclude` maps only to `exclude_files`
       (`crates/blit-app/src/transfers/filter.rs:42`); nothing strips the
       source prefix to make an absolute pattern relative, so an absolute
       exclude under the source root is structurally unmatchable.
       (2) **a directory pattern does not prune its subtree.** Even the
       "correct" relative form `--exclude .java` only drops an entry
       whose relative path or filename is exactly `.java`; the files
       under it are `.java/fonts/…` and globset `*` does not cross `/`,
       so the whole subtree still transfers. There is **no `--exclude-dir`
       flag** (verified: no CLI arg in `blit-cli`/`blit-app`, and
       `FileFilter::exclude_dirs` / `dir_globs` / `should_include_dir`
       at `fs_enum.rs:274-295` is never assigned anywhere in `crates/`),
       so the only incantation that works today is
       `--exclude '.java/**'` (and likely `--exclude .java` too, for the
       dir entry itself). Nothing warns that a pattern matched zero of
       the configured globs — a silent no-op, same foot-gun class as the
       endpoint-parse open question in `docs/STATE.md`. Docs gap:
       `--help` says only "Exclude files matching this glob pattern"
       (`crates/blit-cli/src/cli.rs:292`) with no hint that matching is
       source-relative (not absolute) or that a directory needs `/**`;
       rsync users reasonably expect leading-`/`-anchors-to-transfer-root
       and trailing-`/`-matches-dir semantics, none of which blit
       implements. Confirmed by reading the matcher end-to-end, not run.
       Fix needs a design call (owner input required, `plan` this before
       coding): options span (a) accept absolute patterns under the
       source root by stripping the source prefix before matching;
       (b) give directory patterns rsync-like subtree semantics and/or
       add a real `--exclude-dir`; (c) at minimum, warn when a pattern is
       structurally unmatchable (absolute but not under the source, or
       literal with no possible relative/filename match) instead of
       silently transferring everything. Whatever is chosen must apply
       uniformly across local-mirror, push, pull, and remote-remote (all
       route through the one `FileFilter`/`FilterInputs` chokepoint,
       `cli.rs:288-291`) and ship `--help`/manpage/README updates in the
       same change (docs-after-behavior rule). Distinct from `audit-17`
       (destination-fs charset rejection) — that crash is only a
       *symptom* here; the exclude no-op is the reported bug.
-- [ ] **CLI transfer output redesign** (owner, 2026-07-06): current
-      `blit copy`/`mirror` output "doesn't convey any useful information
-      at all" — owner wants something closer to `rclone`/`cargo`: a
-      persistent stat block at a static screen location, plus a scrolling
-      list of in-flight/recent filenames, instead of what exists today.
+- [ ] **CLI transfer output redesign** (owner, 2026-07-06; re-confirmed
+      2026-07-09): current `blit copy`/`mirror` output "doesn't convey any
+      useful information at all" — owner wants something closer to
+      `rclone`/`cargo`: "a coherent info block with stats and a scrolling
+      list of files in a frame below, so probably a TUI?" (owner wording,
+      2026-07-09) — i.e. a persistent stat block at a static screen
+      location, plus a scrolling list of in-flight/recent filenames,
+      instead of what exists today. 2026-07-09 context: the owner hit this
+      while settling otp-7's error-surfacing question — "the current
+      progress display is absolutely useless for this". The narrow
+      end-of-operation fault summary (name failed files, suggest re-run)
+      ships with otp-7 (D-2026-07-09-1) and is NOT gated on this redesign.
       Confirmed by reading the actual code — there is no persistent/redraw
       rendering anywhere in the transfer output path, only plain
       scrolling `println!`/`eprintln!` lines: (1) the local/streaming-manifest
       path's spinner + `"Enumerated N entries… (streaming manifest)"`
       heartbeat (`crates/blit-core/src/remote/push/client/helpers.rs:176`,
       the same call site audit-16 already flagged for its own separate
       `--verbose`-gating bug); (2) the remote-transfer progress path's
       once-a-second `"[progress] N/M files • X MiB copied • Y MiB/s avg •
       Z MiB/s current"` line (`crates/blit-cli/src/transfers/remote.rs:33-140`,
       `spawn_progress_monitor_with_options`), which just reprints a new
       line every tick rather than redrawing in place. Neither path shows
       a file list, a static stat panel, or does any cursor
       repositioning — every line is transient and scrolls off, which
       matches the owner's complaint exactly. This is a real UX/design
       project, not a bug fix: likely needs a terminal-rendering approach
       (raw ANSI cursor save/restore, or a crate like `indicatif`), has to
       cover both the local and remote transfer paths above, has to decide
       a fallback for non-TTY/`--json`/piped output (today's plain-line
       output is presumably what scripts already parse — a redesign must
       not break `--json` consumers), and touches `blit-cli`+`blit-app`.
       Not designed here — needs its own `plan` before any code, per this
       repo's governance (code changes require an approved plan) and the
       Review policy (D-2026-07-04-1, all code through the codex loop).
       Distinct from `docs/plan/TUI_REWORK.md` (Active), which is about
       the separate interactive `blit-tui` navigation app, not this
       inline CLI progress output during a transfer.
 
 ### Deferred design calls
 
 These are intentionally not next-actionable. Don't pick them up
 without the listed prerequisite — they're tracked here so they
 don't get lost, not so the next agent reimplements them on a hunch.
 
 - [x] **Remote→remote re-evaluation** — resolved by
   `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Phase 1 (`15991ed`)
   added the `DelegatedPull` wire/gate/daemon path; Phase 2 makes
   remote→remote CLI transfers direct by default, keeps
   `--relay-via-cli` as the explicit escape hatch, and pins byte-path
   isolation plus stale-dst/gate/source-refusal no-fallback behavior
   with CLI-side counters.
 
 ## Phase 0: Workspace & Core Logic Foundation
 
 - [x] Initialize Cargo workspace with `blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`.
 - [x] Port `checksum.rs` to `blit-core`.
 - [x] Port `fs_enum.rs` and `enumeration.rs` to `blit-core`.
 - [x] Port `mirror_planner.rs` to `blit-core`.
 - [x] Port `buffer.rs` to `blit-core`.
 - [x] Extract zero-copy primitives into `blit-core/src/zero_copy.rs`.
 - [x] Port unit tests for ported modules.
 
 ## Phase 1: gRPC API & Service Scaffolding
 
 - [x] Create `proto/blit.proto` with the full API definition.
 - [x] Create `build.rs` in `blit-core` to compile the protocol.
 - [x] Add `tonic-build` dependencies to `blit-core/Cargo.toml`.
 - [x] Create `generated` module structure in `blit-core`.
 - [x] Implement skeleton `blitd` server binary in `blit-daemon`.
 - [x] Implement skeleton `blit` CLI binary in `blit-cli`.
 - [x] Add a minimal integration test to verify client-server connection.
 
 ## Phase 2: Orchestrator & Local Operations
 
 - [x] Create `orchestrator.rs` in `blit-core`.
 - [x] Implement the `TransferOrchestrator` struct and `new` method.
 - [x] Implement `execute_local_mirror` method on the orchestrator.
 - [x] Port consolidated path modules (`copy`, `tar_stream`, `transfer_*`, `local_worker`, `logger`, `delete`, `win_fs`) from v1 into `blit-core`.
 - [x] Wire the `blit-cli` `mirror` and `copy` commands to the orchestrator.
 - [x] Refactor `TransferFacade` and planner into streaming producer with heartbeat flushes.
 - [x] Implement 10 s stall detection and progress messaging in orchestrator.
 - [x] Implement fast-path routing for tiny/huge manifests in orchestrator.
 - [x] Add adaptive predictor + local performance history store with `blit diagnostics perf`.
 - [x] Remove `--ludicrous-speed` behaviour (make no-op) and add CLI progress UI.
 - [x] Update unit/integration tests to cover fast-path routing and predictor logic.
 
 ## Phase 2.5: Performance & Validation Checkpoint
 
 - [x] Create benchmark script for local mirror performance.
 - [x] Run and compare against v1. (2025-10-16: v2 ~1.93× slower; optimization needed before GO)
 - [x] Analyse Windows ETW traces (wingpt-4/5.md findings logged) (`logs/blit_windows_bench.zip`) and propose copy-path optimisations.
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index c86a7ce..dd2a783 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -68,80 +68,85 @@ Format:
 - Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
 - Why: review program (D-2026-06-11-1) delivered all three phases; owner is the gate for queue entry and exercised it in full.
 - Supersedes: nothing (completes D-2026-06-11-1; `DESIGN_COHERENCE_REVIEW.md` flips Active → Shipped).
 
 ## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
 - Decision: `zero_copy.rs` is deleted rather than wired in. The w8-1b evaluation (`docs/plan/ZERO_COPY_RECEIVE_EVAL.md`) recommended deletion and the owner agreed (2026-06-12 session). The deletion executes inside w8-1 once the w5-1 sentinel (lib.rs) is graded — it is no longer excluded from that sweep.
 - Why: the dead draft busy-waits on EAGAIN (would be rewritten, not revived); wiring needs a raw-fd special case beside a permanent buffered fallback; the CPU saving is a fraction of one core, Linux-only, and unmeasured. Revisit gate: 10 GbE benchmarks showing receive-side CPU saturation — design notes preserved in the eval doc.
 - Supersedes: D-2026-06-11-2 item (b) (zero_copy exclusion from W8.1 was pending this evaluation; the evaluation is done).
 
 ## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
 - Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
 - Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
 - Scope consequence: this **moots the standalone premise** of the queued incremental work and absorbs the goals — w2-2 (three ladders → one dial) is `ue-1b`; w2-3 multi-stream pull (`MULTISTREAM_PULL.md`) is `ue-1d` via the unified sequencer; w2-4 (delete deprecated Pull RPC) is `ue-1e`; adaptive-streams cherry-pick is `ue-1a`. `MULTISTREAM_PULL.md` is superseded as a standalone plan (kept as reference); its goal survives inside this plan. The design-review queue's correctness findings (w4-1 etc.) are independent and unaffected.
 - Supersedes: the "ground-up redesign" framing of the 2026-06-14 open question recorded in STATE.md (that open question is now closed); `MULTISTREAM_PULL.md` as a standalone plan (goal absorbed into `UNIFIED_TRANSFER_ENGINE.md` slice `ue-1d`).
 
 ## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
 - Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` is **Active**. Owner approved with four parameters that bind the design: (q1) **no probe-then-go phase** — the engine starts moving within ~1s at conservative defaults bounded by the receiver ceiling and the tuner adjusts dials live from the first byte; the "small-transfer threshold" is obviated (no probe to skip), and the **planner** carries the workload-shape judgment (file count vs bytes) that the old size gate proxied. (q2) the receiver advertises a **rich capacity profile** (CPU cores, disk class, load, max streams, drain estimate) — "more data serves the ubergoal"; do not minimize the negotiation payload. (q3) engine type **deferred to the agent**, who recommends a new src/dst-agnostic `TransferEngine` + a local adapter over renaming `TransferOrchestrator` in place — ratified at `ue-1c`. (q4) `ue-2` (mid-transfer stream add/drop via PR3's resize proto) is **in scope at Active**, sequenced last; 11 months of owner benchmarking is the justification, the 10 GbE rig is sign-off not a gate.
 - Why: owner answered the four gating questions (the stated Draft→Active condition) and said "active now." q1 materially improved the design — live-from-first-byte removes the fragile size threshold and collapses the A/B/C probe staging into "adjust what is cheap in `ue-1b`, add stream resize in `ue-2`."
 - Inference flagged for owner (now vetoed — see D-2026-06-20-3): the agent had proposed folding the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b) in as the planner half and superseding its "after audit Round 1" timing. **Owner vetoed 2026-06-20.** The absorption is dropped; D-2026-06-04-3 stands unchanged. The engine's workload-shape-awareness + first-byte-within-~1s requirements remain, stated on their own merits, not as the H10b concept.
 - Supersedes: the "A-first warmup probe" and "size-gated skip-probe" framings in the Draft version of `UNIFIED_TRANSFER_ENGINE.md` (already edited in-place). *(The proposed supersession of D-2026-06-04-3's streaming-planner timing is withdrawn per the owner veto — see D-2026-06-20-3.)*
 
 ## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
 - Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
 - Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
 - Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).
 
 ## D-2026-06-20-4 — Unified transfer engine plan review freeze
 - Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
 - Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
 - Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.
 
 ## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
 - Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
 - Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
 - Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*
 
 ## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
 - Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
 - Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
 - Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.
 
 ## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
 - Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
 - Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
 - Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).
 
 ## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
 - Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
 - Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
 - Supersedes: nothing (closes the STATE.md "commit erratum" blocked item).
 
 ## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
 - Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
 - Why: owner call 2026-07-04 ("flip it"). The original "disconnect is the cancel" rationale predates w4-3's race wiring; the flip is now policy-only, and cancel-from-anywhere (second terminal, TUI) is strictly more operable than find-and-kill-the-client.
 - Supersedes: the DelegatedPull-only cancellation policy recorded in `active_jobs.rs`'s `supports_cancellation` rustdoc (edited when the slice lands) and the corresponding "policy deliberately unchanged" scope note in the w4-3 finding doc (which anticipated exactly this flip).
 
 ## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
 - Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
 - Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
 - Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).
 
 ## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
 - Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
 - Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
 - Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.
 
 ## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
 - Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
 - Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
 - Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).
 
 ## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
 - Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
 - Why: the 10 GbE session showed skippy's 32-core receiver at 1.43 cores — gate not met on that rig — but the gate was always about CPU-bound receivers, and the owner now operates one. On a CPU-bound receiver, cutting the userspace copy is exactly the FAST lever the eval preserved design notes for.
 - Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.
 
 ## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
 - Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
 - Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
 - Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).
+
+## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
+- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
+- Why: owner answered Q1–Q3 in session 2026-07-09; the flip is the approval the plan procedure requires. In the same exchange the owner re-confirmed the broader progress-display redesign (persistent stats block + scrolling file frame, "probably a TUI") — that stays a queued TODO.md item ("CLI transfer output redesign"), NOT otp-7 scope, and needs its own plan.
+- Supersedes: nothing (the plan doc's Open-questions section is rewritten as resolved in the same commit).
diff --git a/docs/STATE.md b/docs/STATE.md
index c828a43..5d3cb43 100644
--- a/docs/STATE.md
+++ b/docs/STATE.md
@@ -1,195 +1,193 @@
 # STATE — single entry point for "what is true right now"
 
-Last updated: 2026-07-06
+Last updated: 2026-07-09
 
 - 2026-07-04: Owner-approved dual push reached 3d8326b (origin: 10d89e0..3d8326b; gitea mirror: 2a77b9f..3d8326b). That push corrected a prior remote-name confusion; windows-latest CI on that push is the "meaningfully green" check referenced in prior notes.
 
-- Current session (2026-07-06): otp-6 CLOSED; otp-7 in DESIGN — slice design drafted at docs/plan/OTP7_RESUME.md (Draft). NO CODE until the owner answers Q1–Q3 and flips the plan Active. otp-6 (a/b) mirror + filters landed and graded. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
+- Current session (2026-07-09): owner answered otp-7's Q1–Q3 (D-2026-07-09-1) — docs/plan/OTP7_RESUME.md is **Active**; otp-7a (resume over the in-stream carrier) is the current slice, through the codex loop. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
 
 - Session work: filed audit-17 and audit-18; noted a CLI-output-redesign item in TODO.md; drafted+reviewed docs/plan/LOCAL_ERROR_TELEMETRY.md (Draft). A session-wide codex pass fixed 5 cross-doc staleness bugs.
 
 - Notes on push state: owner previously pushed master → GitHub at 10d89e0; local commits f6e592e..HEAD remain unpushed and windows-latest CI will ride the next push.
 
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
   open.** Progress (each through the codex loop; closed-slice detail in
   DEVLOG + `.review/` + REVIEW.md):
   - **otp-1 / otp-3 / otp-4a `[x]`** — wire+session contract
     (`docs/TRANSFER_SESSION.md`); role-parameterized drivers over the
     in-process transport (invariance property in the role suite); daemon
     serves `Transfer` as Responder, client push over gRPC; A/B
     byte-identical vs old push; SizeMtime = data-safe skip (open Q below).
   - **otp-4b (1/2/3) `[x]` — push data plane fully on the session, closed**:
     single-stream TCP data plane, mid-transfer resize/multi-stream + sf-2
     shape correction, deterministic mid-transfer cancel. Detail: DEVLOG.
   - **otp-5a `[x]`** (`84be1cc`, codex PASS) — the one served `Transfer`
     RPC serves BOTH roles via `run_responder` (SOURCE-init→daemon
     DESTINATION = push; DEST-init→daemon SOURCE = pull, in-stream).
   - **otp-5b (1/2) `[x]`** — the SOURCE-responder data plane, closed:
     5b-1 (`e6a0b3b`+`13485ee`) decoupled connection role (RESPONDER
     binds+accepts, INITIATOR dials) from byte role; 5b-2 (`d579365`+
     `773a877`) lifted the single-stream cap — the pull data plane resizes
     via sf-2 (same resize frames as push). Defaults to TCP; A/B
     byte-identical vs old `pull_sync`. Suite → **1522**.
   - **otp-6 (a/b) `[x]`** — mirror + filters on the session, closed.
     6a (`c026692`+`0bb27f5`) honors `SessionOpen.filter` via the universal
     `FilteredSource` chokepoint. 6b (`01d9c41`+`3c99557`) is the one delete
     rule: DESTINATION diffs the complete source manifest at SourceDone,
     scan-complete-guarded + filter-scoped. Codex High: keep-set now folds
     case on macOS too (case-insensitive-FS data-loss). Suite → **1529**.
-  - Current: **otp-7 IN DESIGN** — Draft `docs/plan/OTP7_RESUME.md`
-    (`9fb5e4a`) awaiting owner review (see Open questions); no code until
-    Active. otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
+  - Current: **otp-7 ACTIVE (D-2026-07-09-1)** — `docs/plan/OTP7_RESUME.md`
+    flipped Active 2026-07-09 (Q1 contract-wins fallback; Q2 in-place patch
+    + end-of-op fault summary rider; Q3 7a-then-7b). Implementing **otp-7a**
+    (resume over the in-stream carrier) through the codex loop.
+    otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
 - **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
   `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
   until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
   baseline. Principle stands: ceiling-driven, never competitor-relative
   (D-2026-07-04-4; a ≥25% margin answer was retracted — do not
   re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
 - **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
   measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s; owner
   declarations pending in Blocked); 10 GbE session done; w9-3 + review rows
   landed. Codex loop governs all changes (D-2026-07-04-1; DEVLOG 07-04/05).
 
 ## Queue (ordered)
 
 1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
    the only work item until it ships**: slices otp-1..13 through the
    codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
    otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b) `[x]`. Current:
-   **otp-7 IN DESIGN** (`docs/plan/OTP7_RESUME.md` Draft, owner review;
-   no code until Active). otp-2 (symmetric baseline) is RIG-GATED —
+   **otp-7 ACTIVE** (`docs/plan/OTP7_RESUME.md`, D-2026-07-09-1) —
+   implementing otp-7a. otp-2 (symmetric baseline) is RIG-GATED —
    before otp-10 cutover.
 2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
    Shipped (zero-copy resolved — D-2026-07-05-3). Optional owner-gated
    measurement follow-ups (Win 11 bare-metal; disk-path variants;
    >ARC-size push) — disk-path items largely absorbed by otp-2/otp-12's
    symmetric-rig matrices. Env: bench binaries at
    `skippy:/mnt/generic-pool/video/blit-bin/` (/tmp, /home noexec there).
 3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
    resumes/re-derives after ONE_TRANSFER_PATH ships.
 4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
    open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
    (mirror-executor consolidation) likely lands for free inside
    otp-6's one-delete-rule slice; re-check before picking it up.
 5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
    Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
    cutover as a runtime-selected write strategy in the unified receive
    sink (design: eval doc §If-FAST-evidence; dead module deletes in
    w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
    2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
    rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
    test data, everything; nothing written outside it, ever. Zero-copy
    is pre-authorized to be tested there when the post-cutover slice set
    reaches it; no daemon runs on zoey before then without a fresh go.
 6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
    (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
    epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
    `derive_local_plan_tuning` fold-or-retire; receive-side dial
    tuning residue (w3-1 scoped it out).
 
 ## Authoritative docs right now
 
 - **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
-  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Draft** — otp-7 slice
-  design, awaiting owner review before any code).
+  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
+  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
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
 
 - **Three 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
   holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE),
   REV4 → Shipped. (The zero-copy revisit verdict and the a/b/c
   question are RESOLVED — D-2026-07-05-3, unparked; measured skippy
   data 1.43 cores daemon-receive / 0.45 client at 9.5 Gbit/s stays
   recorded in DEVLOG + DIAGNOSIS.md.)
 - **Push go**: local commits `f6e592e`..HEAD await the ref-listing +
   approval flow; windows-latest CI on the w9-3 harness fix rides it.
 - `Cargo.lock`: fresh transitive drift (crossbeam-*, cc, etc.), same class
   as `04c9c6d` — not this session's; owner's call to commit or revert.
 
 ## Open questions
 
-- **(OPEN — owner review, 2026-07-07, otp-7)** `docs/plan/OTP7_RESUME.md`
-  (Draft) awaits the owner's Q1–Q3 (graceful stale fallback; in-place-patch
-  mid-failure model; 7a-then-7b staging — all agent-rec yes) and the flip to
-  Active. That flip unblocks otp-7 implementation.
 - **(OPEN — owner ack, 2026-07-05, otp-4a)** Unified SizeMtime semantic:
   same-size + dest-NEWER — old push clobbers, session adopts **data-safe
   SKIP** (converge-up; `--force` still overwrites; pinned by
   `same_size_newer_destination_is_skipped_not_clobbered`). Owner: confirm
   or ask for old-push clobber. Reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
 - **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
 - **(OPEN, 2026-07-04)** `725aa07` tracked a 236-file stale worktree snapshot
   (`.claude/worktrees/vigilant-mayer/`). Agent rec: `git rm -r`; awaits go.
 - **(OPEN, 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still describe
   the deleted `determine_remote_tuning`/`TuningParams` — fold into
   w10-docs-batch (agent rec) or rewrite sooner?
 - **(OPEN, ripe — data in hand)** REV4 → Shipped flip: the 10 GbE
   session delivered the measurement evidence; flip awaits the three
   declarations in Blocked (was four — zero-copy resolved,
   D-2026-07-05-3).
 - **(OPEN, new 2026-07-05)** CLI foot-gun found during the session:
   `blit copy src_large dst` with an existing local dir, no `./`,
   parses the bare name as an mDNS discovery endpoint and errors
   "remote source must include a module or root"
   (blit-app endpoints.rs). Should local-path existence win over the
   discovery interpretation, or at least improve the error? Candidate
   review-queue row; owner to slot.
 - **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
   locally; daemon-spawn e2e flakiness root-caused + fixed on Linux (w9-3:
   port-TOCTOU race + cargo-lock contention). Remaining: windows-latest CI
   on the next push (10d89e0 predates the w9-3 fix).
 
 ## Handoff log (newest first, keep ≤ 3)
 
 - **2026-07-06 (39th)** @ `598f102` — **Session-wide codex review (5
   findings, all fixed); one new backlog item filed; otp-7 still
   untouched.** `/playbook reviewloop` named a generic template this repo's
   own guidance says isn't the operative loop here (branch-per-finding
   conflicts with no-agent-branches) — ran `GPT_REVIEW_LOOP.md`'s mechanism
   instead over the whole session diff (`9876687..44de868`). 3 Medium + 2
   Low, all cross-doc staleness/contradictions from mid-session edits not
   propagating everywhere (stale Q5 header, a STATE/plan-doc contradiction,
   a stale handoff entry, a date-drift note, an overstated claim) — fixed
   `419f5d1`, verdict `.review/results/session-2026-07-06.gpt-verdict.md`.
   Also: owner asked for a CLI transfer-output redesign (rclone/cargo-style
   static stat block + file list; current output is bare scrolling
   `println!`s, `helpers.rs:176`/`transfers/remote.rs:33-140`) — filed to
   `TODO.md` only (`598f102`), needs its own `plan` when picked up.
   **Exact first action next
   session**: otp-7 — owner's Q1–Q3 on `docs/plan/OTP7_RESUME.md`, flip
   Active, codex-review, implement otp-7a. In-flight: none. Done since
   38th: the session-wide review pass; the CLI-output-redesign TODO item.
 - **2026-07-06 (38th)** @ `44de868` — **`LOCAL_ERROR_TELEMETRY.md` drafted
   + reviewed twice (3+3 findings fixed), Q1-Q5 resolved; still Draft, no
   code.** Full detail: DEVLOG 20:15Z/21:00Z entries and the plan doc's own
   Q5 section (pickup timing); the 39th entry above covers the staleness
   bugs this left behind, since fixed. Done since 37th: audit-17/18 filed
   (`5628c03`, `deb3800`); the telemetry plan end-to-end.
diff --git a/docs/plan/OTP7_RESUME.md b/docs/plan/OTP7_RESUME.md
index c794426..3634177 100644
--- a/docs/plan/OTP7_RESUME.md
+++ b/docs/plan/OTP7_RESUME.md
@@ -1,174 +1,183 @@
 # otp-7 — resume block phase (design)
 
-**Status**: Draft
+**Status**: Active (owner Q1–Q3 answered + "confirmed", 2026-07-09; D-2026-07-09-1)
 **Created**: 2026-07-07
 **Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-7.
 **Contract**: `docs/TRANSFER_SESSION.md` (resume exception + frame table, pinned otp-1).
-**Governs**: no code until the owner flips this to `**Status**: Active`
-(AGENTS.md; `.agents/repo-guidance.md` plan operator). Per D-2026-07-04-1 this
-plan change also goes through the codex loop.
+**Governs**: implementation proceeds 7a → 7b, one slice per codex loop pass
+(D-2026-07-04-1). Owner's deciding principle, quoted: "FAST, SIMPLE, RELIABLE
+file transfer. if we abort the whole thing when we could have fixed or
+surfaced a single error, we are violating all of those."
 
 ## Why this doc
 
 otp-7 is the plan's **explicit RELIABLE exception**: resumed files use a
 strictly-ordered block-hash exchange, and the choreography is novel (unlike the
 mechanical carrier splits of otp-4b/5b/6). The owner asked for the design on paper
 before the intricate code. This doc records the choreography, the reuse map, the
 design decisions (most already settled by the contract), the staging, and the
 guard-proof targets — so implementation is a transcription, not a discovery.
 
 ## What resume is (contract, already pinned in otp-1)
 
 A `NeedEntry` may be flagged `resume=true`. For such a file the DESTINATION sends
 its `BlockHashList` (Blake3 per block of the existing partial) and the SOURCE
 **must not send any byte of that file until it has received that list**. The SOURCE
 then transfers only the blocks whose hashes differ (or that the dest lacks), as
 `BlockTransfer` records, ending with `BlockTransferComplete{total_bytes}`. Stale or
 mismatched partials fall back to full-file transfer.
 
 Frames (field numbers frozen, `TRANSFER_SESSION.md`): `8 BlockHashList` (DEST),
 `14 BlockTransfer` (SOURCE), `15 BlockTransferComplete` (SOURCE). `SessionOpen.resume`
 carries `ResumeSettings{enabled, block_size}`. `NeedEntry.resume` is field 2.
 
 ## What already exists (reused verbatim — no reinvention)
 
 - **Wire frames + payload enums**: `BlockHashList`/`BlockTransfer`/`BlockTransferComplete`
   frames and `PreparedPayload::{FileBlock, FileBlockComplete}` are defined and
   name-mapped in the session (`transfer_session/mod.rs:250,256,257`).
 - **DEST apply (reassembly)**: `FsTransferSink::write_file_block_payload`
   (`sink.rs:641`, seek+write into the partial in place) and
   `write_file_block_complete` (`sink.rs:687`, `set_len` + fsync + stamp mtime/perms).
   In-place patch of the partial — the partial IS the destination; no temp+rename
   (matches the old pull client).
 - **DEST block hashing**: `compute_block_hashes` (`remote/pull.rs:1139`) — streams
   the partial in `block_size` chunks, `blake3::hash`, returns 32-byte digests; an
   absent file returns an empty vec (the implicit full-file fallback).
 - **Block-diff reference**: `resume_copy_file` (`copy/file_copy/resume.rs:52`) is the
   canonical block-compare (write a block iff beyond dst len, a partial tail, or
   hashes differ; truncate if dst longer). The SOURCE-side diff is the same logic.
 - **Defaults**: `DEFAULT_BLOCK_SIZE` = 1 MiB, `MAX_BLOCK_SIZE` = 64 MiB
   (`copy/file_copy/resume.rs:16,19`). `ResumeSettings.block_size == 0` ⇒ default.
 
 ## What is new (the otp-7 work)
 
 1. **Un-stub the four refusal sites**: both open validators (`mod.rs:362,401`), the
    source recv-half resume-need rejection (`mod.rs:799`), and the outbound-planner
    FileBlock bail (`mod.rs:1446`).
 2. **The strict-ordering exchange choreography** in the session's source/dest halves.
 3. **A home for the SOURCE-side block-diff** — today hand-rolled in `pull_sync.rs`,
    not on any trait (see Design decision D3).
 
 ## Choreography (strict ordering)
 
 ```
 DESTINATION (diff loop)                     SOURCE (send half)
 ─────────────────────────                   ──────────────────
 for each manifest entry:
   if resume-eligible (see D2):
      NeedEntry{path, resume=true} ───────►  recv: ResumeNeed(header)
      BlockHashList{path, bsz, hashes} ───►  recv: BlockHashes(path, hashes)
                                             (send half correlates the two;
                                              a resume need is HELD until its
                                              BlockHashList arrives — the
                                              RELIABLE ordering guarantee)
   else:
      NeedEntry{path, resume=false} ──────►  recv: Need(header)  (unchanged)
 
                                             for a held resume need + its hashes:
                                               read source file block-by-block,
                                               blake3 each; for block i where
                                               i >= hashes.len() OR hash != hashes[i]:
   recv BlockTransfer{path,off,bytes} ◄────    send BlockTransfer{path, off, block}
      sink.write_file_block_payload            (in-stream carrier: control-lane
                                                frames; data-plane: send_block, 7b)
   recv BlockTransferComplete{path,total} ◄─  send BlockTransferComplete{path,total}
      sink.write_file_block_complete
      files_resumed += 1
 ```
 
 The source's per-file byte phase for a resume need is "send changed blocks then
 complete", replacing the whole-file record it sends for a non-resume need. Ordering
 is enforced on the SOURCE: it will not emit a block for a path before it holds that
 path's `BlockHashList` (fail-fast if a block phase would start without one).
 
 ## Design decisions
 
 - **D1 — stale/mismatched partial ⇒ graceful full-file fallback**, per the contract
   (`TRANSFER_SESSION.md:84`), NOT the hard `Status::internal` the old *data-plane*
   path uses (`pull_sync.rs:1377`) — that is a pre-cutover quirk the gRPC path already
   contradicts (`pull_sync.rs:1544`, graceful). An empty / short / all-mismatched
   hash list simply means "send all blocks" = full transfer. **Reconcile in favor of
   the contract.**
 - **D2 — resume eligibility** (which needs get `resume=true`): the file exists at the
   dest as a non-empty partial AND `ResumeSettings.enabled` AND the compare says the
   file must transfer (changed). A missing/empty dest file is a normal full transfer
   (no resume flag, no BlockHashList). This mirrors the daemon's `effective_resume`
   set (`pull_sync.rs:262`) minus the mtime-only-touch special case, which the session
   already handles via SizeMtime skip.
 - **D3 — SOURCE block-diff home**: a free helper in the session
   (`resume_block_diff(source, header, dest_hashes, block_size) -> stream of blocks`)
   rather than a new `TransferSource` trait method. Rationale: it needs only
   `source.open_file(header)` (already on the trait) + blake3, and keeping it out of
   the trait avoids every future `TransferSource` impl re-implementing it (the same
   reasoning that made `FilteredSource` the one filter chokepoint in otp-6a). Flag for
   codex: confirm the helper doesn't belong on the trait.
 - **D4 — mid-resume-failure**: block writes patch the partial in place (no
   temp+rename, matching the old client). A fault mid-block-transfer surfaces as a
   `SessionFault` (peer-notified) and aborts; the partial is left partially patched,
   and the NEXT resume re-syncs via a fresh block-hash exchange (the partial's new
   hashes reflect whatever landed). The pin asserts the fault surfaces cleanly and no
   file is falsely counted `files_resumed`. (No stronger atomicity than the code we
   are replacing — called out as a Known gap, not a regression.)
+  **Owner rider (2026-07-09, Q2)**: the fault must also appear in the CLI's
+  **end-of-operation summary** — naming the affected file(s) and suggesting a
+  re-run to converge — not only as a mid-stream line that scrolls away. Small
+  CLI-layer deliverable, lands within otp-7 (the session already collects the
+  per-file fault; this is about where it is reported). The full progress-display
+  redesign it brushes against is a separate queued item (TODO.md "CLI transfer
+  output redesign") and is NOT in otp-7 scope.
 - **D5 — block size**: `ResumeSettings.block_size` clamped to `MAX_BLOCK_SIZE`, `0` ⇒
   `DEFAULT_BLOCK_SIZE`. The DEST chooses (it hashes first); the SOURCE reads the size
   from the `BlockHashList`, so the two never disagree.
 - **D6 — invariance**: resume runs identically whichever end initiated (the flag is
   in the open; the DEST computes hashes and applies; the SOURCE diffs and sends). The
   role suite runs both initiator assignments, as for every prior slice.
 
 ## Staging
 
 - **otp-7a — resume over the in-stream carrier.** Fully exercisable in
   `transfer_session_roles.rs` (both initiator roles, in-stream). Un-stub the
   refusals; implement the choreography + block-diff helper + DEST hash-send + apply
   wiring over the control-lane `BlockTransfer`/`Complete` frames; `files_resumed`.
   Pins: happy-path partial, identical-file (zero blocks), stale-partial fallback,
   mid-resume-failure.
 - **otp-7b — resume over the TCP data plane.** Port the block records onto the data
   plane (`data_plane.rs::send_block`/`send_block_complete` already exist) with the
   same choreography; e2e in the daemon harness. Follows 7a exactly as otp-4b-1→4b-2
   and otp-5b-1→5b-2 did.
 
 ## Guard-proof targets (the plan's mandate: "pins the stale-partial and
 mid-resume-failure cases")
 
 1. **Partial resume** — a multi-block file with some blocks already correct at the
    dest: only the changed blocks move (assert BlockTransfer count / bytes), final
    bytes identical, `files_resumed == 1`. Guard: neuter the block-diff so it sends
    all blocks ⇒ the "only changed blocks moved" assertion FAILS.
 2. **Identical file** — zero blocks transferred, file untouched, still counted done.
 3. **Stale-partial fallback** — a dest partial that shares no blocks with the source
    ⇒ full content lands, bytes identical, no hang/fault. Guard: force the source to
    trust the stale hashes ⇒ corrupt output.
 4. **Mid-resume-failure** — inject a source fault mid-block-phase ⇒ a clean
    `SessionFault` surfaces to both ends, `files_resumed` not incremented for the
    aborted file, no deadlock.
 
-## Open questions for the owner
-
-- **Q1**: D1 (graceful stale fallback) reconciles the old data-plane hard-error
-  against the contract. Confirm the contract wins (agent rec: yes — it is the pinned
-  wire behavior and the safer one).
-- **Q2**: D4 keeps the old in-place-patch failure model (no temp+rename atomicity).
-  Acceptable as-is for otp-7, or do you want atomic partial-file handling as a
-  follow-up item? (agent rec: keep parity now, file a follow-up if wanted.)
-- **Q3**: Staging — 7a (in-stream) then 7b (data-plane), per the AskUserQuestion
-  answer's default. Confirm, or collapse into one.
+## Open questions — RESOLVED (owner, 2026-07-09; D-2026-07-09-1)
+
+- **Q1 — contract wins.** Stale/mismatched partial degrades gracefully to a
+  full-file transfer, never an abort. Owner's principle (quoted in the header)
+  is the rationale; D1 stands as written.
+- **Q2 — keep in-place patch, surface at end of op.** No temp+rename atomicity
+  for otp-7 (parity with the code being replaced). The owner's rider: the fault
+  is surfaced in the end-of-operation summary with a re-run suggestion — see D4.
+  No atomicity follow-up filed; convergence-on-retry is the reliability model.
+- **Q3 — 7a then 7b, no collapse.** Owner: "confirmed. no collapse. keep the
+  reviewloop codex playbook going slice by slice."
 
 ## Verification (when Active)
 
 - `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
   `cargo test --workspace` (count must not drop).
 - Windows parity after touching `win_fs`/sink paths: `scripts/windows/run-blit-tests.ps1`.
 - Each sub-slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`), guard proof
   per pin above.

Changes:

TODO.md
  @@ -192,165 +192,172 @@ explicitly-deferred logging epic (F15).
  -- [ ] **CLI transfer output redesign** (owner, 2026-07-06): current
  -      `blit copy`/`mirror` output "doesn't convey any useful information
  -      at all" — owner wants something closer to `rclone`/`cargo`: a
  -      persistent stat block at a static screen location, plus a scrolling
  -      list of in-flight/recent filenames, instead of what exists today.
  +- [ ] **CLI transfer output redesign** (owner, 2026-07-06; re-confirmed
  +      2026-07-09): current `blit copy`/`mirror` output "doesn't convey any
  +      useful information at all" — owner wants something closer to
  +      `rclone`/`cargo`: "a coherent info block with stats and a scrolling
  +      list of files in a frame below, so probably a TUI?" (owner wording,
  +      2026-07-09) — i.e. a persistent stat block at a static screen
  +      location, plus a scrolling list of in-flight/recent filenames,
  +      instead of what exists today. 2026-07-09 context: the owner hit this
  +      while settling otp-7's error-surfacing question — "the current
  +      progress display is absolutely useless for this". The narrow
  +      end-of-operation fault summary (name failed files, suggest re-run)
  +      ships with otp-7 (D-2026-07-09-1) and is NOT gated on this redesign.
         Confirmed by reading the actual code — there is no persistent/redraw
         rendering anywhere in the transfer output path, only plain
         scrolling `println!`/`eprintln!` lines: (1) the local/streaming-manifest
         path's spinner + `"Enumerated N entries… (streaming manifest)"`
         heartbeat (`crates/blit-core/src/remote/push/client/helpers.rs:176`,
         the same call site audit-16 already flagged for its own separate
         `--verbose`-gating bug); (2) the remote-transfer progress path's
         once-a-second `"[progress] N/M files • X MiB copied • Y MiB/s avg •
         Z MiB/s current"` line (`crates/blit-cli/src/transfers/remote.rs:33-140`,
         `spawn_progress_monitor_with_options`), which just reprints a new
         line every tick rather than redrawing in place. Neither path shows
         a file list, a static stat panel, or does any cursor
         repositioning — every line is transient and scrolls off, which
         matches the owner's complaint exactly. This is a real UX/design
         project, not a bug fix: likely needs a terminal-rendering approach
         (raw ANSI cursor save/restore, or a crate like `indicatif`), has to
         cover both the local and remote transfer paths above, has to decide
         a fallback for non-TTY/`--json`/piped output (today's plain-line
         output is presumably what scripts already parse — a redesign must
         not break `--json` consumers), and touches `blit-cli`+`blit-app`.
         Not designed here — needs its own `plan` before any code, per this
         repo's governance (code changes require an approved plan) and the
         Review policy (D-2026-07-04-1, all code through the codex loop).
         Distinct from `docs/plan/TUI_REWORK.md` (Active), which is about
         the separate interactive `blit-tui` navigation app, not this
         inline CLI progress output during a transfer.
   
   ### Deferred design calls
   
   These are intentionally not next-actionable. Don't pick them up
   without the listed prerequisite — they're tracked here so they
   don't get lost, not so the next agent reimplements them on a hunch.
   
   - [x] **Remote→remote re-evaluation** — resolved by
     `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Phase 1 (`15991ed`)
     added the `DelegatedPull` wire/gate/daemon path; Phase 2 makes
     remote→remote CLI transfers direct by default, keeps
     `--relay-via-cli` as the explicit escape hatch, and pins byte-path
     isolation plus stale-dst/gate/source-refusal no-fallback behavior
     with CLI-side counters.
   
   ## Phase 0: Workspace & Core Logic Foundation
   
   - [x] Initialize Cargo workspace with `blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`.
   - [x] Port `checksum.rs` to `blit-core`.
   - [x] Port `fs_enum.rs` and `enumeration.rs` to `blit-core`.
   - [x] Port `mirror_planner.rs` to `blit-core`.
   - [x] Port `buffer.rs` to `blit-core`.
   - [x] Extract zero-copy primitives into `blit-core/src/zero_copy.rs`.
   - [x] Port unit tests for ported modules.
   
   ## Phase 1: gRPC API & Service Scaffolding
   
   - [x] Create `proto/blit.proto` with the full API definition.
   - [x] Create `build.rs` in `blit-core` to compile the protocol.
   - [x] Add `tonic-build` dependencies to `blit-core/Cargo.toml`.
   - [x] Create `generated` module structure in `blit-core`.
   - [x] Implement skeleton `blitd` server binary in `blit-daemon`.
   - [x] Implement skeleton `blit` CLI binary in `blit-cli`.
   - [x] Add a minimal integration test to verify client-server connection.
   
   ## Phase 2: Orchestrator & Local Operations
   
   - [x] Create `orchestrator.rs` in `blit-core`.
   - [x] Implement the `TransferOrchestrator` struct and `new` method.
   - [x] Implement `execute_local_mirror` method on the orchestrator.
   - [x] Port consolidated path modules (`copy`, `tar_stream`, `transfer_*`, `local_worker`, `logger`, `delete`, `win_fs`) from v1 into `blit-core`.
   - [x] Wire the `blit-cli` `mirror` and `copy` commands to the orchestrator.
   - [x] Refactor `TransferFacade` and planner into streaming producer with heartbeat flushes.
   - [x] Implement 10 s stall detection and progress messaging in orchestrator.
   - [x] Implement fast-path routing for tiny/huge manifests in orchestrator.
   - [x] Add adaptive predictor + local performance history store with `blit diagnostics perf`.
   - [x] Remove `--ludicrous-speed` behaviour (make no-op) and add CLI progress UI.
   - [x] Update unit/integration tests to cover fast-path routing and predictor logic.
   
   ## Phase 2.5: Performance & Validation Checkpoint
   
   - [x] Create benchmark script for local mirror performance.
   - [x] Run and compare against v1. (2025-10-16: v2 ~1.93× slower; optimization needed before GO)
   - [x] Analyse Windows ETW traces (wingpt-4/5.md findings logged) (`logs/blit_windows_bench.zip`) and propose copy-path optimisations.
  +12 -5

docs/DECISIONS.md
  @@ -68,80 +68,85 @@ Format:
  +
  +## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
  +- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
  +- Why: owner answered Q1–Q3 in session 2026-07-09; the flip is the approval the plan procedure requires. In the same exchange the owner re-confirmed the broader progress-display redesign (persistent stats block + scrolling file frame, "probably a TUI") — that stays a queued TODO.md item ("CLI transfer output redesign"), NOT otp-7 scope, and needs its own plan.
  +- Supersedes: nothing (the plan doc's Open-questions section is rewritten as resolved in the same commit).
  +5 -0

docs/STATE.md
  @@ -1,195 +1,193 @@
  -Last updated: 2026-07-06
  +Last updated: 2026-07-09
   
   - 2026-07-04: Owner-approved dual push reached 3d8326b (origin: 10d89e0..3d8326b; gitea mirror: 2a77b9f..3d8326b). That push corrected a prior remote-name confusion; windows-latest CI on that push is the "meaningfully green" check referenced in prior notes.
   
  -- Current session (2026-07-06): otp-6 CLOSED; otp-7 in DESIGN — slice design drafted at docs/plan/OTP7_RESUME.md (Draft). NO CODE until the owner answers Q1–Q3 and flips the plan Active. otp-6 (a/b) mirror + filters landed and graded. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
  +- Current session (2026-07-09): owner answered otp-7's Q1–Q3 (D-2026-07-09-1) — docs/plan/OTP7_RESUME.md is **Active**; otp-7a (resume over the in-stream carrier) is the current slice, through the codex loop. ONE_TRANSFER_PATH otp-1..6 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).
   
   - Session work: filed audit-17 and audit-18; noted a CLI-output-redesign item in TODO.md; drafted+reviewed docs/plan/LOCAL_ERROR_TELEMETRY.md (Draft). A session-wide codex pass fixed 5 cross-doc staleness bugs.
   
   - Notes on push state: owner previously pushed master → GitHub at 10d89e0; local commits f6e592e..HEAD remain unpushed and windows-latest CI will ride the next push.
   
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
     open.** Progress (each through the codex loop; closed-slice detail in
     DEVLOG + `.review/` + REVIEW.md):
     - **otp-1 / otp-3 / otp-4a `[x]`** — wire+session contract
       (`docs/TRANSFER_SESSION.md`); role-parameterized drivers over the
       in-process transport (invariance property in the role suite); daemon
       serves `Transfer` as Responder, client push over gRPC; A/B
       byte-identical vs old push; SizeMtime = data-safe skip (open Q below).
     - **otp-4b (1/2/3) `[x]` — push data plane fully on the session, closed**:
       single-stream TCP data plane, mid-transfer resize/multi-stream + sf-2
       shape correction, deterministic mid-transfer cancel. Detail: DEVLOG.
     - **otp-5a `[x]`** (`84be1cc`, codex PASS) — the one served `Transfer`
       RPC serves BOTH roles via `run_responder` (SOURCE-init→daemon
       DESTINATION = push; DEST-init→daemon SOURCE = pull, in-stream).
     - **otp-5b (1/2) `[x]`** — the SOURCE-responder data plane, closed:
       5b-1 (`e6a0b3b`+`13485ee`) decoupled connection role (RESPONDER
       binds+accepts, INITIATOR dials) from byte role; 5b-2 (`d579365`+
       `773a877`) lifted the single-stream cap — the pull data plane resizes
       via sf-2 (same resize frames as push). Defaults to TCP; A/B
       byte-identical vs old `pull_sync`. Suite → **1522**.
     - **otp-6 (a/b) `[x]`** — mirror + filters on the session, closed.
       6a (`c026692`+`0bb27f5`) honors `SessionOpen.filter` via the universal
       `FilteredSource` chokepoint. 6b (`01d9c41`+`3c99557`) is the one delete
       rule: DESTINATION diffs the complete source manifest at SourceDone,
       scan-complete-guarded + filter-scoped. Codex High: keep-set now folds
       case on macOS too (case-insensitive-FS data-loss). Suite → **1529**.
  -  - Current: **otp-7 IN DESIGN** — Draft `docs/plan/OTP7_RESUME.md`
  -    (`9fb5e4a`) awaiting owner review (see Open questions); no code until
  -    Active. otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
  +  - Current: **otp-7 ACTIVE (D-2026-07-09-1)** — `docs/plan/OTP7_RESUME.md`
  +    flipped Active 2026-07-09 (Q1 contract-wins fallback; Q2 in-place patch
  +    + end-of-op fault summary rider; Q3 7a-then-7b). Implementing **otp-7a**
  +    (resume over the in-stream carrier) through the codex loop.
  +    otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
   - **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
     `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
     until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
     baseline. Principle stands: ceiling-driven, never competitor-relative
     (D-2026-07-04-4; a ≥25% margin answer was retracted — do not
     re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
   - **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
     measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s; owner
     declarations pending in Blocked); 10 GbE session done; w9-3 + review rows
     landed. Codex loop governs all changes (D-2026-07-04-1; DEVLOG 07-04/05).
   
   ## Queue (ordered)
   
   1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
      the only work item until it ships**: slices otp-1..13 through the
      codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
      otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b) `[x]`. Current:
  -   **otp-7 IN DESIGN** (`docs/plan/OTP7_RESUME.md` Draft, owner review;
  -   no code until Active). otp-2 (symmetric baseline) is RIG-GATED —
  +   **otp-7 ACTIVE** (`docs/plan/OTP7_RESUME.md`, D-2026-07-09-1) —
  +   implementing otp-7a. otp-2 (symmetric baseline) is RIG-GATED —
      before otp-10 cutover.
   2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
      Shipped (zero-copy resolved — D-2026-07-05-3). Optional owner-gated
      measurement follow-ups (Win 11 bare-metal; disk-path variants;
      >ARC-size push) — disk-path items largely absorbed by otp-2/otp-12's
      symmetric-rig matrices. Env: bench binaries at
      `skippy:/mnt/generic-pool/video/blit-bin/` (/tmp, /home noexec there).
   3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
      resumes/re-derives after ONE_TRANSFER_PATH ships.
   4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
      open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
      (mirror-executor consolidation) likely lands for free inside
      otp-6's one-delete-rule slice; re-check before picking it up.
   5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
      Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
      cutover as a runtime-selected write strategy in the unified receive
      sink (design: eval doc §If-FAST-evidence; dead module deletes in
      w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
      2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
      rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
  ... (8 lines truncated)
  +11 -13

docs/plan/OTP7_RESUME.md
  @@ -1,174 +1,183 @@
  -**Status**: Draft
  +**Status**: Active (owner Q1–Q3 answered + "confirmed", 2026-07-09; D-2026-07-09-1)
   **Created**: 2026-07-07
   **Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-7.
   **Contract**: `docs/TRANSFER_SESSION.md` (resume exception + frame table, pinned otp-1).
  -**Governs**: no code until the owner flips this to `**Status**: Active`
  -(AGENTS.md; `.agents/repo-guidance.md` plan operator). Per D-2026-07-04-1 this
  -plan change also goes through the codex loop.
  +**Governs**: implementation proceeds 7a → 7b, one slice per codex loop pass
  +(D-2026-07-04-1). Owner's deciding principle, quoted: "FAST, SIMPLE, RELIABLE
  +file transfer. if we abort the whole thing when we could have fixed or
  +surfaced a single error, we are violating all of those."
   
   ## Why this doc
   
   otp-7 is the plan's **explicit RELIABLE exception**: resumed files use a
   strictly-ordered block-hash exchange, and the choreography is novel (unlike the
   mechanical carrier splits of otp-4b/5b/6). The owner asked for the design on paper
   before the intricate code. This doc records the choreography, the reuse map, the
   design decisions (most already settled by the contract), the staging, and the
   guard-proof targets — so implementation is a transcription, not a discovery.
   
   ## What resume is (contract, already pinned in otp-1)
   
   A `NeedEntry` may be flagged `resume=true`. For such a file the DESTINATION sends
   its `BlockHashList` (Blake3 per block of the existing partial) and the SOURCE
   **must not send any byte of that file until it has received that list**. The SOURCE
   then transfers only the blocks whose hashes differ (or that the dest lacks), as
   `BlockTransfer` records, ending with `BlockTransferComplete{total_bytes}`. Stale or
   mismatched partials fall back to full-file transfer.
   
   Frames (field numbers frozen, `TRANSFER_SESSION.md`): `8 BlockHashList` (DEST),
   `14 BlockTransfer` (SOURCE), `15 BlockTransferComplete` (SOURCE). `SessionOpen.resume`
   carries `ResumeSettings{enabled, block_size}`. `NeedEntry.resume` is field 2.
   
   ## What already exists (reused verbatim — no reinvention)
   
   - **Wire frames + payload enums**: `BlockHashList`/`BlockTransfer`/`BlockTransferComplete`
     frames and `PreparedPayload::{FileBlock, FileBlockComplete}` are defined and
     name-mapped in the session (`transfer_session/mod.rs:250,256,257`).
   - **DEST apply (reassembly)**: `FsTransferSink::write_file_block_payload`
     (`sink.rs:641`, seek+write into the partial in place) and
     `write_file_block_complete` (`sink.rs:687`, `set_len` + fsync + stamp mtime/perms).
     In-place patch of the partial — the partial IS the destination; no temp+rename
     (matches the old pull client).
   - **DEST block hashing**: `compute_block_hashes` (`remote/pull.rs:1139`) — streams
     the partial in `block_size` chunks, `blake3::hash`, returns 32-byte digests; an
     absent file returns an empty vec (the implicit full-file fallback).
   - **Block-diff reference**: `resume_copy_file` (`copy/file_copy/resume.rs:52`) is the
     canonical block-compare (write a block iff beyond dst len, a partial tail, or
     hashes differ; truncate if dst longer). The SOURCE-side diff is the same logic.
   - **Defaults**: `DEFAULT_BLOCK_SIZE` = 1 MiB, `MAX_BLOCK_SIZE` = 64 MiB
     (`copy/file_copy/resume.rs:16,19`). `ResumeSettings.block_size == 0` ⇒ default.
   
   ## What is new (the otp-7 work)
   
   1. **Un-stub the four refusal sites**: both open validators (`mod.rs:362,401`), the
      source recv-half resume-need rejection (`mod.rs:799`), and the outbound-planner
      FileBlock bail (`mod.rs:1446`).
   2. **The strict-ordering exchange choreography** in the session's source/dest halves.
   3. **A home for the SOURCE-side block-diff** — today hand-rolled in `pull_sync.rs`,
      not on any trait (see Design decision D3).
   
   ## Choreography (strict ordering)
   
   ```
   DESTINATION (diff loop)                     SOURCE (send half)
   ─────────────────────────                   ──────────────────
   for each manifest entry:
     if resume-eligible (see D2):
        NeedEntry{path, resume=true} ───────►  recv: ResumeNeed(header)
        BlockHashList{path, bsz, hashes} ───►  recv: BlockHashes(path, hashes)
                                               (send half correlates the two;
                                                a resume need is HELD until its
                                                BlockHashList arrives — the
                                                RELIABLE ordering guarantee)
     else:
        NeedEntry{path, resume=false} ──────►  recv: Need(header)  (unchanged)
   
                                               for a held resume need + its hashes:
                                                 read source file block-by-block,
                                                 blake3 each; for block i where
                                                 i >= hashes.len() OR hash != hashes[i]:
     recv BlockTransfer{path,off,bytes} ◄────    send BlockTransfer{path, off, block}
        sink.write_file_block_payload            (in-stream carrier: control-lane
                                                  frames; data-plane: send_block, 7b)
     recv BlockTransferComplete{path,total} ◄─  send BlockTransferComplete{path,total}
        sink.write_file_block_complete
        files_resumed += 1
   ```
   
   The source's per-file byte phase for a resume need is "send changed blocks then
   complete", replacing the whole-file record it sends for a non-resume need. Ordering
   is enforced on the SOURCE: it will not emit a block for a path before it holds that
   path's `BlockHashList` (fail-fast if a block phase would start without one).
   
   ## Design decisions
   
   - **D1 — stale/mismatched partial ⇒ graceful full-file fallback**, per the contract
     (`TRANSFER_SESSION.md:84`), NOT the hard `Status::internal` the old *data-plane*
  ... (28 lines truncated)
  +23 -14
[full diff: rtk git diff --no-compact]

exec
/bin/zsh -lc "rtk git grep -n -i -E '(draft|await(ing|s)?|open question|in design|no code|flip).{0,160}(OTP7_RESUME|otp.?7)|(OTP7_RESUME|otp.?7).{0,160}(draft|await(ing|s)?|open question|in design|no code|flip)' 70c9688 -- docs" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7z3C3CvR' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GALDyQNu' (errno=Operation not permitted)
70c9688:docs/DECISIONS.md:149:## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
70c9688:docs/DECISIONS.md:150:- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
70c9688:docs/STATE.md:185:  session**: otp-7 — owner's Q1–Q3 on `docs/plan/OTP7_RESUME.md`, flip
70c9688:docs/plan/LOCAL_ERROR_TELEMETRY.md:17:does not enter the Queue and no code lands until the current otp-7 slice

exec
/bin/zsh -lc 'rtk scripts/agent/check-docs.sh' in /Users/michael/Dev/blit_v2
 succeeded in 367ms:
check-docs: OK

exec
/bin/zsh -lc "rg -n -C 5 'SessionFault|TransferSummary|SessionError' crates proto/blit.proto | head -n 280" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
proto/blit.proto-1230-  TRANSFER_ROLE_DESTINATION = 2;
proto/blit.proto-1231-}
proto/blit.proto-1232-
proto/blit.proto-1233-// First frame BOTH directions. Exact-match same-build handshake
proto/blit.proto-1234-// (D-2026-07-05-2): any inequality in either field is a
proto/blit.proto:1235:// SessionError{BUILD_MISMATCH} naming both ids, then close. No
proto/blit.proto-1236-// negotiate-down, no advisory fields, no capability bits.
proto/blit.proto-1237-message SessionHello {
proto/blit.proto-1238-  // "<crate version>+<git commit>[.dirty]", composed at compile time.
proto/blit.proto-1239-  string build_id = 1;
proto/blit.proto-1240-  // Bumped on any wire-shape change; exact match required.
--
proto/blit.proto-1266-  // receiver advertises capacity — D-2026-06-20-1/-2; absent/0 =
proto/blit.proto-1267-  // unknown hardware value, conservative, never "old peer").
proto/blit.proto-1268-  CapacityProfile receiver_capacity = 12;
proto/blit.proto-1269-}
proto/blit.proto-1270-
proto/blit.proto:1271:// Responder's reply. Refusals are SessionError frames, never silent
proto/blit.proto-1272-// closes.
proto/blit.proto-1273-message SessionAccept {
proto/blit.proto-1274-  // Set iff the responder is DESTINATION.
proto/blit.proto-1275-  CapacityProfile receiver_capacity = 1;
proto/blit.proto-1276-  // Absent = in-stream carrier (requested, or listener bind failed).
--
proto/blit.proto-1317-message SourceDone {}
proto/blit.proto-1318-
proto/blit.proto-1319-// DESTINATION → SOURCE at close: the end that wrote bytes and
proto/blit.proto-1320-// executed deletes attests to the outcome (one summary shape for
proto/blit.proto-1321-// every direction; replaces PushSummary/PullSummary at cutover).
proto/blit.proto:1322:message TransferSummary {
proto/blit.proto-1323-  uint64 files_transferred = 1;
proto/blit.proto-1324-  uint64 bytes_transferred = 2;
proto/blit.proto-1325-  uint64 entries_deleted = 3;   // mirror executed destination-local
proto/blit.proto-1326-  bool in_stream_carrier_used = 4;
proto/blit.proto-1327-  uint64 files_resumed = 5;
proto/blit.proto-1328-}
proto/blit.proto-1329-
proto/blit.proto-1330-// Structured refusal/abort — an end says why before closing.
proto/blit.proto:1331:message SessionError {
proto/blit.proto-1332-  enum Code {
proto/blit.proto-1333-    SESSION_ERROR_UNSPECIFIED = 0;
proto/blit.proto-1334-    BUILD_MISMATCH = 1;
proto/blit.proto-1335-    MODULE_UNKNOWN = 2;
proto/blit.proto-1336-    READ_ONLY = 3;
--
proto/blit.proto-1371-    BlockTransfer block = 14;
proto/blit.proto-1372-    BlockTransferComplete block_complete = 15;
proto/blit.proto-1373-    DataPlaneResize resize = 16;
proto/blit.proto-1374-    DataPlaneResizeAck resize_ack = 17;
proto/blit.proto-1375-    SourceDone source_done = 18;
proto/blit.proto:1376:    TransferSummary summary = 19;
proto/blit.proto:1377:    SessionError error = 20;
proto/blit.proto-1378-  }
proto/blit.proto-1379-}
--
crates/blit-daemon/src/service/transfer.rs-32-
crates/blit-daemon/src/service/transfer.rs-33-use blit_core::generated::session_error::Code;
crates/blit-daemon/src/service/transfer.rs-34-use blit_core::generated::{SessionOpen, TransferFrame};
crates/blit-daemon/src/service/transfer.rs-35-use blit_core::transfer_session::transport::grpc_daemon_transport;
crates/blit-daemon/src/service/transfer.rs-36-use blit_core::transfer_session::{
crates/blit-daemon/src/service/transfer.rs:37:    run_responder, DestinationTarget, HelloConfig, OpenResolver, ResolvedEndpoint, SessionFault,
crates/blit-daemon/src/service/transfer.rs-38-    SourceResponderTarget,
crates/blit-daemon/src/service/transfer.rs-39-};
crates/blit-daemon/src/service/transfer.rs-40-
crates/blit-daemon/src/service/transfer.rs-41-use super::util::{resolve_contained_path, resolve_module, resolve_relative_path};
crates/blit-daemon/src/service/transfer.rs-42-use crate::runtime::{ModuleConfig, RootExport};
crates/blit-daemon/src/service/transfer.rs-43-
crates/blit-daemon/src/service/transfer.rs:44:/// Map a resolver `tonic::Status` onto a `SessionError` code. blit-core
crates/blit-daemon/src/service/transfer.rs-45-/// is deliberately `Status`-free, so the daemon picks the wire code:
crates/blit-daemon/src/service/transfer.rs-46-/// an unknown module is `MODULE_UNKNOWN`, a bad or escaping path is a
crates/blit-daemon/src/service/transfer.rs-47-/// `PROTOCOL_VIOLATION` (the initiator sent an unusable request),
crates/blit-daemon/src/service/transfer.rs-48-/// anything else is `INTERNAL`.
crates/blit-daemon/src/service/transfer.rs:49:fn status_to_fault(status: Status) -> SessionFault {
crates/blit-daemon/src/service/transfer.rs-50-    let code = match status.code() {
crates/blit-daemon/src/service/transfer.rs-51-        tonic::Code::NotFound => Code::ModuleUnknown,
crates/blit-daemon/src/service/transfer.rs-52-        tonic::Code::InvalidArgument | tonic::Code::PermissionDenied => Code::ProtocolViolation,
crates/blit-daemon/src/service/transfer.rs-53-        _ => Code::Internal,
crates/blit-daemon/src/service/transfer.rs-54-    };
crates/blit-daemon/src/service/transfer.rs:55:    SessionFault::refusal(code, status.message().to_string())
crates/blit-daemon/src/service/transfer.rs-56-}
crates/blit-daemon/src/service/transfer.rs-57-
crates/blit-daemon/src/service/transfer.rs-58-/// Build the daemon's [`OpenResolver`]: given a received `SessionOpen`,
crates/blit-daemon/src/service/transfer.rs-59-/// resolve its module + path to an absolute local root and report the
crates/blit-daemon/src/service/transfer.rs-60-/// module's read-only flag. Mirrors the push Header sequence
crates/blit-daemon/src/service/transfer.rs-61-/// (`resolve_module` → path validation → F2 canonical containment via
crates/blit-daemon/src/service/transfer.rs:62:/// `resolve_contained_path`), refusing with a `SessionError` instead of
crates/blit-daemon/src/service/transfer.rs-63-/// a `tonic::Status`. The closure is `Fn` (callable once per session)
crates/blit-daemon/src/service/transfer.rs-64-/// and clones its captured handles per call so it stays `Send + Sync`.
crates/blit-daemon/src/service/transfer.rs-65-pub(crate) fn make_open_resolver(
crates/blit-daemon/src/service/transfer.rs-66-    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
crates/blit-daemon/src/service/transfer.rs-67-    default_root: Option<RootExport>,
--
crates/blit-daemon/src/service/transfer.rs-97-/// the client's declared initiator role via [`run_responder`]: a SOURCE
crates/blit-daemon/src/service/transfer.rs-98-/// initiator makes the daemon the DESTINATION (push-equivalent, otp-4);
crates/blit-daemon/src/service/transfer.rs-99-/// a DESTINATION initiator makes the daemon the SOURCE (pull-equivalent,
crates/blit-daemon/src/service/transfer.rs-100-/// otp-5). Returns `Ok(())` on a clean transfer or `Err(Status)`
crates/blit-daemon/src/service/transfer.rs-101-/// carrying the session fault's message for the jobs record. The session
crates/blit-daemon/src/service/transfer.rs:102:/// communicates its own refusals to the peer as `SessionError` *frames*
crates/blit-daemon/src/service/transfer.rs-103-/// (via the response stream); this `Status` is for the daemon's outcome
crates/blit-daemon/src/service/transfer.rs-104-/// record and `resolve_streaming_outcome`'s terminal handling, not the
crates/blit-daemon/src/service/transfer.rs-105-/// primary error channel.
crates/blit-daemon/src/service/transfer.rs-106-pub(crate) async fn run_transfer_session(
crates/blit-daemon/src/service/transfer.rs-107-    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
--
crates/blit-daemon/src/service/transfer.rs-128-        // daemon record does not distinguish push- from pull-equivalent
crates/blit-daemon/src/service/transfer.rs-129-        // (the jobs kind stays Push until the taxonomy is revisited at
crates/blit-daemon/src/service/transfer.rs-130-        // cutover — see the dispatcher).
crates/blit-daemon/src/service/transfer.rs-131-        Ok(_) => Ok(()),
crates/blit-daemon/src/service/transfer.rs-132-        Err(report) => {
crates/blit-daemon/src/service/transfer.rs:133:            // run_responder already emitted a SessionError frame to the
crates/blit-daemon/src/service/transfer.rs-134-            // peer; surface the reason for the record.
crates/blit-daemon/src/service/transfer.rs-135-            let msg = report
crates/blit-daemon/src/service/transfer.rs:136:                .downcast_ref::<SessionFault>()
crates/blit-daemon/src/service/transfer.rs-137-                .map(|f| f.message.clone())
crates/blit-daemon/src/service/transfer.rs-138-                .unwrap_or_else(|| format!("{report:#}"));
crates/blit-daemon/src/service/transfer.rs-139-            Err(Status::internal(msg))
crates/blit-daemon/src/service/transfer.rs-140-        }
crates/blit-daemon/src/service/transfer.rs-141-    }
--
crates/blit-daemon/src/service/transfer_session_e2e.rs-10-//!   over the data plane and over the in-stream fallback;
crates/blit-daemon/src/service/transfer_session_e2e.rs-11-//! - **A/B parity**: the same fixture through OLD push and the NEW
crates/blit-daemon/src/service/transfer_session_e2e.rs-12-//!   session (data plane) yields byte-identical destination trees +
crates/blit-daemon/src/service/transfer_session_e2e.rs-13-//!   equal shared summary counters (the converge-up bar);
crates/blit-daemon/src/service/transfer_session_e2e.rs-14-//! - responder refusals (read-only module, unknown module) arrive as
crates/blit-daemon/src/service/transfer_session_e2e.rs:15://!   `SessionError` frames, surfaced to the client as faults;
crates/blit-daemon/src/service/transfer_session_e2e.rs-16-//! - the unified SizeMtime semantic: a same-size destination file that
crates/blit-daemon/src/service/transfer_session_e2e.rs-17-//!   is NEWER than the source is SKIPPED (the data-safe, pull-style
crates/blit-daemon/src/service/transfer_session_e2e.rs-18-//!   converged behavior — see the finding doc's compare decision).
crates/blit-daemon/src/service/transfer_session_e2e.rs-19-//!
crates/blit-daemon/src/service/transfer_session_e2e.rs-20-//! otp-5a/5b add the pull-equivalent (roles flipped): the client initiates
--
crates/blit-daemon/src/service/transfer_session_e2e.rs-43-use blit_core::remote::transfer::session_client::{
crates/blit-daemon/src/service/transfer_session_e2e.rs-44-    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
crates/blit-daemon/src/service/transfer_session_e2e.rs-45-};
crates/blit-daemon/src/service/transfer_session_e2e.rs-46-use blit_core::remote::transfer::source::FsTransferSource;
crates/blit-daemon/src/service/transfer_session_e2e.rs-47-use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePullClient, RemotePushClient};
crates/blit-daemon/src/service/transfer_session_e2e.rs:48:use blit_core::transfer_session::SessionFault;
crates/blit-daemon/src/service/transfer_session_e2e.rs-49-use tokio::sync::oneshot;
crates/blit-daemon/src/service/transfer_session_e2e.rs-50-
crates/blit-daemon/src/service/transfer_session_e2e.rs-51-use crate::runtime::ModuleConfig;
crates/blit-daemon/src/service/transfer_session_e2e.rs-52-use crate::service::BlitService;
crates/blit-daemon/src/service/transfer_session_e2e.rs-53-
--
crates/blit-daemon/src/service/transfer_session_e2e.rs-201-        ("dir one/b.log", b"beta beta beta", 1_600_000_003),
crates/blit-daemon/src/service/transfer_session_e2e.rs-202-        ("dir one/deeper/c.dat", b"gamma-content", 1_600_000_004),
crates/blit-daemon/src/service/transfer_session_e2e.rs-203-    ]
crates/blit-daemon/src/service/transfer_session_e2e.rs-204-}
crates/blit-daemon/src/service/transfer_session_e2e.rs-205-
crates/blit-daemon/src/service/transfer_session_e2e.rs:206:fn fault_of(err: &eyre::Report) -> &SessionFault {
crates/blit-daemon/src/service/transfer_session_e2e.rs:207:    err.downcast_ref::<SessionFault>()
crates/blit-daemon/src/service/transfer_session_e2e.rs:208:        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
crates/blit-daemon/src/service/transfer_session_e2e.rs-209-}
crates/blit-daemon/src/service/transfer_session_e2e.rs-210-
crates/blit-daemon/src/service/transfer_session_e2e.rs-211-// --- otp-4b-3: deterministic mid-transfer cancel over the data plane ---
crates/blit-daemon/src/service/transfer_session_e2e.rs-212-
crates/blit-daemon/src/service/transfer_session_e2e.rs-213-/// A `TransferSource` that puts a transfer into a provably-stuck
--
crates/blit-daemon/src/service/transfer_session_e2e.rs-285-}
crates/blit-daemon/src/service/transfer_session_e2e.rs-286-
crates/blit-daemon/src/service/transfer_session_e2e.rs-287-/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
crates/blit-daemon/src/service/transfer_session_e2e.rs-288-/// exactly what the RPC handler fires) while a payload is stuck mid-flight
crates/blit-daemon/src/service/transfer_session_e2e.rs-289-/// over the TCP data plane. The client must surface
crates/blit-daemon/src/service/transfer_session_e2e.rs:290:/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
crates/blit-daemon/src/service/transfer_session_e2e.rs-291-/// than the data-plane transport break it also causes, and it must not
crates/blit-daemon/src/service/transfer_session_e2e.rs-292-/// hang. The daemon must then tear the job down cleanly (the active row
crates/blit-daemon/src/service/transfer_session_e2e.rs-293-/// drains).
crates/blit-daemon/src/service/transfer_session_e2e.rs-294-#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
crates/blit-daemon/src/service/transfer_session_e2e.rs-295-async fn mid_transfer_cancel_surfaces_cancelled_over_the_data_plane() {
--
crates/blit-daemon/src/service/core.rs-354-    /// by running `run_destination` as the Responder — the byte
crates/blit-daemon/src/service/core.rs-355-    /// RECEIVER of a client-initiated SOURCE push. Mirrors `push`:
crates/blit-daemon/src/service/core.rs-356-    /// register a jobs row, race the session against cancel/hangup, and
crates/blit-daemon/src/service/core.rs-357-    /// return the response stream immediately (the session runs in the
crates/blit-daemon/src/service/core.rs-358-    /// spawned task, feeding the `ReceiverStream`). Session refusals
crates/blit-daemon/src/service/core.rs:359:    /// travel to the peer as `SessionError` frames; the daemon-specific
crates/blit-daemon/src/service/core.rs-360-    /// module resolution + transport assembly live in `super::transfer`.
crates/blit-daemon/src/service/core.rs-361-    /// Contract: docs/TRANSFER_SESSION.md.
crates/blit-daemon/src/service/core.rs-362-    async fn transfer(
crates/blit-daemon/src/service/core.rs-363-        &self,
crates/blit-daemon/src/service/core.rs-364-        request: Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
--
crates/blit-daemon/src/service/core.rs-390-        tokio::spawn(async move {
crates/blit-daemon/src/service/core.rs-391-            let guard = guard;
crates/blit-daemon/src/service/core.rs-392-            let job = job;
crates/blit-daemon/src/service/core.rs-393-            let cancel_token = job.cancellation_token().clone();
crates/blit-daemon/src/service/core.rs-394-            // Session variant: cancel surfaces as a framed
crates/blit-daemon/src/service/core.rs:395:            // SessionError{CANCELLED}, not a bare Status (codex F1).
crates/blit-daemon/src/service/core.rs-396-            let (ok, err_msg) = resolve_transfer_session_outcome(
crates/blit-daemon/src/service/core.rs-397-                super::transfer::run_transfer_session(modules, default_root, inbound, tx.clone()),
crates/blit-daemon/src/service/core.rs-398-                &tx,
crates/blit-daemon/src/service/core.rs-399-                &cancel_token,
crates/blit-daemon/src/service/core.rs-400-                &metrics,
--
crates/blit-daemon/src/service/core.rs-1414-    }
crates/blit-daemon/src/service/core.rs-1415-}
crates/blit-daemon/src/service/core.rs-1416-
crates/blit-daemon/src/service/core.rs-1417-/// Session variant of [`resolve_streaming_outcome`] for the `Transfer`
crates/blit-daemon/src/service/core.rs-1418-/// RPC: identical hangup / completion / fault handling, but on
crates/blit-daemon/src/service/core.rs:1419:/// `CancelJob` it emits a framed `SessionError{CANCELLED}` on the
crates/blit-daemon/src/service/core.rs-1420-/// response stream instead of a bare `Status::cancelled` (otp-4a codex
crates/blit-daemon/src/service/core.rs-1421-/// F1). The session speaks `TransferFrame`s, so the client reads the
crates/blit-daemon/src/service/core.rs-1422-/// framed error — and the aborted session future can't send it itself
crates/blit-daemon/src/service/core.rs-1423-/// once the select drops it, so the dispatcher does. A session that
crates/blit-daemon/src/service/core.rs-1424-/// faults on its own already framed the reason; the trailing `Status`
--
crates/blit-daemon/src/service/core.rs-1514-        .await;
crates/blit-daemon/src/service/core.rs-1515-        assert_eq!(outcome, None, "a running handler must yield to cancel");
crates/blit-daemon/src/service/core.rs-1516-    }
crates/blit-daemon/src/service/core.rs-1517-
crates/blit-daemon/src/service/core.rs-1518-    /// otp-4a codex F1: a `CancelJob` on a served `Transfer` session
crates/blit-daemon/src/service/core.rs:1519:    /// must reach the client as a framed `SessionError{CANCELLED}` on
crates/blit-daemon/src/service/core.rs-1520-    /// the response stream — not a bare `Status::cancelled` (the
crates/blit-daemon/src/service/core.rs-1521-    /// session speaks frames, and the aborted session future can't
crates/blit-daemon/src/service/core.rs-1522-    /// send it itself). Guard: with the cancel branch reverted to
crates/blit-daemon/src/service/core.rs-1523-    /// `Err(Status::cancelled)` this fails (no `Ok` error frame lands).
crates/blit-daemon/src/service/core.rs-1524-    #[tokio::test]
--
crates/blit-daemon/src/service/core.rs-1538-
crates/blit-daemon/src/service/core.rs-1539-        let frame = rx
crates/blit-daemon/src/service/core.rs-1540-            .recv()
crates/blit-daemon/src/service/core.rs-1541-            .await
crates/blit-daemon/src/service/core.rs-1542-            .expect("a terminal frame")
crates/blit-daemon/src/service/core.rs:1543:            .expect("a framed SessionError, not a gRPC Status");
crates/blit-daemon/src/service/core.rs-1544-        match frame.frame {
crates/blit-daemon/src/service/core.rs-1545-            Some(WireFrame::Error(err)) => assert_eq!(
crates/blit-daemon/src/service/core.rs-1546-                err.code,
crates/blit-daemon/src/service/core.rs-1547-                Code::Cancelled as i32,
crates/blit-daemon/src/service/core.rs:1548:                "cancel must emit a framed CANCELLED SessionError"
crates/blit-daemon/src/service/core.rs-1549-            ),
crates/blit-daemon/src/service/core.rs-1550-            other => panic!("expected a CANCELLED error frame, got {other:?}"),
crates/blit-daemon/src/service/core.rs-1551-        }
crates/blit-daemon/src/service/core.rs-1552-    }
crates/blit-daemon/src/service/core.rs-1553-
--
crates/blit-core/src/transfer_session/mod.rs-30-
crates/blit-core/src/transfer_session/mod.rs-31-use crate::generated::transfer_frame::Frame;
crates/blit-core/src/transfer_session/mod.rs-32-use crate::generated::{
crates/blit-core/src/transfer_session/mod.rs-33-    session_error, ComparisonMode, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp,
crates/blit-core/src/transfer_session/mod.rs-34-    FileData, FileHeader, FilterSpec, ManifestComplete, MirrorMode, NeedBatch, NeedComplete,
crates/blit-core/src/transfer_session/mod.rs:35:    NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
crates/blit-core/src/transfer_session/mod.rs:36:    TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
crates/blit-core/src/transfer_session/mod.rs-37-};
crates/blit-core/src/transfer_session/mod.rs-38-use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
crates/blit-core/src/transfer_session/mod.rs-39-use crate::remote::transfer::diff_planner;
crates/blit-core/src/transfer_session/mod.rs-40-use crate::remote::transfer::payload::PreparedPayload;
crates/blit-core/src/transfer_session/mod.rs-41-use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
--
crates/blit-core/src/transfer_session/mod.rs-143-/// A session-terminating fault: either end refusing, aborting, or
crates/blit-core/src/transfer_session/mod.rs-144-/// catching the peer in a protocol violation. Carried as the error
crates/blit-core/src/transfer_session/mod.rs-145-/// payload of the drivers' `eyre::Report`s — downcast to inspect the
crates/blit-core/src/transfer_session/mod.rs-146-/// wire code.
crates/blit-core/src/transfer_session/mod.rs-147-#[derive(Debug, Clone)]
crates/blit-core/src/transfer_session/mod.rs:148:pub struct SessionFault {
crates/blit-core/src/transfer_session/mod.rs-149-    pub code: session_error::Code,
crates/blit-core/src/transfer_session/mod.rs-150-    pub message: String,
crates/blit-core/src/transfer_session/mod.rs-151-    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
crates/blit-core/src/transfer_session/mod.rs-152-    /// which end is stale (contract §Errors).
crates/blit-core/src/transfer_session/mod.rs-153-    pub local_build_id: String,
crates/blit-core/src/transfer_session/mod.rs-154-    pub peer_build_id: String,
crates/blit-core/src/transfer_session/mod.rs-155-    /// True when the peer already knows about this fault — it sent
crates/blit-core/src/transfer_session/mod.rs:156:    /// the `SessionError` frame itself, or this end already emitted
crates/blit-core/src/transfer_session/mod.rs-157-    /// one. Drivers must not send another.
crates/blit-core/src/transfer_session/mod.rs-158-    pub peer_notified: bool,
crates/blit-core/src/transfer_session/mod.rs-159-}
crates/blit-core/src/transfer_session/mod.rs-160-
crates/blit-core/src/transfer_session/mod.rs:161:impl SessionFault {
crates/blit-core/src/transfer_session/mod.rs-162-    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
crates/blit-core/src/transfer_session/mod.rs-163-        Self {
crates/blit-core/src/transfer_session/mod.rs-164-            code,
crates/blit-core/src/transfer_session/mod.rs-165-            message: message.into(),
crates/blit-core/src/transfer_session/mod.rs-166-            local_build_id: String::new(),
--
crates/blit-core/src/transfer_session/mod.rs-180-    fn read_only(message: impl Into<String>) -> Self {
crates/blit-core/src/transfer_session/mod.rs-181-        Self::new(session_error::Code::ReadOnly, message)
crates/blit-core/src/transfer_session/mod.rs-182-    }
crates/blit-core/src/transfer_session/mod.rs-183-
crates/blit-core/src/transfer_session/mod.rs-184-    /// Public constructor for a caller-side refusal (e.g. the daemon's
crates/blit-core/src/transfer_session/mod.rs:185:    /// [`OpenResolver`] mapping a `tonic::Status` to a `SessionError`
crates/blit-core/src/transfer_session/mod.rs-186-    /// code). blit-core stays free of `tonic::Status`, so the caller
crates/blit-core/src/transfer_session/mod.rs-187-    /// picks the wire code.

exec
/bin/zsh -lc "rg -n -C 8 'message TransferSummary|struct TransferSummary|TransferSummary \\{' proto crates/blit-core/src/remote/transfer_session crates/blit-cli crates/blit-app | head -n 260" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
rg: crates/blit-core/src/remote/transfer_session: No such file or directory (os error 2)
proto/blit.proto-1314-message NeedComplete {}
proto/blit.proto-1315-
proto/blit.proto-1316-// SOURCE's promise that every requested payload byte is flushed.
proto/blit.proto-1317-message SourceDone {}
proto/blit.proto-1318-
proto/blit.proto-1319-// DESTINATION → SOURCE at close: the end that wrote bytes and
proto/blit.proto-1320-// executed deletes attests to the outcome (one summary shape for
proto/blit.proto-1321-// every direction; replaces PushSummary/PullSummary at cutover).
proto/blit.proto:1322:message TransferSummary {
proto/blit.proto-1323-  uint64 files_transferred = 1;
proto/blit.proto-1324-  uint64 bytes_transferred = 2;
proto/blit.proto-1325-  uint64 entries_deleted = 3;   // mirror executed destination-local
proto/blit.proto-1326-  bool in_stream_carrier_used = 4;
proto/blit.proto-1327-  uint64 files_resumed = 5;
proto/blit.proto-1328-}
proto/blit.proto-1329-
proto/blit.proto-1330-// Structured refusal/abort — an end says why before closing.

exec
/bin/zsh -lc "rg -n -i -C 4 'per-file fault|failed_files|failed files|file fault|continue.*fault|fault.*continue|SessionFault' docs crates/blit-core/src/remote/transfer_session proto/blit.proto | head -n 320" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
rg: crates/blit-core/src/remote/transfer_session: No such file or directory (os error 2)
docs/audit/DESIGN_MAP_2026-06-11.md-478-- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:27 — Canonical classifier: is_retryable(eyre::Report) — walks err.chain() looking for a std::io::Error whose kind is transient — Only mechanism in the workspace that decides retry vs fatal. Sees ONLY io::Error in the eyre chain; anything stripped to a String is invisible.
docs/audit/DESIGN_MAP_2026-06-11.md-479-- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:35 — is_retryable_io_kind: the retryable kind list — TimedOut, ConnectionReset, ConnectionAborted, ConnectionRefused, BrokenPipe, UnexpectedEof, NotConnected.
docs/audit/DESIGN_MAP_2026-06-11.md-480-- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:55 — run_with_retries: the only backoff loop — fixed (non-exponential) wait, eprintln notice at lines 69-73, sleep at line 73 — retries==0 means single attempt. Comment at lines 5-10 ties viability to transfer resumability.
docs/audit/DESIGN_MAP_2026-06-11.md-481-- /home/michael/dev/Blit/crates/blit-cli/src/main.rs:55 — --retry/--wait plumbing: run_with_retries wraps Copy (line 55), Mirror (62), Move (69) — Import at line 24. No other command (scan/ls/jobs/admin) and no other crate calls run_with_retries — TUI transfers have no retry loop at all.
docs/audit/DESIGN_MAP_2026-06-11.md:482:- /home/michael/dev/Blit/crates/blit-cli/src/cli.rs:278 — --retry flag (default 0) and --wait flag (line 286, default 5s); --resume flag at line 267 — Help text (268-271) promises 'each retry continues rather than restarts' via resumability. Parse/default tests at 630-648.
docs/audit/DESIGN_MAP_2026-06-11.md-483-- /home/michael/dev/Blit/crates/blit-core/src/errors.rs:90 — DUPLICATE, DEAD classifier: categorize_io_error + ErrorCategory (line 12) + TransferError with should_retry/with_attempt (66-74) — Zero importers anywhere in the workspace (searched blit_core::errors, use crate::errors, ErrorCategory, TransferError, should_retry, with_attempt). Re-exported via lib.rs:9 'pub mod errors'. Contradicts the live classifier — see duplicates.
docs/audit/DESIGN_MAP_2026-06-11.md-484-- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69 — TRANSFER_STALL_TIMEOUT = 30s; StallGuard (line 75) read-side and StallGuardWriter (line 139) write-side idle watchdogs that mint io::ErrorKind::TimedOut — This is the designed producer of classifier-visible retryable errors: an idle stall becomes TimedOut, which is_retryable accepts.
docs/audit/DESIGN_MAP_2026-06-11.md-485-- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1764 — Wiring: CLI pull-receive TCP socket wrapped in StallGuard (audit-1c) — Chain preserved through the pipeline — pipeline.rs:921-953 test asserts the TimedOut survives in the error chain.
docs/audit/DESIGN_MAP_2026-06-11.md-486-- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:841 — Wiring: daemon push-receive socket wrapped in StallGuard (audit-h3a) — Daemon-side TimedOut; surfaced to the pushing CLI only as a tonic Status string via control plane, so the client classifier never sees it directly (client usually sees its own BrokenPipe/Reset instead).
--
docs/audit/DESIGN_MAP_2026-06-11.md-501-- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:348 — Resilience fallback (not retry): jobs watch Subscribe stream error falls back to one final GetState reconciliation (348-362, helper at 370); optional wall-clock deadline at 180-181 — Flags at cli.rs:137-143 (interval_ms default 1000, timeout_secs default 0 = wait forever).
docs/audit/DESIGN_MAP_2026-06-11.md-502-- /home/michael/dev/Blit/crates/blit-cli/src/transfers/local.rs:189 — Resume entry point (local): --resume plumbed into the local engine options — Block-level resume implemented in blit-core/src/copy/file_copy/resume.rs (DEFAULT_BLOCK_SIZE line 16, MAX_BLOCK_SIZE line 19).
docs/audit/DESIGN_MAP_2026-06-11.md-503-- /home/michael/dev/Blit/crates/blit-cli/src/transfers/remote.rs:382 — Resume entry point (pull): resume: args.resume, block_size: 0 (= default 1 MiB) into PullSyncOptions — Carried as ResumeSettings in the spec (pull.rs:609-614); client answers BlockHashRequest / writes BlockTransfer at pull.rs:966-1062; hashes computed at pull.rs:1154.
docs/audit/DESIGN_MAP_2026-06-11.md-504-- /home/michael/dev/Blit/crates/blit-cli/src/transfers/remote_remote_direct.rs:103 — Resume entry point (remote-to-remote): resume flag + block_size 0 forwarded into the delegated spec
docs/audit/DESIGN_MAP_2026-06-11.md:505:- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:229 — Resume decision site (daemon): effective_resume set (229-233) — Modified+same-size files always block-resumed; --resume expands to all Modified; block_size clamped to MAX_BLOCK_SIZE at 84; dispatch at 261/282 — ResumeSettings normalization default-disabled at operation_spec.rs:122 (struct field at :70). NOTE: remote/push has zero 'resume' hits — push has no block resume; the retry loop's 'continues rather than restarts' holds for push only at whole-file granularity (diff skip).
docs/audit/DESIGN_MAP_2026-06-11.md-506-- /home/michael/dev/Blit/proto/blit.proto:564 — Wire surface for resume: ResumeSettings (564), supports_resume capability (576), block messages (282-333)
docs/audit/DESIGN_MAP_2026-06-11.md-507-- /home/michael/dev/Blit/crates/blit-tui/src/main.rs:1457 — TUI site (recorded only): apply_deferred_refan re-attempts F2 watch setup after Failed when a daemon appears (1451-1462) — Event-driven re-attempt, no backoff, no classifier.
docs/audit/DESIGN_MAP_2026-06-11.md-508-- /home/michael/dev/Blit/crates/blit-tui/src/exec_plan.rs:49 — TUI site (recorded only): comment asserting interrupted local copy 'only under-copies, which is safe and retryable' — No actual retry mechanism behind it.
docs/audit/DESIGN_MAP_2026-06-11.md-509-
--
docs/plan/OTP7_RESUME.md-114-  reasoning that made `FilteredSource` the one filter chokepoint in otp-6a). Flag for
docs/plan/OTP7_RESUME.md-115-  codex: confirm the helper doesn't belong on the trait.
docs/plan/OTP7_RESUME.md-116-- **D4 — mid-resume-failure**: block writes patch the partial in place (no
docs/plan/OTP7_RESUME.md-117-  temp+rename, matching the old client). A fault mid-block-transfer surfaces as a
docs/plan/OTP7_RESUME.md:118:  `SessionFault` (peer-notified) and aborts; the partial is left partially patched,
docs/plan/OTP7_RESUME.md-119-  and the NEXT resume re-syncs via a fresh block-hash exchange (the partial's new
docs/plan/OTP7_RESUME.md-120-  hashes reflect whatever landed). The pin asserts the fault surfaces cleanly and no
docs/plan/OTP7_RESUME.md-121-  file is falsely counted `files_resumed`. (No stronger atomicity than the code we
docs/plan/OTP7_RESUME.md-122-  are replacing — called out as a Known gap, not a regression.)
docs/plan/OTP7_RESUME.md-123-  **Owner rider (2026-07-09, Q2)**: the fault must also appear in the CLI's
docs/plan/OTP7_RESUME.md-124-  **end-of-operation summary** — naming the affected file(s) and suggesting a
docs/plan/OTP7_RESUME.md-125-  re-run to converge — not only as a mid-stream line that scrolls away. Small
docs/plan/OTP7_RESUME.md-126-  CLI-layer deliverable, lands within otp-7 (the session already collects the
docs/plan/OTP7_RESUME.md:127:  per-file fault; this is about where it is reported). The full progress-display
docs/plan/OTP7_RESUME.md-128-  redesign it brushes against is a separate queued item (TODO.md "CLI transfer
docs/plan/OTP7_RESUME.md-129-  output redesign") and is NOT in otp-7 scope.
docs/plan/OTP7_RESUME.md-130-- **D5 — block size**: `ResumeSettings.block_size` clamped to `MAX_BLOCK_SIZE`, `0` ⇒
docs/plan/OTP7_RESUME.md-131-  `DEFAULT_BLOCK_SIZE`. The DEST chooses (it hashes first); the SOURCE reads the size
--
docs/plan/OTP7_RESUME.md-158-3. **Stale-partial fallback** — a dest partial that shares no blocks with the source
docs/plan/OTP7_RESUME.md-159-   ⇒ full content lands, bytes identical, no hang/fault. Guard: force the source to
docs/plan/OTP7_RESUME.md-160-   trust the stale hashes ⇒ corrupt output.
docs/plan/OTP7_RESUME.md-161-4. **Mid-resume-failure** — inject a source fault mid-block-phase ⇒ a clean
docs/plan/OTP7_RESUME.md:162:   `SessionFault` surfaces to both ends, `files_resumed` not incremented for the
docs/plan/OTP7_RESUME.md-163-   aborted file, no deadlock.
docs/plan/OTP7_RESUME.md-164-
docs/plan/OTP7_RESUME.md-165-## Open questions — RESOLVED (owner, 2026-07-09; D-2026-07-09-1)
docs/plan/OTP7_RESUME.md-166-

exec
/bin/zsh -lc 'rtk git blame -L 47,70 70c9688 -- docs/plan/ONE_TRANSFER_PATH.md && rtk git blame -L 203,211 70c9688 -- docs/plan/ONE_TRANSFER_PATH.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XAxWcoTx' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-YIKappQ3' (errno=Operation not permitted)
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 47) ## Non-goals
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 48) 
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 49) - Version compatibility of ANY kind (D-2026-07-05-2, owner standing
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 50)   rule: "backward compatibility is NOT a consideration... same build
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 51)   only. do not engineer tech debt into an unshipped product"). A blit
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 52)   client talks only to a blit-daemon from the same build; the session
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 53)   handshake REFUSES a mismatched peer outright. No negotiate-down, no
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 54)   advisory fields, no feature-capability bits for version skew.
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 55)   `Push`/`PullSync` are deleted at cutover with no bridge. (Old-path
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 56)   code coexists in-tree during the migration slices solely so each
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 57)   slice lands green — that is migration scaffolding, not wire
943b39ca (Michael Coelho 2026-07-05 07:30:18 -0400 58)   compatibility.)
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 59) - Making different hardware perform identically. If src and dst sit
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 60)   on different disks, the two *data directions* still differ by
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 61)   physics; the invariant is that the same data direction between the
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 62)   same endpoints is identical regardless of who initiates and which
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 63)   verb is used.
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 64) - WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 65) - New features. This is a consolidation; capability parity with
06e5413d (Michael Coelho 2026-07-05 07:13:43 -0400 66)   today (mirror, filters, resume, fallback, delegation, progress,
bd47f81e (Michael Coelho 2026-07-05 12:07:28 -0400 67)   jobs, cancellation) is the bar. Zero-copy receive is **unparked**
bd47f81e (Michael Coelho 2026-07-05 12:07:28 -0400 68)   (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
bd47f81e (Michael Coelho 2026-07-05 12:07:28 -0400 69)   after cutover, not one of this plan's slices — see the Design note
bd47f81e (Michael Coelho 2026-07-05 12:07:28 -0400 70)   on the write-strategy seam.
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-0x6LCUJY' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-TXxmfEtE' (errno=Operation not permitted)
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 203) **Resume ordering (RELIABLE exception, codex F5)**: resumed files use
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 204) a strictly-ordered block-hash exchange — the DESTINATION's block map
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 205) for a file must complete before the SOURCE sends any block of that
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 206) file, and stale/mismatched partials fall back to full-file transfer.
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 207) This is an explicit exception to the immediate-start rule, exactly as
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 208) today's resume path is an explicit single-stream RELIABLE exception
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 209) (ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 210) contract; otp-7 pins the stale-partial and mid-resume-failure cases
496357df (Michael Coelho 2026-07-05 07:21:33 -0400 211) in tests.

codex
- [docs/plan/OTP7_RESUME.md:116](/Users/michael/Dev/blit_v2/docs/plan/OTP7_RESUME.md:116) — **High** — D4 retains a session-terminating `SessionFault`, conflicting with the owner’s principle against aborting the whole operation for one surfaceable error. File-only versus whole-session failure semantics remain ambiguous.

- [docs/plan/OTP7_RESUME.md:139](/Users/michael/Dev/blit_v2/docs/plan/OTP7_RESUME.md:139) — **Medium** — Neither 7a nor 7b owns or tests the mandatory CLI fault summary. Both slices could complete without naming failed files or suggesting a re-run.

- [docs/plan/ONE_TRANSFER_PATH.md:65](/Users/michael/Dev/blit_v2/docs/plan/ONE_TRANSFER_PATH.md:65) — **Medium** — The parent excludes all new features, while D-2026-07-09-1 adds a new CLI summary deliverable within otp-7. It needs an explicit narrow exception; the broader TUI redesign remains correctly excluded.

- [docs/STATE.md:185](/Users/michael/Dev/blit_v2/docs/STATE.md:185) — **Low** — The handoff still instructs the next session to obtain Q1–Q3 and flip OTP7_RESUME Active, contradicting the newly recorded Active state.

**VERDICT: CHANGES REQUESTED**
tokens used
67,947
- [docs/plan/OTP7_RESUME.md:116](/Users/michael/Dev/blit_v2/docs/plan/OTP7_RESUME.md:116) — **High** — D4 retains a session-terminating `SessionFault`, conflicting with the owner’s principle against aborting the whole operation for one surfaceable error. File-only versus whole-session failure semantics remain ambiguous.

- [docs/plan/OTP7_RESUME.md:139](/Users/michael/Dev/blit_v2/docs/plan/OTP7_RESUME.md:139) — **Medium** — Neither 7a nor 7b owns or tests the mandatory CLI fault summary. Both slices could complete without naming failed files or suggesting a re-run.

- [docs/plan/ONE_TRANSFER_PATH.md:65](/Users/michael/Dev/blit_v2/docs/plan/ONE_TRANSFER_PATH.md:65) — **Medium** — The parent excludes all new features, while D-2026-07-09-1 adds a new CLI summary deliverable within otp-7. It needs an explicit narrow exception; the broader TUI redesign remains correctly excluded.

- [docs/STATE.md:185](/Users/michael/Dev/blit_v2/docs/STATE.md:185) — **Low** — The handoff still instructs the next session to obtain Q1–Q3 and flip OTP7_RESUME Active, contradicting the newly recorded Active state.

**VERDICT: CHANGES REQUESTED**
