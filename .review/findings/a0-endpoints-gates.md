# a0-endpoints-gates: Phase 5 A.0 — remote-transfer support gates take primitive inputs

**Severity**: Refactor (no behavior change)
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: filled by the sentinel commit

## What

A.0 sub-slice 6 of the `transfers/*` track. Pulls the three
remote-transfer support gates (`ensure_remote_common`,
`ensure_remote_pull_supported`, `ensure_remote_push_supported`)
into `blit_app::endpoints` so the future TUI can call them
without a clap / `TransferArgs` dependency.

New in `blit_app::endpoints`:

- `ensure_remote_transfer_supported(dry_run: bool,
  workers_limit_set: bool) -> Result<()>` — the common gate
  shared by every remote-touching path. Rejects `--dry-run`
  (remote transfers don't have dry-run semantics yet) and
  `--workers` (the limiter is a local-side concept that the
  remote pipeline ignores).
- `ensure_remote_pull_supported(dry_run, workers_limit_set)`
  — remote→local pull gate. Allows `--checksum` because
  PullSync has F11 ack negotiation (the daemon advertises
  checksum support and the CLI bails cleanly if it's off).
- `ensure_remote_push_supported(dry_run, workers_limit_set,
  checksum)` — local→remote push + remote→remote relay gate.
  Rejects `--checksum` because the push protocol has no
  per-transfer capability negotiation today.

All three take primitive booleans. Error messages still
reference CLI flag names (`--dry-run`, `--workers`,
`--checksum`) — those are the documented surface the user
sees, and if the TUI ever surfaces the refusal verbatim it
can map flag names to its own labels.

CLI side
(`crates/blit-cli/src/transfers/endpoints.rs`):

- Reduced to two paper-thin wrappers that translate
  `&TransferArgs` → primitive booleans:
  - `ensure_remote_pull_supported(args)` →
    `blit_app::endpoints::ensure_remote_pull_supported(
       args.dry_run, args.workers.is_some())`.
  - `ensure_remote_push_supported(args)` →
    `blit_app::endpoints::ensure_remote_push_supported(
       args.dry_run, args.workers.is_some(), args.checksum)`.
- Module doc updated. The shim preserves the existing public
  call-site names (`crate::transfers::endpoints::ensure_*`)
  so `transfers/mod.rs` dispatch arms work without import
  changes.
- `ensure_remote_common` no longer exists CLI-side
  (replaced by the library's
  `ensure_remote_transfer_supported`).

## Approach

The gates are tiny (a few field reads + bail messages) but
their inputs were clap-shaped. The translation is mechanical:
each `args.X` field maps to a primitive bool parameter; the
CLI wrapper does the mapping; the library function has no
notion of `TransferArgs`. Same pattern as the
`build_filter` / `build_filter_spec` wrappers from the
filter slice.

The library function's error messages still mention CLI flag
names because:

1. The release-1 user only ever sees the CLI; flag names are
   their mental model.
2. The TUI's release won't be in 0.1.0 (per the release plan);
   when it lands, it can wrap each library call and remap the
   message to TUI vocabulary at the catch point.
3. The alternative (library returns a structured refusal
   reason; caller formats) would be heavier for three
   bail conditions and would force me to design a stable
   refusal-reason enum across both consumers.

## Files changed

- `crates/blit-app/src/endpoints.rs`
  - +`ensure_remote_transfer_supported` (5 LOC + 9 LOC doc).
  - +`ensure_remote_pull_supported` (3 LOC + 7 LOC doc).
  - +`ensure_remote_push_supported` (10 LOC + 5 LOC doc).
- `crates/blit-cli/src/transfers/endpoints.rs`
  - Removed `ensure_remote_common` (replaced by library).
  - Trimmed bail messages (now in library).
  - Two wrappers delegate to library functions.
  - Module doc updated.

## Tests added

None new. Workspace total unchanged at 503 passed. The gates
are simple boolean checks — the existing CLI integration
tests that exercise rejection paths (e.g., `--dry-run` on
remote, `--checksum` on push) continue to exercise the
library function through the wrappers.

## Known gaps

- Final cleanup pass — last remaining A.0 item. Looking at
  what's left in CLI's `transfers/`:
  - `transfers/mod.rs` — still owns `run_transfer`,
    `run_move`, the local `display_endpoint` /
    `collapse_slashes` helpers, the
    `filter_inputs` / `build_filter` / `build_filter_spec`
    wrappers, `confirm_destructive_operation`, and the
    `dest_is_container` / `resolve_destination` /
    `source_is_contents` re-exports.
  - `transfers/local.rs` — likely still has thin CLI
    wrappers; should audit for dead re-exports.
  - `transfers/remote.rs` — the progress monitor +
    presentation printers; intentionally CLI-side.
  - `transfers/remote_remote_direct.rs` — the
    presentation printers; intentionally CLI-side.
  - `transfers/endpoints.rs` — two wrappers + re-exports.
  None of these are *dead* — they're all wrappers or
  presentation. The "Final cleanup" slice should walk each
  file and confirm: every remaining symbol is either
  (a) clap-coupled (wrapper), (b) presentation
  (println/eprintln/print_json), or (c) interactive
  (stdin prompts). Anything that's pure logic without
  one of those three reasons should move.

## Reviewer comments

### Round 1 (reviewed sha `c392f37`) — reopened

Reviewer: `codex-reviewer`. Validation green. One low-severity
finding:

1. The module-level doc on `crates/blit-app/src/endpoints.rs`
   still said the two clap-coupled gates stayed in `blit-cli`
   and would move in a "later" commit. This commit IS the
   move — `ensure_remote_transfer_supported`,
   `ensure_remote_pull_supported`, and
   `ensure_remote_push_supported` now live in
   `blit_app::endpoints` with primitive inputs. Public-doc
   contract issue, not a behavior bug.

   Fix direction: update the module doc to describe current
   ownership.

### Round 2 (sha pending) — addresses round-1 finding

Reworded the module doc block to describe current ownership:

- Endpoint parsing (`parse_transfer_endpoint` /
  `parse_endpoint_or_local`) — same as before.
- Three remote-transfer support gates now in `blit-app` with
  primitive-bool inputs; CLI keeps two paper-thin
  `&TransferArgs` wrappers. Documented the rationale for
  keeping CLI flag names in the error messages.
- Endpoint-shape gates (Discovery-rejection) — was previously
  undocumented in the module preamble; added a line.

No code changes. Workspace still at 503 passed.
