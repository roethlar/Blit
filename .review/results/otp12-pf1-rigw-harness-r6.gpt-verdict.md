# otp12-pf1-rigw-harness round 6 — GPT verdict

- Reviewer: `gpt-5.6-sol` (`xhigh`) via `codex-cli 0.144.4`
- Reviewed range: `4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..75a9a33ce600e4707438ed885de2ce0cdf27d946`
- Review timestamp: `2026-07-15T12:28:24Z`
- Raw review: `.review/results/otp12-pf1-rigw-harness-r6.codex.md`
- Review verdict: `NEEDS FIXES`

## Adjudication

### F1 — Accepted (High): expected helper hash is adopted from mutable worktree bytes

At `scripts/bench_otp12pf_rigw.sh:1370` and `:1770`, the clean-tree check
precedes several endpoint gates, while `stage_purge_helper` later computes and
adopts the expected SHA-256 from the working file. A concurrent replacement
during that interval can therefore become the manifested expected helper; a
no-op replacement that emits `standby-purged` then satisfies every G6 check.
The existing mocks prove stage/per-arm consistency but not reviewed-Git-blob
to staged-content identity.

Fix by deriving the one expected SHA-256 from the helper blob addressed by the
exact reviewed commit, storing that immutable blob identity, and requiring the
working file to match immediately before SCP. The remote post-copy and per-arm
checks continue comparing against the Git-derived value. Add a temporary-repo
guard that commits one helper, changes the working file, and requires the
binding gate to refuse; removing the blob/worktree comparison must turn that
guard red. Fresh complete review is required afterward.

No finding was rejected or deferred. No endpoint was contacted.

reviewer: gpt-5.6-sol
