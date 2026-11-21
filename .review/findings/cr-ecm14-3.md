# cr-ecm14-3: the answer two daemons give a CLI was not versioned with what it says

**Severity**: HIGH — a CLI of this build and two daemons of an earlier one agree to work together, and the CLI's retry asks for another file
**Status**: Verified — `combined-r16` (Codex openreview, 2026-11-21) reviewed `35adeacb..13faafd2` — utilities slices 5 and 6 and the eleven `combined-r15` fixes — with `guard_confirmed: true`, which required no change to it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `25fa29be`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. It completes cr-ecm13-3 (`6f0479dc`, `b2b77b78`); the reviewer found it by inspection.

## Evidence
"By inspection, a head CLI and two contract-25 daemons still agree on delegated spec version 12. Those daemons omit the raw failed-path list, so the CLI can retry a different filename with the same display text. Gate the CLI-to-daemon exchange on support for the response it consumes."

Cause. A transfer between two daemons is asked for by a CLI in a request that carries a version (`TransferOperationSpec.spec_version`), and nothing else is compared between the CLI and the daemons — the contract version is compared between the two daemons. cr-ecm13-3 added `failed_paths_raw` to the answer (`DelegatedPullSummary`) and moved the contract to 26, which gates the daemons against each other and says nothing to the machine that asks. The request stayed at version 12. It is the second time (cr-uv3-2): there a request field went out under an old number; here a field of the ANSWER did.

## Intake
Admitted as the reviewer put it. One more change of the same kind is taken with it: cr-uv4-1 changed what the answer's `source_ticket` stands for (wire contract 29), and a CLI of this build asking two daemons of the build before would be handed a ticket for the loose account. One number covers both.

## What
`SUPPORTED_SPEC_VERSION` is 13. The request names the version of the whole exchange — what is asked and what is answered — and its history says so at 13: a daemon answers only a request of its own version, and a CLI's request to a daemon of version 12 is refused there, with both numbers, before anything is read.

## Guard proof
- `operation_spec::tests::a_request_names_the_version_of_the_answer_it_reads`: this build's number is 13 or later; a request of version 12, and one of the version after this build's, are each refused naming the number sent and the number expected; this build's own is read.
- The eight guards of the earlier versions hold unchanged.
- Mutations, each restored byte for byte: the constant back at 12 (the guard red); a daemon answering any request of a version no later than its own (the guard red, and the eight before it).

## Files changed
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — the version, its history; guard.

## Known gaps
- **The rule is still a rule, not a check.** Nothing fails when a field is added to `DelegatedPullSummary` or `TransferOperationSpec` without the number moving; it has now been missed twice. A check that held the two messages' shape against the number would have caught both, and is not written here.
- Not driven end to end against a daemon of the earlier build: there is none to run. The refusal is `from_spec`'s, which every delegated request passes through.
- The CLI learns the refusal from the daemon's answer; it does not ask the daemon its version first.
