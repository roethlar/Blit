# cr-ecm12-3: A destination directory passed for a file already copied

**Severity**: HIGH — `copy` reports a file copied, exit 0, where a directory stands in its place and nothing was written
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm12-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r12` over `4d0f1a68..c5bab07a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r12.codex.md`. Older than the plans under review; found through cr-ecm11-1's and -2's guards.

## Evidence
"The new retry fixtures exposed an existing comparison bug on Btrfs. The source file and blocking directory both reported a size of 20 bytes, and their timestamps satisfied the skip rule. `copy` reported one file copied, zero bytes written, and exit 0. The saved-job test therefore had no failure to retry, and the automatic-retry test timed out waiting for a retry. Requiring a regular destination file before accepting the size/mtime match made both tests pass in the disposable copy."

Cause. `copy/compare.rs` decides whether a file needs copying from what stands at the destination: its size, its time, its hash. It asked for those of whatever is there. A directory has a size and a time too; on Btrfs a directory holding one entry is some twenty bytes, and a file of that size written just before it is "the same".

Reproduced here with the tests' temporary files on Btrfs: without the fix `a_retry_of_a_single_file_does_not_take_a_directory_of_its_name` and `a_single_file_replaced_by_a_directory_during_the_wait_is_not_retried_into` fail as the reviewer saw; this session's gates had run them on tmpfs, where a directory's size is another.

## Intake
Admitted as the reviewer put it.

## What
`comparable(dst)`: the destination is compared with only when it is a regular file (or a link to one). Anything else — a directory — needs copying, in every comparison mode, and the write then says why it cannot be (`Is a directory`), which is a failure the run reports.

## Guard proof
- `copy::compare::tests::a_directory_is_never_the_same_as_the_file`, on any filesystem: the source is made exactly the directory's size and time, and every mode, by path and by the opened source, says it needs copying; a regular file of that size and time is still the same.
- With temporary files on Btrfs, the two retry guards above: red without the fix, green with it.
- Mutation, restored byte for byte: whatever stands at the destination compared by its size and time, as before → the unit guard red on tmpfs and Btrfs, the two retry guards red on Btrfs.

## Files changed
- `crates/blit-core/src/copy/compare.rs` — `comparable`; the guard.

## Known gaps
- **This gate's tests run with their temporary files on tmpfs.** The whole suite was run once here with them on Btrfs: 1837 passed, and two build-identity tests failed only because that directory lay inside this checkout. A filesystem-dependent fault of this kind can be missed again.
- Run on Linux only.
