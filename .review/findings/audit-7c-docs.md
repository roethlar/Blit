# audit-7c-docs: ARCHITECTURE/README updates for the full crate + RPC surface

**Severity**: Docs
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `43087f5`
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

## Reviewer comments

(empty — pending review)
