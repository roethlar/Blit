# GPT review loop (code → review → fix)

**Status**: Active
**Created**: 2026-06-20
**Decision**: D-2026-06-20-6
**Applies to**: unified transfer engine slices (`ue-r2-*`, plan
`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`). All other work keeps using
the async two-agent loop in `.review/README.md`.

## Shape

A synchronous, single-driver loop. The coding agent (Claude) writes one
slice, commits it, invokes GPT-5.5 via `codex` to review that commit,
**adjudicates each finding against the actual source and tests**, fixes
the accepted ones, then moves to the next slice. There is no second
human/agent in the loop and no async sentinel hand-off — `codex` is
called inline and its output is read in the same session.

The owner is not a developer; per-slice code-quality sign-off is
delegated to this loop plus the validation suite. Do **not** ask the
owner whether the code "looks good" — that is theater (D-2026-06-20-6).

## Per-slice steps

1. Implement one slice from REV4, in the order/dependencies the plan's
   "Slice dependencies" section gives (`ue-r2-1a` first).
2. **Validation gate** (never skip, never commit on failure; test count
   must not drop):
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
3. Commit to `master` — ungated for this loop (D-2026-06-20-6): no agent
   branches, never push. Subject `ue-r2-XX: <summary>`. Write
   `.review/findings/<id>.md` (What / Approach / Files / Tests / Known
   gaps) and commit it with the slice.
4. **GPT review** of the commit (slow — minutes; run in the background and
   capture to a file):
   ```bash
   codex exec -s read-only \
     -c 'plugins."superpowers@openai-curated".enabled=false' \
     "Review the diff of commit <SHA> (run: git show <SHA>). It implements
      slice <id> of docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md. Check:
      correctness regressions, the slice's acceptance criteria,
      FAST/SIMPLE/RELIABLE, byte-identical / StallGuard / cancellation /
      byte-accounting invariants, and that the test count did not drop.
      Output a concise markdown findings list — each finding with
      file:line, severity, rationale — then a final VERDICT line. Be
      concise; do not invoke skills." \
     > .review/results/<id>.codex.md 2>&1
   ```
   (`codex review --commit <SHA>` is the promptless alternative — it can't
   take a custom prompt, so it can't be pointed at the slice criteria.)
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
   `ue-r2-XX: address review (<n> findings)`), re-run the validation gate,
   append the fix sha to the verdict file.
7. Append one `DEVLOG.md` line: what landed, what GPT caught, what was
   rejected and why, the fix sha.
8. Next slice.

## When to pause for the owner

Proceed autonomously through code→review→fix→next. Pause and surface only
for:
- a genuine design choice REV4 did not settle;
- a GPT finding that implies a **plan change** (edit REV4 + record a
  decision, then continue);
- a blocker, an ambiguity, or a RELIABLE risk needing an owner call;
- an owner-gated checkpoint: **push** (always — §8), and the **10 GbE
  benchmark sign-off** (`ue-1`/`ue-2`, plan-defined).

Never pause merely to have the owner bless code that already passed
validation and review.

## Records (reuses `.review/`, drops the async parts)

- `.review/findings/<id>.md` — implementation record (existing format).
- `.review/results/<id>.codex.md` — raw GPT review output.
- `.review/results/<id>.gpt-verdict.md` — per-finding adjudication + fix
  sha.
- `REVIEW.md` → "Unified transfer engine (REV4)" section — status index.
- **No** `.review/ready/<id>.json` sentinel and **no** `reviewer-wait.sh`:
  those exist to wake a *separate* reviewer agent; this loop calls codex
  inline.

## Environment notes

- `codex` runs headless here (provider `headroom`, model `gpt-5.5`, no
  interactive login). Reviews run read-only sandboxed.
- The `superpowers@openai-curated` codex plugin injects a skill framework
  that derails focused reviews — disable it per-invocation as shown.
- The async two-agent loop is documented in `.review/README.md` +
  `docs/agent/SETUP.md`; this synchronous loop reuses only their
  `findings/` + `results/` records.
- Branch model: AGENTS.md §8 (work on `master`, no agent branches) governs
  over `.review/README.md`'s "branch per finding."
