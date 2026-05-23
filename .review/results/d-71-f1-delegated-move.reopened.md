Reviewed sha: `be0121a5c0e6e37e3a803e929b4727f51c3997d6`

# Reopened: d-71-f1-delegated-move

## Finding

`plan_f1_trigger` routes remote-source + remote-destination triggers straight into
`plan_f1_delegated` with the raw parsed destination endpoint, but it never applies
the shared `resolve_destination(raw_source, raw_dest, src_endpoint, raw_dst)`
step that the CLI uses before every `copy` / `mirror` / `move`.

That is now a data-loss bug for delegated move. Example:

```text
source: nas:/photos/2024
dest:   skippy:/backup/
kind:   move
```

CLI semantics resolve the destination to `skippy:/backup/2024` because the source
has no trailing slash and the destination is a container. The d-71 TUI path passes
`skippy:/backup/` unchanged to `build_delegated_execution`; the destination daemon
therefore writes into the destination root/container itself. After that misplaced
copy succeeds, `spawn_f1_delegated_pull` deletes `nas:/photos/2024`.

The new unit coverage uses `nas:/photos/2024/` as the source, which is a
"copy contents" form and therefore hides the missing basename-append case. Please
add a non-trailing-source regression that proves delegated move resolves the
remote destination exactly like the CLI before the source-delete step can fire.

Relevant code:

- `crates/blit-tui/src/main.rs`: remote-source branch calls
  `plan_f1_delegated(app, source_ep.clone(), dst_ep, kind, confirmed)` without
  resolution.
- `crates/blit-cli/src/transfers/mod.rs`: `run_move` parses raw endpoints and
  resolves the destination before dispatch.
- `crates/blit-app/src/transfers/resolution.rs`: existing
  `resolve_destination_remote_source_appends_basename_on_container` test pins the
  expected remote-source/container behavior.

## Gates

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All passed on the reviewed SHA.
