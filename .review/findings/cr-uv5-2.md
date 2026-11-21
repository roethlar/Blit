# cr-uv5-2: a move removed a file as rewritten, whose new bytes nobody had been sent

**Severity**: HIGH — data loss with exit 0: a file rewritten while a move writes its manifest goes from the source, and its new bytes are nowhere
**Status**: Fixed on Unix, awaiting review; the Windows half is open and blocked (below)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv5-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. It completes cr-uv4-1 (`c4241da0`).

## Evidence
"On the `move --checksum` path, a same-size edit in the same timestamp second can occur after hashing and before the receipt's separate stat. The destination can accept the earlier checksum while cleanup authorizes the newer contents for deletion. Windows also records zero for file identity and change time, so a replacement preserving size and mtime can pass. Capture the version from the open file supplying the bytes or checksum and revalidate it at cleanup."

The reviewer traced it in the code. It is reproduced here end to end (Linux, tmpfs, the build before this commit): `blit move --checksum HOST:/m/tree/ dest/`, the tree holding one small file and six that take a while to hash, the destination already holding the small file's bytes under another time (so by checksum nothing of it is sent); 250 ms into the move another hand rewrites the small file — as many bytes, its time put back.

```
before   run 0..4: exit 0 | source: gone | destination b'1234' | LOST   (5 of 5)
after    run 0..4: exit 1 | source b'4321' | destination b'1234' | kept at the source   (5 of 5)
```

Cause. cr-uv4-1 holds a file in a move's account as the file it was, and the removal takes it only while it stands exactly so. But WHEN it was looked at for the account was the manifest's end, in a pass of its own (`account_of`), and that pass took any file still of the size and the whole second the manifest wrote. Everything read of a file before that — its checksum, hashed as the manifest is written; its bytes, which go before the manifest's end on the TCP lane — was read from a file the account never saw. Rewritten in between within the size and the second, the file went into the account as rewritten.

It is wider than the checksum path the reviewer names: a copy's bytes can be sent before the manifest is complete, and the same window stands there.

## Intake
Admitted. The remedy is the reviewer's in what it binds, and simpler in where it looks: the version is taken at the walk's own look at the file — the look the manifest's size and time come from, which is before anything of the file is read — and not at each handle that later reads it. One look that comes first binds every read that follows: a file changed after it, by the time the removal comes, is another file wherever the change fell (the time a file was last changed moves with every change and no program sets it). Taking the version again at each open would bind nothing more, and would have to be carried from four places that read (the hash, the stream, the shard, the block-resume) instead of one.

**The Windows half is open.** On Windows what an account holds of a file is its size and the time it was last written: the standard library gives neither the time of last change nor which file it is without calls that are unstable in it. A replacement, or a rewrite, that keeps the size and puts the time back therefore passes there, as the reviewer says, and this commit does not change that. What it needs — the file's identity (`transfers::filter::entry_identity` already reads it, for another purpose) and its change time (`GetFileInformationByHandleEx`, `FileBasicInfo`), in `moved::Version::of` and in the removal's own look — is code that runs only on Windows, in the path that deletes a source, and no Windows host has been reachable to run it on (cr-uv2-2 and cr-uv3-4 wait on the same). It is not landed unrun.

## What
- `moved::Sightings`: what a walk saw of each file, by the file's name — the digest an account holds, made from the walk's own look. A walk records only once asked (`begin`); `account(root, listed)` gives the account of the files that went on the manifest, each as seen. A file the walk saw that the manifest does not list (a filter left it out) is not in it.
- `TransferSource::sightings()`: `FsTransferSource` keeps them and its manifest task records each file from the `Metadata` the enumeration already holds — no further look at the disk; the wrappers delegate. A source that reads no disk gives none, and a move from it is given no account and removes nothing.
- The session's send half asks the walk to record before it begins, keeps each manifest entry by its name, and at the manifest's end writes the account from what was seen. `account_of` and its pass over the disk are gone.
- Nothing on the wire changes; the removal is as it was.

## Guard proof
- `crates/blit-cli/tests/remote_move.rs::a_move_never_removes_bytes_nobody_was_sent` (end to end; the reproduction above as a test): whenever the rewrite lands, the rewritten bytes are at the source still or at the destination, and a file left is named with exit 1. **Proved by taking the fix out**: with `crates/blit-core/src` as it was before this commit and the guard kept, it fails — "the rewritten bytes are nowhere: gone from the source, and the destination holds "1234"" — twice of two runs; with the fix back, it passes. It can only go red when the rewrite lands between the hash and the manifest's end, which its sizes make most of a second wide; it is green wherever the rewrite lands.
- `transfer_session::…::a_moves_account_holds_each_file_as_the_walk_saw_it` (in place of the guard for the pass that is gone): a real walk of a tree, then — the files rewritten with as many bytes and the time put back, grown, given another time, gone, a link put in its place, another file put in its place: none is held; the untouched ones are; a file the manifest does not list is not; a name that is not text is held by its bytes; a source that is one file, under no name.
- `moved::tests::an_account_is_written_from_what_the_walk_saw`: listed and seen is held; seen and not listed, listed and never seen, are not; nothing before a walk is asked; a new walk starts from nothing.
- Shown first against the code as it stood, with the old function: a file rewritten after the walk — as many bytes, its time put back — is held by the account written at the manifest's end ("the removal would take the file as rewritten: bytes nobody was sent").
- Mutations, each restored byte for byte: the account holds whatever the walk saw, listed or not (both unit guards red); the walk not asked to record (every move from a daemon red — nine guards); a name that is not text recorded by its text (the unit guard and the two-names move red); what the walk saw not handed over (the unit guard and every move red).
- The whole suite in the scratch copy before porting: 1,928 passed, 0 failed.
- Speed, a move of 20,000 files from a loopback daemon, median of five: 339.5 ms before, 339.4 ms after on tmpfs; 507.5 and 510.3 ms on Btrfs.

## Files changed
- `crates/blit-core/src/moved.rs` — `Sightings`, `sighting`, `name_key`; guard.
- `crates/blit-core/src/remote/transfer/source.rs` — `TransferSource::sightings`, `FsTransferSource`, the manifest task, the three wrappers.
- `crates/blit-core/src/transfer_session/local.rs` — the wrapper there delegates.
- `crates/blit-core/src/transfer_session/mod.rs` — the send half; `account_of` removed; guard.
- `crates/blit-cli/tests/remote_move.rs` — guard.
- `CHANGELOG.md`.

## Known gaps
- **Windows**, as above: open, and blocked on a host to run it on.
- A file changed after the walk's look and delivered AS CHANGED stays at the source with exit 1, though the destination holds its bytes: the account cannot tell that from a file whose new bytes went nowhere. A second run moves it.
- Where a system keeps the time of last change coarsely (a clock tick, on older Linux kernels — an assumption about them, not measured here), a rewrite inside the same tick as the walk's look, keeping the size and the time written, is not seen. Before this fix that window was the whole time from the look to the manifest's end.
- The test `a_moves_account_holds_each_file_as_it_stands_on_this_disk` is gone with the function it guarded; the count of tests rose all the same (1,926 to 1,928).
- The end-to-end guard makes twelve files of 512 MiB that are all holes, and takes about a second and a half.
- Linux only.
