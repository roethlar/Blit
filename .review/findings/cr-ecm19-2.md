# cr-ecm19-2: a streamed write was reported delivered without asking whose file stood at its name

**Severity**: HIGH — a move can remove a source whose copy was never delivered: the destination holds a file somebody put there while the record arrived
**Status**: Verified — `combined-r21` (Codex openreview, 2026-11-22) reviewed `c15ebf5a..b92c9a17` — the seven `combined-r20` fixes — with `guard_confirmed: true`: "Ordinary Unix streams and shard members verify the destination identity at delivery. I would keep all three changes." Its known gaps — a local copy's own write, and Windows — are cr-ecm20-4's
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `3f7d60b4`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r20` over `17f4eb97..c15ebf5a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r20.codex.md`. It completes cr-ecm18-1 (`ba0191ee`).

## Evidence
"I replaced the destination during a stream. Commit reported one file written and zero failures, although the destination contained the replacement. The writer records identity for every stream but uses it for delivery only under `--ignore-existing`. A move can consequently authorize removal of an undelivered source. Use the recorded identity at commit, accounting for supported writes through links."

## Intake
Admitted as the reviewer put it, and carried to the other writes a session receives: a shard member's — many small files in one payload — was asked the same only under `--ignore-existing`.

## What
- `still_its_own(dst, own, new_only)` — the one delivery check: what the write made (`own`) must still be what the name holds. A run that keeps what exists asks for the name itself (as before, `THE_NAME_IS_TAKEN`); any other run, whose open follows a link standing at the name, also takes a link that leads to that file; anything else is `REPLACED_WHILE_WRITTEN`, and the file there is left as it is.
- A streamed record asks it at commit in every run, with the identity it noted when it made its file (`made`, the `--ignore-existing`-only copy of it, is gone).
- A shard member — both writers — notes which file it made in every run and asks the same.
- A move strikes every path that did not land from its account, so a record that says so keeps its source.

## Guard proof
- `sink::…::an_ordinary_record_counts_as_delivered_only_where_its_file_stands`: an ordinary run's record with another file renamed over its name mid-stream — it says it did not land and the other file is whole; a record written through a link at its name — it lands, the link stays, the file it names holds the bytes.
- `sink::…::an_ordinary_shard_member_lands_only_where_its_file_stands`: another file renamed over a member's name once it is made (the claim seam, now run in every run) — the member fails with `REPLACED_WHILE_WRITTEN` and the other file is whole.
- Mutations, each restored byte for byte: only a run keeping what exists asks (both red); a write through a link not counted (the record's red); the stream asking as if it kept what exists (red); a member noting its file only when it keeps what exists (red).
- The `--ignore-existing` guards of cr-ecm16-2 and cr-ecm14-2 pass unchanged. The sink tests (104) and the whole suite in the scratch copy (1,961) pass.
- Speed, a push of 20,000 small files to a loopback daemon on Btrfs: ordinary, median 387.2 → 394.6 ms (best 380.4 → 381.5); `--ignore-existing`, 399.8 → 400.5. One `lstat` more per file at commit; identical builds have moved about 4 ms run to run.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `still_its_own`, `REPLACED_WHILE_WRITTEN`, the record writer, both shard writers; guards; `claimed_identity` removed.

## Known gaps
- The probed shard writer (a diagnostic) has no guard of its own; it asks through the same `still_its_own`.
- A local copy of a run that keeps nothing (the File payload: clone, `copy_file_range`, `CopyFileEx`) opens its destination inside the copy engine, and the sink does not know which file it wrote; it is not asked. Not raised.
- Windows: no file identity; nothing is asked there, as before.
