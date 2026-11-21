# cr-uv3-2: The request between a CLI and a daemon was not versioned with what it says

**Severity**: HIGH — a CLI and daemons of different builds agree to a transfer and read its source differently
**Status**: Verified — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`, which required no change to it. (A later answer between a CLI and two daemons that is not versioned with what it says — cr-ecm13-3's — is cr-ecm14-3)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `19296bfb`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r3` over `eb0cc4d8..dd0a3179` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r3.codex.md`. It completes cr-uv2-1.

## Evidence
"I copied a regular file spelled `one.txt/` between two daemons. A head CLI with base daemons and a base CLI with head daemons both exited 0 and copied the file. With all three at head, the command refused it and created nothing. The daemon-to-daemon contract advanced to 19, but the CLI-to-daemon specification stayed at 7. Bump the delegated version and guard both mixed combinations."

Cause. The contract version gates the two ends of a session — for a transfer between two daemons, the daemons against each other. The machine that ASKS is gated by the request's own version (`SUPPORTED_SPEC_VERSION`), whose history states the rule: a request that says something an older daemon does not know must carry a version that daemon refuses. cr-uv2-1 changed what `source_path` says (the mark that a source was typed as a directory) and bumped only the contract. cr-ecm11-1 then added `FilterSpec.files_from_root`, which rides the same request, the same way.

## Intake
Admitted as the reviewer put it. One bump covers both changes: neither had been reviewed with a version of its own, and a daemon either knows both or neither.

## What
`SUPPORTED_SPEC_VERSION` is 8. A daemon refuses any other version, in both directions, as it always has.

## Guard proof
- `operation_spec::…::a_request_that_says_how_its_source_was_typed_is_version_8_or_later`: this build's request with `one.txt/` and `files_from_root` is read with the mark and the list's mark kept; the same request as version 7 is refused; the number is 8 or more.
- Mutation, restored byte for byte: the version left at 7 → the guard red.

## Files changed
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — the version, its history, the guard.

## Known gaps
- The two mixed combinations are guarded by the refusal of any other version, in this build; no daemon of the earlier build was run against it here (the reviewer ran both).
- Run on Linux only.
