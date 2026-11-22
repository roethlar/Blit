# cr-ecm27-3: a FIFO or a link that took the name during acquisition blocked it or was overwritten

**Severity**: HIGH — a read-only FIFO put in a regular file's place after the look could block the open indefinitely, before the descriptor check; a link put at the resolved name took the replacement branch and was overwritten
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm27-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r28` over `e025a3e2..10145127` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r28.codex.md`. It completes cr-ecm26-5 (`1435ba23`).

## Evidence
"A read-only FIFO that takes a regular file's place after lookup can block the open indefinitely, before the descriptor check runs. Use a nonblocking open and validate the descriptor. A symlink that appears at the resolved leaf can instead enter the generic replacement branch and be overwritten. Retry link resolution or fail that file while preserving the link."

## Intake
Admitted; a link at the name fails the file and is left as it is.

## What
- Local (`open_resume_partial_with`): its opens — writable, read-only, and the writable reopen — do not wait (`O_NONBLOCK`). The descriptor is then checked to be a regular file (cr-ecm26-5), and anything else is replaced.
- A link that has come to stand at the resolved name, after the chain was followed, is left as it is, and the file fails — locally and in the wire's whole hold (`hold_whole`, which now looks at the name rather than only asking whether something stands there).
- A seam (`AFTER_A_RESOLVE_HOOKS`) runs once a destination is resolved and before anything at the name is looked at.

## Guard proof
- `resume::tests::what_takes_the_name_during_acquisition_is_met_safely` (Unix): a `0444` FIFO put in a reusable file's place after the look — the resume ends within its bound, a regular file with the source's bytes. A link put at the name before the look — the resume fails, the link and the file it names untouched.
- `sink::tests::a_link_that_takes_the_name_of_a_whole_hold_is_left` (Unix): the same link for the wire's whole hold — left a link, the file fails.
- Mutations, each restored byte for byte: the read-only open waiting (red); a link at the name replaced locally (red); replaced by the wire (red).
- The resume, sink and session tests (332) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — the opens, `is_link`, `after_a_destination_is_resolved`, its seam; guard.
- `crates/blit-core/src/copy/mod.rs` — `is_link` and the seam to the crate.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_whole`; guard.

## Known gaps
- The resolution is not retried; such a file fails, and the next run resolves it afresh.
- Unix only.
