# b-1-active-jobs: introduce ActiveJobs table on BlitService

**Severity**: Feature (no behavior change visible on the wire)
**Status**: In progress / pending review
**Branch**: `phase5/getstate`
**Commit**: filled by the sentinel commit

## What

First sub-slice of milestone B (§6.3 of
`docs/plan/TUI_DESIGN.md`). Adds the always-on in-memory
registry of in-flight transfers that `GetState.active[]` will
read from in a later sub-slice. This commit introduces the
table + the RAII register/drain plumbing only — no proto
change, no RPC, no CLI surface.

## Approach

The dispatch boundary in `crates/blit-daemon/src/service/core.rs`
already owns the per-RPC active-transfers gauge guard for
`metrics`. The same boundary mints a transfer's identity by
inserting an `ActiveJob` row into the new table and returning
an `ActiveJobGuard` whose Drop removes the row. Both guards
travel into the spawned task together so any termination path
— Ok, Err, panic, client cancellation — drains the row.

Transfer ids are minted as `t<unix-ms>-<atomic-counter>`. Short
(~22 chars), sortable, unique within a daemon instance. Daemon
restart resets the counter; persistence is deferred per §10.

Streaming RPCs (`push`, `pull_sync`) are intentionally not
wired here — their module + path arrive in the first stream
frame, and the guard update path needed to fill the row
asynchronously belongs in its own slice (b-2). The
`ActiveJobKind` enum still includes the `Push` / `PullSync`
variants so the table's wire shape doesn't have to change
between slices.

## Files changed

- `crates/blit-daemon/src/active_jobs.rs` (new, ~290 LOC
  including doc comments + 3 unit tests):
  - `ActiveJobKind` enum (4 variants, lowercase `as_str()`
    helper for future wire serialization).
  - `ActiveJob` row struct.
  - `ActiveJobs` registry — `Arc<Mutex<HashMap<...>>>` + an
    atomic counter feeding the id mint.
  - `ActiveJobs::register(kind, peer, module, path)` returns
    an `ActiveJobGuard`.
  - `ActiveJobs::snapshot()` exposes a `Vec<ActiveJob>` for
    tests now; will be consumed by `GetState` later.
  - `ActiveJobGuard::Drop` uses `try_lock` then falls back to
    a spawned cleanup task if the table is briefly contended.
- `crates/blit-daemon/src/main.rs` — declares the new module
  (one-line addition).
- `crates/blit-daemon/src/service/core.rs`:
  - `BlitService` gets an `active_jobs: ActiveJobs` field,
    initialized in `from_runtime`.
  - `pull` and `delegated_pull` RPC dispatchers register a
    row alongside the metrics gauge guard and move both into
    the spawned task; `drop(job)` runs on every exit path.
  - New `peer_addr_string()` private helper at the bottom of
    the file formats `request.remote_addr()` as `<ip>:<port>`
    or `"unknown"` for in-process tests.

Doc comments on the read-side helpers
(`ActiveJobs::snapshot`, `ActiveJobGuard::transfer_id`,
`ActiveJob` fields, `ActiveJobKind::as_str`) all carry
`#[allow(dead_code)]` with a comment pointing at the future
consumer slice. The read path lands in b-4 (the `GetState`
RPC handler).

## Tests added

- `active_jobs::tests::register_inserts_then_drop_removes` —
  registers a row, asserts it appears in `snapshot()` with
  the expected kind/peer/module/path/transfer_id shape,
  drops the guard, asserts the table drains.
- `active_jobs::tests::transfer_ids_unique_under_concurrent_registers`
  — spawns 64 parallel registers, waits long enough to
  observe all rows live, collects all transfer_ids,
  dedups, asserts the count is unchanged. After all guards
  drop, the table drains.
- `active_jobs::tests::kind_strings_match_dispatch_site_names`
  — pins the lowercase `as_str()` mapping so the future wire
  serialization doesn't drift from log strings.

Workspace test count: 506 (was 503, +3).

## Known gaps

1. **Streaming RPC coverage (push, pull_sync) is deferred to
   b-2.** Today those two RPCs run without ActiveJob rows.
   `GetState.active[]` will under-report them until b-2
   lands. The slice's `ActiveJobKind` already includes the
   variants so the proto enum mapping stays stable.

2. **Recent-runs ring buffer is deferred to b-2.** The
   `ActiveJobGuard::Drop` currently just removes the row;
   the design (§6.3) calls for pushing a `TransferRecord`
   into a parallel ring at drain time. Adding that here
   would mean introducing a struct (`TransferRecord`) and
   bookkeeping (`completed_at_unix_ms`, `ok` flag, error
   message) that the slice's RPC handlers can't yet provide
   — outcome currently surfaces via the `Result<_, Status>`
   inside the spawn closure, not via a struct passed back
   up to the guard. The ring + outcome capture lands as one
   coherent unit in b-2.

3. **No `GetState` RPC consumer.** The table is populated but
   unread. Out of scope for this slice; arrives in b-3 (proto)
   + b-4 (handler).

4. **Drop fallback may spawn a one-shot task.** When the
   table is contended in `try_lock`, the fallback path spawns
   a tokio task to do the removal asynchronously. This is
   bounded by snapshot duration and shouldn't fire under any
   real load, but it does mean Drop technically races with
   `snapshot()` callers. The race is benign — Drop always
   wins eventually — but the reviewer should sanity-check
   the rationale.

## Reviewer comments

### Round 1 (reviewed sha `a842a00`) — reopened

Reviewer: `codex-reviewer`. Validation green. Two findings:

1. **Medium** — `ActiveJobGuard::Drop` used `try_lock` +
   spawned-fallback. Drop wasn't deterministic; contended
   drops left the row in the next `snapshot()` and the
   spawned cleanup could leak on shutdown.
2. **Low** — concurrent-registers test used fixed sleeps
   (`5ms` / `20ms`), flake-prone under load.

### Round 2 (sha pending) — addresses both

**Finding 1**: switched the registry from
`tokio::sync::Mutex` to `std::sync::Mutex`. The critical
sections are in-memory HashMap ops bounded by the size of
the active table — sync locking is appropriate. Drop now
blocks on `Mutex::lock()` (clippy `if_let_mutex`-friendly:
single `lock()` call with `unwrap_or_else(|e| e.into_inner())`
for poison handling) and is sync + deterministic. Callers in
`service/core.rs` lost their `.await`. Module doc carries a
"## Locking" section explaining why std mutex is the right
choice here and crediting the reviewer for catching the
round-1 leak.

**Finding 2**: replaced the sleeps with two
`tokio::sync::Barrier`s. The `registered` barrier
synchronizes the parent's snapshot with all spawns having
registered; the `release` barrier synchronizes drop. Parent
also `.await`s every join handle so every Drop is observable
before the final empty-table assertion. No timing assumption.

**New test**: `drop_blocks_on_contended_lock_then_removes`
forces the contended path with a `spawn_blocking` holder
thread holding the mutex while a `spawn_blocking` dropper
thread drops its guard. Uses an `AtomicBool` to assert the
dropper hasn't progressed past the lock acquisition while
the holder still holds it. Once the holder releases, the
dropper completes, the row is gone. This is the explicit
coverage the reviewer asked for in finding 1.

Workspace: 507 passed (was 506; +1 contended-drop test).

### Round 2 (reviewed sha `c173edf`) — reopened

Reviewer: `codex-reviewer`. Validation: fmt + clippy passed,
`cargo test --workspace` failed on the new
`drop_blocks_on_contended_lock_then_removes` test — the test
itself was racy. The holder task was spawned before the
dropper but no synchronization proved the holder had
acquired the mutex by the time the dropper started. The
dropper sometimes completed before the holder ever held the
lock, then the holder asserted `finished_drop == false`
and panicked. Production change (std mutex) looked correct.

Fix direction: explicit "holder has lock" handshake before
starting the dropper.

### Round 3 (sha pending) — addresses round-2 finding

Rewrote the test with two `std::sync::mpsc::sync_channel`s
gating the sequencing:

  1. Holder acquires the mutex, sends `LOCKED`.
  2. Parent receives `LOCKED` (holder definitely owns the
     lock), then spawns the dropper.
  3. Parent sends `RELEASE`; the holder returns (mutex
     drops), the dropper unblocks.
  4. Parent awaits both threads, asserts table empty.

Also dropped the "dropper hasn't finished while holder
held the lock" assertion entirely. With `std::sync::Mutex`,
the Drop path's `lock()` cannot complete until the holder
releases — that's a structural property of the mutex, not
a testable timing property. Spinning to confirm "still
blocked" was the original race; removing the spin removes
the race. The remaining property under test is the one
that matters for `GetState.active[]`: after the guard is
dropped, the row is gone.

Test renamed from `drop_blocks_on_contended_lock_then_removes`
→ `drop_removes_row_after_holder_releases_contended_lock`
to match what it actually asserts.

Verified locally: 50 consecutive test runs all pass. Full
workspace still 507 / 0.
