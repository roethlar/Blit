# otp-9b — the delegated transfer rides the unified session

**What**: The otp-9 core: a daemon receiving a delegated request
becomes an initiator of the same session against the other daemon
(plan §Design, Delegated transfer). `run_delegated_pull`'s validation
front half (locator parse, spec validation, delegation gate, module
resolution + opt-outs, containment, metrics) is untouched; everything
behind it — the bespoke old-driver body — is replaced by a
DESTINATION-initiator `Transfer` session. The `DelegatedPull` RPC is
now trigger + progress relay only.

**Approach**:

- **Reroute** (`delegated_pull.rs`): the handler connects with
  `connect_transfer_client` (same bounded 30 s policy the audit-1
  timeout provided — the old `SOURCE_CONNECT_TIMEOUT`/`net_timeout`
  wrapper retired), emits `Started`, maps the validated wire spec onto
  `PullSessionOptions` (compare/ignore-existing/require-complete-scan/
  filter/resume verbatim; `force_grpc` → the session's in-stream
  carrier; mirror through the same `delete_list_authorized` gate), and
  runs `run_pull_session_with_client` with the row's
  `ByteProgressSink` (otp-9a) so the jobs row stays live. The
  destination-computed session summary maps onto the wire
  `DelegatedPullSummary` (`in_stream_carrier_used` →
  `tcp_fallback_used`, the field's historical meaning).
- **By-construction wins**: the pre-enumerated local manifest dies
  (the session diffs incrementally — the pull fast-start applies to
  delegation too); the source-attested delete list dies (mirror runs
  locally via otp-6b's one delete rule, so R30-F1/R32-F1/R34-F1 hold
  structurally — this end deletes and reports only what its own
  scan-complete-guarded diff decided); the R25-F2/ue-r2-1b
  capabilities + receiver-capacity overrides die (`run_destination`
  advertises THIS end's own capacity; same-build peers make
  capability bits moot, D-2026-07-05-2).
- **Open-refusal phasing** (`session_client.rs`): a peer refusing the
  `Transfer` RPC at open is a negotiation failure, not a generic
  transport error — `transfer_open_refusal` maps the gRPC status onto
  the `SessionFault` code the same refusal would carry as a frame
  (Unimplemented → BUILD_MISMATCH, PermissionDenied →
  DELEGATION_REFUSED), so the handler phases refusals structurally
  (NEGOTIATE) exactly as the old typed `PullSyncError` boundary did
  (R37-F1). Applied to both push and pull session clients.
- **Cancellation unchanged**: core.rs still races the handler future
  against `tx.closed()` and the row's CancelJob token; dropping the
  session future tears the transport down and the source daemon's
  served session cleans up (its own row + AbortOnDrop).
- **Retired with the old body** (+ their tests, called out below):
  `dst_capabilities`, `apply_dst_capabilities_override`,
  `apply_delete_list`, `build_summary`, `enumerate_local_manifest`.
  `validate_spec`, `delete_list_authorized`, `err_progress`, and the
  gate stay. The `RejectingPullSyncBlit` CLI-test fake now models its
  ACL refusal on the `Transfer` surface (where the dst daemon actually
  knocks post-reroute).

**Files**:

- `crates/blit-daemon/src/service/delegated_pull.rs` — the reroute +
  retirements.
- `crates/blit-core/src/remote/transfer/session_client.rs` —
  `run_pull_session_with_client` split, `connect_transfer_client` pub,
  `transfer_open_refusal`.
- `crates/blit-daemon/src/service/delegated_session_e2e.rs` — NEW:
  two-daemon in-process e2e (below).
- `crates/blit-daemon/src/service/{mod,transfer_session_e2e}.rs` —
  module registration; tree helpers shared `pub(crate)`.
- `crates/blit-cli/tests/remote_remote.rs` — the fake source's refusal
  moved to the Transfer surface (contract unchanged: NEGOTIATE wording,
  no relay fallback).

**Tests** (suite 1558 → 1552: −9 retired with their helpers, +3 e2e —
see Known gaps):

- `delegated_transfer_rides_the_session_and_lands_bytes` — two real
  in-process daemons, `DelegatedPull` driven as the CLI drives it:
  byte-identical landing, Started→Summary ordering, dst-authoritative
  counts, data-plane default.
- `delegated_mirror_purges_extraneous_locally` — plain copy never
  deletes (entries_deleted 0, extraneous survives), mirror ALL purges
  locally (entries_deleted 1, trees identical).
- `delegated_force_grpc_rides_the_in_stream_carrier` — `force_grpc`
  maps to the in-stream carrier, surfacing on the wire-compat bit.
- The full `remote_remote.rs` CLI suite stays green over the reroute —
  including the load-bearing no-CLI-byte-path isolation pin and the
  negotiation-refusal/no-relay-fallback pin.
- Guard proofs by temporary mutation, all run live: (a) dropping the
  mirror/force_grpc option mapping fails both new e2es; (b) reverting
  `transfer_open_refusal`'s PermissionDenied mapping fails the CLI
  negotiation-wording pin. Restored; all green.

**Known gaps**:

- **Test count drops 1558 → 1552** (workspace total): the 9 unit tests
  of the five retired helpers die with them (4 × dst_override, 3 ×
  build_summary, 2 × apply_delete_list). Their contracts either moved
  into the session where they are already pinned (deletion containment
  → the session mirror pass's canonical-containment pins; summary
  authority → the session scorer + the new e2es) or ceased to exist
  (capability override). Called out per the verification rule.
- **Checksum compare degrades to transfer-for-verification**: the old
  path pre-enumerated dest checksums for the source's diff; the
  session's destination diff has no local-checksum computation yet, so
  `--checksum` delegated transfers re-copy unchanged files (safe,
  byte-identical outcome, no skip optimization). Session-wide gap, not
  delegated-specific; lands with the otp-10 verb wiring or a dedicated
  follow-up.
- The dst row's live byte count is wired (otp-9a sink) but not pinned
  mid-flight (the row drains at completion); the counter/summary
  equality is pinned at the session level (otp-9a).
- `trace_data_plane` on the request was already unused by the old body
  and remains so; `DelegatedPullStarted.stream_count` stays 0
  (informational, matches old behavior).
- The bespoke delegated-pull DRIVER machinery inside
  `remote/pull.rs` (`pull_sync_with_spec` and friends) still exists —
  the CLI's plain pull and the relay still use it; it dies at otp-10
  with the other old paths. This slice deleted the delegated-specific
  helpers only.
