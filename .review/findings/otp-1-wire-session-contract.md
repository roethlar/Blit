# otp-1 — Unified Transfer session: wire + session contract

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-1.
**Status**: implemented, codex review pending.

## What

The complete wire surface of the single `Transfer` session that
replaces `Push` and `PullSync` at cutover — contract doc, proto, and
compiled-but-refusing stubs. No behavior: the daemon handler returns
UNIMPLEMENTED, pinned by test.

## Approach

- **`docs/TRANSFER_SESSION.md`** (new, the authoritative contract):
  role-tagged single frame vocabulary (`TransferFrame` both wire
  directions — no push-shaped or pull-shaped message set exists to
  diverge); exact-match same-build handshake as the FIRST frame each
  way (D-2026-07-05-2 — `build_id` + `contract_version`, mismatch →
  `SessionError{BUILD_MISMATCH}` naming both ids); phase state machine
  (hello → open/accept → concurrent manifest/need/payload → closing)
  with fail-fast `PROTOCOL_VIOLATION` on phase misuse; diff owner =
  DESTINATION always; dial contract carried (D-2026-06-20-1/-2:
  receiver capacity travels DESTINATION→SOURCE in open OR accept
  depending on who holds the role; absent/0 = unknown hardware, never
  "old peer"); sf-2 shape correction named as the only stream policy,
  SOURCE is resize controller in every session; transport facts
  (responder binds, initiator dials; in-stream carrier as byte-carrier
  option; local = in-process frame channel); resume RELIABLE exception
  (per-file `NeedEntry.resume` → destination `BlockHashList` strictly
  before that file's bytes); mirror destination-local (no delete list
  crosses the wire); error/cancel/StallGuard/jobs semantics.
- **`proto/blit.proto`**: `rpc Transfer(stream TransferFrame) returns
  (stream TransferFrame)` + `TransferRole`, `SessionHello`,
  `SessionOpen`, `SessionAccept`, `DataPlaneGrant`,
  `NeedEntry`/`NeedBatch`/`NeedComplete`, `SourceDone`,
  `TransferSummary` (one summary shape, DESTINATION→SOURCE),
  `SessionError` (structured refusal codes incl. BUILD_MISMATCH), and
  the 20-arm `TransferFrame` oneof reusing the engine's existing
  payload vocabulary verbatim (`FileHeader`, `FileData`, `TarShard*`,
  `Block*`, `DataPlaneResize`/`Ack`, `CapacityProfile`, `FilterSpec`,
  enums). Deliberately absent: `PeerCapabilities`, `spec_version`
  negotiation, delete lists, any per-direction message.
- **Stubs** (mechanical, required by tonic's non-optional trait
  methods): `BlitService::transfer` → UNIMPLEMENTED with a pointer to
  the plan; the five test fakes (remote_remote ×2, jobs_lifecycle,
  pull_sync_with_spec_wire ×2) gain the same refusing stub.
- **Future home staked**: `crates/blit-daemon/src/service/transfer.rs`
  holds the pin test now and becomes the session module at otp-4.

## Files

- `docs/TRANSFER_SESSION.md` (new — contract)
- `proto/blit.proto` (Transfer RPC + session messages; Push annotated
  with its otp-10 deletion notice)
- `crates/blit-daemon/src/service/core.rs` (stub handler)
- `crates/blit-daemon/src/service/transfer.rs` (new — pin test +
  future session home), `service/mod.rs` (registration)
- `crates/blit-cli/tests/remote_remote.rs`,
  `crates/blit-cli/tests/jobs_lifecycle.rs`,
  `crates/blit-core/tests/pull_sync_with_spec_wire.rs` (fake stubs)

## Tests

Suite 1483 → **1484 passed / 0 failed** (37 suites, same 2 ignored);
fmt + clippy clean.

- `transfer_rpc_exists_and_refuses_unimplemented` (in-process real
  service + real generated client): the RPC is reachable on the wire
  and refuses with UNIMPLEMENTED — not UNKNOWN — until otp-3/otp-4.
  Guard shape: if the RPC left the proto this test does not compile;
  if the stub's refusal contract changed it fails.

## Known gaps

- The handshake's `build_id` composition (version + git sha + dirty
  flag) is specified in the contract but not yet emitted by any build
  script — that lands with the first session behavior (otp-3), which
  is also when the mismatch-refusal test becomes writable.
- `TransferSummary` unifies Push/PullSummary on the wire; the CLI
  rendering migration happens at cutover (otp-10), not before.
- Frame table field numbers are frozen by the doc from this slice on;
  any change before cutover is a contract change and re-enters review.
- The contract's in-stream carrier reuses `FileHeader file_begin`
  framing; exact per-frame chunk sizing stays dial-owned (w2-2) and
  is not a wire constant.
