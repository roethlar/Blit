# w4-5-supports-cancellation-flip — CancelJob works on attached Push/PullSync

**Branch**: `master`
**Commit**: `05a8b39` + review fix `1708075`
**Source**: D-2026-07-04-3 (owner: "flip it"), queued as the
`w4-5-supports-cancellation-flip` row in REVIEW.md; anticipated by the
w4-3 finding doc's "Deliberately out of scope" note (now annotated as
superseded).

## What

`CancelJob` dispatch policy stops refusing attached Push/PullSync jobs.
`ActiveJobKind::supports_cancellation` — the single policy bit both
`ActiveJobs::cancel` and `cancel_authorized` gate on — now returns true
for every kind that can hold an active row (Push, PullSync,
DelegatedPull) and false only for history-only `Pull` (RPC deleted in
ue-r2-1h; rows of that kind survive only in recents rehydration, so no
active `Pull` row can exist).

Behavior change: `blit jobs cancel <id>` (and the TUI `K` / `Shift+X`)
against an attached push/pull_sync now fires the row token; the w4-3
dispatcher race tears the handler down and sends the still-connected
client a terminal `Status::cancelled`. The CLI contract for those kinds
moves from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on
success; the TUI's Unsupported footer stops occurring for them. The
0/1/2 outcome→exit mapping itself is unchanged, and the
Unsupported/FailedPrecondition arm survives end-to-end as the contract's
escape hatch (gated `Pull`, older daemons).

## Approach

Policy-only, exactly as w4-3 set up: the capability (token race,
teardown, client notification) landed in w4-3 and was
production-unreachable for the streaming kinds; this slice flips the one
predicate and updates every comment/doc surface that encoded the old
policy. No dispatcher, CLI, or TUI logic changes — a workflow sweep over
prod code, tests, and docs confirmed the TUI has no kind-based cancel
gating and the CLI mapping is outcome-based, so the daemon-side flip is
the entire behavior change.

- `active_jobs.rs`: `supports_cancellation` →
  `!matches!(self, ActiveJobKind::Pull)`; policy rustdoc rewritten per
  D-2026-07-04-3 (the "disconnecting IS the cancel" rationale recorded
  as superseded); `CancelOutcome::Unsupported` doc now describes the
  Pull-only gate as an escape hatch.
- `service/core.rs`: `cancel_job`'s FailedPrecondition message no longer
  claims "CLI is in the byte path; cancel from the originating client" —
  it is now kind-neutral. `resolve_streaming_outcome`'s "armed but
  production-unreachable" rustdoc updated: the token arm is live.
- `proto/blit.proto`: the CancelJob RPC comment — the wire-contract doc
  of record and the last flat statement of the old policy — rewritten
  (which kinds cancel, what FAILED_PRECONDITION now means).
- `blit-app/src/admin/jobs.rs`: `CancelJobOutcome::Unsupported` variant
  doc updated (kept for contract completeness).
- `blit-cli/tests/jobs_lifecycle.rs`: module header's "before changing
  cancellation" framing annotated — the change landed here.
- `.review/findings/w4-3-daemon-disconnect-racing.md`: supersession
  annotation added above the out-of-scope note D-2026-07-04-3 names.

## Files

- `crates/blit-daemon/src/active_jobs.rs` (policy bit + docs + tests)
- `crates/blit-daemon/src/service/core.rs` (message + docs + tests)
- `crates/blit-app/src/admin/jobs.rs` (doc only)
- `crates/blit-cli/tests/jobs_lifecycle.rs` (header doc only)
- `proto/blit.proto` (comments only; no wire-shape change)
- `.review/findings/w4-3-daemon-disconnect-racing.md` (supersession note)

## Tests (blit-daemon 168 → 170, macOS count)

Rewritten/added, all deterministic:

- `supports_cancellation_matches_dispatch_policy` — flipped: Push and
  PullSync now assert true; Pull stays false.
- `cancel_fires_token_for_push_and_pull_sync` (replaces the Push/Pull/
  PullSync loop of `cancel_returns_unsupported_for_non_delegated_kinds`)
  — `cancel` returns `Cancelled` and fires the row token for both
  streaming kinds.
- `cancel_returns_unsupported_for_history_only_pull` — the remaining
  gate: `Unsupported`, token not fired.
- `cancel_authorized_bypass_and_precedence` — the Unsupported-precedes-
  authz pin re-anchored on a `Pull` row (Push no longer exercises the
  policy gate), plus a new assertion that the flipped kinds hit the
  authz check: a different-host caller cancelling a Push row gets
  `Unauthorized`, not `Unsupported`.
- `cancel_job_ok_for_push_and_pull_sync` (RPC-handler level; replaces
  `cancel_job_failed_precondition_for_non_delegated_kind`) — CancelJob
  returns OK echoing the id and fires the token — the daemon side of
  the CLI's new exit-0 path.
- `cancel_job_failed_precondition_for_history_only_pull` — the handler
  still maps the gated kind to `FailedPrecondition` without firing the
  token.

Contract links already pinned elsewhere (unchanged): outcome→exit
mapping incl. exit 0 on OK (`cancel_exit_code_maps_each_outcome_to_the_
contract_code`, `cancel_of_active_delegated_job_exits_zero` e2e);
token→teardown+client-notify (`streaming_canceljob_resolves_pending_
handler_and_notifies_client` and kin, w4-3).

**Mutation verification**: reverting the flip (restoring
`matches!(self, ActiveJobKind::DelegatedPull)`) fails exactly the four
rewritten policy/dispatch tests (4 failed / 15 passed under the `cancel`
filter); restoring it turns all 19 green again. Details in
`.review/results/w4-5-supports-cancellation-flip.gpt-verdict.md`.

## Known gaps

- No end-to-end test drives `blit jobs cancel` against a live attached
  push/pull_sync mid-flight (spawned daemon + second client). Parking a
  streaming handler deterministically mid-transfer needs a test seam or
  timing assumptions the daemon-spawn e2e family is already flaky under
  (w9-3); the pin chain above (policy bit → dispatch → RPC handler →
  kind-agnostic exit mapping → w4-3 teardown tests) covers every link
  the e2e would compose. Same evidence shape w4-3 was graded on.
- The TUI relies on the daemon's FailedPrecondition message for its
  Unsupported footer; that message is now generic ("dispatch policy
  does not support cancellation") rather than actionable advice. It is
  production-unreachable from current daemons, so no UX text was added.
