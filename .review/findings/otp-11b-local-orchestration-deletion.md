# otp-11b — the local orchestration deletion (the last old path out of the tree)

**Slice**: ONE_TRANSFER_PATH otp-11, stage 11b per
`docs/plan/OTP11_LOCAL_SESSION.md`; completes the acceptance
criteria's deletion-proof line for "the separate local orchestration
path" (D-2026-07-05-1: "anything else does not exist"). Includes the
otp-10c-2 F2 deferred `compare_manifests` sweep. Net −6,197 lines
(+1,336 for re-homes, converted pins, and the floor restoration).

## What

The engine-era local orchestration no longer exists: every local
transfer runs `run_local_session` (otp-11a's route) with no second
implementation left in the tree to reach. The one remaining engine
piece the session shares — the dial — is re-homed to `src/dial.rs`;
the app-facing option/summary types re-homed into
`transfer_session/local.rs`. The unsound change-journal skip
(data-loss repro: `docs/bench/otp11-local-2026-07-11/README.md`) is
deleted with the engine.

## Deletion proof — file by file

**blit-core (the local orchestration):**

- `src/orchestrator/` — DELETED WHOLE (`orchestrator.rs` 802 lines:
  `TransferOrchestrator`, `execute_local_mirror[_async]`; `mod.rs` 7:
  the `blit_core::orchestrator::` re-export surface; 16 unit tests).
- `src/engine/` — DELETED WHOLE except the relocation below
  (`mod.rs` 815: `TransferEngine`/`EngineRequest`, strategy
  selection, journal probe, streaming leg; `strategy.rs` 280 (3
  tests); `streaming_plan.rs` 356 (2); `single_file.rs` 210;
  `mirror.rs` 198 (`apply_mirror_deletions` — the session's
  `mirror_delete_pass` is the one delete rule, now returning the same
  (files, dirs) split the old pass reported); `history.rs` 210 (2);
  `journal.rs` 72; `tuning.rs` 592 (12); `options.rs` 157 +
  `summary.rs` 121 — types re-homed, see below).
- `src/engine/dial.rs` → **RELOCATED VERBATIM** → `src/dial.rs`
  (1,051 lines; 17 tests carry (count corrected per codex B6);
  blob-identical old/new per the reviewer; consumers re-pointed:
  `transfer_session/{mod,data_plane}.rs` + doc-comment paths).
- `src/local_worker.rs` — DELETED WHOLE (139; `copy_paths_blocking`,
  `copy_large_blocking`).
- `src/auto_tune/` — DELETED WHOLE (273; 6 tests;
  `derive_local_plan_tuning` — absorbs the STATE residue item
  "derive_local_plan_tuning fold-or-retire": retired).
- `src/change_journal/` — DELETED WHOLE (512: snapshot/tracker/
  types/util; the UNSOUND journal skip — its `NoChanges` decayed to
  root-dir metadata a deep write never touches; silent data loss
  reproduced against `d2bd843` and pinned on the session route by
  `deep_modification_after_warm_runs_syncs`). The macOS-only
  `objc2-core-services` dependency died with it (Cargo.toml).
- `src/copy/parallel.rs` (51) + `src/copy/stats.rs` (17) — DELETED
  (caller-less `parallel_copy_files`/`CopyStats`; `copy/mod.rs`
  re-exports dropped). The REST of `copy/` survives untouched —
  shared by the sink, diff, mirror planner, and session.
- `lib.rs::CopyConfig` — DELETED (its consumers were the engine and
  local_worker).
- `TransferOutcome::JournalSkip` + `PredictorEstimate` +
  `LocalMirrorSummary.predictor_estimate` — RETIRED with the journal
  and the predictor's training loop (no frontend consumer existed for
  the estimate; the CLI's JournalSkip print arms deleted).
- Types **RE-HOMED** → `transfer_session/local.rs`:
  `LocalMirrorOptions` (minus the unreachable engine-era axes
  `force_tar`/`preserve_symlinks`/`include_symlinks`/`skip_unchanged`
  — design doc F6: no production caller ever set them; the persisted
  perf-history `OptionSnapshot` keeps its schema, fed the historical
  defaults), `LocalCompareMode` (+ both resolve mappings),
  `LocalMirrorDeleteScope`, `LocalMirrorSummary`, `TransferOutcome`.
  Frontends re-import via `blit_core::transfer_session::` (the
  `orchestrator::` path no longer exists).

**The otp-10c-2 F2 deferred sweep (`src/manifest.rs`):**

- `compare_manifests` + `ManifestDiff` + `FileComparison` +
  `CompareOptions.include_deletions` — DELETED (caller-less since the
  PullSync deletion; deletions are the otp-6b mirror pass, never a
  per-entry flag; the last `include_deletions: false` write in
  `destination_session` removed). **SURVIVE deliberately**:
  `header_transfer_status` (the one compare owner, called by the
  session diff for every manifest entry), `compare_file`,
  `CompareMode` + its `From<ComparisonMode>`, `CompareOptions`,
  `FileStatus`. The `destination_needs` doc comment retyped.

**Stranded-by-the-engine dead code
(`src/remote/transfer/diff_planner.rs`):**

- `LocalDiffInputs` + `plan_local_mirror` + `filter_unchanged` +
  `local_needs_copy` — DELETED (only production caller was the
  engine's streaming plan). `plan_push_payloads` SURVIVES (the
  session source's two call sites).

**blit-core integration tests:** `tests/local_transfers.rs` (7),
`tests/predictor_streaming.rs` (2), `tests/engine_streaming_plan.rs`
(1) — DELETED; accounting below.

**Frontends:** import re-points only (CLI/blit-app/blit-tui), the two
CLI JournalSkip print arms deleted, the UpToDate line's "examined"
count now reads `scanned_files` (it printed the planned count), the
`--workers` debug banner retyped (the auto-tuning it named is gone),
comment sweep to zero `orchestrator` references.

**Contract doc:** `docs/TRANSFER_SESSION.md` §Transport selection's
"Local (in-process)" bullet expanded to state the LOCAL byte-carrier
precisely (the design-round F1 commitment): the process-local
`LocalApply` delta, what stays shared verbatim, the
`PROTOCOL_VIOLATION` posture for stray records, and the sink-level
resume equivalence.

**Grep proof** (run at the slice head): the alternation
`TransferOrchestrator|execute_local_mirror|copy_paths_blocking|copy_large_blocking|derive_local_plan_tuning|ChangeTracker|JournalSkip|compare_manifests|ManifestDiff|plan_local_mirror|LocalDiffInputs|CopyConfig|parallel_copy_files|CopyStats|local_worker|auto_tune|change_journal`
over `crates/` + `proto/` returns only dated otp-11b deletion notes
(the allowed residue class) — zero live references. The tracked
`.claude/worktrees/vigilant-mayer/**` snapshot still holds the old
tree (owner-gated `725aa07` question, otp-10c-2 F6 posture): the
proof is scoped to the workspace source tree.

## Tests (suite 1513 → 1484, exactly the retirements below; gate green: fmt, clippy -D warnings, cargo test --workspace 1484/0 (2 ignored), the pre-existing blit_utils daemon-spawn flake excluded from the main run and green in isolation)

**Died inside deleted modules (41):** orchestrator 16, engine
strategy 3, streaming_plan 2, tuning 12, history 2, auto_tune 6.
(`dial.rs`'s 15 relocated intact; `change_journal`/`local_worker`/
`mirror.rs`/`journal.rs`/`single_file.rs`/`options.rs`/`summary.rs`
had none.)

**Deleted integration files (10):** `local_transfers.rs` 7 (all
behavior re-pinned on the session route in 11a's
`tests/local_session.rs` — no-op/up-to-date, source-empty,
single-file, cross-chunk, nested-dest, tree copies, session history
rows), `predictor_streaming.rs` 2 (capability retired with the
predictor's training loop — its strategy consumer no longer exists),
`engine_streaming_plan.rs` 1 (converted: the streaming-overlap
property is `first_apply_lands_before_enumeration_completes`, a
gated-source port on the session route).

**Retired with the swept surface (5):** manifest aggregate-shape 3
(`test_empty_manifests`, `test_deletions_for_mirror`,
`test_mixed_scenario` — the aggregate died; deletion semantics are
pinned by the session mirror pins incl. the new empty-source and
nested-split pins), diff_planner 2
(`ignore_existing_skips_existing_regardless_of_mode` — superseded by
manifest's `ignore_existing_*` pins + the session
`ignore_existing` e2es; `plan_local_mirror_skip_unchanged_on` —
superseded by `up_to_date_second_run_copies_nothing`).

**Converted in place, not dropped (25):** manifest 13 (direct
`header_transfer_status` pins, 1:1 from the compare_manifests block),
diff_planner 12 (9 direct `file_needs_copy_with_mode` pins — the
sink defense layer's decision tree — + 3 direct
`plan_transfer_payloads` pins).

**New pins (+27):** 15 session-route e2es in `local_session.rs`
(empty-source mirror full-delete + split, nested extraneous split,
ignore-times tree, checksum content-change + content-equal cells,
ignore-existing tree, include-filter mirror scope, All-scope nested
excluded dirs, dry-run single-file, nested parent creation + force
re-copy, resume fresh-dest fallback, unreadable subdir copy-continues,
size-only mismatch counterpart, planner-mix stats, scanned-bytes
accounting), 7 unit pins in `transfer_session/local.rs` (3
`build_local_record` contract pins — scanned features, bucket
counters, null/dry-run lanes (split from the writer for testability,
the R44-F1 rationale); 2 resolve-mapping pins; the
`DestSubtreeExcludedSource` stream-filter pin; the streaming-overlap
port), 2 `mirror_delete_pass` unit pins (plan-only counts nothing
deleted; (files, dirs) split), 2 manifest arm pins (ignore_existing
wins over Checksum; absent-target New in every mode), 1 sink file-root
pin (`file_root_payload_copies_root_to_root` — the 11a ENOTDIR fix at
unit level).

**Arithmetic (corrected per codex B5):** 1513 − 41 (died in modules)
− 10 (deleted integration files) − 5 (retired with the surface — the
net shrink of the two converted blocks: manifest 16→13, diff_planner
14→12) + 27 (new pins; `planner_keeps_every_header` and the two
manifest arm pins are NEW pins in this bucket, not conversions) =
**1484**. The 25 conversions replaced old tests in place and do not
appear in the delta. The otp-13 floor (≥1483) is met at the deletion
slice itself, by real pins, margin +1.

**Guard proof this slice:** `file_needs_copy_with_mode` SizeOnly arm
mutated to always-copy → `size_only_ignores_mtime_when_sizes_match`
FAILS → restored → passes (the converted pins genuinely guard the
live decision tree). The 11a proofs (dest-subtree bypass, dry-run
execute, split swap, apply-time mirror guard) all still stand on the
same code.

## Known gaps

- Windows parity: `copy/windows.rs`'s 6 cfg(windows) tests ride the
  unmoved `copy/` module; `scripts/windows/run-blit-tests.ps1` on the
  owner's machine + windows-latest CI ride the next push (the
  standing Blocked item).
- Pre-existing dead code NOT swept here (out of scope, unchanged):
  `delete.rs`, `zero_copy.rs::sendfile_chunk`.
- `blit profile`'s predictor columns go stale for local runs (the
  training loop retired); history rows keep flowing
  (`fast_path: "session"` / `"null_sink"` lanes).
- The floor margin is +1; otp-12's acceptance pins add on top.

## Reviewer comments

(appended after the codex round)
