# cr-uv6-4: what a walk saw was kept on a source that scans can share

**Severity**: HIGH — one move's account can hold a file as another scan of the same source saw it, and remove a version nobody sent
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv6-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It completes cr-uv5-2 (`f4e1ab22`).

## Evidence
"The built-in daemon creates a fresh source per session, but the public fixed-source responder accepts a shared `Arc`. Overlapping scans on that source can clear or overwrite each other's observations, so one move can authorize removal of a version another scan saw. Attach the observations to `SourceScan`."

cr-uv5-2 kept `moved::Sightings` on `FsTransferSource`: a move set it recording (`begin`, which also cleared it), the walk published into it, and the account was read from it at the manifest's end. A second scan of the same source in between cleared the first one's record and wrote its own over it.

## Intake
Admitted as the reviewer put it.

## What
- `TransferSource::scan_seeing(filter, unreadable, preserve_windows_metadata)`: a scan that records what its walk sees, the record handed back in that scan's own `SourceScan` (`SourceScan::sightings`). `FsTransferSource` makes a fresh `Sightings` for each such scan; the default scans as ever and records nothing (a move from such a source is given no account and removes nothing, as before).
- Every wrapper passes the request on to its inner source (`FilteredSource`, `ChecksummingSource`, `WindowsMetadataDroppingSource`, and `DestSubtreeExcludedSource` on this machine); the record comes back through them with the `SourceScan` they already pass along.
- `Sightings` loses `begin` and `wanted`; `TransferSource::sightings` and the field on `FsTransferSource` are gone. The session asks a move's scan by `scan_seeing` and writes the account from that scan's record.
- Nothing on the wire changes.

## Guard proof
- `transfer_session::…::two_scans_of_one_source_keep_their_own_record`: one source shared by two scans, the file rewritten between them; the first walk's account holds the file as the first walk saw it and not as the second did, and the other way round; a plain scan records nothing.
- `transfer_session::…::every_wrapper_passes_the_request_to_record_on`: through the filter, the checksum, the one that drops Windows metadata, and all three nested — each scan comes back with a record that holds the file.
- `a_moves_account_holds_each_file_as_the_walk_saw_it` (cr-uv5-2's) now walks by `scan_seeing`.
- Mutations, each restored byte for byte: one record kept for every scan (the two-scans guard red); a move's scan not asked to record (every move from a daemon red); each of the three wrappers scanning its inner source the plain way (the wrapper guard red; for the checksum, `a_move_never_removes_bytes_nobody_was_sent` too).
- The whole suite in the scratch copy: 1,937 passed, 0 failed.

## Files changed
- `crates/blit-core/src/moved.rs` — `Sightings`; guard adjusted.
- `crates/blit-core/src/remote/transfer/source.rs` — `SourceScan::{seeing, sightings}`, `TransferSource::scan_seeing`, `FsTransferSource`, the three wrappers.
- `crates/blit-core/src/transfer_session/local.rs` — the wrapper there.
- `crates/blit-core/src/transfer_session/mod.rs` — the send half; guards.

## Known gaps
- `DestSubtreeExcludedSource` (a copy on this machine into its own source) passes the request on and has no guard of its own: a move whose source is on this machine does not keep an account (it removes its whole source tree, as recorded for the owner).
- A source outside this crate that implements `TransferSource` and wraps another must override `scan_seeing` to pass it on; if it does not, a move through it removes nothing — the safe side.
- Linux only.
