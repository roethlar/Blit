# Perf History That Plans — record everything, warm-start the dials

**Status**: Draft
**Created**: 2026-08-20
**Supersedes**: the narrow reading of Post-REV4 residue item (2) (report-completeness only)
**Decision ref**: D-2026-08-20-1 (blocker ruling; plan activation pending)

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
- **No wire/proto change.** Each end seeds its own dials from its own
  store; nothing new crosses the session. CONTRACT_VERSION untouched.
- No change to what the live controllers are allowed to do once running —
  ldt-2's adaptive role parity and the checkers' settle-on-regression
  behavior stay authoritative over any seed.

## Constraints

- **A seed can never be worse than a bad guess.** Warm-start only moves the
  *starting* rung/count; the live dial logic must be able to walk a bad
  seed back down (regression step-back for checkers, REMOVE for workers),
  and a machine with no history behaves exactly as today. Fail-open to
  cold-start on any store read problem.
- **Contamination filter stands (R56-F1).** Only `RunKind::Real` records
  feed seeds, and only records matching the run's route and peer key.
  Dry-run / null-sink / bench records never teach anything.
- Store stays the capped JSONL under the user's config dir; the daemon uses
  the same code against its own config dir and respects the same
  enable/disable toggle semantics.
- FAST, SIMPLE, RELIABLE: implementation may be complex under the hood, but
  the behavioral contract is "it just gets faster on repeat runs and the
  reports tell the truth" — nothing for the user to configure.

## Acceptance criteria

- [ ] A remote push, a remote pull, and a daemon-served transfer each land a
      route-labeled record in the recording machine's store (integration
      tests per route; daemon side proven, not assumed).
- [ ] `blit profile` / `blit diagnostics perf` (text + JSON) label routes
      and keep per-route aggregates separate; pre-v3 records still load
      (migration test).
- [ ] Checkers: with a seeded rung, first chunk runs at the seed; a
      poisoned (too-high) seed walks back down via the existing regression
      logic (both red-proven).
- [ ] Session workers: seeded start count honored at session open, live
      controller still converges identically in the ldt-2 parity traces.
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

- **Schema v3**: `PerformanceRecord` gains `route` (local | remote_push |
  remote_pull | daemon_served; serde-default local so v0–v2 records stay
  loadable) and `peer_key` (stable destination identity: endpoint
  host + destination root for remote routes, dest filesystem root for
  local; plain text — the file is on-device and debuggable). `RunKind`
  stays orthogonal (measurement lane, unchanged).
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
  were a model of one.
- **Risks**: over-fine keying makes seeds never hit (mitigate: coarse
  workload classes, host-level peer key); a stale seed after hardware
  change is walked back by the live dials (that property is red-proven,
  not assumed); `blit profile` output changes when the predictor goes
  (called out to the owner in R1, not slipped in).

## Slices

1. **ph-1 — schema v3 + record every route.** Add `route`/`peer_key`,
   migration defaults, and the three missing recording call sites
   (remote client push, remote client pull, daemon-served). Tests per
   route + migration guard.
2. **ph-2 — honest reports.** Route labels and per-route aggregate
   separation in `blit profile` / `blit diagnostics perf`, text + JSON.
3. **ph-3 — seed store.** Persist settled dial values per key at session
   close; retire the predictor per R1 (its file, state versioning, and
   `blit profile`'s predicted-duration lines).
4. **ph-4 — warm-start checkers.** Optional seed rung into
   `AdaptiveCheckers`; poisoned-seed walk-down red-proven.
5. **ph-5 — warm-start session workers.** Seed the adaptive worker
   controller's start count; ldt-2 parity traces unchanged.
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
