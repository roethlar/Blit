# b-5-jobs-list: `blit jobs list <remote>` consumes GetState

**Severity**: Feature (new CLI verb, no changes to existing
verbs or wire protocol)
**Status**: In progress / pending review
**Branch**: `phase5/getstate`
**Commit**: filled by the sentinel commit

## What

Final sub-slice of milestone B. The daemon side has had the
always-on `ActiveJobs` table (b-1/b-2), the recent-runs ring
(b-3), and the `GetState` RPC (b-4) since the earlier slices;
this slice gives operators a CLI verb to query them. After
this slice milestone B is complete and the daemon's
observability surface is fully consumable.

## Approach

Same pattern as `blit list-modules <remote>`: an
`admin::jobs` library helper in `blit-app` wraps the
`BlitClient::get_state` RPC; a thin CLI runner in
`crates/blit-cli/src/jobs.rs` parses args, calls the library,
and formats human or JSON output.

The library exposes:

- `query(remote, recent_limit) -> Result<DaemonState>` — thin
  RPC wrapper. Same error-handling shape as
  `list_modules::query`.
- `kind_label(kind: i32) -> &'static str` — stable string
  label for the `TransferKind` proto enum. Unknown /
  Unspecified maps to `"unknown"` so a forward-version
  daemon emitting a kind we don't recognise yet renders
  safely.

CLI surface:

```
blit jobs list <remote> [--recent-limit N] [--json]
```

`--recent-limit 0` (the default) asks the daemon for its
default ring depth. Non-zero truncates server-side via the
existing GetState contract (verified in b-4 r2). `--json`
emits pretty-JSON; default is human-readable.

Human format example:

```
Daemon: blit 0.1.0 on host:9031 — uptime 1h 23m
Delegation: enabled
Modules: mod-a, mod-b

Active (1):
  t1233-9  pull  mod-a/sub/dir  peer=10.0.0.5:443  age=2.4s

Recent (3):
  t1233-8  push  mod-b/upload.tar  peer=10.0.0.5:443  duration=3.5s  FAILED: module not found
  t1233-7  pull  mod-a/x.txt       peer=10.0.0.5:443  duration=540ms  ok
  t1233-6  pull  mod-a/file.txt    peer=10.0.0.5:443  duration=1.2s   ok

Counters: push=0 pull=2 purge=0 active=1 errors=1
```

Recent rows are rendered newest-first for human eyes even
though the wire is oldest-first — the JSON output preserves
the wire order so consumers (e.g. the future TUI) can
reverse on their own terms.

## Files changed

- `crates/blit-app/src/admin/jobs.rs` (new, ~80 LOC including
  doc + tests): `query` RPC wrapper, `kind_label` helper, 2
  unit tests.
- `crates/blit-app/src/admin/mod.rs`: declares the new
  module.
- `crates/blit-cli/src/cli.rs`:
  - `+Commands::Jobs { command: JobsCommand }` enum variant.
  - `+JobsCommand::List(JobsListArgs)`.
  - `+JobsListArgs { remote, recent_limit, json }`.
- `crates/blit-cli/src/jobs.rs` (new, ~220 LOC including
  doc + tests):
  - `run_jobs(JobsCommand)` dispatcher.
  - `run_jobs_list(JobsListArgs)` — parses endpoint, calls
    library, formats.
  - `print_human(remote, state)` — multi-section formatter.
  - `print_json(state)` — pretty-JSON with `kind_label`
    strings replacing the wire enum int.
  - `format_uptime` / `format_ms` / `module_path` helpers.
  - 3 unit tests covering the helpers.
- `crates/blit-cli/src/main.rs`:
  - `+mod jobs;`
  - `+use crate::jobs::run_jobs;`
  - `+Commands::Jobs { command } => run_jobs(command).await?;`

## Tests added

In `blit_app::admin::jobs::tests`:

- `kind_label_maps_known_variants` — all 4 transfer kinds.
- `kind_label_unknown_or_unspecified_is_safe` — Unspecified
  + a future-only value (999) both map to `"unknown"`, no
  panic.

In `blit_cli::jobs::tests`:

- `format_uptime_renders_hours_minutes_seconds` — 0s, 45s,
  2m 5s, 1h 1m ladder.
- `format_ms_switches_to_seconds_above_1k` — 0ms/999ms/1.0s/3.5s
  transitions.
- `module_path_handles_each_empty_combination` — `(empty,
  empty)`, `(empty, p)`, `(mod, empty)`, `(mod, sub/dir)`.

Workspace: 523 passed (was 518; +5).

## Known gaps

1. **No integration test against a real tonic server.** The
   unit tests cover formatting + the kind-label mapping;
   `query()` itself is exercised through the CLI's
   `blit jobs list` invocation in a future end-to-end smoke
   test. Same posture as `blit list-modules`.

2. **Byte / file fields render as `0`.** Until milestone C's
   write-loop instrumentation lands the fields are always
   zero on the wire; the human formatter doesn't print them.
   The JSON formatter does (`bytes`, `files`) so downstream
   tooling can rely on the keys being present from day one.

3. **Newest-first ordering in human output is a presentation
   choice.** The JSON output preserves the daemon's
   oldest-first wire order so TUI / scripted consumers can
   reverse independently. The human format diverges because
   terminals scroll bottom-up and people skim recent rows
   first.

4. **No `man` page entry yet.** `docs/cli/blit.1.md` doesn't
   describe `blit jobs list`. Out of scope for this slice
   to keep it focused on the verb plumbing; a docs-only
   follow-up can land it before the 0.1.0 release cut.

## Reviewer comments

(empty — pending grade)
