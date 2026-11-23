# cr-ecm31-6: the release note did not state the accepted limits, and a fresh-inode guard could be fooled

**Severity**: MEDIUM — the `--resume` entry of `CHANGELOG.md` did not state the two limits the owner accepted (D-2026-11-23-1, D-2026-11-23-2); and guards that a fresh partial has a new inode compared numbers with the old file released, which a filesystem may hand straight back (ext4)
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm31-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r32` over `93ecf7b2..714ed0b7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r32.codex.md`. It completes cr-ecm30-8 (`f4b1249b`).

## Evidence
"The release note should state the two limits the owner accepted. An arrival between the final check and unlink can be removed, and an automatic retry can overwrite a file preserved by the preceding pass. The fresh-file test should also hold the old file open while comparing inode numbers. A disposable ext4 image reused the same number immediately after removal."

## Intake
Admitted as the reviewer put it.

## What
- `CHANGELOG.md`, the `--resume` entry, now states both limits. Something put at the name between the last look and the removal is removed with it. An automatic retry copies over whatever stands at the name by then, a file the pass before it left as it is included.
- Three guards now hold the old file open while comparing inode numbers, so its number cannot be given to the fresh partial:
  - `resume::tests::a_fresh_partial_is_made_at_the_name_itself`;
  - `resume::tests::a_source_of_no_mode_bits_is_more_private_than_its_destination`;
  - `sink::tests::a_wire_partial_through_a_link_is_replaced_whole_or_not_at_all`.
- Two others already did: one through a reader it holds, one through a hard link.

## Guard proof
- The three guards pass with the old file held open.
- No mutation applies: the change keeps a guard from passing by chance, and changes no behaviour. Its red case needs a filesystem that reuses a freed number at once. Neither filesystem the gate runs on (tmpfs, Btrfs) does, and no ext4 image can be mounted here without root. Not shown red here.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs`, `crates/blit-core/src/remote/transfer/sink.rs` — the guards.
- `CHANGELOG.md`.

## Known gaps
- Not run on ext4.
