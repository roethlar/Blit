# c-1a-byte-counter-api: registry-side byte-counter infrastructure

**Severity**: Feature (foundational slice of milestone C)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

First atomic slice of milestone C (`docs/plan/TUI_DESIGN.md`
§6.2). Adds the per-row byte counter and the `ByteProgressSink`
handle handlers will use to report data-plane progress. Wires
the wire-level byte fields (`ActiveTransfer.bytes_completed`,
`TransferRecord.bytes`) to read from the atomic. No data-plane
callers yet — every transfer still reports zero bytes; the
shape is just no longer hardcoded.

## Why ship this as its own slice

The full byte-level work for C is large (~1500 daemon LOC by
the design doc's estimate): write-loop instrumentation, a
state-machine for byte aggregation, throughput EWMA, plus the
`Subscribe` RPC. Slicing it lets the reviewer scrutinize each
piece in isolation rather than reviewing a monster commit.

This slice ships the **registry-side API contract** in
isolation: the public type `ByteProgressSink`, the
`ActiveJobGuard::bytes_counter` accessor, and the snapshot/Drop
plumbing. The shape is what the data-plane crate (`blit-core`)
will couple to in c-1b; locking it down first means c-1b's
review can focus on the receive-loop callsite without
relitigating the API.

## Approach

`TableEntry` gets an `Arc<AtomicU64>` field. `ActiveJobGuard`
holds a cloned `Arc` to the same atomic so Drop can read the
final value without re-taking the table lock.

`ByteProgressSink` wraps the Arc as a public type:

```rust
#[derive(Clone)]
pub struct ByteProgressSink {
    counter: Arc<AtomicU64>,
}

impl ByteProgressSink {
    pub fn report(&self, delta: u64) {
        self.counter.fetch_add(delta, Ordering::Relaxed);
    }
}
```

Clone is cheap (Arc bump); `report` is cheap
(`fetch_add` Relaxed). Outliving the guard is a benign no-op —
the orphaned atomic is gone when its last reference drops.

`snapshot()` loads the atomic inside the table lock so the
returned `ActiveJob.bytes_completed` is internally consistent
with the rest of the snapshot.

Drop loads the atomic from the entry being removed (still
inside the table lock for that frame), passes it into
`build_record`, and the resulting `TransferRecord.bytes`
freezes the final value into the recent-runs ring.

## Why Relaxed ordering

The byte counter is a write-heavy single-producer (the data-
plane write loop) / multi-reader (snapshot, Drop) shape. The
readers only need eventual visibility; nothing synchronizes
against the value (it's not a lock or a flag). `Relaxed` avoids
the cost of an acquire/release barrier on every chunk write.

## Files changed

- `crates/blit-core/src/remote/transfer/progress.rs` (round 2):
  - `+ByteProgressSink` public type (Clone, `report`,
    `new`, `from_counter`).
  - +3 unit tests:
    - `report_accumulates_on_single_sink`
    - `clones_share_underlying_counter`
    - `from_counter_wraps_external_arc`
- `crates/blit-core/src/remote/transfer/mod.rs` (round 2):
  - Re-export `ByteProgressSink` alongside `ProgressEvent` and
    `RemoteTransferProgress`.
- `crates/blit-daemon/src/active_jobs.rs`:
  - Round 1 added a local `ByteProgressSink`; round 2 deleted
    it and imports the blit-core type.
  - `+ActiveJob.bytes_completed: u64` snapshot field.
  - `+TransferRecord.bytes: u64` record field.
  - `+TableEntry.bytes_counter: Arc<AtomicU64>`.
  - `+ActiveJobGuard.bytes_counter: Arc<AtomicU64>`.
  - `+ActiveJobGuard::bytes_counter()` accessor.
  - `register` mints the Arc + threads it into both the entry
    and the guard.
  - `snapshot` loads the atomic inside the table lock.
  - `Drop` reads the final value, passes it to `build_record`.
  - `build_record` takes a `bytes: u64` parameter.
  - +4 unit tests:
    - `bytes_counter_starts_at_zero_and_reflects_reports`
    - `bytes_counter_clones_share_state`
    - `drop_records_final_bytes_in_recent`
    - `report_after_drop_does_not_resurrect_row`
- `crates/blit-daemon/src/service/core.rs`:
  - `GetState.active[].bytes_completed` reads from
    `ActiveJob.bytes_completed` (was hardcoded 0).
  - `GetState.recent[].bytes` reads from
    `TransferRecord.bytes` (was hardcoded 0).
  - `get_state_returns_active_then_drains_to_recent` test
    extended to exercise `bytes_counter().report(4096)` →
    snapshot reflects 4096 → drop → recent record carries 4096.

## Tests added

4 in `active_jobs::tests` (listed above) + 1 amended GetState
test in `service::core::tests`. Workspace: 548 passed (was
544; +4).

## Out of scope (next slices)

- **c-1b-byte-counter-wiring**: thread `Option<&ByteProgressSink>`
  through `receive_stream_double_buffered` and
  `handle_delegated_pull` so an active delegated_pull transfer
  actually reports bytes.
- **c-2-bytes-total**: wire `bytes_total` from the manifest stage
  (the discovery step that produces the FilePlan).
- **c-3-throughput**: 1-second throughput EWMA in the ActiveJob row.
- **c-4-files-counter**: `files_completed` / `files` analogue of
  the byte counter, fed from `report_file_complete`.
- **c-5-event-ring**: per-job event ring (m-jobs-4 deferred to C).
- **c-6-subscribe**: Subscribe RPC + DaemonEvent family +
  broadcast + `transfer_id_filter` (the remaining bulk of C).
- **c-7-jobs-watch-stream**: upgrade `blit jobs watch` from
  polling to streaming.

## Known gaps

1. **No call sites yet.** This is by design — the data-plane
   wiring is c-1b. `cargo build` doesn't see `report` as dead
   code because the unit tests use it. `#[allow(dead_code)]`
   on `report` documents the intent.

2. **No integration test of mid-transfer snapshot.** A real
   delegated_pull transfer can't yet exercise the counter
   (c-1b adds the data-plane callsite). Unit tests in
   active_jobs cover the registry-side API in isolation;
   end-to-end coverage lands once c-1b wires the producer.

3. **`bytes_total` and `files_completed` stay at zero.** The
   wire shape carries them but no producer feeds them in this
   slice. They are explicit out-of-scope deferrals listed above.

## Round 2 (sha `234d2c6`)

Reviewer caught a crate-dependency direction bug: round 1
placed `ByteProgressSink` inside `blit-daemon::active_jobs`,
but the documented c-1b plan was for
`blit-core::receive_stream_double_buffered` to take the sink
as a parameter. `blit-core` is the lower crate; it cannot name
a `blit-daemon` type without a cycle.

Fix: moved `ByteProgressSink` into
`blit-core::remote::transfer::progress` (re-exported from
`blit_core::remote::transfer`). The type now sits next to
`RemoteTransferProgress` / `ProgressEvent`, which is the
natural neighbor — they're all transfer-progress reporters at
different granularities (file-level vs. byte-level).

API additions in blit-core:

- `ByteProgressSink::new()` constructs a fresh sink with a new
  Arc<AtomicU64> (general purpose).
- `ByteProgressSink::from_counter(Arc<AtomicU64>)` wraps an
  existing counter. Daemon uses this so the sink it hands the
  data plane shares the atomic that lives on the `ActiveJobs`
  row.

`blit-daemon::active_jobs` now imports the type and constructs
sinks via `ByteProgressSink::from_counter(Arc::clone(&row_counter))`.
Wire-up to the per-row atomic is unchanged.

Docs updated: module preamble in `active_jobs.rs` and the
"Files changed" section below now point at the cross-crate
location.

Tests: +3 blit-core unit tests in `progress::tests`:

- `report_accumulates_on_single_sink`
- `clones_share_underlying_counter`
- `from_counter_wraps_external_arc`

Workspace: 551 passing serially (was 548; +3 from the new
blit-core tests).

## Reviewer comments

(empty — pending grade)
