# audit-7c-docs: ARCHITECTURE/README updates for the full crate + RPC surface

**Severity**: Docs
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `a11845a`
**Parent finding**: `audit-7-code-health`.

## What

`docs/ARCHITECTURE.md` documented only `blit-core` / `blit-cli` /
`blit-daemon` (the workspace has six crates) and its gRPC-service listing
covered 7 of the 15 RPCs. `README.md`'s clone URL was a `your_org`
placeholder.

## Approach (docs-only)

- **ARCHITECTURE.md — Crate Structure:** added sections for the three
  undocumented crates, written against the actual code:
  - `blit-app` — shared orchestration library (endpoints, transfer
    dispatch/resolution/filter, the DNS-aware control-plane client, admin
    verbs, diagnostics) used by both binaries.
  - `blit-tui` — ratatui control surface (F1–F4; Subscribe/GetState/
    CancelJob/ClearRecent; mDNS discovery; configurable keys/theme).
  - `blit-prometheus-bridge` — pull-model `/metrics` exporter (fresh
    GetState per scrape; up-but-down on failure).
- **ARCHITECTURE.md — System Overview diagram:** added the TUI + bridge
  front-ends and the `blit-app` layer.
- **ARCHITECTURE.md — gRPC Services:** completed the proto surface to all
  15 RPCs (added PullSync, DelegatedPull, CompletePath, ListModules,
  GetState, Subscribe, CancelJob, ClearRecent), verified against
  `proto/blit.proto`, with a one-line note on the state/observability
  verbs.
- **README.md:** clone URL `your_org/blit` → `roethlar/Blit` (the actual
  `origin` remote), `cd blit` → `cd Blit`.

## Verification

Docs-only — no code change. `cargo fmt --check`, `cargo build
--workspace`, `cargo test --workspace` all still green. RPC list and crate
set checked against `proto/blit.proto` and `crates/`. README has no crate
tree of its own (the tree lives in ARCHITECTURE.md), so only the clone URL
needed fixing there.

## Scope

One sub-item of audit-7. Remaining: 7d (main.rs refactor). 7-cargo-lock /
7b / 7e verified.

## Round 2 (commit `a11845a`)

**Reopen finding (docs accuracy, 3 points):**
1. The blit-app prose listed only CLI/TUI as consumers, but
   `blit-prometheus-bridge` also depends on blit-app (imports
   `blit_app::admin::jobs`) — and the diagram already placed the bridge
   over blit-app.
2. The blit-app module summary omitted live public modules (`check`,
   `scan`, `display`).
3. F4 was described as just "profile"; it renders Profile + Verify +
   Diagnostics.

**Fix:** rewrote the blit-app section with a module table covering all of
`lib.rs`'s exports (endpoints/transfers/client/admin/check/scan/
diagnostics/profile/display) and naming the bridge as an `admin::jobs`
consumer; changed the blit-tui line to "F4 profile/verify/diagnostics".
Docs-only.

## Reviewer comments

(empty — pending round-2 grade)
