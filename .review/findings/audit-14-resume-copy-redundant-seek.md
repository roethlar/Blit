# audit-14-resume-copy-redundant-seek: Redundant seek system calls in sequential block-level resume

**Severity**: Performance
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `b7f8177`

## What

Audit of [`crates/blit-core/src/copy/file_copy/resume.rs`](file:///Users/michael/Dev/Blit/crates/blit-core/src/copy/file_copy/resume.rs) identified redundant `seek` system calls executed in every iteration of the block-level hash comparison loop.

In `resume_copy_file` (lines 90-128):
```rust
    while offset < src_len {
        let remaining = src_len - offset;
        let this_block = remaining.min(block_size as u64) as usize;

        // Read source block and compute hash
        src_file.seek(SeekFrom::Start(offset))?; // Redundant
        src_file.read_exact(&mut src_buf[..this_block])?;
        let src_hash = hash_block(&src_buf[..this_block]);

        // Check if we can compare with destination
        let should_write = if offset < dst_len {
            let dst_available = (dst_len - offset).min(this_block as u64) as usize;

            if dst_available == this_block {
                // Full block available, read and hash
                dst_file.seek(SeekFrom::Start(offset))?; // Redundant after first iteration
                dst_file.read_exact(&mut dst_buf[..this_block])?;
                let dst_hash = hash_block(&dst_buf[..this_block]);
                src_hash != dst_hash
            } else {
                // Partial block at end of dest, need to write
                true
            }
        } else {
            // Beyond destination, definitely need to write
            true
        };

        if should_write {
            dst_file.seek(SeekFrom::Start(offset))?; // Only needed because read_exact advanced it
            dst_file.write_all(&src_buf[..this_block])?;
            bytes_transferred += this_block as u64;
            blocks_transferred += 1;
        } else {
            blocks_skipped += 1;
        }

        offset += this_block as u64;
    }
```

Since the file operations in the loop are entirely sequential:
1. `src_file` is read sequentially. Its file pointer is always at `offset` before `read_exact`. Calling `src_file.seek(SeekFrom::Start(offset))` in every iteration is completely redundant.
2. `dst_file` is read sequentially (when checking hashes). Its file pointer is also advanced by `read_exact` to `offset + this_block`. Calling `dst_file.seek(SeekFrom::Start(offset))` before `read_exact` is redundant for every iteration except the first, or when a write occurred.
3. If a write occurs (`should_write` is true), we do need to seek the destination file back to `offset` because the previous `read_exact` advanced it to `offset + this_block`. After `write_all`, the destination file pointer is again at `offset + this_block`, which matches the next loop's starting offset.

Redundant seek system calls add significant kernel/user-space transition overhead, which degrades copying throughput on fast SSDs or network-mounted storage.

## Approach

Track the logical file pointer positions of the source and destination files (or keep track of whether they are already at the correct offset) and avoid issuing `seek` system calls unless necessary.
- Since `src_file` is always read sequentially, the `src_file.seek` call can be removed entirely.
- For `dst_file`, keep track of `dst_cursor_pos`. Only seek if the current cursor position does not match the target `offset`.

Proposed optimization:
```rust
    let mut dst_cursor_pos = 0u64;

    while offset < src_len {
        let remaining = src_len - offset;
        let this_block = remaining.min(block_size as u64) as usize;

        // Read source block sequentially (no seek needed)
        src_file.read_exact(&mut src_buf[..this_block])?;
        let src_hash = hash_block(&src_buf[..this_block]);

        let should_write = if offset < dst_len {
            let dst_available = (dst_len - offset).min(this_block as u64) as usize;

            if dst_available == this_block {
                // Only seek dst if the cursor is not already aligned
                if dst_cursor_pos != offset {
                    dst_file.seek(SeekFrom::Start(offset))?;
                }
                dst_file.read_exact(&mut dst_buf[..this_block])?;
                dst_cursor_pos = offset + this_block as u64;

                let dst_hash = hash_block(&dst_buf[..this_block]);
                src_hash != dst_hash
            } else {
                true
            }
        } else {
            true
        };

        if should_write {
            // Seek back to write offset (always needed after a read_exact advanced it, 
            // or if we jumped beyond the end)
            if dst_cursor_pos != offset {
                dst_file.seek(SeekFrom::Start(offset))?;
            }
            dst_file.write_all(&src_buf[..this_block])?;
            dst_cursor_pos = offset + this_block as u64;
            
            bytes_transferred += this_block as u64;
            blocks_transferred += 1;
        } else {
            blocks_skipped += 1;
        }

        offset += this_block as u64;
    }
```

## Files changed

- `crates/blit-core/src/copy/file_copy/resume.rs`

## Tests

- Validate using existing integration tests for block-level resume (`remote_resume.rs`).
- Assert file correctness (byte-by-byte comparison) after resume copies with mismatched and matched blocks.

## Resolution (commit `b7f8177`)

Applied as proposed: removed the per-iteration `src_file.seek` entirely
(src is read sequentially from 0, cursor always at `offset`) and added a
`dst_cursor_pos` tracker so the dst read-seek and write-seek fire only
when the cursor isn't already aligned. Pure syscall-count reduction;
seeks to the correct absolute offset are preserved wherever actually
needed.

No new test: correctness is fully covered by the existing resume suite,
which asserts byte-exact `dst == src` across new / partial / identical /
truncate-longer and the corrupted-middle-block case. In particular
`test_resume_corrupted_block` corrupts bytes 3000–4000 (spanning blocks
2–3) surrounded by matching blocks, so it exercises exactly the new
branch behavior — matched block read (cursor advances, no seek), then a
mismatched block whose write must seek back over the read, then matched
blocks again. All 5 tests stay green; adding another would duplicate
coverage.

## Reviewer comments

(empty — pending review)
