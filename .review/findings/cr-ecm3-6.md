# cr-ecm3-6: A machine ID read while another process was still writing it

**Severity**: MEDIUM — a test flake on a fresh settings folder, and behind it a race in the product: of two commands first using one settings folder at the same moment, one ran unlogged and could not save or export its job
**Status**: Verified — `exclude-case-r4` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..0669279f`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding (it found the wait's timeout accepting an unfinished ID — cr-ecm4-5)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `5ff0c967`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r3` over `3bc3ee47..467773b4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r3.codex.md`.

## Evidence
The reviewer reproduced a test failure twice with a fresh configuration: a new job-replay test "read the shared machine-ID file before another process finished writing it", and asked that each prompt test have its own configuration directory.

Reproduced here before any change (2026-11-20, Linux): the four test binaries that drive `blit` run twelve times, the shared test settings folder removed before each run — 3 of 12 runs failed, each in `case_note::a_job_run_again_shows_the_line_as_its_first_run_did`, with `blit: warning: this run is not logged: …/machine-id does not hold a machine ID` and `this command's job cannot be exported: this machine's folder or ID cannot be had`.

Cause, in the product (`crates/blit-core/src/job_log.rs` `machine_id`, older than this plan): the first user of a settings folder creates `machine-id` and then writes the ID into it. A second process arriving between those two steps finds the file, reads it empty, and refuses it as a file that "does not hold a machine ID". The tests added by slices 2 and 3b were the first to export jobs from the settings folder every test shares, so they were the first to meet it.

## Predicted observable failure
Two `blit` commands started together on a machine that has never run one (a script, a scheduler): one of them is not logged, and with `--save` or `--export` it fails.

## Intake
Admitted, for the tests as the reviewer asked and for the product, where the race is.

## What
- **Product:** a machine-ID file that is empty, or whose one line is not finished, is read again shortly (up to 100 times, 10 ms apart) until the ID is there. The line is finished when it ends or holds the 32 characters `machine_id` writes; what a finished line holds is judged at once, so a file of anything else is refused as before, without waiting.
- **Tests:** `case_note.rs` and `pattern_files.rs` give each test its own settings folder.

## Guard proof
- `job_log::tests::a_machine_id_still_being_written_is_waited_for`: a reader that finds the file empty, or holding half an ID, while a writer finishes it 50 ms later gets the whole ID; a finished line that is no ID is refused at once; a whole ID with no line ending is read at once; a file left empty for good is refused after the wait.
- `job_log::tests::first_users_of_one_folder_at_once_all_read_the_same_machine_id`: eight first users of a fresh folder, twenty times over, all read one ID.
- Mutations, each restored byte for byte: a file still being written judged at once → the first guard red, and the second too (the race shown inside one process); an unfinished line taken for the ID → the first guard red.
- The flake, tried again after the fix: the same twelve runs from a removed shared settings folder, five test binaries — no failure.

## Files changed
- `crates/blit-core/src/job_log.rs` — `machine_id` waits for an unfinished file; two guards.
- `crates/blit-cli/tests/case_note.rs`, `crates/blit-cli/tests/pattern_files.rs` — a settings folder per test.

## Known gaps
- A process that created the file and died before writing it leaves it empty; every later command then waits one second and runs unlogged, saying so, as before but for the wait. Nothing repairs the file.
- A hand-made `machine-id` holding fewer than 32 characters and no line ending is read after the one-second wait.
- Other test files still share one settings folder; none of them exports or saves a job from it. Run on Linux only.
