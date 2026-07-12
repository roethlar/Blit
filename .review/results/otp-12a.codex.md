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
session id: 019f54b3-dd37-7143-930b-ca9a893ac67c
--------
user
Review the diff of commit 8f4fbf9 (run: git show 8f4fbf9). It implements otp-12a's harness half per docs/plan/OTP12_ACCEPTANCE_RUN.md (Active) D1/D2/D5/D6: scripts/bench_otp12_zoey.sh, a new self-contained benchmark harness derived from the frozen scripts/bench_otp2_baseline.sh, running the 12-comparison otp-2 matrix as interleaved matched-pair old-vs-new A/B on the Mac<->zoey rig. Check: (1) faithfulness to the design doc's pre-registered rules - ABBA counterbalance per pair slot, the pair-void valid-run rule (nonzero exit OR undrained window voids the PAIR, re-run same slot, cap 2xRUNS attempts, INCOMPLETE surfaced), verdicts against BOTH references (same-session old arm AND docs/bench/otp2-baseline-2026-07-10/summary.csv) with the exact outcome vocabulary (PASS/FAIL-SAME-SESSION/FAIL-REFERENCE-DRIFT/FAIL-BOTH/INCOMPLETE) and the integer-exact 1.10 bar; (2) methodology fidelity to bench_otp2_baseline.sh where the design says verbatim (self-timed flushes, drain-then-purge order, wall-clock windows, fixture recipes, fresh destinations, median = floor of mean of middle two); (3) bash correctness: macOS bash 3.2 compatibility (no associative arrays, no bash-4-isms), set -euo pipefail interactions (arithmetic in conditions, exit-code capture around the timed blit invocation, trap ordering, SESSION_TAG defined before the trap/main), quoting of remote ssh heredocs, the CSV row assembly, and the python3 verdict pass (csv parsing, median, integer bar check 10*new<=11*ref, INCOMPLETE handling when an arm has no valid runs); (4) any way a run could record an unfair or bogus time (flush attribution, daemon swap inside a timed window, staged pull sources, smoke dirs swept); (5) the one-line schema amendment in the plan doc (runs.csv +valid) and the findings doc's claims. The suite is untouched (no crates/proto changes) - test count is not a concern for this slice. Output a concise markdown findings list - each finding with file:line, severity, rationale - then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the commit against the active acceptance plan and frozen baseline, then independently check shell/control-flow and verdict logic. The ptk server isn’t available here, so I’ll use read-only shell commands.
exec
/bin/zsh -lc "sed -n '1,260p' AGENTS.md && sed -n '1,320p' .agents/repo-guidance.md && if [ -f .agents/state.md ]; then sed -n '1,320p' .agents/state.md; fi" in /Users/michael/Dev/blit_v2
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
/bin/zsh -lc 'git show --stat --oneline --decorate --no-renames 8f4fbf9 && git show --format=fuller --no-ext-diff --no-renames 8f4fbf9 --' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-b63LM0yD' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Imz6rQ87' (errno=Operation not permitted)
8f4fbf9 (HEAD -> master) otp-12a: zoey converge-up harness (interleaved old-vs-new, ABBA, both-reference verdicts)
 .review/findings/otp-12a-zoey-harness.md |  72 +++++
 docs/plan/OTP12_ACCEPTANCE_RUN.md        |   5 +-
 scripts/bench_otp12_zoey.sh              | 481 +++++++++++++++++++++++++++++++
 3 files changed, 557 insertions(+), 1 deletion(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-59Fn2XrC' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-vQcj6Gl8' (errno=Operation not permitted)
commit 8f4fbf9a7cacacdf22e6f57530cef2e91fb7351c
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 12 01:01:10 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 12 01:01:10 2026 -0400

    otp-12a: zoey converge-up harness (interleaved old-vs-new, ABBA, both-reference verdicts)
    
    bench_otp12_zoey.sh: otp-2 matrix as matched-pair A/B (old=e757dcc pair,
    new=run commit), ABBA counterbalance (F5), pair-void valid-run rule with
    2xRUNS cap + INCOMPLETE (F7), exit codes captured, stale-daemon refusal,
    sha256 staging manifest, PREFLIGHT_ONLY mode, python3 summary+verdicts
    against both references (D2). runs.csv schema +valid column (doc amended).
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

diff --git a/.review/findings/otp-12a-zoey-harness.md b/.review/findings/otp-12a-zoey-harness.md
new file mode 100644
index 0000000..3a76090
--- /dev/null
+++ b/.review/findings/otp-12a-zoey-harness.md
@@ -0,0 +1,72 @@
+# otp-12a — zoey converge-up harness (interleaved old-vs-new)
+
+**Plan**: `docs/plan/OTP12_ACCEPTANCE_RUN.md` (Active, owner 2026-07-12),
+sub-slice 12a, harness half. The recorded-run half follows on the rig
+(needs the owner's fresh go for daemon runs on zoey + zoey out of
+maintenance).
+**Status**: implemented, codex review pending.
+
+## What
+
+`scripts/bench_otp12_zoey.sh` — the otp-2 verdict matrix ({large, small,
+mixed} × {push, pull} × {tcp, grpc} = 12 comparisons) rerun as
+matched-pair interleaved A/B: arm old = pinned `e757dcc` pair (Mac client
+staged at `$MAC_WORK/bins/blit-e757dcc`, zoey's kept 2026-07-10 daemon),
+arm new = the run commit's pair (local release build + freshly zigbuilt
+musl daemon staged beside the old one). Per-direction converge-up only
+(D-2026-07-05-1); verdicts computed against BOTH references (same-session
+old arm AND the committed `docs/bench/otp2-baseline-2026-07-10/summary.csv`
+medians), per design D2.
+
+## Approach
+
+Methodology functions carried verbatim from `bench_otp2_baseline.sh`
+(wall-clock windows, self-timed destination flushes, drain-then-purge
+ordering, fixture recipes, ControlMaster mux). New mechanics per the
+design doc: ABBA counterbalanced pair order (F5); pair-void-and-re-run
+valid-run rule with a 2×RUNS attempt cap and INCOMPLETE surfacing (F7);
+blit exit codes captured with per-run logs under `$OUT_DIR/blit-logs/`
+(the old harness swallowed them); daemon lifecycle parameterized by arm
+with swap-only-on-arm-change (untimed) plus a stale-daemon refusal
+(otp-2w F2 posture, new on this rig); binary provenance recorded to
+`staging-manifest.txt` (sha256 all four binaries — the OLD pair predates
+the handshake, so provenance is the staging record; the NEW pair's smoke
+transfer doubles as its build-identity check via D-2026-07-05-2);
+`PREFLIGHT_ONLY=1` mode (no daemon start, nothing timed); summary +
+verdict computation in one python3 pass (macOS ships bash 3.2 — no
+associative arrays anywhere).
+
+## Files
+
+- `scripts/bench_otp12_zoey.sh` (new; self-contained by design D5 — the
+  frozen `bench_otp2_baseline.sh` is untouched).
+- `docs/plan/OTP12_ACCEPTANCE_RUN.md` — D5 `runs.csv` schema gains the
+  `valid` column (pair-fate under the D2 rule; one-line amendment).
+
+## Tests
+
+- `bash -n` clean. shellcheck not installed on this machine (recorded
+  here rather than claimed).
+- No crates/proto/Cargo changes anywhere in otp-12
+  (`git diff --stat ce36da3..HEAD -- crates proto Cargo.toml Cargo.lock`
+  is empty); the suite stands at the recorded 1484 green from otp-11b.
+  A fresh gate run at this tree confirmed fmt + clippy pass and showed
+  no test failures.
+- The harness itself is verified by the probe/recorded-run discipline on
+  the rig (otp-2 precedent): the recorded-run half commits the evidence.
+
+## Known gaps
+
+- Not yet executed against the rig — PREFLIGHT_ONLY and the full matrix
+  both need zoey (maintenance 2026-07-11) and the owner's fresh daemon
+  go. First live session may surface busybox/ssh quirks the otp-2 script
+  did not (pgrep availability, sha256sum path).
+- Old-arm provenance rests on the staging record + sha256 manifest, not
+  a handshake (pre-handshake binaries) — accepted residual risk per the
+  design doc.
+- The escalation rule (straddle + spread > 25% → RUNS=8 fresh session)
+  is manual by design, not automated in the script.
+- `meta.csv` (pairs-attempted/completeness) is a working file consumed
+  by the verdict pass; the committed evidence carries its content via
+  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
+  rows.
diff --git a/docs/plan/OTP12_ACCEPTANCE_RUN.md b/docs/plan/OTP12_ACCEPTANCE_RUN.md
index e2d2a84..3e3f1d7 100644
--- a/docs/plan/OTP12_ACCEPTANCE_RUN.md
+++ b/docs/plan/OTP12_ACCEPTANCE_RUN.md
@@ -273,7 +273,10 @@ old scripts, both recorded sharp edges:
 - **Multi-token flags ride an array**, not an unquoted scalar.
 
 CSV schema (all rigs):
-`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain`
+`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
+(`valid` = the PAIR's fate under the D2 valid-run rule — an
+individually-clean run whose partner voided reads `no`; amended at the
+12a harness slice)
 `summary.csv`:
 `cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
 (medians over valid runs only — the D2 valid-run rule)
diff --git a/scripts/bench_otp12_zoey.sh b/scripts/bench_otp12_zoey.sh
new file mode 100644
index 0000000..8ba42b9
--- /dev/null
+++ b/scripts/bench_otp12_zoey.sh
@@ -0,0 +1,481 @@
+#!/usr/bin/env bash
+# otp-12a: interleaved OLD-vs-NEW converge-up matrix on the Mac<->zoey rig
+# (ONE_TRANSFER_PATH slice otp-12, sub-slice 12a; design:
+# docs/plan/OTP12_ACCEPTANCE_RUN.md D1/D2/D5/D6).
+#
+# What this measures: the otp-2 verdict matrix ({large,small,mixed} x
+# {push,pull} x {tcp,grpc} = 12 comparisons) rerun as matched-pair A/B —
+# arm "old" = the pinned pre-cutover pair (default e757dcc: Mac client
+# rebuilt at that sha in a detached worktree, zoey daemon already staged
+# in blit-temp since 2026-07-10), arm "new" = the run commit's pair.
+# This rig anchors PER-DIRECTION converge-up ONLY (hardware-asymmetric
+# endpoints, D-2026-07-05-1): a clean PASS needs new <= x1.10 of BOTH
+# references — the same-session old arm AND the committed 2026-07-10
+# baseline median (docs/bench/otp2-baseline-2026-07-10/summary.csv).
+# Cross-direction and invariance claims live on rig W (otp-12b), never
+# here.
+#
+# Methodology inherited verbatim from scripts/bench_otp2_baseline.sh
+# (cold caches both ends, drain-then-purge order, durable self-timed
+# destination flush, fresh never-seen destinations, wall-clock windows,
+# median = floor of the mean of the middle two). New in otp-12a:
+#   * ABBA counterbalanced interleave (codex design F5): pair slots run
+#     old,new / new,old / old,new / new,old — each arm leads half the
+#     pairs, so arm never confounds with within-pair order on the
+#     stateful pool.
+#   * Valid-run rule (codex design F7): a run with a nonzero blit exit
+#     OR an undrained pre-run window voids its whole PAIR; the pair is
+#     re-run at the same slot until RUNS valid pairs exist, capped at
+#     2*RUNS pair attempts per comparison; at the cap the comparison is
+#     recorded INCOMPLETE — never a silent pass, never a short median.
+#   * Exit codes checked (the old harness swallowed them inside the
+#     timed window); per-run blit output kept under $OUT_DIR/blit-logs/.
+#   * verdicts.csv computed at the end against both references
+#     (PASS / FAIL-SAME-SESSION / FAIL-REFERENCE-DRIFT / FAIL-BOTH /
+#     INCOMPLETE, per design D2).
+#   * Escalation (manual, design D2): a comparison that straddles its
+#     bar with either arm's spread > 25% is re-run in a fresh session
+#     at RUNS=8; both sessions get committed.
+#
+# Usage (from the client Mac):
+#   export ZOEY_SSH=root@zoey
+#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
+#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
+#   RUNS=4 ./scripts/bench_otp12_zoey.sh
+#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh   # checks only
+#
+# Prerequisites:
+#   * NEW pair: `cargo build --release` at the run commit with a CLEAN
+#     tree (a dirty build mints a distinct build id and the
+#     D-2026-07-05-2 handshake refuses the pair); zoey daemon zigbuilt
+#     (aarch64-musl, static) at the SAME commit and staged at
+#     $ZOEY_TEMP/blit-daemon-<sha>.
+#   * OLD pair: Mac client rebuilt at $OLD_SHA in a detached worktree
+#     and staged at $MAC_WORK/bins/blit-$OLD_SHA; zoey's pinned old
+#     daemon at $ZOEY_TEMP/blit-daemon (.agents/machines.md staging,
+#     kept for otp-12).
+#   * The OLD pair predates the handshake: its provenance is the
+#     staging record — this script records sha256 of every binary into
+#     staging-manifest.txt. The NEW pair's smoke transfer doubles as
+#     its identity check (a mismatched pair refuses with
+#     BUILD_MISMATCH at the first frame).
+#   * python3 + a NOPASSWD sudoers rule for /usr/sbin/purge on the Mac.
+#   * A RIG RUN needs the owner's fresh go for daemon runs on zoey
+#     (standing STATE rule). PREFLIGHT_ONLY=1 starts no daemon and
+#     times nothing (read-only ssh checks + local purge probe).
+#
+# Everything on the daemon host stays inside $ZOEY_TEMP (owner rule).
+
+set -euo pipefail
+
+SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
+REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
+
+ZOEY_SSH=${ZOEY_SSH:-root@zoey}
+ZOEY_TEMP=${ZOEY_TEMP:?set ZOEY_TEMP to the blit-temp folder on the daemon host}
+ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
+PORT=${PORT:-9031}
+RUNS=${RUNS:-4}
+PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
+# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
+MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
+
+OLD_SHA=${OLD_SHA_ZOEY:-e757dcc}
+NEW_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
+NEW_BLIT=${NEW_BLIT:-$REPO_ROOT/target/release/blit}
+OLD_BLIT=${OLD_BLIT:-$MAC_WORK/bins/blit-$OLD_SHA}
+OLD_DAEMON=${OLD_DAEMON:-$ZOEY_TEMP/blit-daemon}
+NEW_DAEMON=${NEW_DAEMON:-$ZOEY_TEMP/blit-daemon-$NEW_SHA}
+BASELINE_SUMMARY=${BASELINE_SUMMARY:-$REPO_ROOT/docs/bench/otp2-baseline-2026-07-10/summary.csv}
+
+OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_zoey_$(date +%Y%m%dT%H%M%S)}
+mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs" "$MAC_WORK"
+
+MODULE_ROOT="$ZOEY_TEMP/bench-module"
+REMOTE="$ZOEY_HOST:$PORT:/bench/"
+
+log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
+die() { log "FATAL: $*"; exit 1; }
+# ControlMaster multiplexing: an ssh connection to this host costs
+# ~1.2s (slow-core key exchange) — reuse one connection.
+SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
+zssh() { ssh "${SSH_MUX[@]}" "$ZOEY_SSH" "$@"; }
+# Wall-clock ms across two separate python3 processes (deliberate; see
+# bench_otp2_baseline.sh for why monotonic is wrong here).
+now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
+# Self-timed durability steps (codex otp-2w F3): the timed window is
+# transfer + destination flush and NOTHING else; each flush times
+# ITSELF on the destination and reports only its own duration.
+sync_dest_ms() {   # Linux sync on the daemon host; prints its elapsed ms
+    zssh 'a=$(awk "{print int(\$1*1000)}" /proc/uptime); sync; b=$(awk "{print int(\$1*1000)}" /proc/uptime); echo $((b-a))'
+}
+# Durable pull window: macOS sync(2) only SCHEDULES writes; fsync every
+# landed file instead (media-level F_FULLFSYNC deliberately not used —
+# the Linux side does not pay media flush either).
+fsync_tree_ms() {
+    python3 - "$1" <<'PYEOF'
+import os, sys, time
+t = time.monotonic()
+for root, dirs, files in os.walk(sys.argv[1]):
+    for name in files:
+        fd = os.open(os.path.join(root, name), os.O_RDONLY)
+        os.fsync(fd)
+        os.close(fd)
+print(int((time.monotonic() - t) * 1000))
+PYEOF
+}
+
+arm_blit()   { case "$1" in old) echo "$OLD_BLIT";;   new) echo "$NEW_BLIT";;   esac; }
+arm_daemon() { case "$1" in old) echo "$OLD_DAEMON";; new) echo "$NEW_DAEMON";; esac; }
+arm_sha()    { case "$1" in old) echo "$OLD_SHA";;    new) echo "$NEW_SHA";;    esac; }
+
+# --- Preflight ---------------------------------------------------------
+preflight() {
+    [[ -x "$NEW_BLIT" ]] || die "missing $NEW_BLIT (cargo build --release first)"
+    [[ -x "$OLD_BLIT" ]] || die "old client not staged at $OLD_BLIT (rebuild at $OLD_SHA in a detached worktree: git worktree add --detach /tmp/blit-old $OLD_SHA && cargo build --release in it, then copy target/release/blit here)"
+    command -v python3 >/dev/null || die "python3 required (timing + fsync_tree + verdicts)"
+    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
+    sudo -n /usr/sbin/purge || die "cold-cache purge needs a NOPASSWD sudoers rule for /usr/sbin/purge"
+    zssh "test -x '$OLD_DAEMON'" || die "old daemon not staged at $OLD_DAEMON"
+    zssh "test -x '$NEW_DAEMON'" || die "new daemon not staged at $NEW_DAEMON (zigbuild aarch64-musl at $NEW_SHA, stage BESIDE the old one)"
+    # Stale-daemon refusal (the otp-2w F2 posture, new on this rig): a
+    # leftover daemon would mask a bind failure and get benchmarked in
+    # place of the arm's build.
+    if zssh "pgrep blit-daemon >/dev/null 2>&1"; then
+        die "a blit-daemon is already running on zoey — stop it first"
+    fi
+    if [[ -n $(git -C "$REPO_ROOT" status --porcelain) ]]; then
+        log "WARNING: working tree DIRTY — the new client's build id is <sha>.dirty.* and the D-2026-07-05-2 handshake will refuse a clean-built daemon; the new-pair smoke will fail"
+    fi
+    log "preflight OK  old pair: $OLD_SHA  new pair: $NEW_SHA  runs/arm: $RUNS"
+}
+
+write_manifest() {   # binary provenance for the evidence README (design D6)
+    {
+        echo "arm,role,sha,sha256,path"
+        echo "old,client,$OLD_SHA,$(shasum -a 256 "$OLD_BLIT" | cut -d' ' -f1),$OLD_BLIT"
+        echo "new,client,$NEW_SHA,$(shasum -a 256 "$NEW_BLIT" | cut -d' ' -f1),$NEW_BLIT"
+        echo "old,daemon,$OLD_SHA,$(zssh "sha256sum '$OLD_DAEMON'" | cut -d' ' -f1),$OLD_DAEMON"
+        echo "new,daemon,$NEW_SHA,$(zssh "sha256sum '$NEW_DAEMON'" | cut -d' ' -f1),$NEW_DAEMON"
+    } > "$OUT_DIR/staging-manifest.txt"
+    log "staging manifest recorded"
+}
+
+# --- Daemon lifecycle (everything inside ZOEY_TEMP; one arm at a time) --
+CURRENT_ARM=""
+start_daemon() {   # $1 = arm
+    local arm="$1" bin
+    bin=$(arm_daemon "$arm")
+    zssh "mkdir -p '$MODULE_ROOT' && cat > '$ZOEY_TEMP/bench-config.toml' <<EOF
+[daemon]
+bind = \"0.0.0.0\"
+port = $PORT
+no_mdns = true
+
+[[module]]
+name = \"bench\"
+path = \"$MODULE_ROOT\"
+EOF
+nohup '$bin' --config '$ZOEY_TEMP/bench-config.toml' \
+  > '$ZOEY_TEMP/bench-daemon.log' 2>&1 &
+echo \$! > '$ZOEY_TEMP/bench-daemon.pid'"
+    sleep 1
+    zssh "kill -0 \$(cat '$ZOEY_TEMP/bench-daemon.pid')" \
+        || { zssh "cat '$ZOEY_TEMP/bench-daemon.log'"; die "$arm daemon failed to start"; }
+    CURRENT_ARM="$arm"
+    log "daemon up ($arm pair, $(arm_sha "$arm")) on $ZOEY_HOST:$PORT"
+}
+stop_daemon() {
+    zssh "kill \$(cat '$ZOEY_TEMP/bench-daemon.pid' 2>/dev/null) 2>/dev/null; \
+          rm -f '$ZOEY_TEMP/bench-daemon.pid'" || true
+    CURRENT_ARM=""
+}
+ensure_daemon() {   # $1 = arm; swap only when the arm changes (untimed)
+    [[ "$CURRENT_ARM" == "$1" ]] && return 0
+    [[ -n "$CURRENT_ARM" ]] && stop_daemon
+    start_daemon "$1"
+}
+# Sweep this invocation's push destinations even on an interrupted run —
+# never leave content a rerun could no-op onto. Staged pull sources are
+# kept for re-runs by design (shared across arms, design D5).
+sweep_push_dirs() {
+    zssh "cd '$MODULE_ROOT' 2>/dev/null && rm -rf push_${SESSION_TAG}_*" || true
+}
+
+# --- Pool drain + cold caches, both ends -------------------------------
+# Order matters: FIRST flush dirty pages (sync — Linux sync waits), THEN
+# wait for the tier to destage until quiet (three consecutive 2s windows
+# under 2 MiB written; timeout 240s), then cold the caches. An undrained
+# window VOIDS the pair (design F7) — recorded, never silent.
+drain_pool() {
+    zssh 'sync
+quiet=0
+for i in $(seq 1 120); do
+  a=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
+  sleep 2
+  b=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
+  if [ $((b-a)) -lt 4096 ]; then quiet=$((quiet+1)); else quiet=0; fi
+  if [ $quiet -ge 3 ]; then echo "drained ${i}x2s"; exit 0; fi
+done
+echo "DRAIN-TIMEOUT"'
+}
+
+RUN_DRAIN=""
+drop_caches() {   # $1 = run label; sets RUN_DRAIN
+    local outcome
+    outcome=$(drain_pool || true)
+    RUN_DRAIN=${outcome:-DRAIN-ERROR}
+    RUN_DRAIN=${RUN_DRAIN// /_}
+    echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
+    [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
+    sync
+    sudo -n /usr/sbin/purge
+    zssh "echo 3 > /proc/sys/vm/drop_caches"
+}
+
+# --- Fixtures (client disk; generated once; shapes = otp-2/sf-1) -------
+gen_fixtures() {
+    if [[ ! -d "$MAC_WORK/src_large" ]]; then
+        mkdir -p "$MAC_WORK/src_large"
+        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
+        log "generated large fixture (1 GiB)"
+    fi
+    if [[ ! -d "$MAC_WORK/src_small" ]]; then
+        mkdir -p "$MAC_WORK/src_small"
+        for i in $(seq 1 10000); do
+            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
+            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
+        done
+        log "generated small fixture (10000 x 4 KiB)"
+    fi
+    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
+        mkdir -p "$MAC_WORK/src_mixed"
+        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
+        for i in $(seq 1 5000); do
+            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
+            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
+        done
+        log "generated mixed fixture (512 MiB + 5000 x 2 KiB)"
+    fi
+}
+
+# --- Smoke + staging ----------------------------------------------------
+smoke_pair() {   # $1 = arm; 1-file untimed transfer proves the pair works.
+    # For the NEW pair this is also the build-identity check: a
+    # mismatched pair refuses with BUILD_MISMATCH at the first frame
+    # (D-2026-07-05-2). The OLD pair has no handshake — its identity is
+    # the staging manifest.
+    local arm="$1" blit
+    blit=$(arm_blit "$arm")
+    ensure_daemon "$arm"
+    mkdir -p "$MAC_WORK/smoke_src"
+    echo "otp12-smoke" > "$MAC_WORK/smoke_src/probe.txt"
+    "$blit" copy "$MAC_WORK/smoke_src" "${REMOTE}push_${SESSION_TAG}_smoke_${arm}/" --yes \
+        > "$OUT_DIR/blit-logs/smoke_$arm.log" 2>&1 \
+        || die "smoke transfer FAILED for the $arm pair (blit-logs/smoke_$arm.log; on the new pair a BUILD_MISMATCH means the staged daemon is not $NEW_SHA)"
+    log "smoke ok: $arm pair"
+}
+
+stage_pull_sources() {
+    # Untimed; sources are SHARED across arms by design (bytes are
+    # bytes — design D5); kept across sessions, staged only if absent.
+    log "staging pull sources (untimed, new pair)"
+    ensure_daemon new
+    local w
+    for w in large small mixed; do
+        if zssh "test -d '$MODULE_ROOT/pull_src_$w/src_$w'"; then
+            log "  pull_src_$w already staged (kept from a prior session)"
+        else
+            "$NEW_BLIT" copy "$MAC_WORK/src_$w" "${REMOTE}pull_src_$w/" --yes \
+                > "$OUT_DIR/blit-logs/stage_$w.log" 2>&1 \
+                || die "staging pull_src_$w failed (blit-logs/stage_$w.log)"
+            log "  staged pull_src_$w"
+        fi
+    done
+}
+
+# --- Timed runs ---------------------------------------------------------
+CSV="$OUT_DIR/runs.csv"
+echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
+META="$OUT_DIR/meta.csv"
+echo "cell,pairs_attempted,complete" > "$META"
+
+RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
+
+timed_push_run() {   # arm cell rid src [flags...]; fresh dest per run
+    local arm="$1" cell="$2" rid="$3" src="$4"; shift 4
+    local blit start end rc=0
+    blit=$(arm_blit "$arm")
+    ensure_daemon "$arm"
+    drop_caches "${cell}_${arm}-$rid"
+    start=$(now_ms)
+    "$blit" copy "$src" "${REMOTE}push_${SESSION_TAG}_${cell}_${arm}_${rid}/" --yes "$@" \
+        > "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.log" 2>&1 || rc=$?
+    end=$(now_ms)
+    RUN_FLUSH=$(sync_dest_ms)   # durable at dest, self-timed
+    RUN_MS=$(( end - start + RUN_FLUSH ))
+    RUN_EXIT=$rc
+    RUN_VALID=yes
+    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
+}
+
+timed_pull_run() {   # arm cell rid remote_src [flags...]; fresh dest per run
+    local arm="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
+    local blit start end rc=0
+    blit=$(arm_blit "$arm")
+    ensure_daemon "$arm"
+    rm -rf "$MAC_WORK/dst_pull"
+    mkdir -p "$MAC_WORK/dst_pull"
+    drop_caches "${cell}_${arm}-$rid"
+    start=$(now_ms)
+    "$blit" copy "$rsrc" "$MAC_WORK/dst_pull" --yes "$@" \
+        > "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.log" 2>&1 || rc=$?
+    end=$(now_ms)
+    RUN_FLUSH=$(fsync_tree_ms "$MAC_WORK/dst_pull")   # durable, self-timed
+    RUN_MS=$(( end - start + RUN_FLUSH ))
+    RUN_EXIT=$rc
+    RUN_VALID=yes
+    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
+}
+
+run_comparison() {   # cell kind src_or_remote [flags...]
+    local cell="$1" kind="$2" src="$3"; shift 3
+    local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
+    log "=== $cell (interleaved old/new, ABBA, $RUNS pairs) ==="
+    while (( valid < RUNS && attempts < max_attempts )); do
+        attempts=$(( attempts + 1 ))
+        # ABBA: odd slots run old first, even slots run new first.
+        local order pair_valid=yes arm rid
+        if (( slot % 2 )); then order="old new"; else order="new old"; fi
+        local row_old="" row_new=""
+        for arm in $order; do
+            rid="s${slot}a${attempts}"
+            if [[ "$kind" == push ]]; then
+                timed_push_run "$arm" "$cell" "$rid" "$src" "$@"
+            else
+                timed_pull_run "$arm" "$cell" "$rid" "$src" "$@"
+            fi
+            [[ "$RUN_VALID" == yes ]] || pair_valid=no
+            local row="$cell,$arm,$(arm_sha "$arm"),mac,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
+            if [[ "$arm" == old ]]; then row_old="$row"; else row_new="$row"; fi
+            log "  $cell/$arm slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
+        done
+        # The valid column reflects the PAIR's fate (design F7): an
+        # individually-clean run whose partner voided does not count.
+        echo "$row_old,$pair_valid" >> "$CSV"
+        echo "$row_new,$pair_valid" >> "$CSV"
+        if [[ "$pair_valid" == yes ]]; then
+            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
+        else
+            log "  $cell: pair at slot $slot VOIDED — re-running the slot"
+        fi
+    done
+    if (( valid < RUNS )); then
+        echo "$cell,$attempts,no" >> "$META"
+        log "  $cell INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
+    else
+        echo "$cell,$attempts,yes" >> "$META"
+    fi
+}
+
+# --- Verdicts (design D2: both references must pass) --------------------
+compute_verdicts() {
+    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
+import csv, sys
+runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
+rows = list(csv.DictReader(open(runs_p)))
+meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
+base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}
+
+by_arm = {}
+voided = {}
+for r in rows:
+    key = (r["cell"], r["arm"])
+    if r["valid"] == "yes":
+        by_arm.setdefault(key, []).append(int(r["ms"]))
+    else:
+        voided[key] = voided.get(key, 0) + 1
+
+def median(v):
+    v = sorted(v)
+    n = len(v)
+    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
+
+with open(summary_p, "w") as f:
+    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
+    for (cell, arm) in sorted(by_arm):
+        v = by_arm[(cell, arm)]
+        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
+        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
+                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")
+
+def bar_pass(new, ref):   # new <= ref * 1.10, integer-exact
+    return 10 * new <= 11 * ref
+
+with open(verdicts_p, "w") as f:
+    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
+    for cell in sorted({c for (c, _) in by_arm}):
+        if meta[cell]["complete"] != "yes" or (cell, "new") not in by_arm or (cell, "old") not in by_arm:
+            f.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
+            continue
+        new_m = median(by_arm[(cell, "new")])
+        old_m = median(by_arm[(cell, "old")])
+        ref_m = base.get(cell)
+        p1 = bar_pass(new_m, old_m)
+        f.write(f"{cell},converge,new,old_session,{new_m},{old_m},"
+                f"{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
+        if ref_m is None:
+            f.write(f"{cell},converge,new,old_committed,{new_m},,,1.10,NO-REFERENCE\n")
+            f.write(f"{cell},converge,new,combined,{new_m},{old_m},,1.10,"
+                    f"{'FAIL-SAME-SESSION' if not p1 else 'NO-REFERENCE'}\n")
+            continue
+        p2 = bar_pass(new_m, ref_m)
+        f.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},"
+                f"{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
+        combined = ("PASS" if p1 and p2
+                    else "FAIL-REFERENCE-DRIFT" if p1
+                    else "FAIL-SAME-SESSION" if p2
+                    else "FAIL-BOTH")
+        f.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
+PYEOF
+}
+
+# --- Matrix -------------------------------------------------------------
+main() {
+    preflight
+    write_manifest
+    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
+        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
+        exit 0
+    fi
+    BUILD_LINE="old=$OLD_SHA new=$NEW_SHA"
+    log "session $SESSION_TAG  $BUILD_LINE  client: $(uname -m) macOS  daemon host: $ZOEY_HOST"
+
+    gen_fixtures
+    smoke_pair old
+    smoke_pair new
+    stage_pull_sources
+
+    local w
+    for w in large small mixed; do
+        run_comparison "push_tcp_${w}"  push "$MAC_WORK/src_$w"
+        run_comparison "push_grpc_${w}" push "$MAC_WORK/src_$w" --force-grpc
+        run_comparison "pull_tcp_${w}"  pull "${REMOTE}pull_src_$w/src_$w/"
+        run_comparison "pull_grpc_${w}" pull "${REMOTE}pull_src_$w/src_$w/" --force-grpc
+    done
+
+    stop_daemon
+    compute_verdicts
+
+    log ""
+    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
+    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
+    log ""
+    log "=== VERDICTS (design D2: PASS needs BOTH references) ==="
+    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
+    log "runs: $CSV"
+}
+
+SESSION_TAG=$(date +%H%M%S).$$
+trap 'stop_daemon; sweep_push_dirs' EXIT
+main "$@"

exec
/bin/zsh -lc 'git status --short --untracked-files=all && git remote -v && git rev-parse HEAD' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-qhGgFQaG' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rPmIqyRL' (errno=Operation not permitted)
?? .review/results/otp-12a.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZK9auijr' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LGyGcngF' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-wDUVgtej' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-gOAEjZZm' (errno=Operation not permitted)
8f4fbf9a7cacacdf22e6f57530cef2e91fb7351c

codex
I’ve split the review into independent design, shell, and methodology passes. I’m now anchoring line numbers to the committed tree and checking the active plan/current-state evidence so findings aren’t based on the working tree by accident.
exec
/bin/zsh -lc 'git show 8f4fbf9:scripts/bench_otp12_zoey.sh | nl -ba' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-g17Lsf2v' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-zU5ia7mP' (errno=Operation not permitted)
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
    53	#   * OLD pair: Mac client rebuilt at $OLD_SHA in a detached worktree
    54	#     and staged at $MAC_WORK/bins/blit-$OLD_SHA; zoey's pinned old
    55	#     daemon at $ZOEY_TEMP/blit-daemon (.agents/machines.md staging,
    56	#     kept for otp-12).
    57	#   * The OLD pair predates the handshake: its provenance is the
    58	#     staging record — this script records sha256 of every binary into
    59	#     staging-manifest.txt. The NEW pair's smoke transfer doubles as
    60	#     its identity check (a mismatched pair refuses with
    61	#     BUILD_MISMATCH at the first frame).
    62	#   * python3 + a NOPASSWD sudoers rule for /usr/sbin/purge on the Mac.
    63	#   * A RIG RUN needs the owner's fresh go for daemon runs on zoey
    64	#     (standing STATE rule). PREFLIGHT_ONLY=1 starts no daemon and
    65	#     times nothing (read-only ssh checks + local purge probe).
    66	#
    67	# Everything on the daemon host stays inside $ZOEY_TEMP (owner rule).
    68	
    69	set -euo pipefail
    70	
    71	SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    72	REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
    73	
    74	ZOEY_SSH=${ZOEY_SSH:-root@zoey}
    75	ZOEY_TEMP=${ZOEY_TEMP:?set ZOEY_TEMP to the blit-temp folder on the daemon host}
    76	ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
    77	PORT=${PORT:-9031}
    78	RUNS=${RUNS:-4}
    79	PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
    80	# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
    81	MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
    82	
    83	OLD_SHA=${OLD_SHA_ZOEY:-e757dcc}
    84	NEW_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
    85	NEW_BLIT=${NEW_BLIT:-$REPO_ROOT/target/release/blit}
    86	OLD_BLIT=${OLD_BLIT:-$MAC_WORK/bins/blit-$OLD_SHA}
    87	OLD_DAEMON=${OLD_DAEMON:-$ZOEY_TEMP/blit-daemon}
    88	NEW_DAEMON=${NEW_DAEMON:-$ZOEY_TEMP/blit-daemon-$NEW_SHA}
    89	BASELINE_SUMMARY=${BASELINE_SUMMARY:-$REPO_ROOT/docs/bench/otp2-baseline-2026-07-10/summary.csv}
    90	
    91	OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_zoey_$(date +%Y%m%dT%H%M%S)}
    92	mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs" "$MAC_WORK"
    93	
    94	MODULE_ROOT="$ZOEY_TEMP/bench-module"
    95	REMOTE="$ZOEY_HOST:$PORT:/bench/"
    96	
    97	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
    98	die() { log "FATAL: $*"; exit 1; }
    99	# ControlMaster multiplexing: an ssh connection to this host costs
   100	# ~1.2s (slow-core key exchange) — reuse one connection.
   101	SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
   102	zssh() { ssh "${SSH_MUX[@]}" "$ZOEY_SSH" "$@"; }
   103	# Wall-clock ms across two separate python3 processes (deliberate; see
   104	# bench_otp2_baseline.sh for why monotonic is wrong here).
   105	now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
   106	# Self-timed durability steps (codex otp-2w F3): the timed window is
   107	# transfer + destination flush and NOTHING else; each flush times
   108	# ITSELF on the destination and reports only its own duration.
   109	sync_dest_ms() {   # Linux sync on the daemon host; prints its elapsed ms
   110	    zssh 'a=$(awk "{print int(\$1*1000)}" /proc/uptime); sync; b=$(awk "{print int(\$1*1000)}" /proc/uptime); echo $((b-a))'
   111	}
   112	# Durable pull window: macOS sync(2) only SCHEDULES writes; fsync every
   113	# landed file instead (media-level F_FULLFSYNC deliberately not used —
   114	# the Linux side does not pay media flush either).
   115	fsync_tree_ms() {
   116	    python3 - "$1" <<'PYEOF'
   117	import os, sys, time
   118	t = time.monotonic()
   119	for root, dirs, files in os.walk(sys.argv[1]):
   120	    for name in files:
   121	        fd = os.open(os.path.join(root, name), os.O_RDONLY)
   122	        os.fsync(fd)
   123	        os.close(fd)
   124	print(int((time.monotonic() - t) * 1000))
   125	PYEOF
   126	}
   127	
   128	arm_blit()   { case "$1" in old) echo "$OLD_BLIT";;   new) echo "$NEW_BLIT";;   esac; }
   129	arm_daemon() { case "$1" in old) echo "$OLD_DAEMON";; new) echo "$NEW_DAEMON";; esac; }
   130	arm_sha()    { case "$1" in old) echo "$OLD_SHA";;    new) echo "$NEW_SHA";;    esac; }
   131	
   132	# --- Preflight ---------------------------------------------------------
   133	preflight() {
   134	    [[ -x "$NEW_BLIT" ]] || die "missing $NEW_BLIT (cargo build --release first)"
   135	    [[ -x "$OLD_BLIT" ]] || die "old client not staged at $OLD_BLIT (rebuild at $OLD_SHA in a detached worktree: git worktree add --detach /tmp/blit-old $OLD_SHA && cargo build --release in it, then copy target/release/blit here)"
   136	    command -v python3 >/dev/null || die "python3 required (timing + fsync_tree + verdicts)"
   137	    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
   138	    sudo -n /usr/sbin/purge || die "cold-cache purge needs a NOPASSWD sudoers rule for /usr/sbin/purge"
   139	    zssh "test -x '$OLD_DAEMON'" || die "old daemon not staged at $OLD_DAEMON"
   140	    zssh "test -x '$NEW_DAEMON'" || die "new daemon not staged at $NEW_DAEMON (zigbuild aarch64-musl at $NEW_SHA, stage BESIDE the old one)"
   141	    # Stale-daemon refusal (the otp-2w F2 posture, new on this rig): a
   142	    # leftover daemon would mask a bind failure and get benchmarked in
   143	    # place of the arm's build.
   144	    if zssh "pgrep blit-daemon >/dev/null 2>&1"; then
   145	        die "a blit-daemon is already running on zoey — stop it first"
   146	    fi
   147	    if [[ -n $(git -C "$REPO_ROOT" status --porcelain) ]]; then
   148	        log "WARNING: working tree DIRTY — the new client's build id is <sha>.dirty.* and the D-2026-07-05-2 handshake will refuse a clean-built daemon; the new-pair smoke will fail"
   149	    fi
   150	    log "preflight OK  old pair: $OLD_SHA  new pair: $NEW_SHA  runs/arm: $RUNS"
   151	}
   152	
   153	write_manifest() {   # binary provenance for the evidence README (design D6)
   154	    {
   155	        echo "arm,role,sha,sha256,path"
   156	        echo "old,client,$OLD_SHA,$(shasum -a 256 "$OLD_BLIT" | cut -d' ' -f1),$OLD_BLIT"
   157	        echo "new,client,$NEW_SHA,$(shasum -a 256 "$NEW_BLIT" | cut -d' ' -f1),$NEW_BLIT"
   158	        echo "old,daemon,$OLD_SHA,$(zssh "sha256sum '$OLD_DAEMON'" | cut -d' ' -f1),$OLD_DAEMON"
   159	        echo "new,daemon,$NEW_SHA,$(zssh "sha256sum '$NEW_DAEMON'" | cut -d' ' -f1),$NEW_DAEMON"
   160	    } > "$OUT_DIR/staging-manifest.txt"
   161	    log "staging manifest recorded"
   162	}
   163	
   164	# --- Daemon lifecycle (everything inside ZOEY_TEMP; one arm at a time) --
   165	CURRENT_ARM=""
   166	start_daemon() {   # $1 = arm
   167	    local arm="$1" bin
   168	    bin=$(arm_daemon "$arm")
   169	    zssh "mkdir -p '$MODULE_ROOT' && cat > '$ZOEY_TEMP/bench-config.toml' <<EOF
   170	[daemon]
   171	bind = \"0.0.0.0\"
   172	port = $PORT
   173	no_mdns = true
   174	
   175	[[module]]
   176	name = \"bench\"
   177	path = \"$MODULE_ROOT\"
   178	EOF
   179	nohup '$bin' --config '$ZOEY_TEMP/bench-config.toml' \
   180	  > '$ZOEY_TEMP/bench-daemon.log' 2>&1 &
   181	echo \$! > '$ZOEY_TEMP/bench-daemon.pid'"
   182	    sleep 1
   183	    zssh "kill -0 \$(cat '$ZOEY_TEMP/bench-daemon.pid')" \
   184	        || { zssh "cat '$ZOEY_TEMP/bench-daemon.log'"; die "$arm daemon failed to start"; }
   185	    CURRENT_ARM="$arm"
   186	    log "daemon up ($arm pair, $(arm_sha "$arm")) on $ZOEY_HOST:$PORT"
   187	}
   188	stop_daemon() {
   189	    zssh "kill \$(cat '$ZOEY_TEMP/bench-daemon.pid' 2>/dev/null) 2>/dev/null; \
   190	          rm -f '$ZOEY_TEMP/bench-daemon.pid'" || true
   191	    CURRENT_ARM=""
   192	}
   193	ensure_daemon() {   # $1 = arm; swap only when the arm changes (untimed)
   194	    [[ "$CURRENT_ARM" == "$1" ]] && return 0
   195	    [[ -n "$CURRENT_ARM" ]] && stop_daemon
   196	    start_daemon "$1"
   197	}
   198	# Sweep this invocation's push destinations even on an interrupted run —
   199	# never leave content a rerun could no-op onto. Staged pull sources are
   200	# kept for re-runs by design (shared across arms, design D5).
   201	sweep_push_dirs() {
   202	    zssh "cd '$MODULE_ROOT' 2>/dev/null && rm -rf push_${SESSION_TAG}_*" || true
   203	}
   204	
   205	# --- Pool drain + cold caches, both ends -------------------------------
   206	# Order matters: FIRST flush dirty pages (sync — Linux sync waits), THEN
   207	# wait for the tier to destage until quiet (three consecutive 2s windows
   208	# under 2 MiB written; timeout 240s), then cold the caches. An undrained
   209	# window VOIDS the pair (design F7) — recorded, never silent.
   210	drain_pool() {
   211	    zssh 'sync
   212	quiet=0
   213	for i in $(seq 1 120); do
   214	  a=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
   215	  sleep 2
   216	  b=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
   217	  if [ $((b-a)) -lt 4096 ]; then quiet=$((quiet+1)); else quiet=0; fi
   218	  if [ $quiet -ge 3 ]; then echo "drained ${i}x2s"; exit 0; fi
   219	done
   220	echo "DRAIN-TIMEOUT"'
   221	}
   222	
   223	RUN_DRAIN=""
   224	drop_caches() {   # $1 = run label; sets RUN_DRAIN
   225	    local outcome
   226	    outcome=$(drain_pool || true)
   227	    RUN_DRAIN=${outcome:-DRAIN-ERROR}
   228	    RUN_DRAIN=${RUN_DRAIN// /_}
   229	    echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
   230	    [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
   231	    sync
   232	    sudo -n /usr/sbin/purge
   233	    zssh "echo 3 > /proc/sys/vm/drop_caches"
   234	}
   235	
   236	# --- Fixtures (client disk; generated once; shapes = otp-2/sf-1) -------
   237	gen_fixtures() {
   238	    if [[ ! -d "$MAC_WORK/src_large" ]]; then
   239	        mkdir -p "$MAC_WORK/src_large"
   240	        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
   241	        log "generated large fixture (1 GiB)"
   242	    fi
   243	    if [[ ! -d "$MAC_WORK/src_small" ]]; then
   244	        mkdir -p "$MAC_WORK/src_small"
   245	        for i in $(seq 1 10000); do
   246	            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
   247	            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
   248	        done
   249	        log "generated small fixture (10000 x 4 KiB)"
   250	    fi
   251	    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
   252	        mkdir -p "$MAC_WORK/src_mixed"
   253	        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
   254	        for i in $(seq 1 5000); do
   255	            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
   256	            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
   257	        done
   258	        log "generated mixed fixture (512 MiB + 5000 x 2 KiB)"
   259	    fi
   260	}
   261	
   262	# --- Smoke + staging ----------------------------------------------------
   263	smoke_pair() {   # $1 = arm; 1-file untimed transfer proves the pair works.
   264	    # For the NEW pair this is also the build-identity check: a
   265	    # mismatched pair refuses with BUILD_MISMATCH at the first frame
   266	    # (D-2026-07-05-2). The OLD pair has no handshake — its identity is
   267	    # the staging manifest.
   268	    local arm="$1" blit
   269	    blit=$(arm_blit "$arm")
   270	    ensure_daemon "$arm"
   271	    mkdir -p "$MAC_WORK/smoke_src"
   272	    echo "otp12-smoke" > "$MAC_WORK/smoke_src/probe.txt"
   273	    "$blit" copy "$MAC_WORK/smoke_src" "${REMOTE}push_${SESSION_TAG}_smoke_${arm}/" --yes \
   274	        > "$OUT_DIR/blit-logs/smoke_$arm.log" 2>&1 \
   275	        || die "smoke transfer FAILED for the $arm pair (blit-logs/smoke_$arm.log; on the new pair a BUILD_MISMATCH means the staged daemon is not $NEW_SHA)"
   276	    log "smoke ok: $arm pair"
   277	}
   278	
   279	stage_pull_sources() {
   280	    # Untimed; sources are SHARED across arms by design (bytes are
   281	    # bytes — design D5); kept across sessions, staged only if absent.
   282	    log "staging pull sources (untimed, new pair)"
   283	    ensure_daemon new
   284	    local w
   285	    for w in large small mixed; do
   286	        if zssh "test -d '$MODULE_ROOT/pull_src_$w/src_$w'"; then
   287	            log "  pull_src_$w already staged (kept from a prior session)"
   288	        else
   289	            "$NEW_BLIT" copy "$MAC_WORK/src_$w" "${REMOTE}pull_src_$w/" --yes \
   290	                > "$OUT_DIR/blit-logs/stage_$w.log" 2>&1 \
   291	                || die "staging pull_src_$w failed (blit-logs/stage_$w.log)"
   292	            log "  staged pull_src_$w"
   293	        fi
   294	    done
   295	}
   296	
   297	# --- Timed runs ---------------------------------------------------------
   298	CSV="$OUT_DIR/runs.csv"
   299	echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
   300	META="$OUT_DIR/meta.csv"
   301	echo "cell,pairs_attempted,complete" > "$META"
   302	
   303	RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
   304	
   305	timed_push_run() {   # arm cell rid src [flags...]; fresh dest per run
   306	    local arm="$1" cell="$2" rid="$3" src="$4"; shift 4
   307	    local blit start end rc=0
   308	    blit=$(arm_blit "$arm")
   309	    ensure_daemon "$arm"
   310	    drop_caches "${cell}_${arm}-$rid"
   311	    start=$(now_ms)
   312	    "$blit" copy "$src" "${REMOTE}push_${SESSION_TAG}_${cell}_${arm}_${rid}/" --yes "$@" \
   313	        > "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.log" 2>&1 || rc=$?
   314	    end=$(now_ms)
   315	    RUN_FLUSH=$(sync_dest_ms)   # durable at dest, self-timed
   316	    RUN_MS=$(( end - start + RUN_FLUSH ))
   317	    RUN_EXIT=$rc
   318	    RUN_VALID=yes
   319	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   320	}
   321	
   322	timed_pull_run() {   # arm cell rid remote_src [flags...]; fresh dest per run
   323	    local arm="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
   324	    local blit start end rc=0
   325	    blit=$(arm_blit "$arm")
   326	    ensure_daemon "$arm"
   327	    rm -rf "$MAC_WORK/dst_pull"
   328	    mkdir -p "$MAC_WORK/dst_pull"
   329	    drop_caches "${cell}_${arm}-$rid"
   330	    start=$(now_ms)
   331	    "$blit" copy "$rsrc" "$MAC_WORK/dst_pull" --yes "$@" \
   332	        > "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.log" 2>&1 || rc=$?
   333	    end=$(now_ms)
   334	    RUN_FLUSH=$(fsync_tree_ms "$MAC_WORK/dst_pull")   # durable, self-timed
   335	    RUN_MS=$(( end - start + RUN_FLUSH ))
   336	    RUN_EXIT=$rc
   337	    RUN_VALID=yes
   338	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   339	}
   340	
   341	run_comparison() {   # cell kind src_or_remote [flags...]
   342	    local cell="$1" kind="$2" src="$3"; shift 3
   343	    local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
   344	    log "=== $cell (interleaved old/new, ABBA, $RUNS pairs) ==="
   345	    while (( valid < RUNS && attempts < max_attempts )); do
   346	        attempts=$(( attempts + 1 ))
   347	        # ABBA: odd slots run old first, even slots run new first.
   348	        local order pair_valid=yes arm rid
   349	        if (( slot % 2 )); then order="old new"; else order="new old"; fi
   350	        local row_old="" row_new=""
   351	        for arm in $order; do
   352	            rid="s${slot}a${attempts}"
   353	            if [[ "$kind" == push ]]; then
   354	                timed_push_run "$arm" "$cell" "$rid" "$src" "$@"
   355	            else
   356	                timed_pull_run "$arm" "$cell" "$rid" "$src" "$@"
   357	            fi
   358	            [[ "$RUN_VALID" == yes ]] || pair_valid=no
   359	            local row="$cell,$arm,$(arm_sha "$arm"),mac,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
   360	            if [[ "$arm" == old ]]; then row_old="$row"; else row_new="$row"; fi
   361	            log "  $cell/$arm slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
   362	        done
   363	        # The valid column reflects the PAIR's fate (design F7): an
   364	        # individually-clean run whose partner voided does not count.
   365	        echo "$row_old,$pair_valid" >> "$CSV"
   366	        echo "$row_new,$pair_valid" >> "$CSV"
   367	        if [[ "$pair_valid" == yes ]]; then
   368	            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   369	        else
   370	            log "  $cell: pair at slot $slot VOIDED — re-running the slot"
   371	        fi
   372	    done
   373	    if (( valid < RUNS )); then
   374	        echo "$cell,$attempts,no" >> "$META"
   375	        log "  $cell INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
   376	    else
   377	        echo "$cell,$attempts,yes" >> "$META"
   378	    fi
   379	}
   380	
   381	# --- Verdicts (design D2: both references must pass) --------------------
   382	compute_verdicts() {
   383	    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
   384	import csv, sys
   385	runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
   386	rows = list(csv.DictReader(open(runs_p)))
   387	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   388	base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}
   389	
   390	by_arm = {}
   391	voided = {}
   392	for r in rows:
   393	    key = (r["cell"], r["arm"])
   394	    if r["valid"] == "yes":
   395	        by_arm.setdefault(key, []).append(int(r["ms"]))
   396	    else:
   397	        voided[key] = voided.get(key, 0) + 1
   398	
   399	def median(v):
   400	    v = sorted(v)
   401	    n = len(v)
   402	    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
   403	
   404	with open(summary_p, "w") as f:
   405	    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
   406	    for (cell, arm) in sorted(by_arm):
   407	        v = by_arm[(cell, arm)]
   408	        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   409	        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
   410	                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")
   411	
   412	def bar_pass(new, ref):   # new <= ref * 1.10, integer-exact
   413	    return 10 * new <= 11 * ref
   414	
   415	with open(verdicts_p, "w") as f:
   416	    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
   417	    for cell in sorted({c for (c, _) in by_arm}):
   418	        if meta[cell]["complete"] != "yes" or (cell, "new") not in by_arm or (cell, "old") not in by_arm:
   419	            f.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
   420	            continue
   421	        new_m = median(by_arm[(cell, "new")])
   422	        old_m = median(by_arm[(cell, "old")])
   423	        ref_m = base.get(cell)
   424	        p1 = bar_pass(new_m, old_m)
   425	        f.write(f"{cell},converge,new,old_session,{new_m},{old_m},"
   426	                f"{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
   427	        if ref_m is None:
   428	            f.write(f"{cell},converge,new,old_committed,{new_m},,,1.10,NO-REFERENCE\n")
   429	            f.write(f"{cell},converge,new,combined,{new_m},{old_m},,1.10,"
   430	                    f"{'FAIL-SAME-SESSION' if not p1 else 'NO-REFERENCE'}\n")
   431	            continue
   432	        p2 = bar_pass(new_m, ref_m)
   433	        f.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},"
   434	                f"{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
   435	        combined = ("PASS" if p1 and p2
   436	                    else "FAIL-REFERENCE-DRIFT" if p1
   437	                    else "FAIL-SAME-SESSION" if p2
   438	                    else "FAIL-BOTH")
   439	        f.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
   440	PYEOF
   441	}
   442	
   443	# --- Matrix -------------------------------------------------------------
   444	main() {
   445	    preflight
   446	    write_manifest
   447	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   448	        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   449	        exit 0
   450	    fi
   451	    BUILD_LINE="old=$OLD_SHA new=$NEW_SHA"
   452	    log "session $SESSION_TAG  $BUILD_LINE  client: $(uname -m) macOS  daemon host: $ZOEY_HOST"
   453	
   454	    gen_fixtures
   455	    smoke_pair old
   456	    smoke_pair new
   457	    stage_pull_sources
   458	
   459	    local w
   460	    for w in large small mixed; do
   461	        run_comparison "push_tcp_${w}"  push "$MAC_WORK/src_$w"
   462	        run_comparison "push_grpc_${w}" push "$MAC_WORK/src_$w" --force-grpc
   463	        run_comparison "pull_tcp_${w}"  pull "${REMOTE}pull_src_$w/src_$w/"
   464	        run_comparison "pull_grpc_${w}" pull "${REMOTE}pull_src_$w/src_$w/" --force-grpc
   465	    done
   466	
   467	    stop_daemon
   468	    compute_verdicts
   469	
   470	    log ""
   471	    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
   472	    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
   473	    log ""
   474	    log "=== VERDICTS (design D2: PASS needs BOTH references) ==="
   475	    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
   476	    log "runs: $CSV"
   477	}
   478	
   479	SESSION_TAG=$(date +%H%M%S).$$
   480	trap 'stop_daemon; sweep_push_dirs' EXIT
   481	main "$@"

exec
/bin/zsh -lc 'git show 8f4fbf9:docs/plan/OTP12_ACCEPTANCE_RUN.md | nl -ba' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-tvrigtkR' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-f36Cf0q1' (errno=Operation not permitted)
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
   181	
   182	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
   183	
   184	For a FIXED data direction the two initiators are:
   185	
   186	- **Mac→Windows**: arm A = Mac client pushes
   187	  (`blit copy $MAC_WORK/src_<w> $WIN_HOST:9031:/bench/<fresh>/ --yes`);
   188	  arm B = Windows client pulls
   189	  (`blit.exe copy $MAC_HOST:9031:/bench/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   190	- **Windows→Mac**: arm A = Mac client pulls (staged
   191	  `pull_src_<w>/src_<w>/` source, the otp-2w pattern); arm B = Windows
   192	  client pushes the same staged tree as a local path
   193	  (`blit.exe copy D:\blit-test\bench-module\pull_src_<w>\src_<w> $MAC_HOST:9031:/bench/<fresh>/ --yes`).
   194	
   195	New plumbing this requires, each keyed by ROLE not verb:
   196	
   197	1. **A daemon on the Mac** (new build only): config written like the rig
   198	   scripts do today (`[daemon] bind/port/no_mdns` + `[[module]] name =
   199	   "bench"` pointing at `$MAC_MODULE_ROOT`, **default `$MAC_WORK`
   200	   itself** — the module exports the exact fixture trees arm A pushes,
   201	   so both initiators read the same physical inodes; no fixture copy or
   202	   move on the Mac (codex design F6)), local launch, pid file,
   203	   stale-refusal, PID-scoped teardown. macOS application firewall must
   204	   admit `blit-daemon` — gated by a preflight smoke transfer from
   205	   Windows, not assumed.
   206	2. **A Windows client** (`blit.exe`, new build, built natively alongside
   207	   the daemon). Its timed window is measured ON Windows —
   208	   `[Diagnostics.Stopwatch]` bracketing the `blit.exe copy` inside one ssh
   209	   invocation, output CRLF-stripped (`tr -cd '0-9'`) — the otp-2w
   210	   self-timed pattern (README §Timing-overhead correction); the ssh
   211	   round-trip cost stays outside the window by construction.
   212	3. **Flush keyed by destination OS, never verb**: dest Windows ⇒ self-timed
   213	   `Write-VolumeCache D`; dest macOS ⇒ the local self-timed per-file fsync
   214	   walk. Cold caches both ends before every run (purge / standby-purge);
   215	   drain keyed by the destination disk (Windows `Get-Counter` loop when D:
   216	   receives; the Mac side has no drain equivalent — recorded decision: Mac
   217	   destination runs rely on `sync` + purge exactly as the recorded otp-2w
   218	   pull cells did).
   219	
   220	Arm A cells run fresh inside the invariance block (interleaved A,B,A,B…) —
   221	block-1 new-arm numbers are NOT reused, so rig-state drift between blocks
   222	cannot masquerade as an initiator effect.
   223	
   224	### D4 — delegated cells = delegated-vs-direct parity (rig D)
   225	
   226	Per data direction, the delegated arm and the direct arm drive the SAME
   227	session code with the same roles on the same endpoints; the only deltas are
   228	who spawns the initiator (daemon vs CLI) and the trigger/progress relay:
   229	
   230	- **skippy→Windows**: delegated = Mac runs
   231	  `blit copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ $WIN_HOST:9031:/bench/<fresh>/ --yes`
   232	  (Windows daemon initiates, DESTINATION role); direct = Windows client
   233	  pulls the same source to the same disk
   234	  (`blit.exe copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   235	- **Windows→skippy**: delegated = the mirror-image Mac command (skippy
   236	  daemon initiates); direct = skippy client pulls from the Windows daemon
   237	  (self-timed `/proc/uptime`-bracketed window over ssh, the zoey pattern).
   238	
   239	Timing: the delegated arm is timed on the Mac around the CLI invocation
   240	(the CLI blocks until the relayed Summary), plus the destination's
   241	self-timed flush — deliberately INCLUDING the trigger RPC + relay overhead
   242	(that is the honest end-to-end cost of delegation; on this LAN the trigger
   243	is sub-ms against multi-second cells). The direct arm is self-timed on the
   244	initiating host plus the same flush. Destination flush: Windows ⇒
   245	`Write-VolumeCache`; skippy ⇒ self-timed `sync` bracketed by
   246	`/proc/uptime` reads in one ssh shell. Cold caches: standby-purge (Windows)
   247	+ `drop_caches` (skippy, root/sudo) both ends every run; drain the
   248	destination disk (Windows counter loop; skippy `/proc/diskstats` quiet-
   249	window loop with a device-regex knob).
   250	
   251	Carrier: TCP is the verdict carrier; one secondary grpc pair
   252	(large × skippy→Windows, both arms) is recorded as a smoke row — carrier
   253	selection reads `SessionOpen.in_stream_bytes`/policy, never role or
   254	initiator (`transfer_session/mod.rs:790,805`), and carrier invariance is
   255	measured properly on rig W.
   256	
   257	Config: BOTH daemons get `[delegation] allow_delegated_pull = true` with
   258	`allowed_source_hosts` naming the peer (each is destination in one
   259	direction); bench modules writable, `delegation_allowed` not narrowed.
   260	
   261	### D5 — three self-contained scripts; the frozen baselines stay frozen
   262	
   263	`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
   264	`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
   265	precedent: duplicate the shape, don't refactor recorded evidence;
   266	`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
   267	old scripts, both recorded sharp edges:
   268	
   269	- **Exit codes are checked**: the old harnesses swallow the blit exit code
   270	  inside the timed window; otp-12 records it per run (`exit` column) and a
   271	  nonzero exit voids the interleave pair per the D2 valid-run rule — a
   272	  failed transfer must never contribute a time.
   273	- **Multi-token flags ride an array**, not an unquoted scalar.
   274	
   275	CSV schema (all rigs):
   276	`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
   277	(`valid` = the PAIR's fate under the D2 valid-run rule — an
   278	individually-clean run whose partner voided reads `no`; amended at the
   279	12a harness slice)
   280	`summary.csv`:
   281	`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
   282	(medians over valid runs only — the D2 valid-run rule)
   283	`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
   284	where `cell` = `<fixture>_<direction>_<carrier>`, `arm` ∈
   285	`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
   286	`initiator` = host name, `kind` ∈ `converge|invariance|delegated|cross`.
   287	
   288	Fixtures: identical shapes to otp-2 (1 GiB large / 10k×4 KiB small /
   289	512 MiB+5k×2 KiB mixed), generated with the existing recipes (BSD vs GNU
   290	`dd` block-size spelling handled per host), staged untimed; pull sources
   291	shared across arms (bytes are bytes — recorded explicitly); every timed
   292	destination is fresh and never-seen (`SESSION_TAG` + arm + run in the
   293	path).
   294	
   295	New env knobs: `MAC_HOST` (the Mac's 10 GbE IP — required, no default),
   296	`MAC_MODULE_ROOT` (default `$MAC_WORK` — see D3), `SKIPPY_SSH` (default
   297	`admin@skippy`), `SKIPPY_HOST`, `SKIPPY_BIN` (default
   298	`/mnt/generic-pool/video/blit-bin`), `SKIPPY_DISK_REGEX`,
   299	`OLD_SHA_ZOEY=e757dcc`, `OLD_SHA_WIN=0f922de`.
   300	
   301	Verification entry point for harness commits (no crates/proto touched; the
   302	cargo gates don't exercise bash): `bash -n` on each script + shellcheck
   303	where installed + `bash scripts/agent/check-docs.sh` + the codex review;
   304	the methodology itself is verified by the probe/recorded-run discipline
   305	(otp-2 precedent) and each script supports `PREFLIGHT_ONLY=1` (run every
   306	preflight check and exit before fixtures).
   307	
   308	### D6 — staging per host
   309	
   310	| host | old arm | new arm |
   311	|------|---------|---------|
   312	| Mac | rebuild client at the pinned sha in a detached worktree → `~/blit-bench-work/bins/blit-<sha>` | `cargo build --release` at the run commit |
   313	| zoey | already staged (`$ZOEY_TEMP/blit-daemon`, `e757dcc` pair kept for otp-12 — machines.md) | `cargo zigbuild --release --target aarch64-unknown-linux-musl` → staged BESIDE the old one as `blit-daemon-<sha>` (never overwrite); everything stays inside `blit-temp/` |
   314	| Windows | copy the detached-checkout exes ASIDE first (`D:\blit-test\bins\0f922de\`) before any checkout movement | fresh git bundle (pushes are owner-gated; origin lags at `6d37a22`) → checkout run commit → native `cargo build --release` (daemon AND `blit.exe` client) → `D:\blit-test\bins\<sha>\` |
   315	| skippy | none (no old baseline; July binaries unusable) | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (static — sidesteps the recorded glibc 2.36 ceiling) → `$SKIPPY_BIN/bins/<sha>/` (pool paths are exec-friendly; `/tmp` and `/home` are noexec) — `blit` + `blit-daemon` |
   316	
   317	Windows daemon-swap mechanics: the active arm's exe is COPIED to the fixed
   318	path `D:\blit-test\bins\active\blit-daemon.exe` and launched from there —
   319	one program-scoped firewall rule total (the rule is exe-path-scoped;
   320	sha-named dirs keep provenance, the copy log records each swap). Launch
   321	stays WMI `Win32_Process.Create` + stale-refusal + PID-scoped teardown
   322	(otp-2w README §Host plumbing). A staging manifest (sha256 of every binary
   323	on every host) is recorded in each evidence README.
   324	
   325	### D7 — matrix size and session budget
   326	
   327	| rig | comparisons | timed runs | est. wall |
   328	|-----|------------:|-----------:|----------:|
   329	| Z converge-up | 12 (3 fixtures × 2 dirs × 2 carriers) | 96 | 1.5–2.5 h (drains dominate) |
   330	| W converge-up | 12 | 96 | ~1.5 h |
   331	| W invariance | 12 (3 × 2 dirs × 2 carriers, new-only) | 96 | ~1.5 h |
   332	| D delegated | 6 (3 × 2 dirs, TCP) + 1 grpc smoke | 56 | ~1 h |
   333	
   334	Each rig session needs the owner's machines on and otherwise idle; sessions
   335	are independent and may run on different days (each records its own rig
   336	state).
   337	
   338	## Staging (sub-slices; each commit through the codex loop)
   339	
   340	- **otp-12a — rig Z**: `bench_otp12_zoey.sh` (harness commit; codex; fix) →
   341	  recorded run → `docs/bench/otp12-zoey-<date>/README.md` + CSVs (evidence
   342	  commit; codex; fix). Preflight gates: staged old pair present; new musl
   343	  daemon staged beside it; **fresh owner go for daemon runs on zoey**
   344	  (standing STATE rule) and zoey out of maintenance.
   345	- **otp-12b — rig W**: `bench_otp12_win.sh` covering converge-up block +
   346	  invariance block; same two-commit shape. Preflight gates: bundle
   347	  delivered + old exes copied aside + new native build (daemon + client);
   348	  Mac daemon smoke from Windows (firewall).
   349	- **otp-12c — rig D**: `bench_otp12_delegated.sh`; same shape. Preflight
   350	  gates: fresh skippy staging on the pool; `sudo -n` drop_caches on skippy;
   351	  delegation config both daemons; reachability smokes in both directions
   352	  (control port + a 1-file TCP-carrier transfer — the data plane binds
   353	  ephemeral ports, so the smoke IS the firewall test).
   354	- **otp-12d — assembly**: `docs/bench/otp12-acceptance-<date>/README.md` —
   355	  the plan-level verdict matrix assembling every comparison row
   356	  criterion-by-criterion (the artifact otp-13 walks). Docs-only commit.
   357	  The plan's acceptance-criteria checkboxes are NOT flipped here — that
   358	  is the otp-13 owner walk (codex design F4; checkpoints are owner-only).
   359	
   360	Rig order may flex with availability; 12d requires all three.
   361	
   362	## Evidence layout
   363	
   364	`docs/bench/otp12-{zoey,win,delegated}-<date>/` each carry: `README.md`
   365	(otp-2 README shape: Status/Scope, Build with all arm shas, Rig, results
   366	tables, stability, methodology deltas, reproduction), `runs.csv`,
   367	`summary.csv`, `verdicts.csv`, `drain-outcomes.txt`, `staging-manifest.txt`
   368	(sha256 per binary per host). `docs/bench/otp12-acceptance-<date>/README.md`
   369	is the assembly. Raw session logs stay under `logs/` (untracked) as usual.
   370	
   371	## Known gaps / risks
   372	
   373	- **No rig is truly fs-identical.** The plan's "symmetric rig" is
   374	  instantiated by the owner-designated closest-spec pair; rig W's two
   375	  directions still land on different OS write paths (APFS vs NTFS +
   376	  Defender at its normal state). D2's discriminator computation is the
   377	  pre-registered, evidence-backed handling; a platform-residue cell counts
   378	  as satisfied per D-2026-07-12-1.
   379	- **Old-arm provenance is a staging record, not a handshake** (old paths
   380	  predate it). Mitigated by machines.md provenance + the sha256 manifest;
   381	  accepted residual risk.
   382	- **First-of-kind surfaces**: a daemon on the Mac (application firewall
   383	  unknown until the smoke) and a client on skippy (musl-static, untested
   384	  there — the zoey zigbuild recipe retargeted). Both are preflight-gated;
   385	  failures block the affected block only.
   386	- **zoey availability**: under maintenance 2026-07-11; daemon runs there
   387	  need a fresh owner go regardless (STATE rule).
   388	- **Delegated arm includes trigger/relay overhead by design** — recorded,
   389	  expected sub-ms on this LAN; if it ever dominates a cell, that IS a
   390	  finding, not noise.
   391	- **Suite/test count**: untouched — no crates/proto changes anywhere in
   392	  otp-12; the ≥1483 floor stands at 1484 from otp-11b.
   393	
   394	## Open questions — RESOLVED (owner, 2026-07-12; D-2026-07-12-1)
   395	
   396	- **Q1 — cross-direction residue on rig W**: RESOLVED "yes" — a cell that
   397	  beats its own old direction, is initiator-invariant, and misses the
   398	  `min(old_push, old_pull) × 1.10` bar only by a discriminator-attributed
   399	  platform write-path residue (same gap in the old arm, same session)
   400	  **counts as satisfying the cross-direction half of criterion 2**
   401	  (D-2026-07-12-1). The evidence still records both computations per
   402	  cell; the otp-13 walk reviews the numbers, but a platform-residue cell
   403	  is not a blocker.

exec
/bin/zsh -lc 'git show 8f4fbf9''^:scripts/bench_otp2_baseline.sh | nl -ba' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-i1KKLdid' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GJpQysNk' (errno=Operation not permitted)
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
   131	SESSION_TAG=$(date +%H%M%S).$$
   132	log "build sha: $BUILD_SHA  client: $(uname -m) macOS  daemon: $ZOEY_HOST  session: $SESSION_TAG"
   133	
   134	# --- Daemon lifecycle (everything inside ZOEY_TEMP) ------------------
   135	start_daemon() {
   136	    zssh "mkdir -p '$MODULE_ROOT' && cat > '$ZOEY_TEMP/bench-config.toml' <<EOF
   137	[daemon]
   138	bind = \"0.0.0.0\"
   139	port = $PORT
   140	no_mdns = true
   141	
   142	[[module]]
   143	name = \"bench\"
   144	path = \"$MODULE_ROOT\"
   145	EOF
   146	nohup '$ZOEY_TEMP/blit-daemon' --config '$ZOEY_TEMP/bench-config.toml' \
   147	  > '$ZOEY_TEMP/bench-daemon.log' 2>&1 &
   148	echo \$! > '$ZOEY_TEMP/bench-daemon.pid'"
   149	    sleep 1
   150	    zssh "kill -0 \$(cat '$ZOEY_TEMP/bench-daemon.pid')" \
   151	        || { zssh "cat '$ZOEY_TEMP/bench-daemon.log'"; exit 1; }
   152	    log "daemon up on $ZOEY_HOST:$PORT (module bench -> $MODULE_ROOT)"
   153	}
   154	
   155	stop_daemon() {
   156	    zssh "kill \$(cat '$ZOEY_TEMP/bench-daemon.pid' 2>/dev/null) 2>/dev/null; \
   157	          rm -f '$ZOEY_TEMP/bench-daemon.pid'" || true
   158	}
   159	# Sweep this invocation's push destinations even on an interrupted run
   160	# (F5) — never leave content a rerun could no-op onto. Staged pull
   161	# sources are kept for re-runs by design.
   162	sweep_push_dirs() {
   163	    zssh "cd '$MODULE_ROOT' 2>/dev/null && rm -rf push_${SESSION_TAG}_*" || true
   164	}
   165	trap 'stop_daemon; sweep_push_dirs' EXIT
   166	
   167	# --- Pool drain + cold caches, both ends ------------------------------
   168	# Order matters (codex otp-2 F4): FIRST flush the daemon host's dirty
   169	# pages into the pool (`sync` — Linux sync waits), THEN wait for the
   170	# tier to destage until quiet (three consecutive 2s windows with
   171	# < 2 MiB written across all physical disks; timeout 240s), then cold
   172	# the caches. A drain timeout is recorded against the run's label in
   173	# drain.log AND bench.log so an undrained row is identifiable, never
   174	# silent.
   175	drain_pool() {
   176	    zssh 'sync
   177	quiet=0
   178	for i in $(seq 1 120); do
   179	  a=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
   180	  sleep 2
   181	  b=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
   182	  if [ $((b-a)) -lt 4096 ]; then quiet=$((quiet+1)); else quiet=0; fi
   183	  if [ $quiet -ge 3 ]; then echo "drained ${i}x2s"; exit 0; fi
   184	done
   185	echo "DRAIN-TIMEOUT"'
   186	}
   187	
   188	drop_caches() {   # $1 = run label for the drain record
   189	    local outcome
   190	    outcome=$(drain_pool || true)
   191	    echo "$1: ${outcome:-DRAIN-ERROR}" >> "$OUT_DIR/drain.log"
   192	    # Anything but a positive "drained" report is a warned anomaly —
   193	    # a timeout AND a failed/empty probe alike (fail loud, not open).
   194	    [[ "$outcome" == drained* ]] || log "  WARNING: $1 ran UNDRAINED (${outcome:-probe failed})"
   195	    sync
   196	    sudo -n /usr/sbin/purge
   197	    zssh "echo 3 > /proc/sys/vm/drop_caches"
   198	}
   199	
   200	# --- Fixtures (client disk; generated once) --------------------------
   201	gen_fixtures() {
   202	    if [[ ! -d "$MAC_WORK/src_large" ]]; then
   203	        mkdir -p "$MAC_WORK/src_large"
   204	        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
   205	        log "generated large fixture (1 GiB)"
   206	    fi
   207	    if [[ ! -d "$MAC_WORK/src_small" ]]; then
   208	        mkdir -p "$MAC_WORK/src_small"
   209	        for i in $(seq 1 10000); do
   210	            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
   211	            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
   212	        done
   213	        log "generated small fixture (10000 x 4 KiB)"
   214	    fi
   215	    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
   216	        mkdir -p "$MAC_WORK/src_mixed"
   217	        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
   218	        for i in $(seq 1 5000); do
   219	            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
   220	            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
   221	        done
   222	        log "generated mixed fixture (512 MiB + 5000 x 2 KiB)"
   223	    fi
   224	}
   225	
   226	# --- Timing core ------------------------------------------------------
   227	CSV="$OUT_DIR/results.csv"
   228	echo "cell,run,ms" > "$CSV"
   229	SUMMARY="$OUT_DIR/summary.csv"
   230	echo "cell,median_ms,avg_ms,best_ms" > "$SUMMARY"
   231	
   232	finish_cell() {  # label total best  (per-run times read back from CSV)
   233	    local label="$1" total="$2" best="$3"
   234	    local median
   235	    median=$(grep "^$label," "$CSV" | cut -d, -f3 | sort -n | awk '
   236	        { v[NR] = $1 }
   237	        END { if (NR % 2) print v[(NR+1)/2];
   238	              else print int((v[NR/2] + v[NR/2+1]) / 2) }')
   239	    echo "$label,$median,$(( total / RUNS )),$best" >> "$SUMMARY"
   240	    log "  $label median: ${median}ms avg: $(( total / RUNS ))ms best: ${best}ms"
   241	}
   242	
   243	# push: client fixture -> fresh, never-seen module subdir per run.
   244	# SESSION_TAG makes destinations unique per INVOCATION too (codex otp-2
   245	# F5): an interrupted run's leftovers can never turn a rerun's copy
   246	# into a partial no-op; the EXIT trap also sweeps them.
   247	push_cell() {    # label src flag(optional)
   248	    local label="$1" src="$2" flag="${3:-}"
   249	    local total=0 best=999999999 run start end ms
   250	    local flush_ms
   251	    for run in $(seq 1 "$RUNS"); do
   252	        drop_caches "$label-r$run"
   253	        start=$(now_ms)
   254	        # shellcheck disable=SC2086
   255	        "$BLIT" copy "$src" "${REMOTE}push_${SESSION_TAG}_${label}_r${run}/" --yes $flag >/dev/null 2>&1
   256	        end=$(now_ms)
   257	        flush_ms=$(sync_dest_ms)   # durable at dest, self-timed
   258	        ms=$(( end - start + flush_ms ))
   259	        total=$(( total + ms )); (( ms < best )) && best=$ms
   260	        log "  $label run $run: ${ms}ms (sync ${flush_ms}ms)"
   261	        echo "$label,$run,$ms" >> "$CSV"
   262	    done
   263	    finish_cell "$label" "$total" "$best"
   264	}
   265	
   266	# pull: staged module subdir -> fresh local dest per run.
   267	pull_cell() {    # label remote_src flag(optional)
   268	    local label="$1" remote_src="$2" flag="${3:-}"
   269	    local total=0 best=999999999 run start end ms fsync_ms
   270	    for run in $(seq 1 "$RUNS"); do
   271	        rm -rf "$MAC_WORK/dst_pull"
   272	        mkdir -p "$MAC_WORK/dst_pull"
   273	        drop_caches "$label-r$run"
   274	        start=$(now_ms)
   275	        # shellcheck disable=SC2086
   276	        "$BLIT" copy "$remote_src" "$MAC_WORK/dst_pull" --yes $flag >/dev/null 2>&1
   277	        end=$(now_ms)
   278	        fsync_ms=$(fsync_tree_ms "$MAC_WORK/dst_pull")   # durable, self-timed
   279	        ms=$(( end - start + fsync_ms ))
   280	        total=$(( total + ms )); (( ms < best )) && best=$ms
   281	        log "  $label run $run: ${ms}ms (fsync ${fsync_ms}ms)"
   282	        echo "$label,$run,$ms" >> "$CSV"
   283	    done
   284	    finish_cell "$label" "$total" "$best"
   285	}
   286	
   287	# --- Matrix ------------------------------------------------------------
   288	main() {
   289	    gen_fixtures
   290	    start_daemon
   291	
   292	    # Stage pull sources once (untimed): each fixture into its own
   293	    # module subdir. Caches are dropped before every timed pull, so
   294	    # the staging write does not warm anything that matters.
   295	    log "staging pull sources (untimed)"
   296	    local w
   297	    for w in large small mixed; do
   298	        "$BLIT" copy "$MAC_WORK/src_$w" "${REMOTE}pull_src_$w/" --yes >/dev/null 2>&1
   299	    done
   300	
   301	    for w in large small mixed; do
   302	        push_cell "push_tcp_${w}" "$MAC_WORK/src_$w"
   303	        push_cell "push_grpc_${w}" "$MAC_WORK/src_$w" --force-grpc
   304	        pull_cell "pull_tcp_${w}" "${REMOTE}pull_src_$w/src_$w/"
   305	        pull_cell "pull_grpc_${w}" "${REMOTE}pull_src_$w/src_$w/" --force-grpc
   306	    done
   307	
   308	    stop_daemon
   309	
   310	    log ""
   311	    log "=== SUMMARY (cold-cache, disk-to-disk, $RUNS runs/cell) ==="
   312	    column -t -s, "$SUMMARY" | tee -a "$OUT_DIR/bench.log"
   313	    log "results: $CSV"
   314	}
   315	
   316	main "$@"

exec
/bin/zsh -lc "sed -n '1,320p' docs/STATE.md && sed -n '1,260p' REVIEW.md && find .review/findings -maxdepth 2 -type f -print | sort" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
   c-1/2), **otp-11 (a + b)** `[x]`. Current: **otp-12 (the
   symmetric-rig acceptance run — initiator/verb invariance A/B +
   every cell ≤ the better old direction + noise)** — needs the rigs
   (Blocked below); then otp-13 (owner checklist walk).
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

- **Rig availability (owner, 2026-07-10, verified by ssh)**: for the
  otp-12 matrix — remote↔remote (delegated) uses the Windows box
  (`michael@10.1.10.173`) + TrueNAS `skippy` (`admin@skippy`,
  x86_64; existing test folder `/mnt/generic-pool/video/blit-bin/`
  with July binaries + bench.toml; /tmp and /home are noexec there);
  skippy also available for Mac↔Linux cells "if needed" (owner).
  zoey = per-direction rig; Windows pair = cross-direction rig.
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
# Review status

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
| m2f-1-f2-source-daemon | Feature | Tag F2 transfer rows with their source daemon (multi-daemon F2 step 1) | `[x]` | `phase5/a1` | `aeac25d` |
| m2f-2-f2-composite-key | Feature | Key F2 transfers by (daemon, transfer_id) (multi-daemon F2 step 2, round 2) | `[x]` | `phase5/a1` | `1aed724` |
| m2f-3-f2-merge-snapshot | Feature | Additive per-daemon snapshot hydration + refresh identity fix (multi-daemon F2 step 3) | `[x]` | `phase5/a1` | `7202418` |
| m2f-4-f2-tagged-events | Feature | Carry the source daemon per F2 stream event (multi-daemon F2 step 4) | `[x]` | `phase5/a1` | `8979ff2` |
| m2f-5-f2-fanout | Feature | F2 watches all discovered daemons via merged Subscribe streams (multi-daemon F2 step 5, round 2) | `[x]` | `phase5/a1` | `49f1fce` |
| m2f-6-f2-daemon-column | Feature | Render the source-daemon column in F2 tables (multi-daemon F2 step 6) | `[x]` | `phase5/a1` | `a5456cc` |
| m2f-7-f2-multi-daemon-cancel | Feature | Single cancel (K) targets the selected row's daemon (multi-daemon F2 step 7) | `[x]` | `phase5/a1` | `bbd0084` |
| m2f-8-f2-batch-cancel | Feature | Batch cancel (X) targets each active row's own daemon (multi-daemon F2 step 8) | `[x]` | `phase5/a1` | `dfdaabd` |
| m2f-9-f2-discovery-refan | Feature | Auto re-fan F2 when the discovered-daemon set changes (multi-daemon F2 step 9, round 3) | `[x]` | `phase5/a1` | `9204a4d` |
| e-8-config-default-remote | Feature | Fall back to `[daemon] default_remote` config when no --remote flag (Milestone E) | `[x]` | `phase5/a1` | `bf56a66` |
| m2f-10-f2-per-daemon-health | Feature | Partial-degrade F2 banner when one daemon's stream drops (multi-daemon F2 step 10) | `[x]` | `phase5/a1` | `365be9a` |
| e-9-theme-f2-row-highlight | Feature | F2 active-row highlight honors `[theme] accent_color`, contrasting fg (Milestone E, round 2) | `[x]` | `phase5/a1` | `7dd3e31` |
| e-10-theme-f3f4-highlight | Feature | F3/F4 selection highlights honor `[theme] accent_color` + contrasting fg (Milestone E) | `[x]` | `phase5/a1` | `895fe06` |
| e-11-theme-f1-highlight | Feature | F1 daemon-list highlight honors `[theme] accent_color` + contrasting fg (Milestone E) | `[x]` | `phase5/a1` | `ab85658` |
| bridge-1-prometheus-scaffold | Feature | Prometheus bridge step 1: GetState→prom-text formatter + print-once CLI (Milestone E, round 2) | `[x]` | `phase5/a1` | `9411754` |
| bridge-2-prometheus-http | Feature | Prometheus bridge step 2: long-running /metrics HTTP server, pull-model scrape (Milestone E, round 3) | `[x]` | `phase5/a1` | `5eb9e61` |
| bridge-3-prometheus-readme | Docs | Prometheus bridge step 3: operator README (usage, scrape config, metric reference) (Milestone E) | `[x]` | `phase5/a1` | `9561fb2` |
| keys-1-config-quit | Feature | Key remapping step 1: configurable quit key via `[keys]` config + KeyMap (Milestone E, round 2) | `[x]` | `phase5/a1` | `19c6b7f` |
| keys-2-config-refresh | Feature | Key remapping step 2: configurable refresh key via `[keys]` config, quit/refresh collision policy (Milestone E, round 2) | `[x]` | `phase5/a1` | `ead1adb` |
| keys-3-config-pane-switch | Feature | Key remapping step 3: configurable pane-switch digit aliases + generalized collision policy (Milestone E) | `[x]` | `phase5/a1` | `43d5842` |
| dark-1-theme-base-colors | Feature | dark/light step 1: configurable `[theme] background`/`foreground` base layer (Milestone E) | `[x]` | `phase5/a1` | `775bbe7` |
| dark-2-theme-mode-preset | Feature | dark/light step 2: `[theme] mode = dark\|light` presets (explicit colors override, incl. invalid→terminal-default) (Milestone E, round 2) | `[x]` | `phase5/a1` | `ce4c50f` |
| keys-4-config-movement | Feature | Key remapping step 4: configurable list-cursor aliases `[keys] move_down/up/top/bottom`, lowest-precedence in the collision policy, arrow/Home/End failsafe (Milestone E) | `[x]` | `phase5/a1` | `f9e3378` |
| rec-1-recent-persistence | Feature | Persist `GetState.recent[]` across daemon restarts via dedicated recents.jsonl (separate from planner's perf_local.jsonl); non-blocking write-through + atomic rewrite, opt-in (recent-persistence step 1) | `[x]` | `phase5/a1` | `7c095b2` |
| rec-2-clear-recent | Feature | `ClearRecent` RPC: wipe recent ring + recents.jsonl, never touching planner's perf_local.jsonl (core safety test); empty request, count response (recent-persistence step 2) | `[x]` | `phase5/a1` | `9c2955e` |
| rec-3-tui-clear-recent | Feature | F2 `E` "clear recent" action: empties local view + fans ClearRecent RPC to watched daemons (fire-and-forget); blit-app client helper; footer hint (recent-persistence step 3, final) | `[x]` | `phase5/a1` | `00d2ba5` |
| audit-3a-mutex-poisoning | Robustness | Recover poisoned ActiveJobs table/recent mutexes via `unwrap_or_else(into_inner)` instead of `expect` panic cascade (audit-3 part 1 of 2) | `[x]` | `phase5/a1` | `198ff31` |
| audit-3b-rng-fallible | Robustness | `generate_token` returns `Result` (RNG failure → Status::Internal) instead of panicking the spawned data-plane task; 6 callers propagate via `?` (audit-3 part 2 of 2) | `[x]` | `phase5/a1` | `eeb7c16` |
| audit-5a-bridge-correctness | Robustness | Prometheus bridge: one-shot scrape timeout (8s, fail-loudly) + `\r` escaping in escape_label (audit-5 part 1 of 2) | `[x]` | `phase5/a1` | `f6d2d2d` |
| audit-1a-delegation-port-zero | Robustness | Reject IANA-reserved source port 0 at the delegation gate before DNS/connect (audit-1 item 5; timeouts deferred to audit-1b + owner decision on idle-timeouts) | `[x]` | `phase5/a1` | `a3147b6` |
| audit-1b-net-timeouts-keepalive | Robustness | Delegation DNS-resolve (10s) + dst→src connect (30s) timeouts via net_timeout::within; daemon HTTP/2 keepalive (30s/20s) reaps vanished subscribers — owner-decided over idle-close (audit-1 items 1/2/4) | `[x]` | `phase5/a1` | `1d88fea` |
| audit-7-cargo-lock | Style | Track Cargo.lock for reproducible builds (4-binary workspace); remove from .gitignore (audit-7 item 10, owner-approved — supersedes the never-add rule for the lockfile only) | `[x]` | `phase5/a1` | `dfaecfe` |
| rec-4-clear-recent-confirm | Feature | F2 `E` clear-recent now asks `clear recent? y/N` first (owner-requested); reuses F2CancelStatus confirm machinery via ConfirmingClearRecent variant | `[x]` | `phase5/a1` | `3673ee1` |
| audit-2a-cli-connect-timeout | Robustness | blit_app::client::connect_with_timeout + swap all 10 admin BlitClient::connect sites incl jobs::query. Round 2: DNS-aware outer timeout (connect_timeout alone didn't bound slow DNS). Round 3: corrected stale connect_timeout docs (audit-2 part 1 of 2) | `[x]` | `phase5/a1` | `179f5fa` |
| audit-2b-remote-connect-timeout | Robustness | Bound remaining connects DNS-aware: RemotePull/PushClient::connect at source (fixes 3 data-path sites) + transfers/remote 2 BlitClient sites + blit-cli completions (audit-2 part 2 of 2) | `[x]` | `phase5/a1` | `40ed2d6` |
| audit-4-windows-handle-leak | Bug | RAII OwnedHandle guard closes the CreateFileW handle on every exit path in capture_snapshot (was leaked on the GetFileInformationByHandle `?`). Windows target cargo check passed with `CARGO_FEATURE_PURE=1`; target clippy blocked by pre-existing Windows warnings; Darwin gates pass | `[x]` | `phase5/a1` | `4e77897` |
| audit-5b1-bridge-listener-write | Robustness | Bridge: SO_REUSEADDR listener (build_listener via TcpSocket) + response write timeout (write_all_within, 10s) (audit-5 items 5/6; part 1 of 2 for the server hardening) | `[x]` | `phase5/a1` | `28e9956` |
| audit-5b2-bridge-server-lifecycle | Robustness | Bridge: graceful ctrl_c shutdown + Semaphore concurrency bound (MAX_CONCURRENT_SCRAPES=64) in serve() (audit-5 items 3/4; part 2 of 2 — completes audit-5) | `[x]` | `phase5/a1` | `05f77ec` |
| audit-6d-path-safety-unicode | Test Gap | path_safety: lock in Unicode-opaque containment boundary (NFC/NFD, bidi U+202E, ZWJ, separator/dot lookalikes) — preserved verbatim, can't smuggle traversal; non-UTF-8 unreachable via &str (audit-6 item 4) | `[x]` | `phase5/a1` | `d75cdcf` |
| audit-11-data-plane-underflow | Bug | send_file_double_buffered: clamp each read to `remaining` before subtracting — an over-returning reader (file grew / lying TransferSource) underflowed remaining (debug panic / release u64::MAX runaway) and could push undeclared bytes; now sends exactly header.size | `[x]` | `phase5/a1` | `6a0feb0` |
| audit-12-buffer-pool-leak | Robustness | BufferPool acquire/try_acquire: defer std::mem::forget(permit) until after the vec! allocation so an alloc panic releases the memory-budget permit by unwind instead of leaking it (permit leak → pool starvation) | `[x]` | `phase5/a1` | `326b3ff` |
| audit-9-cancel-auth | Bug | CancelJob now authorizes the caller against the transfer's originating peer (host/IP-only, port-insensitive; loopback + UDS bypass); cross-tenant cancel → PermissionDenied. New CancelOutcome::Unauthorized | `[x]` | `phase5/a1` | `3c5a398` |
| audit-10-cancel-completion-race | Bug | DelegatedPull select: order the handler branch first in the biased select (via resolve_delegated_pull_outcome helper) so a completion wins over a simultaneous CancelJob — was mis-recording a success as "cancelled via CancelJob" | `[x]` | `phase5/a1` | `3601f1e` |
| audit-8-tui-task-leak | Robustness | TUI Subscribe forwarder races tx.closed() (via forward_step) so it exits on F2 re-fan even for a silent daemon (was leaking conn+Receiver+slot); + outer tokio::time::timeout around jobs::subscribe open. Round 2: also bound the initial GetState snapshot fetch (fetch_snapshot_within → degraded Err) | `[x]` | `phase5/a1` | `2d7b6f7` |
| audit-6f-dns-rebinding-test | Test Gap | delegation_gate DNS-rebinding regression: ScriptedResolver returns IP A then B; assert the gate resolves once, binds A, never consults B (+ converse: denies on the first resolution only) (audit-6 item 6) | `[x]` | `phase5/a1` | `28e0b95` |
| audit-6g-copy-fallback-test | Test Gap | copy_file fast-path→fallback: byte-identical copy on all platforms + macOS clonefile-EEXIST forces the clonefile→fcopyfile hop. Round 2: assert clone_succeeded to pin the hop (not the buffered tail). Buffered-tail needs a production seam (flagged) (audit-6 item 7) | `[x]` | `phase5/a1` | `4c4db89` |
| audit-7e-cleanup | Style | Remove 33 tracked AppleDouble ._* sidecars + 2 empty npm stubs (package.json/lock); gitignore ._*. Rust-only workspace, no build/test impact (audit-7 code-health) | `[x]` | `phase5/a1` | `16a92ce` |
| audit-6c-bridge-http-integration | Test Gap | bridge end-to-end HTTP test: real client → handle_conn over loopback; GET /metrics (unreachable daemon → 200 + blit_daemon_up 0) and GET /favicon → 404 (audit-6 item 3) | `[x]` | `phase5/a1` | `02c7a9c` |
| audit-7b-dead-code | Style | Remove dead compare.rs fns (+orphaned imports), 3 STALE fs_enum allow(dead_code) (fields are live), write-only diagnostics written_at field, empty blit-app progress.rs stub. remote_remote_direct.rs left (live, 285 lines) (audit-7 code-health) | `[x]` | `phase5/a1` | `5a5f735` |
| audit-6a-blit-app-filter-tests | Test Gap | blit-app transfers/filter.rs: 6 tests on build/build_spec (glob/size/age propagation, reference_time capture, malformed-glob + bad-size rejection, Duration→secs). Note: "zero #[cfg(test)]" premise stale — 8 files already tested (audit-6 item 1) | `[x]` | `phase5/a1` | `8820226` |
| audit-7c-docs | Docs | ARCHITECTURE.md: add blit-app/blit-tui/blit-prometheus-bridge crate sections + diagram; complete gRPC surface to all 15 RPCs (verified vs proto). README: fix clone URL your_org→roethlar/Blit. Round 2: bridge as blit-app consumer, full module table (check/scan/display), F4=profile/verify/diagnostics (audit-7 code-health) | `[x]` | `phase5/a1` | `a11845a` |
| audit-6b-tui-render-test | Test Gap | F4 render_into driven through ratatui TestBackend (Profile+Verify+Diagnostics+Transfer): renders default state at 120x40 + tiny 8x3 area, asserts no panic (clamp). f2/f3/help already covered (audit-6 item 2) | `[x]` | `phase5/a1` | `267f093` |
| audit-6e-move-directory-coverage | Test Gap | directory-tree push-move + pull-move integration tests (recursive copy-then-delete-source; all files land + entire source removed). All 4 cardinal directions already covered single-file; this fills the multi-file gap (audit-6 item 5). Round 2: assert recursive remote source-tree removal on pull-move | `[x]` | `phase5/a1` | `6d410ac` |
| audit-1c1-stall-guard | Robustness | StallGuard<R> AsyncRead idle-timeout adapter (no-bytes-for-30s → TimedOut; re-armed per read = idle not total). Owner scope=all pulls. Part 1 of 2; part 2 wires it into the receive pipeline (audit-1c) | `[x]` | `phase5/a1` | `0cfa534` |
| audit-1c2-stall-wiring | Robustness | Wire StallGuard into the receive pipeline: generic-ize execute_receive_pipeline + 6 read helpers over AsyncRead, wrap the socket in pull.rs (unconditional → all pulls). Completes audit-1c (audit-1 item 3) | `[x]` | `phase5/a1` | `906cedf` |
| retry-wait1-classifier-loop | Feature | retryable-error classifier (transient io kinds incl. StallGuard TimedOut; fatal eyre/path/gate not retried) + run_with_retries loop in blit-app. Owner-approved --retry/--wait part 1 of 2; part 2 adds flags+wiring | `[x]` | `phase5/a1` | `e5e59fb` |
| retry-wait2-cli-wiring | Feature | --retry<N>/--wait<SECS> on TransferArgs (default 0/5) + wrap run_transfer/run_move in run_with_retries (resumable retry on transient failures). Completes the retry-wait feature (owner-approved follow-up) | `[x]` | `phase5/a1` | `68b34ac` |
| audit-13-buffer-pool-double-locking | Performance | BufferPool release/return_vec: single-lock cache return via cache_returned_buffer + drop redundant per-release buffer_size zeroing (truncate common path); verified no consumer relies on pre-zeroed pool buffers (Gemini-sourced) | `[x]` | `phase5/a1` | `f9d3f2f` |
| audit-14-resume-copy-redundant-seek | Performance | resume_copy_file: drop the redundant per-iteration src seek (sequential) + track dst_cursor_pos to seek dst only on divergence. Pure syscall reduction; existing byte-exact resume suite covers it (Gemini-sourced) | `[x]` | `phase5/a1` | `b7f8177` |
| audit-15-grpc-missing-connection-timeouts | Robustness | RECOMMEND-DEFER (analysis only, no code): blanket Server::timeout(30s) would kill the 7 streaming RPCs (Subscribe/DelegatedPull/Pull/PullSync/Push/Find/DiskUsage); dead-peer case already covered by audit-1b keepalive. Reviewer to grade decision (Gemini-sourced) | `[x]` | `phase5/a1` | `f0ed9e5` |
| audit-7d1-extract-progress-accum | Refactor | main.rs split part 1: extract 5 pure progress helpers (accumulate_pull/push/delegated_progress, pull_throughput, du_total_from_entries) verbatim into crate::progress_accum; crate-root use keeps call sites + tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `5112705` |
| audit-7d2-extract-display-f3 | Refactor | main.rs split part 2: extract 4 pure F3 state→display mappers (f3_pull_to_display + private confirm_detail, f3_du_to_display, f3_del_to_display) verbatim into crate::display_f3; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `315f923` |
| audit-7d3-extract-display-f1 | Refactor | main.rs split part 3: extract 4 pure F1 state→display mappers (f1_trigger_prompt, f1_push_status + private push_present_verb/push_past_verb) verbatim into crate::display_f1; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `1e50f7d` |
| audit-7d4-extract-display-f2 | Refactor | main.rs split part 4: extract 2 pure F2 cancel mappers (cancel_status_to_display, cancel_status_remaining_ttl) verbatim into crate::display_f2; F2CancelStatus stays in main.rs (event loop mutates it), referenced read-only via crate-root path; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `0ed685a` |
| audit-7d5-extract-config-reload | Refactor | main.rs split part 5: extract Ctrl+R config hot-reload helpers (reload_tui_config I/O wrapper + pure classify_reload) verbatim into crate::config_reload; ReloadBanner stays in main.rs (AppState field), referenced via crate-root path; reload_tui_config re-exported at crate root, classify_reload imported test-locally (sole non-test caller moved with it). Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `4e728b5` |
| audit-7d6-extract-tick-budget | Refactor | main.rs split part 6: extract pure sleep-budget math (compute_tick_budget, min_opt — Duration/Option, no AppState) verbatim into crate::tick_budget; crate-root use keeps event-loop call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `a99a136` |
| audit-7d7-extract-del-request | Refactor | main.rs split part 7: extract 3 pure F3-delete request builders (del_wire_path, build_delete_request, is_deletable_remote_path — no async/AppState) verbatim into crate::del_request; crate-root use keeps dispatcher + plan_f1_* + spawn tasks + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `b18e5f9` |
| audit-7d8-extract-exec-plan | Refactor | main.rs split part 8: extract 3 pure transfer-execution builders (f3_pull_options, build_f1_push_execution, build_delegated_execution — no async/AppState/IO) verbatim into crate::exec_plan; remove_local_source stays in main.rs (does IO); crate-root use keeps spawn_* tasks + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `d47cc24` |
| audit-7d9-extract-theme-color | Refactor | main.rs split part 9: extract 2 pure theme-color mappers (base_theme_style, raw_color_to_ratatui — no AppState) verbatim into crate::theme_color; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d). NOTE: plan_f1_trigger/plan_f1_delegated inspected + NOT moved (mutate &mut AppState, coupled). Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `6ddce2e` |
| bug-mirror-literal-backslash | Bug fix | Mirror failure on POSIX filenames containing literal `\` (e.g. macOS Logic Pro `1\4 Single.pst`): blanket `path.to_string_lossy().replace('\\', "/")` at 11 sites was destructive. New canonical `blit_core::path_posix::relative_path_to_posix` (component-walk) + every Path→wire site routed through it. Round 2: GPT-reopen — `relative_str_to_posix` was dropping trailing `/` in admin completion input (regressed dir-prefix completions); now preserves trailing-separator UX semantic for user input while keeping wire/manifest form clean. 10 regression tests. Owner-verified. | `[x]` | `phase5/a1` | `5a034dd` |
| tui-key-dispatch-press-only-filter | Bug fix | TUI input task dropped Repeat events: `spawn_input_task` matched only `KeyEventKind::Press`, silently dropping autorepeat (and any other non-Press kind). Now accepts Press + Repeat, only filters Release. Plus a `BLIT_TUI_INPUT_TRACE=1` env-gated diagnostic log to `/tmp/blit-tui-input.log` for follow-up if needed. Owner-authorized. | `[x]` | `phase5/a1` | `2e5bcb9` |
| windows-move-tree-hang | Known issue | `test_remote_move_local_to_remote_directory_tree` hangs on Windows CI (14+ consecutive runs). Other 3 `remote_move` tests pass. Suspected: local source-delete `fs::remove_dir_all` blocked by open file handles from push enumeration (POSIX-vs-Windows unlink semantics). Test gated off Windows with `cfg_attr ignore`; root cause needs interactive Windows debugging. Owner-authorized defer. | `[x]` | `phase5/a1` | `2e5bcb9` |

## Open findings

| ID         | Severity | Title                                                    | Branch |
|------------|----------|----------------------------------------------------------|--------|
| B          | Feature  | `GetState` RPC + `ActiveJobs` table + recent ring        | `phase5/getstate` |
| M-Jobs     | Feature  | Daemon-owned transfer lifecycle (`CancelJob`, `detach`)  | `phase5/m-jobs` |
| C          | Feature  | `Subscribe` RPC + byte-level instrumentation             | `phase5/c` |
| A.1        | Feature  | TUI implementation                                       |        |
| D          | Feature  | Verify + diagnostics screens                             |        |
| E          | Feature  | Polish (themes, refresh rates, config)                   |        |
| P0-§2.6    | Feature  | Live remote benchmark capture (hardware-bound)           |        |
| audit-1-daemon-timeouts | Robustness | Network operation timeout gaps in delegation path (DNS, gRPC connect, pull_sync_with_spec, subscribe idle) — items 1/2/4 done (audit-1a/1b); item 3 = audit-1c (design pending) | |
| audit-1c-transfer-stall-timeout | Robustness | DESIGN PENDING APPROVAL: no-bytes-30s idle timeout on the delegated pull via an opt-in AsyncRead StallGuard adapter (delegated-only). See finding for approach + open scope question. Prereq for --retry/--wait | |
| audit-2-cli-timeouts | Robustness | Missing connection timeouts on all CLI/admin-verb gRPC connections (~15 sites) | |
| audit-3-panic-resilience | Robustness | SysRng panic in generate_token + 7 mutex poisoning expects in ActiveJobs | |
| audit-4-windows-handle-leak | Bug | Windows HANDLE leak on GetFileInformationByHandle failure in change journal snapshot | |
| audit-5-bridge-robustness | Robustness | Prometheus bridge: one-shot timeout, \r escaping, graceful shutdown, connection limit, write timeout, SO_REUSEADDR | |
| audit-6-test-gaps | Test Gap | Missing test coverage: blit-app (zero tests), TUI rendering, bridge integration, Unicode paths, DNS rebinding, copy fallback | |
| audit-7-code-health | Style | Monolithic files (11K-line main.rs), dead code, 28 AppleDouble artifacts, stale docs, missing Cargo.lock | |
| audit-8-tui-task-leak | Robustness | TUI subscribe forwarder task leak on reconnect; setup task has no connect/RPC timeout | |
| audit-9-cancel-auth | Bug | CancelJob RPC lacks peer authorization — any client can cancel any transfer | |
| audit-10-cancel-completion-race | Bug | Cancel/completion race in delegated_pull: success recorded as "cancelled" under biased select | |
| audit-11-data-plane-underflow | Bug | Buffer underflow in send_file_double_buffered when reader returns excess bytes | |
| audit-12-buffer-pool-leak | Robustness | Semaphore permit leak on OOM panic in BufferPool acquire/try_acquire | |
| audit-13-buffer-pool-double-locking | Performance | Double-locking and redundant memory zeroing in BufferPool | |
| audit-14-resume-copy-redundant-seek | Performance | Redundant seek system calls in sequential block-level resume | |
| audit-15-grpc-missing-connection-timeouts | Robustness | Missing request/idle timeouts on tonic gRPC server control plane | |

## Verified history

Phase 5 A.0 complete. Per-finding audit trails in
.review/findings/a0-delegated-execution.md
.review/findings/a0-dispatch.md
.review/findings/a0-endpoints-gates.md
.review/findings/a0-final-cleanup.md
.review/findings/a0-pull-execution.md
.review/findings/a0-push-execution.md
.review/findings/a0-remote-helpers.md
.review/findings/a0-resolution-fixup.md
.review/findings/a1-1-tui-scaffold.md
.review/findings/a1-2-f2-transfers.md
.review/findings/a1-3-f1-daemons.md
.review/findings/a1-3b-f1-getstate-detail.md
.review/findings/a1-4-f3-browse.md
.review/findings/a1-5-f4-profile.md
.review/findings/a1-6-screen-router.md
.review/findings/a1-6b-state-preservation.md
.review/findings/audit-1-daemon-timeouts.md
.review/findings/audit-10-cancel-completion-race.md
.review/findings/audit-11-data-plane-underflow.md
.review/findings/audit-12-buffer-pool-leak.md
.review/findings/audit-13-buffer-pool-double-locking.md
.review/findings/audit-14-resume-copy-redundant-seek.md
.review/findings/audit-15-grpc-missing-connection-timeouts.md
.review/findings/audit-1a-delegation-port-zero.md
.review/findings/audit-1b-net-timeouts-keepalive.md
.review/findings/audit-1c-transfer-stall-timeout.md
.review/findings/audit-1c1-stall-guard.md
.review/findings/audit-1c2-stall-wiring.md
.review/findings/audit-2-cli-timeouts.md
.review/findings/audit-2a-cli-connect-timeout.md
.review/findings/audit-2b-remote-connect-timeout.md
.review/findings/audit-3-panic-resilience.md
.review/findings/audit-3a-mutex-poisoning.md
.review/findings/audit-3b-rng-fallible.md
.review/findings/audit-4-windows-handle-leak.md
.review/findings/audit-5-bridge-robustness.md
.review/findings/audit-5a-bridge-correctness.md
.review/findings/audit-5b1-bridge-listener-write.md
.review/findings/audit-5b2-bridge-server-lifecycle.md
.review/findings/audit-6-test-gaps.md
.review/findings/audit-6a-blit-app-filter-tests.md
.review/findings/audit-6b-tui-render-test.md
.review/findings/audit-6c-bridge-http-integration.md
.review/findings/audit-6d-path-safety-unicode.md
.review/findings/audit-6e-move-directory-coverage.md
.review/findings/audit-6f-dns-rebinding-test.md
.review/findings/audit-6g-copy-fallback-test.md
.review/findings/audit-7-cargo-lock.md
.review/findings/audit-7-code-health.md
.review/findings/audit-7b-dead-code.md
.review/findings/audit-7c-docs.md
.review/findings/audit-7d1-extract-progress-accum.md
.review/findings/audit-7d2-extract-display-f3.md
.review/findings/audit-7d3-extract-display-f1.md
.review/findings/audit-7d4-extract-display-f2.md
.review/findings/audit-7d5-extract-config-reload.md
.review/findings/audit-7d6-extract-tick-budget.md
.review/findings/audit-7d7-extract-del-request.md
.review/findings/audit-7d8-extract-exec-plan.md
.review/findings/audit-7d9-extract-theme-color.md
.review/findings/audit-7e-cleanup.md
.review/findings/audit-8-tui-task-leak.md
.review/findings/audit-9-cancel-auth.md
.review/findings/audit-h1-mirror-relay-incomplete-scan.md
.review/findings/audit-h11-f1-confirm-detail-err.md
.review/findings/audit-h3a-push-receive-stall.md
.review/findings/audit-h3b-pull-data-plane-write-stall.md
.review/findings/audit-h3c-slice1-grpc-fallback-frame-contract.md
.review/findings/audit-l39-m27-env-var-purge.md
.review/findings/audit-m28-tui-sot-sweep.md
.review/findings/b-1-active-jobs.md
.review/findings/b-2-set-endpoint.md
.review/findings/b-3-recent-ring.md
.review/findings/b-4-getstate.md
.review/findings/b-5-jobs-list.md
.review/findings/bridge-1-prometheus-scaffold.md
.review/findings/bridge-2-prometheus-http.md
.review/findings/bridge-3-prometheus-readme.md
.review/findings/bug-mirror-literal-backslash.md
.review/findings/c-1a-byte-counter-api.md
.review/findings/c-1b-byte-counter-wiring.md
.review/findings/c-2-subscribe-skeleton.md
.review/findings/c-3-transfer-finished-events.md
.review/findings/c-4-transfer-progress.md
.review/findings/c-5a-transfer-id-filter.md
.review/findings/c-5b-event-ring.md
.review/findings/c-6-jobs-watch-stream.md
.review/findings/c-7-watch-replay.md
.review/findings/d-1-f4-profile-lifecycle.md
.review/findings/d-10-transfer-throughput.md
.review/findings/d-11-freshness-tick.md
.review/findings/d-12-esc-cancels-confirm.md
.review/findings/d-13-f2-freshness-footer.md
.review/findings/d-14-f2-active-row-age.md
.review/findings/d-15-f2-active-row-progress.md
.review/findings/d-16-help-overlay-keymap-sync.md
.review/findings/d-17-verify-result-preview.md
.review/findings/d-18-verify-form-clear.md
.review/findings/d-19-digit-tab-shortcuts.md
.review/findings/d-2-f4-verify.md
.review/findings/d-20-f2-recent-throughput.md
.review/findings/d-21-f2-active-cursor.md
.review/findings/d-22-f2-cancel-selected.md
.review/findings/d-23-cancel-status-auto-clear.md
.review/findings/d-24-config-cancel-ttl.md
.review/findings/d-25-f2-tib-tier.md
.review/findings/d-26-f3-filter.md
.review/findings/d-27-f3-sort.md
.review/findings/d-28-f3-no-matches-msg.md
.review/findings/d-29-confirm-cancel.md
.review/findings/d-3-f4-diagnostics.md
.review/findings/d-30-batch-cancel.md
.review/findings/d-31-help-scroll.md
.review/findings/d-32-help-scrollbar.md
.review/findings/d-33-f3-pull-source.md
.review/findings/d-34-f3-pull-endpoint.md
.review/findings/d-35-f3-pull-execute.md
.review/findings/d-36-hot-reload-config.md
.review/findings/d-37-f3-pull-progress.md
.review/findings/d-38-f3-pull-ttl.md
.review/findings/d-39-f3-pull-throughput.md
.review/findings/d-4-f4-local-transfers.md
.review/findings/d-40-config-pull-ttl.md
.review/findings/d-41-f3-du.md
.review/findings/d-42-jump-nav.md
.review/findings/d-43-du-cache.md
.review/findings/d-44-f2-jump-nav.md
.review/findings/d-45-f3-delete.md
.review/findings/d-46-readonly-delete-gate.md
.review/findings/d-47-f1-browse-nav.md
.review/findings/d-48-f2-follows-browse.md
.review/findings/d-49-f3-multiselect.md
.review/findings/d-5-f4-local-move.md
.review/findings/d-50-f3-batch-delete.md
.review/findings/d-51-f3-select-all.md
.review/findings/d-52-config-delete-ttl.md
.review/findings/d-53-f3-batch-pull.md
.review/findings/d-54-f1-module-capacity.md
.review/findings/d-55-f3-mirror.md
.review/findings/d-56-f3-mirror-delete-count.md
.review/findings/d-57-f3-move.md
.review/findings/d-58-f1-trigger.md
.review/findings/d-59-f1-trigger-mirror.md
.review/findings/d-6-f4-verify-checksum-toggle.md
.review/findings/d-60-f1-trigger-move.md
.review/findings/d-61-f1-trigger-push.md
.review/findings/d-62-f1-trigger-error.md
.review/findings/d-63-f1-push-progress.md
.review/findings/d-64-f1-push-ttl.md
.review/findings/d-65-f1-push-mirror-move.md
.review/findings/d-66-f4-clear-confirm.md
.review/findings/d-67-help-clear-confirm.md
.review/findings/d-68-f1-remote-remote-copy.md
.review/findings/d-69-f1-delegated-progress.md
.review/findings/d-7-f4-verify-one-way-toggle.md
.review/findings/d-70-f1-delegated-mirror.md
.review/findings/d-71-f1-delegated-move.md
.review/findings/d-8-f4-elapsed-time.md
.review/findings/d-9-live-tick.md
.review/findings/dark-1-theme-base-colors.md
.review/findings/dark-2-theme-mode-preset.md
.review/findings/design-1-cli-pull-byte-double-count.md
.review/findings/design-2-orphaned-daemon-data-planes.md
.review/findings/design-3-unbounded-data-plane-connects.md
.review/findings/design-4-fallback-midmanifest-negotiation.md
.review/findings/design-5-send-failure-masks-rejection.md
.review/findings/e-1-help-overlay.md
.review/findings/e-10-theme-f3f4-highlight.md
.review/findings/e-11-theme-f1-highlight.md
.review/findings/e-2-tab-strip-counts.md
.review/findings/e-3-config-scaffold.md
.review/findings/e-4-config-tab-strip-counts.md
.review/findings/e-5-config-live-tick-interval.md
.review/findings/e-6-verify-prefill.md
.review/findings/e-7-config-theme.md
.review/findings/e-8-config-default-remote.md
.review/findings/e-9-theme-f2-row-highlight.md
.review/findings/keys-1-config-quit.md
.review/findings/keys-2-config-refresh.md
.review/findings/keys-3-config-pane-switch.md
.review/findings/keys-4-config-movement.md
.review/findings/m-jobs-1-cancel-token.md
.review/findings/m-jobs-2-cancel-rpc.md
.review/findings/m-jobs-3-detach.md
.review/findings/m-jobs-6-watch.md
.review/findings/m2f-1-f2-source-daemon.md
.review/findings/m2f-10-f2-per-daemon-health.md
.review/findings/m2f-2-f2-composite-key.md
.review/findings/m2f-3-f2-merge-snapshot.md
.review/findings/m2f-4-f2-tagged-events.md
.review/findings/m2f-5-f2-fanout.md
.review/findings/m2f-6-f2-daemon-column.md
.review/findings/m2f-7-f2-multi-daemon-cancel.md
.review/findings/m2f-8-f2-batch-cancel.md
.review/findings/m2f-9-f2-discovery-refan.md
.review/findings/otp-1-wire-session-contract.md
.review/findings/otp-10a-push-verb-rides-session.md
.review/findings/otp-10b-1-session-checksum-compare.md
.review/findings/otp-10b-2-pull-verb-rides-session.md
.review/findings/otp-10c-1-relay-removal.md
.review/findings/otp-10c-2-driver-deletion.md
.review/findings/otp-11a-local-session-route.md
.review/findings/otp-11b-local-orchestration-deletion.md
.review/findings/otp-12a-zoey-harness.md
.review/findings/otp-2-symmetric-baseline.md
.review/findings/otp-2w-windows-baseline.md
.review/findings/otp-3-transfer-session-core.md
.review/findings/otp-4-daemon-serves-transfer.md
.review/findings/otp-4b-data-plane.md
.review/findings/otp-5-daemon-as-source.md
.review/findings/otp-5b-2-pull-data-plane-resize.md
.review/findings/otp-5b-source-responder-data-plane.md
.review/findings/otp-6a-session-filters.md
.review/findings/otp-6b-session-mirror.md
.review/findings/otp-7a-resume-in-stream.md
.review/findings/otp-7b-1-resume-data-plane.md
.review/findings/otp-7b-2-fault-summary.md
.review/findings/otp-8-fallback-byte-carrier.md
.review/findings/otp-9a-session-client-surface.md
.review/findings/otp-9b-delegated-session-reroute.md
.review/findings/rec-1-recent-persistence.md
.review/findings/rec-2-clear-recent.md
.review/findings/rec-3-tui-clear-recent.md
.review/findings/rec-4-clear-recent-confirm.md
.review/findings/relay-1-subpath-double-join.md
.review/findings/retry-wait1-classifier-loop.md
.review/findings/retry-wait2-cli-wiring.md
.review/findings/sf-1-tripwire-harness.md
.review/findings/sf-2-shape-correction-resize.md
.review/findings/tui-key-dispatch-press-only-filter.md
.review/findings/ue-r2-1a.md
.review/findings/ue-r2-1b.md
.review/findings/ue-r2-1c.md
.review/findings/ue-r2-1d.md
.review/findings/ue-r2-1e.md
.review/findings/ue-r2-1f.md
.review/findings/ue-r2-1g.md
.review/findings/ue-r2-1h.md
.review/findings/ue-r2-2.md
.review/findings/w1-2-data-socket-policy-helper.md
.review/findings/w1-3-tcp-keepalive-honesty.md
.review/findings/w1-4-accept-token-constants.md
.review/findings/w2-1-delete-warmup-machinery.md
.review/findings/w2-2-stream-ladder-owner.md
.review/findings/w3-1-memory-aware-buffer-pool.md
.review/findings/w4-1-abortondrop-family.md
.review/findings/w4-2-delete-push-upload-channel.md
.review/findings/w4-3-daemon-disconnect-racing.md
.review/findings/w4-4-blocking-work-off-runtime.md
.review/findings/w4-5-supports-cancellation-flip.md
.review/findings/w5-1-log-backend.md
.review/findings/w5-2-retry-classifier-consolidation.md
.review/findings/w6-1-progress-event-contract.md
.review/findings/w6-2-progress-residue-verify.md
.review/findings/w7-4-hash-reader-helper.md
.review/findings/w7-6-default-port-pub.md
.review/findings/w8-1b-zero-copy-fast-eval.md
.review/findings/w9-1-ungate-windows-tests.md
.review/findings/w9-2-revive-root-tests.md
.review/findings/w9-3-test-harness-builder.md
.review/findings/w9-4-readonly-enforcement-tests.md
.review/findings/w9-5-jobs-lifecycle-e2e.md
.review/findings/win-1-push-needlist-separators.md
.review/findings/windows-move-tree-hang.md

exec
/bin/zsh -lc 'git diff-tree --no-commit-id --name-status -r 8f4fbf9 && git status --short --untracked-files=all' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jGAsISDm' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LqmOQVvT' (errno=Operation not permitted)
A	.review/findings/otp-12a-zoey-harness.md
M	docs/plan/OTP12_ACCEPTANCE_RUN.md
A	scripts/bench_otp12_zoey.sh
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HeEZam5g' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-E0cRkqA3' (errno=Operation not permitted)
?? .review/results/otp-12a.codex.md

exec
/bin/zsh -lc 'git ls-remote origin HEAD && git rev-parse refs/remotes/origin/master 2>/dev/null || true && git rev-parse HEAD' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-231lHhuN' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jjOBI8bW' (errno=Operation not permitted)
fatal: unable to access 'http://q:3000/michael/blit_v2.git/': Could not resolve host: q
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-KUaiuz4Y' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ES3bdJWV' (errno=Operation not permitted)
8f4fbf9a7cacacdf22e6f57530cef2e91fb7351c

exec
/bin/zsh -lc "git show 8f4fbf9:docs/plan/OTP12_ACCEPTANCE_RUN.md | rg -n '"'^#{1,4} |D1|D2|D5|D6|ABBA|INCOMPLETE|1'"\\.10|valid-run|valid pair|pairs|runs\\.csv|verdict'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-BFrlaIAh' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1PKi2XDE' (errno=Operation not permitted)
1:# otp-12 — symmetric-rig acceptance run (design)
13:precedent, REVIEW.md §otp). The verdict WALK is otp-13 and belongs to the
17:## Why this doc
30:## What otp-12 must produce (plan anchors)
44:## Current state (verified at HEAD `ce36da3`)
71:  swap BOTH ends together (matched pairs)**.
79:  otp-12 arm**; skippy gets fresh staging (D6).
91:## Rigs and what each anchors
95:| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
96:| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
103:## Design decisions
105:### D1 — matched-pair interleaved A/B (build identity is the axis)
108:order `A,B,B,A,A,B,B,A` (ABBA per pair-of-pairs — each arm leads half the
109:pairs, so arm never confounds with within-pair position on the stateful
112:`delegated` (rig D). Interleaving is the verdict method, not a nicety:
131:### D2 — verdict arithmetic (what the evidence computes; the owner declares)
140:position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
142:`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
146:  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
159:  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
162:  `max(delegated, direct)/min ≤ 1.10`.
165:  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
182:### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
224:### D4 — delegated cells = delegated-vs-direct parity (rig D)
251:Carrier: TCP is the verdict carrier; one secondary grpc pair
261:### D5 — three self-contained scripts; the frozen baselines stay frozen
271:  nonzero exit voids the interleave pair per the D2 valid-run rule — a
276:`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
277:(`valid` = the PAIR's fate under the D2 valid-run rule — an
281:`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
282:(medians over valid runs only — the D2 valid-run rule)
283:`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
308:### D6 — staging per host
325:### D7 — matrix size and session budget
338:## Staging (sub-slices; each commit through the codex loop)
355:  the plan-level verdict matrix assembling every comparison row
362:## Evidence layout
366:tables, stability, methodology deltas, reproduction), `runs.csv`,
367:`summary.csv`, `verdicts.csv`, `drain-outcomes.txt`, `staging-manifest.txt`
371:## Known gaps / risks
376:  Defender at its normal state). D2's discriminator computation is the
394:## Open questions — RESOLVED (owner, 2026-07-12; D-2026-07-12-1)
398:  `min(old_push, old_pull) × 1.10` bar only by a discriminator-attributed

exec
/bin/zsh -lc 'git diff 8f4fbf9''^ 8f4fbf9 -- docs/plan/OTP12_ACCEPTANCE_RUN.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-D04TV662' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ijhGjZdx' (errno=Operation not permitted)
diff --git a/docs/plan/OTP12_ACCEPTANCE_RUN.md b/docs/plan/OTP12_ACCEPTANCE_RUN.md
index e2d2a84..3e3f1d7 100644
--- a/docs/plan/OTP12_ACCEPTANCE_RUN.md
+++ b/docs/plan/OTP12_ACCEPTANCE_RUN.md
@@ -273,7 +273,10 @@ old scripts, both recorded sharp edges:
 - **Multi-token flags ride an array**, not an unquoted scalar.
 
 CSV schema (all rigs):
-`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain`
+`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
+(`valid` = the PAIR's fate under the D2 valid-run rule — an
+individually-clean run whose partner voided reads `no`; amended at the
+12a harness slice)
 `summary.csv`:
 `cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
 (medians over valid runs only — the D2 valid-run rule)

exec
/bin/zsh -lc 'git show 8f4fbf9:.review/findings/otp-12a-zoey-harness.md | nl -ba' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-NcEXhDTB' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Byami0Au' (errno=Operation not permitted)
     1	# otp-12a — zoey converge-up harness (interleaved old-vs-new)
     2	
     3	**Plan**: `docs/plan/OTP12_ACCEPTANCE_RUN.md` (Active, owner 2026-07-12),
     4	sub-slice 12a, harness half. The recorded-run half follows on the rig
     5	(needs the owner's fresh go for daemon runs on zoey + zoey out of
     6	maintenance).
     7	**Status**: implemented, codex review pending.
     8	
     9	## What
    10	
    11	`scripts/bench_otp12_zoey.sh` — the otp-2 verdict matrix ({large, small,
    12	mixed} × {push, pull} × {tcp, grpc} = 12 comparisons) rerun as
    13	matched-pair interleaved A/B: arm old = pinned `e757dcc` pair (Mac client
    14	staged at `$MAC_WORK/bins/blit-e757dcc`, zoey's kept 2026-07-10 daemon),
    15	arm new = the run commit's pair (local release build + freshly zigbuilt
    16	musl daemon staged beside the old one). Per-direction converge-up only
    17	(D-2026-07-05-1); verdicts computed against BOTH references (same-session
    18	old arm AND the committed `docs/bench/otp2-baseline-2026-07-10/summary.csv`
    19	medians), per design D2.
    20	
    21	## Approach
    22	
    23	Methodology functions carried verbatim from `bench_otp2_baseline.sh`
    24	(wall-clock windows, self-timed destination flushes, drain-then-purge
    25	ordering, fixture recipes, ControlMaster mux). New mechanics per the
    26	design doc: ABBA counterbalanced pair order (F5); pair-void-and-re-run
    27	valid-run rule with a 2×RUNS attempt cap and INCOMPLETE surfacing (F7);
    28	blit exit codes captured with per-run logs under `$OUT_DIR/blit-logs/`
    29	(the old harness swallowed them); daemon lifecycle parameterized by arm
    30	with swap-only-on-arm-change (untimed) plus a stale-daemon refusal
    31	(otp-2w F2 posture, new on this rig); binary provenance recorded to
    32	`staging-manifest.txt` (sha256 all four binaries — the OLD pair predates
    33	the handshake, so provenance is the staging record; the NEW pair's smoke
    34	transfer doubles as its build-identity check via D-2026-07-05-2);
    35	`PREFLIGHT_ONLY=1` mode (no daemon start, nothing timed); summary +
    36	verdict computation in one python3 pass (macOS ships bash 3.2 — no
    37	associative arrays anywhere).
    38	
    39	## Files
    40	
    41	- `scripts/bench_otp12_zoey.sh` (new; self-contained by design D5 — the
    42	  frozen `bench_otp2_baseline.sh` is untouched).
    43	- `docs/plan/OTP12_ACCEPTANCE_RUN.md` — D5 `runs.csv` schema gains the
    44	  `valid` column (pair-fate under the D2 rule; one-line amendment).
    45	
    46	## Tests
    47	
    48	- `bash -n` clean. shellcheck not installed on this machine (recorded
    49	  here rather than claimed).
    50	- No crates/proto/Cargo changes anywhere in otp-12
    51	  (`git diff --stat ce36da3..HEAD -- crates proto Cargo.toml Cargo.lock`
    52	  is empty); the suite stands at the recorded 1484 green from otp-11b.
    53	  A fresh gate run at this tree confirmed fmt + clippy pass and showed
    54	  no test failures.
    55	- The harness itself is verified by the probe/recorded-run discipline on
    56	  the rig (otp-2 precedent): the recorded-run half commits the evidence.
    57	
    58	## Known gaps
    59	
    60	- Not yet executed against the rig — PREFLIGHT_ONLY and the full matrix
    61	  both need zoey (maintenance 2026-07-11) and the owner's fresh daemon
    62	  go. First live session may surface busybox/ssh quirks the otp-2 script
    63	  did not (pgrep availability, sha256sum path).
    64	- Old-arm provenance rests on the staging record + sha256 manifest, not
    65	  a handshake (pre-handshake binaries) — accepted residual risk per the
    66	  design doc.
    67	- The escalation rule (straddle + spread > 25% → RUNS=8 fresh session)
    68	  is manual by design, not automated in the script.
    69	- `meta.csv` (pairs-attempted/completeness) is a working file consumed
    70	  by the verdict pass; the committed evidence carries its content via
    71	  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
    72	  rows.

exec
/bin/zsh -lc "git show 8f4fbf9:scripts/bench_otp12_zoey.sh | nl -ba | sed -n '285,470p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uSK53yNn' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-9M6kawkO' (errno=Operation not permitted)
   285	    for w in large small mixed; do
   286	        if zssh "test -d '$MODULE_ROOT/pull_src_$w/src_$w'"; then
   287	            log "  pull_src_$w already staged (kept from a prior session)"
   288	        else
   289	            "$NEW_BLIT" copy "$MAC_WORK/src_$w" "${REMOTE}pull_src_$w/" --yes \
   290	                > "$OUT_DIR/blit-logs/stage_$w.log" 2>&1 \
   291	                || die "staging pull_src_$w failed (blit-logs/stage_$w.log)"
   292	            log "  staged pull_src_$w"
   293	        fi
   294	    done
   295	}
   296	
   297	# --- Timed runs ---------------------------------------------------------
   298	CSV="$OUT_DIR/runs.csv"
   299	echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
   300	META="$OUT_DIR/meta.csv"
   301	echo "cell,pairs_attempted,complete" > "$META"
   302	
   303	RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
   304	
   305	timed_push_run() {   # arm cell rid src [flags...]; fresh dest per run
   306	    local arm="$1" cell="$2" rid="$3" src="$4"; shift 4
   307	    local blit start end rc=0
   308	    blit=$(arm_blit "$arm")
   309	    ensure_daemon "$arm"
   310	    drop_caches "${cell}_${arm}-$rid"
   311	    start=$(now_ms)
   312	    "$blit" copy "$src" "${REMOTE}push_${SESSION_TAG}_${cell}_${arm}_${rid}/" --yes "$@" \
   313	        > "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.log" 2>&1 || rc=$?
   314	    end=$(now_ms)
   315	    RUN_FLUSH=$(sync_dest_ms)   # durable at dest, self-timed
   316	    RUN_MS=$(( end - start + RUN_FLUSH ))
   317	    RUN_EXIT=$rc
   318	    RUN_VALID=yes
   319	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   320	}
   321	
   322	timed_pull_run() {   # arm cell rid remote_src [flags...]; fresh dest per run
   323	    local arm="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
   324	    local blit start end rc=0
   325	    blit=$(arm_blit "$arm")
   326	    ensure_daemon "$arm"
   327	    rm -rf "$MAC_WORK/dst_pull"
   328	    mkdir -p "$MAC_WORK/dst_pull"
   329	    drop_caches "${cell}_${arm}-$rid"
   330	    start=$(now_ms)
   331	    "$blit" copy "$rsrc" "$MAC_WORK/dst_pull" --yes "$@" \
   332	        > "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.log" 2>&1 || rc=$?
   333	    end=$(now_ms)
   334	    RUN_FLUSH=$(fsync_tree_ms "$MAC_WORK/dst_pull")   # durable, self-timed
   335	    RUN_MS=$(( end - start + RUN_FLUSH ))
   336	    RUN_EXIT=$rc
   337	    RUN_VALID=yes
   338	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   339	}
   340	
   341	run_comparison() {   # cell kind src_or_remote [flags...]
   342	    local cell="$1" kind="$2" src="$3"; shift 3
   343	    local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
   344	    log "=== $cell (interleaved old/new, ABBA, $RUNS pairs) ==="
   345	    while (( valid < RUNS && attempts < max_attempts )); do
   346	        attempts=$(( attempts + 1 ))
   347	        # ABBA: odd slots run old first, even slots run new first.
   348	        local order pair_valid=yes arm rid
   349	        if (( slot % 2 )); then order="old new"; else order="new old"; fi
   350	        local row_old="" row_new=""
   351	        for arm in $order; do
   352	            rid="s${slot}a${attempts}"
   353	            if [[ "$kind" == push ]]; then
   354	                timed_push_run "$arm" "$cell" "$rid" "$src" "$@"
   355	            else
   356	                timed_pull_run "$arm" "$cell" "$rid" "$src" "$@"
   357	            fi
   358	            [[ "$RUN_VALID" == yes ]] || pair_valid=no
   359	            local row="$cell,$arm,$(arm_sha "$arm"),mac,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
   360	            if [[ "$arm" == old ]]; then row_old="$row"; else row_new="$row"; fi
   361	            log "  $cell/$arm slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
   362	        done
   363	        # The valid column reflects the PAIR's fate (design F7): an
   364	        # individually-clean run whose partner voided does not count.
   365	        echo "$row_old,$pair_valid" >> "$CSV"
   366	        echo "$row_new,$pair_valid" >> "$CSV"
   367	        if [[ "$pair_valid" == yes ]]; then
   368	            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   369	        else
   370	            log "  $cell: pair at slot $slot VOIDED — re-running the slot"
   371	        fi
   372	    done
   373	    if (( valid < RUNS )); then
   374	        echo "$cell,$attempts,no" >> "$META"
   375	        log "  $cell INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
   376	    else
   377	        echo "$cell,$attempts,yes" >> "$META"
   378	    fi
   379	}
   380	
   381	# --- Verdicts (design D2: both references must pass) --------------------
   382	compute_verdicts() {
   383	    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
   384	import csv, sys
   385	runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
   386	rows = list(csv.DictReader(open(runs_p)))
   387	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   388	base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}
   389	
   390	by_arm = {}
   391	voided = {}
   392	for r in rows:
   393	    key = (r["cell"], r["arm"])
   394	    if r["valid"] == "yes":
   395	        by_arm.setdefault(key, []).append(int(r["ms"]))
   396	    else:
   397	        voided[key] = voided.get(key, 0) + 1
   398	
   399	def median(v):
   400	    v = sorted(v)
   401	    n = len(v)
   402	    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
   403	
   404	with open(summary_p, "w") as f:
   405	    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
   406	    for (cell, arm) in sorted(by_arm):
   407	        v = by_arm[(cell, arm)]
   408	        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   409	        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
   410	                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")
   411	
   412	def bar_pass(new, ref):   # new <= ref * 1.10, integer-exact
   413	    return 10 * new <= 11 * ref
   414	
   415	with open(verdicts_p, "w") as f:
   416	    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
   417	    for cell in sorted({c for (c, _) in by_arm}):
   418	        if meta[cell]["complete"] != "yes" or (cell, "new") not in by_arm or (cell, "old") not in by_arm:
   419	            f.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
   420	            continue
   421	        new_m = median(by_arm[(cell, "new")])
   422	        old_m = median(by_arm[(cell, "old")])
   423	        ref_m = base.get(cell)
   424	        p1 = bar_pass(new_m, old_m)
   425	        f.write(f"{cell},converge,new,old_session,{new_m},{old_m},"
   426	                f"{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
   427	        if ref_m is None:
   428	            f.write(f"{cell},converge,new,old_committed,{new_m},,,1.10,NO-REFERENCE\n")
   429	            f.write(f"{cell},converge,new,combined,{new_m},{old_m},,1.10,"
   430	                    f"{'FAIL-SAME-SESSION' if not p1 else 'NO-REFERENCE'}\n")
   431	            continue
   432	        p2 = bar_pass(new_m, ref_m)
   433	        f.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},"
   434	                f"{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
   435	        combined = ("PASS" if p1 and p2
   436	                    else "FAIL-REFERENCE-DRIFT" if p1
   437	                    else "FAIL-SAME-SESSION" if p2
   438	                    else "FAIL-BOTH")
   439	        f.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
   440	PYEOF
   441	}
   442	
   443	# --- Matrix -------------------------------------------------------------
   444	main() {
   445	    preflight
   446	    write_manifest
   447	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   448	        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   449	        exit 0
   450	    fi
   451	    BUILD_LINE="old=$OLD_SHA new=$NEW_SHA"
   452	    log "session $SESSION_TAG  $BUILD_LINE  client: $(uname -m) macOS  daemon host: $ZOEY_HOST"
   453	
   454	    gen_fixtures
   455	    smoke_pair old
   456	    smoke_pair new
   457	    stage_pull_sources
   458	
   459	    local w
   460	    for w in large small mixed; do
   461	        run_comparison "push_tcp_${w}"  push "$MAC_WORK/src_$w"
   462	        run_comparison "push_grpc_${w}" push "$MAC_WORK/src_$w" --force-grpc
   463	        run_comparison "pull_tcp_${w}"  pull "${REMOTE}pull_src_$w/src_$w/"
   464	        run_comparison "pull_grpc_${w}" pull "${REMOTE}pull_src_$w/src_$w/" --force-grpc
   465	    done
   466	
   467	    stop_daemon
   468	    compute_verdicts
   469	
   470	    log ""

exec
/bin/zsh -lc "rg -n -C 4 'otp-12|OTP12|8f4fbf9' docs/STATE.md REVIEW.md .agents/state.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/STATE.md-32-  - **otp-2 `[x]` (both halves).** zoey = PER-DIRECTION reference;
docs/STATE.md-33-    Mac↔Windows = cross-direction rig (otp-2w). Harnesses
docs/STATE.md-34-    `scripts/bench_otp2{,w}_baseline.sh`, evidence
docs/STATE.md-35-    `docs/bench/otp2{,w}-baseline-2026-07-10/README.md`. Key reading:
docs/STATE.md:36:    old push trails old pull on BOTH rigs — otp-12's interleaved
docs/STATE.md-37-    old-vs-new discriminates code cost from platform write-path cost.
docs/STATE.md-38-  - **otp-10 `[x]` CLOSED (a, b-1/2, c-1/2)** — verb cutover + THE
docs/STATE.md-39-    CUTOVER DELETION: one chokepoint per verb shape (`blit_app
docs/STATE.md-40-    run_remote_push`/`run_remote_pull`), ONE args→compare mapping,
--
docs/STATE.md-76-   the only work item until it ships**: slices otp-1..13 through the
docs/STATE.md-77-   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
docs/STATE.md-78-   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
docs/STATE.md-79-   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
docs/STATE.md:80:   c-1/2), **otp-11 (a + b)** `[x]`. Current: **otp-12 (the
docs/STATE.md-81-   symmetric-rig acceptance run — initiator/verb invariance A/B +
docs/STATE.md-82-   every cell ≤ the better old direction + noise)** — needs the rigs
docs/STATE.md-83-   (Blocked below); then otp-13 (owner checklist walk).
docs/STATE.md-84-2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
docs/STATE.md-85-   Shipped (zero-copy resolved — D-2026-07-05-3). Optional follow-ups
docs/STATE.md:86:   largely absorbed by otp-2/otp-12's rig matrices; skippy env facts
docs/STATE.md-87-   moved to Blocked → Rig availability.
docs/STATE.md-88-3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
docs/STATE.md-89-   resumes/re-derives after ONE_TRANSFER_PATH ships.
docs/STATE.md-90-4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
--
docs/STATE.md-135-
docs/STATE.md-136-## Blocked / waiting (all owner declarations; checkpoints are owner-only)
docs/STATE.md-137-
docs/STATE.md-138-- **Rig availability (owner, 2026-07-10, verified by ssh)**: for the
docs/STATE.md:139:  otp-12 matrix — remote↔remote (delegated) uses the Windows box
docs/STATE.md-140-  (`michael@10.1.10.173`) + TrueNAS `skippy` (`admin@skippy`,
docs/STATE.md-141-  x86_64; existing test folder `/mnt/generic-pool/video/blit-bin/`
docs/STATE.md-142-  with July binaries + bench.toml; /tmp and /home are noexec there);
docs/STATE.md-143-  skippy also available for Mac↔Linux cells "if needed" (owner).
--
docs/STATE.md-192-- **2026-07-12 (45th, this session)** — **otp-11 CLOSED WHOLE (11a
docs/STATE.md-193-  route + journal-hole addendum + 11b deletion, four codex rounds;
docs/STATE.md-194-  suite 1488 → 1484 with the ≥1483 floor met by real pins; the
docs/STATE.md-195-  separate local orchestration no longer exists)**. In-flight: none;
docs/STATE.md:196:  tree clean. **Next**: otp-12 (rig-gated, Blocked) → otp-13.
docs/STATE.md-197-- **2026-07-11 (44th)** — otp-10c closed (relay removal + the cutover
docs/STATE.md-198-  deletion); suite 1605 → 1488. Owner ask pending: `725aa07` snapshot.
docs/STATE.md-199-- **2026-07-11 (43rd)** — otp-10a/10b closed; verb cutover complete.
docs/STATE.md-200-- *(42nd and earlier pruned to the cap — see DEVLOG 2026-07-06..12.)*

exec
/bin/zsh -lc "git show 8f4fbf9:docs/plan/OTP12_ACCEPTANCE_RUN.md | nl -ba | sed -n '1,330p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-VZLefwBo' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uVW3vm9D' (errno=Operation not permitted)
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
   181	
   182	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
   183	
   184	For a FIXED data direction the two initiators are:
   185	
   186	- **Mac→Windows**: arm A = Mac client pushes
   187	  (`blit copy $MAC_WORK/src_<w> $WIN_HOST:9031:/bench/<fresh>/ --yes`);
   188	  arm B = Windows client pulls
   189	  (`blit.exe copy $MAC_HOST:9031:/bench/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   190	- **Windows→Mac**: arm A = Mac client pulls (staged
   191	  `pull_src_<w>/src_<w>/` source, the otp-2w pattern); arm B = Windows
   192	  client pushes the same staged tree as a local path
   193	  (`blit.exe copy D:\blit-test\bench-module\pull_src_<w>\src_<w> $MAC_HOST:9031:/bench/<fresh>/ --yes`).
   194	
   195	New plumbing this requires, each keyed by ROLE not verb:
   196	
   197	1. **A daemon on the Mac** (new build only): config written like the rig
   198	   scripts do today (`[daemon] bind/port/no_mdns` + `[[module]] name =
   199	   "bench"` pointing at `$MAC_MODULE_ROOT`, **default `$MAC_WORK`
   200	   itself** — the module exports the exact fixture trees arm A pushes,
   201	   so both initiators read the same physical inodes; no fixture copy or
   202	   move on the Mac (codex design F6)), local launch, pid file,
   203	   stale-refusal, PID-scoped teardown. macOS application firewall must
   204	   admit `blit-daemon` — gated by a preflight smoke transfer from
   205	   Windows, not assumed.
   206	2. **A Windows client** (`blit.exe`, new build, built natively alongside
   207	   the daemon). Its timed window is measured ON Windows —
   208	   `[Diagnostics.Stopwatch]` bracketing the `blit.exe copy` inside one ssh
   209	   invocation, output CRLF-stripped (`tr -cd '0-9'`) — the otp-2w
   210	   self-timed pattern (README §Timing-overhead correction); the ssh
   211	   round-trip cost stays outside the window by construction.
   212	3. **Flush keyed by destination OS, never verb**: dest Windows ⇒ self-timed
   213	   `Write-VolumeCache D`; dest macOS ⇒ the local self-timed per-file fsync
   214	   walk. Cold caches both ends before every run (purge / standby-purge);
   215	   drain keyed by the destination disk (Windows `Get-Counter` loop when D:
   216	   receives; the Mac side has no drain equivalent — recorded decision: Mac
   217	   destination runs rely on `sync` + purge exactly as the recorded otp-2w
   218	   pull cells did).
   219	
   220	Arm A cells run fresh inside the invariance block (interleaved A,B,A,B…) —
   221	block-1 new-arm numbers are NOT reused, so rig-state drift between blocks
   222	cannot masquerade as an initiator effect.
   223	
   224	### D4 — delegated cells = delegated-vs-direct parity (rig D)
   225	
   226	Per data direction, the delegated arm and the direct arm drive the SAME
   227	session code with the same roles on the same endpoints; the only deltas are
   228	who spawns the initiator (daemon vs CLI) and the trigger/progress relay:
   229	
   230	- **skippy→Windows**: delegated = Mac runs
   231	  `blit copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ $WIN_HOST:9031:/bench/<fresh>/ --yes`
   232	  (Windows daemon initiates, DESTINATION role); direct = Windows client
   233	  pulls the same source to the same disk
   234	  (`blit.exe copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   235	- **Windows→skippy**: delegated = the mirror-image Mac command (skippy
   236	  daemon initiates); direct = skippy client pulls from the Windows daemon
   237	  (self-timed `/proc/uptime`-bracketed window over ssh, the zoey pattern).
   238	
   239	Timing: the delegated arm is timed on the Mac around the CLI invocation
   240	(the CLI blocks until the relayed Summary), plus the destination's
   241	self-timed flush — deliberately INCLUDING the trigger RPC + relay overhead
   242	(that is the honest end-to-end cost of delegation; on this LAN the trigger
   243	is sub-ms against multi-second cells). The direct arm is self-timed on the
   244	initiating host plus the same flush. Destination flush: Windows ⇒
   245	`Write-VolumeCache`; skippy ⇒ self-timed `sync` bracketed by
   246	`/proc/uptime` reads in one ssh shell. Cold caches: standby-purge (Windows)
   247	+ `drop_caches` (skippy, root/sudo) both ends every run; drain the
   248	destination disk (Windows counter loop; skippy `/proc/diskstats` quiet-
   249	window loop with a device-regex knob).
   250	
   251	Carrier: TCP is the verdict carrier; one secondary grpc pair
   252	(large × skippy→Windows, both arms) is recorded as a smoke row — carrier
   253	selection reads `SessionOpen.in_stream_bytes`/policy, never role or
   254	initiator (`transfer_session/mod.rs:790,805`), and carrier invariance is
   255	measured properly on rig W.
   256	
   257	Config: BOTH daemons get `[delegation] allow_delegated_pull = true` with
   258	`allowed_source_hosts` naming the peer (each is destination in one
   259	direction); bench modules writable, `delegation_allowed` not narrowed.
   260	
   261	### D5 — three self-contained scripts; the frozen baselines stay frozen
   262	
   263	`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
   264	`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
   265	precedent: duplicate the shape, don't refactor recorded evidence;
   266	`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
   267	old scripts, both recorded sharp edges:
   268	
   269	- **Exit codes are checked**: the old harnesses swallow the blit exit code
   270	  inside the timed window; otp-12 records it per run (`exit` column) and a
   271	  nonzero exit voids the interleave pair per the D2 valid-run rule — a
   272	  failed transfer must never contribute a time.
   273	- **Multi-token flags ride an array**, not an unquoted scalar.
   274	
   275	CSV schema (all rigs):
   276	`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
   277	(`valid` = the PAIR's fate under the D2 valid-run rule — an
   278	individually-clean run whose partner voided reads `no`; amended at the
   279	12a harness slice)
   280	`summary.csv`:
   281	`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
   282	(medians over valid runs only — the D2 valid-run rule)
   283	`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
   284	where `cell` = `<fixture>_<direction>_<carrier>`, `arm` ∈
   285	`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
   286	`initiator` = host name, `kind` ∈ `converge|invariance|delegated|cross`.
   287	
   288	Fixtures: identical shapes to otp-2 (1 GiB large / 10k×4 KiB small /
   289	512 MiB+5k×2 KiB mixed), generated with the existing recipes (BSD vs GNU
   290	`dd` block-size spelling handled per host), staged untimed; pull sources
   291	shared across arms (bytes are bytes — recorded explicitly); every timed
   292	destination is fresh and never-seen (`SESSION_TAG` + arm + run in the
   293	path).
   294	
   295	New env knobs: `MAC_HOST` (the Mac's 10 GbE IP — required, no default),
   296	`MAC_MODULE_ROOT` (default `$MAC_WORK` — see D3), `SKIPPY_SSH` (default
   297	`admin@skippy`), `SKIPPY_HOST`, `SKIPPY_BIN` (default
   298	`/mnt/generic-pool/video/blit-bin`), `SKIPPY_DISK_REGEX`,
   299	`OLD_SHA_ZOEY=e757dcc`, `OLD_SHA_WIN=0f922de`.
   300	
   301	Verification entry point for harness commits (no crates/proto touched; the
   302	cargo gates don't exercise bash): `bash -n` on each script + shellcheck
   303	where installed + `bash scripts/agent/check-docs.sh` + the codex review;
   304	the methodology itself is verified by the probe/recorded-run discipline
   305	(otp-2 precedent) and each script supports `PREFLIGHT_ONLY=1` (run every
   306	preflight check and exit before fixtures).
   307	
   308	### D6 — staging per host
   309	
   310	| host | old arm | new arm |
   311	|------|---------|---------|
   312	| Mac | rebuild client at the pinned sha in a detached worktree → `~/blit-bench-work/bins/blit-<sha>` | `cargo build --release` at the run commit |
   313	| zoey | already staged (`$ZOEY_TEMP/blit-daemon`, `e757dcc` pair kept for otp-12 — machines.md) | `cargo zigbuild --release --target aarch64-unknown-linux-musl` → staged BESIDE the old one as `blit-daemon-<sha>` (never overwrite); everything stays inside `blit-temp/` |
   314	| Windows | copy the detached-checkout exes ASIDE first (`D:\blit-test\bins\0f922de\`) before any checkout movement | fresh git bundle (pushes are owner-gated; origin lags at `6d37a22`) → checkout run commit → native `cargo build --release` (daemon AND `blit.exe` client) → `D:\blit-test\bins\<sha>\` |
   315	| skippy | none (no old baseline; July binaries unusable) | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (static — sidesteps the recorded glibc 2.36 ceiling) → `$SKIPPY_BIN/bins/<sha>/` (pool paths are exec-friendly; `/tmp` and `/home` are noexec) — `blit` + `blit-daemon` |
   316	
   317	Windows daemon-swap mechanics: the active arm's exe is COPIED to the fixed
   318	path `D:\blit-test\bins\active\blit-daemon.exe` and launched from there —
   319	one program-scoped firewall rule total (the rule is exe-path-scoped;
   320	sha-named dirs keep provenance, the copy log records each swap). Launch
   321	stays WMI `Win32_Process.Create` + stale-refusal + PID-scoped teardown
   322	(otp-2w README §Host plumbing). A staging manifest (sha256 of every binary
   323	on every host) is recorded in each evidence README.
   324	
   325	### D7 — matrix size and session budget
   326	
   327	| rig | comparisons | timed runs | est. wall |
   328	|-----|------------:|-----------:|----------:|
   329	| Z converge-up | 12 (3 fixtures × 2 dirs × 2 carriers) | 96 | 1.5–2.5 h (drains dominate) |
   330	| W converge-up | 12 | 96 | ~1.5 h |

exec
/bin/zsh -lc "git show 8f4fbf9:scripts/bench_otp12_zoey.sh | nl -ba | sed -n '440,510p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-xhcIx2JG' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-bSXm9DSq' (errno=Operation not permitted)
   440	PYEOF
   441	}
   442	
   443	# --- Matrix -------------------------------------------------------------
   444	main() {
   445	    preflight
   446	    write_manifest
   447	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   448	        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   449	        exit 0
   450	    fi
   451	    BUILD_LINE="old=$OLD_SHA new=$NEW_SHA"
   452	    log "session $SESSION_TAG  $BUILD_LINE  client: $(uname -m) macOS  daemon host: $ZOEY_HOST"
   453	
   454	    gen_fixtures
   455	    smoke_pair old
   456	    smoke_pair new
   457	    stage_pull_sources
   458	
   459	    local w
   460	    for w in large small mixed; do
   461	        run_comparison "push_tcp_${w}"  push "$MAC_WORK/src_$w"
   462	        run_comparison "push_grpc_${w}" push "$MAC_WORK/src_$w" --force-grpc
   463	        run_comparison "pull_tcp_${w}"  pull "${REMOTE}pull_src_$w/src_$w/"
   464	        run_comparison "pull_grpc_${w}" pull "${REMOTE}pull_src_$w/src_$w/" --force-grpc
   465	    done
   466	
   467	    stop_daemon
   468	    compute_verdicts
   469	
   470	    log ""
   471	    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
   472	    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
   473	    log ""
   474	    log "=== VERDICTS (design D2: PASS needs BOTH references) ==="
   475	    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
   476	    log "runs: $CSV"
   477	}
   478	
   479	SESSION_TAG=$(date +%H%M%S).$$
   480	trap 'stop_daemon; sweep_push_dirs' EXIT
   481	main "$@"

exec
/bin/zsh -lc 'git show 8f4fbf9:docs/bench/otp2-baseline-2026-07-10/summary.csv | nl -ba && git show 8f4fbf9:docs/bench/otp2-baseline-2026-07-10/runs.csv 2>/dev/null | head -5 || true' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GwWUhQgu' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7TJ4oyws' (errno=Operation not permitted)
     1	cell,median_ms,avg_ms,best_ms
     2	push_tcp_large,2702,2652,2384
     3	push_grpc_large,4510,4487,4177
     4	pull_tcp_large,1744,1719,1541
     5	pull_grpc_large,2585,2624,2535
     6	push_tcp_small,4263,4152,3252
     7	push_grpc_small,5217,5164,4964
     8	pull_tcp_small,2784,2761,2662
     9	pull_grpc_small,4188,4245,4074
    10	push_tcp_mixed,2070,2051,1794
    11	push_grpc_mixed,3889,3865,3393
    12	pull_tcp_mixed,1401,1477,1375
    13	pull_grpc_mixed,2222,2225,2167
cell,run,ms
push_tcp_large,1,2812
push_tcp_large,2,2593
push_tcp_large,3,2384
push_tcp_large,4,2819

exec
/bin/zsh -lc 'rg -n "build_id|BUILD_ID|version" crates/blit-cli crates/blit-core/build.rs crates/blit-core/src | head -120' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/build.rs:90:    // same-build, so cross-version instability is fine — different
crates/blit-core/src/remote/retry.rs:9://! Interrupted/WouldBlock as retryable — the exact inversions the
crates/blit-core/src/remote/retry.rs:132:                local_build_id: String::new(),
crates/blit-core/src/remote/retry.rs:133:                peer_build_id: String::new(),
crates/blit-cli/src/jobs.rs:664:        "version": state.version,
crates/blit-cli/src/jobs.rs:679:        state.version,
crates/blit-cli/src/scan.rs:13:    version: Option<String>,
crates/blit-cli/src/scan.rs:39:                version: s.properties.get("version").cloned(),
crates/blit-cli/src/scan.rs:81:        if let Some(version) = service.properties.get("version") {
crates/blit-cli/src/scan.rs:82:            println!("  Version: {}", version);
crates/blit-cli/src/transfers/mod.rs:187:    // JSON output. Version dropped — `blit --version` is the right place
crates/blit-cli/src/transfers/remote.rs:606:            local_build_id: String::new(),
crates/blit-cli/src/transfers/remote.rs:607:            peer_build_id: String::new(),
crates/blit-core/src/remote/transfer/data_plane.rs:769:    /// like the pre-c-1b version: bytes are copied, no
crates/blit-cli/src/diagnostics.rs:177:        "blit_version": env!("CARGO_PKG_VERSION"),
crates/blit-cli/src/diagnostics.rs:202:        "  version     : {}",
crates/blit-cli/src/diagnostics.rs:203:        v["blit_version"].as_str().unwrap_or("?")
crates/blit-core/src/remote/transfer/operation_spec.rs:11://! `FileFilter`, capabilities validated, `spec_version` accepted or
crates/blit-core/src/remote/transfer/operation_spec.rs:15://! validate the spec version, etc., instead of every call site
crates/blit-core/src/remote/transfer/operation_spec.rs:37:/// Highest `spec_version` we know how to interpret. Bumped whenever the
crates/blit-core/src/remote/transfer/operation_spec.rs:51:/// into a usable `FileFilter`, and validates the spec version up front.
crates/blit-core/src/remote/transfer/operation_spec.rs:55:/// the conversion rules.
crates/blit-core/src/remote/transfer/operation_spec.rs:96:    /// Returns `Err` for unsupported spec versions, malformed filter
crates/blit-core/src/remote/transfer/operation_spec.rs:101:        // Spec version: accept exact match for now. We have no
crates/blit-core/src/remote/transfer/operation_spec.rs:103:        // versions are a programming error rather than a wire
crates/blit-core/src/remote/transfer/operation_spec.rs:105:        if spec.spec_version != SUPPORTED_SPEC_VERSION {
crates/blit-core/src/remote/transfer/operation_spec.rs:107:                "unsupported TransferOperationSpec spec_version {} (expected {})",
crates/blit-core/src/remote/transfer/operation_spec.rs:108:                spec.spec_version,
crates/blit-core/src/remote/transfer/operation_spec.rs:308:        spec_version: SUPPORTED_SPEC_VERSION,
crates/blit-core/src/remote/transfer/operation_spec.rs:356:        assert_eq!(spec.spec_version, SUPPORTED_SPEC_VERSION);
crates/blit-core/src/remote/transfer/operation_spec.rs:486:            spec_version: SUPPORTED_SPEC_VERSION,
crates/blit-core/src/remote/transfer/operation_spec.rs:521:    fn unsupported_version_rejected() {
crates/blit-core/src/remote/transfer/operation_spec.rs:523:        spec.spec_version = 99;
crates/blit-core/src/remote/transfer/operation_spec.rs:525:        assert!(err.to_string().contains("spec_version 99"));
crates/blit-core/src/buffer.rs:29:/// sysinfo 0.38 reports **bytes** (not KiB — an earlier version of
crates/blit-core/src/path_posix.rs:191:    /// about double-conversion.
crates/blit-cli/tests/diagnostics_dump.rs:44:    assert!(v["blit_version"].is_string(), "blit_version present");
crates/blit-core/src/perf_predictor.rs:27:/// version forces `load()` to reset and rebuild from clean
crates/blit-core/src/perf_predictor.rs:30:/// `PerformancePredictor::load` resets state on version mismatch
crates/blit-core/src/perf_predictor.rs:199:    version: u32,
crates/blit-core/src/perf_predictor.rs:207:            version: STATE_VERSION,
crates/blit-core/src/perf_predictor.rs:559:/// drift. The version-mismatch reset is the load-time invariant
crates/blit-core/src/perf_predictor.rs:569:        if state.version != STATE_VERSION {
crates/blit-core/src/perf_predictor.rs:593:    /// two can't drift — a regression that broke version
crates/blit-core/src/perf_predictor.rs:638:            schema_version: crate::perf_history::CURRENT_SCHEMA_VERSION,
crates/blit-core/src/perf_predictor.rs:1079:    fn schema_version_mismatch_resets_state_on_load() {
crates/blit-core/src/perf_predictor.rs:1086:            "version": 1,
crates/blit-core/src/perf_predictor.rs:1094:        // exercise the version-check logic by parsing then reloading.
crates/blit-core/src/perf_predictor.rs:1097:        let post = if parsed.version != STATE_VERSION {
crates/blit-core/src/perf_predictor.rs:1102:        assert_eq!(post.version, STATE_VERSION);
crates/blit-core/src/perf_predictor.rs:1275:    /// GPT explicit ask: predictor state version bump means any
crates/blit-core/src/perf_predictor.rs:1283:    fn state_version_bumped_for_r56_invalidation() {
crates/blit-core/src/perf_predictor.rs:1291:    /// file with the previous version + a phony profile, load it
crates/blit-core/src/perf_predictor.rs:1294:    /// version must reset to a fresh state.
crates/blit-core/src/perf_predictor.rs:1296:    fn load_resets_state_on_version_mismatch() {
crates/blit-core/src/perf_predictor.rs:1304:        fake_state.version = 2;
crates/blit-core/src/perf_predictor.rs:1317:        // Sanity: the bytes parse as a PredictorState with version 2.
crates/blit-core/src/perf_predictor.rs:1320:        assert_eq!(parsed.version, 2);
crates/blit-core/src/perf_predictor.rs:1324:        // actually exercises version invalidation. Pre-fix this
crates/blit-core/src/perf_predictor.rs:1327:        // version check.
crates/blit-core/src/perf_predictor.rs:1335:        assert_eq!(predictor.state.version, STATE_VERSION);
crates/blit-core/src/perf_predictor.rs:1339:    /// with a profile loads intact (no version mismatch).
crates/blit-core/src/perf_predictor.rs:1341:    fn load_preserves_state_when_version_matches() {
crates/blit-core/src/perf_predictor.rs:1345:        // STATE_VERSION-version, which is the current shipped value.
crates/blit-core/src/perf_predictor.rs:1360:        assert_eq!(predictor.state.version, STATE_VERSION);
crates/blit-core/src/perf_predictor.rs:1364:            "load() must preserve profiles when version matches"
crates/blit-cli/tests/remote_tcp_fallback.rs:137:/// 2,000-file version timed out at 120 s (run 27429395227). Windows
crates/blit-core/src/mdns.rs:140:    properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
crates/blit-core/src/mdns.rs:323:        let s = service(&[("version", "0.1.0")]);
crates/blit-core/src/mdns.rs:345:        let s = service(&[("version", "0.1.0")]);
crates/blit-cli/Cargo.toml:3:version = "0.1.0"
crates/blit-cli/Cargo.toml:17:tokio = { version = "1", features = ["full"] }
crates/blit-cli/Cargo.toml:19:clap = { version = "4", features = ["derive"] }
crates/blit-cli/Cargo.toml:26:chrono = { version = "0.4", default-features = false, features = ["clock"] }
crates/blit-cli/Cargo.toml:30:serde = { version = "1.0", features = ["derive"] }
crates/blit-cli/Cargo.toml:32:sysinfo = { version = "0.38", default-features = false, features = ["disk"] }
crates/blit-cli/Cargo.toml:36:serde = { version = "1.0", features = ["derive"] }
crates/blit-cli/Cargo.toml:40:tokio-stream = { version = "0.1", features = ["net"] }
crates/blit-core/src/perf_history.rs:20:/// Current schema version for PerformanceRecord.
crates/blit-core/src/perf_history.rs:23:/// version field deserialize as version 0 thanks to `#[serde(default)]`.
crates/blit-core/src/perf_history.rs:26:///   0 - implicit (records written before versioning was added)
crates/blit-core/src/perf_history.rs:27:///   1 - added schema_version field
crates/blit-core/src/perf_history.rs:131:/// The `schema_version` field tracks the format version for migration support.
crates/blit-core/src/perf_history.rs:132:/// See [`CURRENT_SCHEMA_VERSION`] for the version history.
crates/blit-core/src/perf_history.rs:136:    pub schema_version: u32,
crates/blit-core/src/perf_history.rs:207:            schema_version: CURRENT_SCHEMA_VERSION,
crates/blit-core/src/perf_history.rs:267:/// Migrate a record from an older schema version to the current version.
crates/blit-core/src/perf_history.rs:269:/// Returns the record with `schema_version` set to `CURRENT_SCHEMA_VERSION`.
crates/blit-core/src/perf_history.rs:271:/// as version-gated transformations.
crates/blit-core/src/perf_history.rs:273:    // v0 → v1: no field changes; v1 just stamped the version field.
crates/blit-core/src/perf_history.rs:288:    if record.schema_version < 2 {
crates/blit-core/src/perf_history.rs:297:    record.schema_version = CURRENT_SCHEMA_VERSION;
crates/blit-core/src/perf_history.rs:333:/// Rewrite the history file, migrating all records to the current schema version.
crates/blit-core/src/perf_history.rs:498:        // A record without schema_version (pre-versioning format)
crates/blit-core/src/perf_history.rs:503:        r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"mirror","source_fs":"apfs","dest_fs":"apfs","file_count":5,"total_bytes":512,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":true,"workers":2},"fast_path":"tiny","planner_duration_ms":10,"transfer_duration_ms":100,"stall_events":0,"error_count":0,"tar_shard_tasks":1,"tar_shard_files":5,"tar_shard_bytes":512,"raw_bundle_tasks":0,"raw_bundle_files":0,"raw_bundle_bytes":0,"large_tasks":0,"large_bytes":0}"#
crates/blit-core/src/perf_history.rs:510:        assert_eq!(record.schema_version, 0);
crates/blit-core/src/perf_history.rs:519:        assert_eq!(record.schema_version, 1);
crates/blit-core/src/perf_history.rs:525:    fn migrate_record_stamps_current_version() {
crates/blit-core/src/perf_history.rs:528:        assert_eq!(old.schema_version, 0);
crates/blit-core/src/perf_history.rs:531:        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
crates/blit-core/src/perf_history.rs:550:        // Both should be migrated to current version
crates/blit-core/src/perf_history.rs:551:        assert_eq!(records[0].schema_version, CURRENT_SCHEMA_VERSION);
crates/blit-core/src/perf_history.rs:552:        assert_eq!(records[1].schema_version, CURRENT_SCHEMA_VERSION);
crates/blit-core/src/perf_history.rs:575:    fn new_record_has_current_version() {
crates/blit-core/src/perf_history.rs:598:        assert_eq!(record.schema_version, CURRENT_SCHEMA_VERSION);
crates/blit-core/src/perf_history.rs:658:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/perf_history.rs:672:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":"null_sink","planner_duration_ms":5,"transfer_duration_ms":2,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/transfer_session/mod.rs:59:/// Belt-and-braces wire-shape version, bumped on any change to the
crates/blit-core/src/transfer_session/mod.rs:132:/// This build's session identity: `<crate version>+<git sha>[.dirty]`
crates/blit-core/src/transfer_session/mod.rs:135:pub fn session_build_id() -> &'static str {
crates/blit-core/src/transfer_session/mod.rs:143:    pub build_id: String,
crates/blit-core/src/transfer_session/mod.rs:144:    pub contract_version: u32,
crates/blit-core/src/transfer_session/mod.rs:150:            build_id: session_build_id().to_string(),
crates/blit-core/src/transfer_session/mod.rs:151:            contract_version: CONTRACT_VERSION,
crates/blit-core/src/transfer_session/mod.rs:280:    pub local_build_id: String,
crates/blit-core/src/transfer_session/mod.rs:281:    pub peer_build_id: String,
crates/blit-core/src/transfer_session/mod.rs:309:            local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs:310:            peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs:385:            local_build_id: err.peer_build_id,
crates/blit-core/src/transfer_session/mod.rs:386:            peer_build_id: err.local_build_id,

exec
/bin/zsh -lc 'rg -n "fn shutdown|shutdown_signal|ctrl_c|SIGTERM|serve_with_shutdown|reuse_port" crates/blit-daemon crates/blit-core | head -160' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
exec
/bin/zsh -lc "git show --format=fuller --stat 8f4fbf9 && git log -1 --format='%H %P %s' 8f4fbf9" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-BK4fZvpT' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-aE1H72Hp' (errno=Operation not permitted)
commit 8f4fbf9a7cacacdf22e6f57530cef2e91fb7351c
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 12 01:01:10 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 12 01:01:10 2026 -0400

    otp-12a: zoey converge-up harness (interleaved old-vs-new, ABBA, both-reference verdicts)
    
    bench_otp12_zoey.sh: otp-2 matrix as matched-pair A/B (old=e757dcc pair,
    new=run commit), ABBA counterbalance (F5), pair-void valid-run rule with
    2xRUNS cap + INCOMPLETE (F7), exit codes captured, stale-daemon refusal,
    sha256 staging manifest, PREFLIGHT_ONLY mode, python3 summary+verdicts
    against both references (D2). runs.csv schema +valid column (doc amended).
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

 .review/findings/otp-12a-zoey-harness.md |  72 +++++
 docs/plan/OTP12_ACCEPTANCE_RUN.md        |   5 +-
 scripts/bench_otp12_zoey.sh              | 481 +++++++++++++++++++++++++++++++
 3 files changed, 557 insertions(+), 1 deletion(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XCzCbajM' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-H6yx1IKb' (errno=Operation not permitted)
8f4fbf9a7cacacdf22e6f57530cef2e91fb7351c ae498d3eac17e5dbb5702364dc3e377fd662af70 otp-12a: zoey converge-up harness (interleaved old-vs-new, ABBA, both-reference verdicts)

exec
/bin/zsh -lc "sed -n '120,165p' crates/blit-core/src/transfer_session/mod.rs && sed -n '150,220p' crates/blit-cli/src/diagnostics.rs && rg -n \"session_build_id\\(\" crates" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/// 65_536 × 32 B = 2 MiB of hashes. A partial with more blocks than
/// this degrades to the empty list — the contract's full-transfer
/// fallback (plan D1) — never an oversized frame.
const MAX_RESUME_BLOCK_HASHES: u64 = 65_536;

/// Does a partial of `dst_len` bytes get a real hash list, or the empty
/// full-transfer fallback (cap rationale above)? Pure, so the cap is
/// unit-testable without a multi-GiB fixture.
fn resume_hash_list_fits(dst_len: u64, block_size: usize) -> bool {
    dst_len.div_ceil(block_size.max(1) as u64) <= MAX_RESUME_BLOCK_HASHES
}

/// This build's session identity: `<crate version>+<git sha>[.dirty]`
/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
/// "unknown" when git was unavailable at compile time.
pub fn session_build_id() -> &'static str {
    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
}

/// The identity this end presents in `SessionHello`. Defaults to the
/// real compile-time identity; tests inject mismatches.
#[derive(Debug, Clone)]
pub struct HelloConfig {
    pub build_id: String,
    pub contract_version: u32,
}

impl Default for HelloConfig {
    fn default() -> Self {
        Self {
            build_id: session_build_id().to_string(),
            contract_version: CONTRACT_VERSION,
        }
    }
}

/// Which handshake part this end plays. Orthogonal to role: all four
/// initiator/role combinations run the same state machine (contract
/// §Invariants 3).
pub enum SessionEndpoint {
    /// This end opened the transport; it sends `SessionOpen`.
    /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
    Initiator { open: Box<SessionOpen> },
    /// This end answers `SessionOpen` with `SessionAccept`. Daemon
    /// module/path/read-only validation attaches here at otp-4.
    Responder,
/// rsync destination resolution, filesystem caps, disk space — without
/// reading source. One invocation → a single pasteable blob.
///
/// The per-endpoint snapshot helpers (`endpoint_snapshot`,
/// `endpoint_display`, `same_device`) live in
/// `blit_app::diagnostics::dump`; the rsync-resolution helpers
/// (`source_is_contents`, `dest_is_container`,
/// `resolve_destination`) live in `blit_app::transfers::resolution`.
/// Both sets are imported directly at the top of this file; this
/// function orchestrates them.
pub fn run_diagnostics_dump(args: &DiagnosticsDumpArgs) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let raw_dst = parse_transfer_endpoint(&args.destination)?;
    let pre_resolve_dst = raw_dst.clone();
    let resolved_dst = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);

    let source_contents_mode = source_is_contents(&args.source);
    let dest_is_container_flag = dest_is_container(&args.destination, &pre_resolve_dst);

    let src_json = endpoint_snapshot(&args.source, &src_endpoint);
    let dst_json = endpoint_snapshot(&args.destination, &resolved_dst);
    let pre_resolve_json = endpoint_display(&pre_resolve_dst);
    let resolved_display = endpoint_display(&resolved_dst);

    let same_device_result = same_device(&src_endpoint, &resolved_dst);

    let output = json!({
        "blit_version": env!("CARGO_PKG_VERSION"),
        "invocation": std::env::args().collect::<Vec<_>>(),
        "source": src_json,
        "destination": dst_json,
        "rsync_resolution": {
            "source_is_contents": source_contents_mode,
            "destination_is_container": dest_is_container_flag,
            "pre_resolve_destination": pre_resolve_json,
            "resolved_destination": resolved_display,
            "resolution_changed": pre_resolve_json != resolved_display,
        },
        "same_device": same_device_result,
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_dump_human(&output);
    }
    Ok(())
}

fn print_dump_human(v: &Value) {
    println!("blit diagnostics dump");
    println!(
        "  version     : {}",
        v["blit_version"].as_str().unwrap_or("?")
    );
    if let Some(invocation) = v["invocation"].as_array() {
        let joined: Vec<&str> = invocation.iter().filter_map(|s| s.as_str()).collect();
        println!("  invocation  : {}", joined.join(" "));
    }
    println!();
    println!("Source");
    print_endpoint_human(&v["source"]);
    println!();
    println!("Destination");
    print_endpoint_human(&v["destination"]);
    println!();
    let res = &v["rsync_resolution"];
    println!("Rsync resolution");
    println!(
        "  source_is_contents     : {}",
        res["source_is_contents"].as_bool().unwrap_or(false)
crates/blit-core/src/transfer_session/mod.rs:135:pub fn session_build_id() -> &'static str {
crates/blit-core/src/transfer_session/mod.rs:150:            build_id: session_build_id().to_string(),
crates/blit-core/src/transfer_session/mod.rs:4206:        let id = session_build_id();

exec
/bin/zsh -lc 'rg -n -C 3 "local_build_id|peer_build_id|BUILD_MISMATCH|BuildMismatch|build identity|build_id" crates/blit-core/src/transfer_session crates/blit-cli/src/transfers crates/blit-daemon/src | head -240' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-cli/src/transfers/remote.rs-603-        SessionFault {
crates/blit-cli/src/transfers/remote.rs-604-            code: Code::Internal,
crates/blit-cli/src/transfers/remote.rs-605-            message: "'big.bin' hit EOF with 42 bytes still promised".into(),
crates/blit-cli/src/transfers/remote.rs:606:            local_build_id: String::new(),
crates/blit-cli/src/transfers/remote.rs:607:            peer_build_id: String::new(),
crates/blit-cli/src/transfers/remote.rs-608-            peer_notified: true,
crates/blit-cli/src/transfers/remote.rs-609-            relative_path: Some(path.into()),
crates/blit-cli/src/transfers/remote.rs-610-            io_kind: None,
--
crates/blit-core/src/transfer_session/mod.rs-132-/// This build's session identity: `<crate version>+<git sha>[.dirty]`
crates/blit-core/src/transfer_session/mod.rs-133-/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
crates/blit-core/src/transfer_session/mod.rs-134-/// "unknown" when git was unavailable at compile time.
crates/blit-core/src/transfer_session/mod.rs:135:pub fn session_build_id() -> &'static str {
crates/blit-core/src/transfer_session/mod.rs-136-    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
crates/blit-core/src/transfer_session/mod.rs-137-}
crates/blit-core/src/transfer_session/mod.rs-138-
--
crates/blit-core/src/transfer_session/mod.rs-140-/// real compile-time identity; tests inject mismatches.
crates/blit-core/src/transfer_session/mod.rs-141-#[derive(Debug, Clone)]
crates/blit-core/src/transfer_session/mod.rs-142-pub struct HelloConfig {
crates/blit-core/src/transfer_session/mod.rs:143:    pub build_id: String,
crates/blit-core/src/transfer_session/mod.rs-144-    pub contract_version: u32,
crates/blit-core/src/transfer_session/mod.rs-145-}
crates/blit-core/src/transfer_session/mod.rs-146-
crates/blit-core/src/transfer_session/mod.rs-147-impl Default for HelloConfig {
crates/blit-core/src/transfer_session/mod.rs-148-    fn default() -> Self {
crates/blit-core/src/transfer_session/mod.rs-149-        Self {
crates/blit-core/src/transfer_session/mod.rs:150:            build_id: session_build_id().to_string(),
crates/blit-core/src/transfer_session/mod.rs-151-            contract_version: CONTRACT_VERSION,
crates/blit-core/src/transfer_session/mod.rs-152-        }
crates/blit-core/src/transfer_session/mod.rs-153-    }
--
crates/blit-core/src/transfer_session/mod.rs-275-pub struct SessionFault {
crates/blit-core/src/transfer_session/mod.rs-276-    pub code: session_error::Code,
crates/blit-core/src/transfer_session/mod.rs-277-    pub message: String,
crates/blit-core/src/transfer_session/mod.rs:278:    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
crates/blit-core/src/transfer_session/mod.rs-279-    /// which end is stale (contract §Errors).
crates/blit-core/src/transfer_session/mod.rs:280:    pub local_build_id: String,
crates/blit-core/src/transfer_session/mod.rs:281:    pub peer_build_id: String,
crates/blit-core/src/transfer_session/mod.rs-282-    /// True when the peer already knows about this fault — it sent
crates/blit-core/src/transfer_session/mod.rs-283-    /// the `SessionError` frame itself, or this end already emitted
crates/blit-core/src/transfer_session/mod.rs-284-    /// one. Drivers must not send another.
--
crates/blit-core/src/transfer_session/mod.rs-306-        Self {
crates/blit-core/src/transfer_session/mod.rs-307-            code,
crates/blit-core/src/transfer_session/mod.rs-308-            message: message.into(),
crates/blit-core/src/transfer_session/mod.rs:309:            local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs:310:            peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-311-            peer_notified: false,
crates/blit-core/src/transfer_session/mod.rs-312-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-313-            io_kind: None,
--
crates/blit-core/src/transfer_session/mod.rs-382-                .unwrap_or(session_error::Code::SessionErrorUnspecified),
crates/blit-core/src/transfer_session/mod.rs-383-            message: err.message,
crates/blit-core/src/transfer_session/mod.rs-384-            // The peer reports its view: its "local" is our peer.
crates/blit-core/src/transfer_session/mod.rs:385:            local_build_id: err.peer_build_id,
crates/blit-core/src/transfer_session/mod.rs:386:            peer_build_id: err.local_build_id,
crates/blit-core/src/transfer_session/mod.rs-387-            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs-388-            // Explicit wire presence (codex 7b-2 G1): "" is the valid
crates/blit-core/src/transfer_session/mod.rs-389-            // identity of a single-file-root transfer, not absence.
--
crates/blit-core/src/transfer_session/mod.rs-398-        SessionError {
crates/blit-core/src/transfer_session/mod.rs-399-            code: self.code as i32,
crates/blit-core/src/transfer_session/mod.rs-400-            message: self.message.clone(),
crates/blit-core/src/transfer_session/mod.rs:401:            local_build_id: self.local_build_id.clone(),
crates/blit-core/src/transfer_session/mod.rs:402:            peer_build_id: self.peer_build_id.clone(),
crates/blit-core/src/transfer_session/mod.rs-403-            relative_path: self.relative_path.clone(),
crates/blit-core/src/transfer_session/mod.rs-404-        }
crates/blit-core/src/transfer_session/mod.rs-405-    }
--
crates/blit-core/src/transfer_session/mod.rs-483-/// fires mid-session (the session future is aborted by the select and
crates/blit-core/src/transfer_session/mod.rs-484-/// cannot send it itself — otp-4a codex F1); blit-core stays the one
crates/blit-core/src/transfer_session/mod.rs-485-/// owner of the frame grammar. The build-id fields are left empty:
crates/blit-core/src/transfer_session/mod.rs:486:/// they are only meaningful for `BUILD_MISMATCH`.
crates/blit-core/src/transfer_session/mod.rs-487-pub fn session_error_frame(code: session_error::Code, message: impl Into<String>) -> TransferFrame {
crates/blit-core/src/transfer_session/mod.rs-488-    frame(Frame::Error(SessionError {
crates/blit-core/src/transfer_session/mod.rs-489-        code: code as i32,
crates/blit-core/src/transfer_session/mod.rs-490-        message: message.into(),
crates/blit-core/src/transfer_session/mod.rs:491:        local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs:492:        peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-493-        relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-494-    }))
crates/blit-core/src/transfer_session/mod.rs-495-}
--
crates/blit-core/src/transfer_session/mod.rs-660-async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) -> Result<()> {
crates/blit-core/src/transfer_session/mod.rs-661-    transport
crates/blit-core/src/transfer_session/mod.rs-662-        .send(frame(Frame::Hello(SessionHello {
crates/blit-core/src/transfer_session/mod.rs:663:            build_id: hello.build_id.clone(),
crates/blit-core/src/transfer_session/mod.rs-664-            contract_version: hello.contract_version,
crates/blit-core/src/transfer_session/mod.rs-665-        })))
crates/blit-core/src/transfer_session/mod.rs-666-        .await?;
--
crates/blit-core/src/transfer_session/mod.rs-679-        }
crates/blit-core/src/transfer_session/mod.rs-680-    };
crates/blit-core/src/transfer_session/mod.rs-681-
crates/blit-core/src/transfer_session/mod.rs:682:    if peer_hello.build_id != hello.build_id
crates/blit-core/src/transfer_session/mod.rs-683-        || peer_hello.contract_version != hello.contract_version
crates/blit-core/src/transfer_session/mod.rs-684-    {
crates/blit-core/src/transfer_session/mod.rs-685-        let fault = SessionFault {
crates/blit-core/src/transfer_session/mod.rs:686:            code: session_error::Code::BuildMismatch,
crates/blit-core/src/transfer_session/mod.rs-687-            message: format!(
crates/blit-core/src/transfer_session/mod.rs-688-                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
crates/blit-core/src/transfer_session/mod.rs:689:                hello.build_id, hello.contract_version,
crates/blit-core/src/transfer_session/mod.rs:690:                peer_hello.build_id, peer_hello.contract_version,
crates/blit-core/src/transfer_session/mod.rs-691-            ),
crates/blit-core/src/transfer_session/mod.rs:692:            local_build_id: hello.build_id.clone(),
crates/blit-core/src/transfer_session/mod.rs:693:            peer_build_id: peer_hello.build_id.clone(),
crates/blit-core/src/transfer_session/mod.rs-694-            peer_notified: false,
crates/blit-core/src/transfer_session/mod.rs-695-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-696-            io_kind: None,
--
crates/blit-core/src/transfer_session/mod.rs-4202-    }
crates/blit-core/src/transfer_session/mod.rs-4203-
crates/blit-core/src/transfer_session/mod.rs-4204-    #[test]
crates/blit-core/src/transfer_session/mod.rs:4205:    fn build_id_has_version_and_git_components() {
crates/blit-core/src/transfer_session/mod.rs:4206:        let id = session_build_id();
crates/blit-core/src/transfer_session/mod.rs-4207-        let (version, git) = id.split_once('+').expect("build id must be version+git");
crates/blit-core/src/transfer_session/mod.rs-4208-        assert_eq!(version, env!("CARGO_PKG_VERSION"));
crates/blit-core/src/transfer_session/mod.rs-4209-        assert!(!git.is_empty(), "git component must be non-empty");
--
crates/blit-core/src/transfer_session/mod.rs-4245-        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs-4246-            code: session_error::Code::Cancelled,
crates/blit-core/src/transfer_session/mod.rs-4247-            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs:4248:            local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs:4249:            peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-4250-            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs-4251-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-4252-            io_kind: None,
--
crates/blit-core/src/transfer_session/mod.rs-4285-        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs-4286-            code: session_error::Code::Cancelled,
crates/blit-core/src/transfer_session/mod.rs-4287-            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs:4288:            local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs:4289:            peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-4290-            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs-4291-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-4292-            io_kind: None,
--
crates/blit-core/src/transfer_session/mod.rs-4392-    #[test]
crates/blit-core/src/transfer_session/mod.rs-4393-    fn fault_round_trips_the_wire_shape() {
crates/blit-core/src/transfer_session/mod.rs-4394-        let fault = SessionFault {
crates/blit-core/src/transfer_session/mod.rs:4395:            code: session_error::Code::BuildMismatch,
crates/blit-core/src/transfer_session/mod.rs-4396-            message: "boom".into(),
crates/blit-core/src/transfer_session/mod.rs:4397:            local_build_id: "1.0+aaa".into(),
crates/blit-core/src/transfer_session/mod.rs:4398:            peer_build_id: "1.0+bbb".into(),
crates/blit-core/src/transfer_session/mod.rs-4399-            peer_notified: false,
crates/blit-core/src/transfer_session/mod.rs-4400-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-4401-            io_kind: None,
crates/blit-core/src/transfer_session/mod.rs-4402-        };
crates/blit-core/src/transfer_session/mod.rs-4403-        let wire = fault.to_wire();
crates/blit-core/src/transfer_session/mod.rs-4404-        let back = SessionFault::from_wire(wire);
crates/blit-core/src/transfer_session/mod.rs:4405:        assert_eq!(back.code, session_error::Code::BuildMismatch);
crates/blit-core/src/transfer_session/mod.rs-4406-        assert_eq!(back.message, "boom");
crates/blit-core/src/transfer_session/mod.rs-4407-        // from_wire swaps perspective: the sender's local is our peer.
crates/blit-core/src/transfer_session/mod.rs:4408:        assert_eq!(back.peer_build_id, "1.0+aaa");
crates/blit-core/src/transfer_session/mod.rs:4409:        assert_eq!(back.local_build_id, "1.0+bbb");
crates/blit-core/src/transfer_session/mod.rs-4410-        assert!(back.peer_notified);
crates/blit-core/src/transfer_session/mod.rs-4411-    }
crates/blit-core/src/transfer_session/mod.rs-4412-
--
crates/blit-daemon/src/service/delegated_pull.rs-87-    }
crates/blit-daemon/src/service/delegated_pull.rs-88-    match err.downcast_ref::<SessionFault>().map(|f| f.code) {
crates/blit-daemon/src/service/delegated_pull.rs-89-        Some(
crates/blit-daemon/src/service/delegated_pull.rs:90:            session_error::Code::BuildMismatch
crates/blit-daemon/src/service/delegated_pull.rs-91-            | session_error::Code::ModuleUnknown
crates/blit-daemon/src/service/delegated_pull.rs-92-            | session_error::Code::ReadOnly
crates/blit-daemon/src/service/delegated_pull.rs-93-            | session_error::Code::DelegationRefused

exec
/bin/zsh -lc "sed -n '1,125p' crates/blit-core/build.rs" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
use protoc_bin_vendored::protoc_bin_path;
use std::hash::{BuildHasher, Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

/// Git identity for the same-build session handshake
/// (D-2026-07-05-2, docs/TRANSFER_SESSION.md §Invariants 2). The
/// handshake is the ONLY compatibility gate — no negotiation exists —
/// so an imprecise identity must never let two different builds
/// exact-match (otp-3 codex F1):
///
/// - clean tree  → `<short sha>` — two ends built from the same
///   commit match, per the contract's definition.
/// - dirty tree  → `<short sha>.dirty.<content hash>` — the nonce is
///   a deterministic hash of the porcelain status + diff, so
///   byte-identical dirty trees still match (and no-op rebuilds
///   don't churn the id) while any content difference refuses.
/// - no git      → `unknown.<per-compilation entropy>` — independent
///   compilations can never false-match; one binary deployed to both
///   ends still matches itself.
///
/// Residual window (accepted, reviewed): a first edit to a
/// previously-clean file OUTSIDE blit-core/proto, with no git
/// operation in between, keeps the last sampled identity until the
/// next script trigger. Closing it means watching every workspace
/// source and recompiling the world on any edit anywhere —
/// deliberately not done; see the otp-3 verdict record.
fn git_build_suffix(manifest_dir: &std::path::Path) -> String {
    let run = |args: &[&str]| -> Option<Vec<u8>> {
        let out = Command::new("git")
            .args(args)
            .current_dir(manifest_dir)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(out.stdout)
    };
    let run_str = |args: &[&str]| -> Option<String> {
        run(args).map(|b| String::from_utf8_lossy(&b).trim().to_string())
    };

    let Some(sha) = run_str(&["rev-parse", "--short=12", "HEAD"]).filter(|s| !s.is_empty()) else {
        // No git identity: entropy from a randomly-keyed hasher (no
        // extra deps) so separate compilations get distinct ids.
        let nonce = std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish();
        return format!("unknown.{nonce:016x}");
    };

    // Re-sample identity when git state moves (HEAD/refs/index) or
    // the wire-owning sources change. src/ + proto/ make blit-core
    // edits re-run this script; index catches add/commit/checkout.
    if let Some(git_dir) = run_str(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/refs");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src").display()
    );

    let porcelain = run(&["status", "--porcelain", "-z"]).unwrap_or_default();
    if porcelain.is_empty() {
        return sha;
    }

    // Watch each currently-dirty path so continued edits to it
    // re-run this script and refresh the content nonce.
    if let Some(root) = run_str(&["rev-parse", "--show-toplevel"]) {
        let root = PathBuf::from(root);
        for entry in porcelain.split(|b| *b == 0) {
            // porcelain -z entries: "XY <path>"; renames add a second
            // NUL-separated path record, which parses the same way.
            if entry.len() > 3 {
                let path = String::from_utf8_lossy(&entry[3..]);
                println!(
                    "cargo:rerun-if-changed={}",
                    root.join(path.as_ref()).display()
                );
            }
        }
    }

    // Deterministic content nonce: same dirty content → same id
    // (std's zero-keyed DefaultHasher; same-rustc is implied by
    // same-build, so cross-version instability is fine — different
    // toolchains SHOULD refuse each other).
    let diff = run(&["diff", "HEAD"]).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    porcelain.hash(&mut hasher);
    diff.hash(&mut hasher);
    format!("{sha}.dirty.{:016x}", hasher.finish())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc_path);

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proto_dir = manifest_dir.join("..").join("..").join("proto");
    let proto_file = proto_dir.join("blit.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());
    println!(
        "cargo:rustc-env=BLIT_GIT_SHA={}",
        git_build_suffix(&manifest_dir)
    );

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[proto_file.as_path()], &[proto_dir.as_path()])?;
    Ok(())
}

exec
/bin/zsh -lc 'rg -n -C 5 "FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE|same-session|both references" .review docs/plan/OTP12_ACCEPTANCE_RUN.md | head -260' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/plan/OTP12_ACCEPTANCE_RUN.md-32-1. **Invariance matrix** (criterion 1): per data direction × workload
docs/plan/OTP12_ACCEPTANCE_RUN.md-33-   (large / 10k-small / mixed), wall time initiating from end A vs end B —
docs/plan/OTP12_ACCEPTANCE_RUN.md-34-   push-verb vs pull-verb — within run noise (±10%). Committed as evidence.
docs/plan/OTP12_ACCEPTANCE_RUN.md-35-2. **Converge-up matrix** (criterion 2 / codex F4): every unified cell ≤ the
docs/plan/OTP12_ACCEPTANCE_RUN.md-36-   better of that cell's two old directions + noise (±10%), against the
docs/plan/OTP12_ACCEPTANCE_RUN.md:37:   recorded old-path baselines, confirmed by interleaved same-session
docs/plan/OTP12_ACCEPTANCE_RUN.md-38-   old-vs-new A/B (the otp-2 README's standing prescription for this rig
docs/plan/OTP12_ACCEPTANCE_RUN.md-39-   class).
docs/plan/OTP12_ACCEPTANCE_RUN.md-40-3. **Delegated cells** (owner rig designation, 2026-07-10, STATE Blocked):
docs/plan/OTP12_ACCEPTANCE_RUN.md-41-   remote↔remote on the Windows box + skippy — the delegated trigger must
docs/plan/OTP12_ACCEPTANCE_RUN.md-42-   not cost wall time vs the same session driven directly.
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-137-**Valid-run rule (codex design F7)**: a run with a nonzero blit exit OR an
docs/plan/OTP12_ACCEPTANCE_RUN.md-138-undrained pre-run window VOIDS its whole interleave pair (both arms at
docs/plan/OTP12_ACCEPTANCE_RUN.md-139-that counterbalance position); the pair is re-run — appended at the same
docs/plan/OTP12_ACCEPTANCE_RUN.md-140-position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
docs/plan/OTP12_ACCEPTANCE_RUN.md-141-pair attempts per comparison. At the cap the cell is recorded
docs/plan/OTP12_ACCEPTANCE_RUN.md:142:`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
docs/plan/OTP12_ACCEPTANCE_RUN.md-143-a median over fewer than RUNS valid runs.
docs/plan/OTP12_ACCEPTANCE_RUN.md-144-
docs/plan/OTP12_ACCEPTANCE_RUN.md-145-- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
docs/plan/OTP12_ACCEPTANCE_RUN.md:146:  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
docs/plan/OTP12_ACCEPTANCE_RUN.md-147-  interleaved old arm AND the committed 2026-07-10 baseline median for
docs/plan/OTP12_ACCEPTANCE_RUN.md-148-  that cell (codex design F2: the fixed pre-cutover bar must not be
docs/plan/OTP12_ACCEPTANCE_RUN.md:149:  loosened by a slower old rerun). A cell passing same-session but
docs/plan/OTP12_ACCEPTANCE_RUN.md:150:  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
docs/plan/OTP12_ACCEPTANCE_RUN.md-151-  gets one pre-registered fresh-session re-run; a persisting drift stands
docs/plan/OTP12_ACCEPTANCE_RUN.md-152-  as a recorded failure for the otp-13 walk. **Every unified arm of a
docs/plan/OTP12_ACCEPTANCE_RUN.md-153-  data direction — both initiators on rig W, both blocks — must meet
docs/plan/OTP12_ACCEPTANCE_RUN.md-154-  these bars independently** (codex design F3: the invariance ratio is an
docs/plan/OTP12_ACCEPTANCE_RUN.md-155-  additional constraint, never a substitute ceiling — otherwise
--
.review/results/otp-1-wire-session-contract.codex.md-1141-+
.review/results/otp-1-wire-session-contract.codex.md-1142-+## Errors, cancel, stall
.review/results/otp-1-wire-session-contract.codex.md-1143-+
.review/results/otp-1-wire-session-contract.codex.md-1144-+- `SessionError{code, message, detail}` codes:
.review/results/otp-1-wire-session-contract.codex.md-1145-+  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
.review/results/otp-1-wire-session-contract.codex.md:1146:+  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
.review/results/otp-1-wire-session-contract.codex.md-1147-+  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
.review/results/otp-1-wire-session-contract.codex.md-1148-+  or aborts says why before closing; operators never diagnose from a
.review/results/otp-1-wire-session-contract.codex.md-1149-+  bare stream reset.
.review/results/otp-1-wire-session-contract.codex.md-1150-+- `CancelJob` interop: the responder registers the session in
.review/results/otp-1-wire-session-contract.codex.md-1151-+  `ActiveJobs` at OPEN (same transfer_id contract as today); the
--
.review/results/otp-1-wire-session-contract.codex.md-1480-+    SESSION_ERROR_UNSPECIFIED = 0;
.review/results/otp-1-wire-session-contract.codex.md-1481-+    BUILD_MISMATCH = 1;
.review/results/otp-1-wire-session-contract.codex.md-1482-+    MODULE_UNKNOWN = 2;
.review/results/otp-1-wire-session-contract.codex.md-1483-+    READ_ONLY = 3;
.review/results/otp-1-wire-session-contract.codex.md-1484-+    DELEGATION_REFUSED = 4;
.review/results/otp-1-wire-session-contract.codex.md:1485:+    SCAN_INCOMPLETE = 5;
.review/results/otp-1-wire-session-contract.codex.md-1486-+    PROTOCOL_VIOLATION = 6;
.review/results/otp-1-wire-session-contract.codex.md-1487-+    DATA_PLANE_FAILED = 7;
.review/results/otp-1-wire-session-contract.codex.md-1488-+    CANCELLED = 8;
.review/results/otp-1-wire-session-contract.codex.md-1489-+    INTERNAL = 9;
.review/results/otp-1-wire-session-contract.codex.md-1490-+  }
--
.review/results/otp-1-wire-session-contract.codex.md-1693-   160	
.review/results/otp-1-wire-session-contract.codex.md-1694-   161	## Errors, cancel, stall
.review/results/otp-1-wire-session-contract.codex.md-1695-   162	
.review/results/otp-1-wire-session-contract.codex.md-1696-   163	- `SessionError{code, message, detail}` codes:
.review/results/otp-1-wire-session-contract.codex.md-1697-   164	  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
.review/results/otp-1-wire-session-contract.codex.md:1698:   165	  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
.review/results/otp-1-wire-session-contract.codex.md-1699-   166	  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
.review/results/otp-1-wire-session-contract.codex.md-1700-   167	  or aborts says why before closing; operators never diagnose from a
.review/results/otp-1-wire-session-contract.codex.md-1701-   168	  bare stream reset.
.review/results/otp-1-wire-session-contract.codex.md-1702-   169	- `CancelJob` interop: the responder registers the session in
.review/results/otp-1-wire-session-contract.codex.md-1703-   170	  `ActiveJobs` at OPEN (same transfer_id contract as today); the
--
.review/results/otp-1-wire-session-contract.codex.md-2482-  1325	    SESSION_ERROR_UNSPECIFIED = 0;
.review/results/otp-1-wire-session-contract.codex.md-2483-  1326	    BUILD_MISMATCH = 1;
.review/results/otp-1-wire-session-contract.codex.md-2484-  1327	    MODULE_UNKNOWN = 2;
.review/results/otp-1-wire-session-contract.codex.md-2485-  1328	    READ_ONLY = 3;
.review/results/otp-1-wire-session-contract.codex.md-2486-  1329	    DELEGATION_REFUSED = 4;
.review/results/otp-1-wire-session-contract.codex.md:2487:  1330	    SCAN_INCOMPLETE = 5;
.review/results/otp-1-wire-session-contract.codex.md-2488-  1331	    PROTOCOL_VIOLATION = 6;
.review/results/otp-1-wire-session-contract.codex.md-2489-  1332	    DATA_PLANE_FAILED = 7;
.review/results/otp-1-wire-session-contract.codex.md-2490-  1333	    CANCELLED = 8;
.review/results/otp-1-wire-session-contract.codex.md-2491-  1334	    INTERNAL = 9;
.review/results/otp-1-wire-session-contract.codex.md-2492-  1335	  }
--
.review/results/local-error-telemetry-plan.gpt-verdict.md-56-section (unlike `OTP7_RESUME.md`'s D1-D6 convention it was modeled after).
.review/results/local-error-telemetry-plan.gpt-verdict.md-57-
.review/results/local-error-telemetry-plan.gpt-verdict.md-58-**Adjudication: ACCEPTED (real).** Verified: the doc has an "Open questions" section
.review/results/local-error-telemetry-plan.gpt-verdict.md-59-(Q1-Q5) but no separate "Design decisions" section with D-numbered entries.
.review/results/local-error-telemetry-plan.gpt-verdict.md-60-
.review/results/local-error-telemetry-plan.gpt-verdict.md:61:**Fix**: retargeted both references to "(see Q1 below)", the actual open question
.review/results/local-error-telemetry-plan.gpt-verdict.md-62-covering that fork.
.review/results/local-error-telemetry-plan.gpt-verdict.md-63-
.review/results/local-error-telemetry-plan.gpt-verdict.md-64-## Summary
.review/results/local-error-telemetry-plan.gpt-verdict.md-65-
.review/results/local-error-telemetry-plan.gpt-verdict.md-66-All 3 findings accepted and fixed in `ebb668f`. No findings rejected or deferred.
--
.review/results/otp-4b3-data-plane.fix-review.codex.md-5186-+   210	## Errors, cancel, stall
.review/results/otp-4b3-data-plane.fix-review.codex.md-5187-+   211	
.review/results/otp-4b3-data-plane.fix-review.codex.md-5188-+   212	- `SessionError{code, message}` codes (plus both build ids on
.review/results/otp-4b3-data-plane.fix-review.codex.md-5189-+   213	  BUILD_MISMATCH):
.review/results/otp-4b3-data-plane.fix-review.codex.md-5190-+   214	  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
.review/results/otp-4b3-data-plane.fix-review.codex.md:5191:+   215	  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
.review/results/otp-4b3-data-plane.fix-review.codex.md-5192-+   216	  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
.review/results/otp-4b3-data-plane.fix-review.codex.md-5193-+   217	  or aborts says why before closing; operators never diagnose from a
.review/results/otp-4b3-data-plane.fix-review.codex.md-5194-+   218	  bare stream reset.
.review/results/otp-4b3-data-plane.fix-review.codex.md-5195-+   219	- `CancelJob` interop: the responder registers the session in
.review/results/otp-4b3-data-plane.fix-review.codex.md-5196-+   220	  `ActiveJobs` at OPEN (same transfer_id contract as today); the
--
.review/results/otp-12-design.codex.md-47-codex
.review/results/otp-12-design.codex.md-48-### Findings
.review/results/otp-12-design.codex.md-49-
.review/results/otp-12-design.codex.md-50-- **High** — [OTP12_ACCEPTANCE_RUN.md:93](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:93), [OTP12_ACCEPTANCE_RUN.md:367](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:367) — Rig W is APFS↔NTFS, not the same filesystem class required by acceptance criterion 1. The residue discriminator may explain a miss, but recommending that a criterion-2 numerical failure count as satisfied weakens the parent’s unconditional bar.
.review/results/otp-12-design.codex.md-51-
.review/results/otp-12-design.codex.md:52:- **High** — [OTP12_ACCEPTANCE_RUN.md:131](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:131) — D2 makes the same-session old rerun the hard converge-up reference and demotes the committed pre-cutover baseline to corroboration. A slower old rerun could loosen the fixed acceptance bar; both references must pass.
.review/results/otp-12-design.codex.md-53-
.review/results/otp-12-design.codex.md-54-- **High** — [OTP12_ACCEPTANCE_RUN.md:136](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:136) — Converge-up and invariance tolerances compound. With old-best=100, A=110 passes converge-up and B=121 passes B/A≤1.10, although B violates the required 110 ceiling. Each initiator arm must independently meet criterion 2.
.review/results/otp-12-design.codex.md-55-
.review/results/otp-12-design.codex.md-56-- **Medium** — [OTP12_ACCEPTANCE_RUN.md:326](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:326) — otp-12d schedules acceptance-checkbox edits despite the document stating otp-12 “declares nothing” and otp-13’s owner walk owns the verdict. Checkbox flips must remain in otp-13.
.review/results/otp-12-design.codex.md-57-
--
.review/results/otp-12-design.codex.md-66-125,286
.review/results/otp-12-design.codex.md-67-### Findings
.review/results/otp-12-design.codex.md-68-
.review/results/otp-12-design.codex.md-69-- **High** — [OTP12_ACCEPTANCE_RUN.md:93](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:93), [OTP12_ACCEPTANCE_RUN.md:367](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:367) — Rig W is APFS↔NTFS, not the same filesystem class required by acceptance criterion 1. The residue discriminator may explain a miss, but recommending that a criterion-2 numerical failure count as satisfied weakens the parent’s unconditional bar.
.review/results/otp-12-design.codex.md-70-
.review/results/otp-12-design.codex.md:71:- **High** — [OTP12_ACCEPTANCE_RUN.md:131](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:131) — D2 makes the same-session old rerun the hard converge-up reference and demotes the committed pre-cutover baseline to corroboration. A slower old rerun could loosen the fixed acceptance bar; both references must pass.
.review/results/otp-12-design.codex.md-72-
.review/results/otp-12-design.codex.md-73-- **High** — [OTP12_ACCEPTANCE_RUN.md:136](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:136) — Converge-up and invariance tolerances compound. With old-best=100, A=110 passes converge-up and B=121 passes B/A≤1.10, although B violates the required 110 ceiling. Each initiator arm must independently meet criterion 2.
.review/results/otp-12-design.codex.md-74-
.review/results/otp-12-design.codex.md-75-- **Medium** — [OTP12_ACCEPTANCE_RUN.md:326](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:326) — otp-12d schedules acceptance-checkbox edits despite the document stating otp-12 “declares nothing” and otp-13’s owner walk owns the verdict. Checkbox flips must remain in otp-13.
.review/results/otp-12-design.codex.md-76-
--
.review/results/otp-4a-daemon-serves-transfer.codex.md-456-## Errors, cancel, stall
.review/results/otp-4a-daemon-serves-transfer.codex.md-457-
.review/results/otp-4a-daemon-serves-transfer.codex.md-458-- `SessionError{code, message}` codes (plus both build ids on
.review/results/otp-4a-daemon-serves-transfer.codex.md-459-  BUILD_MISMATCH):
.review/results/otp-4a-daemon-serves-transfer.codex.md-460-  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
.review/results/otp-4a-daemon-serves-transfer.codex.md:461:  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
.review/results/otp-4a-daemon-serves-transfer.codex.md-462-  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
.review/results/otp-4a-daemon-serves-transfer.codex.md-463-  or aborts says why before closing; operators never diagnose from a
.review/results/otp-4a-daemon-serves-transfer.codex.md-464-  bare stream reset.
.review/results/otp-4a-daemon-serves-transfer.codex.md-465-- `CancelJob` interop: the responder registers the session in
.review/results/otp-4a-daemon-serves-transfer.codex.md-466-  `ActiveJobs` at OPEN (same transfer_id contract as today); the
--
.review/results/otp-4a-daemon-serves-transfer.codex.md-11436-   210	## Errors, cancel, stall
.review/results/otp-4a-daemon-serves-transfer.codex.md-11437-   211	
.review/results/otp-4a-daemon-serves-transfer.codex.md-11438-   212	- `SessionError{code, message}` codes (plus both build ids on
.review/results/otp-4a-daemon-serves-transfer.codex.md-11439-   213	  BUILD_MISMATCH):
.review/results/otp-4a-daemon-serves-transfer.codex.md-11440-   214	  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
.review/results/otp-4a-daemon-serves-transfer.codex.md:11441:   215	  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
.review/results/otp-4a-daemon-serves-transfer.codex.md-11442-   216	  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
.review/results/otp-4a-daemon-serves-transfer.codex.md-11443-   217	  or aborts says why before closing; operators never diagnose from a
.review/results/otp-4a-daemon-serves-transfer.codex.md-11444-   218	  bare stream reset.
.review/results/otp-4a-daemon-serves-transfer.codex.md-11445-   219	- `CancelJob` interop: the responder registers the session in
.review/results/otp-4a-daemon-serves-transfer.codex.md-11446-   220	  `ActiveJobs` at OPEN (same transfer_id contract as today); the
--
.review/results/otp-4a-daemon-serves-transfer.codex.md-11534-   159	  delegated otp-9.
.review/results/otp-4a-daemon-serves-transfer.codex.md-11535-
.review/results/otp-4a-daemon-serves-transfer.codex.md-11536-codex
.review/results/otp-4a-daemon-serves-transfer.codex.md-11537-One more edge I’m checking before finalizing: whether the pushed `require_complete_scan` option is silently ignored on the new session path. That option crosses `SessionOpen` in this slice, so it is in scope if old push enforces it.
.review/results/otp-4a-daemon-serves-transfer.codex.md-11538-exec
.review/results/otp-4a-daemon-serves-transfer.codex.md:11539:/usr/bin/zsh -lc 'rg -n "require_complete_scan|scan_complete|ScanIncomplete|SCAN_INCOMPLETE|scan incomplete" crates/blit-daemon/src/service/push crates/blit-core/src/transfer_session crates/blit-core/src/remote/transfer/session_client.rs crates/blit-core/src/remote/push crates/blit-core/src/remote/transfer -S' in /home/michael/dev/Blit
.review/results/otp-4a-daemon-serves-transfer.codex.md-11540- succeeded in 0ms:
.review/results/otp-4a-daemon-serves-transfer.codex.md-11541-crates/blit-core/src/remote/transfer/session_client.rs:37:    pub require_complete_scan: bool,
.review/results/otp-4a-daemon-serves-transfer.codex.md-11542-crates/blit-core/src/remote/transfer/session_client.rs:46:            require_complete_scan: false,
.review/results/otp-4a-daemon-serves-transfer.codex.md-11543-crates/blit-core/src/remote/transfer/session_client.rs:84:        require_complete_scan: options.require_complete_scan,
.review/results/otp-4a-daemon-serves-transfer.codex.md-11544-crates/blit-core/src/transfer_session/mod.rs:739:    let scan_complete = unreadable
--
.review/results/otp-12-design.gpt-verdict.md-24-evaluation rule is now annotated in the parent (bfb9670). Residual fix in
.review/results/otp-12-design.gpt-verdict.md-25-this commit: parent criterion 1 gains the same instantiation note (the
.review/results/otp-12-design.gpt-verdict.md-26-designated pair + why invariance A/B is valid there: both arms of a pair
.review/results/otp-12-design.gpt-verdict.md-27-share endpoints, so endpoint asymmetry cancels within the pair).
.review/results/otp-12-design.gpt-verdict.md-28-
.review/results/otp-12-design.gpt-verdict.md:29:## F2 (High) — same-session old rerun as THE hard reference; a slower old rerun could loosen the fixed bar
.review/results/otp-12-design.gpt-verdict.md-30-
.review/results/otp-12-design.gpt-verdict.md-31-**ACCEPTED.** D2 rewritten: a clean converge-up PASS now requires the new
.review/results/otp-12-design.gpt-verdict.md:32:arm ≤ ×1.10 against **BOTH** references — the same-session interleaved old
.review/results/otp-12-design.gpt-verdict.md-33-arm AND the committed 2026-07-10 baseline median. A cell that passes
.review/results/otp-12-design.gpt-verdict.md:34:same-session but fails the committed reference is recorded
.review/results/otp-12-design.gpt-verdict.md:35:`FAIL-REFERENCE-DRIFT` and triggers one pre-registered fresh session re-run
.review/results/otp-12-design.gpt-verdict.md-36-for that cell; if the drift persists it stands as a recorded failure for
.review/results/otp-12-design.gpt-verdict.md-37-the otp-13 walk — never silently excused by rig-state drift.
.review/results/otp-12-design.gpt-verdict.md-38-
.review/results/otp-12-design.gpt-verdict.md-39-## F3 (High) — tolerance compounding: arm B could reach 1.21× the old bar
.review/results/otp-12-design.gpt-verdict.md-40-
--
.review/results/otp-12-design.gpt-verdict.md-66-
.review/results/otp-12-design.gpt-verdict.md-67-**ACCEPTED.** D2/D5 rewritten: a run with nonzero exit OR an undrained
.review/results/otp-12-design.gpt-verdict.md-68-window voids its whole interleave PAIR (both arms at that position); the
.review/results/otp-12-design.gpt-verdict.md-69-pair is re-run (appended, same counterbalance position) until RUNS valid
.review/results/otp-12-design.gpt-verdict.md-70-pairs exist, capped at 2×RUNS pair attempts; at the cap the cell is
.review/results/otp-12-design.gpt-verdict.md:71:recorded `INCOMPLETE` with the drain log — surfaced, never a silent pass
.review/results/otp-12-design.gpt-verdict.md-72-or a short median.
.review/results/otp-12-design.gpt-verdict.md-73-
.review/results/otp-12-design.gpt-verdict.md-74-## Fix commit
.review/results/otp-12-design.gpt-verdict.md-75-
.review/results/otp-12-design.gpt-verdict.md-76-fix sha: `92e1d51` (docs-only; check-docs gate green). Related same-day
--
.review/results/otp-9b.gpt-verdict.md-18-`ManifestComplete` arm gated the refusal on `mirror_enabled` alone
.review/results/otp-9b.gpt-verdict.md-19-(mod.rs). This is a SESSION gap, not a delegated one — the old paths'
.review/results/otp-9b.gpt-verdict.md-20-R49-F2 enforcement had no session equivalent, and otp-10's verb cutover
.review/results/otp-9b.gpt-verdict.md-21-would have inherited it too. Fixed at the same abort point as the
.review/results/otp-9b.gpt-verdict.md-22-mirror guard: `open.require_complete_scan && !scan_complete` now
.review/results/otp-9b.gpt-verdict.md:23:refuses with `SCAN_INCOMPLETE` before any transfer. Pinned by
.review/results/otp-9b.gpt-verdict.md-24-`incomplete_scan_refused_when_completeness_required` (scripted source
.review/results/otp-9b.gpt-verdict.md-25-peer, bounded wait); guard proof: disabling the check makes the test
.review/results/otp-9b.gpt-verdict.md-26-fail at its timeout (the destination proceeds instead of refusing).
.review/results/otp-9b.gpt-verdict.md-27-
.review/results/otp-9b.gpt-verdict.md-28-## F2 (High) — mirror pass in one `spawn_blocking` outlives cancellation
--
.review/results/sf-1-tripwire-harness.gpt-verdict.md-18-2. **High — "clean" verdict without full matrix coverage** (summary
.review/results/sf-1-tripwire-harness.gpt-verdict.md-19-   awk). ACCEPTED. Skipped tools and all-failed rows (e.g. rclone auth)
.review/results/sf-1-tripwire-harness.gpt-verdict.md-20-   silently shrank the rival set. Fixed: the expected transport set is
.review/results/sf-1-tripwire-harness.gpt-verdict.md-21-   fixed by the plan (local: blit/rsync/rclone/cp; remote: blit/rsyncd/
.review/results/sf-1-tripwire-harness.gpt-verdict.md-22-   rsync_ssh/rclone_sftp); any expected tool with no successful run in
.review/results/sf-1-tripwire-harness.gpt-verdict.md:23:   a cell marks it `INCOMPLETE (no run: …)` and the run exits 4. Trips
.review/results/sf-1-tripwire-harness.gpt-verdict.md-24-   still take precedence (exit 3). Verified: loopback run without sftp
.review/results/sf-1-tripwire-harness.gpt-verdict.md:25:   auth now shows INCOMPLETE on every remote cell.
.review/results/sf-1-tripwire-harness.gpt-verdict.md-26-
.review/results/sf-1-tripwire-harness.gpt-verdict.md-27-3. **Medium — `SPIN_DAEMONS=0` couldn't run rsyncd cells**. ACCEPTED,
.review/results/sf-1-tripwire-harness.gpt-verdict.md-28-   and adjudication found the flagged gating was the shallow half:
.review/results/sf-1-tripwire-harness.gpt-verdict.md-29-   external mode was broken for every remote cell, because tools write
.review/results/sf-1-tripwire-harness.gpt-verdict.md-30-   daemon-relative paths (module root) while the harness prepared
--
.review/results/sf-1-tripwire-harness.gpt-verdict.md-57-
.review/results/sf-1-tripwire-harness.gpt-verdict.md-58-- `bash -n` clean; `bash scripts/agent/check-docs.sh` OK.
.review/results/sf-1-tripwire-harness.gpt-verdict.md-59-- Local-only run: exit 3 (cp trips blit on tiny local copies on the
.review/results/sf-1-tripwire-harness.gpt-verdict.md-60-  dev box — tripwire working as designed; rig verdicts are sf-4).
.review/results/sf-1-tripwire-harness.gpt-verdict.md-61-- Loopback spun-daemons run: remote cells + scale probe green,
.review/results/sf-1-tripwire-harness.gpt-verdict.md:62:  INCOMPLETE correctly reported for the auth-less rclone_sftp cells,
.review/results/sf-1-tripwire-harness.gpt-verdict.md-63-  no stray daemons, session dir removed.
.review/results/sf-1-tripwire-harness.gpt-verdict.md-64-- Loopback external-daemons run (`SPIN_DAEMONS=0`): rsyncd probe found
.review/results/sf-1-tripwire-harness.gpt-verdict.md-65-  the daemon, all REL-prefixed paths worked, stream counts read from
.review/results/sf-1-tripwire-harness.gpt-verdict.md-66-  `BLITD_LOG`, teardown removed only the session dir.
.review/results/sf-1-tripwire-harness.gpt-verdict.md-67-- Cargo suite unaffected by the fix commit (scripts + docs only).
--
.review/results/ue-r2-1c.gpt-verdict.md-14-
.review/results/ue-r2-1c.gpt-verdict.md-15-1. **`engine/mirror.rs:32` — Low — Accepted.** The engine referenced
.review/results/ue-r2-1c.gpt-verdict.md-16-   `crate::orchestrator::LocalMirrorDeleteScope` — a type the engine
.review/results/ue-r2-1c.gpt-verdict.md-17-   itself now owns and the orchestrator merely re-exports. Runtime
.review/results/ue-r2-1c.gpt-verdict.md-18-   behavior identical, but it inverts the engine→adapter layering and
.review/results/ue-r2-1c.gpt-verdict.md:19:   makes future adapter refactors brittle. **Fix**: both references in
.review/results/ue-r2-1c.gpt-verdict.md-20-   `mirror.rs` switched to `super::options::LocalMirrorDeleteScope`;
.review/results/ue-r2-1c.gpt-verdict.md-21-   the same sweep found one more back-reference codex did not flag
.review/results/ue-r2-1c.gpt-verdict.md-22-   (`history.rs:130` test import of `crate::orchestrator::
.review/results/ue-r2-1c.gpt-verdict.md-23-   TransferOutcome`) — fixed to the engine-local path for the same
.review/results/ue-r2-1c.gpt-verdict.md-24-   reason. `grep -rn "crate::orchestrator" crates/blit-core/src/engine/`
--
.review/findings/otp-2-symmetric-baseline.md-55-**Known gaps**:
.review/findings/otp-2-symmetric-baseline.md-56-
.review/findings/otp-2-symmetric-baseline.md-57-- **Push-cell residual spread ±10–20%** (one outlier per ~4 runs)
.review/findings/otp-2-symmetric-baseline.md-58-  even with drain+sync — inherent to the pool's tiered write path.
.review/findings/otp-2-symmetric-baseline.md-59-  Mitigation recorded as an otp-12 prescription: interleaved
.review/findings/otp-2-symmetric-baseline.md:60:  same-session A/B for push verdicts; the `731023b` binaries stay
.review/findings/otp-2-symmetric-baseline.md-61-  staged on zoey for that.
.review/findings/otp-2-symmetric-baseline.md-62-- **OPEN QUESTION routed to the owner** (STATE): the plan's
.review/findings/otp-2-symmetric-baseline.md-63-  cross-direction acceptance bar ("every cell ≤ the better of that
.review/findings/otp-2-symmetric-baseline.md-64-  cell's two old directions + noise") presupposes hardware-symmetric
.review/findings/otp-2-symmetric-baseline.md-65-  endpoints; this rig's write ends are asymmetric (SSD vs pool), so
--
.review/results/otp-5b-source-responder-data-plane.codex.md-875-## Errors, cancel, stall
.review/results/otp-5b-source-responder-data-plane.codex.md-876-
.review/results/otp-5b-source-responder-data-plane.codex.md-877-- `SessionError{code, message}` codes (plus both build ids on
.review/results/otp-5b-source-responder-data-plane.codex.md-878-  BUILD_MISMATCH):

codex
The core mechanics are mostly recognizable, but two edge paths around incomplete comparisons already look unsafe: a partially complete cell can still receive a short median, while a zero-valid cell can disappear from verdicts entirely. I’m validating those against the exact D2 wording and checking for similarly evidence-corrupting paths before I finalize severity.
exec
/bin/zsh -lc '/bin/bash --version | head -1
/bin/bash -n scripts/bench_otp12_zoey.sh
command -v shellcheck || true
git show --check --oneline 8f4fbf9' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Yc6tGrcJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-btKNDudj' (errno=Operation not permitted)
8f4fbf9 otp-12a: zoey converge-up harness (interleaved old-vs-new, ABBA, both-reference verdicts)

exec
/bin/zsh -lc "git show 8f4fbf9:docs/bench/otp2-baseline-2026-07-10/README.md | rg -n -C 4 'build|commit|SHA|731023b|e757dcc|staged|old' | head -180 && git log --oneline --decorate -15 8f4fbf9"'^' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-8tQkNpYo' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uVkJtV51' (errno=Operation not permitted)
3-**Status**: Recorded. **Scope (load-bearing)**: this rig's endpoints
4-are hardware-asymmetric (client SSD vs daemon pool), and
5-D-2026-07-05-1 rules that cross-direction performance comparisons are
6-valid **only on symmetric endpoints**. This dataset therefore anchors
7:**per-direction converge-up** (new ≤ old, same cell) and cannot anchor
8-the otp-12 acceptance bar's cross-direction half — the owner
9-designated the Mac↔Windows pair for that
10-(`docs/bench/otp2w-baseline-2026-07-10/`).
11-
12:**Build**: `e757dcc` binaries both ends (client macOS arm64 release;
13-daemon static aarch64-musl via
14:`cargo zigbuild --release --target aarch64-unknown-linux-musl`); the
15-recorded run used the harness as of `ceea6ed`+review fixes.
16-**Harness**: `scripts/bench_otp2_baseline.sh` (methodology in its
17-header; the probe CSVs here are the evidence that earned each rule).
18-
--
22-  APFS SSD (`~/blit-bench-work`, never `/tmp`).
23-- **Daemon**: `zoey` (UNAS 8 Pro; Alpine-based aarch64, 4 slow cores,
24-  16 GiB RAM; 8-spindle pool ~102 TiB behind a mirrored-NVMe write
25-  tier). All daemon-side state confined to the owner's `blit-temp`
26:  folder (standing safety rule).
27-- **Link**: Thunderbolt 10GbE (Mac `en9`) ↔ zoey (10.1.10.206), same
28-  /24, ~0.4 ms RTT, endpoint pinned by IP.
29-- Owner-stated and confirmed: zoey's CPU cannot saturate the link;
30-  cells are CPU/storage-bound (the reference is per-cell on identical
31-  hardware, not wire-speed).
32-
33:## Verdict-cell results (median of 4 cold, drained, durable runs; ms)
34-
35-| fixture | push tcp | push grpc | pull tcp | pull grpc |
36-|---------|---------:|----------:|---------:|----------:|
37-| large (1 GiB)            | 2702 | 4510 | 1744 | 2585 |
--
46-Sanity: TCP < gRPC in all 12 cells. 1 GiB durable ≈ 3.2 Gbit/s push /
47-4.9 Gbit/s pull. Small files are per-file-cost bound (push ≈ 426
48-µs/file, pull ≈ 278 µs/file on zoey's 4 slow cores — the July skippy
49-diagnosis's per-file-bound shape at a slower constant). Old-pull beats
50:old-push in every cell, ×1.25–×1.75 — but on THESE endpoints that gap
51-is confounded with destination hardware (pool vs SSD), which is
52-exactly why D-2026-07-05-1 excludes cross-direction verdicts here.
53-
54-## Run-to-run stability (this dataset)
--
59-fully stops being stateful; the MEDIAN is the cell statistic
60-precisely because of this, and every run is visible in `runs.csv`.
61-**otp-12 prescription**: on this rig, verdicts (especially push
62-cells) should be confirmed by interleaved same-session A/B
63:(old-build vs new-build alternating), not by absolute comparison
64:alone. The old-path binaries stay staged in zoey's `blit-temp`.
65-
66-## Methodology findings (why the harness looks the way it does)
67-
68-1. **Naive transfer-return timing is a write-cache lottery**
--
113-export ZOEY_HOST=10.1.10.206
114-RUNS=4 ./scripts/bench_otp2_baseline.sh
115-```
116-
117:Requires: the staged same-commit daemon in `$ZOEY_TEMP`, a NOPASSWD
118-sudoers rule for `/usr/sbin/purge` on the client, python3 on the
119-client, and SSH key auth to the daemon host.
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-UElOZHKi' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-kMOTnXFA' (errno=Operation not permitted)
ae498d3 REVIEW: close the stale otp-2 row (closed both halves 2026-07-10; owner go)
260fb26 otp-12 design: flip Draft -> Active (owner yes, 2026-07-12)
ba892d2 otp-12 design: record review verdict + fix sha 92e1d51; DEVLOG [state: skip]
92e1d51 otp-12 design: address review (7 findings)
bfb9670 decision D-2026-07-12-1: otp-12 cross-direction platform-residue cells count as satisfied (owner yes)
045da4a otp-12 design: acceptance-run matrix + harness plan (docs/plan/OTP12_ACCEPTANCE_RUN.md)
ce36da3 otp-11b: record review fix sha 9e810ee; REVIEW row + DEVLOG + STATE (otp-11 closed whole) [state: skip]
9e810ee otp-11b: address review (6 findings)
b1650c4 otp-11b: live-docs sweep (ARCHITECTURE module table, README, WHITEPAPER journal claim) [state: skip]
805e48c otp-11b: delete the local orchestration (the last old path)
0bffa84 otp-11a addendum: record review fix sha 5f1188c; DEVLOG [state: skip]
5f1188c otp-11a addendum: address review (4 findings)
d173691 otp-11a addendum records: REVIEW row + STATE (codex review of d74c1ac pending on quota) [state: skip]
4148705 otp-11a addendum: flake-rate note; gate certification 1491/0 + blit_utils isolated [state: skip]
d74c1ac otp-11a addendum: the old journal fast path was unsound (proven); no-op gate re-baselined

exec
/bin/zsh -lc 'git remote -v' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-UlobVz8n' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-cZDqSDon' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)

collab: Wait
collab: Wait
codex
### Findings

- [scripts/bench_otp12_zoey.sh:404](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:404) — **High** — Incomplete handling is broken: 1–3 valid pairs produce forbidden short medians, while zero-valid cells disappear from `verdicts.csv` instead of emitting `INCOMPLETE`. This also contradicts [the finding record:69](/Users/michael/Dev/blit_v2/.review/findings/otp-12a-zoey-harness.md:69).

- [scripts/bench_otp12_zoey.sh:238](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:238), [scripts/bench_otp12_zoey.sh:286](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:286) — **High** — Fixture and persistent pull-source directories are trusted by existence alone. Interrupted generation/staging leaves partial workloads that later runs silently benchmark, enabling false PASSes.

- [scripts/bench_otp12_zoey.sh:147](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:147), [scripts/bench_otp12_zoey.sh:153](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:153) — **High** — D1/D6 provenance is not enforced. Matching stale or identically dirty binaries pass the handshake but are labeled as current `NEW_SHA`; additionally, hash-command failures inside `echo "$(…)"` are masked by `echo`, permitting blank manifest hashes.

- [scripts/bench_otp12_zoey.sh:188](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:188), [scripts/bench_otp12_zoey.sh:480](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:480) — **High** — The unconditional EXIT trap kills whatever PID appears in the fixed remote pidfile without ownership verification. Preflight refusal or `PREFLIGHT_ONLY` can therefore kill an unrelated or pre-existing process.

- [scripts/bench_otp12_zoey.sh:89](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:89), [scripts/bench_otp12_zoey.sh:425](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:425) — **Medium** — The fixed committed reference is silently overrideable, and the outcome column emits unregistered `FAIL`/`NO-REFERENCE` values. Missing reference coverage does not fail closed with the required vocabulary.

- [scripts/bench_otp12_zoey.sh:312](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:312) — **Medium** — Timed invocations write per-run logs to the client disk, unlike the frozen harness’s `/dev/null`. Arm-dependent output and unflushed log I/O contaminate the measured window.

- [scripts/bench_otp12_zoey.sh:327](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:327) — **Low** — Pulls reuse `$MAC_WORK/dst_pull`, contrary to D5’s explicit never-seen `SESSION_TAG + arm + run` destination rule.

- [scripts/bench_otp12_zoey.sh:78](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:78) — **Low** — `RUNS` is unrestricted: odd values lose equal-lead ABBA counterbalancing, and zero produces empty, nonsensical evidence. Only preregistered 4/8 should be accepted.

- [OTP12_ACCEPTANCE_RUN.md:284](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:284) — **Low** — The amended schema describes `<fixture>_<direction>_<carrier>`, while both the harness and committed reference use `<direction>_<carrier>_<fixture>`.

VERDICT: REQUEST CHANGES
tokens used
107,052
### Findings

- [scripts/bench_otp12_zoey.sh:404](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:404) — **High** — Incomplete handling is broken: 1–3 valid pairs produce forbidden short medians, while zero-valid cells disappear from `verdicts.csv` instead of emitting `INCOMPLETE`. This also contradicts [the finding record:69](/Users/michael/Dev/blit_v2/.review/findings/otp-12a-zoey-harness.md:69).

- [scripts/bench_otp12_zoey.sh:238](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:238), [scripts/bench_otp12_zoey.sh:286](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:286) — **High** — Fixture and persistent pull-source directories are trusted by existence alone. Interrupted generation/staging leaves partial workloads that later runs silently benchmark, enabling false PASSes.

- [scripts/bench_otp12_zoey.sh:147](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:147), [scripts/bench_otp12_zoey.sh:153](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:153) — **High** — D1/D6 provenance is not enforced. Matching stale or identically dirty binaries pass the handshake but are labeled as current `NEW_SHA`; additionally, hash-command failures inside `echo "$(…)"` are masked by `echo`, permitting blank manifest hashes.

- [scripts/bench_otp12_zoey.sh:188](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:188), [scripts/bench_otp12_zoey.sh:480](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:480) — **High** — The unconditional EXIT trap kills whatever PID appears in the fixed remote pidfile without ownership verification. Preflight refusal or `PREFLIGHT_ONLY` can therefore kill an unrelated or pre-existing process.

- [scripts/bench_otp12_zoey.sh:89](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:89), [scripts/bench_otp12_zoey.sh:425](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:425) — **Medium** — The fixed committed reference is silently overrideable, and the outcome column emits unregistered `FAIL`/`NO-REFERENCE` values. Missing reference coverage does not fail closed with the required vocabulary.

- [scripts/bench_otp12_zoey.sh:312](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:312) — **Medium** — Timed invocations write per-run logs to the client disk, unlike the frozen harness’s `/dev/null`. Arm-dependent output and unflushed log I/O contaminate the measured window.

- [scripts/bench_otp12_zoey.sh:327](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:327) — **Low** — Pulls reuse `$MAC_WORK/dst_pull`, contrary to D5’s explicit never-seen `SESSION_TAG + arm + run` destination rule.

- [scripts/bench_otp12_zoey.sh:78](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:78) — **Low** — `RUNS` is unrestricted: odd values lose equal-lead ABBA counterbalancing, and zero produces empty, nonsensical evidence. Only preregistered 4/8 should be accepted.

- [OTP12_ACCEPTANCE_RUN.md:284](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:284) — **Low** — The amended schema describes `<fixture>_<direction>_<carrier>`, while both the harness and committed reference use `<direction>_<carrier>_<fixture>`.

VERDICT: REQUEST CHANGES
