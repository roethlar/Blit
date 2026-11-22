# cr-uv8-1: neither read of what a file holds was tied to the file it was read for

**Severity**: HIGH — a move's removal can take the last name of a file holding bytes that were never sent, or somebody else's file put at the name
**Status**: Reviewed, found incomplete — `combined-r20` (Codex openreview, 2026-11-22) reviewed `17f4eb97..c15ebf5a` — the six `combined-r19` fixes — with `guard_confirmed: true`: "checking a move digest against the observed version … [is a] sound choice", but "A mismatched regular-file descriptor advanced by all 65,536 bytes in my probe before Blit rejected it. The final check protects the receipt. An initial check also avoids reading an unrelated file for the length of a potentially large source." The remaining work is cr-uv9-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `81b6c829`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r19` over `ca377805..17f4eb97` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r19.codex.md`. It completes cr-uv7-1 (`16379d85`).

## Evidence
"The scan reopens an absolute path without checking that it still holds the enumerated regular file. During removal, a write to an already-hashed block can leave the digest matching the receipt while the last unlink discards the new bytes. Hash through a verified descriptor, bound the read to the recorded size, recheck its version after hashing, and retain the name on any mismatch."

## Intake
Admitted as the reviewer put it. One look after the read stands for the look before it too: the time a file was last changed moves with every change and no program sets it, so a descriptor that is the version once read was that file, unchanged, from the look the version came from to the end of the read.

## What
- `moved::content_of(file, version)` — the one read, for both ends: through the descriptor it is given, no more than `version.size` bytes, and then the descriptor must be `version` (device, number, size, both times). Anything else is an error, and nothing is known of what the file holds.
- The walk (`FsTransferSource`, a move's scan): a file with several names is opened by its path with no link followed and no wait (`O_NOFOLLOW | O_NONBLOCK`) and read as the version this walk's look saw. Not that file, or changed: recorded without its content, and its later names stay.
- The removal (`remove_asking`): a later name's content is read through the descriptor opened at the name (as before), as the version the removal's look found there. Not that file, or changed by the end of the read: unreadable, and the name stays.

## Guard proof
- `moved::content_tests::a_content_is_known_only_of_the_file_as_it_was_seen`: unchanged — its digest; its first bytes rewritten once read (as many, its time put back) — error; another file of the same bytes and time — error.
- `moved::content_tests::a_content_read_goes_no_further_than_the_size_seen`: a file grown as it is read — no more than its seen size read, and an error.
- `transfer_session::…::a_moves_walk_reads_only_the_file_it_saw`: between the walk's look and its read, another file of the same bytes and time renamed over one name, and another file rewritten in place with its time put back — no content recorded for either name.
- `deletion::…::a_later_names_content_is_read_from_the_file_seen`: before a later name is read, somebody else's file of the same bytes and time renamed over it — it stays, whole; once read, the file's first bytes rewritten (time put back) — it stays, with what was written.
- Mutations, each restored byte for byte: nothing looked at once read (red: the first and the removal's guards, and the other two); the read on to the end of the file (red); the walk reading whatever it opens (red); the walk reading the file as it is when opened (red); the removal reading whatever it opens (red).
- The four guards green three runs in a row. The whole suite in the scratch copy: 1,957 passed, 0 failed.
- Speed: a move of 20,000 files from a loopback daemon, tmpfs, median 231.2 ms before and 231.3 ms after.

## Files changed
- `crates/blit-core/src/moved.rs` — `content_of`, a seam; guards.
- `crates/blit-core/src/remote/transfer/source.rs` — the walk's read, a seam.
- `crates/blit-core/src/deletion.rs` — `remove_asking`, `ByName::content_of`, `Content` doc; guard; `held_of`.
- `crates/blit-core/src/transfer_session/mod.rs` — guard; the sibling's digests.
- `crates/blit-daemon/src/service/admin.rs` — a guard's digest.

## Known gaps
- Between the removal's look after the read and its unlink of the last name, a write is still lost; that window is every removal's (a look, then an unlink — there is no "unlink if unchanged"), not one this read adds.
- A filesystem whose time of last change is coarse — or a kernel that keeps it to the tick — can show a change within that tick as no change; this holds for every `Version` comparison.
- Not raised: the walk's readability probe (`open_for_scan`) opens each file by its path with a plain open, so a FIFO put at a file's name between the enumeration's look and the probe waits there for a writer. That is older than this, and not changed.
- Windows: no file identity or time of last change; contents are not read there (cr-uv6-3).
