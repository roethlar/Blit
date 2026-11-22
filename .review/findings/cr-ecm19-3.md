# cr-ecm19-3: the creation guard settled with no length

**Severity**: HIGH — a record of one byte cancelled where its file cannot be removed leaves one zero byte, which the next run takes for a copy
**Status**: Verified — `combined-r21` (Codex openreview, 2026-11-22) reviewed `c15ebf5a..b92c9a17` — the seven `combined-r20` fixes — with `guard_confirmed: true`: "The creation guard now uses the record's actual length"
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `00edeca4`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r20` over `17f4eb97..c15ebf5a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r20.codex.md`. It completes cr-ecm18-1 (`ba0191ee`).

## Evidence
"I cancelled a one-byte record after open in a directory that allowed writing the file but prevented unlinking it. `Made::drop` supplied zero as the source length, so cleanup left one zero byte. The default comparison then skipped the file. Carry the header size into `Made` so an unremovable partial remains distinguishable from a complete copy."

A file that cannot be removed is marked by its length: one byte longer than its source. Settled as of a source of no bytes, it was one byte long.

## Intake
Admitted as the reviewer put it.

## What
`create_stream_destination` takes the record's size and `Made` keeps it; unclaimed, it is settled with that size, so a file left behind is one byte longer than the record.

## Guard proof
- `sink::…::a_record_cancelled_where_its_file_stays_is_left_longer_than_it`: records of 1 and 10 bytes; the seam in the step that makes the file closes the directory to removals (mode 555) and holds; the record is cancelled; the file left is 2 and 11 bytes long. Skipped, and says so, for a process a directory's mode does not bar. Three runs, green each time.
- Mutation, restored byte for byte: the guard settling as of no size (red).
- The sink tests (102) pass.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `create_stream_destination`, `Made`; guard.

## Known gaps
- The guard proves nothing run as root, which a directory's mode does not bar; it says so and passes.
