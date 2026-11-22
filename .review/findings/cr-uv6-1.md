# cr-uv6-1: a write between a removal's two looks was taken for the removal's own change

**Severity**: HIGH — a move removes the last name of a file with bytes appended to it that nobody was sent
**Status**: Reviewed, found incomplete — `combined-r18` (Codex openreview, 2026-11-22) reviewed `7912da16..ca377805` — the six `combined-r17` fixes — with `guard_confirmed: true`. "On Unix, a same-length rewrite that preserves mtime during hard-link removal passes the new comparison because it discards ctime. Cleanup can then delete the last name for unsent bytes … Verify the delivered content in the ambiguous hard-link case … and retain the source whenever verification fails." (This record's own known gap, required.) The Windows half is cr-uv6-3's. The remaining work is cr-uv7-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `17ac7abe`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It completes cr-uv5-3 (`55094166`).

## Evidence
"The move records the entire version observed after unlinking the first name. An ordinary append before that observation becomes part of the version it attributes to its own unlink. The next name can then pass as the original file, deleting the last copy of the appended bytes. Reject changes in size or modification time and retain the remaining name whenever the content version is ambiguous."

cr-uv5-3 looks at a file with several names before a name goes (it must be the file that was let go) and again after, and records the second look whole as "how this removal left it". Anything done to the file between the two looks was in that record. cr-uv5-3's own known gap named only a write that keeps the size and puts the time back; an ordinary append is in the same window, and worse, since nothing has to be put back.

## Intake
Admitted as the reviewer put it.

## What
`remove_asking` puts how a file was left on record only when the second look differs from the first in nothing but the time the file was last changed — the same file, the same size, written at the same time (`left_by_an_unlink_alone`): what taking a name does, and no more. Otherwise nothing goes on record, and the file's other names are asked about as they stand — and stay, as any changed file's do.

## Guard proof
- `deletion::…::a_write_between_a_removals_two_looks_keeps_the_other_name`: at a new seam between the look that holds the file and the unlink, the file is appended to by its other name. That name stays, with what was written.
- Mutations, each restored byte for byte: however the file stands after its name went taken for the removal's own doing, as it was (the new guard red); nothing ever put on record (cr-uv5-3's guard red).
- The end-to-end moves (`remote_move`, 14) pass.

## Files changed
- `crates/blit-core/src/deletion.rs` — `remove_asking`, `left_by_an_unlink_alone`, a seam in `remove_one_name`; guard.

## Known gaps
- A write between the two looks that changes neither the file's size nor the time it was written — its time put back to the nanosecond — is still taken for the removal's own change of the time last changed. Those two looks are a few system calls apart, and no look at a file can tell that case from an unlink.
- Unix only; on Windows nothing moves when a name goes (cr-uv6-3).
