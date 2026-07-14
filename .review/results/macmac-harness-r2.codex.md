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
session id: 019f5fbf-d12d-76a2-8a4e-1010f8f57e8e
--------
user
Review commit 24660ae (run: git show 24660ae). It is ROUND 2 of the Mac<->Mac benchmark instrument. Round 1 (of e1e351d) returned NOT READY with 3 BLOCKER, 6 HIGH, 1 MEDIUM, 1 LOW; all 11 were accepted and this commit is the fix. The adjudication is .review/results/macmac-harness.gpt-verdict.md and the design is docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (now rev 3).

Files: scripts/bench_otp12pf_mac.sh, scripts/otp12pf_mac_verdict.py (new, mechanizes the decision rule), scripts/otp12pf_mac_verdict_test.py (new guard test).

NO DATA HAS BEEN TAKEN YET. This is the last gate before the rig runs. This project has retracted three claims that all traced to a harness that lied while looking correct, so assume it still lies.

Verify, hardest first:

1. Did the round-1 BLOCKERs actually get fixed, or only appear to?
   - BLOCKER 1: does the harness now compute the FULL registered rule end to end -- RIG-VOID, the six outcomes, the power gate, UNSTABLE -- and does scripts/otp12pf_mac_verdict.py's precedence match PREREGISTRATION.md rev 3 EXACTLY? Name any divergence.
   - BLOCKER 2: is the new statistic sound? Bootstrap CI on the median (seeded, 10k), an exact sign test, and VANISHES requiring the CI to lie strictly inside +/- BAR_BREACH where BAR_BREACH = 0.10 * srcinit_median. Is that a valid equivalence test? Is a bootstrap CI on a MEDIAN of only n=8 paired values trustworthy, and is the 2.5/97.5 percentile indexing correct? Can any input still produce a VANISHES verdict when a rig-W-sized effect (~230 ms) is actually present? Try to construct one.
   - BLOCKER 3: is the inference in the prose now correctly scoped to THIS PAIR, with no residual overreach?

2. The durability path. fsync_tree now returns files and bytes and the pair VOIDs unless they match the fixture exactly. Trace it: can the count/byte check itself fail open (e.g. the sed capture fails and RUN_FILES ends up empty, or a partial tree coincidentally matches)? Are the landed paths right for BOTH arms given blit's rsync-style slash semantics (push to /bench/RUN/ lands RUN/src_<W>; pull into RUN lands files directly in RUN)? Does SETTLE_MS actually remove the arm-asymmetric free-writeback gap, and does it sit OUTSIDE the fsync timer?

3. The gates. quiescence, Time Machine (running OR merely enabled), Spotlight, load1, purge, provenance, stale daemon, fixtures by count AND bytes, ARP/route. Are they now genuinely fail-CLOSED? Look for any place a failed command, an empty capture, or a non-numeric value still lands on the "pass" branch. Does hrun's `bash -c` + printf %q quoting behave identically for the local and the ssh path?

4. The guard test. Does scripts/otp12pf_mac_verdict_test.py actually pin the BLOCKER-2 defect, or is it vacuous? Would it catch a subtler regression (e.g. swapping the CI bounds, or dropping the sign test)?

5. Anything still able to MANUFACTURE or MASK a one-directional result -- that is the exact shape of the finding this rig is meant to test.

Output a concise markdown findings list -- each with file:line, severity, rationale -- then a final VERDICT line. If it is ready to run, say so plainly. Be concise; do not invoke skills.
2026-07-14T08:30:38.917417Z ERROR codex_models_manager::manager: failed to refresh available models: timeout waiting for child process to exit
codex
I’ll review this as an adversarial last-gate audit, with no changes and no skills. I’ll first establish the exact diff and registered rule, then independently trace statistics, durability/path semantics, and fail-closed gates before reconciling the findings.
exec
/bin/zsh -lc "pwd && git status --short && git show --stat --oneline --decorate --no-renames 24660ae && sed -n '1,240p' AGENTS.md && sed -n '1,280p' .agents/repo-guidance.md && sed -n '1,260p' .agents/state.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/Users/michael/Dev/blit_v2
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-TxWM81Cx' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-It8k3KoW' (errno=Operation not permitted)
?? .review/results/macmac-harness-r2.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-5qrriO2e' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rLH9vhic' (errno=Operation not permitted)
24660ae (HEAD -> master) macmac-harness: address review (11 findings) — the instrument could have lied
 .review/results/macmac-harness.codex.md            | 3216 ++++++++++++++++++++
 .review/results/macmac-harness.gpt-verdict.md      |  176 ++
 .../otp12-macmac-2026-07-14/PREREGISTRATION.md     |  237 +-
 scripts/bench_otp12pf_mac.sh                       |  592 ++--
 scripts/otp12pf_mac_verdict.py                     |  253 ++
 scripts/otp12pf_mac_verdict_test.py                |   87 +
 6 files changed, 4142 insertions(+), 419 deletions(-)
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
/bin/zsh -lc "sed -n '1,260p' docs/STATE.md && sed -n '1,260p' REVIEW.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-14

- **⛔ THE MAC↔MAC RIG IS NOT CLEARED TO RUN — rev 3 required (codex round 2: 3 BLOCKER, 6 HIGH, 2 LOW; 11/11 accepted, `.review/results/macmac-prereg-r2.gpt-verdict.md`).** No rig time spent. Three things must land first: **(a)** rev 2's inference is STILL overclaimed — a reproduction on these two Macs could be **macOS/APFS or host×role residue**, not a "platform-general layout cost", and a null licenses only "did not reproduce on this pair", NOT "Windows required" (and the "platform-residue escape" it claims to close **does not exist** — the parent says P1 has no escape hatch); **(b)** the power gate is **broken** — `S = max−min` is a range, not an MDE, and codex's counterexample (`d = [0,180,180,190,190,200,200,200]`, 7/8 positive, effect 83% of the 230 ms reference) returns **"VANISHES, powered=yes"**; it needs a real paired **equivalence** test (distribution-free CI on median(d_i); at n=8 the order stats `[d₍₂₎,d₍₇₎]`); **(c)** `compute_verdicts` implements **none** of the registered rule (no control gate, no clustering, no six outcomes — just PASS/FAIL), so a human would apply it after seeing the numbers. **The harness must COMPUTE the verdict; the prose must only describe what the code does.**
- **NEXT ACTION — rev 3 of the Mac↔Mac pre-registration + harness, then run it (Queue 1(ii); the second of the two experiments that come BEFORE any pf code).** Experiment (i), the A-B-B-A MTU run, is **DONE** (pf-0 below). Rig: nagatha `10.1.10.92` ↔ `q` `10.1.10.54`, both 10GbE/9000. Pre-registered (**rev 2**, codex round 1 = 9 findings, **9/9 accepted**): `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`; harness `scripts/bench_otp12pf_mac.sh`. **⚠ CORRECTED 2026-07-14 — it is NOT an H1 discriminator.** The earlier claim here ("reproduces ⇒ **H1 DIES**, H1 accuses the *Windows* accept branch") was **WRONG and is retracted**: H1 accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **the word "Windows" appears nowhere in H1**, and that code runs on macOS too. A Mac↔Mac reproduction is therefore *consistent with* H1, not fatal to it. What the rig **does** answer: **does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost of the destination-initiated layout?** Reproduces ⇒ P1 is **not platform residue** (no Windows peer needed) → the "accept it as platform residue" escape closes and every code-level hypothesis strengthens. Vanishes ⇒ P1 is **pairing-dependent** → platform-agnostic code mechanisms weaken. Either way the hypothesis space moves, which is why it precedes pf-1. **Then `pf-1`** (the HARD GATE), which two pf-0 results now BIND: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor) — pf-1 must **measure its own paired within-session floor and register a resolution check** before grading; (b) **the fast arm is BISTABLE** — grade the run distribution, not the median.
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

- **2026-07-14 (48th)** — **pf-0 ran and MTU is KILLED as a cause of P1**
  (`r = −3.1%`; A-B-B-A on `q`, 256 runs, 0 voided, MSS gate held every session;
  `docs/bench/otp12-jumbo-win-2026-07-13/`). codex NOT READY → **7/7 accepted**
  (`11f0c2a`) — it confirmed every number and killed every *claim* that outran
  them: the run is **not powered** to exclude a *contributing*-size effect
  (46 ms < the 78 ms floor), "P1 is code-shaped" was **not** established (MTU is
  one variable; segment fill unmeasured), and declaring the frozen baseline VOID
  was **not an agent's call**. **The fast arm is BISTABLE** (bimodal `win_init`;
  the mode mixture, not MTU, sets the noise floor) — a pf-1 counterfactual that
  shifts the mixture would fake a recovery. Rig: Time Machine on `q` fired 1 min
  before the run (owner disabled it; **the harness's quiet-gate does not catch
  it**), and three starts died on a **physically flapping `en8`** the owner
  reseated — I chased three deterministic theories and falsified all three.
  **In-flight: none. Rigs clean, Windows MTU 9000, TM still OFF on `q` (owner
  re-enables), 4 + 4 commits unpushed.**
  **NEXT: the MAC↔MAC rig** (Queue 1(ii) — the last experiment before any pf
  code), **then pf-1.** The baseline re-record (D-2026-07-14-1) is a `pf-final`
  prerequisite, not a pf-1 blocker.
- **2026-07-13/14 (47th)** — P1 reproduces on a second Mac (`q`); new bench Mac;
  Windows attrs+ADS bug (D-2026-07-13-3); the robocopy headline was WRONG
  (D-2026-07-13-2); MTU prereg rev 1→4. Full: **DEVLOG 2026-07-14 00:15Z**.
- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig (8/8 PASS
  → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1); **three claims
  retracted, all from unvalidated instruments**. Full: **DEVLOG 2026-07-13 20:00Z**.
- *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*
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
| otp-12a | Zoey converge-up A/B recorded (design `docs/plan/OTP12_ACCEPTANCE_RUN.md` Active — owner flip; D-2026-07-12-1 residue rule). Three codex rounds: design CHANGES REQUIRED 7 findings (6 accepted + 1 overtaken-by-owner-decision); harness REQUEST CHANGES 9/9 accepted (zero false positives); run round FAIL 6/6 accepted (provenance `+sha` form, D2 supersession amendment, drift/gap wording per CSVs). En route: otp-2 daemon provenance corrected (staged pair was dirty `731023b`, not `e757dcc`); zoey I/O-storm diagnosed → per-run dest sweep. Evidence `docs/bench/otp12-zoey-2026-07-12/` (3 sessions incl. aborted storm): **10 PASS; pull_tcp_large FAIL-REFERENCE-DRIFT (rig-side by strongest evidence); push_tcp_small FAIL-SAME-SESSION 1.105** — both carried to the otp-13 walk. | `[x]` | design `045da4a`+`92e1d51`; harness `8f4fbf9`+`50dc135`; run `b2b6901`+`b3729da`+`042c06f`+`6bc9cb6`+`b0ebf73`+fixes `fa18787` |
| otp-12b | Mac↔Windows acceptance session recorded — THE INVARIANCE CRITERION MEASURED: 11/12 cells PASS at 1.003–1.057 (the owner's sentence holds); wm_tcp_mixed FAIL 1.237 (TCP×mixed×destination-initiator — real, block-1-corroborated, code-shaped). Converge 10/12 (push_tcp_small 1.149 FAIL-BOTH — matches zoey's 1.105, second rig; pull_tcp_mixed 1.313 same root). Cross: Win→Mac 6/6 beat the better old direction; Mac→Win gap rows recorded per D-2026-07-12-1 shapes (large unchanged / mixed+grpc_small narrowed / tcp_small widened), adjudication reserved to otp-13. Three codex rounds: harness FAIL 12/12 accepted; run-round FAIL 3/3 accepted (self-adjudication scrubbed); + two found-live fixes (pwsh `$rc:R` scope-parse sentinel; CR-split verdicts). 192 runs, zero voided. Evidence `docs/bench/otp12-win-2026-07-12/`. | `[x]` | harness `d30b1e3`+`772cfe6`+`d3eae58`; run `e21cf84`+`856af64`+`44c2046`+fixes `49dee5c` |
| otp-12c | Rig-D delegated-parity session recorded (netwatch-01↔skippy) + a rig-W re-baseline at the CUTOVER sha `f35702a` (12b measured `e21cf84`, so no committed rig-W evidence existed at the sha the shipped binaries embed). New harness `scripts/bench_otp12_delegated.sh` (plan D4: delegated = Mac CLI triggers `DelegatedPull`, no payload through the Mac; direct = the destination host's own CLI pulls; same session code, roles, data plane, destination disk and flush — only the initiator differs). **Rig D: 7/7 PASS** — RUNS=4 gave 5 PASS / 2 FAIL (`sw_tcp_mixed` 1.119, `ws_tcp_large` 1.129); both FAIL cells met D2's pre-registered escalation trigger (straddle + >25% arm spread) and re-ran at RUNS=8, whose medians govern per the D2 supersession amendment → both PASS (1.035, 1.068), with the wide spread appearing on the *direct* arm too at higher n. 88 timed runs across two sessions, **zero voided pairs**. Rig-W re-baseline: 198 runs, 93 PASS / 12 FAIL / 3 FAIL-SAME-SESSION / 12 RECORDED — `wm_tcp_mixed` invariance **1.300** (12b: 1.237), i.e. the TCP×mixed×dest-initiator cell did NOT wash out at the cutover sha. Three harness bugs found live, each caught by the script's own gates (apostrophes in `:?` messages swallowing assignments — the otp-12b `772cfe6` bug re-made; macOS `$TMPDIR` blowing ssh's 104-byte ControlPath limit; skippy's `drop_caches` needing the exact NOPASSWD grant, whose generic form silently no-op'd → runs would have read WARM). Codex FAIL → **7/7 accepted, 0 rejected**: F1 cold-cache fail-open (HIGH — grant now a hard gate; a failed purge voids the pair); **F2 D2 misread (HIGH — the first draft scoped the escalation amendment to converge-up rows only and so ducked the verdict; the rule says "a comparison", delegated parity included → rig D 7/7 PASS)**; F3 provenance (`proto/` added to the dirty-tree gate; `+sha` no longer substring-matches `+sha.dirty.<hash>` — the otp-12a zoey trap); F4 machine-readable build fields recorded harness HEAD, not the gated binary identity; F5 silent `sync`/drain failures (failed sync → NA → void; a disk regex matching no device is DRAIN-NODEV, not drained); F6 teardown logged "stopped" without verifying (a survivor now exits nonzero); F7 a PASS listed among the FAILs. Codex independently confirmed the otp-12b F5 arm asymmetry does NOT recur and that every committed CSV recomputes exactly. Evidence `docs/bench/otp12c-{win,delegated}-2026-07-13/`. Acceptance reserved to the otp-13 owner walk. Suite untouched at **1484** (zero `crates/`/`proto/` changes). | `[x]` | harness `c26bc2d`+`b49413d`+`a2dea3f`; evidence `d12534d`+`68bb490`; record `9350b24` + review fixes `0fb4a64`+`4cc9b6e` |
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


exec
/bin/zsh -lc 'git show --format=fuller --no-ext-diff --no-renames -- .review/results/macmac-harness.gpt-verdict.md docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md scripts/bench_otp12pf_mac.sh scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py 24660ae' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-JH4Znr1S' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-aYr7zN7Q' (errno=Operation not permitted)
commit 24660ae4f1c75d7e877b5a889fec59846ea664fc
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Tue Jul 14 04:29:54 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Tue Jul 14 04:29:54 2026 -0400

    macmac-harness: address review (11 findings) — the instrument could have lied
    
    codex (gpt-5.6-sol, ultra) on e1e351d: NOT READY -- 3 BLOCKER, 6 HIGH, 1 MEDIUM,
    1 LOW. 11/11 accepted. NO DATA WAS TAKEN. Three of these would have produced a
    confidently wrong result from a clean-looking run.
    
    BLOCKER 1 -- the harness did not compute its own registered rule. It emitted only
    per-cell PASS/FAIL, so the six-outcome decision, the RIG-VOID gate, the power gate
    and the UNSTABLE override would have been applied BY HAND, after seeing the
    numbers -- exactly what pre-registration exists to prevent. New
    scripts/otp12pf_mac_verdict.py mechanizes the whole rule and emits
    session_verdict.txt.
    
    BLOCKER 2 -- the noise statistic would have declared a REAL effect "vanished".
    S = max(d)-min(d) is a RANGE: it grows with n and is dominated by outliers, so a
    large consistent effect hides under it. Codex's counterexample, which my code
    accepted: srcinit=2000, d=[0,180,180,190,190,200,200,200] -> D=190, S=200, bar
    PASS => "VANISHES" -- on 7/8 positive pairs and an effect 83% the size of rig W's
    Delta_P1. Replaced with a real paired inference: bootstrap 95% CI on median(d_i)
    (10k resamples, SEEDED so the verdict is deterministic) + an exact sign test, and
    a null is now an EQUIVALENCE result -- VANISHES requires the CI to exclude a
    bar-breaching effect (0.10 x srcinit_median), not merely to look small. Otherwise
    the cell is UNDERPOWERED and the session is INCONCLUSIVE-UNDERPOWERED.
    scripts/otp12pf_mac_verdict_test.py pins this; MUTATION-PROVEN (restore the range
    rule and the counterexample flips back to VANISHES and the test fails).
    
    BLOCKER 3 -- the inference still overreached. Rev 2 asked whether P1 is "a
    platform-general cost". Two machines cannot license that. Now scoped: can P1 occur
    WITHOUT a Windows peer, ON THIS PAIR. A null is consistent with "Windows is
    required" but does NOT prove it (could be these two machines/disks/OS version).
    That is the THIRD tightening of the same claim.
    
    HIGH -- the durability check was FAIL-OPEN, and I found it independently just
    before the review returned: os.walk() of a missing/empty path returns 0 files in
    0 ms and reads as a FAST SUCCESSFUL FLUSH. The two arms need DIFFERENT landed
    paths (verified empirically: a push to /bench/RUNDIR/ lands RUNDIR/src_<W>; a pull
    into RUNDIR lands files directly IN RUNDIR), so a wrong path would charge one arm
    ZERO durability -- the otp-2w bug that once MANUFACTURED P1. The fsync walk now
    returns its file count and byte sum and the pair VOIDS unless both match the
    fixture exactly, so an exit-0 zero-byte transfer can no longer become a fast row.
    
    HIGH -- the free-writeback gap REVERSED SIGN WITH DIRECTION. Between the client
    exiting and the fsync starting the OS writes back dirty pages for free, and that
    gap is longer for whichever arm ran over ssh -- which is destinit in nq and
    srcinit in qn. Since P1's signature is ONE-DIRECTIONAL, that artifact could have
    manufactured the result. MEASURED before fixing (verify the instrument, do not
    argue about it): a 10/20/200 ms pre-fsync delay produced NO measurable change in
    fsync time (72-94 ms, no trend) -- APFS fsync here is per-file-metadata bound, not
    writeback bound. Fixed structurally anyway with a fixed equal SETTLE_MS on both
    arms. Also: prep_run certified the drain BEFORE the purge (which itself dirties
    the disk) -- now purge first, then drain.
    
    HIGH -- every environmental gate failed OPEN (pgrep errors read as "quiet", a
    tmutil read error read as "disabled", top failures read as 0%, malformed load read
    as 0). All now fail CLOSED: a gate that cannot answer must never answer "fine" --
    the same class as pf-0's ps decaying average reporting a FINISHED backup as 255%.
    
    HIGH -- the ARP/link gate proved nothing: it ignored ping failure, accepted ANY
    MAC without comparing it to the peer's real one (so the documented own-MAC BLACK
    HOLE passed), and never checked the reverse direction or that the route egresses
    the 10GbE NIC. Now compares against measured MACs (q en8 00:01:d2:19:04:a3,
    nagatha en11 00:e0:4d:01:4c:a3), checks both directions, and asserts the route
    leaves the 10GbE interface (the multi-NIC trap: macOS routes by SERVICE order, so
    a 1GbE NIC can win and every run would silently go over gigabit).
    
    HIGH -- the registered protocol was unenforced (RUNS>=2 accepted; a misspelled
    CELLS could drop every control or measure nothing; blank CELLS ran 12 cells, not
    the 6 registered). Now: RUNS must be 8, CELLS must be a subset of the registered
    set, and every threshold is recorded in the manifest.
    
    HIGH -- instrument provenance was weak (a MODIFIED harness could still claim the
    reviewed commit; sha256_of accepted empty hashes; `! grep` turned a read error
    into "clean"). The harness now hashes ITSELF into the manifest and every
    provenance failure is fatal.
    
    MEDIUM -- `nc -z` proved only that a handshake reached some listener's backlog.
    Liveness is now a REAL blit smoke transfer to each daemon, and a teardown that
    cannot be verified is a failure, not a success.
    
    LOW -- median convention (low median for even n) stated once and applied
    everywhere.
    
    Prereg -> rev 3, matching exactly what the harness computes. bash -n OK; verdict
    engine compiles; guard test 4/4 and mutation-proven; check-docs OK.

diff --git a/.review/results/macmac-harness.gpt-verdict.md b/.review/results/macmac-harness.gpt-verdict.md
new file mode 100644
index 0000000..6c48493
--- /dev/null
+++ b/.review/results/macmac-harness.gpt-verdict.md
@@ -0,0 +1,176 @@
+# macmac-harness — adjudication of the codex review (round 1)
+
+**Slice**: `e1e351d` — `scripts/bench_otp12pf_mac.sh`, the Mac↔Mac harness
+(+ pre-registration rev 2).
+**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = ultra`.
+**Raw review**: `.review/results/macmac-harness.codex.md`
+**Verdict**: NOT READY — 3 BLOCKER, 6 HIGH, 1 MEDIUM, 1 LOW.
+**Adjudication: 11 findings, 11 ACCEPTED, 0 rejected.**
+
+**No data has been taken. The instrument was reviewed before it measured
+anything** — which is the only reason none of this became a retraction. Three of
+these would have produced a *confidently wrong result*.
+
+---
+
+## BLOCKER 1 — the harness does not compute its own registered rule → **ACCEPTED**
+
+The pre-registration defines six ordered outcomes, a RIG-VOID gate, a power gate
+and an UNSTABLE override. The harness emits only per-cell `PASS/FAIL` plus paired
+stats. **The session verdict would therefore have been applied BY HAND, after
+seeing the numbers** — which is exactly what pre-registration exists to prevent.
+Codex also notes the prose tree is itself still overlapping/incomplete.
+
+**Fix**: the harness must mechanize the rule end-to-end and emit a
+`session_verdict.txt` (RIG-VOID / REPRODUCES / INVERSION / VANISHES / PARTIAL /
+MIXED-SIGN / INCONCLUSIVE-UNDERPOWERED / UNSTABLE), with the prose tightened to
+match exactly.
+
+## BLOCKER 2 — the noise statistic would have declared a REAL effect "vanished" → **ACCEPTED**
+
+`S = max(d) − min(d)` is a **range**, not an MDE or an equivalence bound: it grows
+with n and is dominated by outliers, so a *large, consistent* effect can hide
+under it. Codex's counterexample, which my code accepts:
+
+    srcinit = 2000 ms (×8);  d = [0, 180, 180, 190, 190, 200, 200, 200]
+    -> D = 190, S = 200, bar = PASS, powered = yes
+    -> |D| <= S  =>  "VANISHES"
+
+…despite **7/8 pairs positive** and `D` at **83% of rig W's Δ_P1**. Repeated in
+both directions it would have declared "P1 requires the Windows peer" off an
+effect nearly the size of P1 itself. This is pf-0's underpowered-null error
+wearing a power gate.
+
+**Fix**: replace the range with a real paired inference —
+- **bootstrap 95% CI on median(d_i)** (n=8, resampled in-process, no scipy);
+- **exact sign test** (k of 8 positive, two-sided binomial);
+- **REPRODUCES** requires bar FAIL **and** CI lower bound > 0;
+- **VANISHES** requires bar PASS **and** the CI **upper** bound below the
+  bar-breaching effect for that cell (`0.10 × srcinit_median` — the effect that
+  would push the ratio to 1.10), i.e. a genuine **equivalence** result;
+- otherwise **INCONCLUSIVE** (and **UNDERPOWERED** when the CI is too wide to
+  exclude a bar-breaching effect).
+
+## BLOCKER 3 — the registered inference still overreaches → **ACCEPTED**
+
+Rev 2 narrowed rev 1's "H1 dies" to "platform-general cost of the layout". Still
+too strong. A reproduction proves only that **P1 can occur without a Windows peer
+on THIS pair**; a null proves only **non-reproduction on this pair** — not that
+Windows is *required* (it could be a property of these two specific machines,
+disks, or macOS versions).
+
+**Fix**: rev 3 scopes every claim to *this pair*, and states the residual
+alternatives explicitly. (This is the third tightening of the same claim; the
+lesson is that each round I stated a conclusion one step broader than the design
+could carry.)
+
+## HIGH — the fsync walk is fail-open, and nothing checks that bytes landed → **ACCEPTED** *(found independently by the author before the review returned)*
+
+`os.walk()` on a missing, unreadable or empty path emits a perfectly valid
+`F:0:F` — **a missing tree reads as a fast, successful flush**. The push and pull
+landed paths are *currently* correct (verified empirically: a push to
+`/bench/RUNDIR/` lands `RUNDIR/src_<W>`; a pull into `RUNDIR` lands the files
+directly in `RUNDIR`), but that is **luck, not a guard** — and there is **no
+destination count or byte-sum check**, so an exit-0 zero-byte or partial transfer
+becomes a valid *fast* row. This is the otp-2w bug's exact shape.
+
+**Fix**: the fsync walk returns `F:<ms>:<files>:F`; the harness **VOIDs the pair**
+unless the landed file count equals the fixture count **and** the landed byte sum
+matches. Source fixtures get a byte-sum check too, not just a count.
+
+## HIGH — transfer and fsync are disjoint intervals, and the free-writeback gap REVERSES BY DIRECTION → **ACCEPTED**
+
+The sharpest finding, and the one that could have *manufactured the result*.
+Between the client exiting and the fsync starting, the OS writes back dirty pages
+**for free** (charged to neither interval). That gap is **longer for whichever arm
+ran over ssh**, because the ssh return trip happens first:
+
+    cell nq (src=nagatha, dest=q):  srcinit = LOCAL client,  destinit = REMOTE client
+    cell qn (src=q, dest=nagatha):  srcinit = REMOTE client, destinit = LOCAL client
+
+So the favoured arm **flips sign with the data direction**. P1's whole signature is
+*one-directional* — meaning this artifact is capable of **producing a
+one-directional "reproduction" out of nothing**. Codex also notes `prep_run`
+certifies the drain *before* `sync; purge` and never re-checks it.
+
+**Fix (needs an owner decision — see below)**: make the client launch **symmetric**
+so neither arm carries an ssh return the other lacks. Also re-order `prep_run` so
+the drain is certified *after* the purge, and re-checked.
+
+## HIGH — environmental gates fail OPEN → **ACCEPTED**
+
+`pgrep` errors read as "quiet"; `tmutil` errors/empty parse to zero; an AutoBackup
+**read error explicitly becomes "disabled"**; `top` failures become zero and a
+trailing idle `mds` sample can overwrite a busy one; malformed/empty `load1`
+becomes 0. Every one of these fails toward "go".
+
+**Fix**: each gate must fail **closed** — an unreadable gate is a VOID, never a
+pass. (This is the same class as pf-0's `ps` decaying-average trap: an instrument
+that cannot answer must not answer "fine".)
+
+## HIGH — the ARP/link gate does not prove the link → **ACCEPTED**
+
+It ignores ping failure, accepts *any* complete MAC without comparing it to `q`'s
+**known** MAC (so the documented **own-MAC black hole** passes), and never checks
+the q→nagatha direction or that the route uses the 10GbE NIC rather than falling
+back to 1GbE.
+
+**Fix**: compare against the recorded peer MAC, check **both** directions, and
+assert the route egresses the 10GbE interface — plus the existing rule that an ssh
+throughput test is **not** a valid link check.
+
+## HIGH — the registered protocol is unenforced → **ACCEPTED**
+
+`RUNS>=2` is accepted (the design says 8); a misspelled `CELLS` can silently drop
+every control or measure nothing; blank `CELLS` runs **12** cells, not the six
+registered. Overridable drain thresholds are not recorded in the evidence.
+
+**Fix**: validate `CELLS` against the registered set, require the registered
+`RUNS`, and record every threshold in the manifest.
+
+## HIGH — instrument provenance is weak → **ACCEPTED**
+
+The manifest records `HEAD`, so a **modified** harness still claims the reviewed
+commit; `sha256_of` accepts empty/malformed hashes; and `! grep` turns a
+*read error* on the dirty-marker check into "clean".
+
+**Fix**: hash the harness file itself into the manifest, refuse a dirty harness,
+and make hash/provenance failures fatal.
+
+## MEDIUM — daemon liveness and teardown → **ACCEPTED**
+
+`nc -z` proves only that a handshake reached *some* listener's backlog — not that
+the captured PID accepts or speaks blit. Teardown logs "verified gone" when the
+ssh/`ps` probe *itself* failed, and cleanup discards a positively detected
+survivor.
+
+**Fix**: probe with a real blit call (the smoke), and treat a survivor or an
+unverifiable teardown as fatal.
+
+## LOW — median/IQR conventions → **ACCEPTED**
+
+Even-sample medians are floored before the "exact" bar and the `D > S`
+comparisons; the n=8 IQR convention is unstated. Codex confirms the ABBA void
+retry, slot pairing and the `destinit − srcinit` sign are otherwise **correct**.
+
+**Fix**: state the convention and apply it consistently.
+
+---
+
+## The one finding that needs the owner: symmetric client launch
+
+Fixing the free-writeback asymmetry requires the two arms to be launched
+identically. The options are an infrastructure choice, not a code choice:
+
+- **(A)** drive the harness from a **third host** (skippy/magneto) so **both**
+  Macs are remote and symmetric — needs ssh keys from that host to both Macs;
+- **(B)** keep the driver on nagatha but launch **both** clients over ssh,
+  including nagatha→itself — needs a host key + `authorized_keys` entry on
+  nagatha;
+- **(C)** equalize with a fixed settle window before the fsync on both arms —
+  no infra change, but it lets writeback complete "for free" for both arms and so
+  weakens what destination-keyed durability is meant to charge.
+
+Recorded for the owner; **no rig time until it is resolved**, because this
+artifact is capable of manufacturing exactly the one-directional result the
+experiment is looking for.
diff --git a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
index 36e4f3c..39997a6 100644
--- a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
+++ b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
@@ -1,10 +1,17 @@
 # otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
 
-**Status**: Pre-registered, **revision 2**. **No data exists yet.**
-Codex round 1 (of `f0343f4`): **NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → 9/9
-accepted.** Adjudication: `.review/results/macmac-prereg.gpt-verdict.md`.
-Committed BEFORE the data so the decision rule cannot be authored around the
-numbers (the pf-0 discipline).
+**Status**: Pre-registered, **revision 3**. **No data exists yet.**
+- Codex round 1 (of `f0343f4`, the design): NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW
+  → **9/9 accepted** (`.review/results/macmac-prereg.gpt-verdict.md`).
+- Codex round 2 (of `e1e351d`, the **instrument**): NOT READY — **3 BLOCKER** +
+  6 HIGH + 1 MEDIUM + 1 LOW → **11/11 accepted**
+  (`.review/results/macmac-harness.gpt-verdict.md`).
+
+Committed BEFORE the data so the rule cannot be authored around the numbers.
+**Two rounds of review have now caught, between them, an invalid inference, a
+statistic that would have declared a real effect absent, a fail-open durability
+check, and a timing artifact that reverses sign with direction — all before a
+single timed run.**
 
 **Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).
 
@@ -26,15 +33,19 @@ The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
 accept branch") and copied without checking H1's text. **That is a repo error and
 it is corrected wherever it appears.**
 
-**What this rig CAN answer — and it is still decision-relevant:**
+**What this rig CAN answer — and revision 2 STILL overstated it (round-2 BLOCKER).**
+Rev 2 asked whether P1 is "a platform-general cost of the layout". A rig with two
+machines cannot license that. The claim is now scoped to **this pair**:
 
-> **Does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost
-> of the destination-initiated layout?**
+> **Can P1 occur WITHOUT a Windows peer — on this pair of Macs?**
 
-| outcome | what it licenses |
+| outcome | what it licenses — and its limit |
 |---|---|
-| **P1 REPRODUCES macOS↔macOS** | The failure needs **no Windows peer**. P1 is **not platform residue** — it is a cost of the layout/code that survives with the Windows half removed. This **closes the "accept it as platform residue" escape** (the D-2026-07-12-1 shape) and **strengthens every code-level hypothesis, H1 included**. It does **not** name the mechanism. |
-| **P1 VANISHES macOS↔macOS** | The failure **requires the Windows peer**: it is pairing-dependent / platform-interacting. Code-only mechanisms that should bite on any OS are **weakened**; a Windows-specific cost, or a macOS↔Windows interaction, rises. It does **not** confirm H1 — H1's accept branch would then have to be *platform-conditionally* slow, which is a further claim needing pf-1's counterfactual. |
+| **P1 REPRODUCES** | P1 **does not require a Windows peer** (on this pair). It is therefore **not** "platform residue" that could be waived under the D-2026-07-12-1 shape, and every code-level hypothesis strengthens. **Limits**: it does **not** establish a platform-*general* cost (two Macs are not "all platforms"), it does **not** name the mechanism, and it does **not** kill H1 — the code H1 accuses runs here too, so a reproduction is *consistent with* H1. |
+| **P1 does NOT reproduce (null)** | P1 **did not occur on this pair**. That is **consistent with** "the Windows peer is required" — but does **not prove it**: it could equally be a property of *these two machines*, their disks, or this macOS version. It does **not** confirm H1 either. |
+
+A null is only reportable at all if the rig could have **seen** a rig-W-sized
+effect — see the POWER GATE. Otherwise it is `INCONCLUSIVE-UNDERPOWERED`.
 
 Either outcome materially reshapes the hypothesis space and bears directly on
 whether P1 **must be fixed in code** or **could be accepted as platform residue**.
@@ -80,89 +91,127 @@ directions differ in *which machine is the destination*, a one-directional resul
 is explicitly **not** dismissible as "machine asymmetry" (revision 1 did exactly
 that, which would have let a real reproduction be waved away).
 
-## The noise model — PAIRED and within-cell (round-1 HIGH; revision 1's was not a noise floor at all)
-
-Revision 1 defined `N` = max |ratio−1| over the four control cells. That is **not
-a noise floor**: it is four point estimates drawn from different carriers,
-fixtures and destinations, so it conflates *genuine control-specific initiator
-effects* with *sampling noise*, and could equally mask a real effect or bless a
-fake one.
-
-Replaced with the **paired within-cell** statistic — the same construction pf-0's
-review demanded of pf-1:
-
-    For each cell, each ABBA slot i yields a matched pair (srcinit_i, destinit_i).
-      d_i   = destinit_i − srcinit_i          (positive = P1's direction)
-      D     = median(d_i)                     <- the effect
-      S     = the spread of d_i               <- the PAIRED noise (report max−min AND IQR)
-      MDE   = the smallest |D| this cell can resolve, taken as S (conservative)
-
-`D` and `S` come from the *same* slots, under the *same* conditions, so ABBA
-pairing is respected and between-session drift cannot enter. Every threshold below
-is expressed against `S`, the 1.10 bar, or rig W's measured `Δ_P1 ≈ 230 ms` — none
-is invented.
-
-## POWER GATE — evaluated BEFORE any "vanish" claim (round-1 HIGH; pf-0's exact error, pre-empted)
-
-pf-0 reported a KILL with an instrument that could not have resolved the effect it
-killed. That must not recur.
-
-For each TCP×mixed cell, **before** reading a verdict:
-
-1. Compute `MDE` (above) and the effect size that a rig-W-scale P1 would have
-   here: `Δ_ref = 230 ms` (rig W's Δ_P1), and also in ratio terms against **this
-   rig's own fast arm** — because the 1.10 bar is a *ratio*, a 230 ms effect is
-   only visible if the fast arm is fast enough (at a 2.3 s fast arm, 230 ms is
-   exactly 1.10 and would sit **on** the bar).
-2. **If `MDE > Δ_ref`, or if `Δ_ref` on this cell's fast arm does not exceed the
-   1.10 bar, the cell is UNDERPOWERED and a PASS there is INCONCLUSIVE — it may
-   NOT be reported as "P1 vanishes".** The rig gets reported as unable to see the
-   effect, and the experiment does not close.
-
-A **reproduction** does not need this gate (an effect that is seen is seen); a
-**null** does.
-
-## Decision rule — pre-registered, exhaustive, mutually exclusive, evaluated in order
-
-Invariance uses the harness's **exact integer arithmetic** (`10·hi ≤ 11·lo`),
-never the printed ratio. Per TCP×mixed cell: `D` = median paired difference,
-`S` = paired spread.
-
-1. **RIG-VOID.** Any control cell FAILS the 1.10 bar → the rig is not measuring
-   cleanly and **no verdict is read**. (A rig whose gRPC control fails cannot
-   adjudicate a TCP-only claim.) Report and stop.
-2. **REPRODUCES (in a named direction).** A TCP×mixed cell FAILS the 1.10 bar with
-   `D > 0` **and** `D > S`. Reported per direction; **either direction suffices.**
-   → *P1 does not need a Windows peer.*
-3. **INVERSION (in a named direction).** A TCP×mixed cell FAILS with `D < 0` and
-   `|D| > S` (source-initiated is the slow arm). A **new finding**, reported as
-   such — never banked as "P1 absent" and never counted as a reproduction.
-4. **VANISHES.** *Both* TCP×mixed cells PASS the 1.10 bar, **and** `|D| ≤ S` in
-   both, **and both cells cleared the POWER GATE.** → *P1 requires the Windows
-   peer.* If the power gate was not cleared, this branch is unavailable and the
-   result is **INCONCLUSIVE-UNDERPOWERED**.
-5. **PARTIAL.** Any TCP×mixed cell PASSES the bar but has `|D| > S` in P1's
-   direction — a real, sub-bar asymmetry. Reported with `D` stated against
-   `Δ_ref = 230 ms`. Neither a reproduction nor a vanish; pf-1 owns it.
-6. **MIXED-SIGN.** One direction reproduces (case 2) and the other inverts
-   (case 3). Reported verbatim as a **host×role interaction**, which the rig
-   cannot decompose. Explicitly **inconclusive** for the pairing question.
-
-Cases 2/3/5/6 are read per direction and then combined by this order; the first
-matching case that applies to the *session* is the headline, with every cell's own
-outcome recorded. **No case is left unmapped, and no outcome may be reported that
-is not one of these.**
-
-**Bistability override, defined as a statistic, not a vibe (round-1 HIGH).** pf-0
-found the rig-W fast arm bimodal, where the mode *mixture* moved a median 72 ms at
-constant conditions. Here: if any arm's 8 runs split into two clusters separated by
-more than `S` **and** the cell's verdict would flip when graded on the pooled runs
-rather than the medians, the cell is reported **UNSTABLE**, not resolved. All 8
-runs of every arm are printed in `summary.csv` so this is checkable, not asserted.
-
-## Gates — fail-closed (round-1 HIGH: revision 1 only *warned* on the one that bit pf-0)
-
-A run that misses any of these is **VOID**, not "close enough":
+## The paired statistic — and why revision 2's was BROKEN (round-2 BLOCKER)
+
+Rev 1 used `N` = max |ratio−1| over four control cells: four point estimates from
+different carriers, fixtures and destinations — not a noise floor at all. Rev 2
+replaced it with the paired difference and `S = spread(d_i)` as the noise. **That
+is still broken**, because a *range* grows with n and is dominated by outliers, so
+a **large, consistent effect can hide under it**. Codex's counterexample, which
+rev 2's rule accepted:
+
+    srcinit = 2000 ms (×8);   d = [0, 180, 180, 190, 190, 200, 200, 200]
+    -> D = 190, S = 200, bar PASSES, |D| <= S   =>   rev 2 says "VANISHES"
+
+…on **7/8 positive pairs** and an effect **83% the size of rig W's Δ_P1**. It
+would have reported "P1 requires the Windows peer" off an effect nearly as big as
+P1 itself.
+
+**Replaced with a real paired inference** (computed by
+`scripts/otp12pf_mac_verdict.py`, and guarded by a test that asserts the
+counterexample above no longer returns VANISHES):
+
+    per ABBA slot i:  d_i = destinit_i − srcinit_i     (positive = P1's direction)
+      D    = median(d_i)
+      CI   = 95% BOOTSTRAP CI on the median (10k resamples, SEEDED -> deterministic)
+      sign = exact two-sided binomial test on the count of positive d_i
+      BAR_BREACH = 0.10 × srcinit_median   <- the effect that would reach the 1.10 bar
+
+The median convention is the **low median** for even n, stated once and applied
+everywhere (round-2 LOW).
+
+## POWER GATE — a null must be an EQUIVALENCE result, not an absence of evidence
+
+pf-0 reported a KILL with an instrument that could not resolve the effect it
+killed. This design pre-empts that:
+
+- A **null is only reportable** if the CI **excludes a bar-breaching effect** —
+  i.e. the whole CI lies strictly inside ±`BAR_BREACH`. That is a genuine
+  *equivalence* claim: "an effect big enough to matter is ruled out."
+- If the CI is **too wide** to exclude it, the cell is **UNDERPOWERED** and the
+  session verdict is **INCONCLUSIVE-UNDERPOWERED**. A PASS is then *not*
+  "P1 vanishes" — it is "this rig could not have seen it".
+- A **reproduction** needs no such gate: an effect that is seen is seen.
+
+## Decision rule — computed BY THE HARNESS, exhaustive, in strict precedence
+
+The harness emits `session_verdict.txt`. **The verdict is not applied by hand
+after the numbers are visible** (round-2 BLOCKER: rev 2's harness computed only
+PASS/FAIL, which would have left the rule to me, post-hoc).
+
+Per cell (integer-exact bar `10·hi ≤ 11·lo`, never the printed ratio):
+
+| cell outcome | condition |
+|---|---|
+| **REPRODUCES** | bar **FAILS** and `CI_lo > 0` |
+| **INVERSION** | bar **FAILS** and `CI_hi < 0` |
+| **VANISHES** | bar **PASSES** and the CI lies strictly inside ±`BAR_BREACH` |
+| **UNDERPOWERED** | bar **PASSES** and the CI cannot exclude `BAR_BREACH` |
+| **PARTIAL** | bar **PASSES**, CI excludes 0, effect not excluded as small |
+| **UNSTABLE** | (override) an arm is bimodal *and* the bar verdict flips on pooled runs |
+
+Session precedence (first match wins; every cell's own outcome is still recorded):
+
+1. **INCOMPLETE** — any cell short of its pairs.
+2. **RIG-VOID** — any **control** cell FAILS the bar. A rig whose gRPC/large
+   control fails cannot adjudicate a TCP-only claim. No verdict is read.
+3. **UNSTABLE** — a bimodal arm whose verdict flips. Reported as unstable, not
+   resolved.
+4. **MIXED-SIGN** — one direction REPRODUCES and the other INVERTS: a host×role
+   interaction this rig **cannot decompose**. Inconclusive for the question.
+5. **REPRODUCES** — either direction. → *P1 can occur without a Windows peer, on
+   this pair.*
+6. **INVERSION** — a new finding; never banked as "P1 absent".
+7. **INCONCLUSIVE-UNDERPOWERED** — the null branch is unavailable.
+8. **VANISHES** — both TCP×mixed cells exclude a bar-breaching effect.
+9. **PARTIAL** — a real but sub-bar asymmetry; pf-1 owns it.
+
+**No outcome may be reported that is not one of these.**
+
+**Bistability is a STATISTIC, not a vibe.** pf-0 found the rig-W fast arm bimodal,
+where the mode *mixture* moved a median 72 ms at constant conditions. Here: an arm
+whose runs split into two clusters separated by more than the paired spread, **and**
+whose bar verdict flips when graded on pooled runs rather than medians, is
+**UNSTABLE**. All 8 runs of every arm are printed in `summary.csv`, so this is
+checkable rather than asserted.
+
+## The instrument — two defects that could have MANUFACTURED the result (round-2 HIGH)
+
+**1. The durability check was fail-open.** `os.walk()` on a missing, unreadable or
+empty path returns **0 files in 0 ms** — a missing tree reads as a *fast,
+successful flush*. The two arms need **different** landed paths, because blit uses
+rsync-style slash semantics (verified empirically: a push to `/bench/RUNDIR/` lands
+the tree at `RUNDIR/src_<W>`; a pull into `RUNDIR` lands the files **directly in**
+`RUNDIR`). A wrong path would have charged one arm **zero** durability while the
+other paid full — the otp-2w bug that once manufactured P1.
+**Fixed**: the fsync walk returns its **file count and byte sum**, and the pair
+**VOIDs** unless both match the fixture exactly. An exit-0 zero-byte or partial
+transfer can no longer become a valid *fast* row.
+
+**2. The free-writeback gap REVERSED SIGN WITH DIRECTION.** Between a client
+exiting and the fsync starting, the OS writes back dirty pages **for free**, and
+that gap is longer for whichever arm ran over ssh:
+
+    cell nq (dest = q):        srcinit = LOCAL client,  destinit = REMOTE client
+    cell qn (dest = nagatha):  srcinit = REMOTE client, destinit = LOCAL client
+
+So the *favoured arm flips with the data direction*. Since P1's signature is
+**one-directional**, this artifact is capable of producing a one-directional
+"reproduction" **out of nothing**.
+**Measured before fixing** (the instrument is verified, not assumed): a pre-fsync
+delay of **10 / 20 / 200 ms produced no measurable change in fsync time**
+(72–94 ms, no trend) — APFS fsync here is per-file-metadata bound, not writeback
+bound. **Fixed anyway, structurally**: a fixed, equal `SETTLE_MS` (250 ms) precedes
+the fsync on **both** arms, so the asymmetry is removed by construction without
+weakening what durability charges.
+
+## Gates — fail-closed (round-1 HIGH: revision 1 only *warned*; round-2 HIGH: they all failed OPEN)
+
+A run that misses any of these is **VOID**, not "close enough". **Every gate fails
+CLOSED**: a gate that cannot answer must never answer "fine" (round 2 found
+`pgrep` errors reading as "quiet", a `tmutil` read error reading as "disabled",
+`top` failures reading as 0% — the same class as pf-0's `ps` decaying average that
+reported a *finished* backup as 255%).
 
 - **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
   **either** Mac (both are bench **ends** here — nagatha is no longer just the
diff --git a/scripts/bench_otp12pf_mac.sh b/scripts/bench_otp12pf_mac.sh
index b6cdcd6..d24907c 100755
--- a/scripts/bench_otp12pf_mac.sh
+++ b/scripts/bench_otp12pf_mac.sh
@@ -8,93 +8,89 @@
 # WHY THIS RIG EXISTS
 # -------------------
 # P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
-# on macOS<->Windows. Linux<->Linux shows NO P1 (8/8 PASS). macOS<->macOS is the
-# untested cell of the 2x2. It answers ONE question:
+# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
+# cell. It answers ONE question, SCOPED TO THIS PAIR:
 #
-#     Does P1 REQUIRE the macOS<->Windows PAIRING, or is it a platform-general
-#     cost of the destination-initiated layout?
+#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
 #
-#   * reproduces -> P1 needs no Windows peer: it is NOT platform residue, the
-#     "accept it as platform residue" escape closes, and every code-level
-#     hypothesis strengthens;
-#   * vanishes   -> P1 is pairing-dependent: platform-agnostic code mechanisms
-#     weaken and a Windows-specific cost (or an interaction) rises.
+#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
+#     "platform residue" that can be waived; code-level hypotheses strengthen.
+#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
+#     "Windows is required", but does NOT prove it: it could equally be a
+#     property of these two machines, their disks, or this macOS version.
 #
 # ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
-# Revision 1 of this script and of docs/STATE.md claimed "reproduces => H1 DIES,
-# because H1 accuses the Windows accept branch". That is FALSE and is retracted:
 # H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
-# InitiatorReceivePlaneRun.add_dialed_stream, the synchronous dial-before-ACK at
+# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
 # transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
-# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with
-# H1, not fatal to it. (The parent plan itself warns: "'consistent with H1' is
-# not confirmation.") Caught by codex review of the pre-registration, BEFORE any
-# rig time was spent.
+# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
+# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
 #
 # WHAT IT MEASURES
-#   cell = <nq|qn>_<carrier>_<fixture>
-#     nq_* : data nagatha -> q        qn_* : data q -> nagatha
-#   arms per cell (the ONLY variable):
-#     srcinit  : the SOURCE host's CLI pushes      (source-initiated)
-#     destinit : the DEST   host's CLI pulls       (destination-initiated)
-#   BOTH data directions are measured, but a reproduction is NOT required in
-#   both: P1's recorded signature on rig W is ONE-DIRECTIONAL (wm_tcp_mixed FAILS
-#   while mw_tcp_mixed PASSES), so demanding failure in both would rewrite the
-#   finding. A reproduction in EITHER direction demonstrates the layout cost
-#   without a Windows peer.
+#   cell = <nq|qn>_<carrier>_<fixture>;  nq_* = data nagatha->q, qn_* = q->nagatha
+#   arms (the ONLY variable): srcinit (source's CLI pushes) / destinit (dest's CLI
+#   pulls). BOTH directions are measured, but a reproduction is NOT required in
+#   both — P1's rig-W signature is ONE-DIRECTIONAL (wm FAILS, mw PASSES), so
+#   demanding both would rewrite the finding.
 #
-#   Endpoint asymmetry does NOT simply cancel: switching the initiator also
-#   reassigns which Mac runs the CLI and which runs the daemon, and q is the
-#   faster machine. Only arm-independent costs cancel; host x role interactions
-#   do not. Hence both directions are reported SEPARATELY and no conclusion may
-#   lean on perfect cancellation.
+#   Endpoint asymmetry does NOT cancel: switching the initiator also reassigns
+#   which Mac runs the CLI vs the daemon, and q is faster. Both directions are
+#   therefore reported separately and no conclusion leans on cancellation.
 #
-# VERDICT: invariance bar, max(srcinit,destinit)/min <= 1.10, integer-exact
-# (10*hi <= 11*lo). This script COMPUTES; it DECLARES nothing.
+# THE INSTRUMENT IS THE RISK (three claims have been retracted to harness bugs).
+# Everything below fails CLOSED. Codex review of the first revision found 11
+# defects (3 BLOCKER) in this file before it measured anything; they are fixed
+# here and named at their site.
 #
-# METHODOLOGY (otp-12 shape + the two gates pf-0 proved were missing)
-#   * QUIESCENCE gate on BOTH Macs (codex/cargo/rustc) — here nagatha is a bench
-#     END, not merely the driver; load on either end contaminates ASYMMETRICALLY.
-#   * TIME MACHINE gate on BOTH Macs — the hole pf-0 found: the old quiet-gate
-#     watched only codex/cargo/rustc and would have sailed straight through the
-#     backup that fired 1 minute before pf-0's run (hourly cadence; one
-#     destination is a network share on skippy = the same 10GbE fabric).
-#   * cold caches BOTH ends every run via `sudo -n /usr/sbin/purge` (a failed
-#     purge VOIDS the pair — a warm row is worse than no row);
-#   * destination disk drained to quiet (iostat) before each timed window;
-#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER BY THE INITIATOR/VERB:
-#     the macOS per-file fsync walk runs on the destination for BOTH arms. (The
-#     otp-2w rule, re-learned the hard way: a sync inside the initiator's bracket
-#     charges the pull arm for writeback the push arm gets free and MANUFACTURES
-#     invariance failures — including on the gRPC control that must stay clean.)
-#   * fresh never-seen destination per run; ABBA counterbalance; pair-void with a
-#     2*RUNS cap then INCOMPLETE; nonzero exit or undrained window voids the pair;
-#   * same-build gate: every binary embeds a CLEAN +EXPECT_SHA (never +sha.dirty).
+#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER THE INITIATOR/VERB, and
+#     the fsync walk VERIFIES WHAT IT FLUSHED: it returns the file count and byte
+#     sum, and the pair VOIDS unless they match the fixture exactly. (os.walk of a
+#     missing/empty path returns 0 files in 0 ms and reads as a FAST SUCCESSFUL
+#     FLUSH — the otp-2w bug's exact shape. Verified empirically: a push to
+#     /bench/RUNDIR/ lands RUNDIR/src_<W>, a pull into RUNDIR lands files directly
+#     in RUNDIR, so the two arms need DIFFERENT landed paths and a wrong one would
+#     silently charge an arm nothing.)
+#   * A FIXED, EQUAL SETTLE (SETTLE_MS) precedes the fsync on BOTH arms. Between
+#     a client exiting and the fsync starting, the OS writes back dirty pages FOR
+#     FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
+#     BY DIRECTION (in nq the remote arm is destinit; in qn it is srcinit). Since
+#     P1's signature is one-directional, that artifact could MANUFACTURE the
+#     result. Measured on this rig before fixing: a 10/20/200 ms pre-fsync delay
+#     produced NO measurable change in fsync time (72-94 ms, no trend) — APFS
+#     fsync here is per-file-metadata bound, not writeback bound — so the fixed
+#     settle removes the structural asymmetry without weakening what durability
+#     charges.
+#   * cold caches BOTH ends every run (purge), then the destination disk is
+#     drained to quiet AND RE-CHECKED — the purge itself dirties the disk, so a
+#     drain certified BEFORE it proves nothing.
+#   * pair-void on: nonzero exit, undrained window, failed purge, fsync mismatch.
+#   * same-build gate: clean +EXPECT_SHA, never +sha.dirty; hash failures FATAL.
+#   * the HARNESS ITSELF is hashed into the manifest — a modified harness must not
+#     be able to claim the reviewed commit.
 #
-# TOPOLOGY NOTE (why one end is local): the driver runs on nagatha, so the nagatha
-# end is LOCAL and the q end is over ssh. This is the proven rig-W shape: each
-# timed window is self-timed ON the initiating host — locally for nagatha, and
-# INSIDE a single ssh for q — so the ssh round trip is outside the window by
-# construction and neither arm is charged for dispatch. The driver is blocked
-# waiting during every timed window, so its own load is idle and identical across
-# arms.
+# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
+# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
+# one ssh), so dispatch is outside the window by construction.
 #
 # Usage:
-#   EXPECT_SHA=f35702a RUNS=8 bash scripts/bench_otp12pf_mac.sh
+#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
 #   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
-#   CELLS=nq_tcp_mixed,qn_tcp_mixed RUNS=8 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
 # =============================================================================
 set -euo pipefail
 
 SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
 REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
+SELF="${BASH_SOURCE[0]}"
 
 HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
-EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a) — the binaries are gated on it}"
+HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
+EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"
 
-# --- nagatha: LOCAL end (driver runs here) -----------------------------------
+# --- nagatha: LOCAL end (driver) ---------------------------------------------
 N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
-N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"    # the pinned clone
+N_NIC="${N_NIC:-en11}"
+N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
+N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
 N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
 N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
 N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
@@ -102,6 +98,8 @@ N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
 # --- q: REMOTE end ------------------------------------------------------------
 Q_SSH="${Q_SSH:-michael@q}"
 Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
+Q_NIC="${Q_NIC:-en8}"
+Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
 Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
 Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
 Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
@@ -110,14 +108,23 @@ Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
 PORT="${PORT:-9031}"
 RUNS="${RUNS:-8}"
 PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
-CELLS="${CELLS:-}"
+SETTLE_MS="${SETTLE_MS:-250}"     # equal pre-fsync window on BOTH arms
+LOAD_MAX="${LOAD_MAX:-3.0}"
+DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
+DRAIN_MBPS="${DRAIN_MBPS:-2}"
+DELTA_REF_MS="${DELTA_REF_MS:-230}"   # rig W's measured Delta_P1 (the reference effect)
+
+# The REGISTERED cell set. An unregistered or misspelled CELLS must not be able to
+# drop every control, or silently measure nothing (codex HIGH).
+REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
+CELLS="${CELLS:-$REGISTERED_CELLS}"
+CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
+VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
+
 SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
 OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
-DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
-DRAIN_MBPS="${DRAIN_MBPS:-2}"     # dest disk considered quiet below this MB/s
 
-# /tmp, not $TMPDIR: macOS TMPDIR busts ssh's 104-byte ControlPath cap (otp-12c).
-MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"
+MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
 SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
          -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
 qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
@@ -126,271 +133,296 @@ mkdir -p "$OUT_DIR/blit-logs"
 log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
 die() { log "FATAL: $*"; exit 1; }
 nocr() { tr -d '\r'; }
-want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }
+want_cell() { [[ ",$CELLS," == *",$1,"* ]]; }
 
-# --- host abstraction: $1 = n (local nagatha) | q (remote) --------------------
+# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
 # if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
-# falls through to the wrong host (the exact trap the Linux harness documents).
+# falls through to the wrong host (the trap the Linux harness documents).
+# `bash -c` locally pins the inner shell so local and remote parse identically
+# (q's login shell is not assumed).
 hrun() {
   local h="$1"; shift
-  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "$*"; fi
+  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "bash -c $(printf '%q' "$*")"; fi
 }
 hblit()   { [[ "$1" == n ]] && echo "$N_BLIT"   || echo "$Q_BLIT"; }
 hdaemon() { [[ "$1" == n ]] && echo "$N_DAEMON" || echo "$Q_DAEMON"; }
 hmod()    { [[ "$1" == n ]] && echo "$N_MODULE" || echo "$Q_MODULE"; }
 hip()     { [[ "$1" == n ]] && echo "$N_IP"     || echo "$Q_IP"; }
+hnic()    { [[ "$1" == n ]] && echo "$N_NIC"    || echo "$Q_NIC"; }
+hmac()    { [[ "$1" == n ]] && echo "$N_MAC"    || echo "$Q_MAC"; }
 hname()   { [[ "$1" == n ]] && echo nagatha     || echo q; }
+other()   { [[ "$1" == n ]] && echo q           || echo n; }
 
-# --- fixtures (otp-2 shapes; verified by count, never trusted) ----------------
-FIX_COUNT_large=1;     FIX_COUNT_small=10000;  FIX_COUNT_mixed=5001
+# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
+FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
+FIX_COUNT_small=10000; FIX_BYTES_small=40960000
+FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
 
-# --- provenance: embed +sha AND reject +sha.dirty -----------------------------
-embeds_clean() {   # $1=host $2=path
-  hrun "$1" "grep -qa -- '+$EXPECT_SHA' '$2' && ! grep -qa -- '+$EXPECT_SHA.dirty' '$2'"
+# --- provenance ---------------------------------------------------------------
+embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
+  local h="$1" p="$2" hit dirty
+  hit="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA' '$p' 2>/dev/null || echo X" | nocr)"
+  dirty="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA.dirty' '$p' 2>/dev/null || echo X" | nocr)"
+  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
+  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
 }
-sha256_of() {      # $1=host $2=path
-  hrun "$1" "shasum -a 256 '$2' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f'
+sha256_of() {      # fail CLOSED on an empty/short hash
+  local h="$1" p="$2" v
+  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')"
+  [[ ${#v} -eq 64 ]] || die "$(hname "$h"): sha256 of $p returned '${v}' (not 64 hex) — refusing"
+  echo "$v"
 }
 
-# --- the two gates pf-0 proved were missing -----------------------------------
-quiescence_gate() {   # $1 = host. Bench ENDS must be quiet; load contaminates ASYMMETRICALLY.
-  local h="$1" busy
-  busy="$(hrun "$h" "pgrep -x codex >/dev/null && echo codex; pgrep -x cargo >/dev/null && echo cargo; pgrep -x rustc >/dev/null && echo rustc; true" | nocr | tr '\n' ' ')"
-  busy="$(echo "$busy" | xargs || true)"
-  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). Both Macs are bench ENDS — a busy end inflates one arm and MANUFACTURES P1 (.agents/machines.md). Stop them (do NOT blanket-kill the owner's sessions) and re-run."
+# --- gates: every one fails CLOSED (codex HIGH: they all failed OPEN) ----------
+norm_mac() { tr 'A-F' 'a-f' | awk -F: '{for(i=1;i<=NF;i++){printf "%s%02x", (i>1?":":""), strtonum("0x" $i)}; print ""}'; }
+
+quiescence_gate() {
+  local h="$1" out
+  out="$(hrun "$h" "pgrep -x codex >/dev/null 2>&1 && echo codex; pgrep -x cargo >/dev/null 2>&1 && echo cargo; pgrep -x rustc >/dev/null 2>&1 && echo rustc; echo __OK__" | nocr)" \
+    || die "$(hname "$h"): quiescence probe FAILED — a gate that cannot answer must not answer 'fine'"
+  [[ "$out" == *__OK__* ]] || die "$(hname "$h"): quiescence probe returned no sentinel — refusing"
+  local busy; busy="$(echo "$out" | grep -v __OK__ | tr '\n' ' ' | xargs || true)"
+  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
 }
-timemachine_gate() {   # $1 = host. FAIL-CLOSED — the hole pf-0 found.
+timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   local h="$1" running auto
-  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';'" | nocr | tr -cd '0-9')"
-  [[ "${running:-0}" == 1 ]] && die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench END (one destination is on skippy, the same 10GbE fabric)."
-  # AUTOBACKUP ENABLED is itself disqualifying, not a warning: macOS repeats
-  # HOURLY, so a backup can begin *inside* the window. pf-0's fired 1 minute
-  # before its run and the old gate never looked. A warning here would let the
-  # session start and be silently contaminated mid-flight.
-  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null || echo 0" | nocr | tr -cd '0-9')"
-  [[ "${auto:-0}" == 1 ]] && die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED — macOS repeats hourly, so a backup can start MID-SESSION. Disable it for the window (\`sudo tmutil disable\`) and re-enable after."
-  true
+  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
+  [[ "$running" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
+  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
+  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1; echo" | nocr | tr -cd '0-9')" || auto=""
+  [[ "$auto" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
+  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
 }
-spotlight_gate() {   # $1 = host. mds_stores is a recorded contaminant (.agents/machines.md).
-  # Instantaneous sample: `ps` %CPU is a DECAYING AVERAGE and reads a finished
-  # backup as 255% (learned in pf-0) — top -l 2 is the honest instrument.
+spotlight_gate() {
   local h="$1" cpu
-  cpu="$(hrun "$h" "top -l 2 -n 20 -o cpu -stats command,cpu 2>/dev/null | awk '/mds_stores|^mds /{c=\$NF} END{print c+0}'" | nocr | tr -cd '0-9.')"
-  awk -v c="${cpu:-0}" 'BEGIN{exit !(c+0 > 20)}' \
-    && die "$(hname "$h"): Spotlight (mds_stores) is actively indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
-  true
+  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null | awk '/^mds_stores/{c=\$2} END{printf \"%d\", c+0}'" | nocr | tr -cd '0-9')" || cpu=""
+  [[ "$cpu" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot sample Spotlight CPU — refusing"
+  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
+}
+load_gate() {
+  local h="$1" l ok
+  l="$(hrun "$h" "sysctl -n vm.loadavg" | nocr | awk '{print $2}')" || l=""
+  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die "$(hname "$h"): cannot read load1 (got '$l') — refusing"
+  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
+  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
 }
 load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
-load_gate() {   # $1 = host. The Macs idle at ~1.5-2.0; above 3.0 something is running.
-  local h="$1" l; l="$(load1 "$h")"
-  awk -v l="${l:-0}" 'BEGIN{exit !(l+0 > 3.0)}' \
-    && die "$(hname "$h"): load1 is $l (> 3.0) — a bench END must be quiet. Find what is running before starting a timed session."
-  true
+
+link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
+  local h="$1" o peer_ip want got route_nic
+  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"
+  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
+    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
+  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk '{print \$4}'" | nocr | norm_mac)"
+  [[ -n "$got" && "$got" != "(incomplete)" ]] || die "$(hname "$h"): no ARP entry for $peer_ip"
+  [[ "$got" == "$want" ]] \
+    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
+  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
+  [[ "$route_nic" == "$(hnic "$h")" ]] \
+    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
 }
 
 preflight() {
-  [[ "$RUNS" -ge 2 ]] || die "RUNS must be >= 2"
-  local h p
+  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
+  local c
+  for c in ${CELLS//,/ }; do
+    [[ ",$REGISTERED_CELLS," == *",$c,"* ]] \
+      || die "cell '$c' is not in the REGISTERED set ($REGISTERED_CELLS) — a misspelled cell must not silently drop a control or measure nothing"
+  done
+  local h p w want got wantb gotb
   for h in n q; do
-    quiescence_gate "$h"
-    timemachine_gate "$h"
-    spotlight_gate "$h"
-    load_gate "$h"
+    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
     for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
       hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
-      embeds_clean "$h" "$p" \
-        || die "$(hname "$h"): $p does not embed a CLEAN +$EXPECT_SHA (same-build rule, D-2026-07-05-2)"
+      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
     done
-    # Cold-cache capability is METHODOLOGY, not a nicety — hard gate, fail closed.
-    hrun "$h" "sudo -n /usr/sbin/purge" \
-      || die "$(hname "$h") cannot purge without a password (need the NOPASSWD /usr/sbin/purge sudoers rule) — every run would read WARM"
-    hrun "$h" "pgrep -x blit-daemon >/dev/null" \
-      && die "$(hname "$h"): a blit-daemon is already running — stop it first (stale-daemon refusal)"
-    # Fixtures.
-    local w want got
+    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
+    if hrun "$h" "pgrep -x blit-daemon >/dev/null 2>&1"; then die "$(hname "$h"): a blit-daemon is already running — stop it first"; fi
     for w in large mixed small; do
-      want="$(eval echo "\$FIX_COUNT_$w")"
+      want="$(eval echo "\$FIX_COUNT_$w")"; wantb="$(eval echo "\$FIX_BYTES_$w")"
       got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
-      [[ "${got:-0}" == "$want" ]] \
-        || die "$(hname "$h"): src_$w has ${got:-0}/$want files — stage the fixtures before a timed run"
+      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
+      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
+        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
     done
+    link_gate "$h"
   done
-  # Link validity, MEASURED not assumed (.agents/machines.md): the peer's ARP entry
-  # must be the PEER's MAC, never our own — a host route on a directly-connected
-  # subnet installs a BLACK HOLE that still reports the right interface.
-  local pmac
-  ping -c1 -W1 "$Q_IP" >/dev/null 2>&1 || true
-  pmac="$(arp -n "$Q_IP" 2>/dev/null | awk '{print $4}')"
-  [[ -n "$pmac" && "$pmac" != "(incomplete)" ]] || die "no ARP entry for q ($Q_IP) — the link is not up"
-  log "preflight OK  build=$EXPECT_SHA (harness HEAD=$HARNESS_HEAD)  runs/arm=$RUNS  q_mac=$pmac"
+  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
 }
 
 write_manifest() {
   local f="$OUT_DIR/staging-manifest.txt" h
-  { echo "# harness_head=$HARNESS_HEAD binary_identity=$EXPECT_SHA"
+  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
+    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
+    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
+    echo "# cells=$CELLS"
     echo "host,role,sha,sha256,path"
     for h in n q; do
       echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
       echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$(hdaemon "$h")"),$(hdaemon "$h")"
     done; } > "$f"
-  log "staging manifest recorded (4 hashes)"
+  log "staging manifest recorded (harness sha256 + 4 binary hashes + every threshold)"
 }
 
-# --- daemons (both ends serve: the source daemon serves pulls, the dest daemon
-#     serves pushes) --------------------------------------------------------
+# --- daemons ------------------------------------------------------------------
 N_PID=""; Q_PID=""
-daemon_start() {   # $1 = host
+daemon_start() {
   local h="$1" cfg mod bin pid
-  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"
-  cfg="$mod/mm-bench.toml"
+  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   hrun "$h" "mkdir -p '$mod'
 printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
 nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
-sleep 2
-pgrep -x blit-daemon | head -1" >/dev/null 2>&1 || true
+sleep 2" >/dev/null 2>&1 || true
   pid="$(hrun "$h" "pgrep -x blit-daemon | head -1" | nocr | tr -cd '0-9')"
   [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start (see $(hmod "$h")/mm-daemon.log)"
-  # Listening, not merely alive (the rig-W lesson: the process check passed while
-  # the socket was not accepting, and the smoke died on a transport error).
-  hrun "$h" "nc -z -G 3 127.0.0.1 $PORT" \
-    || die "$(hname "$h"): daemon pid $pid is up but NOT listening on $PORT"
   [[ "$h" == n ]] && N_PID="$pid" || Q_PID="$pid"
   log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
 }
-daemon_stop() {   # $1 = host; PID-scoped, comm-verified, and the death is VERIFIED
+# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
+# handshake reached some listener's backlog — not that the daemon speaks blit).
+smoke() {
+  local h="$1" o probe
+  o="$(other "$h")"
+  probe="$(hmod "$o")/mm_smoke_${SESSION_TAG}"
+  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 || true
+  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
+    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
+    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
+  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
+  log "smoke ok: $(hname "$h") daemon serves blit"
+}
+daemon_stop() {
   local h="$1" pid; pid="$([[ "$h" == n ]] && echo "$N_PID" || echo "$Q_PID")"
   [[ -n "$pid" ]] || return 0
-  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; true" >/dev/null 2>&1 || true
-  if hrun "$h" "ps -p $pid >/dev/null 2>&1"; then
+  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; echo __DONE__" >/dev/null 2>&1 || true
+  # A teardown that cannot be VERIFIED is a failure, not a success (codex MEDIUM).
+  if hrun "$h" "ps -p $pid >/dev/null 2>&1 && echo ALIVE || echo GONE" | nocr | grep -q ALIVE; then
     log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"
     return 1
   fi
   log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
 }
-cleanup() {
-  daemon_stop n || true
-  daemon_stop q || true
-  rm -rf "$MUX" 2>/dev/null || true
-}
+cleanup() { daemon_stop n || true; daemon_stop q || true; rm -rf "$MUX" 2>/dev/null || true; }
 trap cleanup EXIT
 
-# --- cold + drain -------------------------------------------------------------
+# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
 RUN_DRAIN=""; RUN_COLD=""
-drain_host() {   # $1 = DESTINATION host; wait until its disk is quiet (macOS iostat)
+drain_host() {
   hrun "$1" "quiet=0
 for i in \$(seq 1 $DRAIN_ITERS); do
   w=\$(iostat -d -w 2 -c 2 disk0 2>/dev/null | tail -1 | awk '{print \$3}')
   ok=\$(awk -v w=\"\${w:-99}\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
-  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained \${i}x2s\"; exit 0; fi
+  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
 done
 echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
 }
-prep_run() {   # $1 = dest host. Drain the DEST, then cold BOTH ends. A failed purge VOIDS.
-  local dh="$1" out cn=ok cq=ok
-  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"; RUN_DRAIN="${RUN_DRAIN// /_}"
-  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
+prep_run() {   # $1 = dest host
+  local dh="$1" cn=ok cq=ok out
+  # Purge BOTH ends first — the purge itself dirties the disk, so a drain
+  # certified before it proves nothing (codex HIGH).
   hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
+  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
+  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
 }
 
-# --- durability: ALWAYS the DESTINATION host, identically for both arms --------
-fsync_tree_ms() {   # $1 = DEST host, $2 = landed path. Prints ms, or NA (=> VOID).
+# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
+RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0
+fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes" or "NA 0 0"
   local out
-  out="$(hrun "$1" "python3 - '$2' <<'PYEOF'
+  out="$(hrun "$1" "sleep $(awk -v m=$SETTLE_MS 'BEGIN{printf \"%.3f\", m/1000}')
+python3 - '$2' <<'PYEOF'
 import os, sys, time
+p = sys.argv[1]
+if not os.path.isdir(p):
+    print('F:NA:0:0:F')          # a MISSING tree must never read as a fast flush
+    raise SystemExit
 t = time.monotonic()
-for root, dirs, files in os.walk(sys.argv[1]):
-    for name in files:
-        fd = os.open(os.path.join(root, name), os.O_RDONLY)
+files = 0
+nbytes = 0
+for root, _d, fs in os.walk(p):
+    for name in fs:
+        fp = os.path.join(root, name)
+        fd = os.open(fp, os.O_RDONLY)
         os.fsync(fd)
         os.close(fd)
-print('F:%d:F' % int((time.monotonic() - t) * 1000))
-PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p' | head -1)"
-  echo "${out:-NA}"   # a failed fsync must never read as a plausible flush
+        files += 1
+        nbytes += os.fstat(os.open(fp, os.O_RDONLY)).st_size if False else os.path.getsize(fp)
+print('F:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes))
+PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3/p' | head -1)"
+  echo "${out:-NA 0 0}"
 }
 
 # --- one timed run ------------------------------------------------------------
-RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
-timed_run() {   # $1=initiating host $2=src spec $3=dst spec $4=DEST host $5=landed path $6=flag
-  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" out bin
+RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
+timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
+  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin r
   bin="$(hblit "$ih")"
   prep_run "$dh"
-  # The window is self-timed ON the initiating host (locally for nagatha; inside a
-  # SINGLE ssh for q), so dispatch/round-trip is outside it by construction.
-  # NO sync in here — durability is charged to the destination, below.
-  # ONE python process brackets the transfer. Two reasons, both load-bearing:
-  #   1. time.monotonic()'s REFERENCE POINT IS UNDEFINED ACROSS PROCESSES (python
-  #      docs; only same-process differences are valid). The first draft of this
-  #      function read t0 in one `python3 -c` and t1 in another and subtracted
-  #      them — which is meaningless, and measurably so: consecutive reads on this
-  #      rig returned -1 and -4 ms. It would have produced garbage timings that
-  #      still looked plausible.
-  #   2. Interpreter startup now falls OUTSIDE the timer. With a per-invocation
-  #      clock read, startup sat INSIDE the window — and since the two arms of a
-  #      cell are initiated by DIFFERENT Macs, any startup difference between them
-  #      is charged to one arm. That is the otp-2w failure mode (a cost billed to
-  #      one arm and not the other) in a new disguise.
-  out="$(hrun "$ih" "python3 - '$bin' '$src' '$dst' '$flag' <<'PYEOF'
-import subprocess, sys, time
-binp, src, dst, flag = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
-cmd = [binp, 'copy', src, dst, '--yes'] + ([flag] if flag else [])
-err = open('/tmp/mm-client.err', 'wb')
-t = time.monotonic()
-rc = subprocess.call(cmd, stdout=subprocess.DEVNULL, stderr=err)
-print('R:%d,%d:R' % (int((time.monotonic() - t) * 1000), rc))
-PYEOF" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
+  out="$(hrun "$ih" "t0=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
+'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
+t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
+echo \"R:\$((t1-t0)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
-  RUN_FLUSH="$(fsync_tree_ms "$dh" "$landed")"
+  read -r RUN_FLUSH RUN_FILES RUN_BYTES <<<"$(fsync_tree "$dh" "$landed")"
   RUN_VALID=yes
-  if [[ "$RUN_FLUSH" == NA ]]; then RUN_VALID=no; RUN_FLUSH=0; fi
+  local wc wb; wc="$(eval echo "\$FIX_COUNT_$w")"; wb="$(eval echo "\$FIX_BYTES_$w")"
+  if [[ "$RUN_FLUSH" == NA ]]; then
+    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
+    RUN_VALID=no; RUN_FLUSH=0
+  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
+    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
+    RUN_VALID=no
+  fi
   RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
 }
 
-# --- arms: the ONLY variable is which host's CLI initiates --------------------
+# --- arms ---------------------------------------------------------------------
+# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
+# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
+# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
+# above is what makes a wrong path fatal instead of silently free.
 CUR_W=""; CUR_FLAG=""
-arm_srcinit() {    # the SOURCE host pushes into the DEST daemon
-  local cell="$1" rid="$2" sh="$3" dh="$4" landed
-  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}/src_$CUR_W"
-  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" \
-                  "$(hip "$dh"):$PORT:/bench/${SESSION_TAG}_${cell}_${rid}/" \
-                  "$dh" "$landed" "$CUR_FLAG"
-  hrun "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" >/dev/null 2>&1 || true
+arm_srcinit() {
+  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
+  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
+            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
+  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
 }
-arm_destinit() {   # the DEST host pulls from the SOURCE daemon
-  local cell="$1" rid="$2" sh="$3" dh="$4" landed
-  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}"
-  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" \
-                  "$landed" \
-                  "$dh" "$landed" "$CUR_FLAG"
-  hrun "$dh" "rm -rf '$landed'" >/dev/null 2>&1 || true
+arm_destinit() {
+  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
+  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
+            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
+  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
 }
 
-CSV="$OUT_DIR/runs.csv"; echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid" > "$CSV"
+CSV="$OUT_DIR/runs.csv"
+echo "cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
 META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"
 
-run_pair_loop() {   # $1=cell $2=src host $3=dest host
+run_pair_loop() {
   local cell="$1" sh="$2" dh="$3"
   local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   while (( valid < RUNS && attempts < max )); do
     attempts=$(( attempts + 1 ))
-    local order pair=yes rowA="" rowB="" arm rid aname init
+    local order pair=yes rowA="" rowB="" arm aname init rid run
     if (( slot % 2 )); then order="A B"; else order="B A"; fi
     for arm in $order; do
       if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
-      rid="${aname}_s${slot}a${attempts}"
-      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh"
-      else arm_destinit "$cell" "$rid" "$sh" "$dh"; fi
+      rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
+      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh" "$run"
+      else arm_destinit "$cell" "$rid" "$sh" "$dh" "$run"; fi
       [[ "$RUN_VALID" == yes ]] || pair=no
-      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
+      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
       [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
-      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
+      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
     done
     echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
     if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
@@ -401,100 +433,10 @@ run_pair_loop() {   # $1=cell $2=src host $3=dest host
 }
 
 compute_verdicts() {
-  python3 - "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" "$OUT_DIR/paired.csv" <<'PY'
-import csv, sys
-runs_p, meta_p, sum_p, ver_p, pair_p = sys.argv[1:6]
-rows = list(csv.DictReader(open(runs_p)))
-meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
-by, void = {}, {}
-# PAIRED slots: the pre-registered noise model. Each ABBA slot yields a matched
-# (srcinit, destinit) pair under identical conditions, so d_i = destinit - srcinit
-# is a WITHIN-slot difference — no between-session drift can enter it. pf-0's
-# review established that an unpaired spread is NOT a noise floor.
-slots = {}
-for r in rows:
-    k = (r["cell"], r["arm"])
-    if r["valid"] == "yes":
-        by.setdefault(k, []).append(int(r["ms"]))
-        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
-    else:
-        void[k] = void.get(k, 0) + 1
-
-def med(v):
-    v = sorted(v); n = len(v)
-    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
-
-def complete(c):
-    if c not in meta or meta[c]["complete"] != "yes":
-        return False
-    arms = [a for (cc, a) in by if cc == c]
-    return "srcinit" in arms and "destinit" in arms
-
-with open(sum_p, "w") as f:
-    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,pairs_attempted,runs\n")
-    for (c, a) in sorted(by):
-        if not complete(c):
-            continue
-        v = by[(c, a)]
-        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
-        # every run is printed: pf-0 found the fast arm BIMODAL, and a median
-        # alone hides a mode-mixture shift that would fake a recovery.
-        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s,%s\n" % (
-            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
-            void.get((c, a), 0), meta[c]["pairs_attempted"],
-            " ".join(str(x) for x in v)))
-
-# The paired statistics the pre-registered rule is actually graded on.
-#   D = median(d_i)  -> the effect (positive = destination-initiated is slower)
-#   S = spread(d_i)  -> the PAIRED noise floor (max-min; IQR also reported)
-#   MDE = S          -> conservatively, the smallest |D| this cell can resolve
-# DELTA_REF = 230 ms: rig W's measured Delta_P1, the effect size this rig must be
-# able to see before any "vanishes" claim is permitted (the POWER GATE).
-DELTA_REF = 230
-with open(pair_p, "w") as f:
-    f.write("cell,n_pairs,D_median_ms,S_spread_ms,IQR_ms,MDE_ms,fast_arm_ms,"
-            "delta_ref_ms,ref_ratio_on_fast_arm,powered_for_null,d_i\n")
-    for c in sorted(meta):
-        ds = sorted(v["destinit"] - v["srcinit"]
-                    for (cc, _r), v in slots.items()
-                    if cc == c and "srcinit" in v and "destinit" in v)
-        if not ds:
-            continue
-        n = len(ds)
-        D = med(ds)
-        S = max(ds) - min(ds)
-        q1, q3 = ds[n // 4], ds[(3 * n) // 4 - (1 if n % 4 == 0 else 0)]
-        fast = min(med(by[(c, "srcinit")]), med(by[(c, "destinit")])) if complete(c) else 0
-        # A 230 ms effect is only VISIBLE against a ratio bar if the fast arm is
-        # fast enough: at a 2.3 s fast arm, 230 ms IS exactly 1.10 and sits ON the
-        # bar. So the null branch needs BOTH: MDE <= DELTA_REF, and a ref-sized
-        # effect that would actually breach 1.10 here.
-        ref_ratio = (fast + DELTA_REF) / fast if fast else 0.0
-        powered = "yes" if (S <= DELTA_REF and 10 * (fast + DELTA_REF) > 11 * fast) else "NO"
-        f.write("%s,%d,%d,%d,%d,%d,%d,%d,%.3f,%s,%s\n" % (
-            c, n, D, S, q3 - q1, S, fast, DELTA_REF, ref_ratio, powered,
-            " ".join(str(x) for x in ds)))
-
-with open(ver_p, "w") as f:
-    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,D_ms,S_ms,bar,outcome,powered_for_null\n")
-    for c in sorted(meta):
-        if not complete(c):
-            f.write("%s,invariance,srcinit,destinit,,,,,,1.10,INCOMPLETE,\n" % c)
-            continue
-        s, d = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
-        hi, lo = max(s, d), min(s, d)
-        # integer-exact bar (10*hi <= 11*lo) — never the printed 3-decimal ratio
-        outcome = "PASS" if 10 * hi <= 11 * lo else "FAIL"
-        ds = sorted(v["destinit"] - v["srcinit"]
-                    for (cc, _r), v in slots.items()
-                    if cc == c and "srcinit" in v and "destinit" in v)
-        D = med(ds) if ds else 0
-        S = (max(ds) - min(ds)) if ds else 0
-        fast = lo
-        powered = "yes" if (ds and S <= DELTA_REF and 10 * (fast + DELTA_REF) > 11 * fast) else "NO"
-        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,%d,1.10,%s,%s\n" % (
-            c, s, d, (hi / lo) if lo else 0.0, D, S, outcome, powered))
-PY
+  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" CONTROL_CELLS="$CONTROL_CELLS" \
+  python3 "$SCRIPT_DIR/otp12pf_mac_verdict.py" \
+    "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/paired.csv" \
+    "$OUT_DIR/verdicts.csv" "$OUT_DIR/session_verdict.txt"
 }
 
 main() {
@@ -505,27 +447,27 @@ main() {
     exit 0
   fi
   log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
-  daemon_start n
-  daemon_start q
+  daemon_start n; daemon_start q
+  smoke n; smoke q
 
   local carrier w flag cell
   for w in mixed large small; do
     for carrier in tcp grpc; do
       [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
       CUR_W="$w"; CUR_FLAG="$flag"
-      cell="nq_${carrier}_${w}"                       # data nagatha -> q
-      want_cell "$cell" && run_pair_loop "$cell" n q
-      cell="qn_${carrier}_${w}"                       # data q -> nagatha
-      want_cell "$cell" && run_pair_loop "$cell" q n
+      cell="nq_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" n q
+      cell="qn_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" q n
     done
   done
 
   compute_verdicts
   log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
-  log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/cell; ABBA) ==="
+  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
   column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
-  log "=== VERDICTS (computed, NOT declared — read the pre-registered rule) ==="
-  column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
+  log "=== PAIRED STATS (the rule is graded on these) ==="
+  column -t -s, "$OUT_DIR/paired.csv" | tee -a "$OUT_DIR/bench.log"
+  log "=== SESSION VERDICT (computed by the harness from the PRE-REGISTERED rule) ==="
+  cat "$OUT_DIR/session_verdict.txt" | tee -a "$OUT_DIR/bench.log"
   log "runs: $CSV"
 }
 main "$@"
diff --git a/scripts/otp12pf_mac_verdict.py b/scripts/otp12pf_mac_verdict.py
new file mode 100644
index 0000000..b4d42b7
--- /dev/null
+++ b/scripts/otp12pf_mac_verdict.py
@@ -0,0 +1,253 @@
+#!/usr/bin/env python3
+"""Mechanize the Mac<->Mac pre-registered decision rule.
+
+docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md is the spec. The harness
+must COMPUTE the verdict, not leave it to be applied by hand after the numbers
+are visible -- that is what pre-registration exists to prevent (codex BLOCKER 1).
+
+The noise statistic is a PAIRED inference, not a range. A range (max-min) grows
+with n and is dominated by outliers, so a large consistent effect can hide under
+it: with srcinit=2000 and d=[0,180,180,190,190,200,200,200] a range rule reports
+"VANISHES" despite 7/8 positive pairs and an effect 83% the size of rig W's P1
+(codex BLOCKER 2). Instead:
+
+  d_i  = destinit_i - srcinit_i     (positive = destination-initiated is slower)
+  D    = median(d_i)
+  CI   = 95% bootstrap CI on the median (seeded => the verdict is deterministic)
+  sign = exact two-sided binomial test on the count of positive d_i
+
+  BAR_BREACH = the effect that would push this cell's ratio to the 1.10 bar
+             = 0.10 * srcinit_median
+
+  REPRODUCES : bar FAILS and CI_lo > 0            (a real, bar-breaking slowdown)
+  INVERSION  : bar FAILS and CI_hi < 0            (source-initiated is the slow arm)
+  VANISHES   : bar PASSES and |CI| lies strictly inside +/-BAR_BREACH
+               -> a genuine EQUIVALENCE result: an effect big enough to matter is
+                  EXCLUDED, not merely unobserved.
+  PARTIAL    : bar PASSES, CI excludes 0, but the effect is not excluded as small
+  UNDERPOWERED: bar PASSES and the CI is too wide to exclude a bar-breaching
+               effect -> a null here is INCONCLUSIVE, never "P1 vanishes".
+"""
+import csv, os, random, sys
+from math import comb
+
+runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]
+DELTA_REF = int(os.environ.get("DELTA_REF_MS", "230"))
+VERDICT_CELLS = os.environ.get("VERDICT_CELLS", "").split(",")
+CONTROL_CELLS = os.environ.get("CONTROL_CELLS", "").split(",")
+
+rows = list(csv.DictReader(open(runs_p)))
+meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
+
+by, slots, void = {}, {}, {}
+for r in rows:
+    key = (r["cell"], r["arm"])
+    if r["valid"] == "yes":
+        by.setdefault(key, []).append(int(r["ms"]))
+        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
+    else:
+        void[key] = void.get(key, 0) + 1
+
+
+def med(v):
+    """Low median for even n, stated and applied consistently (codex LOW)."""
+    v = sorted(v)
+    return v[(len(v) - 1) // 2]
+
+
+def complete(c):
+    if c not in meta or meta[c]["complete"] != "yes":
+        return False
+    arms = [a for (cc, a) in by if cc == c]
+    return "srcinit" in arms and "destinit" in arms
+
+
+def boot_ci(d, iters=10000, seed=12345):
+    """95% bootstrap CI on the median. Seeded: the verdict must be reproducible."""
+    rng = random.Random(seed)
+    n = len(d)
+    meds = sorted(med([d[rng.randrange(n)] for _ in range(n)]) for _ in range(iters))
+    return meds[int(0.025 * iters)], meds[int(0.975 * iters) - 1]
+
+
+def sign_p(d):
+    """Exact two-sided binomial test on the count of positive differences."""
+    nz = [x for x in d if x != 0]
+    n = len(nz)
+    if n == 0:
+        return 1.0, 0, 0
+    k = sum(1 for x in nz if x > 0)
+    tail = sum(comb(n, i) for i in range(0, min(k, n - k) + 1))
+    return min(1.0, 2.0 * tail / (2 ** n)), k, n
+
+
+# ---- summary: every run printed (pf-0's bistability lesson) ------------------
+with open(sum_p, "w") as f:
+    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,runs\n")
+    for (c, a) in sorted(by):
+        if not complete(c):
+            continue
+        v = by[(c, a)]
+        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
+        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s\n" % (
+            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
+            void.get((c, a), 0), " ".join(str(x) for x in v)))
+
+# ---- paired stats + per-cell outcome ----------------------------------------
+cell_outcome, cell_detail = {}, {}
+with open(pair_p, "w") as f:
+    f.write("cell,n_pairs,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,"
+            "sign_p,k_pos_of_n,bar_breach_ms,delta_ref_ms,powered_for_null,unstable,outcome\n")
+    for c in sorted(meta):
+        if not complete(c):
+            cell_outcome[c] = "INCOMPLETE"
+            f.write("%s,,,,,,,,,,,,,,,INCOMPLETE\n" % c)
+            continue
+        d = [v["destinit"] - v["srcinit"]
+             for (cc, _run), v in sorted(slots.items())
+             if cc == c and "srcinit" in v and "destinit" in v]
+        s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
+        hi, lo = max(s_med, d_med), min(s_med, d_med)
+        bar = "PASS" if 10 * hi <= 11 * lo else "FAIL"      # integer-exact
+        D = med(d)
+        ci_lo, ci_hi = boot_ci(d)
+        p, k, n = sign_p(d)
+        breach = 0.10 * s_med                                # effect that reaches 1.10
+        powered = (ci_hi - ci_lo) < breach                   # can we exclude a breaching effect?
+
+        # UNSTABLE, as a STATISTIC not a vibe: an arm splits into two clusters
+        # separated by more than the paired spread, AND the bar verdict flips when
+        # graded on pooled runs instead of medians.
+        unstable = "no"
+        for arm in ("srcinit", "destinit"):
+            v = sorted(by[(c, arm)])
+            gaps = [(v[i + 1] - v[i], i) for i in range(len(v) - 1)]
+            gmax, gi = max(gaps) if gaps else (0, 0)
+            if gmax > (max(d) - min(d)) and gmax > 0:
+                pooled_hi = max(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
+                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
+                pooled_lo = min(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
+                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
+                pooled_bar = "PASS" if 10 * pooled_hi <= 11 * pooled_lo else "FAIL"
+                if pooled_bar != bar:
+                    unstable = "yes"
+
+        if bar == "FAIL" and ci_lo > 0:
+            out = "REPRODUCES"
+        elif bar == "FAIL" and ci_hi < 0:
+            out = "INVERSION"
+        elif bar == "PASS" and ci_lo > -breach and ci_hi < breach:
+            out = "VANISHES"
+        elif bar == "PASS" and not powered:
+            out = "UNDERPOWERED"
+        elif bar == "PASS" and (ci_lo > 0 or ci_hi < 0):
+            out = "PARTIAL"
+        else:
+            out = "INCONCLUSIVE"
+        if unstable == "yes":
+            out = "UNSTABLE"
+
+        cell_outcome[c] = out
+        cell_detail[c] = dict(D=D, ci=(ci_lo, ci_hi), p=p, k=k, n=n, bar=bar,
+                              ratio=hi / lo if lo else 0.0, breach=breach)
+        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%d/%d,%d,%d,%s,%s,%s\n" % (
+            c, len(d), s_med, d_med, (hi / lo if lo else 0.0), bar, D, ci_lo, ci_hi,
+            p, k, n, breach, DELTA_REF, "yes" if powered else "no", unstable, out))
+
+# ---- per-cell invariance rows (unchanged shape) ------------------------------
+with open(ver_p, "w") as f:
+    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,delta_ms,bar,outcome\n")
+    for c in sorted(meta):
+        if not complete(c):
+            f.write("%s,invariance,srcinit,destinit,,,,,1.10,INCOMPLETE\n" % c)
+            continue
+        s, dd = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
+        hi, lo = max(s, dd), min(s, dd)
+        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,1.10,%s\n" % (
+            c, s, dd, hi / lo if lo else 0.0, dd - s,
+            "PASS" if 10 * hi <= 11 * lo else "FAIL"))
+
+# ---- SESSION VERDICT: the six registered outcomes, in strict precedence ------
+lines = []
+ctrl = [c for c in CONTROL_CELLS if c in cell_outcome]
+verd = [c for c in VERDICT_CELLS if c in cell_outcome]
+
+ctrl_fail = [c for c in ctrl
+             if cell_outcome[c] not in ("VANISHES", "INCONCLUSIVE", "UNDERPOWERED")
+             and cell_detail.get(c, {}).get("bar") == "FAIL"]
+incomplete = [c for c in (ctrl + verd) if cell_outcome[c] == "INCOMPLETE"]
+
+if incomplete:
+    verdict = "INCOMPLETE"
+    why = "cells did not complete: %s" % ", ".join(incomplete)
+elif ctrl_fail:
+    # 1. RIG-VOID -- a rig whose control fails cannot adjudicate a TCP-only claim.
+    verdict = "RIG-VOID"
+    why = ("control cell(s) FAILED the 1.10 bar: %s. The rig is not measuring "
+           "cleanly; NO verdict may be read." % ", ".join(ctrl_fail))
+else:
+    outs = {c: cell_outcome[c] for c in verd}
+    repro = [c for c, o in outs.items() if o == "REPRODUCES"]
+    inv = [c for c, o in outs.items() if o == "INVERSION"]
+    unst = [c for c, o in outs.items() if o == "UNSTABLE"]
+    van = [c for c, o in outs.items() if o == "VANISHES"]
+    part = [c for c, o in outs.items() if o == "PARTIAL"]
+    under = [c for c, o in outs.items() if o in ("UNDERPOWERED", "INCONCLUSIVE")]
+
+    if unst:
+        verdict = "UNSTABLE"
+        why = ("bimodal arm(s) whose verdict flips on pooled runs: %s. Report as "
+               "unstable, NOT resolved." % ", ".join(unst))
+    elif repro and inv:
+        verdict = "MIXED-SIGN"
+        why = ("reproduces in %s but INVERTS in %s -- a host x role interaction "
+               "this rig cannot decompose. INCONCLUSIVE for the pairing question."
+               % (", ".join(repro), ", ".join(inv)))
+    elif repro:
+        verdict = "REPRODUCES"
+        why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: "
+               "it shows P1 CAN occur macOS<->macOS -- it does NOT establish a "
+               "platform-general layout cost, and it does NOT kill H1 (H1 accuses "
+               "code, and that code runs here too)." % ", ".join(repro))
+    elif inv:
+        verdict = "INVERSION"
+        why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank "
+               "this as 'P1 absent'." % ", ".join(inv))
+    elif under:
+        verdict = "INCONCLUSIVE-UNDERPOWERED"
+        why = ("cells cannot exclude a bar-breaching effect: %s. A PASS here is NOT "
+               "'P1 vanishes' -- the instrument could not have seen it (pf-0's "
+               "error, pre-empted)." % ", ".join(under))
+    elif van and len(van) == len(verd):
+        verdict = "VANISHES"
+        why = ("both TCP-mixed cells EXCLUDE a bar-breaching effect (equivalence). "
+               "Scoped to THIS pair: P1 did not reproduce macOS<->macOS. That is "
+               "CONSISTENT with 'Windows is required' but does NOT prove it -- it "
+               "could be a property of these two machines/disks/OS version.")
+    elif part:
+        verdict = "PARTIAL"
+        why = ("a real but sub-bar asymmetry in: %s. Neither a reproduction nor a "
+               "vanish; pf-1 owns it." % ", ".join(part))
+    else:
+        verdict = "INCONCLUSIVE"
+        why = "no registered case matched cleanly; report the cells verbatim."
+
+lines.append("SESSION VERDICT: %s" % verdict)
+lines.append("")
+lines.append(why)
+lines.append("")
+lines.append("Per-cell outcomes (the rule is graded on paired.csv):")
+for c in sorted(cell_outcome):
+    d = cell_detail.get(c)
+    if d:
+        lines.append("  %-14s %-12s ratio=%.3f bar=%s  D=%+dms CI=[%+d,%+d] sign_p=%.3f (%d/%d pos)"
+                     % (c, cell_outcome[c], d["ratio"], d["bar"], d["D"],
+                        d["ci"][0], d["ci"][1], d["p"], d["k"], d["n"]))
+    else:
+        lines.append("  %-14s %s" % (c, cell_outcome[c]))
+lines.append("")
+lines.append("This file is COMPUTED from the pre-registered rule. It declares nothing")
+lines.append("beyond it, and the owner walks the numbers.")
+
+open(sess_p, "w").write("\n".join(lines) + "\n")
+print("\n".join(lines))
diff --git a/scripts/otp12pf_mac_verdict_test.py b/scripts/otp12pf_mac_verdict_test.py
new file mode 100644
index 0000000..f09a9c0
--- /dev/null
+++ b/scripts/otp12pf_mac_verdict_test.py
@@ -0,0 +1,87 @@
+#!/usr/bin/env python3
+"""Guard test for otp12pf_mac_verdict.py — run it before trusting a Mac<->Mac run.
+
+    python3 scripts/otp12pf_mac_verdict_test.py
+
+The defect it guards (codex round-2 BLOCKER on the harness): the first revision
+graded "did the effect vanish?" against S = max(d) - min(d), a RANGE. A range
+grows with n and is dominated by outliers, so a large CONSISTENT effect hides
+under it:
+
+    srcinit = 2000 ms;  d = [0,180,180,190,190,200,200,200]
+    -> D = 190, S = 200, bar PASSES, |D| <= S  =>  "VANISHES"
+
+...on 7/8 positive pairs, with an effect 83% the size of rig W's Delta_P1. It
+would have reported "P1 requires the Windows peer" off an effect nearly as large
+as P1 itself. The rule now uses a bootstrap CI + an equivalence bound against the
+bar-breaching effect, and this test pins that.
+"""
+import csv, os, subprocess, sys, tempfile
+
+HERE = os.path.dirname(os.path.abspath(__file__))
+VERDICT = os.path.join(HERE, "otp12pf_mac_verdict.py")
+CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")
+VERDICT_CELLS = ("nq_tcp_mixed", "qn_tcp_mixed")
+
+
+def verdict_for(d, src=2000):
+    tmp = tempfile.mkdtemp()
+    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
+    with open(runs, "w") as f:
+        w = csv.writer(f)
+        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid".split(","))
+        for cell in VERDICT_CELLS:
+            for i, di in enumerate(d, 1):
+                w.writerow([cell, "srcinit", "x", "h", i, src, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
+                w.writerow([cell, "destinit", "x", "h", i, src + di, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
+        for cell in CONTROLS:            # clean controls, so the rig is not VOID
+            for i in range(1, 9):
+                w.writerow([cell, "srcinit", "x", "h", i, 1000, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
+                w.writerow([cell, "destinit", "x", "h", i, 1005, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
+    with open(meta, "w") as f:
+        f.write("cell,pairs_attempted,complete\n")
+        for cell in VERDICT_CELLS + CONTROLS:
+            f.write("%s,8,yes\n" % cell)
+    env = dict(os.environ, DELTA_REF_MS="230",
+               VERDICT_CELLS=",".join(VERDICT_CELLS),
+               CONTROL_CELLS=",".join(CONTROLS))
+    out = subprocess.run([sys.executable, VERDICT, runs, meta,
+                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
+                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
+                         env=env, capture_output=True, text=True)
+    if out.returncode != 0:
+        raise SystemExit("verdict engine failed:\n" + out.stderr)
+    return out.stdout.splitlines()[0].split(":", 1)[1].strip()
+
+
+CASES = [
+    # (name, d, src, must_be, must_not_be)
+    ("codex counterexample: real 190ms effect, 7/8 positive",
+     [0, 180, 180, 190, 190, 200, 200, 200], 2000, None, "VANISHES"),
+    ("a genuinely absent effect",
+     [-4, -2, -1, 0, 0, 1, 2, 3], 2000, "VANISHES", None),
+    ("a bar-breaking slowdown (destination-initiated)",
+     [300, 310, 320, 330, 340, 350, 360, 370], 1000, "REPRODUCES", None),
+    ("source-initiated is the slow arm",
+     [-300, -310, -320, -330, -340, -350, -360, -370], 1000, "INVERSION", None),
+]
+
+failures = 0
+for name, d, src, must_be, must_not_be in CASES:
+    got = verdict_for(d, src)
+    ok = True
+    if must_be and got != must_be:
+        ok = False
+    if must_not_be and got == must_not_be:
+        ok = False
+    print("%-52s -> %-26s %s" % (name, got, "ok" if ok else "*** FAIL ***"))
+    if not ok:
+        failures += 1
+        if must_be:
+            print("      expected %s" % must_be)
+        if must_not_be:
+            print("      must NOT be %s (the range-rule bug is back)" % must_not_be)
+
+print()
+print("%d/%d cases passed" % (len(CASES) - failures, len(CASES)))
+sys.exit(1 if failures else 0)

codex
The first pass confirms the commit is the current `HEAD`; the canonical remote freshness check is unavailable in this sandbox, so I’m treating repository state as local evidence only. I’ve also noted an existing untracked round-2 review file and will not touch it. I’m now testing the decision engine against the specification, including adversarial synthetic inputs rather than trusting its labels.
exec
/bin/zsh -lc "nl -ba .review/results/macmac-harness.gpt-verdict.md | sed -n '1,260p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md | sed -n '1,320p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '1,700p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# macmac-harness — adjudication of the codex review (round 1)
     2	
     3	**Slice**: `e1e351d` — `scripts/bench_otp12pf_mac.sh`, the Mac↔Mac harness
     4	(+ pre-registration rev 2).
     5	**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = ultra`.
     6	**Raw review**: `.review/results/macmac-harness.codex.md`
     7	**Verdict**: NOT READY — 3 BLOCKER, 6 HIGH, 1 MEDIUM, 1 LOW.
     8	**Adjudication: 11 findings, 11 ACCEPTED, 0 rejected.**
     9	
    10	**No data has been taken. The instrument was reviewed before it measured
    11	anything** — which is the only reason none of this became a retraction. Three of
    12	these would have produced a *confidently wrong result*.
    13	
    14	---
    15	
    16	## BLOCKER 1 — the harness does not compute its own registered rule → **ACCEPTED**
    17	
    18	The pre-registration defines six ordered outcomes, a RIG-VOID gate, a power gate
    19	and an UNSTABLE override. The harness emits only per-cell `PASS/FAIL` plus paired
    20	stats. **The session verdict would therefore have been applied BY HAND, after
    21	seeing the numbers** — which is exactly what pre-registration exists to prevent.
    22	Codex also notes the prose tree is itself still overlapping/incomplete.
    23	
    24	**Fix**: the harness must mechanize the rule end-to-end and emit a
    25	`session_verdict.txt` (RIG-VOID / REPRODUCES / INVERSION / VANISHES / PARTIAL /
    26	MIXED-SIGN / INCONCLUSIVE-UNDERPOWERED / UNSTABLE), with the prose tightened to
    27	match exactly.
    28	
    29	## BLOCKER 2 — the noise statistic would have declared a REAL effect "vanished" → **ACCEPTED**
    30	
    31	`S = max(d) − min(d)` is a **range**, not an MDE or an equivalence bound: it grows
    32	with n and is dominated by outliers, so a *large, consistent* effect can hide
    33	under it. Codex's counterexample, which my code accepts:
    34	
    35	    srcinit = 2000 ms (×8);  d = [0, 180, 180, 190, 190, 200, 200, 200]
    36	    -> D = 190, S = 200, bar = PASS, powered = yes
    37	    -> |D| <= S  =>  "VANISHES"
    38	
    39	…despite **7/8 pairs positive** and `D` at **83% of rig W's Δ_P1**. Repeated in
    40	both directions it would have declared "P1 requires the Windows peer" off an
    41	effect nearly the size of P1 itself. This is pf-0's underpowered-null error
    42	wearing a power gate.
    43	
    44	**Fix**: replace the range with a real paired inference —
    45	- **bootstrap 95% CI on median(d_i)** (n=8, resampled in-process, no scipy);
    46	- **exact sign test** (k of 8 positive, two-sided binomial);
    47	- **REPRODUCES** requires bar FAIL **and** CI lower bound > 0;
    48	- **VANISHES** requires bar PASS **and** the CI **upper** bound below the
    49	  bar-breaching effect for that cell (`0.10 × srcinit_median` — the effect that
    50	  would push the ratio to 1.10), i.e. a genuine **equivalence** result;
    51	- otherwise **INCONCLUSIVE** (and **UNDERPOWERED** when the CI is too wide to
    52	  exclude a bar-breaching effect).
    53	
    54	## BLOCKER 3 — the registered inference still overreaches → **ACCEPTED**
    55	
    56	Rev 2 narrowed rev 1's "H1 dies" to "platform-general cost of the layout". Still
    57	too strong. A reproduction proves only that **P1 can occur without a Windows peer
    58	on THIS pair**; a null proves only **non-reproduction on this pair** — not that
    59	Windows is *required* (it could be a property of these two specific machines,
    60	disks, or macOS versions).
    61	
    62	**Fix**: rev 3 scopes every claim to *this pair*, and states the residual
    63	alternatives explicitly. (This is the third tightening of the same claim; the
    64	lesson is that each round I stated a conclusion one step broader than the design
    65	could carry.)
    66	
    67	## HIGH — the fsync walk is fail-open, and nothing checks that bytes landed → **ACCEPTED** *(found independently by the author before the review returned)*
    68	
    69	`os.walk()` on a missing, unreadable or empty path emits a perfectly valid
    70	`F:0:F` — **a missing tree reads as a fast, successful flush**. The push and pull
    71	landed paths are *currently* correct (verified empirically: a push to
    72	`/bench/RUNDIR/` lands `RUNDIR/src_<W>`; a pull into `RUNDIR` lands the files
    73	directly in `RUNDIR`), but that is **luck, not a guard** — and there is **no
    74	destination count or byte-sum check**, so an exit-0 zero-byte or partial transfer
    75	becomes a valid *fast* row. This is the otp-2w bug's exact shape.
    76	
    77	**Fix**: the fsync walk returns `F:<ms>:<files>:F`; the harness **VOIDs the pair**
    78	unless the landed file count equals the fixture count **and** the landed byte sum
    79	matches. Source fixtures get a byte-sum check too, not just a count.
    80	
    81	## HIGH — transfer and fsync are disjoint intervals, and the free-writeback gap REVERSES BY DIRECTION → **ACCEPTED**
    82	
    83	The sharpest finding, and the one that could have *manufactured the result*.
    84	Between the client exiting and the fsync starting, the OS writes back dirty pages
    85	**for free** (charged to neither interval). That gap is **longer for whichever arm
    86	ran over ssh**, because the ssh return trip happens first:
    87	
    88	    cell nq (src=nagatha, dest=q):  srcinit = LOCAL client,  destinit = REMOTE client
    89	    cell qn (src=q, dest=nagatha):  srcinit = REMOTE client, destinit = LOCAL client
    90	
    91	So the favoured arm **flips sign with the data direction**. P1's whole signature is
    92	*one-directional* — meaning this artifact is capable of **producing a
    93	one-directional "reproduction" out of nothing**. Codex also notes `prep_run`
    94	certifies the drain *before* `sync; purge` and never re-checks it.
    95	
    96	**Fix (needs an owner decision — see below)**: make the client launch **symmetric**
    97	so neither arm carries an ssh return the other lacks. Also re-order `prep_run` so
    98	the drain is certified *after* the purge, and re-checked.
    99	
   100	## HIGH — environmental gates fail OPEN → **ACCEPTED**
   101	
   102	`pgrep` errors read as "quiet"; `tmutil` errors/empty parse to zero; an AutoBackup
   103	**read error explicitly becomes "disabled"**; `top` failures become zero and a
   104	trailing idle `mds` sample can overwrite a busy one; malformed/empty `load1`
   105	becomes 0. Every one of these fails toward "go".
   106	
   107	**Fix**: each gate must fail **closed** — an unreadable gate is a VOID, never a
   108	pass. (This is the same class as pf-0's `ps` decaying-average trap: an instrument
   109	that cannot answer must not answer "fine".)
   110	
   111	## HIGH — the ARP/link gate does not prove the link → **ACCEPTED**
   112	
   113	It ignores ping failure, accepts *any* complete MAC without comparing it to `q`'s
   114	**known** MAC (so the documented **own-MAC black hole** passes), and never checks
   115	the q→nagatha direction or that the route uses the 10GbE NIC rather than falling
   116	back to 1GbE.
   117	
   118	**Fix**: compare against the recorded peer MAC, check **both** directions, and
   119	assert the route egresses the 10GbE interface — plus the existing rule that an ssh
   120	throughput test is **not** a valid link check.
   121	
   122	## HIGH — the registered protocol is unenforced → **ACCEPTED**
   123	
   124	`RUNS>=2` is accepted (the design says 8); a misspelled `CELLS` can silently drop
   125	every control or measure nothing; blank `CELLS` runs **12** cells, not the six
   126	registered. Overridable drain thresholds are not recorded in the evidence.
   127	
   128	**Fix**: validate `CELLS` against the registered set, require the registered
   129	`RUNS`, and record every threshold in the manifest.
   130	
   131	## HIGH — instrument provenance is weak → **ACCEPTED**
   132	
   133	The manifest records `HEAD`, so a **modified** harness still claims the reviewed
   134	commit; `sha256_of` accepts empty/malformed hashes; and `! grep` turns a
   135	*read error* on the dirty-marker check into "clean".
   136	
   137	**Fix**: hash the harness file itself into the manifest, refuse a dirty harness,
   138	and make hash/provenance failures fatal.
   139	
   140	## MEDIUM — daemon liveness and teardown → **ACCEPTED**
   141	
   142	`nc -z` proves only that a handshake reached *some* listener's backlog — not that
   143	the captured PID accepts or speaks blit. Teardown logs "verified gone" when the
   144	ssh/`ps` probe *itself* failed, and cleanup discards a positively detected
   145	survivor.
   146	
   147	**Fix**: probe with a real blit call (the smoke), and treat a survivor or an
   148	unverifiable teardown as fatal.
   149	
   150	## LOW — median/IQR conventions → **ACCEPTED**
   151	
   152	Even-sample medians are floored before the "exact" bar and the `D > S`
   153	comparisons; the n=8 IQR convention is unstated. Codex confirms the ABBA void
   154	retry, slot pairing and the `destinit − srcinit` sign are otherwise **correct**.
   155	
   156	**Fix**: state the convention and apply it consistently.
   157	
   158	---
   159	
   160	## The one finding that needs the owner: symmetric client launch
   161	
   162	Fixing the free-writeback asymmetry requires the two arms to be launched
   163	identically. The options are an infrastructure choice, not a code choice:
   164	
   165	- **(A)** drive the harness from a **third host** (skippy/magneto) so **both**
   166	  Macs are remote and symmetric — needs ssh keys from that host to both Macs;
   167	- **(B)** keep the driver on nagatha but launch **both** clients over ssh,
   168	  including nagatha→itself — needs a host key + `authorized_keys` entry on
   169	  nagatha;
   170	- **(C)** equalize with a fixed settle window before the fsync on both arms —
   171	  no infra change, but it lets writeback complete "for free" for both arms and so
   172	  weakens what destination-keyed durability is meant to charge.
   173	
   174	Recorded for the owner; **no rig time until it is resolved**, because this
   175	artifact is capable of manufacturing exactly the one-directional result the
   176	experiment is looking for.

 succeeded in 0ms:
     1	# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
     2	
     3	**Status**: Pre-registered, **revision 3**. **No data exists yet.**
     4	- Codex round 1 (of `f0343f4`, the design): NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW
     5	  → **9/9 accepted** (`.review/results/macmac-prereg.gpt-verdict.md`).
     6	- Codex round 2 (of `e1e351d`, the **instrument**): NOT READY — **3 BLOCKER** +
     7	  6 HIGH + 1 MEDIUM + 1 LOW → **11/11 accepted**
     8	  (`.review/results/macmac-harness.gpt-verdict.md`).
     9	
    10	Committed BEFORE the data so the rule cannot be authored around the numbers.
    11	**Two rounds of review have now caught, between them, an invalid inference, a
    12	statistic that would have declared a real effect absent, a fail-open durability
    13	check, and a timing artifact that reverses sign with direction — all before a
    14	single timed run.**
    15	
    16	**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).
    17	
    18	## What revision 1 got WRONG, and what this experiment actually answers
    19	
    20	Revision 1 claimed this rig "discriminates H1 outright": *P1 reproduces
    21	macOS↔macOS ⇒ H1 dies, because H1 accuses the Windows accept branch.*
    22	
    23	**That inference is invalid, and the premise is false.** H1, verbatim in the
    24	parent, accuses **blit's own code paths** — the `SourceSockets` Dial/Accept
    25	branches, `InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's
    26	synchronous dial-before-ACK at `transfer_session/mod.rs:3113`. **The word
    27	"Windows" appears nowhere in H1.** Windows is merely *who happens to be the
    28	accepting source* in P1's slow arm on rig W. The accused code runs on macOS too.
    29	So a Mac↔Mac reproduction is **consistent with H1**, not fatal to it — and the
    30	parent already warns that *"'consistent with H1' is not confirmation."*
    31	
    32	The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
    33	accept branch") and copied without checking H1's text. **That is a repo error and
    34	it is corrected wherever it appears.**
    35	
    36	**What this rig CAN answer — and revision 2 STILL overstated it (round-2 BLOCKER).**
    37	Rev 2 asked whether P1 is "a platform-general cost of the layout". A rig with two
    38	machines cannot license that. The claim is now scoped to **this pair**:
    39	
    40	> **Can P1 occur WITHOUT a Windows peer — on this pair of Macs?**
    41	
    42	| outcome | what it licenses — and its limit |
    43	|---|---|
    44	| **P1 REPRODUCES** | P1 **does not require a Windows peer** (on this pair). It is therefore **not** "platform residue" that could be waived under the D-2026-07-12-1 shape, and every code-level hypothesis strengthens. **Limits**: it does **not** establish a platform-*general* cost (two Macs are not "all platforms"), it does **not** name the mechanism, and it does **not** kill H1 — the code H1 accuses runs here too, so a reproduction is *consistent with* H1. |
    45	| **P1 does NOT reproduce (null)** | P1 **did not occur on this pair**. That is **consistent with** "the Windows peer is required" — but does **not prove it**: it could equally be a property of *these two machines*, their disks, or this macOS version. It does **not** confirm H1 either. |
    46	
    47	A null is only reportable at all if the rig could have **seen** a rig-W-sized
    48	effect — see the POWER GATE. Otherwise it is `INCONCLUSIVE-UNDERPOWERED`.
    49	
    50	Either outcome materially reshapes the hypothesis space and bears directly on
    51	whether P1 **must be fixed in code** or **could be accepted as platform residue**.
    52	That is why it runs before pf-1. **It is not an H1 kill/confirm and this document
    53	must never be cited as one.**
    54	
    55	## Rig
    56	
    57	- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
    58	- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
    59	- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
    60	  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
    61	  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
    62	  deliberate.
    63	
    64	**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Revision 1 claimed
    65	it did. It does not: switching the initiator also **reassigns which machine runs
    66	the CLI and which runs the daemon**, and `q` is the faster Mac. Only
    67	arm-independent costs cancel; **host×role interactions do not.** This is handled
    68	by *measuring both data directions and reporting them separately* (below), not by
    69	assertion — and any conclusion that depends on the cancellation being perfect is
    70	out of bounds.
    71	
    72	## Cells
    73	
    74	Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
    75	**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
    76	`destinit` (dest's CLI pulls).
    77	
    78	    CELLS = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
    79	            nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
    80	            nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)
    81	
    82	`RUNS=8`, ABBA-counterbalanced, pair-void.
    83	
    84	**Both directions are measured, but a reproduction is NOT required in both
    85	(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
    86	`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
    87	directions here would rewrite the finding. So: **a reproduction in EITHER
    88	direction demonstrates the layout cost without a Windows peer.** Whether it is
    89	direction-symmetric is reported as a descriptive fact — and, because the two
    90	directions differ in *which machine is the destination*, a one-directional result
    91	is explicitly **not** dismissible as "machine asymmetry" (revision 1 did exactly
    92	that, which would have let a real reproduction be waved away).
    93	
    94	## The paired statistic — and why revision 2's was BROKEN (round-2 BLOCKER)
    95	
    96	Rev 1 used `N` = max |ratio−1| over four control cells: four point estimates from
    97	different carriers, fixtures and destinations — not a noise floor at all. Rev 2
    98	replaced it with the paired difference and `S = spread(d_i)` as the noise. **That
    99	is still broken**, because a *range* grows with n and is dominated by outliers, so
   100	a **large, consistent effect can hide under it**. Codex's counterexample, which
   101	rev 2's rule accepted:
   102	
   103	    srcinit = 2000 ms (×8);   d = [0, 180, 180, 190, 190, 200, 200, 200]
   104	    -> D = 190, S = 200, bar PASSES, |D| <= S   =>   rev 2 says "VANISHES"
   105	
   106	…on **7/8 positive pairs** and an effect **83% the size of rig W's Δ_P1**. It
   107	would have reported "P1 requires the Windows peer" off an effect nearly as big as
   108	P1 itself.
   109	
   110	**Replaced with a real paired inference** (computed by
   111	`scripts/otp12pf_mac_verdict.py`, and guarded by a test that asserts the
   112	counterexample above no longer returns VANISHES):
   113	
   114	    per ABBA slot i:  d_i = destinit_i − srcinit_i     (positive = P1's direction)
   115	      D    = median(d_i)
   116	      CI   = 95% BOOTSTRAP CI on the median (10k resamples, SEEDED -> deterministic)
   117	      sign = exact two-sided binomial test on the count of positive d_i
   118	      BAR_BREACH = 0.10 × srcinit_median   <- the effect that would reach the 1.10 bar
   119	
   120	The median convention is the **low median** for even n, stated once and applied
   121	everywhere (round-2 LOW).
   122	
   123	## POWER GATE — a null must be an EQUIVALENCE result, not an absence of evidence
   124	
   125	pf-0 reported a KILL with an instrument that could not resolve the effect it
   126	killed. This design pre-empts that:
   127	
   128	- A **null is only reportable** if the CI **excludes a bar-breaching effect** —
   129	  i.e. the whole CI lies strictly inside ±`BAR_BREACH`. That is a genuine
   130	  *equivalence* claim: "an effect big enough to matter is ruled out."
   131	- If the CI is **too wide** to exclude it, the cell is **UNDERPOWERED** and the
   132	  session verdict is **INCONCLUSIVE-UNDERPOWERED**. A PASS is then *not*
   133	  "P1 vanishes" — it is "this rig could not have seen it".
   134	- A **reproduction** needs no such gate: an effect that is seen is seen.
   135	
   136	## Decision rule — computed BY THE HARNESS, exhaustive, in strict precedence
   137	
   138	The harness emits `session_verdict.txt`. **The verdict is not applied by hand
   139	after the numbers are visible** (round-2 BLOCKER: rev 2's harness computed only
   140	PASS/FAIL, which would have left the rule to me, post-hoc).
   141	
   142	Per cell (integer-exact bar `10·hi ≤ 11·lo`, never the printed ratio):
   143	
   144	| cell outcome | condition |
   145	|---|---|
   146	| **REPRODUCES** | bar **FAILS** and `CI_lo > 0` |
   147	| **INVERSION** | bar **FAILS** and `CI_hi < 0` |
   148	| **VANISHES** | bar **PASSES** and the CI lies strictly inside ±`BAR_BREACH` |
   149	| **UNDERPOWERED** | bar **PASSES** and the CI cannot exclude `BAR_BREACH` |
   150	| **PARTIAL** | bar **PASSES**, CI excludes 0, effect not excluded as small |
   151	| **UNSTABLE** | (override) an arm is bimodal *and* the bar verdict flips on pooled runs |
   152	
   153	Session precedence (first match wins; every cell's own outcome is still recorded):
   154	
   155	1. **INCOMPLETE** — any cell short of its pairs.
   156	2. **RIG-VOID** — any **control** cell FAILS the bar. A rig whose gRPC/large
   157	   control fails cannot adjudicate a TCP-only claim. No verdict is read.
   158	3. **UNSTABLE** — a bimodal arm whose verdict flips. Reported as unstable, not
   159	   resolved.
   160	4. **MIXED-SIGN** — one direction REPRODUCES and the other INVERTS: a host×role
   161	   interaction this rig **cannot decompose**. Inconclusive for the question.
   162	5. **REPRODUCES** — either direction. → *P1 can occur without a Windows peer, on
   163	   this pair.*
   164	6. **INVERSION** — a new finding; never banked as "P1 absent".
   165	7. **INCONCLUSIVE-UNDERPOWERED** — the null branch is unavailable.
   166	8. **VANISHES** — both TCP×mixed cells exclude a bar-breaching effect.
   167	9. **PARTIAL** — a real but sub-bar asymmetry; pf-1 owns it.
   168	
   169	**No outcome may be reported that is not one of these.**
   170	
   171	**Bistability is a STATISTIC, not a vibe.** pf-0 found the rig-W fast arm bimodal,
   172	where the mode *mixture* moved a median 72 ms at constant conditions. Here: an arm
   173	whose runs split into two clusters separated by more than the paired spread, **and**
   174	whose bar verdict flips when graded on pooled runs rather than medians, is
   175	**UNSTABLE**. All 8 runs of every arm are printed in `summary.csv`, so this is
   176	checkable rather than asserted.
   177	
   178	## The instrument — two defects that could have MANUFACTURED the result (round-2 HIGH)
   179	
   180	**1. The durability check was fail-open.** `os.walk()` on a missing, unreadable or
   181	empty path returns **0 files in 0 ms** — a missing tree reads as a *fast,
   182	successful flush*. The two arms need **different** landed paths, because blit uses
   183	rsync-style slash semantics (verified empirically: a push to `/bench/RUNDIR/` lands
   184	the tree at `RUNDIR/src_<W>`; a pull into `RUNDIR` lands the files **directly in**
   185	`RUNDIR`). A wrong path would have charged one arm **zero** durability while the
   186	other paid full — the otp-2w bug that once manufactured P1.
   187	**Fixed**: the fsync walk returns its **file count and byte sum**, and the pair
   188	**VOIDs** unless both match the fixture exactly. An exit-0 zero-byte or partial
   189	transfer can no longer become a valid *fast* row.
   190	
   191	**2. The free-writeback gap REVERSED SIGN WITH DIRECTION.** Between a client
   192	exiting and the fsync starting, the OS writes back dirty pages **for free**, and
   193	that gap is longer for whichever arm ran over ssh:
   194	
   195	    cell nq (dest = q):        srcinit = LOCAL client,  destinit = REMOTE client
   196	    cell qn (dest = nagatha):  srcinit = REMOTE client, destinit = LOCAL client
   197	
   198	So the *favoured arm flips with the data direction*. Since P1's signature is
   199	**one-directional**, this artifact is capable of producing a one-directional
   200	"reproduction" **out of nothing**.
   201	**Measured before fixing** (the instrument is verified, not assumed): a pre-fsync
   202	delay of **10 / 20 / 200 ms produced no measurable change in fsync time**
   203	(72–94 ms, no trend) — APFS fsync here is per-file-metadata bound, not writeback
   204	bound. **Fixed anyway, structurally**: a fixed, equal `SETTLE_MS` (250 ms) precedes
   205	the fsync on **both** arms, so the asymmetry is removed by construction without
   206	weakening what durability charges.
   207	
   208	## Gates — fail-closed (round-1 HIGH: revision 1 only *warned*; round-2 HIGH: they all failed OPEN)
   209	
   210	A run that misses any of these is **VOID**, not "close enough". **Every gate fails
   211	CLOSED**: a gate that cannot answer must never answer "fine" (round 2 found
   212	`pgrep` errors reading as "quiet", a `tmutil` read error reading as "disabled",
   213	`top` failures reading as 0% — the same class as pf-0's `ps` decaying average that
   214	reported a *finished* backup as 255%).
   215	
   216	- **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
   217	  **either** Mac (both are bench **ends** here — nagatha is no longer just the
   218	  driver). *(Already proven live: this gate fired on its first invocation and
   219	  refused to start while the codex review of revision 1 was running.)*
   220	- **TIME MACHINE, BOTH MACS — FAIL-CLOSED, not a warning.** Refuse to start if a
   221	  backup is running **or if autobackup is merely ENABLED**, because macOS repeats
   222	  hourly and a backup can begin *inside* the window (pf-0's did, 1 minute before
   223	  the run; one destination is a network share on `skippy` — the same 10 GbE
   224	  fabric). Revision 1 downgraded this to a warning; that is exactly the hole pf-0
   225	  exposed.
   226	- **SPOTLIGHT, BOTH MACS.** `mds_stores` is a recorded contaminant
   227	  (`.agents/machines.md`). Refuse to start while it is actively indexing.
   228	- **LOAD THRESHOLD.** `load1` recorded on both Macs at start **and end**; a start
   229	  `load1` above **3.0** on either end VOIDs the session (the Macs idle at ~1.5–2).
   230	- **Cold caches both ends every run** via `sudo -n /usr/sbin/purge` (NOPASSWD
   231	  granted on both); a failed purge **VOIDS the pair** — a warm row is worse than
   232	  no row.
   233	- **Destination-keyed durability, never verb-keyed**: the macOS per-file `fsync`
   234	  walk runs **on the destination host for both arms**, is **timed**, and a failed
   235	  walk returns `NA` → the pair **VOIDS** (it must never read as a plausible flush).
   236	  (The otp-2w rule: a sync inside the initiator's bracket charges the pull arm for
   237	  writeback the push arm gets free and *manufactures* invariance failures — the
   238	  gRPC control is what exposed it.)
   239	- **Drain**: destination disk quiet before each timed window — macOS `iostat`,
   240	  `< 2 MB/s` for 3 consecutive 2 s samples; `DRAIN-TIMEOUT` VOIDs the pair.
   241	- **Fixtures verified by count on both ends** (`large` 1, `mixed` 5001,
   242	  `small` 10000) before any timed run — the arms must read the same trees.
   243	- **Provenance**: clean `+f35702a` on all four binaries; sha256 staging manifest.
   244	- **Link validity, measured not assumed**: peer ARP resolves to the **peer's** MAC
   245	  (a host route on a directly-connected subnet installs a black hole that still
   246	  reports the right interface); an ssh throughput test **cannot** validate this
   247	  link (~79 MB/s either way regardless) and is not used.
   248	
   249	## What this does NOT establish
   250	
   251	- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
   252	- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
   253	  pair is staged on the Macs. P2 on the Linux rig is a separate, outstanding
   254	  experiment.
   255	- **A null here cannot retract rig W.** P1 is measured and reproduced four times
   256	  there; this rig can only speak about the *pairing*.
   257	- **It cannot decompose host×role.** nagatha and `q` differ; outcome 6 exists
   258	  precisely because that case is beyond this rig.
   259	
   260	## The runs
   261	
   262	```sh
   263	EXPECT_SHA=f35702a RUNS=8 \
   264	  CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large \
   265	  bash scripts/bench_otp12pf_mac.sh
   266	```
   267	
   268	Harness: `scripts/bench_otp12pf_mac.sh`. It **computes; it declares nothing** —
   269	the verdict is read off the rule above.

 succeeded in 0ms:
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
     4	# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
     5	# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
     6	# =============================================================================
     7	#
     8	# WHY THIS RIG EXISTS
     9	# -------------------
    10	# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
    11	# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
    12	# cell. It answers ONE question, SCOPED TO THIS PAIR:
    13	#
    14	#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
    15	#
    16	#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
    17	#     "platform residue" that can be waived; code-level hypotheses strengthen.
    18	#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
    19	#     "Windows is required", but does NOT prove it: it could equally be a
    20	#     property of these two machines, their disks, or this macOS version.
    21	#
    22	# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
    23	# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
    24	# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
    25	# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
    26	# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
    27	# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
    28	#
    29	# WHAT IT MEASURES
    30	#   cell = <nq|qn>_<carrier>_<fixture>;  nq_* = data nagatha->q, qn_* = q->nagatha
    31	#   arms (the ONLY variable): srcinit (source's CLI pushes) / destinit (dest's CLI
    32	#   pulls). BOTH directions are measured, but a reproduction is NOT required in
    33	#   both — P1's rig-W signature is ONE-DIRECTIONAL (wm FAILS, mw PASSES), so
    34	#   demanding both would rewrite the finding.
    35	#
    36	#   Endpoint asymmetry does NOT cancel: switching the initiator also reassigns
    37	#   which Mac runs the CLI vs the daemon, and q is faster. Both directions are
    38	#   therefore reported separately and no conclusion leans on cancellation.
    39	#
    40	# THE INSTRUMENT IS THE RISK (three claims have been retracted to harness bugs).
    41	# Everything below fails CLOSED. Codex review of the first revision found 11
    42	# defects (3 BLOCKER) in this file before it measured anything; they are fixed
    43	# here and named at their site.
    44	#
    45	#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER THE INITIATOR/VERB, and
    46	#     the fsync walk VERIFIES WHAT IT FLUSHED: it returns the file count and byte
    47	#     sum, and the pair VOIDS unless they match the fixture exactly. (os.walk of a
    48	#     missing/empty path returns 0 files in 0 ms and reads as a FAST SUCCESSFUL
    49	#     FLUSH — the otp-2w bug's exact shape. Verified empirically: a push to
    50	#     /bench/RUNDIR/ lands RUNDIR/src_<W>, a pull into RUNDIR lands files directly
    51	#     in RUNDIR, so the two arms need DIFFERENT landed paths and a wrong one would
    52	#     silently charge an arm nothing.)
    53	#   * A FIXED, EQUAL SETTLE (SETTLE_MS) precedes the fsync on BOTH arms. Between
    54	#     a client exiting and the fsync starting, the OS writes back dirty pages FOR
    55	#     FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
    56	#     BY DIRECTION (in nq the remote arm is destinit; in qn it is srcinit). Since
    57	#     P1's signature is one-directional, that artifact could MANUFACTURE the
    58	#     result. Measured on this rig before fixing: a 10/20/200 ms pre-fsync delay
    59	#     produced NO measurable change in fsync time (72-94 ms, no trend) — APFS
    60	#     fsync here is per-file-metadata bound, not writeback bound — so the fixed
    61	#     settle removes the structural asymmetry without weakening what durability
    62	#     charges.
    63	#   * cold caches BOTH ends every run (purge), then the destination disk is
    64	#     drained to quiet AND RE-CHECKED — the purge itself dirties the disk, so a
    65	#     drain certified BEFORE it proves nothing.
    66	#   * pair-void on: nonzero exit, undrained window, failed purge, fsync mismatch.
    67	#   * same-build gate: clean +EXPECT_SHA, never +sha.dirty; hash failures FATAL.
    68	#   * the HARNESS ITSELF is hashed into the manifest — a modified harness must not
    69	#     be able to claim the reviewed commit.
    70	#
    71	# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
    72	# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
    73	# one ssh), so dispatch is outside the window by construction.
    74	#
    75	# Usage:
    76	#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    77	#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    78	# =============================================================================
    79	set -euo pipefail
    80	
    81	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    82	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    83	SELF="${BASH_SOURCE[0]}"
    84	
    85	HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
    86	HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
    87	EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"
    88	
    89	# --- nagatha: LOCAL end (driver) ---------------------------------------------
    90	N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
    91	N_NIC="${N_NIC:-en11}"
    92	N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
    93	N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
    94	N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
    95	N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
    96	N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
    97	
    98	# --- q: REMOTE end ------------------------------------------------------------
    99	Q_SSH="${Q_SSH:-michael@q}"
   100	Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
   101	Q_NIC="${Q_NIC:-en8}"
   102	Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
   103	Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
   104	Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
   105	Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
   106	Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
   107	
   108	PORT="${PORT:-9031}"
   109	RUNS="${RUNS:-8}"
   110	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
   111	SETTLE_MS="${SETTLE_MS:-250}"     # equal pre-fsync window on BOTH arms
   112	LOAD_MAX="${LOAD_MAX:-3.0}"
   113	DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
   114	DRAIN_MBPS="${DRAIN_MBPS:-2}"
   115	DELTA_REF_MS="${DELTA_REF_MS:-230}"   # rig W's measured Delta_P1 (the reference effect)
   116	
   117	# The REGISTERED cell set. An unregistered or misspelled CELLS must not be able to
   118	# drop every control, or silently measure nothing (codex HIGH).
   119	REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   120	CELLS="${CELLS:-$REGISTERED_CELLS}"
   121	CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   122	VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
   123	
   124	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   125	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
   126	
   127	MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
   128	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   129	         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
   130	qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
   131	
   132	mkdir -p "$OUT_DIR/blit-logs"
   133	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
   134	die() { log "FATAL: $*"; exit 1; }
   135	nocr() { tr -d '\r'; }
   136	want_cell() { [[ ",$CELLS," == *",$1,"* ]]; }
   137	
   138	# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
   139	# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
   140	# falls through to the wrong host (the trap the Linux harness documents).
   141	# `bash -c` locally pins the inner shell so local and remote parse identically
   142	# (q's login shell is not assumed).
   143	hrun() {
   144	  local h="$1"; shift
   145	  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "bash -c $(printf '%q' "$*")"; fi
   146	}
   147	hblit()   { [[ "$1" == n ]] && echo "$N_BLIT"   || echo "$Q_BLIT"; }
   148	hdaemon() { [[ "$1" == n ]] && echo "$N_DAEMON" || echo "$Q_DAEMON"; }
   149	hmod()    { [[ "$1" == n ]] && echo "$N_MODULE" || echo "$Q_MODULE"; }
   150	hip()     { [[ "$1" == n ]] && echo "$N_IP"     || echo "$Q_IP"; }
   151	hnic()    { [[ "$1" == n ]] && echo "$N_NIC"    || echo "$Q_NIC"; }
   152	hmac()    { [[ "$1" == n ]] && echo "$N_MAC"    || echo "$Q_MAC"; }
   153	hname()   { [[ "$1" == n ]] && echo nagatha     || echo q; }
   154	other()   { [[ "$1" == n ]] && echo q           || echo n; }
   155	
   156	# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
   157	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   158	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   159	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   160	
   161	# --- provenance ---------------------------------------------------------------
   162	embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
   163	  local h="$1" p="$2" hit dirty
   164	  hit="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA' '$p' 2>/dev/null || echo X" | nocr)"
   165	  dirty="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA.dirty' '$p' 2>/dev/null || echo X" | nocr)"
   166	  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
   167	  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
   168	}
   169	sha256_of() {      # fail CLOSED on an empty/short hash
   170	  local h="$1" p="$2" v
   171	  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')"
   172	  [[ ${#v} -eq 64 ]] || die "$(hname "$h"): sha256 of $p returned '${v}' (not 64 hex) — refusing"
   173	  echo "$v"
   174	}
   175	
   176	# --- gates: every one fails CLOSED (codex HIGH: they all failed OPEN) ----------
   177	norm_mac() { tr 'A-F' 'a-f' | awk -F: '{for(i=1;i<=NF;i++){printf "%s%02x", (i>1?":":""), strtonum("0x" $i)}; print ""}'; }
   178	
   179	quiescence_gate() {
   180	  local h="$1" out
   181	  out="$(hrun "$h" "pgrep -x codex >/dev/null 2>&1 && echo codex; pgrep -x cargo >/dev/null 2>&1 && echo cargo; pgrep -x rustc >/dev/null 2>&1 && echo rustc; echo __OK__" | nocr)" \
   182	    || die "$(hname "$h"): quiescence probe FAILED — a gate that cannot answer must not answer 'fine'"
   183	  [[ "$out" == *__OK__* ]] || die "$(hname "$h"): quiescence probe returned no sentinel — refusing"
   184	  local busy; busy="$(echo "$out" | grep -v __OK__ | tr '\n' ' ' | xargs || true)"
   185	  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
   186	}
   187	timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   188	  local h="$1" running auto
   189	  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
   190	  [[ "$running" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
   191	  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
   192	  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1; echo" | nocr | tr -cd '0-9')" || auto=""
   193	  [[ "$auto" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
   194	  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
   195	}
   196	spotlight_gate() {
   197	  local h="$1" cpu
   198	  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null | awk '/^mds_stores/{c=\$2} END{printf \"%d\", c+0}'" | nocr | tr -cd '0-9')" || cpu=""
   199	  [[ "$cpu" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot sample Spotlight CPU — refusing"
   200	  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
   201	}
   202	load_gate() {
   203	  local h="$1" l ok
   204	  l="$(hrun "$h" "sysctl -n vm.loadavg" | nocr | awk '{print $2}')" || l=""
   205	  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die "$(hname "$h"): cannot read load1 (got '$l') — refusing"
   206	  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   207	  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
   208	}
   209	load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
   210	
   211	link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
   212	  local h="$1" o peer_ip want got route_nic
   213	  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"
   214	  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
   215	    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
   216	  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk '{print \$4}'" | nocr | norm_mac)"
   217	  [[ -n "$got" && "$got" != "(incomplete)" ]] || die "$(hname "$h"): no ARP entry for $peer_ip"
   218	  [[ "$got" == "$want" ]] \
   219	    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
   220	  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
   221	  [[ "$route_nic" == "$(hnic "$h")" ]] \
   222	    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
   223	}
   224	
   225	preflight() {
   226	  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
   227	  local c
   228	  for c in ${CELLS//,/ }; do
   229	    [[ ",$REGISTERED_CELLS," == *",$c,"* ]] \
   230	      || die "cell '$c' is not in the REGISTERED set ($REGISTERED_CELLS) — a misspelled cell must not silently drop a control or measure nothing"
   231	  done
   232	  local h p w want got wantb gotb
   233	  for h in n q; do
   234	    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
   235	    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
   236	      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
   237	      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
   238	    done
   239	    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
   240	    if hrun "$h" "pgrep -x blit-daemon >/dev/null 2>&1"; then die "$(hname "$h"): a blit-daemon is already running — stop it first"; fi
   241	    for w in large mixed small; do
   242	      want="$(eval echo "\$FIX_COUNT_$w")"; wantb="$(eval echo "\$FIX_BYTES_$w")"
   243	      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
   244	      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
   245	      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
   246	        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
   247	    done
   248	    link_gate "$h"
   249	  done
   250	  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   251	  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
   252	}
   253	
   254	write_manifest() {
   255	  local f="$OUT_DIR/staging-manifest.txt" h
   256	  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
   257	    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
   258	    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
   259	    echo "# cells=$CELLS"
   260	    echo "host,role,sha,sha256,path"
   261	    for h in n q; do
   262	      echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
   263	      echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$(hdaemon "$h")"),$(hdaemon "$h")"
   264	    done; } > "$f"
   265	  log "staging manifest recorded (harness sha256 + 4 binary hashes + every threshold)"
   266	}
   267	
   268	# --- daemons ------------------------------------------------------------------
   269	N_PID=""; Q_PID=""
   270	daemon_start() {
   271	  local h="$1" cfg mod bin pid
   272	  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   273	  hrun "$h" "mkdir -p '$mod'
   274	printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
   275	nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
   276	sleep 2" >/dev/null 2>&1 || true
   277	  pid="$(hrun "$h" "pgrep -x blit-daemon | head -1" | nocr | tr -cd '0-9')"
   278	  [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start (see $(hmod "$h")/mm-daemon.log)"
   279	  [[ "$h" == n ]] && N_PID="$pid" || Q_PID="$pid"
   280	  log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
   281	}
   282	# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
   283	# handshake reached some listener's backlog — not that the daemon speaks blit).
   284	smoke() {
   285	  local h="$1" o probe
   286	  o="$(other "$h")"
   287	  probe="$(hmod "$o")/mm_smoke_${SESSION_TAG}"
   288	  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 || true
   289	  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
   290	    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
   291	    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
   292	  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
   293	  log "smoke ok: $(hname "$h") daemon serves blit"
   294	}
   295	daemon_stop() {
   296	  local h="$1" pid; pid="$([[ "$h" == n ]] && echo "$N_PID" || echo "$Q_PID")"
   297	  [[ -n "$pid" ]] || return 0
   298	  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; echo __DONE__" >/dev/null 2>&1 || true
   299	  # A teardown that cannot be VERIFIED is a failure, not a success (codex MEDIUM).
   300	  if hrun "$h" "ps -p $pid >/dev/null 2>&1 && echo ALIVE || echo GONE" | nocr | grep -q ALIVE; then
   301	    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"
   302	    return 1
   303	  fi
   304	  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
   305	}
   306	cleanup() { daemon_stop n || true; daemon_stop q || true; rm -rf "$MUX" 2>/dev/null || true; }
   307	trap cleanup EXIT
   308	
   309	# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
   310	RUN_DRAIN=""; RUN_COLD=""
   311	drain_host() {
   312	  hrun "$1" "quiet=0
   313	for i in \$(seq 1 $DRAIN_ITERS); do
   314	  w=\$(iostat -d -w 2 -c 2 disk0 2>/dev/null | tail -1 | awk '{print \$3}')
   315	  ok=\$(awk -v w=\"\${w:-99}\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   316	  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   317	  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
   318	done
   319	echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
   320	}
   321	prep_run() {   # $1 = dest host
   322	  local dh="$1" cn=ok cq=ok out
   323	  # Purge BOTH ends first — the purge itself dirties the disk, so a drain
   324	  # certified before it proves nothing (codex HIGH).
   325	  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   326	  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   327	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   328	  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
   329	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
   330	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   331	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   332	}
   333	
   334	# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
   335	RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0
   336	fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes" or "NA 0 0"
   337	  local out
   338	  out="$(hrun "$1" "sleep $(awk -v m=$SETTLE_MS 'BEGIN{printf \"%.3f\", m/1000}')
   339	python3 - '$2' <<'PYEOF'
   340	import os, sys, time
   341	p = sys.argv[1]
   342	if not os.path.isdir(p):
   343	    print('F:NA:0:0:F')          # a MISSING tree must never read as a fast flush
   344	    raise SystemExit
   345	t = time.monotonic()
   346	files = 0
   347	nbytes = 0
   348	for root, _d, fs in os.walk(p):
   349	    for name in fs:
   350	        fp = os.path.join(root, name)
   351	        fd = os.open(fp, os.O_RDONLY)
   352	        os.fsync(fd)
   353	        os.close(fd)
   354	        files += 1
   355	        nbytes += os.fstat(os.open(fp, os.O_RDONLY)).st_size if False else os.path.getsize(fp)
   356	print('F:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes))
   357	PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3/p' | head -1)"
   358	  echo "${out:-NA 0 0}"
   359	}
   360	
   361	# --- one timed run ------------------------------------------------------------
   362	RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
   363	timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
   364	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin r
   365	  bin="$(hblit "$ih")"
   366	  prep_run "$dh"
   367	  out="$(hrun "$ih" "t0=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
   368	'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
   369	t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
   370	echo \"R:\$((t1-t0)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   371	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   372	  read -r RUN_FLUSH RUN_FILES RUN_BYTES <<<"$(fsync_tree "$dh" "$landed")"
   373	  RUN_VALID=yes
   374	  local wc wb; wc="$(eval echo "\$FIX_COUNT_$w")"; wb="$(eval echo "\$FIX_BYTES_$w")"
   375	  if [[ "$RUN_FLUSH" == NA ]]; then
   376	    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
   377	    RUN_VALID=no; RUN_FLUSH=0
   378	  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
   379	    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
   380	    RUN_VALID=no
   381	  fi
   382	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   383	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
   384	}
   385	
   386	# --- arms ---------------------------------------------------------------------
   387	# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
   388	# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
   389	# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
   390	# above is what makes a wrong path fatal instead of silently free.
   391	CUR_W=""; CUR_FLAG=""
   392	arm_srcinit() {
   393	  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
   394	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
   395	            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
   396	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   397	}
   398	arm_destinit() {
   399	  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
   400	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
   401	            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
   402	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   403	}
   404	
   405	CSV="$OUT_DIR/runs.csv"
   406	echo "cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
   407	META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"
   408	
   409	run_pair_loop() {
   410	  local cell="$1" sh="$2" dh="$3"
   411	  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   412	  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   413	  while (( valid < RUNS && attempts < max )); do
   414	    attempts=$(( attempts + 1 ))
   415	    local order pair=yes rowA="" rowB="" arm aname init rid run
   416	    if (( slot % 2 )); then order="A B"; else order="B A"; fi
   417	    for arm in $order; do
   418	      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   419	      rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
   420	      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh" "$run"
   421	      else arm_destinit "$cell" "$rid" "$sh" "$dh" "$run"; fi
   422	      [[ "$RUN_VALID" == yes ]] || pair=no
   423	      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   424	      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
   425	      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   426	    done
   427	    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   428	    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   429	    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
   430	  done
   431	  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
   432	  else echo "$cell,$attempts,yes" >> "$META"; fi
   433	}
   434	
   435	compute_verdicts() {
   436	  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" CONTROL_CELLS="$CONTROL_CELLS" \
   437	  python3 "$SCRIPT_DIR/otp12pf_mac_verdict.py" \
   438	    "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/paired.csv" \
   439	    "$OUT_DIR/verdicts.csv" "$OUT_DIR/session_verdict.txt"
   440	}
   441	
   442	main() {
   443	  preflight
   444	  write_manifest
   445	  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   446	    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   447	    exit 0
   448	  fi
   449	  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
   450	  daemon_start n; daemon_start q
   451	  smoke n; smoke q
   452	
   453	  local carrier w flag cell
   454	  for w in mixed large small; do
   455	    for carrier in tcp grpc; do
   456	      [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
   457	      CUR_W="$w"; CUR_FLAG="$flag"
   458	      cell="nq_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" n q
   459	      cell="qn_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" q n
   460	    done
   461	  done
   462	
   463	  compute_verdicts
   464	  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
   465	  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
   466	  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
   467	  log "=== PAIRED STATS (the rule is graded on these) ==="
   468	  column -t -s, "$OUT_DIR/paired.csv" | tee -a "$OUT_DIR/bench.log"
   469	  log "=== SESSION VERDICT (computed by the harness from the PRE-REGISTERED rule) ==="
   470	  cat "$OUT_DIR/session_verdict.txt" | tee -a "$OUT_DIR/bench.log"
   471	  log "runs: $CSV"
   472	}
   473	main "$@"

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict.py | sed -n '1,320p'; nl -ba scripts/otp12pf_mac_verdict_test.py | sed -n '1,180p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env python3
     2	"""Mechanize the Mac<->Mac pre-registered decision rule.
     3	
     4	docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md is the spec. The harness
     5	must COMPUTE the verdict, not leave it to be applied by hand after the numbers
     6	are visible -- that is what pre-registration exists to prevent (codex BLOCKER 1).
     7	
     8	The noise statistic is a PAIRED inference, not a range. A range (max-min) grows
     9	with n and is dominated by outliers, so a large consistent effect can hide under
    10	it: with srcinit=2000 and d=[0,180,180,190,190,200,200,200] a range rule reports
    11	"VANISHES" despite 7/8 positive pairs and an effect 83% the size of rig W's P1
    12	(codex BLOCKER 2). Instead:
    13	
    14	  d_i  = destinit_i - srcinit_i     (positive = destination-initiated is slower)
    15	  D    = median(d_i)
    16	  CI   = 95% bootstrap CI on the median (seeded => the verdict is deterministic)
    17	  sign = exact two-sided binomial test on the count of positive d_i
    18	
    19	  BAR_BREACH = the effect that would push this cell's ratio to the 1.10 bar
    20	             = 0.10 * srcinit_median
    21	
    22	  REPRODUCES : bar FAILS and CI_lo > 0            (a real, bar-breaking slowdown)
    23	  INVERSION  : bar FAILS and CI_hi < 0            (source-initiated is the slow arm)
    24	  VANISHES   : bar PASSES and |CI| lies strictly inside +/-BAR_BREACH
    25	               -> a genuine EQUIVALENCE result: an effect big enough to matter is
    26	                  EXCLUDED, not merely unobserved.
    27	  PARTIAL    : bar PASSES, CI excludes 0, but the effect is not excluded as small
    28	  UNDERPOWERED: bar PASSES and the CI is too wide to exclude a bar-breaching
    29	               effect -> a null here is INCONCLUSIVE, never "P1 vanishes".
    30	"""
    31	import csv, os, random, sys
    32	from math import comb
    33	
    34	runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]
    35	DELTA_REF = int(os.environ.get("DELTA_REF_MS", "230"))
    36	VERDICT_CELLS = os.environ.get("VERDICT_CELLS", "").split(",")
    37	CONTROL_CELLS = os.environ.get("CONTROL_CELLS", "").split(",")
    38	
    39	rows = list(csv.DictReader(open(runs_p)))
    40	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
    41	
    42	by, slots, void = {}, {}, {}
    43	for r in rows:
    44	    key = (r["cell"], r["arm"])
    45	    if r["valid"] == "yes":
    46	        by.setdefault(key, []).append(int(r["ms"]))
    47	        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
    48	    else:
    49	        void[key] = void.get(key, 0) + 1
    50	
    51	
    52	def med(v):
    53	    """Low median for even n, stated and applied consistently (codex LOW)."""
    54	    v = sorted(v)
    55	    return v[(len(v) - 1) // 2]
    56	
    57	
    58	def complete(c):
    59	    if c not in meta or meta[c]["complete"] != "yes":
    60	        return False
    61	    arms = [a for (cc, a) in by if cc == c]
    62	    return "srcinit" in arms and "destinit" in arms
    63	
    64	
    65	def boot_ci(d, iters=10000, seed=12345):
    66	    """95% bootstrap CI on the median. Seeded: the verdict must be reproducible."""
    67	    rng = random.Random(seed)
    68	    n = len(d)
    69	    meds = sorted(med([d[rng.randrange(n)] for _ in range(n)]) for _ in range(iters))
    70	    return meds[int(0.025 * iters)], meds[int(0.975 * iters) - 1]
    71	
    72	
    73	def sign_p(d):
    74	    """Exact two-sided binomial test on the count of positive differences."""
    75	    nz = [x for x in d if x != 0]
    76	    n = len(nz)
    77	    if n == 0:
    78	        return 1.0, 0, 0
    79	    k = sum(1 for x in nz if x > 0)
    80	    tail = sum(comb(n, i) for i in range(0, min(k, n - k) + 1))
    81	    return min(1.0, 2.0 * tail / (2 ** n)), k, n
    82	
    83	
    84	# ---- summary: every run printed (pf-0's bistability lesson) ------------------
    85	with open(sum_p, "w") as f:
    86	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,runs\n")
    87	    for (c, a) in sorted(by):
    88	        if not complete(c):
    89	            continue
    90	        v = by[(c, a)]
    91	        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
    92	        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s\n" % (
    93	            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
    94	            void.get((c, a), 0), " ".join(str(x) for x in v)))
    95	
    96	# ---- paired stats + per-cell outcome ----------------------------------------
    97	cell_outcome, cell_detail = {}, {}
    98	with open(pair_p, "w") as f:
    99	    f.write("cell,n_pairs,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,"
   100	            "sign_p,k_pos_of_n,bar_breach_ms,delta_ref_ms,powered_for_null,unstable,outcome\n")
   101	    for c in sorted(meta):
   102	        if not complete(c):
   103	            cell_outcome[c] = "INCOMPLETE"
   104	            f.write("%s,,,,,,,,,,,,,,,INCOMPLETE\n" % c)
   105	            continue
   106	        d = [v["destinit"] - v["srcinit"]
   107	             for (cc, _run), v in sorted(slots.items())
   108	             if cc == c and "srcinit" in v and "destinit" in v]
   109	        s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   110	        hi, lo = max(s_med, d_med), min(s_med, d_med)
   111	        bar = "PASS" if 10 * hi <= 11 * lo else "FAIL"      # integer-exact
   112	        D = med(d)
   113	        ci_lo, ci_hi = boot_ci(d)
   114	        p, k, n = sign_p(d)
   115	        breach = 0.10 * s_med                                # effect that reaches 1.10
   116	        powered = (ci_hi - ci_lo) < breach                   # can we exclude a breaching effect?
   117	
   118	        # UNSTABLE, as a STATISTIC not a vibe: an arm splits into two clusters
   119	        # separated by more than the paired spread, AND the bar verdict flips when
   120	        # graded on pooled runs instead of medians.
   121	        unstable = "no"
   122	        for arm in ("srcinit", "destinit"):
   123	            v = sorted(by[(c, arm)])
   124	            gaps = [(v[i + 1] - v[i], i) for i in range(len(v) - 1)]
   125	            gmax, gi = max(gaps) if gaps else (0, 0)
   126	            if gmax > (max(d) - min(d)) and gmax > 0:
   127	                pooled_hi = max(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
   128	                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
   129	                pooled_lo = min(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
   130	                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
   131	                pooled_bar = "PASS" if 10 * pooled_hi <= 11 * pooled_lo else "FAIL"
   132	                if pooled_bar != bar:
   133	                    unstable = "yes"
   134	
   135	        if bar == "FAIL" and ci_lo > 0:
   136	            out = "REPRODUCES"
   137	        elif bar == "FAIL" and ci_hi < 0:
   138	            out = "INVERSION"
   139	        elif bar == "PASS" and ci_lo > -breach and ci_hi < breach:
   140	            out = "VANISHES"
   141	        elif bar == "PASS" and not powered:
   142	            out = "UNDERPOWERED"
   143	        elif bar == "PASS" and (ci_lo > 0 or ci_hi < 0):
   144	            out = "PARTIAL"
   145	        else:
   146	            out = "INCONCLUSIVE"
   147	        if unstable == "yes":
   148	            out = "UNSTABLE"
   149	
   150	        cell_outcome[c] = out
   151	        cell_detail[c] = dict(D=D, ci=(ci_lo, ci_hi), p=p, k=k, n=n, bar=bar,
   152	                              ratio=hi / lo if lo else 0.0, breach=breach)
   153	        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%d/%d,%d,%d,%s,%s,%s\n" % (
   154	            c, len(d), s_med, d_med, (hi / lo if lo else 0.0), bar, D, ci_lo, ci_hi,
   155	            p, k, n, breach, DELTA_REF, "yes" if powered else "no", unstable, out))
   156	
   157	# ---- per-cell invariance rows (unchanged shape) ------------------------------
   158	with open(ver_p, "w") as f:
   159	    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,delta_ms,bar,outcome\n")
   160	    for c in sorted(meta):
   161	        if not complete(c):
   162	            f.write("%s,invariance,srcinit,destinit,,,,,1.10,INCOMPLETE\n" % c)
   163	            continue
   164	        s, dd = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   165	        hi, lo = max(s, dd), min(s, dd)
   166	        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,1.10,%s\n" % (
   167	            c, s, dd, hi / lo if lo else 0.0, dd - s,
   168	            "PASS" if 10 * hi <= 11 * lo else "FAIL"))
   169	
   170	# ---- SESSION VERDICT: the six registered outcomes, in strict precedence ------
   171	lines = []
   172	ctrl = [c for c in CONTROL_CELLS if c in cell_outcome]
   173	verd = [c for c in VERDICT_CELLS if c in cell_outcome]
   174	
   175	ctrl_fail = [c for c in ctrl
   176	             if cell_outcome[c] not in ("VANISHES", "INCONCLUSIVE", "UNDERPOWERED")
   177	             and cell_detail.get(c, {}).get("bar") == "FAIL"]
   178	incomplete = [c for c in (ctrl + verd) if cell_outcome[c] == "INCOMPLETE"]
   179	
   180	if incomplete:
   181	    verdict = "INCOMPLETE"
   182	    why = "cells did not complete: %s" % ", ".join(incomplete)
   183	elif ctrl_fail:
   184	    # 1. RIG-VOID -- a rig whose control fails cannot adjudicate a TCP-only claim.
   185	    verdict = "RIG-VOID"
   186	    why = ("control cell(s) FAILED the 1.10 bar: %s. The rig is not measuring "
   187	           "cleanly; NO verdict may be read." % ", ".join(ctrl_fail))
   188	else:
   189	    outs = {c: cell_outcome[c] for c in verd}
   190	    repro = [c for c, o in outs.items() if o == "REPRODUCES"]
   191	    inv = [c for c, o in outs.items() if o == "INVERSION"]
   192	    unst = [c for c, o in outs.items() if o == "UNSTABLE"]
   193	    van = [c for c, o in outs.items() if o == "VANISHES"]
   194	    part = [c for c, o in outs.items() if o == "PARTIAL"]
   195	    under = [c for c, o in outs.items() if o in ("UNDERPOWERED", "INCONCLUSIVE")]
   196	
   197	    if unst:
   198	        verdict = "UNSTABLE"
   199	        why = ("bimodal arm(s) whose verdict flips on pooled runs: %s. Report as "
   200	               "unstable, NOT resolved." % ", ".join(unst))
   201	    elif repro and inv:
   202	        verdict = "MIXED-SIGN"
   203	        why = ("reproduces in %s but INVERTS in %s -- a host x role interaction "
   204	               "this rig cannot decompose. INCONCLUSIVE for the pairing question."
   205	               % (", ".join(repro), ", ".join(inv)))
   206	    elif repro:
   207	        verdict = "REPRODUCES"
   208	        why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: "
   209	               "it shows P1 CAN occur macOS<->macOS -- it does NOT establish a "
   210	               "platform-general layout cost, and it does NOT kill H1 (H1 accuses "
   211	               "code, and that code runs here too)." % ", ".join(repro))
   212	    elif inv:
   213	        verdict = "INVERSION"
   214	        why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank "
   215	               "this as 'P1 absent'." % ", ".join(inv))
   216	    elif under:
   217	        verdict = "INCONCLUSIVE-UNDERPOWERED"
   218	        why = ("cells cannot exclude a bar-breaching effect: %s. A PASS here is NOT "
   219	               "'P1 vanishes' -- the instrument could not have seen it (pf-0's "
   220	               "error, pre-empted)." % ", ".join(under))
   221	    elif van and len(van) == len(verd):
   222	        verdict = "VANISHES"
   223	        why = ("both TCP-mixed cells EXCLUDE a bar-breaching effect (equivalence). "
   224	               "Scoped to THIS pair: P1 did not reproduce macOS<->macOS. That is "
   225	               "CONSISTENT with 'Windows is required' but does NOT prove it -- it "
   226	               "could be a property of these two machines/disks/OS version.")
   227	    elif part:
   228	        verdict = "PARTIAL"
   229	        why = ("a real but sub-bar asymmetry in: %s. Neither a reproduction nor a "
   230	               "vanish; pf-1 owns it." % ", ".join(part))
   231	    else:
   232	        verdict = "INCONCLUSIVE"
   233	        why = "no registered case matched cleanly; report the cells verbatim."
   234	
   235	lines.append("SESSION VERDICT: %s" % verdict)
   236	lines.append("")
   237	lines.append(why)
   238	lines.append("")
   239	lines.append("Per-cell outcomes (the rule is graded on paired.csv):")
   240	for c in sorted(cell_outcome):
   241	    d = cell_detail.get(c)
   242	    if d:
   243	        lines.append("  %-14s %-12s ratio=%.3f bar=%s  D=%+dms CI=[%+d,%+d] sign_p=%.3f (%d/%d pos)"
   244	                     % (c, cell_outcome[c], d["ratio"], d["bar"], d["D"],
   245	                        d["ci"][0], d["ci"][1], d["p"], d["k"], d["n"]))
   246	    else:
   247	        lines.append("  %-14s %s" % (c, cell_outcome[c]))
   248	lines.append("")
   249	lines.append("This file is COMPUTED from the pre-registered rule. It declares nothing")
   250	lines.append("beyond it, and the owner walks the numbers.")
   251	
   252	open(sess_p, "w").write("\n".join(lines) + "\n")
   253	print("\n".join(lines))
     1	#!/usr/bin/env python3
     2	"""Guard test for otp12pf_mac_verdict.py — run it before trusting a Mac<->Mac run.
     3	
     4	    python3 scripts/otp12pf_mac_verdict_test.py
     5	
     6	The defect it guards (codex round-2 BLOCKER on the harness): the first revision
     7	graded "did the effect vanish?" against S = max(d) - min(d), a RANGE. A range
     8	grows with n and is dominated by outliers, so a large CONSISTENT effect hides
     9	under it:
    10	
    11	    srcinit = 2000 ms;  d = [0,180,180,190,190,200,200,200]
    12	    -> D = 190, S = 200, bar PASSES, |D| <= S  =>  "VANISHES"
    13	
    14	...on 7/8 positive pairs, with an effect 83% the size of rig W's Delta_P1. It
    15	would have reported "P1 requires the Windows peer" off an effect nearly as large
    16	as P1 itself. The rule now uses a bootstrap CI + an equivalence bound against the
    17	bar-breaching effect, and this test pins that.
    18	"""
    19	import csv, os, subprocess, sys, tempfile
    20	
    21	HERE = os.path.dirname(os.path.abspath(__file__))
    22	VERDICT = os.path.join(HERE, "otp12pf_mac_verdict.py")
    23	CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")
    24	VERDICT_CELLS = ("nq_tcp_mixed", "qn_tcp_mixed")
    25	
    26	
    27	def verdict_for(d, src=2000):
    28	    tmp = tempfile.mkdtemp()
    29	    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
    30	    with open(runs, "w") as f:
    31	        w = csv.writer(f)
    32	        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid".split(","))
    33	        for cell in VERDICT_CELLS:
    34	            for i, di in enumerate(d, 1):
    35	                w.writerow([cell, "srcinit", "x", "h", i, src, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
    36	                w.writerow([cell, "destinit", "x", "h", i, src + di, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
    37	        for cell in CONTROLS:            # clean controls, so the rig is not VOID
    38	            for i in range(1, 9):
    39	                w.writerow([cell, "srcinit", "x", "h", i, 1000, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
    40	                w.writerow([cell, "destinit", "x", "h", i, 1005, 0, 1, 1, 0, "drained_1x2s", "cold", "yes"])
    41	    with open(meta, "w") as f:
    42	        f.write("cell,pairs_attempted,complete\n")
    43	        for cell in VERDICT_CELLS + CONTROLS:
    44	            f.write("%s,8,yes\n" % cell)
    45	    env = dict(os.environ, DELTA_REF_MS="230",
    46	               VERDICT_CELLS=",".join(VERDICT_CELLS),
    47	               CONTROL_CELLS=",".join(CONTROLS))
    48	    out = subprocess.run([sys.executable, VERDICT, runs, meta,
    49	                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
    50	                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
    51	                         env=env, capture_output=True, text=True)
    52	    if out.returncode != 0:
    53	        raise SystemExit("verdict engine failed:\n" + out.stderr)
    54	    return out.stdout.splitlines()[0].split(":", 1)[1].strip()
    55	
    56	
    57	CASES = [
    58	    # (name, d, src, must_be, must_not_be)
    59	    ("codex counterexample: real 190ms effect, 7/8 positive",
    60	     [0, 180, 180, 190, 190, 200, 200, 200], 2000, None, "VANISHES"),
    61	    ("a genuinely absent effect",
    62	     [-4, -2, -1, 0, 0, 1, 2, 3], 2000, "VANISHES", None),
    63	    ("a bar-breaking slowdown (destination-initiated)",
    64	     [300, 310, 320, 330, 340, 350, 360, 370], 1000, "REPRODUCES", None),
    65	    ("source-initiated is the slow arm",
    66	     [-300, -310, -320, -330, -340, -350, -360, -370], 1000, "INVERSION", None),
    67	]
    68	
    69	failures = 0
    70	for name, d, src, must_be, must_not_be in CASES:
    71	    got = verdict_for(d, src)
    72	    ok = True
    73	    if must_be and got != must_be:
    74	        ok = False
    75	    if must_not_be and got == must_not_be:
    76	        ok = False
    77	    print("%-52s -> %-26s %s" % (name, got, "ok" if ok else "*** FAIL ***"))
    78	    if not ok:
    79	        failures += 1
    80	        if must_be:
    81	            print("      expected %s" % must_be)
    82	        if must_not_be:
    83	            print("      must NOT be %s (the range-rule bug is back)" % must_not_be)
    84	
    85	print()
    86	print("%d/%d cases passed" % (len(CASES) - failures, len(CASES)))
    87	sys.exit(1 if failures else 0)

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '1,180p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '176,360p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
     4	# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
     5	# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
     6	# =============================================================================
     7	#
     8	# WHY THIS RIG EXISTS
     9	# -------------------
    10	# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
    11	# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
    12	# cell. It answers ONE question, SCOPED TO THIS PAIR:
    13	#
    14	#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
    15	#
    16	#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
    17	#     "platform residue" that can be waived; code-level hypotheses strengthen.
    18	#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
    19	#     "Windows is required", but does NOT prove it: it could equally be a
    20	#     property of these two machines, their disks, or this macOS version.
    21	#
    22	# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
    23	# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
    24	# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
    25	# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
    26	# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
    27	# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
    28	#
    29	# WHAT IT MEASURES
    30	#   cell = <nq|qn>_<carrier>_<fixture>;  nq_* = data nagatha->q, qn_* = q->nagatha
    31	#   arms (the ONLY variable): srcinit (source's CLI pushes) / destinit (dest's CLI
    32	#   pulls). BOTH directions are measured, but a reproduction is NOT required in
    33	#   both — P1's rig-W signature is ONE-DIRECTIONAL (wm FAILS, mw PASSES), so
    34	#   demanding both would rewrite the finding.
    35	#
    36	#   Endpoint asymmetry does NOT cancel: switching the initiator also reassigns
    37	#   which Mac runs the CLI vs the daemon, and q is faster. Both directions are
    38	#   therefore reported separately and no conclusion leans on cancellation.
    39	#
    40	# THE INSTRUMENT IS THE RISK (three claims have been retracted to harness bugs).
    41	# Everything below fails CLOSED. Codex review of the first revision found 11
    42	# defects (3 BLOCKER) in this file before it measured anything; they are fixed
    43	# here and named at their site.
    44	#
    45	#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER THE INITIATOR/VERB, and
    46	#     the fsync walk VERIFIES WHAT IT FLUSHED: it returns the file count and byte
    47	#     sum, and the pair VOIDS unless they match the fixture exactly. (os.walk of a
    48	#     missing/empty path returns 0 files in 0 ms and reads as a FAST SUCCESSFUL
    49	#     FLUSH — the otp-2w bug's exact shape. Verified empirically: a push to
    50	#     /bench/RUNDIR/ lands RUNDIR/src_<W>, a pull into RUNDIR lands files directly
    51	#     in RUNDIR, so the two arms need DIFFERENT landed paths and a wrong one would
    52	#     silently charge an arm nothing.)
    53	#   * A FIXED, EQUAL SETTLE (SETTLE_MS) precedes the fsync on BOTH arms. Between
    54	#     a client exiting and the fsync starting, the OS writes back dirty pages FOR
    55	#     FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
    56	#     BY DIRECTION (in nq the remote arm is destinit; in qn it is srcinit). Since
    57	#     P1's signature is one-directional, that artifact could MANUFACTURE the
    58	#     result. Measured on this rig before fixing: a 10/20/200 ms pre-fsync delay
    59	#     produced NO measurable change in fsync time (72-94 ms, no trend) — APFS
    60	#     fsync here is per-file-metadata bound, not writeback bound — so the fixed
    61	#     settle removes the structural asymmetry without weakening what durability
    62	#     charges.
    63	#   * cold caches BOTH ends every run (purge), then the destination disk is
    64	#     drained to quiet AND RE-CHECKED — the purge itself dirties the disk, so a
    65	#     drain certified BEFORE it proves nothing.
    66	#   * pair-void on: nonzero exit, undrained window, failed purge, fsync mismatch.
    67	#   * same-build gate: clean +EXPECT_SHA, never +sha.dirty; hash failures FATAL.
    68	#   * the HARNESS ITSELF is hashed into the manifest — a modified harness must not
    69	#     be able to claim the reviewed commit.
    70	#
    71	# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
    72	# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
    73	# one ssh), so dispatch is outside the window by construction.
    74	#
    75	# Usage:
    76	#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    77	#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    78	# =============================================================================
    79	set -euo pipefail
    80	
    81	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    82	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    83	SELF="${BASH_SOURCE[0]}"
    84	
    85	HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
    86	HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
    87	EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"
    88	
    89	# --- nagatha: LOCAL end (driver) ---------------------------------------------
    90	N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
    91	N_NIC="${N_NIC:-en11}"
    92	N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
    93	N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
    94	N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
    95	N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
    96	N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
    97	
    98	# --- q: REMOTE end ------------------------------------------------------------
    99	Q_SSH="${Q_SSH:-michael@q}"
   100	Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
   101	Q_NIC="${Q_NIC:-en8}"
   102	Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
   103	Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
   104	Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
   105	Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
   106	Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
   107	
   108	PORT="${PORT:-9031}"
   109	RUNS="${RUNS:-8}"
   110	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
   111	SETTLE_MS="${SETTLE_MS:-250}"     # equal pre-fsync window on BOTH arms
   112	LOAD_MAX="${LOAD_MAX:-3.0}"
   113	DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
   114	DRAIN_MBPS="${DRAIN_MBPS:-2}"
   115	DELTA_REF_MS="${DELTA_REF_MS:-230}"   # rig W's measured Delta_P1 (the reference effect)
   116	
   117	# The REGISTERED cell set. An unregistered or misspelled CELLS must not be able to
   118	# drop every control, or silently measure nothing (codex HIGH).
   119	REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   120	CELLS="${CELLS:-$REGISTERED_CELLS}"
   121	CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   122	VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
   123	
   124	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   125	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
   126	
   127	MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
   128	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   129	         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
   130	qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
   131	
   132	mkdir -p "$OUT_DIR/blit-logs"
   133	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
   134	die() { log "FATAL: $*"; exit 1; }
   135	nocr() { tr -d '\r'; }
   136	want_cell() { [[ ",$CELLS," == *",$1,"* ]]; }
   137	
   138	# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
   139	# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
   140	# falls through to the wrong host (the trap the Linux harness documents).
   141	# `bash -c` locally pins the inner shell so local and remote parse identically
   142	# (q's login shell is not assumed).
   143	hrun() {
   144	  local h="$1"; shift
   145	  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "bash -c $(printf '%q' "$*")"; fi
   146	}
   147	hblit()   { [[ "$1" == n ]] && echo "$N_BLIT"   || echo "$Q_BLIT"; }
   148	hdaemon() { [[ "$1" == n ]] && echo "$N_DAEMON" || echo "$Q_DAEMON"; }
   149	hmod()    { [[ "$1" == n ]] && echo "$N_MODULE" || echo "$Q_MODULE"; }
   150	hip()     { [[ "$1" == n ]] && echo "$N_IP"     || echo "$Q_IP"; }
   151	hnic()    { [[ "$1" == n ]] && echo "$N_NIC"    || echo "$Q_NIC"; }
   152	hmac()    { [[ "$1" == n ]] && echo "$N_MAC"    || echo "$Q_MAC"; }
   153	hname()   { [[ "$1" == n ]] && echo nagatha     || echo q; }
   154	other()   { [[ "$1" == n ]] && echo q           || echo n; }
   155	
   156	# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
   157	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   158	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   159	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   160	
   161	# --- provenance ---------------------------------------------------------------
   162	embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
   163	  local h="$1" p="$2" hit dirty
   164	  hit="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA' '$p' 2>/dev/null || echo X" | nocr)"
   165	  dirty="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA.dirty' '$p' 2>/dev/null || echo X" | nocr)"
   166	  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
   167	  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
   168	}
   169	sha256_of() {      # fail CLOSED on an empty/short hash
   170	  local h="$1" p="$2" v
   171	  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')"
   172	  [[ ${#v} -eq 64 ]] || die "$(hname "$h"): sha256 of $p returned '${v}' (not 64 hex) — refusing"
   173	  echo "$v"
   174	}
   175	
   176	# --- gates: every one fails CLOSED (codex HIGH: they all failed OPEN) ----------
   177	norm_mac() { tr 'A-F' 'a-f' | awk -F: '{for(i=1;i<=NF;i++){printf "%s%02x", (i>1?":":""), strtonum("0x" $i)}; print ""}'; }
   178	
   179	quiescence_gate() {
   180	  local h="$1" out

 succeeded in 0ms:
   176	# --- gates: every one fails CLOSED (codex HIGH: they all failed OPEN) ----------
   177	norm_mac() { tr 'A-F' 'a-f' | awk -F: '{for(i=1;i<=NF;i++){printf "%s%02x", (i>1?":":""), strtonum("0x" $i)}; print ""}'; }
   178	
   179	quiescence_gate() {
   180	  local h="$1" out
   181	  out="$(hrun "$h" "pgrep -x codex >/dev/null 2>&1 && echo codex; pgrep -x cargo >/dev/null 2>&1 && echo cargo; pgrep -x rustc >/dev/null 2>&1 && echo rustc; echo __OK__" | nocr)" \
   182	    || die "$(hname "$h"): quiescence probe FAILED — a gate that cannot answer must not answer 'fine'"
   183	  [[ "$out" == *__OK__* ]] || die "$(hname "$h"): quiescence probe returned no sentinel — refusing"
   184	  local busy; busy="$(echo "$out" | grep -v __OK__ | tr '\n' ' ' | xargs || true)"
   185	  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
   186	}
   187	timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   188	  local h="$1" running auto
   189	  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
   190	  [[ "$running" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
   191	  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
   192	  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1; echo" | nocr | tr -cd '0-9')" || auto=""
   193	  [[ "$auto" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
   194	  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
   195	}
   196	spotlight_gate() {
   197	  local h="$1" cpu
   198	  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null | awk '/^mds_stores/{c=\$2} END{printf \"%d\", c+0}'" | nocr | tr -cd '0-9')" || cpu=""
   199	  [[ "$cpu" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot sample Spotlight CPU — refusing"
   200	  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
   201	}
   202	load_gate() {
   203	  local h="$1" l ok
   204	  l="$(hrun "$h" "sysctl -n vm.loadavg" | nocr | awk '{print $2}')" || l=""
   205	  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die "$(hname "$h"): cannot read load1 (got '$l') — refusing"
   206	  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   207	  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
   208	}
   209	load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
   210	
   211	link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
   212	  local h="$1" o peer_ip want got route_nic
   213	  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"
   214	  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
   215	    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
   216	  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk '{print \$4}'" | nocr | norm_mac)"
   217	  [[ -n "$got" && "$got" != "(incomplete)" ]] || die "$(hname "$h"): no ARP entry for $peer_ip"
   218	  [[ "$got" == "$want" ]] \
   219	    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
   220	  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
   221	  [[ "$route_nic" == "$(hnic "$h")" ]] \
   222	    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
   223	}
   224	
   225	preflight() {
   226	  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
   227	  local c
   228	  for c in ${CELLS//,/ }; do
   229	    [[ ",$REGISTERED_CELLS," == *",$c,"* ]] \
   230	      || die "cell '$c' is not in the REGISTERED set ($REGISTERED_CELLS) — a misspelled cell must not silently drop a control or measure nothing"
   231	  done
   232	  local h p w want got wantb gotb
   233	  for h in n q; do
   234	    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
   235	    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
   236	      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
   237	      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
   238	    done
   239	    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
   240	    if hrun "$h" "pgrep -x blit-daemon >/dev/null 2>&1"; then die "$(hname "$h"): a blit-daemon is already running — stop it first"; fi
   241	    for w in large mixed small; do
   242	      want="$(eval echo "\$FIX_COUNT_$w")"; wantb="$(eval echo "\$FIX_BYTES_$w")"
   243	      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
   244	      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
   245	      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
   246	        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
   247	    done
   248	    link_gate "$h"
   249	  done
   250	  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   251	  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
   252	}
   253	
   254	write_manifest() {
   255	  local f="$OUT_DIR/staging-manifest.txt" h
   256	  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
   257	    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
   258	    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
   259	    echo "# cells=$CELLS"
   260	    echo "host,role,sha,sha256,path"
   261	    for h in n q; do
   262	      echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
   263	      echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$(hdaemon "$h")"),$(hdaemon "$h")"
   264	    done; } > "$f"
   265	  log "staging manifest recorded (harness sha256 + 4 binary hashes + every threshold)"
   266	}
   267	
   268	# --- daemons ------------------------------------------------------------------
   269	N_PID=""; Q_PID=""
   270	daemon_start() {
   271	  local h="$1" cfg mod bin pid
   272	  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   273	  hrun "$h" "mkdir -p '$mod'
   274	printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
   275	nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
   276	sleep 2" >/dev/null 2>&1 || true
   277	  pid="$(hrun "$h" "pgrep -x blit-daemon | head -1" | nocr | tr -cd '0-9')"
   278	  [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start (see $(hmod "$h")/mm-daemon.log)"
   279	  [[ "$h" == n ]] && N_PID="$pid" || Q_PID="$pid"
   280	  log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
   281	}
   282	# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
   283	# handshake reached some listener's backlog — not that the daemon speaks blit).
   284	smoke() {
   285	  local h="$1" o probe
   286	  o="$(other "$h")"
   287	  probe="$(hmod "$o")/mm_smoke_${SESSION_TAG}"
   288	  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 || true
   289	  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
   290	    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
   291	    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
   292	  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
   293	  log "smoke ok: $(hname "$h") daemon serves blit"
   294	}
   295	daemon_stop() {
   296	  local h="$1" pid; pid="$([[ "$h" == n ]] && echo "$N_PID" || echo "$Q_PID")"
   297	  [[ -n "$pid" ]] || return 0
   298	  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; echo __DONE__" >/dev/null 2>&1 || true
   299	  # A teardown that cannot be VERIFIED is a failure, not a success (codex MEDIUM).
   300	  if hrun "$h" "ps -p $pid >/dev/null 2>&1 && echo ALIVE || echo GONE" | nocr | grep -q ALIVE; then
   301	    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"
   302	    return 1
   303	  fi
   304	  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
   305	}
   306	cleanup() { daemon_stop n || true; daemon_stop q || true; rm -rf "$MUX" 2>/dev/null || true; }
   307	trap cleanup EXIT
   308	
   309	# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
   310	RUN_DRAIN=""; RUN_COLD=""
   311	drain_host() {
   312	  hrun "$1" "quiet=0
   313	for i in \$(seq 1 $DRAIN_ITERS); do
   314	  w=\$(iostat -d -w 2 -c 2 disk0 2>/dev/null | tail -1 | awk '{print \$3}')
   315	  ok=\$(awk -v w=\"\${w:-99}\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   316	  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   317	  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
   318	done
   319	echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
   320	}
   321	prep_run() {   # $1 = dest host
   322	  local dh="$1" cn=ok cq=ok out
   323	  # Purge BOTH ends first — the purge itself dirties the disk, so a drain
   324	  # certified before it proves nothing (codex HIGH).
   325	  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   326	  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   327	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   328	  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
   329	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
   330	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   331	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   332	}
   333	
   334	# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
   335	RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0
   336	fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes" or "NA 0 0"
   337	  local out
   338	  out="$(hrun "$1" "sleep $(awk -v m=$SETTLE_MS 'BEGIN{printf \"%.3f\", m/1000}')
   339	python3 - '$2' <<'PYEOF'
   340	import os, sys, time
   341	p = sys.argv[1]
   342	if not os.path.isdir(p):
   343	    print('F:NA:0:0:F')          # a MISSING tree must never read as a fast flush
   344	    raise SystemExit
   345	t = time.monotonic()
   346	files = 0
   347	nbytes = 0
   348	for root, _d, fs in os.walk(p):
   349	    for name in fs:
   350	        fp = os.path.join(root, name)
   351	        fd = os.open(fp, os.O_RDONLY)
   352	        os.fsync(fd)
   353	        os.close(fd)
   354	        files += 1
   355	        nbytes += os.fstat(os.open(fp, os.O_RDONLY)).st_size if False else os.path.getsize(fp)
   356	print('F:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes))
   357	PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3/p' | head -1)"
   358	  echo "${out:-NA 0 0}"
   359	}
   360	

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '361,520p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   361	# --- one timed run ------------------------------------------------------------
   362	RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
   363	timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
   364	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin r
   365	  bin="$(hblit "$ih")"
   366	  prep_run "$dh"
   367	  out="$(hrun "$ih" "t0=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
   368	'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
   369	t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
   370	echo \"R:\$((t1-t0)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   371	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   372	  read -r RUN_FLUSH RUN_FILES RUN_BYTES <<<"$(fsync_tree "$dh" "$landed")"
   373	  RUN_VALID=yes
   374	  local wc wb; wc="$(eval echo "\$FIX_COUNT_$w")"; wb="$(eval echo "\$FIX_BYTES_$w")"
   375	  if [[ "$RUN_FLUSH" == NA ]]; then
   376	    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
   377	    RUN_VALID=no; RUN_FLUSH=0
   378	  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
   379	    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
   380	    RUN_VALID=no
   381	  fi
   382	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   383	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
   384	}
   385	
   386	# --- arms ---------------------------------------------------------------------
   387	# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
   388	# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
   389	# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
   390	# above is what makes a wrong path fatal instead of silently free.
   391	CUR_W=""; CUR_FLAG=""
   392	arm_srcinit() {
   393	  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
   394	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
   395	            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
   396	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   397	}
   398	arm_destinit() {
   399	  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
   400	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
   401	            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
   402	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   403	}
   404	
   405	CSV="$OUT_DIR/runs.csv"
   406	echo "cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
   407	META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"
   408	
   409	run_pair_loop() {
   410	  local cell="$1" sh="$2" dh="$3"
   411	  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   412	  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   413	  while (( valid < RUNS && attempts < max )); do
   414	    attempts=$(( attempts + 1 ))
   415	    local order pair=yes rowA="" rowB="" arm aname init rid run
   416	    if (( slot % 2 )); then order="A B"; else order="B A"; fi
   417	    for arm in $order; do
   418	      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   419	      rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
   420	      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh" "$run"
   421	      else arm_destinit "$cell" "$rid" "$sh" "$dh" "$run"; fi
   422	      [[ "$RUN_VALID" == yes ]] || pair=no
   423	      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   424	      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
   425	      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   426	    done
   427	    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   428	    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   429	    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
   430	  done
   431	  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
   432	  else echo "$cell,$attempts,yes" >> "$META"; fi
   433	}
   434	
   435	compute_verdicts() {
   436	  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" CONTROL_CELLS="$CONTROL_CELLS" \
   437	  python3 "$SCRIPT_DIR/otp12pf_mac_verdict.py" \
   438	    "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/paired.csv" \
   439	    "$OUT_DIR/verdicts.csv" "$OUT_DIR/session_verdict.txt"
   440	}
   441	
   442	main() {
   443	  preflight
   444	  write_manifest
   445	  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   446	    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   447	    exit 0
   448	  fi
   449	  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
   450	  daemon_start n; daemon_start q
   451	  smoke n; smoke q
   452	
   453	  local carrier w flag cell
   454	  for w in mixed large small; do
   455	    for carrier in tcp grpc; do
   456	      [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
   457	      CUR_W="$w"; CUR_FLAG="$flag"
   458	      cell="nq_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" n q
   459	      cell="qn_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" q n
   460	    done
   461	  done
   462	
   463	  compute_verdicts
   464	  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
   465	  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
   466	  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
   467	  log "=== PAIRED STATS (the rule is graded on these) ==="
   468	  column -t -s, "$OUT_DIR/paired.csv" | tee -a "$OUT_DIR/bench.log"
   469	  log "=== SESSION VERDICT (computed by the harness from the PRE-REGISTERED rule) ==="
   470	  cat "$OUT_DIR/session_verdict.txt" | tee -a "$OUT_DIR/bench.log"
   471	  log "runs: $CSV"
   472	}
   473	main "$@"

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md | sed -n '120,280p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   120	The median convention is the **low median** for even n, stated once and applied
   121	everywhere (round-2 LOW).
   122	
   123	## POWER GATE — a null must be an EQUIVALENCE result, not an absence of evidence
   124	
   125	pf-0 reported a KILL with an instrument that could not resolve the effect it
   126	killed. This design pre-empts that:
   127	
   128	- A **null is only reportable** if the CI **excludes a bar-breaching effect** —
   129	  i.e. the whole CI lies strictly inside ±`BAR_BREACH`. That is a genuine
   130	  *equivalence* claim: "an effect big enough to matter is ruled out."
   131	- If the CI is **too wide** to exclude it, the cell is **UNDERPOWERED** and the
   132	  session verdict is **INCONCLUSIVE-UNDERPOWERED**. A PASS is then *not*
   133	  "P1 vanishes" — it is "this rig could not have seen it".
   134	- A **reproduction** needs no such gate: an effect that is seen is seen.
   135	
   136	## Decision rule — computed BY THE HARNESS, exhaustive, in strict precedence
   137	
   138	The harness emits `session_verdict.txt`. **The verdict is not applied by hand
   139	after the numbers are visible** (round-2 BLOCKER: rev 2's harness computed only
   140	PASS/FAIL, which would have left the rule to me, post-hoc).
   141	
   142	Per cell (integer-exact bar `10·hi ≤ 11·lo`, never the printed ratio):
   143	
   144	| cell outcome | condition |
   145	|---|---|
   146	| **REPRODUCES** | bar **FAILS** and `CI_lo > 0` |
   147	| **INVERSION** | bar **FAILS** and `CI_hi < 0` |
   148	| **VANISHES** | bar **PASSES** and the CI lies strictly inside ±`BAR_BREACH` |
   149	| **UNDERPOWERED** | bar **PASSES** and the CI cannot exclude `BAR_BREACH` |
   150	| **PARTIAL** | bar **PASSES**, CI excludes 0, effect not excluded as small |
   151	| **UNSTABLE** | (override) an arm is bimodal *and* the bar verdict flips on pooled runs |
   152	
   153	Session precedence (first match wins; every cell's own outcome is still recorded):
   154	
   155	1. **INCOMPLETE** — any cell short of its pairs.
   156	2. **RIG-VOID** — any **control** cell FAILS the bar. A rig whose gRPC/large
   157	   control fails cannot adjudicate a TCP-only claim. No verdict is read.
   158	3. **UNSTABLE** — a bimodal arm whose verdict flips. Reported as unstable, not
   159	   resolved.
   160	4. **MIXED-SIGN** — one direction REPRODUCES and the other INVERTS: a host×role
   161	   interaction this rig **cannot decompose**. Inconclusive for the question.
   162	5. **REPRODUCES** — either direction. → *P1 can occur without a Windows peer, on
   163	   this pair.*
   164	6. **INVERSION** — a new finding; never banked as "P1 absent".
   165	7. **INCONCLUSIVE-UNDERPOWERED** — the null branch is unavailable.
   166	8. **VANISHES** — both TCP×mixed cells exclude a bar-breaching effect.
   167	9. **PARTIAL** — a real but sub-bar asymmetry; pf-1 owns it.
   168	
   169	**No outcome may be reported that is not one of these.**
   170	
   171	**Bistability is a STATISTIC, not a vibe.** pf-0 found the rig-W fast arm bimodal,
   172	where the mode *mixture* moved a median 72 ms at constant conditions. Here: an arm
   173	whose runs split into two clusters separated by more than the paired spread, **and**
   174	whose bar verdict flips when graded on pooled runs rather than medians, is
   175	**UNSTABLE**. All 8 runs of every arm are printed in `summary.csv`, so this is
   176	checkable rather than asserted.
   177	
   178	## The instrument — two defects that could have MANUFACTURED the result (round-2 HIGH)
   179	
   180	**1. The durability check was fail-open.** `os.walk()` on a missing, unreadable or
   181	empty path returns **0 files in 0 ms** — a missing tree reads as a *fast,
   182	successful flush*. The two arms need **different** landed paths, because blit uses
   183	rsync-style slash semantics (verified empirically: a push to `/bench/RUNDIR/` lands
   184	the tree at `RUNDIR/src_<W>`; a pull into `RUNDIR` lands the files **directly in**
   185	`RUNDIR`). A wrong path would have charged one arm **zero** durability while the
   186	other paid full — the otp-2w bug that once manufactured P1.
   187	**Fixed**: the fsync walk returns its **file count and byte sum**, and the pair
   188	**VOIDs** unless both match the fixture exactly. An exit-0 zero-byte or partial
   189	transfer can no longer become a valid *fast* row.
   190	
   191	**2. The free-writeback gap REVERSED SIGN WITH DIRECTION.** Between a client
   192	exiting and the fsync starting, the OS writes back dirty pages **for free**, and
   193	that gap is longer for whichever arm ran over ssh:
   194	
   195	    cell nq (dest = q):        srcinit = LOCAL client,  destinit = REMOTE client
   196	    cell qn (dest = nagatha):  srcinit = REMOTE client, destinit = LOCAL client
   197	
   198	So the *favoured arm flips with the data direction*. Since P1's signature is
   199	**one-directional**, this artifact is capable of producing a one-directional
   200	"reproduction" **out of nothing**.
   201	**Measured before fixing** (the instrument is verified, not assumed): a pre-fsync
   202	delay of **10 / 20 / 200 ms produced no measurable change in fsync time**
   203	(72–94 ms, no trend) — APFS fsync here is per-file-metadata bound, not writeback
   204	bound. **Fixed anyway, structurally**: a fixed, equal `SETTLE_MS` (250 ms) precedes
   205	the fsync on **both** arms, so the asymmetry is removed by construction without
   206	weakening what durability charges.
   207	
   208	## Gates — fail-closed (round-1 HIGH: revision 1 only *warned*; round-2 HIGH: they all failed OPEN)
   209	
   210	A run that misses any of these is **VOID**, not "close enough". **Every gate fails
   211	CLOSED**: a gate that cannot answer must never answer "fine" (round 2 found
   212	`pgrep` errors reading as "quiet", a `tmutil` read error reading as "disabled",
   213	`top` failures reading as 0% — the same class as pf-0's `ps` decaying average that
   214	reported a *finished* backup as 255%).
   215	
   216	- **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
   217	  **either** Mac (both are bench **ends** here — nagatha is no longer just the
   218	  driver). *(Already proven live: this gate fired on its first invocation and
   219	  refused to start while the codex review of revision 1 was running.)*
   220	- **TIME MACHINE, BOTH MACS — FAIL-CLOSED, not a warning.** Refuse to start if a
   221	  backup is running **or if autobackup is merely ENABLED**, because macOS repeats
   222	  hourly and a backup can begin *inside* the window (pf-0's did, 1 minute before
   223	  the run; one destination is a network share on `skippy` — the same 10 GbE
   224	  fabric). Revision 1 downgraded this to a warning; that is exactly the hole pf-0
   225	  exposed.
   226	- **SPOTLIGHT, BOTH MACS.** `mds_stores` is a recorded contaminant
   227	  (`.agents/machines.md`). Refuse to start while it is actively indexing.
   228	- **LOAD THRESHOLD.** `load1` recorded on both Macs at start **and end**; a start
   229	  `load1` above **3.0** on either end VOIDs the session (the Macs idle at ~1.5–2).
   230	- **Cold caches both ends every run** via `sudo -n /usr/sbin/purge` (NOPASSWD
   231	  granted on both); a failed purge **VOIDS the pair** — a warm row is worse than
   232	  no row.
   233	- **Destination-keyed durability, never verb-keyed**: the macOS per-file `fsync`
   234	  walk runs **on the destination host for both arms**, is **timed**, and a failed
   235	  walk returns `NA` → the pair **VOIDS** (it must never read as a plausible flush).
   236	  (The otp-2w rule: a sync inside the initiator's bracket charges the pull arm for
   237	  writeback the push arm gets free and *manufactures* invariance failures — the
   238	  gRPC control is what exposed it.)
   239	- **Drain**: destination disk quiet before each timed window — macOS `iostat`,
   240	  `< 2 MB/s` for 3 consecutive 2 s samples; `DRAIN-TIMEOUT` VOIDs the pair.
   241	- **Fixtures verified by count on both ends** (`large` 1, `mixed` 5001,
   242	  `small` 10000) before any timed run — the arms must read the same trees.
   243	- **Provenance**: clean `+f35702a` on all four binaries; sha256 staging manifest.
   244	- **Link validity, measured not assumed**: peer ARP resolves to the **peer's** MAC
   245	  (a host route on a directly-connected subnet installs a black hole that still
   246	  reports the right interface); an ssh throughput test **cannot** validate this
   247	  link (~79 MB/s either way regardless) and is not used.
   248	
   249	## What this does NOT establish
   250	
   251	- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
   252	- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
   253	  pair is staged on the Macs. P2 on the Linux rig is a separate, outstanding
   254	  experiment.
   255	- **A null here cannot retract rig W.** P1 is measured and reproduced four times
   256	  there; this rig can only speak about the *pairing*.
   257	- **It cannot decompose host×role.** nagatha and `q` differ; outcome 6 exists
   258	  precisely because that case is beyond this rig.
   259	
   260	## The runs
   261	
   262	```sh
   263	EXPECT_SHA=f35702a RUNS=8 \
   264	  CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large \
   265	  bash scripts/bench_otp12pf_mac.sh
   266	```
   267	
   268	Harness: `scripts/bench_otp12pf_mac.sh`. It **computes; it declares nothing** —
   269	the verdict is read off the rule above.

2026-07-14T08:33:43.919581Z ERROR codex_models_manager::manager: failed to refresh available models: timeout waiting for child process to exit
exec
/bin/zsh -lc "python3 -c 'from math import comb
for j in range(1,9):
 p=j/8; cdf=sum(comb(8,k)*p**k*(1-p)**(8-k) for k in range(4,9)); print(j, f\"P(boot low-med <= x{j})={cdf:.9f}\")
print(\"coverage [X2,X7] population median:\",1-2*sum(comb(8,k) for k in range(0,2))/2**8)
print(\"coverage [X1,X8] population median:\",1-2*sum(comb(8,k) for k in range(0,1))/2**8)' && python3 -c 'import random
def med(v): return sorted(v)[(len(v)-1)//2]
def boot(d):
 r=random.Random(12345); n=len(d); m=sorted(med([d[r.randrange(n)] for _ in range(n)]) for _ in range(10000)); return m[250],m[9749],{x:m.count(x) for x in sorted(set(m))}
for d in ([0,180,180,190,190,200,200,200],[230]*8,[0]*7+[230],[0]*6+[230]*2,list(range(8))): print(d,boot(list(d)))'" in /Users/michael/Dev/blit_v2
 succeeded in 131ms:
[0, 180, 180, 190, 190, 200, 200, 200] (180, 200, {0: 114, 180: 3418, 190: 5101, 200: 1367})
[230, 230, 230, 230, 230, 230, 230, 230] (230, 230, {230: 10000})
[0, 0, 0, 0, 0, 0, 0, 230] (0, 0, {0: 9992, 230: 8})
[0, 0, 0, 0, 0, 0, 230, 230] (0, 230, {0: 9746, 230: 254})
[0, 1, 2, 3, 4, 5, 6, 7] (1, 6, {0: 114, 1: 1094, 2: 2324, 3: 2843, 4: 2258, 5: 1113, 6: 246, 7: 8})

exec
/bin/zsh -lc "command -v awk; awk 'BEGIN{print strtonum(\"0x10\")}' </dev/null; printf 'clean-marker-check: '; bash -c 'v=\""'$(grep -c -- definitely_absent /dev/null 2>/dev/null || echo X)"; printf "<%s> regex=%s'"\\n\" \""'$v" "$([[ "$v" =~ ''^[0-9]+$ ]] && echo yes || echo no)"'"'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/usr/bin/awk
awk: calling undefined function strtonum
 source line number 1
clean-marker-check: <0
X> regex=no

exec
/bin/zsh -lc 'git diff 24660ae''^ 24660ae -- scripts/bench_otp12pf_mac.sh | sed -n '"'1,700p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-C4IpV6lg' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-aEYGrIyo' (errno=Operation not permitted)
diff --git a/scripts/bench_otp12pf_mac.sh b/scripts/bench_otp12pf_mac.sh
index b6cdcd6..d24907c 100755
--- a/scripts/bench_otp12pf_mac.sh
+++ b/scripts/bench_otp12pf_mac.sh
@@ -8,93 +8,89 @@
 # WHY THIS RIG EXISTS
 # -------------------
 # P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
-# on macOS<->Windows. Linux<->Linux shows NO P1 (8/8 PASS). macOS<->macOS is the
-# untested cell of the 2x2. It answers ONE question:
+# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
+# cell. It answers ONE question, SCOPED TO THIS PAIR:
 #
-#     Does P1 REQUIRE the macOS<->Windows PAIRING, or is it a platform-general
-#     cost of the destination-initiated layout?
+#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
 #
-#   * reproduces -> P1 needs no Windows peer: it is NOT platform residue, the
-#     "accept it as platform residue" escape closes, and every code-level
-#     hypothesis strengthens;
-#   * vanishes   -> P1 is pairing-dependent: platform-agnostic code mechanisms
-#     weaken and a Windows-specific cost (or an interaction) rises.
+#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
+#     "platform residue" that can be waived; code-level hypotheses strengthen.
+#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
+#     "Windows is required", but does NOT prove it: it could equally be a
+#     property of these two machines, their disks, or this macOS version.
 #
 # ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
-# Revision 1 of this script and of docs/STATE.md claimed "reproduces => H1 DIES,
-# because H1 accuses the Windows accept branch". That is FALSE and is retracted:
 # H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
-# InitiatorReceivePlaneRun.add_dialed_stream, the synchronous dial-before-ACK at
+# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
 # transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
-# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with
-# H1, not fatal to it. (The parent plan itself warns: "'consistent with H1' is
-# not confirmation.") Caught by codex review of the pre-registration, BEFORE any
-# rig time was spent.
+# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
+# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
 #
 # WHAT IT MEASURES
-#   cell = <nq|qn>_<carrier>_<fixture>
-#     nq_* : data nagatha -> q        qn_* : data q -> nagatha
-#   arms per cell (the ONLY variable):
-#     srcinit  : the SOURCE host's CLI pushes      (source-initiated)
-#     destinit : the DEST   host's CLI pulls       (destination-initiated)
-#   BOTH data directions are measured, but a reproduction is NOT required in
-#   both: P1's recorded signature on rig W is ONE-DIRECTIONAL (wm_tcp_mixed FAILS
-#   while mw_tcp_mixed PASSES), so demanding failure in both would rewrite the
-#   finding. A reproduction in EITHER direction demonstrates the layout cost
-#   without a Windows peer.
+#   cell = <nq|qn>_<carrier>_<fixture>;  nq_* = data nagatha->q, qn_* = q->nagatha
+#   arms (the ONLY variable): srcinit (source's CLI pushes) / destinit (dest's CLI
+#   pulls). BOTH directions are measured, but a reproduction is NOT required in
+#   both — P1's rig-W signature is ONE-DIRECTIONAL (wm FAILS, mw PASSES), so
+#   demanding both would rewrite the finding.
 #
-#   Endpoint asymmetry does NOT simply cancel: switching the initiator also
-#   reassigns which Mac runs the CLI and which runs the daemon, and q is the
-#   faster machine. Only arm-independent costs cancel; host x role interactions
-#   do not. Hence both directions are reported SEPARATELY and no conclusion may
-#   lean on perfect cancellation.
+#   Endpoint asymmetry does NOT cancel: switching the initiator also reassigns
+#   which Mac runs the CLI vs the daemon, and q is faster. Both directions are
+#   therefore reported separately and no conclusion leans on cancellation.
 #
-# VERDICT: invariance bar, max(srcinit,destinit)/min <= 1.10, integer-exact
-# (10*hi <= 11*lo). This script COMPUTES; it DECLARES nothing.
+# THE INSTRUMENT IS THE RISK (three claims have been retracted to harness bugs).
+# Everything below fails CLOSED. Codex review of the first revision found 11
+# defects (3 BLOCKER) in this file before it measured anything; they are fixed
+# here and named at their site.
 #
-# METHODOLOGY (otp-12 shape + the two gates pf-0 proved were missing)
-#   * QUIESCENCE gate on BOTH Macs (codex/cargo/rustc) — here nagatha is a bench
-#     END, not merely the driver; load on either end contaminates ASYMMETRICALLY.
-#   * TIME MACHINE gate on BOTH Macs — the hole pf-0 found: the old quiet-gate
-#     watched only codex/cargo/rustc and would have sailed straight through the
-#     backup that fired 1 minute before pf-0's run (hourly cadence; one
-#     destination is a network share on skippy = the same 10GbE fabric).
-#   * cold caches BOTH ends every run via `sudo -n /usr/sbin/purge` (a failed
-#     purge VOIDS the pair — a warm row is worse than no row);
-#   * destination disk drained to quiet (iostat) before each timed window;
-#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER BY THE INITIATOR/VERB:
-#     the macOS per-file fsync walk runs on the destination for BOTH arms. (The
-#     otp-2w rule, re-learned the hard way: a sync inside the initiator's bracket
-#     charges the pull arm for writeback the push arm gets free and MANUFACTURES
-#     invariance failures — including on the gRPC control that must stay clean.)
-#   * fresh never-seen destination per run; ABBA counterbalance; pair-void with a
-#     2*RUNS cap then INCOMPLETE; nonzero exit or undrained window voids the pair;
-#   * same-build gate: every binary embeds a CLEAN +EXPECT_SHA (never +sha.dirty).
+#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER THE INITIATOR/VERB, and
+#     the fsync walk VERIFIES WHAT IT FLUSHED: it returns the file count and byte
+#     sum, and the pair VOIDS unless they match the fixture exactly. (os.walk of a
+#     missing/empty path returns 0 files in 0 ms and reads as a FAST SUCCESSFUL
+#     FLUSH — the otp-2w bug's exact shape. Verified empirically: a push to
+#     /bench/RUNDIR/ lands RUNDIR/src_<W>, a pull into RUNDIR lands files directly
+#     in RUNDIR, so the two arms need DIFFERENT landed paths and a wrong one would
+#     silently charge an arm nothing.)
+#   * A FIXED, EQUAL SETTLE (SETTLE_MS) precedes the fsync on BOTH arms. Between
+#     a client exiting and the fsync starting, the OS writes back dirty pages FOR
+#     FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
+#     BY DIRECTION (in nq the remote arm is destinit; in qn it is srcinit). Since
+#     P1's signature is one-directional, that artifact could MANUFACTURE the
+#     result. Measured on this rig before fixing: a 10/20/200 ms pre-fsync delay
+#     produced NO measurable change in fsync time (72-94 ms, no trend) — APFS
+#     fsync here is per-file-metadata bound, not writeback bound — so the fixed
+#     settle removes the structural asymmetry without weakening what durability
+#     charges.
+#   * cold caches BOTH ends every run (purge), then the destination disk is
+#     drained to quiet AND RE-CHECKED — the purge itself dirties the disk, so a
+#     drain certified BEFORE it proves nothing.
+#   * pair-void on: nonzero exit, undrained window, failed purge, fsync mismatch.
+#   * same-build gate: clean +EXPECT_SHA, never +sha.dirty; hash failures FATAL.
+#   * the HARNESS ITSELF is hashed into the manifest — a modified harness must not
+#     be able to claim the reviewed commit.
 #
-# TOPOLOGY NOTE (why one end is local): the driver runs on nagatha, so the nagatha
-# end is LOCAL and the q end is over ssh. This is the proven rig-W shape: each
-# timed window is self-timed ON the initiating host — locally for nagatha, and
-# INSIDE a single ssh for q — so the ssh round trip is outside the window by
-# construction and neither arm is charged for dispatch. The driver is blocked
-# waiting during every timed window, so its own load is idle and identical across
-# arms.
+# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
+# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
+# one ssh), so dispatch is outside the window by construction.
 #
 # Usage:
-#   EXPECT_SHA=f35702a RUNS=8 bash scripts/bench_otp12pf_mac.sh
+#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
 #   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
-#   CELLS=nq_tcp_mixed,qn_tcp_mixed RUNS=8 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
 # =============================================================================
 set -euo pipefail
 
 SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
 REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
+SELF="${BASH_SOURCE[0]}"
 
 HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
-EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a) — the binaries are gated on it}"
+HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
+EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"
 
-# --- nagatha: LOCAL end (driver runs here) -----------------------------------
+# --- nagatha: LOCAL end (driver) ---------------------------------------------
 N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
-N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"    # the pinned clone
+N_NIC="${N_NIC:-en11}"
+N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
+N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
 N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
 N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
 N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
@@ -102,6 +98,8 @@ N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
 # --- q: REMOTE end ------------------------------------------------------------
 Q_SSH="${Q_SSH:-michael@q}"
 Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
+Q_NIC="${Q_NIC:-en8}"
+Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
 Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
 Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
 Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
@@ -110,14 +108,23 @@ Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
 PORT="${PORT:-9031}"
 RUNS="${RUNS:-8}"
 PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
-CELLS="${CELLS:-}"
+SETTLE_MS="${SETTLE_MS:-250}"     # equal pre-fsync window on BOTH arms
+LOAD_MAX="${LOAD_MAX:-3.0}"
+DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
+DRAIN_MBPS="${DRAIN_MBPS:-2}"
+DELTA_REF_MS="${DELTA_REF_MS:-230}"   # rig W's measured Delta_P1 (the reference effect)
+
+# The REGISTERED cell set. An unregistered or misspelled CELLS must not be able to
+# drop every control, or silently measure nothing (codex HIGH).
+REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
+CELLS="${CELLS:-$REGISTERED_CELLS}"
+CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
+VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
+
 SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
 OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
-DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
-DRAIN_MBPS="${DRAIN_MBPS:-2}"     # dest disk considered quiet below this MB/s
 
-# /tmp, not $TMPDIR: macOS TMPDIR busts ssh's 104-byte ControlPath cap (otp-12c).
-MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"
+MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
 SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
          -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
 qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
@@ -126,271 +133,296 @@ mkdir -p "$OUT_DIR/blit-logs"
 log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
 die() { log "FATAL: $*"; exit 1; }
 nocr() { tr -d '\r'; }
-want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }
+want_cell() { [[ ",$CELLS," == *",$1,"* ]]; }
 
-# --- host abstraction: $1 = n (local nagatha) | q (remote) --------------------
+# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
 # if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
-# falls through to the wrong host (the exact trap the Linux harness documents).
+# falls through to the wrong host (the trap the Linux harness documents).
+# `bash -c` locally pins the inner shell so local and remote parse identically
+# (q's login shell is not assumed).
 hrun() {
   local h="$1"; shift
-  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "$*"; fi
+  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "bash -c $(printf '%q' "$*")"; fi
 }
 hblit()   { [[ "$1" == n ]] && echo "$N_BLIT"   || echo "$Q_BLIT"; }
 hdaemon() { [[ "$1" == n ]] && echo "$N_DAEMON" || echo "$Q_DAEMON"; }
 hmod()    { [[ "$1" == n ]] && echo "$N_MODULE" || echo "$Q_MODULE"; }
 hip()     { [[ "$1" == n ]] && echo "$N_IP"     || echo "$Q_IP"; }
+hnic()    { [[ "$1" == n ]] && echo "$N_NIC"    || echo "$Q_NIC"; }
+hmac()    { [[ "$1" == n ]] && echo "$N_MAC"    || echo "$Q_MAC"; }
 hname()   { [[ "$1" == n ]] && echo nagatha     || echo q; }
+other()   { [[ "$1" == n ]] && echo q           || echo n; }
 
-# --- fixtures (otp-2 shapes; verified by count, never trusted) ----------------
-FIX_COUNT_large=1;     FIX_COUNT_small=10000;  FIX_COUNT_mixed=5001
+# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
+FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
+FIX_COUNT_small=10000; FIX_BYTES_small=40960000
+FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
 
-# --- provenance: embed +sha AND reject +sha.dirty -----------------------------
-embeds_clean() {   # $1=host $2=path
-  hrun "$1" "grep -qa -- '+$EXPECT_SHA' '$2' && ! grep -qa -- '+$EXPECT_SHA.dirty' '$2'"
+# --- provenance ---------------------------------------------------------------
+embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
+  local h="$1" p="$2" hit dirty
+  hit="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA' '$p' 2>/dev/null || echo X" | nocr)"
+  dirty="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA.dirty' '$p' 2>/dev/null || echo X" | nocr)"
+  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
+  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
 }
-sha256_of() {      # $1=host $2=path
-  hrun "$1" "shasum -a 256 '$2' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f'
+sha256_of() {      # fail CLOSED on an empty/short hash
+  local h="$1" p="$2" v
+  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')"
+  [[ ${#v} -eq 64 ]] || die "$(hname "$h"): sha256 of $p returned '${v}' (not 64 hex) — refusing"
+  echo "$v"
 }
 
-# --- the two gates pf-0 proved were missing -----------------------------------
-quiescence_gate() {   # $1 = host. Bench ENDS must be quiet; load contaminates ASYMMETRICALLY.
-  local h="$1" busy
-  busy="$(hrun "$h" "pgrep -x codex >/dev/null && echo codex; pgrep -x cargo >/dev/null && echo cargo; pgrep -x rustc >/dev/null && echo rustc; true" | nocr | tr '\n' ' ')"
-  busy="$(echo "$busy" | xargs || true)"
-  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). Both Macs are bench ENDS — a busy end inflates one arm and MANUFACTURES P1 (.agents/machines.md). Stop them (do NOT blanket-kill the owner's sessions) and re-run."
+# --- gates: every one fails CLOSED (codex HIGH: they all failed OPEN) ----------
+norm_mac() { tr 'A-F' 'a-f' | awk -F: '{for(i=1;i<=NF;i++){printf "%s%02x", (i>1?":":""), strtonum("0x" $i)}; print ""}'; }
+
+quiescence_gate() {
+  local h="$1" out
+  out="$(hrun "$h" "pgrep -x codex >/dev/null 2>&1 && echo codex; pgrep -x cargo >/dev/null 2>&1 && echo cargo; pgrep -x rustc >/dev/null 2>&1 && echo rustc; echo __OK__" | nocr)" \
+    || die "$(hname "$h"): quiescence probe FAILED — a gate that cannot answer must not answer 'fine'"
+  [[ "$out" == *__OK__* ]] || die "$(hname "$h"): quiescence probe returned no sentinel — refusing"
+  local busy; busy="$(echo "$out" | grep -v __OK__ | tr '\n' ' ' | xargs || true)"
+  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
 }
-timemachine_gate() {   # $1 = host. FAIL-CLOSED — the hole pf-0 found.
+timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   local h="$1" running auto
-  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';'" | nocr | tr -cd '0-9')"
-  [[ "${running:-0}" == 1 ]] && die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench END (one destination is on skippy, the same 10GbE fabric)."
-  # AUTOBACKUP ENABLED is itself disqualifying, not a warning: macOS repeats
-  # HOURLY, so a backup can begin *inside* the window. pf-0's fired 1 minute
-  # before its run and the old gate never looked. A warning here would let the
-  # session start and be silently contaminated mid-flight.
-  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null || echo 0" | nocr | tr -cd '0-9')"
-  [[ "${auto:-0}" == 1 ]] && die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED — macOS repeats hourly, so a backup can start MID-SESSION. Disable it for the window (\`sudo tmutil disable\`) and re-enable after."
-  true
+  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
+  [[ "$running" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
+  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
+  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1; echo" | nocr | tr -cd '0-9')" || auto=""
+  [[ "$auto" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
+  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
 }
-spotlight_gate() {   # $1 = host. mds_stores is a recorded contaminant (.agents/machines.md).
-  # Instantaneous sample: `ps` %CPU is a DECAYING AVERAGE and reads a finished
-  # backup as 255% (learned in pf-0) — top -l 2 is the honest instrument.
+spotlight_gate() {
   local h="$1" cpu
-  cpu="$(hrun "$h" "top -l 2 -n 20 -o cpu -stats command,cpu 2>/dev/null | awk '/mds_stores|^mds /{c=\$NF} END{print c+0}'" | nocr | tr -cd '0-9.')"
-  awk -v c="${cpu:-0}" 'BEGIN{exit !(c+0 > 20)}' \
-    && die "$(hname "$h"): Spotlight (mds_stores) is actively indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
-  true
+  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null | awk '/^mds_stores/{c=\$2} END{printf \"%d\", c+0}'" | nocr | tr -cd '0-9')" || cpu=""
+  [[ "$cpu" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot sample Spotlight CPU — refusing"
+  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
+}
+load_gate() {
+  local h="$1" l ok
+  l="$(hrun "$h" "sysctl -n vm.loadavg" | nocr | awk '{print $2}')" || l=""
+  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die "$(hname "$h"): cannot read load1 (got '$l') — refusing"
+  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
+  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
 }
 load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
-load_gate() {   # $1 = host. The Macs idle at ~1.5-2.0; above 3.0 something is running.
-  local h="$1" l; l="$(load1 "$h")"
-  awk -v l="${l:-0}" 'BEGIN{exit !(l+0 > 3.0)}' \
-    && die "$(hname "$h"): load1 is $l (> 3.0) — a bench END must be quiet. Find what is running before starting a timed session."
-  true
+
+link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
+  local h="$1" o peer_ip want got route_nic
+  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"
+  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
+    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
+  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk '{print \$4}'" | nocr | norm_mac)"
+  [[ -n "$got" && "$got" != "(incomplete)" ]] || die "$(hname "$h"): no ARP entry for $peer_ip"
+  [[ "$got" == "$want" ]] \
+    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
+  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
+  [[ "$route_nic" == "$(hnic "$h")" ]] \
+    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
 }
 
 preflight() {
-  [[ "$RUNS" -ge 2 ]] || die "RUNS must be >= 2"
-  local h p
+  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
+  local c
+  for c in ${CELLS//,/ }; do
+    [[ ",$REGISTERED_CELLS," == *",$c,"* ]] \
+      || die "cell '$c' is not in the REGISTERED set ($REGISTERED_CELLS) — a misspelled cell must not silently drop a control or measure nothing"
+  done
+  local h p w want got wantb gotb
   for h in n q; do
-    quiescence_gate "$h"
-    timemachine_gate "$h"
-    spotlight_gate "$h"
-    load_gate "$h"
+    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
     for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
       hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
-      embeds_clean "$h" "$p" \
-        || die "$(hname "$h"): $p does not embed a CLEAN +$EXPECT_SHA (same-build rule, D-2026-07-05-2)"
+      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
     done
-    # Cold-cache capability is METHODOLOGY, not a nicety — hard gate, fail closed.
-    hrun "$h" "sudo -n /usr/sbin/purge" \
-      || die "$(hname "$h") cannot purge without a password (need the NOPASSWD /usr/sbin/purge sudoers rule) — every run would read WARM"
-    hrun "$h" "pgrep -x blit-daemon >/dev/null" \
-      && die "$(hname "$h"): a blit-daemon is already running — stop it first (stale-daemon refusal)"
-    # Fixtures.
-    local w want got
+    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
+    if hrun "$h" "pgrep -x blit-daemon >/dev/null 2>&1"; then die "$(hname "$h"): a blit-daemon is already running — stop it first"; fi
     for w in large mixed small; do
-      want="$(eval echo "\$FIX_COUNT_$w")"
+      want="$(eval echo "\$FIX_COUNT_$w")"; wantb="$(eval echo "\$FIX_BYTES_$w")"
       got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
-      [[ "${got:-0}" == "$want" ]] \
-        || die "$(hname "$h"): src_$w has ${got:-0}/$want files — stage the fixtures before a timed run"
+      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
+      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
+        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
     done
+    link_gate "$h"
   done
-  # Link validity, MEASURED not assumed (.agents/machines.md): the peer's ARP entry
-  # must be the PEER's MAC, never our own — a host route on a directly-connected
-  # subnet installs a BLACK HOLE that still reports the right interface.
-  local pmac
-  ping -c1 -W1 "$Q_IP" >/dev/null 2>&1 || true
-  pmac="$(arp -n "$Q_IP" 2>/dev/null | awk '{print $4}')"
-  [[ -n "$pmac" && "$pmac" != "(incomplete)" ]] || die "no ARP entry for q ($Q_IP) — the link is not up"
-  log "preflight OK  build=$EXPECT_SHA (harness HEAD=$HARNESS_HEAD)  runs/arm=$RUNS  q_mac=$pmac"
+  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
 }
 
 write_manifest() {
   local f="$OUT_DIR/staging-manifest.txt" h
-  { echo "# harness_head=$HARNESS_HEAD binary_identity=$EXPECT_SHA"
+  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
+    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
+    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
+    echo "# cells=$CELLS"
     echo "host,role,sha,sha256,path"
     for h in n q; do
       echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
       echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$(hdaemon "$h")"),$(hdaemon "$h")"
     done; } > "$f"
-  log "staging manifest recorded (4 hashes)"
+  log "staging manifest recorded (harness sha256 + 4 binary hashes + every threshold)"
 }
 
-# --- daemons (both ends serve: the source daemon serves pulls, the dest daemon
-#     serves pushes) --------------------------------------------------------
+# --- daemons ------------------------------------------------------------------
 N_PID=""; Q_PID=""
-daemon_start() {   # $1 = host
+daemon_start() {
   local h="$1" cfg mod bin pid
-  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"
-  cfg="$mod/mm-bench.toml"
+  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   hrun "$h" "mkdir -p '$mod'
 printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
 nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
-sleep 2
-pgrep -x blit-daemon | head -1" >/dev/null 2>&1 || true
+sleep 2" >/dev/null 2>&1 || true
   pid="$(hrun "$h" "pgrep -x blit-daemon | head -1" | nocr | tr -cd '0-9')"
   [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start (see $(hmod "$h")/mm-daemon.log)"
-  # Listening, not merely alive (the rig-W lesson: the process check passed while
-  # the socket was not accepting, and the smoke died on a transport error).
-  hrun "$h" "nc -z -G 3 127.0.0.1 $PORT" \
-    || die "$(hname "$h"): daemon pid $pid is up but NOT listening on $PORT"
   [[ "$h" == n ]] && N_PID="$pid" || Q_PID="$pid"
   log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
 }
-daemon_stop() {   # $1 = host; PID-scoped, comm-verified, and the death is VERIFIED
+# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
+# handshake reached some listener's backlog — not that the daemon speaks blit).
+smoke() {
+  local h="$1" o probe
+  o="$(other "$h")"
+  probe="$(hmod "$o")/mm_smoke_${SESSION_TAG}"
+  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 || true
+  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
+    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
+    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
+  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
+  log "smoke ok: $(hname "$h") daemon serves blit"
+}
+daemon_stop() {
   local h="$1" pid; pid="$([[ "$h" == n ]] && echo "$N_PID" || echo "$Q_PID")"
   [[ -n "$pid" ]] || return 0
-  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; true" >/dev/null 2>&1 || true
-  if hrun "$h" "ps -p $pid >/dev/null 2>&1"; then
+  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; echo __DONE__" >/dev/null 2>&1 || true
+  # A teardown that cannot be VERIFIED is a failure, not a success (codex MEDIUM).
+  if hrun "$h" "ps -p $pid >/dev/null 2>&1 && echo ALIVE || echo GONE" | nocr | grep -q ALIVE; then
     log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"
     return 1
   fi
   log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
 }
-cleanup() {
-  daemon_stop n || true
-  daemon_stop q || true
-  rm -rf "$MUX" 2>/dev/null || true
-}
+cleanup() { daemon_stop n || true; daemon_stop q || true; rm -rf "$MUX" 2>/dev/null || true; }
 trap cleanup EXIT
 
-# --- cold + drain -------------------------------------------------------------
+# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
 RUN_DRAIN=""; RUN_COLD=""
-drain_host() {   # $1 = DESTINATION host; wait until its disk is quiet (macOS iostat)
+drain_host() {
   hrun "$1" "quiet=0
 for i in \$(seq 1 $DRAIN_ITERS); do
   w=\$(iostat -d -w 2 -c 2 disk0 2>/dev/null | tail -1 | awk '{print \$3}')
   ok=\$(awk -v w=\"\${w:-99}\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
-  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained \${i}x2s\"; exit 0; fi
+  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
 done
 echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
 }
-prep_run() {   # $1 = dest host. Drain the DEST, then cold BOTH ends. A failed purge VOIDS.
-  local dh="$1" out cn=ok cq=ok
-  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"; RUN_DRAIN="${RUN_DRAIN// /_}"
-  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
+prep_run() {   # $1 = dest host
+  local dh="$1" cn=ok cq=ok out
+  # Purge BOTH ends first — the purge itself dirties the disk, so a drain
+  # certified before it proves nothing (codex HIGH).
   hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
+  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
+  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
 }
 
-# --- durability: ALWAYS the DESTINATION host, identically for both arms --------
-fsync_tree_ms() {   # $1 = DEST host, $2 = landed path. Prints ms, or NA (=> VOID).
+# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
+RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0
+fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes" or "NA 0 0"
   local out
-  out="$(hrun "$1" "python3 - '$2' <<'PYEOF'
+  out="$(hrun "$1" "sleep $(awk -v m=$SETTLE_MS 'BEGIN{printf \"%.3f\", m/1000}')
+python3 - '$2' <<'PYEOF'
 import os, sys, time
+p = sys.argv[1]
+if not os.path.isdir(p):
+    print('F:NA:0:0:F')          # a MISSING tree must never read as a fast flush
+    raise SystemExit
 t = time.monotonic()
-for root, dirs, files in os.walk(sys.argv[1]):
-    for name in files:
-        fd = os.open(os.path.join(root, name), os.O_RDONLY)
+files = 0
+nbytes = 0
+for root, _d, fs in os.walk(p):
+    for name in fs:
+        fp = os.path.join(root, name)
+        fd = os.open(fp, os.O_RDONLY)
         os.fsync(fd)
         os.close(fd)
-print('F:%d:F' % int((time.monotonic() - t) * 1000))
-PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p' | head -1)"
-  echo "${out:-NA}"   # a failed fsync must never read as a plausible flush
+        files += 1
+        nbytes += os.fstat(os.open(fp, os.O_RDONLY)).st_size if False else os.path.getsize(fp)
+print('F:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes))
+PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3/p' | head -1)"
+  echo "${out:-NA 0 0}"
 }
 
 # --- one timed run ------------------------------------------------------------
-RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
-timed_run() {   # $1=initiating host $2=src spec $3=dst spec $4=DEST host $5=landed path $6=flag
-  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" out bin
+RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
+timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
+  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin r
   bin="$(hblit "$ih")"
   prep_run "$dh"
-  # The window is self-timed ON the initiating host (locally for nagatha; inside a
-  # SINGLE ssh for q), so dispatch/round-trip is outside it by construction.
-  # NO sync in here — durability is charged to the destination, below.
-  # ONE python process brackets the transfer. Two reasons, both load-bearing:
-  #   1. time.monotonic()'s REFERENCE POINT IS UNDEFINED ACROSS PROCESSES (python
-  #      docs; only same-process differences are valid). The first draft of this
-  #      function read t0 in one `python3 -c` and t1 in another and subtracted
-  #      them — which is meaningless, and measurably so: consecutive reads on this
-  #      rig returned -1 and -4 ms. It would have produced garbage timings that
-  #      still looked plausible.
-  #   2. Interpreter startup now falls OUTSIDE the timer. With a per-invocation
-  #      clock read, startup sat INSIDE the window — and since the two arms of a
-  #      cell are initiated by DIFFERENT Macs, any startup difference between them
-  #      is charged to one arm. That is the otp-2w failure mode (a cost billed to
-  #      one arm and not the other) in a new disguise.
-  out="$(hrun "$ih" "python3 - '$bin' '$src' '$dst' '$flag' <<'PYEOF'
-import subprocess, sys, time
-binp, src, dst, flag = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
-cmd = [binp, 'copy', src, dst, '--yes'] + ([flag] if flag else [])
-err = open('/tmp/mm-client.err', 'wb')
-t = time.monotonic()
-rc = subprocess.call(cmd, stdout=subprocess.DEVNULL, stderr=err)
-print('R:%d,%d:R' % (int((time.monotonic() - t) * 1000), rc))
-PYEOF" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
+  out="$(hrun "$ih" "t0=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
+'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
+t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
+echo \"R:\$((t1-t0)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
-  RUN_FLUSH="$(fsync_tree_ms "$dh" "$landed")"
+  read -r RUN_FLUSH RUN_FILES RUN_BYTES <<<"$(fsync_tree "$dh" "$landed")"
   RUN_VALID=yes
-  if [[ "$RUN_FLUSH" == NA ]]; then RUN_VALID=no; RUN_FLUSH=0; fi
+  local wc wb; wc="$(eval echo "\$FIX_COUNT_$w")"; wb="$(eval echo "\$FIX_BYTES_$w")"
+  if [[ "$RUN_FLUSH" == NA ]]; then
+    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
+    RUN_VALID=no; RUN_FLUSH=0
+  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
+    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
+    RUN_VALID=no
+  fi
   RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
 }
 
-# --- arms: the ONLY variable is which host's CLI initiates --------------------
+# --- arms ---------------------------------------------------------------------
+# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
+# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
+# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
+# above is what makes a wrong path fatal instead of silently free.
 CUR_W=""; CUR_FLAG=""
-arm_srcinit() {    # the SOURCE host pushes into the DEST daemon
-  local cell="$1" rid="$2" sh="$3" dh="$4" landed
-  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}/src_$CUR_W"
-  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" \
-                  "$(hip "$dh"):$PORT:/bench/${SESSION_TAG}_${cell}_${rid}/" \
-                  "$dh" "$landed" "$CUR_FLAG"
-  hrun "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" >/dev/null 2>&1 || true
+arm_srcinit() {
+  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
+  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
+            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
+  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
 }
-arm_destinit() {   # the DEST host pulls from the SOURCE daemon
-  local cell="$1" rid="$2" sh="$3" dh="$4" landed
-  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}"
-  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" \
-                  "$landed" \
-                  "$dh" "$landed" "$CUR_FLAG"
-  hrun "$dh" "rm -rf '$landed'" >/dev/null 2>&1 || true
+arm_destinit() {
+  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
+  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
+            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
+  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
 }
 
-CSV="$OUT_DIR/runs.csv"; echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid" > "$CSV"
+CSV="$OUT_DIR/runs.csv"
+echo "cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
 META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"
 
-run_pair_loop() {   # $1=cell $2=src host $3=dest host
+run_pair_loop() {
   local cell="$1" sh="$2" dh="$3"
   local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   while (( valid < RUNS && attempts < max )); do
     attempts=$(( attempts + 1 ))
-    local order pair=yes rowA="" rowB="" arm rid aname init
+    local order pair=yes rowA="" rowB="" arm aname init rid run
     if (( slot % 2 )); then order="A B"; else order="B A"; fi
     for arm in $order; do
       if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
-      rid="${aname}_s${slot}a${attempts}"
-      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh"
-      else arm_destinit "$cell" "$rid" "$sh" "$dh"; fi
+      rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
+      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh" "$run"
+      else arm_destinit "$cell" "$rid" "$sh" "$dh" "$run"; fi
       [[ "$RUN_VALID" == yes ]] || pair=no
-      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
+      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
       [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
-      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
+      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
     done
     echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
     if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
@@ -401,100 +433,10 @@ run_pair_loop() {   # $1=cell $2=src host $3=dest host
 }
 
 compute_verdicts() {
-  python3 - "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" "$OUT_DIR/paired.csv" <<'PY'
-import csv, sys
-runs_p, meta_p, sum_p, ver_p, pair_p = sys.argv[1:6]
-rows = list(csv.DictReader(open(runs_p)))
-meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
-by, void = {}, {}
-# PAIRED slots: the pre-registered noise model. Each ABBA slot yields a matched
-# (srcinit, destinit) pair under identical conditions, so d_i = destinit - srcinit
-# is a WITHIN-slot difference — no between-session drift can enter it. pf-0's
-# review established that an unpaired spread is NOT a noise floor.
-slots = {}
-for r in rows:
-    k = (r["cell"], r["arm"])
-    if r["valid"] == "yes":
-        by.setdefault(k, []).append(int(r["ms"]))
-        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
-    else:
-        void[k] = void.get(k, 0) + 1
-
-def med(v):
-    v = sorted(v); n = len(v)
-    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
-
-def complete(c):
-    if c not in meta or meta[c]["complete"] != "yes":
-        return False
-    arms = [a for (cc, a) in by if cc == c]
-    return "srcinit" in arms and "destinit" in arms
-
-with open(sum_p, "w") as f:
-    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,pairs_attempted,runs\n")
-    for (c, a) in sorted(by):
-        if not complete(c):
-            continue
-        v = by[(c, a)]
-        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
-        # every run is printed: pf-0 found the fast arm BIMODAL, and a median
-        # alone hides a mode-mixture shift that would fake a recovery.
-        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s,%s\n" % (
-            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
-            void.get((c, a), 0), meta[c]["pairs_attempted"],
-            " ".join(str(x) for x in v)))
-
-# The paired statistics the pre-registered rule is actually graded on.
-#   D = median(d_i)  -> the effect (positive = destination-initiated is slower)
-#   S = spread(d_i)  -> the PAIRED noise floor (max-min; IQR also reported)
-#   MDE = S          -> conservatively, the smallest |D| this cell can resolve
-# DELTA_REF = 230 ms: rig W's measured Delta_P1, the effect size this rig must be
-# able to see before any "vanishes" claim is permitted (the POWER GATE).
-DELTA_REF = 230

exec
/bin/zsh -lc 'git show 24660ae''^:scripts/bench_otp12pf_mac.sh | nl -ba | sed -n '"'330,470p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-EpbFer1d' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-40HzMtH2' (errno=Operation not permitted)
   330	  #      them — which is meaningless, and measurably so: consecutive reads on this
   331	  #      rig returned -1 and -4 ms. It would have produced garbage timings that
   332	  #      still looked plausible.
   333	  #   2. Interpreter startup now falls OUTSIDE the timer. With a per-invocation
   334	  #      clock read, startup sat INSIDE the window — and since the two arms of a
   335	  #      cell are initiated by DIFFERENT Macs, any startup difference between them
   336	  #      is charged to one arm. That is the otp-2w failure mode (a cost billed to
   337	  #      one arm and not the other) in a new disguise.
   338	  out="$(hrun "$ih" "python3 - '$bin' '$src' '$dst' '$flag' <<'PYEOF'
   339	import subprocess, sys, time
   340	binp, src, dst, flag = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
   341	cmd = [binp, 'copy', src, dst, '--yes'] + ([flag] if flag else [])
   342	err = open('/tmp/mm-client.err', 'wb')
   343	t = time.monotonic()
   344	rc = subprocess.call(cmd, stdout=subprocess.DEVNULL, stderr=err)
   345	print('R:%d,%d:R' % (int((time.monotonic() - t) * 1000), rc))
   346	PYEOF" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   347	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   348	  RUN_FLUSH="$(fsync_tree_ms "$dh" "$landed")"
   349	  RUN_VALID=yes
   350	  if [[ "$RUN_FLUSH" == NA ]]; then RUN_VALID=no; RUN_FLUSH=0; fi
   351	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   352	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
   353	}
   354	
   355	# --- arms: the ONLY variable is which host's CLI initiates --------------------
   356	CUR_W=""; CUR_FLAG=""
   357	arm_srcinit() {    # the SOURCE host pushes into the DEST daemon
   358	  local cell="$1" rid="$2" sh="$3" dh="$4" landed
   359	  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}/src_$CUR_W"
   360	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" \
   361	                  "$(hip "$dh"):$PORT:/bench/${SESSION_TAG}_${cell}_${rid}/" \
   362	                  "$dh" "$landed" "$CUR_FLAG"
   363	  hrun "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" >/dev/null 2>&1 || true
   364	}
   365	arm_destinit() {   # the DEST host pulls from the SOURCE daemon
   366	  local cell="$1" rid="$2" sh="$3" dh="$4" landed
   367	  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}"
   368	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" \
   369	                  "$landed" \
   370	                  "$dh" "$landed" "$CUR_FLAG"
   371	  hrun "$dh" "rm -rf '$landed'" >/dev/null 2>&1 || true
   372	}
   373	
   374	CSV="$OUT_DIR/runs.csv"; echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid" > "$CSV"
   375	META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"
   376	
   377	run_pair_loop() {   # $1=cell $2=src host $3=dest host
   378	  local cell="$1" sh="$2" dh="$3"
   379	  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   380	  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   381	  while (( valid < RUNS && attempts < max )); do
   382	    attempts=$(( attempts + 1 ))
   383	    local order pair=yes rowA="" rowB="" arm rid aname init
   384	    if (( slot % 2 )); then order="A B"; else order="B A"; fi
   385	    for arm in $order; do
   386	      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   387	      rid="${aname}_s${slot}a${attempts}"
   388	      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh"
   389	      else arm_destinit "$cell" "$rid" "$sh" "$dh"; fi
   390	      [[ "$RUN_VALID" == yes ]] || pair=no
   391	      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   392	      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
   393	      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   394	    done
   395	    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   396	    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   397	    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
   398	  done
   399	  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
   400	  else echo "$cell,$attempts,yes" >> "$META"; fi
   401	}
   402	
   403	compute_verdicts() {
   404	  python3 - "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" "$OUT_DIR/paired.csv" <<'PY'
   405	import csv, sys
   406	runs_p, meta_p, sum_p, ver_p, pair_p = sys.argv[1:6]
   407	rows = list(csv.DictReader(open(runs_p)))
   408	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   409	by, void = {}, {}
   410	# PAIRED slots: the pre-registered noise model. Each ABBA slot yields a matched
   411	# (srcinit, destinit) pair under identical conditions, so d_i = destinit - srcinit
   412	# is a WITHIN-slot difference — no between-session drift can enter it. pf-0's
   413	# review established that an unpaired spread is NOT a noise floor.
   414	slots = {}
   415	for r in rows:
   416	    k = (r["cell"], r["arm"])
   417	    if r["valid"] == "yes":
   418	        by.setdefault(k, []).append(int(r["ms"]))
   419	        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
   420	    else:
   421	        void[k] = void.get(k, 0) + 1
   422	
   423	def med(v):
   424	    v = sorted(v); n = len(v)
   425	    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
   426	
   427	def complete(c):
   428	    if c not in meta or meta[c]["complete"] != "yes":
   429	        return False
   430	    arms = [a for (cc, a) in by if cc == c]
   431	    return "srcinit" in arms and "destinit" in arms
   432	
   433	with open(sum_p, "w") as f:
   434	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,pairs_attempted,runs\n")
   435	    for (c, a) in sorted(by):
   436	        if not complete(c):
   437	            continue
   438	        v = by[(c, a)]
   439	        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   440	        # every run is printed: pf-0 found the fast arm BIMODAL, and a median
   441	        # alone hides a mode-mixture shift that would fake a recovery.
   442	        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s,%s\n" % (
   443	            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
   444	            void.get((c, a), 0), meta[c]["pairs_attempted"],
   445	            " ".join(str(x) for x in v)))
   446	
   447	# The paired statistics the pre-registered rule is actually graded on.
   448	#   D = median(d_i)  -> the effect (positive = destination-initiated is slower)
   449	#   S = spread(d_i)  -> the PAIRED noise floor (max-min; IQR also reported)
   450	#   MDE = S          -> conservatively, the smallest |D| this cell can resolve
   451	# DELTA_REF = 230 ms: rig W's measured Delta_P1, the effect size this rig must be
   452	# able to see before any "vanishes" claim is permitted (the POWER GATE).
   453	DELTA_REF = 230
   454	with open(pair_p, "w") as f:
   455	    f.write("cell,n_pairs,D_median_ms,S_spread_ms,IQR_ms,MDE_ms,fast_arm_ms,"
   456	            "delta_ref_ms,ref_ratio_on_fast_arm,powered_for_null,d_i\n")
   457	    for c in sorted(meta):
   458	        ds = sorted(v["destinit"] - v["srcinit"]
   459	                    for (cc, _r), v in slots.items()
   460	                    if cc == c and "srcinit" in v and "destinit" in v)
   461	        if not ds:
   462	            continue
   463	        n = len(ds)
   464	        D = med(ds)
   465	        S = max(ds) - min(ds)
   466	        q1, q3 = ds[n // 4], ds[(3 * n) // 4 - (1 if n % 4 == 0 else 0)]
   467	        fast = min(med(by[(c, "srcinit")]), med(by[(c, "destinit")])) if complete(c) else 0
   468	        # A 230 ms effect is only VISIBLE against a ratio bar if the fast arm is
   469	        # fast enough: at a 2.3 s fast arm, 230 ms IS exactly 1.10 and sits ON the
   470	        # bar. So the null branch needs BOTH: MDE <= DELTA_REF, and a ref-sized

exec
/bin/zsh -lc "/usr/bin/python3 -c 'import sys; print(sys.version)' && for i in 1 2 3 4 5; do /usr/bin/python3 -c 'import time;print(int(time.monotonic()*1000))'; done" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-zawU7XdG' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-kbvkRXiq' (errno=Operation not permitted)
3.9.6 (default, May 22 2026, 11:13:45) 
[Clang 21.0.0 (clang-2100.1.1.101)]
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-wDaHsQzK' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-aFOzMJvy' (errno=Operation not permitted)
3
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-PUQyYREU' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-PysPZFvb' (errno=Operation not permitted)
3
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-4P6M2jid' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-uchcbPGg' (errno=Operation not permitted)
3
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-yzcUWHEi' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-Ip3Sw87E' (errno=Operation not permitted)
3
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-0gHcPW1W' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-DxnhfMqs' (errno=Operation not permitted)
3

exec
/bin/zsh -lc "bash -c 't0="'$(/usr/bin/python3 -c '"'\\''import time;print(int(time.monotonic()*1000))'\\''); sleep 1; t1="'$(/usr/bin/python3 -c '"'\\''import time;print(int(time.monotonic()*1000))'\\''); printf \"t0=%s t1=%s delta=%s (after 1000ms sleep)\\n\" \""'$t0" "$t1" "$((t1-t0))"'"'" in /Users/michael/Dev/blit_v2
 succeeded in 1020ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-nr1rYtIO' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-IoYl68RU' (errno=Operation not permitted)
t0=4 t1=5 delta=1 (after 1000ms sleep)

exec
/bin/zsh -lc "tmutil status 2>&1 | sed -n '1,20p'; defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>&1 | sed -n '1,10p'; sysctl -n vm.loadavg; top -l 1 -n 1 -stats command,cpu 2>/dev/null | sed -n '1,12p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
Backup session status:
{
    ClientID = "com.apple.backupd";
    Running = 0;
}
0
sysctl: sysctl fmt -1 1024 1: Operation not permitted

2026-07-14T08:42:48.818620Z ERROR codex_core::tools::router: error=timeout_ms must be at least 10000
2026-07-14T08:42:49.725090Z ERROR codex_models_manager::manager: failed to refresh available models: timeout waiting for child process to exit
exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '1,90p'; nl -ba docs/STATE.md | sed -n '1,35p'" in /Users/michael/Dev/blit_v2
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
     1	# STATE — single entry point for "what is true right now"
     2	
     3	Last updated: 2026-07-14
     4	
     5	- **NEXT ACTION — the MAC↔MAC RUN (Queue 1(ii)), pending its round-2 instrument review.** Design **rev 3** + the reworked harness are committed (`24660ae`): `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`, `scripts/bench_otp12pf_mac.sh`, `scripts/otp12pf_mac_verdict.py` (mechanizes the rule), `scripts/otp12pf_mac_verdict_test.py` (guard test, mutation-proven). **NO DATA TAKEN YET.** Two codex rounds → **20 findings, 20/20 accepted** (design 9, instrument 11): `.review/results/macmac-{prereg,harness}.gpt-verdict.md`. **⚠ TO RUN IT THE OWNER MUST CLOSE THEIR CODEX SESSIONS** — nagatha is now a bench **END**, not just the driver, and the quiescence gate refuses to start while `codex`/`cargo`/`rustc` runs on **either** Mac (it already fired, correctly, on its first invocation). Time Machine is OFF on both Macs (required: the gate is fail-closed on autobackup merely *enabled*). Rig: nagatha `10.1.10.92` ↔ `q` `10.1.10.54`, both 10GbE/9000, build pinned `f35702a`. **Then `pf-1`** (the HARD GATE), which two pf-0 results BIND: between-session grading is dead (a 20% recovery = 46 ms sits under the 78 ms floor), so pf-1 must **measure its own paired within-session floor** before grading; and the fast arm is **BISTABLE** — grade the distribution, not the median.
     6	- **⚠ THE MAC↔MAC RIG IS *NOT* AN H1 DISCRIMINATOR — retracted 2026-07-14.** The earlier claim ("reproduces ⇒ **H1 DIES**, H1 accuses the *Windows* accept branch") was **WRONG**: H1 accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **"Windows" appears nowhere in H1**, and that code runs on macOS too, so a reproduction is *consistent with* H1. H1 now carries a **canonical note** in the parent plan so the shorthand cannot mislead again. What the rig **does** answer, scoped to **this pair**: **can P1 occur WITHOUT a Windows peer?** Reproduces ⇒ P1 is **not** waivable "platform residue" and code hypotheses strengthen (it does **not** prove a platform-*general* cost). A null ⇒ P1 did not reproduce **on this pair** — consistent with "Windows required" but **not proof** of it, and only reportable at all if the run **excludes a bar-breaching effect** (else `INCONCLUSIVE-UNDERPOWERED`).
     7	- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
     8	- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
     9	- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
    10	- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
    11	- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
    12	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
    13	- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
    14	- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**
    15	
    16	Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
    17	≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
    18	procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
    19	
    20	## Now (active work)
    21	
    22	- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
    23	  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
    24	  invariant (plan doc, verbatim): ONE block of transfer code;
    25	  direction/initiator/verb can NEVER affect wall time by blit's doing
    26	  — impossible by construction because the per-direction drivers and
    27	  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
    28	  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
    29	  cells. **D-2026-07-05-2: same-build peers only, refusal at session
    30	  open.** Progress (each slice through the codex loop; per-slice
    31	  detail lives in DEVLOG + `.review/`, NOT here):
    32	  - **Closed `[x]`: otp-1 … otp-11** — the whole session machine, the
    33	    baselines (otp-2/2w), the **CUTOVER DELETION** (4 drivers +
    34	    `Push`/`PullSync` + 13 messages out of tree AND proto, −13.8k lines,
    35	    no bridge; relay removed D-2026-07-11-1), and **otp-11b's deletion of

exec
/bin/zsh -lc 'git diff --check 24660ae''^ 24660ae -- scripts/bench_otp12pf_mac.sh scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "bash -c 'set -o pipefail; out=\""'$(top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null | awk '"'\\''/"'^mds_stores/{c=$2} END{printf "%d", c+0}'"'\\'' | tr -cd '\\''0-9'\\'')\"; rc="'$?; printf "spotlight capture=<%s> rc=%s (top itself is denied here)'"\\n\" \""'$out" "$rc"; pgrep -x __certainly_not_a_process__ >/dev/null; printf "pgrep absent rc=%s'"\\n\" \""'$?"'"'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-4lr7QShp' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-9OAvUNJ2' (errno=Operation not permitted)

 succeeded in 0ms:
spotlight capture=<0> rc=126 (top itself is denied here)
sysmon request failed with error: sysmond service not found
pgrep: Cannot get process list
pgrep absent rc=3

exec
/bin/zsh -lc "bash -n scripts/bench_otp12pf_mac.sh && python3 -c 'import ast; [ast.parse(open(p).read(), filename=p) for p in [\"scripts/otp12pf_mac_verdict.py\",\"scripts/otp12pf_mac_verdict_test.py\"]]; print(\"syntax ok\")'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-Vak4hylW' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-dtIO3hSt' (errno=Operation not permitted)
syntax ok

codex
This is not converging toward “ready.” The hardest failures are now reproduced: the transfer timer regressed to cross-process `monotonic()` (a 1,000 ms delay measured as 1 ms here), a failed control can still yield session `VANISHES`, and a constant 230 ms effect in all eight pairs can also yield `VANISHES`. I’m reconciling those with the remaining fail-closed and provenance defects for the concise verdict.
exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict.py | sed -n '105,245p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   105	            continue
   106	        d = [v["destinit"] - v["srcinit"]
   107	             for (cc, _run), v in sorted(slots.items())
   108	             if cc == c and "srcinit" in v and "destinit" in v]
   109	        s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   110	        hi, lo = max(s_med, d_med), min(s_med, d_med)
   111	        bar = "PASS" if 10 * hi <= 11 * lo else "FAIL"      # integer-exact
   112	        D = med(d)
   113	        ci_lo, ci_hi = boot_ci(d)
   114	        p, k, n = sign_p(d)
   115	        breach = 0.10 * s_med                                # effect that reaches 1.10
   116	        powered = (ci_hi - ci_lo) < breach                   # can we exclude a breaching effect?
   117	
   118	        # UNSTABLE, as a STATISTIC not a vibe: an arm splits into two clusters
   119	        # separated by more than the paired spread, AND the bar verdict flips when
   120	        # graded on pooled runs instead of medians.
   121	        unstable = "no"
   122	        for arm in ("srcinit", "destinit"):
   123	            v = sorted(by[(c, arm)])
   124	            gaps = [(v[i + 1] - v[i], i) for i in range(len(v) - 1)]
   125	            gmax, gi = max(gaps) if gaps else (0, 0)
   126	            if gmax > (max(d) - min(d)) and gmax > 0:
   127	                pooled_hi = max(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
   128	                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
   129	                pooled_lo = min(sum(by[(c, "srcinit")]) / len(by[(c, "srcinit")]),
   130	                                sum(by[(c, "destinit")]) / len(by[(c, "destinit")]))
   131	                pooled_bar = "PASS" if 10 * pooled_hi <= 11 * pooled_lo else "FAIL"
   132	                if pooled_bar != bar:
   133	                    unstable = "yes"
   134	
   135	        if bar == "FAIL" and ci_lo > 0:
   136	            out = "REPRODUCES"
   137	        elif bar == "FAIL" and ci_hi < 0:
   138	            out = "INVERSION"
   139	        elif bar == "PASS" and ci_lo > -breach and ci_hi < breach:
   140	            out = "VANISHES"
   141	        elif bar == "PASS" and not powered:
   142	            out = "UNDERPOWERED"
   143	        elif bar == "PASS" and (ci_lo > 0 or ci_hi < 0):
   144	            out = "PARTIAL"
   145	        else:
   146	            out = "INCONCLUSIVE"
   147	        if unstable == "yes":
   148	            out = "UNSTABLE"
   149	
   150	        cell_outcome[c] = out
   151	        cell_detail[c] = dict(D=D, ci=(ci_lo, ci_hi), p=p, k=k, n=n, bar=bar,
   152	                              ratio=hi / lo if lo else 0.0, breach=breach)
   153	        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%d/%d,%d,%d,%s,%s,%s\n" % (
   154	            c, len(d), s_med, d_med, (hi / lo if lo else 0.0), bar, D, ci_lo, ci_hi,
   155	            p, k, n, breach, DELTA_REF, "yes" if powered else "no", unstable, out))
   156	
   157	# ---- per-cell invariance rows (unchanged shape) ------------------------------
   158	with open(ver_p, "w") as f:
   159	    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,delta_ms,bar,outcome\n")
   160	    for c in sorted(meta):
   161	        if not complete(c):
   162	            f.write("%s,invariance,srcinit,destinit,,,,,1.10,INCOMPLETE\n" % c)
   163	            continue
   164	        s, dd = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   165	        hi, lo = max(s, dd), min(s, dd)
   166	        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,1.10,%s\n" % (
   167	            c, s, dd, hi / lo if lo else 0.0, dd - s,
   168	            "PASS" if 10 * hi <= 11 * lo else "FAIL"))
   169	
   170	# ---- SESSION VERDICT: the six registered outcomes, in strict precedence ------
   171	lines = []
   172	ctrl = [c for c in CONTROL_CELLS if c in cell_outcome]
   173	verd = [c for c in VERDICT_CELLS if c in cell_outcome]
   174	
   175	ctrl_fail = [c for c in ctrl
   176	             if cell_outcome[c] not in ("VANISHES", "INCONCLUSIVE", "UNDERPOWERED")
   177	             and cell_detail.get(c, {}).get("bar") == "FAIL"]
   178	incomplete = [c for c in (ctrl + verd) if cell_outcome[c] == "INCOMPLETE"]
   179	
   180	if incomplete:
   181	    verdict = "INCOMPLETE"
   182	    why = "cells did not complete: %s" % ", ".join(incomplete)
   183	elif ctrl_fail:
   184	    # 1. RIG-VOID -- a rig whose control fails cannot adjudicate a TCP-only claim.
   185	    verdict = "RIG-VOID"
   186	    why = ("control cell(s) FAILED the 1.10 bar: %s. The rig is not measuring "
   187	           "cleanly; NO verdict may be read." % ", ".join(ctrl_fail))
   188	else:
   189	    outs = {c: cell_outcome[c] for c in verd}
   190	    repro = [c for c, o in outs.items() if o == "REPRODUCES"]
   191	    inv = [c for c, o in outs.items() if o == "INVERSION"]
   192	    unst = [c for c, o in outs.items() if o == "UNSTABLE"]
   193	    van = [c for c, o in outs.items() if o == "VANISHES"]
   194	    part = [c for c, o in outs.items() if o == "PARTIAL"]
   195	    under = [c for c, o in outs.items() if o in ("UNDERPOWERED", "INCONCLUSIVE")]
   196	
   197	    if unst:
   198	        verdict = "UNSTABLE"
   199	        why = ("bimodal arm(s) whose verdict flips on pooled runs: %s. Report as "
   200	               "unstable, NOT resolved." % ", ".join(unst))
   201	    elif repro and inv:
   202	        verdict = "MIXED-SIGN"
   203	        why = ("reproduces in %s but INVERTS in %s -- a host x role interaction "
   204	               "this rig cannot decompose. INCONCLUSIVE for the pairing question."
   205	               % (", ".join(repro), ", ".join(inv)))
   206	    elif repro:
   207	        verdict = "REPRODUCES"
   208	        why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: "
   209	               "it shows P1 CAN occur macOS<->macOS -- it does NOT establish a "
   210	               "platform-general layout cost, and it does NOT kill H1 (H1 accuses "
   211	               "code, and that code runs here too)." % ", ".join(repro))
   212	    elif inv:
   213	        verdict = "INVERSION"
   214	        why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank "
   215	               "this as 'P1 absent'." % ", ".join(inv))
   216	    elif under:
   217	        verdict = "INCONCLUSIVE-UNDERPOWERED"
   218	        why = ("cells cannot exclude a bar-breaching effect: %s. A PASS here is NOT "
   219	               "'P1 vanishes' -- the instrument could not have seen it (pf-0's "
   220	               "error, pre-empted)." % ", ".join(under))
   221	    elif van and len(van) == len(verd):
   222	        verdict = "VANISHES"
   223	        why = ("both TCP-mixed cells EXCLUDE a bar-breaching effect (equivalence). "
   224	               "Scoped to THIS pair: P1 did not reproduce macOS<->macOS. That is "
   225	               "CONSISTENT with 'Windows is required' but does NOT prove it -- it "
   226	               "could be a property of these two machines/disks/OS version.")
   227	    elif part:
   228	        verdict = "PARTIAL"
   229	        why = ("a real but sub-bar asymmetry in: %s. Neither a reproduction nor a "
   230	               "vanish; pf-1 owns it." % ", ".join(part))
   231	    else:
   232	        verdict = "INCONCLUSIVE"
   233	        why = "no registered case matched cleanly; report the cells verbatim."
   234	
   235	lines.append("SESSION VERDICT: %s" % verdict)
   236	lines.append("")
   237	lines.append(why)
   238	lines.append("")
   239	lines.append("Per-cell outcomes (the rule is graded on paired.csv):")
   240	for c in sorted(cell_outcome):
   241	    d = cell_detail.get(c)
   242	    if d:
   243	        lines.append("  %-14s %-12s ratio=%.3f bar=%s  D=%+dms CI=[%+d,%+d] sign_p=%.3f (%d/%d pos)"
   244	                     % (c, cell_outcome[c], d["ratio"], d["bar"], d["D"],
   245	                        d["ci"][0], d["ci"][1], d["p"], d["k"], d["n"]))

codex
## Findings

- **BLOCKER — [scripts/bench_otp12pf_mac.sh:367](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:367)** — The transfer timer regressed to subtracting `monotonic()` values from two Python processes. On this Mac’s Python 3.9, a 1,000 ms delay measured as 1 ms. Valid rows therefore contain roughly fsync time alone; negative clock noise is selectively voided. This can manufacture or mask either arm effect.

- **BLOCKER — [scripts/bench_otp12pf_mac.sh:225](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:225), [scripts/otp12pf_mac_verdict.py:172](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:172)** — BLOCKER 1 is not fixed. `CELLS` may omit controls or one measurand; absent cells are filtered rather than `INCOMPLETE`. A one-cell run can emit `VANISHES` while claiming “both” cells vanished. Additionally, a control with bar `FAIL` but outcome `INCONCLUSIVE` is excluded from RIG-VOID at line 175, and UNSTABLE controls are ignored at line 192.

- **BLOCKER — [scripts/otp12pf_mac_verdict.py:35](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:35), [scripts/otp12pf_mac_verdict.py:115](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:115)** — BLOCKER 2 remains. `DELTA_REF_MS=230` never participates in a decision. With both measurands `srcinit=2500` and all eight `d_i=230`, the engine reports `VANISHES`: ratio 1.092, CI `[230,230]`, sign `p=.0078`, margin 250. A rig-W-sized effect is present in every pair.

- **HIGH — [scripts/otp12pf_mac_verdict.py:65](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:65)** — The claimed 95% median CI is not trustworthy as such at `n=8`. For distinct samples it resolves to approximately `[d₂,d₇]`, whose exact population-median coverage is 92.97%, not 95%; 10k seeded resamples add no information. The lower percentile uses index 250 rather than nearest-rank index 249. The exact sign-test arithmetic is correct, but no verdict reads its result: 7/8 positives can produce `REPRODUCES` while the registered two-sided sign test says `p=.0703`.

- **HIGH — [scripts/otp12pf_mac_verdict.py:115](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:115)** — The symmetric `±0.10·src` equivalence interval is wrong for the symmetric ratio bar. The negative boundary is `-src/11`, about −9.09%. With `src=2000` and CI `[-190,0]`, the engine says `VANISHES`, although −190 implies an inversion ratio of 1.105. `powered` also tests CI width, not exclusion of the margins, and can record `powered_for_null=no` alongside `VANISHES`.

- **BLOCKER — [scripts/bench_otp12pf_mac.sh:164](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:164), [scripts/bench_otp12pf_mac.sh:177](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:177)** — Preflight cannot succeed as written. For a clean binary, the absent dirty marker makes `grep -c ... || echo X` return `0\nX`, failing the numeric check. Separately, `norm_mac` uses GNU awk’s `strtonum()`; stock macOS `/usr/bin/awk` exits with “undefined function strtonum.”

- **HIGH — [scripts/bench_otp12pf_mac.sh:338](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:338)** — `SETTLE_MS` is outside the fsync timer and excluded from recorded time, but it does not remove the asymmetric gap. It adds 250 ms after the remote arm has already received extra free writeback during the SSH return. The difference still reverses by direction; the code merely assumes 250 ms saturates it.

- **HIGH — [scripts/bench_otp12pf_mac.sh:179](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:179), [scripts/bench_otp12pf_mac.sh:196](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:196), [scripts/bench_otp12pf_mac.sh:314](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:314)** — Several gates still fail open: `pgrep` errors are indistinguishable from quiet; failed `top` becomes 0%, and its last sample overwrites an earlier busy sample; nonnumeric `iostat` becomes zero and can certify drainage. End load is only logged after verdict computation. The known zsh/`printf %q` path round-trips current commands, but both child Bash processes have `pipefail` off, enabling these failures. Time Machine, numeric start-load, and purge failures do now fail closed.

- **HIGH — [scripts/bench_otp12pf_mac.sh:239](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:239), [scripts/bench_otp12pf_mac.sh:295](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:295)** — A stale-daemon `pgrep` error passes; startup then captures the first daemon PID, while smoke proves only that some blit daemon answers. Teardown still calls a failed SSH/`ps` probe “GONE,” and cleanup discards a positively detected survivor.

- **HIGH — [scripts/bench_otp12pf_mac.sh:169](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:169), [scripts/bench_otp12pf_mac.sh:254](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:254), [scripts/bench_otp12pf_mac.sh:435](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:435)** — Provenance remains fail-open. `die` inside `$(sha256_of ...)` exits only that subshell; outer `echo` succeeds with an empty hash. The harness hash is recorded but never compared with reviewed content, dirty worktrees are accepted, `EXPECT_SHA` is not fixed to `f35702a`, and the separately executable verdict engine is not hashed.

- **BLOCKER — [PREREGISTRATION.md:44](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:44), [scripts/bench_otp12pf_mac.sh:16](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:16)** — BLOCKER 3 is only partly fixed. The null and generated verdict text are pair-scoped, but “not platform residue” and “every code-level hypothesis strengthens” still exclude live macOS/APFS and host×role explanations. The design’s line 50 also implies this run decides whether P1 may be accepted as residue, contradicting the parent plan’s no-escape rule.

- **HIGH — [scripts/otp12pf_mac_verdict_test.py:57](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:57)** — The guard is non-vacuous for the exact old range defect and would catch the current bounds being swapped. It only requires the counterexample to be “not VANISHES,” however, so other wrong outcomes pass; dropping/corrupting the sign test changes nothing. It does not guard RIG-VOID, missing cells, reference-effect power, UNSTABLE, or precedence.

- **MEDIUM — [scripts/bench_otp12pf_mac.sh:243](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:243), [scripts/bench_otp12pf_mac.sh:336](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:336)** — The landed paths are correct for both slash-semantic arms, and missing/uncaptured fsync output becomes `NA 0 0`. But count plus aggregate bytes is not exact tree identity: different relative paths or per-file size distributions can match both totals, and `find`/`os.walk` traversal errors are suppressed. That can preserve precisely the mixed-fixture directional confound.

**VERDICT: NOT READY — DO NOT RUN THE RIG.**
tokens used
144,505
## Findings

- **BLOCKER — [scripts/bench_otp12pf_mac.sh:367](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:367)** — The transfer timer regressed to subtracting `monotonic()` values from two Python processes. On this Mac’s Python 3.9, a 1,000 ms delay measured as 1 ms. Valid rows therefore contain roughly fsync time alone; negative clock noise is selectively voided. This can manufacture or mask either arm effect.

- **BLOCKER — [scripts/bench_otp12pf_mac.sh:225](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:225), [scripts/otp12pf_mac_verdict.py:172](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:172)** — BLOCKER 1 is not fixed. `CELLS` may omit controls or one measurand; absent cells are filtered rather than `INCOMPLETE`. A one-cell run can emit `VANISHES` while claiming “both” cells vanished. Additionally, a control with bar `FAIL` but outcome `INCONCLUSIVE` is excluded from RIG-VOID at line 175, and UNSTABLE controls are ignored at line 192.

- **BLOCKER — [scripts/otp12pf_mac_verdict.py:35](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:35), [scripts/otp12pf_mac_verdict.py:115](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:115)** — BLOCKER 2 remains. `DELTA_REF_MS=230` never participates in a decision. With both measurands `srcinit=2500` and all eight `d_i=230`, the engine reports `VANISHES`: ratio 1.092, CI `[230,230]`, sign `p=.0078`, margin 250. A rig-W-sized effect is present in every pair.

- **HIGH — [scripts/otp12pf_mac_verdict.py:65](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:65)** — The claimed 95% median CI is not trustworthy as such at `n=8`. For distinct samples it resolves to approximately `[d₂,d₇]`, whose exact population-median coverage is 92.97%, not 95%; 10k seeded resamples add no information. The lower percentile uses index 250 rather than nearest-rank index 249. The exact sign-test arithmetic is correct, but no verdict reads its result: 7/8 positives can produce `REPRODUCES` while the registered two-sided sign test says `p=.0703`.

- **HIGH — [scripts/otp12pf_mac_verdict.py:115](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:115)** — The symmetric `±0.10·src` equivalence interval is wrong for the symmetric ratio bar. The negative boundary is `-src/11`, about −9.09%. With `src=2000` and CI `[-190,0]`, the engine says `VANISHES`, although −190 implies an inversion ratio of 1.105. `powered` also tests CI width, not exclusion of the margins, and can record `powered_for_null=no` alongside `VANISHES`.

- **BLOCKER — [scripts/bench_otp12pf_mac.sh:164](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:164), [scripts/bench_otp12pf_mac.sh:177](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:177)** — Preflight cannot succeed as written. For a clean binary, the absent dirty marker makes `grep -c ... || echo X` return `0\nX`, failing the numeric check. Separately, `norm_mac` uses GNU awk’s `strtonum()`; stock macOS `/usr/bin/awk` exits with “undefined function strtonum.”

- **HIGH — [scripts/bench_otp12pf_mac.sh:338](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:338)** — `SETTLE_MS` is outside the fsync timer and excluded from recorded time, but it does not remove the asymmetric gap. It adds 250 ms after the remote arm has already received extra free writeback during the SSH return. The difference still reverses by direction; the code merely assumes 250 ms saturates it.

- **HIGH — [scripts/bench_otp12pf_mac.sh:179](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:179), [scripts/bench_otp12pf_mac.sh:196](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:196), [scripts/bench_otp12pf_mac.sh:314](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:314)** — Several gates still fail open: `pgrep` errors are indistinguishable from quiet; failed `top` becomes 0%, and its last sample overwrites an earlier busy sample; nonnumeric `iostat` becomes zero and can certify drainage. End load is only logged after verdict computation. The known zsh/`printf %q` path round-trips current commands, but both child Bash processes have `pipefail` off, enabling these failures. Time Machine, numeric start-load, and purge failures do now fail closed.

- **HIGH — [scripts/bench_otp12pf_mac.sh:239](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:239), [scripts/bench_otp12pf_mac.sh:295](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:295)** — A stale-daemon `pgrep` error passes; startup then captures the first daemon PID, while smoke proves only that some blit daemon answers. Teardown still calls a failed SSH/`ps` probe “GONE,” and cleanup discards a positively detected survivor.

- **HIGH — [scripts/bench_otp12pf_mac.sh:169](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:169), [scripts/bench_otp12pf_mac.sh:254](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:254), [scripts/bench_otp12pf_mac.sh:435](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:435)** — Provenance remains fail-open. `die` inside `$(sha256_of ...)` exits only that subshell; outer `echo` succeeds with an empty hash. The harness hash is recorded but never compared with reviewed content, dirty worktrees are accepted, `EXPECT_SHA` is not fixed to `f35702a`, and the separately executable verdict engine is not hashed.

- **BLOCKER — [PREREGISTRATION.md:44](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:44), [scripts/bench_otp12pf_mac.sh:16](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:16)** — BLOCKER 3 is only partly fixed. The null and generated verdict text are pair-scoped, but “not platform residue” and “every code-level hypothesis strengthens” still exclude live macOS/APFS and host×role explanations. The design’s line 50 also implies this run decides whether P1 may be accepted as residue, contradicting the parent plan’s no-escape rule.

- **HIGH — [scripts/otp12pf_mac_verdict_test.py:57](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict_test.py:57)** — The guard is non-vacuous for the exact old range defect and would catch the current bounds being swapped. It only requires the counterexample to be “not VANISHES,” however, so other wrong outcomes pass; dropping/corrupting the sign test changes nothing. It does not guard RIG-VOID, missing cells, reference-effect power, UNSTABLE, or precedence.

- **MEDIUM — [scripts/bench_otp12pf_mac.sh:243](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:243), [scripts/bench_otp12pf_mac.sh:336](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:336)** — The landed paths are correct for both slash-semantic arms, and missing/uncaptured fsync output becomes `NA 0 0`. But count plus aggregate bytes is not exact tree identity: different relative paths or per-file size distributions can match both totals, and `find`/`os.walk` traversal errors are suppressed. That can preserve precisely the mixed-fixture directional confound.

**VERDICT: NOT READY — DO NOT RUN THE RIG.**
