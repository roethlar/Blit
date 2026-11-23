# cr-ecm33-1: an ordinary local copy through a destination link was refused at completion

**Severity**: HIGH — brought in by cr-ecm32-2. On Linux the local copy armed its cleanup for the file it opened, which follows a link standing at the destination. Its completion then asked whether the name itself was that file. A link there was not, so a correct copy through an unchanged, in-destination link was refused, and the run exited 2. The refusal used `--ignore-existing`'s words ("something stands at the destination that was not there when this run looked"), with that option off.
**Status**: Fixed on Linux, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm33-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r34.codex.md`. It completes cr-ecm32-2 (`02522882`).

## Evidence
"I copied through an unchanged, in-tree destination symlink. The correct bytes landed, but `blit copy --retries 0` exited 2 and blamed `--ignore-existing`, which was off. The completion check compares the opened file's inode with the link's inode. Use the existing follow-link completion rule for ordinary copies and retain the stricter check for exclusive creation."

## Intake
Admitted as the reviewer put it.

## What
- `PartialTarget::arm_opened` arms an ordinary copy's cleanup for the file it opened, and notes that its open followed a link at the name (`follows`). `arm_made` stays for an exclusive creation, whose open never follows one.
- `PartialTarget::no_longer_its_own` follows the same rule as `still_its_own`, which a resume already used:
  - the name is the file written, or, for a write whose open followed a link, the name leads to it.
  - Anything else there is another's, and the copy did not land.
- The completion's refusal now names what happened. Under `--ignore-existing` the name was taken (`THE_NAME_IS_TAKEN`). In any other run, a file was put there while this one was written (`REPLACED_WHILE_WRITTEN`).

## Guard proof
- `sink::tests::an_ordinary_copy_through_a_destination_link_is_delivered` (Linux): `dst/link.bin → real.bin`, copied from `src/link.bin`. The copy is delivered (no failure), the link is kept, and `real.bin` holds the new bytes.
- `a_local_copys_cleanup_removes_only_the_file_it_opened` still holds.
- Mutation, restored byte for byte: the completion asking the name itself only → red.
- All 1,161 `blit-core` tests pass. Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `PartialTarget` (`follows`, `arm_opened`, `no_longer_its_own`), `copy_and_stamp_opened`; guard.

## Known gaps
- None known.
