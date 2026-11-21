# cr-ecm6-3: A changed deletion rule travelled under the old versions

**Severity**: MEDIUM — between two builds of the same contract, one planned a deletion the other did not; empty directories only
**Status**: Verified — `exclude-case-r7` (Codex openreview, 2026-11-21) reviewed `3bc3ee47..cbab0c39`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `eb0db4b7`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r6` over `3bc3ee47..a13238b6` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r6.codex.md`.

## Evidence
Given the same decoded empty `--files-from` list, head planned no deletion. The planner from `0cf36891` scheduled a destination-only empty directory for deletion. Both use session contract 10 and delegated spec 5, so both version gates admit the older behaviour.

Cause: cr-ecm5-5 changed what the destination's delete pass takes for a candidate under a name list, and bumped nothing — it added no field. The end that plans a mirror's deletions is the destination, which is this machine on a pull and the daemon on a push, so a rule that differs between two builds of one contract gives one command two results. The rule was already stated twice in this plan's history for FIELDS (cr-ecm3-1, cr-ecm5-1); this is the same rule for behaviour.

## Predicted observable failure
This CLI pushing `mirror --files-from LIST` to a daemon built from `46ee5dde`..`0cf36891` (contract 10): the daemon removes empty directories the list does not name, where a daemon of this build removes none.

## Intake
Admitted.

## What
Session `CONTRACT_VERSION` 10 → 11 and delegated `SUPPORTED_SPEC_VERSION` 5 → 6, with no change on the wire: the versions now name the deletion rule. README, CHANGELOG, the release notes and `docs/TRANSFER_SESSION.md` say 11, and the latter says what the contract changed.

## Guard proof
- `transfer_session::tests::the_name_list_deletion_rule_holds_only_under_contract_11_or_later` and `operation_spec::tests::a_request_under_the_name_list_deletion_rule_is_version_6_or_later`: the versions read at run time; this build refuses a version-5 request by name.
- Mutations, each restored byte for byte: the contract back at 10 → the first red; the spec version back at 5 → the second red.
- Run against real daemons of contract 10 (release `blit-daemon` built from the tree at `a13238b6`), this build's CLI, `mirror --yes … --files-from EMPTY`: a direct push is refused, `session BUILD_MISMATCH: contract version mismatch (D-2026-08-18-2): local v11 vs peer v10`; a daemon-to-daemon request between two such daemons is refused, `unsupported TransferOperationSpec spec_version 6 (expected 5)`.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `CONTRACT_VERSION`, its history, the guard.
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — `SUPPORTED_SPEC_VERSION`, its history, the guard.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`, `docs/TRANSFER_SESSION.md` — contract 11.

## Known gaps
- The two guards hold the numbers, not the rule: nothing in a test can make an older planner run. The rule itself is guarded where it lives (cr-ecm5-5).
- Other fixes of this plan also changed what a build does with one request without a field — the form a local filter holds an include in (cr-ecm6-1) changes only what this machine does for itself; none of the others changes what a PEER plans or deletes. That is this author's reading of each, not a proof.
- Every daemon must be rebuilt with the CLI, as at each contract change. Run on Linux only.
