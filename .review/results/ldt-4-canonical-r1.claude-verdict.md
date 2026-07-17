# ldt-4 canonical-fixture round 1 — Claude adjudication

**Status**: Findings; two Low corrections admitted before another live launch.
**Reviewer**: Claude CLI 2.1.212, `claude-fable-5`, effort `max`
**Base SHA**: `4e0fdc307ba26e81f8532cd191089fa291c7f1aa`
**Reviewed SHA**: `ef48920720b02d09a490c1c07f6acd35651aba65`
**Retained worktree**: `/private/tmp/blit-openreview-ldt4-canonical-ef48920-r4`
**Terminal result**: `.review/results/ldt-4-canonical-r1.claude.json`
**Recorded**: `2026-07-17T04:31:46Z`

## Dispatch and acceptance checks

The substantive prompt was exactly the neutral best-way question. The remaining
text supplied only the immutable repository coordinates, side-effect boundary,
model-emitted heartbeat requirement, independent guard requirement, and result
schema. Prompt SHA-256:
`25fcce0db70385a19abae3473bc4111027e2adfa1e82b3ff619366fad3446783`.
Schema SHA-256:
`02d943b7f907aa2b568b38a2d0633726aa96eaf64914f7d8cda3390a3a3091ab`.

The one-shot process exited zero. The envelope and structured result agree on
`verdict=findings`, exactly two populated findings, the dispatched base/head
SHAs, and literal `guard_confirmed=true`. The reviewer had only Bash, Edit,
Read, Grep, Glob, and structured-output tools under `dontAsk`; its init event
showed no MCP server. A deterministic PreToolUse guard restricted Bash and Edit
to the retained worktree and denied deletion, external commands, shell escapes,
and live-harness execution without `SELFTEST=1`. Settings SHA-256:
`d88763abf2f2427653ee56b63589e98e033e472628da22639d6376eccd1864e8`;
guard SHA-256:
`f84e43f18db7901cdba2de4bc09c297ff94859a0a6289f8e61804cb2a2a76bad`.

The reviewer emitted ordinary activity-only text heartbeats while mapping the
goal, tracing the harness, checking records, and running the guard. The raw
stream is retained at
`/private/tmp/ldt4-canonical-fable-r4.claude.stream.jsonl`, SHA-256
`4007964f66bb1c7a188113d65e0f98f153066a84752d162e6327288de9cee945`.
Its stderr log is empty. Several inspection command forms were denied by the
narrow allowlist or an intentionally conservative harness-name rule; the
reviewer adapted to permitted reads and checks. No denied command executed.

An earlier retained attempt (`r3`) is not a verdict. It issued a prohibited
`rm -rf /tmp/ldt4_guard` command while constructing its guard, so the
orchestrator interrupted it with exit 130 before any terminal result. Because
the path's pre-attempt existence cannot be proven, no claim is made that the
command deleted nothing. Its two resulting files, worktree, prompt, schema, and
raw stream remain retained; the stream SHA-256 is
`0dedd0f6be09219162829e0784ba66fd568b0adf5b60c4b8aecbf3eef577b296`.
The valid `r4` retry added the mechanical boundary above instead of relying on
prompt wording alone.

## Independent guard proof

Fable first ran the exact-head offline harness self-test successfully. It then
changed the tracked `q_to_windows:large` source mapping from the stable staged
path back to `/Users/michael/blit-bench-work/src_large` with Edit. The same
self-test failed with `q large fixture mapping selftest failed`. Fable reversed
that exact Edit and the self-test returned `PASS (96 arms, no SSH)`. It also ran
all 75 analyzer tests successfully. Direct post-run checks confirmed the
detached worktree is clean at the exact reviewed SHA and the restored harness
blob matches HEAD.

## Intake and adjudication

`ldt-4-r3-f1` — **ADMITTED (LOW): stable-path promotion should use the
existing exclusive atomic rename primitive.** `stage_fixtures` validates the
incoming tree, checks that the destination is absent, then calls `mv -n` into
the staging directory. The same harness already has
`rename_q_directory_exclusive`, backed by macOS `renameatx_np(RENAME_EXCL)`, for
the analogous no-clobber retention transition. The two configured pathnames
share a `/Users/michael` prefix, but the harness does not assert that they are
on one device. If either subtree is a separate mount, `mv` may degrade an
interrupted promotion into copy-plus-source-removal and leave a partial stable
fixture. Replace `mv -n` with the existing exact-destination helper and remove
`mv` from the prerequisite list. Severity is Low because current pre/post
checks remain fail-closed and the triggering layout is not the registered
default, but the existing helper removes the latent failure and the race with
no new primitive.

`ldt-4-r3-f2` — **ADMITTED (LOW): validate canonical shape before the large
copy and size the space gate from that validated manifest.** The Windows
manifest is written, hash-verified, and fetched before `scp`, but its shape is
not compared with the registered fixture until after transfer. The free-space
gate therefore trusts the registered byte constant while a drifted or
oversized canonical source can consume space before the harness voids. Compute
and validate `win_shape` immediately after fetch, derive `fixture_bytes` from
that validated shape, then copy; retain the post-copy q shape and exact manifest
comparison. Severity is Low because wrong input still fails closed, but today
it can waste a large transfer and breach the intended retained-space floor in
an already-failing launch.

Both candidates have concrete code evidence and observable failure conditions,
so neither is declined as style. They land one finding per commit with a
mutation-sensitive self-test guard, followed by fresh exact-head local gates
and one final neutral Fable whole-change pass. No endpoint launch occurs first.

