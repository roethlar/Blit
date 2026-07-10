# otp-9a — pull session-client surface for the delegated reroute

**What**: The first otp-9 (delegated transfer) sub-slice, staged like
4b/5b/6a/6b/7a/7b: before the delegated-pull handler can become an
initiator of the unified session (otp-9b), the pull session client
needs the surface the handler's old driver had — mirror, filter, and a
live byte counter. The session itself has honored mirror and filters
since otp-6 (`destination_open_validator` accepts them; the one delete
rule and the `FilteredSource` chokepoint are pinned in the role
suite); what was missing was purely the CLIENT wiring:
`PullSessionOptions` carried neither, and the DESTINATION session sink
had no `ByteProgressSink` hook (the old drivers all thread one — the
delegated dst daemon uses it to keep its ActiveJobs row's byte count
live).

**Approach**:

- `DestinationSessionConfig.byte_progress: Option<ByteProgressSink>`
  (blit-core `transfer_session/mod.rs`), threaded
  `run_destination` → `drive_destination` → `destination_session`,
  where the session's `FsTransferSink` gains
  `.with_byte_progress(...)` — the sink's EXISTING reporting contract
  (applied payload bytes), no new counting code. The served responder
  path passes `None` (wiring the daemon row's counter through
  `run_responder` stays the core.rs jobs-row follow-up, revisited at
  cutover).
- `PullSessionOptions` gains `filter: Option<FilterSpec>`,
  `mirror_enabled: bool`, `mirror_kind: MirrorMode`,
  `byte_progress: Option<ByteProgressSink>`; `run_pull_session` maps
  the first three onto `SessionOpen` and hands the sink to the config.
  Push options unchanged — the delegated flow is destination-initiator;
  the push-side mirror/filter client wiring belongs to the otp-10 verb
  cutover.

**Files**:

- `crates/blit-core/src/transfer_session/mod.rs` — config field +
  threading + sink hook.
- `crates/blit-core/src/remote/transfer/session_client.rs` — options +
  open mapping.
- `crates/blit-core/tests/transfer_session_roles.rs` — mechanical
  `byte_progress: None` on the 14 config literals.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — the three
  pins below.

**Tests** (suite 1555 → 1558):

- `pull_session_mirror_purges_extraneous_via_client_options` —
  mirror ALL through the options purges an extraneous dest file over a
  daemon-served session; `entries_deleted == 1`; trees identical.
- `pull_session_filter_limits_manifest_via_client_options` — an
  include glob through the options scopes the REMOTE source scan; only
  the matching file lands.
- `pull_session_reports_bytes_against_the_callers_counter` — a
  caller-owned `AtomicU64` sees bytes land and agrees exactly with
  `summary.bytes_transferred`.
- Guard proofs by temporary revert: (a) dropping the open mapping
  fails both option pins (extraneous file survives / both files land);
  (b) disabling the sink hook fails the counter pin at 0. Restored;
  all green.

**Known gaps**:

- The served (responder) destination still reports no bytes to the
  daemon's ActiveJobs row — pre-existing follow-up noted in core.rs's
  `transfer` handler, unchanged here.
- `byte_progress` counts APPLIED payload bytes (the sink contract the
  old drivers share), not wire bytes; skips and mirror deletions do
  not count. Same semantics as the old delegated path's counter.
- Push-side mirror/filter client wiring deliberately deferred to
  otp-10 (no CLI verb rides the session yet).
