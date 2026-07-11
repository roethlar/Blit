# otp-10b-2 — the pull-shaped verb rides the unified session

**What**: Second otp-10b sub-slice: every pull-shaped verb (CLI
`copy`/`mirror`/`move` from a remote source, TUI F3 pull) initiates a
DESTINATION-role `Transfer` session via `run_pull_session` through one
chokepoint (`blit_app::transfers::remote::run_remote_pull`) — the old
per-direction `RemotePullClient::pull_sync` driver is no longer
reachable from any verb (it stays in-tree for the relay read half,
the delegated spec builder, and the otp-10c A/B reference). With
otp-10a this completes the verb cutover; otp-10c is pure deletion.

**Approach**:

- **ONE args→compare mapping for BOTH verbs**
  (`blit_app::transfers::compare`): the old pull's
  `build_spec_from_options` precedence — `--ignore-times` >
  `--force` > `--size-only` > `--checksum` > SizeMtime — extracted
  into `verb_comparison_mode(flags, move_verb)` and used by push and
  pull alike. Converge-up: the old push driver silently ignored every
  compare flag (R54-F2 documented it); now both verbs honor the same
  flags identically. Push's `--checksum` gate
  (`ensure_remote_push_supported`) lifts — the session's Checksum
  compare (otp-10b-1, contract v3) serves both roles, and a
  `--no-server-checksums` daemon refuses at OPEN with
  `CHECKSUM_DISABLED` either direction.
- **Move mapping** (codex otp-10a F1 mirrored): a move verb maps to
  `Checksum` when `--checksum` is given (a content-proven skip is
  safe before source-delete), else `IgnoreTimes` (transfer
  unconditionally). `--size-only` joins `--force`/`--ignore-times` as
  a rejected move flag (new gate): the old move-pull honored it, and
  a size-only skip of a changed file followed by source-delete
  destroys the only copy — the exact F1 hazard. The stale R54-F2/R55
  gate texts (which claimed push `--checksum` had no safe end-to-end
  path) are rewritten to the current truth; the pinned
  `move does not support --force/--ignore-times` prefixes kept.
- **`--ignore-existing`** rides `SessionOpen.ignore_existing` for
  both verbs (the session destination has honored it since otp-4; the
  old push driver ignored it silently).
- **Dest-side w6-1 progress**: `DestinationSessionConfig` grows
  `DestinationInstruments { progress, byte_progress,
  trace_data_plane }` (symmetric with `SourceInstruments`;
  `byte_progress` folded in). The DESTINATION reports need batches as
  the denominator (`ManifestBatch` per NeedBatch emitted — the same
  files-to-transfer semantic the push verb reports), and per-file
  `Payload`/`FileComplete` from both carriers: the data-plane receive
  threads the handle into `execute_receive_pipeline` (which already
  spoke the contract; the session passed `None`), the in-stream
  record arms report inline with the same per-record conventions.
- **Pull `--trace-data-plane`**: the DESTINATION initiator's
  epoch-0 and resize dials emit the `[data-plane-client]
  connecting to …` trace (the old pull had no trace wiring at all;
  its unconditional `[pull-data-plane]` per-stream line dies with
  the driver).
- **Mirror = the one delete rule**: the session DESTINATION already
  plans+executes deletions at SourceDone (otp-6b) and scores them in
  `summary.entries_deleted`; `apply_pull_mirror_purge` and the
  client-manifest upload (`enumerate_local_manifest`) leave the verb
  path (deleted with the driver at otp-10c).
- **Printers retype** to the session `TransferSummary` (the push
  printer shape): JSON keys
  `operation/destination/files_transferred/bytes_transferred/
  files_resumed/entries_deleted/tcp_fallback`; human output keeps the
  pinned `Pull complete:` prefix and `[gRPC fallback]` marker, purge
  reports as `Mirror purge removed N entr(y|ies)` (files+dirs one
  count — the wire carries no split). `bytes_zero_copy` dies with the
  driver (always 0 on the session; zero-copy returns as a
  post-cutover write strategy, D-2026-07-05-3).
- **Verb lifecycle simplifies**: no post-RPC destructive step remains
  (deletes are in-session), so the pull monitor lifetime matches the
  push verb's; the a0-pull-execution round-2 split becomes moot on
  the verb path. Move's remote-source delete still runs after the
  session, gated by `require_complete_scan` (`SessionOpen` field; the
  session refuses partial scans at ManifestComplete — otp-9b F1). The
  old pull's pre-created destination parent proved redundant on the
  session path (the sink creates each write target's parent chain,
  single-file case included — mutation-verified) and was not ported.
- **TUI F3**: `build_f3_pull_execution` (unit-pinned, incl. the move→
  IgnoreTimes pin mirroring `build_f1_push_execution`); mirror's
  purge count reads `summary.entries_deleted`. `f3_pull_options`
  survives only for the delegated builder (PullSync spec — otp-10c's
  concern).

**Files**:

- `crates/blit-core/src/transfer_session/mod.rs` —
  `DestinationInstruments`, destination progress reporting (need
  batches + in-stream arms), threading.
- `crates/blit-core/src/transfer_session/data_plane.rs` — receive
  progress + dial traces on the initiator receive plane.
- `crates/blit-core/src/remote/transfer/session_client.rs` —
  `PullSessionOptions { progress, trace_data_plane }`.
- `crates/blit-app/src/transfers/compare.rs` — the one mapping.
- `crates/blit-app/src/transfers/remote.rs` — `PullExecution` +
  `run_remote_pull`; `PushExecution.ignore_existing`.
- `crates/blit-app/src/endpoints.rs` — push checksum gate lifted.
- `crates/blit-cli/src/transfers/{remote,mod,endpoints}.rs` — verb
  rewiring, printers, move `--size-only` gate, gate-text refresh.
- `crates/blit-tui/src/{exec_plan,main}.rs` — F3 pull on the session.

**Tests** (suite 1581 → see commit; grows, never drops):

- `crates/blit-app/src/transfers/compare.rs` unit pins: the old
  pull's precedence table verbatim + the move override.
- `crates/blit-cli/tests/pull_session_cutover.rs` (new, 11 pins): A/B
  parity vs the old pull driver (twin daemons, byte-identical trees +
  count parity), mirror purge scoring, wire filter, `--force-grpc`
  carrier, daemon `--force-grpc-data` (the SOURCE-responder half of
  otp-10a F3), w6-1 progress totals, resume block-patch + progress on
  BOTH carriers, move-shaped IgnoreTimes with a SizeMtime control,
  verb-level checksum content-skip with a SizeMtime control,
  `--ignore-existing`, single-file pull layout (missing parent).
- Binary e2es: `pull_move_lands_source_bytes_over_same_size_newer_
  destination` (remote_move.rs — the pull twin of the 10a data-loss
  pin), `push_checksum_{rejected,succeeds}…` (the gate-lift pair in
  remote_checksum_negotiation.rs), `local_move_rejects_size_only_flag`
  (cli_arg_safety_gates.rs), `test_pull_tcp_negotiation` now asserts
  a traced receive dial.
- `test_pull_multistream_many_files` ported deliberately: the old
  driver's unconditional `[pull-data-plane]` stderr line no longer
  exists; the fan-out proof becomes `--trace-data-plane` dial traces
  (≥2 receive sockets: epoch-0 + a shape-correction resize). Called
  out here per the assertion-change rule.
- TUI pin: `build_f3_pull_execution_wires_mirror_and_move_safety`.
- Guard proofs by temporary mutation, each run to a FAILING pin then
  restored: (N) mirror wiring, (O) filter wiring, (Q) ignore_existing,
  (S) resume, (T) force_grpc, (P1) the need-batch denominator report,
  (P2) the data-plane per-file progress lane, (V) the CLI move
  mapping — reproduced the exact skip-then-delete data loss at the
  binary level (exit 0, wrong bytes at dest), (W) the pull trace
  wiring (binary). The checksum/move-shaped pins carry in-test
  SizeMtime controls instead; the A/B pin is its own reference.
  Mutation (U) — deleting the ported parent-creation step — left the
  single-file pin GREEN, so the redundant code was removed rather
  than kept vacuously.

**Known gaps**:

- The old pull driver, `PullSyncOptions`, `enumerate_local_manifest`,
  and `apply_pull_mirror_purge` remain in-tree (unreachable from
  verbs) until otp-10c deletes them with ported-test accounting.
- The TUI mirror purge count now includes deleted directories
  (`entries_deleted` is one count; the old reply carried
  `files_deleted` only).
- Relay (`--relay-via-cli`) still reads its remote source through the
  old PullSync client; its fate is otp-10c.
- Byte-progress accounting differs slightly per carrier for tar
  shards: the shared receive pipeline reports wire bytes (archive
  framing included) on the data plane, while the in-stream arm and
  the push source report content/planned sizes. Display-only; the
  progress pin bounds it (`content ≤ bytes ≤ content + 1 MiB`).
- The delegated dst daemon passes `progress: None` /
  `trace_data_plane: false` — its live lane stays the jobs-row byte
  counter, and daemon stderr is a log, not an operator terminal.
