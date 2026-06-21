# Two-agent code review workflow — setup guide

> **This repo (Blit) — read first.** What follows is the *generic, portable*
> guide. Two project rules override the generic contract below:
> - **No agent-created branches (AGENTS.md §8).** All work happens on `master`
>   (or the owner's active branch). Ignore the `fix/<id>` branch + `git worktree`
>   steps — they are the portable default, not Blit's. Atomic per-commit slices
>   and the `findings/`+`results/` records still apply.
> - **Active variant for the unified-transfer engine:** the synchronous
>   code→GPT-review→fix loop in `docs/agent/GPT_REVIEW_LOOP.md` (D-2026-06-20-6)
>   replaces the async sentinel / `reviewer-wait` hand-off for `ue-r2-*` slices.
>   The async two-agent loop below still governs all other work.
> - **Validation suite** is the three commands in `AGENTS.md` §5
>   (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
>   warnings`, `cargo test --workspace`).

A portable workflow for running a **coder agent** and a **reviewer agent** in parallel on the same repo, with strong verification per fix. Hand this doc to either agent and they should be able to bootstrap the workflow in any git repo.

---

## Why this exists

When two agents collaborate on multi-fix work (security pass, refactor, bug-fix sweep), the naïve loop — "coder pushes, reviewer comments, both update a shared `REVIEW.md`" — has three reliable failure modes:

1. **Stacked WIP.** Coder bundles 5–15 fixes into one undifferentiated working tree. If a regression appears, no clean bisect.
2. **Shared-prose drift.** Both agents write to the same Markdown file; status flips and reviewer comments get tangled.
3. **No interlock.** Coder moves on before reviewer grades the previous fix. By the time the reviewer reopens something, coder has stacked work on top.

This workflow eliminates all three by making **one finding ↔ one branch ↔ one sentinel ↔ one verdict** the atomic unit.

---

## Prerequisites

- A git repo. That's it.
- Two AI agents capable of running shell commands, reading/writing files, and dispatching subagents (or one agent that swaps hats — see "single-agent mode" below).
- A way for the reviewer agent to be woken on file changes — most agent harnesses support a `Monitor`-like tool that emits a notification per stdout line of a long-running script. If not, fall back to polling.

---

## Directory layout

Create this at the repo root:

```
.review/
├── README.md                     The contract (this file's twin, project-specific)
├── SETUP.md                      Optional — this generic setup doc
├── findings/<id>.md              Implementation record per finding
├── ready/<id>.json               Coder → reviewer signal
└── results/
    ├── <id>.verified.json        Reviewer → coder: accepted
    └── <id>.reopened.md          Reviewer → coder: needs fix-ups

REVIEW.md                         (root) Human-readable status index
```

Commit `.review/` to git. The audit trail of `ready/` and `results/` is part of the project's verification history.

---

## Step 1 — Scaffold

```bash
mkdir -p .review/{findings,ready,results}
```

Commit nothing yet. The contract docs come next.

---

## Step 2 — Write `.review/README.md` (the contract)

This is the project-specific contract. Customize the validation suite for your repo; the rest is boilerplate.

```markdown
# Review workflow

Two-agent loop: **Coder** is the implementer, **Reviewer** is the gate.
`REVIEW.md` at the repo root is the human-readable status index;
this directory is the structured handoff channel.

## Layout

(copy the layout block above)

## Branch contract

- **One branch per finding**, named `fix/<id-lowercased>-<short-slug>`
  (e.g. `fix/c-1-scrivener-uuid`).
- **Each branch is the smallest coherent slice** that addresses one
  finding id. No bundling.
- Touch only files declared in `.review/findings/<id>.md` under
  **Files changed**. If overlap is unavoidable, declare it in
  **Known gaps**.
- Use `git worktree add ../<repo>-<id> fix/<id-…>` for parallel work
  without checkout thrash.

## Validation suite

Run from repo root. All must pass before commit.

```bash
# project-specific — examples:
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run lint && npm run build
```

## Coder loop

1. Pick the highest-priority `[ ]` (Open) item in `REVIEW.md`.
2. Create branch + worktree. Implement the fix and write tests.
3. Run the **Validation suite**. Do not commit on failure.
4. Commit with subject `Fix <id>: <one-line summary>` and a body
   mirroring `.review/findings/<id>.md`.
5. Write `.review/findings/<id>.md` with: **What / Approach / Files
   changed / Tests added / Known gaps**.
6. Update `REVIEW.md` row: `[ ]` → `[~]`, link the branch.
7. Atomic sentinel write — use `mktemp` then `mv`:
   ```bash
   tmp=$(mktemp .review/ready/.<id>.json.XXXX)
   cat > "$tmp" <<EOF
   {"id":"<id>","branch":"fix/<id-…>","sha":"<git-rev-parse-HEAD>","ts":"<utc-iso8601>"}
   EOF
   mv "$tmp" .review/ready/<id>.json
   ```
8. Move to the next finding. Do not wait for reviewer verdict to start
   the next branch — but do not stack work on a branch that already has
   a `.review/ready/<id>.json` pending.

## Reviewer loop

Wakes on each new sentinel in `.review/ready/`.

1. Read `.review/ready/<id>.json`, parse `branch` + `sha`.
2. `git checkout <branch>` (or use a separate worktree). Run validation.
3. Dispatch a specialist subagent on the diff `master..<branch>` with
   the finding scope.
4. Write the verdict:
   - **Accepted** → `.review/results/<id>.verified.json`:
     ```json
     {"id":"<id>","sha":"<sha>","ts":"<utc-iso8601>","reviewer":"<name>"}
     ```
     Fast-forward merge into master. Update `REVIEW.md` row to `[x]`.
     Delete `.review/ready/<id>.json`.
   - **Reopened** → `.review/results/<id>.reopened.md` with concrete
     file:line comments. Update `REVIEW.md` row to `[ ]`. Delete
     `.review/ready/<id>.json`. The branch stays so the coder can push
     fix-ups; coder writes a new sentinel after addressing comments.

## WIP limit

- **Strict mode (default)**: at most one branch may have a pending
  sentinel at a time.
- **Faster mode**: multiple sentinels permitted iff each branch's
  `Files changed` is fully disjoint from every other pending branch.

## No broad sweeps

Multi-finding branches are forbidden unless the human explicitly asks
for a sweep (e.g. emergency rollback or coordinated workflow change).
```

---

## Step 3 — Write `REVIEW.md` at the repo root (the status index)

Keep this short. It is the human-readable scoreboard. Details live in `.review/findings/<id>.md`.

```markdown
# Review status

See `.review/README.md` for the workflow contract.
See `.review/findings/<id>.md` for per-finding details.

## Legend
- `[ ]` Open
- `[~]` In progress / pending review
- `[x]` Verified

## Findings

| ID    | Severity | Title                                  | Status | Branch |
|-------|----------|----------------------------------------|--------|--------|
| C-1   | CRITICAL | Scrivener UUID path traversal          | `[ ]`  |        |
| C-2   | CRITICAL | Scrivener FileExtension sanitization   | `[ ]`  |        |
| ...   | ...      | ...                                    | ...    | ...    |
```

---

## Step 4 — Per-finding files

`.review/findings/<id>.md` is written by the coder when they start work on a finding. Template:

```markdown
# <id>: <title>

**Severity**: CRITICAL | HIGH | MEDIUM | LOW
**Status**: Open | In progress | Verified
**Branch**: `fix/<id-lowercased>-<short-slug>`
**Commit**: `<git-sha>` (filled in after commit)

## What

Concise statement of the bug or risk. File:line citations for the
problem. One paragraph.

## Approach

What was done, and *why this fixes the root cause* rather than the
surface symptom. Cite the new functions/files. 2–6 sentences.

## Files changed

- `path/to/file.rs:lines` — what changed
- ...

## Tests added

- `path/to/test.rs::name` — what it asserts
- ...

## Known gaps

Anything the coder is uncertain about, anything explicitly out of
scope, any cross-finding overlap that the reviewer should grade
explicitly. Empty if nothing.

## Reviewer comments

(Reviewer writes here on reopen; coder addresses; sentinel resets.)
```

---

## Step 5 — Reviewer agent: arm the Monitor

This is the wake mechanism. The reviewer agent runs this once (persistent) at session start:

```bash
cd <repo-root> && last=""
while true; do
  current=$(ls .review/ready/*.json 2>/dev/null | xargs -n1 basename 2>/dev/null | sort | tr '\n' ' ')
  for name in $current; do
    case " $last " in
      *" $name "*) ;;
      *) echo "READY: $name" ;;
    esac
  done
  last="$current"
  sleep 5
done
```

Portable across macOS and Linux (no `inotify-tools` or `fswatch` dependency). Each new sentinel produces one `READY: <id>.json` notification.

**Filter coverage**: the loop emits on every new file. If your harness's `Monitor` requires a coverage check, that's the entire vocabulary — there is no "failure" state to also surface, because failure for the reviewer is silence, which is handled by periodic human check-in or a separate heartbeat.

If your harness does not support a persistent monitor, fall back to a low-frequency cron (e.g. every 5 min) that runs the same scan logic and exits.

---

## Step 6 — Reviewer subagent dispatch

When the reviewer wakes on a sentinel:

```
1. Read .review/ready/<id>.json
2. Check out the branch (or worktree it)
3. git diff master..<branch>
4. Choose subagent by domain:
     - security-engineer: path validation, auth, capabilities, deserialization
     - quality-engineer: error handling, locking, concurrency, internals
     - frontend-architect: UI/UX, accessibility, state management
     - system-architect: cross-platform/cross-frontend, workspace
5. Hand the subagent: commit SHA, files changed, finding scope, the
   specific REVIEW.md/finding doc the commit claims to address.
6. Ask only: (a) does the fix address the root cause, (b) are tests
   adequate, (c) any regressions introduced.
7. Use the subagent's verdict to write .review/results/<id>.*
```

---

## Step 7 — Sentinel JSON schema

`.review/ready/<id>.json`:

```json
{
  "id": "C-1",
  "branch": "fix/c-1-scrivener-uuid",
  "sha": "abcdef1234567890",
  "ts": "2026-05-16T09:47:23Z"
}
```

All four fields are required. Reviewer rejects malformed sentinels (writes a `.review/results/<id>.reopened.md` noting the schema violation).

`.review/results/<id>.verified.json`:

```json
{
  "id": "C-1",
  "sha": "abcdef1234567890",
  "ts": "2026-05-16T09:52:11Z",
  "reviewer": "claude"
}
```

`.review/results/<id>.reopened.md`: free-form Markdown with concrete file:line comments. No schema constraints; clarity matters more than structure.

---

## Step 8 — Status state machine

```
                  coder picks finding
[ ] Open  ─────────────────────────────────►  [~] In progress
                                                       │
                                                       │ coder commits
                                                       │ + sentinel
                                                       ▼
                                          [~] In progress (pending)
                                                       │
                                                       │ reviewer wakes
                                                       ▼
                                         ┌──────────────────────────┐
                                         │ reviewer verdict          │
                                         └─┬──────────────────────┬──┘
                                accepted   │                      │   reopened
                                           ▼                      ▼
                                       [x] Verified         [ ] Open
                                       (merged)             (branch retained,
                                                            comments in
                                                            results/<id>.reopened.md)
```

---

## Migrating existing WIP

If the coder has accumulated multi-finding WIP before adopting this workflow:

```bash
git stash                                  # park everything
git stash show -p > /tmp/wip.patch         # inspect
# For each finding id:
git checkout -b fix/<id>-<slug> master
git apply --include='<relevant paths>' /tmp/wip.patch
# or: git checkout -p stash@{0} -- <paths>
# run validation, commit, write sentinel
```

The reviewer does **not** commit on the coder's behalf — it's the coder's responsibility to produce atomic per-finding commits.

For genuinely entangled WIP that resists splitting, document the entanglement in `.review/findings/<id>.md` under **Known gaps** and ship one branch covering both; the reviewer will grade them together but flag the bundling as a process violation, not a code defect.

---

## Anti-patterns (don't do these)

- **Broad sweeps.** "Fix C-1..M-6 in one commit" — kills bisection, violates the atomic-unit rule. Allowed only on explicit human request.
- **Editing `REVIEW.md` prose freely.** It's a status index. Long-form discussion goes in `.review/findings/<id>.md` or `.review/results/<id>.reopened.md`.
- **Skipping the sentinel.** "I committed and assumed Claude would see it." The Monitor watches sentinels, not commits. No sentinel = no review.
- **Stacking new commits on a pending-review branch.** Wait for the verdict or signal it via a fresh sentinel.
- **Reviewer modifying the coder's branch.** Reviewer's job is verdict + merge (or reopen). Reviewer does not push fix-ups; that's the coder's role.

---

## Customization knobs

- **Single-agent mode**: one agent plays both roles, ratcheting between coder hat and reviewer hat. Drop the WIP limit (the agent serializes naturally). Keep the per-finding branches and the sentinel/results audit trail — they still make bisection easy.
- **Multiple coders**: each coder owns disjoint findings. Reviewer enforces the disjoint-files rule via the faster-mode WIP limit. Coders identify themselves in the sentinel JSON (`"coder": "..."`).
- **Multiple reviewers**: reviewer identity goes in `.review/results/<id>.verified.json`. Use a per-reviewer Monitor on `.review/ready/` with a domain filter (e.g. only security findings) to load-balance.
- **Auto-merge**: reviewer can fast-forward merge on accept. For higher-stakes work, have reviewer push a `merge-<id>` branch instead and require a human to merge.

---

## Pulling it together — minimal bootstrap sequence

For any new repo, an agent can run:

```bash
# 1. Scaffold
mkdir -p .review/{findings,ready,results}

# 2. Drop in .review/README.md (the contract — customize the validation suite)
# 3. Drop in REVIEW.md at root (the status index)
# 4. Seed initial findings list in REVIEW.md and stubs in .review/findings/
# 5. Commit:
git add .review REVIEW.md
git commit -m "chore: scaffold two-agent review workflow"

# 6. Reviewer agent: arm the Monitor (Step 5 above)
# 7. Coder agent: pick first [ ], create branch, commit, sentinel
```

That's the whole loop.
