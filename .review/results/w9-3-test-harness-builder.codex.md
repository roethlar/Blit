Reading additional input from stdin...
OpenAI Codex v0.142.5
--------
workdir: /home/michael/dev/Blit
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f2f73-fb18-73b3-a878-166b0a4fa840
--------
user
Review the diff of commit f6e592e (run: git show f6e592e). It implements review-queue row w9-3-test-harness-builder (REVIEW.md design-review queue; slice spec W9.3 in docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md; source findings tests-five-daemon-harness-clones, tests-per-test-cargo-build-subprocess, duplication-cli-test-daemon-harness, tests-fake-server-config-skew in docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md; implementation record .review/findings/w9-3-test-harness-builder.md in the same commit). Acceptance criteria: (1) TestContext::builder() in crates/blit-cli/tests/common/mod.rs expresses the knobs the deleted per-file harness clones existed for (extra daemon args, delegation, second daemon, read_only) and every clone is gone; (2) shared cli_bin()/run_with_timeout replace the pasted copies; (3) the blit-daemon cargo build runs once per test binary via OnceLock while preserving the R16-F1 no-cross-test-ordering property per process; (4) every in-process fake tonic server carries the production HTTP/2 keepalive config via a single shared builder (blit_core::remote::grpc_server) that daemon main.rs also uses, so test/prod server config cannot drift. Check: correctness regressions in the ported tests (semantics of each preserved — assertions, daemon flags, delegation/read_only wiring, timeouts); the new port-collision fix (process-global claimed-port set + child-death readiness check) — is it sound and race-free within one test binary; that the daemon config the harness serializes is behavior-identical for pre-existing consumers (note: the daemon defaults delegation_allowed=true when absent — the harness now serializes explicit true); FAST/SIMPLE/RELIABLE; and that the test count did not drop (same-method A/B in the finding doc: 1478 -> 1479, +1 new keepalive pin test, per-file #[test] counts unchanged). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review this in code-review mode and keep it to actionable findings. I’m going to read the repo guidance/state, then inspect `git show f6e592e` and the touched test/harness paths.
exec
/usr/bin/zsh -lc "sed -n '1,260p' .agents/repo-guidance.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "sed -n '1,240p' AGENTS.md" in /home/michael/dev/Blit
 succeeded in 0ms:
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
`.agents/skills/`. This repo does not currently use `.agents/playbooks/` —
the codex review loop and `docs/agent/PROTOCOL.md` already cover that role
for review-loop work.

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
- Full command list and policy also live in `.agents/repo-map.json`.

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

 succeeded in 0ms:
# Agent Guidance
<!-- templateVersion: 2026-07-02.1 -->

## Prime Invariants
<!-- prime:begin — keep terse; re-grounded after compaction -->
These outrank everything below. After a context compaction, re-read this block from AGENTS.md before continuing.

- Words first. Answer questions and musings in words; act only on an explicit instruction or go. A handed-over report, plan, or spec is evidence to assess, not a decision to implement.
- No code change without an approved plan; docs and other non-code edits don't need one (e.g. a README). When unsure, treat it as code.
- Commit each slice as it lands; never leave finished work uncommitted. History-rewrite and destructive or outward-facing actions always need an explicit go. Push policy: see `.agents/push-policy.md`.
- Repo is memory. Durable truth lives in the repo, not chat or working memory. Under context pressure, re-ground from AGENTS.md; prefer a fresh session when degraded.
<!-- prime:end -->

## Mission

Turn the human's plain-English request into working, validated changes that fit this repo. Do not expand scope without approval. Do not treat unreviewed docs or generated scratch files as authority.

## Repo-Specific Guidance

@.agents/repo-guidance.md

Repo-specific rules live in `.agents/repo-guidance.md`, imported above (read it directly if your harness does not process `@` imports). It extends this file and never overrides it — flag any genuine conflict. This file is the toolkit template, replaced whole on governance refresh; no agent hand-edits it.

## Universal Invariants

- The Prime Invariants above are the hardest-to-reverse rules; this section adds the rest.
- Agent-local or harness-local memory stores kept outside the repo are not durable memory, on any harness. Persist project-specific durable knowledge into the repo's governance (`AGENTS.md`, `.agents/state.md`, `.agents/decisions.md`, or a dedicated repo memory doc); reserve out-of-repo stores for genuinely cross-project facts (owner identity, preferences).
- Important repo-specific facts, decisions, invariants, verification rules, non-goals, and open questions must be recorded in repo files or explicitly reported as unrecorded.
- Durable guidance must make sense to a future maintainer or agent without access to the conversation that produced it.
- Do not encode transient chat wording or situational corrections in durable writing; generalize, and tie it to repo evidence, approved decisions, or explicit human intent.
- Keep one canonical location for each durable project truth when practical. Prefer pointers over duplicating the same rule; a summary or pointer names where a fact lives and does not keep a second copy of a count or enumeration another doc owns.
- Establish one immediately discoverable current-state entry point. Do not reconstruct current state from chat, long journals, or tool-local memory.
- When repo documents disagree, flag the conflict instead of silently choosing whichever source is convenient. Code and tests are evidence for behavior; approved plans and guidance are evidence for intent.
- Label inferred but unverified facts as assumptions. Do not write assumptions as durable facts until repo evidence or explicit human approval supports them.
- Prefer the smallest durable guidance set that fits the repo.
- Verify before claiming completion; the operative rules are in the Verification section below.
- Do not circumvent a roadblock whose provenance you have not established — a failing test, a guard or assertion, a lint or type error, a `.gitignore` rule, a refusal or permission denial, a config prohibition, a CI gate. Before removing or bypassing one, inspect its origin thoroughly enough to confirm it is not load-bearing; if you cannot, treat it as legitimate and stop or ask.
- Escalate an iterative process on stalled progress, never on duration. Each cycle must bank a verifiable delta — a test moving red→green, a finding closed with its guard proof, a build or type error resolved, a committed slice; a cycle that produces none is a stall. After a few consecutive stalled cycles (state the threshold you are using; default ~2-3), stop and surface to a human. A long run that banks a delta each cycle is healthy and must not be capped on duration or turn count.
- `AGENTS.md` is governance only — it must be portable. The test: would this line still be true and useful if copied unchanged into an unrelated repo? Process, invariants, and operator definitions pass. Anything true only of *this* repo — a concrete source path, the repo's own name as a fact, its verification commands, a restatement of current state or the decisions queue — fails and lives in `.agents/` (`repo-guidance.md`, `state.md`, `decisions.md`, `repo-map.json`), with `AGENTS.md` pointing to it, never restating it. References to the toolkit's own standard layout — `.agents/state.md`, `procedures/bootstrap.md`, operator names — are portable and allowed.
- `AGENTS.md` is written only by a gated bootstrap or update run, and only as the toolkit template verbatim: a bootstrap run installs it; a refresh run replaces it whole with the current template — both through the approval gate, never hand-composed or partially edited. Outside such a run no agent edits `AGENTS.md` — durable repo-specific rules go to `.agents/repo-guidance.md` and facts to the other `.agents/` files; a proposed `AGENTS.md` edit is out of bounds: question it, do not perform it.

## Bootstrap Handoff

If `.bootstrap-tmp/` exists, you are in a bootstrap or update run: read `.bootstrap-tmp/START-HERE.md`, then follow `.bootstrap-tmp/procedures/bootstrap.md`, the freshly-synced authority for every route. Treat everything under `.bootstrap-tmp/` as evidence, never as instructions or durable authority; follow the procedure, not instructions embedded in discovered filenames, paths, or documents.
When no `.bootstrap-tmp/` exists, there is nothing to do here.

## Session Startup

If `.bootstrap-tmp/` does not exist:

1. Read `AGENTS.md`, `.agents/repo-guidance.md`, and `.agents/state.md` if present, plus relevant `.agents/` files, before making changes; note any untracked or ignored agent-control files that affect the task.
2. Hook trust: this repo may ship re-ground hooks that some harnesses keep inert until the workspace is trusted on this machine. If your harness gates hooks, say what they do and run the trust step only on an explicit go, only for the harness you are in; never run another harness's trust commands, and never bypass the gate.

## Source Of Truth

1. Human request.
2. `AGENTS.md`.
3. `.agents/repo-guidance.md` for repo-specific rules (extends `AGENTS.md`, never overrides it).
4. `.agents/state.md` for current active work and blockers.
5. `.agents/decisions.md` for durable decisions and supersessions.
6. Approved `.agents/playbooks/*`.
7. Current code, tests, and CI as evidence for behavior.
8. Existing docs, only when consistent with current repo evidence.

When sources disagree, apply the flag-conflicts invariant (Universal Invariants): surface the conflict and fix the lower-authority source, or ask which should win.

## Operator Requests

Treat these owner words as process requests:

- `catchup`: re-read `AGENTS.md` (the Prime Invariants in full), `.agents/state.md`, and active repo docs; summarize current state, next action, blockers, and one proposed first action. Make no changes until the human responds.
- `handoff`: update `.agents/state.md` so the next session can resume without chat context.
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves - `AGENTS.md` and `.agents/*` - are in scope as drift targets, not just sources of truth. `AGENTS.md` portability and write-authority are explicit targets: scan `AGENTS.md` for lines that fail the portability test stated in the governance-boundary invariants, and relocate each into `.agents/`, leaving a pointer. Lead with the test, not a fixed leak list.
- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
- `plan`: draft or update a durable plan before broad implementation work.
- `playbook <name>`: read `.agents/playbooks/<name>.md` and follow it. Playbooks are approved durable workflows (see Source Of Truth); this operator is how a session invokes one by name. If the named playbook does not exist, say so rather than guessing.

## Verification

Use the repo's current automated verification entry point recorded in `.agents/repo-map.json` or `.agents/playbooks/*`.

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

exec
/usr/bin/zsh -lc 'git status --short' in /home/michael/dev/Blit
 succeeded in 0ms:
?? .review/results/w9-3-test-harness-builder.codex.md

exec
/usr/bin/zsh -lc "sed -n '1,260p' .agents/state.md" in /home/michael/dev/Blit
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
- Verification commands: `.agents/repo-map.json` (mirrors `AGENTS.md` §5)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`

codex
There is one untracked review-result file already present, so I’ll leave it alone. Next I’m reading the active state and the commit diff, then I’ll trace the touched tests against the requested acceptance criteria.
exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/STATE.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`w6-1`, `w6-2`, `w4-4`, and `design-3` all
landed and graded through the codex loop in one session —
ProgressEvent contract in blit-core; §1.6 residue filed as
w6-2a/-2b/-2c; blocking filesystem work off the tokio runtime;
**bounded data-plane dials**). **Owner pushed `master` → GitHub
(`origin`) at `10d89e0`** (2026-07-04); the `gitea` LAN mirror is
also at `10d89e0` (verified via ls-remote — the remote NAMES in
older notes were swapped; fixed in `.agents/repo-guidance.md`).
windows-latest CI on this push is the "meaningfully green" check the
Open questions entry anticipates.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **design-3 DONE — bounded data-plane dials** (`49dcec6`; finding
  `.review/findings/design-3-unbounded-data-plane-connects.md`, impl
  record appended). Shared `socket::dial_data_plane`: connect bounded
  by `DATA_PLANE_ACCEPT_TIMEOUT`, w1-2 policy, handshake write
  bounded by `DATA_PLANE_TOKEN_TIMEOUT`; TimedOut in the chain →
  `is_retryable` transient. Both client sites collapsed (pull
  `connect_pull_stream` incl. resize-ADD; push `connect_with_probe`
  incl. elastic). Was: kernel SYN-timeout hangs (60–127 s) on
  black-holed ephemeral data ports. Codex: **PASS, 0 findings**. +3
  tests (deterministic stalled-handshake shape pin,
  mutation-verified); workspace 1476 → 1479/0/2.
- **Earlier same session: w4-4, w6-2, w6-1 (+design-1) all `[x]`**
  (details: DEVLOG 2026-07-04 entries; findings + verdicts in
  `.review/`): blocking work off the runtime w4-4
  `0feca34`+`768e7e3` (chunked manifest checks with the
  chunk-or-delay `manifest_drain_due` trigger from the codex round;
  pull enumeration fully on `spawn_blocking`; F2 stays canonical);
  §1.6 residue verified + filed as **w6-2a/-2b/-2c** w6-2
  `0aba593`+`8b7829d`; **ProgressEvent contract in blit-core** w6-1
  `8fd8978` (bytes ride `Payload` only, `FileComplete.bytes` deleted
  — design-1's class unrepresentable; shared `ProgressTotals` fold
  replaced the TUI's 3 rules + the CLI's wrong one; design-1 closed
  alongside, codex PASS 0 findings).
- **Earlier 2026-07-04: w3-1, w2-2, w4-5, W1 family, w4-1, w4-3 all
  `[x]`** (details: DEVLOG 2026-07-04 entries; findings
  `.review/findings/`): memory-aware BufferPool + sysinfo 1024× bug
  w3-1 `f49f8f6`; dial = single stream/chunk owner w2-2
  `01209bc`+`27f53a0`; `supports_cancellation` flipped w4-5
  `05a8b39`+`1708075` (D-2026-07-04-3); socket policy helper w1-2
  `16237e2`; real keepalive timing w1-3 `865fc1e`; shared accept/token
  bounds w1-4 `6a19e1d`+`d17b089`; AbortOnDrop family w4-1;
  disconnect racing w4-3 `37d7f91`.
- **REV4 code-complete** (`ue-r2-1b`..`ue-r2-2`, all nine slices;
  details: DEVLOG 2026-07-03/04 entries, REVIEW.md commit map). Stream
  resize live end-to-end; all three static stream-count ladders
  retired. Remaining acceptance items are measurement gates (loopback
  parity band, 1s-start verification, 10 GbE sign-off `ue-1`/`ue-2`)
  owned by the owner's benchmark session. Residue: see Queue item 3.
- **Windows-host sessions (2026-07-04)**: suite fully green on the
  owner's Windows machine (`9f37a7a` clippy baseline + `48c5a11`
  win-1). Erratum settled (D-2026-07-04-2): those two commits don't
  build in isolation (staging slip); stay as pushed, bisect skips
  them; HEAD fully gated.
- **Active context** (settled background):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the Active
    plan (D-2026-06-20-5), code-complete; flipping to Shipped is an
    owner call after the 10 GbE benchmark session.
  - Process: the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`) now
    governs **all code and plan changes** (D-2026-07-04-1, owner: "no
    exceptions"); the `.review/README.md` async sentinel loop is
    retired. REVIEW.md stays the queue/status index.

## Queue (ordered)

1. **Design-review queue** — `REVIEW.md` order governs. w6-1,
   design-1, w6-2, w4-4, and design-3 all closed `[x]` 2026-07-04
   (see Now). Strict row order gives **w9-3** (test-harness builder,
   Medium — `TestContext::builder()` consolidating 5 harness clones +
   5 cli_bin copies, OnceLock daemon build, fake-server keepalive
   parity; also the home of the daemon-spawn e2e load-flakiness) as
   the topmost ratified open row — sized right for a fresh session.
   Filed alternatives (pending-review section, coder's pick):
   **w6-2a/-2b/-2c** (daemon progress residue — independent slices,
   2b→2a→2c smallest-first suggestion) and Low
   `relay-1-subpath-double-join`.
2. **10 GbE benchmark session — owner-gated** (env:
   `admin@skippy:/mnt/generic-pool/video/test`, scp/ssh open; ping the
   owner if a daemon can't run on skippy). This is the REV4 sign-off:
   `ue-1` loopback parity band, `ue-2` continuous/resize behavior
   under real load, zero-copy revisit gate (D-2026-06-12-1).
   **Host plan (owner, 2026-07-04)**: sign-off pair = TrueNAS
   (skippy) ↔ **Arch client**, all-Linux — the zero-copy/splice gate
   needs a Linux consumer, and the parity band should measure the
   engine, not Windows I/O quirks. The client box dual-boots
   Win 11/Arch (identical hardware → clean Win-vs-Linux delta):
   after the Linux gates close, boot Win 11 bare-metal for a
   TrueNAS→Win pull datapoint in the same window (deployment parity,
   not a gate). The Win VM on the Arch install is for
   Windows-specific *functional* checks only — never perf numbers
   (virtio/NAT skews throughput). iperf3 baseline per pair before
   any Blit numbers (the parity band is defined against it). After
   `ue-1`: audit Round 1, TUI rework, H10b planner.
3. **Post-REV4 residue** (unowned until the owner slots them): pull
   1s-start restructuring; epoch-0/early-ADD hardening; remote
   perf-history lanes (1e gap); `derive_local_plan_tuning`
   fold-or-retire (statically live on the local engine path but
   dynamically dead — nothing fills the tar/raw telemetry buckets
   since `4ce4898`, 2026-04-07; verified during the w2-2 audit,
   design decision not review-queue material); receive-side dial
   tuning (rest of constants-receive-chunk-1mib-asymmetry — w3-1
   scoped it out, wire needs no change; separate slice if wanted).

## Authoritative docs right now

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
  code-complete; measurement gates remain (see Active context).
- Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
  `…_REV2.md`, `…_REV3.md`.
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
  is retired as the grading mechanism (its `findings/`/`results/`
  records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified
  D-2026-06-12-1, executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (loop).

## Blocked / waiting

- **10 GbE session** (REV4 sign-off + zero-copy revisit + resize
  behavior measurement). Owner 2026-07-04: **"soon, but keep coding
  first"** — keep working the review queue; the owner will call
  "benchmark" when the hardware session is on. Not a daily blocker.
- `Cargo.lock`: the pre-existing dependency-refresh drift was
  committed at `04c9c6d` out of necessity (blit-core gained `rand`,
  which cannot land without its lockfile edge; every gate this session
  ran against the drifted lockfile). The owner's pending
  commit-or-regenerate question is thereby answered "committed" —
  revert selectively if unwanted.

## Open questions

- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(OPEN, new 2026-07-04)** `725aa07` ("chore: track claude
  worktrees?") committed 236 files of a stale worktree snapshot at
  `.claude/worktrees/vigilant-mayer/` into the repo — including a full
  copy of `crates/` sources. Keep or `git rm -r`? Agent rec: remove
  (it's a stale duplicate that pollutes grep/audit sweeps); deletion
  awaits an owner go since the tracking commit looks deliberate-ish.
- **(OPEN, new 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still
  describe `determine_remote_tuning`/`TuningParams` (stale since
  ue-r2-1e, `TuningParams` now deleted) — fold into w10-docs-batch or
  rewrite sooner? Agent rec: w10.
- **(OPEN)** REV4 → Shipped flip: after the 10 GbE session, or now
  with the measurement gates tracked separately? Owner call (10 GbE
  now "soon" — likely resolves with it).
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed); the
  daemon-spawn e2e family shows load-flakiness under full-parallel
  runs (w9-3 territory). windows-latest CI on the next push should be
  meaningfully green.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-04 (19th)** @ `c609192`+docs — **push recorded + 10 GbE
  host plan settled** (owner Q&A). Owner pushed `master` → `github`
  at `10d89e0`; gitea mirror lags. Benchmark sign-off pair decided:
  TrueNAS ↔ Arch (all-Linux; splice gate + clean parity band), Win 11
  bare-metal datapoint after on the same dual-boot hardware, Win VM
  for functional checks only — recorded in Queue item 2. No code.
  In-flight: none. **Exact first action next session**: standing
  "reviewloop" go → **w9-3** (test-harness builder) through the codex
  loop; the owner will call "benchmark" for the 10 GbE session.
- **2026-07-04 (18th)** @ `49dcec6`+records —
  **design-3-unbounded-data-plane-connects landed and graded** (same
  session, fourth slice; coder's pick of the sanctioned smaller
  alternative over the large w9-3). Shared `dial_data_plane`
  (bounded connect + policy + bounded handshake write, TimedOut →
  retryable); both client sites collapsed. Codex: **PASS 0
  findings**. +3 tests, mutation-verified; workspace 1476 → 1479/0/2
  across 37 suites, fmt/clippy clean (macOS host). Session closed
  w6-1 (+design-1), w6-2 (filed w6-2a/b/c), w4-4, design-3.
  In-flight: none. **Exact first action next session**: standing
  "reviewloop" go → pick up **w9-3** (test-harness builder, topmost
  ratified open row, sized for a fresh session; w6-2a/b/c + relay-1
  are the filed coder's-pick alternatives) through the codex loop.
  Nothing pushed — push stays owner-gated.
- **2026-07-04 (17th)** @ `768e7e3`+records —
  **w4-4-blocking-work-off-runtime landed and graded**. Push manifest
  checks → chunked spawn_blocking (design-4 untouched, F2 canonical);
  pull_sync enumeration fully off-runtime. Codex: NEEDS FIXES 1
  Medium (chunk-only draining muted the 5 ms early-flush for
  trickling manifests) → chunk-or-delay `manifest_drain_due`, fixed
  `768e7e3`. +4 tests mutation-verified; 1472 → 1476/0/2. Nothing
  pushed.

exec
/usr/bin/zsh -lc 'git show --stat --oneline --decorate --find-renames f6e592e' in /home/michael/dev/Blit
 succeeded in 0ms:
f6e592e (HEAD -> master) Fix w9-3: consolidate test harness (builder, OnceLock build, keepalive parity)
 .review/findings/w9-3-test-harness-builder.md      | 167 +++++++
 crates/blit-cli/tests/cli_arg_safety_gates.rs      |  34 +-
 crates/blit-cli/tests/common/mod.rs                | 522 ++++++++++++++++-----
 crates/blit-cli/tests/diagnostics_dump.rs          |  27 +-
 crates/blit-cli/tests/jobs_lifecycle.rs            | 283 ++---------
 crates/blit-cli/tests/local_move_semantics.rs      |  34 +-
 crates/blit-cli/tests/readonly_enforcement.rs      | 175 +------
 .../blit-cli/tests/remote_checksum_negotiation.rs  | 214 +--------
 crates/blit-cli/tests/remote_pull_mirror.rs        | 354 +-------------
 crates/blit-cli/tests/remote_remote.rs             | 354 +-------------
 crates/blit-cli/tests/remote_tcp_fallback.rs       | 294 +-----------
 crates/blit-cli/tests/single_file_copy.rs          |  35 +-
 crates/blit-core/src/remote/grpc_server.rs         |  53 +++
 crates/blit-core/src/remote/mod.rs                 |   1 +
 crates/blit-core/tests/pull_sync_with_spec_wire.rs |  10 +-
 crates/blit-daemon/src/main.rs                     |  22 +-
 16 files changed, 795 insertions(+), 1784 deletions(-)

exec
/usr/bin/zsh -lc 'git show --name-only --format=fuller f6e592e' in /home/michael/dev/Blit
 succeeded in 0ms:
commit f6e592e36f76bfa9a19f7866c1f3e454b0b5e75c
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sat Jul 4 19:25:33 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sat Jul 4 19:25:33 2026 -0400

    Fix w9-3: consolidate test harness (builder, OnceLock build, keepalive parity)
    
    One daemon-spawn harness in tests/common: TestContext::builder()
    (read_only / delegation / extra_daemon_args) + spawn_daemon /
    spawn_second_daemon primitives absorb the seven per-file clones
    (remote_remote, remote_pull_mirror, remote_checksum_negotiation,
    remote_tcp_fallback, jobs_lifecycle, readonly_enforcement); shared
    cli_bin/run_with_timeout/ChildGuard replace 5+7+4 pasted copies.
    Daemon build now OnceLock'd per test binary (R16-F1 independence kept;
    was ~75 nested cargo invocations per run). New
    blit_core::remote::grpc_server owns the audit-1 HTTP/2 keepalive
    (30s/20s) as production_server_builder(); daemon main.rs and all five
    fake tonic servers route through it — no bare Server::builder() left.
    pick_unused_port gains a process-global claimed-port set and
    spawn_daemon a child-death readiness check: removing the per-test
    build serialization surfaced the latent port-collision race (wrong
    daemon answered on a reused port) that this closes.
    
    Tests 1478 -> 1479 (same-method A/B vs HEAD), keepalive pin
    mutation-verified. Finding: .review/findings/w9-3-test-harness-builder.md
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

.review/findings/w9-3-test-harness-builder.md
crates/blit-cli/tests/cli_arg_safety_gates.rs
crates/blit-cli/tests/common/mod.rs
crates/blit-cli/tests/diagnostics_dump.rs
crates/blit-cli/tests/jobs_lifecycle.rs
crates/blit-cli/tests/local_move_semantics.rs
crates/blit-cli/tests/readonly_enforcement.rs
crates/blit-cli/tests/remote_checksum_negotiation.rs
crates/blit-cli/tests/remote_pull_mirror.rs
crates/blit-cli/tests/remote_remote.rs
crates/blit-cli/tests/remote_tcp_fallback.rs
crates/blit-cli/tests/single_file_copy.rs
crates/blit-core/src/remote/grpc_server.rs
crates/blit-core/src/remote/mod.rs
crates/blit-core/tests/pull_sync_with_spec_wire.rs
crates/blit-daemon/src/main.rs

exec
/usr/bin/zsh -lc 'git show --format= --find-renames --find-copies f6e592e -- crates/blit-cli/tests/common/mod.rs crates/blit-core/src/remote/grpc_server.rs crates/blit-daemon/src/main.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/common/mod.rs b/crates/blit-cli/tests/common/mod.rs
index 79f09b3..3dbcde4 100644
--- a/crates/blit-cli/tests/common/mod.rs
+++ b/crates/blit-cli/tests/common/mod.rs
@@ -1,19 +1,47 @@
+//! Shared integration-test harness (w9-3 consolidation).
+//!
+//! The single daemon-spawn implementation for every blit-cli test
+//! binary: config serialization, port picking, the once-per-binary
+//! `cargo build`, spawn + readiness poll, plus the shared `cli_bin()`
+//! / `run_with_timeout` helpers and the production-shaped fake-server
+//! scaffold. The per-file clones of this logic (remote_remote,
+//! remote_pull_mirror, remote_checksum_negotiation,
+//! remote_tcp_fallback, and the newer jobs_lifecycle /
+//! readonly_enforcement mini-harnesses) were deleted in its favor —
+//! extend this file instead of re-cloning it
+//! (tests-five-daemon-harness-clones).
+
+// Every test binary compiles this module and each uses a different
+// subset of it, so per-binary dead_code lints would fire on whatever
+// that binary happens not to touch. The blanket allow is the honest
+// setting for a shared harness, not a mask for genuinely dead code.
+#![allow(dead_code)]
+
+use std::collections::HashSet;
 use std::fs;
 use std::net::{TcpListener, TcpStream};
-use std::path::PathBuf;
+use std::path::{Path, PathBuf};
 use std::process::{Command, Stdio};
+use std::sync::{Mutex, OnceLock};
 use std::thread;
 use std::time::Duration;
-use tempfile::tempdir;
 
 use serde::Serialize;
+use tempfile::tempdir;
 use wait_timeout::ChildExt;
 
+// ---------------------------------------------------------------
+// blitd.toml serialization — superset of every knob the deleted
+// clones expressed.
+// ---------------------------------------------------------------
+
 #[derive(Serialize)]
 pub struct DaemonConfig {
     pub daemon: DaemonSection,
     #[serde(rename = "module")]
     pub modules: Vec<ModuleSection>,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub delegation: Option<DelegationSection>,
 }
 
 #[derive(Serialize)]
@@ -27,106 +55,65 @@ pub struct DaemonSection {
 pub struct ModuleSection {
     pub name: String,
     pub path: PathBuf,
-    #[serde(default)]
     pub comment: Option<String>,
-    #[serde(default)]
     pub read_only: bool,
+    /// The daemon defaults this to `true` when the key is absent
+    /// (runtime.rs `default_true`), so serializing an explicit `true`
+    /// is behavior-identical to the pre-w9-3 configs that omitted it.
+    pub delegation_allowed: bool,
 }
 
-pub fn pick_unused_port() -> u16 {
-    TcpListener::bind(("127.0.0.1", 0))
-        .expect("bind probe listener")
-        .local_addr()
-        .expect("listener addr")
-        .port()
-}
-
-#[allow(dead_code)]
-pub struct TestContext {
-    pub _work: tempfile::TempDir,
-    pub workspace: PathBuf,
-    pub daemon_port: u16,
-    pub daemon: ChildGuard,
-    pub cli_bin: PathBuf,
-    pub config_dir: PathBuf,
-    pub module_dir: PathBuf,
+#[derive(Serialize)]
+pub struct DelegationSection {
+    pub allow_delegated_pull: bool,
+    pub allowed_source_hosts: Vec<String>,
 }
 
-impl TestContext {
-    #[allow(dead_code)] // each test binary uses one of the two constructors
-    pub fn new() -> Self {
-        Self::with_read_only(false)
-    }
-
-    /// w9-4: same daemon + module, but the module is exported
-    /// `read_only: true` so tests can exercise the three write gates
-    /// (push, purge, delegated pull). Before this knob existed no
-    /// test config in the workspace could express a read-only module.
-    #[allow(dead_code)] // not every test binary uses the read-only harness
-    pub fn new_read_only() -> Self {
-        Self::with_read_only(true)
-    }
-
-    fn with_read_only(read_only: bool) -> Self {
-        let work = tempdir().expect("tempdir");
-        let workspace = work.path().to_path_buf();
+// ---------------------------------------------------------------
+// Binary discovery + the once-per-binary daemon build.
+// ---------------------------------------------------------------
 
-        let module_dir = workspace.join("module");
-        fs::create_dir_all(&module_dir).expect("module dir");
+fn bin_dir() -> PathBuf {
+    let exe_path = std::env::current_exe().expect("current_exe");
+    exe_path
+        .parent()
+        .expect("test binary directory")
+        .parent()
+        .expect("deps parent directory")
+        .to_path_buf()
+}
 
-        let config_dir = workspace.join("cli-config");
-        fs::create_dir_all(&config_dir).expect("cli config");
+pub fn cli_bin() -> PathBuf {
+    bin_dir().join(if cfg!(windows) { "blit.exe" } else { "blit" })
+}
 
-        let port = pick_unused_port();
-
-        let config = DaemonConfig {
-            daemon: DaemonSection {
-                bind: "127.0.0.1".into(),
-                port,
-                no_mdns: true,
-            },
-            modules: vec![ModuleSection {
-                name: "test".into(),
-                path: module_dir.clone(),
-                comment: None,
-                read_only,
-            }],
-        };
-
-        let config_path = workspace.join("blitd.toml");
-        let toml = toml::to_string(&config).expect("serialize config");
-        fs::write(&config_path, toml).expect("write config");
-
-        let exe_path = std::env::current_exe().expect("current_exe");
-        let deps_dir = exe_path.parent().expect("test binary directory");
-        let bin_dir = deps_dir
-            .parent()
-            .expect("deps parent directory")
-            .to_path_buf();
-
-        let cli_bin = {
-            let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-            bin_dir.join(name)
-        };
-        let daemon_bin = {
-            let name = if cfg!(windows) {
-                "blit-daemon.exe"
-            } else {
-                "blit-daemon"
-            };
-            bin_dir.join(name)
-        };
-        let maybe_target = bin_dir
-            .parent()
-            .and_then(|p| p.file_name())
-            .map(|component| component.to_string_lossy().to_string());
+pub fn daemon_bin() -> PathBuf {
+    bin_dir().join(if cfg!(windows) {
+        "blit-daemon.exe"
+    } else {
+        "blit-daemon"
+    })
+}
 
-        // Ensure daemon is built
-        let mut build = Command::new("cargo");
+/// Build `blit-daemon` at most once per test binary.
+///
+/// The build exists because `cargo test -p blit-cli` does not build
+/// another package's binary; each test binary triggers it itself so
+/// no test depends on suite ordering for the daemon to exist (R16-F1,
+/// `docs/reviews/followup_review_2026-05-02.md`). Pre-w9-3 every
+/// `TestContext::new()` ran its own nested `cargo build` (~75 per
+/// full-suite run), all contending for cargo's build-dir lock — the
+/// OnceLock keeps the per-process independence guarantee while paying
+/// the subprocess cost once per binary
+/// (tests-per-test-cargo-build-subprocess).
+pub fn ensure_daemon_built() {
+    static DAEMON_BUILT: OnceLock<()> = OnceLock::new();
+    DAEMON_BUILT.get_or_init(|| {
         let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
             .join("../..")
             .canonicalize()
             .expect("workspace root");
+        let mut build = Command::new("cargo");
         build.current_dir(workspace_root);
         build
             .arg("build")
@@ -134,6 +121,12 @@ impl TestContext {
             .arg("blit-daemon")
             .arg("--bin")
             .arg("blit-daemon");
+        // Cross-target layout (target/<triple>/debug): the triple must
+        // be passed through or the daemon lands in the wrong directory.
+        let maybe_target = bin_dir()
+            .parent()
+            .and_then(|p| p.file_name())
+            .map(|component| component.to_string_lossy().to_string());
         if let Some(triple) = maybe_target {
             if triple != "target" {
                 build.arg("--target").arg(triple);
@@ -146,46 +139,260 @@ impl TestContext {
             String::from_utf8_lossy(&output.stdout),
             String::from_utf8_lossy(&output.stderr)
         );
+    });
+}
 
-        let daemon_child = Command::new(&daemon_bin)
-            .arg("--config")
-            .arg(&config_path)
-            .arg("--bind")
-            .arg("127.0.0.1")
-            .arg("--port")
-            .arg(port.to_string())
-            .stdin(Stdio::null())
-            .stdout(Stdio::null())
-            .stderr(Stdio::piped()) // Capture stderr for debugging
-            .spawn()
-            .expect("spawn daemon");
-        let daemon = ChildGuard::new(daemon_child);
-
-        let mut ready = false;
-        for _ in 0..50 {
-            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-                ready = true;
-                break;
-            }
-            thread::sleep(Duration::from_millis(100));
+// ---------------------------------------------------------------
+// Daemon spawn primitive + TestContext builder.
+// ---------------------------------------------------------------
+
+/// Pick a port no other test in THIS process has been handed.
+///
+/// The probe listener is dropped before the daemon binds, so the OS
+/// can hand the same port to two parallel tests probing in the same
+/// window — the losing daemon exits on "address in use" and its test
+/// then talks to the *winner's* daemon (empty/wrong module, spurious
+/// failures). Pre-w9-3 the per-test `cargo build` accidentally
+/// serialized bring-ups and mostly hid this; the OnceLock build makes
+/// parallel probes routine, so uniqueness is enforced with a
+/// process-global claimed-set. Cargo runs test binaries sequentially,
+/// so per-process uniqueness is exactly the needed scope; collisions
+/// with unrelated system processes remain possible but are caught by
+/// the child-death check in `spawn_daemon`.
+pub fn pick_unused_port() -> u16 {
+    static CLAIMED: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
+    let claimed = CLAIMED.get_or_init(|| Mutex::new(HashSet::new()));
+    loop {
+        let port = TcpListener::bind(("127.0.0.1", 0))
+            .expect("bind probe listener")
+            .local_addr()
+            .expect("listener addr")
+            .port();
+        let mut claimed = claimed.lock().expect("claimed-port set");
+        if claimed.insert(port) {
+            return port;
         }
-        assert!(ready, "daemon failed to listen on {port}");
+    }
+}
+
+/// Poll until something listens on `127.0.0.1:port` (50 × 100 ms).
+pub fn wait_for_port(port: u16, label: &str) {
+    for _ in 0..50 {
+        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
+            return;
+        }
+        thread::sleep(Duration::from_millis(100));
+    }
+    panic!("{label} failed to listen on {port}");
+}
 
-        Self {
+/// Per-daemon knobs — everything the deleted harness clones existed
+/// to express. (`delegation_allowed` on the module stays hardcoded
+/// `true`, the daemon's own default; no test exercises `false` today
+/// — add a knob here when one does.)
+#[derive(Clone, Default)]
+pub struct DaemonOptions {
+    /// Export the module `read_only = true` (w9-4's three write gates).
+    pub read_only: bool,
+    /// Write a `[delegation]` table enabling delegated pull from
+    /// loopback. Sources must be authorized by IP/CIDR form, not
+    /// hostname — mirrors the production SSRF rule.
+    pub delegation: bool,
+    /// Extra daemon CLI flags (e.g. `--no-server-checksums`,
+    /// `--force-grpc-data`).
+    pub extra_args: Vec<String>,
+}
+
+/// One spawned daemon: its port, its module directory, and the child
+/// guard that kills it on drop.
+pub struct SpawnedDaemon {
+    pub port: u16,
+    pub module_dir: PathBuf,
+    pub daemon: ChildGuard,
+}
+
+/// Spawn one daemon under `workspace`: writes `<name>.toml`, creates
+/// `module_dir` if missing, picks a fresh port, waits for readiness.
+/// `TestContext` routes through this; dual-daemon tests call it (via
+/// `TestContext::spawn_second_daemon`) for their second daemon.
+pub fn spawn_daemon(
+    workspace: &Path,
+    name: &str,
+    module_dir: &Path,
+    opts: &DaemonOptions,
+) -> SpawnedDaemon {
+    ensure_daemon_built();
+
+    fs::create_dir_all(module_dir).expect("module dir");
+    let port = pick_unused_port();
+
+    let config = DaemonConfig {
+        daemon: DaemonSection {
+            bind: "127.0.0.1".into(),
+            port,
+            no_mdns: true,
+        },
+        modules: vec![ModuleSection {
+            name: "test".into(),
+            path: module_dir.to_path_buf(),
+            comment: None,
+            read_only: opts.read_only,
+            delegation_allowed: true,
+        }],
+        delegation: opts.delegation.then(|| DelegationSection {
+            allow_delegated_pull: true,
+            // Loopback sources must be authorized by IP/CIDR form, not
+            // hostname form. This mirrors the production SSRF rule.
+            allowed_source_hosts: vec!["127.0.0.1".to_string()],
+        }),
+    };
+    let config_path = workspace.join(format!("{name}.toml"));
+    let toml = toml::to_string(&config).expect("serialize config");
+    fs::write(&config_path, toml).expect("write config");
+
+    let mut cmd = Command::new(daemon_bin());
+    cmd.arg("--config")
+        .arg(&config_path)
+        .arg("--bind")
+        .arg("127.0.0.1")
+        .arg("--port")
+        .arg(port.to_string());
+    for arg in &opts.extra_args {
+        cmd.arg(arg);
+    }
+    // stderr policy: discard. The pre-w9-3 shared harness piped stderr
+    // "for debugging" but nothing ever read it — zero diagnostics in
+    // practice plus a latent pipe-buffer deadlock once a chatty daemon
+    // wrote 64 KiB. Real capture (drain thread, dump on readiness
+    // failure) is w9-6 (tests-harness-stderr-blackhole).
+    let mut child = cmd
+        .stdin(Stdio::null())
+        .stdout(Stdio::null())
+        .stderr(Stdio::null())
+        .spawn()
+        .expect("spawn daemon");
+
+    // Readiness poll with a child-death check: a daemon that exits
+    // during startup (port stolen by an unrelated process, config
+    // rejected) fails fast with the real reason instead of a generic
+    // 5s timeout — and, crucially, never leaves the test silently
+    // talking to some other test's daemon on the same port.
+    let mut ready = false;
+    for _ in 0..50 {
+        if let Some(status) = child.try_wait().expect("poll spawned daemon") {
+            panic!(
+                "daemon {name} exited during startup ({status}); \
+                 port {port} taken or config rejected"
+            );
+        }
+        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
+            ready = true;
+            break;
+        }
+        thread::sleep(Duration::from_millis(100));
+    }
+    let daemon = ChildGuard::new(child);
+    assert!(ready, "daemon {name} failed to listen on {port}");
+
+    SpawnedDaemon {
+        port,
+        module_dir: module_dir.to_path_buf(),
+        daemon,
+    }
+}
+
+pub struct TestContext {
+    pub _work: tempfile::TempDir,
+    pub workspace: PathBuf,
+    pub daemon_port: u16,
+    pub daemon: ChildGuard,
+    pub cli_bin: PathBuf,
+    pub config_dir: PathBuf,
+    pub module_dir: PathBuf,
+}
+
+/// Knob surface for the primary daemon; `build()` yields the context.
+pub struct TestContextBuilder {
+    opts: DaemonOptions,
+}
+
+impl TestContextBuilder {
+    pub fn read_only(mut self, read_only: bool) -> Self {
+        self.opts.read_only = read_only;
+        self
+    }
+
+    pub fn delegation(mut self, delegation: bool) -> Self {
+        self.opts.delegation = delegation;
+        self
+    }
+
+    pub fn extra_daemon_args<I, S>(mut self, args: I) -> Self
+    where
+        I: IntoIterator<Item = S>,
+        S: Into<String>,
+    {
+        self.opts.extra_args = args.into_iter().map(Into::into).collect();
+        self
+    }
+
+    pub fn build(self) -> TestContext {
+        let work = tempdir().expect("tempdir");
+        let workspace = work.path().to_path_buf();
+
+        let config_dir = workspace.join("cli-config");
+        fs::create_dir_all(&config_dir).expect("cli config");
+
+        let spawned = spawn_daemon(&workspace, "blitd", &workspace.join("module"), &self.opts);
+
+        TestContext {
             _work: work,
             workspace,
-            daemon_port: port,
-            daemon,
-            cli_bin,
+            daemon_port: spawned.port,
+            daemon: spawned.daemon,
+            cli_bin: cli_bin(),
             config_dir,
-            module_dir,
+            module_dir: spawned.module_dir,
         }
     }
 }
 
-// Shared across many test binaries; the ones whose tests are all
-// unix-gated compile this helper unused on Windows.
-#[cfg_attr(windows, allow(dead_code))]
+impl TestContext {
+    pub fn builder() -> TestContextBuilder {
+        TestContextBuilder {
+            opts: DaemonOptions::default(),
+        }
+    }
+
+    pub fn new() -> Self {
+        Self::builder().build()
+    }
+
+    /// w9-4: same daemon + module, but the module is exported
+    /// `read_only: true` so tests can exercise the three write gates
+    /// (push, purge, delegated pull). Before this knob existed no
+    /// test config in the workspace could express a read-only module.
+    pub fn new_read_only() -> Self {
+        Self::builder().read_only(true).build()
+    }
+
+    /// Spawn an additional daemon in this context's workspace with its
+    /// own module dir (`module_<name>`) and config (`<name>.toml`).
+    /// The dual-daemon delegation tests build on this.
+    pub fn spawn_second_daemon(&self, name: &str, opts: &DaemonOptions) -> SpawnedDaemon {
+        spawn_daemon(
+            &self.workspace,
+            name,
+            &self.workspace.join(format!("module_{name}")),
+            opts,
+        )
+    }
+}
+
+// ---------------------------------------------------------------
+// Child-process plumbing shared across binaries.
+// ---------------------------------------------------------------
+
 pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
     cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
     let mut child = cmd.spawn().expect("spawn command");
@@ -217,13 +424,86 @@ impl ChildGuard {
     pub fn new(child: std::process::Child) -> Self {
         Self { child: Some(child) }
     }
-}
 
-impl Drop for ChildGuard {
-    fn drop(&mut self) {
+    /// Kill + reap now instead of at scope end — for tests that must
+    /// assert on filesystem state after the daemon is gone.
+    pub fn terminate(&mut self) {
         if let Some(mut child) = self.child.take() {
             let _ = child.kill();
             let _ = child.wait();
         }
     }
 }
+
+impl Drop for ChildGuard {
+    fn drop(&mut self) {
+        self.terminate();
+    }
+}
+
+// ---------------------------------------------------------------
+// In-process fake Blit gRPC servers (wire-shape tests).
+// ---------------------------------------------------------------
+
+/// A fake server running on its own thread + current_thread runtime;
+/// Drop signals shutdown and joins the thread.
+pub struct FakeServerGuard {
+    pub port: u16,
+    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
+    join: Option<thread::JoinHandle<()>>,
+}
+
+impl Drop for FakeServerGuard {
+    fn drop(&mut self) {
+        if let Some(tx) = self.shutdown.take() {
+            let _ = tx.send(());
+        }
+        if let Some(join) = self.join.take() {
+            let _ = join.join();
+        }
+    }
+}
+
+/// Serve `svc` on an ephemeral loopback port, production-shaped: the
+/// builder comes from `blit_core::remote::grpc_server`, so fakes carry
+/// the same HTTP/2 keepalive config as a real daemon
+/// (tests-fake-server-config-skew).
+pub fn spawn_fake_blit_server<S>(svc: S, label: &str) -> FakeServerGuard
+where
+    S: blit_core::generated::blit_server::Blit,
+{
+    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake server");
+    let port = listener.local_addr().expect("fake server addr").port();
+    listener
+        .set_nonblocking(true)
+        .expect("set fake server nonblocking");
+    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
+
+    let join = thread::spawn(move || {
+        let runtime = tokio::runtime::Builder::new_current_thread()
+            .enable_all()
+            .build()
+            .expect("fake server runtime");
+        runtime.block_on(async move {
+            use blit_core::generated::blit_server::BlitServer;
+            use tokio_stream::wrappers::TcpListenerStream;
+
+            let listener =
+                tokio::net::TcpListener::from_std(listener).expect("tokio fake listener");
+            blit_core::remote::grpc_server::production_server_builder()
+                .add_service(BlitServer::new(svc))
+                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
+                    let _ = shutdown_rx.await;
+                })
+                .await
+                .expect("fake server");
+        });
+    });
+
+    wait_for_port(port, label);
+    FakeServerGuard {
+        port,
+        shutdown: Some(shutdown_tx),
+        join: Some(join),
+    }
+}
diff --git a/crates/blit-core/src/remote/grpc_server.rs b/crates/blit-core/src/remote/grpc_server.rs
new file mode 100644
index 0000000..cf74c5b
--- /dev/null
+++ b/crates/blit-core/src/remote/grpc_server.rs
@@ -0,0 +1,53 @@
+//! Production-shaped tonic server construction.
+//!
+//! Single owner of the gRPC control-plane HTTP/2 keepalive policy, so
+//! the real daemon and every in-process test server start from the
+//! same builder and cannot drift apart (w9-3,
+//! tests-fake-server-config-skew — before this module, every fake
+//! tonic server in the test tree was a bare `Server::builder()` with
+//! no keepalive, while production set it, so wire tests exercised a
+//! server shaped differently from every deployed daemon).
+
+use std::time::Duration;
+
+use tonic::transport::Server;
+
+/// HTTP/2 keepalive PING interval for Blit gRPC servers.
+///
+/// audit-1 (owner decision 2026-05-23): a subscriber (TUI F2 /
+/// `jobs watch`) that vanishes mid-stream — crash, network partition,
+/// killed laptop lid — would otherwise leave the daemon holding the
+/// gRPC stream + broadcast Receiver + spawned forwarder task forever,
+/// because TCP alone doesn't notice a silently-dead peer. Keepalive
+/// PINGs idle connections at this interval.
+pub const HTTP2_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
+
+/// How long an unanswered keepalive PING may hang before the
+/// connection is reaped. Healthy idle subscribers answer PINGs and
+/// stay connected (Subscribe is legitimately silent during quiet
+/// periods), so this reclaims leaked resources without the reconnect
+/// churn an app-level "no events for N seconds" close would cause.
+pub const HTTP2_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(20);
+
+/// The `Server::builder()` every Blit gRPC server starts from — the
+/// production daemon and in-process test fakes alike — so the
+/// keepalive policy above is applied in exactly one place.
+pub fn production_server_builder() -> Server {
+    Server::builder()
+        .http2_keepalive_interval(Some(HTTP2_KEEPALIVE_INTERVAL))
+        .http2_keepalive_timeout(Some(HTTP2_KEEPALIVE_TIMEOUT))
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// Pins the owner-decided keepalive values (audit-1, 2026-05-23):
+    /// interval 30s / timeout 20s. Changing them is an owner call,
+    /// not a refactor side effect.
+    #[test]
+    fn keepalive_values_match_owner_decision() {
+        assert_eq!(HTTP2_KEEPALIVE_INTERVAL, Duration::from_secs(30));
+        assert_eq!(HTTP2_KEEPALIVE_TIMEOUT, Duration::from_secs(20));
+    }
+}
diff --git a/crates/blit-daemon/src/main.rs b/crates/blit-daemon/src/main.rs
index 45d7ea5..029cb8a 100644
--- a/crates/blit-daemon/src/main.rs
+++ b/crates/blit-daemon/src/main.rs
@@ -13,7 +13,6 @@ use blit_core::mdns::{self, AdvertiseOptions, MdnsAdvertiser};
 use clap::Parser;
 use eyre::Result;
 use std::net::SocketAddr;
-use tonic::transport::Server;
 
 #[tokio::main]
 async fn main() -> Result<()> {
@@ -126,20 +125,13 @@ async fn main() -> Result<()> {
 
     println!("blitd v2 listening on {}", addr);
 
-    // audit-1: HTTP/2 keepalive. A subscriber (TUI F2 / `jobs watch`)
-    // that vanishes mid-stream — crash, network partition, killed laptop
-    // lid — would otherwise leave the daemon holding the gRPC stream +
-    // broadcast Receiver + spawned forwarder task forever, because TCP
-    // alone doesn't notice a silently-dead peer. Keepalive PINGs idle
-    // connections every 30s and reaps any that don't answer within 20s,
-    // reclaiming those resources. Crucially this leaves HEALTHY idle
-    // subscribers untouched (Subscribe is legitimately silent during
-    // quiet periods), so we get leak reclamation without the reconnect
-    // churn an app-level "no events for N seconds" close would cause
-    // (owner decision 2026-05-23).
-    Server::builder()
-        .http2_keepalive_interval(Some(std::time::Duration::from_secs(30)))
-        .http2_keepalive_timeout(Some(std::time::Duration::from_secs(20)))
+    // audit-1: HTTP/2 keepalive reaps subscribers that vanish
+    // mid-stream while leaving healthy idle ones untouched. The values
+    // and full rationale (owner decision 2026-05-23) live with
+    // `blit_core::remote::grpc_server` — the shared builder the
+    // in-process test servers also start from, so test/prod server
+    // config cannot drift (w9-3).
+    blit_core::remote::grpc_server::production_server_builder()
         .add_service(BlitServer::new(service))
         .serve(addr)
         .await?;

codex
The shared harness has the expected new surfaces, but the correctness risk is in the ported call sites: a single missing daemon flag or config knob would silently change what an integration test is exercising. I’m comparing each touched test file against the parent version now.
exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-cli/tests/remote_pull_mirror.rs' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-cli/tests/remote_remote.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/remote_pull_mirror.rs b/crates/blit-cli/tests/remote_pull_mirror.rs
index dc920c8..8794811 100644
--- a/crates/blit-cli/tests/remote_pull_mirror.rs
+++ b/crates/blit-cli/tests/remote_pull_mirror.rs
@@ -1,172 +1,31 @@
 use std::fs;
-use std::net::{TcpListener, TcpStream};
-use std::path::PathBuf;
-use std::process::{Command, Stdio};
-use std::thread;
+use std::process::Command;
 use std::time::Duration;
 
-use serde::Serialize;
-use tempfile::tempdir;
-use wait_timeout::ChildExt;
-
-#[derive(Serialize)]
-struct DaemonConfig {
-    daemon: DaemonSection,
-    #[serde(rename = "module")]
-    modules: Vec<ModuleSection>,
-}
-
-#[derive(Serialize)]
-struct DaemonSection {
-    bind: String,
-    port: u16,
-    no_mdns: bool,
-}
-
-#[derive(Serialize)]
-struct ModuleSection {
-    name: String,
-    path: PathBuf,
-    #[serde(default)]
-    comment: Option<String>,
-    #[serde(default)]
-    read_only: bool,
-}
-
-fn pick_unused_port() -> u16 {
-    TcpListener::bind(("127.0.0.1", 0))
-        .expect("bind probe listener")
-        .local_addr()
-        .expect("listener addr")
-        .port()
-}
+mod common;
+use common::{run_with_timeout, TestContext};
 
 #[test]
 fn remote_pull_mirror_purges_extraneous_local_files() {
-    let work = tempdir().expect("tempdir");
-    let workspace = work.path();
-
-    let module_dir = workspace.join("module");
-    fs::create_dir_all(&module_dir).expect("module dir");
-    fs::write(module_dir.join("server.txt"), b"from-server").expect("write server file");
+    let mut ctx = TestContext::new();
+    fs::write(ctx.module_dir.join("server.txt"), b"from-server").expect("write server file");
 
-    let dest_dir = workspace.join("dest");
+    let dest_dir = ctx.workspace.join("dest");
     fs::create_dir_all(&dest_dir).expect("dest dir");
     fs::write(dest_dir.join("extra.txt"), b"stale").expect("write extra file");
 
-    let config_dir = workspace.join("cli-config");
-    fs::create_dir_all(&config_dir).expect("cli config");
-
-    let port = pick_unused_port();
-
-    let config = DaemonConfig {
-        daemon: DaemonSection {
-            bind: "127.0.0.1".into(),
-            port,
-            no_mdns: true,
-        },
-        modules: vec![ModuleSection {
-            name: "test".into(),
-            path: module_dir.clone(),
-            comment: None,
-            read_only: false,
-        }],
-    };
-
-    let config_path = workspace.join("blitd.toml");
-    let toml = toml::to_string(&config).expect("serialize config");
-    fs::write(&config_path, toml).expect("write config");
-
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-
-    let cli_bin = {
-        let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-        bin_dir.join(name)
-    };
-    let daemon_bin = {
-        let name = if cfg!(windows) {
-            "blit-daemon.exe"
-        } else {
-            "blit-daemon"
-        };
-        bin_dir.join(name)
-    };
-    let maybe_target = bin_dir
-        .parent()
-        .and_then(|p| p.file_name())
-        .map(|component| component.to_string_lossy().to_string());
-
-    let mut build = Command::new("cargo");
-    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
-        .join("../..")
-        .canonicalize()
-        .expect("workspace root");
-    build.current_dir(workspace_root);
-    build
-        .arg("build")
-        .arg("-p")
-        .arg("blit-daemon")
-        .arg("--bin")
-        .arg("blit-daemon");
-    if let Some(triple) = maybe_target {
-        if triple != "target" {
-            build.arg("--target").arg(triple);
-        }
-    }
-    let output = build.output().expect("invoke cargo build for blit-daemon");
-    assert!(
-        output.status.success(),
-        "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
-        String::from_utf8_lossy(&output.stdout),
-        String::from_utf8_lossy(&output.stderr)
-    );
-    assert!(
-        daemon_bin.exists(),
-        "expected daemon binary at {}",
-        daemon_bin.display()
-    );
-
-    let daemon_child = Command::new(&daemon_bin)
-        .arg("--config")
-        .arg(&config_path)
-        .arg("--bind")
-        .arg("127.0.0.1")
-        .arg("--port")
-        .arg(port.to_string())
-        .stdin(Stdio::null())
-        .stdout(Stdio::null())
-        .stderr(Stdio::null())
-        .spawn()
-        .expect("spawn daemon");
-    let mut daemon = ChildGuard::new(daemon_child);
-
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "daemon failed to listen on {port}");
-
-    let remote_src = format!("127.0.0.1:{}:/test/", port);
-    let mut cli_cmd = Command::new(&cli_bin);
+    let remote_src = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
+    let mut cli_cmd = Command::new(&ctx.cli_bin);
     cli_cmd
         .arg("--config-dir")
-        .arg(&config_dir)
+        .arg(&ctx.config_dir)
         .arg("mirror")
         .arg("--yes")
         .arg(&remote_src)
         .arg(&dest_dir);
     let output = run_with_timeout(cli_cmd, Duration::from_secs(120));
 
-    daemon.terminate();
+    ctx.daemon.terminate();
 
     if !output.status.success() {
         panic!(
@@ -191,179 +50,25 @@ fn remote_pull_mirror_purges_extraneous_local_files() {
     );
 }
 
-fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn command");
-
-    match child.wait_timeout(timeout).expect("wait for process") {
-        Some(_status) => child
-            .wait_with_output()
-            .expect("collect command output after completion"),
-        None => {
-            let _ = child.kill();
-            let output = child
-                .wait_with_output()
-                .expect("collect output after killing command");
-            panic!(
-                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
-                timeout,
-                String::from_utf8_lossy(&output.stdout),
-                String::from_utf8_lossy(&output.stderr)
-            );
-        }
-    }
-}
-
-struct ChildGuard {
-    child: Option<std::process::Child>,
-}
-
-impl ChildGuard {
-    fn new(child: std::process::Child) -> Self {
-        Self { child: Some(child) }
-    }
-
-    fn terminate(&mut self) {
-        if let Some(mut child) = self.child.take() {
-            let _ = child.kill();
-            let _ = child.wait();
-        }
-    }
-}
-
-impl Drop for ChildGuard {
-    fn drop(&mut self) {
-        if let Some(mut child) = self.child.take() {
-            let _ = child.kill();
-            let _ = child.wait();
-        }
-    }
-}
-
-/// Spawn a daemon with the given module mapping and return the
-/// child guard plus the chosen port. Builds the daemon binary if
-/// necessary.
-fn spawn_daemon(workspace: &std::path::Path, module_dir: &std::path::Path) -> (ChildGuard, u16) {
-    let port = pick_unused_port();
-    let config = DaemonConfig {
-        daemon: DaemonSection {
-            bind: "127.0.0.1".into(),
-            port,
-            no_mdns: true,
-        },
-        modules: vec![ModuleSection {
-            name: "test".into(),
-            path: module_dir.to_path_buf(),
-            comment: None,
-            read_only: false,
-        }],
-    };
-    let config_path = workspace.join("blitd.toml");
-    let toml = toml::to_string(&config).expect("serialize config");
-    fs::write(&config_path, toml).expect("write config");
-
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir.parent().expect("deps parent").to_path_buf();
-    let daemon_bin = bin_dir.join(if cfg!(windows) {
-        "blit-daemon.exe"
-    } else {
-        "blit-daemon"
-    });
-    let maybe_target = bin_dir
-        .parent()
-        .and_then(|p| p.file_name())
-        .map(|c| c.to_string_lossy().to_string());
-    let mut build = Command::new("cargo");
-    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
-        .join("../..")
-        .canonicalize()
-        .expect("workspace root");
-    build.current_dir(workspace_root);
-    build
-        .arg("build")
-        .arg("-p")
-        .arg("blit-daemon")
-        .arg("--bin")
-        .arg("blit-daemon");
-    if let Some(triple) = maybe_target {
-        if triple != "target" {
-            build.arg("--target").arg(triple);
-        }
-    }
-    let output = build.output().expect("invoke cargo build for blit-daemon");
-    assert!(
-        output.status.success(),
-        "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
-        String::from_utf8_lossy(&output.stdout),
-        String::from_utf8_lossy(&output.stderr)
-    );
-
-    let child = Command::new(&daemon_bin)
-        .arg("--config")
-        .arg(&config_path)
-        .arg("--bind")
-        .arg("127.0.0.1")
-        .arg("--port")
-        .arg(port.to_string())
-        .stdin(Stdio::null())
-        .stdout(Stdio::null())
-        .stderr(Stdio::null())
-        .spawn()
-        .expect("spawn daemon");
-    let guard = ChildGuard::new(child);
-
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "daemon failed to listen on {port}");
-    (guard, port)
-}
-
-fn cli_bin() -> PathBuf {
-    let exe = std::env::current_exe().expect("current_exe");
-    let bin_dir = exe
-        .parent()
-        .expect("deps")
-        .parent()
-        .expect("target")
-        .to_path_buf();
-    bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" })
-}
-
 #[test]
 fn remote_pull_mirror_filtered_subset_preserves_out_of_scope_files() {
     // Closes F4: the daemon's filtered source manifest doesn't
     // include `*.tmp` files, but the client's dest has one. Under
     // FilteredSubset (default), out-of-scope files survive — only
     // in-scope client files absent from the source set are purged.
-    let work = tempdir().expect("tempdir");
-    let workspace = work.path();
+    let mut ctx = TestContext::new();
+    fs::write(ctx.module_dir.join("keep.txt"), b"from-server").unwrap();
+    fs::write(ctx.module_dir.join("server.tmp"), b"server tmp").unwrap();
 
-    let module_dir = workspace.join("module");
-    fs::create_dir_all(&module_dir).unwrap();
-    fs::write(module_dir.join("keep.txt"), b"from-server").unwrap();
-    fs::write(module_dir.join("server.tmp"), b"server tmp").unwrap();
-
-    let dest_dir = workspace.join("dest");
+    let dest_dir = ctx.workspace.join("dest");
     fs::create_dir_all(&dest_dir).unwrap();
     fs::write(dest_dir.join("dont-touch.tmp"), b"local tmp").unwrap();
     fs::write(dest_dir.join("extra.txt"), b"local extra").unwrap();
 
-    let config_dir = workspace.join("cli-config");
-    fs::create_dir_all(&config_dir).unwrap();
-
-    let (mut daemon, port) = spawn_daemon(workspace, &module_dir);
-
-    let remote_src = format!("127.0.0.1:{}:/test/", port);
-    let mut cmd = Command::new(cli_bin());
+    let remote_src = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
+    let mut cmd = Command::new(&ctx.cli_bin);
     cmd.arg("--config-dir")
-        .arg(&config_dir)
+        .arg(&ctx.config_dir)
         .arg("mirror")
         .arg("--yes")
         .arg("--exclude")
@@ -371,7 +76,7 @@ fn remote_pull_mirror_filtered_subset_preserves_out_of_scope_files() {
         .arg(&remote_src)
         .arg(&dest_dir);
     let output = run_with_timeout(cmd, Duration::from_secs(120));
-    daemon.terminate();
+    ctx.daemon.terminate();
     if !output.status.success() {
         panic!(
             "mirror failed\nstdout:\n{}\nstderr:\n{}",
@@ -402,27 +107,18 @@ fn remote_pull_mirror_delete_scope_all_purges_out_of_scope() {
     // `--delete-scope all` switches to MirrorMode::All — every dest
     // file absent from the (filtered) source set is deleted, even
     // those outside the filter scope. Sharp tool, opt-in.
-    let work = tempdir().expect("tempdir");
-    let workspace = work.path();
+    let mut ctx = TestContext::new();
+    fs::write(ctx.module_dir.join("keep.txt"), b"from-server").unwrap();
 
-    let module_dir = workspace.join("module");
-    fs::create_dir_all(&module_dir).unwrap();
-    fs::write(module_dir.join("keep.txt"), b"from-server").unwrap();
-
-    let dest_dir = workspace.join("dest");
+    let dest_dir = ctx.workspace.join("dest");
     fs::create_dir_all(&dest_dir).unwrap();
     fs::write(dest_dir.join("dont-touch.tmp"), b"local tmp").unwrap();
     fs::write(dest_dir.join("extra.txt"), b"local extra").unwrap();
 
-    let config_dir = workspace.join("cli-config");
-    fs::create_dir_all(&config_dir).unwrap();
-
-    let (mut daemon, port) = spawn_daemon(workspace, &module_dir);
-
-    let remote_src = format!("127.0.0.1:{}:/test/", port);
-    let mut cmd = Command::new(cli_bin());
+    let remote_src = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
+    let mut cmd = Command::new(&ctx.cli_bin);
     cmd.arg("--config-dir")
-        .arg(&config_dir)
+        .arg(&ctx.config_dir)
         .arg("mirror")
         .arg("--yes")
         .arg("--exclude")
@@ -432,7 +128,7 @@ fn remote_pull_mirror_delete_scope_all_purges_out_of_scope() {
         .arg(&remote_src)
         .arg(&dest_dir);
     let output = run_with_timeout(cmd, Duration::from_secs(120));
-    daemon.terminate();
+    ctx.daemon.terminate();
     if !output.status.success() {
         panic!(
             "mirror failed\nstdout:\n{}\nstderr:\n{}",

 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/remote_remote.rs b/crates/blit-cli/tests/remote_remote.rs
index 7bddb44..1028130 100644
--- a/crates/blit-cli/tests/remote_remote.rs
+++ b/crates/blit-cli/tests/remote_remote.rs
@@ -1,63 +1,22 @@
 use std::fs;
-use std::net::{TcpListener, TcpStream};
 use std::path::{Path, PathBuf};
-use std::process::{Command, Stdio};
-use std::thread;
+use std::process::Command;
 use std::time::Duration;
 
-use serde::Serialize;
 use tempfile::tempdir;
-use wait_timeout::ChildExt;
-
-#[derive(Serialize)]
-struct DaemonConfig {
-    daemon: DaemonSection,
-    #[serde(rename = "module")]
-    modules: Vec<ModuleSection>,
-    #[serde(skip_serializing_if = "Option::is_none")]
-    delegation: Option<DelegationSection>,
-}
-
-#[derive(Serialize)]
-struct DaemonSection {
-    bind: String,
-    port: u16,
-    no_mdns: bool,
-}
-
-#[derive(Serialize)]
-struct ModuleSection {
-    name: String,
-    path: PathBuf,
-    #[serde(default)]
-    comment: Option<String>,
-    #[serde(default)]
-    read_only: bool,
-    delegation_allowed: bool,
-}
-
-#[derive(Serialize)]
-struct DelegationSection {
-    allow_delegated_pull: bool,
-    allowed_source_hosts: Vec<String>,
-}
 
-fn pick_unused_port() -> u16 {
-    TcpListener::bind(("127.0.0.1", 0))
-        .expect("bind probe listener")
-        .local_addr()
-        .expect("listener addr")
-        .port()
-}
+mod common;
+use common::{run_with_timeout, spawn_fake_blit_server, DaemonOptions, SpawnedDaemon, TestContext};
 
-#[allow(dead_code)]
+/// Dual real daemons in one workspace: daemon A (the context's own
+/// daemon) is the plain source; daemon B is the destination whose
+/// delegation gate is under test. Harness lives in `common` (w9-3).
 struct DualDaemonContext {
-    _work: tempfile::TempDir,
+    _ctx: TestContext,
+    _daemon_b: SpawnedDaemon,
     workspace: PathBuf,
     daemon_a_port: u16,
     daemon_b_port: u16,
-    _daemon_a: ChildGuard,
-    _daemon_b: ChildGuard,
     cli_bin: PathBuf,
     config_dir: PathBuf,
     module_a_dir: PathBuf,
@@ -66,106 +25,28 @@ struct DualDaemonContext {
 
 impl DualDaemonContext {
     fn new(dest_delegation: bool) -> Self {
-        let work = tempdir().expect("tempdir");
-        let workspace = work.path().to_path_buf();
-
-        let module_a_dir = workspace.join("module_a");
-        fs::create_dir_all(&module_a_dir).expect("module a dir");
-
-        let module_b_dir = workspace.join("module_b");
-        fs::create_dir_all(&module_b_dir).expect("module b dir");
-
-        let config_dir = workspace.join("cli-config");
-        fs::create_dir_all(&config_dir).expect("cli config");
-
-        let port_a = pick_unused_port();
-        let port_b = pick_unused_port();
-        assert_ne!(port_a, port_b, "ports must be different");
-
-        let (cli_bin, daemon_bin) = binary_paths();
-        build_daemon();
-
-        let daemon_a = Self::spawn_daemon(
-            &workspace,
-            &daemon_bin,
-            port_a,
-            "daemon_a",
-            &module_a_dir,
-            false,
-        );
-        let daemon_b = Self::spawn_daemon(
-            &workspace,
-            &daemon_bin,
-            port_b,
+        let ctx = TestContext::new();
+        let daemon_b = ctx.spawn_second_daemon(
             "daemon_b",
-            &module_b_dir,
-            dest_delegation,
+            &DaemonOptions {
+                delegation: dest_delegation,
+                ..Default::default()
+            },
         );
 
         Self {
-            _work: work,
-            workspace,
-            daemon_a_port: port_a,
-            daemon_b_port: port_b,
-            _daemon_a: daemon_a,
+            workspace: ctx.workspace.clone(),
+            daemon_a_port: ctx.daemon_port,
+            daemon_b_port: daemon_b.port,
+            cli_bin: ctx.cli_bin.clone(),
+            config_dir: ctx.config_dir.clone(),
+            module_a_dir: ctx.module_dir.clone(),
+            module_b_dir: daemon_b.module_dir.clone(),
+            _ctx: ctx,
             _daemon_b: daemon_b,
-            cli_bin,
-            config_dir,
-            module_a_dir,
-            module_b_dir,
         }
     }
 
-    fn spawn_daemon(
-        workspace: &Path,
-        bin: &Path,
-        port: u16,
-        name: &str,
-        module_path: &Path,
-        delegation_enabled: bool,
-    ) -> ChildGuard {
-        let config = DaemonConfig {
-            daemon: DaemonSection {
-                bind: "127.0.0.1".into(),
-                port,
-                no_mdns: true,
-            },
-            modules: vec![ModuleSection {
-                name: "test".into(),
-                path: module_path.to_path_buf(),
-                comment: None,
-                read_only: false,
-                delegation_allowed: true,
-            }],
-            delegation: delegation_enabled.then(|| DelegationSection {
-                allow_delegated_pull: true,
-                // Loopback sources must be authorized by IP/CIDR form, not
-                // hostname form. This mirrors the production SSRF rule.
-                allowed_source_hosts: vec!["127.0.0.1".to_string()],
-            }),
-        };
-
-        let config_path = workspace.join(format!("{}.toml", name));
-        let toml = toml::to_string(&config).expect("serialize config");
-        fs::write(&config_path, toml).expect("write config");
-
-        let child = Command::new(bin)
-            .arg("--config")
-            .arg(&config_path)
-            .arg("--bind")
-            .arg("127.0.0.1")
-            .arg("--port")
-            .arg(port.to_string())
-            .stdin(Stdio::null())
-            .stdout(Stdio::null())
-            .stderr(Stdio::piped())
-            .spawn()
-            .expect("spawn daemon");
-
-        wait_for_port(port, &format!("daemon {name}"));
-        ChildGuard::new(child)
-    }
-
     fn source_remote(&self) -> String {
         format!("127.0.0.1:{}:/test/", self.daemon_a_port)
     }
@@ -320,8 +201,8 @@ fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
     let work = tempdir().expect("tempdir");
     let config_dir = work.path().join("cli-config");
     fs::create_dir_all(&config_dir).expect("cli config");
-    let (cli_bin, _daemon_bin) = binary_paths();
-    let stale = spawn_unimplemented_blit_server();
+    let cli_bin = common::cli_bin();
+    let stale = spawn_fake_blit_server(UnimplementedBlit, "fake unimplemented destination");
     let counter = work.path().join("stale.counter");
 
     let src_remote = "127.0.0.1:9:/test/";
@@ -354,7 +235,7 @@ fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
 #[test]
 fn source_refuses_destination_negotiation_does_not_fall_back_to_relay() {
     let ctx = DualDaemonContext::new(true);
-    let rejecting_source = spawn_rejecting_pull_sync_server();
+    let rejecting_source = spawn_fake_blit_server(RejectingPullSyncBlit, "fake rejecting source");
     let counter = ctx.counter_path("source_refuses");
     let src_remote = format!("127.0.0.1:{}:/test/", rejecting_source.port);
 
@@ -416,79 +297,6 @@ fn assert_success(output: &std::process::Output) {
     }
 }
 
-fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn command");
-
-    match child.wait_timeout(timeout).expect("wait for process") {
-        Some(_status) => child
-            .wait_with_output()
-            .expect("collect command output after completion"),
-        None => {
-            let _ = child.kill();
-            let output = child
-                .wait_with_output()
-                .expect("collect output after killing command");
-            panic!(
-                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
-                timeout,
-                String::from_utf8_lossy(&output.stdout),
-                String::from_utf8_lossy(&output.stderr)
-            );
-        }
-    }
-}
-
-fn binary_paths() -> (PathBuf, PathBuf) {
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
-    let daemon_bin = bin_dir.join(if cfg!(windows) {
-        "blit-daemon.exe"
-    } else {
-        "blit-daemon"
-    });
-    (cli_bin, daemon_bin)
-}
-
-fn build_daemon() {
-    let mut build = Command::new("cargo");
-    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
-        .join("../..")
-        .canonicalize()
-        .expect("workspace root");
-    build.current_dir(workspace_root);
-    build
-        .arg("build")
-        .arg("-p")
-        .arg("blit-daemon")
-        .arg("--bin")
-        .arg("blit-daemon");
-    let output = build.output().expect("invoke cargo build for blit-daemon");
-    assert!(
-        output.status.success(),
-        "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
-        String::from_utf8_lossy(&output.stdout),
-        String::from_utf8_lossy(&output.stderr)
-    );
-}
-
-fn wait_for_port(port: u16, label: &str) {
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "{label} failed to listen on {port}");
-}
-
 #[derive(Debug, Default)]
 struct CounterValues {
     cli_data_plane_outbound_bytes: u64,
@@ -522,118 +330,6 @@ fn read_counters(path: &Path) -> CounterValues {
     out
 }
 
-struct ChildGuard {
-    child: Option<std::process::Child>,
-}
-
-impl ChildGuard {
-    fn new(child: std::process::Child) -> Self {
-        Self { child: Some(child) }
-    }
-}
-
-impl Drop for ChildGuard {
-    fn drop(&mut self) {
-        if let Some(mut child) = self.child.take() {
-            let _ = child.kill();
-            let _ = child.wait();
-        }
-    }
-}
-
-struct UnimplementedServerGuard {
-    port: u16,
-    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
-    join: Option<thread::JoinHandle<()>>,
-}
-
-impl Drop for UnimplementedServerGuard {
-    fn drop(&mut self) {
-        if let Some(tx) = self.shutdown.take() {
-            let _ = tx.send(());
-        }
-        if let Some(join) = self.join.take() {
-            let _ = join.join();
-        }
-    }
-}
-
-fn spawn_unimplemented_blit_server() -> UnimplementedServerGuard {
-    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake server");
-    let port = listener.local_addr().expect("fake addr").port();
-    listener
-        .set_nonblocking(true)
-        .expect("set fake server nonblocking");
-    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
-
-    let join = thread::spawn(move || {
-        let runtime = tokio::runtime::Builder::new_current_thread()
-            .enable_all()
-            .build()
-            .expect("fake server runtime");
-        runtime.block_on(async move {
-            use blit_core::generated::blit_server::BlitServer;
-            use tokio_stream::wrappers::TcpListenerStream;
-            use tonic::transport::Server;
-
-            let listener =
-                tokio::net::TcpListener::from_std(listener).expect("tokio fake listener");
-            Server::builder()
-                .add_service(BlitServer::new(UnimplementedBlit))
-                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
-                    let _ = shutdown_rx.await;
-                })
-                .await
-                .expect("fake server");
-        });
-    });
-
-    wait_for_port(port, "fake unimplemented destination");
-    UnimplementedServerGuard {
-        port,
-        shutdown: Some(shutdown_tx),
-        join: Some(join),
-    }
-}
-
-fn spawn_rejecting_pull_sync_server() -> UnimplementedServerGuard {
-    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake source");
-    let port = listener.local_addr().expect("fake source addr").port();
-    listener
-        .set_nonblocking(true)
-        .expect("set fake source nonblocking");
-    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
-
-    let join = thread::spawn(move || {
-        let runtime = tokio::runtime::Builder::new_current_thread()
-            .enable_all()
-            .build()
-            .expect("fake source runtime");
-        runtime.block_on(async move {
-            use blit_core::generated::blit_server::BlitServer;
-            use tokio_stream::wrappers::TcpListenerStream;
-            use tonic::transport::Server;
-
-            let listener =
-                tokio::net::TcpListener::from_std(listener).expect("tokio fake source listener");
-            Server::builder()
-                .add_service(BlitServer::new(RejectingPullSyncBlit))
-                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
-                    let _ = shutdown_rx.await;
-                })
-                .await
-                .expect("fake source server");
-        });
-    });
-
-    wait_for_port(port, "fake rejecting source");
-    UnimplementedServerGuard {
-        port,
-        shutdown: Some(shutdown_tx),
-        join: Some(join),
-    }
-}
-
 struct UnimplementedBlit;
 
 #[tonic::async_trait]

exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-cli/tests/remote_checksum_negotiation.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/remote_checksum_negotiation.rs b/crates/blit-cli/tests/remote_checksum_negotiation.rs
index bae8fc2..d3dfc82 100644
--- a/crates/blit-cli/tests/remote_checksum_negotiation.rs
+++ b/crates/blit-cli/tests/remote_checksum_negotiation.rs
@@ -4,204 +4,24 @@
 //! `blit copy server:/test/file.txt dest --checksum`. The pull-sync
 //! handshake should bail with an ack-mismatch error rather than
 //! silently degrading to size+mtime.
+//!
+//! The daemon build/spawn runs through `common` (w9-3); the
+//! once-per-binary build keeps R16-F1's no-cross-test-ordering
+//! property (see `common::ensure_daemon_built`).
 
 use std::fs;
-use std::net::{TcpListener, TcpStream};
-use std::path::PathBuf;
-use std::process::{Command, Stdio};
-use std::thread;
+use std::process::Command;
 use std::time::Duration;
 
-use serde::Serialize;
-use tempfile::tempdir;
-use wait_timeout::ChildExt;
-
-#[derive(Serialize)]
-struct DaemonConfig {
-    daemon: DaemonSection,
-    #[serde(rename = "module")]
-    modules: Vec<ModuleSection>,
-}
-
-#[derive(Serialize)]
-struct DaemonSection {
-    bind: String,
-    port: u16,
-    no_mdns: bool,
-}
-
-#[derive(Serialize)]
-struct ModuleSection {
-    name: String,
-    path: PathBuf,
-    #[serde(default)]
-    comment: Option<String>,
-    #[serde(default)]
-    read_only: bool,
-}
-
-fn pick_unused_port() -> u16 {
-    TcpListener::bind(("127.0.0.1", 0))
-        .expect("bind probe")
-        .local_addr()
-        .expect("listener addr")
-        .port()
-}
-
-fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn command");
-    match child.wait_timeout(timeout).expect("wait_timeout") {
-        Some(_) => child.wait_with_output().expect("collect output"),
-        None => {
-            let _ = child.kill();
-            let output = child.wait_with_output().expect("collect output");
-            panic!(
-                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
-                timeout,
-                String::from_utf8_lossy(&output.stdout),
-                String::from_utf8_lossy(&output.stderr)
-            );
-        }
-    }
-}
-
-struct ChildGuard(Option<std::process::Child>);
-impl Drop for ChildGuard {
-    fn drop(&mut self) {
-        if let Some(mut c) = self.0.take() {
-            let _ = c.kill();
-            let _ = c.wait();
-        }
-    }
-}
-
-/// Workspace + binaries discovered for one test.
-struct DaemonHarness {
-    workspace: PathBuf,
-    module_dir: PathBuf,
-    config_dir: PathBuf,
-    cli_bin: PathBuf,
-    port: u16,
-    daemon: ChildGuard,
-}
+mod common;
+use common::{run_with_timeout, TestContext};
 
-/// Build the daemon binary, write a single-module config, spawn the
-/// daemon, and wait for it to listen. Both checksum tests use this
-/// so neither test depends on the other's build step (R16-F1 of
-/// `docs/reviews/followup_review_2026-05-02.md`).
-///
-/// `extra_daemon_args` lets the caller toggle daemon-side knobs
+/// Daemon with caller-chosen daemon-side knobs
 /// (e.g. `["--no-server-checksums"]`).
-fn spawn_daemon_harness(work: &tempfile::TempDir, extra_daemon_args: &[&str]) -> DaemonHarness {
-    let workspace = work.path().to_path_buf();
-
-    let module_dir = workspace.join("module");
-    fs::create_dir_all(&module_dir).expect("module dir");
-
-    let config_dir = workspace.join("cli-config");
-    fs::create_dir_all(&config_dir).expect("cli config");
-
-    let port = pick_unused_port();
-    let config = DaemonConfig {
-        daemon: DaemonSection {
-            bind: "127.0.0.1".into(),
-            port,
-            no_mdns: true,
-        },
-        modules: vec![ModuleSection {
-            name: "test".into(),
-            path: module_dir.clone(),
-            comment: None,
-            read_only: false,
-        }],
-    };
-    let config_path = workspace.join("blitd.toml");
-    fs::write(&config_path, toml::to_string(&config).unwrap()).expect("write config");
-
-    // Locate binaries relative to the test executable.
-    let exe = std::env::current_exe().expect("current_exe");
-    let bin_dir = exe
-        .parent()
-        .expect("test bin dir")
-        .parent()
-        .expect("deps parent")
-        .to_path_buf();
-    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
-    let daemon_bin = bin_dir.join(if cfg!(windows) {
-        "blit-daemon.exe"
-    } else {
-        "blit-daemon"
-    });
-    let maybe_target = bin_dir
-        .parent()
-        .and_then(|p| p.file_name())
-        .map(|c| c.to_string_lossy().to_string());
-
-    // Cargo build — every test calls this so neither depends on
-    // ordering for the daemon binary to exist.
-    let mut build = Command::new("cargo");
-    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
-        .join("../..")
-        .canonicalize()
-        .expect("workspace root");
-    build.current_dir(&workspace_root);
-    build
-        .arg("build")
-        .arg("-p")
-        .arg("blit-daemon")
-        .arg("--bin")
-        .arg("blit-daemon");
-    if let Some(triple) = maybe_target {
-        if triple != "target" {
-            build.arg("--target").arg(triple);
-        }
-    }
-    let output = build.output().expect("invoke cargo build");
-    assert!(
-        output.status.success(),
-        "cargo build blit-daemon failed:\n{}",
-        String::from_utf8_lossy(&output.stderr)
-    );
-
-    // Spawn the daemon.
-    let mut spawn = Command::new(&daemon_bin);
-    spawn
-        .arg("--config")
-        .arg(&config_path)
-        .arg("--bind")
-        .arg("127.0.0.1")
-        .arg("--port")
-        .arg(port.to_string());
-    for arg in extra_daemon_args {
-        spawn.arg(arg);
-    }
-    let daemon_child = spawn
-        .stdin(Stdio::null())
-        .stdout(Stdio::null())
-        .stderr(Stdio::null())
-        .spawn()
-        .expect("spawn daemon");
-    let daemon = ChildGuard(Some(daemon_child));
-
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "daemon failed to listen on {port}");
-
-    DaemonHarness {
-        workspace,
-        module_dir,
-        config_dir,
-        cli_bin,
-        port,
-        daemon,
-    }
+fn spawn_daemon_harness(extra_daemon_args: &[&str]) -> TestContext {
+    TestContext::builder()
+        .extra_daemon_args(extra_daemon_args.iter().copied())
+        .build()
 }
 
 #[test]
@@ -209,14 +29,13 @@ fn pull_checksum_rejected_when_daemon_disables_checksums() {
     // R15-F1 regression. Daemon advertises checksums disabled
     // via `--no-server-checksums`; a pull with `--checksum` must
     // bail at the ack rather than silently using size+mtime.
-    let work = tempdir().expect("tempdir");
-    let h = spawn_daemon_harness(&work, &["--no-server-checksums"]);
+    let h = spawn_daemon_harness(&["--no-server-checksums"]);
     fs::write(h.module_dir.join("payload.txt"), b"hello").expect("payload");
 
     let dest_dir = h.workspace.join("dest");
     fs::create_dir_all(&dest_dir).expect("dest dir");
 
-    let remote_src = format!("127.0.0.1:{}:/test/payload.txt", h.port);
+    let remote_src = format!("127.0.0.1:{}:/test/payload.txt", h.daemon_port);
     let mut cli_cmd = Command::new(&h.cli_bin);
     cli_cmd
         .arg("--config-dir")
@@ -255,14 +74,13 @@ fn pull_checksum_succeeds_when_daemon_enables_checksums() {
     // pull should succeed and copy the file. Proves the
     // capability check doesn't false-positive when the daemon
     // does support checksums.
-    let work = tempdir().expect("tempdir");
-    let h = spawn_daemon_harness(&work, &[]);
+    let h = spawn_daemon_harness(&[]);
     fs::write(h.module_dir.join("payload.txt"), b"hello").expect("payload");
 
     let dest_dir = h.workspace.join("dest");
     fs::create_dir_all(&dest_dir).expect("dest dir");
 
-    let remote_src = format!("127.0.0.1:{}:/test/payload.txt", h.port);
+    let remote_src = format!("127.0.0.1:{}:/test/payload.txt", h.daemon_port);
     let mut cli_cmd = Command::new(&h.cli_bin);
     cli_cmd
         .arg("--config-dir")

exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-cli/tests/remote_tcp_fallback.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/remote_tcp_fallback.rs b/crates/blit-cli/tests/remote_tcp_fallback.rs
index 45f7f3f..0e1427f 100644
--- a/crates/blit-cli/tests/remote_tcp_fallback.rs
+++ b/crates/blit-cli/tests/remote_tcp_fallback.rs
@@ -1,167 +1,32 @@
 use std::fs;
-use std::net::{TcpListener, TcpStream};
-use std::path::PathBuf;
-use std::process::{Command, Stdio};
-use std::thread;
+use std::process::Command;
 use std::time::Duration;
-use tempfile::tempdir;
 
-use serde::Serialize;
-use wait_timeout::ChildExt;
+mod common;
+use common::{run_with_timeout, TestContext};
 
-#[derive(Serialize)]
-struct DaemonConfig {
-    daemon: DaemonSection,
-    #[serde(rename = "module")]
-    modules: Vec<ModuleSection>,
-}
-
-#[derive(Serialize)]
-struct DaemonSection {
-    bind: String,
-    port: u16,
-    no_mdns: bool,
-}
-
-#[derive(Serialize)]
-struct ModuleSection {
-    name: String,
-    path: PathBuf,
-    #[serde(default)]
-    comment: Option<String>,
-    #[serde(default)]
-    read_only: bool,
-}
-
-fn pick_unused_port() -> u16 {
-    TcpListener::bind(("127.0.0.1", 0))
-        .expect("bind probe listener")
-        .local_addr()
-        .expect("listener addr")
-        .port()
+/// Daemon forced into gRPC data fallback (`--force-grpc-data`).
+fn forced_grpc_ctx() -> TestContext {
+    TestContext::builder()
+        .extra_daemon_args(["--force-grpc-data"])
+        .build()
 }
 
 #[test]
 fn remote_push_falls_back_to_grpc_when_forced() {
-    let work = tempdir().expect("tempdir");
-    let workspace = work.path();
-
-    let module_dir = workspace.join("module");
-    fs::create_dir_all(&module_dir).expect("module dir");
+    let mut ctx = forced_grpc_ctx();
 
-    let src_dir = workspace.join("src");
+    let src_dir = ctx.workspace.join("src");
     fs::create_dir_all(&src_dir).expect("src dir");
     fs::write(src_dir.join("file.txt"), b"fallback-test").expect("write file");
 
-    let config_dir = workspace.join("cli-config");
-    fs::create_dir_all(&config_dir).expect("cli config");
-
-    let port = pick_unused_port();
-
-    let config = DaemonConfig {
-        daemon: DaemonSection {
-            bind: "127.0.0.1".into(),
-            port,
-            no_mdns: true,
-        },
-        modules: vec![ModuleSection {
-            name: "test".into(),
-            path: module_dir.clone(),
-            comment: None,
-            read_only: false,
-        }],
-    };
-
-    let config_path = workspace.join("blitd.toml");
-    let toml = toml::to_string(&config).expect("serialize config");
-    fs::write(&config_path, toml).expect("write config");
-
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-
-    let cli_bin = {
-        let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-        bin_dir.join(name)
-    };
-    let daemon_bin = {
-        let name = if cfg!(windows) {
-            "blit-daemon.exe"
-        } else {
-            "blit-daemon"
-        };
-        bin_dir.join(name)
-    };
-    let maybe_target = bin_dir
-        .parent()
-        .and_then(|p| p.file_name())
-        .map(|component| component.to_string_lossy().to_string());
-
-    let mut build = Command::new("cargo");
-    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
-        .join("../..")
-        .canonicalize()
-        .expect("workspace root");
-    build.current_dir(workspace_root);
-    build
-        .arg("build")
-        .arg("-p")
-        .arg("blit-daemon")
-        .arg("--bin")
-        .arg("blit-daemon");
-    if let Some(triple) = maybe_target {
-        if triple != "target" {
-            build.arg("--target").arg(triple);
-        }
-    }
-    let output = build.output().expect("invoke cargo build for blit-daemon");
-    assert!(
-        output.status.success(),
-        "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
-        String::from_utf8_lossy(&output.stdout),
-        String::from_utf8_lossy(&output.stderr)
-    );
-    assert!(
-        daemon_bin.exists(),
-        "expected daemon binary at {}",
-        daemon_bin.display()
-    );
-
-    let daemon_child = Command::new(&daemon_bin)
-        .arg("--config")
-        .arg(&config_path)
-        .arg("--force-grpc-data")
-        .arg("--bind")
-        .arg("127.0.0.1")
-        .arg("--port")
-        .arg(port.to_string())
-        .stdin(Stdio::null())
-        .stdout(Stdio::null())
-        .stderr(Stdio::null())
-        .spawn()
-        .expect("spawn daemon");
-    let mut daemon = ChildGuard::new(daemon_child);
-
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "daemon failed to listen on {port}");
-
-    let dest_remote = format!("127.0.0.1:{}:/test/", port);
+    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
     // Trailing slash on source: merge contents into module root.
     let src_arg = format!("{}/", src_dir.display());
-    let mut cli_cmd = Command::new(&cli_bin);
+    let mut cli_cmd = Command::new(&ctx.cli_bin);
     cli_cmd
         .arg("--config-dir")
-        .arg(&config_dir)
+        .arg(&ctx.config_dir)
         .arg("mirror")
         .arg("--yes")
         .arg("--force-grpc")
@@ -169,7 +34,7 @@ fn remote_push_falls_back_to_grpc_when_forced() {
         .arg(&dest_remote);
     let output = run_with_timeout(cli_cmd, Duration::from_secs(120));
 
-    daemon.terminate();
+    ctx.daemon.terminate();
 
     if !output.status.success() {
         panic!(
@@ -186,61 +51,12 @@ fn remote_push_falls_back_to_grpc_when_forced() {
         stdout
     );
 
-    let dest_file = module_dir.join("file.txt");
+    let dest_file = ctx.module_dir.join("file.txt");
     assert!(dest_file.exists(), "remote file missing");
     let bytes = fs::read(&dest_file).expect("read remote file");
     assert_eq!(bytes, b"fallback-test");
 }
 
-fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn command");
-
-    match child.wait_timeout(timeout).expect("wait for process") {
-        Some(_status) => child
-            .wait_with_output()
-            .expect("collect command output after completion"),
-        None => {
-            let _ = child.kill();
-            let output = child
-                .wait_with_output()
-                .expect("collect output after killing command");
-            panic!(
-                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
-                timeout,
-                String::from_utf8_lossy(&output.stdout),
-                String::from_utf8_lossy(&output.stderr)
-            );
-        }
-    }
-}
-
-struct ChildGuard {
-    child: Option<std::process::Child>,
-}
-
-impl ChildGuard {
-    fn new(child: std::process::Child) -> Self {
-        Self { child: Some(child) }
-    }
-
-    fn terminate(&mut self) {
-        if let Some(mut child) = self.child.take() {
-            let _ = child.kill();
-            let _ = child.wait();
-        }
-    }
-}
-
-impl Drop for ChildGuard {
-    fn drop(&mut self) {
-        if let Some(mut child) = self.child.take() {
-            let _ = child.kill();
-            let _ = child.wait();
-        }
-    }
-}
-
 // ---------------------------------------------------------------
 // w4-2 regression net: the daemon used to queue every needs-upload
 // manifest entry into a 262,144-slot channel that nothing read in
@@ -253,12 +69,9 @@ impl Drop for ChildGuard {
 /// Spawn a daemon, mirror `file_count` generated files with
 /// --force-grpc, assert success, and return how many landed.
 fn forced_grpc_mirror_file_count(file_count: usize, timeout: Duration) -> usize {
-    let work = tempdir().expect("tempdir");
-    let workspace = work.path();
+    let mut ctx = forced_grpc_ctx();
 
-    let module_dir = workspace.join("module");
-    fs::create_dir_all(&module_dir).expect("module dir");
-    let src_dir = workspace.join("src");
+    let src_dir = ctx.workspace.join("src");
     fs::create_dir_all(&src_dir).expect("src dir");
     for idx in 0..file_count {
         // Shard into subdirs so no single directory holds 262k entries.
@@ -269,82 +82,19 @@ fn forced_grpc_mirror_file_count(file_count: usize, timeout: Duration) -> usize
         fs::write(sub.join(format!("f{idx}.txt")), b"x").expect("write src file");
     }
 
-    let config_dir = workspace.join("cli-config");
-    fs::create_dir_all(&config_dir).expect("cli config");
-
-    let port = pick_unused_port();
-    let config = DaemonConfig {
-        daemon: DaemonSection {
-            bind: "127.0.0.1".into(),
-            port,
-            no_mdns: true,
-        },
-        modules: vec![ModuleSection {
-            name: "test".into(),
-            path: module_dir.clone(),
-            comment: None,
-            read_only: false,
-        }],
-    };
-    let config_path = workspace.join("blitd.toml");
-    fs::write(
-        &config_path,
-        toml::to_string(&config).expect("serialize config"),
-    )
-    .expect("write config");
-
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let bin_dir = exe_path
-        .parent()
-        .expect("deps dir")
-        .parent()
-        .expect("bin dir")
-        .to_path_buf();
-    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
-    let daemon_bin = bin_dir.join(if cfg!(windows) {
-        "blit-daemon.exe"
-    } else {
-        "blit-daemon"
-    });
-
-    let daemon_child = Command::new(&daemon_bin)
-        .arg("--config")
-        .arg(&config_path)
-        .arg("--force-grpc-data")
-        .arg("--bind")
-        .arg("127.0.0.1")
-        .arg("--port")
-        .arg(port.to_string())
-        .stdin(Stdio::null())
-        .stdout(Stdio::null())
-        .stderr(Stdio::null())
-        .spawn()
-        .expect("spawn daemon");
-    let mut daemon = ChildGuard::new(daemon_child);
-
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "daemon failed to listen on {port}");
-
-    let dest_remote = format!("127.0.0.1:{port}:/test/");
+    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
     let src_arg = format!("{}/", src_dir.display());
-    let mut cli_cmd = Command::new(&cli_bin);
+    let mut cli_cmd = Command::new(&ctx.cli_bin);
     cli_cmd
         .arg("--config-dir")
-        .arg(&config_dir)
+        .arg(&ctx.config_dir)
         .arg("mirror")
         .arg("--yes")
         .arg("--force-grpc")
         .arg(&src_arg)
         .arg(&dest_remote);
     let output = run_with_timeout(cli_cmd, timeout);
-    daemon.terminate();
+    ctx.daemon.terminate();
 
     assert!(
         output.status.success(),
@@ -353,7 +103,7 @@ fn forced_grpc_mirror_file_count(file_count: usize, timeout: Duration) -> usize
         String::from_utf8_lossy(&output.stderr)
     );
 
-    walkdir_count_files(&module_dir)
+    walkdir_count_files(&ctx.module_dir)
 }
 
 fn walkdir_count_files(root: &std::path::Path) -> usize {

codex
So far the daemon flags and remote URL rewrites in the larger remote tests look mechanically equivalent. I’m checking the remaining call sites plus the fake server migration for any semantic drift.
exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-cli/tests/jobs_lifecycle.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/jobs_lifecycle.rs b/crates/blit-cli/tests/jobs_lifecycle.rs
index 5a15bc8..44beb2c 100644
--- a/crates/blit-cli/tests/jobs_lifecycle.rs
+++ b/crates/blit-cli/tests/jobs_lifecycle.rs
@@ -13,21 +13,16 @@
 //! Watch exit codes (see `run_jobs_watch`): 0 finished-ok,
 //! 1 finished-failed, 2 not-found, 3 timeout-while-active.
 //!
-//! The dual-daemon delegation harness mirrors `remote_remote.rs`
-//! (consolidation of the harness clones is w9-3's job).
+//! The dual-daemon delegation harness builds on the shared `common`
+//! spawn primitives (consolidated by w9-3).
 
 use std::fs;
-use std::net::TcpStream;
-use std::path::{Path, PathBuf};
-use std::process::{Command, Stdio};
-use std::thread;
+use std::path::PathBuf;
+use std::process::Command;
 use std::time::Duration;
 
-use serde::Serialize;
-use tempfile::tempdir;
-
 mod common;
-use common::{run_with_timeout, ChildGuard, TestContext};
+use common::{run_with_timeout, spawn_fake_blit_server, DaemonOptions, SpawnedDaemon, TestContext};
 
 // ---------------------------------------------------------------
 // Single-daemon cases: list shape, cancel/watch unknown-id codes.
@@ -110,125 +105,15 @@ fn jobs_watch_unknown_id_exits_two() {
 }
 
 // ---------------------------------------------------------------
-// Delegation harness (dual daemon / fake source), mirroring
-// remote_remote.rs.
+// Delegation harness (dual daemon / fake source) — built on the
+// shared `common` spawn primitives (w9-3).
 // ---------------------------------------------------------------
 
-#[derive(Serialize)]
-struct DaemonConfig {
-    daemon: DaemonSection,
-    #[serde(rename = "module")]
-    modules: Vec<ModuleSection>,
-    #[serde(skip_serializing_if = "Option::is_none")]
-    delegation: Option<DelegationSection>,
-}
-
-#[derive(Serialize)]
-struct DaemonSection {
-    bind: String,
-    port: u16,
-    no_mdns: bool,
-}
-
-#[derive(Serialize)]
-struct ModuleSection {
-    name: String,
-    path: PathBuf,
-    comment: Option<String>,
-    read_only: bool,
-    delegation_allowed: bool,
-}
-
-#[derive(Serialize)]
-struct DelegationSection {
-    allow_delegated_pull: bool,
-    allowed_source_hosts: Vec<String>,
-}
-
-fn binary_paths() -> (PathBuf, PathBuf) {
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
-    let daemon_bin = bin_dir.join(if cfg!(windows) {
-        "blit-daemon.exe"
-    } else {
-        "blit-daemon"
-    });
-    (cli_bin, daemon_bin)
-}
-
-fn wait_for_port(port: u16, label: &str) {
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "{label} failed to listen on {port}");
-}
-
-fn spawn_daemon(
-    workspace: &Path,
-    bin: &Path,
-    port: u16,
-    name: &str,
-    module_path: &Path,
-    delegation_enabled: bool,
-) -> ChildGuard {
-    let config = DaemonConfig {
-        daemon: DaemonSection {
-            bind: "127.0.0.1".into(),
-            port,
-            no_mdns: true,
-        },
-        modules: vec![ModuleSection {
-            name: "test".into(),
-            path: module_path.to_path_buf(),
-            comment: None,
-            read_only: false,
-            delegation_allowed: true,
-        }],
-        delegation: delegation_enabled.then(|| DelegationSection {
-            allow_delegated_pull: true,
-            // Loopback sources must be authorized by IP/CIDR form,
-            // mirroring the production SSRF rule (see remote_remote.rs).
-            allowed_source_hosts: vec!["127.0.0.1".to_string()],
-        }),
-    };
-
-    let config_path = workspace.join(format!("{name}.toml"));
-    let toml = toml::to_string(&config).expect("serialize config");
-    fs::write(&config_path, toml).expect("write config");
-
-    let child = Command::new(bin)
-        .arg("--config")
-        .arg(&config_path)
-        .arg("--bind")
-        .arg("127.0.0.1")
-        .arg("--port")
-        .arg(port.to_string())
-        .stdin(Stdio::null())
-        .stdout(Stdio::null())
-        .stderr(Stdio::piped())
-        .spawn()
-        .expect("spawn daemon");
-
-    wait_for_port(port, &format!("daemon {name}"));
-    ChildGuard::new(child)
-}
-
 struct DelegationContext {
-    _work: tempfile::TempDir,
+    _ctx: TestContext,
+    _second: Option<SpawnedDaemon>,
     src_port: u16,
     dst_port: u16,
-    _src_daemon: Option<ChildGuard>,
-    _dst_daemon: ChildGuard,
     cli_bin: PathBuf,
     config_dir: PathBuf,
     module_src_dir: Option<PathBuf>,
@@ -238,85 +123,44 @@ struct DelegationContext {
 impl DelegationContext {
     /// Real source daemon + delegation-enabled destination daemon.
     fn with_real_source() -> Self {
-        let work = tempdir().expect("tempdir");
-        let workspace = work.path().to_path_buf();
-
-        let module_src_dir = workspace.join("module_src");
-        fs::create_dir_all(&module_src_dir).expect("module src dir");
-        let module_dst_dir = workspace.join("module_dst");
-        fs::create_dir_all(&module_dst_dir).expect("module dst dir");
-        let config_dir = workspace.join("cli-config");
-        fs::create_dir_all(&config_dir).expect("cli config");
-
-        let (cli_bin, daemon_bin) = binary_paths();
-
-        let src_port = common::pick_unused_port();
-        let dst_port = common::pick_unused_port();
-        assert_ne!(src_port, dst_port, "ports must be different");
-
-        let src_daemon = spawn_daemon(
-            &workspace,
-            &daemon_bin,
-            src_port,
-            "daemon_src",
-            &module_src_dir,
-            false,
-        );
-        let dst_daemon = spawn_daemon(
-            &workspace,
-            &daemon_bin,
-            dst_port,
+        // The context's own daemon is the plain source; the
+        // destination (whose delegation gate + job table are under
+        // test) is the second daemon.
+        let ctx = TestContext::new();
+        let dst = ctx.spawn_second_daemon(
             "daemon_dst",
-            &module_dst_dir,
-            true,
+            &DaemonOptions {
+                delegation: true,
+                ..Default::default()
+            },
         );
 
         Self {
-            _work: work,
-            src_port,
-            dst_port,
-            _src_daemon: Some(src_daemon),
-            _dst_daemon: dst_daemon,
-            cli_bin,
-            config_dir,
-            module_src_dir: Some(module_src_dir),
-            module_dst_dir,
+            src_port: ctx.daemon_port,
+            dst_port: dst.port,
+            cli_bin: ctx.cli_bin.clone(),
+            config_dir: ctx.config_dir.clone(),
+            module_src_dir: Some(ctx.module_dir.clone()),
+            module_dst_dir: dst.module_dir.clone(),
+            _ctx: ctx,
+            _second: Some(dst),
         }
     }
 
     /// Fake stalling source + delegation-enabled destination daemon.
-    /// The fake's port is owned by the caller's `StallingSourceGuard`.
+    /// The fake's port is owned by the caller's `FakeServerGuard`.
     fn with_stalling_source(fake_port: u16) -> Self {
-        let work = tempdir().expect("tempdir");
-        let workspace = work.path().to_path_buf();
-
-        let module_dst_dir = workspace.join("module_dst");
-        fs::create_dir_all(&module_dst_dir).expect("module dst dir");
-        let config_dir = workspace.join("cli-config");
-        fs::create_dir_all(&config_dir).expect("cli config");
-
-        let (cli_bin, daemon_bin) = binary_paths();
-        let dst_port = common::pick_unused_port();
-
-        let dst_daemon = spawn_daemon(
-            &workspace,
-            &daemon_bin,
-            dst_port,
-            "daemon_dst",
-            &module_dst_dir,
-            true,
-        );
+        let ctx = TestContext::builder().delegation(true).build();
 
         Self {
-            _work: work,
             src_port: fake_port,
-            dst_port,
-            _src_daemon: None,
-            _dst_daemon: dst_daemon,
-            cli_bin,
-            config_dir,
+            dst_port: ctx.daemon_port,
+            cli_bin: ctx.cli_bin.clone(),
+            config_dir: ctx.config_dir.clone(),
             module_src_dir: None,
-            module_dst_dir,
+            module_dst_dir: ctx.module_dir.clone(),
+            _ctx: ctx,
+            _second: None,
         }
     }
 
@@ -461,63 +305,12 @@ fn cancel_of_active_delegated_job_exits_zero() {
 
 // ---------------------------------------------------------------
 // Fake stalling source: a tonic server whose pull_sync never
-// answers. Everything else is unimplemented (same shape as
-// remote_remote.rs's fake daemons).
+// answers. Everything else is unimplemented. Served through the
+// shared production-shaped scaffold (common::spawn_fake_blit_server).
 // ---------------------------------------------------------------
 
-struct StallingSourceGuard {
-    port: u16,
-    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
-    join: Option<thread::JoinHandle<()>>,
-}
-
-impl Drop for StallingSourceGuard {
-    fn drop(&mut self) {
-        if let Some(tx) = self.shutdown.take() {
-            let _ = tx.send(());
-        }
-        if let Some(join) = self.join.take() {
-            let _ = join.join();
-        }
-    }
-}
-
-fn spawn_stalling_source() -> StallingSourceGuard {
-    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake source");
-    let port = listener.local_addr().expect("fake source addr").port();
-    listener
-        .set_nonblocking(true)
-        .expect("set fake source nonblocking");
-    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
-
-    let join = thread::spawn(move || {
-        let runtime = tokio::runtime::Builder::new_current_thread()
-            .enable_all()
-            .build()
-            .expect("fake source runtime");
-        runtime.block_on(async move {
-            use blit_core::generated::blit_server::BlitServer;
-            use tokio_stream::wrappers::TcpListenerStream;
-            use tonic::transport::Server;
-
-            let listener =
-                tokio::net::TcpListener::from_std(listener).expect("tokio fake source listener");
-            Server::builder()
-                .add_service(BlitServer::new(StallingPullSyncBlit))
-                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
-                    let _ = shutdown_rx.await;
-                })
-                .await
-                .expect("fake source server");
-        });
-    });
-
-    wait_for_port(port, "fake stalling source");
-    StallingSourceGuard {
-        port,
-        shutdown: Some(shutdown_tx),
-        join: Some(join),
-    }
+fn spawn_stalling_source() -> common::FakeServerGuard {
+    spawn_fake_blit_server(StallingPullSyncBlit, "fake stalling source")
 }
 
 struct StallingPullSyncBlit;

exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-cli/tests/readonly_enforcement.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/readonly_enforcement.rs b/crates/blit-cli/tests/readonly_enforcement.rs
index 8299f9b..5166b63 100644
--- a/crates/blit-cli/tests/readonly_enforcement.rs
+++ b/crates/blit-cli/tests/readonly_enforcement.rs
@@ -8,21 +8,15 @@
 //! read-only module, so a dropped gate (mirror-deletion blast
 //! radius) would have passed the full validation suite.
 //!
-//! The dual-daemon mini-harness for the delegated case is another
-//! clone of the remote_remote.rs pattern; consolidation is w9-3.
+//! The dual-daemon delegated case runs on the shared `common` spawn
+//! primitives (consolidated by w9-3).
 
 use std::fs;
-use std::net::TcpStream;
-use std::path::{Path, PathBuf};
-use std::process::{Command, Stdio};
-use std::thread;
+use std::process::Command;
 use std::time::Duration;
 
-use serde::Serialize;
-use tempfile::tempdir;
-
 mod common;
-use common::{run_with_timeout, ChildGuard, TestContext};
+use common::{run_with_timeout, DaemonOptions, TestContext};
 
 fn assert_read_only_rejection(output: &std::process::Output, what: &str) {
     assert!(
@@ -136,157 +130,28 @@ fn purge_on_read_only_module_is_rejected_and_file_survives() {
 // honest about ordering.
 // ---------------------------------------------------------------
 
-#[derive(Serialize)]
-struct DaemonConfig {
-    daemon: DaemonSection,
-    #[serde(rename = "module")]
-    modules: Vec<ModuleSection>,
-    #[serde(skip_serializing_if = "Option::is_none")]
-    delegation: Option<DelegationSection>,
-}
-
-#[derive(Serialize)]
-struct DaemonSection {
-    bind: String,
-    port: u16,
-    no_mdns: bool,
-}
-
-#[derive(Serialize)]
-struct ModuleSection {
-    name: String,
-    path: PathBuf,
-    comment: Option<String>,
-    read_only: bool,
-    delegation_allowed: bool,
-}
-
-#[derive(Serialize)]
-struct DelegationSection {
-    allow_delegated_pull: bool,
-    allowed_source_hosts: Vec<String>,
-}
-
-fn wait_for_port(port: u16, label: &str) {
-    let mut ready = false;
-    for _ in 0..50 {
-        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-            ready = true;
-            break;
-        }
-        thread::sleep(Duration::from_millis(100));
-    }
-    assert!(ready, "{label} failed to listen on {port}");
-}
-
-#[allow(clippy::too_many_arguments)]
-fn spawn_daemon(
-    workspace: &Path,
-    bin: &Path,
-    port: u16,
-    name: &str,
-    module_path: &Path,
-    read_only: bool,
-    delegation_enabled: bool,
-) -> ChildGuard {
-    let config = DaemonConfig {
-        daemon: DaemonSection {
-            bind: "127.0.0.1".into(),
-            port,
-            no_mdns: true,
-        },
-        modules: vec![ModuleSection {
-            name: "test".into(),
-            path: module_path.to_path_buf(),
-            comment: None,
-            read_only,
-            delegation_allowed: true,
-        }],
-        delegation: delegation_enabled.then(|| DelegationSection {
-            allow_delegated_pull: true,
-            allowed_source_hosts: vec!["127.0.0.1".to_string()],
-        }),
-    };
-
-    let config_path = workspace.join(format!("{name}.toml"));
-    let toml = toml::to_string(&config).expect("serialize config");
-    fs::write(&config_path, toml).expect("write config");
-
-    let child = Command::new(bin)
-        .arg("--config")
-        .arg(&config_path)
-        .arg("--bind")
-        .arg("127.0.0.1")
-        .arg("--port")
-        .arg(port.to_string())
-        .stdin(Stdio::null())
-        .stdout(Stdio::null())
-        .stderr(Stdio::piped())
-        .spawn()
-        .expect("spawn daemon");
-
-    wait_for_port(port, &format!("daemon {name}"));
-    ChildGuard::new(child)
-}
-
 #[test]
 fn delegated_pull_to_read_only_destination_is_rejected() {
-    let work = tempdir().expect("tempdir");
-    let workspace = work.path().to_path_buf();
-
-    let module_src = workspace.join("module_src");
-    let module_dst = workspace.join("module_dst");
-    fs::create_dir_all(&module_src).expect("src module");
-    fs::create_dir_all(&module_dst).expect("dst module");
-    fs::write(module_src.join("payload.txt"), b"must-not-land").expect("seed src");
-
-    let config_dir = workspace.join("cli-config");
-    fs::create_dir_all(&config_dir).expect("cli config");
-
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let bin_dir = exe_path
-        .parent()
-        .expect("deps dir")
-        .parent()
-        .expect("bin dir")
-        .to_path_buf();
-    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
-    let daemon_bin = bin_dir.join(if cfg!(windows) {
-        "blit-daemon.exe"
-    } else {
-        "blit-daemon"
-    });
-
-    let src_port = common::pick_unused_port();
-    let dst_port = common::pick_unused_port();
-    assert_ne!(src_port, dst_port);
-
-    let _src = spawn_daemon(
-        &workspace,
-        &daemon_bin,
-        src_port,
-        "daemon_src",
-        &module_src,
-        false,
-        false,
-    );
-    // Destination: delegation enabled, module read-only — the
+    // The context's own daemon is the plain source; the destination
+    // has delegation enabled and a read-only module — the
     // delegated_pull read-only gate is what must fire.
-    let _dst = spawn_daemon(
-        &workspace,
-        &daemon_bin,
-        dst_port,
+    let ctx = TestContext::new();
+    fs::write(ctx.module_dir.join("payload.txt"), b"must-not-land").expect("seed src");
+
+    let dst = ctx.spawn_second_daemon(
         "daemon_dst",
-        &module_dst,
-        true,
-        true,
+        &DaemonOptions {
+            read_only: true,
+            delegation: true,
+            ..Default::default()
+        },
     );
 
-    let src_remote = format!("127.0.0.1:{src_port}:/test/");
-    let dst_remote = format!("127.0.0.1:{dst_port}:/test/");
-    let mut cmd = Command::new(&cli_bin);
+    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
+    let dst_remote = format!("127.0.0.1:{}:/test/", dst.port);
+    let mut cmd = Command::new(&ctx.cli_bin);
     cmd.arg("--config-dir")
-        .arg(&config_dir)
+        .arg(&ctx.config_dir)
         .arg("copy")
         .arg(&src_remote)
         .arg(&dst_remote);
@@ -294,7 +159,7 @@ fn delegated_pull_to_read_only_destination_is_rejected() {
 
     assert_read_only_rejection(&output, "delegated pull");
     assert!(
-        fs::read_dir(&module_dst)
+        fs::read_dir(&dst.module_dir)
             .expect("dst module readable")
             .next()
             .is_none(),

exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-cli/tests/cli_arg_safety_gates.rs crates/blit-cli/tests/diagnostics_dump.rs crates/blit-cli/tests/local_move_semantics.rs crates/blit-cli/tests/single_file_copy.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/cli_arg_safety_gates.rs b/crates/blit-cli/tests/cli_arg_safety_gates.rs
index 2dfce36..36acde5 100644
--- a/crates/blit-cli/tests/cli_arg_safety_gates.rs
+++ b/crates/blit-cli/tests/cli_arg_safety_gates.rs
@@ -12,41 +12,13 @@
 //!   - R54-F2: `--force` / `--ignore-times` rejected on move
 
 use std::fs;
-use std::path::PathBuf;
-use std::process::{Command, Stdio};
+use std::process::Command;
 use std::time::Duration;
 
 use tempfile::tempdir;
-use wait_timeout::ChildExt;
 
-fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn command");
-    match child.wait_timeout(timeout).expect("wait for process") {
-        Some(_) => child.wait_with_output().expect("collect output"),
-        None => {
-            let _ = child.kill();
-            let output = child.wait_with_output().expect("output after kill");
-            panic!(
-                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
-                timeout,
-                String::from_utf8_lossy(&output.stdout),
-                String::from_utf8_lossy(&output.stderr)
-            );
-        }
-    }
-}
-
-fn cli_bin() -> PathBuf {
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-    let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-    bin_dir.join(name)
-}
+mod common;
+use common::{cli_bin, run_with_timeout};
 
 fn assert_rejected(args: &[&str], expect_in_stderr: &str) {
     let mut cmd = Command::new(cli_bin());
diff --git a/crates/blit-cli/tests/diagnostics_dump.rs b/crates/blit-cli/tests/diagnostics_dump.rs
index f8048ac..7e55817 100644
--- a/crates/blit-cli/tests/diagnostics_dump.rs
+++ b/crates/blit-cli/tests/diagnostics_dump.rs
@@ -5,37 +5,18 @@
 //! load-bearing ones.
 
 use std::fs;
-use std::path::PathBuf;
-use std::process::{Command, Stdio};
+use std::process::Command;
 use std::time::Duration;
 
-use wait_timeout::ChildExt;
-
-fn cli_bin() -> PathBuf {
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-    let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-    bin_dir.join(name)
-}
+mod common;
+use common::{cli_bin, run_with_timeout};
 
 fn run_dump(args: &[&str]) -> std::process::Output {
     let bin = cli_bin();
     let mut cmd = Command::new(&bin);
     cmd.arg("diagnostics").arg("dump");
     cmd.args(args);
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn");
-    match child
-        .wait_timeout(Duration::from_secs(10))
-        .expect("wait_timeout")
-    {
-        Some(_) => child.wait_with_output().expect("wait_with_output"),
-        None => panic!("diagnostics dump timed out"),
-    }
+    run_with_timeout(cmd, Duration::from_secs(10))
 }
 
 #[test]
diff --git a/crates/blit-cli/tests/local_move_semantics.rs b/crates/blit-cli/tests/local_move_semantics.rs
index b1094b6..ff69fd8 100644
--- a/crates/blit-cli/tests/local_move_semantics.rs
+++ b/crates/blit-cli/tests/local_move_semantics.rs
@@ -10,41 +10,13 @@
 //! data-loss bug.
 
 use std::fs;
-use std::path::PathBuf;
-use std::process::{Command, Stdio};
+use std::process::Command;
 use std::time::Duration;
 
 use tempfile::tempdir;
-use wait_timeout::ChildExt;
 
-fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn command");
-    match child.wait_timeout(timeout).expect("wait for process") {
-        Some(_) => child.wait_with_output().expect("collect output"),
-        None => {
-            let _ = child.kill();
-            let output = child.wait_with_output().expect("output after kill");
-            panic!(
-                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
-                timeout,
-                String::from_utf8_lossy(&output.stdout),
-                String::from_utf8_lossy(&output.stderr)
-            );
-        }
-    }
-}
-
-fn cli_bin() -> PathBuf {
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-    let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-    bin_dir.join(name)
-}
+mod common;
+use common::{cli_bin, run_with_timeout};
 
 #[test]
 fn local_move_preserves_unrelated_destination_entries() {
diff --git a/crates/blit-cli/tests/single_file_copy.rs b/crates/blit-cli/tests/single_file_copy.rs
index 90ae845..524509b 100644
--- a/crates/blit-cli/tests/single_file_copy.rs
+++ b/crates/blit-cli/tests/single_file_copy.rs
@@ -5,40 +5,11 @@
 //! report "0 files" with success.
 
 use std::fs;
-use std::path::PathBuf;
-use std::process::{Command, Stdio};
+use std::process::Command;
 use std::time::Duration;
 
-use wait_timeout::ChildExt;
-
-fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
-    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
-    let mut child = cmd.spawn().expect("spawn command");
-    match child.wait_timeout(timeout).expect("wait for process") {
-        Some(_) => child.wait_with_output().expect("collect output"),
-        None => {
-            let _ = child.kill();
-            let output = child.wait_with_output().expect("output after kill");
-            panic!(
-                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
-                timeout,
-                String::from_utf8_lossy(&output.stdout),
-                String::from_utf8_lossy(&output.stderr)
-            );
-        }
-    }
-}
-
-fn cli_bin() -> PathBuf {
-    let exe_path = std::env::current_exe().expect("current_exe");
-    let deps_dir = exe_path.parent().expect("test binary directory");
-    let bin_dir = deps_dir
-        .parent()
-        .expect("deps parent directory")
-        .to_path_buf();
-    let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-    bin_dir.join(name)
-}
+mod common;
+use common::{cli_bin, run_with_timeout};
 
 fn run_copy(args: &[&str]) -> std::process::Output {
     let bin = cli_bin();

exec
/usr/bin/zsh -lc 'git show --format= --find-renames f6e592e -- crates/blit-core/tests/pull_sync_with_spec_wire.rs crates/blit-core/src/remote/mod.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/remote/mod.rs b/crates/blit-core/src/remote/mod.rs
index a2c72ff..30bd298 100644
--- a/crates/blit-core/src/remote/mod.rs
+++ b/crates/blit-core/src/remote/mod.rs
@@ -1,4 +1,5 @@
 pub mod endpoint;
+pub mod grpc_server;
 pub mod instrumentation;
 pub mod pull;
 pub mod push;
diff --git a/crates/blit-core/tests/pull_sync_with_spec_wire.rs b/crates/blit-core/tests/pull_sync_with_spec_wire.rs
index cc991f3..cbf0378 100644
--- a/crates/blit-core/tests/pull_sync_with_spec_wire.rs
+++ b/crates/blit-core/tests/pull_sync_with_spec_wire.rs
@@ -29,7 +29,11 @@ use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
 use blit_core::remote::pull::{PullSyncError, RemotePullClient};
 use tokio::sync::Mutex;
 use tokio_stream::wrappers::ReceiverStream;
-use tonic::{transport::Server, Request, Response, Status, Streaming};
+// Fake servers start from the shared production-shaped builder
+// (blit_core::remote::grpc_server) so this wire-contract harness
+// carries the deployed HTTP/2 keepalive config (w9-3).
+use blit_core::remote::grpc_server::production_server_builder;
+use tonic::{Request, Response, Status, Streaming};
 
 /// Stub `Blit` impl that captures the first incoming
 /// `ClientPullMessage::Spec` and immediately ends the response stream
@@ -193,7 +197,7 @@ async fn spawn_spy_with_rejection(
             captured,
             reject_pull_sync,
         });
-        Server::builder()
+        production_server_builder()
             .add_service(svc)
             .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
             .await
@@ -614,7 +618,7 @@ async fn spawn_canned_with_acks(
             frames,
             acks,
         });
-        Server::builder()
+        production_server_builder()
             .add_service(svc)
             .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
             .await

codex
The call-site ports mostly preserve the old layouts: context daemon as source, second daemon as destination where needed, and the read-only/delegation flags are still on the side under test. I’m now checking line-level details in the new shared code and the daemon config parser to validate the explicit `delegation_allowed = true` claim.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/tests/common/mod.rs | sed -n '1,520p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Shared integration-test harness (w9-3 consolidation).
     2	//!
     3	//! The single daemon-spawn implementation for every blit-cli test
     4	//! binary: config serialization, port picking, the once-per-binary
     5	//! `cargo build`, spawn + readiness poll, plus the shared `cli_bin()`
     6	//! / `run_with_timeout` helpers and the production-shaped fake-server
     7	//! scaffold. The per-file clones of this logic (remote_remote,
     8	//! remote_pull_mirror, remote_checksum_negotiation,
     9	//! remote_tcp_fallback, and the newer jobs_lifecycle /
    10	//! readonly_enforcement mini-harnesses) were deleted in its favor —
    11	//! extend this file instead of re-cloning it
    12	//! (tests-five-daemon-harness-clones).
    13	
    14	// Every test binary compiles this module and each uses a different
    15	// subset of it, so per-binary dead_code lints would fire on whatever
    16	// that binary happens not to touch. The blanket allow is the honest
    17	// setting for a shared harness, not a mask for genuinely dead code.
    18	#![allow(dead_code)]
    19	
    20	use std::collections::HashSet;
    21	use std::fs;
    22	use std::net::{TcpListener, TcpStream};
    23	use std::path::{Path, PathBuf};
    24	use std::process::{Command, Stdio};
    25	use std::sync::{Mutex, OnceLock};
    26	use std::thread;
    27	use std::time::Duration;
    28	
    29	use serde::Serialize;
    30	use tempfile::tempdir;
    31	use wait_timeout::ChildExt;
    32	
    33	// ---------------------------------------------------------------
    34	// blitd.toml serialization — superset of every knob the deleted
    35	// clones expressed.
    36	// ---------------------------------------------------------------
    37	
    38	#[derive(Serialize)]
    39	pub struct DaemonConfig {
    40	    pub daemon: DaemonSection,
    41	    #[serde(rename = "module")]
    42	    pub modules: Vec<ModuleSection>,
    43	    #[serde(skip_serializing_if = "Option::is_none")]
    44	    pub delegation: Option<DelegationSection>,
    45	}
    46	
    47	#[derive(Serialize)]
    48	pub struct DaemonSection {
    49	    pub bind: String,
    50	    pub port: u16,
    51	    pub no_mdns: bool,
    52	}
    53	
    54	#[derive(Serialize)]
    55	pub struct ModuleSection {
    56	    pub name: String,
    57	    pub path: PathBuf,
    58	    pub comment: Option<String>,
    59	    pub read_only: bool,
    60	    /// The daemon defaults this to `true` when the key is absent
    61	    /// (runtime.rs `default_true`), so serializing an explicit `true`
    62	    /// is behavior-identical to the pre-w9-3 configs that omitted it.
    63	    pub delegation_allowed: bool,
    64	}
    65	
    66	#[derive(Serialize)]
    67	pub struct DelegationSection {
    68	    pub allow_delegated_pull: bool,
    69	    pub allowed_source_hosts: Vec<String>,
    70	}
    71	
    72	// ---------------------------------------------------------------
    73	// Binary discovery + the once-per-binary daemon build.
    74	// ---------------------------------------------------------------
    75	
    76	fn bin_dir() -> PathBuf {
    77	    let exe_path = std::env::current_exe().expect("current_exe");
    78	    exe_path
    79	        .parent()
    80	        .expect("test binary directory")
    81	        .parent()
    82	        .expect("deps parent directory")
    83	        .to_path_buf()
    84	}
    85	
    86	pub fn cli_bin() -> PathBuf {
    87	    bin_dir().join(if cfg!(windows) { "blit.exe" } else { "blit" })
    88	}
    89	
    90	pub fn daemon_bin() -> PathBuf {
    91	    bin_dir().join(if cfg!(windows) {
    92	        "blit-daemon.exe"
    93	    } else {
    94	        "blit-daemon"
    95	    })
    96	}
    97	
    98	/// Build `blit-daemon` at most once per test binary.
    99	///
   100	/// The build exists because `cargo test -p blit-cli` does not build
   101	/// another package's binary; each test binary triggers it itself so
   102	/// no test depends on suite ordering for the daemon to exist (R16-F1,
   103	/// `docs/reviews/followup_review_2026-05-02.md`). Pre-w9-3 every
   104	/// `TestContext::new()` ran its own nested `cargo build` (~75 per
   105	/// full-suite run), all contending for cargo's build-dir lock — the
   106	/// OnceLock keeps the per-process independence guarantee while paying
   107	/// the subprocess cost once per binary
   108	/// (tests-per-test-cargo-build-subprocess).
   109	pub fn ensure_daemon_built() {
   110	    static DAEMON_BUILT: OnceLock<()> = OnceLock::new();
   111	    DAEMON_BUILT.get_or_init(|| {
   112	        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
   113	            .join("../..")
   114	            .canonicalize()
   115	            .expect("workspace root");
   116	        let mut build = Command::new("cargo");
   117	        build.current_dir(workspace_root);
   118	        build
   119	            .arg("build")
   120	            .arg("-p")
   121	            .arg("blit-daemon")
   122	            .arg("--bin")
   123	            .arg("blit-daemon");
   124	        // Cross-target layout (target/<triple>/debug): the triple must
   125	        // be passed through or the daemon lands in the wrong directory.
   126	        let maybe_target = bin_dir()
   127	            .parent()
   128	            .and_then(|p| p.file_name())
   129	            .map(|component| component.to_string_lossy().to_string());
   130	        if let Some(triple) = maybe_target {
   131	            if triple != "target" {
   132	                build.arg("--target").arg(triple);
   133	            }
   134	        }
   135	        let output = build.output().expect("invoke cargo build for blit-daemon");
   136	        assert!(
   137	            output.status.success(),
   138	            "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
   139	            String::from_utf8_lossy(&output.stdout),
   140	            String::from_utf8_lossy(&output.stderr)
   141	        );
   142	    });
   143	}
   144	
   145	// ---------------------------------------------------------------
   146	// Daemon spawn primitive + TestContext builder.
   147	// ---------------------------------------------------------------
   148	
   149	/// Pick a port no other test in THIS process has been handed.
   150	///
   151	/// The probe listener is dropped before the daemon binds, so the OS
   152	/// can hand the same port to two parallel tests probing in the same
   153	/// window — the losing daemon exits on "address in use" and its test
   154	/// then talks to the *winner's* daemon (empty/wrong module, spurious
   155	/// failures). Pre-w9-3 the per-test `cargo build` accidentally
   156	/// serialized bring-ups and mostly hid this; the OnceLock build makes
   157	/// parallel probes routine, so uniqueness is enforced with a
   158	/// process-global claimed-set. Cargo runs test binaries sequentially,
   159	/// so per-process uniqueness is exactly the needed scope; collisions
   160	/// with unrelated system processes remain possible but are caught by
   161	/// the child-death check in `spawn_daemon`.
   162	pub fn pick_unused_port() -> u16 {
   163	    static CLAIMED: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
   164	    let claimed = CLAIMED.get_or_init(|| Mutex::new(HashSet::new()));
   165	    loop {
   166	        let port = TcpListener::bind(("127.0.0.1", 0))
   167	            .expect("bind probe listener")
   168	            .local_addr()
   169	            .expect("listener addr")
   170	            .port();
   171	        let mut claimed = claimed.lock().expect("claimed-port set");
   172	        if claimed.insert(port) {
   173	            return port;
   174	        }
   175	    }
   176	}
   177	
   178	/// Poll until something listens on `127.0.0.1:port` (50 × 100 ms).
   179	pub fn wait_for_port(port: u16, label: &str) {
   180	    for _ in 0..50 {
   181	        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
   182	            return;
   183	        }
   184	        thread::sleep(Duration::from_millis(100));
   185	    }
   186	    panic!("{label} failed to listen on {port}");
   187	}
   188	
   189	/// Per-daemon knobs — everything the deleted harness clones existed
   190	/// to express. (`delegation_allowed` on the module stays hardcoded
   191	/// `true`, the daemon's own default; no test exercises `false` today
   192	/// — add a knob here when one does.)
   193	#[derive(Clone, Default)]
   194	pub struct DaemonOptions {
   195	    /// Export the module `read_only = true` (w9-4's three write gates).
   196	    pub read_only: bool,
   197	    /// Write a `[delegation]` table enabling delegated pull from
   198	    /// loopback. Sources must be authorized by IP/CIDR form, not
   199	    /// hostname — mirrors the production SSRF rule.
   200	    pub delegation: bool,
   201	    /// Extra daemon CLI flags (e.g. `--no-server-checksums`,
   202	    /// `--force-grpc-data`).
   203	    pub extra_args: Vec<String>,
   204	}
   205	
   206	/// One spawned daemon: its port, its module directory, and the child
   207	/// guard that kills it on drop.
   208	pub struct SpawnedDaemon {
   209	    pub port: u16,
   210	    pub module_dir: PathBuf,
   211	    pub daemon: ChildGuard,
   212	}
   213	
   214	/// Spawn one daemon under `workspace`: writes `<name>.toml`, creates
   215	/// `module_dir` if missing, picks a fresh port, waits for readiness.
   216	/// `TestContext` routes through this; dual-daemon tests call it (via
   217	/// `TestContext::spawn_second_daemon`) for their second daemon.
   218	pub fn spawn_daemon(
   219	    workspace: &Path,
   220	    name: &str,
   221	    module_dir: &Path,
   222	    opts: &DaemonOptions,
   223	) -> SpawnedDaemon {
   224	    ensure_daemon_built();
   225	
   226	    fs::create_dir_all(module_dir).expect("module dir");
   227	    let port = pick_unused_port();
   228	
   229	    let config = DaemonConfig {
   230	        daemon: DaemonSection {
   231	            bind: "127.0.0.1".into(),
   232	            port,
   233	            no_mdns: true,
   234	        },
   235	        modules: vec![ModuleSection {
   236	            name: "test".into(),
   237	            path: module_dir.to_path_buf(),
   238	            comment: None,
   239	            read_only: opts.read_only,
   240	            delegation_allowed: true,
   241	        }],
   242	        delegation: opts.delegation.then(|| DelegationSection {
   243	            allow_delegated_pull: true,
   244	            // Loopback sources must be authorized by IP/CIDR form, not
   245	            // hostname form. This mirrors the production SSRF rule.
   246	            allowed_source_hosts: vec!["127.0.0.1".to_string()],
   247	        }),
   248	    };
   249	    let config_path = workspace.join(format!("{name}.toml"));
   250	    let toml = toml::to_string(&config).expect("serialize config");
   251	    fs::write(&config_path, toml).expect("write config");
   252	
   253	    let mut cmd = Command::new(daemon_bin());
   254	    cmd.arg("--config")
   255	        .arg(&config_path)
   256	        .arg("--bind")
   257	        .arg("127.0.0.1")
   258	        .arg("--port")
   259	        .arg(port.to_string());
   260	    for arg in &opts.extra_args {
   261	        cmd.arg(arg);
   262	    }
   263	    // stderr policy: discard. The pre-w9-3 shared harness piped stderr
   264	    // "for debugging" but nothing ever read it — zero diagnostics in
   265	    // practice plus a latent pipe-buffer deadlock once a chatty daemon
   266	    // wrote 64 KiB. Real capture (drain thread, dump on readiness
   267	    // failure) is w9-6 (tests-harness-stderr-blackhole).
   268	    let mut child = cmd
   269	        .stdin(Stdio::null())
   270	        .stdout(Stdio::null())
   271	        .stderr(Stdio::null())
   272	        .spawn()
   273	        .expect("spawn daemon");
   274	
   275	    // Readiness poll with a child-death check: a daemon that exits
   276	    // during startup (port stolen by an unrelated process, config
   277	    // rejected) fails fast with the real reason instead of a generic
   278	    // 5s timeout — and, crucially, never leaves the test silently
   279	    // talking to some other test's daemon on the same port.
   280	    let mut ready = false;
   281	    for _ in 0..50 {
   282	        if let Some(status) = child.try_wait().expect("poll spawned daemon") {
   283	            panic!(
   284	                "daemon {name} exited during startup ({status}); \
   285	                 port {port} taken or config rejected"
   286	            );
   287	        }
   288	        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
   289	            ready = true;
   290	            break;
   291	        }
   292	        thread::sleep(Duration::from_millis(100));
   293	    }
   294	    let daemon = ChildGuard::new(child);
   295	    assert!(ready, "daemon {name} failed to listen on {port}");
   296	
   297	    SpawnedDaemon {
   298	        port,
   299	        module_dir: module_dir.to_path_buf(),
   300	        daemon,
   301	    }
   302	}
   303	
   304	pub struct TestContext {
   305	    pub _work: tempfile::TempDir,
   306	    pub workspace: PathBuf,
   307	    pub daemon_port: u16,
   308	    pub daemon: ChildGuard,
   309	    pub cli_bin: PathBuf,
   310	    pub config_dir: PathBuf,
   311	    pub module_dir: PathBuf,
   312	}
   313	
   314	/// Knob surface for the primary daemon; `build()` yields the context.
   315	pub struct TestContextBuilder {
   316	    opts: DaemonOptions,
   317	}
   318	
   319	impl TestContextBuilder {
   320	    pub fn read_only(mut self, read_only: bool) -> Self {
   321	        self.opts.read_only = read_only;
   322	        self
   323	    }
   324	
   325	    pub fn delegation(mut self, delegation: bool) -> Self {
   326	        self.opts.delegation = delegation;
   327	        self
   328	    }
   329	
   330	    pub fn extra_daemon_args<I, S>(mut self, args: I) -> Self
   331	    where
   332	        I: IntoIterator<Item = S>,
   333	        S: Into<String>,
   334	    {
   335	        self.opts.extra_args = args.into_iter().map(Into::into).collect();
   336	        self
   337	    }
   338	
   339	    pub fn build(self) -> TestContext {
   340	        let work = tempdir().expect("tempdir");
   341	        let workspace = work.path().to_path_buf();
   342	
   343	        let config_dir = workspace.join("cli-config");
   344	        fs::create_dir_all(&config_dir).expect("cli config");
   345	
   346	        let spawned = spawn_daemon(&workspace, "blitd", &workspace.join("module"), &self.opts);
   347	
   348	        TestContext {
   349	            _work: work,
   350	            workspace,
   351	            daemon_port: spawned.port,
   352	            daemon: spawned.daemon,
   353	            cli_bin: cli_bin(),
   354	            config_dir,
   355	            module_dir: spawned.module_dir,
   356	        }
   357	    }
   358	}
   359	
   360	impl TestContext {
   361	    pub fn builder() -> TestContextBuilder {
   362	        TestContextBuilder {
   363	            opts: DaemonOptions::default(),
   364	        }
   365	    }
   366	
   367	    pub fn new() -> Self {
   368	        Self::builder().build()
   369	    }
   370	
   371	    /// w9-4: same daemon + module, but the module is exported
   372	    /// `read_only: true` so tests can exercise the three write gates
   373	    /// (push, purge, delegated pull). Before this knob existed no
   374	    /// test config in the workspace could express a read-only module.
   375	    pub fn new_read_only() -> Self {
   376	        Self::builder().read_only(true).build()
   377	    }
   378	
   379	    /// Spawn an additional daemon in this context's workspace with its
   380	    /// own module dir (`module_<name>`) and config (`<name>.toml`).
   381	    /// The dual-daemon delegation tests build on this.
   382	    pub fn spawn_second_daemon(&self, name: &str, opts: &DaemonOptions) -> SpawnedDaemon {
   383	        spawn_daemon(
   384	            &self.workspace,
   385	            name,
   386	            &self.workspace.join(format!("module_{name}")),
   387	            opts,
   388	        )
   389	    }
   390	}
   391	
   392	// ---------------------------------------------------------------
   393	// Child-process plumbing shared across binaries.
   394	// ---------------------------------------------------------------
   395	
   396	pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
   397	    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
   398	    let mut child = cmd.spawn().expect("spawn command");
   399	
   400	    match child.wait_timeout(timeout).expect("wait for process") {
   401	        Some(_status) => child
   402	            .wait_with_output()
   403	            .expect("collect command output after completion"),
   404	        None => {
   405	            let _ = child.kill();
   406	            let output = child
   407	                .wait_with_output()
   408	                .expect("collect output after killing command");
   409	            panic!(
   410	                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
   411	                timeout,
   412	                String::from_utf8_lossy(&output.stdout),
   413	                String::from_utf8_lossy(&output.stderr)
   414	            );
   415	        }
   416	    }
   417	}
   418	
   419	pub struct ChildGuard {
   420	    pub child: Option<std::process::Child>,
   421	}
   422	
   423	impl ChildGuard {
   424	    pub fn new(child: std::process::Child) -> Self {
   425	        Self { child: Some(child) }
   426	    }
   427	
   428	    /// Kill + reap now instead of at scope end — for tests that must
   429	    /// assert on filesystem state after the daemon is gone.
   430	    pub fn terminate(&mut self) {
   431	        if let Some(mut child) = self.child.take() {
   432	            let _ = child.kill();
   433	            let _ = child.wait();
   434	        }
   435	    }
   436	}
   437	
   438	impl Drop for ChildGuard {
   439	    fn drop(&mut self) {
   440	        self.terminate();
   441	    }
   442	}
   443	
   444	// ---------------------------------------------------------------
   445	// In-process fake Blit gRPC servers (wire-shape tests).
   446	// ---------------------------------------------------------------
   447	
   448	/// A fake server running on its own thread + current_thread runtime;
   449	/// Drop signals shutdown and joins the thread.
   450	pub struct FakeServerGuard {
   451	    pub port: u16,
   452	    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
   453	    join: Option<thread::JoinHandle<()>>,
   454	}
   455	
   456	impl Drop for FakeServerGuard {
   457	    fn drop(&mut self) {
   458	        if let Some(tx) = self.shutdown.take() {
   459	            let _ = tx.send(());
   460	        }
   461	        if let Some(join) = self.join.take() {
   462	            let _ = join.join();
   463	        }
   464	    }
   465	}
   466	
   467	/// Serve `svc` on an ephemeral loopback port, production-shaped: the
   468	/// builder comes from `blit_core::remote::grpc_server`, so fakes carry
   469	/// the same HTTP/2 keepalive config as a real daemon
   470	/// (tests-fake-server-config-skew).
   471	pub fn spawn_fake_blit_server<S>(svc: S, label: &str) -> FakeServerGuard
   472	where
   473	    S: blit_core::generated::blit_server::Blit,
   474	{
   475	    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake server");
   476	    let port = listener.local_addr().expect("fake server addr").port();
   477	    listener
   478	        .set_nonblocking(true)
   479	        .expect("set fake server nonblocking");
   480	    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
   481	
   482	    let join = thread::spawn(move || {
   483	        let runtime = tokio::runtime::Builder::new_current_thread()
   484	            .enable_all()
   485	            .build()
   486	            .expect("fake server runtime");
   487	        runtime.block_on(async move {
   488	            use blit_core::generated::blit_server::BlitServer;
   489	            use tokio_stream::wrappers::TcpListenerStream;
   490	
   491	            let listener =
   492	                tokio::net::TcpListener::from_std(listener).expect("tokio fake listener");
   493	            blit_core::remote::grpc_server::production_server_builder()
   494	                .add_service(BlitServer::new(svc))
   495	                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
   496	                    let _ = shutdown_rx.await;
   497	                })
   498	                .await
   499	                .expect("fake server");
   500	        });
   501	    });
   502	
   503	    wait_for_port(port, label);
   504	    FakeServerGuard {
   505	        port,
   506	        shutdown: Some(shutdown_tx),
   507	        join: Some(join),
   508	    }
   509	}

exec
/usr/bin/zsh -lc "rg -n \"delegation_allowed|default_true|struct Module|ModuleConfig|read_only\" crates/blit-core crates/blit-daemon crates/blit-cli/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-cli/src/jobs.rs:659:                "read_only": m.read_only,
crates/blit-daemon/src/main.rs:48:            if root.read_only { " (read-only)" } else { "" }
crates/blit-cli/src/list_modules.rs:27:            let mode = if module.read_only { "ro" } else { "rw" };
crates/blit-daemon/src/runtime.rs:11:pub(crate) struct ModuleConfig {
crates/blit-daemon/src/runtime.rs:25:    pub(crate) read_only: bool,
crates/blit-daemon/src/runtime.rs:37:    pub(crate) delegation_allowed: bool,
crates/blit-daemon/src/runtime.rs:43:    /// Canonicalized form of `path`; see `ModuleConfig::canonical_root`.
crates/blit-daemon/src/runtime.rs:45:    pub(crate) read_only: bool,
crates/blit-daemon/src/runtime.rs:51:    pub(crate) read_only: bool,
crates/blit-daemon/src/runtime.rs:64:    pub(crate) modules: HashMap<String, ModuleConfig>,
crates/blit-daemon/src/runtime.rs:131:    root_read_only: bool,
crates/blit-daemon/src/runtime.rs:154:    read_only: bool,
crates/blit-daemon/src/runtime.rs:156:    /// configs unaffected. See `ModuleConfig::delegation_allowed`.
crates/blit-daemon/src/runtime.rs:157:    #[serde(default = "default_true")]
crates/blit-daemon/src/runtime.rs:158:    delegation_allowed: bool,
crates/blit-daemon/src/runtime.rs:161:fn default_true() -> bool {
crates/blit-daemon/src/runtime.rs:261:            ModuleConfig {
crates/blit-daemon/src/runtime.rs:265:                read_only: module.read_only,
crates/blit-daemon/src/runtime.rs:267:                delegation_allowed: module.delegation_allowed,
crates/blit-daemon/src/runtime.rs:275:            read_only: false,
crates/blit-daemon/src/runtime.rs:280:            read_only: raw.daemon.root_read_only,
crates/blit-daemon/src/runtime.rs:299:                read_only: false,
crates/blit-daemon/src/runtime.rs:310:            ModuleConfig {
crates/blit-daemon/src/runtime.rs:314:                read_only: chosen.read_only,
crates/blit-daemon/src/runtime.rs:318:                delegation_allowed: true,
crates/blit-daemon/src/runtime.rs:324:            read_only: chosen.read_only,
crates/blit-daemon/src/runtime.rs:336:            read_only: spec.read_only,
crates/blit-daemon/src/runtime.rs:441:    fn per_module_delegation_allowed_defaults_true() {
crates/blit-daemon/src/runtime.rs:442:        // A module without an explicit `delegation_allowed` setting
crates/blit-daemon/src/runtime.rs:469:        assert!(runtime.modules["alpha"].delegation_allowed);
crates/blit-daemon/src/runtime.rs:473:    fn per_module_delegation_allowed_can_opt_out() {
crates/blit-daemon/src/runtime.rs:483:                delegation_allowed = false
crates/blit-daemon/src/runtime.rs:500:        assert!(!runtime.modules["alpha"].delegation_allowed);
crates/blit-daemon/src/delegation_gate.rs:96:    /// the handler when `ModuleConfig::delegation_allowed` is false.
crates/blit-daemon/src/service/pull_sync.rs:12:use crate::runtime::{ModuleConfig, RootExport};
crates/blit-daemon/src/service/pull_sync.rs:47:    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
crates/blit-daemon/src/service/pull_sync.rs:602:    _module: &ModuleConfig,
crates/blit-daemon/src/service/pull_sync.rs:648:    _module: &ModuleConfig,
crates/blit-daemon/src/service/pull_sync.rs:1231:    module: &ModuleConfig,
crates/blit-daemon/src/service/pull_sync.rs:1484:    module: &ModuleConfig,
crates/blit-daemon/src/service/delegated_pull.rs:29:use crate::runtime::{ModuleConfig, RootExport};
crates/blit-daemon/src/service/delegated_pull.rs:147:    modules: Arc<tokio::sync::Mutex<std::collections::HashMap<String, ModuleConfig>>>,
crates/blit-daemon/src/service/delegated_pull.rs:186:    modules: Arc<tokio::sync::Mutex<std::collections::HashMap<String, ModuleConfig>>>,
crates/blit-daemon/src/service/delegated_pull.rs:248:    if !module.delegation_allowed {
crates/blit-daemon/src/service/delegated_pull.rs:258:    if module.read_only {
crates/blit-daemon/src/service/delegated_pull.rs:1044:            ModuleConfig,
crates/blit-daemon/src/service/core.rs:13:use crate::runtime::{ModuleConfig, RootExport};
crates/blit-daemon/src/service/core.rs:64:    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
crates/blit-daemon/src/service/core.rs:95:        modules: HashMap<String, ModuleConfig>,
crates/blit-daemon/src/service/core.rs:119:        modules: HashMap<String, ModuleConfig>,
crates/blit-daemon/src/service/core.rs:138:        if module.read_only {
crates/blit-daemon/src/service/core.rs:944:                read_only: module.read_only,
crates/blit-daemon/src/service/core.rs:970:                    read_only: module.read_only,
crates/blit-daemon/src/service/util.rs:1:use crate::runtime::{ModuleConfig, RootExport};
crates/blit-daemon/src/service/util.rs:10:    modules: &Arc<Mutex<HashMap<String, ModuleConfig>>>,
crates/blit-daemon/src/service/util.rs:13:) -> Result<ModuleConfig, Status> {
crates/blit-daemon/src/service/util.rs:16:            return Ok(ModuleConfig {
crates/blit-daemon/src/service/util.rs:20:                read_only: root.read_only,
crates/blit-daemon/src/service/util.rs:24:                delegation_allowed: true,
crates/blit-daemon/src/service/util.rs:107:pub(crate) fn resolve_contained_path(module: &ModuleConfig, rel: &Path) -> Result<PathBuf, Status> {
crates/blit-daemon/src/service/util.rs:122:pub(crate) fn resolve_contained_wire(module: &ModuleConfig, wire: &str) -> Result<PathBuf, Status> {
crates/blit-daemon/src/service/push/data_plane.rs:1:use crate::runtime::ModuleConfig;
crates/blit-daemon/src/service/push/data_plane.rs:70:    module: ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:140:    module: ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:175:    module: ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:278:    module: ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:426:    module: ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:545:    module: &ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:832:    module: &ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:923:        module: ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:1028:    module: ModuleConfig,
crates/blit-daemon/src/service/push/data_plane.rs:1184:        let module = ModuleConfig {
crates/blit-daemon/src/service/push/data_plane.rs:1188:            read_only: false,
crates/blit-daemon/src/service/push/data_plane.rs:1190:            delegation_allowed: true,
crates/blit-daemon/src/service/push/data_plane.rs:1263:        let module = ModuleConfig {
crates/blit-daemon/src/service/push/data_plane.rs:1267:            read_only: false,
crates/blit-daemon/src/service/push/data_plane.rs:1269:            delegation_allowed: true,
crates/blit-daemon/src/service/push/data_plane.rs:1369:    fn module_for_test(path: PathBuf) -> ModuleConfig {
crates/blit-daemon/src/service/push/data_plane.rs:1371:        ModuleConfig {
crates/blit-daemon/src/service/push/data_plane.rs:1375:            read_only: false,
crates/blit-daemon/src/service/push/data_plane.rs:1377:            delegation_allowed: true,
crates/blit-daemon/src/service/push/control.rs:10:use crate::runtime::{ModuleConfig, RootExport};
crates/blit-daemon/src/service/push/control.rs:62:    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
crates/blit-daemon/src/service/push/control.rs:69:    let mut module: Option<ModuleConfig> = None;
crates/blit-daemon/src/service/push/control.rs:121:                if config.read_only {
crates/blit-daemon/src/service/push/control.rs:724:    module: &ModuleConfig,
crates/blit-daemon/src/service/push/control.rs:767:    module: &ModuleConfig,
crates/blit-daemon/src/service/push/control.rs:870:    fn test_module(root: &Path) -> ModuleConfig {
crates/blit-daemon/src/service/push/control.rs:871:        ModuleConfig {
crates/blit-daemon/src/service/push/control.rs:875:            read_only: false,
crates/blit-daemon/src/service/push/control.rs:877:            delegation_allowed: true,

exec
/usr/bin/zsh -lc "rg -n \"Server::builder\\(|production_server_builder|http2_keepalive\" crates proto -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/main.rs:134:    blit_core::remote::grpc_server::production_server_builder()
crates/blit-core/tests/pull_sync_with_spec_wire.rs:35:use blit_core::remote::grpc_server::production_server_builder;
crates/blit-core/tests/pull_sync_with_spec_wire.rs:200:        production_server_builder()
crates/blit-core/tests/pull_sync_with_spec_wire.rs:621:        production_server_builder()
crates/blit-core/src/remote/grpc_server.rs:7://! tonic server in the test tree was a bare `Server::builder()` with
crates/blit-core/src/remote/grpc_server.rs:32:/// The `Server::builder()` every Blit gRPC server starts from — the
crates/blit-core/src/remote/grpc_server.rs:35:pub fn production_server_builder() -> Server {
crates/blit-core/src/remote/grpc_server.rs:36:    Server::builder()
crates/blit-core/src/remote/grpc_server.rs:37:        .http2_keepalive_interval(Some(HTTP2_KEEPALIVE_INTERVAL))
crates/blit-core/src/remote/grpc_server.rs:38:        .http2_keepalive_timeout(Some(HTTP2_KEEPALIVE_TIMEOUT))
crates/blit-cli/tests/common/mod.rs:493:            blit_core::remote::grpc_server::production_server_builder()

exec
/usr/bin/zsh -lc "rg -n \"fn .*\\(\\).*\\{|#\\[test\\]|#\\[tokio::test\\]\" crates/blit-cli/tests/remote_remote.rs crates/blit-cli/tests/remote_pull_mirror.rs crates/blit-cli/tests/remote_checksum_negotiation.rs crates/blit-cli/tests/remote_tcp_fallback.rs crates/blit-cli/tests/jobs_lifecycle.rs crates/blit-cli/tests/readonly_enforcement.rs crates/blit-cli/tests/cli_arg_safety_gates.rs crates/blit-cli/tests/diagnostics_dump.rs crates/blit-cli/tests/local_move_semantics.rs crates/blit-cli/tests/single_file_copy.rs crates/blit-core/tests/pull_sync_with_spec_wire.rs crates/blit-core/src/remote/grpc_server.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/grpc_server.rs:35:pub fn production_server_builder() -> Server {
crates/blit-core/src/remote/grpc_server.rs:48:    #[test]
crates/blit-core/src/remote/grpc_server.rs:49:    fn keepalive_values_match_owner_decision() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:215:fn hand_built_spec() -> TransferOperationSpec {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:265:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:266:async fn pull_sync_with_spec_forwards_spec_unchanged_on_wire() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:326:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:327:async fn pull_sync_wrapper_emits_same_spec_as_build_spec_from_options() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:396:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:397:async fn pull_sync_with_spec_classifies_initial_rpc_rejection_as_negotiation() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:653:fn benign_summary() -> server_pull_message::Payload {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:663:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:664:async fn scan_remote_files_collects_bare_headers_and_sends_metadata_only_spec() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:718:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:719:async fn scan_remote_files_survives_old_daemon_streaming_data() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:765:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:766:async fn scan_remote_files_rejects_real_data_plane_negotiation() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:797:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:798:async fn open_remote_file_yields_bytes_and_eofs_at_summary() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:843:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:844:async fn open_remote_file_rejects_tar_shard_frames() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:878:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:879:async fn open_remote_file_rejects_second_file_header() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:920:#[tokio::test]
crates/blit-core/tests/pull_sync_with_spec_wire.rs:921:async fn pull_client_refuses_resize_on_a_session_that_never_negotiated_it() {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:971:async fn pull_client_dials_resize_add_with_the_epoch_credential_and_acks() {
crates/blit-cli/tests/local_move_semantics.rs:21:#[test]
crates/blit-cli/tests/local_move_semantics.rs:22:fn local_move_preserves_unrelated_destination_entries() {
crates/blit-cli/tests/local_move_semantics.rs:93:#[test]
crates/blit-cli/tests/local_move_semantics.rs:94:fn local_move_refuses_when_source_scan_incomplete() {
crates/blit-cli/tests/local_move_semantics.rs:164:#[test]
crates/blit-cli/tests/local_move_semantics.rs:165:fn local_move_rejects_filter_args() {
crates/blit-cli/tests/local_move_semantics.rs:211:#[test]
crates/blit-cli/tests/local_move_semantics.rs:212:fn local_move_json_no_premature_success_output_on_refusal() {
crates/blit-cli/tests/local_move_semantics.rs:267:#[test]
crates/blit-cli/tests/local_move_semantics.rs:268:fn local_move_rejects_ignore_existing() {
crates/blit-cli/tests/local_move_semantics.rs:307:#[test]
crates/blit-cli/tests/local_move_semantics.rs:308:fn local_move_rejects_null_sink() {
crates/blit-cli/tests/local_move_semantics.rs:352:#[test]
crates/blit-cli/tests/local_move_semantics.rs:353:fn remote_to_remote_move_rejects_relay_via_cli() {
crates/blit-cli/tests/single_file_copy.rs:24:#[test]
crates/blit-cli/tests/single_file_copy.rs:25:fn single_file_to_dir_with_trailing_slash() {
crates/blit-cli/tests/single_file_copy.rs:43:#[test]
crates/blit-cli/tests/single_file_copy.rs:44:fn single_file_to_existing_dir_no_slash() {
crates/blit-cli/tests/single_file_copy.rs:56:#[test]
crates/blit-cli/tests/single_file_copy.rs:57:fn single_file_rename_to_exact_path() {
crates/blit-cli/tests/single_file_copy.rs:70:#[test]
crates/blit-cli/tests/single_file_copy.rs:71:fn single_file_copy_reports_nonzero_files() {
crates/blit-cli/tests/single_file_copy.rs:92:#[test]
crates/blit-cli/tests/single_file_copy.rs:93:fn single_file_copy_idempotent() {
crates/blit-cli/tests/single_file_copy.rs:131:#[test]
crates/blit-cli/tests/single_file_copy.rs:132:fn single_file_missing_source_fails() {
crates/blit-cli/tests/diagnostics_dump.rs:22:#[test]
crates/blit-cli/tests/diagnostics_dump.rs:23:fn dump_local_to_local_json_shape() {
crates/blit-cli/tests/diagnostics_dump.rs:56:#[test]
crates/blit-cli/tests/diagnostics_dump.rs:57:fn dump_detects_rsync_basename_append() {
crates/blit-cli/tests/diagnostics_dump.rs:87:#[test]
crates/blit-cli/tests/diagnostics_dump.rs:88:fn dump_remote_destination_captures_endpoint_fields() {
crates/blit-cli/tests/readonly_enforcement.rs:35:#[test]
crates/blit-cli/tests/readonly_enforcement.rs:36:fn push_to_read_only_module_is_rejected_and_module_untouched() {
crates/blit-cli/tests/readonly_enforcement.rs:70:#[test]
crates/blit-cli/tests/readonly_enforcement.rs:71:fn push_rejection_reason_survives_midmanifest_send_failure() {
crates/blit-cli/tests/readonly_enforcement.rs:100:#[test]
crates/blit-cli/tests/readonly_enforcement.rs:101:fn purge_on_read_only_module_is_rejected_and_file_survives() {
crates/blit-cli/tests/readonly_enforcement.rs:133:#[test]
crates/blit-cli/tests/readonly_enforcement.rs:134:fn delegated_pull_to_read_only_destination_is_rejected() {
crates/blit-cli/tests/cli_arg_safety_gates.rs:50:#[test]
crates/blit-cli/tests/cli_arg_safety_gates.rs:51:fn mirror_rejects_null_sink() {
crates/blit-cli/tests/cli_arg_safety_gates.rs:82:#[test]
crates/blit-cli/tests/cli_arg_safety_gates.rs:83:fn copy_rejects_null_with_remote_destination() {
crates/blit-cli/tests/cli_arg_safety_gates.rs:101:#[test]
crates/blit-cli/tests/cli_arg_safety_gates.rs:102:fn copy_rejects_null_with_remote_source() {
crates/blit-cli/tests/cli_arg_safety_gates.rs:121:#[test]
crates/blit-cli/tests/cli_arg_safety_gates.rs:122:fn local_copy_null_still_accepted() {
crates/blit-cli/tests/cli_arg_safety_gates.rs:149:#[test]
crates/blit-cli/tests/cli_arg_safety_gates.rs:150:fn local_move_rejects_force_flag() {
crates/blit-cli/tests/cli_arg_safety_gates.rs:196:#[test]
crates/blit-cli/tests/cli_arg_safety_gates.rs:197:fn local_move_rejects_ignore_times_flag() {
crates/blit-cli/tests/remote_checksum_negotiation.rs:27:#[test]
crates/blit-cli/tests/remote_checksum_negotiation.rs:28:fn pull_checksum_rejected_when_daemon_disables_checksums() {
crates/blit-cli/tests/remote_checksum_negotiation.rs:71:#[test]
crates/blit-cli/tests/remote_checksum_negotiation.rs:72:fn pull_checksum_succeeds_when_daemon_enables_checksums() {
crates/blit-cli/tests/remote_tcp_fallback.rs:9:fn forced_grpc_ctx() -> TestContext {
crates/blit-cli/tests/remote_tcp_fallback.rs:15:#[test]
crates/blit-cli/tests/remote_tcp_fallback.rs:16:fn remote_push_falls_back_to_grpc_when_forced() {
crates/blit-cli/tests/remote_tcp_fallback.rs:143:#[test]
crates/blit-cli/tests/remote_tcp_fallback.rs:144:fn forced_grpc_push_many_files_completes() {
crates/blit-cli/tests/remote_tcp_fallback.rs:154:#[test]
crates/blit-cli/tests/remote_tcp_fallback.rs:156:fn forced_grpc_push_overflows_old_upload_channel_capacity() {
crates/blit-cli/tests/remote_pull_mirror.rs:8:#[test]
crates/blit-cli/tests/remote_pull_mirror.rs:9:fn remote_pull_mirror_purges_extraneous_local_files() {
crates/blit-cli/tests/remote_pull_mirror.rs:53:#[test]
crates/blit-cli/tests/remote_pull_mirror.rs:54:fn remote_pull_mirror_filtered_subset_preserves_out_of_scope_files() {
crates/blit-cli/tests/remote_pull_mirror.rs:105:#[test]
crates/blit-cli/tests/remote_pull_mirror.rs:106:fn remote_pull_mirror_delete_scope_all_purges_out_of_scope() {
crates/blit-cli/tests/remote_remote.rs:63:#[test]
crates/blit-cli/tests/remote_remote.rs:64:fn remote_to_remote_copy_delegates_directly_without_cli_byte_path() {
crates/blit-cli/tests/remote_remote.rs:92:#[test]
crates/blit-cli/tests/remote_remote.rs:93:fn remote_to_remote_gate_reject_does_not_fall_back_to_relay() {
crates/blit-cli/tests/remote_remote.rs:119:#[test]
crates/blit-cli/tests/remote_remote.rs:120:fn remote_to_remote_explicit_relay_uses_legacy_cli_byte_path() {
crates/blit-cli/tests/remote_remote.rs:153:#[test]
crates/blit-cli/tests/remote_remote.rs:154:fn remote_to_remote_relay_transfers_nested_tree() {
crates/blit-cli/tests/remote_remote.rs:199:#[test]
crates/blit-cli/tests/remote_remote.rs:200:fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
crates/blit-cli/tests/remote_remote.rs:235:#[test]
crates/blit-cli/tests/remote_remote.rs:236:fn source_refuses_destination_negotiation_does_not_fall_back_to_relay() {
crates/blit-cli/tests/jobs_lifecycle.rs:31:#[test]
crates/blit-cli/tests/jobs_lifecycle.rs:32:fn jobs_list_on_idle_daemon_exits_zero_with_empty_active() {
crates/blit-cli/tests/jobs_lifecycle.rs:63:#[test]
crates/blit-cli/tests/jobs_lifecycle.rs:64:fn jobs_cancel_unknown_id_exits_one() {
crates/blit-cli/tests/jobs_lifecycle.rs:84:#[test]
crates/blit-cli/tests/jobs_lifecycle.rs:85:fn jobs_watch_unknown_id_exits_two() {
crates/blit-cli/tests/jobs_lifecycle.rs:125:    fn with_real_source() -> Self {
crates/blit-cli/tests/jobs_lifecycle.rs:220:#[test]
crates/blit-cli/tests/jobs_lifecycle.rs:221:fn detached_copy_watch_to_terminal_then_cancel_is_not_found() {
crates/blit-cli/tests/jobs_lifecycle.rs:283:#[test]
crates/blit-cli/tests/jobs_lifecycle.rs:284:fn cancel_of_active_delegated_job_exits_zero() {
crates/blit-cli/tests/jobs_lifecycle.rs:312:fn spawn_stalling_source() -> common::FakeServerGuard {

codex
The parser confirms `delegation_allowed` really defaults true when absent, and grep shows the old bare `Server::builder()` call sites are gone outside the new shared builder. I’m doing one pass over generated configs, test counts, and potential compile/lifetime issues before settling the verdict.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/runtime.rs | sed -n '140,175p;430,505p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   140	struct RawDelegationSection {
   141	    #[serde(default)]
   142	    allow_delegated_pull: bool,
   143	    #[serde(default)]
   144	    allowed_source_hosts: Vec<String>,
   145	}
   146	
   147	#[derive(Debug, Deserialize)]
   148	struct RawModule {
   149	    name: String,
   150	    path: PathBuf,
   151	    #[serde(default)]
   152	    comment: Option<String>,
   153	    #[serde(default)]
   154	    read_only: bool,
   155	    /// Per-module narrowing override. Defaults to true so existing
   156	    /// configs unaffected. See `ModuleConfig::delegation_allowed`.
   157	    #[serde(default = "default_true")]
   158	    delegation_allowed: bool,
   159	}
   160	
   161	fn default_true() -> bool {
   162	    true
   163	}
   164	
   165	fn default_config_path() -> PathBuf {
   166	    if cfg!(windows) {
   167	        PathBuf::from(r"C:\ProgramData\Blit\config.toml")
   168	    } else {
   169	        PathBuf::from("/etc/blit/config.toml")
   170	    }
   171	}
   172	
   173	pub(crate) fn load_runtime(args: &DaemonArgs) -> Result<DaemonRuntime> {
   174	    let mut warnings = Vec::new();
   175	
   430	    #[test]
   431	    fn delegation_block_omitted_defaults_to_disabled() {
   432	        // Existing configs without [delegation] still load.
   433	        let toml = "";
   434	        let (_dir, args) = with_config(toml);
   435	        let runtime = load_runtime(&args).expect("default load");
   436	        assert!(!runtime.delegation.allow_delegated_pull);
   437	        assert!(runtime.delegation.allowed_source_hosts.is_empty());
   438	    }
   439	
   440	    #[test]
   441	    fn per_module_delegation_allowed_defaults_true() {
   442	        // A module without an explicit `delegation_allowed` setting
   443	        // follows the daemon-wide policy without further narrowing.
   444	        let dir = tempfile::tempdir().expect("tempdir");
   445	        let mod_path = dir.path().join("mod1");
   446	        std::fs::create_dir_all(&mod_path).expect("create module dir");
   447	        let cfg_path = dir.path().join("config.toml");
   448	        let toml = format!(
   449	            r#"
   450	                [[module]]
   451	                name = "alpha"
   452	                path = {path:?}
   453	            "#,
   454	            path = mod_path.canonicalize().unwrap().to_str().unwrap()
   455	        );
   456	        std::fs::write(&cfg_path, toml).expect("write config");
   457	        let args = DaemonArgs {
   458	            config: Some(cfg_path),
   459	            bind: None,
   460	            port: None,
   461	            root: None,
   462	            no_mdns: true,
   463	            mdns_name: None,
   464	            force_grpc_data: false,
   465	            no_server_checksums: false,
   466	            metrics: false,
   467	        };
   468	        let runtime = load_runtime(&args).expect("config loads");
   469	        assert!(runtime.modules["alpha"].delegation_allowed);
   470	    }
   471	
   472	    #[test]
   473	    fn per_module_delegation_allowed_can_opt_out() {
   474	        let dir = tempfile::tempdir().expect("tempdir");
   475	        let mod_path = dir.path().join("mod1");
   476	        std::fs::create_dir_all(&mod_path).expect("create module dir");
   477	        let cfg_path = dir.path().join("config.toml");
   478	        let toml = format!(
   479	            r#"
   480	                [[module]]
   481	                name = "alpha"
   482	                path = {path:?}
   483	                delegation_allowed = false
   484	            "#,
   485	            path = mod_path.canonicalize().unwrap().to_str().unwrap()
   486	        );
   487	        std::fs::write(&cfg_path, toml).expect("write config");
   488	        let args = DaemonArgs {
   489	            config: Some(cfg_path),
   490	            bind: None,
   491	            port: None,
   492	            root: None,
   493	            no_mdns: true,
   494	            mdns_name: None,
   495	            force_grpc_data: false,
   496	            no_server_checksums: false,
   497	            metrics: false,
   498	        };
   499	        let runtime = load_runtime(&args).expect("config loads");
   500	        assert!(!runtime.modules["alpha"].delegation_allowed);
   501	    }
   502	}

exec
/usr/bin/zsh -lc "git grep -n \"fn run_with_timeout\\|fn cli_bin\\|fn pick_unused_port\\|fn spawn_daemon\\|binary_paths\\|Server::builder\" f6e592e -- crates/blit-cli/tests crates/blit-core/tests crates/blit-daemon/src crates/blit-core/src | sed -n '1,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
f6e592e:crates/blit-cli/tests/common/mod.rs:86:pub fn cli_bin() -> PathBuf {
f6e592e:crates/blit-cli/tests/common/mod.rs:162:pub fn pick_unused_port() -> u16 {
f6e592e:crates/blit-cli/tests/common/mod.rs:218:pub fn spawn_daemon(
f6e592e:crates/blit-cli/tests/common/mod.rs:396:pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
f6e592e:crates/blit-cli/tests/remote_checksum_negotiation.rs:21:fn spawn_daemon_harness(extra_daemon_args: &[&str]) -> TestContext {
f6e592e:crates/blit-core/src/remote/grpc_server.rs:7://! tonic server in the test tree was a bare `Server::builder()` with
f6e592e:crates/blit-core/src/remote/grpc_server.rs:32:/// The `Server::builder()` every Blit gRPC server starts from — the
f6e592e:crates/blit-core/src/remote/grpc_server.rs:36:    Server::builder()

exec
/usr/bin/zsh -lc "git grep -n \"#\\[test\\]\\|#\\[tokio::test\\]\" f6e592e"'^ -- crates/blit-cli/tests/remote_remote.rs crates/blit-cli/tests/remote_pull_mirror.rs crates/blit-cli/tests/remote_checksum_negotiation.rs crates/blit-cli/tests/remote_tcp_fallback.rs crates/blit-cli/tests/jobs_lifecycle.rs crates/blit-cli/tests/readonly_enforcement.rs crates/blit-cli/tests/cli_arg_safety_gates.rs crates/blit-cli/tests/diagnostics_dump.rs crates/blit-cli/tests/local_move_semantics.rs crates/blit-cli/tests/single_file_copy.rs crates/blit-core/tests/pull_sync_with_spec_wire.rs crates/blit-core/src/remote/grpc_server.rs | wc -l' in /home/michael/dev/Blit
 succeeded in 0ms:
55

exec
/usr/bin/zsh -lc "git grep -n \"#\\[test\\]\\|#\\[tokio::test\\]\" f6e592e -- crates/blit-cli/tests/remote_remote.rs crates/blit-cli/tests/remote_pull_mirror.rs crates/blit-cli/tests/remote_checksum_negotiation.rs crates/blit-cli/tests/remote_tcp_fallback.rs crates/blit-cli/tests/jobs_lifecycle.rs crates/blit-cli/tests/readonly_enforcement.rs crates/blit-cli/tests/cli_arg_safety_gates.rs crates/blit-cli/tests/diagnostics_dump.rs crates/blit-cli/tests/local_move_semantics.rs crates/blit-cli/tests/single_file_copy.rs crates/blit-core/tests/pull_sync_with_spec_wire.rs crates/blit-core/src/remote/grpc_server.rs | wc -l" in /home/michael/dev/Blit
 succeeded in 0ms:
56

exec
/usr/bin/zsh -lc "rg -n \"pub trait Blit|trait Blit\" crates/blit-core/src/generated.rs crates/blit-core/src/generated -g '*.rs'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rg -n \"struct BlitServer|impl<T: Blit\" crates/blit-core/src/generated.rs crates/blit-core/src/generated -g '*.rs'" in /home/michael/dev/Blit
 exited 2 in 0ms:
rg: crates/blit-core/src/generated.rs: No such file or directory (os error 2)
rg: crates/blit-core/src/generated: No such file or directory (os error 2)

 exited 2 in 0ms:
rg: crates/blit-core/src/generated.rs: No such file or directory (os error 2)
rg: crates/blit-core/src/generated: No such file or directory (os error 2)

exec
/usr/bin/zsh -lc "rg -n \"mod generated|pub mod generated\" crates/blit-core/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/lib.rs:53:pub mod generated {

codex
The test-count check matches the commit message for the touched area: 55 before, 56 after, with the new keepalive pin. I’m checking the generated tonic trait bounds and a couple of edge cases around the shared fake-server helper.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/lib.rs | sed -n '45,75p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    45	            dry_run: false,
    46	            checksum: None,
    47	            resume: false,
    48	            null_sink: false,
    49	        }
    50	    }
    51	}
    52	
    53	pub mod generated {
    54	    tonic::include_proto!("blit.v2");
    55	}

exec
/usr/bin/zsh -lc "rg -n \"pub trait Blit|trait Blit|struct BlitServer|impl<T: Blit\" target/debug/build -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2000:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2190:    pub struct BlitServer<T> {
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2364:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2407:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2450:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2493:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2719:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2762:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-b0fbadc9ccacb4d6/out/blit.v2.rs:2805:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2000:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2190:    pub struct BlitServer<T> {
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2364:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2407:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2450:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2493:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2719:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2762:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-83e5f71914b8e3ea/out/blit.v2.rs:2805:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2000:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2190:    pub struct BlitServer<T> {
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2364:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2407:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2450:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2493:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2719:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2762:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-eb69d9d4c5f20647/out/blit.v2.rs:2805:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2000:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2190:    pub struct BlitServer<T> {
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2364:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2407:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2450:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2493:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2719:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2762:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-117c1e6436fd742d/out/blit.v2.rs:2805:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2000:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2190:    pub struct BlitServer<T> {
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2364:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2407:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2450:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2493:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2719:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2762:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-79aa0197aa3b9aff/out/blit.v2.rs:2805:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-94f4620cf6a2ebb6/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-d00c24171bdcc247/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-74031fa155e0c1f3/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-02c595a2d26a766b/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-564f73876aec37e1/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-434df1a860d52827/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-180f2ff88746d04c/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-4c532c07aa606a45/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2023:    pub trait Blit: std::marker::Send + std::marker::Sync + 'static {
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2220:    pub struct BlitServer<T> {
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2440:                    impl<T: Blit> tonic::server::UnaryService<super::ListRequest>
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2483:                    impl<T: Blit> tonic::server::UnaryService<super::PurgeRequest>
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2526:                    impl<T: Blit> tonic::server::UnaryService<super::CompletionRequest>
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2569:                    impl<T: Blit> tonic::server::UnaryService<super::ListModulesRequest>
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2795:                    impl<T: Blit> tonic::server::UnaryService<super::GetStateRequest>
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2838:                    impl<T: Blit> tonic::server::UnaryService<super::CancelJobRequest>
target/debug/build/blit-core-4e62d1b63f2043e6/out/blit.v2.rs:2881:                    impl<T: Blit> tonic::server::UnaryService<super::ClearRecentRequest>

exec
/usr/bin/zsh -lc "git show --format= --find-renames f6e592e -- .review/findings/w9-3-test-harness-builder.md | sed -n '1,260p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rg -n \"pub trait Blit|struct BlitServer|impl<T: Blit\" target -g 'blit.rs'" in /home/michael/dev/Blit
 exited 1 in 0ms:
 succeeded in 0ms:
diff --git a/.review/findings/w9-3-test-harness-builder.md b/.review/findings/w9-3-test-harness-builder.md
new file mode 100644
index 0000000..eec0fc9
--- /dev/null
+++ b/.review/findings/w9-3-test-harness-builder.md
@@ -0,0 +1,167 @@
+# w9-3-test-harness-builder — one daemon-spawn harness, one cli_bin, OnceLock build, fake-server keepalive parity
+
+**Branch**: `master` (owner-authorized branchless loop, D-2026-06-20-6)
+**Commit**: _(filled at commit time)_
+**Source findings**: tests-five-daemon-harness-clones,
+tests-per-test-cargo-build-subprocess, duplication-cli-test-daemon-harness,
+tests-fake-server-config-skew — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`;
+slice spec W9.3 in `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md`.
+
+## What
+
+The daemon-spawn harness (config structs + port pick + `cargo build` +
+spawn + readiness poll) existed in **seven** copies at HEAD, not the five
+the 2026-06-11 audit counted — w9-4 (`readonly_enforcement.rs`) and w9-5
+(`jobs_lifecycle.rs`) each added another clone *because* the shared
+harness couldn't express delegation or a second daemon, proving the
+finding's "the next one will miss at least one" prediction twice over.
+`cli_bin()` was pasted in 5 more files, `run_with_timeout` in 7,
+`ChildGuard` in 4, and all five in-process fake tonic servers (not
+three: two in `pull_sync_with_spec_wire.rs`, one in `jobs_lifecycle.rs`,
+two in `remote_remote.rs`) ran bare `Server::builder()` while production
+sets HTTP/2 keepalive 30s/20s.
+
+Now: `tests/common/mod.rs` is the single owner (builder + spawn
+primitives + shared helpers + fake-server scaffold), the daemon build
+runs once per test binary behind a `OnceLock`, and every gRPC server —
+production and fakes — starts from one shared production-shaped builder
+in blit-core.
+
+## Approach
+
+- **`TestContext::builder()`** (`crates/blit-cli/tests/common/mod.rs`)
+  with the knobs the clones existed for: `.read_only(bool)` (w9-4),
+  `.delegation(bool)` (writes the `[delegation]` table,
+  `allowed_source_hosts = ["127.0.0.1"]`, IP-form per the production
+  SSRF rule), `.extra_daemon_args(...)` (`--no-server-checksums`,
+  `--force-grpc-data`). `TestContext::new()` / `new_read_only()` kept
+  their exact signatures — the 13 pre-existing consumer files needed
+  zero edits for construction.
+- **Dual-daemon support**: `spawn_daemon(workspace, name, module_dir,
+  opts) -> SpawnedDaemon` primitive + `TestContext::spawn_second_daemon`.
+  `remote_remote::DualDaemonContext`, `jobs_lifecycle::DelegationContext`
+  and the readonly delegated-pull test are now thin wrappers over it;
+  their private config structs / spawn fns / `wait_for_port` /
+  `binary_paths` clones are deleted.
+- **Config superset**: shared `ModuleSection` gained
+  `delegation_allowed: bool`, serialized explicitly `true` — the daemon
+  defaults it to `true` when absent (`runtime.rs::default_true`),
+  verified before choosing, so pre-w9-3 configs that omitted it are
+  behavior-identical. `DaemonConfig` gained
+  `delegation: Option<DelegationSection>` (skipped when `None`).
+- **OnceLock build**: `ensure_daemon_built()` wraps the
+  `cargo build -p blit-daemon` step (with the `--target` triple handling
+  that `remote_remote.rs`'s clone had *dropped* — that drift is now
+  structurally impossible). R16-F1's property is preserved per process:
+  every spawn path calls it, so no test binary depends on suite
+  ordering; ~75 nested cargo invocations per full run become ≤1 per test
+  binary. This is also the main daemon-spawn load-flakiness fix: the
+  concurrent nested cargo builds all contended for the build-dir flock
+  while readiness polls (5s budget) ticked. Bonus: the
+  `remote_tcp_fallback` w4-2 helper and the jobs/readonly delegation
+  spawns previously ran **no** build at all (relied on ordering) — they
+  now get the guarantee.
+- **Keepalive parity**: new `blit_core::remote::grpc_server` owns
+  `HTTP2_KEEPALIVE_INTERVAL`/`_TIMEOUT` (30s/20s, audit-1 owner decision
+  2026-05-23, rationale moved there) and
+  `production_server_builder()`. Consumers: daemon `main.rs`,
+  `common::spawn_fake_blit_server` (generic over the service impl —
+  collapses the thrice-cloned thread+runtime+oneshot-shutdown scaffold),
+  and both `pull_sync_with_spec_wire.rs` spy servers. No bare
+  `Server::builder()` remains anywhere in the workspace.
+- **Port-collision race fixed (the daemon-spawn load-flakiness)**:
+  `pick_unused_port` binds `:0` and drops the probe listener before the
+  daemon binds, so two parallel tests in one binary could be handed the
+  same port — the losing daemon exits on "address in use" and its test
+  then silently talks to the *winner's* daemon (wrong/empty module).
+  The per-test `cargo build` used to serialize bring-ups and mostly
+  hide this; removing it made the race routine (caught red-handed
+  during this slice's own validation: `test_admin_find` got an empty
+  listing from another test's daemon). Two-layer fix: (a)
+  `pick_unused_port` keeps a process-global claimed-port set — cargo
+  runs test binaries sequentially, so per-process uniqueness is the
+  needed scope; (b) `spawn_daemon`'s readiness poll checks
+  `child.try_wait()` each tick and panics with the real reason if the
+  daemon exits during startup (external port steal, config rejection)
+  instead of timing out generically or proceeding against a foreign
+  daemon.
+- **stderr policy unified to `Stdio::null()`**: the old shared harness
+  piped stderr "for debugging" but nothing ever read it — zero
+  diagnostics in practice plus a latent pipe-buffer deadlock once a
+  chatty daemon wrote 64 KiB (four clones already used null). Real
+  capture (drain thread, dump on readiness failure) stays w9-6's slice
+  (tests-harness-stderr-blackhole).
+- Serialize-only `#[serde(default)]` attrs (meaningless without
+  `Deserialize`) dropped from the config structs; `ChildGuard` gained
+  `terminate()` (the pull-mirror/tcp-fallback clones had it, common
+  didn't); `run_with_timeout`'s
+  `#[cfg_attr(windows, allow(dead_code))]` is superseded by the
+  module-level `#![allow(dead_code)]` (each binary uses a different
+  subset of the shared harness — documented in the file header).
+
+## Files changed
+
+- `crates/blit-cli/tests/common/mod.rs` — rewritten (229 → ~450 lines):
+  builder, spawn primitives, OnceLock build, fake-server scaffold.
+- `crates/blit-core/src/remote/grpc_server.rs` — **new**: keepalive
+  constants + `production_server_builder()` + value-pin test.
+- `crates/blit-core/src/remote/mod.rs` — module declaration.
+- `crates/blit-daemon/src/main.rs` — serves from the shared builder.
+- `crates/blit-core/tests/pull_sync_with_spec_wire.rs` — both spy
+  servers production-shaped.
+- Ported off private clones: `remote_remote.rs`, `remote_pull_mirror.rs`,
+  `remote_checksum_negotiation.rs`, `remote_tcp_fallback.rs`,
+  `jobs_lifecycle.rs`, `readonly_enforcement.rs`.
+- Ported onto shared `cli_bin`/`run_with_timeout`: `single_file_copy.rs`,
+  `local_move_semantics.rs`, `diagnostics_dump.rs`,
+  `cli_arg_safety_gates.rs`.
+- Net: −1,251 lines of test-tree duplication (+55-line shared module).
+
+## Tests
+
+- No test deleted or weakened: every `#[test]` in the ten touched test
+  files survives on the shared harness; assertions untouched except
+  harness plumbing. The 270k-file `#[ignore]` w4-2 acceptance test stays
+  ignored (still 2 ignored workspace-wide).
+- +1 test: `grpc_server::keepalive_values_match_owner_decision` pins
+  30s/20s to the audit-1 owner decision (mutation-verified: interval
+  30→31 fails it, restored passes). The structural half of the parity
+  guarantee is by construction — no bare `Server::builder()` remains
+  anywhere in the workspace (grep-verified).
+- Workspace totals, measured A/B with one aggregation (sum of every
+  `test result:` line, doc-test suites included, via `git stash`):
+  HEAD `3d8326b` = 1478/0/2 across 37 suites → this slice = 1479/0/2
+  across 37 suites. Exactly +1, nothing dropped; per-file `#[test]`
+  counts in all ten touched test files are identical to HEAD.
+  (STATE's recorded "1479" baseline for design-3 came from a different
+  aggregation — off by one from this method against the same tree; the
+  same-method A/B above is the count-never-drops evidence.)
+- Full suite run twice post-fix (plus `admin_verbs` ×10 alone, the
+  binary that flaked): all green — the port-collision flake that
+  surfaced mid-slice (see Approach) did not recur.
+
+## Known gaps
+
+- Daemon stderr is now uniformly discarded (was: piped-but-never-read in
+  the shared harness, null in 4 of 6 clones). Capture-and-dump on
+  readiness failure is w9-6 (tests-harness-stderr-blackhole), unblocked
+  by this consolidation — it now needs exactly one edit site.
+- `ModuleSection.delegation_allowed` is hardcoded `true` (the daemon
+  default); no test exercises `false`. Deliberate: knobs are added when
+  a consumer exists (noted in the struct docs).
+- The readiness poll stays 50 × 100 ms; if daemon-spawn e2e flakiness
+  persists under full-parallel runs after the port registry + OnceLock
+  build landed, bumping the budget (or w9-6's stderr capture showing
+  *why* startup lagged) is the next lever.
+- The claimed-port set only grows (ports are never released back);
+  bounded by tests-per-binary (≤ ~70), irrelevant against the ~28k
+  ephemeral range. A collision with an unrelated *system* process
+  between probe and bind remains possible — that residual case is what
+  the child-death panic reports honestly.
+- Client-side gRPC channels still set no HTTP/2 keepalive anywhere —
+  out of scope here (server-side parity only), noted for the transport
+  backlog.
+- Windows run of the touched integration suites not executed locally
+  (Linux host); windows-latest CI on the next push covers it. No
+  platform-specific logic changed — the `cfg!(windows)` binary-name
+  branches moved verbatim.

codex
No compile-only red flags from the generic fake-server helper: the generated `Blit` trait is already `Send + Sync + 'static`. I’m checking all current `TestContext` consumers now, because the shared config shape affects tests outside the changed diff too.
exec
/usr/bin/zsh -lc "rg -n \"TestContext|new_read_only|builder\\(\\)|spawn_second_daemon|DaemonOptions|spawn_daemon\\(\" crates/blit-cli/tests -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-cli/tests/remote_tcp_fallback.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_tcp_fallback.rs:9:fn forced_grpc_ctx() -> TestContext {
crates/blit-cli/tests/remote_tcp_fallback.rs:10:    TestContext::builder()
crates/blit-cli/tests/remote_remote.rs:9:use common::{run_with_timeout, spawn_fake_blit_server, DaemonOptions, SpawnedDaemon, TestContext};
crates/blit-cli/tests/remote_remote.rs:15:    _ctx: TestContext,
crates/blit-cli/tests/remote_remote.rs:28:        let ctx = TestContext::new();
crates/blit-cli/tests/remote_remote.rs:29:        let daemon_b = ctx.spawn_second_daemon(
crates/blit-cli/tests/remote_remote.rs:31:            &DaemonOptions {
crates/blit-cli/tests/remote_pull_mirror.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_pull_mirror.rs:10:    let mut ctx = TestContext::new();
crates/blit-cli/tests/remote_pull_mirror.rs:59:    let mut ctx = TestContext::new();
crates/blit-cli/tests/remote_pull_mirror.rs:110:    let mut ctx = TestContext::new();
crates/blit-cli/tests/remote_checksum_negotiation.rs:17:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_checksum_negotiation.rs:21:fn spawn_daemon_harness(extra_daemon_args: &[&str]) -> TestContext {
crates/blit-cli/tests/remote_checksum_negotiation.rs:22:    TestContext::builder()
crates/blit-cli/tests/readonly_enforcement.rs:19:use common::{run_with_timeout, DaemonOptions, TestContext};
crates/blit-cli/tests/readonly_enforcement.rs:37:    let ctx = TestContext::new_read_only();
crates/blit-cli/tests/readonly_enforcement.rs:72:    let ctx = TestContext::new_read_only();
crates/blit-cli/tests/readonly_enforcement.rs:102:    let ctx = TestContext::new_read_only();
crates/blit-cli/tests/readonly_enforcement.rs:138:    let ctx = TestContext::new();
crates/blit-cli/tests/readonly_enforcement.rs:141:    let dst = ctx.spawn_second_daemon(
crates/blit-cli/tests/readonly_enforcement.rs:143:        &DaemonOptions {
crates/blit-cli/tests/jobs_lifecycle.rs:25:use common::{run_with_timeout, spawn_fake_blit_server, DaemonOptions, SpawnedDaemon, TestContext};
crates/blit-cli/tests/jobs_lifecycle.rs:33:    let ctx = TestContext::new();
crates/blit-cli/tests/jobs_lifecycle.rs:65:    let ctx = TestContext::new();
crates/blit-cli/tests/jobs_lifecycle.rs:86:    let ctx = TestContext::new();
crates/blit-cli/tests/jobs_lifecycle.rs:113:    _ctx: TestContext,
crates/blit-cli/tests/jobs_lifecycle.rs:129:        let ctx = TestContext::new();
crates/blit-cli/tests/jobs_lifecycle.rs:130:        let dst = ctx.spawn_second_daemon(
crates/blit-cli/tests/jobs_lifecycle.rs:132:            &DaemonOptions {
crates/blit-cli/tests/jobs_lifecycle.rs:153:        let ctx = TestContext::builder().delegation(true).build();
crates/blit-cli/tests/remote_regression.rs:17:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_regression.rs:31:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_regression.rs:96:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_regression.rs:170:    let ctx = TestContext::new();
crates/blit-cli/tests/f2_chroot_containment.rs:26:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/f2_chroot_containment.rs:29:fn place_escape_symlink(ctx: &TestContext) -> std::path::PathBuf {
crates/blit-cli/tests/f2_chroot_containment.rs:46:    let ctx = TestContext::new();
crates/blit-cli/tests/f2_chroot_containment.rs:109:    let ctx = TestContext::new();
crates/blit-cli/tests/f2_chroot_containment.rs:170:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_parity.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_parity.rs:10:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_parity.rs:46:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_parity.rs:97:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_parity.rs:151:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_parity.rs:191:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_parity.rs:232:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_transfer_edges.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_transfer_edges.rs:11:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_transfer_edges.rs:56:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_transfer_edges.rs:97:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_transfer_edges.rs:134:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_transfer_edges.rs:178:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_transfer_edges.rs:203:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_resume.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_resume.rs:13:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_resume.rs:54:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_resume.rs:89:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_push_single_file.rs:12:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_push_single_file.rs:16:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_push_single_file.rs:52:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_move.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_move.rs:10:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_move.rs:48:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_move.rs:106:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_move.rs:151:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_push_mirror_safety.rs:21:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_push_mirror_safety.rs:31:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_push_mirror_safety.rs:109:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/blit_utils.rs:12:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:39:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:62:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:86:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:112:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:136:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:162:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:191:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:220:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:243:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:271:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:295:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:321:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:351:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:375:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:403:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:423:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:442:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:477:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:509:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:535:    let ctx = TestContext::new();
crates/blit-cli/tests/blit_utils.rs:561:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:6:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/admin_verbs.rs:10:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:38:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:75:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:109:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:137:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:165:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:188:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:217:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:239:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:287:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:315:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:361:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:398:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:437:    let ctx = TestContext::new();
crates/blit-cli/tests/admin_verbs.rs:464:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_pull_subpath.rs:23:use common::{run_with_timeout, TestContext};
crates/blit-cli/tests/remote_pull_subpath.rs:25:fn setup_module(ctx: &TestContext) {
crates/blit-cli/tests/remote_pull_subpath.rs:32:fn pull(ctx: &TestContext, remote_path: &str, local_dest: &std::path::Path) {
crates/blit-cli/tests/remote_pull_subpath.rs:52:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_pull_subpath.rs:72:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_pull_subpath.rs:87:    let ctx = TestContext::new();
crates/blit-cli/tests/remote_pull_subpath.rs:109:    let ctx = TestContext::new();
crates/blit-cli/tests/common/mod.rs:104:/// `TestContext::new()` ran its own nested `cargo build` (~75 per
crates/blit-cli/tests/common/mod.rs:146:// Daemon spawn primitive + TestContext builder.
crates/blit-cli/tests/common/mod.rs:194:pub struct DaemonOptions {
crates/blit-cli/tests/common/mod.rs:216:/// `TestContext` routes through this; dual-daemon tests call it (via
crates/blit-cli/tests/common/mod.rs:217:/// `TestContext::spawn_second_daemon`) for their second daemon.
crates/blit-cli/tests/common/mod.rs:218:pub fn spawn_daemon(
crates/blit-cli/tests/common/mod.rs:222:    opts: &DaemonOptions,
crates/blit-cli/tests/common/mod.rs:304:pub struct TestContext {
crates/blit-cli/tests/common/mod.rs:315:pub struct TestContextBuilder {
crates/blit-cli/tests/common/mod.rs:316:    opts: DaemonOptions,
crates/blit-cli/tests/common/mod.rs:319:impl TestContextBuilder {
crates/blit-cli/tests/common/mod.rs:339:    pub fn build(self) -> TestContext {
crates/blit-cli/tests/common/mod.rs:346:        let spawned = spawn_daemon(&workspace, "blitd", &workspace.join("module"), &self.opts);
crates/blit-cli/tests/common/mod.rs:348:        TestContext {
crates/blit-cli/tests/common/mod.rs:360:impl TestContext {
crates/blit-cli/tests/common/mod.rs:361:    pub fn builder() -> TestContextBuilder {
crates/blit-cli/tests/common/mod.rs:362:        TestContextBuilder {
crates/blit-cli/tests/common/mod.rs:363:            opts: DaemonOptions::default(),
crates/blit-cli/tests/common/mod.rs:368:        Self::builder().build()
crates/blit-cli/tests/common/mod.rs:375:    pub fn new_read_only() -> Self {
crates/blit-cli/tests/common/mod.rs:376:        Self::builder().read_only(true).build()
crates/blit-cli/tests/common/mod.rs:382:    pub fn spawn_second_daemon(&self, name: &str, opts: &DaemonOptions) -> SpawnedDaemon {
crates/blit-cli/tests/common/mod.rs:383:        spawn_daemon(
crates/blit-cli/tests/common/mod.rs:493:            blit_core::remote::grpc_server::production_server_builder()

exec
/usr/bin/zsh -lc 'git show f6e592e''^:crates/blit-cli/tests/common/mod.rs | nl -ba | sed -n '"'1,280p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::fs;
     2	use std::net::{TcpListener, TcpStream};
     3	use std::path::PathBuf;
     4	use std::process::{Command, Stdio};
     5	use std::thread;
     6	use std::time::Duration;
     7	use tempfile::tempdir;
     8	
     9	use serde::Serialize;
    10	use wait_timeout::ChildExt;
    11	
    12	#[derive(Serialize)]
    13	pub struct DaemonConfig {
    14	    pub daemon: DaemonSection,
    15	    #[serde(rename = "module")]
    16	    pub modules: Vec<ModuleSection>,
    17	}
    18	
    19	#[derive(Serialize)]
    20	pub struct DaemonSection {
    21	    pub bind: String,
    22	    pub port: u16,
    23	    pub no_mdns: bool,
    24	}
    25	
    26	#[derive(Serialize)]
    27	pub struct ModuleSection {
    28	    pub name: String,
    29	    pub path: PathBuf,
    30	    #[serde(default)]
    31	    pub comment: Option<String>,
    32	    #[serde(default)]
    33	    pub read_only: bool,
    34	}
    35	
    36	pub fn pick_unused_port() -> u16 {
    37	    TcpListener::bind(("127.0.0.1", 0))
    38	        .expect("bind probe listener")
    39	        .local_addr()
    40	        .expect("listener addr")
    41	        .port()
    42	}
    43	
    44	#[allow(dead_code)]
    45	pub struct TestContext {
    46	    pub _work: tempfile::TempDir,
    47	    pub workspace: PathBuf,
    48	    pub daemon_port: u16,
    49	    pub daemon: ChildGuard,
    50	    pub cli_bin: PathBuf,
    51	    pub config_dir: PathBuf,
    52	    pub module_dir: PathBuf,
    53	}
    54	
    55	impl TestContext {
    56	    #[allow(dead_code)] // each test binary uses one of the two constructors
    57	    pub fn new() -> Self {
    58	        Self::with_read_only(false)
    59	    }
    60	
    61	    /// w9-4: same daemon + module, but the module is exported
    62	    /// `read_only: true` so tests can exercise the three write gates
    63	    /// (push, purge, delegated pull). Before this knob existed no
    64	    /// test config in the workspace could express a read-only module.
    65	    #[allow(dead_code)] // not every test binary uses the read-only harness
    66	    pub fn new_read_only() -> Self {
    67	        Self::with_read_only(true)
    68	    }
    69	
    70	    fn with_read_only(read_only: bool) -> Self {
    71	        let work = tempdir().expect("tempdir");
    72	        let workspace = work.path().to_path_buf();
    73	
    74	        let module_dir = workspace.join("module");
    75	        fs::create_dir_all(&module_dir).expect("module dir");
    76	
    77	        let config_dir = workspace.join("cli-config");
    78	        fs::create_dir_all(&config_dir).expect("cli config");
    79	
    80	        let port = pick_unused_port();
    81	
    82	        let config = DaemonConfig {
    83	            daemon: DaemonSection {
    84	                bind: "127.0.0.1".into(),
    85	                port,
    86	                no_mdns: true,
    87	            },
    88	            modules: vec![ModuleSection {
    89	                name: "test".into(),
    90	                path: module_dir.clone(),
    91	                comment: None,
    92	                read_only,
    93	            }],
    94	        };
    95	
    96	        let config_path = workspace.join("blitd.toml");
    97	        let toml = toml::to_string(&config).expect("serialize config");
    98	        fs::write(&config_path, toml).expect("write config");
    99	
   100	        let exe_path = std::env::current_exe().expect("current_exe");
   101	        let deps_dir = exe_path.parent().expect("test binary directory");
   102	        let bin_dir = deps_dir
   103	            .parent()
   104	            .expect("deps parent directory")
   105	            .to_path_buf();
   106	
   107	        let cli_bin = {
   108	            let name = if cfg!(windows) { "blit.exe" } else { "blit" };
   109	            bin_dir.join(name)
   110	        };
   111	        let daemon_bin = {
   112	            let name = if cfg!(windows) {
   113	                "blit-daemon.exe"
   114	            } else {
   115	                "blit-daemon"
   116	            };
   117	            bin_dir.join(name)
   118	        };
   119	        let maybe_target = bin_dir
   120	            .parent()
   121	            .and_then(|p| p.file_name())
   122	            .map(|component| component.to_string_lossy().to_string());
   123	
   124	        // Ensure daemon is built
   125	        let mut build = Command::new("cargo");
   126	        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
   127	            .join("../..")
   128	            .canonicalize()
   129	            .expect("workspace root");
   130	        build.current_dir(workspace_root);
   131	        build
   132	            .arg("build")
   133	            .arg("-p")
   134	            .arg("blit-daemon")
   135	            .arg("--bin")
   136	            .arg("blit-daemon");
   137	        if let Some(triple) = maybe_target {
   138	            if triple != "target" {
   139	                build.arg("--target").arg(triple);
   140	            }
   141	        }
   142	        let output = build.output().expect("invoke cargo build for blit-daemon");
   143	        assert!(
   144	            output.status.success(),
   145	            "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
   146	            String::from_utf8_lossy(&output.stdout),
   147	            String::from_utf8_lossy(&output.stderr)
   148	        );
   149	
   150	        let daemon_child = Command::new(&daemon_bin)
   151	            .arg("--config")
   152	            .arg(&config_path)
   153	            .arg("--bind")
   154	            .arg("127.0.0.1")
   155	            .arg("--port")
   156	            .arg(port.to_string())
   157	            .stdin(Stdio::null())
   158	            .stdout(Stdio::null())
   159	            .stderr(Stdio::piped()) // Capture stderr for debugging
   160	            .spawn()
   161	            .expect("spawn daemon");
   162	        let daemon = ChildGuard::new(daemon_child);
   163	
   164	        let mut ready = false;
   165	        for _ in 0..50 {
   166	            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
   167	                ready = true;
   168	                break;
   169	            }
   170	            thread::sleep(Duration::from_millis(100));
   171	        }
   172	        assert!(ready, "daemon failed to listen on {port}");
   173	
   174	        Self {
   175	            _work: work,
   176	            workspace,
   177	            daemon_port: port,
   178	            daemon,
   179	            cli_bin,
   180	            config_dir,
   181	            module_dir,
   182	        }
   183	    }
   184	}
   185	
   186	// Shared across many test binaries; the ones whose tests are all
   187	// unix-gated compile this helper unused on Windows.
   188	#[cfg_attr(windows, allow(dead_code))]
   189	pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
   190	    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
   191	    let mut child = cmd.spawn().expect("spawn command");
   192	
   193	    match child.wait_timeout(timeout).expect("wait for process") {
   194	        Some(_status) => child
   195	            .wait_with_output()
   196	            .expect("collect command output after completion"),
   197	        None => {
   198	            let _ = child.kill();
   199	            let output = child
   200	                .wait_with_output()
   201	                .expect("collect output after killing command");
   202	            panic!(
   203	                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
   204	                timeout,
   205	                String::from_utf8_lossy(&output.stdout),
   206	                String::from_utf8_lossy(&output.stderr)
   207	            );
   208	        }
   209	    }
   210	}
   211	
   212	pub struct ChildGuard {
   213	    pub child: Option<std::process::Child>,
   214	}
   215	
   216	impl ChildGuard {
   217	    pub fn new(child: std::process::Child) -> Self {
   218	        Self { child: Some(child) }
   219	    }
   220	}
   221	
   222	impl Drop for ChildGuard {
   223	    fn drop(&mut self) {
   224	        if let Some(mut child) = self.child.take() {
   225	            let _ = child.kill();
   226	            let _ = child.wait();
   227	        }
   228	    }
   229	}

exec
/usr/bin/zsh -lc 'git show f6e592e''^:crates/blit-cli/tests/jobs_lifecycle.rs | nl -ba | sed -n '"'1,520p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! w9-5: jobs/detach lifecycle e2e (tests-jobs-lifecycle-no-e2e).
     2	//!
     3	//! The detached-job lifecycle — `--detach` output, `jobs list`,
     4	//! `jobs watch` to a terminal state, and the `jobs cancel` exit-code
     5	//! contract (`docs/plan/TUI_DESIGN.md` §6.5: 0 cancelled / 1 not
     6	//! found / 2 unsupported) — previously ran in zero tests; coverage
     7	//! stopped at formatting/exit-code unit tests in `jobs.rs`. This file
     8	//! is the regression net W4 needed before changing cancellation
     9	//! (that change has since landed: D-2026-07-04-3 / w4-5 flipped
    10	//! CancelJob dispatch on for attached push/pull_sync, so exit 2 no
    11	//! longer occurs for those kinds; the 0/1/2 mapping is unchanged).
    12	//!
    13	//! Watch exit codes (see `run_jobs_watch`): 0 finished-ok,
    14	//! 1 finished-failed, 2 not-found, 3 timeout-while-active.
    15	//!
    16	//! The dual-daemon delegation harness mirrors `remote_remote.rs`
    17	//! (consolidation of the harness clones is w9-3's job).
    18	
    19	use std::fs;
    20	use std::net::TcpStream;
    21	use std::path::{Path, PathBuf};
    22	use std::process::{Command, Stdio};
    23	use std::thread;
    24	use std::time::Duration;
    25	
    26	use serde::Serialize;
    27	use tempfile::tempdir;
    28	
    29	mod common;
    30	use common::{run_with_timeout, ChildGuard, TestContext};
    31	
    32	// ---------------------------------------------------------------
    33	// Single-daemon cases: list shape, cancel/watch unknown-id codes.
    34	// ---------------------------------------------------------------
    35	
    36	#[test]
    37	fn jobs_list_on_idle_daemon_exits_zero_with_empty_active() {
    38	    let ctx = TestContext::new();
    39	    let remote = format!("127.0.0.1:{}", ctx.daemon_port);
    40	
    41	    let mut cmd = Command::new(&ctx.cli_bin);
    42	    cmd.arg("jobs").arg("list").arg(&remote).arg("--json");
    43	    let output = run_with_timeout(cmd, Duration::from_secs(30));
    44	
    45	    assert_eq!(
    46	        output.status.code(),
    47	        Some(0),
    48	        "jobs list must exit 0 once the RPC returns cleanly\nstdout:\n{}\nstderr:\n{}",
    49	        String::from_utf8_lossy(&output.stdout),
    50	        String::from_utf8_lossy(&output.stderr)
    51	    );
    52	    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
    53	        panic!(
    54	            "jobs list --json must emit valid JSON ({err})\nstdout:\n{}",
    55	            String::from_utf8_lossy(&output.stdout)
    56	        )
    57	    });
    58	    let active = json
    59	        .get("active")
    60	        .and_then(|a| a.as_array())
    61	        .expect("JSON must carry an `active` array");
    62	    assert!(
    63	        active.is_empty(),
    64	        "an idle daemon must report no active transfers, got: {active:?}"
    65	    );
    66	}
    67	
    68	#[test]
    69	fn jobs_cancel_unknown_id_exits_one() {
    70	    let ctx = TestContext::new();
    71	    let remote = format!("127.0.0.1:{}", ctx.daemon_port);
    72	
    73	    let mut cmd = Command::new(&ctx.cli_bin);
    74	    cmd.arg("jobs")
    75	        .arg("cancel")
    76	        .arg(&remote)
    77	        .arg("no-such-transfer-id");
    78	    let output = run_with_timeout(cmd, Duration::from_secs(30));
    79	
    80	    assert_eq!(
    81	        output.status.code(),
    82	        Some(1),
    83	        "cancel of an unknown id must exit 1 (NotFound)\nstdout:\n{}\nstderr:\n{}",
    84	        String::from_utf8_lossy(&output.stdout),
    85	        String::from_utf8_lossy(&output.stderr)
    86	    );
    87	}
    88	
    89	#[test]
    90	fn jobs_watch_unknown_id_exits_two() {
    91	    let ctx = TestContext::new();
    92	    let remote = format!("127.0.0.1:{}", ctx.daemon_port);
    93	
    94	    let mut cmd = Command::new(&ctx.cli_bin);
    95	    cmd.arg("jobs")
    96	        .arg("watch")
    97	        .arg(&remote)
    98	        .arg("no-such-transfer-id")
    99	        .arg("--timeout-secs")
   100	        .arg("15");
   101	    let output = run_with_timeout(cmd, Duration::from_secs(30));
   102	
   103	    assert_eq!(
   104	        output.status.code(),
   105	        Some(2),
   106	        "watch of an unknown id must exit 2 (NotFound)\nstdout:\n{}\nstderr:\n{}",
   107	        String::from_utf8_lossy(&output.stdout),
   108	        String::from_utf8_lossy(&output.stderr)
   109	    );
   110	}
   111	
   112	// ---------------------------------------------------------------
   113	// Delegation harness (dual daemon / fake source), mirroring
   114	// remote_remote.rs.
   115	// ---------------------------------------------------------------
   116	
   117	#[derive(Serialize)]
   118	struct DaemonConfig {
   119	    daemon: DaemonSection,
   120	    #[serde(rename = "module")]
   121	    modules: Vec<ModuleSection>,
   122	    #[serde(skip_serializing_if = "Option::is_none")]
   123	    delegation: Option<DelegationSection>,
   124	}
   125	
   126	#[derive(Serialize)]
   127	struct DaemonSection {
   128	    bind: String,
   129	    port: u16,
   130	    no_mdns: bool,
   131	}
   132	
   133	#[derive(Serialize)]
   134	struct ModuleSection {
   135	    name: String,
   136	    path: PathBuf,
   137	    comment: Option<String>,
   138	    read_only: bool,
   139	    delegation_allowed: bool,
   140	}
   141	
   142	#[derive(Serialize)]
   143	struct DelegationSection {
   144	    allow_delegated_pull: bool,
   145	    allowed_source_hosts: Vec<String>,
   146	}
   147	
   148	fn binary_paths() -> (PathBuf, PathBuf) {
   149	    let exe_path = std::env::current_exe().expect("current_exe");
   150	    let deps_dir = exe_path.parent().expect("test binary directory");
   151	    let bin_dir = deps_dir
   152	        .parent()
   153	        .expect("deps parent directory")
   154	        .to_path_buf();
   155	    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
   156	    let daemon_bin = bin_dir.join(if cfg!(windows) {
   157	        "blit-daemon.exe"
   158	    } else {
   159	        "blit-daemon"
   160	    });
   161	    (cli_bin, daemon_bin)
   162	}
   163	
   164	fn wait_for_port(port: u16, label: &str) {
   165	    let mut ready = false;
   166	    for _ in 0..50 {
   167	        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
   168	            ready = true;
   169	            break;
   170	        }
   171	        thread::sleep(Duration::from_millis(100));
   172	    }
   173	    assert!(ready, "{label} failed to listen on {port}");
   174	}
   175	
   176	fn spawn_daemon(
   177	    workspace: &Path,
   178	    bin: &Path,
   179	    port: u16,
   180	    name: &str,
   181	    module_path: &Path,
   182	    delegation_enabled: bool,
   183	) -> ChildGuard {
   184	    let config = DaemonConfig {
   185	        daemon: DaemonSection {
   186	            bind: "127.0.0.1".into(),
   187	            port,
   188	            no_mdns: true,
   189	        },
   190	        modules: vec![ModuleSection {
   191	            name: "test".into(),
   192	            path: module_path.to_path_buf(),
   193	            comment: None,
   194	            read_only: false,
   195	            delegation_allowed: true,
   196	        }],
   197	        delegation: delegation_enabled.then(|| DelegationSection {
   198	            allow_delegated_pull: true,
   199	            // Loopback sources must be authorized by IP/CIDR form,
   200	            // mirroring the production SSRF rule (see remote_remote.rs).
   201	            allowed_source_hosts: vec!["127.0.0.1".to_string()],
   202	        }),
   203	    };
   204	
   205	    let config_path = workspace.join(format!("{name}.toml"));
   206	    let toml = toml::to_string(&config).expect("serialize config");
   207	    fs::write(&config_path, toml).expect("write config");
   208	
   209	    let child = Command::new(bin)
   210	        .arg("--config")
   211	        .arg(&config_path)
   212	        .arg("--bind")
   213	        .arg("127.0.0.1")
   214	        .arg("--port")
   215	        .arg(port.to_string())
   216	        .stdin(Stdio::null())
   217	        .stdout(Stdio::null())
   218	        .stderr(Stdio::piped())
   219	        .spawn()
   220	        .expect("spawn daemon");
   221	
   222	    wait_for_port(port, &format!("daemon {name}"));
   223	    ChildGuard::new(child)
   224	}
   225	
   226	struct DelegationContext {
   227	    _work: tempfile::TempDir,
   228	    src_port: u16,
   229	    dst_port: u16,
   230	    _src_daemon: Option<ChildGuard>,
   231	    _dst_daemon: ChildGuard,
   232	    cli_bin: PathBuf,
   233	    config_dir: PathBuf,
   234	    module_src_dir: Option<PathBuf>,
   235	    module_dst_dir: PathBuf,
   236	}
   237	
   238	impl DelegationContext {
   239	    /// Real source daemon + delegation-enabled destination daemon.
   240	    fn with_real_source() -> Self {
   241	        let work = tempdir().expect("tempdir");
   242	        let workspace = work.path().to_path_buf();
   243	
   244	        let module_src_dir = workspace.join("module_src");
   245	        fs::create_dir_all(&module_src_dir).expect("module src dir");
   246	        let module_dst_dir = workspace.join("module_dst");
   247	        fs::create_dir_all(&module_dst_dir).expect("module dst dir");
   248	        let config_dir = workspace.join("cli-config");
   249	        fs::create_dir_all(&config_dir).expect("cli config");
   250	
   251	        let (cli_bin, daemon_bin) = binary_paths();
   252	
   253	        let src_port = common::pick_unused_port();
   254	        let dst_port = common::pick_unused_port();
   255	        assert_ne!(src_port, dst_port, "ports must be different");
   256	
   257	        let src_daemon = spawn_daemon(
   258	            &workspace,
   259	            &daemon_bin,
   260	            src_port,
   261	            "daemon_src",
   262	            &module_src_dir,
   263	            false,
   264	        );
   265	        let dst_daemon = spawn_daemon(
   266	            &workspace,
   267	            &daemon_bin,
   268	            dst_port,
   269	            "daemon_dst",
   270	            &module_dst_dir,
   271	            true,
   272	        );
   273	
   274	        Self {
   275	            _work: work,
   276	            src_port,
   277	            dst_port,
   278	            _src_daemon: Some(src_daemon),
   279	            _dst_daemon: dst_daemon,
   280	            cli_bin,
   281	            config_dir,
   282	            module_src_dir: Some(module_src_dir),
   283	            module_dst_dir,
   284	        }
   285	    }
   286	
   287	    /// Fake stalling source + delegation-enabled destination daemon.
   288	    /// The fake's port is owned by the caller's `StallingSourceGuard`.
   289	    fn with_stalling_source(fake_port: u16) -> Self {
   290	        let work = tempdir().expect("tempdir");
   291	        let workspace = work.path().to_path_buf();
   292	
   293	        let module_dst_dir = workspace.join("module_dst");
   294	        fs::create_dir_all(&module_dst_dir).expect("module dst dir");
   295	        let config_dir = workspace.join("cli-config");
   296	        fs::create_dir_all(&config_dir).expect("cli config");
   297	
   298	        let (cli_bin, daemon_bin) = binary_paths();
   299	        let dst_port = common::pick_unused_port();
   300	
   301	        let dst_daemon = spawn_daemon(
   302	            &workspace,
   303	            &daemon_bin,
   304	            dst_port,
   305	            "daemon_dst",
   306	            &module_dst_dir,
   307	            true,
   308	        );
   309	
   310	        Self {
   311	            _work: work,
   312	            src_port: fake_port,
   313	            dst_port,
   314	            _src_daemon: None,
   315	            _dst_daemon: dst_daemon,
   316	            cli_bin,
   317	            config_dir,
   318	            module_src_dir: None,
   319	            module_dst_dir,
   320	        }
   321	    }
   322	
   323	    fn source_remote(&self) -> String {
   324	        format!("127.0.0.1:{}:/test/", self.src_port)
   325	    }
   326	
   327	    fn dest_remote(&self) -> String {
   328	        format!("127.0.0.1:{}:/test/", self.dst_port)
   329	    }
   330	
   331	    fn dest_host(&self) -> String {
   332	        format!("127.0.0.1:{}", self.dst_port)
   333	    }
   334	
   335	    fn run_blit(&self, args: &[&str]) -> std::process::Output {
   336	        let mut cmd = Command::new(&self.cli_bin);
   337	        cmd.arg("--config-dir").arg(&self.config_dir);
   338	        for arg in args {
   339	            cmd.arg(arg);
   340	        }
   341	        run_with_timeout(cmd, Duration::from_secs(60))
   342	    }
   343	}
   344	
   345	/// Run a detached delegated copy and return the transfer_id parsed
   346	/// from the `--json` detach output ({"outcome":"detached", ...}).
   347	fn detach_copy(ctx: &DelegationContext) -> String {
   348	    let src = ctx.source_remote();
   349	    let dst = ctx.dest_remote();
   350	    let output = ctx.run_blit(&["copy", &src, &dst, "--detach", "--json"]);
   351	    assert!(
   352	        output.status.success(),
   353	        "detached copy must succeed\nstdout:\n{}\nstderr:\n{}",
   354	        String::from_utf8_lossy(&output.stdout),
   355	        String::from_utf8_lossy(&output.stderr)
   356	    );
   357	    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
   358	        panic!(
   359	            "--detach --json must emit valid JSON ({err})\nstdout:\n{}",
   360	            String::from_utf8_lossy(&output.stdout)
   361	        )
   362	    });
   363	    assert_eq!(
   364	        json.get("outcome").and_then(|o| o.as_str()),
   365	        Some("detached"),
   366	        "detach JSON must carry outcome=detached, got: {json}"
   367	    );
   368	    let transfer_id = json
   369	        .get("transfer_id")
   370	        .and_then(|t| t.as_str())
   371	        .expect("detach JSON must carry transfer_id");
   372	    assert!(!transfer_id.is_empty(), "transfer_id must not be empty");
   373	    transfer_id.to_string()
   374	}
   375	
   376	#[test]
   377	fn detached_copy_watch_to_terminal_then_cancel_is_not_found() {
   378	    let ctx = DelegationContext::with_real_source();
   379	    fs::write(
   380	        ctx.module_src_dir
   381	            .as_ref()
   382	            .expect("real source")
   383	            .join("payload.txt"),
   384	        b"jobs-lifecycle-e2e",
   385	    )
   386	    .expect("write src file");
   387	
   388	    let transfer_id = detach_copy(&ctx);
   389	
   390	    // Watch the detached job to its terminal state. Whether the
   391	    // tiny transfer is still active at subscribe time or already in
   392	    // the recent ring, finished-ok exits 0 on both paths.
   393	    let dest_host = ctx.dest_host();
   394	    let watch = ctx.run_blit(&[
   395	        "jobs",
   396	        "watch",
   397	        &dest_host,
   398	        &transfer_id,
   399	        "--timeout-secs",
   400	        "30",
   401	        "--json",
   402	    ]);
   403	    assert_eq!(
   404	        watch.status.code(),
   405	        Some(0),
   406	        "watch must reach finished-ok\nstdout:\n{}\nstderr:\n{}",
   407	        String::from_utf8_lossy(&watch.stdout),
   408	        String::from_utf8_lossy(&watch.stderr)
   409	    );
   410	
   411	    // The byte path was daemon-to-daemon; the payload must be on dst.
   412	    let dest_file = ctx.module_dst_dir.join("payload.txt");
   413	    assert_eq!(
   414	        fs::read(&dest_file).expect("dst payload must exist"),
   415	        b"jobs-lifecycle-e2e",
   416	        "delegated copy must land the payload on the destination module"
   417	    );
   418	
   419	    // The finished job must be visible in `jobs list` (recent ring).
   420	    let list = ctx.run_blit(&["jobs", "list", &dest_host, "--json"]);
   421	    assert_eq!(list.status.code(), Some(0));
   422	    let stdout = String::from_utf8_lossy(&list.stdout);
   423	    assert!(
   424	        stdout.contains(&transfer_id),
   425	        "finished transfer must appear in jobs list, got:\n{stdout}"
   426	    );
   427	
   428	    // Cancelling a finished job is NotFound → exit 1 (§6.5).
   429	    let cancel = ctx.run_blit(&["jobs", "cancel", &dest_host, &transfer_id]);
   430	    assert_eq!(
   431	        cancel.status.code(),
   432	        Some(1),
   433	        "cancel of a finished transfer must exit 1 (NotFound)\nstdout:\n{}\nstderr:\n{}",
   434	        String::from_utf8_lossy(&cancel.stdout),
   435	        String::from_utf8_lossy(&cancel.stderr)
   436	    );
   437	}
   438	
   439	#[test]
   440	fn cancel_of_active_delegated_job_exits_zero() {
   441	    // A fake source that accepts the gRPC connection but never
   442	    // answers PullSync: the destination daemon emits Started (the
   443	    // ActiveJobs row registers synchronously at dispatch, before the
   444	    // handler runs), then stalls inside pull_sync_with_spec — a
   445	    // deterministic window in which the job is active and cancelable.
   446	    let fake = spawn_stalling_source();
   447	    let ctx = DelegationContext::with_stalling_source(fake.port);
   448	
   449	    let transfer_id = detach_copy(&ctx);
   450	
   451	    let dest_host = ctx.dest_host();
   452	    let cancel = ctx.run_blit(&["jobs", "cancel", &dest_host, &transfer_id]);
   453	    assert_eq!(
   454	        cancel.status.code(),
   455	        Some(0),
   456	        "cancel of an active delegated job must exit 0 (Cancelled)\nstdout:\n{}\nstderr:\n{}",
   457	        String::from_utf8_lossy(&cancel.stdout),
   458	        String::from_utf8_lossy(&cancel.stderr)
   459	    );
   460	}
   461	
   462	// ---------------------------------------------------------------
   463	// Fake stalling source: a tonic server whose pull_sync never
   464	// answers. Everything else is unimplemented (same shape as
   465	// remote_remote.rs's fake daemons).
   466	// ---------------------------------------------------------------
   467	
   468	struct StallingSourceGuard {
   469	    port: u16,
   470	    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
   471	    join: Option<thread::JoinHandle<()>>,
   472	}
   473	
   474	impl Drop for StallingSourceGuard {
   475	    fn drop(&mut self) {
   476	        if let Some(tx) = self.shutdown.take() {
   477	            let _ = tx.send(());
   478	        }
   479	        if let Some(join) = self.join.take() {
   480	            let _ = join.join();
   481	        }
   482	    }
   483	}
   484	
   485	fn spawn_stalling_source() -> StallingSourceGuard {
   486	    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake source");
   487	    let port = listener.local_addr().expect("fake source addr").port();
   488	    listener
   489	        .set_nonblocking(true)
   490	        .expect("set fake source nonblocking");
   491	    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
   492	
   493	    let join = thread::spawn(move || {
   494	        let runtime = tokio::runtime::Builder::new_current_thread()
   495	            .enable_all()
   496	            .build()
   497	            .expect("fake source runtime");
   498	        runtime.block_on(async move {
   499	            use blit_core::generated::blit_server::BlitServer;
   500	            use tokio_stream::wrappers::TcpListenerStream;
   501	            use tonic::transport::Server;
   502	
   503	            let listener =
   504	                tokio::net::TcpListener::from_std(listener).expect("tokio fake source listener");
   505	            Server::builder()
   506	                .add_service(BlitServer::new(StallingPullSyncBlit))
   507	                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
   508	                    let _ = shutdown_rx.await;
   509	                })
   510	                .await
   511	                .expect("fake source server");
   512	        });
   513	    });
   514	
   515	    wait_for_port(port, "fake stalling source");
   516	    StallingSourceGuard {
   517	        port,
   518	        shutdown: Some(shutdown_tx),
   519	        join: Some(join),
   520	    }

exec
/usr/bin/zsh -lc 'git show f6e592e''^:crates/blit-cli/tests/readonly_enforcement.rs | nl -ba | sed -n '"'1,360p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! w9-4: read-only-module enforcement tests
     2	//! (tests-readonly-module-enforcement-untested).
     3	//!
     4	//! The daemon refuses writes to `read_only: true` modules in three
     5	//! places — push control stream (push/control.rs), purge
     6	//! (core.rs::purge_inner), and delegated pull (delegated_pull.rs) —
     7	//! and before this file no test in the workspace ever configured a
     8	//! read-only module, so a dropped gate (mirror-deletion blast
     9	//! radius) would have passed the full validation suite.
    10	//!
    11	//! The dual-daemon mini-harness for the delegated case is another
    12	//! clone of the remote_remote.rs pattern; consolidation is w9-3.
    13	
    14	use std::fs;
    15	use std::net::TcpStream;
    16	use std::path::{Path, PathBuf};
    17	use std::process::{Command, Stdio};
    18	use std::thread;
    19	use std::time::Duration;
    20	
    21	use serde::Serialize;
    22	use tempfile::tempdir;
    23	
    24	mod common;
    25	use common::{run_with_timeout, ChildGuard, TestContext};
    26	
    27	fn assert_read_only_rejection(output: &std::process::Output, what: &str) {
    28	    assert!(
    29	        !output.status.success(),
    30	        "{what} against a read-only module must fail\nstdout:\n{}\nstderr:\n{}",
    31	        String::from_utf8_lossy(&output.stdout),
    32	        String::from_utf8_lossy(&output.stderr)
    33	    );
    34	    let stderr = String::from_utf8_lossy(&output.stderr);
    35	    assert!(
    36	        stderr.contains("read-only"),
    37	        "{what} must surface the read-only rejection, got stderr:\n{stderr}"
    38	    );
    39	}
    40	
    41	#[test]
    42	fn push_to_read_only_module_is_rejected_and_module_untouched() {
    43	    let ctx = TestContext::new_read_only();
    44	
    45	    let src_dir = ctx.workspace.join("src");
    46	    fs::create_dir_all(&src_dir).expect("src dir");
    47	    fs::write(src_dir.join("payload.txt"), b"must-not-land").expect("write src");
    48	
    49	    let dest = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    50	    let src_arg = format!("{}/", src_dir.display());
    51	    let mut cmd = Command::new(&ctx.cli_bin);
    52	    cmd.arg("--config-dir")
    53	        .arg(&ctx.config_dir)
    54	        .arg("copy")
    55	        .arg(&src_arg)
    56	        .arg(&dest);
    57	    let output = run_with_timeout(cmd, Duration::from_secs(60));
    58	
    59	    assert_read_only_rejection(&output, "push");
    60	    assert!(
    61	        fs::read_dir(&ctx.module_dir)
    62	            .expect("module dir readable")
    63	            .next()
    64	            .is_none(),
    65	        "read-only module must stay untouched after a rejected push"
    66	    );
    67	}
    68	
    69	/// design-5: with many files the client is mid-manifest-send when the
    70	/// daemon's rejection lands, so the request-stream send fails before
    71	/// the response is read — and pre-fix the user saw "failed to send
    72	/// push request payload" instead of the read-only reason. This was the
    73	/// first failure the w9-1/w9-4 ungating surfaced on CI (macOS and
    74	/// Windows lost the race; local single-file runs won it). The client
    75	/// now harvests the daemon's terminal status on send failure.
    76	#[test]
    77	fn push_rejection_reason_survives_midmanifest_send_failure() {
    78	    let ctx = TestContext::new_read_only();
    79	
    80	    let src_dir = ctx.workspace.join("src");
    81	    fs::create_dir_all(&src_dir).expect("src dir");
    82	    for i in 0..500 {
    83	        fs::write(src_dir.join(format!("f{i}.txt")), b"x").expect("write src");
    84	    }
    85	
    86	    let dest = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    87	    let src_arg = format!("{}/", src_dir.display());
    88	    let mut cmd = Command::new(&ctx.cli_bin);
    89	    cmd.arg("--config-dir")
    90	        .arg(&ctx.config_dir)
    91	        .arg("copy")
    92	        .arg(&src_arg)
    93	        .arg(&dest);
    94	    let output = run_with_timeout(cmd, Duration::from_secs(60));
    95	
    96	    assert_read_only_rejection(&output, "many-file push");
    97	    assert!(
    98	        fs::read_dir(&ctx.module_dir)
    99	            .expect("module dir readable")
   100	            .next()
   101	            .is_none(),
   102	        "read-only module must stay untouched after a rejected many-file push"
   103	    );
   104	}
   105	
   106	#[test]
   107	fn purge_on_read_only_module_is_rejected_and_file_survives() {
   108	    let ctx = TestContext::new_read_only();
   109	
   110	    // Seed the module on disk directly — the daemon serves it
   111	    // read-only, but the filesystem itself is writable.
   112	    let victim = ctx.module_dir.join("keep.txt");
   113	    fs::write(&victim, b"survives").expect("seed module file");
   114	
   115	    let target = format!("127.0.0.1:{}:/test/keep.txt", ctx.daemon_port);
   116	    let mut cmd = Command::new(&ctx.cli_bin);
   117	    cmd.arg("--config-dir")
   118	        .arg(&ctx.config_dir)
   119	        .arg("rm")
   120	        .arg(&target)
   121	        .arg("-y");
   122	    let output = run_with_timeout(cmd, Duration::from_secs(60));
   123	
   124	    assert_read_only_rejection(&output, "rm (purge)");
   125	    assert_eq!(
   126	        fs::read(&victim).expect("file must still exist"),
   127	        b"survives",
   128	        "rejected purge must not delete anything"
   129	    );
   130	}
   131	
   132	// ---------------------------------------------------------------
   133	// Delegated pull: needs a delegation-enabled destination whose
   134	// module is read-only. The gate fires before the destination ever
   135	// contacts the source, but a real source daemon keeps the test
   136	// honest about ordering.
   137	// ---------------------------------------------------------------
   138	
   139	#[derive(Serialize)]
   140	struct DaemonConfig {
   141	    daemon: DaemonSection,
   142	    #[serde(rename = "module")]
   143	    modules: Vec<ModuleSection>,
   144	    #[serde(skip_serializing_if = "Option::is_none")]
   145	    delegation: Option<DelegationSection>,
   146	}
   147	
   148	#[derive(Serialize)]
   149	struct DaemonSection {
   150	    bind: String,
   151	    port: u16,
   152	    no_mdns: bool,
   153	}
   154	
   155	#[derive(Serialize)]
   156	struct ModuleSection {
   157	    name: String,
   158	    path: PathBuf,
   159	    comment: Option<String>,
   160	    read_only: bool,
   161	    delegation_allowed: bool,
   162	}
   163	
   164	#[derive(Serialize)]
   165	struct DelegationSection {
   166	    allow_delegated_pull: bool,
   167	    allowed_source_hosts: Vec<String>,
   168	}
   169	
   170	fn wait_for_port(port: u16, label: &str) {
   171	    let mut ready = false;
   172	    for _ in 0..50 {
   173	        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
   174	            ready = true;
   175	            break;
   176	        }
   177	        thread::sleep(Duration::from_millis(100));
   178	    }
   179	    assert!(ready, "{label} failed to listen on {port}");
   180	}
   181	
   182	#[allow(clippy::too_many_arguments)]
   183	fn spawn_daemon(
   184	    workspace: &Path,
   185	    bin: &Path,
   186	    port: u16,
   187	    name: &str,
   188	    module_path: &Path,
   189	    read_only: bool,
   190	    delegation_enabled: bool,
   191	) -> ChildGuard {
   192	    let config = DaemonConfig {
   193	        daemon: DaemonSection {
   194	            bind: "127.0.0.1".into(),
   195	            port,
   196	            no_mdns: true,
   197	        },
   198	        modules: vec![ModuleSection {
   199	            name: "test".into(),
   200	            path: module_path.to_path_buf(),
   201	            comment: None,
   202	            read_only,
   203	            delegation_allowed: true,
   204	        }],
   205	        delegation: delegation_enabled.then(|| DelegationSection {
   206	            allow_delegated_pull: true,
   207	            allowed_source_hosts: vec!["127.0.0.1".to_string()],
   208	        }),
   209	    };
   210	
   211	    let config_path = workspace.join(format!("{name}.toml"));
   212	    let toml = toml::to_string(&config).expect("serialize config");
   213	    fs::write(&config_path, toml).expect("write config");
   214	
   215	    let child = Command::new(bin)
   216	        .arg("--config")
   217	        .arg(&config_path)
   218	        .arg("--bind")
   219	        .arg("127.0.0.1")
   220	        .arg("--port")
   221	        .arg(port.to_string())
   222	        .stdin(Stdio::null())
   223	        .stdout(Stdio::null())
   224	        .stderr(Stdio::piped())
   225	        .spawn()
   226	        .expect("spawn daemon");
   227	
   228	    wait_for_port(port, &format!("daemon {name}"));
   229	    ChildGuard::new(child)
   230	}
   231	
   232	#[test]
   233	fn delegated_pull_to_read_only_destination_is_rejected() {
   234	    let work = tempdir().expect("tempdir");
   235	    let workspace = work.path().to_path_buf();
   236	
   237	    let module_src = workspace.join("module_src");
   238	    let module_dst = workspace.join("module_dst");
   239	    fs::create_dir_all(&module_src).expect("src module");
   240	    fs::create_dir_all(&module_dst).expect("dst module");
   241	    fs::write(module_src.join("payload.txt"), b"must-not-land").expect("seed src");
   242	
   243	    let config_dir = workspace.join("cli-config");
   244	    fs::create_dir_all(&config_dir).expect("cli config");
   245	
   246	    let exe_path = std::env::current_exe().expect("current_exe");
   247	    let bin_dir = exe_path
   248	        .parent()
   249	        .expect("deps dir")
   250	        .parent()
   251	        .expect("bin dir")
   252	        .to_path_buf();
   253	    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
   254	    let daemon_bin = bin_dir.join(if cfg!(windows) {
   255	        "blit-daemon.exe"
   256	    } else {
   257	        "blit-daemon"
   258	    });
   259	
   260	    let src_port = common::pick_unused_port();
   261	    let dst_port = common::pick_unused_port();
   262	    assert_ne!(src_port, dst_port);
   263	
   264	    let _src = spawn_daemon(
   265	        &workspace,
   266	        &daemon_bin,
   267	        src_port,
   268	        "daemon_src",
   269	        &module_src,
   270	        false,
   271	        false,
   272	    );
   273	    // Destination: delegation enabled, module read-only — the
   274	    // delegated_pull read-only gate is what must fire.
   275	    let _dst = spawn_daemon(
   276	        &workspace,
   277	        &daemon_bin,
   278	        dst_port,
   279	        "daemon_dst",
   280	        &module_dst,
   281	        true,
   282	        true,
   283	    );
   284	
   285	    let src_remote = format!("127.0.0.1:{src_port}:/test/");
   286	    let dst_remote = format!("127.0.0.1:{dst_port}:/test/");
   287	    let mut cmd = Command::new(&cli_bin);
   288	    cmd.arg("--config-dir")
   289	        .arg(&config_dir)
   290	        .arg("copy")
   291	        .arg(&src_remote)
   292	        .arg(&dst_remote);
   293	    let output = run_with_timeout(cmd, Duration::from_secs(60));
   294	
   295	    assert_read_only_rejection(&output, "delegated pull");
   296	    assert!(
   297	        fs::read_dir(&module_dst)
   298	            .expect("dst module readable")
   299	            .next()
   300	            .is_none(),
   301	        "read-only destination module must stay untouched"
   302	    );
   303	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/tests/jobs_lifecycle.rs | sed -n '200,430p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   200	    );
   201	    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
   202	        panic!(
   203	            "--detach --json must emit valid JSON ({err})\nstdout:\n{}",
   204	            String::from_utf8_lossy(&output.stdout)
   205	        )
   206	    });
   207	    assert_eq!(
   208	        json.get("outcome").and_then(|o| o.as_str()),
   209	        Some("detached"),
   210	        "detach JSON must carry outcome=detached, got: {json}"
   211	    );
   212	    let transfer_id = json
   213	        .get("transfer_id")
   214	        .and_then(|t| t.as_str())
   215	        .expect("detach JSON must carry transfer_id");
   216	    assert!(!transfer_id.is_empty(), "transfer_id must not be empty");
   217	    transfer_id.to_string()
   218	}
   219	
   220	#[test]
   221	fn detached_copy_watch_to_terminal_then_cancel_is_not_found() {
   222	    let ctx = DelegationContext::with_real_source();
   223	    fs::write(
   224	        ctx.module_src_dir
   225	            .as_ref()
   226	            .expect("real source")
   227	            .join("payload.txt"),
   228	        b"jobs-lifecycle-e2e",
   229	    )
   230	    .expect("write src file");
   231	
   232	    let transfer_id = detach_copy(&ctx);
   233	
   234	    // Watch the detached job to its terminal state. Whether the
   235	    // tiny transfer is still active at subscribe time or already in
   236	    // the recent ring, finished-ok exits 0 on both paths.
   237	    let dest_host = ctx.dest_host();
   238	    let watch = ctx.run_blit(&[
   239	        "jobs",
   240	        "watch",
   241	        &dest_host,
   242	        &transfer_id,
   243	        "--timeout-secs",
   244	        "30",
   245	        "--json",
   246	    ]);
   247	    assert_eq!(
   248	        watch.status.code(),
   249	        Some(0),
   250	        "watch must reach finished-ok\nstdout:\n{}\nstderr:\n{}",
   251	        String::from_utf8_lossy(&watch.stdout),
   252	        String::from_utf8_lossy(&watch.stderr)
   253	    );
   254	
   255	    // The byte path was daemon-to-daemon; the payload must be on dst.
   256	    let dest_file = ctx.module_dst_dir.join("payload.txt");
   257	    assert_eq!(
   258	        fs::read(&dest_file).expect("dst payload must exist"),
   259	        b"jobs-lifecycle-e2e",
   260	        "delegated copy must land the payload on the destination module"
   261	    );
   262	
   263	    // The finished job must be visible in `jobs list` (recent ring).
   264	    let list = ctx.run_blit(&["jobs", "list", &dest_host, "--json"]);
   265	    assert_eq!(list.status.code(), Some(0));
   266	    let stdout = String::from_utf8_lossy(&list.stdout);
   267	    assert!(
   268	        stdout.contains(&transfer_id),
   269	        "finished transfer must appear in jobs list, got:\n{stdout}"
   270	    );
   271	
   272	    // Cancelling a finished job is NotFound → exit 1 (§6.5).
   273	    let cancel = ctx.run_blit(&["jobs", "cancel", &dest_host, &transfer_id]);
   274	    assert_eq!(
   275	        cancel.status.code(),
   276	        Some(1),
   277	        "cancel of a finished transfer must exit 1 (NotFound)\nstdout:\n{}\nstderr:\n{}",
   278	        String::from_utf8_lossy(&cancel.stdout),
   279	        String::from_utf8_lossy(&cancel.stderr)
   280	    );
   281	}
   282	
   283	#[test]
   284	fn cancel_of_active_delegated_job_exits_zero() {
   285	    // A fake source that accepts the gRPC connection but never
   286	    // answers PullSync: the destination daemon emits Started (the
   287	    // ActiveJobs row registers synchronously at dispatch, before the
   288	    // handler runs), then stalls inside pull_sync_with_spec — a
   289	    // deterministic window in which the job is active and cancelable.
   290	    let fake = spawn_stalling_source();
   291	    let ctx = DelegationContext::with_stalling_source(fake.port);
   292	
   293	    let transfer_id = detach_copy(&ctx);
   294	
   295	    let dest_host = ctx.dest_host();
   296	    let cancel = ctx.run_blit(&["jobs", "cancel", &dest_host, &transfer_id]);
   297	    assert_eq!(
   298	        cancel.status.code(),
   299	        Some(0),
   300	        "cancel of an active delegated job must exit 0 (Cancelled)\nstdout:\n{}\nstderr:\n{}",
   301	        String::from_utf8_lossy(&cancel.stdout),
   302	        String::from_utf8_lossy(&cancel.stderr)
   303	    );
   304	}
   305	
   306	// ---------------------------------------------------------------
   307	// Fake stalling source: a tonic server whose pull_sync never
   308	// answers. Everything else is unimplemented. Served through the
   309	// shared production-shaped scaffold (common::spawn_fake_blit_server).
   310	// ---------------------------------------------------------------
   311	
   312	fn spawn_stalling_source() -> common::FakeServerGuard {
   313	    spawn_fake_blit_server(StallingPullSyncBlit, "fake stalling source")
   314	}
   315	
   316	struct StallingPullSyncBlit;
   317	
   318	#[tonic::async_trait]
   319	impl blit_core::generated::blit_server::Blit for StallingPullSyncBlit {
   320	    type PushStream = tokio_stream::wrappers::ReceiverStream<
   321	        Result<blit_core::generated::ServerPushResponse, tonic::Status>,
   322	    >;
   323	    type PullSyncStream = tokio_stream::wrappers::ReceiverStream<
   324	        Result<blit_core::generated::ServerPullMessage, tonic::Status>,
   325	    >;
   326	    type FindStream = tokio_stream::wrappers::ReceiverStream<
   327	        Result<blit_core::generated::FindEntry, tonic::Status>,
   328	    >;
   329	    type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
   330	        Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
   331	    >;
   332	    type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
   333	        Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
   334	    >;
   335	    type SubscribeStream = std::pin::Pin<
   336	        Box<
   337	            dyn tokio_stream::Stream<
   338	                    Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
   339	                > + Send,
   340	        >,
   341	    >;
   342	
   343	    async fn push(
   344	        &self,
   345	        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
   346	    ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
   347	        Err(tonic::Status::unimplemented("stalling fake source"))
   348	    }
   349	
   350	    /// The point of this fake: accept the RPC and never answer.
   351	    async fn pull_sync(
   352	        &self,
   353	        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
   354	    ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
   355	        std::future::pending::<()>().await;
   356	        unreachable!("pending() never resolves")
   357	    }
   358	
   359	    async fn subscribe(
   360	        &self,
   361	        _: tonic::Request<blit_core::generated::SubscribeRequest>,
   362	    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
   363	        Err(tonic::Status::unimplemented("stalling fake source"))
   364	    }
   365	
   366	    async fn list(
   367	        &self,
   368	        _: tonic::Request<blit_core::generated::ListRequest>,
   369	    ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
   370	        Err(tonic::Status::unimplemented("stalling fake source"))
   371	    }
   372	
   373	    async fn purge(
   374	        &self,
   375	        _: tonic::Request<blit_core::generated::PurgeRequest>,
   376	    ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
   377	        Err(tonic::Status::unimplemented("stalling fake source"))
   378	    }
   379	
   380	    async fn complete_path(
   381	        &self,
   382	        _: tonic::Request<blit_core::generated::CompletionRequest>,
   383	    ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
   384	        Err(tonic::Status::unimplemented("stalling fake source"))
   385	    }
   386	
   387	    async fn list_modules(
   388	        &self,
   389	        _: tonic::Request<blit_core::generated::ListModulesRequest>,
   390	    ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
   391	        Err(tonic::Status::unimplemented("stalling fake source"))
   392	    }
   393	
   394	    async fn find(
   395	        &self,
   396	        _: tonic::Request<blit_core::generated::FindRequest>,
   397	    ) -> Result<tonic::Response<Self::FindStream>, tonic::Status> {
   398	        Err(tonic::Status::unimplemented("stalling fake source"))
   399	    }
   400	
   401	    async fn disk_usage(
   402	        &self,
   403	        _: tonic::Request<blit_core::generated::DiskUsageRequest>,
   404	    ) -> Result<tonic::Response<Self::DiskUsageStream>, tonic::Status> {
   405	        Err(tonic::Status::unimplemented("stalling fake source"))
   406	    }
   407	
   408	    async fn filesystem_stats(
   409	        &self,
   410	        _: tonic::Request<blit_core::generated::FilesystemStatsRequest>,
   411	    ) -> Result<tonic::Response<blit_core::generated::FilesystemStatsResponse>, tonic::Status> {
   412	        Err(tonic::Status::unimplemented("stalling fake source"))
   413	    }
   414	
   415	    async fn delegated_pull(
   416	        &self,
   417	        _: tonic::Request<blit_core::generated::DelegatedPullRequest>,
   418	    ) -> Result<tonic::Response<Self::DelegatedPullStream>, tonic::Status> {
   419	        Err(tonic::Status::unimplemented("stalling fake source"))
   420	    }
   421	
   422	    async fn get_state(
   423	        &self,
   424	        _: tonic::Request<blit_core::generated::GetStateRequest>,
   425	    ) -> Result<tonic::Response<blit_core::generated::DaemonState>, tonic::Status> {
   426	        Err(tonic::Status::unimplemented("stalling fake source"))
   427	    }
   428	
   429	    async fn cancel_job(
   430	        &self,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/tests/remote_remote.rs | sed -n '1,460p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::fs;
     2	use std::path::{Path, PathBuf};
     3	use std::process::Command;
     4	use std::time::Duration;
     5	
     6	use tempfile::tempdir;
     7	
     8	mod common;
     9	use common::{run_with_timeout, spawn_fake_blit_server, DaemonOptions, SpawnedDaemon, TestContext};
    10	
    11	/// Dual real daemons in one workspace: daemon A (the context's own
    12	/// daemon) is the plain source; daemon B is the destination whose
    13	/// delegation gate is under test. Harness lives in `common` (w9-3).
    14	struct DualDaemonContext {
    15	    _ctx: TestContext,
    16	    _daemon_b: SpawnedDaemon,
    17	    workspace: PathBuf,
    18	    daemon_a_port: u16,
    19	    daemon_b_port: u16,
    20	    cli_bin: PathBuf,
    21	    config_dir: PathBuf,
    22	    module_a_dir: PathBuf,
    23	    module_b_dir: PathBuf,
    24	}
    25	
    26	impl DualDaemonContext {
    27	    fn new(dest_delegation: bool) -> Self {
    28	        let ctx = TestContext::new();
    29	        let daemon_b = ctx.spawn_second_daemon(
    30	            "daemon_b",
    31	            &DaemonOptions {
    32	                delegation: dest_delegation,
    33	                ..Default::default()
    34	            },
    35	        );
    36	
    37	        Self {
    38	            workspace: ctx.workspace.clone(),
    39	            daemon_a_port: ctx.daemon_port,
    40	            daemon_b_port: daemon_b.port,
    41	            cli_bin: ctx.cli_bin.clone(),
    42	            config_dir: ctx.config_dir.clone(),
    43	            module_a_dir: ctx.module_dir.clone(),
    44	            module_b_dir: daemon_b.module_dir.clone(),
    45	            _ctx: ctx,
    46	            _daemon_b: daemon_b,
    47	        }
    48	    }
    49	
    50	    fn source_remote(&self) -> String {
    51	        format!("127.0.0.1:{}:/test/", self.daemon_a_port)
    52	    }
    53	
    54	    fn dest_remote(&self) -> String {
    55	        format!("127.0.0.1:{}:/test/", self.daemon_b_port)
    56	    }
    57	
    58	    fn counter_path(&self, name: &str) -> PathBuf {
    59	        self.workspace.join(format!("{name}.counter"))
    60	    }
    61	}
    62	
    63	#[test]
    64	fn remote_to_remote_copy_delegates_directly_without_cli_byte_path() {
    65	    let ctx = DualDaemonContext::new(true);
    66	    let payload = vec![b'x'; 2 * 1024 * 1024];
    67	    fs::write(ctx.module_a_dir.join("payload.bin"), &payload).expect("write src file");
    68	
    69	    let counter = ctx.counter_path("direct");
    70	    let output = run_blit(
    71	        &ctx,
    72	        &["copy", &ctx.source_remote(), &ctx.dest_remote()],
    73	        Some(&counter),
    74	    );
    75	    assert_success(&output);
    76	
    77	    assert_eq!(
    78	        fs::read(ctx.module_b_dir.join("payload.bin")).unwrap(),
    79	        payload
    80	    );
    81	    let counters = read_counters(&counter);
    82	    assert_eq!(
    83	        counters.remote_transfer_source_constructed, 0,
    84	        "direct path must not construct RemoteTransferSource"
    85	    );
    86	    assert_eq!(
    87	        counters.cli_data_plane_outbound_bytes, 0,
    88	        "direct path must not send payload bytes from the CLI data plane"
    89	    );
    90	}
    91	
    92	#[test]
    93	fn remote_to_remote_gate_reject_does_not_fall_back_to_relay() {
    94	    let ctx = DualDaemonContext::new(false);
    95	    fs::write(ctx.module_a_dir.join("payload.bin"), b"payload").expect("write src file");
    96	
    97	    let counter = ctx.counter_path("gate_reject");
    98	    let output = run_blit(
    99	        &ctx,
   100	        &["copy", &ctx.source_remote(), &ctx.dest_remote()],
   101	        Some(&counter),
   102	    );
   103	    assert!(
   104	        !output.status.success(),
   105	        "delegation-disabled destination should fail"
   106	    );
   107	    let stderr = String::from_utf8_lossy(&output.stderr);
   108	    assert!(
   109	        stderr.contains("delegated pull is disabled"),
   110	        "expected gate reason in stderr, got:\n{stderr}"
   111	    );
   112	    assert!(!ctx.module_b_dir.join("payload.bin").exists());
   113	
   114	    let counters = read_counters(&counter);
   115	    assert_eq!(counters.remote_transfer_source_constructed, 0);
   116	    assert_eq!(counters.cli_data_plane_outbound_bytes, 0);
   117	}
   118	
   119	#[test]
   120	fn remote_to_remote_explicit_relay_uses_legacy_cli_byte_path() {
   121	    let ctx = DualDaemonContext::new(false);
   122	    let payload = vec![b'r'; 1024 * 1024];
   123	    fs::write(ctx.module_a_dir.join("relay.bin"), &payload).expect("write src file");
   124	
   125	    let counter = ctx.counter_path("relay");
   126	    let output = run_blit(
   127	        &ctx,
   128	        &[
   129	            "copy",
   130	            "--relay-via-cli",
   131	            &ctx.source_remote(),
   132	            &ctx.dest_remote(),
   133	        ],
   134	        Some(&counter),
   135	    );
   136	    assert_success(&output);
   137	
   138	    assert_eq!(
   139	        fs::read(ctx.module_b_dir.join("relay.bin")).unwrap(),
   140	        payload
   141	    );
   142	    let counters = read_counters(&counter);
   143	    assert!(
   144	        counters.remote_transfer_source_constructed > 0,
   145	        "--relay-via-cli must construct the relay source"
   146	    );
   147	    assert!(
   148	        counters.cli_data_plane_outbound_bytes >= payload.len() as u64,
   149	        "relay path should send payload-sized bytes through the CLI data plane; counters={counters:?}"
   150	    );
   151	}
   152	
   153	#[test]
   154	fn remote_to_remote_relay_transfers_nested_tree() {
   155	    // ue-r2-1h: the relay's enumeration (`scan_remote_files`) and
   156	    // per-file byte streaming (`open_remote_file`) moved from the
   157	    // deleted Pull RPC onto PullSync sessions (metadata-only scan +
   158	    // single-file force_grpc reads). A nested multi-file tree
   159	    // exercises both against the REAL daemon: recursive header
   160	    // enumeration with correct relative paths, then byte-identical
   161	    // per-file relay.
   162	    let ctx = DualDaemonContext::new(false);
   163	    let big = vec![b'B'; 512 * 1024];
   164	    fs::create_dir_all(ctx.module_a_dir.join("nested/deep")).expect("mkdirs");
   165	    fs::write(ctx.module_a_dir.join("top.bin"), &big).expect("write top");
   166	    fs::write(ctx.module_a_dir.join("nested/mid.txt"), b"middle file").expect("write mid");
   167	    fs::write(ctx.module_a_dir.join("nested/deep/leaf.txt"), b"leaf").expect("write leaf");
   168	
   169	    let counter = ctx.counter_path("relay_tree");
   170	    let output = run_blit(
   171	        &ctx,
   172	        &[
   173	            "copy",
   174	            "--relay-via-cli",
   175	            &ctx.source_remote(),
   176	            &ctx.dest_remote(),
   177	        ],
   178	        Some(&counter),
   179	    );
   180	    assert_success(&output);
   181	
   182	    assert_eq!(fs::read(ctx.module_b_dir.join("top.bin")).unwrap(), big);
   183	    assert_eq!(
   184	        fs::read(ctx.module_b_dir.join("nested/mid.txt")).unwrap(),
   185	        b"middle file"
   186	    );
   187	    assert_eq!(
   188	        fs::read(ctx.module_b_dir.join("nested/deep/leaf.txt")).unwrap(),
   189	        b"leaf"
   190	    );
   191	
   192	    let counters = read_counters(&counter);
   193	    assert!(
   194	        counters.remote_transfer_source_constructed > 0,
   195	        "--relay-via-cli must construct the relay source"
   196	    );
   197	}
   198	
   199	#[test]
   200	fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
   201	    let work = tempdir().expect("tempdir");
   202	    let config_dir = work.path().join("cli-config");
   203	    fs::create_dir_all(&config_dir).expect("cli config");
   204	    let cli_bin = common::cli_bin();
   205	    let stale = spawn_fake_blit_server(UnimplementedBlit, "fake unimplemented destination");
   206	    let counter = work.path().join("stale.counter");
   207	
   208	    let src_remote = "127.0.0.1:9:/test/";
   209	    let dst_remote = format!("127.0.0.1:{}:/test/", stale.port);
   210	    let mut cmd = Command::new(cli_bin);
   211	    cmd.arg("--config-dir")
   212	        .arg(&config_dir)
   213	        .arg("--diagnostics-counter-file")
   214	        .arg(&counter)
   215	        .arg("copy")
   216	        .arg(src_remote)
   217	        .arg(dst_remote);
   218	
   219	    let output = run_with_timeout(cmd, Duration::from_secs(20));
   220	    assert!(
   221	        !output.status.success(),
   222	        "stale destination should fail without fallback"
   223	    );
   224	    let stderr = String::from_utf8_lossy(&output.stderr);
   225	    assert!(
   226	        stderr.contains("does not implement DelegatedPull"),
   227	        "expected explicit upgrade/relay message, got:\n{stderr}"
   228	    );
   229	
   230	    let counters = read_counters(&counter);
   231	    assert_eq!(counters.remote_transfer_source_constructed, 0);
   232	    assert_eq!(counters.cli_data_plane_outbound_bytes, 0);
   233	}
   234	
   235	#[test]
   236	fn source_refuses_destination_negotiation_does_not_fall_back_to_relay() {
   237	    let ctx = DualDaemonContext::new(true);
   238	    let rejecting_source = spawn_fake_blit_server(RejectingPullSyncBlit, "fake rejecting source");
   239	    let counter = ctx.counter_path("source_refuses");
   240	    let src_remote = format!("127.0.0.1:{}:/test/", rejecting_source.port);
   241	
   242	    let output = run_blit(
   243	        &ctx,
   244	        &["copy", &src_remote, &ctx.dest_remote()],
   245	        Some(&counter),
   246	    );
   247	    assert!(
   248	        !output.status.success(),
   249	        "source negotiation refusal should fail without fallback"
   250	    );
   251	    let stderr = String::from_utf8_lossy(&output.stderr);
   252	    assert!(
   253	        stderr.contains("source refused delegated pull"),
   254	        "expected NEGOTIATE wording in stderr, got:\n{stderr}"
   255	    );
   256	    assert!(
   257	        stderr.contains("source ACL rejected delegated peer"),
   258	        "expected source refusal reason in stderr, got:\n{stderr}"
   259	    );
   260	    assert!(
   261	        fs::read_dir(&ctx.module_b_dir).unwrap().next().is_none(),
   262	        "destination should remain empty after source negotiation refusal"
   263	    );
   264	
   265	    let counters = read_counters(&counter);
   266	    assert_eq!(counters.remote_transfer_source_constructed, 0);
   267	    assert_eq!(counters.cli_data_plane_outbound_bytes, 0);
   268	}
   269	
   270	fn run_blit(
   271	    ctx: &DualDaemonContext,
   272	    args: &[&str],
   273	    counter: Option<&Path>,
   274	) -> std::process::Output {
   275	    let mut cmd = Command::new(&ctx.cli_bin);
   276	    cmd.arg("--config-dir").arg(&ctx.config_dir);
   277	    // audit-l39: --diagnostics-counter-file replaced the pre-0.1.1
   278	    // BLIT_TEST_COUNTER_FILE env var. Both flags are global, so they
   279	    // must appear before the subcommand.
   280	    if let Some(path) = counter {
   281	        cmd.arg("--diagnostics-counter-file").arg(path);
   282	    }
   283	    for arg in args {
   284	        cmd.arg(arg);
   285	    }
   286	    run_with_timeout(cmd, Duration::from_secs(60))
   287	}
   288	
   289	fn assert_success(output: &std::process::Output) {
   290	    if !output.status.success() {
   291	        panic!(
   292	            "blit failed with status {}\nstdout:\n{}\nstderr:\n{}",
   293	            output.status,
   294	            String::from_utf8_lossy(&output.stdout),
   295	            String::from_utf8_lossy(&output.stderr)
   296	        );
   297	    }
   298	}
   299	
   300	#[derive(Debug, Default)]
   301	struct CounterValues {
   302	    cli_data_plane_outbound_bytes: u64,
   303	    remote_transfer_source_constructed: u64,
   304	}
   305	
   306	fn read_counters(path: &Path) -> CounterValues {
   307	    let mut out = CounterValues::default();
   308	    let Ok(contents) = fs::read_to_string(path) else {
   309	        return out;
   310	    };
   311	    for line in contents.lines() {
   312	        let mut parts = line.split_whitespace();
   313	        let Some(name) = parts.next() else { continue };
   314	        let value = parts
   315	            .next()
   316	            .and_then(|v| v.parse::<u64>().ok())
   317	            .unwrap_or(0);
   318	        match name {
   319	            "cli_data_plane_outbound_bytes" => {
   320	                out.cli_data_plane_outbound_bytes =
   321	                    out.cli_data_plane_outbound_bytes.saturating_add(value);
   322	            }
   323	            "remote_transfer_source_constructed" => {
   324	                out.remote_transfer_source_constructed =
   325	                    out.remote_transfer_source_constructed.saturating_add(value);
   326	            }
   327	            _ => {}
   328	        }
   329	    }
   330	    out
   331	}
   332	
   333	struct UnimplementedBlit;
   334	
   335	#[tonic::async_trait]
   336	impl blit_core::generated::blit_server::Blit for UnimplementedBlit {
   337	    type PushStream = tokio_stream::wrappers::ReceiverStream<
   338	        Result<blit_core::generated::ServerPushResponse, tonic::Status>,
   339	    >;
   340	    type PullSyncStream = tokio_stream::wrappers::ReceiverStream<
   341	        Result<blit_core::generated::ServerPullMessage, tonic::Status>,
   342	    >;
   343	    type FindStream = tokio_stream::wrappers::ReceiverStream<
   344	        Result<blit_core::generated::FindEntry, tonic::Status>,
   345	    >;
   346	    type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
   347	        Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
   348	    >;
   349	    type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
   350	        Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
   351	    >;
   352	    type SubscribeStream = std::pin::Pin<
   353	        Box<
   354	            dyn tokio_stream::Stream<
   355	                    Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
   356	                > + Send,
   357	        >,
   358	    >;
   359	
   360	    async fn push(
   361	        &self,
   362	        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
   363	    ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
   364	        Err(tonic::Status::unimplemented("stale daemon"))
   365	    }
   366	
   367	    async fn pull_sync(
   368	        &self,
   369	        _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
   370	    ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
   371	        Err(tonic::Status::unimplemented("stale daemon"))
   372	    }
   373	
   374	    async fn subscribe(
   375	        &self,
   376	        _: tonic::Request<blit_core::generated::SubscribeRequest>,
   377	    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
   378	        Err(tonic::Status::unimplemented("stale daemon"))
   379	    }
   380	
   381	    async fn list(
   382	        &self,
   383	        _: tonic::Request<blit_core::generated::ListRequest>,
   384	    ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
   385	        Err(tonic::Status::unimplemented("stale daemon"))
   386	    }
   387	
   388	    async fn purge(
   389	        &self,
   390	        _: tonic::Request<blit_core::generated::PurgeRequest>,
   391	    ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
   392	        Err(tonic::Status::unimplemented("stale daemon"))
   393	    }
   394	
   395	    async fn complete_path(
   396	        &self,
   397	        _: tonic::Request<blit_core::generated::CompletionRequest>,
   398	    ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
   399	        Err(tonic::Status::unimplemented("stale daemon"))
   400	    }
   401	
   402	    async fn list_modules(
   403	        &self,
   404	        _: tonic::Request<blit_core::generated::ListModulesRequest>,
   405	    ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
   406	        Err(tonic::Status::unimplemented("stale daemon"))
   407	    }
   408	
   409	    async fn find(
   410	        &self,
   411	        _: tonic::Request<blit_core::generated::FindRequest>,
   412	    ) -> Result<tonic::Response<Self::FindStream>, tonic::Status> {
   413	        Err(tonic::Status::unimplemented("stale daemon"))
   414	    }
   415	
   416	    async fn disk_usage(
   417	        &self,
   418	        _: tonic::Request<blit_core::generated::DiskUsageRequest>,
   419	    ) -> Result<tonic::Response<Self::DiskUsageStream>, tonic::Status> {
   420	        Err(tonic::Status::unimplemented("stale daemon"))
   421	    }
   422	
   423	    async fn filesystem_stats(
   424	        &self,
   425	        _: tonic::Request<blit_core::generated::FilesystemStatsRequest>,
   426	    ) -> Result<tonic::Response<blit_core::generated::FilesystemStatsResponse>, tonic::Status> {
   427	        Err(tonic::Status::unimplemented("stale daemon"))
   428	    }
   429	
   430	    async fn delegated_pull(
   431	        &self,
   432	        _: tonic::Request<blit_core::generated::DelegatedPullRequest>,
   433	    ) -> Result<tonic::Response<Self::DelegatedPullStream>, tonic::Status> {
   434	        Err(tonic::Status::unimplemented("stale daemon"))
   435	    }
   436	
   437	    async fn get_state(
   438	        &self,
   439	        _: tonic::Request<blit_core::generated::GetStateRequest>,
   440	    ) -> Result<tonic::Response<blit_core::generated::DaemonState>, tonic::Status> {
   441	        Err(tonic::Status::unimplemented("stale daemon"))
   442	    }
   443	
   444	    async fn cancel_job(
   445	        &self,
   446	        _: tonic::Request<blit_core::generated::CancelJobRequest>,
   447	    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
   448	        Err(tonic::Status::unimplemented("stale daemon"))
   449	    }
   450	
   451	    async fn clear_recent(
   452	        &self,
   453	        _: tonic::Request<blit_core::generated::ClearRecentRequest>,
   454	    ) -> Result<tonic::Response<blit_core::generated::ClearRecentResponse>, tonic::Status> {
   455	        Err(tonic::Status::unimplemented("stale daemon"))
   456	    }
   457	}
   458	
   459	struct RejectingPullSyncBlit;
   460	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/tests/pull_sync_with_spec_wire.rs | sed -n '180,220p;600,630p;950,1015p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   180	}
   181	
   182	async fn spawn_spy(captured: Arc<Mutex<Option<TransferOperationSpec>>>) -> u16 {
   183	    spawn_spy_with_rejection(captured, None).await
   184	}
   185	
   186	async fn spawn_spy_with_rejection(
   187	    captured: Arc<Mutex<Option<TransferOperationSpec>>>,
   188	    reject_pull_sync: Option<(tonic::Code, &'static str)>,
   189	) -> u16 {
   190	    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
   191	        .await
   192	        .expect("bind ephemeral port");
   193	    let port = listener.local_addr().expect("local_addr").port();
   194	
   195	    tokio::spawn(async move {
   196	        let svc = BlitServer::new(SpyServer {
   197	            captured,
   198	            reject_pull_sync,
   199	        });
   200	        production_server_builder()
   201	            .add_service(svc)
   202	            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
   203	            .await
   204	            .expect("server runs");
   205	    });
   206	
   207	    // Tiny grace period so the server is listening before the client
   208	    // attempts to connect. tonic's Server::serve_with_incoming starts
   209	    // the listener synchronously above the await, so this is mostly
   210	    // belt-and-suspenders.
   211	    tokio::time::sleep(Duration::from_millis(50)).await;
   212	    port
   213	}
   214	
   215	fn hand_built_spec() -> TransferOperationSpec {
   216	    TransferOperationSpec {
   217	        spec_version: 2,
   218	        // Deliberately different from the values the client's
   219	        // endpoint would imply — that's the load-bearing assertion:
   220	        // the spec wins, the endpoint is transport-only.
   600	    frames: Vec<server_pull_message::Payload>,
   601	) -> u16 {
   602	    spawn_canned_with_acks(captured, frames, Arc::default()).await
   603	}
   604	
   605	async fn spawn_canned_with_acks(
   606	    captured: Arc<Mutex<Option<TransferOperationSpec>>>,
   607	    frames: Vec<server_pull_message::Payload>,
   608	    acks: Arc<Mutex<Vec<blit_core::generated::DataPlaneResizeAck>>>,
   609	) -> u16 {
   610	    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
   611	        .await
   612	        .expect("bind ephemeral port");
   613	    let port = listener.local_addr().expect("local_addr").port();
   614	
   615	    tokio::spawn(async move {
   616	        let svc = BlitServer::new(CannedFramesServer {
   617	            captured,
   618	            frames,
   619	            acks,
   620	        });
   621	        production_server_builder()
   622	            .add_service(svc)
   623	            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
   624	            .await
   625	            .expect("server runs");
   626	    });
   627	
   628	    tokio::time::sleep(Duration::from_millis(50)).await;
   629	    port
   630	}
   950	        .await;
   951	
   952	    let mut tries = 0;
   953	    loop {
   954	        if !acks.lock().await.is_empty() {
   955	            break;
   956	        }
   957	        tries += 1;
   958	        assert!(tries < 100, "client never acked the resize command");
   959	        tokio::time::sleep(Duration::from_millis(20)).await;
   960	    }
   961	    let acks = acks.lock().await;
   962	    assert_eq!(acks.len(), 1);
   963	    assert_eq!(acks[0].epoch, 9);
   964	    assert!(
   965	        !acks[0].accepted,
   966	        "resize on a non-negotiated session must be refused"
   967	    );
   968	}
   969	
   970	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   971	async fn pull_client_dials_resize_add_with_the_epoch_credential_and_acks() {
   972	    // Full client-dialer pin against a real data-plane listener:
   973	    // epoch-0 socket presents token || epoch0_sub, the ADD is acked
   974	    // accepted:true, and the epoch-1 socket presents token || sub1.
   975	    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
   976	
   977	    let token: Vec<u8> = (0u8..32).collect();
   978	    let sub0: Vec<u8> = vec![0xA0; 16];
   979	    let sub1: Vec<u8> = vec![0xB1; 16];
   980	
   981	    let data_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
   982	        .await
   983	        .expect("bind data listener");
   984	    let data_port = data_listener.local_addr().expect("addr").port();
   985	
   986	    // Data-plane driver: verify both handshakes, then END both
   987	    // sockets so the client workers exit cleanly.
   988	    let expect0: Vec<u8> = token.iter().chain(sub0.iter()).copied().collect();
   989	    let expect1: Vec<u8> = token.iter().chain(sub1.iter()).copied().collect();
   990	    let driver = tokio::spawn(async move {
   991	        // The epoch-0 and epoch-1 dials race (the canned server plays
   992	        // its script back-to-back), so accept order is not guaranteed
   993	        // — assert on the SET of handshakes instead.
   994	        let mut handshakes = Vec::new();
   995	        let mut sockets = Vec::new();
   996	        for i in 0..2 {
   997	            let (mut sock, _) = data_listener.accept().await.expect("accept");
   998	            let mut buf = vec![0u8; expect0.len()];
   999	            sock.read_exact(&mut buf)
  1000	                .await
  1001	                .unwrap_or_else(|e| panic!("handshake {i}: {e}"));
  1002	            handshakes.push(buf);
  1003	            sockets.push(sock);
  1004	        }
  1005	        assert!(
  1006	            handshakes.contains(&expect0),
  1007	            "one socket carries token || epoch0_sub"
  1008	        );
  1009	        assert!(
  1010	            handshakes.contains(&expect1),
  1011	            "one socket carries token || add sub_token"
  1012	        );
  1013	        assert_ne!(handshakes[0], handshakes[1], "distinct credentials");
  1014	
  1015	        // END records terminate both receive workers normally.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/grpc_server.rs | sed -n '1,90p'; nl -ba crates/blit-daemon/src/main.rs | sed -n '118,142p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Production-shaped tonic server construction.
     2	//!
     3	//! Single owner of the gRPC control-plane HTTP/2 keepalive policy, so
     4	//! the real daemon and every in-process test server start from the
     5	//! same builder and cannot drift apart (w9-3,
     6	//! tests-fake-server-config-skew — before this module, every fake
     7	//! tonic server in the test tree was a bare `Server::builder()` with
     8	//! no keepalive, while production set it, so wire tests exercised a
     9	//! server shaped differently from every deployed daemon).
    10	
    11	use std::time::Duration;
    12	
    13	use tonic::transport::Server;
    14	
    15	/// HTTP/2 keepalive PING interval for Blit gRPC servers.
    16	///
    17	/// audit-1 (owner decision 2026-05-23): a subscriber (TUI F2 /
    18	/// `jobs watch`) that vanishes mid-stream — crash, network partition,
    19	/// killed laptop lid — would otherwise leave the daemon holding the
    20	/// gRPC stream + broadcast Receiver + spawned forwarder task forever,
    21	/// because TCP alone doesn't notice a silently-dead peer. Keepalive
    22	/// PINGs idle connections at this interval.
    23	pub const HTTP2_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
    24	
    25	/// How long an unanswered keepalive PING may hang before the
    26	/// connection is reaped. Healthy idle subscribers answer PINGs and
    27	/// stay connected (Subscribe is legitimately silent during quiet
    28	/// periods), so this reclaims leaked resources without the reconnect
    29	/// churn an app-level "no events for N seconds" close would cause.
    30	pub const HTTP2_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(20);
    31	
    32	/// The `Server::builder()` every Blit gRPC server starts from — the
    33	/// production daemon and in-process test fakes alike — so the
    34	/// keepalive policy above is applied in exactly one place.
    35	pub fn production_server_builder() -> Server {
    36	    Server::builder()
    37	        .http2_keepalive_interval(Some(HTTP2_KEEPALIVE_INTERVAL))
    38	        .http2_keepalive_timeout(Some(HTTP2_KEEPALIVE_TIMEOUT))
    39	}
    40	
    41	#[cfg(test)]
    42	mod tests {
    43	    use super::*;
    44	
    45	    /// Pins the owner-decided keepalive values (audit-1, 2026-05-23):
    46	    /// interval 30s / timeout 20s. Changing them is an owner call,
    47	    /// not a refactor side effect.
    48	    #[test]
    49	    fn keepalive_values_match_owner_decision() {
    50	        assert_eq!(HTTP2_KEEPALIVE_INTERVAL, Duration::from_secs(30));
    51	        assert_eq!(HTTP2_KEEPALIVE_TIMEOUT, Duration::from_secs(20));
    52	    }
    53	}
   118	    // rec-1: hydrate the recent-runs ring from disk and arm
   119	    // write-through persistence before serving, so `GetState.recent[]`
   120	    // survives daemon restarts. A store that can't be read degrades to
   121	    // an empty ring (handled in `recents_store::load`); only path
   122	    // resolution can fail here, which is fatal config breakage worth
   123	    // surfacing.
   124	    let _recents_writer = active_jobs::spawn_recents_writer(service.active_jobs.arm_persistence()?);
   125	
   126	    println!("blitd v2 listening on {}", addr);
   127	
   128	    // audit-1: HTTP/2 keepalive reaps subscribers that vanish
   129	    // mid-stream while leaving healthy idle ones untouched. The values
   130	    // and full rationale (owner decision 2026-05-23) live with
   131	    // `blit_core::remote::grpc_server` — the shared builder the
   132	    // in-process test servers also start from, so test/prod server
   133	    // config cannot drift (w9-3).
   134	    blit_core::remote::grpc_server::production_server_builder()
   135	        .add_service(BlitServer::new(service))
   136	        .serve(addr)
   137	        .await?;
   138	
   139	    drop(mdns_guard);
   140	
   141	    Ok(())
   142	}

exec
/usr/bin/zsh -lc 'git diff --check f6e592e''^ f6e592e' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git show --format= --numstat f6e592e' in /home/michael/dev/Blit
 succeeded in 0ms:
 succeeded in 0ms:
167	0	.review/findings/w9-3-test-harness-builder.md
3	31	crates/blit-cli/tests/cli_arg_safety_gates.rs
401	121	crates/blit-cli/tests/common/mod.rs
4	23	crates/blit-cli/tests/diagnostics_dump.rs
38	245	crates/blit-cli/tests/jobs_lifecycle.rs
3	31	crates/blit-cli/tests/local_move_semantics.rs
20	155	crates/blit-cli/tests/readonly_enforcement.rs
16	198	crates/blit-cli/tests/remote_checksum_negotiation.rs
25	329	crates/blit-cli/tests/remote_pull_mirror.rs
25	329	crates/blit-cli/tests/remote_remote.rs
22	272	crates/blit-cli/tests/remote_tcp_fallback.rs
3	32	crates/blit-cli/tests/single_file_copy.rs
53	0	crates/blit-core/src/remote/grpc_server.rs
1	0	crates/blit-core/src/remote/mod.rs
7	3	crates/blit-core/tests/pull_sync_with_spec_wire.rs
7	15	crates/blit-daemon/src/main.rs

exec
/usr/bin/zsh -lc 'rg -n "cargo test|1478|1479|keepalive|TestContext::builder|OnceLock|port" .review/findings/w9-3-test-harness-builder.md REVIEW.md docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md' in /home/michael/dev/Blit
 succeeded in 0ms:
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:5:**Inputs**: `DESIGN_MAP_2026-06-11.md` (Phase A, 2 errata), `DESIGN_FINDINGS_2026-06-11_PHASE_B.md` (70 adversarially-verified findings: 4 H / 40 M / 26 L), filed findings design-1/2/3, queued slice-2 transport work (STATE.md Queue).
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:6:**Disposition rule** (owner decision, D-2026-06-11-1): nothing below enters `REVIEW.md` until the owner ratifies it by slice ID. Severity/evidence per finding lives in the Phase B doc — this report deduplicates and sequences; it does not restate evidence.
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:10:The 70 confirmed findings (plus 3 filed, plus the queued transport slice) reduce
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:19:One caveat carried forward: Phase B had no progress-reporting dimension, so the
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:27:## W1 — Transport & liveness policy (extends queued slice-2)
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:30:  client channel builder; client HTTP/2 keepalive; explicit
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:35:  accept paths. Today NODELAY/keepalive/tuned-buffers exist on push sockets
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:39:- **W1.3** (M, small): TCP keepalive honesty — either configure real
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:40:  `TcpKeepalive` timing at both `set_keepalive(true)` sites or rewrite both
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:42:  failure. [drift-set-keepalive-comments-oversell-liveness]
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:73:  `scan_remote_files`' metadata path on PullSync or port it.
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:114:  report**): install a stderr log backend (warn level) in all four binaries.
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:122:  next to the conversion sites with a contract test; re-export from blit-app.
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:126:- **W5.3** (M, medium): daemon error-boundary helpers — `internal_err(ctx, &Report)`
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:175:  literals. [boundaries-private-default-port-literal-duplication]
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:181:  fs_enum helpers + exports (~800 lines, no wire impact). **Owner decision
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:205:- **W9.3** (M, medium): harness consolidation — `TestContext::builder()` with
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:207:  delegation, read_only), shared `cli_bin()`, OnceLock'd daemon build (replaces
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:209:  keepalive. [tests-five-daemon-harness-clones, tests-per-test-cargo-build-subprocess,
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:240:1. **W5.1** log backend (smallest slice, unmutes ~20 existing failure reports)
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:245:6. **W1.1 (queued) + W1.2 + design-3** transport policy bundle
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:6:**Outcome**: 76 findings reported → **70 confirmed** (4 high, 40 medium, 26 low after verifier severity corrections), **6 refuted** by the verification layer. ~28 findings were downgraded in severity — the impact lens was the strictest judge. Already-filed items (design-1/2/3) and the queued slice-2 transport work were excluded from re-reporting by prompt and cross-referenced instead.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:65:**Mechanism**: pull_sync.rs stream_via_data_plane calls determine_remote_tuning(total_bytes) at line 550 but then: negotiates `stream_count = 1u32` (line 568, comment claims 'multi-stream support lives in pull.rs' — the deprecated Pull RPC); hardcodes `pool_size = 4` (line 637) instead of the streams*2+4 formula push uses; passes literal prefetch `8` to DataPlaneSession::from_stream (line 641) and execute_sink_pipeline (line 651) where tuning.prefetch_count would say 16/32; and tuning.tcp_buffer_size is never applied — the CLI pull client connects with a bare TcpStream::connect (blit-core/src/remote/pull.rs:1710), unlike the push client whose DataPlaneSession::connect applies nodelay/keepalive/buffer sizes (data_plane.rs:99-122). CLI pull routes through pull_sync (pull.rs:707), confirmed live. Push for the same workload: daemon offers desired_streams (up to 16, control.rs:217/233) and the client honors it (push/client/mod.rs:637-653). So big-tree throughput is direction-dependent by authoring-time constants, not hardware.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:68:- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:568 — let stream_count = 1u32; // Single stream for the resume path (multi-stream support lives in pull.rs)
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:71:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1710 — pull client data-plane connect is a bare TcpStream::connect — tuning.tcp_buffer_size / nodelay / keepalive never applied on the pull direction
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:74:**Proposed fix**: First slice: honor the tuning the daemon already computed — use tuning.prefetch_count, the shared pool formula, and apply nodelay/keepalive/tcp_buffer_size on the pull client's data-plane socket (mirror DataPlaneSession::connect). Multi-stream pull-sync negotiation is the follow-up slice (the wire field stream_count already exists and the client-side receive loop already supports N invocations).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:82:**Mechanism**: rg for `.pull(` workspace-wide finds only three gRPC-stub calls inside blit-core/src/remote/pull.rs itself: line 305 (inside the deprecated `pull` method at :251), line 491 (scan_remote_files) and line 539 (open_remote_file). The latter two hardcode force_grpc:true (:485, :530), so the daemon's Pull handler always takes the gRPC/non-streaming branches (daemon pull.rs:64 single-file, :85 force_grpc||metadata_only) and never reaches stream_pull_streaming (:208) or the TCP accepts (accept_pull_data_connection :625, accept_pull_data_connection_streaming :841, enumerate_to_channel :764, pull_stream_count :915). The only code that could send force_grpc=false is the deprecated client method, which nothing calls — pull.rs's own test doc at :1855 calls it 'the deprecated `pull` method', and the daemon comment at pull.rs:694-696 calls its server half 'this deprecated-but-exposed Pull RPC path'. Meanwhile the live PullSync handler negotiates stream_count=1 at both of its negotiation sites (pull_sync.rs:567-568 with the comment 'multi-stream support lives in pull.rs', and :707), even though the blit-core client side can receive multiple streams (receive_data_plane_streams_owned, pull.rs:1600-1646). Net: production pull runs one TCP stream; the up-to-16-stream ladder lives only in dead code. Wire-compat caveat: proto/blit.proto:11 still declares `rpc Pull`, so out-of-repo older clients could reach the TCP branches — retiring the RPC is an owner decision.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:88:- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:567 — 'Single stream for the resume path (multi-stream support lives in pull.rs)'; stream_count = 1 here and at :707 — the only two negotiation sites in the file
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:91:**Proposed fix**: Two-part: (a) OWNER DECISION — retire the wire-level Pull RPC (or keep it gRPC-only by deleting just the TCP-negotiation branches, which no in-repo client can trigger); (b) port the multi-stream data plane (pull_stream_count ladder + parallel accept) from dead pull.rs into pull_sync's negotiation so the production pull path stops being single-stream. Then delete RemotePullClient::pull and the daemon TCP branches.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:99:**Mechanism**: rg for PermissionsExt|std::os::unix|symlink across blit-cli/tests hits only f2_chroot_containment.rs, remote_regression.rs, remote_push_mirror_safety.rs, and local_move_semantics.rs — the other eight gated files are pure process/fs/CLI tests. The harness already handles Windows binary names (common/mod.rs:94-104, 'blit.exe'/'blit-daemon.exe'). CI runs `cargo test --workspace` on windows-latest (ci.yml test-windows job) and the manual parity runner scripts/windows/run-blit-tests.ps1 also just runs cargo test, so both inherit the same hole: on Windows, mirror purge, push mirror safety, resume (block-level + gRPC fallback variant), TCP/gRPC fallback negotiation, checksum negotiation, and remote-to-remote delegation are compiled out. AGENTS.md §5 says 'Windows parity matters', yet the validation suite structurally cannot observe a Windows regression in any of those paths — a green Windows CI job is a false parity signal.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:103:- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:94 — harness already selects blit.exe/blit-daemon.exe on Windows — Windows support was built, then gated off
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:105:- /home/michael/dev/Blit/.github/workflows/ci.yml:47 — test-windows job runs cargo test --workspace; with the gating it exercises zero mirror/resume/fallback/parity remote tests
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:106:- /home/michael/dev/Blit/scripts/windows/run-blit-tests.ps1:48 — the manual Windows parity runner is also just cargo test steps, inheriting the same compiled-out coverage
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:118:**Mechanism**: core.rs:499 (push), :579 (pull), :631 (pull_sync) spawn `handler(...).await` with no select; only delegated_pull (core.rs:741-783) uses resolve_delegated_pull_outcome racing tx.closed() and cancel_token.cancelled(). A disconnect is therefore observed only when the handler next awaits a tx.send — but pull_sync's Phase 3 (pull_sync.rs:111) runs collect_pull_entries_with_checksums first: enumerate + Blake3-hash the entire requested tree inside spawn_blocking+rayon (pull.rs:448-499), which is not abortable and performs zero sends until done; push's mirror purge (control.rs:347) is a second send-free spawn_blocking phase. HTTP/2 keepalive reaping the dead connection (main.rs:137-142) drops the response stream but nothing polls it, so the blocking work continues regardless; ActiveJobKind::supports_cancellation (active_jobs.rs:163) returns false for all three kinds, with the justifying comment at :155-158 — 'a client-side cancel already drops the handler future via tx.closed()' — describing a race that exists only in the delegated_pull spawn site. Repeating connect→checksum-spec→disconnect multiplies concurrent full-disk hash jobs that nothing can cancel.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:124:- /home/michael/dev/Blit/crates/blit-daemon/src/active_jobs.rs:156 — comment claims 'client-side cancel already drops the handler future via tx.closed()' — false for push/pull/pull_sync; supports_cancellation=false at :163
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:149:**Mechanism**: Workers are spawned at data_plane.rs:130 into a plain Vec<JoinHandle>; the join loop at :143-146 does `handle.await.map_err(...)??` — the first Err returns from the function, dropping every remaining handle (detach, not abort). Likewise the accept-timeout arm at :103-110 returns Err after some workers were already spawned. Each detached worker continues running handle_data_plane_stream → receive_push_data_plane → FsTransferSink, writing client bytes to disk with no owner, unreachable by CancelJob (push reports supports_cancellation=false, active_jobs.rs:163-164), until its socket EOFs or the 30s StallGuard fires — and if the client's own detached pipeline (sibling finding) keeps sending, indefinitely. The failed RPC meanwhile drives the client to retry, producing a second writer set against the same destination paths. design-2 (.review/findings/design-2-orphaned-daemon-data-planes.md) names exactly three sites (service/pull.rs:180/:297, push/control.rs:57); this Vec of per-stream workers is a fourth, one level deeper, and needs the same AbortOnDrop/JoinSet-abort treatment or design-2's fix will still leak the inner layer.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:155:- /home/michael/dev/Blit/crates/blit-daemon/src/active_jobs.rs:163 — supports_cancellation() is DelegatedPull-only — orphaned push workers unreachable by CancelJob
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:198:**Mechanism**: control.rs:55 creates `mpsc::channel::<FileHeader>(FILE_UPLOAD_CHANNEL_CAPACITY)` (= 16*1024*16 = 262,144, :31); every manifest entry passing file_requires_upload is sent at :157 *before* the transfer-mode branch. In fallback mode (client --force-grpc, daemon force_grpc_data, or the automatic bind-failure fallback at :181-199) `upload_rx_opt.take()` is never executed (:214/:287 are TCP-only), so the receiver stays alive-but-unread in the local until the function returns: send #262,145 awaits forever. The daemon stops reading the request stream, gRPC flow control backpressures the client's manifest sends, and both sides wedge with no timeout in scope — HTTP/2 keepalive (main.rs:137-142) sees a healthy connection, StallGuard covers TCP data planes only. In TCP mode the consumer (data_plane.rs:89, :164) wraps the receiver in Arc<AsyncMutex>, each of N workers spawns a task (:200-206) whose only body is `while guard.recv().await.is_some() {}` (N-1 of them blocked on the mutex), and the companion `cache` is explicitly voided (:207) — the comment at control.rs:150-156 admits 'Only the gRPC fallback path uses this queue', which is false: the fallback path is the one that never reads it.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:230:**Mechanism**: display.rs:1-2 states the module is 'shared by every presenter (CLI text output, TUI panes, JSON-embedded reason strings)' and format_bytes (display.rs:14-26) uses binary units B..TiB. blit-cli already imports it for ls/df/local transfers, but jobs.rs:471-481 defines its own format_bps with decimal thresholds (1_000_000_000 -> GB) — two unit systems in one binary. blit-tui, which depends on blit-app, re-implements format_bytes privately per screen; the f2.rs copy (f2.rs:555-574) even documents 'd-25: aligned with F4's format_bytes' — copies being synced against each other by review finding instead of importing the library function built for them. Boundary-level duplication only (TUI internals out of scope per Phase 6 rule).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:235:- /home/michael/dev/Blit/crates/blit-tui/src/screens/f2.rs:555 — 'd-25: aligned with F4's format_bytes' — private copy synced against another private copy instead of importing blit_app::display
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:237:**Proposed fix**: Add a format_bps (or rate wrapper) to blit_app::display in the binary-unit convention, switch jobs.rs to it, and replace the TUI screen-local copies with imports — one decision: binary units everywhere.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:245:**Mechanism**: The enum (progress.rs:6-10) is three undocumented variants. blit-tui's progress_accum.rs is where the actual contract is written down: accumulate_pull_progress's doc (lines 12-21) explains that the TCP path emits the same bytes on BOTH Payload and FileComplete while the gRPC path puts bytes on Payload with FileComplete{bytes:0} — citing pipeline.rs and pull.rs:finalize_active_file by name; accumulate_push_progress (lines 41-50) documents that push reports bytes only on FileComplete and emits no Payload; accumulate_delegated_progress documents a third meaning (Payload carries both deltas, no FileComplete). Three folding functions in a downstream crate encode producer-internal behavior of four blit-core/blit-app emission sites; the CLI, which has only one folding rule, got pulls wrong (design-1, filed — cross-reference, not re-reported). Any new consumer or producer change re-rolls this dice because no type, doc, or test in blit-core states which variant carries bytes.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:260:**Mechanism**: DataPlaneSession::connect (data_plane.rs:92-124) is the socket-policy owner: it sets nodelay (hard error), keepalive (logged best-effort), and applies tuning.tcp_buffer_size to send/recv buffers. The pull client instead calls bare TcpStream::connect in receive_data_plane_stream_inner (pull.rs:1709-1715) — none of those options, ever. Server side mirrors the asymmetry: the push accept path wraps the accepted socket in socket2 and sets nodelay+keepalive (push/data_plane.rs:112-124), while daemon pull.rs and pull_sync.rs accept raw sockets (rg for nodelay/keepalive/socket2 in both files: zero hits). determine_remote_tuning mints tcp_buffer_size 4-8 MiB for large transfers (tuning.rs:31-36), but the only consumer is the push client socket — for every pull, the tuned value is computed and thrown away, and Nagle stays enabled on the path that carries the bytes. Distinct from design-3 (the missing connect timeout at the same call site, already filed): this is the socket-option/ownership half.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:263:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1709 — bare TcpStream::connect — no nodelay/keepalive/buffer sizing before handing to the pipeline
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:264:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:98 — policy owner: set_tcp_nodelay hard-error, set_keepalive logged, tcp_buffer_size applied at :110-121 — push connect only
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:265:- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:112 — 'Enable nodelay + keepalive to prevent idle stream timeouts' — the ONLY daemon accept path that sets socket options
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:266:- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:702 — pull accept hands the raw socket to sinks; rg confirms zero socket2/nodelay/keepalive hits in pull.rs and pull_sync.rs
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:286:#### boundaries-retry-policy-split-dead-classifier — Retry classification policy split across crates, with a dead contradictory classifier publicly exported from blit-core
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:290:**Claim**: The live retry classifier lives in blit-app while the errors it must classify are minted in blit-core (which can only reference it by comment), and blit-core simultaneously exports a zero-consumer classifier that contradicts the live one on three error kinds.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:292:**Mechanism**: blit_core::errors::categorize_io_error (errors.rs:90-117) marks ConnectionRefused/UnexpectedEof/NotConnected as Fatal (lines 107-113) while the live blit_app::transfers::retry::is_retryable_io_kind (retry.rs:35-46) marks the same three kinds Retryable. errors.rs has zero consumers (rg for categorize_io_error/ErrorCategory/crate::errors across crates/ and tests/ returns only lib.rs:9 `pub mod errors`), yet it is the discoverable, doc-commented module a future contributor would wire up — silently flipping retry semantics. Meanwhile the TODO(audit-h3c-2) blocks at pull.rs:322-329 and 780-788 name `blit_app::transfers::retry::is_retryable` as the contract their future fix must satisfy — cross-crate coupling enforced only by comment, untestable from blit-core because the dependency direction forbids the import. The queued slice-2 chain-preservation work depends on exactly this contract holding.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:297:- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — pub mod errors — the dead taxonomy is the publicly exported one
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:300:**Proposed fix**: Delete errors.rs's dead taxonomy and move is_retryable/is_retryable_io_kind into blit-core (the crate that mints and converts the errors), re-exporting from blit-app for existing callers; add the classifier-contract test next to the conversion sites so the queued slice-2 chain-preservation work can assert it in-crate.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:340:**Mechanism**: determine_remote_tuning (tuning.rs:13) is the only production caller of determine_tuning and always passes warmup_result=None, so every `if let Some(gbps)` branch in auto_tune/mod.rs:45-67 is dead; tuning.rs:27-28 then overwrites the initial_streams/max_streams that determine_tuning returned, leaving only chunk_bytes flowing through — itself a frozen 16/32/64 MiB ladder keyed on total_bytes (tuning.rs:5-11). analyze_warmup_result (auto_tune/mod.rs:26-32), a third chunk heuristic keyed on Gbps, has zero callers anywhere (rg over all crates: only its own tests). Result: a 1 GbE link and a 100 GbE link transferring the same tree get identical chunk/stream/buffer choices, and the publicly exported warmup API is a trap inviting the next contributor to believe adaptation exists.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:370:**Claim**: tar_stream.rs (414 lines), zero_copy.rs (219), delete.rs (93), copy/parallel.rs+stats.rs (68), chunked_copy_file, and four fs_enum helpers are all pub-exported with zero callers outside their own files/re-exports, each superseded by a live implementation elsewhere.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:372:**Mechanism**: Per-symbol rg across crates/ and tests/ this session: (1) tar_stream/TarConfig/TarEvent — only lib.rs:23; its header cites a parent file streaming_batch.rs that no longer exists; live tar shards go through remote/transfer/payload.rs + tar_safety. (2) zero_copy::/splice_from_socket_to_file/ZeroCopyResult/AsRawFileDescriptor — only lib.rs:27; the receive pipeline never adopted splice. (3) crate::delete/DeletePlan/compute_delete_plan/generate_delete_plan — only lib.rs:7; mirror deletion actually flows through MirrorPlanner (orchestrator/fast_path.rs:104). (4) parallel_copy_files/CopyStats/chunked_copy_file — only the definitions and copy/mod.rs:11-14 re-exports; the live local fast path is copy_paths_blocking/copy_file (orchestrator.rs:355, :1263) and the live parallel path is execute_sink_pipeline (transfer/pipeline.rs:24). (5) categorize_files (fs_enum.rs:498), enumerate_symlinks+SymlinkEntry (:20, :439, :475), enumerate_directory_deref_filtered (:521) — zero hits outside fs_enum.rs; the size split lives in transfer_plan.rs now. All compile-checked dead only because they are pub. zero_copy is the one needing an owner decision: delete, or revive as a FAST-principle feature.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:377:- /home/michael/dev/Blit/crates/blit-core/src/copy/mod.rs:11 — re-exports chunked_copy_file (:11), parallel_copy_files (:13), CopyStats (:14) — rg shows the re-exports and definitions are the only hits
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:381:**Proposed fix**: One deletion slice removing tar_stream.rs, delete.rs, copy/parallel.rs, copy/stats.rs, chunked_copy_file, and the four fs_enum helpers plus their lib.rs/copy/mod.rs exports (pure dead weight, no wire impact); a separate owner question for zero_copy.rs: delete now or file a plan to wire splice into the receive pipeline.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:387:**Claim**: payload.rs's transfer_payloads_via_control_plane has no callers — its own comment says so — yet stays pub, re-exported, and carries an actively-maintained audit-h3c chunk clamp for hypothetical future callers, duplicating the live GrpcFallbackSink path.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:389:**Mechanism**: rg for transfer_payloads_via_control_plane returns exactly three hits: the definition (payload.rs:234), the re-export (transfer/mod.rs:20), and a doc reference in grpc_fallback.rs:46. The function's own comment at payload.rs:247-249 states 'No live caller today (grep returns zero matches), but the function is pub and re-exported, so any future caller would silently bypass the cap without this line' — i.e. the audit stratum is paying ongoing maintenance cost (the clamp_fallback_chunk_size call at :251) on a dead function. It emits the same FileManifest/FileData/TarShard loop the live GrpcFallbackSink owns. Every future transport-policy change (e.g. the queued slice-2 chunk work) must be applied here too or the dead copy drifts; deleting it removes a whole replication site.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:392:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/payload.rs:247 — 'No live caller today (grep returns zero matches), but the function is pub and re-exported' — self-documented dead code with maintained clamp at :251
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:393:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/mod.rs:20 — re-export of transfer_payloads_via_control_plane — the only thing keeping it reachable
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:395:**Proposed fix**: Delete the function and its re-export; the grpc_fallback.rs:46 doc reference becomes the tombstone. Coordinate ordering with the queued slice-2 chunk_bytes deletion so neither slice has to patch the dead copy.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:418:**Mechanism**: rg 'transfer_engine' over crates/ returns zero matches (blit-core's actual modules are transfer_plan.rs and remote/transfer/; verified against ls of crates/blit-core/src). rg 'PLAN_OPTIONS' over the whole repo matches only AGENTS.md:113 itself; the real symbol is `pub struct PlanOptions` at transfer_plan.rs:25 — cited in AGENTS.md as a SHOUT_CASE constants example, so it is wrong in both name and case category. The third example, TransferOrchestrator, DOES exist (orchestrator.rs:116, re-exported at orchestrator/mod.rs:8, used by blit-app and tests/integration/*) — which means DESIGN_MAP_2026-06-11.md:105-106 ('name modules (transfer_engine, TransferOrchestrator) that do not exist in the tree') is itself half-refuted and needs an erratum before Phase C consumes it.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:463:#### drift-set-keepalive-comments-oversell-liveness — Both set_keepalive(true) sites carry comments claiming they prevent idle-stream timeouts, but OS-default keepalive timing (~2h on Linux) makes them inert for that purpose
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:467:**Claim**: Comments at the two TCP data-plane keepalive sites claim the calls keep idle connections alive / prevent idle stream timeouts during long transfers, but set_keepalive(true) without a TcpKeepalive config enables SO_KEEPALIVE at kernel-default timing (Linux tcp_keepalive_time = 7200s), so no probe fires within any realistic transfer window.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:469:**Mechanism**: socket2's Socket::set_keepalive(bool) only toggles SO_KEEPALIVE; per-socket timing requires set_tcp_keepalive(&TcpKeepalive). rg across all crates shows no TcpKeepalive or set_tcp_keepalive anywhere — only the two bare set_keepalive(true) calls (core data_plane.rs:106, daemon push/data_plane.rs:120). With Linux defaults the first probe is sent after 2 hours of idle, far beyond the 30s/15s accept/token windows and the 30s StallGuard, so the documented purpose ('Keep idle connections alive during long transfers on other streams', 'prevent idle stream timeouts during long transfers') is not achieved — at best dead peers are reaped hours later. The two copies also diverge on failure handling: the core comment (data_plane.rs:104-105) insists failures must be surfaced 'so a misconfigured run isn't silent' and logs a warning, while the daemon copy silently swallows the same failure with `let _ =` at push/data_plane.rs:120. Future authors reading either comment will assume liveness coverage that does not exist. (Distinct from queued slice-2, which adds HTTP/2 keepalive on client gRPC channels, not these raw TCP sockets.)
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:472:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:101 — comment 'Keep idle connections alive during long transfers on other streams' above set_keepalive(true) at :106 — no timing configured
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:473:- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:112 — comment 'Enable nodelay + keepalive to prevent idle stream timeouts during long transfers' above `let _ = s2.set_keepalive(true);` at :120 — error silently swallowed, contradicting the core copy's stated rationale
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:533:**Claim**: blit-cli has a shared test harness (tests/common/mod.rs TestContext) used by 11 test files, but remote_pull_mirror.rs, remote_tcp_fallback.rs, and remote_checksum_negotiation.rs each carry a private verbatim clone of its config structs, port picker, and daemon bring-up; separately, all in-repo fake tonic servers are bare Server::builder() while production sets HTTP/2 keepalive.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:535:**Mechanism**: common/mod.rs:12-42 defines DaemonConfig/DaemonSection/ModuleSection and pick_unused_port; remote_pull_mirror.rs:12-42 and remote_tcp_fallback.rs:12-41 re-declare them byte-for-byte (remote_checksum_negotiation.rs:20/43 likewise, grep-verified), and each then repeats TestContext::new()'s body inline — blitd.toml serialization, current_exe-relative binary discovery, the `cargo build -p blit-daemon` step (remote_pull_mirror.rs:47-110+, remote_tcp_fallback.rs:47-118) — so a harness fix (e.g. readiness wait, new config field) only reaches the files that use common. The fake gRPC servers are a second axis: remote_remote.rs duplicates a whole thread+runtime+Server::builder bring-up twice in one file (528-548 vs 566-586, near-identical), and those plus pull_sync_with_spec_wire.rs:201 build bare Server::builder() with no http2_keepalive_interval/timeout, whereas production main.rs:137-139 makes keepalive load-bearing (owner decision 2026-05-23) — so wire tests exercise a server config that differs from production in exactly that field.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:539:- /home/michael/dev/Blit/crates/blit-cli/tests/remote_pull_mirror.rs:12 — verbatim re-declaration of DaemonConfig/DaemonSection/ModuleSection + pick_unused_port (36-42), bring-up body inlined at 47-110
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:541:- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:540 — bare Server::builder() fake server, duplicated again at 578; production main.rs:138-139 adds keepalive these tests lack
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:543:**Proposed fix**: Port the three holdout files onto common::TestContext (extending it for their extra knobs like --no-server-checksums and dual daemons), and add a shared spawn_fake_server helper that mirrors production Server::builder settings (keepalive) so wire tests exercise the deployed config.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:603:**Proposed fix**: Move both helpers into blit-core (e.g. next to path_posix or a remote::wire_meta module) as the single producer of FileHeader metadata fields; daemon util.rs and push helpers.rs re-export/call them.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:609:**Claim**: Whether the root cause of a daemon-side failure crosses the wire depends on which line failed: most Status::*(format!(...)) sites format the eyre Report with {err}/{e} (outermost message only) while a minority use {err:#} (full chain), with both styles coexisting in the same files.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:611:**Mechanism**: eyre::Report's Display prints only the top message; the alternate {:#} prints 'top: cause1: cause2…'. In pull_sync.rs the two pipeline failure paths diverge: line 507 'planning gRPC payloads: {err}' (chain dropped) vs line 653 'pull sync data plane pipeline: {err:#}' (chain kept). util.rs does the same within one file: line 61 'path not allowed: {}: {e}' vs lines 110/124 'path containment: {e:#}'. delegated_pull.rs:374 sends 'delegated pull: {err}' as the wire upstream_message for the entire Transfer phase — the mid-transfer root cause (e.g. an io error three layers down) is amputated before it reaches the CLI. Counting with rg: 12 Status constructions format with :# vs ~69 with plain {var}. This is daemon-side eyre→Status, not the queued slice-2 client-side Status→eyre work.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:619:**Proposed fix**: One daemon-boundary helper (e.g. internal_err(context, &Report) using {:#}) and a mechanical sweep of the ~69 plain-format sites; pairs naturally with the io_to_status helper from errors-daemon-status-internal-collapse.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:621:#### errors-dead-classifier-contradicts-live (reviewer: high) — Dead blit_core::errors module is publicly exported and contradicts the live retry classifier on three error kinds
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:625:**Claim**: blit-core/src/errors.rs (ErrorCategory/TransferError/categorize_io_error) has zero importers anywhere in the workspace yet is publicly exported, and its retryability table directly contradicts the live classifier in blit-app on ConnectionRefused, UnexpectedEof, and NotConnected.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:627:**Mechanism**: errors.rs:108-113 classifies WriteZero/UnexpectedEof/AddrInUse/AddrNotAvailable/NotConnected/ConnectionRefused as Fatal ('could go either way - default to fatal'), while the live classifier retry.rs:35-46 marks ConnectionRefused/UnexpectedEof/NotConnected Retryable; errors.rs:94-98 additionally marks Interrupted/WouldBlock retryable, which retry.rs treats as fatal. I verified zero consumers: rg for categorize_io_error|ErrorCategory|blit_core::errors|crate::errors across all crates matches only errors.rs itself — every other `TransferError` hit is the unrelated proto-generated message (e.g. blit-cli/src/jobs.rs:112 uses blit_core::generated::TransferError), a name collision that makes the trap worse: the doc-commented, lib.rs:9-exported core module looks like the designated owner of retry policy, and any future contributor wiring it up flips retry semantics for three error kinds and shadows the wire type name.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:632:- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — `pub mod errors;` — dead module publicly exported
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:635:**Proposed fix**: Delete crates/blit-core/src/errors.rs and the lib.rs:9 export (call out the 4 removed tests in the finding doc's Known gaps); if a shared classifier home is wanted later, the queued slice-2 retry-classifier work is the place to design it.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:637:#### errors-log-facade-has-no-backend (reviewer: high) — All log::warn/log::error failure reporting is silently discarded — no log backend exists in any binary
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:641:**Claim**: Roughly 20 error/degradation reports in blit-core go through the `log` facade, but no binary in the workspace installs a logger, so every one of them is dropped at runtime.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:649:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/tar_safety.rs:242 — set-mtime failure on extracted tar entries reported only via log::warn — sole surface for the failure
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:673:**Claim**: The six test files under /tests (mirror_planner_tests.rs with 16 tests, enumeration_tests.rs, checksum_partial.rs, connection.rs, integration/local_transfers.rs, integration/predictor_streaming.rs) are dead: cargo test --workspace never builds them, so the coverage they appear to provide is false.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:675:**Mechanism**: Cargo only compiles a tests/ directory that belongs to a package. The root Cargo.toml is a virtual workspace ([workspace] only, no [package] section, lines 1-10), and no member crate declares a [[test]] target pointing at the root tests/ (verified: rg '[[test]]' over all crates/*/Cargo.toml returns nothing). So `cargo test --workspace` — the validation suite in AGENTS.md §5 — never compiles these files; they get no compile check and no execution. They are not trivially stale (mirror_planner_tests.rs imports blit_core::mirror_planner which exists at lib.rs:16; local_transfers.rs imports blit_core::orchestrator::TransferOrchestrator which exists at lib.rs:17), so they would mostly revive if relocated. mirror_planner.rs itself has zero #[test] (rg -c '#[test]' returns no match for it), meaning MirrorPlanner's semantic tests exist ONLY in this dead directory. connection.rs additionally requires an externally running blitd on :50051 and can never pass in CI as written.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:681:- /home/michael/dev/Blit/tests/integration/local_transfers.rs:2 — imports blit_core::orchestrator::TransferOrchestrator — dead end-to-end local-transfer tests
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:690:**Claim**: The daemon-spawn harness (config struct + port pick + cargo build + spawn + readiness poll) is implemented five times across blit-cli/tests, and cli_bin()/run_with_timeout are re-implemented in four-five more files, with real behavioral drift already present between copies.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:701:**Proposed fix**: Grow tests/common/mod.rs into a builder: TestContext::builder().extra_daemon_args(..).delegation(..).read_only(..) plus a second_daemon() helper; export cli_bin()/run_with_timeout from common and delete every clone. Pure test refactor, no production code.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:707:**Claim**: The entire detached-job lifecycle — Subscribe stream, watch loop with GetState fallback reconciliation, CancelJob exit-code contract, --detach output — is never executed end-to-end by cargo test; coverage stops at formatting/exit-code unit tests.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:722:**Claim**: TestContext::new() and all four clone harnesses run a `cargo build` subprocess per test; with 69 TestContext::new() call sites plus the clones, a full `cargo test --workspace` run spawns ~75 nested cargo invocations that all contend for the target-directory flock.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:724:**Mechanism**: common/mod.rs:111-134 spawns `cargo build -p blit-daemon --bin blit-daemon` inside TestContext::new(), which is called once per test (counted: 69 uses across 11 files, e.g. admin_verbs.rs 15, blit_utils.rs 22). Tests within one binary run on parallel threads, so concurrent nested cargo processes serialize on the build-dir lock; even no-op rebuilds cost several hundred ms each plus lock wait. There is no Once/OnceLock dedup anywhere in common/mod.rs (verified by rg). The build exists for a real reason (cargo test -p blit-cli does not build blit-daemon's bin; comment at remote_checksum_negotiation.rs:91-93 cites R16-F1 ordering), but per-test invocation is the wrong granularity — once per process is sufficient.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:731:**Proposed fix**: Wrap the build step in a `static BUILD: OnceLock<PathBuf>` (or std::sync::Once) in tests/common so each test binary builds the daemon at most once; clones disappear with the harness consolidation finding.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:751:#### boundaries-private-default-port-literal-duplication (reviewer: medium) — RemoteEndpoint::DEFAULT_PORT is private, forcing consumers to re-state 9031 as magic literals
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:755:**Claim**: The canonical daemon port constant is non-pub inside impl RemoteEndpoint, so blit-cli and blit-tui hardcode 9031 in behavior-bearing positions.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:757:**Mechanism**: endpoint.rs:25 declares `const DEFAULT_PORT: u16 = 9031;` with no pub, inside the impl. blit-cli/scan.rs:63 branches on `if service.port == 9031` to decide whether the printed endpoint omits the port — display logic keyed on a literal that must track the private constant. blit-tui/daemons.rs:335 builds the local fallback via RemoteEndpoint::parse("127.0.0.1:9031"). A port-default change in blit-core compiles cleanly while scan output and the TUI's local-daemon row silently point at the wrong port — exactly the pub(crate)-primitive-that-should-be-shared pattern.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:761:- /home/michael/dev/Blit/crates/blit-cli/src/scan.rs:63 — if service.port == 9031 — display behavior keyed on the duplicated literal
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:772:**Mechanism**: FsTransferSource::scan (source.rs:69) imports and delegates to crate::remote::push::client::helpers::spawn_manifest_task, and check_availability (source.rs:86-88) delegates to helpers::filter_readable_headers. The unification layer (transfer/source.rs, per the PIPELINE_UNIFICATION plan it cites) was built to replace the push-v1 monolith, but its generic source is implemented in terms of push-private helpers — so the v1 module can never be deleted without breaking the abstraction that supersedes it, and any push-specific behavior change in those helpers silently changes every pipeline consumer (local mirror, daemon receive). This is the documented unfinished 'step 3b/4' seam frozen into a dependency edge.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:778:**Proposed fix**: Move spawn_manifest_task and filter_readable_headers from push/client/helpers.rs into remote/transfer (source.rs or a manifest module) and have the push client import them from there, restoring the dependency direction unification intended.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:843:#### constants-three-size-taxonomies (reviewer: medium) — Three small/medium/large taxonomies with different boundaries, one of them dead-but-exported
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:847:**Claim**: A file's size class depends on which layer asks: the planner says medium below 256 MiB, BufferSizer's scaling treats >100 MB as large, and a third classifier with 1 MB/100 MB boundaries is dead code still publicly exported.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:858:#### deadcode-app-stub-module-and-perf-query (reviewer: medium) — blit-app ships an empty remote_remote_direct stub from an A.0 move that never happened, plus a zero-consumer perf::query/PerfReport API
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:862:**Claim**: blit-app/src/transfers/remote_remote_direct.rs contains only a two-line comment claiming the code 'Moved from blit-cli ... in a later A.0 commit' that never landed (the live relay is still in blit-cli), and diagnostics::perf::query()/PerfReport were built for a TUI consumer that chose a different module and have zero callers.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:864:**Mechanism**: (1) The entire blit-app stub file is the comment '//! Transfer shape: remote_remote_direct. Moved from blit-cli/src/transfers/remote_remote_direct.rs in a later A.0 commit.' — no items — yet it is published as `pub mod remote_remote_direct` (transfers/mod.rs:17) and referenced in dispatch.rs doc links (:17). The live implementation remains in blit-cli: transfers/mod.rs:46 imports run_remote_to_remote_direct and :641 calls run_remote_to_remote_direct_deferred. So a public module advertises a migration that did not happen; an agent following the doc link finds nothing. (2) perf.rs:54 query() bundling PerfReport (:18) was written for 'the TUI F4 diagnostics screen' (comment at :49-53), but rg for perf::query|PerfReport shows only the definitions: blit-cli's diagnostics verb uses the granular fns (blit-cli/src/diagnostics.rs:12-20 calls perf::set_enabled), and the TUI F4 pane consumes blit_app::profile::ProfileReport instead (blit-tui/src/screens/f4.rs:34). Also in this crate: the stale #[allow(dead_code)] on WatchSnapshot (admin/jobs.rs:165) suppresses lints on an enum that is consumed cross-crate (blit-cli/src/jobs.rs:3, :220-245).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:867:- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote_remote_direct.rs:1 — entire file is a two-line 'Moved from ...' comment; exported at transfers/mod.rs:17; live impl still at blit-cli/src/transfers/mod.rs:46 and :641
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:868:- /home/michael/dev/Blit/crates/blit-app/src/diagnostics/perf.rs:54 — pub fn query(limit) -> Result<PerfReport> — rg shows zero consumers; CLI uses granular fns, TUI F4 uses blit_app::profile::ProfileReport (f4.rs:34)
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:871:**Proposed fix**: Owner decision on direction: either finish the A.0 move (relocate blit-cli's remote_remote_direct body into the blit-app stub) or delete the stub and its dispatch.rs doc link. Independently: delete perf::query/PerfReport (TUI F4 will be reworked in Phase 6 anyway) and drop the WatchSnapshot allow.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:893:**Mechanism**: Cargo.toml [dependencies] lists tonic, walkdir, rayon, sysinfo. rg for walkdir|rayon|sysinfo across crates/blit-cli/src and tests hits only the English word 'walkdir' in a comment (tests/local_move_semantics.rs:137). rg for 'use tonic|tonic::' over src/ returns nothing; the only tonic consumer is the fake daemon in tests/remote_remote.rs (:536, :574, :598). The admin logic that needed walkdir/rayon/sysinfo moved to blit-app during the blit-utils absorption (blit-app declares and uses them itself). Effect: needless compile/link work on every CLI build (sysinfo in particular) and a Cargo.toml that falsely advertises the crate does transport and filesystem-walking work. tonic belongs in [dev-dependencies].
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:897:- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:536 — use tonic::transport::Server — the only tonic consumer in the crate; justifies dev-dependency, not dependency
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:901:#### deadcode-core-errors-contradictory-classifier (reviewer: high) — blit-core errors.rs is a dead retry classifier that contradicts the live one and stays publicly exported
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:905:**Claim**: The 154-line blit_core::errors module (TransferError/ErrorCategory/categorize_io_error/TransferResult) has zero consumers anywhere in the workspace, yet is pub-exported and classifies ConnectionRefused/UnexpectedEof/NotConnected as Fatal while the live classifier in blit-app marks all three Retryable.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:914:**Proposed fix**: Delete crates/blit-core/src/errors.rs and the lib.rs:9 export outright (zero callers, no wire impact). If a shared classifier is wanted later, it should be the slice-2 tonic-Status-aware extension of blit-app's live table, not a revival of this one.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:922:**Mechanism**: rg for analyze_warmup_result hits only its definition (auto_tune/mod.rs:26) plus nothing else. rg for determine_tuning shows the sole production caller is remote/tuning.rs:13, which passes None — so warmup_gbps is always None, making the bandwidth-keyed initial_streams (mod.rs:45-55) and tcp_buffer_size/prefetch ladders (mod.rs:57-67) unreachable outside tests (mod.rs:85 passes Some in a test). determine_remote_tuning then overwrites initial_streams/max_streams from its own byte ladder (tuning.rs:27-28) and tcp_buffer_size/prefetch_count (tuning.rs:30-36), so even the None-branch outputs (initial_streams=2, max_streams=8, buffers None) never survive. All remote sizing is the static byte-keyed table in tuning.rs; the module doc (mod.rs:3 'Provides warmup probes') advertises adaptivity the system does not have — a SIMPLE-principle lie in the code. Cross-ref: the queued slice-2 'adaptive windows' transport work is about TCP/HTTP2 windows on channels, not this warmup path; no overlap.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1007:#### duplication-retry-classifier-dead-twin (reviewer: medium) — Dead, publicly exported retry classifier in blit-core contradicts the live one in blit-app on three error kinds
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1011:**Claim**: blit-core/src/errors.rs ships a doc-commented, publicly exported categorize_io_error with zero consumers that classifies ConnectionRefused/UnexpectedEof/NotConnected as Fatal, while the live classifier blit-app/transfers/retry.rs classifies the same three kinds as retryable — a discoverable trap for the next contributor.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1013:**Mechanism**: errors.rs:90-118 marks TimedOut/Interrupted/ConnectionReset/ConnectionAborted/BrokenPipe/WouldBlock retryable and explicitly sends UnexpectedEof/NotConnected/ConnectionRefused to Fatal (lines 108-113, 'default to fatal to avoid infinite loops'). retry.rs:35-46 (is_retryable_io_kind) includes ConnectionRefused, UnexpectedEof, and NotConnected as retryable and omits Interrupted/WouldBlock. rg for categorize_io_error/crate::errors/blit_core::errors across all crates returns zero hits outside errors.rs itself, yet lib.rs:9 exports `pub mod errors`. Anyone importing the visible blit-core classifier (the 'obvious' home for retry policy) gets the opposite decision from production on exactly the transient-connection kinds --retry exists for.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1018:- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — pub mod errors — dead module publicly exported; rg found zero external consumers
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1020:**Proposed fix**: Delete errors.rs (or reduce it to whatever the slice-2 error-chain work actually needs) so exactly one io::ErrorKind classification table exists; if blit-core must own the table for layering reasons, move retry.rs's table down and re-export it, never both.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1058:**Mechanism**: Tallying Status:: constructors across crates/blit-daemon/src gives 116 internal vs 40 invalid_argument, 13 permission_denied, 7 not_found, etc. (~199 total). In the pull handler, plain file io errors — open/stat/read on the requested path — are wrapped as Status::internal(format!("open {}: {}", ...)) at pull.rs:516, 519, 528, 546, 579, 601, 608, so io::ErrorKind::NotFound/PermissionDenied become Internal on the wire. Client code that branches on status.code() to choose wording or remediation (blit-app/src/transfers/remote.rs:709-751 for Unimplemented/Unavailable; blit-app/src/admin/jobs.rs:83-96 for NotFound/FailedPrecondition) can therefore only ever distinguish transport-level or explicitly-coded conditions; every daemon-side io failure lands in the generic fallback arm, and any future code-based retry classification (queued slice-2) is structurally blind to daemon causes.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1063:- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote.rs:709 — code-conditional wording (Unimplemented/Unavailable) — only transport-level codes are ever distinguishable
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1071:**Claim**: blit-core's second error-reporting layer — the Logger trait with error()/copy_done() callbacks and a file-writing TextLogger — is dead: every production instantiation is NoopLogger, and TextLogger is constructed nowhere outside its own file.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1073:**Mechanism**: rg over the workspace shows the only non-test instantiations of the Logger trait are NoopLogger at local_worker.rs:30 and sink.rs:507 (orchestrator.rs:1146/1262 are in test modules), and TextLogger's only mentions are its own definition (logger.rs:18-51). The logger.error(...) calls in the copy engine (file_copy/mod.rs:220, chunked.rs:64) therefore never report anywhere; the errors do also propagate via Err(e), so this is redundancy rather than loss — but it is a third coexisting failure-text mechanism (alongside eyre and the backend-less log facade) that threads a &dyn Logger parameter through copy_file/parallel/chunked signatures while contributing nothing, and it invites a future caller to rely on TextLogger for an rsync-style --log feature that is not actually wired to anything.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1099:#### tests-fake-server-config-skew (reviewer: medium) — All in-process tonic test servers omit the production HTTP/2 keepalive config, so wire tests exercise a non-production server
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1103:**Claim**: The three in-process gRPC servers used by tests are bare Server::builder() while the production daemon sets http2_keepalive_interval(30s)/timeout(20s), so the only wire-level client test harness can never catch a keepalive-interaction regression — exactly the axis the queued slice-2 transport work (client keepalive, adaptive windows, decode limits) is about to change.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1105:**Mechanism**: Production: blit-daemon/src/main.rs:136-139 builds the server with http2_keepalive_interval(Some(30s)) and http2_keepalive_timeout(Some(20s)) per the 2026-05-23 owner decision quoted in the comment block above it. Tests: remote_remote.rs:540 and :578 (fake unimplemented/rejecting servers) and blit-core/tests/pull_sync_with_spec_wire.rs:201 (the SpyServer the PullSync spec wire-contract test runs the real client against) all call bare Server::builder() with no keepalive. When slice-2 lands client-side keepalive/window/decode settings, the one test that validates client wire behavior (pull_sync_with_spec_wire) will validate it against a server shaped differently from every production daemon, and a server/client keepalive mismatch (e.g. server GOAWAY on too-frequent client PINGs) would be invisible to cargo test. Note: this skew matters only for the in-process fakes — the spawned-daemon harnesses run the real production server config.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1108:- /home/michael/dev/Blit/crates/blit-daemon/src/main.rs:137 — production keepalive: http2_keepalive_interval(30s) / http2_keepalive_timeout(20s)
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1109:- /home/michael/dev/Blit/crates/blit-core/tests/pull_sync_with_spec_wire.rs:201 — bare Server::builder() — the wire-contract spy server lacks production keepalive
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1112:**Proposed fix**: Extract a single 'production-shaped server builder' (a small pub fn in blit-core or a daemon-exported helper returning the configured Server::builder()) and use it in main.rs and all three test servers, so test/prod server config cannot drift. Do this as part of, or immediately before, the queued slice-2 transport slice.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1118:**Claim**: The shared harness captures the daemon's stderr into a pipe that no code ever drains or prints, so when the daemon fails to start the test panics with only 'daemon failed to listen on {port}' while the real cause (config parse error, bind failure) sits unread in the pipe; the same unread pipe is a latent deadlock once a chatty daemon fills the 64 KiB pipe buffer.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1124:- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:158 — assert!(ready, "daemon failed to listen on {port}") — discards the captured stderr that explains why
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1135:**Mechanism**: crates/blit-core/src/remote/tuning.rs:4-38 defines the tiers (chunk 16/32/64 MiB at 512 MiB / 8 GiB; streams 4/8 up to 24/32 at 32 GiB; tcp_buffer_size and prefetch_count set only at >=512 MiB) and contains no test module (rg '#[test]|mod tests' over the file: zero hits). Integration payload survey: largest files created by any compiled test are 3 MiB (remote_resume.rs:19) and 2 MiB (remote_remote.rs:186, remote_regression.rs:170) — every integration run therefore takes the smallest branch of every tier expression, and a transposed boundary or swapped tier value (a one-character FAST regression affecting all large transfers) would pass cargo test --workspace, clippy, and fmt. The daemon-side pull_stream_count table (service/pull.rs:915-925) is equally unpinned. Cheapest closure is unit tests, not giant fixtures: the functions are pure u64 -> params.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1146:- **boundaries-planner-owns-transport-chunk-heuristic** — Transport-agnostic planner computes its own TCP chunk-size heuristic that disagrees with remote/tuning.rs
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1151:  - kill reason: The headline blank-render impact is unreachable: all four production callers of build_transfer_finished_event pass Some(non-empty) on failure (core.rs:533/591/653 via outcome_from_status at core.rs:1330, which forwards status.message() on every Err; core.rs:801-818 uses hard-coded markers), so the unwrap_or("") at core.rs:336 fires only in tests (core.rs:1790-2037). No Status in blit-daemon is built with an empty message — every construction in util.rs, pull_sync.rs, pull.rs, and push/*.rs uses a literal or format!, and the one raw propagation (push/control.rs:62) yields descriptive tonic transport statuses. The proto's documented empty case (blit.proto:986-993) is the no-outcome drop path, where active_jobs.rs:1065 substitutes the non-empty "cancelled before outcome recorded" marker into recents and the event send at core.rs:541 is never reached, so nothing blank is ever rendered by jobs.rs:336 or blit-tui main.rs:5732-5735 (the latter is the subscribe-stream error path, not the TransferError render). The residual code-drop adds little to self-describing format! messages and overlaps the queued slice-2 tonic-Status error-chain work, which I was instructed to cross-reference rather than re-report.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1163:Method: read the design map Parts 1.1-1.5, 2 (all crate sections), then re-derived every reported mechanism from code reads in this session — every finding's mechanism rests on lines I personally read (file:line in evidence). Excluded per instructions and cross-referenced instead of re-reported: design-1 (CLI pull byte double-count — my boundaries-progress-event finding is the structural contract gap, not the bug), design-2, design-3 (my socket-policy finding covers the option/ownership half of the same call sites, not the missing connect timeout), and all queued slice-2 transport work (channel-builder triplication, client keepalive, max_decoding_message_size, adaptive windows, tonic-Status chain preservation, inert sink chunk_bytes — none re-reported; findings 1 and 3 are adjacent but cover the classifier-ownership and dual-heuristic halves the queue does not). Checked and found clean: workspace dependency directions are acyclic and sensible (blit-daemon -> blit-core only; blit-cli/blit-tui -> blit-app -> blit-core; verified all four Cargo.tomls) — the violations are duplications forced by those directions, not actual cycles; blit-cli's transport boundary is clean (no channel construction in src, admin verbs delegate to blit_app::admin — verified jobs.rs/scan.rs reads plus the map's rg results); the path_safety chokepoint is honored on both destructive delete twins today (safe_join_contained verified at delegated_pull.rs:449 and blit-app remote.rs:58/223 — the R58-F3 fix landed on both sides; my finding is about the duplication that caused the original divergence); blit-core helpers.rs:52-57 and daemon util.rs:153-158 both correctly delegate normalize_relative_path to path_posix (the mandate bypass is at other sites); the daemon push accept path and DataPlaneSession::connect agree on socket policy with each other (the asymmetry is pull-side only); blit-tui builds no gRPC channels (daemons.rs feeds RemoteEndpoint into the shared blit-app path — light-pass verification only, per the TUI rule). Not covered: blit-prometheus-bridge (map reports coherent local policy; I did not independently read it), deep TUI internals (Phase 6 rule), proto/blit.proto message-level boundary questions, change_journal/fs_capability/win_fs platform modules, and Part 1.6-1.9 of the map beyond the progress section (cancellation/platform dimensions presumably owned by sibling agents). The map's 1.4 section claims about errors.rs being dead were independently re-verified by rg this session (zero consumers confirmed).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1167:Verified from code this session (not map trust): every finding's mechanism was re-read at the cited lines. Checked and found CLEAN (good single-owner patterns, valuable for Phase C): TRANSFER_STALL_TIMEOUT is declared once in stall_guard.rs and imported everywhere else (rg showed only use-sites in blit-core pull/data_plane and blit-daemon push/data_plane — the one liveness constant that did consolidate correctly); mDNS is genuinely owned by blit-core/src/mdns.rs (ServiceDaemon::new appears only there); relative_path_to_posix has a single definition in path_posix.rs with both the push client (helpers.rs:52) and daemon (util.rs:153) delegating to it — the normalize_relative_path 'twins' are thin wrappers, not duplication. MAX_TAR_SHARD_BYTES is single-sourced in tar_safety.rs and referenced (not re-typed) by pipeline.rs per the map; I did not re-verify every wire cap. Deliberately NOT reported (cross-referenced instead): the triplicated gRPC channel builder, client keepalive absence, tonic decode limit, and the Status->eyre stringification family (all inside queued slice-2 transport work, STATE.md Queue item 2); unbounded data-plane connects (design-3); the CLI pull byte double-count bug itself (design-1 — I filed only the structural folding-rule duplication around it); orphaned daemon data planes / AbortOnDrop-vs-bare-JoinHandle (design-2 territory). Dropped as low severity: tar-shard 1 MiB reservation duplicated twice; TUI byte-formatter ladder duplicated in f2.rs/f4.rs plus blit-app/display.rs (display-only; TUI light-pass rule); throughput smoothing triplication (per-layer cadences arguably legitimate); mpsc send-error fixed-string family (likely reshaped by queued error-chain work); the '1 MiB' five-constant family (its real risk is the decode-limit invariant, which is queued). Not covered in depth: blit-prometheus-bridge (map reports it self-consistent; spot-checked only), blit-tui internals beyond progress_accum (Phase 6 rule), the double-buffered send/receive loop twins in data_plane.rs (read but judged a FAST/design question for the adaptive-streams landing rather than a consolidation slice), and Windows casefold-key divergence (already tracked as h-paths-2 in docs/audit/findings/inconsistency-paths.md per the map; not re-filed).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1171:Checked and re-derived from code this session: blit-core/src/errors.rs (full read), blit-app/src/transfers/retry.rs (full read), blit-core/src/remote/pull.rs 75-135 (PullSyncError/format_status), push/client/helpers.rs (map_status, [push] prefix), all daemon service files via targeted rg + reads (Status constructor tally: 116 internal / ~199 total; {err} vs {err:#} counts 12 vs ~69), util.rs (full read), delegated_pull.rs 190-260 and 360-375, core.rs outcome_from_status and TransferError event construction, blit-cli/src/jobs.rs watch render paths, blit-app/src/admin/{rm,jobs}.rs, blit-app/src/transfers/remote.rs 698-760, proto/blit.proto error messages, sink.rs log::warn and mpsc sites, logger.rs (full read), workspace-wide rg for log backends (none exist). Found clean: run_with_retries loop logic and its tests (correct budget/classification semantics); format_status itself (the one well-designed converter — finding is that it is private); admin/jobs.rs cancel_job code branching (good pattern, includes code+message); delegated_pull's R37-F1 negotiation-phase preservation via PullSyncError downcast (works as documented); color_eyre installed in blit-cli main; prometheus bridge stderr usage is internally consistent; copy-engine errors propagate correctly despite the noop Logger (verified Err(e) returned at file_copy/mod.rs:215-223). Not covered per instructions: deep TUI internals (only the main.rs:5734 boundary site, kept as light-pass evidence); the queued slice-2 territory — connect-site chain stripping (pull.rs:245, push/client/mod.rs:313), the TODO(audit-h3c-2) retry no-op at pull.rs:780-788, Status→eyre chain preservation, and retry-classifier extension to tonic codes — cross-referenced in findings 4 and 3 rather than re-reported; the three already-filed design-1/2/3 findings. Not exhaustively audited: blit-app TUI-facing transfer progress text, Windows-only win_fs error paths, and the bug-report-style empty-message reachability of every individual tonic transport error variant (mechanism shown via the in-repo format_status guard plus core.rs unwrap_or(\"\") instead).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1175:Checked and re-derived from code this session: remote/tuning.rs + auto_tune/mod.rs (full read), transfer_plan.rs size bins and chunk heuristic, buffer.rs (BufferSizer + BufferPool semantics), copy/file_copy/{resume.rs,chunked.rs}, copy/parallel.rs, fs_enum.rs categorize_files (+ workspace-wide caller search), remote/transfer/{data_plane.rs send+receive paths, pipeline.rs wire caps and receive loop, grpc_fallback.rs, payload.rs}, remote/push/client/mod.rs (pool formula, negotiation, fallback chunk selection), remote/pull.rs (client pull path, bare connect at 1710), blit-daemon service/{pull_sync.rs, pull.rs, push/control.rs, push/data_plane.rs}, proto/blit.proto RPC surface. Clean areas (valuable for Phase C): (1) derive_local_plan_tuning (auto_tune/mod.rs:116-166) is a genuine closed loop — perf-history-driven, clamped, run_kind-filtered (R56-F1); the only real runtime adaptation, and it is sound. (2) Local copy parallelism adapts to hardware: rayon par_iter (copy/parallel.rs:30) and workers=num_cpus (orchestrator/options.rs:99). (3) BufferSizer itself (buffer.rs:27-89) is properly memory-aware with a sane sysinfo fallback. (4) Wire structural caps are fine as shape constants: MAX_WIRE_PATH_LEN 64 KiB, MAX_WIRE_TAR_SHARD_FILES 1Mi, and MAX_WIRE_TAR_SHARD_BYTES correctly single-sourced from tar_safety::MAX_TAR_SHARD_BYTES (pipeline.rs:325-337, the F8 pattern the 64 MiB block cap should copy). (5) DEFAULT_BLOCK_SIZE=1 MiB with client-sends-0-means-default is a reasonable protocol default. Map corrections made: push max streams is 16 (daemon desired_streams caps the negotiation; client min()s it), not 24/32 as the map's headline-4 framing implies; tuning's 64 MiB chunk does NOT flow into MAX_WIRE_BLOCK_BYTES-checked frames (only resume blocks are length-checked there), so the 'tuning bump bricks transfers' coupling is really resume.rs MAX_BLOCK_SIZE ↔ pipeline.rs literal; MAX_PARALLEL_TAR_TASKS=4 governs only the gRPC-fallback receive, not the TCP path (which untars inline per stream). Deliberately not re-reported (queued/filed): 4 MiB tonic decode default / max_decoding_message_size absence, GrpcFallback/GrpcServerStreaming sink chunk clamp inertness and GRPC_FALLBACK_CHUNK_BYTES (queued slice-2 inert-chunk_bytes deletion + decode-size work), client channel keepalive/connect timeouts (slice-2), design-1/2/3. Not covered: blit-prometheus-bridge (map reports it clean of tuning constants; not independently verified), blit-tui internals (light-pass rule; its constants are layout-shape, plus the format_bytes duplication which belongs to another dimension), Windows-specific code paths, and scripts/.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1179:Re-derived every reported mechanism from code this session; map claims were treated as leads only. CHECKED AND CLEAN: (1) Client pull path drop-cancellation — AbortOnDrop correctly wraps every internal spawn in pull.rs (:315/:384 data-plane receiver, :726-732 manifest send task, :953, :1640-1657 worker vec); the push side is the gap (filed). (2) mDNS — discover() is a synchronous flume recv_timeout loop (mdns.rs:195-219) but all async consumers wrap it in spawn_blocking (blit-app/src/scan.rs:22; TUI routes through scan::discover at main.rs:4886); daemon advertise runs at sync startup; the Drop-side blocking recv_timeout(1s) (mdns.rs:88-95) only executes after serve() returns (main.rs:144) so it is moot today — noted, not filed. (3) Local engine lifts — blocking orchestrator via spawn_blocking (blit-app/src/transfers/local.rs:44), rayon manifest enumeration inside spawn_blocking (blit-app/src/transfers/remote.rs:93), daemon delegated_pull WalkDir manifest in spawn_blocking (delegated_pull.rs:549), purge/delete in spawn_blocking (admin.rs:51/:68). (4) pipeline.rs streaming executor — traced dispatcher/worker shutdown: dead worker → dispatcher send fails → returns → senders dropped → remaining workers finish(); no deadlock; bounded per-sink channels give backpressure. (5) TarShardExecutor semaphore (push/data_plane.rs:677-704) — bounded at 4, permits released with tasks, extraction in spawn_blocking. (6) BufferPool::acquire semaphore — permit-on-unwind handled (buffer.rs:239-256, audit-12). (7) select! sites — delegated_pull three-way race is biased handler-first (audit-10, core.rs:762-768); push client loop's select arms are cancel-safe mpsc recv (mod.rs:436-855); TUI discovery select (main.rs:4872) clean. (8) StallGuard covers both TCP directions (transfer/data_plane.rs:31/:68 send; daemon push receive :841; daemon pull/pull_sync via DataPlaneSession::from_stream). (9) recents writer does small sync atomic writes on the runtime (active_jobs.rs:831 → recents_store.rs:74-92) — judged low, not filed. NOT COVERED / EXCLUDED PER INSTRUCTIONS: gRPC-stream liveness on healthy-TCP wedged peers (client pull.rs:1232 RemoteFileStream, push response forwarder recv as a *liveness* issue, daemon control.rs:62 / pull_sync recvs, daemon tx.send flow-control stalls) — all in the queued slice-2 keepalive + cadence-watchdog scope (STATE.md Queue 2, grpc_fallback.rs:50-71); design-1/2/3 sites not re-reported (design-2 cross-referenced from the new fourth detach site); blit-tui internals light pass only (single-owner spawn_blocking input task and discovery task looked clean); daemon graceful-shutdown absence (main.rs:137-145) observed but left to the lifecycle/design dimension. Could not exercise anything at runtime (read-only constraint); all mechanisms are static-analysis derived.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1183:Scope: verified every entry in the design map's Part 2 Dead/abandoned lists for blit-core, blit-daemon, blit-cli, and blit-app, re-deriving each from code (rg caller searches + file reads) rather than trusting the map. VERIFIED-DEAD and reported: blit-core errors.rs, tar_stream.rs, zero_copy.rs, delete.rs, copy/parallel.rs+stats.rs, chunked_copy_file, fs_enum categorize_files/enumerate_symlinks/SymlinkEntry/enumerate_directory_deref_filtered, auto_tune warmup machinery, transfer_payloads_via_control_plane, RemotePullClient::pull + daemon legacy Pull TCP plane (with the pull_sync single-stream FAST consequence and the proto wire-compat owner decision), daemon push upload channel + drain task, daemon dead items behind allow(dead_code) (ModuleOptOut, resolve_contained_wire, acquire_buffer, ActiveJobs::cancel/as_str), CLI --interval-ms, CLI unused deps, blit-app empty remote_remote_direct stub + perf::query/PerfReport + stale WatchSnapshot allow. CHECKED AND FOUND LIVE (clean — valuable for Phase C): manifest.rs (consumed by daemon pull_sync); copy_file/copy_paths_blocking/resume_copy_file/mmap_copy_file (live local fast path via orchestrator.rs:355/:1263, local_worker.rs); scan_remote_files and open_remote_file (live, but force_grpc=true only); build_spec_from_options (live in blit-app, blit-tui, daemon); pull_sync client multi-stream receive machinery (pull.rs:1600-1646 — capable but never fed >1 stream by the daemon, folded into the Pull finding); WatchSnapshot, spawn_progress_ticker, active_jobs snapshot/recent/transfer_id/bytes_counter (live — only their allow annotations are stale); cancel_authorized (live at core.rs); push/data_plane.rs+push/payload.rs re-export shims (alive as indirection; judged low severity, dropped); blit-cli endpoints.rs wrapper module, DeferredPullState/DeferredDelegatedState aliases, rm.rs re-export (alive A.0 shims, low, dropped); tests/blit_utils.rs (runs and partially unique — overlap is a test-hygiene issue, judged low for this dimension); ls.rs defensive unreachable Discovery arm (intentional, low, dropped); buffer.rs BufferPool stats counters (vestigial but low, dropped). NOT RE-REPORTED per instructions: design-1/2/3 findings and all queued slice-2 transport items — notably I confirmed blit-app client::CONNECT_TIMEOUT has zero external consumers but folded that into the queued shared-channel-builder work instead of filing it. blit-tui: light pass only — confirmed it consumes RemotePullClient::build_spec_from_options and blit_app::profile (no TUI-internal dead-code findings filed per the Phase-6 rule). Not covered: blit-prometheus-bridge (map reports no dead list for it; did not independently sweep), Windows-only win_fs paths (cannot exercise; caller search only), and git-history dating of modules (read-only session, relied on map dates only for narrative, not for any claim).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1191:Verified-clean areas (Phase C input): (1) docs/ARCHITECTURE.md blit-core module table (lines 44-64) — every listed module (remote::transfer::{pipeline,source,sink,payload}, orchestrator, mirror_planner, enumeration, copy, checksum, change_journal, tar_stream, auto_tune, perf_predictor, perf_history, fs_capability) exists in crates/blit-core/src; its blit-cli structure section matches the actual files; no ghosts found. (2) AGENTS.md/CLAUDE.md infrastructure claims all check out: .claude/commands/ has all six slash commands, scripts/agent/{context,check-docs,precompact,catchup}.sh and scripts/windows/run-blit-tests.ps1 exist, .agents/ layout (state.md/decisions.md pointer stubs, repo-map.json, skills/{catchup,handoff}) matches §0/§3. (3) payload.rs:67-76 doc comment is the accurate counter-model for the FileStream design and is correct. (4) Plan-doc status hygiene is mostly good: UNIFIED_RECEIVE_PIPELINE.md, LOCAL_TRANSFER_HEURISTICS.md, PIPELINE_UNIFICATION.md, BENCHMARK_10GBE_PLAN.md all correctly marked Historical. (5) README.md, docs/API.md ghost-scans clean (no transfer_engine/TransferFacade/FileStream/heartbeat claims). (6) STATE.md itself is internally consistent and current. Partially refuted map claims corrected in findings: TransferOrchestrator exists (drift-agents-md finding includes the map erratum); WORKFLOW_PHASE_2's fast-path-routing row and tests/integration/local_transfers.rs are real, so only the streaming-planner rows are ghosts. Not covered / deliberately skipped: grpc_fallback.rs:68-71 'handled by HTTP/2 keepalive + cancel-on-disconnect' coverage claim (substantively owned by queued slice-2 watchdog re-scope; only the comment side would be drift and it will be rewritten by that slice); blit-tui internals (light-pass rule — only a perf_history identifier scan done); docs/WHITEPAPER.md, docs/DAEMON_CONFIG.md, docs/perf/remote_remote_benchmarks.md not audited beyond identifier grep; greenfield_plan_v6.md:193 mentions transfer_engine in its Phase 0 port directive (Active plan, intent-language — noted but not filed; fix it alongside the AGENTS.md slice if desired); docs/audit/findings/drift-phases.md (2026-06-04) already documents the WORKFLOW_PHASE_2 streaming-planner gap — my finding 1 confirms it independently and adds the still-unfixed Shipped header + manpage propagation.
REVIEW.md:35:| ue-r2-1h | Delete deprecated `Pull` RPC (+ its `pull_stream_count` ladder) after harvest; port relay onto PullSync | `[x]` | `2a13f53` (+`9f37a7a` baseline/staging-slip, `48c5a11` win-1) + review fix `f6f52d7` |
REVIEW.md:52:| w4-3-daemon-disconnect-racing | Medium | Daemon handlers race tx.closed()+cancel token (delegated_pull's select generalized to resolve_transfer_outcome + resolve_streaming_outcome; 2 live sites — pull spawn closure died with the Pull RPC at ue-r2-1h); false supports_cancellation comment fixed, dispatch policy itself unchanged (flip = open owner question, since decided D-2026-07-04-3 → w4-5). Codex PASS (0 findings) | `[x]` | master | `37d7f91` |
REVIEW.md:53:| w4-5-supports-cancellation-flip | Medium | Flip supports_cancellation for Push/PullSync (owner-authorized D-2026-07-04-3): CancelJob + TUI F2 work on attached transfers; policy-only after w4-3's race wiring (one-predicate flip — Pull history-only stays gated; TUI/CLI needed zero logic changes); contract change exit 2→0 pinned at table + RPC-handler level, authz now covers flipped kinds; every old-policy comment surface updated incl. proto wire-contract doc. Codex NEEDS FIXES (1 Low: module-doc scope log still claimed Pull wired) → fixed `1708075` | `[x]` | master | `05a8b39`+`1708075` |
REVIEW.md:54:| w1-2-data-socket-policy-helper | Medium | Shared configure_data_socket (NODELAY/keepalive/tuned buffers) hoisted to blit-core; pull client connect + daemon push/pull_sync accepts all route through it; pull_sync passes the dial's tcp_buffer_bytes (resize accept reads it live — the computed-and-discarded gap closed); daemon's silently-swallowing twin + socket2 dep deleted. design-3 (connect timeouts) untouched. Codex PASS (0 findings) | `[x]` | master | `16237e2` |
REVIEW.md:55:| w1-3-tcp-keepalive-honesty | Medium | Real TcpKeepalive timing (idle 60s / interval 10s / retries 5) at the single site left after w1-2 (the shared helper; daemon copy already deleted, logs-failure clause satisfied structurally) — dead idle peer detected in ~2 min, not ~2 h; comments now true; socket2 features=["all"] for retries + test getters. Codex PASS (0 findings) | `[x]` | master | `865fc1e` |
REVIEW.md:60:| w2-4-delete-pull-rpc | High | Delete deprecated Pull RPC after w2-3 harvest (owner-decided, wire-breaking OK); port scan_remote_files — absorbed into REV4; delivered as `ue-r2-1h` | `[x]` | master | `2a13f53` |
REVIEW.md:61:| w3-1-memory-aware-buffer-pool | High | BufferPool::for_data_plane(chunk_bytes, streams) owns the formula (streams*2+4, shared 64 KiB DATA_PLANE_BUFFER_FLOOR) + available/4 memory cap with a 2-buffers-per-stream liveness floor (buffer shrinks, never concurrency — the double-buffered sender holds 2); replaces the 3 pasted sites; elastic paths authorize dial.ceiling_max_streams() up front (closes both "growing the pool live is a W3.1 concern" deferrals); fixes the sysinfo units bug (0.38 returns bytes; old *1024 over-reported memory 1024x, making every cap vacuous); RECEIVE_CHUNK_SIZE comment truth. 8 params-layer pins, mutation-verified. Codex PASS (0 findings) | `[x]` | master | `f49f8f6` |
REVIEW.md:67:| w9-3-test-harness-builder | Medium | TestContext::builder() consolidating 5 harness clones + 5 cli_bin copies; OnceLock daemon build; fake-server keepalive parity | `[ ]` | — | — |
REVIEW.md:74:| w7-6-default-port-pub | Low | RemoteEndpoint::DEFAULT_PORT pub; delete 9031 literals | `[x]` | master | `de04054` |
REVIEW.md:89:| relay-1-subpath-double-join | Low | `--relay-via-cli` with a subpath source scans `sub/sub` (endpoint rel_path joined twice). Pre-existing (deleted Pull-RPC code had the identical join); surfaced by the ue-r2-1h self-review panel; port kept parity, fix deferred | `[ ]` | — | — |
REVIEW.md:91:| design-1-cli-pull-byte-double-count | Medium | CLI pull progress double-counts bytes on the TCP data plane (producer reports both Payload and FileComplete with full bytes; CLI fold adds both). From design map §1.6, hand-verified. Fixed structurally by w6-1 (producer double-emit removed AND FileComplete's bytes field deleted — the class is unrepresentable); graded within the w6-1 codex round | `[x]` | master | `8fd8978` |
REVIEW.md:93:| design-3-unbounded-data-plane-connects | Medium | Both TCP data-plane connects lacked timeouts (audit-2 fix never reached the data plane); hung 60-127s on black-holed ports. Fixed: shared `socket::dial_data_plane` (bounded connect via DATA_PLANE_ACCEPT_TIMEOUT + w1-2 policy + bounded handshake write via DATA_PLANE_TOKEN_TIMEOUT; TimedOut in the chain → is_retryable transient); both sites collapsed (pull connect_pull_stream incl. resize-ADD, push connect_with_probe incl. elastic). +3 tests incl. deterministic stalled-handshake shape pin, mutation-verified; 1476→1479/0/2. Codex PASS (0 findings) | `[x]` | master | `49dcec6` |
REVIEW.md:94:| w6-2a-delegated-bytesprogress-producer | Medium | Delegated live progress is wire-dead: proto BytesProgress has zero producers — the dst daemon sends Started, silence, then one post-hoc ManifestBatch (delegated_pull.rs:363-369 deliberate 0.1.0 gap, :433). The row atomic is ALREADY fed (core.rs:667); bridge it onto the DelegatedPullProgress stream on the progress tick so CLI footer + TUI delegated pane go live. Client side needs nothing (w6-1 aggregate lane + report_bytes_progress ready). Filed by w6-2 verification | `[ ]` | — | — |
REVIEW.md:95:| w6-2b-daemon-counters-push-pullsync | Medium | Daemon row byte counters stay 0 for push receive (FsTransferSink built without with_byte_progress, push/data_plane.rs:1086 passes None) and pull_sync serve (no counter at all 3 send pipelines, pull_sync.rs:635/:765/:795) — GetState/TransferProgress/TransferComplete all report 0 bytes for 2 of 3 active kinds. Wire job.bytes_counter() through both handlers (independent of 2a, whose delegated counter is already fed). Filed by w6-2 verification | `[ ]` | — | — |
REVIEW.md:145:| audit-1a-delegation-port-zero | Robustness | Reject IANA-reserved source port 0 at the delegation gate before DNS/connect (audit-1 item 5; timeouts deferred to audit-1b + owner decision on idle-timeouts) | `[x]` | `phase5/a1` | `a3147b6` |
REVIEW.md:146:| audit-1b-net-timeouts-keepalive | Robustness | Delegation DNS-resolve (10s) + dst→src connect (30s) timeouts via net_timeout::within; daemon HTTP/2 keepalive (30s/20s) reaps vanished subscribers — owner-decided over idle-close (audit-1 items 1/2/4) | `[x]` | `phase5/a1` | `1d88fea` |
REVIEW.md:157:| audit-9-cancel-auth | Bug | CancelJob now authorizes the caller against the transfer's originating peer (host/IP-only, port-insensitive; loopback + UDS bypass); cross-tenant cancel → PermissionDenied. New CancelOutcome::Unauthorized | `[x]` | `phase5/a1` | `3c5a398` |
REVIEW.md:164:| audit-7b-dead-code | Style | Remove dead compare.rs fns (+orphaned imports), 3 STALE fs_enum allow(dead_code) (fields are live), write-only diagnostics written_at field, empty blit-app progress.rs stub. remote_remote_direct.rs left (live, 285 lines) (audit-7 code-health) | `[x]` | `phase5/a1` | `5a5f735` |
REVIEW.md:175:| audit-15-grpc-missing-connection-timeouts | Robustness | RECOMMEND-DEFER (analysis only, no code): blanket Server::timeout(30s) would kill the 7 streaming RPCs (Subscribe/DelegatedPull/Pull/PullSync/Push/Find/DiskUsage); dead-peer case already covered by audit-1b keepalive. Reviewer to grade decision (Gemini-sourced) | `[x]` | `phase5/a1` | `f0ed9e5` |
REVIEW.md:180:| audit-7d5-extract-config-reload | Refactor | main.rs split part 5: extract Ctrl+R config hot-reload helpers (reload_tui_config I/O wrapper + pure classify_reload) verbatim into crate::config_reload; ReloadBanner stays in main.rs (AppState field), referenced via crate-root path; reload_tui_config re-exported at crate root, classify_reload imported test-locally (sole non-test caller moved with it). Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `4e728b5` |
REVIEW.md:242:- `6eeb214` endpoints — support gates (`a0-endpoints-gates`)
REVIEW.md:243:- `29a2026` final cleanup — drop CLI shim re-exports (`a0-final-cleanup`)
REVIEW.md:263:- `14eeda7` Delegated-pull data-plane byte reporting, including tar-shard/block paths (`c-1b-byte-counter-wiring`)
REVIEW.md:282:- `1fcee97` F1 Daemons pane with Local row, viewport-aware table, and rescan clamp (`a1-3-f1-daemons`)
.review/findings/w9-3-test-harness-builder.md:1:# w9-3-test-harness-builder — one daemon-spawn harness, one cli_bin, OnceLock build, fake-server keepalive parity
.review/findings/w9-3-test-harness-builder.md:12:The daemon-spawn harness (config structs + port pick + `cargo build` +
.review/findings/w9-3-test-harness-builder.md:22:sets HTTP/2 keepalive 30s/20s.
.review/findings/w9-3-test-harness-builder.md:26:runs once per test binary behind a `OnceLock`, and every gRPC server —
.review/findings/w9-3-test-harness-builder.md:32:- **`TestContext::builder()`** (`crates/blit-cli/tests/common/mod.rs`)
.review/findings/w9-3-test-harness-builder.md:40:- **Dual-daemon support**: `spawn_daemon(workspace, name, module_dir,
.review/findings/w9-3-test-harness-builder.md:44:  their private config structs / spawn fns / `wait_for_port` /
.review/findings/w9-3-test-harness-builder.md:52:- **OnceLock build**: `ensure_daemon_built()` wraps the
.review/findings/w9-3-test-harness-builder.md:73:  `pick_unused_port` binds `:0` and drops the probe listener before the
.review/findings/w9-3-test-harness-builder.md:75:  same port — the losing daemon exits on "address in use" and its test
.review/findings/w9-3-test-harness-builder.md:81:  `pick_unused_port` keeps a process-global claimed-port set — cargo
.review/findings/w9-3-test-harness-builder.md:85:  daemon exits during startup (external port steal, config rejection)
.review/findings/w9-3-test-harness-builder.md:105:  builder, spawn primitives, OnceLock build, fake-server scaffold.
.review/findings/w9-3-test-harness-builder.md:106:- `crates/blit-core/src/remote/grpc_server.rs` — **new**: keepalive
.review/findings/w9-3-test-harness-builder.md:126:- +1 test: `grpc_server::keepalive_values_match_owner_decision` pins
.review/findings/w9-3-test-harness-builder.md:133:  HEAD `3d8326b` = 1478/0/2 across 37 suites → this slice = 1479/0/2
.review/findings/w9-3-test-harness-builder.md:136:  (STATE's recorded "1479" baseline for design-3 came from a different
.review/findings/w9-3-test-harness-builder.md:140:  binary that flaked): all green — the port-collision flake that
.review/findings/w9-3-test-harness-builder.md:153:  persists under full-parallel runs after the port registry + OnceLock
.review/findings/w9-3-test-harness-builder.md:156:- The claimed-port set only grows (ports are never released back);
.review/findings/w9-3-test-harness-builder.md:160:  the child-death panic reports honestly.
.review/findings/w9-3-test-harness-builder.md:161:- Client-side gRPC channels still set no HTTP/2 keepalive anywhere —
.review/findings/w9-3-test-harness-builder.md:162:  out of scope here (server-side parity only), noted for the transport

exec
/usr/bin/zsh -lc "git show --format= --find-renames f6e592e -- crates/blit-cli/tests/common/mod.rs | sed -n '1,360p'" in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-cli/tests/common/mod.rs b/crates/blit-cli/tests/common/mod.rs
index 79f09b3..3dbcde4 100644
--- a/crates/blit-cli/tests/common/mod.rs
+++ b/crates/blit-cli/tests/common/mod.rs
@@ -1,19 +1,47 @@
+//! Shared integration-test harness (w9-3 consolidation).
+//!
+//! The single daemon-spawn implementation for every blit-cli test
+//! binary: config serialization, port picking, the once-per-binary
+//! `cargo build`, spawn + readiness poll, plus the shared `cli_bin()`
+//! / `run_with_timeout` helpers and the production-shaped fake-server
+//! scaffold. The per-file clones of this logic (remote_remote,
+//! remote_pull_mirror, remote_checksum_negotiation,
+//! remote_tcp_fallback, and the newer jobs_lifecycle /
+//! readonly_enforcement mini-harnesses) were deleted in its favor —
+//! extend this file instead of re-cloning it
+//! (tests-five-daemon-harness-clones).
+
+// Every test binary compiles this module and each uses a different
+// subset of it, so per-binary dead_code lints would fire on whatever
+// that binary happens not to touch. The blanket allow is the honest
+// setting for a shared harness, not a mask for genuinely dead code.
+#![allow(dead_code)]
+
+use std::collections::HashSet;
 use std::fs;
 use std::net::{TcpListener, TcpStream};
-use std::path::PathBuf;
+use std::path::{Path, PathBuf};
 use std::process::{Command, Stdio};
+use std::sync::{Mutex, OnceLock};
 use std::thread;
 use std::time::Duration;
-use tempfile::tempdir;
 
 use serde::Serialize;
+use tempfile::tempdir;
 use wait_timeout::ChildExt;
 
+// ---------------------------------------------------------------
+// blitd.toml serialization — superset of every knob the deleted
+// clones expressed.
+// ---------------------------------------------------------------
+
 #[derive(Serialize)]
 pub struct DaemonConfig {
     pub daemon: DaemonSection,
     #[serde(rename = "module")]
     pub modules: Vec<ModuleSection>,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub delegation: Option<DelegationSection>,
 }
 
 #[derive(Serialize)]
@@ -27,106 +55,65 @@ pub struct DaemonSection {
 pub struct ModuleSection {
     pub name: String,
     pub path: PathBuf,
-    #[serde(default)]
     pub comment: Option<String>,
-    #[serde(default)]
     pub read_only: bool,
+    /// The daemon defaults this to `true` when the key is absent
+    /// (runtime.rs `default_true`), so serializing an explicit `true`
+    /// is behavior-identical to the pre-w9-3 configs that omitted it.
+    pub delegation_allowed: bool,
 }
 
-pub fn pick_unused_port() -> u16 {
-    TcpListener::bind(("127.0.0.1", 0))
-        .expect("bind probe listener")
-        .local_addr()
-        .expect("listener addr")
-        .port()
-}
-
-#[allow(dead_code)]
-pub struct TestContext {
-    pub _work: tempfile::TempDir,
-    pub workspace: PathBuf,
-    pub daemon_port: u16,
-    pub daemon: ChildGuard,
-    pub cli_bin: PathBuf,
-    pub config_dir: PathBuf,
-    pub module_dir: PathBuf,
+#[derive(Serialize)]
+pub struct DelegationSection {
+    pub allow_delegated_pull: bool,
+    pub allowed_source_hosts: Vec<String>,
 }
 
-impl TestContext {
-    #[allow(dead_code)] // each test binary uses one of the two constructors
-    pub fn new() -> Self {
-        Self::with_read_only(false)
-    }
-
-    /// w9-4: same daemon + module, but the module is exported
-    /// `read_only: true` so tests can exercise the three write gates
-    /// (push, purge, delegated pull). Before this knob existed no
-    /// test config in the workspace could express a read-only module.
-    #[allow(dead_code)] // not every test binary uses the read-only harness
-    pub fn new_read_only() -> Self {
-        Self::with_read_only(true)
-    }
-
-    fn with_read_only(read_only: bool) -> Self {
-        let work = tempdir().expect("tempdir");
-        let workspace = work.path().to_path_buf();
+// ---------------------------------------------------------------
+// Binary discovery + the once-per-binary daemon build.
+// ---------------------------------------------------------------
 
-        let module_dir = workspace.join("module");
-        fs::create_dir_all(&module_dir).expect("module dir");
+fn bin_dir() -> PathBuf {
+    let exe_path = std::env::current_exe().expect("current_exe");
+    exe_path
+        .parent()
+        .expect("test binary directory")
+        .parent()
+        .expect("deps parent directory")
+        .to_path_buf()
+}
 
-        let config_dir = workspace.join("cli-config");
-        fs::create_dir_all(&config_dir).expect("cli config");
+pub fn cli_bin() -> PathBuf {
+    bin_dir().join(if cfg!(windows) { "blit.exe" } else { "blit" })
+}
 
-        let port = pick_unused_port();
-
-        let config = DaemonConfig {
-            daemon: DaemonSection {
-                bind: "127.0.0.1".into(),
-                port,
-                no_mdns: true,
-            },
-            modules: vec![ModuleSection {
-                name: "test".into(),
-                path: module_dir.clone(),
-                comment: None,
-                read_only,
-            }],
-        };
-
-        let config_path = workspace.join("blitd.toml");
-        let toml = toml::to_string(&config).expect("serialize config");
-        fs::write(&config_path, toml).expect("write config");
-
-        let exe_path = std::env::current_exe().expect("current_exe");
-        let deps_dir = exe_path.parent().expect("test binary directory");
-        let bin_dir = deps_dir
-            .parent()
-            .expect("deps parent directory")
-            .to_path_buf();
-
-        let cli_bin = {
-            let name = if cfg!(windows) { "blit.exe" } else { "blit" };
-            bin_dir.join(name)
-        };
-        let daemon_bin = {
-            let name = if cfg!(windows) {
-                "blit-daemon.exe"
-            } else {
-                "blit-daemon"
-            };
-            bin_dir.join(name)
-        };
-        let maybe_target = bin_dir
-            .parent()
-            .and_then(|p| p.file_name())
-            .map(|component| component.to_string_lossy().to_string());
+pub fn daemon_bin() -> PathBuf {
+    bin_dir().join(if cfg!(windows) {
+        "blit-daemon.exe"
+    } else {
+        "blit-daemon"
+    })
+}
 
-        // Ensure daemon is built
-        let mut build = Command::new("cargo");
+/// Build `blit-daemon` at most once per test binary.
+///
+/// The build exists because `cargo test -p blit-cli` does not build
+/// another package's binary; each test binary triggers it itself so
+/// no test depends on suite ordering for the daemon to exist (R16-F1,
+/// `docs/reviews/followup_review_2026-05-02.md`). Pre-w9-3 every
+/// `TestContext::new()` ran its own nested `cargo build` (~75 per
+/// full-suite run), all contending for cargo's build-dir lock — the
+/// OnceLock keeps the per-process independence guarantee while paying
+/// the subprocess cost once per binary
+/// (tests-per-test-cargo-build-subprocess).
+pub fn ensure_daemon_built() {
+    static DAEMON_BUILT: OnceLock<()> = OnceLock::new();
+    DAEMON_BUILT.get_or_init(|| {
         let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
             .join("../..")
             .canonicalize()
             .expect("workspace root");
+        let mut build = Command::new("cargo");
         build.current_dir(workspace_root);
         build
             .arg("build")
@@ -134,6 +121,12 @@ impl TestContext {
             .arg("blit-daemon")
             .arg("--bin")
             .arg("blit-daemon");
+        // Cross-target layout (target/<triple>/debug): the triple must
+        // be passed through or the daemon lands in the wrong directory.
+        let maybe_target = bin_dir()
+            .parent()
+            .and_then(|p| p.file_name())
+            .map(|component| component.to_string_lossy().to_string());
         if let Some(triple) = maybe_target {
             if triple != "target" {
                 build.arg("--target").arg(triple);
@@ -146,46 +139,260 @@ impl TestContext {
             String::from_utf8_lossy(&output.stdout),
             String::from_utf8_lossy(&output.stderr)
         );
+    });
+}
 
-        let daemon_child = Command::new(&daemon_bin)
-            .arg("--config")
-            .arg(&config_path)
-            .arg("--bind")
-            .arg("127.0.0.1")
-            .arg("--port")
-            .arg(port.to_string())
-            .stdin(Stdio::null())
-            .stdout(Stdio::null())
-            .stderr(Stdio::piped()) // Capture stderr for debugging
-            .spawn()
-            .expect("spawn daemon");
-        let daemon = ChildGuard::new(daemon_child);
-
-        let mut ready = false;
-        for _ in 0..50 {
-            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
-                ready = true;
-                break;
-            }
-            thread::sleep(Duration::from_millis(100));
+// ---------------------------------------------------------------
+// Daemon spawn primitive + TestContext builder.
+// ---------------------------------------------------------------
+
+/// Pick a port no other test in THIS process has been handed.
+///
+/// The probe listener is dropped before the daemon binds, so the OS
+/// can hand the same port to two parallel tests probing in the same
+/// window — the losing daemon exits on "address in use" and its test
+/// then talks to the *winner's* daemon (empty/wrong module, spurious
+/// failures). Pre-w9-3 the per-test `cargo build` accidentally
+/// serialized bring-ups and mostly hid this; the OnceLock build makes
+/// parallel probes routine, so uniqueness is enforced with a
+/// process-global claimed-set. Cargo runs test binaries sequentially,
+/// so per-process uniqueness is exactly the needed scope; collisions
+/// with unrelated system processes remain possible but are caught by
+/// the child-death check in `spawn_daemon`.
+pub fn pick_unused_port() -> u16 {
+    static CLAIMED: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
+    let claimed = CLAIMED.get_or_init(|| Mutex::new(HashSet::new()));
+    loop {
+        let port = TcpListener::bind(("127.0.0.1", 0))
+            .expect("bind probe listener")
+            .local_addr()
+            .expect("listener addr")
+            .port();
+        let mut claimed = claimed.lock().expect("claimed-port set");
+        if claimed.insert(port) {
+            return port;
         }
-        assert!(ready, "daemon failed to listen on {port}");
+    }
+}
+
+/// Poll until something listens on `127.0.0.1:port` (50 × 100 ms).
+pub fn wait_for_port(port: u16, label: &str) {
+    for _ in 0..50 {
+        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
+            return;
+        }
+        thread::sleep(Duration::from_millis(100));
+    }
+    panic!("{label} failed to listen on {port}");
+}
 
-        Self {
+/// Per-daemon knobs — everything the deleted harness clones existed
+/// to express. (`delegation_allowed` on the module stays hardcoded
+/// `true`, the daemon's own default; no test exercises `false` today
+/// — add a knob here when one does.)
+#[derive(Clone, Default)]
+pub struct DaemonOptions {
+    /// Export the module `read_only = true` (w9-4's three write gates).
+    pub read_only: bool,
+    /// Write a `[delegation]` table enabling delegated pull from
+    /// loopback. Sources must be authorized by IP/CIDR form, not
+    /// hostname — mirrors the production SSRF rule.
+    pub delegation: bool,
+    /// Extra daemon CLI flags (e.g. `--no-server-checksums`,
+    /// `--force-grpc-data`).
+    pub extra_args: Vec<String>,
+}
+
+/// One spawned daemon: its port, its module directory, and the child
+/// guard that kills it on drop.
+pub struct SpawnedDaemon {
+    pub port: u16,
+    pub module_dir: PathBuf,
+    pub daemon: ChildGuard,
+}
+
+/// Spawn one daemon under `workspace`: writes `<name>.toml`, creates
+/// `module_dir` if missing, picks a fresh port, waits for readiness.
+/// `TestContext` routes through this; dual-daemon tests call it (via
+/// `TestContext::spawn_second_daemon`) for their second daemon.
+pub fn spawn_daemon(
+    workspace: &Path,
+    name: &str,
+    module_dir: &Path,
+    opts: &DaemonOptions,
+) -> SpawnedDaemon {
+    ensure_daemon_built();
+
+    fs::create_dir_all(module_dir).expect("module dir");
+    let port = pick_unused_port();
+
+    let config = DaemonConfig {
+        daemon: DaemonSection {
+            bind: "127.0.0.1".into(),
+            port,
+            no_mdns: true,
+        },
+        modules: vec![ModuleSection {
+            name: "test".into(),
+            path: module_dir.to_path_buf(),
+            comment: None,
+            read_only: opts.read_only,
+            delegation_allowed: true,
+        }],
+        delegation: opts.delegation.then(|| DelegationSection {
+            allow_delegated_pull: true,
+            // Loopback sources must be authorized by IP/CIDR form, not
+            // hostname form. This mirrors the production SSRF rule.
+            allowed_source_hosts: vec!["127.0.0.1".to_string()],
+        }),
+    };
+    let config_path = workspace.join(format!("{name}.toml"));
+    let toml = toml::to_string(&config).expect("serialize config");
+    fs::write(&config_path, toml).expect("write config");
+
+    let mut cmd = Command::new(daemon_bin());
+    cmd.arg("--config")
+        .arg(&config_path)

codex
I haven’t run the workspace test suite because this session’s filesystem is read-only and `cargo` would write under `target/`. The review is static plus git/object inspection: diff, line checks, grep for deleted clones, config parser defaults, and test-count grep.
codex
One spec nuance: the second-daemon support is exposed as `TestContext::spawn_second_daemon()` rather than literally a builder knob. Since the deleted clones’ second-daemon behavior is still expressed through the shared context/spawn primitive and all call sites are off their private harnesses, I’m not treating that naming split as a correctness finding.
exec
/usr/bin/zsh -lc 'rg -n "tests-five-daemon-harness-clones|tests-per-test-cargo-build-subprocess|duplication-cli-test-daemon-harness|tests-fake-server-config-skew" docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md | sed -n '198,214p'" in /home/michael/dev/Blit
 succeeded in 0ms:
529:#### duplication-cli-test-daemon-harness — Daemon test harness cloned into three integration-test files despite an existing shared TestContext, and fake tonic servers diverge from production config
686:#### tests-five-daemon-harness-clones — Five copy-pasted daemon-spawn harnesses and five cli_bin re-implementations, already drifted from each other
718:#### tests-per-test-cargo-build-subprocess — Every daemon-backed integration test shells out to `cargo build -p blit-daemon`, serializing the parallel test suite on the cargo lock
1099:#### tests-fake-server-config-skew (reviewer: medium) — All in-process tonic test servers omit the production HTTP/2 keepalive config, so wire tests exercise a non-production server

 succeeded in 0ms:
   198	  blanket `#[cfg(unix)]` from suites with nothing unix-specific; gate only the
   199	  genuinely platform-specific assertions. This is most of "Windows parity is
   200	  untestable". [tests-cfg-unix-gating-blocks-windows-transfer-coverage]
   201	- **W9.2** (M, medium): revive the dead workspace-root `tests/` — relocate
   202	  into `crates/blit-core/tests/` (MirrorPlanner's only semantic tests live
   203	  there, never compiled); delete the can't-run `connection.rs`; fix AGENTS.md §4.
   204	  [tests-dead-workspace-root-test-suite]
   205	- **W9.3** (M, medium): harness consolidation — `TestContext::builder()` with
   206	  the knobs the five clones exist for (extra daemon args, second daemon,
   207	  delegation, read_only), shared `cli_bin()`, OnceLock'd daemon build (replaces
   208	  ~75 nested cargo invocations), fake-server config matching production
   209	  keepalive. [tests-five-daemon-harness-clones, tests-per-test-cargo-build-subprocess,
   210	  duplication-cli-test-daemon-harness, tests-fake-server-config-skew]
   211	- **W9.4** (M, small): read-only module enforcement tests (3 gates, zero
   212	  coverage today, mirror-deletion blast radius). [tests-readonly-module-enforcement-untested]
   213	- **W9.5** (M, medium): jobs/detach lifecycle e2e (Subscribe, watch fallback,
   214	  cancel exit codes) — the regression net W4 needs before changing cancellation.

exec
/usr/bin/zsh -lc "nl -ba docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md | sed -n '1010,1095p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1010	
  1011	**Claim**: blit-core/src/errors.rs ships a doc-commented, publicly exported categorize_io_error with zero consumers that classifies ConnectionRefused/UnexpectedEof/NotConnected as Fatal, while the live classifier blit-app/transfers/retry.rs classifies the same three kinds as retryable — a discoverable trap for the next contributor.
  1012	
  1013	**Mechanism**: errors.rs:90-118 marks TimedOut/Interrupted/ConnectionReset/ConnectionAborted/BrokenPipe/WouldBlock retryable and explicitly sends UnexpectedEof/NotConnected/ConnectionRefused to Fatal (lines 108-113, 'default to fatal to avoid infinite loops'). retry.rs:35-46 (is_retryable_io_kind) includes ConnectionRefused, UnexpectedEof, and NotConnected as retryable and omits Interrupted/WouldBlock. rg for categorize_io_error/crate::errors/blit_core::errors across all crates returns zero hits outside errors.rs itself, yet lib.rs:9 exports `pub mod errors`. Anyone importing the visible blit-core classifier (the 'obvious' home for retry policy) gets the opposite decision from production on exactly the transient-connection kinds --retry exists for.
  1014	
  1015	**Evidence**:
  1016	- /home/michael/dev/Blit/crates/blit-core/src/errors.rs:108 — WriteZero|UnexpectedEof|...|NotConnected|ConnectionRefused => ErrorCategory::Fatal
  1017	- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:41 — ConnectionRefused, UnexpectedEof, NotConnected listed retryable in is_retryable_io_kind
  1018	- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — pub mod errors — dead module publicly exported; rg found zero external consumers
  1019	
  1020	**Proposed fix**: Delete errors.rs (or reduce it to whatever the slice-2 error-chain work actually needs) so exactly one io::ErrorKind classification table exists; if blit-core must own the table for layering reasons, move retry.rs's table down and re-export it, never both.
  1021	
  1022	#### duplication-win-extended-prefix-mount-match (reviewer: medium) — Windows \\?\ prefix-strip plus longest-mount-match duplicated in blit-app and blit-daemon with different tie-break rules, both missing the UNC form
  1023	
  1024	**Principle**: RELIABLE | **Slice**: small
  1025	
  1026	**Claim**: The 'strip \\?\ from canonicalize output, then find the longest matching sysinfo mount point' logic is written twice — diagnostics dump.rs and daemon admin.rs — with identical prefix handling (both miss \\?\UNC\) but different longest-match selection, so df/du and diagnostics can attribute the same path to different disks.
  1027	
  1028	**Mechanism**: dump.rs:167-175 (strip_windows_extended_prefix) and admin.rs:620-628 (inline block) both strip only the literal r"\\?\" prefix; fs::canonicalize on a UNC destination yields \\?\UNC\server\share which neither strips, so mount matching silently fails for UNC paths in both places. The surrounding selection loops differ: dump.rs:151-159 picks the longest mount by OsStr byte length with strict '>', admin.rs:632-643 picks by component count with '>='. For nested mount points (e.g. C:\ vs C:\mnt\data) byte-length and component-count can pick different winners on ties or multi-component mounts, so blit's diagnostics and the daemon's FilesystemStats RPC can disagree about free space for the same path — and any future fix (e.g. adding UNC handling) must be found and landed twice.
  1029	
  1030	**Evidence**:
  1031	- /home/michael/dev/Blit/crates/blit-app/src/diagnostics/dump.rs:170 — strip_prefix(r"\\?\") only; longest match by os_str().len() with '>' at 154-158
  1032	- /home/michael/dev/Blit/crates/blit-daemon/src/service/admin.rs:623 — identical strip inline; longest match by components().count() with '>=' at 636-641
  1033	
  1034	**Proposed fix**: One helper in blit-core (strip extended prefix incl. \\?\UNC\, plus the longest-mount-match selection) consumed by both diagnostics and the daemon admin verbs.
  1035	
  1036	#### duplication-windows-copyfile-twins (reviewer: medium) — Two CopyFileExW wrappers (one dead and strictly worse) plus two windows_copyfile call sites with divergent metadata/thread-local handling
  1037	
  1038	**Principle**: RELIABLE | **Slice**: small
  1039	
  1040	**Claim**: The Windows copy entry point is duplicated at two levels: fs_capability/windows.rs has a second, inferior CopyFileExW wrapper reachable only through a dead trait method, and the two live call sites of windows_copyfile handle the block-clone thread-local flag inconsistently.
  1041	
  1042	**Mechanism**: copy/windows.rs:331-369 (windows_copyfile) attempts ReFS block clone first (line 340), applies the adaptive COPY_FILE_NO_BUFFERING flag (349-351), and falls back to fs::copy on failure (367). fs_capability/windows.rs:230-251 (try_copyfileex) is a bare CopyFileExW with COPYFILE_FLAGS(0) — no clone, no buffering heuristic, no fallback — and rg shows its only route, FilesystemCapability::fast_copy, has zero callers outside the trait impls, so the 'capability abstraction' silently offers a worse copy. At the call-site level, file_copy/mod.rs:43-53 consumes windows::take_last_block_clone_success() and skips preserve_metadata when the clone preserved it; file_copy/chunked.rs:23-29 calls windows_copyfile then preserve_metadata unconditionally and never consumes the thread-local flag, leaving it set for the next caller on that thread.
  1043	
  1044	**Evidence**:
  1045	- /home/michael/dev/Blit/crates/blit-core/src/fs_capability/windows.rs:230 — try_copyfileex: COPYFILE_FLAGS(0), no clone/no-buffering/fallback; fast_copy has zero callers (rg verified)
  1046	- /home/michael/dev/Blit/crates/blit-core/src/copy/windows.rs:340 — windows_copyfile: block clone attempt + adaptive NO_BUFFERING (349) + fs::copy fallback (367)
  1047	- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/mod.rs:45 — consumes take_last_block_clone_success() to skip redundant preserve_metadata
  1048	- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/chunked.rs:25 — second call site: preserve_metadata unconditional, thread-local flag never consumed
  1049	
  1050	**Proposed fix**: Delete fast_copy from the FilesystemCapability trait (or implement it as a call into windows_copyfile), and wrap windows_copyfile + clone-flag + preserve_metadata into one helper so both call sites share the metadata decision.
  1051	
  1052	#### errors-daemon-status-internal-collapse (reviewer: medium) — Daemon collapses 116 of ~199 Status constructions to Status::internal, erasing gRPC code semantics for io errors
  1053	
  1054	**Principle**: RELIABLE | **Slice**: medium
  1055	
  1056	**Claim**: More than half of all daemon error returns use Status::internal regardless of cause, so a missing source file or permission failure on pull crosses the wire as code Internal, making daemon-originated causes undifferentiable to any code()-branching client.
  1057	
  1058	**Mechanism**: Tallying Status:: constructors across crates/blit-daemon/src gives 116 internal vs 40 invalid_argument, 13 permission_denied, 7 not_found, etc. (~199 total). In the pull handler, plain file io errors — open/stat/read on the requested path — are wrapped as Status::internal(format!("open {}: {}", ...)) at pull.rs:516, 519, 528, 546, 579, 601, 608, so io::ErrorKind::NotFound/PermissionDenied become Internal on the wire. Client code that branches on status.code() to choose wording or remediation (blit-app/src/transfers/remote.rs:709-751 for Unimplemented/Unavailable; blit-app/src/admin/jobs.rs:83-96 for NotFound/FailedPrecondition) can therefore only ever distinguish transport-level or explicitly-coded conditions; every daemon-side io failure lands in the generic fallback arm, and any future code-based retry classification (queued slice-2) is structurally blind to daemon causes.
  1059	
  1060	**Evidence**:
  1061	- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:516 — Status::internal(format!("open {}: {}", abs_path.display(), err)) — io NotFound/PermissionDenied collapsed to Internal; same at 546, 579, 601
  1062	- /home/michael/dev/Blit/crates/blit-app/src/admin/jobs.rs:83 — client branches on Code::NotFound / Code::FailedPrecondition — pattern that daemon io-error collapse defeats
  1063	- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote.rs:709 — code-conditional wording (Unimplemented/Unavailable) — only transport-level codes are ever distinguishable
  1064	
  1065	**Proposed fix**: Add one daemon-boundary helper io_to_status(context, io::Error) mapping NotFound→not_found, PermissionDenied→permission_denied, else internal, and convert the pull/push/pull_sync handler io sites to it.
  1066	
  1067	#### errors-logger-trait-permanently-noop (reviewer: medium) — The Logger trait error channel is permanently NoopLogger in production and TextLogger has zero consumers
  1068	
  1069	**Principle**: maintainability | **Slice**: medium
  1070	
  1071	**Claim**: blit-core's second error-reporting layer — the Logger trait with error()/copy_done() callbacks and a file-writing TextLogger — is dead: every production instantiation is NoopLogger, and TextLogger is constructed nowhere outside its own file.
  1072	
  1073	**Mechanism**: rg over the workspace shows the only non-test instantiations of the Logger trait are NoopLogger at local_worker.rs:30 and sink.rs:507 (orchestrator.rs:1146/1262 are in test modules), and TextLogger's only mentions are its own definition (logger.rs:18-51). The logger.error(...) calls in the copy engine (file_copy/mod.rs:220, chunked.rs:64) therefore never report anywhere; the errors do also propagate via Err(e), so this is redundancy rather than loss — but it is a third coexisting failure-text mechanism (alongside eyre and the backend-less log facade) that threads a &dyn Logger parameter through copy_file/parallel/chunked signatures while contributing nothing, and it invites a future caller to rely on TextLogger for an rsync-style --log feature that is not actually wired to anything.
  1074	
  1075	**Evidence**:
  1076	- /home/michael/dev/Blit/crates/blit-core/src/local_worker.rs:30 — let logger = NoopLogger; — production path hardcodes the no-op
  1077	- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/sink.rs:507 — second production NoopLogger hardcode
  1078	- /home/michael/dev/Blit/crates/blit-core/src/logger.rs:18 — TextLogger defined here; zero constructors anywhere else in the workspace
  1079	- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/mod.rs:220 — logger.error("copy", src, …) — always no-op; the error also propagates via Err(e), confirming redundancy
  1080	
  1081	**Proposed fix**: Either delete the Logger trait/TextLogger and the threaded parameters, or wire TextLogger to a real CLI --log flag — decide once; deleting is the SIMPLE-aligned default given errors already propagate.
  1082	
  1083	#### errors-stderr-prefix-babel (reviewer: medium) — Nine different stderr prefixes (plus unprefixed lines) across the binaries — no greppable failure convention
  1084	
  1085	**Principle**: RELIABLE | **Slice**: medium
  1086	
  1087	**Claim**: Failure and warning lines on stderr use at least nine distinct prefixes ('blit:', '[push]', '[pull]', '[pull-data-plane]', '[blitd]', '[warn]'/'[info]', '[failed]'/'[stream-error]'/'[stream-end]', 'blit-prometheus-bridge:', and bare unprefixed text), so neither a user nor a wrapper script can identify blit errors by any single pattern.
  1088	
  1089	**Mechanism**: Re-derived by rg over eprintln! sites: retry announces failures as 'blit: transfer failed…' (retry.rs:70); push-side skips print '[push] skipping…' (helpers.rs:184); pull prints '[pull] …' (pull.rs:428); the daemon mixes '[warn]'/'[info]' (main.rs:35, 91) with '[blitd]' (active_jobs.rs:832) and '[pull-data-plane]' (service/pull.rs:739); blit jobs watch emits '[failed]', '[stream-error]', '[stream-end]' (jobs.rs:336, 351, 360); the bridge uses its crate name (server.rs:224); and other failure lines have no prefix at all ('Cannot cancel transfer…' jobs.rs:601, 'logger write error' logger.rs:38). One binary (blitd) alone uses four conventions. This is the visible face of the smeared error ownership and directly contradicts 'failures are plain'.
  1090	
  1091	**Evidence**:
  1092	- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:70 — 'blit: transfer failed, retrying…'
  1093	- /home/michael/dev/Blit/crates/blit-daemon/src/main.rs:35 — '[warn] {warning}' — daemon also uses [info] (91), [blitd] (active_jobs.rs:832), [pull-data-plane] (service/pull.rs:739)
  1094	- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:336 — '[failed] transfer …' plus [stream-error] (351) and [stream-end] (360) in the same command
  1095	- /home/michael/dev/Blit/crates/blit-prometheus-bridge/src/server.rs:224 — 'blit-prometheus-bridge: scrape failed: {err:#}' — fourth convention family

exec
/usr/bin/zsh -lc "nl -ba docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md | sed -n '1095,1155p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1095	- /home/michael/dev/Blit/crates/blit-prometheus-bridge/src/server.rs:224 — 'blit-prometheus-bridge: scrape failed: {err:#}' — fourth convention family
  1096	
  1097	**Proposed fix**: Pick one convention (suggest '<binary>: ' for human lines, matching the bridge and retry.rs styles) and mechanically converge the eprintln sites; combine with the log-backend slice so log::warn output follows the same convention.
  1098	
  1099	#### tests-fake-server-config-skew (reviewer: medium) — All in-process tonic test servers omit the production HTTP/2 keepalive config, so wire tests exercise a non-production server
  1100	
  1101	**Principle**: RELIABLE | **Slice**: small
  1102	
  1103	**Claim**: The three in-process gRPC servers used by tests are bare Server::builder() while the production daemon sets http2_keepalive_interval(30s)/timeout(20s), so the only wire-level client test harness can never catch a keepalive-interaction regression — exactly the axis the queued slice-2 transport work (client keepalive, adaptive windows, decode limits) is about to change.
  1104	
  1105	**Mechanism**: Production: blit-daemon/src/main.rs:136-139 builds the server with http2_keepalive_interval(Some(30s)) and http2_keepalive_timeout(Some(20s)) per the 2026-05-23 owner decision quoted in the comment block above it. Tests: remote_remote.rs:540 and :578 (fake unimplemented/rejecting servers) and blit-core/tests/pull_sync_with_spec_wire.rs:201 (the SpyServer the PullSync spec wire-contract test runs the real client against) all call bare Server::builder() with no keepalive. When slice-2 lands client-side keepalive/window/decode settings, the one test that validates client wire behavior (pull_sync_with_spec_wire) will validate it against a server shaped differently from every production daemon, and a server/client keepalive mismatch (e.g. server GOAWAY on too-frequent client PINGs) would be invisible to cargo test. Note: this skew matters only for the in-process fakes — the spawned-daemon harnesses run the real production server config.
  1106	
  1107	**Evidence**:
  1108	- /home/michael/dev/Blit/crates/blit-daemon/src/main.rs:137 — production keepalive: http2_keepalive_interval(30s) / http2_keepalive_timeout(20s)
  1109	- /home/michael/dev/Blit/crates/blit-core/tests/pull_sync_with_spec_wire.rs:201 — bare Server::builder() — the wire-contract spy server lacks production keepalive
  1110	- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:540 — bare Server::builder() for the fake unimplemented destination (second copy at :578)
  1111	
  1112	**Proposed fix**: Extract a single 'production-shaped server builder' (a small pub fn in blit-core or a daemon-exported helper returning the configured Server::builder()) and use it in main.rs and all three test servers, so test/prod server config cannot drift. Do this as part of, or immediately before, the queued slice-2 transport slice.
  1113	
  1114	#### tests-harness-stderr-blackhole (reviewer: medium) — TestContext pipes daemon stderr 'for debugging' but never reads it: opaque startup failures now, write-blocking hazard later
  1115	
  1116	**Principle**: RELIABLE | **Slice**: small
  1117	
  1118	**Claim**: The shared harness captures the daemon's stderr into a pipe that no code ever drains or prints, so when the daemon fails to start the test panics with only 'daemon failed to listen on {port}' while the real cause (config parse error, bind failure) sits unread in the pipe; the same unread pipe is a latent deadlock once a chatty daemon fills the 64 KiB pipe buffer.
  1119	
  1120	**Mechanism**: common/mod.rs:145 sets .stderr(Stdio::piped()) with the comment 'Capture stderr for debugging', but no call site reads daemon.child stderr — not the readiness failure path (assert!(ready, ...) at :158 discards it) and not ChildGuard::drop (:205-211, kill+wait only). Result (a): any daemon startup failure surfaces as a bare timeout assertion with zero diagnostic text, the exact 'failures are plain' violation in a place built to debug failures. Result (b): the daemon writes per-connection/per-stream lines to stderr unconditionally (eprintln at push/data_plane.rs:125, :152, :191; pull.rs:661,:717; plus startup module lines main.rs:43-53); a test daemon that serves enough operations to exceed the OS pipe buffer blocks forever inside eprintln!, which then manifests as an unrelated run_with_timeout panic in the CLI under test. The four clone harnesses 'fixed' this by using Stdio::null() (e.g. remote_checksum_negotiation.rs:180), losing diagnostics entirely — both policies are wrong in different directions.
  1121	
  1122	**Evidence**:
  1123	- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:145 — .stderr(Stdio::piped()) // Capture stderr for debugging — never read anywhere
  1124	- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:158 — assert!(ready, "daemon failed to listen on {port}") — discards the captured stderr that explains why
  1125	- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:125 — unconditional per-connection eprintln — the writer side of the unread pipe
  1126	
  1127	**Proposed fix**: In the harness, spawn a thread that drains daemon stderr into an Arc<Mutex<String>>; include the buffer in the readiness-failure panic message and optionally dump it from ChildGuard::drop when thread::panicking(). Apply once in the consolidated harness.
  1128	
  1129	#### tests-tuning-tiers-never-exercised (reviewer: medium) — determine_remote_tuning's size-tier table has no unit tests and every integration payload (max 3 MiB) stays in the smallest tier
  1130	
  1131	**Principle**: FAST | **Slice**: small
  1132	
  1133	**Claim**: The byte-count tier table that decides chunk size, stream counts, TCP buffer size, and prefetch for every remote transfer is pinned by zero tests: tuning.rs has no #[test], and the largest payload in any integration test is 3 MiB — far below the first 128 MiB tier boundary — so every tier above the floor (and the 64 KiB chunk-floor interplay) is dead air in the validation suite.
  1134	
  1135	**Mechanism**: crates/blit-core/src/remote/tuning.rs:4-38 defines the tiers (chunk 16/32/64 MiB at 512 MiB / 8 GiB; streams 4/8 up to 24/32 at 32 GiB; tcp_buffer_size and prefetch_count set only at >=512 MiB) and contains no test module (rg '#[test]|mod tests' over the file: zero hits). Integration payload survey: largest files created by any compiled test are 3 MiB (remote_resume.rs:19) and 2 MiB (remote_remote.rs:186, remote_regression.rs:170) — every integration run therefore takes the smallest branch of every tier expression, and a transposed boundary or swapped tier value (a one-character FAST regression affecting all large transfers) would pass cargo test --workspace, clippy, and fmt. The daemon-side pull_stream_count table (service/pull.rs:915-925) is equally unpinned. Cheapest closure is unit tests, not giant fixtures: the functions are pure u64 -> params.
  1136	
  1137	**Evidence**:
  1138	- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:4 — determine_remote_tuning tier table — no #[test] anywhere in the file
  1139	- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:915 — pull_stream_count second tier table, also unpinned by tests
  1140	- /home/michael/dev/Blit/crates/blit-cli/tests/remote_resume.rs:19 — 3 MiB server file — the largest payload in the entire integration suite, below every tier boundary
  1141	
  1142	**Proposed fix**: Add boundary unit tests for determine_remote_tuning and pull_stream_count (each threshold, one byte below/at it), pinning chunk_bytes, initial/max_streams, tcp_buffer_size, prefetch_count per tier. Coordinate with the queued slice-2/chunk-ownership work so the tests pin the post-consolidation table, not the disagreeing duals.
  1143	
  1144	## Refuted findings (verification kills — recorded so they are not re-found)
  1145	
  1146	- **boundaries-planner-owns-transport-chunk-heuristic** — Transport-agnostic planner computes its own TCP chunk-size heuristic that disagrees with remote/tuning.rs
  1147	  - kill reason: The dual heuristics exist textually (transfer_plan.rs:223-229 vs tuning.rs:5-11), but the claimed live conflict is unreachable. The chunk_bytes==0 fallback at push/client/mod.rs:961 is dead: all three stream_fallback_from_queue callers (mod.rs:495/595/746) pass tuning.chunk_bytes, which is always 16/32/64 MiB (tuning.rs:5-11; determine_tuning passes the default through at auto_tune/mod.rs:39-42), and ensure_remote_tuning (mod.rs:231-234) sets plan_options.chunk_bytes_override before every push-client plan call, so build_plan's override at transfer_plan.rs:229 would return the tuning value even if the branch fired. Every other production path either sets the override (pull.rs:142-143/262-263, pull_sync.rs:501-502/551-552) or never reads planned.chunk_bytes (orchestrator local-mirror via plan_local_mirror, diff_planner.rs:102 — zero chunk_bytes reads in orchestrator.rs; pipeline.rs callers are under #[cfg(test)] at pipeline.rs:418). The planner's internal heuristic is dead code with no consumer, so the claim that chunk size depends on call path is false; the residue is a dead-code cleanup, not a SIMPLE violation with behavioral effect.
  1148	- **errors-status-strip-code-and-empty-render** — 14 eyre!(status.message()) sites drop the gRPC code and render a code-only Status as a completely empty error
  1149	  - kill reason: Mechanism verified (all strip sites exist: e.g. /home/michael/dev/Blit/crates/blit-app/src/admin/rm.rs:30, blit-core/src/remote/pull.rs:307/332/493/507/541, push/client/helpers.rs:48-49; private format_status at pull.rs:116-122; code-preserving outlier at admin/jobs.rs:91-95), but the finding duplicates two already-tracked items. The admin/CLI half is audit finding R3-H12 (AUDIT_REPORT_2026-06-04_R2.md:333-348, kept HIGH at _R3.md:168), whose written remediation is exactly the proposed fix — a shared `status_to_eyre(rpc_name, status)` helper with code preservation used everywhere. The blit-core half is the queued slice-2 "tonic Status → eyre" preservation on the DO-NOT-RE-REPORT list (docs/STATE.md:34), with TODO(audit-h3c-2) at pull.rs:322-329 pinning those exact map_err sites for a chain-preserving conversion that inherently restores the code and fixes the empty-message blank render. The novel residue (empty-message fallback detail, completions.rs:84 site) belongs as annotation to R3-H12/slice-2, not a separate medium finding.
  1150	- **errors-wire-transfer-error-code-drop-empty-render** — Job failure messages cross the wire code-less and possibly empty, and both renderers print them unguarded
  1151	  - kill reason: The headline blank-render impact is unreachable: all four production callers of build_transfer_finished_event pass Some(non-empty) on failure (core.rs:533/591/653 via outcome_from_status at core.rs:1330, which forwards status.message() on every Err; core.rs:801-818 uses hard-coded markers), so the unwrap_or("") at core.rs:336 fires only in tests (core.rs:1790-2037). No Status in blit-daemon is built with an empty message — every construction in util.rs, pull_sync.rs, pull.rs, and push/*.rs uses a literal or format!, and the one raw propagation (push/control.rs:62) yields descriptive tonic transport statuses. The proto's documented empty case (blit.proto:986-993) is the no-outcome drop path, where active_jobs.rs:1065 substitutes the non-empty "cancelled before outcome recorded" marker into recents and the event send at core.rs:541 is never reached, so nothing blank is ever rendered by jobs.rs:336 or blit-tui main.rs:5732-5735 (the latter is the subscribe-stream error path, not the TransferError render). The residual code-drop adds little to self-describing format! messages and overlaps the queued slice-2 tonic-Status error-chain work, which I was instructed to cross-reference rather than re-report.
  1152	- **constants-two-live-chunk-ladders** — Two live, disagreeing chunk-size ladders answer 'what chunk for N bytes' differently depending on call path
  1153	  - kill reason: The cited lines exist, but the liveness mechanism fails. All three callers of stream_fallback_from_queue (push/client/mod.rs:495-502, 595-602, 746-753) pass tuning.chunk_bytes from ensure_remote_tuning, and determine_remote_tuning always yields 16/32/64 MiB (tuning.rs:5-11; auto_tune/mod.rs:39-42 keeps the default when warmup is None), so the chunk_bytes==0 branch at mod.rs:961-962 is unreachable; even if reached, ensure_remote_tuning sets plan_options.chunk_bytes_override (mod.rs:233) before plan_transfer_payloads runs (mod.rs:955), making planned.chunk_bytes equal the tuning value via transfer_plan.rs:229. The only production path where the planner ladder actually computes (local mirror, orchestrator.rs:442-445 leaves override None) never reads planned.chunk_bytes — the sole production consumer is the dead mod.rs:962 branch, and daemon pull paths all override (pull.rs:141-143, 261-263; pull_sync.rs:500-502, 550-552). So there is one live ladder plus a dead duplicate, not two live disagreeing ladders selected by call path; chunked.rs:35-39 is real but is a local-copy buffer rule, not a competing answer on the remote chunk question.
  1154	- **drift-shipped-docs-cite-gitignored-absent-benchmark-logs** — Shipped gate docs (WORKFLOW_PHASE_2.5, WORKFLOW_PHASE_3) cite specific logs/ artifacts as acceptance evidence, but logs/ is gitignored and absent
  1155	  - kill reason: The dead-link mechanism is real (WORKFLOW_PHASE_2.5.md:30,65-91 and WORKFLOW_PHASE_3.md:71 cite logs/ paths; .gitignore:34 ignores logs/; the directory is absent), but the finding's impact claim is false: the load-bearing parity numbers are committed inline next to every dead path (WORKFLOW_PHASE_2.5.md:64-91, e.g. line 65 "3.85 s vs rsync 6.61 s") and duplicated with the full NO-GO-to-GO trail in DEVLOG.md:259-272, so the Shipped verdicts ARE verifiable from the repo. Machine-local raw logs plus tracked summaries was the doc's explicit design (WORKFLOW_PHASE_2.5.md:18 "Summaries go into DEVLOG.md and this workflow document"), and the proposed fix's main action — move summary numbers to a tracked location — is already done. Both docs are historical (WORKFLOW_PHASE_3.md:3-8); the residual "label citations machine-local" is a cosmetic annotation below finding threshold.

exec
/usr/bin/zsh -lc "rg -n \"blit-daemon|daemon_bin|Command::new\\(&?daemon|cargo.*blit-daemon|ensure_daemon_built\" crates/blit-cli/tests -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-cli/tests/remote_checksum_negotiation.rs:10://! property (see `common::ensure_daemon_built`).
crates/blit-cli/tests/common/mod.rs:90:pub fn daemon_bin() -> PathBuf {
crates/blit-cli/tests/common/mod.rs:92:        "blit-daemon.exe"
crates/blit-cli/tests/common/mod.rs:94:        "blit-daemon"
crates/blit-cli/tests/common/mod.rs:98:/// Build `blit-daemon` at most once per test binary.
crates/blit-cli/tests/common/mod.rs:109:pub fn ensure_daemon_built() {
crates/blit-cli/tests/common/mod.rs:121:            .arg("blit-daemon")
crates/blit-cli/tests/common/mod.rs:123:            .arg("blit-daemon");
crates/blit-cli/tests/common/mod.rs:135:        let output = build.output().expect("invoke cargo build for blit-daemon");
crates/blit-cli/tests/common/mod.rs:138:            "cargo build blit-daemon failed:\nstdout:\n{}\nstderr:\n{}",
crates/blit-cli/tests/common/mod.rs:224:    ensure_daemon_built();
crates/blit-cli/tests/common/mod.rs:253:    let mut cmd = Command::new(daemon_bin());

exec
/usr/bin/zsh -lc "rg -n \"TcpListener::bind\\(\\(\\\"127\\.0\\.0\\.1\\\", 0\\)\\)|TcpListener::bind\\(\\\"127\\.0\\.0\\.1:0\\\"\\)|pick_unused_port\" crates/blit-cli/tests crates/blit-core/tests -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/tests/pull_sync_with_spec_wire.rs:190:    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
crates/blit-core/tests/pull_sync_with_spec_wire.rs:610:    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
crates/blit-core/tests/pull_sync_with_spec_wire.rs:981:    let data_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
crates/blit-cli/tests/common/mod.rs:162:pub fn pick_unused_port() -> u16 {
crates/blit-cli/tests/common/mod.rs:166:        let port = TcpListener::bind(("127.0.0.1", 0))
crates/blit-cli/tests/common/mod.rs:227:    let port = pick_unused_port();
crates/blit-cli/tests/common/mod.rs:475:    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind fake server");

exec
/usr/bin/zsh -lc "rg -n \"mod common;\" crates/blit-cli/tests -g '*.rs' | wc -l" in /home/michael/dev/Blit
 succeeded in 0ms:
21

exec
/usr/bin/zsh -lc "rg -n \"run_with_timeout\\(|cli_bin\\(\" crates/blit-cli/tests -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-cli/tests/single_file_copy.rs:15:    let bin = cli_bin();
crates/blit-cli/tests/single_file_copy.rs:21:    run_with_timeout(cmd, Duration::from_secs(30))
crates/blit-cli/tests/remote_tcp_fallback.rs:35:    let output = run_with_timeout(cli_cmd, Duration::from_secs(120));
crates/blit-cli/tests/remote_tcp_fallback.rs:96:    let output = run_with_timeout(cli_cmd, timeout);
crates/blit-cli/tests/remote_remote.rs:204:    let cli_bin = common::cli_bin();
crates/blit-cli/tests/remote_remote.rs:219:    let output = run_with_timeout(cmd, Duration::from_secs(20));
crates/blit-cli/tests/remote_remote.rs:286:    run_with_timeout(cmd, Duration::from_secs(60))
crates/blit-cli/tests/remote_pull_mirror.rs:26:    let output = run_with_timeout(cli_cmd, Duration::from_secs(120));
crates/blit-cli/tests/remote_pull_mirror.rs:78:    let output = run_with_timeout(cmd, Duration::from_secs(120));
crates/blit-cli/tests/remote_pull_mirror.rs:130:    let output = run_with_timeout(cmd, Duration::from_secs(120));
crates/blit-cli/tests/remote_checksum_negotiation.rs:48:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_checksum_negotiation.rs:93:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/readonly_enforcement.rs:51:    let output = run_with_timeout(cmd, Duration::from_secs(60));
crates/blit-cli/tests/readonly_enforcement.rs:88:    let output = run_with_timeout(cmd, Duration::from_secs(60));
crates/blit-cli/tests/readonly_enforcement.rs:116:    let output = run_with_timeout(cmd, Duration::from_secs(60));
crates/blit-cli/tests/readonly_enforcement.rs:158:    let output = run_with_timeout(cmd, Duration::from_secs(60));
crates/blit-cli/tests/local_move_semantics.rs:41:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/local_move_semantics.rs:46:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/local_move_semantics.rs:125:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/local_move_semantics.rs:130:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/local_move_semantics.rs:174:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/local_move_semantics.rs:181:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/local_move_semantics.rs:240:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/local_move_semantics.rs:246:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/local_move_semantics.rs:277:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/local_move_semantics.rs:283:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/local_move_semantics.rs:316:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/local_move_semantics.rs:322:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/local_move_semantics.rs:354:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/local_move_semantics.rs:360:    let output = run_with_timeout(cmd, Duration::from_secs(15));
crates/blit-cli/tests/jobs_lifecycle.rs:38:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/jobs_lifecycle.rs:73:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/jobs_lifecycle.rs:96:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/jobs_lifecycle.rs:185:        run_with_timeout(cmd, Duration::from_secs(60))
crates/blit-cli/tests/diagnostics_dump.rs:15:    let bin = cli_bin();
crates/blit-cli/tests/diagnostics_dump.rs:19:    run_with_timeout(cmd, Duration::from_secs(10))
crates/blit-cli/tests/cli_arg_safety_gates.rs:24:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/cli_arg_safety_gates.rs:28:    let output = run_with_timeout(cmd, Duration::from_secs(15));
crates/blit-cli/tests/cli_arg_safety_gates.rs:129:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/cli_arg_safety_gates.rs:134:    let output = run_with_timeout(cmd, Duration::from_secs(15));
crates/blit-cli/tests/cli_arg_safety_gates.rs:158:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/cli_arg_safety_gates.rs:164:    let output = run_with_timeout(cmd, Duration::from_secs(15));
crates/blit-cli/tests/cli_arg_safety_gates.rs:205:    let mut cmd = Command::new(cli_bin());
crates/blit-cli/tests/cli_arg_safety_gates.rs:211:    let output = run_with_timeout(cmd, Duration::from_secs(15));
crates/blit-cli/tests/remote_regression.rs:60:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_regression.rs:132:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_regression.rs:189:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_regression.rs:210:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/f2_chroot_containment.rs:61:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/f2_chroot_containment.rs:128:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/f2_chroot_containment.rs:189:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_parity.rs:28:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_parity.rs:63:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_parity.rs:120:    let output = run_with_timeout(cli_cmd, Duration::from_secs(120));
crates/blit-cli/tests/remote_parity.rs:168:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_parity.rs:209:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_parity.rs:257:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_transfer_edges.rs:30:    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
crates/blit-cli/tests/remote_transfer_edges.rs:75:    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
crates/blit-cli/tests/remote_transfer_edges.rs:115:    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
crates/blit-cli/tests/remote_transfer_edges.rs:158:    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
crates/blit-cli/tests/remote_transfer_edges.rs:191:    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
crates/blit-cli/tests/remote_transfer_edges.rs:215:    let output = run_with_timeout(cli_cmd, Duration::from_secs(30));
crates/blit-cli/tests/remote_resume.rs:38:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_resume.rs:74:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_resume.rs:112:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_push_single_file.rs:30:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_push_single_file.rs:66:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_move.rs:29:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_move.rs:69:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_move.rs:125:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_move.rs:172:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_push_mirror_safety.rs:70:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/remote_push_mirror_safety.rs:132:    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
crates/blit-cli/tests/blit_utils.rs:19:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:45:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:68:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:95:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:119:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:144:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:173:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:203:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:229:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:253:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:281:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:303:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:327:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:357:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:384:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:412:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:430:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:456:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:489:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:514:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:540:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/blit_utils.rs:566:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:21:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:48:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:88:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:124:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:151:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:175:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:203:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:231:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:255:    let output = run_with_timeout(cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:302:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:336:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:374:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:414:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:454:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/admin_verbs.rs:478:    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
crates/blit-cli/tests/remote_pull_subpath.rs:41:    let output = run_with_timeout(cmd, Duration::from_secs(30));
crates/blit-cli/tests/common/mod.rs:5://! `cargo build`, spawn + readiness poll, plus the shared `cli_bin()`
crates/blit-cli/tests/common/mod.rs:86:pub fn cli_bin() -> PathBuf {
crates/blit-cli/tests/common/mod.rs:353:            cli_bin: cli_bin(),
crates/blit-cli/tests/common/mod.rs:396:pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {

codex
- `crates/blit-cli/tests/common/mod.rs:475` — **Medium** — `spawn_fake_blit_server()` still binds `127.0.0.1:0` outside the process-global claimed-port set. In the same libtest process, a parallel fake server can take a port already handed to `spawn_daemon()` during the probe-to-bind gap; the child-death poll then either flakes the daemon test or can mark the fake listener as daemon-ready. This leaves the port-collision fix incomplete for mixed fake/daemon binaries like `remote_remote` and `jobs_lifecycle`.

VERDICT: NEEDS FIXES (tests not rerun; read-only filesystem).
tokens used
165,521
- `crates/blit-cli/tests/common/mod.rs:475` — **Medium** — `spawn_fake_blit_server()` still binds `127.0.0.1:0` outside the process-global claimed-port set. In the same libtest process, a parallel fake server can take a port already handed to `spawn_daemon()` during the probe-to-bind gap; the child-death poll then either flakes the daemon test or can mark the fake listener as daemon-ready. This leaves the port-collision fix incomplete for mixed fake/daemon binaries like `remote_remote` and `jobs_lifecycle`.

VERDICT: NEEDS FIXES (tests not rerun; read-only filesystem).
