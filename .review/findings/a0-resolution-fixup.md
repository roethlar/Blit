# a0-resolution-fixup: Phase 5 A.0 resolution slice — three followups

**Severity**: Low
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: `65f6031`

## What

Three low-severity followups against `3639159` (the
rsync-resolution slice of A.0):

1. Stale doc comment in `crates/blit-cli/src/diagnostics.rs:154`
   claimed the rsync-resolution helpers still lived in
   `crate::transfers::mod`. They moved to
   `blit_app::transfers::resolution` in `3639159`.
2. `docs/bugs/copy-destination-semantics.md:3` Resolution
   section pointed at the pre-move CLI path
   (`crates/blit-cli/src/transfers/mod.rs::resolve_destination`).
3. The 14 rsync-resolution unit tests (`source_is_contents_*`,
   `resolve_destination_*`) were still in
   `crates/blit-cli/src/transfers/mod.rs` test module; they
   passed through the CLI re-export shim but the public library
   API in `blit_app::transfers::resolution` carried no tests of
   its own.

## Approach

1. Doc rewording: now points at `blit_app::transfers::resolution`
   and mentions the CLI re-export shim pattern for orientation.
2. Bug-doc Resolution block updated to the blit-app path; CLI
   re-export mentioned; in-library tests added to the
   regression-test citation list.
3. Tests moved (not copied) from the CLI test module to a new
   `#[cfg(test)] mod tests` inside
   `crates/blit-app/src/transfers/resolution.rs`. The CLI's
   test module retains only the two end-to-end dispatcher tests
   (`copy_local_transfers_file`,
   `copy_local_dry_run_creates_no_files`).

## Files changed

- `crates/blit-cli/src/diagnostics.rs:154` — doc reword
- `docs/bugs/copy-destination-semantics.md:3` — path update
- `crates/blit-app/src/transfers/resolution.rs` — appended
  `#[cfg(test)] mod tests` block (14 tests)
- `crates/blit-cli/src/transfers/mod.rs` — removed the 14 moved
  tests; dropped unused `use std::path::PathBuf` from the test
  module

## Tests added

None new — the 14 tests moved crates without changing.
Workspace total stays at 496.

## Known gaps

The CLI's `pub(crate) use blit_app::transfers::resolution::*`
shim re-export in `crates/blit-cli/src/transfers/mod.rs:207`
still exists so existing in-crate call sites
(`crate::diagnostics::run_diagnostics_dump`,
`crate::transfers::run_transfer`) work unchanged. Removing the
shim is deferred to a later A.0 slice when the call sites
migrate to `use blit_app::transfers::resolution::*` directly.

## Reviewer comments

(empty — pending grade)
