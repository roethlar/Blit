# cr-ecm28-3: the staged name itself could be replaced before it was published

**Severity**: HIGH — the staged partial stood beside the destination, so another directory operation could replace the staged name, and the exclusive rename then published an inode other than the verified one the caller held; the error cleanup could likewise unlink the replacement
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm28-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r29` over `10145127..e3ea9517` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r29.codex.md`. It completes cr-ecm27-1 (`0a70800e`).

## Evidence
"If another directory operation replaces the staged name, the exclusive rename can publish a different inode from the verified one the caller still holds. Error cleanup can likewise unlink the replacement. Use handle-based publication or a private staging directory held by descriptor."

## Intake
Admitted; the private staging directory, held by descriptor.

## What
`Resolved::fresh_as` stages a fresh partial in a directory of its own. The directory (`.blit-partial-<pid>-<n>`) is made private (700) in the destination's directory and opened without following a link. It is checked to be the one this run made: a directory this run's user owns, private (`open_stage`). The partial is made there, made ready, and published from there — renamed in, by the rename that will not replace, or by a second name. The emptied directory is then let go. The cleanup removes the partial only inside that directory, which no other user can write.

## Guard proof
- `resume::tests::a_fresh_partial_is_staged_in_a_private_directory` (Unix): while the partial is staged, the only other entry beside the destination is one directory, `0700` and this run's, with the partial in it. Once it is published, nothing is left beside the destination.
- Mutations, each restored byte for byte: the staging directory not private (red); not let go (red).
- The resume, sink and session tests (338) and the local-copy-mode tests pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_as`, `publish_exclusive`, `open_stage`, `STAGE_MODE`, the seam; guard.

## Known gaps
- The emptied staging directory is let go by its name. Only an empty directory can be removed that way.
- A same-user process can still work inside the private directory; the protection is against other users.
- Unix only.
