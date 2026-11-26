# cr-ecm43-7: the `df` wording guard built its expectation with the formatter it checks

**Severity**: MEDIUM — a formatter giving any words passes it. ci-windows-df-words's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm43-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r44` over `4e12d761..34c77a48` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r44.codex.md`.

## Evidence
"Check `df`'s diagnostic independently. I changed the production formatter to return `review placeholder` for a missing file. The focused test still passed because it calls that same formatter to construct its expectation. Derive the expected text directly from the OS error."

## Intake
Admitted as written; and for ci-windows-missing-text's guard in the daemon, which built its expectation the same way.

## What
Both guards write the words as each system writes them: "No such file or directory" on Unix and "The system cannot find the file specified." on Windows — as `deletion.rs`'s guards already do — not as `os_reason`, the formatter they check, gives them.

## Guard proof
- `du_df::df_reports_the_filesystem_that_holds_the_path` and `service::admin::typed_operand_tests::an_operand_is_removed_as_it_was_typed`: green.
- Mutation, restored byte for byte: the reviewer's — the formatter's words its own (`review placeholder`) → both red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-cli/tests/du_df.rs` — the `df` guard.
- `crates/blit-daemon/src/service/admin.rs` — the `rm` guard.

## Known gaps
- The Windows words are the English system's, as `deletion.rs`'s; the CI runner's. Whether a Windows daemon should give GNU's words is the owner's open question.
