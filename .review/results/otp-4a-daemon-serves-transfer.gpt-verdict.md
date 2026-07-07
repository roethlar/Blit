# otp-4a — codex review adjudication

**Commit reviewed**: `4b07bbb`
**Raw output**: `.review/results/otp-4a-daemon-serves-transfer.codex.md`
**reviewer: gpt-5.5** (codex exec, read-only, superpowers plugin disabled)
**Codex verdict**: NEEDS FIXES — 1 finding (Medium). Everything else
validated: gRPC bidi startup (no deadlock), resolver ordering (refusal
replaces SessionAccept), F2 containment on the resolved path,
`tonic::Status` not leaking into blit-core, the SizeMtime skip pin,
the DESTINATION-declaring-initiator refusal, and 1501 → 1508 test
accounting.

## F1 — cancel emits `Status::cancelled`, not a `SessionError{CANCELLED}` frame (Medium)

**Claim** (`core.rs:394`): the `Transfer` handler routes cancel through
`resolve_streaming_outcome`, which on `CancelJob` sends
`Status::cancelled` — a bare gRPC status — instead of the contract's
`SessionError{CANCELLED}` frame. The client reads framed errors, so it
sees a transport error / INTERNAL path, not a clean framed cancel. The
finding doc parked this as an otp-4b gap, but the RPC is served now, so
it is a live contract miss.

**Adjudication: Accepted.** Verified against `docs/TRANSFER_SESSION.md`
§Errors ("the peer receives `SessionError{CANCELLED}`") and
`core.rs` `resolve_streaming_outcome` (sends `Err(Status::cancelled)`
on the cancel branch). Correct call: the deferral was defensible for
the deterministic *mid-transfer cancel test* (which needs a
long-running transfer and is fiddly to make deterministic — that stays
an otp-4b item), but NOT for the frame emission itself, which is a
small, contained fix and a real contract obligation on the served
path.

**Fix**:
- New public `blit_core::transfer_session::session_error_frame(code,
  message)` — single owner of the frame grammar builds the wire
  `SessionError` frame (the aborted session future can't send it
  itself once the select drops it).
- New `core.rs::resolve_transfer_session_outcome` — identical to
  `resolve_streaming_outcome` except the cancel branch emits
  `Ok(session_error_frame(CANCELLED, …))` on the response stream
  instead of `Err(Status::cancelled)`. The handler calls it in place
  of `resolve_streaming_outcome`. Hangup + fault + completion handling
  unchanged (a session fault already framed itself; the trailing
  `Status` is kept as belt-and-braces for a pre-frame transport break).
- Guard test `transfer_cancel_emits_framed_cancelled_error`
  (`core.rs` tests): a pre-fired cancel token + a never-completing
  session future must land an `Ok` `SessionError{CANCELLED}` frame on
  the response channel and record "cancelled via CancelJob". Guard
  proven by revert: with the cancel branch reverted to
  `Err(Status::cancelled)` the test fails (no `Ok` error frame).

The full deterministic **mid-transfer** cancel e2e (fire CancelJob
while bytes are in flight, assert the client surfaces a
`SessionFault{CANCELLED}`) remains an otp-4b item — it needs the data
plane and a long-enough transfer to cancel mid-stream. The frame path
is now correct and unit-guarded.

**Fix sha**: `25f538b` (gate re-run: fmt + clippy clean, workspace
suite 1508 → 1509 passed / 0 failed).
