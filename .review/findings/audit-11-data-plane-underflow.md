# audit-11-data-plane-underflow: Buffer underflow in send_file_double_buffered

**Severity**: Bug
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `6a0feb0`

## What

Ground-up audit found an arithmetic underflow in the data-plane file sender that
can panic (debug) or infinite-loop (release) when a source reader returns more
bytes than declared in the `FileHeader`.

**`crates/blit-core/src/remote/transfer/data_plane.rs:262-306`** —
`send_file_double_buffered` initializes `remaining = header.size`, then enters
a `tokio::join!` loop that reads from the file concurrently with writing to
the stream:

```rust
let bytes_b = file.read(buf_b.as_mut_slice()).await?;
remaining -= bytes_b as u64;   // line 306
```

If the source reader returns more bytes than `header.size` (e.g. the file grew
after the manifest was computed, or a malicious `TransferSource` implementation
lies about the size), `remaining -= bytes_b` underflows:

- **Debug build**: panics via `overflow_checks`.
- **Release build**: wraps to `u64::MAX`. The loop condition `while remaining > 0`
  stays true, so the loop either spins sending garbage or sends massive amounts
  of data.

The `bytes_a` path at line 284 has the same issue, though the initial read is
usually smaller.

## Approach

Clamp `bytes_a` and `bytes_b` to `remaining` before subtracting, or saturate at
zero:

```rust
let bytes_b = bytes_b.min(remaining as usize);
remaining -= bytes_b as u64;
```

Alternatively, treat excess bytes as an error and bail early.

## Files changed

TBD by coder. Primarily `crates/blit-core/src/remote/transfer/data_plane.rs`.

## Tests

- Unit test: mock reader that returns more bytes than header.size → verify
  graceful error (no panic, no infinite loop).
- Existing data-plane tests must still pass.

## Resolution (commit `6a0feb0`)

Took the clamp option (over the bail option). At both subtraction sites —
the initial read and the loop read — the byte count is clamped to
`remaining` before subtracting:

```rust
bytes_a = (bytes_a as u64).min(remaining) as usize;
remaining -= bytes_a as u64;
```

Clamping is preferred over bailing because it is also protocol-correct:
the data plane frames each file by `header.size`, so the sender must emit
*exactly* `header.size` bytes. Sending the reader's excess would push
undeclared bytes onto the stream and mis-frame the next file. The clamp
caps both the counter subtraction (no underflow) and the bytes written
(`buf_a[..bytes_a]` with the clamped `bytes_a`), so total emitted ==
`header.size` and the loop terminates when `remaining` reaches 0.

**Test:** `underflow_tests::over_returning_reader_sends_exactly_declared_size`
— a 100-byte `FileHeader` with a 4 KiB `Cursor` reader, driven over a
loopback TCP pair (the `DataPlaneSession` holds a concrete `TcpStream`, so
a real socket is the cleanest harness). Asserts the send completes without
panic/underflow and the drain side receives exactly 100 bytes. Full
workspace gate green (debug build, so the old code would have panicked).

## Reviewer comments

(empty — pending review)
