# AGENTS.md — Blit agent contract

Canonical instructions for every coding agent working in this repo (Claude Code,
Codex, Antigravity/Gemini). Claude Code loads this via the `@AGENTS.md` import in
`CLAUDE.md`; Codex and Antigravity CLI read it natively. Keep this file small —
procedures live in `docs/agent/PROTOCOL.md`, current state lives in `docs/STATE.md`.

## 0. Prime directives

1. **Files are memory; chat is not.** Any requirement, decision, or scope change
   agreed in conversation MUST be written to the relevant file (`docs/STATE.md`,
   `docs/DECISIONS.md`, or the active plan doc) in the same turn it is agreed.
   Assume the conversation can be compacted or lost at any moment.
2. **No code before a written plan.** Implementation work requires either an open
   finding (`REVIEW.md` / `.review/`) or a plan doc in `docs/plan/` with
   `**Status**: Active`. If neither exists, run the `plan` procedure first.
3. **One entry point for state.** On any doubt about "what is true right now",
   read `docs/STATE.md`. Do not reconstruct state from `DEVLOG.md`, `TODO.md`,
   chat history, or tool-local memories.

## 1. Source-of-truth precedence

When documents disagree, higher wins. Never silently follow the loser — flag the
conflict and fix the lower-precedence doc (or open a question in STATE.md).

1. `docs/STATE.md` — what is happening *now* (active slice, queue, blockers)
2. The active plan doc(s) named in STATE.md
3. `REVIEW.md` + `.review/` — review-loop status for in-flight findings
4. `docs/DECISIONS.md` — settled choices and supersessions
5. Everything else in `docs/` — reference or historical; check its `**Status**:` header
6. Code + tests are ground truth for *behavior*; plans are ground truth for
   *intent*. A mismatch is a drift finding, not permission to pick whichever is
   convenient.

Special roles: `DEVLOG.md` is an append-only journal — write to it, never read it
to determine current state. `TODO.md` is the long-horizon backlog — the actionable
queue lives in STATE.md and REVIEW.md. `.serena/memories/` and any tool-local
memory are scratch, never authoritative.

## 2. Session protocol

- **Start:** read `docs/STATE.md` (Claude Code injects it via hook; other tools:
  read it yourself or run `scripts/agent/context.sh`). Then read the active plan
  doc or finding it names. Before large changes, confirm your understanding of
  "Now" in one or two lines.
- **During:** update files as facts change (Prime directive 1). New decision →
  `decision` procedure. Scope change → edit the plan doc in the same turn.
- **Before ending, before compaction, or on request:** run the `handoff`
  procedure. Work not reflected in STATE.md is invisible to the next session.
- **After compaction or resume:** re-read `docs/STATE.md` and the active plan doc
  before taking any action, even if the summary feels sufficient.

## 3. Trigger vocabulary

These words from the owner are commands. Each maps to a procedure in
`docs/agent/PROTOCOL.md` — read that file and execute the matching section.

| Trigger | Effect |
|---|---|
| `catchup` | Re-ground from STATE.md + active docs; summarize now/next/blockers |
| `plan <topic>` | Interview owner, write `docs/plan/<NAME>.md`; no code until Active |
| `decision <topic>` | Record in DECISIONS.md, propagate supersessions |
| `handoff` | Update STATE.md for the next session; prune to caps |
| `drift [scope]` | Audit a doc against code; fix docs, file findings, raise questions |
| `slice` | Pick up the next review finding per `.review/README.md` |

(Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
Antigravity exposes `catchup`/`handoff` as workspace skills in `.agents/skills/`.)

## 4. Project map

- `crates/blit-core/` — core library (enumeration, planner, transfer engine,
  orchestrator); most logic and unit tests live here. New modules get re-exported
  in `crates/blit-core/src/lib.rs`.
- `crates/blit-cli/`, `crates/blit-daemon/` — CLI and daemon binaries; admin verbs
  (scan, ls, find, du, df, rm, completions, profile, list-modules) live in
  `blit-cli` alongside transfer commands.
- `crates/blit-app/`, `crates/blit-tui/` — TUI application layers (Phase 5/6 work).
- `crates/blit-prometheus-bridge/` — metrics bridge.
- `proto/blit.proto` — gRPC definitions; `blit-core`'s build script vendors protoc.
- `tests/` — workspace-level integration tests; `scripts/` — helper tooling.

## 5. Build, test, validation

Branch model: `master` is default; per-finding branches `fix/<id>-<slug>` per
`.review/README.md`.

**Validation suite** — must pass before any commit or review sentinel:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Test count may grow but never drop versus the prior baseline unless the removal is
called out in the finding doc's Known gaps. Windows parity matters: after touching
platform-specific code (`win_fs`, planners), run
`scripts/windows/run-blit-tests.ps1`.

**Docs gate (CI):** a push touching `crates/**` or `proto/**` must also touch
`docs/STATE.md`, unless the commit message contains `[state: skip]` (reserved for
mechanical changes — renames, comment fixes). `scripts/agent/check-docs.sh` must
pass; run it locally before pushing docs changes.

## 6. Style

- Rust edition 2021; format with rustfmt. Modules snake_case, types PascalCase,
  constants SHOUT_CASE; match existing names (`transfer_engine`,
  `TransferOrchestrator`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep tests
  deterministic; capture long logs under `logs/`.

## 7. Commits and docs hygiene

- Commit subject: short imperative ("Add streaming planner heartbeat").
  Review-loop commits: `Fix <id>: <one-line summary>` per `.review/README.md`.
- After meaningful work: append a `DEVLOG.md` entry (newest-first, ISO timestamp)
  and update `docs/STATE.md` — the `handoff` procedure does both.
- Every doc in `docs/plan/` carries a `**Status**:` header, one of:
  `Draft | Active | Shipped | Superseded | Historical`. Superseding a doc requires
  a DECISIONS.md entry naming winner and loser, and an edit to the superseded text.
- `docs/STATE.md` stays ≤ 200 lines with ≤ 3 handoff entries; prune the overflow
  into DEVLOG.md.

## 8. Git safety

These rules are absolute. They exist because an unapproved `git merge -s ours`
octopus (commit `c793df2`) was pushed to `origin/master` without the owner's
consent (see `docs/DECISIONS.md` D-2026-06-07-1).

- **Never, without the owner approving that exact action in the current
  session:** `push`, `push --force` / `--force-with-lease`, `reset --hard`,
  rebase or any history rewrite, `commit --amend` on pushed commits, or the
  deletion of any branch, tag, or ref (local or remote).
- **Branch deletion is by explicit name only.** Never delete by `--merged`,
  pattern, or "looks stale". The owner names the branch; you delete that branch.
- **Before any push:** list the exact local refs, remote refs, and destination
  remotes, then stop and wait for approval. Spell out every ref — no "and the
  rest".
- **`--merged` / `--no-merged` are unreliable in this repo.** The `-s ours`
  octopus made `eafb187` (adaptive-streams-pr3) and `d9d4ec7`
  (adaptive-streams-pr3-resizable, does-not-build) *parents* of `master`, so
  `git branch --merged master` falsely lists them as merged and a plain
  `git merge` of those branches **no-ops without landing any code**. Landing
  adaptive work means cherry-pick or rebase onto new commits, never a merge
  (see D-2026-06-07-2).
- Working-tree edits, local commits on a non-default branch, and read-only
  inspection (`status`/`log`/`diff`/`show`) need no special approval; the gate
  is on anything that publishes, rewrites, or destroys.

## 9. Checkpoints

Only an explicit owner message satisfies a checkpoint or verification step.
Agents report observations; the owner declares pass/fail. Never self-certify a
gate or continue a plan past one because the condition appears met. Approvals
are single-use, step-specific, and never carried across sessions.
