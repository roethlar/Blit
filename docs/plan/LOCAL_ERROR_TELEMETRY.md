# Local transfer error telemetry (design)

**Status**: Draft
**Created**: 2026-07-06
**Supersedes**: nothing
**Decision ref**: pending — Q1-Q4 owner-confirmed 2026-07-06 (folded into
this doc below); Q5 (Draft→Active gate vs. D-2026-07-05-4) pending final
confirmation, see bottom.

**Held, not queued (pending confirmation)**: `docs/STATE.md`'s Queue is
pinned to ONE_TRANSFER_PATH exclusively (**D-2026-07-05-4**, "the only work
item until it ships"). The owner initially chose Draft-only/hold, then on
review of these open questions said "we need to start collecting errors
sooner than later to aid in dev" — signaling urgency, but the exact
mechanism (a recorded exception to D-2026-07-05-4 now vs. some other
sequencing) still needs one explicit confirmation before this doc enters
the Queue, flips Active, or any code lands (see **Q5** at the bottom).

## Why this doc

The owner hit the same hard-failure crash (`audit-17` — a destination
filesystem rejecting a `:` in a filename, `os error 22`) three times across
two different USB drives while backing up `/home/michael/`, each time having
to copy-paste the terminal error into chat. The ask: persist transfer
failures locally so they can be reviewed as a batch ("sweep these all up")
instead of by hand, per-crash — specifically so a future Claude Code session
can be told "address the errors in telemetry" and discover the full set
itself, across whichever route each failure came from.

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

Any `blit copy`/`mirror` invocation — local, remote-mediated push/pull, or
relayed/delegated remote→remote — that returns a top-level `Err` appends one
record (timestamp, command shape, which route it took, and the full error
chain already printed to stderr) to a new local, **uncapped** JSONL file. A
new `blit diagnostics errors` verb lists those records (most-recent-first,
`--limit`, `--json`, `--clear`), so the owner (or an agent session told to
"address the errors in telemetry") can discover the full accumulated set
without re-running commands or pasting terminal output.

## Decisions (owner-confirmed 2026-07-06)

These were open questions in the reviewed draft; the owner has answered all
four, so they're stated here as settled, not open:

- **Dedicated file** (was Q1): `errors_local.jsonl` is a new sibling file to
  `perf_local.jsonl`, not a schema change to it.
- **Message-chain only for v1** (was Q2): ship `error_chain` (the `eyre`
  frame list) without `error_location` for the first slice; add
  `error_location` capture later only if the `eyre`/`color_eyre` API allows
  it without restructuring `color_eyre::install()`.
- **Both local and remote-mediated routes** (was Q3): the owner wants one
  place that "sees them all" regardless of route — so this now covers every
  `TransferRoute` the CLI's `run_transfer` observes (`LocalToLocal`,
  `LocalToRemote`, `RemoteToLocal`, `RemoteToRemoteRelay`,
  `RemoteToRemoteDelegated` — confirmed by reading `transfers/mod.rs:101-287`
  that every arm is `.await`ed inline, so the CLI process always sees the
  `Result` regardless of route; there is no fire-and-forget/detached path
  through `run_transfer` today). This does **not** mean merging with the
  daemon's own `recents.jsonl` (`blit-daemon/src/recents_store.rs`) — that
  stays a separate mechanism for failures the *daemon* observes independent
  of what the CLI sees (e.g. a detached job with no client attached); this
  plan only ever records what the CLI process itself sees returned from
  `run_transfer`.
- **No cap, for now** (was Q4): `errors_local.jsonl` is explicitly
  **unbounded** during this development phase — the owner clears it
  manually (`--clear`) once the bugs it names are addressed. This is stated
  as a deliberately temporary posture: **a follow-up slice** (not part of
  this plan's initial slices) will make the whole feature **config-gated
  opt-in** (mirroring `perf_history`'s `--enable`/`--disable`/
  `options.perf_history` pattern) once it graduates past active development
  use. Recording is **on by default, unconditionally**, for this phase —
  no new CLI flag or config toggle in the initial slices.

## Non-goals

- **Does not fix `audit-17`/`audit-18` themselves.** Those stay separate
  TODO.md findings with their own owner design call (skip-and-report vs.
  sanitize vs. clean fail-fast). This plan makes failures *durable and
  reviewable*; it does not change transfer behavior on failure.
- **Does not build a fault-kind taxonomy** (permission-denied / ENOSPC /
  invalid-name / etc. as a structured enum). That's adjacent to the
  deferred `F15` structured-logging epic (`TODO.md`). This plan persists the
  raw `eyre` error-chain text, not a classified error type.
- **Does not merge with the daemon's `recents.jsonl`** — see Decisions
  above. Covers CLI-observed failures only, across every route `run_transfer`
  dispatches.
- **`Commands::Move` and admin verbs (`scan`/`ls`/`du`/`df`/`rm`/`find`/
  etc.) are out of scope for the initial slices** — `Commands::Move` calls
  a separate `run_move` function, not `run_transfer`; folding it in is a
  candidate follow-up, not assumed here.
- **No network transmission of any kind.** Fully local, on-device, same
  trust model as `perf_local.jsonl` — this is a diagnostic log the owner
  (or an agent working locally) reads with a CLI verb, never phoned home.
- **No automatic remediation** (retry-with-sanitized-name, skip-and-continue,
  etc.) — that's `audit-17`'s decision, not this plan's.
- **No config-gated opt-in in the initial slices** — see Decisions above;
  deferred to a follow-up once this leaves active-development use.

## Constraints

- Local-only, on-device storage (matches `perf_local.jsonl`'s trust model —
  a backup tool must not silently exfiltrate path/filename data).
- Append-only JSONL, **no size cap for now** (see Decisions — Q4). The
  owner clears it manually via `--clear`; a cap/rotation policy is deferred
  to the future config-gated-opt-in slice, not this pass.
- Must not slow down the hot (success) path — the write happens once, on
  the already-exceptional error/abort path, at process exit.
- Cross-platform: reuses `blit_core::config::config_dir()`, already
  cross-platform (`directories::ProjectDirs`). No new platform-specific
  code needed.
- The recorder itself must be failure-tolerant: a broken/unwritable config
  dir must never mask or replace the original error — recording is
  best-effort, silent by default (matching `engine/history.rs`'s existing
  `--verbose`-gated `eprintln!` convention for `perf_local.jsonl` write
  failures, not the `log` facade — see Design), around the real `Result`
  that still propagates to the process exit code and stderr exactly as
  today.

## Acceptance criteria

- [ ] Any `blit copy`/`mirror` invocation whose top-level result is `Err` —
      regardless of `TransferRoute` (local, remote-mediated, relayed,
      delegated) — appends exactly one record to a new local JSONL file
      before the process exits, containing at minimum: schema_version,
      timestamp, mode (Copy/Mirror), route, source root, dest root, and the
      error chain (every `eyre` context frame's message, same content
      already printed to stderr by `color_eyre`).
- [ ] `blit diagnostics errors [--limit N] [--json] [--clear]` reads the
      file back, newest-first, mirroring `blit diagnostics perf`'s flag
      conventions.
- [ ] The file has **no size cap** in this pass — verify it is *not*
      rotated/truncated automatically; only `--clear` empties it.
- [ ] `perf_local.jsonl` and its reader/predictor are completely unaffected
      — this is an additive, separate file, not a schema change to the
      existing one.
- [ ] Recording is unconditional (on by default) — no new flag/config gate
      in this pass.
- [ ] Process exit code and stderr output for a failing command are
      **byte-identical** to today's — the recorder taps the `Result`, it
      never changes what the user sees or the exit code.
- [ ] A forced-failure integration test per route family (at least one
      local-route failure and one remote-mediated-route failure) asserts a
      record lands with the expected fields, including the correct `route`.
- [ ] `cargo fmt`/`clippy`/`test --workspace` all green; test count does
      not drop.

## Design

New module `blit-core/src/error_history.rs`, mirroring `perf_history.rs`'s
shape (`FailureRecord` struct, `record_failure(...)`, `read_failures(limit)`,
`clear_failures()`), writing to `errors_local.jsonl` in the same
`config::config_dir()` as `perf_local.jsonl` — a sibling file, not a shared
schema.

Draft schema (`FailureRecord`):
- `schema_version: u32`
- `timestamp` (same convention as `PerformanceRecord`)
- `mode: TransferMode` (reuse the existing `Copy`/`Mirror` enum from
  `perf_history.rs`)
- `route: String` — the matched `TransferRoute` variant name
  (`local_to_local` / `local_to_remote` / `remote_to_local` /
  `remote_to_remote_relay` / `remote_to_remote_delegated`), so a later
  triage pass (human or agent) can tell at a glance whether a failure
  implicates local-fs code, the wire protocol, or the daemon.
- `source: String`, `dest: String` (the two root paths/endpoints as given
  on the CLI)
- `error_chain: Vec<String>` — each frame of the returned `eyre::Report`'s
  `.chain()`, in order (outermost context first, root cause last) — the
  same information `color_eyre` prints as the numbered `0:`/`1:`/... list,
  captured programmatically instead of scraped from stderr text.
- `error_location: Option<String>` — always `None` in the first slice (see
  Decisions — Q2); a later slice may populate it.

**Wiring** — a single chokepoint in `crates/blit-cli/src/main.rs`'s
`Commands::Copy`/`Commands::Mirror` arms, wrapping
`run_with_retries(..., || run_transfer(...)).await` to bind the `Result`
before it returns, call `error_history::record_failure(...)` when it's
`Err` (tagging `route` by re-deriving it from `args`/`select_transfer_route`
inside the recorder, or by having `run_transfer` return the route alongside
its `Result` — an implementation detail for the slice, not pinned here),
then propagate the *original, untouched* `Result` unchanged. This one point
naturally covers every `TransferRoute`, matching the owner's "both" answer
(Q3) — no need to instrument each of the five route arms individually.

*(History: an earlier draft of this doc scoped itself to local-only and, on
codex review, was found to have wired the recorder at this exact same
`main.rs` chokepoint — which the reviewer correctly flagged as inconsistent
with a **local-only** stated scope, since this chokepoint sees every route.
The owner has since widened the stated scope to cover every route (Q3), so
this chokepoint is now the right one for the (new) stated scope; the
review's underlying point — design and wiring must agree on scope — still
holds and is satisfied here.)*

**Recorder-failure handling**: matches the existing precedent in
`engine/history.rs::record_performance_history` (`history.rs:36-40`), which
already solves this exact problem for `perf_local.jsonl` — a failed history
write is silently dropped unless `--verbose`, via a direct `eprintln!`
gated on `options.verbose`, **not** the `log` facade (`blit` installs a
real stderr backend for `log::warn!` in `stderr_log.rs`, so using it here
would itself alter stderr on a recorder failure, breaking the
byte-identical-stderr acceptance criterion above).

New CLI verb: `blit diagnostics errors` alongside the existing
`run_diagnostics_perf` in `crates/blit-cli/src/diagnostics.rs`, same flag
shape (`--limit`, `--json`, `--clear`).

## Slices

1. **`error_history` module** — schema (incl. `route`), `record_failure`/
   `read_failures`/`clear_failures` (no cap/rotation — see Decisions Q4),
   unit tests (round-trip, tolerant read of a corrupted/partial last line —
   matching `perf_history.rs`'s existing tolerance).
2. **Wire the `Commands::Copy`/`Commands::Mirror` arms** in `main.rs` to
   call `record_failure` on `Err`, before propagating, unchanged exit
   code/stderr. Integration tests: force a local-route failure (e.g.
   destination path that can't be created) and, separately, a
   remote-mediated-route failure (e.g. an unreachable daemon endpoint);
   assert each lands exactly one record with the expected
   `source`/`dest`/`mode`/`route`/non-empty `error_chain`; assert
   stderr/exit-code parity with the no-recorder baseline for both.
3. **`blit diagnostics errors` read-back verb** — list/limit/json/clear,
   unit + CLI-level tests.

Deliberately **not** a slice here (future follow-ups, owner-gated):
folding `Move`/admin verbs into the same recorder; capturing
`error_location` if a clean API surface exists (Q2); config-gated opt-in
(Q4 follow-up) with a size cap once that lands; any interaction with
`audit-17`'s eventual skip-and-report behavior, where a partially-successful
transfer with per-file skips might also want a record here — that's a
follow-up once `audit-17` itself is designed, not this plan's job.

## Q5 — the one remaining open question (gate, not design)

The owner's stated urgency ("we need to start collecting errors sooner
than later to aid in dev") is clear, but the precise mechanism against
**D-2026-07-05-4** ("ONE_TRANSFER_PATH — the only work item until it
ships") is not yet confirmed. Two live options:

- **(a) Explicit, recorded exception, start now**: record a new
  `D-2026-07-06-n` decision carving out this specific exception, add this
  plan to `docs/STATE.md`'s Queue, flip `**Status**: Draft` → `Active`, and
  begin slice 1 in parallel with otp-7.
- **(b) Prioritize it as the very next thing**, without formally reopening
  D-2026-07-05-4 — i.e., pick this up the moment the current
  ONE_TRANSFER_PATH slice reaches a natural pause (not necessarily "after
  ONE_TRANSFER_PATH fully ships").

Agent rec: (a) — the owner's own words ("sooner than later... to aid in
dev") describe an active-development need, not something that benefits from
waiting for a natural pause point, and D-2026-07-05-4 is exactly the kind
of hard-worded decision this repo's governance says should be overridden
only via an explicit, recorded exception rather than inferred from urgency
language — worth one confirmation before touching the Queue.

## Verification (when Active)

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --
  -D warnings`; `cargo test --workspace` (count must not drop).
- Each slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`).
- Stderr/exit-code byte-parity check for the failure path (before vs. after
  wiring `record_failure` in) — the whole point is that recording is
  invisible to the user-facing failure behavior.
