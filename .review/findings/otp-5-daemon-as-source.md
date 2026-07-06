# otp-5 — roles swapped: client initiates as DESTINATION (pull-equivalent)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-5.
**Status**: otp-5a implementing — in-stream pull-equivalent (daemon-as-SOURCE
responder, client DESTINATION initiator). otp-5b (data plane) pending.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-4 (daemon serves `Transfer`, client SOURCE initiator). The
role-parameterized state machine (`run_source`/`run_destination`) already runs
BOTH assignments in-process (`transfer_session_roles.rs` exercises source-as-
Responder with a Fixed source). otp-5 adds the *daemon* wiring for the flipped
direction.

## Staging (mirrors otp-4a/4b)

- **otp-5a (this commit)**: in-stream pull-equivalent. The daemon serves the
  same `Transfer` RPC and now DISPATCHES on the declared initiator role — a
  client that declares DESTINATION makes the daemon the SOURCE Responder
  (resolve module→source root, stream its manifest, send payloads); a client
  that declares SOURCE keeps otp-4's behavior unchanged. Client gets
  `run_pull_session` (DESTINATION initiator, in-stream carrier). A/B parity vs
  old `pull_sync`. **No data plane** for the SOURCE responder yet.
- **otp-5b (next)**: the data-plane transport/role decoupling. Today the data
  plane is keyed to ROLE (DEST binds+grants+accepts, SOURCE dials+sends). The
  plan's transport rule is that the **connection-initiating end dials** (NAT
  reality) while **byte direction is by role**. For pull the DESTINATION is the
  *initiator* and must dial; the SOURCE is the *responder* and must bind+grant+
  accept while *sending* bytes. That decoupling (responder-binds vs role-sends)
  is otp-5b.

## What otp-5a proves

The pull-equivalent rides the one unified session end to end: a daemon serving
`Transfer` streams its module tree to a client that initiated as DESTINATION and
wrote it **byte-identically** to what old `pull_sync` produces, with equal
shared summary counters (the converge-up bar), over the in-stream carrier. The
same served RPC still handles push (otp-4) — role is chosen by the client's
`SessionOpen.initiator_role`, never by a second code path.

## Approach (as implemented)

- **Handshake split** (`transfer_session/mod.rs`): `establish` is factored into
  `exchange_hello` (HELLO both ways, exact match — D-2026-07-05-2) and
  `responder_finish` (complement check → validate → resolve → data-plane prepare
  → `SessionAccept`, taking an already-read `SessionOpen`). `establish` keeps its
  old shape for the direct role drivers (the in-process role suite); the split
  lets a serving end read the open, learn the initiator's declared role, and
  only then pick which driver to run.
- **Unified responder** (`run_responder`): the daemon's single serving entry. It
  exchanges HELLO, reads the `SessionOpen`, and dispatches on
  `initiator_role`: initiator SOURCE ⇒ local DESTINATION (existing receive
  path); initiator DESTINATION ⇒ local SOURCE (new send path). It carries a
  `DestinationTarget` and a new `SourceResponderTarget` and uses whichever the
  role selects. Returns `ResponderOutcome::{Destination,Source}`.
- **`SourceResponderTarget`**: `Fixed(Arc<dyn TransferSource>)` (a root known up
  front — tests) or `Resolve(Box<OpenResolver>)` (the daemon: resolve module→
  root via the SAME `OpenResolver` the DESTINATION path uses, then build
  `FsTransferSource::new(root)` inside blit-core — symmetric with how
  `run_destination` builds its sink from `dst_root`). blit-core stays free of
  module/`tonic::Status` types; read-only is ignored for a SOURCE (reading a
  read-only module is fine — the establish read-only refusal is DESTINATION-only,
  already so since otp-4a).
- **Body reuse**: run_source's post-establish body is `drive_source` and
  run_destination's is `drive_destination` (both include the fault-notify
  wrapping). `run_source`/`run_destination`/`run_responder` all call them, so
  the source/destination session loops are single-sourced. `source_send_half`
  now takes `plan_options` + `data_plane_host` directly instead of the whole
  `SourceSessionConfig` (run_responder has no initiator config).
- **Daemon** (`service/transfer.rs`): `run_transfer_session` builds both a
  source and a destination resolver (`make_open_resolver` cloned) and calls
  `run_responder`; both outcome arms map to `Ok(())`/`Err(Status)` for the jobs
  record exactly as before. The `core.rs::transfer` dispatcher is unchanged
  (still `resolve_transfer_session_outcome` + `ActiveJobKind::Push`; a pull
  served by the daemon is still a daemon-side transfer row — kind taxonomy
  revisited at cutover).
- **Client** (`remote/transfer/session_client.rs`): `run_pull_session(endpoint,
  dest_root, PullSessionOptions)` opens the bidi RPC, declares
  `initiator_role = DESTINATION`, and runs `run_destination` as Initiator with
  `DestinationTarget::Fixed(dest_root)`. `in_stream_bytes = true` (otp-5a is
  in-stream only; the SOURCE responder grants no data plane regardless, so the
  carrier is in-stream either way — the flag is set for clarity and forward
  intent). Not wired to CLI verbs (otp-10).

## Compare semantics

Unchanged from otp-4a: the destination is the one diff owner and uses the
mode-aware `header_transfer_status`; the same-size + dest-NEWER cell resolves to
the data-safe SKIP (the still-open owner-ack question from otp-4a; not reopened
here). Old pull already SKIPs that cell, so A/B-vs-old-pull is byte-identical
with no caveat (unlike the push A/B, where old push clobbers).

## Files

- `crates/blit-core/src/transfer_session/mod.rs` — `exchange_hello`,
  `responder_finish`, `drive_source`, `drive_destination`, `run_responder`,
  `SourceResponderTarget`, `ResponderOutcome`; `source_send_half` signature.
- `crates/blit-daemon/src/service/transfer.rs` — `run_transfer_session` via
  `run_responder`.
- `crates/blit-core/src/remote/transfer/session_client.rs` — `run_pull_session`,
  `PullSessionOptions`.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — pull e2e tests.

## Tests

New e2e (real loopback daemon serving as SOURCE):
- `pull_session_lands_bytes_and_scores_them` — the daemon's module tree lands in
  the client's dest byte-identically; summary files/bytes correct;
  `in_stream_carrier_used` true.
- `old_pull_and_session_produce_identical_trees_and_counts` — A/B parity: same
  daemon source through OLD `pull_sync` and the NEW session → byte-identical
  dest trees + equal shared counters.
- `unknown_module_refuses_the_pull_session` — `MODULE_UNKNOWN` fault to a
  DESTINATION initiator.

Guard proof: the daemon dispatch is guarded by
`old_pull_and_session_produce_identical_trees_and_counts` — reverting the
`run_responder` dispatch (leaving `run_destination` unconditional) makes the
daemon refuse a DESTINATION initiator with `PROTOCOL_VIOLATION` (the complement
check), failing the pull tests; restoring passes.

## Known gaps (carried into otp-5b / later)

- **Data plane for the SOURCE responder**: otp-5a is in-stream only. The
  transport/role decoupling (responder binds+accepts while sending; initiator
  dials while receiving) is otp-5b.
- **Source plan_options for the daemon**: `run_responder`'s source path uses
  `PlanOptions::default()` (the SOURCE owns planner knobs; the daemon has no
  client-supplied ones). Matches today's daemon-send defaults.
- Mirror/filters otp-6; resume otp-7; fallback-carrier parity otp-8; delegated
  otp-9; cutover/deletion otp-10.
