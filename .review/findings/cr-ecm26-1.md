# cr-ecm26-1: a partial where nothing stood was not made exclusively from the first look

**Severity**: HIGH — locally, a look that found nothing was followed by an ordinary writable open that could adopt a file made in between; where links are refused, a look and a rename could replace a file made between them; and a staged name left after a successful link was not reported
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm26-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r27` over `05ff50d7..e025a3e2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r27.codex.md`. It completes cr-ecm25-2 (`e2f50cae`) and cr-ecm25-4 (`a8283606`).

## Evidence
"Locally, a missing-name lookup is followed by an ordinary writable open, which can adopt a file created in between. On a filesystem that refuses hard links, the final existence check and `renameat` can likewise overwrite a concurrent creation. Select exclusive publication immediately and use an atomic no-replace operation, failing that file safely if unavailable. Also check staging-name removal after a successful hard link so a cleanup failure cannot silently leave a second name."

## Intake
Admitted as the reviewer put it.

## What
- `open_resume_partial_with`: a look that finds nothing at the name decides. The partial is made exclusively (`fresh_exclusive`), and nothing is opened by the name.
- `Resolved::publish_exclusive`: the staged partial takes the name through a second name (`linkat`, which fails if the name is taken), and the staged name is let go. If that removal fails, the name it took is removed again while it is this partial, the staged name is tried once more, and the file fails, saying why. Where the filesystem has no second names (`EPERM`/`EOPNOTSUPP`), it is renamed with `renameat2(RENAME_NOREPLACE)` on Linux. Where there is no such rename, the file fails safely; the look-and-rename is gone.
- Test seams: one runs after the look (`AFTER_A_LOOK_HOOKS`); others refuse the second name (`LINK_FAULTS`) and the staged name's removal (`STAGED_UNLINK_FAULTS`).

## Guard proof
- `resume::tests::a_partial_where_nothing_stood_is_made_exclusively` (Unix): a file put at the name after the look is left as it is and the resume fails, with links and with links refused. A staged name that cannot be let go fails the resume, leaving nothing at the name and no staged name.
- Mutations, each restored byte for byte: a look that finds nothing followed by an open (red); where links are refused, a rename that replaces (red); the staged name's removal not asked (red).
- The resume, sink and session tests (327) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial_with`, `Resolved::publish_exclusive`, the seams; guard.

## Known gaps
- Off Linux, a filesystem with no second names now fails such a file rather than risk replacing one. No such filesystem is reachable from here.
- Unix only.
