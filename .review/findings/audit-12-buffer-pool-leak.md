# audit-12-buffer-pool-leak: Semaphore permit leak on OOM panic in BufferPool

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `326b3ff`

## What

Ground-up audit found that `BufferPool::acquire` and `try_acquire` can permanently
leak a semaphore permit if the subsequent buffer allocation panics (OOM).

**`crates/blit-core/src/buffer.rs:207-241`** — `acquire` obtains an owned
semaphore permit, then immediately calls `std::mem::forget(permit)` to transfer
ownership into manual management:

```rust
let permit = sem.clone().acquire_owned().await.expect("...");
std::mem::forget(permit);   // line 217 — BEFORE allocation
// ...
vec![0u8; self.buffer_size] // line 231 — can panic on OOM
```

**`crates/blit-core/src/buffer.rs:245-273`** — `try_acquire` has the same
pattern at lines 249 and 263.

If `vec![0u8; self.buffer_size]` panics (out of memory, or `buffer_size` is
pathologically large), the permit is lost forever. The semaphore bounds total
concurrent buffer usage; each leaked permit permanently reduces the pool's
effective capacity. Under memory pressure, this creates a self-reinforcing
starvation loop: tasks panic → permits leak → fewer tasks can acquire → more
tasks wait indefinitely.

## Approach

Acquire the buffer first, then call `std::mem::forget(permit)` only after a
successful allocation. Or wrap the allocation in `std::panic::catch_unwind` and
release the permit on panic (though `catch_unwind` requires `UnwindSafe`).

Simpler: defer `std::mem::forget(permit)` until after the `vec!` allocation.
If `vec!` panics, the permit is still owned by the local variable and will be
dropped (releasing the semaphore) as the stack unwinds.

## Files changed

TBD by coder. Primarily `crates/blit-core/src/buffer.rs`.

## Tests

- Unit test: simulate allocation failure (e.g. by using a mock allocator or by
  setting buffer_size to usize::MAX) and verify the semaphore permit count is
  restored.
- Existing buffer pool tests must still pass.

## Resolution (commit `326b3ff`)

Took the "defer `std::mem::forget`" option (simplest, no `catch_unwind` /
`UnwindSafe` constraints). Both `acquire` and `try_acquire` now bind the
owned permit to a local `Option<OwnedSemaphorePermit>` and only
`std::mem::forget` it *after* the `vec![0u8; buffer_size]` allocation
succeeds. If the allocation panics, the local permit is dropped during
unwinding, which returns the permit to the semaphore (the normal owned-
permit Drop behavior) — no leak.

**Test:** `pool_tests::try_acquire_releases_permit_when_allocation_panics`.
`buffer_size = usize::MAX` makes `vec!` panic with "capacity overflow"
(size > isize::MAX) — an *unwinding* panic, not an allocator abort, so it
exercises exactly the protected path; `budget = 1024` yields exactly one
permit. The test catches the first panic, then calls `try_acquire` a
second time: if the permit had leaked, the second call returns `None`
without panicking; because the permit was restored, the second call
reaches the allocation and panics again. Asserting the second call panics
is therefore a precise discriminator between the bug and the fix. (Used
`try_acquire` rather than the async `acquire` so `catch_unwind` applies
cleanly; the forget-ordering fix is identical on both paths.)

## Reviewer comments

(empty — pending review)
