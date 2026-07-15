# otp12-pf1-rigw-harness round 8 — Grok adjudication

- Reviewer: `grok-4.5` via `grok 0.2.101 (5bc4b5dfadcf)`, reasoning `high`
- Reviewed range: `4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..6fb369e3d70f7633ad1d697afeda35abf5e276cb`
- Reviewed: `2026-07-15T13:54:36Z`
- Raw envelope: `.review/results/otp12-pf1-rigw-harness-r8.grok.json`
- Independent verdict: `ACCEPTED`
- Guard confirmed: `true`

The orchestrator verified an `EndTurn` envelope, schema-valid structured
output, exact base and reviewed SHAs, the registered verdict enum, and literal
`guard_confirmed=true`. In a detached disposable worktree, Grok independently
returned Bash syntax and the Bash 3.2 self-test green. Removing only
`GIT_NO_REPLACE_OBJECTS=1`, bypassing the wrapper only for commit/path lookup,
or bypassing it only for blob-content lookup each made the exact replacement
provenance guard red; every restoration returned the self-test green.

Grok independently confirmed that G8 covers provenance HEAD/short/status and
helper path/type/content reads; that the fixture proves an unchanged visible
HEAD and clean ordinary status under active commit and blob replacements; and
that cleanup explicitly deletes and verifies replacement refs before restoring
a clean tree. It also reconfirmed one `Transfer` RPC, SOURCE-send and
DESTINATION-receive semantics under both initiator layouts, role-invariant
endpoint-local paths, and the shared 1→8 worker target without push/pull caps.
The worktree ended at the exact reviewed SHA with clean tracked state and was
removed. No endpoint was contacted.

An attempted same-model Codex review was interrupted and its partial output
discarded after the owner correctly noted that Codex reviewing Codex is not an
independent review. It was not considered or recorded as review evidence.

reviewer: grok-4.5
