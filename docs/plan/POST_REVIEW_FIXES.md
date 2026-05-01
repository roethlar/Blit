# Post-review fixes — work plan

Synthesized from three external code reviews (DeepSeek pass + two
gpt/claude reviewers) cross-checked against the whitepaper. Items
already done are marked with their commit; pending items grouped by
risk + effort.

## Already done (do not redo)

| Item | Commit |
|---|---|
| Auto-promote `Modified` (size-match, mtime-mismatch) → block-hash compare without `--resume` | a7d659f |
| `BLOCK_COMPLETE` wire format extended with `mtime + perms` so zero-block transfers still update dest mtime | a7d659f |
| Regression test: `pull_sync_does_not_deadlock_with_populated_destination` | a7d659f |
| Regression test: `pull_preserves_mtime_end_to_end` | a7d659f |
| Regression test: `mtime_only_change_does_not_re_transfer_full_file` | a7d659f |
| Wire-format fuzz harness (`fuzz_wire_format_parser_does_not_panic`) | a7d659f |
| DoS bounds on parser allocations (path / tar shard / block) | a7d659f |

## Round 1 — cheap correctness wins (~half day)

Pick up tomorrow. All low-risk; collectively close one data-loss
window and remove ~100 LOC of dead code.

### 1.1 Fix error swallowing in `sink.rs`

**File:** `crates/blit-core/src/remote/transfer/sink.rs`

Several `let _ = ...` patterns hide failures that the caller assumes
succeeded. Most consequential: the `file.flush().await` immediately
before mtime application — if flush fails (disk full, EIO), the user
believes the file is durable when it isn't.

Concrete edits:

- **Line ~256** `write_file_stream`: change `let _ = file.flush().await;`
  to `file.flush().await.with_context(|| format!("flushing {}", dst.display()))?;`
  — flush failure is a data-loss signal, propagate it.
- **Line ~269 (mtime)**, **~282 (perms)**, **~426 (tar shard mtime)**,
  **~508–525 (block-complete mtime + perms)**: keep best-effort but
  log via `tracing::warn!("set mtime on {}: {}", dst.display(), e);`
  so failures are visible. Add `tracing` to blit-core dev-deps if not
  already there. Currently failures are completely invisible — that's
  why the bug fixed in 946bd77 passed every existing test.

**File:** `crates/blit-core/src/remote/transfer/data_plane.rs`

- **Lines 84–88**: TCP tuning failures (`set_keepalive`,
  `set_send_buffer_size`, `set_recv_buffer_size`) are silently
  swallowed. Wrap each in a logged warn so missed buffer sizes show
  up in tracing. Don't fail — these are best-effort knobs — but the
  silence has been masking config issues.

**File:** `crates/blit-core/src/local_worker.rs`

- **Lines 57, 62**: `mmap_copy_file` ignores `set_file_mtime` failures
  silently. Same warn-don't-fail treatment.

**Tests:** the existing `pull_preserves_mtime_end_to_end` would already
catch a regression where mtime silently fails to apply; no new tests
needed for that aspect. Add a smaller unit test that exercises
`write_file_stream` against a read-only directory and asserts the
flush-failure case bubbles up as an `Err` instead of returning Ok.

### 1.1b Surface real pipeline error from `MultiStreamSender::queue`

**File:** `crates/blit-core/src/remote/push/client/mod.rs:122`

When the streaming pipeline dies (sink worker errored, remote daemon
closed, disk full on dest), the receiver inside
`execute_sink_pipeline_streaming` is dropped. The next `tx.send().await`
in `queue()` then fails with the generic:

```
data plane pipeline closed unexpectedly
```

— and the *actual* error (the `Err` sitting inside `pipeline_handle`)
is never surfaced. Real-world hit (2026-05-01):
`./target/release/blit-cli mirror ~/dev 10.1.10.12:9031://dev -p -v`
crashed after 712 files / 6.34 MiB with this exact message and no
underlying cause — the dest module was nowhere near full, and no
clue what actually went wrong on the daemon side.

Fix: when `tx.send().await` fails in `queue()`, await `pipeline_handle`
to extract the real error and propagate that instead. Sketch:

```rust
for payload in payloads {
    if tx.send(payload).await.is_err() {
        // Receiver dropped — pipeline died. Drain the handle for the
        // real error.
        drop(self.payload_tx.take());
        let handle = self.pipeline_handle.take().expect("handle present");
        let err = match handle.await {
            Ok(Ok(_)) => eyre!("data plane pipeline closed without \
                                error but channel was closed"),
            Ok(Err(e)) => e.wrap_err("data plane pipeline failed"),
            Err(join) => eyre!("data plane pipeline panicked: {join}"),
        };
        return Err(err);
    }
}
```

Requires `pipeline_handle` to become `Option<JoinHandle<…>>` so we can
`take()` it here and in `finish()`. Same trick covers the symmetric
failure where `finish()` is called and the pipeline already errored.

Add a regression test in `pipeline.rs` that wires a sink which returns
`Err` on the first `write_payload`, sends payloads via the streaming
producer, and asserts the producer surfaces the sink's error (not the
generic "data plane pipeline closed unexpectedly" string).

Also rerun the failing mirror after the fix lands to confirm the real
cause is now visible.

### 1.2 Delete `TarShardExecutor` (or document why it stays)

**File:** `crates/blit-daemon/src/service/push/data_plane.rs`
(lines ~463–566)

After Phase 5 of the receive-pipeline unification, the daemon's TCP
push receive routes through `FsTransferSink::write_tar_shard_payload`
(rayon-parallel). `TarShardExecutor` is now used **only** by the gRPC
fallback path (`receive_fallback_data`) and carries its own buffer
pool that duplicates the shared `BufferPool` already wired into the
daemon.

Steps:

1. `grep -rn TarShardExecutor crates/` to confirm gRPC fallback is the
   only remaining caller. `acquire_buffer` already has `#[allow(dead_code)]`.
2. Inspect `receive_fallback_data` and its tar-shard branch. Refactor
   to call `FsTransferSink::write_payload(PreparedPayload::TarShard { headers, data })`
   so it uses the rayon-parallel path. (Same pattern as the TCP
   receive side.)
3. Delete `TarShardExecutor`, `MAX_PARALLEL_TAR_TASKS`, the dedicated
   `BufferPool` instance, and `acquire_buffer`. ~100 LOC.
4. Run `cargo test --workspace` — the `apply_tar_shard_handles_long_paths`
   test will need to either move into `sink.rs` (against
   `write_tar_shard_payload`) or be adapted.

If for some reason the gRPC fallback can't use the unified path
(unlikely — it's just bytes-then-extract), document why in a comment
and keep the executor. Either way, the current state of "this exists
in two places" is wrong.

### 1.3 Update `WHITEPAPER.md` for `BLOCK_COMPLETE` wire change

**File:** `docs/WHITEPAPER.md`, §3 ("Data plane wire format")

Current text:

```
BLOCK_COMPLETE := 0x03 path_len:u32 path:bytes total_size:u64
```

New:

```
BLOCK_COMPLETE := 0x03 path_len:u32 path:bytes total_size:u64 mtime:i64 perms:u32
```

Add a sentence noting mtime/perms now travel with the terminator so
the auto-promote (zero-block-transfer) case correctly updates the
destination metadata. Reference commit a7d659f.

## Round 2 — medium effort defensive work

### 2.1 `change_journal/` test coverage

**Path:** `crates/blit-core/src/change_journal/`

`grep -r "fn test_" crates/blit-core/src/change_journal/ | wc -l` → **0**.

The change journal drives the no-op fast path (the win that gives blit
its 0.01 s vs rsync 0.21 s on `large noop`). Three platform backends
(Linux ctime snapshot, macOS FSEvents, Windows USN) all untested at
the unit level.

Concrete tests to add:

- **Snapshot round-trip** for each backend: capture, persist, reload,
  verify equivalence.
- **Linux ctime semantics**: ctime changes on metadata touch (`chown`,
  `chmod`, xattr update) — verify the comparator doesn't false-negative
  on those (they should still trigger a re-enumeration).
- **Modification detection**: write a file, snapshot, modify, verify
  the journal flags the change.
- **Deletion detection**: similar.
- **Cross-snapshot stale entries**: simulate a long gap between
  snapshots (entries deleted between captures) and verify the
  comparator handles them sensibly.

Estimated 200 LOC of test, including some platform-cfg gates.

### 2.2 Drain task `tokio::Mutex` anti-pattern

**File:** `crates/blit-daemon/src/service/push/data_plane.rs:147`

```rust
let drain_handle = {
    let files = Arc::clone(&files);
    tokio::spawn(async move {
        let mut guard = files.lock().await;       // <-- held across awaits
        while guard.recv().await.is_some() {}
    })
};
```

Holds `tokio::sync::Mutex` guard across `await` for the entire
data-plane transfer. Low contention in practice (single drain task)
but a canonical anti-pattern.

Fix: restructure so the receiver is owned by exactly one task. Options:

- Pass `mpsc::Receiver<FileHeader>` directly (not behind `Arc<Mutex>`)
  to a single drain task at the start of `accept_data_connection_stream`.
- Or swap to `flume` channels, which don't require a Mutex around the
  receiver.

Either way, eliminate the Arc<AsyncMutex<Receiver>> wrapping for
the data plane case. The gRPC fallback path may still need the wrapper
since multiple pulls share the same channel — leave that path alone.

### 2.3 (Optional) Pre-allocation guard in `read_tar_shard`

`Vec::with_capacity(count)` allocates immediately. The fuzz fix bounds
`count` at 1 048 576, but a malicious peer can still cause a 1 M ×
80 B = 80 MiB allocation per shard. Consider growing the vec
lazily (`Vec::new()` + `push`) for the wire-parsed headers — Rust
doubles capacity, so it's at most 2× peak instead of pre-committed.
Trade-off: marginal CPU vs marginal memory. Probably defer.

## Round 3 — architectural / matches whitepaper §8.4

These are big and require a focused effort with measurable goals. Do
not start them as part of Round 1/2.

### 3.1 Adaptive tuning expansion

- `auto_tune` covers `chunk_bytes`, `initial_streams`, `prefetch_count`,
  `tcp_buffer_size`. **Doesn't cover**: manifest batch size, channel
  capacities, planner thresholds (size buckets, tar shard targets),
  `RECEIVE_CHUNK_SIZE`.
- Bucketing is coarse — three bandwidth brackets, two chunk sizes,
  fixed `max_streams = 8`. No RTT, no filesystem-type, no mid-transfer
  feedback.
- All static thresholds in `transfer_plan.rs` and `remote/tuning.rs`
  should funnel through `TuningParams`.

Target metric: **adaptive batched manifest** (the whitepaper's §8.1
fix) closes the small-file cold gap vs rsync. Ship that first as the
"first non-trivial use of adaptive tuning beyond chunk_bytes" and use
the implementation as the template for migrating the other hardcoded
constants.

### 3.2 `change_journal` consulted for remote transfers

Today the journal is local-only. For recurring remote mirrors (the
dominant daily-use shape for backup-style users), the daemon could
hash its enumeration result and skip the per-call walk if nothing has
changed — equivalent of the local fast-path's 0.01 s, but for remote.

Big change: requires a journal-snapshot RPC, client-side caching of
the snapshot ID per peer, and an "if your last cached ID matches
mine, skip the manifest" fast path on top of `pull_sync`.

### 3.3 Mid-transfer parameter adaptation

After the warmup probe, parameters are frozen. If congestion changes
mid-stream (other process consumes bandwidth, NIC saturation drops),
blit keeps the initial settings. Whitepaper §8 calls this "the top
architectural gap."

This is a research-y item — the right design borrows from BBR's
bandwidth/RTT estimator. Defer until Round 3.1's adaptive batching
provides a stable adaptation framework to build on top of.

## Not pursuing

- **f64 precision loss for transfers > 9 PiB** — academic; threshold is
  unrealistic. Cost of fixing is non-zero (`bytes_to_mb` is in the hot
  path of every prediction); trade isn't worth it.
- **Linear perf-predictor model** — a per-profile linear regression
  is a reasonable starting point. Going to non-linear / Bayesian /
  confidence intervals is real work for ambiguous gain.

## How to pick up tomorrow

Open this file. Start at Round 1. The three items there can be done
in any order; each is independent. Suggested:

1. §1.3 (whitepaper text update) — 5 min, no risk.
2. §1.2 (delete `TarShardExecutor`) — 1 h, includes a `cargo test --workspace`
   verification.
3. §1.1 (error swallowing) — 1–2 h, hand-edit each `let _ =` and
   audit the change. Most surgical.

After Round 1, run `testing/mirror-vs.sh skippy admin 9031 /mnt/generic-pool/video/test/blit-bench`
to confirm no perf regression. Then decide between Round 2.1 (journal
tests) and Round 2.2 (drain Mutex) based on what's most pressing.
