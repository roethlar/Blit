# cr-ecm32-1: a whole-file resume did not carry where its comparison led

**Severity**: HIGH — two holes. (1) A failed lookup was sent as "unknown". An `ENOTDIR` while following a link the comparison had seen sent the file `Whole`, and the hold then looked for itself and replaced whatever had come. (2) The comparison carried no place. With a link of the chain redirected between two empty names, both baselines matched, and the partial was made at the new name.
**Status**: Reviewed, found incomplete — `combined-r34` (Codex openreview, 2026-11-23) reviewed `70efef8a..0f912b06` — the `combined-r33` fixes — with `guard_confirmed: true`: "Checking before truncation, opening FIFOs without waiting, and recording the resolved parent and name for a Unix resume are sound changes." "On Unix, if a previously present partial's parent vanishes before hashing, `place` becomes `None`. If the parent returns as a symlink to another in-root directory, an empty name there can receive the file. Fail that pass when the parent cannot be established. On Windows the receiver discards even `Unjustified`, so a failed comparison can still authorize a later write. Enforce that refusal on every platform and retain the validated Windows handle for writes." The remaining work is cr-ecm33-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f22af07f`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r33` over `714ed0b7..70efef8a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r33.codex.md`. It completes cr-ecm31-2 (`31eac5d2`).

## Evidence
"I confirmed that an `ENOTDIR` while following a successfully observed symlink discards the leaf observation and returns `Whole`. The receiver can then replace whatever arrives before the first block. I also confirmed that redirecting an intermediate symlink between two different empty names produces identical baselines, allowing creation at the new name. Carry the resolved parent identity and final name, distinguish a failed lookup from observed absence, and fail that file when the comparison cannot justify the write."

## Intake
Admitted as the reviewer put it.

## What
- `copy::Place` holds the directory a name's chain of links ends in (device and number) and the name there. `Resolved::place` gives it.
- `ResumeBaseline::WholeOf` gains `place`.
  - The hashing (`compute_resume_block_hashes`) resolves the destination as the hold does (`resolve_destination`) and keeps where it led.
  - A name whose directory is not there yet leads nowhere yet (`None`).
  - The hold (`hold_whole`) resolves again. It makes the partial only if that leads to the same place; anything else is left as it is, and the file fails.
- `ResumeBaseline::Unjustified` is new: nothing was hashed, and nothing could justify a write.
  - The hashing sends it when a look at the name, or at where it leads, fails other than as `NotFound`.
  - The hold fails that file and makes nothing. Its retry compares afresh (D-2026-11-23-2).
  - `is_whole` covers it: every block is sent.
- `ResumeBaseline` is `Clone`, no longer `Copy`, since it now carries a name.

## Guard proof
- `transfer_session::tests::a_look_that_fails_justifies_no_write` (Unix): a link through a file (`file.bin/inner`, `ENOTDIR`) is sent `Unjustified`.
- `sink::tests::a_whole_resume_writes_only_where_its_comparison_led` (Unix):
  - an `Unjustified` resume leaves what stands at the name and fails;
  - `link.bin → mid.bin → a.bin`, compared, then `mid.bin` redirected to `b.bin`: nothing is made at either name, and the file fails.
- `what_decided_a_whole_resume_goes_with_it` and `a_hashing_that_fails_part_way_keeps_what_it_found` now expect each place as the hold would resolve it.
- Older sink guards that build their own baseline carry no place.
- Mutations, each restored byte for byte:
  - a failed look sent as unknown → red;
  - the hold taking an unjustified resume for unknown → red;
  - the hold asking nothing of where the comparison led → red.
- All 1,160 `blit-core` tests pass. Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs`, `crates/blit-core/src/copy/mod.rs` — `Place`, `Resolved::place`.
- `crates/blit-core/src/remote/transfer/sink.rs` — `ResumeBaseline` (`WholeOf::place`, `Unjustified`, no longer `Copy`), `hold_partial`, `hold_whole`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guards.

## Known gaps
- Off Unix no place is found (`None`); the Windows half is cr-ecm32-4, blocked.
- A name whose directory comes into being between the comparison and the hold is written wherever its resolution then leads.
