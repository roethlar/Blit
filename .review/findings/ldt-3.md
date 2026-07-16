# ldt-3 — lifecycle and observer closure

**Slice**: `LIVE_DIAL_TUNING` ldt-3. Close terminal resize races, own and join
all transfer helpers, and expose a default-off aggregate dial observer whose
sample and lifecycle records are exact without changing policy or the wire.

**Status**: Review-fix candidate — the one admitted Low observer-ordering
defect is fixed and mutation-proved; neutral re-review is pending.

**Branch**: `master`

**Commit**: `436e1bb5f29ca9ea1dece6eb2c5656a63bce7564` + review fix pending

## What

The ldt-2 controller supplied role-invariant adaptive membership, but terminal
`NeedComplete`, error, and cancellation paths did not yet prove that an
accepted resize completes honestly or that every spawned scan, tuner,
pipeline, nested worker, and receive task is reaped before return. The observer
also needed one exact, default-off schema with distinct raw samples, lifecycle
settlement, final membership, and peak membership.

## Approach

- Keep the SOURCE membership endpoint and terminal ledger alive until the one
  in-flight proposal is classified. An unaccepted proposal settles unchanged;
  an accepted ADD completes admission and normal END, while an accepted REMOVE
  retires the exact member or observes its prior normal end.
- Give SOURCE and DESTINATION data planes explicit finish/abort-and-join
  ownership. Error and cancellation epilogues reap the tuner, elastic pipeline,
  nested workers, accepted sockets, and every destination receive task before
  returning the original fault.
- Return an owned `SourceScan` task chain from scanning/decorator setup. Reap
  helpers in reverse creation order and close the bounded header receiver before
  joining a non-abortable blocking producer on manifest failure.
- Add cooperative pipeline Finish/Abort commands whose acknowledgements happen
  only after forwarders and nested workers are reaped. Retain a cancellation-
  safe owner when a join future is dropped.
- Emit observer snapshots only when enabled. A sample carries coherent raw
  counters and exactly one sample reason; pending and settlement events use a
  separate lifecycle taxonomy. Capture settlement and peak under the epoch
  mutex, and report peak separately from final logical membership.
- Extend the existing session-phase JSON additively under schema version 1.
  There is no proto or wire change.

## Files changed

- `crates/blit-core/src/dial.rs` — raw dial samples, exact sample/lifecycle
  reasons, optional observer, synchronized settlement, and distinct peak count.
- `crates/blit-core/src/remote/transfer/{abort_on_drop,pipeline,source}.rs` —
  borrowed join ownership, cooperative teardown, nested-worker cleanup, and the
  ordered `SourceScan` task chain.
- `crates/blit-core/src/remote/transfer/{session_client,session_phase}.rs` —
  additive observer event fields and session plumbing.
- `crates/blit-core/src/transfer_session/{data_plane,local,mod}.rs` — common
  observer mapping, terminal resize settlement, explicit SOURCE/DESTINATION
  cleanup, scan ownership, final/peak reporting, and role-invariant guards.
- `crates/blit-core/tests/transfer_session_roles.rs` and
  `crates/blit-daemon/src/service/transfer_session_e2e.rs` — signature updates
  and bounded cleanup coverage.
- `docs/TRANSFER_SESSION.md`, active plans/state/review records, and historical
  exact-eight records — lifecycle contract, candidate status, and correction
  from adaptive claims to pre-ldt-2 static-target orientation evidence.

## Tests and guard proof

- Terminal proposal guards cover ready-but-unsent ADD and REMOVE plus accepted
  ADD and REMOVE after `NeedComplete` in both initiator layouts. Removing the
  terminal event-loop continuation made the accepted case report final 5
  instead of 4; restoration passes.
- Dial observer guards pin all ten exact sample labels (`idle`, `rebaseline`,
  `hysteresis`, `cheap-up`, `cheap-down`, `sustain`, `cooldown`, `bound`, `add`,
  `remove`) and lifecycle labels (`pending`, `add`, `remove`, `refused`). Label
  mutations for sustain, REMOVE sample, and REMOVE settlement each failed.
- Observer guards require OFF/ON transfer-result parity, no OFF snapshot work,
  coherent raw byte/blocked/elapsed/stream fields whose ratio recomputes, and
  peak membership distinct from final. Disabling attachment, zeroing injected
  raw counters, and removing peak updates each failed before restoration.
- `settlement_observer_precedes_waiter_notification` parks the settlement
  observer callback and proves a registered waiter remains blocked until that
  callback returns. Moving notification back under the epoch mutex makes the
  guard fail immediately; exact restoration passes.
- SOURCE fault and cancellation guards require both layouts to record exact
  pipeline/tuner/receive cleanup without a false resize settlement. Removing
  SOURCE scan cleanup, DESTINATION abort accounting, or receive-sibling drain
  made the corresponding guard fail.
- `need_complete_before_manifest_complete_faults_the_source` bounds both the
  refusal and source join. Removing `header_rx.close()` deadlocked the blocking
  manifest producer until the five-second guard failed; restoration completes
  promptly.
- `abort_reaps_downstream_replacement_before_blocked_scan_producer` reproduces
  a non-abortable scan task blocked on its second `blocking_send`. Reaping in
  forward order timed out at five seconds; reverse creation order passes.
- `elastic_abort_waits_for_nested_worker_cleanup` and
  `cancelled_clean_join_retains_cooperative_abort_and_reaps_workers` prove an
  Abort acknowledgement is post-cleanup and that dropping a join future does
  not lose the cleanup owner. Early acknowledgement mutations failed both
  cleanup guards.
- `receive_finish_reaps_siblings_before_returning_first_error` and phase-trace
  assertions pin cleanup even when a receive task is aborted before its future
  first polls. Moving the stop sentinel inside the future made one socket's
  terminal event disappear; restoration passes repeated runs.

## Validation

- Coder mutation proofs above are restored green.
- Exact review-fix validation passes: formatting, strict workspace clippy,
  1,532 workspace tests with 2 ignored, 30 focused release dial tests,
  documentation checks, and diff checks. The final results are recorded in
  `DEVLOG.md`.

## Known gaps

- Hosted Windows CI is unobserved without a push or pull request and cannot be
  claimed green from this local slice.
- ldt-4 owns the first quiet Mac-to-Mac adaptive evidence. This slice contacts
  no endpoint and makes no performance claim.

## Reviewer comments

Claude Fable 5/max reviewed exact range
`e863ef073698b27519ccda07e8907c053d4cc7df..436e1bb5f29ca9ea1dece6eb2c5656a63bce7564`
under the neutral best-way question and returned schema-valid `findings` with
exact SHAs and `guard_confirmed=true`. One Low is admitted: settlement waiters
are notified before the optional settlement observer event is emitted, so a
woken tuner can theoretically emit the next epoch's pending event first. The
fix moves notification after settlement emission and adds a deterministic
ordering guard; both are now locally green and await fixed-SHA re-review. The
first formal call was a server-side 529 before any reviewer turn; the one
allowed retry produced the authoritative result. Records:
`.review/results/ldt-3-r1.claude.json` and
`.review/results/ldt-3-r1.claude-verdict.md`.
