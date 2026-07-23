# Terminal data-plane attribution plan r1 — formal review

- Reviewer: `claude-opus-4-8` via Claude Code `2.1.218`, effort `max`
- Reviewed range:
  `16fb33d785b5ff0d0d7f6b930bf64214c71527df..7954b06a5102cc2e4e485dab45b552211294964e`
- Review session: `ab6df8d0-51d6-4df6-b26a-1fdef23450bc`
- Detached worktree: `/tmp/blit-review.kORBip`
- Raw event stream: `terminal-data-plane-attribution-plan-r1.opus.jsonl`
- Acceptance: **SUPERSEDED**

The reviewer returned a schema-valid clean verdict with the exact base and head,
an empty findings list, and `guard_confirmed: true`. Its docs guard passed at
the reviewed head, failed when the new plan's Status header was mutated
off-vocabulary, and passed again after restoration. The worktree was clean and
removed.

Primary review afterward found that this draft incorrectly compared
payload-only probe bytes with a send outcome that can include Windows metadata
or alternate-data-stream bytes, and overloaded periodic `sample_*` fields with
cumulative terminal meanings. Commit `510d33a2` corrected both issues. This
round is retained as evidence but is not acceptance of the final draft.
