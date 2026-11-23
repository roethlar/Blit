# cr-ecm32-2: a destination was cut before it was checked, and the local copy's cleanup was armed before the open it guarded

**Severity**: HIGH — brought in by cr-ecm31-4. `create_regular` truncated on opening, so a check that then failed left an existing file at no length (11 bytes to 0, in the reviewer's check). The local copy probed the destination, armed a by-name cleanup, then opened it again by name. A FIFO put there between was refused by the copy's open and then removed by the cleanup.
**Status**: Reviewed, found incomplete — `combined-r34` (Codex openreview, 2026-11-23) reviewed `70efef8a..0f912b06` — the `combined-r33` fixes — with `guard_confirmed: true`: "Checking before truncation, opening FIFOs without waiting, and recording the resolved parent and name for a Unix resume are sound changes." "I copied through an unchanged, in-tree destination symlink. The correct bytes landed, but `blit copy --retries 0` exited 2 and blamed `--ignore-existing`, which was off. The completion check compares the opened file's inode with the link's inode. Use the existing follow-link completion rule for ordinary copies and retain the stricter check for exclusive creation." "With a `0444` source, a writable existing destination in a `0555` directory, and an injected metadata failure, cleanup could neither unlink the file nor reopen it for writing. It left eight bytes, exactly the source size, so a size-based retry can skip it. Capture identity during validation and give cleanup the still-writable handle." "I observed errors in both directions. Failed validation at a previously absent destination left an unreported zero-byte file. A source-only failure whose pathname contained the diagnostic phrase put an untouched destination in `left_in_place`. Track whether acquisition created a file, settle it on failure, and carry the outcome into the summary independently of the error text." The remaining work is cr-ecm33-1, -2 and -5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `02522882`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r33` over `714ed0b7..70efef8a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r33.codex.md`. It completes cr-ecm31-4 (`374ed558`).

## Evidence
"Injecting a metadata error immediately after a streamed destination opened left an existing 11-byte file at zero bytes. `create_regular` had already truncated it. The local path also arms deletion between its writable probe and the actual open, so cleanup can unlink a FIFO that the actual open correctly refuses. Validate the handle first, then truncate and tie cleanup to the object actually acquired."

## Intake
Admitted as the reviewer put it, on Linux. macOS clones into an absent name and Windows clones with its own handles, so their copies still go by the name. Neither can be run here.

## What
- `copy::create_regular` opens without truncating. It checks that what it opened is a regular file, then cuts it through the handle (`set_len(0)`). `CREATE_LOOK_FAULTS` is a new seam that refuses that check.
- The local copy (`copy_and_stamp_opened`, Linux):
  - opens the destination once with `create_regular`;
  - arms its cleanup for that file and no other (`PartialTarget::arm_made`: removed only while it stands at the name);
  - copies through what it opened with `copy_opened_into`, the same cascade on descriptors;
  - stamps the time through that handle.
- Off Linux the probe and the by-name copy stay. The probe and `PartialTarget::arm` are now compiled only there.
- A destination the copy cannot open is still not armed for (2026-10-07 defect (a)): the open fails before anything is armed.
- And, as with `--ignore-existing`, the copy is delivered only while what it wrote stands at the name (cr-ecm15-1's check, now armed on this path too).

## Guard proof
- `file_copy::mode_tests::a_file_opened_to_be_written_is_checked_before_it_is_cut` (Unix):
  - an 11-byte file whose check is refused is left whole;
  - unrefused, it is cut to 0 through the handle.
- `sink::tests::a_local_copys_cleanup_removes_only_the_file_it_opened` (Linux): after the copy, a file is renamed over the name and the copy then fails. The newcomer is left as it is, and the file fails.
- Mutations, each restored byte for byte:
  - cut on opening → red;
  - the cleanup armed for the name → red.
- Checks:
  - All 1,153 `blit-core` tests pass.
  - The four local-copy-mode CLI tests pass; modes are carried through the handle.
  - Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — `create_regular`, `create_look_fault`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `copy_and_stamp_opened`, `probe_destination_writable`, `PartialTarget::arm`; guard.

## Known gaps
- macOS and Windows still probe, arm by name and copy by name. A FIFO or file put at the name between the probe and the copy's open meets the old cleanup there. Neither platform can be run here.
