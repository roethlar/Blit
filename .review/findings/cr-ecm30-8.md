# cr-ecm30-8: the resume's fresh partial was staged and swapped in, against the owner's no-staging ruling

**Severity**: HIGH — a governance fault with behaviour behind it: since cr-ecm21-3 (2026-11-22) a `--resume` that may not reuse a destination staged a fresh partial beside it — later in a private directory of its own — and renamed or swapped it into place. The owner's ruling D-2026-09-29-2 ("no staging files. that was decided last year") forbids that, and the agent had not weighed it in nine rounds. The staging's own faults followed (cr-ecm30-2, -3, -4), and the exchange it needed made macOS and Linux's NFS and SMB clients fail such files.
**Status**: Reviewed, found incomplete — `combined-r32` (Codex openreview, 2026-11-23) reviewed `93ecf7b2..714ed0b7` — the `combined-r31` round, the in-place rework of D-2026-11-23-1 among it — with `guard_confirmed: true`: "The Unix implementation removes the staging directory and the dependency on atomic exchange. Creating the partial directly also preserves the destination directory's group inheritance. I would keep that design and the shared nonblocking checksum opener." "The first failed metadata read after creation leaves a zero-byte file before its unfinished length is set. Later cleanup also discards lookup and unlink errors. In a focused check, loss of directory write permission left a 19-byte partial, and the returned error mentioned only preparation. Attempt the unfinished mark through the held handle and report any retained file and cleanup error." "The release note should state the two limits the owner accepted. An arrival between the final check and unlink can be removed, and an automatic retry can overwrite a file preserved by the preceding pass. The fresh-file test should also hold the old file open while comparing inode numbers. A disposable ext4 image reused the same number immediately after removal." The remaining work is cr-ecm31-3 and -6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm30-8`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` over `fd603571..93ecf7b2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`. Owner ruling D-2026-11-23-1 (2026-11-23, "Back to in-place").

## Evidence
"The named stage predates this diff, but the change expands its role in recovery. Retaining it calls for an explicit, acquisition-only exception to the repository's no-staging decision." (`docs/DECISIONS.md` D-2026-09-29-2.) The reviewer also gave the design it would choose: "Patch an eligible file through its held handle, publish a fresh private file exclusively at a vacant name, and fail the individual file if the occupant has changed."

## Intake
Put to the owner: keep the staging as an exception, or go back to in-place. The owner chose in-place (D-2026-11-23-1).

## What
`Resolved::fresh_as` (`crates/blit-core/src/copy/file_copy/resume.rs`) makes the fresh partial in place:
- A link or a directory at the name is left as it is, and the file fails, as before.
- What stands at the name is removed (`unlinkat` in the held directory) only while it is still what the caller judged — its look, or the handle it opened (cr-ecm29-2). Anything else there now is left as it is, and the file fails. Nothing there any more: the name is empty already.
- The partial is made at the empty name by `O_CREAT|O_EXCL` in the held directory, 600 from the moment it exists. If the name has been taken meanwhile, that is left as it is, and the file fails.
- It is made ready (600, its unfinished length, what `ready` learns). Otherwise it is removed, only while it is still the one made, and the file fails for its retry.

Gone, with nothing taking their place:
- the private staging directory;
- the rename that will not replace, the second name and the exchange;
- the refusal where there is no exchange;
- `Resolved::place`, `publish_exclusive`, `publish_replacing`, `Replacing`, `open_stage`, `STAGE_MODE`;
- the seams `RENAME_FAULTS`, `LINK_FAULTS`, `STAGED_UNLINK_FAULTS`, `EXCHANGE_FAULTS`, `BACK_FAULTS`, `AFTER_AN_EXCHANGE_HOOKS`, `AFTER_A_STAGE_IS_MADE_HOOKS` and `BEFORE_A_FRESH_PARTIAL_IS_PUBLISHED_HOOKS`.

One seam is new: `BEFORE_A_FRESH_PARTIAL_TAKES_THE_NAME_HOOKS`, between the removal and the making. The wire's `hold_whole` and the public `fresh_partial` go through the same `fresh_as`. `CHANGELOG.md` says what a resume now does.

## Guard proof
New guards:
- `resume::tests::a_fresh_partial_is_made_at_the_name_itself`: while the partial is made ready, nothing but the name stands in its directory, and the partial is the file at the name, 600 and unfinished.
- `resume::tests::a_fresh_partial_takes_its_setgid_directorys_group` (cr-ecm30-2's case): in a setgid directory of another of this run's groups, the finished file takes the directory's group.
- `local_copy_mode::a_resume_under_a_narrow_mask_makes_its_fresh_partial` now runs under masks 0200 and 0400 (cr-ecm30-3's case), and both copies complete.
- **Revert proof:** each of the three was run against the staging code of the commit before. Each went red:
  - `[".blit-partial-…", "dst"]` where `["dst"]` was expected;
  - group 1000 where 3 was expected;
  - mask 400 failed the copy.
  The tree was then restored.

Rewritten for in place:
- `a_partial_where_nothing_stood_is_made_exclusively`
- `a_destination_gone_before_its_fresh_partial_takes_the_partial`
- `a_link_that_takes_the_name_while_a_partial_is_had_is_left` (a link put before the removal, and one at the emptied name)
- `a_file_come_since_the_first_look_is_not_replaced` and `sink::tests::the_wires_fresh_partial_leaves_a_file_come_since_its_look`, now on Unix rather than Linux only

Changed expectation, as D-2026-11-23-1 rules: when a fresh partial cannot be made ready, the destination it replaces is already removed. Nothing that looks finished is left at the name, and the file fails for its retry. Before, the destination stood as it was. The guards changed:
- `a_resume_refused_its_cleanup_handle_leaves_nothing_that_looks_finished` (was `…_leaves_the_destination`);
- `sink::tests::a_fresh_partial_is_known_before_it_takes_the_name`;
- `local_copy_mode::a_resume_that_cannot_ready_its_partial_leaves_nothing_that_looks_finished` (was `…_leaves_the_destination`).

Mutations, each restored byte for byte:
- what stands at the name removed whatever it is → red (three guards);
- the partial made whether or not the name is taken → red;
- a destination gone already taken for something else → red;
- a partial not made ready left at the name → red (two guards).

Other checks:
- The resume, sink, session and copy tests (348) and the four local-copy-mode tests pass.
- Windows cross-clippy is clean.
- macOS cross-clippy (`aarch64-apple-darwin`, every target of the workspace, with BLAKE3's C build replaced by its pure-Rust one through `CARGO_FEATURE_PURE=1`) is clean. Before this commit it found cr-ecm30-7's two errors.

Removed guards (test count down by those, called out):
- `what_is_not_put_back_is_left_where_the_error_says`
- `what_comes_while_it_is_put_back_is_left_where_the_error_says`
- `where_there_is_no_exchange_nothing_is_replaced`
- `a_moved_stage_is_not_let_go_by_its_old_name`
- `a_stage_not_known_to_be_the_one_made_is_left`
- `a_fresh_partial_is_staged_in_a_private_directory`
- the second half of `a_partial_where_nothing_stood_is_made_exclusively` (a staged name not let go)

All of them guarded the staging or the exchange, which are gone.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_as` and its docs; the staging and its seams removed; guards.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_whole`'s comment; guards.
- `crates/blit-cli/tests/local_copy_mode.rs` — guards.
- `CHANGELOG.md` — the `--resume` entry.

## Known gaps
Both of these are accepted in D-2026-11-23-1:
- The name stands empty from the removal until the partial is made, and until the copy lands if the partial cannot be made ready.
- Something that comes to the name between the identity check and the removal is removed with it.

Also:
- macOS was checked by cross-clippy only, never run.
