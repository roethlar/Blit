# otp-10a — the push-shaped verb rides the unified session

**What**: First otp-10 (cutover + deletion) sub-slice. The push-shaped
verb — CLI `copy`/`mirror`/`move` with a remote destination (including
the `--relay-via-cli` remote→remote relay) and the TUI F1 push — now
initiates the unified `Transfer` session as its SOURCE instead of
driving the old per-direction push client. The cutover happens at the
one chokepoint both frontends share,
`blit_app::transfers::remote::run_remote_push`, so no verb retains a
path to `RemotePushClient::push` (deletion is otp-10c). This lands the
verb wiring the staged slices deferred to otp-10: push-side
mirror/filter client options, `--force-grpc`, the CLI progress line,
the `end_of_operation_summary` fault print (D-2026-07-09-1 Q2), resume
flags, `--trace-data-plane`, and the unreadable-scan error `blit
move`'s source-delete gate relies on.

**Approach**:

- `PushSessionOptions` (session_client.rs) gains `filter:
  Option<FilterSpec>`, `mirror_enabled`, `mirror_kind`, `progress:
  Option<RemoteTransferProgress>`, `trace_data_plane` — mapped onto
  `SessionOpen` exactly like pull's otp-9a fields; the session has
  honored mirror/filters since otp-6, this is client wiring only.
- New `SourceInstruments` on `SourceSessionConfig` (progress /
  unreadable accumulator / trace flag, all default-off; the daemon
  SOURCE responder passes defaults). Threaded
  `run_source` → `drive_source` → recv/send halves:
  - **Progress = the w6-1 event contract**, not a bare byte counter.
    The plan's Risks section pins "the session emits the existing
    event contract (w6-1) at the same boundaries", and the CLI/TUI
    monitors fold `ManifestBatch`/`Payload`/`FileComplete` through the
    shared `ProgressTotals` — a `ByteProgressSink` (the STATE sketch's
    shorthand) could not feed the files/manifest denominators. The
    recv half reports `ManifestBatch` per received `NeedBatch` (the
    push-direction denominator, same as the old driver; entries are
    unique by contract, dupes fault). The data-plane pipeline already
    emits per-file `Payload`+`FileComplete` when handed a progress
    handle — the session just passed `None`; now it threads the
    caller's. The in-stream carrier's `send_payload_records` reports
    the identical shape (per-file lane, planned manifest sizes) so
    both carriers agree.
  - **Unreadable-scan posture = old push's**: the session still
    streams what it can (`ManifestComplete{scan_complete=false}`), and
    `run_push_session` then fails with the old driver's exact error
    shape ("N file(s) were skipped due to permission or access
    errors: …"). This is load-bearing for `blit move`: its
    source-delete step keys on push success, so success must never
    mask silently-skipped files. Mirror keeps the stronger
    `require_complete_scan` refusal (SCAN_INCOMPLETE at
    ManifestComplete, otp-9b F1).
  - **`--trace-data-plane`** threads to the shared
    `DataPlaneSession::connect/from_stream` trace flag (epoch-0 and
    resize sockets) — the `[data-plane-client]` stderr lines the
    `test_push_tcp_negotiation` pin greps for.
- `PushExecution` retyped: `filter` is now the wire `FilterSpec`
  (single filter chokepoint is the session SOURCE's `FilteredSource`
  wrap from `SessionOpen.filter` — no more CLI-side pre-wrap), plus
  `resume`/`resume_block_size`. `PushExecutionOutcome.report:
  RemotePushReport` → `summary: TransferSummary` (the session's
  destination-computed summary), so otp-10c stays pure deletion —
  no UI rewiring rides the deletion slice.
- CLI (`blit-cli/src/transfers/{mod,remote}.rs`): push verbs build
  `build_filter_spec` (pull parity), pass `resume: args.resume`;
  printers consume `TransferSummary`; on error the new
  `emit_session_fault_summary` walks the eyre chain for
  `SessionFault`/`TransferOpenRefusal` and prints
  `end_of_operation_summary()` (names the faulted file, suggests a
  re-run) to stderr — the verb-level print otp-7b-2 staged here.
- TUI: `build_f1_push_execution` drops the runtime filter, adds the
  resume defaults; the F1 reply reads `outcome.summary` (2 lines).

**Files**:

- `crates/blit-core/src/transfer_session/mod.rs` — `SourceInstruments`
  + threading; `NeedBatch` progress; in-stream per-file reports;
  responder passes defaults.
- `crates/blit-core/src/transfer_session/data_plane.rs` — source
  planes take `&SourceInstruments` (pipeline progress + socket trace,
  incl. resize sockets via the stored `trace`).
- `crates/blit-core/src/remote/transfer/session_client.rs` — options,
  open mapping, unreadable check + error.
- `crates/blit-app/src/transfers/remote.rs` — `run_remote_push` body
  rerouted onto `run_push_session`; `PushExecution`/`Outcome` retyped.
- `crates/blit-cli/src/transfers/{mod,remote}.rs` — filter spec,
  resume flag, `TransferSummary` printers, fault-summary print.
- `crates/blit-tui/src/{exec_plan,main}.rs` — builder + field reads.
- `crates/blit-core/tests/transfer_session_roles.rs` — mechanical
  `instruments: Default::default()` on the 14 source-config literals.
- `crates/blit-cli/tests/push_session_cutover.rs` — new pin suite.

**Tests** (suite 1555 → 1562; nothing removed):

New `push_session_cutover.rs` (drives `run_remote_push` — the verb
boundary — against real spawned daemons):

- `push_verb_and_old_push_produce_identical_trees_and_counts` — A/B
  vs the old driver on twin daemons: byte-identical trees, equal
  files/bytes counts, data-plane default attested.
- `push_verb_mirror_purges_extraneous_and_scores_deletions` — mirror
  ALL through the verb options purges and scores (`entries_deleted`).
- `push_verb_wire_filter_scopes_the_source_scan` — `*.log` exclude
  rides the wire; excluded file never lands.
- `push_verb_force_grpc_rides_the_in_stream_carrier` —
  `in_stream_carrier_used` attested; tree identical.
- `push_verb_reports_w6_1_progress_events` — folded totals equal the
  fixture exactly: denominator == files, one FileComplete per file,
  bytes == planned sizes.
- `push_verb_fails_when_source_has_unreadable_entries` (unix) —
  readable subset lands, then the call errors naming the skip.
- `push_verb_resume_patches_changed_partials_blockwise` —
  `files_resumed == 1`, patched dest byte-identical.

Existing e2e suites (remote_parity push, remote_tcp_fallback ×3,
remote_push_mirror_safety ×2, remote_move push arm, readonly ×2,
remote_remote relay) now exercise the session end-to-end through the
real binary — the ported guards.

Guard proofs by temporary revert (each mutation applied alone, the
named pin run, then restored):

- drop the mirror open-mapping → mirror pin fails (stale survives);
- drop the filter open-mapping → filter pin fails (noise.log lands);
- skip the unreadable check → unreadable pin fails (push "succeeds");
- drop the progress threading → progress pin fails (0 events);
- drop the force-grpc mapping → carrier pin fails;
- drop the resume mapping → resume pin fails (files_resumed 0).

**Review round** (codex on `0fbc966`: NEEDS FIXES, 8 findings — 7
accepted + fixed, F1 accepted in part; adjudication + guard proofs in
`.review/results/otp-10a.gpt-verdict.md`): move pushes with
`IgnoreTimes` (F1 — the compare-mode skip + source-delete data-loss
window, mutation-proven); wire paths POSIX-normalized in
`endpoint_module_path` (F2); daemon `--force-grpc-data` honored by
served sessions (F3); relay+resume refused up front (F4);
`SessionFault.io_kind` keeps `--retry` classification alive across
the fault boundary (F5); resumed files report w6-1 progress on both
carriers (F6); fault-summary extraction unit-pinned (F7);
`build_spec` validates globs pre-connection (F8). Suite 1562 → 1576.

**Known gaps**:

- Output shape changed with the report retype (unshipped product, no
  compatibility bar): JSON drops `files_requested` / `bytes_zero_copy`
  / `first_payload_ms`, gains `files_resumed`; human output drops the
  post-hoc "Negotiation complete … data port N" and first-payload
  lines. `[gRPC fallback]`, "already up to date", "Remote purge
  removed", and "Destination:" survive verbatim.
- `first_payload_elapsed` and `data_plane_streams` verb-level
  observability died with the old report; sf-2 stream-count pins live
  at the session layer (otp-4b-2), so nothing is unguarded.
- COPY-verb compare semantics: the session's same-size dest-newer
  data-safe skip remains the standing owner question (STATE, otp-4a);
  `--checksum` on push stays gate-rejected; compare-mode flag wiring
  (`--size-only`/`--ignore-times`/`--force`/`--ignore-existing`, today
  silently ignored by old push — the R54 gaps) is deferred to otp-10b
  so both verbs get ONE args→open mapping. Move no longer depends on
  that question (it transfers unconditionally — codex F1). The
  R54-F2/R55 move bail texts for `--force`/`--ignore-times` now
  OVERSTATE the local→remote risk (the push arm no longer skips);
  their local→local rationale still holds, so the gates stay —
  reword with the 10b compare-mode wiring.
- The relay path's source-read half still rides PullSync
  (`RemoteTransferSource`); its fate (session-read adapter or removal)
  is the otp-10c PullSync-deletion decision. Relay+resume is refused
  (codex F4) until then.
- `SessionFault.io_kind` is captured at `fault_from_report` and the
  four data-plane dial sites; other direct fault constructions (e.g.
  responder-side accept errors) stay kind-less — the daemon never
  retries, so nothing consumes them today.
- Move re-uploads unchanged files (IgnoreTimes) — the price of
  delete-safety without a push-side checksum compare; revisit if
  checksum compare lands on the session.
- Old push soft-skipped files that vanished mid-transfer
  (`check_availability` → skip + end-of-push error); the session
  faults naming the file. Both end in an error; move stays safe.
- The old `[data-plane-client] aggregate … Gbps` throughput trace line
  is not reproduced (connect traces are).
