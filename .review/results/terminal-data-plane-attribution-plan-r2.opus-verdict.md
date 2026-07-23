# Terminal data-plane attribution plan r2 — formal review

- Reviewer: `claude-opus-4-8` via Claude Code `2.1.218`, effort `max`
- Reviewed range:
  `16fb33d785b5ff0d0d7f6b930bf64214c71527df..510d33a25fa045a82b2c5b903a0490d55c6eedd1`
- Review session: `e132f328-cd26-48cf-b242-e3851bf98c26`
- Detached worktree: `/tmp/blit_review_wt`
- Raw event stream: `terminal-data-plane-attribution-plan-r2.opus.jsonl`
- Acceptance: **CLEAN**

The reviewer found no actionable defect and emitted a schema-valid terminal
result with `is_error: false`, the exact reviewed base and head SHAs, an empty
findings list, and `guard_confirmed: true`. It verified every referenced path,
type, field, event, and environment variable against the head tree.

The review independently confirmed both corrections. Existing periodic
`sample_*` values are per-tick deltas, so separate cumulative `terminal_*`
fields preserve their semantics. `SinkOutcome.bytes_written` can include
Windows metadata and alternate-data-stream bytes, so it is not a valid equality
check for payload-only probe totals. `SourceDataPlane::finish` also provides the
claimed post-pipeline-join, pre-`data_plane_complete` insertion point.

The docs gate passed at the reviewed head, failed when the new plan's Status
header was mutated off-vocabulary, and passed after restoration.
`git diff --check` was clean and `docs/STATE.md` remained exactly 200 lines.
Two direct Bash reads were refused by the repository's ptk routing rule before
execution; the reviewer reran the exact checks through ptk's PowerShell route.
The detached worktree finished clean and was removed.
