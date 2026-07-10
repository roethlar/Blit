# otp-7b-2 — end-of-op fault summary (D4 rider) + cancel-during-resume

**What**: The second (final) otp-7b pass: the D-2026-07-09-1 Q2 owner
rider — a mid-transfer fault surfaces at the END of the operation naming
the affected file(s) with a re-run suggestion — built at the layer that
survives the otp-10 cutover, plus the two items the otp-7a codex review
deferred here: the cancel-during-resume e2e (F4) and, discovered by this
slice's own validation gate, a RELIABLE fix in the resume block write.

**Approach**:

- **Structured file identity on `SessionFault`** (`relative_path:
  Option<String>`), carried on the wire as `SessionError.relative_path
  = 5` so BOTH ends can name the file wherever the fault originated.
  Wire-shape change ⇒ `CONTRACT_VERSION` 1 → 2 (same-build peers,
  D-2026-07-05-2, make this a free change). Never scraped from the
  message.
- **`FaultedPath`** (`remote/transfer/faulted_path.rs`): a typed
  error-chain marker per-file read/write failures attach via eyre
  `wrap_err`; the session's `fault_from_report` lifts it into the
  fault. Attached at: `ResumeBlockDiff` (read errors + EOF-short, both
  carriers), `DataPlaneSink` file/resume-record sends, the receive
  pipeline's mid-block socket read, the in-stream file-record source
  loop, and (as `SessionFault.with_path` / `violation_for`) every
  in-stream record-receive violation and every `NeedListSink` claim
  violation. `tag_path` deliberately never wraps a report already
  carrying a `SessionFault` — wrapping would bury the downcast
  `fault_from_report` depends on.
- **`SessionFault::end_of_operation_summary()`**: the summary block the
  otp-10 verb switch will print — "transfer aborted: <message> /
  affected file: <path> — partial data … re-run the same command to
  converge". `None` when the fault names no file (nothing to converge
  on). Pinned at the session-client/e2e level per the plan's staging
  note; no CLI verb rides the session until otp-10.
- **Cancel-during-resume e2e** (codex otp-7a F4): `CancelJob` fired
  while the resume block phase is provably in progress over the
  daemon-served data plane → client surfaces framed CANCELLED, no hang,
  daemon drains the row (otp-4b-3's shape, stuck source held inside the
  block phase).
- **RELIABLE fix (gate-discovered): flush the resume block write.**
  `write_file_block_payload` returned after `write_all` on a
  `tokio::fs::File` with NO flush — tokio buffers file writes and runs
  them on the blocking pool in the background, so an acknowledged block
  could reach the OS arbitrarily late (or race the finalization
  truncate on its separate handle). Landed in otp-7a; exposed as a
  ~50% full-suite flake of the 7a `mid_resume_source_fault…` pin once
  7b added suite load (the faulted session's already-applied block 0
  was missing from the partial). One `flush().await` before reporting
  the write done; 12 consecutive full-suite runs of the roles suite
  clean after (previously ~50% failure). Bundled into this slice
  commit deliberately — it was found by this slice's gate and sits on
  the same resume write path; called out here for the reviewer.

**Files**:

- `proto/blit.proto` — `SessionError.relative_path = 5`.
- `crates/blit-core/src/transfer_session/mod.rs` — fault field +
  `with_path`/`violation_for`/`tag_path` + `end_of_operation_summary`;
  wire mapping; `CONTRACT_VERSION` = 2; path attachment at record
  sites; unit pin.
- `crates/blit-core/src/transfer_session/data_plane.rs` —
  `NeedListSink` violations carry the path; inner write errors tagged.
- `crates/blit-core/src/remote/transfer/faulted_path.rs` — NEW.
- `crates/blit-core/src/remote/transfer/{resume_diff,sink,pipeline}.rs`
  — FaultedPath attachment; the block-write flush fix.
- `crates/blit-core/tests/transfer_session_roles.rs` — mid-resume pin
  extended: structured path on BOTH ends (wire carry) + summary.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` —
  `mid_resume_cancel_surfaces_cancelled_over_the_data_plane`,
  `mid_resume_fault_names_the_file_in_the_end_of_operation_summary`.

**Tests** (suite 1545 → 1548):

- Unit: `fault_summary_names_the_file_and_survives_the_wire` (summary
  content, None case, wire round trip, eyre lift, SessionFault
  passthrough).
- Roles suite: mid-resume pin now asserts `relative_path` on both ends
  + the summary text (in-stream carrier = wire carry via the error
  frame).
- Daemon e2e: the two tests above (data-plane carrier; local lift on
  the client side; cancel teardown).
- Guard proofs by temporary revert: (a) neutered `fault_from_report`
  lift → unit pin + roles source-side assert FAIL; (b) neutered
  `from_wire` path mapping → roles dest-side (wire-carry) assert FAIL.
  Restored; suite green. The flush fix's guard is the previously-flaky
  7a pin itself: 12/12 clean full-suite runs after, ~50% before.

**Known gaps**:

- The verb-level PRINT of `end_of_operation_summary` lands with the
  otp-10 verb switch (no CLI verb rides the session yet) — exactly the
  plan's staging note.
- One fault = one path: the session aborts on the first fault, so
  "file(s)" is a single file today; a per-file continue-on-error model
  (which could accumulate several) is explicitly out of otp-7 scope
  (plan D4, `LOCAL_ERROR_TELEMETRY.md` is the adjacent draft).
- `mid_resume_fault_names_…` waits out the peer-fault stall window
  (~30 s) before surfacing the local dp error — pre-existing
  `prefer_peer_fault` semantics, not changed here; noted for suite
  wall-time.
- Faults with no file identity (handshake refusals, cancels, mirror
  errors) yield no summary block by design.
