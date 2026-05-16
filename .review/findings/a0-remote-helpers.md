# a0-remote-helpers: Phase 5 A.0 — pull-flow helpers to blit-app

**Severity**: Refactor (no behavior change)
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: (filled by sentinel after commit)

## What

Sub-slice 1 of the `transfers/remote.rs` track. Pulls two pure
orchestration helpers used by the pull-sync flow out of the CLI
into `blit_app::transfers::remote`:

- `enumerate_local_manifest(root, compute_checksums)` — walks a
  local destination tree and produces the `Vec<FileHeader>` the
  pull-sync handshake sends to the daemon for comparison.
  Compute-checksums mode hashes in parallel via rayon;
  metadata-only mode runs sequentially.
- `delete_listed_paths(dest_root, relative_paths)` + the small
  `LocalPurgeStats { files_deleted, dirs_deleted }` struct —
  applies the daemon-authored delete list during a mirror
  pull, with canonical-containment safety
  (`safe_join_contained`) per R5-F1 / R46-F3.

These are pure orchestration helpers — no clap, no
presentation. The TUI's future pull-trigger affordance
consumes them directly with no wrapper.

The push and pull *entry-points* (`run_remote_push_transfer`,
`run_remote_pull_transfer`) and the CLI-side progress monitor
(`spawn_progress_monitor_with_options`) stay in
`crates/blit-cli/src/transfers/remote.rs` for the next
sub-slice; the progress reshape into `AppProgressEvent` is its
own separate pause point per the reviewer's earlier guidance.

## Approach

Move + visibility widening, no algorithmic changes. The R46-F3
canonical-containment behavior in `delete_listed_paths`
(fail-closed if dest can't be canonicalized) is preserved
verbatim — that's the load-bearing safety property.

`LocalPurgeStats.files_deleted` and `.dirs_deleted` change from
private fields (private to the CLI's `transfers/remote.rs`
module, where the printer code also lived) to `pub` — the CLI's
JSON / text formatters read them across the new crate boundary.

CLI imports the three names at the top of
`crates/blit-cli/src/transfers/remote.rs` via
`use blit_app::transfers::remote::{...}`. Existing call sites
inside `run_remote_pull_transfer_inner` and the printers don't
change.

## Files changed

- `crates/blit-app/src/transfers/remote.rs` — new module body
  with the three items (89 + 4 + 70 lines of substance plus
  doc comments)
- `crates/blit-cli/src/transfers/remote.rs` —
  - removed: `enumerate_local_manifest`, `delete_listed_paths`,
    `LocalPurgeStats` (215 LOC)
  - added: 3 `use` imports from `blit_app::transfers::remote`
  - dropped now-unused `use eyre::eyre` (only `Context` remains)

## Tests added

None new. Existing pull-sync integration tests at
`crates/blit-cli/tests/remote_pull_*.rs` and
`crates/blit-cli/tests/remote_pull_mirror.rs` exercise the
moved code through the unchanged CLI entry-points; workspace
total stays at 496.

## Known gaps

The R46-F3 mirror-purge containment property is the highest-risk
preserved behavior in this slice. Worth a targeted spot-check
during review:
`blit_app::transfers::remote::delete_listed_paths` lines
~160–175 must still call `canonical_dest_root` first and bail
on error, then route every `rel` through `safe_join_contained`
before any `remove_file` call. No lexical fallback.

## Reviewer comments

(empty — pending grade)
