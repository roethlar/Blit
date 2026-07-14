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
session id: 019f5f44-6ee4-7042-ad59-36ae3769dc87
--------
user
Review the diff of commit d71c0ed (run: git show d71c0ed). It records owner decision D-2026-07-14-1: the committed acceptance baselines are RE-RECORDED with their original OLD builds at MTU 9000, then re-frozen, because pf-0 (docs/bench/otp12-jumbo-win-2026-07-13/README.md) measured jumbo making both arms 3-4 percent faster -- so grading a jumbo build against a 1500-recorded ceiling is lenient rather than conservative.

Check, hardest first:
(1) Is the stated rationale actually supported by the pf-0 data, or is the 3-4 percent claim being over-applied? Does re-recording at 9000 genuinely remove the leniency, or does it introduce a NEW confound (e.g. the old build re-run today on different hardware/OS state than 2026-07-10, so the new ceiling differs from the old one for reasons unrelated to MTU -- is that acknowledged)?
(2) Is the SCOPE right? It claims BOTH rig W and rig Z baselines are MTU-mismatched and rig D is unaffected. Verify against the harnesses (scripts/bench_otp12_win.sh, scripts/bench_otp12_zoey.sh, scripts/bench_otp12_delegated.sh) and .agents/machines.md (which records when each host went to MTU 9000). Was zoey actually at 1500 when otp2-baseline-2026-07-10 was recorded? If the premise is wrong for a rig, say so.
(3) Coherence with docs/DECISIONS.md (esp. D-2026-07-05-4, D-2026-07-12-1) and with OTP12_ACCEPTANCE_RUN.md D2/D5. Does calling this an amendment of the PIN but not the FREEZE actually hold, or is it a rationalization that weakens an anti-drift control?
(4) Does the decision leave any acceptance row ungraded or double-counted? Is retaining the 2026-07-10 baselines as historical (not deleting) handled coherently?
(5) Anything in the propagated edits (OTP12_PERF_FINDINGS section pf-0, OTP12_ACCEPTANCE_RUN D5 note, docs/STATE.md) that overstates, understates, or contradicts the decision text.

Output a concise markdown findings list -- each finding with file:line, severity, rationale -- then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the commit, governing repo state, benchmark evidence, and harness history, then cross-check the decision’s rationale, scope, and acceptance accounting. I’ll keep this read-only and return only actionable findings plus a verdict.
exec
/bin/zsh -lc "sed -n '1,260p' AGENTS.md && sed -n '1,280p' .agents/repo-guidance.md && sed -n '1,260p' .agents/state.md" in /Users/michael/Dev/blit_v2
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

exec
/bin/zsh -lc 'git status --short --branch && git remote -v && git rev-parse HEAD && git show --stat --oneline --decorate --no-renames d71c0ed && git show --format=fuller --no-ext-diff --no-renames --find-renames=0 d71c0ed' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZdjNvPIJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZHZF0nW5' (errno=Operation not permitted)
## master...origin/master [ahead 11]
?? .review/results/pf-0-rebaseline-decision.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-92esU4Xu' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-OIYSTsOh' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-xMIBeba5' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-cSiNcuWd' (errno=Operation not permitted)
d71c0edaa395fdd25756dbe6ba9a5cb272475666
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-fLiGI4EM' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-NU9ImG4v' (errno=Operation not permitted)
d71c0ed (HEAD -> master) decision D-2026-07-14-1: re-record the committed baselines at MTU 9000
 docs/DECISIONS.md                 | 10 +++++++++
 docs/STATE.md                     | 12 +++++------
 docs/plan/OTP12_ACCEPTANCE_RUN.md | 10 +++++++++
 docs/plan/OTP12_PERF_FINDINGS.md  | 45 ++++++++++++++++++++++-----------------
 4 files changed, 51 insertions(+), 26 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-wOx6XHJr' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-JmImgVRo' (errno=Operation not permitted)
commit d71c0edaa395fdd25756dbe6ba9a5cb272475666
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Tue Jul 14 02:15:23 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Tue Jul 14 02:15:23 2026 -0400

    decision D-2026-07-14-1: re-record the committed baselines at MTU 9000
    
    Owner, 2026-07-14, choosing between three presented options, verbatim:
    "Re-record the baseline at 9000".
    
    Why: pf-0 measured jumbo making BOTH arms 3-4% faster. The committed anti-drift
    ceilings were recorded at MTU 1500, before the fabric-wide jumbo raise. So
    grading a jumbo NEW arm against a 1500-recorded ceiling is LENIENT, not
    conservative -- the MTU gain flatters the ratio and a regression up to roughly
    the size of that gain could pass unseen. P1 is the one finding between blit and
    shipping; a lenient ceiling is the wrong error to accept there.
    
    Scope is BOTH rigs, not just rig W: each harness hardcodes its own reference and
    both predate the jumbo raise (rig W bench_otp12_win.sh:105 -> otp2w-baseline;
    rig Z bench_otp12_zoey.sh:102 -> otp2-baseline). Rig D has no old baseline.
    
    The FREEZE principle stands -- a baseline is immutable once recorded, and no run
    may re-point its own ceiling. Only the PIN moves, once: each rig re-records with
    its ORIGINAL old build at MTU 9000 and re-freezes. The 2026-07-10 baselines are
    retained as historical MTU-1500 records, never rewritten. BASELINE_SUMMARY stays
    override-free (a run must not be able to quietly re-point its own reference), so
    re-pointing is a reviewed source edit; and pf-0's start-AND-end MSS gate applies
    to the re-baseline sessions, since a baseline recorded at an unverified MTU is
    precisely the defect being fixed.
    
    Propagated: OTP12_PERF_FINDINGS §pf-0 OPEN -> RESOLVED; OTP12_ACCEPTANCE_RUN D5
    carries the amendment note where it asserts the freeze; STATE's next action is
    now pf-1 (the re-baseline is a pf-final prerequisite, not a pf-1 blocker).

diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 9e2056d..f0aea48 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -190,3 +190,13 @@ Format:
 - Sequencing: behind otp-12, and **planned together with `LOCAL_SMALL_FILE_PATH.md`** (D-2026-07-13-2) — they touch the same tar path and pull in opposite directions (a fidelity fix ADDS per-file work to a path already losing 1.9× to robocopy at equal thread count). Planning them separately would optimise one against the other.
 - Not in scope / not a bug: **empty directories**. Their absence is a documented design position — `blit check`'s help (`crates/blit-cli/src/cli.rs:20-35`) states the equivalence model skips empty directories and points at `diff -r` for full tree equivalence. blit models files, not directories. (`test_push_empty_directory` only asserts the command succeeds; it never checks the directory arrived — a crash smoke test, not a fidelity test.) **ACLs** are likewise out: robocopy does not copy them either without `/COPY:S`.
 - Supersedes: nothing. Adds `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` to the `docs/STATE.md` queue behind otp-12, alongside D-2026-07-13-2.
+
+## D-2026-07-14-1 — the committed baselines are RE-RECORDED at MTU 9000 (amends OTP12_ACCEPTANCE_RUN D5's pin, not its freeze)
+- Decision: the frozen committed baselines that `pf-final` grades against are **re-recorded with their OLD builds at MTU 9000**, so acceptance compares old and new like-for-like on the fabric the fleet actually runs. Owner, 2026-07-14, choosing between three presented options, verbatim: **"Re-record the baseline at 9000"**. The 2026-07-10 baselines are **retained as historical MTU-1500 records** — superseded as the acceptance reference, never deleted or rewritten.
+- Why: pf-0 (`docs/bench/otp12-jumbo-win-2026-07-13/`) measured jumbo making **both arms 3–4% faster**. Grading a jumbo NEW arm against a **1500-recorded** ceiling is therefore **LENIENT, not conservative** — the MTU gain flatters the ratio, so a real regression up to roughly the size of that gain could pass unseen. P1 is the one finding between blit and shipping; a lenient ceiling is the wrong error to accept there.
+- Scope — **BOTH rigs, not just rig W.** Each harness hardcodes its own committed reference, and both predate the 2026-07-13 fabric-wide jumbo raise (`.agents/machines.md`): rig W `scripts/bench_otp12_win.sh:105` → `docs/bench/otp2w-baseline-2026-07-10/`; rig Z `scripts/bench_otp12_zoey.sh:102` → `docs/bench/otp2-baseline-2026-07-10/`. Rig D (delegated) has **no** old baseline and is unaffected.
+- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
+  * **Each rig's re-baseline MUST use the same OLD build as its original baseline**, with provenance manifest-verified — rig W `0f922de`; rig Z the build staged in `blit-temp` (which embeds `731023bfc8a1.dirty`, **not** `e757dcc` — see the otp-2 README correction). A re-baseline on a different old build would silently change the reference twice.
+  * `BASELINE_SUMMARY` is hardcoded **by design** (no override) so a run cannot quietly re-point its own ceiling. Re-pointing it is therefore a reviewed source edit, not an env var — and the new value must be a **committed** dated dir.
+  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
+- Supersedes: the *pin* in `OTP12_ACCEPTANCE_RUN.md` D5 ("the frozen baselines stay frozen") — the **freeze principle stands** (a baseline is immutable once recorded, and no run may re-point its own reference), but the acceptance reference is re-recorded once, at the fabric's MTU, and re-frozen. Closes the OPEN item raised in `OTP12_PERF_FINDINGS.md` §pf-0.
diff --git a/docs/STATE.md b/docs/STATE.md
index b811238..2edff44 100644
--- a/docs/STATE.md
+++ b/docs/STATE.md
@@ -2,9 +2,10 @@
 
 Last updated: 2026-07-14
 
-- **NEXT ACTION — an OWNER DECISION, and it blocks pf-final's assembly: the frozen committed baseline is MTU-mismatched.** The fabric now runs **MTU 9000**; the committed anti-drift ceiling `docs/bench/otp2w-baseline-2026-07-10/summary.csv` was recorded at **MTU 1500**, and acceptance requires **BOTH** references (`OTP12_ACCEPTANCE_RUN.md` D2/D5, frozen by design). pf-0 measured jumbo making **both arms 3–4% faster**, so a jumbo NEW arm graded against a 1500 ceiling is **LENIENT, not conservative** — the MTU gain flatters the ratio and could let a real regression pass. Ways out (re-record the baseline at 9000 / run pf-final at 1500 / an explicit MTU-mismatch rule) each change the frozen contract or the rig config, so **each needs the owner's amendment — no agent may pick one.** Full exposure: `docs/plan/OTP12_PERF_FINDINGS.md` §pf-0. Then: **pf-1**.
+- **NEXT ACTION — `pf-1` (the HARD GATE): instrumentation + the interleaved counterfactuals.** Two pf-0 results now BIND it: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor), so pf-1 must **measure its own paired within-session noise floor on the unmodified build and register a resolution check** — smallest reportable recovery > that floor — *before* grading any hypothesis; (b) **the fast arm is BISTABLE**, so grade the run distribution, not the median. Design: `docs/plan/OTP12_PERF_FINDINGS.md` §Method + §pf-1 decision rule.
+- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
 - **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
-- **THE FAST ARM IS BISTABLE — a trap for pf-1.** `win_init` runs are **bimodal** (~730 ms and ~840 ms); S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. **A counterfactual that merely shifts the mixture would masquerade as a recovery: grade the run distribution, not the median.**
+- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
 - **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
 - **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
 - Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
@@ -193,8 +194,7 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 - **2026-07-13/14 (47th)** — P1 reproduces on a second Mac (`q`); new bench Mac;
   Windows attrs+ADS bug (D-2026-07-13-3); the robocopy headline was WRONG
   (D-2026-07-13-2); MTU prereg rev 1→4. Full: **DEVLOG 2026-07-14 00:15Z**.
-- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig built
-  (8/8 PASS → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1);
-  **three claims retracted, all from trusting an unvalidated instrument**.
-  Full entry: **DEVLOG 2026-07-13 20:00Z**.
+- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig (8/8 PASS
+  → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1); **three claims
+  retracted, all from unvalidated instruments**. Full: **DEVLOG 2026-07-13 20:00Z**.
 - *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*
diff --git a/docs/plan/OTP12_ACCEPTANCE_RUN.md b/docs/plan/OTP12_ACCEPTANCE_RUN.md
index a3c2804..0ad1375 100644
--- a/docs/plan/OTP12_ACCEPTANCE_RUN.md
+++ b/docs/plan/OTP12_ACCEPTANCE_RUN.md
@@ -289,6 +289,16 @@ direction); bench modules writable, `delegation_allowed` not narrowed.
 
 ### D5 — three self-contained scripts; the frozen baselines stay frozen
 
+> **AMENDED by D-2026-07-14-1 (2026-07-14) — the *pin* moves once; the *freeze*
+> stands.** The committed baselines this section pins were recorded at **MTU
+> 1500**, before the fabric-wide jumbo raise. pf-0 measured jumbo making both
+> arms 3–4% faster, so grading a jumbo build against a 1500 ceiling is **lenient,
+> not conservative**. Each rig's committed baseline is therefore **re-recorded
+> once with its ORIGINAL old build at MTU 9000** and re-frozen; the 2026-07-10
+> baselines are retained as historical MTU-1500 records. Immutability and the
+> no-override rule on `BASELINE_SUMMARY` are unchanged — see D-2026-07-14-1 and
+> `OTP12_PERF_FINDINGS.md` §pf-0.
+
 `scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
 `scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
 precedent: duplicate the shape, don't refactor recorded evidence;
diff --git a/docs/plan/OTP12_PERF_FINDINGS.md b/docs/plan/OTP12_PERF_FINDINGS.md
index beb241f..f40b372 100644
--- a/docs/plan/OTP12_PERF_FINDINGS.md
+++ b/docs/plan/OTP12_PERF_FINDINGS.md
@@ -343,26 +343,31 @@ the `q` pairing give **Δ_P1 ≈ 230 ms** (229 at 1500, 236 at 9000).
   verdict is robust to this: pooling all 16 runs per condition gives
   Δ_9000 = 232, Δ_1500 = 221.5, r = −4.7% — same KILLED grade.)
 
-**OPEN — pf-final's committed reference is MTU-mismatched (owner's amendment,
-NOT decided here).** The fabric now runs MTU 9000; the committed reference
-`docs/bench/otp2w-baseline-2026-07-10/summary.csv` was recorded at **MTU 1500**
-and is deliberately **frozen** as an anti-drift ceiling
-(`OTP12_ACCEPTANCE_RUN.md` D2/D5). Acceptance requires **both** references, so
-this plan must not quietly reinterpret the contract — the following is the
-exposure, stated for the owner, and this plan asserts no void rule of its own:
-
-- pf-0 measured jumbo making both arms **3–4% faster**. A jumbo NEW arm compared
-  against a **1500-recorded** ceiling is therefore **lenient, not conservative**
-  — the MTU gain flatters the ratio and could let a real regression pass. That
-  is the actual risk, and it argues the mismatch matters.
-- The ways out (re-recording the frozen baseline at 9000; running pf-final at
-  1500; or an explicit MTU-mismatch rule) each **change the frozen-baseline
-  contract or the rig configuration, and so require an owner amendment**.
-  Re-recording additionally needs a harness change (`bench_otp12_win.sh:105`
-  hardcodes `BASELINE_SUMMARY` with no override, by design).
-
-Same-session references (`old_session`) are MTU-matched by construction and are
-unaffected either way.
+**RESOLVED — the committed baselines are RE-RECORDED at MTU 9000
+(D-2026-07-14-1, owner, 2026-07-14).** The exposure pf-0 surfaced: the fabric now
+runs MTU 9000 while the committed anti-drift ceilings were recorded at **MTU
+1500**, and pf-0 measured jumbo making **both arms 3–4% faster** — so grading a
+jumbo NEW arm against a 1500-recorded ceiling is **LENIENT, not conservative**:
+the MTU gain flatters the ratio and a real regression could pass unseen.
+
+The owner's resolution is to **re-record each rig's committed baseline with its
+ORIGINAL OLD build at MTU 9000**, then re-freeze it. The freeze principle is
+unchanged (a baseline is immutable once recorded; no run may re-point its own
+ceiling) — only the *pin* moves, once. The 2026-07-10 baselines are retained as
+historical MTU-1500 records.
+
+**This is a prerequisite slice for `pf-final`, and it affects BOTH rigs** (each
+harness hardcodes its own reference, and both predate the fabric-wide jumbo
+raise): rig W `bench_otp12_win.sh:105` → `otp2w-baseline-2026-07-10/`; rig Z
+`bench_otp12_zoey.sh:102` → `otp2-baseline-2026-07-10/`. Rig D has no old
+baseline and is unaffected. Constraints (same old build per rig,
+manifest-verified; `BASELINE_SUMMARY` stays override-free and is re-pointed by a
+reviewed source edit; the pf-0 start-AND-end MSS gate applies, since a baseline
+recorded at an unverified MTU is the very defect being fixed) are in
+D-2026-07-14-1 and are not restated here.
+
+Same-session references (`old_session`) are MTU-matched by construction and were
+never at risk.
 
 ## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
 

exec
/bin/zsh -lc 'git ls-remote origin HEAD' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
e6845ca407ecced78b356b7923f82f388b3a431e	HEAD

exec
/bin/zsh -lc "find docs/bench/otp12-jumbo-win-2026-07-13 -maxdepth 2 -type f -print | sort; nl -ba docs/bench/otp12-jumbo-win-2026-07-13/README.md | sed -n '1,300p'; for f in docs/bench/otp12-jumbo-win-2026-07-13/*.csv; do echo \""'$f"; sed -n '"'1,30p' \""'$f"; done' in /Users/michael/Dev/blit_v2
 exited 1 in 0ms:
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md
docs/bench/otp12-jumbo-win-2026-07-13/README.md
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/bench.log
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/drain.log
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/load-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/load-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/meta.csv
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/mss-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/mss-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/runs.csv
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/session.log
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/staging-manifest.txt
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/summary.csv
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/verdicts.csv
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/bench.log
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/drain.log
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/load-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/load-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/meta.csv
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/mss-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/mss-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/runs.csv
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/session.log
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/staging-manifest.txt
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/summary.csv
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/verdicts.csv
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/bench.log
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/drain.log
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/load-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/load-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/meta.csv
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/mss-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/mss-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/runs.csv
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/session.log
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/staging-manifest.txt
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/summary.csv
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/verdicts.csv
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/bench.log
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/drain.log
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/load-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/load-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/meta.csv
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/mss-end.txt
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/mss-start.txt
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/runs.csv
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/session.log
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/staging-manifest.txt
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/summary.csv
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/verdicts.csv
     1	# MTU IS NOT THE CAUSE OF P1 — A-B-B-A jumbo experiment, rig `q` ↔ netwatch-01 (2026-07-14)
     2	
     3	**Status**: Evidence (recorded). This README applies the **pre-registered**
     4	decision rule in `PREREGISTRATION.md` to the data, and states nothing the rule
     5	does not license. **Provenance of the rule, stated precisely**: the decision
     6	rule, thresholds and guards were fixed in **rev 3**, before any of the S1–S4
     7	data existed, and rev 4 left them untouched (it re-described the *rig* after the
     8	`q` baseline). So no threshold was authored around these numbers — but "the
     9	document was written before any data existed" would be false, since a `q`
    10	baseline and a discarded A-B-B-A attempt preceded rev 4. It is
    11	**not** a plan amendment: per the pre-registration, a result here "licenses
    12	evidence for a plan amendment only" — killing the MTU hypothesis in
    13	`docs/plan/OTP12_PERF_FINDINGS.md` is a separate, reviewed change.
    14	
    15	**Design executed as registered**: four sessions **A-B-B-A** = 9000, 1500,
    16	1500, 9000, `RUNS=8`, `CELLS=wm_tcp_mixed,wm_tcp_large,mw_tcp_mixed,wm_grpc_mixed`,
    17	sha `f35702a` both ends, old arm `0f922de`, Mac end `q` (10.1.10.54, `en8`).
    18	**256 timed runs, 0 voided.**
    19	
    20	## Result — `r = −3.1%` → **KILLED as a material cause**
    21	
    22	| session | MTU | mac_init | win_init | **Δ** | ratio | invariance |
    23	|---|---:|---:|---:|---:|---:|---|
    24	| S1 | 9000 | 1035 | 760 | **275** | 1.362 | FAIL |
    25	| S2 | 1500 | 1071 | 830 | **241** | 1.290 | FAIL |
    26	| S3 | 1500 | 1066 | 849 | **217** | 1.256 | FAIL |
    27	| S4 | 9000 | 1029 | 832 | **197** | 1.237 | FAIL |
    28	
    29	    Δ_9000 = mean(275, 197) = 236 ms
    30	    Δ_1500 = mean(241, 217) = 229 ms
    31	    N_Δ    = max(|275−197|, |241−217|) = max(78, 24) = 78 ms   [measured noise floor]
    32	
    33	**Domain guard (evaluated first)**: `Δ_1500 (229) > N_Δ (78)` — the gap under
    34	study is present above this rig's own session-to-session noise, so the
    35	experiment is **in domain** and the recovery is computed.
    36	
    37	    r = (Δ_1500 − Δ_9000) / Δ_1500 = (229 − 236) / 229 = −3.1%
    38	
    39	On the parent plan's uniform pre-registered scale (`r < 20%`), that is
    40	**KILLED as a material cause**. Raising the MTU did not recover *any* of the
    41	gap; the point estimate is slightly negative (the gap was nominally *wider* at
    42	jumbo), but **|Δ_9000 − Δ_1500| = 7 ms is far inside the measured noise floor
    43	of 78 ms** — so the honest statement is not "jumbo made it worse" but **"the
    44	two conditions are indistinguishable: MTU has no measurable effect on Δ."**
    45	
    46	**Registered edge cases**: no INVERSION (`Δ_9000 = 236 > 0`); `r` not >100%;
    47	and `Δ_9000 (236) > N_Δ (78)`, so the residual gap is **not** inside the noise
    48	— P1 survives jumbo as a real, measurable asymmetry.
    49	
    50	**P1 fails in all four sessions** (1.237–1.362) regardless of MTU, by the
    51	harness's exact integer arithmetic (`10·hi ≤ 11·lo`), not the printed ratio.
    52	
    53	## ⚠ The resolution limit — this run cannot exclude a *contributing*-size effect
    54	
    55	The registered rule grades the **point estimate**, and the point estimate is ~0.
    56	But the experiment's own noise floor bounds what it could have seen:
    57	
    58	| effect size | in ms (of Δ_1500 = 229) | vs floor N_Δ = 78 ms | can this run exclude it? |
    59	|---|---:|---|---|
    60	| DOMINANT (`r ≥ 50%`) | ≥ 114 ms | comfortably above | **yes** |
    61	| CONTRIBUTING (`r ≥ 20%`) | ≥ 46 ms | **below the floor** | **NO** |
    62	
    63	So the honest scope of this null is: **jumbo is not a dominant cause of P1, and
    64	its measured contribution is indistinguishable from zero — but a
    65	contributing-size (~46 ms) MTU effect could be swamped by this rig's
    66	session-to-session noise and would not have been detected.** The KILLED grade
    67	stands as the pre-registered rule returns it; it must not be re-read as a
    68	stronger exclusion than that. (Pre-registration §"the noise model" fixed the
    69	floor as *measured*, not assumed — this is the price of that honesty, and it is
    70	stated rather than hidden.)
    71	
    72	## Where the noise actually comes from: the fast arm is BISTABLE
    73	
    74	The 78 ms floor is not diffuse jitter. The `win_init` runs are **bimodal** —
    75	one cluster near ~730 ms and one near ~840 ms — and the two same-MTU replicates
    76	simply drew different **mixtures** of the two modes:
    77	
    78	    S1 (9000) win_init: 699 715 750 753 767 776 | 843 844      -> 6 low, 2 high, median 760
    79	    S4 (9000) win_init: 752 755 | 825 828 836 837 838 860      -> 2 low, 6 high, median 832
    80	
    81	Same MTU, same build, same rig: the 72 ms gap between those medians is a
    82	**mode-mixture artifact**, and it is what sets N_Δ. The `mac_init` arm shows
    83	nothing of the kind (replicate medians differ by **5 and 6 ms**). This matches
    84	the local-rig bi-stability already recorded in
    85	`docs/bench/win-local-ab-2026-07-13/`.
    86	
    87	**Consequence for pf-1 (a trap):** a counterfactual that merely shifts the mode
    88	mixture would look exactly like a partial recovery. Grade on the run
    89	distribution, not the median alone.
    90	
    91	**The MTU verdict is robust to it.** Pooling all 16 runs per condition (instead
    92	of averaging session medians) gives `Δ_9000 = 232`, `Δ_1500 = 221.5`,
    93	**`r = −4.7%`** — the same KILLED grade.
    94	
    95	## The manipulation demonstrably reached the wire (the null is not vacuous)
    96	
    97	The most important defense of a null result is proof that the treatment was
    98	actually applied. Three independent instruments say it was:
    99	
   100	- **MSS gate, start AND end of every session** (the rev-4 requirement):
   101	  **8948/8948** in both jumbo sessions, **1448/1448** in both 1500 sessions.
   102	  No session is VOID on this gate.
   103	- **`wm_tcp_large` (registered as CONTEXT, never a gate)** got **3–4% faster at
   104	  jumbo on both arms** (mac_init 960→924 ms, win_init 945→916 ms). Jumbo does
   105	  real work on this path — it just does not touch the asymmetry.
   106	- **Both arms of `wm_tcp_mixed` also sped up slightly at jumbo** (mac 1068→1032,
   107	  win 840→796) while Δ stayed put. The benefit is **symmetric**, which is
   108	  precisely why it cannot explain an **asymmetry**.
   109	
   110	## Masking guard — the ratio did not improve, and no artifact is hiding a fix
   111	
   112	Rebuilt on the measured noise (`N_arm = 72 ms`, the largest same-MTU replicate
   113	difference across both arms). **Disclosure**: the pre-registration did not say
   114	how the two replicate medians become one condition-level value per arm; this
   115	analysis uses their **mean**. Every plausible alternative (either replicate
   116	alone, or the pooled runs) gives the same guard outcome, but "exactly as
   117	pre-registered" would overstate the spec's precision, so the choice is named
   118	here rather than left implicit.
   119	
   120	- **Fast-arm guard**: `win_init` at 9000 did **not** regress (−43.5 ms, i.e.
   121	  faster). OK.
   122	- **Convergence target**: `mac_9000 (1032) ≤ win_1500 (839.5) + N_arm (72) = 911.5`
   123	  → **NOT MET**. The slow arm did not approach the fast arm.
   124	- **Both-arms-slower (bottleneck compression)**: **False**.
   125	
   126	So there is no shared-floor artifact and no compression — there is simply **no
   127	fix**.
   128	
   129	## Controls (all four sessions, both conditions)
   130	
   131	| cell | S1 (9000) | S2 (1500) | S3 (1500) | S4 (9000) |
   132	|---|---|---|---|---|
   133	| `mw_tcp_mixed` (opposite direction) | 1.042 P | 0.979 P | 1.072 P | 1.021 P |
   134	| `wm_grpc_mixed` (opposite carrier) | 0.994 P | 1.022 P | 1.016 P | 1.020 P |
   135	| `wm_tcp_large` (opposite fixture) | 1.000 P | 1.015 P | 1.017 P | 1.017 P |
   136	
   137	P1's signature is unchanged by MTU: **TCP only, `mixed` only,
   138	destination-initiator only.**
   139	
   140	## What this does NOT establish (carried from the pre-registration)
   141	
   142	- **Segment fill is unmeasured.** 8948 is the MSS *ceiling*, not the *fill*.
   143	  The only conclusion supported is: *"raising the MTU did not improve these
   144	  cells under the observed packetization."* It does **not** prove per-packet
   145	  cost is irrelevant to blit in general. (The `wm_tcp_large` speedup shows
   146	  packetization matters *somewhere* — just not for Δ.)
   147	- **The MSS gate is start-and-end, not per-connection.** A mid-session change
   148	  that reverted before the end would go undetected.
   149	- **Verdict rows VOID at jumbo**: every `converge … old_committed`,
   150	  `cross … min_old_committed`, and block-1 `combined` row is graded against the
   151	  MTU-1500 `otp2w-baseline-2026-07-10` reference and is **VOID in the 9000
   152	  sessions**. None of the conclusions above use them. The **invariance** rows —
   153	  the measurand — are new-vs-new within one session and are MTU-matched by
   154	  construction.
   155	- The `NO-SAME-SESSION-REF` / absent discriminator-gap rows are the **declared
   156	  omission** (rev-4 F8), expected because these four cells have no block-1
   157	  counterparts in `CELLS`.
   158	
   159	## Rig log (recorded so it is not rediscovered)
   160	
   161	- **Time Machine was disabled on `q` for the window** (owner-executed; autobackup
   162	  had fired at 23:54 and macOS repeats hourly, which would have landed inside
   163	  the ~70-minute run, and one of its destinations is a network share on
   164	  `skippy` — i.e. the same 10 GbE fabric). **The harness's quiet-gate does not
   165	  catch this**: it refuses to start on `codex`/`cargo`/`rustc` only.
   166	- **`en8` was physically flapping before the run** and the owner reseated the
   167	  connection. Three harness starts died at the old-pair smoke with a gRPC
   168	  `transport error` while it was unstable; the daemon, the binaries, the
   169	  firewall, the MTU-set and the daemon-start timing were each individually
   170	  cleared (the daemon binds in 169–665 ms; a hand-run smoke succeeded
   171	  repeatedly). After the reseat, a 5 × 1 GiB link test ran at **891–897 ms**
   172	  (≈1.2 GB/s, 0 errors) and all four sessions then completed with 0 voided runs.
   173	- A `bash -x` diagnostic run at MTU 9000 was **discarded, not banked**: it
   174	  differed from its own replicate (tracing to disk on the bench Mac), and the
   175	  design requires the four sessions be identical.
   176	- `load1` on `q` sat at 1.5–2.3 through the sessions (macOS idle baseline on this
   177	  box; instantaneous CPU was <3%).
   178	
   179	## Files
   180	
   181	`S1_9000/`, `S2_1500/`, `S3_1500/`, `S4_9000/` — each with `runs.csv` (64 timed
   182	runs), `summary.csv`, `verdicts.csv`, `meta.csv`, `bench.log`, `session.log`,
   183	`staging-manifest.txt`, and the session's `mss-start.txt` / `mss-end.txt` +
   184	`load-start.txt` / `load-end.txt`.
zsh:1: no matches found: docs/bench/otp12-jumbo-win-2026-07-13/*.csv

exec
/bin/zsh -lc 'rg -n "MTU|jumbo|zoey|Zoey|otp2-baseline|otp2w-baseline|netwatch|rig [WZD]|2026-07-10|2026-07-13" .agents/machines.md docs/bench/otp12-jumbo-win-2026-07-13 docs/bench/otp2-baseline-2026-07-10 docs/bench/otp2w-baseline-2026-07-10 scripts/bench_otp12_win.sh scripts/bench_otp12_zoey.sh scripts/bench_otp12_delegated.sh' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/bench_otp12_delegated.sh:3:# bench_otp12_delegated.sh  —  otp-12c "rig D" delegated-vs-direct parity
scripts/bench_otp12_delegated.sh:8:# WHAT THIS MEASURES (plan D4, rig D — delegated-vs-direct parity)
scripts/bench_otp12_delegated.sh:30:# DIRECTIONS / CELLS (plan D5 label grammar, extended to rig D)
scripts/bench_otp12_delegated.sh:109:WIN_SSH="${WIN_SSH:-michael@netwatch-01}"
scripts/bench_otp12_delegated.sh:127:# drain gate (2s quiet windows, matching the zoey/win loops)
scripts/bench_otp12_delegated.sh:176:# provenance check on zoey). A clean build must embed "+<sha>" NOT followed by
scripts/bench_otp12_win.sh:12:#   same-session old arm AND docs/bench/otp2w-baseline-2026-07-10/
scripts/bench_otp12_win.sh:32:# refusal + PID-scoped teardown) and from bench_otp12_zoey.sh (ABBA
scripts/bench_otp12_win.sh:36:# destination sweep after the measured flush — the zoey I/O-storm
scripts/bench_otp12_win.sh:75:# Defaults match the box's 2026-07-12 reality: hostname netwatch-01,
scripts/bench_otp12_win.sh:78:WIN_SSH=${WIN_SSH:-michael@netwatch-01}
scripts/bench_otp12_win.sh:105:BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2w-baseline-2026-07-10/summary.csv"
scripts/bench_otp12_win.sh:568:            # F3; the zoey harness always had this).
scripts/bench_otp12_zoey.sh:2:# otp-12a: interleaved OLD-vs-NEW converge-up matrix on the Mac<->zoey rig
scripts/bench_otp12_zoey.sh:9:# rebuilt at that sha in a detached worktree, zoey daemon already staged
scripts/bench_otp12_zoey.sh:10:# in blit-temp since 2026-07-10), arm "new" = the run commit's pair.
scripts/bench_otp12_zoey.sh:13:# references — the same-session old arm AND the committed 2026-07-10
scripts/bench_otp12_zoey.sh:14:# baseline median (docs/bench/otp2-baseline-2026-07-10/summary.csv).
scripts/bench_otp12_zoey.sh:15:# Cross-direction and invariance claims live on rig W (otp-12b), never
scripts/bench_otp12_zoey.sh:41:#   export ZOEY_SSH=root@zoey
scripts/bench_otp12_zoey.sh:44:#   RUNS=4 ./scripts/bench_otp12_zoey.sh
scripts/bench_otp12_zoey.sh:45:#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh   # checks only
scripts/bench_otp12_zoey.sh:50:#     D-2026-07-05-2 handshake refuses the pair); zoey daemon zigbuilt
scripts/bench_otp12_zoey.sh:54:#     detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA; zoey daemon
scripts/bench_otp12_zoey.sh:56:#     unqualified 2026-07-10 staging at $ZOEY_TEMP/blit-daemon FAILED
scripts/bench_otp12_zoey.sh:65:#   * A RIG RUN needs the owner's fresh go for daemon runs on zoey
scripts/bench_otp12_zoey.sh:76:ZOEY_SSH=${ZOEY_SSH:-root@zoey}
scripts/bench_otp12_zoey.sh:93:# The 2026-07-10 staging at $ZOEY_TEMP/blit-daemon FAILED provenance
scripts/bench_otp12_zoey.sh:102:BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2-baseline-2026-07-10/summary.csv"
scripts/bench_otp12_zoey.sh:104:OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_zoey_$(date +%Y%m%dT%H%M%S)}
scripts/bench_otp12_zoey.sh:194:        die "a blit-daemon is already running on zoey — stop it first"
.agents/machines.md:3:## This Mac (owner's workstation) — recorded 2026-07-10, moved here 2026-07-11
.agents/machines.md:5:- Rig SSH keys installed: zoey (root), Windows box (`michael@10.1.10.173`),
.agents/machines.md:7:- NOPASSWD sudoers rule for the zoey pool-drain/purge helper.
.agents/machines.md:20:  never a bench end~~ **SUPERSEDED 2026-07-13: magneto IS a valid bench
.agents/machines.md:22:  saturate 10GbE where Zoey is definitely not"; services quiescable on
.agents/machines.md:26:  platform terms. NOPASSWD `drop_caches` grant added 2026-07-13.
.agents/machines.md:28:  enp2s0=10.1.10.59, both MTU 9000). **NOT usable as a bench end**: at
.agents/machines.md:31:  ratio toward 1.0 and MASKING the effect under test — zoey's failure
.agents/machines.md:34:## Network / MTU (rig-critical — read before touching MTU)
.agents/machines.md:36:- **THE macOS PING TRAP (cost ~1h on 2026-07-13; do not repeat).**
.agents/machines.md:39:  from a Mac **no matter what the real path MTU is**. This is a limit on
.agents/machines.md:41:  I misread it as "macOS cannot transmit jumbo frames", blamed the
.agents/machines.md:43:  hardware for nothing. **Verify jumbo with a real TCP transfer** (e.g.
.agents/machines.md:45:- **Jumbo works end-to-end at MTU 9000** (verified 2026-07-13 by real
.agents/machines.md:49:- **Windows (netwatch-01) ran at MTU 1500 for EVERY benchmark ever
.agents/machines.md:50:  recorded** (otp-2w, otp-12a/b/c). It was raised to 9000 on 2026-07-13.
.agents/machines.md:52:  **jumbo has never been exercised in a blit benchmark.** Those numbers
.agents/machines.md:53:  are valid — they are simply *1500-MTU* numbers — and rig W at jumbo is
.agents/machines.md:55:  `enp1s0f1` to 9000 to make the Linux rig jumbo too).
.agents/machines.md:56:- Mismatched MTUs on one L2 segment are fine: TCP MSS negotiation
.agents/machines.md:59:- **Fleet MTU as of 2026-07-13 — the whole 10 GbE fabric is now 9000:**
.agents/machines.md:61:  | host | iface | MTU | persistent? |
.agents/machines.md:64:  | netwatch-01 | Ethernet | 9000 | yes (raised 1500→9000 today) |
.agents/machines.md:66:  | **zoey** | `enp0s0` (RJ45, NFS data .206) | **9000** | yes — `[Link] MTUBytes=9000` in `/etc/systemd/network/enp0s0.network` |
.agents/machines.md:67:  | **zoey** | `enp0s1` (SFP, mgmt .210) | **9000** | yes — same, in `enp0s1.network` |
.agents/machines.md:69:  | magneto | `enp1s0f1` | 9000 | yes — NM profile `Wired connection 3` saved `mtu=9000` (2026-07-13) |
.agents/machines.md:71:  **Verified end-to-end 2026-07-13**: a jumbo DF ping from skippy reaches
.agents/machines.md:72:  magneto, zoey, altiera, netwatch-01 AND the Mac — all OK. Every 10 GbE
.agents/machines.md:76:- **zoey (UniFi UNAS Pro) jumbo — how it was done, and the trap.**
.agents/machines.md:78:  fine. Persistence = a `[Link]` / `MTUBytes=9000` stanza in each
.agents/machines.md:85:  `ssh root@zoey 'cat /sys/class/net/enp0s0/mtu'` → want 9000.
.agents/machines.md:90:- **Live NFS/TCP connections do NOT pick up a new MTU.** MSS is fixed at
.agents/machines.md:94:- Two-NICs-on-one-subnet (both `altiera` and `zoey`, and it is the
.agents/machines.md:100:## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)
.agents/machines.md:108:- **10GbE**: `en8` = **10.1.10.54**, MTU **9000**, media 10Gbase-T. This is the
.agents/machines.md:110:  *different* NIC at **10.1.10.92** (also MTU 9000). Any doc naming
.agents/machines.md:136:  `visudo -c` rejects any other mode). ssh key authorized on netwatch-01 in
.agents/machines.md:142:## THE MAC IS A BENCH END — keep it quiet (recorded 2026-07-13, learned the hard way)
.agents/machines.md:149:the very finding under test. (Same shape as the 2026-07-13 durability
.agents/machines.md:152:- **This actually happened** (2026-07-13, first A-B-B-A attempt): codex jobs ran
.agents/machines.md:153:  on the Mac for the whole 53-minute window. The same-MTU replicates caught it —
.agents/machines.md:154:  `wm_tcp_large` read 911 ms in S1 and 1847 ms in S4 **at the same MTU**, and the
.agents/machines.md:165:  codex sessions (2026-07-13). Ask the owner to clear the machine, or kill only
.agents/machines.md:171:## Rig residue (recorded 2026-07-10)
.agents/machines.md:173:- **The Mac's 10GbE IP and NIC CHANGED 2026-07-13** — this is a live
.agents/machines.md:176:    MTU 9000, 10Gbase-T. (SSH into the Mac = `michael@10.1.10.54`;
.agents/machines.md:177:    Remote Login is ON and netwatch-01's key is in the Mac's
.agents/machines.md:188:- Windows box = **`michael@netwatch-01`, IP 10.1.10.177 as of
.agents/machines.md:190:  by hostname; if the bare name stops resolving, `netwatch-01.local` or
.agents/machines.md:191:  the IP both work — the host key is filed under both). **MTU raised
.agents/machines.md:192:  1500 → 9000 on 2026-07-13** (see Network/MTU above). SMB File Sharing
.agents/machines.md:201:- **Rig pairing constraint (owner, 2026-07-13): zoey's CPU is too slow
.agents/machines.md:202:  to be a match for skippy** — a zoey↔skippy pair is NOT a valid
.agents/machines.md:203:  symmetric/performance-matched rig; a zoey endpoint becomes the
.agents/machines.md:206:  same-OS, real-network, performance-matched pair** (one Mac; zoey too
.agents/machines.md:211:- zoey: binaries staged 2026-07-10 in `blit-temp/` — **corrected
docs/bench/otp2w-baseline-2026-07-10/README.md:1:# otp-2w — OLD-path baseline on the owner-designated cross-direction rig (2026-07-10)
docs/bench/otp2w-baseline-2026-07-10/README.md:4:otp-12 acceptance bar's **cross-direction half** after the Mac↔zoey
docs/bench/otp2w-baseline-2026-07-10/README.md:10:The zoey dataset (`docs/bench/otp2-baseline-2026-07-10/`) remains the
docs/bench/otp2-baseline-2026-07-10/README.md:1:# otp-2 — OLD-path PER-DIRECTION disk-to-disk baseline (2026-07-10)
docs/bench/otp2-baseline-2026-07-10/README.md:10:(`docs/bench/otp2w-baseline-2026-07-10/`).
docs/bench/otp2-baseline-2026-07-10/README.md:18:> the daemon staged in zoey's `blit-temp/` — the binary this dataset's
docs/bench/otp2-baseline-2026-07-10/README.md:38:- **Daemon**: `zoey` (UNAS 8 Pro; Alpine-based aarch64, 4 slow cores,
docs/bench/otp2-baseline-2026-07-10/README.md:42:- **Link**: Thunderbolt 10GbE (Mac `en9`) ↔ zoey (10.1.10.206), same
docs/bench/otp2-baseline-2026-07-10/README.md:44:- Owner-stated and confirmed: zoey's CPU cannot saturate the link;
docs/bench/otp2-baseline-2026-07-10/README.md:63:µs/file, pull ≈ 278 µs/file on zoey's 4 slow cores — the July skippy
docs/bench/otp2-baseline-2026-07-10/README.md:79:alone. The old-path binaries stay staged in zoey's `blit-temp`.
docs/bench/otp2-baseline-2026-07-10/README.md:104:   quantified): `ssh zoey sync` inside the window costs ~1.2 s of
docs/bench/otp2-baseline-2026-07-10/README.md:126:export ZOEY_SSH=root@zoey
docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/staging-manifest.txt:8:-,reference,-,9ccb5c86d8654fc9de93dd25455f03bfb4dfaf856bab64f82fe371b245aa1337,/Users/michael/Dev/blit_v2_f35702a/docs/bench/otp2w-baseline-2026-07-10/summary.csv
docs/bench/otp12-jumbo-win-2026-07-13/README.md:1:# MTU IS NOT THE CAUSE OF P1 — A-B-B-A jumbo experiment, rig `q` ↔ netwatch-01 (2026-07-14)
docs/bench/otp12-jumbo-win-2026-07-13/README.md:12:evidence for a plan amendment only" — killing the MTU hypothesis in
docs/bench/otp12-jumbo-win-2026-07-13/README.md:22:| session | MTU | mac_init | win_init | **Δ** | ratio | invariance |
docs/bench/otp12-jumbo-win-2026-07-13/README.md:40:**KILLED as a material cause**. Raising the MTU did not recover *any* of the
docs/bench/otp12-jumbo-win-2026-07-13/README.md:42:jumbo), but **|Δ_9000 − Δ_1500| = 7 ms is far inside the measured noise floor
docs/bench/otp12-jumbo-win-2026-07-13/README.md:43:of 78 ms** — so the honest statement is not "jumbo made it worse" but **"the
docs/bench/otp12-jumbo-win-2026-07-13/README.md:44:two conditions are indistinguishable: MTU has no measurable effect on Δ."**
docs/bench/otp12-jumbo-win-2026-07-13/README.md:48:— P1 survives jumbo as a real, measurable asymmetry.
docs/bench/otp12-jumbo-win-2026-07-13/README.md:50:**P1 fails in all four sessions** (1.237–1.362) regardless of MTU, by the
docs/bench/otp12-jumbo-win-2026-07-13/README.md:63:So the honest scope of this null is: **jumbo is not a dominant cause of P1, and
docs/bench/otp12-jumbo-win-2026-07-13/README.md:65:contributing-size (~46 ms) MTU effect could be swamped by this rig's
docs/bench/otp12-jumbo-win-2026-07-13/README.md:75:one cluster near ~730 ms and one near ~840 ms — and the two same-MTU replicates
docs/bench/otp12-jumbo-win-2026-07-13/README.md:81:Same MTU, same build, same rig: the 72 ms gap between those medians is a
docs/bench/otp12-jumbo-win-2026-07-13/README.md:85:`docs/bench/win-local-ab-2026-07-13/`.
docs/bench/otp12-jumbo-win-2026-07-13/README.md:91:**The MTU verdict is robust to it.** Pooling all 16 runs per condition (instead
docs/bench/otp12-jumbo-win-2026-07-13/README.md:101:  **8948/8948** in both jumbo sessions, **1448/1448** in both 1500 sessions.
docs/bench/otp12-jumbo-win-2026-07-13/README.md:104:  jumbo on both arms** (mac_init 960→924 ms, win_init 945→916 ms). Jumbo does
docs/bench/otp12-jumbo-win-2026-07-13/README.md:106:- **Both arms of `wm_tcp_mixed` also sped up slightly at jumbo** (mac 1068→1032,
docs/bench/otp12-jumbo-win-2026-07-13/README.md:112:Rebuilt on the measured noise (`N_arm = 72 ms`, the largest same-MTU replicate
docs/bench/otp12-jumbo-win-2026-07-13/README.md:137:P1's signature is unchanged by MTU: **TCP only, `mixed` only,
docs/bench/otp12-jumbo-win-2026-07-13/README.md:143:  The only conclusion supported is: *"raising the MTU did not improve these
docs/bench/otp12-jumbo-win-2026-07-13/README.md:149:- **Verdict rows VOID at jumbo**: every `converge … old_committed`,
docs/bench/otp12-jumbo-win-2026-07-13/README.md:151:  MTU-1500 `otp2w-baseline-2026-07-10` reference and is **VOID in the 9000
docs/bench/otp12-jumbo-win-2026-07-13/README.md:153:  the measurand — are new-vs-new within one session and are MTU-matched by
docs/bench/otp12-jumbo-win-2026-07-13/README.md:169:  firewall, the MTU-set and the daemon-start timing were each individually
docs/bench/otp12-jumbo-win-2026-07-13/README.md:173:- A `bash -x` diagnostic run at MTU 9000 was **discarded, not banked**: it
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:1:# otp-12 rig-W MTU experiment — PRE-REGISTRATION (written before any timed run)
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:10:**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (**Active**, D-2026-07-13-1).
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:19:estimates variance **within** a session, while the entire MTU comparison is
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:20:**between** sessions — and MTU was perfectly aliased with session order.
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:34:## Design — counterbalanced, with same-MTU replicates (round-2 F1)
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:38:| session | MTU | role |
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:45:- **MTU is no longer aliased with order** (A first *and* last).
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:46:- **The same-MTU pairs are the noise model.** S1↔S4 (maximally separated in
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:49:  **with MTU held constant**. This is the "sham repeat" round 2 asked for.
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:51:**⚠ RIG CHANGED (revision 4, 2026-07-13) — the Mac end is now `q`.** Revisions
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:54:is the **M4 Mac mini `q`** (10.1.10.54, MTU 9000, MSS 8948) — quiet and
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:59:**The design is rig-independent** — A-B-B-A compares MTU conditions *within one
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:60:rig* and derives its noise floor from same-MTU replicates *on that rig* — so
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:65:(`docs/bench/otp12-q-baseline-2026-07-13/`): P1 **reproduces** there —
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:66:`wm_tcp_mixed` **1.385 FAIL** at MTU 9000 — while all three control cells PASS
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:68:~2–4%. That single-condition baseline is **not** this experiment (no same-MTU
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:96:| interface MTU, both ends | `ifconfig` / `Get-NetIPInterface` | 9000 / 9000 (NIC `Jumbo Packet = 9014`) |
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:98:| **negotiated MSS** | `getsockopt(TCP_MAXSEG)` + Linux `ss -ti` | **8948** each way (1448 at MTU 1500) |
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:114:- **A null result supports exactly one conclusion**: *"raising the MTU did not
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:135:two same-MTU replicate differences — the conservative choice:
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:139:    N_arm        = max over {win_init, mac_init} of the same-MTU |differences|
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:153:Otherwise the MTU recovery is
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:175:Reported **separately** (different questions): **does P1 pass at jumbo?**
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:186:  session noise** — which the same-MTU replicates decide, and which is exactly
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:199:movement in `mixed` is not an MTU effect." **That is unsound and is
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:201:`r = 78.4%` and invariance 1.065 — a real, plausible MTU effect — while
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:211:## Verdict rows VOID at jumbo (round-1 F7 — round 2 confirms the inventory is now complete)
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:213:The harness grades against `otp2w-baseline-2026-07-10/summary.csv`
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:214:(`bench_otp12_win.sh:105`), **recorded at MTU 1500**. In the 9000 condition:
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:220:  `converge … old_session` rows. Both are MTU-matched by construction.
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:227:stays at jumbo, **formal acceptance of P1 and the global rule — not merely
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:238:- `netwatch-01` intermittently fails to resolve (DHCP/mDNS); every session
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:245:in order **A(9000), B(1500), B(1500), A(9000)** — flipping the Windows MTU
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:249:# Windows MTU: Set-NetIPInterface -InterfaceAlias Ethernet -NlMtu <9000|1500>
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:254:# restore MTU 9000 afterwards
docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md:257:Preflight verified 2026-07-13: worktree clean at `f35702a`; binaries embed
docs/bench/otp12-jumbo-win-2026-07-13/S4_9000/staging-manifest.txt:8:-,reference,-,9ccb5c86d8654fc9de93dd25455f03bfb4dfaf856bab64f82fe371b245aa1337,/Users/michael/Dev/blit_v2_f35702a/docs/bench/otp2w-baseline-2026-07-10/summary.csv
docs/bench/otp12-jumbo-win-2026-07-13/S3_1500/staging-manifest.txt:8:-,reference,-,9ccb5c86d8654fc9de93dd25455f03bfb4dfaf856bab64f82fe371b245aa1337,/Users/michael/Dev/blit_v2_f35702a/docs/bench/otp2w-baseline-2026-07-10/summary.csv
docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/staging-manifest.txt:8:-,reference,-,9ccb5c86d8654fc9de93dd25455f03bfb4dfaf856bab64f82fe371b245aa1337,/Users/michael/Dev/blit_v2_f35702a/docs/bench/otp2w-baseline-2026-07-10/summary.csv

exec
/bin/zsh -lc "nl -ba .agents/machines.md | sed -n '1,320p'; nl -ba scripts/bench_otp12_win.sh | sed -n '1,180p'; nl -ba scripts/bench_otp12_zoey.sh | sed -n '1,180p'; nl -ba scripts/bench_otp12_delegated.sh | sed -n '1,190p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba docs/DECISIONS.md | sed -n '1,230p'; nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '120,360p'; nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '250,390p'; nl -ba docs/STATE.md | sed -n '1,45p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
   100	## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)
   101	
   102	`ssh michael@q` — Apple **M4 Mac mini**, 16 GB, macOS 26.5.2, arm64. It is now
   103	the rig-W Mac end: **quiet, dedicated, and faster than nagatha** (1 GiB in
   104	~908 ms ≈ 1.18 GB/s, vs nagatha's ~1.3–1.8 s). Using it **decouples the codex
   105	review loop from rig-W benchmarking** — the contention that destroyed a
   106	53-minute experiment (below).
   107	
   108	- **10GbE**: `en8` = **10.1.10.54**, MTU **9000**, media 10Gbase-T. This is the
   109	  **Aquantia adapter physically moved off nagatha**, so nagatha's 10GbE is now a
   110	  *different* NIC at **10.1.10.92** (also MTU 9000). Any doc naming
   111	  "Aquantia @ .54 on nagatha" is stale.
   112	- **⚠ THE MULTI-NIC ROUTING TRAP (cost ~1h).** `q` has THREE IPs on
   113	  10.1.10.0/24 — `en0` (1GbE, .221), `en1` (Wi-Fi, .108), `en8` (10GbE, .54) —
   114	  and macOS routes the subnet via the highest-ranked **network service**, not by
   115	  which IP "matches". `en0` outranked `en8`, so **every benchmark would have run
   116	  over gigabit**. Fixed by promoting the service that owns `en8` — confusingly
   117	  named **"Thunderbolt Ethernet Slot 3"** — to rank 1
   118	  (`sudo networksetup -ordernetworkservices …`). It has the same router
   119	  (10.1.10.1), so `q` keeps its internet.
   120	- **DO NOT "fix" this with a host route.**
   121	  `sudo route -n add -host 10.1.10.177 -interface en8` on a *directly-connected*
   122	  subnet installs a next hop of **the interface's own MAC** — a black hole. It
   123	  drops 100% of packets while `route -n get` still cheerfully reports
   124	  `interface: en8`. Verify with `arp -n <peer>`: the MAC must be the PEER's, not
   125	  `q`'s (`00:01:d2:19:04:a3`).
   126	- **An ssh transfer CANNOT verify this link.** ssh caps at ~79 MB/s on this path
   127	  (nagatha's known-good 10GbE scores the same 79), which is *below* the gigabit
   128	  ceiling — so a degraded link and a healthy one look identical through it. Use
   129	  `ifconfig en8 | grep media` (the PHY's negotiated rate) and blit's own
   130	  `wm_tcp_large` time (~908 ms for 1 GiB = 10GbE; ~10 s = 1GbE).
   131	- **Staged**: repo clone at `~/Dev/blit_v2_f35702a` (detached `f35702a`, cloned
   132	  from the LOCAL gitea — `q` *is* the gitea host); `target/release/{blit,blit-daemon}`
   133	  arm64 copied from nagatha (embed-verified `+f35702a`); old client at
   134	  `~/blit-bench-work/bins/blit-0f922de`; fixtures in `~/blit-bench-work`.
   135	  NOPASSWD `/usr/sbin/purge` granted (`/etc/sudoers.d/blit-bench`, mode 0440 —
   136	  `visudo -c` rejects any other mode). ssh key authorized on netwatch-01 in
   137	  **`C:\ProgramData\ssh\administrators_authorized_keys`** (michael is an admin
   138	  there, so the per-user file is ignored). macOS firewall is OFF on `q`.
   139	- **`q` RUNS GITEA** (it is `origin`, `http://q:3000`). It idles cheaply, but
   140	  **do not push to `origin` during a benchmark session**.
   141	
   142	## THE MAC IS A BENCH END — keep it quiet (recorded 2026-07-13, learned the hard way)
   143	
   144	**A rig-W (Mac↔Windows) benchmark requires a QUIET Mac.** The Mac is not a
   145	neutral driver: it runs the client in `mac_init` arms and serves the daemon in
   146	`win_init` arms. Any heavy Mac process contaminates the measurement — and
   147	**asymmetrically**, because `mac_init` runs the client locally while `win_init`
   148	runs it on Windows. CPU starvation therefore **inflates Δ and MANUFACTURES P1**,
   149	the very finding under test. (Same shape as the 2026-07-13 durability
   150	retraction: a cost billed to one arm and not the other.)
   151	
   152	- **This actually happened** (2026-07-13, first A-B-B-A attempt): codex jobs ran
   153	  on the Mac for the whole 53-minute window. The same-MTU replicates caught it —
   154	  `wm_tcp_large` read 911 ms in S1 and 1847 ms in S4 **at the same MTU**, and the
   155	  noise floor came out at 473 ms, larger than the 325 ms gap under test. The run
   156	  was discarded. Without the replicates it would have looked clean and been
   157	  reported.
   158	- **Offenders**: `codex` (the review loop!), `cargo`/`rustc`, Spotlight
   159	  reindexing, any build. **The review loop and a rig-W session cannot run at the
   160	  same time** — sequence them.
   161	- **The runner gates on this**: it refuses to start a session while
   162	  codex/cargo/rustc is running, and records `load1` per session so contamination
   163	  is visible in the evidence rather than hidden in the noise.
   164	- **NEVER blanket-kill to get quiet.** `pkill -f codex` killed the OWNER's own
   165	  codex sessions (2026-07-13). Ask the owner to clear the machine, or kill only
   166	  PIDs you can prove you launched.
   167	- The Linux rig (magneto↔skippy) does not involve the Mac and has no such
   168	  constraint — but P1's cell only exists on the Mac↔Windows pairing, so it
   169	  cannot substitute.
   170	
   171	## Rig residue (recorded 2026-07-10)
   172	
   173	- **The Mac's 10GbE IP and NIC CHANGED 2026-07-13** — this is a live
   174	  confound in the otp-12 numbers, not a bookkeeping detail:
   175	  * **now: `en9` = 10.1.10.54**, a Thunderbolt **Aquantia** adapter,
   176	    MTU 9000, 10Gbase-T. (SSH into the Mac = `michael@10.1.10.54`;
   177	    Remote Login is ON and netwatch-01's key is in the Mac's
   178	    `authorized_keys`, so Windows→Mac ssh/sftp works.)
   179	  * otp-12b (`wm_tcp_mixed` **1.237**) ran on the Aquantia at
   180	    **10.1.10.54**; otp-12c (**1.300**) ran on a Thunderbolt-5 dock's
   181	    built-in 10GbE at **10.1.10.91**. **Different NICs.** So the
   182	    "1.237 → 1.300, it got worse at the cutover sha" reading is
   183	    CONFOUNDED by a hardware change and must not be cited as evidence
   184	    of a code regression. Both runs still showed the same qualitative
   185	    asymmetry; only the delta is suspect.
   186	  * Harnesses take the Mac IP via `MAC_HOST=` — pass **10.1.10.54**
   187	    (older invocations in the DEVLOG say 10.1.10.91).
   188	- Windows box = **`michael@netwatch-01`, IP 10.1.10.177 as of
   189	  2026-07-12** (the earlier-recorded 10.1.10.173 is STALE — DHCP; ssh
   190	  by hostname; if the bare name stops resolving, `netwatch-01.local` or
   191	  the IP both work — the host key is filed under both). **MTU raised
   192	  1500 → 9000 on 2026-07-13** (see Network/MTU above). SMB File Sharing
   193	  is now ON on the Mac and Windows is authenticated to it
   194	  (`net use \\10.1.10.91\blit-bench-work`), so robocopy can reach it.
   195	  Rules: `blit-bench-daemon` (otp-2w, repo-path-scoped)
   196	  + `blit-otp12-daemon` (active-path-scoped) + staged
   197	  `purge-standby.ps1`; repo checkout DETACHED at `e21cf84` since the
   198	  otp-12b session (owner's `bench-cargo-lock` stash untouched); old
   199	  `0f922de` exes aside-copied at `D:\blit-test\bins\0f922de\`; run
   200	  bins under `D:\blit-test\bins\<sha>\`.
   201	- **Rig pairing constraint (owner, 2026-07-13): zoey's CPU is too slow
   202	  to be a match for skippy** — a zoey↔skippy pair is NOT a valid
   203	  symmetric/performance-matched rig; a zoey endpoint becomes the
   204	  bottleneck and MASKS data-plane effects rather than measuring them.
   205	  Consequence, recorded so it is not re-proposed: the fleet has **no
   206	  same-OS, real-network, performance-matched pair** (one Mac; zoey too
   207	  slow for skippy; magneto is a busy BitTorrent box — build-only, never
   208	  a bench end). Platform-vs-role confounds on a two-host rig therefore
   209	  cannot be broken by rig juggling and need a code-level counterfactual
   210	  (see `docs/plan/OTP12_PERF_FINDINGS.md`).
   211	- zoey: binaries staged 2026-07-10 in `blit-temp/` — **corrected
   212	  2026-07-12**: the staged daemon embeds `731023bfc8a1.dirty.…`, NOT
   213	  `e757dcc` as previously recorded (otp-2 README carries the full
   214	  correction note; daemon code is identical between the two commits).
   215	  Kept untouched as the otp-2 artifact; otp-12a stages clean sha-named
   216	  rebuilds beside it (`blit-daemon-e757dcc`, `blit-daemon-<run sha>`).
   217	  blit-temp path: `/volume/a595ddbf-d201-4e55-8183-ec78c8cd83e0/.srv/`
   218	  `.unifi-drive/michael/.data/blit-temp`.
     1	#!/usr/bin/env bash
     2	# otp-12b: the Mac<->Windows acceptance session (ONE_TRANSFER_PATH slice
     3	# otp-12, sub-slice 12b; design: docs/plan/OTP12_ACCEPTANCE_RUN.md
     4	# D1-D3/D5/D6). Two blocks on the owner-designated closest-spec pair:
     5	#
     6	#   BLOCK 1 — converge-up (Mac-initiated, matches the otp-2w recorded
     7	#   conditions): {large,small,mixed} x {push,pull} x {tcp,grpc} = 12
     8	#   comparisons, matched-pair interleaved A/B — arm "old" = the pinned
     9	#   pre-cutover pair (default 0f922de: Mac client rebuilt in a detached
    10	#   worktree; Windows daemon built natively at that commit), arm "new"
    11	#   = the run commit's pair. Verdicts against BOTH references (the
    12	#   same-session old arm AND docs/bench/otp2w-baseline-2026-07-10/
    13	#   summary.csv), per design D2 as amended.
    14	#
    15	#   BLOCK 2 — initiator/verb invariance (NEW pair only; the owner's
    16	#   sentence, measured): per data direction x fixture x carrier, arm
    17	#   "mac_init" vs arm "win_init" interleaved ABBA. Data Mac->Win (mw_*):
    18	#   Mac client pushes vs Windows client pulls the SAME physical source
    19	#   (the Mac module root IS $MAC_WORK — design F6). Data Win->Mac
    20	#   (wm_*): Mac client pulls vs Windows client pushes the same staged
    21	#   tree on D:. Cell grammar: <mw|wm>_<carrier>_<fixture>. Every arm
    22	#   also gets converge rows against its data direction's old references
    23	#   (design F3: no tolerance compounding), plus the F4 cross-direction
    24	#   rows and the D-2026-07-12-1 discriminator gap rows (recorded, never
    25	#   self-adjudicated).
    26	#
    27	# Methodology inherited verbatim from scripts/bench_otp2w_baseline.sh
    28	# (self-timed durability: Write-VolumeCache on Windows / per-file fsync
    29	# walk on macOS, keyed by DESTINATION OS never verb; Get-Counter drain;
    30	# standby-list purge + macOS purge; WMI daemon launch — Windows OpenSSH
    31	# kills session children; TOML literal-string module paths; stale-daemon
    32	# refusal + PID-scoped teardown) and from bench_otp12_zoey.sh (ABBA
    33	# counterbalance, pair-void valid-run rule with 2xRUNS cap + INCOMPLETE,
    34	# exit codes checked, +sha provenance, sha256 staging manifest,
    35	# PREFLIGHT_ONLY, CELLS allowlist for D2 escalations, per-run
    36	# destination sweep after the measured flush — the zoey I/O-storm
    37	# lesson, kept uniform here).
    38	#
    39	# Windows-side timed windows (win_init arms) are measured ON Windows —
    40	# a Stopwatch brackets the blit.exe invocation inside one ssh call and
    41	# prints "<ms>,<exit>"; the ssh round trip stays outside the window by
    42	# construction (the otp-2w F3 rule applied to a whole client run).
    43	#
    44	# Usage (from the client Mac):
    45	#   export WIN_SSH=michael@10.1.10.173
    46	#   export WIN_HOST=10.1.10.173
    47	#   export WIN_TEST='D:\blit-test'
    48	#   export MAC_HOST=<the Mac's 10GbE IP>      # required, no default
    49	#   RUNS=4 ./scripts/bench_otp12_win.sh
    50	#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
    51	#   CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_win.sh   # escalation
    52	#
    53	# Staging prerequisites (the rig session does these before preflight):
    54	#   * Mac: clean tree at the run commit; `cargo build --release` (client
    55	#     AND daemon — the Mac daemon serves block 2); old client rebuilt at
    56	#     $OLD_SHA in a detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA.
    57	#   * Windows: BEFORE moving the checkout, copy the detached-build exes
    58	#     aside to $WIN_TEST\bins\$OLD_SHA\; then fresh git bundle ->
    59	#     checkout the run commit -> native `cargo build --release` ->
    60	#     copy blit-daemon.exe AND blit.exe to $WIN_TEST\bins\<run sha>\.
    61	#     Daemons always LAUNCH from the fixed path
    62	#     $WIN_TEST\bins\active\blit-daemon.exe (arm swap = Copy-Item over
    63	#     it) so ONE program-scoped firewall rule covers both arms
    64	#     ("blit-otp12-daemon"; the otp-2w rule points at the repo path and
    65	#     is left alone).
    66	#   * Pre-cutover CLIENT binaries embed no build id (otp-12a-run F1):
    67	#     old-client provenance = the clean-worktree rebuild + the manifest,
    68	#     acknowledged via OLD_CLIENT_PROVENANCE_BY_BUILD=1.
    69	
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
     1	#!/usr/bin/env bash
     2	# otp-12a: interleaved OLD-vs-NEW converge-up matrix on the Mac<->zoey rig
     3	# (ONE_TRANSFER_PATH slice otp-12, sub-slice 12a; design:
     4	# docs/plan/OTP12_ACCEPTANCE_RUN.md D1/D2/D5/D6).
     5	#
     6	# What this measures: the otp-2 verdict matrix ({large,small,mixed} x
     7	# {push,pull} x {tcp,grpc} = 12 comparisons) rerun as matched-pair A/B —
     8	# arm "old" = the pinned pre-cutover pair (default e757dcc: Mac client
     9	# rebuilt at that sha in a detached worktree, zoey daemon already staged
    10	# in blit-temp since 2026-07-10), arm "new" = the run commit's pair.
    11	# This rig anchors PER-DIRECTION converge-up ONLY (hardware-asymmetric
    12	# endpoints, D-2026-07-05-1): a clean PASS needs new <= x1.10 of BOTH
    13	# references — the same-session old arm AND the committed 2026-07-10
    14	# baseline median (docs/bench/otp2-baseline-2026-07-10/summary.csv).
    15	# Cross-direction and invariance claims live on rig W (otp-12b), never
    16	# here.
    17	#
    18	# Methodology inherited verbatim from scripts/bench_otp2_baseline.sh
    19	# (cold caches both ends, drain-then-purge order, durable self-timed
    20	# destination flush, fresh never-seen destinations, wall-clock windows,
    21	# median = floor of the mean of the middle two). New in otp-12a:
    22	#   * ABBA counterbalanced interleave (codex design F5): pair slots run
    23	#     old,new / new,old / old,new / new,old — each arm leads half the
    24	#     pairs, so arm never confounds with within-pair order on the
    25	#     stateful pool.
    26	#   * Valid-run rule (codex design F7): a run with a nonzero blit exit
    27	#     OR an undrained pre-run window voids its whole PAIR; the pair is
    28	#     re-run at the same slot until RUNS valid pairs exist, capped at
    29	#     2*RUNS pair attempts per comparison; at the cap the comparison is
    30	#     recorded INCOMPLETE — never a silent pass, never a short median.
    31	#   * Exit codes checked (the old harness swallowed them inside the
    32	#     timed window); per-run blit output kept under $OUT_DIR/blit-logs/.
    33	#   * verdicts.csv computed at the end against both references
    34	#     (PASS / FAIL-SAME-SESSION / FAIL-REFERENCE-DRIFT / FAIL-BOTH /
    35	#     INCOMPLETE, per design D2).
    36	#   * Escalation (manual, design D2): a comparison that straddles its
    37	#     bar with either arm's spread > 25% is re-run in a fresh session
    38	#     at RUNS=8; both sessions get committed.
    39	#
    40	# Usage (from the client Mac):
    41	#   export ZOEY_SSH=root@zoey
    42	#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
    43	#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
    44	#   RUNS=4 ./scripts/bench_otp12_zoey.sh
    45	#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh   # checks only
    46	#
    47	# Prerequisites:
    48	#   * NEW pair: `cargo build --release` at the run commit with a CLEAN
    49	#     tree (a dirty build mints a distinct build id and the
    50	#     D-2026-07-05-2 handshake refuses the pair); zoey daemon zigbuilt
    51	#     (aarch64-musl, static) at the SAME commit and staged at
    52	#     $ZOEY_TEMP/blit-daemon-<sha>.
    53	#   * OLD pair: BOTH ends rebuilt clean at $OLD_SHA (Mac client in a
    54	#     detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA; zoey daemon
    55	#     zigbuilt and staged as $ZOEY_TEMP/blit-daemon-$OLD_SHA). The
    56	#     unqualified 2026-07-10 staging at $ZOEY_TEMP/blit-daemon FAILED
    57	#     provenance (dirty 731023b — otp-2 README correction) and is
    58	#     never used.
    59	#   * The OLD pair predates the handshake: its provenance is the
    60	#     staging record — this script records sha256 of every binary into
    61	#     staging-manifest.txt. The NEW pair's smoke transfer doubles as
    62	#     its identity check (a mismatched pair refuses with
    63	#     BUILD_MISMATCH at the first frame).
    64	#   * python3 + a NOPASSWD sudoers rule for /usr/sbin/purge on the Mac.
    65	#   * A RIG RUN needs the owner's fresh go for daemon runs on zoey
    66	#     (standing STATE rule). PREFLIGHT_ONLY=1 starts no daemon and
    67	#     times nothing (read-only ssh checks + local purge probe).
    68	#
    69	# Everything on the daemon host stays inside $ZOEY_TEMP (owner rule).
    70	
    71	set -euo pipefail
    72	
    73	SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    74	REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
    75	
    76	ZOEY_SSH=${ZOEY_SSH:-root@zoey}
    77	ZOEY_TEMP=${ZOEY_TEMP:?set ZOEY_TEMP to the blit-temp folder on the daemon host}
    78	ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
    79	PORT=${PORT:-9031}
    80	RUNS=${RUNS:-4}
    81	PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
    82	# Comma-separated comparison allowlist for the D2 escalation rule
    83	# (straddle + spread>25% -> fresh session at RUNS=8 for JUST those
    84	# comparisons; both sessions committed). Empty = the full matrix.
    85	CELLS=${CELLS:-}
    86	# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
    87	MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
    88	
    89	OLD_SHA=${OLD_SHA_ZOEY:-e757dcc}
    90	NEW_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
    91	NEW_BLIT=${NEW_BLIT:-$REPO_ROOT/target/release/blit}
    92	OLD_BLIT=${OLD_BLIT:-$MAC_WORK/bins/blit-$OLD_SHA}
    93	# The 2026-07-10 staging at $ZOEY_TEMP/blit-daemon FAILED provenance
    94	# (embeds 731023bfc8a1.dirty.…, not e757dcc — correction note in the
    95	# otp-2 README); both arms therefore run sha-named CLEAN rebuilds
    96	# staged beside it. The original is left untouched as the otp-2
    97	# artifact.
    98	OLD_DAEMON=${OLD_DAEMON:-$ZOEY_TEMP/blit-daemon-$OLD_SHA}
    99	NEW_DAEMON=${NEW_DAEMON:-$ZOEY_TEMP/blit-daemon-$NEW_SHA}
   100	# The committed reference is FIXED (pre-registered, design D2) — no env
   101	# override (codex otp-12a F5); its sha256 is recorded in the manifest.
   102	BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2-baseline-2026-07-10/summary.csv"
   103	
   104	OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_zoey_$(date +%Y%m%dT%H%M%S)}
   105	mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs" "$MAC_WORK"
   106	
   107	MODULE_ROOT="$ZOEY_TEMP/bench-module"
   108	REMOTE="$ZOEY_HOST:$PORT:/bench/"
   109	
   110	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
   111	die() { log "FATAL: $*"; exit 1; }
   112	# ControlMaster multiplexing: an ssh connection to this host costs
   113	# ~1.2s (slow-core key exchange) — reuse one connection.
   114	SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
   115	zssh() { ssh "${SSH_MUX[@]}" "$ZOEY_SSH" "$@"; }
   116	# Wall-clock ms across two separate python3 processes (deliberate; see
   117	# bench_otp2_baseline.sh for why monotonic is wrong here).
   118	now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
   119	# Self-timed durability steps (codex otp-2w F3): the timed window is
   120	# transfer + destination flush and NOTHING else; each flush times
   121	# ITSELF on the destination and reports only its own duration.
   122	sync_dest_ms() {   # Linux sync on the daemon host; prints its elapsed ms
   123	    zssh 'a=$(awk "{print int(\$1*1000)}" /proc/uptime); sync; b=$(awk "{print int(\$1*1000)}" /proc/uptime); echo $((b-a))'
   124	}
   125	# Durable pull window: macOS sync(2) only SCHEDULES writes; fsync every
   126	# landed file instead (media-level F_FULLFSYNC deliberately not used —
   127	# the Linux side does not pay media flush either).
   128	fsync_tree_ms() {
   129	    python3 - "$1" <<'PYEOF'
   130	import os, sys, time
   131	t = time.monotonic()
   132	for root, dirs, files in os.walk(sys.argv[1]):
   133	    for name in files:
   134	        fd = os.open(os.path.join(root, name), os.O_RDONLY)
   135	        os.fsync(fd)
   136	        os.close(fd)
   137	print(int((time.monotonic() - t) * 1000))
   138	PYEOF
   139	}
   140	
   141	want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }
   142	arm_blit()   { case "$1" in old) echo "$OLD_BLIT";;   new) echo "$NEW_BLIT";;   esac; }
   143	arm_daemon() { case "$1" in old) echo "$OLD_DAEMON";; new) echo "$NEW_DAEMON";; esac; }
   144	arm_sha()    { case "$1" in old) echo "$OLD_SHA";;    new) echo "$NEW_SHA";;    esac; }
   145	
   146	# --- Preflight ---------------------------------------------------------
   147	preflight() {
   148	    [[ "$RUNS" == 4 || "$RUNS" == 8 ]] \
   149	        || die "RUNS must be 4 (standard) or 8 (the D2 escalation) — got '$RUNS' (codex otp-12a F8: odd values break ABBA balance)"
   150	    [[ -x "$NEW_BLIT" ]] || die "missing $NEW_BLIT (cargo build --release first)"
   151	    [[ -x "$OLD_BLIT" ]] || die "old client not staged at $OLD_BLIT (rebuild at $OLD_SHA in a detached worktree: git worktree add --detach /tmp/blit-old $OLD_SHA && cargo build --release in it, then copy target/release/blit here)"
   152	    command -v python3 >/dev/null || die "python3 required (timing + fsync_tree + verdicts)"
   153	    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
   154	    sudo -n /usr/sbin/purge || die "cold-cache purge needs a NOPASSWD sudoers rule for /usr/sbin/purge"
   155	    zssh "test -x '$OLD_DAEMON'" || die "old daemon not staged at $OLD_DAEMON"
   156	    zssh "test -x '$NEW_DAEMON'" || die "new daemon not staged at $NEW_DAEMON (zigbuild aarch64-musl at $NEW_SHA, stage BESIDE the old one)"
   157	    # Provenance enforcement (codex otp-12a F3): a stale-but-matching
   158	    # pair passes the handshake yet is not the labeled build. Every
   159	    # binary must embed its arm's sha (session_build_id/BLIT_GIT_SHA is
   160	    # a compile-time literal in the binary; the old commits embed it
   161	    # too — they postdate otp-3).
   162	    # -a + LC_ALL=C are load-bearing: BSD grep on macOS silently
   163	    # misses matches inside binaries without them (UTF-8 line
   164	    # handling). The pattern is the BUILD-ID form "+<sha>" — a bare
   165	    # sha false-positives on build-directory paths cargo embeds
   166	    # (codex otp-12a-run F1: the e757dcc client's only bare match was
   167	    # the worktree path). This still cannot distinguish a clean id
   168	    # from "<sha>.dirty.…" — the clean-tree die above covers the new
   169	    # arm at run time; old-arm cleanliness rests on the build
   170	    # procedure (detached worktree = clean by construction) + the
   171	    # sha256 manifest.
   172	    LC_ALL=C grep -qa "+$NEW_SHA" "$NEW_BLIT" \
   173	        || die "$NEW_BLIT does not embed +$NEW_SHA — rebuild at the run commit (stale target/release?)"
   174	    zssh "grep -qa '+$NEW_SHA' '$NEW_DAEMON'" \
   175	        || die "$NEW_DAEMON does not embed +$NEW_SHA — restage the new daemon"
   176	    zssh "grep -qa '+$OLD_SHA' '$OLD_DAEMON'" \
   177	        || die "$OLD_DAEMON does not embed +$OLD_SHA — the staged old daemon is not the pinned pair"
   178	    # Pre-cutover CLIENT binaries embed NO greppable build id (codex
   179	    # otp-12a-run F1, verified against the e757dcc client). Where the
   180	    # id exists we require it; otherwise the operator must explicitly
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# bench_otp12_delegated.sh  —  otp-12c "rig D" delegated-vs-direct parity
     4	# ONE_TRANSFER_PATH slice otp-12, sub-slice 12c; design:
     5	# docs/plan/OTP12_ACCEPTANCE_RUN.md  D1 / D2 / D4 / D5 / D6 / D7.
     6	# =============================================================================
     7	#
     8	# WHAT THIS MEASURES (plan D4, rig D — delegated-vs-direct parity)
     9	# ----------------------------------------------------------------
    10	# For one logical remote<->remote transfer (skippy daemon <-> Windows daemon,
    11	# over 10 GbE) we compare two ways of moving the SAME bytes over the SAME data
    12	# plane to the SAME destination disk. The ONLY difference is who spawns the
    13	# initiator and the trigger/progress relay:
    14	#
    15	#   delegated : Mac runs `blit copy SRC_DAEMON DST_DAEMON --yes`. Remote<->remote
    16	#               is delegated-only (D-2026-07-11-1): this ALWAYS calls DelegatedPull
    17	#               on the DESTINATION daemon, which initiates the one session against
    18	#               the source daemon in the DESTINATION role. The Mac only relays
    19	#               control + progress (no payload through the Mac). Timed ON THE MAC
    20	#               around the CLI (it blocks until the relayed Summary), PLUS the
    21	#               destination's self-timed flush — deliberately INCLUDING the
    22	#               trigger RPC + relay overhead (the honest end-to-end delegation cost).
    23	#   direct    : the DESTINATION host runs the pull itself — `blit copy SRC_DAEMON
    24	#               LOCAL_DIR --yes` (a normal remote->local pull, NOT delegated). Timed
    25	#               on that host (self-timed), PLUS the same flush.
    26	#
    27	# Data plane, destination disk, and flush are identical across arms; only the
    28	# initiator (Mac-relayed daemon vs local CLI) differs. That is the parity axis.
    29	#
    30	# DIRECTIONS / CELLS (plan D5 label grammar, extended to rig D)
    31	#   sw_<carrier>_<fixture> : source = skippy, dest = Windows
    32	#   ws_<carrier>_<fixture> : source = Windows, dest = skippy
    33	# 6 TCP verdict cells (3 fixtures x 2 dirs) + 1 secondary gRPC smoke cell
    34	# (sw_grpc_large). 2 arms x RUNS(4) x (6+1) = 56 timed runs (plan D7).
    35	#
    36	# VERDICT (plan D2): per cell, delegated-parity bar = max(delegated,direct)/min
    37	# <= 1.10. TCP cells are the verdict rows; the grpc cell is computed identically
    38	# and labeled secondary (its cell name carries the carrier). The script COMPUTES
    39	# and WRITES the matrix; it never flips a plan checkbox (checkpoints are owner-only).
    40	#
    41	# ------------------------------------------------------------------------------
    42	# BUILD IDENTITY — READ BEFORE RUNNING (sharp edge; plan: same-build handshake)
    43	# ------------------------------------------------------------------------------
    44	# The verdict is meaningful only if every binary on all three hosts is the SAME
    45	# build. NEW_SHA is computed from `git rev-parse --short HEAD`; the harness refuses
    46	# to run unless `blit --version` on the Mac, skippy AND Windows all embed
    47	# EXPECT_SHA (default = NEW_SHA), and the staged Windows daemon == the launched
    48	# (active) daemon byte-for-byte.
    49	#
    50	#   * At authoring, HEAD = dcbd6ea ("governance refresh: toolkit ...") sits ONE
    51	#     docs/tooling-only commit above f35702a (the sha in the rig-W staging paths).
    52	#     dcbd6ea does NOT touch crates/, so a release build there SHOULD be identical
    53	#     to one at f35702a — but this harness does not assume it.
    54	#   * OPERATOR ACTION: rebuild release binaries at CURRENT HEAD on all three hosts
    55	#     and stage them under the $NEW_SHA-derived paths (…/bins/$NEW_SHA/), OR, if you
    56	#     have independently confirmed the f35702a binaries are byte-identical to HEAD,
    57	#     run with EXPECT_SHA=f35702a (and point SKIPPY_BLIT/…/WIN_BLIT at those paths).
    58	#     Do not silence this gate.
    59	#   * The clean-tree gate ignores docs/ churn but fails on any dirt under crates/
    60	#     or Cargo.{toml,lock} — those affect binary identity; docs do not.
    61	#
    62	# OTHER SHARP EDGES (each guarded below)
    63	#   * Daemon kills are PID-scoped + comm/name-verified — NEVER a blunt `pkill blit`.
    64	#   * Stale-listener refusal on $PORT on both daemon hosts before launch.
    65	#   * ABBA counterbalanced interleave (A,B,B,A,A,B,B,A; A=delegated, B=direct) with
    66	#     the D2 valid-run rule: a run with nonzero exit OR an undrained pre-run window
    67	#     VOIDS its whole pair; the pair reruns at the same slot until RUNS valid pairs
    68	#     exist, capped at 2*RUNS attempts; at the cap the cell is INCOMPLETE.
    69	#   * Cold caches on BOTH data-plane ends every run (skippy drop_caches via sudo -n;
    70	#     Windows standby purge) + drain-gate the destination disk (Windows Get-Counter
    71	#     loop; skippy /proc/diskstats quiet-window loop with a device-regex knob).
    72	#   * Delegation authorization is IP/CIDR, not hostname (production SSRF rule):
    73	#     MAC_HOST / SKIPPY_HOST / WIN_HOST MUST be numeric IPs.
    74	#
    75	# SCOPE: writes fixtures/config/logs locally + on the two rig hosts, drives the
    76	# matrix, emits CSVs + verdicts. Does not commit; does not touch git remotes.
    77	# PREFLIGHT_ONLY=1 runs every static gate and exits before fixtures/daemons.
    78	#
    79	# NOTE: this harness cannot be end-to-end tested from the authoring host (no rig
    80	# access). It follows the rig-W/rig-Z template shapes verbatim where possible;
    81	# treat the first live run as a shakeout and prefer PREFLIGHT_ONLY=1 first.
    82	# =============================================================================
    83	
    84	set -euo pipefail
    85	
    86	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    87	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    88	
    89	# ------------------------------------------------------------------ config ----
    90	NEW_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
    91	EXPECT_SHA="${EXPECT_SHA:-$NEW_SHA}"          # binary-embed gate; override only with proof
    92	
    93	# Mac — initiator of the delegated arm (NOT a data endpoint)
    94	MAC_HOST="${MAC_HOST:?set MAC_HOST to the Mac 10GbE IP, numeric, used in delegation allowlists}"
    95	MAC_BLIT="${MAC_BLIT:-$REPO_ROOT/target/release/blit}"
    96	MAC_WORK="${MAC_WORK:-$HOME/blit-bench-work}"
    97	
    98	# skippy — Linux daemon host (source for sw_*, dest for ws_*)
    99	SKIPPY_SSH="${SKIPPY_SSH:-admin@skippy}"
   100	SKIPPY_HOST="${SKIPPY_HOST:?set SKIPPY_HOST to the skippy 10GbE IP, numeric}"
   101	SKIPPY_BIN="${SKIPPY_BIN:-/mnt/generic-pool/video/blit-bin}"
   102	SKIPPY_BLIT="${SKIPPY_BLIT:-$SKIPPY_BIN/bins/$EXPECT_SHA/blit}"
   103	SKIPPY_DAEMON="${SKIPPY_DAEMON:-$SKIPPY_BIN/bins/$EXPECT_SHA/blit-daemon}"
   104	SKIPPY_MODULE="${SKIPPY_MODULE:-/mnt/generic-pool/video/bench-data}"   # module 'bench' data root
   105	SKIPPY_TEMP="${SKIPPY_TEMP:-/mnt/generic-pool/video/blit-bin}"         # config/log dir (exec-friendly pool)
   106	SKIPPY_DISK_REGEX="${SKIPPY_DISK_REGEX:-^sd[a-z]$|^nvme[0-9]+n1$|^dm-[0-9]+$}"  # /proc/diskstats field-3 match
   107	
   108	# Windows — daemon host (dest for sw_*, source for ws_*)
   109	WIN_SSH="${WIN_SSH:-michael@netwatch-01}"
   110	WIN_HOST="${WIN_HOST:-10.1.10.177}"
   111	WIN_DRIVE="${WIN_DRIVE:-D}"
   112	WIN_TEST="${WIN_TEST:-D:\\blit-test}"
   113	WIN_BINS="${WIN_BINS:-$WIN_TEST\\bins\\$EXPECT_SHA}"
   114	WIN_BLIT="${WIN_BLIT:-$WIN_BINS\\blit.exe}"
   115	NEW_WIN_DAEMON="${NEW_WIN_DAEMON:-$WIN_BINS\\blit-daemon.exe}"
   116	ACTIVE_WIN_DAEMON="${ACTIVE_WIN_DAEMON:-$WIN_TEST\\bins\\active\\blit-daemon.exe}"
   117	WIN_MODULE="${WIN_MODULE:-$WIN_TEST\\bench-module}"
   118	
   119	# common
   120	PORT="${PORT:-9031}"
   121	RUNS="${RUNS:-4}"
   122	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
   123	CELLS="${CELLS:-}"                            # empty = full matrix; else comma-list of cell names
   124	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   125	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12_delegated_$SESSION_TAG}"
   126	
   127	# drain gate (2s quiet windows, matching the zoey/win loops)
   128	DRAIN_ITERS="${DRAIN_ITERS:-60}"              # up to 60x2s = 120s
   129	DRAIN_QUIET="${DRAIN_QUIET:-3}"               # consecutive quiet windows
   130	WIN_DRAIN_THRESH="${WIN_DRAIN_THRESH:-1048576}"   # bytes/sec on D: considered idle
   131	SKIPPY_DRAIN_SECTORS="${SKIPPY_DRAIN_SECTORS:-4096}"  # sectors written / 2s considered idle
   132	
   133	# ssh multiplexing
   134	MUX_DIR="$(mktemp -d /tmp/blit-deleg-mux.XXXXXX)"   # /tmp, not $TMPDIR: macOS TMPDIR busts the 104-byte ControlPath socket limit
   135	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   136	         -o ControlMaster=auto -o "ControlPath=$MUX_DIR/%C" -o ControlPersist=180)
   137	
   138	mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs"
   139	
   140	# ------------------------------------------------------------------ helpers ---
   141	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
   142	die() { log "FATAL: $*"; exit 1; }
   143	now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
   144	sssh() { ssh "${SSH_MUX[@]}" "$SKIPPY_SSH" "$@"; }
   145	wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }             # remote default shell assumed PowerShell
   146	nocr() { tr -d '\r'; }
   147	
   148	want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }
   149	
   150	# ---- self-timed durability (destination-OS keyed, never verb keyed) ----------
   151	flush_win_ms() {   # Windows volume flush, self-timed; prints ms or NA
   152	  wssh "\$a=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); try{ Write-VolumeCache -DriveLetter '$WIN_DRIVE' -ErrorAction Stop }catch{ 'F:NA:F'; exit 0 }; \$b=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); \"F:\$(\$b-\$a):F\"" 2>/dev/null \
   153	    | nocr | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p;s/.*F:NA:F.*/NA/p' | head -1
   154	}
   155	sync_skippy_ms() {   # skippy sync bracketed by /proc/uptime in one shell
   156	  # codex otp-12c F5: a FAILED sync must not read as a plausible flush. The
   157	  # sentinel is emitted only on rc=0, so a broken sync yields NA -> the run
   158	  # voids (fail closed), instead of a numeric flush on unflushed bytes.
   159	  local out
   160	  out="$(sssh "a=\$(awk '{print int(\$1*1000)}' /proc/uptime); if sync; then b=\$(awk '{print int(\$1*1000)}' /proc/uptime); echo \"S:\$((b-a)):S\"; fi" 2>/dev/null \
   161	    | nocr | sed -n 's/.*S:\([0-9][0-9]*\):S.*/\1/p' | head -1)"
   162	  echo "${out:-NA}"
   163	}
   164	
   165	# ---- sha256 + version-embed provenance ---------------------------------------
   166	sha256_local() { local h; h=$(shasum -a 256 "$1" | cut -d' ' -f1) || die "sha256 failed for $1"; [[ ${#h} -eq 64 ]] || die "bad sha256 '$h' for $1"; echo "$h"; }
   167	sha256_skippy() { local h; h=$(sssh "sha256sum '$1' 2>/dev/null | cut -d' ' -f1" | nocr) || die "remote sha256 failed $1"; [[ ${#h} -eq 64 ]] || die "bad remote sha256 '$h' for $1"; echo "$h"; }
   168	sha256_win()   { local h; h=$(wssh "(Get-FileHash -Algorithm SHA256 '$1' -ErrorAction SilentlyContinue).Hash" | nocr | tr 'A-F' 'a-f' | tr -cd '0-9a-f'); [[ ${#h} -eq 64 ]] || die "bad win sha256 '$h' for $1"; echo "$h"; }
   169	# Build identity is checked by grepping the BUILD-ID form "+<sha>" out of the
   170	# binary itself (a compile-time literal; otp-12a-run F1). There is NO usable
   171	# `blit --version` — the CLI rejects the flag — so grep the exe, never run it.
   172	# LC_ALL=C + -a are load-bearing on BSD grep (macOS) for matches inside binaries.
   173	#
   174	# codex otp-12c F3: a bare substring test for "+<sha>" ALSO matches a dirty
   175	# build id ("+<sha>.dirty.<hash>" — the exact shape that fooled otp-12a's
   176	# provenance check on zoey). A clean build must embed "+<sha>" NOT followed by
   177	# ".dirty", so each gate below asserts the id is present AND that no dirty
   178	# variant of it is present.
   179	embeds_clean_local() {   # $1 = path
   180	  LC_ALL=C grep -qa -- "+$EXPECT_SHA" "$1" || return 1
   181	  LC_ALL=C grep -qa -- "+$EXPECT_SHA.dirty" "$1" && return 1
   182	  return 0
   183	}
   184	embeds_clean_skippy() {   # $1 = remote path
   185	  sssh "grep -qa -- '+$EXPECT_SHA' '$1' && ! grep -qa -- '+$EXPECT_SHA.dirty' '$1'"
   186	}
   187	embeds_clean_win() {   # $1 = remote path
   188	  wssh "if((Select-String -Path '$1' -SimpleMatch -Quiet -Pattern '+$EXPECT_SHA') -and -not (Select-String -Path '$1' -SimpleMatch -Quiet -Pattern '+$EXPECT_SHA.dirty')){exit 0}else{exit 1}"
   189	}
   190	

 succeeded in 0ms:
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
   178	
   179	## D-2026-07-13-2 — the local small-file finding queues BEHIND OTP12_PERF_FINDINGS
   180	- Decision: `docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft) is sequenced **behind** the ACTIVE `docs/plan/OTP12_PERF_FINDINGS.md` — the MTU experiment, then pf-1, then its fix slices. Owner, 2026-07-13, verbatim: **"well, odds that one affects the other? if this is contributory, would we know? probably irrelevant. behind."** No local-path code lands until otp-12's investigation has its attribution. The finding itself (blit vs robocopy, local `D: -> E:`, `docs/bench/win-local-ab-2026-07-13/`) is recorded now; only the *fix* waits.
   181	- Why: two reasons, one causal and one procedural. **Causal**: the local finding is very unlikely to explain either otp-12 finding. P1 is an *initiator-invariance* failure — both arms run identical code and differ only in who dials, so a worker-count or per-file cost cancels between them, and a local copy has no initiator axis at all. P2 is a *new-vs-old* regression, whereas the local cost is *old*: otp-11's own gate measured old-vs-new local `small` at 1684 -> 1750 ms (+3.9% PASS, `docs/bench/otp11-local-2026-07-11/`) and otp-11 D1 explicitly preserved the old pipeline's payload shapes (`PreparedPayload::File`/`TarShard` "exactly as the old local pipeline"). A long-standing cost cannot produce a new regression. **Procedural**: fixing local *first* would touch code shared with the wire sink, perturb P1/P2 mid-investigation, and void the pre-fix baselines pf-final depends on — destroying the attribution rather than adding to it. Sequencing behind keeps every counterfactual legible, and pf-final's full-matrix rerun would still surface any shared-code effect as a number.
   182	- Carried into pf-1 as a cheap check (the one way the two could touch): the local apply pipeline runs **one** worker by default (`transfer_session/local.rs:602`, `sink_workers` is 1 unless the hidden `--workers` flag sets `debug_mode`). If the unified session likewise changed the **remote receive** side's worker count versus old push, that WOULD be new, per-file, and a live P2 candidate. Establish it by reading the executed old path, not by assuming.
   183	- Supersedes: nothing. Adds `LOCAL_SMALL_FILE_PATH.md` to the `docs/STATE.md` queue behind item 1a.
   184	
   185	## D-2026-07-13-3 — Windows attribute/ADS loss is a real gap; fix it AFTER otp-12
   186	- Decision: `blit` silently drops Windows file attributes (ReadOnly/Hidden/System) and alternate data streams on the tar-shard path — **on both the local and the remote route**, exit code 0, no warning — and it will be **fixed after the current phase (otp-12) completes**, not now. Owner, 2026-07-13, verbatim: **"well that, while funny, makes sense. we started this as a linux alternative to robocopy, and full windows support was always a goal... but obviously not landed. so, good, let's address that. after this current phase is complete."** Finding, repro, and root cause: `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`.
   187	- Framing (owner's, and it is the correct one): this is **unlanded Windows support**, NOT a regression. blit began as a Linux alternative to robocopy; full Windows parity was always a goal and the metadata half never shipped. It predates the unified session and is not P1, P2, or otp-11 fallout.
   188	- What makes it more than a missing feature: the loss is **conditional on file count**, so it is silent and non-obvious. `transfer_plan.rs:103-109` sends a transfer down the tar path when there are ≥2 small files AND (≥32 of them OR average ≤128 KiB); otherwise files go through `CopyFileExW`, which carries attributes and ADS for free. So the SAME file keeps its metadata when copied alone and loses it when copied alongside 39 siblings. Proven with identical 200 KiB files where only the count varied (40 → LOST, 3 → PRESERVED), locally and over the wire.
   189	- **Fixing it is a WIRE CONTRACT change.** The tar shard is the wire payload format for small files, so carrying attributes/ADS means extending the shard header or the manifest — a frame change, which trips the stop-and-amend rule: `docs/TRANSFER_SESSION.md` is amended through the codex loop BEFORE any code. Same-build-both-ends (D-2026-07-05-2) means no compatibility surface is created, but the contract doc still governs. The header-vs-manifest choice is a design decision reserved for the owner.
   190	- Sequencing: behind otp-12, and **planned together with `LOCAL_SMALL_FILE_PATH.md`** (D-2026-07-13-2) — they touch the same tar path and pull in opposite directions (a fidelity fix ADDS per-file work to a path already losing 1.9× to robocopy at equal thread count). Planning them separately would optimise one against the other.
   191	- Not in scope / not a bug: **empty directories**. Their absence is a documented design position — `blit check`'s help (`crates/blit-cli/src/cli.rs:20-35`) states the equivalence model skips empty directories and points at `diff -r` for full tree equivalence. blit models files, not directories. (`test_push_empty_directory` only asserts the command succeeds; it never checks the directory arrived — a crash smoke test, not a fidelity test.) **ACLs** are likewise out: robocopy does not copy them either without `/COPY:S`.
   192	- Supersedes: nothing. Adds `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` to the `docs/STATE.md` queue behind otp-12, alongside D-2026-07-13-2.
   193	
   194	## D-2026-07-14-1 — the committed baselines are RE-RECORDED at MTU 9000 (amends OTP12_ACCEPTANCE_RUN D5's pin, not its freeze)
   195	- Decision: the frozen committed baselines that `pf-final` grades against are **re-recorded with their OLD builds at MTU 9000**, so acceptance compares old and new like-for-like on the fabric the fleet actually runs. Owner, 2026-07-14, choosing between three presented options, verbatim: **"Re-record the baseline at 9000"**. The 2026-07-10 baselines are **retained as historical MTU-1500 records** — superseded as the acceptance reference, never deleted or rewritten.
   196	- Why: pf-0 (`docs/bench/otp12-jumbo-win-2026-07-13/`) measured jumbo making **both arms 3–4% faster**. Grading a jumbo NEW arm against a **1500-recorded** ceiling is therefore **LENIENT, not conservative** — the MTU gain flatters the ratio, so a real regression up to roughly the size of that gain could pass unseen. P1 is the one finding between blit and shipping; a lenient ceiling is the wrong error to accept there.
   197	- Scope — **BOTH rigs, not just rig W.** Each harness hardcodes its own committed reference, and both predate the 2026-07-13 fabric-wide jumbo raise (`.agents/machines.md`): rig W `scripts/bench_otp12_win.sh:105` → `docs/bench/otp2w-baseline-2026-07-10/`; rig Z `scripts/bench_otp12_zoey.sh:102` → `docs/bench/otp2-baseline-2026-07-10/`. Rig D (delegated) has **no** old baseline and is unaffected.
   198	- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
   199	  * **Each rig's re-baseline MUST use the same OLD build as its original baseline**, with provenance manifest-verified — rig W `0f922de`; rig Z the build staged in `blit-temp` (which embeds `731023bfc8a1.dirty`, **not** `e757dcc` — see the otp-2 README correction). A re-baseline on a different old build would silently change the reference twice.
   200	  * `BASELINE_SUMMARY` is hardcoded **by design** (no override) so a run cannot quietly re-point its own ceiling. Re-pointing it is therefore a reviewed source edit, not an env var — and the new value must be a **committed** dated dir.
   201	  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
   202	- Supersedes: the *pin* in `OTP12_ACCEPTANCE_RUN.md` D5 ("the frozen baselines stay frozen") — the **freeze principle stands** (a baseline is immutable once recorded, and no run may re-point its own reference), but the acceptance reference is re-recorded once, at the fabric's MTU, and re-frozen. Closes the OPEN item raised in `OTP12_PERF_FINDINGS.md` §pf-0.
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
   221	  client pushes the same staged tree as a local path
   222	  (`blit.exe copy D:\blit-test\bench-module\pull_src_<w>\src_<w> $MAC_HOST:9031:/bench/<fresh>/ --yes`).
   223	
   224	New plumbing this requires, each keyed by ROLE not verb:
   225	
   226	1. **A daemon on the Mac** (new build only): config written like the rig
   227	   scripts do today (`[daemon] bind/port/no_mdns` + `[[module]] name =
   228	   "bench"` pointing at `$MAC_MODULE_ROOT`, **default `$MAC_WORK`
   229	   itself** — the module exports the exact fixture trees arm A pushes,
   230	   so both initiators read the same physical inodes; no fixture copy or
   231	   move on the Mac (codex design F6)), local launch, pid file,
   232	   stale-refusal, PID-scoped teardown. macOS application firewall must
   233	   admit `blit-daemon` — gated by a preflight smoke transfer from
   234	   Windows, not assumed.
   235	2. **A Windows client** (`blit.exe`, new build, built natively alongside
   236	   the daemon). Its timed window is measured ON Windows —
   237	   `[Diagnostics.Stopwatch]` bracketing the `blit.exe copy` inside one ssh
   238	   invocation, output CRLF-stripped (`tr -cd '0-9'`) — the otp-2w
   239	   self-timed pattern (README §Timing-overhead correction); the ssh
   240	   round-trip cost stays outside the window by construction.
   241	3. **Flush keyed by destination OS, never verb**: dest Windows ⇒ self-timed
   242	   `Write-VolumeCache D`; dest macOS ⇒ the local self-timed per-file fsync
   243	   walk. Cold caches both ends before every run (purge / standby-purge);
   244	   drain keyed by the destination disk (Windows `Get-Counter` loop when D:
   245	   receives; the Mac side has no drain equivalent — recorded decision: Mac
   246	   destination runs rely on `sync` + purge exactly as the recorded otp-2w
   247	   pull cells did).
   248	
   249	Arm A cells run fresh inside the invariance block (interleaved A,B,A,B…) —
   250	block-1 new-arm numbers are NOT reused, so rig-state drift between blocks
   251	cannot masquerade as an initiator effect.
   252	
   253	### D4 — delegated cells = delegated-vs-direct parity (rig D)
   254	
   255	Per data direction, the delegated arm and the direct arm drive the SAME
   256	session code with the same roles on the same endpoints; the only deltas are
   257	who spawns the initiator (daemon vs CLI) and the trigger/progress relay:
   258	
   259	- **skippy→Windows**: delegated = Mac runs
   260	  `blit copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ $WIN_HOST:9031:/bench/<fresh>/ --yes`
   261	  (Windows daemon initiates, DESTINATION role); direct = Windows client
   262	  pulls the same source to the same disk
   263	  (`blit.exe copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   264	- **Windows→skippy**: delegated = the mirror-image Mac command (skippy
   265	  daemon initiates); direct = skippy client pulls from the Windows daemon
   266	  (self-timed `/proc/uptime`-bracketed window over ssh, the zoey pattern).
   267	
   268	Timing: the delegated arm is timed on the Mac around the CLI invocation
   269	(the CLI blocks until the relayed Summary), plus the destination's
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
   292	> **AMENDED by D-2026-07-14-1 (2026-07-14) — the *pin* moves once; the *freeze*
   293	> stands.** The committed baselines this section pins were recorded at **MTU
   294	> 1500**, before the fabric-wide jumbo raise. pf-0 measured jumbo making both
   295	> arms 3–4% faster, so grading a jumbo build against a 1500 ceiling is **lenient,
   296	> not conservative**. Each rig's committed baseline is therefore **re-recorded
   297	> once with its ORIGINAL old build at MTU 9000** and re-frozen; the 2026-07-10
   298	> baselines are retained as historical MTU-1500 records. Immutability and the
   299	> no-override rule on `BASELINE_SUMMARY` are unchanged — see D-2026-07-14-1 and
   300	> `OTP12_PERF_FINDINGS.md` §pf-0.
   301	
   302	`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
   303	`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
   304	precedent: duplicate the shape, don't refactor recorded evidence;
   305	`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
   306	old scripts, both recorded sharp edges:
   307	
   308	- **Exit codes are checked**: the old harnesses swallow the blit exit code
   309	  inside the timed window; otp-12 records it per run (`exit` column) and a
   310	  nonzero exit voids the interleave pair per the D2 valid-run rule — a
   311	  failed transfer must never contribute a time.
   312	- **Multi-token flags ride an array**, not an unquoted scalar.
   313	
   314	CSV schema (all rigs):
   315	`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
   316	(`valid` = the PAIR's fate under the D2 valid-run rule — an
   317	individually-clean run whose partner voided reads `no`; amended at the
   318	12a harness slice)
   319	`summary.csv`:
   320	`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
   321	(medians over valid runs only — the D2 valid-run rule)
   322	`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
   323	where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
   324	otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
   325	reference CSVs; corrected at the 12a review, codex F9),
   326	`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
   327	direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
   328	discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
   329	self-adjudicated; added at the 12b harness slice), `arm` ∈
   330	`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
   331	`initiator` = host name, `kind` ∈
   332	`converge|invariance|delegated|cross|cross-gap`.
   333	Verdict outcome vocabulary (closed; 12b review, codex F12): per-reference
   334	rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
   335	carries the registered D2 set
   336	(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
   337	`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
   338	converge row whose same-session block-1 counterpart is absent or
   339	incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
   340	artifact — the committed-reference row still governs). Nothing else is
   341	legal, and a missing committed-reference row aborts the verdict pass
   342	(fail closed).
   343	
   344	Fixtures: identical shapes to otp-2 (1 GiB large / 10k×4 KiB small /
   345	512 MiB+5k×2 KiB mixed), generated with the existing recipes (BSD vs GNU
   346	`dd` block-size spelling handled per host), staged untimed; pull sources
   347	shared across arms (bytes are bytes — recorded explicitly); every timed
   348	destination is fresh and never-seen (`SESSION_TAG` + arm + run in the
   349	path).
   350	
   351	New env knobs: `MAC_HOST` (the Mac's 10 GbE IP — required, no default),
   352	`MAC_MODULE_ROOT` (default `$MAC_WORK` — see D3), `SKIPPY_SSH` (default
   353	`admin@skippy`), `SKIPPY_HOST`, `SKIPPY_BIN` (default
   354	`/mnt/generic-pool/video/blit-bin`), `SKIPPY_DISK_REGEX`,
   355	`OLD_SHA_ZOEY=e757dcc`, `OLD_SHA_WIN=0f922de`.
   356	
   357	Verification entry point for harness commits (no crates/proto touched; the
   358	cargo gates don't exercise bash): `bash -n` on each script + shellcheck
   359	where installed + `bash scripts/agent/check-docs.sh` + the codex review;
   360	the methodology itself is verified by the probe/recorded-run discipline
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
   281	## pf-0 — the environmental control (MTU): **KILLED as a material cause of P1** (recorded 2026-07-14)
   282	
   283	Executed as pre-registered
   284	(`docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md`); evidence + full
   285	adjudication in that directory's `README.md`. **The decision rule, thresholds
   286	and guards were registered in rev 3, before any of the S1–S4 data existed, and
   287	were unchanged by rev 4** (rev 4 re-described the *rig* after the `q` baseline —
   288	so "written before the data" is true of the rule, not of the whole document, and
   289	no threshold was authored around these numbers). Counterbalanced **A-B-B-A**
   290	(9000, 1500, 1500, 9000) on rig W with the `q` Mac end, `RUNS=8`, **256 timed
   291	runs, 0 voided**, MSS gate held at the start AND end of every session (8948
   292	jumbo / 1448 at 1500).
   293	
   294	    Δ_9000 = 236 ms    Δ_1500 = 229 ms    N_Δ (measured noise floor) = 78 ms
   295	    r = (Δ_1500 − Δ_9000) / Δ_1500 = −3.1%   →   KILLED (r < 20%, the scale below)
   296	
   297	**What this licenses — exactly the registered outcome, and no more.** Raising
   298	the MTU **did not improve these cells under the observed packetization**: the
   299	point estimate of the MTU contribution to P1 is ~0. The null is **not vacuous**
   300	— the manipulation demonstrably reached the wire (`wm_tcp_large` ran **3–4%
   301	faster at jumbo on both arms**, and both `wm_tcp_mixed` arms sped up slightly) —
   302	and the benefit is **symmetric**, which is why it cannot explain an
   303	**asymmetry**. P1 FAILED in all four sessions (1.237–1.362) regardless of MTU;
   304	all controls passed in all four.
   305	
   306	**What it does NOT license (do not restate this result as more than it is).**
   307	- **The wire is not exonerated, and "P1 is code-shaped" is NOT established
   308	  here.** MTU is *one* environmental variable. Segment **fill** is unmeasured
   309	  (8948 is the MSS *ceiling*), so underfilled segments, a bottleneck elsewhere,
   310	  or a smaller wire contribution are all still live. This result kills **MTU**,
   311	  not "the environment".
   312	- **It is not powered to exclude a CONTRIBUTING-size MTU effect.** The
   313	  CONFIRMED-CONTRIBUTING threshold is 20% of Δ_P1 ≈ **46 ms**, which is
   314	  **below the rig's measured between-session noise floor of 78 ms**. So the
   315	  experiment can exclude a **DOMINANT** effect (50% ≈ 114 ms, comfortably above
   316	  the floor) but **cannot exclude a contributing-size one** — a 46 ms effect
   317	  could be swamped. The registered rule returns KILLED on the point estimate,
   318	  and that grade stands as registered; the *resolution limit* is stated here so
   319	  the grade is never read as a stronger exclusion than the data supports.
   320	- It confirms no hypothesis. pf-1 still owns attribution.
   321	
   322	**`Δ_P1(rig W)` is re-estimated, and the noise floor constrains how pf-1 may
   323	grade.** The `282 ms` above is a **single nagatha session**; four sessions on
   324	the `q` pairing give **Δ_P1 ≈ 230 ms** (229 at 1500, 236 at 9000).
   325	
   326	- **Between-session grading of a counterfactual is now definitively ruled out**
   327	  on this rig: a 46 ms (20%) recovery is smaller than the 78 ms between-session
   328	  floor, so an unpaired before/after across sessions cannot separate
   329	  CONTRIBUTING from KILLED.
   330	- **This does NOT prove the interleaved design has enough resolution** — that is
   331	  a different (paired, within-session) variance, and pf-0 did not measure it.
   332	  **pf-1 must measure its own paired within-session noise floor on the
   333	  unmodified build and register a resolution check** (its smallest reportable
   334	  recovery must exceed that floor) *before* grading any hypothesis. A pf-1
   335	  recovery quoted without its paired floor is uninterpretable.
   336	- **The noise is not diffuse — it is a bistable fast arm.** The `win_init` runs
   337	  are **bimodal** (roughly ~730 ms and ~840 ms clusters); S1 drew 6 low/2 high
   338	  and S4 drew 2 low/6 high **at the same MTU**, and that mixture — not MTU — is
   339	  what produced the 72 ms `win_init` replicate spread and hence N_Δ. The
   340	  `mac_init` arm is by contrast stable to **5–6 ms**. **Trap for pf-1: a
   341	  counterfactual that merely shifts the mode mixture would masquerade as a
   342	  recovery.** Grade on the run distribution, not the median alone. (The MTU
   343	  verdict is robust to this: pooling all 16 runs per condition gives
   344	  Δ_9000 = 232, Δ_1500 = 221.5, r = −4.7% — same KILLED grade.)
   345	
   346	**RESOLVED — the committed baselines are RE-RECORDED at MTU 9000
   347	(D-2026-07-14-1, owner, 2026-07-14).** The exposure pf-0 surfaced: the fabric now
   348	runs MTU 9000 while the committed anti-drift ceilings were recorded at **MTU
   349	1500**, and pf-0 measured jumbo making **both arms 3–4% faster** — so grading a
   350	jumbo NEW arm against a 1500-recorded ceiling is **LENIENT, not conservative**:
   351	the MTU gain flatters the ratio and a real regression could pass unseen.
   352	
   353	The owner's resolution is to **re-record each rig's committed baseline with its
   354	ORIGINAL OLD build at MTU 9000**, then re-freeze it. The freeze principle is
   355	unchanged (a baseline is immutable once recorded; no run may re-point its own
   356	ceiling) — only the *pin* moves, once. The 2026-07-10 baselines are retained as
   357	historical MTU-1500 records.
   358	
   359	**This is a prerequisite slice for `pf-final`, and it affects BOTH rigs** (each
   360	harness hardcodes its own reference, and both predate the fabric-wide jumbo
   361	raise): rig W `bench_otp12_win.sh:105` → `otp2w-baseline-2026-07-10/`; rig Z
   362	`bench_otp12_zoey.sh:102` → `otp2-baseline-2026-07-10/`. Rig D has no old
   363	baseline and is unaffected. Constraints (same old build per rig,
   364	manifest-verified; `BASELINE_SUMMARY` stays override-free and is re-pointed by a
   365	reviewed source edit; the pf-0 start-AND-end MSS gate applies, since a baseline
   366	recorded at an unverified MTU is the very defect being fixed) are in
   367	D-2026-07-14-1 and are not restated here.
   368	
   369	Same-session references (`old_session`) are MTU-matched by construction and were
   370	never at risk.
   371	
   372	## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
   373	
   374	- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
   375	  connection-initiating end DIALS; byte direction is role-set
   376	  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
   377	  session the SOURCE is the responder: each sf-2 resize epoch is
   378	  ACCEPTED off the source's listener while the DESTINATION dials
   379	  (otp-5b-2: `SourceSockets` Dial/Accept branches;
   380	  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
   381	  accept/dial round-trips or serialization in the accept branch that the
   382	  dial branch does not pay.
   383	  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
   384	  was "mixed exercises resize hardest", but **all three fixtures target
   385	  eight streams before clamping** (`src/dial.rs:474`) — so resize
   386	  *count* cannot explain mixed-only behaviour, and H1 must name what
   387	  about mixed differs (shard-boundary timing? the tar-shard small half
   388	  interleaving with the big-file stream at the moment epochs fire?) or
   389	  be killed. **H1 also names the wrong half without proof**: it accuses
   390	  `Accept` while the destination's **synchronous dial-before-ACK** path
     1	# STATE — single entry point for "what is true right now"
     2	
     3	Last updated: 2026-07-14
     4	
     5	- **NEXT ACTION — `pf-1` (the HARD GATE): instrumentation + the interleaved counterfactuals.** Two pf-0 results now BIND it: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor), so pf-1 must **measure its own paired within-session noise floor on the unmodified build and register a resolution check** — smallest reportable recovery > that floor — *before* grading any hypothesis; (b) **the fast arm is BISTABLE**, so grade the run distribution, not the median. Design: `docs/plan/OTP12_PERF_FINDINGS.md` §Method + §pf-1 decision rule.
     6	- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
     7	- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
     8	- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
     9	- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
    10	- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
    11	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
    12	- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
    13	- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**
    14	
    15	Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
    16	≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
    17	procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
    18	
    19	## Now (active work)
    20	
    21	- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
    22	  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
    23	  invariant (plan doc, verbatim): ONE block of transfer code;
    24	  direction/initiator/verb can NEVER affect wall time by blit's doing
    25	  — impossible by construction because the per-direction drivers and
    26	  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
    27	  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
    28	  cells. **D-2026-07-05-2: same-build peers only, refusal at session
    29	  open.** Progress (each slice through the codex loop; per-slice
    30	  detail lives in DEVLOG + `.review/`, NOT here):
    31	  - **Closed `[x]`: otp-1 … otp-11** — the whole session machine, the
    32	    baselines (otp-2/2w), the **CUTOVER DELETION** (4 drivers +
    33	    `Push`/`PullSync` + 13 messages out of tree AND proto, −13.8k lines,
    34	    no bridge; relay removed D-2026-07-11-1), and **otp-11b's deletion of
    35	    the entire old orchestration** (−6.2k lines: orchestrator, engine,
    36	    local_worker, auto_tune, change_journal — the last an UNSOUND fast
    37	    path that silently lost data). The deletion-proof acceptance line
    38	    COMPLETES. Detail: DEVLOG 2026-07-10/11/12; evidence
    39	    `docs/bench/otp2{,w}-baseline-2026-07-10/`, `otp11-local-2026-07-11/`.
    40	- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
    41	  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
    42	  resume/re-derive on the unified baseline. Principle: ceiling-driven,
    43	  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
    44	- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
    45	  in Blocked); the codex loop governs all changes (D-2026-07-04-1).

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-jumbo-win-2026-07-13/README.md | sed -n '1,210p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba docs/bench/otp2-baseline-2026-07-10/README.md | sed -n '1,155p'; nl -ba docs/bench/otp2w-baseline-2026-07-10/README.md | sed -n '1,155p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba .agents/machines.md | sed -n '34,100p'; nl -ba .agents/machines.md | sed -n '171,235p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# MTU IS NOT THE CAUSE OF P1 — A-B-B-A jumbo experiment, rig `q` ↔ netwatch-01 (2026-07-14)
     2	
     3	**Status**: Evidence (recorded). This README applies the **pre-registered**
     4	decision rule in `PREREGISTRATION.md` to the data, and states nothing the rule
     5	does not license. **Provenance of the rule, stated precisely**: the decision
     6	rule, thresholds and guards were fixed in **rev 3**, before any of the S1–S4
     7	data existed, and rev 4 left them untouched (it re-described the *rig* after the
     8	`q` baseline). So no threshold was authored around these numbers — but "the
     9	document was written before any data existed" would be false, since a `q`
    10	baseline and a discarded A-B-B-A attempt preceded rev 4. It is
    11	**not** a plan amendment: per the pre-registration, a result here "licenses
    12	evidence for a plan amendment only" — killing the MTU hypothesis in
    13	`docs/plan/OTP12_PERF_FINDINGS.md` is a separate, reviewed change.
    14	
    15	**Design executed as registered**: four sessions **A-B-B-A** = 9000, 1500,
    16	1500, 9000, `RUNS=8`, `CELLS=wm_tcp_mixed,wm_tcp_large,mw_tcp_mixed,wm_grpc_mixed`,
    17	sha `f35702a` both ends, old arm `0f922de`, Mac end `q` (10.1.10.54, `en8`).
    18	**256 timed runs, 0 voided.**
    19	
    20	## Result — `r = −3.1%` → **KILLED as a material cause**
    21	
    22	| session | MTU | mac_init | win_init | **Δ** | ratio | invariance |
    23	|---|---:|---:|---:|---:|---:|---|
    24	| S1 | 9000 | 1035 | 760 | **275** | 1.362 | FAIL |
    25	| S2 | 1500 | 1071 | 830 | **241** | 1.290 | FAIL |
    26	| S3 | 1500 | 1066 | 849 | **217** | 1.256 | FAIL |
    27	| S4 | 9000 | 1029 | 832 | **197** | 1.237 | FAIL |
    28	
    29	    Δ_9000 = mean(275, 197) = 236 ms
    30	    Δ_1500 = mean(241, 217) = 229 ms
    31	    N_Δ    = max(|275−197|, |241−217|) = max(78, 24) = 78 ms   [measured noise floor]
    32	
    33	**Domain guard (evaluated first)**: `Δ_1500 (229) > N_Δ (78)` — the gap under
    34	study is present above this rig's own session-to-session noise, so the
    35	experiment is **in domain** and the recovery is computed.
    36	
    37	    r = (Δ_1500 − Δ_9000) / Δ_1500 = (229 − 236) / 229 = −3.1%
    38	
    39	On the parent plan's uniform pre-registered scale (`r < 20%`), that is
    40	**KILLED as a material cause**. Raising the MTU did not recover *any* of the
    41	gap; the point estimate is slightly negative (the gap was nominally *wider* at
    42	jumbo), but **|Δ_9000 − Δ_1500| = 7 ms is far inside the measured noise floor
    43	of 78 ms** — so the honest statement is not "jumbo made it worse" but **"the
    44	two conditions are indistinguishable: MTU has no measurable effect on Δ."**
    45	
    46	**Registered edge cases**: no INVERSION (`Δ_9000 = 236 > 0`); `r` not >100%;
    47	and `Δ_9000 (236) > N_Δ (78)`, so the residual gap is **not** inside the noise
    48	— P1 survives jumbo as a real, measurable asymmetry.
    49	
    50	**P1 fails in all four sessions** (1.237–1.362) regardless of MTU, by the
    51	harness's exact integer arithmetic (`10·hi ≤ 11·lo`), not the printed ratio.
    52	
    53	## ⚠ The resolution limit — this run cannot exclude a *contributing*-size effect
    54	
    55	The registered rule grades the **point estimate**, and the point estimate is ~0.
    56	But the experiment's own noise floor bounds what it could have seen:
    57	
    58	| effect size | in ms (of Δ_1500 = 229) | vs floor N_Δ = 78 ms | can this run exclude it? |
    59	|---|---:|---|---|
    60	| DOMINANT (`r ≥ 50%`) | ≥ 114 ms | comfortably above | **yes** |
    61	| CONTRIBUTING (`r ≥ 20%`) | ≥ 46 ms | **below the floor** | **NO** |
    62	
    63	So the honest scope of this null is: **jumbo is not a dominant cause of P1, and
    64	its measured contribution is indistinguishable from zero — but a
    65	contributing-size (~46 ms) MTU effect could be swamped by this rig's
    66	session-to-session noise and would not have been detected.** The KILLED grade
    67	stands as the pre-registered rule returns it; it must not be re-read as a
    68	stronger exclusion than that. (Pre-registration §"the noise model" fixed the
    69	floor as *measured*, not assumed — this is the price of that honesty, and it is
    70	stated rather than hidden.)
    71	
    72	## Where the noise actually comes from: the fast arm is BISTABLE
    73	
    74	The 78 ms floor is not diffuse jitter. The `win_init` runs are **bimodal** —
    75	one cluster near ~730 ms and one near ~840 ms — and the two same-MTU replicates
    76	simply drew different **mixtures** of the two modes:
    77	
    78	    S1 (9000) win_init: 699 715 750 753 767 776 | 843 844      -> 6 low, 2 high, median 760
    79	    S4 (9000) win_init: 752 755 | 825 828 836 837 838 860      -> 2 low, 6 high, median 832
    80	
    81	Same MTU, same build, same rig: the 72 ms gap between those medians is a
    82	**mode-mixture artifact**, and it is what sets N_Δ. The `mac_init` arm shows
    83	nothing of the kind (replicate medians differ by **5 and 6 ms**). This matches
    84	the local-rig bi-stability already recorded in
    85	`docs/bench/win-local-ab-2026-07-13/`.
    86	
    87	**Consequence for pf-1 (a trap):** a counterfactual that merely shifts the mode
    88	mixture would look exactly like a partial recovery. Grade on the run
    89	distribution, not the median alone.
    90	
    91	**The MTU verdict is robust to it.** Pooling all 16 runs per condition (instead
    92	of averaging session medians) gives `Δ_9000 = 232`, `Δ_1500 = 221.5`,
    93	**`r = −4.7%`** — the same KILLED grade.
    94	
    95	## The manipulation demonstrably reached the wire (the null is not vacuous)
    96	
    97	The most important defense of a null result is proof that the treatment was
    98	actually applied. Three independent instruments say it was:
    99	
   100	- **MSS gate, start AND end of every session** (the rev-4 requirement):
   101	  **8948/8948** in both jumbo sessions, **1448/1448** in both 1500 sessions.
   102	  No session is VOID on this gate.
   103	- **`wm_tcp_large` (registered as CONTEXT, never a gate)** got **3–4% faster at
   104	  jumbo on both arms** (mac_init 960→924 ms, win_init 945→916 ms). Jumbo does
   105	  real work on this path — it just does not touch the asymmetry.
   106	- **Both arms of `wm_tcp_mixed` also sped up slightly at jumbo** (mac 1068→1032,
   107	  win 840→796) while Δ stayed put. The benefit is **symmetric**, which is
   108	  precisely why it cannot explain an **asymmetry**.
   109	
   110	## Masking guard — the ratio did not improve, and no artifact is hiding a fix
   111	
   112	Rebuilt on the measured noise (`N_arm = 72 ms`, the largest same-MTU replicate
   113	difference across both arms). **Disclosure**: the pre-registration did not say
   114	how the two replicate medians become one condition-level value per arm; this
   115	analysis uses their **mean**. Every plausible alternative (either replicate
   116	alone, or the pooled runs) gives the same guard outcome, but "exactly as
   117	pre-registered" would overstate the spec's precision, so the choice is named
   118	here rather than left implicit.
   119	
   120	- **Fast-arm guard**: `win_init` at 9000 did **not** regress (−43.5 ms, i.e.
   121	  faster). OK.
   122	- **Convergence target**: `mac_9000 (1032) ≤ win_1500 (839.5) + N_arm (72) = 911.5`
   123	  → **NOT MET**. The slow arm did not approach the fast arm.
   124	- **Both-arms-slower (bottleneck compression)**: **False**.
   125	
   126	So there is no shared-floor artifact and no compression — there is simply **no
   127	fix**.
   128	
   129	## Controls (all four sessions, both conditions)
   130	
   131	| cell | S1 (9000) | S2 (1500) | S3 (1500) | S4 (9000) |
   132	|---|---|---|---|---|
   133	| `mw_tcp_mixed` (opposite direction) | 1.042 P | 0.979 P | 1.072 P | 1.021 P |
   134	| `wm_grpc_mixed` (opposite carrier) | 0.994 P | 1.022 P | 1.016 P | 1.020 P |
   135	| `wm_tcp_large` (opposite fixture) | 1.000 P | 1.015 P | 1.017 P | 1.017 P |
   136	
   137	P1's signature is unchanged by MTU: **TCP only, `mixed` only,
   138	destination-initiator only.**
   139	
   140	## What this does NOT establish (carried from the pre-registration)
   141	
   142	- **Segment fill is unmeasured.** 8948 is the MSS *ceiling*, not the *fill*.
   143	  The only conclusion supported is: *"raising the MTU did not improve these
   144	  cells under the observed packetization."* It does **not** prove per-packet
   145	  cost is irrelevant to blit in general. (The `wm_tcp_large` speedup shows
   146	  packetization matters *somewhere* — just not for Δ.)
   147	- **The MSS gate is start-and-end, not per-connection.** A mid-session change
   148	  that reverted before the end would go undetected.
   149	- **Verdict rows VOID at jumbo**: every `converge … old_committed`,
   150	  `cross … min_old_committed`, and block-1 `combined` row is graded against the
   151	  MTU-1500 `otp2w-baseline-2026-07-10` reference and is **VOID in the 9000
   152	  sessions**. None of the conclusions above use them. The **invariance** rows —
   153	  the measurand — are new-vs-new within one session and are MTU-matched by
   154	  construction.
   155	- The `NO-SAME-SESSION-REF` / absent discriminator-gap rows are the **declared
   156	  omission** (rev-4 F8), expected because these four cells have no block-1
   157	  counterparts in `CELLS`.
   158	
   159	## Rig log (recorded so it is not rediscovered)
   160	
   161	- **Time Machine was disabled on `q` for the window** (owner-executed; autobackup
   162	  had fired at 23:54 and macOS repeats hourly, which would have landed inside
   163	  the ~70-minute run, and one of its destinations is a network share on
   164	  `skippy` — i.e. the same 10 GbE fabric). **The harness's quiet-gate does not
   165	  catch this**: it refuses to start on `codex`/`cargo`/`rustc` only.
   166	- **`en8` was physically flapping before the run** and the owner reseated the
   167	  connection. Three harness starts died at the old-pair smoke with a gRPC
   168	  `transport error` while it was unstable; the daemon, the binaries, the
   169	  firewall, the MTU-set and the daemon-start timing were each individually
   170	  cleared (the daemon binds in 169–665 ms; a hand-run smoke succeeded
   171	  repeatedly). After the reseat, a 5 × 1 GiB link test ran at **891–897 ms**
   172	  (≈1.2 GB/s, 0 errors) and all four sessions then completed with 0 voided runs.
   173	- A `bash -x` diagnostic run at MTU 9000 was **discarded, not banked**: it
   174	  differed from its own replicate (tracing to disk on the bench Mac), and the
   175	  design requires the four sessions be identical.
   176	- `load1` on `q` sat at 1.5–2.3 through the sessions (macOS idle baseline on this
   177	  box; instantaneous CPU was <3%).
   178	
   179	## Files
   180	
   181	`S1_9000/`, `S2_1500/`, `S3_1500/`, `S4_9000/` — each with `runs.csv` (64 timed
   182	runs), `summary.csv`, `verdicts.csv`, `meta.csv`, `bench.log`, `session.log`,
   183	`staging-manifest.txt`, and the session's `mss-start.txt` / `mss-end.txt` +
   184	`load-start.txt` / `load-end.txt`.

 succeeded in 0ms:
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
   100	## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)
   171	## Rig residue (recorded 2026-07-10)
   172	
   173	- **The Mac's 10GbE IP and NIC CHANGED 2026-07-13** — this is a live
   174	  confound in the otp-12 numbers, not a bookkeeping detail:
   175	  * **now: `en9` = 10.1.10.54**, a Thunderbolt **Aquantia** adapter,
   176	    MTU 9000, 10Gbase-T. (SSH into the Mac = `michael@10.1.10.54`;
   177	    Remote Login is ON and netwatch-01's key is in the Mac's
   178	    `authorized_keys`, so Windows→Mac ssh/sftp works.)
   179	  * otp-12b (`wm_tcp_mixed` **1.237**) ran on the Aquantia at
   180	    **10.1.10.54**; otp-12c (**1.300**) ran on a Thunderbolt-5 dock's
   181	    built-in 10GbE at **10.1.10.91**. **Different NICs.** So the
   182	    "1.237 → 1.300, it got worse at the cutover sha" reading is
   183	    CONFOUNDED by a hardware change and must not be cited as evidence
   184	    of a code regression. Both runs still showed the same qualitative
   185	    asymmetry; only the delta is suspect.
   186	  * Harnesses take the Mac IP via `MAC_HOST=` — pass **10.1.10.54**
   187	    (older invocations in the DEVLOG say 10.1.10.91).
   188	- Windows box = **`michael@netwatch-01`, IP 10.1.10.177 as of
   189	  2026-07-12** (the earlier-recorded 10.1.10.173 is STALE — DHCP; ssh
   190	  by hostname; if the bare name stops resolving, `netwatch-01.local` or
   191	  the IP both work — the host key is filed under both). **MTU raised
   192	  1500 → 9000 on 2026-07-13** (see Network/MTU above). SMB File Sharing
   193	  is now ON on the Mac and Windows is authenticated to it
   194	  (`net use \\10.1.10.91\blit-bench-work`), so robocopy can reach it.
   195	  Rules: `blit-bench-daemon` (otp-2w, repo-path-scoped)
   196	  + `blit-otp12-daemon` (active-path-scoped) + staged
   197	  `purge-standby.ps1`; repo checkout DETACHED at `e21cf84` since the
   198	  otp-12b session (owner's `bench-cargo-lock` stash untouched); old
   199	  `0f922de` exes aside-copied at `D:\blit-test\bins\0f922de\`; run
   200	  bins under `D:\blit-test\bins\<sha>\`.
   201	- **Rig pairing constraint (owner, 2026-07-13): zoey's CPU is too slow
   202	  to be a match for skippy** — a zoey↔skippy pair is NOT a valid
   203	  symmetric/performance-matched rig; a zoey endpoint becomes the
   204	  bottleneck and MASKS data-plane effects rather than measuring them.
   205	  Consequence, recorded so it is not re-proposed: the fleet has **no
   206	  same-OS, real-network, performance-matched pair** (one Mac; zoey too
   207	  slow for skippy; magneto is a busy BitTorrent box — build-only, never
   208	  a bench end). Platform-vs-role confounds on a two-host rig therefore
   209	  cannot be broken by rig juggling and need a code-level counterfactual
   210	  (see `docs/plan/OTP12_PERF_FINDINGS.md`).
   211	- zoey: binaries staged 2026-07-10 in `blit-temp/` — **corrected
   212	  2026-07-12**: the staged daemon embeds `731023bfc8a1.dirty.…`, NOT
   213	  `e757dcc` as previously recorded (otp-2 README carries the full
   214	  correction note; daemon code is identical between the two commits).
   215	  Kept untouched as the otp-2 artifact; otp-12a stages clean sha-named
   216	  rebuilds beside it (`blit-daemon-e757dcc`, `blit-daemon-<run sha>`).
   217	  blit-temp path: `/volume/a595ddbf-d201-4e55-8183-ec78c8cd83e0/.srv/`
   218	  `.unifi-drive/michael/.data/blit-temp`.

 succeeded in 0ms:
     1	# otp-2 — OLD-path PER-DIRECTION disk-to-disk baseline (2026-07-10)
     2	
     3	**Status**: Recorded. **Scope (load-bearing)**: this rig's endpoints
     4	are hardware-asymmetric (client SSD vs daemon pool), and
     5	D-2026-07-05-1 rules that cross-direction performance comparisons are
     6	valid **only on symmetric endpoints**. This dataset therefore anchors
     7	**per-direction converge-up** (new ≤ old, same cell) and cannot anchor
     8	the otp-12 acceptance bar's cross-direction half — the owner
     9	designated the Mac↔Windows pair for that
    10	(`docs/bench/otp2w-baseline-2026-07-10/`).
    11	
    12	**Build**: `e757dcc` binaries both ends (client macOS arm64 release;
    13	daemon static aarch64-musl via
    14	`cargo zigbuild --release --target aarch64-unknown-linux-musl`); the
    15	recorded run used the harness as of `ceea6ed`+review fixes.
    16	
    17	> **Correction (2026-07-12, found by the otp-12a provenance preflight)**:
    18	> the daemon staged in zoey's `blit-temp/` — the binary this dataset's
    19	> daemon end actually ran — embeds build id `0.1.0+731023bfc8a1.dirty.…`,
    20	> i.e. a DIRTY build of `731023b`, not `e757dcc` as claimed above.
    21	> `git diff 731023b e757dcc -- crates proto Cargo.toml Cargo.lock` is
    22	> empty, so the COMMITTED daemon code is identical between the two
    23	> commits; the dirt's content, however, is unknowable after the fact —
    24	> the in-progress otp-2 harness/docs are a plausible candidate, but
    25	> that cannot be established, so treat these medians as carrying that
    26	> residual uncertainty. otp-12a therefore runs
    27	> its old arm on a CLEAN `e757dcc` rebuild staged separately
    28	> (`blit-daemon-e757dcc`), keeps this dataset as the committed reference
    29	> per OTP12 D2, and the pre-registered `FAIL-REFERENCE-DRIFT` outcome
    30	> covers any disagreement. The original staged pair is left untouched.
    31	**Harness**: `scripts/bench_otp2_baseline.sh` (methodology in its
    32	header; the probe CSVs here are the evidence that earned each rule).
    33	
    34	## Rig
    35	
    36	- **Client**: the owner's Mac (Apple Silicon), data on the internal
    37	  APFS SSD (`~/blit-bench-work`, never `/tmp`).
    38	- **Daemon**: `zoey` (UNAS 8 Pro; Alpine-based aarch64, 4 slow cores,
    39	  16 GiB RAM; 8-spindle pool ~102 TiB behind a mirrored-NVMe write
    40	  tier). All daemon-side state confined to the owner's `blit-temp`
    41	  folder (standing safety rule).
    42	- **Link**: Thunderbolt 10GbE (Mac `en9`) ↔ zoey (10.1.10.206), same
    43	  /24, ~0.4 ms RTT, endpoint pinned by IP.
    44	- Owner-stated and confirmed: zoey's CPU cannot saturate the link;
    45	  cells are CPU/storage-bound (the reference is per-cell on identical
    46	  hardware, not wire-speed).
    47	
    48	## Verdict-cell results (median of 4 cold, drained, durable runs; ms)
    49	
    50	| fixture | push tcp | push grpc | pull tcp | pull grpc |
    51	|---------|---------:|----------:|---------:|----------:|
    52	| large (1 GiB)            | 2702 | 4510 | 1744 | 2585 |
    53	| small (10k × 4 KiB)      | 4263 | 5217 | 2784 | 4188 |
    54	| mixed (512 MiB + 5k×2K)  | 2070 | 3889 | 1401 | 2222 |
    55	
    56	Per-run data: `runs.csv`; avg/best alongside medians: `summary.csv`;
    57	per-run drain outcomes: `drain-outcomes.txt` (zero anomalies).
    58	Rounding: integer ms; even-count median = floor of the mean of the
    59	middle two.
    60	
    61	Sanity: TCP < gRPC in all 12 cells. 1 GiB durable ≈ 3.2 Gbit/s push /
    62	4.9 Gbit/s pull. Small files are per-file-cost bound (push ≈ 426
    63	µs/file, pull ≈ 278 µs/file on zoey's 4 slow cores — the July skippy
    64	diagnosis's per-file-bound shape at a slower constant). Old-pull beats
    65	old-push in every cell, ×1.25–×1.75 — but on THESE endpoints that gap
    66	is confounded with destination hardware (pool vs SSD), which is
    67	exactly why D-2026-07-05-1 excludes cross-direction verdicts here.
    68	
    69	## Run-to-run stability (this dataset)
    70	
    71	Zero drain anomalies; per-cell (max−min)/min spreads: 5.6–26.5%
    72	typical, worst 48.6% (`push_tcp_small` — one fast outlier run, the
    73	others within 9% of each other). The pool's tiered write path never
    74	fully stops being stateful; the MEDIAN is the cell statistic
    75	precisely because of this, and every run is visible in `runs.csv`.
    76	**otp-12 prescription**: on this rig, verdicts (especially push
    77	cells) should be confirmed by interleaved same-session A/B
    78	(old-build vs new-build alternating), not by absolute comparison
    79	alone. The old-path binaries stay staged in zoey's `blit-temp`.
    80	
    81	## Methodology findings (why the harness looks the way it does)
    82	
    83	1. **Naive transfer-return timing is a write-cache lottery**
    84	   (`probe1-no-sync-runs.csv`): per-cell spread up to 8.0× (mixed
    85	   push 1446/6119/11577 ms) purely from how much of the payload the
    86	   write tier absorbed before writeback throttled. Fix:
    87	   durable-at-destination windows.
    88	2. **Durability must be equivalent on both ends**: Linux `sync`
    89	   waits for writeback (push windows); macOS `sync(2)` only
    90	   SCHEDULES writes, so pull windows fsync every landed file instead
    91	   (media-level F_FULLFSYNC deliberately not used — the Linux side
    92	   does not pay media flush either).
    93	3. **The daemon host's write path is stateful**
    94	   (`probe2-no-drain-runs.csv`): durable-timed pushes ascend
    95	   2.7 s → 13.4 s within a session as the NVMe tier fills and
    96	   destages. Fix: sync-then-drain before every run (three
    97	   consecutive quiet 2 s windows; timeouts recorded per run label,
    98	   never silent). `probe3-drained-pushes.csv` is the manual
    99	   confirmation probe.
   100	4. **Wall clock, not monotonic**: start/end stamps are separate
   101	   processes; cross-process `time.monotonic()` has undefined
   102	   reference points and produced 0/negative windows (aborted run).
   103	5. **The durability step must time ITSELF** (codex otp-2w F3,
   104	   quantified): `ssh zoey sync` inside the window costs ~1.2 s of
   105	   connection setup (slow-core key exchange, measured) that lands
   106	   only on push cells. `probe5-sshoverhead-{runs,summary}.csv` is
   107	   the affected session — its push medians run ~0.3–0.6 s high. The
   108	   recorded dataset uses self-timed destination flushes (the remote
   109	   `sync` measures its own duration via `/proc/uptime`; the local
   110	   fsync walk reports its own elapsed), so connection/shell overhead
   111	   is excluded from every window on both rigs.
   112	
   113	## Wire-reference data (explicitly NOT verdict cells)
   114	
   115	The July 2026-07-05 measurements (`docs/bench/10gbe-2026-07-05/`)
   116	used tmpfs local ends, ARC-warm re-reads, and no sync — deliberate
   117	engine-vs-wire isolation on a different rig. Per this plan slice they
   118	are re-labeled **wire-reference only**: never compare directions or
   119	absolute times from that data against these verdict cells
   120	(D-2026-07-05-1). `probe4-prereview-session-runs.csv` is an earlier
   121	session of THIS matrix kept for cross-session corroboration.
   122	
   123	## Reproduction
   124	
   125	```
   126	export ZOEY_SSH=root@zoey
   127	export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
   128	export ZOEY_HOST=10.1.10.206
   129	RUNS=4 ./scripts/bench_otp2_baseline.sh
   130	```
   131	
   132	Requires: the staged same-commit daemon in `$ZOEY_TEMP`, a NOPASSWD
   133	sudoers rule for `/usr/sbin/purge` on the client, python3 on the
   134	client, and SSH key auth to the daemon host.
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
    81	push/pull ratios accordingly (codex otp-2w F3, upheld and
    82	quantified). The recorded dataset uses SELF-TIMED durability steps —
    83	the flush measures its own duration on the destination and only that
    84	is added to the transfer segment — on both rigs.
    85	
    86	## Reproduction
    87	
    88	```
    89	export WIN_SSH=michael@10.1.10.173
    90	export WIN_HOST=10.1.10.173
    91	export WIN_REPO='F:\dev\blit_v2'
    92	export WIN_TEST='D:\blit-test'
    93	RUNS=4 ./scripts/bench_otp2w_baseline.sh
    94	```
    95	
    96	Requires: daemon built on the host (`cargo build --release` in
    97	`$WIN_REPO`), OpenSSH key auth with an admin token, python3 + the
    98	NOPASSWD purge rule on the client.

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '1,125p'; nl -ba scripts/bench_otp12_zoey.sh | sed -n '1,120p'; nl -ba scripts/bench_otp12_delegated.sh | sed -n '1,140p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env bash
     2	# otp-12b: the Mac<->Windows acceptance session (ONE_TRANSFER_PATH slice
     3	# otp-12, sub-slice 12b; design: docs/plan/OTP12_ACCEPTANCE_RUN.md
     4	# D1-D3/D5/D6). Two blocks on the owner-designated closest-spec pair:
     5	#
     6	#   BLOCK 1 — converge-up (Mac-initiated, matches the otp-2w recorded
     7	#   conditions): {large,small,mixed} x {push,pull} x {tcp,grpc} = 12
     8	#   comparisons, matched-pair interleaved A/B — arm "old" = the pinned
     9	#   pre-cutover pair (default 0f922de: Mac client rebuilt in a detached
    10	#   worktree; Windows daemon built natively at that commit), arm "new"
    11	#   = the run commit's pair. Verdicts against BOTH references (the
    12	#   same-session old arm AND docs/bench/otp2w-baseline-2026-07-10/
    13	#   summary.csv), per design D2 as amended.
    14	#
    15	#   BLOCK 2 — initiator/verb invariance (NEW pair only; the owner's
    16	#   sentence, measured): per data direction x fixture x carrier, arm
    17	#   "mac_init" vs arm "win_init" interleaved ABBA. Data Mac->Win (mw_*):
    18	#   Mac client pushes vs Windows client pulls the SAME physical source
    19	#   (the Mac module root IS $MAC_WORK — design F6). Data Win->Mac
    20	#   (wm_*): Mac client pulls vs Windows client pushes the same staged
    21	#   tree on D:. Cell grammar: <mw|wm>_<carrier>_<fixture>. Every arm
    22	#   also gets converge rows against its data direction's old references
    23	#   (design F3: no tolerance compounding), plus the F4 cross-direction
    24	#   rows and the D-2026-07-12-1 discriminator gap rows (recorded, never
    25	#   self-adjudicated).
    26	#
    27	# Methodology inherited verbatim from scripts/bench_otp2w_baseline.sh
    28	# (self-timed durability: Write-VolumeCache on Windows / per-file fsync
    29	# walk on macOS, keyed by DESTINATION OS never verb; Get-Counter drain;
    30	# standby-list purge + macOS purge; WMI daemon launch — Windows OpenSSH
    31	# kills session children; TOML literal-string module paths; stale-daemon
    32	# refusal + PID-scoped teardown) and from bench_otp12_zoey.sh (ABBA
    33	# counterbalance, pair-void valid-run rule with 2xRUNS cap + INCOMPLETE,
    34	# exit codes checked, +sha provenance, sha256 staging manifest,
    35	# PREFLIGHT_ONLY, CELLS allowlist for D2 escalations, per-run
    36	# destination sweep after the measured flush — the zoey I/O-storm
    37	# lesson, kept uniform here).
    38	#
    39	# Windows-side timed windows (win_init arms) are measured ON Windows —
    40	# a Stopwatch brackets the blit.exe invocation inside one ssh call and
    41	# prints "<ms>,<exit>"; the ssh round trip stays outside the window by
    42	# construction (the otp-2w F3 rule applied to a whole client run).
    43	#
    44	# Usage (from the client Mac):
    45	#   export WIN_SSH=michael@10.1.10.173
    46	#   export WIN_HOST=10.1.10.173
    47	#   export WIN_TEST='D:\blit-test'
    48	#   export MAC_HOST=<the Mac's 10GbE IP>      # required, no default
    49	#   RUNS=4 ./scripts/bench_otp12_win.sh
    50	#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
    51	#   CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_win.sh   # escalation
    52	#
    53	# Staging prerequisites (the rig session does these before preflight):
    54	#   * Mac: clean tree at the run commit; `cargo build --release` (client
    55	#     AND daemon — the Mac daemon serves block 2); old client rebuilt at
    56	#     $OLD_SHA in a detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA.
    57	#   * Windows: BEFORE moving the checkout, copy the detached-build exes
    58	#     aside to $WIN_TEST\bins\$OLD_SHA\; then fresh git bundle ->
    59	#     checkout the run commit -> native `cargo build --release` ->
    60	#     copy blit-daemon.exe AND blit.exe to $WIN_TEST\bins\<run sha>\.
    61	#     Daemons always LAUNCH from the fixed path
    62	#     $WIN_TEST\bins\active\blit-daemon.exe (arm swap = Copy-Item over
    63	#     it) so ONE program-scoped firewall rule covers both arms
    64	#     ("blit-otp12-daemon"; the otp-2w rule points at the repo path and
    65	#     is left alone).
    66	#   * Pre-cutover CLIENT binaries embed no build id (otp-12a-run F1):
    67	#     old-client provenance = the clean-worktree rebuild + the manifest,
    68	#     acknowledged via OLD_CLIENT_PROVENANCE_BY_BUILD=1.
    69	
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
     1	#!/usr/bin/env bash
     2	# otp-12a: interleaved OLD-vs-NEW converge-up matrix on the Mac<->zoey rig
     3	# (ONE_TRANSFER_PATH slice otp-12, sub-slice 12a; design:
     4	# docs/plan/OTP12_ACCEPTANCE_RUN.md D1/D2/D5/D6).
     5	#
     6	# What this measures: the otp-2 verdict matrix ({large,small,mixed} x
     7	# {push,pull} x {tcp,grpc} = 12 comparisons) rerun as matched-pair A/B —
     8	# arm "old" = the pinned pre-cutover pair (default e757dcc: Mac client
     9	# rebuilt at that sha in a detached worktree, zoey daemon already staged
    10	# in blit-temp since 2026-07-10), arm "new" = the run commit's pair.
    11	# This rig anchors PER-DIRECTION converge-up ONLY (hardware-asymmetric
    12	# endpoints, D-2026-07-05-1): a clean PASS needs new <= x1.10 of BOTH
    13	# references — the same-session old arm AND the committed 2026-07-10
    14	# baseline median (docs/bench/otp2-baseline-2026-07-10/summary.csv).
    15	# Cross-direction and invariance claims live on rig W (otp-12b), never
    16	# here.
    17	#
    18	# Methodology inherited verbatim from scripts/bench_otp2_baseline.sh
    19	# (cold caches both ends, drain-then-purge order, durable self-timed
    20	# destination flush, fresh never-seen destinations, wall-clock windows,
    21	# median = floor of the mean of the middle two). New in otp-12a:
    22	#   * ABBA counterbalanced interleave (codex design F5): pair slots run
    23	#     old,new / new,old / old,new / new,old — each arm leads half the
    24	#     pairs, so arm never confounds with within-pair order on the
    25	#     stateful pool.
    26	#   * Valid-run rule (codex design F7): a run with a nonzero blit exit
    27	#     OR an undrained pre-run window voids its whole PAIR; the pair is
    28	#     re-run at the same slot until RUNS valid pairs exist, capped at
    29	#     2*RUNS pair attempts per comparison; at the cap the comparison is
    30	#     recorded INCOMPLETE — never a silent pass, never a short median.
    31	#   * Exit codes checked (the old harness swallowed them inside the
    32	#     timed window); per-run blit output kept under $OUT_DIR/blit-logs/.
    33	#   * verdicts.csv computed at the end against both references
    34	#     (PASS / FAIL-SAME-SESSION / FAIL-REFERENCE-DRIFT / FAIL-BOTH /
    35	#     INCOMPLETE, per design D2).
    36	#   * Escalation (manual, design D2): a comparison that straddles its
    37	#     bar with either arm's spread > 25% is re-run in a fresh session
    38	#     at RUNS=8; both sessions get committed.
    39	#
    40	# Usage (from the client Mac):
    41	#   export ZOEY_SSH=root@zoey
    42	#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
    43	#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
    44	#   RUNS=4 ./scripts/bench_otp12_zoey.sh
    45	#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh   # checks only
    46	#
    47	# Prerequisites:
    48	#   * NEW pair: `cargo build --release` at the run commit with a CLEAN
    49	#     tree (a dirty build mints a distinct build id and the
    50	#     D-2026-07-05-2 handshake refuses the pair); zoey daemon zigbuilt
    51	#     (aarch64-musl, static) at the SAME commit and staged at
    52	#     $ZOEY_TEMP/blit-daemon-<sha>.
    53	#   * OLD pair: BOTH ends rebuilt clean at $OLD_SHA (Mac client in a
    54	#     detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA; zoey daemon
    55	#     zigbuilt and staged as $ZOEY_TEMP/blit-daemon-$OLD_SHA). The
    56	#     unqualified 2026-07-10 staging at $ZOEY_TEMP/blit-daemon FAILED
    57	#     provenance (dirty 731023b — otp-2 README correction) and is
    58	#     never used.
    59	#   * The OLD pair predates the handshake: its provenance is the
    60	#     staging record — this script records sha256 of every binary into
    61	#     staging-manifest.txt. The NEW pair's smoke transfer doubles as
    62	#     its identity check (a mismatched pair refuses with
    63	#     BUILD_MISMATCH at the first frame).
    64	#   * python3 + a NOPASSWD sudoers rule for /usr/sbin/purge on the Mac.
    65	#   * A RIG RUN needs the owner's fresh go for daemon runs on zoey
    66	#     (standing STATE rule). PREFLIGHT_ONLY=1 starts no daemon and
    67	#     times nothing (read-only ssh checks + local purge probe).
    68	#
    69	# Everything on the daemon host stays inside $ZOEY_TEMP (owner rule).
    70	
    71	set -euo pipefail
    72	
    73	SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    74	REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
    75	
    76	ZOEY_SSH=${ZOEY_SSH:-root@zoey}
    77	ZOEY_TEMP=${ZOEY_TEMP:?set ZOEY_TEMP to the blit-temp folder on the daemon host}
    78	ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
    79	PORT=${PORT:-9031}
    80	RUNS=${RUNS:-4}
    81	PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
    82	# Comma-separated comparison allowlist for the D2 escalation rule
    83	# (straddle + spread>25% -> fresh session at RUNS=8 for JUST those
    84	# comparisons; both sessions committed). Empty = the full matrix.
    85	CELLS=${CELLS:-}
    86	# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
    87	MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
    88	
    89	OLD_SHA=${OLD_SHA_ZOEY:-e757dcc}
    90	NEW_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
    91	NEW_BLIT=${NEW_BLIT:-$REPO_ROOT/target/release/blit}
    92	OLD_BLIT=${OLD_BLIT:-$MAC_WORK/bins/blit-$OLD_SHA}
    93	# The 2026-07-10 staging at $ZOEY_TEMP/blit-daemon FAILED provenance
    94	# (embeds 731023bfc8a1.dirty.…, not e757dcc — correction note in the
    95	# otp-2 README); both arms therefore run sha-named CLEAN rebuilds
    96	# staged beside it. The original is left untouched as the otp-2
    97	# artifact.
    98	OLD_DAEMON=${OLD_DAEMON:-$ZOEY_TEMP/blit-daemon-$OLD_SHA}
    99	NEW_DAEMON=${NEW_DAEMON:-$ZOEY_TEMP/blit-daemon-$NEW_SHA}
   100	# The committed reference is FIXED (pre-registered, design D2) — no env
   101	# override (codex otp-12a F5); its sha256 is recorded in the manifest.
   102	BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2-baseline-2026-07-10/summary.csv"
   103	
   104	OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_zoey_$(date +%Y%m%dT%H%M%S)}
   105	mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs" "$MAC_WORK"
   106	
   107	MODULE_ROOT="$ZOEY_TEMP/bench-module"
   108	REMOTE="$ZOEY_HOST:$PORT:/bench/"
   109	
   110	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
   111	die() { log "FATAL: $*"; exit 1; }
   112	# ControlMaster multiplexing: an ssh connection to this host costs
   113	# ~1.2s (slow-core key exchange) — reuse one connection.
   114	SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
   115	zssh() { ssh "${SSH_MUX[@]}" "$ZOEY_SSH" "$@"; }
   116	# Wall-clock ms across two separate python3 processes (deliberate; see
   117	# bench_otp2_baseline.sh for why monotonic is wrong here).
   118	now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
   119	# Self-timed durability steps (codex otp-2w F3): the timed window is
   120	# transfer + destination flush and NOTHING else; each flush times
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# bench_otp12_delegated.sh  —  otp-12c "rig D" delegated-vs-direct parity
     4	# ONE_TRANSFER_PATH slice otp-12, sub-slice 12c; design:
     5	# docs/plan/OTP12_ACCEPTANCE_RUN.md  D1 / D2 / D4 / D5 / D6 / D7.
     6	# =============================================================================
     7	#
     8	# WHAT THIS MEASURES (plan D4, rig D — delegated-vs-direct parity)
     9	# ----------------------------------------------------------------
    10	# For one logical remote<->remote transfer (skippy daemon <-> Windows daemon,
    11	# over 10 GbE) we compare two ways of moving the SAME bytes over the SAME data
    12	# plane to the SAME destination disk. The ONLY difference is who spawns the
    13	# initiator and the trigger/progress relay:
    14	#
    15	#   delegated : Mac runs `blit copy SRC_DAEMON DST_DAEMON --yes`. Remote<->remote
    16	#               is delegated-only (D-2026-07-11-1): this ALWAYS calls DelegatedPull
    17	#               on the DESTINATION daemon, which initiates the one session against
    18	#               the source daemon in the DESTINATION role. The Mac only relays
    19	#               control + progress (no payload through the Mac). Timed ON THE MAC
    20	#               around the CLI (it blocks until the relayed Summary), PLUS the
    21	#               destination's self-timed flush — deliberately INCLUDING the
    22	#               trigger RPC + relay overhead (the honest end-to-end delegation cost).
    23	#   direct    : the DESTINATION host runs the pull itself — `blit copy SRC_DAEMON
    24	#               LOCAL_DIR --yes` (a normal remote->local pull, NOT delegated). Timed
    25	#               on that host (self-timed), PLUS the same flush.
    26	#
    27	# Data plane, destination disk, and flush are identical across arms; only the
    28	# initiator (Mac-relayed daemon vs local CLI) differs. That is the parity axis.
    29	#
    30	# DIRECTIONS / CELLS (plan D5 label grammar, extended to rig D)
    31	#   sw_<carrier>_<fixture> : source = skippy, dest = Windows
    32	#   ws_<carrier>_<fixture> : source = Windows, dest = skippy
    33	# 6 TCP verdict cells (3 fixtures x 2 dirs) + 1 secondary gRPC smoke cell
    34	# (sw_grpc_large). 2 arms x RUNS(4) x (6+1) = 56 timed runs (plan D7).
    35	#
    36	# VERDICT (plan D2): per cell, delegated-parity bar = max(delegated,direct)/min
    37	# <= 1.10. TCP cells are the verdict rows; the grpc cell is computed identically
    38	# and labeled secondary (its cell name carries the carrier). The script COMPUTES
    39	# and WRITES the matrix; it never flips a plan checkbox (checkpoints are owner-only).
    40	#
    41	# ------------------------------------------------------------------------------
    42	# BUILD IDENTITY — READ BEFORE RUNNING (sharp edge; plan: same-build handshake)
    43	# ------------------------------------------------------------------------------
    44	# The verdict is meaningful only if every binary on all three hosts is the SAME
    45	# build. NEW_SHA is computed from `git rev-parse --short HEAD`; the harness refuses
    46	# to run unless `blit --version` on the Mac, skippy AND Windows all embed
    47	# EXPECT_SHA (default = NEW_SHA), and the staged Windows daemon == the launched
    48	# (active) daemon byte-for-byte.
    49	#
    50	#   * At authoring, HEAD = dcbd6ea ("governance refresh: toolkit ...") sits ONE
    51	#     docs/tooling-only commit above f35702a (the sha in the rig-W staging paths).
    52	#     dcbd6ea does NOT touch crates/, so a release build there SHOULD be identical
    53	#     to one at f35702a — but this harness does not assume it.
    54	#   * OPERATOR ACTION: rebuild release binaries at CURRENT HEAD on all three hosts
    55	#     and stage them under the $NEW_SHA-derived paths (…/bins/$NEW_SHA/), OR, if you
    56	#     have independently confirmed the f35702a binaries are byte-identical to HEAD,
    57	#     run with EXPECT_SHA=f35702a (and point SKIPPY_BLIT/…/WIN_BLIT at those paths).
    58	#     Do not silence this gate.
    59	#   * The clean-tree gate ignores docs/ churn but fails on any dirt under crates/
    60	#     or Cargo.{toml,lock} — those affect binary identity; docs do not.
    61	#
    62	# OTHER SHARP EDGES (each guarded below)
    63	#   * Daemon kills are PID-scoped + comm/name-verified — NEVER a blunt `pkill blit`.
    64	#   * Stale-listener refusal on $PORT on both daemon hosts before launch.
    65	#   * ABBA counterbalanced interleave (A,B,B,A,A,B,B,A; A=delegated, B=direct) with
    66	#     the D2 valid-run rule: a run with nonzero exit OR an undrained pre-run window
    67	#     VOIDS its whole pair; the pair reruns at the same slot until RUNS valid pairs
    68	#     exist, capped at 2*RUNS attempts; at the cap the cell is INCOMPLETE.
    69	#   * Cold caches on BOTH data-plane ends every run (skippy drop_caches via sudo -n;
    70	#     Windows standby purge) + drain-gate the destination disk (Windows Get-Counter
    71	#     loop; skippy /proc/diskstats quiet-window loop with a device-regex knob).
    72	#   * Delegation authorization is IP/CIDR, not hostname (production SSRF rule):
    73	#     MAC_HOST / SKIPPY_HOST / WIN_HOST MUST be numeric IPs.
    74	#
    75	# SCOPE: writes fixtures/config/logs locally + on the two rig hosts, drives the
    76	# matrix, emits CSVs + verdicts. Does not commit; does not touch git remotes.
    77	# PREFLIGHT_ONLY=1 runs every static gate and exits before fixtures/daemons.
    78	#
    79	# NOTE: this harness cannot be end-to-end tested from the authoring host (no rig
    80	# access). It follows the rig-W/rig-Z template shapes verbatim where possible;
    81	# treat the first live run as a shakeout and prefer PREFLIGHT_ONLY=1 first.
    82	# =============================================================================
    83	
    84	set -euo pipefail
    85	
    86	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    87	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    88	
    89	# ------------------------------------------------------------------ config ----
    90	NEW_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
    91	EXPECT_SHA="${EXPECT_SHA:-$NEW_SHA}"          # binary-embed gate; override only with proof
    92	
    93	# Mac — initiator of the delegated arm (NOT a data endpoint)
    94	MAC_HOST="${MAC_HOST:?set MAC_HOST to the Mac 10GbE IP, numeric, used in delegation allowlists}"
    95	MAC_BLIT="${MAC_BLIT:-$REPO_ROOT/target/release/blit}"
    96	MAC_WORK="${MAC_WORK:-$HOME/blit-bench-work}"
    97	
    98	# skippy — Linux daemon host (source for sw_*, dest for ws_*)
    99	SKIPPY_SSH="${SKIPPY_SSH:-admin@skippy}"
   100	SKIPPY_HOST="${SKIPPY_HOST:?set SKIPPY_HOST to the skippy 10GbE IP, numeric}"
   101	SKIPPY_BIN="${SKIPPY_BIN:-/mnt/generic-pool/video/blit-bin}"
   102	SKIPPY_BLIT="${SKIPPY_BLIT:-$SKIPPY_BIN/bins/$EXPECT_SHA/blit}"
   103	SKIPPY_DAEMON="${SKIPPY_DAEMON:-$SKIPPY_BIN/bins/$EXPECT_SHA/blit-daemon}"
   104	SKIPPY_MODULE="${SKIPPY_MODULE:-/mnt/generic-pool/video/bench-data}"   # module 'bench' data root
   105	SKIPPY_TEMP="${SKIPPY_TEMP:-/mnt/generic-pool/video/blit-bin}"         # config/log dir (exec-friendly pool)
   106	SKIPPY_DISK_REGEX="${SKIPPY_DISK_REGEX:-^sd[a-z]$|^nvme[0-9]+n1$|^dm-[0-9]+$}"  # /proc/diskstats field-3 match
   107	
   108	# Windows — daemon host (dest for sw_*, source for ws_*)
   109	WIN_SSH="${WIN_SSH:-michael@netwatch-01}"
   110	WIN_HOST="${WIN_HOST:-10.1.10.177}"
   111	WIN_DRIVE="${WIN_DRIVE:-D}"
   112	WIN_TEST="${WIN_TEST:-D:\\blit-test}"
   113	WIN_BINS="${WIN_BINS:-$WIN_TEST\\bins\\$EXPECT_SHA}"
   114	WIN_BLIT="${WIN_BLIT:-$WIN_BINS\\blit.exe}"
   115	NEW_WIN_DAEMON="${NEW_WIN_DAEMON:-$WIN_BINS\\blit-daemon.exe}"
   116	ACTIVE_WIN_DAEMON="${ACTIVE_WIN_DAEMON:-$WIN_TEST\\bins\\active\\blit-daemon.exe}"
   117	WIN_MODULE="${WIN_MODULE:-$WIN_TEST\\bench-module}"
   118	
   119	# common
   120	PORT="${PORT:-9031}"
   121	RUNS="${RUNS:-4}"
   122	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
   123	CELLS="${CELLS:-}"                            # empty = full matrix; else comma-list of cell names
   124	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   125	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12_delegated_$SESSION_TAG}"
   126	
   127	# drain gate (2s quiet windows, matching the zoey/win loops)
   128	DRAIN_ITERS="${DRAIN_ITERS:-60}"              # up to 60x2s = 120s
   129	DRAIN_QUIET="${DRAIN_QUIET:-3}"               # consecutive quiet windows
   130	WIN_DRAIN_THRESH="${WIN_DRAIN_THRESH:-1048576}"   # bytes/sec on D: considered idle
   131	SKIPPY_DRAIN_SECTORS="${SKIPPY_DRAIN_SECTORS:-4096}"  # sectors written / 2s considered idle
   132	
   133	# ssh multiplexing
   134	MUX_DIR="$(mktemp -d /tmp/blit-deleg-mux.XXXXXX)"   # /tmp, not $TMPDIR: macOS TMPDIR busts the 104-byte ControlPath socket limit
   135	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   136	         -o ControlMaster=auto -o "ControlPath=$MUX_DIR/%C" -o ControlPersist=180)
   137	
   138	mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs"
   139	
   140	# ------------------------------------------------------------------ helpers ---

exec
/bin/zsh -lc "nl -ba docs/bench/otp2-baseline-2026-07-10/README.md | sed -n '1,150p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp-2 — OLD-path PER-DIRECTION disk-to-disk baseline (2026-07-10)
     2	
     3	**Status**: Recorded. **Scope (load-bearing)**: this rig's endpoints
     4	are hardware-asymmetric (client SSD vs daemon pool), and
     5	D-2026-07-05-1 rules that cross-direction performance comparisons are
     6	valid **only on symmetric endpoints**. This dataset therefore anchors
     7	**per-direction converge-up** (new ≤ old, same cell) and cannot anchor
     8	the otp-12 acceptance bar's cross-direction half — the owner
     9	designated the Mac↔Windows pair for that
    10	(`docs/bench/otp2w-baseline-2026-07-10/`).
    11	
    12	**Build**: `e757dcc` binaries both ends (client macOS arm64 release;
    13	daemon static aarch64-musl via
    14	`cargo zigbuild --release --target aarch64-unknown-linux-musl`); the
    15	recorded run used the harness as of `ceea6ed`+review fixes.
    16	
    17	> **Correction (2026-07-12, found by the otp-12a provenance preflight)**:
    18	> the daemon staged in zoey's `blit-temp/` — the binary this dataset's
    19	> daemon end actually ran — embeds build id `0.1.0+731023bfc8a1.dirty.…`,
    20	> i.e. a DIRTY build of `731023b`, not `e757dcc` as claimed above.
    21	> `git diff 731023b e757dcc -- crates proto Cargo.toml Cargo.lock` is
    22	> empty, so the COMMITTED daemon code is identical between the two
    23	> commits; the dirt's content, however, is unknowable after the fact —
    24	> the in-progress otp-2 harness/docs are a plausible candidate, but
    25	> that cannot be established, so treat these medians as carrying that
    26	> residual uncertainty. otp-12a therefore runs
    27	> its old arm on a CLEAN `e757dcc` rebuild staged separately
    28	> (`blit-daemon-e757dcc`), keeps this dataset as the committed reference
    29	> per OTP12 D2, and the pre-registered `FAIL-REFERENCE-DRIFT` outcome
    30	> covers any disagreement. The original staged pair is left untouched.
    31	**Harness**: `scripts/bench_otp2_baseline.sh` (methodology in its
    32	header; the probe CSVs here are the evidence that earned each rule).
    33	
    34	## Rig
    35	
    36	- **Client**: the owner's Mac (Apple Silicon), data on the internal
    37	  APFS SSD (`~/blit-bench-work`, never `/tmp`).
    38	- **Daemon**: `zoey` (UNAS 8 Pro; Alpine-based aarch64, 4 slow cores,
    39	  16 GiB RAM; 8-spindle pool ~102 TiB behind a mirrored-NVMe write
    40	  tier). All daemon-side state confined to the owner's `blit-temp`
    41	  folder (standing safety rule).
    42	- **Link**: Thunderbolt 10GbE (Mac `en9`) ↔ zoey (10.1.10.206), same
    43	  /24, ~0.4 ms RTT, endpoint pinned by IP.
    44	- Owner-stated and confirmed: zoey's CPU cannot saturate the link;
    45	  cells are CPU/storage-bound (the reference is per-cell on identical
    46	  hardware, not wire-speed).
    47	
    48	## Verdict-cell results (median of 4 cold, drained, durable runs; ms)
    49	
    50	| fixture | push tcp | push grpc | pull tcp | pull grpc |
    51	|---------|---------:|----------:|---------:|----------:|
    52	| large (1 GiB)            | 2702 | 4510 | 1744 | 2585 |
    53	| small (10k × 4 KiB)      | 4263 | 5217 | 2784 | 4188 |
    54	| mixed (512 MiB + 5k×2K)  | 2070 | 3889 | 1401 | 2222 |
    55	
    56	Per-run data: `runs.csv`; avg/best alongside medians: `summary.csv`;
    57	per-run drain outcomes: `drain-outcomes.txt` (zero anomalies).
    58	Rounding: integer ms; even-count median = floor of the mean of the
    59	middle two.
    60	
    61	Sanity: TCP < gRPC in all 12 cells. 1 GiB durable ≈ 3.2 Gbit/s push /
    62	4.9 Gbit/s pull. Small files are per-file-cost bound (push ≈ 426
    63	µs/file, pull ≈ 278 µs/file on zoey's 4 slow cores — the July skippy
    64	diagnosis's per-file-bound shape at a slower constant). Old-pull beats
    65	old-push in every cell, ×1.25–×1.75 — but on THESE endpoints that gap
    66	is confounded with destination hardware (pool vs SSD), which is
    67	exactly why D-2026-07-05-1 excludes cross-direction verdicts here.
    68	
    69	## Run-to-run stability (this dataset)
    70	
    71	Zero drain anomalies; per-cell (max−min)/min spreads: 5.6–26.5%
    72	typical, worst 48.6% (`push_tcp_small` — one fast outlier run, the
    73	others within 9% of each other). The pool's tiered write path never
    74	fully stops being stateful; the MEDIAN is the cell statistic
    75	precisely because of this, and every run is visible in `runs.csv`.
    76	**otp-12 prescription**: on this rig, verdicts (especially push
    77	cells) should be confirmed by interleaved same-session A/B
    78	(old-build vs new-build alternating), not by absolute comparison
    79	alone. The old-path binaries stay staged in zoey's `blit-temp`.
    80	
    81	## Methodology findings (why the harness looks the way it does)
    82	
    83	1. **Naive transfer-return timing is a write-cache lottery**
    84	   (`probe1-no-sync-runs.csv`): per-cell spread up to 8.0× (mixed
    85	   push 1446/6119/11577 ms) purely from how much of the payload the
    86	   write tier absorbed before writeback throttled. Fix:
    87	   durable-at-destination windows.
    88	2. **Durability must be equivalent on both ends**: Linux `sync`
    89	   waits for writeback (push windows); macOS `sync(2)` only
    90	   SCHEDULES writes, so pull windows fsync every landed file instead
    91	   (media-level F_FULLFSYNC deliberately not used — the Linux side
    92	   does not pay media flush either).
    93	3. **The daemon host's write path is stateful**
    94	   (`probe2-no-drain-runs.csv`): durable-timed pushes ascend
    95	   2.7 s → 13.4 s within a session as the NVMe tier fills and
    96	   destages. Fix: sync-then-drain before every run (three
    97	   consecutive quiet 2 s windows; timeouts recorded per run label,
    98	   never silent). `probe3-drained-pushes.csv` is the manual
    99	   confirmation probe.
   100	4. **Wall clock, not monotonic**: start/end stamps are separate
   101	   processes; cross-process `time.monotonic()` has undefined
   102	   reference points and produced 0/negative windows (aborted run).
   103	5. **The durability step must time ITSELF** (codex otp-2w F3,
   104	   quantified): `ssh zoey sync` inside the window costs ~1.2 s of
   105	   connection setup (slow-core key exchange, measured) that lands
   106	   only on push cells. `probe5-sshoverhead-{runs,summary}.csv` is
   107	   the affected session — its push medians run ~0.3–0.6 s high. The
   108	   recorded dataset uses self-timed destination flushes (the remote
   109	   `sync` measures its own duration via `/proc/uptime`; the local
   110	   fsync walk reports its own elapsed), so connection/shell overhead
   111	   is excluded from every window on both rigs.
   112	
   113	## Wire-reference data (explicitly NOT verdict cells)
   114	
   115	The July 2026-07-05 measurements (`docs/bench/10gbe-2026-07-05/`)
   116	used tmpfs local ends, ARC-warm re-reads, and no sync — deliberate
   117	engine-vs-wire isolation on a different rig. Per this plan slice they
   118	are re-labeled **wire-reference only**: never compare directions or
   119	absolute times from that data against these verdict cells
   120	(D-2026-07-05-1). `probe4-prereview-session-runs.csv` is an earlier
   121	session of THIS matrix kept for cross-session corroboration.
   122	
   123	## Reproduction
   124	
   125	```
   126	export ZOEY_SSH=root@zoey
   127	export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
   128	export ZOEY_HOST=10.1.10.206
   129	RUNS=4 ./scripts/bench_otp2_baseline.sh
   130	```
   131	
   132	Requires: the staged same-commit daemon in `$ZOEY_TEMP`, a NOPASSWD
   133	sudoers rule for `/usr/sbin/purge` on the client, python3 on the
   134	client, and SSH key auth to the daemon host.

exec
/bin/zsh -lc "nl -ba docs/bench/otp2w-baseline-2026-07-10/README.md | sed -n '1,170p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
    81	push/pull ratios accordingly (codex otp-2w F3, upheld and
    82	quantified). The recorded dataset uses SELF-TIMED durability steps —
    83	the flush measures its own duration on the destination and only that
    84	is added to the transfer segment — on both rigs.
    85	
    86	## Reproduction
    87	
    88	```
    89	export WIN_SSH=michael@10.1.10.173
    90	export WIN_HOST=10.1.10.173
    91	export WIN_REPO='F:\dev\blit_v2'
    92	export WIN_TEST='D:\blit-test'
    93	RUNS=4 ./scripts/bench_otp2w_baseline.sh
    94	```
    95	
    96	Requires: daemon built on the host (`cargo build --release` in
    97	`$WIN_REPO`), OpenSSH key auth with an admin token, python3 + the
    98	NOPASSWD purge rule on the client.

exec
/bin/zsh -lc "git log --date=iso --format='%h %ad %s' -- .agents/machines.md | head -40; git blame -L 45,85 --date=short .agents/machines.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nQamd4TA' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-fDFiQ6kx' (errno=Operation not permitted)
4456aaf 2026-07-13 23:48:55 -0400 handoff: P1 REPRODUCES on a second Mac; new bench rig q; MTU prereg rev 4
0f52e6a 2026-07-13 21:21:37 -0400 machines: the Mac is a BENCH END — a rig-W session needs a quiet Mac
9aea1af 2026-07-13 16:52:43 -0400 handoff: STATE + DEVLOG for the compaction — jumbo re-run is the next action
e90409a 2026-07-13 16:45:48 -0400 machines: the fleet is on jumbo (MTU 9000) — zoey converted, fleet table recorded
c9101db 2026-07-13 16:26:26 -0400 handoff: rig facts, the macOS ping/MTU trap, and the jumbo experiment queued
56ae82e 2026-07-13 10:44:09 -0400 otp-12pf: the confound is BROKEN — P1 reproduces on a same-OS rig, so it is CODE not platform (+ codex r4: 6/6 accepted)
b0a7bd9 2026-07-12 15:09:35 -0400 otp-12b: record run-round verdict + fix sha 49dee5c; REVIEW row + DEVLOG + STATE (otp-12b closed both halves); netwatch-01 IP corrections
1fd50d7 2026-07-12 12:41:27 -0400 machines: magneto + VM are build-only fallbacks (owner rule) - never benchmark ends
d5fd17f 2026-07-12 12:29:42 -0400 machines: record magneto (Arch x86_64, 10GbE) + the Ubuntu ARM VM (owner-offered Linux hosts)
b2b6901 2026-07-12 09:23:22 -0400 otp-2 evidence correction: zoey's staged daemon was a dirty 731023b build, not e757dcc
5b39d97 2026-07-11 12:13:02 -0400 STATE under the 200-line cap: refresh stale top block; machine-local facts to .agents/machines.md [state: skip]
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-oBuoS7WI' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-eiTJ05QD' (errno=Operation not permitted)
c9101db4 (Michael Coelho 2026-07-13 45) - **Jumbo works end-to-end at MTU 9000** (verified 2026-07-13 by real
c9101db4 (Michael Coelho 2026-07-13 46)   TCP, not ping): Mac↔Windows 231/225 MB/s, Mac↔skippy 157 MB/s (all
c9101db4 (Michael Coelho 2026-07-13 47)   ssh-encrypted, so CPU-bound floors — the wire is not the limit). The
c9101db4 (Michael Coelho 2026-07-13 48)   UniFi switching passes 9018-byte frames fine.
c9101db4 (Michael Coelho 2026-07-13 49) - **Windows (netwatch-01) ran at MTU 1500 for EVERY benchmark ever
c9101db4 (Michael Coelho 2026-07-13 50)   recorded** (otp-2w, otp-12a/b/c). It was raised to 9000 on 2026-07-13.
c9101db4 (Michael Coelho 2026-07-13 51)   Every prior measurement therefore negotiated down to a 1460-byte MSS:
c9101db4 (Michael Coelho 2026-07-13 52)   **jumbo has never been exercised in a blit benchmark.** Those numbers
c9101db4 (Michael Coelho 2026-07-13 53)   are valid — they are simply *1500-MTU* numbers — and rig W at jumbo is
c9101db4 (Michael Coelho 2026-07-13 54)   a genuinely untested condition. magneto is still 1500 (raise
c9101db4 (Michael Coelho 2026-07-13 55)   `enp1s0f1` to 9000 to make the Linux rig jumbo too).
c9101db4 (Michael Coelho 2026-07-13 56) - Mismatched MTUs on one L2 segment are fine: TCP MSS negotiation
c9101db4 (Michael Coelho 2026-07-13 57)   handles it, each host advertising what it can receive. What is NOT
c9101db4 (Michael Coelho 2026-07-13 58)   fine is a host advertising a size it cannot actually send.
e90409ac (Michael Coelho 2026-07-13 59) - **Fleet MTU as of 2026-07-13 — the whole 10 GbE fabric is now 9000:**
e90409ac (Michael Coelho 2026-07-13 60) 
e90409ac (Michael Coelho 2026-07-13 61)   | host | iface | MTU | persistent? |
e90409ac (Michael Coelho 2026-07-13 62)   |---|---|---|---|
e90409ac (Michael Coelho 2026-07-13 63)   | Mac | `en9` (Aquantia) | 9000 | yes (macOS net service) |
e90409ac (Michael Coelho 2026-07-13 64)   | netwatch-01 | Ethernet | 9000 | yes (raised 1500→9000 today) |
e90409ac (Michael Coelho 2026-07-13 65)   | skippy | `enp66s0f1` | 9000 | yes |
e90409ac (Michael Coelho 2026-07-13 66)   | **zoey** | `enp0s0` (RJ45, NFS data .206) | **9000** | yes — `[Link] MTUBytes=9000` in `/etc/systemd/network/enp0s0.network` |
e90409ac (Michael Coelho 2026-07-13 67)   | **zoey** | `enp0s1` (SFP, mgmt .210) | **9000** | yes — same, in `enp0s1.network` |
e90409ac (Michael Coelho 2026-07-13 68)   | altiera | `enp1s0`/`enp2s0` | 9000 | yes (NetworkManager profiles) |
9aea1af2 (Michael Coelho 2026-07-13 69)   | magneto | `enp1s0f1` | 9000 | yes — NM profile `Wired connection 3` saved `mtu=9000` (2026-07-13) |
9aea1af2 (Michael Coelho 2026-07-13 70) 
9aea1af2 (Michael Coelho 2026-07-13 71)   **Verified end-to-end 2026-07-13**: a jumbo DF ping from skippy reaches
9aea1af2 (Michael Coelho 2026-07-13 72)   magneto, zoey, altiera, netwatch-01 AND the Mac — all OK. Every 10 GbE
9aea1af2 (Michael Coelho 2026-07-13 73)   pair in the fleet carries 9000-byte frames. (Always test from a LINUX
9aea1af2 (Michael Coelho 2026-07-13 74)   host; the Mac's `ping` cannot send >8192 — see the raw-socket trap.)
e90409ac (Michael Coelho 2026-07-13 75) 
e90409ac (Michael Coelho 2026-07-13 76) - **zoey (UniFi UNAS Pro) jumbo — how it was done, and the trap.**
e90409ac (Michael Coelho 2026-07-13 77)   Debian 11 + `systemd-networkd`; NIC `maxmtu` is 9216 so the hardware is
e90409ac (Michael Coelho 2026-07-13 78)   fine. Persistence = a `[Link]` / `MTUBytes=9000` stanza in each
e90409ac (Michael Coelho 2026-07-13 79)   `/etc/systemd/network/enp0s*.network` (originals backed up as
e90409ac (Michael Coelho 2026-07-13 80)   `*.premtu`). Proven by `networkctl reload && networkctl reconfigure`
e90409ac (Michael Coelho 2026-07-13 81)   with the static IP intact — no reboot needed. **TRAP: `/` is an
e90409ac (Michael Coelho 2026-07-13 82)   overlayfs** (`lowerdir=/mnt/.rofs` read-only + writable upper), so a
e90409ac (Michael Coelho 2026-07-13 83)   UniFi *firmware update* can replace the base image and silently drop
e90409ac (Michael Coelho 2026-07-13 84)   this. Re-check after any UNAS update:
e90409ac (Michael Coelho 2026-07-13 85)   `ssh root@zoey 'cat /sys/class/net/enp0s0/mtu'` → want 9000.

exec
/bin/zsh -lc "git log --all --date=iso --format='%h %ad %s' -- docs/bench/otp2-baseline-2026-07-10 docs/bench/otp2w-baseline-2026-07-10 scripts/bench_otp2_baseline.sh scripts/bench_otp2w_baseline.sh | head -80" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-xzeLIf0W' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1dpCucHR' (errno=Operation not permitted)
fa18787 2026-07-12 12:16:15 -0400 otp-12a: address run-round review (6 findings)
b2b6901 2026-07-12 09:23:22 -0400 otp-2 evidence correction: zoey's staged daemon was a dirty 731023b build, not e757dcc
7e732d4 2026-07-10 17:49:00 -0400 otp-2w: address review (7 findings); self-timed durability, both matrices re-run
ceea6ed 2026-07-10 16:43:26 -0400 otp-2/otp-2w: commit the drain outcomes (*.log is gitignored)
0c43d2a 2026-07-10 16:42:42 -0400 otp-2w: record the Windows cross-direction baseline; otp-2 closed both halves
4286c23 2026-07-10 15:43:39 -0400 otp-2: address review (8 findings); re-run baseline under the fixed harness
e757dcc 2026-07-10 13:59:02 -0400 otp-2: record the symmetric disk-to-disk baseline (harness + evidence)

codex
The first pass confirms the central evidence is narrower than the decision’s wording: pf-0’s 3–4% observation is a rig-W, `wm_tcp_large` result, while the re-record decision applies it to every rig-Z acceptance cell. I’m now checking whether repository history independently establishes Zoey’s July 10 MTU and whether the re-run controls non-MTU drift.
exec
/bin/zsh -lc 'find docs/bench/otp2-baseline-2026-07-10 docs/bench/otp2w-baseline-2026-07-10 -maxdepth 1 -type f -print | sort; for f in docs/bench/otp2-baseline-2026-07-10/meta.csv docs/bench/otp2w-baseline-2026-07-10/meta.csv docs/bench/otp2-baseline-2026-07-10/staging-manifest.txt docs/bench/otp2w-baseline-2026-07-10/staging-manifest.txt; do if test -f "$f"; then echo "FILE $f"; nl -ba "$f"; fi; done' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/bench/otp2-baseline-2026-07-10/README.md
docs/bench/otp2-baseline-2026-07-10/drain-outcomes.txt
docs/bench/otp2-baseline-2026-07-10/probe1-no-sync-runs.csv
docs/bench/otp2-baseline-2026-07-10/probe2-no-drain-runs.csv
docs/bench/otp2-baseline-2026-07-10/probe3-drained-pushes.csv
docs/bench/otp2-baseline-2026-07-10/probe4-prereview-session-runs.csv
docs/bench/otp2-baseline-2026-07-10/probe5-sshoverhead-runs.csv
docs/bench/otp2-baseline-2026-07-10/probe5-sshoverhead-summary.csv
docs/bench/otp2-baseline-2026-07-10/runs.csv
docs/bench/otp2-baseline-2026-07-10/summary.csv
docs/bench/otp2w-baseline-2026-07-10/README.md
docs/bench/otp2w-baseline-2026-07-10/drain-outcomes.txt
docs/bench/otp2w-baseline-2026-07-10/probe1-sshoverhead-runs.csv
docs/bench/otp2w-baseline-2026-07-10/probe1-sshoverhead-summary.csv
docs/bench/otp2w-baseline-2026-07-10/runs.csv
docs/bench/otp2w-baseline-2026-07-10/summary.csv

exec
/bin/zsh -lc "rg -n -i \"zoey.{0,80}(mtu|jumbo)|(mtu|jumbo).{0,80}zoey|1500.{0,80}zoey|zoey.{0,80}1500\" --glob '"'!DEVLOG.md'"' --glob '"'!target/**'"' ." in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
./docs/STATE.md:6:- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.

exec
/bin/zsh -lc 'git show --format=fuller --stat e90409a; git show --format= -- .agents/machines.md e90409a' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-wJnw0vHt' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-q91277UP' (errno=Operation not permitted)
commit e90409ac12ed2fdcdbf98239320c21fb9d31ffb2
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Mon Jul 13 16:45:48 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Mon Jul 13 16:45:48 2026 -0400

    machines: the fleet is on jumbo (MTU 9000) — zoey converted, fleet table recorded
    
    zoey (UniFi UNAS Pro, Debian 11 + systemd-networkd) converted 1500 -> 9000 on
    BOTH NICs (enp0s0 RJ45 = the NFS data path .206; enp0s1 SFP = mgmt .210). NIC
    maxmtu is 9216, so the hardware was always capable. Persisted as a [Link] /
    MTUBytes=9000 stanza per .network file, proven by a live networkctl reconfigure
    with the static IP intact (no reboot). Originals kept as *.premtu. Verified with
    a jumbo DF ping FROM altiera (a Linux box — the Mac's ping cannot be trusted for
    this, see the raw-socket trap above); NFS stayed readable throughout, 200MB read
    at ~190 MB/s, ssh never dropped.
    
    Recorded because they will bite someone later:
    
    - zoey's / is an OVERLAYFS: a UniFi firmware update can replace the read-only
      base and silently revert the MTU. Re-check after any UNAS update.
    - Safe method for a remote NIC change: arm a self-healing auto-revert BEFORE
      touching the interface, and change the NIC you are not ssh'd through.
    - Live TCP/NFS connections do NOT adopt a new MTU. MSS is fixed at connect time,
      so an existing mount keeps its old frame size until it reconnects.
    - magneto is 9000 at RUNTIME ONLY (its NM profile still says mtu=auto), so it
      reverts on reboot until the profile is saved.
    - altiera AND zoey both run two NICs on one subnet with arp_ignore=0 — ARP flux
      and asymmetric routing; working today, latently fragile.
    
    Fleet is now uniformly 9000, which matters for blit: Windows sat at 1500 for
    EVERY benchmark ever recorded, so jumbo has never once been exercised. The
    rig-W invariance re-run at jumbo is Queue 1a.

 .agents/machines.md | 33 +++++++++++++++++++++++++++++++++
 1 file changed, 33 insertions(+)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uVdX4goj' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-kUV1B6su' (errno=Operation not permitted)

exec
/bin/zsh -lc "git log --all -G 'zoey.*(MTU|mtu|jumbo)|(?:MTU|mtu|jumbo).*zoey' --date=iso --format='%h %ad %s' -- . ':"'!DEVLOG.md'"'; git log --all -S 'MTUBytes=9000' --date=iso --format='%h %ad %s' -- ." in /Users/michael/Dev/blit_v2
 succeeded in 440ms:
exec
/bin/zsh -lc 'rg -n "BASELINE|old_committed|old_session|reference|verdicts.csv|combined|comparison" scripts/bench_otp12_delegated.sh scripts/bench_otp12_win.sh scripts/bench_otp12_zoey.sh' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/bench_otp12_zoey.sh:7:# {push,pull} x {tcp,grpc} = 12 comparisons) rerun as matched-pair A/B —
scripts/bench_otp12_zoey.sh:13:# references — the same-session old arm AND the committed 2026-07-10
scripts/bench_otp12_zoey.sh:29:#     2*RUNS pair attempts per comparison; at the cap the comparison is
scripts/bench_otp12_zoey.sh:33:#   * verdicts.csv computed at the end against both references
scripts/bench_otp12_zoey.sh:36:#   * Escalation (manual, design D2): a comparison that straddles its
scripts/bench_otp12_zoey.sh:82:# Comma-separated comparison allowlist for the D2 escalation rule
scripts/bench_otp12_zoey.sh:84:# comparisons; both sessions committed). Empty = the full matrix.
scripts/bench_otp12_zoey.sh:100:# The committed reference is FIXED (pre-registered, design D2) — no env
scripts/bench_otp12_zoey.sh:102:BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2-baseline-2026-07-10/summary.csv"
scripts/bench_otp12_zoey.sh:153:    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
scripts/bench_otp12_zoey.sh:222:    h_ref=$(sha256_local "$BASELINE_SUMMARY")
scripts/bench_otp12_zoey.sh:229:        echo "-,reference,-,$h_ref,$BASELINE_SUMMARY"
scripts/bench_otp12_zoey.sh:471:run_comparison() {   # cell kind src_or_remote [flags...]
scripts/bench_otp12_zoey.sh:511:# --- Verdicts (design D2: both references must pass) --------------------
scripts/bench_otp12_zoey.sh:513:    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
scripts/bench_otp12_zoey.sh:534:# A cell is usable only when its comparison completed (RUNS valid
scripts/bench_otp12_zoey.sh:537:# verdict loop iterates EVERY attempted comparison (meta), so a
scripts/bench_otp12_zoey.sh:557:    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
scripts/bench_otp12_zoey.sh:560:            f.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
scripts/bench_otp12_zoey.sh:566:            # committed reference row; a miss is a harness/reference
scripts/bench_otp12_zoey.sh:568:            sys.exit(f"FATAL: no committed reference row for {cell} in {base_p}")
scripts/bench_otp12_zoey.sh:572:        f.write(f"{cell},converge,new,old_session,{new_m},{old_m},"
scripts/bench_otp12_zoey.sh:574:        f.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},"
scripts/bench_otp12_zoey.sh:576:        combined = ("PASS" if p1 and p2
scripts/bench_otp12_zoey.sh:580:        f.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
scripts/bench_otp12_zoey.sh:602:        if want_cell "push_tcp_${w}";  then run_comparison "push_tcp_${w}"  push "$MAC_WORK/src_$w"; fi
scripts/bench_otp12_zoey.sh:603:        if want_cell "push_grpc_${w}"; then run_comparison "push_grpc_${w}" push "$MAC_WORK/src_$w" --force-grpc; fi
scripts/bench_otp12_zoey.sh:604:        if want_cell "pull_tcp_${w}";  then run_comparison "pull_tcp_${w}"  pull "${REMOTE}pull_src_$w/src_$w/"; fi
scripts/bench_otp12_zoey.sh:605:        if want_cell "pull_grpc_${w}"; then run_comparison "pull_grpc_${w}" pull "${REMOTE}pull_src_$w/src_$w/" --force-grpc; fi
scripts/bench_otp12_zoey.sh:613:                || die "CELLS entry '$c' matched no comparison — nothing was measured for it"
scripts/bench_otp12_zoey.sh:621:    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
scripts/bench_otp12_zoey.sh:624:    log "=== VERDICTS (design D2: PASS needs BOTH references) ==="
scripts/bench_otp12_zoey.sh:625:    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
scripts/bench_otp12_delegated.sh:347:  wssh "\$ErrorActionPreference='Stop'
scripts/bench_otp12_delegated.sh:355:  wssh "\$ErrorActionPreference='Stop'
scripts/bench_otp12_delegated.sh:397:  wssh "\$ErrorActionPreference='Stop'
scripts/bench_otp12_delegated.sh:633:  python3 - "$RUNS_CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" "$OUT_DIR/drain-outcomes.txt" <<'PYEOF'
scripts/bench_otp12_delegated.sh:674:# verdicts.csv (plan D5 schema) — delegated parity, bar max/min <= 1.10
scripts/bench_otp12_delegated.sh:676:    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
scripts/bench_otp12_delegated.sh:757:      tail -n +2 "$META" | grep -q "^$c," || die "CELLS entry '$c' matched no comparison — nothing measured"
scripts/bench_otp12_delegated.sh:764:  log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
scripts/bench_otp12_delegated.sh:768:  column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
scripts/bench_otp12_win.sh:8:#   comparisons, matched-pair interleaved A/B — arm "old" = the pinned
scripts/bench_otp12_win.sh:11:#   = the run commit's pair. Verdicts against BOTH references (the
scripts/bench_otp12_win.sh:22:#   also gets converge rows against its data direction's old references
scripts/bench_otp12_win.sh:104:# Fixed committed reference (pre-registered, D2) — no override.
scripts/bench_otp12_win.sh:105:BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2w-baseline-2026-07-10/summary.csv"
scripts/bench_otp12_win.sh:126:    v=$(wssh "\$ErrorActionPreference = 'Stop'; \$sw = [Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache $WIN_DRIVE; \$sw.Stop(); \"F:\$([int]\$sw.Elapsed.TotalMilliseconds):F\"" 2>/dev/null \
scripts/bench_otp12_win.sh:170:    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
scripts/bench_otp12_win.sh:222:    h_ref=$(sha256_local "$BASELINE_SUMMARY")
scripts/bench_otp12_win.sh:231:        echo "-,reference,-,$h_ref,$BASELINE_SUMMARY"
scripts/bench_otp12_win.sh:276:    wssh "\$ErrorActionPreference = 'Stop'
scripts/bench_otp12_win.sh:369:    wssh '$ErrorActionPreference = "Stop"
scripts/bench_otp12_win.sh:386:    # the row before its `valid` field — every comparison then reads
scripts/bench_otp12_win.sh:551:# One interleaved comparison; ABBA; pair-void; INCOMPLETE at the cap.
scripts/bench_otp12_win.sh:643:    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
scripts/bench_otp12_win.sh:672:out.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
scripts/bench_otp12_win.sh:687:# Block 1: converge-up, both references (12a logic verbatim).
scripts/bench_otp12_win.sh:691:        out.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
scripts/bench_otp12_win.sh:695:        sys.exit(f"FATAL: no committed reference row for {cell}")
scripts/bench_otp12_win.sh:698:    out.write(f"{cell},converge,new,old_session,{new_m},{old_m},{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
scripts/bench_otp12_win.sh:699:    out.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
scripts/bench_otp12_win.sh:700:    combined = ("PASS" if p1 and p2 else "FAIL-REFERENCE-DRIFT" if p1
scripts/bench_otp12_win.sh:702:    out.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
scripts/bench_otp12_win.sh:716:    # Committed references are MANDATORY (fail closed, codex otp-12b
scripts/bench_otp12_win.sh:717:    # F8); the same-session reference requires the block-1 counterpart
scripts/bench_otp12_win.sh:724:        sys.exit(f"FATAL: no committed reference row for {b1} (needed by {cell})")
scripts/bench_otp12_win.sh:729:            out.write(f"{cell},converge,{armname},old_session,{val},{old_sess},{val/old_sess:.3f},1.10,{'PASS' if bar(val, old_sess) else 'FAIL'}\n")
scripts/bench_otp12_win.sh:731:            out.write(f"{cell},converge,{armname},old_session,{val},,,1.10,NO-SAME-SESSION-REF\n")
scripts/bench_otp12_win.sh:732:        out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
scripts/bench_otp12_win.sh:738:        sys.exit(f"FATAL: committed push/pull reference missing for {carrier}_{fixture}")
scripts/bench_otp12_win.sh:741:    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
scripts/bench_otp12_win.sh:815:                || die "CELLS entry '$c' matched no comparison — nothing was measured for it"
scripts/bench_otp12_win.sh:824:    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
scripts/bench_otp12_win.sh:827:    log "=== VERDICTS (D2 both-references; invariance; F4 cross + gap rows) ==="
scripts/bench_otp12_win.sh:828:    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"

exec
/bin/zsh -lc 'rg -n "D-2026-07-05-4|D-2026-07-12-1|D2|D5|baseline|frozen|freeze|pin|old_committed|acceptance|rig [WZD]|Rig [WZD]|criterion|row" docs/DECISIONS.md docs/plan/OTP12_ACCEPTANCE_RUN.md' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba docs/DECISIONS.md | sed -n '35,105p'; nl -ba docs/DECISIONS.md | sed -n '145,218p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/plan/OTP12_ACCEPTANCE_RUN.md:1:# otp-12 — symmetric-rig acceptance run (design)
docs/plan/OTP12_ACCEPTANCE_RUN.md:4:question was ruled by D-2026-07-12-1; design codex round closed at
docs/plan/OTP12_ACCEPTANCE_RUN.md:8:**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-12.
docs/plan/OTP12_ACCEPTANCE_RUN.md:14:headline criterion) and P2 (`push_tcp_small` 1.149 → 1.201).
docs/plan/OTP12_ACCEPTANCE_RUN.md:19:**D-2026-07-12-1's platform-residue discriminator is the frame for it at the
docs/plan/OTP12_ACCEPTANCE_RUN.md:26:from the current rows** — they are pre-fix, and that plan's `pf-final` voids
docs/plan/OTP12_ACCEPTANCE_RUN.md:27:pre-fix unified arms for acceptance. Sequence:
docs/plan/OTP12_ACCEPTANCE_RUN.md:42:otp-12 is the plan's acceptance-evidence slice: rerun the otp-2 matrix on the
docs/plan/OTP12_ACCEPTANCE_RUN.md:44:the better old direction + noise (`ONE_TRANSFER_PATH.md` slice 12, acceptance
docs/plan/OTP12_ACCEPTANCE_RUN.md:49:rationale (`docs/bench/otp2-baseline-2026-07-10/README.md` §Methodology
docs/plan/OTP12_ACCEPTANCE_RUN.md:50:findings, `docs/bench/otp2w-baseline-2026-07-10/README.md` §Timing-overhead
docs/plan/OTP12_ACCEPTANCE_RUN.md:55:1. **Invariance matrix** (criterion 1): per data direction × workload
docs/plan/OTP12_ACCEPTANCE_RUN.md:58:2. **Converge-up matrix** (criterion 2 / codex F4): every unified cell ≤ the
docs/plan/OTP12_ACCEPTANCE_RUN.md:60:   recorded old-path baselines, confirmed by interleaved same-session
docs/plan/OTP12_ACCEPTANCE_RUN.md:103:- Baselines on record: `docs/bench/otp2-baseline-2026-07-10/` (zoey,
docs/plan/OTP12_ACCEPTANCE_RUN.md:105:  corollary) and `docs/bench/otp2w-baseline-2026-07-10/` (Mac↔Windows, the
docs/plan/OTP12_ACCEPTANCE_RUN.md:120:| **D** | Windows daemon ↔ skippy daemon (TrueNAS, x86_64), Mac as delegating CLI | delegated-vs-direct parity (trigger invariance) | owner-designated delegated rig; no old baseline exists on this pair |
docs/plan/OTP12_ACCEPTANCE_RUN.md:124:a substitution records fresh baselines and is per-direction only.
docs/plan/OTP12_ACCEPTANCE_RUN.md:134:arm (8 timed runs per comparison). A = `old` (rig Z/W converge-up) or
docs/plan/OTP12_ACCEPTANCE_RUN.md:135:`delegated` (rig D). Interleaving is the verdict method, not a nicety:
docs/plan/OTP12_ACCEPTANCE_RUN.md:140:outside the timed window. Old arms exist only where an old baseline exists
docs/plan/OTP12_ACCEPTANCE_RUN.md:143:delegated baseline.
docs/plan/OTP12_ACCEPTANCE_RUN.md:146:sha, all hosts). Old arms = the pinned baseline shas (`e757dcc` zoey,
docs/plan/OTP12_ACCEPTANCE_RUN.md:147:`0f922de` Windows). Old-arm Mac clients are rebuilt at the pinned sha in a
docs/plan/OTP12_ACCEPTANCE_RUN.md:154:### D2 — verdict arithmetic (what the evidence computes; the owner declares)
docs/plan/OTP12_ACCEPTANCE_RUN.md:156:All statistics per the recorded baselines: integer ms; median of 4, even
docs/plan/OTP12_ACCEPTANCE_RUN.md:170:  interleaved old arm AND the committed 2026-07-10 baseline median for
docs/plan/OTP12_ACCEPTANCE_RUN.md:176:  data direction — both initiators on rig W, both blocks — must meet
docs/plan/OTP12_ACCEPTANCE_RUN.md:180:- **Invariance (rig W, hard bar — the owner's sentence)**: per fixture ×
docs/plan/OTP12_ACCEPTANCE_RUN.md:182:  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
docs/plan/OTP12_ACCEPTANCE_RUN.md:183:  rows; grpc rows are recorded, same bar, labeled secondary.
docs/plan/OTP12_ACCEPTANCE_RUN.md:184:- **Delegated parity (rig D, hard bar)**: per fixture × direction,
docs/plan/OTP12_ACCEPTANCE_RUN.md:186:- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
docs/plan/OTP12_ACCEPTANCE_RUN.md:198:  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
docs/plan/OTP12_ACCEPTANCE_RUN.md:208:the escalation's entire purpose. The RUNS=4 rows stay committed and
docs/plan/OTP12_ACCEPTANCE_RUN.md:211:### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
docs/plan/OTP12_ACCEPTANCE_RUN.md:253:### D4 — delegated cells = delegated-vs-direct parity (rig D)
docs/plan/OTP12_ACCEPTANCE_RUN.md:281:(large × skippy→Windows, both arms) is recorded as a smoke row — carrier
docs/plan/OTP12_ACCEPTANCE_RUN.md:284:measured properly on rig W.
docs/plan/OTP12_ACCEPTANCE_RUN.md:288:direction); bench modules writable, `delegation_allowed` not narrowed.
docs/plan/OTP12_ACCEPTANCE_RUN.md:290:### D5 — three self-contained scripts; the frozen baselines stay frozen
docs/plan/OTP12_ACCEPTANCE_RUN.md:292:> **AMENDED by D-2026-07-14-1 (2026-07-14) — the *pin* moves once; the *freeze*
docs/plan/OTP12_ACCEPTANCE_RUN.md:293:> stands.** The committed baselines this section pins were recorded at **MTU
docs/plan/OTP12_ACCEPTANCE_RUN.md:296:> not conservative**. Each rig's committed baseline is therefore **re-recorded
docs/plan/OTP12_ACCEPTANCE_RUN.md:297:> once with its ORIGINAL old build at MTU 9000** and re-frozen; the 2026-07-10
docs/plan/OTP12_ACCEPTANCE_RUN.md:298:> baselines are retained as historical MTU-1500 records. Immutability and the
docs/plan/OTP12_ACCEPTANCE_RUN.md:305:`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
docs/plan/OTP12_ACCEPTANCE_RUN.md:310:  nonzero exit voids the interleave pair per the D2 valid-run rule — a
docs/plan/OTP12_ACCEPTANCE_RUN.md:316:(`valid` = the PAIR's fate under the D2 valid-run rule — an
docs/plan/OTP12_ACCEPTANCE_RUN.md:321:(medians over valid runs only — the D2 valid-run rule)
docs/plan/OTP12_ACCEPTANCE_RUN.md:328:discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
docs/plan/OTP12_ACCEPTANCE_RUN.md:334:rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
docs/plan/OTP12_ACCEPTANCE_RUN.md:335:carries the registered D2 set
docs/plan/OTP12_ACCEPTANCE_RUN.md:337:`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
docs/plan/OTP12_ACCEPTANCE_RUN.md:338:converge row whose same-session block-1 counterpart is absent or
docs/plan/OTP12_ACCEPTANCE_RUN.md:340:artifact — the committed-reference row still governs). Nothing else is
docs/plan/OTP12_ACCEPTANCE_RUN.md:341:legal, and a missing committed-reference row aborts the verdict pass
docs/plan/OTP12_ACCEPTANCE_RUN.md:368:| Mac | rebuild client at the pinned sha in a detached worktree → `~/blit-bench-work/bins/blit-<sha>` | `cargo build --release` at the run commit |
docs/plan/OTP12_ACCEPTANCE_RUN.md:371:| skippy | none (no old baseline; July binaries unusable) | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (static — sidesteps the recorded glibc 2.36 ceiling) → `$SKIPPY_BIN/bins/<sha>/` (pool paths are exec-friendly; `/tmp` and `/home` are noexec) — `blit` + `blit-daemon` |
docs/plan/OTP12_ACCEPTANCE_RUN.md:396:- **otp-12a — rig Z**: `bench_otp12_zoey.sh` (harness commit; codex; fix) →
docs/plan/OTP12_ACCEPTANCE_RUN.md:401:- **otp-12b — rig W**: `bench_otp12_win.sh` covering converge-up block +
docs/plan/OTP12_ACCEPTANCE_RUN.md:405:- **otp-12c — rig D**: `bench_otp12_delegated.sh`; same shape. Preflight
docs/plan/OTP12_ACCEPTANCE_RUN.md:410:- **otp-12d — assembly**: `docs/bench/otp12-acceptance-<date>/README.md` —
docs/plan/OTP12_ACCEPTANCE_RUN.md:411:  the plan-level verdict matrix assembling every comparison row
docs/plan/OTP12_ACCEPTANCE_RUN.md:412:  criterion-by-criterion (the artifact otp-13 walks). Docs-only commit.
docs/plan/OTP12_ACCEPTANCE_RUN.md:413:  The plan's acceptance-criteria checkboxes are NOT flipped here — that
docs/plan/OTP12_ACCEPTANCE_RUN.md:424:(sha256 per binary per host). `docs/bench/otp12-acceptance-<date>/README.md`
docs/plan/OTP12_ACCEPTANCE_RUN.md:430:  instantiated by the owner-designated closest-spec pair; rig W's two
docs/plan/OTP12_ACCEPTANCE_RUN.md:432:  Defender at its normal state). D2's discriminator computation is the
docs/plan/OTP12_ACCEPTANCE_RUN.md:434:  as satisfied per D-2026-07-12-1.
docs/plan/OTP12_ACCEPTANCE_RUN.md:450:## Open questions — RESOLVED (owner, 2026-07-12; D-2026-07-12-1)
docs/plan/OTP12_ACCEPTANCE_RUN.md:452:- **Q1 — cross-direction residue on rig W**: RESOLVED "yes" — a cell that
docs/plan/OTP12_ACCEPTANCE_RUN.md:456:  **counts as satisfying the cross-direction half of criterion 2**
docs/plan/OTP12_ACCEPTANCE_RUN.md:457:  (D-2026-07-12-1). The evidence still records both computations per
docs/DECISIONS.md:21:## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
docs/DECISIONS.md:22:- Decision: `RELEASE_PLAN_v2_2026-05-04.md` is a frozen reference, no longer the active source of truth.
docs/DECISIONS.md:63:- Decision: `docs/plan/DESIGN_COHERENCE_REVIEW.md` flipped Draft → Active. Owner approval authorizes **Phase A only** (concept-ownership map + per-crate stratum inventory); Phases B and C each need a fresh go/no-go at the preceding checkpoint. Interview decisions bound into the plan: blit-tui light pass, owner ratifies each Phase C finding, wire-breaking recommendations in scope (proto not frozen).
docs/DECISIONS.md:64:- Why: the repo was built by many models across several greenfield restarts and the owner judges it too inconsistently designed to trust as-is; mapping concept ownership precedes any re-scope (audit-h3c slice 2) or feature landing (adaptive-streams) so the fixes get designed once.
docs/DECISIONS.md:68:- Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
docs/DECISIONS.md:94:## D-2026-06-20-4 — Unified transfer engine plan review freeze
docs/DECISIONS.md:95:- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
docs/DECISIONS.md:100:- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
docs/DECISIONS.md:102:- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*
docs/DECISIONS.md:105:- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
docs/DECISIONS.md:107:- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.
docs/DECISIONS.md:115:- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
docs/DECISIONS.md:116:- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
docs/DECISIONS.md:120:- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
docs/DECISIONS.md:125:- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
docs/DECISIONS.md:130:- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
docs/DECISIONS.md:132:- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.
docs/DECISIONS.md:135:- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
docs/DECISIONS.md:137:- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).
docs/DECISIONS.md:144:## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
docs/DECISIONS.md:145:- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
docs/DECISIONS.md:154:## D-2026-07-10-1 — Resume wire bounds on the in-stream carrier (amends OTP7_RESUME D5)
docs/DECISIONS.md:156:- Why: plan D5 as drafted clamped only to `MAX_BLOCK_SIZE` (64 MiB), which is fine for local block copies but 16× over the unraised tonic frame limit the served in-stream carrier actually has — a legal open would fail mid-transfer (RELIABLE violation), and a hostile-or-buggy tiny block size would OOM-amplify the hash list. Pinned by `resume_block_size_floor_clamps_tiny_requests`, `resume_block_size_ceiling_clamps_oversized_requests` (guard-proven by clamp removal), and the pure-fn cap boundary test.
docs/DECISIONS.md:157:- Supersedes: OTP7_RESUME.md D5's "clamped to `MAX_BLOCK_SIZE`" wording (amended in place, same commit).
docs/DECISIONS.md:162:- Supersedes: nothing — completes the revisit D-2026-07-10-1 explicitly deferred to otp-7b (OTP7_RESUME.md D5 amended in place, same commit).
docs/DECISIONS.md:165:- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
docs/DECISIONS.md:169:## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
docs/DECISIONS.md:170:- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
docs/DECISIONS.md:172:- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
docs/DECISIONS.md:181:- Why: two reasons, one causal and one procedural. **Causal**: the local finding is very unlikely to explain either otp-12 finding. P1 is an *initiator-invariance* failure — both arms run identical code and differ only in who dials, so a worker-count or per-file cost cancels between them, and a local copy has no initiator axis at all. P2 is a *new-vs-old* regression, whereas the local cost is *old*: otp-11's own gate measured old-vs-new local `small` at 1684 -> 1750 ms (+3.9% PASS, `docs/bench/otp11-local-2026-07-11/`) and otp-11 D1 explicitly preserved the old pipeline's payload shapes (`PreparedPayload::File`/`TarShard` "exactly as the old local pipeline"). A long-standing cost cannot produce a new regression. **Procedural**: fixing local *first* would touch code shared with the wire sink, perturb P1/P2 mid-investigation, and void the pre-fix baselines pf-final depends on — destroying the attribution rather than adding to it. Sequencing behind keeps every counterfactual legible, and pf-final's full-matrix rerun would still surface any shared-code effect as a number.
docs/DECISIONS.md:194:## D-2026-07-14-1 — the committed baselines are RE-RECORDED at MTU 9000 (amends OTP12_ACCEPTANCE_RUN D5's pin, not its freeze)
docs/DECISIONS.md:195:- Decision: the frozen committed baselines that `pf-final` grades against are **re-recorded with their OLD builds at MTU 9000**, so acceptance compares old and new like-for-like on the fabric the fleet actually runs. Owner, 2026-07-14, choosing between three presented options, verbatim: **"Re-record the baseline at 9000"**. The 2026-07-10 baselines are **retained as historical MTU-1500 records** — superseded as the acceptance reference, never deleted or rewritten.
docs/DECISIONS.md:196:- Why: pf-0 (`docs/bench/otp12-jumbo-win-2026-07-13/`) measured jumbo making **both arms 3–4% faster**. Grading a jumbo NEW arm against a **1500-recorded** ceiling is therefore **LENIENT, not conservative** — the MTU gain flatters the ratio, so a real regression up to roughly the size of that gain could pass unseen. P1 is the one finding between blit and shipping; a lenient ceiling is the wrong error to accept there.
docs/DECISIONS.md:197:- Scope — **BOTH rigs, not just rig W.** Each harness hardcodes its own committed reference, and both predate the 2026-07-13 fabric-wide jumbo raise (`.agents/machines.md`): rig W `scripts/bench_otp12_win.sh:105` → `docs/bench/otp2w-baseline-2026-07-10/`; rig Z `scripts/bench_otp12_zoey.sh:102` → `docs/bench/otp2-baseline-2026-07-10/`. Rig D (delegated) has **no** old baseline and is unaffected.
docs/DECISIONS.md:198:- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
docs/DECISIONS.md:199:  * **Each rig's re-baseline MUST use the same OLD build as its original baseline**, with provenance manifest-verified — rig W `0f922de`; rig Z the build staged in `blit-temp` (which embeds `731023bfc8a1.dirty`, **not** `e757dcc` — see the otp-2 README correction). A re-baseline on a different old build would silently change the reference twice.
docs/DECISIONS.md:201:  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
docs/DECISIONS.md:202:- Supersedes: the *pin* in `OTP12_ACCEPTANCE_RUN.md` D5 ("the frozen baselines stay frozen") — the **freeze principle stands** (a baseline is immutable once recorded, and no run may re-point its own reference), but the acceptance reference is re-recorded once, at the fabric's MTU, and re-frozen. Closes the OPEN item raised in `OTP12_PERF_FINDINGS.md` §pf-0.

 succeeded in 0ms:
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
   178	
   179	## D-2026-07-13-2 — the local small-file finding queues BEHIND OTP12_PERF_FINDINGS
   180	- Decision: `docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft) is sequenced **behind** the ACTIVE `docs/plan/OTP12_PERF_FINDINGS.md` — the MTU experiment, then pf-1, then its fix slices. Owner, 2026-07-13, verbatim: **"well, odds that one affects the other? if this is contributory, would we know? probably irrelevant. behind."** No local-path code lands until otp-12's investigation has its attribution. The finding itself (blit vs robocopy, local `D: -> E:`, `docs/bench/win-local-ab-2026-07-13/`) is recorded now; only the *fix* waits.
   181	- Why: two reasons, one causal and one procedural. **Causal**: the local finding is very unlikely to explain either otp-12 finding. P1 is an *initiator-invariance* failure — both arms run identical code and differ only in who dials, so a worker-count or per-file cost cancels between them, and a local copy has no initiator axis at all. P2 is a *new-vs-old* regression, whereas the local cost is *old*: otp-11's own gate measured old-vs-new local `small` at 1684 -> 1750 ms (+3.9% PASS, `docs/bench/otp11-local-2026-07-11/`) and otp-11 D1 explicitly preserved the old pipeline's payload shapes (`PreparedPayload::File`/`TarShard` "exactly as the old local pipeline"). A long-standing cost cannot produce a new regression. **Procedural**: fixing local *first* would touch code shared with the wire sink, perturb P1/P2 mid-investigation, and void the pre-fix baselines pf-final depends on — destroying the attribution rather than adding to it. Sequencing behind keeps every counterfactual legible, and pf-final's full-matrix rerun would still surface any shared-code effect as a number.
   182	- Carried into pf-1 as a cheap check (the one way the two could touch): the local apply pipeline runs **one** worker by default (`transfer_session/local.rs:602`, `sink_workers` is 1 unless the hidden `--workers` flag sets `debug_mode`). If the unified session likewise changed the **remote receive** side's worker count versus old push, that WOULD be new, per-file, and a live P2 candidate. Establish it by reading the executed old path, not by assuming.
   183	- Supersedes: nothing. Adds `LOCAL_SMALL_FILE_PATH.md` to the `docs/STATE.md` queue behind item 1a.
   184	
   185	## D-2026-07-13-3 — Windows attribute/ADS loss is a real gap; fix it AFTER otp-12
   186	- Decision: `blit` silently drops Windows file attributes (ReadOnly/Hidden/System) and alternate data streams on the tar-shard path — **on both the local and the remote route**, exit code 0, no warning — and it will be **fixed after the current phase (otp-12) completes**, not now. Owner, 2026-07-13, verbatim: **"well that, while funny, makes sense. we started this as a linux alternative to robocopy, and full windows support was always a goal... but obviously not landed. so, good, let's address that. after this current phase is complete."** Finding, repro, and root cause: `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`.
   187	- Framing (owner's, and it is the correct one): this is **unlanded Windows support**, NOT a regression. blit began as a Linux alternative to robocopy; full Windows parity was always a goal and the metadata half never shipped. It predates the unified session and is not P1, P2, or otp-11 fallout.
   188	- What makes it more than a missing feature: the loss is **conditional on file count**, so it is silent and non-obvious. `transfer_plan.rs:103-109` sends a transfer down the tar path when there are ≥2 small files AND (≥32 of them OR average ≤128 KiB); otherwise files go through `CopyFileExW`, which carries attributes and ADS for free. So the SAME file keeps its metadata when copied alone and loses it when copied alongside 39 siblings. Proven with identical 200 KiB files where only the count varied (40 → LOST, 3 → PRESERVED), locally and over the wire.
   189	- **Fixing it is a WIRE CONTRACT change.** The tar shard is the wire payload format for small files, so carrying attributes/ADS means extending the shard header or the manifest — a frame change, which trips the stop-and-amend rule: `docs/TRANSFER_SESSION.md` is amended through the codex loop BEFORE any code. Same-build-both-ends (D-2026-07-05-2) means no compatibility surface is created, but the contract doc still governs. The header-vs-manifest choice is a design decision reserved for the owner.
   190	- Sequencing: behind otp-12, and **planned together with `LOCAL_SMALL_FILE_PATH.md`** (D-2026-07-13-2) — they touch the same tar path and pull in opposite directions (a fidelity fix ADDS per-file work to a path already losing 1.9× to robocopy at equal thread count). Planning them separately would optimise one against the other.
   191	- Not in scope / not a bug: **empty directories**. Their absence is a documented design position — `blit check`'s help (`crates/blit-cli/src/cli.rs:20-35`) states the equivalence model skips empty directories and points at `diff -r` for full tree equivalence. blit models files, not directories. (`test_push_empty_directory` only asserts the command succeeds; it never checks the directory arrived — a crash smoke test, not a fidelity test.) **ACLs** are likewise out: robocopy does not copy them either without `/COPY:S`.
   192	- Supersedes: nothing. Adds `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` to the `docs/STATE.md` queue behind otp-12, alongside D-2026-07-13-2.
   193	
   194	## D-2026-07-14-1 — the committed baselines are RE-RECORDED at MTU 9000 (amends OTP12_ACCEPTANCE_RUN D5's pin, not its freeze)
   195	- Decision: the frozen committed baselines that `pf-final` grades against are **re-recorded with their OLD builds at MTU 9000**, so acceptance compares old and new like-for-like on the fabric the fleet actually runs. Owner, 2026-07-14, choosing between three presented options, verbatim: **"Re-record the baseline at 9000"**. The 2026-07-10 baselines are **retained as historical MTU-1500 records** — superseded as the acceptance reference, never deleted or rewritten.
   196	- Why: pf-0 (`docs/bench/otp12-jumbo-win-2026-07-13/`) measured jumbo making **both arms 3–4% faster**. Grading a jumbo NEW arm against a **1500-recorded** ceiling is therefore **LENIENT, not conservative** — the MTU gain flatters the ratio, so a real regression up to roughly the size of that gain could pass unseen. P1 is the one finding between blit and shipping; a lenient ceiling is the wrong error to accept there.
   197	- Scope — **BOTH rigs, not just rig W.** Each harness hardcodes its own committed reference, and both predate the 2026-07-13 fabric-wide jumbo raise (`.agents/machines.md`): rig W `scripts/bench_otp12_win.sh:105` → `docs/bench/otp2w-baseline-2026-07-10/`; rig Z `scripts/bench_otp12_zoey.sh:102` → `docs/bench/otp2-baseline-2026-07-10/`. Rig D (delegated) has **no** old baseline and is unaffected.
   198	- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
   199	  * **Each rig's re-baseline MUST use the same OLD build as its original baseline**, with provenance manifest-verified — rig W `0f922de`; rig Z the build staged in `blit-temp` (which embeds `731023bfc8a1.dirty`, **not** `e757dcc` — see the otp-2 README correction). A re-baseline on a different old build would silently change the reference twice.
   200	  * `BASELINE_SUMMARY` is hardcoded **by design** (no override) so a run cannot quietly re-point its own ceiling. Re-pointing it is therefore a reviewed source edit, not an env var — and the new value must be a **committed** dated dir.
   201	  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
   202	- Supersedes: the *pin* in `OTP12_ACCEPTANCE_RUN.md` D5 ("the frozen baselines stay frozen") — the **freeze principle stands** (a baseline is immutable once recorded, and no run may re-point its own reference), but the acceptance reference is re-recorded once, at the fabric's MTU, and re-frozen. Closes the OPEN item raised in `OTP12_PERF_FINDINGS.md` §pf-0.

exec
/bin/zsh -lc 'git show d71c0ed''^:docs/plan/OTP12_ACCEPTANCE_RUN.md | nl -ba | sed -n '"'90,345p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-bZopd9vq' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-n7lY7mZJ' (errno=Operation not permitted)
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
   221	  client pushes the same staged tree as a local path
   222	  (`blit.exe copy D:\blit-test\bench-module\pull_src_<w>\src_<w> $MAC_HOST:9031:/bench/<fresh>/ --yes`).
   223	
   224	New plumbing this requires, each keyed by ROLE not verb:
   225	
   226	1. **A daemon on the Mac** (new build only): config written like the rig
   227	   scripts do today (`[daemon] bind/port/no_mdns` + `[[module]] name =
   228	   "bench"` pointing at `$MAC_MODULE_ROOT`, **default `$MAC_WORK`
   229	   itself** — the module exports the exact fixture trees arm A pushes,
   230	   so both initiators read the same physical inodes; no fixture copy or
   231	   move on the Mac (codex design F6)), local launch, pid file,
   232	   stale-refusal, PID-scoped teardown. macOS application firewall must
   233	   admit `blit-daemon` — gated by a preflight smoke transfer from
   234	   Windows, not assumed.
   235	2. **A Windows client** (`blit.exe`, new build, built natively alongside
   236	   the daemon). Its timed window is measured ON Windows —
   237	   `[Diagnostics.Stopwatch]` bracketing the `blit.exe copy` inside one ssh
   238	   invocation, output CRLF-stripped (`tr -cd '0-9'`) — the otp-2w
   239	   self-timed pattern (README §Timing-overhead correction); the ssh
   240	   round-trip cost stays outside the window by construction.
   241	3. **Flush keyed by destination OS, never verb**: dest Windows ⇒ self-timed
   242	   `Write-VolumeCache D`; dest macOS ⇒ the local self-timed per-file fsync
   243	   walk. Cold caches both ends before every run (purge / standby-purge);
   244	   drain keyed by the destination disk (Windows `Get-Counter` loop when D:
   245	   receives; the Mac side has no drain equivalent — recorded decision: Mac
   246	   destination runs rely on `sync` + purge exactly as the recorded otp-2w
   247	   pull cells did).
   248	
   249	Arm A cells run fresh inside the invariance block (interleaved A,B,A,B…) —
   250	block-1 new-arm numbers are NOT reused, so rig-state drift between blocks
   251	cannot masquerade as an initiator effect.
   252	
   253	### D4 — delegated cells = delegated-vs-direct parity (rig D)
   254	
   255	Per data direction, the delegated arm and the direct arm drive the SAME
   256	session code with the same roles on the same endpoints; the only deltas are
   257	who spawns the initiator (daemon vs CLI) and the trigger/progress relay:
   258	
   259	- **skippy→Windows**: delegated = Mac runs
   260	  `blit copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ $WIN_HOST:9031:/bench/<fresh>/ --yes`
   261	  (Windows daemon initiates, DESTINATION role); direct = Windows client
   262	  pulls the same source to the same disk
   263	  (`blit.exe copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   264	- **Windows→skippy**: delegated = the mirror-image Mac command (skippy
   265	  daemon initiates); direct = skippy client pulls from the Windows daemon
   266	  (self-timed `/proc/uptime`-bracketed window over ssh, the zoey pattern).
   267	
   268	Timing: the delegated arm is timed on the Mac around the CLI invocation
   269	(the CLI blocks until the relayed Summary), plus the destination's
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
   341	New env knobs: `MAC_HOST` (the Mac's 10 GbE IP — required, no default),
   342	`MAC_MODULE_ROOT` (default `$MAC_WORK` — see D3), `SKIPPY_SSH` (default
   343	`admin@skippy`), `SKIPPY_HOST`, `SKIPPY_BIN` (default
   344	`/mnt/generic-pool/video/blit-bin`), `SKIPPY_DISK_REGEX`,
   345	`OLD_SHA_ZOEY=e757dcc`, `OLD_SHA_WIN=0f922de`.

exec
/bin/zsh -lc "rg -n '"'^## D-2026-07-05-4|''^## D-2026-07-12-1|''^## D-2026-07-14-1'"' docs/DECISIONS.md; nl -ba docs/DECISIONS.md | sed -n '138,210p'; nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '150,215p'; nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '286,340p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
144:## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
169:## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
194:## D-2026-07-14-1 — the committed baselines are RE-RECORDED at MTU 9000 (amends OTP12_ACCEPTANCE_RUN D5's pin, not its freeze)
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
   178	
   179	## D-2026-07-13-2 — the local small-file finding queues BEHIND OTP12_PERF_FINDINGS
   180	- Decision: `docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft) is sequenced **behind** the ACTIVE `docs/plan/OTP12_PERF_FINDINGS.md` — the MTU experiment, then pf-1, then its fix slices. Owner, 2026-07-13, verbatim: **"well, odds that one affects the other? if this is contributory, would we know? probably irrelevant. behind."** No local-path code lands until otp-12's investigation has its attribution. The finding itself (blit vs robocopy, local `D: -> E:`, `docs/bench/win-local-ab-2026-07-13/`) is recorded now; only the *fix* waits.
   181	- Why: two reasons, one causal and one procedural. **Causal**: the local finding is very unlikely to explain either otp-12 finding. P1 is an *initiator-invariance* failure — both arms run identical code and differ only in who dials, so a worker-count or per-file cost cancels between them, and a local copy has no initiator axis at all. P2 is a *new-vs-old* regression, whereas the local cost is *old*: otp-11's own gate measured old-vs-new local `small` at 1684 -> 1750 ms (+3.9% PASS, `docs/bench/otp11-local-2026-07-11/`) and otp-11 D1 explicitly preserved the old pipeline's payload shapes (`PreparedPayload::File`/`TarShard` "exactly as the old local pipeline"). A long-standing cost cannot produce a new regression. **Procedural**: fixing local *first* would touch code shared with the wire sink, perturb P1/P2 mid-investigation, and void the pre-fix baselines pf-final depends on — destroying the attribution rather than adding to it. Sequencing behind keeps every counterfactual legible, and pf-final's full-matrix rerun would still surface any shared-code effect as a number.
   182	- Carried into pf-1 as a cheap check (the one way the two could touch): the local apply pipeline runs **one** worker by default (`transfer_session/local.rs:602`, `sink_workers` is 1 unless the hidden `--workers` flag sets `debug_mode`). If the unified session likewise changed the **remote receive** side's worker count versus old push, that WOULD be new, per-file, and a live P2 candidate. Establish it by reading the executed old path, not by assuming.
   183	- Supersedes: nothing. Adds `LOCAL_SMALL_FILE_PATH.md` to the `docs/STATE.md` queue behind item 1a.
   184	
   185	## D-2026-07-13-3 — Windows attribute/ADS loss is a real gap; fix it AFTER otp-12
   186	- Decision: `blit` silently drops Windows file attributes (ReadOnly/Hidden/System) and alternate data streams on the tar-shard path — **on both the local and the remote route**, exit code 0, no warning — and it will be **fixed after the current phase (otp-12) completes**, not now. Owner, 2026-07-13, verbatim: **"well that, while funny, makes sense. we started this as a linux alternative to robocopy, and full windows support was always a goal... but obviously not landed. so, good, let's address that. after this current phase is complete."** Finding, repro, and root cause: `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`.
   187	- Framing (owner's, and it is the correct one): this is **unlanded Windows support**, NOT a regression. blit began as a Linux alternative to robocopy; full Windows parity was always a goal and the metadata half never shipped. It predates the unified session and is not P1, P2, or otp-11 fallout.
   188	- What makes it more than a missing feature: the loss is **conditional on file count**, so it is silent and non-obvious. `transfer_plan.rs:103-109` sends a transfer down the tar path when there are ≥2 small files AND (≥32 of them OR average ≤128 KiB); otherwise files go through `CopyFileExW`, which carries attributes and ADS for free. So the SAME file keeps its metadata when copied alone and loses it when copied alongside 39 siblings. Proven with identical 200 KiB files where only the count varied (40 → LOST, 3 → PRESERVED), locally and over the wire.
   189	- **Fixing it is a WIRE CONTRACT change.** The tar shard is the wire payload format for small files, so carrying attributes/ADS means extending the shard header or the manifest — a frame change, which trips the stop-and-amend rule: `docs/TRANSFER_SESSION.md` is amended through the codex loop BEFORE any code. Same-build-both-ends (D-2026-07-05-2) means no compatibility surface is created, but the contract doc still governs. The header-vs-manifest choice is a design decision reserved for the owner.
   190	- Sequencing: behind otp-12, and **planned together with `LOCAL_SMALL_FILE_PATH.md`** (D-2026-07-13-2) — they touch the same tar path and pull in opposite directions (a fidelity fix ADDS per-file work to a path already losing 1.9× to robocopy at equal thread count). Planning them separately would optimise one against the other.
   191	- Not in scope / not a bug: **empty directories**. Their absence is a documented design position — `blit check`'s help (`crates/blit-cli/src/cli.rs:20-35`) states the equivalence model skips empty directories and points at `diff -r` for full tree equivalence. blit models files, not directories. (`test_push_empty_directory` only asserts the command succeeds; it never checks the directory arrived — a crash smoke test, not a fidelity test.) **ACLs** are likewise out: robocopy does not copy them either without `/COPY:S`.
   192	- Supersedes: nothing. Adds `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` to the `docs/STATE.md` queue behind otp-12, alongside D-2026-07-13-2.
   193	
   194	## D-2026-07-14-1 — the committed baselines are RE-RECORDED at MTU 9000 (amends OTP12_ACCEPTANCE_RUN D5's pin, not its freeze)
   195	- Decision: the frozen committed baselines that `pf-final` grades against are **re-recorded with their OLD builds at MTU 9000**, so acceptance compares old and new like-for-like on the fabric the fleet actually runs. Owner, 2026-07-14, choosing between three presented options, verbatim: **"Re-record the baseline at 9000"**. The 2026-07-10 baselines are **retained as historical MTU-1500 records** — superseded as the acceptance reference, never deleted or rewritten.
   196	- Why: pf-0 (`docs/bench/otp12-jumbo-win-2026-07-13/`) measured jumbo making **both arms 3–4% faster**. Grading a jumbo NEW arm against a **1500-recorded** ceiling is therefore **LENIENT, not conservative** — the MTU gain flatters the ratio, so a real regression up to roughly the size of that gain could pass unseen. P1 is the one finding between blit and shipping; a lenient ceiling is the wrong error to accept there.
   197	- Scope — **BOTH rigs, not just rig W.** Each harness hardcodes its own committed reference, and both predate the 2026-07-13 fabric-wide jumbo raise (`.agents/machines.md`): rig W `scripts/bench_otp12_win.sh:105` → `docs/bench/otp2w-baseline-2026-07-10/`; rig Z `scripts/bench_otp12_zoey.sh:102` → `docs/bench/otp2-baseline-2026-07-10/`. Rig D (delegated) has **no** old baseline and is unaffected.
   198	- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
   199	  * **Each rig's re-baseline MUST use the same OLD build as its original baseline**, with provenance manifest-verified — rig W `0f922de`; rig Z the build staged in `blit-temp` (which embeds `731023bfc8a1.dirty`, **not** `e757dcc` — see the otp-2 README correction). A re-baseline on a different old build would silently change the reference twice.
   200	  * `BASELINE_SUMMARY` is hardcoded **by design** (no override) so a run cannot quietly re-point its own ceiling. Re-pointing it is therefore a reviewed source edit, not an env var — and the new value must be a **committed** dated dir.
   201	  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
   202	- Supersedes: the *pin* in `OTP12_ACCEPTANCE_RUN.md` D5 ("the frozen baselines stay frozen") — the **freeze principle stands** (a baseline is immutable once recorded, and no run may re-point its own reference), but the acceptance reference is re-recorded once, at the fabric's MTU, and re-frozen. Closes the OPEN item raised in `OTP12_PERF_FINDINGS.md` §pf-0.
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
   286	Config: BOTH daemons get `[delegation] allow_delegated_pull = true` with
   287	`allowed_source_hosts` naming the peer (each is destination in one
   288	direction); bench modules writable, `delegation_allowed` not narrowed.
   289	
   290	### D5 — three self-contained scripts; the frozen baselines stay frozen
   291	
   292	> **AMENDED by D-2026-07-14-1 (2026-07-14) — the *pin* moves once; the *freeze*
   293	> stands.** The committed baselines this section pins were recorded at **MTU
   294	> 1500**, before the fabric-wide jumbo raise. pf-0 measured jumbo making both
   295	> arms 3–4% faster, so grading a jumbo build against a 1500 ceiling is **lenient,
   296	> not conservative**. Each rig's committed baseline is therefore **re-recorded
   297	> once with its ORIGINAL old build at MTU 9000** and re-frozen; the 2026-07-10
   298	> baselines are retained as historical MTU-1500 records. Immutability and the
   299	> no-override rule on `BASELINE_SUMMARY` are unchanged — see D-2026-07-14-1 and
   300	> `OTP12_PERF_FINDINGS.md` §pf-0.
   301	
   302	`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
   303	`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
   304	precedent: duplicate the shape, don't refactor recorded evidence;
   305	`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
   306	old scripts, both recorded sharp edges:
   307	
   308	- **Exit codes are checked**: the old harnesses swallow the blit exit code
   309	  inside the timed window; otp-12 records it per run (`exit` column) and a
   310	  nonzero exit voids the interleave pair per the D2 valid-run rule — a
   311	  failed transfer must never contribute a time.
   312	- **Multi-token flags ride an array**, not an unquoted scalar.
   313	
   314	CSV schema (all rigs):
   315	`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
   316	(`valid` = the PAIR's fate under the D2 valid-run rule — an
   317	individually-clean run whose partner voided reads `no`; amended at the
   318	12a harness slice)
   319	`summary.csv`:
   320	`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
   321	(medians over valid runs only — the D2 valid-run rule)
   322	`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
   323	where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
   324	otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
   325	reference CSVs; corrected at the 12a review, codex F9),
   326	`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
   327	direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
   328	discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
   329	self-adjudicated; added at the 12b harness slice), `arm` ∈
   330	`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
   331	`initiator` = host name, `kind` ∈
   332	`converge|invariance|delegated|cross|cross-gap`.
   333	Verdict outcome vocabulary (closed; 12b review, codex F12): per-reference
   334	rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
   335	carries the registered D2 set
   336	(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
   337	`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
   338	converge row whose same-session block-1 counterpart is absent or
   339	incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
   340	artifact — the committed-reference row still governs). Nothing else is

exec
/bin/zsh -lc "rg -n \"pf-final|P1|P2|old_committed|committed|baseline|acceptance row|full matrix|matrix\" docs/plan/OTP12_PERF_FINDINGS.md docs/plan/OTP12_ACCEPTANCE_RUN.md docs/plan/ONE_TRANSFER_PATH.md | sed -n '1,260p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '640,800p'; nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '1020,1135p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   640	  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
   641	  P2 bars below but regresses any other cell against either reference is
   642	  **not** accepted.
   643	- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
   644	  `pull_tcp_mixed` ≤ 1.10 against BOTH references on the netwatch-01
   645	  rig (CELLS escalation session, RUNS=8), with `wm_grpc_mixed` and the
   646	  other invariance PASSes unregressed against both references. (Meeting
   647	  this bar does not by itself accept the build — see the global rule.)
   648	- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
   649	  (same-session AND committed) on BOTH rigs (CELLS sessions), with the
   650	  gRPC small-push cells unregressed. **"Unregressed" is given a
   651	  reference and a tolerance (review round 3)**: each gRPC small-push
   652	  cell must stay ≤ 1.10 against both of its own references AND must not
   653	  worsen by more than **10% against its own pre-fix median on the same
   654	  rig** (zoey 4731 ms; netwatch-01 2264 ms at 12c-win). The second
   655	  clause exists because those cells currently range 0.801–1.001 — a fix
   656	  that dragged Windows gRPC from 0.85 back to 1.05 would still pass a
   657	  bare ≤1.10 bar while having eaten a real, measured win.
   658	- Cross-direction converge-up is a SEPARATE bar (review round 2):
   659	  every final cross-direction row must still meet the parent plan's
   660	  new-vs-old ceiling (`ONE_TRANSFER_PATH.md` acceptance) or satisfy
   661	  the registered platform-residue discriminator — invariance plus the
   662	  per-direction bars alone would pass if a "fix" slowed BOTH layouts
   663	  equally, violating converge-up.
   664	- No suite regressions; the floor is ≥ the CURRENT count (1484 —
   665	  ≥1483 would permit silently losing a test); any new pins carry
   666	  guard proofs (temporary revert) per the loop.
   667	- If investigation attributes part of a gap to something the plan's
   668	  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
   669	  that residue is RECORDED with its experiment and goes to the owner's
   670	  otp-13 walk — never silently accepted.
   671	
   672	## Staging (each through the codex loop)
   673	
   674	- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
   675	  the two-layout phase-timing report (TCP-carrier mode included) + the
   676	  `0f922de` historical control; probe record committed AND
   677	  codex-reviewed BEFORE any pf-2 branch exists. No fix lands on
   678	  pre-pf-1 evidence.
   679	- **pf-2..n**: one fix slice per confirmed root cause (smallest
   680	  change that moves the phase timing; A/B'd locally before rig time).
   681	- **pf-final**: NOT just the two escalation cells — the final build
   682	  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
   683	  gRPC controls) on **all THREE rigs: Z (zoey), W (netwatch-01) and
   684	  D (delegated, netwatch-01↔skippy)**. **No mixed-build evidence: every
   685	  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
   686	  (corrected, review round 2 — "every row" was impossible: the
   687	  same-session `old` arms and the committed baselines are OLD builds by
   688	  construction, which is the entire point of a reference). Pre-fix
   689	  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
   690	  which are **replication and control evidence, not acceptance
   691	  evidence**.
   692	  **Rig D is included even though it is not a suspect (review round
   693	  3).** Voiding otp-12c's pre-fix rows while re-running only Z and W
   694	  would leave the parent plan's **delegated-parity bar**
   695	  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
   696	  evidence at all. "Not implicated" scopes what pf-1 must
   697	  *instrument* — it does not waive an acceptance bar. Rig D's TCP
   698	  verdict cells (+ the gRPC smoke) therefore rerun on the final build;
   699	  both arms are new-build by construction there (rig D has no old
   700	  baseline), so the whole cell is re-measured.
   701	  **Every gRPC row the acceptance method requires reruns
   702	  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
   703	  earlier "if shared code changed, the gRPC cells rerun too" left the
   704	  decision to the author's own judgement of what counts as shared, which
   705	  is exactly the loophole H7 exploits: a shared regression can hide under
   706	  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
   707	  complete Z/W gRPC converge and invariance rows, so those are
   708	  final-build rows, full stop — no conditional. Results land in fresh
   709	  dated evidence dirs. **Then** otp-12d assembles the matrix from
   710	  final-build rows, and the otp-13 owner walk reads it.
   711	
   712	## Known gaps
   713	
   714	- H1–H5 were graded against the actual tree by codex review
   715	  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
   716	  The old drivers are deleted from HEAD, but the pinned `0f922de`
   717	  source/binaries diff and run fine — historical claims get live
   718	  controls in pf-1, not pin-archaeology.
   719	- zoey never measured P1: its rig anchors converge-up only, so there
   720	  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
   721	  says nothing about layout asymmetry (review 2026-07-12). pf-1's
   722	  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
   723	  qualifies per the 12b wm numbers).
   724	- **The 12c-win rows are replication, not acceptance** (2026-07-13).
   725	  They are pre-fix by definition, so `pf-final` voids them for
   726	  acceptance; their value is that they (a) reproduce P1 and P2 on an
   727	  independent session at the shipped sha, (b) supply the
   728	  opposite-direction control (`mw_tcp_mixed` 1.044 PASS vs
   729	  `wm_tcp_mixed` 1.300 FAIL — same carrier, same fixture) that narrows
   730	  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
   731	  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
   732	  P2 1.149→1.201), so neither is drifting toward the bar on its own.
   733	- **Rig-D delegated parity is not a SUSPECT, but it is still an
   734	  ACCEPTANCE bar** (2026-07-13; scoped correctly at review round 3): the
   735	  delegated-vs-direct matrix passed 7/7
   736	  (`docs/bench/otp12c-delegated-2026-07-13/`), so delegation adds no
   737	  measurable cost and pf-1 need not instrument the delegated trigger
   738	  path. That is a statement about *where to look for the bug* — it does
   739	  **not** waive the parent plan's delegated-parity bar, whose evidence
   740	  is pre-fix and therefore void under pf-final. Rig D reruns on the
   741	  final build (see pf-final).

 succeeded in 0ms:
docs/plan/ONE_TRANSFER_PATH.md:116:      run-to-run noise (±10%). Matrix committed as evidence.
docs/plan/ONE_TRANSFER_PATH.md:124:      corrected symmetric-fs harness records a per-cell baseline of
docs/plan/ONE_TRANSFER_PATH.md:132:      `docs/plan/OTP12_ACCEPTANCE_RUN.md` D2.)
docs/plan/ONE_TRANSFER_PATH.md:146:- [ ] Suite green throughout; final test count ≥ pre-plan baseline
docs/plan/ONE_TRANSFER_PATH.md:244:baseline parity pins per slice. Wire break — lockstep upgrade,
docs/plan/ONE_TRANSFER_PATH.md:272:2. **otp-2 symmetric baseline (harness + rig, no production code)**:
docs/plan/ONE_TRANSFER_PATH.md:273:   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
docs/plan/ONE_TRANSFER_PATH.md:275:   and record the OLD paths' per-cell, per-direction baseline on the
docs/plan/ONE_TRANSFER_PATH.md:305:12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
docs/plan/ONE_TRANSFER_PATH.md:307:    AND every cell ≤ the better old direction + noise; committed as
docs/plan/ONE_TRANSFER_PATH.md:311:    against the unified baseline — owner call at that point.
docs/plan/OTP12_ACCEPTANCE_RUN.md:10:and NO wire surface; it is harness scripts + rig runs + committed evidence).
docs/plan/OTP12_ACCEPTANCE_RUN.md:11:**AMENDED 2026-07-13 — 12d is GATED on `OTP12_PERF_FINDINGS.md`.** 12a/12b/12c
docs/plan/OTP12_ACCEPTANCE_RUN.md:13:P1 (`wm_tcp_mixed` invariance 1.237 → 1.300, a miss of the PARENT plan's
docs/plan/OTP12_ACCEPTANCE_RUN.md:14:headline criterion) and P2 (`push_tcp_small` 1.149 → 1.201).
docs/plan/OTP12_ACCEPTANCE_RUN.md:15:**P1 does NOT reproduce on a same-OS rig**: with Linux on both ends
docs/plan/OTP12_ACCEPTANCE_RUN.md:16:(magneto↔skippy, full methodology) all 8 invariance cells PASS and P1's own
docs/plan/OTP12_ACCEPTANCE_RUN.md:17:cell lands at 1.092 / 1.003 (`docs/bench/otp12-perf-2026-07-13/`). So P1 is
docs/plan/OTP12_ACCEPTANCE_RUN.md:24:inversion counterfactual settles it. `docs/plan/OTP12_PERF_FINDINGS.md`
docs/plan/OTP12_ACCEPTANCE_RUN.md:25:therefore still governs the next step, and **12d must NOT assemble the matrix
docs/plan/OTP12_ACCEPTANCE_RUN.md:26:from the current rows** — they are pre-fix, and that plan's `pf-final` voids
docs/plan/OTP12_ACCEPTANCE_RUN.md:28:**12a/12b/12c (done) → pf-1 → pf-2..n (if a fix is warranted) → pf-final
docs/plan/OTP12_ACCEPTANCE_RUN.md:37:owner — this slice computes and commits the matrix; it declares nothing
docs/plan/OTP12_ACCEPTANCE_RUN.md:42:otp-12 is the plan's acceptance-evidence slice: rerun the otp-2 matrix on the
docs/plan/OTP12_ACCEPTANCE_RUN.md:49:rationale (`docs/bench/otp2-baseline-2026-07-10/README.md` §Methodology
docs/plan/OTP12_ACCEPTANCE_RUN.md:50:findings, `docs/bench/otp2w-baseline-2026-07-10/README.md` §Timing-overhead
docs/plan/OTP12_ACCEPTANCE_RUN.md:55:1. **Invariance matrix** (criterion 1): per data direction × workload
docs/plan/OTP12_ACCEPTANCE_RUN.md:58:2. **Converge-up matrix** (criterion 2 / codex F4): every unified cell ≤ the
docs/plan/OTP12_ACCEPTANCE_RUN.md:60:   recorded old-path baselines, confirmed by interleaved same-session
docs/plan/OTP12_ACCEPTANCE_RUN.md:103:- Baselines on record: `docs/bench/otp2-baseline-2026-07-10/` (zoey,
docs/plan/OTP12_ACCEPTANCE_RUN.md:105:  corollary) and `docs/bench/otp2w-baseline-2026-07-10/` (Mac↔Windows, the
docs/plan/OTP12_ACCEPTANCE_RUN.md:120:| **D** | Windows daemon ↔ skippy daemon (TrueNAS, x86_64), Mac as delegating CLI | delegated-vs-direct parity (trigger invariance) | owner-designated delegated rig; no old baseline exists on this pair |
docs/plan/OTP12_ACCEPTANCE_RUN.md:124:a substitution records fresh baselines and is per-direction only.
docs/plan/OTP12_ACCEPTANCE_RUN.md:140:outside the timed window. Old arms exist only where an old baseline exists
docs/plan/OTP12_ACCEPTANCE_RUN.md:143:delegated baseline.
docs/plan/OTP12_ACCEPTANCE_RUN.md:146:sha, all hosts). Old arms = the pinned baseline shas (`e757dcc` zoey,
docs/plan/OTP12_ACCEPTANCE_RUN.md:156:All statistics per the recorded baselines: integer ms; median of 4, even
docs/plan/OTP12_ACCEPTANCE_RUN.md:170:  interleaved old arm AND the committed 2026-07-10 baseline median for
docs/plan/OTP12_ACCEPTANCE_RUN.md:173:  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
docs/plan/OTP12_ACCEPTANCE_RUN.md:203:interleaved in a fresh session; both sessions are committed.
docs/plan/OTP12_ACCEPTANCE_RUN.md:208:the escalation's entire purpose. The RUNS=4 rows stay committed and
docs/plan/OTP12_ACCEPTANCE_RUN.md:290:### D5 — three self-contained scripts; the frozen baselines stay frozen
docs/plan/OTP12_ACCEPTANCE_RUN.md:293:> stands.** The committed baselines this section pins were recorded at **MTU
docs/plan/OTP12_ACCEPTANCE_RUN.md:296:> not conservative**. Each rig's committed baseline is therefore **re-recorded
docs/plan/OTP12_ACCEPTANCE_RUN.md:298:> baselines are retained as historical MTU-1500 records. Immutability and the
docs/plan/OTP12_ACCEPTANCE_RUN.md:300:> `OTP12_PERF_FINDINGS.md` §pf-0.
docs/plan/OTP12_ACCEPTANCE_RUN.md:305:`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
docs/plan/OTP12_ACCEPTANCE_RUN.md:324:otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
docs/plan/OTP12_ACCEPTANCE_RUN.md:340:artifact — the committed-reference row still governs). Nothing else is
docs/plan/OTP12_ACCEPTANCE_RUN.md:341:legal, and a missing committed-reference row aborts the verdict pass
docs/plan/OTP12_ACCEPTANCE_RUN.md:371:| skippy | none (no old baseline; July binaries unusable) | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (static — sidesteps the recorded glibc 2.36 ceiling) → `$SKIPPY_BIN/bins/<sha>/` (pool paths are exec-friendly; `/tmp` and `/home` are noexec) — `blit` + `blit-daemon` |
docs/plan/OTP12_ACCEPTANCE_RUN.md:381:### D7 — matrix size and session budget
docs/plan/OTP12_ACCEPTANCE_RUN.md:411:  the plan-level verdict matrix assembling every comparison row
docs/plan/OTP12_PERF_FINDINGS.md:8:3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
docs/plan/OTP12_PERF_FINDINGS.md:19:**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
docs/plan/OTP12_PERF_FINDINGS.md:20:assumed by this plan):** P1 has **no escape hatch on the books**.
docs/plan/OTP12_PERF_FINDINGS.md:22:that is *already* invariance-passing; P1 is the invariance failure
docs/plan/OTP12_PERF_FINDINGS.md:23:itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
docs/plan/OTP12_PERF_FINDINGS.md:30:review." P1 is a miss of the parent's HEADLINE acceptance criterion
docs/plan/OTP12_PERF_FINDINGS.md:38:fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
docs/plan/OTP12_PERF_FINDINGS.md:41:work — under `pf-final` they are **pre-fix rows, void for acceptance**,
docs/plan/OTP12_PERF_FINDINGS.md:44:independent corroboration the round-2 review said P1 lacked; and (b) the
docs/plan/OTP12_PERF_FINDINGS.md:46:deferred** until P1/P2 are fixed or explained at code level — assembling
docs/plan/OTP12_PERF_FINDINGS.md:47:an acceptance matrix out of pre-fix rows would build the artifact otp-13
docs/plan/OTP12_PERF_FINDINGS.md:50:## The two findings (evidence, both committed)
docs/plan/OTP12_PERF_FINDINGS.md:52:**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
docs/plan/OTP12_PERF_FINDINGS.md:112:**P1 does NOT reproduce.** Its own cell passes with room to spare:
docs/plan/OTP12_PERF_FINDINGS.md:116:| `sm_tcp_mixed` (P1's cell) | 1745 | 1905 | **1.092** | PASS |
docs/plan/OTP12_PERF_FINDINGS.md:117:| `ms_tcp_mixed` (P1's cell) | 2085 | 2079 | **1.003** | PASS |
docs/plan/OTP12_PERF_FINDINGS.md:125:- **P1 requires the Mac↔Windows pairing.** It is NOT a pure layout
docs/plan/OTP12_PERF_FINDINGS.md:129:- **⚠ BUT P1 HAS NO ESCAPE HATCH TODAY (review round 5, BLOCKER).** An
docs/plan/OTP12_PERF_FINDINGS.md:131:  accept P1 as a platform residue. **It does not.** That decision excuses
docs/plan/OTP12_PERF_FINDINGS.md:134:  ±10%"** (`docs/DECISIONS.md` D-2026-07-12-1). **P1 IS the invariance
docs/plan/OTP12_PERF_FINDINGS.md:138:  1. **FIX IT** — P1 ≤ 1.10 on rig W. This remains the default and the
docs/plan/OTP12_PERF_FINDINGS.md:155:- **P2 is untested by this rig** (it is a converge bar vs the OLD build,
docs/plan/OTP12_PERF_FINDINGS.md:160:> asserted the opposite — "P1 reproduces at 1.78 → the confound breaks
docs/plan/OTP12_PERF_FINDINGS.md:238:new-vs-old check, not a two-layout measurement. P1 was never measured
docs/plan/OTP12_PERF_FINDINGS.md:241:**P2 — unified small-file push pays ~10–20% vs old push, both rigs**,
docs/plan/OTP12_PERF_FINDINGS.md:251:earlier "win 0.98-ish per cells" was wrong against the committed CSVs;
docs/plan/OTP12_PERF_FINDINGS.md:253:same-session / committed:
docs/plan/OTP12_PERF_FINDINGS.md:255:| rig | same-session | committed |
docs/plan/OTP12_PERF_FINDINGS.md:266:whatever P2 is, it is TCP-data-plane-specific, source-initiated, and
docs/plan/OTP12_PERF_FINDINGS.md:281:## pf-0 — the environmental control (MTU): **KILLED as a material cause of P1** (recorded 2026-07-14)
docs/plan/OTP12_PERF_FINDINGS.md:287:were unchanged by rev 4** (rev 4 re-described the *rig* after the `q` baseline —
docs/plan/OTP12_PERF_FINDINGS.md:299:point estimate of the MTU contribution to P1 is ~0. The null is **not vacuous**
docs/plan/OTP12_PERF_FINDINGS.md:303:**asymmetry**. P1 FAILED in all four sessions (1.237–1.362) regardless of MTU;
docs/plan/OTP12_PERF_FINDINGS.md:307:- **The wire is not exonerated, and "P1 is code-shaped" is NOT established
docs/plan/OTP12_PERF_FINDINGS.md:313:  CONFIRMED-CONTRIBUTING threshold is 20% of Δ_P1 ≈ **46 ms**, which is
docs/plan/OTP12_PERF_FINDINGS.md:322:**`Δ_P1(rig W)` is re-estimated, and the noise floor constrains how pf-1 may
docs/plan/OTP12_PERF_FINDINGS.md:324:the `q` pairing give **Δ_P1 ≈ 230 ms** (229 at 1500, 236 at 9000).
docs/plan/OTP12_PERF_FINDINGS.md:346:**RESOLVED — the committed baselines are RE-RECORDED at MTU 9000
docs/plan/OTP12_PERF_FINDINGS.md:348:runs MTU 9000 while the committed anti-drift ceilings were recorded at **MTU
docs/plan/OTP12_PERF_FINDINGS.md:353:The owner's resolution is to **re-record each rig's committed baseline with its
docs/plan/OTP12_PERF_FINDINGS.md:355:unchanged (a baseline is immutable once recorded; no run may re-point its own
docs/plan/OTP12_PERF_FINDINGS.md:356:ceiling) — only the *pin* moves, once. The 2026-07-10 baselines are retained as
docs/plan/OTP12_PERF_FINDINGS.md:359:**This is a prerequisite slice for `pf-final`, and it affects BOTH rigs** (each
docs/plan/OTP12_PERF_FINDINGS.md:361:raise): rig W `bench_otp12_win.sh:105` → `otp2w-baseline-2026-07-10/`; rig Z
docs/plan/OTP12_PERF_FINDINGS.md:362:`bench_otp12_zoey.sh:102` → `otp2-baseline-2026-07-10/`. Rig D has no old
docs/plan/OTP12_PERF_FINDINGS.md:363:baseline and is unaffected. Constraints (same old build per rig,
docs/plan/OTP12_PERF_FINDINGS.md:365:reviewed source edit; the pf-0 start-AND-end MSS gate applies, since a baseline
docs/plan/OTP12_PERF_FINDINGS.md:374:- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
docs/plan/OTP12_PERF_FINDINGS.md:394:- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
docs/plan/OTP12_PERF_FINDINGS.md:402:- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
docs/plan/OTP12_PERF_FINDINGS.md:420:- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
docs/plan/OTP12_PERF_FINDINGS.md:427:- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
docs/plan/OTP12_PERF_FINDINGS.md:440:- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
docs/plan/OTP12_PERF_FINDINGS.md:445:  TCP-only and per-member (so small-file-heavy) — matches the P2
docs/plan/OTP12_PERF_FINDINGS.md:462:  account for a material share of the P2 gap. If H6 is confirmed, the P2
docs/plan/OTP12_PERF_FINDINGS.md:472:  does NOT trip the Contract rule. Grade its recovery against `Δ_P2` on
docs/plan/OTP12_PERF_FINDINGS.md:476:- **H7 (P2; added by review round 4 — the SHARED-controller candidate
docs/plan/OTP12_PERF_FINDINGS.md:484:  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
docs/plan/OTP12_PERF_FINDINGS.md:512:3. **Historical control, then bisect P2**: old push is deleted from
docs/plan/OTP12_PERF_FINDINGS.md:556:   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
docs/plan/OTP12_PERF_FINDINGS.md:559:   P2 to H6;
docs/plan/OTP12_PERF_FINDINGS.md:563:4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
docs/plan/OTP12_PERF_FINDINGS.md:564:   local rig is Mac↔Mac loopback: it removes the very platform terms P1
docs/plan/OTP12_PERF_FINDINGS.md:565:   is confounded with, and it may equally fail to surface P2 (whose
docs/plan/OTP12_PERF_FINDINGS.md:568:   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
docs/plan/OTP12_PERF_FINDINGS.md:569:   since P2 was measured on both) with the same spans and the CELLS
docs/plan/OTP12_PERF_FINDINGS.md:572:5. Every experiment lands as a committed probe record under
docs/plan/OTP12_PERF_FINDINGS.md:573:   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
docs/plan/OTP12_PERF_FINDINGS.md:590:  (review round 5: the earlier text left it ambiguous between P1's
docs/plan/OTP12_PERF_FINDINGS.md:591:  layout gap and P2's old/new gap, which are different quantities):
docs/plan/OTP12_PERF_FINDINGS.md:592:  - **`Δ_P1(rig)`** = `destinit_median − srcinit_median` for
docs/plan/OTP12_PERF_FINDINGS.md:598:    grading. Read §pf-0 before grading any recovery against `Δ_P1`. On
docs/plan/OTP12_PERF_FINDINGS.md:600:    **P1 counterfactuals are graded on rig W only**; a Linux-rig recovery is
docs/plan/OTP12_PERF_FINDINGS.md:602:  - **`Δ_P2(rig)`** = `new_median − old_same_session_median` for
docs/plan/OTP12_PERF_FINDINGS.md:636:  Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in
docs/plan/OTP12_PERF_FINDINGS.md:638:  same-session reference AND the committed baseline — no arm may exceed
docs/plan/OTP12_PERF_FINDINGS.md:640:  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
docs/plan/OTP12_PERF_FINDINGS.md:641:  P2 bars below but regresses any other cell against either reference is
docs/plan/OTP12_PERF_FINDINGS.md:643:- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
docs/plan/OTP12_PERF_FINDINGS.md:648:- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
docs/plan/OTP12_PERF_FINDINGS.md:649:  (same-session AND committed) on BOTH rigs (CELLS sessions), with the
docs/plan/OTP12_PERF_FINDINGS.md:676:  `0f922de` historical control; probe record committed AND
docs/plan/OTP12_PERF_FINDINGS.md:681:- **pf-final**: NOT just the two escalation cells — the final build
docs/plan/OTP12_PERF_FINDINGS.md:687:  same-session `old` arms and the committed baselines are OLD builds by
docs/plan/OTP12_PERF_FINDINGS.md:695:  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
docs/plan/OTP12_PERF_FINDINGS.md:700:  baseline), so the whole cell is re-measured.
docs/plan/OTP12_PERF_FINDINGS.md:706:  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
docs/plan/OTP12_PERF_FINDINGS.md:709:  dated evidence dirs. **Then** otp-12d assembles the matrix from
docs/plan/OTP12_PERF_FINDINGS.md:719:- zoey never measured P1: its rig anchors converge-up only, so there
docs/plan/OTP12_PERF_FINDINGS.md:722:  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
docs/plan/OTP12_PERF_FINDINGS.md:725:  They are pre-fix by definition, so `pf-final` voids them for
docs/plan/OTP12_PERF_FINDINGS.md:726:  acceptance; their value is that they (a) reproduce P1 and P2 on an
docs/plan/OTP12_PERF_FINDINGS.md:730:  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
docs/plan/OTP12_PERF_FINDINGS.md:731:  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
docs/plan/OTP12_PERF_FINDINGS.md:732:  P2 1.149→1.201), so neither is drifting toward the bar on its own.
docs/plan/OTP12_PERF_FINDINGS.md:735:  delegated-vs-direct matrix passed 7/7
docs/plan/OTP12_PERF_FINDINGS.md:740:  is pre-fix and therefore void under pf-final. Rig D reruns on the
docs/plan/OTP12_PERF_FINDINGS.md:741:  final build (see pf-final).

exec
/bin/zsh -lc 'for d in docs/bench/otp12a-zoey-2026-07-12 docs/bench/otp12b-win-2026-07-12 docs/bench/otp12c-delegated-2026-07-13; do if test -d "$d"; then echo DIR:$d; find "$d" -maxdepth 2 -type f -name '"'verdicts.csv' -o -name 'README.md'; fi; done; find docs/bench -maxdepth 2 -type f -name verdicts.csv | sort | tail -30" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
DIR:docs/bench/otp12c-delegated-2026-07-13
docs/bench/otp12c-delegated-2026-07-13/README.md
docs/bench/otp12c-delegated-2026-07-13/rerun-8pair/verdicts.csv
docs/bench/otp12c-delegated-2026-07-13/verdicts.csv
docs/bench/otp12-perf-2026-07-13/verdicts.csv
docs/bench/otp12-q-baseline-2026-07-13/verdicts.csv
docs/bench/otp12-win-2026-07-12/verdicts.csv
docs/bench/otp12-zoey-2026-07-12/verdicts.csv
docs/bench/otp12c-delegated-2026-07-13/verdicts.csv
docs/bench/otp12c-win-2026-07-13/verdicts.csv

exec
/bin/zsh -lc 'git show --check d71c0ed; git diff-tree --check d71c0ed''^ d71c0ed' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-V6IRnBrY' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GUNXzDzc' (errno=Operation not permitted)
commit d71c0edaa395fdd25756dbe6ba9a5cb272475666
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Tue Jul 14 02:15:23 2026 -0400

    decision D-2026-07-14-1: re-record the committed baselines at MTU 9000
    
    Owner, 2026-07-14, choosing between three presented options, verbatim:
    "Re-record the baseline at 9000".
    
    Why: pf-0 measured jumbo making BOTH arms 3-4% faster. The committed anti-drift
    ceilings were recorded at MTU 1500, before the fabric-wide jumbo raise. So
    grading a jumbo NEW arm against a 1500-recorded ceiling is LENIENT, not
    conservative -- the MTU gain flatters the ratio and a regression up to roughly
    the size of that gain could pass unseen. P1 is the one finding between blit and
    shipping; a lenient ceiling is the wrong error to accept there.
    
    Scope is BOTH rigs, not just rig W: each harness hardcodes its own reference and
    both predate the jumbo raise (rig W bench_otp12_win.sh:105 -> otp2w-baseline;
    rig Z bench_otp12_zoey.sh:102 -> otp2-baseline). Rig D has no old baseline.
    
    The FREEZE principle stands -- a baseline is immutable once recorded, and no run
    may re-point its own ceiling. Only the PIN moves, once: each rig re-records with
    its ORIGINAL old build at MTU 9000 and re-freezes. The 2026-07-10 baselines are
    retained as historical MTU-1500 records, never rewritten. BASELINE_SUMMARY stays
    override-free (a run must not be able to quietly re-point its own reference), so
    re-pointing is a reviewed source edit; and pf-0's start-AND-end MSS gate applies
    to the re-baseline sessions, since a baseline recorded at an unverified MTU is
    precisely the defect being fixed.
    
    Propagated: OTP12_PERF_FINDINGS §pf-0 OPEN -> RESOLVED; OTP12_ACCEPTANCE_RUN D5
    carries the amendment note where it asserts the freeze; STATE's next action is
    now pf-1 (the re-baseline is a pf-final prerequisite, not a pf-1 blocker).

git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-gk97UeUz' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZXjBVlkT' (errno=Operation not permitted)

exec
/bin/zsh -lc "nl -ba .agents/machines.md | sed -n '100,145p'; rg -n \"rig.W.*q|Rig.W.*q|runs on "'`q`|Mac end.*q|q.*netwatch|pf-final.*q|re-baseline|rebaseline" docs/STATE.md docs/plan/OTP12_PERF_FINDINGS.md docs/plan/OTP12_ACCEPTANCE_RUN.md .agents/machines.md docs/DECISIONS.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   100	## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)
   101	
   102	`ssh michael@q` — Apple **M4 Mac mini**, 16 GB, macOS 26.5.2, arm64. It is now
   103	the rig-W Mac end: **quiet, dedicated, and faster than nagatha** (1 GiB in
   104	~908 ms ≈ 1.18 GB/s, vs nagatha's ~1.3–1.8 s). Using it **decouples the codex
   105	review loop from rig-W benchmarking** — the contention that destroyed a
   106	53-minute experiment (below).
   107	
   108	- **10GbE**: `en8` = **10.1.10.54**, MTU **9000**, media 10Gbase-T. This is the
   109	  **Aquantia adapter physically moved off nagatha**, so nagatha's 10GbE is now a
   110	  *different* NIC at **10.1.10.92** (also MTU 9000). Any doc naming
   111	  "Aquantia @ .54 on nagatha" is stale.
   112	- **⚠ THE MULTI-NIC ROUTING TRAP (cost ~1h).** `q` has THREE IPs on
   113	  10.1.10.0/24 — `en0` (1GbE, .221), `en1` (Wi-Fi, .108), `en8` (10GbE, .54) —
   114	  and macOS routes the subnet via the highest-ranked **network service**, not by
   115	  which IP "matches". `en0` outranked `en8`, so **every benchmark would have run
   116	  over gigabit**. Fixed by promoting the service that owns `en8` — confusingly
   117	  named **"Thunderbolt Ethernet Slot 3"** — to rank 1
   118	  (`sudo networksetup -ordernetworkservices …`). It has the same router
   119	  (10.1.10.1), so `q` keeps its internet.
   120	- **DO NOT "fix" this with a host route.**
   121	  `sudo route -n add -host 10.1.10.177 -interface en8` on a *directly-connected*
   122	  subnet installs a next hop of **the interface's own MAC** — a black hole. It
   123	  drops 100% of packets while `route -n get` still cheerfully reports
   124	  `interface: en8`. Verify with `arp -n <peer>`: the MAC must be the PEER's, not
   125	  `q`'s (`00:01:d2:19:04:a3`).
   126	- **An ssh transfer CANNOT verify this link.** ssh caps at ~79 MB/s on this path
   127	  (nagatha's known-good 10GbE scores the same 79), which is *below* the gigabit
   128	  ceiling — so a degraded link and a healthy one look identical through it. Use
   129	  `ifconfig en8 | grep media` (the PHY's negotiated rate) and blit's own
   130	  `wm_tcp_large` time (~908 ms for 1 GiB = 10GbE; ~10 s = 1GbE).
   131	- **Staged**: repo clone at `~/Dev/blit_v2_f35702a` (detached `f35702a`, cloned
   132	  from the LOCAL gitea — `q` *is* the gitea host); `target/release/{blit,blit-daemon}`
   133	  arm64 copied from nagatha (embed-verified `+f35702a`); old client at
   134	  `~/blit-bench-work/bins/blit-0f922de`; fixtures in `~/blit-bench-work`.
   135	  NOPASSWD `/usr/sbin/purge` granted (`/etc/sudoers.d/blit-bench`, mode 0440 —
   136	  `visudo -c` rejects any other mode). ssh key authorized on netwatch-01 in
   137	  **`C:\ProgramData\ssh\administrators_authorized_keys`** (michael is an admin
   138	  there, so the per-user file is ignored). macOS firewall is OFF on `q`.
   139	- **`q` RUNS GITEA** (it is `origin`, `http://q:3000`). It idles cheaply, but
   140	  **do not push to `origin` during a benchmark session**.
   141	
   142	## THE MAC IS A BENCH END — keep it quiet (recorded 2026-07-13, learned the hard way)
   143	
   144	**A rig-W (Mac↔Windows) benchmark requires a QUIET Mac.** The Mac is not a
   145	neutral driver: it runs the client in `mac_init` arms and serves the daemon in
zsh:1: command not found: q
docs/DECISIONS.md:181:- Why: two reasons, one causal and one procedural. **Causal**: the local finding is very unlikely to explain either otp-12 finding. P1 is an *initiator-invariance* failure — both arms run identical code and differ only in who dials, so a worker-count or per-file cost cancels between them, and a local copy has no initiator axis at all. P2 is a *new-vs-old* regression, whereas the local cost is *old*: otp-11's own gate measured old-vs-new local `small` at 1684 -> 1750 ms (+3.9% PASS, `docs/bench/otp11-local-2026-07-11/`) and otp-11 D1 explicitly preserved the old pipeline's payload shapes (`PreparedPayload::File`/`TarShard` "exactly as the old local pipeline"). A long-standing cost cannot produce a new regression. **Procedural**: fixing local *first* would touch code shared with the wire sink, perturb P1/P2 mid-investigation, and void the pre-fix baselines pf-final depends on — destroying the attribution rather than adding to it. Sequencing behind keeps every counterfactual legible, and pf-final's full-matrix rerun would still surface any shared-code effect as a number.
docs/DECISIONS.md:198:- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
docs/DECISIONS.md:199:  * **Each rig's re-baseline MUST use the same OLD build as its original baseline**, with provenance manifest-verified — rig W `0f922de`; rig Z the build staged in `blit-temp` (which embeds `731023bfc8a1.dirty`, **not** `e757dcc` — see the otp-2 README correction). A re-baseline on a different old build would silently change the reference twice.
docs/DECISIONS.md:201:  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
.agents/machines.md:103:the rig-W Mac end: **quiet, dedicated, and faster than nagatha** (1 GiB in
.agents/machines.md:144:**A rig-W (Mac↔Windows) benchmark requires a QUIET Mac.** The Mac is not a
docs/plan/OTP12_ACCEPTANCE_RUN.md:399:  daemon staged beside it; **fresh owner go for daemon runs on zoey**
docs/plan/OTP12_PERF_FINDINGS.md:38:fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
docs/plan/OTP12_PERF_FINDINGS.md:103:separated by any number of additional runs on this rig.
docs/plan/OTP12_PERF_FINDINGS.md:290:(9000, 1500, 1500, 9000) on rig W with the `q` Mac end, `RUNS=8`, **256 timed
docs/plan/OTP12_PERF_FINDINGS.md:740:  is pre-fix and therefore void under pf-final. Rig D reruns on the
docs/STATE.md:9:- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
docs/STATE.md:10:- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
docs/STATE.md:122:   no daemon runs on zoey without a fresh go.

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '84,155p'; nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '352,410p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
   352	`MAC_MODULE_ROOT` (default `$MAC_WORK` — see D3), `SKIPPY_SSH` (default
   353	`admin@skippy`), `SKIPPY_HOST`, `SKIPPY_BIN` (default
   354	`/mnt/generic-pool/video/blit-bin`), `SKIPPY_DISK_REGEX`,
   355	`OLD_SHA_ZOEY=e757dcc`, `OLD_SHA_WIN=0f922de`.
   356	
   357	Verification entry point for harness commits (no crates/proto touched; the
   358	cargo gates don't exercise bash): `bash -n` on each script + shellcheck
   359	where installed + `bash scripts/agent/check-docs.sh` + the codex review;
   360	the methodology itself is verified by the probe/recorded-run discipline
   361	(otp-2 precedent) and each script supports `PREFLIGHT_ONLY=1` (run every
   362	preflight check and exit before fixtures).
   363	
   364	### D6 — staging per host
   365	
   366	| host | old arm | new arm |
   367	|------|---------|---------|
   368	| Mac | rebuild client at the pinned sha in a detached worktree → `~/blit-bench-work/bins/blit-<sha>` | `cargo build --release` at the run commit |
   369	| zoey | clean `e757dcc` zigbuild staged as `blit-daemon-e757dcc` — the 2026-07-10 staging at `blit-daemon` FAILED provenance (a dirty `731023b` build; correction note in the otp-2 README) and is left untouched as the otp-2 artifact | `cargo zigbuild --release --target aarch64-unknown-linux-musl` → staged beside as `blit-daemon-<sha>` (never overwrite); everything stays inside `blit-temp/` |
   370	| Windows | copy the detached-checkout exes ASIDE first (`D:\blit-test\bins\0f922de\`) before any checkout movement | fresh git bundle (pushes are owner-gated; origin lags at `6d37a22`) → checkout run commit → native `cargo build --release` (daemon AND `blit.exe` client) → `D:\blit-test\bins\<sha>\` |
   371	| skippy | none (no old baseline; July binaries unusable) | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (static — sidesteps the recorded glibc 2.36 ceiling) → `$SKIPPY_BIN/bins/<sha>/` (pool paths are exec-friendly; `/tmp` and `/home` are noexec) — `blit` + `blit-daemon` |
   372	
   373	Windows daemon-swap mechanics: the active arm's exe is COPIED to the fixed
   374	path `D:\blit-test\bins\active\blit-daemon.exe` and launched from there —
   375	one program-scoped firewall rule total (the rule is exe-path-scoped;
   376	sha-named dirs keep provenance, the copy log records each swap). Launch
   377	stays WMI `Win32_Process.Create` + stale-refusal + PID-scoped teardown
   378	(otp-2w README §Host plumbing). A staging manifest (sha256 of every binary
   379	on every host) is recorded in each evidence README.
   380	
   381	### D7 — matrix size and session budget
   382	
   383	| rig | comparisons | timed runs | est. wall |
   384	|-----|------------:|-----------:|----------:|
   385	| Z converge-up | 12 (3 fixtures × 2 dirs × 2 carriers) | 96 | 1.5–2.5 h (drains dominate) |
   386	| W converge-up | 12 | 96 | ~1.5 h |
   387	| W invariance | 12 (3 × 2 dirs × 2 carriers, new-only) | 96 | ~1.5 h |
   388	| D delegated | 6 (3 × 2 dirs, TCP) + 1 grpc smoke | 56 | ~1 h |
   389	
   390	Each rig session needs the owner's machines on and otherwise idle; sessions
   391	are independent and may run on different days (each records its own rig
   392	state).
   393	
   394	## Staging (sub-slices; each commit through the codex loop)
   395	
   396	- **otp-12a — rig Z**: `bench_otp12_zoey.sh` (harness commit; codex; fix) →
   397	  recorded run → `docs/bench/otp12-zoey-<date>/README.md` + CSVs (evidence
   398	  commit; codex; fix). Preflight gates: staged old pair present; new musl
   399	  daemon staged beside it; **fresh owner go for daemon runs on zoey**
   400	  (standing STATE rule) and zoey out of maintenance.
   401	- **otp-12b — rig W**: `bench_otp12_win.sh` covering converge-up block +
   402	  invariance block; same two-commit shape. Preflight gates: bundle
   403	  delivered + old exes copied aside + new native build (daemon + client);
   404	  Mac daemon smoke from Windows (firewall).
   405	- **otp-12c — rig D**: `bench_otp12_delegated.sh`; same shape. Preflight
   406	  gates: fresh skippy staging on the pool; `sudo -n` drop_caches on skippy;
   407	  delegation config both daemons; reachability smokes in both directions
   408	  (control port + a 1-file TCP-carrier transfer — the data plane binds
   409	  ephemeral ports, so the smoke IS the firewall test).
   410	- **otp-12d — assembly**: `docs/bench/otp12-acceptance-<date>/README.md` —

exec
/bin/zsh -lc 'for d in S1_9000 S2_1500 S3_1500 S4_9000; do echo $d; awk -F, '"'"'$1 ~ /''^(wm_tcp_large|wm_tcp_mixed|mw_tcp_mixed|wm_grpc_mixed)$/ {print}'"' docs/bench/otp12-jumbo-win-2026-07-13/"'$d/summary.csv; done' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
S1_9000
mw_tcp_mixed,mac_init,1710,1705,1592,10.7,0,8
mw_tcp_mixed,win_init,1641,1681,1585,16.5,0,8
wm_grpc_mixed,mac_init,1121,1145,1112,8.8,0,8
wm_grpc_mixed,win_init,1128,1130,1099,6.4,0,8
wm_tcp_large,mac_init,908,914,907,2.9,0,8
wm_tcp_large,win_init,908,909,904,1.9,0,8
wm_tcp_mixed,mac_init,1035,1044,1020,7.1,0,8
wm_tcp_mixed,win_init,760,768,699,20.7,0,8
S2_1500
mw_tcp_mixed,mac_init,1607,1645,1567,12.1,0,8
mw_tcp_mixed,win_init,1641,1632,1545,13.5,0,8
wm_grpc_mixed,mac_init,1184,1192,1133,17.6,0,8
wm_grpc_mixed,win_init,1159,1163,1128,6.2,0,8
wm_tcp_large,mac_init,960,963,954,2.9,0,8
wm_tcp_large,win_init,946,960,943,10.9,0,8
wm_tcp_mixed,mac_init,1071,1082,1056,6.6,0,8
wm_tcp_mixed,win_init,830,812,716,22.2,0,8
S3_1500
mw_tcp_mixed,mac_init,1735,1698,1565,15.1,0,8
mw_tcp_mixed,win_init,1619,1618,1530,10.5,0,8
wm_grpc_mixed,mac_init,1178,1172,1126,6.9,0,8
wm_grpc_mixed,win_init,1159,1160,1125,6.3,0,8
wm_tcp_large,mac_init,960,962,954,4.3,0,8
wm_tcp_large,win_init,944,944,940,1.5,0,8
wm_tcp_mixed,mac_init,1066,1078,1059,5.0,0,8
wm_tcp_mixed,win_init,849,862,729,44.6,0,8
S4_9000
mw_tcp_mixed,mac_init,1667,1727,1615,24.1,0,8
mw_tcp_mixed,win_init,1632,1650,1568,19.1,0,8
wm_grpc_mixed,mac_init,1145,1142,1108,6.0,0,8
wm_grpc_mixed,win_init,1123,1125,1102,5.4,0,8
wm_tcp_large,mac_init,940,933,917,3.1,0,8
wm_tcp_large,win_init,924,926,913,4.4,0,8
wm_tcp_mixed,mac_init,1029,1041,1024,6.7,0,8
wm_tcp_mixed,win_init,832,816,752,14.4,0,8

exec
/bin/zsh -lc "nl -ba scripts/bench_otp2_baseline.sh | sed -n '1,130p'; rg -n \"BUILD|SHA|blit-daemon|OLD|version|manifest|MTU|MSS\" scripts/bench_otp2_baseline.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env bash
     2	# otp-2: symmetric-fs disk-to-disk baseline of the OLD transfer paths
     3	# (ONE_TRANSFER_PATH plan, slice 2). This is the converge-up reference
     4	# the otp-12 acceptance run compares against, per cell, per direction.
     5	#
     6	# Methodology (corrects the sf-1 harness — see
     7	# docs/bench/10gbe-2026-07-05/DIAGNOSIS.md for what it replaces):
     8	#   * VERDICT CELLS are symmetric-fs disk-to-disk: the client end's
     9	#     data lives on the client machine's real disk (never /tmp — on
    10	#     Linux that is tmpfs), the daemon end's module root lives on the
    11	#     daemon machine's real pool. Both directions of a cell use the
    12	#     SAME two storage ends, so push vs pull is a fair comparison.
    13	#   * COLD CACHES before every timed run, both ends: `purge` on the
    14	#     macOS client (needs a NOPASSWD sudoers rule), drop_caches on the
    15	#     Linux daemon host.
    16	#   * DURABLE-AT-DESTINATION timing: the timed window is the transfer
    17	#     PLUS a destination flush — remote `sync` for pushes (Linux sync
    18	#     waits for writeback), a per-file fsync walk for pulls (macOS
    19	#     sync(2) only SCHEDULES writes, so a bare local sync would
    20	#     under-time pulls relative to pushes). Without durable windows a
    21	#     run's number is a write-cache lottery — probe 1 showed up to 8x
    22	#     spread on push cells purely from how much of the payload the
    23	#     pool absorbed into cache before writeback throttled.
    24	#   * POOL DRAIN before every timed run, AFTER flushing dirty pages
    25	#     (sync first, then wait quiet): the daemon host's write path has
    26	#     state (an NVMe tier destaging to the spinning RAID); pushes
    27	#     timed against a partially-full tier ascend 2.7s -> 13.4s for
    28	#     identical work (probe run 2). Quiet = three consecutive 2s
    29	#     windows under 2 MiB written; a drain TIMEOUT is recorded against
    30	#     the run's label, never silent.
    31	#   * MEDIAN is the cell statistic (robust to the residual one-in-four
    32	#     outlier drained pushes still show); avg and best recorded too.
    33	#     All times integer ms; an even-count median is the floor of the
    34	#     mean of the middle two.
    35	#   * FRESH destination every run (blit no-ops onto delivered
    36	#     content), unique per invocation (an interrupted run cannot
    37	#     leave content a rerun would no-op onto).
    38	#   * Prerequisite: python3 on the client (monotonic timing + the
    39	#     fsync walk).
    40	#   * No competitor rows (D-2026-07-04-4: ceiling-driven, never
    41	#     competitor-relative). The July tmpfs/warm rows remain in
    42	#     docs/bench/10gbe-2026-07-05/ as explicitly-labeled
    43	#     wire-reference data only.
    44	#
    45	# Cells: {large, small, mixed} x {push, pull} x {tcp, grpc} = 12.
    46	# Fixture shapes match sf-1 for continuity: large = 1 GiB single file,
    47	# small = 10,000 x 4 KiB, mixed = 512 MiB + 5,000 x 2 KiB.
    48	#
    49	# Usage (from the client Mac):
    50	#   export ZOEY_SSH=root@zoey
    51	#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
    52	#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
    53	#   ./scripts/bench_otp2_baseline.sh
    54	#
    55	# The daemon binary must already be staged at $ZOEY_TEMP/blit-daemon
    56	# (static aarch64-musl build of the SAME commit as the local client).
    57	# Everything on the daemon host stays inside $ZOEY_TEMP (owner rule).
    58	
    59	set -euo pipefail
    60	
    61	SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    62	REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
    63	BLIT="$REPO_ROOT/target/release/blit"
    64	
    65	ZOEY_SSH=${ZOEY_SSH:-root@zoey}
    66	ZOEY_TEMP=${ZOEY_TEMP:?set ZOEY_TEMP to the blit-temp folder on the daemon host}
    67	ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
    68	PORT=${PORT:-9031}
    69	RUNS=${RUNS:-3}
    70	# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
    71	MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
    72	
    73	OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp2_baseline_$(date +%Y%m%dT%H%M%S)}
    74	mkdir -p "$OUT_DIR" "$MAC_WORK"
    75	
    76	MODULE_ROOT="$ZOEY_TEMP/bench-module"
    77	REMOTE="$ZOEY_HOST:$PORT:/bench/"
    78	
    79	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
    80	# ControlMaster multiplexing: an ssh connection to this host costs
    81	# ~1.2s (slow-core key exchange) — reuse one connection.
    82	SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
    83	zssh() { ssh "${SSH_MUX[@]}" "$ZOEY_SSH" "$@"; }
    84	# Wall-clock ms. Deliberately NOT time.monotonic(): its reference
    85	# point is per-process-undefined, and start/end here are two separate
    86	# python3 processes — a monotonic attempt produced 0/negative windows
    87	# while the daemon log showed multi-second transfers. Wall clock is
    88	# correct across processes; the windows are seconds long and the
    89	# median absorbs the (rare) NTP-step outlier. python3 is a documented
    90	# prerequisite (preflight-checked).
    91	now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
    92	# --- Self-timed durability steps (codex otp-2w F3, applied here too) --
    93	# The timed window = transfer + destination flush, and NOTHING else.
    94	# An `ssh host sync` wrapped in the window adds ~1.2s of connection
    95	# setup (measured) that lands only on push cells; each durability step
    96	# therefore times ITSELF on the destination machine and reports its
    97	# own duration, which the harness adds to the locally-timed transfer
    98	# segment. /proc/uptime is the remote monotonic ms source (busybox-
    99	# safe; both reads happen in one shell, so the reference is shared).
   100	sync_dest_ms() {   # Linux sync on the daemon host; prints its elapsed ms
   101	    zssh 'a=$(awk "{print int(\$1*1000)}" /proc/uptime); sync; b=$(awk "{print int(\$1*1000)}" /proc/uptime); echo $((b-a))'
   102	}
   103	# Durable pull window (codex otp-2 F2): macOS sync(2) SCHEDULES writes
   104	# and may return early, unlike Linux sync(2) which waits — so a bare
   105	# `sync` under-times the pull cells relative to the push cells' remote
   106	# sync. fsync every file in the dest tree instead: on macOS fsync
   107	# flushes to the drive, the closest equivalent of Linux sync's
   108	# wait-for-writeback depth (F_FULLFSYNC-to-media is deliberately NOT
   109	# used — the Linux side does not pay media-flush either).
   110	fsync_tree_ms() {
   111	    python3 - "$1" <<'PYEOF'
   112	import os, sys, time
   113	t = time.monotonic()
   114	for root, dirs, files in os.walk(sys.argv[1]):
   115	    for name in files:
   116	        fd = os.open(os.path.join(root, name), os.O_RDONLY)
   117	        os.fsync(fd)
   118	        os.close(fd)
   119	print(int((time.monotonic() - t) * 1000))
   120	PYEOF
   121	}
   122	
   123	# --- Preflight -------------------------------------------------------
   124	[[ -x "$BLIT" ]] || { echo "missing $BLIT (cargo build --release first)"; exit 1; }
   125	command -v python3 >/dev/null || { echo "python3 required (timing + fsync_tree)"; exit 1; }
   126	sudo -n /usr/sbin/purge || {
   127	    echo "cold-cache purge needs a NOPASSWD sudoers rule for /usr/sbin/purge"; exit 1; }
   128	zssh "test -x '$ZOEY_TEMP/blit-daemon'" || {
   129	    echo "daemon binary not staged at $ZOEY_TEMP/blit-daemon"; exit 1; }
   130	BUILD_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
2:# otp-2: symmetric-fs disk-to-disk baseline of the OLD transfer paths
13:#   * COLD CACHES before every timed run, both ends: `purge` on the
55:# The daemon binary must already be staged at $ZOEY_TEMP/blit-daemon
128:zssh "test -x '$ZOEY_TEMP/blit-daemon'" || {
129:    echo "daemon binary not staged at $ZOEY_TEMP/blit-daemon"; exit 1; }
130:BUILD_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
132:log "build sha: $BUILD_SHA  client: $(uname -m) macOS  daemon: $ZOEY_HOST  session: $SESSION_TAG"
146:nohup '$ZOEY_TEMP/blit-daemon' --config '$ZOEY_TEMP/bench-config.toml' \

exec
/bin/zsh -lc 'git show d71c0ed''^:docs/plan/OTP12_PERF_FINDINGS.md | nl -ba | sed -n '"'330,382p'; git show d71c0ed"'^:docs/STATE.md | nl -ba | sed -n '"'1,15p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Dh1HPfMJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-k0xzAJPe' (errno=Operation not permitted)
   330	- **This does NOT prove the interleaved design has enough resolution** — that is
   331	  a different (paired, within-session) variance, and pf-0 did not measure it.
   332	  **pf-1 must measure its own paired within-session noise floor on the
   333	  unmodified build and register a resolution check** (its smallest reportable
   334	  recovery must exceed that floor) *before* grading any hypothesis. A pf-1
   335	  recovery quoted without its paired floor is uninterpretable.
   336	- **The noise is not diffuse — it is a bistable fast arm.** The `win_init` runs
   337	  are **bimodal** (roughly ~730 ms and ~840 ms clusters); S1 drew 6 low/2 high
   338	  and S4 drew 2 low/6 high **at the same MTU**, and that mixture — not MTU — is
   339	  what produced the 72 ms `win_init` replicate spread and hence N_Δ. The
   340	  `mac_init` arm is by contrast stable to **5–6 ms**. **Trap for pf-1: a
   341	  counterfactual that merely shifts the mode mixture would masquerade as a
   342	  recovery.** Grade on the run distribution, not the median alone. (The MTU
   343	  verdict is robust to this: pooling all 16 runs per condition gives
   344	  Δ_9000 = 232, Δ_1500 = 221.5, r = −4.7% — same KILLED grade.)
   345	
   346	**OPEN — pf-final's committed reference is MTU-mismatched (owner's amendment,
   347	NOT decided here).** The fabric now runs MTU 9000; the committed reference
   348	`docs/bench/otp2w-baseline-2026-07-10/summary.csv` was recorded at **MTU 1500**
   349	and is deliberately **frozen** as an anti-drift ceiling
   350	(`OTP12_ACCEPTANCE_RUN.md` D2/D5). Acceptance requires **both** references, so
   351	this plan must not quietly reinterpret the contract — the following is the
   352	exposure, stated for the owner, and this plan asserts no void rule of its own:
   353	
   354	- pf-0 measured jumbo making both arms **3–4% faster**. A jumbo NEW arm compared
   355	  against a **1500-recorded** ceiling is therefore **lenient, not conservative**
   356	  — the MTU gain flatters the ratio and could let a real regression pass. That
   357	  is the actual risk, and it argues the mismatch matters.
   358	- The ways out (re-recording the frozen baseline at 9000; running pf-final at
   359	  1500; or an explicit MTU-mismatch rule) each **change the frozen-baseline
   360	  contract or the rig configuration, and so require an owner amendment**.
   361	  Re-recording additionally needs a harness change (`bench_otp12_win.sh:105`
   362	  hardcodes `BASELINE_SUMMARY` with no override, by design).
   363	
   364	Same-session references (`old_session`) are MTU-matched by construction and are
   365	unaffected either way.
   366	
   367	## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
   368	
   369	- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
   370	  connection-initiating end DIALS; byte direction is role-set
   371	  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
   372	  session the SOURCE is the responder: each sf-2 resize epoch is
   373	  ACCEPTED off the source's listener while the DESTINATION dials
   374	  (otp-5b-2: `SourceSockets` Dial/Accept branches;
   375	  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
   376	  accept/dial round-trips or serialization in the accept branch that the
   377	  dial branch does not pay.
   378	  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
   379	  was "mixed exercises resize hardest", but **all three fixtures target
   380	  eight streams before clamping** (`src/dial.rs:474`) — so resize
   381	  *count* cannot explain mixed-only behaviour, and H1 must name what
   382	  about mixed differs (shard-boundary timing? the tar-shard small half
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-yYpB3046' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LtZYylr4' (errno=Operation not permitted)
     1	# STATE — single entry point for "what is true right now"
     2	
     3	Last updated: 2026-07-14
     4	
     5	- **NEXT ACTION — an OWNER DECISION, and it blocks pf-final's assembly: the frozen committed baseline is MTU-mismatched.** The fabric now runs **MTU 9000**; the committed anti-drift ceiling `docs/bench/otp2w-baseline-2026-07-10/summary.csv` was recorded at **MTU 1500**, and acceptance requires **BOTH** references (`OTP12_ACCEPTANCE_RUN.md` D2/D5, frozen by design). pf-0 measured jumbo making **both arms 3–4% faster**, so a jumbo NEW arm graded against a 1500 ceiling is **LENIENT, not conservative** — the MTU gain flatters the ratio and could let a real regression pass. Ways out (re-record the baseline at 9000 / run pf-final at 1500 / an explicit MTU-mismatch rule) each change the frozen contract or the rig config, so **each needs the owner's amendment — no agent may pick one.** Full exposure: `docs/plan/OTP12_PERF_FINDINGS.md` §pf-0. Then: **pf-1**.
     6	- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
     7	- **THE FAST ARM IS BISTABLE — a trap for pf-1.** `win_init` runs are **bimodal** (~730 ms and ~840 ms); S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. **A counterfactual that merely shifts the mixture would masquerade as a recovery: grade the run distribution, not the median.**
     8	- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
     9	- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
    10	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
    11	- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
    12	- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**
    13	
    14	Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
    15	≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`

exec
/bin/zsh -lc "nl -ba docs/STATE.md | sed -n '60,104p'; nl -ba docs/STATE.md | sed -n '180,202p'; git diff d71c0ed"'^ d71c0ed -- docs/STATE.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    60	   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
    61	   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
    62	   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
    63	   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
    64	   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
    65	   **otp-12d and otp-13 are DEFERRED, not next** — otp-12c's rows are
    66	   PRE-FIX, and `docs/plan/OTP12_PERF_FINDINGS.md` (pf-final) voids
    67	   pre-fix new arms for acceptance. Assembling the acceptance matrix now
    68	   would build otp-13's artifact from void rows.
    69	1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
    70	   (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
    71	   reviewloop slice by slice"; implementation proceeds, each slice
    72	   through the codex loop).
    73	   Two experiments come BEFORE any code; both docs own their detail.
    74	   **(i) The A-B-B-A MTU run on `q`** —
    75	   `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` (rev 4;
    76	   codex 15/15 accepted). Answers how much MTU *contributes*; we already
    77	   know jumbo does not FIX P1 (q baseline, 1.385 at 9000).
    78	   **(ii) THE MAC↔MAC RIG — the missing cell, and it discriminates the
    79	   hypotheses** (owner, 2026-07-13; UNTESTED, now possible: nagatha `.92`
    80	   + `q` `.54`, both 10GbE/MTU 9000). Linux↔Linux = **no P1** (8/8 PASS);
    81	   macOS↔Windows = **P1** (1.237/1.300/1.385); macOS↔macOS = **?**
    82	   - reproduces → P1 needs **no Windows peer**; it is macOS-side and
    83	     **H1 DIES** (H1 accuses the *Windows* accept branch);
    84	   - vanishes → P1 **requires** the Windows peer → H1 strongly supported.
    85	   Needs a 3rd harness variant (rig-W's is Windows-specific; the Linux
    86	   one is Linux-specific) — macOS durability (fsync walk) + `purge` both
    87	   ends; through the codex loop. **Schedule for nagatha idle time.**
    88	   **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a
    89	   *cross-direction* miss for a cell that ALREADY passes invariance — P1
    90	   *is* the invariance failure. **Fix it to ≤1.10, or the owner amends
    91	   acceptance criterion 1.** Not assumed either way. P2
    92	   (`push_tcp_small` 1.105–1.201) is a converge bar vs the OLD build,
    93	   UNTESTED on the Linux rig. Sequence: **MTU run + Mac↔Mac → pf-1 → fix
    94	   → pf-final (ALL rigs) → otp-12d → otp-13.**
    95	1b. **AFTER otp-12 — the Windows/local pair, planned TOGETHER** (same tar
    96	   path, opposite directions: a fidelity fix ADDS per-file work to a path
    97	   already losing to robocopy, so planning them apart optimises one against
    98	   the other). Both docs own their detail; do not restate it here.
    99	   - **`docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` (D-2026-07-13-3)**
   100	     — Windows attributes + ADS silently dropped, exit 0, **both routes
   101	     (measured)**; loss is **conditional on file count**
   102	     (`transfer_plan.rs:103-109`). Unlanded Windows support, NOT a regression.
   103	     **Fix = WIRE CONTRACT change** → amend `TRANSFER_SESSION.md` first.
   104	   - **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2)** — local
   180	  (`11f0c2a`) — it confirmed every number and killed every *claim* that outran
   181	  them: the run is **not powered** to exclude a *contributing*-size effect
   182	  (46 ms < the 78 ms floor), "P1 is code-shaped" was **not** established (MTU is
   183	  one variable; segment fill unmeasured), and declaring the frozen baseline VOID
   184	  was **not an agent's call**. **The fast arm is BISTABLE** (bimodal `win_init`;
   185	  the mode mixture, not MTU, sets the noise floor) — a pf-1 counterfactual that
   186	  shifts the mixture would fake a recovery. Rig: Time Machine on `q` fired 1 min
   187	  before the run (owner disabled it; **the harness's quiet-gate does not catch
   188	  it**), and three starts died on a **physically flapping `en8`** the owner
   189	  reseated — I chased three deterministic theories and falsified all three.
   190	  **In-flight: none. Rigs clean, Windows MTU 9000, TM still OFF on `q` (owner
   191	  re-enables), 4 + 4 commits unpushed.**
   192	  **NEXT: the owner's decision on the MTU-mismatched frozen baseline** (it gates
   193	  pf-final's assembly — see NEXT ACTION at the top), **then pf-1.**
   194	- **2026-07-13/14 (47th)** — P1 reproduces on a second Mac (`q`); new bench Mac;
   195	  Windows attrs+ADS bug (D-2026-07-13-3); the robocopy headline was WRONG
   196	  (D-2026-07-13-2); MTU prereg rev 1→4. Full: **DEVLOG 2026-07-14 00:15Z**.
   197	- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig (8/8 PASS
   198	  → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1); **three claims
   199	  retracted, all from unvalidated instruments**. Full: **DEVLOG 2026-07-13 20:00Z**.
   200	- *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-W2gokV0B' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-hS2BQPOR' (errno=Operation not permitted)
diff --git a/docs/STATE.md b/docs/STATE.md
index b811238..2edff44 100644
--- a/docs/STATE.md
+++ b/docs/STATE.md
@@ -2,9 +2,10 @@
 
 Last updated: 2026-07-14
 
-- **NEXT ACTION — an OWNER DECISION, and it blocks pf-final's assembly: the frozen committed baseline is MTU-mismatched.** The fabric now runs **MTU 9000**; the committed anti-drift ceiling `docs/bench/otp2w-baseline-2026-07-10/summary.csv` was recorded at **MTU 1500**, and acceptance requires **BOTH** references (`OTP12_ACCEPTANCE_RUN.md` D2/D5, frozen by design). pf-0 measured jumbo making **both arms 3–4% faster**, so a jumbo NEW arm graded against a 1500 ceiling is **LENIENT, not conservative** — the MTU gain flatters the ratio and could let a real regression pass. Ways out (re-record the baseline at 9000 / run pf-final at 1500 / an explicit MTU-mismatch rule) each change the frozen contract or the rig config, so **each needs the owner's amendment — no agent may pick one.** Full exposure: `docs/plan/OTP12_PERF_FINDINGS.md` §pf-0. Then: **pf-1**.
+- **NEXT ACTION — `pf-1` (the HARD GATE): instrumentation + the interleaved counterfactuals.** Two pf-0 results now BIND it: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor), so pf-1 must **measure its own paired within-session noise floor on the unmodified build and register a resolution check** — smallest reportable recovery > that floor — *before* grading any hypothesis; (b) **the fast arm is BISTABLE**, so grade the run distribution, not the median. Design: `docs/plan/OTP12_PERF_FINDINGS.md` §Method + §pf-1 decision rule.
+- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
 - **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
-- **THE FAST ARM IS BISTABLE — a trap for pf-1.** `win_init` runs are **bimodal** (~730 ms and ~840 ms); S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. **A counterfactual that merely shifts the mixture would masquerade as a recovery: grade the run distribution, not the median.**
+- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
 - **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
 - **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
 - Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
@@ -193,8 +194,7 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 - **2026-07-13/14 (47th)** — P1 reproduces on a second Mac (`q`); new bench Mac;
   Windows attrs+ADS bug (D-2026-07-13-3); the robocopy headline was WRONG
   (D-2026-07-13-2); MTU prereg rev 1→4. Full: **DEVLOG 2026-07-14 00:15Z**.
-- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig built
-  (8/8 PASS → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1);
-  **three claims retracted, all from trusting an unvalidated instrument**.
-  Full entry: **DEVLOG 2026-07-13 20:00Z**.
+- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig (8/8 PASS
+  → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1); **three claims
+  retracted, all from unvalidated instruments**. Full: **DEVLOG 2026-07-13 20:00Z**.
 - *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '500,585p'; nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '28,52p'; git blame -L 69,94 --date=short docs/STATE.md; git blame -L 190,193 --date=short docs/STATE.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   500	   the receive path, shard planner in/out timestamps.
   501	2. **A/B the role layouts in one process**: the role suite already
   502	   runs both initiator layouts over identical fixtures (otp-3) — but
   503	   it forces the in-stream carrier (`transfer_session_roles.rs`), so
   504	   the timing-harness variant MUST add a TCP-carrier mode; it reports
   505	   phase timings per layout for mixed and small fixtures. A positive
   506	   layout-dependent delta in a named phase confirms; local ABSENCE
   507	   does not kill H1 (loopback removes the Windows↔Mac topology). So
   508	   that H1 stays falsifiable: if the local run is negative, pf-1
   509	   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
   510	   CELLS fixtures) before pf-1 may close — every hypothesis exits
   511	   pf-1 confirmed or killed, never "unfalsified" (review round 2).
   512	3. **Historical control, then bisect P2**: old push is deleted from
   513	   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
   514	   build and run; the control is an old-vs-new run on identical
   515	   fixtures. The new tracing spans do NOT exist in `0f922de` (review
   516	   round 2), so the control is observed externally — phase boundaries
   517	   from wire + filesystem timestamps and stdout progress, with event
   518	   semantics mapped span-for-span to the new names — or, where that is
   519	   too coarse, a minimal probe backport onto the pinned `0f922de`
   520	   source with identical event names. Either way every timed
   521	   configuration runs an instrumentation-on/off pair to bound observer
   522	   overhead (per-member tracing across ~10k files can perturb a
   523	   double-digit share of the measured gap). Experiments, corrected per
   524	   review 2026-07-12: (a) precreate-vs-not stays but is
   525	   environmental-only (it cannot attribute code); (b) the flush/
   526	   instrument toggles missed the tar-shard path — instrument the
   527	   tar-shard write path itself; (c) REPLACED (review round 2) — the
   528	   ramp pin discriminated nothing (old push also opened at one
   529	   stream), but H4 keeps a code-level counterfactual: a batch-cadence
   530	   replay toggle that processes need batches at the recorded old-push
   531	   shard-boundary cadence; (d) NEW, for H5 — the overlap experiment,
   532	   metric DEFINED (review round 2: "manifest-complete→first-payload
   533	   gap" was underdefined, and for old push the quantity is expected to
   534	   be NEGATIVE, which an unsigned "gap" cannot express). Record, per
   535	   run, on ONE common clock with a SIGNED offset from the
   536	   `ManifestComplete` event, three separately-named events on the
   537	   source side plus one on the destination:
   538	   `t_manifest_complete`; `t_first_payload_queued` (the payload enters
   539	   the send queue); `t_first_socket_write` (first byte handed to the
   540	   TCP data plane); `t_first_payload_received` (destination side —
   541	   requires the two clocks to be reconciled, so record the ssh/NTP
   542	   offset per run and report it with the number, or state that the
   543	   destination event was not usable). The overlap DIFFERENCE is
   544	   established only if `t_first_socket_write − t_manifest_complete` is
   545	   ≈0-or-positive on the new build and provably NEGATIVE on the pinned
   546	   `0f922de` control for the SAME fixture — i.e. old push really did put
   547	   TCP bytes on the wire before its manifest completed, and the new
   548	   session does not.
   549	   **That timestamp proves ORDERING, not CAUSATION, so it cannot confirm
   550	   H5 (review round 3).** H5 is confirmed only by a causal
   551	   counterfactual: a debug-flag toggle that restores mid-manifest TCP
   552	   payload queueing (queueing/ordering only — if it cannot be done
   553	   without a wire change, this plan's Contract stop-and-amend rule fires
   554	   FIRST) and measures WALL TIME on the same fixture and rig,
   555	   interleaved old-vs-new. Pre-registered: H5 is CONFIRMED iff the
   556	   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
   557	   KILLED if it restores the old ordering but does not move wall time —
   558	   which would prove the lost overlap is real and irrelevant, and hand
   559	   P2 to H6;
   560	   (e) per-member locking/framing timings are now an unconditional pf-1
   561	   measurement (they discriminate H6), not contingent on the trace
   562	   implicating them.
   563	4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
   564	   local rig is Mac↔Mac loopback: it removes the very platform terms P1
   565	   is confounded with, and it may equally fail to surface P2 (whose
   566	   Windows arms are the sharpest). So the rule is symmetric — **if a
   567	   finding does not reproduce locally, pf-1 REQUIRES the rig-side
   568	   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
   569	   since P2 was measured on both) with the same spans and the CELLS
   570	   fixtures, before pf-1 may close. Every hypothesis exits pf-1
   571	   confirmed or killed — never "did not reproduce, moving on".
   572	5. Every experiment lands as a committed probe record under
   573	   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   574	   loop per slice as usual.
   575	
   576	## pf-1 decision rule — UNIFORM, pre-registered (added round 5)
   577	
   578	Round-4 review: individual hypotheses had no shared decision threshold —
   579	H1 accepted any positive phase delta, H4's cadence replay had no
   580	threshold, H5 left a 1–49% recovery undecided, H6 left "material share"
   581	undefined. A phase-timing delta is **descriptive**; only wall time
   582	decides. So ONE rule governs every hypothesis (H1, H4, H5, H6, H7):
   583	
   584	- Each hypothesis must have a **wall-time counterfactual**: a debug-flag
   585	  variant that removes or restores exactly the accused mechanism, run
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
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-YHzo4lYV' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ke7s75Hd' (errno=Operation not permitted)
c9101db4 (Michael Coelho 2026-07-13 69) 1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
c9101db4 (Michael Coelho 2026-07-13 70)    (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
c9101db4 (Michael Coelho 2026-07-13 71)    reviewloop slice by slice"; implementation proceeds, each slice
c9101db4 (Michael Coelho 2026-07-13 72)    through the codex loop).
4456aaf4 (Michael Coelho 2026-07-13 73)    Two experiments come BEFORE any code; both docs own their detail.
4456aaf4 (Michael Coelho 2026-07-13 74)    **(i) The A-B-B-A MTU run on `q`** —
4456aaf4 (Michael Coelho 2026-07-13 75)    `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` (rev 4;
4456aaf4 (Michael Coelho 2026-07-13 76)    codex 15/15 accepted). Answers how much MTU *contributes*; we already
4456aaf4 (Michael Coelho 2026-07-13 77)    know jumbo does not FIX P1 (q baseline, 1.385 at 9000).
4456aaf4 (Michael Coelho 2026-07-13 78)    **(ii) THE MAC↔MAC RIG — the missing cell, and it discriminates the
4456aaf4 (Michael Coelho 2026-07-13 79)    hypotheses** (owner, 2026-07-13; UNTESTED, now possible: nagatha `.92`
4456aaf4 (Michael Coelho 2026-07-13 80)    + `q` `.54`, both 10GbE/MTU 9000). Linux↔Linux = **no P1** (8/8 PASS);
4456aaf4 (Michael Coelho 2026-07-13 81)    macOS↔Windows = **P1** (1.237/1.300/1.385); macOS↔macOS = **?**
4456aaf4 (Michael Coelho 2026-07-13 82)    - reproduces → P1 needs **no Windows peer**; it is macOS-side and
4456aaf4 (Michael Coelho 2026-07-13 83)      **H1 DIES** (H1 accuses the *Windows* accept branch);
4456aaf4 (Michael Coelho 2026-07-13 84)    - vanishes → P1 **requires** the Windows peer → H1 strongly supported.
4456aaf4 (Michael Coelho 2026-07-13 85)    Needs a 3rd harness variant (rig-W's is Windows-specific; the Linux
4456aaf4 (Michael Coelho 2026-07-13 86)    one is Linux-specific) — macOS durability (fsync walk) + `purge` both
4456aaf4 (Michael Coelho 2026-07-13 87)    ends; through the codex loop. **Schedule for nagatha idle time.**
4456aaf4 (Michael Coelho 2026-07-13 88)    **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a
4456aaf4 (Michael Coelho 2026-07-13 89)    *cross-direction* miss for a cell that ALREADY passes invariance — P1
4456aaf4 (Michael Coelho 2026-07-13 90)    *is* the invariance failure. **Fix it to ≤1.10, or the owner amends
4456aaf4 (Michael Coelho 2026-07-13 91)    acceptance criterion 1.** Not assumed either way. P2
4456aaf4 (Michael Coelho 2026-07-13 92)    (`push_tcp_small` 1.105–1.201) is a converge bar vs the OLD build,
4456aaf4 (Michael Coelho 2026-07-13 93)    UNTESTED on the Linux rig. Sequence: **MTU run + Mac↔Mac → pf-1 → fix
4456aaf4 (Michael Coelho 2026-07-13 94)    → pf-final (ALL rigs) → otp-12d → otp-13.**
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Um0c1fgu' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-VXiud8oG' (errno=Operation not permitted)
bb912f45 (Michael Coelho 2026-07-14 190)   **In-flight: none. Rigs clean, Windows MTU 9000, TM still OFF on `q` (owner
bb912f45 (Michael Coelho 2026-07-14 191)   re-enables), 4 + 4 commits unpushed.**
bb912f45 (Michael Coelho 2026-07-14 192)   **NEXT: the owner's decision on the MTU-mismatched frozen baseline** (it gates
bb912f45 (Michael Coelho 2026-07-14 193)   pf-final's assembly — see NEXT ACTION at the top), **then pf-1.**

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/runs.csv | sed -n '1,8p'; nl -ba docs/bench/otp12-jumbo-win-2026-07-13/S1_9000/summary.csv | sed -n '1,12p'; nl -ba docs/bench/otp12-jumbo-win-2026-07-13/S2_1500/summary.csv | sed -n '1,12p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid
     2	wm_tcp_large,mac_init,f35702a,mac,1,932,0,0,drained_10s,yes
     3	wm_tcp_large,win_init,f35702a,win,1,914,0,0,drained_8s,yes
     4	wm_tcp_large,mac_init,f35702a,mac,2,908,0,0,drained_6s,yes
     5	wm_tcp_large,win_init,f35702a,win,2,906,0,0,drained_6s,yes
     6	wm_tcp_large,mac_init,f35702a,mac,3,909,0,0,drained_6s,yes
     7	wm_tcp_large,win_init,f35702a,win,3,904,0,0,drained_6s,yes
     8	wm_tcp_large,mac_init,f35702a,mac,4,907,0,0,drained_6s,yes
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	mw_tcp_mixed,mac_init,1710,1705,1592,10.7,0,8
     3	mw_tcp_mixed,win_init,1641,1681,1585,16.5,0,8
     4	wm_grpc_mixed,mac_init,1121,1145,1112,8.8,0,8
     5	wm_grpc_mixed,win_init,1128,1130,1099,6.4,0,8
     6	wm_tcp_large,mac_init,908,914,907,2.9,0,8
     7	wm_tcp_large,win_init,908,909,904,1.9,0,8
     8	wm_tcp_mixed,mac_init,1035,1044,1020,7.1,0,8
     9	wm_tcp_mixed,win_init,760,768,699,20.7,0,8
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	mw_tcp_mixed,mac_init,1607,1645,1567,12.1,0,8
     3	mw_tcp_mixed,win_init,1641,1632,1545,13.5,0,8
     4	wm_grpc_mixed,mac_init,1184,1192,1133,17.6,0,8
     5	wm_grpc_mixed,win_init,1159,1163,1128,6.2,0,8
     6	wm_tcp_large,mac_init,960,963,954,2.9,0,8
     7	wm_tcp_large,win_init,946,960,943,10.9,0,8
     8	wm_tcp_mixed,mac_init,1071,1082,1056,6.6,0,8
     9	wm_tcp_mixed,win_init,830,812,716,22.2,0,8

codex
- **HIGH — [docs/DECISIONS.md:195](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:195), [OTP12_ACCEPTANCE_RUN.md:168](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:168)** — Re-recording controls build and MSS, but not endpoint/day state. Rig W now uses the new, faster `q` ([machines.md:100](/Users/michael/Dev/blit_v2/.agents/machines.md:100)); thus the ceiling changes hardware, OS/storage state, and MTU simultaneously. Without an endpoint pin or non-loosening guard, a slower rerun can weaken exactly the anti-drift protection D2 says prevents slower OLD reruns from loosening acceptance. Old-file immutability survives; the functional freeze does not.

- **HIGH — [OTP12_ACCEPTANCE_RUN.md:168](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:168), [OTP12_ACCEPTANCE_RUN.md:292](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:292)** — D2 still mandates the committed **2026-07-10** median while D5 declares it historical and superseded. Because each harness has only one `BASELINE_SUMMARY`, repointing either violates D2 or leaves the new baseline without a coherent grading contract. The historical baseline READMEs also remain labeled as committed references.

- **HIGH — [docs/DECISIONS.md:199](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:199), [OTP12_ACCEPTANCE_RUN.md:93](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:93), [OTP12_ACCEPTANCE_RUN.md:369](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:369)** — Rig Z has no single reproducible “original OLD build”: its client was clean `e757dcc`, but its daemon was an unknowably dirty `731023b`. D-14 requires that dirty artifact, while D1/D6 require a clean matched `e757dcc` pair. Reusing one violates clean-pair discipline; using the other changes the reference build.

- **MEDIUM — [docs/DECISIONS.md:196](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:196), [pf-0 README.md:103](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:103)** — The 3–4% claim is over-applied. It describes `wm_tcp_large` on rig W; both measured arms were the same new `f35702a` build, not OLD versus NEW. pf-0 measured no small cells, no rig Z cells, and no OLD-build MTU response; its committed-reference rows were explicitly void ([README.md:149](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:149)). It establishes a mismatch, not blanket 3–4% leniency across both matrices.

- **MEDIUM — [docs/STATE.md:5](/Users/michael/Dev/blit_v2/docs/STATE.md:5)** — STATE names `pf-1` next, but its live queue still requires Mac↔Mac before `pf-1` ([STATE.md:73](/Users/michael/Dev/blit_v2/docs/STATE.md:73)), and its newest handoff still says the owner baseline decision is next ([STATE.md:192](/Users/michael/Dev/blit_v2/docs/STATE.md:192)).

- **LOW — [docs/DECISIONS.md:196](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:196)** — “P1 is the one finding between blit and shipping” understates the active contract: P2 remains a committed, hard both-rigs converge-up bar ([OTP12_PERF_FINDINGS.md:648](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:648)).

**VERDICT: NOT READY.** The W/Z/D scope itself is correct—Zoey was converted 1500→9000 on July 13, and rig D has no external baseline—and there is no separate mechanical row omission or double-count. The replacement pin, however, is internally contradictory and does not preserve D2’s anti-drift guarantee.
tokens used
132,955
- **HIGH — [docs/DECISIONS.md:195](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:195), [OTP12_ACCEPTANCE_RUN.md:168](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:168)** — Re-recording controls build and MSS, but not endpoint/day state. Rig W now uses the new, faster `q` ([machines.md:100](/Users/michael/Dev/blit_v2/.agents/machines.md:100)); thus the ceiling changes hardware, OS/storage state, and MTU simultaneously. Without an endpoint pin or non-loosening guard, a slower rerun can weaken exactly the anti-drift protection D2 says prevents slower OLD reruns from loosening acceptance. Old-file immutability survives; the functional freeze does not.

- **HIGH — [OTP12_ACCEPTANCE_RUN.md:168](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:168), [OTP12_ACCEPTANCE_RUN.md:292](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:292)** — D2 still mandates the committed **2026-07-10** median while D5 declares it historical and superseded. Because each harness has only one `BASELINE_SUMMARY`, repointing either violates D2 or leaves the new baseline without a coherent grading contract. The historical baseline READMEs also remain labeled as committed references.

- **HIGH — [docs/DECISIONS.md:199](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:199), [OTP12_ACCEPTANCE_RUN.md:93](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:93), [OTP12_ACCEPTANCE_RUN.md:369](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:369)** — Rig Z has no single reproducible “original OLD build”: its client was clean `e757dcc`, but its daemon was an unknowably dirty `731023b`. D-14 requires that dirty artifact, while D1/D6 require a clean matched `e757dcc` pair. Reusing one violates clean-pair discipline; using the other changes the reference build.

- **MEDIUM — [docs/DECISIONS.md:196](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:196), [pf-0 README.md:103](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:103)** — The 3–4% claim is over-applied. It describes `wm_tcp_large` on rig W; both measured arms were the same new `f35702a` build, not OLD versus NEW. pf-0 measured no small cells, no rig Z cells, and no OLD-build MTU response; its committed-reference rows were explicitly void ([README.md:149](/Users/michael/Dev/blit_v2/docs/bench/otp12-jumbo-win-2026-07-13/README.md:149)). It establishes a mismatch, not blanket 3–4% leniency across both matrices.

- **MEDIUM — [docs/STATE.md:5](/Users/michael/Dev/blit_v2/docs/STATE.md:5)** — STATE names `pf-1` next, but its live queue still requires Mac↔Mac before `pf-1` ([STATE.md:73](/Users/michael/Dev/blit_v2/docs/STATE.md:73)), and its newest handoff still says the owner baseline decision is next ([STATE.md:192](/Users/michael/Dev/blit_v2/docs/STATE.md:192)).

- **LOW — [docs/DECISIONS.md:196](/Users/michael/Dev/blit_v2/docs/DECISIONS.md:196)** — “P1 is the one finding between blit and shipping” understates the active contract: P2 remains a committed, hard both-rigs converge-up bar ([OTP12_PERF_FINDINGS.md:648](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:648)).

**VERDICT: NOT READY.** The W/Z/D scope itself is correct—Zoey was converted 1500→9000 on July 13, and rig D has no external baseline—and there is no separate mechanical row omission or double-count. The replacement pin, however, is internally contradictory and does not preserve D2’s anti-drift guarantee.
