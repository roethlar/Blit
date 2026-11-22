# cr-ecm23-5: a Windows source without its metadata counted as mode 0000

**Severity**: MEDIUM — a Windows source sent with `--drop-windows-metadata` was taken for a Unix file of mode 0000, so a destination others could read was never reused and every block was sent again
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm23-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r24` over `24d4064f..c89b1eb1` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r24.codex.md`. It completes cr-ecm22-4 (`8c1f1363`), which brought it in.

## Evidence
"`--drop-windows-metadata` produces a Windows header with zero permissions and no metadata. The receiver interprets it as Unix mode `0000`. A simulated Windows source and Linux receiver transferred all 131,072 bytes of an identical `0644` file. Using the full wire mode's presence restored the expected zero-byte transfer."

## Intake
Admitted as the reviewer put it. A Unix source's header carries `permissions_mode`, the whole `st_mode`, file-type bits included (`remote/transfer/source.rs`), so a Unix file of mode 0000 arrives as `0o100000`; a source with no Unix mode sends 0.

## What
`wire_source_mode(header)`: the mode is known when the header carries one (non-zero), and `None` otherwise — whatever the Windows metadata. The destination's resume hashing takes it.

## Guard proof
- `own_incomplete_copy_tests::a_source_mode_is_known_by_the_mode_its_header_carries`: `0o100644` and `0o100000` (a Unix file of mode 0000) are modes; 0 is none, with or without Windows metadata.
- Mutation, restored byte for byte: the mode told by the Windows metadata (red).
- The resume, sink and session tests (297) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `wire_source_mode`, its call; guard.

## Known gaps
- The guard is at the decoding; the reviewer's end-to-end measurement (a simulated Windows source, an identical file, no bytes sent) is not repeated as a test.
