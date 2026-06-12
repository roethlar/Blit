# w9-4-readonly-enforcement-tests — cover all 3 read-only-module write gates

**Branch**: `master` (owner-authorized branchless session 2026-06-11)
**Commit**: `4d67210`
**Source finding**: tests-readonly-module-enforcement-untested — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

No test configured a `read_only: true` module anywhere in the workspace
(the harness couldn't express one), leaving the three daemon write gates —
push control stream (`push/control.rs`), purge (`core.rs::purge_inner`),
delegated pull destination (`delegated_pull.rs`) — structurally untestable.
A dropped gate (mirror-deletion blast radius) would have passed validation.

## Approach

- `tests/common/mod.rs`: `TestContext::new_read_only()` knob; `new()`
  delegates to the same internal constructor with `read_only: false`.
- New `tests/readonly_enforcement.rs`, three tests asserting failure exit,
  the `read-only` error text (locks in failure-message quality per the
  slice spec), and no destination mutation:
  1. push (`blit copy local → remote`) rejected; module dir stays empty.
  2. purge (`blit rm remote-path -y` → Purge RPC) rejected; the seeded
     file survives.
  3. delegated pull (real dual-daemon pair, delegation enabled, dest
     module read-only) rejected; destination module stays empty.

## Files changed

- `crates/blit-cli/tests/common/mod.rs` (read_only knob; not in any
  pending sentinel's footprint)
- `crates/blit-cli/tests/readonly_enforcement.rs` (new)

## Tests added

3 (suite 1365 → 1368; nothing removed).

## Known gaps

- The delegated-case dual-daemon mini-harness is another clone of the
  remote_remote.rs pattern (w9-3 consolidates).
- The gates are exercised via the CLI end-to-end; there are still no
  daemon-internal unit tests for `purge_inner`/control-stream rejection
  (acceptable: the e2e asserts cover gate + error text + no-mutation).
- Not verified on Windows from this machine.
