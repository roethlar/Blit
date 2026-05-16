# a0-dispatch: Phase 5 A.0 — TransferKind + route selector

**Severity**: Refactor (no behavior change)
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: filled by the sentinel commit

## What

A.0 sub-slice 5 of the `transfers/*` track. Pulls
transport-route selection into `blit-app` so the CLI's
`run_transfer` and the future TUI's transfer launcher answer
the same "which transport do we use?" question via a single
pure function.

New in `blit_app::transfers::dispatch`:

- `TransferKind` (`Copy` / `Mirror`) — moved from the CLI's
  `transfers::mod`. CLI re-exports it.
- `TransferKind::is_mirror()` — small convenience used by
  the two CLI call sites that previously did
  `matches!(mode, TransferKind::Mirror)`.
- `TransferRoute` enum — 5 variants naming the transport
  choices:
  - `LocalToLocal { src: PathBuf, dst: PathBuf, mirror: bool }`
  - `LocalToRemote { src: PathBuf, dst: RemoteEndpoint, mirror }`
  - `RemoteToLocal { src: RemoteEndpoint, dst: PathBuf, mirror }`
  - `RemoteToRemoteDelegated { src, dst, mirror }`
  - `RemoteToRemoteRelay { src, dst, mirror }`
- `select_transfer_route(src, dst, kind, relay_via_cli) ->
  TransferRoute` — pure dispatch. Total over the
  `(Endpoint, Endpoint)` product; no I/O, no error cases. The
  `mirror` flag is reproduced on every variant so the caller
  doesn't have to thread `mirror_mode` separately.

7 unit tests cover each variant plus the
"relay_via_cli only affects remote→remote" invariant.

## Approach — why `run_transfer` and `run_move` stay in CLI

The original plan in REVIEW.md listed `run_transfer`,
`run_move`, and `TransferKind` together as a single dispatcher
slice. Working through the bodies showed that's the wrong
shape: ~80% of those functions is CLI-specific work that
doesn't translate to a library:

- **`--null` data-loss gates** (R52, R54) — error messages
  mention `blit copy --null SRC DST` as the safe escape hatch.
  Library can't reference CLI verbs.
- **`run_move`'s filter / `--force` / `--checksum` /
  `--ignore-times` / `--ignore-existing` rejections** —
  multi-paragraph guidance pointing users at `blit copy
  --checksum` / `blit rm` recovery commands, tailored per
  transport direction (R49-F1, R51-F1, R55, R54-F2).
- **Mirror confirmation prompt** — interactive stdin, CLI
  only. (TUI would have its own modal.)
- **Banner emission** — `eprintln!("starting copy SRC -> DST")`.
- **`display_endpoint` / `collapse_slashes`** — CLI display
  helpers.

The reusable kernel is the dispatch decision itself —
"given parsed endpoints + mode + relay choice, pick the
transport." That's what this slice extracts.

The TUI will write its own `run_transfer` analogue that:
1. Builds parsed endpoints from picker selections.
2. Runs TUI-shaped equivalent gates (modal warnings,
   confirmation dialogs).
3. Calls `select_transfer_route`.
4. Matches on the route variant to invoke the right
   per-transport execution function in
   `blit_app::transfers::{local, remote}`.

Both consumers share the dispatch primitive. Neither bakes
the other's presentation into the library.

## Files changed

- `crates/blit-app/src/transfers/dispatch.rs` — new module:
  `TransferKind` + `TransferRoute` + `select_transfer_route` +
  7 unit tests (~210 LOC including doc comments).
- `crates/blit-app/src/transfers/mod.rs` — declares `dispatch`
  module; module doc updated to call out the CLI-stays
  decision.
- `crates/blit-cli/src/transfers/mod.rs`
  - Removed local `TransferKind` definition.
  - Added `pub use blit_app::transfers::dispatch::{...}`.
  - `run_transfer` dispatch arm rewritten to match on
    `TransferRoute`.
  - `matches!(mode, TransferKind::Mirror)` → `mode.is_mirror()`
    at the two call sites that needed it.

## Tests added

7 new dispatch unit tests in `blit-app`. Workspace total goes
from 496 → 503 passed. No tests removed.

## Known gaps

- Endpoints clap-coupled gates (`ensure_remote_pull_supported`
  / `ensure_remote_push_supported` / `ensure_remote_common`)
  still take `&TransferArgs`. The gate logic is tiny (a few
  field reads + bail messages) — reshape to primitive inputs
  is the next slice.
- Final cleanup pass after the gates: drop any CLI shim
  re-exports that no longer have callers.

## Reviewer comments

(empty — pending grade)
