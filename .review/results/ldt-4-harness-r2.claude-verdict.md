# ldt-4 harness/analyzer neutral Fable r2 verdict

**Status**: Clean formal openreview; harness accepted for the live ldt-4 gate.
**Reviewer**: Claude CLI 2.1.212, `claude-fable-5`, effort `max`
**Base SHA**: `e41b87173f2073a9b6694a62813eddc14a7844ad`
**Reviewed SHA**: `4e0fdc307ba26e81f8532cd191089fa291c7f1aa`
**Retained worktree**: `/tmp/blit-openreview-ldt4-harness-4e0fdc3-r2`
**Raw result**: `.review/results/ldt-4-harness-r2.claude.json`

## Dispatch

The substantive prompt was exactly the neutral best-way question. The rest of
the prompt contained only fixed repository coordinates, detached-worktree and
side-effect boundaries, the independent guard requirement, and the structured
result contract. Prompt SHA-256:
`66d8aa0ed55b3625996bc204b62544f1315466a9f4167ef36587a49ae589922b`.
Schema SHA-256:
`02d943b7f907aa2b568b38a2d0633726aa96eaf64914f7d8cda3390a3a3091ab`.

The one-shot process exited zero after 67 reviewer turns. Its envelope and
inner structured result agree exactly: `verdict=clean`, no findings, exact
base/head SHAs, and literal `guard_confirmed=true`. It recorded no permission
denials and no web search or fetch. The raw envelope retains the CLI's usage
telemetry without interpreting its nominal USD field as subscription billing.

## Independent guard proof

Fable mutated the one production step-up comparison in
`crates/blit-core/src/dial.rs` from
`blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO` to `blocked_ratio > ...`.
`cargo test -p blit-core --lib dial` then reported 17 passed and 13 failed.
After restoring the exact tracked file, the same focused suite reported all 30
passed. The restored file SHA-256 was
`48b2eaad9b791844d201bc5ec83e274a06d877dba5a27e046927bfa6f7e323fe`,
`git status --porcelain` was empty, and HEAD remained the exact reviewed SHA.

## Acceptance

The orchestrator independently parsed and checked the envelope, inner schema,
verdict/findings consistency, exact SHAs, guard literal, terminal success,
permission record, and web counters. A direct post-run check confirmed the
retained worktree is clean and detached at the reviewed SHA. The formal
fixed-SHA review gate therefore passes with no candidate finding to triage.

This accepts the additive harness/analyzer for live gating. It does not claim
hosted Windows CI, endpoint quietness, artifact readiness, a completed arm, or
adaptive hardware evidence.
