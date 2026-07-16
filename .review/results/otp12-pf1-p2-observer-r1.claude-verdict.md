# otp12-pf1-p2-observer — Claude adjudication

- Reviewer: Claude Fable 5 via Claude Code 2.1.211
- Effort: `max`
- Reviewed range: `f01d662351039a4153ee9a64ee6c59c10d29b9b7..713526e8f4cbe61f881a24d1f19cc481d0a8b188`
- Completed: 2026-07-16 03:58:44Z
- Verdict: **ACCEPTED**
- Guard confirmed: `true`
- Raw result: `.review/results/otp12-pf1-p2-observer-r1.claude.json`
- Non-authoritative interrupted attempt: `.review/results/otp12-pf1-p2-observer-r1.claude-attempt1-error.json`

## Envelope validation

The authoritative invocation exited zero with `subtype=success`,
`is_error=false`, `terminal_reason=completed`, the required structured payload,
the exact base and reviewed SHAs, `verdict=accepted`, and
`guard_confirmed=true`. The result contains no material finding. One attempted
`rtk` convenience command was denied after ptk had been excluded; it did not
substitute for the review proof. Claude completed the proof with direct Cargo
commands, and the adjudicator independently confirmed the retained worktree is
clean at the exact reviewed SHA and that the restored production blob matches
HEAD.

The first invocation was operator-interrupted before it returned any verdict
or completed a guard proof. Its error envelope is retained separately and is
not used for adjudication.

## Independent guard proof

Claude first passed the three `small_file_probe` unit tests and the complete
role/carrier observer guard. Keeping test code unchanged, it replaced the
production keyed, ordered shard ID with one constant. The unit guard failed at
the predicted cross-run key-isolation assertion because both IDs became
`constant-shard-id`. Claude restored the exact reviewed production blob, then
reran both focused test commands green. The retained worktree ended clean at
`713526e8f4cbe61f881a24d1f19cc481d0a8b188`; the restored blob is
`181975bac4ef8df43325b40f4de32b5963c7149b` both in the worktree and in HEAD.

## Adjudication

Accepted with no fix round. The observer is default-off, bounded, role- and
carrier-complete, and descriptive only. This review supplies no performance
measurement or causal grade. The next approved work is adapting the neutral
schema to the pinned `0f922de` historical control and running the local
old/new plus observer-OFF/ON TCP/gRPC comparison. In-stream runs still require
one identical, unique run ID per observed session; the existing block-level
rig harness is invalid unchanged.
