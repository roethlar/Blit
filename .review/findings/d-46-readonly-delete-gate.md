# d-46-readonly-delete-gate: D disabled on read-only modules

**Severity**: Feature (polish — closes d-45 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `57c5bc6`

## What

d-45 shipped F3 `D` delete relying purely on **server-side**
read-only enforcement: pressing `D` on a read-only module's entry
opened the confirm prompt, fired the Purge, and surfaced the
daemon's rejection in the footer. TUI_DESIGN §5.3 specifies the
nicer behavior — "Read-only modules disable the key." d-46 adds
that client-side gate so `D` is inert (no prompt) on a read-only
module.

## Approach

The `read_only` flag lives only on top-level `BrowseRowKind::Module`
rows; once the operator descends into a module the rows are
dirs/files that don't carry it. So `BrowseState` captures it:

- New `module_read_only: bool` field.
- `descend` into a `Module { read_only }` row records it.
- `ascend` back to the modules list clears it (no module is
  "current" there).
- `current_module_read_only()` getter.

The F3 `D` dispatcher gates on it:

```rust
if !app.browse.current_module_read_only() && is_deletable_remote_path(&target) {
    app.f3_del.begin(target, path);
}
```

Server enforcement is unchanged (the daemon still rejects Purge
on a read-only module) — this is defense-in-depth + the spec'd
UX, not a correctness substitute.

## Files changed

- `crates/blit-tui/src/browse.rs`: `module_read_only` field +
  capture on descend / clear on ascend + `current_module_read_only`
  getter; 3 tests.
- `crates/blit-tui/src/main.rs`: F3DeleteBegin dispatch gates on
  the flag.

## Tests

+3 tests (455 → 458):

- `module_read_only_tracks_descend_into_readonly_module` —
  descending into the read-only `backups` module sets the flag.
- `module_read_only_false_for_writable_module`.
- `module_read_only_clears_on_ascend_to_modules`.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **No visual "read-only" cue.** `D` is silently inert on a
   read-only module — there's no footer hint explaining why. The
   modules list could mark read-only modules (e.g. a `ro` tag),
   which would also explain the disabled key. Deferred to keep
   this slice atomic.
2. **Flag is descend-scoped, not re-validated.** It's captured
   from the modules-list row at descend time. If a module's
   read-only status changed server-side mid-session, the cached
   flag would be stale until the operator re-lists modules. The
   daemon's own enforcement is the backstop.

## Out of scope

- A read-only marker in the F3 modules list / header.
- Re-validating read-only status on each delete.

## Reviewer comments

(empty — pending grade)
