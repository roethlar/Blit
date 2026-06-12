# w9-2-revive-root-tests — relocate dead workspace-root tests/ into blit-core/tests

**Branch**: `master` (owner-authorized branchless session 2026-06-11)
**Commit**: `461525d`
**Source finding**: tests-dead-workspace-root-test-suite (reviewer: high) — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

The root `tests/` directory was never compiled (`Cargo.toml` is a virtual
workspace with no `[package]`), so its six files provided false coverage —
including the *only* semantic tests for `MirrorPlanner`. Relocated five
files into `crates/blit-core/tests/`, deleted `connection.rs`, fixed
AGENTS.md §4's stale description.

## Approach

- `git mv` → `crates/blit-core/tests/`: mirror_planner_tests (15 tests),
  enumeration_tests (2), checksum_partial (2), local_transfers (2),
  predictor_streaming (2). All deps already in blit-core
  (tempfile dev-dep; eyre/filetime/tokio main deps).
- **Deleted** `connection.rs` per the slice spec — it required an
  externally running blitd on a hardcoded port and could never pass
  unattended.
- Drift fixed to revive:
  - `predictor_streaming`: `PerformancePredictor::for_tests` no longer
    exists → `load()` under the same config-dir override guard
    local_transfers uses; `OptionSnapshot` needed the R59 `compare_mode`
    field.
  - `local_transfers`: the "streaming path" test seeded 32 files, below
    today's fast-path tiny budget (`TINY_FILE_LIMIT = 256`) — its
    assertion failed honestly on revival. Now 300 files.
  - Both config-dir-mutating files serialize their tests via a
    poison-tolerant `static Mutex` — they mutate process-global
    config-dir state and raced when run on parallel threads.
  - clippy `bool_assert_comparison` in checksum_partial; rustfmt pass on
    all five (they predate enforcement).
- AGENTS.md §4: replaced "`tests/` — workspace-level integration tests"
  with the per-crate layout note (root `tests/` would never compile).

## Files changed

- `tests/` → removed entirely (5 moved, 1 deleted)
- `crates/blit-core/tests/` ← 5 files (one rewritten: predictor_streaming)
- `AGENTS.md` (§4 project-map entry only)

## Tests added

+24 to the running suite (1341 → 1365); the 2 dead tests in connection.rs
are gone but were never counted (never compiled). The revived
mirror_planner_tests file carries 15 tests, not the 16 the finding
estimated.

## Known gaps

- The revived tests assert what the originals asserted (modulo the drift
  fixes); no new coverage was authored. The predictor tests in particular
  only assert the copy succeeds (copied_files == 1), not which path the
  predictor chose — strengthening them would be new w9-6-class work.
- AGENTS.md §4 still carries the ghost identifiers (`transfer_engine`,
  `PLAN_OPTIONS`) — that is W10.1's slice, deliberately untouched here
  (same file, different lines; w10 must sequence after this lands or
  refresh against it).
