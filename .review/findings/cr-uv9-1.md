# cr-uv9-1: a daemon read was not bound to the start its check passed

**Severity**: HIGH — `ls`, `du` or `find` can read outside the module: a start, or a directory below it, replaced by a link after the check, is read through the link
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv9-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r20` over `17f4eb97..c15ebf5a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r20.codex.md`. It completes cr-uv8-2 (`31e8a0c6`).

## Evidence
"A missing `missing/child` below an existing module returned `Readable::At` with the full pathname. The verb resolves that name again, so a concurrent path change can take the read outside the checked module. I also observed a module whose parent became a regular file turn `ENOTDIR` into gRPC `PermissionDenied`. Carry observed operand errors to the verb and anchor successful reads to the verified directory."

## Intake
Admitted as the reviewer put it, and carried to the walk below the start: each walk descended by `read_dir(entry.path())`, a path looked up again from the top for every directory — a directory replaced by a link between its listing and its reading led the walk wherever the link pointed (the class `rm` was closed against in cr-uv1-2).

## What
- The check (`start_is_contained`): a start that cannot be looked up is told with the error the check saw (`Readable::Gone`) — nothing is read by its name again; a module a folder above which was made a file (`Not a directory`) is told so, as a missing one is (cr-uv7-2), not refused.
- `blit_core::held` (new): what a read-only walk reads, through what holds it. On Unix the start is reached from the module root one name at a time, none of them a link (a link the start was typed to be read through is resolved first, and the way to where it leads is opened the same way — what lies outside the root is refused); each directory is held open and read through that — entries listed from it, looked up in it (`fstatat`), entered in it (`openat`, no link followed), links read in it. A directory replaced by a link after that is an error for that entry, not a way out. Elsewhere, by path, as before.
- `listing::list`, `disk_usage::disk_usage_in` and `find::find_in` read through it; `disk_usage` and `find` are the same with no root. The daemon's `ls`, `du` and `find` pass the module's root.

## Guard proof
- `typed_operand_tests::a_start_is_judged_by_the_part_of_it_that_can_be_looked_up` (changed): `ring/x`, `ring/x/y`, `ring/`, `missing/x` are `Gone` with the error seen; a module a folder above which was made a file is `Gone` with `NotADirectory`.
- `find::tests::a_directory_replaced_by_a_link_as_the_walk_reaches_it_is_not_gone_into` and `disk_usage::tests::a_directory_replaced_by_a_link_as_the_walk_reaches_it_is_not_counted_into`: a seam replaces `a` with a link to a folder outside just before the walk enters it — nothing outside is found or counted, and `a` is an entry that could not be read.
- `listing::tests::a_listing_reads_nothing_outside_its_root`: the start replaced by a link as it is about to be read; a directory on the way to what `link/` leads to replaced once the link has been followed; `out/`, a link out of the root — each an error with nothing outside listed.
- Mutations, each restored byte for byte, six: the walk entering a directory through a link (both walk guards red); the start entered through a link; the way to a start through links; a start read wherever its path leads (each the listing guard red); a start that cannot be looked up read by its name again; a module whose way is a file refused (each the check's guard red).
- The core walk tests (47), the daemon's (220), and the CLI's `ls`, `find`, `du`/`df`, admin, `rm` and `move` tests pass, a module that is one file among them. Windows cross-clippy clean.
- Speed, 200,000 empty files in 2,000 directories through a loopback daemon: `du` 142.9 → 124.9 ms; `find`, every path, median 74.2 → 66.8 ms; `find -delete` 290.3 → 293.3; `ls` of one directory of 200,000 files 85.6 → 83.8 ms, `ls -l` 233.1 → 208.8 ms.

## Files changed
- `crates/blit-core/src/held.rs` (new), `crates/blit-core/src/lib.rs`.
- `crates/blit-core/src/listing.rs`, `crates/blit-core/src/disk_usage.rs`, `crates/blit-core/src/find.rs` — the walks; guards.
- `crates/blit-daemon/src/service/admin.rs` — `start_is_contained`, `Readable`, the three verbs; guard.

## Known gaps
- Windows: by path, as before; nothing there holds a directory to read in it.
- The module root itself is opened by its path, which is resolved when the daemon starts (cr-uv6-2 asks that it still resolves to itself); what lies above it is the host's.
- `find -delete`'s own checks before it removes (a link on the way, a start that is a link to a directory) still look at the start by its path; the removal itself was anchored in cr-uv1-2.
- The local CLI's `ls` (no root) reads through the same walk, its start opened by its path.
