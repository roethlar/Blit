# otp-10c-2 — the four drivers and `Push`/`PullSync` are deleted

**What**: The otp-10 deletion proper (plan §Slices item 10; directive
D-2026-07-05-1 "anything else does not exist"). The four per-direction
drivers, the `Push` and `PullSync` RPCs, and every wire message only
they referenced are out of the tree AND the proto — no bridge
(D-2026-07-05-2). One `TransferSession` and one `Transfer` RPC remain;
`DelegatedPull` survives as trigger + progress relay only, with the
no-payload-bytes proof recorded below. otp-10c-1 (relay removal,
D-2026-07-11-1) cleared the last non-driver consumer beforehand.

## Deletion proof — file by file

**blit-core (client drivers):**

- `remote/pull.rs` — DELETED WHOLE (2,574 lines: `RemotePullClient`,
  `pull_sync`/`pull_sync_with_spec`, `scan_remote_files`,
  `open_remote_file`, `RemoteFileStream`, `PullSyncOptions`,
  `PullSyncError`, `RemotePullReport`, 23 unit tests). The only
  non-driver residents were relocated first, verbatim:
  `build_spec_from_options` + `PullSyncOptions` →
  `remote/transfer/operation_spec.rs` as `delegated_spec_from_options`
  + `DelegatedSpecOptions` (the delegated trigger's spec builder — its
  only remaining consumer; same precedence, same wire bytes, minus the
  deleted `metadata_only` field).
- `remote/push/` — DELETED WHOLE (client driver: `client/{mod,helpers,
  types}.rs`, `data_plane.rs`, `payload.rs`; `RemotePushClient`, the
  bidi response loop, the pre-dial ADD choreography; 7 unit tests).
  The two fs-scan helpers the live `FsTransferSource` used from
  `client/helpers.rs` — `spawn_manifest_task`, `filter_readable_headers`
  (+ `record_unreadable_entry`/`unix_seconds`/`permissions_mode`) —
  moved verbatim into `remote/transfer/source.rs` (private).
- `remote/mod.rs` — `pub mod pull/push` + driver re-exports removed.
- `remote/transfer/sink.rs` — `GrpcFallbackSink` +
  `GrpcServerStreamingSink` DELETED (they spoke `ClientPushRequest` /
  `ServerPullMessage`; the session's in-stream carrier sends
  `TransferFrame`s directly and never used them) with their 6 tests —
  the h3c chunk-cap property they pinned holds on the session by
  construction: `IN_STREAM_CHUNK = CONTROL_PLANE_CHUNK_SIZE` (1 MiB),
  the same frame unit the caps enforced.
- `remote/transfer/payload.rs` — `transfer_payloads_via_control_plane`
  + `send_payload` DELETED (its own comment recorded zero live
  callers; pub-reachable dead code).
- `remote/transfer/grpc_fallback.rs` — DELETED WHOLE (module + 7
  tests): `GRPC_FALLBACK_CHUNK_BYTES`/`clamp_fallback_chunk_size`
  clamped only the two deleted sinks and the deleted dead helper.

**blit-daemon (serving drivers):**

- `service/push/` — DELETED WHOLE (`control.rs` choreography,
  `data_plane.rs` bind/arm/accept + TarShardExecutor,
  `shape_resize_e2e.rs`; 21 tests).
- `service/pull_sync.rs` — DELETED WHOLE (the source-side-diff
  choreography, `GrpcServerStreamingSink` feeding, metadata-only scan
  serving; 13 tests).
- `service/core.rs` — the `push`/`pull_sync` dispatch arms, their
  stream-type aliases, and `resolve_streaming_outcome` (its only
  callers) deleted; `resolve_transfer_session_outcome` (the session
  variant) now documents itself as the owner of that race.
- `service/mod.rs` — module decls + `PushSender`/`PullSyncSender`.
- `service/util.rs` — `resolve_manifest_relative_path`,
  `permissions_mode`, `normalize_relative_path` (handler-only helpers).
- `service/admin.rs` — `purge_extraneous_entries` +
  `plan_extraneous_entries` (the old push-mirror purge; the session's
  one delete rule owns deletion planning) + their 2 tests — the R59
  FilteredSubset scoping they pinned is pinned at the session level
  (`transfer_session_roles.rs` otp-6b filter-scope cells).
- `active_jobs.rs` — `ActiveJobGuard::set_endpoint` (the streaming-RPC
  fill-in; served sessions use `ActiveJobUpdater::set_kind_and_endpoint`
  since otp-10b-2 F4). Its 2 pins re-pointed to the updater.
  **`ActiveJobKind::{Push, PullSync}` and the wire `TransferKind` enum
  values SURVIVE deliberately** — they are the served sessions' row
  labels (jobs UX), not RPC surface.
- `delegated_pull.rs` — the `metadata_only` rejection guard deleted
  with the field (the rejected shape is unrepresentable); its pin
  retired.

**proto/blit.proto** (12.8 KB removed; same-build peers only, so no
reserved-field ceremony is owed — mismatched builds refuse at open):

- `rpc Push`, `rpc PullSync` — DELETED (service block carries the
  dated note).
- Messages deleted (each referenced ONLY by the two dead RPCs):
  `ClientPushRequest`, `ServerPushResponse`, `PushHeader`,
  `UploadComplete`, `Ack`, `PullSyncAck`, `FileList`, `PushSummary`,
  `PullSummary`, `ClientPullMessage`, `ServerPullMessage`,
  `BlockHashRequest`, `DataTransferNegotiation`, and
  `TransferOperationSpec.metadata_only` (the relay scan flag — its
  caller died at otp-10c-1) + its `NormalizedTransferOperation`
  plumbing.
- Messages kept (verified live referents): `TransferFrame` set whole
  (`FileHeader`, `FileData`, `ManifestComplete`, `TarShard*`,
  `BlockHashList`, `BlockTransfer*`, `DataPlaneResize*`, session
  frames), `TransferOperationSpec` + `FilterSpec`/`ResumeSettings`/
  `PeerCapabilities`/`CapacityProfile` (session open + delegated
  trigger), `ManifestBatch`/`BytesProgress` (DelegatedPullProgress),
  the `DelegatedPull*` set, the job-kind `TransferKind` enum, all
  admin/observability messages.

**Grep proof**: `RemotePushClient|RemotePullClient|PullSyncOptions|
handle_push_stream|handle_pull_sync_stream|ClientPushRequest|
ServerPullMessage|rpc Push|rpc PullSync|metadata_only|
GrpcFallbackSink|GrpcServerStreamingSink` over `crates/`, `proto/`,
and live docs returns only: the surviving job-kind labels + their
tests, dated deletion notes, and historical records (`docs/audit/**`,
`docs/reviews/**`, Historical/Shipped plans, DEVLOG) kept verbatim by
convention.

## DelegatedPull no-payload-bytes proof (plan codex F3)

Structural half — the wire cannot carry payload: `DelegatedPullRequest`
is `dst_module`/`dst_destination_path`/`RemoteSourceLocator`/
`TransferOperationSpec`/`trace_data_plane`/`detach` (trigger only);
`DelegatedPullProgress.payload` is a oneof of exactly `Started` /
`ManifestBatch` / `BytesProgress` / `Summary` / `Error` — counters and
diagnostics, no file-content field anywhere in the set. Behavioral
half — the CLI moves no bytes on the delegated route:
`remote_to_remote_copy_delegates_directly_without_cli_byte_path`
asserts `cli_data_plane_outbound_bytes == 0` against two real daemons,
and `local_to_remote_push_is_the_positive_counter_control` (otp-10c-1
F1) proves that counter observes real payloads through the same
flag/file/parser. Payload bytes flow only inside the dst-initiated
`Transfer` session (otp-9b e2es pin byte-identical landing).

## Comment/docs sweep

Old-path comment blocks died with their messages (the otp-1 plan
adjudication): TUI F3 docs retyped to session terms; the test fakes
renamed (`RejectingTransferBlit`, `StallingTransferBlit`) and their
push/pull_sync arms deleted with the trait methods; stall-guard /
session-client / data-plane / compare / active-jobs / dispatcher docs
retargeted; `docs/ARCHITECTURE.md` (service block, module table,
source/sink lists, wiring table) and `docs/WHITEPAPER.md` (combination
table, sinks list, resume section, reviewer-focus list) rewritten to
the one-path shape.

## Tests (suite 1586 → **1480**, exactly the 106 retirements below;
gate green: fmt, clippy -D warnings, `cargo test --workspace` 1480/0.
Two full-run hiccups on the way, neither this slice's code: a
BUILD_MISMATCH refusal between the freshly built CLI and daemon
binaries — the otp-3-reviewed dirty-tree identity sampling window,
converged by resampling blit-core's build script — and one
`test_utils_completions` connection-refused flake, the pre-existing
w9-3 daemon-spawn class, 3/3 green isolated)

- **Died inside deleted modules (71)**: `remote/pull.rs` 23,
  `remote/push/client/mod.rs` 7, `service/pull_sync.rs` 13,
  `service/push/control.rs` 5, `service/push/data_plane.rs` 15,
  `service/push/shape_resize_e2e.rs` 1 (the session's sf-2 shape pins
  live at otp-4b-2), `grpc_fallback.rs` 7.
- **Retired with the wire surface (35)**:
  `pull_sync_with_spec_wire.rs` 11 (relay-scan/resize/spec pins on
  the deleted PullSync client); `proto_wire_compat.rs` 12 → 2 (the 10
  mixed-version replicas tested version tolerance abolished by
  D-2026-07-05-2, over messages deleted here; the 2 surviving new↔new
  round-trip pins kept); the 6 gRPC-sink tests inside the surviving
  `sink.rs` (died with the two deleted sinks);
  `validate_spec_rejects_metadata_only` 1 (the rejected shape is
  unrepresentable); `streaming_canceljob_resolves_pending_handler_
  and_notifies_client` 1 (superseded by the framed-cancel pin
  `transfer_cancel_emits_framed_cancelled_error`, which pins the same
  arm with the stronger frame assertion); admin `purge_filter_tests`
  2; blit-app `delete_listed_paths` containment tests 4 (the
  daemon-authored delete list no longer exists — deletion containment
  is the session mirror's canonical-containment, pinned since otp-6b).
- **Converted, not dropped**: the four A/B parity pins (CLI push/pull
  cutover + daemon push/pull e2e) became ABSOLUTE pins — byte-identical
  tree AND exact fixture counts (what the A/B equality proved
  transitively; the perf half is the committed otp-2/otp-2w baselines
  + otp-12's interleaved old-binary acceptance runs, which use the
  PINNED `e757dcc` binaries staged on zoey, not in-tree code). The two
  `set_endpoint` pins re-pointed to `ActiveJobUpdater::
  set_kind_and_endpoint`; 4 streaming-dispatch pins re-pointed to
  `resolve_transfer_session_outcome`.
- No new production code beyond the two relocations; no new tests —
  every surviving property was already pinned at the session level
  (cited per item above).

## Known gaps

- **1480 is 3 BELOW the plan's end-of-plan floor** (final count ≥
  1483, the pre-plan baseline — an otp-13 acceptance criterion, not a
  per-slice gate). This slice's drop is the deletion the plan always
  priced in, but the margin is now negative: otp-11 (local
  orchestration deletion) retires more while porting local perf pins,
  and otp-12/13 add none automatically. Flagged for the otp-13
  checklist walk — the criterion must be met by real pins (ported or
  new), not re-baselined.
- `DelegatedSpecOptions` still carries `client_capabilities`/
  `receiver_capacity` the dst daemon mandatorily overrides (proto
  boundary note) — relocation was verbatim by design; pruning the
  delegated trigger spec is post-cutover cleanup if the owner wants
  it, not this slice.
- Historical/audit/review docs and Shipped/Historical plans keep
  their PullSync-era text verbatim per repo convention; the
  `docs/WHITEPAPER.md` deep sections beyond the ones corrected here
  remain on the queued w10-docs-batch.
- otp-11 (local orchestration) and the remaining otp-12/13 acceptance
  work are unchanged; the acceptance criteria's deletion-proof line
  for "the separate local orchestration path" completes at otp-11.
