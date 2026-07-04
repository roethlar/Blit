# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`ue-r2-1h` complete — seventh REV4 slice
through the code→review→fix loop; first session validated on the
owner's Windows host, which surfaced and fixed two pre-existing
Windows bugs); unpushed to `origin`/gitea: everything after `7603177`
(the ue-r2-1f/1g/1h stacks + records).

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`ue-r2-1h` COMPLETE** — deprecated Pull RPC deleted; relay ported
  onto PullSync (details: DEVLOG 2026-07-04). Proto loses `rpc Pull` +
  `PullRequest`/`PullChunk`; daemon loses `service/pull.rs` and the
  `pull_stream_count` ladder — **all three static ladders are now
  gone** (1e/1f/1h). `PullEntry`/`collect_pull_entries_with_checksums`
  relocated into `pull_sync.rs`. The relay's `scan_remote_files`/
  `open_remote_file` (owner-mandated port, w2-4/D-2026-06-11-2) ride
  PullSync: additive `TransferOperationSpec.metadata_only = 13`
  header-scan sessions + single-file force_grpc streaming. codex
  NEEDS FIXES (3) + 3-lens panel → 5 fixed (`f6f52d7`: delegated pull
  REJECTS forwarded metadata_only — both reviewers independently
  caught the zero-byte-materialization hazard; adapter recursion →
  loop; second-header guard; API/ARCHITECTURE docs), 1 deferred
  (relay-1 subpath double-join, pre-existing), 1 rejected with
  citation. Commits `2a13f53` + `f6f52d7` (+`9f37a7a`, `48c5a11`
  below); tests **1393 / 0 / 3 (Windows host)** — the unix baseline
  1413/0/2 is not host-comparable (unix-gated tests compile out).
  Deleting pull.rs also resolved 2 of design-2's 3 spawn sites (w4-1
  rescoped). **Erratum**: the pull.rs deletion was staged early and
  rode into `9f37a7a`, so `9f37a7a`/`48c5a11` don't build in isolation
  (bisect must skip); disclosed in the finding doc — owner may
  authorize a history fix or leave it.
- **Windows-host discoveries (fixed, own commits)**: `9f37a7a` — 15
  pre-existing Windows-only clippy violations (the ps1 gate never ran
  clippy); `48c5a11` — **win-1 (High)**: daemon push need-list echoed
  backslash paths, so every nested push to a Windows daemon planned
  zero payloads and stalled 30s (`relative_path_to_posix` one-liner;
  un-broke `test_push_nested_directories` +
  `forced_grpc_push_many_files_completes` on Windows).
- **`ue-r2-1b`..`ue-r2-1g` COMPLETE** — wire dial contract, engine
  shell, streaming plan foundation, live cheap dials, push converge,
  PullSync multistream (each through the full loop; details in DEVLOG
  2026-07-03 entries and per-slice `.review/findings/` docs; REVIEW.md
  has the commit map).
- **Active context** (settled background for the slice work):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the **Active**
    convergence plan (D-2026-06-20-5); v1/REV2/REV3 Superseded.
  - Direction (D-2026-06-20-1/-2): one src/dst-agnostic sequencer for all
    four paths; one live dial replaced the three static stream-count
    ladders (`determine_remote_tuning` 1e, `desired_streams` 1f,
    `pull_stream_count` 1h — done); **bounded-unilateral**, **no probe
    phase**, workload-shape-aware planner.
  - Process (D-2026-06-20-6, `docs/agent/GPT_REVIEW_LOOP.md`): for `ue-r2-*`
    slices Claude codes+commits each slice, `codex`/GPT-5.5 reviews it,
    Claude adjudicates + fixes. Per-slice commits to `master` ungated (no
    branches, never push); per-slice code acceptance owner-delegated. Owner
    gates remaining: **push**, **10 GbE sign-off**.

## Queue (ordered)

1. **`ue-r2-2` (stream resize)** — the LAST REV4 slice: negotiated
   `DataPlaneResize`/`DataPlaneResizeAck` (wire landed at 1b), add/drop
   streams mid-transfer from live telemetry via the 1a elastic queue on
   the already-mutable dial. Prior art: `origin/feat/adaptive-streams-
   pr3-resizable` controller/dialer wiring. Unlocks pull 1s-start
   (1g Known gaps). Carried context: remote perf-history lanes still
   unrecorded (1e gap); dead `derive_local_plan_tuning` window awaits
   fold-or-retire (w2-2); sequential-accept pin growth deferred to the
   W1 socket-policy row. Per D-2026-06-20-6 the loop may continue
   autonomously on owner "continue"; owner may push the stack first.
2. **Design-review queue (independent, survives the convergence)** —
   `REVIEW.md` order governs. Highest open ratified row is **w4-1**
   (AbortOnDrop family, High) — rescoped at 1h: design-2 now covers only
   the `push/control.rs` spawn site. Then w4-3, W1 socket-policy /
   timeout constants. New open rows from 1h: `relay-1-subpath-double-join`
   (Low, pre-existing).
3. **10 GbE benchmark session — DEFERRED** (owner 2026-06-12). The `ue-1`
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
- Review loop: `REVIEW.md` (`ue-r2-1a`..`1h` rows `[x]`; w2-3/w2-4 flipped
  `[x]` as absorbed-and-delivered; design-queue rows) +
  `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (`ue-1`/`ue-2` gate).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (review loop).

## Blocked / waiting

- **Owner**: "continue" → I pick up `ue-r2-2` (or push `origin..master`
  — everything after `7603177`: the 1f/1g/1h stacks — first). Doesn't
  block autonomous continuation per D-2026-06-20-6.
- **Owner call on the 1h commit erratum**: `9f37a7a`/`48c5a11` don't
  build in isolation (staging slip; HEAD fully gated). Leave as-is
  (default, no history rewrite) or authorize a rewrite of the unpushed
  stack.
- **Owner call on `Cargo.lock`**: working-tree drift (dep version
  bumps + new transitive `approx`) predates this session and every
  gate since at least 1f ran WITH it; left uncommitted. Commit it,
  or regenerate from the committed lockfile?

## Open questions

- **(OPEN)** Edit D-2026-06-20-1 to strip its superseded warmup/size-gate
  wording? Owner: not sure. (Agent rec: edit with a one-line note → -2/-5.)
- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: the full suite now
  runs green locally on the owner's Windows machine (first time ever) —
  the two real failures found became `9f37a7a` (clippy baseline) and
  win-1 (`48c5a11`). The daemon-spawn e2e family shows load-flakiness
  under full-parallel runs (admin_verbs flaked once, passed isolated
  and on rerun) — the w9-3 harness row's territory. windows-latest CI
  on the next push should now be meaningfully green.
- **(RESOLVED 2026-07-03)** Adaptive-streams prior art consumed:
  `ue-r2-1b` harvested the `d9d4ec7` proto contract; the branch
  `origin/feat/adaptive-streams-pr3-resizable` remains prior art for
  `ue-r2-2`'s controller/dialer wiring. (History: DEVLOG 2026-06-21.)
- **(RESOLVED 2026-07-03)** Engine type — ratified at `ue-r2-1c` as
  planned: new `TransferEngine` + `TransferOrchestrator` as local
  adapter (REV4 Design §1); owner did not override.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-04 (7th)** @ `f6f52d7`+docs — `ue-r2-1h` landed end-to-end
  (`2a13f53` deletion+port; codex NEEDS FIXES 3 + panel → 5 fixed
  `f6f52d7`, 1 deferred relay-1, 1 rejected; plus Windows-host
  pre-existing fixes `9f37a7a` clippy baseline + `48c5a11` win-1
  push-separator). fmt/clippy clean; tests 1393/0/3 on Windows.
  In-flight: none — paused at a slice boundary; `Cargo.lock` drift
  left uncommitted (owner call). **Exact first action next session**:
  on owner "continue", start `ue-r2-2` (stream resize — the last REV4
  slice) through the loop; else owner pushes the stack / decides the
  erratum + Cargo.lock + D-2026-06-20-1 questions.
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
  `ue-r2-1g` (PullSync multistream through the engine) through the
  loop; else owner pushes the stack / decides the D-2026-06-20-1 edit.
