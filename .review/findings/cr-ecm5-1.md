# cr-ecm5-1: The empty-list marker travelled under versions an older peer accepts

**Severity**: HIGH — a daemon of the earlier contract discarded the marker and its mirror deleted a destination-only file the list kept out of scope, exit 0
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm5-1: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r5` over `3bc3ee47..585d3599` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r5.codex.md`.

## Evidence
The pinned CLI accepted a daemon built from `c35e43d4`, an earlier revision pushed to `origin/master`. With a comments-only `--files-from`, `--ignore-case` and `--include '*.jpg'`, that daemon deleted destination-only `only.JPG` and returned success. The head daemon kept it.

Cause: cr-ecm4-2 added `FilterSpec.files_from_set` inside session contract 9 and delegated spec version 4, and recorded as a known gap that builds made between slice 3 and that commit "speak the same versions and do not read the field", on the reasoning that no RELEASE carries contract 9. Such builds exist all the same — every revision from slice 3 to cr-ecm4-2 was published on the development branch — and the rule this plan had already paid for once (cr-ecm3-1, and the constant's own history since R51-F3) is that a field an older peer would ignore to destructive effect gets a version that peer refuses.

## Predicted observable failure
A CLI at or after `697eefce` (cr-ecm4-2) against a daemon built from any earlier revision that carries contract 9 — slice 3's landing through `004ac102`; the reviewer used `c35e43d4`: `mirror --files-from EMPTY --include PATTERN` selects nothing here, and the daemon applies the include, copies and deletes.

## Intake
Admitted. The known gap was a wrong call.

## What
The marker has versions of its own: session `CONTRACT_VERSION` 9 → 10 and delegated `SUPPORTED_SPEC_VERSION` 4 → 5. A contract-9 peer is refused at HELLO; a daemon of spec version 4 refuses the request. README, CHANGELOG, the release notes and `docs/TRANSFER_SESSION.md` say 10.

## Guard proof
- `transfer_session::tests::the_empty_list_marker_travels_only_under_contract_10_or_later`: the marker is on the wire, the unmarked filter is a different message, and the contract read at run time is 10 or later.
- `operation_spec::tests::a_request_with_the_empty_list_marker_is_version_5_or_later`: this build's own version is accepted and yields a list that holds no name; version 4 is refused by name; the version read at run time is 5 or later.
- Mutations, each restored byte for byte: the contract back at 9 → the first guard red; the spec version back at 4 → the second red.
- Run against real daemons of the earlier contract (release `blit-daemon` built from `585d3599`: contract 9, spec version 4), this build's CLI, `mirror --yes --ignore-case --include '*.jpg' --files-from EMPTY`: a direct push is refused, `session BUILD_MISMATCH: contract version mismatch (D-2026-08-18-2): local v10 vs peer v9`; a daemon-to-daemon request between two such daemons is refused, `unsupported TransferOperationSpec spec_version 5 (expected 4)`; the destination-only `only.JPG` is there after both.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `CONTRACT_VERSION`, its history, the guard.
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — `SUPPORTED_SPEC_VERSION`, its history, the guard.
- `crates/blit-core/proto/blit.proto` — the field's comment.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`, `docs/TRANSFER_SESSION.md` — contract 10.

## Known gaps
- The daemons run against were built from `585d3599` (contract 9 WITH the field), not from `c35e43d4` as the reviewer's was; what is shown is that any contract-9 daemon is now refused, which covers that one.
- Every daemon must be rebuilt with the CLI, as at each contract change.
- Run on Linux only.
