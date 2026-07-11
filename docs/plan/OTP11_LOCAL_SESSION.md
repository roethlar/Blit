# otp-11 — local transfers on the session (design)

**Status**: Active
**Created**: 2026-07-11
**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-11.
**Contract**: `docs/TRANSFER_SESSION.md` (unchanged — this slice adds NO wire
surface; the local byte-carrier is process-local by construction).
**Governs**: implementation proceeds 11a → 11b, one slice per codex loop pass
(D-2026-07-04-1). D-2026-07-05-1 names "separate local orchestration" on the
deletion list; this doc records how it dies without regressing local behavior
or the local perf pins.

## Why this doc

otp-11 deletes the largest surviving old-path block (~4.8k LOC: `orchestrator/`,
`engine/` minus dial, `local_worker.rs`, `auto_tune/`, `change_journal/`) and
re-routes three frontends (CLI, TUI, blit-app) at once, near the otp-13 test
floor (suite 1488 vs ≥1483). Three design decisions were not settled by the
parent plan and are fixed here so implementation is transcription: the local
byte-carrier, the app-facing option/summary surface, and the fate of each
engine-only feature. Slice-level unknowns are delegated to the agent + codex
loop (parent plan §Open questions).

## Current state (verified 2026-07-11, HEAD `d2bd843`)

One chokepoint already exists: CLI (`crates/blit-cli/src/transfers/local.rs:125`)
and TUI (`crates/blit-tui/src/main.rs:4089,4161`) both call
`blit_app::transfers::local::run` (`crates/blit-app/src/transfers/local.rs:36`),
which `spawn_blocking`s into `TransferOrchestrator::execute_local_mirror`
(`crates/blit-core/src/orchestrator/orchestrator.rs:37`) → `TransferEngine`
(`crates/blit-core/src/engine/mod.rs:101`): strategy selection (single-file
shortcut, journal skip, tiny/huge fast paths via `local_worker.rs`, streaming
plan) → `remote/transfer/pipeline.rs` → `FsTransferSink`.

The unified session (`crates/blit-core/src/transfer_session/`) is
role-parameterized over `FrameTransport`; `in_process_pair()`
(`transport.rs:95`) is proven by the 38-test otp-3 role suite
(`crates/blit-core/tests/transfer_session_roles.rs:121-164` is the wiring:
`tokio::join!(run_source(...), run_destination(...))` with
`in_stream_bytes:true`, `data_plane_host:None`, `DestinationTarget::Fixed`).

Load-bearing facts:

- `PreparedPayload::File(FileHeader)` (`remote/transfer/payload.rs:96-98`) is
  "source has it accessible by `src_root.join(relative_path)`; the sink
  performs a (zero-copy when possible) local copy" — and `FsTransferSink`
  (`remote/transfer/sink.rs:113`) implements it via `copy::copy_file`:
  **clonefile on APFS, block-clone on Windows (ReFS/DevDrive),
  copy_file_range/sendfile on Linux**, plus dry-run, sink-level resume,
  compare-mode re-check, and canonical containment. This is the old local
  pipeline's own data path, in SHARED surviving code.
- Therefore pure byte-relay (in-stream frames or loopback TCP) is NOT
  acceptable for local: it would replace a ~instant same-volume clone with a
  full read+write of every byte — the 1 GiB local perf pin would fail by
  orders of magnitude on APFS/ReFS.
- Symlinks: parity holds with no work. The old local path enumerates symlink
  entries (`LocalMirrorOptions.include_symlinks` default true) but every
  consumer drops them (`EnumeratedEntry::into_file_entry` → `None`
  (`enumeration.rs:317`), the strategy fold matches `File` only
  (`engine/strategy.rs:120`), `fs_enum::enumerate_symlinks` is caller-less).
  Neither the old local path nor the session copies symlinks today.
- `copy/` and `engine/dial.rs` are NOT deletable: `remote/transfer/sink.rs:15,468`,
  `diff_planner.rs:146`, `transfer_session/mod.rs:31`, `mirror_planner.rs:7`
  consume `copy/`; `transfer_session/{data_plane.rs:53, mod.rs:785,2200}`
  import the dial via `crate::engine`. Dial re-homes before `engine/` dies.

## Design

### D1 — byte-carrier: destination-side local apply (no bytes on any transport)

The local route runs the UNCHANGED session choreography — hello (exact-match
build identity, trivially satisfied in one process), `SessionOpen` with
`in_stream_bytes:true` (no data plane granted), source streams its manifest,
destination diffs incrementally (one diff owner), mirror = the one delete pass
at SourceDone (`mod.rs:2451`), destination scores and sends the one summary —
with one process-local extension: `DestinationSessionConfig` gains an
`Option<LocalApply>` carrying `src_root` + sink overrides. When set, the
destination applies each needed header itself through the existing payload
planner (`plan_transfer_payloads`) and pipeline into an
`FsTransferSink::new(src_root, dst_root, cfg)` — i.e. `PreparedPayload::File`
/ `TarShard` exactly as the old local pipeline, with the same parallelism and
the same zero-copy primitives. No need is ever sent to the source; the source
streams the manifest, accumulates unreadables, and sends SourceDone.

Why this is one path, not a second one: the parent plan already frames
carriers as transport facts inside the same session ("the gRPC-fallback lane
becomes a byte-carrier option inside the same session... not a separate
transfer path") and D-2026-07-05-3 fixes the receive sink as a
"runtime-selected write-strategy seam" where "strategy selection reads
capability and payload type, never role or initiator". Local apply is that
seam's same-process strategy: selected by a capability only the process that
holds BOTH roots can construct (it is config, not wire — a remote peer
structurally cannot request it; nothing is negotiated). Choreography, diff
ownership, delete rule, refusals, and the summary shape are byte-identical to
the remote session.

`needed_paths`, the diff decisions, `require_complete_scan`/`ScanIncomplete`,
and the both-ends-summary-equality invariant all carry unchanged. The
destination-side apply counts `files_transferred`/`bytes_transferred` from
`SinkOutcome` (the scorer stays the destination).

### D2 — app surface: `LocalMirrorOptions` in, `LocalMirrorSummary` out

`blit_app::transfers::local::run(src, dst, LocalMirrorOptions) →
LocalMirrorSummary` keeps its exact signature; only its body changes (the
`spawn_blocking` + nested-runtime wrapper becomes a direct async call into the
new blit-core entry `run_local_session`). CLI and TUI call sites do not move;
all verb-level pins (single_file_copy.rs 6, local_move_semantics.rs 7,
cli_arg_safety_gates.rs, the TUI move pins) must pass UNCHANGED — including
the three otp-11 regression pins for the move data-loss shape
(`local_move_lands_source_bytes_over_same_size_same_mtime_destination`,
`perform_local_move_lands_source_bytes_over_matching_metadata`,
`perform_local_move_deletes_source_after_copy`): `build_local_options`'
move→IgnoreTimes/Checksum mapping is untouched and now load-bearing (the
session diff DOES skip same-size+same-mtime under SizeMtime).

Summary synthesis (destination side sees everything):
`scanned_files`/`scanned_bytes` = manifest headers folded in the diff loop;
`planned_files` = needed count; `copied_files`/`total_bytes` = sink outcomes;
`deleted_files`/`deleted_dirs` = the delete pass, split (the pass already
loops files then dirs separately — it returns the split so CLI output is
unchanged; the wire `entries_deleted` stays the sum); `unreadable_paths` =
`SourceInstruments.unreadable` (move's R47-F4 source-delete gate keeps its
exact caller-side posture); `duration` measured at the entry; `outcome`:
`SourceEmpty` (scanned==0), `UpToDate` (scanned>0, copied==0, nothing
deleted), else `Transferred` — `JournalSkip` is retired with the journal
(D3). Planner-mix fields (`tar_shard_*`, `raw_bundle_*`, `large_*`) are
filled from the payload plan when cheaply available; `predictor_estimate`
retires with the predictor (D3).

Option mapping: `mirror`+`delete_scope` → `mirror_enabled` +
`MirrorMode::{FilteredSubset,All}`; `compare_mode`+`checksum` →
`resolve_comparison_mode` (existing single home); `filter` → the source's
`FilteredSource` + open's mirror scope; `ignore_existing` → open;
`dry_run` → sink cfg + the delete pass is skipped (deleted counts report the
plan, matching today's `!options.dry_run` execute gate); `null_sink` →
`NullSink` swap at the seam (diff still runs against the real dest);
`resume` → sink-level `FsSinkConfig.resume` (the old local resume mechanism;
the wire block phase stays remote-only — `SessionOpen.resume` unset);
`preserve_times` → sink cfg; `workers` → the apply pipeline's concurrency
argument (same knob the old streaming pipeline took).

### D3 — engine-only features: retired with the orchestration

Per D-2026-07-05-1 ("anything else does not exist") these die with the engine
rather than being rebuilt beside the session; none is on the parent plan's
capability-parity list:

- **Change-journal skip** (`change_journal/`, `engine/journal.rs`): retired.
  Second-run no-op now costs one enumerate+diff (measured in the bench gate).
  `TransferOutcome::JournalSkip` and its CLI line die.
- **Auto-tune + tuning windows** (`auto_tune/`, `engine/tuning.rs`): retired;
  the session's dial + sf-2 shape correction is the one stream policy.
- **Predictor** (`engine/history.rs::update_predictor`, strategy selection
  consumer): retired. `perf_predictor.rs` stays (readers: `blit profile`),
  no new training rows.
- **Perf-history recording**: KEPT, one thin write at the local entry
  (`fast_path:"session"`) so `blit profile` keeps its local data feed —
  `perf_history.rs` already survives for its readers. (Remote session runs
  don't record today; unifying that is out of scope — noted residue.)
- **`--workers` debug limiter**: re-mapped to apply-pipeline concurrency (D2);
  `debug_mode`'s engine debug output dies. `force_tar`: unreachable from the
  CLI already; field retired.

The STATE residue item "derive_local_plan_tuning fold-or-retire" is absorbed:
retired here with `auto_tune/`.

## Staging

### otp-11a — the local session route (code slice)

1. blit-core: `run_local_session(src_root, dst_root, LocalMirrorOptions) →
   LocalMirrorSummary` (new `transfer_session/local.rs`): `in_process_pair()`,
   `tokio::join!` of the two role drivers, `LocalApply` destination extension
   (D1), option mapping + summary synthesis (D2), perf-history write (D3).
2. blit-app: `transfers/local.rs::run` re-pointed (D2). CLI/TUI untouched.
3. Tests: port the orchestrator behavior pins (16) and the
   `local_transfers.rs`/`predictor_streaming.rs`/`engine_streaming_plan.rs`
   pins (10) to session-local equivalents in a new
   `crates/blit-core/tests/local_session.rs` — tag pins become behavior pins
   (e.g. `up_to_date_second_run_records_no_work` → second run scans N,
   copies 0, outcome UpToDate). All verb-level local e2es pass unchanged.
   Old orchestration stays in-tree (scaffolding, per the plan's transitional
   rule) but production-caller-less.
4. Bench gate (perf pins): A/B on this machine (APFS) with
   `scripts/bench_local_mirror.sh` (SIZE_MB=1024 single-huge + default tree)
   + a 10k-small-file tree + no-op mirror second run, old entry vs session
   entry, ≥3 runs, medians; unified ≤ old + 10% per cell (converge-up
   locally). Evidence committed under `docs/bench/otp11-local-2026-07-11/`.
   A failed cell blocks 11b until fixed.

### otp-11b — the deletion (cutover slice)

1. Re-home `engine/dial.rs` → `src/dial.rs` (verbatim; 15 tests carry);
   re-home `LocalMirrorOptions`/`LocalCompareMode`/`LocalMirrorDeleteScope`/
   `LocalMirrorSummary`/`TransferOutcome` → `transfer_session/local.rs`
   (frontends re-import; `orchestrator::` re-export path dies).
2. Delete: `orchestrator/`, `engine/` (remainder), `local_worker.rs`,
   `auto_tune/`, `change_journal/`, `copy/parallel.rs` + `copy/stats.rs`,
   `lib.rs::CopyConfig`.
3. Deferred dead-code sweep (otp-10c-2 F2): `manifest::compare_manifests` +
   `ManifestDiff` + `FileComparison` + their test block;
   `CompareOptions.include_deletions` (write-only false);
   the doc comment at `transfer_session/mod.rs:3372`. `header_transfer_status`,
   `compare_file`, `CompareMode`, `CompareOptions`, `FileStatus` SURVIVE
   (live via the session diff, `mod.rs:3431`).
4. Stranded-by-this-slice dead code: `diff_planner::plan_local_mirror` +
   `LocalDiffInputs` (+4 in-file tests), `pipeline::execute_sink_pipeline_streaming`
   (the session uses `_elastic`). Pre-existing unrelated dead code
   (`delete.rs`, `zero_copy.rs::sendfile_chunk`) is NOT swept here — noted in
   Known gaps for a separate sweep.
5. Deletion proof in the otp-10c-2 format (file-by-file, DELETED WHOLE with
   line/test counts, relocations called out, grep proof over `crates/` +
   live docs), completing the acceptance criteria's "separate local
   orchestration path" line.
6. Retirement accounting in the otp-10c-2 categories, summing exactly;
   ≥1483 floor re-checked with real pins.

## Test-floor arithmetic (pre-slice plan)

Baseline 1488/0 (verified this session). Retire-if-nothing-ported: 16
orchestrator unit + 19 engine-non-dial unit + 6 auto_tune + 10 blit-core
integration + ~15 compare_manifests block + 4 plan_local_mirror = ~70.
Ports/conversions in 11a cover the 16+10 as session-local pins (≥26 back),
manifest's live-half tests stay, and the new local-session e2e pins (dry-run,
null, mirror scopes, move gate, single-file, unreadable, delete-scope split)
add on top. Target: ≥1483 after 11b with margin, met by real pins, not a
re-baseline. Windows parity (`scripts/windows/run-blit-tests.ps1`) required —
`copy/windows.rs`'s 6 cfg(windows) tests ride the shared `copy/` module,
which does not move.

## Known gaps / owner-visible changes

- Journal fast-path skip, auto-tune, and predictor training retire with the
  engine (D3) — behavior change is limited to second-run wall time on very
  large unchanged trees (bench-gated) and `blit profile`'s predictor columns
  going stale for local runs.
- `TransferOutcome::JournalSkip` CLI line disappears; `--verbose` planner-mix
  block reflects the payload plan (raw-bundle counters may read 0 if the
  session plan doesn't use that task kind locally).
- The tracked `.claude/worktrees/vigilant-mayer/**` snapshot still holds
  old-path code — its removal stays the owner-gated 725aa07 question; the
  11b deletion proof is scoped to the workspace source tree (otp-10c-2 F6
  posture).
