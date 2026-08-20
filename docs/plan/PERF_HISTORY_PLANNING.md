# Perf History That Plans — record everything, warm-start the dials

**Status**: Draft
**Created**: 2026-08-20
**Supersedes**: the narrow reading of Post-REV4 residue item (2) (report-completeness only)
**Decision ref**: D-2026-08-20-1 (blocker ruling; plan activation pending)
**Review**: openreview 2026-08-20 (codex/gpt-5.6-sol, owner-dispatched,
`47f27238..d9021896`): `acceptable_with_changes`. Its four material changes
are folded into this revision; the two that need an owner choice are R4 and
R5 below. Record: `REVIEW.md` (Plan reviews) +
`.review/results/2026-08-20-perf-history-plan-*`.

## Goal

Perf data does two jobs, and the next release does not ship until both work
(owner ruling, 2026-08-20: "that's a blocker for a next release. we need to
make the perf data actually useful", scope "both"):

1. **Feed planning.** Every machine keeps a local history of its real
   transfers — local runs, remote runs it drove, remote runs its daemon
   served. On a repeat run to a known destination, the adaptive dials
   (checker pool, session workers) start from the settings that settled
   last time instead of the conservative cold-start floor, then keep
   adapting live exactly as today. Second visits get fast sooner.
2. **Honest reporting.** `blit profile` and `blit diagnostics perf` show
   every real transfer the machine took part in, labeled by route
   (local / remote push / remote pull / daemon-served), and never mix
   local-disk numbers into remote aggregates or vice versa.

Today neither is true: only local transfers record, the daemon records
nothing, and the "adaptive planning predictor" has exactly one consumer —
the `blit profile` display. Nothing plans from the history (verified
2026-08-20; the tuner that once did was retired in R56-F1).

## Non-goals

- **No fleet aggregation or wire exposure of history.** Daemons do not ship
  their history to clients or future G/TUIs under this plan; each machine's
  store stays on that machine. (Open ruling R2 confirms the deferral.)
- **No new user-visible CLI options** (standing rule, owner 2026-08-01).
  Tuning the program can work out at runtime stays at runtime. The existing
  `blit diagnostics perf --enable/--disable/--clear` surface is unchanged.
- **No wire/proto change — challenged by review, ruling R4.** Each end
  seeds its own dials from its own store. The review showed this conflicts
  with ph-5 as written: `data_plane.rs` validates every epoch-zero grant
  against `receiver_initial_streams` and always grants the conservative
  floor, so a learned start count cannot be honored at session open without
  a wire change. R4 picks between a defaultable, receiver-clamped
  `preferred_initial_streams` negotiation field (CONTRACT_VERSION bump)
  and re-scoping ph-5 to post-open accelerated ramping. Until R4 is ruled,
  this non-goal stands and ph-5 is blocked.
- No change to what the live controllers are allowed to do once running —
  ldt-2's adaptive role parity and the checkers' settle-on-regression
  behavior stay authoritative over any seed.

## Constraints

- **A seed can never be worse than a bad guess.** Warm-start only moves the
  *starting* rung/count, and a machine with no history behaves exactly as
  today. Fail-open to cold-start on any store read problem. **The existing
  checker regression logic is NOT sufficient for this (review):** it
  baselines the first observation and probes upward, undoing only the last
  probe — seeded at 32 with a true optimum of 4 it settles at 32 forever.
  Seeded starts require bidirectional bracketing: probe downward from the
  seed while throughput improves, upward otherwise, before settling.
- **Contamination filter stands (R56-F1).** Only `RunKind::Real` records
  feed seeds, and only records matching the run's route and peer key.
  Dry-run / null-sink / bench records never teach anything.
- Store stays the capped JSONL under the user's config dir for CLI-driven
  runs. **The daemon cannot share that store (review, ruling R5):** the
  documented service runs as `User=blit` under `ProtectSystem=strict` /
  `ProtectHome=read-only` (`docs/DAEMON_CONFIG.md`), so a config-dir store
  is unwritable there, and even where writable it is the service account's
  store — invisible to an operator's `blit profile`. The daemon gets an
  explicitly writable service data directory for its store; the operator
  surface question is R5. Enable/disable semantics must control whichever
  store the recording process owns.
- **Writer safety (review):** the current rotate path checks length then
  truncates the live file without locking or atomic replacement; the daemon
  spawns served transfers independently. All appends in a process go
  through one writer (actor or equivalent serialization); compaction writes
  a temp file, syncs, then atomically renames, never truncating a live
  file. Concurrent-completion loss is guard-tested.
- FAST, SIMPLE, RELIABLE: implementation may be complex under the hood, but
  the behavioral contract is "it just gets faster on repeat runs and the
  reports tell the truth" — nothing for the user to configure.

## Acceptance criteria

- [ ] A remote push, a remote pull, and a daemon-served transfer each land a
      route-labeled record in the recording machine's store (integration
      tests per route; daemon side proven, not assumed).
- [ ] `blit profile` / `blit diagnostics perf` (text + JSON) label
      topology/role/initiator and keep aggregates separate per key —
      including daemon-recorded runs, surfaced per the R5 ruling with
      origin labels; pre-v3 records still load (migration test).
- [ ] Recording-matrix test covers local, push, pull, both delegated
      daemon participants, and the delegated coordinator; concurrent
      session completions lose no records (writer-serialization guard);
      compaction is atomic (no live-file truncation).
- [ ] Checkers: with a seeded rung, first chunk runs at the seed; a
      poisoned seed in EITHER direction (too high or too low) brackets
      back to the true optimum (both red-proven).
- [ ] Session workers: seeded start honored per the R4 ruling (at session
      open if R4a, via accelerated post-open ramp if R4b); live controller
      still converges identically in the ldt-2 parity traces.
- [ ] Seeds persist only from settled runs (min-samples + confidence gate
      red-proven: an interrupted probing run writes no seed).
- [ ] Rig proof (magneto↔skippy, small-file fixture): on a repeat run,
      time-to-settle is strictly lower warm than cold, and wall time is
      ≤ cold within the rig's noise floor. One session, alternating
      A/B, per the repo's bench discipline.
- [ ] Full gate green (fmt, clippy native + linux-cross, workspace tests,
      check-docs) and CI green on all three platforms before the plan
      closes; test count never drops.

## Design

Affected: `crates/blit-core/src/perf_history.rs` (schema v3),
`perf_predictor.rs` (retired or reduced, per R1), `profile.rs`,
`diagnostics/perf.rs`, `transfer_session/{local.rs,mod.rs,checkers.rs}`,
`crates/blit-daemon/src/service/core.rs` (record daemon-served sessions),
`crates/blit-cli/src/diagnostics.rs` and the transfers front-ends (route
tagging at the call sites).

- **Schema v3 (revised per review)**: a flat `route` enum cannot represent
  every supported transfer — dispatch has a separate
  `RemoteToRemoteDelegated` route, and the daemon already distinguishes
  push and pull roles that a single `daemon_served` bucket would collapse,
  contaminating seeds and reports. `PerformanceRecord` instead gains
  orthogonal fields: `topology` (local | remote | remote_to_remote),
  `local_role` (source | destination | coordinator), and `initiator`
  (cli | daemon) — all serde-defaulted so v0–v2 records stay loadable —
  plus `peer_key` (stable destination identity: endpoint host +
  destination root for remote routes, dest filesystem root for local;
  plain text — the file is on-device and debuggable). `RunKind` stays
  orthogonal (measurement lane, unchanged). ph-1 carries an explicit
  recording-matrix test: local↔local, push, pull, both delegated daemon
  participants, and the delegated coordinator.
- **Recording**: the unified `transfer_session` already sees every
  transfer. Each end that runs a session appends one record to its own
  store at session close, tagged with its route as it experienced it.
  The daemon reads the same settings toggle; disabled means disabled.
- **Seed store**: at session close, alongside the record, persist the
  *settled* dial values keyed by `(route, peer_key, workload class)` —
  the checker ladder rung actually settled on and the worker count the
  controller converged to. Seeds are read at the next session open for a
  matching key. This replaces the gradient-descent predictor as the
  planning consumer (R1): settled values are ground truth; coefficients
  were a model of one. **Persistence gate (review):** a seed is written
  only after a minimum sample count and an explicit settled-confidence
  condition — never from a run that was still probing when it ended —
  so one anomalous run cannot teach a bad seed.
- **Risks**: over-fine keying makes seeds never hit (mitigate: coarse
  workload classes, host-level peer key); a stale seed after hardware
  change is walked back by the live dials (that property is red-proven,
  not assumed); `blit profile` output changes when the predictor goes
  (called out to the owner in R1, not slipped in).

## Slices

1. **ph-1 — schema v3 + record everything.** Add
   `topology`/`local_role`/`initiator`/`peer_key`, migration defaults,
   the serialized single-writer + atomic compaction, the daemon's
   writable store location, and the missing recording call sites
   (remote client push/pull, daemon roles, delegated coordinator).
   Recording-matrix + migration + concurrency guards.
2. **ph-2 — honest reports.** Topology/role/initiator labels and per-key
   aggregate separation in `blit profile` / `blit diagnostics perf`,
   text + JSON; daemon-record visibility per R5.
3. **ph-3 — seed store.** Persist settled dial values per key at session
   close, gated on min-samples + settled confidence; retire the predictor
   per R1 (its file, state versioning, and `blit profile`'s
   predicted-duration lines).
4. **ph-4 — warm-start checkers.** Optional seed rung into
   `AdaptiveCheckers` with bidirectional bracketing from the seed;
   poisoned-seed recovery red-proven in both directions.
5. **ph-5 — warm-start session workers (blocked on R4).** R4a: negotiate
   a defaultable, receiver-clamped preferred initial stream count
   (CONTRACT_VERSION bump); R4b: keep the wire frozen and seed via an
   accelerated post-open ramp. ldt-2 parity traces unchanged either way.
6. **ph-6 — rig proof.** Cold-vs-warm repeat-run A/B on magneto↔skippy;
   evidence dir under `docs/bench/`.

## Open questions

- R1: retire the gradient-descent predictor in favor of the settled-dial
  seed store (recommend **yes** — it predicts for no consumer; settled
  values are simpler and true)? — owner
- R2: confirm fleet/wire exposure of history is deferred to a later plan
  (recommend **yes**)? — owner
- R3: is ph-6's rig demonstration the closing acceptance bar for the plan
  (recommend **yes**)? — owner
- R4 (from review): worker warm-start mechanism — (a) add a defaultable,
  receiver-clamped `preferred_initial_streams` negotiation field with a
  CONTRACT_VERSION bump, or (b) keep the wire frozen and re-scope ph-5 to
  accelerated post-open ramping (recommend **b** — preserves the no-wire
  non-goal; the ramp reaches the seed within the first epochs, which is
  most of the win)? — owner
- R5 (from review): how does an operator see daemon-recorded history —
  (a) daemon store stays private to the service (reports cover CLI-side
  records only, honestly labeled as such), or (b) `blit profile` /
  `blit diagnostics perf` additionally read/query the daemon's store and
  label record origin (recommend **b** — "reports tell the truth" is the
  blocker ruling; a daemon-served transfer invisible to the operator
  fails it)? — owner
