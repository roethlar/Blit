# d-33-f3-pull-source: F3 pull-source spec preview

**Severity**: Feature (F3 transfer-from-cursor, slice 1 of N)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

First slice of **F3 transfer-from-cursor** — the
biggest remaining feature gap (F3 can browse a remote
but not act on it). The full feature is a remote→local
pull initiated from the F3 cursor; like F4's local
transfers, the TUI owns the lifecycle (the daemon's
`PullSync` RPC streams bytes to the TUI process — it's
NOT a daemon-tracked job, so it won't appear on F2).

This slice lands the **foundation**: a pure helper that
derives the canonical remote pull-source spec
(`<host>:/<module>/<rel-path>`) from the cursor, plus a
read-only `Pull:` line in the Stats block that surfaces
it. Complete and observable on its own — the operator
can see (and copy into a CLI command) exactly what
remote path the cursor resolves to. The destination
prompt and the actual pull execution are explicit
follow-on slices.

```
┌ Stats ───────────────────────────────────────┐
│ Selected: img001.jpg · file · 2.40 MiB        │
│ View: photos/2024 · 12 entries                │
│ Pull: nas:/photos/2024/img001.jpg             │
└───────────────────────────────────────────────┘
```

## Approach

### Pure derivation

`browse::pull_source_spec(view, selected, host) ->
Option<String>`:

- `Modules` view, cursor on a `Module` row →
  `<host>:/<module>/` (pull the whole module root).
- `Module` view, cursor on a `Directory` →
  `<host>:/<module>/<path>/<dir>/` (trailing slash).
- `Module` view, cursor on a `File` →
  `<host>:/<module>/<path>/<file>` (no trailing slash).
- `None` when host is empty, no row is selected, or the
  kind contradicts the view (a Module row inside a
  Module view, etc. — states the model never produces).

The string mirrors `RemoteEndpoint`'s `server:/module/…`
parse form. It's the human-readable preview; the
eventual execution slice will reconstruct a
`RemoteEndpoint` from the operator's `--remote` (host +
port) plus the module + rel_path this logic identifies,
rather than re-parsing the display string.

### Rendering

`f3::render_into` gains a `host: Option<&str>` param
(the host from `parsed_remote`, not the raw
`--remote` label which may carry a path). `render_stats`
appends the `Pull:` line when a spec is derivable. The
Stats block constraint grew `Length(4) → Length(5)` to
fit the third line.

## Files changed

- `crates/blit-tui/src/browse.rs`:
  - `pull_source_spec` free function.
  - 8 unit tests.
- `crates/blit-tui/src/screens/f3.rs`:
  - `render_into` + `render_stats` take `host`.
  - Stats block grows to 5 rows; renders the `Pull:`
    line.
  - Module-doc layout sketch updated.
- `crates/blit-tui/src/main.rs`:
  - F3 render call passes
    `app.parsed_remote.as_ref().map(|e| e.host.as_str())`.

## Tests

+8 tests (346 → 354), all on `pull_source_spec`:

- `pull_source_none_without_host`.
- `pull_source_none_without_selection`.
- `pull_source_module_root_from_modules_view` —
  `nas:/photos/`.
- `pull_source_directory_at_module_root` —
  `nas:/photos/2024/`.
- `pull_source_directory_nested` —
  `nas:/photos/2024/summer/beach/`.
- `pull_source_file_has_no_trailing_slash` —
  `nas:/photos/2024/img001.jpg`.
- `pull_source_file_at_module_root` —
  `host:/docs/readme.txt`.
- `pull_source_rejects_contradictory_kind` — Module row
  inside a Module view, and a non-module row in the
  Modules view, both return None.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps (closed by later slices)

1. **No destination prompt yet.** The next slice adds a
   text-input mode (reusing the d-26 filter-input
   keystroke pattern) to capture the local destination.

2. **No execution yet.** A later slice spawns the pull
   (reusing `blit_app::transfers::remote::run_pull_sync`,
   the same entry the CLI uses) on a task, with a
   Running/Done/Error lifecycle and progress in the F3
   footer.

3. **Port not shown.** The preview spec omits the port
   (e.g. `:9031`); the execution slice uses the parsed
   endpoint's port directly, so the omission is
   cosmetic.

## Out of scope (this slice)

- Destination prompt (next slice).
- Pull execution + progress (later slice).
- Push (local→remote) from F3 — pull is the natural
  F3-cursor direction.

## Reviewer comments

(empty — pending grade)
