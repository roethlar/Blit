# cr-ecm28-2: a link that took an occupied name during staging was overwritten

**Severity**: HIGH — the check catches a link present at the look, but a link that took an occupied entry's place after that check was overwritten by the final rename, locally and in the wire's whole-file path
**Status**: Fixed on Linux, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm28-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r29` over `10145127..e3ea9517` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r29.codex.md`. It completes cr-ecm27-3 (`fa9e1dcf`).

## Evidence
"The new check catches a link present at lookup. If a link takes the place of an occupied entry after that check, the final `renameat` overwrites it. Carry the expected occupant through publication and fail the file on a conflict. This affects both local replacement and the wire whole-file path."

## Intake
Admitted as the reviewer put it. Both paths replace through `Resolved::fresh`, so one change covers both.

## What
- `Resolved::fresh_as`: when the partial is begun, it looks at what stands at the name (`occupant`: mode, device, inode). Nothing there: it is published exclusively. A link or a directory there: left as it is, and the file fails. Anything else is the expected occupant.
- `Resolved::publish_replacing` (Linux): the staged partial is swapped in (`renameat2(RENAME_EXCHANGE)`), and what came out — now in the private stage (cr-ecm28-3) — is looked at. If it is the expected occupant, it goes. If it is anything else, it is swapped back and left as it is, and the file fails. Where there is no swap (`EINVAL`/`ENOSYS`/`EOPNOTSUPP`), and off Linux, the partial is renamed over what stands there, as before.

## Guard proof
- `resume::tests::a_link_that_takes_the_name_while_a_partial_is_staged_is_left` (Linux): a `0644` destination replaced by a link while a fresh partial for a `0600` source is staged (the publication seam) — the resume fails, the link and the file it names untouched, no stage left.
- Mutations, each restored byte for byte: what is swapped out not asked (red); the partial renamed over what stands there (red).
- The resume, sink and session tests (339) and the local-copy-mode tests pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_as`, `Resolved::occupant`, `Resolved::publish_replacing`, `is_directory`, `stage_path`; guard.

## Known gaps
- What came out of the swap is looked at by its path through `/proc/self/fd`.
- Off Linux, and where the filesystem has no swap, the rename over what stands there keeps its window. macOS's swap (`RENAME_SWAP`) is with cr-ecm27-6, blocked.
