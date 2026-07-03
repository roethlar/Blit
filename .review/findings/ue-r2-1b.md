# ue-r2-1b: Wire dial contract (capacity profile, capability bits, resize messages)

**Slice**: ue-r2-1b — second slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded; under GPT review (`docs/agent/GPT_REVIEW_LOOP.md`)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: slice commit (this finding committed with it)

## What

Define the REV4 §5 wire shape before any behavior depends on it: the
receiver capacity profile, the peer-capability bits that gate resize,
and the `DataPlaneResize`/`DataPlaneResizeAck` control messages — plus
old-client/new-daemon and new-client/old-daemon compatibility tests.
No behavior change: every new field is written with its default and
every new oneof variant is ignored on receive exactly as an old binary
would ignore an unknown payload.

## Approach

- **`CapacityProfile`** (new message): the rich receiver→sender profile
  from REV4 Design §4 — `cpu_cores`, `drain_class` (new coarse
  `DrainClass` enum), `load_percent`, `max_streams`,
  `drain_rate_bytes_per_sec`, `max_chunk_bytes`, `max_inflight_bytes`.
  0/UNSPECIFIED = unknown; the sender must treat unknown as "stay
  conservative", never "unlimited".
- **`DataTransferNegotiation`**: `receiver_capacity = 11` (REV4-fixed
  number; first free after the 5–10 RDMA reservation) — the push-path
  profile carrier (daemon is the byte receiver; message already travels
  daemon→client). Plus `resize_enabled = 12` (daemon-authoritative
  resize gate, prior-art semantics) and `epoch0_sub_token = 13`
  (epoch-0 handshake suffix, defined now so `ue-r2-2` is a wire-up).
- **`TransferOperationSpec.receiver_capacity = 12`**: the pull_sync /
  delegated profile carrier (client / dst daemon is the byte receiver).
  **`spec_version` stays 2 — deliberately no bump.** `from_spec`
  enforces the version by exact match, so a bump would make old daemons
  reject new clients outright; the profile is a pure optimization hint
  and old daemons skipping it is the intended mixed-version behavior
  (documented in the proto history note).
- **Capability bits**: `PushHeader.supports_stream_resize = 8` (push
  opens with PushHeader, not PeerCapabilities) and
  `PeerCapabilities.supports_stream_resize = 5`. Both stay false until
  `ue-r2-2` implements resize. The daemon folds peer bit + own support
  + live TCP data plane into `resize_enabled`; resize frames are never
  sent to a peer that didn't advertise.
- **Resize messages**: `DataPlaneResizeOp`/`DataPlaneResize`/
  `DataPlaneResizeAck` carried over from the adaptive PR3 prior art
  (`d9d4ec7`), which was working code. Field-number clash resolved:
  PR3 used negotiation 11–14 for min/max/adaptive_enabled/sub_token;
  REV4 reserves 11 for `receiver_capacity`, PR3's min/max stream
  bounds are subsumed by `CapacityProfile.max_streams` (floor is 1),
  so only `resize_enabled` (12) and `epoch0_sub_token` (13) remain.
- **Oneof slots** (all first-free numbers): `ClientPushRequest.
  data_plane_resize = 9`, `ServerPushResponse.data_plane_resize_ack
  = 5`, `ClientPullMessage.data_plane_resize_ack = 5`,
  `ServerPullMessage.data_plane_resize = 16`. Receive arms at the three
  previously-exhaustive match sites ignore the new variants with a
  comment (identical to the None/unknown arm an old binary hits).
- **Delegated override boundary extended** (the one intentional
  semantic addition): `apply_dst_capabilities_override` now also strips
  any CLI-supplied `receiver_capacity` to `None` — same R25-F2
  rationale as `client_capabilities` (the CLI is not the byte
  recipient; a fabricated ceiling would become live input the moment
  `ue-r2-1e` reads the field on the src side). Replaced with the dst
  daemon's real profile when `ue-r2-1e` builds one.

## Files changed

- `proto/blit.proto` — everything above, with mixed-version behavior
  documented per field.
- `crates/blit-core/src/remote/pull.rs` — spec builder defaults
  (`receiver_capacity: None`, `supports_stream_resize: false`);
  ignore-arm for `ServerPullMessage::DataPlaneResize`.
- `crates/blit-core/src/remote/push/client/mod.rs` — PushHeader
  default; ignore-arm for `ServerPushResponse::DataPlaneResizeAck`.
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — test
  helper gains the new field (normalization logic untouched).
- `crates/blit-daemon/src/service/{pull.rs,pull_sync.rs}`,
  `.../push/{control.rs,data_plane.rs}` — the seven
  `DataTransferNegotiation` literals gain the three new fields (unset),
  each commented with who stamps them and when; ignore-arm for
  `ClientPushRequest::DataPlaneResize` in the push control loop.
- `crates/blit-daemon/src/service/delegated_pull.rs` —
  `dst_capabilities()` + override strip; tests extended.
- `crates/blit-core/tests/proto_wire_compat.rs` — new; see Tests.
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs` — hand-built
  spec now sets every new field so the byte-identical spy-server
  assertion regression-guards the new shapes end-to-end.

## Tests added

13 new (baseline 1378 → 1391, 2 ignored unchanged):

- `proto_wire_compat.rs` (12): old→new and new→old decode for
  `DataTransferNegotiation`, `TransferOperationSpec` (including
  normalization through the real `from_spec` chokepoint, and the
  spec_version-stays-2 gate property), `PushHeader`,
  `PeerCapabilities`; resize command/ack frames decode as
  `payload: None` on old-shape peers for all four extended oneofs
  (with known-variant controls); new↔new round trips for
  `CapacityProfile`, `DataPlaneResize`/`Ack`, and the extended
  negotiation. Old shapes are test-local `#[derive(prost::Message)]`
  replicas (first use of this technique in the repo; blit-core already
  depends on prost).
- `delegated_pull.rs` (1): `dst_override_strips_cli_supplied_receiver_capacity`;
  the existing unconditional-override test also now pins that a CLI
  over-claiming `supports_stream_resize` is forced back to false.

## Known gaps

- No behavior consumes any new field yet — by design (slice contract).
  `receiver_capacity` is stamped/read starting `ue-r2-1e`;
  `resize_enabled`/`epoch0_sub_token`/resize frames starting `ue-r2-2`;
  capability bits flip true when the implementing slice lands.
- The deprecated `Pull` RPC path never sets the new negotiation fields
  and is deleted at `ue-r2-1h`.
- Old-peer behavior is proven at the decode layer (unknown frame →
  `payload: None`) plus source inspection of the None arms; there is no
  binary-level test against an actually-old build (no released old
  binary exists to pin against — pre-0.1.0).
- `NormalizedTransferOperation` does not carry `receiver_capacity`
  through normalization yet; the consuming slice (`ue-r2-1e`) plumbs it
  when there is a consumer, keeping this slice zero-behavior.
