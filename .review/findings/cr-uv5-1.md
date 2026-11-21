# cr-uv5-1: a move from a daemon left at its source what it had delivered

**Severity**: HIGH — a move of more than 128 files from a daemon delivers everything, leaves part of the source in place, and exits 1
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv5-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. **And the working agent, independently**: it met the fault the same hour, measuring a move of 20,000 files while that review was still running, and had this fix written and proven before the review's result was read. `utility-slices-r3`'s fix cr-uv3-1 (`57e8ee24`) introduced it, `combined-r15` reviewed that fix without meeting it, and cr-uv4-1 (`c4241da0`) rewrote the code around it and kept it.

## Evidence
The reviewer: "I moved 2,048 ordinary, unchanged files. All 2,048 arrived with the correct contents, but 384 remained at the source and the command exited 1. Incoming requests consume entries from the same map that cleanup later snapshots. Record each file independently as it is processed, then confirm the receipt from the destination's summary."

The working agent's own measurement:

`blit move -y HOST:/bench/tree/ ./landed/` of 20,000 files of 64 bytes in 200 directories, through a loopback daemon, release builds, Linux (`continuum`), 2026-11-21:

- the build at `f1a9055d` (slice 5) and the build at `33ac6700` (after every `combined-r15` fix), tmpfs and Btrfs alike: all 20,000 files landed; **3,840 (4,224; 3,328; 3,712 on other runs) stayed at the source**, each reported `cannot remove 'tree/d0199/f099': it is not a file the move sent: it is new, or has changed since it was read`; exit 1. None of the files left had changed: write time, change time and inode as planted.
- what was left was the HEAD of the walk — the first directories the filesystem listed (the last by name on tmpfs, the first on Btrfs).
- smaller trees: 1,000 files leave 128, every run; 3,000 leave 384 or 512; 6,000 leave 896 or 1,152. Always a multiple of 128.

Cause. The source end keeps a map of the manifest (`sent`: text → header) so that a need from the other end can be answered with the entry it names — and a need TAKES its entry out of the map (`sent.remove`). Needs arrive in batches of 128 while the manifest is still being sent. cr-uv3-1 wrote the move's account, when the manifest was complete, from what the map then held: every file already asked for was missing from it. The removal that ends the move then found those files, unchanged, in no account. The guards for cr-uv3-1 and cr-uv4-1 move a handful of files: their needs all arrive after the manifest's end.

It fails toward keeping: nothing is lost, and the same command run again finds everything at the destination, asks for nothing, and removes the rest. But a move of any real tree did not do what it said, and said something untrue about why.

## Intake
Admitted as the reviewer put it; the fix is the one the reviewer names.

## What
The entries a move's account is written from are kept as they go onto the manifest — each one's name, size and time, nothing else — in a list of the send half's own (`listed_for_account`), and the account is written from that list at the manifest's end. The map of the manifest is no longer asked: it was never a record of what had been listed.

## Guard proof
- `crates/blit-cli/tests/remote_move.rs::a_move_of_many_files_removes_every_one_it_delivered` (end to end): 1,000 files in ten directories moved from a daemon — exit 0, all 1,000 at the destination, none at the source, the emptied directories gone.
- The ten guards of `remote_move.rs` that stood before, the account's own (`moved::tests`, `a_moves_account_holds_each_file_as_it_stands_on_this_disk`) and the daemon's (`a_moves_removal_takes_what_its_sessions_sent`) hold unchanged.
- Mutations, each restored byte for byte: the account written again from what is left of the map (the new guard red, and ONLY it — the ten earlier guards stay green, which is how this stood for two review rounds); no entry kept for the account as it goes onto the manifest (the new guard red, and six earlier ones).
- Measured after the fix, the same 20,000 files, five runs on each filesystem, every run exit 0 with the source empty and everything landed: 335 ms on tmpfs, 516 ms on Btrfs (best of five; medians 336 and 519).

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — the list kept as the manifest is sent; the account written from it.
- `crates/blit-cli/tests/remote_move.rs` — guard.

## Known gaps
- **No timing "before" for a large move exists**: before this fix such a move did not complete. The numbers above are the first.
- The list is one more copy of each listed file's name, for a move's source end, until the manifest is complete: memory in proportion to the tree, as the map of the manifest already is.
- The guard is 1,000 files, which is past one batch of needs on this machine every time it was tried (three runs each at 1,000, 3,000, 6,000 and 2,400 in four directories). A machine on which no need arrives before the manifest's end would not show the fault; the mutation shows the guard catches it here.
- A move between two daemons takes the same code on the source daemon; it was not measured.
- Linux only.
