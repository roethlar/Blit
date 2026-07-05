# otp-4 — daemon serves `Transfer`, client initiates as SOURCE

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-4.
**Status**: otp-4a implemented + reviewed — codex 1/1 finding accepted
and fixed (`.review/results/otp-4a-daemon-serves-transfer.gpt-verdict.md`).
otp-4b (data plane + resize + sf-2 pin) pending.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-3 (`85bf611`) — drivers exist; this slice adds the
gRPC transport, the daemon handler, and the client SOURCE entry.

## Staging

- **otp-4a (this commit)**: serve + initiate over the **in-stream
  carrier**. Real daemon runs `run_destination` as Responder; real
  client runs `run_source` as SOURCE initiator over gRPC. A/B parity
  vs old push. No data plane, no resize.
- **otp-4b (next)**: TCP data plane grant in SessionAccept, source
  dials + authenticates, ported `maybe_shape_resize` (frames 16/17),
  sf-2 10k-file >1-stream pin ported to the session, cancel-mid-
  transfer test.

## What otp-4a proves

The push-equivalent rides the unified session end to end: the daemon
`Transfer` RPC serves a real session (no longer UNIMPLEMENTED), and a
client pushes a tree through it that lands **byte-identically** to the
old push path with **equal summary counters** — the converge-up bar,
in-stream. Responder refusals (read-only, unknown module) arrive as
`SessionError` frames.

## Approach (as implemented)

- **gRPC transport** (`transfer_session/transport.rs`): `GrpcFrameRx`
  over `tonic::Streaming` (`message()` maps 1:1 to the `FrameRx`
  clean-close/error contract); `GrpcClientFrameTx` (outbound item =
  bare `TransferFrame`, fed to `BlitClient::transfer` via a
  `ReceiverStream`) and `GrpcDaemonFrameTx` (outbound item =
  `Result<TransferFrame, Status>`, the response stream). Public
  assemblers `grpc_client_transport` / `grpc_daemon_transport`.
  Capacity 32 (matches push); backpressure via the channel + HTTP/2
  flow control.
- **Responder-resolution API** (`transfer_session/mod.rs`): a
  Responder can't know its write root until the `SessionOpen` arrives
  mid-handshake, so `establish()` gains an async `OpenResolver`
  (`Option<&OpenResolver>`) consulted on the Responder branch *after*
  `validate_open` and *before* `SessionAccept` — a refusal replaces
  the accept, never follows it. `run_destination`'s 3rd param becomes
  `DestinationTarget::{Fixed(root), Resolve(resolver)}`; the resolved
  root (Responder+Resolve) wins, else the Fixed root. Read-only is
  enforced by establish for a DESTINATION responder
  (`SessionFault::read_only` → `SessionError{READ_ONLY}`); a SOURCE
  responder (otp-5) will not refuse read-only. New public surface:
  `ResolvedEndpoint`, `OpenResolver`, `DestinationTarget`,
  `SessionFault::refusal` (caller picks the wire code — blit-core
  stays `tonic::Status`-free).
- **Client entry** (`remote/transfer/session_client.rs`):
  `run_push_session(endpoint, source, PushSessionOptions)` builds the
  `BlitClient` (same bounded-connect as `RemotePushClient::connect`),
  opens the bidi RPC, assembles the client transport, and runs
  `run_source` as SOURCE initiator. Not wired to CLI verbs (otp-10).
- **Daemon handler** (`service/transfer.rs` + `core.rs::transfer`):
  `core.rs::transfer` mirrors `push` — register an `ActiveJobs` row,
  spawn, race the session against cancel/hangup via
  `resolve_streaming_outcome`, return the `ReceiverStream`.
  `service/transfer.rs` owns the daemon-specific pieces:
  `make_open_resolver` (wraps the push Header sequence —
  `resolve_module` → path validation → F2 `resolve_contained_path` —
  mapping `tonic::Status`→`SessionFault`) and `run_transfer_session`
  (assembles the transport, runs `run_destination` with the resolver,
  maps the outcome to `Result<(), Status>` for the jobs record).

## Compare-semantics decision (the SizeMtime fork, resolved)

The one push/pull compare divergence is **same-size + destination
NEWER**: old push clobbers (re-transfers), old pull + the session
safely SKIP. The session already inherits the pull-style safe arm
(`manifest::compare_file` Default). This slice **keeps the safe skip**
and encodes it in the parity pin. Justification (agent-verified, full
evidence in the workflow journal):

- **Direction-invariance (owner's prime invariant)** is satisfied the
  moment there is one predicate; "byte-identical to BOTH old
  directions" is *impossible* in this cell because old push and old
  pull themselves disagree — a choice is forced.
- **Converge UP** (plan constraint) says pick the better direction;
  not clobbering a newer destination on a plain sync is the data-safe
  behavior. `--force` (`CompareMode::Force`) still overwrites.
- **Zero blast radius**: keeps the shared arm untouched, so live
  `pull_sync` is unchanged pre-cutover and the existing pin
  (`manifest.rs` `test_target_newer_unchanged`) stays green. No test
  pins old push's clobber behavior (`file_requires_upload` has none).

Pinned by `same_size_newer_destination_is_skipped_not_clobbered`
(dest keeps its newer same-size file; a stale file still updates).

**⚠ Owner ack requested (narrow)**: the plan's "byte-identical trees
vs old push" acceptance wording is not literally achievable in this
one cell without violating direction-invariance + data safety. The
session encodes the safe skip there; `--force` is the overwrite escape
hatch. Flagged as a STATE open question — not blocking (constrained by
converge-up + data safety), reversible (one compare-mode line),
codex-reviewed. If the owner wants old-push clobber semantics as the
unified default instead, it's a small change.

## Files

- `crates/blit-core/src/transfer_session/transport.rs` (gRPC adapters)
- `crates/blit-core/src/transfer_session/mod.rs` (resolver API)
- `crates/blit-core/src/remote/transfer/session_client.rs` (new) +
  `remote/transfer/mod.rs` (export)
- `crates/blit-daemon/src/service/transfer.rs` (handler helpers;
  replaces the otp-1 UNIMPLEMENTED pin)
- `crates/blit-daemon/src/service/core.rs` (`transfer` handler)
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` (new) +
  `service/mod.rs` (module decl)
- `crates/blit-core/tests/transfer_session_roles.rs` (call sites moved
  to `DestinationTarget::Fixed`)

## Tests

Suite 1501 → 1508 (+7 net: removed the 1 UNIMPLEMENTED pin, added 3
`status_to_fault` unit tests + 5 e2e). New e2e (real loopback daemon):
- `session_lands_bytes_and_scores_them` — a session lands the tree
  byte-identically and scores files/bytes.
- `old_push_and_session_produce_identical_trees_and_counts` — **A/B
  parity**: same fixture through OLD push (rides the TCP data plane)
  and the NEW session (in-stream) → byte-identical trees + equal
  shared summary counters.
- `read_only_module_refuses_the_session` — `READ_ONLY` fault, no bytes
  land. **Guard-proven**: neutering the establish read-only check
  makes it fail; restored, it passes.
- `unknown_module_refuses_the_session` — `MODULE_UNKNOWN` fault.
- `same_size_newer_destination_is_skipped_not_clobbered` — the compare
  decision.

Gate: `cargo fmt --check` ✓, `clippy --workspace --all-targets
-D warnings` ✓, `cargo test --workspace` 1508/0 ✓.

## Known gaps (carried into otp-4b / later)

- **Data plane**: otp-4a is in-stream carrier only; TCP grant,
  socket auth, resize, and the ported sf-2 pin are otp-4b.
- **Cancel** (frame fixed post-review, codex F1): `CancelJob` now
  emits a framed `SessionError{CANCELLED}` on the response stream
  (`core.rs::resolve_transfer_session_outcome` +
  `blit_core::transfer_session::session_error_frame`), unit-guarded by
  `transfer_cancel_emits_framed_cancelled_error`. The deterministic
  **mid-transfer** cancel e2e (fire CancelJob while bytes flow, assert
  the client surfaces `SessionFault{CANCELLED}`) is still an otp-4b
  item — it needs the data plane + a long-enough transfer.
- **Jobs-row endpoint**: the row registers with an empty module/path
  (like push before its Header); populating it from the SessionOpen
  needs a small ActiveJobs affordance — deferred. Row still supports
  CancelJob + GetState presence. Reuses `ActiveJobKind::Push`.
- **Progress bytes**: `with_byte_progress` not threaded (the sink is
  internal to `run_destination`); session rows report
  `bytes_completed=0`, same as today's push rows — no regression.
- **Daemon-as-SOURCE (pull-equivalent)** + the four-layout responder
  dispatch: otp-5. A DESTINATION-declaring initiator (daemon would be
  SOURCE) is currently refused by establish's role-complement check
  (PROTOCOL_VIOLATION), which otp-5 replaces with the real path.
- Mirror/filters otp-6; resume otp-7; fallback-carrier parity otp-8;
  delegated otp-9.
