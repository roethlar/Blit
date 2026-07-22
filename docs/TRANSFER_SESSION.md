# TransferSession wire + session contract (otp-1)

**Status**: Active (contract; the session is the ONLY remote transfer
path since cutover, otp-10c-2)
**Contract version**: 4 (Windows file attributes + named `$DATA` streams)
**Created**: 2026-07-05
**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)

This document is the authoritative contract for the single `Transfer`
RPC — the only byte-moving RPC since cutover (`Push` and `PullSync`
were deleted whole at otp-10c-2). Proto truth lives in
`proto/blit.proto` under "ONE_TRANSFER_PATH unified session"; this
doc explains the state machine the proto cannot.

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
   features. `build_id` = `<crate version>+<git commit hash>`
   composed at compile time; `contract_version` is a belt-and-braces
   integer bumped on any wire-shape change (exact match required).
   Imprecise identities never false-match (otp-3 codex F1): a dirty
   tree composes `<sha>.dirty.<content hash>` (deterministic — only
   byte-identical dirty trees match), and a build without git
   identity composes `unknown.<per-compilation entropy>` (only the
   selfsame binary matches itself).
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
6. **One live stream policy.** The data plane opens at a conservative
   receiver-bounded floor. SOURCE is the sole resize controller in every
   session and adjusts stream count upward or downward from measured
   per-stream send telemetry. Workload file/byte totals do not select a
   terminal worker count. The DESTINATION's advertised capacity is a
   safety ceiling, not a target.
7. **Carrier-independent Windows metadata (contract v4).** A Windows SOURCE
   describes the settable file attributes and every named `$DATA` stream in
   `FileHeader.windows_metadata`; it does so for every regular file, including
   a file with no named streams. The manifest carries stream descriptors and
   hashes but no stream content. A granted file, tar-shard, or resume payload
   carries the same descriptors plus the complete named-stream content. The
   DESTINATION validates payload metadata against the retained manifest header
   and applies it only after the unnamed file data has landed. Local and remote
   carriers use the same representation and validation; planner choice cannot
   change fidelity.

`docs/plan/LIVE_DIAL_TUNING.md` is the implementation and evidence record for
invariant 6. Its ldt-2 cutover removed sf-2's static shape target and made the
same SOURCE-owned ADD/REMOVE controller operational in both socket layouts.

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
  |==== from here the lanes are ROLES, not initiator/responder ====|
  |  (whichever end holds SOURCE sends source-lane frames,          |
  |   regardless of which end opened the RPC)                       |
  |                                                                 |
  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
  |  DEST streams:    NeedBatch* ... NeedComplete                  |
  |  SOURCE TCP data-plane payload may overlap the still-open      |
  |  manifest after its matching NeedBatch; in-stream payload      |
  |  starts only after ManifestComplete.                           |
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
  |  CLOSING (role-directed, both initiator layouts):               |
  |    SOURCE -> DEST:  SourceDone (all requested payloads flushed) |
  |    DEST -> SOURCE:  TransferSummary (DEST is the scorer)        |
  |  then the INITIATOR closes the RPC stream.                      |
```

- Phase violations (a frame arriving in a phase where its role may
  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
  fail-fast, no tolerant parsing.
- `NeedComplete` is DESTINATION's promise that no further need
  batches follow (SOURCE may finish after flushing what was asked).
  It may be sent only after BOTH: the source's `ManifestComplete`
  has been received AND the destination has finished diffing every
  received manifest entry. Mirror deletions additionally require the
  scan-complete guard, as above.
- **TCP scan/transfer overlap:** for an ordinary copy that does not require a
  complete scan, SOURCE may queue a need-authorized TCP payload while later
  manifest entries are still being enumerated. The data sockets are a separate
  authenticated lane, and DESTINATION inserts every requested path into its
  outstanding set before sending the corresponding `NeedBatch`, so the same
  strict need-list claim still gates every early payload. Mirror and
  `require_complete_scan` operations retain the stronger pre-write refusal:
  they do not queue payload until `ManifestComplete{scan_complete=true}` has
  passed. `NeedComplete`, `SourceDone`, final outstanding-set validation, and
  mirror deletion ordering are unchanged. This is an ordering correction, not
  a new frame or a mixed-build compatibility surface; exact build matching is
  still mandatory.
- **Flow control is the transport's, deliberately:** manifest, need,
  and in-stream payload frames ride gRPC/HTTP-2 stream flow control;
  each end holds only bounded internal queues (the engine's existing
  batching — 128-entry manifest check chunks, need-list batcher).
  Nothing in the contract requires unbounded buffering of the peer's
  stream, and implementations must not introduce it.
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

Shared messages (`FileHeader`, `FileData`, `TarShard*`,
`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
`MirrorMode`, `ResumeSettings`, `CapacityProfile`) are the engine's payload
vocabulary. Contract v4 extends `FileHeader` as specified below. New session
messages (`SessionHello`, `SessionOpen`,
`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
defined in the proto with their field numbers.

### `FileHeader` Windows metadata (contract v4)

`FileHeader` adds field 6, `optional WindowsFileMetadata windows_metadata`.
The nested shapes are:

```
message WindowsFileMetadata {
  uint32 file_attributes = 1;
  repeated WindowsNamedStream named_streams = 2;
}
message WindowsNamedStream {
  string name = 1;
  uint64 size = 2;
  bytes checksum = 3; // Blake3, exactly 32 bytes
  bytes content = 4;  // empty in a manifest; exact `size` bytes in a payload
}
```

- `file_attributes` contains only the durable attributes that ordinary files
  can be required to retain across supported Windows filesystems: READONLY,
  HIDDEN, SYSTEM, and ARCHIVE (mask `0x00000027`). Volatile or storage-policy
  bits such as TEMPORARY, OFFLINE, and NOT_CONTENT_INDEXED, plus structural or
  separately-managed bits such as DIRECTORY, REPARSE_POINT, SPARSE_FILE,
  COMPRESSED, and ENCRYPTED, are never represented by this field. Zero means
  the destination applies `FILE_ATTRIBUTE_NORMAL`. After
  `SetFileAttributesW` succeeds, the destination reads the durable mask back;
  a mismatch fails the file instead of reporting success and retransferring it
  on every later run.
- Named streams are data only: enumeration accepts the default `::$DATA` but
  does not serialize it, accepts `:name:$DATA` as a named stream, and rejects
  every other stream type. The serialized `name` is only `name`, never either
  colon or the `$DATA` suffix. A name must be non-empty valid Unicode, at most
  1,024 UTF-8 bytes, contain no NUL, control character, `:`, `/`, or `\`, and
  not be `.` or `..`. Names must be unique under Unicode lowercase comparison.
- One file may carry at most 64 named streams and at most 2 MiB of named-stream
  content in aggregate. Each descriptor's `size` must fit that aggregate cap,
  its checksum must be exactly 32 bytes, and payload content must be exactly
  `size` bytes and match the checksum. Enumeration, hydration, framing, and
  receipt enforce the same constants before allocation or filesystem writes.
  A source outside these bounds is recorded as unreadable and omitted with its
  affected path while unrelated files continue; the shared incomplete-scan
  gate still refuses mirror/move deletion. It is never copied with metadata
  silently omitted. An existing destination stream set outside these bounds is
  a metadata mismatch: bounded replacement removes stale names without reading
  their content instead of making comparison unusable.
- A manifest header has `content == empty` for every descriptor, including a
  non-empty stream. Before a payload is queued, SOURCE reopens each stream,
  reads exactly its declared size, rejects growth/truncation/hash drift from
  the manifest descriptor, and fills `content`. A payload with missing,
  duplicate, extra, incomplete, or changed stream data is a protocol failure.
  The DESTINATION retains the granted manifest header and compares the payload
  attributes plus ordered-by-name descriptors before claiming the need.
- On Windows, destination comparison treats a metadata mismatch as a need
  even when the ordinary size/time or checksum comparison would skip; explicit
  `ignore_existing` still means no change to an existing destination. Applying
  metadata replaces the named-stream set: stale destination streams are
  deleted, declared streams are written completely, then the declared
  attributes are applied. Any failure fails the file/session; metadata errors
  are not best-effort warnings. On a non-Windows DESTINATION, a present
  `windows_metadata` is refused rather than silently discarded.
- `FileHeader.size` remains the unnamed stream's size. File and byte completion
  is published only after named streams and attributes have been applied.
  Cancellation or a metadata failure therefore cannot produce a successful
  per-file completion or final transfer summary. Named-stream bytes count as
  transferred payload bytes; manifest descriptor bytes do not.

### Cross-platform Windows metadata policy (contract v5)

Strict preservation is the default. If a manifest entry contains
`windows_metadata` and the DESTINATION platform cannot represent it, the
destination rejects that entry during manifest comparison: after safe path
resolution, but before filesystem comparison, `NeedBatch`, `BlockHashList`, or
any payload grant. An existing partial file is therefore unchanged, including
when resume was requested.

The only downgrade is the explicit CLI flag `--drop-windows-metadata`, which
prints a warning that Windows attributes and named data streams are permanently
discarded. It travels as `SessionOpen.drop_windows_metadata = 13`; delegated
remote-to-remote requests carry the same bit in `TransferOperationSpec` v3.
When set, the SOURCE omits Windows metadata before manifest emission. The
filesystem source also skips metadata enumeration and named-stream hashing, so
an unrepresentable stream set cannot prevent the unnamed file from being
copied under the user's explicit lossy choice. Payload preparation consequently
does not hydrate or send named-stream bytes. The policy is role- and
carrier-independent: local, push, pull, and delegated transfers all enter the
same SOURCE scan and DESTINATION diff chokepoints. Session contract v5 exact
matching prevents a peer from silently ignoring the policy.

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
  SOURCE writes, DESTINATION reads.
  **`initial_streams` is the exact epoch-0 logical membership:**
  `min(DIAL_FLOOR_INITIAL_STREAMS, receiver_stream_ceiling)`. The
  responder arms exactly that many accepts and the initiator opens exactly
  that many sockets. A zero/absent receiver maximum resolves to the documented
  default safety limit; it is not a one-stream cap. Later membership changes
  only through SOURCE-initiated live resize. Each target is absolute and must
  be exactly the current settled count plus one for ADD or minus one for
  REMOVE. REMOVE opens no socket: SOURCE retires one worker at a payload
  boundary and its normal `END` closes the matching receive worker.
  **Socket auth, exact:** every epoch-0 socket opens with
  `session_token` (16 bytes) immediately followed by
  `epoch0_sub_token` (16 bytes); every resize-ADD socket opens with
  `session_token` followed by that epoch's `sub_token` from the
  `DataPlaneResize` frame. Every ADD epoch uses a fresh 16-byte token;
  REMOVE's token is empty. Tokens are single-session: the shared epoch-0
  credential authenticates each of the exact `initial_streams` sockets, while
  each fresh ADD credential admits exactly one socket. A socket presenting
  anything else is closed without response. The DESTINATION validates the next monotonic epoch,
  one-step target, token shape, floor/ceiling, and current settled count
  before ACKing. An exact duplicate request replays the prior ACK without a
  second membership change; changed, stale, or future frames are protocol
  violations. A matching `DataPlaneResizeAck{accepted:false}` consumes that
  epoch and is terminal for further resize proposals; settled workers
  continue. An accepted ADD/REMOVE settles only after SOURCE membership
  actually joins/retires; a post-accept live-membership failure faults the
  session rather than publishing a false effective count. At terminal
  `NeedComplete`, a proposal the peer never accepted settles unchanged. An
  accepted ADD still completes admission/authentication and then reaches its
  normal `END`, even when the payload queue has already closed; an accepted
  REMOVE settles only after the exact member retires or is confirmed already
  ended. Terminal cleanup cannot rewrite an accepted operation as refused.
  **Resume on the data plane (otp-7b):** in a resume session, block
  records ride the sockets as the binary `BLOCK`/`BLOCK_COMPLETE`
  record shapes (the receive pipeline's existing tags), while the
  `BlockHashList` stays a control-lane frame. All records of one
  resumed file travel one socket, in order, ending with its
  `BLOCK_COMPLETE` (which carries mtime+perms so zero-block resumes
  still stamp metadata). The DESTINATION-chosen block size clamps to
  the CARRIER's ceiling — 2 MiB in-stream (tonic frame limit,
  D-2026-07-10-1), 64 MiB data plane (the wire block record bound,
  D-2026-07-10-2) — with a shared 64 KiB floor and 65_536-hash list
  cap; both ends read the carrier from grant presence.
  **Windows metadata records (contract v4):** every binary FILE and each
  TAR_SHARD member carries the payload form of `WindowsFileMetadata` after its
  existing path/size/mtime/permissions fields. BLOCK_COMPLETE carries the
  payload metadata for the resumed file. Length/count fields use the same caps
  above; the receiver validates them before allocation and validates the whole
  payload against the retained manifest header before claiming the need.
- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
  (operator `--force-grpc` diagnostics) or granted by the responder
  when it cannot bind a data plane (`SessionAccept` with no grant).
  Payload frames 9-15 ride the RPC itself. Same choreography, same
  planner decisions, different byte carrier.
  **Record grammar (fail-fast):** payload records on the
  source-lane are STRICTLY SERIALIZED — after `file_begin(header)`,
  only `file_data` frames for that file may follow on the lane until
  the record completes; completion is inferred at exactly
  `header.size` cumulative bytes (a `file_begin`/`tar_shard_header`/
  `block` arriving early, or bytes overrunning `size`, is
  `PROTOCOL_VIOLATION`). Tar-shard records run
  `tar_shard_header … tar_shard_chunk* … tar_shard_complete`; block
  records complete with `block_complete`. Payload records may begin
  only AFTER the source's `ManifestComplete` — this per-transport
  ordering rule applies identically to both roles and mirrors the
  design-4-proven fallback ordering, so in-stream manifest frames and payload
  records never interleave. TCP data-plane payload is governed separately by
  the need-authorized overlap rule above. DESTINATION-lane frames (need batches,
  acks, summary) are unaffected — they travel the other direction.
  Contract v4 applies the manifest/payload distinction above to both
  `file_begin` and every `TarShardHeader.files` member. Header-size splitting
  includes the encoded Windows metadata, so no metadata-heavy shard can cross
  the existing in-stream protobuf-frame ceiling.
- **Local (in-process, otp-11):** both roles run in one process over
  the in-process frame channel — no RPC, no sockets — with the LOCAL
  byte-carrier: a process-local destination extension (`LocalApply`,
  crate-private, NO wire representation — a peer structurally cannot
  select it) under which the need-grant/payload phase collapses into
  the destination, which plans the needed headers
  (`plan_transfer_payloads`) and applies them in-process through the
  filesystem sink (clonefile / block-clone / copy_file_range), so no
  payload byte rides any lane. Everything else is the shared state
  machine verbatim: hello (exact-match build identity), open
  validation/refusals, manifest streaming + `ManifestComplete
  {scan_complete}`, the destination-owned diff (`DEST_DIFF_CHUNK`
  batching, both carriers), `NeedComplete`, the mirror guards + the
  one delete pass at SourceDone, the destination-computed summary. No
  NeedBatch is sent and nothing enters the outstanding set — a
  payload record arriving anyway is `PROTOCOL_VIOLATION`. Resume on
  this carrier is the sink-level block phase (`resume_copy_file`:
  hash the partial, rewrite differing blocks, full-file fallback) —
  the same resume semantic without serializing block records the
  same process would immediately deserialize. Strategy selection
  (tar-shard vs file vs block) stays planner-owned and reads workload
  shape + capability, never role/initiator/transport.
  Contract v4 is not bypassed by this optimization: local payload preparation
  hydrates and validates the same manifest descriptors, and the filesystem sink
  applies the same exact named-stream replacement and attributes after either
  the file-copy cascade or tar extraction completes.

## Errors, cancel, stall

- `SessionError{code, message}` codes (plus both build ids on
  BUILD_MISMATCH):
  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`, `CHECKSUM_DISABLED`
  (contract v3, below). An end that refuses
  or aborts says why before closing; operators never diagnose from a
  bare stream reset. Since contract v2 (otp-7b-2, the D-2026-07-09-1
  Q2 rider) the frame also carries `optional relative_path` — the
  file the fault concerns, when one is known (per-file read/write
  failures name their file; optional because "" is itself the valid
  identity of a single-file-root transfer). Both ends can therefore
  name the affected file in their end-of-operation summary,
  structurally, wherever the fault originated.
- **Checksum compare (contract v3, otp-10b-1)**: a session opened
  with `COMPARISON_MODE_CHECKSUM` is a content compare — the SOURCE
  fills each `ManifestEntry.checksum` (Blake3, hashed through its own
  read path, after the filter so only in-scope files pay), and the
  DESTINATION hashes its same-size candidates during the diff, so a
  content-equal file SKIPS regardless of mtime and a content-differing
  same-size+mtime file transfers. Role-agnostic: whichever end holds
  SOURCE hashes the manifest; whichever holds DESTINATION hashes its
  candidates. A responder whose operator disabled hashing
  (`--no-server-checksums` / `server_checksums_enabled = false`)
  refuses the open with `CHECKSUM_DISABLED` — the session never
  silently degrades a content-compare request to a weaker mode. A
  missing checksum on either side of a comparison degrades to
  transfer (conservative, never a false skip).
- **Windows metadata failure (contract v4):** enumeration, payload hydration,
  destination comparison, validation, stream replacement, and attribute apply
  are correctness steps, not best-effort decoration. An error names the file,
  aborts the session, joins owned workers, and suppresses that file's completion
  plus the final success summary. Cancellation races metadata work through the
  existing session cancellation path and has the same no-success rule.
- `CancelJob` interop: the responder registers the session in
  `ActiveJobs` at OPEN (same transfer_id contract as today); the
  cancel token races the session exactly as w4-3 wired, and the
  peer receives `SessionError{CANCELLED}`.
- StallGuard, byte-accounting, and progress events (w6-1 contract)
  attach at the same boundaries they do today; the session emits the
  existing `DaemonEvent` payloads.
- Session-owned asynchronous work is joined on every normal, error, and cancel
  exit: SOURCE scan/count/filter/checksum helpers, the dial tuner, elastic
  pipeline and nested workers, and all DESTINATION receive workers. Cleanup
  closes bounded scan inputs before joining non-abortable blocking producers,
  and terminal acknowledgements are emitted only after the owned work is
  reaped.
- The aggregate dial observer is default-off, wire-neutral, and policy-inert.
  Sample events carry raw inputs and one exact sample reason; lifecycle events
  separately carry pending/settlement state. Final logical membership and peak
  observed membership are distinct fields.

## What this replaced

Cutover is DONE (otp-10c-2, 2026-07-11): `Push`, `PullSync`, their
message choreographies, and the four per-direction drivers are
deleted from the proto and the tree — no bridge (D-2026-07-05-2).
`DelegatedPull` survives as trigger + progress relay only (no payload
bytes; proof recorded in the otp-10c-2 finding doc).

Progress: otp-3 landed the role-parameterized drivers over the
in-process transport; **otp-4a** made the daemon serve `Transfer` for
real (runs `run_destination` as Responder; a client `run_source`s as
SOURCE initiator over gRPC, in-stream carrier) — the RPC no longer
returns `UNIMPLEMENTED`. The TCP data plane grant + resize land at
otp-4b; the daemon-as-SOURCE (pull-equivalent) layout at otp-5.
