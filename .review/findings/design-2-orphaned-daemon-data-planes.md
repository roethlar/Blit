# design-2 — Daemon data-plane tasks detach (not abort) when the control stream dies

> **Closed 2026-07-04 by `w4-1`**: the remaining `push/control.rs:57` site
> (see the scope update below) is now wrapped in the hoisted
> `AbortOnDrop`. See `.review/findings/w4-1-abortondrop-family.md` for the
> fix and its regression test.

**Source**: Design-coherence review Phase A (`docs/audit/DESIGN_MAP_2026-06-11.md` §1.9),
mechanism re-verified by hand 2026-06-11 before filing.
**Severity**: High (DoS-class resource leak; transfer continues unreachable by
any cancel mechanism).

## What

> **Scope update (2026-07-03, ue-r2-1h)**: the two `service/pull.rs`
> sites below were **deleted with that file** when the deprecated Pull
> RPC was removed (slice `ue-r2-1h`, see
> `.review/findings/ue-r2-1h.md`). Only the
> `push/control.rs:57` site remains; the w4-1 row now scopes to it
> (plus the AbortOnDrop hoist and the regression test, which are
> unchanged). Note pull_sync's own data plane runs inline in the
> handler (no spawned handle), so it was never on this list.

Three daemon spawn sites hold **bare `tokio::task::JoinHandle`s** for their TCP
data-plane tasks:

- ~~`crates/blit-daemon/src/service/pull.rs:180`~~ (deleted at ue-r2-1h) —
  `transfer_task` in the legacy Pull path (`accept_pull_data_connection`).
- ~~`crates/blit-daemon/src/service/pull.rs:297`~~ (deleted at ue-r2-1h) —
  `data_plane_handle` in
  `stream_pull_streaming` (`accept_pull_data_connection_streaming`).
- `crates/blit-daemon/src/service/push/control.rs:57` —
  `data_plane_handle: Option<JoinHandle<...>>` in the push control loop.

Dropping a `JoinHandle` **detaches** the task; it does not abort it. When the
client disconnects mid-transfer, the handler errors on its next `tx.send(...)`
(e.g. `pull.rs:325`) and the `?` return drops the handle — the TCP data plane
keeps running until it hits a TCP error, or indefinitely if the client process
keeps the data socket open while abandoning the control stream. The orphan is
unreachable by `CancelJob` (Pull/Push report `supports_cancellation = false`,
`active_jobs.rs:162-164`) and there is no daemon-side send StallGuard to kill it.

The fix for exactly this bug class exists in the workspace —
`AbortOnDrop` (`crates/blit-core/src/remote/pull.rs:31`, R32-F2) — but it is
`pub(crate)` to blit-core and was applied only to the client pull path. Classic
fix-propagation failure (map, executive synthesis).

## Proposed fix (slice-sized)

Wrap the three daemon spawn sites in an abort-on-drop guard (hoist
`AbortOnDrop` to a shared location — e.g. `blit-core::remote::transfer` — and
use it from the daemon), so handler exit (error or cancel) tears down the data
plane. Add a regression test: start a pull/push, kill the control stream,
assert the data-plane task terminates (no orphan socket/task).

## Cross-references

- Map §1.9 (cancellation) — the one-of-four cancel coverage inventory.
- Sibling design issue (not this slice): tokens minted for all four transfer
  kinds but honored by DelegatedPull only; CancelJob policy/handler race
  coupling (`core.rs:1112-1115`).
