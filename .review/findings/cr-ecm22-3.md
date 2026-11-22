# cr-ecm22-3: a completion acknowledged without a verified identity

**Severity**: HIGH — a resume reported complete for a file it never verified was the one it held
**Status**: Fixed on Unix, awaiting review; the Windows half is blocked (no Windows host reachable)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm22-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r23` over `5d2f8def..24d4064f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r23.codex.md`. It completes cr-ecm21-1 (`fe38ecd4`) and cr-ecm21-4 (`50b416ed`).

## Evidence
"On Windows, if a nonempty destination disappears after all its blocks matched, the new completion hold can create a zero-filled file and report it complete. The Windows path discards the hash baseline and reopens by name. On Unix, a failed metadata read in `HeldPartial::of` also disables the final identity check. Propagate acquisition errors and fail the record when the receiver cannot verify delivery."

## Intake
Admitted. The Unix half is fixed here. The Windows half — a hold that keeps no handle and knows no file identity there, and does not use the hash baseline — needs Windows file identity and a Windows hold, and cannot be run here: both Windows hosts answer "No route to host" (2026-11-22). It stays open, blocked.

## What
`HeldPartial::of` (Unix) returns an error when the look at which file it holds fails; it was dropped, leaving the identity unknown and the completion's check that the file is still the one at the name skipped. Every hold takes the error — hashed, whole, and with no word of the hashes — and the file fails at its completion record.

## Guard proof
- `sink::tests::a_hold_that_cannot_tell_its_file_fails_it` (Unix): for a hashed, a whole and an un-noted hold whose look fails (a seam, `IDENTITY_FAULTS`), the completion reports the file failed.
- Mutation, restored byte for byte: the look's failure dropped (red).
- The resume, sink and session tests (294) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `HeldPartial::of`, its seam, its three callers; guard.

## Known gaps
- Windows: blocked, as above.
- A streamed whole-file record (`Made`) drops a failed look the same way. Its unclaimed-file cleanup goes by that identity, so failing there needs its own design. Not changed here.
