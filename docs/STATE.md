# STATE — single entry point for "what is true right now"

Last updated: 2026-07-03 (`ue-r2-1b` complete — wire dial contract landed
via the code→review→fix loop) at commits `2741dc8`+`5bd345a`; unpushed to
`origin`: `fcf3345`+`2741dc8`+`5bd345a` (gitea has `fcf3345` already —
owner pushed it 2026-07-03; this handoff commits on top).

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`ue-r2-1b` COMPLETE** — second REV4 slice through the GPT review loop.
  Wire dial contract defined before any behavior depends on it
  (`2741dc8`): `CapacityProfile` (rich receiver→sender profile, 0 =
  unknown = stay conservative) as `DataTransferNegotiation.
  receiver_capacity = 11` (push; daemon is receiver) and
  `TransferOperationSpec.receiver_capacity = 12` (pull_sync/delegated;
  client/dst is receiver) — **spec_version stays 2, deliberately no bump**
  (exact-match gate would make old daemons reject new clients; profile is
  a skippable hint); daemon-authoritative `resize_enabled = 12` +
  `epoch0_sub_token = 13`; capability bits `PushHeader.supports_stream_
  resize = 8` / `PeerCapabilities.supports_stream_resize = 5` (false until
  `ue-r2-2`); `DataPlaneResize`/`Ack` oneof variants in all four control
  streams (prior art `d9d4ec7`; its 11–14 clash resolved —
  min/max_stream_count subsumed by `CapacityProfile.max_streams`).
  Delegated dst override now also strips CLI-supplied `receiver_capacity`
  (R25-F2 boundary). Compat tests both mixed-version directions
  (`crates/blit-core/tests/proto_wire_compat.rs`, old-shape prost
  replicas). Review: codex/GPT-5.5 **PASS, zero findings**; supplementary
  4-lens self-review found 1 Low (false deprecated-Pull claim in a proto
  comment), fixed in `5bd345a`. Validation: fmt/clippy clean,
  `cargo test --workspace` **1391 / 0 / 2** (baseline 1378, +13).
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

1. **`ue-r2-1c` (engine shell + local adapter)** — next REV4 slice: add
   `TransferEngine`, convert `TransferOrchestrator` into the local
   adapter, move the local fast paths (`journal_no_work`, `no_work`,
   `tiny_manifest`, `single_huge_file`, single-file shortcut) under
   engine-owned strategies, preserving behavior and adding accounting
   where the single-file shortcut lacked it. Engine-type recommendation
   (new `TransferEngine` + local adapter) is ratified at this slice —
   owner may override before it starts. Per D-2026-06-20-6 the loop may
   continue autonomously on owner "continue"; owner may push the
   `origin..master` stack first.
2. **Then** the rest of the REV4 slice list in order —
   `1d`/`1e`/`1f` → `1g` → `1h` → `ue-r2-2`
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
- Review loop: `REVIEW.md` (`ue-r2-1a`/`ue-r2-1b` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (`ue-1`/`ue-2` gate).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (review loop).

## Blocked / waiting

- **Owner**: (1) "continue" → I pick up `ue-r2-1c` (or push the
  `origin..master` stack — `fcf3345`+`2741dc8`+`5bd345a` — first; gitea
  additionally lacks the last two). Doesn't block autonomous continuation
  per D-2026-06-20-6. (2) RESOLVED since last handoff: `439a2a7` (Windows
  test-tuning) is now on `origin` — the Windows-CI push blocker is gone.

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
- **Engine type** — agent recommends a new `TransferEngine` + local adapter;
  ratified at `ue-r2-1c`, owner may override.
- **Windows**: w9-1/w9-5/w9-4/w4-2 added ungated daemon-spawn tests,
  unverified on Windows; `439a2a7` is now on origin, so the next
  windows-latest CI run is meaningful — triage real failures into findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-03** @ `5bd345a` — `ue-r2-1b` landed end-to-end through the
  code→GPT-review→fix loop (wire contract `2741dc8`; codex PASS zero
  findings; 1 Low self-review finding fixed `5bd345a`). fmt/clippy clean;
  tests 1391/0/2. All on master; unpushed to origin:
  `fcf3345`+`2741dc8`+`5bd345a`. In-flight: none — paused at a slice
  boundary. **Exact first action next session**: on owner "continue",
  start `ue-r2-1c` (engine shell + local adapter) through the loop —
  ratifying the engine-type recommendation unless the owner overrides;
  else owner pushes the stack / decides the D-2026-06-20-1 edit.
- **2026-06-21** @ `2c1b839` — `ue-r2-1a` landed end-to-end through the
  code→GPT-review→fix loop (substrate cherry-pick `e569eea`/`3844a15`/
  `ec561f2`, conflict resolved, tests `771a632`, codex review → 4 findings
  all fixed `90ed43d`, docs `2c1b839`). fmt/clippy clean; test 1378/0/2.
- **2026-06-20** @ `09268eb` — reviewed all three unified-transfer candidates,
  produced REV4 (code-reality corrected; REV3's "two static tables" ladder
  claim was wrong — all three ladders live), and on owner's "rev4 replaces
  v1" recorded **D-2026-06-20-5** + propagated (REV4 Active; v1/REV2/REV3
  Superseded). Then established the GPT review loop (D-2026-06-20-6) and
  ported then removed SETUP.md (folded into governance).
