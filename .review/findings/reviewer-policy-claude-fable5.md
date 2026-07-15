# reviewer-policy-claude-fable5 — use the owner-selected external reviewer

**Status**: In progress — docs committed for independent Claude review.

## What

Current process guidance still mandates Codex and treats Claude review as
forbidden, but the owner explicitly selected Claude Fable 5 at max effort for
every future review dispatch while grandfathering the already-in-flight G12
Grok review.

## Approach

Record D-2026-07-15-1 as the specific reviewer-selection authority. Point the
current process documents at the harness-neutral synchronous reviewloop and
the exact Claude flags. Mark the old GPT/Codex procedure historical without
rewriting its body, so prior review evidence remains intelligible.

## Files changed

- `.agents/repo-guidance.md` — current reviewer policy and workflow pointer.
- `docs/DECISIONS.md` — settled reviewer choice and exact supersession scope.
- `docs/agent/PROTOCOL.md` — current `plan` and `slice` dispatch steps.
- `.review/README.md` — current workflow pointer above the retired async body.
- `docs/agent/GPT_REVIEW_LOOP.md` — historical/do-not-execute banner.
- `docs/STATE.md` — current process entry point.
- `DEVLOG.md` — durable chronological record.

## Tests added

None; documentation/governance only. `scripts/agent/check-docs.sh` and
`git diff --check` are the validation gate.

## Known gaps

Older plan prose and historical records retain their original Codex/Grok
wording. D-2026-07-15-1 explicitly supersedes reviewer-selection nouns in
normative plan boilerplate; historical outcomes are not rewritten.

## Reviewer comments

Pending independent Claude review of the exact committed SHA.
