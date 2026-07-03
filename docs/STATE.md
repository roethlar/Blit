# STATE — single entry point for "what is true right now"

Last updated: 2026-07-03 (`ue-r2-1b`+`1c`+`1d`+`1e` complete — wire
contract, engine shell, streaming plan, live dials — all through the
code→review→fix loop); unpushed to `origin`/gitea: everything after
`e1a21a1` (six ue-r2-1e commits + this handoff; owner pushed through
`e1a21a1` earlier today).

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`ue-r2-1e` COMPLETE** — live cheap dials (details: DEVLOG
  2026-07-03). `TransferDial` (conservative floor start, receiver-
  profile-clamped ceilings, hysteresis stepper) replaces the DELETED
  `determine_remote_tuning` ladder at all six call sites; receivers
  advertise real `CapacityProfile`s (daemon push negotiation, pull_sync
  client spec, delegated dst); live tuner samples PR1 telemetry on the
  push data plane (first production `LiveProbe`); `write_blocked_nanos`
  now times only the socket write (carried 1a finding). codex NEEDS
  FIXES → 3 Mediums fixed (`46da929`: in-flight bounds chunk; idle
  ticks are no-signal; shard writes feed the blocked signal). Commits
  `3be9105`..`46da929`; tests **1402 / 0 / 2** (4 deleted ladder tests
  called out, offset by 7 dial tests). Stream-count ladders remaining:
  `desired_streams` (1f), `pull_stream_count` (1g/1h).
- **`ue-r2-1d` COMPLETE** — streaming plan foundation (details: DEVLOG
  2026-07-03). The engine's local leg plans from a partial header
  stream: `engine/streaming_plan.rs` (`InitialPlan` novel/known split,
  `PlanUpdate`, 512-header/250ms-timer/close batch flush) feeding
  `execute_sink_pipeline_streaming` concurrently. Mirror deletion still
  requires a complete clean scan (RELIABLE); phase split redefined
  (planner = time-to-first-payload). Structural proof revert-proven
  (collect-all deadlocks the gated test). codex FAIL → 2 findings
  accepted + fixed (`29159ca`): nested-dest self-copy exclusion (+
  two-run regression test) and scan-handle observation on error paths.
  Commits `c08a5c1`+`29159ca`; tests **1399 / 0 / 2** (baseline 1394).
  Surfaced pre-existing gap: streaming summaries carry no tar/raw
  bucket stats → tuning window never admits streaming records →
  `derive_local_plan_tuning` dead at HEAD (see 1d finding Known gaps;
  1e/w2-2 territory).
- **`ue-r2-1c` COMPLETE** — engine shell landed (details: DEVLOG
  2026-07-03). New `crates/blit-core/src/engine/`: `TransferEngine::
  execute(EngineRequest)` owns strategy selection (single-file → journal
  → fast paths → streaming) and the streaming leg; `TransferOrchestrator`
  is now the local adapter (source/sink construction + option
  translation; public API preserved via re-exports). **Engine type
  ratified** (REV4 Design §1: new engine + adapter). Single-file strategy
  gained the REV4-named missing accounting (tag `single_file`, guard
  test proven by revert). Commits `7730eb1` (pins) + `dc9b0ed` (move,
  fidelity machine-checked) + `29e210b` (accounting); codex retry PASS
  with 1 Low fixed (`15e6334`). Validation: fmt/clippy clean,
  `cargo test --workspace` **1394 / 0 / 2** (baseline 1391, +3).
- **`ue-r2-1b` COMPLETE** — wire dial contract (`2741dc8` + review fix
  `5bd345a`; codex PASS zero findings): `CapacityProfile` as
  `DataTransferNegotiation.receiver_capacity = 11` and
  `TransferOperationSpec.receiver_capacity = 12` (**spec_version stays
  2 — no bump**, exact-match gate), `resize_enabled = 12` +
  `epoch0_sub_token = 13`, capability bits (PushHeader 8 /
  PeerCapabilities 5, false until `ue-r2-2`), `DataPlaneResize`/`Ack`
  variants in all four control streams (prior art `d9d4ec7`, 11–14
  clash resolved via `CapacityProfile.max_streams`). Delegated dst
  override also strips CLI-supplied `receiver_capacity`. Both
  mixed-version directions compat-tested
  (`crates/blit-core/tests/proto_wire_compat.rs`). Tests 1378→1391.
- **Active context** (settled background for the slice work):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the **Active**
    convergence plan (D-2026-06-20-5); v1/REV2/REV3 Superseded.
  - Direction (D-2026-06-20-1/-2): one src/dst-agnostic sequencer for all
    four paths; one live dial replaces the three live stream-count ladders
    (`determine_remote_tuning`, `desired_streams`, `pull_stream_count`);
    **bounded-unilateral**, **no probe phase** (start within ~1s at
    conservative defaults, tune live), workload-shape-aware planner.
  - Process (D-2026-06-20-6, `docs/agent/GPT_REVIEW_LOOP.md`): for `ue-r2-*`
    slices Claude codes+commits each slice, `codex`/GPT-5.5 reviews it,
    Claude adjudicates + fixes. Per-slice commits to `master` ungated (no
    branches, never push); per-slice code acceptance owner-delegated. Owner
    gates remaining: **push**, **10 GbE sign-off**.

## Queue (ordered)

1. **`ue-r2-1f` (push converge)** — next REV4 slice: route push
   through the engine preserving manifest streaming, need-list
   batching, fallback timing, scan-completeness purge safety, old/new
   compat; **retire the daemon `desired_streams` ladder** into the
   dial. Note for 1f: remote transfers record no perf history
   (local-only lanes) — candidate to add remote lanes there; the dead
   `derive_local_plan_tuning` window (1d finding) also still awaits a
   fold-or-retire call (w2-2). Per D-2026-06-20-6 the loop may continue
   autonomously on owner "continue"; owner may push the 1e stack first.
2. **Then** `1g` → `1h` → `ue-r2-2`
   (deps in REV4 §"Slice dependencies"), each through the GPT review loop.
3. **Design-review queue (independent, survives the convergence)** —
   `REVIEW.md` order governs. Highest open ratified row is **w4-1**
   (AbortOnDrop family, High) — now also owns the `ue-r2-1a` hard-abort gap.
   Then w4-3, W1 socket-policy / timeout constants. May fold into `ue-r2-1c`
   or fix standalone, owner's call.
4. **10 GbE benchmark session — DEFERRED** (owner 2026-06-12). The `ue-1`
   sign-off measure (loopback parity band: local↔local / local→daemon /
   daemon→local within a tight band) AND the `ue-2` (continuous/C) gate; also
   the zero-copy revisit gate (D-2026-06-12-1). Capture before/after
   baselines there. After `ue-1`: audit Round 1, TUI rework, H10b planner.
   **Test environment (owner, 2026-07-03)**: `admin@skippy:/mnt/generic-pool/video/test`
   — scp and ssh open from this user to `admin@skippy`; if a daemon needs
   to run on skippy and can't, ping the owner. (BENCHMARK_10GBE_PLAN.md is
   `Status: Historical`; the environment note lives here until a live
   benchmark doc exists.)

## Authoritative docs right now

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** — convergence
  engine (D-2026-06-20-5); absorbs `MULTISTREAM_PULL.md` as slice `ue-r2-1g`.
- Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
  `…_REV2.md`, `…_REV3.md`.
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active, D-2026-06-20-6) governs
  `ue-r2-*`; `.review/README.md` async loop governs other work.
- Review loop: `REVIEW.md` (`ue-r2-1a`..`1e` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (`ue-1`/`ue-2` gate).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (review loop).

## Blocked / waiting

- **Owner**: "continue" → I pick up `ue-r2-1f` (or push
  `origin..master` — the ue-r2-1e stack after `e1a21a1` — first).
  Doesn't block autonomous continuation per D-2026-06-20-6.

## Open questions

- **(OPEN)** Edit D-2026-06-20-1 to strip its superseded warmup/size-gate
  wording? Owner: not sure. (Agent rec: edit with a one-line note → -2/-5.)
- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(RESOLVED 2026-07-03)** Adaptive-streams prior art consumed:
  `ue-r2-1b` harvested the `d9d4ec7` proto contract (resize messages kept
  near-verbatim; its negotiation fields 11–14 renumbered/subsumed — see
  the `NOTE on field numbers` block in `proto/blit.proto`). The branch
  `origin/feat/adaptive-streams-pr3-resizable` has served its purpose for
  the wire slice; still referenced as prior art for `ue-r2-2`'s
  controller/dialer wiring. (Full audit history: DEVLOG 2026-06-21 /
  STATE history at `2c1b839`.)
- **(RESOLVED 2026-07-03)** Engine type — ratified at `ue-r2-1c` as
  planned: new `TransferEngine` + `TransferOrchestrator` as local
  adapter (REV4 Design §1); owner did not override.
- **Windows**: w9-1/w9-5/w9-4/w4-2 added ungated daemon-spawn tests,
  unverified on Windows; `439a2a7` is now on origin, so the next
  windows-latest CI run is meaningful — triage real failures into findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-03 (4th)** @ `46da929`+docs — `ue-r2-1e` landed end-to-end
  (dial `3be9105`, profiles `a0d2c9f`, ladder retired `98943b7`, tuner
  `15968f4`, codex 3 Mediums fixed `46da929`). fmt/clippy clean; tests
  1402/0/2. In-flight: none — paused at a slice boundary. **Exact
  first action next session**: on owner "continue", start `ue-r2-1f`
  (push converge, retire `desired_streams`) through the loop; else
  owner pushes the stack / decides the D-2026-06-20-1 edit.
- **2026-07-03 (3rd)** @ `29159ca`+docs — `ue-r2-1d` landed end-to-end
  (slice `c08a5c1`; codex FAIL → nested-dest self-copy High + scan-handle
  Medium both fixed `29159ca`). fmt/clippy clean; tests 1399/0/2.
  In-flight: none — paused at a slice boundary. **Exact first action
  next session**: on owner "continue", start `ue-r2-1e` (live cheap
  dials) through the loop; else owner pushes the stack / decides the
  D-2026-06-20-1 edit.
- **2026-07-03 (later)** @ `15e6334`+docs — `ue-r2-1c` landed end-to-end
  (pins `7730eb1`, engine move `dc9b0ed`, single-file accounting
  `29e210b`, codex retry PASS → 1 Low fixed `15e6334`). fmt/clippy
  clean; tests 1394/0/2. Also: owner provided the 10GbE test env
  (Queue item 4) and restored codex after a quota outage. In-flight:
  none — paused at a slice boundary. **Exact first action next
  session**: on owner "continue", start `ue-r2-1d` (streaming plan
  foundation) through the loop; else owner pushes the stack / decides
  the D-2026-06-20-1 edit.
