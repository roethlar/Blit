# cr-ecm41-3: the sender told a sibling's bytes once a request had taken the granted header out

**Severity**: MEDIUM — the log names a file it sent as its sibling. cr-ecm40-4's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm41-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r42` over `b6e6fab9..e2c2e08d` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r42.codex.md`.

## Evidence
"A `NeedBatch` removes the first header from the sender's map. A later byte-named sibling can then appear to be the first entry with that display text. With 8,192 intervening manifest entries, the receiver wrote only the UTF-8 file `dup�`, but the sender logged it as `dup\xff`. Emit the completion identity from the granted header or retain a consumed marker for the whole pass."

## Intake
Admitted; the first remedy, which needs no marker kept for the whole pass.

## What
- The sender no longer tells a file's bytes as the manifest goes out.
- Its need handler (`source_recv_half`) tells them from the header the destination asks for, as it takes that header out of the map, and before it counts the need.
- cr-ecm40-4's `keep_first_of_its_text` is gone; the map keeps the first of each spelling, as before it.

## Guard proof
- `transfer_session_roles::a_sender_names_a_file_by_the_header_asked_for` (Unix): a scripted destination asks for the first of a spelling before its sibling is listed.
  - The sender is held back by its full 64-frame lane.
  - Two pairs are made in both orders around 5,000 files.
  - Only the name asked for is told by its bytes.
- Red on the unfixed code on tmpfs and on Btrfs: the sender told both siblings, though only the text spelling was asked for.
- Mutation, restored byte for byte: the granted header's bytes never told → red, for this guard and for cr-ecm40-4's `a_push_logs_each_file_it_sent_by_its_own_name`.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's six fixes: 2,152 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `source_recv_half`, the manifest emitter.
- `crates/blit-core/tests/transfer_session_roles.rs` — guard.

## Known gaps
- End to end, with 12,000 files between the siblings, the fault did not show on this host: the sender listed everything before the first request came back. The scripted guard is the one that bites.
