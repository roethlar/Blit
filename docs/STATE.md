# STATE — single entry point for "what is true right now"

Last updated: 2026-07-03 (`ue-r2-1b` through `ue-r2-1g` complete — six
REV4 slices in one day, all through the code→review→fix loop); unpushed
to `origin`/gitea: everything after `7603177` (the ue-r2-1f and
ue-r2-1g commits + records; owner pushed through `7603177` earlier
today).

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`ue-r2-1g` COMPLETE** — PullSync multistream through the engine
  (details: DEVLOG 2026-07-03). Daemon proposes via
  `engine::initial_stream_proposal` gated on the client's advertised
  `receiver_capacity` (absent/unknown → 1 stream, today's behavior);
  `accept_and_wrap_sinks` harvested from the deprecated Pull RPC into
  `pull_sync.rs`; client side needed no change (fan-out predates the
  profile — gate proven safe from git history); NO proto changes;
  resume stays single-stream (explicit RELIABLE exception); delegated
  inherits free. codex NEEDS FIXES → 2 accepted + fixed, plus a
  3-lens self-review panel: 2 more fixed (client-side stream-count
  clamp; token-status restore), 1 deferred (accept-pin growth → W1).
  Commits `48e583e`+`4a2e58d`; tests **1413 / 0 / 2**. Pull 1s-start
  explicitly rides on `ue-r2-2` resize (finding doc Known gaps).
  Ladders remaining: `pull_stream_count` dies with its RPC at `1h`.
- **`ue-r2-1f` COMPLETE** — push converge (details: DEVLOG 2026-07-03).
  `desired_streams` retired into `engine::initial_stream_proposal`
  (same shape table, receiver-ceiling-clamped, first-ever tests with
  exact tier boundaries); wire-identical negotiations; codex PASS with
  1 Low fixed (`0c8da50`) and judged the finding doc's boundary
  interpretation plan-conformant. Commits `a4a9f70`+`0c8da50`; tests
  **1403 / 0 / 2**. Ladders remaining: `pull_stream_count` only
  (`1g`/`1h`).
- **`ue-r2-1b`..`ue-r2-1e` COMPLETE** — wire dial contract, engine
  shell, streaming plan foundation, live cheap dials (each through the
  full loop; details pruned to DEVLOG 2026-07-03 entries and the
  per-slice `.review/findings/` docs; REVIEW.md has the commit map).
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

1. **`ue-r2-1h` (delete deprecated Pull RPC)** — next REV4 slice:
   delete the deprecated `Pull` RPC + its `pull_stream_count` ladder;
   1g harvested the multistream pattern (`accept_and_wrap_sinks` now
   lives in `pull_sync.rs`; the deprecated handlers borrow it back),
   and the 1b compat tests are green. Must relocate `PullEntry` +
   `collect_pull_entries_with_checksums` (pull_sync depends on them)
   — 1g finding doc Known gaps. Carried context: remote perf-history
   lanes still unrecorded (1e gap); dead `derive_local_plan_tuning`
   window awaits fold-or-retire (w2-2); pull 1s-start rides on
   `ue-r2-2` resize (1g Known gaps); sequential-accept pin growth
   deferred to the W1 socket-policy row. Per D-2026-06-20-6 the loop
   may continue autonomously on owner "continue"; owner may push the
   1f/1g stack first.
2. **Then** `ue-r2-2` (stream resize — the last REV4 slice; deps in
   REV4 §"Slice dependencies"), through the GPT review loop.
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
- Review loop: `REVIEW.md` (`ue-r2-1a`..`1f` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (`ue-1`/`ue-2` gate).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (review loop).

## Blocked / waiting

- **Owner**: "continue" → I pick up `ue-r2-1h` (or push
  `origin..master` — the ue-r2-1f + ue-r2-1g commits after `7603177` —
  first). Doesn't block autonomous continuation per D-2026-06-20-6.

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

- **2026-07-03 (6th)** @ `4a2e58d`+docs — `ue-r2-1g` landed end-to-end
  (`48e583e` multistream + engine proposal; codex NEEDS FIXES → 2
  fixed + self-review panel 2 fixed / 1 deferred, `4a2e58d`). fmt/
  clippy clean; tests 1413/0/2. In-flight: none — paused at a slice
  boundary. **Exact first action next session**: on owner "continue",
  start `ue-r2-1h` (delete deprecated Pull RPC; relocate `PullEntry`/
  `collect_pull_entries_with_checksums` per the 1g finding doc)
  through the loop; else owner pushes the stack / decides the
  D-2026-06-20-1 edit.
- **2026-07-03 (5th)** @ `0c8da50`+docs — `ue-r2-1f` landed end-to-end
  (`a4a9f70` ladder retirement; codex PASS, 1 Low boundary-test gap
  fixed `0c8da50`; interpretation judged plan-conformant). fmt/clippy
  clean; tests 1403/0/2. In-flight: none — paused at a slice boundary.
  **Exact first action next session**: on owner "continue", start
  `ue-r2-1g` (PullSync multistream through the engine — the biggest
  remaining slice; fresh session recommended) through the loop; else
  owner pushes the stack / decides the D-2026-06-20-1 edit.
- **2026-07-03 (4th)** @ `46da929`+docs — `ue-r2-1e` landed end-to-end
  (dial `3be9105`, profiles `a0d2c9f`, ladder retired `98943b7`, tuner
  `15968f4`, codex 3 Mediums fixed `46da929`). fmt/clippy clean; tests
  1402/0/2. In-flight: none — paused at a slice boundary. **Exact
  first action next session**: on owner "continue", start `ue-r2-1f`
  (push converge, retire `desired_streams`) through the loop; else
  owner pushes the stack / decides the D-2026-06-20-1 edit.
