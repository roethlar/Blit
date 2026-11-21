# cr-ecm11-1: A retry took a directory of the file's name for the file that failed

**Severity**: HIGH — a retry writes into the destination a file that was never the one being retried, and exits 0; after a `move`, that success allows the removal of the directory
**Status**: Reviewed, found incomplete — `exclude-case-r12` (Codex openreview, 2026-11-21) reviewed `4d0f1a68..c5bab07a`, the six `exclude-case-r11` fixes, with `guard_confirmed: true`. "Contract 20's explicit root marker … [is] sound", and it was the reviewer's own red/green proof. The converse case is open: a retry's list names a CHILD by its name, and a source that has since become one file answers to that name as a `--files-from` list lets it (cr-ecm9-3) — a failed child `Report.txt` of a directory `Report.txt`, the directory replaced by a file, and `jobs retry` overwrote an unrelated `out/Report.txt`, exit 0. And this fix's two end-to-end guards do not fail their first copy on Btrfs, where the blocking directory's size and time pass for the file's (an older comparison fault). The remaining work is cr-ecm12-1 and cr-ecm12-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `513a7835`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r11` over `d3e33acc..bfa537f9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r11.codex.md`. It completes cr-ecm10-3.

## Evidence
"After a failed `Report.txt` became a directory containing another `Report.txt`, `jobs retry` overwrote `out/Report.txt/Report.txt` and exited 0. The base retry selector left that destination intact and exited 2. Resolve the root marker only when the source enumerator sees a root file. This also matters to `move`, whose successful retry can authorize recursive removal of the replacement directory."

Cause. A source that is one file has one entry, with no path below its root: its failure is recorded under the EMPTY path. A list of names cannot hold that (cr-ecm7-1), so its retry named nothing — cr-ecm10-3's finding — and cr-ecm10-3 put the file's NAME in its place before the list was made. A name is answered by any entry of that name. With the source replaced by a directory, the walk has a child called `Report.txt`, the list names it, and it is sent — to `out/Report.txt/Report.txt`, the destination root being `out/Report.txt`.

## Predicted observable failure
A single-file copy fails; before `blit jobs retry` (or during `--retry-wait`) the file is replaced by a directory of the same name — an unpacked archive, a build output: the retry copies part of that directory into the destination and reports success. For a `move`, the source directory may then be removed.

## Intake
Admitted as the reviewer put it.

## What
- **A retry asks for what failed as it failed.** The translation is gone (`named_for_a_list`, `source_name`): the retry passes of a command and `jobs retry` hand on the paths recorded, the empty one among them.
- **The empty path in a list is the mark for the source itself.** It is no line of a list read from a file (a blank line is skipped there) and no line that names nothing: `split_listed` keeps it with what is asked for.
- **Only a source that IS a file answers to it** (`FileFilter::allows_root_file`, used by the walk at its root and by the scan's filter for the entry with no path): asked under its own name, as before, or passed by the mark. A directory has no entry without a path, so nothing in it answers — and the source reports the mark as listed and not found, under the same empty path (`source: missing at retry`), exit 2.
- **On the wire the mark has a field of its own**, `FilterSpec.files_from_root`, **wire contract 20**: among the names it would be an empty one, which a receiver refuses. A peer of contract 19 would not read it.

## Guard proof
- `job_retry.rs::a_retry_of_a_single_file_does_not_take_a_directory_of_its_name`, from this machine and from a daemon: the reviewer's sequence — the copy fails, the source becomes a directory holding another `Report.txt`, `jobs retry`: exit 2, `missing at retry`, the file standing at `out/Report.txt/Report.txt` untouched; the source put back as a file and its place freed, the next retry lands it.
- `retry_pass.rs::a_single_file_replaced_by_a_directory_during_the_wait_is_not_retried_into`: the same within one command's `--retries`.
- `retry_pass.rs::a_single_file_freed_during_the_wait_lands_on_the_retry_pass` (cr-ecm10-3's guard) passes unchanged: the file itself is still retried.
- Unit: `transfers::filter::…::a_retry_lists_the_source_itself_by_a_mark_of_its_own` (in the list here, a field on the wire, in the list at the peer; a list of names never holds it); `enumeration::…::the_source_itself_is_not_an_entry_of_a_directory`, `a_root_that_is_one_file_is_filtered_by_its_own_name`; `transfer_session::…::the_source_itself_is_missing_unless_it_was_sent`.
- Mutations, each restored byte for byte: a saved job's retry, and a retry pass, asking by the file's name as before → the two end-to-end guards; the walk, and the scan's filter, not knowing the mark → the root-file guards and the retries that must land; the mark read as a line that names nothing; the mark not leaving with the list; the mark not read at the peer — seven, each red on the guards named for it.

## Files changed
- `crates/blit-core/proto/blit.proto` — `FilterSpec.files_from_root`.
- `crates/blit-core/src/fs_enum.rs` — `allows_root_file`, `lists_the_source_itself`.
- `crates/blit-core/src/enumeration.rs`, `crates/blit-core/src/remote/transfer/source.rs` — the root entry asks it.
- `crates/blit-core/src/transfers/filter.rs`, `crates/blit-core/src/remote/transfer/operation_spec.rs` — the mark in a list, to the wire and back.
- `crates/blit-core/src/transfer_session/mod.rs` — `CONTRACT_VERSION = 20`; guard.
- `crates/blit-cli/src/transfers/retry.rs`, `crates/blit-cli/src/jobs.rs` — the paths as recorded.
- `crates/blit-cli/tests/job_retry.rs`, `crates/blit-cli/tests/retry_pass.rs` — guards.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- A source that was a DIRECTORY and is now a file of the same name: its failed entries are names, none of which the file answers to, and each is reported missing — as before.
- The file replaced by ANOTHER file of the same name is retried as the source itself: it is the source typed.
- Run on Linux only.
