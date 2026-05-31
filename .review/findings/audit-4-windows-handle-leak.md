# audit-4-windows-handle-leak: Windows HANDLE leak on GetFileInformationByHandle failure

**Severity**: Bug
**Status**: In progress / pending review (⚠ Windows-only — see Verification gap)
**Branch**: `phase5/a1`
**Commit**: `4e77897`

## What

Ground-up audit found a deterministic resource leak in the Windows change
journal snapshot code.

**`crates/blit-core/src/change_journal/snapshot.rs:195-216`** —
`capture_snapshot()` opens a handle via `CreateFileW`, then calls
`GetFileInformationByHandle`. If that call fails, the `?` operator returns
`Err` immediately — but `CloseHandle` is several lines later and is never
reached. The raw `HANDLE` is dropped without being closed.

A filesystem that transiently rejects `GetFileInformationByHandle` (permission
error, etc.) triggers the leak deterministically. Each leaked handle consumes
kernel resources. Many repeated failures could exhaust the handle table.

## Approach

Introduce a scope guard or `Drop` wrapper that calls `CloseHandle` on the
raw HANDLE, or restructure the function to close the handle in an `Err`
path. The `DeviceIoControl` failure path is fine (CloseHandle already
executes before the return).

## Files changed

TBD by coder. Primarily `crates/blit-core/src/change_journal/snapshot.rs`.

## Tests

- Windows-only: simulate `GetFileInformationByHandle` failure → verify
  handle is closed (Process Explorer or test double)
- Existing change journal tests must still pass

## Resolution (commit `4e77897`)

**Implemented** the RAII-guard option: `OwnedHandle(HANDLE)` whose `Drop`
calls `CloseHandle`. `capture_snapshot` wraps the `CreateFileW` result in
it and uses `handle.0` for the FFI calls; the explicit `CloseHandle`
block is removed. Every exit path — success, the
`GetFileInformationByHandle` `?` error, and the early `Ok(None)` — now
releases the handle via `Drop`. (`HANDLE` is `Copy`, so `handle.0` passes
by value to the FFI calls without moving out of the guard.) The previous
explicit close surfaced `CloseHandle` errors; the guard drops them —
close failures during cleanup aren't actionable.

### ⚠ Verification gap (reviewer / CI action required)

`#[cfg(windows)]` code, **not compile-verified on the darwin dev host**:

- darwin build / clippy / test: **unaffected** (the `windows` mod is
  cfg-excluded), all pass.
- `cargo fmt`: **does** cover this code (cfg-agnostic) — formatting OK.
- `cargo check --target x86_64-pc-windows-msvc`: **blocked** — `blake3`'s
  build script needs the MSVC assembler `ml64.exe`, absent here, so the
  dependency chain fails before reaching `blit-core`.

**Please run the Windows-target build + clippy in CI before relying on
this.** The diff is a textbook RAII guard, reviewed by inspection, but is
not machine-verified on the target.
