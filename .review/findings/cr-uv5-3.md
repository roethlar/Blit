# cr-uv5-3: a move left the second name of a file with two

**Severity**: MEDIUM — a move of a file with several names delivers them all, leaves all but the first at the source and exits 1
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv5-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. It completes cr-uv4-1 (`c4241da0`).

## Evidence
"In a two-name hard-link fixture, both files arrived. Removing the first name changed the inode's ctime, so the move rejected the second name as changed and exited 1. Track the link-count change caused by cleanup while retaining the checks for an independent write or replacement."

Cause. cr-uv4-1 holds a file in a move's account as the file it was — its size, its time to the nanosecond, when it was last changed in any way, which file it is — and the removal takes a name only while the file stands exactly so. Taking a name is itself a change to the file: the time it was last changed moves. Every further name of it then failed the check the removal's own unlink had broken.

## Intake
Admitted as the reviewer put it.

## What
In `deletion.rs`, for a removal that asks before it removes (a move's; plain `rm` asks nothing and is as it was):
- `Met` says how many names the file has (`links`).
- A file with other names is removed by `ByName::remove_one_name`: the file is HELD by that name first (`O_PATH` on Linux; opened to be read, without waiting, where there is no such way), must still be the file that was looked at and let go — anything else there by then is left, with "it has changed since it was read" — the name is removed, and the file is looked at again through what is held: how this removal's unlink LEFT it.
- `OwnChanges`, one for each operand removed: by which file it is, the file as it stood before this removal touched it and as its latest unlink left it. `remove_asking` asks `keep` about a further name of that file as it stood BEFORE — but only while it stands exactly as this removal left it. Written to, replaced or changed in any way by anyone else since, it is asked about as it is, and stays, as a file with one name would.
- Nothing in the account or on the wire changes.

## Guard proof
- `deletion::…::a_file_with_several_names_goes_by_each_that_was_sent`: three names of one file in two directories, and a file beside them, all held — all go, and the tree with them; then two names held and a third not — the two go, the third stays with the file's bytes.
- `deletion::…::a_file_changed_after_one_of_its_names_went_keeps_the_other`: between the two names of a file the file is appended to by another hand (a hook at a new stage, `removed`); the other name stays, with what was written.
- `crates/blit-cli/tests/remote_move.rs::a_move_takes_every_name_of_a_file_with_several` (end to end, the reviewer's fixture with a third name in a directory below): a move from a daemon, exit 0, every name at the destination with the file's bytes, nothing left at the source.
- Mutations, each restored byte for byte: a file asked about as it stands whatever this removal did to it (the first and the end-to-end guard red); asked about as it stood whatever was done to it since (the second red); how the file is left not put on record (first and end to end red); the record taken from the file as it stood after the first name went (first and end to end red — it is the third name that shows it).
- Run on tmpfs and on Btrfs.

## Files changed
- `crates/blit-core/src/deletion.rs` — `Met::links`, `OwnChanges`, `remove_asking` (in place of `stays`), `ByName::remove_one_name`, `version_of`, `version_held`; guards.
- `crates/blit-cli/tests/remote_move.rs` — guard.

## Known gaps
- Between the look that lets a name go and the look that says how the file was left, a write by another hand that also keeps the file's size and puts back its time is taken for this removal's own change. That is the window every check-then-remove has, here a few system calls wide; it is not new, and it is narrower than it was for a file with other names (the file is held, and compared, before its name goes).
- Names of one file in two OPERANDS of one removal: the record is kept for each operand, so the second operand's name stays, as before this fix. A move's removal names one operand, its source.
- Where a file cannot be held (off Linux, a file that may not be read), its name goes and its other names stay, as before this fix. Not run on macOS or FreeBSD; the open flags there are untried.
- Windows keeps neither the time of last change nor which file it is in what an account holds (cr-uv5-2), so nothing moves when a name goes, and nothing here applies.
- The destination writes each name as a file of its own: a move does not carry the fact that they were one.
