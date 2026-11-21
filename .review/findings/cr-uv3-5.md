# cr-uv3-5: A module that is itself one file, typed with a final `.`, was copied

**Severity**: MEDIUM — a source typed as a directory is copied as the file it is
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv3-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r3` over `eb0cc4d8..dd0a3179` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r3.codex.md`. It completes cr-uv2-1.

## Evidence
"With a module backed by one regular file, `copy HOST:/single/. DEST` exited 0 and wrote that file. Preserve the distinction between the mandatory separator in `HOST:/single/` and an explicit final `.`. Validate the latter even when the normalized relative path is empty."

Cause. cr-uv2-1 sends a source's path as typed and has the daemon refuse one typed as a directory that is none — and left the module's ROOT out of both, as "needing no mark": `HOST:/single/` and `HOST:/single/.` both went as the empty path, and the daemon did not look at a root.

## Intake
Admitted as the reviewer put it.

## What
- The `/` after a module's name is the grammar's and says nothing: the root goes as the empty path. A `.` typed after it is a claim, and the root then goes as `./` (`path_posix::source_path_to_posix`).
- The daemon asks what stands at the root when the path says "a directory", as for any other path (`refuse_source_not_as_typed`).
- Between two daemons the request's own word for the root, `.`, still says nothing; `./` says the claim, and the daemon that pulls opens its session with what the request wrote (`delegated_pull::source_rel_path`).

## Guard proof
- `blit-daemon` `…::a_source_typed_as_a_directory_is_refused_when_it_is_none`: a root that is a file — empty path served, `./` refused `Not a directory`; a path to a file with and without the `/`; directories served either way; a missing path left to the scan.
- `path_posix::…::a_source_keeps_the_mark_that_it_was_typed_as_a_directory` (`.`, `./`, `.//`, `./.` → `./`; empty → empty), `session_client::…::a_source_endpoint_says_it_was_typed_as_a_directory`, `operation_spec::…::a_source_typed_as_a_directory_is_delegated_as_one`, `delegated_pull::…::a_delegated_source_is_opened_as_its_request_wrote_it`.
- Mutations, each restored byte for byte: a `.` typed after the module saying nothing, as before → the three client guards; the daemon not asking what the root is → the daemon guard; the request's word for the root read as a claim → the delegated guard.

## Files changed
- `crates/blit-core/src/path_posix.rs`, `crates/blit-core/src/remote/transfer/{session_client,operation_spec}.rs` (guards), `crates/blit-daemon/src/service/{transfer,delegated_pull}.rs`.

## Known gaps
- **No end-to-end guard**: the test harness serves a directory as its module and has no way to serve a file; the daemon's check is guarded by itself, with a file as the root.
- Run on Linux only.
