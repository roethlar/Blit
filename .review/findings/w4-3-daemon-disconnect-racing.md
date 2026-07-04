# w4-3-daemon-disconnect-racing — race push/pull_sync handlers against client hangup + cancel token

**Branch**: `master`
**Commit**: `37d7f91`
**Source**: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W4.3 (ratified
D-2026-06-11-2), finding
`async-daemon-handlers-blind-to-disconnect-in-compute-phases` in
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`.

## What

Only `delegated_pull`'s spawn closure raced its handler against
`tx.closed()` (client hangup, R30-F2) and the row's cancellation token
(m-jobs-1). The `push` and `pull_sync` dispatchers bare-awaited their
handlers, so a client that disconnected during a send-free compute phase
(pull_sync's enumerate+checksum collection is the longest such window;
push's mirror purge is another) left the daemon running the whole
remaining handler for a dead peer — unbounded, unobservable work that
`CancelJob` also explicitly refuses to touch — while the
`supports_cancellation` comment in `active_jobs.rs` asserted "a
client-side cancel already drops the handler future via `tx.closed()`",
a mechanism that did not exist for these kinds.

The audit spec said "all three spawn closures"; the legacy Pull RPC (and
its spawn closure) was deleted at `ue-r2-1h`, so two live sites remained.
`ActiveJobKind::Pull` itself survives only in recents-history
rehydration and tests.

## Approach

- `resolve_delegated_pull_outcome` → `resolve_transfer_outcome<T>`:
  generalized over the handler's output type (was hardcoded
  `Output = bool`), keeping the audit-10 `biased` handler-first select
  and the `detach` gate byte-for-byte. It stays the single owner of the
  three-way race; `delegated_pull`'s call site changes only in name.
- New `resolve_streaming_outcome` (core.rs): wraps
  `resolve_transfer_outcome` for the streaming dispatchers. Races the
  handler against `tx.closed()` + `cancel_token.cancelled()`
  (`detach: false` hardcoded — streaming RPCs have no detach mode), then
  maps the result onto the `(ok, error_message)` pair the ActiveJobs
  ring records:
  - handler completed → `outcome_from_status`; an `Err` is counted
    (`inc_error`) and forwarded to the client — the pre-fix dispatcher
    behavior, unchanged;
  - client hung up → `(false, "client cancelled")`, nothing sent;
  - token fired → `(false, "cancelled via CancelJob")` + terminal
    `Status::cancelled` to the still-connected client. Same
    disambiguation (`is_cancelled()`) and message vocabulary as
    `delegated_pull`'s inline mapping.
- `push` / `pull_sync` spawn closures rewired through the helper; the
  record→build-event→drain→broadcast tail of each closure is unchanged.
  Reverting either site to a bare `handler.await` leaves
  `resolve_streaming_outcome` dead, which `clippy -D warnings` rejects —
  the wiring itself is lint-guarded.
- Comment fixes: `supports_cancellation` rustdoc rewritten as a policy
  statement (dispatch policy vs handler capability, now real, hangup
  race noted, `Pull` marked history-only);
  `cancel_returns_unsupported_for_non_delegated_kinds` test comment and
  the `CancelOutcome::Unsupported` doc updated to match.

## Deliberately out of scope

- **`supports_cancellation` policy unchanged** (DelegatedPull-only).
  The ratified W4.3 text calls for the race + the comment fix, not a
  CancelJob contract change (exit-code 2 / FailedPrecondition / TUI
  Unsupported surfaces). With the token race wired, flipping the policy
  for push/pull_sync is now policy-only — flagged as an open question
  for the owner in STATE.md.
- Because the policy is unchanged, the cancel-token arm is
  production-unreachable for push/pull_sync today (`cancel()` returns
  `Unsupported` without firing the token for these kinds). It is wired
  per the ratified spec text and pinned by a unit test.

## Tests added (blit-daemon 162 → 167)

All deterministic (`ready`/`pending` futures, level-set channels — no
timing):

- `streaming_hangup_resolves_pending_handler_as_client_cancelled` — a
  dropped response `Receiver` resolves a `pending()` handler as
  `(false, "client cancelled")`. Pre-fix shape (bare await) hangs.
- `streaming_canceljob_resolves_pending_handler_and_notifies_client` —
  a fired token resolves a `pending()` handler as
  `(false, "cancelled via CancelJob")` and the client receives
  `Code::Cancelled`.
- `streaming_completed_handler_wins_simultaneous_races` — audit-10's
  completion-beats-simultaneous-cancel guarantee extended to the
  streaming path.
- `streaming_handler_error_recorded_and_forwarded_to_client` — the
  pre-existing dispatcher error path survives the rewire.
- `streaming_handler_success_records_ok_and_sends_nothing`.

The three pre-existing audit-10 tests were renamed-in-place to call
`resolve_transfer_outcome`; their coverage is unchanged.

**Mutation verification** (each arm's guard proven, then the real
implementation restored and the gate re-run green):

- M1 (remove `tx_closed` arm): hangup test hangs (timeout kill); the
  CancelJob test still passes — the hangup test pins that arm.
- M2 (remove `cancelled` arm): the CancelJob streaming test AND the
  original `resolve_pull_pending_handler_yields_to_cancel` hang; the
  hangup test still passes.
- M3 (handler arm ordered last): `streaming_completed_handler_wins_…`
  and the original audit-10 bias test fail on `None`.

Full suite: fmt clean, clippy clean (workspace, all targets,
`-D warnings`), `cargo test --workspace` all green — 37 suites, blit-
daemon 167 (was 162); no other crate's count changed.

## Known gaps

- An in-flight `spawn_blocking` enumeration/checksum batch
  (`pull_sync.rs:1677` and kin) still runs to its natural end when the
  handler future is dropped — the drop stops all further phases but
  can't abort blocking work already on the pool. Making the collect
  phase abortable between rayon batches is the audit's stated follow-up
  slice, not this one.
- `pull_sync.rs:958`'s resize-socket validation tasks remain a bare
  `Vec<JoinHandle>` (short-lived, timeout-bounded accepts). Pre-existing
  shape, unchanged exposure class on error returns; noted for the w4-1
  family's ledger rather than expanded here.
- No end-to-end test drives a real gRPC client disconnect mid-compute:
  parking a handler deterministically in a send-free compute phase
  requires either a filesystem-level blocker (not cross-platform) or a
  test seam in the handler. The helper-level tests plus the
  clippy-guarded wiring cover the contract; the delegated_pull
  precedent (audit-10, R30-F2) was graded on the same evidence shape.
