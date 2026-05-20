# d-54-f1-module-capacity: per-module capacity in F1 detail

**Severity**: Feature (designed — TUI_DESIGN §5.1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `9777dd3`

## What

TUI_DESIGN §5.1's F1 detail mockup shows module capacity —
"Modules: home (12.3 TiB / 16.0 TiB), backups, media". The F1
detail (a1-3b) listed module names only; `GetState`'s `ModuleInfo`
carries no capacity. d-54 adds it via a `df` fan-out folded into
the existing detail fetch.

## Approach

### Fold into the detail fetch (no new lifecycle)

`spawn_detail_fetch` already runs `GetState` per selected daemon
(generation-guarded by `request_id`). d-54 extends it: after a
successful `GetState`, it fans out a `df`/`FilesystemStats` per
advertised module and bundles `(name, used_bytes, total_bytes)`
into the reply. `DetailUpdate.result` becomes
`Result<(DaemonState, Vec<(String,u64,u64)>), String>`;
`DaemonDetail::Loaded` gains a `capacities` field. No new
channel, no new generation logic — the existing detail-fetch
machinery carries it.

**Best-effort.** A module whose `df` fails is simply omitted from
`capacities`; `GetState` (the primary payload) still succeeds and
the row still loads. The renderer falls back to the bare name for
any module without a capacity entry.

### Render

The F1 modules line annotates each module: `home (12.00 GiB /
16.00 GiB)` when its capacity is present, bare `media` otherwise.
A local `format_bytes` (IEC tiers matching F2/F4 per d-25;
duplicated per the existing per-screen convention) formats the
sizes.

## Files changed

- `crates/blit-tui/src/daemons.rs`: `DaemonDetail::Loaded` gains
  `capacities`; 2 test fixtures updated.
- `crates/blit-tui/src/main.rs`: `spawn_detail_fetch` df fan-out;
  `DetailUpdate.result` tuple; reply arm builds `Loaded` with
  capacities; 1 test fixture updated.
- `crates/blit-tui/src/screens/f1.rs`: capacity-annotated modules
  line; local `format_bytes`; module-doc sketch; 1 test.

## Tests

+1 test (487 → 488):

- `detail_lines_annotate_module_capacity_with_fallback` — `home`
  (with capacity) renders `home (12.00 GiB / 16.00 GiB)`; `media`
  (no capacity) falls back to the bare name.

The df fan-out itself needs a live daemon and is exercised
manually; the render (the part unique to d-54) is unit-tested,
and the existing detail-fetch generation tests still pass.

## Known gaps

1. **Sequential df fan-out adds latency.** Each module's `df`
   runs after `GetState`, one at a time, before the detail
   resolves. For daemons with many modules this delays the
   capacity display (the `Pending` spinner shows meanwhile).
   Module counts are small in practice; parallel df is a
   possible future optimization.
2. **Capacity is fetched once per detail load** (cached with the
   `Loaded` detail). `r` re-fetches it; it doesn't live-update.

## Out of scope

- Parallel df fan-out.
- Live capacity refresh.

## Reviewer comments

(empty — pending grade)
