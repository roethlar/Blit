# otp12-pf1-rigw-harness round 15 — Claude adjudication

- Reviewer: `claude-fable-5` via Claude Code `2.1.210`, effort `max`
- Reviewed range: `af2ffb5fb4c57bfd4b4f18371fd9ace5d7431b39..8e019ef5e948b94a7aca7cb3a8d0be41204742af`
- Code fix: `7bdaf8bda5919f2ed03a17709baf6d4aefabe8e0`
- Retained worktree: `/tmp/blit-review-g15-8e019ef`
- Orchestrator record: `.review/results/otp12-pf1-rigw-harness-r15.claude.json`
- Independent verdict: `ACCEPTED`
- Guard confirmed: `true`

Claude confirmed the live diagnosis: the registered controller fed all four
block records through the loop's standard input, while ordinary Windows SSH
inherited and drained that descriptor during block 1. The loop therefore
reached EOF after 32 valid arms, and the unchanged analyzer correctly rejected
the partial session against its exact 128-row schedule.

G15 adds `-n` only to the noninteractive `wssh` wrapper. No `wssh` caller
supplies remote stdin; `wscp` is unchanged; and the separate batched clock
sampler continues to launch SSH directly with its intentional pipe. Claude's
whole-loop audit found no second schedule-stdin consumer.

Syntax, the complete Bash 3.2 self-test, all 23 analyzer tests, the docs gate,
and diff checks passed. Keeping every G15 test, Claude removed only the
production `-n`; the complete self-test failed exactly on
`noninteractive Windows SSH consumed the registered block schedule`. Exact
reviewed bytes were restored, syntax and the complete self-test returned
green, and the script matched blob
`ae60920a45322f6ad4fa550408118d4d17cb56dc`, SHA-256
`85cda14fbeecb9446b1ad2462f938e5cf397f2f042a0d106690b6332c295a05d`.
The retained worktree ended clean at exact `8e019ef`.

The review found no change to transfer roles, SOURCE/DESTINATION semantics,
role-invariant paths, worker policy, schedule contents, measured-arm timing,
trace schema, analyzer rules, thresholds, Time Machine, or mounts. No endpoint
was contacted and no retained artifact was deleted.
