# GPT review loop (code → review → fix)

**Status**: Active
**Created**: 2026-06-20
**Decision**: D-2026-06-20-6 (created, `ue-r2-*` scope);
**D-2026-07-04-1** (widened: this loop now governs **all code changes and
all plan changes** — owner: "no exceptions").
**Applies to**: every code change and every plan/docs change in this
repo. The async two-agent sentinel loop in `.review/README.md` is retired
as the grading mechanism (D-2026-07-04-1); this loop reuses its
`findings/` + `results/` records and the `REVIEW.md` status index.
**Precedence**: where this loop conflicts with `AGENTS.md`,
`docs/STATE.md`, or `docs/DECISIONS.md`, governance wins (AGENTS.md §1).

## Shape

A synchronous, single-driver loop. The coding agent (Claude) writes one
slice, commits it, invokes GPT-5.5 via `codex` to review that commit,
**adjudicates each finding against the actual source and tests**, fixes
the accepted ones, then moves to the next slice. There is no second
human/agent in the loop and no async sentinel hand-off — `codex` is
called inline and its output is read in the same session.

**Codex is the only reviewer** (owner, 2026-07-04): do not add
same-model self-review panels, Claude subagent reviewers, or any other
substitute — the author's model grading its own work is what the
Identity rule forbids and what codex exists to avoid. Claude's only
grading role is adjudicating codex's findings against source.

The owner is not a developer; per-slice code-quality sign-off is
delegated to this loop plus the validation suite. Do **not** ask the
owner whether the code "looks good" — that is theater (D-2026-06-20-6).

## Per-slice steps

1. Implement one commit-sized unit of work from whatever queue governs:
   a `REVIEW.md` design-queue row, a plan slice, a bug fix, or a
   plan/docs change. One coherent slice per commit; no bundling.
2. **Validation gate** (never skip, never commit on failure; test count
   must not drop):
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   For docs/plan-only changes the gate is `bash scripts/agent/check-docs.sh`
   instead (D-2026-07-04-1; `.agents/repo-guidance.md` Verification) —
   the review step below still runs.
3. Commit to `master` — ungated for this loop (D-2026-06-20-6): no agent
   branches, never push. Subject per repo convention (`Fix <id>: <summary>`
   for review-queue rows; short imperative otherwise). For code slices,
   write `.review/findings/<id>.md` (What / Approach / Files / Tests /
   Known gaps) and commit it with the slice.
4. **GPT review** of the commit (slow — minutes; run in the background and
   capture to a file):
   ```bash
   codex exec -s read-only \
     -c 'plugins."superpowers@openai-curated".enabled=false' \
     "Review the diff of commit <SHA> (run: git show <SHA>). It implements
      <id / one-line description; name the spec doc or queue row>. Check:
      correctness regressions, the change's acceptance criteria,
      FAST/SIMPLE/RELIABLE, the invariants relevant to the touched area
      (byte-identical / StallGuard / cancellation / byte-accounting for
      transfer code; internal coherence + no contradiction with
      docs/DECISIONS.md for plan changes), and that the test count did
      not drop. Output a concise markdown findings list — each finding
      with file:line, severity, rationale — then a final VERDICT line. Be
      concise; do not invoke skills." \
     </dev/null > .review/results/<id>.codex.md 2>&1
   ```
   **The `</dev/null` is load-bearing** on codex-cli ≥ 0.142: with a prompt
   arg AND an open stdin, `codex exec` appends stdin as a `<stdin>` block and
   blocks on EOF — a backgrounded review then hangs at 0 CPU indefinitely
   (observed otp-5a, 2026-07-06). Redirecting stdin from `/dev/null` closes it
   so the review starts. (`codex review --commit <SHA>` is the promptless
   alternative — it can't take a custom prompt, so it can't be pointed at the
   slice criteria.)
5. **Adjudicate — the load-bearing step.** GPT is a claim source, not an
   authority: a 60k-token codex-class review this very day produced a
   confident-but-false "two static tables, not three" finding. For each
   finding, verify against source/tests and record a verdict in
   `.review/results/<id>.gpt-verdict.md`:
   - **Accepted** (real) — fix it.
   - **Rejected** (false positive) — cite the file:line that disproves it.
   - **Deferred** (real but out of slice scope) — file a finding or add to
     the STATE queue.
   Sign `reviewer: gpt-5.5` honestly (`.review/README.md` → Identity).
6. Fix the Accepted findings (follow-up commit
   `<id>: address review (<n> findings)`), re-run the validation gate,
   append the fix sha to the verdict file.
7. Append one `DEVLOG.md` line: what landed, what GPT caught, what was
   rejected and why, the fix sha.
8. Next slice.

## When to pause for the owner

Proceed autonomously through code→review→fix→next. Pause and surface only
for:
- a genuine design choice the governing plan/spec did not settle;
- a GPT finding that implies a **plan change** (edit the plan + record a
  decision, then continue);
- a blocker, an ambiguity, or a RELIABLE risk needing an owner call;
- an owner-gated checkpoint: **push** (always — §8), and any plan-defined
  gate (e.g. the 10 GbE benchmark sign-off, `ue-1`/`ue-2`).

Never pause merely to have the owner bless code that already passed
validation and review.

## Records (reuses `.review/`, drops the async parts)

- `.review/findings/<id>.md` — implementation record (existing format).
- `.review/results/<id>.codex.md` — raw GPT review output.
- `.review/results/<id>.gpt-verdict.md` — per-finding adjudication + fix
  sha.
- `REVIEW.md` — status index (whichever section the row lives in).
- **No** `.review/ready/<id>.json` sentinel and **no** `reviewer-wait.sh`:
  those existed to wake a *separate* reviewer agent; this loop calls codex
  inline and is now the only grading mechanism (D-2026-07-04-1).

## Environment notes

- `codex` runs headless here (no interactive login); reviews run read-only
  sandboxed. **The reviewer model is whatever `~/.codex/config.toml` says —
  read it, do not assume.** As of 2026-07-13 it is **`gpt-5.6-sol`** at
  `model_reasoning_effort = "ultra"` (codex-cli 0.144.3); this section
  previously said `gpt-5.5`, and a verdict file was signed with that stale
  name before the drift was caught. Sign every verdict with the model the
  config actually names (`.review/README.md` → Identity).
- The `superpowers@openai-curated` codex plugin injects a skill framework
  that derails focused reviews — disable it per-invocation as shown.
- The retired async two-agent loop is documented in `.review/README.md`
  (historical since D-2026-07-04-1); this synchronous loop reuses only its
  `findings/` + `results/` records and the `REVIEW.md` index.
- Branch model: AGENTS.md §8 (work on `master`, no agent branches) governs
  over `.review/README.md`'s "branch per finding."
