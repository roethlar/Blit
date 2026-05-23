Reviewed sha: `c18c49361b417b6e49916efe2508541f897c9cc9`

# Reopened: d-71-f1-delegated-move round 2

## Finding

Round 2 closes the remote→remote half of the first finding: the delegated branch
now applies `resolve_destination` before `plan_f1_delegated`, and the new
non-trailing-source regression covers `nas:/photos/2024 -> skippy:/backup/`.

The same data-loss class still exists in the local→remote F1 trigger branch. It
parses the remote destination, validates it, and then launches `spawn_f1_push`
with the raw destination endpoint:

- `crates/blit-tui/src/main.rs:3733` parses `dest`.
- `crates/blit-tui/src/main.rs:3754` uses `remote.display()` for the launched
  label.
- `crates/blit-tui/src/main.rs:3758` passes that same unresolved `remote` to
  `spawn_f1_push`.

Example:

```text
source: /tmp/src
dest:   nas:/home/
kind:   move
```

CLI semantics resolve this to `nas:/home/src` because the source has no trailing
slash and the destination is a container. The current TUI path pushes to
`nas:/home/` unchanged, then `spawn_f1_push` deletes `/tmp/src` after the push
succeeds. That is the same "copied to the wrong destination, then source-delete"
failure mode, just on the local-source move arm.

Please apply the same shared `resolve_destination(src, dest, &source, raw_dst)`
step before the local→remote push launch, and add coverage for the destructive
move case (`/tmp/src -> nas:/home/` resolves to `nas:/home/src`) plus the
trailing-source "copy contents" no-append case if it is not already covered.

## Gates

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All passed on the reviewed SHA.
