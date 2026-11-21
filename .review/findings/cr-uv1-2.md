# cr-uv1-2: The recursive removal was not anchored to the directory it was listing

**Severity**: HIGH — a directory replaced by a link while `rm -r` runs could turn the deletion outside the module
**Status**: Reviewed, found incomplete — `utility-slices-r2` (Codex openreview, 2026-11-21) reviewed `4841b2d4..3cd49e40`, the seven `utility-slices-r1` fixes, with `guard_confirmed: true`. "Unix descriptor-relative deletion … worth keeping." Two things are not done: on Windows the removal still checks a path and then acts on it (the known gap below, now a finding), and on Unix every directory on the way to an operand is opened for reading, where `rm` needs only to search it (mode 0300). The remaining work is cr-uv2-2 and cr-uv2-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `1f2c2d09`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r1` over `b55efff7..49350572` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r1.codex.md`.

## Evidence
From source inspection, the reviewer: the walker reopens a child by pathname, obtains its deletion handle separately, and falls back to pathname removal if that handle is unavailable. A concurrent directory replacement can therefore direct deletion outside the module. Descend from the verified parent without following links, enumerate and remove through the same held directory, and report an entry failure when that guarantee cannot be established.

Cause. Slice 2 replaced the standard library's recursive removal — which is anchored — with a walk of its own, so that every failure is reported and the walk goes on. That walk found each directory by its path twice: `read_dir(path)` for the listing, which follows a link standing at `path`, and a separate open with `O_NOFOLLOW` for removal by name (`HeldDirectory`), which fails on a link — and then fell back to `remove_file(entry.path())`, the path through the link. The operand itself was checked for containment by its path and then removed by its path.

## Predicted observable failure
While `blit rm -r HOST:/m/tree` runs, a writer inside the module moves `tree/sub` away and puts a link to `/elsewhere` in its place: the walk lists `/elsewhere` and deletes its entries. The same for the operand's own directory, swapped between the containment check and the removal.

## Intake
Admitted as the reviewer put it.

## What
On Unix (`deletion::anchored::Dir`, `remove_operand`):
- **Nothing is found by a path twice.** The operand's directory is resolved to its canonical path (no link in it), which must lie in the module, and is then opened from the module's root down, one name at a time, each IN the directory before it with `O_NOFOLLOW`. A directory of the tree is opened the same way, IN the directory that listed it.
- **A directory is listed and emptied through the one descriptor it was opened with**: `fdopendir`/`readdir` for the entries, `unlinkat` for each by name, `openat` for each directory below.
- **No removal by path.** A directory that cannot be opened so is one reported failure and is left. A name the listing gave as a directory where no directory stands any more — a link put in its place — is removed as what it is, by name; what it points to is never entered.
- A directory that could not be reached a moment after it was resolved is reported with `-f` too: it was there.

Elsewhere (Windows) the tree is removed by the standard library's `remove_dir_all`, which is anchored to the handles it opens. It stops at the first entry it cannot remove: that operand gets one failure and whatever had not gone yet stays. The count of entries removed is taken from a walk before and, after a failure, a walk after; a count, never a decision.

Speed is unchanged or better: a directory is opened once, where it was opened twice by its whole path.

## Guard proof
- `deletion::rm_tests::a_directory_swapped_for_a_link_mid_walk_is_not_followed` (Unix): as the walk is about to go into `dir/sub`, which it listed as a directory, the directory is moved away and a link to another tree with the same names is put there (a test-only moment between the listing and the open). The other tree is whole, the link went as a link, the operand is gone, three entries counted.
- `deletion::rm_tests::an_operand_is_not_reached_through_a_directory_swapped_for_a_link` (Unix): the operand `dir/sub/y.txt` is resolved; then `dir` is moved away and a link to a tree holding `sub/y.txt` is put there. One failure, nothing removed in either tree, with and without `-f`.
- `deletion::rm_tests::a_directory_that_cannot_be_held_is_reported_and_left` (Unix): a directory of mode 000 in the tree is one failure, its contents stay, everything else goes.
- `deletion::rm_tests::a_held_directory_lists_and_removes_its_own_entries` (Unix): the listing (no `.` or `..`, a link to a directory told from one), a link not opened as a directory and removed as a link, a directory outside the root not reachable from it.
- The earlier guards stand: `a_symbolic_link_is_removed_and_never_followed`, `a_removal_goes_on_past_an_entry_it_cannot_remove`, and `rm.rs::rm_equals_the_system_rm` against GNU `rm`.
- Mutations, each restored byte for byte: a directory opened through a link standing in its place (no `O_NOFOLLOW`) → the first, second and fourth guards red; the operand's directory opened by its path, not from the root down → the second; a directory outside the root openable → the fourth; a link found where a directory was listed left and reported → the first; an entry's kind asked of what a link points to → `a_symbolic_link_is_removed_and_never_followed` and `rm_equals_the_system_rm`.

## Files changed
- `crates/blit-core/src/deletion.rs` — `remove_as_rm`, `remove_operand` (Unix and not), `anchored::Dir` in place of `HeldDirectory`, `Removal::fail`; the guards.
- `CHANGELOG.md`.

## Known gaps
- **Windows stops at the first entry it cannot remove** inside a tree, where slice 2 reported each and went on. Doing both there needs handle-relative opens, which the standard library uses and does not expose. Not run on Windows; it compiles for `x86_64-pc-windows-gnu`.
- The module's root is opened by its path: it is configuration, and a root that is itself swapped is outside what this guards.
- One descriptor is held per level of the tree being removed. A tree deeper than the process's descriptor limit fails at that depth with the system's error, reported for the directory that could not be opened.
- The listing's errors are told from its end by `errno`, reset before each read on Linux, Android, macOS and the BSDs; on another Unix an error reads as the end, and the directory then fails to be removed and is reported so.
- macOS: the code is written for it and not compiled or run there (no cross-compiler for that target on this machine). Run on Linux only.
