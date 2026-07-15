# Agent procedures (PROTOCOL.md)

**Status**: Active

Single source for the trigger vocabulary defined in `AGENTS.md` §3. Claude Code
slash commands, Antigravity workspace skills, and plain-text triggers in Codex all
resolve here, so behavior is identical across tools. Execute the matching section
exactly; do not improvise steps.

## Invocation rules

Run a procedure's steps in order and do not improvise additional ones. Every
procedure ends by emitting an **attestation line** in this exact format:

    protocol: <name> | steps run: <list> | caps: <met, or which violated>

`<name>` is the procedure (e.g. `catchup`); `steps run:` lists the step numbers
actually executed; `caps:` states whether the procedure's output caps were met,
or names any that were violated.

- This file's text must be in context in the same turn a procedure runs — via
  the command's embedded copy or a fresh read in that turn. Memory of an earlier
  read never counts.
- A procedure ends at its final step. Proposing or staging actions beyond it is
  a violation even if nothing was modified.

---

## catchup
Re-ground in current project state before doing any work.
1. Run `bash scripts/agent/catchup.sh`.
2. Show its output to the owner verbatim — never rephrase,
   reorder, expand, or summarize them.
3. Append exactly one line: `Proposed first action: <one sentence>`.
4. Append the attestation line and stop. No modifications, no further
   plans, until the owner responds.

---

## plan `<topic>`

Turn a talked-through idea into a durable plan before any implementation.

1. Interview the owner: goal, non-goals, constraints, acceptance criteria,
   affected crates/files, risks. Ask focused questions until every template field
   can be filled without guessing. **Each requirement stated in chat must land in
   the draft doc in the same turn it is stated** — write incrementally, not at the
   end.
2. Create `docs/plan/<NAME>.md` from `docs/plan/TEMPLATE.md` with
   `**Status**: Draft`.
3. Slice the work into review-loop-sized slices (one coherent, testable change
   each) in the doc's Slices section.
4. Add the doc to STATE.md's Queue (and to "Authoritative docs" if it will be the
   active plan).
5. Commit the plan doc and run the commit through the synchronous reviewloop
   (`.agents/playbooks/reviewloop.md`; D-2026-07-04-1 includes plan changes and
   D-2026-07-15-1 selects Claude CLI with `--model claude-fable-5 --effort
   max`; docs gate is `bash scripts/agent/check-docs.sh`). Adjudicate and fix
   the accepted findings before surfacing the draft to the owner.
6. **Stop.** No implementation until the owner approves; record approval by
   flipping `**Status**: Draft` → `Active` and adding a DECISIONS.md entry.

---

## decision `<topic>`

Record a settled choice so no future session relitigates or misses it.

1. Append to `docs/DECISIONS.md`:

   ```
   ## D-<YYYY-MM-DD>-<n> — <short title>
   - Decision: <one line>
   - Why: <one line>
   - Supersedes: <doc §/decision ID, or "nothing">
   ```

2. If it supersedes plan text: edit that plan text **now** (rewrite or strike,
   annotate "superseded by D-…"). Do not leave stale text standing.
3. If it changes Now or the Queue: update `docs/STATE.md`.
4. Confirm to the owner with the entry ID.

---

## handoff

Make the current session's state durable for the next one. Run before ending a
session, when context is filling up, or on request.

1. Update `docs/STATE.md`: rewrite **Now**; reorder **Queue**; refresh
   **Blocked** and **Open questions**; prepend a handoff entry with date, HEAD
   sha, and 1–3 lines covering *done / in-flight / exact first action for the
   next session*.
2. Enforce caps: ≤ 200 lines total, ≤ 3 handoff entries. Move pruned material
   into `DEVLOG.md`.
3. If meaningful work landed, append a `DEVLOG.md` entry (newest-first, ISO
   timestamp, same style as existing entries).
4. Run `bash scripts/agent/check-docs.sh`; fix any failures.
5. Commit the doc updates alongside the work they describe (or as
   `Handoff: <date>` if docs-only).

---

## drift `[scope]`

Audit one document against reality. Never run unscoped.

1. Scope = the argument, or ask the owner for one doc or one subsystem.
2. Extract every checkable claim from the doc: behavior, file paths, flags,
   module names, status checkboxes, "X is done/not built" statements.
3. Verify each claim against code and tests (read source; run targeted tests
   where cheap).
4. Triage into three lists and act:
   - **Doc wrong** → fix the doc in this session.
   - **Code wrong** → file a finding per `.review/README.md`, or add to the
     STATE.md Queue if it isn't review-loop material.
   - **Ambiguous / needs owner** → add to STATE.md Open questions.
5. Record any supersessions via the `decision` procedure.
6. Report the three lists to the owner.

---

## slice

Pick up the next unit of review-queue work.

1. Run `catchup` first if you haven't this session.
2. Pick the highest-priority `[ ]` item in `REVIEW.md` and run it through
   the synchronous reviewloop in `.agents/playbooks/reviewloop.md`
   (D-2026-07-04-1 — all code and plan changes, no exceptions;
   D-2026-07-15-1 — invoke Claude CLI with `--model claude-fable-5 --effort
   max`): implement with tests on `master` (no agent branches), pass the
   validation suite, commit, write the finding doc, invoke Claude on the exact
   commit, adjudicate every finding, fix the accepted ones, record the
   harness-identified verdict under `.review/results/`, and update the
   `REVIEW.md` row. No sentinel — the async hand-off is retired.
3. Finish with the `handoff` procedure.
