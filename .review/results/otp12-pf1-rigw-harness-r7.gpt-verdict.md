# otp12-pf1-rigw-harness round 7 — GPT verdict

- Reviewer: `gpt-5.6-sol` (`xhigh`) via `codex-cli 0.144.4`
- Reviewed range: `4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..a53971574a8badb2ddf4ab952168fc7b2739ff89`
- Review timestamp: `2026-07-15T13:23:47Z`
- Raw review: `.review/results/otp12-pf1-rigw-harness-r7.codex.md`
- Review verdict: `NEEDS FIXES`

## Adjudication

### F1 — Accepted (High): replacement objects can rewrite reviewed helper content

At `scripts/bench_otp12pf_rigw.sh:1445`, ordinary Git object lookup honors
`refs/replace`. A replacement commit can leave `HEAD_FULL` unchanged and
`git status` clean while `HEAD_FULL:scripts/windows/purge-standby.ps1`
resolves to a substituted blob. G7 would then derive, stage, manifest, and
recheck the substituted digest as though it belonged to the reviewed commit;
a no-op helper could invalidate the cold-cache evidence.

Fix every reviewed-helper commit/path resolution, object-type inspection, and
blob-content read by disabling replacement-object interpretation. A Bash 3.2
temporary-repository guard must install a replacement commit that preserves
the visible HEAD and clean status while ordinary Git resolves substituted
helper bytes. The reviewed-object binding must refuse that spoof, and removing
the no-replace protection must turn the guard red. Fresh complete review is
required afterward.

No finding was rejected or deferred. No endpoint was contacted.

Fix commit: `29d63b7ad45dff21d052a678fff795029b300e6d`.

reviewer: gpt-5.6-sol
