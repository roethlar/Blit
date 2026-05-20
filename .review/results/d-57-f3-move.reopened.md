# d-57-f3-move reopened

Reviewed commit: `a194c5a25f0dbe0cf441ac6eca69174a1e42262c`
Reviewed at: `2026-05-20T22:44:52Z`
Reviewer: `claude-reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed (509 tests).

## Finding

### 1. `v` can launch an impossible module-root move after copying the whole module

Severity: Medium

`UserAction::F3MoveBegin` only checks `current_module_read_only()` before deriving the source and calling `begin_move` (`crates/blit-tui/src/main.rs:1531`). In the top-level modules view, `pull_source_endpoint` intentionally maps a selected module row to `Module { module, rel_path: "" }` (`crates/blit-tui/src/browse.rs:681`, `crates/blit-tui/src/browse.rs:697`). The move task then converts that empty rel path with `del_wire_path` and calls `delete_remote_path` after the receive succeeds (`crates/blit-tui/src/main.rs:3004`).

That delete cannot succeed: the daemon rejects empty purge paths and module-root deletion in `sanitize_request_paths` (`crates/blit-daemon/src/service/admin.rs:27`, `crates/blit-daemon/src/service/admin.rs:32`, `crates/blit-daemon/src/service/admin.rs:38`). So pressing `v` on a module row can copy the whole module locally, then fail the final source delete with a backend validation error. This also means the advertised read-only gate is incomplete for read-only module rows in the modules list, because `current_module_read_only()` only tracks the module currently being browsed after descent, not the selected module row at the top level.

Expected behavior: either reject module-root moves up front like F3 delete rejects module-root purges, or explicitly implement and test whole-module move semantics end-to-end. In either case, the read-only decision for top-level module rows needs to use the selected row's `BrowseRowKind::Module { read_only }`, not only `current_module_read_only()`.
