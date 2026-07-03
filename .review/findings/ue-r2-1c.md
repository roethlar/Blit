# ue-r2-1c: TransferEngine shell + TransferOrchestrator as local adapter

**Slice**: ue-r2-1c — third slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: In progress (this doc doubles as the slice implementation plan)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: (filled as they land)

## What

Introduce `TransferEngine` (new `crates/blit-core/src/engine/` module) and
convert `TransferOrchestrator` into the local adapter, per REV4 Design §1
(engine type ratified at this slice: new engine + adapter, not an in-place
rename). Local fast paths become engine-owned strategies with common
accounting: `journal_no_work`, `no_work`, `tiny_manifest`,
`single_huge_file`, the single-file shortcut, and the streaming pipeline.
The single intentional behavior addition: the single-file shortcut gains
the perf-history/predictor accounting it lacks today (REV4 Design §2).

## Approach (move plan)

Everything is within blit-core, so the engine/adapter seam at this slice
is module organization + ownership, not dependency inversion (traits
arrive when push/pull converge at `ue-r2-1f`/`1g` and only where needed).

- **`crates/blit-core/src/engine/`** (new; re-exported from `lib.rs`):
  - `mod.rs` — `TransferEngine` + `EngineRequest { src_root, dest_root,
    source: Arc<dyn TransferSource>, sink: Arc<dyn TransferSink>,
    options }` + `execute()`: owns strategy selection order (single-file
    → journal probe → fast-path walk → streaming), dispatch, and the
    streaming leg (tuning window → scan/collect → plan → pipeline →
    mirror deletions → journal checkpoints → history/predictor).
  - `strategy.rs` — `FastPathDecision`/`FastPathOutcome`/
    `maybe_select_fast_path` moved whole from `orchestrator/fast_path.rs`
    (tests move with it).
  - `single_file.rs` — `execute_single_file_copy` moved from
    `orchestrator.rs:1138`, **plus new accounting**: every return path
    records perf history (tag `single_file`) and updates the predictor
    (skipped for null_sink, same rule as streaming at
    `orchestrator.rs:863`). Records with `tar_shard_tasks == 0` are
    already excluded from the tuning window (`orchestrator.rs:72`), so
    the new tag cannot contaminate auto-tuning.
  - `options.rs`, `summary.rs`, `history.rs` — moved from
    `orchestrator/` unchanged (names kept; generalizing the option type
    is 1f/1g work). `LocalCompareMode` gains two small resolvers
    (`resolve_comparison_mode`, `resolve_compare_snapshot`) replacing
    the three duplicated match blocks (`orchestrator.rs:467/:520/:1159`).
  - `tuning.rs` — `select_tuning_window`/`select_tuning_window_from_history`
    + `TUNING_WINDOW_SIZE` + their 12 tests, moved from `orchestrator.rs`.
  - `mirror.rs` — `apply_mirror_deletions`; `journal.rs` —
    journal probe + `persist_journal_checkpoints` + `log_probe`.
- **`crates/blit-core/src/orchestrator/`** shrinks to the adapter:
  `TransferOrchestrator::{new, default}`, the sync runtime wrapper
  (unchanged), and an async method that checks preconditions (src
  exists, create dest parent), constructs local `FsTransferSource`/
  `FilteredSource` + `FsTransferSink`/`NullSink` (translation of
  compare-mode via the new resolver), builds the `EngineRequest`, and
  calls `TransferEngine::execute`. `orchestrator/mod.rs` keeps the
  existing six public names via `pub use crate::engine::...` so every
  external caller (blit-app `transfers/local.rs:36-57`, blit-cli,
  blit-tui, tests) compiles unchanged.
- Sink construction moves ahead of planning (adapter builds it up
  front). `FsTransferSink::new` is pure state (paths + config), so
  constructing it on runs that end in a fast path is behavior-neutral.

## Behavior pins added BEFORE the move (commit 1)

The test-inventory pass found these currently unpinned; each is cheap
and pins a strategy this slice relocates:

- empty source dir → `FastPathDecision::NoWork{examined:0}` →
  `TransferOutcome::SourceEmpty`.
- all-up-to-date second run (dir, skip_unchanged) →
  `NoWork{examined>0}` → `UpToDate`, perf-history tag `no_work`.
- (with commit 3) single-file run records history tag `single_file` —
  the new accounting's own guard.

Not pinnable here: `single_huge_file` (needs a ≥1 GiB file) and
`journal_no_work` (needs journal-capable FS state) — unchanged code
moves, existing Known gaps.

## Files changed

(filled as commits land)

## Tests

Baseline entering the slice: 1391 passed / 0 failed / 2 ignored. Count
must not drop; expected to rise (new pins + single-file accounting
test).

## Known gaps

- `single_huge_file` and `journal_no_work` strategies move without new
  coverage (pre-existing gap; needs 1 GiB fixtures / journal-capable FS).
- The engine's option/summary types keep their local names
  (`LocalMirrorOptions`/`LocalMirrorSummary`) until push/pull converge
  (`ue-r2-1f`/`1g`) — renaming now would churn every caller twice.
- Dial creation, payload-queue ownership, and progress/telemetry wiring
  stay where they are until `ue-r2-1d`/`1e` (engine owns them per REV4,
  arriving with those slices).
