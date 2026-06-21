# STATE — single entry point for "what is true right now"

Last updated: 2026-06-21 (`ue-r2-1a` complete — first REV4 slice landed via
the code→review→fix loop) at commit `2c1b839`; unpushed stack
`b663091..2c1b839` (this handoff commits on top).

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`ue-r2-1a` COMPLETE** — first REV4 slice; the code→GPT-review→fix loop
  ran end-to-end. Salvaged the adaptive substrate via cherry-pick over the
  `-s ours` octopus trap (D-2026-06-07-2, where a plain merge no-ops): PR1
  zero-cost `Probe` telemetry (`e569eea`), PR2 work-stealing `flume` queue
  (`3844a15`), PR2 forwarder-halt fix (`ec561f2`); hand-resolved the
  `data_plane.rs` StallGuard-vs-`Probe` conflict; work-stealing behaviour
  tests (`771a632`); codex/GPT-5.5 review → fix-then-ship, 4 findings all
  accepted + fixed (`90ed43d`); STATE/DEVLOG + codex-artifact trim
  (`2c1b839`). Validation: fmt/clippy clean, `cargo test --workspace`
  **1378 / 0 / 2** (baseline 1370). Carried to `ue-r2-1e`: PR1
  `write_blocked_nanos` timing accuracy (no telemetry consumer until the
  dial). Hard-abort-on-drop of workers stays `w4-1`.
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

1. **`ue-r2-1b` (wire dial contract)** — next REV4 slice: define the
   capacity-profile + peer-capability + resize proto shape
   (`receiver_capacity = 11`, `DataPlaneResize`/`Ack`) with old/new compat
   tests, before any code depends on the fields. Per D-2026-06-20-6 the loop
   may continue autonomously on owner "continue"; owner may push the
   `b663091..2c1b839` stack first. Also pending separately: push approval for
   the Windows test-tuning commit (`439a2a7`, local-only — Windows CI red
   until it lands).
2. **Then** the rest of the REV4 slice list in order —
   `1c` → `1d`/`1e`/`1f` → `1g` → `1h` → `ue-r2-2`
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

## Authoritative docs right now

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** — convergence
  engine (D-2026-06-20-5); absorbs `MULTISTREAM_PULL.md` as slice `ue-r2-1g`.
- Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
  `…_REV2.md`, `…_REV3.md`.
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active, D-2026-06-20-6) governs
  `ue-r2-*`; `.review/README.md` async loop governs other work.
- Review loop: `REVIEW.md` (`ue-r2-1a` row `[x]`; design-queue rows) +
  `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (`ue-1`/`ue-2` gate).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (review loop).

## Blocked / waiting

- **Owner**: (1) "continue" → I pick up `ue-r2-1b` (or push the
  `b663091..2c1b839` stack first); (2) push approval for `439a2a7` (Windows
  CI red until it lands). Neither blocks me from continuing autonomously per
  D-2026-06-20-6 once you say go.

## Open questions

- **(OPEN)** Edit D-2026-06-20-1 to strip its superseded warmup/size-gate
  wording? Owner: not sure. (Agent rec: edit with a one-line note → -2/-5.)
- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(OPEN, now live)** Disposition of the adaptive-streams branch refs
  (`e6ef095`…`eafb187`, `d9d4ec7`) now that PR1/PR2 are cherry-picked onto
  master (D-2026-06-07-2). Deleting them is an owner-named-branch op.
- **Engine type** — agent recommends a new `TransferEngine` + local adapter;
  ratified at `ue-r2-1c`, owner may override.
- **Windows**: w9-1/w9-5/w9-4/w4-2 added ungated daemon-spawn tests,
  unverified on Windows; next windows-latest CI run or run-blit-tests.ps1
  triages real failures into findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-06-21** @ `2c1b839` — `ue-r2-1a` landed end-to-end through the
  code→GPT-review→fix loop (substrate cherry-pick `e569eea`/`3844a15`/
  `ec561f2`, conflict resolved, tests `771a632`, codex review → 4 findings
  all fixed `90ed43d`, docs `2c1b839`). fmt/clippy clean; test 1378/0/2. All
  on master, **unpushed** (`b663091..2c1b839`). In-flight: none — paused at a
  slice boundary. **Exact first action next session**: on owner "continue",
  start `ue-r2-1b` (wire/dial proto contract) through the loop; else owner
  pushes the stack / approves `439a2a7` / decides the D-2026-06-20-1 edit.
- **2026-06-20** @ `09268eb` — reviewed all three unified-transfer candidates,
  produced REV4 (code-reality corrected; REV3's "two static tables" ladder
  claim was wrong — all three ladders live), and on owner's "rev4 replaces
  v1" recorded **D-2026-06-20-5** + propagated (REV4 Active; v1/REV2/REV3
  Superseded). Then established the GPT review loop (D-2026-06-20-6) and
  ported then removed SETUP.md (folded into governance).
