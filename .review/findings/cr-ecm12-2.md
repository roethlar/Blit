# cr-ecm12-2: A retry under `--ignore-existing` replaced whatever stood where its incomplete copy had been

**Severity**: HIGH — a retry overwrites a file someone put at the destination, under the option that promises to keep it, and exits 0
**Status**: Reviewed, found incomplete — `exclude-case-r13` (Codex openreview, 2026-11-21) reviewed `c99ad483..bd84649f`, the five `exclude-case-r12` fixes, with `guard_confirmed: true`. The mark does not bind the repair to the file: "I replaced a real partial with a different 20-byte file that preserved its modification time. `jobs retry` overwrote it with 128 KiB and exited 0. Every run uses the same timestamp, and the check happens before the sink opens the file for writing." A copy that fails again in a retry leaves a record with no incomplete copy in it, and the retry after that exits 0 with the destination at zero bytes; so does a retry of a path this fix reported as kept. And with `--size-only` a stamped copy of the source's own length is "copied" with no bytes written — the sink compares again after the session ordered the repair. The remaining work is cr-ecm13-1, -4 and -5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `bd84649f`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r12` over `4d0f1a68..c5bab07a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r12.codex.md`. Older than the plans under review (cr-jl4-1, cr-rework-1); found through cr-ecm11-2's guard.

## Evidence
"A real write failure during `copy --resume --ignore-existing` left an empty file and recorded the root in `left_in_place`. I atomically replaced that partial with a different file. `jobs retry` overwrote the replacement with 128 KiB and exited 0. Bind the repair to the destination file left by the failed write and verify its identity before overwriting it. Honor `--ignore-existing` when ownership cannot be established."

Cause. A path whose failed write left the run's own incomplete copy is retried apart from the rest, because `--ignore-existing` would keep that copy as "a file that is there". The pass did it by turning the option OFF for those paths — and with the option off the end that writes replaces whatever it finds. Nothing tied the pass to the file the failed write had left.

Reproduced here as the reviewer did it: no file allowed to grow past 64 KiB (`ulimit -f`, the signal ignored), a 128 KiB source, `copy --resume --ignore-existing` — exit 2, an empty `big.bin` left and recorded; another file renamed over it, or written into it; `blit jobs retry` — 128 KiB written over it, exit 0.

## Intake
Admitted as the reviewer put it.

## What
**The incomplete copy is marked when it is left, and the retry replaces only a file that still bears the mark.**

- Every failed write that leaves its target behind already marks it — one byte longer than the source, so that no comparison takes it for the file. It now also stamps it: modification time `UNFINISHED_STAMP_SECONDS`, 1985-06-11T20:11:14Z, a whole even second. The stamp is set after the length and whether or not the length could be set: the reviewer's empty file is one the write could not lengthen either.
- The pass of the run's own copies keeps `--ignore-existing` ON and says what its list is (`FilterSpec.files_from_left`, **wire contract 23**, request version 11). At such a path the end that writes replaces a file that bears the stamp — always, whatever a comparison would say of it — copies to a path where nothing stands, and KEEPS any other file, reporting the path as failed: `the file at the destination is not the incomplete copy this run left there, and --ignore-existing keeps it`. The run did not copy that path, and cannot say that what stands there is it.
- Anything written to the copy, or put in its place, has another time. So does a copy on a filesystem that could not hold the stamp: it is kept and reported, never replaced on a guess.

Why a mark and not a record of the file (device, file number, size, time) carried to the retry — which is what the reviewer's words suggest most literally: such a record has to travel from the failed write through the failure report, the summary's exact sets and the job's log, and back through every route's opening to the end that writes; it is a different thing on each system; and it goes stale as soon as a retry pass of the run itself writes to the copy and fails again. The mark is in the file: nothing travels but one bit saying what the list is, the check is one comparison of a time the comparison had already read, and the code is the same on every system. It is weaker in one respect, stated under Known gaps.

## Guard proof
- `job_retry.rs::an_ignore_existing_retry_keeps_a_file_put_in_place_of_its_incomplete_copy` (Unix), the reviewer's sequence with a real failed write: the copy left is stamped though it could not be lengthened; untouched, the retry completes it; with a file renamed over it, or written into it, the retry exits 2, says why, and the file is as it was put.
- `transfer_session::own_incomplete_copy_tests::a_retry_replaces_only_what_still_bears_the_mark_of_an_incomplete_copy`: a stamped copy one byte longer, and one at the very size a comparison would call copied — replaced; a path with nothing — copied; a file with another time, two seconds off included — kept and recorded; any other list — the option keeps every file and records nothing; without the option the list is not asked.
- `sink::unfinished_stamp_tests::an_incomplete_copy_left_in_place_is_stamped_as_one`; `operation_spec::…::a_request_for_the_runs_own_incomplete_copies_is_version_11_or_later`; `transfers::filter::…::a_retry_lists_the_source_itself_by_a_mark_of_its_own` (what the list is, here and at the peer).
- The guards of cr-jl4-1 and cr-ecm11-2 (`an_ignore_existing_retry_replaces_only_its_own_incomplete_copies`, `…_of_a_single_file_…`) made their "incomplete copy" by writing a file; they now stamp it as a failed write does, and pass. Six unit guards of the retry loop and one of `jobs retry` asserted the pass ran with the option off; they assert that it stays on and that the pass says what it is.
- Mutations, each restored byte for byte: the copy left without the stamp; whatever stands at a listed path taken for the run's own, as before; a file kept there not reported; the run's own copy compared like any file; the end that writes not asking what the list is; a pass, or a saved retry's part, not saying so; what the list is not reaching the filter, not leaving with it, not read at the peer; the request's version left at 10 — eleven, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `UNFINISHED_STAMP_SECONDS`, `NOT_THIS_RUNS_COPY`, `mark_unfinished`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `destination_needs`, the session's record of what it kept, `CONTRACT_VERSION = 23`; guard.
- `crates/blit-core/proto/blit.proto`, `crates/blit-core/src/fs_enum.rs`, `crates/blit-core/src/transfers/filter.rs`, `crates/blit-core/src/remote/transfer/operation_spec.rs` — the list says what it is.
- `crates/blit-cli/src/{cli,main}.rs`, `crates/blit-cli/src/transfers/{retry,mod,remote_remote_direct}.rs`, `crates/blit-cli/src/{check,jobs}.rs` — the pass says so; the option is not turned off.
- `crates/blit-cli/tests/job_retry.rs`, `crates/blit-daemon/src/service/delegated_pull.rs` (a test's request); `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- **The mark is the same for every run.** Another run's incomplete copy left at the same path is taken for this run's and replaced — a file that run itself marked as not the file. And a file someone deliberately gives that exact time is taken for one. A mark made from the run's ID would close the first; it needs the ID at every place a failed write is settled, and was not built.
- **A leftover of every failed write now reads 1985-06-11**, with or without `--ignore-existing`. Its time said nothing before either (the time of the failure).
- **A path kept this way stays failed** on every later retry of the job: the run has no way to learn that the file there is one the user is content with.
- A leftover a run could neither remove nor stamp is kept and reported by the retry where it used to be replaced.
- **Nothing ran on Windows or macOS**; the stamp is set and read through the same calls there. The guard with a real failed write is Unix-only and exercises this machine as the end that writes; a daemon as that end runs the same check and has no guard of its own.
- Run on Linux only.
