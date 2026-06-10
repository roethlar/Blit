# Harvest Report: blit2, 2026-06-10

Governance rules from this repo that other repos would benefit from.

## Ideas

### Never trust `--merged` after a content-less merge

- **Source:** `AGENTS.md` §8 and `docs/DECISIONS.md` D-2026-06-07-1/-2 — an
  unapproved `git merge -s ours` octopus (commit `c793df2`) recorded two feature
  branches as ancestors of `master` without landing a line of their code; one of
  them does not even build.
- **The rule:** A merge can record ancestry without content, so `git branch
  --merged` and "this branch is already merged" conclusions must be verified
  against content (`git diff <branch> <main>`), never against ancestry alone.
  Agents delete branches only when the owner names them explicitly, and land
  "already-merged" work via cherry-pick or rebase onto fresh commits when the
  diff shows the code never arrived.
- **Why it generalizes:** Any repo that ever had an `-s ours`, `--strategy=ours`,
  or botched octopus merge silently breaks every ancestry-based merged check,
  and an agent acting on those checks deletes unlanded work or "merges" a no-op.
- **Proposed home:** `AGENTS.template.md` (a git-safety bullet) or a note in
  `procedures/migration.md`'s battle-earned-rules guidance.

### CI gate tying code changes to the state file

- **Source:** `.github/workflows/docs-gate.yml` + `scripts/agent/check-docs.sh`,
  earned from the 2026-06-04 audit's drift findings (state smeared across
  TODO/DEVLOG/plan docs/tool memories; see `docs/DECISIONS.md` D-2026-06-06-1).
- **The rule:** Enforce the "files are memory" contract mechanically: a CI job
  fails any push that touches code without touching the current-state file
  (with an explicit `[state: skip]` escape for mechanical changes), and a lint
  script caps the state file's length so it stays a readable entry point rather
  than a journal.
- **Why it generalizes:** Every agent-governed repo relies on the state file
  being updated, and prose rules alone demonstrably drift; a two-line CI check
  converts the convention into an invariant.
- **Proposed home:** `AGENTS.template.md` (state-file section) or a new optional
  snippet in `templates/` that greenfield runs can offer when the repo has CI.
