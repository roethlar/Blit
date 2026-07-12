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
- Symlinks: parity holds for every REACHABLE option value (codex design
  F6 scoped this claim). With the production defaults the old local
  path enumerates symlink entries but every consumer drops them
  (`EnumeratedEntry::into_file_entry` → `None` (`enumeration.rs:317`),
  the strategy fold matches `File` only (`engine/strategy.rs:120`),
  `fs_enum::enumerate_symlinks` is caller-less) — neither path copies
  symlinks. The divergent axes (`preserve_symlinks=false` made the old
  walker follow links; `skip_unchanged=false` forced copies) are
  unreachable from any production caller (the CLI never sets them; the
  TUI transfer path uses defaults; `screens/f4.rs` is the browse
  screen) and retire with the options re-home at 11b.
- `copy/` and `engine/dial.rs` are NOT deletable: `remote/transfer/sink.rs:15,468`,
  `diff_planner.rs:146`, `transfer_session/mod.rs:31`, `mirror_planner.rs:7`
  consume `copy/`; `transfer_session/{data_plane.rs:53, mod.rs:785,2200}`
  import the dial via `crate::engine`. Dial re-homes before `engine/` dies.

## Design

### D1 — byte-carrier: destination-side local apply (no bytes on any transport)

The local route runs the session with one precisely-bounded carrier
delta (stated explicitly — codex design F1 rejected this section's
earlier "unchanged choreography" overclaim). Shared,
function-for-function: hello (exact-match build identity, trivially
satisfied in one process), `SessionOpen` validation and refusals with
`in_stream_bytes:true` (no data plane granted), source manifest
streaming with the same unreadable accumulation and
`ManifestComplete{scan_complete}`, the destination-owned diff (the
same `destination_needs` verdicts, `granted` dedup, `needed_paths`
record), `NeedComplete`, the mirror scan-complete guard (plus, on this
carrier, the SourceDone apply-time unreadable guard — R46-F2's exact
old posture, codex otp-11a F4), the one delete pass at SourceDone
(`mod.rs:2451`), the destination-computed summary and its final
exchange. Diff batching is session-uniform: BOTH carriers diff in
`DEST_DIFF_CHUNK` batches (the wire session has granted needs this way
since otp-4), so the local route inherits the session's start latency,
not the old engine's 3-header eagerness — the streaming-overlap
property (first work lands before a >chunk enumeration completes)
ports as a pin at 11b (codex otp-11a F2 adjudication). The delta: under the local carrier the
need-grant/payload phase collapses into the destination —
`DestinationSessionConfig` gains an `Option<LocalApply>` (`src_root` +
sink overrides) under which needed headers are planned
(`plan_transfer_payloads`) and applied in-process into an
`FsTransferSink::new(src_root, dst_root, cfg)` — i.e.
`PreparedPayload::File`/`TarShard` exactly as the old local pipeline,
same parallelism, same zero-copy primitives. No NeedBatch is sent, the
source serves no payloads, and nothing enters `outstanding` (a payload
record arriving anyway still violates). The source streams the
manifest, accumulates unreadables, and sends SourceDone.

Why this is one path, not a second one: the parent plan already frames
carriers as transport facts inside the same session ("the gRPC-fallback lane
becomes a byte-carrier option inside the same session... not a separate
transfer path") and D-2026-07-05-3 fixes the receive sink as a
"runtime-selected write-strategy seam" where "strategy selection reads
capability and payload type, never role or initiator". Local apply is that
seam's same-process strategy: selected by a capability only the process that
holds BOTH roots can construct (it is config, not wire — a remote peer
structurally cannot request it; nothing is negotiated). The owner's
invariant (direction/initiator/verb never select code) is preserved in
its own terms: local↔local has no direction/initiator asymmetry, and
every semantic layer — compare, planner, sink, delete rule, refusals,
summary — is the same code every remote session runs. The rejected
alternative (relaying payload bytes through frames or loopback TCP)
keeps the payload phase textually identical but loses same-volume
clonefile/block-clone by orders of magnitude — failing this plan's own
perf gate. At 11b, `docs/TRANSFER_SESSION.md` gains a short "Local
carrier" note so the contract doc names the delta explicitly.

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
exact caller-side posture); `duration` measured at the entry; `outcome`
replicates the OLD strategy gate's reachability (codex otp-11a F8
settled impl-vs-doc in favor of parity): `SourceEmpty`/`UpToDate` only
on runs the old fast paths could reach (non-mirror, non-checksum,
non-force_tar, default SizeMtime compare — mirror/checksum/SizeOnly
runs always reported `Transferred` on the old streaming leg, and still
do, so mirror deletion lines never hide behind an early-return
outcome); `JournalSkip` is retired with the journal (D3). Planner-mix
fields (`tar_shard_*`, `raw_bundle_*`, `large_*`) are filled from the
payload plan when cheaply available; `predictor_estimate` retires with
the predictor (D3).

Option mapping: `mirror`+`delete_scope` → `mirror_enabled` +
`MirrorMode::{FilteredSubset,All}`; `compare_mode`+`checksum` →
`resolve_comparison_mode` (existing single home); `filter` → the source's
`FilteredSource` + open's mirror scope; `ignore_existing` → open;
`dry_run` → sink cfg + the delete pass is skipped (deleted counts report the
plan, matching today's `!options.dry_run` execute gate); `null_sink` →
`NullSink` swap at the seam (diff still runs against the real dest);
`resume` → sink-level `FsSinkConfig.resume` — the local carrier's
block phase (codex design F5): the same resume semantic (hash the
partial, rewrite only differing blocks, full-file fallback) executed
by the shared `resume_copy_file` primitive without serializing block
records that the same process would immediately deserialize;
`SessionOpen.resume` stays unset (running the wire block phase over
in-process frames would relay every changed block's bytes — the relay
this carrier exists to avoid). Pinned by a local `--resume` behavior
test;
`preserve_times` → sink cfg; `workers` → the apply pipeline's concurrency
argument (same knob the old streaming pipeline took).

### D3 — engine-only features: retired with the orchestration

Per D-2026-07-05-1 ("anything else does not exist") these die with the engine
rather than being rebuilt beside the session; none is on the parent plan's
capability-parity list:

- **Change-journal skip** (`change_journal/`, `engine/journal.rs`):
  retired — and the retirement removes a PROVEN data-loss bug, not a
  feature (2026-07-12; supersedes the earlier codex-design-F8
  reading). The probe's macOS/Linux `NoChanges` verdict decays to
  ROOT-dir mtime equality whenever the global event counter moved
  (always, between runs), and deep modifications never touch the root
  dir's mtime — reproduced against the pre-otp-11 binary: warm
  journal + modify `src/sub/deep.txt` → "Up to date", destination
  stale (transcript: `docs/bench/otp11-local-2026-07-11/README.md`).
  The honest sound-vs-sound no-op baseline is the old path's full
  pass, certified by 5-run medians with its journal cache cleared per
  run (507 ms on 10k files); the session's 226 ms beats it 2.2× — the
  no-op gate cell PASSES against that baseline. Pinned on the session
  route by `deep_modification_after_warm_runs_syncs`.
  `TransferOutcome::JournalSkip` and its CLI line die.
  **Filed future capability (design sketch, own slice set after this
  plan ships)**: journal-assisted no-op done SOUNDLY as a negotiated
  SESSION phase — each end probes ITS root with real journal replay
  (Windows: USN range read since the stored USN; macOS: FSEvents
  historical replay since the stored event id, honoring
  must-rescan/dropped-event flags; Linux: unsupported), memos pair
  via a sync nonce recorded by both ends at the last completed
  session, any doubt fails open to the full session. One
  implementation, both roles, both carriers — remote no-ops skip
  manifest streaming too. New wire surface (open/accept fields) +
  platform work: follows the wire-contract discipline as its own
  reviewed slice set.
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
   `LocalDiffInputs` (+4 in-file tests). NOT
   `pipeline::execute_sink_pipeline_streaming` — 11a made it the local
   apply pipeline, a live production caller; it and its tests stay
   (codex design F9, reversed by implementation). Pre-existing
   unrelated dead code (`delete.rs`, `zero_copy.rs::sendfile_chunk`)
   is NOT swept here — noted in Known gaps for a separate sweep.
5. Sink defense-layer alignment decision (codex design F3): the
   File-payload write re-check (`file_needs_copy_with_checksum_type`:
   size → first/last-MiB partial hash → mtime tolerance) can skip a
   file the session diff flagged, counting it `files_written` — the
   OLD local pipeline's exact behavior and accounting, preserved by
   11a. 11b settles whether the re-check aligns with
   `header_transfer_status` (the one compare owner) or is retired for
   session-driven writes; either way local and remote counting stop
   diverging inside the tolerance window.
6. Deletion proof in the otp-10c-2 format (file-by-file, DELETED WHOLE with
   line/test counts, relocations called out, grep proof over `crates/` +
   live docs), completing the acceptance criteria's "separate local
   orchestration path" line.
7. Retirement accounting in the otp-10c-2 categories, summing exactly;
   ≥1483 floor re-checked with real pins.

## Test-floor arithmetic (amended per codex design F10)

Post-11a suite: 1513/0 (baseline 1488 + the 22 landed 11a pins + the
2 fix-round pins + the journal-hole regression pin). 11b retirements,
exact: 16 orchestrator unit + 19 engine-non-dial unit (strategy 3,
streaming_plan 2, tuning 12, history 2) + 6 auto_tune + 10 blit-core
integration (local_transfers 7, predictor_streaming 2,
engine_streaming_plan 1) + 16 manifest `compare_manifests` block (ALL
16 drive `compare_manifests`; none pins `header_transfer_status`
directly — the earlier "live-half tests stay" claim was wrong) +
4 `plan_local_mirror` = **71** → 1442 without replacements; the
end-of-plan ≥1483 floor needs **≈ +41 real pins** by otp-13. Named
closure sources for 11b: direct `header_transfer_status` unit ports
(~8 — the live compare owner deserves direct pins), local `--resume`
behavior pins (2), un-consolidating the 11a orchestrator ports back
toward 1:1 (+4–6), `record_local_history` contract ports of the
history.rs R44-F1 tests (2), `mirror_delete_pass` execute/plan unit
pins (2), a session streaming-overlap port of
`first_work_lands_before_enumeration_completes` (1), plus new
session-edge pins (cancel-drop, empty-source mirror full-delete,
nested-ENOTEMPTY delete-scope, single-file×{dry-run,force} shapes).
Whatever residual remains after 11b is carried as an explicit deficit
line to the otp-13 checklist walk — the floor is met by real pins,
never a re-baseline. `dial.rs` (15), `copy/file_copy` (7 + 6
cfg(windows)), and the pipeline-streaming tests are relocations or
keeps, not retirements. Windows parity
(`scripts/windows/run-blit-tests.ps1`) required.

## Known gaps / owner-visible changes

- Auto-tune and predictor training retire with the engine (D3);
  `blit profile`'s predictor columns go stale for local runs. The
  journal skip's retirement is a RELIABILITY FIX (proven silent
  data loss on deep modifications — D3); repeated no-op wall time on
  unchanged trees is 2.8× BETTER than the old sound pass and O(N)
  worse than the old unsound skip, with the sound O(changes) tier
  filed as a future session capability (D3).
- `TransferOutcome::JournalSkip` CLI line disappears; `--verbose` planner-mix
  block reflects the payload plan (raw-bundle counters may read 0 if the
  session plan doesn't use that task kind locally).
- The tracked `.claude/worktrees/vigilant-mayer/**` snapshot still holds
  old-path code — its removal stays the owner-gated 725aa07 question; the
  11b deletion proof is scoped to the workspace source tree (otp-10c-2 F6
  posture).
