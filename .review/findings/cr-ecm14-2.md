# cr-ecm14-2: `--ignore-existing` wrote through a link that stood at the path

**Severity**: HIGH — a run told to keep what exists writes a file somewhere it was never pointed at, through a link, and exits 0
**Status**: Reviewed, found incomplete — `combined-r16` (Codex openreview, 2026-11-21) reviewed `35adeacb..13faafd2` — utilities slices 5 and 6 and the eleven `combined-r15` fixes — with `guard_confirmed: true`. "The wire destination hard-codes the option to `false`. A file created after planning can therefore be truncated by a remote transfer. The local copy path acquires an exclusive handle, drops it, and reopens the pathname." And Windows attributes are prepared at a destination before it is this run's. The remaining work is cr-ecm15-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `e672e1c2`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. It completes cr-ecm13-1 (`fc3df295`).

## Evidence
"I retried a recorded leftover with a dangling symlink in its place. The retry followed the link, created its referent, and exited 0. Check the entry without following it and use a create operation that fails if the name is occupied."

Cause, in the two places the reviewer names:
- **The decision.** What stands at a destination path is found by the session's directory sweep, and for an entry the sweep cannot judge — a link — by a stat that LOOKS THROUGH it. A link to nothing is "nothing there". cr-ecm13-1's rule — nothing that stands at a path is written over — was asked of that answer.
- **The write.** Every way a file is written opened its destination by a plain creation, which writes through a link and makes what a link to nothing names. The rule was the decision's alone; nothing held it at the moment of writing.

## Intake
Admitted as the reviewer put it. It is not only the retry's: a plain `blit copy --ignore-existing` over a link to nothing wrote through it the same way (measured, 2026-11-21), and is fixed with it.

## What
- **The decision asks the entry itself.** A session that keeps what exists asks, of an entry the sweep could not judge, whether it is a symbolic link (`resolve_destination_target`, one more look at a link and none at anything else). A link standing at a path is kept, like a file, whatever it points to — a file, a directory, nothing. Where the run left its own incomplete copy it is reported too, as a file there is.
- **The write is a creation that fails if the name is taken** (`create_to_write`, `create_new`), in every way a file is written when the run keeps what exists: a stream from a peer (`create_stream_destination`), a copy on this machine with and without `--resume` (`copy_and_stamp_opened`, which takes the name first and owns what it made from there), and each member of a shard of small files (both writers). A name taken by anything — a file, a directory, a link to nothing — fails that file with `something stands at the destination that was not there when this run looked: --ignore-existing keeps it`; what is there is not touched, and nothing is settled as this run's.
- Without `--ignore-existing` nothing changes: a creation is a plain one, as it was.

## Guard proof
- `transfer_session::…::a_link_at_the_path_is_kept_like_a_file_whatever_it_points_to`: links to nothing, to a directory and to a file are each kept under the option — silently in a plain run, reported where the run left its own copy; a path with nothing there is still copied to; without the option the link is not asked about.
- `sink::…::a_run_that_keeps_what_exists_writes_through_no_link`: with and without `--resume`, a copy from this machine and a stream, each to a name taken by a link to nothing — the file fails in the words above, the link stays, what it names is not made, nothing of the run's is left; each to a free name — written. Without the option a file that is there is written over, as before.
- `sink::…::a_shard_member_is_created_or_fails_when_what_exists_is_kept`, `a_shard_written_by_a_run_that_keeps_what_exists_takes_no_taken_name`: the same for a shard's member, alone and through the sink, the member beside it landing.
- `crates/blit-cli/tests/job_retry.rs::an_ignore_existing_retry_follows_no_link_left_where_its_copy_was` (end to end): the reviewer's case — a recorded leftover and a failed path, each with a link to nothing in its place; the retry exits 2, names the leftover, leaves both links and makes neither thing they name; and a plain `copy --ignore-existing` over a link to nothing exits 0 with the link kept, nothing made, and the file beside it copied.
- `a_local_resume_that_keeps_existing_files_leaves_no_partial_of_its_own` (cr-ecm13-1) changed where it asked the sink directly to write a file that was there: the sink now refuses and leaves the file as it was, where it held it one byte longer.
- Mutations, each restored byte for byte — six, each red on the guards named for it: the link looked through; a link to nothing taken for an empty place; a stream created over whatever has its name; the local copy taking no name first; a shard's members created plainly; the creation that must be new made a plain one.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — the link asked about; the decision; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `THE_NAME_IS_TAKEN`, `create_to_write`, `creation_failed`; the three ways a file is written; guards.
- `crates/blit-cli/tests/job_retry.rs` — guard.
- `CHANGELOG.md`.

## Known gaps
- Between the creation and the copy that follows it on this machine, the name is this run's empty file, opened again by path: something that replaces THAT in the instant between is written through as before. The stream and the shard writers write through the handle the creation gave, and have no such instant.
- On macOS a copy on this machine under `--ignore-existing` no longer clones (`fclonefileat` refuses a name that exists, and the name is now made first): it copies. Not measured; nothing ran on macOS.
- A DIRECTORY standing where a file is to go is as before: not kept in silence, the file fails. Only a link is newly "something that exists".
- A run without `--ignore-existing` writes through a link at a destination path, as it always has. That is not this finding's.
- A file refused at the write for a name taken since the run looked is a failure of the run (exit 2), not a quiet skip: the run cannot say that what stands there is the file.
- Linux only.
