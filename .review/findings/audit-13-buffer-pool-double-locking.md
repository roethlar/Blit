# audit-13-buffer-pool-double-locking: Double-locking and redundant memory zeroing in BufferPool

**Severity**: Performance
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `f9d3f2f`

## What

Ground-up audit of [`crates/blit-core/src/buffer.rs`](file:///Users/michael/Dev/Blit/crates/blit-core/src/buffer.rs) revealed that both the `release` and `return_vec` methods acquire the cache mutex lock twice in succession.

In `release` (lines 296-314):
```rust
        // Only cache if we haven't exceeded pool size
        let should_cache = {
            let cache = self.cache.lock();
            cache.len() < self.pool_size
        };

        if should_cache && buffer.capacity() >= self.buffer_size {
            // Reset length but keep capacity
            buffer.clear();
            buffer.resize(self.buffer_size, 0);

            let mut cache = self.cache.lock();
            if cache.len() < self.pool_size {
                cache.push(buffer);
            }
        }
```

And in `return_vec` (lines 333-349):
```rust
        // Only cache if we haven't exceeded pool size and buffer has right capacity
        let should_cache = {
            let cache = self.cache.lock();
            cache.len() < self.pool_size
        };

        if should_cache && buffer.capacity() >= self.buffer_size {
            // Reset length but keep capacity
            buffer.clear();
            buffer.resize(self.buffer_size, 0);

            let mut cache = self.cache.lock();
            if cache.len() < self.pool_size {
                cache.push(buffer);
            }
        }
```

This implementation suffers from three distinct issues:
1. **Mutex Lock Overhead:** Under high concurrency (e.g. streaming multiple parallel TCP streams), acquiring the `cache` lock twice in every release loop path introduces unnecessary contention and CPU overhead.
2. **Race Condition & Wasted Work:** Between releasing the lock in the first block and re-acquiring it in the second block, another thread could fill up the cache. If that happens, the buffer will have been zero-initialized (via `clear()` and `resize()`) only to be immediately dropped when the second lock acquisition detects `cache.len() >= self.pool_size`. Zeroing out large buffers (e.g., 16 MiB) is an expensive operation that consumes substantial CPU memory bandwidth.
3. **Redundant Memory Zeroing:** Calling `buffer.clear()` followed by `buffer.resize(self.buffer_size, 0)` is a performance anti-pattern. Since the buffer's contents are immediately written over (either by read calls in `send_file_double_buffered` or gRPC streams), zero-initializing the memory is redundant.

## Approach

Acquire the `cache` lock only once. Furthermore, to avoid redundant zeroing, do not call `.clear()` before resizing. Instead, simply adjust the vector length using `.resize` or `.truncate` to restore its logical length, which will keep the existing (dirty) capacity intact without writing zeroes.

Proposed implementation:
```rust
    fn release(&self, mut buffer: Vec<u8>) {
        self.in_use.fetch_sub(1, Ordering::Relaxed);

        if buffer.capacity() >= self.buffer_size {
            let mut cache = self.cache.lock();
            if cache.len() < self.pool_size {
                if buffer.len() < self.buffer_size {
                    buffer.resize(self.buffer_size, 0);
                } else {
                    buffer.truncate(self.buffer_size);
                }
                cache.push(buffer);
            }
        }

        // Release semaphore permit if we have a memory budget
        if let Some(ref sem) = self.memory_semaphore {
            sem.add_permits(1);
        }
    }
```

## Files changed

- `crates/blit-core/src/buffer.rs`

## Tests

- Add a benchmark/test to verify cache hit behavior under parallel loads and verify that double-locking is eliminated.
- Assert that buffers retrieved from the pool can carry dirty/previously-written data safely, and that no code assumes the retrieved pool buffers are pre-zeroed.

## Resolution (commit `f9d3f2f`)

Implemented essentially as proposed, factored into a shared
`cache_returned_buffer()` helper used by both `release` and `return_vec`:

- **Single lock** — one `cache.lock()`; cache only if `len < pool_size`,
  else drop. No more pre-check/re-check double acquisition.
- **No redundant zeroing** — restore the logical length with `truncate`
  on the common path (the buffer is already at `buffer_size`, so it's a
  no-op; nothing is zeroed), and `resize(buffer_size, 0)` only in the
  rare `take()`+`return_vec` shrunk case (zeroes just the delta).
- **Length restore only when caching** — moved under the lock-room check
  so a full cache never pays for the work (the reviewer's point 2).

**Safety verified before dropping the zeroing** (the one real risk — a
stale-byte wire leak): the pool's only production consumer is
`send_file_double_buffered`, which fills buffers via `read()` and writes
`&buf[..bytes_read]`, so bytes beyond the read length are never observed.
`receive_stream_double_buffered` uses local `vec!`s, not the pool. No
consumer assumes pool buffers are pre-zeroed.

Test: `pool_tests::reused_buffer_keeps_full_length_and_may_be_dirty` —
a dirtied buffer is reused (not reallocated) and handed back at full
`buffer_size`; deliberately does not assert zeroed contents, pinning the
dirty-reuse contract. Existing pool tests pass.

## Reviewer comments

(empty — pending review)
