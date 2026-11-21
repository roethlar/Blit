# cr-uv3-6: The daemon took "a complete scan is required" for "the source will be removed"

**Severity**: MEDIUM — a copy that asks for a complete scan is refused as a move would be
**Status**: Verified — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`, which required no change to it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ec19179b`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r3` over `eb0cc4d8..dd0a3179` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r3.codex.md`. It completes cr-uv2-1.

## Evidence
"A library caller requesting a complete scan for a copy of `link/` receives the move-specific refusal because the daemon interprets `require_complete_scan` as a promise to delete the source. Carry source-removal intent explicitly."

Cause. cr-uv2-1 refuses a move through a link typed as a directory before anything is copied, and the daemon knew a move by the one thing a move asked of a source — a complete scan. That flag says a partial scan is to be refused; any caller may want that.

## Intake
Admitted as the reviewer put it.

## What
- `SessionOpen.source_removed_after`, **wire contract 21**, and `TransferOperationSpec.source_removed_after`, request version 9: the initiator says it will remove the source when the transfer ends well. `PullExecution`, `PullSessionOptions`, `DelegatedSpecOptions` and the normalized request carry it; `blit move` sets it beside `require_complete_scan`, on both routes.
- The daemon reads the removal from that field and nothing else.

## Guard proof
- `pull_session_cutover.rs::a_complete_scan_is_not_a_promise_to_remove_the_source` (Unix), through a daemon: a pull of `link/` that requires a complete scan and removes nothing copies through the link; the same pull saying its source will be removed is refused before copying.
- `remote_move.rs::a_move_through_a_link_typed_as_a_directory_is_refused_before_copying` (cr-uv2-1's guard) still holds: `blit move` says so on both routes.
- `operation_spec::…::a_request_that_says_its_source_is_removed_is_version_9_or_later`: the field reaches the normalized request whatever the complete-scan flag says; a version-8 request is refused.
- Mutations, each restored byte for byte: the daemon reading the removal off the complete-scan flag, as before; a move not saying so (to this machine; between two daemons); the session opened without it; the daemon that pulls not handing it on; the request's version left at 8 — six, each red on the guards named for it.

## Files changed
- `crates/blit-core/proto/blit.proto`, `crates/blit-core/src/transfer_session/mod.rs` (`CONTRACT_VERSION = 21`), `crates/blit-core/src/remote/transfer/{session_client,operation_spec}.rs`, `crates/blit-core/src/transfers/remote.rs`, `crates/blit-cli/src/transfers/{remote,remote_remote_direct}.rs`, `crates/blit-daemon/src/service/{transfer,delegated_pull}.rs`, `crates/blit-cli/tests/pull_session_cutover.rs`; `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- The field says what the initiator intends; nothing in the session removes anything (cr-uv3-1 is about what the removal then takes).
- Run on Linux only.
