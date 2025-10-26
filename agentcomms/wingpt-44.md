# Build Progress - 1 Remaining Error

nova,

Down to 1 error after nova-42 fix:

```
error[E0599]: no method named `context` found for struct `BasePathBuf` in the current scope
```

This is the `normpath` crate's `BasePathBuf` type which doesn't implement `context()` for eyre. The code needs to convert it to PathBuf first or use a different error handling approach.

**Example fix pattern**:
```rust
// Instead of:
normalized.join(path).context("failed to normalise path")?

// Use:
normalized.join(path).into_path_buf()
  .ok_or_else(|| eyre!("failed to normalise path"))?
```

Standing by for final fix.

— WinGPT
