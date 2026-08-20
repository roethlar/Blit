# Perf History That Plans — record everything, warm-start the dials

**Status**: Shipped
**Created**: 2026-08-20
**Supersedes**: the narrow reading of Post-REV4 residue item (2) (report-completeness only)
**Decision ref**: D-2026-08-20-1 (blocker ruling; plan activation pending),
D-2026-08-20-2 (design rulings R1–R5 resolved, owner-delegated)
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
- **No wire/proto change.** Ruled R4b (D-2026-08-20-2): the wire stays
  frozen, CONTRACT_VERSION untouched. Epoch-zero grants keep the
  conservative floor (`data_plane.rs` validates them against
  `receiver_initial_streams`, so a seeded count cannot be honored at
  session open without negotiation); a worker seed instead drives an
  accelerated post-open ramp toward the seeded count. A seed is a hint,
  not a contract.
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
  explicitly writable service data directory for its store; ruled R5b
  (D-2026-08-20-2): `blit profile` / `blit diagnostics perf` merge
  daemon-recorded history with origin labels — no new command.
  Enable/disable semantics must control whichever store the recording
  process owns.
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
      including daemon-recorded runs, merged with origin labels (R5b);
      pre-v3 records still load (migration test).
- [ ] Recording-matrix test covers local, push, pull, both delegated
      daemon participants, and the delegated coordinator; concurrent
      session completions lose no records (writer-serialization guard);
      compaction is atomic (no live-file truncation).
- [ ] Checkers: with a seeded rung, first chunk runs at the seed; a
      poisoned seed in EITHER direction (too high or too low) brackets
      back to the true optimum (both red-proven).
- [ ] Session workers: seeded start honored via the accelerated post-open
      ramp (R4b); live controller still converges identically in the ldt-2
      parity traces.
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
- **Served-end recording + close honesty (ph-1c, landed)**: a serving
  daemon cannot record from inside its raced responder future — the
  initiator legitimately hangs up the instant it holds what it needs,
  and the dispatcher race (w4-3) then drops that future mid-teardown.
  Worse, the pull initiator's terminal summary sat in the client's
  request-stream channel when the call dropped, so the served source
  scored "peer closed before TransferSummary" for every completed
  direct pull — dishonest jobs/metrics/history. Three cooperating
  mechanisms fix this without wire changes (no new frames, no
  CONTRACT_VERSION bump): (1) `on_terminal_summary` instruments fire
  at each end's contract-terminal point (destination: summary
  finalized; source: summary received) into a dispatcher-owned
  `ServedSessionRecorder`, which appends the daemon's row AFTER the
  race settles and upgrades a hangup verdict to `ok` when the session
  had completed; (2) a bounded hangup grace (`HANGUP_GRACE`, 500ms)
  lets a handler absorb frames that already arrived before it is
  dropped — mid-transfer hangups still tear down, just one grace
  later, which no one is left to observe; (3) a DESTINATION initiator
  performs a graceful close (`SUMMARY_DELIVERY_CLOSE_GRACE`, 2s cap):
  after sending its summary it drains the response stream until the
  responder closes, so the RPC ends cleanly and the summary actually
  flushes instead of dying with a cancelled call. Daemon `peer_key`
  conventions mirror ph-1b's client shapes from the other side of the
  wire: serving a push keys `peer_host:local_root`, serving a pull
  keys the peer host alone (the peer's destination root never crosses
  the wire), the delegated dst daemon keys `src_host:dest_root`; no
  resolvable host → no key (a shared `unknown` bucket must never
  teach seeds). The daemon store is injected at service construction
  (`BlitService::from_runtime`), `HistoryStore::daemon()` in
  production, temp-dir stores in the e2e matrix — tests never touch a
  real store.
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
3. **ph-3 — seed store.** LANDED 2026-08-20. Persist settled dial values
   per key at session close, gated on min-samples + settled confidence;
   retire the predictor per R1 (its file, state versioning, and
   `blit profile`'s predicted-duration lines).
4. **ph-4 — warm-start checkers.** LANDED 2026-08-20. Design refined
   from the original "bidirectional bracketing" sketch (delegated
   envelope, D-2026-08-20-2) to something simpler and provably
   unpinnable: the seed never relocates the cold start — the dial takes
   its normal baseline, then jumps to the seeded rung for exactly ONE
   measured chunk and keeps it only if it beat the baseline; rejection
   returns the walk to where it was, unsettled. Read side is
   route-latest across classes (the run's own class is unknowable at
   open; measurement makes cross-class seeds survivable). Poisoned-seed
   recovery red-proven in both directions: bad-seed pinning (fallback
   stubbed out → test fails → restored) and absent/corrupt store
   (cold start, store-level error never invents a seed). `--checkers`
   pins outrank seeds and never teach them.
5. **ph-5 — warm-start session workers.** LANDED 2026-08-20. Wire frozen
   (R4b): the sender's session dial reads the route's settled workers
   seed at open and, when one exists, ramps toward it on an accelerated
   schedule instead of the cold-start ladder — acceleration only, never a
   pin: the live controller keeps full authority to walk past or below
   the seed on evidence, and the seed store's settle gate unchanged.
   Seeded coverage is the PUSH direction (CLI is the byte sender and
   both reads the seed and writes the settled count back). The PULL
   direction stays cold-start: the daemon is the byte sender there, its
   `ResponderInstruments` are built before `SessionOpen` reveals the
   route, so it has no seed to arm and no mirror cell to record —
   documented gap, candidate follow-up after ph-6, not a release
   blocker (pull inherits the live tuner, which is the larger share of
   the win). ldt-2 parity traces unchanged.
6. **ph-6 — rig proof.** Cold-vs-warm repeat-run A/B on magneto↔skippy;
   evidence dir under `docs/bench/`.
   **Executed 2026-08-20** — both directions, both workload classes, clean
   reruns after discarding a build-contaminated first pass. Evidence:
   `docs/bench/PERF_HISTORY_PLANNING/`. Result: warm == cold within noise
   (no regression), seeds persist/reuse/settle correctly (forward settles
   at 4, reverse down to 1), merged reporting verified live on the daemon
   box. Win ≈ 0 on this rig because the dial settles within the first
   epochs even cold — the demonstrated claims are correctness and no
   regression, not a speedup. **Owner ruled PASS 2026-08-20**
   (D-2026-08-20-4): the R3 acceptance bar is met, this plan is Shipped,
   and the D-2026-08-20-1 release-tag block is cleared.

## Rulings (all resolved — D-2026-08-20-2, owner-delegated design under
"FAST, SIMPLE, RELIABLE")

- R1 — **retire the gradient-descent predictor**; the settled-dial seed
  store is the planning consumer. It predicted for no consumer; settled
  values are simpler and true.
- R2 — **fleet/wire exposure of history stays deferred** to a later plan.
- R3 — **ph-6's rig demonstration is the closing acceptance bar**.
- R4 — **(b) wire frozen; accelerated post-open ramp** toward the seeded
  worker count. No negotiation field, no CONTRACT_VERSION bump; the ramp
  reaches the seed within the first epochs, which is most of the win.
- R5 — **(b) merged reporting**: `blit profile` / `blit diagnostics perf`
  read the daemon's store too and label record origin. Chosen over a
  separate `blit-daemon profile` command (owner floated it, no
  preference): one existing surface telling the whole truth is SIMPLE;
  a daemon-served transfer invisible to the operator fails the
  D-2026-08-20-1 "reports tell the truth" bar.
