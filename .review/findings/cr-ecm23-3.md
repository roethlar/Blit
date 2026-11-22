# cr-ecm23-3: the end of a chain of links was not held within the destination

**Severity**: HIGH — a resume through a link whose chain ends at a name outside the destination, not there yet, made its fresh partial there: an unfinished file outside the selected tree
**Status**: Reviewed, found incomplete — `combined-r25` (Codex openreview, 2026-11-22) reviewed `c89b1eb1..19025839` — the six `combined-r24` fixes — with `guard_confirmed: true`. "`open_resume_partial` opens with `create(true)` before consulting `within`. A dangling destination link can therefore create a file outside the selected tree. The newly created `0600` file can immediately pass the reuse check. Resolve and hold the final parent for this path too." "A resumed loopback pull from a `0600` source to an existing `0644` target succeeded at base and exited 2 at head. The canonical root is the file itself, so testing whether its parent starts with that root rejects the requested target." "The base rejected `absent/`. The head created a regular file named `absent` because `file_name()` discarded the trailing slash before `renameat`. Preserve the directory requirement when resolving the final name, including a link target ending in `/` or `/.`." "A fresh replacement in a `0300` directory succeeded at base and failed with `EACCES` at head. The operation needs write and search permission. The new `O_RDONLY` open also demands permission to list the directory. Use the existing `TO_SEARCH` flags." The remaining work is cr-ecm24-1, -3, -6 and -7; three of them are faults it brought in
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `6bf9c2f9`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r24` over `24d4064f..c89b1eb1` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r24.codex.md`. It completes cr-ecm22-2 (`2678a740`).

## Evidence
"When a whole-file fallback encounters a dangling link chain, the containment check can accept the original link's parent. `fresh_partial` then follows the chain and can create an unfinished file outside the selected tree. Validate and hold the final parent before creating the sibling there."

## Intake
Admitted as the reviewer put it. The containment check (`path_safety::verify_contained`) resolves the deepest part of a path that exists, so for a link to a name that does not exist it judges the link's own directory. The fresh partial is where the chain is followed to its end, so that is where its end is judged.

## What
`fresh_partial` / `fresh_partial_ready` take the destination's canonical root (`within`). Once the chain of links is followed to its end:
- the directory it ends in is resolved; when a root is given, a directory outside it fails the file ("leads out of the destination");
- that directory is HELD, opened from `/` one name at a time and never through a link (`held::directory_unlinked`);
- the staged partial is made in it (`openat`, `O_EXCL | O_NOFOLLOW`), made ready, and renamed in it (`renameat`), or removed from it (`unlinkat`).

A folder on the way replaced since cannot move where the file is made or named. The wire's whole hold passes the sink's canonical root, and so does the local route (`write_file_payload` → `copy_resolved_file_payload` → `copy_and_stamp_opened` → `open_resume_partial`). A file that is its own root, and the standalone `resume_copy_file`, pass none.

## Guard proof
- `sink::tests::a_fresh_partial_is_not_made_outside_the_destination` (Unix): `dst/r.bin → ../outside/new`, held whole — nothing made in `outside`, and the completion fails.
- `resume::tests::a_fresh_partial_is_made_in_the_directory_it_holds` (Unix): the destination's folder renamed and its path made a link to another folder once the directory is held (a seam) — the partial lands in the folder held, nothing in the other.
- Mutations, each restored byte for byte: the end of the chain not checked against the destination (red); the directory opened by its path when the partial is made (red).
- The resume, sink and session tests (302) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/held.rs` — `directory_unlinked`.
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_partial`, `fresh_partial_ready`, `open_resume_partial`, the seam; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — the root carried to the hold and the local route; guard.

## Known gaps
- The chain itself is read by its path, link by link, before its end's directory is held. A link of the chain changed while it is read can change which directory is judged and held. Once held, that directory is the one written to.
- A hold with no word of the hashes (`hold_partial_unfinished`) still opens the name with `create`. Production always notes the hashes' word first.
- Unix only.
