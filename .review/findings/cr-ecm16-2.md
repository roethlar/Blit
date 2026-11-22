# cr-ecm16-2: `--ignore-existing` was not held to the END of every write

**Severity**: HIGH — a file moved over a name while a run that keeps what exists writes there is counted delivered, given the run's mode, or removed
**Status**: Fixed on Unix, awaiting review; the Windows half is open and blocked (below)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm16-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It completes cr-ecm15-1 (`b2a9e435`).

## Evidence
"If another file takes the destination name after exclusive creation, a stream can report delivery after writing to the displaced file. A shard can stamp the replacement's permissions, and failure cleanup can unlink it. Windows also returns no identity for the local ownership check. Retain the acquired handle, verify what stands at the name before counting delivery, and leave an uncertain replacement alone."

cr-ecm15-1 held the copy on this machine to the file it made; the two writers of the wire were left as they were. A record (a file streamed in) is written through one handle but was counted delivered whatever stood at its name by then. A shard member was written through its handle, the handle was closed, and its time and mode were then set BY THE NAME — onto whatever stood there — and a member whose tail failed was removed by the name. cr-ecm15-1's own known gaps said so.

## Intake
Admitted as the reviewer put it. Also in this commit, as part of the same cleanup: cr-ecm15-1's follow-up, written while `combined-r17` ran and never landed — a failed write that still holds its handle removes the destination only while the file it holds stands there (`settle_failed_target`), for every run. Making it, a flaky test showed one more way it could miss: a record given up with a write still in flight cannot have its handle back, so `abort` now lets the write finish first.

## What
- Under `--ignore-existing` every writer notes WHICH FILE it made (`claimed_identity`: device and number) — a record when it begins, a shard member when it is created.
- A record counts as delivered, and a shard member lands, only while that file still stands at its name (`still_its_own`); otherwise the write fails with `THE_NAME_IS_TAKEN` and the file there is left whole.
- A shard member's time and mode go through the handle it was written through (Unix), for every run — never by the name.
- A failed write settles its target only while the file it made stands there (`settle_own_target`): by the noted identity, or by the handle it still holds.
- `FsRecordWriter::abort` flushes before it settles.
- Nothing on the wire changes.

## Guard proof
- `sink::…::a_shard_member_lands_only_where_its_own_file_stands` (through `write_shard_member`, a seam after the member's name is made): another file moved over the name — whole afterwards, its own mode kept, the member says the name is taken; the same with a tail that fails — not removed. A free name is written with its time and mode; a run that keeps nothing writes as ever.
- `sink::…::a_record_counts_as_delivered_only_where_its_own_file_stands`: half a record, a file moved over its name, the other half, commit — the record says the name is taken and the file there is whole.
- `sink::…::a_record_given_up_removes_only_the_file_it_made` (the follow-up), with and without the option.
- `sink::…::a_target_settled_without_its_handle_is_removed_only_if_it_is_its_own`: the case the noted identity is for — no handle to look through.
- Mutations, each restored byte for byte: a record counted delivered whatever stands at its name; a member the same; a member's mode set by its name; a failed write removing whatever stands at the name; no file noted; the handle check of a given-up record removed — each red on its guard.
- Run four times over the whole library after the flush: no flake.
- The whole suite in the scratch copy: 1,941 passed, 0 failed. Speed, a copy of 20,000 files of 512 bytes to a loopback daemon on Btrfs, median of five: 394.4 → 391.5 ms without the option, 403.1 → 395.5 ms with it.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `claimed_identity`, `still_its_own`, `settle_own_target`, `keep_for_the_tail`, `settle_failed_target`, `FsRecordWriter`, both shard writers, `stamp_shard_member_metadata`; guards.

## Known gaps
- **Windows**: the system does not say there which file is which in what the standard library gives, so nothing is noted, a replacement is not noticed, and a failed write removes what stands at the name; a shard member's handle is closed before its tail, as before. What it needs runs only on Windows (as cr-uv6-3), and no Windows machine has been reachable.
- A record dropped outright (the session cancelled) with a write in flight, in a run that keeps nothing, cannot look through its handle and removes what stands at its name, as before.
- Between the last look at the name and the count, another hand can still move a file over it; the window is one system call wide.
- The small-file probe's shard writer (a diagnostic) is changed the same way and has no guard of its own.
- Linux only.
