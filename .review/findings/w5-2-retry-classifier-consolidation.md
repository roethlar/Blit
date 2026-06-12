# w5-2-retry-classifier-consolidation — delete dead errors.rs; classifier to blit-core

**Branch**: `master` (owner-authorized session 2026-06-12, "Continue with 1")
**Commit**: `9c960dc`
**Source findings**: errors-dead-classifier-contradicts-live (reviewer: high),
boundaries-retry-policy-split-dead-classifier, duplication-retry-classifier-dead-twin,
deadcode-core-errors-contradictory-classifier — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

`blit-core/src/errors.rs` (154 lines) had zero importers, was publicly
exported, name-collided with the proto `TransferError`, and inverted the
live retry policy on five `io::ErrorKind`s — a trap inviting the next
contributor to "wire up" the wrong table. Deleted. The live classifier
moves from blit-app into blit-core as the single owner.

## Approach

- Deleted `errors.rs` + the `lib.rs` export.
- New `blit_core::remote::retry`: `is_retryable` (verbatim semantics) +
  `is_retryable_io_kind` (now `pub` — W1.1's tests will want to classify
  bare kinds). Module doc records why it lives here and what the dead
  twin got wrong.
- `blit_app::transfers::retry` re-exports `is_retryable` (public API
  unchanged — `run_with_retries` and its loop tests stay put); its two
  classifier tests survive as re-export checks.
- Stale `blit_app::transfers::retry::is_retryable` comment paths in core
  `remote/pull.rs` (audit-h3c-2 TODOs) updated to the new home.

## Files changed

- `crates/blit-core/src/errors.rs` (deleted), `src/lib.rs`,
  `src/remote/mod.rs`, `src/remote/retry.rs` (new),
  `src/remote/pull.rs` (two comment lines)
- `crates/blit-app/src/transfers/retry.rs`

## Tests added

3 in `remote/retry.rs`: a kind-by-kind contract test pinning the full
retryable/fatal table (explicitly covering the five kinds the dead module
inverted: ConnectionRefused/UnexpectedEof/NotConnected retryable,
Interrupted/WouldBlock fatal); plain-eyre-is-fatal; and a
deep-context-chain classification test (the W1.1 chain-preservation net).

**Test count drops 1369 → 1368**: the dead module's 4 unit tests are
deleted with the module they tested (AGENTS.md §5 call-out — they
asserted the *inverted* table, so they were pinning wrong behavior).

## Known gaps

- The audit-h3c-2 TODOs in pull.rs (chain-amputating Status→String
  conversions) remain — that is W1.1-class work by design; the
  deep-chain test here is its forward net.
- `is_retryable` still only classifies `std::io::Error` in the chain;
  tonic::Status-aware classification is part of the same future W1.1 work.
