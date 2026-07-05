# otp-1 — codex review adjudication

**Commit reviewed**: `a3e2acb`
**Raw review**: `.review/results/otp-1-wire-session-contract.codex.md`
**Reviewer**: gpt-5.5 (codex exec, read-only sandbox)
**Verdict line**: NEEDS FIXES before otp-3/otp-4 consume the contract
(6 findings)
(fix sha at end of file)

## Findings

1. **High — closing-flow diagram contradicted the role rule —
   ACCEPTED.** The diagram drew `SourceDone`/`TransferSummary` on
   initiator/responder lanes, which reverses them for
   initiator=DESTINATION sessions. Fixed: the diagram now switches to
   explicit ROLE lanes after OPEN/ACCEPT and the closing phase is
   role-directed with a note covering both initiator layouts.
2. **High — `DataPlaneGrant.initial_streams` blurred the sender-owned
   dial — ACCEPTED.** As written, a DESTINATION responder could be
   read as choosing the sender's initial dial. Fixed (doc + proto
   comment): it is an ACCEPT ceiling — the number of armed epoch-0
   slots, min(dial floor, destination capacity ceiling); SOURCE owns
   the dial, may use fewer, unclaimed slots expire; growth only via
   SOURCE-initiated resize.
3. **Medium — socket auth underspecified — ACCEPTED.** Fixed: exact
   handshake specified (epoch-0 sockets present `session_token` then
   `epoch0_sub_token`; ADD sockets present `session_token` then that
   epoch's `sub_token`; one socket per armed slot, no replay,
   unclaimed slots expire, bad credentials closed without response).
4. **Medium — in-stream carrier record grammar missing — ACCEPTED.**
   Fixed: strict record serialization; file completion inferred at
   exactly `header.size` bytes (over/underrun = PROTOCOL_VIOLATION);
   tar/block record shapes named; payload records begin only after
   `ManifestComplete` on this carrier (mirrors the design-4-proven
   fallback ordering; applies identically to both roles).
5. **Medium — backpressure + NeedComplete ordering left to invention —
   ACCEPTED.** Fixed: flow control is gRPC/HTTP-2 stream flow control
   plus the engine's existing bounded queues, named as a contract
   requirement (no unbounded buffering); `NeedComplete` only after
   ManifestComplete received AND all entries diffed.
6. **Low — doc `detail` field doesn't exist in proto — ACCEPTED.**
   Doc text corrected to `{code, message}` + build ids on
   BUILD_MISMATCH.

Nothing rejected, nothing deferred. Gate re-run: fmt/clippy clean,
1484 passed / 0 failed.

reviewer: gpt-5.5
