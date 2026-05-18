# a1-3-f1-daemons reopened

Reviewed sha: `629d410ce97f53698595c4641e8bf9b4f5ad5912`
Reviewed at: 2026-05-18T04:36:18Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. F1 omits the signed local endpoint / GetState detail contract

Severity: Medium

Location: `crates/blit-tui/src/daemons.rs:121`, `crates/blit-tui/src/screens/f1.rs:159`, `docs/plan/TUI_DESIGN.md:938`

The A.1 plan says F1 ships as an mDNS list plus a per-daemon detail pane lit by `GetState`, with `"local"` as a first-class sentinel endpoint. The explicit owner-signoff note in §10 also says `blit-tui` must work without any daemon on the LAN and that `Local` appears in F1 so F2/F3 flows can treat it symmetrically with remote daemons.

This slice does neither, and the finding doc does not list either as a remaining A.1 follow-up. `replace_from_discovery` replaces the row set from `services.iter()` only, so there is no synthetic local row or endpoint-kind discriminator in `DaemonRow`. The detail pane is also only mDNS TXT data; it never queries or renders the `GetState` fields that B already landed.

As written, `blit-tui --screen f1` on a machine with no LAN daemons has no selectable local endpoint, which breaks the local-first-class commitment and leaves future F3/F2 routing without the promised F1 source of truth. Please either close this in code now or split it into an explicit later A.1 sentinel before this one is verified.

### 2. The selected row can move out of the visible table

Severity: Low

Location: `crates/blit-tui/src/screens/f1.rs:67`, `crates/blit-tui/src/daemons.rs:162`

`select_next` can advance the cursor to any discovered row, but `render_table` always renders the full row vector from offset zero with an inline style on the selected row. There is no viewport offset, `TableState`, or row slicing tied to the selected index.

Once the daemon count exceeds the table body's visible height, arrowing down past the first page makes the highlighted row disappear while the detail block continues to show the off-screen selection. That breaks the core "table with cursor selection" behavior for larger LANs or smaller terminals.

Please keep the selected row visible by rendering a window around it or by using stateful table/scroll state. A small helper or render test with more rows than the viewport would pin the contract.

### 3. Selection reset does not match the documented clamp behavior

Severity: Low

Location: `crates/blit-tui/src/daemons.rs:126`

The reducer doc says that when the selected daemon is not present in a refreshed snapshot, the selection "clamps to the new length." The implementation only preserves by name and otherwise uses `unwrap_or(0)`, so losing a selected daemon in the middle or bottom of a list jumps the operator back to the first row even when the prior index is still valid.

With a 5s mDNS rescan cadence and no departure events, transient discovery changes can produce this jump repeatedly. Please retain the prior selected index and fall back to `min(prior_index, rows.len().saturating_sub(1))` when the selected name is gone, with a regression test where a non-final selected row disappears but later rows remain.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
