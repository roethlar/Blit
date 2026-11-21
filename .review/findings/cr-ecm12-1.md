# cr-ecm12-1: A retry of a child took a file of its name for it

**Severity**: HIGH — a retry overwrites a file at the destination's root that was never part of the run, and exits 0
**Status**: Verified — `exclude-case-r13` (Codex openreview, 2026-11-21) reviewed `c99ad483..bd84649f`, the five `exclude-case-r12` fixes, with `guard_confirmed: true`: "The change correctly distinguishes a single-file source from a same-named child … and versions the new retry semantics for daemon peers. I would keep those decisions." (What a retry does with a name that is not text, in the same code, is cr-ecm13-3)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `07bf16c1`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r12` over `4d0f1a68..c5bab07a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r12.codex.md`. It completes cr-ecm11-1.

## Evidence
"A copy of the directory `Report.txt` failed on its child `Report.txt`. I replaced the source directory with a regular file and placed an unrelated file at `out/Report.txt`. `jobs retry` overwrote that destination and exited 0. A retry for a child must require that child in the manifest. Preserve basename matching for an ordinary `--files-from` list, and carry the distinction through missing-file accounting."

Cause. Two kinds of list go through one filter. A `--files-from` list holds NAMES a person typed, and a source that is one file answers to its own name (cr-ecm9-3). A retry's list holds the PATHS that failed, each a path below the source. cr-ecm11-1 gave the source itself a mark of its own in a retry's list, and left the list otherwise read as names — so a source that had become one file answered to the name of a child that failed, and was sent as the source itself, to the destination's root.

## Intake
Admitted as the reviewer put it.

## What
- **A retry's list says that it holds exact paths** (`FileFilter.files_from_exact`; `FilterSpec.files_from_exact` on the wire, **wire contract 22**, request version 10). Under such a list a source that is one file answers to the mark for itself and to nothing else (`allows_root_file`).
- **A `--files-from` list is unchanged**: it still names a single-file source by that file's name.
- **The account of what was not found knows the difference** (`sole_name_of`): under a list of exact paths a file sent as the source itself accounts for no path below it, so the child is reported `missing at retry`.

## Guard proof
- `job_retry.rs::a_retry_of_a_child_does_not_take_a_file_of_its_name_for_it`, from this machine and from a daemon: the reviewer's sequence — exit 2, `missing at retry`, the file at `out/Report.txt` untouched.
- `transfers::filter::…::a_retry_lists_the_source_itself_by_a_mark_of_its_own`: a retry's list is exact here and at the peer; a single-file source answers to the mark, and not to a child's name; a list a person wrote still names it.
- `transfer_session::…::only_a_list_of_names_calls_a_single_file_by_its_name`; `operation_spec::…::a_request_with_a_list_of_exact_paths_is_version_10_or_later`.
- cr-ecm11-1's and cr-ecm10-3's guards pass unchanged: the source itself is still retried.
- Mutations, each restored byte for byte: a single-file source answering to a child's name in a retry's list, as before; the list built, or sent, as a list of names; the kind of list not read at the peer; a list of exact paths having the file's name for the source; the request's version left at 9 — six, each red on the guards named for it.

## Files changed
- `crates/blit-core/proto/blit.proto`, `crates/blit-core/src/fs_enum.rs`, `crates/blit-core/src/transfers/filter.rs`, `crates/blit-core/src/remote/transfer/{operation_spec,source}.rs`, `crates/blit-core/src/transfer_session/{mod,local}.rs` (`CONTRACT_VERSION = 22`), `crates/blit-daemon/src/service/delegated_pull.rs` (a test's request), `crates/blit-cli/tests/job_retry.rs`; `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- The same within one command's `--retries` goes through the same list and filter; it has no end-to-end guard of its own.
- Run on Linux only.
