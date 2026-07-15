# otp12-pf1-rigw-harness round 14 — Claude adjudication

- Reviewer: `claude-fable-5` via Claude Code `2.1.210`, effort `max`
- Reviewed range: `35f160f03d6dedc6beb841a3033076dab2b6e07c..1f62ce564f127e18f5e0f044dd9fd4605f3a610c`
- Code fix: `942c88e601ea2d27f0a1da52aa5408b763ee61f4`
- Retained authoritative worktree: `/tmp/blit-review-g14-1f62ce5-r2`
- Orchestrator record: `.review/results/otp12-pf1-rigw-harness-r14.claude.json`
- Independent verdict: `ACCEPTED`
- Guard confirmed: `true`

The first invocation through the request proxy returned no verdict. After the
owner removed that proxy and explicitly instructed a retry, the exact stale
Claude process was terminated with `SIGTERM`; its worktree, prompt, and debug
log remain retained as non-authoritative evidence. The fresh retry ran in a
new clean detached worktree with no matching proxy environment variables and
returned exit zero, a schema-valid `accepted` verdict, the exact dispatched
SHAs, and `guard_confirmed=true`.

Claude verified that the 10% Spotlight ceiling and immediate preflight remain
unchanged. Runtime checks outside timed arms may use the existing five-second,
120-second budget for either high load or high Spotlight, but succeed only
when both are simultaneously below their ceilings. Conflicting processes,
Time Machine, load parsing, and Spotlight parsing are rechecked every sample;
persistent and malformed contamination still fail closed.

Syntax, the complete Bash 3.2 self-test, all 23 analyzer tests, the docs gate,
and diff checks passed. Claude independently kept every new G14 test while
performing three production-only red mutations:

1. The byte-exact G13 `q_quiet_gate` failed on the post-transfer Spotlight
   recovery guard.
2. Polling offenders every ten seconds instead of every five failed on the
   next-poll offender guard.
3. Over-advancing elapsed recovery time failed on the exact-bound guard.

Exact reviewed bytes were restored after every mutation. The final script blob
is `f3a6195e121eaae74eb715a798e5e2d1aef70edb`, SHA-256
`21f4b686fdcd5ddfb47e8478a8666e5d10d0f9e838db2bf2f7227d0da75852d8`.
Syntax and the complete self-test returned green, and the retained worktree
ended clean at exact `1f62ce5`. No benchmark endpoint was contacted, no
retained artifact was deleted, and no Time Machine or mount state changed.
