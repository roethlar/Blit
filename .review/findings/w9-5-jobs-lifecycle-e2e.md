# w9-5-jobs-lifecycle-e2e — jobs/detach lifecycle e2e tests

**Branch**: `master` (owner-authorized branchless session 2026-06-11)
**Commit**: `ad773d8`
**Source finding**: tests-jobs-lifecycle-no-e2e — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

The detached-job lifecycle (`--detach` output, `jobs list`, `jobs watch` to a
terminal state, `jobs cancel` exit-code contract per TUI_DESIGN §6.5) ran in
zero integration tests. New file `crates/blit-cli/tests/jobs_lifecycle.rs`
is the regression net W4 (w4-1/w4-3 cancellation changes) needs first.

## Approach

Five tests, all against real spawned binaries:

1. **list empty**: idle daemon → exit 0, valid `--json`, `active: []`.
2. **cancel unknown** → exit 1 (NotFound).
3. **watch unknown** → exit 2 (NotFound).
4. **detach lifecycle** (real dual-daemon delegation pair, mirrors
   remote_remote.rs): `copy A B --detach --json` → parse `transfer_id`;
   `jobs watch` → exit 0 (finished-ok on either the active-then-stream or
   already-in-recent path); payload verified on the destination module;
   `jobs list` shows the id; cancel of the finished job → exit 1.
5. **cancel active** → exit 0 (Cancelled). Determinism: a fake tonic source
   whose `pull_sync` is `std::future::pending()` — the destination daemon
   registers the ActiveJobs row synchronously at dispatch, emits `Started`
   after the source connect succeeds, then stalls in `pull_sync_with_spec`,
   so after the detach output returns the job is guaranteed active and
   cancelable. (Verified against `core.rs::delegated_pull` ordering.)

Deliberate choices: assertions target **exit codes + JSON**, not human
stderr prose (w5-1 just changed those strings; the net should survive
wording changes). No `#[cfg(unix)]` gate — nothing platform-specific
(the blanket-gating of remote tests is exactly what w9-1 calls out).

## Files changed

- `crates/blit-cli/tests/jobs_lifecycle.rs` (new, 656 lines)

## Tests added

5 (suite 1334 → 1339; nothing removed). Lifecycle file re-run 4× locally,
no flakes; full run takes ~1.3s.

## Known gaps

- Exit 2 on cancel (Unsupported — non-delegated job cancelled by another
  client) is not e2e-covered: it needs an active CLI-in-byte-path job at
  cancel time, which is inherently racy with loopback speeds. The mapping
  itself is unit-tested in `jobs.rs`.
- Watch's stream-error → GetState reconciliation fallback (jobs.rs Err(status)
  arm) is not deterministically triggerable from outside; covered only down
  the NotFound/terminal paths.
- Not verified on Windows from this machine (run scripts/windows/run-blit-tests.ps1
  per AGENTS.md §5 when on Windows).
- Post-cancel daemon-side teardown of the stalled delegated pull is NOT
  asserted — whether cancellation actually reaches the stalled inner future
  is the design-2/w4-3 question this net exists to guard.
