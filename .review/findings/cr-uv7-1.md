# cr-uv7-1: a later name of a file with several could go as the file that was sent, rewritten

**Severity**: HIGH — a move removes the last name of a file rewritten (as many bytes, its time put back) while the removal was between its two looks at an earlier name: the rewritten bytes were sent to nobody
**Status**: Reviewed, found incomplete — `combined-r19` (Codex openreview, 2026-11-22) reviewed `ca377805..17f4eb97` — the eight `combined-r18` fixes — with `guard_confirmed: true`. "The scan reopens an absolute path without checking that it still holds the enumerated regular file. During removal, a write to an already-hashed block can leave the digest matching the receipt while the last unlink discards the new bytes. Hash through a verified descriptor, bound the read to the recorded size, recheck its version after hashing, and retain the name on any mismatch." The remaining work is cr-uv8-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `16379d85`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It completes cr-uv6-1 (`17ac7abe`).

## Evidence
"On Unix, a same-length rewrite that preserves mtime during hard-link removal passes the new comparison because it discards ctime. Cleanup can then delete the last name for unsent bytes … Verify the delivered content in the ambiguous hard-link case, capture Windows identity and change time from a handle, and retain the source whenever verification fails."

Taking one name of a file changes the time it was last changed; cr-uv5-3 lets a later name go as the file stood before, while the file stands exactly as the removal's own unlink left it, and cr-uv6-1 refused that when the size or the time written had changed too. A rewrite of as many bytes whose time is put back, landing between the look that lets the earlier name go and the look after its unlink, changes nothing a look can see but the time last changed — which the unlink changes anyway. No look at the file tells the two apart. (cr-uv6-1's record named this as its gap.)

## Intake
Admitted, the Unix half. Verifying "the delivered content" is done against what the walk read — the bytes a move sends are the file the walk saw, or the move's first name of it is refused (cr-uv5-2) and no later name is asked about this way. The Windows half is cr-uv6-3's.

## What
- The walk of a move reads what a file with SEVERAL names holds (`moved::content_of`, BLAKE3), and the account keeps it by path (`Account::record_content`, `Closed::holds_content`); a path that did not land is struck from it as from the versions. A file of one name is not read: nothing asks for it.
- When the removal would take a later name on the strength of its own unlink, it reads what that name holds (`ByName::content_of`: by its name in the held directory, never through a link) and asks the account for it (`deletion::Content`). The daemon's check takes the name only if the account holds that content for that path; one that cannot be read stays.

## Guard proof
- `deletion::…::a_rewrite_no_look_can_see_keeps_the_other_name`: at the seam between the look and the unlink, the other name is rewritten with as many bytes and its time put back; that name stays, with what was written.
- `blit-daemon` `…::a_later_name_goes_only_with_the_content_that_was_sent`: through the daemon's own removal — with the content the walk read, both names go; with other content, the later one stays.
- `transfer_session::…::a_moves_walk_reads_a_file_with_several_names`: the walk reads it under each name, and does not read a file of one name.
- `moved::…::an_account_holds_what_a_file_held_by_its_path`: by path, not another; struck when it did not land.
- `remote_move::a_move_takes_every_name_of_a_file_with_several` (cr-uv5-3's, end to end) passes: the content matches, every name goes.
- Mutations, each restored byte for byte: a later name asked about by its version alone (the rewrite guard red, and the daemon's); the walk reading nothing (the walk guard and the end-to-end move red); the daemon taking a later name whatever it holds (red); a path that did not land keeping its content (red).
- The whole suite in the scratch copy: 1,951 passed, 0 failed. A move of 20,000 ordinary files: 334.4 → 336.3 ms median (tmpfs) — they are not read.

## Files changed
- `crates/blit-core/src/moved.rs` — `content_of`, `content_sighting`, `Account::record_content`, `Closed::holds_content`, `Sightings::publish_contents`; guard.
- `crates/blit-core/src/deletion.rs` — `Content`, `Met::content`, `ByName::content_of`, `remove_asking`; guard.
- `crates/blit-core/src/remote/transfer/source.rs` — the walk.
- `crates/blit-core/src/transfer_session/mod.rs` — guard.
- `crates/blit-daemon/src/service/admin.rs` — the daemon's check; guard.
- `CHANGELOG.md`.

## Known gaps
- A move of a tree of many files with several names reads each of them once more at the walk and each later name at the removal; not measured.
- Between reading a later name's content and unlinking it, a write by another hand is not seen: a window of a few system calls, which every look before a removal has.
- A file that gains a name after the walk is refused at its first name (its time last changed moves), and so at all of them.
- Windows: cr-uv6-3.
