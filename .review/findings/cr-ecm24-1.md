# cr-ecm24-1: a local resume created through a link before the destination was checked

**Severity**: HIGH — a local resume through a link to a name outside the destination, not there yet, made a `0600` file there, and that new file then passed the reuse check
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm24-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It completes cr-ecm23-3 (`6bf9c2f9`).

## Evidence
"`open_resume_partial` opens with `create(true)` before consulting `within`. A dangling destination link can therefore create a file outside the selected tree. The newly created `0600` file can immediately pass the reuse check. Resolve and hold the final parent for this path too."

## Intake
Admitted, done as the reviewer's verdict frames it: "use one operation to acquire the destination for both local copies and wire receives … resolve the selected target before creating anything".

## What
- `resolve_destination(dst, within) -> Resolved` (Unix) is the one resolution. It follows the chain of links at the name, refuses a name written as a directory's, checks the end's directory against the destination when a root is given, and holds that directory to be searched. A `Resolved` opens the file at its name there (`open`, never through a link, one made being 600), or makes a fresh partial in its place (`fresh`: staged, readied, renamed, all in the directory held).
- The local resume (`open_resume_partial`) has its partial through it — opened where it stands, opened read-only to be made writable, opened again, or replaced fresh — never by the name.
- The wire's hold with no word of the hashes (`hold_partial_unfinished`) has its partial the same way on Unix.
- `fresh_partial_ready` is `resolve_destination(..).fresh(..)`.

## Guard proof
- `resume::tests::a_local_resume_makes_nothing_outside_the_destination` (Unix): `dst/r.bin → ../outside/new` resumed locally within `dst` — refused, nothing in `outside`.
- `sink::tests::a_hold_with_no_word_of_the_hashes_makes_nothing_outside` (Unix): the same link held by the wire with no baseline — nothing in `outside`, and the completion fails.
- Mutations, each restored byte for byte: a local resume opening the name to make it before the check (red); the wire's hold opening the name to make it (red).
- The resume, sink and session tests (308) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `resolve_destination`, `Resolved`, `open_in`; `fresh_partial_ready` and `open_resume_partial` through them; guard.
- `crates/blit-core/src/copy/mod.rs` — `resolve_destination` to the crate.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_partial_unfinished`; guard.

## Known gaps
- The wire's hashed hold (`hold_hashed`) still opens the file by its name, twice, comparing what it opened. The file is one that exists, and the path check resolves it whole.
- Unix only.
