# m-jobs-1-cancel-token: per-row CancellationToken + delegated_pull race

**Severity**: Feature (no behavior change visible on the wire
yet — the token's listener already exists, but nothing fires
it until m-jobs-2 lands the CancelJob RPC)
**Status**: In progress / pending review
**Branch**: `phase5/m-jobs`
**Commit**: filled by the sentinel commit

## What

First sub-slice of milestone M-Jobs from
`docs/plan/TUI_DESIGN.md` §6.5. Adds the cancellation
plumbing the upcoming `CancelJob` RPC (m-jobs-2) will fire,
and wires `delegated_pull` — the one RPC where daemon-side
cancellation is meaningful — to honor it.

`push`, `pull`, `pull_sync` are intentionally not wired
because the CLI is in the byte path for those; a client-side
cancel already drops the handler future via the existing
`tx.closed()` race. CancelJob from another client doesn't
have a meaningful semantic there.

## Approach

`tokio_util::sync::CancellationToken` is the right primitive:
cheap-to-clone (internal Arc), supports `.cancel()` from one
task + `.cancelled()` awaitable from another, integrates
natively with `tokio::select!`.

The token lives in a parallel `Mutex<HashMap<String,
CancellationToken>>` on `Inner`, keyed by transfer_id.
Parallel rather than embedded in `ActiveJob` because
`ActiveJob` is the wire-shape snapshot row — the token isn't
user-visible.

`register()` mints the token, inserts into both `table` and
`cancellations`, stashes a clone on the returned guard.
`Drop` removes from both. `cancel(id)` looks the token up by
id and fires it.

`delegated_pull`'s spawn closure grows a third arm in the
existing `tokio::select!`: the cancellation token's
`.cancelled()` future. Resolves on either client hangup
(existing `tx.closed()` arm) or `CancelJob` (new
`cancel_token.cancelled()` arm). Same teardown path — the
handler future is dropped, the data plane cleans up via the
existing cancellation chain. The recent-ring record's
`error_message` distinguishes the cause:
`"cancelled via CancelJob"` vs `"client cancelled"`.

## Files changed

- `crates/blit-daemon/Cargo.toml`:
  - `+tokio-util = "0.7"`.
- `crates/blit-daemon/src/active_jobs.rs`:
  - `+Inner.cancellations: Mutex<HashMap<String, CancellationToken>>`.
  - `+ActiveJobGuard.cancellation: CancellationToken`.
  - `+ActiveJobGuard::cancellation_token(&self) -> &CancellationToken`.
  - `+ActiveJobs::cancel(transfer_id) -> bool`.
  - `register()` extended to mint + store the token.
  - `Drop` extended to remove the cancellations entry.
    Lock order doc comment added: table → cancellations →
    recent, held sequentially.
  - Module doc grew the `m-jobs-1-cancel-token` bullet and
    revised the "out of scope" section to enumerate the
    remaining M-Jobs sub-slices.
- `crates/blit-daemon/src/service/core.rs`:
  - `delegated_pull` spawn closure: cloned the token off the
    guard before move, added a new
    `_ = cancel_token.cancelled() => None` arm to the
    select, extended the outcome→error_message match to
    distinguish CancelJob cause from client hangup.

## Tests added

- `active_jobs::tests::cancel_fires_token_for_known_transfer_id` —
  register, capture token, assert not cancelled, call
  `cancel(id)`, assert returns true and token reads as
  cancelled. Second call returns true (idempotent).
- `active_jobs::tests::cancel_returns_false_for_unknown_transfer_id` —
  unknown id returns false. After a guard drains, the
  (now-removed) id also returns false.
- `active_jobs::tests::cancellation_token_wakes_awaiter` —
  spawn a task awaiting `token.cancelled()`, yield, call
  `cancel(id)` from the parent, await the task to join.
  Verifies the handler-shape contract `delegated_pull`
  relies on.

Workspace: 526 passed (was 523; +3).

## Known gaps

1. **No `CancelJob` RPC yet.** The token's listener side
   works — handlers race against it — but nothing fires it
   from over the wire. m-jobs-2 adds the RPC + handler.

2. **No CLI verb yet.** `blit jobs cancel <remote> <id>`
   lands in m-jobs-2 alongside the RPC.

3. **Only `delegated_pull` honors the token.** Push, pull,
   and pull_sync register rows + tokens (for consistent
   shape), but their spawn closures don't `.cancelled()`-race
   them. By design — CLI is in the byte path for those, so
   client-side cancel already drops the handler. Documented
   in the slice commit + this doc.

4. **CancelJob-cause vs client-cancel distinction is
   inherently racy.** When the outcome match reads
   `cancel_token.is_cancelled()`, the token's state reflects
   whatever last happened. If a CancelJob fires
   simultaneously with a client hangup, the error_message
   could attribute the cause to either. Acceptable for an
   observability surface; both paths take the same teardown.

5. **No "cancellation took effect within X ms" guarantee.**
   The handler future is dropped on the next `.await`
   resolve, so a handler stuck in a sync section runs to
   completion. Same posture as `tx.closed()` cancel. The
   data plane's cancellation chain handles the I/O-bound
   wait points.

## Reviewer comments

### Round 1 (reviewed sha `4a2eb0a`) — reopened

Reviewer: `codex-reviewer`. Validation green. Two findings:

1. **Medium** — `cancel(id) -> bool` returned `true` for any
   row in the cancellations map. Only `delegated_pull` races
   the token in its handler select; push / pull / pull_sync
   would silently keep running while the caller saw
   "cancelled". Suggested: return a structured outcome and
   encode "this kind doesn't support cancellation" as a
   distinct variant rather than treating token-presence as
   cancellation support.
2. **Low** — `register` inserted into `table` first, then
   into a parallel `cancellations` map. Snapshot-then-cancel
   could observe the row before the token entry existed and
   get `false` back. Suggested: publish the token before the
   row, or merge into a single locked struct.

### Round 2 (sha pending) — addresses both

**Finding 1**: `ActiveJobKind::supports_cancellation()`
predicate encodes the policy (DelegatedPull-only today).
`CancelOutcome` enum (Cancelled / Unsupported / NotFound)
replaces the bool. `cancel(id)` consults the kind and
returns `Unsupported` synchronously WITHOUT firing the
token when the kind doesn't honor it, so callers can't be
lied to.

**Finding 2**: Merged `table` + `cancellations` into a
single `Mutex<HashMap<String, TableEntry>>` where
`TableEntry { job: ActiveJob, cancellation: CancellationToken }`.
One insert + one remove + one read path. "Visible in
snapshot" strictly implies "cancel can be evaluated" —
no scheduler window between maps. Lock-order doc reduced
to "table → recent."

Tests reorganized:

- 3 round-1 tests replaced with the structured-outcome
  versions.
- `cancel_returns_unsupported_for_non_delegated_kinds`
  (new) loops over push / pull / pull_sync, asserts
  `Unsupported` outcome AND the token was NOT fired (catches
  the regression the reviewer flagged).
- `supports_cancellation_matches_dispatch_policy` (new)
  pins the predicate so dispatch-site policy can't drift
  silently from the table API.

Workspace: 528 passed (was 526; +2 net).
