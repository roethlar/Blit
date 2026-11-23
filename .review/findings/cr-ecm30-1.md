# cr-ecm30-1: what prompted a whole-file resume was not carried through the command

**Severity**: HIGH — (A) the wire's whole-file baseline kept nothing of what was found when it was decided, so a file that came to the name before the first block was taken for it and replaced; (B) a saved device and number could match a different file given the number of one gone; (C) an automatic retry pass, carrying only the path, can overwrite a newcomer whose conflict the first pass detected
**Status**: Reviewed, found incomplete — `combined-r32` (Codex openreview, 2026-11-23) reviewed `93ecf7b2..714ed0b7` — the `combined-r31` round, the in-place rework of D-2026-11-23-1 among it — with `guard_confirmed: true`: "The Unix implementation removes the staging directory and the dependency on atomic exchange. Creating the partial directly also preserves the destination directory's group inheritance. I would keep that design and the shared nonblocking checksum opener." "A focused check confirmed that a read error after the first block returns `Whole`, discarding the version already obtained. The receiver can then remove a replacement that arrived before the first write. Carry the last trustworthy observation through that fallback, or fail the individual file for this pass." "The wire comparison follows links. A dangling link introduced after an observation of absence can therefore pass the receiver's absence check and redirect creation to another in-root name. Conversely, I verified that removing the previously observed file and leaving the name vacant produces a conflict. Preserve the leaf observation across comparison and acquisition, and allow exclusive creation when the original file has simply gone." The remaining work is cr-ecm31-1 and -2; part C stays declined (D-2026-11-23-2)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm30-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` over `fd603571..93ecf7b2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`. It completes cr-ecm29-2 (`72729f85`).

## Evidence
"The wire's `Whole` baseline discards which file prompted whole-file transfer. A replacement arriving before the first block becomes the expected occupant. Within publication, a saved device/inode pair can also match a recycled inode after the original is released. Even when Blit detects a conflict correctly, its default retry carries only the pathname and can overwrite a different-length newcomer on the next pass. Retain the original observation, including absence, and keep detected conflicts failed across automatic retries." (`crates/blit-core/src/transfer_session/mod.rs:6711`, `crates/blit-core/src/copy/file_copy/resume.rs:984`, `crates/blit-cli/src/transfers/retry.rs:574`)

## Intake
Admitted in three parts. A and B are fixed here, under the in-place design of D-2026-11-23-1.

C is put to the owner and not built. It needs a third per-path set from the destination, beside the failed paths and `left_in_place`. Such a set crosses the wire (two messages, a contract bump), job records and run logs. It also runs against the owner's model, D-2026-07-09-1 ("convergence-on-retry is the reliability model"): a retry pass that copies the source over what is at the name converges, as any later run would.

## What
- (A) `ResumeBaseline::WholeOf(Option<moved::Version>)`.
  - When a look decides that a resumed file comes whole, the hashing (`compute_resume_block_hashes`) keeps what that look found: the file as it stood, from the path's look or the opened handle's, or nothing (`NotFound`).
  - The hold (`hold_whole`) replaces that and nothing else. A file at the name that is not that version, or any file where nothing was found, is left as it is, and the file fails.
  - Where no look decided it (a read failed, the file changed while it was hashed), the baseline stays `Whole`, and the hold looks for itself as before.
  - `ResumeBaseline::is_whole` covers both.
- (B) `Entry` carries `version`: size, both times, device and number, read from the same `fstatat`. A fresh partial's removal of what stood at the name asks for that version, not only the device and number. The partial's own cleanup still asks only which file it is, because readying it changes its length and times.

## Guard proof
- `resume::tests::a_file_changed_since_it_was_seen_is_not_removed` (Unix): the destination, the same file and number, grows by a byte after it was seen. The resume fails, and the file is left with its byte.
- `sink::tests::the_wires_whole_resume_leaves_what_came_after_the_comparison` (Unix), two cases, each left as it is with its completion failed:
  - a file renamed over the one compared;
  - a file put where nothing was found.
- `transfer_session::tests::what_decided_a_whole_resume_goes_with_it` (Unix): a destination too wide is sent `WholeOf` its version, and an absent one `WholeOf(None)`.
- Eleven older assertions that a baseline is `Whole` now ask `is_whole()`.
- Mutations, each restored byte for byte:
  - a removal asking only which number the file had → red;
  - the hold asking nothing of what the comparison found → red;
  - the hashing sending nothing of what decided a whole send → red.
- The resume, sink, session, copy and data-plane tests (358) pass. Windows cross-clippy is clean, and so is macOS cross-clippy (`CARGO_FEATURE_PURE=1`).

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Entry::version`, `entry_at`, `Entry::of`, `fresh_as`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `ResumeBaseline::WholeOf`, `is_whole`, `hold_partial`, `hold_whole`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard; assertions.

## Known gaps
- (C) declined (D-2026-11-23-2): an automatic retry pass re-sends a path whose conflict the first pass detected, and on that pass whatever stands there is compared and copied over, as on any run — convergence on retry, the owner's model.
- A version check and the removal after it are still two calls; a change between them goes unseen (D-2026-11-23-1).
