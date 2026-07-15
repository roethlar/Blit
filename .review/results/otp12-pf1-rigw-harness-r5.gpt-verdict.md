# otp12-pf1-rigw-harness round 5 — GPT verdict

- Reviewer: `gpt-5.6-sol` (`xhigh`) via `codex-cli 0.144.4`
- Reviewed range: `4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..06b33228d502c51da24bc2a78fba7eddcf6c0723`
- Review timestamp: `2026-07-15T12:01:21Z`
- Raw review: `.review/results/otp12-pf1-rigw-harness-r5.codex.md`
- Review verdict: `NEEDS FIXES`

Codex independently confirmed G5: the production parser selects exactly the
registered interface, rejects zero or duplicate registered-interface rows,
normalizes MAC case, and its real three-interface Bash 3.2 fixture fails when
the interface predicate is removed.

## Adjudication

### F1 — Accepted (High): Windows purge helper lacks immutable provenance

At `scripts/bench_otp12pf_rigw.sh:1965`, the harness executes
`D:/blit-test/purge-standby.ps1` after checking only that it exists and exits
zero. The reviewed repository owns `scripts/windows/purge-standby.ps1`, but
the harness neither stages that exact file nor hashes the remote copy. A
stale or no-op helper can therefore leave Windows data warm while the run is
recorded as cold; an unreviewed helper also makes the no-unregistered-policy-
mutation claim unprovable. No existing source or test pins the endpoint file.

Fix by staging the exact reviewed helper into the per-session Windows tree,
verifying its SHA-256 after staging and before every invocation, requiring its
exact success sentinel, recording it in the staging manifest, and adding an
offline guard whose production-hash mutation turns the Bash 3.2 self-test
red. Fresh complete review is required afterward.

No finding was rejected or deferred. No endpoint was contacted.

reviewer: gpt-5.6-sol
