# d-34-f3-pull-endpoint: pull-source preview via RemoteEndpoint

**Severity**: Feature (F3 transfer-from-cursor, slice 2 of N)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Slice 2 of F3 transfer-from-cursor. d-33 surfaced the
pull-source preview via a hand-built string
(`{authority}:/{module}/...`). d-34 replaces that with a
real `RemoteEndpoint` derivation: `pull_source_endpoint`
clones the operator's `--remote` endpoint (host + port)
and points its path at the cursor's module + rel_path.
The preview renders `endpoint.display()`, so the shown
spec is exactly what the eventual pull execution
(slice 3) will target.

Two wins:

1. **Parse-fidelity by construction.** The d-33-round-1
   IPv6 bug (raw `host` → un-bracketed `::1:/...`) can't
   recur — the host is never re-stringified by hand;
   `RemoteEndpoint::display()` is the single authority
   for bracketing + port rendering.
2. **The execution slice's input is ready.**
   `pull_source_endpoint` returns the exact
   `RemoteEndpoint` that slice 3 will hand to
   `run_pull_sync` — display and execution share one
   derivation, so they can't drift.

## Approach

### Derivation

`browse::pull_source_endpoint(view, selected, base) ->
Option<RemoteEndpoint>`:

```rust
let (module, rel_path) = match view {
    Modules + Module row   => (row.name, ""),
    Module{name,path} + Dir/File => (name, path.join() + row.name),
    contradiction          => return None,
};
Some(RemoteEndpoint {
    host: base.host.clone(),
    port: base.port,
    path: RemotePath::Module { module, rel_path },
})
```

The `base` is the operator's parsed `--remote` (used
only for host + port; its path is overwritten).

### Preview

`main.rs` derives the spec at the draw site:

```rust
let f3_pull_spec = app.parsed_remote.as_ref().and_then(|base| {
    browse::pull_source_endpoint(app.browse.view(), app.browse.selected_row(), base)
        .map(|e| e.display())
});
```

`f3::render_into` / `render_stats` now take the
pre-rendered `pull_spec: Option<&str>` (replacing the
`host: Option<&str>` of d-33), keeping the screens layer
free of `blit_core` types.

### Behavior delta from d-33

Directory specs no longer carry a cosmetic trailing
slash — `endpoint.display()` renders
`nas:/photos/2024` (the directory itself), not
`nas:/photos/2024/`. The trailing slash was display-only
in d-33; the underlying `rel_path` is identical. This
makes the preview match the execution target exactly.

## Files changed

- `crates/blit-tui/src/browse.rs`:
  - `pull_source_spec` (string) → `pull_source_endpoint`
    (RemoteEndpoint). Imports `RemoteEndpoint` /
    `RemotePath` / `PathBuf`.
  - 10 d-33 string tests → 8 endpoint tests.
- `crates/blit-tui/src/screens/f3.rs`:
  - `render_into` / `render_stats` take pre-rendered
    `pull_spec: Option<&str>`.
- `crates/blit-tui/src/main.rs`:
  - Draw site derives `f3_pull_spec` via
    `pull_source_endpoint(...).display()`.

## Tests

8 endpoint tests (356 → 354; net −2 as 10 string tests
became 8 endpoint tests):

- `pull_source_endpoint_none_without_selection`.
- `pull_source_endpoint_module_root_from_modules_view` —
  rel_path empty, `display()` = `nas:/photos/`.
- `pull_source_endpoint_directory_at_module_root` —
  rel_path `2024`, `display()` = `nas:/photos/2024`.
- `pull_source_endpoint_directory_nested` — rel_path
  `2024/summer/beach`.
- `pull_source_endpoint_file` — rel_path
  `2024/img001.jpg`.
- `pull_source_endpoint_rejects_contradictory_kind`.
- `pull_source_endpoint_ipv6_round_trips` — `[::1]` base
  → `[::1]:/share/docs/readme.txt`, re-parses to host
  `::1`.
- `pull_source_endpoint_non_default_port` — port 9999
  carries into the endpoint + display.

Each asserts both the `RemotePath::Module` structure
(module + rel_path) and the `display()` string.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps (closed by later slices)

1. **No destination prompt yet** (next slice): a
   text-input mode capturing the local destination.
2. **No execution yet** (next slice): spawn
   `run_pull_sync(PullSyncExecution { remote: <this
   endpoint>, dest_root, options: defaults }, None)` on
   a task, with Running/Done/Error in the F3 footer.

## Out of scope (this slice)

- Destination prompt + pull execution (next slice).
- Push (local→remote) from F3.

## Reviewer comments

(empty — pending grade)
