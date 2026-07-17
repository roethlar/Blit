# ldt-4 canonical-fixture round 2 — Claude acceptance

**Status**: Clean; exact reviewed harness head accepted for fresh live gates.
**Reviewer**: Claude CLI 2.1.212, `claude-fable-5`, effort `max`
**Base SHA**: `4e0fdc307ba26e81f8532cd191089fa291c7f1aa`
**Reviewed SHA**: `5a2265e202a4ca5b4bbf08f8b58b7ff59ff75a8b`
**Retained worktree**: `/private/tmp/blit-openreview-ldt4-final-5a2265e-r5`
**Structured result**: `.review/results/ldt-4-canonical-r2.claude.json`
**Recorded**: `2026-07-17T05:18:51Z`

## Dispatch and acceptance checks

The substantive prompt was exactly the neutral best-way question. The remaining
text supplied only immutable repository coordinates, the side-effect boundary,
the model-emitted heartbeat requirement, the independent guard requirement,
and the result schema. Prompt SHA-256:
`d0800bba585edc39676402bedd435c984c3de8cd15cc56ed2511eb035ef5a106`.
Schema SHA-256:
`02d943b7f907aa2b568b38a2d0633726aa96eaf64914f7d8cda3390a3a3091ab`.

The one-shot process exited zero. Its terminal envelope reported success and
completion, and its structured result is schema-valid with `verdict=clean`, no
findings, the dispatched base/head SHAs, and literal `guard_confirmed=true`.
The reviewer had only Bash, Edit, Read, Grep, Glob, and structured-output tools
under `dontAsk`; its init event showed no MCP server. Settings SHA-256:
`8cf912df8bd654e8f7371bc258a7e87d199767c5693214f7564c1b44d81f14df`;
guard SHA-256:
`f21a61d55c4b82b8737e1681fab90b9cba0472b5b0014c63d5064539cdd28bb7`;
launcher SHA-256:
`b8b4cf70d9bbfdd8c5058d6c0056df3e1802dd3ed50b2cbb8f3a940f5b47e5dd`.

Fable emitted ordinary activity heartbeats while discovering the goal, tracing
manifest and promotion helpers, checking session-namespace assumptions,
reviewing records, running checks, and executing its guard. The raw stream is
retained at `/private/tmp/ldt4-final-fable-r5.claude.stream.jsonl`, SHA-256
`6ca88f5e61978fce035ad0e4e4fc17473fe9541c459edf31abc63e0ce7b93694`;
its stderr log is empty. Several read-only command forms were denied by the
narrow allowlist or conservative safety matcher; the reviewer adapted to
permitted reads and checks. No denied command executed.

## Independent guard proof

Fable ran all 75 analyzer tests and the exact-head offline harness self-test;
both passed. It then replaced only
`rename_q_directory_exclusive "$incoming" "$local_destination"` with the old
`mv -n "$incoming" "$local_destination"` form using Edit. The self-test exited
one with `canonical fixture promotion is not an exclusive atomic rename`.
Fable reversed that exact Edit, and the self-test returned
`PASS (96 arms, no SSH)`. Direct post-run checks confirmed the detached
worktree is clean at the exact reviewed SHA.

## Acceptance

This is a complete clean openreview result, so there are no candidate findings
to triage. Exact `5a2265e`, not this later review-record commit, is accepted for
fresh additive staging and the registered live environment/preflight gates.
The review does not create hardware evidence, validate endpoint quietness, or
accept any ldt-4 matrix arm.
