# cr-ecm13-1: A retry under `--ignore-existing` still wrote over a file put where its incomplete copy had been

**Severity**: HIGH — a retry overwrites a file someone put at the destination, under the option that promises to keep it, and exits 0
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm13-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r13` over `c99ad483..bd84649f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r13.codex.md`. It completes cr-ecm12-2, by withdrawing what that fix tried to make safe.

## Evidence
"I replaced a real partial with a different 20-byte file that preserved its modification time. `jobs retry` overwrote it with 128 KiB and exited 0. Every run uses the same timestamp, and the check happens before the sink opens the file for writing." — "Verify ownership on the destination handle before writing."

Cause. cr-ecm12-2 knew a run's own incomplete copy by a modification time stamped on it. A time is carried to another file by anything that keeps times, it is the same for every run, and it is read when the session compares, not when the sink writes.

## Intake
Admitted. **Not fixed as the reviewer put it**: the repair is not bound more tightly to the file; it is taken away.

Three rounds have each found a way for "replace the run's own incomplete copy, under an option that keeps every file" to replace something else (cr-ecm11-2, cr-ecm12-2, this one), and this round's cr-ecm13-4 and -5 are two more faults of the same repair. Binding it to the file for certain needs the file's identity recorded where the write failed, carried through every report, record and log to every later retry, and checked on the handle each sink opens — on each system in its own way. What the reference tool does was measured instead (`rsync` 3.5.2, kept in the session's scratchpad): by default a transfer cut short leaves NOTHING at the destination; with `--partial` it leaves the partial under the file's name, and the same command with `--ignore-existing` then skips it and exits 0. It never writes over a file under that option, its own partial included.

## What
**Under `--ignore-existing` nothing that is at the destination is written over, and a run leaves nothing of its own there to be told from a file.**

- A write that fails removes the file it made — with `--resume` too (`FsSinkConfig.ignore_existing`; a streamed record and a shard member already did). D-2026-09-29-2 says as much of an aborted record: "removes the partial target so nothing that looks finished is left behind … the retry pass re-lands the file". A retry then finds nothing there and copies; and a file someone puts there meanwhile is kept, like any file.
- Where the copy could NOT be removed, it stays and is recorded as left, as before. A retry sends such a path apart, the option still on (cr-ecm12-2's `files_from_left`): it is copied to only if nothing stands there; a file that does is kept, and — unlike any other file under the option — reported, with the path failed. Nothing tells the copy from a file put in its place, and nothing tries to.
- No time is stamped on anything, and none is read: an incomplete copy is no longer dated 1985.
- `FilterSpec.files_from_left` means "never written over" where it meant "written over if marked": **wire contract 25**, request version 12.

## Guard proof
- `job_retry.rs::an_ignore_existing_run_leaves_no_partial_for_a_retry_to_write_over` (Unix), with a real failed write (`ulimit -f`, the reviewer's): the failed `copy --resume --ignore-existing` leaves nothing; with nothing there the retry copies the file; with a 20-byte file there — and with the same file given the time cr-ecm12-2 stamped — the retry keeps it.
- `…::an_ignore_existing_retry_writes_over_nothing_that_is_there`, `…_of_a_single_file_knows_where_its_copy_was_left` (the guards of cr-jl4-1 and cr-ecm11-2, which asserted the repair): a file standing where a copy was left is kept and the retry exits 2 saying so; once it is removed the retry copies; a file where the file merely failed is kept and that is all.
- `transfer_session::own_incomplete_copy_tests::nothing_is_written_over_where_an_incomplete_copy_was_left`; `sink::…::a_local_resume_that_keeps_existing_files_leaves_no_partial_of_its_own`; `operation_spec::…::a_request_for_the_runs_own_incomplete_copies_is_version_12_or_later`.
- Mutations, each restored byte for byte: a resume copy keeping the partial it made; the sink not told of the option; what stands where a copy was left written over, or kept and nothing said, or kept and not a failure of the session; the request's version left at 11 — six, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the stamp removed; `FsSinkConfig.ignore_existing`; `A_FILE_WHERE_A_COPY_WAS_LEFT`; guard. Thirty test configurations gain the field.
- `crates/blit-core/src/transfer_session/{mod,local}.rs` — `destination_needs`; `CONTRACT_VERSION = 25`; guard.
- `crates/blit-core/proto/blit.proto`, `crates/blit-core/src/fs_enum.rs`, `crates/blit-core/src/remote/transfer/{operation_spec,pipeline}.rs`, `crates/blit-cli/src/{cli.rs,transfers/retry.rs}` — what the list means, in words; request version 12.
- `crates/blit-cli/tests/job_retry.rs`; `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- **With `--resume --ignore-existing`, a file that fails part-way starts again from the beginning on a retry.** Its partial was kept and resumed; it is now removed. Under that option a kept partial was of use to a retry only — any later run skipped it as a file that is there.
- **An incomplete copy that could not be removed is never repaired by a retry.** It is kept and reported until someone removes it. It used to be replaced.
- **For the owner**: this is a rule taken without being asked — the recorded decisions and the measured reference both point to it, and three review rounds on the alternative did not converge. The alternative, a repair bound to the file by its recorded identity, is described under Intake.
- Between the session finding nothing at a path and the sink creating the file, a file put there is written over — under this option as under none; that window is every copy's, and is not closed here.
- Nothing ran on Windows or macOS. Run on Linux only.
