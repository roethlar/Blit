# Blit review workflow

Two-agent loop: **Coder** is the implementer, **Reviewer** is the
gate. `REVIEW.md` at the repo root is the human-readable status
index; this directory is the structured handoff channel.

For the workflow's design rationale (and onboarding any new agent
into the contract) see `/Users/michael/Dev/SETUP.md`.

## Layout

```
.review/
├── README.md                     This file — the project-specific contract
├── findings/<id>.md              Implementation record per finding
├── ready/<id>.json               Coder → reviewer signal
└── results/
    ├── <id>.verified.json        Reviewer → coder: accepted
    └── <id>.reopened.md          Reviewer → coder: needs fix-ups

REVIEW.md                         (root) Human-readable status index
```

Everything under `.review/` is committed. The audit trail of
`ready/` and `results/` is part of the project's verification
history.

## Validation suite — the green-light gate

Every coder commit MUST pass all three before the sentinel goes
out. Reviewer re-runs them as the first step of grading. Run from
the repo root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Tests must show "passed" with zero failures. Test count may grow
as new tests land, but never drop versus the prior baseline
unless a test was intentionally removed (and the removal is
called out in the finding doc's **Known gaps**).

## Branch model

**Default**: one branch per finding, named
`fix/<id-lowercased>-<short-slug>` (e.g. `fix/c-1-uuid-traversal`).
One coherent slice per branch; no bundling.

**Exception — linear refactor sequences**: a single feature
branch (`phase5/blit-app-extract` is the current example) may
host multiple atomic per-commit slices when the slices form a
dependency chain (slice N requires slice N-1's structure to
exist). Each commit on the branch is its own atomic unit and
gets its own finding doc + sentinel + verdict; the branch
unifies them only because rebasing slice N onto master before
slice N-1 lands would break the build. The reviewer grades
slice by slice, just on a shared branch.

When in doubt, default to per-finding branches.

## Coder loop

1. Pick the highest-priority `[ ]` (Open) item in `REVIEW.md`.
2. Create branch (or, on a linear sequence, use the existing
   feature branch — see exception above). Implement the fix and
   write tests.
3. Run the **Validation suite**. Do not commit on failure.
4. Commit with subject `Fix <id>: <one-line summary>` (or, on a
   linear sequence, the sub-slice's natural commit subject) and a
   body mirroring `.review/findings/<id>.md`.
5. Write `.review/findings/<id>.md` with: **What / Approach /
   Files changed / Tests added / Known gaps**.
6. Update `REVIEW.md` row: `[ ]` → `[~]`, link the branch + commit.
7. Atomic sentinel write — use `mktemp` then `mv`:
   ```bash
   tmp=$(mktemp .review/ready/.<id>.json.XXXX)
   cat > "$tmp" <<EOF
   {"id":"<id>","branch":"<branch>","sha":"$(git rev-parse HEAD)","ts":"$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
   EOF
   mv "$tmp" .review/ready/<id>.json
   ```
8. Commit the sentinel + finding doc + REVIEW.md update on the
   same branch.
9. Move to the next finding. Do not wait for reviewer verdict to
   start the next branch — but do not stack work on a branch that
   already has a `.review/ready/<id>.json` pending without
   refreshing the sentinel.

## Reviewer loop

Wakes on each new sentinel in `.review/ready/`. To arm the
wake-on-sentinel monitor in the reviewer's session:

```bash
cd /Users/michael/Dev/Blit && last=""
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

(Or call this from the agent harness's `Monitor` tool — each new
sentinel produces one `READY: <id>.json` notification.)

Per-sentinel steps:

1. Read `.review/ready/<id>.json`, parse `branch` + `sha`.
2. `git checkout <branch>` (or use a worktree). Run validation.
3. Inspect the diff `<prev>..<sha>` (or `master..<sha>` for
   per-finding branches) with the finding scope in mind.
4. Write the verdict:
   - **Accepted** → `.review/results/<id>.verified.json`:
     ```json
     {"id":"<id>","sha":"<sha>","ts":"<utc-iso8601>","reviewer":"<name>"}
     ```
     Update `REVIEW.md` row to `[x]`. Delete `.review/ready/<id>.json`.
     For per-finding branches: fast-forward merge into master (or
     leave for the coder to merge if higher-stakes).
   - **Reopened** → `.review/results/<id>.reopened.md` with
     concrete file:line comments. Update `REVIEW.md` row to `[ ]`.
     Delete `.review/ready/<id>.json`. The branch stays so the
     coder can push fix-ups; coder writes a new sentinel after
     addressing comments.
5. Commit the verdict file + REVIEW.md update.

## WIP limits

- **Strict (default)**: at most one branch may have a pending
  sentinel at a time.
- **Faster mode**: multiple sentinels permitted iff each
  branch's `Files changed` is fully disjoint from every other
  pending branch.
- **Linear-sequence exception**: a single feature branch may
  have at most one pending sentinel at a time; the coder pauses
  before issuing the next slice's sentinel until the current
  one is graded.

## Anti-patterns

- **Broad sweeps.** "Fix C-1..M-6 in one commit" — kills
  bisection. Allowed only on explicit human request.
- **Editing `REVIEW.md` prose freely.** It's a status index.
  Long-form discussion goes in `.review/findings/<id>.md` or
  `.review/results/<id>.reopened.md`.
- **Skipping the sentinel.** "I committed and assumed the
  reviewer would see it." The Monitor watches sentinels, not
  commits. No sentinel = no review.
- **Stacking new commits on a pending-review branch.** Wait for
  the verdict or refresh the sentinel.
- **Reviewer modifying the coder's branch's code.** Reviewer's
  job is verdict + merge (or reopen). Reviewer's only writes
  are to `.review/results/` and the `REVIEW.md` status column.
  Reviewer does not push code fix-ups; that's the coder's role.

## Identity

- **Coder**: `claude-coder` (the agent running the
  implementation session, currently driving from `claude.ai/code`).
- **Reviewer**: `claude-reviewer` (the agent running the
  parallel review session). Identifies itself in
  `.review/results/<id>.verified.json` as `reviewer`.
