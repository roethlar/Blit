Reviewed sha: `1e6e871bec3c6c359252fd22765cce65e0885713`

# Reopened: m2f-9-f2-discovery-refan

## Findings

1. `crates/blit-tui/src/main.rs:1997` — a changed watch set that becomes empty does not drop the existing merged receiver.

   Round 2 fixes the pending-setup discovery race, but the disappearance side of the m2f-9 contract is still broken for the mDNS-only / last-daemon case. `handle_discovery_watch_change` correctly detects the transition from `{skippy}` to `{}`, then calls `refan_f2_setup`; `refan_f2_setup` returns before `*transfers_event_rx = None` when `watched.is_empty()`. The old Subscribe receiver and forwarders stay live, so the vanished daemon is still watched until the stream happens to close. The finding doc explicitly calls out that a vanished daemon's streams should drop without a manual refresh.

   Expected: make "empty watched set" a first-class re-fan outcome. On a real change to empty, drop the merged receiver and put F2 in an appropriate no-daemon/degraded state instead of returning before cleanup. Add a regression test for: no launch remote, one discovered daemon with a live receiver, next discovery result empty, receiver is dropped and the daemon is no longer watched.

2. `crates/blit-tui/src/main.rs:1159` and `crates/blit-tui/src/state.rs:166` — removed daemons leave stale transfer rows after a non-empty shrink.

   The non-empty disappearance path drops the old receiver and starts a new setup for the remaining daemons, but setup hydration only merges snapshots for daemons that are still watched. `TransfersState::merge_snapshot` intentionally replaces rows for one source daemon and leaves every other daemon untouched. So if F2 is watching `nas + skippy` and discovery drops `skippy`, the fresh setup for `nas` will never remove `skippy`'s active rows. Those rows can remain in the active table indefinitely even though the stream for `skippy` was dropped and no completion/error event can arrive.

   Expected: when the watch set shrinks, reconcile the view cache with the new watched-daemon set. At minimum, remove active rows for daemon identities no longer watched; decide explicitly whether recent rows from removed daemons should remain as history or be removed too. Add a regression test for `A+B -> A` showing `B` active rows do not survive the re-fan.

## Gates

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (591 passed)
