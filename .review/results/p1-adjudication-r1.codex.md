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
session id: 019f62cc-00cd-7412-b622-6ed3d8094fd4
--------
user
You are one of two independent reviewers. I will compare your answer against the other's; if you disagree, I will show you each other's reasoning and ask again, for as many rounds as it takes. Reason from the recorded data. Do NOT defer to me, to the other reviewer, or to the prior reviews cited below — several of those prior reviews may be wrong, which is part of what you are adjudicating.

THE FINDING (call it P1)
On a macOS<->Windows pair, over TCP, on a "mixed" fixture (5001 files, 547 MB), a blit file
transfer is ~25-38% slower when the DESTINATION machine initiates the transfer than when the
SOURCE machine does. Same code, same bytes, same direction of data flow — only which end typed
the command changes. Measured on two different Mac/Windows pairs, four times: ratios 1.237,
1.300, 1.385, 1.362. The project's release criterion is that no transfer may vary by more than
10% based on who initiated it, so P1 blocks the release.

Also known: Linux<->Linux shows NO such effect (8/8 cells pass). macOS<->macOS has never been
tested. MTU has been excluded as a cause (a 256-run controlled experiment).

THE DISPUTE YOU ARE ADJUDICATING
A prior review concluded that the P1 numbers may be a MEASUREMENT ARTIFACT and must be
re-measured before anything else is done. Its argument:

  The harness that measured P1 (scripts/bench_otp12_win.sh) stops its wall-clock timer when the
  transfer process exits, then immediately walks the destination tree calling fsync() on every
  file and ADDS that flush time to the total. There is no "settle" pause between the two. In
  the gap between the client exiting and the fsync starting, the OS writes back dirty pages
  "for free." That gap can differ between the two arms of the comparison (one arm runs the
  client locally, the other over ssh on the far host). If the gap is direction-asymmetric, it
  could MANUFACTURE a one-directional timing difference that looks exactly like P1. A newer
  harness (scripts/bench_otp12pf_mac.sh) adds an equal settle on both arms to neutralize this.
  Conclusion of the prior review: the old P1 numbers are suspect; re-measure P1 on the
  Mac<->Windows pair with the corrected method before drawing any conclusion.

THE QUESTION
Given the DATA THAT WAS ACTUALLY RECORDED, is P1 a real effect, or is it plausibly (wholly or
partly) this free-writeback timing artifact? Does the settle concern actually invalidate the
recorded measurements? Decide from the numbers, not the abstract argument.

THE DATA — read it directly, run awk/python over it:
- docs/bench/otp12-q-baseline-2026-07-13/  — the session where P1 measured 1.385. The per-run
  CSV records, for every cell and BOTH arms, the total ms AND a separate flush_ms column. Cells:
  wm_tcp_mixed (the P1 cell), wm_grpc_mixed (SAME fixture, different carrier), wm_tcp_large
  (different fixture), mw_tcp_mixed (opposite direction). A README summarizes.
- docs/bench/otp12-perf-2026-07-13/ — the Linux<->Linux session (no P1).
- scripts/bench_otp12_win.sh — the harness that took the P1 numbers (the "flawed" method).
- scripts/bench_otp12pf_mac.sh — the newer harness (the "corrected" method).
- docs/plan/OTP12_PERF_FINDINGS.md — the parent plan (defines P1/P2).
- .review/results/macmac-r12.codex-design.md and .review/results/macmac-r12.grok-design.md —
  the prior reviews that recommended re-measuring. Engage with their argument; do not assume it
  is correct.

THE ARTIFACT HYPOTHESIS MAKES TESTABLE PREDICTIONS. Check them against the recorded data and
report what you find — either direction is a valid, useful answer:
- If the free-writeback gap is producing P1, the flush_ms component should be ASYMMETRIC
  between the two arms of the P1 cell (the arm with more free writeback should show a smaller
  fsync). Is it? By how much, versus the size of the effect?
- Is the P1 effect located in the transfer time, or in the flush time? (Total minus flush.)
- The artifact operates on pending dirty writeback, which is a function of the FIXTURE. The P1
  cell and the grpc control use the SAME fixture. If a writeback artifact caused P1, what should
  it do to the grpc control? What did it actually do?
- Linux<->Linux uses the same immediate-flush method and shows no P1. Does that support or
  weaken the artifact hypothesis?
Finding ASYMMETRIC flush, or an effect that lives in the flush and shrinks when flush is
removed, or a grpc control that ALSO fails, would SUPPORT the artifact hypothesis. Finding the
opposite would REFUTE it. Report whichever the data shows.

It is entirely acceptable to conclude any of: "P1 is real and the artifact does not explain it";
"the artifact could explain some or all of it, re-measure"; or "the recorded data cannot
distinguish these — here is the cheapest experiment that would." If the prior review was wrong,
say so plainly; if it was right, say so.

Output: a short prose verdict (real effect vs artifact, and your confidence), then the reasoning
with SPECIFIC NUMBERS pulled from the data, then a final line exactly one of:
P1 REAL / P1 ARTIFACT / CANNOT TELL — followed by one sentence on the right next action.
Do not invoke skills.
codex
I’ll audit the raw per-run columns and recompute arm-level medians/decompositions, then test each prediction against the same-fixture gRPC control and Linux session. I’ll also verify exactly how both harnesses place the timer, settle, and flush.
exec
/bin/zsh -lc "pwd; rg --files -g 'AGENTS.md' -g '.agents/repo-guidance.md' -g '.agents/state.md' -g 'docs/bench/otp12-q-baseline-2026-07-13/**' -g 'docs/bench/otp12-perf-2026-07-13/**' -g 'scripts/bench_otp12_win.sh' -g 'scripts/bench_otp12pf_mac.sh' -g 'docs/plan/OTP12_PERF_FINDINGS.md' -g '.review/results/macmac-r12.*-design.md'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/Users/michael/Dev/blit_v2
AGENTS.md
scripts/bench_otp12_win.sh
scripts/bench_otp12pf_mac.sh
docs/plan/OTP12_PERF_FINDINGS.md
docs/bench/otp12-perf-2026-07-13/meta.csv
docs/bench/otp12-perf-2026-07-13/verdicts.csv
docs/bench/otp12-perf-2026-07-13/rerun-8pair/meta.csv
docs/bench/otp12-perf-2026-07-13/rerun-8pair/verdicts.csv
docs/bench/otp12-perf-2026-07-13/rerun-8pair/drain-outcomes.txt
docs/bench/otp12-perf-2026-07-13/rerun-8pair/staging-manifest.txt
docs/bench/otp12-perf-2026-07-13/rerun-8pair/runs.csv
docs/bench/otp12-perf-2026-07-13/rerun-8pair/summary.csv
docs/bench/otp12-perf-2026-07-13/drain-outcomes.txt
docs/bench/otp12-perf-2026-07-13/README.md
docs/bench/otp12-perf-2026-07-13/staging-manifest.txt
docs/bench/otp12-perf-2026-07-13/runs.csv
docs/bench/otp12-perf-2026-07-13/summary.csv
docs/bench/otp12-q-baseline-2026-07-13/meta.csv
docs/bench/otp12-q-baseline-2026-07-13/verdicts.csv
docs/bench/otp12-q-baseline-2026-07-13/README.md
docs/bench/otp12-q-baseline-2026-07-13/staging-manifest.txt
docs/bench/otp12-q-baseline-2026-07-13/runs.csv
docs/bench/otp12-q-baseline-2026-07-13/summary.csv

exec
/bin/zsh -lc "sed -n '1,260p' AGENTS.md; sed -n '1,300p' .agents/repo-guidance.md; sed -n '1,260p' .agents/state.md" in /Users/michael/Dev/blit_v2
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
/bin/zsh -lc "sed -n '1,240p' docs/bench/otp12-perf-2026-07-13/README.md; sed -n '1,80p' docs/bench/otp12-perf-2026-07-13/runs.csv; sed -n '1,120p' docs/bench/otp12-perf-2026-07-13/summary.csv; sed -n '1,100p' docs/bench/otp12-perf-2026-07-13/rerun-8pair/runs.csv; sed -n '1,120p' docs/bench/otp12-perf-2026-07-13/rerun-8pair/summary.csv" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "sed -n '1,240p' docs/bench/otp12-q-baseline-2026-07-13/README.md; sed -n '1,80p' docs/bench/otp12-q-baseline-2026-07-13/runs.csv; sed -n '1,120p' docs/bench/otp12-q-baseline-2026-07-13/summary.csv; sed -n '1,120p' docs/bench/otp12-q-baseline-2026-07-13/verdicts.csv" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# otp-12pf — the same-OS invariance rig (magneto↔skippy, 2026-07-13)

**Status**: Recorded. **This README declares nothing** — acceptance is the
owner's at otp-13. **Result: P1 does NOT reproduce with Linux on both
ends.**

**⚠ SUPERSEDES an earlier version of this file** which claimed the exact
opposite ("P1 reproduces at 1.78 → the confound breaks toward CODE"). That
claim was **WRONG and is RETRACTED**: it rested on a scratch probe whose
durability accounting was broken (below). The corrected, methodology-clean
session refutes it. The retraction is kept visible rather than quietly
overwritten — the wrong number was reported to the owner and briefly drove
the plan.

## Why this rig exists

P1 (destination-initiated TCP mixed pays ~25–30%) was only ever measured on
rig W (Mac↔Windows). On a **two-host** rig, host identity IS role: in the
slow arm the destination is the Mac (which dials) AND the source is Windows
(which accepts) — inseparable. So P1 was equally consistent with:

1. **code** — the destination-initiator layout is genuinely slow (H1/H5/H6), or
2. **platform residue** — a macOS/Windows write-path or TCP-stack artifact,
   which D-2026-07-12-1 lets the owner accept as satisfied.

This rig removes every platform term: **Linux on both ends**. Endpoints need
not match each other — an invariance comparison holds both endpoints fixed
and varies only the initiator, so endpoint asymmetry cancels *within* each
pair (`ONE_TRANSFER_PATH.md` criterion 1). What zoey failed was the
absolute-speed floor; magneto clears it (owner: "fast enough to saturate
10 GbE where zoey is definitely not").

## Rig

| host | CPU | disk | link |
|---|---|---|---|
| **skippy** | AMD EPYC (32 threads) | ZFS `generic-pool` | 10 GbE (10.1.10.143) |
| **magneto** | Intel, power-efficient (4c) | WD SN850 NVMe (Gen4) | 10 GbE (10.1.10.10) |

Harness `scripts/bench_otp12pf_linux.sh` @ `2c0af86`; binaries `+f35702a`
x86_64-musl both ends (same-build handshake, D-2026-07-05-2). Session
`20260713T134444`: cold caches on BOTH ends every run (`drop_caches` via the
exact NOPASSWD grant — a failed drop voids the pair), destination drained to
quiet, fresh never-seen destination per run, ABBA interleave, pair-void,
RUNS=4, exit codes gated. **64 timed runs, 8/8 cells complete, ZERO voided
pairs.**

## Result — P1 does NOT reproduce

Invariance bar `max/min ≤ 1.10`. `srcinit` = the SOURCE host's CLI pushes;
`destinit` = the DESTINATION host's CLI pulls. **8 / 8 PASS** (7 at RUNS=4;
`ms_grpc_mixed` via its pre-registered RUNS=8 escalation, below).

| cell | srcinit | destinit | ratio | outcome |
|---|---|---|---|---|
| **`sm_tcp_mixed`** — P1's cell | 1745 | 1905 | **1.092** | **PASS** |
| **`ms_tcp_mixed`** — P1's cell | 2085 | 2079 | **1.003** | **PASS** |
| `sm_tcp_large` | 2595 | 2530 | 1.026 | PASS |
| `ms_tcp_large` | 4584 | 5029 | 1.097 | PASS |
| `sm_tcp_small` | 820 | 870 | 1.061 | PASS |
| `ms_tcp_small` | 2135 | 2114 | 1.010 | PASS |
| `sm_grpc_mixed` (carrier control) | 2390 | 2325 | 1.028 | PASS |
| `ms_grpc_mixed` (carrier control) | 4139 | 2974 | 1.392 | **FAIL** → escalated |

**TCP × mixed × destination-initiated — the exact P1 cell — passes at 1.092
and 1.003.** There is no 25–30% destination-initiator penalty with Linux on
both ends.

`ms_grpc_mixed` failed at 1.392 with the **source**-initiated arm slow (the
opposite direction from P1) on spreads of 25.1% / 36.9% — which trips D2's
pre-registered escalation trigger (straddle + >25% arm spread). It reran at
RUNS=8 (`rerun-8pair/`, 16 runs, 0 voided) and, per D2's supersession
amendment, **the RUNS=8 medians govern**:

```
ms_grpc_mixed,invariance,srcinit,destinit,3435,3230,1.063,1.10,PASS
```

So the cell **PASSES at 1.063**; the 1.392 was low-n noise. Spreads stay
high (48.2% / 61.2%) — that cell is simply noisy on this rig, and the
RUNS=4 rows remain committed and visible.

**Governing outcome: 8/8 PASS.**

## Reading (numbers only; no adjudication)

- **P1 requires the Mac↔Windows pairing.** It does not appear when the
  platform terms are removed. The confound is broken — **toward platform,
  not toward code** (the reverse of this file's retracted first version).
  D-2026-07-12-1's platform-residue discriminator is therefore the relevant
  frame for P1 at the otp-13 walk.
- **This does NOT fully exonerate the code.** It rules out a *pure layout*
  property (which would have shown up here), but a code path whose cost only
  becomes material under a specific platform — e.g. a slow accept path on the
  Windows side, which H1 accuses — would look exactly like this. It narrows
  the hypothesis space; it does not close it. H1's dial/accept inversion
  counterfactual on rig W remains the way to finish the job.
- **P2 is NOT tested here.** P2 is a converge bar (new vs OLD push), and this
  rig has no `0f922de` build staged. Nothing in this session speaks to it.

## The bug that produced the retracted claim

The first revision of the harness (and the scratch probe before it) ran the
durability `sync` **inside the initiating host's timed bracket**:

- **push arm**: initiator = the SOURCE, which only READ. Its `sync` was a
  no-op; the destination's writeback was **never paid**.
- **pull arm**: initiator = the DESTINATION, which had just written the
  bytes. It paid the **full writeback**.

One arm was charged for durability the other got for free — multi-second on
skippy's ZFS at 1 GiB. That manufactured invariance "failures" across every
carrier and fixture, worst on the largest files (`ms_tcp_large` 3.285), and
crucially **including the gRPC carrier control** (`sm_grpc_mixed` 1.400)
which is supposed to be clean. **The carrier-independence is what exposed
it**: a real code effect is carrier-specific; an accounting artifact is not.

Fix (`2c0af86`): the transfer window is bracketed on the initiator with **no
sync**; the destination-side sync is then self-timed **on the destination
host** and added to **both** arms identically. A failed sync yields `NA` and
voids the run. `flush_ms` is its own `runs.csv` column so the accounting is
auditable. This is the otp-2w rule — *durability keyed by DESTINATION, never
by verb* — which this harness broke and has re-learned.

Buggy-session numbers are NOT committed as evidence; they exist only in the
retraction note above and in `logs/otp12pf_linux_20260713T133110/`
(untracked).

## Files

`runs.csv` (incl. `flush_ms`), `summary.csv`, `verdicts.csv`, `meta.csv`,
`staging-manifest.txt`, `drain-outcomes.txt`; the RUNS=8 `ms_grpc_mixed`
escalation under `rerun-8pair/`.
cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid
sm_tcp_mixed,srcinit,f35702a,skippy,1,1750,760,0,drained_3x2s,cold,yes
sm_tcp_mixed,destinit,f35702a,magneto,1,1890,780,0,drained_3x2s,cold,yes
sm_tcp_mixed,srcinit,f35702a,skippy,2,1750,800,0,drained_3x2s,cold,yes
sm_tcp_mixed,destinit,f35702a,magneto,2,1840,780,0,drained_3x2s,cold,yes
sm_tcp_mixed,srcinit,f35702a,skippy,3,1710,750,0,drained_3x2s,cold,yes
sm_tcp_mixed,destinit,f35702a,magneto,3,1920,760,0,drained_3x2s,cold,yes
sm_tcp_mixed,srcinit,f35702a,skippy,4,1741,810,0,drained_3x2s,cold,yes
sm_tcp_mixed,destinit,f35702a,magneto,4,2040,820,0,drained_3x2s,cold,yes
ms_tcp_mixed,srcinit,f35702a,magneto,1,2070,590,0,drained_4x2s,cold,yes
ms_tcp_mixed,destinit,f35702a,skippy,1,2120,480,0,drained_6x2s,cold,yes
ms_tcp_mixed,srcinit,f35702a,magneto,2,1991,451,0,drained_6x2s,cold,yes
ms_tcp_mixed,destinit,f35702a,skippy,2,2039,510,0,drained_4x2s,cold,yes
ms_tcp_mixed,srcinit,f35702a,magneto,3,2511,771,0,drained_4x2s,cold,yes
ms_tcp_mixed,destinit,f35702a,skippy,3,1959,489,0,drained_6x2s,cold,yes
ms_tcp_mixed,srcinit,f35702a,magneto,4,2101,551,0,drained_6x2s,cold,yes
ms_tcp_mixed,destinit,f35702a,skippy,4,2190,630,0,drained_4x2s,cold,yes
sm_grpc_mixed,srcinit,f35702a,skippy,1,2380,780,0,drained_3x2s,cold,yes
sm_grpc_mixed,destinit,f35702a,magneto,1,2340,780,0,drained_3x2s,cold,yes
sm_grpc_mixed,srcinit,f35702a,skippy,2,2400,760,0,drained_3x2s,cold,yes
sm_grpc_mixed,destinit,f35702a,magneto,2,2330,790,0,drained_3x2s,cold,yes
sm_grpc_mixed,srcinit,f35702a,skippy,3,2490,860,0,drained_3x2s,cold,yes
sm_grpc_mixed,destinit,f35702a,magneto,3,2150,770,0,drained_3x2s,cold,yes
sm_grpc_mixed,srcinit,f35702a,skippy,4,2060,820,0,drained_3x2s,cold,yes
sm_grpc_mixed,destinit,f35702a,magneto,4,2320,790,0,drained_3x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,1,4060,1170,0,drained_5x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,1,2929,530,0,drained_5x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,2,4219,1289,0,drained_6x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,2,2789,450,0,drained_6x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,3,4480,1530,0,drained_8x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,3,3019,649,0,drained_5x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,4,3581,541,0,drained_5x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,4,3819,1470,0,drained_6x2s,cold,yes
sm_tcp_small,srcinit,f35702a,skippy,1,840,100,0,drained_3x2s,cold,yes
sm_tcp_small,destinit,f35702a,magneto,1,860,90,0,drained_3x2s,cold,yes
sm_tcp_small,srcinit,f35702a,skippy,2,830,100,0,drained_3x2s,cold,yes
sm_tcp_small,destinit,f35702a,magneto,2,870,110,0,drained_3x2s,cold,yes
sm_tcp_small,srcinit,f35702a,skippy,3,810,100,0,drained_3x2s,cold,yes
sm_tcp_small,destinit,f35702a,magneto,3,870,110,0,drained_3x2s,cold,yes
sm_tcp_small,srcinit,f35702a,skippy,4,780,90,0,drained_3x2s,cold,yes
sm_tcp_small,destinit,f35702a,magneto,4,870,90,0,drained_3x2s,cold,yes
ms_tcp_small,srcinit,f35702a,magneto,1,2110,300,0,drained_5x2s,cold,yes
ms_tcp_small,destinit,f35702a,skippy,1,2140,250,0,drained_6x2s,cold,yes
ms_tcp_small,srcinit,f35702a,magneto,2,2080,230,0,drained_4x2s,cold,yes
ms_tcp_small,destinit,f35702a,skippy,2,2089,240,0,drained_6x2s,cold,yes
ms_tcp_small,srcinit,f35702a,magneto,3,2161,211,0,drained_6x2s,cold,yes
ms_tcp_small,destinit,f35702a,skippy,3,2611,601,0,drained_6x2s,cold,yes
ms_tcp_small,srcinit,f35702a,magneto,4,2180,180,0,drained_4x2s,cold,yes
ms_tcp_small,destinit,f35702a,skippy,4,2000,190,0,drained_6x2s,cold,yes
sm_tcp_large,srcinit,f35702a,skippy,1,2610,1320,0,drained_3x2s,cold,yes
sm_tcp_large,destinit,f35702a,magneto,1,2580,1300,0,drained_3x2s,cold,yes
sm_tcp_large,srcinit,f35702a,skippy,2,2470,1290,0,drained_3x2s,cold,yes
sm_tcp_large,destinit,f35702a,magneto,2,2460,1310,0,drained_3x2s,cold,yes
sm_tcp_large,srcinit,f35702a,skippy,3,2610,1280,0,drained_3x2s,cold,yes
sm_tcp_large,destinit,f35702a,magneto,3,2480,1280,0,drained_3x2s,cold,yes
sm_tcp_large,srcinit,f35702a,skippy,4,2580,1270,0,drained_3x2s,cold,yes
sm_tcp_large,destinit,f35702a,magneto,4,2610,1280,0,drained_3x2s,cold,yes
ms_tcp_large,srcinit,f35702a,magneto,1,4609,3179,0,drained_4x2s,cold,yes
ms_tcp_large,destinit,f35702a,skippy,1,5019,3620,0,drained_4x2s,cold,yes
ms_tcp_large,srcinit,f35702a,magneto,2,4560,3160,0,drained_4x2s,cold,yes
ms_tcp_large,destinit,f35702a,skippy,2,4819,3399,0,drained_4x2s,cold,yes
ms_tcp_large,srcinit,f35702a,magneto,3,4830,3430,0,drained_4x2s,cold,yes
ms_tcp_large,destinit,f35702a,skippy,3,5160,3760,0,drained_4x2s,cold,yes
ms_tcp_large,srcinit,f35702a,magneto,4,4270,2880,0,drained_4x2s,cold,yes
ms_tcp_large,destinit,f35702a,skippy,4,5039,3630,0,drained_4x2s,cold,yes
cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
ms_grpc_mixed,destinit,2974,3139,2789,36.9,0,4
ms_grpc_mixed,srcinit,4139,4085,3581,25.1,0,4
ms_tcp_large,destinit,5029,5009,4819,7.1,0,4
ms_tcp_large,srcinit,4584,4567,4270,13.1,0,4
ms_tcp_mixed,destinit,2079,2077,1959,11.8,0,4
ms_tcp_mixed,srcinit,2085,2168,1991,26.1,0,4
ms_tcp_small,destinit,2114,2210,2000,30.6,0,4
ms_tcp_small,srcinit,2135,2132,2080,4.8,0,4
sm_grpc_mixed,destinit,2325,2285,2150,8.8,0,4
sm_grpc_mixed,srcinit,2390,2332,2060,20.9,0,4
sm_tcp_large,destinit,2530,2532,2460,6.1,0,4
sm_tcp_large,srcinit,2595,2567,2470,5.7,0,4
sm_tcp_mixed,destinit,1905,1922,1840,10.9,0,4
sm_tcp_mixed,srcinit,1745,1737,1710,2.3,0,4
sm_tcp_small,destinit,870,867,860,1.2,0,4
sm_tcp_small,srcinit,820,815,780,7.7,0,4
cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid
ms_grpc_mixed,srcinit,f35702a,magneto,1,3450,460,0,drained_3x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,1,3811,1491,0,drained_5x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,2,4040,1160,0,drained_5x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,2,3521,1360,0,drained_8x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,3,3390,500,0,drained_5x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,3,4061,1700,0,drained_4x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,4,4360,1610,0,drained_6x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,4,2940,540,0,drained_5x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,5,4490,1610,0,drained_5x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,5,3780,1530,0,drained_8x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,6,3030,190,0,drained_4x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,6,2680,529,0,drained_5x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,7,3420,440,0,drained_6x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,7,2870,520,0,drained_5x2s,cold,yes
ms_grpc_mixed,srcinit,f35702a,magneto,8,3230,270,0,drained_4x2s,cold,yes
ms_grpc_mixed,destinit,f35702a,skippy,8,2519,449,0,drained_6x2s,cold,yes
cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
ms_grpc_mixed,destinit,3230,3272,2519,61.2,0,8
ms_grpc_mixed,srcinit,3435,3676,3030,48.2,0,8

 succeeded in 0ms:
# P1 REPRODUCES ON A SECOND MAC — rig `q` ↔ netwatch-01, MTU 9000 (2026-07-13)

**Status**: Evidence (recorded). **This README declares nothing** — it records
what was measured.

**Why this session exists**: P1 (`wm_tcp_mixed` invariance failure) had **only
ever been measured on one Mac** — nagatha. The live **P1** hypothesis in
`docs/plan/OTP12_PERF_FINDINGS.md` — **H1**, plus H2's residual — assumes P1 is
a property of the **macOS↔Windows pairing**. *That assumption had never been
tested.* This session tests it on a different Mac.

**CORRECTION (2026-07-14, codex review of the pf-0 amendment)**: this README
originally named "H1, H5, H6, H7" here and below. **H5, H6 and H7 are P2
hypotheses, not P1** (see the plan's hypothesis list) — only H1 (and H2 as a
residual) bear on P1. The error propagated to `docs/STATE.md`; both are fixed.
The session's *result* is unaffected.

**Rig (NEW — `q`)**: Apple M4 Mac mini, 16 GB, macOS 26.5.2, arm64. 10GbE =
`en8` (the Aquantia adapter physically moved from nagatha), **10.1.10.54**,
**MTU 9000**, negotiated **MSS 8948**. Peer: netwatch-01 (`10.1.10.177`,
MTU 9000). Harness `scripts/bench_otp12_win.sh` @ `f35702a`, RUNS=8, ABBA,
pair-void, cold caches + drain both ends, destination-keyed durability.
Binaries: `f35702a` both ends (embed-verified), arm64 copied from nagatha.

## Result

| cell | mac_init | win_init | ratio | verdict |
|---|---:|---:|---:|---|
| **`wm_tcp_mixed`** (P1's cell) | **1093** | **789** | **1.385** | **FAIL** |
| `mw_tcp_mixed` (opposite direction) | 1688 | 1618 | 1.043 | PASS |
| `wm_grpc_mixed` (opposite carrier) | 1254 | 1230 | 1.020 | PASS |
| `wm_tcp_large` (opposite fixture) | 909 | 907 | 1.002 | PASS |

**P1 reproduces on a different Mac, harder** (1.385 here; 1.237 and 1.300 on
nagatha — different hardware, so the magnitudes are not comparable, but the
*failure* is).

### The controls ARE the noise model, and they are tight

All three control cells passed at **1.002–1.043** *in the same session, same
rig, same conditions*. So this rig's asymmetry noise is ~**2–4%** — and P1 is
**38.5%**, an order of magnitude outside it. This is a **within-session**
comparison, which is the only kind this project has learned to trust
(`.agents/machines.md`, and the local-rig bi-stability in
`docs/bench/win-local-ab-2026-07-13/`).

### What it establishes

- **P1 is NOT a nagatha artifact.** Different Mac, different CPU and disk (M4
  mini vs nagatha), same adapter → the failure follows the **platform pairing**,
  not the machine. The assumption under **H1** (see the correction above)
  survives its first real test.
- **The signature is unchanged and sharp**: TCP only (gRPC passes at 1.020),
  `mixed` only (`large` passes at 1.002), destination-initiator only (the
  reverse direction passes at 1.043).
- **P1 FAILS AT MTU 9000.** This session ran at jumbo. So **jumbo does not
  dissolve P1** — the premise behind `docs/STATE.md`'s Queue 1a.

### What it does NOT establish

- **It does not quantify MTU's contribution.** "Jumbo doesn't fix it" is not
  "MTU contributes nothing" — MTU could still be a CONFIRMED CONTRIBUTING cause
  on the parent plan's own 20–50% band while P1 still fails its 1.10 bar. That
  requires the matched 1500 arm, which is what the counterbalanced A-B-B-A run
  in `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` (rev 3) exists
  to measure. **This session is NOT that experiment** — it is a single
  condition, with no same-MTU replicate and therefore no session-level noise
  floor.
- **It is not acceptance evidence.** `pf-final` owns acceptance, and its rows
  are pre-registered on the *designated* rig.

## Rig-validity checks (all passed before any timed run)

The link was proven, not assumed — three prior instruments lied tonight:

- `en8` media: **10Gbase-T**, full-duplex, active; MTU 9000.
- Route to netwatch-01 → `en8`, with netwatch-01's **real** MAC in ARP.
  (An earlier `route add -host … -interface en8` created a **black hole**: the
  next hop resolved to `q`'s *own* NIC MAC, 100% packet loss, while
  `route -n get` still reported `interface: en8`. Fixed by promoting the
  10GbE's network *service* above the 1GbE's instead.)
- Client socket source = **10.1.10.54**, MSS = **8948**.
- **Throughput**: `wm_tcp_large` moves 1 GiB in **~908 ms ≈ 1.18 GB/s** —
  saturating 10GbE (a 1GbE link would need ~10 s). Note an ssh-based transfer
  test **cannot** detect a 1GbE fallback here: ssh caps at ~79 MB/s on this path
  regardless (nagatha's known-good 10GbE scores the same 79), so it is an ssh
  test, not a link test.

## Files

`summary.csv`, `verdicts.csv`, `runs.csv` (64 timed runs, 0 voided),
`meta.csv`, `staging-manifest.txt` (7 hashes).
cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid
wm_tcp_large,mac_init,f35702a,mac,1,1270,0,0,drained_8s,yes
wm_tcp_large,win_init,f35702a,win,1,909,0,0,drained_6s,yes
wm_tcp_large,mac_init,f35702a,mac,2,927,0,0,drained_6s,yes
wm_tcp_large,win_init,f35702a,win,2,905,0,0,drained_8s,yes
wm_tcp_large,mac_init,f35702a,mac,3,910,0,0,drained_6s,yes
wm_tcp_large,win_init,f35702a,win,3,905,0,0,drained_6s,yes
wm_tcp_large,mac_init,f35702a,mac,4,908,0,0,drained_8s,yes
wm_tcp_large,win_init,f35702a,win,4,907,0,0,drained_8s,yes
wm_tcp_large,mac_init,f35702a,mac,5,912,0,0,drained_6s,yes
wm_tcp_large,win_init,f35702a,win,5,924,0,0,drained_6s,yes
wm_tcp_large,mac_init,f35702a,mac,6,908,0,0,drained_8s,yes
wm_tcp_large,win_init,f35702a,win,6,908,0,0,drained_6s,yes
wm_tcp_large,mac_init,f35702a,mac,7,909,0,0,drained_6s,yes
wm_tcp_large,win_init,f35702a,win,7,904,0,0,drained_6s,yes
wm_tcp_large,mac_init,f35702a,mac,8,907,0,0,drained_8s,yes
wm_tcp_large,win_init,f35702a,win,8,910,0,0,drained_6s,yes
mw_tcp_mixed,mac_init,f35702a,mac,1,1772,657,0,drained_8s,yes
mw_tcp_mixed,win_init,f35702a,win,1,1652,658,0,drained_6s,yes
mw_tcp_mixed,mac_init,f35702a,mac,2,1761,661,0,drained_8s,yes
mw_tcp_mixed,win_init,f35702a,win,2,1647,656,0,drained_8s,yes
mw_tcp_mixed,mac_init,f35702a,mac,3,1598,652,0,drained_8s,yes
mw_tcp_mixed,win_init,f35702a,win,3,1662,655,0,drained_8s,yes
mw_tcp_mixed,mac_init,f35702a,mac,4,1730,675,0,drained_6s,yes
mw_tcp_mixed,win_init,f35702a,win,4,1615,659,0,drained_6s,yes
mw_tcp_mixed,mac_init,f35702a,mac,5,1598,658,0,drained_8s,yes
mw_tcp_mixed,win_init,f35702a,win,5,1622,648,0,drained_8s,yes
mw_tcp_mixed,mac_init,f35702a,mac,6,1646,670,0,drained_8s,yes
mw_tcp_mixed,win_init,f35702a,win,6,1573,659,0,drained_8s,yes
mw_tcp_mixed,mac_init,f35702a,mac,7,1624,641,0,drained_8s,yes
mw_tcp_mixed,win_init,f35702a,win,7,1598,656,0,drained_6s,yes
mw_tcp_mixed,mac_init,f35702a,mac,8,1809,643,0,drained_6s,yes
mw_tcp_mixed,win_init,f35702a,win,8,1562,650,0,drained_8s,yes
wm_tcp_mixed,mac_init,f35702a,mac,1,1176,72,0,drained_8s,yes
wm_tcp_mixed,win_init,f35702a,win,1,844,73,0,drained_6s,yes
wm_tcp_mixed,mac_init,f35702a,mac,2,1095,73,0,drained_8s,yes
wm_tcp_mixed,win_init,f35702a,win,2,719,72,0,drained_8s,yes
wm_tcp_mixed,mac_init,f35702a,mac,3,1091,71,0,drained_6s,yes
wm_tcp_mixed,win_init,f35702a,win,3,796,74,0,drained_8s,yes
wm_tcp_mixed,mac_init,f35702a,mac,4,1096,73,0,drained_8s,yes
wm_tcp_mixed,win_init,f35702a,win,4,894,74,0,drained_8s,yes
wm_tcp_mixed,mac_init,f35702a,mac,5,1089,72,0,drained_6s,yes
wm_tcp_mixed,win_init,f35702a,win,5,724,74,0,drained_6s,yes
wm_tcp_mixed,mac_init,f35702a,mac,6,1145,64,0,drained_8s,yes
wm_tcp_mixed,win_init,f35702a,win,6,783,64,0,drained_8s,yes
wm_tcp_mixed,mac_init,f35702a,mac,7,1082,62,0,drained_8s,yes
wm_tcp_mixed,win_init,f35702a,win,7,723,73,0,drained_8s,yes
wm_tcp_mixed,mac_init,f35702a,mac,8,1085,73,0,drained_8s,yes
wm_tcp_mixed,win_init,f35702a,win,8,841,71,0,drained_6s,yes
wm_grpc_mixed,mac_init,f35702a,mac,1,1205,76,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,1,1278,72,0,drained_8s,yes
wm_grpc_mixed,mac_init,f35702a,mac,2,1193,66,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,2,1237,70,0,drained_6s,yes
wm_grpc_mixed,mac_init,f35702a,mac,3,1268,64,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,3,1200,63,0,drained_8s,yes
wm_grpc_mixed,mac_init,f35702a,mac,4,1262,65,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,4,1196,62,0,drained_6s,yes
wm_grpc_mixed,mac_init,f35702a,mac,5,1276,66,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,5,1280,72,0,drained_8s,yes
wm_grpc_mixed,mac_init,f35702a,mac,6,1194,72,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,6,1223,71,0,drained_8s,yes
wm_grpc_mixed,mac_init,f35702a,mac,7,1246,74,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,7,1217,71,0,drained_8s,yes
wm_grpc_mixed,mac_init,f35702a,mac,8,1290,74,0,drained_8s,yes
wm_grpc_mixed,win_init,f35702a,win,8,1292,72,0,drained_6s,yes
cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
mw_tcp_mixed,mac_init,1688,1692,1598,13.2,0,8
mw_tcp_mixed,win_init,1618,1616,1562,6.4,0,8
wm_grpc_mixed,mac_init,1254,1241,1193,8.1,0,8
wm_grpc_mixed,win_init,1230,1240,1196,8.0,0,8
wm_tcp_large,mac_init,909,956,907,40.0,0,8
wm_tcp_large,win_init,907,909,904,2.2,0,8
wm_tcp_mixed,mac_init,1093,1107,1082,8.7,0,8
wm_tcp_mixed,win_init,789,790,719,24.3,0,8
comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
mw_tcp_mixed,invariance,mac_init,win_init,1688,1618,1.043,1.10,PASS
mw_tcp_mixed,converge,mac_init,old_session,1688,,,1.10,NO-SAME-SESSION-REF
mw_tcp_mixed,converge,mac_init,old_committed,1688,2288,0.738,1.10,PASS
mw_tcp_mixed,converge,win_init,old_session,1618,,,1.10,NO-SAME-SESSION-REF
mw_tcp_mixed,converge,win_init,old_committed,1618,2288,0.707,1.10,PASS
mw_tcp_mixed,cross,worst_arm,min_old_committed,1688,1284,1.315,1.10,FAIL
wm_grpc_mixed,invariance,mac_init,win_init,1254,1230,1.020,1.10,PASS
wm_grpc_mixed,converge,mac_init,old_session,1254,,,1.10,NO-SAME-SESSION-REF
wm_grpc_mixed,converge,mac_init,old_committed,1254,1408,0.891,1.10,PASS
wm_grpc_mixed,converge,win_init,old_session,1230,,,1.10,NO-SAME-SESSION-REF
wm_grpc_mixed,converge,win_init,old_committed,1230,1408,0.874,1.10,PASS
wm_grpc_mixed,cross,worst_arm,min_old_committed,1254,1408,0.891,1.10,PASS
wm_tcp_large,invariance,mac_init,win_init,909,907,1.002,1.10,PASS
wm_tcp_large,converge,mac_init,old_session,909,,,1.10,NO-SAME-SESSION-REF
wm_tcp_large,converge,mac_init,old_committed,909,1294,0.702,1.10,PASS
wm_tcp_large,converge,win_init,old_session,907,,,1.10,NO-SAME-SESSION-REF
wm_tcp_large,converge,win_init,old_committed,907,1294,0.701,1.10,PASS
wm_tcp_large,cross,worst_arm,min_old_committed,909,1294,0.702,1.10,PASS
wm_tcp_mixed,invariance,mac_init,win_init,1093,789,1.385,1.10,FAIL
wm_tcp_mixed,converge,mac_init,old_session,1093,,,1.10,NO-SAME-SESSION-REF
wm_tcp_mixed,converge,mac_init,old_committed,1093,1284,0.851,1.10,PASS
wm_tcp_mixed,converge,win_init,old_session,789,,,1.10,NO-SAME-SESSION-REF
wm_tcp_mixed,converge,win_init,old_committed,789,1284,0.614,1.10,PASS
wm_tcp_mixed,cross,worst_arm,min_old_committed,1093,1284,0.851,1.10,PASS

codex
The key cell already shows a strong diagnostic: its two flush distributions are essentially identical, while every transfer-minus-flush observation in the slow arm is slower than every observation in the fast arm. I’m checking the wrapper mechanics and controls now so the verdict reflects the actual accounting rather than just the README summaries.
exec
/bin/zsh -lc "rg -n -C 10 'P1|P2|initiator|invariance|artifact|flush|settle' docs/plan/OTP12_PERF_FINDINGS.md" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "sed -n '1,260p' .agents/state.md; sed -n '1,320p' docs/STATE.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-14 (51st handoff — both Macs confirmed ready; DEVLOG backfilled for rounds 7-11)

- **NEXT ACTION — fix ROUND 11's findings (all documented, none started), then re-review, then run.** Instrument at `8997f92`, prereg **rev 10**, rule **rewritten and simplified** (D-2026-07-14-3, owner: "simplify"). **NO DATA HAS EVER BEEN TAKEN.** Read `.review/results/macmac-harness-r11.{codex-engine,codex-harness,grok}.md`.
  **THE ENGINE HAS NO BLOCKERS FOR THE FIRST TIME IN ELEVEN ROUNDS.** Two HIGHs remain in it:
  (a) **`B` can exceed `T`** on a slow measurand — `T` is capped at Δ_ref=230ms, but the controls' permitted bias is a *fraction* (≤5% of the arm), so at src>4600ms the permitted rig bias is larger than the effect threshold, and `T−B` goes negative (a null becomes impossible while `T+B` licenses an effect that is mostly rig). **Proposed fix: `B ≥ T/2` on any measurand ⇒ `CONTROLS-NOT-CLEAN`.**
  (b) **`B` hardens each cell but can make the SESSION verdict easier via `MIXED` precedence** — if B pushes one cell out of `EFFECT`, the `MIXED` branch stops firing and the session reports `REPRODUCES` instead of the inconclusive `MIXED`.
  **THE HARNESS HAS ONE BLOCKER**: **the registered topology is NOT enforced** — NIC/IP/MAC are env-overridable and **the MTU is never checked at all**, so the run could silently go over the 1GbE NIC or at MTU 1500. (pf-0 spent 256 runs on MTU; the rig must prove it is on the fabric it claims.) Plus four HIGHs: `resolve_disk` discards the `df` pipeline status; **both Time Machine gates use `tr -cd '0-9'`, so a malformed `"0%"` reads as "disabled"**; the new per-pair RTT gate is in the code but **not in the pre-registration**; and the drain still discards its producers' statuses.
  - **THE RULE, whole**: per cell, the paired ABBA differences (n is **exactly 8**), their median, one exact ≥95% CI (**at n=8 that IS `[min,max]`, so nothing can be trimmed** — the identity everything leans on), and the full range. One threshold `T = min(src/10, 230ms)`. Four states: **EFFECT** (CI clears +T+B), **INVERTED** (clears −T−B), **NONE** (the **full range** inside ±(T−B)), **UNCLEAR**. `B` = the arm bias the controls could not rule out, **relative to the arm**. **Every control must be NONE at T/2 or NO verdict is read — not a null, and not a reproduction.** The 1.10 bar is reported and takes **no part in inference**; the sign test is reported, not decided on. **No escalation**: a noisy rig is fixed by a quieter rig, not more pairs.
  - **⚠ TO RUN IT THE OWNER MUST CLOSE THEIR CODEX SESSIONS** — nagatha is a bench **END**; the quiescence gate refuses while `codex`/`cargo`/`rustc` runs on **either** Mac. Time Machine OFF on both. Rig = nagatha ↔ `q` over the 10GbE fabric at MTU 9000, build pinned `f35702a`; **host facts (NICs, IPs, MTU) live in `.agents/machines.md` — never restated here.** **Then `pf-1`.**
  - **CODEX'S CONTENT FILTER**: a whole-instrument review request gets **killed** ("flagged for possible cybersecurity risk") and produces **no review at all** — the "find the fail-open protection" framing reads as vulnerability scanning on top of ssh/sudo code. **Split it (engine / harness) and word it as plain measurement-correctness.** A killed run's file must never be mistaken for a review.
- **THE INSTRUMENT IS THE RISK — ~110 findings across TEN reviews of this ONE harness, all accepted, none rejected, and it has still never run.** Three project claims were already retracted to harness bugs. **TWO DEFECT CLASSES recur in EVERY round; the next review must assume both are present.** (1) **"Fixed the branch I was shown, not the class"** — the same materiality bug escaped **four** rounds; a fail-open `pgrep` was fixed in one gate and left in its duplicate; the drain was fixed by VALUE and left failing by STATUS; Spotlight coerced a non-number to 0 exactly as the drain once accepted `"."`. **And a deletion regressed the build pin**: cutting the escalation block out took the adjacent `EXPECT_SHA` check with it, so any sha — including `.dirty` — was accepted. (2) **"A protection that never executes, or cannot fail"** — `SETTLE_MS` **had never run in any revision** (a quoting bug killed the `sleep` and its status was discarded), while the prereg asserted it for three revisions; the ssh-dispatch **bound** was measured once at preflight and never enforced on a run. Earned rules: **verify the instrument before believing the measurement**; **`bash -n` is not an execution**; **a protection that cannot be observed is not a protection**; **a mutation that cannot be killed is not a proof.**
- **⚠ THE MAC↔MAC RIG IS *NOT* AN H1 DISCRIMINATOR — retracted 2026-07-14.** "Reproduces ⇒ H1 dies" was **WRONG**: H1 accuses **blit's own code paths**, not Windows, and that code runs on macOS too — so a reproduction is *consistent with* H1. It answers one thing, scoped to this pair: **can P1 occur WITHOUT a Windows peer?** A reproduction ⇒ P1 is not waivable as "Windows residue" (it does **not** prove a platform-*general* cost, and leaves macOS/APFS and host×role open). A null ⇒ it did not reproduce *on this pair* — consistent with "Windows required", **not proof** of it, and reportable only if the run could have SEEN the effect. Detail: the pre-registration.
- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488 as of `bb28ddd`** — the last commit to touch `crates/`+`proto/`; every commit since is docs/scripts, so the count stands unre-run. SMALL_FILE_CEILING paused (D-2026-07-05-1).
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

- **The Mac↔Mac run is BLOCKED and NOT clearable by an agent** — round 11's
  findings are unfixed (engine 2 HIGH, harness 1 BLOCKER + 4 HIGH) and both
  Macs must be codex-quiet. Basis and detail: NEXT ACTION at the top of this
  file; never restated here (re-verified 2026-07-14 against
  `.review/results/macmac-harness-r11.*` and `git log -- scripts/bench_otp12pf_mac.sh`,
  whose newest commit is still round 10's `8997f92`).
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
  Linux daemon-spawn flakiness; **windows-latest CI has never been
  observed green — check it live, do not record push state here.**
  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.
- *(Resolved 2026-07-12/13: SizeMtime SKIP, `725aa07`, the `./NAME` foot-gun,
  otp-5b-3 cancel, the change-journal premise — all landed; see DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-14 (51st)** — **BOTH MACS CONFIRMED READY (owner); DEVLOG backfilled for rounds
  7–11, closing the 50th's gap. No code, no rig time, no data.** Verified, not assumed: tree
  clean; local = `origin` = `github` (via `ls-remote`, not tracking refs); docs gate OK; **zero
  `blit-daemon` on either Mac**; **TM autobackup = 0 on BOTH** (the gate is fail-closed on
  *enabled*, not merely *running* — so neither Mac backs up until the owner re-enables it).
  **⛔ A READY RIG IS NOT A READY INSTRUMENT** — round 11's harness BLOCKER stands (topology
  unenforced, **MTU never checked**: a run could go over the **1GbE** NIC or at **MTU 1500**,
  and pf-0 spent 256 runs proving MTU moves wall time). **NEXT: fix round 11 → re-review →
  THEN run**; the owner must close `codex` first (the quiescence gate refuses on either Mac).
- **2026-07-14 (50th, `f933097`)** — **`drift`: STATE hygiene.** The handoff log was four rounds
  stale (said round 7 in flight; 7–11 had landed). Created `docs/history/state-archive.md`,
  anchored `Suite 1488 as of bb28ddd`, rig IPs → `.agents/machines.md`. Full: **DEVLOG 21:10Z**.
- **2026-07-14 (49th)** — **Mac↔Mac instrument, rounds 3–6: 48 findings, 48 accepted, NO DATA
  TAKEN; the harness REFUSES a timed run.** Every cycle found a defect capable of a **false
  claim** in a branch the previous fix missed. Full: **DEVLOG 18:45Z**.
- *(48th and earlier pruned to the cap — pf-0/MTU is summarised at the top of this file; full entries in DEVLOG 2026-07-06..14.)*

 succeeded in 0ms:
1-# otp-12 perf findings — investigate + fix before acceptance (design)
2-
3-**Status**: Active
4-**Approved**: D-2026-07-13-1 — owner, 2026-07-13, verbatim:
5-**"one more round with codex on the plan then just write the code and
6-reviewloop slice by slice. that converges faster than plans with no
7-ground truth to test."** The final round ran (round 5, verdict NOT READY,
8:3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
9-counterfactual, F3 the inexecutable decision rule); all three are fixed
10-in this revision, and implementation now proceeds **slice by slice, each
11-through the codex loop** (D-2026-07-04-1 unchanged). A non-converged plan
12-verdict is no longer a gate — the plan's earlier "flip to Active at codex
13-convergence" rule is superseded by D-2026-07-13-1, because rounds 2–5
14-were increasingly finding defects in the *prose* while the plan's central
15:factual claim was settled by *measurement* (the same-OS rig refuted a
16-claim four review rounds had left standing). pf-1 exists to generate
17-ground truth; it starts now.
18-
19:**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
20:assumed by this plan):** P1 has **no escape hatch on the books**.
21-D-2026-07-12-1 waives a cross-direction converge-up miss only for a cell
22:that is *already* invariance-passing; P1 is the invariance failure
23:itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
24-plan pursues) or the owner must **amend acceptance criterion 1** in a new
25-decision. pf-1 proceeds either way: it produces the evidence that
26-decision would rest on.
27-**Created**: 2026-07-12
28-**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
29-say the quiet part: "Unification that slows the fast direction fails
30:review." P1 is a miss of the parent's HEADLINE acceptance criterion
31:(initiator/verb invariance, ±10%) — not a nice-to-have.
32-**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
33-if an investigation slice needs one, it stops and this doc is amended
34-through the loop first.
35-
36-**Sequencing (corrected 2026-07-13).** This doc originally deferred
37-otp-12c/12d/13 outright. In fact **otp-12c RAN on 2026-07-13** under a
38-fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
39-at the cutover sha `f35702a`; `docs/bench/otp12c-{delegated,win}-2026-07-13/`).
40-That does not change this plan's standing, and the rows are not lost
41-work — under `pf-final` they are **pre-fix rows, void for acceptance**,
42-but they serve two real purposes: (a) an **independent replication** of
43-both findings at the shipped sha (below), which is exactly the
44:independent corroboration the round-2 review said P1 lacked; and (b) the
45-pre-pf-1 control the investigation needs. **otp-12d and otp-13 remain
46:deferred** until P1/P2 are fixed or explained at code level — assembling
47:an acceptance matrix out of pre-fix rows would build the artifact otp-13
48-walks from rows this plan declares void.
49-
50-## The two findings (evidence, both committed)
51-
52:**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
53-(`docs/bench/otp12-win-2026-07-12/`, replicated in
54:`docs/bench/otp12c-win-2026-07-13/`). `wm_tcp_mixed` invariance FAILs in
55-**two independent sessions**, and got WORSE at the shipped sha:
56-
57-| session | build | mac_init | win_init | ratio | arm spreads |
58-|---|---|---|---|---|---|
59-| 12b (2026-07-12) | `e21cf84` | 1127 | 911 | **1.237** | 8.2 / 3.3% |
60-| 12c-win (2026-07-13) | `f35702a` (cutover) | 1221 | 939 | **1.300** | 6.4 / 8.4% |
61-
62-Corroborated by block-1 `pull_tcp_mixed` new-vs-old-same-session:
63-**1.313** (12b: 1138/867) and **1.247** (12c-win: 1192/956).
64-
--
68-a different day at a different sha — the round-2 review's objection that
69-the 1.313 corroboration was "same rig/session, not independent" is now
70-answered by an independent session reproducing the same cell.)
71-
72-**What the evidence actually supports — and the confound it does NOT
73-escape** (corrected, review round 3; an earlier draft of this section
74-claimed the `mw` cell was a clean control isolating "destination
75-initiation" as the cause. It is not, and the correction matters because
76-it re-aims the hypotheses):
77-
78:Every invariance cell compares two arms that share the same endpoints
79:and the same data direction, so **within** a cell the initiator is the
80-only variable — that part is clean. Arm medians (12c-win):
81-
82-| cell | data direction | dest-initiated arm | source-initiated arm | ratio | spreads |
83-|---|---|---|---|---|---|
84-| `wm_tcp_mixed` | Win→Mac | 1221 | 939 | **1.300 FAIL** | 6.4 / 8.4% |
85-| `mw_tcp_mixed` | Mac→Win | 1477 | 1415 | 1.044 PASS | 20.8 / 20.5% |
86-
87:The initiator penalty is therefore **real and large in the Win→Mac
88-direction only**. In Mac→Win the two layouts are within noise, and the
89-ordering even **flips between sessions** (12b: dest-initiated 1502 was
90-*faster* than source-initiated 1587), on spreads of 17–25%.
91-
92-Crossing from `wm` to `mw` is **not** a controlled swap of one variable:
93-it also swaps the destination filesystem (APFS vs NTFS), the TCP stack,
94:which host runs the client, and the flush method. So the supported
95:signature is an **interaction — TCP × mixed × Win→Mac × initiator** —
96-not "destination initiation" on its own.
97-
98-Worse, on a two-host rig the failing configuration is **confounded by
99-construction**: in the slow arm the destination is the Mac (which dials)
100-*and* the source is Windows (which accepts). With only two hosts, **host
101-identity IS role** — "Mac-as-dialing-destination" and
102-"Windows-as-accepting-source" are the same configuration and cannot be
103-separated by any number of additional runs on this rig.
104-
105-### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)
106-
107-**Evidence: `docs/bench/otp12-perf-2026-07-13/` — magneto↔skippy, Linux on
108-BOTH ends, real 10 GbE, full otp-12 methodology** (cold caches both ends,
109-destination drained, ABBA, pair-void, RUNS=4; 64 runs, 8/8 cells, zero
110-voided). Harness `scripts/bench_otp12pf_linux.sh`.
111-
112:**P1 does NOT reproduce.** Its own cell passes with room to spare:
113-
114-| cell | srcinit | destinit | ratio | outcome |
115-|---|---|---|---|---|
116:| `sm_tcp_mixed` (P1's cell) | 1745 | 1905 | **1.092** | PASS |
117:| `ms_tcp_mixed` (P1's cell) | 2085 | 2079 | **1.003** | PASS |
118-
119:**8/8 invariance cells PASS** (`ms_grpc_mixed` via its pre-registered
120:RUNS=8 escalation → 1.063). There is no destination-initiator penalty at
121-all when both ends are Linux.
122-
123-Therefore:
124-
125:- **P1 requires the Mac↔Windows pairing.** It is NOT a pure layout
126-  property of blit's code — a pure layout cost would have appeared here,
127-  on the same code, same carrier, same fixture.
128-
129:- **⚠ BUT P1 HAS NO ESCAPE HATCH TODAY (review round 5, BLOCKER).** An
130-  earlier revision of this section said D-2026-07-12-1 lets the owner
131:  accept P1 as a platform residue. **It does not.** That decision excuses
132-  a **cross-direction converge-up** miss for a cell that has ALREADY
133:  satisfied its precondition **"(b) is initiator/verb-invariant within
134:  ±10%"** (`docs/DECISIONS.md` D-2026-07-12-1). **P1 IS the invariance
135-  failure** (`wm_tcp_mixed` 1.300 FAIL) — the precondition it would need
136-  is the very thing it violates. No decision on the books waives it.
137-  Therefore exactly two exits exist, and pf-1 must aim at them:
138:  1. **FIX IT** — P1 ≤ 1.10 on rig W. This remains the default and the
139-     bar (`ONE_TRANSFER_PATH.md` acceptance criterion 1 is mandatory).
140-  2. **A NEW OWNER DECISION amending criterion 1** — for which the
141-     same-OS result is the honest evidence base: criterion 1 asks for
142:     invariance "on a symmetric rig", Mac↔Windows was designated only
143-     because no better pair existed, and one now does — magneto↔skippy,
144-     where blit measures **8/8 invariant**. An owner could reasonably
145-     rule that criterion 1 is judged on the rig that isolates blit's own
146-     behaviour, with the Mac↔Windows delta recorded as platform residue.
147-     **That ruling does not exist. It must not be assumed, and this plan
148-     must not be written as though it will be granted.**
149-- **This does NOT fully exonerate the code.** It rules out a pure layout
150-  property; it does not rule out a code path whose cost only becomes
151-  material under a particular platform — e.g. a slow accept branch on the
152-  Windows side, which is exactly what H1 accuses. H1/H5/H6 stay LIVE but
153-  are now **narrowed to platform-interacting mechanisms**, and only the
154-  dial/accept inversion counterfactual on rig W can finish the job.
155:- **P2 is untested by this rig** (it is a converge bar vs the OLD build,
156-  and no `0f922de` build is staged on these hosts). Nothing here speaks
157-  to it.
158-
159-> **⚠ A RETRACTED CLAIM LIVED HERE.** An earlier revision of this section
160:> asserted the opposite — "P1 reproduces at 1.78 → the confound breaks
161-> toward CODE → the fix is mandatory and cannot be waived" — and STATE and
162-> the acceptance plan were amended to match. That was **WRONG**. It rested
163-> on a scratch probe (and a first harness revision) that ran the durability
164-> `sync` inside the INITIATING host's timed bracket: in the push arm the
165:> initiator is the SOURCE, which only read, so its sync was a no-op and the
166:> destination's writeback was never paid; in the pull arm the initiator IS
167-> the destination, so it paid the full writeback. One arm was charged for
168-> durability the other got free — multi-second on skippy's ZFS — which
169-> manufactured "failures" on every carrier and fixture, **including the
170-> gRPC control that is supposed to be clean**. That carrier-independence is
171-> what exposed it: a real code effect is carrier-specific; an accounting
172:> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
173-> never by verb — the otp-2w rule, re-learned). The retraction is recorded
174-> rather than quietly overwritten because the wrong number was reported to
175-> the owner and briefly drove this plan.
176-
177-### The residual confound (WHICH code) still needs a counterfactual
178-
179-Breaking platform-vs-code does NOT tell us *which* layout property costs
180-the time. On any two-host rig, host identity remains welded to role, so
181-"the accepting end" cannot be separated from "that host" by more runs:
182-
183-- **pf-1 must compare all four rig-W arms** (both cells × both
184:  initiators), not two, and report the interaction — not a single ratio.
185-- **The disambiguator is a dial/accept inversion counterfactual, not a
186-  rig** — but it is **NOT sufficient on its own** (review round 5): the
187-  inversion swaps the source's `Accept`, the destination's `Dial`, AND
188-  the epoch-0 topology **simultaneously**, so a positive result implicates
189-  *the topology pair*, not H1 specifically. It cannot distinguish
190-  source-accept serialization from synchronous destination dialing
191-  (`transfer_session/mod.rs:3113`), nor prove the resize-specific claim.
192-  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
193-  1. **dial/accept inversion** — same direction, same hosts, same fixture;
194-     only who dials changes. Implicates the topology pair (or exonerates it).
--
205-  resizes, or if (3) shows the cost is the synchronous dial rather than the
206-  accept branch. Any of these that changes connection topology — (1) and
207-  (2) do — **trips this plan's Contract stop-and-amend rule**
208-  (`TRANSFER_SESSION.md` amended through the loop BEFORE the flag is
209-  written). Same-build-both-ends (D-2026-07-05-2) means no compatibility
210-  surface is created.
211-  **H1 is also WEAKENED by the Linux null** (it predicts a layout cost that
212-  did not appear on a real-network same-OS pair), so pf-1 must be prepared
213-  to kill it and fall through to H5/H6/H7.
214-- **The same-platform loopback run is a ONE-WAY test** (corrected — an
215:  earlier draft of this section had it backwards). A dest-initiator
216-  penalty that still appears on Mac↔Mac loopback proves **pure layout**
217-  (code). Its ABSENCE proves **nothing**: loopback has no NIC, near-zero
218-  RTT and a huge MTU, so it erases exactly the per-epoch accept/dial
219-  round-trip cost H1 accuses. A negative local result is **INCONCLUSIVE**
220-  and never reads as "no code bug" — it escalates to the inversion
221-  counterfactual and the rig-side instrumented run (Method 2).
222-
223-This refines rather than weakens H1: H1 accuses the **source's accept
224-branch** under resize, and the source in the slow arm is Windows —
225-consistent. But consistency is not confirmation, and the confound above
226-is exactly why pf-1 exists.
227-
228-The rest of the signature is unchanged and sharp:
229-- **carrier**: TCP only — `wm_grpc_mixed` **1.021 PASS** (12b: 1.013);
230-- **fixture**: mixed only — `wm_tcp_large` **1.039** and `wm_tcp_small`
231-  **1.027** both PASS;
232:- **isolation**: in 12c-win, 11 of 12 invariance cells pass at
233-  1.003–1.044. `wm_tcp_mixed` is the sole outlier, by a wide margin.
234-
235-Also present in 12a's data? NOT testable there (review 2026-07-12):
236-zoey's rig anchors converge-up only (12a README), so it has no
237:mac_init/win_init invariance pair; its pull_tcp_mixed 0.966 is a
238:new-vs-old check, not a two-layout measurement. P1 was never measured
239-on zoey — that PASS must not be read as absence or masking evidence.
240-
241:**P2 — unified small-file push pays ~10–20% vs old push, both rigs**,
242-`push_tcp_small` new-vs-old-same-session:
243-
244-| session | build | new | old | ratio |
245-|---|---|---|---|---|
246-| 12a zoey (RUNS=8, tight) | `e757dcc` old arm | — | — | **1.105** |
247-| 12b netwatch-01 (3–4% spreads) | `e21cf84` | 2080 | 1811 | **1.149** |
248-| 12c-win (2026-07-13) | `f35702a` (cutover) | 1975 | 1644 | **1.201** |
249-
250-**gRPC small push did NOT regress** (correction, review round 2: the
251-earlier "win 0.98-ish per cells" was wrong against the committed CSVs;
--
256-|---|---|---|
257-| zoey | **1.001** | 0.907 |
258-| netwatch-01 (12b) | **0.801** | 0.835 |
259-| netwatch-01 (12c-win) | **0.852** | 0.802 |
260-
261-So the cross-rig range is **0.801–1.001**: gRPC small push is at parity
262-on zoey and materially FASTER on Windows. The honest statement is **"TCP
263-regressed while gRPC did not"** — not "gRPC is uniformly faster".
264-
265-That asymmetry is the finding's sharpest constraint on mechanism:
266:whatever P2 is, it is TCP-data-plane-specific, source-initiated, and
267-small-file-heavy (10k×4 KiB). **But it is a constraint, not a proof of
268-innocence** (review round 3): an aggregate gRPC *improvement* cannot
269-exclude a shared regression on both carriers that a larger
270-gRPC-specific gain simply masks. Shared controller/planner/sink code is
271-therefore NOT exonerated by the gRPC numbers, and pf-1 must attribute
272-the TCP gap to a named delta rather than infer "TCP-only ⇒ not shared".
273-
274-Cross-block note (12b README): block-2 `mw_tcp_small` mac_init measured
275-1922 vs block-1 new 2080 in the same session — the only mechanical
276-difference is block-2's precreated destination container and per-arm
277-path shapes; the investigation must confirm or kill that lead. It is a
278-lead, not an attribution (a precreated container is environmental and
279-cannot attribute code — Method 3(a)).
280-
281:## pf-0 — the environmental control (MTU): **KILLED as a material cause of P1** (recorded 2026-07-14)
282-
283-Executed as pre-registered
284-(`docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md`); evidence + full
285-adjudication in that directory's `README.md`. **The decision rule, thresholds
286-and guards were registered in rev 3, before any of the S1–S4 data existed, and
287-were unchanged by rev 4** (rev 4 re-described the *rig* after the `q` baseline —
288-so "written before the data" is true of the rule, not of the whole document, and
289-no threshold was authored around these numbers). Counterbalanced **A-B-B-A**
290-(9000, 1500, 1500, 9000) on rig W with the `q` Mac end, `RUNS=8`, **256 timed
291-runs, 0 voided**, MSS gate held at the start AND end of every session (8948
292-jumbo / 1448 at 1500).
293-
294-    Δ_9000 = 236 ms    Δ_1500 = 229 ms    N_Δ (measured noise floor) = 78 ms
295-    r = (Δ_1500 − Δ_9000) / Δ_1500 = −3.1%   →   KILLED (r < 20%, the scale below)
296-
297-**What this licenses — exactly the registered outcome, and no more.** Raising
298-the MTU **did not improve these cells under the observed packetization**: the
299:point estimate of the MTU contribution to P1 is ~0. The null is **not vacuous**
300-— the manipulation demonstrably reached the wire (`wm_tcp_large` ran **3–4%
301-faster at jumbo on both arms**, and both `wm_tcp_mixed` arms sped up slightly) —
302-and the benefit is **symmetric**, which is why it cannot explain an
303:**asymmetry**. P1 FAILED in all four sessions (1.237–1.362) regardless of MTU;
304-all controls passed in all four.
305-
306-**What it does NOT license (do not restate this result as more than it is).**
307:- **The wire is not exonerated, and "P1 is code-shaped" is NOT established
308-  here.** MTU is *one* environmental variable. Segment **fill** is unmeasured
309-  (8948 is the MSS *ceiling*), so underfilled segments, a bottleneck elsewhere,
310-  or a smaller wire contribution are all still live. This result kills **MTU**,
311-  not "the environment".
312-- **It is not powered to exclude a CONTRIBUTING-size MTU effect.** The
313:  CONFIRMED-CONTRIBUTING threshold is 20% of Δ_P1 ≈ **46 ms**, which is
314-  **below the rig's measured between-session noise floor of 78 ms**. So the
315-  experiment can exclude a **DOMINANT** effect (50% ≈ 114 ms, comfortably above
316-  the floor) but **cannot exclude a contributing-size one** — a 46 ms effect
317-  could be swamped. The registered rule returns KILLED on the point estimate,
318-  and that grade stands as registered; the *resolution limit* is stated here so
319-  the grade is never read as a stronger exclusion than the data supports.
320-- It confirms no hypothesis. pf-1 still owns attribution.
321-
322:**`Δ_P1(rig W)` is re-estimated, and the noise floor constrains how pf-1 may
323-grade.** The `282 ms` above is a **single nagatha session**; four sessions on
324:the `q` pairing give **Δ_P1 ≈ 230 ms** (229 at 1500, 236 at 9000).
325-
326-- **Between-session grading of a counterfactual is now definitively ruled out**
327-  on this rig: a 46 ms (20%) recovery is smaller than the 78 ms between-session
328-  floor, so an unpaired before/after across sessions cannot separate
329-  CONTRIBUTING from KILLED.
330-- **This does NOT prove the interleaved design has enough resolution** — that is
331-  a different (paired, within-session) variance, and pf-0 did not measure it.
332-  **pf-1 must measure its own paired within-session noise floor on the
333-  unmodified build and register a resolution check** (its smallest reportable
334-  recovery must exceed that floor) *before* grading any hypothesis. A pf-1
--
370-clean rebuild is not a new reference build). Rig D has no old baseline and is
371-unaffected. The remaining constraints (`BASELINE_SUMMARY` stays override-free and
372-is re-pointed by a reviewed source edit; pf-0's start-AND-end MSS gate applies)
373-live in D-2026-07-14-1 and are not restated here.
374-
375-Same-session references (`old_session`) are MTU-matched by construction and were
376-never at risk.
377-
378-## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
379-
380:- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
381-  connection-initiating end DIALS; byte direction is role-set
382-  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
383-  session the SOURCE is the responder: each sf-2 resize epoch is
384-  ACCEPTED off the source's listener while the DESTINATION dials
385-  (otp-5b-2: `SourceSockets` Dial/Accept branches;
386-  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
387-  accept/dial round-trips or serialization in the accept branch that the
388-  dial branch does not pay.
389-  **⚠ H1 ACCUSES CODE, NOT A PLATFORM (canonical; added 2026-07-14 after the
390-  shorthand misled two sessions).** The word "Windows" appears nowhere above.
391:  Windows is merely *who happens to be the accepting source* in P1's slow arm on
392-  rig W, so other docs say "H1's Windows accept branch" as **shorthand for where
393-  the accused code runs on that rig** — it is NOT a claim that H1 requires
394:  Windows. Two consequences, both load-bearing: (a) **a reproduction of P1 on a
395-  non-Windows pair does NOT kill H1** — the accused code runs there too, so it is
396-  *consistent with* H1 (and "consistent with H1" is not confirmation, below);
397:  (b) **a disappearance of P1 without Windows does not CONFIRM H1** either — it
398-  would only mean the accused cost is platform-conditional, which is a further
399:  claim. Only the dial/accept inversion counterfactual in pf-1 can settle H1.
400-  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
401-  was "mixed exercises resize hardest", but **all three fixtures target
402-  eight streams before clamping** (`src/dial.rs:474`) — so resize
403-  *count* cannot explain mixed-only behaviour, and H1 must name what
404-  about mixed differs (shard-boundary timing? the tar-shard small half
405-  interleaving with the big-file stream at the moment epochs fire?) or
406-  be killed. **H1 also names the wrong half without proof**: it accuses
407-  `Accept` while the destination's **synchronous dial-before-ACK** path
408-  (`transfer_session/mod.rs:3113`) is an equally good suspect. pf-1 must
409-  separate them with the dial/accept inversion counterfactual below —
410-  "consistent with H1" is not confirmation.
411:- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
412-  interleave cannot happen — resize begins only after
413-  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
414-  layouts drain the same fixed 128-entry destination need loop, so
415-  batch emission cannot interleave with the resize controller during
416-  manifest/need emission in either layout. Kept only as a residual: if
417-  pf-1 timing shows a layout-dependent need-batch delta anyway, the
418-  mechanism must be re-derived from the trace, not from this text.
419:- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
420-  already killed its named candidates (the small half is tar-sharded and
421-  written with parallel per-file `create_dir_all`/`fs::write`, NO
422:  per-file flush; per-file progress emission to the served push
423-  destination is disabled — `remote/transfer/sink.rs`; and old push used
424:  the same served sink, so fsync/flush policy and progress emission are
425-  NOT old/new deltas). What was left — "dest-side directory work/handle
426-  churn" — **names no old/new code delta at all**, and its only probe
427-  (precreate-vs-not) is explicitly environmental and cannot attribute
428-  code (Method 3(a)). A hypothesis that cannot be confirmed *or* killed
429-  by pf-1 is not a hypothesis; keeping it would let pf-1 close with a
430-  shrug. It is therefore retired, and its one code-attributable
431-  descendant — a per-member cost on the TCP receive path that old push
432-  did not pay — lives on as **H6**, which names an executed-path delta.
433-  H3 may only be revived if the pf-1 trace names a concrete old/new
434-  delta in the destination directory/handle path; the 12b cross-block
435-  precreated-container lead (8%, NTFS) is recorded as an environmental
436-  lead for that trace, not as an attribution.
437:- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
438-  unchanged since `0f922de` (`remote/transfer/data_plane.rs`; the
439-  earlier `dial.rs` attribution was wrong), and old small push ALSO
440:  opened at one stream (after its 128-file early flush) then resized
441-  live — so neither framing nor "fixed-count opening" discriminates.
442-  What survives of H4 is ramp cadence/shard-boundary timing only, and
443-  it is subordinate to H5.
444:- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
445-  scan/diff/transfer overlap on the TCP plane — current code withholds
446-  every TCP payload until `ManifestComplete`
447-  (`transfer_session/mod.rs`), while old push negotiated and queued
448-  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
449-  gRPC's in-stream carrier did not change comparably — which matches
450-  the exact signature "TCP regressed while gRPC did not" (zoey gRPC at
451-  parity 1.001, Windows gRPC faster; NOT "gRPC uniformly at parity" —
452-  review round 3). NOTE: an H5 fix
453-  reorders session phases and multi-ADD/pipelined epochs conflict with
454-  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
455-  ordering), so any H5 fix triggers this plan's Contract
456-  stop-and-amend rule BEFORE implementation.
457:- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
458-  need-claim locking on the TCP receive plane — TCP receive
459-  (`NeedListSink`) takes a separate mutex/hash-set claim per member
460-  (`transfer_session/data_plane.rs:1167`), while the gRPC path claims
461-  a whole shard under one lock (`transfer_session/mod.rs:3047`).
462:  TCP-only and per-member (so small-file-heavy) — matches the P2
463-  signature independently of H5. Discriminated by the pf-1 per-member
464-  locking timings (Method 3(e), now unconditional).
465-  **Historical control — corrected (review round 3): test the EXECUTED
466-  path, not source presence.** `NeedListSink` *exists* in the tree at
467-  `0f922de`, so "does the symbol exist there" is the wrong question and
468-  would wrongly force H6 into a "multiplied claim frequency" story. What
469-  matters is what old push actually RAN: at `0f922de` the served push
470-  data plane goes `socket → StallGuard → execute_receive_pipeline →
471-  FsTransferSink → disk`
472-  (`crates/blit-daemon/src/service/push/data_plane.rs:185-206`) —
473-  it **bypasses `NeedListSink` entirely** and takes no per-member claim.
474-  So H6's claim is precise and falsifiable: the unified TCP receive path
475-  introduced a per-member lock/hash-set claim on a path whose old
476-  counterpart took none. pf-1 confirms it by (a) reading the executed
477-  old path (done — cited above) and (b) the per-member locking timings;
478-  it is KILLED if those timings do not scale with member count or do not
479:  account for a material share of the P2 gap. If H6 is confirmed, the P2
480-  fix bar applies unchanged (≤ 1.10 against BOTH references, BOTH rigs);
481-  no separate bar is granted.
482-  **H6's WALL-TIME counterfactual (added round 5 — timings alone would
483-  strand pf-1 under the uniform decision rule):** behind a debug flag,
484-  claim the whole tar shard under ONE lock on the TCP receive path —
485-  i.e. give TCP the same batch-claim shape the gRPC path already uses
486-  (`transfer_session/mod.rs:3047`), rather than a per-member claim
487-  (`data_plane.rs:1167`). This is safe and wire-neutral (it changes only
488-  the granularity of a local mutex/hash-set claim, not any frame), so it
489:  does NOT trip the Contract rule. Grade its recovery against `Δ_P2` on
490-  the uniform scale. If per-member claiming is the cost, batch-claiming
491-  recovers it; if not, H6 dies with a number rather than a shrug.
492-
493:- **H7 (P2; added by review round 4 — the SHARED-controller candidate
494-  the gRPC caveat predicted)**: HEAD's need/manifest bookkeeping is
495-  heavier than old push's per entry. The unified source keeps a
496-  **mutex-protected sent-manifest map** with per-entry insertion and
497-  removal, and routes each need through a **per-need event-channel hop**
498-  (`transfer_session/mod.rs:1038`, `:1123`, `:1350`); old push used a
499-  **task-local map and handled need batches inline**, with no lock and no
500-  channel hop per entry. This is **per-entry**, so it scales with FILE
501:  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
502-  **shared by BOTH carriers**. That is the precise class the round-3
503-  gRPC caveat warned about: a shared regression can hide under gRPC's
504-  larger carrier-specific gain, so "TCP-only symptom" does NOT exonerate
505-  shared code. No prior hypothesis tested it. Discriminated by: per-entry
506-  bookkeeping timings scaled against file count, plus the wall-time
507-  counterfactual (a task-local/batch-inline path behind a debug flag).
508-  H7 and H6 are independent and may BOTH contribute.
509-
510-## Method (the investigation slice — no behavior changes)
511-
512-1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
513-   in-process/two-process rigs on the Mac with the otp-2 fixture
514-   shapes; `--trace-data-plane` + targeted `tracing` spans (added
515-   behind a debug flag, kept) around: resize epochs (arm→accept/dial→
516-   ack), need-batch emission times, per-file sink open/write/close in
517-   the receive path, shard planner in/out timestamps.
518-2. **A/B the role layouts in one process**: the role suite already
519:   runs both initiator layouts over identical fixtures (otp-3) — but
520-   it forces the in-stream carrier (`transfer_session_roles.rs`), so
521-   the timing-harness variant MUST add a TCP-carrier mode; it reports
522-   phase timings per layout for mixed and small fixtures. A positive
523-   layout-dependent delta in a named phase confirms; local ABSENCE
524-   does not kill H1 (loopback removes the Windows↔Mac topology). So
525-   that H1 stays falsifiable: if the local run is negative, pf-1
526-   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
527-   CELLS fixtures) before pf-1 may close — every hypothesis exits
528-   pf-1 confirmed or killed, never "unfalsified" (review round 2).
529:3. **Historical control, then bisect P2**: old push is deleted from
530-   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
531-   build and run; the control is an old-vs-new run on identical
532-   fixtures. The new tracing spans do NOT exist in `0f922de` (review
533-   round 2), so the control is observed externally — phase boundaries
534-   from wire + filesystem timestamps and stdout progress, with event
535-   semantics mapped span-for-span to the new names — or, where that is
536-   too coarse, a minimal probe backport onto the pinned `0f922de`
537-   source with identical event names. Either way every timed
538-   configuration runs an instrumentation-on/off pair to bound observer
539-   overhead (per-member tracing across ~10k files can perturb a
540-   double-digit share of the measured gap). Experiments, corrected per
541-   review 2026-07-12: (a) precreate-vs-not stays but is
542:   environmental-only (it cannot attribute code); (b) the flush/
543-   instrument toggles missed the tar-shard path — instrument the
544-   tar-shard write path itself; (c) REPLACED (review round 2) — the
545-   ramp pin discriminated nothing (old push also opened at one
546-   stream), but H4 keeps a code-level counterfactual: a batch-cadence
547-   replay toggle that processes need batches at the recorded old-push
548-   shard-boundary cadence; (d) NEW, for H5 — the overlap experiment,
549-   metric DEFINED (review round 2: "manifest-complete→first-payload
550-   gap" was underdefined, and for old push the quantity is expected to
551-   be NEGATIVE, which an unsigned "gap" cannot express). Record, per
552-   run, on ONE common clock with a SIGNED offset from the
--
563-   `0f922de` control for the SAME fixture — i.e. old push really did put
564-   TCP bytes on the wire before its manifest completed, and the new
565-   session does not.
566-   **That timestamp proves ORDERING, not CAUSATION, so it cannot confirm
567-   H5 (review round 3).** H5 is confirmed only by a causal
568-   counterfactual: a debug-flag toggle that restores mid-manifest TCP
569-   payload queueing (queueing/ordering only — if it cannot be done
570-   without a wire change, this plan's Contract stop-and-amend rule fires
571-   FIRST) and measures WALL TIME on the same fixture and rig,
572-   interleaved old-vs-new. Pre-registered: H5 is CONFIRMED iff the
573:   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
574-   KILLED if it restores the old ordering but does not move wall time —
575-   which would prove the lost overlap is real and irrelevant, and hand
576:   P2 to H6;
577-   (e) per-member locking/framing timings are now an unconditional pf-1
578-   measurement (they discriminate H6), not contingent on the trace
579-   implicating them.
580:4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
581:   local rig is Mac↔Mac loopback: it removes the very platform terms P1
582:   is confounded with, and it may equally fail to surface P2 (whose
583-   Windows arms are the sharpest). So the rule is symmetric — **if a
584-   finding does not reproduce locally, pf-1 REQUIRES the rig-side
585:   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
586:   since P2 was measured on both) with the same spans and the CELLS
587-   fixtures, before pf-1 may close. Every hypothesis exits pf-1
588-   confirmed or killed — never "did not reproduce, moving on".
589-5. Every experiment lands as a committed probe record under
590-   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
591-   loop per slice as usual.
592-
593-## pf-1 decision rule — UNIFORM, pre-registered (added round 5)
594-
595-Round-4 review: individual hypotheses had no shared decision threshold —
596-H1 accepted any positive phase delta, H4's cadence replay had no
597-threshold, H5 left a 1–49% recovery undecided, H6 left "material share"
598-undefined. A phase-timing delta is **descriptive**; only wall time
599-decides. So ONE rule governs every hypothesis (H1, H4, H5, H6, H7):
600-
601-- Each hypothesis must have a **wall-time counterfactual**: a debug-flag
602-  variant that removes or restores exactly the accused mechanism, run
603-  interleaved against the unmodified build on the same rig and fixture.
604-  A hypothesis with no counterfactual **cannot be confirmed** — it is
605-  carried as UNTESTED and pf-1 does not close.
606-- **`Δ` is defined per finding and per rig — it is NOT one number**
607:  (review round 5: the earlier text left it ambiguous between P1's
608:  layout gap and P2's old/new gap, which are different quantities):
609:  - **`Δ_P1(rig)`** = `destinit_median − srcinit_median` for
610:    `wm_tcp_mixed` on THAT rig (an invariance gap: new-vs-new, no old
611-    build involved). On rig W it is 1221 − 939 = **282 ms** — a **single
612-    nagatha session**; §pf-0 re-estimates it from four sessions on the `q`
613-    pairing, rules out **between-session** grading of any counterfactual, and
614-    requires pf-1 to measure its own **paired within-session** floor before
615:    grading. Read §pf-0 before grading any recovery against `Δ_P1`. On
616-    magneto↔skippy it is ~0 (8/8 pass) — so
617:    **P1 counterfactuals are graded on rig W only**; a Linux-rig recovery is
618-    meaningless against a gap that does not exist there.
619:  - **`Δ_P2(rig)`** = `new_median − old_same_session_median` for
620-    `push_tcp_small` on THAT rig (a converge gap, requires the `0f922de`
621-    build on that rig). netwatch-01: 1975 − 1644 = **331 ms**; zoey:
622-    4033 − 3636 = **397 ms**.
623-  Every reported recovery names its `Δ` and its rig. A counterfactual run
624-  on a rig whose `Δ` is ~0 proves nothing and is not reported as a kill.
625-- **Overlapping causes are attributed SEQUENTIALLY, never summed**
626-  (review round 5: H4/H7, and H6/H7, can each recover the same
627-  milliseconds, so independent recoveries would double-count and could
628-  "explain" >100% of `Δ`). Procedure: grade each hypothesis's recovery
629-  ALONE against the unmodified build; then, for every confirmed
--
643-  never "several hypotheses were consistent, moving on".
644-- Every measurement runs instrumentation-on/off pairs (per-member tracing
645-  across ~10k files can itself perturb a double-digit share of `Δ`).
646-
647-## Fix criteria (pre-registered; the owner walks the final numbers)
648-
649-- **The global rule dominates every bar below** (review round 2 flagged
650-  a contradiction between "necessary, not sufficient" and the `⇔`
651-  bars — the `⇔`s are hereby scoped as *definitions of the named
652-  finding's own bar*, never as a sufficient condition for acceptance).
653:  Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in
654-  EVERY acceptance cell passes independently against BOTH its
655-  same-session reference AND the committed baseline — no arm may exceed
656-  1.10 against either reference even when its counterpart bar passes
657:  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
658:  P2 bars below but regresses any other cell against either reference is
659-  **not** accepted.
660:- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
661-  `pull_tcp_mixed` ≤ 1.10 against BOTH references on the netwatch-01
662-  rig (CELLS escalation session, RUNS=8), with `wm_grpc_mixed` and the
663:  other invariance PASSes unregressed against both references. (Meeting
664-  this bar does not by itself accept the build — see the global rule.)
665:- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
666-  (same-session AND committed) on BOTH rigs (CELLS sessions), with the
667-  gRPC small-push cells unregressed. **"Unregressed" is given a
668-  reference and a tolerance (review round 3)**: each gRPC small-push
669-  cell must stay ≤ 1.10 against both of its own references AND must not
670-  worsen by more than **10% against its own pre-fix median on the same
671-  rig** (zoey 4731 ms; netwatch-01 2264 ms at 12c-win). The second
672-  clause exists because those cells currently range 0.801–1.001 — a fix
673-  that dragged Windows gRPC from 0.85 back to 1.05 would still pass a
674-  bare ≤1.10 bar while having eaten a real, measured win.
675-- Cross-direction converge-up is a SEPARATE bar (review round 2):
676-  every final cross-direction row must still meet the parent plan's
677-  new-vs-old ceiling (`ONE_TRANSFER_PATH.md` acceptance) or satisfy
678:  the registered platform-residue discriminator — invariance plus the
679-  per-direction bars alone would pass if a "fix" slowed BOTH layouts
680-  equally, violating converge-up.
681-- No suite regressions; the floor is ≥ the CURRENT count (1484 —
682-  ≥1483 would permit silently losing a test); any new pins carry
683-  guard proofs (temporary revert) per the loop.
684-- If investigation attributes part of a gap to something the plan's
685-  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
686-  that residue is RECORDED with its experiment and goes to the owner's
687-  otp-13 walk — never silently accepted.
688-
--
702-  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
703-  (corrected, review round 2 — "every row" was impossible: the
704-  same-session `old` arms and the committed baselines are OLD builds by
705-  construction, which is the entire point of a reference). Pre-fix
706-  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
707-  which are **replication and control evidence, not acceptance
708-  evidence**.
709-  **Rig D is included even though it is not a suspect (review round
710-  3).** Voiding otp-12c's pre-fix rows while re-running only Z and W
711-  would leave the parent plan's **delegated-parity bar**
712:  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
713-  evidence at all. "Not implicated" scopes what pf-1 must
714-  *instrument* — it does not waive an acceptance bar. Rig D's TCP
715-  verdict cells (+ the gRPC smoke) therefore rerun on the final build;
716-  both arms are new-build by construction there (rig D has no old
717-  baseline), so the whole cell is re-measured.
718-  **Every gRPC row the acceptance method requires reruns
719-  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
720-  earlier "if shared code changed, the gRPC cells rerun too" left the
721-  decision to the author's own judgement of what counts as shared, which
722-  is exactly the loophole H7 exploits: a shared regression can hide under
723:  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
724:  complete Z/W gRPC converge and invariance rows, so those are
725-  final-build rows, full stop — no conditional. Results land in fresh
726-  dated evidence dirs. **Then** otp-12d assembles the matrix from
727-  final-build rows, and the otp-13 owner walk reads it.
728-
729-## Known gaps
730-
731-- H1–H5 were graded against the actual tree by codex review
732-  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
733-  The old drivers are deleted from HEAD, but the pinned `0f922de`
734-  source/binaries diff and run fine — historical claims get live
735-  controls in pf-1, not pin-archaeology.
736:- zoey never measured P1: its rig anchors converge-up only, so there
737:  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
738-  says nothing about layout asymmetry (review 2026-07-12). pf-1's
739:  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
740-  qualifies per the 12b wm numbers).
741-- **The 12c-win rows are replication, not acceptance** (2026-07-13).
742-  They are pre-fix by definition, so `pf-final` voids them for
743:  acceptance; their value is that they (a) reproduce P1 and P2 on an
744-  independent session at the shipped sha, (b) supply the
745-  opposite-direction control (`mw_tcp_mixed` 1.044 PASS vs
746-  `wm_tcp_mixed` 1.300 FAIL — same carrier, same fixture) that narrows
747:  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
748:  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
749:  P2 1.149→1.201), so neither is drifting toward the bar on its own.
750-- **Rig-D delegated parity is not a SUSPECT, but it is still an
751-  ACCEPTANCE bar** (2026-07-13; scoped correctly at review round 3): the
752-  delegated-vs-direct matrix passed 7/7
753-  (`docs/bench/otp12c-delegated-2026-07-13/`), so delegation adds no
754-  measurable cost and pf-1 need not instrument the delegated trigger
755-  path. That is a statement about *where to look for the bug* — it does
756-  **not** waive the parent plan's delegated-parity bar, whose evidence
757-  is pre-fix and therefore void under pf-final. Rig D reruns on the
758-  final build (see pf-final).

exec
/bin/zsh -lc "rg -n -C 8 'flush|settle|sleep|start_ms|end_ms|elapsed|date \\+%s|Get-Date|measure|fsync|sync' scripts/bench_otp12_win.sh scripts/bench_otp12pf_mac.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/bench_otp12_win.sh-8-#   comparisons, matched-pair interleaved A/B — arm "old" = the pinned
scripts/bench_otp12_win.sh-9-#   pre-cutover pair (default 0f922de: Mac client rebuilt in a detached
scripts/bench_otp12_win.sh-10-#   worktree; Windows daemon built natively at that commit), arm "new"
scripts/bench_otp12_win.sh-11-#   = the run commit's pair. Verdicts against BOTH references (the
scripts/bench_otp12_win.sh-12-#   same-session old arm AND docs/bench/otp2w-baseline-2026-07-10/
scripts/bench_otp12_win.sh-13-#   summary.csv), per design D2 as amended.
scripts/bench_otp12_win.sh-14-#
scripts/bench_otp12_win.sh-15-#   BLOCK 2 — initiator/verb invariance (NEW pair only; the owner's
scripts/bench_otp12_win.sh:16:#   sentence, measured): per data direction x fixture x carrier, arm
scripts/bench_otp12_win.sh-17-#   "mac_init" vs arm "win_init" interleaved ABBA. Data Mac->Win (mw_*):
scripts/bench_otp12_win.sh-18-#   Mac client pushes vs Windows client pulls the SAME physical source
scripts/bench_otp12_win.sh-19-#   (the Mac module root IS $MAC_WORK — design F6). Data Win->Mac
scripts/bench_otp12_win.sh-20-#   (wm_*): Mac client pulls vs Windows client pushes the same staged
scripts/bench_otp12_win.sh-21-#   tree on D:. Cell grammar: <mw|wm>_<carrier>_<fixture>. Every arm
scripts/bench_otp12_win.sh-22-#   also gets converge rows against its data direction's old references
scripts/bench_otp12_win.sh-23-#   (design F3: no tolerance compounding), plus the F4 cross-direction
scripts/bench_otp12_win.sh-24-#   rows and the D-2026-07-12-1 discriminator gap rows (recorded, never
scripts/bench_otp12_win.sh-25-#   self-adjudicated).
scripts/bench_otp12_win.sh-26-#
scripts/bench_otp12_win.sh-27-# Methodology inherited verbatim from scripts/bench_otp2w_baseline.sh
scripts/bench_otp12_win.sh:28:# (self-timed durability: Write-VolumeCache on Windows / per-file fsync
scripts/bench_otp12_win.sh-29-# walk on macOS, keyed by DESTINATION OS never verb; Get-Counter drain;
scripts/bench_otp12_win.sh-30-# standby-list purge + macOS purge; WMI daemon launch — Windows OpenSSH
scripts/bench_otp12_win.sh-31-# kills session children; TOML literal-string module paths; stale-daemon
scripts/bench_otp12_win.sh-32-# refusal + PID-scoped teardown) and from bench_otp12_zoey.sh (ABBA
scripts/bench_otp12_win.sh-33-# counterbalance, pair-void valid-run rule with 2xRUNS cap + INCOMPLETE,
scripts/bench_otp12_win.sh-34-# exit codes checked, +sha provenance, sha256 staging manifest,
scripts/bench_otp12_win.sh-35-# PREFLIGHT_ONLY, CELLS allowlist for D2 escalations, per-run
scripts/bench_otp12_win.sh:36:# destination sweep after the measured flush — the zoey I/O-storm
scripts/bench_otp12_win.sh-37-# lesson, kept uniform here).
scripts/bench_otp12_win.sh-38-#
scripts/bench_otp12_win.sh:39:# Windows-side timed windows (win_init arms) are measured ON Windows —
scripts/bench_otp12_win.sh-40-# a Stopwatch brackets the blit.exe invocation inside one ssh call and
scripts/bench_otp12_win.sh-41-# prints "<ms>,<exit>"; the ssh round trip stays outside the window by
scripts/bench_otp12_win.sh-42-# construction (the otp-2w F3 rule applied to a whole client run).
scripts/bench_otp12_win.sh-43-#
scripts/bench_otp12_win.sh-44-# Usage (from the client Mac):
scripts/bench_otp12_win.sh-45-#   export WIN_SSH=michael@10.1.10.173
scripts/bench_otp12_win.sh-46-#   export WIN_HOST=10.1.10.173
scripts/bench_otp12_win.sh-47-#   export WIN_TEST='D:\blit-test'
--
scripts/bench_otp12_win.sh-113-
scripts/bench_otp12_win.sh-114-log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
scripts/bench_otp12_win.sh-115-die() { log "FATAL: $*"; exit 1; }
scripts/bench_otp12_win.sh-116-SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
scripts/bench_otp12_win.sh-117-wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
scripts/bench_otp12_win.sh-118-now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
scripts/bench_otp12_win.sh-119-
scripts/bench_otp12_win.sh-120-# --- Self-timed durability (destination-OS-keyed, never verb-keyed) ----
scripts/bench_otp12_win.sh:121:flush_win_ms() {   # Windows volume flush, self-timed; prints ms or NA
scripts/bench_otp12_win.sh-122-    # Sentinel-framed and error-terminating (codex otp-12b F7): a
scripts/bench_otp12_win.sh:123:    # failed flush or garbage output must never read as a plausible
scripts/bench_otp12_win.sh-124-    # number — NA makes the caller VOID the run per the D2 rule.
scripts/bench_otp12_win.sh-125-    local v
scripts/bench_otp12_win.sh-126-    v=$(wssh "\$ErrorActionPreference = 'Stop'; \$sw = [Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache $WIN_DRIVE; \$sw.Stop(); \"F:\$([int]\$sw.Elapsed.TotalMilliseconds):F\"" 2>/dev/null \
scripts/bench_otp12_win.sh-127-        | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p' | head -1)
scripts/bench_otp12_win.sh-128-    echo "${v:-NA}"
scripts/bench_otp12_win.sh-129-}
scripts/bench_otp12_win.sh:130:fsync_tree_ms() {   # macOS per-file fsync walk; prints its own elapsed ms
scripts/bench_otp12_win.sh-131-    python3 - "$1" <<'PYEOF'
scripts/bench_otp12_win.sh-132-import os, sys, time
scripts/bench_otp12_win.sh-133-t = time.monotonic()
scripts/bench_otp12_win.sh-134-for root, dirs, files in os.walk(sys.argv[1]):
scripts/bench_otp12_win.sh-135-    for name in files:
scripts/bench_otp12_win.sh-136-        fd = os.open(os.path.join(root, name), os.O_RDONLY)
scripts/bench_otp12_win.sh:137:        os.fsync(fd)
scripts/bench_otp12_win.sh-138-        os.close(fd)
scripts/bench_otp12_win.sh-139-print(int((time.monotonic() - t) * 1000))
scripts/bench_otp12_win.sh-140-PYEOF
scripts/bench_otp12_win.sh-141-}
scripts/bench_otp12_win.sh-142-
scripts/bench_otp12_win.sh-143-want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }
scripts/bench_otp12_win.sh-144-
scripts/bench_otp12_win.sh-145-# --- Provenance + manifest (otp-12a lessons: +sha form, fail closed) ---
--
scripts/bench_otp12_win.sh-288-'@
scripts/bench_otp12_win.sh-289-\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = 'cmd /c \"\"$ACTIVE_WIN_DAEMON\" --config \"$WIN_TEST\\bench-config.toml\" > \"$WIN_TEST\\daemon-out.log\" 2> \"$WIN_TEST\\daemon-err.log\"\"' }
scripts/bench_otp12_win.sh-290-if (\$r.ReturnValue -ne 0) { \"wmi create failed: \$(\$r.ReturnValue)\"; exit 1 }
scripts/bench_otp12_win.sh-291-Set-Content -Path '$WIN_TEST\\daemon-wmi.pid' -Value \$r.ProcessId"
scripts/bench_otp12_win.sh-292-    # The WMI pid is cmd's, recorded IMMEDIATELY so an interruption
scripts/bench_otp12_win.sh-293-    # before the verify step leaves nothing untracked (codex otp-12b
scripts/bench_otp12_win.sh-294-    # F9); the verify step resolves the daemon pid as the blit-daemon
scripts/bench_otp12_win.sh-295-    # whose PARENT is our cmd — a name lookup tied to THIS launch.
scripts/bench_otp12_win.sh:296:    sleep 2
scripts/bench_otp12_win.sh-297-    wssh "\$cmdpid = Get-Content '$WIN_TEST\\daemon-wmi.pid' -ErrorAction SilentlyContinue
scripts/bench_otp12_win.sh-298-\$d = Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object { \$_.ParentProcessId -eq \$cmdpid } | Select-Object -First 1
scripts/bench_otp12_win.sh-299-if (-not \$d) { Get-Content '$WIN_TEST\\daemon-err.log' -ErrorAction SilentlyContinue | Select-Object -First 10; exit 1 }
scripts/bench_otp12_win.sh-300-Set-Content -Path '$WIN_TEST\\daemon.pid' -Value \$d.ProcessId" \
scripts/bench_otp12_win.sh-301-        || die "$arm Windows daemon failed to start"
scripts/bench_otp12_win.sh-302-    WIN_ARM="$arm"
scripts/bench_otp12_win.sh-303-    log "windows daemon up ($arm pair) on $WIN_HOST:$PORT"
scripts/bench_otp12_win.sh-304-}
--
scripts/bench_otp12_win.sh-336-[[module]]
scripts/bench_otp12_win.sh-337-name = "bench"
scripts/bench_otp12_win.sh-338-path = "$MAC_MODULE_ROOT"
scripts/bench_otp12_win.sh-339-EOF
scripts/bench_otp12_win.sh-340-    MAC_DAEMON_STARTED=1
scripts/bench_otp12_win.sh-341-    nohup "$MAC_DAEMON" --config "$MAC_WORK/bench-daemon-config.toml" \
scripts/bench_otp12_win.sh-342-        > "$MAC_WORK/bench-daemon.log" 2>&1 &
scripts/bench_otp12_win.sh-343-    echo $! > "$MAC_WORK/bench-daemon.pid"
scripts/bench_otp12_win.sh:344:    sleep 1
scripts/bench_otp12_win.sh-345-    kill -0 "$(cat "$MAC_WORK/bench-daemon.pid")" 2>/dev/null \
scripts/bench_otp12_win.sh-346-        || { tail -5 "$MAC_WORK/bench-daemon.log"; die "Mac daemon failed to start"; }
scripts/bench_otp12_win.sh-347-    log "mac daemon up on $MAC_HOST:$PORT (module bench -> $MAC_MODULE_ROOT)"
scripts/bench_otp12_win.sh-348-}
scripts/bench_otp12_win.sh-349-mac_daemon_stop() {
scripts/bench_otp12_win.sh-350-    local p
scripts/bench_otp12_win.sh-351-    p=$(cat "$MAC_WORK/bench-daemon.pid" 2>/dev/null) || true
scripts/bench_otp12_win.sh-352-    if [[ -n "${p:-}" ]] && ps -p "$p" -o comm= 2>/dev/null | grep -q blit-daemon; then
--
scripts/bench_otp12_win.sh-385-    # bare \r mid-row makes python's universal-newline csv reader split
scripts/bench_otp12_win.sh-386-    # the row before its `valid` field — every comparison then reads
scripts/bench_otp12_win.sh-387-    # INCOMPLETE (found live: the whole first e21cf84 session verdicted
scripts/bench_otp12_win.sh-388-    # INCOMPLETE off 192 perfectly valid runs).
scripts/bench_otp12_win.sh-389-    RUN_DRAIN=${RUN_DRAIN//$'\r'/}
scripts/bench_otp12_win.sh-390-    RUN_DRAIN=${RUN_DRAIN// /_}
scripts/bench_otp12_win.sh-391-    echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
scripts/bench_otp12_win.sh-392-    [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
scripts/bench_otp12_win.sh:393:    sync
scripts/bench_otp12_win.sh-394-    sudo -n /usr/sbin/purge
scripts/bench_otp12_win.sh-395-    wssh "pwsh -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null
scripts/bench_otp12_win.sh-396-}
scripts/bench_otp12_win.sh-397-
scripts/bench_otp12_win.sh-398-# --- Fixtures (shape-verified; the otp-12a F2 rule) ----------------------
scripts/bench_otp12_win.sh-399-FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
scripts/bench_otp12_win.sh-400-FIX_COUNT_small=10000; FIX_BYTES_small=40960000
scripts/bench_otp12_win.sh-401-FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
--
scripts/bench_otp12_win.sh-460-        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
scripts/bench_otp12_win.sh-461-        [[ "$got" == "$want" ]] || die "pull_src_$w still wrong after staging ($got/$want)"
scripts/bench_otp12_win.sh-462-        log "  staged pull_src_$w ($got files)"
scripts/bench_otp12_win.sh-463-    done
scripts/bench_otp12_win.sh-464-}
scripts/bench_otp12_win.sh-465-
scripts/bench_otp12_win.sh-466-# --- Timed runs -----------------------------------------------------------
scripts/bench_otp12_win.sh-467-CSV="$OUT_DIR/runs.csv"
scripts/bench_otp12_win.sh:468:echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
scripts/bench_otp12_win.sh-469-META="$OUT_DIR/meta.csv"
scripts/bench_otp12_win.sh-470-echo "cell,pairs_attempted,complete" > "$META"
scripts/bench_otp12_win.sh-471-
scripts/bench_otp12_win.sh-472-RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
scripts/bench_otp12_win.sh-473-
scripts/bench_otp12_win.sh-474-# Mac-initiated runs (block 1 both arms; block 2 mac_init arms).
scripts/bench_otp12_win.sh-475-mac_push_run() {   # blit_bin cell rid dest_remote src [flags...]
scripts/bench_otp12_win.sh-476-    local blit="$1" cell="$2" rid="$3" dest="$4" src="$5"; shift 5
scripts/bench_otp12_win.sh-477-    local start end rc=0
scripts/bench_otp12_win.sh-478-    drop_caches "${cell}-$rid"
scripts/bench_otp12_win.sh-479-    start=$(now_ms)
scripts/bench_otp12_win.sh-480-    "$blit" copy "$src" "${dest}push_${SESSION_TAG}_${cell}_${rid}/" --yes "$@" \
scripts/bench_otp12_win.sh-481-        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
scripts/bench_otp12_win.sh-482-    end=$(now_ms)
scripts/bench_otp12_win.sh-483-    if [[ "$dest" == "$WIN_REMOTE" ]]; then
scripts/bench_otp12_win.sh:484:        RUN_FLUSH=$(flush_win_ms)
scripts/bench_otp12_win.sh-485-        wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\push_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
scripts/bench_otp12_win.sh-486-    else
scripts/bench_otp12_win.sh:487:        RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
scripts/bench_otp12_win.sh-488-        rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
scripts/bench_otp12_win.sh-489-    fi
scripts/bench_otp12_win.sh-490-    RUN_VALID=yes
scripts/bench_otp12_win.sh-491-    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
scripts/bench_otp12_win.sh-492-    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc
scripts/bench_otp12_win.sh-493-    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
scripts/bench_otp12_win.sh-494-}
scripts/bench_otp12_win.sh-495-mac_pull_run() {   # blit_bin cell rid remote_src [flags...]
--
scripts/bench_otp12_win.sh-497-    local start end rc=0
scripts/bench_otp12_win.sh-498-    local dst="$MAC_WORK/dst_pull_${SESSION_TAG}_${cell}_${rid}"
scripts/bench_otp12_win.sh-499-    mkdir -p "$dst"
scripts/bench_otp12_win.sh-500-    drop_caches "${cell}-$rid"
scripts/bench_otp12_win.sh-501-    start=$(now_ms)
scripts/bench_otp12_win.sh-502-    "$blit" copy "$rsrc" "$dst" --yes "$@" \
scripts/bench_otp12_win.sh-503-        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
scripts/bench_otp12_win.sh-504-    end=$(now_ms)
scripts/bench_otp12_win.sh:505:    RUN_FLUSH=$(fsync_tree_ms "$dst")
scripts/bench_otp12_win.sh-506-    rm -rf "$dst"
scripts/bench_otp12_win.sh-507-    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc; RUN_VALID=yes
scripts/bench_otp12_win.sh-508-    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
scripts/bench_otp12_win.sh-509-}
scripts/bench_otp12_win.sh-510-# Windows-initiated runs (block 2 win_init arms): the transfer window is
scripts/bench_otp12_win.sh-511-# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
scripts/bench_otp12_win.sh-512-win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
scripts/bench_otp12_win.sh-513-    # Sentinel-framed (codex otp-12b F7): anything but a clean
--
scripts/bench_otp12_win.sh-526-        wssh "Get-Content '$WIN_TEST\\client-err.log' -ErrorAction SilentlyContinue | Select-Object -First 20" \
scripts/bench_otp12_win.sh-527-            > "$OUT_DIR/blit-logs/${cell}_${rid}.err" 2>&1 || true
scripts/bench_otp12_win.sh-528-    fi
scripts/bench_otp12_win.sh-529-}
scripts/bench_otp12_win.sh-530-win_pull_run() {   # cell rid remote_src(from mac) [flag]; dest = win module
scripts/bench_otp12_win.sh-531-    local cell="$1" rid="$2" rsrc="$3" flag="${4:-}"
scripts/bench_otp12_win.sh-532-    drop_caches "${cell}-$rid"
scripts/bench_otp12_win.sh-533-    win_client_run "$cell" "$rid" "$rsrc" "$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}" "$flag"
scripts/bench_otp12_win.sh:534:    RUN_FLUSH=$(flush_win_ms)
scripts/bench_otp12_win.sh-535-    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
scripts/bench_otp12_win.sh-536-    RUN_VALID=yes
scripts/bench_otp12_win.sh-537-    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
scripts/bench_otp12_win.sh-538-    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC
scripts/bench_otp12_win.sh-539-    [[ "$T_RC" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
scripts/bench_otp12_win.sh-540-}
scripts/bench_otp12_win.sh-541-win_push_run() {   # cell rid src(win local path) [flag]; dest = mac module
scripts/bench_otp12_win.sh-542-    local cell="$1" rid="$2" src="$3" flag="${4:-}"
scripts/bench_otp12_win.sh-543-    drop_caches "${cell}-$rid"
scripts/bench_otp12_win.sh-544-    win_client_run "$cell" "$rid" "$src" "${MAC_REMOTE}push_${SESSION_TAG}_${cell}_${rid}/" "$flag"
scripts/bench_otp12_win.sh:545:    RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
scripts/bench_otp12_win.sh-546-    rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
scripts/bench_otp12_win.sh-547-    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC; RUN_VALID=yes
scripts/bench_otp12_win.sh-548-    [[ "$T_RC" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
scripts/bench_otp12_win.sh-549-}
scripts/bench_otp12_win.sh-550-
scripts/bench_otp12_win.sh-551-# One interleaved comparison; ABBA; pair-void; INCOMPLETE at the cap.
scripts/bench_otp12_win.sh-552-# run_one <cell> <armA> <armB> <fnA...>|<fnB...> dispatch happens via
scripts/bench_otp12_win.sh-553-# small wrappers below to keep bash 3.2-simple.
--
scripts/bench_otp12_win.sh-571-            [[ "$RUN_VALID" == yes ]] || pair_valid=no
scripts/bench_otp12_win.sh-572-            case "$aname" in
scripts/bench_otp12_win.sh-573-                old) bld="$OLD_SHA"; init=mac;;
scripts/bench_otp12_win.sh-574-                new|mac_init) bld="$NEW_SHA"; init=mac;;
scripts/bench_otp12_win.sh-575-                win_init) bld="$NEW_SHA"; init=win;;
scripts/bench_otp12_win.sh-576-            esac
scripts/bench_otp12_win.sh-577-            local row="$cell,$aname,$bld,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
scripts/bench_otp12_win.sh-578-            if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
scripts/bench_otp12_win.sh:579:            log "  $cell/$aname slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
scripts/bench_otp12_win.sh-580-        done
scripts/bench_otp12_win.sh-581-        echo "$rowA,$pair_valid" >> "$CSV"
scripts/bench_otp12_win.sh-582-        echo "$rowB,$pair_valid" >> "$CSV"
scripts/bench_otp12_win.sh-583-        if [[ "$pair_valid" == yes ]]; then
scripts/bench_otp12_win.sh-584-            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
scripts/bench_otp12_win.sh-585-        else
scripts/bench_otp12_win.sh-586-            log "  $cell: pair at slot $slot VOIDED — re-running the slot"
scripts/bench_otp12_win.sh-587-        fi
--
scripts/bench_otp12_win.sh-807-    done
scripts/bench_otp12_win.sh-808-
scripts/bench_otp12_win.sh-809-    if [[ -n "$CELLS" ]]; then
scripts/bench_otp12_win.sh-810-        local c
scripts/bench_otp12_win.sh-811-        for c in ${CELLS//,/ }; do
scripts/bench_otp12_win.sh-812-            # Header excluded — CELLS=cell must not match "cell,…"
scripts/bench_otp12_win.sh-813-            # (codex otp-12b F10).
scripts/bench_otp12_win.sh-814-            tail -n +2 "$META" | grep -q "^$c," \
scripts/bench_otp12_win.sh:815:                || die "CELLS entry '$c' matched no comparison — nothing was measured for it"
scripts/bench_otp12_win.sh-816-        done
scripts/bench_otp12_win.sh-817-    fi
scripts/bench_otp12_win.sh-818-
scripts/bench_otp12_win.sh-819-    win_daemon_stop
scripts/bench_otp12_win.sh-820-    mac_daemon_stop
scripts/bench_otp12_win.sh-821-    compute_verdicts
scripts/bench_otp12_win.sh-822-
scripts/bench_otp12_win.sh-823-    log ""
--
scripts/bench_otp12pf_mac.sh-22-# work (they take NO data); a timed run refuses.
scripts/bench_otp12pf_mac.sh-23-# =============================================================================
scripts/bench_otp12pf_mac.sh-24-# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
scripts/bench_otp12pf_mac.sh-25-# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (rev 11)
scripts/bench_otp12pf_mac.sh-26-# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
scripts/bench_otp12pf_mac.sh-27-#
scripts/bench_otp12pf_mac.sh-28-# WHY THIS RIG EXISTS
scripts/bench_otp12pf_mac.sh-29-# -------------------
scripts/bench_otp12pf_mac.sh:30:# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
scripts/bench_otp12pf_mac.sh-31-# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
scripts/bench_otp12pf_mac.sh-32-# cell. It answers ONE question, SCOPED TO THIS PAIR:
scripts/bench_otp12pf_mac.sh-33-#
scripts/bench_otp12pf_mac.sh-34-#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
scripts/bench_otp12pf_mac.sh-35-#
scripts/bench_otp12pf_mac.sh-36-#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
scripts/bench_otp12pf_mac.sh-37-#     "platform residue" that can be waived; code-level hypotheses strengthen. It
scripts/bench_otp12pf_mac.sh-38-#     leaves macOS/APFS and host x role explanations OPEN.
--
scripts/bench_otp12pf_mac.sh-48-# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
scripts/bench_otp12pf_mac.sh-49-#
scripts/bench_otp12pf_mac.sh-50-# THE INSTRUMENT IS THE RISK. Three claims in this project have been retracted to
scripts/bench_otp12pf_mac.sh-51-# harness bugs, and this harness alone has now had 20 defects found across two
scripts/bench_otp12pf_mac.sh-52-# reviews. What round 2 caught, and what is fixed here:
scripts/bench_otp12pf_mac.sh-53-#
scripts/bench_otp12pf_mac.sh-54-#   * THE TIMER WAS MEASURING FSYNC NOISE. It captured time.monotonic() in TWO
scripts/bench_otp12pf_mac.sh-55-#     separate `python3 -c` processes and subtracted them. On macOS that clock is
scripts/bench_otp12pf_mac.sh:56:#     PROCESS-RELATIVE: a 1000 ms sleep measured -1 ms on nagatha and 2 ms on q
scripts/bench_otp12pf_mac.sh:57:#     (measured; yes, negative). Every `ms` row would have been ~= fsync_ms alone,
scripts/bench_otp12pf_mac.sh-58-#     and the invariance ratio — THE ENTIRE MEASURAND — would have been computed on
scripts/bench_otp12pf_mac.sh:59:#     fsync noise, which can manufacture or mask a one-directional effect at will.
scripts/bench_otp12pf_mac.sh-60-#     The repo ALREADY documents this trap (bench_otp12_zoey.sh:116 uses time.time()
scripts/bench_otp12pf_mac.sh-61-#     precisely because monotonic is wrong across processes) and I reintroduced it
scripts/bench_otp12pf_mac.sh-62-#     anyway. Now: ONE process times itself and spawns the client (time_argv), and
scripts/bench_otp12pf_mac.sh:63:#     PREFLIGHT PROVES IT on both hosts against a known sleep before any data.
scripts/bench_otp12pf_mac.sh-64-#   * The preflight COULD NOT SUCCEED: `grep -c` exits 1 on no match, so a CLEAN
scripts/bench_otp12pf_mac.sh-65-#     binary tripped the dirty-marker probe and died; and norm_mac used gawk's
scripts/bench_otp12pf_mac.sh-66-#     strtonum(), absent from stock macOS awk. The round-1 "fixes" were never
scripts/bench_otp12pf_mac.sh-67-#     executed — I ran `bash -n`, not the gates. Every gate below is now exercised
scripts/bench_otp12pf_mac.sh-68-#     by SELFTEST=1, which runs them for real.
scripts/bench_otp12pf_mac.sh-69-#   * Gates FAILED OPEN: pgrep errors read as "quiet"; a failed `top` read as 0%
scripts/bench_otp12pf_mac.sh-70-#     CPU and a late idle sample could overwrite a busy one; non-numeric `iostat`
scripts/bench_otp12pf_mac.sh-71-#     read as zero and CERTIFIED drainage; the drain watched a hardcoded `disk0`
--
scripts/bench_otp12pf_mac.sh-159-  echo "REFUSING: the pre-registered constants are NOT tunable, and these are set in the" >&2
scripts/bench_otp12pf_mac.sh-160-  echo "environment:$_overrides" >&2
scripts/bench_otp12pf_mac.sh-161-  echo "A rule the operator can retune after seeing the data is not a pre-registration." >&2
scripts/bench_otp12pf_mac.sh-162-  echo "To change one, amend docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md and" >&2
scripts/bench_otp12pf_mac.sh-163-  echo "put it back through review. That is the entire point of the document." >&2
scripts/bench_otp12pf_mac.sh-164-  exit 2
scripts/bench_otp12pf_mac.sh-165-fi
scripts/bench_otp12pf_mac.sh-166-
scripts/bench_otp12pf_mac.sh:167:SETTLE_MS=250              # equal pre-fsync window on BOTH arms
scripts/bench_otp12pf_mac.sh-168-# Computed ONCE, HERE, at top level — and this line is load-bearing history.
scripts/bench_otp12pf_mac.sh-169-#
scripts/bench_otp12pf_mac.sh:170:# It used to be computed inline as `sleep $(awk ... 'BEGIN{printf \"%.3f\", m/1000}')`
scripts/bench_otp12pf_mac.sh-171-# INSIDE the double-quoted hrun string. A command substitution is parsed FRESH by
scripts/bench_otp12pf_mac.sh-172-# bash, so those `\"` escapes — which are correct for hrun's two-level strings — were
scripts/bench_otp12pf_mac.sh:173:# literal backslashes to awk. **The awk errored on EVERY call, `sleep` got an empty
scripts/bench_otp12pf_mac.sh-174-# argument and FAILED, and the old code ignored its exit status because the python
scripts/bench_otp12pf_mac.sh-175-# walk that followed supplied the status.**
scripts/bench_otp12pf_mac.sh-176-#
scripts/bench_otp12pf_mac.sh-177-# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
scripts/bench_otp12pf_mac.sh-178-# it. And 24660ae is the commit that added it TO FIX the free-writeback asymmetry
scripts/bench_otp12pf_mac.sh-179-# that reverses sign with direction — the artifact judged capable of MANUFACTURING a
scripts/bench_otp12pf_mac.sh:180:# one-directional P1 out of nothing. The pre-registration has claimed an equal settle
scripts/bench_otp12pf_mac.sh-181-# on both arms through revisions 3, 4 and 5. It was never applied.
scripts/bench_otp12pf_mac.sh-182-#
scripts/bench_otp12pf_mac.sh-183-# Found only by EXECUTING it (round-5 codex flagged the ignored exit status; running
scripts/bench_otp12pf_mac.sh-184-# it showed the status was ALWAYS failure). `bash -n` sees nothing here.
scripts/bench_otp12pf_mac.sh-185-SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
scripts/bench_otp12pf_mac.sh:186:[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
scripts/bench_otp12pf_mac.sh-187-LOAD_MAX=3.0               # start AND end load1 bar on both Macs
scripts/bench_otp12pf_mac.sh-188-DRAIN_ITERS=60
scripts/bench_otp12pf_mac.sh-189-DRAIN_QUIET=3
scripts/bench_otp12pf_mac.sh-190-DRAIN_MBPS=2               # destination disk must be below this to start a window
scripts/bench_otp12pf_mac.sh:191:DELTA_REF_MS=230           # rig W's measured Delta_P1 — THE reference effect
scripts/bench_otp12pf_mac.sh:192:TIMER_TOLERANCE_MS=120     # the timer self-test's allowed error on a 1000 ms sleep
scripts/bench_otp12pf_mac.sh-193-
scripts/bench_otp12pf_mac.sh-194-# THE REGISTERED TOPOLOGY. These are assigned HERE, BELOW the override check, for the same
scripts/bench_otp12pf_mac.sh-195-# reason SETTLE_MS is: assigned above it, the pinning line itself would set the variable and
scripts/bench_otp12pf_mac.sh-196-# the check would then see it as "present in the environment" and refuse EVERY run. (It did.
scripts/bench_otp12pf_mac.sh-197-# Caught by RUNNING it -- the first draft of this fix put them at the top of the file and the
scripts/bench_otp12pf_mac.sh-198-# harness refused to start. A protection that cannot PASS is as dead as one that cannot FAIL.)
scripts/bench_otp12pf_mac.sh-199-N_IP=10.1.10.92                       # nagatha, 10GbE en11
scripts/bench_otp12pf_mac.sh-200-N_NIC=en11
scripts/bench_otp12pf_mac.sh:201:N_MAC=00:e0:4d:01:4c:a3               # nagatha's OWN en11 MAC (measured)
scripts/bench_otp12pf_mac.sh-202-Q_SSH=michael@q
scripts/bench_otp12pf_mac.sh-203-Q_IP=10.1.10.54                       # q, 10GbE en8
scripts/bench_otp12pf_mac.sh-204-Q_NIC=en8
scripts/bench_otp12pf_mac.sh:205:Q_MAC=00:01:d2:19:04:a3               # q's OWN en8 MAC (measured)
scripts/bench_otp12pf_mac.sh-206-
scripts/bench_otp12pf_mac.sh-207-# RUNS and PORT are registered too (round-11 grok, LOW): RUNS was `${RUNS:-8}` and absent from
scripts/bench_otp12pf_mac.sh-208-# the refusal list above, so it was pinned only by a preflight check -- a weaker guarantee than
scripts/bench_otp12pf_mac.sh-209-# every other registered constant, for the one number the whole rule depends on (n is EXACTLY
scripts/bench_otp12pf_mac.sh-210-# 8; at any larger n the >=95% interval starts TRIMMING and a bimodal arm yields a false null).
scripts/bench_otp12pf_mac.sh-211-# The preflight check STAYS: it also guards a source edit, which this line cannot.
scripts/bench_otp12pf_mac.sh-212-RUNS=8
scripts/bench_otp12pf_mac.sh-213-PORT=9031
--
scripts/bench_otp12pf_mac.sh-273-FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
scripts/bench_otp12pf_mac.sh-274-FIX_COUNT_small=10000; FIX_BYTES_small=40960000
scripts/bench_otp12pf_mac.sh-275-FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12pf_mac.sh-276-fix_count() { case "$1" in large) echo $FIX_COUNT_large;; mixed) echo $FIX_COUNT_mixed;; small) echo $FIX_COUNT_small;; esac; }
scripts/bench_otp12pf_mac.sh-277-fix_bytes() { case "$1" in large) echo $FIX_BYTES_large;; mixed) echo $FIX_BYTES_mixed;; small) echo $FIX_BYTES_small;; esac; }
scripts/bench_otp12pf_mac.sh-278-
scripts/bench_otp12pf_mac.sh-279-# =============================================================================
scripts/bench_otp12pf_mac.sh-280-# THE TIMER. One process times itself AND spawns the client, so the interval is
scripts/bench_otp12pf_mac.sh:281:# measured by a single clock and python's startup cost falls outside it.
scripts/bench_otp12pf_mac.sh-282-#
scripts/bench_otp12pf_mac.sh-283-# NEVER bracket a command with two separate `python3 -c 'time.monotonic()'` calls:
scripts/bench_otp12pf_mac.sh:284:# on macOS that clock is PROCESS-RELATIVE and the difference is garbage (measured:
scripts/bench_otp12pf_mac.sh:285:# -1 ms and 2 ms for a 1000 ms sleep). bench_otp12_zoey.sh:116 already said so.
scripts/bench_otp12pf_mac.sh-286-# =============================================================================
scripts/bench_otp12pf_mac.sh-287-time_argv() {   # $1 = host; rest = argv. Echoes "MS,RC" or "" on a broken probe.
scripts/bench_otp12pf_mac.sh-288-  local h="$1"; shift
scripts/bench_otp12pf_mac.sh-289-  local qa="" a
scripts/bench_otp12pf_mac.sh-290-  for a in "$@"; do qa="$qa $(printf '%q' "$a")"; done
scripts/bench_otp12pf_mac.sh-291-  hrun "$h" "$(hpy "$h") - $qa <<'PYEOF'
scripts/bench_otp12pf_mac.sh-292-import subprocess, sys, time
scripts/bench_otp12pf_mac.sh-293-argv = [a for a in sys.argv[1:] if a]          # an empty flag must not become argv
--
scripts/bench_otp12pf_mac.sh-299-print('R:%d,%d:R' % (ms, rc))
scripts/bench_otp12pf_mac.sh-300-PYEOF" | nocr | sed -n 's/.*R:\(-\{0,1\}[0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1
scripts/bench_otp12pf_mac.sh-301-}
scripts/bench_otp12pf_mac.sh-302-
scripts/bench_otp12pf_mac.sh-303-# The gate that makes the timer bug unshippable: prove the clock on the rig,
scripts/bench_otp12pf_mac.sh-304-# against a known interval, before any data is taken.
scripts/bench_otp12pf_mac.sh-305-timer_gate() {
scripts/bench_otp12pf_mac.sh-306-  local h="$1" out ms rc lo hi
scripts/bench_otp12pf_mac.sh:307:  out="$(time_argv "$h" /bin/sleep 1)"
scripts/bench_otp12pf_mac.sh-308-  [[ "$out" == *,* ]] || die_blind "$(hname "$h"): the timer probe returned nothing — refusing"
scripts/bench_otp12pf_mac.sh-309-  ms="${out%%,*}"; rc="${out##*,}"
scripts/bench_otp12pf_mac.sh-310-  [[ "$rc" == 0 ]] || die_blind "$(hname "$h"): the timer probe's own child exited $rc"
scripts/bench_otp12pf_mac.sh-311-  lo=$(( 1000 - TIMER_TOLERANCE_MS )); hi=$(( 1000 + TIMER_TOLERANCE_MS ))
scripts/bench_otp12pf_mac.sh-312-  if (( ms < lo || ms > hi )); then
scripts/bench_otp12pf_mac.sh:313:    die "$(hname "$h"): THE TIMER IS LYING — a 1000 ms sleep measured ${ms} ms (allowed ${lo}-${hi}).
scripts/bench_otp12pf_mac.sh-314-This is the round-2 killer: cross-process time.monotonic() on macOS is PROCESS-RELATIVE and
scripts/bench_otp12pf_mac.sh:315:read -1 ms / 2 ms for this exact sleep. Every row would be fsync noise. REFUSING to take data."
scripts/bench_otp12pf_mac.sh-316-  fi
scripts/bench_otp12pf_mac.sh:317:  log "  timer ok on $(hname "$h"): a 1000 ms sleep measures ${ms} ms"
scripts/bench_otp12pf_mac.sh-318-}
scripts/bench_otp12pf_mac.sh-319-
scripts/bench_otp12pf_mac.sh-320-# --- provenance ---------------------------------------------------------------
scripts/bench_otp12pf_mac.sh-321-# `die` inside $(...) exits only the SUBSHELL, so the outer command substitution
scripts/bench_otp12pf_mac.sh-322-# succeeds with an empty value. These return non-zero instead and the CALLER dies.
scripts/bench_otp12pf_mac.sh-323-embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
scripts/bench_otp12pf_mac.sh-324-  local h="$1" p="$2" raw hit dirty
scripts/bench_otp12pf_mac.sh-325-  # `grep -c` exits 1 on NO MATCH, which is not an error. Only rc>=2 is. The old
--
scripts/bench_otp12pf_mac.sh-421-  # The MAX across samples, not the last: a late idle sample could overwrite an
scripts/bench_otp12pf_mac.sh-422-  # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
scripts/bench_otp12pf_mac.sh-423-  # `$2+0` coerced ANY non-numeric field to zero -- the same defect the drain had with "."
scripts/bench_otp12pf_mac.sh-424-  # (round-10 codex). A field that is not a number is an ERROR, not 0% CPU.
scripts/bench_otp12pf_mac.sh-425-  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
scripts/bench_otp12pf_mac.sh-426-    | awk '/^mds_stores/{ if (\$2 !~ /^[0-9]+(\\.[0-9]+)?\$/) { bad = 1 } else if (\$2+0 > m) { m = \$2+0 } }
scripts/bench_otp12pf_mac.sh-427-           END{ if (NR == 0 || bad) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
scripts/bench_otp12pf_mac.sh-428-  [[ "$cpu" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
scripts/bench_otp12pf_mac.sh:429:  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
scripts/bench_otp12pf_mac.sh-430-}
scripts/bench_otp12pf_mac.sh-431-
scripts/bench_otp12pf_mac.sh-432-load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
scripts/bench_otp12pf_mac.sh-433-load_gate() {
scripts/bench_otp12pf_mac.sh-434-  local h="$1" l ok
scripts/bench_otp12pf_mac.sh-435-  l="$(load1 "$h")" || l=""
scripts/bench_otp12pf_mac.sh-436-  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die_blind "$(hname "$h"): cannot read load1 (got '$l') — refusing"
scripts/bench_otp12pf_mac.sh-437-  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
--
scripts/bench_otp12pf_mac.sh-538-PYEOF" | nocr | sed -n 's/.*M:\([0-9][0-9]*\):\([0-9.][0-9.]*\):M.*/\1 \2/p' | head -1
scripts/bench_otp12pf_mac.sh-539-}
scripts/bench_otp12pf_mac.sh-540-
scripts/bench_otp12pf_mac.sh-541-mss_gate() {   # PREFLIGHT policy: no data has been taken, so REFUSE.
scripts/bench_otp12pf_mac.sh-542-  local h="$1" out mss local_ip peer
scripts/bench_otp12pf_mac.sh-543-  peer="$(hip "$(other "$h")")"
scripts/bench_otp12pf_mac.sh-544-  out="$(mss_read "$h")" || out=""
scripts/bench_otp12pf_mac.sh-545-  [[ -n "$out" ]] \
scripts/bench_otp12pf_mac.sh:546:    || die_blind "$(hname "$h"): cannot measure the negotiated MSS to $peer — refusing (an unproven path is not a proven one)"
scripts/bench_otp12pf_mac.sh-547-  read -r mss local_ip <<<"$out"
scripts/bench_otp12pf_mac.sh-548-  [[ "$mss" =~ ^[0-9]+$ ]] \
scripts/bench_otp12pf_mac.sh-549-    || die_blind "$(hname "$h"): the MSS probe returned '$mss' — refusing"
scripts/bench_otp12pf_mac.sh-550-  [[ "$mss" == "$REGISTERED_MSS" ]] \
scripts/bench_otp12pf_mac.sh:551:    || die "$(hname "$h"): the PATH to $peer negotiated MSS $mss, not the registered $REGISTERED_MSS. Both NICs can sit at MTU $REGISTERED_MTU while a 1500 hop between them clamps the segments (MSS 1448) — and THAT is the run pf-0 already measured as a DIFFERENT CONDITION."
scripts/bench_otp12pf_mac.sh-552-  [[ "$local_ip" == "$(hip "$h")" ]] \
scripts/bench_otp12pf_mac.sh-553-    || die "$(hname "$h"): the connection to $peer EGRESSED $local_ip, not the registered $(hip "$h") — the traffic is leaving on a DIFFERENT INTERFACE (this Mac has eight others, all at MTU 1500)."
scripts/bench_otp12pf_mac.sh-554-  log "  path from $(hname "$h") -> $(hname "$(other "$h")") ($peer): MSS $mss via $local_ip — the fabric is jumbo end-to-end"
scripts/bench_otp12pf_mac.sh-555-}
scripts/bench_otp12pf_mac.sh-556-
scripts/bench_otp12pf_mac.sh:557:end_mss_gate() {   # SESSION-END policy: the data exists, so VOID it rather than die.
scripts/bench_otp12pf_mac.sh-558-  local h out mss local_ip
scripts/bench_otp12pf_mac.sh-559-  for h in n q; do
scripts/bench_otp12pf_mac.sh-560-    out="$(mss_read "$h")" || out=""
scripts/bench_otp12pf_mac.sh-561-    if [[ -z "$out" ]]; then
scripts/bench_otp12pf_mac.sh-562-      SESSION_VOID_REASON="the path MSS on $(hname "$h") could not be read at the END of the session — a session whose fabric cannot be confirmed at both ends cannot be graded"
scripts/bench_otp12pf_mac.sh-563-      return
scripts/bench_otp12pf_mac.sh-564-    fi
scripts/bench_otp12pf_mac.sh-565-    read -r mss local_ip <<<"$out"
--
scripts/bench_otp12pf_mac.sh-624-  if [[ ! "$probe" =~ ^[0-9]+\.?[0-9]*$ ]]; then
scripts/bench_otp12pf_mac.sh-625-    log "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — cannot certify drainage"
scripts/bench_otp12pf_mac.sh-626-    return 1
scripts/bench_otp12pf_mac.sh-627-  fi
scripts/bench_otp12pf_mac.sh-628-  if [[ "$h" == n ]]; then N_DISK="$dev"; else Q_DISK="$dev"; fi
scripts/bench_otp12pf_mac.sh-629-  log "  drain device on $(hname "$h"): $dev (backs $p, idle probe ${probe} MB/s)"
scripts/bench_otp12pf_mac.sh-630-}
scripts/bench_otp12pf_mac.sh-631-
scripts/bench_otp12pf_mac.sh:632:# --- the settle-gap bound (NOT a removal — a measured bound) -------------------
scripts/bench_otp12pf_mac.sh:633:# Between the client exiting and the fsync starting, the OS writes back dirty pages
scripts/bench_otp12pf_mac.sh-634-# FOR FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
scripts/bench_otp12pf_mac.sh-635-# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
scripts/bench_otp12pf_mac.sh:636:# ssh return-path difference, which is bounded by the round-trip time measured here.
scripts/bench_otp12pf_mac.sh-637-# It is NOT "removed by construction", and the pre-registration no longer says so.
scripts/bench_otp12pf_mac.sh-638-#
scripts/bench_otp12pf_mac.sh-639-# Timed in ONE process, for the same reason the transfer is. Bracketing each ssh
scripts/bench_otp12pf_mac.sh-640-# with two `python3 -c time.time()` calls would have charged it TWO interpreter
scripts/bench_otp12pf_mac.sh:641:# startups (~30 ms) and reported them as network latency — measured: it read 35 ms
scripts/bench_otp12pf_mac.sh-642-# for a round trip that is actually ~5 ms. The instrument's own bound would have
scripts/bench_otp12pf_mac.sh-643-# been wrong by 7x, in the direction that flatters nothing and confuses everything.
scripts/bench_otp12pf_mac.sh-644-SSH_RTT_MS=0
scripts/bench_otp12pf_mac.sh:645:measure_ssh_rtt() {
scripts/bench_otp12pf_mac.sh-646-  # A FAILED ssh must not contribute a plausible number (round-5 codex, MEDIUM): a
scripts/bench_otp12pf_mac.sh:647:  # fast-failing connection would report a small "bound" and flatter the settle claim.
scripts/bench_otp12pf_mac.sh-648-  SSH_RTT_MS="$(python3 -c '
scripts/bench_otp12pf_mac.sh-649-import statistics, subprocess, sys, time
scripts/bench_otp12pf_mac.sh-650-argv = sys.argv[1:]
scripts/bench_otp12pf_mac.sh-651-ts = []
scripts/bench_otp12pf_mac.sh-652-for _ in range(5):
scripts/bench_otp12pf_mac.sh-653-    t = time.monotonic()
scripts/bench_otp12pf_mac.sh-654-    rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
scripts/bench_otp12pf_mac.sh-655-    if rc != 0:
scripts/bench_otp12pf_mac.sh-656-        print("SSH-FAILED")
scripts/bench_otp12pf_mac.sh-657-        raise SystemExit
scripts/bench_otp12pf_mac.sh-658-    ts.append((time.monotonic() - t) * 1000.0)
scripts/bench_otp12pf_mac.sh-659-print(int(statistics.median(ts)))
scripts/bench_otp12pf_mac.sh-660-' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
scripts/bench_otp12pf_mac.sh:661:  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
scripts/bench_otp12pf_mac.sh-662-  local rtt_max=$(( SETTLE_MS / 4 ))
scripts/bench_otp12pf_mac.sh-663-  (( SSH_RTT_MS <= rtt_max )) \
scripts/bench_otp12pf_mac.sh:664:    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
scripts/bench_otp12pf_mac.sh:665:  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
scripts/bench_otp12pf_mac.sh-666-}
scripts/bench_otp12pf_mac.sh-667-
scripts/bench_otp12pf_mac.sh-668-# =============================================================================
scripts/bench_otp12pf_mac.sh-669-preflight() {
scripts/bench_otp12pf_mac.sh-670-  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
scripts/bench_otp12pf_mac.sh-671-  # judged on the FULL RANGE, which only WIDENS with n, so more pairs could never rescue an
scripts/bench_otp12pf_mac.sh-672-  # UNCLEAR rig or certify a control -- and if you already have an EFFECT you do not need
scripts/bench_otp12pf_mac.sh-673-  # it. Its p-hacking guard surface goes with it.
--
scripts/bench_otp12pf_mac.sh-718-      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
scripts/bench_otp12pf_mac.sh-719-        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
scripts/bench_otp12pf_mac.sh-720-    done
scripts/bench_otp12pf_mac.sh-721-    link_gate "$h"
scripts/bench_otp12pf_mac.sh-722-    topology_gate "$h"          # the NIC: MTU + negotiated media (never checked before r11)
scripts/bench_otp12pf_mac.sh-723-    mss_gate "$h"               # the PATH: the MSS the two kernels actually agreed
scripts/bench_otp12pf_mac.sh-724-    resolve_disk "$h" || die "$(hname "$h"): cannot establish the drain device — refusing"
scripts/bench_otp12pf_mac.sh-725-  done
scripts/bench_otp12pf_mac.sh:726:  measure_ssh_rtt
scripts/bench_otp12pf_mac.sh:727:  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
scripts/bench_otp12pf_mac.sh-728-  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
scripts/bench_otp12pf_mac.sh-729-}
scripts/bench_otp12pf_mac.sh-730-
scripts/bench_otp12pf_mac.sh-731-write_manifest() {
scripts/bench_otp12pf_mac.sh-732-  local f="$OUT_DIR/staging-manifest.txt" h nb nd qb qd vh th
scripts/bench_otp12pf_mac.sh-733-  # Hashes computed FIRST, in the caller's shell: `die` inside $(...) exits only the
scripts/bench_otp12pf_mac.sh-734-  # subshell, so the old code wrote an EMPTY hash and called it provenance.
scripts/bench_otp12pf_mac.sh-735-  nb="$(sha256_of n "$N_BLIT")"   || die "nagatha: cannot hash $N_BLIT"
scripts/bench_otp12pf_mac.sh-736-  nd="$(sha256_of n "$N_DAEMON")" || die "nagatha: cannot hash $N_DAEMON"
scripts/bench_otp12pf_mac.sh-737-  qb="$(sha256_of q "$Q_BLIT")"   || die "q: cannot hash $Q_BLIT"
scripts/bench_otp12pf_mac.sh-738-  qd="$(sha256_of q "$Q_DAEMON")" || die "q: cannot hash $Q_DAEMON"
scripts/bench_otp12pf_mac.sh-739-  vh="$(shasum -a 256 "$VERDICT_PY" | cut -d' ' -f1)"
scripts/bench_otp12pf_mac.sh-740-  th="$(shasum -a 256 "$VERDICT_TEST" | cut -d' ' -f1)"
scripts/bench_otp12pf_mac.sh-741-  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
scripts/bench_otp12pf_mac.sh-742-    echo "# verdict_sha256=$vh verdict_test_sha256=$th"   # the engine grades separately: hash it too
scripts/bench_otp12pf_mac.sh:743:    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
scripts/bench_otp12pf_mac.sh-744-    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
scripts/bench_otp12pf_mac.sh-745-    echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
scripts/bench_otp12pf_mac.sh-746-    # THE FABRIC, RECORDED — not just gated. pf-0's whole finding turns on which MTU a
scripts/bench_otp12pf_mac.sh-747-    # session ran at; a session that does not record its own fabric cannot be re-read later.
scripts/bench_otp12pf_mac.sh-748-    # Measured with the same single reader the gates use, never a second copy of the probe.
scripts/bench_otp12pf_mac.sh-749-    echo "# fabric_registered: nic_mtu=$REGISTERED_MTU media=$REGISTERED_MEDIA mss=$REGISTERED_MSS"
scripts/bench_otp12pf_mac.sh:750:    echo "# fabric_measured: nagatha=[$(mss_read n)] q=[$(mss_read q)]  (mss local_ip)"
scripts/bench_otp12pf_mac.sh-751-    echo "# cells=$CELLS"
scripts/bench_otp12pf_mac.sh-752-    echo "host,role,sha,sha256,path"
scripts/bench_otp12pf_mac.sh-753-    echo "nagatha,client,$EXPECT_SHA,$nb,$N_BLIT"
scripts/bench_otp12pf_mac.sh-754-    echo "nagatha,daemon,$EXPECT_SHA,$nd,$N_DAEMON"
scripts/bench_otp12pf_mac.sh-755-    echo "q,client,$EXPECT_SHA,$qb,$Q_BLIT"
scripts/bench_otp12pf_mac.sh-756-    echo "q,daemon,$EXPECT_SHA,$qd,$Q_DAEMON"; } > "$f"
scripts/bench_otp12pf_mac.sh-757-  log "staging manifest recorded (harness + verdict-engine + 4 binary hashes, every threshold)"
scripts/bench_otp12pf_mac.sh-758-}
--
scripts/bench_otp12pf_mac.sh-765-  p="$(hrun "$h" "command -v python3" | nocr)" || p=""
scripts/bench_otp12pf_mac.sh-766-  if [[ "$p" != /* ]]; then
scripts/bench_otp12pf_mac.sh-767-    log "$(hname "$h"): cannot resolve an absolute python3 (got '$p')"; return 1
scripts/bench_otp12pf_mac.sh-768-  fi
scripts/bench_otp12pf_mac.sh-769-  if ! hrun "$h" "test -x '$p'"; then
scripts/bench_otp12pf_mac.sh-770-    log "$(hname "$h"): python3 at '$p' is not executable"; return 1
scripts/bench_otp12pf_mac.sh-771-  fi
scripts/bench_otp12pf_mac.sh-772-  if [[ "$h" == n ]]; then N_PY="$p"; else Q_PY="$p"; fi
scripts/bench_otp12pf_mac.sh:773:  log "  python3 on $(hname "$h"): $p (absolute — a PATH entry or shell function cannot stand in for the interpreter that MEASURES the settle)"
scripts/bench_otp12pf_mac.sh-774-}
scripts/bench_otp12pf_mac.sh-775-
scripts/bench_otp12pf_mac.sh-776-N_PID=""; Q_PID=""; TEARDOWN_FAILED=0
scripts/bench_otp12pf_mac.sh-777-daemon_start() {
scripts/bench_otp12pf_mac.sh-778-  local h="$1" cfg mod bin pid
scripts/bench_otp12pf_mac.sh-779-  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
scripts/bench_otp12pf_mac.sh-780-  # The daemon's OWN pid, from $! — not `pgrep | head -1`, which picks the first of
scripts/bench_otp12pf_mac.sh-781-  # whatever happens to be running.
--
scripts/bench_otp12pf_mac.sh-784-nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
scripts/bench_otp12pf_mac.sh-785-echo \"P:\$!:P\"" | nocr | sed -n 's/.*P:\([0-9][0-9]*\):P.*/\1/p' | head -1)"
scripts/bench_otp12pf_mac.sh-786-  [[ "$pid" =~ ^[0-9]+$ ]] || die "$(hname "$h"): daemon did not report a pid (see $mod/mm-daemon.log)"
scripts/bench_otp12pf_mac.sh-787-  # OWN THE PID BEFORE VALIDATING IT (round-5 codex, MEDIUM): the old code stored it
scripts/bench_otp12pf_mac.sh-788-  # only AFTER the alive/listening checks, so a daemon that started but failed
scripts/bench_otp12pf_mac.sh-789-  # validation was `die`d on while the EXIT trap did not yet know its pid — leaking a
scripts/bench_otp12pf_mac.sh-790-  # live daemon holding the port for the next session to trip over.
scripts/bench_otp12pf_mac.sh-791-  if [[ "$h" == n ]]; then N_PID="$pid"; else Q_PID="$pid"; fi
scripts/bench_otp12pf_mac.sh:792:  sleep 2
scripts/bench_otp12pf_mac.sh-793-  hrun "$h" "ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon" \
scripts/bench_otp12pf_mac.sh-794-    || die "$(hname "$h"): daemon pid $pid is not alive (see $mod/mm-daemon.log)"
scripts/bench_otp12pf_mac.sh:795:  # ALIVE is not SERVING: it must hold the port we are about to measure through.
scripts/bench_otp12pf_mac.sh-796-  hrun "$h" "lsof -nP -a -p $pid -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1" \
scripts/bench_otp12pf_mac.sh-797-    || die "$(hname "$h"): daemon pid $pid is NOT LISTENING on $PORT (see $mod/mm-daemon.log)"
scripts/bench_otp12pf_mac.sh-798-  log "$(hname "$h") daemon up (pid $pid, listening) on $(hip "$h"):$PORT"
scripts/bench_otp12pf_mac.sh-799-}
scripts/bench_otp12pf_mac.sh-800-# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
scripts/bench_otp12pf_mac.sh-801-# handshake reached some listener's backlog — not that the daemon speaks blit).
scripts/bench_otp12pf_mac.sh-802-smoke() {
scripts/bench_otp12pf_mac.sh-803-  local h="$1" o
--
scripts/bench_otp12pf_mac.sh-810-  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
scripts/bench_otp12pf_mac.sh-811-  log "smoke ok: $(hname "$h") daemon serves blit"
scripts/bench_otp12pf_mac.sh-812-}
scripts/bench_otp12pf_mac.sh-813-daemon_stop() {
scripts/bench_otp12pf_mac.sh-814-  local h="$1" pid state
scripts/bench_otp12pf_mac.sh-815-  if [[ "$h" == n ]]; then pid="$N_PID"; else pid="$Q_PID"; fi
scripts/bench_otp12pf_mac.sh-816-  [[ -n "$pid" ]] || return 0
scripts/bench_otp12pf_mac.sh-817-  hrun "$h" "kill $pid 2>/dev/null || true
scripts/bench_otp12pf_mac.sh:818:for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done
scripts/bench_otp12pf_mac.sh:819:if ps -p $pid >/dev/null 2>&1; then kill -9 $pid 2>/dev/null || true; sleep 1; fi" >/dev/null 2>&1 || true
scripts/bench_otp12pf_mac.sh-820-  # A teardown that cannot be VERIFIED is a failure, not a success. The old probe
scripts/bench_otp12pf_mac.sh-821-  # called a FAILED ssh "GONE".
scripts/bench_otp12pf_mac.sh-822-  # A BROKEN `ps` probe is not "GONE" (round-10 codex). The sentinel must come back, or the
scripts/bench_otp12pf_mac.sh-823-  # teardown is unverified and the session says so.
scripts/bench_otp12pf_mac.sh-824-  # `ps -p`: 0 = the pid EXISTS, 1 = it does NOT, anything else = THE PROBE IS BROKEN. The
scripts/bench_otp12pf_mac.sh-825-  # old form mapped every non-zero status onto "GONE" (round-11 codex, MEDIUM: r10 fixed the
scripts/bench_otp12pf_mac.sh-826-  # ssh-failure half and left the rc half), so a `ps` that could not run at all -- no PATH, a
scripts/bench_otp12pf_mac.sh-827-  # full process table -- certified the daemon dead and left it holding the port.
--
scripts/bench_otp12pf_mac.sh-889-    DRAIN-TIMEOUT)     echo DRAIN-TIMEOUT ;;
scripts/bench_otp12pf_mac.sh-890-    *)                 echo DRAIN-ERROR ;;
scripts/bench_otp12pf_mac.sh-891-  esac
scripts/bench_otp12pf_mac.sh-892-}
scripts/bench_otp12pf_mac.sh-893-prep_run() {   # $1 = dest host
scripts/bench_otp12pf_mac.sh-894-  local dh="$1" cn=ok cq=ok out
scripts/bench_otp12pf_mac.sh-895-  # Purge BOTH ends first — the purge itself dirties the disk, so a drain certified
scripts/bench_otp12pf_mac.sh-896-  # BEFORE it proves nothing.
scripts/bench_otp12pf_mac.sh:897:  # `sync; purge` reported only PURGE's status — a failed sync then read as a clean cold
scripts/bench_otp12pf_mac.sh-898-  # cache (round-10 codex). Both must succeed.
scripts/bench_otp12pf_mac.sh:899:  hrun n "sync && sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
scripts/bench_otp12pf_mac.sh:900:  hrun q "sync && sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
scripts/bench_otp12pf_mac.sh-901-  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
scripts/bench_otp12pf_mac.sh-902-  else RUN_COLD="COLD-FAIL(nagatha=$cn;q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi   # ';' not ',': this lands in a CSV
scripts/bench_otp12pf_mac.sh-903-  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
scripts/bench_otp12pf_mac.sh-904-  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
scripts/bench_otp12pf_mac.sh-905-  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
scripts/bench_otp12pf_mac.sh-906-}
scripts/bench_otp12pf_mac.sh-907-
scripts/bench_otp12pf_mac.sh-908-# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
scripts/bench_otp12pf_mac.sh-909-RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
scripts/bench_otp12pf_mac.sh:910:fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
scripts/bench_otp12pf_mac.sh-911-  local out
scripts/bench_otp12pf_mac.sh-912-  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
scripts/bench_otp12pf_mac.sh-913-  #
scripts/bench_otp12pf_mac.sh:914:  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
scripts/bench_otp12pf_mac.sh:915:  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
scripts/bench_otp12pf_mac.sh:916:  # RAN. Round 6 then found the repair was still not provable: `sleep` is
scripts/bench_otp12pf_mac.sh-917-  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
scripts/bench_otp12pf_mac.sh:918:  # counted files — so a no-op `sleep` would pass while the log narrated "settle
scripts/bench_otp12pf_mac.sh:919:  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
scripts/bench_otp12pf_mac.sh-920-  #
scripts/bench_otp12pf_mac.sh:921:  # A protection that cannot be OBSERVED is not a protection. The settle now happens
scripts/bench_otp12pf_mac.sh-922-  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
scripts/bench_otp12pf_mac.sh:923:  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
scripts/bench_otp12pf_mac.sh-924-  # to shadow, no exit status left to discard, and no narration left to trust.
scripts/bench_otp12pf_mac.sh-925-  out="$(hrun "$1" "$(hpy "$1") - '$SETTLE_SEC' '$2' <<'PYEOF'
scripts/bench_otp12pf_mac.sh-926-import os, sys, time
scripts/bench_otp12pf_mac.sh:927:settle = float(sys.argv[1])
scripts/bench_otp12pf_mac.sh-928-p = sys.argv[2]
scripts/bench_otp12pf_mac.sh-929-t0 = time.monotonic()
scripts/bench_otp12pf_mac.sh:930:time.sleep(settle)
scripts/bench_otp12pf_mac.sh:931:settled_ms = int((time.monotonic() - t0) * 1000)
scripts/bench_otp12pf_mac.sh-932-if not os.path.isdir(p):
scripts/bench_otp12pf_mac.sh:933:    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
scripts/bench_otp12pf_mac.sh-934-    raise SystemExit
scripts/bench_otp12pf_mac.sh-935-t = time.monotonic()
scripts/bench_otp12pf_mac.sh-936-files = 0
scripts/bench_otp12pf_mac.sh-937-nbytes = 0
scripts/bench_otp12pf_mac.sh-938-for root, _d, fs in os.walk(p):
scripts/bench_otp12pf_mac.sh-939-    for name in fs:
scripts/bench_otp12pf_mac.sh-940-        fp = os.path.join(root, name)
scripts/bench_otp12pf_mac.sh-941-        nbytes += os.path.getsize(fp)
scripts/bench_otp12pf_mac.sh-942-        fd = os.open(fp, os.O_RDONLY)
scripts/bench_otp12pf_mac.sh:943:        os.fsync(fd)
scripts/bench_otp12pf_mac.sh-944-        os.close(fd)
scripts/bench_otp12pf_mac.sh-945-        files += 1
scripts/bench_otp12pf_mac.sh:946:print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
scripts/bench_otp12pf_mac.sh-947-PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
scripts/bench_otp12pf_mac.sh-948-  echo "${out:-NA 0 0 0}"
scripts/bench_otp12pf_mac.sh-949-}
scripts/bench_otp12pf_mac.sh:950:# The settle actually elapsed, on the destination's own clock. Anything else voids.
scripts/bench_otp12pf_mac.sh:951:settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
scripts/bench_otp12pf_mac.sh-952-
scripts/bench_otp12pf_mac.sh-953-# --- one timed run ------------------------------------------------------------
scripts/bench_otp12pf_mac.sh:954:# The ssh dispatch, measured RIGHT NOW rather than assumed from preflight. The residual
scripts/bench_otp12pf_mac.sh:955:# free-writeback asymmetry between the arms is bounded BY this number, and a bound measured
scripts/bench_otp12pf_mac.sh-956-# once at the start is not a bound on a run taken twenty minutes later (round-10 codex).
scripts/bench_otp12pf_mac.sh-957-RUN_RTT=0
scripts/bench_otp12pf_mac.sh-958-rtt_now() {
scripts/bench_otp12pf_mac.sh-959-  local v
scripts/bench_otp12pf_mac.sh-960-  v="$(python3 -c '
scripts/bench_otp12pf_mac.sh-961-import statistics, subprocess, sys, time
scripts/bench_otp12pf_mac.sh-962-argv = sys.argv[1:]
scripts/bench_otp12pf_mac.sh-963-ts = []
--
scripts/bench_otp12pf_mac.sh-975-RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
scripts/bench_otp12pf_mac.sh-976-timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
scripts/bench_otp12pf_mac.sh-977-  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
scripts/bench_otp12pf_mac.sh-978-  bin="$(hblit "$ih")"
scripts/bench_otp12pf_mac.sh-979-  prep_run "$dh"
scripts/bench_otp12pf_mac.sh-980-  RUN_RTT="$(rtt_now)"
scripts/bench_otp12pf_mac.sh-981-  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
scripts/bench_otp12pf_mac.sh-982-  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
scripts/bench_otp12pf_mac.sh:983:  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
scripts/bench_otp12pf_mac.sh-984-  RUN_VALID=yes
scripts/bench_otp12pf_mac.sh-985-  wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
scripts/bench_otp12pf_mac.sh:986:  # The equal settle is the ONLY thing standing between this rig and a free-writeback
scripts/bench_otp12pf_mac.sh-987-  # artifact that REVERSES SIGN WITH DIRECTION — i.e. that can manufacture P1 out of
scripts/bench_otp12pf_mac.sh-988-  # nothing. It has already been silently dead once. If it did not measurably elapse,
scripts/bench_otp12pf_mac.sh-989-  # the row is not a fast row; it is a VOID row.
scripts/bench_otp12pf_mac.sh:990:  if ! settle_ok "$RUN_SETTLED"; then
scripts/bench_otp12pf_mac.sh:991:    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
scripts/bench_otp12pf_mac.sh-992-    RUN_VALID=no
scripts/bench_otp12pf_mac.sh-993-  fi
scripts/bench_otp12pf_mac.sh-994-  if [[ "$RUN_FLUSH" == NA ]]; then
scripts/bench_otp12pf_mac.sh:995:    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
scripts/bench_otp12pf_mac.sh-996-    RUN_VALID=no; RUN_FLUSH=0
scripts/bench_otp12pf_mac.sh-997-  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
scripts/bench_otp12pf_mac.sh-998-    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
scripts/bench_otp12pf_mac.sh-999-    RUN_VALID=no
scripts/bench_otp12pf_mac.sh-1000-  fi
scripts/bench_otp12pf_mac.sh-1001-  # A negative or absurd transfer time means the CLOCK failed, not that the transfer
scripts/bench_otp12pf_mac.sh-1002-  # was fast. It must never enter the data.
scripts/bench_otp12pf_mac.sh-1003-  if [[ ! "$RUN_MS" =~ ^[0-9]+$ ]] || (( RUN_MS < 1 )); then
scripts/bench_otp12pf_mac.sh-1004-    log "  VOID: transfer timer returned '$RUN_MS' — the clock failed (round 2's killer). NOT a fast run."
scripts/bench_otp12pf_mac.sh-1005-    RUN_VALID=no; RUN_MS=0
scripts/bench_otp12pf_mac.sh-1006-  fi
scripts/bench_otp12pf_mac.sh-1007-  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
scripts/bench_otp12pf_mac.sh-1008-  # The dispatch bound, enforced on THIS pair rather than assumed from preflight.
scripts/bench_otp12pf_mac.sh-1009-  local rtt_max=$(( SETTLE_MS / 4 ))
scripts/bench_otp12pf_mac.sh-1010-  if [[ ! "$RUN_RTT" =~ ^[0-9]+$ ]] || (( RUN_RTT > rtt_max )); then
scripts/bench_otp12pf_mac.sh:1011:    log "  VOID: ssh dispatch measured ${RUN_RTT}ms (max ${rtt_max}ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible"
scripts/bench_otp12pf_mac.sh-1012-    RUN_VALID=no
scripts/bench_otp12pf_mac.sh-1013-  fi
scripts/bench_otp12pf_mac.sh-1014-  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
scripts/bench_otp12pf_mac.sh-1015-}
scripts/bench_otp12pf_mac.sh-1016-
scripts/bench_otp12pf_mac.sh-1017-# --- arms ---------------------------------------------------------------------
scripts/bench_otp12pf_mac.sh:1018:# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
scripts/bench_otp12pf_mac.sh-1019-# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
scripts/bench_otp12pf_mac.sh-1020-# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
scripts/bench_otp12pf_mac.sh-1021-# above is what makes a wrong path fatal instead of silently free.
scripts/bench_otp12pf_mac.sh-1022-CUR_W=""; CUR_FLAG=""
scripts/bench_otp12pf_mac.sh-1023-arm_srcinit() {
scripts/bench_otp12pf_mac.sh-1024-  local sh="$1" dh="$2" run="$3"
scripts/bench_otp12pf_mac.sh-1025-  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
scripts/bench_otp12pf_mac.sh-1026-            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
--
scripts/bench_otp12pf_mac.sh-1086-  if (( slot % 2 )); then order="A B"; else order="B A"; fi
scripts/bench_otp12pf_mac.sh-1087-  for arm in $order; do
scripts/bench_otp12pf_mac.sh-1088-    if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
scripts/bench_otp12pf_mac.sh-1089-    rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
scripts/bench_otp12pf_mac.sh-1090-    if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"; else arm_destinit "$sh" "$dh" "$run"; fi
scripts/bench_otp12pf_mac.sh-1091-    [[ "$RUN_VALID" == yes ]] || pair=no
scripts/bench_otp12pf_mac.sh-1092-    local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_RTT,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
scripts/bench_otp12pf_mac.sh-1093-    if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
scripts/bench_otp12pf_mac.sh:1094:    log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (fsync ${RUN_FLUSH}ms, settle ${RUN_SETTLED}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
scripts/bench_otp12pf_mac.sh-1095-  done
scripts/bench_otp12pf_mac.sh-1096-  echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
scripts/bench_otp12pf_mac.sh-1097-  if [[ "$pair" == yes ]]; then
scripts/bench_otp12pf_mac.sh-1098-    CELL_VALID[$i]=$(( ${CELL_VALID[$i]:-0} + 1 ))
scripts/bench_otp12pf_mac.sh-1099-    PAIR_OK=1
scripts/bench_otp12pf_mac.sh-1100-    return 0
scripts/bench_otp12pf_mac.sh-1101-  fi
scripts/bench_otp12pf_mac.sh-1102-  log "  $cell: pair at slot $slot VOIDED"
--
scripts/bench_otp12pf_mac.sh-1215-    SELFTEST_FIRED=$(( SELFTEST_FIRED + 1 ))
scripts/bench_otp12pf_mac.sh-1216-    log "  [FIRED]  $label — the gate REFUSED a genuinely unmet condition. It works."
scripts/bench_otp12pf_mac.sh-1217-  fi
scripts/bench_otp12pf_mac.sh-1218-  # Never hide what the gate said — including its own evidence on success.
scripts/bench_otp12pf_mac.sh-1219-  [[ -n "$err" ]] && sed 's/^/           | /' <<<"$err" | tee -a "$OUT_DIR/bench.log" >&2
scripts/bench_otp12pf_mac.sh-1220-  return 0
scripts/bench_otp12pf_mac.sh-1221-}
scripts/bench_otp12pf_mac.sh-1222-
scripts/bench_otp12pf_mac.sh:1223:# The fsync/settle path, exercised for real on a throwaway tree. It is the durability
scripts/bench_otp12pf_mac.sh:1224:# measurement AND the equal-settle window — the two things that once manufactured P1 —
scripts/bench_otp12pf_mac.sh-1225-# and the self-test never touched them.
scripts/bench_otp12pf_mac.sh:1226:selftest_fsync() {
scripts/bench_otp12pf_mac.sh:1227:  local h="$1" d ms files bytes settled
scripts/bench_otp12pf_mac.sh-1228-  d="$(hmod "$h")/selftest_${SESSION_TAG}"
scripts/bench_otp12pf_mac.sh-1229-  hrun "$h" "rm -rf '$d' && mkdir -p '$d' && printf 'aaaa' > '$d/a' && printf 'bb' > '$d/b'" \
scripts/bench_otp12pf_mac.sh:1230:    || { log "  [BROKEN] fsync/settle — cannot stage a probe tree"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); return 1; }
scripts/bench_otp12pf_mac.sh:1231:  read -r ms files bytes settled <<<"$(fsync_tree "$h" "$d")"
scripts/bench_otp12pf_mac.sh-1232-  hrun "$h" "rm -rf '$d'" >/dev/null 2>&1 || true
scripts/bench_otp12pf_mac.sh-1233-  if [[ "$ms" == NA || "$files" != 2 || "$bytes" != 6 ]]; then
scripts/bench_otp12pf_mac.sh:1234:    log "  [BROKEN] fsync/settle — walk returned ms=$ms files=$files bytes=$bytes, want 2 files / 6 bytes"
scripts/bench_otp12pf_mac.sh-1235-    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
scripts/bench_otp12pf_mac.sh-1236-  fi
scripts/bench_otp12pf_mac.sh-1237-  # THE SETTLE MUST BE PROVED, NOT NARRATED (round-6, both reviewers). The old check
scripts/bench_otp12pf_mac.sh:1238:  # counted files and then LOGGED "settle included" — which is a sentence, not an
scripts/bench_otp12pf_mac.sh:1239:  # assertion. It would have passed with the settle stone dead, which is precisely how
scripts/bench_otp12pf_mac.sh:1240:  # the settle stayed dead for three revisions.
scripts/bench_otp12pf_mac.sh:1241:  if ! settle_ok "$settled"; then
scripts/bench_otp12pf_mac.sh:1242:    log "  [BROKEN] fsync/settle — THE SETTLE DID NOT ELAPSE: measured ${settled}ms, want >= ${SETTLE_MS}ms"
scripts/bench_otp12pf_mac.sh-1243-    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
scripts/bench_otp12pf_mac.sh-1244-  fi
scripts/bench_otp12pf_mac.sh:1245:  log "  [OK]     fsync/settle — 2 files/6 bytes walked in ${ms}ms; settle MEASURED at ${settled}ms (>= ${SETTLE_MS}ms), counts VERIFIED"
scripts/bench_otp12pf_mac.sh-1246-}
scripts/bench_otp12pf_mac.sh-1247-
scripts/bench_otp12pf_mac.sh-1248-selftest() {
scripts/bench_otp12pf_mac.sh-1249-  local h
scripts/bench_otp12pf_mac.sh-1250-  log "SELFTEST — exercising the gates for real. No transfer, NO DATA."
scripts/bench_otp12pf_mac.sh-1251-  log "instrument: harness=$HARNESS_SHA256"
scripts/bench_otp12pf_mac.sh-1252-  log "--- the verdict engine's own guard test (incl. mutation proof) ---"
scripts/bench_otp12pf_mac.sh-1253-  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
--
scripts/bench_otp12pf_mac.sh-1288-    # DRAIN-TIMEOUT is a genuinely busy disk (the gate WORKING); DRAIN-ERROR is a blind
scripts/bench_otp12pf_mac.sh-1289-    # probe. Scoring them the same made the classification untrustworthy (grok r6, F7).
scripts/bench_otp12pf_mac.sh-1290-    local dr; dr="$(drain_host "$h")"
scripts/bench_otp12pf_mac.sh-1291-    case "$dr" in
scripts/bench_otp12pf_mac.sh-1292-      drained*)      log "  [OK]     drain loop    ($dr)" ;;
scripts/bench_otp12pf_mac.sh-1293-      DRAIN-TIMEOUT) log "  [FIRED]  drain loop    — the disk is genuinely busy; the gate would void the pair"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
scripts/bench_otp12pf_mac.sh-1294-      *)             log "  [BROKEN] drain loop    — the probe could not answer ('$dr')"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
scripts/bench_otp12pf_mac.sh-1295-    esac
scripts/bench_otp12pf_mac.sh:1296:    selftest_fsync "$h"
scripts/bench_otp12pf_mac.sh-1297-    log "  [--]     mac parse (no gawk strtonum): $(hmac "$h") -> $(hmac "$h" | norm_mac)"
scripts/bench_otp12pf_mac.sh-1298-  done
scripts/bench_otp12pf_mac.sh-1299-  SESSION_VOID_REASON=""; end_load_gate
scripts/bench_otp12pf_mac.sh-1300-  if [[ -z "$SESSION_VOID_REASON" ]]; then
scripts/bench_otp12pf_mac.sh-1301-    log "  [OK]     end-load gate (both Macs under $LOAD_MAX; it CAN void a session)"
scripts/bench_otp12pf_mac.sh-1302-  elif [[ "$SESSION_VOID_REASON" == *"could not be read"* ]]; then
scripts/bench_otp12pf_mac.sh-1303-    # An UNREADABLE end-load is a blind probe, not a busy machine (grok r6, F7).
scripts/bench_otp12pf_mac.sh-1304-    log "  [BROKEN] end-load gate — $SESSION_VOID_REASON"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1))
scripts/bench_otp12pf_mac.sh-1305-  else
scripts/bench_otp12pf_mac.sh-1306-    log "  [FIRED]  end-load gate — $SESSION_VOID_REASON"; SELFTEST_FIRED=$((SELFTEST_FIRED+1))
scripts/bench_otp12pf_mac.sh-1307-  fi
scripts/bench_otp12pf_mac.sh-1308-  # The END-of-session fabric re-check, exercised for real. Same three-way scoring: a probe
scripts/bench_otp12pf_mac.sh-1309-  # that cannot answer is BLIND, a fabric that genuinely changed is the gate WORKING.
scripts/bench_otp12pf_mac.sh:1310:  SESSION_VOID_REASON=""; end_mss_gate
scripts/bench_otp12pf_mac.sh-1311-  if [[ -z "$SESSION_VOID_REASON" ]]; then
scripts/bench_otp12pf_mac.sh-1312-    log "  [OK]     end-fabric gate (MSS $REGISTERED_MSS on both ends; it CAN void a session)"
scripts/bench_otp12pf_mac.sh-1313-  elif [[ "$SESSION_VOID_REASON" == *"could not be read"* ]]; then
scripts/bench_otp12pf_mac.sh-1314-    log "  [BROKEN] end-fabric gate — $SESSION_VOID_REASON"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1))
scripts/bench_otp12pf_mac.sh-1315-  else
scripts/bench_otp12pf_mac.sh-1316-    log "  [FIRED]  end-fabric gate — $SESSION_VOID_REASON"; SELFTEST_FIRED=$((SELFTEST_FIRED+1))
scripts/bench_otp12pf_mac.sh-1317-  fi
scripts/bench_otp12pf_mac.sh-1318-  SESSION_VOID_REASON=""
scripts/bench_otp12pf_mac.sh:1319:  measure_ssh_rtt
scripts/bench_otp12pf_mac.sh-1320-  log ""
scripts/bench_otp12pf_mac.sh-1321-  log "SELFTEST: $SELFTEST_FIRED gate(s) refused a genuinely unmet condition; $SELFTEST_BROKEN blind."
scripts/bench_otp12pf_mac.sh-1322-  log "NOT exercised here (they need a real transfer): daemon start/lsof/teardown, the"
scripts/bench_otp12pf_mac.sh-1323-  log "smoke transfer, the ABBA pair loop, pair-voiding, and the manifest. PREFLIGHT_ONLY=1"
scripts/bench_otp12pf_mac.sh-1324-  log "covers the manifest and the build-provenance gates. This self-test does NOT claim"
scripts/bench_otp12pf_mac.sh-1325-  log "to run every gate — the previous one did, and it was not true."
scripts/bench_otp12pf_mac.sh-1326-  log "THIS IS NOT CLEARANCE TO TAKE DATA. The review is."
scripts/bench_otp12pf_mac.sh-1327-  if (( SELFTEST_BROKEN > 0 )); then
--
scripts/bench_otp12pf_mac.sh-1341-  fi
scripts/bench_otp12pf_mac.sh-1342-  preflight
scripts/bench_otp12pf_mac.sh-1343-  write_manifest
scripts/bench_otp12pf_mac.sh-1344-  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
scripts/bench_otp12pf_mac.sh-1345-    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
scripts/bench_otp12pf_mac.sh-1346-    exit 0
scripts/bench_otp12pf_mac.sh-1347-  fi
scripts/bench_otp12pf_mac.sh-1348-  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
scripts/bench_otp12pf_mac.sh:1349:  echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,rtt_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
scripts/bench_otp12pf_mac.sh-1350-  echo "cell,pairs_attempted,complete" > "$META"
scripts/bench_otp12pf_mac.sh-1351-  daemon_start n; daemon_start q
scripts/bench_otp12pf_mac.sh-1352-  smoke n; smoke q
scripts/bench_otp12pf_mac.sh-1353-
scripts/bench_otp12pf_mac.sh-1354-  run_all_cells
scripts/bench_otp12pf_mac.sh-1355-
scripts/bench_otp12pf_mac.sh-1356-  # End-load AND end-fabric, BEFORE the verdict is computed. Both can VOID the session.
scripts/bench_otp12pf_mac.sh-1357-  # The fabric is re-proved at the END for the same reason pf-0 did it: a link that flapped
scripts/bench_otp12pf_mac.sh-1358-  # or re-routed mid-session (onto the 1GbE NIC, or a 1500 hop) would have carried some of
scripts/bench_otp12pf_mac.sh:1359:  # the timed windows, and a start-only check cannot see that. A gate measured once at the
scripts/bench_otp12pf_mac.sh-1360-  # start is not a gate on a run taken an hour later.
scripts/bench_otp12pf_mac.sh-1361-  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
scripts/bench_otp12pf_mac.sh-1362-  end_load_gate
scripts/bench_otp12pf_mac.sh:1363:  [[ -n "$SESSION_VOID_REASON" ]] || end_mss_gate
scripts/bench_otp12pf_mac.sh-1364-  if [[ -n "$SESSION_VOID_REASON" ]]; then
scripts/bench_otp12pf_mac.sh-1365-    log "ERROR: SESSION VOID — $SESSION_VOID_REASON"
scripts/bench_otp12pf_mac.sh-1366-    touch "$OUT_DIR/SESSION-VOID"
scripts/bench_otp12pf_mac.sh-1367-  fi
scripts/bench_otp12pf_mac.sh-1368-  compute_verdicts
scripts/bench_otp12pf_mac.sh-1369-  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
scripts/bench_otp12pf_mac.sh-1370-  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
scripts/bench_otp12pf_mac.sh-1371-  log "=== PAIRED STATS (the rule is graded on these) ==="

exec
/bin/zsh -lc "sed -n '1,360p' .review/results/macmac-r12.codex-design.md; sed -n '1,360p' .review/results/macmac-r12.grok-design.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
I'll treat this as a design challenge, not a rubber stamp: read the preregistration, harness, and verdict rule, run the selftests, then argue whether the Mac↔Mac run can actually answer the question.Continuing the harness (ABBA schedule and measurement path) and running the selftests.Running the verdict guard tests and the harness selftest, and probing a few design-risk spots.Checking a few more concrete failure modes that could produce a wrong verdict:## 1–2. Verdict on the question and the outcomes

**The experiment can answer the question it claims — and that question is the wrong next move.**

The only clean test of “can P1 occur with no Windows peer?” is a real same-OS pair that is not Linux↔Linux. Mac↔Mac over 10GbE is that cell. Loopback would not do (the parent plan already says a negative loopback is inconclusive). So as a *scientific* 2×2 fill, the *shape* of the question is fine.

As a *project* move, it is a detour.

What is already known:

| Pair | P1? |
|---|---|
| Linux↔Linux | No (8/8) |
| macOS↔Windows | Yes, four times, 1.24–1.39 |
| macOS↔macOS | Untested |

MTU is dead as a dominant cause. H1 names **blit code that also runs on macOS**, not “Windows.” P1 has **no escape hatch** on the books: fix to ≤1.10 on rig W, or the owner amends criterion 1. Nothing in this rig creates a waiver.

So:

| Outcome | Legitimate learning | Temptation you must not take | Changes next engineering action? |
|---|---|---|---|
| **REPRODUCES** | P1 can appear with no Windows peer **on this pair**. Not “Windows residue.” Code-level and macOS/APFS/host×role explanations stay open. Does **not** kill or confirm H1. | “H1 is confirmed” / “platform-general layout cost” / “skip Windows investigation” | **No.** Still pf-1 / fix on **rig W**, where release is blocked. |
| **DOES-NOT-REPRODUCE** | On **this** pair, an effect of size T was excluded with clean controls. Consistent with “Windows peer matters”; **does not prove** it (could be these disks, these Macs, this macOS). | “P1 is Windows-only, waive it” / “H1 confirmed” | **No.** P1 still fails on W. Still pf-1 on W (maybe with more weight on Windows-side accept). |
| **MIXED** | Host×role interaction on this pair; cannot attribute initiator cost. | Reading one direction as “P1” and ignoring the invert | **No.** Inconclusive; still W. |
| **INVERTED** | New finding (source-init slower). | “P1 absent” | **No.** New puzzle; P1 on W untouched. |
| **UNCLEAR** | Rig could not resolve ±T. | Treating as soft null | **No.** Fix noise or abandon; do not add pairs (rule forbids it). |
| **CONTROLS-NOT-CLEAN** | gRPC/large (or residual bias B) not clean enough to attribute TCP×mixed | “Controls dirty ⇒ P1 is rig-wide ⇒ not real on W” (W already has clean controls in the original data) | **No.** Re-quiet or stop. |
| **INCOMPLETE / RIG-VOID** | Session failed integrity | Anything causal | **No.** |

**No outcome changes the release-critical next step:** attribute and fix (or formally re-scope) P1 on the macOS↔Windows pair. Mac↔Mac only reweights priors inside pf-1, and only if it returns a *decisive* REPRODUCES / DOES-NOT-REPRODUCE. Under the rule’s own austerity, decisive outcomes are not the mode you should bet on (below).

The one non-engineering fork: a clean null, stacked on Linux null, would be the honest evidence package **if** the owner is weighing amending criterion 1. The prereg correctly says that decision does not exist and this run does not create it. If you are not about to have that conversation, the 2×2 is map-making while the ship is on fire.

**What I would do instead / first**

1. **pf-1 on rig W**, where the 25–38% effect is real: dial/accept inversion, no-resize, per-side dial-before-ACK ordering — the parent’s own H1 discriminators. That can change code.
2. **Cheap diagnostics on W**: spans on `SourceSockets` Dial/Accept, `add_dialed_stream`, dial-before-ACK; optional packet timing of accept/dial RTT under mixed TCP. Hours, not another review epoch.
3. **Only if** you need the platform map for a criterion conversation: a **minimal** Mac↔Mac TCP×mixed both directions (even informal n≈4) before this six-cell cathedral. If ratios sit at ~1.00 vs ~1.30, you already know; then either formalize or stop.

Eleven rounds, ~110 defects, **zero data** is not “we are careful.” It is a signal that the instrument became the project. Selftest and the verdict suite **pass now**. Further review rounds are how you never ship the measurement or the fix.

---

## 3. Design shape

**Controls (right idea, harsh implementation)**  
gRPC×mixed and TCP×large, both directions, match P1’s claimed specificity (TCP-only, mixed-only on W). Slot-major + rotated cell order correctly stops “controls certify a window they never shared.” Good.

**ABBA + slot-major**  
Sound for drift and shared transients. Caveat: void retries run **in place** before the next cell, so “same slot” is not “same wall-clock moment.” Session-level co-temporal is true; per-slot simultaneity is not. Acceptable if you do not overclaim.

**n = 8 and the full-range null**  
At n=8 the ≥95% order-statistic CI **is** `[min, max]` (coverage 99.2%). EFFECT and NONE both effectively require **every pair** to cooperate. That is deliberate and honest for equivalence — and it is **power-hostile**.

Rough session-level simulation (true +250 ms on both measurands; four null controls):

| Meas. pair σ | Ctrl pair σ | Rough outcome mix |
|---|---|---|
| 40 ms | 15 ms | ~92% REPRODUCES, ~8% CONTROLS-NOT-CLEAN |
| 60 ms | 20 ms | ~56% REPRODUCES, ~41% CONTROLS-NOT-CLEAN |
| 80 ms | 25 ms | ~17% REPRODUCES, ~78% CONTROLS-NOT-CLEAN |

Four independent full-range “NONE at T/2” controls: P(all clean) collapses once pair noise is ~20–25 ms (≈39% → ≈10%). Rig W already showed **~110 ms bimodality on a fast arm** between sessions; ABBA helps, but it does not make control ranges tiny by magic. **The design’s most likely “successful” execution may be CONTROLS-NOT-CLEAN** — which buys a re-run, not an answer.

**Missing / unseparated**

- Host×role is only split by running both data directions; nagatha vs `q` speed still confounds one-directional effects (prereg admits this; MIXED is the escape). Fine if you obey MIXED.
- No mid-session quiescence re-check (start + end load only). Residual.
- Δ_ref = 230 ms is **W’s** absolute effect. On faster Mac↔Mac arms, T often becomes `src/10` (stricter null, easier small relative EFFECT). Not wrong, but the null gets harder on a fast pair.
- Does not separate macOS-as-dest vs Windows-as-source (would need Mac↔Linux or the W inversion). Out of scope for the stated question; still the more diagnostic experiment for *mechanism*.

**Severity summary (design)**

| Sev | Finding |
|---|---|
| **HIGH** | Four controls × full-range NONE at **T/2** × n=8 makes **CONTROLS-NOT-CLEAN** a likely modal outcome under mild pair noise; then the session answers nothing. |
| **HIGH** | EFFECT also needs **min(d) ≥ T+B** (every pair). A real ~25% effect with one soft pair → UNCLEAR, not REPRODUCES. Conservative, but weak power. |
| **MEDIUM** | Slot-major + in-place retry ≠ per-slot contemporaneous controls. |
| **MEDIUM** | No TCP×small control (optional; W already says small is clean). |
| **LOW** | Δ_ref imported from another pairing; dual threshold is intentional, document when T is bar-limited vs Δ-limited. |

---

## 4. Decision rule

A statistician would recognize the pieces: **paired differences**, **exact sign/order-statistic CI on the median**, **range-based equivalence** (stricter than TOST on the median), **pre-registered threshold**, **control precondition**. They would **trust a NONE** as “no pair showed an effect near T” — stronger than “median is small,” and hard to game. They would **not** call it a standard 95% test of H₀: median = 0; they would call it a **deliberately austere decision procedure**.

What is sound:

- One T; four exhaustive cell states; no bar-in-the-loop; sign test reported only.
- B only hardens; MIXED on unhardened states (fixes noisier-rig → stronger-claim).
- B ≥ T/2 → refuse to grade (closes capped-T vs fractional-B dead zone).
- n exactly 8 (no trim); incomplete on wrong n.
- −src/11 for inverse 1.10 ratio symmetry — correct.

What is “too clever” in the bad sense:

- **B as max fractional range across controls**, then scaled onto the measurand, plus T/2 control certification, plus B ≥ T/2 session kill: each piece fixed a real past bug; **together they stack conservatism** until many honest quiet-ish sessions cannot speak.
- At n=8, “CI vs range” is the same two numbers; the split is future-proofing. Fine, slightly ornamental today.

Would I trust a null out of it? **Yes**, if I got one. Would I expect to get one on a live APFS/10GbE pair with purge/drain/fsync in the loop? **Often no** — UNCLEAR / CONTROLS-NOT-CLEAN instead. That is the rule working as a nervous instrument, not as a sharp classifier.

| Sev | Finding |
|---|---|
| **HIGH** | Rule is trustworthy when decisive; **under-powered for decisiveness** under plausible noise (see §3). |
| **MEDIUM** | Stacked conservatism (T/2 controls + B + B≥T/2 + full-range null) may be more complex than D-2026-07-14-3 “simplify” still wants. |
| **LOW** | EFFECT inclusive vs NONE strict at the boundary — conservative, asymmetric, OK. |

---

## 5. Correctness (harness / rule) — bugs that wrong the measurement or verdict

Selftest: **PASSED** (0 blind; quiescence correctly **FIRED** on nagatha while `codex` runs).  
Verdict: **40/40 cases, 19/19 mutations killed, 300/300 fuzz, n=8 CI≡range identity OK.**

I did **not** find a live “false REPRODUCES / false DOES-NOT-REPRODUCE” in the graded path comparable to the historical timer/settle/control-void blockers. Remaining issues are smaller or process-level.

| Sev | Where | Issue |
|---|---|---|
| **MEDIUM** | `bench_otp12pf_mac.sh` ~1118–1126, 1052–1063 | In-place void retries break the “controls share the measurand’s moment” story inside a slot. Can mis-fire CONTROLS clean while measurand retried under different load (or the reverse). |
| **MEDIUM** | `bench_otp12pf_mac.sh` 1322–1325; main path | Selftest **still does not** run daemon lifecycle, smoke, ABBA, or pair-void. First full path execution **is** the first data session. Historically this class is where defects hide. Not a known wrong formula; residual risk. |
| **MEDIUM** | `otp12pf_mac_verdict.py` 229–257 + control loop 305–323 | Not a code bug: full-range control NONE is so tight that **correct** code yields **frequent non-answers**. Wrong *utility*, not wrong arithmetic. |
| **LOW** | `bench_otp12pf_mac.sh` 1008–1013 vs preflight RTT | Per-pair RTT void is good; document already registers it. Residual free-writeback bound is measurement-backed, not proof of zero. |
| **LOW** | `otp12pf_mac_verdict_test.py` header vs body | Doc counts (34 cases / 14 mutations) drift vs 40 / 19. Guard is not vacuous; commentary is stale. |
| **LOW** | `bench_otp12pf_mac.sh` 1–23, 96–104 | Header still “round-11 / round-12 gate” narrative; process, not measurand. |
| **INFO** | Historical classes you named | Settle is now in-process and selftest-proved; constants pinned **after** override check; topology/MSS gated; timer single-process + gate. Those specific landmines look closed **as of this selftest**. |

I am **not** claiming the harness is bug-free. I am claiming: at this revision, the failure mode that should scare you most is **a clean, confident non-answer** (or a twelfth review), not an obvious arithmetic reverse of P1.

---

## Bottom line

This is a **well-built instrument pointed at a real but low-leverage question**. It will answer “Windows-free P1 on nagatha↔q?” only if noise is unusually kind; many runs will return CONTROLS-NOT-CLEAN / UNCLEAR and license nothing. When it is decisive, **you still do pf-1 on rig W**. The expensive thing left is not another design pass — it is either taking data once or admitting the 2×2 cell is optional science and moving to the pair that blocks release.

**DO NOT RUN IT — skip Mac↔Mac for now and spend the session on pf-1 / dial-accept diagnostics on the macOS↔Windows pair where P1 is known; no outcome of this rig changes that obligation, and the rule is likely to spend the run on CONTROLS-NOT-CLEAN.**

exec
/bin/zsh -lc "rg -n 'settle|SETTLE|writeback|free|re-meas|remeas|corrected|fsync|flush' .review/results/macmac-r12.codex-design.md .review/results/macmac-r12.grok-design.md scripts/bench_otp12_win.sh scripts/bench_otp12pf_mac.sh docs/plan/OTP12_PERF_FINDINGS.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/plan/OTP12_PERF_FINDINGS.md:15:factual claim was settled by *measurement* (the same-OS rig refuted a
docs/plan/OTP12_PERF_FINDINGS.md:36:**Sequencing (corrected 2026-07-13).** This doc originally deferred
docs/plan/OTP12_PERF_FINDINGS.md:73:escape** (corrected, review round 3; an earlier draft of this section
docs/plan/OTP12_PERF_FINDINGS.md:94:which host runs the client, and the flush method. So the supported
docs/plan/OTP12_PERF_FINDINGS.md:166:> destination's writeback was never paid; in the pull arm the initiator IS
docs/plan/OTP12_PERF_FINDINGS.md:167:> the destination, so it paid the full writeback. One arm was charged for
docs/plan/OTP12_PERF_FINDINGS.md:168:> durability the other got free — multi-second on skippy's ZFS — which
docs/plan/OTP12_PERF_FINDINGS.md:214:- **The same-platform loopback run is a ONE-WAY test** (corrected — an
docs/plan/OTP12_PERF_FINDINGS.md:252:range corrected again in round 3). `push_grpc_small` new-vs-old,
docs/plan/OTP12_PERF_FINDINGS.md:359:The resolution — re-record each rig's baseline at MTU 9000 and re-freeze —
docs/plan/OTP12_PERF_FINDINGS.md:371:unaffected. The remaining constraints (`BASELINE_SUMMARY` stays override-free and
docs/plan/OTP12_PERF_FINDINGS.md:399:  claim. Only the dial/accept inversion counterfactual in pf-1 can settle H1.
docs/plan/OTP12_PERF_FINDINGS.md:422:  per-file flush; per-file progress emission to the served push
docs/plan/OTP12_PERF_FINDINGS.md:424:  the same served sink, so fsync/flush policy and progress emission are
docs/plan/OTP12_PERF_FINDINGS.md:440:  opened at one stream (after its 128-file early flush) then resized
docs/plan/OTP12_PERF_FINDINGS.md:465:  **Historical control — corrected (review round 3): test the EXECUTED
docs/plan/OTP12_PERF_FINDINGS.md:540:   double-digit share of the measured gap). Experiments, corrected per
docs/plan/OTP12_PERF_FINDINGS.md:542:   environmental-only (it cannot attribute code); (b) the flush/
docs/plan/OTP12_PERF_FINDINGS.md:703:  (corrected, review round 2 — "every row" was impossible: the
docs/plan/OTP12_PERF_FINDINGS.md:717:  baseline), so the whole cell is re-measured.
docs/plan/OTP12_PERF_FINDINGS.md:719:  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
docs/plan/OTP12_PERF_FINDINGS.md:732:  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
scripts/bench_otp12_win.sh:28:# (self-timed durability: Write-VolumeCache on Windows / per-file fsync
scripts/bench_otp12_win.sh:36:# destination sweep after the measured flush — the zoey I/O-storm
scripts/bench_otp12_win.sh:121:flush_win_ms() {   # Windows volume flush, self-timed; prints ms or NA
scripts/bench_otp12_win.sh:123:    # failed flush or garbage output must never read as a plausible
scripts/bench_otp12_win.sh:130:fsync_tree_ms() {   # macOS per-file fsync walk; prints its own elapsed ms
scripts/bench_otp12_win.sh:137:        os.fsync(fd)
scripts/bench_otp12_win.sh:468:echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
scripts/bench_otp12_win.sh:484:        RUN_FLUSH=$(flush_win_ms)
scripts/bench_otp12_win.sh:487:        RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
scripts/bench_otp12_win.sh:505:    RUN_FLUSH=$(fsync_tree_ms "$dst")
scripts/bench_otp12_win.sh:534:    RUN_FLUSH=$(flush_win_ms)
scripts/bench_otp12_win.sh:545:    RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
scripts/bench_otp12_win.sh:579:            log "  $cell/$aname slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
scripts/bench_otp12pf_mac.sh:57:#     (measured; yes, negative). Every `ms` row would have been ~= fsync_ms alone,
scripts/bench_otp12pf_mac.sh:59:#     fsync noise, which can manufacture or mask a one-directional effect at will.
scripts/bench_otp12pf_mac.sh:153:for _v in SETTLE_MS LOAD_MAX DRAIN_ITERS DRAIN_QUIET DRAIN_MBPS DELTA_REF_MS TIMER_TOLERANCE_MS \
scripts/bench_otp12pf_mac.sh:167:SETTLE_MS=250              # equal pre-fsync window on BOTH arms
scripts/bench_otp12pf_mac.sh:177:# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
scripts/bench_otp12pf_mac.sh:178:# it. And 24660ae is the commit that added it TO FIX the free-writeback asymmetry
scripts/bench_otp12pf_mac.sh:180:# one-directional P1 out of nothing. The pre-registration has claimed an equal settle
scripts/bench_otp12pf_mac.sh:185:SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
scripts/bench_otp12pf_mac.sh:186:[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
scripts/bench_otp12pf_mac.sh:195:# reason SETTLE_MS is: assigned above it, the pinning line itself would set the variable and
scripts/bench_otp12pf_mac.sh:315:read -1 ms / 2 ms for this exact sleep. Every row would be fsync noise. REFUSING to take data."
scripts/bench_otp12pf_mac.sh:429:  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
scripts/bench_otp12pf_mac.sh:632:# --- the settle-gap bound (NOT a removal — a measured bound) -------------------
scripts/bench_otp12pf_mac.sh:633:# Between the client exiting and the fsync starting, the OS writes back dirty pages
scripts/bench_otp12pf_mac.sh:635:# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
scripts/bench_otp12pf_mac.sh:647:  # fast-failing connection would report a small "bound" and flatter the settle claim.
scripts/bench_otp12pf_mac.sh:662:  local rtt_max=$(( SETTLE_MS / 4 ))
scripts/bench_otp12pf_mac.sh:664:    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
scripts/bench_otp12pf_mac.sh:665:  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
scripts/bench_otp12pf_mac.sh:727:  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
scripts/bench_otp12pf_mac.sh:743:    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
scripts/bench_otp12pf_mac.sh:773:  log "  python3 on $(hname "$h"): $p (absolute — a PATH entry or shell function cannot stand in for the interpreter that MEASURES the settle)"
scripts/bench_otp12pf_mac.sh:909:RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
scripts/bench_otp12pf_mac.sh:910:fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
scripts/bench_otp12pf_mac.sh:912:  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
scripts/bench_otp12pf_mac.sh:915:  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
scripts/bench_otp12pf_mac.sh:918:  # counted files — so a no-op `sleep` would pass while the log narrated "settle
scripts/bench_otp12pf_mac.sh:919:  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
scripts/bench_otp12pf_mac.sh:921:  # A protection that cannot be OBSERVED is not a protection. The settle now happens
scripts/bench_otp12pf_mac.sh:925:  out="$(hrun "$1" "$(hpy "$1") - '$SETTLE_SEC' '$2' <<'PYEOF'
scripts/bench_otp12pf_mac.sh:927:settle = float(sys.argv[1])
scripts/bench_otp12pf_mac.sh:930:time.sleep(settle)
scripts/bench_otp12pf_mac.sh:931:settled_ms = int((time.monotonic() - t0) * 1000)
scripts/bench_otp12pf_mac.sh:933:    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
scripts/bench_otp12pf_mac.sh:943:        os.fsync(fd)
scripts/bench_otp12pf_mac.sh:946:print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
scripts/bench_otp12pf_mac.sh:950:# The settle actually elapsed, on the destination's own clock. Anything else voids.
scripts/bench_otp12pf_mac.sh:951:settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
scripts/bench_otp12pf_mac.sh:955:# free-writeback asymmetry between the arms is bounded BY this number, and a bound measured
scripts/bench_otp12pf_mac.sh:983:  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
scripts/bench_otp12pf_mac.sh:986:  # The equal settle is the ONLY thing standing between this rig and a free-writeback
scripts/bench_otp12pf_mac.sh:990:  if ! settle_ok "$RUN_SETTLED"; then
scripts/bench_otp12pf_mac.sh:991:    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
scripts/bench_otp12pf_mac.sh:995:    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
scripts/bench_otp12pf_mac.sh:1009:  local rtt_max=$(( SETTLE_MS / 4 ))
scripts/bench_otp12pf_mac.sh:1011:    log "  VOID: ssh dispatch measured ${RUN_RTT}ms (max ${rtt_max}ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible"
scripts/bench_otp12pf_mac.sh:1021:# above is what makes a wrong path fatal instead of silently free.
scripts/bench_otp12pf_mac.sh:1092:    local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_RTT,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
scripts/bench_otp12pf_mac.sh:1094:    log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (fsync ${RUN_FLUSH}ms, settle ${RUN_SETTLED}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
scripts/bench_otp12pf_mac.sh:1223:# The fsync/settle path, exercised for real on a throwaway tree. It is the durability
scripts/bench_otp12pf_mac.sh:1224:# measurement AND the equal-settle window — the two things that once manufactured P1 —
scripts/bench_otp12pf_mac.sh:1226:selftest_fsync() {
scripts/bench_otp12pf_mac.sh:1227:  local h="$1" d ms files bytes settled
scripts/bench_otp12pf_mac.sh:1230:    || { log "  [BROKEN] fsync/settle — cannot stage a probe tree"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); return 1; }
scripts/bench_otp12pf_mac.sh:1231:  read -r ms files bytes settled <<<"$(fsync_tree "$h" "$d")"
scripts/bench_otp12pf_mac.sh:1234:    log "  [BROKEN] fsync/settle — walk returned ms=$ms files=$files bytes=$bytes, want 2 files / 6 bytes"
scripts/bench_otp12pf_mac.sh:1237:  # THE SETTLE MUST BE PROVED, NOT NARRATED (round-6, both reviewers). The old check
scripts/bench_otp12pf_mac.sh:1238:  # counted files and then LOGGED "settle included" — which is a sentence, not an
scripts/bench_otp12pf_mac.sh:1239:  # assertion. It would have passed with the settle stone dead, which is precisely how
scripts/bench_otp12pf_mac.sh:1240:  # the settle stayed dead for three revisions.
scripts/bench_otp12pf_mac.sh:1241:  if ! settle_ok "$settled"; then
scripts/bench_otp12pf_mac.sh:1242:    log "  [BROKEN] fsync/settle — THE SETTLE DID NOT ELAPSE: measured ${settled}ms, want >= ${SETTLE_MS}ms"
scripts/bench_otp12pf_mac.sh:1245:  log "  [OK]     fsync/settle — 2 files/6 bytes walked in ${ms}ms; settle MEASURED at ${settled}ms (>= ${SETTLE_MS}ms), counts VERIFIED"
scripts/bench_otp12pf_mac.sh:1296:    selftest_fsync "$h"
scripts/bench_otp12pf_mac.sh:1349:  echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,rtt_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
.review/results/macmac-r12.codex-design.md:91:   executes, or that cannot fail — e.g. an equal-settle window that was dead for three
.review/results/macmac-r12.codex-design.md:175:The Mac↔Mac run is not the right next move. At best, a positive result gives you a convenient local reproduction rig. It does not adjudicate H1, identify a cause, or alter P1’s release obligation. A negative result is especially weak because the Mac↔Mac harness introduces the corrected equal-settle method that was not used for the Mac↔Windows measurements defining P1.
.review/results/macmac-r12.codex-design.md:177:First, rerun the known-failing Mac↔Windows cell with this corrected timing method while recording dial, accept, ACK, resize-epoch, first-byte, and completion timestamps; take a packet capture concurrently. If P1 survives, test H1 with a causal dial/accept counterfactual. A targeted microbenchmark comes after the trace identifies a phase, because the mixed-only interaction may disappear in isolation. A cheap two-cell Mac↔Mac screen could then establish whether local debugging is possible; the current 96-transfer apparatus is disproportionate.
.review/results/macmac-r12.codex-design.md:196:- **BLOCKER — no same-method positive control.** The new instrument deliberately inserts and verifies a settle before fsync at [bench_otp12pf_mac.sh:925](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:925), because the previous timing gap could manufacture a one-directional effect. The original rig-W harness flushes immediately at [bench_otp12_win.sh:505](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:505). Before asking whether Windows is necessary, establish that P1 still exists on Mac↔Windows under the corrected method. Otherwise a Mac↔Mac null conflates platform pairing with measurement repair.
.review/results/macmac-r12.codex-design.md:214:- **HIGH — `B` is not a statistically justified bias bound.** It assumes rig bias scales multiplicatively with arm time at [otp12pf_mac_verdict.py:303](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:303). The known SSH dispatch residual is naturally additive in milliseconds, while free-writeback response may be nonlinear. A 49 ms additive bias observed on a slow control is scaled downward on a faster measurand and can under-harden a verdict. No control evidence establishes the chosen proportional model. A conservative bound would need a predeclared confound model—at minimum the maximum of additive and proportional bounds—or closely speed-matched controls.
.review/results/macmac-r12.codex-design.md:234:**DO NOT RUN IT — first validate P1 with the corrected measurement on the known-failing Mac↔Windows pair and instrument the suspected path; the current rule can call a 1.092/PASS cell “REPRODUCES,” and the end-fabric gate is incomplete.**
.review/results/macmac-r12.codex-design.md:237:The Mac↔Mac run is not the right next move. At best, a positive result gives you a convenient local reproduction rig. It does not adjudicate H1, identify a cause, or alter P1’s release obligation. A negative result is especially weak because the Mac↔Mac harness introduces the corrected equal-settle method that was not used for the Mac↔Windows measurements defining P1.
.review/results/macmac-r12.codex-design.md:239:First, rerun the known-failing Mac↔Windows cell with this corrected timing method while recording dial, accept, ACK, resize-epoch, first-byte, and completion timestamps; take a packet capture concurrently. If P1 survives, test H1 with a causal dial/accept counterfactual. A targeted microbenchmark comes after the trace identifies a phase, because the mixed-only interaction may disappear in isolation. A cheap two-cell Mac↔Mac screen could then establish whether local debugging is possible; the current 96-transfer apparatus is disproportionate.
.review/results/macmac-r12.codex-design.md:258:- **BLOCKER — no same-method positive control.** The new instrument deliberately inserts and verifies a settle before fsync at [bench_otp12pf_mac.sh:925](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:925), because the previous timing gap could manufacture a one-directional effect. The original rig-W harness flushes immediately at [bench_otp12_win.sh:505](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:505). Before asking whether Windows is necessary, establish that P1 still exists on Mac↔Windows under the corrected method. Otherwise a Mac↔Mac null conflates platform pairing with measurement repair.
.review/results/macmac-r12.codex-design.md:276:- **HIGH — `B` is not a statistically justified bias bound.** It assumes rig bias scales multiplicatively with arm time at [otp12pf_mac_verdict.py:303](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:303). The known SSH dispatch residual is naturally additive in milliseconds, while free-writeback response may be nonlinear. A 49 ms additive bias observed on a slow control is scaled downward on a faster measurand and can under-harden a verdict. No control evidence establishes the chosen proportional model. A conservative bound would need a predeclared confound model—at minimum the maximum of additive and proportional bounds—or closely speed-matched controls.
.review/results/macmac-r12.codex-design.md:296:**DO NOT RUN IT — first validate P1 with the corrected measurement on the known-failing Mac↔Windows pair and instrument the suspected path; the current rule can call a 1.092/PASS cell “REPRODUCES,” and the end-fabric gate is incomplete.**
.review/results/macmac-r12.grok-design.md:102:Would I trust a null out of it? **Yes**, if I got one. Would I expect to get one on a live APFS/10GbE pair with purge/drain/fsync in the loop? **Often no** — UNCLEAR / CONTROLS-NOT-CLEAN instead. That is the rule working as a nervous instrument, not as a sharp classifier.
.review/results/macmac-r12.grok-design.md:117:I did **not** find a live “false REPRODUCES / false DOES-NOT-REPRODUCE” in the graded path comparable to the historical timer/settle/control-void blockers. Remaining issues are smaller or process-level.
.review/results/macmac-r12.grok-design.md:124:| **LOW** | `bench_otp12pf_mac.sh` 1008–1013 vs preflight RTT | Per-pair RTT void is good; document already registers it. Residual free-writeback bound is measurement-backed, not proof of zero. |
.review/results/macmac-r12.grok-design.md:129:I am **not** claiming the harness is bug-free. I am claiming: at this revision, the failure mode that should scare you most is **a clean, confident non-answer** (or a twelfth review), not an obvious arithmetic reverse of P1.
.review/results/macmac-r12.grok-design.md:135:This is a **well-built instrument pointed at a real but low-leverage question**. It will answer “Windows-free P1 on nagatha↔q?” only if noise is unusually kind; many runs will return CONTROLS-NOT-CLEAN / UNCLEAR and license nothing. When it is decisive, **you still do pf-1 on rig W**. The expensive thing left is not another design pass — it is either taking data once or admitting the 2×2 cell is optional science and moving to the pair that blocks release.

exec
/bin/zsh -lc "python3 -c '
import csv, statistics as s

def load(path):
    with open(path,newline=\"\") as f: return list(csv.DictReader(f))
def armstats(rows):
    t=[int(r[\"ms\"]) for r in rows]; f=[int(r[\"flush_ms\"]) for r in rows]; x=[a-b for a,b in zip(t,f)]
    st=lambda v:(s.median(v),s.mean(v),min(v),max(v))
    return st(t),st(f),st(x),t,f,x
q=load(\"docs/bench/otp12-q-baseline-2026-07-13/runs.csv\")
print(\"Q BASELINE\")
for cell in sorted({r[\"cell\"] for r in q}):
    print(\"\\\\n\"+cell); arms={}
    for arm in sorted({r[\"arm\"] for r in q if r[\"cell\"]==cell}):
        z=armstats([r for r in q if r[\"cell\"]==cell and r[\"arm\"]==arm]); arms[arm]=z
        print(arm,\"total med/mean/range=\",z[0],\"flush=\",z[1],\"xfer=\",z[2])
        print(\" totals\",z[3]); print(\" flush \",z[4]); print(\" xfer  \",z[5])
    an=list(arms); a,b=arms[an[0]],arms[an[1]]
    print(\"median ratios total/flush/xfer\",*(a[i][0]/b[i][0] if b[i][0] else None for i in range(3)))
    print(\"median deltas total/flush/xfer\",*(a[i][0]-b[i][0] for i in range(3)))
l=load(\"docs/bench/otp12-perf-2026-07-13/runs.csv\")
print(\"\\\\nLINUX TCP MIXED\")
for cell in [\"sm_tcp_mixed\",\"ms_tcp_mixed\"]:
    print(\"\\\\n\"+cell); arms={}
    for arm in [\"srcinit\",\"destinit\"]:
        z=armstats([r for r in l if r[\"cell\"]==cell and r[\"arm\"]==arm]); arms[arm]=z
        print(arm,\"total med/mean/range=\",z[0],\"flush=\",z[1],\"xfer=\",z[2]); print(\" xfer\",z[5])
    src,dst=arms[\"srcinit\"],arms[\"destinit\"]
    for label,i in [(\"total\",0),(\"flush\",1),(\"xfer\",2)]:
        vals=(src[i][0],dst[i][0]); print(label,\"src,dest,max/min,dest/src=\",*vals,max(vals)/min(vals),vals[1]/vals[0])
'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-CJT6wh3W' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-5CMlviAD' (errno=Operation not permitted)
Q BASELINE
\nmw_tcp_mixed
mac_init total med/mean/range= (1688.0, 1692.25, 1598, 1809) flush= (657.5, 657.125, 641, 675) xfer= (1019.0, 1035.125, 940, 1166)
 totals [1772, 1761, 1598, 1730, 1598, 1646, 1624, 1809]
 flush  [657, 661, 652, 675, 658, 670, 641, 643]
 xfer   [1115, 1100, 946, 1055, 940, 976, 983, 1166]
win_init total med/mean/range= (1618.5, 1616.375, 1562, 1662) flush= (656.0, 655.125, 648, 659) xfer= (965.0, 961.25, 912, 1007)
 totals [1652, 1647, 1662, 1615, 1622, 1573, 1598, 1562]
 flush  [658, 656, 655, 659, 648, 659, 656, 650]
 xfer   [994, 991, 1007, 956, 974, 914, 942, 912]
median ratios total/flush/xfer 1.0429409947482238 1.0022865853658536 1.0559585492227979
median deltas total/flush/xfer 69.5 1.5 54.0
\nwm_grpc_mixed
mac_init total med/mean/range= (1254.0, 1241.75, 1193, 1290) flush= (69.0, 69.625, 64, 76) xfer= (1184.5, 1172.125, 1122, 1216)
 totals [1205, 1193, 1268, 1262, 1276, 1194, 1246, 1290]
 flush  [76, 66, 64, 65, 66, 72, 74, 74]
 xfer   [1129, 1127, 1204, 1197, 1210, 1122, 1172, 1216]
win_init total med/mean/range= (1230.0, 1240.375, 1196, 1292) flush= (71.0, 69.125, 62, 72) xfer= (1159.5, 1171.25, 1134, 1220)
 totals [1278, 1237, 1200, 1196, 1280, 1223, 1217, 1292]
 flush  [72, 70, 63, 62, 72, 71, 71, 72]
 xfer   [1206, 1167, 1137, 1134, 1208, 1152, 1146, 1220]
median ratios total/flush/xfer 1.0195121951219512 0.971830985915493 1.0215610176800345
median deltas total/flush/xfer 24.0 -2.0 25.0
\nwm_tcp_large
mac_init total med/mean/range= (909.5, 956.375, 907, 1270) flush= (0.0, 0, 0, 0) xfer= (909.5, 956.375, 907, 1270)
 totals [1270, 927, 910, 908, 912, 908, 909, 907]
 flush  [0, 0, 0, 0, 0, 0, 0, 0]
 xfer   [1270, 927, 910, 908, 912, 908, 909, 907]
win_init total med/mean/range= (907.5, 909, 904, 924) flush= (0.0, 0, 0, 0) xfer= (907.5, 909, 904, 924)
 totals [909, 905, 905, 907, 924, 908, 904, 910]
 flush  [0, 0, 0, 0, 0, 0, 0, 0]
 xfer   [909, 905, 905, 907, 924, 908, 904, 910]
median ratios total/flush/xfer 1.0022038567493112 None 1.0022038567493112
median deltas total/flush/xfer 2.0 0.0 2.0
\nwm_tcp_mixed
mac_init total med/mean/range= (1093.0, 1107.375, 1082, 1176) flush= (72.0, 70, 62, 73) xfer= (1021.0, 1037.375, 1012, 1104)
 totals [1176, 1095, 1091, 1096, 1089, 1145, 1082, 1085]
 flush  [72, 73, 71, 73, 72, 64, 62, 73]
 xfer   [1104, 1022, 1020, 1023, 1017, 1081, 1020, 1012]
win_init total med/mean/range= (789.5, 790.5, 719, 894) flush= (73.0, 71.875, 64, 74) xfer= (720.5, 718.625, 647, 820)
 totals [844, 719, 796, 894, 724, 783, 723, 841]
 flush  [73, 72, 74, 74, 74, 64, 73, 71]
 xfer   [771, 647, 722, 820, 650, 719, 650, 770]
median ratios total/flush/xfer 1.3844205193160228 0.9863013698630136 1.4170714781401805
median deltas total/flush/xfer 303.5 -1.0 300.5
\nLINUX TCP MIXED
\nsm_tcp_mixed
srcinit total med/mean/range= (1745.5, 1737.75, 1710, 1750) flush= (780.0, 780, 750, 810) xfer= (955.0, 957.75, 931, 990)
 xfer [990, 950, 960, 931]
destinit total med/mean/range= (1905.0, 1922.5, 1840, 2040) flush= (780.0, 785, 760, 820) xfer= (1135.0, 1137.5, 1060, 1220)
 xfer [1110, 1060, 1160, 1220]
total src,dest,max/min,dest/src= 1745.5 1905.0 1.0913778287023777 1.0913778287023777
flush src,dest,max/min,dest/src= 780.0 780.0 1.0 1.0
xfer src,dest,max/min,dest/src= 955.0 1135.0 1.1884816753926701 1.1884816753926701
\nms_tcp_mixed
srcinit total med/mean/range= (2085.5, 2168.25, 1991, 2511) flush= (570.5, 590.75, 451, 771) xfer= (1545.0, 1577.5, 1480, 1740)
 xfer [1480, 1540, 1740, 1550]
destinit total med/mean/range= (2079.5, 2077, 1959, 2190) flush= (499.5, 527.25, 480, 630) xfer= (1544.5, 1549.75, 1470, 1640)
 xfer [1640, 1529, 1470, 1560]
total src,dest,max/min,dest/src= 2085.5 2079.5 1.0028853089685021 0.9971229920882282
flush src,dest,max/min,dest/src= 570.5 499.5 1.142142142142142 0.8755477651183172
xfer src,dest,max/min,dest/src= 1545.0 1544.5 1.0003237293622531 0.9996763754045307

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '110,150p'; nl -ba scripts/bench_otp12_win.sh | sed -n '450,560p'; nl -ba scripts/bench_otp12pf_mac.sh | sed -n '150,205p'; nl -ba scripts/bench_otp12pf_mac.sh | sed -n '900,1020p'; nl -ba .review/results/macmac-r12.codex-design.md | sed -n '165,205p'; nl -ba .review/results/macmac-r12.grok-design.md | sed -n '108,142p'; nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '80,180p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
   150	# very line meant to pin it.
   151	# =============================================================================
   152	_overrides=""
   153	for _v in SETTLE_MS LOAD_MAX DRAIN_ITERS DRAIN_QUIET DRAIN_MBPS DELTA_REF_MS TIMER_TOLERANCE_MS \
   154	          N_IP N_NIC N_MAC Q_IP Q_NIC Q_MAC Q_SSH REGISTERED_MTU REGISTERED_MEDIA REGISTERED_MSS \
   155	          RUNS PORT; do
   156	  [[ -n "${!_v+set}" ]] && _overrides="$_overrides $_v=${!_v}"
   157	done
   158	if [[ -n "$_overrides" ]]; then
   159	  echo "REFUSING: the pre-registered constants are NOT tunable, and these are set in the" >&2
   160	  echo "environment:$_overrides" >&2
   161	  echo "A rule the operator can retune after seeing the data is not a pre-registration." >&2
   162	  echo "To change one, amend docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md and" >&2
   163	  echo "put it back through review. That is the entire point of the document." >&2
   164	  exit 2
   165	fi
   166	
   167	SETTLE_MS=250              # equal pre-fsync window on BOTH arms
   168	# Computed ONCE, HERE, at top level — and this line is load-bearing history.
   169	#
   170	# It used to be computed inline as `sleep $(awk ... 'BEGIN{printf \"%.3f\", m/1000}')`
   171	# INSIDE the double-quoted hrun string. A command substitution is parsed FRESH by
   172	# bash, so those `\"` escapes — which are correct for hrun's two-level strings — were
   173	# literal backslashes to awk. **The awk errored on EVERY call, `sleep` got an empty
   174	# argument and FAILED, and the old code ignored its exit status because the python
   175	# walk that followed supplied the status.**
   176	#
   177	# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
   178	# it. And 24660ae is the commit that added it TO FIX the free-writeback asymmetry
   179	# that reverses sign with direction — the artifact judged capable of MANUFACTURING a
   180	# one-directional P1 out of nothing. The pre-registration has claimed an equal settle
   181	# on both arms through revisions 3, 4 and 5. It was never applied.
   182	#
   183	# Found only by EXECUTING it (round-5 codex flagged the ignored exit status; running
   184	# it showed the status was ALWAYS failure). `bash -n` sees nothing here.
   185	SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
   186	[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
   187	LOAD_MAX=3.0               # start AND end load1 bar on both Macs
   188	DRAIN_ITERS=60
   189	DRAIN_QUIET=3
   190	DRAIN_MBPS=2               # destination disk must be below this to start a window
   191	DELTA_REF_MS=230           # rig W's measured Delta_P1 — THE reference effect
   192	TIMER_TOLERANCE_MS=120     # the timer self-test's allowed error on a 1000 ms sleep
   193	
   194	# THE REGISTERED TOPOLOGY. These are assigned HERE, BELOW the override check, for the same
   195	# reason SETTLE_MS is: assigned above it, the pinning line itself would set the variable and
   196	# the check would then see it as "present in the environment" and refuse EVERY run. (It did.
   197	# Caught by RUNNING it -- the first draft of this fix put them at the top of the file and the
   198	# harness refused to start. A protection that cannot PASS is as dead as one that cannot FAIL.)
   199	N_IP=10.1.10.92                       # nagatha, 10GbE en11
   200	N_NIC=en11
   201	N_MAC=00:e0:4d:01:4c:a3               # nagatha's OWN en11 MAC (measured)
   202	Q_SSH=michael@q
   203	Q_IP=10.1.10.54                       # q, 10GbE en8
   204	Q_NIC=en8
   205	Q_MAC=00:01:d2:19:04:a3               # q's OWN en8 MAC (measured)
   900	  hrun q "sync && sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   901	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   902	  else RUN_COLD="COLD-FAIL(nagatha=$cn;q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi   # ';' not ',': this lands in a CSV
   903	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
   904	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   905	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   906	}
   907	
   908	# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
   909	RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
   910	fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
   911	  local out
   912	  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
   913	  #
   914	  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
   915	  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
   916	  # RAN. Round 6 then found the repair was still not provable: `sleep` is
   917	  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
   918	  # counted files — so a no-op `sleep` would pass while the log narrated "settle
   919	  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
   920	  #
   921	  # A protection that cannot be OBSERVED is not a protection. The settle now happens
   922	  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
   923	  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
   924	  # to shadow, no exit status left to discard, and no narration left to trust.
   925	  out="$(hrun "$1" "$(hpy "$1") - '$SETTLE_SEC' '$2' <<'PYEOF'
   926	import os, sys, time
   927	settle = float(sys.argv[1])
   928	p = sys.argv[2]
   929	t0 = time.monotonic()
   930	time.sleep(settle)
   931	settled_ms = int((time.monotonic() - t0) * 1000)
   932	if not os.path.isdir(p):
   933	    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
   934	    raise SystemExit
   935	t = time.monotonic()
   936	files = 0
   937	nbytes = 0
   938	for root, _d, fs in os.walk(p):
   939	    for name in fs:
   940	        fp = os.path.join(root, name)
   941	        nbytes += os.path.getsize(fp)
   942	        fd = os.open(fp, os.O_RDONLY)
   943	        os.fsync(fd)
   944	        os.close(fd)
   945	        files += 1
   946	print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
   947	PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
   948	  echo "${out:-NA 0 0 0}"
   949	}
   950	# The settle actually elapsed, on the destination's own clock. Anything else voids.
   951	settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
   952	
   953	# --- one timed run ------------------------------------------------------------
   954	# The ssh dispatch, measured RIGHT NOW rather than assumed from preflight. The residual
   955	# free-writeback asymmetry between the arms is bounded BY this number, and a bound measured
   956	# once at the start is not a bound on a run taken twenty minutes later (round-10 codex).
   957	RUN_RTT=0
   958	rtt_now() {
   959	  local v
   960	  v="$(python3 -c '
   961	import statistics, subprocess, sys, time
   962	argv = sys.argv[1:]
   963	ts = []
   964	for _ in range(3):
   965	    t = time.monotonic()
   966	    if subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) != 0:
   967	        print(-1); raise SystemExit
   968	    ts.append((time.monotonic() - t) * 1000.0)
   969	print(int(statistics.median(ts)))
   970	' ssh "${SSH_MUX[@]}" "$Q_SSH" true 2>/dev/null)" || v=-1
   971	  [[ "$v" =~ ^-?[0-9]+$ ]] || v=-1
   972	  echo "$v"
   973	}
   974	
   975	RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
   976	timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
   977	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
   978	  bin="$(hblit "$ih")"
   979	  prep_run "$dh"
   980	  RUN_RTT="$(rtt_now)"
   981	  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
   982	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   983	  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
   984	  RUN_VALID=yes
   985	  wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
   986	  # The equal settle is the ONLY thing standing between this rig and a free-writeback
   987	  # artifact that REVERSES SIGN WITH DIRECTION — i.e. that can manufacture P1 out of
   988	  # nothing. It has already been silently dead once. If it did not measurably elapse,
   989	  # the row is not a fast row; it is a VOID row.
   990	  if ! settle_ok "$RUN_SETTLED"; then
   991	    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
   992	    RUN_VALID=no
   993	  fi
   994	  if [[ "$RUN_FLUSH" == NA ]]; then
   995	    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
   996	    RUN_VALID=no; RUN_FLUSH=0
   997	  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
   998	    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
   999	    RUN_VALID=no
  1000	  fi
  1001	  # A negative or absurd transfer time means the CLOCK failed, not that the transfer
  1002	  # was fast. It must never enter the data.
  1003	  if [[ ! "$RUN_MS" =~ ^[0-9]+$ ]] || (( RUN_MS < 1 )); then
  1004	    log "  VOID: transfer timer returned '$RUN_MS' — the clock failed (round 2's killer). NOT a fast run."
  1005	    RUN_VALID=no; RUN_MS=0
  1006	  fi
  1007	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
  1008	  # The dispatch bound, enforced on THIS pair rather than assumed from preflight.
  1009	  local rtt_max=$(( SETTLE_MS / 4 ))
  1010	  if [[ ! "$RUN_RTT" =~ ^[0-9]+$ ]] || (( RUN_RTT > rtt_max )); then
  1011	    log "  VOID: ssh dispatch measured ${RUN_RTT}ms (max ${rtt_max}ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible"
  1012	    RUN_VALID=no
  1013	  fi
  1014	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
  1015	}
  1016	
  1017	# --- arms ---------------------------------------------------------------------
  1018	# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
  1019	# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
  1020	# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
   165	mcp: ptk/ptk_job (completed)
   166	mcp: ptk/ptk_job started
   167	mcp: ptk/ptk_job (completed)
   168	mcp: ptk/ptk_invoke started
   169	mcp: ptk/ptk_invoke (completed)
   170	mcp: ptk/ptk_invoke started
   171	mcp: ptk/ptk_invoke (completed)
   172	mcp: ptk/ptk_invoke started
   173	mcp: ptk/ptk_invoke (completed)
   174	codex
   175	The Mac↔Mac run is not the right next move. At best, a positive result gives you a convenient local reproduction rig. It does not adjudicate H1, identify a cause, or alter P1’s release obligation. A negative result is especially weak because the Mac↔Mac harness introduces the corrected equal-settle method that was not used for the Mac↔Windows measurements defining P1.
   176	
   177	First, rerun the known-failing Mac↔Windows cell with this corrected timing method while recording dial, accept, ACK, resize-epoch, first-byte, and completion timestamps; take a packet capture concurrently. If P1 survives, test H1 with a causal dial/accept counterfactual. A targeted microbenchmark comes after the trace identifies a phase, because the mixed-only interaction may disappear in isolation. A cheap two-cell Mac↔Mac screen could then establish whether local debugging is possible; the current 96-transfer apparatus is disproportionate.
   178	
   179	## What each outcome buys
   180	
   181	| Outcome | Legitimate learning | Not licensed |
   182	|---|---|---|
   183	| `REPRODUCES` | P1-shaped behavior can occur on these Macs without Windows; use them as a local debugging rig. | H1 confirmed or killed; platform-general behavior; Windows exonerated as a contributor; release acceptance. |
   184	| `DOES-NOT-REPRODUCE` | Under this method and window, both directions exclude a median gap of the registered size, subject to the statistical assumptions. | Windows is necessary; H1 confirmed; the four Mac↔Windows observations retracted; no tail or intermittent effect. |
   185	| `MIXED` | Strong host×role, ordering, or endpoint interaction requiring decomposition. | Either reproduction or null. |
   186	| `INVERTED` | A new, opposite initiator asymmetry. | P1 absent. |
   187	| `UNCLEAR` | The rig did not resolve the margin. | A weak null or partial reproduction. |
   188	| `CONTROLS-NOT-CLEAN` | Either a control has initiator asymmetry or the controls cannot bound the proposed bias. | Anything about P1 itself. |
   189	| `INCOMPLETE` | Only an execution/feasibility failure. | Any performance inference. |
   190	| `RIG-VOID` | Environmental conditions invalidated the session. | Any performance inference. |
   191	
   192	Only `REPRODUCES` changes anything operationally: it changes where debugging can happen. Every substantive path still leads to direct instrumentation and a causal counterfactual, so the full experiment is close to theatre even before its remaining defects.
   193	
   194	## (3) Design findings
   195	
   196	- **BLOCKER — no same-method positive control.** The new instrument deliberately inserts and verifies a settle before fsync at [bench_otp12pf_mac.sh:925](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:925), because the previous timing gap could manufacture a one-directional effect. The original rig-W harness flushes immediately at [bench_otp12_win.sh:505](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:505). Before asking whether Windows is necessary, establish that P1 still exists on Mac↔Windows under the corrected method. Otherwise a Mac↔Mac null conflates platform pairing with measurement repair.
   197	
   198	- **HIGH — the controls do not fully support “TCP × mixed-specific.”** The registered controls are gRPC-mixed and TCP-large at [PREREGISTRATION.md:164](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:164), but prior evidence defining the signature also uses TCP-small as a clean fixture control at [OTP12_PERF_FINDINGS.md:228](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:228). Mixed contains 5,000 small files; TCP-small is the most relevant check for a small-file/metadata interaction masquerading as “mixed-only.”
   199	
   200	- **HIGH — n=8 is enough for an exact median CI, not enough to promise a useful answer.** Under independent, identically distributed paired differences, `[min,max]` has 99.22% coverage for the population median. But `EFFECT` requires every pair to clear the margin and `NONE` requires every pair inside it. One soft pair produces `UNCLEAR`; modest control noise produces `CONTROLS-NOT-CLEAN`. No empirical power calculation justifies expecting decisiveness. ABBA controls first-order drift but does not establish independence or stationarity. See [PREREGISTRATION.md:201](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:201) and [PREREGISTRATION.md:380](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:380).
   201	
   202	- **HIGH — the null’s estimand is unclear.** If the claim is median equivalence, the exact CI is recognizable and defensible under IID assumptions. If “no transfer may vary by more than 10%” means individual or future transfers, eight observed values inside the margin cannot establish that; it is not a population tolerance bound. The document alternates between median inference and “every pair” rhetoric at [PREREGISTRATION.md:250](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:250).
   203	
   204	- **MEDIUM — in-place retries weaken contemporaneous controls.** A voided cell retries repeatedly before its siblings run at [bench_otp12pf_mac.sh:1118](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:1118). After several long retries, its valid observation and the controls no longer share the same transient. Retry after completing the rest of the slot, or explicitly abandon that slot.
   205	
   108	| **LOW** | EFFECT inclusive vs NONE strict at the boundary — conservative, asymmetric, OK. |
   109	
   110	---
   111	
   112	## 5. Correctness (harness / rule) — bugs that wrong the measurement or verdict
   113	
   114	Selftest: **PASSED** (0 blind; quiescence correctly **FIRED** on nagatha while `codex` runs).  
   115	Verdict: **40/40 cases, 19/19 mutations killed, 300/300 fuzz, n=8 CI≡range identity OK.**
   116	
   117	I did **not** find a live “false REPRODUCES / false DOES-NOT-REPRODUCE” in the graded path comparable to the historical timer/settle/control-void blockers. Remaining issues are smaller or process-level.
   118	
   119	| Sev | Where | Issue |
   120	|---|---|---|
   121	| **MEDIUM** | `bench_otp12pf_mac.sh` ~1118–1126, 1052–1063 | In-place void retries break the “controls share the measurand’s moment” story inside a slot. Can mis-fire CONTROLS clean while measurand retried under different load (or the reverse). |
   122	| **MEDIUM** | `bench_otp12pf_mac.sh` 1322–1325; main path | Selftest **still does not** run daemon lifecycle, smoke, ABBA, or pair-void. First full path execution **is** the first data session. Historically this class is where defects hide. Not a known wrong formula; residual risk. |
   123	| **MEDIUM** | `otp12pf_mac_verdict.py` 229–257 + control loop 305–323 | Not a code bug: full-range control NONE is so tight that **correct** code yields **frequent non-answers**. Wrong *utility*, not wrong arithmetic. |
   124	| **LOW** | `bench_otp12pf_mac.sh` 1008–1013 vs preflight RTT | Per-pair RTT void is good; document already registers it. Residual free-writeback bound is measurement-backed, not proof of zero. |
   125	| **LOW** | `otp12pf_mac_verdict_test.py` header vs body | Doc counts (34 cases / 14 mutations) drift vs 40 / 19. Guard is not vacuous; commentary is stale. |
   126	| **LOW** | `bench_otp12pf_mac.sh` 1–23, 96–104 | Header still “round-11 / round-12 gate” narrative; process, not measurand. |
   127	| **INFO** | Historical classes you named | Settle is now in-process and selftest-proved; constants pinned **after** override check; topology/MSS gated; timer single-process + gate. Those specific landmines look closed **as of this selftest**. |
   128	
   129	I am **not** claiming the harness is bug-free. I am claiming: at this revision, the failure mode that should scare you most is **a clean, confident non-answer** (or a twelfth review), not an obvious arithmetic reverse of P1.
   130	
   131	---
   132	
   133	## Bottom line
   134	
   135	This is a **well-built instrument pointed at a real but low-leverage question**. It will answer “Windows-free P1 on nagatha↔q?” only if noise is unusually kind; many runs will return CONTROLS-NOT-CLEAN / UNCLEAR and license nothing. When it is decisive, **you still do pf-1 on rig W**. The expensive thing left is not another design pass — it is either taking data once or admitting the 2×2 cell is optional science and moving to the pair that blocks release.
   136	
   137	**DO NOT RUN IT — skip Mac↔Mac for now and spend the session on pf-1 / dial-accept diagnostics on the macOS↔Windows pair where P1 is known; no outcome of this rig changes that obligation, and the rule is likely to spend the run on CONTROLS-NOT-CLEAN.**
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

codex
Verdict: P1 is real in the recorded data, with high confidence. The settle issue is sensible harness hardening, but it does not plausibly explain a material portion of the 1.385 result. The prior review’s “re-measure before anything else” conclusion was wrong.

- For `wm_tcp_mixed`, the summary reports destination-init 1093 ms versus source-init 789 ms, ratio 1.385. Recomputed conventional medians are 1093.0 and 789.5 ms.

- Flush is not asymmetric in the required direction: medians are 72 ms versus 73 ms; means are 70.0 versus 71.875 ms. The faster arm actually has the slightly larger flush. A free-writeback artifact capable of making that arm appear roughly 300 ms faster predicts a substantially smaller flush there.

- Removing flush strengthens P1. Transfer-minus-flush medians are 1021.0 versus 720.5 ms, ratio 1.417. Their mean gap is 318.75 ms, while the total mean gap is 316.875 ms: flush offsets 1.875 ms rather than creating the difference. Every slow-arm transfer observation, 1012–1104 ms, exceeds every fast-arm observation, 647–820 ms; paired differences are 203–375 ms.

- The same-fixture gRPC control behaves oppositely to the artifact prediction. Its totals are 1254 versus 1230 ms, ratio 1.020; flush medians are 69 versus 71 ms; transfer-minus-flush ratio is 1.022. The fixture, destination, local/SSH arm arrangement, and dirty-byte volume are shared, yet no P1-sized carrier-independent failure appears. TCP-large and opposite-direction TCP-mixed also pass at 1.002 and 1.043.

- Linux’s immediate-flush results provide secondary support: all eight cells pass, with the two TCP-mixed ratios at 1.092 and 1.003. One Linux direction has a larger transfer-only ratio, 1.188, but its flush medians are exactly equal at 780 ms; that is not evidence of a free-writeback artifact. Because Linux writeback differs from macOS, this control is weaker than the direct Mac flush and gRPC evidence.

Thus the old harness contains an abstract opportunity for unequal excluded delay, but the recorded columns show that opportunity did not manufacture P1. The data locate the effect in the measured transfer process, not durability flushing. This establishes the effect, not its underlying code-versus-platform mechanism.

P1 REAL — Proceed directly to phase instrumentation and a causal dial/accept counterfactual on Mac↔Windows; treat an equal-settle rerun as optional confirmation, not a prerequisite.
tokens used
81,890
Verdict: P1 is real in the recorded data, with high confidence. The settle issue is sensible harness hardening, but it does not plausibly explain a material portion of the 1.385 result. The prior review’s “re-measure before anything else” conclusion was wrong.

- For `wm_tcp_mixed`, the summary reports destination-init 1093 ms versus source-init 789 ms, ratio 1.385. Recomputed conventional medians are 1093.0 and 789.5 ms.

- Flush is not asymmetric in the required direction: medians are 72 ms versus 73 ms; means are 70.0 versus 71.875 ms. The faster arm actually has the slightly larger flush. A free-writeback artifact capable of making that arm appear roughly 300 ms faster predicts a substantially smaller flush there.

- Removing flush strengthens P1. Transfer-minus-flush medians are 1021.0 versus 720.5 ms, ratio 1.417. Their mean gap is 318.75 ms, while the total mean gap is 316.875 ms: flush offsets 1.875 ms rather than creating the difference. Every slow-arm transfer observation, 1012–1104 ms, exceeds every fast-arm observation, 647–820 ms; paired differences are 203–375 ms.

- The same-fixture gRPC control behaves oppositely to the artifact prediction. Its totals are 1254 versus 1230 ms, ratio 1.020; flush medians are 69 versus 71 ms; transfer-minus-flush ratio is 1.022. The fixture, destination, local/SSH arm arrangement, and dirty-byte volume are shared, yet no P1-sized carrier-independent failure appears. TCP-large and opposite-direction TCP-mixed also pass at 1.002 and 1.043.

- Linux’s immediate-flush results provide secondary support: all eight cells pass, with the two TCP-mixed ratios at 1.092 and 1.003. One Linux direction has a larger transfer-only ratio, 1.188, but its flush medians are exactly equal at 780 ms; that is not evidence of a free-writeback artifact. Because Linux writeback differs from macOS, this control is weaker than the direct Mac flush and gRPC evidence.

Thus the old harness contains an abstract opportunity for unequal excluded delay, but the recorded columns show that opportunity did not manufacture P1. The data locate the effect in the measured transfer process, not durability flushing. This establishes the effect, not its underlying code-versus-platform mechanism.

P1 REAL — Proceed directly to phase instrumentation and a causal dial/accept counterfactual on Mac↔Windows; treat an equal-settle rerun as optional confirmation, not a prerequisite.
