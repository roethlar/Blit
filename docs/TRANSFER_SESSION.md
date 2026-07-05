# TransferSession wire + session contract (otp-1)

**Status**: Active (contract; implementation lands otp-3..otp-10)
**Created**: 2026-07-05
**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)

This document is the authoritative contract for the single `Transfer`
RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
session"; this doc explains the state machine the proto cannot.

## Invariants

1. **One vocabulary, role-tagged.** Both wire directions carry the
   same frame type (`TransferFrame`). Which frames an end may send is
   determined by its ROLE (SOURCE or DESTINATION), never by whether
   it is the gRPC client or server. This is the structural form of
   the owner's invariant: there is no push-shaped or pull-shaped
   message set to diverge.
2. **Same build only (D-2026-07-05-2).** The first frame each way is
   `SessionHello{build_id, contract_version}`. Both ends compare for
   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
   naming both ids, then stream close. No negotiate-down, no advisory
   fields, no feature-capability bits — same build implies same
   features. `build_id` = `<crate version>+<git commit hash>[.dirty]`
   composed at compile time; `contract_version` is a belt-and-braces
   integer bumped on any wire-shape change (exact match required).
3. **Roles.** The initiator (the end that opened the RPC — a CLI
   client, or a daemon acting as delegated initiator) declares in
   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
   (always a daemon) takes the other role. All four
   initiator/role combinations run the identical state machine.
4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
   from live enumeration (immediate start — no buffered-enumeration
   phase in any direction). DESTINATION diffs incrementally against
   its own filesystem and streams need batches back. DESTINATION is
   authoritative for what it has; SOURCE is authoritative for what
   exists to send.
5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
   (whichever end holds DESTINATION) advertises its
   `CapacityProfile` at session open — in `SessionOpen` when the
   initiator is DESTINATION, in `SessionAccept` when the responder
   is. The byte SENDER (SOURCE) owns the live dial bounded by that
   profile. Absent/0 profile fields mean "unknown hardware value" —
   conservative defaults, never unlimited, and NEVER "old peer"
   (there are no old peers).
6. **One stream policy.** The data plane opens at the dial floor
   immediately; SOURCE shape-corrects the stream count upward via
   resize as the need list accumulates (the sf-2 mechanism —
   `TransferDial::propose_shape_resize` — now the only policy).
   SOURCE is the resize controller in every session.

## Phase state machine

```
INITIATOR                                RESPONDER
  |-- SessionHello ----------------------->|   (phase: HELLO)
  |<------------------------ SessionHello--|
  |     both verify build_id exact match; mismatch => SessionError + close
  |-- SessionOpen ------------------------>|   (phase: OPEN)
  |<---------------------- SessionAccept --|
  |     responder validates module/path/read-only/gate here;
  |     refusal is a SessionError, never a silent close
  |                                        |
  |==== manifest + need + payload phases run CONCURRENTLY =========|
  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
  |  DEST streams:    NeedBatch* ... NeedComplete                  |
  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
  |                   frames when the in-stream carrier is chosen) |
  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
  |                                                                 |
  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
  |  `resume=true` is followed by DEST's BlockHashList for that     |
  |  file BEFORE SOURCE may send any byte of that file; stale or    |
  |  mismatched partials fall back to full-file transfer.           |
  |                                                                 |
  |  mirror: DEST computes deletions LOCALLY from the completed     |
  |  source manifest (filter-scoped, scan-complete-guarded) and     |
  |  executes them itself. No delete list crosses the wire.         |
  |                                                                 |
  |-- SourceDone (all payloads flushed) -->|   (phase: CLOSING)
  |<---------------- TransferSummary ------|   (DEST is the scorer)
  |     stream close                        |
```

- Phase violations (a frame arriving in a phase where its role may
  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
  fail-fast, no tolerant parsing.
- `NeedComplete` is DESTINATION's promise that no further need
  batches follow (SOURCE may finish after flushing what was asked).
- `TransferSummary` always travels DESTINATION → SOURCE (the end
  that wrote bytes and executed deletes is the end that can attest
  to them), then the initiator surfaces it to the operator.

## Frame set and field numbers

`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`

`TransferFrame.frame` oneof (field numbers frozen by this doc):

| # | frame | sender | phase |
|---|-------|--------|-------|
| 1 | `SessionHello` | both, first frame | HELLO |
| 2 | `SessionOpen` | initiator | OPEN |
| 3 | `SessionAccept` | responder | OPEN |
| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
| 10 | `FileData file_data` | SOURCE | in-stream carrier |
| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
| 14 | `BlockTransfer block` | SOURCE | resume |
| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
| 18 | `SourceDone source_done` | SOURCE | closing |
| 19 | `TransferSummary summary` | DESTINATION | closing |
| 20 | `SessionError error` | both | any |

Reused messages (`FileHeader`, `FileData`, `TarShard*`,
`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
existing shapes — the session reuses the engine's payload vocabulary
verbatim. New messages (`SessionHello`, `SessionOpen`,
`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
defined in the proto with their field numbers.

Deliberately absent: `PeerCapabilities` (same build = same
features), `spec_version` negotiation (the hello's exact match
replaces it), any delete list (mirror is destination-local), any
push/pull-specific message.

## Transport selection

- **TCP data plane (default):** the RESPONDER binds the listener and
  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
  dials (NAT/firewall reality — connection topology, not
  choreography). Byte direction on the sockets is set by role:
  SOURCE writes, DESTINATION reads. Resize ADD epochs arm one
  accept per `sub_token`, exactly as ue-r2-2 built.
- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
  (operator `--force-grpc` diagnostics) or granted by the responder
  when it cannot bind a data plane (`SessionAccept` with no grant).
  Payload frames 9-15 ride the RPC itself. Same choreography, same
  planner decisions, different byte carrier.
- **Local (in-process):** the identical session state machine runs
  with both roles in one process over an in-process frame channel —
  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
  file vs block) is planner-owned and reads workload shape +
  capability, never role/initiator/transport.

## Errors, cancel, stall

- `SessionError{code, message, detail}` codes:
  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
  or aborts says why before closing; operators never diagnose from a
  bare stream reset.
- `CancelJob` interop: the responder registers the session in
  `ActiveJobs` at OPEN (same transfer_id contract as today); the
  cancel token races the session exactly as w4-3 wired, and the
  peer receives `SessionError{CANCELLED}`.
- StallGuard, byte-accounting, and progress events (w6-1 contract)
  attach at the same boundaries they do today; the session emits the
  existing `DaemonEvent` payloads.

## What this replaces

At cutover (otp-10): `Push`, `PullSync`, and their message
choreographies are deleted from the proto and the tree; the four
per-direction drivers die with them; `DelegatedPull` shrinks to
trigger + progress relay (no payload bytes). Until then this
contract's surface exists compiled-but-refusing
(`Transfer` returns `UNIMPLEMENTED`; pinned by test).
