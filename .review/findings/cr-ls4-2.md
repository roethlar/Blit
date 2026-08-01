# cr-ls4-2 — the test-only sink seam shipped in the production API

**Severity**: LOW
**Status**: FIXED — awaiting reviewer verification
**Source**: `clp3-ls4-range` round-2 codex dispatch over `ac74e059..439a201d`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached, disposable
worktree); codex-cli 0.146.0. **`guard_confirmed: true`** — cr-ls4-1
verified closed by this pass.
Record: `.review/results/clp3-ls4-range.codex.r2.json`.

## The finding, as returned

> The concurrency-test seam is shipped as a public `LocalMirrorOptions`
> field and re-exported type, plus a runtime branch. This unnecessarily
> changes the public struct API, can break downstream exhaustive
> construction, and lets callers bypass the normal
> `FsTransferSink`/`NullSink` configuration that enforces dry-run, checksum,
> and resume behavior. Keep the override behind `cfg(test)`.

## Why it is admitted

Correct on both counts. The bypass half is the one that matters: a public
override skips the construction that applies `dry_run`, `checksum` and
`resume` to the sink, so a downstream caller could build a session whose
write backend silently ignores those options. The seam earns its keep only
inside this crate's own tests, and shipping it any wider trades API surface
for nothing — the same instinct D-2026-08-01-1 pinned for CLI flags, applied
one layer down.

## Fix

`SinkOverride`, the `sink_override` field, its `Default` arm and the
construction branch are all `#[cfg(test)]`. Production builds compile
neither the field nor the branch — the seam is not merely unused, it does
not exist. The `mod.rs` re-export is removed; only this crate's unit tests
can name the type, which is exactly the set of callers the guard needs.

`a_normal_session_holds_multiple_payloads_in_flight` is unchanged and still
green — it lives in the crate's unit-test module, where the cfg is active,
and still drives `run_local_session` end to end.

Verified after the change: fmt, clippy `-D warnings` on both
`x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` (the lib target of
each compiles WITHOUT the seam, so the production shape is what was linted),
workspace 1754 passed / 0 failed.
