# audit-7b-dead-code: remove dead functions, stale allows, and an empty module

**Severity**: Style / Code health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `5a5f735`
**Parent finding**: `audit-7-code-health`.

## What

Several `#[allow(dead_code)]` sites and an empty placeholder module. Each
was verified individually (grep for real usage) before removal — the
right action differs per site (delete the code vs. just drop a stale
allow).

## Per-site resolution

- **`crates/blit-core/src/copy/compare.rs`** — deleted
  `files_have_different_content` + `hash_file_content`. Provably dead: a
  workspace grep found no caller of the former, and the latter was only
  called by it. Also removed their now-orphaned `std::fs::File` /
  `std::io::Read` imports.
- **`crates/blit-core/src/fs_enum.rs`** — removed the three
  `#[allow(dead_code)]` on `FileFilter`'s `compiled_includes` /
  `compiled_files` / `compiled_dirs`. The attributes were **stale**: the
  fields ARE read via `include_globs` / `file_globs` / `dir_globs`
  (`get_or_init`). `clippy --workspace --all-targets -D warnings` stays
  clean without them, confirming the fields are live.
- **`crates/blit-tui/src/diagnostics.rs`** — removed the write-only
  `written_at: Instant` field on `DiagnosticsStatus::Done` (set in
  `apply_done`, never read anywhere) plus its now-unused
  `std::time::Instant` import.
- **`crates/blit-app/src/progress.rs`** — deleted the empty placeholder
  module (a lone doc comment, no items) and its `pub mod progress;`
  declaration. Nothing imports `blit_app::progress` (the live
  `RemoteTransferProgress` lives in `blit_core`'s transfer module).

## Not touched

`crates/blit-cli/src/transfers/remote_remote_direct.rs` — the loop note
called it an empty module, but it is 285 lines and **live**
(`run_remote_to_remote_direct*` are called from `transfers/mod.rs`). Left
as-is.

## Verification

`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
and `cargo test --workspace` all pass. The clippy run is the load-bearing
check that the fs_enum allow-removals didn't surface a real dead-code
warning.

## Scope

One sub-item of audit-7. Remaining: 7c (ARCHITECTURE/README), 7d (main.rs
refactor). 7-cargo-lock + 7e verified.

## Reviewer comments

(empty — pending review)
