# cr-ecm20-3: a failed local partial could be left unwritable for the retry

**Severity**: HIGH — a resume of a read-only source that fails at its last step leaves a read-only partial, and every retry is refused
**Status**: Verified — `combined-r22` (Codex openreview, 2026-11-22) reviewed `b92c9a17..5d2f8def` — the eight `combined-r21` fixes — with `guard_confirmed: true`; no change required of it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c31a4594`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-ecm19-4 (`af308007`).

## Evidence
"In my Linux probe, a six-byte `0444` source followed by a final metadata error left a seven-byte `0444` partial. After I cleared the fault, the retry failed with `Permission denied`. Apply the source mode at successful completion, restore `0600` through the held handle on failure, and preserve recovery of existing owner-controlled read-only partials."

The resume gives its partial the source's mode when the blocks are done (cr-ecm16-1); the metadata tail after it can still fail, and the partial kept for the next run then had the source's mode.

## Intake
Admitted as the reviewer put it.

## What
- A partial kept for the next run (`OnFailure::KeepUnfinished`, local and over the wire) is made private and writable by its owner again (600) through the handle it was written through, beside its one-byte-longer mark.
- `open_resume_partial`: a partial there already that this run may read but not write (left read-only by such a failure before, or by an older blit) is had to be read, made private and writable through that handle, and opened again to be written — the same file, by device and number, or nothing is had. One this run may not read either is not had, as before (cr-ecm19-1).

## Guard proof
- `sink::…::a_failed_local_resume_leaves_its_partial_writable`: a six-byte 444 source and a failing metadata tail — the partial is seven bytes and 600; the fault cleared, the retry lands the bytes, with mode 444.
- `resume::tests::a_partial_left_read_only_is_made_writable_and_resumed`: a 444 destination holding half the source is resumed in place — the same file, blocks skipped, the source's bytes and mode.
- Mutations, each restored byte for byte: a partial left read-only not made writable (red); a kept partial keeping whatever mode it was given (red).
- cr-ecm19-1's guard (a write-only destination in a closed folder left as it stands) passes. The sink and resume tests (120) pass.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `settle_failed_target_known`; guard.

## Known gaps
- The source's mode is still given by the resume itself, before the sink's metadata tail; a failure in the tail now takes it back.
- A partial kept without a handle (one no longer held) is marked by its name and its mode is not changed.
- The guards prove nothing run as root, which writes a read-only file anyway.
- Unix only.
