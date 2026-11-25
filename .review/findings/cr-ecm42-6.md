# cr-ecm42-6: the sender guard's order was the filesystem's

**Severity**: MEDIUM — the guard fails before it checks anything where both siblings are listed bytes-first. cr-ecm41-3's guard.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm42-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r43` over `e2c2e08d..4e12d761` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r43.codex.md`.

## Evidence
"Changing only fixture creation order so both byte-named siblings appeared first made the guard panic before it checked the sender's output. Use a scripted source that releases the sibling after the first header has been requested."

## Intake
Admitted as written.

## What
- **`ScriptedManifestSource`** (new, in the test) gives the manifest in the test's order: the text spelling first, its byte-named sibling last, held back until the test releases it.
- **The test** releases the sibling once the sender has taken the requested header out of its map, so the order is fixed whatever order the files were made or are listed in. The files are made in both orders.
- The 5,000 files between the siblings, and the full-lane timing they relied on, are gone.

## Guard proof
- `transfer_session_roles::a_sender_names_a_file_by_the_header_asked_for`, files made in both orders: green on tmpfs and on Btrfs.
- Mutations, restored byte for byte, each → red:
  - bytes told as the manifest goes out, the fault cr-ecm41-3 fixed;
  - the granted header's bytes never told.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2,156 tests over 57 binaries.

## Files changed
- `crates/blit-core/tests/transfer_session_roles.rs` — `ScriptedManifestSource`; guard.

## Known gaps
- None known.
