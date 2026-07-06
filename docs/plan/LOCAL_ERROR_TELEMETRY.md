# Local transfer error telemetry (design)

**Status**: Draft
**Created**: 2026-07-06
**Supersedes**: nothing
**Decision ref**: pending (owner review)

**Held, not queued**: `docs/STATE.md`'s Queue is pinned to ONE_TRANSFER_PATH
exclusively (**D-2026-07-05-4**, "the only work item until it ships"). The
owner asked for this feature but explicitly chose to hold it as Draft-only —
**not** entered in the Queue, **not** built — until ONE_TRANSFER_PATH ships
or the owner gives an explicit go. This doc exists so the design is ready
the moment that gate lifts.

## Why this doc

The owner hit the same hard-failure crash (`audit-17` — a destination
filesystem rejecting a `:` in a filename, `os error 22`) three times across
two different USB drives while backing up `/home/michael/`, each time having
to copy-paste the terminal error into chat. The ask: persist transfer
failures locally so they can be reviewed as a batch ("sweep these all up")
instead of by hand, per-crash.

Today's "telemetry" (`perf_history.rs` → `perf_local.jsonl`, read via
`blit diagnostics perf`) only records **successful** transfers. Its schema
has an `error_count` field, but every writer hardcodes it to `0`
(`engine/history.rs`, `auto_tune/mod.rs`, `perf_predictor.rs`,
`engine/tuning.rs`) — dead. Worse, `record_performance_history` is only
reached from the success path inside `run_local_mirror` (`engine/mod.rs:220,
277, 314, 350, 792`, `engine/single_file.rs:42`); a top-level `Err` (exactly
the `os error 22` case) writes nothing. Hard failures leave zero trace
on disk today.

## Goal

A `blit copy`/`mirror` run that returns a top-level `Err` appends one record
— timestamp, command shape, and the full error chain already printed to
stderr — to a new local, capped JSONL file. A new `blit diagnostics errors`
verb lists those records (most-recent-first, `--limit`, `--json`, `--clear`),
so the owner can review accumulated failures without re-running commands or
pasting terminal output.

## Non-goals

- **Does not fix `audit-17`/`audit-18` themselves.** Those stay separate
  TODO.md findings with their own owner design call (skip-and-report vs.
  sanitize vs. clean fail-fast). This plan makes failures *durable and
  reviewable*; it does not change transfer behavior on failure.
- **Does not build a fault-kind taxonomy** (permission-denied / ENOSPC /
  invalid-name / etc. as a structured enum). That's adjacent to the
  deferred `F15` structured-logging epic (`TODO.md`). This plan persists the
  raw `eyre` error-chain text, not a classified error type.
- **Does not unify with the daemon's `recents.jsonl`** (`blit-daemon/src/
  recents_store.rs`, read via `blit jobs list <remote>`). That mechanism
  already covers daemon-mediated remote push/pull across a different
  process boundary; this plan covers local `copy`/`mirror` only (see Q3).
- **No network transmission of any kind.** Fully local, on-device, same
  trust model as `perf_local.jsonl` — this is a diagnostic log the owner
  reads with a CLI verb, never phoned home.
- **No automatic remediation** (retry-with-sanitized-name, skip-and-continue,
  etc.) — that's `audit-17`'s decision, not this plan's.

## Constraints

- Local-only, on-device storage (matches `perf_local.jsonl`'s trust model —
  a backup tool must not silently exfiltrate path/filename data).
- Append-only JSONL, capped size (reuse `perf_history.rs`'s
  `DEFAULT_MAX_BYTES` ~1 MiB rotation convention) so a machine that hits the
  same crash repeatedly doesn't grow the file unbounded.
- Must not slow down the hot (success) path — the write happens once, on
  the already-exceptional error/abort path, at process exit.
- Cross-platform: reuses `blit_core::config::config_dir()`, already
  cross-platform (`directories::ProjectDirs`). No new platform-specific
  code needed.
- The recorder itself must be failure-tolerant: a broken/unwritable config
  dir must never mask or replace the original error — recording is
  best-effort (log a `log::warn!` and proceed) around the real `Result`
  that still propagates to the process exit code and stderr exactly as
  today.

## Acceptance criteria

- [ ] A `blit copy`/`mirror` run whose top-level result is `Err` appends
      exactly one record to a new local JSONL file before the process
      exits, containing at minimum: schema_version, timestamp, mode
      (Copy/Mirror), source root, dest root, and the error chain (every
      `eyre` context frame's message, same content already printed to
      stderr by `color_eyre`).
- [ ] `blit diagnostics errors [--limit N] [--json] [--clear]` reads the
      file back, newest-first, mirroring `blit diagnostics perf`'s flag
      conventions.
- [ ] The file is capped/rotated the same way as `perf_local.jsonl` (oldest
      records evicted first) so repeated identical crashes can't grow it
      unbounded.
- [ ] `perf_local.jsonl` and its reader/predictor are completely unaffected
      — this is an additive, separate file, not a schema change to the
      existing one (see D1).
- [ ] Process exit code and stderr output for a failing command are
      **byte-identical** to today's — the recorder taps the `Result`, it
      never changes what the user sees or the exit code.
- [ ] A forced-failure integration test (e.g. an unwritable destination)
      asserts exactly one error record lands with the expected fields.
- [ ] `cargo fmt`/`clippy`/`test --workspace` all green; test count does
      not drop.

## Design

New module `blit-core/src/error_history.rs`, mirroring `perf_history.rs`'s
shape (`FailureRecord` struct, `record_failure(...)`, `read_failures(limit)`,
`clear_failures()`), writing to `errors_local.jsonl` in the same
`config::config_dir()` as `perf_local.jsonl` — a sibling file, not a shared
schema (see D1).

Draft schema (`FailureRecord`):
- `schema_version: u32`
- `timestamp` (same convention as `PerformanceRecord`)
- `mode: TransferMode` (reuse the existing `Copy`/`Mirror` enum from
  `perf_history.rs`)
- `source: String`, `dest: String` (the two root paths as given on the CLI)
- `error_chain: Vec<String>` — each frame of the returned `eyre::Report`'s
  `.chain()`, in order (outermost context first, root cause last) — the
  same information `color_eyre` prints as the numbered `0:`/`1:`/... list,
  captured programmatically instead of scraped from stderr text.
- `error_location: Option<String>` — best-effort; see Q2, this may not be
  cleanly capturable without touching how `color_eyre::install()` is set
  up, and may ship as `None` in the first slice.

Wiring: `crates/blit-cli/src/main.rs`'s `Commands::Copy`/`Commands::Mirror`
arms currently call `run_with_retries(..., || run_transfer(...)).await?` —
the `?` bubbles straight out of the `match`, so there is no single point
inside `main` today that sees the `Result` before it becomes the function's
return value. The fix is the same "single chokepoint" shape this repo
already favors elsewhere (`FilteredSource`, `safe_join`,
`contained_join`): change those two arms to bind the `Result` instead of
`?`-ing it immediately, call `error_history::record_failure(...)` when it's
`Err` (best-effort — a recorder failure is logged via `log::warn!` and
never replaces the original error), then return/`?` the *original,
untouched* `Result` so behavior for the user is identical to today.

New CLI verb: `blit diagnostics errors` alongside the existing
`run_diagnostics_perf` in `crates/blit-cli/src/diagnostics.rs`, same flag
shape (`--limit`, `--json`, `--clear`).

## Slices

1. **`error_history` module** — schema, `record_failure`/`read_failures`/
   `clear_failures`, cap/rotation (mirrors `perf_history.rs`'s existing
   logic), unit tests (round-trip, cap eviction, tolerant read of a
   corrupted/partial last line — matching `perf_history.rs`'s existing
   tolerance).
2. **Wire the `Copy`/`Mirror` CLI arms** in `main.rs` to call
   `record_failure` on `Err` before propagating, unchanged exit
   code/stderr. Integration test: force a failure (e.g. destination path
   that can't be created), assert exactly one record lands with the
   expected `source`/`dest`/`mode`/non-empty `error_chain`, and assert
   stderr/exit-code parity with the no-recorder baseline.
3. **`blit diagnostics errors` read-back verb** — list/limit/json/clear,
   unit + CLI-level tests.

Deliberately **not** a slice here (future follow-ups, owner-gated): folding
`Move`/remote-mediated commands into the same recorder (Q3); capturing
`error_location` if a clean API surface exists (Q2); any interaction with
`audit-17`'s eventual skip-and-report behavior, where a partially-successful
transfer with per-file skips might also want a record here — that's a
follow-up once `audit-17` itself is designed, not this plan's job.

## Open questions for the owner

- **Q1**: A new dedicated `errors_local.jsonl`, or extend `perf_local.jsonl`
  to carry failure rows (finally populating the dead `error_count` field)?
  Agent rec: new dedicated file. `perf_local.jsonl`'s reader
  (`perf_predictor.rs`) is built around successful-run regression inputs;
  mixing failure rows into that stream complicates the predictor's read
  path for no benefit, and keeping them separate matches the existing
  precedent of `recents.jsonl` being its own file rather than folded into
  `perf_local.jsonl`.
- **Q2**: Capture `error_location` (the `Location:` file:line `color_eyre`
  prints) or ship with `error_chain` message text only? Capturing it
  cleanly may require restructuring how `color_eyre::install()` hooks
  panic/error reporting (a real technical risk, not yet spiked). Agent
  rec: ship message-chain-only first (still fully "sweepable" — the chain
  already names the failing path and OS error), file `error_location`
  capture as a fast-follow if the `eyre`/`color_eyre` API allows it without
  restructuring the install.
- **Q3**: Local `copy`/`mirror` only for now — should remote push/pull
  (daemon-mediated) ever unify onto this same file, or stay on
  `recents.jsonl` permanently? Agent rec: leave remote alone permanently;
  different process boundary (daemon vs. CLI), already has a working
  mechanism — don't force a merge for its own sake.
- **Q4**: Reuse `perf_local.jsonl`'s ~1 MiB cap as-is, given failure records
  (full error chains, long paths) may run larger per-record than perf
  records? Agent rec: same cap, oldest-evicted — consistent with the
  existing convention; revisit only if it proves too small in practice.
- **Q5 (gate, not design)**: When does this leave Draft? Per the owner's
  choice this session, not until ONE_TRANSFER_PATH ships or the owner
  explicitly lifts D-2026-07-05-4's Queue-exclusivity for this item.

## Verification (when Active)

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --
  -D warnings`; `cargo test --workspace` (count must not drop).
- Each slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`).
- Stderr/exit-code byte-parity check for the failure path (before vs. after
  wiring `record_failure` in) — the whole point is that recording is
  invisible to the user-facing failure behavior.
