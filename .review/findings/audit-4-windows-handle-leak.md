# audit-4-windows-handle-leak: Windows HANDLE leak on GetFileInformationByHandle failure

**Severity**: Bug
**Status**: Open
**Branch**: (none yet)

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
