# cr-ecm24-6: a name typed as a directory was made a file

**Severity**: MEDIUM — a fresh partial for `absent/` was renamed to a regular file `absent`; before cr-ecm23-3, which brought this in, the name was refused
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm24-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It completes cr-ecm23-3 (`6bf9c2f9`).

## Evidence
"The base rejected `absent/`. The head created a regular file named `absent` because `file_name()` discarded the trailing slash before `renameat`. Preserve the directory requirement when resolving the final name, including a link target ending in `/` or `/.`."

## Intake
Admitted as the reviewer put it.

## What
`fresh_partial_ready` refuses with `ENOTDIR` a destination written as a directory's — ending in `/` or `/.` — and, while it follows the chain, a link whose target is written so. A file is not made where a directory was named, and nothing is made.

## Guard proof
- `resume::tests::a_name_typed_as_a_directory_takes_no_fresh_partial` (Unix): `absent/`, a link to `gone/` and a link to `gone/.` each refused, and nothing made beside the two links.
- Mutation, restored byte for byte: the directory a name is written as not asked (red).
- The resume, sink and session tests (306) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_partial_ready`, `typed_as_directory`; guard.

## Known gaps
- None known.
