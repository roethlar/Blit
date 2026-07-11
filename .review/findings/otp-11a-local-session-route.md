# otp-11a — local transfers ride the session (the local route)

**Slice**: ONE_TRANSFER_PATH otp-11, stage 11a per
`docs/plan/OTP11_LOCAL_SESSION.md` (D1–D3). Deletion is 11b.

## What

Local (non-remote) transfers now run as one unified `TransferSession`:
`blit_app::transfers::local::run` — the single chokepoint both the CLI
verbs and the TUI F4 forms already call — rides the new
`blit_core::transfer_session::run_local_session` instead of
`TransferOrchestrator::execute_local_mirror`. Both role drivers join
over `in_process_pair()` (the otp-3 wiring), with the LOCAL
byte-carrier: a process-local `LocalApply` extension on
`DestinationSessionConfig` under which the destination applies each
needed file itself through the shared payload planner and
`FsTransferSink` (clonefile / block-clone / copy_file_range) — no
payload byte rides any transport, and no wire peer can select the
carrier (it has no wire representation; its fields are crate-private
and only `run_local_session` constructs it). The old orchestration
stays in-tree as production-caller-less scaffolding until 11b deletes
it.

## Approach

- **One choreography, one new carrier seam.** The session runs
  unchanged: hello (exact-match build identity — trivially satisfied
  in one process), manifest streaming, destination-owned diff (the
  same `destination_needs` verdicts, `granted` dedup, and
  `needed_paths` record via the new `diff_chunk_and_apply_local`
  twin), `NeedComplete`, the one mirror delete pass at `SourceDone`,
  destination-computed summary. Differences are confined to the
  carrier: needed headers are planned (`plan_transfer_payloads`) and
  queued onto `execute_sink_pipeline_streaming` — the old local
  pipeline itself, retained with a live caller — and joined at
  `SourceDone` with the data-plane receive's discipline. No frames are
  sent for needs; a payload record arriving anyway still violates
  (the `outstanding` set stays empty).
- **Option/summary surface unchanged (D2).** `LocalMirrorOptions` in,
  `LocalMirrorSummary` out; CLI/TUI call sites and all verb-level pins
  untouched. Summary synthesis destination-side: scanned counts folded
  in the diff loop, planned = needed_paths, copied/bytes from
  `SinkOutcome`, deleted split from the mirror pass (now returning
  `(files, dirs)`; wire summaries carry the sum), unreadable = one
  merged accumulator (scan side + apply-side availability), outcome
  classified by the old strategy gate's shape (SourceEmpty / UpToDate
  only on default-compare non-mirror runs — the shapes that could hit
  the old fast paths).
- **Feature parity by construction**: dry-run rides `FsSinkConfig.dry_run`
  plus a plan-only mirror pass (`mirror_delete_pass` gains `execute`);
  `--null` swaps `NullSink` at the same seam; local resume is
  sink-level (`FsSinkConfig.resume`), the wire block phase stays
  remote-only; mirror scope threads the user's `FileFilter` directly
  (process-local; no lossy FilterSpec round-trip); dest-nested-in-src
  is excluded by a source wrapper (`DestSubtreeExcludedSource`), the
  session twin of the engine's `exclude_dest_subtree`; move's
  compare mapping and caller-side unreadable gate are untouched and now
  load-bearing (the three otp-10b-2 F3 regression pins run against
  this route).
- **Perf history kept, strategies retired (D3)**: one thin
  `record_local_history` write (`fast_path: "session"`) keeps
  `blit profile`'s local feed; journal-skip/auto-tune/predictor
  training die with the engine at 11b.
- **Fix surfaced by the ported single-file pin**: the sink's
  File-payload arm joined an empty `relative_path` (a file source
  root) onto the roots, producing trailing-slash paths that fail
  ENOTDIR — the exact hazard `FsTransferSource::open_file` documents.
  `write_file_payload` now routes the file-root identity case past the
  joins (`copy_root_file_payload`); the local session route is the
  first caller to send file-root File payloads through it.

## Files changed

- `crates/blit-core/src/transfer_session/local.rs` (new) —
  `run_local_session`, `LocalApply`/`LocalApplyStats`/`LocalApplyRun`,
  `DestSubtreeExcludedSource`, summary synthesis, perf-history write.
- `crates/blit-core/src/transfer_session/mod.rs` — `local` module +
  re-export; `DestinationSessionConfig.local_apply`;
  `diff_chunk_and_apply_local`; local-apply spawn/join in
  `destination_session`; sink selection honors the local sink (record
  helpers widened to `&dyn TransferSink`); mirror scope override;
  `mirror_delete_pass(execute) -> (files, dirs)`.
- `crates/blit-core/src/remote/transfer/sink.rs` — file-root File
  payload fix (`copy_root_file_payload` + shared
  `copy_resolved_file_payload` tail).
- `crates/blit-app/src/transfers/local.rs` — chokepoint re-pointed to
  the session; `spawn_blocking` + nested-runtime wrapper gone.
- `crates/blit-core/src/remote/transfer/session_client.rs`,
  `crates/blit-core/tests/transfer_session_roles.rs` — `local_apply:
  None` at the 17 existing `DestinationSessionConfig` sites.
- `crates/blit-core/tests/local_session.rs` (new) — 21 pins (below).
- `scripts/bench_otp11_local_ab.sh` (new) — the 11a perf gate harness.
- `docs/plan/OTP11_LOCAL_SESSION.md` — the governing slice doc
  (committed one commit earlier, codex-reviewed).

## Tests

Suite before → after: 1488 → 1510 (+22: the 21 pins below plus the
`dest_subtree_rel_detects_nesting` unit test in
`transfer_session/local.rs`; nothing retired in 11a — the old
orchestration tests still run against the still-present engine until
11b's accounting). Gate green this session: fmt --check, clippy
--workspace --all-targets -D warnings, cargo test --workspace
1510/0 (2 ignored).

New `crates/blit-core/tests/local_session.rs` (21):

- Ports of `local_transfers.rs` (7): small tree + session history row,
  up-to-date second run (UpToDate, 0 copies), source-empty, single
  file (+history features), cross-chunk 600-file boundary, nested
  destination self-copy exclusion, 300-file tree.
- Ports of the orchestrator behavior pins (10): incremental bytes
  exclude skipped; mirror refuses incomplete scan; mirror delete
  failure propagates; synced subdir mirrors clean; unrelated dest
  dirs deleted (+ split counters); dry-run creates nothing and
  reports the plan (copy + mirror); single-file filter exclude;
  ignore-existing; size-only skips same-size; force copies identical
  tree (sink second-guess included).
- New otp-11 pins (4): null sink counts but writes nothing; unreadable
  source file recorded in `unreadable_paths` while the copy continues
  (the move-gate signal); mirror subset keeps excluded dest entries;
  all-scope deletes through the filter.

Guard proofs (mutation → failing pin → restore, run this session):

- `DestSubtreeExcludedSource` bypassed (wrapper not installed) →
  `nested_destination_does_not_self_copy` FAILS (second run copies the
  destination into itself); restored → passes.
- `mirror_delete_pass` `execute` forced `true` (dry-run ignored) →
  `dry_run_creates_nothing_and_reports_the_plan` FAILS (stale.txt
  deleted); restored → passes.
- Split delete counters swapped (files↔dirs) →
  `mirror_deletes_unrelated_destination_dirs_and_reports_split` FAILS;
  restored → passes.

Verb-level local pins pass unchanged on the session route:
`single_file_copy.rs` (6), `local_move_semantics.rs` (7 — incl. the
otp-11 regression pin
`local_move_lands_source_bytes_over_same_size_same_mtime_destination`),
`cli_arg_safety_gates.rs`, `diagnostics_dump.rs`, the TUI pins
(`perform_local_move_lands_source_bytes_over_matching_metadata`,
`perform_local_move_deletes_source_after_copy`), and the in-module CLI
verb tests.

Perf gate (11a step 4): `scripts/bench_otp11_local_ab.sh` A/B —
results recorded in `docs/bench/otp11-local-2026-07-11/README.md`.

## Known gaps

- The old orchestration (orchestrator/, engine/, local_worker,
  auto_tune, change_journal) is production-caller-less but in-tree —
  deleted at 11b with the deletion proof and retirement accounting.
- `TransferOutcome::JournalSkip` is now unreachable (no journal on the
  session route); the variant and its CLI line retire at 11b.
- `--workers` no longer bounds a nested runtime (the session runs on
  the ambient runtime; the apply pipeline uses the shared prefetch
  discipline). The flag maps to nothing behavioral in 11a; its
  disposition (inert vs removed) is settled at 11b with the option
  re-home.
- Planner-mix summary fields: `raw_bundle_*` always 0 on the session
  route (the payload planner emits File/TarShard); `--verbose` output
  reflects that, per the slice doc's Known gaps.
- The `remote_remote.rs` positive-control flake under full-suite load
  reproduced once this session and passed in isolation (w9-3 flake
  class, pre-existing); not touched by this slice.

## Reviewer comments

(appended after the codex round)
