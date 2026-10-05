# cr-win-1: Copy/mirror push retries can falsely clear a named-stream failure

**Severity**: HIGH — a retry pass can skip a file whose first-pass failure left the main bytes intact (e.g. a rejected NTFS stream), clear the failure, and exit 0 with an incomplete backup
**Status**: Fixed (reworked) — red/green on macOS; the rework's Windows run and the owner-approved codex review are pending
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0528e78c` + `4a06aec5` (rework; the first fix `501c408d` is superseded by the second)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 5eeff4ac..8a1f04cb (Windows fixes win-1..5), record .review/results/ssc-win-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/mod.rs:417 — retry passes now invoke the deferred push with move_verb=false, selecting the ordinary SizeMtime/SizeOnly comparison at crates/blit-cli/src/transfers/remote.rs:493; that comparison explicitly ignores named streams at crates/blit-core/src/transfer_session/mod.rs:5929.

Context: before win-4 (`44442e54`) every push — main pass and retries — compared with the move rule (IgnoreTimes); win-4 restored the copy rule for the main pass and, with it, for the retry passes too.

## Predicted observable failure
On a Windows push using --size-only, if the destination writes the main file but rejects an NTFS alternate data stream, the first pass records a per-file failure. The retry sees the matching main-file size, skips the file without checking streams, replaces the failure set with an empty result, and exits 0 although the stream is missing—silently producing an incomplete backup.

## Reviewer's suggested approach
Use the ordinary copy comparison only for the main pass. Retry-only entries should be transferred unconditionally or verified with an exhaustive comparison that includes named streams; add a Windows guard where an ADS-tail failure remains reported until the stream is successfully applied.

## What
**First fix, superseded** (`501c408d`): retry passes forced `ignore_times`, re-sending their whole set. It cleared the false success but re-copied current files, overwrote a newer destination that the user's compare keeps, left an `--ignore-existing` hole, and did nothing for a plain re-run. Owner, 2026-10-04: "why would I accept this? fix it."

**Root cause.** A file that failed after its bytes reached the destination was left there at the source's size with a write-time mtime. Every compare took it for finished, on a retry pass and on any later run, contrary to D-2026-09-29-2 ("nothing that looks finished is left behind"). The default SizeMtime compare skips a same-size newer target, and the ls-6 default compare never looks at streams, so `--size-only` was not the only exposure.

**Rework 1/2** (`0528e78c`, sink): a failed write never leaves a finished-looking target.
- Streamed records (push, pull, remote-to-remote): a failed flush or metadata tail removes the target, as an abort does.
- Tar-shard members (both writers): removed after a failure, but only once the target was created; an earlier failure leaves the previous version untouched.
- Local copy: the cr-ssc4-2 guard settles explicitly.
- Resume (local and both remote lanes): the partial is held one byte longer than the source from the first patched block until the completion truncates it. This uses `resume_copy_from` and the new `TransferSink::hold_resume_partial`, called by the control lane and `NeedListSink`. An interrupted in-place patch of a same-size file therefore never keeps the source's size, and a completion whose tail fails keeps the partial marked. Nothing is touched before a block arrives; a grant-time hold was tried and rejected because it appended a byte to a previous version the source never replaced.
- A target that cannot be removed (another process holds it without delete sharing) is left one byte longer through the write's own handle, with the removal retried after that handle closes. Its reason then carries `INCOMPLETE_LEFT_IN_PLACE`, as does a kept resume partial of a file that did not exist before.

**Rework 2/2** (`4a06aec5`, CLI): retry passes compare exactly as the main pass does, with the forced re-send removed. Under `--ignore-existing`, paths whose reason carries `INCOMPLETE_LEFT_IN_PLACE` retry in their own session with it off, because that copy is this run's own and not one the user asked to keep.

## Guard proof
Rework 1/2 (portable, via the test-only `METADATA_TAIL_FAULT_PREFIXES` hook), in `sink::cr_win_1_tests`:
- `a_streamed_record_whose_metadata_tail_fails_leaves_no_target`
- `a_shard_member_whose_metadata_tail_fails_leaves_no_target`
- `a_local_copy_whose_metadata_tail_fails_leaves_no_target`
- `a_local_resume_whose_metadata_tail_fails_keeps_an_unfinished_partial` (a `created` file's reason carries the marker)
- `a_resume_completion_whose_metadata_tail_fails_keeps_an_unfinished_partial`
- `a_resume_partial_is_held_off_the_source_size_until_it_completes`
- `a_target_that_cannot_be_removed_is_left_one_byte_longer` (unix, read-only directory; skips for root)

Also `resume::an_interrupted_resume_never_leaves_the_source_size`, and `source_side_containment` `assert_resume_fault_contained` now asserts the faulted same-size partial is `len + 1` on both carriers.

Each mechanism reverted alone turned its test RED: streamed settle, shard discard, local Remove arm, local KeepUnfinished arm, completion settle, the hold, the `resume_copy_from` start mark, and the fallback mark. Dropping the control-lane hold call reddened only the in-stream test; dropping the `NeedListSink` forward reddened only the data-plane test. Restored byte-identical → green.

Rework 2/2:
- `retry_pass::a_retry_pass_compares_like_the_main_pass` (local + push; replaces the forced re-send's lookalike test). During the wait one blocked file gets a current copy and another a newer same-size file. Result: exit 0, `files_transferred` 1, the newer file kept.
- Unit tests `a_retry_pass_keeps_the_users_compare` and `ignore_existing_retries_this_runs_own_leftovers_without_it`.
- Mutations: putting the forced re-send back reddened both compare tests; dropping the leftover override reddened the split test.

Windows (compile-checked by cross-clippy only; the run is pending):
- The sink's real-NTFS refused-tail test (a 300-character stream name) now asserts the target is removed.
- `windows_a_rejected_named_stream_is_re_sent_on_the_retry_pass` asserts the failed copy is not left at the source's size, and gains a plain re-run half: `--retries 0` exits 2, then an ordinary re-run lands the stream.

Gate (macOS): `0528e78c` 1353/0/2; `4a06aec5` 1355/0/2; fmt and clippy `-D warnings` clean native, linux-cross, and windows-msvc-cross (`blake3/pure`).

First fix's proof, for the record: `501c408d` was red/green on macOS and the ARM64 VM (VM full suite 1318/1/2 at `42779ff6`), and CI Test (Windows, x86_64) was green at `54bfd2d9`.

## Known gaps
- Crash consistency (a killed process, power loss) between a streamed record's last byte and its tail, or between a resume completion's truncate and its stamp, can still leave a complete-content target without its metadata. That is in-place writing with no staging (D-2026-09-29-2), and the windows are milliseconds.
- `--ignore-existing` leftovers are recognised from the named failure report, so leftovers beyond the report cap retry under the user's flag.
- Under `--ignore-existing`, a plain re-run (not a retry pass) still skips this run's own leftover, because a later run cannot know whose copy it is. The run's report names it.
- The local copy guard still removes a destination whose copy failed at open, before any byte was written. This pre-existing cr-ssc4-2 behavior was not changed.
- No Windows run of the rework yet. The VM was not running on 2026-10-05. The x86_64 evidence will come from CI.
