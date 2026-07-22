# ldt-4 live-f14 — tactical Opus review round 1

- Reviewer: Claude Opus 4.8 via Claude Code 2.1.217, effort `max`
- Reviewed range: `679253c7e2f12f4e313f0bfc26d2d044ce377e61..8385d2334b155cd1044fb9c11fb3a33f2e8078e0`
- Review session: `7a84f4a9-dab8-496a-a509-f2a28880cce2`
- Retained detached worktree: `/private/tmp/blit-opus-ldt4-f14-8385d23`
- Result: `findings`, one Low admitted, `guard_confirmed: true`
- Authority: tactical advisory code review; not formal `openreview` acceptance

Opus confirmed the f14 core is exact and fail-closed. It traced the dial
operation provenance to prove only `ADD`/`REMOVE` can reach the derived SOURCE
expectation, verified production SOURCE events use protobuf `as_str_name()`,
and confirmed dial and DESTINATION action namespaces remain unchanged. Its
required shorthand mutation reproduced the exact 27-test failure and clean
restoration returned all 88 analyzer tests green.

One Low finding is admitted as `ldt-4-live-f14-r1-f1`. The new test pinned the
synthetic fixture's production spelling but did not prove analyzer rejection.
Opus replaced both SOURCE action comparisons with constant `False`; all 88
tests still passed. A later relaxation could therefore accept malformed
control-lane evidence. The bounded fix adds one negative valid-session guard
for the shared resize-event comparison and one for `source_settled`.

The prompt supplied an invalid long expansion of the base SHA. The reviewer
resolved short `679253c` to the exact base recorded above before reviewing the
range. The structured result and reviewed hashes are in
`ldt-4-live-f14-r1.opus.json`.
