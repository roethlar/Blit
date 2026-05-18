# d-9-live-tick reopened

Reviewed sha: `d406e3d8f15e89c6d8c0f1e8bb37251a087b0150`

Verdict: reopened

## Findings

### Low — `needs_live_tick` has the wrong leading rustdoc

`crates/blit-tui/src/main.rs:1391` still carries the old `can_start_transfer` comment:

```text
`true` when the operator can kick a local transfer
```

That rustdoc is now attached to `needs_live_tick`, whose contract is about arming the 500ms render wakeup. `can_start_transfer` itself is left undocumented at the declaration below. This is misleading for the next maintainer because the helper returns true while a transfer or verify run is already active, which is the opposite of "can kick a local transfer".

Move the old comment back onto `can_start_transfer` or replace it with a dedicated `needs_live_tick` comment.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
