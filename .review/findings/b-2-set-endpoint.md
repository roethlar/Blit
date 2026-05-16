# b-2-set-endpoint: streaming RPCs populate ActiveJobs rows

**Severity**: Feature (no behavior change visible on the wire)
**Status**: In progress / pending review
**Branch**: `phase5/getstate`
**Commit**: filled by the sentinel commit

## What

Closes the b-1 known gap: `push` and `pull_sync` are streaming
RPCs whose `module` + `path` arrive in the first stream frame,
not synchronously with the dispatch request, so b-1 left them
unregistered. This slice adds the handler-side update path
and wires both RPCs.

## Approach

`ActiveJobGuard` grows a `set_endpoint(&self, module: String,
path: String)` method. It locks the registry's
`std::sync::Mutex`, looks up the row by `transfer_id`, and
updates the two fields. Synchronous (no `.await`); no-op when
the row has already been drained — a handler may parse the
first frame just as the client cancels, in which case Drop
already ran and we'd rather skip the update than re-insert
a stale row.

At dispatch (`service/core.rs`) both `push` and `pull_sync`
register a row with empty `module` / `path` strings, mint a
transfer_id, and move the guard into the spawned task. The
handler signature grew a new `&ActiveJobGuard` parameter; the
call site in each handler fires `set_endpoint(...)` right
after parsing the first frame (the `Header` payload for push,
the `TransferOperationSpec` for pull_sync). Wire values are
recorded verbatim — containment / safety checks downstream
are unchanged.

After this slice all four `ActiveJobKind` variants are
constructed on the wire path. The `#[allow(dead_code)]` on
the enum was dropped accordingly.

## Files changed

- `crates/blit-daemon/src/active_jobs.rs`:
  - `+set_endpoint` on `ActiveJobGuard` (~20 LOC incl. doc).
  - Module doc updated to describe the b-1 + b-2 surface in
    a single "Scope so far" section, dropped the now-stale
    "out of scope (next sub-slice b-2)" block.
  - Stale doc paragraphs on `ActiveJobKind` and `register`
    referencing b-2 as future work removed; their content is
    now accurate.
  - Dropped the `#[allow(dead_code)]` from `ActiveJobKind`
    (all variants constructed now).
  - +2 unit tests in `active_jobs::tests`.

- `crates/blit-daemon/src/service/core.rs`:
  - `push` and `pull_sync` dispatchers now call
    `peer_addr_string()` + `active_jobs.register(...)` with
    empty endpoint fields, move the guard into the spawn
    task, drop it on every exit path. Same shape as the
    pre-existing `pull` / `delegated_pull` wiring.

- `crates/blit-daemon/src/service/push/control.rs`:
  - `handle_push_stream` signature grew an
    `active_job: &ActiveJobGuard` parameter.
  - After parsing `client_push_request::Payload::Header`
    (and before the existing module-resolution call) the
    handler calls
    `active_job.set_endpoint(header.module.clone(),
    header.destination_path.clone())`.

- `crates/blit-daemon/src/service/pull_sync.rs`:
  - `handle_pull_sync_stream` signature grew an
    `active_job: &ActiveJobGuard` parameter.
  - After `NormalizedTransferOperation::from_spec(...)` the
    handler calls
    `active_job.set_endpoint(spec.module.clone(),
    spec.source_path.clone())` — right before
    `resolve_module` runs so a snapshot taken in between
    sees the populated row.

## Tests added

- `active_jobs::tests::set_endpoint_updates_row_in_place` —
  registers with empty fields, calls set_endpoint, snapshots,
  verifies the row has the same transfer_id +
  start_unix_ms (set_endpoint doesn't re-stamp) but
  populated module + path.
- `active_jobs::tests::set_endpoint_is_noop_after_guard_drops`
  — drains the row out from under the guard (simulating a
  client-cancel race where Drop ran before the handler
  reached set_endpoint), then calls set_endpoint and
  asserts the table stays empty. Catches a "re-insert
  stale row" regression that would otherwise silently
  resurrect drained transfers in `GetState.active[]`.

Workspace: 509 passed (was 507; +2).

## Known gaps

- **Recent-runs ring buffer is still deferred** (b-3). Drop
  removes the row but doesn't push a `TransferRecord` into
  any history structure yet.
- **No `GetState` RPC consumer.** Out of scope; b-3 / b-4.
- **Outcome capture for the ring.** When the ring lands the
  guard will need to know whether the transfer succeeded or
  failed at Drop time. Today that information lives in the
  spawn closure's `Result<_, Status>` and is logged to
  metrics, not threaded back to the guard. The ring slice
  will route the outcome through.

## Reviewer comments

(empty — pending grade)
