Reading additional input from stdin...
OpenAI Codex v0.144.1
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f581f-8692-7ea3-8d70-6790359c9ddb
--------
user
Review docs/plan/OTP12_PERF_FINDINGS.md (revision, commit 9c7b00e - read the CURRENT working-tree file, NOT the older 1114a92 draft) - an investigation+fix plan for two recorded perf findings. A prior review round already landed these changes: H2 marked CONTRADICTED, H3 candidates corrected, H4 narrowed to shard-boundary/ramp cadence, H5 (lost scan/diff/TCP-transfer overlap) added as prime P2 suspect, pf-1 made a hard gate with an 0f922de historical control, fix bars made both-reference (same-session AND committed), full-matrix final re-verification required before otp-13, and the zoey 0.966 claim reworded as converge-up-only (not a P1 invariance measurement). Verify those revisions are adequate and flag anything still missing or newly inconsistent, plus the usual design checks: (a) H1-H4 vs the code: read the TCP data-plane socket acquisition for both initiator layouts (dial vs accept per resize epoch), the shape-correction/resize controller, the need-list/tar-shard planner cadence, and the destination receive path (per-file open/write/flush/instrument costs) - for each hypothesis say SUPPORTED / CONTRADICTED / UNVERIFIABLE-FROM-CODE with file:line evidence; flag any obvious 5th mechanism the plan missed. (b) Evidence fidelity: the P1/P2 numbers vs the committed records (docs/bench/otp12-win-2026-07-12/README.md, docs/bench/otp12-zoey-2026-07-12/, docs/bench/otp2w-baseline-2026-07-10/summary.csv) - including the claim that zoey's pull_tcp_mixed PASSed 0.966 and the grpc-at-parity claims. (c) Method: will the pf-1 local two-layout phase-timing reproduction actually discriminate H1-H4? Is anything unfalsifiable as staged? Is the P2 bisect (precreate / flush-toggle / ramp-pin experiment flags) sound given the old drivers are deleted? (d) Fix criteria: are the pre-registered bars (wm_tcp_mixed <=1.10, pull_tcp_mixed <=1.10 same-session, push_tcp_small <=1.10 both rigs, unregressed grpc + suite floor 1483) complete - what regression could pass them while violating the parent plan's converge-up constraint? (e) Scope: no wire changes claimed - is that credible for each hypothesis' likely fix (esp. H1 epoch accept/dial and H4 ramp)? (f) Staging: is pf-1-before-any-fix enforced by the doc's wording, and are the CELLS escalation re-runs (not full matrices) sufficient rig evidence for the owner's otp-13 walk? Output a concise markdown findings list - each finding with file:line, severity, rationale - then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review the current working-tree plan, split the code-path and benchmark-record checks in parallel, then reconcile the findings against the plan’s exact staging, falsifiability, scope, and acceptance language. I’ll make no changes.
exec
/bin/zsh -lc "pwd && git status --short && git remote -v && git rev-parse HEAD && git log -1 --oneline --decorate && sed -n '1,260p' AGENTS.md && sed -n '1,320p' .agents/repo-guidance.md && sed -n '1,280p' .agents/state.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/Users/michael/Dev/blit_v2
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-u3EZ52Dq' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LTuAKMqV' (errno=Operation not permitted)
?? .review/results/otp12-perf-findings.codex.md
?? .review/results/otp12-perf-findings.codex.stale-1114a92-rerun.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XM1isIZ7' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-bLQ5LoHO' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-8jwhffRA' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-t65OGkNI' (errno=Operation not permitted)
9c7b00eb60095ffb92ebf02b4441335e4a854d67
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2rntB4OG' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-DLRIDtJI' (errno=Operation not permitted)
9c7b00e (HEAD -> master) otp-12pf: address design review (7 findings) — H2 contradicted, H3 corrected, H4 narrowed, H5 added, P2 bisect redesigned with 0f922de historical control, both-reference fix bars, pf-1 hard gate + no mixed-build acceptance
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
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves — `AGENTS.md` and `.agents/*` — are in scope as drift targets, not just sources of truth. `drift` also owns the deliberate state-hygiene pass: rotate landed or superseded `## Now` entries verbatim to `docs/history/state-archive.md` (create on first use); re-verify the recorded basis of every parked or blocked item and move anything falsified into `## Blockers` with the new evidence; volatile facts (push status, CI state, counts) carry `as of <commit>` and are re-verified or dropped; a count or enumeration another file owns is pointed to, never copied; machine-specific facts relocate to `.agents/machines.md`, and stale entries there are pruned.
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

exec
/bin/zsh -lc "git ls-remote github HEAD; sed -n '1,280p' docs/STATE.md; rg -n \"OTP12|otp-12|converge|perf\" docs/STATE.md docs/DECISIONS.md REVIEW.md docs/plan -g '*.md'; wc -l docs/plan/OTP12_PERF_FINDINGS.md; nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '1,420p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-oxOSYDdZ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-q6dIgoYH' (errno=Operation not permitted)
fatal: unable to access 'https://github.com/roethlar/Blit.git/': Could not resolve host: github.com
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-12

- Recent sessions (2026-07-11/12, 44th–45th): **otp-10 AND otp-11 fully closed through the codex loop** — every transfer (local included) rides the ONE session; the separate local orchestration no longer exists (−6.2k lines at 11b); the old journal fast path was proven UNSOUND (data-loss repro recorded) and died with it. Suite **1484** (the otp-13 ≥1483 floor met at the deletion slice). SMALL_FILE_CEILING paused (D-2026-07-05-1). Push state: see Blocked.

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
  - **Closed `[x]`: otp-1, otp-3, otp-4 (a, b-1/2/3), otp-5 (a,
    b-1/2), otp-6 (a/b), otp-7 (a, b-1/2), otp-8, otp-9 (a/b)** —
    the full session machine: contract, role drivers, daemon
    serving, both data planes + sf-2 resize + cancel, mirror/filters
    (one delete rule), resume both carriers (wire bounds
    D-2026-07-10-1/-2), fallback byte-carrier, delegated-on-session.
    Suite → **1555** (as of `1ce73b5`; later commits are
    bench/docs-only). SizeMtime = data-safe skip (open Q below).
    Per-slice detail: DEVLOG 2026-07-10 entries + `.review/`.
  - **otp-2 `[x]` (both halves).** zoey = PER-DIRECTION reference;
    Mac↔Windows = cross-direction rig (otp-2w). Harnesses
    `scripts/bench_otp2{,w}_baseline.sh`, evidence
    `docs/bench/otp2{,w}-baseline-2026-07-10/README.md`. Key reading:
    old push trails old pull on BOTH rigs — otp-12's interleaved
    old-vs-new discriminates code cost from platform write-path cost.
  - **otp-10 `[x]` CLOSED (a, b-1/2, c-1/2)** — verb cutover + THE
    CUTOVER DELETION: one chokepoint per verb shape (`blit_app
    run_remote_push`/`run_remote_pull`), ONE args→compare mapping,
    move maps IgnoreTimes/Checksum-only on every route; relay removed
    (D-2026-07-11-1); 4 drivers + `Push`/`PullSync` + 13 messages out
    of tree AND proto (−13.8k lines, no bridge); DelegatedPull
    no-payload proof recorded. Suite 1555 → … → **1488**. Per-slice
    detail: DEVLOG 2026-07-11 entries + `.review/`.
  - **otp-11 `[x]` CLOSED (a + addendum + b)** — local transfers ride
    the session (`run_local_session` over `in_process_pair`; the
    LOCAL byte-carrier = process-local `LocalApply`, no wire shape,
    clonefile/block-clone preserved — slice design
    `docs/plan/OTP11_LOCAL_SESSION.md`, every round codex-reviewed:
    design 10 + slice 9 + addendum 4 + deletion 6 findings, all
    adjudicated in `.review/results/otp-11*`). Perf gate PASS against
    SOUND baselines (1 GiB local = 22 ms both binaries; the old 21 ms
    journal no-op was proven UNSOUND — silent data loss on deep
    modifications, repro in `docs/bench/otp11-local-2026-07-11/`).
    **11b deleted the whole old orchestration** (−6.2k lines:
    orchestrator/engine/local_worker/auto_tune/change_journal +
    the compare_manifests sweep; dial re-homed verbatim; types →
    `transfer_session/local.rs`); the acceptance criteria's
    deletion-proof line for "the separate local orchestration path"
    COMPLETES. Suite 1488 → 1513 → **1484** (≥1483 floor met at the
    deletion slice, margin +1). Detail: DEVLOG 2026-07-12 entries.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
  baseline. Principle stands: ceiling-driven, never competitor-relative
  (D-2026-07-04-4; a ≥25% margin answer was retracted — do not
  re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete, gates
  DATA-COMPLETE (declarations pending in Blocked); codex loop governs
  all changes (D-2026-07-04-1; DEVLOG 07-04/05).

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
   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). Current: **otp-12c
   (delegated, netwatch-01↔skippy)**, then 12d, otp-13.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Optional follow-ups
   largely absorbed by otp-2/otp-12's rig matrices; skippy env facts
   moved to Blocked → Rig availability.
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
   open row) — same directive; w7-1 likely landed for free inside
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
   tuning residue (w3-1 scoped it out); the source send half's bounded
   `dp.queue()` is not raced against control-lane events (deferred at
   codex otp-7b-1 F3; otp-8 F1 gave the in-stream sends a fault race —
   residual: the narrow CANCELLED→INTERNAL decay, verdict file);
   CLI progress monitor lives through the in-session mirror purge
   (display-only ticks/avg dilution; fix = the M-C `AppProgressEvent`
   phase reshape — deferred at codex otp-10b-2 F5).

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

- **Rigs**: owner go GIVEN 2026-07-12; zoey (12a) + netwatch-01 (12b)
  sessions done. Remaining: 12c delegated = netwatch-01↔skippy
  (`admin@skippy`, x86_64, pool paths only; fresh staging needed).
- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)
- **Push go**: origin/master = `6d37a22` (re-verified via `ls-remote`
  2026-07-11 — a partial push landed outside these sessions); unpushed
  `6d37a22..HEAD` (12 at the 10c-1 record). Awaits the ref-listing +
  approval flow; windows-latest CI on the w9-3 fix rides it.
- **otp-5b-3** (pull mid-transfer cancel e2e, marked optional): pick
  up while otp-10 runs, or drop? — standing question.
- ~~The change-journal question~~ **RESOLVED 2026-07-12 (owner:
  "neither option passes — figure out a real fix"; the premise was
  false)**: the old 21 ms journal skip was UNSOUND — `NoChanges`
  decays to root-dir mtime equality, so deep modifications silently
  never synced (REPRODUCED against the pre-otp-11 binary; transcript
  in `docs/bench/otp11-local-2026-07-11/README.md`). Sound-vs-sound
  the session no-op wins 2.2× (226 vs 507 ms/10k, 5-run medians) →
  gate passes;
  11b's journal deletion removes a data-loss bug. Pinned:
  `deep_modification_after_warm_runs_syncs`. Sound O(changes) no-op
  (journal REPLAY as a session phase, both carriers) filed as future
  capability — slice doc D3. **otp-11b is UNBLOCKED.**

## Open questions

- **(OPEN — owner ack, 2026-07-05, otp-4a)** Unified SizeMtime semantic:
  same-size + dest-NEWER — old push clobbers, session adopts **data-safe
  SKIP** (converge-up; `--force` still overwrites; pinned by
  `same_size_newer_destination_is_skipped_not_clobbered`). Owner: confirm
  or ask for old-push clobber. Reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
- **(OPEN)** `725aa07` tracked a stale worktree snapshot (rec
  `git rm -r`, awaits go); historical docs embed `/Users/...` paths
  (rec: leave).
- **(OPEN, 2026-07-04)** `docs/WHITEPAPER.md` describes the deleted
  `determine_remote_tuning` — fold into w10-docs-batch?
- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
- **(OPEN, new 2026-07-05)** CLI foot-gun: a bare local dir name with
  no `./` parses as an mDNS discovery endpoint and errors (blit-app
  endpoints.rs). Local-path existence wins, or better error? Owner to
  slot.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: w9-3 fixed the
  Linux daemon-spawn flakiness; windows-latest CI pending the next
  push. NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-12 (45th, this session)** — **otp-11 CLOSED WHOLE (11a
  route + journal-hole addendum + 11b deletion, four codex rounds;
  suite 1488 → 1484 with the ≥1483 floor met by real pins; the
  separate local orchestration no longer exists)**. In-flight: none;
  tree clean. **Next**: otp-12 (rig-gated, Blocked) → otp-13.
- **2026-07-11 (44th)** — otp-10c closed (relay removal + the cutover
  deletion); suite 1605 → 1488. Owner ask pending: `725aa07` snapshot.
- **2026-07-11 (43rd)** — otp-10a/10b closed; verb cutover complete.
- *(42nd and earlier pruned to the cap — see DEVLOG 2026-07-06..12.)*
docs/STATE.md:19:  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
docs/STATE.md:36:    old push trails old pull on BOTH rigs — otp-12's interleaved
docs/STATE.md:80:   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
docs/STATE.md:86:   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). Current: **otp-12c
docs/STATE.md:90:   largely absorbed by otp-2/otp-12's rig matrices; skippy env facts
docs/STATE.md:109:   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
docs/STATE.md:170:  SKIP** (converge-up; `--force` still overwrites; pinned by
docs/STATE.md:196:  tree clean. **Next**: otp-12 (rig-gated, Blocked) → otp-13.
docs/plan/OTP12_ACCEPTANCE_RUN.md:1:# otp-12 — symmetric-rig acceptance run (design)
docs/plan/OTP12_ACCEPTANCE_RUN.md:8:**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-12.
docs/plan/OTP12_ACCEPTANCE_RUN.md:19:otp-12 is the plan's acceptance-evidence slice: rerun the otp-2 matrix on the
docs/plan/OTP12_ACCEPTANCE_RUN.md:30:## What otp-12 must produce (plan anchors)
docs/plan/OTP12_ACCEPTANCE_RUN.md:79:  otp-12 arm**; skippy gets fresh staging (D6).
docs/plan/OTP12_ACCEPTANCE_RUN.md:95:| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
docs/plan/OTP12_ACCEPTANCE_RUN.md:96:| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
docs/plan/OTP12_ACCEPTANCE_RUN.md:111:arm (8 timed runs per comparison). A = `old` (rig Z/W converge-up) or
docs/plan/OTP12_ACCEPTANCE_RUN.md:145:- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
docs/plan/OTP12_ACCEPTANCE_RUN.md:166:  while passing per-direction converge-up AND invariance, the evidence
docs/plan/OTP12_ACCEPTANCE_RUN.md:172:  APFS — the plan's Non-goals: different hardware need not perform
docs/plan/OTP12_ACCEPTANCE_RUN.md:181:**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
docs/plan/OTP12_ACCEPTANCE_RUN.md:276:  inside the timed window; otp-12 records it per run (`exit` column) and a
docs/plan/OTP12_ACCEPTANCE_RUN.md:290:where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
docs/plan/OTP12_ACCEPTANCE_RUN.md:299:`converge|invariance|delegated|cross|cross-gap`.
docs/plan/OTP12_ACCEPTANCE_RUN.md:305:converge row whose same-session block-1 counterpart is absent or
docs/plan/OTP12_ACCEPTANCE_RUN.md:352:| Z converge-up | 12 (3 fixtures × 2 dirs × 2 carriers) | 96 | 1.5–2.5 h (drains dominate) |
docs/plan/OTP12_ACCEPTANCE_RUN.md:353:| W converge-up | 12 | 96 | ~1.5 h |
docs/plan/OTP12_ACCEPTANCE_RUN.md:363:- **otp-12a — rig Z**: `bench_otp12_zoey.sh` (harness commit; codex; fix) →
docs/plan/OTP12_ACCEPTANCE_RUN.md:368:- **otp-12b — rig W**: `bench_otp12_win.sh` covering converge-up block +
docs/plan/OTP12_ACCEPTANCE_RUN.md:372:- **otp-12c — rig D**: `bench_otp12_delegated.sh`; same shape. Preflight
docs/plan/OTP12_ACCEPTANCE_RUN.md:377:- **otp-12d — assembly**: `docs/bench/otp12-acceptance-<date>/README.md` —
docs/plan/OTP12_ACCEPTANCE_RUN.md:415:  otp-12; the ≥1483 floor stands at 1484 from otp-11b.
docs/DECISIONS.md:77:## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
docs/DECISIONS.md:78:- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
docs/DECISIONS.md:79:- Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
docs/DECISIONS.md:90:- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
docs/DECISIONS.md:91:- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
docs/DECISIONS.md:97:- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.
docs/DECISIONS.md:99:## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
docs/DECISIONS.md:102:- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*
docs/DECISIONS.md:130:- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
docs/DECISIONS.md:145:- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
docs/DECISIONS.md:150:- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
docs/DECISIONS.md:169:## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
docs/DECISIONS.md:170:- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
docs/DECISIONS.md:171:- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
docs/DECISIONS.md:172:- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
REVIEW.md:33:| ue-r2-1f | Push converge through the engine; retire daemon `desired_streams` ladder | `[x]` | `a4a9f70` + review fix `0c8da50` |
REVIEW.md:75:| otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |
REVIEW.md:77:| otp-12a | Zoey converge-up A/B recorded (design `docs/plan/OTP12_ACCEPTANCE_RUN.md` Active — owner flip; D-2026-07-12-1 residue rule). Three codex rounds: design CHANGES REQUIRED 7 findings (6 accepted + 1 overtaken-by-owner-decision); harness REQUEST CHANGES 9/9 accepted (zero false positives); run round FAIL 6/6 accepted (provenance `+sha` form, D2 supersession amendment, drift/gap wording per CSVs). En route: otp-2 daemon provenance corrected (staged pair was dirty `731023b`, not `e757dcc`); zoey I/O-storm diagnosed → per-run dest sweep. Evidence `docs/bench/otp12-zoey-2026-07-12/` (3 sessions incl. aborted storm): **10 PASS; pull_tcp_large FAIL-REFERENCE-DRIFT (rig-side by strongest evidence); push_tcp_small FAIL-SAME-SESSION 1.105** — both carried to the otp-13 walk. | `[x]` | design `045da4a`+`92e1d51`; harness `8f4fbf9`+`50dc135`; run `b2b6901`+`b3729da`+`042c06f`+`6bc9cb6`+`b0ebf73`+fixes `fa18787` |
REVIEW.md:78:| otp-12b | Mac↔Windows acceptance session recorded — THE INVARIANCE CRITERION MEASURED: 11/12 cells PASS at 1.003–1.057 (the owner's sentence holds); wm_tcp_mixed FAIL 1.237 (TCP×mixed×destination-initiator — real, block-1-corroborated, code-shaped). Converge 10/12 (push_tcp_small 1.149 FAIL-BOTH — matches zoey's 1.105, second rig; pull_tcp_mixed 1.313 same root). Cross: Win→Mac 6/6 beat the better old direction; Mac→Win gap rows recorded per D-2026-07-12-1 shapes (large unchanged / mixed+grpc_small narrowed / tcp_small widened), adjudication reserved to otp-13. Three codex rounds: harness FAIL 12/12 accepted; run-round FAIL 3/3 accepted (self-adjudication scrubbed); + two found-live fixes (pwsh `$rc:R` scope-parse sentinel; CR-split verdicts). 192 runs, zero voided. Evidence `docs/bench/otp12-win-2026-07-12/`. | `[x]` | harness `d30b1e3`+`772cfe6`+`d3eae58`; run `e21cf84`+`856af64`+`44c2046`+fixes `49dee5c` |
REVIEW.md:182:| rec-1-recent-persistence | Feature | Persist `GetState.recent[]` across daemon restarts via dedicated recents.jsonl (separate from planner's perf_local.jsonl); non-blocking write-through + atomic rewrite, opt-in (recent-persistence step 1) | `[x]` | `phase5/a1` | `7c095b2` |
REVIEW.md:183:| rec-2-clear-recent | Feature | `ClearRecent` RPC: wipe recent ring + recents.jsonl, never touching planner's perf_local.jsonl (core safety test); empty request, count response (recent-persistence step 2) | `[x]` | `phase5/a1` | `9c2955e` |
REVIEW.md:272:- `334a684` diagnostics (perf + dump)
REVIEW.md:273:- `2626f9b` diagnostics — perf best-effort fix
REVIEW.md:328:- `d33fedc` F4 Profile pane with read-only perf history + predictor (`a1-5-f4-profile`)
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:80:Both audits converged on the same headline: the prior
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:128:- Local performance history (`perf_history.rs`) with capped JSONL,
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:328:shell completions, and performance profiling.
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:363:0.1.0 ships with documented "performance claims to be verified" and
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:365:0.1.0 depends on the benchmark numbers; the perf-history /
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:375:`docs/perf/remote_remote_benchmarks.md` is template-only.
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:380:network, capture results into the perf doc with avg + best MiB/s
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:482:`perf_predictor.rs`, drop `update_predictor` calls, keep
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:483:`perf_history` + `derive_local_plan_tuning` which are the
docs/plan/ONE_TRANSFER_PATH.md:38:performs. A transfer has a SOURCE role and a DESTINATION role; which
docs/plan/ONE_TRANSFER_PATH.md:59:- Making different hardware perform identically. If src and dst sit
docs/plan/ONE_TRANSFER_PATH.md:129:      a cell that meets per-direction converge-up and invariance but
docs/plan/ONE_TRANSFER_PATH.md:132:      `docs/plan/OTP12_ACCEPTANCE_RUN.md` D2.)
docs/plan/ONE_TRANSFER_PATH.md:230:in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
docs/plan/ONE_TRANSFER_PATH.md:243:faster direction — mitigated by the converge-up constraint and
docs/plan/ONE_TRANSFER_PATH.md:276:   rig. This is the converge-up reference the acceptance criteria
docs/plan/ONE_TRANSFER_PATH.md:304:    separate local orchestration is deleted; local perf pins hold.
docs/plan/ONE_TRANSFER_PATH.md:305:12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
docs/plan/PIPELINE_UNIFICATION.md:177:  performance and simplicity win for cross-server workloads.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:15:**Decision refs**: D-2026-06-20-1 (convergence direction),
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:21:Keep the v1 direction: converge the transfer subsystem around one
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:28:REV3 keeps convergence, not rebuild. It tightens the plan where review
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:130:The existing code already has useful convergence substrate:
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:195:The 1s start requirement cannot be hidden inside the sequencer-converge
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:210:(`perf_history` appends a `PerformanceRecord` per transfer;
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:211:`perf_predictor` loads it and trains per-profile coefficients):
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:355:6. **`ue-r2-1f-push-converge`** — Route push through the engine while
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:358:7. **`ue-r2-1g-pull-multistream-converge`** — Route PullSync through the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:395:  push convergence, and pull convergence into separate slices.
docs/plan/REMOTE_TRANSFER_PARITY.md:23:- ⚠️ We still need end-to-end performance validation + documentation once the new batching ships to the field, along with the remote↔remote (server-to-server) orchestration so every src/dst combination has parity.
docs/plan/WORKFLOW_PHASE_3.md:53:| 3.3.4 | Implement CLI data plane client: token validation, gRPC fallback, progress events. *(2025-11-10: `RemotePullClient` now connects to the negotiated TCP port, applies file/tar shard payloads locally, records summaries, and the CLI reuses the shared `RemoteTransferProgress` monitor for both push/pull. Auto-tune chunk sizing + payload prefetch are now plumbed through both push and pull, and manifest need-lists flush immediately so first payloads launch within milliseconds even on huge manifests. 2025-11-15: `RemotePushClient` gained size-aware batching so multi-stream sends actually utilize every TCP worker, and daemon/client heuristics now negotiate up to 16 TCP streams on multi-GiB manifests; next step is capturing perf logs + remote↔remote orchestration.)* | `blit-cli` transport layer + integration tests. |
docs/plan/WORKFLOW_PHASE_3.md:69:| 3.4.5 | Integrate `profile` command with local performance history (read-only insights). | CLI output + documentation. |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:940:   - Capture result template in `docs/perf/remote_remote_benchmarks.md`.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:942:   `docs/perf/remote_remote_benchmarks.md` with real results.
docs/plan/BENCHMARK_10GBE_PLAN.md:34:No network needed. Confirms the refactored local→local path works and performs.
docs/plan/BENCHMARK_10GBE_PLAN.md:62:- [ ] Small file perf — NFS will be slower than local due to metadata RTTs
docs/plan/BLIT_UTILS_PLAN.md:38:| `blit-utils profile` | Displays local performance history / predictor coefficients. | Reuses existing JSONL + predictor state. |
docs/plan/WORKFLOW_PHASE_2.md:3:**Goal**: Deliver the local transfer pipeline defined in plan v6 (streaming planner, adaptive predictor, local performance history, and progress UX) while keeping FAST/SIMPLE/RELIABLE/PRIVATE principles intact.
docs/plan/WORKFLOW_PHASE_2.md:6:**Critical Path**: Adaptive predictor/performance history, CLI progress UX.
docs/plan/WORKFLOW_PHASE_2.md:13:- Telemetry log and `blit diagnostics perf` work; predictor adjusts routing automatically.
docs/plan/WORKFLOW_PHASE_2.md:19:1. **No user tunables** – Planner owns performance decisions. The sole debug limiter (`--workers`) must be clearly labelled, pause “FAST” guarantees when active, and remain hidden from normal help output (documented in `docs/cli/blit.1.md`).
docs/plan/WORKFLOW_PHASE_2.md:38:| 2.2.1 | Implement local performance history writer (capped JSONL). | `perf_history.rs` with rotate-on-size logic. |
docs/plan/WORKFLOW_PHASE_2.md:41:| 2.2.4 | Add `blit diagnostics perf` CLI command. | ✅ Command prints recent runs + stats. |
docs/plan/WORKFLOW_PHASE_2.md:42:| 2.2.5 | Add CLI/config toggle for telemetry (`profile` command remains visible). Replace environment variable usage. | Diagnostics toggles (`blit diagnostics perf --enable/--disable`) + settings file. |
docs/plan/WORKFLOW_PHASE_2.md:60:| 2.4.3 | Keep macOS/Linux + Windows benchmarks v2-only (synthetic payload, perf-history disabled by default) and capture rsync/robocopy baselines. | `scripts/bench_local_mirror.sh` (vs `rsync`) / `scripts/windows/bench-local-mirror.ps1` (vs `robocopy`) emit summary timings + log paths. |
docs/plan/WORKFLOW_PHASE_2.md:61:| 2.4.4 | Quantify performance history warm-up impact (first vs. 10th vs. 100th run) across representative workloads. | Benchmark report captured in docs with hard numbers and log references. |
docs/plan/WORKFLOW_PHASE_2.md:85:| Predictor destabilises routing | Start with conservative defaults; log mispredictions; allow performance history opt-out |
docs/plan/UNIFIED_RECEIVE_PIPELINE.md:287:- `DEVLOG.md` — entry covering the unification + perf restoration.
docs/plan/UNIFIED_RECEIVE_PIPELINE.md:300:   payload. At 35k files this is 35k allocations. Negligible perf
docs/plan/UNIFIED_RECEIVE_PIPELINE.md:326:- **Total: ~9 hours of focused work**, plus iteration if perf doesn't
docs/plan/UNIFIED_RECEIVE_PIPELINE.md:333:  large TCP throughput. Both within 10 % of iperf3 baseline.
docs/plan/POST_REVIEW_FIXES.md:293:- **Linear perf-predictor model** — a per-profile linear regression
docs/plan/POST_REVIEW_FIXES.md:309:to confirm no perf regression. Then decide between Round 2.1 (journal
docs/plan/WORKFLOW_PHASE_4.md:52:| 4.4.2 | Ensure `blit diagnostics perf`, `blit-utils profile`, and other support tools produce useful output. | Sanity tests + docs. |
docs/plan/WORKFLOW_PHASE_4.md:89:| 4.8.3 | *(0.2.0)* Extend `blit diagnostics profile` to run local probes and attach results to performance history/telemetry. | CLI profile output updated + docs. |
docs/plan/review/code_review_phase2.md:4:**Scope:** `orchestrator.rs`, `perf_history.rs`, `perf_predictor.rs`, `local_worker.rs`, supporting docs
docs/plan/review/code_review_phase2.md:18:| Performance history rotation | `enforce_size_cap` rewrote the log without protecting against concurrent appends (possible record loss, O(n²) trimming). | **Fixed** in `perf_history.rs`: switch to `VecDeque`, only rewrite when we actually trim, and skip the rewrite if the file grew after we sampled its size. |
docs/plan/review/code_review_phase2.md:19:| Predictor granularity | Profiles ignored `skip_unchanged` / `checksum`, mixing radically different planner costs. | **Fixed** in `perf_predictor.rs` (`ProfileKey` now captures both flags; orchestrator supplies them). |
docs/plan/review/code_review_phase2.md:40:- The gradient-descent update (`LEARNING_RATE = 0.0005`) is intentionally scaled by the error magnitude and file/byte counts; it converges quickly in practice (dozens of observations), so no change is required.
docs/plan/review/code_review_phase2.md:47:1. Profile the Windows copy path (perf history now clean; benchmark gap tracked in Phase 2.5 doc).
docs/plan/review/code_review_phase2.md:49:3. Re-run the Windows benchmark after performance work and update the Phase 2.5 gate status.
docs/plan/OTP12_PERF_FINDINGS.md:1:# otp-12 perf findings — investigate + fix before otp-12c (design)
docs/plan/OTP12_PERF_FINDINGS.md:5:fix once converged" — the flip to Active happens at codex convergence
docs/plan/OTP12_PERF_FINDINGS.md:10:review." otp-12a/b measured exactly two such cells; otp-12c/12d/13 are
docs/plan/OTP12_PERF_FINDINGS.md:27:zoey's rig anchors converge-up only (12a README), so it has no
docs/plan/OTP12_PERF_FINDINGS.md:123:   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
docs/plan/OTP12_PERF_FINDINGS.md:128:- Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in an
docs/plan/OTP12_PERF_FINDINGS.md:165:  re-verifies on the full matrix; then otp-12c proceeds on the fixed
docs/plan/OTP12_PERF_FINDINGS.md:175:- zoey never measured P1: its rig anchors converge-up only, so there
docs/plan/UNIFIED_TRANSFER_ENGINE.md:12:question (resolved as *convergence*, not rebuild — D-2026-06-20-1).
docs/plan/UNIFIED_TRANSFER_ENGINE.md:15:shared byte-moving leaf but never converged the sequencer+dials layer
docs/plan/UNIFIED_TRANSFER_ENGINE.md:49:  payload planner (`plan_transfer_payloads`) stay. This converges the layer
docs/plan/UNIFIED_TRANSFER_ENGINE.md:127:Two seams + one convergence. The byte-mover at the bottom is already shared
docs/plan/UNIFIED_TRANSFER_ENGINE.md:128:(map confirmed 2026-06-20); we converge the layer above it.
docs/plan/UNIFIED_TRANSFER_ENGINE.md:130:### 1. The engine — sequencer convergence
docs/plan/UNIFIED_TRANSFER_ENGINE.md:211:- **PR2** is both a perf win (a slow sink can no longer head-of-line-block
docs/plan/UNIFIED_TRANSFER_ENGINE.md:230:  converged engine + the finished resize protocol, not because it is
docs/plan/UNIFIED_TRANSFER_ENGINE.md:266:last because it needs the converged engine + finished resize protocol,
docs/plan/UNIFIED_TRANSFER_ENGINE.md:280:3. **`ue-1c-sequencer-converge`** — introduce the src/dst-agnostic
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:1:# Unified Transfer Engine REV4 — the Active convergence plan (code-reality corrected)
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:22:  question (resolved as *convergence*, not rebuild — D-2026-06-20-1).
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:25:  shared byte-moving leaf but never converged the sequencer+dials layer.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:31:**Decision refs**: D-2026-06-20-1 (convergence direction),
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:37:Keep the v1 direction: converge the transfer subsystem around one
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:44:REV4 keeps convergence, not rebuild. It tightens the plan where review
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:124:      field 4) those ladders feed onto the wire. After convergence no
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:182:The existing code already has useful convergence substrate:
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:195:- Cross-run history exists in-tree: `perf_history::PerformanceRecord`
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:196:  (`perf_history.rs:135`) is appended per transfer; `perf_predictor`
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:197:  (`perf_predictor.rs`) `load()`s it (:220) and maintains per-profile
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:295:The 1s start requirement cannot be hidden inside the sequencer-converge
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:310:(`perf_history` appends a `PerformanceRecord` per transfer;
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:311:`perf_predictor` loads it and trains per-profile coefficients):
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:425:- **Under-converged ladders (new in REV4).** Because REV3 mis-counted the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:426:  ladders, a coder following it would have converged only
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:472:6. **`ue-r2-1f-push-converge`** — Route push through the engine while
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:478:7. **`ue-r2-1g-pull-multistream-converge`** — Route PullSync through the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:521:  push convergence, and pull convergence into separate slices.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:542:  "under-converged ladders" risk.
docs/plan/greenfield_plan_v6.md:5:**Strategy**: A greenfield Cargo Workspace using a hybrid transport model: gRPC for control and a raw TCP data plane for maximum performance.
docs/plan/greenfield_plan_v6.md:23:2.  **Data Plane (Raw TCP):** For the actual bulk transfer of large files, the control plane will negotiate a separate, short-lived, raw TCP connection. This allows us to reuse the hyper-optimized, zero-copy `sendfile` and `splice` logic from v1 directly on a raw socket, bypassing any potential gRPC overhead and guaranteeing maximum performance.
docs/plan/greenfield_plan_v6.md:66:// to establish the high-performance data plane connection.
docs/plan/greenfield_plan_v6.md:119:**Goal:** Objectively verify that the new architecture's local performance is acceptable before building network features.
docs/plan/greenfield_plan_v6.md:158:2. **Hybrid Remote Transport** — Remote push/pull mirror the v1 data-path performance by keeping:
docs/plan/greenfield_plan_v6.md:165:   - Capped JSONL log (`~/.config/blit/perf_local.jsonl`) storing workload signature, planner/copy durations, stall events.
docs/plan/greenfield_plan_v6.md:166:   - `blit diagnostics perf` surfaces recent runs for troubleshooting.
docs/plan/greenfield_plan_v6.md:218:   - Add `blit diagnostics perf` command.
docs/plan/greenfield_plan_v6.md:263:- AuthN/authZ (token-based or mTLS) once core performance is validated.
docs/plan/greenfield_plan_v6.md:291:3. Do not add user-facing performance tunables unless explicitly approved.
docs/plan/greenfield_plan_v6.md:338:- `move` performs a mirror followed by source removal (local or remote).
docs/plan/greenfield_plan_v6.md:349:- Implement subcommands: `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, and a `profile` command for local performance capture.
docs/plan/TUI_REWORK.md:63:   transfer the CLI can perform must have a TUI path; no scenario
docs/plan/TUI_DESIGN.md:68:| `blit profile` | F4 Profile / status bar | Local perf-history summary; one-shot read of `~/.config/blit/perf_local.jsonl`. |
docs/plan/TUI_DESIGN.md:69:| `blit diagnostics perf` | F4 settings panel | Enable / disable / clear toggles. |
docs/plan/TUI_DESIGN.md:246:┌─ Profile (local performance history) ────────────────────────┐
docs/plan/TUI_DESIGN.md:266:- Profile pane reads `perf_local.jsonl` directly — no RPC
docs/plan/TUI_DESIGN.md:778:| `crates/blit-cli/src/diagnostics.rs` (dump emitter + perf toggles) | `blit-app::diagnostics::{dump, perf}` |
docs/plan/TUI_DESIGN.md:779:| `crates/blit-cli/src/profile.rs` (perf-history summarizer) | `blit-app::profile` |
docs/plan/TUI_DESIGN.md:957:- F4 Profile: reads `~/.config/blit/perf_local.jsonl` directly.
docs/plan/TUI_DESIGN.md:1016:   `perf_local.jsonl` (durable, reuses existing storage)?
docs/plan/TUI_DESIGN.md:1018:   for B; if persistence is wanted later, reuse `perf_local`
docs/plan/MULTISTREAM_PULL.md:10:(superseded it — convergence plan absorbs the goal).
docs/plan/PROJECT_STATE_ASSESSMENT.md:47:- Adaptive performance predictor with online gradient descent
docs/plan/PROJECT_STATE_ASSESSMENT.md:123:├── perf_predictor    — adaptive heuristics
docs/plan/PROJECT_STATE_ASSESSMENT.md:124:├── perf_history      — versioned JSONL storage
docs/plan/MASTER_WORKFLOW.md:15:2. **SIMPLE** – No user-facing speed knobs. Planner, orchestrator, and heuristics own performance.
docs/plan/WORKFLOW_PHASE_2.5.md:3:**Goal**: Confirm Blit v2 meets the plan v6 local-performance targets (≥95 % of baseline workloads) before proceeding to remote work.
docs/plan/SMALL_FILE_CEILING.md:109:   pull write paths): profile first (`strace -c`/`perf` during a
docs/plan/SMALL_FILE_CEILING.md:144:   precedent): `strace -c`/`perf` profile of daemon receive and
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:14:- **Privacy:** performance history is strictly local; nothing leaves the user’s machine.
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:83:- Metrics are stored locally as a capped JSON Lines file (e.g., `~/.config/blit/perf_local.jsonl`, max ~1 MiB).
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:85:- No data is sent off-machine. Use `blit diagnostics perf --disable` (and `--enable`) to toggle recording in the local config directory.
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:89:- Metrics feed diagnostic tooling (`blit diagnostics perf`) for support. 
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:117:- When performance history capture is disabled, prediction falls back to conservative defaults and no updates occur.
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:124:- Small-file workloads (≥32 sub-1 MiB files or avg size ≤64 KiB) immediately enter the tar-shard path; shards flush around 8 MiB/≈1 k files and scale up to 32/64 MiB as manifests grow, keeping per-file overhead invisible. Recent performance history nudges these thresholds to mirror what actually worked best on the current machine.
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:145:   - Prediction model persistence (e.g., store coefficients alongside performance history log).
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:172:| Do we expose performance summaries? | Yes, via `blit diagnostics perf` (local only). |
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:174:| Cross-filesystem performance differences? | Predictor coefficients segmented by source/dest FS profile; transfer engine monitors backpressure to throttle. |
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:186:- Consider remote performance history opt-in to improve heuristics globally (opt-in only).
docs/plan/LOCAL_TRANSFER_HEURISTICS.md:191:**Status:** Updated design approved for immediate implementation. No further staged phases; work continues until the entire orchestration stack meets performance goals.
docs/plan/WORKFLOW_V2.md:9:**Goal:** Realise the v5 local design—streaming planner, adaptive predictor, and performance-history-backed heuristics.
docs/plan/WORKFLOW_V2.md:42:1. TLS for control plane (optional data-plane TLS after perf validation).
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:10:**Decision refs**: D-2026-06-20-1 (convergence direction),
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:16:Keep the v1 direction: converge the transfer subsystem around one
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:23:REV2 keeps convergence, not rebuild. It tightens the plan where review
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:105:The existing code already has useful convergence substrate:
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:166:The 1s start requirement cannot be hidden inside the sequencer-converge
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:271:6. **`ue-r2-1f-push-converge`** - Route push through the engine while
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:274:7. **`ue-r2-1g-pull-multistream-converge`** - Route PullSync through the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:289:  push convergence, and pull convergence into separate slices.
docs/plan/OTP11_LOCAL_SESSION.md:11:or the local perf pins.
docs/plan/OTP11_LOCAL_SESSION.md:46:  performs a (zero-copy when possible) local copy" — and `FsTransferSink`
docs/plan/OTP11_LOCAL_SESSION.md:54:  full read+write of every byte — the 1 GiB local perf pin would fail by
docs/plan/OTP11_LOCAL_SESSION.md:122:perf gate. At 11b, `docs/TRANSFER_SESSION.md` gains a short "Local
docs/plan/OTP11_LOCAL_SESSION.md:140:`perform_local_move_lands_source_bytes_over_matching_metadata`,
docs/plan/OTP11_LOCAL_SESSION.md:141:`perform_local_move_deletes_source_after_copy`): `build_local_options`'
docs/plan/OTP11_LOCAL_SESSION.md:219:  consumer): retired. `perf_predictor.rs` stays (readers: `blit profile`),
docs/plan/OTP11_LOCAL_SESSION.md:223:  `perf_history.rs` already survives for its readers. (Remote session runs
docs/plan/OTP11_LOCAL_SESSION.md:239:   (D1), option mapping + summary synthesis (D2), perf-history write (D3).
docs/plan/OTP11_LOCAL_SESSION.md:249:4. Bench gate (perf pins): A/B on this machine (APFS) with
docs/plan/OTP11_LOCAL_SESSION.md:252:   entry, ≥3 runs, medians; unified ≤ old + 10% per cell (converge-up
docs/plan/LOCAL_ERROR_TELEMETRY.md:32:Today's "telemetry" (`perf_history.rs` → `perf_local.jsonl`, read via
docs/plan/LOCAL_ERROR_TELEMETRY.md:33:`blit diagnostics perf`) only records **successful** transfers. Its schema
docs/plan/LOCAL_ERROR_TELEMETRY.md:35:(`engine/history.rs:87-96`, `build_performance_record`) passes a literal
docs/plan/LOCAL_ERROR_TELEMETRY.md:37:`auto_tune/mod.rs`/`engine/tuning.rs`/`perf_predictor.rs` are test-only
docs/plan/LOCAL_ERROR_TELEMETRY.md:38:record constructors, not writers). Worse, `record_performance_history` is only
docs/plan/LOCAL_ERROR_TELEMETRY.md:61:  `perf_local.jsonl`, not a schema change to it.
docs/plan/LOCAL_ERROR_TELEMETRY.md:97:  opt-in** (mirroring `perf_history`'s `--enable`/`--disable`/
docs/plan/LOCAL_ERROR_TELEMETRY.md:98:  `options.perf_history` pattern) once it graduates past active development
docs/plan/LOCAL_ERROR_TELEMETRY.md:123:  trust model as `perf_local.jsonl` — this is a diagnostic log the owner
docs/plan/LOCAL_ERROR_TELEMETRY.md:132:- Local-only, on-device storage (matches `perf_local.jsonl`'s trust model —
docs/plan/LOCAL_ERROR_TELEMETRY.md:145:  `--verbose`-gated `eprintln!` convention for `perf_local.jsonl` write
docs/plan/LOCAL_ERROR_TELEMETRY.md:165:      file back, newest-first, mirroring `blit diagnostics perf`'s flag
docs/plan/LOCAL_ERROR_TELEMETRY.md:169:- [ ] `perf_local.jsonl` and its reader/predictor are completely unaffected
docs/plan/LOCAL_ERROR_TELEMETRY.md:185:New module `blit-core/src/error_history.rs`, mirroring `perf_history.rs`'s
docs/plan/LOCAL_ERROR_TELEMETRY.md:188:`config::config_dir()` as `perf_local.jsonl` — a sibling file, not a shared
docs/plan/LOCAL_ERROR_TELEMETRY.md:195:  `perf_history.rs`)
docs/plan/LOCAL_ERROR_TELEMETRY.md:250:`engine/history.rs::record_performance_history` (`history.rs:36-40`), which
docs/plan/LOCAL_ERROR_TELEMETRY.md:251:already solves this exact problem for `perf_local.jsonl` — a failed history
docs/plan/LOCAL_ERROR_TELEMETRY.md:259:`run_diagnostics_perf` in `crates/blit-cli/src/diagnostics.rs`, same flag
docs/plan/LOCAL_ERROR_TELEMETRY.md:267:   matching `perf_history.rs`'s existing tolerance).
docs/plan/OTP7_RESUME.md:132:  re-run to converge — not only as a mid-stream line that scrolls away. Small
docs/plan/OTP7_RESUME.md:189:    converge, with a test pinning that the failed path appears in the final
docs/plan/OTP7_RESUME.md:276:  No atomicity follow-up filed; convergence-on-retry is the reliability model.
     179 docs/plan/OTP12_PERF_FINDINGS.md
     1	# otp-12 perf findings — investigate + fix before otp-12c (design)
     2	
     3	**Status**: Draft (owner, 2026-07-12: "let's fix the code before
     4	devoting another block of time to testing. plan, reviewloop codex, then
     5	fix once converged" — the flip to Active happens at codex convergence
     6	per that instruction; implementation not before).
     7	**Created**: 2026-07-12
     8	**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
     9	say the quiet part: "Unification that slows the fast direction fails
    10	review." otp-12a/b measured exactly two such cells; otp-12c/12d/13 are
    11	deferred until they are fixed or explained at code level.
    12	**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
    13	if an investigation slice needs one, it stops and this doc is amended
    14	through the loop first.
    15	
    16	## The two findings (evidence, both committed)
    17	
    18	**P1 — destination-initiated TCP mixed transfers pay ~25%**
    19	(`docs/bench/otp12-win-2026-07-12/`): `wm_tcp_mixed` invariance FAIL at
    20	**1.237** (mac_init pull 1127 ms vs win_init push 911 ms, spreads
    21	8.2/3.3%), corroborated independently by block-1 `pull_tcp_mixed` new
    22	1138 vs old-same-session 867 (**1.313**). The signature is sharp:
    23	- carrier: TCP data plane only (wm_grpc_mixed = 1.013 PASS);
    24	- fixture: mixed only (512 MiB + 5k×2 KiB; large 1.023, small 1.011);
    25	- role: only when the DESTINATION end initiates (pull-verb).
    26	Also present in 12a's data? NOT testable there (review 2026-07-12):
    27	zoey's rig anchors converge-up only (12a README), so it has no
    28	mac_init/win_init invariance pair; its pull_tcp_mixed 0.966 is a
    29	new-vs-old check, not a two-layout measurement. P1 was never measured
    30	on zoey — that PASS must not be read as absence or masking evidence.
    31	
    32	**P2 — unified small-file push pays ~11–15% vs old push, both rigs**:
    33	zoey `push_tcp_small` 1.105 (RUNS=8, tight), netwatch-01 1.149 (3–4%
    34	spreads); grpc small pushes are AT parity (zoey 1.001, win 0.98-ish per
    35	cells) — so P2 is also TCP-data-plane-specific, source-initiated,
    36	10k×4 KiB. Cross-block note (12b README): block-2 `mw_tcp_small`
    37	mac_init measured 1922 vs block-1 new 2080 in the same session — the
    38	only mechanical difference is block-2's precreated destination
    39	container and per-arm path shapes; the investigation must confirm or
    40	kill that lead.
    41	
    42	## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
    43	
    44	- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
    45	  connection-initiating end DIALS; byte direction is role-set
    46	  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
    47	  session the SOURCE is the responder: each sf-2 resize epoch is
    48	  ACCEPTED off the source's listener while the DESTINATION dials
    49	  (otp-5b-2: `SourceSockets` Dial/Accept branches;
    50	  `InitiatorReceivePlaneRun.add_dialed_stream`). Mixed is the fixture
    51	  that exercises mid-transfer shape correction hardest (tar-shard small
    52	  half + big-file stream). Suspect: per-epoch accept/dial round-trips
    53	  or serialization in the accept branch that the dial branch does not
    54	  pay, surfacing only when resize fires under a fast source.
    55	- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
    56	  interleave cannot happen — resize begins only after
    57	  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
    58	  layouts drain the same fixed 128-entry destination need loop, so
    59	  batch emission cannot interleave with the resize controller during
    60	  manifest/need emission in either layout. Kept only as a residual: if
    61	  pf-1 timing shows a layout-dependent need-batch delta anyway, the
    62	  mechanism must be re-derived from the trace, not from this text.
    63	- **H3 (P2) — mechanics CORRECTED (review 2026-07-12)**: dest-side
    64	  cost in the receive path that old push didn't pay — but the listed
    65	  candidates were wrong: the small half is tar-sharded and written
    66	  with parallel per-file `create_dir_all`/`fs::write` and NO per-file
    67	  flush, and per-file progress emission to the served push destination
    68	  is disabled (`remote/transfer/sink.rs`); old push used the same
    69	  served sink. So per-file fsync/flush policy and progress emission
    70	  are NOT old/new deltas. Surviving candidates: dest-side directory
    71	  work/handle churn (the 12b cross-block 8% precreated-container lead
    72	  on NTFS) plus whatever the pf-1 trace names; zoey showing 1.105 says
    73	  the residue is not Windows-only.
    74	- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
    75	  unchanged since `0f922de` (`dial.rs`), and old small push ALSO
    76	  opened at one stream (after its 128-file early flush) then resized
    77	  live — so neither framing nor "fixed-count opening" discriminates.
    78	  What survives of H4 is ramp cadence/shard-boundary timing only, and
    79	  it is subordinate to H5.
    80	- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
    81	  scan/diff/transfer overlap on the TCP plane — current code withholds
    82	  every TCP payload until `ManifestComplete`
    83	  (`transfer_session/mod.rs`), while old push negotiated and queued
    84	  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
    85	  gRPC's in-stream carrier did not change comparably — which matches
    86	  the exact signature "TCP regressed, gRPC at parity". NOTE: an H5 fix
    87	  reorders session phases and multi-ADD/pipelined epochs conflict with
    88	  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
    89	  ordering), so any H5 fix triggers this plan's Contract
    90	  stop-and-amend rule BEFORE implementation.
    91	
    92	## Method (the investigation slice — no behavior changes)
    93	
    94	1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
    95	   in-process/two-process rigs on the Mac with the otp-2 fixture
    96	   shapes; `--trace-data-plane` + targeted `tracing` spans (added
    97	   behind a debug flag, kept) around: resize epochs (arm→accept/dial→
    98	   ack), need-batch emission times, per-file sink open/write/close in
    99	   the receive path, shard planner in/out timestamps.
   100	2. **A/B the role layouts in one process**: the role suite already
   101	   runs both initiator layouts over identical fixtures (otp-3) — but
   102	   it forces the in-stream carrier (`transfer_session_roles.rs`), so
   103	   the timing-harness variant MUST add a TCP-carrier mode; it reports
   104	   phase timings per layout for mixed and small fixtures. A positive
   105	   layout-dependent delta in a named phase confirms; local ABSENCE
   106	   does not kill H1 (loopback removes the Windows↔Mac topology) — an
   107	   H-kill needs either local reproduction or a rig-side instrumented
   108	   run.
   109	3. **Historical control, then bisect P2**: old push is deleted from
   110	   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
   111	   build and run; the control is an old-vs-new run on identical
   112	   fixtures with the same instrumentation. Experiments, corrected per
   113	   review 2026-07-12: (a) precreate-vs-not stays but is
   114	   environmental-only (it cannot attribute code); (b) the flush/
   115	   instrument toggles missed the tar-shard path — instrument the
   116	   tar-shard write path itself; (c) DROPPED — the ramp pin reproduces
   117	   the same one-stream opening old push already had, so it
   118	   discriminates nothing; (d) NEW, for H5: measure the
   119	   manifest-complete→first-TCP-payload gap new vs old (overlap
   120	   experiment); (e) per-member locking/framing timings only if the
   121	   pf-1 trace implicates them.
   122	4. Every experiment lands as a committed probe record under
   123	   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   124	   loop per slice as usual.
   125	
   126	## Fix criteria (pre-registered; the owner walks the final numbers)
   127	
   128	- Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in an
   129	  acceptance cell passes independently against BOTH its same-session
   130	  reference AND the committed baseline. The listed bars below are
   131	  necessary, not sufficient — no arm may exceed 1.10 against either
   132	  reference even when its counterpart bar passes (closes the
   133	  1.10×1.10 ≈ 1.21 hole; review 2026-07-12).
   134	- P1 fixed ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND `pull_tcp_mixed`
   135	  ≤ 1.10 against BOTH references on the netwatch-01 rig (CELLS
   136	  escalation session, RUNS=8), with `wm_grpc_mixed` and the other
   137	  invariance PASSes unregressed against both references.
   138	- P2 fixed ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
   139	  (same-session AND committed) on BOTH rigs (CELLS sessions), grpc
   140	  small parity unregressed against both.
   141	- No suite regressions; the floor is ≥ the CURRENT count (1484 —
   142	  ≥1483 would permit silently losing a test); any new pins carry
   143	  guard proofs (temporary revert) per the loop.
   144	- If investigation attributes part of a gap to something the plan's
   145	  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
   146	  that residue is RECORDED with its experiment and goes to the owner's
   147	  otp-13 walk — never silently accepted.
   148	
   149	## Staging (each through the codex loop)
   150	
   151	- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
   152	  the two-layout phase-timing report (TCP-carrier mode included) + the
   153	  `0f922de` historical control; probe record committed AND
   154	  codex-reviewed BEFORE any pf-2 branch exists. No fix lands on
   155	  pre-pf-1 evidence.
   156	- **pf-2..n**: one fix slice per confirmed root cause (smallest
   157	  change that moves the phase timing; A/B'd locally before rig time).
   158	- **pf-final**: NOT just the two escalation cells — the final build
   159	  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
   160	  gRPC controls) on BOTH rigs. No mixed-build evidence: every row
   161	  cited for acceptance comes from the final build; pre-fix PASS rows
   162	  are void for acceptance. If any shared controller/planner/sink code
   163	  changed, the gRPC control cells rerun on the final build too.
   164	  Results land in fresh dated evidence dirs; the otp-13 walk
   165	  re-verifies on the full matrix; then otp-12c proceeds on the fixed
   166	  code.
   167	
   168	## Known gaps
   169	
   170	- H1–H5 were graded against the actual tree by codex review
   171	  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
   172	  The old drivers are deleted from HEAD, but the pinned `0f922de`
   173	  source/binaries diff and run fine — historical claims get live
   174	  controls in pf-1, not pin-archaeology.
   175	- zoey never measured P1: its rig anchors converge-up only, so there
   176	  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
   177	  says nothing about layout asymmetry (review 2026-07-12). pf-1's
   178	  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
   179	  qualifies per the 12b wm numbers).

exec
/bin/zsh -lc "rg -n \"Constraints|otp-12|otp-13|Unification|converge-up|±10|10%|same-session|committed|suite|1483|wire|Phase ordering|one-token|one-ADD\" docs/plan/ONE_TRANSFER_PATH.md docs/plan/OTP12_ACCEPTANCE_RUN.md docs/TRANSFER_SESSION.md; nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '1,190p'; nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '420,570p'; nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '130,205p'; nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '340,430p'; nl -ba docs/TRANSFER_SESSION.md | sed -n '1,210p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/TRANSFER_SESSION.md:1:# TransferSession wire + session contract (otp-1)
docs/TRANSFER_SESSION.md:18:1. **One vocabulary, role-tagged.** Both wire directions carry the
docs/TRANSFER_SESSION.md:31:   integer bumped on any wire-shape change (exact match required).
docs/TRANSFER_SESSION.md:91:  |  executes them itself. No delete list crosses the wire.         |
docs/TRANSFER_SESSION.md:195:  D-2026-07-10-1), 64 MiB data plane (the wire block record bound,
docs/TRANSFER_SESSION.md:220:  crate-private, NO wire representation — a peer structurally cannot
docs/TRANSFER_SESSION.md:272:  cancel token races the session exactly as w4-3 wired, and the
docs/plan/ONE_TRANSFER_PATH.md:9:build only)** — annotated in REV4 §Constraints
docs/plan/ONE_TRANSFER_PATH.md:29:Scope, wire, and process were explicitly delegated to the agent
docs/plan/ONE_TRANSFER_PATH.md:57:  slice lands green — that is migration scaffolding, not wire
docs/plan/ONE_TRANSFER_PATH.md:75:## Constraints
docs/plan/ONE_TRANSFER_PATH.md:81:  must match the better of today's two directions (within ±10% run
docs/plan/ONE_TRANSFER_PATH.md:82:  noise), not their average. Unification that slows the fast
docs/plan/ONE_TRANSFER_PATH.md:106:- Windows parity: suite green on the owner's machine + windows-latest
docs/plan/ONE_TRANSFER_PATH.md:116:      run-to-run noise (±10%). Matrix committed as evidence.
docs/plan/ONE_TRANSFER_PATH.md:127:      + run noise (±10%). A symmetric-but-slower result fails.
docs/plan/ONE_TRANSFER_PATH.md:129:      a cell that meets per-direction converge-up and invariance but
docs/plan/ONE_TRANSFER_PATH.md:147:      (1483); all REV4 invariant pins and the sf-2 pin pass
docs/plan/ONE_TRANSFER_PATH.md:151:      explicitly-labeled wire-reference rows (never compared across
docs/plan/ONE_TRANSFER_PATH.md:153:- [ ] Windows: full suite green (owner machine) + windows-latest CI.
docs/plan/ONE_TRANSFER_PATH.md:213:session's progress events." It stays wire-compatible or is folded at
docs/plan/ONE_TRANSFER_PATH.md:223:(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
docs/plan/ONE_TRANSFER_PATH.md:228:transport (both roles in one process, no wire). The engine underneath
docs/plan/ONE_TRANSFER_PATH.md:243:faster direction — mitigated by the converge-up constraint and
docs/plan/ONE_TRANSFER_PATH.md:255:1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
docs/plan/ONE_TRANSFER_PATH.md:274:   cells, cold caches, tmpfs rows re-labeled wire-reference only —
docs/plan/ONE_TRANSFER_PATH.md:276:   rig. This is the converge-up reference the acceptance criteria
docs/plan/ONE_TRANSFER_PATH.md:281:   fixtures — the invariance property enters the test suite here.
docs/plan/ONE_TRANSFER_PATH.md:286:   equivalent) — the same code with roles flipped; the parity suite
docs/plan/ONE_TRANSFER_PATH.md:305:12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
docs/plan/ONE_TRANSFER_PATH.md:307:    AND every cell ≤ the better old direction + noise; committed as
docs/plan/ONE_TRANSFER_PATH.md:309:13. **otp-13 verdict**: acceptance checklist walked with the owner;
docs/plan/ONE_TRANSFER_PATH.md:315:- None requiring owner input now — scope, wire, and process were
docs/plan/OTP12_ACCEPTANCE_RUN.md:1:# otp-12 — symmetric-rig acceptance run (design)
docs/plan/OTP12_ACCEPTANCE_RUN.md:8:**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-12.
docs/plan/OTP12_ACCEPTANCE_RUN.md:10:and NO wire surface; it is harness scripts + rig runs + committed evidence).
docs/plan/OTP12_ACCEPTANCE_RUN.md:13:precedent, REVIEW.md §otp). The verdict WALK is otp-13 and belongs to the
docs/plan/OTP12_ACCEPTANCE_RUN.md:19:otp-12 is the plan's acceptance-evidence slice: rerun the otp-2 matrix on the
docs/plan/OTP12_ACCEPTANCE_RUN.md:30:## What otp-12 must produce (plan anchors)
docs/plan/OTP12_ACCEPTANCE_RUN.md:34:   push-verb vs pull-verb — within run noise (±10%). Committed as evidence.
docs/plan/OTP12_ACCEPTANCE_RUN.md:36:   better of that cell's two old directions + noise (±10%), against the
docs/plan/OTP12_ACCEPTANCE_RUN.md:37:   recorded old-path baselines, confirmed by interleaved same-session
docs/plan/OTP12_ACCEPTANCE_RUN.md:79:  otp-12 arm**; skippy gets fresh staging (D6).
docs/plan/OTP12_ACCEPTANCE_RUN.md:95:| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
docs/plan/OTP12_ACCEPTANCE_RUN.md:96:| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
docs/plan/OTP12_ACCEPTANCE_RUN.md:111:arm (8 timed runs per comparison). A = `old` (rig Z/W converge-up) or
docs/plan/OTP12_ACCEPTANCE_RUN.md:145:- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
docs/plan/OTP12_ACCEPTANCE_RUN.md:146:  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
docs/plan/OTP12_ACCEPTANCE_RUN.md:147:  interleaved old arm AND the committed 2026-07-10 baseline median for
docs/plan/OTP12_ACCEPTANCE_RUN.md:149:  loosened by a slower old rerun). A cell passing same-session but
docs/plan/OTP12_ACCEPTANCE_RUN.md:150:  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
docs/plan/OTP12_ACCEPTANCE_RUN.md:152:  as a recorded failure for the otp-13 walk. **Every unified arm of a
docs/plan/OTP12_ACCEPTANCE_RUN.md:166:  while passing per-direction converge-up AND invariance, the evidence
docs/plan/OTP12_ACCEPTANCE_RUN.md:176:  the otp-13 walk reviews the recorded numbers.
docs/plan/OTP12_ACCEPTANCE_RUN.md:180:interleaved in a fresh session; both sessions are committed.
docs/plan/OTP12_ACCEPTANCE_RUN.md:181:**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
docs/plan/OTP12_ACCEPTANCE_RUN.md:185:the escalation's entire purpose. The RUNS=4 rows stay committed and
docs/plan/OTP12_ACCEPTANCE_RUN.md:186:visible; the otp-13 walk sees both sessions.**
docs/plan/OTP12_ACCEPTANCE_RUN.md:276:  inside the timed window; otp-12 records it per run (`exit` column) and a
docs/plan/OTP12_ACCEPTANCE_RUN.md:290:where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
docs/plan/OTP12_ACCEPTANCE_RUN.md:291:otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
docs/plan/OTP12_ACCEPTANCE_RUN.md:305:converge row whose same-session block-1 counterpart is absent or
docs/plan/OTP12_ACCEPTANCE_RUN.md:307:artifact — the committed-reference row still governs). Nothing else is
docs/plan/OTP12_ACCEPTANCE_RUN.md:308:legal, and a missing committed-reference row aborts the verdict pass
docs/plan/OTP12_ACCEPTANCE_RUN.md:352:| Z converge-up | 12 (3 fixtures × 2 dirs × 2 carriers) | 96 | 1.5–2.5 h (drains dominate) |
docs/plan/OTP12_ACCEPTANCE_RUN.md:353:| W converge-up | 12 | 96 | ~1.5 h |
docs/plan/OTP12_ACCEPTANCE_RUN.md:363:- **otp-12a — rig Z**: `bench_otp12_zoey.sh` (harness commit; codex; fix) →
docs/plan/OTP12_ACCEPTANCE_RUN.md:368:- **otp-12b — rig W**: `bench_otp12_win.sh` covering converge-up block +
docs/plan/OTP12_ACCEPTANCE_RUN.md:372:- **otp-12c — rig D**: `bench_otp12_delegated.sh`; same shape. Preflight
docs/plan/OTP12_ACCEPTANCE_RUN.md:377:- **otp-12d — assembly**: `docs/bench/otp12-acceptance-<date>/README.md` —
docs/plan/OTP12_ACCEPTANCE_RUN.md:379:  criterion-by-criterion (the artifact otp-13 walks). Docs-only commit.
docs/plan/OTP12_ACCEPTANCE_RUN.md:381:  is the otp-13 owner walk (codex design F4; checkpoints are owner-only).
docs/plan/OTP12_ACCEPTANCE_RUN.md:415:  otp-12; the ≥1483 floor stands at 1484 from otp-11b.
docs/plan/OTP12_ACCEPTANCE_RUN.md:425:  cell; the otp-13 walk reviews the numbers, but a platform-residue cell
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
   181	   the one pipeline into the data plane; DESTINATION writes through
   182	   the one receive path. The receive sink is built with a
   183	   **runtime-selected write-strategy seam**: buffered relay is the
   184	   universal strategy; capability-gated alternatives slot in behind
   185	   it without new paths — the first is zero-copy/splice
   186	   (D-2026-07-05-3, unparked for CPU-bound receivers like the
   187	   owner's UNAS 8 Pro; design input:
   188	   `ZERO_COPY_RECEIVE_EVAL.md` §If-FAST-evidence), landing as a
   189	   follow-on slice set after cutover. Strategy selection reads
   190	   capability and payload type, never role or initiator.
   130	
   131	### D2 — verdict arithmetic (what the evidence computes; the owner declares)
   132	
   133	All statistics per the recorded baselines: integer ms; median of 4, even
   134	count = floor of the mean of the middle two; per-cell spread
   135	`(max−min)/min` recorded.
   136	
   137	**Valid-run rule (codex design F7)**: a run with a nonzero blit exit OR an
   138	undrained pre-run window VOIDS its whole interleave pair (both arms at
   139	that counterbalance position); the pair is re-run — appended at the same
   140	position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
   141	pair attempts per comparison. At the cap the cell is recorded
   142	`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
   143	a median over fewer than RUNS valid runs.
   144	
   145	- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
   146	  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
   147	  interleaved old arm AND the committed 2026-07-10 baseline median for
   148	  that cell (codex design F2: the fixed pre-cutover bar must not be
   149	  loosened by a slower old rerun). A cell passing same-session but
   150	  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
   151	  gets one pre-registered fresh-session re-run; a persisting drift stands
   152	  as a recorded failure for the otp-13 walk. **Every unified arm of a
   153	  data direction — both initiators on rig W, both blocks — must meet
   154	  these bars independently** (codex design F3: the invariance ratio is an
   155	  additional constraint, never a substitute ceiling — otherwise
   156	  tolerances compound to 1.21×).
   157	- **Invariance (rig W, hard bar — the owner's sentence)**: per fixture ×
   158	  carrier × data direction, arm A (Mac-initiated) vs arm B
   159	  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
   160	  rows; grpc rows are recorded, same bar, labeled secondary.
   161	- **Delegated parity (rig D, hard bar)**: per fixture × direction,
   162	  `max(delegated, direct)/min ≤ 1.10`.
   163	- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
   164	  each unified direction's median vs
   165	  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
   166	  while passing per-direction converge-up AND invariance, the evidence
   167	  additionally computes the **platform-residue discriminator** the otp-2w
   168	  README pre-registered: compare the old arm's direction gap
   169	  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
   170	  session. Gap unchanged ⇒ the residue exists identically without blit's
   171	  old choreography and lands on the platform write path (NTFS/Defender vs
   172	  APFS — the plan's Non-goals: different hardware need not perform
   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
   174	  README records BOTH computations per cell; a discriminator-attributed
   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
   176	  the otp-13 walk reviews the recorded numbers.
   177	
   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
   180	interleaved in a fresh session; both sessions are committed.
   181	**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
   182	text defined the trigger but not which session governs): the RUNS=8
   183	escalation session's medians govern the escalated comparison's combined
   184	outcome — more data where noise or a straddle made RUNS=4 undecidable is
   185	the escalation's entire purpose. The RUNS=4 rows stay committed and
   186	visible; the otp-13 walk sees both sessions.**
   187	
   188	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
   189	
   190	For a FIXED data direction the two initiators are:
   191	
   192	- **Mac→Windows**: arm A = Mac client pushes
   193	  (`blit copy $MAC_WORK/src_<w> $WIN_HOST:9031:/bench/<fresh>/ --yes`);
   194	  arm B = Windows client pulls
   195	  (`blit.exe copy $MAC_HOST:9031:/bench/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   196	- **Windows→Mac**: arm A = Mac client pulls (staged
   197	  `pull_src_<w>/src_<w>/` source, the otp-2w pattern); arm B = Windows
   198	  client pushes the same staged tree as a local path
   199	  (`blit.exe copy D:\blit-test\bench-module\pull_src_<w>\src_<w> $MAC_HOST:9031:/bench/<fresh>/ --yes`).
   200	
   201	New plumbing this requires, each keyed by ROLE not verb:
   202	
   203	1. **A daemon on the Mac** (new build only): config written like the rig
   204	   scripts do today (`[daemon] bind/port/no_mdns` + `[[module]] name =
   205	   "bench"` pointing at `$MAC_MODULE_ROOT`, **default `$MAC_WORK`
   340	Windows daemon-swap mechanics: the active arm's exe is COPIED to the fixed
   341	path `D:\blit-test\bins\active\blit-daemon.exe` and launched from there —
   342	one program-scoped firewall rule total (the rule is exe-path-scoped;
   343	sha-named dirs keep provenance, the copy log records each swap). Launch
   344	stays WMI `Win32_Process.Create` + stale-refusal + PID-scoped teardown
   345	(otp-2w README §Host plumbing). A staging manifest (sha256 of every binary
   346	on every host) is recorded in each evidence README.
   347	
   348	### D7 — matrix size and session budget
   349	
   350	| rig | comparisons | timed runs | est. wall |
   351	|-----|------------:|-----------:|----------:|
   352	| Z converge-up | 12 (3 fixtures × 2 dirs × 2 carriers) | 96 | 1.5–2.5 h (drains dominate) |
   353	| W converge-up | 12 | 96 | ~1.5 h |
   354	| W invariance | 12 (3 × 2 dirs × 2 carriers, new-only) | 96 | ~1.5 h |
   355	| D delegated | 6 (3 × 2 dirs, TCP) + 1 grpc smoke | 56 | ~1 h |
   356	
   357	Each rig session needs the owner's machines on and otherwise idle; sessions
   358	are independent and may run on different days (each records its own rig
   359	state).
   360	
   361	## Staging (sub-slices; each commit through the codex loop)
   362	
   363	- **otp-12a — rig Z**: `bench_otp12_zoey.sh` (harness commit; codex; fix) →
   364	  recorded run → `docs/bench/otp12-zoey-<date>/README.md` + CSVs (evidence
   365	  commit; codex; fix). Preflight gates: staged old pair present; new musl
   366	  daemon staged beside it; **fresh owner go for daemon runs on zoey**
   367	  (standing STATE rule) and zoey out of maintenance.
   368	- **otp-12b — rig W**: `bench_otp12_win.sh` covering converge-up block +
   369	  invariance block; same two-commit shape. Preflight gates: bundle
   370	  delivered + old exes copied aside + new native build (daemon + client);
   371	  Mac daemon smoke from Windows (firewall).
   372	- **otp-12c — rig D**: `bench_otp12_delegated.sh`; same shape. Preflight
   373	  gates: fresh skippy staging on the pool; `sudo -n` drop_caches on skippy;
   374	  delegation config both daemons; reachability smokes in both directions
   375	  (control port + a 1-file TCP-carrier transfer — the data plane binds
   376	  ephemeral ports, so the smoke IS the firewall test).
   377	- **otp-12d — assembly**: `docs/bench/otp12-acceptance-<date>/README.md` —
   378	  the plan-level verdict matrix assembling every comparison row
   379	  criterion-by-criterion (the artifact otp-13 walks). Docs-only commit.
   380	  The plan's acceptance-criteria checkboxes are NOT flipped here — that
   381	  is the otp-13 owner walk (codex design F4; checkpoints are owner-only).
   382	
   383	Rig order may flex with availability; 12d requires all three.
   384	
   385	## Evidence layout
   386	
   387	`docs/bench/otp12-{zoey,win,delegated}-<date>/` each carry: `README.md`
   388	(otp-2 README shape: Status/Scope, Build with all arm shas, Rig, results
   389	tables, stability, methodology deltas, reproduction), `runs.csv`,
   390	`summary.csv`, `verdicts.csv`, `drain-outcomes.txt`, `staging-manifest.txt`
   391	(sha256 per binary per host). `docs/bench/otp12-acceptance-<date>/README.md`
   392	is the assembly. Raw session logs stay under `logs/` (untracked) as usual.
   393	
   394	## Known gaps / risks
   395	
   396	- **No rig is truly fs-identical.** The plan's "symmetric rig" is
   397	  instantiated by the owner-designated closest-spec pair; rig W's two
   398	  directions still land on different OS write paths (APFS vs NTFS +
   399	  Defender at its normal state). D2's discriminator computation is the
   400	  pre-registered, evidence-backed handling; a platform-residue cell counts
   401	  as satisfied per D-2026-07-12-1.
   402	- **Old-arm provenance is a staging record, not a handshake** (old paths
   403	  predate it). Mitigated by machines.md provenance + the sha256 manifest;
   404	  accepted residual risk.
   405	- **First-of-kind surfaces**: a daemon on the Mac (application firewall
   406	  unknown until the smoke) and a client on skippy (musl-static, untested
   407	  there — the zoey zigbuild recipe retargeted). Both are preflight-gated;
   408	  failures block the affected block only.
   409	- **zoey availability**: under maintenance 2026-07-11; daemon runs there
   410	  need a fresh owner go regardless (STATE rule).
   411	- **Delegated arm includes trigger/relay overhead by design** — recorded,
   412	  expected sub-ms on this LAN; if it ever dominates a cell, that IS a
   413	  finding, not noise.
   414	- **Suite/test count**: untouched — no crates/proto changes anywhere in
   415	  otp-12; the ≥1483 floor stands at 1484 from otp-11b.
   416	
   417	## Open questions — RESOLVED (owner, 2026-07-12; D-2026-07-12-1)
   418	
   419	- **Q1 — cross-direction residue on rig W**: RESOLVED "yes" — a cell that
   420	  beats its own old direction, is initiator-invariant, and misses the
   421	  `min(old_push, old_pull) × 1.10` bar only by a discriminator-attributed
   422	  platform write-path residue (same gap in the old arm, same session)
   423	  **counts as satisfying the cross-direction half of criterion 2**
   424	  (D-2026-07-12-1). The evidence still records both computations per
   425	  cell; the otp-13 walk reviews the numbers, but a platform-residue cell
   426	  is not a blocker.
     1	# TransferSession wire + session contract (otp-1)
     2	
     3	**Status**: Active (contract; the session is the ONLY remote transfer
     4	path since cutover, otp-10c-2)
     5	**Created**: 2026-07-05
     6	**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
     7	**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
     8	(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)
     9	
    10	This document is the authoritative contract for the single `Transfer`
    11	RPC — the only byte-moving RPC since cutover (`Push` and `PullSync`
    12	were deleted whole at otp-10c-2). Proto truth lives in
    13	`proto/blit.proto` under "ONE_TRANSFER_PATH unified session"; this
    14	doc explains the state machine the proto cannot.
    15	
    16	## Invariants
    17	
    18	1. **One vocabulary, role-tagged.** Both wire directions carry the
    19	   same frame type (`TransferFrame`). Which frames an end may send is
    20	   determined by its ROLE (SOURCE or DESTINATION), never by whether
    21	   it is the gRPC client or server. This is the structural form of
    22	   the owner's invariant: there is no push-shaped or pull-shaped
    23	   message set to diverge.
    24	2. **Same build only (D-2026-07-05-2).** The first frame each way is
    25	   `SessionHello{build_id, contract_version}`. Both ends compare for
    26	   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
    27	   naming both ids, then stream close. No negotiate-down, no advisory
    28	   fields, no feature-capability bits — same build implies same
    29	   features. `build_id` = `<crate version>+<git commit hash>`
    30	   composed at compile time; `contract_version` is a belt-and-braces
    31	   integer bumped on any wire-shape change (exact match required).
    32	   Imprecise identities never false-match (otp-3 codex F1): a dirty
    33	   tree composes `<sha>.dirty.<content hash>` (deterministic — only
    34	   byte-identical dirty trees match), and a build without git
    35	   identity composes `unknown.<per-compilation entropy>` (only the
    36	   selfsame binary matches itself).
    37	3. **Roles.** The initiator (the end that opened the RPC — a CLI
    38	   client, or a daemon acting as delegated initiator) declares in
    39	   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
    40	   (always a daemon) takes the other role. All four
    41	   initiator/role combinations run the identical state machine.
    42	4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
    43	   from live enumeration (immediate start — no buffered-enumeration
    44	   phase in any direction). DESTINATION diffs incrementally against
    45	   its own filesystem and streams need batches back. DESTINATION is
    46	   authoritative for what it has; SOURCE is authoritative for what
    47	   exists to send.
    48	5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
    49	   (whichever end holds DESTINATION) advertises its
    50	   `CapacityProfile` at session open — in `SessionOpen` when the
    51	   initiator is DESTINATION, in `SessionAccept` when the responder
    52	   is. The byte SENDER (SOURCE) owns the live dial bounded by that
    53	   profile. Absent/0 profile fields mean "unknown hardware value" —
    54	   conservative defaults, never unlimited, and NEVER "old peer"
    55	   (there are no old peers).
    56	6. **One stream policy.** The data plane opens at the dial floor
    57	   immediately; SOURCE shape-corrects the stream count upward via
    58	   resize as the need list accumulates (the sf-2 mechanism —
    59	   `TransferDial::propose_shape_resize` — now the only policy).
    60	   SOURCE is the resize controller in every session.
    61	
    62	## Phase state machine
    63	
    64	```
    65	INITIATOR                                RESPONDER
    66	  |-- SessionHello ----------------------->|   (phase: HELLO)
    67	  |<------------------------ SessionHello--|
    68	  |     both verify build_id exact match; mismatch => SessionError + close
    69	  |-- SessionOpen ------------------------>|   (phase: OPEN)
    70	  |<---------------------- SessionAccept --|
    71	  |     responder validates module/path/read-only/gate here;
    72	  |     refusal is a SessionError, never a silent close
    73	  |                                        |
    74	  |==== from here the lanes are ROLES, not initiator/responder ====|
    75	  |  (whichever end holds SOURCE sends source-lane frames,          |
    76	  |   regardless of which end opened the RPC)                       |
    77	  |                                                                 |
    78	  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
    79	  |  DEST streams:    NeedBatch* ... NeedComplete                  |
    80	  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
    81	  |                   frames when the in-stream carrier is chosen) |
    82	  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
    83	  |                                                                 |
    84	  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
    85	  |  `resume=true` is followed by DEST's BlockHashList for that     |
    86	  |  file BEFORE SOURCE may send any byte of that file; stale or    |
    87	  |  mismatched partials fall back to full-file transfer.           |
    88	  |                                                                 |
    89	  |  mirror: DEST computes deletions LOCALLY from the completed     |
    90	  |  source manifest (filter-scoped, scan-complete-guarded) and     |
    91	  |  executes them itself. No delete list crosses the wire.         |
    92	  |                                                                 |
    93	  |  CLOSING (role-directed, both initiator layouts):               |
    94	  |    SOURCE -> DEST:  SourceDone (all requested payloads flushed) |
    95	  |    DEST -> SOURCE:  TransferSummary (DEST is the scorer)        |
    96	  |  then the INITIATOR closes the RPC stream.                      |
    97	```
    98	
    99	- Phase violations (a frame arriving in a phase where its role may
   100	  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
   101	  fail-fast, no tolerant parsing.
   102	- `NeedComplete` is DESTINATION's promise that no further need
   103	  batches follow (SOURCE may finish after flushing what was asked).
   104	  It may be sent only after BOTH: the source's `ManifestComplete`
   105	  has been received AND the destination has finished diffing every
   106	  received manifest entry. Mirror deletions additionally require the
   107	  scan-complete guard, as above.
   108	- **Flow control is the transport's, deliberately:** manifest, need,
   109	  and in-stream payload frames ride gRPC/HTTP-2 stream flow control;
   110	  each end holds only bounded internal queues (the engine's existing
   111	  batching — 128-entry manifest check chunks, need-list batcher).
   112	  Nothing in the contract requires unbounded buffering of the peer's
   113	  stream, and implementations must not introduce it.
   114	- `TransferSummary` always travels DESTINATION → SOURCE (the end
   115	  that wrote bytes and executed deletes is the end that can attest
   116	  to them), then the initiator surfaces it to the operator.
   117	
   118	## Frame set and field numbers
   119	
   120	`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`
   121	
   122	`TransferFrame.frame` oneof (field numbers frozen by this doc):
   123	
   124	| # | frame | sender | phase |
   125	|---|-------|--------|-------|
   126	| 1 | `SessionHello` | both, first frame | HELLO |
   127	| 2 | `SessionOpen` | initiator | OPEN |
   128	| 3 | `SessionAccept` | responder | OPEN |
   129	| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
   130	| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
   131	| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
   132	| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
   133	| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
   134	| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
   135	| 10 | `FileData file_data` | SOURCE | in-stream carrier |
   136	| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
   137	| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
   138	| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
   139	| 14 | `BlockTransfer block` | SOURCE | resume |
   140	| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
   141	| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
   142	| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
   143	| 18 | `SourceDone source_done` | SOURCE | closing |
   144	| 19 | `TransferSummary summary` | DESTINATION | closing |
   145	| 20 | `SessionError error` | both | any |
   146	
   147	Reused messages (`FileHeader`, `FileData`, `TarShard*`,
   148	`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
   149	`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
   150	`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
   151	existing shapes — the session reuses the engine's payload vocabulary
   152	verbatim. New messages (`SessionHello`, `SessionOpen`,
   153	`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
   154	`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
   155	defined in the proto with their field numbers.
   156	
   157	Deliberately absent: `PeerCapabilities` (same build = same
   158	features), `spec_version` negotiation (the hello's exact match
   159	replaces it), any delete list (mirror is destination-local), any
   160	push/pull-specific message.
   161	
   162	## Transport selection
   163	
   164	- **TCP data plane (default):** the RESPONDER binds the listener and
   165	  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
   166	  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
   167	  dials (NAT/firewall reality — connection topology, not
   168	  choreography). Byte direction on the sockets is set by role:
   169	  SOURCE writes, DESTINATION reads.
   170	  **`initial_streams` is an ACCEPT ceiling, not a dial order**
   171	  (D-2026-06-20-1/-2 preserved): it is the number of epoch-0 accept
   172	  slots the responder arms, computed as min(engine dial floor,
   173	  DESTINATION's capacity ceiling). SOURCE — wherever it sits — owns
   174	  the dial and may use fewer epoch-0 sockets than armed; unclaimed
   175	  slots expire harmlessly. Growth beyond epoch 0 happens only via
   176	  SOURCE-initiated resize (sf-2 shape correction / tuner), one armed
   177	  accept per ADD epoch, exactly as ue-r2-2 built.
   178	  **Socket auth, exact:** every epoch-0 socket opens with
   179	  `session_token` (16 bytes) immediately followed by
   180	  `epoch0_sub_token` (16 bytes); every resize-ADD socket opens with
   181	  `session_token` followed by that epoch's `sub_token` from the
   182	  `DataPlaneResize` frame. Tokens are single-session; each armed
   183	  accept slot admits exactly one socket (no replay within a
   184	  session); armed slots that go unclaimed expire, as today's resize
   185	  wiring already does. A socket presenting anything else is closed
   186	  without response.
   187	  **Resume on the data plane (otp-7b):** in a resume session, block
   188	  records ride the sockets as the binary `BLOCK`/`BLOCK_COMPLETE`
   189	  record shapes (the receive pipeline's existing tags), while the
   190	  `BlockHashList` stays a control-lane frame. All records of one
   191	  resumed file travel one socket, in order, ending with its
   192	  `BLOCK_COMPLETE` (which carries mtime+perms so zero-block resumes
   193	  still stamp metadata). The DESTINATION-chosen block size clamps to
   194	  the CARRIER's ceiling — 2 MiB in-stream (tonic frame limit,
   195	  D-2026-07-10-1), 64 MiB data plane (the wire block record bound,
   196	  D-2026-07-10-2) — with a shared 64 KiB floor and 65_536-hash list
   197	  cap; both ends read the carrier from grant presence.
   198	- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
   199	  (operator `--force-grpc` diagnostics) or granted by the responder
   200	  when it cannot bind a data plane (`SessionAccept` with no grant).
   201	  Payload frames 9-15 ride the RPC itself. Same choreography, same
   202	  planner decisions, different byte carrier.
   203	  **Record grammar (fail-fast):** payload records on the
   204	  source-lane are STRICTLY SERIALIZED — after `file_begin(header)`,
   205	  only `file_data` frames for that file may follow on the lane until
   206	  the record completes; completion is inferred at exactly
   207	  `header.size` cumulative bytes (a `file_begin`/`tar_shard_header`/
   208	  `block` arriving early, or bytes overrunning `size`, is
   209	  `PROTOCOL_VIOLATION`). Tar-shard records run
   210	  `tar_shard_header … tar_shard_chunk* … tar_shard_complete`; block

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '1,190p'; nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '245,320p'; git show --stat --oneline 9c7b00e; git show --format=fuller --no-ext-diff 9c7b00e -- docs/plan/OTP12_PERF_FINDINGS.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp-12 — symmetric-rig acceptance run (design)
     2	
     3	**Status**: Active (owner "yes to both", 2026-07-12 — the doc's only open
     4	question was ruled by D-2026-07-12-1; design codex round closed at
     5	`92e1d51`. The zoey RIG RUN still requires its own fresh owner go at run
     6	time — standing STATE rule.)
     7	**Created**: 2026-07-12
     8	**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-12.
     9	**Contract**: `docs/TRANSFER_SESSION.md` (unchanged — this slice adds NO code
    10	and NO wire surface; it is harness scripts + rig runs + committed evidence).
    11	**Governs**: execution proceeds 12a → 12b → 12c → 12d, each commit through the
    12	codex loop (D-2026-07-04-1); rig availability may reorder 12a–12c (the otp-2
    13	precedent, REVIEW.md §otp). The verdict WALK is otp-13 and belongs to the
    14	owner — this slice computes and commits the matrix; it declares nothing
    15	(Earned Practices: checkpoints are owner-only).
    16	
    17	## Why this doc
    18	
    19	otp-12 is the plan's acceptance-evidence slice: rerun the otp-2 matrix on the
    20	unified path — initiator/verb invariance A/B within noise AND every cell ≤
    21	the better old direction + noise (`ONE_TRANSFER_PATH.md` slice 12, acceptance
    22	criteria 1–2). Three rigs, three different measurement obligations, two of
    23	them first-of-kind (reverse-initiator arms; delegated remote↔remote cells).
    24	Every methodology rule below that is not new is inherited verbatim from the
    25	reviewed otp-2/otp-2w harnesses — this doc cites rather than restates their
    26	rationale (`docs/bench/otp2-baseline-2026-07-10/README.md` §Methodology
    27	findings, `docs/bench/otp2w-baseline-2026-07-10/README.md` §Timing-overhead
    28	correction).
    29	
    30	## What otp-12 must produce (plan anchors)
    31	
    32	1. **Invariance matrix** (criterion 1): per data direction × workload
    33	   (large / 10k-small / mixed), wall time initiating from end A vs end B —
    34	   push-verb vs pull-verb — within run noise (±10%). Committed as evidence.
    35	2. **Converge-up matrix** (criterion 2 / codex F4): every unified cell ≤ the
    36	   better of that cell's two old directions + noise (±10%), against the
    37	   recorded old-path baselines, confirmed by interleaved same-session
    38	   old-vs-new A/B (the otp-2 README's standing prescription for this rig
    39	   class).
    40	3. **Delegated cells** (owner rig designation, 2026-07-10, STATE Blocked):
    41	   remote↔remote on the Windows box + skippy — the delegated trigger must
    42	   not cost wall time vs the same session driven directly.
    43	
    44	## Current state (verified at HEAD `ce36da3`)
    45	
    46	Load-bearing facts, with evidence:
    47	
    48	- One `copy` verb drives everything; a remote endpoint is `host:/module/path`
    49	  or `host:port:/module/path`, default port 9031
    50	  (`crates/blit-core/src/remote/endpoint.rs:28,64-91,165-195`).
    51	- Carrier switch: default = TCP data plane (responder binds an EPHEMERAL
    52	  listener, initiator dials — `transfer_session/data_plane.rs:129,204`;
    53	  grant present ⇒ TCP, `transfer_session/mod.rs:805`); `--force-grpc`
    54	  forces the in-stream carrier (`blit-cli/src/cli.rs:317-319`), and rides
    55	  the delegated spec too (`proto/blit.proto:408`,
    56	  `blit-daemon/src/service/delegated_pull.rs:334`).
    57	- Remote↔remote is delegated-only (D-2026-07-11-1): `blit copy A:/m/p B:/m/q`
    58	  always calls `DelegatedPull` on the **destination** daemon, which initiates
    59	  the one session against the source daemon in the DESTINATION role
    60	  (`blit-app/src/transfers/remote.rs:462-484`,
    61	  `delegated_pull.rs:312-327,352`). There is no push-shaped delegated form.
    62	  The RPC carries trigger + progress only (no-payload proof recorded at
    63	  otp-10: `cli_data_plane_outbound_bytes == 0`).
    64	- Delegation gate: destination daemon config `[delegation]
    65	  allow_delegated_pull = true` + `allowed_source_hosts` allowlist
    66	  (`blit-daemon/src/runtime.rs:139-145`); per-module `delegation_allowed`.
    67	- Same-build handshake (D-2026-07-05-2): first frame both directions; exact
    68	  `build_id` + `contract_version` equality or `BuildMismatch` refusal
    69	  (`transfer_session/mod.rs:660-701`). Dirty builds mint distinct ids
    70	  (`blit-core/build.rs:28-97`) — **all arms must be clean-tree builds; arms
    71	  swap BOTH ends together (matched pairs)**.
    72	- Old-arm binaries route the OLD drivers: `e757dcc` (zoey pair, staged in
    73	  `blit-temp/` — `.agents/machines.md`) and `0f922de` (Windows pair, checkout
    74	  detached there) both PREDATE the verb cutover (`0fbc966`), so their verbs
    75	  still call `Push`/`PullSync` — they are genuine old-path arms. Verified by
    76	  ancestry + `git ls-tree` (old drivers present at both shas).
    77	- July skippy binaries (`/mnt/generic-pool/video/blit-bin/`) are REV4-era:
    78	  unknown commit, no `Transfer` RPC, no handshake — **unusable for any
    79	  otp-12 arm**; skippy gets fresh staging (D6).
    80	- Baselines on record: `docs/bench/otp2-baseline-2026-07-10/` (zoey,
    81	  per-direction only — hardware-asymmetric endpoints, D-2026-07-05-1
    82	  corollary) and `docs/bench/otp2w-baseline-2026-07-10/` (Mac↔Windows, the
    83	  owner-designated cross-direction rig).
    84	- Flags a harness touches that changed since the old scripts: none — `copy`,
    85	  `--yes`, `--force-grpc` are name-stable; `--diagnostics-counter-file` is a
    86	  global flag preceding the subcommand.
    87	- SizeMtime safe-skip delta (STATE open question) cannot affect these cells:
    88	  every timed run writes into a fresh, never-seen destination, so no
    89	  same-size/dest-newer candidates exist in any arm.
    90	
    91	## Rigs and what each anchors
    92	
    93	| rig | endpoints | anchors | why scoped so |
    94	|-----|-----------|---------|---------------|
    95	| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
    96	| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
    97	| **D** | Windows daemon ↔ skippy daemon (TrueNAS, x86_64), Mac as delegating CLI | delegated-vs-direct parity (trigger invariance) | owner-designated delegated rig; no old baseline exists on this pair |
    98	
    99	Contingency: skippy is available for Mac↔Linux cells "if needed" (owner) —
   100	used only if zoey is unavailable (it was under maintenance 2026-07-11); such
   101	a substitution records fresh baselines and is per-direction only.
   102	
   103	## Design decisions
   104	
   105	### D1 — matched-pair interleaved A/B (build identity is the axis)
   106	
   107	Each comparison interleaves arms in the deterministic counterbalanced
   108	order `A,B,B,A,A,B,B,A` (ABBA per pair-of-pairs — each arm leads half the
   109	pairs, so arm never confounds with within-pair position on the stateful
   110	rigs; pre-registered, no randomness, codex design F5) with `RUNS=4` per
   111	arm (8 timed runs per comparison). A = `old` (rig Z/W converge-up) or
   112	`delegated` (rig D). Interleaving is the verdict method, not a nicety:
   113	zoey's tiered write path never fully stops being stateful (otp-2 README
   114	§Run-to-run stability) and interleaving holds Defender state equal across
   115	arms on Windows (otp-2w README §Readings). Arm swap = stop one daemon
   116	pair, start the other (PID-scoped, stale-refusal preserved), always
   117	outside the timed window. Old arms exist only where an old baseline exists
   118	(rigs Z and W); invariance and delegated arms are new-build only — the old
   119	path is known non-invariant (the plan's founding defect) and has no
   120	delegated baseline.
   121	
   122	Build discipline: one clean commit per arm. New arm = the run commit (same
   123	sha, all hosts). Old arms = the pinned baseline shas (`e757dcc` zoey,
   124	`0f922de` Windows). Old-arm Mac clients are rebuilt at the pinned sha in a
   125	detached worktree (`git worktree add --detach` — the otp-11a precedent) and
   126	stashed at `~/blit-bench-work/bins/blit-<sha>`. The handshake enforces new-
   127	arm pair identity at the first frame; old arms predate it, so old-arm
   128	provenance rests on the staging record (`.agents/machines.md`) plus a
   129	sha256 manifest recorded in the evidence (Known gaps).
   130	
   131	### D2 — verdict arithmetic (what the evidence computes; the owner declares)
   132	
   133	All statistics per the recorded baselines: integer ms; median of 4, even
   134	count = floor of the mean of the middle two; per-cell spread
   135	`(max−min)/min` recorded.
   136	
   137	**Valid-run rule (codex design F7)**: a run with a nonzero blit exit OR an
   138	undrained pre-run window VOIDS its whole interleave pair (both arms at
   139	that counterbalance position); the pair is re-run — appended at the same
   140	position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
   141	pair attempts per comparison. At the cap the cell is recorded
   142	`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
   143	a median over fewer than RUNS valid runs.
   144	
   145	- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
   146	  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
   147	  interleaved old arm AND the committed 2026-07-10 baseline median for
   148	  that cell (codex design F2: the fixed pre-cutover bar must not be
   149	  loosened by a slower old rerun). A cell passing same-session but
   150	  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
   151	  gets one pre-registered fresh-session re-run; a persisting drift stands
   152	  as a recorded failure for the otp-13 walk. **Every unified arm of a
   153	  data direction — both initiators on rig W, both blocks — must meet
   154	  these bars independently** (codex design F3: the invariance ratio is an
   155	  additional constraint, never a substitute ceiling — otherwise
   156	  tolerances compound to 1.21×).
   157	- **Invariance (rig W, hard bar — the owner's sentence)**: per fixture ×
   158	  carrier × data direction, arm A (Mac-initiated) vs arm B
   159	  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
   160	  rows; grpc rows are recorded, same bar, labeled secondary.
   161	- **Delegated parity (rig D, hard bar)**: per fixture × direction,
   162	  `max(delegated, direct)/min ≤ 1.10`.
   163	- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
   164	  each unified direction's median vs
   165	  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
   166	  while passing per-direction converge-up AND invariance, the evidence
   167	  additionally computes the **platform-residue discriminator** the otp-2w
   168	  README pre-registered: compare the old arm's direction gap
   169	  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
   170	  session. Gap unchanged ⇒ the residue exists identically without blit's
   171	  old choreography and lands on the platform write path (NTFS/Defender vs
   172	  APFS — the plan's Non-goals: different hardware need not perform
   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
   174	  README records BOTH computations per cell; a discriminator-attributed
   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
   176	  the otp-13 walk reviews the recorded numbers.
   177	
   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
   180	interleaved in a fresh session; both sessions are committed.
   181	**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
   182	text defined the trigger but not which session governs): the RUNS=8
   183	escalation session's medians govern the escalated comparison's combined
   184	outcome — more data where noise or a straddle made RUNS=4 undecidable is
   185	the escalation's entire purpose. The RUNS=4 rows stay committed and
   186	visible; the otp-13 walk sees both sessions.**
   187	
   188	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
   189	
   190	For a FIXED data direction the two initiators are:
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
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ChBbSu2x' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-gM5M9AbL' (errno=Operation not permitted)
9c7b00e otp-12pf: address design review (7 findings) — H2 contradicted, H3 corrected, H4 narrowed, H5 added, P2 bisect redesigned with 0f922de historical control, both-reference fix bars, pf-1 hard gate + no mixed-build acceptance
 docs/plan/OTP12_PERF_FINDINGS.md | 147 ++++++++++++++++++++++++++-------------
 1 file changed, 98 insertions(+), 49 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-B2gMoIo3' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nQMuks1R' (errno=Operation not permitted)
commit 9c7b00eb60095ffb92ebf02b4441335e4a854d67
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 12 16:34:34 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 12 16:34:34 2026 -0400

    otp-12pf: address design review (7 findings) — H2 contradicted, H3 corrected, H4 narrowed, H5 added, P2 bisect redesigned with 0f922de historical control, both-reference fix bars, pf-1 hard gate + no mixed-build acceptance

diff --git a/docs/plan/OTP12_PERF_FINDINGS.md b/docs/plan/OTP12_PERF_FINDINGS.md
index b3e0704..9c77d24 100644
--- a/docs/plan/OTP12_PERF_FINDINGS.md
+++ b/docs/plan/OTP12_PERF_FINDINGS.md
@@ -23,9 +23,11 @@ through the loop first.
 - carrier: TCP data plane only (wm_grpc_mixed = 1.013 PASS);
 - fixture: mixed only (512 MiB + 5k×2 KiB; large 1.023, small 1.011);
 - role: only when the DESTINATION end initiates (pull-verb).
-Also present in 12a's data in weaker form? No — zoey pull_tcp_mixed
-PASSed (0.966) — so the cost needs the fast-NVMe/Windows-source rig or
-is masked by zoey's pool; the investigation must say which.
+Also present in 12a's data? NOT testable there (review 2026-07-12):
+zoey's rig anchors converge-up only (12a README), so it has no
+mac_init/win_init invariance pair; its pull_tcp_mixed 0.966 is a
+new-vs-old check, not a two-layout measurement. P1 was never measured
+on zoey — that PASS must not be read as absence or masking evidence.
 
 **P2 — unified small-file push pays ~11–15% vs old push, both rigs**:
 zoey `push_tcp_small` 1.105 (RUNS=8, tight), netwatch-01 1.149 (3–4%
@@ -50,24 +52,42 @@ kill that lead.
   half + big-file stream). Suspect: per-epoch accept/dial round-trips
   or serialization in the accept branch that the dial branch does not
   pay, surfacing only when resize fires under a fast source.
-- **H2 (P1)**: need-list/diff cadence differs by initiator layout for
-  the tar-shard planner — the destination diffs incrementally and
-  returns need batches; when the destination is also the session
-  initiator, batch emission may interleave differently with the resize
-  controller (controller-at-sender), delaying shard planning for the
-  small half of mixed.
-- **H3 (P2)**: per-file dest-side cost in the receive path that old
-  push didn't pay — candidate mechanics: per-file fsync/flush policy,
-  directory-handle churn, or per-file progress emission (w6-1
-  `SourceInstruments`/dest instruments) synchronous with the write
-  loop. The 12b cross-block 8% delta (precreated container) points at
-  dest-side directory work as a real component on NTFS; zoey showing
-  1.105 says the rest is not Windows-only.
-- **H4 (P2)**: tar-shard boundaries/stream ramp on the TCP plane —
-  grpc-at-parity means the in-stream carrier's shard handling is fine;
-  the TCP path's binary record framing or its dial-floor ramp
-  (`stream count corrects as the need list accumulates`) may start
-  slower than old push's fixed-count opening for 10k tiny files.
+- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
+  interleave cannot happen — resize begins only after
+  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
+  layouts drain the same fixed 128-entry destination need loop, so
+  batch emission cannot interleave with the resize controller during
+  manifest/need emission in either layout. Kept only as a residual: if
+  pf-1 timing shows a layout-dependent need-batch delta anyway, the
+  mechanism must be re-derived from the trace, not from this text.
+- **H3 (P2) — mechanics CORRECTED (review 2026-07-12)**: dest-side
+  cost in the receive path that old push didn't pay — but the listed
+  candidates were wrong: the small half is tar-sharded and written
+  with parallel per-file `create_dir_all`/`fs::write` and NO per-file
+  flush, and per-file progress emission to the served push destination
+  is disabled (`remote/transfer/sink.rs`); old push used the same
+  served sink. So per-file fsync/flush policy and progress emission
+  are NOT old/new deltas. Surviving candidates: dest-side directory
+  work/handle churn (the 12b cross-block 8% precreated-container lead
+  on NTFS) plus whatever the pf-1 trace names; zoey showing 1.105 says
+  the residue is not Windows-only.
+- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
+  unchanged since `0f922de` (`dial.rs`), and old small push ALSO
+  opened at one stream (after its 128-file early flush) then resized
+  live — so neither framing nor "fixed-count opening" discriminates.
+  What survives of H4 is ramp cadence/shard-boundary timing only, and
+  it is subordinate to H5.
+- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
+  scan/diff/transfer overlap on the TCP plane — current code withholds
+  every TCP payload until `ManifestComplete`
+  (`transfer_session/mod.rs`), while old push negotiated and queued
+  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
+  gRPC's in-stream carrier did not change comparably — which matches
+  the exact signature "TCP regressed, gRPC at parity". NOTE: an H5 fix
+  reorders session phases and multi-ADD/pipelined epochs conflict with
+  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
+  ordering), so any H5 fix triggers this plan's Contract
+  stop-and-amend rule BEFORE implementation.
 
 ## Method (the investigation slice — no behavior changes)
 
@@ -78,29 +98,48 @@ kill that lead.
    ack), need-batch emission times, per-file sink open/write/close in
    the receive path, shard planner in/out timestamps.
 2. **A/B the role layouts in one process**: the role suite already
-   runs both initiator layouts over identical fixtures (otp-3) — add a
-   timing harness variant that reports phase timings per layout for
-   mixed and small fixtures; the P1 signature must reproduce as a
-   layout-dependent delta in a named phase, or H1/H2 die.
-3. **Bisect P2 against old push mechanics**: old push is deleted, but
-   its recorded per-phase behavior (sf-2 pins, otp-2 baselines) and
-   the block-2 container lead give three testable deltas: (a)
-   precreate-vs-not (pure fs experiment on NTFS + APFS), (b) per-file
-   flush/instrument cost (toggle via debug flag), (c) ramp (fix the
-   initial stream count to old push's opening as an experiment flag).
+   runs both initiator layouts over identical fixtures (otp-3) — but
+   it forces the in-stream carrier (`transfer_session_roles.rs`), so
+   the timing-harness variant MUST add a TCP-carrier mode; it reports
+   phase timings per layout for mixed and small fixtures. A positive
+   layout-dependent delta in a named phase confirms; local ABSENCE
+   does not kill H1 (loopback removes the Windows↔Mac topology) — an
+   H-kill needs either local reproduction or a rig-side instrumented
+   run.
+3. **Historical control, then bisect P2**: old push is deleted from
+   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
+   build and run; the control is an old-vs-new run on identical
+   fixtures with the same instrumentation. Experiments, corrected per
+   review 2026-07-12: (a) precreate-vs-not stays but is
+   environmental-only (it cannot attribute code); (b) the flush/
+   instrument toggles missed the tar-shard path — instrument the
+   tar-shard write path itself; (c) DROPPED — the ramp pin reproduces
+   the same one-stream opening old push already had, so it
+   discriminates nothing; (d) NEW, for H5: measure the
+   manifest-complete→first-TCP-payload gap new vs old (overlap
+   experiment); (e) per-member locking/framing timings only if the
+   pf-1 trace implicates them.
 4. Every experiment lands as a committed probe record under
    `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
    loop per slice as usual.
 
 ## Fix criteria (pre-registered; the owner walks the final numbers)
 
+- Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in an
+  acceptance cell passes independently against BOTH its same-session
+  reference AND the committed baseline. The listed bars below are
+  necessary, not sufficient — no arm may exceed 1.10 against either
+  reference even when its counterpart bar passes (closes the
+  1.10×1.10 ≈ 1.21 hole; review 2026-07-12).
 - P1 fixed ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND `pull_tcp_mixed`
-  same-session converge ≤ 1.10 on the netwatch-01 rig (CELLS
+  ≤ 1.10 against BOTH references on the netwatch-01 rig (CELLS
   escalation session, RUNS=8), with `wm_grpc_mixed` and the other
-  invariance PASSes unregressed.
-- P2 fixed ⇔ `push_tcp_small` same-session ≤ 1.10 on BOTH rigs (CELLS
-  sessions), grpc small parity unregressed.
-- No suite regressions; the ≥1483 floor stands; any new pins carry
+  invariance PASSes unregressed against both references.
+- P2 fixed ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
+  (same-session AND committed) on BOTH rigs (CELLS sessions), grpc
+  small parity unregressed against both.
+- No suite regressions; the floor is ≥ the CURRENT count (1484 —
+  ≥1483 would permit silently losing a test); any new pins carry
   guard proofs (temporary revert) per the loop.
 - If investigation attributes part of a gap to something the plan's
   Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
@@ -109,22 +148,32 @@ kill that lead.
 
 ## Staging (each through the codex loop)
 
-- **pf-1**: instrumentation + local reproduction harness + the
-  two-layout phase-timing report; probe record committed. No fix.
+- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
+  the two-layout phase-timing report (TCP-carrier mode included) + the
+  `0f922de` historical control; probe record committed AND
+  codex-reviewed BEFORE any pf-2 branch exists. No fix lands on
+  pre-pf-1 evidence.
 - **pf-2..n**: one fix slice per confirmed root cause (smallest
   change that moves the phase timing; A/B'd locally before rig time).
-- **pf-final**: the two CELLS escalation sessions above (rig time is
-  cheap here: ~10 min each, not another full matrix); results appended
-  to the otp-12 evidence dirs; then otp-12c proceeds on the fixed
+- **pf-final**: NOT just the two escalation cells — the final build
+  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
+  gRPC controls) on BOTH rigs. No mixed-build evidence: every row
+  cited for acceptance comes from the final build; pre-fix PASS rows
+  are void for acceptance. If any shared controller/planner/sink code
+  changed, the gRPC control cells rerun on the final build too.
+  Results land in fresh dated evidence dirs; the otp-13 walk
+  re-verifies on the full matrix; then otp-12c proceeds on the fixed
   code.
 
 ## Known gaps
 
-- The hypotheses cite recorded mechanisms, not yet-verified code paths
-  (the deleted old drivers can't be diffed live; their behavior is
-  known from pins + baselines). pf-1 exists to kill or confirm them —
-  a codex reviewer should grade H1–H4 against the actual tree.
-- zoey's P1 absence (pull_tcp_mixed PASSed there) is itself evidence:
-  whatever P1 is, a slow-pool destination masks it. pf-1's local rig
-  must be fast enough to surface it (the Mac's APFS NVMe qualifies per
-  the 12b wm numbers).
+- H1–H5 were graded against the actual tree by codex review
+  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
+  The old drivers are deleted from HEAD, but the pinned `0f922de`
+  source/binaries diff and run fine — historical claims get live
+  controls in pf-1, not pin-archaeology.
+- zoey never measured P1: its rig anchors converge-up only, so there
+  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
+  says nothing about layout asymmetry (review 2026-07-12). pf-1's
+  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
+  qualifies per the 12b wm numbers).

exec
/bin/zsh -lc "rg -n \"SourceSockets|InitiatorReceivePlaneRun|add_dialed_stream|resize|ManifestComplete|MANIFEST_BATCH|NEED|NeedBatch|need_batch|TarShard|tar_shard|write_tar|create_dir_all|fs::write|flush|emit_progress|progress\" crates/blit-core/src/remote/transfer_session crates/blit-core/src/remote/transfer -g '*.rs'" in /Users/michael/Dev/blit_v2
 exited 2 in 0ms:
rg: crates/blit-core/src/remote/transfer_session: No such file or directory (os error 2)
crates/blit-core/src/remote/transfer/resume_diff.rs:164:        std::fs::write(dir.path().join("f.bin"), &content).unwrap();
crates/blit-core/src/remote/transfer/payload.rs:19:    TarShard {
crates/blit-core/src/remote/transfer/payload.rs:54:        TransferPayload::TarShard { headers } => {
crates/blit-core/src/remote/transfer/payload.rs:58:                task::spawn_blocking(move || build_tar_shard(&source_root_clone, &headers_clone))
crates/blit-core/src/remote/transfer/payload.rs:61:            Ok(PreparedPayload::TarShard { headers, data })
crates/blit-core/src/remote/transfer/payload.rs:86:/// `File` and `TarShard` are used by both outbound and inbound paths
crates/blit-core/src/remote/transfer/payload.rs:101:    TarShard {
crates/blit-core/src/remote/transfer/payload.rs:168:            TransferTask::TarShard(paths) => {
crates/blit-core/src/remote/transfer/payload.rs:177:                    payloads.push(TransferPayload::TarShard {
crates/blit-core/src/remote/transfer/payload.rs:209:        TransferPayload::TarShard { .. } => (0, 0),
crates/blit-core/src/remote/transfer/payload.rs:224:            TransferPayload::TarShard { headers } => headers.len(),
crates/blit-core/src/remote/transfer/payload.rs:254:pub fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
crates/blit-core/src/remote/transfer/pipeline.rs:15:use super::progress::RemoteTransferProgress;
crates/blit-core/src/remote/transfer/pipeline.rs:29:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/pipeline.rs:55:    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
crates/blit-core/src/remote/transfer/pipeline.rs:82:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/pipeline.rs:84:    execute_sink_pipeline_elastic(source, sinks, payload_rx, prefetch, progress, None).await
crates/blit-core/src/remote/transfer/pipeline.rs:87:/// Control commands for a RUNNING pipeline (`ue-r2-2` stream resize).
crates/blit-core/src/remote/transfer/pipeline.rs:115:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/pipeline.rs:161:        progress: Option<RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/pipeline.rs:203:                        PreparedPayload::TarShard { headers, .. } => headers
crates/blit-core/src/remote/transfer/pipeline.rs:227:                    if let Some(p) = &progress {
crates/blit-core/src/remote/transfer/pipeline.rs:228:                        // Contract (progress.rs): bytes ride Payload, one
crates/blit-core/src/remote/transfer/pipeline.rs:271:            progress.cloned(),
crates/blit-core/src/remote/transfer/pipeline.rs:307:    // resize control channel. `join_next() == None` means every worker
crates/blit-core/src/remote/transfer/pipeline.rs:343:                                progress.cloned(),
crates/blit-core/src/remote/transfer/pipeline.rs:425:/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
crates/blit-core/src/remote/transfer/pipeline.rs:438:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/pipeline.rs:468:                if let Some(p) = progress {
crates/blit-core/src/remote/transfer/pipeline.rs:475:                let (headers, data) = read_tar_shard(socket).await?;
crates/blit-core/src/remote/transfer/pipeline.rs:481:                    progress.map(|_| headers.iter().map(|h| h.relative_path.clone()).collect());
crates/blit-core/src/remote/transfer/pipeline.rs:482:                let payload = PreparedPayload::TarShard { headers, data };
crates/blit-core/src/remote/transfer/pipeline.rs:487:                if let Some(p) = progress {
crates/blit-core/src/remote/transfer/pipeline.rs:533:                if let Some(p) = progress {
crates/blit-core/src/remote/transfer/pipeline.rs:556:                let path_for_progress = progress.map(|_| path.clone());
crates/blit-core/src/remote/transfer/pipeline.rs:567:                if let Some(p) = progress {
crates/blit-core/src/remote/transfer/pipeline.rs:568:                    p.report_file_complete(path_for_progress.unwrap_or_default());
crates/blit-core/src/remote/transfer/pipeline.rs:653:async fn read_tar_shard<R: AsyncRead + Unpin>(
crates/blit-core/src/remote/transfer/pipeline.rs:733:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:735:        std::fs::write(src.join("a.txt"), b"alpha").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:736:        std::fs::write(src.join("b.txt"), b"bravo").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:737:        std::fs::create_dir_all(src.join("sub")).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:738:        std::fs::write(src.join("sub/c.txt"), b"charlie").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:783:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:785:            std::fs::write(src.join(format!("f{i}.txt")), format!("content-{i}")).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:846:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:848:            std::fs::write(src.join(format!("f{i}.txt")), format!("n{i}")).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:932:            ("tar shard with zero entries", encode_tar_shard(&[], 0, &[])),
crates/blit-core/src/remote/transfer/pipeline.rs:935:                encode_tar_shard(&[("f.txt", 5, 1_600_000_000, 0o644)], 5, &[0u8; 5]),
crates/blit-core/src/remote/transfer/pipeline.rs:1090:    fn encode_tar_shard(
crates/blit-core/src/remote/transfer/pipeline.rs:1136:    // (progress.rs): bytes ride Payload only; FileComplete is byteless
crates/blit-core/src/remote/transfer/pipeline.rs:1140:    use crate::remote::transfer::progress::{
crates/blit-core/src/remote/transfer/pipeline.rs:1156:                PreparedPayload::TarShard { headers, data } => (headers.len(), data.len() as u64),
crates/blit-core/src/remote/transfer/pipeline.rs:1215:        let (sink, progress, mut rx) = recording_receive_setup();
crates/blit-core/src/remote/transfer/pipeline.rs:1217:        let outcome = execute_receive_pipeline(&mut reader, sink, Some(&progress))
crates/blit-core/src/remote/transfer/pipeline.rs:1221:        drop(progress);
crates/blit-core/src/remote/transfer/pipeline.rs:1247:    async fn receive_pipeline_tar_shard_counts_member_files() {
crates/blit-core/src/remote/transfer/pipeline.rs:1248:        let mut wire = encode_tar_shard(
crates/blit-core/src/remote/transfer/pipeline.rs:1255:        let (sink, progress, mut rx) = recording_receive_setup();
crates/blit-core/src/remote/transfer/pipeline.rs:1257:        execute_receive_pipeline(&mut reader, sink, Some(&progress))
crates/blit-core/src/remote/transfer/pipeline.rs:1260:        drop(progress);
crates/blit-core/src/remote/transfer/pipeline.rs:1282:    async fn receive_pipeline_resume_records_report_progress() {
crates/blit-core/src/remote/transfer/pipeline.rs:1287:        let (sink, progress, mut rx) = recording_receive_setup();
crates/blit-core/src/remote/transfer/pipeline.rs:1289:        execute_receive_pipeline(&mut reader, sink, Some(&progress))
crates/blit-core/src/remote/transfer/pipeline.rs:1292:        drop(progress);
crates/blit-core/src/remote/transfer/pipeline.rs:1317:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1318:        std::fs::write(src.join("a.txt"), b"alpha").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1319:        std::fs::write(src.join("b.txt"), b"bravo").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1320:        std::fs::write(src.join("c.txt"), b"charlie").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1350:        let progress = RemoteTransferProgress::new(tx);
crates/blit-core/src/remote/transfer/pipeline.rs:1351:        execute_sink_pipeline(source, vec![sink], planned, 4, Some(&progress))
crates/blit-core/src/remote/transfer/pipeline.rs:1354:        drop(progress);
crates/blit-core/src/remote/transfer/pipeline.rs:1384:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1385:        std::fs::write(src.join("a.txt"), b"alpha").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1521:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1524:            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1640:        std::fs::create_dir_all(src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1642:            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1777:        assert_eq!(kept + retired, n as u64, "exactly-once across the resize");
crates/blit-core/src/remote/transfer/pipeline.rs:1901:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1904:            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:1966:                PreparedPayload::TarShard { headers, .. } => {
crates/blit-core/src/remote/transfer/pipeline.rs:1993:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:2000:            std::fs::write(src.join(format!("f{i}.dat")), &body).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:2071:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:2074:            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:2131:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/pipeline.rs:2134:            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
crates/blit-core/src/remote/transfer/tar_safety.rs:5://!   - `crates/blit-core/src/remote/pull.rs::apply_pull_tar_shard`
crates/blit-core/src/remote/transfer/tar_safety.rs:7://!   - `crates/blit-core/src/remote/transfer/sink.rs::write_tar_shard_payload`
crates/blit-core/src/remote/transfer/tar_safety.rs:9://!   - `crates/blit-daemon/src/service/push/data_plane.rs::apply_tar_shard_sync`
crates/blit-core/src/remote/transfer/tar_safety.rs:52:/// Tunable knobs for `safe_extract_tar_shard`.
crates/blit-core/src/remote/transfer/tar_safety.rs:54:pub struct TarShardExtractOptions {
crates/blit-core/src/remote/transfer/tar_safety.rs:67:impl Default for TarShardExtractOptions {
crates/blit-core/src/remote/transfer/tar_safety.rs:104:pub fn safe_extract_tar_shard(
crates/blit-core/src/remote/transfer/tar_safety.rs:108:    options: &TarShardExtractOptions,
crates/blit-core/src/remote/transfer/tar_safety.rs:228:        std::fs::create_dir_all(parent)
crates/blit-core/src/remote/transfer/tar_safety.rs:231:    std::fs::write(&file.dest_path, &file.contents)
crates/blit-core/src/remote/transfer/tar_safety.rs:302:        let err = safe_extract_tar_shard(
crates/blit-core/src/remote/transfer/tar_safety.rs:306:            &TarShardExtractOptions::default(),
crates/blit-core/src/remote/transfer/tar_safety.rs:316:        let err = safe_extract_tar_shard(
crates/blit-core/src/remote/transfer/tar_safety.rs:320:            &TarShardExtractOptions::default(),
crates/blit-core/src/remote/transfer/tar_safety.rs:330:        let opts = TarShardExtractOptions {
crates/blit-core/src/remote/transfer/tar_safety.rs:335:            safe_extract_tar_shard(&buffer, vec![fh("big.txt", 2)], tmp.path(), &opts).unwrap_err();
crates/blit-core/src/remote/transfer/tar_safety.rs:343:        let err = safe_extract_tar_shard(
crates/blit-core/src/remote/transfer/tar_safety.rs:347:            &TarShardExtractOptions::default(),
crates/blit-core/src/remote/transfer/tar_safety.rs:357:        let err = safe_extract_tar_shard(
crates/blit-core/src/remote/transfer/tar_safety.rs:361:            &TarShardExtractOptions::default(),
crates/blit-core/src/remote/transfer/tar_safety.rs:375:        let extracted = safe_extract_tar_shard(
crates/blit-core/src/remote/transfer/tar_safety.rs:379:            &TarShardExtractOptions::default(),
crates/blit-core/src/remote/transfer/tar_safety.rs:439:        std::fs::create_dir_all(&dest).unwrap();
crates/blit-core/src/remote/transfer/tar_safety.rs:440:        let err = safe_extract_tar_shard(
crates/blit-core/src/remote/transfer/tar_safety.rs:444:            &TarShardExtractOptions::default(),
crates/blit-core/src/remote/transfer/socket.rs:4://! connect, and all daemon accept paths (push epoch-0/resize,
crates/blit-core/src/remote/transfer/socket.rs:5://! pull_sync epoch-0/resize/resume) — routes through
crates/blit-core/src/remote/transfer/socket.rs:48:/// (an armed resize slot, a stream waiting for work while siblings
crates/blit-core/src/remote/transfer/socket.rs:78:///   defaults; resize-ADD sockets get the ramped size), and `None`
crates/blit-core/src/remote/transfer/progress.rs:5:/// One progress observation from a transfer producer.
crates/blit-core/src/remote/transfer/progress.rs:50:/// accumulator (w6-1) — the CLI progress monitor and all three TUI
crates/blit-core/src/remote/transfer/progress.rs:88:/// Cumulative byte-progress reporter for data-plane write loops.
crates/blit-core/src/remote/transfer/progress.rs:145:mod progress_totals_tests {
crates/blit-core/src/remote/transfer/progress.rs:364:    /// Bumped each time the controller resizes; lets a stale snapshot be
crates/blit-core/src/remote/transfer/progress.rs:365:    /// discarded across a resize boundary.
crates/blit-core/src/remote/transfer/progress.rs:401:/// same way it carries byte progress.
crates/blit-core/src/remote/transfer/mod.rs:8:pub mod progress;
crates/blit-core/src/remote/transfer/mod.rs:26:    build_tar_shard, payload_file_count, plan_transfer_payloads, prepare_payload,
crates/blit-core/src/remote/transfer/mod.rs:35:pub use progress::{
crates/blit-core/src/remote/transfer/data_plane.rs:10:use super::progress::{NoProbe, Probe};
crates/blit-core/src/remote/transfer/data_plane.rs:22:/// ue-r2-2: length of the per-epoch resize credential a data socket
crates/blit-core/src/remote/transfer/data_plane.rs:23:/// echoes after the one-time token when resize was negotiated
crates/blit-core/src/remote/transfer/data_plane.rs:28:/// Generate one 16-byte resize sub-token. Same fallible-RNG posture
crates/blit-core/src/remote/transfer/data_plane.rs:53:/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
crates/blit-core/src/remote/transfer/data_plane.rs:55:/// (15+ minutes). All existing `self.stream.write_all/.flush` call
crates/blit-core/src/remote/transfer/data_plane.rs:83:    /// observable write progress instead of pinning the worker for
crates/blit-core/src/remote/transfer/data_plane.rs:191:        self.send_payloads_with_progress(source, payloads, None)
crates/blit-core/src/remote/transfer/data_plane.rs:195:    pub async fn send_payloads_with_progress(
crates/blit-core/src/remote/transfer/data_plane.rs:199:        progress: Option<&super::progress::RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/data_plane.rs:209:                    if let Some(progress) = progress {
crates/blit-core/src/remote/transfer/data_plane.rs:210:                        progress.report_payload(0, header.size);
crates/blit-core/src/remote/transfer/data_plane.rs:211:                        progress.report_file_complete(header.relative_path.clone());
crates/blit-core/src/remote/transfer/data_plane.rs:214:                PreparedPayload::TarShard { headers, data } => {
crates/blit-core/src/remote/transfer/data_plane.rs:216:                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
crates/blit-core/src/remote/transfer/data_plane.rs:220:                    if let Some(progress) = progress {
crates/blit-core/src/remote/transfer/data_plane.rs:222:                            progress.report_payload(0, header.size);
crates/blit-core/src/remote/transfer/data_plane.rs:223:                            progress.report_file_complete(header.relative_path.clone());
crates/blit-core/src/remote/transfer/data_plane.rs:244:            .flush()
crates/blit-core/src/remote/transfer/data_plane.rs:246:            .context("flushing data plane stream")
crates/blit-core/src/remote/transfer/data_plane.rs:446:    pub async fn send_prepared_tar_shard(
crates/blit-core/src/remote/transfer/data_plane.rs:671:/// `byte_progress` (optional) gets a `report(delta)` call after
crates/blit-core/src/remote/transfer/data_plane.rs:683:    byte_progress: Option<&crate::remote::transfer::progress::ByteProgressSink>,
crates/blit-core/src/remote/transfer/data_plane.rs:716:        if let Some(progress) = byte_progress {
crates/blit-core/src/remote/transfer/data_plane.rs:717:            progress.report(bytes_a as u64);
crates/blit-core/src/remote/transfer/data_plane.rs:736:        if let Some(progress) = byte_progress {
crates/blit-core/src/remote/transfer/data_plane.rs:737:            progress.report(bytes_a as u64);
crates/blit-core/src/remote/transfer/data_plane.rs:762:mod byte_progress_tests {
crates/blit-core/src/remote/transfer/data_plane.rs:764:    use crate::remote::transfer::progress::ByteProgressSink;
crates/blit-core/src/remote/transfer/data_plane.rs:768:    /// With `byte_progress = None` the function behaves exactly
crates/blit-core/src/remote/transfer/data_plane.rs:772:    async fn copies_without_progress_when_sink_omitted() {
crates/blit-core/src/remote/transfer/data_plane.rs:813:    /// final batch — proves the progress hook is INSIDE the loop,
crates/blit-core/src/remote/transfer/source.rs:176:                    // R46-F4: progress to stderr, never stdout — the
crates/blit-core/src/remote/transfer/stall_guard.rs:3://! `io::ErrorKind::TimedOut`, while leaving a steadily-progressing
crates/blit-core/src/remote/transfer/stall_guard.rs:12://! it is an **idle** timeout (re-armed on every read that makes progress)
crates/blit-core/src/remote/transfer/stall_guard.rs:14://! keeps making progress is never aborted. (Owner decision, memory
crates/blit-core/src/remote/transfer/stall_guard.rs:23://!   adapter mirroring [`StallGuard`] for **write** progress. The
crates/blit-core/src/remote/transfer/stall_guard.rs:28://!   progress, with the same idle-vs-total-deadline semantics as the
crates/blit-core/src/remote/transfer/stall_guard.rs:33://!   the missing guard is daemon pull-data-plane **write progress
crates/blit-core/src/remote/transfer/stall_guard.rs:41://!   2 pending** (dynamic progress watchdog + retryable `TimedOut`
crates/blit-core/src/remote/transfer/stall_guard.rs:54:/// data-plane progress (read or write) is observable for this long, the
crates/blit-core/src/remote/transfer/stall_guard.rs:61:/// - Daemon pull-data-plane **write progress after token acceptance**
crates/blit-core/src/remote/transfer/stall_guard.rs:72:/// Wraps an `AsyncRead` so a read that makes no progress within `timeout`
crates/blit-core/src/remote/transfer/stall_guard.rs:102:                // that's progress, so re-arm the idle deadline.
crates/blit-core/src/remote/transfer/stall_guard.rs:109:                // window has elapsed since the last progress; otherwise
crates/blit-core/src/remote/transfer/stall_guard.rs:123:/// Wraps an `AsyncWrite` so a write that makes no progress within `timeout`
crates/blit-core/src/remote/transfer/stall_guard.rs:125:/// successful `poll_write` (any byte count > 0 counts as progress), so it
crates/blit-core/src/remote/transfer/stall_guard.rs:134:/// observable write progress.
crates/blit-core/src/remote/transfer/stall_guard.rs:138:/// progressing transfer (any non-trivial network at all) is never
crates/blit-core/src/remote/transfer/stall_guard.rs:169:                // Per the doc contract above, "no progress" means zero
crates/blit-core/src/remote/transfer/stall_guard.rs:174:                // progress doesn't show up within the window the
crates/blit-core/src/remote/transfer/stall_guard.rs:180:                // n > 0: real progress. Reset the idle deadline so a
crates/blit-core/src/remote/transfer/stall_guard.rs:181:                // steadily-progressing transfer is never aborted.
crates/blit-core/src/remote/transfer/stall_guard.rs:190:                // progress; otherwise stay pending (the deadline poll
crates/blit-core/src/remote/transfer/stall_guard.rs:195:                        format!("transfer stalled: no write progress for {:?}", this.timeout),
crates/blit-core/src/remote/transfer/stall_guard.rs:203:    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
crates/blit-core/src/remote/transfer/stall_guard.rs:205:        // on the deadline because a stuck flush manifests as a stuck
crates/blit-core/src/remote/transfer/stall_guard.rs:207:        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
crates/blit-core/src/remote/transfer/stall_guard.rs:279:    /// write progress is observable. We simulate this with a duplex
crates/blit-core/src/remote/transfer/stall_guard.rs:302:    /// audit-h3b: an actively-draining peer keeps writes progressing,
crates/blit-core/src/remote/transfer/sink.rs:19:use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
crates/blit-core/src/remote/transfer/sink.rs:126:    /// Optional byte-level progress sink. When set,
crates/blit-core/src/remote/transfer/sink.rs:129:    /// writes report cumulative byte progress against the
crates/blit-core/src/remote/transfer/sink.rs:132:    /// [`FsTransferSink::with_byte_progress`] from
crates/blit-core/src/remote/transfer/sink.rs:134:    byte_progress: Option<ByteProgressSink>,
crates/blit-core/src/remote/transfer/sink.rs:153:            byte_progress: None,
crates/blit-core/src/remote/transfer/sink.rs:157:    /// Attach a byte-level progress sink. When set,
crates/blit-core/src/remote/transfer/sink.rs:161:    /// tracks live progress; CLI-side callers omit it.
crates/blit-core/src/remote/transfer/sink.rs:162:    pub fn with_byte_progress(mut self, sink: ByteProgressSink) -> Self {
crates/blit-core/src/remote/transfer/sink.rs:163:        self.byte_progress = Some(sink);
crates/blit-core/src/remote/transfer/sink.rs:196:        // through tokio). Local-source payloads (File / TarShard) stay
crates/blit-core/src/remote/transfer/sink.rs:237:            PreparedPayload::File(_) | PreparedPayload::TarShard { .. } => {
crates/blit-core/src/remote/transfer/sink.rs:250:                    PreparedPayload::TarShard { headers, data } => write_tar_shard_payload(
crates/blit-core/src/remote/transfer/sink.rs:257:                    _ => unreachable!("outer match guarantees File or TarShard"),
crates/blit-core/src/remote/transfer/sink.rs:272:        // `write_tar_shard_payload`'s dry-run early returns), so
crates/blit-core/src/remote/transfer/sink.rs:275:        if let Some(bp) = &self.byte_progress {
crates/blit-core/src/remote/transfer/sink.rs:312:            // Do NOT report against `byte_progress` — by contract
crates/blit-core/src/remote/transfer/sink.rs:332:            tokio::fs::create_dir_all(parent)
crates/blit-core/src/remote/transfer/sink.rs:347:                self.byte_progress.as_ref(),
crates/blit-core/src/remote/transfer/sink.rs:358:            // POST_REVIEW_FIXES §1.1: flush failure is a data-loss
crates/blit-core/src/remote/transfer/sink.rs:361:            file.flush()
crates/blit-core/src/remote/transfer/sink.rs:363:                .with_context(|| format!("flushing {}", dst.display()))?;
crates/blit-core/src/remote/transfer/sink.rs:371:        // is its END marker plus the OS's own flush; matches rsync's
crates/blit-core/src/remote/transfer/sink.rs:490:        std::fs::create_dir_all(parent)
crates/blit-core/src/remote/transfer/sink.rs:528:fn write_tar_shard_payload(
crates/blit-core/src/remote/transfer/sink.rs:556:    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};
crates/blit-core/src/remote/transfer/sink.rs:558:    let opts = TarShardExtractOptions::default();
crates/blit-core/src/remote/transfer/sink.rs:559:    let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;
crates/blit-core/src/remote/transfer/sink.rs:563:    // lexical safe_join inside safe_extract_tar_shard. A pre-
crates/blit-core/src/remote/transfer/sink.rs:576:            "write_tar_shard_payload at '{}' has no canonical root; \
crates/blit-core/src/remote/transfer/sink.rs:592:    // Write in parallel. Each closure does its own create_dir_all +
crates/blit-core/src/remote/transfer/sink.rs:593:    // fs::write + best-effort mtime/permission application — same
crates/blit-core/src/remote/transfer/sink.rs:600:                std::fs::create_dir_all(parent)
crates/blit-core/src/remote/transfer/sink.rs:603:            std::fs::write(&f.dest_path, &f.contents)
crates/blit-core/src/remote/transfer/sink.rs:674:    // bytes reached the OS. Without this flush an acknowledged block can
crates/blit-core/src/remote/transfer/sink.rs:680:    file.flush()
crates/blit-core/src/remote/transfer/sink.rs:682:        .with_context(|| format!("flushing block write to {}", dst.display()))?;
crates/blit-core/src/remote/transfer/sink.rs:799:            PreparedPayload::TarShard { headers, data } => {
crates/blit-core/src/remote/transfer/sink.rs:803:                    .send_prepared_tar_shard(headers, &data)
crates/blit-core/src/remote/transfer/sink.rs:952:            PreparedPayload::TarShard { headers, data } => Ok(SinkOutcome {
crates/blit-core/src/remote/transfer/sink.rs:982:        // not advance a daemon-side progress counter for these
crates/blit-core/src/remote/transfer/sink.rs:1030:        std::fs::write(&src, b"root payload").unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1043:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1044:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1047:        std::fs::write(src.join("file.txt"), content).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1077:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1078:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1080:        std::fs::write(src.join("file.txt"), b"data").unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1114:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1115:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1116:        std::fs::create_dir_all(src.join("sub")).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1117:        std::fs::write(src.join("sub/file.txt"), b"data").unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1146:    /// receive — the pre-fix create_dir_all ran above the dry-run
crates/blit-core/src/remote/transfer/sink.rs:1153:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1154:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1185:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1186:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1189:        std::fs::write(src.join("same.txt"), content).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1190:        std::fs::write(dst.join("same.txt"), content).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1215:    async fn fs_sink_extracts_tar_shard() {
crates/blit-core/src/remote/transfer/sink.rs:1218:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1262:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1279:        std::fs::create_dir_all(src.join("a/b/c")).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1282:        std::fs::write(src.join("a/b/c/deep.txt"), content).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1318:    async fn null_sink_counts_tar_shard() {
crates/blit-core/src/remote/transfer/sink.rs:1328:            .write_payload(PreparedPayload::TarShard { headers, data })
crates/blit-core/src/remote/transfer/sink.rs:1354:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1355:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1428:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1429:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1463:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1508:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1509:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1510:        std::fs::create_dir_all(&outside).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1564:        std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1565:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1566:        std::fs::create_dir_all(&outside).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1568:        std::fs::write(src_root.join("link/victim.txt"), b"payload").ok();
crates/blit-core/src/remote/transfer/sink.rs:1569:        std::fs::create_dir_all(src_root.join("link")).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1570:        std::fs::write(src_root.join("link/victim.txt"), b"payload").unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1605:    /// `PreparedPayload::TarShard` must reject any extracted entry
crates/blit-core/src/remote/transfer/sink.rs:1607:    /// existing dst escape symlink. Pre-fix `write_tar_shard_payload`
crates/blit-core/src/remote/transfer/sink.rs:1608:    /// used `safe_extract_tar_shard` which does lexical
crates/blit-core/src/remote/transfer/sink.rs:1614:    async fn fs_sink_write_payload_tar_shard_rejects_escape() {
crates/blit-core/src/remote/transfer/sink.rs:1622:        std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1623:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1624:        std::fs::create_dir_all(&outside).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1658:        let payload = PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1683:    async fn write_payload_reports_tar_shard_bytes_against_byte_progress() {
crates/blit-core/src/remote/transfer/sink.rs:1686:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1711:        let byte_progress = ByteProgressSink::new();
crates/blit-core/src/remote/transfer/sink.rs:1717:        let sink_progress = ByteProgressSink::from_counter(std::sync::Arc::clone(&probe_counter));
crates/blit-core/src/remote/transfer/sink.rs:1718:        let _ = byte_progress; // keep `new()` covered too
crates/blit-core/src/remote/transfer/sink.rs:1731:        .with_byte_progress(sink_progress);
crates/blit-core/src/remote/transfer/sink.rs:1734:            .write_payload(PreparedPayload::TarShard {
crates/blit-core/src/remote/transfer/sink.rs:1747:            "tar shard byte progress must equal outcome.bytes_written"
crates/blit-core/src/remote/transfer/sink.rs:1756:    async fn write_payload_reports_file_block_bytes_against_byte_progress() {
crates/blit-core/src/remote/transfer/sink.rs:1759:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1762:        std::fs::write(dst.join("resume.bin"), vec![0u8; 64]).unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1765:        let sink_progress = ByteProgressSink::from_counter(std::sync::Arc::clone(&probe_counter));
crates/blit-core/src/remote/transfer/sink.rs:1778:        .with_byte_progress(sink_progress);
crates/blit-core/src/remote/transfer/sink.rs:1796:            "FileBlock byte progress must equal outcome.bytes_written"
crates/blit-core/src/remote/transfer/diff_planner.rs:4://! payloads (whole-file `File`, batched `TarShard`) via
crates/blit-core/src/remote/transfer/diff_planner.rs:65:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:66:        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:70:                std::fs::create_dir_all(parent).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:72:            std::fs::write(full, content).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:77:                std::fs::create_dir_all(parent).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:79:            std::fs::write(full, content).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:225:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:226:        std::fs::write(src.join("a.txt"), b"x").unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:227:        std::fs::write(src.join("b.txt"), b"y").unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:238:    fn planner_batches_many_small_files_into_tar_shard() {
crates/blit-core/src/remote/transfer/diff_planner.rs:240:        // least one TarShard payload. Only "some tar shard exists" is
crates/blit-core/src/remote/transfer/diff_planner.rs:244:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:248:            std::fs::write(src.join(&name), b"tiny").unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:252:        let tar_shards = planned
crates/blit-core/src/remote/transfer/diff_planner.rs:254:            .filter(|p| matches!(p, TransferPayload::TarShard { .. }))
crates/blit-core/src/remote/transfer/diff_planner.rs:257:            tar_shards >= 1,
crates/blit-core/src/remote/transfer/diff_planner.rs:258:            "expected at least one TarShard payload for 50 small files, got {} payloads: {:?}",
crates/blit-core/src/remote/transfer/diff_planner.rs:268:        std::fs::create_dir_all(&src).unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:271:            std::fs::write(src.join(&name), b"x").unwrap();
crates/blit-core/src/remote/transfer/diff_planner.rs:285:            .any(|p| matches!(p, TransferPayload::TarShard { .. }));
crates/blit-core/src/remote/transfer/diff_planner.rs:286:        assert!(has_tar, "force_tar must produce a TarShard payload");
crates/blit-core/src/remote/transfer/session_client.rs:47:/// since otp-10a (mirror, filters, progress, trace); the SOURCE owns
crates/blit-core/src/remote/transfer/session_client.rs:81:    /// otp-10a: w6-1 progress events from this SOURCE's send side —
crates/blit-core/src/remote/transfer/session_client.rs:83:    /// file sent on either carrier. The CLI progress line and the TUI
crates/blit-core/src/remote/transfer/session_client.rs:85:    pub progress: Option<RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/session_client.rs:104:            progress: None,
crates/blit-core/src/remote/transfer/session_client.rs:177:            progress: options.progress,
crates/blit-core/src/remote/transfer/session_client.rs:236:    pub byte_progress: Option<ByteProgressSink>,
crates/blit-core/src/remote/transfer/session_client.rs:237:    /// otp-10b-2: w6-1 progress events from this DESTINATION's receive
crates/blit-core/src/remote/transfer/session_client.rs:239:    /// per record received on either carrier. The CLI progress line and
crates/blit-core/src/remote/transfer/session_client.rs:241:    /// driver. Symmetric with [`PushSessionOptions::progress`].
crates/blit-core/src/remote/transfer/session_client.rs:242:    pub progress: Option<RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/session_client.rs:260:            byte_progress: None,
crates/blit-core/src/remote/transfer/session_client.rs:261:            progress: None,
crates/blit-core/src/remote/transfer/session_client.rs:339:            progress: options.progress,
crates/blit-core/src/remote/transfer/session_client.rs:340:            byte_progress: options.byte_progress,

exec
/bin/zsh -lc "rg --files crates/blit-core/src | rg 'transfer_session|session|data_plane'; rg -n \"SourceSockets|InitiatorReceivePlaneRun|add_dialed_stream|ManifestComplete|DataPlaneResize|propose_shape_resize|NEED_BATCH|128\" crates/blit-core/src crates/blit-daemon/src -g '*.rs'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/data_plane.rs
crates/blit-core/src/remote/transfer/session_client.rs
crates/blit-core/src/transfer_session/mod.rs
crates/blit-core/src/transfer_session/data_plane.rs
crates/blit-core/src/transfer_session/transport.rs
crates/blit-core/src/transfer_session/local.rs
crates/blit-daemon/src/active_jobs.rs:673:                ((delta_bytes as u128) * 1000 / (delta_ms as u128)) as u64
crates/blit-daemon/src/active_jobs.rs:712:                ((delta_bytes as u128) * 1000 / (delta_ms as u128)) as u64
crates/blit-core/src/tar_stream.rs:178:        _ => 128.min(config.channel_capacity * 2),
crates/blit-core/src/tar_stream.rs:298:        _ => 128.min(config.channel_capacity * 2),
crates/blit-core/src/buffer.rs:571:        // 8 streams × 2 × 64 MiB = 1 GiB liveness vs a 128 MiB cap.
crates/blit-core/src/buffer.rs:574:        assert_eq!(buffer_size, 8 * MIB, "128 MiB cap / (8 streams × 2)");
crates/blit-core/src/buffer.rs:576:        assert_eq!(budget, 128 * MIB);
crates/blit-core/src/dial.rs:20://!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
crates/blit-core/src/dial.rs:119:/// control stream turns this into a wire `DataPlaneResize` (the engine
crates/blit-core/src/dial.rs:316:        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
crates/blit-core/src/dial.rs:349:    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
crates/blit-core/src/dial.rs:486:    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
crates/blit-core/src/dial.rs:505:    let denom = elapsed.as_nanos().saturating_mul(streams as u128);
crates/blit-core/src/dial.rs:697:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
crates/blit-core/src/dial.rs:698:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
crates/blit-core/src/dial.rs:957:        assert_eq!(dial.propose_shape_resize(0), None);
crates/blit-core/src/dial.rs:958:        assert_eq!(dial.propose_shape_resize(1), None);
crates/blit-core/src/dial.rs:962:        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
crates/blit-core/src/dial.rs:971:        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
crates/blit-core/src/dial.rs:976:        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
crates/blit-core/src/dial.rs:981:        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
crates/blit-core/src/dial.rs:984:        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
crates/blit-core/src/dial.rs:988:            dial.propose_shape_resize(4).is_some(),
crates/blit-core/src/dial.rs:998:            .propose_shape_resize(100)
crates/blit-core/src/dial.rs:1003:            dial.propose_shape_resize(100),
crates/blit-core/src/perf_predictor.rs:609:        planner_ms: u128,
crates/blit-core/src/perf_predictor.rs:631:        planner_ms: u128,
crates/blit-core/src/perf_predictor.rs:632:        transfer_ms: u128,
crates/blit-core/src/perf_predictor.rs:715:                target_ms as u128,
crates/blit-core/src/perf_predictor.rs:753:                target_ms as u128,
crates/blit-core/src/perf_predictor.rs:767:                target_ms as u128,
crates/blit-core/src/perf_predictor.rs:1113:        planner_ms: u128,
crates/blit-core/src/perf_predictor.rs:1114:        transfer_ms: u128,
crates/blit-core/src/transfer_plan.rs:60:    let mut total_bytes: u128 = 0;
crates/blit-core/src/transfer_plan.rs:71:        total_bytes += e.size as u128;
crates/blit-core/src/transfer_plan.rs:106:        small_count >= 32 || avg_small_size <= 128 * 1024
crates/blit-core/src/transfer_plan.rs:127:        count_target = count_target.clamp(128, 4096);
crates/blit-core/src/transfer_plan.rs:152:    let mut target_bundle: u64 = options.medium_target.unwrap_or(128 * 1024 * 1024);
crates/blit-core/src/transfer_plan.rs:214:        // Two tiny files (avg ≤ 128 KiB → tar-eligible), one medium,
crates/blit-core/src/transfer_plan.rs:273:        // 300 tiny files with a count target of 128 must split into
crates/blit-core/src/transfer_plan.rs:274:        // ceil(300/128) = 3 shards (the clamp floor).
crates/blit-core/src/transfer_plan.rs:277:            small_count_target: Some(1), // clamped up to 128
crates/blit-core/src/transfer_plan.rs:288:        assert_eq!(shard_sizes, vec![128, 128, 44]);
crates/blit-core/src/perf_history.rs:137:    pub timestamp_epoch_ms: u128,
crates/blit-core/src/perf_history.rs:153:    pub planner_duration_ms: u128,
crates/blit-core/src/perf_history.rs:154:    pub transfer_duration_ms: u128,
crates/blit-core/src/perf_history.rs:185:        planner_duration_ms: u128,
crates/blit-core/src/perf_history.rs:186:        transfer_duration_ms: u128,
crates/blit-core/src/transfer_session/mod.rs:41:    DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp, FileData, FileHeader, FilterSpec,
crates/blit-core/src/transfer_session/mod.rs:42:    ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, SessionAccept, SessionError,
crates/blit-core/src/transfer_session/mod.rs:77:const DEST_DIFF_CHUNK: usize = 128;
crates/blit-core/src/transfer_session/mod.rs:209:    /// `ManifestComplete{scan_complete=false}`; callers that must not
crates/blit-core/src/transfer_session/mod.rs:452:        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs:463:        Some(Frame::Resize(_)) => "DataPlaneResize",
crates/blit-core/src/transfer_session/mod.rs:464:        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
crates/blit-core/src/transfer_session/mod.rs:929:    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
crates/blit-core/src/transfer_session/mod.rs:931:    ResizeAck(DataPlaneResizeAck),
crates/blit-core/src/transfer_session/mod.rs:1039:    // Set by the send half the moment ManifestComplete goes out. On
crates/blit-core/src/transfer_session/mod.rs:1043:    // after ManifestComplete received + all entries diffed).
crates/blit-core/src/transfer_session/mod.rs:1189:                        "NeedComplete before the source's ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs:1379:    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
crates/blit-core/src/transfer_session/mod.rs:1389:    // ManifestComplete.
crates/blit-core/src/transfer_session/mod.rs:1609:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
crates/blit-core/src/transfer_session/mod.rs:1758:                    "DataPlaneResizeAck on a session with no data plane",
crates/blit-core/src/transfer_session/mod.rs:1789:/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
crates/blit-core/src/transfer_session/mod.rs:1805:        tx.send(frame(Frame::Resize(DataPlaneResize {
crates/blit-core/src/transfer_session/mod.rs:1806:            op: DataPlaneResizeOp::Add as i32,
crates/blit-core/src/transfer_session/mod.rs:1906:            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
crates/blit-core/src/transfer_session/mod.rs:2413:            // already travels as `ManifestComplete{scan_complete}`.
crates/blit-core/src/transfer_session/mod.rs:2812:                        "manifest entry '{}' after ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs:2856:            Some(Frame::ManifestComplete(complete)) => {
crates/blit-core/src/transfer_session/mod.rs:2858:                    return Err(violation("duplicate ManifestComplete".into()));
crates/blit-core/src/transfer_session/mod.rs:2918:                // NeedComplete only after ManifestComplete received
crates/blit-core/src/transfer_session/mod.rs:2937:                        "payload record for '{}' before ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs:3031:                    return Err(violation("tar shard record before ManifestComplete".into()));
crates/blit-core/src/transfer_session/mod.rs:3092:                        "DataPlaneResize on a session with no data plane".into(),
crates/blit-core/src/transfer_session/mod.rs:3095:                let op = DataPlaneResizeOp::try_from(resize.op)
crates/blit-core/src/transfer_session/mod.rs:3096:                    .unwrap_or(DataPlaneResizeOp::Unspecified);
crates/blit-core/src/transfer_session/mod.rs:3097:                if op != DataPlaneResizeOp::Add {
crates/blit-core/src/transfer_session/mod.rs:3105:                        "DataPlaneResize sub_token must be 16 bytes".into(),
crates/blit-core/src/transfer_session/mod.rs:3112:                // dial failure is fatal (`add_dialed_stream`); a gone accept
crates/blit-core/src/transfer_session/mod.rs:3125:                            run.add_dialed_stream(&resize.sub_token).await?;
crates/blit-core/src/transfer_session/mod.rs:3141:                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
crates/blit-core/src/transfer_session/mod.rs:3150:                    return Err(violation("SourceDone before ManifestComplete".into()));
crates/blit-core/src/transfer_session/mod.rs:3173:                // scan-complete guard fired at ManifestComplete, but the
crates/blit-core/src/transfer_session/mod.rs:3785:/// them — otp-7b), only after ManifestComplete, only in a resume
crates/blit-core/src/transfer_session/mod.rs:3814:            format!("block record for '{relative_path}' before ManifestComplete"),
crates/blit-core/src/remote/transfer/data_plane.rs:25:/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).
crates/blit-core/src/transfer_session/data_plane.rs:28://! re-runs the shape table and proposes `DataPlaneResize{ADD}` (one stream
crates/blit-core/src/transfer_session/data_plane.rs:30://! `DataPlaneResizeAck` and grows its receive set. The control-lane frames
crates/blit-core/src/transfer_session/data_plane.rs:467:/// dialed instead of accepted. Resize (otp-5b-2): on a `DataPlaneResize`
crates/blit-core/src/transfer_session/data_plane.rs:469:/// [`Self::add_dialed_stream`] (the pull mirror of the SOURCE responder's
crates/blit-core/src/transfer_session/data_plane.rs:472:pub(super) struct InitiatorReceivePlaneRun {
crates/blit-core/src/transfer_session/data_plane.rs:488:    /// each epoch-N resize dial in [`Self::add_dialed_stream`].
crates/blit-core/src/transfer_session/data_plane.rs:504:) -> Result<InitiatorReceivePlaneRun> {
crates/blit-core/src/transfer_session/data_plane.rs:532:    Ok(InitiatorReceivePlaneRun {
crates/blit-core/src/transfer_session/data_plane.rs:544:impl InitiatorReceivePlaneRun {
crates/blit-core/src/transfer_session/data_plane.rs:553:    pub(super) async fn add_dialed_stream(&mut self, sub_token: &[u8]) -> Result<()> {
crates/blit-core/src/transfer_session/data_plane.rs:606:    Initiator(InitiatorReceivePlaneRun),
crates/blit-core/src/transfer_session/data_plane.rs:626:/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
crates/blit-core/src/transfer_session/data_plane.rs:627:/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
crates/blit-core/src/transfer_session/data_plane.rs:640:enum SourceSockets {
crates/blit-core/src/transfer_session/data_plane.rs:677:    sockets: SourceSockets,
crates/blit-core/src/transfer_session/data_plane.rs:765:        sockets: SourceSockets::Dial {
crates/blit-core/src/transfer_session/data_plane.rs:854:        sockets: SourceSockets::Accept {
crates/blit-core/src/transfer_session/data_plane.rs:871:    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
crates/blit-core/src/transfer_session/data_plane.rs:881:        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
crates/blit-core/src/transfer_session/data_plane.rs:913:            SourceSockets::Dial { host, tcp_port } => {
crates/blit-core/src/transfer_session/data_plane.rs:929:            SourceSockets::Accept { listener } => {

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '1,180p'; nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '440,590p'; nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '600,965p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '990,1430p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1650,1850p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2770,3170p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	//! Session-side TCP data-plane orchestration (otp-4b).
     2	//!
     3	//! The unified session reuses blit-core's data-plane byte plumbing —
     4	//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
     5	//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
     6	//! its OWN choreography here. The per-direction drivers (the old
     7	//! daemon push service loop and `remote::push::client` send driver)
     8	//! were deleted at cutover (otp-10c-2); this file is the one
     9	//! choreography.
    10	//!
    11	//! Two orthogonal axes (otp-5b): the **connection role** — the RESPONDER
    12	//! binds+accepts, the INITIATOR dials (NAT reality) — and the **byte
    13	//! role** — the SOURCE sends, the DESTINATION receives. otp-4b wired the
    14	//! push pair (DESTINATION responder accepts+receives; SOURCE initiator
    15	//! dials+sends); otp-5b adds the pull pair (SOURCE responder accepts+
    16	//! sends via [`accept_source_data_plane`]; DESTINATION initiator dials+
    17	//! receives via [`dial_destination_data_plane`]). The byte machinery is
    18	//! shared — send is `DataPlaneSession`/`DataPlaneSink`/the elastic
    19	//! pipeline, receive is `execute_receive_pipeline` — only socket
    20	//! acquisition differs per byte role. Because the grant is issued before
    21	//! any manifest is seen, the zero-knowledge `initial_stream_proposal` is
    22	//! 1 — the session data plane always starts single-stream (otp-4b-1) and
    23	//! grows via resize in BOTH directions (push otp-4b-2, pull otp-5b-2).
    24	//!
    25	//! Mid-transfer growth (otp-4b-2 push, otp-5b-2 pull): the SOURCE owns a
    26	//! [`TransferDial`] (bounded by the receiver's advertised capacity) and
    27	//! drives the sf-2 shape correction — as the need list accumulates it
    28	//! re-runs the shape table and proposes `DataPlaneResize{ADD}` (one stream
    29	//! per epoch) on the control lane; the DESTINATION replies
    30	//! `DataPlaneResizeAck` and grows its receive set. The control-lane frames
    31	//! are identical in both directions — only the transport action flips
    32	//! (the connection-initiating end always dials, the responder always
    33	//! accepts): in push the SOURCE **initiator** dials the epoch-N socket and
    34	//! the DESTINATION **responder** arms+accepts it; in pull the DESTINATION
    35	//! **initiator** dials and the SOURCE **responder** accepts. Either way
    36	//! the SOURCE hands its new send socket to the running elastic pipeline
    37	//! via [`SinkControl::Add`]. The cheap-dial live tuner (chunk/prefetch) is
    38	//! still future work — the resize moves only the stream count.
    39	
    40	use std::collections::{HashMap, HashSet};
    41	use std::path::{Path, PathBuf};
    42	use std::sync::atomic::{AtomicU64, Ordering};
    43	use std::sync::{Arc, Mutex as StdMutex};
    44	
    45	use async_trait::async_trait;
    46	use eyre::Result;
    47	use tokio::io::AsyncReadExt;
    48	use tokio::net::{TcpListener, TcpStream};
    49	use tokio::sync::mpsc;
    50	use tokio::task::JoinSet;
    51	
    52	use crate::buffer::BufferPool;
    53	use crate::dial::{initial_stream_proposal, local_receiver_capacity, TransferDial};
    54	use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
    55	use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
    56	use crate::remote::transfer::pipeline::execute_receive_pipeline;
    57	use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
    58	use crate::remote::transfer::socket::{
    59	    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
    60	};
    61	use crate::remote::transfer::source::TransferSource;
    62	use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
    63	use crate::remote::transfer::{
    64	    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession,
    65	    RemoteTransferProgress, SinkControl, SUB_TOKEN_LEN,
    66	};
    67	
    68	use super::{SessionFault, SourceInstruments};
    69	
    70	/// The set of granted-but-not-yet-received needs, shared between the
    71	/// destination's control loop (which inserts each path before sending
    72	/// its `NeedBatch`) and the data-plane receive (which claims each path
    73	/// as its payload lands). Completion is an empty set — the same signal
    74	/// the in-stream carrier uses via its inline `outstanding.remove`.
    75	pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;
    76	
    77	/// Headers of resume-granted needs (otp-7a/7b), keyed by relative path
    78	/// and retained until the grant's block record completes. Shared
    79	/// between the destination's control loop (which inserts each header
    80	/// before sending that file's `BlockHashList`, and claims it inline on
    81	/// the in-stream carrier) and the data-plane receive (which validates
    82	/// and claims it as block records land on the sockets) — the same
    83	/// sharing shape as [`OutstandingNeeds`].
    84	pub(super) type ResumeHeaders = Arc<StdMutex<HashMap<String, FileHeader>>>;
    85	
    86	/// otp-7b: the resume half of the data-plane receive contract — present
    87	/// only when the session negotiated resume. `headers` is the shared
    88	/// grant map above; `resumed` is the destination's `files_resumed`
    89	/// counter, incremented here because the control loop never sees
    90	/// data-plane block records.
    91	pub(super) struct ResumeRecv {
    92	    pub(super) headers: ResumeHeaders,
    93	    pub(super) resumed: Arc<AtomicU64>,
    94	}
    95	
    96	fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    97	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
    98	}
    99	
   100	/// [`dp_fault`] for failures that stringify an underlying I/O-bearing
   101	/// report (socket dials): carry the `io::ErrorKind` so the retry
   102	/// classifier still sees a transient transport condition (codex
   103	/// otp-10a F5).
   104	fn dp_fault_io(err: &eyre::Report, msg: impl Into<String>) -> eyre::Report {
   105	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg).with_io_kind_from(err))
   106	}
   107	
   108	// ---------------------------------------------------------------------------
   109	// Responder (DESTINATION) — bind, grant, accept, receive
   110	// ---------------------------------------------------------------------------
   111	
   112	/// A bound data-plane listener plus the credentials the responder
   113	/// advertises in its `SessionAccept`. Held by the responder driver
   114	/// across the handshake so the accept loop can run after establish.
   115	pub(super) struct ResponderDataPlane {
   116	    listener: TcpListener,
   117	    session_token: Vec<u8>,
   118	    epoch0_sub_token: Vec<u8>,
   119	    initial_streams: u32,
   120	    port: u16,
   121	}
   122	
   123	/// Bind a data-plane listener and mint credentials for the grant. Any
   124	/// failure (bind, addr, RNG) logs and returns `None` — the caller then
   125	/// issues a grant-less `SessionAccept` and the session falls back to the
   126	/// in-stream carrier (contract §Transport selection: a responder that
   127	/// cannot bind grants no data plane).
   128	pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
   129	    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
   130	        Ok(listener) => listener,
   131	        Err(err) => {
   132	            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
   133	            return None;
   134	        }
   135	    };
   136	    let port = match listener.local_addr() {
   137	        Ok(addr) => addr.port(),
   138	        Err(err) => {
   139	            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
   140	            return None;
   141	        }
   142	    };
   143	    // Two independent 16-byte credentials (contract §Transport: a socket
   144	    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
   145	    // is the fallible-RNG minter — a missing system RNG is an error, not
   146	    // a weaker credential.
   147	    let session_token = match generate_sub_token() {
   148	        Ok(token) => token,
   149	        Err(err) => {
   150	            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
   151	            return None;
   152	        }
   153	    };
   154	    let epoch0_sub_token = match generate_sub_token() {
   155	        Ok(token) => token,
   156	        Err(err) => {
   157	            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
   158	            return None;
   159	        }
   160	    };
   161	    // The grant is issued before any manifest is seen, so the proposal
   162	    // has zero knowledge: initial_streams == 1. All growth is via resize
   163	    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
   164	    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   165	    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
   166	    Some(ResponderDataPlane {
   167	        listener,
   168	        session_token,
   169	        epoch0_sub_token,
   170	        initial_streams,
   171	        port,
   172	    })
   173	}
   174	
   175	/// Aggregated destination-side receive result: the write outcome plus
   176	/// the number of data sockets accepted (epoch-0 + accepted resizes),
   177	/// which IS the settled live stream count this end observed. The sf-2
   178	/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
   179	pub(super) struct ReceiveTotals {
   180	    pub(super) outcome: SinkOutcome,
   440	        }
   441	    }
   442	    if buf[..session_token.len()] != *session_token {
   443	        return Err(dp_fault(
   444	            "resize data socket presented a wrong session token",
   445	        ));
   446	    }
   447	    let sub = &buf[session_token.len()..];
   448	    match armed.iter().position(|t| t.as_slice() == sub) {
   449	        Some(idx) => {
   450	            armed.swap_remove(idx);
   451	            Ok(socket)
   452	        }
   453	        None => Err(dp_fault(
   454	            "resize data socket presented an unarmed credential",
   455	        )),
   456	    }
   457	}
   458	
   459	// ---------------------------------------------------------------------------
   460	// Initiator (DESTINATION) — dial, receive (otp-5b-1)
   461	// ---------------------------------------------------------------------------
   462	
   463	/// Live handle to a DESTINATION **initiator** receive data plane (the
   464	/// pull direction): the initiator dials the granted epoch-0 socket(s) and
   465	/// drains each into the sink through the shared receive pipeline — the
   466	/// same byte machinery the DESTINATION responder uses, only the socket is
   467	/// dialed instead of accepted. Resize (otp-5b-2): on a `DataPlaneResize`
   468	/// the control loop dials one more epoch-N socket via
   469	/// [`Self::add_dialed_stream`] (the pull mirror of the SOURCE responder's
   470	/// accept). [`Self::finish`] joins the workers for the aggregated write
   471	/// outcome + settled stream count.
   472	pub(super) struct InitiatorReceivePlaneRun {
   473	    receives: JoinSet<Result<SinkOutcome>>,
   474	    streams: usize,
   475	    /// The responder host+port and session token, retained so a resize can
   476	    /// dial another receive socket to the same listener (otp-5b-2). The
   477	    /// DESTINATION initiator always dials; the SOURCE responder accepts.
   478	    host: String,
   479	    tcp_port: u32,
   480	    session_token: Vec<u8>,
   481	    /// The shared need-list receive sink each dialed worker drains into.
   482	    sink: Arc<dyn TransferSink>,
   483	    /// w6-1 progress lane each receive worker reports into (otp-10b-2);
   484	    /// cloned per worker, including resize-added ones.
   485	    progress: Option<RemoteTransferProgress>,
   486	    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
   487	    /// otp-10b-2). Applied to the epoch-0 dials at construction and to
   488	    /// each epoch-N resize dial in [`Self::add_dialed_stream`].
   489	    trace: bool,
   490	}
   491	
   492	/// Dial the granted epoch-0 socket(s) and spawn one receive worker per
   493	/// socket. `host` is the responder's host (the initiator reached the
   494	/// control plane there; the data plane rides the same host on the granted
   495	/// port — contract §Transport: the initiator always dials). Each worker
   496	/// drains its socket into `sink` (a [`NeedListSink`], same strictness the
   497	/// in-stream carrier applies inline).
   498	pub(super) async fn dial_destination_data_plane(
   499	    host: &str,
   500	    grant: &DataPlaneGrant,
   501	    sink: Arc<dyn TransferSink>,
   502	    progress: Option<RemoteTransferProgress>,
   503	    trace: bool,
   504	) -> Result<InitiatorReceivePlaneRun> {
   505	    let initial = grant.initial_streams.max(1) as usize;
   506	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   507	    let mut handshake = grant.session_token.clone();
   508	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   509	    let addr = format!("{host}:{}", grant.tcp_port);
   510	
   511	    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   512	    let mut streams = 0usize;
   513	    for _ in 0..initial {
   514	        // `dial_data_plane` connects, applies the data-socket policy, and
   515	        // writes the handshake credential — the same bounded dial the
   516	        // SOURCE initiator uses (design-3: one owner for every client-side
   517	        // data-plane dial, both directions).
   518	        if trace {
   519	            eprintln!("[data-plane-client] connecting to {addr} (receive)");
   520	        }
   521	        let socket = dial_data_plane(&addr, &handshake, None)
   522	            .await
   523	            .map_err(|err| {
   524	                dp_fault_io(
   525	                    &err,
   526	                    format!("dialing session data plane (receive): {err:#}"),
   527	                )
   528	            })?;
   529	        streams += 1;
   530	        spawn_receive(&mut receives, socket, &sink, progress.clone());
   531	    }
   532	    Ok(InitiatorReceivePlaneRun {
   533	        receives,
   534	        streams,
   535	        host: host.to_string(),
   536	        tcp_port: grant.tcp_port,
   537	        session_token: grant.session_token.clone(),
   538	        sink,
   539	        progress,
   540	        trace,
   541	    })
   542	}
   543	
   544	impl InitiatorReceivePlaneRun {
   545	    /// Dial one epoch-N resize socket to the responder and spawn its
   546	    /// receive worker (otp-5b-2 — the pull mirror of the SOURCE
   547	    /// responder's accept). Credential `session_token ‖ sub_token`. A dial
   548	    /// failure is FATAL, matching the SOURCE initiator's `add_stream`: a
   549	    /// same-build peer that granted+bound epoch-0 failing an epoch-N dial
   550	    /// is a transport fault worth surfacing (the DESTINATION dials before
   551	    /// it acks, so a failure faults the session before the SOURCE
   552	    /// responder commits to accepting the socket).
   553	    pub(super) async fn add_dialed_stream(&mut self, sub_token: &[u8]) -> Result<()> {
   554	        let mut handshake = self.session_token.clone();
   555	        handshake.extend_from_slice(sub_token);
   556	        let addr = format!("{}:{}", self.host, self.tcp_port);
   557	        if self.trace {
   558	            eprintln!("[data-plane-client] connecting to {addr} (receive resize)");
   559	        }
   560	        let socket = dial_data_plane(&addr, &handshake, None)
   561	            .await
   562	            .map_err(|err| {
   563	                dp_fault_io(
   564	                    &err,
   565	                    format!("dialing resize data plane (receive): {err:#}"),
   566	                )
   567	            })?;
   568	        self.streams += 1;
   569	        spawn_receive(
   570	            &mut self.receives,
   571	            socket,
   572	            &self.sink,
   573	            self.progress.clone(),
   574	        );
   575	        Ok(())
   576	    }
   577	
   578	    /// Join every receive worker for the aggregated write totals. A worker
   579	    /// error (receive failure / stall) surfaces here; each drains to its
   580	    /// socket's END record on a clean transfer.
   581	    async fn finish(mut self) -> Result<ReceiveTotals> {
   582	        let mut total = SinkOutcome::default();
   583	        while let Some(joined) = self.receives.join_next().await {
   584	            let outcome =
   585	                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   586	            total.files_written += outcome.files_written;
   587	            total.bytes_written += outcome.bytes_written;
   588	        }
   589	        Ok(ReceiveTotals {
   590	            outcome: total,
   600	pub(super) enum DestRecvPlane {
   601	    /// DESTINATION **responder** (push, otp-4b): accepts sockets; resize
   602	    /// grows the set by arming a credential its accept loop then accepts.
   603	    Responder(ResponderDataPlaneRun),
   604	    /// DESTINATION **initiator** (pull, otp-5b): dials sockets; resize grows
   605	    /// the set by dialing one more epoch-N socket (otp-5b-2).
   606	    Initiator(InitiatorReceivePlaneRun),
   607	}
   608	
   609	impl DestRecvPlane {
   610	    /// Drain the data plane to completion and report the settled stream
   611	    /// count + write outcome (the DESTINATION is the scorer).
   612	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   613	        match self {
   614	            DestRecvPlane::Responder(run) => run.finish().await,
   615	            DestRecvPlane::Initiator(run) => run.finish().await,
   616	        }
   617	    }
   618	}
   619	
   620	// ---------------------------------------------------------------------------
   621	// Initiator (SOURCE) — dial, authenticate, send, resize
   622	// ---------------------------------------------------------------------------
   623	
   624	/// A resize the SOURCE has proposed and minted a credential for but not
   625	/// yet completed: the driver has sent (or will send) the matching
   626	/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
   627	/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
   628	/// flight (the dial's `pending_epoch` enforces it; this is the
   629	/// driver-side record the ack is matched against).
   630	pub(super) struct PendingResize {
   631	    pub(super) epoch: u32,
   632	    pub(super) target_streams: u32,
   633	    pub(super) sub_token: Vec<u8>,
   634	}
   635	
   636	/// How the SOURCE acquires each epoch-N data socket for a shape resize —
   637	/// the two connection roles of otp-5b. Byte direction is identical (the
   638	/// SOURCE sends), and `propose_resize` is the same either way; only socket
   639	/// acquisition flips.
   640	enum SourceSockets {
   641	    /// SOURCE **initiator** (push, otp-4b-2): dials each epoch-N socket to
   642	    /// the granted host:port.
   643	    Dial { host: String, tcp_port: u32 },
   644	    /// SOURCE **responder** (pull, otp-5b-2): accepts each epoch-N socket
   645	    /// off the listener it already bound for epoch-0, credential
   646	    /// `session_token ‖ sub_token`.
   647	    Accept { listener: TcpListener },
   648	}
   649	
   650	/// A running source-side data plane: the dialed/accepted socket(s) wrapped
   651	/// as an ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the
   652	/// sf-2 shape correction). Planned payloads are fed via [`Self::queue`];
   653	/// closing via [`Self::finish`] drains the pipeline, emits each socket's
   654	/// END record, and returns the bytes this end sent.
   655	pub(super) struct SourceDataPlane {
   656	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   657	    control_tx: mpsc::UnboundedSender<SinkControl>,
   658	    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
   659	    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
   660	    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
   661	    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
   662	    // advertised capacity (contract §Invariants 5). The resize drives only
   663	    // its shape-correction stream count; the cheap-dial tuner is future
   664	    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
   665	    dial: Arc<TransferDial>,
   666	    source: Arc<dyn TransferSource>,
   667	    session_token: Vec<u8>,
   668	    pool: Arc<BufferPool>,
   669	    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
   670	    /// otp-10a). Applied to the epoch-0 sockets at construction and to
   671	    /// each epoch-N resize socket in [`Self::add_stream`].
   672	    trace: bool,
   673	    /// How each epoch-N resize socket is acquired (dial for the SOURCE
   674	    /// initiator, accept for the SOURCE responder). The data plane grows
   675	    /// mid-transfer in both cases; the control-lane resize choreography is
   676	    /// identical — only this transport action flips (otp-5b-2).
   677	    sockets: SourceSockets,
   678	}
   679	
   680	/// Dial the granted data plane and start the elastic send pipeline.
   681	/// `host` is the responder's host (the initiator connected the control
   682	/// plane to it; the data plane rides the same host on the granted port —
   683	/// contract §Transport: the initiator always dials). `receiver_capacity`
   684	/// is the DESTINATION's advertised profile from `SessionAccept`; it
   685	/// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
   686	/// never unlimited).
   687	pub(super) async fn dial_source_data_plane(
   688	    host: &str,
   689	    grant: &DataPlaneGrant,
   690	    receiver_capacity: Option<&CapacityProfile>,
   691	    source: Arc<dyn TransferSource>,
   692	    instruments: &SourceInstruments,
   693	) -> Result<SourceDataPlane> {
   694	    let initial = grant.initial_streams.max(1) as usize;
   695	    // The byte sender's dial, bounded by the receiver's advertised
   696	    // capacity. Seed the settled live count to the granted epoch-0
   697	    // streams — every shape-resize proposal steps from here.
   698	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   699	    dial.set_negotiated_streams(initial);
   700	
   701	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   702	    let mut handshake = grant.session_token.clone();
   703	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   704	
   705	    // Provision the pool for the dial ceiling so resize-added sockets
   706	    // draw buffers from the same pool without re-pooling (as old push
   707	    // does — a shared pool sized for the maximum stream count).
   708	    let pool = Arc::new(BufferPool::for_data_plane(
   709	        dial.chunk_bytes(),
   710	        dial.ceiling_max_streams().max(1),
   711	    ));
   712	    let trace = instruments.trace_data_plane;
   713	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   714	    for _ in 0..initial {
   715	        let session = DataPlaneSession::connect(
   716	            host,
   717	            grant.tcp_port,
   718	            &handshake,
   719	            dial.chunk_bytes(),
   720	            dial.prefetch_count(),
   721	            trace,
   722	            dial.tcp_buffer_bytes(),
   723	            Arc::clone(&pool),
   724	        )
   725	        .await
   726	        .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?;
   727	        // The source-side sink never reads its dst_root (it only sends);
   728	        // `root()` is consulted by the relay/receive case, not here.
   729	        sinks.push(Arc::new(DataPlaneSink::new(
   730	            session,
   731	            Arc::clone(&source),
   732	            PathBuf::new(),
   733	        )));
   734	    }
   735	
   736	    let prefetch = dial.prefetch_count().max(1);
   737	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   738	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   739	    let pipe_source = Arc::clone(&source);
   740	    let pipe_progress = instruments.progress.clone();
   741	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   742	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   743	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   744	        execute_sink_pipeline_elastic(
   745	            pipe_source,
   746	            sinks,
   747	            payload_rx,
   748	            prefetch,
   749	            pipe_progress.as_ref(),
   750	            Some(control_rx),
   751	        )
   752	        .await
   753	    }));
   754	    Ok(SourceDataPlane {
   755	        payload_tx: Some(payload_tx),
   756	        control_tx,
   757	        pipeline: Some(pipeline),
   758	        dial,
   759	        source,
   760	        session_token: grant.session_token.clone(),
   761	        pool,
   762	        trace,
   763	        // SOURCE initiator: each epoch-N resize socket is dialed to the
   764	        // granted host:port.
   765	        sockets: SourceSockets::Dial {
   766	            host: host.to_string(),
   767	            tcp_port: grant.tcp_port,
   768	        },
   769	    })
   770	}
   771	
   772	/// Accept the granted epoch-0 socket(s) off a bound responder listener and
   773	/// start the elastic SEND pipeline over them — the SOURCE **responder**
   774	/// half of the pull data plane (otp-5b-1). Symmetric with
   775	/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
   776	/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
   777	/// socket acquisition differs (accept here, dial there).
   778	/// `DataPlaneSession::from_stream` builds a send session from an already-
   779	/// accepted socket — the same primitive the old `pull_sync` daemon-send
   780	/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
   781	/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
   782	/// wherever it initiates). The bound listener is retained so each epoch-N
   783	/// resize socket is accepted off it (otp-5b-2): the DESTINATION initiator
   784	/// dials, this end accepts, the control-lane frames identical to push.
   785	pub(super) async fn accept_source_data_plane(
   786	    bound: ResponderDataPlane,
   787	    receiver_capacity: Option<&CapacityProfile>,
   788	    source: Arc<dyn TransferSource>,
   789	    instruments: &SourceInstruments,
   790	) -> Result<SourceDataPlane> {
   791	    let initial = bound.initial_streams.max(1) as usize;
   792	    // The byte sender's dial, bounded by the receiver's advertised
   793	    // capacity; seed the live count to the granted epoch-0 streams. Growth
   794	    // is via resize (otp-5b-2): the accept-based epoch-N socket steps from
   795	    // here, one stream per epoch, same as the SOURCE initiator.
   796	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   797	    dial.set_negotiated_streams(initial);
   798	
   799	    // Epoch-0 credential the dialing DESTINATION presents:
   800	    // session_token ‖ epoch0_sub_token (contract §Transport).
   801	    let mut epoch0 = bound.session_token.clone();
   802	    epoch0.extend_from_slice(&bound.epoch0_sub_token);
   803	
   804	    let pool = Arc::new(BufferPool::for_data_plane(
   805	        dial.chunk_bytes(),
   806	        dial.ceiling_max_streams().max(1),
   807	    ));
   808	    let trace = instruments.trace_data_plane;
   809	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   810	    for _ in 0..initial {
   811	        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
   812	        let session = DataPlaneSession::from_stream(
   813	            socket,
   814	            trace,
   815	            dial.chunk_bytes(),
   816	            dial.prefetch_count(),
   817	            Arc::clone(&pool),
   818	        )
   819	        .await;
   820	        sinks.push(Arc::new(DataPlaneSink::new(
   821	            session,
   822	            Arc::clone(&source),
   823	            PathBuf::new(),
   824	        )));
   825	    }
   826	
   827	    let prefetch = dial.prefetch_count().max(1);
   828	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   829	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   830	    let pipe_source = Arc::clone(&source);
   831	    let pipe_progress = instruments.progress.clone();
   832	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   833	        execute_sink_pipeline_elastic(
   834	            pipe_source,
   835	            sinks,
   836	            payload_rx,
   837	            prefetch,
   838	            pipe_progress.as_ref(),
   839	            Some(control_rx),
   840	        )
   841	        .await
   842	    }));
   843	    Ok(SourceDataPlane {
   844	        payload_tx: Some(payload_tx),
   845	        control_tx,
   846	        pipeline: Some(pipeline),
   847	        dial,
   848	        source,
   849	        session_token: bound.session_token,
   850	        pool,
   851	        trace,
   852	        // SOURCE responder: each epoch-N resize socket is accepted off the
   853	        // same listener epoch-0 came in on (otp-5b-2).
   854	        sockets: SourceSockets::Accept {
   855	            listener: bound.listener,
   856	        },
   857	    })
   858	}
   859	
   860	impl SourceDataPlane {
   861	    /// The live dial (the byte sender owns it). The driver reads
   862	    /// `live_streams()` for observability and calls `resize_settled` as
   863	    /// each proposal completes.
   864	    pub(super) fn dial(&self) -> &Arc<TransferDial> {
   865	        &self.dial
   866	    }
   867	
   868	    /// sf-2 shape correction: propose one ADD toward the stream count the
   869	    /// accumulated need list implies, if none is in flight and the shape
   870	    /// wants more than the current live count. Mints the resize
   871	    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
   872	    /// the record back on the matching ack.
   873	    pub(super) fn propose_resize(
   874	        &self,
   875	        needed_bytes: u64,
   876	        needed_count: usize,
   877	    ) -> Result<Option<PendingResize>> {
   878	        let desired =
   879	            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
   880	                as usize;
   881	        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
   882	            return Ok(None);
   883	        };
   884	        let sub_token = generate_sub_token()
   885	            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
   886	        Ok(Some(PendingResize {
   887	            epoch: proposal.epoch,
   888	            target_streams: proposal.target_streams as u32,
   889	            sub_token,
   890	        }))
   891	    }
   892	
   893	    /// Acquire the epoch-N data socket for an accepted resize and hand it
   894	    /// to the running pipeline (`SinkControl::Add`). The SOURCE initiator
   895	    /// (push) DIALS it; the SOURCE responder (pull, otp-5b-2) ACCEPTS the
   896	    /// socket the DESTINATION initiator dials after its ack, off the same
   897	    /// listener epoch-0 came in on. A dial/accept failure is FATAL
   898	    /// (fail-fast): a same-build peer that established epoch-0 failing an
   899	    /// epoch-N socket is a transport fault worth surfacing — and faulting
   900	    /// the session aborts the peer's counterpart via AbortOnDrop, so no
   901	    /// slot orphans. (Old push recovers non-fatally via an arm TTL; the
   902	    /// session trades that for simplicity — noted in the finding doc.) If
   903	    /// the pipeline is already gone (transfer completing under the ADD),
   904	    /// the just-acquired socket is closed cleanly so the peer's worker sees
   905	    /// its END, not a reset.
   906	    ///
   907	    /// The accept is bounded and unambiguous: at most one resize is in
   908	    /// flight (the driver's `pending_resize`) and epoch-0 is already
   909	    /// accepted, so the next connection off the listener is exactly this
   910	    /// resize's socket — verified against `session_token ‖ sub_token`.
   911	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   912	        let session = match &self.sockets {
   913	            SourceSockets::Dial { host, tcp_port } => {
   914	                let mut handshake = self.session_token.clone();
   915	                handshake.extend_from_slice(sub_token);
   916	                DataPlaneSession::connect(
   917	                    host,
   918	                    *tcp_port,
   919	                    &handshake,
   920	                    self.dial.chunk_bytes(),
   921	                    self.dial.prefetch_count(),
   922	                    self.trace,
   923	                    self.dial.tcp_buffer_bytes(),
   924	                    Arc::clone(&self.pool),
   925	                )
   926	                .await
   927	                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?
   928	            }
   929	            SourceSockets::Accept { listener } => {
   930	                let mut expected = self.session_token.clone();
   931	                expected.extend_from_slice(sub_token);
   932	                let socket = accept_authenticated(listener, &expected).await?;
   933	                DataPlaneSession::from_stream(
   934	                    socket,
   935	                    self.trace,
   936	                    self.dial.chunk_bytes(),
   937	                    self.dial.prefetch_count(),
   938	                    Arc::clone(&self.pool),
   939	                )
   940	                .await
   941	            }
   942	        };
   943	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   944	            session,
   945	            Arc::clone(&self.source),
   946	            PathBuf::new(),
   947	        ));
   948	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   949	            if let SinkControl::Add(sink) = returned.0 {
   950	                let _ = sink.finish().await;
   951	            }
   952	        }
   953	        Ok(())
   954	    }
   955	
   956	    /// Feed one planned batch into the send pipeline. The pipeline
   957	    /// prepares each payload (tar-shard/file) and writes it through the
   958	    /// data-plane record framing across the live socket(s).
   959	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   960	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   961	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   962	        })?;
   963	        for payload in payloads {
   964	            tx.send(payload).await.map_err(|_| {
   965	                dp_fault("data-plane send pipeline closed before all payloads sent")
   990	            eyre::bail!("run_source initiator config unsupported: {fault}");
   991	        }
   992	    }
   993	
   994	    let negotiated = establish(
   995	        &mut transport,
   996	        &cfg.hello,
   997	        &cfg.endpoint,
   998	        TransferRole::Source,
   999	        &source_open_validator,
  1000	        // run_source only ever resolves nothing: a SOURCE *initiator*
  1001	        // owns its own root, and a SOURCE *responder* driven directly
  1002	        // (the in-process role suite) is handed a Fixed source. The
  1003	        // daemon SOURCE responder resolves module→root inside
  1004	        // `run_responder`, not here (otp-5).
  1005	        None,
  1006	    )
  1007	    .await?;
  1008	
  1009	    drive_source(
  1010	        cfg.plan_options,
  1011	        cfg.data_plane_host,
  1012	        cfg.instruments,
  1013	        negotiated,
  1014	        transport,
  1015	        source,
  1016	    )
  1017	    .await
  1018	}
  1019	
  1020	/// The SOURCE session body after establish: spawn the receive half,
  1021	/// run the send half, and map a fault to a peer-notified report. Shared
  1022	/// by [`run_source`] (initiator or direct-responder) and
  1023	/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
  1024	/// choreography is single-sourced.
  1025	async fn drive_source(
  1026	    plan_options: PlanOptions,
  1027	    data_plane_host: Option<String>,
  1028	    instruments: SourceInstruments,
  1029	    mut negotiated: Negotiated,
  1030	    transport: FrameTransport,
  1031	    source: Arc<dyn TransferSource>,
  1032	) -> Result<TransferSummary> {
  1033	    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
  1034	    // its send sockets on; a SOURCE initiator (push) has none and dials the
  1035	    // grant it received instead. Take it here so the send half owns it.
  1036	    let responder_data_plane = negotiated.responder_data_plane.take();
  1037	    let (mut tx, rx) = transport.split();
  1038	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
  1039	    // Set by the send half the moment ManifestComplete goes out. On
  1040	    // an ordered transport, a NeedComplete arriving while this is
  1041	    // still false is provably premature — the peer cannot have
  1042	    // received what we have not sent (contract: NeedComplete only
  1043	    // after ManifestComplete received + all entries diffed).
  1044	    let manifest_sent = Arc::new(AtomicBool::new(false));
  1045	    let (event_tx, event_rx) = mpsc::unbounded_channel();
  1046	    // Fault side-channel (codex otp-8 F1): the in-stream send path
  1047	    // races this signal against blocked record sends; see
  1048	    // `SourceEventSender`.
  1049	    let (fault_tx, fault_rx) = watch::channel(None::<SessionFault>);
  1050	    // AbortOnDrop: an early error return below must abort the receive
  1051	    // half instead of leaking it (same rationale as design-2 / w4-1).
  1052	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
  1053	        rx,
  1054	        Arc::clone(&sent),
  1055	        Arc::clone(&manifest_sent),
  1056	        resume_negotiated(&negotiated.open),
  1057	        // otp-10a: the recv half owns need-batch arrival, which is the
  1058	        // push-direction progress denominator (contract on
  1059	        // `ProgressEvent::ManifestBatch`: "push: need-list batches").
  1060	        instruments.progress.clone(),
  1061	        SourceEventSender {
  1062	            tx: event_tx,
  1063	            fault_signal: fault_tx,
  1064	        },
  1065	    )));
  1066	
  1067	    match source_send_half(
  1068	        plan_options,
  1069	        data_plane_host.as_deref(),
  1070	        instruments,
  1071	        &negotiated,
  1072	        responder_data_plane,
  1073	        &mut tx,
  1074	        source,
  1075	        sent,
  1076	        &manifest_sent,
  1077	        event_rx,
  1078	        fault_rx,
  1079	    )
  1080	    .await
  1081	    {
  1082	        Ok(summary) => Ok(summary),
  1083	        Err(report) => {
  1084	            let mut fault = fault_from_report(report);
  1085	            if !fault.peer_notified {
  1086	                let _ = tx.send(error_frame(&fault)).await;
  1087	                fault.peer_notified = true;
  1088	            }
  1089	            Err(eyre::Report::new(fault))
  1090	        }
  1091	    }
  1092	}
  1093	
  1094	/// Receive half of the source driver: drains the transport for the
  1095	/// whole session so destination sends can never deadlock against a
  1096	/// blocked source send, and routes the destination lane to the send
  1097	/// half. Terminates on summary, error, close, or violation.
  1098	async fn source_recv_half(
  1099	    mut rx: Box<dyn FrameRx>,
  1100	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
  1101	    manifest_sent: Arc<AtomicBool>,
  1102	    resume_session: bool,
  1103	    progress: Option<RemoteTransferProgress>,
  1104	    events: SourceEventSender,
  1105	) {
  1106	    loop {
  1107	        let received = match rx.recv().await {
  1108	            Ok(Some(f)) => f,
  1109	            Ok(None) => {
  1110	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
  1111	                    "peer closed before TransferSummary",
  1112	                )));
  1113	                return;
  1114	            }
  1115	            Err(err) => {
  1116	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
  1117	                    "transport receive failed: {err:#}"
  1118	                ))));
  1119	                return;
  1120	            }
  1121	        };
  1122	        match received.frame {
  1123	            Some(Frame::NeedBatch(batch)) => {
  1124	                // otp-10a: the need list is the push-direction progress
  1125	                // denominator ("N of M files"). Entries are unique by
  1126	                // contract (a duplicate need faults below), so every
  1127	                // batch is newly-requested work — same semantics as the
  1128	                // old push driver's `report_manifest_batch`.
  1129	                if let Some(p) = &progress {
  1130	                    if !batch.entries.is_empty() {
  1131	                        p.report_manifest_batch(batch.entries.len());
  1132	                    }
  1133	                }
  1134	                for entry in batch.entries {
  1135	                    if entry.resume && !resume_session {
  1136	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1137	                            format!(
  1138	                                "resume-flagged need for '{}' in a session opened without resume",
  1139	                                entry.relative_path
  1140	                            ),
  1141	                        )));
  1142	                        return;
  1143	                    }
  1144	                    let header = sent
  1145	                        .lock()
  1146	                        .expect("sent-manifest lock poisoned")
  1147	                        .remove(&entry.relative_path);
  1148	                    match header {
  1149	                        Some(h) if entry.resume => {
  1150	                            let _ = events.send(SourceEvent::ResumeNeed(h));
  1151	                        }
  1152	                        Some(h) => {
  1153	                            let _ = events.send(SourceEvent::Need(h));
  1154	                        }
  1155	                        None => {
  1156	                            let _ = events.send(SourceEvent::Fault(
  1157	                                SessionFault::protocol_violation(format!(
  1158	                                    "need for unknown or already-needed path '{}'",
  1159	                                    entry.relative_path
  1160	                                )),
  1161	                            ));
  1162	                            return;
  1163	                        }
  1164	                    }
  1165	                }
  1166	            }
  1167	            Some(Frame::BlockHashes(list)) => {
  1168	                // otp-7a: the destination's hashes for a resume-flagged
  1169	                // need. The send half correlates it with the held need;
  1170	                // in a non-resume session the frame is off-contract.
  1171	                if !resume_session {
  1172	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1173	                        format!(
  1174	                            "BlockHashList for '{}' in a session opened without resume",
  1175	                            list.relative_path
  1176	                        ),
  1177	                    )));
  1178	                    return;
  1179	                }
  1180	                let _ = events.send(SourceEvent::BlockHashes(list));
  1181	            }
  1182	            Some(Frame::NeedComplete(_)) => {
  1183	                if !manifest_sent.load(Ordering::Acquire) {
  1184	                    // Fail fast at arrival time (otp-3 codex F2): the
  1185	                    // event queue would otherwise let an early
  1186	                    // NeedComplete be processed late and pass as
  1187	                    // legitimate.
  1188	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1189	                        "NeedComplete before the source's ManifestComplete",
  1190	                    )));
  1191	                    return;
  1192	                }
  1193	                let _ = events.send(SourceEvent::NeedComplete);
  1194	            }
  1195	            Some(Frame::ResizeAck(ack)) => {
  1196	                // The destination's response to a shape-resize proposal
  1197	                // (otp-4b-2). Forward it to the send half, which owns the
  1198	                // dial and dials the epoch-N socket on `accepted`.
  1199	                let _ = events.send(SourceEvent::ResizeAck(ack));
  1200	            }
  1201	            Some(Frame::Summary(summary)) => {
  1202	                let _ = events.send(SourceEvent::Summary(summary));
  1203	                return;
  1204	            }
  1205	            Some(Frame::Error(err)) => {
  1206	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
  1207	                return;
  1208	            }
  1209	            other => {
  1210	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1211	                    format!("{} on the source's receive lane", frame_name(&other)),
  1212	                )));
  1213	                return;
  1214	            }
  1215	        }
  1216	    }
  1217	}
  1218	
  1219	/// otp-7a: the send half's resume bookkeeping. A resume-flagged need is
  1220	/// HELD until its `BlockHashList` arrives (the contract's strict
  1221	/// ordering — the source must not send a byte of that file first); the
  1222	/// correlated pair then queues for the block phase.
  1223	#[derive(Default)]
  1224	struct ResumeSendState {
  1225	    held: HashMap<String, FileHeader>,
  1226	    ready: Vec<(FileHeader, BlockHashList)>,
  1227	}
  1228	
  1229	#[allow(clippy::too_many_arguments)]
  1230	async fn source_send_half(
  1231	    plan_options: PlanOptions,
  1232	    data_plane_host: Option<&str>,
  1233	    instruments: SourceInstruments,
  1234	    negotiated: &Negotiated,
  1235	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
  1236	    tx: &mut Box<dyn FrameTx>,
  1237	    source: Arc<dyn TransferSource>,
  1238	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
  1239	    manifest_sent: &AtomicBool,
  1240	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
  1241	    mut fault_signal: watch::Receiver<Option<SessionFault>>,
  1242	) -> Result<TransferSummary> {
  1243	    let mut pending: Vec<FileHeader> = Vec::new();
  1244	    let mut resume: ResumeSendState = ResumeSendState::default();
  1245	    let mut need_complete = false;
  1246	
  1247	    // Data plane (otp-4b/5b): set up the send sockets up front — BEFORE
  1248	    // streaming the manifest — so the peer sees the connections promptly
  1249	    // rather than waiting out a bounded-accept/connect timeout while a long
  1250	    // manifest streams. Which end connects depends on connection role
  1251	    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
  1252	    // listener; a SOURCE **initiator** (push) dials the grant it received.
  1253	    // Byte direction is the same either way (SOURCE sends), so both yield a
  1254	    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
  1255	    // in-stream carrier (fallback), which needs no early setup.
  1256	    let mut data_plane = match responder_data_plane {
  1257	        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
  1258	        // initiator advertised its capacity in the open (byte RECEIVER
  1259	        // advertises, wherever it initiates); the accept plane is single-
  1260	        // stream (otp-5b-1).
  1261	        Some(bound) => Some(
  1262	            data_plane::accept_source_data_plane(
  1263	                bound,
  1264	                negotiated.open.receiver_capacity.as_ref(),
  1265	                Arc::clone(&source),
  1266	                &instruments,
  1267	            )
  1268	            .await?,
  1269	        ),
  1270	        // SOURCE initiator (push, otp-4b): dial the grant if the responder
  1271	        // granted a data plane; else in-stream.
  1272	        None => match &negotiated.accept.data_plane {
  1273	            Some(grant) => {
  1274	                let host = data_plane_host.ok_or_else(|| {
  1275	                    eyre::Report::new(SessionFault::internal(
  1276	                        "responder granted a TCP data plane but this initiator has no host to dial",
  1277	                    ))
  1278	                })?;
  1279	                Some(
  1280	                    data_plane::dial_source_data_plane(
  1281	                        host,
  1282	                        grant,
  1283	                        negotiated.accept.receiver_capacity.as_ref(),
  1284	                        Arc::clone(&source),
  1285	                        &instruments,
  1286	                    )
  1287	                    .await?,
  1288	                )
  1289	            }
  1290	            None => None,
  1291	        },
  1292	    };
  1293	
  1294	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
  1295	    // fed to the shape table so the SOURCE grows the data-plane stream
  1296	    // count as the workload's shape becomes known. Append-only (a need is
  1297	    // counted once, when it arrives), and the in-flight resize record the
  1298	    // ack is matched against (at most one — the dial enforces it).
  1299	    let mut needed_bytes: u64 = 0;
  1300	    let mut needed_count: usize = 0;
  1301	    let mut pending_resize: Option<data_plane::PendingResize> = None;
  1302	
  1303	    // Streaming manifest: entries go out as enumeration produces them
  1304	    // (immediate start in every direction — plan §Design 2). The open
  1305	    // carries no source path (the source end owns its local endpoint) but
  1306	    // does carry the include/exclude/size/age filter (otp-6a): only
  1307	    // matching files are manifested and transferred. The filter MUST ride
  1308	    // the wire (not be pre-wrapped by a local caller) because for pull the
  1309	    // SOURCE is the remote daemon responder — it, not the client, owns the
  1310	    // scan. Apply it through the universal `FilteredSource` decorator, the
  1311	    // single filter chokepoint every source impl routes through, rather
  1312	    // than the per-impl `scan(filter)` arg — a source impl is free to
  1313	    // ignore that arg (the since-deleted relay source did; codex otp-6a
  1314	    // F1), and the chokepoint makes filtering independent of it. A
  1315	    // default/absent filter scans everything (unchanged from otp-3). Globs
  1316	    // were validated at OPEN (`source_open_validator`), so the conversion
  1317	    // cannot fail on a validated open; map any error to a fault regardless.
  1318	    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
  1319	        Some(spec) if *spec != FilterSpec::default() => {
  1320	            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  1321	                .map_err(|e| {
  1322	                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
  1323	                })?;
  1324	            Arc::new(crate::remote::transfer::source::FilteredSource::new(
  1325	                Arc::clone(&source),
  1326	                filter,
  1327	            ))
  1328	        }
  1329	        _ => Arc::clone(&source),
  1330	    };
  1331	    // otp-10b-1: a Checksum session fills each manifest header's
  1332	    // checksum so the DESTINATION can skip content-equal files
  1333	    // regardless of mtime. Wrapped OUTSIDE the filter so only
  1334	    // in-scope files pay the hash; a serving end that refuses to hash
  1335	    // never gets here (CHECKSUM_DISABLED at OPEN).
  1336	    let scan_source: Arc<dyn TransferSource> =
  1337	        if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
  1338	            Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
  1339	                scan_source,
  1340	            ))
  1341	        } else {
  1342	            scan_source
  1343	        };
  1344	    // otp-10a: callers that must not treat a partial transfer as success
  1345	    // (the push verb, `blit move`'s source-delete gate) supply their own
  1346	    // accumulator via `SourceInstruments` and inspect it after the
  1347	    // session returns; the wire behavior is identical either way.
  1348	    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
  1349	    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
  1350	    while let Some(header) = header_rx.recv().await {
  1351	        sent.lock()
  1352	            .expect("sent-manifest lock poisoned")
  1353	            .insert(header.relative_path.clone(), header.clone());
  1354	        tx.send(frame(Frame::ManifestEntry(header))).await?;
  1355	        // Faults detected by the receive half abort the stream now,
  1356	        // not after the full scan; needs just accumulate. (Resize acks
  1357	        // cannot arrive yet — none is proposed before the payload phase.)
  1358	        drain_ready_source_events(
  1359	            &mut events,
  1360	            &mut pending,
  1361	            &mut resume,
  1362	            &mut need_complete,
  1363	            &mut needed_bytes,
  1364	            &mut needed_count,
  1365	            data_plane.as_ref(),
  1366	            tx,
  1367	            &mut pending_resize,
  1368	        )
  1369	        .await?;
  1370	    }
  1371	    let scanned = scan_handle
  1372	        .await
  1373	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
  1374	    let scan_complete = unreadable
  1375	        .lock()
  1376	        .expect("unreadable list lock poisoned")
  1377	        .is_empty();
  1378	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
  1379	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
  1380	        scan_complete,
  1381	    })))
  1382	    .await?;
  1383	    manifest_sent.store(true, Ordering::Release);
  1384	
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch (one ADD per epoch; a no-op while one is in
  1417	                    // flight or the shape wants no more).
  1418	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1419	                        .await?;
  1420	                    let payloads =
  1421	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1422	                    // A cancel while earlier batches are actively moving
  1423	                    // closes the send pipeline under backpressure, so this
  1424	                    // queue fails with a data-plane error — prefer the
  1425	                    // peer's framed reason (CANCELLED) the same way the
  1426	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1427	                    // against events like finish(): live `Need`s still
  1428	                    // arrive here, and `recv_peer_fault` would consume them.
  1429	                    if let Err(dp_err) = dp.queue(payloads).await {
  1430	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1650	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1651	/// next ADD (the one-per-epoch ramp).
  1652	#[allow(clippy::too_many_arguments)]
  1653	async fn process_source_event(
  1654	    event: SourceEvent,
  1655	    pending: &mut Vec<FileHeader>,
  1656	    resume: &mut ResumeSendState,
  1657	    need_complete: &mut bool,
  1658	    needed_bytes: &mut u64,
  1659	    needed_count: &mut usize,
  1660	    data_plane: Option<&data_plane::SourceDataPlane>,
  1661	    tx: &mut Box<dyn FrameTx>,
  1662	    pending_resize: &mut Option<data_plane::PendingResize>,
  1663	) -> Result<()> {
  1664	    match event {
  1665	        SourceEvent::Need(header) => {
  1666	            if *need_complete {
  1667	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1668	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1669	                )));
  1670	            }
  1671	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1672	            *needed_count += 1;
  1673	            pending.push(header);
  1674	            Ok(())
  1675	        }
  1676	        SourceEvent::ResumeNeed(header) => {
  1677	            if *need_complete {
  1678	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1679	                    format!(
  1680	                        "resume need for '{}' after NeedComplete",
  1681	                        header.relative_path
  1682	                    ),
  1683	                )));
  1684	            }
  1685	            // Shape totals count the whole file — the diff hasn't run
  1686	            // yet, so the need list's implied workload is the honest
  1687	            // upper bound (same accounting a plain need gets).
  1688	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1689	            *needed_count += 1;
  1690	            // HELD until its BlockHashList arrives; no duplicate is
  1691	            // possible (the receive half's sent-map removal already
  1692	            // faults a second need for the same path).
  1693	            resume.held.insert(header.relative_path.clone(), header);
  1694	            Ok(())
  1695	        }
  1696	        SourceEvent::BlockHashes(list) => {
  1697	            // Validate the wire block size at ARRIVAL (codex F5), not
  1698	            // when the record is eventually sent — pending plain files
  1699	            // go out first, and an already-invalid frame must fail fast.
  1700	            // A conforming destination clamps into this range (D5 /
  1701	            // D-2026-07-10-1); same-build peers make a mismatch a
  1702	            // violation, never a negotiation. The ceiling is the
  1703	            // CARRIER's (otp-7b, D-2026-07-10-2): binary data-plane
  1704	            // records take up to the wire block cap; in-stream frames
  1705	            // must stay under the gRPC frame limit.
  1706	            let ceiling = if data_plane.is_some() {
  1707	                MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  1708	            } else {
  1709	                MAX_IN_STREAM_RESUME_BLOCK_SIZE
  1710	            };
  1711	            let bs = list.block_size as usize;
  1712	            if !(MIN_RESUME_BLOCK_SIZE..=ceiling).contains(&bs) {
  1713	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1714	                    format!(
  1715	                        "BlockHashList for '{}' block_size {bs} outside \
  1716	                         [{MIN_RESUME_BLOCK_SIZE}, {ceiling}]",
  1717	                        list.relative_path
  1718	                    ),
  1719	                )));
  1720	            }
  1721	            match resume.held.remove(&list.relative_path) {
  1722	                Some(header) => {
  1723	                    resume.ready.push((header, list));
  1724	                    Ok(())
  1725	                }
  1726	                None => Err(eyre::Report::new(SessionFault::protocol_violation(
  1727	                    format!(
  1728	                        "BlockHashList for '{}' without a held resume need",
  1729	                        list.relative_path
  1730	                    ),
  1731	                ))),
  1732	            }
  1733	        }
  1734	        SourceEvent::NeedComplete => {
  1735	            if *need_complete {
  1736	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1737	                    "duplicate NeedComplete",
  1738	                )));
  1739	            }
  1740	            // Ordered lane: the destination sends every BlockHashList
  1741	            // before its NeedComplete, so a still-held resume need here
  1742	            // means the peer broke the choreography — fail fast rather
  1743	            // than hang waiting for a list that can no longer arrive.
  1744	            if !resume.held.is_empty() {
  1745	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1746	                    format!(
  1747	                        "NeedComplete with {} resume need(s) missing their BlockHashList",
  1748	                        resume.held.len()
  1749	                    ),
  1750	                )));
  1751	            }
  1752	            *need_complete = true;
  1753	            Ok(())
  1754	        }
  1755	        SourceEvent::ResizeAck(ack) => {
  1756	            let dp = data_plane.ok_or_else(|| {
  1757	                eyre::Report::new(SessionFault::protocol_violation(
  1758	                    "DataPlaneResizeAck on a session with no data plane",
  1759	                ))
  1760	            })?;
  1761	            // Match the ack to the in-flight proposal; stale/unsolicited
  1762	            // acks (wrong epoch, or none pending) are ignored, matching
  1763	            // old push. `take()` + restore keeps the borrow simple.
  1764	            let pending_r = match pending_resize.take() {
  1765	                Some(p) if p.epoch == ack.epoch => p,
  1766	                restored => {
  1767	                    *pending_resize = restored;
  1768	                    return Ok(());
  1769	                }
  1770	            };
  1771	            if ack.accepted {
  1772	                dp.add_stream(&pending_r.sub_token).await?;
  1773	                dp.dial()
  1774	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1775	            } else {
  1776	                dp.dial()
  1777	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1778	            }
  1779	            // Ramp one stream per accepted epoch: propose the next ADD.
  1780	            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1781	        }
  1782	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1783	            "TransferSummary before SourceDone",
  1784	        ))),
  1785	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1786	    }
  1787	}
  1788	
  1789	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1790	/// the stream count the accumulated need list implies, if none is in
  1791	/// flight. A no-op when the shape wants no more than the live count (the
  1792	/// dial returns `None`). Sends the frame and records the in-flight
  1793	/// proposal for the ack to match.
  1794	async fn maybe_propose_resize(
  1795	    dp: &data_plane::SourceDataPlane,
  1796	    tx: &mut Box<dyn FrameTx>,
  1797	    needed_bytes: u64,
  1798	    needed_count: usize,
  1799	    pending_resize: &mut Option<data_plane::PendingResize>,
  1800	) -> Result<()> {
  1801	    if pending_resize.is_some() {
  1802	        return Ok(());
  1803	    }
  1804	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1805	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1806	            op: DataPlaneResizeOp::Add as i32,
  1807	            epoch: proposal.epoch,
  1808	            target_stream_count: proposal.target_streams,
  1809	            sub_token: proposal.sub_token.clone(),
  1810	        })))
  1811	        .await?;
  1812	        *pending_resize = Some(proposal);
  1813	    }
  1814	    Ok(())
  1815	}
  1816	
  1817	/// Block for the ack of the one in-flight resize and dial its socket (or
  1818	/// settle it refused). Does NOT propose further — it resolves exactly the
  1819	/// pending proposal so the destination's armed slot is consumed before we
  1820	/// finish the data plane.
  1821	async fn resolve_in_flight_resize(
  1822	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1823	    dp: &data_plane::SourceDataPlane,
  1824	    pending: data_plane::PendingResize,
  1825	) -> Result<()> {
  1826	    loop {
  1827	        match events.recv().await {
  1828	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1829	                if ack.accepted {
  1830	                    dp.add_stream(&pending.sub_token).await?;
  1831	                    dp.dial()
  1832	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1833	                } else {
  1834	                    dp.dial()
  1835	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1836	                }
  1837	                return Ok(());
  1838	            }
  1839	            // A stale ack for an already-settled epoch: ignore, keep
  1840	            // waiting for ours.
  1841	            Some(SourceEvent::ResizeAck(_)) => continue,
  1842	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1843	            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1844	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1845	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1846	                )))
  1847	            }
  1848	            Some(SourceEvent::BlockHashes(l)) => {
  1849	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1850	                    format!(
  2770	    let resume_block_size = {
  2771	        let ceiling = if data_plane_recv.is_some() {
  2772	            MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  2773	        } else {
  2774	            MAX_IN_STREAM_RESUME_BLOCK_SIZE
  2775	        };
  2776	        match negotiated
  2777	            .open
  2778	            .resume
  2779	            .as_ref()
  2780	            .map(|r| r.block_size as usize)
  2781	            .unwrap_or(0)
  2782	        {
  2783	            0 => DEFAULT_BLOCK_SIZE,
  2784	            bs => bs.clamp(MIN_RESUME_BLOCK_SIZE, ceiling),
  2785	        }
  2786	    };
  2787	
  2788	    let mut pending: Vec<FileHeader> = Vec::new();
  2789	    let mut needed_paths: Vec<String> = Vec::new();
  2790	    let mut manifest_complete = false;
  2791	    let mut files_written: u64 = 0;
  2792	    let mut bytes_written: u64 = 0;
  2793	
  2794	    // otp-11: the LOCAL carrier's apply pipeline — spawned before the
  2795	    // loop so applies run concurrent with the diff, exactly as the
  2796	    // data-plane receive does.
  2797	    let mut local_run = local_apply.as_ref().map(|la| la.start(progress.clone()));
  2798	
  2799	    loop {
  2800	        let received = match transport.recv().await? {
  2801	            Some(f) => f,
  2802	            None => {
  2803	                return Err(eyre::Report::new(SessionFault::internal(
  2804	                    "peer closed mid-session",
  2805	                )))
  2806	            }
  2807	        };
  2808	        match received.frame {
  2809	            Some(Frame::ManifestEntry(header)) => {
  2810	                if manifest_complete {
  2811	                    return Err(violation(format!(
  2812	                        "manifest entry '{}' after ManifestComplete",
  2813	                        header.relative_path
  2814	                    )));
  2815	                }
  2816	                // otp-6b: retain the full source path set for the mirror
  2817	                // diff (the need list keeps only files needing transfer).
  2818	                if mirror_enabled {
  2819	                    source_files.insert(header.relative_path.clone());
  2820	                }
  2821	                pending.push(header);
  2822	                if pending.len() >= DEST_DIFF_CHUNK {
  2823	                    let chunk = std::mem::take(&mut pending);
  2824	                    if let Some(la) = &local_apply {
  2825	                        diff_chunk_and_apply_local(
  2826	                            la,
  2827	                            &mut local_run,
  2828	                            chunk,
  2829	                            dst_root,
  2830	                            canonical_dst_root.as_deref(),
  2831	                            &compare_opts,
  2832	                            &mut granted,
  2833	                            &mut needed_paths,
  2834	                            progress.as_ref(),
  2835	                        )
  2836	                        .await?;
  2837	                    } else {
  2838	                        diff_chunk_and_send_needs(
  2839	                            transport,
  2840	                            chunk,
  2841	                            dst_root,
  2842	                            canonical_dst_root.as_deref(),
  2843	                            &compare_opts,
  2844	                            resume_enabled,
  2845	                            resume_block_size,
  2846	                            &resume_headers,
  2847	                            &mut granted,
  2848	                            &outstanding,
  2849	                            &mut needed_paths,
  2850	                            progress.as_ref(),
  2851	                        )
  2852	                        .await?;
  2853	                    }
  2854	                }
  2855	            }
  2856	            Some(Frame::ManifestComplete(complete)) => {
  2857	                if manifest_complete {
  2858	                    return Err(violation("duplicate ManifestComplete".into()));
  2859	                }
  2860	                // otp-6b: mirror deletions are data-loss-dangerous when the
  2861	                // source scan was incomplete — a source file missing from an
  2862	                // aborted scan would be misclassified extraneous and deleted
  2863	                // at the dest. Refuse here (before any transfer or deletion)
  2864	                // rather than partial-mirror. Matches the old paths'
  2865	                // require-complete-scan guard.
  2866	                if mirror_enabled && !complete.scan_complete {
  2867	                    return Err(eyre::Report::new(SessionFault::internal(
  2868	                        "mirror refused: the source scan did not complete \
  2869	                         (unreadable paths) — deleting now could remove files \
  2870	                         the source still has",
  2871	                    )));
  2872	                }
  2873	                // codex otp-9b F1 (R49-F2 on the session): an initiator
  2874	                // that declared "the source will be deleted after this
  2875	                // transfer" (`blit move`) must NOT get a success out of
  2876	                // an incomplete source scan — files the scan could not
  2877	                // read would be silently lost when the caller deletes
  2878	                // the source. Same abort point as the mirror guard.
  2879	                if negotiated.open.require_complete_scan && !complete.scan_complete {
  2880	                    return Err(eyre::Report::new(SessionFault::refusal(
  2881	                        session_error::Code::ScanIncomplete,
  2882	                        "transfer refused: the source scan did not complete \
  2883	                         (unreadable paths) and the operation requires a \
  2884	                         complete scan (move deletes the source afterwards)",
  2885	                    )));
  2886	                }
  2887	                let chunk = std::mem::take(&mut pending);
  2888	                if let Some(la) = &local_apply {
  2889	                    diff_chunk_and_apply_local(
  2890	                        la,
  2891	                        &mut local_run,
  2892	                        chunk,
  2893	                        dst_root,
  2894	                        canonical_dst_root.as_deref(),
  2895	                        &compare_opts,
  2896	                        &mut granted,
  2897	                        &mut needed_paths,
  2898	                        progress.as_ref(),
  2899	                    )
  2900	                    .await?;
  2901	                } else {
  2902	                    diff_chunk_and_send_needs(
  2903	                        transport,
  2904	                        chunk,
  2905	                        dst_root,
  2906	                        canonical_dst_root.as_deref(),
  2907	                        &compare_opts,
  2908	                        resume_enabled,
  2909	                        resume_block_size,
  2910	                        &resume_headers,
  2911	                        &mut granted,
  2912	                        &outstanding,
  2913	                        &mut needed_paths,
  2914	                        progress.as_ref(),
  2915	                    )
  2916	                    .await?;
  2917	                }
  2918	                // NeedComplete only after ManifestComplete received
  2919	                // AND every entry diffed — both true here.
  2920	                transport
  2921	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  2922	                    .await?;
  2923	                manifest_complete = true;
  2924	            }
  2925	            Some(Frame::FileBegin(header)) => {
  2926	                // Payload records ride the control lane only under the
  2927	                // in-stream carrier; with a TCP data plane active they
  2928	                // flow over the sockets, so one here is a violation.
  2929	                if data_plane_recv.is_some() {
  2930	                    return Err(violation(format!(
  2931	                        "file record '{}' on the control lane while a TCP data plane is active",
  2932	                        header.relative_path
  2933	                    )));
  2934	                }
  2935	                if !manifest_complete {
  2936	                    return Err(violation(format!(
  2937	                        "payload record for '{}' before ManifestComplete",
  2938	                        header.relative_path
  2939	                    )));
  2940	                }
  2941	                // A resume-flagged grant may be satisfied ONLY by its
  2942	                // block record — a whole-file record for it bypasses the
  2943	                // hash choreography this end committed to (codex F3).
  2944	                if resume_headers
  2945	                    .lock()
  2946	                    .expect("resume-headers lock poisoned")
  2947	                    .contains_key(&header.relative_path)
  2948	                {
  2949	                    return Err(violation(format!(
  2950	                        "file record for resume-flagged '{}' — the contract requires \
  2951	                         its block record",
  2952	                        header.relative_path
  2953	                    )));
  2954	                }
  2955	                if !outstanding
  2956	                    .lock()
  2957	                    .expect("outstanding-needs lock poisoned")
  2958	                    .remove(&header.relative_path)
  2959	                {
  2960	                    return Err(violation(format!(
  2961	                        "payload for '{}' which is not on the need list",
  2962	                        header.relative_path
  2963	                    )));
  2964	                }
  2965	                let outcome = receive_file_record(transport, sink.as_ref(), &header).await?;
  2966	                files_written += outcome.files_written as u64;
  2967	                bytes_written += outcome.bytes_written;
  2968	                // otp-10b-2: in-stream per-file progress, same convention
  2969	                // as the data-plane receive (`execute_receive_pipeline`):
  2970	                // bytes ride Payload, FileComplete is byteless.
  2971	                if let Some(p) = &progress {
  2972	                    p.report_payload(0, outcome.bytes_written);
  2973	                    p.report_file_complete(header.relative_path.clone());
  2974	                }
  2975	            }
  2976	            Some(Frame::Block(block)) => {
  2977	                // otp-7a: a resume block record opens with its first
  2978	                // BlockTransfer (no begin frame). Claim the need and run
  2979	                // the strictly-serialized record to its completion frame.
  2980	                let header = claim_resume_record(
  2981	                    &block.relative_path,
  2982	                    resume_enabled,
  2983	                    data_plane_recv.is_some(),
  2984	                    manifest_complete,
  2985	                    &resume_headers,
  2986	                    &outstanding,
  2987	                )?;
  2988	                let outcome =
  2989	                    receive_block_record(transport, sink.as_ref(), &header, block).await?;
  2990	                files_written += outcome.files_written as u64;
  2991	                bytes_written += outcome.bytes_written;
  2992	                files_resumed.fetch_add(1, Ordering::Relaxed);
  2993	                // The whole block record (patch bytes + completion) ran
  2994	                // to its completion frame — one resumed file done.
  2995	                if let Some(p) = &progress {
  2996	                    p.report_payload(0, outcome.bytes_written);
  2997	                    p.report_file_complete(header.relative_path.clone());
  2998	                }
  2999	            }
  3000	            Some(Frame::BlockComplete(complete)) => {
  3001	                // otp-7a: a zero-block record — every block matched
  3002	                // (identical content, e.g. an mtime-only touch), so the
  3003	                // completion frame arrives with no blocks before it and
  3004	                // finalization stamps size/mtime/perms.
  3005	                let header = claim_resume_record(
  3006	                    &complete.relative_path,
  3007	                    resume_enabled,
  3008	                    data_plane_recv.is_some(),
  3009	                    manifest_complete,
  3010	                    &resume_headers,
  3011	                    &outstanding,
  3012	                )?;
  3013	                let outcome = finish_block_record(sink.as_ref(), &header, &complete).await?;
  3014	                files_written += outcome.files_written as u64;
  3015	                bytes_written += outcome.bytes_written;
  3016	                files_resumed.fetch_add(1, Ordering::Relaxed);
  3017	                // Zero-block record: nothing transferred, the file is
  3018	                // complete (identical content, metadata stamped).
  3019	                if let Some(p) = &progress {
  3020	                    p.report_file_complete(header.relative_path.clone());
  3021	                }
  3022	            }
  3023	            Some(Frame::TarShardHeader(shard)) => {
  3024	                if data_plane_recv.is_some() {
  3025	                    return Err(violation(
  3026	                        "tar shard record on the control lane while a TCP data plane is active"
  3027	                            .into(),
  3028	                    ));
  3029	                }
  3030	                if !manifest_complete {
  3031	                    return Err(violation("tar shard record before ManifestComplete".into()));
  3032	                }
  3033	                // Same rule as file records (codex F3): a resume-flagged
  3034	                // grant may not be satisfied through a tar shard.
  3035	                {
  3036	                    let held = resume_headers.lock().expect("resume-headers lock poisoned");
  3037	                    for h in &shard.files {
  3038	                        if held.contains_key(&h.relative_path) {
  3039	                            return Err(violation(format!(
  3040	                                "tar shard entry for resume-flagged '{}' — the contract \
  3041	                                 requires its block record",
  3042	                                h.relative_path
  3043	                            )));
  3044	                        }
  3045	                    }
  3046	                }
  3047	                {
  3048	                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  3049	                    for h in &shard.files {
  3050	                        if !out.remove(&h.relative_path) {
  3051	                            return Err(violation(format!(
  3052	                                "tar shard entry '{}' which is not on the need list",
  3053	                                h.relative_path
  3054	                            )));
  3055	                        }
  3056	                    }
  3057	                }
  3058	                // Capture member paths for the per-file progress lane
  3059	                // before the record consumes the shard (the data-plane
  3060	                // receive does the same); skip the allocation when no one
  3061	                // is listening.
  3062	                let member_paths: Option<Vec<String>> = progress.as_ref().map(|_| {
  3063	                    shard
  3064	                        .files
  3065	                        .iter()
  3066	                        .map(|h| h.relative_path.clone())
  3067	                        .collect()
  3068	                });
  3069	                let outcome = receive_tar_record(transport, sink.as_ref(), shard).await?;
  3070	                files_written += outcome.files_written as u64;
  3071	                bytes_written += outcome.bytes_written;
  3072	                if let Some(p) = &progress {
  3073	                    p.report_payload(0, outcome.bytes_written);
  3074	                    for path in member_paths.unwrap_or_default() {
  3075	                        p.report_file_complete(path);
  3076	                    }
  3077	                }
  3078	            }
  3079	            Some(Frame::Resize(resize)) => {
  3080	                // sf-2 shape correction (otp-4b-2 push, otp-5b-2 pull): the
  3081	                // SOURCE proposes one ADD; the DESTINATION grows its receive
  3082	                // set (bump `resize_live`) and acks so the SOURCE completes
  3083	                // the epoch-N socket. The control-lane frames are identical
  3084	                // in both directions — only the transport action flips: a
  3085	                // DESTINATION **responder** (push) ARMS a credential its
  3086	                // accept loop then accepts; a DESTINATION **initiator**
  3087	                // (pull) DIALS the epoch-N socket itself. Only ADD occurs
  3088	                // (REMOVE is a tuner concern, future work); anything else
  3089	                // fails fast.
  3090	                if data_plane_recv.is_none() {
  3091	                    return Err(violation(
  3092	                        "DataPlaneResize on a session with no data plane".into(),
  3093	                    ));
  3094	                }
  3095	                let op = DataPlaneResizeOp::try_from(resize.op)
  3096	                    .unwrap_or(DataPlaneResizeOp::Unspecified);
  3097	                if op != DataPlaneResizeOp::Add {
  3098	                    return Err(violation(format!(
  3099	                        "unsupported data-plane resize op {}",
  3100	                        op.as_str_name()
  3101	                    )));
  3102	                }
  3103	                if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
  3104	                    return Err(violation(
  3105	                        "DataPlaneResize sub_token must be 16 bytes".into(),
  3106	                    ));
  3107	                }
  3108	                // Cumulative ceiling bound (defense in depth — the source's
  3109	                // dial already clamps to the same profile). Under the
  3110	                // ceiling, grow per connection role: arm the credential
  3111	                // (responder) or dial the epoch-N socket (initiator). A
  3112	                // dial failure is fatal (`add_dialed_stream`); a gone accept
  3113	                // loop returns false (arm). The initiator dials BEFORE the
  3114	                // ack so the SOURCE responder — which accepts on the ack —
  3115	                // never commits to an accept the DESTINATION did not dial.
  3116	                let accepted = if resize_live < resize_ceiling {
  3117	                    match data_plane_recv
  3118	                        .as_mut()
  3119	                        .expect("data plane present (checked above)")
  3120	                    {
  3121	                        data_plane::DestRecvPlane::Responder(run) => {
  3122	                            run.arm(resize.sub_token.clone())
  3123	                        }
  3124	                        data_plane::DestRecvPlane::Initiator(run) => {
  3125	                            run.add_dialed_stream(&resize.sub_token).await?;
  3126	                            true
  3127	                        }
  3128	                    }
  3129	                } else {
  3130	                    false
  3131	                };
  3132	                if accepted {
  3133	                    resize_live += 1;
  3134	                }
  3135	                let effective = if accepted {
  3136	                    resize.target_stream_count
  3137	                } else {
  3138	                    resize_live as u32
  3139	                };
  3140	                transport
  3141	                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
  3142	                        epoch: resize.epoch,
  3143	                        effective_stream_count: effective,
  3144	                        accepted,
  3145	                    })))
  3146	                    .await?;
  3147	            }
  3148	            Some(Frame::SourceDone(_)) => {
  3149	                if !manifest_complete {
  3150	                    return Err(violation("SourceDone before ManifestComplete".into()));
  3151	                }
  3152	                // Completion, both carriers: the shared `outstanding`
  3153	                // set must be empty (every granted need claimed exactly
  3154	                // once). In-stream claims inline above; the data-plane
  3155	                // NeedListSink claims as payloads land, so joining the
  3156	                // receive task first drains the last of them (and
  3157	                // surfaces any receive error / stall). Set membership —
  3158	                // not a file count — is the contract (codex F1: a count
  3159	                // proxy let a peer substitute or duplicate paths).
  3160	                // `finish()` drops the arm sender (no more resizes), joins
  3161	                // the accept loop, and reports the settled stream count.
  3162	                //
  3163	                // otp-11: the LOCAL carrier joins its apply pipeline with
  3164	                // the same discipline (drain every write, surface its
  3165	                // error) and takes the write totals as this end's
  3166	                // counters — the scorer stays the destination.
  3167	                if let Some(run) = local_run.take() {
  3168	                    let totals = run.finish().await?;
  3169	                    files_written = totals.files_written as u64;
  3170	                    bytes_written = totals.bytes_written;

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '180,440p'; nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '620,965p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1000,1225p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1280,1430p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1660,1845p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1845,1935p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   180	    pub(super) outcome: SinkOutcome,
   181	    pub(super) streams: usize,
   182	}
   183	
   184	/// Live handle to a running responder data plane. The control loop arms
   185	/// resize credentials through [`Self::arm`] and joins the accept loop at
   186	/// `SourceDone` via [`Self::finish`].
   187	pub(super) struct ResponderDataPlaneRun {
   188	    arm_tx: mpsc::UnboundedSender<Vec<u8>>,
   189	    task: AbortOnDrop<Result<ReceiveTotals>>,
   190	    /// The `session_token` half of every socket credential (the control
   191	    /// loop does not need it, but keeping it here documents the shape).
   192	    #[allow(dead_code)]
   193	    session_token: Vec<u8>,
   194	    /// The receiver's advertised `max_streams` — the control loop refuses
   195	    /// a resize that would grow past it (defense in depth; the source's
   196	    /// dial already clamps to the same ceiling).
   197	    pub(super) ceiling: usize,
   198	}
   199	
   200	impl ResponderDataPlane {
   201	    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
   202	    pub(super) fn grant(&self) -> DataPlaneGrant {
   203	        DataPlaneGrant {
   204	            tcp_port: self.port as u32,
   205	            session_token: self.session_token.clone(),
   206	            initial_streams: self.initial_streams,
   207	            epoch0_sub_token: self.epoch0_sub_token.clone(),
   208	        }
   209	    }
   210	
   211	    /// The epoch-0 stream count this responder granted (always 1 — the
   212	    /// zero-knowledge proposal). The control loop seeds its `resize_live`
   213	    /// counter from it.
   214	    pub(super) fn initial_streams(&self) -> u32 {
   215	        self.initial_streams
   216	    }
   217	
   218	    /// Spawn the accept+receive loop and return a live handle. The loop
   219	    /// accepts the epoch-0 socket(s) immediately, then accepts one more
   220	    /// socket per armed resize credential until the control loop signals
   221	    /// `SourceDone` (drops the arm sender) and every receive worker has
   222	    /// drained its END. Runs concurrently with the control-stream diff
   223	    /// loop; the DESTINATION is the scorer, so it returns the totals.
   224	    pub(super) fn spawn(
   225	        self,
   226	        sink: Arc<dyn TransferSink>,
   227	        progress: Option<RemoteTransferProgress>,
   228	    ) -> ResponderDataPlaneRun {
   229	        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   230	        let session_token = self.session_token.clone();
   231	        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<Vec<u8>>();
   232	        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(sink, progress, arm_rx)));
   233	        ResponderDataPlaneRun {
   234	            arm_tx,
   235	            task,
   236	            session_token,
   237	            ceiling,
   238	        }
   239	    }
   240	
   241	    async fn accept_loop(
   242	        self,
   243	        sink: Arc<dyn TransferSink>,
   244	        progress: Option<RemoteTransferProgress>,
   245	        arm_rx: mpsc::UnboundedReceiver<Vec<u8>>,
   246	    ) -> Result<ReceiveTotals> {
   247	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   248	        let mut epoch0 = self.session_token.clone();
   249	        epoch0.extend_from_slice(&self.epoch0_sub_token);
   250	
   251	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   252	        let mut total = SinkOutcome::default();
   253	        let mut streams = 0usize;
   254	
   255	        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
   256	        // grant is always 1; the loop handles N for symmetry).
   257	        for _ in 0..self.initial_streams {
   258	            let socket = accept_authenticated(&self.listener, &epoch0).await?;
   259	            streams += 1;
   260	            spawn_receive(&mut receives, socket, &sink, progress.clone());
   261	        }
   262	
   263	        // Resize ADDs: each arms a `session_token ‖ sub_token` credential
   264	        // whose socket the SOURCE dials right after its ack. `no_more` is
   265	        // set when the control loop drops the arm sender at `SourceDone`;
   266	        // the loop then drains the last armed sockets and workers. Because
   267	        // the SOURCE only dials a credential it was acked for (and a dial
   268	        // failure faults the whole session, aborting this task via
   269	        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
   270	        let mut armed: Vec<Vec<u8>> = Vec::new();
   271	        let mut arm_rx = Some(arm_rx);
   272	        let mut no_more = false;
   273	        loop {
   274	            if no_more && armed.is_empty() && receives.is_empty() {
   275	                break;
   276	            }
   277	            // A closed arm channel resolves `recv()` instantly to `None`
   278	            // every poll; parking it on `pending()` once closed keeps the
   279	            // biased select from starving the accept/join arms (otherwise
   280	            // the None arm wins every race and the loop spins without ever
   281	            // collecting a finished worker).
   282	            let arm_recv = async {
   283	                match arm_rx.as_mut() {
   284	                    Some(rx) => rx.recv().await,
   285	                    None => std::future::pending().await,
   286	                }
   287	            };
   288	            tokio::select! {
   289	                biased;
   290	                // Control FIRST: an arm must register before its socket
   291	                // (which the SOURCE dials only after the ack the control
   292	                // loop sends right after arming), so the accept arm below
   293	                // always sees a populated `armed` set.
   294	                arm = arm_recv => match arm {
   295	                    Some(sub_token) => armed.push(sub_token),
   296	                    // Arm sender dropped at SourceDone: no more resizes.
   297	                    None => {
   298	                        arm_rx = None;
   299	                        no_more = true;
   300	                    }
   301	                },
   302	                // Accept only when a resize credential is armed. `accept`
   303	                // is cancel-safe, so losing this arm to another (its
   304	                // pending connection stays queued) drops no socket. The
   305	                // credential read happens OUTSIDE the select (below) so a
   306	                // select cancel can never truncate a half-read socket.
   307	                accepted = accept_raw(&self.listener), if !armed.is_empty() => {
   308	                    let socket = accepted?;
   309	                    let socket =
   310	                        authenticate_resize(socket, &self.session_token, &mut armed).await?;
   311	                    streams += 1;
   312	                    spawn_receive(&mut receives, socket, &sink, progress.clone());
   313	                }
   314	                joined = receives.join_next(), if !receives.is_empty() => {
   315	                    let outcome = joined
   316	                        .expect("join_next is None only when empty, guarded above")
   317	                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   318	                    total.files_written += outcome.files_written;
   319	                    total.bytes_written += outcome.bytes_written;
   320	                }
   321	            }
   322	        }
   323	        Ok(ReceiveTotals {
   324	            outcome: total,
   325	            streams,
   326	        })
   327	    }
   328	}
   329	
   330	impl ResponderDataPlaneRun {
   331	    /// Arm a resize credential so the next socket presenting
   332	    /// `session_token ‖ sub_token` is accepted. Returns false if the
   333	    /// accept loop is gone (its receiver dropped) — the control loop then
   334	    /// acks the resize as refused.
   335	    pub(super) fn arm(&self, sub_token: Vec<u8>) -> bool {
   336	        self.arm_tx.send(sub_token).is_ok()
   337	    }
   338	
   339	    /// Signal `SourceDone` (no more resizes) and join the accept loop for
   340	    /// the aggregated receive totals.
   341	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   342	        let ResponderDataPlaneRun { arm_tx, task, .. } = self;
   343	        // Dropping the arm sender is the "no more resizes" signal.
   344	        drop(arm_tx);
   345	        task.join()
   346	            .await
   347	            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")))?
   348	    }
   349	}
   350	
   351	/// Spawn one receive worker draining `socket` into `sink` via the shared
   352	/// receive pipeline, guarded by the transfer stall timeout (carried REV4
   353	/// RELIABLE invariant, matching the old push receive: a peer that
   354	/// authenticates then stalls mid-record trips the stall timeout rather
   355	/// than pinning the task until TCP keepalive).
   356	fn spawn_receive(
   357	    receives: &mut JoinSet<Result<SinkOutcome>>,
   358	    socket: TcpStream,
   359	    sink: &Arc<dyn TransferSink>,
   360	    progress: Option<RemoteTransferProgress>,
   361	) {
   362	    let sink = Arc::clone(sink);
   363	    receives.spawn(async move {
   364	        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
   365	        execute_receive_pipeline(&mut guarded, sink, progress.as_ref()).await
   366	    });
   367	}
   368	
   369	/// Accept one data socket under the shared bounded-accept timeout and
   370	/// apply the data-plane socket policy. Cancel-safe (the accept itself is;
   371	/// no bytes are read here).
   372	async fn accept_raw(listener: &TcpListener) -> Result<TcpStream> {
   373	    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
   374	    let socket = match accept {
   375	        Ok(Ok((socket, _peer))) => socket,
   376	        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
   377	        Err(_) => {
   378	            return Err(dp_fault(format!(
   379	            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
   380	        )))
   381	        }
   382	    };
   383	    configure_data_socket(&socket, None)
   384	        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
   385	    Ok(socket)
   386	}
   387	
   388	/// Read the fixed-length epoch-0 credential and verify it whole. A socket
   389	/// presenting anything else is a `DATA_PLANE_FAILED` fault (the session
   390	/// arms exactly the sockets it dials, so a mismatch is fatal here).
   391	async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
   392	    let mut socket = accept_raw(listener).await?;
   393	    let mut buf = vec![0u8; expected.len()];
   394	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   395	    match read {
   396	        Ok(Ok(_)) => {}
   397	        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
   398	        Err(_) => {
   399	            return Err(dp_fault(format!(
   400	                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   401	            )))
   402	        }
   403	    }
   404	    // Constant-time comparison is not required: the tokens are 16 random
   405	    // bytes read once per socket, single-session; a timing oracle buys
   406	    // nothing against per-transfer secrets (same posture as the old push
   407	    // acceptor's `token == expected_token`).
   408	    if buf != expected {
   409	        return Err(dp_fault(
   410	            "data-plane socket presented an invalid credential",
   411	        ));
   412	    }
   413	    Ok(socket)
   414	}
   415	
   416	/// Read a resize socket's `session_token ‖ sub_token(16)` credential
   417	/// (bounded), verify the session token, and match the sub-token against
   418	/// an armed credential — removing it so each arm is consumed once. Runs
   419	/// in the accept loop body (never a select arm), so a select cancel can
   420	/// never truncate a half-read socket.
   421	async fn authenticate_resize(
   422	    socket: TcpStream,
   423	    session_token: &[u8],
   424	    armed: &mut Vec<Vec<u8>>,
   425	) -> Result<TcpStream> {
   426	    let mut socket = socket;
   427	    let mut buf = vec![0u8; session_token.len() + SUB_TOKEN_LEN];
   428	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   429	    match read {
   430	        Ok(Ok(_)) => {}
   431	        Ok(Err(err)) => {
   432	            return Err(dp_fault(format!(
   433	                "reading resize data-plane credential: {err}"
   434	            )))
   435	        }
   436	        Err(_) => {
   437	            return Err(dp_fault(format!(
   438	                "resize data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   439	            )))
   440	        }
   620	// ---------------------------------------------------------------------------
   621	// Initiator (SOURCE) — dial, authenticate, send, resize
   622	// ---------------------------------------------------------------------------
   623	
   624	/// A resize the SOURCE has proposed and minted a credential for but not
   625	/// yet completed: the driver has sent (or will send) the matching
   626	/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
   627	/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
   628	/// flight (the dial's `pending_epoch` enforces it; this is the
   629	/// driver-side record the ack is matched against).
   630	pub(super) struct PendingResize {
   631	    pub(super) epoch: u32,
   632	    pub(super) target_streams: u32,
   633	    pub(super) sub_token: Vec<u8>,
   634	}
   635	
   636	/// How the SOURCE acquires each epoch-N data socket for a shape resize —
   637	/// the two connection roles of otp-5b. Byte direction is identical (the
   638	/// SOURCE sends), and `propose_resize` is the same either way; only socket
   639	/// acquisition flips.
   640	enum SourceSockets {
   641	    /// SOURCE **initiator** (push, otp-4b-2): dials each epoch-N socket to
   642	    /// the granted host:port.
   643	    Dial { host: String, tcp_port: u32 },
   644	    /// SOURCE **responder** (pull, otp-5b-2): accepts each epoch-N socket
   645	    /// off the listener it already bound for epoch-0, credential
   646	    /// `session_token ‖ sub_token`.
   647	    Accept { listener: TcpListener },
   648	}
   649	
   650	/// A running source-side data plane: the dialed/accepted socket(s) wrapped
   651	/// as an ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the
   652	/// sf-2 shape correction). Planned payloads are fed via [`Self::queue`];
   653	/// closing via [`Self::finish`] drains the pipeline, emits each socket's
   654	/// END record, and returns the bytes this end sent.
   655	pub(super) struct SourceDataPlane {
   656	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   657	    control_tx: mpsc::UnboundedSender<SinkControl>,
   658	    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
   659	    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
   660	    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
   661	    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
   662	    // advertised capacity (contract §Invariants 5). The resize drives only
   663	    // its shape-correction stream count; the cheap-dial tuner is future
   664	    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
   665	    dial: Arc<TransferDial>,
   666	    source: Arc<dyn TransferSource>,
   667	    session_token: Vec<u8>,
   668	    pool: Arc<BufferPool>,
   669	    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
   670	    /// otp-10a). Applied to the epoch-0 sockets at construction and to
   671	    /// each epoch-N resize socket in [`Self::add_stream`].
   672	    trace: bool,
   673	    /// How each epoch-N resize socket is acquired (dial for the SOURCE
   674	    /// initiator, accept for the SOURCE responder). The data plane grows
   675	    /// mid-transfer in both cases; the control-lane resize choreography is
   676	    /// identical — only this transport action flips (otp-5b-2).
   677	    sockets: SourceSockets,
   678	}
   679	
   680	/// Dial the granted data plane and start the elastic send pipeline.
   681	/// `host` is the responder's host (the initiator connected the control
   682	/// plane to it; the data plane rides the same host on the granted port —
   683	/// contract §Transport: the initiator always dials). `receiver_capacity`
   684	/// is the DESTINATION's advertised profile from `SessionAccept`; it
   685	/// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
   686	/// never unlimited).
   687	pub(super) async fn dial_source_data_plane(
   688	    host: &str,
   689	    grant: &DataPlaneGrant,
   690	    receiver_capacity: Option<&CapacityProfile>,
   691	    source: Arc<dyn TransferSource>,
   692	    instruments: &SourceInstruments,
   693	) -> Result<SourceDataPlane> {
   694	    let initial = grant.initial_streams.max(1) as usize;
   695	    // The byte sender's dial, bounded by the receiver's advertised
   696	    // capacity. Seed the settled live count to the granted epoch-0
   697	    // streams — every shape-resize proposal steps from here.
   698	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   699	    dial.set_negotiated_streams(initial);
   700	
   701	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   702	    let mut handshake = grant.session_token.clone();
   703	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   704	
   705	    // Provision the pool for the dial ceiling so resize-added sockets
   706	    // draw buffers from the same pool without re-pooling (as old push
   707	    // does — a shared pool sized for the maximum stream count).
   708	    let pool = Arc::new(BufferPool::for_data_plane(
   709	        dial.chunk_bytes(),
   710	        dial.ceiling_max_streams().max(1),
   711	    ));
   712	    let trace = instruments.trace_data_plane;
   713	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   714	    for _ in 0..initial {
   715	        let session = DataPlaneSession::connect(
   716	            host,
   717	            grant.tcp_port,
   718	            &handshake,
   719	            dial.chunk_bytes(),
   720	            dial.prefetch_count(),
   721	            trace,
   722	            dial.tcp_buffer_bytes(),
   723	            Arc::clone(&pool),
   724	        )
   725	        .await
   726	        .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?;
   727	        // The source-side sink never reads its dst_root (it only sends);
   728	        // `root()` is consulted by the relay/receive case, not here.
   729	        sinks.push(Arc::new(DataPlaneSink::new(
   730	            session,
   731	            Arc::clone(&source),
   732	            PathBuf::new(),
   733	        )));
   734	    }
   735	
   736	    let prefetch = dial.prefetch_count().max(1);
   737	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   738	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   739	    let pipe_source = Arc::clone(&source);
   740	    let pipe_progress = instruments.progress.clone();
   741	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   742	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   743	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   744	        execute_sink_pipeline_elastic(
   745	            pipe_source,
   746	            sinks,
   747	            payload_rx,
   748	            prefetch,
   749	            pipe_progress.as_ref(),
   750	            Some(control_rx),
   751	        )
   752	        .await
   753	    }));
   754	    Ok(SourceDataPlane {
   755	        payload_tx: Some(payload_tx),
   756	        control_tx,
   757	        pipeline: Some(pipeline),
   758	        dial,
   759	        source,
   760	        session_token: grant.session_token.clone(),
   761	        pool,
   762	        trace,
   763	        // SOURCE initiator: each epoch-N resize socket is dialed to the
   764	        // granted host:port.
   765	        sockets: SourceSockets::Dial {
   766	            host: host.to_string(),
   767	            tcp_port: grant.tcp_port,
   768	        },
   769	    })
   770	}
   771	
   772	/// Accept the granted epoch-0 socket(s) off a bound responder listener and
   773	/// start the elastic SEND pipeline over them — the SOURCE **responder**
   774	/// half of the pull data plane (otp-5b-1). Symmetric with
   775	/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
   776	/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
   777	/// socket acquisition differs (accept here, dial there).
   778	/// `DataPlaneSession::from_stream` builds a send session from an already-
   779	/// accepted socket — the same primitive the old `pull_sync` daemon-send
   780	/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
   781	/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
   782	/// wherever it initiates). The bound listener is retained so each epoch-N
   783	/// resize socket is accepted off it (otp-5b-2): the DESTINATION initiator
   784	/// dials, this end accepts, the control-lane frames identical to push.
   785	pub(super) async fn accept_source_data_plane(
   786	    bound: ResponderDataPlane,
   787	    receiver_capacity: Option<&CapacityProfile>,
   788	    source: Arc<dyn TransferSource>,
   789	    instruments: &SourceInstruments,
   790	) -> Result<SourceDataPlane> {
   791	    let initial = bound.initial_streams.max(1) as usize;
   792	    // The byte sender's dial, bounded by the receiver's advertised
   793	    // capacity; seed the live count to the granted epoch-0 streams. Growth
   794	    // is via resize (otp-5b-2): the accept-based epoch-N socket steps from
   795	    // here, one stream per epoch, same as the SOURCE initiator.
   796	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   797	    dial.set_negotiated_streams(initial);
   798	
   799	    // Epoch-0 credential the dialing DESTINATION presents:
   800	    // session_token ‖ epoch0_sub_token (contract §Transport).
   801	    let mut epoch0 = bound.session_token.clone();
   802	    epoch0.extend_from_slice(&bound.epoch0_sub_token);
   803	
   804	    let pool = Arc::new(BufferPool::for_data_plane(
   805	        dial.chunk_bytes(),
   806	        dial.ceiling_max_streams().max(1),
   807	    ));
   808	    let trace = instruments.trace_data_plane;
   809	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   810	    for _ in 0..initial {
   811	        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
   812	        let session = DataPlaneSession::from_stream(
   813	            socket,
   814	            trace,
   815	            dial.chunk_bytes(),
   816	            dial.prefetch_count(),
   817	            Arc::clone(&pool),
   818	        )
   819	        .await;
   820	        sinks.push(Arc::new(DataPlaneSink::new(
   821	            session,
   822	            Arc::clone(&source),
   823	            PathBuf::new(),
   824	        )));
   825	    }
   826	
   827	    let prefetch = dial.prefetch_count().max(1);
   828	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   829	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   830	    let pipe_source = Arc::clone(&source);
   831	    let pipe_progress = instruments.progress.clone();
   832	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   833	        execute_sink_pipeline_elastic(
   834	            pipe_source,
   835	            sinks,
   836	            payload_rx,
   837	            prefetch,
   838	            pipe_progress.as_ref(),
   839	            Some(control_rx),
   840	        )
   841	        .await
   842	    }));
   843	    Ok(SourceDataPlane {
   844	        payload_tx: Some(payload_tx),
   845	        control_tx,
   846	        pipeline: Some(pipeline),
   847	        dial,
   848	        source,
   849	        session_token: bound.session_token,
   850	        pool,
   851	        trace,
   852	        // SOURCE responder: each epoch-N resize socket is accepted off the
   853	        // same listener epoch-0 came in on (otp-5b-2).
   854	        sockets: SourceSockets::Accept {
   855	            listener: bound.listener,
   856	        },
   857	    })
   858	}
   859	
   860	impl SourceDataPlane {
   861	    /// The live dial (the byte sender owns it). The driver reads
   862	    /// `live_streams()` for observability and calls `resize_settled` as
   863	    /// each proposal completes.
   864	    pub(super) fn dial(&self) -> &Arc<TransferDial> {
   865	        &self.dial
   866	    }
   867	
   868	    /// sf-2 shape correction: propose one ADD toward the stream count the
   869	    /// accumulated need list implies, if none is in flight and the shape
   870	    /// wants more than the current live count. Mints the resize
   871	    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
   872	    /// the record back on the matching ack.
   873	    pub(super) fn propose_resize(
   874	        &self,
   875	        needed_bytes: u64,
   876	        needed_count: usize,
   877	    ) -> Result<Option<PendingResize>> {
   878	        let desired =
   879	            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
   880	                as usize;
   881	        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
   882	            return Ok(None);
   883	        };
   884	        let sub_token = generate_sub_token()
   885	            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
   886	        Ok(Some(PendingResize {
   887	            epoch: proposal.epoch,
   888	            target_streams: proposal.target_streams as u32,
   889	            sub_token,
   890	        }))
   891	    }
   892	
   893	    /// Acquire the epoch-N data socket for an accepted resize and hand it
   894	    /// to the running pipeline (`SinkControl::Add`). The SOURCE initiator
   895	    /// (push) DIALS it; the SOURCE responder (pull, otp-5b-2) ACCEPTS the
   896	    /// socket the DESTINATION initiator dials after its ack, off the same
   897	    /// listener epoch-0 came in on. A dial/accept failure is FATAL
   898	    /// (fail-fast): a same-build peer that established epoch-0 failing an
   899	    /// epoch-N socket is a transport fault worth surfacing — and faulting
   900	    /// the session aborts the peer's counterpart via AbortOnDrop, so no
   901	    /// slot orphans. (Old push recovers non-fatally via an arm TTL; the
   902	    /// session trades that for simplicity — noted in the finding doc.) If
   903	    /// the pipeline is already gone (transfer completing under the ADD),
   904	    /// the just-acquired socket is closed cleanly so the peer's worker sees
   905	    /// its END, not a reset.
   906	    ///
   907	    /// The accept is bounded and unambiguous: at most one resize is in
   908	    /// flight (the driver's `pending_resize`) and epoch-0 is already
   909	    /// accepted, so the next connection off the listener is exactly this
   910	    /// resize's socket — verified against `session_token ‖ sub_token`.
   911	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   912	        let session = match &self.sockets {
   913	            SourceSockets::Dial { host, tcp_port } => {
   914	                let mut handshake = self.session_token.clone();
   915	                handshake.extend_from_slice(sub_token);
   916	                DataPlaneSession::connect(
   917	                    host,
   918	                    *tcp_port,
   919	                    &handshake,
   920	                    self.dial.chunk_bytes(),
   921	                    self.dial.prefetch_count(),
   922	                    self.trace,
   923	                    self.dial.tcp_buffer_bytes(),
   924	                    Arc::clone(&self.pool),
   925	                )
   926	                .await
   927	                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?
   928	            }
   929	            SourceSockets::Accept { listener } => {
   930	                let mut expected = self.session_token.clone();
   931	                expected.extend_from_slice(sub_token);
   932	                let socket = accept_authenticated(listener, &expected).await?;
   933	                DataPlaneSession::from_stream(
   934	                    socket,
   935	                    self.trace,
   936	                    self.dial.chunk_bytes(),
   937	                    self.dial.prefetch_count(),
   938	                    Arc::clone(&self.pool),
   939	                )
   940	                .await
   941	            }
   942	        };
   943	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   944	            session,
   945	            Arc::clone(&self.source),
   946	            PathBuf::new(),
   947	        ));
   948	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   949	            if let SinkControl::Add(sink) = returned.0 {
   950	                let _ = sink.finish().await;
   951	            }
   952	        }
   953	        Ok(())
   954	    }
   955	
   956	    /// Feed one planned batch into the send pipeline. The pipeline
   957	    /// prepares each payload (tar-shard/file) and writes it through the
   958	    /// data-plane record framing across the live socket(s).
   959	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   960	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   961	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   962	        })?;
   963	        for payload in payloads {
   964	            tx.send(payload).await.map_err(|_| {
   965	                dp_fault("data-plane send pipeline closed before all payloads sent")
  1000	        // run_source only ever resolves nothing: a SOURCE *initiator*
  1001	        // owns its own root, and a SOURCE *responder* driven directly
  1002	        // (the in-process role suite) is handed a Fixed source. The
  1003	        // daemon SOURCE responder resolves module→root inside
  1004	        // `run_responder`, not here (otp-5).
  1005	        None,
  1006	    )
  1007	    .await?;
  1008	
  1009	    drive_source(
  1010	        cfg.plan_options,
  1011	        cfg.data_plane_host,
  1012	        cfg.instruments,
  1013	        negotiated,
  1014	        transport,
  1015	        source,
  1016	    )
  1017	    .await
  1018	}
  1019	
  1020	/// The SOURCE session body after establish: spawn the receive half,
  1021	/// run the send half, and map a fault to a peer-notified report. Shared
  1022	/// by [`run_source`] (initiator or direct-responder) and
  1023	/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
  1024	/// choreography is single-sourced.
  1025	async fn drive_source(
  1026	    plan_options: PlanOptions,
  1027	    data_plane_host: Option<String>,
  1028	    instruments: SourceInstruments,
  1029	    mut negotiated: Negotiated,
  1030	    transport: FrameTransport,
  1031	    source: Arc<dyn TransferSource>,
  1032	) -> Result<TransferSummary> {
  1033	    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
  1034	    // its send sockets on; a SOURCE initiator (push) has none and dials the
  1035	    // grant it received instead. Take it here so the send half owns it.
  1036	    let responder_data_plane = negotiated.responder_data_plane.take();
  1037	    let (mut tx, rx) = transport.split();
  1038	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
  1039	    // Set by the send half the moment ManifestComplete goes out. On
  1040	    // an ordered transport, a NeedComplete arriving while this is
  1041	    // still false is provably premature — the peer cannot have
  1042	    // received what we have not sent (contract: NeedComplete only
  1043	    // after ManifestComplete received + all entries diffed).
  1044	    let manifest_sent = Arc::new(AtomicBool::new(false));
  1045	    let (event_tx, event_rx) = mpsc::unbounded_channel();
  1046	    // Fault side-channel (codex otp-8 F1): the in-stream send path
  1047	    // races this signal against blocked record sends; see
  1048	    // `SourceEventSender`.
  1049	    let (fault_tx, fault_rx) = watch::channel(None::<SessionFault>);
  1050	    // AbortOnDrop: an early error return below must abort the receive
  1051	    // half instead of leaking it (same rationale as design-2 / w4-1).
  1052	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
  1053	        rx,
  1054	        Arc::clone(&sent),
  1055	        Arc::clone(&manifest_sent),
  1056	        resume_negotiated(&negotiated.open),
  1057	        // otp-10a: the recv half owns need-batch arrival, which is the
  1058	        // push-direction progress denominator (contract on
  1059	        // `ProgressEvent::ManifestBatch`: "push: need-list batches").
  1060	        instruments.progress.clone(),
  1061	        SourceEventSender {
  1062	            tx: event_tx,
  1063	            fault_signal: fault_tx,
  1064	        },
  1065	    )));
  1066	
  1067	    match source_send_half(
  1068	        plan_options,
  1069	        data_plane_host.as_deref(),
  1070	        instruments,
  1071	        &negotiated,
  1072	        responder_data_plane,
  1073	        &mut tx,
  1074	        source,
  1075	        sent,
  1076	        &manifest_sent,
  1077	        event_rx,
  1078	        fault_rx,
  1079	    )
  1080	    .await
  1081	    {
  1082	        Ok(summary) => Ok(summary),
  1083	        Err(report) => {
  1084	            let mut fault = fault_from_report(report);
  1085	            if !fault.peer_notified {
  1086	                let _ = tx.send(error_frame(&fault)).await;
  1087	                fault.peer_notified = true;
  1088	            }
  1089	            Err(eyre::Report::new(fault))
  1090	        }
  1091	    }
  1092	}
  1093	
  1094	/// Receive half of the source driver: drains the transport for the
  1095	/// whole session so destination sends can never deadlock against a
  1096	/// blocked source send, and routes the destination lane to the send
  1097	/// half. Terminates on summary, error, close, or violation.
  1098	async fn source_recv_half(
  1099	    mut rx: Box<dyn FrameRx>,
  1100	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
  1101	    manifest_sent: Arc<AtomicBool>,
  1102	    resume_session: bool,
  1103	    progress: Option<RemoteTransferProgress>,
  1104	    events: SourceEventSender,
  1105	) {
  1106	    loop {
  1107	        let received = match rx.recv().await {
  1108	            Ok(Some(f)) => f,
  1109	            Ok(None) => {
  1110	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
  1111	                    "peer closed before TransferSummary",
  1112	                )));
  1113	                return;
  1114	            }
  1115	            Err(err) => {
  1116	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
  1117	                    "transport receive failed: {err:#}"
  1118	                ))));
  1119	                return;
  1120	            }
  1121	        };
  1122	        match received.frame {
  1123	            Some(Frame::NeedBatch(batch)) => {
  1124	                // otp-10a: the need list is the push-direction progress
  1125	                // denominator ("N of M files"). Entries are unique by
  1126	                // contract (a duplicate need faults below), so every
  1127	                // batch is newly-requested work — same semantics as the
  1128	                // old push driver's `report_manifest_batch`.
  1129	                if let Some(p) = &progress {
  1130	                    if !batch.entries.is_empty() {
  1131	                        p.report_manifest_batch(batch.entries.len());
  1132	                    }
  1133	                }
  1134	                for entry in batch.entries {
  1135	                    if entry.resume && !resume_session {
  1136	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1137	                            format!(
  1138	                                "resume-flagged need for '{}' in a session opened without resume",
  1139	                                entry.relative_path
  1140	                            ),
  1141	                        )));
  1142	                        return;
  1143	                    }
  1144	                    let header = sent
  1145	                        .lock()
  1146	                        .expect("sent-manifest lock poisoned")
  1147	                        .remove(&entry.relative_path);
  1148	                    match header {
  1149	                        Some(h) if entry.resume => {
  1150	                            let _ = events.send(SourceEvent::ResumeNeed(h));
  1151	                        }
  1152	                        Some(h) => {
  1153	                            let _ = events.send(SourceEvent::Need(h));
  1154	                        }
  1155	                        None => {
  1156	                            let _ = events.send(SourceEvent::Fault(
  1157	                                SessionFault::protocol_violation(format!(
  1158	                                    "need for unknown or already-needed path '{}'",
  1159	                                    entry.relative_path
  1160	                                )),
  1161	                            ));
  1162	                            return;
  1163	                        }
  1164	                    }
  1165	                }
  1166	            }
  1167	            Some(Frame::BlockHashes(list)) => {
  1168	                // otp-7a: the destination's hashes for a resume-flagged
  1169	                // need. The send half correlates it with the held need;
  1170	                // in a non-resume session the frame is off-contract.
  1171	                if !resume_session {
  1172	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1173	                        format!(
  1174	                            "BlockHashList for '{}' in a session opened without resume",
  1175	                            list.relative_path
  1176	                        ),
  1177	                    )));
  1178	                    return;
  1179	                }
  1180	                let _ = events.send(SourceEvent::BlockHashes(list));
  1181	            }
  1182	            Some(Frame::NeedComplete(_)) => {
  1183	                if !manifest_sent.load(Ordering::Acquire) {
  1184	                    // Fail fast at arrival time (otp-3 codex F2): the
  1185	                    // event queue would otherwise let an early
  1186	                    // NeedComplete be processed late and pass as
  1187	                    // legitimate.
  1188	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1189	                        "NeedComplete before the source's ManifestComplete",
  1190	                    )));
  1191	                    return;
  1192	                }
  1193	                let _ = events.send(SourceEvent::NeedComplete);
  1194	            }
  1195	            Some(Frame::ResizeAck(ack)) => {
  1196	                // The destination's response to a shape-resize proposal
  1197	                // (otp-4b-2). Forward it to the send half, which owns the
  1198	                // dial and dials the epoch-N socket on `accepted`.
  1199	                let _ = events.send(SourceEvent::ResizeAck(ack));
  1200	            }
  1201	            Some(Frame::Summary(summary)) => {
  1202	                let _ = events.send(SourceEvent::Summary(summary));
  1203	                return;
  1204	            }
  1205	            Some(Frame::Error(err)) => {
  1206	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
  1207	                return;
  1208	            }
  1209	            other => {
  1210	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1211	                    format!("{} on the source's receive lane", frame_name(&other)),
  1212	                )));
  1213	                return;
  1214	            }
  1215	        }
  1216	    }
  1217	}
  1218	
  1219	/// otp-7a: the send half's resume bookkeeping. A resume-flagged need is
  1220	/// HELD until its `BlockHashList` arrives (the contract's strict
  1221	/// ordering — the source must not send a byte of that file first); the
  1222	/// correlated pair then queues for the block phase.
  1223	#[derive(Default)]
  1224	struct ResumeSendState {
  1225	    held: HashMap<String, FileHeader>,
  1280	                    data_plane::dial_source_data_plane(
  1281	                        host,
  1282	                        grant,
  1283	                        negotiated.accept.receiver_capacity.as_ref(),
  1284	                        Arc::clone(&source),
  1285	                        &instruments,
  1286	                    )
  1287	                    .await?,
  1288	                )
  1289	            }
  1290	            None => None,
  1291	        },
  1292	    };
  1293	
  1294	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
  1295	    // fed to the shape table so the SOURCE grows the data-plane stream
  1296	    // count as the workload's shape becomes known. Append-only (a need is
  1297	    // counted once, when it arrives), and the in-flight resize record the
  1298	    // ack is matched against (at most one — the dial enforces it).
  1299	    let mut needed_bytes: u64 = 0;
  1300	    let mut needed_count: usize = 0;
  1301	    let mut pending_resize: Option<data_plane::PendingResize> = None;
  1302	
  1303	    // Streaming manifest: entries go out as enumeration produces them
  1304	    // (immediate start in every direction — plan §Design 2). The open
  1305	    // carries no source path (the source end owns its local endpoint) but
  1306	    // does carry the include/exclude/size/age filter (otp-6a): only
  1307	    // matching files are manifested and transferred. The filter MUST ride
  1308	    // the wire (not be pre-wrapped by a local caller) because for pull the
  1309	    // SOURCE is the remote daemon responder — it, not the client, owns the
  1310	    // scan. Apply it through the universal `FilteredSource` decorator, the
  1311	    // single filter chokepoint every source impl routes through, rather
  1312	    // than the per-impl `scan(filter)` arg — a source impl is free to
  1313	    // ignore that arg (the since-deleted relay source did; codex otp-6a
  1314	    // F1), and the chokepoint makes filtering independent of it. A
  1315	    // default/absent filter scans everything (unchanged from otp-3). Globs
  1316	    // were validated at OPEN (`source_open_validator`), so the conversion
  1317	    // cannot fail on a validated open; map any error to a fault regardless.
  1318	    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
  1319	        Some(spec) if *spec != FilterSpec::default() => {
  1320	            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  1321	                .map_err(|e| {
  1322	                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
  1323	                })?;
  1324	            Arc::new(crate::remote::transfer::source::FilteredSource::new(
  1325	                Arc::clone(&source),
  1326	                filter,
  1327	            ))
  1328	        }
  1329	        _ => Arc::clone(&source),
  1330	    };
  1331	    // otp-10b-1: a Checksum session fills each manifest header's
  1332	    // checksum so the DESTINATION can skip content-equal files
  1333	    // regardless of mtime. Wrapped OUTSIDE the filter so only
  1334	    // in-scope files pay the hash; a serving end that refuses to hash
  1335	    // never gets here (CHECKSUM_DISABLED at OPEN).
  1336	    let scan_source: Arc<dyn TransferSource> =
  1337	        if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
  1338	            Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
  1339	                scan_source,
  1340	            ))
  1341	        } else {
  1342	            scan_source
  1343	        };
  1344	    // otp-10a: callers that must not treat a partial transfer as success
  1345	    // (the push verb, `blit move`'s source-delete gate) supply their own
  1346	    // accumulator via `SourceInstruments` and inspect it after the
  1347	    // session returns; the wire behavior is identical either way.
  1348	    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
  1349	    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
  1350	    while let Some(header) = header_rx.recv().await {
  1351	        sent.lock()
  1352	            .expect("sent-manifest lock poisoned")
  1353	            .insert(header.relative_path.clone(), header.clone());
  1354	        tx.send(frame(Frame::ManifestEntry(header))).await?;
  1355	        // Faults detected by the receive half abort the stream now,
  1356	        // not after the full scan; needs just accumulate. (Resize acks
  1357	        // cannot arrive yet — none is proposed before the payload phase.)
  1358	        drain_ready_source_events(
  1359	            &mut events,
  1360	            &mut pending,
  1361	            &mut resume,
  1362	            &mut need_complete,
  1363	            &mut needed_bytes,
  1364	            &mut needed_count,
  1365	            data_plane.as_ref(),
  1366	            tx,
  1367	            &mut pending_resize,
  1368	        )
  1369	        .await?;
  1370	    }
  1371	    let scanned = scan_handle
  1372	        .await
  1373	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
  1374	    let scan_complete = unreadable
  1375	        .lock()
  1376	        .expect("unreadable list lock poisoned")
  1377	        .is_empty();
  1378	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
  1379	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
  1380	        scan_complete,
  1381	    })))
  1382	    .await?;
  1383	    manifest_sent.store(true, Ordering::Release);
  1384	
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch (one ADD per epoch; a no-op while one is in
  1417	                    // flight or the shape wants no more).
  1418	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1419	                        .await?;
  1420	                    let payloads =
  1421	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1422	                    // A cancel while earlier batches are actively moving
  1423	                    // closes the send pipeline under backpressure, so this
  1424	                    // queue fails with a data-plane error — prefer the
  1425	                    // peer's framed reason (CANCELLED) the same way the
  1426	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1427	                    // against events like finish(): live `Need`s still
  1428	                    // arrive here, and `recv_peer_fault` would consume them.
  1429	                    if let Err(dp_err) = dp.queue(payloads).await {
  1430	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1660	    data_plane: Option<&data_plane::SourceDataPlane>,
  1661	    tx: &mut Box<dyn FrameTx>,
  1662	    pending_resize: &mut Option<data_plane::PendingResize>,
  1663	) -> Result<()> {
  1664	    match event {
  1665	        SourceEvent::Need(header) => {
  1666	            if *need_complete {
  1667	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1668	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1669	                )));
  1670	            }
  1671	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1672	            *needed_count += 1;
  1673	            pending.push(header);
  1674	            Ok(())
  1675	        }
  1676	        SourceEvent::ResumeNeed(header) => {
  1677	            if *need_complete {
  1678	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1679	                    format!(
  1680	                        "resume need for '{}' after NeedComplete",
  1681	                        header.relative_path
  1682	                    ),
  1683	                )));
  1684	            }
  1685	            // Shape totals count the whole file — the diff hasn't run
  1686	            // yet, so the need list's implied workload is the honest
  1687	            // upper bound (same accounting a plain need gets).
  1688	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1689	            *needed_count += 1;
  1690	            // HELD until its BlockHashList arrives; no duplicate is
  1691	            // possible (the receive half's sent-map removal already
  1692	            // faults a second need for the same path).
  1693	            resume.held.insert(header.relative_path.clone(), header);
  1694	            Ok(())
  1695	        }
  1696	        SourceEvent::BlockHashes(list) => {
  1697	            // Validate the wire block size at ARRIVAL (codex F5), not
  1698	            // when the record is eventually sent — pending plain files
  1699	            // go out first, and an already-invalid frame must fail fast.
  1700	            // A conforming destination clamps into this range (D5 /
  1701	            // D-2026-07-10-1); same-build peers make a mismatch a
  1702	            // violation, never a negotiation. The ceiling is the
  1703	            // CARRIER's (otp-7b, D-2026-07-10-2): binary data-plane
  1704	            // records take up to the wire block cap; in-stream frames
  1705	            // must stay under the gRPC frame limit.
  1706	            let ceiling = if data_plane.is_some() {
  1707	                MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  1708	            } else {
  1709	                MAX_IN_STREAM_RESUME_BLOCK_SIZE
  1710	            };
  1711	            let bs = list.block_size as usize;
  1712	            if !(MIN_RESUME_BLOCK_SIZE..=ceiling).contains(&bs) {
  1713	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1714	                    format!(
  1715	                        "BlockHashList for '{}' block_size {bs} outside \
  1716	                         [{MIN_RESUME_BLOCK_SIZE}, {ceiling}]",
  1717	                        list.relative_path
  1718	                    ),
  1719	                )));
  1720	            }
  1721	            match resume.held.remove(&list.relative_path) {
  1722	                Some(header) => {
  1723	                    resume.ready.push((header, list));
  1724	                    Ok(())
  1725	                }
  1726	                None => Err(eyre::Report::new(SessionFault::protocol_violation(
  1727	                    format!(
  1728	                        "BlockHashList for '{}' without a held resume need",
  1729	                        list.relative_path
  1730	                    ),
  1731	                ))),
  1732	            }
  1733	        }
  1734	        SourceEvent::NeedComplete => {
  1735	            if *need_complete {
  1736	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1737	                    "duplicate NeedComplete",
  1738	                )));
  1739	            }
  1740	            // Ordered lane: the destination sends every BlockHashList
  1741	            // before its NeedComplete, so a still-held resume need here
  1742	            // means the peer broke the choreography — fail fast rather
  1743	            // than hang waiting for a list that can no longer arrive.
  1744	            if !resume.held.is_empty() {
  1745	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1746	                    format!(
  1747	                        "NeedComplete with {} resume need(s) missing their BlockHashList",
  1748	                        resume.held.len()
  1749	                    ),
  1750	                )));
  1751	            }
  1752	            *need_complete = true;
  1753	            Ok(())
  1754	        }
  1755	        SourceEvent::ResizeAck(ack) => {
  1756	            let dp = data_plane.ok_or_else(|| {
  1757	                eyre::Report::new(SessionFault::protocol_violation(
  1758	                    "DataPlaneResizeAck on a session with no data plane",
  1759	                ))
  1760	            })?;
  1761	            // Match the ack to the in-flight proposal; stale/unsolicited
  1762	            // acks (wrong epoch, or none pending) are ignored, matching
  1763	            // old push. `take()` + restore keeps the borrow simple.
  1764	            let pending_r = match pending_resize.take() {
  1765	                Some(p) if p.epoch == ack.epoch => p,
  1766	                restored => {
  1767	                    *pending_resize = restored;
  1768	                    return Ok(());
  1769	                }
  1770	            };
  1771	            if ack.accepted {
  1772	                dp.add_stream(&pending_r.sub_token).await?;
  1773	                dp.dial()
  1774	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1775	            } else {
  1776	                dp.dial()
  1777	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1778	            }
  1779	            // Ramp one stream per accepted epoch: propose the next ADD.
  1780	            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1781	        }
  1782	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1783	            "TransferSummary before SourceDone",
  1784	        ))),
  1785	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1786	    }
  1787	}
  1788	
  1789	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1790	/// the stream count the accumulated need list implies, if none is in
  1791	/// flight. A no-op when the shape wants no more than the live count (the
  1792	/// dial returns `None`). Sends the frame and records the in-flight
  1793	/// proposal for the ack to match.
  1794	async fn maybe_propose_resize(
  1795	    dp: &data_plane::SourceDataPlane,
  1796	    tx: &mut Box<dyn FrameTx>,
  1797	    needed_bytes: u64,
  1798	    needed_count: usize,
  1799	    pending_resize: &mut Option<data_plane::PendingResize>,
  1800	) -> Result<()> {
  1801	    if pending_resize.is_some() {
  1802	        return Ok(());
  1803	    }
  1804	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1805	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1806	            op: DataPlaneResizeOp::Add as i32,
  1807	            epoch: proposal.epoch,
  1808	            target_stream_count: proposal.target_streams,
  1809	            sub_token: proposal.sub_token.clone(),
  1810	        })))
  1811	        .await?;
  1812	        *pending_resize = Some(proposal);
  1813	    }
  1814	    Ok(())
  1815	}
  1816	
  1817	/// Block for the ack of the one in-flight resize and dial its socket (or
  1818	/// settle it refused). Does NOT propose further — it resolves exactly the
  1819	/// pending proposal so the destination's armed slot is consumed before we
  1820	/// finish the data plane.
  1821	async fn resolve_in_flight_resize(
  1822	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1823	    dp: &data_plane::SourceDataPlane,
  1824	    pending: data_plane::PendingResize,
  1825	) -> Result<()> {
  1826	    loop {
  1827	        match events.recv().await {
  1828	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1829	                if ack.accepted {
  1830	                    dp.add_stream(&pending.sub_token).await?;
  1831	                    dp.dial()
  1832	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1833	                } else {
  1834	                    dp.dial()
  1835	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1836	                }
  1837	                return Ok(());
  1838	            }
  1839	            // A stale ack for an already-settled epoch: ignore, keep
  1840	            // waiting for ours.
  1841	            Some(SourceEvent::ResizeAck(_)) => continue,
  1842	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1843	            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1844	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1845	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1845	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1846	                )))
  1847	            }
  1848	            Some(SourceEvent::BlockHashes(l)) => {
  1849	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1850	                    format!(
  1851	                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
  1852	                        l.relative_path
  1853	                    ),
  1854	                )))
  1855	            }
  1856	            Some(SourceEvent::NeedComplete) => {
  1857	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1858	                    "duplicate NeedComplete",
  1859	                )))
  1860	            }
  1861	            Some(SourceEvent::Summary(_)) => {
  1862	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1863	                    "TransferSummary before SourceDone",
  1864	                )))
  1865	            }
  1866	            None => {
  1867	                return Err(eyre::Report::new(SessionFault::internal(
  1868	                    "source receive half ended with a resize in flight",
  1869	                )))
  1870	            }
  1871	        }
  1872	    }
  1873	}
  1874	
  1875	/// Await the next terminal signal the receive half forwards while the
  1876	/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
  1877	/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
  1878	/// abort the send and surface as the fault.
  1879	///
  1880	/// The drain runs after `resolve_in_flight_resize` and before `SourceDone`
  1881	/// goes out, so the event channel is drained and the peer sends nothing
  1882	/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
  1883	/// or `Summary` is legitimate here. So a `Fault` is returned as-is and any
  1884	/// OTHER event is surfaced as a protocol violation rather than silently
  1885	/// dropped (codex otp-4b-3 F3): dropping it would defer or lose a
  1886	/// fail-fast error and, if the drain is itself stuck, hang. Parks forever
  1887	/// once the channel closes with no event so the data-plane future it
  1888	/// races decides the outcome instead.
  1889	async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
  1890	    match events.recv().await {
  1891	        Some(SourceEvent::Fault(fault)) => fault,
  1892	        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1893	            SessionFault::protocol_violation(format!(
  1894	                "need for '{}' during the data-plane drain (after NeedComplete)",
  1895	                h.relative_path
  1896	            ))
  1897	        }
  1898	        Some(SourceEvent::BlockHashes(l)) => SessionFault::protocol_violation(format!(
  1899	            "BlockHashList for '{}' during the data-plane drain",
  1900	            l.relative_path
  1901	        )),
  1902	        Some(SourceEvent::NeedComplete) => {
  1903	            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
  1904	        }
  1905	        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
  1906	            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
  1907	        ),
  1908	        Some(SourceEvent::Summary(_)) => {
  1909	            SessionFault::protocol_violation("TransferSummary before SourceDone")
  1910	        }
  1911	        None => std::future::pending().await,
  1912	    }
  1913	}
  1914	
  1915	/// A data-plane operation (`queue`/`finish`) failed mid-transfer. The
  1916	/// break is usually the *symptom* of a peer abort — within
  1917	/// `TRANSFER_STALL_TIMEOUT` the peer (which runs the same stall guard on
  1918	/// its receive workers) always frames the real reason on the control
  1919	/// lane. Prefer that framed fault; fall back to the raw data-plane error
  1920	/// if the channel closes first or none arrives in that window.
  1921	///
  1922	/// Unlike `recv_peer_fault` (the finish()-drain select arm, which fails
  1923	/// fast on any stray event), this is called from BOTH error sites,
  1924	/// including the `queue()` error inside the payload loop — where a
  1925	/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
  1926	/// ahead of the peer's `SessionError` (codex otp-4b-3 pass-2 F1). So it
  1927	/// SKIPS non-fault events rather than treating them as violations: we are
  1928	/// already unwinding on a data-plane error, and the framed fault (or the
  1929	/// dp error) is the correct outcome, never a spurious protocol violation.
  1930	async fn prefer_peer_fault(
  1931	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1932	    dp_err: eyre::Report,
  1933	) -> eyre::Report {
  1934	    let framed = async {
  1935	        loop {

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '730,950p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1250,1515p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1515,1665p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   730	            session,
   731	            Arc::clone(&source),
   732	            PathBuf::new(),
   733	        )));
   734	    }
   735	
   736	    let prefetch = dial.prefetch_count().max(1);
   737	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   738	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   739	    let pipe_source = Arc::clone(&source);
   740	    let pipe_progress = instruments.progress.clone();
   741	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   742	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   743	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   744	        execute_sink_pipeline_elastic(
   745	            pipe_source,
   746	            sinks,
   747	            payload_rx,
   748	            prefetch,
   749	            pipe_progress.as_ref(),
   750	            Some(control_rx),
   751	        )
   752	        .await
   753	    }));
   754	    Ok(SourceDataPlane {
   755	        payload_tx: Some(payload_tx),
   756	        control_tx,
   757	        pipeline: Some(pipeline),
   758	        dial,
   759	        source,
   760	        session_token: grant.session_token.clone(),
   761	        pool,
   762	        trace,
   763	        // SOURCE initiator: each epoch-N resize socket is dialed to the
   764	        // granted host:port.
   765	        sockets: SourceSockets::Dial {
   766	            host: host.to_string(),
   767	            tcp_port: grant.tcp_port,
   768	        },
   769	    })
   770	}
   771	
   772	/// Accept the granted epoch-0 socket(s) off a bound responder listener and
   773	/// start the elastic SEND pipeline over them — the SOURCE **responder**
   774	/// half of the pull data plane (otp-5b-1). Symmetric with
   775	/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
   776	/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
   777	/// socket acquisition differs (accept here, dial there).
   778	/// `DataPlaneSession::from_stream` builds a send session from an already-
   779	/// accepted socket — the same primitive the old `pull_sync` daemon-send
   780	/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
   781	/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
   782	/// wherever it initiates). The bound listener is retained so each epoch-N
   783	/// resize socket is accepted off it (otp-5b-2): the DESTINATION initiator
   784	/// dials, this end accepts, the control-lane frames identical to push.
   785	pub(super) async fn accept_source_data_plane(
   786	    bound: ResponderDataPlane,
   787	    receiver_capacity: Option<&CapacityProfile>,
   788	    source: Arc<dyn TransferSource>,
   789	    instruments: &SourceInstruments,
   790	) -> Result<SourceDataPlane> {
   791	    let initial = bound.initial_streams.max(1) as usize;
   792	    // The byte sender's dial, bounded by the receiver's advertised
   793	    // capacity; seed the live count to the granted epoch-0 streams. Growth
   794	    // is via resize (otp-5b-2): the accept-based epoch-N socket steps from
   795	    // here, one stream per epoch, same as the SOURCE initiator.
   796	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   797	    dial.set_negotiated_streams(initial);
   798	
   799	    // Epoch-0 credential the dialing DESTINATION presents:
   800	    // session_token ‖ epoch0_sub_token (contract §Transport).
   801	    let mut epoch0 = bound.session_token.clone();
   802	    epoch0.extend_from_slice(&bound.epoch0_sub_token);
   803	
   804	    let pool = Arc::new(BufferPool::for_data_plane(
   805	        dial.chunk_bytes(),
   806	        dial.ceiling_max_streams().max(1),
   807	    ));
   808	    let trace = instruments.trace_data_plane;
   809	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   810	    for _ in 0..initial {
   811	        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
   812	        let session = DataPlaneSession::from_stream(
   813	            socket,
   814	            trace,
   815	            dial.chunk_bytes(),
   816	            dial.prefetch_count(),
   817	            Arc::clone(&pool),
   818	        )
   819	        .await;
   820	        sinks.push(Arc::new(DataPlaneSink::new(
   821	            session,
   822	            Arc::clone(&source),
   823	            PathBuf::new(),
   824	        )));
   825	    }
   826	
   827	    let prefetch = dial.prefetch_count().max(1);
   828	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   829	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   830	    let pipe_source = Arc::clone(&source);
   831	    let pipe_progress = instruments.progress.clone();
   832	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   833	        execute_sink_pipeline_elastic(
   834	            pipe_source,
   835	            sinks,
   836	            payload_rx,
   837	            prefetch,
   838	            pipe_progress.as_ref(),
   839	            Some(control_rx),
   840	        )
   841	        .await
   842	    }));
   843	    Ok(SourceDataPlane {
   844	        payload_tx: Some(payload_tx),
   845	        control_tx,
   846	        pipeline: Some(pipeline),
   847	        dial,
   848	        source,
   849	        session_token: bound.session_token,
   850	        pool,
   851	        trace,
   852	        // SOURCE responder: each epoch-N resize socket is accepted off the
   853	        // same listener epoch-0 came in on (otp-5b-2).
   854	        sockets: SourceSockets::Accept {
   855	            listener: bound.listener,
   856	        },
   857	    })
   858	}
   859	
   860	impl SourceDataPlane {
   861	    /// The live dial (the byte sender owns it). The driver reads
   862	    /// `live_streams()` for observability and calls `resize_settled` as
   863	    /// each proposal completes.
   864	    pub(super) fn dial(&self) -> &Arc<TransferDial> {
   865	        &self.dial
   866	    }
   867	
   868	    /// sf-2 shape correction: propose one ADD toward the stream count the
   869	    /// accumulated need list implies, if none is in flight and the shape
   870	    /// wants more than the current live count. Mints the resize
   871	    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
   872	    /// the record back on the matching ack.
   873	    pub(super) fn propose_resize(
   874	        &self,
   875	        needed_bytes: u64,
   876	        needed_count: usize,
   877	    ) -> Result<Option<PendingResize>> {
   878	        let desired =
   879	            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
   880	                as usize;
   881	        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
   882	            return Ok(None);
   883	        };
   884	        let sub_token = generate_sub_token()
   885	            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
   886	        Ok(Some(PendingResize {
   887	            epoch: proposal.epoch,
   888	            target_streams: proposal.target_streams as u32,
   889	            sub_token,
   890	        }))
   891	    }
   892	
   893	    /// Acquire the epoch-N data socket for an accepted resize and hand it
   894	    /// to the running pipeline (`SinkControl::Add`). The SOURCE initiator
   895	    /// (push) DIALS it; the SOURCE responder (pull, otp-5b-2) ACCEPTS the
   896	    /// socket the DESTINATION initiator dials after its ack, off the same
   897	    /// listener epoch-0 came in on. A dial/accept failure is FATAL
   898	    /// (fail-fast): a same-build peer that established epoch-0 failing an
   899	    /// epoch-N socket is a transport fault worth surfacing — and faulting
   900	    /// the session aborts the peer's counterpart via AbortOnDrop, so no
   901	    /// slot orphans. (Old push recovers non-fatally via an arm TTL; the
   902	    /// session trades that for simplicity — noted in the finding doc.) If
   903	    /// the pipeline is already gone (transfer completing under the ADD),
   904	    /// the just-acquired socket is closed cleanly so the peer's worker sees
   905	    /// its END, not a reset.
   906	    ///
   907	    /// The accept is bounded and unambiguous: at most one resize is in
   908	    /// flight (the driver's `pending_resize`) and epoch-0 is already
   909	    /// accepted, so the next connection off the listener is exactly this
   910	    /// resize's socket — verified against `session_token ‖ sub_token`.
   911	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   912	        let session = match &self.sockets {
   913	            SourceSockets::Dial { host, tcp_port } => {
   914	                let mut handshake = self.session_token.clone();
   915	                handshake.extend_from_slice(sub_token);
   916	                DataPlaneSession::connect(
   917	                    host,
   918	                    *tcp_port,
   919	                    &handshake,
   920	                    self.dial.chunk_bytes(),
   921	                    self.dial.prefetch_count(),
   922	                    self.trace,
   923	                    self.dial.tcp_buffer_bytes(),
   924	                    Arc::clone(&self.pool),
   925	                )
   926	                .await
   927	                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?
   928	            }
   929	            SourceSockets::Accept { listener } => {
   930	                let mut expected = self.session_token.clone();
   931	                expected.extend_from_slice(sub_token);
   932	                let socket = accept_authenticated(listener, &expected).await?;
   933	                DataPlaneSession::from_stream(
   934	                    socket,
   935	                    self.trace,
   936	                    self.dial.chunk_bytes(),
   937	                    self.dial.prefetch_count(),
   938	                    Arc::clone(&self.pool),
   939	                )
   940	                .await
   941	            }
   942	        };
   943	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   944	            session,
   945	            Arc::clone(&self.source),
   946	            PathBuf::new(),
   947	        ));
   948	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   949	            if let SinkControl::Add(sink) = returned.0 {
   950	                let _ = sink.finish().await;
  1250	    // manifest streams. Which end connects depends on connection role
  1251	    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
  1252	    // listener; a SOURCE **initiator** (push) dials the grant it received.
  1253	    // Byte direction is the same either way (SOURCE sends), so both yield a
  1254	    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
  1255	    // in-stream carrier (fallback), which needs no early setup.
  1256	    let mut data_plane = match responder_data_plane {
  1257	        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
  1258	        // initiator advertised its capacity in the open (byte RECEIVER
  1259	        // advertises, wherever it initiates); the accept plane is single-
  1260	        // stream (otp-5b-1).
  1261	        Some(bound) => Some(
  1262	            data_plane::accept_source_data_plane(
  1263	                bound,
  1264	                negotiated.open.receiver_capacity.as_ref(),
  1265	                Arc::clone(&source),
  1266	                &instruments,
  1267	            )
  1268	            .await?,
  1269	        ),
  1270	        // SOURCE initiator (push, otp-4b): dial the grant if the responder
  1271	        // granted a data plane; else in-stream.
  1272	        None => match &negotiated.accept.data_plane {
  1273	            Some(grant) => {
  1274	                let host = data_plane_host.ok_or_else(|| {
  1275	                    eyre::Report::new(SessionFault::internal(
  1276	                        "responder granted a TCP data plane but this initiator has no host to dial",
  1277	                    ))
  1278	                })?;
  1279	                Some(
  1280	                    data_plane::dial_source_data_plane(
  1281	                        host,
  1282	                        grant,
  1283	                        negotiated.accept.receiver_capacity.as_ref(),
  1284	                        Arc::clone(&source),
  1285	                        &instruments,
  1286	                    )
  1287	                    .await?,
  1288	                )
  1289	            }
  1290	            None => None,
  1291	        },
  1292	    };
  1293	
  1294	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
  1295	    // fed to the shape table so the SOURCE grows the data-plane stream
  1296	    // count as the workload's shape becomes known. Append-only (a need is
  1297	    // counted once, when it arrives), and the in-flight resize record the
  1298	    // ack is matched against (at most one — the dial enforces it).
  1299	    let mut needed_bytes: u64 = 0;
  1300	    let mut needed_count: usize = 0;
  1301	    let mut pending_resize: Option<data_plane::PendingResize> = None;
  1302	
  1303	    // Streaming manifest: entries go out as enumeration produces them
  1304	    // (immediate start in every direction — plan §Design 2). The open
  1305	    // carries no source path (the source end owns its local endpoint) but
  1306	    // does carry the include/exclude/size/age filter (otp-6a): only
  1307	    // matching files are manifested and transferred. The filter MUST ride
  1308	    // the wire (not be pre-wrapped by a local caller) because for pull the
  1309	    // SOURCE is the remote daemon responder — it, not the client, owns the
  1310	    // scan. Apply it through the universal `FilteredSource` decorator, the
  1311	    // single filter chokepoint every source impl routes through, rather
  1312	    // than the per-impl `scan(filter)` arg — a source impl is free to
  1313	    // ignore that arg (the since-deleted relay source did; codex otp-6a
  1314	    // F1), and the chokepoint makes filtering independent of it. A
  1315	    // default/absent filter scans everything (unchanged from otp-3). Globs
  1316	    // were validated at OPEN (`source_open_validator`), so the conversion
  1317	    // cannot fail on a validated open; map any error to a fault regardless.
  1318	    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
  1319	        Some(spec) if *spec != FilterSpec::default() => {
  1320	            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  1321	                .map_err(|e| {
  1322	                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
  1323	                })?;
  1324	            Arc::new(crate::remote::transfer::source::FilteredSource::new(
  1325	                Arc::clone(&source),
  1326	                filter,
  1327	            ))
  1328	        }
  1329	        _ => Arc::clone(&source),
  1330	    };
  1331	    // otp-10b-1: a Checksum session fills each manifest header's
  1332	    // checksum so the DESTINATION can skip content-equal files
  1333	    // regardless of mtime. Wrapped OUTSIDE the filter so only
  1334	    // in-scope files pay the hash; a serving end that refuses to hash
  1335	    // never gets here (CHECKSUM_DISABLED at OPEN).
  1336	    let scan_source: Arc<dyn TransferSource> =
  1337	        if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
  1338	            Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
  1339	                scan_source,
  1340	            ))
  1341	        } else {
  1342	            scan_source
  1343	        };
  1344	    // otp-10a: callers that must not treat a partial transfer as success
  1345	    // (the push verb, `blit move`'s source-delete gate) supply their own
  1346	    // accumulator via `SourceInstruments` and inspect it after the
  1347	    // session returns; the wire behavior is identical either way.
  1348	    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
  1349	    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
  1350	    while let Some(header) = header_rx.recv().await {
  1351	        sent.lock()
  1352	            .expect("sent-manifest lock poisoned")
  1353	            .insert(header.relative_path.clone(), header.clone());
  1354	        tx.send(frame(Frame::ManifestEntry(header))).await?;
  1355	        // Faults detected by the receive half abort the stream now,
  1356	        // not after the full scan; needs just accumulate. (Resize acks
  1357	        // cannot arrive yet — none is proposed before the payload phase.)
  1358	        drain_ready_source_events(
  1359	            &mut events,
  1360	            &mut pending,
  1361	            &mut resume,
  1362	            &mut need_complete,
  1363	            &mut needed_bytes,
  1364	            &mut needed_count,
  1365	            data_plane.as_ref(),
  1366	            tx,
  1367	            &mut pending_resize,
  1368	        )
  1369	        .await?;
  1370	    }
  1371	    let scanned = scan_handle
  1372	        .await
  1373	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
  1374	    let scan_complete = unreadable
  1375	        .lock()
  1376	        .expect("unreadable list lock poisoned")
  1377	        .is_empty();
  1378	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
  1379	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
  1380	        scan_complete,
  1381	    })))
  1382	    .await?;
  1383	    manifest_sent.store(true, Ordering::Release);
  1384	
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch (one ADD per epoch; a no-op while one is in
  1417	                    // flight or the shape wants no more).
  1418	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1419	                        .await?;
  1420	                    let payloads =
  1421	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1422	                    // A cancel while earlier batches are actively moving
  1423	                    // closes the send pipeline under backpressure, so this
  1424	                    // queue fails with a data-plane error — prefer the
  1425	                    // peer's framed reason (CANCELLED) the same way the
  1426	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1427	                    // against events like finish(): live `Need`s still
  1428	                    // arrive here, and `recv_peer_fault` would consume them.
  1429	                    if let Err(dp_err) = dp.queue(payloads).await {
  1430	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1431	                    }
  1432	                }
  1433	                None => {
  1434	                    // codex otp-8 F1: race the record sends against the
  1435	                    // receive half's fault signal — the in-stream twin of
  1436	                    // the data-plane drain's `recv_peer_fault` arm. A peer
  1437	                    // cancel (framed CANCELLED, then RPC teardown) must
  1438	                    // interrupt a send blocked in `reader.read()` or in
  1439	                    // flow-controlled `tx.send()` and surface the framed
  1440	                    // reason, not hang or decay to INTERNAL. Biased:
  1441	                    // when both are ready, the framed fault wins.
  1442	                    tokio::select! {
  1443	                        biased;
  1444	                        fault = peer_fault_signalled(&mut fault_signal) => {
  1445	                            return Err(eyre::Report::new(fault));
  1446	                        }
  1447	                        res = send_payload_records(
  1448	                            tx,
  1449	                            &source,
  1450	                            plan_options,
  1451	                            batch,
  1452	                            &mut read_buf,
  1453	                            instruments.progress.as_ref(),
  1454	                        ) => {
  1455	                            res?;
  1456	                        }
  1457	                    }
  1458	                }
  1459	            }
  1460	            continue;
  1461	        }
  1462	        if !resume.ready.is_empty() {
  1463	            // The block phase for correlated (need, hash-list) pairs.
  1464	            // Data plane (otp-7b): each pair becomes ONE composite
  1465	            // ResumeFile work item, so one pipeline worker runs the
  1466	            // whole record on one socket — strict per-file serialization
  1467	            // without cross-socket reorder hazards. In-stream (otp-7a):
  1468	            // control-lane BlockTransfer/Complete frames, as before.
  1469	            let ready = std::mem::take(&mut resume.ready);
  1470	            match &mut data_plane {
  1471	                Some(dp) => {
  1472	                    // codex 7b-1 F4: resume batches drive the sf-2 shape
  1473	                    // correction exactly as plain batches do — a
  1474	                    // resume-heavy need list must not stay pinned to the
  1475	                    // zero-knowledge single stream.
  1476	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1477	                        .await?;
  1478	                    let payloads = ready
  1479	                        .into_iter()
  1480	                        .map(|(header, hashes)| TransferPayload::ResumeFile {
  1481	                            header,
  1482	                            block_size: hashes.block_size,
  1483	                            dest_hashes: hashes.hashes,
  1484	                        })
  1485	                        .collect();
  1486	                    // Same cancel posture as the plain-batch queue above:
  1487	                    // prefer the peer's framed reason over the transport
  1488	                    // break a cancel also causes (otp-4b-3 codex F1).
  1489	                    if let Err(dp_err) = dp.queue(payloads).await {
  1490	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1491	                    }
  1492	                }
  1493	                None => {
  1494	                    for (header, hashes) in ready {
  1495	                        // codex 7b-2 G2: the whole in-stream record names
  1496	                        // its file on failure, matching the data-plane
  1497	                        // carrier's outer wrap. Same fault race as the
  1498	                        // plain-batch send above (codex otp-8 F1).
  1499	                        tokio::select! {
  1500	                            biased;
  1501	                            fault = peer_fault_signalled(&mut fault_signal) => {
  1502	                                return Err(eyre::Report::new(fault));
  1503	                            }
  1504	                            res = send_resume_block_records(
  1505	                                tx,
  1506	                                &source,
  1507	                                &header,
  1508	                                &hashes,
  1509	                                instruments.progress.as_ref(),
  1510	                            ) => {
  1511	                                res.map_err(|e| tag_path(e, &header.relative_path))?;
  1512	                            }
  1513	                        }
  1514	                    }
  1515	                }
  1515	                }
  1516	            }
  1517	            continue;
  1518	        }
  1519	        if need_complete {
  1520	            break;
  1521	        }
  1522	        match events.recv().await {
  1523	            Some(event) => {
  1524	                process_source_event(
  1525	                    event,
  1526	                    &mut pending,
  1527	                    &mut resume,
  1528	                    &mut need_complete,
  1529	                    &mut needed_bytes,
  1530	                    &mut needed_count,
  1531	                    data_plane.as_ref(),
  1532	                    tx,
  1533	                    &mut pending_resize,
  1534	                )
  1535	                .await?;
  1536	            }
  1537	            None => {
  1538	                return Err(eyre::Report::new(SessionFault::internal(
  1539	                    "source receive half ended before NeedComplete",
  1540	                )))
  1541	            }
  1542	        }
  1543	    }
  1544	
  1545	    // A resize proposed on the last batch may still be in flight. Resolve
  1546	    // it BEFORE finishing so the destination's armed slot is consumed by
  1547	    // the dialed socket — an armed-but-never-dialed credential would hang
  1548	    // its accept loop (which waits for every arm to be claimed). We do not
  1549	    // propose further here: exactly the one in-flight resize is drained.
  1550	    if let Some(dp) = &data_plane {
  1551	        if let Some(pending) = pending_resize.take() {
  1552	            resolve_in_flight_resize(&mut events, dp, pending).await?;
  1553	        }
  1554	    }
  1555	
  1556	    // Close the data plane BEFORE SourceDone so the destination's receive
  1557	    // pipeline sees each socket's END record and completes; SourceDone on
  1558	    // the control lane then lets the destination score and summarize.
  1559	    //
  1560	    // The drain is the byte-transfer phase's wall-time sink, so a
  1561	    // mid-transfer cancel almost always lands here. Race it against a
  1562	    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
  1563	    // the served session frames `SessionError{CANCELLED}`, and the source
  1564	    // must surface THAT — not the data-plane transport break it also
  1565	    // causes. Two orderings, both covered:
  1566	    //   * fault arrives while the drain is still pending (e.g. a worker
  1567	    //     blocked reading a slow file, so the socket break never unblocks
  1568	    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
  1569	    //     `finish()` future drops the data plane, and its `AbortOnDrop`
  1570	    //     stops the in-flight workers.
  1571	    //   * the socket break makes `finish()` return `Err` first → prefer
  1572	    //     the framed reason if the control lane delivers one within the
  1573	    //     stall window (`prefer_peer_fault`).
  1574	    if let Some(dp) = data_plane.take() {
  1575	        tokio::select! {
  1576	            biased;
  1577	            fault = recv_peer_fault(&mut events) => {
  1578	                return Err(eyre::Report::new(fault));
  1579	            }
  1580	            res = dp.finish() => {
  1581	                if let Err(dp_err) = res {
  1582	                    return Err(prefer_peer_fault(&mut events, dp_err).await);
  1583	                }
  1584	            }
  1585	        }
  1586	    }
  1587	
  1588	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
  1589	
  1590	    // CLOSING: the destination is the scorer; the next event must be
  1591	    // its summary (the receive half ends after forwarding it).
  1592	    match events.recv().await {
  1593	        Some(SourceEvent::Summary(summary)) => Ok(summary),
  1594	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
  1595	        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1596	            Err(eyre::Report::new(SessionFault::protocol_violation(
  1597	                format!("need for '{}' after NeedComplete", h.relative_path),
  1598	            )))
  1599	        }
  1600	        Some(SourceEvent::BlockHashes(l)) => {
  1601	            Err(eyre::Report::new(SessionFault::protocol_violation(
  1602	                format!("BlockHashList for '{}' after SourceDone", l.relative_path),
  1603	            )))
  1604	        }
  1605	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
  1606	            SessionFault::protocol_violation("duplicate NeedComplete"),
  1607	        )),
  1608	        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
  1609	            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
  1610	        )),
  1611	        None => Err(eyre::Report::new(SessionFault::internal(
  1612	            "source receive half ended before TransferSummary",
  1613	        ))),
  1614	    }
  1615	}
  1616	
  1617	/// Process every event ready right now (needs accumulating, resize acks
  1618	/// dialing their epoch-N socket) without blocking. Called between
  1619	/// manifest sends and at the top of the payload loop.
  1620	#[allow(clippy::too_many_arguments)]
  1621	async fn drain_ready_source_events(
  1622	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1623	    pending: &mut Vec<FileHeader>,
  1624	    resume: &mut ResumeSendState,
  1625	    need_complete: &mut bool,
  1626	    needed_bytes: &mut u64,
  1627	    needed_count: &mut usize,
  1628	    data_plane: Option<&data_plane::SourceDataPlane>,
  1629	    tx: &mut Box<dyn FrameTx>,
  1630	    pending_resize: &mut Option<data_plane::PendingResize>,
  1631	) -> Result<()> {
  1632	    while let Ok(event) = events.try_recv() {
  1633	        process_source_event(
  1634	            event,
  1635	            pending,
  1636	            resume,
  1637	            need_complete,
  1638	            needed_bytes,
  1639	            needed_count,
  1640	            data_plane,
  1641	            tx,
  1642	            pending_resize,
  1643	        )
  1644	        .await?;
  1645	    }
  1646	    Ok(())
  1647	}
  1648	
  1649	/// Handle one source event. Needs accumulate into `pending` and the
  1650	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1651	/// next ADD (the one-per-epoch ramp).
  1652	#[allow(clippy::too_many_arguments)]
  1653	async fn process_source_event(
  1654	    event: SourceEvent,
  1655	    pending: &mut Vec<FileHeader>,
  1656	    resume: &mut ResumeSendState,
  1657	    need_complete: &mut bool,
  1658	    needed_bytes: &mut u64,
  1659	    needed_count: &mut usize,
  1660	    data_plane: Option<&data_plane::SourceDataPlane>,
  1661	    tx: &mut Box<dyn FrameTx>,
  1662	    pending_resize: &mut Option<data_plane::PendingResize>,
  1663	) -> Result<()> {
  1664	    match event {
  1665	        SourceEvent::Need(header) => {

exec
/bin/zsh -lc "git grep -n \"FsTransferSink\\|write_tar_shard_payload\\|execute_receive_pipeline\\|apply_tar_shard\" 0f922de -- 'crates/**/*.rs'; git grep -n \"plan_push_payloads\\|ManifestComplete\\|early flush\\|NeedBatch\\|needs\" 0f922de -- 'crates/blit-core/src/remote/push/**/*.rs' 'crates/blit-daemon/src/service/push/**/*.rs' 'crates/blit-core/src/dial.rs'; git show 0f922de:crates/blit-daemon/src/service/push/data_plane.rs | nl -ba | sed -n '1,280p'; git show 0f922de:crates/blit-core/src/remote/push/client/mod.rs | nl -ba | sed -n '780,990p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-3ZGCcwQn' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-G0OveNWA' (errno=Operation not permitted)
0f922de:crates/blit-core/src/engine/mod.rs:79:    /// Destination sink for the streaming strategy (`FsTransferSink`
0f922de:crates/blit-core/src/local_worker.rs:3://! The main transfer pipeline now uses `FsTransferSink` (via `execute_sink_pipeline`).
0f922de:crates/blit-core/src/orchestrator/orchestrator.rs:8:use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, NullSink, TransferSink};
0f922de:crates/blit-core/src/orchestrator/orchestrator.rs:92:            Arc::new(FsTransferSink::new(
0f922de:crates/blit-core/src/remote/pull.rs:1369:/// shared helper so this site, `FsTransferSink`, and the daemon push
0f922de:crates/blit-core/src/remote/pull.rs:1899:    // Builds an FsTransferSink rooted at the destination, optionally
0f922de:crates/blit-core/src/remote/pull.rs:1901:    // execute_receive_pipeline parse records + dispatch to the sink.
0f922de:crates/blit-core/src/remote/pull.rs:1902:    use crate::remote::transfer::pipeline::execute_receive_pipeline;
0f922de:crates/blit-core/src/remote/pull.rs:1903:    use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
0f922de:crates/blit-core/src/remote/pull.rs:1917:    let mut sink = FsTransferSink::new(PathBuf::new(), dest_root.to_path_buf(), config);
0f922de:crates/blit-core/src/remote/pull.rs:1945:    let outcome = execute_receive_pipeline(&mut stream, sink, progress).await?;
0f922de:crates/blit-core/src/remote/pull.rs:1975:/// non-FsTransferSink pull paths (legacy direct-write,
0f922de:crates/blit-core/src/remote/transfer/data_plane.rs:305:        // FsTransferSink without consulting an out-of-band manifest cache.
0f922de:crates/blit-core/src/remote/transfer/mod.rs:43:    DataPlaneSink, FsSinkConfig, FsTransferSink, GrpcFallbackSink, GrpcServerStreamingSink,
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:416:/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:420:pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:685:    use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:726:        let sink = Arc::new(FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:774:        let sink = Arc::new(FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:838:            Arc::new(FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:886:        // Build a minimal FsTransferSink that writes to a temp dir.
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:889:        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1030:            // execute_receive_pipeline takes &mut TcpStream. Use a real
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1052:            let result = execute_receive_pipeline(&mut reader, sink, None).await;
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1202:        let outcome = execute_receive_pipeline(&mut reader, sink, Some(&progress))
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1242:        execute_receive_pipeline(&mut reader, sink, Some(&progress))
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1274:        execute_receive_pipeline(&mut reader, sink, Some(&progress))
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1308:        let sink = Arc::new(FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1433:        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/pipeline.rs:1450:        let err = execute_receive_pipeline(&mut guarded, sink, None)
0f922de:crates/blit-core/src/remote/transfer/sink.rs:78:// FsTransferSink — local filesystem writer
0f922de:crates/blit-core/src/remote/transfer/sink.rs:114:pub struct FsTransferSink {
0f922de:crates/blit-core/src/remote/transfer/sink.rs:138:    /// [`FsTransferSink::with_byte_progress`] from
0f922de:crates/blit-core/src/remote/transfer/sink.rs:143:impl FsTransferSink {
0f922de:crates/blit-core/src/remote/transfer/sink.rs:197:                    "FsTransferSink at '{}' has no canonical root; \
0f922de:crates/blit-core/src/remote/transfer/sink.rs:217:impl TransferSink for FsTransferSink {
0f922de:crates/blit-core/src/remote/transfer/sink.rs:262:                eyre::bail!("FsTransferSink does not consume composite ResumeFile payloads")
0f922de:crates/blit-core/src/remote/transfer/sink.rs:286:                    PreparedPayload::TarShard { headers, data } => write_tar_shard_payload(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:313:        // `write_tar_shard_payload`'s dry-run early returns), so
0f922de:crates/blit-core/src/remote/transfer/sink.rs:324:    /// is what makes push and pull receive symmetric on the FsTransferSink.
0f922de:crates/blit-core/src/remote/transfer/sink.rs:462:    // R47-F1: the FsTransferSink::write_payload arm for
0f922de:crates/blit-core/src/remote/transfer/sink.rs:538:fn write_tar_shard_payload(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:571:    // R47-F1: tar shards arriving on FsTransferSink::write_payload
0f922de:crates/blit-core/src/remote/transfer/sink.rs:586:            "write_tar_shard_payload at '{}' has no canonical root; \
0f922de:crates/blit-core/src/remote/transfer/sink.rs:595:    // either way (matches the historical FsTransferSink policy).
0f922de:crates/blit-core/src/remote/transfer/sink.rs:994:        // FsTransferSink.
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1383:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1416:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1453:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1490:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1526:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1583:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1618:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1978:    // validator's surface. These tests exercise the FsTransferSink end of
0f922de:crates/blit-core/src/remote/transfer/sink.rs:1990:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2064:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2100:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2148:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2186:    /// helper via `execute_receive_pipeline`, so this also closes
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2208:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2240:    /// existing dst escape symlink. Pre-fix `write_tar_shard_payload`
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2279:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2353:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/sink.rs:2400:        let sink = FsTransferSink::new(
0f922de:crates/blit-core/src/remote/transfer/tar_safety.rs:7://!   - `crates/blit-core/src/remote/transfer/sink.rs::write_tar_shard_payload`
0f922de:crates/blit-core/src/remote/transfer/tar_safety.rs:9://!   - `crates/blit-daemon/src/service/push/data_plane.rs::apply_tar_shard_sync`
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:4://! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:19://! pipeline, receive is `execute_receive_pipeline` — only socket
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:56:use crate::remote::transfer::pipeline::execute_receive_pipeline;
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:351:        execute_receive_pipeline(&mut guarded, sink, None).await
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:939:/// `execute_receive_pipeline` writes socket-provided paths directly, so
0f922de:crates/blit-core/src/transfer_session/mod.rs:43:use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
0f922de:crates/blit-core/src/transfer_session/mod.rs:69:/// into `FsTransferSink::write_file_stream`. Bounds destination-side
0f922de:crates/blit-core/src/transfer_session/mod.rs:2308:    let mut sink = FsTransferSink::new(
0f922de:crates/blit-core/src/transfer_session/mod.rs:3242:    sink: &FsTransferSink,
0f922de:crates/blit-core/src/transfer_session/mod.rs:3328:    sink: &FsTransferSink,
0f922de:crates/blit-core/src/transfer_session/mod.rs:3357:    sink: &FsTransferSink,
0f922de:crates/blit-core/src/transfer_session/mod.rs:3427:    sink: &FsTransferSink,
0f922de:crates/blit-core/tests/engine_streaming_plan.rs:25:use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink, SinkOutcome, TransferSink};
0f922de:crates/blit-core/tests/engine_streaming_plan.rs:119:/// Wraps `FsTransferSink`; fires `gate` after the first successful
0f922de:crates/blit-core/tests/engine_streaming_plan.rs:122:    inner: FsTransferSink,
0f922de:crates/blit-core/tests/engine_streaming_plan.rs:194:        inner: FsTransferSink::new(
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:7:use blit_core::remote::transfer::pipeline::execute_receive_pipeline;
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:185:    //   socket → StallGuard → execute_receive_pipeline → FsTransferSink → disk
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:187:    // extracted inline by FsTransferSink (parallelism across streams
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:197:    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:206:    let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:858:/// routes through `FsTransferSink::write_tar_shard_payload`
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:863:/// having the gRPC fallback also call `FsTransferSink::write_payload`,
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:869:///    path's `apply_tar_shard` over a single contiguous buffer.
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:940:                apply_tar_shard_sync(module, headers, buffer, pool_buffer_size)
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1027:fn apply_tar_shard_sync(
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1086:    execute_receive_pipeline(&mut guarded, sink, None).await
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1092:    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1129:        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1148:        // execute_receive_pipeline surfaces it as an Err.
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1162:    fn apply_tar_shard_handles_long_paths() {
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1194:        let (stats, _returned_buf) = apply_tar_shard_sync(
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1256:    /// `apply_tar_shard_sync` now routes through
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1260:    fn apply_tar_shard_rejects_symlink_entry() {
0f922de:crates/blit-daemon/src/service/push/data_plane.rs:1291:        let err = apply_tar_shard_sync(module, vec![header], tar_data, TAR_BUFFER_SIZE)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-8B4644nb' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-IidYBMng' (errno=Operation not permitted)
0f922de:crates/blit-core/src/remote/push/client/helpers.rs:16:use crate::generated::{ClientPushRequest, FileHeader, ManifestComplete, ServerPushResponse};
0f922de:crates/blit-core/src/remote/push/client/helpers.rs:304:        ClientPayload::ManifestComplete(ManifestComplete { scan_complete }),
0f922de:crates/blit-core/src/remote/push/client/mod.rs:33:// "diff" itself lives on the daemon side (NeedList) — see plan_push_payloads.
0f922de:crates/blit-core/src/remote/push/client/mod.rs:34:use crate::remote::transfer::diff_planner::plan_push_payloads as plan_transfer_payloads;
0f922de:crates/blit-core/src/remote/push/client/mod.rs:138:/// ue-r2-2: everything an epoch-N dial needs, retained from connect
0f922de:crates/blit-core/src/remote/push/client/mod.rs:738:        // has seen ManifestComplete. Pre-fix, force_grpc initialized
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1352:                            // daemon at ManifestComplete time. Walkdir
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1606:    //! needs the `pipeline_handle` field wired through `AbortOnDrop`.
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-5XomC0eC' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Ab9j4OpS' (errno=Operation not permitted)
     1	use crate::runtime::ModuleConfig;
     2	use blit_core::buffer::BufferPool;
     3	use blit_core::generated::{
     4	    client_push_request, server_push_response, ClientPushRequest, DataTransferNegotiation,
     5	    FileHeader,
     6	};
     7	use blit_core::remote::transfer::pipeline::execute_receive_pipeline;
     8	use blit_core::remote::transfer::sink::{SinkOutcome, TransferSink};
     9	use blit_core::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
    10	use blit_core::remote::transfer::tar_safety;
    11	use blit_core::remote::transfer::{
    12	    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
    13	};
    14	use eyre::Result;
    15	use rand::{rngs::SysRng, TryRng};
    16	use std::collections::HashMap;
    17	use std::path::PathBuf;
    18	use std::sync::Arc;
    19	use std::time::{Duration, Instant};
    20	use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
    21	use tokio::net::{TcpListener, TcpStream};
    22	use tokio::sync::Semaphore;
    23	use tokio::task::JoinSet;
    24	use tonic::{Status, Streaming};
    25	
    26	use super::super::util::resolve_manifest_relative_path;
    27	use super::super::PushSender;
    28	use super::control::send_control_message;
    29	
    30	const TOKEN_LEN: usize = 32;
    31	const MAX_PARALLEL_TAR_TASKS: usize = 4;
    32	
    33	/// Default buffer size for pooled tar shard buffers (4 MiB).
    34	const TAR_BUFFER_SIZE: usize = 4 * 1024 * 1024;
    35	/// Maximum pooled buffers per connection stream.
    36	const TAR_BUFFER_POOL_SIZE: usize = 8;
    37	
    38	#[derive(Debug, Default, Clone, Copy)]
    39	pub(crate) struct TransferStats {
    40	    pub files_transferred: u64,
    41	    pub bytes_transferred: u64,
    42	    pub bytes_zero_copy: u64,
    43	}
    44	
    45	pub(crate) async fn bind_data_plane_listener() -> Result<TcpListener, Status> {
    46	    TcpListener::bind("0.0.0.0:0")
    47	        .await
    48	        .map_err(|err| Status::internal(format!("failed to bind data plane socket: {}", err)))
    49	}
    50	
    51	/// Generate a random data-plane handshake token.
    52	///
    53	/// audit-3b: the OS cryptographic RNG is effectively always available,
    54	/// but `try_fill_bytes` is fallible (a sandboxed / fd-exhausted
    55	/// container can deny it). Pre-fix this `expect`ed and panicked the
    56	/// spawned data-plane task, leaving the control-plane stream hung
    57	/// waiting for a handshake that would never arrive. Now it returns a
    58	/// `Status::Internal` the handler propagates as a clean RPC error.
    59	pub(crate) fn generate_token() -> Result<Vec<u8>, Status> {
    60	    let mut buf = vec![0u8; TOKEN_LEN];
    61	    SysRng
    62	        .try_fill_bytes(&mut buf)
    63	        .map_err(|err| Status::internal(format!("system RNG unavailable: {err}")))?;
    64	    Ok(buf)
    65	}
    66	
    67	pub(crate) async fn accept_data_connection_stream(
    68	    listener: TcpListener,
    69	    expected_token: Vec<u8>,
    70	    module: ModuleConfig,
    71	    stream_count: u32,
    72	) -> Result<TransferStats, Status> {
    73	    let start = Instant::now();
    74	    let streams = stream_count.max(1) as usize;
    75	    // w4-1: a JoinSet, not a Vec<JoinHandle> — dropping a JoinSet
    76	    // aborts every remaining worker, so a first-error return (or this
    77	    // whole future being cancelled) no longer detaches the survivors.
    78	    // Mirrors `accept_data_connection_stream_resizable`, which fixed
    79	    // this same class during ue-r2-2.
    80	    let mut join_set: JoinSet<Result<TransferStats, Status>> = JoinSet::new();
    81	
    82	    for idx in 0..streams {
    83	        let (accepted, addr) =
    84	            match tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await {
    85	                Ok(Ok(pair)) => pair,
    86	                Ok(Err(err)) => {
    87	                    return Err(Status::internal(format!(
    88	                        "data plane accept failed: {}",
    89	                        err
    90	                    )));
    91	                }
    92	                Err(_elapsed) => {
    93	                    return Err(Status::deadline_exceeded(format!(
    94	                        "data plane accept timed out after {:?} waiting for stream {}/{}",
    95	                        DATA_PLANE_ACCEPT_TIMEOUT,
    96	                        idx + 1,
    97	                        streams
    98	                    )));
    99	                }
   100	            };
   101	        // Enable nodelay + keepalive to prevent idle stream timeouts
   102	        // during long transfers on other streams. No tuned buffer:
   103	        // the daemon is the byte receiver here and holds no dial.
   104	        configure_data_socket(&accepted, None)
   105	            .map_err(|err| Status::internal(format!("configuring data socket: {err}")))?;
   106	        let socket = accepted;
   107	        eprintln!(
   108	            "blitd: push data plane: accepted connection {} from {}",
   109	            idx, addr
   110	        );
   111	        let expected_token = expected_token.clone();
   112	        let module_clone = module.clone();
   113	        join_set.spawn(async move {
   114	            handle_data_plane_stream(socket, expected_token, module_clone).await
   115	        });
   116	    }
   117	
   118	    let mut final_stats = TransferStats::default();
   119	    while let Some(joined) = join_set.join_next().await {
   120	        match joined {
   121	            Ok(Ok(stats)) => accumulate_transfer_stats(&mut final_stats, &stats),
   122	            Ok(Err(status)) => return Err(status),
   123	            Err(_) => return Err(Status::internal("data plane worker cancelled")),
   124	        }
   125	    }
   126	
   127	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   128	    let gbps = (final_stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   129	    eprintln!(
   130	        "blitd: push data plane: aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
   131	        gbps, final_stats.bytes_transferred, elapsed
   132	    );
   133	
   134	    Ok(final_stats)
   135	}
   136	
   137	async fn handle_data_plane_stream(
   138	    mut socket: TcpStream,
   139	    expected_token: Vec<u8>,
   140	    module: ModuleConfig,
   141	) -> Result<TransferStats, Status> {
   142	    let start = Instant::now();
   143	    let mut token_buf = vec![0u8; expected_token.len()];
   144	    // R46-F7: bounded wait on the token. A stalled peer that
   145	    // accepted the socket but never sent bytes would otherwise hold
   146	    // this worker indefinitely.
   147	    match tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut token_buf)).await {
   148	        Ok(Ok(_)) => {}
   149	        Ok(Err(err)) => {
   150	            return Err(Status::internal(format!(
   151	                "failed to read data plane token: {}",
   152	                err
   153	            )));
   154	        }
   155	        Err(_elapsed) => {
   156	            return Err(Status::deadline_exceeded(format!(
   157	                "data plane token read timed out after {:?}",
   158	                DATA_PLANE_TOKEN_TIMEOUT
   159	            )));
   160	        }
   161	    }
   162	    if token_buf != expected_token {
   163	        log::warn!("push data plane: invalid token");
   164	        return Err(Status::permission_denied("invalid data plane token"));
   165	    }
   166	    receive_stream_into_module(socket, module, start).await
   167	}
   168	
   169	/// The per-socket receive tail shared by the fixed and resizable
   170	/// accept paths (`ue-r2-2` split it out of `handle_data_plane_stream`
   171	/// so an epoch-N socket runs the identical byte path after its
   172	/// stronger handshake).
   173	async fn receive_stream_into_module(
   174	    socket: TcpStream,
   175	    module: ModuleConfig,
   176	    start: Instant,
   177	) -> Result<TransferStats, Status> {
   178	    eprintln!(
   179	        "blitd: push data plane: token accepted (module='{}', root={})",
   180	        module.name,
   181	        module.path.display()
   182	    );
   183	
   184	    // Route the inbound wire through the unified receive pipeline:
   185	    //   socket → StallGuard → execute_receive_pipeline → FsTransferSink → disk
   186	    // Same call shape as the client's pull-receive side. Tar shards get
   187	    // extracted inline by FsTransferSink (parallelism across streams
   188	    // already comes from N concurrent invocations of this function).
   189	    //
   190	    // audit-h3a (R2/R3 finding H3): symmetric to the audit-1c CLI
   191	    // pull-receive guard. Before this slice the push-receive socket had
   192	    // no idle deadline at all — a hostile or wedged push client that
   193	    // accepted the data plane, sent the token, then went silent would
   194	    // pin this worker indefinitely (DATA_PLANE_TOKEN_TIMEOUT above only
   195	    // bounds the token read). StallGuard turns that into a clean
   196	    // TimedOut after TRANSFER_STALL_TIMEOUT of no progress.
   197	    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
   198	
   199	    let config = FsSinkConfig {
   200	        preserve_times: true,
   201	        dry_run: false,
   202	        checksum: None,
   203	        resume: false,
   204	        compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
   205	    };
   206	    let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
   207	        PathBuf::new(),
   208	        module.path.clone(),
   209	        config,
   210	    ));
   211	    let outcome = receive_push_data_plane(socket, sink)
   212	        .await
   213	        .map_err(|err| Status::internal(format!("data plane receive: {err:#}")))?;
   214	
   215	    let stats = TransferStats {
   216	        files_transferred: outcome.files_written as u64,
   217	        bytes_transferred: outcome.bytes_written,
   218	        bytes_zero_copy: 0,
   219	    };
   220	
   221	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   222	    let gbps = (stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   223	    eprintln!(
   224	        "blitd: push data plane: stream complete: files={}, bytes={} ({:.2} Gbps)",
   225	        stats.files_transferred, stats.bytes_transferred, gbps
   226	    );
   227	    Ok(stats)
   228	}
   229	
   230	// ── ue-r2-2: resizable accept (mid-transfer stream ADD) ──────────────
   231	
   232	/// A control-loop → acceptor registration: the credential the next
   233	/// epoch-N socket must present. Sent BEFORE the daemon acks the ADD,
   234	/// so the accept is armed by the time the client dials.
   235	pub(crate) struct ResizeArm {
   236	    pub(crate) epoch: u32,
   237	    pub(crate) sub_token: Vec<u8>,
   238	}
   239	
   240	struct ArmedEpoch {
   241	    epoch: u32,
   242	    sub_token: Vec<u8>,
   243	    expires: tokio::time::Instant,
   244	}
   245	
   246	/// How long an armed ADD epoch waits for its socket. The client dials
   247	/// immediately after the ack, so an older slot is a failed or
   248	/// abandoned dial. Expiry is NON-fatal: the offer lapses and the
   249	/// transfer continues at its current width (the client settled its
   250	/// side when the dial failed).
   251	const RESIZE_ARM_TTL: Duration = DATA_PLANE_ACCEPT_TIMEOUT;
   252	
   253	/// What a resizable-path socket must present after the one-time token.
   254	enum StreamCredential {
   255	    /// Epoch-0 socket: the negotiation's fixed sub-token; failures are
   256	    /// transfer-fatal, exactly like the fixed path's initial accepts.
   257	    Epoch0(Vec<u8>),
   258	    /// Epoch-N socket: consume a live armed entry. Failures drop the
   259	    /// socket WITHOUT failing the transfer — the accept was an
   260	    /// optional capacity offer, and a stray or hostile dial must not
   261	    /// kill a healthy stream set.
   262	    Armed(Arc<std::sync::Mutex<Vec<ArmedEpoch>>>),
   263	}
   264	
   265	/// `ue-r2-2`: the resize-enabled variant of
   266	/// [`accept_data_connection_stream`]. Epoch 0 behaves exactly like the
   267	/// fixed path (bounded sequential accepts, parallel handshakes,
   268	/// failures fatal); afterwards the listener stays alive for the whole
   269	/// transfer but only accepts while a live armed slot exists — an
   270	/// unarmed listener leaves stray dials in the OS backlog, so the
   271	/// 1g-era "accept phase is bounded" reasoning keeps holding in spirit
   272	/// (every accept is credential-gated and TTL-bounded). Ends when every
   273	/// worker — initial and added — has finished.
   274	pub(crate) async fn accept_data_connection_stream_resizable(
   275	    listener: TcpListener,
   276	    expected_token: Vec<u8>,
   277	    epoch0_sub_token: Vec<u8>,
   278	    module: ModuleConfig,
   279	    stream_count: u32,
   280	    mut arm_rx: tokio::sync::mpsc::UnboundedReceiver<ResizeArm>,
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GJzSAvsL' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ObYLwfwA' (errno=Operation not permitted)
   780	
   781	                maybe_message = response_rx.recv() => {
   782	                    match maybe_message {
   783	                        Some(Ok(message)) => {
   784	                            match message.payload {
   785	                                Some(ServerPayload::Ack(_)) => {}
   786	                                Some(ServerPayload::FilesToUpload(list)) => {
   787	                                    if list.relative_paths.is_empty() {
   788	                                        // Empty terminator — no more need_lists coming.
   789	                                        // Fall through to the bottom of the loop so the
   790	                                        // early-finish check can fire on this iteration;
   791	                                        // don't `continue` (that would skip the check
   792	                                        // and require another response message to wake
   793	                                        // the select, which never arrives).
   794	                                        need_lists_done = true;
   795	                                    } else {
   796	                                    need_list_fresh = true;
   797	                                    let mut rels = list.relative_paths;
   798	                                    files_requested.extend(rels.iter().cloned());
   799	                                    let newly_requested = rels.len();
   800	                                    let mut batch_bytes = 0u64;
   801	                                    for rel in &rels {
   802	                                        requested_files.insert(rel.clone());
   803	                                        if let Some(header) = manifest_lookup.get(rel) {
   804	                                            batch_bytes =
   805	                                                batch_bytes.saturating_add(header.size);
   806	                                        }
   807	                                        // w5-1: was an unconditional per-file
   808	                                        // eprintln — stderr spam proportional
   809	                                        // to file count. Debug-level now;
   810	                                        // visible with BLIT_LOG=debug.
   811	                                        log::debug!("push need-list includes {}", rel);
   812	                                    }
   813	                                    pending_queue.extend(rels.drain(..));
   814	                                    transfer_size_hint =
   815	                                        transfer_size_hint.saturating_add(batch_bytes);
   816	                                    need_list_received = true;
   817	
   818	                                    if !matches!(transfer_mode, TransferMode::Fallback) {
   819	                                        data_plane_outstanding =
   820	                                            data_plane_outstanding.saturating_add(newly_requested);
   821	                                    }
   822	
   823	                                    if let Some(progress) = progress {
   824	                                        if newly_requested > 0 {
   825	                                            progress.report_manifest_batch(newly_requested);
   826	                                        }
   827	                                    }
   828	
   829	                                    match transfer_mode {
   830	                                        TransferMode::Fallback => {
   831	                                            // design-4: hold payloads until the
   832	                                            // daemon's fallback negotiation;
   833	                                            // until then entries just accumulate
   834	                                            // in pending_queue (drained by the
   835	                                            // Negotiation arm).
   836	                                            if fallback_negotiated && need_list_received {
   837	                                                let dial = ensure_dial(
   838	                                                    &mut dial,
   839	                                                    None,
   840	                                                );
   841	                                                let result = stream_fallback_from_queue(
   842	                                                    source.clone(),
   843	                                                    &mut pending_queue,
   844	                                                    &manifest_lookup,
   845	                                                    &tx,
   846	                                                    progress,
   847	                                                    plan_options,
   848	                                                    dial.chunk_bytes(),
   849	                                                    dial.initial_streams(),
   850	                                                    &unreadable_paths,
   851	                                                ).await?;
   852	                                                if result.files_sent > 0 {
   853	                                                    fallback_files_sent =
   854	                                                        fallback_files_sent.saturating_add(result.files_sent);
   855	                                                }
   856	                                                if result.payloads_dispatched
   857	                                                    && first_payload_elapsed.is_none()
   858	                                                {
   859	                                                    first_payload_elapsed = Some(start.elapsed());
   860	                                                }
   861	                                            }
   862	                                        }
   863	                                        TransferMode::DataPlane => {
   864	                                            // sf-2: the need list just grew —
   865	                                            // re-run the shape table and
   866	                                            // correct the stream count before
   867	                                            // queueing the batch.
   868	                                            if resize_negotiated
   869	                                                && shape_resize_enabled
   870	                                                && data_plane_sender.is_some()
   871	                                            {
   872	                                                if let Some(dial_ref) = dial.as_ref() {
   873	                                                    if let Err(send_err) = maybe_shape_resize(
   874	                                                        &tx,
   875	                                                        dial_ref,
   876	                                                        transfer_size_hint,
   877	                                                        files_requested.len(),
   878	                                                        &mut resize_pending,
   879	                                                    )
   880	                                                    .await
   881	                                                    {
   882	                                                        return Err(prefer_server_error(
   883	                                                            &mut response_rx,
   884	                                                            send_err,
   885	                                                        )
   886	                                                        .await);
   887	                                                    }
   888	                                                }
   889	                                            }
   890	                                            if let Some(sender) = data_plane_sender.as_mut() {
   891	                                                let headers =
   892	                                                    drain_pending_headers(&mut pending_queue, &manifest_lookup);
   893	                                                if !headers.is_empty() {
   894	                                                    let headers = source.check_availability(
   895	                                                        headers,
   896	                                                        Arc::clone(&unreadable_paths),
   897	                                                    )
   898	                                                    .await?;
   899	                                                    if headers.is_empty() {
   900	                                                        continue;
   901	                                                    }
   902	                                                    // Dial exists before the first
   903	                                                    // data-plane batch (first-wins).
   904	                                                    ensure_dial(&mut dial, None);
   905	                                            let planned =
   906	                                                plan_transfer_payloads(headers, source_root, plan_options)?;
   907	                                            for payload in &planned {
   908	                                                match payload {
   909	                                                    TransferPayload::File(header) => {
   910	                                                        // w5-1: was unconditional per-file
   911	                                                        // eprintln; BLIT_LOG=debug shows it.
   912	                                                        log::debug!(
   913	                                                            "push enqueue {} for TCP stream",
   914	                                                            header.relative_path
   915	                                                        );
   916	                                                    }
   917	                                                    TransferPayload::TarShard { headers } => {
   918	                                                        for header in headers {
   919	                                                            log::debug!(
   920	                                                                "push enqueue {} via tar shard",
   921	                                                                header.relative_path
   922	                                                            );
   923	                                                        }
   924	                                                    }
   925	                                                    TransferPayload::FileBlock { .. }
   926	                                                    | TransferPayload::FileBlockComplete { .. }
   927	                                                    | TransferPayload::ResumeFile { .. } => {
   928	                                                        // Never produced by the outbound planner.
   929	                                                    }
   930	                                                }
   931	                                            }
   932	                                            if !planned.is_empty() {
   933	                                                        let sent = payload_file_count(&planned);
   934	                                                        sender.queue(planned).await?;
   935	                                                        if sent > 0 && first_payload_elapsed.is_none() {
   936	                                                            first_payload_elapsed = Some(start.elapsed());
   937	                                                        }
   938	                                                        data_plane_files_sent += sent;
   939	                                                        data_plane_outstanding =
   940	                                                            data_plane_outstanding.saturating_sub(sent);
   941	                                                    }
   942	                                                }
   943	                                            }
   944	                                        }
   945	                                        TransferMode::Undecided => {}
   946	                                    }
   947	                                    } // end else (non-empty need_list)
   948	                                }
   949	                                Some(ServerPayload::Negotiation(neg)) => {
   950	                                    if neg.tcp_fallback {
   951	                                        fallback_used = true;
   952	                                        transfer_mode = TransferMode::Fallback;
   953	                                        // design-4: only now may fallback
   954	                                        // payloads flow — the daemon is past
   955	                                        // its manifest loop and ready to
   956	                                        // receive FileData.
   957	                                        fallback_negotiated = true;
   958	
   959	                                            if need_list_received {
   960	                                            let dial = ensure_dial(
   961	                                                &mut dial,
   962	                                                neg.receiver_capacity.as_ref(),
   963	                                            );
   964	                                            let result = stream_fallback_from_queue(
   965	                                                source.clone(),
   966	                                                &mut pending_queue,
   967	                                                &manifest_lookup,
   968	                                                &tx,
   969	                                                progress,
   970	                                                plan_options,
   971	                                                dial.chunk_bytes(),
   972	                                                dial.prefetch_count(),
   973	                                                &unreadable_paths,
   974	                                            ).await?;
   975	                                            if result.files_sent > 0 {
   976	                                                fallback_files_sent =
   977	                                                    fallback_files_sent.saturating_add(result.files_sent);
   978	                                            }
   979	                                            if result.payloads_dispatched
   980	                                                && first_payload_elapsed.is_none()
   981	                                            {
   982	                                                first_payload_elapsed = Some(start.elapsed());
   983	                                            }
   984	                                        }
   985	
   986	                                        data_plane_outstanding = 0;
   987	                                        if let Some(sender) = data_plane_sender.take() {
   988	                                            sender.finish().await?;
   989	                                        }
   990	                                    } else {

exec
/bin/zsh -lc "git grep -n \"initial_stream_proposal\\|start_data_plane\\|data_plane_sender\\|MANIFEST_BATCH\\|manifest_batch\\|flush_manifest\\|128\" 0f922de -- crates/blit-core/src/remote/push/client/mod.rs crates/blit-core/src/remote/push/client/helpers.rs crates/blit-daemon/src/service/push; git show 0f922de:crates/blit-core/src/remote/push/client/mod.rs | nl -ba | sed -n '300,780p'; git show 0f922de:crates/blit-core/src/remote/push/client/mod.rs | nl -ba | sed -n '1220,1430p'; git diff 0f922de HEAD -- crates/blit-core/src/remote/transfer/sink.rs crates/blit-core/src/remote/transfer/pipeline.rs crates/blit-core/src/remote/transfer/diff_planner.rs crates/blit-core/src/transfer_plan.rs crates/blit-core/src/dial.rs | sed -n '1,1200p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jHDnU0Vi' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-RFYVXl4Y' (errno=Operation not permitted)
0f922de:crates/blit-core/src/remote/push/client/mod.rs:523:/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
0f922de:crates/blit-core/src/remote/push/client/mod.rs:545:        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
0f922de:crates/blit-core/src/remote/push/client/mod.rs:721:        let mut data_plane_sender: Option<MultiStreamSender> = None;
0f922de:crates/blit-core/src/remote/push/client/mod.rs:741:        // every forced-gRPC push of ≥128 files (one early need-list flush)
0f922de:crates/blit-core/src/remote/push/client/mod.rs:825:                                            progress.report_manifest_batch(newly_requested);
0f922de:crates/blit-core/src/remote/push/client/mod.rs:870:                                                && data_plane_sender.is_some()
0f922de:crates/blit-core/src/remote/push/client/mod.rs:890:                                            if let Some(sender) = data_plane_sender.as_mut() {
0f922de:crates/blit-core/src/remote/push/client/mod.rs:987:                                        if let Some(sender) = data_plane_sender.take() {
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1005:                                        if data_plane_sender.is_none() {
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1038:                                            data_plane_sender = Some(sender);
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1065:                                        if let Some(sender) = data_plane_sender.as_mut() {
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1124:                                                let added = match data_plane_sender.as_mut() {
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1177:                                                && data_plane_sender.is_some()
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1281:                                    if let Some(sender) = data_plane_sender.as_mut() {
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1416:                                let retired = data_plane_sender
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1484:                if let Some(sender) = data_plane_sender.take() {
0f922de:crates/blit-core/src/remote/push/client/mod.rs:1494:        if let Some(sender) = data_plane_sender.take() {
0f922de:crates/blit-daemon/src/service/push/control.rs:30:const FILE_LIST_EARLY_FLUSH_ENTRIES: usize = 128;
0f922de:crates/blit-daemon/src/service/push/control.rs:48:/// spin-up) within milliseconds, not after 128 entries trickle in.
0f922de:crates/blit-daemon/src/service/push/control.rs:49:/// Under a fast manifest stream 128 entries arrive well inside this
0f922de:crates/blit-daemon/src/service/push/control.rs:233:                    // That broke every forced-gRPC push of ≥128 files
0f922de:crates/blit-daemon/src/service/push/control.rs:800:    blit_core::engine::initial_stream_proposal(
0f922de:crates/blit-daemon/src/service/push/control.rs:958:        // manifest must not wait for 128 entries to see its first
0f922de:crates/blit-daemon/src/service/push/shape_resize_e2e.rs:6://! flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128 entries), so a 10k-file
0f922de:crates/blit-daemon/src/service/push/shape_resize_e2e.rs:7://! push used to negotiate from a ~128-file prefix — 1 stream — and ride
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lZUtSXsA' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-upIlgJkf' (errno=Operation not permitted)
   300	                sinks,
   301	                payload_rx,
   302	                prefetch,
   303	                progress.as_ref(),
   304	                Some(ctl_rx),
   305	            )
   306	            .await
   307	        }));
   308	
   309	        Ok(Self {
   310	            payload_tx: Some(payload_tx),
   311	            tuner_handle,
   312	            pipeline_handle: Some(pipeline_handle),
   313	            started: Instant::now(),
   314	            resize,
   315	            resize_rx,
   316	        })
   317	    }
   318	
   319	    /// ue-r2-2: the tuner's proposal stream (present only when resize
   320	    /// was negotiated). The control loop takes it once and correlates
   321	    /// proposals with the daemon's acks.
   322	    fn take_resize_rx(
   323	        &mut self,
   324	    ) -> Option<tokio::sync::mpsc::UnboundedReceiver<crate::engine::ResizeProposal>> {
   325	        self.resize_rx.take()
   326	    }
   327	
   328	    /// ue-r2-2 ADD: dial one more data socket with the per-epoch
   329	    /// credential (token ‖ sub_token), register its probe with the
   330	    /// tuner, and hand its sink to the running pipeline. Errors are
   331	    /// the caller's to treat as NON-fatal — a failed optional ADD
   332	    /// must never kill a healthy transfer (the daemon's armed accept
   333	    /// slot simply expires).
   334	    async fn add_stream(&mut self, sub_token: &[u8]) -> Result<()> {
   335	        use crate::remote::transfer::progress::{LiveProbe, StreamId, StreamProbe};
   336	        let rt = self
   337	            .resize
   338	            .as_mut()
   339	            .ok_or_else(|| eyre!("resize was not negotiated for this transfer"))?;
   340	        let probe = StreamProbe::new(StreamId(rt.next_stream_id));
   341	        let tuner_probe = StreamProbe::from_telemetry(probe.id(), probe.telemetry());
   342	        let mut handshake = rt.token.clone();
   343	        handshake.extend_from_slice(sub_token);
   344	        let session = DataPlaneSession::connect_with_probe(
   345	            &rt.host,
   346	            rt.port,
   347	            &handshake,
   348	            // Live dial values: an epoch-N socket starts at the
   349	            // CURRENT tuning, not the connect-time snapshot.
   350	            rt.dial.chunk_bytes(),
   351	            rt.dial.prefetch_count(),
   352	            rt.trace,
   353	            rt.dial.tcp_buffer_bytes(),
   354	            Arc::clone(&rt.pool),
   355	            LiveProbe(probe),
   356	        )
   357	        .await?;
   358	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   359	            session,
   360	            rt.source.clone(),
   361	            rt.dst_root.clone(),
   362	        ));
   363	        if let Err(returned) = rt.ctl_tx.send(SinkControl::Add(sink)) {
   364	            // Pipeline already finished (transfer completing under the
   365	            // ADD). Close the just-authorized socket CLEANLY — the END
   366	            // record keeps the daemon's epoch-N worker from dying on a
   367	            // reset, which would fail an otherwise-complete push
   368	            // (post-handshake stream errors are fatal by design).
   369	            if let SinkControl::Add(sink) = returned.0 {
   370	                let _ = sink.finish().await;
   371	            }
   372	            return Err(eyre!("data plane pipeline is no longer running"));
   373	        }
   374	        rt.next_stream_id += 1;
   375	        rt.probes
   376	            .lock()
   377	            .expect("probe registry poisoned")
   378	            .push(tuner_probe);
   379	        Ok(())
   380	    }
   381	
   382	    /// ue-r2-2 REMOVE: retire the most recently added live stream —
   383	    /// its worker drains at the payload boundary and emits its END —
   384	    /// and drop its probe from the tuner registry. Returns false when
   385	    /// nothing can be retired (floor of one stream, or the pipeline is
   386	    /// gone), so the caller can settle the epoch as refused. The probe
   387	    /// pops only AFTER the pipeline accepted the retire (review: the
   388	    /// old order lost a probe when the pipeline was already gone).
   389	    fn retire_stream(&mut self) -> bool {
   390	        let Some(rt) = self.resize.as_mut() else {
   391	            return false;
   392	        };
   393	        {
   394	            let probes = rt.probes.lock().expect("probe registry poisoned");
   395	            if probes.len() <= 1 {
   396	                return false;
   397	            }
   398	        }
   399	        if rt.ctl_tx.send(SinkControl::RetireOne).is_err() {
   400	            return false;
   401	        }
   402	        rt.probes.lock().expect("probe registry poisoned").pop();
   403	        true
   404	    }
   405	
   406	    /// Feed one or more payloads to the streaming pipeline.
   407	    async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   408	        let tx = self
   409	            .payload_tx
   410	            .as_ref()
   411	            .ok_or_else(|| eyre!("data plane sender already finished"))?;
   412	        for payload in payloads {
   413	            if tx.send(payload).await.is_err() {
   414	                // Receiver dropped → pipeline task already exited.
   415	                // Drain `pipeline_handle` to surface the underlying
   416	                // error (sink worker errored, remote daemon closed,
   417	                // disk full on dest…) instead of the previous
   418	                // generic "data plane pipeline closed unexpectedly".
   419	                // POST_REVIEW_FIXES §1.1b.
   420	                drop(self.payload_tx.take());
   421	                let handle = self
   422	                    .pipeline_handle
   423	                    .take()
   424	                    .ok_or_else(|| eyre!("data plane pipeline handle missing"))?;
   425	                return Err(drain_pipeline_error(handle).await);
   426	            }
   427	        }
   428	        Ok(())
   429	    }
   430	
   431	    /// Close the payload channel and wait for the pipeline to drain.
   432	    async fn finish(mut self) -> Result<()> {
   433	        // ue-r2-1e: stop the tuner promptly (it would otherwise idle
   434	        // until its Weak<dial> dies at the end of the push).
   435	        if let Some(tuner) = self.tuner_handle.take() {
   436	            tuner.abort();
   437	        }
   438	        // Drop the sender so the pipeline sees end-of-stream.
   439	        drop(self.payload_tx.take());
   440	        let handle = self
   441	            .pipeline_handle
   442	            .take()
   443	            .ok_or_else(|| eyre!("data plane pipeline handle missing"))?;
   444	        // Route both Ok and Err through the shared drain helper so
   445	        // the failure-path wrapping ("data plane pipeline failed:
   446	        // <cause>" / "data plane pipeline panicked: <join>") matches
   447	        // exactly what `queue()` would produce. R43 follow-up to
   448	        // R42-F2 — earlier this was a hand-rolled match that
   449	        // duplicated the helper's arms.
   450	        let outcome = drain_pipeline_outcome(handle).await?;
   451	        let elapsed = self.started.elapsed().as_secs_f64().max(1e-6);
   452	        let throughput = (outcome.bytes_written as f64 * 8.0) / elapsed / 1e9;
   453	        eprintln!(
   454	            "[data-plane-client] aggregate {:.2} Gbps ({:.2} MiB in {:.2}s)",
   455	            throughput.max(0.0),
   456	            outcome.bytes_written as f64 / 1024.0 / 1024.0,
   457	            elapsed
   458	        );
   459	        Ok(())
   460	    }
   461	}
   462	
   463	/// ue-r2-1e: one dial per push, created at first need. Replaces the
   464	/// memoized size-keyed `determine_remote_tuning` ladder: conservative
   465	/// start, ceilings bounded by the daemon's advertised receiver profile
   466	/// when the negotiation carried one (first-wins, like the old memo).
   467	fn ensure_dial(
   468	    dial: &mut Option<Arc<crate::engine::TransferDial>>,
   469	    receiver_capacity: Option<&crate::generated::CapacityProfile>,
   470	) -> Arc<crate::engine::TransferDial> {
   471	    if dial.is_none() {
   472	        *dial = Some(crate::engine::TransferDial::conservative_within(receiver_capacity).shared());
   473	    }
   474	    dial.as_ref()
   475	        .cloned()
   476	        .expect("dial set by preceding assignment")
   477	}
   478	
   479	/// ue-r2-2 / sf-2 shared pre-dial ADD: mint the epoch credential, send
   480	/// the `DataPlaneResize` ADD, and record the in-flight epoch (the
   481	/// socket itself is dialed on the daemon's ack). A missing credential
   482	/// source settles the epoch failed and is not an error; a send error
   483	/// is returned for the caller to route through `prefer_server_error`.
   484	async fn send_resize_add(
   485	    tx: &mpsc::Sender<ClientPushRequest>,
   486	    dial: &crate::engine::TransferDial,
   487	    proposal: crate::engine::ResizeProposal,
   488	    resize_pending: &mut Option<PendingResize>,
   489	) -> Result<()> {
   490	    match crate::remote::transfer::generate_sub_token() {
   491	        Ok(sub) => {
   492	            send_payload(
   493	                tx,
   494	                ClientPayload::DataPlaneResize(DataPlaneResize {
   495	                    op: DataPlaneResizeOp::Add as i32,
   496	                    epoch: proposal.epoch,
   497	                    target_stream_count: proposal.target_streams as u32,
   498	                    sub_token: sub.clone(),
   499	                }),
   500	            )
   501	            .await?;
   502	            *resize_pending = Some(PendingResize {
   503	                epoch: proposal.epoch,
   504	                target: proposal.target_streams,
   505	                add: true,
   506	                sub_token: sub,
   507	            });
   508	        }
   509	        Err(err) => {
   510	            log::warn!("resize ADD skipped (no credential source): {err:#}");
   511	            dial.resize_settled(proposal.epoch, dial.live_streams(), false);
   512	        }
   513	    }
   514	    Ok(())
   515	}
   516	
   517	/// sf-2: one shape-correction step. The daemon proposes the epoch-0
   518	/// stream count from whatever manifest prefix it had seen at its early
   519	/// flush, so a many-tiny-file push can negotiate far fewer streams
   520	/// than the shape table assigns the full workload
   521	/// (`.review/findings/sf-1-tripwire-harness.md` Known gaps: a
   522	/// 1000-file push measured 1 stream where the table says 2). As the
   523	/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
   524	/// over the ACTUAL transfer shape (need-list files + bytes, not the
   525	/// manifest — an incremental push of a large tree may move only a few
   526	/// files) and correct upward one ADD epoch at a time. Call sites gate
   527	/// on the transfer running resize-enabled on the data plane.
   528	///
   529	/// `need_bytes`/`need_count` must come from the append-only
   530	/// accumulators (`transfer_size_hint`, `files_requested`) — NOT from
   531	/// `requested_files`, which `prune_unrequested_payloads` drains as
   532	/// payloads are matched (codex sf-2 review: the drained set undercounts
   533	/// the shape and can stall the ramp below the table's target).
   534	async fn maybe_shape_resize(
   535	    tx: &mpsc::Sender<ClientPushRequest>,
   536	    dial: &crate::engine::TransferDial,
   537	    need_bytes: u64,
   538	    need_count: usize,
   539	    resize_pending: &mut Option<PendingResize>,
   540	) -> Result<()> {
   541	    if resize_pending.is_some() {
   542	        return Ok(());
   543	    }
   544	    let target =
   545	        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
   546	            as usize;
   547	    match dial.propose_shape_resize(target) {
   548	        Some(proposal) => send_resize_add(tx, dial, proposal, resize_pending).await,
   549	        None => Ok(()),
   550	    }
   551	}
   552	
   553	fn prune_unrequested_payloads(
   554	    payloads: &mut Vec<TransferPayload>,
   555	    requested: &mut HashSet<String>,
   556	) -> usize {
   557	    let mut filtered: Vec<TransferPayload> = Vec::with_capacity(payloads.len());
   558	    let mut skipped = 0usize;
   559	
   560	    for payload in payloads.drain(..) {
   561	        match payload {
   562	            TransferPayload::File(header) => {
   563	                if requested.remove(header.relative_path.as_str()) {
   564	                    filtered.push(TransferPayload::File(header));
   565	                } else {
   566	                    skipped += 1;
   567	                }
   568	            }
   569	            // Resume payloads (per-block, and otp-7b's session-only
   570	            // composite) never route through the old push prune path.
   571	            TransferPayload::FileBlock { .. }
   572	            | TransferPayload::FileBlockComplete { .. }
   573	            | TransferPayload::ResumeFile { .. } => {
   574	                skipped += 1;
   575	            }
   576	            TransferPayload::TarShard { headers } => {
   577	                let mut kept_headers = Vec::with_capacity(headers.len());
   578	                for header in headers {
   579	                    if requested.remove(header.relative_path.as_str()) {
   580	                        kept_headers.push(header);
   581	                    } else {
   582	                        skipped += 1;
   583	                    }
   584	                }
   585	                if !kept_headers.is_empty() {
   586	                    filtered.push(TransferPayload::TarShard {
   587	                        headers: kept_headers,
   588	                    });
   589	                }
   590	            }
   591	        }
   592	    }
   593	
   594	    payloads.extend(filtered);
   595	    skipped
   596	}
   597	
   598	pub struct RemotePushClient {
   599	    endpoint: RemoteEndpoint,
   600	    client: crate::generated::blit_client::BlitClient<tonic::transport::Channel>,
   601	}
   602	
   603	impl RemotePushClient {
   604	    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
   605	        let uri = endpoint.control_plane_uri();
   606	        // audit-2: bound the connect (30s). Plain `BlitClient::connect`
   607	        // has no deadline, so an unreachable destination daemon would
   608	        // hang a remote push for the OS TCP timeout (60-127s). The outer
   609	        // `tokio::time::timeout` is what bounds slow DNS too —
   610	        // `connect_timeout` alone only bounds the post-resolution TCP
   611	        // attempt (tonic/hyper-util resolve the name first).
   612	        let conn = tonic::transport::Endpoint::from_shared(uri.clone())
   613	            .map_err(|err| eyre::eyre!("invalid endpoint {}: {}", uri, err))?
   614	            .connect_timeout(std::time::Duration::from_secs(30));
   615	        let channel = tokio::time::timeout(std::time::Duration::from_secs(30), conn.connect())
   616	            .await
   617	            .map_err(|_| eyre::eyre!("connecting to {} timed out", uri))?
   618	            .map_err(|err| eyre::eyre!("failed to connect to {}: {}", uri, err))?;
   619	        let client = crate::generated::blit_client::BlitClient::new(channel);
   620	
   621	        Ok(Self { endpoint, client })
   622	    }
   623	
   624	    pub async fn push(
   625	        &mut self,
   626	        source: Arc<dyn TransferSource>,
   627	        filter: &FileFilter,
   628	        mirror_mode: bool,
   629	        mirror_kind: crate::generated::MirrorMode,
   630	        force_grpc: bool,
   631	        require_complete_scan: bool,
   632	        progress: Option<&RemotePushProgress>,
   633	        trace_data_plane: bool,
   634	    ) -> Result<RemotePushReport> {
   635	        let source_root = source.root();
   636	        // We don't check source_root.exists() here because source might be remote/virtual.
   637	        // If it's FsTransferSource, it should have been checked before creation or we trust it.
   638	
   639	        let start = Instant::now();
   640	        let mut first_payload_elapsed: Option<Duration> = None;
   641	
   642	        let mut manifest_lookup: HashMap<String, FileHeader> = HashMap::new();
   643	        let mut requested_files: HashSet<String> = HashSet::new();
   644	        let plan_options = PlanOptions::default();
   645	        let mut dial: Option<Arc<crate::engine::TransferDial>> = None;
   646	        let mut manifest_total_bytes: u64 = 0;
   647	        let mut transfer_size_hint: u64 = 0;
   648	
   649	        let (tx, rx) = mpsc::channel(32);
   650	        let outbound = ReceiverStream::new(rx);
   651	
   652	        let response_stream = self
   653	            .client
   654	            .push(outbound)
   655	            .await
   656	            .map_err(map_status)?
   657	            .into_inner();
   658	        let (mut response_rx, response_task) = spawn_response_task(response_stream);
   659	
   660	        let (module, rel_path) = module_and_path(&self.endpoint)?;
   661	        let destination_path = destination_path(&rel_path);
   662	
   663	        // R59 #1 F2: translate the client's FileFilter to wire FilterSpec
   664	        // so the daemon's purge enumerator can honor scope. Pre-fix the
   665	        // daemon used FileFilter::default() and would delete user-excluded
   666	        // destination entries it considered "extraneous".
   667	        let wire_filter = crate::generated::FilterSpec {
   668	            include: filter.include_files.clone(),
   669	            exclude: filter.exclude_files.clone(),
   670	            min_size: filter.min_size,
   671	            max_size: filter.max_size,
   672	            min_age_secs: filter.min_age.map(|d| d.as_secs()),
   673	            max_age_secs: filter.max_age.map(|d| d.as_secs()),
   674	            files_from: filter
   675	                .files_from
   676	                .as_ref()
   677	                .map(|set| {
   678	                    set.iter()
   679	                        .map(|p| p.to_string_lossy().into_owned())
   680	                        .collect()
   681	                })
   682	                .unwrap_or_default(),
   683	        };
   684	        if let Err(send_err) = send_payload(
   685	            &tx,
   686	            ClientPayload::Header(crate::generated::PushHeader {
   687	                module,
   688	                mirror_mode,
   689	                destination_path,
   690	                force_grpc,
   691	                filter: Some(wire_filter),
   692	                mirror_kind: mirror_kind as i32,
   693	                require_complete_scan,
   694	                // ue-r2-2: the client dials and its pipeline is
   695	                // elastic — advertise resize. The daemon folds this
   696	                // with its own support + the TCP-path conditions into
   697	                // `resize_enabled`; against an old daemon the bit is
   698	                // skipped and nothing changes.
   699	                supports_stream_resize: true,
   700	            }),
   701	        )
   702	        .await
   703	        {
   704	            return Err(prefer_server_error(&mut response_rx, send_err).await);
   705	        }
   706	
   707	        let unreadable_paths: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
   708	
   709	        let (manifest_rx, manifest_task) = source.scan(
   710	            Some(filter.clone_without_cache()),
   711	            Arc::clone(&unreadable_paths),
   712	        );
   713	
   714	        let mut manifest_rx = manifest_rx;
   715	
   716	        let mut files_requested: Vec<String> = Vec::new();
   717	        let mut pending_queue: VecDeque<String> = VecDeque::new();
   718	        let mut fallback_upload_complete_sent = false;
   719	        let mut fallback_files_sent: usize = 0;
   720	        let mut need_list_received = false;
   721	        let mut data_plane_sender: Option<MultiStreamSender> = None;
   722	        let mut data_plane_outstanding: usize = 0;
   723	        let mut data_plane_files_sent: usize = 0;
   724	        let mut data_port: Option<u32> = None;
   725	        let mut fallback_used = force_grpc;
   726	        let mut summary: Option<PushSummary> = None;
   727	
   728	        let mut transfer_mode = if force_grpc {
   729	            TransferMode::Fallback
   730	        } else {
   731	            TransferMode::Undecided
   732	        };
   733	        // design-4: the daemon's wire contract rejects FileData while its
   734	        // manifest loop is still running ("data payload received before
   735	        // negotiation"). Even in forced-gRPC mode the client must therefore
   736	        // hold its fallback payloads until the daemon announces
   737	        // Negotiation(tcp_fallback) — which the daemon only sends after it
   738	        // has seen ManifestComplete. Pre-fix, force_grpc initialized
   739	        // Fallback mode and the first mid-manifest need-list batch
   740	        // triggered FileData sends that raced the daemon's manifest loop:
   741	        // every forced-gRPC push of ≥128 files (one early need-list flush)
   742	        // died, and ~100 files was a coin flip.
   743	        let mut fallback_negotiated = false;
   744	
   745	        // ue-r2-2: resize controller state. The tuner's proposal stream
   746	        // appears once a resize-enabled negotiation lands;
   747	        // `resize_pending` is the single epoch awaiting the daemon's
   748	        // ack (the dial enforces one-in-flight too).
   749	        let mut resize_proposal_rx: Option<
   750	            tokio::sync::mpsc::UnboundedReceiver<crate::engine::ResizeProposal>,
   751	        > = None;
   752	        let mut resize_pending: Option<PendingResize> = None;
   753	        // sf-2: shape-correction gate. `resize_negotiated` records that
   754	        // this transfer's data plane went elastic (epoch-0 sub-token
   755	        // present). `shape_resize_enabled` flips off permanently the
   756	        // first time the tuner proposes a REMOVE — live throughput
   757	        // evidence outranks the static shape table, and re-adding what
   758	        // the tuner just retired would flap.
   759	        let mut resize_negotiated = false;
   760	        let mut shape_resize_enabled = true;
   761	
   762	        let mut manifest_done = false;
   763	        // Track whether we received new need-list entries this iteration.
   764	        // Don't finish the data plane until a full iteration passes with
   765	        // no new entries — this ensures all in-flight gRPC batches arrive.
   766	        let mut need_list_fresh: bool;
   767	        // Set when the daemon signals "no more need_lists coming" by
   768	        // sending an empty FilesToUpload terminator. Gates the early
   769	        // finish() so we don't close the data plane while the daemon
   770	        // is still streaming need_list batches.
   771	        let mut need_lists_done = false;
   772	        loop {
   773	            if manifest_done && summary.is_some() {
   774	                break;
   775	            }
   776	            need_list_fresh = false;
   777	
   778	            tokio::select! {
   779	                biased;
   780	
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-IV9TR0ud' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lrMfjpg4' (errno=Operation not permitted)
  1220	                            } else {
  1221	                                header.relative_path.clone()
  1222	                            };
  1223	                            let mut header = header;
  1224	                            header.relative_path = rel.clone();
  1225	
  1226	                            // Check availability via the source abstraction
  1227	                            let available = source.check_availability(vec![header.clone()], Arc::clone(&unreadable_paths)).await?;
  1228	                            if available.is_empty() {
  1229	                                continue;
  1230	                            }
  1231	
  1232	                            manifest_total_bytes =
  1233	                                manifest_total_bytes.saturating_add(header.size);
  1234	                            // design-5: if the daemon already rejected the
  1235	                            // push (e.g. read-only module), this send loses
  1236	                            // a race with the terminal status — surface the
  1237	                            // daemon's reason, not the transport symptom.
  1238	                            if let Err(send_err) =
  1239	                                send_payload(&tx, ClientPayload::FileManifest(header.clone()))
  1240	                                    .await
  1241	                            {
  1242	                                return Err(
  1243	                                    prefer_server_error(&mut response_rx, send_err).await
  1244	                                );
  1245	                            }
  1246	                            manifest_lookup.insert(rel.clone(), header);
  1247	
  1248	                            match transfer_mode {
  1249	                                TransferMode::Fallback => {
  1250	                                    // design-4: never interleave FileData
  1251	                                    // between our own manifest sends — wait
  1252	                                    // for the daemon's fallback negotiation.
  1253	                                    if fallback_negotiated && need_list_received {
  1254	                                        let dial = ensure_dial(
  1255	                                            &mut dial,
  1256	                                            None,
  1257	                                        );
  1258	                                        let result = stream_fallback_from_queue(
  1259	                                            source.clone(),
  1260	                                            &mut pending_queue,
  1261	                                            &manifest_lookup,
  1262	                                            &tx,
  1263	                                            progress,
  1264	                                            plan_options,
  1265	                                            dial.chunk_bytes(),
  1266	                                            dial.initial_streams(),
  1267	                                            &unreadable_paths,
  1268	                                        ).await?;
  1269	                                        if result.files_sent > 0 {
  1270	                                            fallback_files_sent =
  1271	                                                fallback_files_sent.saturating_add(result.files_sent);
  1272	                                        }
  1273	                                        if result.payloads_dispatched
  1274	                                            && first_payload_elapsed.is_none()
  1275	                                        {
  1276	                                            first_payload_elapsed = Some(start.elapsed());
  1277	                                        }
  1278	                                    }
  1279	                                }
  1280	                                TransferMode::DataPlane => {
  1281	                                    if let Some(sender) = data_plane_sender.as_mut() {
  1282	                                        let headers =
  1283	                                            drain_pending_headers(&mut pending_queue, &manifest_lookup);
  1284	                                        if !headers.is_empty() {
  1285	                                            let headers = source.check_availability(
  1286	                                                headers,
  1287	                                                Arc::clone(&unreadable_paths),
  1288	                                            )
  1289	                                            .await?;
  1290	                                            if headers.is_empty() {
  1291	                                                continue;
  1292	                                            }
  1293	                                            // Dial exists before the first
  1294	                                            // data-plane batch (first-wins).
  1295	                                            ensure_dial(&mut dial, None);
  1296	                                            let mut planned =
  1297	                                                plan_transfer_payloads(headers, source_root, plan_options)?;
  1298	                                            let skipped = prune_unrequested_payloads(
  1299	                                                &mut planned,
  1300	                                                &mut requested_files,
  1301	                                            );
  1302	                                            if skipped > 0 {
  1303	                                                log::debug!(
  1304	                                                    "push: daemon did not request {} payload file(s); skipping",
  1305	                                                    skipped
  1306	                                                );
  1307	                                            }
  1308	                                            for payload in &planned {
  1309	                                                match payload {
  1310	                                                    TransferPayload::File(header) => {
  1311	                                                        // w5-1: was unconditional per-file
  1312	                                                        // eprintln; BLIT_LOG=debug shows it.
  1313	                                                        log::debug!(
  1314	                                                            "push enqueue {} for TCP stream",
  1315	                                                            header.relative_path
  1316	                                                        );
  1317	                                                    }
  1318	                                                    TransferPayload::TarShard { headers } => {
  1319	                                                        for header in headers {
  1320	                                                            log::debug!(
  1321	                                                                "push enqueue {} via tar shard",
  1322	                                                                header.relative_path
  1323	                                                            );
  1324	                                                        }
  1325	                                                    }
  1326	                                                    TransferPayload::FileBlock { .. }
  1327	                                                    | TransferPayload::FileBlockComplete { .. }
  1328	                                                    | TransferPayload::ResumeFile { .. } => {
  1329	                                                        // Never produced by the outbound planner.
  1330	                                                    }
  1331	                                                }
  1332	                                            }
  1333	                                            if !planned.is_empty() {
  1334	                                                let sent = payload_file_count(&planned);
  1335	                                                sender.queue(planned).await?;
  1336	                                                if sent > 0 && first_payload_elapsed.is_none() {
  1337	                                                    first_payload_elapsed = Some(start.elapsed());
  1338	                                                }
  1339	                                                data_plane_files_sent += sent;
  1340	                                                data_plane_outstanding =
  1341	                                                    data_plane_outstanding.saturating_sub(sent);
  1342	                                            }
  1343	                                        }
  1344	                                    }
  1345	                                }
  1346	                                TransferMode::Undecided => {}
  1347	                            }
  1348	                        }
  1349	                        None => {
  1350	                            manifest_done = true;
  1351	                            // R59 #1 F1: report scan completeness to the
  1352	                            // daemon at ManifestComplete time. Walkdir
  1353	                            // errors land in `unreadable_paths` synchronously
  1354	                            // during the scan; the channel closing (None)
  1355	                            // guarantees the manifest task has finished
  1356	                            // pushing them, so reading here is race-free.
  1357	                            let scan_complete = unreadable_paths
  1358	                                .lock()
  1359	                                .map(|g| g.is_empty())
  1360	                                .unwrap_or(false);
  1361	                            if let Err(send_err) =
  1362	                                send_manifest_complete(&tx, scan_complete).await
  1363	                            {
  1364	                                return Err(
  1365	                                    prefer_server_error(&mut response_rx, send_err).await
  1366	                                );
  1367	                            }
  1368	                        }
  1369	                    }
  1370	                }
  1371	
  1372	                // ue-r2-2: the tuner proposed a stream-count change.
  1373	                // Lowest select priority (biased): control frames and
  1374	                // manifest flow always come first, and at most one
  1375	                // epoch is in flight.
  1376	                proposal = async {
  1377	                    match resize_proposal_rx.as_mut() {
  1378	                        Some(rx) => rx.recv().await,
  1379	                        None => std::future::pending().await,
  1380	                    }
  1381	                }, if resize_pending.is_none() => {
  1382	                    match proposal {
  1383	                        Some(p) => {
  1384	                            let dial_ref = dial
  1385	                                .as_ref()
  1386	                                .expect("resize only negotiated on the dial path");
  1387	                            if p.add {
  1388	                                // Pre-dial ADD: mint the epoch credential,
  1389	                                // ask the daemon to register it and arm an
  1390	                                // accept; the dial happens on the ack.
  1391	                                if let Err(send_err) =
  1392	                                    send_resize_add(&tx, dial_ref, p, &mut resize_pending).await
  1393	                                {
  1394	                                    return Err(prefer_server_error(
  1395	                                        &mut response_rx,
  1396	                                        send_err,
  1397	                                    )
  1398	                                    .await);
  1399	                                }
  1400	                            } else {
  1401	                                // sf-2: the tuner wants FEWER streams — live
  1402	                                // throughput evidence outranks the static
  1403	                                // shape table from here on. Never re-add what
  1404	                                // the tuner retires.
  1405	                                shape_resize_enabled = false;
  1406	                                // REMOVE: retire locally first — the drained
  1407	                                // worker's END record is the daemon-side
  1408	                                // teardown — then tell the daemon
  1409	                                // (accounting). Settle IMMEDIATELY with what
  1410	                                // actually happened (review: the retire is
  1411	                                // fait accompli; waiting on the
  1412	                                // accounting-only ack could diverge the dial
  1413	                                // from the real worker count on a refusal).
  1414	                                // The daemon's ack then matches no pending
  1415	                                // epoch and is ignored as unsolicited.
  1416	                                let retired = data_plane_sender
  1417	                                    .as_mut()
  1418	                                    .map(|s| s.retire_stream())
  1419	                                    .unwrap_or(false);
  1420	                                if retired {
  1421	                                    if let Err(send_err) = send_payload(
  1422	                                        &tx,
  1423	                                        ClientPayload::DataPlaneResize(DataPlaneResize {
  1424	                                            op: DataPlaneResizeOp::Remove as i32,
  1425	                                            epoch: p.epoch,
  1426	                                            target_stream_count: p.target_streams as u32,
  1427	                                            sub_token: Vec::new(),
  1428	                                        }),
  1429	                                    )
  1430	                                    .await
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-z1r75deZ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ipqCEh3W' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/dial.rs b/crates/blit-core/src/dial.rs
new file mode 100644
index 0000000..611820c
--- /dev/null
+++ b/crates/blit-core/src/dial.rs
@@ -0,0 +1,1051 @@
+//! The single live transfer dial (`ue-r2-1e`, REV4 Design §4).
+//!
+//! Sender-owned, receiver-bounded: the byte sender constructs one
+//! `TransferDial` per transfer, clamped by the receiver's advertised
+//! [`CapacityProfile`] (the `ue-r2-1b` wire fields), starts at the
+//! conservative floor (D-2026-06-20-1/-2: no probe phase, no
+//! size-gated start — begin immediately and tune live), and a tuner
+//! steps the cheap dials from the PR1 stream telemetry.
+//!
+//! Mutability model (the C-ready seam `ue-r2-2` builds on):
+//! - **Cheap dials** — `chunk_bytes`, `prefetch_count`: atomics the
+//!   tuner steps mid-transfer. Consumers read them when a session,
+//!   pipeline, or fallback batch is set up, so a step takes effect for
+//!   sockets/batches started afterwards (epoch-N resize adds, the next
+//!   gRPC-fallback batch) — existing sessions keep their snapshot.
+//! - **Connect-time dials** — `tcp_buffer_bytes`, buffer-pool sizing:
+//!   read when a socket/pool is built; changes affect sockets opened
+//!   afterwards (no setsockopt on live sockets this slice).
+//! - **Negotiated once** — `initial_streams`/`max_streams`: stream
+//!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
+//!   dial only carries the negotiation-time value and the
+//!   profile-clamped ceiling.
+//!
+//! This replaces the size-keyed `determine_remote_tuning` static
+//! ladder: the ladder's floor tier is the dial's start, its top tier
+//! is the dial's default ceiling, and everything between is reached by
+//! ramping on evidence instead of guessing from `total_bytes`.
+
+use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
+use std::sync::Arc;
+
+use crate::generated::CapacityProfile;
+
+const MIB: usize = 1024 * 1024;
+
+/// Floor (conservative start) values — the old ladder's smallest tier.
+pub const DIAL_FLOOR_CHUNK_BYTES: usize = 16 * MIB;
+pub const DIAL_FLOOR_PREFETCH: usize = 4;
+pub const DIAL_FLOOR_INITIAL_STREAMS: usize = 4;
+pub const DIAL_FLOOR_MAX_STREAMS: usize = 8;
+
+/// Default ceilings — the old ladder's top tier (a fully ramped dial
+/// matches today's best static behavior).
+pub const DIAL_CEILING_CHUNK_BYTES: usize = 64 * MIB;
+pub const DIAL_CEILING_PREFETCH: usize = 32;
+pub const DIAL_CEILING_MAX_STREAMS: usize = 32;
+pub const DIAL_CEILING_TCP_BUFFER_BYTES: usize = 8 * MIB;
+
+/// Tuner policy (initial, deliberately simple): sampled every
+/// [`DIAL_TUNER_TICK`]; below [`DIAL_STEP_UP_BLOCKED_RATIO`] blocked
+/// time the pipe is not back-pressured → step up; above
+/// [`DIAL_STEP_DOWN_BLOCKED_RATIO`] → step down. One step per tick
+/// (hysteresis by construction).
+pub const DIAL_TUNER_TICK: std::time::Duration = std::time::Duration::from_millis(500);
+pub const DIAL_STEP_UP_BLOCKED_RATIO: f64 = 0.05;
+pub const DIAL_STEP_DOWN_BLOCKED_RATIO: f64 = 0.30;
+
+/// Resize policy (`ue-r2-2`): streams are the EXPENSIVE dial — a step
+/// costs a control round-trip plus a TCP connect — so they move only
+/// after the cheap dials are pinned at a bound and the signal has held
+/// for [`RESIZE_SUSTAIN_TICKS`] consecutive ticks, and never within
+/// [`RESIZE_COOLDOWN_TICKS`] of the previous settle. One stream per
+/// epoch (the wire carries one `sub_token` per ADD).
+pub const RESIZE_COOLDOWN_TICKS: u32 = 4;
+pub const RESIZE_SUSTAIN_TICKS: i32 = 2;
+
+/// The capacity profile this host advertises when it is the byte
+/// RECEIVER (ue-r2-1e: the first real sender of the ue-r2-1b wire
+/// fields). Honest system facts only — fields we cannot measure yet
+/// stay 0 (= unknown per the wire contract), never fabricated:
+/// ceilings mirror what today's receive paths actually accept.
+pub fn local_receiver_capacity() -> CapacityProfile {
+    CapacityProfile {
+        cpu_cores: num_cpus::get() as u32,
+        drain_class: 0,
+        load_percent: 0,
+        max_streams: DIAL_CEILING_MAX_STREAMS as u32,
+        drain_rate_bytes_per_sec: 0,
+        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
+        max_inflight_bytes: (DIAL_CEILING_CHUNK_BYTES * DIAL_CEILING_PREFETCH) as u64,
+    }
+}
+
+/// The one mutable tuning object for a transfer.
+#[derive(Debug)]
+pub struct TransferDial {
+    chunk_bytes: AtomicUsize,
+    prefetch_count: AtomicUsize,
+    /// 0 = unset (kernel default), matching the old `Option<usize>`.
+    tcp_buffer_bytes: AtomicUsize,
+    initial_streams: AtomicUsize,
+    max_streams: AtomicUsize,
+    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
+    // resize ids; 0 is reserved for the initial stream set) ──────────
+    /// Settled live stream count. Epoch-0 write is
+    /// `set_negotiated_streams`; later writes come from
+    /// `resize_settled` on an accepted epoch.
+    live_streams: AtomicUsize,
+    /// Last settled epoch (0 until the first accepted resize).
+    resize_epoch: AtomicU32,
+    /// In-flight proposal's epoch; 0 = none. While non-zero no new
+    /// proposal is produced (the wire is idempotent but overlapping
+    /// epochs would complicate sub-token registration).
+    pending_epoch: AtomicU32,
+    /// Resize-eligible ticks since the last settle (cooldown clock).
+    ticks_since_settle: AtomicU32,
+    /// Consecutive same-direction tick counter: positive = "pipe clean
+    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
+    /// dials floored" streak. Any other tick resets it.
+    resize_sustain: AtomicI32,
+    // Profile-clamped bounds, fixed at construction.
+    ceiling_chunk_bytes: usize,
+    ceiling_prefetch: usize,
+    ceiling_max_streams: usize,
+    ceiling_tcp_buffer_bytes: usize,
+}
+
+/// One engine resize decision (`ue-r2-2`). The adapter that owns the
+/// control stream turns this into a wire `DataPlaneResize` (the engine
+/// stays wire-type-free here on purpose) and MUST eventually call
+/// [`TransferDial::resize_settled`] for the epoch — with what actually
+/// happened — or no further proposals are produced.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct ResizeProposal {
+    /// The wire epoch for this change (`resize_epoch() + 1`).
+    pub epoch: u32,
+    /// Absolute desired live count (idempotent, per the proto).
+    pub target_streams: usize,
+    /// Convenience: `target_streams > live` at proposal time.
+    pub add: bool,
+}
+
+impl TransferDial {
+    /// Conservative start with default ceilings (no receiver profile).
+    pub fn conservative() -> Self {
+        Self::conservative_within(None)
+    }
+
+    /// Conservative start bounded by the receiver's advertised
+    /// capacity profile. Per the `ue-r2-1b` contract, `0`/absent
+    /// fields mean UNKNOWN and keep the (already conservative)
+    /// default ceiling — never "unlimited". A profile can only lower
+    /// ceilings, never raise them above the defaults this slice.
+    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
+        let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
+        let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
+        let mut ceiling_streams = DIAL_CEILING_MAX_STREAMS;
+        let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
+        if let Some(profile) = profile {
+            if profile.max_chunk_bytes > 0 {
+                ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
+            }
+            if profile.max_streams > 0 {
+                ceiling_streams = ceiling_streams.min(profile.max_streams as usize);
+            }
+            if profile.max_inflight_bytes > 0 {
+                // The in-flight budget bounds the CHUNK ceiling first
+                // (codex ue-r2-1e F1: with max_chunk unknown, a budget
+                // smaller than one chunk must still be honored — floor
+                // 64 KiB, matching the session's minimum buffer), then
+                // prefetch so prefetch × chunk stays within budget
+                // (floor of 1 so work still moves).
+                let inflight = profile.max_inflight_bytes as usize;
+                ceiling_chunk =
+                    ceiling_chunk.min(inflight.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR));
+                let by_inflight = (inflight / ceiling_chunk.max(1)).max(1);
+                ceiling_prefetch = ceiling_prefetch.min(by_inflight);
+            }
+        }
+        Self {
+            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
+            prefetch_count: AtomicUsize::new(DIAL_FLOOR_PREFETCH.min(ceiling_prefetch)),
+            tcp_buffer_bytes: AtomicUsize::new(0),
+            initial_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
+            max_streams: AtomicUsize::new(DIAL_FLOOR_MAX_STREAMS.clamp(1, ceiling_streams.max(1))),
+            live_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
+            resize_epoch: AtomicU32::new(0),
+            pending_epoch: AtomicU32::new(0),
+            ticks_since_settle: AtomicU32::new(0),
+            resize_sustain: AtomicI32::new(0),
+            ceiling_chunk_bytes: ceiling_chunk,
+            ceiling_prefetch,
+            ceiling_max_streams: ceiling_streams,
+            ceiling_tcp_buffer_bytes: ceiling_tcp,
+        }
+    }
+
+    pub fn shared(self) -> Arc<Self> {
+        Arc::new(self)
+    }
+
+    // ── live reads ───────────────────────────────────────────────────
+    pub fn chunk_bytes(&self) -> usize {
+        self.chunk_bytes.load(Ordering::Relaxed)
+    }
+    pub fn prefetch_count(&self) -> usize {
+        self.prefetch_count.load(Ordering::Relaxed)
+    }
+    /// `None` = leave the kernel default (old `tcp_buffer_size`
+    /// semantics). Connect-time dial.
+    pub fn tcp_buffer_bytes(&self) -> Option<usize> {
+        match self.tcp_buffer_bytes.load(Ordering::Relaxed) {
+            0 => None,
+            n => Some(n),
+        }
+    }
+    pub fn initial_streams(&self) -> usize {
+        self.initial_streams.load(Ordering::Relaxed)
+    }
+    /// Ceiling on the negotiated stream count (profile-clamped).
+    pub fn max_streams(&self) -> usize {
+        self.max_streams.load(Ordering::Relaxed)
+    }
+    pub fn ceiling_max_streams(&self) -> usize {
+        self.ceiling_max_streams
+    }
+
+    /// Record the stream count the negotiation actually settled on
+    /// (clamped to the dial's ceiling). This is the epoch-0 settle:
+    /// it also seeds `live_streams`, the baseline every `ue-r2-2`
+    /// resize proposal steps from.
+    pub fn set_negotiated_streams(&self, streams: usize) -> usize {
+        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
+        self.initial_streams.store(clamped, Ordering::Relaxed);
+        self.live_streams.store(clamped, Ordering::Relaxed);
+        clamped
+    }
+
+    // ── ue-r2-2 resize policy ────────────────────────────────────────
+
+    /// The settled live stream count (epoch-0 negotiation, then each
+    /// accepted resize).
+    pub fn live_streams(&self) -> usize {
+        self.live_streams.load(Ordering::Relaxed)
+    }
+
+    /// Last settled resize epoch (0 = only the initial stream set).
+    pub fn resize_epoch(&self) -> u32 {
+        self.resize_epoch.load(Ordering::Relaxed)
+    }
+
+    /// True while a proposal is awaiting `resize_settled`.
+    pub fn resize_pending(&self) -> bool {
+        self.pending_epoch.load(Ordering::Relaxed) != 0
+    }
+
+    fn cheap_dials_maxed(&self) -> bool {
+        self.chunk_bytes.load(Ordering::Relaxed) >= self.ceiling_chunk_bytes
+            && self.prefetch_count.load(Ordering::Relaxed) >= self.ceiling_prefetch
+    }
+
+    fn cheap_dials_floored(&self) -> bool {
+        self.chunk_bytes.load(Ordering::Relaxed)
+            <= DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes)
+            && self.prefetch_count.load(Ordering::Relaxed)
+                <= DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch).max(1)
+    }
+
+    /// One resize-eligible tuner tick. Streams move only as the LAST
+    /// escalation step in either direction: the cheap dials must
+    /// already be pinned at their ceiling (ADD) or floor (REMOVE), the
+    /// signal must hold for [`RESIZE_SUSTAIN_TICKS`] consecutive
+    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
+    /// since the last settle, and no proposal may be in flight. Idle
+    /// ticks (`delta_bytes == 0`) are no signal, matching the cheap
+    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
+    /// folded in at construction — `CapacityProfile.max_streams` is
+    /// authoritative per the proto). One stream per epoch.
+    ///
+    /// The caller must forward the returned proposal to the peer and
+    /// call [`Self::resize_settled`] with the outcome; until then
+    /// every subsequent tick returns `None`.
+    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
+        if self.pending_epoch.load(Ordering::Relaxed) != 0 {
+            return None;
+        }
+        let ticks = self
+            .ticks_since_settle
+            .fetch_add(1, Ordering::Relaxed)
+            .saturating_add(1);
+        if delta_bytes == 0 {
+            self.resize_sustain.store(0, Ordering::Relaxed);
+            return None;
+        }
+        let live = self.live_streams.load(Ordering::Relaxed).max(1);
+        let sustain = if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO && self.cheap_dials_maxed() {
+            let prev = self.resize_sustain.load(Ordering::Relaxed).max(0);
+            let next = prev.saturating_add(1);
+            self.resize_sustain.store(next, Ordering::Relaxed);
+            next
+        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO && self.cheap_dials_floored() {
+            let prev = self.resize_sustain.load(Ordering::Relaxed).min(0);
+            let next = prev.saturating_sub(1);
+            self.resize_sustain.store(next, Ordering::Relaxed);
+            next
+        } else {
+            self.resize_sustain.store(0, Ordering::Relaxed);
+            0
+        };
+        if ticks < RESIZE_COOLDOWN_TICKS {
+            return None;
+        }
+        let target = if sustain >= RESIZE_SUSTAIN_TICKS {
+            (live + 1).min(self.ceiling_max_streams.max(1))
+        } else if sustain <= -RESIZE_SUSTAIN_TICKS {
+            live.saturating_sub(1).max(1)
+        } else {
+            return None;
+        };
+        if target == live {
+            // Already at the bound in the wanted direction.
+            self.resize_sustain.store(0, Ordering::Relaxed);
+            return None;
+        }
+        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
+        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
+        // another task, and a plain store here could stack two live
+        // proposals onto one epoch number.
+        if self
+            .pending_epoch
+            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
+            .is_err()
+        {
+            return None;
+        }
+        self.resize_sustain.store(0, Ordering::Relaxed);
+        Some(ResizeProposal {
+            epoch,
+            target_streams: target,
+            add: target > live,
+        })
+    }
+
+    /// sf-2: shape-correction proposal. On push the daemon proposes the
+    /// epoch-0 stream count from whatever manifest prefix it has seen at
+    /// the early flush (`FILE_LIST_EARLY_FLUSH_ENTRIES`), so a
+    /// many-tiny-file push can negotiate far fewer streams than
+    /// [`initial_stream_proposal`] assigns the full workload. As the
+    /// need list accumulates client-side, the client re-runs the shape
+    /// table and corrects upward through the normal resize wire.
+    ///
+    /// Unlike [`Self::resize_tick`] this is a definite signal — the
+    /// shape is known, not inferred from throughput — so there is no
+    /// sustain/cooldown discipline. It still honors one-in-flight and
+    /// the receiver-profile ceiling, still moves ONE stream per epoch
+    /// (the wire carries one `sub_token` per ADD), and never proposes
+    /// REMOVE: shrinking below a live count is throughput evidence and
+    /// stays the tuner's call.
+    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
+        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
+        let live = self.live_streams.load(Ordering::Relaxed).max(1);
+        if desired <= live {
+            return None;
+        }
+        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
+        if self
+            .pending_epoch
+            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
+            .is_err()
+        {
+            return None;
+        }
+        Some(ResizeProposal {
+            epoch,
+            target_streams: live + 1,
+            add: true,
+        })
+    }
+
+    /// Settle the in-flight proposal with what ACTUALLY happened:
+    /// `effective_streams` is the live count now in effect (from the
+    /// peer's ack, or the local count if a post-ack dial failed and
+    /// nothing changed). `accepted = false` leaves the live count
+    /// untouched. Stale epochs (not the pending one) are ignored.
+    /// Either way the cooldown clock restarts.
+    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
+        if self.pending_epoch.load(Ordering::Relaxed) != epoch || epoch == 0 {
+            return;
+        }
+        self.pending_epoch.store(0, Ordering::Relaxed);
+        self.ticks_since_settle.store(0, Ordering::Relaxed);
+        self.resize_sustain.store(0, Ordering::Relaxed);
+        if accepted {
+            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
+            self.live_streams.store(clamped, Ordering::Relaxed);
+            self.resize_epoch.store(epoch, Ordering::Relaxed);
+        }
+    }
+
+    /// Raise max_streams toward the ceiling (used when a peer's
+    /// negotiation allows more than the floor; still profile-bounded).
+    pub fn allow_streams_up_to(&self, streams: usize) {
+        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
+        self.max_streams.store(clamped, Ordering::Relaxed);
+    }
+
+    // ── tuner steps ──────────────────────────────────────────────────
+    /// One upward step of the cheap dials: chunk ×2 toward the
+    /// ceiling, prefetch +50% (at least +1) toward the ceiling, and
+    /// the tcp buffer to its ceiling (affects future sockets).
+    /// Returns true if anything moved.
+    pub fn step_up_cheap_dials(&self) -> bool {
+        let mut moved = false;
+        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
+        let next = (chunk.saturating_mul(2)).min(self.ceiling_chunk_bytes);
+        if next > chunk {
+            self.chunk_bytes.store(next, Ordering::Relaxed);
+            moved = true;
+        }
+        let prefetch = self.prefetch_count.load(Ordering::Relaxed);
+        let next = (prefetch + (prefetch / 2).max(1)).min(self.ceiling_prefetch);
+        if next > prefetch {
+            self.prefetch_count.store(next, Ordering::Relaxed);
+            moved = true;
+        }
+        let tcp = self.tcp_buffer_bytes.load(Ordering::Relaxed);
+        if tcp < self.ceiling_tcp_buffer_bytes {
+            self.tcp_buffer_bytes
+                .store(self.ceiling_tcp_buffer_bytes, Ordering::Relaxed);
+            moved = true;
+        }
+        moved
+    }
+
+    /// One downward step toward the floors. Returns true if anything
+    /// moved.
+    pub fn step_down_cheap_dials(&self) -> bool {
+        let mut moved = false;
+        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
+        let next = (chunk / 2).max(DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes));
+        if next < chunk {
+            self.chunk_bytes.store(next, Ordering::Relaxed);
+            moved = true;
+        }
+        let prefetch = self.prefetch_count.load(Ordering::Relaxed);
+        let next = (prefetch / 2)
+            .max(DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch))
+            .max(1);
+        if next < prefetch {
+            self.prefetch_count.store(next, Ordering::Relaxed);
+            moved = true;
+        }
+        moved
+    }
+
+    /// One tuner tick: adjust from the observed blocked-time ratio
+    /// (write-blocked nanos across streams ÷ wall nanos × streams for
+    /// the tick window). Between the thresholds nothing moves
+    /// (hysteresis band).
+    pub fn apply_tick(&self, blocked_ratio: f64) -> bool {
+        if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO {
+            self.step_up_cheap_dials()
+        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO {
+            self.step_down_cheap_dials()
+        } else {
+            false
+        }
+    }
+}
+
+/// Workload-shape-aware initial stream proposal (`ue-r2-1f`): the
+/// end that KNOWS the workload shape proposes a starting stream
+/// count — file count matters as much as bytes (many small files
+/// parallelize on per-file overhead even at low byte totals). On push
+/// that is the receiving daemon (it has the manifest) clamped to its
+/// own advertised ceiling; on pull_sync it is the sending daemon (it
+/// enumerated the source) clamped to the CLIENT's advertised
+/// `receiver_capacity.max_streams` (`ue-r2-1g`) — either way the byte
+/// receiver's profile is the bound. Table carried over verbatim from
+/// the daemon push `desired_streams` ladder it retires (the ladder
+/// the old `tuning.rs` doc said "wins"), now engine-owned. The
+/// sender's dial clamps again on its side (`set_negotiated_streams`).
+/// Live mid-transfer stream changes arrive with `ue-r2-2` resize.
+pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
+    if file_count == 0 {
+        return 1;
+    }
+    let proposal: u32 = if total_bytes >= 32 * 1024 * 1024 * 1024 || file_count >= 200_000 {
+        16
+    } else if total_bytes >= 8 * 1024 * 1024 * 1024 || file_count >= 80_000 {
+        12
+    } else if total_bytes >= 2 * 1024 * 1024 * 1024 || file_count >= 50_000 {
+        10
+    } else if total_bytes >= 512 * 1024 * 1024 || file_count >= 10_000 {
+        8
+    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
+        4
+    } else if total_bytes >= 32 * 1024 * 1024 || file_count >= 256 {
+        2
+    } else {
+        1
+    };
+    proposal.min(ceiling.max(1) as u32)
+}
+
+/// Blocked-time ratio for one tuner tick: the share of the tick's
+/// wall-clock (× stream count) the senders spent inside socket writes.
+/// 0 streams or a zero-length tick reads as "no signal" (0.0 — the
+/// hysteresis band holds the dial still rather than guessing).
+pub(crate) fn blocked_ratio(
+    delta_blocked_nanos: u64,
+    elapsed: std::time::Duration,
+    streams: usize,
+) -> f64 {
+    let denom = elapsed.as_nanos().saturating_mul(streams as u128);
+    if denom == 0 {
+        return 0.0;
+    }
+    (delta_blocked_nanos as f64 / denom as f64).clamp(0.0, 1.0)
+}
+
+/// Growable per-transfer probe registry (`ue-r2-2`): resize adds a
+/// probe when a stream joins and removes it when one retires, and the
+/// tuner samples whatever is live each tick. Plain std mutex — locked
+/// only for a snapshot fold every 500ms and on resize events.
+pub type SharedStreamProbes =
+    Arc<std::sync::Mutex<Vec<crate::remote::transfer::progress::StreamProbe>>>;
+
+/// Spawn the live tuner for one transfer (ue-r2-1e): every
+/// [`DIAL_TUNER_TICK`] it sums the PR1 per-stream `write_blocked`
+/// telemetry and steps the dial's cheap dials. Holds only a `Weak` to
+/// the dial, so it self-terminates within one tick of the transfer
+/// dropping its dial; callers may also abort the handle for prompt
+/// shutdown (`MultiStreamSender::finish` does).
+pub fn spawn_dial_tuner(
+    dial: &Arc<TransferDial>,
+    probes: Vec<crate::remote::transfer::progress::StreamProbe>,
+) -> tokio::task::JoinHandle<()> {
+    spawn_dial_tuner_with_resize(dial, Arc::new(std::sync::Mutex::new(probes)), None)
+}
+
+/// `ue-r2-2` tuner: same cheap-dial stepping, but over a growable
+/// probe registry, plus the stream-resize policy when `resize_tx` is
+/// provided — each [`TransferDial::resize_tick`] proposal is forwarded
+/// to the adapter that owns the control stream (unbounded so a
+/// momentarily busy adapter cannot lose a proposal while the dial
+/// holds it pending). Callers without resize pass `None` and get
+/// exactly the ue-r2-1e behavior.
+pub fn spawn_dial_tuner_with_resize(
+    dial: &Arc<TransferDial>,
+    probes: SharedStreamProbes,
+    resize_tx: Option<tokio::sync::mpsc::UnboundedSender<ResizeProposal>>,
+) -> tokio::task::JoinHandle<()> {
+    let weak = Arc::downgrade(dial);
+    tokio::spawn(async move {
+        let mut last_blocked: u64 = 0;
+        let mut last_bytes: u64 = 0;
+        let mut last_tick = tokio::time::Instant::now();
+        loop {
+            tokio::time::sleep(DIAL_TUNER_TICK).await;
+            let Some(dial) = weak.upgrade() else { return };
+            let (blocked, bytes, streams) = {
+                let probes = probes.lock().expect("probe registry poisoned");
+                let (b, n) = probes.iter().fold((0u64, 0u64), |(b, n), p| {
+                    let snap = p.snapshot();
+                    (b + snap.write_blocked_nanos, n + snap.bytes_sent)
+                });
+                (b, n, probes.len())
+            };
+            let elapsed = last_tick.elapsed();
+            last_tick = tokio::time::Instant::now();
+            // A retired stream leaves the registry, so the monotonic
+            // sums can shrink across a REMOVE. Re-baseline and treat
+            // the tick as no-signal rather than reading a bogus delta.
+            if blocked < last_blocked || bytes < last_bytes {
+                last_blocked = blocked;
+                last_bytes = bytes;
+                if let Some(tx) = &resize_tx {
+                    let _ = tx; // no proposal possible on a no-signal tick
+                    dial.resize_tick(0, 0.0);
+                }
+                continue;
+            }
+            let delta_blocked = blocked.saturating_sub(last_blocked);
+            let delta_bytes = bytes.saturating_sub(last_bytes);
+            last_blocked = blocked;
+            last_bytes = bytes;
+            // codex ue-r2-1e F2: an idle tick (no bytes moved) is NO
+            // SIGNAL, not a clean pipe — stepping up during manifest /
+            // preparation stalls would ramp without evidence and break
+            // the conservative-start contract. ue-r2-2 review (panel
+            // F3): the idle tick must still reach `resize_tick` so a
+            // sustain streak cannot survive a stall — "consecutive
+            // busy ticks" means consecutive.
+            if delta_bytes == 0 {
+                if resize_tx.is_some() {
+                    dial.resize_tick(0, 0.0);
+                }
+                continue;
+            }
+            let ratio = blocked_ratio(delta_blocked, elapsed, streams);
+            dial.apply_tick(ratio);
+            if let Some(tx) = &resize_tx {
+                if let Some(proposal) = dial.resize_tick(delta_bytes, ratio) {
+                    if tx.send(proposal).is_err() {
+                        // Controller gone (transfer tearing down):
+                        // release the pending slot so the dial state
+                        // stays honest for late readers.
+                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
+                    }
+                }
+            }
+        }
+    })
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn profile(max_streams: u32, max_chunk: u64, max_inflight: u64) -> CapacityProfile {
+        CapacityProfile {
+            cpu_cores: 0,
+            drain_class: 0,
+            load_percent: 0,
+            max_streams,
+            drain_rate_bytes_per_sec: 0,
+            max_chunk_bytes: max_chunk,
+            max_inflight_bytes: max_inflight,
+        }
+    }
+
+    #[test]
+    fn conservative_start_is_the_old_floor_tier() {
+        let dial = TransferDial::conservative();
+        assert_eq!(dial.chunk_bytes(), 16 * MIB);
+        assert_eq!(dial.prefetch_count(), 4);
+        assert_eq!(dial.tcp_buffer_bytes(), None);
+        assert_eq!(dial.initial_streams(), 4);
+        assert_eq!(dial.max_streams(), 8);
+    }
+
+    #[test]
+    fn unknown_profile_fields_keep_default_ceilings() {
+        let dial = TransferDial::conservative_within(Some(&profile(0, 0, 0)));
+        // Ramp fully: unknown (0) fields must not lower — or lift —
+        // anything relative to the defaults.
+        while dial.step_up_cheap_dials() {}
+        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
+        assert_eq!(dial.prefetch_count(), DIAL_CEILING_PREFETCH);
+        assert_eq!(dial.tcp_buffer_bytes(), Some(DIAL_CEILING_TCP_BUFFER_BYTES));
+        assert_eq!(dial.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
+    }
+
+    #[test]
+    fn profile_lowers_ceilings_but_never_raises_them() {
+        let dial =
+            TransferDial::conservative_within(Some(&profile(4, 32 * MIB as u64, 64 * MIB as u64)));
+        while dial.step_up_cheap_dials() {}
+        assert_eq!(dial.chunk_bytes(), 32 * MIB, "chunk ceiling from profile");
+        // 64 MiB in-flight ÷ 32 MiB chunk ceiling = 2 payload budget.
+        assert_eq!(dial.prefetch_count(), 2, "prefetch bounded by max_inflight");
+        assert_eq!(dial.ceiling_max_streams(), 4);
+
+        // codex F1: an in-flight budget smaller than one chunk bounds
+        // the chunk ceiling itself, even with max_chunk unknown (0).
+        let tight = TransferDial::conservative_within(Some(&profile(0, 0, 8 * MIB as u64)));
+        while tight.step_up_cheap_dials() {}
+        assert_eq!(tight.chunk_bytes(), 8 * MIB);
+        assert_eq!(tight.prefetch_count(), 1);
+
+        let generous = TransferDial::conservative_within(Some(&profile(999, u64::MAX, u64::MAX)));
+        while generous.step_up_cheap_dials() {}
+        assert_eq!(generous.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
+        assert_eq!(generous.prefetch_count(), DIAL_CEILING_PREFETCH);
+        assert_eq!(generous.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
+    }
+
+    #[test]
+    fn steps_respect_floor_and_ceiling_with_hysteresis_band() {
+        let dial = TransferDial::conservative();
+        assert!(!dial.step_down_cheap_dials(), "already at the floor");
+        assert!(dial.apply_tick(0.0), "clean telemetry steps up");
+        assert_eq!(dial.chunk_bytes(), 32 * MIB);
+        assert!(
+            !dial.apply_tick(0.15),
+            "inside the hysteresis band nothing moves"
+        );
+        assert!(dial.apply_tick(0.9), "blocked telemetry steps down");
+        assert_eq!(dial.chunk_bytes(), 16 * MIB);
+        while dial.apply_tick(0.0) {}
+        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
+        assert_eq!(dial.prefetch_count(), DIAL_CEILING_PREFETCH);
+    }
+
+    #[test]
+    fn initial_stream_proposal_matches_the_retired_daemon_table() {
+        const MIB64: u64 = 1024 * 1024;
+        const GIB: u64 = 1024 * MIB64;
+        // Empty need-list → 1 (the old ladder's empty-guard).
+        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
+        // Byte-keyed tiers: exact lower boundaries AND just-below each
+        // (codex ue-r2-1f: representative values would miss a doubled
+        // threshold).
+        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
+        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
+        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
+        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
+        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
+        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
+        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
+        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
+        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
+        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
+        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
+        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
+        // File-count keys fire independently of bytes.
+        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
+        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
+        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
+        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
+        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
+        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
+        // Ceiling clamps the proposal (receiver profile authority).
+        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
+        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
+    }
+
+    #[test]
+    fn blocked_ratio_handles_edges() {
+        use std::time::Duration;
+        assert_eq!(blocked_ratio(0, Duration::from_millis(500), 4), 0.0);
+        assert_eq!(blocked_ratio(1_000, Duration::ZERO, 4), 0.0, "no signal");
+        assert_eq!(blocked_ratio(1_000, Duration::from_millis(500), 0), 0.0);
+        let half = blocked_ratio(500_000_000, Duration::from_millis(500), 2);
+        assert!((half - 0.5).abs() < 1e-9, "got {half}");
+        assert_eq!(
+            blocked_ratio(u64::MAX, Duration::from_nanos(1), 1),
+            1.0,
+            "clamped"
+        );
+    }
+
+    #[tokio::test(start_paused = true)]
+    async fn tuner_steps_up_on_clean_telemetry_and_exits_when_dial_drops() {
+        use crate::remote::transfer::progress::{StreamId, StreamProbe};
+        let dial = TransferDial::conservative().shared();
+        let probes = [StreamProbe::new(StreamId(0)), StreamProbe::new(StreamId(1))];
+        let tuner_view: Vec<StreamProbe> = probes
+            .iter()
+            .map(|p| StreamProbe::from_telemetry(p.id(), p.telemetry()))
+            .collect();
+        let handle = spawn_dial_tuner(&dial, tuner_view);
+        // Let the spawned task run to its first sleep so the timer is
+        // registered before the clock moves.
+        tokio::task::yield_now().await;
+
+        // codex F2: an idle tick (no bytes moved) must NOT step.
+        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
+        for _ in 0..16 {
+            tokio::task::yield_now().await;
+        }
+        assert_eq!(dial.chunk_bytes(), 16 * MIB, "idle tick is no signal");
+
+        // One tick WITH byte progress and zero blocked time: step up.
+        probes[0].record_bytes(1024);
+        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
+        for _ in 0..16 {
+            if dial.chunk_bytes() > 16 * MIB {
+                break;
+            }
+            tokio::task::yield_now().await;
+        }
+        assert_eq!(dial.chunk_bytes(), 32 * MIB, "stepped up once");
+
+        // Drop the transfer's dial: the tuner must self-terminate.
+        drop(dial);
+        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
+        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
+            .await
+            .expect("tuner exits after the dial drops")
+            .expect("tuner does not panic");
+    }
+
+    #[test]
+    fn negotiated_streams_clamp_to_the_profile_ceiling() {
+        let dial = TransferDial::conservative_within(Some(&profile(6, 0, 0)));
+        dial.allow_streams_up_to(32);
+        assert_eq!(dial.max_streams(), 6, "peer cannot exceed the profile");
+        assert_eq!(dial.set_negotiated_streams(16), 6);
+        assert_eq!(dial.set_negotiated_streams(3), 3);
+        assert_eq!(dial.live_streams(), 3, "negotiation seeds the live count");
+    }
+
+    // ── ue-r2-2 resize policy ────────────────────────────────────────
+
+    /// Burn the cooldown with busy, in-band ticks that move no dials.
+    fn burn_cooldown(dial: &TransferDial) {
+        for _ in 0..RESIZE_COOLDOWN_TICKS {
+            assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band tick holds");
+        }
+    }
+
+    #[test]
+    fn resize_add_requires_maxed_cheap_dials_sustain_and_cooldown() {
+        let dial = TransferDial::conservative();
+        dial.set_negotiated_streams(4);
+
+        // Clean pipe but cheap dials NOT at ceiling: never proposes,
+        // no matter how long it holds (cheap dials escalate first).
+        for _ in 0..16 {
+            assert_eq!(dial.resize_tick(1024, 0.0), None);
+        }
+
+        // Pin the cheap dials at the ceiling, then a clean signal must
+        // still survive the sustain requirement before proposing.
+        while dial.step_up_cheap_dials() {}
+        assert_eq!(dial.resize_tick(1024, 0.0), None, "sustain tick 1");
+        let proposal = dial
+            .resize_tick(1024, 0.0)
+            .expect("sustained clean signal at maxed dials proposes");
+        assert_eq!(
+            proposal,
+            ResizeProposal {
+                epoch: 1,
+                target_streams: 5,
+                add: true
+            }
+        );
+        assert!(dial.resize_pending());
+
+        // In flight: no further proposals regardless of signal.
+        for _ in 0..8 {
+            assert_eq!(dial.resize_tick(1024, 0.0), None, "pending blocks");
+        }
+
+        // Accepted settle: live moves, epoch advances, cooldown blocks
+        // the immediate next proposal even under a perfect signal.
+        dial.resize_settled(1, 5, true);
+        assert_eq!(dial.live_streams(), 5);
+        assert_eq!(dial.resize_epoch(), 1);
+        assert!(!dial.resize_pending());
+        for _ in 0..(RESIZE_COOLDOWN_TICKS - 1) {
+            assert_eq!(dial.resize_tick(1024, 0.0), None, "cooldown holds");
+        }
+        // Cooldown expired and the clean streak has been building the
+        // whole time — the next clean tick proposes epoch 2.
+        let next = dial.resize_tick(1024, 0.0).expect("epoch 2 proposes");
+        assert_eq!(next.epoch, 2);
+        assert_eq!(next.target_streams, 6);
+    }
+
+    #[test]
+    fn resize_remove_requires_floored_cheap_dials_and_floors_at_one() {
+        let dial = TransferDial::conservative();
+        dial.set_negotiated_streams(2);
+        burn_cooldown(&dial);
+
+        // Blocked pipe with cheap dials at the floor (conservative
+        // start IS the floor): two sustained ticks propose a drop.
+        assert_eq!(dial.resize_tick(1024, 0.9), None, "sustain tick 1");
+        let proposal = dial.resize_tick(1024, 0.9).expect("sustained block drops");
+        assert_eq!(
+            proposal,
+            ResizeProposal {
+                epoch: 1,
+                target_streams: 1,
+                add: false
+            }
+        );
+        dial.resize_settled(1, 1, true);
+        assert_eq!(dial.live_streams(), 1);
+
+        // At one stream, a blocked pipe can never drop to zero.
+        burn_cooldown(&dial);
+        for _ in 0..8 {
+            assert_eq!(dial.resize_tick(1024, 0.9), None, "floor at 1");
+        }
+    }
+
+    #[test]
+    fn resize_signal_interruptions_and_idle_reset_sustain() {
+        let dial = TransferDial::conservative();
+        dial.set_negotiated_streams(4);
+        while dial.step_up_cheap_dials() {}
+        burn_cooldown(&dial);
+
+        // clean → idle → clean: the idle tick resets the streak, so
+        // the second clean tick is streak 1, not 2.
+        assert_eq!(dial.resize_tick(1024, 0.0), None);
+        assert_eq!(dial.resize_tick(0, 0.0), None, "idle resets");
+        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
+        // clean → in-band → clean: same reset.
+        assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band resets");
+        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
+        assert!(dial.resize_tick(1024, 0.0).is_some(), "streak completes");
+    }
+
+    #[test]
+    fn resize_refusal_keeps_live_count_and_stale_settles_are_ignored() {
+        let dial = TransferDial::conservative();
+        dial.set_negotiated_streams(4);
+        while dial.step_up_cheap_dials() {}
+        burn_cooldown(&dial);
+        assert_eq!(dial.resize_tick(1024, 0.0), None);
+        let proposal = dial.resize_tick(1024, 0.0).expect("proposes");
+
+        // A stale/foreign epoch must not clear the pending slot.
+        dial.resize_settled(proposal.epoch + 7, 9, true);
+        assert!(dial.resize_pending(), "stale settle ignored");
+
+        // Refusal: pending clears, live count and epoch stay put.
+        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
+        assert!(!dial.resize_pending());
+        assert_eq!(dial.live_streams(), 4);
+        assert_eq!(dial.resize_epoch(), 0, "refused epoch never settles");
+    }
+
+    #[test]
+    fn resize_target_clamps_to_the_profile_ceiling() {
+        let dial = TransferDial::conservative_within(Some(&profile(4, 0, 0)));
+        dial.set_negotiated_streams(4); // already at the profile ceiling
+        while dial.step_up_cheap_dials() {}
+        burn_cooldown(&dial);
+        for _ in 0..8 {
+            assert_eq!(
+                dial.resize_tick(1024, 0.0),
+                None,
+                "cannot add past the receiver's advertised ceiling"
+            );
+        }
+    }
+
+    // ── sf-2 shape-correction resize ─────────────────────────────────
+
+    /// The plan's three measured 10 GbE cells mapped through the shape
+    /// table (`docs/plan/SMALL_FILE_CEILING.md`): the small and mixed
+    /// cells must NOT ride the byte tiers alone.
+    #[test]
+    fn shape_table_covers_the_small_file_ceiling_cells() {
+        const KIB: u64 = 1024;
+        const MIB64: u64 = 1024 * KIB;
+        const GIB: u64 = 1024 * MIB64;
+        // push/pull 10k × 4 KiB: 40 MiB is the 2-stream byte tier, but
+        // 10_000 files must key the 8-stream file-count tier.
+        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
+        // 1 × 1 GiB: byte-keyed, file count is irrelevant — unchanged.
+        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
+        // mixed 512 MiB + 5k × 2 KiB: the byte tier already reaches 8;
+        // the 5_001 files alone would say 4 — bytes win.
+        assert_eq!(
+            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
+            8
+        );
+        // sf-1 loopback probe evidence: 1_000 tiny files must propose 2
+        // (the measured transfer rode 1 — the input, not this table,
+        // was wrong).
+        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
+    }
+
+    #[test]
+    fn shape_resize_ramps_one_epoch_at_a_time_toward_the_target() {
+        let dial = TransferDial::conservative();
+        dial.set_negotiated_streams(1);
+
+        // At or below live: nothing to correct.
+        assert_eq!(dial.propose_shape_resize(0), None);
+        assert_eq!(dial.propose_shape_resize(1), None);
+
+        // Target 3 from live 1: epoch 1 proposes 2 (one per epoch),
+        // and the in-flight epoch blocks both proposers.
+        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
+        assert_eq!(
+            p1,
+            ResizeProposal {
+                epoch: 1,
+                target_streams: 2,
+                add: true
+            }
+        );
+        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
+        assert_eq!(dial.resize_tick(1024, 0.0), None, "tuner blocked too");
+
+        // Settle → next step; no cooldown for the definite shape signal.
+        dial.resize_settled(1, 2, true);
+        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
+        assert_eq!(p2.epoch, 2);
+        assert_eq!(p2.target_streams, 3);
+        dial.resize_settled(2, 3, true);
+        assert_eq!(dial.live_streams(), 3);
+        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
+
+        // A refused epoch leaves live untouched; the next call retries.
+        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
+        dial.resize_settled(p3.epoch, dial.live_streams(), false);
+        assert_eq!(dial.live_streams(), 3);
+        assert!(
+            dial.propose_shape_resize(4).is_some(),
+            "retry after refusal"
+        );
+    }
+
+    #[test]
+    fn shape_resize_clamps_to_the_profile_ceiling() {
+        let dial = TransferDial::conservative_within(Some(&profile(2, 0, 0)));
+        dial.set_negotiated_streams(1);
+        let p = dial
+            .propose_shape_resize(100)
+            .expect("clamped, not refused");
+        assert_eq!(p.target_streams, 2);
+        dial.resize_settled(p.epoch, 2, true);
+        assert_eq!(
+            dial.propose_shape_resize(100),
+            None,
+            "at the receiver's advertised ceiling"
+        );
+    }
+
+    #[tokio::test(start_paused = true)]
+    async fn tuner_forwards_resize_proposals_over_the_shared_registry() {
+        use crate::remote::transfer::progress::{StreamId, StreamProbe};
+        let dial = TransferDial::conservative().shared();
+        dial.set_negotiated_streams(2);
+        while dial.step_up_cheap_dials() {}
+        let probe = StreamProbe::new(StreamId(0));
+        let registry: SharedStreamProbes =
+            Arc::new(std::sync::Mutex::new(vec![StreamProbe::from_telemetry(
+                probe.id(),
+                probe.telemetry(),
+            )]));
+        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
+        let handle = spawn_dial_tuner_with_resize(&dial, Arc::clone(&registry), Some(tx));
+        tokio::task::yield_now().await;
+
+        // Enough busy ticks to pass cooldown + sustain: every tick
+        // records fresh bytes with zero blocked time.
+        let mut proposal = None;
+        for _ in 0..(RESIZE_COOLDOWN_TICKS + RESIZE_SUSTAIN_TICKS as u32 + 2) {
+            probe.record_bytes(1024);
+            tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
+            for _ in 0..16 {
+                tokio::task::yield_now().await;
+            }
+            if let Ok(p) = rx.try_recv() {
+                proposal = Some(p);
+                break;
+            }
+        }
+        let proposal = proposal.expect("tuner forwarded a resize proposal");
+        assert_eq!(proposal.target_streams, 3);
+        assert!(proposal.add);
+        assert!(dial.resize_pending());
+
+        drop(dial);
+        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
+        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
+            .await
+            .expect("tuner exits after the dial drops")
+            .expect("tuner does not panic");
+    }
+}
diff --git a/crates/blit-core/src/remote/transfer/diff_planner.rs b/crates/blit-core/src/remote/transfer/diff_planner.rs
index 1c11946..5911c16 100644
--- a/crates/blit-core/src/remote/transfer/diff_planner.rs
+++ b/crates/blit-core/src/remote/transfer/diff_planner.rs
@@ -1,32 +1,21 @@
-//! Unified diff + payload planning stage.
+//! Payload planning for the session's SOURCE send half.
 //!
-//! Sits between `TransferSource::scan` (which emits headers from the
-//! origin's filesystem) and `execute_sink_pipeline_streaming` (which
-//! dispatches payloads to one or more sinks). Decides:
-//!
-//!   1. Which source headers represent files that genuinely need to
-//!      transfer (against the target's destination state).
-//!   2. What payload shapes the surviving files become (whole-file
-//!      `File` payloads, batched `TarShard`, or — once step 4 lands —
-//!      block-level resume `FileBlock` + `FileBlockComplete` pairs).
-//!
-//! Step 3a of `docs/plan/PIPELINE_UNIFICATION.md`. Today this module
-//! consolidates the local-mirror path that lived in `orchestrator.rs`
-//! (`filter_headers_for_copy` + the call to `plan_transfer_payloads`).
-//! Push and pull will adopt the same module in 3b and step 4.
-//!
-//! `ComparisonMode` in `proto/blit.proto` is the canonical input shape.
-//! As of R2-F1 (`docs/reviews/followup_review_2026-05-02.md`) we honor
-//! every variant with concrete semantics — no silent fall-through to
-//! size+mtime. This means callers passing `SizeOnly`, `IgnoreTimes`,
-//! or `Force` get the behavior the wire enum
-//! advertises, not whatever the historical default happened to do.
+//! [`plan_push_payloads`] shapes already-diffed need headers into
+//! payloads (whole-file `File`, batched `TarShard`) via
+//! `plan_transfer_payloads`. The diff itself is destination-owned
+//! (`transfer_session::destination_needs` →
+//! `manifest::header_transfer_status`) on every carrier; this module's
+//! own local-mirror diff stage (`plan_local_mirror`/`filter_unchanged`)
+//! died at otp-11b with the engine. The per-mode comparison semantics
+//! (R2-F1: every `ComparisonMode` variant honored, no silent
+//! fall-through) live in `copy::file_needs_copy_with_mode` — the sink's
+//! defense layer — pinned in this module's tests.
 
 use std::path::Path;
 
 use eyre::{Context, Result};
 
-use crate::generated::{ComparisonMode, FileHeader};
+use crate::generated::FileHeader;
 use crate::remote::transfer::payload::{plan_transfer_payloads, TransferPayload};
 use crate::transfer_plan::PlanOptions;
 
@@ -51,104 +40,18 @@ pub fn plan_push_payloads(
     plan_transfer_payloads(headers, source_root, plan_options).context("planning push payloads")
 }
 
-/// Input bundle for the local-mirror diff stage. Origin and target
-/// are co-located (both on the same filesystem), so the comparison
-/// can stat the destination directly without a wire roundtrip.
-pub struct LocalDiffInputs<'a> {
-    /// Source-rooted absolute path. Headers' `relative_path` is
-    /// joined under this to find the source bytes.
-    pub src_root: &'a Path,
-    /// Destination-rooted absolute path. Headers' `relative_path` is
-    /// joined under this to compare against existing target state.
-    pub dst_root: &'a Path,
-    /// How to decide whether a target-existing file matches.
-    pub compare_mode: ComparisonMode,
-    /// When true, skip any file the destination already has,
-    /// regardless of `compare_mode`. Orthogonal axis; matches the
-    /// `ignore_existing` field on `TransferOperationSpec`.
-    pub ignore_existing: bool,
-    /// Knobs for the tar / large / raw planner (unchanged from the
-    /// pre-extraction call site).
-    pub plan_options: PlanOptions,
-    /// When false, every source header passes the comparison stage —
-    /// equivalent to `--ignore-times`/`--force` in user-facing terms.
-    /// Used by the orchestrator when its `skip_unchanged` flag is off.
-    pub skip_unchanged: bool,
-}
-
-/// Filter source headers down to those that need transferring against
-/// a local destination, then plan the surviving headers into payloads.
-///
-/// This is the single entry point the local-mirror path uses. Future
-/// origin paths (push client, pull daemon) will gain their own entry
-/// points on this module — same diff + planning algorithm, different
-/// "where the destination lives" assumption.
-pub fn plan_local_mirror(
-    source_headers: Vec<FileHeader>,
-    inputs: LocalDiffInputs<'_>,
-) -> Result<Vec<TransferPayload>> {
-    let headers_to_copy = if inputs.skip_unchanged {
-        filter_unchanged(
-            &source_headers,
-            inputs.src_root,
-            inputs.dst_root,
-            inputs.compare_mode,
-            inputs.ignore_existing,
-        )
-    } else {
-        source_headers
-    };
-
-    plan_transfer_payloads(headers_to_copy, inputs.src_root, inputs.plan_options)
-        .context("planning payloads after diff stage")
-}
-
-/// Drop headers whose destination file already matches the source
-/// under the chosen comparison mode. Keeps headers that need transfer.
-///
-/// `ignore_existing` is the orthogonal "skip if dst exists" axis from
-/// `TransferOperationSpec`: when true, present destination files are
-/// dropped before `compare_mode` is consulted at all.
-///
-/// This is the local-mirror flavor: it stats the destination directly.
-/// Remote-source variants (where the destination manifest arrives over
-/// the wire) live in their own helpers — TBD step 4.
-///
-/// Every `ComparisonMode` variant is implemented (R2-F1). `Unspecified`
-/// behaves as `SizeMtime` (the historical default) — callers should fold
-/// `Unspecified` away via `NormalizedTransferOperation::from_spec`
-/// before reaching this function, but we accept it defensively.
-pub fn filter_unchanged(
-    headers: &[FileHeader],
-    src_root: &Path,
-    dst_root: &Path,
-    compare_mode: ComparisonMode,
-    ignore_existing: bool,
-) -> Vec<FileHeader> {
-    headers
-        .iter()
-        .filter(|h| {
-            let src = src_root.join(&h.relative_path);
-            let dst = dst_root.join(&h.relative_path);
-            if ignore_existing && dst.exists() {
-                return false;
-            }
-            local_needs_copy(&src, &dst, compare_mode).unwrap_or(true)
-        })
-        .cloned()
-        .collect()
-}
-
-/// Per-mode comparison predicate. Delegates to the centralized helper
-/// in `copy::compare` so the diff planner, the single-file copy path,

exec
/bin/zsh -lc 'git diff --stat 0f922de HEAD -- crates/blit-core/src/remote/transfer/sink.rs crates/blit-core/src/remote/transfer/pipeline.rs crates/blit-core/src/remote/transfer/payload.rs crates/blit-core/src/transfer_plan.rs crates/blit-core/src/dial.rs; git diff --unified=20 0f922de HEAD -- crates/blit-core/src/remote/transfer/sink.rs | rg -n "''^@@|write_tar|progress|create_dir_all|fs::write|flush|parallel|FsTransferSink|byte_progress"; git diff --unified=20 0f922de HEAD -- crates/blit-core/src/remote/transfer/pipeline.rs | rg -n "''^@@|TarShard|progress|execute_receive|read_tar"; git diff --unified=20 0f922de HEAD -- crates/blit-core/src/remote/transfer/payload.rs crates/blit-core/src/transfer_plan.rs crates/blit-core/src/dial.rs | rg -n "''^@@|TarShard|small|shard|initial_stream|propose_shape|framing|record"' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-bgq6ET4f' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-gxWcuQBv' (errno=Operation not permitted)
 crates/blit-core/src/dial.rs                     | 1051 ++++++++++++++++++++++
 crates/blit-core/src/remote/transfer/payload.rs  |  135 +--
 crates/blit-core/src/remote/transfer/pipeline.rs |   23 +-
 crates/blit-core/src/remote/transfer/sink.rs     |  749 ++-------------
 crates/blit-core/src/transfer_plan.rs            |    2 +-
 5 files changed, 1130 insertions(+), 830 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-tg9mSknJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-X4RAfFdT' (errno=Operation not permitted)
5:@@ -32,43 +32,42 @@ pub struct SinkOutcome {
51:@@ -107,215 +106,175 @@ impl Default for FsSinkConfig {
59: pub struct FsTransferSink {
77:     /// Optional byte-level progress sink. When set,
80:     /// writes report cumulative byte progress against the
83:     /// [`FsTransferSink::with_byte_progress`] from
85:     byte_progress: Option<ByteProgressSink>,
88: impl FsTransferSink {
105:             byte_progress: None,
118:     /// Attach a byte-level progress sink. When set,
122:     /// tracks live progress; CLI-side callers omit it.
123:     pub fn with_byte_progress(mut self, sink: ByteProgressSink) -> Self {
124:         self.byte_progress = Some(sink);
142:                     "FsTransferSink at '{}' has no canonical root; \
162: impl TransferSink for FsTransferSink {
207:                 eyre::bail!("FsTransferSink does not consume composite ResumeFile payloads")
231:                     PreparedPayload::TarShard { headers, data } => write_tar_shard_payload(
258:         // `write_tar_shard_payload`'s dry-run early returns), so
261:         if let Some(bp) = &self.byte_progress {
267:@@ -420,125 +379,156 @@ impl TransferSink for FsTransferSink {
319:     // R47-F1: the FsTransferSink::write_payload arm for
381:         std::fs::create_dir_all(parent)
424: fn write_tar_shard_payload(
431:@@ -995,397 +985,73 @@ impl TransferSink for NullSink {
497:-        // receive side can't measure progress against. See
819:+        std::fs::write(&src, b"root payload").unwrap();
832:         std::fs::create_dir_all(&src).unwrap();
833:         std::fs::create_dir_all(&dst).unwrap();
836:         std::fs::write(src.join("file.txt"), content).unwrap();
838:         let sink = FsTransferSink::new(
847:@@ -1656,339 +1322,40 @@ mod tests {
875:-        std::fs::create_dir_all(&src).unwrap();
876:-        std::fs::write(src.join("hello.txt"), b"world").unwrap();
922:-        std::fs::create_dir_all(&src).unwrap();
926:-        std::fs::write(src.join("big.bin"), vec![0xAAu8; file_size]).unwrap();
988:-        std::fs::create_dir_all(&src).unwrap();
990:-        std::fs::write(src.join("big.bin"), vec![0xBBu8; file_size]).unwrap();
1041:-        std::fs::create_dir_all(&src).unwrap();
1103:-        std::fs::create_dir_all(&src).unwrap();
1170:     // validator's surface. These tests exercise the FsTransferSink end of
1179:         std::fs::create_dir_all(&src).unwrap();
1180:         std::fs::create_dir_all(&dst).unwrap();
1182:         let sink = FsTransferSink::new(
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-YxA1ksYy' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-O50kyTdC' (errno=Operation not permitted)
5:@@ -187,61 +187,76 @@ pub async fn execute_sink_pipeline_elastic(
22:                         PreparedPayload::TarShard { headers, .. } => headers
29:-                        // plane, which reports no per-file progress here.
50:                     if let Some(p) = &progress {
51:                         // Contract (progress.rs): bytes ride Payload, one
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-oLlbbkgJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-oqbyR2ZL' (errno=Operation not permitted)
6:@@ -0,0 +1,1051 @@
25:+//! - **Negotiated once** — `initial_streams`/`max_streams`: stream
42:+/// Floor (conservative start) values — the old ladder's smallest tier.
97:+    initial_streams: AtomicUsize,
165:+                // smaller than one chunk must still be honored — floor
180:+            initial_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
213:+    pub fn initial_streams(&self) -> usize {
214:+        self.initial_streams.load(Ordering::Relaxed)
230:+        self.initial_streams.store(clamped, Ordering::Relaxed);
322:+        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
344:+    /// [`initial_stream_proposal`] assigns the full workload. As the
355:+    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
469:+/// count — file count matters as much as bytes (many small files
480:+pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
635:+        assert_eq!(dial.initial_streams(), 4);
661:+        // codex F1: an in-flight budget smaller than one chunk bounds
693:+    fn initial_stream_proposal_matches_the_retired_daemon_table() {
697:+        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
701:+        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
702:+        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
703:+        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
704:+        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
705:+        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
706:+        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
707:+        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
708:+        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
709:+        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
710:+        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
711:+        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
712:+        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
714:+        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
715:+        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
716:+        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
717:+        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
718:+        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
719:+        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
721:+        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
722:+        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
762:+        probes[0].record_bytes(1024);
933:+    /// table (`docs/plan/SMALL_FILE_CEILING.md`): the small and mixed
936:+    fn shape_table_covers_the_small_file_ceiling_cells() {
942:+        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
944:+        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
948:+            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
954:+        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
963:+        assert_eq!(dial.propose_shape_resize(0), None);
964:+        assert_eq!(dial.propose_shape_resize(1), None);
968:+        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
977:+        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
982:+        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
987:+        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
990:+        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
994:+            dial.propose_shape_resize(4).is_some(),
1004:+            .propose_shape_resize(100)
1009:+            dial.propose_shape_resize(100),
1032:+        // records fresh bytes with zero blocked time.
1035:+            probe.record_bytes(1024);
1062:@@ -1,40 +1,32 @@
1075:-    ClientPushRequest, FileData, FileHeader, TarShardChunk, TarShardComplete, TarShardHeader,
1090:     TarShard {
1104:@@ -242,165 +234,40 @@ pub fn payload_file_count(payloads: &[TransferPayload]) -> usize {
1136:-    // function emits FileData / TarShardChunk over the same gRPC
1196:-            PreparedPayload::TarShard { headers, data } => {
1199:-                    ClientPayload::TarShardHeader(TarShardHeader {
1209:-                        ClientPayload::TarShardChunk(TarShardChunk {
1219:-                send_payload(tx, ClientPayload::TarShardComplete(TarShardComplete {})).await?;
1250: pub fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
1274:@@ -27,41 +27,41 @@ impl PlanOptions {
1277:             small_target: None,
1278:             small_count_target: None,
1305:     let mut small: Vec<PathBuf> = Vec::new();

exec
/bin/zsh -lc "git diff --unified=80 0f922de HEAD -- crates/blit-core/src/remote/transfer/sink.rs | sed -n '1,760p'; git diff --unified=80 0f922de HEAD -- crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '1,260p'; git diff --unified=60 0f922de HEAD -- crates/blit-core/src/transfer_plan.rs crates/blit-core/src/remote/transfer/payload.rs | sed -n '1,520p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lMil5m9Z' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-u3NBvUGJ' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/remote/transfer/sink.rs b/crates/blit-core/src/remote/transfer/sink.rs
index 97ed85c..1895e6d 100644
--- a/crates/blit-core/src/remote/transfer/sink.rs
+++ b/crates/blit-core/src/remote/transfer/sink.rs
@@ -1,604 +1,594 @@
 //! Pluggable write backends for the transfer pipeline.
 //!
 //! Every src→dst combination flows through `TransferSource → plan → prepare → TransferSink`.
 //! Implementations handle the actual write: local filesystem, TCP data plane, etc.
 
 use std::path::{Path, PathBuf};
 use std::sync::Arc;
 
 use async_trait::async_trait;
 use eyre::{Context, Result};
 use filetime::FileTime;
 
 use crate::buffer::BufferSizer;
 use crate::checksum::ChecksumType;
 use crate::copy::{copy_file, resume_copy_file};
 use crate::generated::{ComparisonMode, FileHeader};
 use crate::logger::NoopLogger;
 use crate::remote::transfer::payload::PreparedPayload;
 use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
 use crate::remote::transfer::source::TransferSource;
 
 // Re-export for consumers.
 pub use super::data_plane::DataPlaneSession;
 
 /// Outcome of writing payload(s) to a sink.
 #[derive(Debug, Default, Clone)]
 pub struct SinkOutcome {
     pub files_written: usize,
     pub bytes_written: u64,
 }
 
 impl SinkOutcome {
     pub fn merge(&mut self, other: &SinkOutcome) {
         self.files_written += other.files_written;
         self.bytes_written += other.bytes_written;
     }
 }
 
 /// A pluggable write backend for the transfer pipeline.
 ///
 /// Implementations receive [`PreparedPayload`] items produced by a [`TransferSource`]
 /// and write them to a destination (local filesystem, TCP stream, etc.).
 #[async_trait]
 pub trait TransferSink: Send + Sync {
     /// Write a single prepared payload to the destination.
     async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;
 
     /// Stream a file payload from a borrowed async reader.
     ///
     /// Used by the receive pipeline so file bytes that arrive on a TCP
     /// wire can be written through the same sink as local copies — no
-    /// double-buffering into a `'static` reader. Sinks that don't
-    /// support inbound streaming (e.g. `GrpcFallbackSink`) inherit the
-    /// default error implementation.
+    /// double-buffering into a `'static` reader. Outbound-only sinks
+    /// (e.g. `DataPlaneSink`) inherit the default error implementation.
     async fn write_file_stream(
         &self,
         header: &FileHeader,
         _reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
     ) -> Result<SinkOutcome> {
         eyre::bail!(
             "{} does not support write_file_stream (called for {})",
             std::any::type_name::<Self>(),
             header.relative_path
         )
     }
 
     /// Signal that all payloads have been sent. Flushes buffers, sends terminators, etc.
     /// Default implementation is a no-op.
     async fn finish(&self) -> Result<()> {
         Ok(())
     }
 
     /// Destination root path (if applicable).
     fn root(&self) -> &Path;
 }
 
 // ---------------------------------------------------------------------------
 // FsTransferSink — local filesystem writer
 // ---------------------------------------------------------------------------
 
 /// Configuration for filesystem sink writes.
 #[derive(Debug, Clone)]
 pub struct FsSinkConfig {
     pub preserve_times: bool,
     pub dry_run: bool,
     pub checksum: Option<ChecksumType>,
     pub resume: bool,
     /// R58-followup: comparison policy the sink uses when deciding
     /// whether to copy a `PreparedPayload::File`. The diff_planner
     /// upstream already filters by `compare_mode`, but
     /// `write_file_payload` re-checks before copying as a defense
     /// layer; pre-fix it called `file_needs_copy_with_checksum_type`
     /// which only knows SizeMtime + Checksum, so `Force` and
     /// `IgnoreTimes` were silently downgraded to SizeMtime and
     /// dropped at the sink layer. The default `SizeMtime` keeps
     /// pre-fix behavior for callers that haven't migrated.
     pub compare_mode: ComparisonMode,
 }
 
 impl Default for FsSinkConfig {
     fn default() -> Self {
         Self {
             preserve_times: true,
             dry_run: false,
             checksum: None,
             resume: false,
             compare_mode: ComparisonMode::SizeMtime,
         }
     }
 }
 
 /// Writes files directly to a local filesystem using zero-copy primitives
 /// (copy_file_range, sendfile, clonefile, block clone) where available.
 pub struct FsTransferSink {
     src_root: PathBuf,
     dst_root: PathBuf,
     /// Canonical form of `dst_root` (or its deepest existing
     /// ancestor) captured once at sink construction time. Every
     /// per-entry write resolves the lexical path under `dst_root`
     /// and then verifies it stays inside `canonical_dst_root`
     /// post-symlink. R46-F3: pre-fix the sink only ran lexical
     /// `safe_join`, so a peer-controlled relative path joined under
     /// a `dst_root/link → /outside` symlink would write outside
     /// the destination root.
     canonical_dst_root: Option<PathBuf>,
     config: FsSinkConfig,
-    /// Optional collector for relative paths of successfully-written
-    /// files. Used by remote pull's mirror flow to know which files to
-    /// keep when purging extraneous local entries. Each successful
-    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
-    path_tracker: Option<Arc<std::sync::Mutex<Vec<PathBuf>>>>,
     /// Optional byte-level progress sink. When set,
     /// `write_file_stream` passes it into
     /// `receive_stream_double_buffered` so chunk-granularity
     /// writes report cumulative byte progress against the
     /// daemon's per-transfer counter (c-1a). Unset on the CLI
     /// side; the daemon side sets it via
     /// [`FsTransferSink::with_byte_progress`] from
     /// `ActiveJobGuard::bytes_counter()`.
     byte_progress: Option<ByteProgressSink>,
 }
 
 impl FsTransferSink {
     pub fn new(src_root: PathBuf, dst_root: PathBuf, config: FsSinkConfig) -> Self {
         // Best-effort canonical root capture. We don't fail
         // construction if canonicalize fails (e.g. dst_root is a
         // not-yet-created path under a deeply unusual filesystem) —
         // instead we leave canonical_dst_root as None and the
         // per-write check degrades to lexical-only with a warn.
         // R46-F3: in the common case (dst_root or its ancestor
         // exists) this captures the canonical form needed for
         // symlink-escape rejection.
         let canonical_dst_root = crate::path_safety::canonical_dest_root(&dst_root).ok();
         Self {
             src_root,
             dst_root,
             canonical_dst_root,
             config,
-            path_tracker: None,
             byte_progress: None,
         }
     }
 
-    /// Enable path tracking. After each successful write, the relative
-    /// path of the written file is pushed onto the supplied collector.
-    /// Lets receive callers (e.g. mirror) discover which files survived
-    /// without re-implementing the record dispatch loop.
-    pub fn with_path_tracker(mut self, tracker: Arc<std::sync::Mutex<Vec<PathBuf>>>) -> Self {
-        self.path_tracker = Some(tracker);
-        self
-    }
-
     /// Attach a byte-level progress sink. When set,
     /// `write_file_stream` reports every chunk the data plane
     /// writes against this sink. Used by the daemon side of
     /// remote→remote transfers so `GetState.active[].bytes_completed`
     /// tracks live progress; CLI-side callers omit it.
     pub fn with_byte_progress(mut self, sink: ByteProgressSink) -> Self {
         self.byte_progress = Some(sink);
         self
     }
 
     /// R46-F3: lexical resolve + canonical containment check in one
     /// call. Used by every per-entry write site on this sink so a
     /// peer-controlled relative path can't escape the destination
     /// root via a pre-existing symlink. Falls back to lexical-only
     /// (with a warn) if `canonical_dst_root` was None at
     /// construction time — that path remains exposed but is
     /// extremely unusual in practice.
     fn resolve_destination(&self, wire_path: &str) -> Result<PathBuf> {
         match self.canonical_dst_root.as_ref() {
             Some(canonical) => {
                 crate::path_safety::safe_join_contained(canonical, &self.dst_root, wire_path)
             }
             None => {
                 log::warn!(
                     "FsTransferSink at '{}' has no canonical root; \
                      receive falls back to lexical-only path check \
                      (R46-F3 escape protection unavailable)",
                     self.dst_root.display()
                 );
                 crate::path_safety::safe_join(&self.dst_root, wire_path)
             }
         }
     }
-
-    fn track(&self, rel: &str) {
-        if let Some(tracker) = &self.path_tracker {
-            if let Ok(mut guard) = tracker.lock() {
-                guard.push(PathBuf::from(rel));
-            }
-        }
-    }
 }
 
 #[async_trait]
 impl TransferSink for FsTransferSink {
     async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
         // Resume payloads need async I/O (file open + seek + write
         // through tokio). Local-source payloads (File / TarShard) stay
         // on a blocking thread so the zero-copy cascade and tar
         // extraction can use std::fs.
         let outcome = match payload {
             PreparedPayload::FileBlock {
                 relative_path,
                 offset,
                 bytes,
             } => {
                 write_file_block_payload(
                     &self.dst_root,
                     self.canonical_dst_root.as_deref(),
                     &relative_path,
                     offset,
                     bytes,
                 )
                 .await?
             }
             PreparedPayload::FileBlockComplete {
                 relative_path,
                 total_size,
                 mtime_seconds,
                 permissions,
             } => {
                 let outcome = write_file_block_complete(
                     &self.dst_root,
                     self.canonical_dst_root.as_deref(),
                     &relative_path,
                     total_size,
                     mtime_seconds,
                     permissions,
                 )
                 .await?;
-                if outcome.files_written > 0 {
-                    self.track(&relative_path);
-                }
                 outcome
             }
             // otp-7b: the composite resume payload is send-side only
             // (DataPlaneSink); the receive pipeline decodes per-block
             // FileBlock/FileBlockComplete, never this shape.
             PreparedPayload::ResumeFile { .. } => {
                 eyre::bail!("FsTransferSink does not consume composite ResumeFile payloads")
             }
             PreparedPayload::File(_) | PreparedPayload::TarShard { .. } => {
-                // Capture paths for tracking before payload moves into
-                // the spawn_blocking closure.
-                let tracked_paths: Vec<String> = match &payload {
-                    PreparedPayload::File(h) => vec![h.relative_path.clone()],
-                    PreparedPayload::TarShard { headers, .. } => {
-                        headers.iter().map(|h| h.relative_path.clone()).collect()
-                    }
-                    _ => Vec::new(),
-                };
                 let src_root = self.src_root.clone();
                 let dst_root = self.dst_root.clone();
                 let canonical_dst_root = self.canonical_dst_root.clone();
                 let config = self.config.clone();
                 let outcome = tokio::task::spawn_blocking(move || match payload {
                     PreparedPayload::File(header) => write_file_payload(
                         &src_root,
                         &dst_root,
                         canonical_dst_root.as_deref(),
                         &header,
                         &config,
                     ),
                     PreparedPayload::TarShard { headers, data } => write_tar_shard_payload(
                         &dst_root,
                         canonical_dst_root.as_deref(),
                         &headers,
                         &data,
                         &config,
                     ),
                     _ => unreachable!("outer match guarantees File or TarShard"),
                 })
                 .await
                 .context("sink worker panicked")??;
-                if outcome.files_written > 0 {
-                    for path in tracked_paths {
-                        self.track(&path);
-                    }
-                }
                 outcome
             }
         };
         // c-1b round 2: tar shards and resume blocks land via
         // write_payload, not write_file_stream, so the chunk-
         // granular `receive_stream_double_buffered` hook never
         // fires for them. Report `outcome.bytes_written` here so
         // `GetState.active[].bytes_completed` reflects bytes
         // landed on disk for ALL payload shapes, not just
         // streamed files. Dry-run write paths return
         // `bytes_written: 0` (see `write_file_payload` and
         // `write_tar_shard_payload`'s dry-run early returns), so
         // adding 0 is a no-op for previews — same semantics as
         // `write_file_stream`'s dry-run branch.
         if let Some(bp) = &self.byte_progress {
             bp.report(outcome.bytes_written);
         }
         Ok(outcome)
     }
 
     /// Stream file bytes from the wire to the destination filesystem
     /// using the same double-buffered helper the send side uses. This
     /// is what makes push and pull receive symmetric on the FsTransferSink.
     async fn write_file_stream(
         &self,
         header: &FileHeader,
         reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
     ) -> Result<SinkOutcome> {
         use crate::remote::transfer::data_plane::{
             receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
         };
 
         // R46-F3: lexical resolve + canonical containment check via
         // resolve_destination. Pre-fix this was a bare safe_join,
         // which rejected lexical traversal (`../`) but didn't catch
         // the case where dst_root contained a pre-existing symlink
         // pointing outside (`dst_root/link → /outside`); a peer-
         // controlled relative path `link/file` would then write to
         // `/outside/file`.
         let dst = self
             .resolve_destination(&header.relative_path)
             .with_context(|| format!("validating receive path {:?}", header.relative_path))?;
 
         // R58-F4: dry-run must be side-effect-free. Drain the wire
         // for protocol-stream alignment, but skip the parent-mkdir
         // and the file write. Pre-fix the parent-mkdir ran before
         // the dry-run check below, so `--dry-run` over a remote
         // transfer would create destination directories.
         if self.config.dry_run {
             let mut sink = tokio::io::sink();
             // Dry-run: drain wire bytes for protocol alignment.
             // Do NOT report against `byte_progress` — by contract
             // dry-run is side-effect-free and these bytes never
             // hit user disk; we don't want a daemon-side bytes_completed
             // counter to advance for an aborted preview.
             receive_stream_double_buffered(
                 reader,
                 &mut sink,
                 header.size,
                 RECEIVE_CHUNK_SIZE,
                 None,
             )
             .await
             .with_context(|| format!("draining {} (dry-run)", header.relative_path))?;
             return Ok(SinkOutcome {
                 files_written: 1,
                 bytes_written: 0,
             });
         }
 
         if let Some(parent) = dst.parent() {
             tokio::fs::create_dir_all(parent)
                 .await
                 .with_context(|| format!("creating directory {}", parent.display()))?;
         }
 
         {
             use tokio::io::AsyncWriteExt as _;
             let mut file = tokio::fs::File::create(&dst)
                 .await
                 .with_context(|| format!("creating {}", dst.display()))?;
             receive_stream_double_buffered(
                 reader,
                 &mut file,
                 header.size,
                 RECEIVE_CHUNK_SIZE,
                 self.byte_progress.as_ref(),
             )
             .await
             .with_context(|| format!("writing {}", dst.display()))?;
             // Flush the tokio File's internal buffer state (does NOT
             // fsync — just ensures user-space buffering is drained
             // before we drop the handle and apply mtime). Without
             // this, set_file_mtime races with deferred writes from
             // tokio's blocking-thread pool: 5/8 of mtimes were
             // observed silently bumped to "now" on the receive side.
             //
             // POST_REVIEW_FIXES §1.1: flush failure is a data-loss
             // signal — the user believes the file is durable when it
             // isn't. Propagate, don't swallow.
             file.flush()
                 .await
                 .with_context(|| format!("flushing {}", dst.display()))?;
         }
         // Handle dropped → kernel close() complete → no further
         // metadata churn from this file. Now safe to set mtime by path.
 
         // Intentionally no sync_all: ZFS commits per fsync are
         // multi-second on spinning rust and crater throughput
         // (9.3 → 3.3 Gbps observed). The transfer's durability signal
         // is its END marker plus the OS's own flush; matches rsync's
         // default behavior. Add a config flag if a caller needs sync.
 
         if self.config.preserve_times && header.mtime_seconds > 0 {
             let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
             // Best-effort: cross-fs, root-owned, or ACL-protected
             // destinations can refuse mtime updates. Surface via
             // `log::warn!` so the failure is visible without making
             // it a hard transfer error. POST_REVIEW_FIXES §1.1.
             if let Err(e) = filetime::set_file_mtime(&dst, ft) {
                 log::warn!("set mtime on {}: {}", dst.display(), e);
             }
         }
 
         // Permissions arrive on the wire (Unix mode bits). Apply best-
         // effort; ignore failures (cross-fs, root-owned dst, etc.).
         #[cfg(unix)]
         if header.permissions != 0 {
             use std::os::unix::fs::PermissionsExt;
             if let Err(e) =
                 std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(header.permissions))
             {
                 log::warn!("set permissions on {}: {}", dst.display(), e);
             }
         }
         #[cfg(not(unix))]
         let _ = header.permissions;
 
-        self.track(&header.relative_path);
-
         Ok(SinkOutcome {
             files_written: 1,
             bytes_written: header.size,
         })
     }
 
     fn root(&self) -> &Path {
         &self.dst_root
     }
 }
 
 /// Copy a single file using the zero-copy cascade in `copy::file_copy`.
 fn write_file_payload(
     src_root: &Path,
     dst_root: &Path,
     canonical_dst_root: Option<&Path>,
     header: &FileHeader,
     config: &FsSinkConfig,
 ) -> Result<SinkOutcome> {
+    // An empty relative_path means "the root itself" — the enumeration
+    // root was a single file (same rule as FsTransferSource::open_file):
+    // joining "" can yield a trailing-slash form the OS reads as
+    // "descend into", which fails with ENOTDIR on a regular file. The
+    // local session route (otp-11) is the first caller to send a
+    // file-root File payload through here.
+    if header.relative_path.is_empty() {
+        return copy_root_file_payload(src_root, dst_root, header, config);
+    }
     let src = src_root.join(&header.relative_path);
     // R47-F1: the FsTransferSink::write_payload arm for
     // PreparedPayload::File hit this helper, which previously
     // joined dst_root + header.relative_path lexically. A peer-
     // controlled `link/file` with a pre-existing `dst/link →
     // /outside` symlink would write outside the destination root.
     // Route through the same canonical-containment chokepoint that
     // write_file_stream uses.
     let dst = match canonical_dst_root {
         Some(canonical) => {
             crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
                 .with_context(|| {
                     format!("validating file payload path {:?}", header.relative_path)
                 })?
         }
         None => {
             log::warn!(
                 "write_file_payload at '{}' has no canonical root; \
                  falls back to lexical-only path check (R47-F1 \
                  escape protection unavailable)",
                 dst_root.display()
             );
             crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
                 format!("validating file payload path {:?}", header.relative_path)
             })?
         }
     };
 
+    copy_resolved_file_payload(&src, &dst, header, config)
+}
+
+/// The file-root identity case of [`write_file_payload`]: `src_root`
+/// IS the file and `dst_root` IS the exact target path, so there is
+/// nothing to join and nothing to containment-check — the configured
+/// root cannot escape itself.
+fn copy_root_file_payload(
+    src_root: &Path,
+    dst_root: &Path,
+    header: &FileHeader,
+    config: &FsSinkConfig,
+) -> Result<SinkOutcome> {
+    copy_resolved_file_payload(src_root, dst_root, header, config)
+}
+
+/// Shared tail of the File-payload write: dry-run gate, parent mkdir,
+/// resume/compare/copy cascade, mtime preservation.
+fn copy_resolved_file_payload(
+    src: &Path,
+    dst: &Path,
+    header: &FileHeader,
+    config: &FsSinkConfig,
+) -> Result<SinkOutcome> {
     // R58-F4: dry-run must be side-effect-free. Bail before the
     // parent-mkdir so a dry-run doesn't create destination
     // directories on disk.
     if config.dry_run {
         return Ok(SinkOutcome {
             files_written: 1,
             bytes_written: 0,
         });
     }
 
     if let Some(parent) = dst.parent() {
         std::fs::create_dir_all(parent)
             .with_context(|| format!("creating directory {}", parent.display()))?;
     }
 
     let mut did_copy = false;
     let mut clone_succeeded = false;
 
     if config.resume {
-        let outcome = resume_copy_file(&src, &dst, 0)
+        let outcome = resume_copy_file(src, dst, 0)
             .with_context(|| format!("resume copy {}", header.relative_path))?;
         did_copy = outcome.bytes_transferred > 0;
-    } else if crate::copy::file_needs_copy_with_mode(&src, &dst, config.compare_mode)? {
+    } else if crate::copy::file_needs_copy_with_mode(src, dst, config.compare_mode)? {
         let sizer = BufferSizer::default();
         let logger = NoopLogger;
-        let outcome = copy_file(&src, &dst, &sizer, false, &logger)
+        let outcome = copy_file(src, dst, &sizer, false, &logger)
             .with_context(|| format!("copy {}", header.relative_path))?;
         did_copy = true;
         clone_succeeded = outcome.clone_succeeded;
     }
 
     if config.preserve_times && did_copy && !clone_succeeded {
-        if let Ok(meta) = std::fs::metadata(&src) {
+        if let Ok(meta) = std::fs::metadata(src) {
             if let Ok(modified) = meta.modified() {
                 let ft = FileTime::from_system_time(modified);
-                if let Err(e) = filetime::set_file_mtime(&dst, ft) {
+                if let Err(e) = filetime::set_file_mtime(dst, ft) {
                     log::warn!("set mtime on {}: {}", dst.display(), e);
                 }
             }
         }
     }
 
     Ok(SinkOutcome {
         files_written: 1,
         bytes_written: if did_copy { header.size } else { 0 },
     })
 }
 
 /// Extract an in-memory tar shard to the destination directory.
 fn write_tar_shard_payload(
     dst_root: &Path,
     canonical_dst_root: Option<&Path>,
     headers: &[FileHeader],
     data: &[u8],
     config: &FsSinkConfig,
 ) -> Result<SinkOutcome> {
     if config.dry_run {
         return Ok(SinkOutcome {
             files_written: headers.len(),
             bytes_written: 0,
         });
     }
 
     // Two-phase extraction:
     //   1. Validate + parse the tar serially via the shared
     //      `tar_safety` helper. Tar is a sequential format — entries
     //      can't be read in parallel out of one Archive — and this
     //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
     //   2. Write files to disk in parallel via rayon. Inode creation
     //      and write are the bottleneck for many-small-files shards;
     //      4–8 worker cores can saturate ZFS' inode pipeline.
     //
     // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
     // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
     // toward CPU-or-fs limits.
     use rayon::prelude::*;
 
     use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};
 
     let opts = TarShardExtractOptions::default();
     let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;
 
     // R47-F1: tar shards arriving on FsTransferSink::write_payload
     // (push-receive on the daemon flows through here too) only had
     // lexical safe_join inside safe_extract_tar_shard. A pre-
     // existing dst/link → /outside escape symlink would let an
     // entry path like `link/victim` write through the symlink.
     // Verify each extracted entry's destination against the
     // canonical root before writing.
     if let Some(canonical) = canonical_dst_root {
         for f in &extracted {
             crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
                 format!("tar shard entry {:?} escapes destination root", f.dest_path)
             })?;
         }
     } else {
         log::warn!(
             "write_tar_shard_payload at '{}' has no canonical root; \
              tar-shard receive falls back to lexical-only path \
              checks (R47-F1 escape protection unavailable)",
             dst_root.display()
         );
     }
 
     // Honor the sink's preserve_times toggle by stripping mtimes that
     // the helper would otherwise apply. Permissions are best-effort
     // either way (matches the historical FsTransferSink policy).
     if !config.preserve_times {
         for f in &mut extracted {
             f.mtime = None;
         }
     }
 
     // Write in parallel. Each closure does its own create_dir_all +
     // fs::write + best-effort mtime/permission application — same
     // policy as `tar_safety::write_extracted_file` but inlined so we
@@ -935,517 +925,193 @@ impl<P: Probe> TransferSink for DataPlaneSink<P> {
 /// files, building tar shards) so this measures everything except the write.
 pub struct NullSink {
     label: PathBuf,
 }
 
 impl Default for NullSink {
     fn default() -> Self {
         Self {
             label: PathBuf::from("/dev/null"),
         }
     }
 }
 
 impl NullSink {
     pub fn new() -> Self {
         Self::default()
     }
 }
 
 #[async_trait]
 impl TransferSink for NullSink {
     async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
         match payload {
             PreparedPayload::File(header) => Ok(SinkOutcome {
                 files_written: 1,
                 bytes_written: header.size,
             }),
             PreparedPayload::TarShard { headers, data } => Ok(SinkOutcome {
                 files_written: headers.len(),
                 bytes_written: data.len() as u64,
             }),
             PreparedPayload::FileBlock { bytes, .. } => Ok(SinkOutcome {
                 files_written: 0,
                 bytes_written: bytes.len() as u64,
             }),
             PreparedPayload::FileBlockComplete { .. } => Ok(SinkOutcome::default()),
             // Send-side composite (otp-7b); the receive path this sink
             // benchmarks never produces it.
             PreparedPayload::ResumeFile { .. } => {
                 eyre::bail!("NullSink does not consume composite ResumeFile payloads")
             }
         }
     }
 
     /// Drain the wire so the protocol stream stays aligned, then count
     /// the bytes. Lets `--null` benchmark the receive path end-to-end
     /// without paying for disk writes.
     async fn write_file_stream(
         &self,
         header: &FileHeader,
         reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
     ) -> Result<SinkOutcome> {
         use crate::remote::transfer::data_plane::{
             receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
         };
         let mut sink = tokio::io::sink();
         // --null benchmark: bytes never land on user disk; do
         // not advance a daemon-side progress counter for these
         // drains. Same reasoning as the dry-run path on
         // FsTransferSink.
         let n = receive_stream_double_buffered(
             reader,
             &mut sink,
             header.size,
             RECEIVE_CHUNK_SIZE,
             None,
         )
         .await
         .with_context(|| format!("draining {} (null sink)", header.relative_path))?;
         Ok(SinkOutcome {
             files_written: 1,
             bytes_written: n,
         })
     }
 
     fn root(&self) -> &Path {
         &self.label
     }
 }
 
-// ---------------------------------------------------------------------------
-// GrpcFallbackSink — stream payloads over the gRPC control plane
-// ---------------------------------------------------------------------------
-
-/// Streams payloads to a remote daemon over the gRPC control plane channel.
-///
-/// Used when the TCP data plane is unavailable (`--force-grpc`) or when
-/// negotiation fails. Slower than `DataPlaneSink` but works in restrictive
-/// network environments.
-pub struct GrpcFallbackSink {
-    source: Arc<dyn TransferSource>,
-    tx: tokio::sync::mpsc::Sender<crate::generated::ClientPushRequest>,
-    chunk_bytes: usize,
-    dst_label: PathBuf,
-}
-
-impl GrpcFallbackSink {
-    pub fn new(
-        source: Arc<dyn TransferSource>,
-        tx: tokio::sync::mpsc::Sender<crate::generated::ClientPushRequest>,
-        chunk_bytes: usize,
-        dst_label: PathBuf,
-    ) -> Self {
-        Self {
-            source,
-            tx,
-            chunk_bytes,
-            dst_label,
-        }
-    }
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-RCGAuFi4' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-QOrD0lhE' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/remote/transfer/pipeline.rs b/crates/blit-core/src/remote/transfer/pipeline.rs
index cc5fa4d..5b66f0d 100644
--- a/crates/blit-core/src/remote/transfer/pipeline.rs
+++ b/crates/blit-core/src/remote/transfer/pipeline.rs
@@ -127,181 +127,196 @@ pub async fn execute_sink_pipeline_elastic(
     let capacity = prefetch.max(1) * sink_count;
     let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
 
     // Single shared work queue. Each worker owns exactly one sink but
     // pulls payloads from the common queue, so work is stolen by
     // whichever sink is free rather than pre-assigned round-robin.
     let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
 
     // Cancellation flag set by the first worker that errors. Without it,
     // one sink failing only drops that worker's `work_rx` clone; as long
     // as any other worker is alive `send_async` keeps succeeding, so the
     // forwarder would keep draining `payload_rx` and queueing payloads
     // that can never complete — delaying first-error-wins propagation
     // (Codex review, PR2). With it, the forwarder stops at the next
     // payload boundary and closes the queue so the survivors drain and
     // finish promptly.
     let cancelled = Arc::new(AtomicBool::new(false));
 
     // Dynamic worker membership (`ue-r2-2`): a JoinSet instead of a
     // fixed Vec of handles, plus a per-worker retire flag so a REMOVE
     // can drain exactly one worker. `retire_flags` holds the workers
     // that are live and not yet asked to retire — its length is the
     // count the retire floor checks.
     let mut join_set: tokio::task::JoinSet<(usize, Result<()>)> = tokio::task::JoinSet::new();
     let mut retire_flags: Vec<(usize, tokio::sync::watch::Sender<bool>)> = Vec::new();
     let mut next_slot = 0usize;
 
     #[allow(clippy::too_many_arguments)]
     fn spawn_sink_worker(
         join_set: &mut tokio::task::JoinSet<(usize, Result<()>)>,
         slot: usize,
         sink: Arc<dyn TransferSink>,
         work_rx: flume::Receiver<TransferPayload>,
         source: Arc<dyn TransferSource>,
         progress: Option<RemoteTransferProgress>,
         total: Arc<std::sync::Mutex<SinkOutcome>>,
         cancelled: Arc<std::sync::atomic::AtomicBool>,
         mut retire: tokio::sync::watch::Receiver<bool>,
     ) {
         use std::sync::atomic::Ordering;
         join_set.spawn(async move {
             // Wrap the body so any early-return error trips the shared
             // cancel flag before the `?` unwinds the task.
             let run = async {
                 loop {
                     // Stop pulling queued work once a sibling worker has
                     // errored: first-error-wins should surface without the
                     // survivors draining the rest of the bounded queue.
                     // Interrupting an in-flight prepare/write (true prompt
                     // cancellation) is the AbortOnDrop family, w4-1.
                     if cancelled.load(Ordering::Relaxed) {
                         break;
                     }
                     // ue-r2-2: a retired worker stops at the same payload
                     // boundary; queued payloads stay in the shared queue
                     // for the survivors (dequeue = ownership, so
                     // exactly-once is preserved — flume's RecvFut only
                     // takes an item when it resolves, so racing it is
                     // safe). The watch (not a flag) also frees a worker
                     // parked on an IDLE queue. Its `finish()` below emits
                     // the per-stream END record — the receiver-side
                     // teardown signal.
                     let payload = tokio::select! {
                         biased;
                         _ = retire.changed() => break,
                         recv = work_rx.recv_async() => match recv {
                             Ok(p) => p,
                             Err(_) => break, // queue closed and drained
                         },
                     };
                     let prepared = source
                         .prepare_payload(payload)
                         .await
                         .context("preparing payload")?;
                     let files: Vec<(String, u64)> = match &prepared {
                         PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
                         PreparedPayload::TarShard { headers, .. } => headers
                             .iter()
                             .map(|h| (h.relative_path.clone(), h.size))
                             .collect(),
-                        // Resume-block payloads patch existing files; no
-                        // file-completion event from one-block-at-a-time.
-                        // The composite ResumeFile rides the session data
-                        // plane, which reports no per-file progress here.
+                        // Raw resume-block payloads patch existing files;
+                        // no file-completion event from one-block-at-a-
+                        // time. The composite ResumeFile IS one whole
+                        // file's phase — reported below from the outcome,
+                        // because its byte count (stale blocks only) is
+                        // known only after the write (codex otp-10a F6).
                         PreparedPayload::FileBlock { .. }
                         | PreparedPayload::FileBlockComplete { .. }
                         | PreparedPayload::ResumeFile { .. } => Vec::new(),
                     };
+                    let resumed_file: Option<String> = match &prepared {
+                        PreparedPayload::ResumeFile { header, .. } => {
+                            Some(header.relative_path.clone())
+                        }
+                        _ => None,
+                    };
                     let outcome = sink
                         .write_payload(prepared)
                         .await
                         .context("writing payload")?;
                     if let Some(p) = &progress {
                         // Contract (progress.rs): bytes ride Payload, one
                         // FileComplete per file. `size` is the planned
                         // manifest size — the value this lane has always
                         // reported, now on the right variant.
                         for (name, size) in &files {
                             p.report_payload(0, *size);
                             p.report_file_complete(name.clone());
                         }
+                        // A resumed file finishes like any other (w6-1:
+                        // counted once, per-file lane); its bytes are the
+                        // stale blocks actually sent.
+                        if let Some(name) = resumed_file {
+                            p.report_payload(0, outcome.bytes_written);
+                            p.report_file_complete(name);
+                        }
                     }
                     let mut t = total.lock().unwrap();
                     t.merge(&outcome);
                 }
                 sink.finish().await?;
                 Ok::<(), eyre::Report>(())
             }
             .await;
             if run.is_err() {
                 // Signal the forwarder (and implicitly the other workers,
                 // once the queue closes) to stop feeding new work.
                 cancelled.store(true, Ordering::Relaxed);
             }
             (slot, run)
         });
     }
 
     for sink in sinks {
         let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
         let slot = next_slot;
         next_slot += 1;
         retire_flags.push((slot, retire_tx));
         spawn_sink_worker(
             &mut join_set,
             slot,
             sink,
             work_rx.clone(),
             source.clone(),
             progress.cloned(),
             total.clone(),
             cancelled.clone(),
             retire_rx,
         );
     }
 
     // Forwarder: move payloads from the incoming channel onto the shared
     // work queue. `send_async` applies back-pressure (bounded queue); if
     // every worker has gone away (e.g. all sinks errored) the send fails
     // and we stop. It also bails as soon as a worker sets `cancelled`, so
     // a single sink error halts intake promptly instead of waiting for
     // every worker to drop. Dropping `work_tx` on end-of-stream (or on
     // cancel) signals the workers. (The executor keeps a `work_rx` clone
     // for late-added workers — flume disconnect is sender-driven, so the
     // retained receiver does not keep the queue alive.)
     let cancelled_fwd = cancelled.clone();
     let forwarder = tokio::spawn(async move {
         while let Some(payload) = payload_rx.recv().await {
             if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
                 // A worker errored — stop draining the producer and let
                 // the queue close so survivors finish and the error
                 // surfaces without delay.
                 return;
             }
             if work_tx.send_async(payload).await.is_err() {
                 // All workers dropped their receivers — nothing left to
                 // feed; treat as shutdown.
                 return;
             }
         }
         // Dropping work_tx closes the queue → workers see Disconnected
         // after draining and run finish().
     });
 
     // Supervise: join workers (first error wins) while servicing the
     // resize control channel. `join_next() == None` means every worker
     // — initial and added — has finished, which only happens once the
     // queue closed and drained (or errored/retired), so control is
     // moot beyond that point.
     let mut control_rx = control_rx;
     let mut first_err: Option<eyre::Report> = None;
     loop {
         let control_recv = async {
             match control_rx.as_mut() {
                 Some(rx) => rx.recv().await,
                 None => std::future::pending().await,
             }
         };
         tokio::select! {
             // ue-r2-2 review (panel F2): biased, control FIRST — a
             // ready Add must be processed before the join arm can
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-9MwtyUQy' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lA1Q2JJ6' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/remote/transfer/payload.rs b/crates/blit-core/src/remote/transfer/payload.rs
index 531718d..ab5ae4a 100644
--- a/crates/blit-core/src/remote/transfer/payload.rs
+++ b/crates/blit-core/src/remote/transfer/payload.rs
@@ -1,80 +1,72 @@
 use std::collections::HashMap;
 use std::path::{Path, PathBuf};
 
 use eyre::{bail, eyre, Context, Result};
 use futures::{stream, StreamExt};
-use tokio::io::AsyncReadExt;
-use tokio::sync::mpsc;
 use tokio::task;
 
 use crate::fs_enum::FileEntry;
-use crate::generated::client_push_request::Payload as ClientPayload;
-use crate::generated::{
-    ClientPushRequest, FileData, FileHeader, TarShardChunk, TarShardComplete, TarShardHeader,
-    UploadComplete,
-};
+use crate::generated::FileHeader;
 use crate::transfer_plan::{self, PlanOptions, TransferTask};
 use tar::{Builder, EntryType, Header};
 
-use super::data_plane::CONTROL_PLANE_CHUNK_SIZE;
-use super::progress::RemoteTransferProgress;
 use crate::remote::transfer::source::TransferSource;
 use std::sync::Arc;
 
 #[derive(Debug, Clone)]
 pub enum TransferPayload {
     File(FileHeader),
     TarShard {
         headers: Vec<FileHeader>,
     },
     /// Resume protocol: overwrite a block of an existing file.
     FileBlock {
         relative_path: String,
         offset: u64,
         size: u64,
     },
     /// Resume protocol: finalize a resumed file (truncate to total_size).
     FileBlockComplete {
         relative_path: String,
         total_size: u64,
     },
     /// otp-7b: one resume-flagged file's WHOLE block phase as a single
     /// work item — the manifest header plus the destination's block
     /// hashes. Choreography-originated only (the session's send half
     /// queues it once the file's `BlockHashList` has arrived); the
     /// outbound planner never emits it. One work item ⇒ one pipeline
     /// worker ⇒ one socket, which is what keeps the record strictly
     /// serialized (every `BLOCK` before its `BLOCK_COMPLETE`, no
     /// cross-socket reorder hazard against the truncate+stamp).
     ResumeFile {
         header: FileHeader,
         block_size: u32,
         dest_hashes: Vec<Vec<u8>>,
     },
 }
 
 pub async fn prepare_payload(
     payload: TransferPayload,
     source_root: PathBuf,
 ) -> Result<PreparedPayload> {
     match payload {
         TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
         TransferPayload::TarShard { headers } => {
             let headers_clone = headers.clone();
             let source_root_clone = source_root.clone();
             let data =
                 task::spawn_blocking(move || build_tar_shard(&source_root_clone, &headers_clone))
                     .await
                     .map_err(|err| eyre!("tar shard worker failed: {err}"))??;
             Ok(PreparedPayload::TarShard { headers, data })
         }
         // Resume payloads can only originate on the receive side (parsed
         // off the wire by DataPlaneSource); the file-system source never
         // produces them.
         TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
             bail!("FileBlock payloads cannot be prepared from a filesystem source")
         }
         // otp-7b: nothing to prepare — the block-diff streams the source
         // file inside the sink write (DataPlaneSink), where the record's
         // strict serialization lives. Pass through.
         TransferPayload::ResumeFile {
@@ -202,225 +194,100 @@ pub fn plan_transfer_payloads(
                 }
             }
         }
     }
 
     for (_, header) in header_map.into_iter() {
         payloads.push(TransferPayload::File(header));
     }
 
     // Sort payloads: tar shards first (small, distribute well across streams),
     // then files ascending by size. This ensures all streams stay busy with
     // small work before a single large file monopolizes one stream's tail.
     // Resume variants (FileBlock / FileBlockComplete) are receive-only and
     // never appear here — plan_transfer_payloads is the outbound planner.
     payloads.sort_by_key(|p| match p {
         TransferPayload::TarShard { .. } => (0, 0),
         TransferPayload::File(h) => (1, h.size),
         TransferPayload::ResumeFile { header, .. } => (1, header.size),
         TransferPayload::FileBlock { size, .. } => (2, *size),
         TransferPayload::FileBlockComplete { .. } => (3, 0),
     });
 
     Ok(payloads)
 }
 
 pub fn payload_file_count(payloads: &[TransferPayload]) -> usize {
     payloads
         .iter()
         .map(|payload| match payload {
             TransferPayload::File(_) => 1,
             TransferPayload::TarShard { headers } => headers.len(),
             // Resume payloads patch existing files in-place — they
             // don't add to the "files transferred" count.
             TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => 0,
             // One composite resume item completes exactly one file.
             TransferPayload::ResumeFile { .. } => 1,
         })
         .sum()
 }
 
 fn normalize_relative_path(path: &Path) -> String {
     // Canonical POSIX form — see `crate::path_posix` for why a
     // component-walk is correct on every platform and the historical
     // string `replace('\\', "/")` was destructive on POSIX.
     crate::path_posix::relative_path_to_posix(path)
 }
 
 pub fn prepared_payload_stream(
     payloads: Vec<TransferPayload>,
     source: Arc<dyn TransferSource>,
     prefetch: usize,
 ) -> impl futures::Stream<Item = Result<PreparedPayload>> {
     let capacity = prefetch.max(1);
     stream::iter(payloads.into_iter().map(move |payload| {
         let source = source.clone();
         async move { source.prepare_payload(payload).await }
     }))
     .buffered(capacity)
 }
 
-pub async fn transfer_payloads_via_control_plane(
-    source: Arc<dyn TransferSource>,
-    payloads: Vec<TransferPayload>,
-    tx: &mpsc::Sender<ClientPushRequest>,
-    finish: bool,
-    progress: Option<&RemoteTransferProgress>,
-    chunk_bytes: usize,
-    payload_prefetch: usize,
-) -> Result<()> {
-    // audit-h3c slice 1: clamp at the gRPC fallback ceiling for the
-    // same reason GrpcFallbackSink / GrpcServerStreamingSink do — this
-    // function emits FileData / TarShardChunk over the same gRPC
-    // control plane and must produce frames at observable cadence.
-    // No live caller today (grep returns zero matches), but the
-    // function is `pub` and re-exported, so any future caller would
-    // silently bypass the cap without this line.
-    let chunk_size =
-        super::grpc_fallback::clamp_fallback_chunk_size(chunk_bytes.max(CONTROL_PLANE_CHUNK_SIZE));
-    let mut buffer = vec![0u8; chunk_size];
-    let mut prepared_stream = prepared_payload_stream(payloads, source.clone(), payload_prefetch);
-
-    while let Some(prepared) = prepared_stream.next().await {
-        match prepared? {
-            PreparedPayload::File(header) => {
-                send_payload(tx, ClientPayload::FileManifest(header.clone())).await?;
-
-                if header.size == 0 {
-                    if let Some(progress) = progress {
-                        progress.report_file_complete(header.relative_path.clone());
-                    }
-                    continue;
-                }
-
-                let mut file = source
-                    .open_file(&header)
-                    .await
-                    .with_context(|| format!("opening {}", header.relative_path))?;
-
-                let mut remaining = header.size;
-                while remaining > 0 {
-                    let to_read = buffer.len().min(remaining as usize);
-                    let chunk = file
-                        .read(&mut buffer[..to_read])
-                        .await
-                        .with_context(|| format!("reading {}", header.relative_path))?;
-                    if chunk == 0 {
-                        bail!(
-                            "unexpected EOF while reading {} ({} bytes remaining)",
-                            header.relative_path,
-                            remaining
-                        );
-                    }
-
-                    send_payload(
-                        tx,
-                        ClientPayload::FileData(FileData {
-                            content: buffer[..chunk].to_vec(),
-                        }),
-                    )
-                    .await?;
-                    if let Some(progress) = progress {
-                        progress.report_payload(0, chunk as u64);
-                    }
-                    remaining -= chunk as u64;
-                }
-                if let Some(progress) = progress {
-                    // Bytes already rode the per-chunk Payload reports
-                    // above; FileComplete only marks the file done.
-                    progress.report_file_complete(header.relative_path.clone());
-                }
-            }
-            PreparedPayload::TarShard { headers, data } => {
-                send_payload(
-                    tx,
-                    ClientPayload::TarShardHeader(TarShardHeader {
-                        files: headers.clone(),
-                        archive_size: data.len() as u64,
-                    }),
-                )
-                .await?;
-
-                for chunk in data.chunks(chunk_size) {
-                    send_payload(
-                        tx,
-                        ClientPayload::TarShardChunk(TarShardChunk {
-                            content: chunk.to_vec(),
-                        }),
-                    )
-                    .await?;
-                    if let Some(progress) = progress {
-                        progress.report_payload(0, chunk.len() as u64);
-                    }
-                }
-
-                send_payload(tx, ClientPayload::TarShardComplete(TarShardComplete {})).await?;
-                if let Some(progress) = progress {
-                    for header in &headers {
-                        progress.report_file_complete(header.relative_path.clone());
-                    }
-                }
-            }
-            // Resume variants never traverse the gRPC control plane.
-            PreparedPayload::FileBlock { .. }
-            | PreparedPayload::FileBlockComplete { .. }
-            | PreparedPayload::ResumeFile { .. } => {
-                bail!("resume payloads cannot traverse the gRPC control plane");
-            }
-        }
-    }
-
-    if finish {
-        send_payload(tx, ClientPayload::UploadComplete(UploadComplete {})).await?;
-    }
-
-    Ok(())
-}
-
-async fn send_payload(tx: &mpsc::Sender<ClientPushRequest>, payload: ClientPayload) -> Result<()> {
-    tx.send(ClientPushRequest {
-        payload: Some(payload),
-    })
-    .await
-    .map_err(|_| eyre!("failed to send push request payload"))
-}
-
 pub fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
     let mut builder = Builder::new(Vec::new());
 
     for header in headers {
         let rel = Path::new(&header.relative_path);
         // Empty relative_path = "root is itself the file" (single-file
         // source). See FsTransferSource::open_file for context — join("")
         // can preserve a trailing separator that File::open rejects.
         let full_path = if header.relative_path.is_empty() {
             source_root.to_path_buf()
         } else {
             source_root.join(rel)
         };
         let mut file = std::fs::File::open(&full_path)
             .with_context(|| format!("opening {}", full_path.display()))?;
 
         let mut tar_header = Header::new_gnu();
         tar_header.set_entry_type(EntryType::Regular);
         let mode = if header.permissions == 0 {
             0o644
         } else {
             header.permissions
         };
         tar_header.set_mode(mode);
         tar_header.set_size(header.size);
         let mtime = if header.mtime_seconds >= 0 {
             header.mtime_seconds as u64
         } else {
             0
         };
         tar_header.set_mtime(mtime);
         tar_header.set_cksum();
 
         builder
             .append_data(&mut tar_header, rel, &mut file)
             .with_context(|| format!("adding {} to tar shard", full_path.display()))?;
     }
 
     builder.into_inner().context("finalizing tar shard")
 }
diff --git a/crates/blit-core/src/transfer_plan.rs b/crates/blit-core/src/transfer_plan.rs
index 7cee0ed..8950654 100644
--- a/crates/blit-core/src/transfer_plan.rs
+++ b/crates/blit-core/src/transfer_plan.rs
@@ -1,107 +1,107 @@
 use std::collections::HashMap;
 use std::path::{Path, PathBuf};
 
 /// Adaptive transfer task classification shared across push, pull, and local engines.
 #[derive(Clone, Debug)]
 pub enum TransferTask {
     TarShard(Vec<PathBuf>),
     /// Bundle of medium files to send back-to-back in a single worker turn.
     RawBundle(Vec<PathBuf>),
     /// Large single file; delta/range logic decides stripes internally.
     Large {
         path: PathBuf,
     },
 }
 
 /// Planner tuning options shared across engines.
 #[derive(Clone, Copy, Debug)]
 pub struct PlanOptions {
     pub force_tar: bool,
     pub small_target: Option<u64>,
     pub small_count_target: Option<usize>,
     pub medium_target: Option<u64>,
 }
 
 impl PlanOptions {
     pub fn new() -> Self {
         Self {
             force_tar: false,
             small_target: None,
             small_count_target: None,
             medium_target: None,
         }
     }
 }
 
 impl Default for PlanOptions {
     fn default() -> Self {
         Self::new()
     }
 }
 
 /// Build an adaptive transfer task queue from enumerated file entries.
 ///
 /// The heuristics mirror the original `net_async::client::build_plan` logic so that
 /// every mode (push, pull, local) can share the same task ordering. Wire
 /// chunk sizing is NOT planned here — it is owned by the live
-/// [`crate::engine::TransferDial`] (w2-2: this module's static 16/32 MiB
+/// [`crate::dial::TransferDial`] (w2-2: this module's static 16/32 MiB
 /// chunk ladder was dead policy — every remote path overrode it from the
 /// dial and no consumer read the planned value).
 pub fn build_plan(
     files: &[crate::fs_enum::FileEntry],
     rootsrc: &Path,
     options: PlanOptions,
 ) -> Vec<TransferTask> {
     let mut size_map: HashMap<PathBuf, u64> = HashMap::new();
     let mut small: Vec<PathBuf> = Vec::new();
     let mut medium: Vec<(PathBuf, u64)> = Vec::new();
     let mut total_medium_bytes: u64 = 0;
     let mut large_files: Vec<TransferTask> = Vec::new();
     let mut total_bytes: u128 = 0;
     for e in files {
         if e.is_directory {
             continue;
         }
         let rel = e
             .path
             .strip_prefix(rootsrc)
             .unwrap_or(&e.path)
             .to_path_buf();
         size_map.insert(rel.clone(), e.size);
         total_bytes += e.size as u128;
         if e.size < 1_048_576 {
             // <1MB
             small.push(rel);
         } else if e.size < 256 * 1_048_576 {
             // <256MB
             medium.push((rel, e.size));
             total_medium_bytes = total_medium_bytes.saturating_add(e.size);
         } else {
             // Large: schedule as single large-file task; range/delta decided when sending
             large_files.push(TransferTask::Large { path: rel.clone() });
         }
     }
     // Shard small files into larger tars for multi-GB workloads
     small.sort_by_key(|p| p.as_os_str().len());
 
     let mut small_tasks: Vec<TransferTask> = Vec::new();
     let small_count = small.len();
     let total_small_bytes: u64 = small.iter().fold(0u64, |acc, p| {
         acc.saturating_add(*size_map.get(p).unwrap_or(&(64 * 1024)))
     });
     let avg_small_size = if small_count == 0 {
         0
     } else {
         total_small_bytes / small_count as u64
     };
 
     // Tar shards only make sense for 2+ files (batching) — a single file
     // gains nothing from tar wrapping and breaks the empty-path case
     // produced by enumerating a file root directly.
     let use_tar = if options.force_tar {
         small_count >= 1
     } else if small_count < 2 {
         false
     } else {
         small_count >= 32 || avg_small_size <= 128 * 1024
     };

exec
/bin/zsh -lc 'git diff --unified=100 0f922de HEAD -- crates/blit-core/src/remote/transfer/sink.rs | rg -n "''^@@|fn write_tar_shard_payload"; git diff --unified=100 0f922de HEAD -- crates/blit-core/src/remote/transfer/sink.rs | sed -n '"'430,760p'; git show 0f922de:crates/blit-core/src/remote/transfer/sink.rs | nl -ba | sed -n '520,660p'; nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '500,640p'; nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '420,510p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-FoR15uIk' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-DHOWNbL5' (errno=Operation not permitted)
5:@@ -1,624 +1,614 @@
583: fn write_tar_shard_payload(
670:@@ -915,557 +905,233 @@ impl<P: Probe> TransferSink for DataPlaneSink<P> {
1246:@@ -1576,499 +1242,200 @@ mod tests {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZAHDKqeT' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-26dgZb1V' (errno=Operation not permitted)
             }
         }
 
         // Permissions arrive on the wire (Unix mode bits). Apply best-
         // effort; ignore failures (cross-fs, root-owned dst, etc.).
         #[cfg(unix)]
         if header.permissions != 0 {
             use std::os::unix::fs::PermissionsExt;
             if let Err(e) =
                 std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(header.permissions))
             {
                 log::warn!("set permissions on {}: {}", dst.display(), e);
             }
         }
         #[cfg(not(unix))]
         let _ = header.permissions;
 
-        self.track(&header.relative_path);
-
         Ok(SinkOutcome {
             files_written: 1,
             bytes_written: header.size,
         })
     }
 
     fn root(&self) -> &Path {
         &self.dst_root
     }
 }
 
 /// Copy a single file using the zero-copy cascade in `copy::file_copy`.
 fn write_file_payload(
     src_root: &Path,
     dst_root: &Path,
     canonical_dst_root: Option<&Path>,
     header: &FileHeader,
     config: &FsSinkConfig,
 ) -> Result<SinkOutcome> {
+    // An empty relative_path means "the root itself" — the enumeration
+    // root was a single file (same rule as FsTransferSource::open_file):
+    // joining "" can yield a trailing-slash form the OS reads as
+    // "descend into", which fails with ENOTDIR on a regular file. The
+    // local session route (otp-11) is the first caller to send a
+    // file-root File payload through here.
+    if header.relative_path.is_empty() {
+        return copy_root_file_payload(src_root, dst_root, header, config);
+    }
     let src = src_root.join(&header.relative_path);
     // R47-F1: the FsTransferSink::write_payload arm for
     // PreparedPayload::File hit this helper, which previously
     // joined dst_root + header.relative_path lexically. A peer-
     // controlled `link/file` with a pre-existing `dst/link →
     // /outside` symlink would write outside the destination root.
     // Route through the same canonical-containment chokepoint that
     // write_file_stream uses.
     let dst = match canonical_dst_root {
         Some(canonical) => {
             crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
                 .with_context(|| {
                     format!("validating file payload path {:?}", header.relative_path)
                 })?
         }
         None => {
             log::warn!(
                 "write_file_payload at '{}' has no canonical root; \
                  falls back to lexical-only path check (R47-F1 \
                  escape protection unavailable)",
                 dst_root.display()
             );
             crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
                 format!("validating file payload path {:?}", header.relative_path)
             })?
         }
     };
 
+    copy_resolved_file_payload(&src, &dst, header, config)
+}
+
+/// The file-root identity case of [`write_file_payload`]: `src_root`
+/// IS the file and `dst_root` IS the exact target path, so there is
+/// nothing to join and nothing to containment-check — the configured
+/// root cannot escape itself.
+fn copy_root_file_payload(
+    src_root: &Path,
+    dst_root: &Path,
+    header: &FileHeader,
+    config: &FsSinkConfig,
+) -> Result<SinkOutcome> {
+    copy_resolved_file_payload(src_root, dst_root, header, config)
+}
+
+/// Shared tail of the File-payload write: dry-run gate, parent mkdir,
+/// resume/compare/copy cascade, mtime preservation.
+fn copy_resolved_file_payload(
+    src: &Path,
+    dst: &Path,
+    header: &FileHeader,
+    config: &FsSinkConfig,
+) -> Result<SinkOutcome> {
     // R58-F4: dry-run must be side-effect-free. Bail before the
     // parent-mkdir so a dry-run doesn't create destination
     // directories on disk.
     if config.dry_run {
         return Ok(SinkOutcome {
             files_written: 1,
             bytes_written: 0,
         });
     }
 
     if let Some(parent) = dst.parent() {
         std::fs::create_dir_all(parent)
             .with_context(|| format!("creating directory {}", parent.display()))?;
     }
 
     let mut did_copy = false;
     let mut clone_succeeded = false;
 
     if config.resume {
-        let outcome = resume_copy_file(&src, &dst, 0)
+        let outcome = resume_copy_file(src, dst, 0)
             .with_context(|| format!("resume copy {}", header.relative_path))?;
         did_copy = outcome.bytes_transferred > 0;
-    } else if crate::copy::file_needs_copy_with_mode(&src, &dst, config.compare_mode)? {
+    } else if crate::copy::file_needs_copy_with_mode(src, dst, config.compare_mode)? {
         let sizer = BufferSizer::default();
         let logger = NoopLogger;
-        let outcome = copy_file(&src, &dst, &sizer, false, &logger)
+        let outcome = copy_file(src, dst, &sizer, false, &logger)
             .with_context(|| format!("copy {}", header.relative_path))?;
         did_copy = true;
         clone_succeeded = outcome.clone_succeeded;
     }
 
     if config.preserve_times && did_copy && !clone_succeeded {
-        if let Ok(meta) = std::fs::metadata(&src) {
+        if let Ok(meta) = std::fs::metadata(src) {
             if let Ok(modified) = meta.modified() {
                 let ft = FileTime::from_system_time(modified);
-                if let Err(e) = filetime::set_file_mtime(&dst, ft) {
+                if let Err(e) = filetime::set_file_mtime(dst, ft) {
                     log::warn!("set mtime on {}: {}", dst.display(), e);
                 }
             }
         }
     }
 
     Ok(SinkOutcome {
         files_written: 1,
         bytes_written: if did_copy { header.size } else { 0 },
     })
 }
 
 /// Extract an in-memory tar shard to the destination directory.
 fn write_tar_shard_payload(
     dst_root: &Path,
     canonical_dst_root: Option<&Path>,
     headers: &[FileHeader],
     data: &[u8],
     config: &FsSinkConfig,
 ) -> Result<SinkOutcome> {
     if config.dry_run {
         return Ok(SinkOutcome {
             files_written: headers.len(),
             bytes_written: 0,
         });
     }
 
     // Two-phase extraction:
     //   1. Validate + parse the tar serially via the shared
     //      `tar_safety` helper. Tar is a sequential format — entries
     //      can't be read in parallel out of one Archive — and this
     //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
     //   2. Write files to disk in parallel via rayon. Inode creation
     //      and write are the bottleneck for many-small-files shards;
     //      4–8 worker cores can saturate ZFS' inode pipeline.
     //
     // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
     // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
     // toward CPU-or-fs limits.
     use rayon::prelude::*;
 
     use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};
 
     let opts = TarShardExtractOptions::default();
     let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;
 
     // R47-F1: tar shards arriving on FsTransferSink::write_payload
     // (push-receive on the daemon flows through here too) only had
     // lexical safe_join inside safe_extract_tar_shard. A pre-
     // existing dst/link → /outside escape symlink would let an
     // entry path like `link/victim` write through the symlink.
     // Verify each extracted entry's destination against the
     // canonical root before writing.
     if let Some(canonical) = canonical_dst_root {
         for f in &extracted {
             crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
                 format!("tar shard entry {:?} escapes destination root", f.dest_path)
             })?;
         }
     } else {
         log::warn!(
             "write_tar_shard_payload at '{}' has no canonical root; \
              tar-shard receive falls back to lexical-only path \
              checks (R47-F1 escape protection unavailable)",
             dst_root.display()
         );
     }
 
     // Honor the sink's preserve_times toggle by stripping mtimes that
     // the helper would otherwise apply. Permissions are best-effort
     // either way (matches the historical FsTransferSink policy).
     if !config.preserve_times {
         for f in &mut extracted {
             f.mtime = None;
         }
     }
 
     // Write in parallel. Each closure does its own create_dir_all +
     // fs::write + best-effort mtime/permission application — same
     // policy as `tar_safety::write_extracted_file` but inlined so we
     // can return per-file byte counts for the SinkOutcome.
     let results: Vec<Result<u64>> = extracted
         .into_par_iter()
         .map(|f: ExtractedFile| -> Result<u64> {
             if let Some(parent) = f.dest_path.parent() {
                 std::fs::create_dir_all(parent)
                     .with_context(|| format!("create dir {}", parent.display()))?;
             }
             std::fs::write(&f.dest_path, &f.contents)
                 .with_context(|| format!("write {}", f.dest_path.display()))?;
             if let Some(ft) = f.mtime {
                 if let Err(e) = filetime::set_file_mtime(&f.dest_path, ft) {
                     log::warn!("set mtime on {}: {}", f.dest_path.display(), e);
                 }
             }
             #[cfg(unix)]
             if let Some(perms) = f.permissions {
                 use std::os::unix::fs::PermissionsExt;
                 if let Err(e) =
                     std::fs::set_permissions(&f.dest_path, std::fs::Permissions::from_mode(perms))
@@ -915,557 +905,233 @@ impl<P: Probe> TransferSink for DataPlaneSink<P> {
     }
 
     async fn finish(&self) -> Result<()> {
         let mut session = self.session.lock().await;
         session.finish().await
     }
 
     fn root(&self) -> &Path {
         &self.dst_root
     }
 }
 
 // ---------------------------------------------------------------------------
 // NullSink — discard data, count bytes (for benchmarking)
 // ---------------------------------------------------------------------------
 
 /// Discards all payload data, counting files and bytes.
 ///
 /// Useful for benchmarking source + network throughput without destination
 /// I/O as a bottleneck. The pipeline still prepares payloads (reading source
 /// files, building tar shards) so this measures everything except the write.
 pub struct NullSink {
     label: PathBuf,
 }
 
 impl Default for NullSink {
     fn default() -> Self {
         Self {
             label: PathBuf::from("/dev/null"),
         }
     }
 }
 
 impl NullSink {
     pub fn new() -> Self {
         Self::default()
     }
 }
 
 #[async_trait]
 impl TransferSink for NullSink {
     async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
         match payload {
             PreparedPayload::File(header) => Ok(SinkOutcome {
                 files_written: 1,
                 bytes_written: header.size,
             }),
             PreparedPayload::TarShard { headers, data } => Ok(SinkOutcome {
                 files_written: headers.len(),
                 bytes_written: data.len() as u64,
             }),
             PreparedPayload::FileBlock { bytes, .. } => Ok(SinkOutcome {
                 files_written: 0,
                 bytes_written: bytes.len() as u64,
             }),
             PreparedPayload::FileBlockComplete { .. } => Ok(SinkOutcome::default()),
             // Send-side composite (otp-7b); the receive path this sink
             // benchmarks never produces it.
             PreparedPayload::ResumeFile { .. } => {
                 eyre::bail!("NullSink does not consume composite ResumeFile payloads")
             }
         }
     }
 
     /// Drain the wire so the protocol stream stays aligned, then count
     /// the bytes. Lets `--null` benchmark the receive path end-to-end
     /// without paying for disk writes.
     async fn write_file_stream(
         &self,
         header: &FileHeader,
         reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
     ) -> Result<SinkOutcome> {
         use crate::remote::transfer::data_plane::{
             receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
         };
         let mut sink = tokio::io::sink();
         // --null benchmark: bytes never land on user disk; do
         // not advance a daemon-side progress counter for these
         // drains. Same reasoning as the dry-run path on
         // FsTransferSink.
         let n = receive_stream_double_buffered(
             reader,
             &mut sink,
             header.size,
             RECEIVE_CHUNK_SIZE,
             None,
         )
         .await
         .with_context(|| format!("draining {} (null sink)", header.relative_path))?;
         Ok(SinkOutcome {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-QU5UWZMp' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-dQeXrHVV' (errno=Operation not permitted)
   520	    if config.preserve_times && did_copy && !clone_succeeded {
   521	        if let Ok(meta) = std::fs::metadata(&src) {
   522	            if let Ok(modified) = meta.modified() {
   523	                let ft = FileTime::from_system_time(modified);
   524	                if let Err(e) = filetime::set_file_mtime(&dst, ft) {
   525	                    log::warn!("set mtime on {}: {}", dst.display(), e);
   526	                }
   527	            }
   528	        }
   529	    }
   530	
   531	    Ok(SinkOutcome {
   532	        files_written: 1,
   533	        bytes_written: if did_copy { header.size } else { 0 },
   534	    })
   535	}
   536	
   537	/// Extract an in-memory tar shard to the destination directory.
   538	fn write_tar_shard_payload(
   539	    dst_root: &Path,
   540	    canonical_dst_root: Option<&Path>,
   541	    headers: &[FileHeader],
   542	    data: &[u8],
   543	    config: &FsSinkConfig,
   544	) -> Result<SinkOutcome> {
   545	    if config.dry_run {
   546	        return Ok(SinkOutcome {
   547	            files_written: headers.len(),
   548	            bytes_written: 0,
   549	        });
   550	    }
   551	
   552	    // Two-phase extraction:
   553	    //   1. Validate + parse the tar serially via the shared
   554	    //      `tar_safety` helper. Tar is a sequential format — entries
   555	    //      can't be read in parallel out of one Archive — and this
   556	    //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
   557	    //   2. Write files to disk in parallel via rayon. Inode creation
   558	    //      and write are the bottleneck for many-small-files shards;
   559	    //      4–8 worker cores can saturate ZFS' inode pipeline.
   560	    //
   561	    // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
   562	    // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
   563	    // toward CPU-or-fs limits.
   564	    use rayon::prelude::*;
   565	
   566	    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};
   567	
   568	    let opts = TarShardExtractOptions::default();
   569	    let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;
   570	
   571	    // R47-F1: tar shards arriving on FsTransferSink::write_payload
   572	    // (push-receive on the daemon flows through here too) only had
   573	    // lexical safe_join inside safe_extract_tar_shard. A pre-
   574	    // existing dst/link → /outside escape symlink would let an
   575	    // entry path like `link/victim` write through the symlink.
   576	    // Verify each extracted entry's destination against the
   577	    // canonical root before writing.
   578	    if let Some(canonical) = canonical_dst_root {
   579	        for f in &extracted {
   580	            crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
   581	                format!("tar shard entry {:?} escapes destination root", f.dest_path)
   582	            })?;
   583	        }
   584	    } else {
   585	        log::warn!(
   586	            "write_tar_shard_payload at '{}' has no canonical root; \
   587	             tar-shard receive falls back to lexical-only path \
   588	             checks (R47-F1 escape protection unavailable)",
   589	            dst_root.display()
   590	        );
   591	    }
   592	
   593	    // Honor the sink's preserve_times toggle by stripping mtimes that
   594	    // the helper would otherwise apply. Permissions are best-effort
   595	    // either way (matches the historical FsTransferSink policy).
   596	    if !config.preserve_times {
   597	        for f in &mut extracted {
   598	            f.mtime = None;
   599	        }
   600	    }
   601	
   602	    // Write in parallel. Each closure does its own create_dir_all +
   603	    // fs::write + best-effort mtime/permission application — same
   604	    // policy as `tar_safety::write_extracted_file` but inlined so we
   605	    // can return per-file byte counts for the SinkOutcome.
   606	    let results: Vec<Result<u64>> = extracted
   607	        .into_par_iter()
   608	        .map(|f: ExtractedFile| -> Result<u64> {
   609	            if let Some(parent) = f.dest_path.parent() {
   610	                std::fs::create_dir_all(parent)
   611	                    .with_context(|| format!("create dir {}", parent.display()))?;
   612	            }
   613	            std::fs::write(&f.dest_path, &f.contents)
   614	                .with_context(|| format!("write {}", f.dest_path.display()))?;
   615	            if let Some(ft) = f.mtime {
   616	                if let Err(e) = filetime::set_file_mtime(&f.dest_path, ft) {
   617	                    log::warn!("set mtime on {}: {}", f.dest_path.display(), e);
   618	                }
   619	            }
   620	            #[cfg(unix)]
   621	            if let Some(perms) = f.permissions {
   622	                use std::os::unix::fs::PermissionsExt;
   623	                if let Err(e) =
   624	                    std::fs::set_permissions(&f.dest_path, std::fs::Permissions::from_mode(perms))
   625	                {
   626	                    log::warn!("set permissions on {}: {}", f.dest_path.display(), e);
   627	                }
   628	            }
   629	            Ok(f.size)
   630	        })
   631	        .collect();
   632	
   633	    let mut files_written = 0usize;
   634	    let mut bytes_written = 0u64;
   635	    for r in results {
   636	        bytes_written += r?;
   637	        files_written += 1;
   638	    }
   639	
   640	    Ok(SinkOutcome {
   641	        files_written,
   642	        bytes_written,
   643	    })
   644	}
   645	
   646	/// Resume protocol: overwrite a block of an existing file at the given offset.
   647	async fn write_file_block_payload(
   648	    dst_root: &Path,
   649	    canonical_dst_root: Option<&Path>,
   650	    relative_path: &str,
   651	    offset: u64,
   652	    bytes: Vec<u8>,
   653	) -> Result<SinkOutcome> {
   654	    use tokio::io::{AsyncSeekExt, AsyncWriteExt};
   655	
   656	    // R46-F3: contained resolve when canonical root is available.
   657	    let dst = match canonical_dst_root {
   658	        Some(canonical) => {
   659	            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
   660	                .with_context(|| format!("validating block-write path {:?}", relative_path))?
   500	        did_copy = outcome.bytes_transferred > 0;
   501	    } else if crate::copy::file_needs_copy_with_mode(src, dst, config.compare_mode)? {
   502	        let sizer = BufferSizer::default();
   503	        let logger = NoopLogger;
   504	        let outcome = copy_file(src, dst, &sizer, false, &logger)
   505	            .with_context(|| format!("copy {}", header.relative_path))?;
   506	        did_copy = true;
   507	        clone_succeeded = outcome.clone_succeeded;
   508	    }
   509	
   510	    if config.preserve_times && did_copy && !clone_succeeded {
   511	        if let Ok(meta) = std::fs::metadata(src) {
   512	            if let Ok(modified) = meta.modified() {
   513	                let ft = FileTime::from_system_time(modified);
   514	                if let Err(e) = filetime::set_file_mtime(dst, ft) {
   515	                    log::warn!("set mtime on {}: {}", dst.display(), e);
   516	                }
   517	            }
   518	        }
   519	    }
   520	
   521	    Ok(SinkOutcome {
   522	        files_written: 1,
   523	        bytes_written: if did_copy { header.size } else { 0 },
   524	    })
   525	}
   526	
   527	/// Extract an in-memory tar shard to the destination directory.
   528	fn write_tar_shard_payload(
   529	    dst_root: &Path,
   530	    canonical_dst_root: Option<&Path>,
   531	    headers: &[FileHeader],
   532	    data: &[u8],
   533	    config: &FsSinkConfig,
   534	) -> Result<SinkOutcome> {
   535	    if config.dry_run {
   536	        return Ok(SinkOutcome {
   537	            files_written: headers.len(),
   538	            bytes_written: 0,
   539	        });
   540	    }
   541	
   542	    // Two-phase extraction:
   543	    //   1. Validate + parse the tar serially via the shared
   544	    //      `tar_safety` helper. Tar is a sequential format — entries
   545	    //      can't be read in parallel out of one Archive — and this
   546	    //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
   547	    //   2. Write files to disk in parallel via rayon. Inode creation
   548	    //      and write are the bottleneck for many-small-files shards;
   549	    //      4–8 worker cores can saturate ZFS' inode pipeline.
   550	    //
   551	    // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
   552	    // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
   553	    // toward CPU-or-fs limits.
   554	    use rayon::prelude::*;
   555	
   556	    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};
   557	
   558	    let opts = TarShardExtractOptions::default();
   559	    let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;
   560	
   561	    // R47-F1: tar shards arriving on FsTransferSink::write_payload
   562	    // (push-receive on the daemon flows through here too) only had
   563	    // lexical safe_join inside safe_extract_tar_shard. A pre-
   564	    // existing dst/link → /outside escape symlink would let an
   565	    // entry path like `link/victim` write through the symlink.
   566	    // Verify each extracted entry's destination against the
   567	    // canonical root before writing.
   568	    if let Some(canonical) = canonical_dst_root {
   569	        for f in &extracted {
   570	            crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
   571	                format!("tar shard entry {:?} escapes destination root", f.dest_path)
   572	            })?;
   573	        }
   574	    } else {
   575	        log::warn!(
   576	            "write_tar_shard_payload at '{}' has no canonical root; \
   577	             tar-shard receive falls back to lexical-only path \
   578	             checks (R47-F1 escape protection unavailable)",
   579	            dst_root.display()
   580	        );
   581	    }
   582	
   583	    // Honor the sink's preserve_times toggle by stripping mtimes that
   584	    // the helper would otherwise apply. Permissions are best-effort
   585	    // either way (matches the historical FsTransferSink policy).
   586	    if !config.preserve_times {
   587	        for f in &mut extracted {
   588	            f.mtime = None;
   589	        }
   590	    }
   591	
   592	    // Write in parallel. Each closure does its own create_dir_all +
   593	    // fs::write + best-effort mtime/permission application — same
   594	    // policy as `tar_safety::write_extracted_file` but inlined so we
   595	    // can return per-file byte counts for the SinkOutcome.
   596	    let results: Vec<Result<u64>> = extracted
   597	        .into_par_iter()
   598	        .map(|f: ExtractedFile| -> Result<u64> {
   599	            if let Some(parent) = f.dest_path.parent() {
   600	                std::fs::create_dir_all(parent)
   601	                    .with_context(|| format!("create dir {}", parent.display()))?;
   602	            }
   603	            std::fs::write(&f.dest_path, &f.contents)
   604	                .with_context(|| format!("write {}", f.dest_path.display()))?;
   605	            if let Some(ft) = f.mtime {
   606	                if let Err(e) = filetime::set_file_mtime(&f.dest_path, ft) {
   607	                    log::warn!("set mtime on {}: {}", f.dest_path.display(), e);
   608	                }
   609	            }
   610	            #[cfg(unix)]
   611	            if let Some(perms) = f.permissions {
   612	                use std::os::unix::fs::PermissionsExt;
   613	                if let Err(e) =
   614	                    std::fs::set_permissions(&f.dest_path, std::fs::Permissions::from_mode(perms))
   615	                {
   616	                    log::warn!("set permissions on {}: {}", f.dest_path.display(), e);
   617	                }
   618	            }
   619	            Ok(f.size)
   620	        })
   621	        .collect();
   622	
   623	    let mut files_written = 0usize;
   624	    let mut bytes_written = 0u64;
   625	    for r in results {
   626	        bytes_written += r?;
   627	        files_written += 1;
   628	    }
   629	
   630	    Ok(SinkOutcome {
   631	        files_written,
   632	        bytes_written,
   633	    })
   634	}
   635	
   636	/// Resume protocol: overwrite a block of an existing file at the given offset.
   637	async fn write_file_block_payload(
   638	    dst_root: &Path,
   639	    canonical_dst_root: Option<&Path>,
   640	    relative_path: &str,
   420	///
   421	/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
   422	/// where the outbound executor takes a [`TransferSource`] and dispatches
   423	/// payloads round-robin across N sinks, this one consumes a single
   424	/// inbound wire (parsing record headers and producing
   425	/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
   426	/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
   427	/// sequentially. Multi-stream parallelism comes from spawning N invocations,
   428	/// one per inbound TCP connection.
   429	///
   430	/// Both directions converge on `TransferSink::write_payload`: file data
   431	/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
   432	/// which uses the same `receive_stream_double_buffered` helper as the
   433	/// daemon's push receiver and the client's pull receiver — one path,
   434	/// one optimization surface.
   435	pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
   436	    socket: &mut R,
   437	    sink: Arc<dyn TransferSink>,
   438	    progress: Option<&RemoteTransferProgress>,
   439	) -> Result<SinkOutcome> {
   440	    let mut total = SinkOutcome::default();
   441	
   442	    loop {
   443	        let mut tag = [0u8; 1];
   444	        socket
   445	            .read_exact(&mut tag)
   446	            .await
   447	            .context("reading data-plane record tag")?;
   448	
   449	        match tag[0] {
   450	            DATA_PLANE_RECORD_END => break,
   451	            DATA_PLANE_RECORD_FILE => {
   452	                let mut header = read_file_header(socket).await?;
   453	                let file_size = read_u64(socket).await?;
   454	                let mtime = read_i64(socket).await?;
   455	                let perms = read_u32(socket).await?;
   456	                header.size = file_size;
   457	                header.mtime_seconds = mtime;
   458	                header.permissions = perms;
   459	                // Use AsyncReadExt::take to give the sink exactly
   460	                // file_size bytes of the wire. tokio's Take is the
   461	                // canonical way to limit a borrowed AsyncRead.
   462	                use tokio::io::AsyncReadExt;
   463	                let mut reader = (&mut *socket).take(file_size);
   464	                let outcome = sink
   465	                    .write_file_stream(&header, &mut reader)
   466	                    .await
   467	                    .with_context(|| format!("receiving {}", header.relative_path))?;
   468	                if let Some(p) = progress {
   469	                    p.report_payload(0, outcome.bytes_written);
   470	                    p.report_file_complete(header.relative_path.clone());
   471	                }
   472	                total.merge(&outcome);
   473	            }
   474	            DATA_PLANE_RECORD_TAR_SHARD => {
   475	                let (headers, data) = read_tar_shard(socket).await?;
   476	                let bytes = data.len() as u64;
   477	                // Capture member paths for the per-file lane before the
   478	                // payload takes ownership; skip the allocation when no
   479	                // one is listening (the daemon receive path).
   480	                let member_paths: Option<Vec<String>> =
   481	                    progress.map(|_| headers.iter().map(|h| h.relative_path.clone()).collect());
   482	                let payload = PreparedPayload::TarShard { headers, data };
   483	                let outcome = sink
   484	                    .write_payload(payload)
   485	                    .await
   486	                    .context("writing payload")?;
   487	                if let Some(p) = progress {
   488	                    p.report_payload(0, bytes);
   489	                    for path in member_paths.unwrap_or_default() {
   490	                        p.report_file_complete(path);
   491	                    }
   492	                }
   493	                total.merge(&outcome);
   494	            }
   495	            DATA_PLANE_RECORD_BLOCK => {
   496	                let path = read_string(socket).await?;
   497	                // otp-7b-2 (codex G3): once the record names its file,
   498	                // every failure inside it does too.
   499	                let offset = read_u64(socket)
   500	                    .await
   501	                    .context("reading block offset")
   502	                    .map_err(|e| e.wrap_err(super::faulted_path::FaultedPath(path.clone())))?;
   503	                let len = read_u32(socket)
   504	                    .await
   505	                    .context("reading block length")
   506	                    .map_err(|e| e.wrap_err(super::faulted_path::FaultedPath(path.clone())))?
   507	                    as usize;
   508	                if len > MAX_WIRE_BLOCK_BYTES {
   509	                    bail!(
   510	                        "wire block payload {} bytes exceeds max {} (rejecting to avoid OOM)",

exec
/bin/zsh -lc "rg -n \"FsTransferSink::new|with_byte_progress|DestinationInstruments|progress:\" crates/blit-core/src/transfer_session/mod.rs crates/blit-core/src/remote/transfer/session_client.rs crates/blit-daemon/src/service/transfer.rs crates/blit-daemon/src -g '*.rs'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2250,2375p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2700,2785p'; nl -ba crates/blit-core/src/remote/transfer/session_client.rs | sed -n '210,360p'; rg -n \"run_remote_push|PushSessionOptions|DestinationInstruments|destination.*progress\" crates/blit-app crates/blit-daemon crates/blit-core -g '*.rs'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer.rs:24://! (`with_byte_progress`) is not threaded into served-session rows yet —
crates/blit-core/src/transfer_session/mod.rs:206:    pub progress: Option<RemoteTransferProgress>,
crates/blit-core/src/transfer_session/mod.rs:233:    pub instruments: DestinationInstruments,
crates/blit-core/src/transfer_session/mod.rs:250:pub struct DestinationInstruments {
crates/blit-core/src/transfer_session/mod.rs:256:    pub progress: Option<RemoteTransferProgress>,
crates/blit-core/src/transfer_session/mod.rs:262:    pub byte_progress: Option<crate::remote::transfer::ByteProgressSink>,
crates/blit-core/src/transfer_session/mod.rs:1103:    progress: Option<RemoteTransferProgress>,
crates/blit-core/src/transfer_session/mod.rs:2003:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/transfer_session/mod.rs:2118:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/transfer_session/mod.rs:2274:    instruments: DestinationInstruments,
crates/blit-core/src/transfer_session/mod.rs:2371:                DestinationInstruments::default(),
crates/blit-core/src/transfer_session/mod.rs:2562:    instruments: DestinationInstruments,
crates/blit-core/src/transfer_session/mod.rs:2586:            let mut sink = FsTransferSink::new(
crates/blit-core/src/transfer_session/mod.rs:2601:                sink = sink.with_byte_progress(bp);
crates/blit-core/src/transfer_session/mod.rs:3371:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/transfer_session/mod.rs:3459:    progress: Option<&RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/session_client.rs:41:    run_destination, run_source, DestinationInstruments, DestinationOutcome,
crates/blit-core/src/remote/transfer/session_client.rs:85:    pub progress: Option<RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/session_client.rs:104:            progress: None,
crates/blit-core/src/remote/transfer/session_client.rs:177:            progress: options.progress,
crates/blit-core/src/remote/transfer/session_client.rs:236:    pub byte_progress: Option<ByteProgressSink>,
crates/blit-core/src/remote/transfer/session_client.rs:242:    pub progress: Option<RemoteTransferProgress>,
crates/blit-core/src/remote/transfer/session_client.rs:260:            byte_progress: None,
crates/blit-core/src/remote/transfer/session_client.rs:261:            progress: None,
crates/blit-core/src/remote/transfer/session_client.rs:338:        instruments: DestinationInstruments {
crates/blit-core/src/remote/transfer/session_client.rs:339:            progress: options.progress,
crates/blit-core/src/remote/transfer/session_client.rs:340:            byte_progress: options.byte_progress,
crates/blit-daemon/src/service/delegated_session_e2e.rs:21:    delegated_pull_progress::Payload as ProgressPayload, DelegatedPullProgress,
crates/blit-daemon/src/service/transfer.rs:24://! (`with_byte_progress`) is not threaded into served-session rows yet —
crates/blit-daemon/src/service/delegated_pull.rs:20:    delegated_pull_progress::Payload as ProgressPayload, session_error, ComparisonMode,
crates/blit-daemon/src/service/delegated_pull.rs:131:    byte_progress: blit_core::remote::transfer::ByteProgressSink,
crates/blit-daemon/src/service/delegated_pull.rs:171:    byte_progress: &blit_core::remote::transfer::ByteProgressSink,
crates/blit-daemon/src/service/delegated_pull.rs:344:        byte_progress: Some(byte_progress.clone()),
crates/blit-daemon/src/service/delegated_pull.rs:349:        progress: None,
crates/blit-daemon/src/service/transfer_session_e2e.rs:707:    // The resume block phase is provably in progress: the block-diff has
crates/blit-daemon/src/service/transfer_session_e2e.rs:1397:            byte_progress: Some(blit_core::remote::transfer::ByteProgressSink::from_counter(
  2250	            }
  2251	        },
  2252	    };
  2253	
  2254	    drive_destination(
  2255	        &mut transport,
  2256	        negotiated,
  2257	        &dst_root,
  2258	        cfg.data_plane_host.as_deref(),
  2259	        cfg.instruments,
  2260	        cfg.local_apply,
  2261	    )
  2262	    .await
  2263	}
  2264	
  2265	/// The DESTINATION session body: run the diff/receive loop and map a
  2266	/// fault to a peer-notified report. Shared by [`run_destination`] and
  2267	/// [`run_responder`] (the daemon DESTINATION responder), so the receive
  2268	/// choreography is single-sourced.
  2269	async fn drive_destination(
  2270	    transport: &mut FrameTransport,
  2271	    negotiated: Negotiated,
  2272	    dst_root: &Path,
  2273	    data_plane_host: Option<&str>,
  2274	    instruments: DestinationInstruments,
  2275	    local_apply: Option<local::LocalApply>,
  2276	) -> Result<DestinationOutcome> {
  2277	    match destination_session(
  2278	        transport,
  2279	        negotiated,
  2280	        dst_root,
  2281	        data_plane_host,
  2282	        instruments,
  2283	        local_apply,
  2284	    )
  2285	    .await
  2286	    {
  2287	        Ok(outcome) => Ok(outcome),
  2288	        Err(report) => {
  2289	            let mut fault = fault_from_report(report);
  2290	            if !fault.peer_notified {
  2291	                let _ = transport.send(error_frame(&fault)).await;
  2292	                fault.peer_notified = true;
  2293	            }
  2294	            Err(eyre::Report::new(fault))
  2295	        }
  2296	    }
  2297	}
  2298	
  2299	/// Serve one transfer session as the RESPONDER, dispatching on the
  2300	/// initiator's declared role — the daemon's single serving entry
  2301	/// (contract §Invariants 3: one handshake, roles not directions). A
  2302	/// client that declares SOURCE makes this end the DESTINATION
  2303	/// (push-equivalent, otp-4); a client that declares DESTINATION makes
  2304	/// this end the SOURCE (pull-equivalent, otp-5). The two targets carry
  2305	/// the endpoint resolution for each role; only the one the initiator
  2306	/// selects is used. Returns a [`ResponderOutcome`] tagged with the role
  2307	/// that ran.
  2308	pub async fn run_responder(
  2309	    hello: HelloConfig,
  2310	    transport: FrameTransport,
  2311	    source_target: SourceResponderTarget,
  2312	    dest_target: DestinationTarget,
  2313	    // Operator policy from the serving daemon's runtime config
  2314	    // (`--force-grpc-data`, `--no-server-checksums`).
  2315	    policy: ResponderPolicy,
  2316	) -> Result<ResponderOutcome> {
  2317	    let mut transport = transport;
  2318	    exchange_hello(&mut transport, &hello).await?;
  2319	    let open = match expect_frame(&mut transport).await? {
  2320	        Frame::Open(o) => o,
  2321	        other => {
  2322	            return Err(notify_and_wrap(
  2323	                &mut transport,
  2324	                SessionFault::protocol_violation(format!(
  2325	                    "expected SessionOpen, got {}",
  2326	                    frame_name(&Some(other))
  2327	                )),
  2328	            )
  2329	            .await)
  2330	        }
  2331	    };
  2332	    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
  2333	    match declared {
  2334	        // Initiator SOURCE ⇒ this end is DESTINATION (push-equivalent).
  2335	        TransferRole::Source => {
  2336	            let resolve = match &dest_target {
  2337	                DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  2338	                DestinationTarget::Fixed(_) => None,
  2339	            };
  2340	            let negotiated = responder_finish(
  2341	                &mut transport,
  2342	                open,
  2343	                TransferRole::Destination,
  2344	                &destination_open_validator,
  2345	                resolve,
  2346	                &policy,
  2347	            )
  2348	            .await?;
  2349	            let dst_root = match negotiated.resolved_root.clone() {
  2350	                Some(root) => root,
  2351	                None => match &dest_target {
  2352	                    DestinationTarget::Fixed(root) => root.clone(),
  2353	                    DestinationTarget::Resolve(_) => {
  2354	                        return Err(eyre::Report::new(SessionFault::internal(
  2355	                            "resolver target produced no destination root",
  2356	                        )));
  2357	                    }
  2358	                },
  2359	            };
  2360	            // A DESTINATION responder (push) binds+accepts its receive
  2361	            // sockets — it never dials, so it needs no data-plane host.
  2362	            // Served destination (push-equivalent): no instruments — the
  2363	            // serving daemon has no progress line; wiring the daemon
  2364	            // row's byte counter through here is the core.rs jobs-row
  2365	            // follow-up.
  2366	            let outcome = drive_destination(
  2367	                &mut transport,
  2368	                negotiated,
  2369	                &dst_root,
  2370	                None,
  2371	                DestinationInstruments::default(),
  2372	                // The serving daemon never applies locally — the local
  2373	                // carrier exists only inside run_local_session's process.
  2374	                None,
  2375	            )
  2700	            Some(rdp) => {
  2701	                let initial = rdp.initial_streams() as usize;
  2702	                let run = rdp.spawn(recv_sink, progress.clone());
  2703	                let ceiling = run.ceiling;
  2704	                (
  2705	                    Some(data_plane::DestRecvPlane::Responder(run)),
  2706	                    initial,
  2707	                    ceiling,
  2708	                )
  2709	            }
  2710	            // DESTINATION initiator (pull, otp-5b): dial + receive when the
  2711	            // SOURCE responder granted a data plane and we have a host to dial.
  2712	            None => match (&negotiated.accept.data_plane, data_plane_host) {
  2713	                (Some(grant), Some(host)) => {
  2714	                    let initial = grant.initial_streams.max(1) as usize;
  2715	                    let run = data_plane::dial_destination_data_plane(
  2716	                        host,
  2717	                        grant,
  2718	                        recv_sink,
  2719	                        progress.clone(),
  2720	                        instruments.trace_data_plane,
  2721	                    )
  2722	                    .await?;
  2723	                    // otp-5b-2: the pull data plane resizes too. Seed
  2724	                    // `resize_live` from the epoch-0 streams dialed and bound
  2725	                    // growth by the capacity THIS end advertised in its open
  2726	                    // (it is the byte receiver) — the exact ceiling the SOURCE
  2727	                    // responder's dial already clamps to, so both ends agree
  2728	                    // even when the caller advertised a max_streams below this
  2729	                    // host's fresh local reading (codex otp-5b-2 F1). On a
  2730	                    // Resize frame the initiator dials the epoch-N socket (vs
  2731	                    // the responder path's arm).
  2732	                    let ceiling = negotiated
  2733	                        .open
  2734	                        .receiver_capacity
  2735	                        .as_ref()
  2736	                        .map(|c| c.max_streams)
  2737	                        .unwrap_or(0)
  2738	                        .max(1) as usize;
  2739	                    (
  2740	                        Some(data_plane::DestRecvPlane::Initiator(run)),
  2741	                        initial,
  2742	                        ceiling,
  2743	                    )
  2744	                }
  2745	                // A grant with no host to dial is an inconsistent initiator
  2746	                // config: fail fast, mirroring the SOURCE initiator
  2747	                // (`source_send_half`). The SOURCE responder has already bound
  2748	                // and blocks accepting the socket this end would dial, so
  2749	                // silently taking the in-stream branch cannot fall back — it
  2750	                // would deadlock until the responder's accept times out. A
  2751	                // grant means the initiator MUST dial (contract §Transport).
  2752	                // (codex otp-5b-1 finding.)
  2753	                (Some(_), None) => {
  2754	                    return Err(eyre::Report::new(SessionFault::internal(
  2755	                        "responder granted a TCP data plane but this DESTINATION \
  2756	                     initiator has no host to dial",
  2757	                    )))
  2758	                }
  2759	                // No grant (the responder could not bind, or the initiator
  2760	                // asked for in-stream): the in-stream carrier.
  2761	                (None, _) => (None, 0usize, 0usize),
  2762	            },
  2763	        };
  2764	
  2765	    // otp-7a/7b: the DESTINATION chooses the resume block size (plan D5
  2766	    // — it hashes first; the SOURCE reads the size from each
  2767	    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
  2768	    // (D-2026-07-10-1 in-stream, D-2026-07-10-2 data plane) — decided
  2769	    // here, after the carrier is settled.
  2770	    let resume_block_size = {
  2771	        let ceiling = if data_plane_recv.is_some() {
  2772	            MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  2773	        } else {
  2774	            MAX_IN_STREAM_RESUME_BLOCK_SIZE
  2775	        };
  2776	        match negotiated
  2777	            .open
  2778	            .resume
  2779	            .as_ref()
  2780	            .map(|r| r.block_size as usize)
  2781	            .unwrap_or(0)
  2782	        {
  2783	            0 => DEFAULT_BLOCK_SIZE,
  2784	            bs => bs.clamp(MIN_RESUME_BLOCK_SIZE, ceiling),
  2785	        }
   210	    /// Force the in-stream byte carrier instead of the TCP data plane
   211	    /// (otp-5b). Default `false` = the SOURCE responder grants a data
   212	    /// plane and this DESTINATION initiator dials + receives over TCP
   213	    /// sockets; `true` is the diagnostics / unreachable data-plane
   214	    /// fallback. Symmetric with [`PushSessionOptions::in_stream_bytes`].
   215	    pub in_stream_bytes: bool,
   216	    /// otp-7b: negotiate the resume block phase — symmetric with
   217	    /// [`PushSessionOptions::resume`] (plan D6: the flag is in the open,
   218	    /// so resume runs identically whichever end initiated).
   219	    pub resume: bool,
   220	    /// Requested resume block size in bytes; `0` lets the DESTINATION
   221	    /// (this end) choose. Ignored unless `resume` is true.
   222	    pub resume_block_size: u32,
   223	    /// otp-9a: source-side scan filter, riding `SessionOpen.filter`
   224	    /// (the session honors it since otp-6a — this is the client
   225	    /// wiring). `None` scans everything.
   226	    pub filter: Option<FilterSpec>,
   227	    /// otp-9a: mirror on the session (otp-6b's one delete rule — this
   228	    /// DESTINATION diffs the complete source manifest against its tree
   229	    /// at SourceDone and deletes extraneous entries locally). Explicit
   230	    /// enabled + scope per the contract; `MirrorMode::Off` with
   231	    /// `mirror_enabled` set is refused at OPEN.
   232	    pub mirror_enabled: bool,
   233	    pub mirror_kind: MirrorMode,
   234	    /// otp-9a: live counter the session sink reports applied payload
   235	    /// bytes against (the delegated dst daemon's jobs row, otp-9).
   236	    pub byte_progress: Option<ByteProgressSink>,
   237	    /// otp-10b-2: w6-1 progress events from this DESTINATION's receive
   238	    /// side — need batches as the denominator, `Payload`/`FileComplete`
   239	    /// per record received on either carrier. The CLI progress line and
   240	    /// the TUI footer consume these exactly as they did from the old
   241	    /// driver. Symmetric with [`PushSessionOptions::progress`].
   242	    pub progress: Option<RemoteTransferProgress>,
   243	    /// otp-10b-2: emit `[data-plane-client]` connect traces on the data
   244	    /// plane sockets this DESTINATION dials (`--trace-data-plane`).
   245	    pub trace_data_plane: bool,
   246	}
   247	
   248	impl Default for PullSessionOptions {
   249	    fn default() -> Self {
   250	        Self {
   251	            compare_mode: ComparisonMode::SizeMtime,
   252	            ignore_existing: false,
   253	            require_complete_scan: false,
   254	            in_stream_bytes: false,
   255	            resume: false,
   256	            resume_block_size: 0,
   257	            filter: None,
   258	            mirror_enabled: false,
   259	            mirror_kind: MirrorMode::Off,
   260	            byte_progress: None,
   261	            progress: None,
   262	            trace_data_plane: false,
   263	        }
   264	    }
   265	}
   266	
   267	/// Connect to `endpoint`'s daemon and run one DESTINATION-role transfer
   268	/// session pulling the endpoint's module/path tree into `dest_root`
   269	/// (pull-equivalent, otp-5a). The client initiates and declares
   270	/// DESTINATION, so the daemon becomes the SOURCE Responder (streaming
   271	/// its module tree). Returns the [`DestinationOutcome`] this end
   272	/// computed (contract: the DESTINATION is the scorer).
   273	///
   274	/// otp-5b: the default carrier is the TCP data plane — the SOURCE
   275	/// responder binds+grants+accepts sockets while sending, and this
   276	/// DESTINATION initiator dials + receives over them (the transport/role
   277	/// decoupling). `PullSessionOptions::in_stream_bytes` forces the in-stream
   278	/// fallback (diagnostics / unreachable data plane).
   279	pub async fn run_pull_session(
   280	    endpoint: &RemoteEndpoint,
   281	    dest_root: PathBuf,
   282	    options: PullSessionOptions,
   283	) -> Result<DestinationOutcome> {
   284	    let client = connect_transfer_client(endpoint).await?;
   285	    run_pull_session_with_client(client, endpoint, dest_root, options).await
   286	}
   287	
   288	/// [`run_pull_session`] over an already-connected client (otp-9b). The
   289	/// delegated dst daemon connects separately so a connect failure keeps
   290	/// its own error phase (`ConnectSource`) structurally, without string
   291	/// matching on the session error.
   292	pub async fn run_pull_session_with_client(
   293	    mut client: BlitClient<Channel>,
   294	    endpoint: &RemoteEndpoint,
   295	    dest_root: PathBuf,
   296	    options: PullSessionOptions,
   297	) -> Result<DestinationOutcome> {
   298	    let (module, path) = endpoint_module_path(endpoint)?;
   299	
   300	    let open = SessionOpen {
   301	        initiator_role: TransferRole::Destination as i32,
   302	        module,
   303	        path,
   304	        compare_mode: options.compare_mode as i32,
   305	        ignore_existing: options.ignore_existing,
   306	        require_complete_scan: options.require_complete_scan,
   307	        // otp-5b: default to the TCP data plane; the SOURCE responder
   308	        // grants it in SessionAccept unless this asks for the in-stream
   309	        // fallback.
   310	        in_stream_bytes: options.in_stream_bytes,
   311	        // otp-7b: resume rides the open, role-agnostic (plan D6).
   312	        resume: options.resume.then_some(ResumeSettings {
   313	            enabled: true,
   314	            block_size: options.resume_block_size,
   315	        }),
   316	        // otp-9a: filter + mirror ride the open (otp-6a/6b session
   317	        // support; this is the client wiring).
   318	        filter: options.filter,
   319	        mirror_enabled: options.mirror_enabled,
   320	        mirror_kind: options.mirror_kind as i32,
   321	        ..Default::default()
   322	    };
   323	
   324	    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
   325	    let inbound = client
   326	        .transfer(ReceiverStream::new(out_rx))
   327	        .await
   328	        .map_err(|status| eyre::Report::new(transfer_open_refusal(status)))?
   329	        .into_inner();
   330	    let transport = grpc_client_transport(out_tx, inbound);
   331	
   332	    let cfg = DestinationSessionConfig {
   333	        hello: HelloConfig::default(),
   334	        endpoint: SessionEndpoint::initiator(open),
   335	        // The initiator dials the data plane on the same host it reached
   336	        // the control plane on (contract §Transport: initiator dials).
   337	        data_plane_host: Some(endpoint.host.clone()),
   338	        instruments: DestinationInstruments {
   339	            progress: options.progress,
   340	            byte_progress: options.byte_progress,
   341	            trace_data_plane: options.trace_data_plane,
   342	        },
   343	        local_apply: None,
   344	    };
   345	    run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await
   346	}
   347	
   348	/// Derive the wire `(module, path)` from a resolved endpoint. Empty
   349	/// module targets the daemon's default root export; a discovery-form
   350	/// endpoint is not resolvable to a transfer target. The path is
   351	/// POSIX-normalized (codex otp-10a F2): a `rel_path` that went through
   352	/// `PathBuf::join` (the CLI's rsync destination-resolution rule does)
   353	/// carries native `\` separators on Windows, and `to_string_lossy`
   354	/// would put them on the wire verbatim — a Unix daemon then creates a
   355	/// literal `sub\dir` entry. Every wire-bound relative path routes
   356	/// through `path_posix` (the win-1 rule).
   357	fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
   358	    use crate::path_posix::relative_path_to_posix;
   359	    match &endpoint.path {
   360	        RemotePath::Module { module, rel_path } => {
crates/blit-daemon/src/service/transfer_session_e2e.rs:43:    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
crates/blit-daemon/src/service/transfer_session_e2e.rs:327:            async move { run_push_session(&ep, source, PushSessionOptions::default()).await },
crates/blit-daemon/src/service/transfer_session_e2e.rs:395:    let summary = run_push_session(&daemon.endpoint, source, PushSessionOptions::default())
crates/blit-daemon/src/service/transfer_session_e2e.rs:428:        PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:430:            ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:464:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:486:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:507:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:547:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:587:        PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:589:            ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:616:        PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:618:            ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:698:            PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:701:                ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:848:            PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:851:                ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:907:        PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:910:            ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:1017:        PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:1021:            ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:1129:            PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:1131:                ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:1280:        PushSessionOptions::default(),
crates/blit-app/src/transfers/remote.rs:29://! - [`run_remote_push`] + [`PushExecution`] +
crates/blit-app/src/transfers/remote.rs:57:    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
crates/blit-app/src/transfers/remote.rs:67:/// Inputs for [`run_remote_push`]. Primitive fields only — no
crates/blit-app/src/transfers/remote.rs:117:/// Output of [`run_remote_push`]. `summary` is the
crates/blit-app/src/transfers/remote.rs:149:/// let outcome = run_remote_push(execution, handle.as_ref()).await?;
crates/blit-app/src/transfers/remote.rs:158:pub async fn run_remote_push(
crates/blit-app/src/transfers/remote.rs:164:    let options = PushSessionOptions {
crates/blit-app/src/transfers/remote.rs:182:        ..PushSessionOptions::default()
crates/blit-app/src/transfers/remote.rs:248:/// [`run_remote_push`]. Unlike the old pull there is no post-RPC
crates/blit-core/src/remote/transfer/session_client.rs:14://! otp-10a via `blit_app::transfers::remote::run_remote_push`; the
crates/blit-core/src/remote/transfer/session_client.rs:41:    run_destination, run_source, DestinationInstruments, DestinationOutcome,
crates/blit-core/src/remote/transfer/session_client.rs:49:pub struct PushSessionOptions {
crates/blit-core/src/remote/transfer/session_client.rs:91:impl Default for PushSessionOptions {
crates/blit-core/src/remote/transfer/session_client.rs:117:    options: PushSessionOptions,
crates/blit-core/src/remote/transfer/session_client.rs:214:    /// fallback. Symmetric with [`PushSessionOptions::in_stream_bytes`].
crates/blit-core/src/remote/transfer/session_client.rs:217:    /// [`PushSessionOptions::resume`] (plan D6: the flag is in the open,
crates/blit-core/src/remote/transfer/session_client.rs:241:    /// driver. Symmetric with [`PushSessionOptions::progress`].
crates/blit-core/src/remote/transfer/session_client.rs:338:        instruments: DestinationInstruments {
crates/blit-core/src/transfer_session/mod.rs:233:    pub instruments: DestinationInstruments,
crates/blit-core/src/transfer_session/mod.rs:250:pub struct DestinationInstruments {
crates/blit-core/src/transfer_session/mod.rs:2274:    instruments: DestinationInstruments,
crates/blit-core/src/transfer_session/mod.rs:2371:                DestinationInstruments::default(),
crates/blit-core/src/transfer_session/mod.rs:2562:    instruments: DestinationInstruments,
crates/blit-core/src/transfer_session/local.rs:40:    run_destination, run_source, DestinationInstruments, DestinationSessionConfig,
crates/blit-core/src/transfer_session/local.rs:626:        instruments: DestinationInstruments::default(),
crates/blit-core/src/transfer_session/local.rs:899:            instruments: DestinationInstruments::default(),
crates/blit-core/src/transfer_session/local.rs:1236:            instruments: DestinationInstruments::default(),

codex
The benchmark cross-check is resolving cleanly for P1 and the Zoey 0.966 clarification. One wording defect remains in P2: the Windows gRPC-small result was a substantial improvement, not “0.98-ish” parity. I’m now checking whether that imprecision affects the causal method or only the prose.
exec
/bin/zsh -lc "find docs/bench/otp12-win-2026-07-12 docs/bench/otp12-zoey-2026-07-12 docs/bench/otp2w-baseline-2026-07-10 -maxdepth 2 -type f -print | sort; nl -ba docs/bench/otp12-win-2026-07-12/README.md | sed -n '1,180p'; nl -ba docs/bench/otp12-win-2026-07-12/verdicts.csv | sed -n '1,90p'; nl -ba docs/bench/otp12-win-2026-07-12/summary.csv | sed -n '1,90p'; nl -ba docs/bench/otp12-zoey-2026-07-12/README.md | sed -n '1,150p'; nl -ba docs/bench/otp12-zoey-2026-07-12/verdicts.csv | sed -n '1,80p'; nl -ba docs/bench/otp12-zoey-2026-07-12/escalation-verdicts.csv | sed -n '1,60p'; nl -ba docs/bench/otp12-zoey-2026-07-12/escalation-summary.csv | sed -n '1,50p'; nl -ba docs/bench/otp2w-baseline-2026-07-10/summary.csv | sed -n '1,60p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/bench/otp12-win-2026-07-12/README.md
docs/bench/otp12-win-2026-07-12/drain-outcomes.txt
docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv
docs/bench/otp12-win-2026-07-12/runs.csv
docs/bench/otp12-win-2026-07-12/staging-manifest.txt
docs/bench/otp12-win-2026-07-12/summary.csv
docs/bench/otp12-win-2026-07-12/verdicts.csv
docs/bench/otp12-zoey-2026-07-12/README.md
docs/bench/otp12-zoey-2026-07-12/aborted-storm-drain.txt
docs/bench/otp12-zoey-2026-07-12/aborted-storm-runs.csv
docs/bench/otp12-zoey-2026-07-12/drain-outcomes.txt
docs/bench/otp12-zoey-2026-07-12/escalation-drain-outcomes.txt
docs/bench/otp12-zoey-2026-07-12/escalation-runs.csv
docs/bench/otp12-zoey-2026-07-12/escalation-staging-manifest.txt
docs/bench/otp12-zoey-2026-07-12/escalation-summary.csv
docs/bench/otp12-zoey-2026-07-12/escalation-verdicts.csv
docs/bench/otp12-zoey-2026-07-12/runs.csv
docs/bench/otp12-zoey-2026-07-12/staging-manifest.txt
docs/bench/otp12-zoey-2026-07-12/summary.csv
docs/bench/otp12-zoey-2026-07-12/verdicts.csv
docs/bench/otp2w-baseline-2026-07-10/README.md
docs/bench/otp2w-baseline-2026-07-10/drain-outcomes.txt
docs/bench/otp2w-baseline-2026-07-10/probe1-sshoverhead-runs.csv
docs/bench/otp2w-baseline-2026-07-10/probe1-sshoverhead-summary.csv
docs/bench/otp2w-baseline-2026-07-10/runs.csv
docs/bench/otp2w-baseline-2026-07-10/summary.csv
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
    41	Combined outcomes (`verdicts.csv` carries per-reference rows):
    42	PASS everywhere except —
    43	
    44	| cell | new | old same-session | ratio | committed | ratio | outcome |
    45	|------|----:|----:|----:|----:|----:|---------|
    46	| push_tcp_small | 2080 | 1811 | **1.149** | 1868 | 1.113 | FAIL-BOTH (spreads 3.8/3.0% — real) |
    47	| pull_tcp_mixed | 1138 | 867 | **1.313** | 1284 | 0.886 | FAIL-SAME-SESSION (spreads 5.2/6.7%) |
    48	
    49	No pre-registered escalation trigger fires (no straddle with >25%
    50	spread — these are tight-spread results); both stand recorded for the
    51	otp-13 walk. Rig context: today's old arms run far FASTER than their
    52	2026-07-10 committed medians (e.g. old pull_tcp_mixed 867 vs 1284, old
    53	push_tcp_large 1908 vs 3054) — reference drift in the fast direction,
    54	so the committed bars are easy and the same-session bars are the
    55	binding ones.
    56	
    57	## Block 2 — initiator/verb invariance (new pair): 11/12 PASS
    58	
    59	The owner's sentence, measured: per direction × fixture × carrier,
    60	`max(mac_init, win_init)/min ≤ 1.10`. Eleven cells PASS at ratios
    61	1.003–1.057. The exception:
    62	
    63	- **wm_tcp_mixed FAIL at 1.237** (mac_init 1127 vs win_init 911, tight
    64	  spreads 8.2/3.3%): Win→Mac mixed over the TCP data plane is ~25%
    65	  slower when the MAC initiates (pull-verb, destination role) than when
    66	  Windows initiates (push-verb, source role). Independently
    67	  corroborated by block 1 (`pull_tcp_mixed` new 1138 vs old 867) and
    68	  NOT present on grpc (wm_grpc_mixed 1.013) or other fixtures (large
    69	  1.023, small 1.011) — the signature is specifically
    70	  TCP-carrier × mixed workload × destination-initiator. A
    71	  code-shaped finding for the otp-13 walk (and the exact class of
    72	  defect this criterion exists to catch).
    73	
    74	## Cross-direction (F4 + the D-2026-07-12-1 discriminator)
    75	
    76	- **Win→Mac: all six cells PASS** — the unified path beats even the
    77	  better committed old direction (cross-row ratios 0.760–0.990).
    78	- **Mac→Win: all six cells FAIL** `min(old_push, old_pull) × 1.10`.
    79	  What the gap rows RECORD (adjudication is the otp-13 walk's — this
    80	  README draws no criterion conclusion, codex otp-12b-run F1):
    81	  - **large (tcp 1.979 → 1.951; grpc 1.956 → 1.945): unchanged** —
    82	    exactly the D-2026-07-12-1 discriminator shape (the old arm shows
    83	    the same direction gap in the same session).
    84	  - **mixed (1.946 → 1.408) and grpc_small (1.929 → 1.644): NARROWED**
    85	    — the unified path closed part of the old gap (code improvement);
    86	    a residual gap remains above the bar, and how much of that residue
    87	    is platform is the owner's call with these numbers.
    88	  - **tcp_small: WIDENED (1.332 → 1.527)** — tracks the
    89	    push_tcp_small code gap above; that cell's cross miss is NOT
    90	    fully platform-attributable.
    91	
    92	## Cross-block consistency note
    93	
    94	`push_tcp_small` (block 1 new arm) measured 2080 while `mw_tcp_small`
    95	mac_init (block 2, nominally the same work) measured 1922 — 8% apart in
    96	one session. Block-2 arms use precreated destination containers (design
    97	F5) where block 1 keeps the otp-2w shapes; the delta is recorded here
    98	rather than adjudicated.
    99	
   100	## Reproduction
   101	
   102	```
   103	export WIN_SSH=michael@netwatch-01 WIN_HOST=10.1.10.177
   104	export MAC_HOST=<mac 10GbE ip>  OLD_CLIENT_PROVENANCE_BY_BUILD=1
   105	RUNS=4 ./scripts/bench_otp12_win.sh
   106	PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
   107	```
   108	
   109	Staging per the harness header (aside-copy the old exes BEFORE moving
   110	the checkout; bundle + native build; sha-named bins; the daemons launch
   111	from `bins\active\` under the one `blit-otp12-daemon` firewall rule).
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
     1	# otp-12a — unified-path vs OLD-path interleaved A/B on the Mac↔zoey rig (2026-07-12)
     2	
     3	**Status**: Recorded. **Scope (load-bearing)**: rig Z anchors
     4	**per-direction converge-up only** (hardware-asymmetric endpoints,
     5	D-2026-07-05-1; `docs/bench/otp2-baseline-2026-07-10/README.md` §Status).
     6	Cross-direction and initiator/verb-invariance claims belong to rig W
     7	(otp-12b). **This README declares nothing** — pass/fail is the owner's
     8	at the otp-13 walk (design doc `docs/plan/OTP12_ACCEPTANCE_RUN.md`,
     9	Governs); it records the computed D2 comparisons.
    10	
    11	**Harness**: `scripts/bench_otp12_zoey.sh` (methodology inherited from
    12	the frozen `bench_otp2_baseline.sh`; new mechanics — ABBA counterbalance,
    13	pair-void valid-run rule, both-reference verdicts — per the design doc
    14	D1/D2/D5). RUNS=4 main session, RUNS=8 escalation (the pre-registered D2
    15	rule). Zero voided pairs in any recorded session.
    16	
    17	## Builds (matched pairs, clean trees, sha-embedded — manifests committed)
    18	
    19	- **old arm**: clean `e757dcc` rebuilds BOTH ends (Mac client via
    20	  detached worktree; zoey daemon `cargo zigbuild --release --target
    21	  aarch64-unknown-linux-musl`, staged as `blit-temp/blit-daemon-e757dcc`).
    22	  **Provenance correction found en route**: the 2026-07-10 staging at
    23	  `blit-temp/blit-daemon` embeds `731023bfc8a1.dirty.…`, NOT `e757dcc`
    24	  as the otp-2 README claimed (correction note committed there,
    25	  `b2b6901`); that artifact was left untouched and NOT used here.
    26	- **new arm**: the run commit both ends — `042c06f` (main session),
    27	  `6bc9cb6` (escalation; the inter-session diff is harness-script-only).
    28	  Zero `crates/**`/`proto/**` changes exist anywhere in otp-12: the
    29	  transfer code both sessions is exactly the plan's post-otp-11 HEAD
    30	  (`ce36da3` lineage), suite 1484.
    31	- sha256 of every binary + the committed reference CSV:
    32	  `staging-manifest.txt` / `escalation-staging-manifest.txt`.
    33	- **Old-client provenance basis (codex otp-12a-run F1)**: pre-cutover
    34	  client binaries embed no greppable build id (the daemon does; the
    35	  client's only bare-sha match is a cargo-embedded build-directory
    36	  path), so the harness's binary check cannot establish the old
    37	  client post-hoc. Its provenance here is the build procedure — a
    38	  clean detached worktree at `e757dcc` built in the recorded session —
    39	  pinned by the manifest sha256. The harness now requires the `+<sha>`
    40	  id form where it exists and an explicit
    41	  `OLD_CLIENT_PROVENANCE_BY_BUILD=1` acknowledgment where it cannot.
    42	
    43	## Sessions
    44	
    45	1. **Aborted storm session** (`aborted-storm-runs.csv`, 12 runs kept):
    46	   zoey degraded progressively — load average 1.4 → 444, run times ~10×
    47	   the committed baseline, BOTH arms equally, drains still "passing."
    48	   Root cause consistent with accumulated per-run push destinations
    49	   (~15 GiB) congesting the pool write path: after stopping, load fell
    50	   within minutes; three back-to-back probes WITH per-run deletion held
    51	   at baseline (2466/2525/3714 ms vs committed 2702). Harness now sweeps
    52	   each destination right after its flush is measured (outside the timed
    53	   window). No data from this session feeds any verdict.
    54	2. **Main session** (RUNS=4; `runs.csv`/`summary.csv`/`verdicts.csv`):
    55	   full 12-comparison matrix, 48 pairs, all valid. 9/12 PASS both
    56	   references; 3 escalated per the pre-registered D2 rules.
    57	3. **Escalation session** (RUNS=8, `CELLS` allowlist;
    58	   `escalation-*.csv`): the three flagged comparisons re-run fresh.
    59	
    60	## Final per-comparison state (escalation supersedes where run — D2)
    61	
    62	| comparison | new ms | old same-session | ratio | committed | ratio | combined |
    63	|------------|-------:|-----------------:|------:|----------:|------:|----------|
    64	| push_tcp_large  | 2464 | 2570 | 0.959 | 2702 | 0.912 | **PASS** (RUNS=8 governs per the D2 supersession rule, recorded as a dated amendment after this run surfaced the gap — codex otp-12a-run F2; the RUNS=4 session read FAIL-BOTH at 100% new-arm spread and stays committed in `runs.csv`) |
    65	| push_grpc_large | 4567 | 4369 | 1.045 | 4510 | 1.013 | **PASS** |
    66	| pull_tcp_large  | 2167 | 2177 | 0.995 | 1744 | 1.243 | **FAIL-REFERENCE-DRIFT** (persisted at RUNS=8; see Drift) |
    67	| pull_grpc_large | 2702 | 2706 | 0.999 | 2585 | 1.045 | **PASS** |
    68	| push_tcp_small  | 3984 | 3605 | 1.105 | 4263 | 0.935 | **FAIL-SAME-SESSION** (persisted; see the marginal-gap note) |
    69	| push_grpc_small | 4731 | 4727 | 1.001 | 5217 | 0.907 | **PASS** |
    70	| pull_tcp_small  | 2277 | 2266 | 1.005 | 2784 | 0.818 | **PASS** |
    71	| pull_grpc_small | 3148 | 3463 | 0.909 | 4188 | 0.752 | **PASS** |
    72	| push_tcp_mixed  | 2142 | 2053 | 1.043 | 2070 | 1.035 | **PASS** |
    73	| push_grpc_mixed | 3468 | 3666 | 0.946 | 3889 | 0.892 | **PASS** |
    74	| pull_tcp_mixed  | 1521 | 1575 | 0.966 | 1401 | 1.086 | **PASS** |
    75	| pull_grpc_mixed | 2107 | 2252 | 0.936 | 2222 | 0.948 | **PASS** |
    76	
    77	Rollup: **10 PASS, 1 FAIL-REFERENCE-DRIFT, 1 FAIL-SAME-SESSION** — both
    78	non-PASS cells carried to the otp-13 walk with the analysis below.
    79	
    80	## Drift analysis (pull_tcp_large)
    81	
    82	The strongest available evidence puts the drift rig-side, not the
    83	unified path's: the OLD arm ran 2177 ms median this session vs the
    84	committed 1744 ms (**1.248×**), while new-vs-old same-session is
    85	**0.995** — whatever slowed large pulls slowed both arms alike, and the
    86	unified path is not slower than the old path on this rig, this day.
    87	This is correlation plus same-session parity, not proof (codex
    88	otp-12a-run F3): the committed baseline's daemon was itself the
    89	mislabeled dirty build (see Builds), so a code-content confound in the
    90	reference cannot be fully excluded, and the rig changed between
    91	2026-07-10 and 2026-07-12 (uptime 22 days; owner-side maintenance
    92	touched the box on 07-11). Per D2 a persisting drift stands recorded,
    93	never silently excused.
    94	
    95	## The marginal same-session gap (push_tcp_small)
    96	
    97	Reproducible across both sessions (1.109 at RUNS=4, **1.105** at RUNS=8
    98	with tight spreads: new 16.7%, old 18.7%) — a real ≈10.5% same-session
    99	gap, 0.5% over the ±10% noise bar, on this cell only. Context the walk
   100	needs (stated per the CSVs, codex otp-12a-run F4): the OLD arm ran
   101	15.4% faster this session than its own committed baseline (3605 vs
   102	4263); the unified path still beats that committed baseline by 6.5%
   103	(3984 vs 4263) but sits 10.5% behind the faster same-session old arm.
   104	The neighboring small/mixed same-session ratios for reference:
   105	pull_grpc_small 0.909, push_grpc_small 1.001, pull_tcp_small 1.005,
   106	push_tcp_mixed 1.043. If a per-cell look is wanted, it is a post-otp-12
   107	item; nothing here blocks otp-12b/c mechanically.
   108	
   109	## Reproduction
   110	
   111	```
   112	export ZOEY_SSH=root@zoey
   113	export ZOEY_TEMP=/volume/<pool-uuid>/.srv/.unifi-drive/michael/.data/blit-temp
   114	export ZOEY_HOST=10.1.10.206
   115	RUNS=4 ./scripts/bench_otp12_zoey.sh                     # full matrix
   116	CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_zoey.sh  # D2 escalation
   117	PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh           # checks only
   118	```
   119	
   120	Requires: clean tree at the run commit; old client staged at
   121	`~/blit-bench-work/bins/blit-e757dcc`; both sha-named daemons staged in
   122	`blit-temp/`; python3 + NOPASSWD purge on the Mac. The staged 2026-07-10
   123	`blit-temp/blit-daemon` (dirty-`731023b`) is an otp-2 artifact — never
   124	run it for otp-12 arms.
     1	comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
     2	pull_grpc_large,converge,new,old_session,2702,2706,0.999,1.10,PASS
     3	pull_grpc_large,converge,new,old_committed,2702,2585,1.045,1.10,PASS
     4	pull_grpc_large,converge,new,combined,2702,,,1.10,PASS
     5	pull_grpc_mixed,converge,new,old_session,2107,2252,0.936,1.10,PASS
     6	pull_grpc_mixed,converge,new,old_committed,2107,2222,0.948,1.10,PASS
     7	pull_grpc_mixed,converge,new,combined,2107,,,1.10,PASS
     8	pull_grpc_small,converge,new,old_session,3148,3463,0.909,1.10,PASS
     9	pull_grpc_small,converge,new,old_committed,3148,4188,0.752,1.10,PASS
    10	pull_grpc_small,converge,new,combined,3148,,,1.10,PASS
    11	pull_tcp_large,converge,new,old_session,1958,2093,0.935,1.10,PASS
    12	pull_tcp_large,converge,new,old_committed,1958,1744,1.123,1.10,FAIL
    13	pull_tcp_large,converge,new,combined,1958,,,1.10,FAIL-REFERENCE-DRIFT
    14	pull_tcp_mixed,converge,new,old_session,1521,1575,0.966,1.10,PASS
    15	pull_tcp_mixed,converge,new,old_committed,1521,1401,1.086,1.10,PASS
    16	pull_tcp_mixed,converge,new,combined,1521,,,1.10,PASS
    17	pull_tcp_small,converge,new,old_session,2277,2266,1.005,1.10,PASS
    18	pull_tcp_small,converge,new,old_committed,2277,2784,0.818,1.10,PASS
    19	pull_tcp_small,converge,new,combined,2277,,,1.10,PASS
    20	push_grpc_large,converge,new,old_session,4567,4369,1.045,1.10,PASS
    21	push_grpc_large,converge,new,old_committed,4567,4510,1.013,1.10,PASS
    22	push_grpc_large,converge,new,combined,4567,,,1.10,PASS
    23	push_grpc_mixed,converge,new,old_session,3468,3666,0.946,1.10,PASS
    24	push_grpc_mixed,converge,new,old_committed,3468,3889,0.892,1.10,PASS
    25	push_grpc_mixed,converge,new,combined,3468,,,1.10,PASS
    26	push_grpc_small,converge,new,old_session,4731,4727,1.001,1.10,PASS
    27	push_grpc_small,converge,new,old_committed,4731,5217,0.907,1.10,PASS
    28	push_grpc_small,converge,new,combined,4731,,,1.10,PASS
    29	push_tcp_large,converge,new,old_session,3536,2418,1.462,1.10,FAIL
    30	push_tcp_large,converge,new,old_committed,3536,2702,1.309,1.10,FAIL
    31	push_tcp_large,converge,new,combined,3536,,,1.10,FAIL-BOTH
    32	push_tcp_mixed,converge,new,old_session,2142,2053,1.043,1.10,PASS
    33	push_tcp_mixed,converge,new,old_committed,2142,2070,1.035,1.10,PASS
    34	push_tcp_mixed,converge,new,combined,2142,,,1.10,PASS
    35	push_tcp_small,converge,new,old_session,4033,3636,1.109,1.10,FAIL
    36	push_tcp_small,converge,new,old_committed,4033,4263,0.946,1.10,PASS
    37	push_tcp_small,converge,new,combined,4033,,,1.10,FAIL-SAME-SESSION
     1	comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
     2	pull_tcp_large,converge,new,old_session,2167,2177,0.995,1.10,PASS
     3	pull_tcp_large,converge,new,old_committed,2167,1744,1.243,1.10,FAIL
     4	pull_tcp_large,converge,new,combined,2167,,,1.10,FAIL-REFERENCE-DRIFT
     5	push_tcp_large,converge,new,old_session,2464,2570,0.959,1.10,PASS
     6	push_tcp_large,converge,new,old_committed,2464,2702,0.912,1.10,PASS
     7	push_tcp_large,converge,new,combined,2464,,,1.10,PASS
     8	push_tcp_small,converge,new,old_session,3984,3605,1.105,1.10,FAIL
     9	push_tcp_small,converge,new,old_committed,3984,4263,0.935,1.10,PASS
    10	push_tcp_small,converge,new,combined,3984,,,1.10,FAIL-SAME-SESSION
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	pull_tcp_large,new,2167,2442,1885,132.9,0,8
     3	pull_tcp_large,old,2177,2111,1809,27.9,0,8
     4	push_tcp_large,new,2464,2479,2199,20.9,0,8
     5	push_tcp_large,old,2570,2645,2244,60.6,0,8
     6	push_tcp_small,new,3984,4062,3853,16.7,0,8
     7	push_tcp_small,old,3605,3685,3487,18.7,0,8
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
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '940,1015p'; nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '180,380p'; nl -ba crates/blit-core/src/transfer_plan.rs | sed -n '80,180p'; git show 0f922de:crates/blit-core/src/remote/push/client/mod.rs | nl -ba | sed -n '990,1195p'; git show 0f922de:crates/blit-daemon/src/service/push/control.rs | nl -ba | sed -n '190,340p'; git show 0f922de:crates/blit-daemon/src/service/push/control.rs | nl -ba | sed -n '740,830p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   940	                .await
   941	            }
   942	        };
   943	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   944	            session,
   945	            Arc::clone(&self.source),
   946	            PathBuf::new(),
   947	        ));
   948	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   949	            if let SinkControl::Add(sink) = returned.0 {
   950	                let _ = sink.finish().await;
   951	            }
   952	        }
   953	        Ok(())
   954	    }
   955	
   956	    /// Feed one planned batch into the send pipeline. The pipeline
   957	    /// prepares each payload (tar-shard/file) and writes it through the
   958	    /// data-plane record framing across the live socket(s).
   959	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   960	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   961	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   962	        })?;
   963	        for payload in payloads {
   964	            tx.send(payload).await.map_err(|_| {
   965	                dp_fault("data-plane send pipeline closed before all payloads sent")
   966	            })?;
   967	        }
   968	        Ok(())
   969	    }
   970	
   971	    /// Signal end-of-stream, drain the pipeline (each worker emits its
   972	    /// socket's END record on drain), and return the bytes sent. Must be
   973	    /// awaited before `SourceDone` goes out so the destination's receive
   974	    /// pipeline sees END and completes.
   975	    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
   976	        // Drop the sender: workers observe the closed queue, drain what
   977	        // is left, then `finish()` (END record) and exit.
   978	        self.payload_tx = None;
   979	        let pipeline = self
   980	            .pipeline
   981	            .take()
   982	            .expect("SourceDataPlane::finish called once");
   983	        pipeline
   984	            .join()
   985	            .await
   986	            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
   987	    }
   988	}
   989	
   990	// ---------------------------------------------------------------------------
   991	// Need-list enforcement for the data-plane receive
   992	// ---------------------------------------------------------------------------
   993	
   994	/// Sink decorator that enforces the session's need-list contract on the
   995	/// data-plane receive, giving it the SAME strictness the in-stream
   996	/// carrier applies inline in the control loop (`outstanding.remove`).
   997	/// `execute_receive_pipeline` writes socket-provided paths directly, so
   998	/// without this a peer could substitute an off-need-list path for a
   999	/// needed one (count-preserving), duplicate one, or send resume block
  1000	/// records the session never negotiated (codex otp-4b-1 F1). Every
  1001	/// written path must be a granted, not-yet-received need. Resume
  1002	/// sessions (otp-7b) additionally validate + claim block records
  1003	/// against the shared [`ResumeHeaders`] grant map — with the identical
  1004	/// strictness the in-stream `claim_resume_record` applies — and count
  1005	/// completions into the shared resumed counter; in a non-resume session
  1006	/// block records are rejected outright. The shared [`OutstandingNeeds`]
  1007	/// set makes completion `is_empty()` for both carriers.
  1008	pub(super) struct NeedListSink {
  1009	    inner: Arc<dyn TransferSink>,
  1010	    outstanding: OutstandingNeeds,
  1011	    /// `Some` iff the session negotiated resume (otp-7b): the shared
  1012	    /// grant map + resumed counter block records are validated and
  1013	    /// claimed against. `None` ⇒ any block record is a violation.
  1014	    resume: Option<ResumeRecv>,
  1015	}
   180	                    // ue-r2-2: a retired worker stops at the same payload
   181	                    // boundary; queued payloads stay in the shared queue
   182	                    // for the survivors (dequeue = ownership, so
   183	                    // exactly-once is preserved — flume's RecvFut only
   184	                    // takes an item when it resolves, so racing it is
   185	                    // safe). The watch (not a flag) also frees a worker
   186	                    // parked on an IDLE queue. Its `finish()` below emits
   187	                    // the per-stream END record — the receiver-side
   188	                    // teardown signal.
   189	                    let payload = tokio::select! {
   190	                        biased;
   191	                        _ = retire.changed() => break,
   192	                        recv = work_rx.recv_async() => match recv {
   193	                            Ok(p) => p,
   194	                            Err(_) => break, // queue closed and drained
   195	                        },
   196	                    };
   197	                    let prepared = source
   198	                        .prepare_payload(payload)
   199	                        .await
   200	                        .context("preparing payload")?;
   201	                    let files: Vec<(String, u64)> = match &prepared {
   202	                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   203	                        PreparedPayload::TarShard { headers, .. } => headers
   204	                            .iter()
   205	                            .map(|h| (h.relative_path.clone(), h.size))
   206	                            .collect(),
   207	                        // Raw resume-block payloads patch existing files;
   208	                        // no file-completion event from one-block-at-a-
   209	                        // time. The composite ResumeFile IS one whole
   210	                        // file's phase — reported below from the outcome,
   211	                        // because its byte count (stale blocks only) is
   212	                        // known only after the write (codex otp-10a F6).
   213	                        PreparedPayload::FileBlock { .. }
   214	                        | PreparedPayload::FileBlockComplete { .. }
   215	                        | PreparedPayload::ResumeFile { .. } => Vec::new(),
   216	                    };
   217	                    let resumed_file: Option<String> = match &prepared {
   218	                        PreparedPayload::ResumeFile { header, .. } => {
   219	                            Some(header.relative_path.clone())
   220	                        }
   221	                        _ => None,
   222	                    };
   223	                    let outcome = sink
   224	                        .write_payload(prepared)
   225	                        .await
   226	                        .context("writing payload")?;
   227	                    if let Some(p) = &progress {
   228	                        // Contract (progress.rs): bytes ride Payload, one
   229	                        // FileComplete per file. `size` is the planned
   230	                        // manifest size — the value this lane has always
   231	                        // reported, now on the right variant.
   232	                        for (name, size) in &files {
   233	                            p.report_payload(0, *size);
   234	                            p.report_file_complete(name.clone());
   235	                        }
   236	                        // A resumed file finishes like any other (w6-1:
   237	                        // counted once, per-file lane); its bytes are the
   238	                        // stale blocks actually sent.
   239	                        if let Some(name) = resumed_file {
   240	                            p.report_payload(0, outcome.bytes_written);
   241	                            p.report_file_complete(name);
   242	                        }
   243	                    }
   244	                    let mut t = total.lock().unwrap();
   245	                    t.merge(&outcome);
   246	                }
   247	                sink.finish().await?;
   248	                Ok::<(), eyre::Report>(())
   249	            }
   250	            .await;
   251	            if run.is_err() {
   252	                // Signal the forwarder (and implicitly the other workers,
   253	                // once the queue closes) to stop feeding new work.
   254	                cancelled.store(true, Ordering::Relaxed);
   255	            }
   256	            (slot, run)
   257	        });
   258	    }
   259	
   260	    for sink in sinks {
   261	        let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
   262	        let slot = next_slot;
   263	        next_slot += 1;
   264	        retire_flags.push((slot, retire_tx));
   265	        spawn_sink_worker(
   266	            &mut join_set,
   267	            slot,
   268	            sink,
   269	            work_rx.clone(),
   270	            source.clone(),
   271	            progress.cloned(),
   272	            total.clone(),
   273	            cancelled.clone(),
   274	            retire_rx,
   275	        );
   276	    }
   277	
   278	    // Forwarder: move payloads from the incoming channel onto the shared
   279	    // work queue. `send_async` applies back-pressure (bounded queue); if
   280	    // every worker has gone away (e.g. all sinks errored) the send fails
   281	    // and we stop. It also bails as soon as a worker sets `cancelled`, so
   282	    // a single sink error halts intake promptly instead of waiting for
   283	    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
   284	    // cancel) signals the workers. (The executor keeps a `work_rx` clone
   285	    // for late-added workers — flume disconnect is sender-driven, so the
   286	    // retained receiver does not keep the queue alive.)
   287	    let cancelled_fwd = cancelled.clone();
   288	    let forwarder = tokio::spawn(async move {
   289	        while let Some(payload) = payload_rx.recv().await {
   290	            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
   291	                // A worker errored — stop draining the producer and let
   292	                // the queue close so survivors finish and the error
   293	                // surfaces without delay.
   294	                return;
   295	            }
   296	            if work_tx.send_async(payload).await.is_err() {
   297	                // All workers dropped their receivers — nothing left to
   298	                // feed; treat as shutdown.
   299	                return;
   300	            }
   301	        }
   302	        // Dropping work_tx closes the queue → workers see Disconnected
   303	        // after draining and run finish().
   304	    });
   305	
   306	    // Supervise: join workers (first error wins) while servicing the
   307	    // resize control channel. `join_next() == None` means every worker
   308	    // — initial and added — has finished, which only happens once the
   309	    // queue closed and drained (or errored/retired), so control is
   310	    // moot beyond that point.
   311	    let mut control_rx = control_rx;
   312	    let mut first_err: Option<eyre::Report> = None;
   313	    loop {
   314	        let control_recv = async {
   315	            match control_rx.as_mut() {
   316	                Some(rx) => rx.recv().await,
   317	                None => std::future::pending().await,
   318	            }
   319	        };
   320	        tokio::select! {
   321	            // ue-r2-2 review (panel F2): biased, control FIRST — a
   322	            // ready Add must be processed before the join arm can
   323	            // observe an empty set and break, or an already-authorized
   324	            // socket would drop without its END record (fatal on the
   325	            // peer). Processing a control command is always cheap and
   326	            // never starves joins.
   327	            biased;
   328	
   329	            cmd = control_recv => {
   330	                match cmd {
   331	                    Some(SinkControl::Add(sink)) => {
   332	                        if !cancelled.load(Ordering::Relaxed) {
   333	                            let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
   334	                            let slot = next_slot;
   335	                            next_slot += 1;
   336	                            retire_flags.push((slot, retire_tx));
   337	                            spawn_sink_worker(
   338	                                &mut join_set,
   339	                                slot,
   340	                                sink,
   341	                                work_rx.clone(),
   342	                                source.clone(),
   343	                                progress.cloned(),
   344	                                total.clone(),
   345	                                cancelled.clone(),
   346	                                retire_rx,
   347	                            );
   348	                        }
   349	                        // On a failing transfer the added sink is dropped
   350	                        // unused; its socket closes and the peer's worker
   351	                        // errors into the already-failing teardown.
   352	                    }
   353	                    Some(SinkControl::RetireOne) => {
   354	                        // Floor at one live worker (see SinkControl docs).
   355	                        if retire_flags.len() > 1 {
   356	                            if let Some((_, retire_tx)) = retire_flags.pop() {
   357	                                let _ = retire_tx.send(true);
   358	                            }
   359	                        }
   360	                    }
   361	                    None => control_rx = None, // controller gone; keep draining
   362	                }
   363	            }
   364	            joined = join_set.join_next() => {
   365	                match joined {
   366	                    None => break,
   367	                    Some(Ok((slot, res))) => {
   368	                        retire_flags.retain(|(s, _)| *s != slot);
   369	                        if let Err(e) = res {
   370	                            if first_err.is_none() {
   371	                                first_err = Some(e);
   372	                            }
   373	                        }
   374	                    }
   375	                    Some(Err(join)) => {
   376	                        if first_err.is_none() {
   377	                            first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
   378	                        }
   379	                    }
   380	                }
    80	            // Large: schedule as single large-file task; range/delta decided when sending
    81	            large_files.push(TransferTask::Large { path: rel.clone() });
    82	        }
    83	    }
    84	    // Shard small files into larger tars for multi-GB workloads
    85	    small.sort_by_key(|p| p.as_os_str().len());
    86	
    87	    let mut small_tasks: Vec<TransferTask> = Vec::new();
    88	    let small_count = small.len();
    89	    let total_small_bytes: u64 = small.iter().fold(0u64, |acc, p| {
    90	        acc.saturating_add(*size_map.get(p).unwrap_or(&(64 * 1024)))
    91	    });
    92	    let avg_small_size = if small_count == 0 {
    93	        0
    94	    } else {
    95	        total_small_bytes / small_count as u64
    96	    };
    97	
    98	    // Tar shards only make sense for 2+ files (batching) — a single file
    99	    // gains nothing from tar wrapping and breaks the empty-path case
   100	    // produced by enumerating a file root directly.
   101	    let use_tar = if options.force_tar {
   102	        small_count >= 1
   103	    } else if small_count < 2 {
   104	        false
   105	    } else {
   106	        small_count >= 32 || avg_small_size <= 128 * 1024
   107	    };
   108	
   109	    if use_tar {
   110	        let mut target_shard = options.small_target.unwrap_or(8 * 1024 * 1024);
   111	        if total_small_bytes >= 768 * 1024 * 1024 {
   112	            target_shard = target_shard.max(64 * 1024 * 1024);
   113	        } else if total_small_bytes >= 256 * 1024 * 1024 {
   114	            target_shard = target_shard.max(32 * 1024 * 1024);
   115	        } else {
   116	            target_shard = target_shard.max(4 * 1024 * 1024);
   117	        }
   118	        let mut count_target = options
   119	            .small_count_target
   120	            .unwrap_or(if small_count >= 2048 {
   121	                2048
   122	            } else if small_count >= 1024 {
   123	                1024
   124	            } else {
   125	                256
   126	            });
   127	        count_target = count_target.clamp(128, 4096);
   128	
   129	        let mut cur: Vec<PathBuf> = Vec::new();
   130	        let mut cur_bytes: u64 = 0;
   131	        for p in small.iter() {
   132	            let size = *size_map.get(p).unwrap_or(&(64 * 1024));
   133	            let would_exceed = cur_bytes + size > target_shard;
   134	            let reached_count = cur.len() >= count_target;
   135	            if !cur.is_empty() && (would_exceed || reached_count) {
   136	                small_tasks.push(TransferTask::TarShard(std::mem::take(&mut cur)));
   137	                cur_bytes = 0;
   138	            }
   139	            cur.push(p.clone());
   140	            cur_bytes += size;
   141	        }
   142	        if !cur.is_empty() {
   143	            small_tasks.push(TransferTask::TarShard(cur));
   144	        }
   145	    } else {
   146	        for p in small.iter() {
   147	            small_tasks.push(TransferTask::RawBundle(vec![p.clone()]));
   148	        }
   149	    }
   150	
   151	    let mut medium_tasks: Vec<TransferTask> = Vec::new();
   152	    let mut target_bundle: u64 = options.medium_target.unwrap_or(128 * 1024 * 1024);
   153	    if total_medium_bytes >= 512 * 1024 * 1024 {
   154	        target_bundle = target_bundle.max(384 * 1024 * 1024);
   155	    } else if total_bytes > 1_000_000_000 {
   156	        target_bundle = target_bundle.max(256 * 1024 * 1024);
   157	    }
   158	    // Slight spread to avoid synchronized boundaries
   159	    let max_bundle: u64 = (target_bundle as f64 * 1.25) as u64;
   160	    let mut cur_b: Vec<PathBuf> = Vec::new();
   161	    let mut cur_sz: u64 = 0;
   162	    for (p, sz) in medium.into_iter() {
   163	        if !cur_b.is_empty() && (cur_sz >= target_bundle || cur_sz + sz > max_bundle) {
   164	            medium_tasks.push(TransferTask::RawBundle(std::mem::take(&mut cur_b)));
   165	            cur_sz = 0;
   166	        }
   167	        cur_b.push(p);
   168	        cur_sz += sz;
   169	    }
   170	    if !cur_b.is_empty() {
   171	        medium_tasks.push(TransferTask::RawBundle(cur_b));
   172	    }
   173	
   174	    // Interleave tasks from large, shard, bundle to avoid all streams building tars at once
   175	    let mut tasks = Vec::new();
   176	    let mut i_l = 0usize;
   177	    let mut i_s = 0usize;
   178	    let mut i_m = 0usize;
   179	    while i_l < large_files.len() || i_s < small_tasks.len() || i_m < medium_tasks.len() {
   180	        if i_l < large_files.len() {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-IrrOR95Z' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-TgloQJ3a' (errno=Operation not permitted)
   990	                                    } else {
   991	                                        if neg.tcp_port == 0 {
   992	                                            eyre::bail!("server reported zero data port for negotiated transfer");
   993	                                        }
   994	
   995	                                        let token_bytes = decode_token(&neg.one_time_token)?;
   996	                                        // ue-r2-1e: the daemon (byte
   997	                                        // receiver) advertised its profile
   998	                                        // on this negotiation — the dial's
   999	                                        // ceilings honor it (first-wins,
  1000	                                        // like the old tuning memo).
  1001	                                        let dial = ensure_dial(
  1002	                                            &mut dial,
  1003	                                            neg.receiver_capacity.as_ref(),
  1004	                                        );
  1005	                                        if data_plane_sender.is_none() {
  1006	                                            let stream_target = dial.set_negotiated_streams(
  1007	                                                neg.stream_count.max(1) as usize,
  1008	                                            );
  1009	                                            let payload_prefetch = dial.prefetch_count();
  1010	                                            // ue-r2-2: the daemon's fold said
  1011	                                            // resize is on for this transfer —
  1012	                                            // epoch-0 sockets carry the
  1013	                                            // sub-token suffix and the sender
  1014	                                            // goes elastic. A malformed token
  1015	                                            // length reads as "not enabled"
  1016	                                            // (fail toward today's behavior).
  1017	                                            let resize_sub = (neg.resize_enabled
  1018	                                                && neg.epoch0_sub_token.len()
  1019	                                                    == crate::remote::transfer::SUB_TOKEN_LEN)
  1020	                                                .then(|| neg.epoch0_sub_token.clone());
  1021	                                            resize_negotiated = resize_sub.is_some();
  1022	                                            let mut sender = MultiStreamSender::connect(
  1023	                                                &self.endpoint.host,
  1024	                                                neg.tcp_port,
  1025	                                                &token_bytes,
  1026	                                                dial.chunk_bytes(),
  1027	                                                payload_prefetch,
  1028	                                                stream_target,
  1029	                                                trace_data_plane,
  1030	                                                source.clone(),
  1031	                                                dial.tcp_buffer_bytes(),
  1032	                                                progress.cloned(),
  1033	                                                Some(dial.clone()),
  1034	                                                resize_sub,
  1035	                                            )
  1036	                                            .await?;
  1037	                                            resize_proposal_rx = sender.take_resize_rx();
  1038	                                            data_plane_sender = Some(sender);
  1039	                                            data_port = Some(neg.tcp_port);
  1040	
  1041	                                            // sf-2: need-list batches can
  1042	                                            // predate the negotiation — the
  1043	                                            // accumulated shape may already
  1044	                                            // outgrow the daemon's
  1045	                                            // partial-manifest stream count.
  1046	                                            if resize_negotiated && shape_resize_enabled {
  1047	                                                if let Err(send_err) = maybe_shape_resize(
  1048	                                                    &tx,
  1049	                                                    &dial,
  1050	                                                    transfer_size_hint,
  1051	                                                    files_requested.len(),
  1052	                                                    &mut resize_pending,
  1053	                                                )
  1054	                                                .await
  1055	                                                {
  1056	                                                    return Err(prefer_server_error(
  1057	                                                        &mut response_rx,
  1058	                                                        send_err,
  1059	                                                    )
  1060	                                                    .await);
  1061	                                                }
  1062	                                            }
  1063	                                        }
  1064	
  1065	                                        if let Some(sender) = data_plane_sender.as_mut() {
  1066	                                            let headers =
  1067	                                                drain_pending_headers(&mut pending_queue, &manifest_lookup);
  1068	                                            if !headers.is_empty() {
  1069	                                                let headers = source
  1070	                                                    .check_availability(headers, unreadable_paths.clone())
  1071	                                                    .await?;
  1072	                                                if headers.is_empty() {
  1073	                                                    continue;
  1074	                                                }
  1075	                                            let mut planned = plan_transfer_payloads(
  1076	                                                headers,
  1077	                                                source_root,
  1078	                                                plan_options,
  1079	                                            )?;
  1080	                                            let skipped = prune_unrequested_payloads(
  1081	                                                &mut planned,
  1082	                                                &mut requested_files,
  1083	                                            );
  1084	                                            if skipped > 0 {
  1085	                                                log::debug!(
  1086	                                                    "push: daemon did not request {} payload file(s); skipping",
  1087	                                                    skipped
  1088	                                                );
  1089	                                            }
  1090	                                            if !planned.is_empty() {
  1091	                                                let sent = payload_file_count(&planned);
  1092	                                                sender.queue(planned).await?;
  1093	                                                if sent > 0 && first_payload_elapsed.is_none() {
  1094	                                                    first_payload_elapsed = Some(start.elapsed());
  1095	                                                }
  1096	                                                data_plane_files_sent += sent;
  1097	                                                data_plane_outstanding =
  1098	                                                    data_plane_outstanding.saturating_sub(sent);
  1099	                                                }
  1100	                                            }
  1101	                                        }
  1102	                                        transfer_mode = TransferMode::DataPlane;
  1103	                                    }
  1104	                                }
  1105	                                Some(ServerPayload::Summary(push_summary)) => {
  1106	                                    summary = Some(push_summary);
  1107	                                }
  1108	                                Some(ServerPayload::DataPlaneResizeAck(ack)) => {
  1109	                                    // ue-r2-2: settle the in-flight epoch with
  1110	                                    // what actually happened. An unsolicited or
  1111	                                    // stale ack is ignored exactly as before.
  1112	                                    match resize_pending.take() {
  1113	                                        Some(pending) if ack.epoch == pending.epoch => {
  1114	                                            let dial_ref = dial
  1115	                                                .as_ref()
  1116	                                                .expect("resize only negotiated on the dial path");
  1117	                                            if pending.add && ack.accepted {
  1118	                                                // Daemon armed the accept —
  1119	                                                // dial the new socket. A failed
  1120	                                                // dial must NOT kill a healthy
  1121	                                                // transfer: the armed slot
  1122	                                                // expires daemon-side and the
  1123	                                                // live count simply stands.
  1124	                                                let added = match data_plane_sender.as_mut() {
  1125	                                                    Some(sender) => {
  1126	                                                        match sender
  1127	                                                            .add_stream(&pending.sub_token)
  1128	                                                            .await
  1129	                                                        {
  1130	                                                            Ok(()) => true,
  1131	                                                            Err(err) => {
  1132	                                                                log::warn!(
  1133	                                                                    "resize ADD (epoch {}) dial \
  1134	                                                                     failed; continuing at the \
  1135	                                                                     current stream count: {err:#}",
  1136	                                                                    pending.epoch
  1137	                                                                );
  1138	                                                                false
  1139	                                                            }
  1140	                                                        }
  1141	                                                    }
  1142	                                                    None => false,
  1143	                                                };
  1144	                                                if added {
  1145	                                                    dial_ref.resize_settled(
  1146	                                                        pending.epoch,
  1147	                                                        pending.target,
  1148	                                                        true,
  1149	                                                    );
  1150	                                                } else {
  1151	                                                    dial_ref.resize_settled(
  1152	                                                        pending.epoch,
  1153	                                                        dial_ref.live_streams(),
  1154	                                                        true,
  1155	                                                    );
  1156	                                                }
  1157	                                            } else if !pending.add && ack.accepted {
  1158	                                                dial_ref.resize_settled(
  1159	                                                    pending.epoch,
  1160	                                                    pending.target,
  1161	                                                    true,
  1162	                                                );
  1163	                                            } else {
  1164	                                                dial_ref.resize_settled(
  1165	                                                    pending.epoch,
  1166	                                                    dial_ref.live_streams(),
  1167	                                                    false,
  1168	                                                );
  1169	                                            }
  1170	                                            // sf-2: the epoch settled — if the
  1171	                                            // need-list shape still wants more
  1172	                                            // streams, propose the next single
  1173	                                            // ADD (the ramp is one stream per
  1174	                                            // acked epoch).
  1175	                                            if resize_negotiated
  1176	                                                && shape_resize_enabled
  1177	                                                && data_plane_sender.is_some()
  1178	                                            {
  1179	                                                let dial_ref = dial_ref.clone();
  1180	                                                if let Err(send_err) = maybe_shape_resize(
  1181	                                                    &tx,
  1182	                                                    &dial_ref,
  1183	                                                    transfer_size_hint,
  1184	                                                    files_requested.len(),
  1185	                                                    &mut resize_pending,
  1186	                                                )
  1187	                                                .await
  1188	                                                {
  1189	                                                    return Err(prefer_server_error(
  1190	                                                        &mut response_rx,
  1191	                                                        send_err,
  1192	                                                    )
  1193	                                                    .await);
  1194	                                                }
  1195	                                            }
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-FA3XoJLS' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rXrSRQZ4' (errno=Operation not permitted)
   190	                let module_ref = module.as_ref().ok_or_else(|| {
   191	                    Status::failed_precondition("push manifest received before header")
   192	                })?;
   193	                let rel = resolve_manifest_relative_path(&file.relative_path)?;
   194	                expected_rel_files.push(rel.clone());
   195	                // Wire paths are canonically POSIX (`path_posix`). On
   196	                // Windows, `PathBuf::to_string_lossy` re-joins the
   197	                // validated components with backslashes, so the
   198	                // need-list echoed paths the client's manifest lookup
   199	                // (keyed by its own POSIX strings) could never match —
   200	                // every nested-path push to a Windows daemon planned
   201	                // zero payloads for those files and both ends stalled.
   202	                let sanitized = blit_core::path_posix::relative_path_to_posix(&rel);
   203	                file.relative_path = sanitized.clone();
   204	
   205	                // w4-4: buffer the entry; the requires-upload check
   206	                // (canonical containment + stat, 3+ blocking syscalls)
   207	                // runs in chunked spawn_blocking batches instead of
   208	                // inline on the runtime — a 1M-file push used to run
   209	                // ~3M+ blocking syscalls on an executor worker.
   210	                if manifest_buffered_at.is_none() {
   211	                    manifest_buffered_at = Some(Instant::now());
   212	                }
   213	                pending_manifest.push(PendingManifestEntry {
   214	                    rel,
   215	                    sanitized,
   216	                    file,
   217	                });
   218	                if manifest_drain_due(pending_manifest.len(), manifest_buffered_at) {
   219	                    let flushed = drain_manifest_checks(
   220	                        module_ref,
   221	                        &mut pending_manifest,
   222	                        &mut need_list_sender,
   223	                        &mut files_to_upload,
   224	                    )
   225	                    .await?;
   226	                    manifest_buffered_at = None;
   227	                    // design-4: in forced-gRPC mode the early-flush branch
   228	                    // must NOT announce the fallback negotiation here. The
   229	                    // client reacts to Negotiation(tcp_fallback) by
   230	                    // immediately streaming FileData on this same request
   231	                    // stream — but this loop is still reading the manifest,
   232	                    // and its FileData arm is a hard failed_precondition.
   233	                    // That broke every forced-gRPC push of ≥128 files
   234	                    // (FILE_LIST_EARLY_FLUSH_ENTRIES) and was timing-flaky
   235	                    // near ~100. The post-manifest execute_grpc_fallback
   236	                    // sends the one canonical fallback negotiation — the
   237	                    // path every working small push already takes. Early
   238	                    // negotiation only ever helped the TCP path (it starts
   239	                    // the data plane for pipelining), so it is now TCP-only.
   240	                    // (w4-4 moved this from per-entry to post-chunk-drain:
   241	                    // the data plane still spins up mid-manifest on the
   242	                    // first flush, at chunk granularity.)
   243	                    if flushed && data_plane_handle.is_none() && !force_grpc_effective {
   244	                        {
   245	                            let listener = match bind_data_plane_listener().await {
   246	                                Ok(l) => l,
   247	                                Err(_) => {
   248	                                    // Bind failed: flip to fallback mode but
   249	                                    // stay quiet — announcing mid-manifest
   250	                                    // would trip the same design-4 wedge.
   251	                                    fallback_used = true;
   252	                                    force_grpc_effective = true;
   253	                                    continue;
   254	                                }
   255	                            };
   256	
   257	                            let port = listener
   258	                                .local_addr()
   259	                                .map_err(|err| {
   260	                                    Status::internal(format!("querying listener addr: {}", err))
   261	                                })?
   262	                                .port();
   263	
   264	                            let token = generate_token()?;
   265	                            let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
   266	
   267	                            let module_for_transfer = module_ref.clone();
   268	
   269	                            let stream_target = engine_stream_proposal(&files_to_upload);
   270	                            // ue-r2-2: full resize fold — peer bit AND
   271	                            // own support AND a live TCP data plane
   272	                            // (this literal only exists on that path;
   273	                            // the fallback literal stays false).
   274	                            let resize_on = client_supports_resize;
   275	                            let epoch0_sub = if resize_on {
   276	                                generate_resize_sub_token()?
   277	                            } else {
   278	                                Vec::new()
   279	                            };
   280	                            let transfer_task = if resize_on {
   281	                                let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
   282	                                resize_cmd_tx = Some(cmd_tx);
   283	                                resize_live = stream_target.max(1);
   284	                                AbortOnDrop::new(tokio::spawn(
   285	                                    accept_data_connection_stream_resizable(
   286	                                        listener,
   287	                                        token.clone(),
   288	                                        epoch0_sub.clone(),
   289	                                        module_for_transfer,
   290	                                        stream_target,
   291	                                        cmd_rx,
   292	                                    ),
   293	                                ))
   294	                            } else {
   295	                                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream(
   296	                                    listener,
   297	                                    token.clone(),
   298	                                    module_for_transfer,
   299	                                    stream_target,
   300	                                )))
   301	                            };
   302	
   303	                            send_control_message(
   304	                                &tx,
   305	                                server_push_response::Payload::Negotiation(
   306	                                    DataTransferNegotiation {
   307	                                        tcp_port: port as u32,
   308	                                        one_time_token: token_string,
   309	                                        tcp_fallback: false,
   310	                                        stream_count: stream_target,
   311	                                        // ue-r2-1e: the daemon is the
   312	                                        // byte receiver on push — it
   313	                                        // advertises its capacity so
   314	                                        // the client's dial can ramp
   315	                                        // within it.
   316	                                        receiver_capacity: Some(
   317	                                            blit_core::engine::local_receiver_capacity(),
   318	                                        ),
   319	                                        resize_enabled: resize_on,
   320	                                        epoch0_sub_token: epoch0_sub,
   321	                                    },
   322	                                ),
   323	                            )
   324	                            .await?;
   325	
   326	                            data_plane_handle = Some(transfer_task);
   327	                        }
   328	                    }
   329	                }
   330	            }
   331	            Some(client_push_request::Payload::ManifestComplete(mc)) => {
   332	                // w4-4: drain the sub-chunk remainder before leaving the
   333	                // manifest phase — `need_list_sender.finish()` below and
   334	                // the post-manifest negotiation both need the complete
   335	                // need list / files_to_upload. No mid-manifest data-plane
   336	                // spin-up here: the post-manifest path owns negotiation
   337	                // once the manifest is done.
   338	                if !pending_manifest.is_empty() {
   339	                    let module_ref = module.as_ref().ok_or_else(|| {
   340	                        Status::failed_precondition("push manifest received before header")
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-EycoZpJh' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ARvwE6RI' (errno=Operation not permitted)
   740	    })
   741	    .await
   742	    .map_err(|err| Status::internal(format!("manifest check task failed: {err}")))?;
   743	    let decisions = decisions?;
   744	
   745	    let mut any_flushed = false;
   746	    for (entry, requires_upload) in batch.into_iter().zip(decisions) {
   747	        if requires_upload {
   748	            // w4-2: the 262,144-slot upload channel that used to sit
   749	            // here is gone. Headers travel on the wire post-Phase-5;
   750	            // the TCP receiver drained it into the void, and in gRPC
   751	            // fallback nothing read it at all — so manifest entry
   752	            // #262,145 wedged daemon and client forever with no
   753	            // timeout in scope.
   754	            // w5-1: was an unconditional per-file eprintln — stderr
   755	            // spam proportional to file count. Debug-level now;
   756	            // visible with BLIT_LOG=debug.
   757	            log::debug!("push server queued {}", entry.sanitized);
   758	            let flushed = need_list.push(entry.sanitized).await?;
   759	            any_flushed = any_flushed || flushed;
   760	            files_to_upload.push(entry.file);
   761	        }
   762	    }
   763	    Ok(any_flushed)
   764	}
   765	
   766	fn file_requires_upload(
   767	    module: &ModuleConfig,
   768	    rel: &Path,
   769	    header: &FileHeader,
   770	) -> Result<bool, Status> {
   771	    use super::super::util::resolve_contained_path;
   772	    // F2: canonical containment check before stat. Same protection
   773	    // as the actual write path — a symlink in the parent could
   774	    // otherwise have us stat outside the module.
   775	    let full_path = resolve_contained_path(module, rel)?;
   776	    let requires_upload = match fs::metadata(&full_path) {
   777	        Ok(meta) => {
   778	            if !meta.is_file() {
   779	                true
   780	            } else {
   781	                let same_size = meta.len() == header.size;
   782	                let same_mtime = metadata_mtime_seconds(&meta)
   783	                    .map(|seconds| seconds == header.mtime_seconds)
   784	                    .unwrap_or(false);
   785	                !(same_size && same_mtime)
   786	            }
   787	        }
   788	        Err(_) => true,
   789	    };
   790	    Ok(requires_upload)
   791	}
   792	
   793	/// ue-r2-1f: the daemon's private `desired_streams` ladder retired
   794	/// into the engine's shared shape-aware proposal (same table), clamped
   795	/// to the receiver ceiling this daemon advertises in its
   796	/// CapacityProfile. Single owner for the push stream-count start; the
   797	/// client's dial clamps again on its side.
   798	fn engine_stream_proposal(files: &[FileHeader]) -> u32 {
   799	    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
   800	    blit_core::engine::initial_stream_proposal(
   801	        total_bytes,
   802	        files.len(),
   803	        blit_core::engine::local_receiver_capacity().max_streams as usize,
   804	    )
   805	}
   806	
   807	#[cfg(test)]
   808	mod data_plane_handle_abort_tests {
   809	    //! design-2 / w4-1: `handle_push_stream`'s `data_plane_handle` was
   810	    //! a bare `Option<JoinHandle<...>>`. Any early `?` return while a
   811	    //! data-plane accept/receive task was running (the manifest
   812	    //! loop's several fallible `send_control_message` calls, or the
   813	    //! `stream.message()?` race in the post-manifest select loop)
   814	    //! dropped the handle without aborting it, leaving the task
   815	    //! running with no owner — unreachable by `CancelJob`. This pins
   816	    //! the fix at the field-type level: wrapping the same
   817	    //! `tokio::spawn` result in `AbortOnDrop` and dropping the
   818	    //! `Option` (simulating the early-return path) must abort the
   819	    //! task instead of detaching it. The full handler is exercised
   820	    //! end-to-end elsewhere; reproducing a real gRPC push stream just
   821	    //! to trigger this drop path would be disproportionate to the
   822	    //! fix, which is purely "the field is wrapped now".
   823	
   824	    use super::AbortOnDrop;
   825	    use std::sync::atomic::{AtomicBool, Ordering};
   826	    use std::sync::Arc;
   827	    use std::time::Duration;
   828	    use tonic::Status;
   829	
   830	    #[tokio::test]

exec
/bin/zsh -lc "git show 0f922de:crates/blit-core/src/remote/transfer/pipeline.rs | nl -ba | sed -n '165,255p'; nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '165,255p'; git diff --unified=35 0f922de HEAD -- crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '1,190p'; git show 0f922de:crates/blit-core/src/remote/push/client/mod.rs | nl -ba | sed -n '210,315p'; nl -ba crates/blit-app/src/transfers/remote.rs | sed -n '145,195p'; nl -ba crates/blit-core/src/remote/transfer/session_client.rs | sed -n '45,190p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-hoPRqQJv' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-RQN7rHf1' (errno=Operation not permitted)
   165	    ) {
   166	        use std::sync::atomic::Ordering;
   167	        join_set.spawn(async move {
   168	            // Wrap the body so any early-return error trips the shared
   169	            // cancel flag before the `?` unwinds the task.
   170	            let run = async {
   171	                loop {
   172	                    // Stop pulling queued work once a sibling worker has
   173	                    // errored: first-error-wins should surface without the
   174	                    // survivors draining the rest of the bounded queue.
   175	                    // Interrupting an in-flight prepare/write (true prompt
   176	                    // cancellation) is the AbortOnDrop family, w4-1.
   177	                    if cancelled.load(Ordering::Relaxed) {
   178	                        break;
   179	                    }
   180	                    // ue-r2-2: a retired worker stops at the same payload
   181	                    // boundary; queued payloads stay in the shared queue
   182	                    // for the survivors (dequeue = ownership, so
   183	                    // exactly-once is preserved — flume's RecvFut only
   184	                    // takes an item when it resolves, so racing it is
   185	                    // safe). The watch (not a flag) also frees a worker
   186	                    // parked on an IDLE queue. Its `finish()` below emits
   187	                    // the per-stream END record — the receiver-side
   188	                    // teardown signal.
   189	                    let payload = tokio::select! {
   190	                        biased;
   191	                        _ = retire.changed() => break,
   192	                        recv = work_rx.recv_async() => match recv {
   193	                            Ok(p) => p,
   194	                            Err(_) => break, // queue closed and drained
   195	                        },
   196	                    };
   197	                    let prepared = source
   198	                        .prepare_payload(payload)
   199	                        .await
   200	                        .context("preparing payload")?;
   201	                    let files: Vec<(String, u64)> = match &prepared {
   202	                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   203	                        PreparedPayload::TarShard { headers, .. } => headers
   204	                            .iter()
   205	                            .map(|h| (h.relative_path.clone(), h.size))
   206	                            .collect(),
   207	                        // Resume-block payloads patch existing files; no
   208	                        // file-completion event from one-block-at-a-time.
   209	                        // The composite ResumeFile rides the session data
   210	                        // plane, which reports no per-file progress here.
   211	                        PreparedPayload::FileBlock { .. }
   212	                        | PreparedPayload::FileBlockComplete { .. }
   213	                        | PreparedPayload::ResumeFile { .. } => Vec::new(),
   214	                    };
   215	                    let outcome = sink
   216	                        .write_payload(prepared)
   217	                        .await
   218	                        .context("writing payload")?;
   219	                    if let Some(p) = &progress {
   220	                        // Contract (progress.rs): bytes ride Payload, one
   221	                        // FileComplete per file. `size` is the planned
   222	                        // manifest size — the value this lane has always
   223	                        // reported, now on the right variant.
   224	                        for (name, size) in &files {
   225	                            p.report_payload(0, *size);
   226	                            p.report_file_complete(name.clone());
   227	                        }
   228	                    }
   229	                    let mut t = total.lock().unwrap();
   230	                    t.merge(&outcome);
   231	                }
   232	                sink.finish().await?;
   233	                Ok::<(), eyre::Report>(())
   234	            }
   235	            .await;
   236	            if run.is_err() {
   237	                // Signal the forwarder (and implicitly the other workers,
   238	                // once the queue closes) to stop feeding new work.
   239	                cancelled.store(true, Ordering::Relaxed);
   240	            }
   241	            (slot, run)
   242	        });
   243	    }
   244	
   245	    for sink in sinks {
   246	        let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
   247	        let slot = next_slot;
   248	        next_slot += 1;
   249	        retire_flags.push((slot, retire_tx));
   250	        spawn_sink_worker(
   251	            &mut join_set,
   252	            slot,
   253	            sink,
   254	            work_rx.clone(),
   255	            source.clone(),
   165	    ) {
   166	        use std::sync::atomic::Ordering;
   167	        join_set.spawn(async move {
   168	            // Wrap the body so any early-return error trips the shared
   169	            // cancel flag before the `?` unwinds the task.
   170	            let run = async {
   171	                loop {
   172	                    // Stop pulling queued work once a sibling worker has
   173	                    // errored: first-error-wins should surface without the
   174	                    // survivors draining the rest of the bounded queue.
   175	                    // Interrupting an in-flight prepare/write (true prompt
   176	                    // cancellation) is the AbortOnDrop family, w4-1.
   177	                    if cancelled.load(Ordering::Relaxed) {
   178	                        break;
   179	                    }
   180	                    // ue-r2-2: a retired worker stops at the same payload
   181	                    // boundary; queued payloads stay in the shared queue
   182	                    // for the survivors (dequeue = ownership, so
   183	                    // exactly-once is preserved — flume's RecvFut only
   184	                    // takes an item when it resolves, so racing it is
   185	                    // safe). The watch (not a flag) also frees a worker
   186	                    // parked on an IDLE queue. Its `finish()` below emits
   187	                    // the per-stream END record — the receiver-side
   188	                    // teardown signal.
   189	                    let payload = tokio::select! {
   190	                        biased;
   191	                        _ = retire.changed() => break,
   192	                        recv = work_rx.recv_async() => match recv {
   193	                            Ok(p) => p,
   194	                            Err(_) => break, // queue closed and drained
   195	                        },
   196	                    };
   197	                    let prepared = source
   198	                        .prepare_payload(payload)
   199	                        .await
   200	                        .context("preparing payload")?;
   201	                    let files: Vec<(String, u64)> = match &prepared {
   202	                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   203	                        PreparedPayload::TarShard { headers, .. } => headers
   204	                            .iter()
   205	                            .map(|h| (h.relative_path.clone(), h.size))
   206	                            .collect(),
   207	                        // Raw resume-block payloads patch existing files;
   208	                        // no file-completion event from one-block-at-a-
   209	                        // time. The composite ResumeFile IS one whole
   210	                        // file's phase — reported below from the outcome,
   211	                        // because its byte count (stale blocks only) is
   212	                        // known only after the write (codex otp-10a F6).
   213	                        PreparedPayload::FileBlock { .. }
   214	                        | PreparedPayload::FileBlockComplete { .. }
   215	                        | PreparedPayload::ResumeFile { .. } => Vec::new(),
   216	                    };
   217	                    let resumed_file: Option<String> = match &prepared {
   218	                        PreparedPayload::ResumeFile { header, .. } => {
   219	                            Some(header.relative_path.clone())
   220	                        }
   221	                        _ => None,
   222	                    };
   223	                    let outcome = sink
   224	                        .write_payload(prepared)
   225	                        .await
   226	                        .context("writing payload")?;
   227	                    if let Some(p) = &progress {
   228	                        // Contract (progress.rs): bytes ride Payload, one
   229	                        // FileComplete per file. `size` is the planned
   230	                        // manifest size — the value this lane has always
   231	                        // reported, now on the right variant.
   232	                        for (name, size) in &files {
   233	                            p.report_payload(0, *size);
   234	                            p.report_file_complete(name.clone());
   235	                        }
   236	                        // A resumed file finishes like any other (w6-1:
   237	                        // counted once, per-file lane); its bytes are the
   238	                        // stale blocks actually sent.
   239	                        if let Some(name) = resumed_file {
   240	                            p.report_payload(0, outcome.bytes_written);
   241	                            p.report_file_complete(name);
   242	                        }
   243	                    }
   244	                    let mut t = total.lock().unwrap();
   245	                    t.merge(&outcome);
   246	                }
   247	                sink.finish().await?;
   248	                Ok::<(), eyre::Report>(())
   249	            }
   250	            .await;
   251	            if run.is_err() {
   252	                // Signal the forwarder (and implicitly the other workers,
   253	                // once the queue closes) to stop feeding new work.
   254	                cancelled.store(true, Ordering::Relaxed);
   255	            }
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-dhk6MGoX' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ihji4lQO' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/remote/transfer/pipeline.rs b/crates/blit-core/src/remote/transfer/pipeline.rs
index cc5fa4d..5b66f0d 100644
--- a/crates/blit-core/src/remote/transfer/pipeline.rs
+++ b/crates/blit-core/src/remote/transfer/pipeline.rs
@@ -172,91 +172,106 @@ pub async fn execute_sink_pipeline_elastic(
                     // Stop pulling queued work once a sibling worker has
                     // errored: first-error-wins should surface without the
                     // survivors draining the rest of the bounded queue.
                     // Interrupting an in-flight prepare/write (true prompt
                     // cancellation) is the AbortOnDrop family, w4-1.
                     if cancelled.load(Ordering::Relaxed) {
                         break;
                     }
                     // ue-r2-2: a retired worker stops at the same payload
                     // boundary; queued payloads stay in the shared queue
                     // for the survivors (dequeue = ownership, so
                     // exactly-once is preserved — flume's RecvFut only
                     // takes an item when it resolves, so racing it is
                     // safe). The watch (not a flag) also frees a worker
                     // parked on an IDLE queue. Its `finish()` below emits
                     // the per-stream END record — the receiver-side
                     // teardown signal.
                     let payload = tokio::select! {
                         biased;
                         _ = retire.changed() => break,
                         recv = work_rx.recv_async() => match recv {
                             Ok(p) => p,
                             Err(_) => break, // queue closed and drained
                         },
                     };
                     let prepared = source
                         .prepare_payload(payload)
                         .await
                         .context("preparing payload")?;
                     let files: Vec<(String, u64)> = match &prepared {
                         PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
                         PreparedPayload::TarShard { headers, .. } => headers
                             .iter()
                             .map(|h| (h.relative_path.clone(), h.size))
                             .collect(),
-                        // Resume-block payloads patch existing files; no
-                        // file-completion event from one-block-at-a-time.
-                        // The composite ResumeFile rides the session data
-                        // plane, which reports no per-file progress here.
+                        // Raw resume-block payloads patch existing files;
+                        // no file-completion event from one-block-at-a-
+                        // time. The composite ResumeFile IS one whole
+                        // file's phase — reported below from the outcome,
+                        // because its byte count (stale blocks only) is
+                        // known only after the write (codex otp-10a F6).
                         PreparedPayload::FileBlock { .. }
                         | PreparedPayload::FileBlockComplete { .. }
                         | PreparedPayload::ResumeFile { .. } => Vec::new(),
                     };
+                    let resumed_file: Option<String> = match &prepared {
+                        PreparedPayload::ResumeFile { header, .. } => {
+                            Some(header.relative_path.clone())
+                        }
+                        _ => None,
+                    };
                     let outcome = sink
                         .write_payload(prepared)
                         .await
                         .context("writing payload")?;
                     if let Some(p) = &progress {
                         // Contract (progress.rs): bytes ride Payload, one
                         // FileComplete per file. `size` is the planned
                         // manifest size — the value this lane has always
                         // reported, now on the right variant.
                         for (name, size) in &files {
                             p.report_payload(0, *size);
                             p.report_file_complete(name.clone());
                         }
+                        // A resumed file finishes like any other (w6-1:
+                        // counted once, per-file lane); its bytes are the
+                        // stale blocks actually sent.
+                        if let Some(name) = resumed_file {
+                            p.report_payload(0, outcome.bytes_written);
+                            p.report_file_complete(name);
+                        }
                     }
                     let mut t = total.lock().unwrap();
                     t.merge(&outcome);
                 }
                 sink.finish().await?;
                 Ok::<(), eyre::Report>(())
             }
             .await;
             if run.is_err() {
                 // Signal the forwarder (and implicitly the other workers,
                 // once the queue closes) to stop feeding new work.
                 cancelled.store(true, Ordering::Relaxed);
             }
             (slot, run)
         });
     }
 
     for sink in sinks {
         let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
         let slot = next_slot;
         next_slot += 1;
         retire_flags.push((slot, retire_tx));
         spawn_sink_worker(
             &mut join_set,
             slot,
             sink,
             work_rx.clone(),
             source.clone(),
             progress.cloned(),
             total.clone(),
             cancelled.clone(),
             retire_rx,
         );
     }
 
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Zl7AflPo' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-6m3sVFqR' (errno=Operation not permitted)
   210	        // telemetry and a tuner task steps the dial's cheap dials from
   211	        // it. Without one (no live tuning), the NoProbe path
   212	        // monomorphizes the telemetry away exactly as before.
   213	        let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
   214	        let mut tuner_handle = None;
   215	        let mut resize = None;
   216	        let mut resize_rx = None;
   217	        if let Some(dial) = dial.as_ref() {
   218	            use crate::engine::spawn_dial_tuner_with_resize;
   219	            use crate::remote::transfer::progress::{LiveProbe, StreamId, StreamProbe};
   220	            let mut tuner_probes = Vec::with_capacity(streams);
   221	            for idx in 0..streams {
   222	                let probe = StreamProbe::new(StreamId(idx as u32));
   223	                tuner_probes.push(StreamProbe::from_telemetry(
   224	                    StreamId(idx as u32),
   225	                    probe.telemetry(),
   226	                ));
   227	                let session = DataPlaneSession::connect_with_probe(
   228	                    host,
   229	                    port,
   230	                    &handshake,
   231	                    chunk_bytes,
   232	                    payload_prefetch,
   233	                    trace,
   234	                    tcp_buffer_size,
   235	                    Arc::clone(&pool),
   236	                    LiveProbe(probe),
   237	                )
   238	                .await?;
   239	                sinks.push(Arc::new(DataPlaneSink::new(
   240	                    session,
   241	                    source.clone(),
   242	                    dst_root.clone(),
   243	                )));
   244	            }
   245	            let probes: crate::engine::SharedStreamProbes =
   246	                Arc::new(std::sync::Mutex::new(tuner_probes));
   247	            if resize_sub.is_some() {
   248	                let (proposal_tx, proposal_rx) = tokio::sync::mpsc::unbounded_channel();
   249	                tuner_handle = Some(spawn_dial_tuner_with_resize(
   250	                    dial,
   251	                    Arc::clone(&probes),
   252	                    Some(proposal_tx),
   253	                ));
   254	                resize_rx = Some(proposal_rx);
   255	                resize = Some(ResizeRuntime {
   256	                    ctl_tx: ctl_tx.clone(),
   257	                    probes,
   258	                    host: host.to_string(),
   259	                    port,
   260	                    token: token.to_vec(),
   261	                    trace,
   262	                    pool: Arc::clone(&pool),
   263	                    source: source.clone(),
   264	                    dst_root: dst_root.clone(),
   265	                    dial: Arc::clone(dial),
   266	                    next_stream_id: streams as u32,
   267	                });
   268	            } else {
   269	                tuner_handle = Some(spawn_dial_tuner_with_resize(dial, probes, None));
   270	            }
   271	        } else {
   272	            for _ in 0..streams {
   273	                let session = DataPlaneSession::connect(
   274	                    host,
   275	                    port,
   276	                    &handshake,
   277	                    chunk_bytes,
   278	                    payload_prefetch,
   279	                    trace,
   280	                    tcp_buffer_size,
   281	                    Arc::clone(&pool),
   282	                )
   283	                .await?;
   284	                sinks.push(Arc::new(DataPlaneSink::new(
   285	                    session,
   286	                    source.clone(),
   287	                    dst_root.clone(),
   288	                )));
   289	            }
   290	        }
   291	
   292	        let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));
   293	
   294	        let source_clone = source.clone();
   295	        let prefetch = payload_prefetch.max(1);
   296	        drop(ctl_tx);
   297	        let pipeline_handle = AbortOnDrop::new(tokio::spawn(async move {
   298	            execute_sink_pipeline_elastic(
   299	                source_clone,
   300	                sinks,
   301	                payload_rx,
   302	                prefetch,
   303	                progress.as_ref(),
   304	                Some(ctl_rx),
   305	            )
   306	            .await
   307	        }));
   308	
   309	        Ok(Self {
   310	            payload_tx: Some(payload_tx),
   311	            tuner_handle,
   312	            pipeline_handle: Some(pipeline_handle),
   313	            started: Instant::now(),
   314	            resize,
   315	            resize_rx,
   145	/// awaits the monitor. Standard lifecycle:
   146	///
   147	/// ```text
   148	/// let (handle, task) = spawn_progress_monitor(...);
   149	/// let outcome = run_remote_push(execution, handle.as_ref()).await?;
   150	/// drop(handle);
   151	/// if let Some(t) = task { let _ = t.await; }
   152	/// ```
   153	///
   154	/// Unlike the pull side, there is no need to split this into
   155	/// pre-/post-purge halves — push has no post-RPC destructive
   156	/// step on the caller's filesystem, so the monitor's lifetime
   157	/// already lines up cleanly with the RPC.
   158	pub async fn run_remote_push(
   159	    execution: PushExecution,
   160	    progress: Option<&RemoteTransferProgress>,
   161	) -> Result<PushExecutionOutcome> {
   162	    let source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(execution.source));
   163	
   164	    let options = PushSessionOptions {
   165	        compare_mode: execution.compare_mode,
   166	        ignore_existing: execution.ignore_existing,
   167	        require_complete_scan: execution.require_complete_scan,
   168	        // `--force-grpc`: the session's in-stream byte carrier is the
   169	        // gRPC-fallback lane (otp-8).
   170	        in_stream_bytes: execution.force_grpc,
   171	        resume: execution.resume,
   172	        resume_block_size: execution.resume_block_size,
   173	        filter: execution.filter,
   174	        mirror_enabled: execution.mirror_mode,
   175	        mirror_kind: if execution.mirror_mode {
   176	            execution.mirror_kind
   177	        } else {
   178	            MirrorMode::Off
   179	        },
   180	        progress: progress.cloned(),
   181	        trace_data_plane: execution.trace_data_plane,
   182	        ..PushSessionOptions::default()
   183	    };
   184	
   185	    let summary = run_push_session(&execution.remote, source, options)
   186	        .await
   187	        .with_context(|| format!("pushing to {}", execution.remote_label))?;
   188	
   189	    Ok(PushExecutionOutcome {
   190	        summary,
   191	        destination: execution.remote_label,
   192	    })
   193	}
   194	
   195	/// Inputs for [`run_remote_pull`] (otp-10b-2: the pull-shaped verb on
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
   180	        },
   181	    };
   182	    let summary = run_source(cfg, transport, source).await?;
   183	
   184	    let unreadable = unreadable
   185	        .lock()
   186	        .map_err(|err| eyre!("unreadable-path accumulator poisoned: {err}"))?;
   187	    if !unreadable.is_empty() {
   188	        let preview: Vec<_> = unreadable.iter().take(5).cloned().collect();
   189	        let mut message = format!(
   190	            "{} file(s) were skipped due to permission or access errors: {}",

exec
/bin/zsh -lc "rg -n \"send_prepared_tar_shard|DATA_PLANE_RECORD_TAR_SHARD|write_u|write_all|flush\" crates/blit-core/src/remote/transfer/data_plane.rs; nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '400,500p'; git show 0f922de:crates/blit-core/src/remote/transfer/data_plane.rs | nl -ba | sed -n '390,490p'; git diff --unified=8 0f922de HEAD -- crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '1,420p'; git diff --find-renames --stat 0f922de HEAD -- crates/blit-core/src/engine/dial.rs crates/blit-core/src/dial.rs" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
17:pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
55:/// (15+ minutes). All existing `self.stream.write_all/.flush` call
216:                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
240:            .write_all(&[DATA_PLANE_RECORD_END])
244:            .flush()
246:            .context("flushing data plane stream")
286:            .write_all(&[DATA_PLANE_RECORD_FILE])
290:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
294:            .write_all(path_bytes)
299:            .write_all(&header.size.to_be_bytes())
307:            .write_all(&header.mtime_seconds.to_be_bytes())
311:            .write_all(&header.permissions.to_be_bytes())
373:            // join first polls it and stops when write_all completes —
386:                    let result = stream.write_all(write_slice).await;
430:                .write_all(&buf_a.as_slice()[..bytes_a])
446:    pub async fn send_prepared_tar_shard(
464:            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
468:            .write_all(&(headers.len() as u32).to_be_bytes())
481:                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
485:                .write_all(rel_bytes)
489:                .write_all(&header.size.to_be_bytes())
493:                .write_all(&header.mtime_seconds.to_be_bytes())
497:                .write_all(&header.permissions.to_be_bytes())
503:            .write_all(&(data.len() as u64).to_be_bytes())
517:                .write_all(chunk)
558:            .write_all(&[DATA_PLANE_RECORD_BLOCK])
562:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
566:            .write_all(path_bytes)
570:            .write_all(&offset.to_be_bytes())
574:            .write_all(&(content.len() as u32).to_be_bytes())
578:            .write_all(content)
614:            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
618:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
622:            .write_all(path_bytes)
626:            .write_all(&total_size.to_be_bytes())
630:            .write_all(&mtime_seconds.to_be_bytes())
634:            .write_all(&permissions.to_be_bytes())
707:            dst.write_all(&buf_a[..bytes_a]),
712:        // AFTER `write_all` succeeds so a `bytes_completed`
733:        dst.write_all(&buf_a[..bytes_a])
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
   431	                .await
   432	                .with_context(|| format!("sending {}", rel))?;
   433	            if P::ACTIVE {
   434	                if let Some(t) = tail_start {
   435	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   436	                }
   437	            }
   438	            self.probe.record_bytes(bytes_a as u64);
   439	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
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
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jUV1KfBZ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-D2rzSd9s' (errno=Operation not permitted)
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
   431	                .await
   432	                .with_context(|| format!("sending {}", rel))?;
   433	            if P::ACTIVE {
   434	                if let Some(t) = tail_start {
   435	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   436	                }
   437	            }
   438	            self.probe.record_bytes(bytes_a as u64);
   439	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
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
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nGlk9kAj' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jvh4nP9D' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-z0uCfLQP' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-q356hpaf' (errno=Operation not permitted)
 crates/blit-core/src/{engine => }/dial.rs | 0
 1 file changed, 0 insertions(+), 0 deletions(-)

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '990,1175p'; git show 0f922de:crates/blit-daemon/src/service/push/data_plane.rs | nl -ba | sed -n '175,220p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2560,2725p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2920,3080p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   990	// ---------------------------------------------------------------------------
   991	// Need-list enforcement for the data-plane receive
   992	// ---------------------------------------------------------------------------
   993	
   994	/// Sink decorator that enforces the session's need-list contract on the
   995	/// data-plane receive, giving it the SAME strictness the in-stream
   996	/// carrier applies inline in the control loop (`outstanding.remove`).
   997	/// `execute_receive_pipeline` writes socket-provided paths directly, so
   998	/// without this a peer could substitute an off-need-list path for a
   999	/// needed one (count-preserving), duplicate one, or send resume block
  1000	/// records the session never negotiated (codex otp-4b-1 F1). Every
  1001	/// written path must be a granted, not-yet-received need. Resume
  1002	/// sessions (otp-7b) additionally validate + claim block records
  1003	/// against the shared [`ResumeHeaders`] grant map — with the identical
  1004	/// strictness the in-stream `claim_resume_record` applies — and count
  1005	/// completions into the shared resumed counter; in a non-resume session
  1006	/// block records are rejected outright. The shared [`OutstandingNeeds`]
  1007	/// set makes completion `is_empty()` for both carriers.
  1008	pub(super) struct NeedListSink {
  1009	    inner: Arc<dyn TransferSink>,
  1010	    outstanding: OutstandingNeeds,
  1011	    /// `Some` iff the session negotiated resume (otp-7b): the shared
  1012	    /// grant map + resumed counter block records are validated and
  1013	    /// claimed against. `None` ⇒ any block record is a violation.
  1014	    resume: Option<ResumeRecv>,
  1015	}
  1016	
  1017	impl NeedListSink {
  1018	    pub(super) fn new(
  1019	        inner: Arc<dyn TransferSink>,
  1020	        outstanding: OutstandingNeeds,
  1021	        resume: Option<ResumeRecv>,
  1022	    ) -> Self {
  1023	        Self {
  1024	            inner,
  1025	            outstanding,
  1026	            resume,
  1027	        }
  1028	    }
  1029	
  1030	    /// Remove `path` from the outstanding set, or fault: a path that is
  1031	    /// not present is either off the need list or a duplicate delivery.
  1032	    fn claim(&self, path: &str) -> Result<()> {
  1033	        if self
  1034	            .outstanding
  1035	            .lock()
  1036	            .expect("outstanding-needs lock poisoned")
  1037	            .remove(path)
  1038	        {
  1039	            Ok(())
  1040	        } else {
  1041	            Err(eyre::Report::new(
  1042	                SessionFault::protocol_violation(format!(
  1043	                    "data-plane payload for '{path}' which is not an outstanding need \
  1044	                 (off the need list, or a duplicate delivery)"
  1045	                ))
  1046	                .with_path(path),
  1047	            ))
  1048	        }
  1049	    }
  1050	
  1051	    /// codex otp-7a F3, data-plane parity: a resume-flagged grant may
  1052	    /// be satisfied ONLY by its block record — a whole-file or tar-shard
  1053	    /// delivery for it bypasses the hash choreography this end committed
  1054	    /// to.
  1055	    fn reject_resume_flagged(&self, path: &str) -> Result<()> {
  1056	        if let Some(resume) = &self.resume {
  1057	            if resume
  1058	                .headers
  1059	                .lock()
  1060	                .expect("resume-headers lock poisoned")
  1061	                .contains_key(path)
  1062	            {
  1063	                return Err(eyre::Report::new(
  1064	                    SessionFault::protocol_violation(format!(
  1065	                        "data-plane file payload for resume-flagged '{path}' — the \
  1066	                         contract requires its block record"
  1067	                    ))
  1068	                    .with_path(path),
  1069	                ));
  1070	            }
  1071	        }
  1072	        Ok(())
  1073	    }
  1074	
  1075	    /// otp-7b: validate one mid-record `FileBlock` against its grant —
  1076	    /// the path must hold a live resume grant, still be an outstanding
  1077	    /// need (its completion has not claimed it), and the block must stay
  1078	    /// inside the manifested size. The grant is NOT claimed here;
  1079	    /// [`Self::claim_block_complete`] does that exactly once.
  1080	    fn check_block(&self, path: &str, offset: u64, len: u64) -> Result<()> {
  1081	        let Some(resume) = &self.resume else {
  1082	            return Err(eyre::Report::new(SessionFault::protocol_violation(
  1083	                "resume block record on the data plane of a non-resume session",
  1084	            )));
  1085	        };
  1086	        let size = {
  1087	            let held = resume.headers.lock().expect("resume-headers lock poisoned");
  1088	            match held.get(path) {
  1089	                Some(header) => header.size,
  1090	                None => {
  1091	                    return Err(eyre::Report::new(
  1092	                        SessionFault::protocol_violation(format!(
  1093	                            "data-plane block record for '{path}' which was not granted \
  1094	                             a resume-flagged need"
  1095	                        ))
  1096	                        .with_path(path),
  1097	                    ))
  1098	                }
  1099	            }
  1100	        };
  1101	        if !self
  1102	            .outstanding
  1103	            .lock()
  1104	            .expect("outstanding-needs lock poisoned")
  1105	            .contains(path)
  1106	        {
  1107	            return Err(eyre::Report::new(
  1108	                SessionFault::protocol_violation(format!(
  1109	                    "data-plane block record for '{path}' which is not an outstanding need"
  1110	                ))
  1111	                .with_path(path),
  1112	            ));
  1113	        }
  1114	        if offset.saturating_add(len) > size {
  1115	            return Err(eyre::Report::new(
  1116	                SessionFault::protocol_violation(format!(
  1117	                    "block record '{path}' overran its size: offset {offset} + {len} \
  1118	                     byte(s) > {size}"
  1119	                ))
  1120	                .with_path(path),
  1121	            ));
  1122	        }
  1123	        Ok(())
  1124	    }
  1125	
  1126	    /// otp-7b: claim one `FileBlockComplete` — remove the grant, verify
  1127	    /// the completed size against the manifest promise, and claim the
  1128	    /// outstanding need. Mirrors the in-stream `claim_resume_record` +
  1129	    /// `finish_block_record` checks. The resumed COUNT happens in
  1130	    /// `write_payload` only after the finalization write lands, matching
  1131	    /// the in-stream ordering.
  1132	    fn claim_block_complete(&self, path: &str, total_size: u64) -> Result<()> {
  1133	        let Some(resume) = &self.resume else {
  1134	            return Err(eyre::Report::new(SessionFault::protocol_violation(
  1135	                "resume block record on the data plane of a non-resume session",
  1136	            )));
  1137	        };
  1138	        let header = resume
  1139	            .headers
  1140	            .lock()
  1141	            .expect("resume-headers lock poisoned")
  1142	            .remove(path)
  1143	            .ok_or_else(|| {
  1144	                eyre::Report::new(
  1145	                    SessionFault::protocol_violation(format!(
  1146	                        "data-plane block complete for '{path}' which was not granted \
  1147	                         a resume-flagged need"
  1148	                    ))
  1149	                    .with_path(path),
  1150	                )
  1151	            })?;
  1152	        if total_size != header.size {
  1153	            return Err(eyre::Report::new(
  1154	                SessionFault::protocol_violation(format!(
  1155	                    "block record '{path}' completed at {total_size} byte(s), manifest \
  1156	                     promised {}",
  1157	                    header.size
  1158	                ))
  1159	                .with_path(path),
  1160	            ));
  1161	        }
  1162	        self.claim(path)
  1163	    }
  1164	}
  1165	
  1166	#[async_trait]
  1167	impl TransferSink for NeedListSink {
  1168	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
  1169	        match &payload {
  1170	            PreparedPayload::File(header) => {
  1171	                self.reject_resume_flagged(&header.relative_path)?;
  1172	                self.claim(&header.relative_path)?;
  1173	            }
  1174	            PreparedPayload::TarShard { headers, .. } => {
  1175	                for header in headers {
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lQN775I5' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-NSsL7mYH' (errno=Operation not permitted)
   175	    module: ModuleConfig,
   176	    start: Instant,
   177	) -> Result<TransferStats, Status> {
   178	    eprintln!(
   179	        "blitd: push data plane: token accepted (module='{}', root={})",
   180	        module.name,
   181	        module.path.display()
   182	    );
   183	
   184	    // Route the inbound wire through the unified receive pipeline:
   185	    //   socket → StallGuard → execute_receive_pipeline → FsTransferSink → disk
   186	    // Same call shape as the client's pull-receive side. Tar shards get
   187	    // extracted inline by FsTransferSink (parallelism across streams
   188	    // already comes from N concurrent invocations of this function).
   189	    //
   190	    // audit-h3a (R2/R3 finding H3): symmetric to the audit-1c CLI
   191	    // pull-receive guard. Before this slice the push-receive socket had
   192	    // no idle deadline at all — a hostile or wedged push client that
   193	    // accepted the data plane, sent the token, then went silent would
   194	    // pin this worker indefinitely (DATA_PLANE_TOKEN_TIMEOUT above only
   195	    // bounds the token read). StallGuard turns that into a clean
   196	    // TimedOut after TRANSFER_STALL_TIMEOUT of no progress.
   197	    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
   198	
   199	    let config = FsSinkConfig {
   200	        preserve_times: true,
   201	        dry_run: false,
   202	        checksum: None,
   203	        resume: false,
   204	        compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
   205	    };
   206	    let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
   207	        PathBuf::new(),
   208	        module.path.clone(),
   209	        config,
   210	    ));
   211	    let outcome = receive_push_data_plane(socket, sink)
   212	        .await
   213	        .map_err(|err| Status::internal(format!("data plane receive: {err:#}")))?;
   214	
   215	    let stats = TransferStats {
   216	        files_transferred: outcome.files_written as u64,
   217	        bytes_transferred: outcome.bytes_written,
   218	        bytes_zero_copy: 0,
   219	    };
   220	
  2560	    dst_root: &Path,
  2561	    data_plane_host: Option<&str>,
  2562	    instruments: DestinationInstruments,
  2563	    local_apply: Option<local::LocalApply>,
  2564	) -> Result<DestinationOutcome> {
  2565	    // otp-10b-2: the receive side's w6-1 progress lane. Need batches are
  2566	    // the denominator (reported where they're emitted, below); per-file
  2567	    // events ride each carrier's record handling.
  2568	    let progress = instruments.progress;
  2569	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  2570	        .unwrap_or(ComparisonMode::Unspecified);
  2571	    // Session deletions run via the otp-6b mirror pass (a whole-tree
  2572	    // diff at SourceDone), never a per-entry flag.
  2573	    let compare_opts = CompareOptions {
  2574	        mode: compare_mode.into(),
  2575	        ignore_existing: negotiated.open.ignore_existing,
  2576	    };
  2577	    // src_root is only consumed by local File payloads, which never
  2578	    // occur on a WIRE session destination (payload bytes arrive as
  2579	    // records and go through the stream/tar write paths); the LOCAL
  2580	    // carrier (otp-11) brings its own fully-configured sink, where
  2581	    // File payloads are the point. `Arc` so the data-plane receive
  2582	    // task (otp-4b) can share the one sink across sockets.
  2583	    let sink: Arc<dyn TransferSink> = match &local_apply {
  2584	        Some(la) => Arc::clone(&la.sink),
  2585	        None => {
  2586	            let mut sink = FsTransferSink::new(
  2587	                PathBuf::new(),
  2588	                dst_root.to_path_buf(),
  2589	                FsSinkConfig {
  2590	                    preserve_times: true,
  2591	                    dry_run: false,
  2592	                    checksum: None,
  2593	                    resume: false,
  2594	                    compare_mode,
  2595	                },
  2596	            );
  2597	            // otp-9a: applied payload bytes report against the caller's live
  2598	            // counter (the delegated dst daemon's jobs row) through the sink's
  2599	            // existing ByteProgressSink contract.
  2600	            if let Some(bp) = instruments.byte_progress {
  2601	                sink = sink.with_byte_progress(bp);
  2602	            }
  2603	            Arc::new(sink)
  2604	        }
  2605	    };
  2606	    // Same canonical-containment chokepoint the sink write paths use
  2607	    // (R46-F3), applied to diff stats so a hostile manifest path can't
  2608	    // make the destination stat outside its root.
  2609	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
  2610	
  2611	    // otp-6b: mirror config. The DESTINATION owns the delete pass (it holds
  2612	    // the tree). `mirror_filter` scopes the dest enumeration — the user
  2613	    // filter for FilteredSubset (out-of-scope dest entries are never
  2614	    // candidates), the whole-tree default for All. Globs were validated at
  2615	    // OPEN. `source_files` accumulates the COMPLETE source file set (only
  2616	    // when mirroring) so the pass can diff it against the dest at SourceDone.
  2617	    let mirror_enabled = negotiated.open.mirror_enabled;
  2618	    let mirror_kind = MirrorMode::try_from(negotiated.open.mirror_kind).unwrap_or(MirrorMode::Off);
  2619	    let mirror_filter: crate::fs_enum::FileFilter =
  2620	        if mirror_enabled && mirror_kind == MirrorMode::FilteredSubset {
  2621	            // otp-11: the local carrier threads the user's FileFilter
  2622	            // directly (process-local; no wire FilterSpec round-trip) —
  2623	            // same type, same delete pass, same scope semantics.
  2624	            if let Some(la) = &local_apply {
  2625	                la.mirror_scope_filter.clone_without_cache()
  2626	            } else {
  2627	                match negotiated.open.filter.as_ref() {
  2628	                    Some(spec) if *spec != FilterSpec::default() => {
  2629	                        crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  2630	                            .map_err(|e| {
  2631	                                eyre::Report::new(SessionFault::internal(format!(
  2632	                                    "invalid filter: {e:#}"
  2633	                                )))
  2634	                            })?
  2635	                    }
  2636	                    _ => crate::fs_enum::FileFilter::default(),
  2637	                }
  2638	            }
  2639	        } else {
  2640	            crate::fs_enum::FileFilter::default()
  2641	        };
  2642	    let mut source_files: HashSet<String> = HashSet::new();
  2643	
  2644	    // otp-7a: resume. Headers of resume-granted needs are retained so a
  2645	    // record's completion can finalize with the manifest's
  2646	    // size/mtime/permissions and be validated against the grant. Both
  2647	    // the header map and the resumed counter are SHARED with the
  2648	    // data-plane receive (otp-7b) exactly as `outstanding` is: on the
  2649	    // data plane the control loop never sees block records, so the
  2650	    // NeedListSink claims resume grants and counts completions as they
  2651	    // land on the sockets. The block size is chosen below, once the
  2652	    // carrier is known (the ceiling is per carrier).
  2653	    let resume_enabled = resume_negotiated(&negotiated.open);
  2654	    let resume_headers: data_plane::ResumeHeaders = Arc::default();
  2655	    let files_resumed = Arc::new(std::sync::atomic::AtomicU64::new(0));
  2656	
  2657	    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
  2658	    // `granted` is the ever-granted DEDUP set — control-loop-local,
  2659	    // insert-only, never removed, so a concurrent data-plane claim can
  2660	    // never re-open a grant (a duplicate manifest path is granted at
  2661	    // most once regardless of delivery timing). `outstanding` is the
  2662	    // not-yet-delivered COMPLETION set — inserted for each freshly
  2663	    // granted path before its NeedBatch, claimed by both carriers (the
  2664	    // in-stream arms inline, the data-plane NeedListSink as payloads
  2665	    // land), and empty at SourceDone. A count proxy was insufficient
  2666	    // (F1); merging the two into one set raced the data-plane claim
  2667	    // against the diff (fix-review F1).
  2668	    let mut granted: HashSet<String> = HashSet::new();
  2669	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  2670	
  2671	    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
  2672	    // bytes arrive on sockets (not the control lane). Set it up NOW —
  2673	    // concurrent with the diff loop below, and before the peer sends — so
  2674	    // the connections are established promptly. Which end connects depends
  2675	    // on connection role (otp-5b): a DESTINATION **responder** (push)
  2676	    // accepts sockets off its bound listener; a DESTINATION **initiator**
  2677	    // (pull) dials the grant it received on `data_plane_host`. Byte
  2678	    // direction is the same either way (DESTINATION receives). The
  2679	    // NeedListSink gives the socket receive the same need-list strictness
  2680	    // the in-stream control loop applies inline; AbortOnDrop (inside the
  2681	    // responder run) bounds the accept task to this future. `resize_live`
  2682	    // tracks the stream count this end has grown to (epoch-0 plus each
  2683	    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
  2684	    // max_streams — both directions resize (push arms+accepts, otp-4b-2;
  2685	    // pull dials, otp-5b-2), so both seed these from their epoch-0 streams.
  2686	    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  2687	        Arc::clone(&sink),
  2688	        Arc::clone(&outstanding),
  2689	        // otp-7b: only a resume session accepts block records on the
  2690	        // data plane; the sink validates + claims them against the same
  2691	        // shared grant state the in-stream arms use.
  2692	        resume_enabled.then(|| data_plane::ResumeRecv {
  2693	            headers: Arc::clone(&resume_headers),
  2694	            resumed: Arc::clone(&files_resumed),
  2695	        }),
  2696	    ));
  2697	    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
  2698	        match negotiated.responder_data_plane {
  2699	            // DESTINATION responder (push, otp-4b): accept + receive.
  2700	            Some(rdp) => {
  2701	                let initial = rdp.initial_streams() as usize;
  2702	                let run = rdp.spawn(recv_sink, progress.clone());
  2703	                let ceiling = run.ceiling;
  2704	                (
  2705	                    Some(data_plane::DestRecvPlane::Responder(run)),
  2706	                    initial,
  2707	                    ceiling,
  2708	                )
  2709	            }
  2710	            // DESTINATION initiator (pull, otp-5b): dial + receive when the
  2711	            // SOURCE responder granted a data plane and we have a host to dial.
  2712	            None => match (&negotiated.accept.data_plane, data_plane_host) {
  2713	                (Some(grant), Some(host)) => {
  2714	                    let initial = grant.initial_streams.max(1) as usize;
  2715	                    let run = data_plane::dial_destination_data_plane(
  2716	                        host,
  2717	                        grant,
  2718	                        recv_sink,
  2719	                        progress.clone(),
  2720	                        instruments.trace_data_plane,
  2721	                    )
  2722	                    .await?;
  2723	                    // otp-5b-2: the pull data plane resizes too. Seed
  2724	                    // `resize_live` from the epoch-0 streams dialed and bound
  2725	                    // growth by the capacity THIS end advertised in its open
  2920	                transport
  2921	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  2922	                    .await?;
  2923	                manifest_complete = true;
  2924	            }
  2925	            Some(Frame::FileBegin(header)) => {
  2926	                // Payload records ride the control lane only under the
  2927	                // in-stream carrier; with a TCP data plane active they
  2928	                // flow over the sockets, so one here is a violation.
  2929	                if data_plane_recv.is_some() {
  2930	                    return Err(violation(format!(
  2931	                        "file record '{}' on the control lane while a TCP data plane is active",
  2932	                        header.relative_path
  2933	                    )));
  2934	                }
  2935	                if !manifest_complete {
  2936	                    return Err(violation(format!(
  2937	                        "payload record for '{}' before ManifestComplete",
  2938	                        header.relative_path
  2939	                    )));
  2940	                }
  2941	                // A resume-flagged grant may be satisfied ONLY by its
  2942	                // block record — a whole-file record for it bypasses the
  2943	                // hash choreography this end committed to (codex F3).
  2944	                if resume_headers
  2945	                    .lock()
  2946	                    .expect("resume-headers lock poisoned")
  2947	                    .contains_key(&header.relative_path)
  2948	                {
  2949	                    return Err(violation(format!(
  2950	                        "file record for resume-flagged '{}' — the contract requires \
  2951	                         its block record",
  2952	                        header.relative_path
  2953	                    )));
  2954	                }
  2955	                if !outstanding
  2956	                    .lock()
  2957	                    .expect("outstanding-needs lock poisoned")
  2958	                    .remove(&header.relative_path)
  2959	                {
  2960	                    return Err(violation(format!(
  2961	                        "payload for '{}' which is not on the need list",
  2962	                        header.relative_path
  2963	                    )));
  2964	                }
  2965	                let outcome = receive_file_record(transport, sink.as_ref(), &header).await?;
  2966	                files_written += outcome.files_written as u64;
  2967	                bytes_written += outcome.bytes_written;
  2968	                // otp-10b-2: in-stream per-file progress, same convention
  2969	                // as the data-plane receive (`execute_receive_pipeline`):
  2970	                // bytes ride Payload, FileComplete is byteless.
  2971	                if let Some(p) = &progress {
  2972	                    p.report_payload(0, outcome.bytes_written);
  2973	                    p.report_file_complete(header.relative_path.clone());
  2974	                }
  2975	            }
  2976	            Some(Frame::Block(block)) => {
  2977	                // otp-7a: a resume block record opens with its first
  2978	                // BlockTransfer (no begin frame). Claim the need and run
  2979	                // the strictly-serialized record to its completion frame.
  2980	                let header = claim_resume_record(
  2981	                    &block.relative_path,
  2982	                    resume_enabled,
  2983	                    data_plane_recv.is_some(),
  2984	                    manifest_complete,
  2985	                    &resume_headers,
  2986	                    &outstanding,
  2987	                )?;
  2988	                let outcome =
  2989	                    receive_block_record(transport, sink.as_ref(), &header, block).await?;
  2990	                files_written += outcome.files_written as u64;
  2991	                bytes_written += outcome.bytes_written;
  2992	                files_resumed.fetch_add(1, Ordering::Relaxed);
  2993	                // The whole block record (patch bytes + completion) ran
  2994	                // to its completion frame — one resumed file done.
  2995	                if let Some(p) = &progress {
  2996	                    p.report_payload(0, outcome.bytes_written);
  2997	                    p.report_file_complete(header.relative_path.clone());
  2998	                }
  2999	            }
  3000	            Some(Frame::BlockComplete(complete)) => {
  3001	                // otp-7a: a zero-block record — every block matched
  3002	                // (identical content, e.g. an mtime-only touch), so the
  3003	                // completion frame arrives with no blocks before it and
  3004	                // finalization stamps size/mtime/perms.
  3005	                let header = claim_resume_record(
  3006	                    &complete.relative_path,
  3007	                    resume_enabled,
  3008	                    data_plane_recv.is_some(),
  3009	                    manifest_complete,
  3010	                    &resume_headers,
  3011	                    &outstanding,
  3012	                )?;
  3013	                let outcome = finish_block_record(sink.as_ref(), &header, &complete).await?;
  3014	                files_written += outcome.files_written as u64;
  3015	                bytes_written += outcome.bytes_written;
  3016	                files_resumed.fetch_add(1, Ordering::Relaxed);
  3017	                // Zero-block record: nothing transferred, the file is
  3018	                // complete (identical content, metadata stamped).
  3019	                if let Some(p) = &progress {
  3020	                    p.report_file_complete(header.relative_path.clone());
  3021	                }
  3022	            }
  3023	            Some(Frame::TarShardHeader(shard)) => {
  3024	                if data_plane_recv.is_some() {
  3025	                    return Err(violation(
  3026	                        "tar shard record on the control lane while a TCP data plane is active"
  3027	                            .into(),
  3028	                    ));
  3029	                }
  3030	                if !manifest_complete {
  3031	                    return Err(violation("tar shard record before ManifestComplete".into()));
  3032	                }
  3033	                // Same rule as file records (codex F3): a resume-flagged
  3034	                // grant may not be satisfied through a tar shard.
  3035	                {
  3036	                    let held = resume_headers.lock().expect("resume-headers lock poisoned");
  3037	                    for h in &shard.files {
  3038	                        if held.contains_key(&h.relative_path) {
  3039	                            return Err(violation(format!(
  3040	                                "tar shard entry for resume-flagged '{}' — the contract \
  3041	                                 requires its block record",
  3042	                                h.relative_path
  3043	                            )));
  3044	                        }
  3045	                    }
  3046	                }
  3047	                {
  3048	                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  3049	                    for h in &shard.files {
  3050	                        if !out.remove(&h.relative_path) {
  3051	                            return Err(violation(format!(
  3052	                                "tar shard entry '{}' which is not on the need list",
  3053	                                h.relative_path
  3054	                            )));
  3055	                        }
  3056	                    }
  3057	                }
  3058	                // Capture member paths for the per-file progress lane
  3059	                // before the record consumes the shard (the data-plane
  3060	                // receive does the same); skip the allocation when no one
  3061	                // is listening.
  3062	                let member_paths: Option<Vec<String>> = progress.as_ref().map(|_| {
  3063	                    shard
  3064	                        .files
  3065	                        .iter()
  3066	                        .map(|h| h.relative_path.clone())
  3067	                        .collect()
  3068	                });
  3069	                let outcome = receive_tar_record(transport, sink.as_ref(), shard).await?;
  3070	                files_written += outcome.files_written as u64;
  3071	                bytes_written += outcome.bytes_written;
  3072	                if let Some(p) = &progress {
  3073	                    p.report_payload(0, outcome.bytes_written);
  3074	                    for path in member_paths.unwrap_or_default() {
  3075	                        p.report_file_complete(path);
  3076	                    }
  3077	                }
  3078	            }
  3079	            Some(Frame::Resize(resize)) => {
  3080	                // sf-2 shape correction (otp-4b-2 push, otp-5b-2 pull): the

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '1160,1245p'; git grep -n \"NeedListSink\" 0f922de -- 'crates/**/*.rs'; git diff --stat 0f922de HEAD -- crates/blit-core/src/transfer_session/data_plane.rs; git log --oneline --all -- crates/blit-core/src/transfer_session/data_plane.rs | head -30" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1160	            ));
  1161	        }
  1162	        self.claim(path)
  1163	    }
  1164	}
  1165	
  1166	#[async_trait]
  1167	impl TransferSink for NeedListSink {
  1168	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
  1169	        match &payload {
  1170	            PreparedPayload::File(header) => {
  1171	                self.reject_resume_flagged(&header.relative_path)?;
  1172	                self.claim(&header.relative_path)?;
  1173	            }
  1174	            PreparedPayload::TarShard { headers, .. } => {
  1175	                for header in headers {
  1176	                    self.reject_resume_flagged(&header.relative_path)?;
  1177	                }
  1178	                for header in headers {
  1179	                    self.claim(&header.relative_path)?;
  1180	                }
  1181	            }
  1182	            // otp-7b: resume block records ride the data plane. A
  1183	            // mid-record block validates against its live grant (claimed
  1184	            // only at completion); the completion claims the grant, the
  1185	            // outstanding need, and the resumed count — all against the
  1186	            // same shared state the in-stream arms use inline. In a
  1187	            // non-resume session both are violations, never a
  1188	            // silently-applied patch.
  1189	            PreparedPayload::FileBlock {
  1190	                relative_path,
  1191	                offset,
  1192	                bytes,
  1193	            } => {
  1194	                self.check_block(relative_path, *offset, bytes.len() as u64)?;
  1195	            }
  1196	            PreparedPayload::FileBlockComplete {
  1197	                relative_path,
  1198	                total_size,
  1199	                ..
  1200	            } => {
  1201	                self.claim_block_complete(relative_path, *total_size)?;
  1202	                let path = relative_path.clone();
  1203	                let outcome = self
  1204	                    .inner
  1205	                    .write_payload(payload)
  1206	                    .await
  1207	                    .map_err(|e| super::tag_path(e, &path))?;
  1208	                // Count only after the finalization write landed —
  1209	                // the same ordering the in-stream arms follow.
  1210	                self.resume
  1211	                    .as_ref()
  1212	                    .expect("claim_block_complete verified resume is negotiated")
  1213	                    .resumed
  1214	                    .fetch_add(1, Ordering::Relaxed);
  1215	                return Ok(outcome);
  1216	            }
  1217	            // Send-side composite (otp-7b) — the wire never carries it,
  1218	            // so the receive pipeline can never produce one here.
  1219	            PreparedPayload::ResumeFile { .. } => {
  1220	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1221	                    "composite ResumeFile payload on the data-plane receive",
  1222	                )));
  1223	            }
  1224	        }
  1225	        // Tag the inner write's failure with the file it concerned
  1226	        // (otp-7b-2) where the payload names exactly one file.
  1227	        let tag: Option<String> = match &payload {
  1228	            PreparedPayload::File(h) => Some(h.relative_path.clone()),
  1229	            PreparedPayload::FileBlock { relative_path, .. } => Some(relative_path.clone()),
  1230	            _ => None,
  1231	        };
  1232	        match tag {
  1233	            Some(path) => self
  1234	                .inner
  1235	                .write_payload(payload)
  1236	                .await
  1237	                .map_err(|e| super::tag_path(e, &path)),
  1238	            None => self.inner.write_payload(payload).await,
  1239	        }
  1240	    }
  1241	
  1242	    async fn write_file_stream(
  1243	        &self,
  1244	        header: &FileHeader,
  1245	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HMhZfCA3' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-vUsiMqLT' (errno=Operation not permitted)
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:475:/// drains its socket into `sink` (a [`NeedListSink`], same strictness the
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:950:pub(super) struct NeedListSink {
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:959:impl NeedListSink {
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:1109:impl TransferSink for NeedListSink {
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:1244:        let sink = NeedListSink::new(Arc::new(NullSink::new()), Arc::clone(&outstanding), None);
0f922de:crates/blit-core/src/transfer_session/data_plane.rs:1311:        let sink = NeedListSink::new(
0f922de:crates/blit-core/src/transfer_session/mod.rs:2361:    // NeedListSink claims resume grants and counts completions as they
0f922de:crates/blit-core/src/transfer_session/mod.rs:2375:    // in-stream arms inline, the data-plane NeedListSink as payloads
0f922de:crates/blit-core/src/transfer_session/mod.rs:2390:    // NeedListSink gives the socket receive the same need-list strictness
0f922de:crates/blit-core/src/transfer_session/mod.rs:2397:    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
0f922de:crates/blit-core/src/transfer_session/mod.rs:2787:                // NeedListSink claims as payloads land, so joining the
0f922de:crates/blit-core/src/transfer_session/mod.rs:2813:                // above (in-stream inline, data-plane in NeedListSink),
0f922de:crates/blit-core/src/transfer_session/mod.rs:3177:/// active (with one, blocks ride the sockets and `NeedListSink` claims
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-UgnLSciG' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-3DnUJn63' (errno=Operation not permitted)
 .../blit-core/src/transfer_session/data_plane.rs   | 108 ++++++++++++++++-----
 1 file changed, 83 insertions(+), 25 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uBNytj6s' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uuLwTe0z' (errno=Operation not permitted)
805e48c otp-11b: delete the local orchestration (the last old path)
7aac28b otp-10c-2: delete the four drivers and the Push/PullSync RPCs
2014782 otp-10b-2: pull verb rides the unified session
6b292ed otp-10a: address review (8 findings)
0fbc966 otp-10a: push-shaped verb rides the unified session
071799a otp-7b-2: end-of-op fault summary rider + cancel-during-resume e2e
ecac9b0 otp-7b-1: resume block phase over the TCP data plane
d579365 otp-5b-2: pull data-plane resize (accept-based epoch-N socket)
e6a0b3b otp-5b-1: single-stream SOURCE-responder data plane (transport/role decoupling)
dce56de otp-4b-2: resize + multi-stream + sf-2 pin on the session
e1aafcc otp-4b-1: address review (2 findings)
881d412 otp-4b-1: TCP data plane onto the session (single stream)

2026-07-12T21:04:32.853731Z ERROR codex_core::tools::router: error=timeout_ms must be at least 10000
collab: Wait
exec
/bin/zsh -lc "git diff -- docs/plan/OTP12_PERF_FINDINGS.md; git status --short docs/plan/OTP12_PERF_FINDINGS.md; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1345,1435p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1748,1815p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '3075,3150p'; nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '630,655p'; nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '890,970p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-c3Q4OHrP' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-NbDQ46Sb' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-P01wtUnW' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Yb5r9fEe' (errno=Operation not permitted)
  1345	    // (the push verb, `blit move`'s source-delete gate) supply their own
  1346	    // accumulator via `SourceInstruments` and inspect it after the
  1347	    // session returns; the wire behavior is identical either way.
  1348	    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
  1349	    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
  1350	    while let Some(header) = header_rx.recv().await {
  1351	        sent.lock()
  1352	            .expect("sent-manifest lock poisoned")
  1353	            .insert(header.relative_path.clone(), header.clone());
  1354	        tx.send(frame(Frame::ManifestEntry(header))).await?;
  1355	        // Faults detected by the receive half abort the stream now,
  1356	        // not after the full scan; needs just accumulate. (Resize acks
  1357	        // cannot arrive yet — none is proposed before the payload phase.)
  1358	        drain_ready_source_events(
  1359	            &mut events,
  1360	            &mut pending,
  1361	            &mut resume,
  1362	            &mut need_complete,
  1363	            &mut needed_bytes,
  1364	            &mut needed_count,
  1365	            data_plane.as_ref(),
  1366	            tx,
  1367	            &mut pending_resize,
  1368	        )
  1369	        .await?;
  1370	    }
  1371	    let scanned = scan_handle
  1372	        .await
  1373	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
  1374	    let scan_complete = unreadable
  1375	        .lock()
  1376	        .expect("unreadable list lock poisoned")
  1377	        .is_empty();
  1378	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
  1379	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
  1380	        scan_complete,
  1381	    })))
  1382	    .await?;
  1383	    manifest_sent.store(true, Ordering::Release);
  1384	
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch (one ADD per epoch; a no-op while one is in
  1417	                    // flight or the shape wants no more).
  1418	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1419	                        .await?;
  1420	                    let payloads =
  1421	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1422	                    // A cancel while earlier batches are actively moving
  1423	                    // closes the send pipeline under backpressure, so this
  1424	                    // queue fails with a data-plane error — prefer the
  1425	                    // peer's framed reason (CANCELLED) the same way the
  1426	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1427	                    // against events like finish(): live `Need`s still
  1428	                    // arrive here, and `recv_peer_fault` would consume them.
  1429	                    if let Err(dp_err) = dp.queue(payloads).await {
  1430	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1431	                    }
  1432	                }
  1433	                None => {
  1434	                    // codex otp-8 F1: race the record sends against the
  1435	                    // receive half's fault signal — the in-stream twin of
  1748	                        resume.held.len()
  1749	                    ),
  1750	                )));
  1751	            }
  1752	            *need_complete = true;
  1753	            Ok(())
  1754	        }
  1755	        SourceEvent::ResizeAck(ack) => {
  1756	            let dp = data_plane.ok_or_else(|| {
  1757	                eyre::Report::new(SessionFault::protocol_violation(
  1758	                    "DataPlaneResizeAck on a session with no data plane",
  1759	                ))
  1760	            })?;
  1761	            // Match the ack to the in-flight proposal; stale/unsolicited
  1762	            // acks (wrong epoch, or none pending) are ignored, matching
  1763	            // old push. `take()` + restore keeps the borrow simple.
  1764	            let pending_r = match pending_resize.take() {
  1765	                Some(p) if p.epoch == ack.epoch => p,
  1766	                restored => {
  1767	                    *pending_resize = restored;
  1768	                    return Ok(());
  1769	                }
  1770	            };
  1771	            if ack.accepted {
  1772	                dp.add_stream(&pending_r.sub_token).await?;
  1773	                dp.dial()
  1774	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1775	            } else {
  1776	                dp.dial()
  1777	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1778	            }
  1779	            // Ramp one stream per accepted epoch: propose the next ADD.
  1780	            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1781	        }
  1782	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1783	            "TransferSummary before SourceDone",
  1784	        ))),
  1785	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1786	    }
  1787	}
  1788	
  1789	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1790	/// the stream count the accumulated need list implies, if none is in
  1791	/// flight. A no-op when the shape wants no more than the live count (the
  1792	/// dial returns `None`). Sends the frame and records the in-flight
  1793	/// proposal for the ack to match.
  1794	async fn maybe_propose_resize(
  1795	    dp: &data_plane::SourceDataPlane,
  1796	    tx: &mut Box<dyn FrameTx>,
  1797	    needed_bytes: u64,
  1798	    needed_count: usize,
  1799	    pending_resize: &mut Option<data_plane::PendingResize>,
  1800	) -> Result<()> {
  1801	    if pending_resize.is_some() {
  1802	        return Ok(());
  1803	    }
  1804	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1805	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1806	            op: DataPlaneResizeOp::Add as i32,
  1807	            epoch: proposal.epoch,
  1808	            target_stream_count: proposal.target_streams,
  1809	            sub_token: proposal.sub_token.clone(),
  1810	        })))
  1811	        .await?;
  1812	        *pending_resize = Some(proposal);
  1813	    }
  1814	    Ok(())
  1815	}
  3075	                        p.report_file_complete(path);
  3076	                    }
  3077	                }
  3078	            }
  3079	            Some(Frame::Resize(resize)) => {
  3080	                // sf-2 shape correction (otp-4b-2 push, otp-5b-2 pull): the
  3081	                // SOURCE proposes one ADD; the DESTINATION grows its receive
  3082	                // set (bump `resize_live`) and acks so the SOURCE completes
  3083	                // the epoch-N socket. The control-lane frames are identical
  3084	                // in both directions — only the transport action flips: a
  3085	                // DESTINATION **responder** (push) ARMS a credential its
  3086	                // accept loop then accepts; a DESTINATION **initiator**
  3087	                // (pull) DIALS the epoch-N socket itself. Only ADD occurs
  3088	                // (REMOVE is a tuner concern, future work); anything else
  3089	                // fails fast.
  3090	                if data_plane_recv.is_none() {
  3091	                    return Err(violation(
  3092	                        "DataPlaneResize on a session with no data plane".into(),
  3093	                    ));
  3094	                }
  3095	                let op = DataPlaneResizeOp::try_from(resize.op)
  3096	                    .unwrap_or(DataPlaneResizeOp::Unspecified);
  3097	                if op != DataPlaneResizeOp::Add {
  3098	                    return Err(violation(format!(
  3099	                        "unsupported data-plane resize op {}",
  3100	                        op.as_str_name()
  3101	                    )));
  3102	                }
  3103	                if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
  3104	                    return Err(violation(
  3105	                        "DataPlaneResize sub_token must be 16 bytes".into(),
  3106	                    ));
  3107	                }
  3108	                // Cumulative ceiling bound (defense in depth — the source's
  3109	                // dial already clamps to the same profile). Under the
  3110	                // ceiling, grow per connection role: arm the credential
  3111	                // (responder) or dial the epoch-N socket (initiator). A
  3112	                // dial failure is fatal (`add_dialed_stream`); a gone accept
  3113	                // loop returns false (arm). The initiator dials BEFORE the
  3114	                // ack so the SOURCE responder — which accepts on the ack —
  3115	                // never commits to an accept the DESTINATION did not dial.
  3116	                let accepted = if resize_live < resize_ceiling {
  3117	                    match data_plane_recv
  3118	                        .as_mut()
  3119	                        .expect("data plane present (checked above)")
  3120	                    {
  3121	                        data_plane::DestRecvPlane::Responder(run) => {
  3122	                            run.arm(resize.sub_token.clone())
  3123	                        }
  3124	                        data_plane::DestRecvPlane::Initiator(run) => {
  3125	                            run.add_dialed_stream(&resize.sub_token).await?;
  3126	                            true
  3127	                        }
  3128	                    }
  3129	                } else {
  3130	                    false
  3131	                };
  3132	                if accepted {
  3133	                    resize_live += 1;
  3134	                }
  3135	                let effective = if accepted {
  3136	                    resize.target_stream_count
  3137	                } else {
  3138	                    resize_live as u32
  3139	                };
  3140	                transport
  3141	                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
  3142	                        epoch: resize.epoch,
  3143	                        effective_stream_count: effective,
  3144	                        accepted,
  3145	                    })))
  3146	                    .await?;
  3147	            }
  3148	            Some(Frame::SourceDone(_)) => {
  3149	                if !manifest_complete {
  3150	                    return Err(violation("SourceDone before ManifestComplete".into()));
   630	pub(super) struct PendingResize {
   631	    pub(super) epoch: u32,
   632	    pub(super) target_streams: u32,
   633	    pub(super) sub_token: Vec<u8>,
   634	}
   635	
   636	/// How the SOURCE acquires each epoch-N data socket for a shape resize —
   637	/// the two connection roles of otp-5b. Byte direction is identical (the
   638	/// SOURCE sends), and `propose_resize` is the same either way; only socket
   639	/// acquisition flips.
   640	enum SourceSockets {
   641	    /// SOURCE **initiator** (push, otp-4b-2): dials each epoch-N socket to
   642	    /// the granted host:port.
   643	    Dial { host: String, tcp_port: u32 },
   644	    /// SOURCE **responder** (pull, otp-5b-2): accepts each epoch-N socket
   645	    /// off the listener it already bound for epoch-0, credential
   646	    /// `session_token ‖ sub_token`.
   647	    Accept { listener: TcpListener },
   648	}
   649	
   650	/// A running source-side data plane: the dialed/accepted socket(s) wrapped
   651	/// as an ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the
   652	/// sf-2 shape correction). Planned payloads are fed via [`Self::queue`];
   653	/// closing via [`Self::finish`] drains the pipeline, emits each socket's
   654	/// END record, and returns the bytes this end sent.
   655	pub(super) struct SourceDataPlane {
   890	        }))
   891	    }
   892	
   893	    /// Acquire the epoch-N data socket for an accepted resize and hand it
   894	    /// to the running pipeline (`SinkControl::Add`). The SOURCE initiator
   895	    /// (push) DIALS it; the SOURCE responder (pull, otp-5b-2) ACCEPTS the
   896	    /// socket the DESTINATION initiator dials after its ack, off the same
   897	    /// listener epoch-0 came in on. A dial/accept failure is FATAL
   898	    /// (fail-fast): a same-build peer that established epoch-0 failing an
   899	    /// epoch-N socket is a transport fault worth surfacing — and faulting
   900	    /// the session aborts the peer's counterpart via AbortOnDrop, so no
   901	    /// slot orphans. (Old push recovers non-fatally via an arm TTL; the
   902	    /// session trades that for simplicity — noted in the finding doc.) If
   903	    /// the pipeline is already gone (transfer completing under the ADD),
   904	    /// the just-acquired socket is closed cleanly so the peer's worker sees
   905	    /// its END, not a reset.
   906	    ///
   907	    /// The accept is bounded and unambiguous: at most one resize is in
   908	    /// flight (the driver's `pending_resize`) and epoch-0 is already
   909	    /// accepted, so the next connection off the listener is exactly this
   910	    /// resize's socket — verified against `session_token ‖ sub_token`.
   911	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   912	        let session = match &self.sockets {
   913	            SourceSockets::Dial { host, tcp_port } => {
   914	                let mut handshake = self.session_token.clone();
   915	                handshake.extend_from_slice(sub_token);
   916	                DataPlaneSession::connect(
   917	                    host,
   918	                    *tcp_port,
   919	                    &handshake,
   920	                    self.dial.chunk_bytes(),
   921	                    self.dial.prefetch_count(),
   922	                    self.trace,
   923	                    self.dial.tcp_buffer_bytes(),
   924	                    Arc::clone(&self.pool),
   925	                )
   926	                .await
   927	                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?
   928	            }
   929	            SourceSockets::Accept { listener } => {
   930	                let mut expected = self.session_token.clone();
   931	                expected.extend_from_slice(sub_token);
   932	                let socket = accept_authenticated(listener, &expected).await?;
   933	                DataPlaneSession::from_stream(
   934	                    socket,
   935	                    self.trace,
   936	                    self.dial.chunk_bytes(),
   937	                    self.dial.prefetch_count(),
   938	                    Arc::clone(&self.pool),
   939	                )
   940	                .await
   941	            }
   942	        };
   943	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   944	            session,
   945	            Arc::clone(&self.source),
   946	            PathBuf::new(),
   947	        ));
   948	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   949	            if let SinkControl::Add(sink) = returned.0 {
   950	                let _ = sink.finish().await;
   951	            }
   952	        }
   953	        Ok(())
   954	    }
   955	
   956	    /// Feed one planned batch into the send pipeline. The pipeline
   957	    /// prepares each payload (tar-shard/file) and writes it through the
   958	    /// data-plane record framing across the live socket(s).
   959	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   960	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   961	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   962	        })?;
   963	        for payload in payloads {
   964	            tx.send(payload).await.map_err(|_| {
   965	                dp_fault("data-plane send pipeline closed before all payloads sent")
   966	            })?;
   967	        }
   968	        Ok(())
   969	    }
   970	

exec
/bin/zsh -lc "git show 0f922de:crates/blit-daemon/src/service/push/data_plane.rs | nl -ba | sed -n '1060,1100p'; git show 0f922de:crates/blit-daemon/src/service/push/data_plane.rs | nl -ba | sed -n '840,885p'; git show 0f922de:crates/blit-daemon/src/service/push/data_plane.rs | nl -ba | sed -n '1020,1065p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-SW1rIf3y' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Bvn7KBIk' (errno=Operation not permitted)
  1060	
  1061	    // Only return buffer for pooling if it matches pool size. We
  1062	    // never moved ownership into an Archive, so the buffer is intact.
  1063	    let return_buffer = if buffer_capacity >= pool_buffer_size {
  1064	        Some(buffer)
  1065	    } else {
  1066	        None
  1067	    };
  1068	
  1069	    Ok((stats, return_buffer))
  1070	}
  1071	
  1072	/// audit-h3a: wrap the push-receive socket in a `StallGuard` so a peer
  1073	/// that accepts the data plane and then stops sending bytes is reaped
  1074	/// by `TRANSFER_STALL_TIMEOUT` rather than holding the receive worker
  1075	/// open forever. Symmetric with the CLI pull-receive guard in
  1076	/// `blit_core::remote::pull` (audit-1c).
  1077	///
  1078	/// Extracted from `handle_data_plane_stream` so the wiring is unit-
  1079	/// testable without spinning up a TcpListener + token handshake — see
  1080	/// `receive_push_data_plane_aborts_on_stall` in the tests module.
  1081	async fn receive_push_data_plane<R: AsyncRead + Unpin + Send>(
  1082	    socket: R,
  1083	    sink: Arc<dyn TransferSink>,
  1084	) -> Result<SinkOutcome> {
  1085	    let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
  1086	    execute_receive_pipeline(&mut guarded, sink, None).await
  1087	}
  1088	
  1089	#[cfg(test)]
  1090	mod tests {
  1091	    use super::*;
  1092	    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
  1093	    use std::path::Path;
  1094	    use tar::{Builder, EntryType, Header};
  1095	    use tempfile::tempdir;
  1096	
  1097	    /// audit-3b: `generate_token` returns `Ok` with a full-length token
  1098	    /// under normal conditions (the OS RNG path), and successive tokens
  1099	    /// differ. The failure arm (`Status::Internal`) is unreachable
  1100	    /// without injecting a failing RNG — out of proportion to the fix,
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GWeJNq28' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XKQlO26F' (errno=Operation not permitted)
   840	            tcp_fallback: true,
   841	            stream_count: 0,
   842	            // ue-r2-1b: fallback = control-plane bytes; no capacity
   843	            // profile and never resizable.
   844	            receiver_capacity: None,
   845	            resize_enabled: false,
   846	            epoch0_sub_token: Vec::new(),
   847	        }),
   848	    )
   849	    .await?;
   850	
   851	    let stats = receive_fallback_data(stream, module, files_requested).await?;
   852	
   853	    Ok(stats)
   854	}
   855	
   856	/// Tar-shard work scheduler used **only** by the gRPC fallback
   857	/// receive path (`receive_fallback_data`). The TCP push receive path
   858	/// routes through `FsTransferSink::write_tar_shard_payload`
   859	/// (rayon-parallel via the shared buffer pool) and does not touch
   860	/// this struct.
   861	///
   862	/// POST_REVIEW_FIXES §1.2 considered deleting this in favor of
   863	/// having the gRPC fallback also call `FsTransferSink::write_payload`,
   864	/// unifying the buffer pool. Two reasons it stays for 0.1.0:
   865	///
   866	/// 1. The gRPC fallback path streams chunked `TarShardHeader` →
   867	///    `TarShardChunk` messages and applies them when
   868	///    `TarShardComplete` arrives — different framing than the TCP
   869	///    path's `apply_tar_shard` over a single contiguous buffer.
   870	///    Routing the fallback through `write_payload` would require
   871	///    threading a streaming-tar receiver into the sink shape, which
   872	///    is a meaningful refactor of the receive abstraction.
   873	/// 2. The dedicated buffer pool sized for fallback's typical
   874	///    chunking (TAR_BUFFER_SIZE × TAR_BUFFER_POOL_SIZE) is the kind
   875	///    of tuning that benefits from running the unification with
   876	///    real fallback throughput numbers in hand. We don't have those
   877	///    yet.
   878	///
   879	/// Plan after 0.1.0: collapse this into the unified sink path once
   880	/// the gRPC-fallback streaming receiver lives behind a small
   881	/// `TarShardReceiver` trait. Tracked as deferred technical debt;
   882	/// see `docs/plan/POST_REVIEW_FIXES.md` §1.2.
   883	struct TarShardExecutor {
   884	    semaphore: Arc<Semaphore>,
   885	    tasks: JoinSet<Result<(TransferStats, Option<Vec<u8>>), Status>>,
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-kuTvUsno' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-yV6P5wiU' (errno=Operation not permitted)
  1020	/// receive policy here matches the pull-receive sites bit-for-bit.
  1021	/// Critically, this closes the latent High-severity equivalent of
  1022	/// R5-F2 on the push direction: the previous `Entry::unpack` call
  1023	/// honored tar symlink/hardlink/device entries, letting an
  1024	/// authenticated push client place a symlink at a benign-looking
  1025	/// path that subsequent writes would follow outside the module root.
  1026	/// The shared helper rejects any non-regular entry up front.
  1027	fn apply_tar_shard_sync(
  1028	    module: ModuleConfig,
  1029	    headers: Vec<FileHeader>,
  1030	    buffer: Vec<u8>,
  1031	    pool_buffer_size: usize,
  1032	) -> Result<(TransferStats, Option<Vec<u8>>), Status> {
  1033	    use blit_core::remote::transfer::tar_safety::{
  1034	        safe_extract_tar_shard, write_extracted_file, TarShardExtractOptions,
  1035	    };
  1036	
  1037	    let buffer_capacity = buffer.capacity();
  1038	    let opts = TarShardExtractOptions::default();
  1039	    let extracted = safe_extract_tar_shard(&buffer, headers, &module.path, &opts)
  1040	        .map_err(|err| Status::internal(format!("tar shard validation: {err:#}")))?;
  1041	
  1042	    let mut stats = TransferStats::default();
  1043	    for file in &extracted {
  1044	        // F2: containment check on every entry's destination before
  1045	        // writing. The tar_safety helper only does lexical safe_join;
  1046	        // an existing symlink at a parent component (placed by a
  1047	        // previous push) would otherwise have create_dir_all/write
  1048	        // follow it outside the module root. R5-F2 already rejects
  1049	        // tar entries whose tar header type is Symlink/Hardlink, so
  1050	        // this defends against pre-existing symlinks on disk, not
  1051	        // tar-encoded ones. Check against canonical_root, not path,
  1052	        // because path may be munged with a destination subpath.
  1053	        blit_core::path_safety::verify_contained(&module.canonical_root, &file.dest_path)
  1054	            .map_err(|err| Status::permission_denied(format!("path containment: {err:#}")))?;
  1055	        write_extracted_file(file)
  1056	            .map_err(|err| Status::internal(format!("applying tar shard entry: {err:#}")))?;
  1057	        stats.files_transferred += 1;
  1058	        stats.bytes_transferred += file.size;
  1059	    }
  1060	
  1061	    // Only return buffer for pooling if it matches pool size. We
  1062	    // never moved ownership into an Archive, so the buffer is intact.
  1063	    let return_buffer = if buffer_capacity >= pool_buffer_size {
  1064	        Some(buffer)
  1065	    } else {

codex
Basis: current working-tree file matches local HEAD `9c7b00e`; GitHub freshness could not be checked because network access was unavailable.

- **HIGH — Fix criteria still omit cross-direction converge-up.** [OTP12_PERF_FINDINGS.md:128](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:128) enforces each arm against its own two references, but not the parent’s `min(old_push, old_pull) × 1.10` ceiling at [ONE_TRANSFER_PATH.md:123](/Users/michael/Dev/blit_v2/docs/plan/ONE_TRANSFER_PATH.md:123) and [OTP12_ACCEPTANCE_RUN.md:163](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:163). Equal slowing of both layouts could pass invariance and per-direction bars while violating converge-up. Require every final cross row to pass or satisfy the registered platform-residue discriminator. “Necessary, not sufficient” also conflicts with the subsequent `fixed ⇔` wording.

- **MEDIUM — H1: SUPPORTED structurally, but pf-1 cannot falsify it as staged.** Dial/accept ownership really flips at [data_plane.rs:636](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:636), and destination dial-before-ACK versus responder arm-before-ACK differs at [mod.rs:3079](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3079). But [OTP12_PERF_FINDINGS.md:105](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:105) explicitly says local absence does not kill H1, while pf-1 requires no rig-side fallback. Require an instrumented netwatch run when local reproduction is negative.

- **INFO — H2: CONTRADICTED; the revision is adequate.** Both layouts use fixed 128-entry destination chunks, while resize is not proposed until after `ManifestComplete` ([mod.rs:1350](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1350), [mod.rs:1385](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1385)). The label at [OTP12_PERF_FINDINGS.md:55](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:55) is correct.

- **MEDIUM — H3: CONTRADICTED as written, while a nearby TCP-only mechanism is missed.** The retained directory/open/write work at [OTP12_PERF_FINDINGS.md:63](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:63) uses the same tar sink as old push; old `0f922de` directly ran `execute_receive_pipeline → FsTransferSink` (`crates/blit-daemon/src/service/push/data_plane.rs:184-213`), and the tar writer remains the same at [sink.rs:528](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:528). However, current TCP receive adds `NeedListSink`, taking a separate mutex/hash-set claim per tar member ([data_plane.rs:1167](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:1167)); current gRPC claims a whole shard under one lock ([mod.rs:3047](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3047)), and old TCP paid neither. Add this as an explicit H6/probe point.

- **MEDIUM — H4: SUPPORTED, but remains causally unfalsifiable.** Current code proposes one resize, then awaits the whole bounded payload enqueue before handling its ACK and proposing the next ([mod.rs:1410](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1410), [data_plane.rs:956](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:956)); old push returned to its response/resize loop after each need batch. Dropping the initial-count pin is correct, but [OTP12_PERF_FINDINGS.md:113](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:113) now has no cadence counterfactual. Add an old-cadence/shard-boundary replay or ACK-processing toggle. Also, record framing lives in `remote/transfer/data_plane.rs`, not `dial.rs` as line 75 claims.

- **MEDIUM — H5 is the right fifth mechanism, but its metric is underdefined.** Old first payload may precede `ManifestComplete`, so “manifest-complete→first-payload gap” at [OTP12_PERF_FINDINGS.md:118](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:118) needs signed common-clock timestamps and an exact event definition: first queued payload, first socket write, and ideally first receive.

- **MEDIUM — The historical control lacks “the same instrumentation.”** The targeted spans proposed at [OTP12_PERF_FINDINGS.md:94](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:94) do not exist in the pinned `0f922de` binary. Specify either external observation or a recorded minimal probe backport with aligned event semantics, plus instrumentation-on/off controls; per-member tracing across 10k files can perturb a roughly 10% gap.

- **MEDIUM — Windows gRPC-small was not at parity.** [OTP12_PERF_FINDINGS.md:34](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:34) says “0.98-ish,” but committed results are 2357/2942 = **0.801** same-session and 2357/2822 = **0.835** committed ([verdicts.csv:26](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/verdicts.csv:26)). Say “TCP regressed while gRPC did not; Zoey gRPC was at parity and Windows improved materially.” P1’s numbers and Zoey’s 0.966 converge-up-only characterization are accurate.

- **MEDIUM — “Every row … comes from the final build” is impossible.** [OTP12_PERF_FINDINGS.md:159](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:159) must say every **new/unified arm** comes from the final fix build; the required same-session and committed references necessarily remain old builds.

- **INFO — Staging and conditional wire scope are otherwise adequate.** pf-1-before-fix is a real hard gate at [OTP12_PERF_FINDINGS.md:151](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:151). CELLS runs are sufficient only as intermediate confirmation; pf-final correctly requires complete two-rig matrices before otp-13. H1 internal scheduling, H3/H6, and sequential H4 fixes can be wire-neutral; dial-topology changes, multi-ADD pipelining, or H5 overlap restoration must trigger the documented stop-and-amend rule.

**VERDICT: REVISE — prior-round corrections mostly landed, but cross-direction acceptance, H1/H4 falsifiability, historical instrumentation, and the omitted TCP need-claim mechanism must be tightened before the plan is converged.**
tokens used
211,736
Basis: current working-tree file matches local HEAD `9c7b00e`; GitHub freshness could not be checked because network access was unavailable.

- **HIGH — Fix criteria still omit cross-direction converge-up.** [OTP12_PERF_FINDINGS.md:128](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:128) enforces each arm against its own two references, but not the parent’s `min(old_push, old_pull) × 1.10` ceiling at [ONE_TRANSFER_PATH.md:123](/Users/michael/Dev/blit_v2/docs/plan/ONE_TRANSFER_PATH.md:123) and [OTP12_ACCEPTANCE_RUN.md:163](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:163). Equal slowing of both layouts could pass invariance and per-direction bars while violating converge-up. Require every final cross row to pass or satisfy the registered platform-residue discriminator. “Necessary, not sufficient” also conflicts with the subsequent `fixed ⇔` wording.

- **MEDIUM — H1: SUPPORTED structurally, but pf-1 cannot falsify it as staged.** Dial/accept ownership really flips at [data_plane.rs:636](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:636), and destination dial-before-ACK versus responder arm-before-ACK differs at [mod.rs:3079](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3079). But [OTP12_PERF_FINDINGS.md:105](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:105) explicitly says local absence does not kill H1, while pf-1 requires no rig-side fallback. Require an instrumented netwatch run when local reproduction is negative.

- **INFO — H2: CONTRADICTED; the revision is adequate.** Both layouts use fixed 128-entry destination chunks, while resize is not proposed until after `ManifestComplete` ([mod.rs:1350](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1350), [mod.rs:1385](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1385)). The label at [OTP12_PERF_FINDINGS.md:55](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:55) is correct.

- **MEDIUM — H3: CONTRADICTED as written, while a nearby TCP-only mechanism is missed.** The retained directory/open/write work at [OTP12_PERF_FINDINGS.md:63](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:63) uses the same tar sink as old push; old `0f922de` directly ran `execute_receive_pipeline → FsTransferSink` (`crates/blit-daemon/src/service/push/data_plane.rs:184-213`), and the tar writer remains the same at [sink.rs:528](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:528). However, current TCP receive adds `NeedListSink`, taking a separate mutex/hash-set claim per tar member ([data_plane.rs:1167](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:1167)); current gRPC claims a whole shard under one lock ([mod.rs:3047](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3047)), and old TCP paid neither. Add this as an explicit H6/probe point.

- **MEDIUM — H4: SUPPORTED, but remains causally unfalsifiable.** Current code proposes one resize, then awaits the whole bounded payload enqueue before handling its ACK and proposing the next ([mod.rs:1410](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1410), [data_plane.rs:956](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:956)); old push returned to its response/resize loop after each need batch. Dropping the initial-count pin is correct, but [OTP12_PERF_FINDINGS.md:113](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:113) now has no cadence counterfactual. Add an old-cadence/shard-boundary replay or ACK-processing toggle. Also, record framing lives in `remote/transfer/data_plane.rs`, not `dial.rs` as line 75 claims.

- **MEDIUM — H5 is the right fifth mechanism, but its metric is underdefined.** Old first payload may precede `ManifestComplete`, so “manifest-complete→first-payload gap” at [OTP12_PERF_FINDINGS.md:118](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:118) needs signed common-clock timestamps and an exact event definition: first queued payload, first socket write, and ideally first receive.

- **MEDIUM — The historical control lacks “the same instrumentation.”** The targeted spans proposed at [OTP12_PERF_FINDINGS.md:94](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:94) do not exist in the pinned `0f922de` binary. Specify either external observation or a recorded minimal probe backport with aligned event semantics, plus instrumentation-on/off controls; per-member tracing across 10k files can perturb a roughly 10% gap.

- **MEDIUM — Windows gRPC-small was not at parity.** [OTP12_PERF_FINDINGS.md:34](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:34) says “0.98-ish,” but committed results are 2357/2942 = **0.801** same-session and 2357/2822 = **0.835** committed ([verdicts.csv:26](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/verdicts.csv:26)). Say “TCP regressed while gRPC did not; Zoey gRPC was at parity and Windows improved materially.” P1’s numbers and Zoey’s 0.966 converge-up-only characterization are accurate.

- **MEDIUM — “Every row … comes from the final build” is impossible.** [OTP12_PERF_FINDINGS.md:159](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:159) must say every **new/unified arm** comes from the final fix build; the required same-session and committed references necessarily remain old builds.

- **INFO — Staging and conditional wire scope are otherwise adequate.** pf-1-before-fix is a real hard gate at [OTP12_PERF_FINDINGS.md:151](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:151). CELLS runs are sufficient only as intermediate confirmation; pf-final correctly requires complete two-rig matrices before otp-13. H1 internal scheduling, H3/H6, and sequential H4 fixes can be wire-neutral; dial-topology changes, multi-ADD pipelining, or H5 overlap restoration must trigger the documented stop-and-amend rule.

**VERDICT: REVISE — prior-round corrections mostly landed, but cross-direction acceptance, H1/H4 falsifiability, historical instrumentation, and the omitted TCP need-claim mechanism must be tightened before the plan is converged.**
