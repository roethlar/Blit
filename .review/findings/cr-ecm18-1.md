# cr-ecm18-1: a record's file was not owned from inside the step that made it, nor through commit's last await

**Severity**: HIGH — a cancelled transfer leaves an empty file at the destination, which every later run keeping what exists skips as somebody's; a record cancelled in commit is settled by its name alone
**Status**: Reviewed, found incomplete — `combined-r20` (Codex openreview, 2026-11-22) reviewed `17f4eb97..c15ebf5a` — the six `combined-r19` fixes — with `guard_confirmed: true`: "Arming cleanup inside file creation … [is a] sound choice", but "I replaced the destination during a stream. Commit reported one file written and zero failures, although the destination contained the replacement. The writer records identity for every stream but uses it for delivery only under `--ignore-existing`. A move can consequently authorize removal of an undelivered source. Use the recorded identity at commit, accounting for supported writes through links." and "I cancelled a one-byte record after open in a directory that allowed writing the file but prevented unlinking it. `Made::drop` supplied zero as the source length, so cleanup left one zero byte. The default comparison then skipped the file. Carry the header size into `Made` so an unremovable partial remains distinguishable from a complete copy." The remaining work is cr-ecm19-2 and cr-ecm19-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ba0191ee`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r19` over `ca377805..17f4eb97` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r19.codex.md`. It completes cr-ecm17-1 (`8b984864`).

## Evidence
"I cancelled `begin_record` after its file appeared but before the open result reached the writer. A zero-byte destination remained for `--ignore-existing` to skip. During commit, the writer also gives up its handle before awaiting the flush, so cancellation can fall back to cleanup by pathname. Arm cleanup inside the operation that creates the file and retain an identity-bearing handle through the final await."

cr-ecm17-1 made the writer before the one await it had added; the file was still made by an open on the blocking pool, and an open cancelled mid-way makes the file all the same and hands it to nobody. And `commit` took the file out of the writer before awaiting its flush. cr-ecm17-1's own guard was flaky for a related reason: the look it cancelled at could finish within the same poll (`combined-r19`'s gate and this work's both saw it fail once).

## Intake
Admitted as the reviewer put it. cr-ecm17-1's guard is replaced by one that holds the step that makes the file while the record is cancelled, so it cannot race.

## What
- A record's file is made, and which file it is noted, in ONE blocking step that hands it back as a `Made`: unclaimed — its record cancelled while it was being made — it removes itself (only while it is the file at its name). `begin_record` claims it into the writer with nothing awaited between. There is no await left between the file existing and its writer owning it.
- Every record notes which file it made (`own`), not only under `--ignore-existing`; its cleanup, with its handle or without, removes the name only while that file stands there, and tells a link to it from somebody else's file (`settle_failed_target_known`, cr-ecm17-2's three cases). Under `--ignore-existing` the same identity is what the delivery check asks for (`made`), as before.
- `commit` keeps the file inside the writer — guard armed, handle with it — through its flush, then takes the handle back without another await.

## Guard proof
- `sink::…::a_record_cancelled_while_its_file_is_made_leaves_nothing` (in place of cr-ecm17-1's), with and without `--ignore-existing`: a seam holds the step that makes the file once the file exists; the record is cancelled there (`select!` on the seam); the step is let go; nothing is left at the name. Four runs, green each time.
- `sink::…::a_target_settled_without_its_handle_is_removed_only_if_it_is_its_own` passes through the new path.
- Mutations, each restored byte for byte: a file made and never claimed left in place (the new guard red); a file known but not held settled as if nothing were known (red).
- The sink tests (101) and the whole suite in the scratch copy pass.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `Made`, `create_stream_destination`, `begin_record`, `FsRecordWriter::own`, `commit`, `settle_failed_target_known`, `settle_own_target`, a seam; guard.

## Known gaps
- Cancellation during `commit`'s flush has no deterministic guard (whether it waits at all is the blocking pool's timing); the file stays in the writer there by construction, and its cleanup is tied to its identity.
- Windows: nothing says which file is which, so a cleanup there is by name, as before (cr-ecm16-2's open half).
