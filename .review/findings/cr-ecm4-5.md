# cr-ecm4-5: A machine ID still unfinished when the wait ran out was taken as it was

**Severity**: MEDIUM — a job saved under half an ID was refused by the same machine once the ID was whole
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm4-5: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r4` over `3bc3ee47..0669279f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r4.codex.md`.

## Evidence
When only 16 of its 32 digits were present past the read timeout, blit saved a job under the prefix. Once the ID was complete, replay refused the job as belonging to another machine. The reviewer asked for the complete ID to be published in one operation, or an unfinished read to fail at the timeout.

Cause: cr-ecm3-6 made `machine_id` (`crates/blit-core/src/job_log.rs`) wait for a file that another process had created and not yet written. When the wait ran out it judged whatever the file held, and half an ID passes `check_id` (which accepts shorter IDs). The file was still created under its own name and written afterwards, so the moment existed at all. cr-ecm3-6 recorded the second half of this as a known gap.

## Predicted observable failure
A writer stalled between creating `machine-id` and finishing it for more than a second (a suspended process, a slow disk): another command takes the part written so far for the machine's ID, logs and saves under it, and those jobs are another machine's from then on.

## Intake
Admitted. Both of the reviewer's alternatives are done.

## What
- **Published whole.** The ID is written to a draft beside the file (`machine-id.<pid>.<id>.tmp`), synced, and given the name `machine-id` with a hard link: one operation, which fails if the name is taken and never replaces it. The draft is then removed. From the first moment the name exists, it holds the whole line. Another process that got there first is left standing and read.
- **Never taken unfinished.** A line still unfinished when the wait runs out is refused — "holds a machine ID that was never finished being written; delete the file and blit makes a new one" — and the command runs unlogged, saying so, as for any unusable machine-ID file.
- Where the draft cannot be made or linked (a filesystem with no hard links), the ID is written in place as before; readers wait, and now take nothing unfinished.

## Guard proof
- `job_log::tests::the_machine_id_file_never_exists_without_the_whole_id`: at the moment the file being written exists and is still empty (a test seam inside `write_new`), the folder holds only the draft and no `machine-id`; afterwards `machine-id` holds the whole line and the draft is gone; a name already taken by another ID is left untouched and no draft remains.
- `job_log::tests::a_machine_id_still_being_written_is_waited_for`: a file holding half an ID for good is refused after the full wait, in those words (it used to be returned as the ID).
- `first_users_of_one_folder_at_once_all_read_the_same_machine_id` (cr-ecm3-6) unchanged and green.
- Mutations, each restored byte for byte: the ID written under its own name again → the first guard red; the name given by a rename, which replaces → red, and the eight-first-users guard too; an unfinished line taken as the ID at the timeout → the second guard red.
- The cr-ecm3-6 flake check again: twelve runs of the five test binaries that drive `blit` — no failure.

## Files changed
- `crates/blit-core/src/job_log.rs` — `publish_machine_id`, `write_new`, the timeout; guards.

## Known gaps
- A `machine-id` left empty or unfinished by a build before this fix (its writer died) is refused after a one-second wait on every command, which then runs unlogged and says so. Nothing repairs the file; the message says to delete it.
- A process that dies between writing its draft and linking it leaves the draft file behind. Nothing removes it.
- On a filesystem with no hard links the in-place write remains, with its moment; no such filesystem was tried. Nor was the hard link on Windows or macOS: run on Linux only (it compiles for Windows).
